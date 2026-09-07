//! Window manager: one `WebviewWindow` per Serial Cloner window.
//!
//! Labels are stable identifiers used by capabilities, events and commands.
//! Singleton windows (Toolbar, Help) get fixed labels; per-document windows
//! (Sequence Window, maps, drawers) will be `"{kind}-{document id}"` from M2 on.

use tauri::{
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

/// The always-on-top icon strip. Declared in `tauri.conf.json`.
pub const TOOLBAR: &str = "toolbar";
/// The permanent contextual Help window.
pub const HELP: &str = "help";

/// Logical size of the Help window, roughly the original's proportions.
const HELP_SIZE: (f64, f64) = (360.0, 170.0);

/// Returns the Help window, creating it (bottom-right of the primary monitor,
/// like the original) if it does not exist yet.
pub fn ensure_help(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(HELP) {
        return Ok(window);
    }
    let window = WebviewWindowBuilder::new(
        app,
        HELP,
        WebviewUrl::App("src/windows/help/index.html".into()),
    )
    .title("Help")
    .inner_size(HELP_SIZE.0, HELP_SIZE.1)
    .min_inner_size(240.0, 120.0)
    .always_on_top(true)
    .minimizable(false)
    .maximizable(false)
    .build()?;

    if let Ok(Some(monitor)) = app.primary_monitor() {
        let scale = monitor.scale_factor();
        let size = monitor.size();
        let margin = (24.0 * scale) as i32;
        let frame = window.outer_size()?;
        let x = monitor.position().x + size.width as i32 - frame.width as i32 - margin;
        let y = monitor.position().y + size.height as i32 - frame.height as i32 - margin * 3;
        let _ = window.set_position(PhysicalPosition::new(x.max(0), y.max(0)));
    }
    Ok(window)
}

/// Shows the Help window if hidden, hides it if visible. Returns the new state.
pub fn toggle_help(app: &AppHandle) -> tauri::Result<bool> {
    let window = ensure_help(app)?;
    if window.is_visible()? {
        window.hide()?;
        Ok(false)
    } else {
        window.show()?;
        Ok(true)
    }
}
