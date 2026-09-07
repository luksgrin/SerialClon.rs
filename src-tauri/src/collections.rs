//! Feature collections on disk: `{app data}/features/*.json`. The built-in
//! five are written there on first run so the user can edit them; "Restore
//! built-in collections" puts them back.

use std::path::PathBuf;

use sc_features::{Collection, builtin_collections};
use tauri::{AppHandle, Manager};

fn dir(app: &AppHandle) -> Option<PathBuf> {
    let d = app.path().app_data_dir().ok()?.join("features");
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_owned();
    if s.is_empty() { "collection".into() } else { s }
}

/// Loads every collection, seeding the built-ins when the directory is empty.
pub fn load(app: &AppHandle) -> Vec<Collection> {
    let Some(dir) = dir(app) else {
        return builtin_collections();
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|it| {
            it.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "json"))
                .collect()
        })
        .unwrap_or_default();
    if files.is_empty() {
        let builtins = builtin_collections();
        save(app, &builtins);
        return builtins;
    }
    files.sort();
    let mut out: Vec<Collection> = files
        .iter()
        .filter_map(|f| std::fs::read_to_string(f).ok())
        .filter_map(|t| Collection::from_json(&t).ok())
        .collect();
    // Built-ins first, in their canonical order, then the user's.
    let order = ["Generic", "Yeast", "Other", "Tags & Cut", "Gateway"];
    out.sort_by_key(|c| {
        order
            .iter()
            .position(|o| *o == c.name)
            .unwrap_or(usize::MAX)
    });
    out
}

/// Writes every collection, removing files of collections that are gone.
pub fn save(app: &AppHandle, collections: &[Collection]) {
    let Some(dir) = dir(app) else { return };
    let mut keep = Vec::new();
    for (i, c) in collections.iter().enumerate() {
        let file = dir.join(format!("{i:02}-{}.json", slug(&c.name)));
        let _ = std::fs::write(&file, c.to_json());
        keep.push(file);
    }
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "json") && !keep.contains(&p) {
                let _ = std::fs::remove_file(p);
            }
        }
    }
}

/// Restores the five built-in collections, keeping the user's own.
pub fn restore_builtins(current: &[Collection]) -> Vec<Collection> {
    let mut out = builtin_collections();
    out.extend(current.iter().filter(|c| !c.builtin).cloned());
    out
}
