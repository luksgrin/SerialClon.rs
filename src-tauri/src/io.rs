//! Opening, saving and exporting through the native file dialogs.
//!
//! Everything here blocks on a dialog and must run off the main thread: call
//! it from an `async` command or a spawned thread.

use std::path::{Path, PathBuf};

use sc_seq::Source;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{
    DialogExt, FilePath, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};

use crate::AppState;
use crate::documents::{DocId, Document};
use crate::{autosave, menu, windows};

fn to_path(fp: FilePath) -> Option<PathBuf> {
    fp.into_path().ok()
}

/// Extensions offered by the Open dialog (User Manual §III.A plus the
/// formats added in 2.x).
const OPEN_FILTERS: &[(&str, &[&str])] = &[
    (
        "All sequence files",
        &[
            "xdna", "fasta", "fsta", "fa", "fas", "fna", "gb", "gbk", "genbank", "embl", "txt",
            "text", "asc", "ascii",
        ],
    ),
    ("Serial Cloner / DNA Strider", &["xdna"]),
    ("FASTA", &["fasta", "fsta", "fa", "fas", "fna"]),
    ("GenBank / EMBL", &["gb", "gbk", "genbank", "embl"]),
    ("Text", &["txt", "text", "asc", "ascii"]),
];

/// File ▸ Open…: picks files and opens each sequence in its own window.
pub fn open_dialog(app: &AppHandle) -> Vec<DocId> {
    let mut dialog = app.dialog().file().set_title("Open a sequence");
    for (name, exts) in OPEN_FILTERS {
        dialog = dialog.add_filter(*name, exts);
    }
    let Some(files) = dialog.blocking_pick_files() else {
        return Vec::new();
    };
    let paths: Vec<PathBuf> = files.into_iter().filter_map(to_path).collect();
    open_paths(app, &paths)
}

/// Opens every sequence in every file. Unreadable files get an error box.
pub fn open_paths(app: &AppHandle, paths: &[PathBuf]) -> Vec<DocId> {
    let mut ids = Vec::new();
    for path in paths {
        match sc_format::read_file(path) {
            Ok(seqs) => {
                let native = seqs
                    .first()
                    .is_some_and(|s| matches!(s.source, Source::Strider | Source::SerialCloner));
                let state = app.state::<AppState>();
                for seq in seqs {
                    let doc = Document::new(seq, if native { Some(path.clone()) } else { None });
                    let id = state.store.lock().unwrap().insert(doc);
                    ids.push(id);
                }
                state.recent.lock().unwrap().push(path);
            }
            Err(e) => {
                app.dialog()
                    .message(format!("{}\n\n{e}", path.display()))
                    .title("Cannot open this file")
                    .kind(MessageDialogKind::Error)
                    .blocking_show();
            }
        }
    }
    for &id in &ids {
        windows::open_sequence(app, id);
    }
    if !ids.is_empty() {
        menu::refresh_recent(app);
        menu::refresh_windows(app);
    }
    ids
}

/// File ▸ Save / Save As…. Returns `Ok(false)` if the user cancelled.
pub fn save(app: &AppHandle, id: DocId, save_as: bool) -> Result<bool, String> {
    let (current, name) = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        let doc = store.get(id).ok_or("no such document")?;
        (doc.path.clone(), doc.seq.name.clone())
    };
    let path = match (&current, save_as) {
        (Some(p), false) => p.clone(),
        _ => {
            let mut dialog = app
                .dialog()
                .file()
                .set_title("Save sequence")
                .add_filter("Serial Cloner", &["xdna"])
                .set_file_name(format!("{name}.xdna"));
            if let Some(dir) = current.as_ref().and_then(|p| p.parent()) {
                dialog = dialog.set_directory(dir);
            }
            match dialog.blocking_save_file().and_then(to_path) {
                Some(p) => p,
                None => return Ok(false),
            }
        }
    };
    let state = app.state::<AppState>();
    let bytes = {
        let mut store = state.store.lock().unwrap();
        let doc = store.get_mut(id).ok_or("no such document")?;
        // Saving under a new name renames the sequence, as the original did.
        if save_as || current.is_none() {
            doc.seq.name = sc_format::name_from_path(&path);
        }
        if doc.seq.source == Source::Strider {
            doc.seq.source = Source::SerialCloner;
        }
        sc_format::xdna::write(&doc.seq)
    };
    std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    {
        let mut store = state.store.lock().unwrap();
        if let Some(doc) = store.get_mut(id) {
            doc.path = Some(path.clone());
            doc.dirty = false;
            if doc.seq.source == Source::New
                || doc.seq.source == Source::Fasta
                || doc.seq.source == Source::GenBank
                || doc.seq.source == Source::RawText
            {
                doc.seq.source = Source::SerialCloner;
            }
        }
    }
    state.recent.lock().unwrap().push(&path);
    autosave::discard(app, id);
    menu::refresh_recent(app);
    menu::refresh_windows(app);
    Ok(true)
}

/// Export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// File ▸ Export ▸ FASTA.
    Fasta,
    /// File ▸ Export ▸ GenBank (with features).
    GenBank,
}

/// File ▸ Export. Returns `Ok(false)` if cancelled.
pub fn export(app: &AppHandle, id: DocId, format: ExportFormat) -> Result<bool, String> {
    let (name, text) = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        let doc = store.get(id).ok_or("no such document")?;
        let text = match format {
            ExportFormat::Fasta => sc_format::fasta::write(&doc.seq),
            ExportFormat::GenBank => sc_format::genbank::write(&doc.seq, &today()),
        };
        (doc.seq.name.clone(), text)
    };
    let (filter, ext) = match format {
        ExportFormat::Fasta => ("FASTA", "fasta"),
        ExportFormat::GenBank => ("GenBank", "gb"),
    };
    let Some(path) = app
        .dialog()
        .file()
        .set_title(format!("Export as {filter}"))
        .add_filter(filter, &[ext])
        .set_file_name(format!("{name}.{ext}"))
        .blocking_save_file()
        .and_then(to_path)
    else {
        return Ok(false);
    };
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(true)
}

/// File ▸ Export ▸ All Sequences To Multifasta.
pub fn export_multifasta(app: &AppHandle) -> Result<bool, String> {
    let text = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        let seqs: Vec<_> = store.iter().map(|(_, d)| d.seq.clone()).collect();
        sc_format::fasta::write_multi(seqs.iter())
    };
    let Some(path) = app
        .dialog()
        .file()
        .set_title("Save all sequences as multi-FASTA")
        .add_filter("FASTA", &["fasta"])
        .set_file_name("sequences.fasta")
        .blocking_save_file()
        .and_then(to_path)
    else {
        return Ok(false);
    };
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(true)
}

/// File ▸ Revert To Saved.
pub fn revert(app: &AppHandle, id: DocId) -> Result<bool, String> {
    let (path, name) = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        let doc = store.get(id).ok_or("no such document")?;
        (
            doc.path
                .clone()
                .ok_or("The sequence has never been saved.")?,
            doc.seq.name.clone(),
        )
    };
    let ok = app
        .dialog()
        .message(format!(
            "Revert \"{name}\" to the version saved on disk? All modifications will be lost."
        ))
        .title("Revert To Saved")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Revert".into(),
            "Cancel".into(),
        ))
        .blocking_show();
    if !ok {
        return Ok(false);
    }
    let seqs = sc_format::read_file(&path).map_err(|e| e.to_string())?;
    let seq = seqs.into_iter().next().ok_or("empty file")?;
    let state = app.state::<AppState>();
    let mut store = state.store.lock().unwrap();
    let doc = store.get_mut(id).ok_or("no such document")?;
    let locked = doc.locked;
    *doc = Document::new(seq, Some(path));
    doc.locked = locked;
    Ok(true)
}

/// The user's answer to "Save changes?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseDecision {
    /// Save, then close.
    Save,
    /// Close without saving.
    Discard,
    /// Keep the window open.
    Cancel,
}

/// Asks whether to save a dirty document before closing it.
pub fn confirm_close(app: &AppHandle, id: DocId) -> CloseDecision {
    let name = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        match store.get(id) {
            Some(d) if d.dirty => d.seq.name.clone(),
            _ => return CloseDecision::Discard,
        }
    };
    match app
        .dialog()
        .message(format!(
            "Do you want to save the changes made to \"{name}\"?"
        ))
        .title("Unsaved changes")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNoCancel)
        .blocking_show_with_result()
    {
        MessageDialogResult::Yes => CloseDecision::Save,
        MessageDialogResult::No => CloseDecision::Discard,
        _ => CloseDecision::Cancel,
    }
}

/// A file dropped on the Open button or passed on the command line.
pub fn is_sequence_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    OPEN_FILTERS[0].1.contains(&ext.as_str())
}

/// Today as `DD-MON-YYYY`, the GenBank LOCUS date.
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    const MONTHS: [&str; 12] = [
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ];
    format!("{d:02}-{}-{y}", MONTHS[(m - 1) as usize])
}

#[cfg(test)]
mod tests {
    #[test]
    fn today_is_a_genbank_date() {
        let t = super::today();
        assert_eq!(t.len(), 11, "{t}");
        assert_eq!(t.as_bytes()[2], b'-');
        assert!(
            t.ends_with("202") || t[7..].parse::<u32>().unwrap() >= 2026,
            "{t}"
        );
    }
}
