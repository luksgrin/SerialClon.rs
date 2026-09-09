//! Crash recovery: unsaved documents are written to the app data directory
//! every autosave interval (Preferences) and offered back at the next start (changelog 2.0:
//! "Automatic recovery of unsaved sequences in (hopefully rare) case of
//! crashing").

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::AppState;
use crate::documents::{DocId, Document};

#[derive(Serialize, Deserialize)]
struct Meta {
    name: String,
    path: Option<PathBuf>,
    locked: bool,
}

fn dir(app: &AppHandle) -> Option<PathBuf> {
    let d = app.path().app_data_dir().ok()?.join("autosave");
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

fn files(dir: &std::path::Path, id: DocId) -> (PathBuf, PathBuf) {
    (
        dir.join(format!("doc-{id}.xdna")),
        dir.join(format!("doc-{id}.json")),
    )
}

/// Writes every dirty document.
pub fn snapshot(app: &AppHandle) {
    let Some(dir) = dir(app) else { return };
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    for (id, doc) in store.iter() {
        let (xdna, json) = files(&dir, id);
        if doc.dirty {
            let meta = Meta {
                name: doc.seq.name.clone(),
                path: doc.path.clone(),
                locked: doc.locked,
            };
            let _ = std::fs::write(&xdna, sc_format::xdna::write(&doc.seq));
            let _ = std::fs::write(&json, serde_json::to_vec(&meta).unwrap_or_default());
        } else {
            let _ = std::fs::remove_file(&xdna);
            let _ = std::fs::remove_file(&json);
        }
    }
}

/// Drops the snapshot of a document that was saved or closed.
pub fn discard(app: &AppHandle, id: DocId) {
    if let Some(dir) = dir(app) {
        let (xdna, json) = files(&dir, id);
        let _ = std::fs::remove_file(xdna);
        let _ = std::fs::remove_file(json);
    }
}

/// Loads every snapshot left by a previous run into the store (marked dirty)
/// and deletes the files. Returns the new ids.
pub fn recover(app: &AppHandle) -> Vec<DocId> {
    let Some(dir) = dir(app) else {
        return Vec::new();
    };
    let mut recovered = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return recovered;
    };
    let mut metas: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    metas.sort();
    for json in metas {
        let xdna = json.with_extension("xdna");
        let loaded = (|| {
            let meta: Meta = serde_json::from_slice(&std::fs::read(&json).ok()?).ok()?;
            let mut seq = sc_format::xdna::read(&std::fs::read(&xdna).ok()?, &meta.name).ok()?;
            seq.name = meta.name;
            let mut doc = Document::new(seq, meta.path);
            doc.dirty = true;
            doc.locked = meta.locked;
            Some(doc)
        })();
        let _ = std::fs::remove_file(&json);
        let _ = std::fs::remove_file(&xdna);
        if let Some(doc) = loaded {
            let id = app.state::<AppState>().store.lock().unwrap().insert(doc);
            recovered.push(id);
        }
    }
    recovered
}

/// Starts the periodic snapshot thread.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            let secs = crate::prefs::get(&app).autosave_secs.max(5);
            std::thread::sleep(Duration::from_secs(secs));
            snapshot(&app);
        }
    });
}
