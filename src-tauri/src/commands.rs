//! The command surface exposed to the webviews.
//!
//! Commands are coarse-grained and document-oriented (`PLAN.md` §3): windows
//! send an id and get a whole `DocumentView` back. After any change the new
//! view is also broadcast as a `doc:changed` event to the document's windows.

use sc_seq::alphabet::{is_palindromic, matches_at, reverse_complement};
use sc_seq::stats::{BaseCounts, DEFAULT_NA_MOLAR, melting_temperature, peptide_len, protein_mw};
use sc_seq::translate::{STANDARD, find_orfs};
use sc_seq::{Source, Strand, Topology};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::documents::{DocId, DocumentView, EditOp};
use crate::io::ExportFormat;
use crate::{AppState, autosave, io, menu, windows};

/// Static information for the About box and the Toolbar's hover help.
#[derive(Debug, Serialize)]
pub struct AppInfo {
    /// Product name as bundled.
    pub name: String,
    /// Application version.
    pub version: String,
    /// Tauri runtime version.
    pub tauri_version: String,
    /// `REBASE version NNN` of the bundled enzyme database, if readable.
    pub rebase_release: Option<u32>,
    /// Number of enzymes in the bundled database.
    pub enzyme_count: usize,
}

/// Returns name, version and the bundled enzyme database's release.
#[tauri::command]
pub fn app_info(app: AppHandle) -> AppInfo {
    let state = app.state::<AppState>();
    let package = app.package_info();
    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        tauri_version: tauri::VERSION.to_string(),
        rebase_release: state.library.info.rebase_release,
        enzyme_count: state.library.len(),
    }
}

/// Window ▸ Show/Hide Help Window. Returns `true` if the window is now visible.
#[tauri::command]
pub fn toggle_help_window(app: AppHandle) -> Result<bool, String> {
    windows::toggle_help(&app).map_err(|e| e.to_string())
}

/// Sends the current view of a document to its windows.
pub fn broadcast(app: &AppHandle, id: DocId) {
    let view = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        store.get(id).map(|d| d.view(id))
    };
    if let Some(view) = view {
        for label in [windows::seq_label(id), windows::find_label(id)] {
            let _ = app.emit_to(label.as_str(), "doc:changed", &view);
        }
        if let Some(w) = app.get_webview_window(&windows::seq_label(id)) {
            let title = if view.dirty {
                format!("{} •", view.name)
            } else {
                view.name.clone()
            };
            let _ = w.set_title(&title);
        }
    }
}

fn view_of(app: &AppHandle, id: DocId) -> Result<DocumentView, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    store
        .get(id)
        .map(|d| d.view(id))
        .ok_or_else(|| "no such document".into())
}

/// File ▸ New / New Degenerate DNA. Opens a window and returns the view.
#[tauri::command]
pub fn new_sequence(app: AppHandle, degenerate: bool) -> DocumentView {
    let id = app
        .state::<AppState>()
        .store
        .lock()
        .unwrap()
        .new_blank(degenerate);
    windows::open_sequence(&app, id);
    menu::refresh_windows(&app);
    view_of(&app, id).expect("just inserted")
}

/// File ▸ Open…
#[tauri::command]
pub async fn open_files(app: AppHandle) -> Vec<DocId> {
    io::open_dialog(&app)
}

/// Opens paths without a dialog (recent files, drag and drop).
#[tauri::command]
pub async fn open_paths(app: AppHandle, paths: Vec<String>) -> Vec<DocId> {
    let paths: Vec<std::path::PathBuf> = paths.into_iter().map(Into::into).collect();
    io::open_paths(&app, &paths)
}

/// The document a window shows.
#[tauri::command]
pub fn get_document(app: AppHandle, id: DocId) -> Result<DocumentView, String> {
    view_of(&app, id)
}

/// Ids and names of every open document.
#[tauri::command]
pub fn list_documents(app: AppHandle) -> Vec<(DocId, String)> {
    app.state::<AppState>().store.lock().unwrap().list()
}

/// What an edit returns.
#[derive(Debug, Serialize)]
pub struct EditResponse {
    /// The document after the edit.
    pub view: DocumentView,
    /// Letters dropped because they are not in the window's alphabet.
    pub filtered: usize,
}

/// Applies an edit and returns the new view.
#[tauri::command]
pub fn edit(app: AppHandle, id: DocId, op: EditOp) -> Result<EditResponse, String> {
    let filtered = {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().unwrap();
        let doc = store.get_mut(id).ok_or("no such document")?;
        doc.apply(op)?.filtered
    };
    broadcast(&app, id);
    menu::refresh_windows(&app);
    Ok(EditResponse {
        view: view_of(&app, id)?,
        filtered,
    })
}

/// Edit ▸ Undo.
#[tauri::command]
pub fn undo(app: AppHandle, id: DocId) -> Result<DocumentView, String> {
    {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().unwrap();
        store.get_mut(id).ok_or("no such document")?.undo();
    }
    broadcast(&app, id);
    view_of(&app, id)
}

/// Edit ▸ Redo.
#[tauri::command]
pub fn redo(app: AppHandle, id: DocId) -> Result<DocumentView, String> {
    {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().unwrap();
        store.get_mut(id).ok_or("no such document")?.redo();
    }
    broadcast(&app, id);
    view_of(&app, id)
}

/// Pastes text at `from..to`. FASTA or GenBank text pasted into an empty
/// window brings its name, comment and features along (User Manual §III.B,
/// Features ReadMe §VII.B); otherwise only the bases are inserted.
#[tauri::command]
pub fn paste(
    app: AppHandle,
    id: DocId,
    from: usize,
    to: usize,
    text: String,
) -> Result<EditResponse, String> {
    let parsed = sc_format::read(text.as_bytes(), None)
        .ok()
        .and_then(|mut v| {
            if v.is_empty() {
                None
            } else {
                Some(v.remove(0))
            }
        });
    let filtered = {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().unwrap();
        let doc = store.get_mut(id).ok_or("no such document")?;
        match parsed {
            Some(seq)
                if doc.seq.is_empty() && matches!(seq.source, Source::Fasta | Source::GenBank) =>
            {
                if doc.locked {
                    return Err("The sequence is locked.".into());
                }
                let bases = String::from_utf8_lossy(&seq.bases).into_owned();
                let out = doc.apply(EditOp::Replace {
                    from: 0,
                    to: 0,
                    text: bases,
                })?;
                if !seq.name.is_empty() && doc.seq.name.starts_with("Untitled") {
                    doc.seq.name = seq.name.clone();
                }
                doc.seq.comment = seq.comment.clone();
                doc.seq.features = seq.features.clone();
                doc.seq.topology = seq.topology;
                doc.seq.source = seq.source;
                out.filtered
            }
            Some(seq) => {
                let bases = String::from_utf8_lossy(&seq.bases).into_owned();
                doc.apply(EditOp::Replace {
                    from,
                    to,
                    text: bases,
                })?
                .filtered
            }
            None => doc.apply(EditOp::Replace { from, to, text })?.filtered,
        }
    };
    broadcast(&app, id);
    Ok(EditResponse {
        view: view_of(&app, id)?,
        filtered,
    })
}

/// File ▸ Save / Save As…. Returns the view, or `None` if cancelled.
#[tauri::command]
pub async fn save(
    app: AppHandle,
    id: DocId,
    save_as: bool,
) -> Result<Option<DocumentView>, String> {
    if !io::save(&app, id, save_as)? {
        return Ok(None);
    }
    broadcast(&app, id);
    view_of(&app, id).map(Some)
}

/// File ▸ Export.
#[tauri::command]
pub async fn export(app: AppHandle, id: DocId, format: ExportFormat) -> Result<bool, String> {
    io::export(&app, id, format)
}

/// File ▸ Revert To Saved.
#[tauri::command]
pub async fn revert(app: AppHandle, id: DocId) -> Result<Option<DocumentView>, String> {
    if !io::revert(&app, id)? {
        return Ok(None);
    }
    broadcast(&app, id);
    view_of(&app, id).map(Some)
}

/// Closes a document's window (the unsaved-changes prompt happens in the
/// window's close handler).
#[tauri::command]
pub fn close_document(app: AppHandle, id: DocId) {
    if let Some(w) = app.get_webview_window(&windows::seq_label(id)) {
        let _ = w.close();
    }
}

/// Sequence Window ▸ Find button.
#[tauri::command]
pub fn toggle_find_window(app: AppHandle, id: DocId) -> bool {
    windows::toggle_find(&app, id)
}

/// Window ▸ reference cards.
#[tauri::command]
pub fn open_reference_window(app: AppHandle, which: String) {
    windows::open_reference(&app, &which);
}

/// Writes text to the clipboard.
#[tauri::command]
pub fn clipboard_write(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

/// Reads text from the clipboard.
#[tauri::command]
pub fn clipboard_read(app: AppHandle) -> Result<String, String> {
    app.clipboard().read_text().map_err(|e| e.to_string())
}

/// Copies every open sequence as multi-FASTA.
#[tauri::command]
pub fn copy_multifasta(app: AppHandle) -> Result<usize, String> {
    let text = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        let seqs: Vec<_> = store.iter().map(|(_, d)| d.seq.clone()).collect();
        (sc_format::fasta::write_multi(seqs.iter()), seqs.len())
    };
    app.clipboard()
        .write_text(text.0)
        .map_err(|e| e.to_string())?;
    Ok(text.1)
}

/// The Info panel: composition, Tm and translation of a selection
/// (`from..to`, 0-based half-open).
#[derive(Debug, Serialize)]
pub struct SelectionInfo {
    /// A count.
    pub a: u32,
    /// C count.
    pub c: u32,
    /// G count.
    pub g: u32,
    /// T count.
    pub t: u32,
    /// Other letters.
    pub other: u32,
    /// %GC.
    pub gc_percent: f64,
    /// Tm in °C.
    pub tm: f64,
    /// Translation from the first selected base.
    pub peptide: String,
    /// Amino acids, stops excluded.
    pub size_aa: usize,
    /// MW in Da.
    pub mw: f64,
}

/// Computes the Info panel for a selection. With `first_orf` the peptide
/// stops at the first stop codon.
#[tauri::command]
pub fn selection_info(
    app: AppHandle,
    id: DocId,
    from: usize,
    to: usize,
    first_orf: bool,
) -> Result<SelectionInfo, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let doc = store.get(id).ok_or("no such document")?;
    let n = doc.seq.len();
    let (from, to) = (from.min(n), to.min(n).max(from.min(n)));
    let sel = &doc.seq.bases[from..to];
    let counts = BaseCounts::of(sel);
    let peptide = STANDARD.translate(sel, !first_orf);
    Ok(SelectionInfo {
        a: counts.a,
        c: counts.c,
        g: counts.g,
        t: counts.t,
        other: counts.other,
        gc_percent: counts.gc_percent(),
        tm: melting_temperature(&counts, DEFAULT_NA_MOLAR),
        size_aa: peptide_len(peptide.as_bytes()),
        mw: protein_mw(peptide.as_bytes()),
        peptide,
    })
}

/// A Find result.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Hit {
    /// 1-based position of the first base on the top strand.
    pub position: usize,
    /// Length in bases.
    pub length: usize,
    /// Strand.
    pub forward: bool,
}

fn search(bases: &[u8], topology: Topology, pattern: &[u8]) -> Vec<Hit> {
    let n = bases.len();
    let len = pattern.len();
    let mut hits = Vec::new();
    if len == 0 || n == 0 || (n < len && !topology.is_circular()) {
        return hits;
    }
    let mut extended = bases.to_vec();
    if topology.is_circular() {
        extended.extend_from_slice(&bases[..n.min(len - 1)]);
    }
    let rc = reverse_complement(pattern);
    let both = !is_palindromic(pattern);
    let last = if topology.is_circular() {
        n
    } else {
        n - len + 1
    };
    for pos in 0..last {
        let w = &extended[pos..pos + len];
        if matches_at(pattern, w) {
            hits.push(Hit {
                position: pos + 1,
                length: len,
                forward: true,
            });
        }
        if both && matches_at(&rc, w) {
            hits.push(Hit {
                position: pos + 1,
                length: len,
                forward: false,
            });
        }
    }
    hits
}

/// Find ▸ Sequence: every occurrence of `query` (IUPAC, case-insensitive) on
/// both strands, wrapping the origin of circular sequences.
#[tauri::command]
pub fn find_text(app: AppHandle, id: DocId, query: String) -> Result<Vec<Hit>, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let doc = store.get(id).ok_or("no such document")?;
    let pattern: Vec<u8> = query.bytes().filter(|b| b.is_ascii_alphabetic()).collect();
    Ok(search(&doc.seq.bases, doc.seq.topology, &pattern))
}

/// Find ▸ Site: every site of an enzyme.
#[tauri::command]
pub fn find_site(app: AppHandle, id: DocId, enzyme: String) -> Result<Vec<Hit>, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let doc = store.get(id).ok_or("no such document")?;
    let enzyme = state.library.get(&enzyme).ok_or("unknown enzyme")?;
    Ok(
        sc_enzyme::find_sites(&doc.seq.bases, doc.seq.topology, enzyme)
            .into_iter()
            .map(|s| Hit {
                position: s.position + 1,
                length: enzyme.site.len(),
                forward: s.strand == Strand::Forward,
            })
            .collect(),
    )
}

/// An ORF for the Find drawer.
#[derive(Debug, Clone, Serialize)]
pub struct OrfHit {
    /// 1-based first base of the start codon.
    pub start: usize,
    /// 1-based last base.
    pub end: usize,
    /// Strand.
    pub forward: bool,
    /// Frame 1–3.
    pub frame: u8,
    /// Length in bases.
    pub length: usize,
    /// False if no stop codon was reached (shown in italic).
    pub complete: bool,
}

/// Find ▸ ORFs: every ORF of at least `min_codons`, longest first.
#[tauri::command]
pub fn find_orfs_cmd(app: AppHandle, id: DocId, min_codons: usize) -> Result<Vec<OrfHit>, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let doc = store.get(id).ok_or("no such document")?;
    Ok(find_orfs(
        &doc.seq.bases,
        doc.seq.topology,
        &STANDARD,
        min_codons.max(1),
    )
    .into_iter()
    .map(|o| OrfHit {
        start: o.start as usize,
        end: o.end as usize,
        forward: o.strand == Strand::Forward,
        frame: o.frame,
        length: o.length as usize,
        complete: o.complete,
    })
    .collect())
}

/// One enzyme in a chooser list, with its cut count in the document.
#[derive(Debug, Clone, Serialize)]
pub struct EnzymeEntry {
    /// Name.
    pub name: String,
    /// Site with the cut marked.
    pub site: String,
    /// Number of cuts (bold when 1, italic when 0).
    pub count: usize,
}

/// The enzyme list with per-document cut counts (Find ▸ Site).
#[tauri::command]
pub fn enzyme_list(app: AppHandle, id: DocId) -> Result<Vec<EnzymeEntry>, String> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let doc = store.get(id).ok_or("no such document")?;
    let mut list: Vec<EnzymeEntry> =
        sc_enzyme::site_usage(&doc.seq.bases, doc.seq.topology, &state.library)
            .into_iter()
            .map(|u| EnzymeEntry {
                name: u.enzyme.name.clone(),
                site: u.enzyme.display_site(),
                count: u.count,
            })
            .collect();
    list.sort_by_key(|e| e.name.to_ascii_lowercase());
    Ok(list)
}

/// File ▸ Open Recent entries.
#[tauri::command]
pub fn recent_files(app: AppHandle) -> Vec<String> {
    app.state::<AppState>()
        .recent
        .lock()
        .unwrap()
        .paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

/// Sequence ▸ Show File in Finder / Explorer.
#[tauri::command]
pub fn reveal_file(app: AppHandle, id: DocId) {
    let path = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        store.get(id).and_then(|d| d.path.clone())
    };
    if let Some(p) = path {
        windows::reveal(&p);
    }
}

/// Records which document was used last (for the Toolbar and the menu bar,
/// whose clicks move focus away from the Sequence Window).
#[tauri::command]
pub fn note_focus(app: AppHandle, id: DocId) {
    *app.state::<AppState>().last_doc.lock().unwrap() = Some(id);
}

/// Toolbar buttons act on the front-most Sequence Window.
#[tauri::command]
pub async fn toolbar_action(app: AppHandle, action: String) -> Result<(), String> {
    crate::run_menu_action(&app, &action);
    Ok(())
}

/// Autosave now (called when a window is about to lose its document).
#[tauri::command]
pub fn autosave_now(app: AppHandle) {
    autosave::snapshot(&app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_both_strands_and_origin() {
        let hits = search(b"ccGAATTCccACGTAC", Topology::Linear, b"gaattc");
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].position, hits[0].forward), (3, true));
        // Non-palindromic: found on the reverse strand, reported at its top-strand position.
        let hits = search(b"ccGGTCTCcc", Topology::Linear, b"gagacc");
        assert_eq!((hits[0].position, hits[0].forward), (3, false));
        let hits = search(b"TTCggGAA", Topology::Circular, b"GAATTC");
        assert_eq!((hits[0].position, hits[0].forward), (6, true));
        assert!(search(b"TTCggGAA", Topology::Linear, b"GAATTC").is_empty());
    }
}
