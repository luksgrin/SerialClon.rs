//! The command surface exposed to the webviews.
//!
//! Commands are coarse-grained and document-oriented (`PLAN.md` §3). M0 only
//! ships what the Toolbar and Help windows need.

use serde::Serialize;
use tauri::{AppHandle, Manager, path::BaseDirectory};

use crate::windows;

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
    let library = app
        .path()
        .resolve("RELibrary", BaseDirectory::Resource)
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|text| sc_enzyme::library_info(&text))
        .unwrap_or(sc_enzyme::LibraryInfo {
            rebase_release: None,
            enzyme_count: 0,
        });
    let package = app.package_info();
    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        tauri_version: tauri::VERSION.to_string(),
        rebase_release: library.rebase_release,
        enzyme_count: library.enzyme_count,
    }
}

/// Window ▸ Show/Hide Help Window. Returns `true` if the window is now visible.
#[tauri::command]
pub fn toggle_help_window(app: AppHandle) -> Result<bool, String> {
    windows::toggle_help(&app).map_err(|e| e.to_string())
}
