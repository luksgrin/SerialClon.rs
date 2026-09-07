//! Window manager: one `WebviewWindow` per Serial Cloner window.
//!
//! Labels are stable identifiers used by capabilities, events and commands.
//! Singleton windows (Toolbar, Help, the reference cards) have fixed labels;
//! per-document windows are `seq-{id}` and `find-{id}`. The Find window is a
//! drawer: it sits at the right edge of its Sequence Window and follows it.

use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::AppState;
use crate::documents::DocId;

/// The always-on-top icon strip. Declared in `tauri.conf.json`.
pub const TOOLBAR: &str = "toolbar";
/// The permanent contextual Help window.
pub const HELP: &str = "help";

/// Logical size of the Help window, roughly the original's proportions.
const HELP_SIZE: (f64, f64) = (360.0, 170.0);
/// Logical size of a new Sequence Window (reference/ui/01-sequence-window.png).
const SEQUENCE_SIZE: (f64, f64) = (640.0, 470.0);
/// Logical size of the Find drawer.
const FIND_SIZE: (f64, f64) = (320.0, 440.0);

/// Kinds of per-document windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocWindow {
    /// The Sequence Window.
    Sequence,
    /// The Find drawer.
    Find,
}

/// Label of a document's Sequence Window.
pub fn seq_label(id: DocId) -> String {
    format!("seq-{id}")
}

/// Label of a document's Find drawer.
pub fn find_label(id: DocId) -> String {
    format!("find-{id}")
}

/// Which document window a label denotes, if any.
pub fn parse_label(label: &str) -> Option<(DocWindow, DocId)> {
    if let Some(id) = label.strip_prefix("seq-") {
        return id.parse().ok().map(|id| (DocWindow::Sequence, id));
    }
    if let Some(id) = label.strip_prefix("find-") {
        return id.parse().ok().map(|id| (DocWindow::Find, id));
    }
    None
}

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
    toggle(&window)
}

/// Window ▸ Show/Hide Toolbar.
pub fn toggle_toolbar(app: &AppHandle) -> tauri::Result<bool> {
    match app.get_webview_window(TOOLBAR) {
        Some(w) => toggle(&w),
        None => Ok(false),
    }
}

fn toggle(window: &WebviewWindow) -> tauri::Result<bool> {
    if window.is_visible()? {
        window.hide()?;
        Ok(false)
    } else {
        window.show()?;
        Ok(true)
    }
}

/// Opens (or focuses) the Sequence Window of a document.
pub fn open_sequence(app: &AppHandle, id: DocId) {
    let label = seq_label(id);
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.set_focus();
        return;
    }
    let name = {
        let state = app.state::<AppState>();
        let store = state.store.lock().unwrap();
        store
            .get(id)
            .map(|d| d.seq.name.clone())
            .unwrap_or_default()
    };
    // Cascade new windows so they do not stack exactly.
    let open = app
        .webview_windows()
        .keys()
        .filter(|l| l.starts_with("seq-"))
        .count() as f64;
    let offset = 28.0 * (open % 10.0);
    let result = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App(format!("src/windows/sequence/index.html?id={id}").into()),
    )
    .title(name)
    .inner_size(SEQUENCE_SIZE.0, SEQUENCE_SIZE.1)
    .min_inner_size(520.0, 360.0)
    .position(60.0 + offset, 130.0 + offset)
    .build();
    if result.is_ok() {
        *app.state::<AppState>().last_doc.lock().unwrap() = Some(id);
    }
}

/// Sequence Window ▸ Find: opens the drawer, or closes it if already open
/// (pressing Find again closes it, as in the original).
pub fn toggle_find(app: &AppHandle, id: DocId) -> bool {
    let label = find_label(id);
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.close();
        return false;
    }
    let Some(parent) = app.get_webview_window(&seq_label(id)) else {
        return false;
    };
    let title = parent.title().unwrap_or_default();
    let built = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App(format!("src/windows/find/index.html?id={id}").into()),
    )
    .title(format!("Find — {title}"))
    .inner_size(FIND_SIZE.0, FIND_SIZE.1)
    .min_inner_size(260.0, 300.0)
    .minimizable(false)
    .maximizable(false)
    .build();
    if built.is_ok() {
        position_find(app, id);
        true
    } else {
        false
    }
}

/// Snaps the Find drawer to the right edge of its Sequence Window.
pub fn position_find(app: &AppHandle, id: DocId) {
    let (Some(parent), Some(find)) = (
        app.get_webview_window(&seq_label(id)),
        app.get_webview_window(&find_label(id)),
    ) else {
        return;
    };
    if let (Ok(pos), Ok(size)) = (parent.outer_position(), parent.outer_size()) {
        let _ = find.set_position(PhysicalPosition::new(pos.x + size.width as i32, pos.y));
        let scale = parent.scale_factor().unwrap_or(1.0);
        let _ = find.set_size(LogicalSize::new(FIND_SIZE.0, size.height as f64 / scale));
    }
}

/// Closes the drawer of a document, if open.
pub fn close_find(app: &AppHandle, id: DocId) {
    if let Some(w) = app.get_webview_window(&find_label(id)) {
        let _ = w.close();
    }
}

/// The reference cards and About box (Window menu).
pub fn open_reference(app: &AppHandle, which: &str) {
    let (title, size) = match which {
        "about" => ("About SerialClon", (380.0, 300.0)),
        "genetic-code" => ("Standard Genetic Code", (460.0, 400.0)),
        "alphabet" => ("DNA Alphabet", (320.0, 380.0)),
        _ => return,
    };
    if let Some(w) = app.get_webview_window(which) {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(
        app,
        which,
        WebviewUrl::App(format!("src/windows/{which}/index.html").into()),
    )
    .title(title)
    .inner_size(size.0, size.1)
    .resizable(false)
    .minimizable(false)
    .maximizable(false)
    .build();
}

/// Window ▸ Bring All To Front.
pub fn bring_all_to_front(app: &AppHandle) {
    for (_, w) in app.webview_windows() {
        if w.is_minimized().unwrap_or(false) {
            let _ = w.unminimize();
        }
        if w.is_visible().unwrap_or(false) {
            let _ = w.set_focus();
        }
    }
}

/// Sequence ▸ Show File in Finder / Explorer.
pub fn reveal(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
    #[cfg(target_os = "linux")]
    if let Some(dir) = path.parent() {
        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    }
}
