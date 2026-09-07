//! The Tauri shell of SerialClon.rs.
//!
//! This crate is deliberately thin (`PLAN.md` §3): commands, the window manager,
//! preferences, recent files and autosave. All the science lives in the
//! UI-free `sc-*` crates.
//!
//! Every Serial Cloner window is a real OS window (`WebviewWindow`); the
//! [`windows`] module owns their labels, sizes and lifecycle.

mod commands;
mod windows;

use tauri::{Manager, WindowEvent};

/// Builds and runs the application. Called from `main.rs`.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            windows::ensure_help(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                match window.label() {
                    // The Help window is permanent: closing it only hides it
                    // (Window ▸ Show/Hide Help Window brings it back).
                    windows::HELP => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    // Until the menu bar lands (M2), closing the Toolbar quits.
                    windows::TOOLBAR => window.app_handle().exit(0),
                    _ => {}
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::toggle_help_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SerialClon");
}
