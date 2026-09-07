//! The Tauri shell of SerialClon.rs.
//!
//! This crate is deliberately thin (`PLAN.md` §3): commands, the window manager,
//! the menu bar, recent files and autosave. All the science lives in the
//! UI-free `sc-*` crates.
//!
//! Every Serial Cloner window is a real OS window (`WebviewWindow`); the
//! [`windows`] module owns their labels, sizes and lifecycle. Sequence state
//! lives in the [`documents`] store; windows hold ids.

mod autosave;
mod commands;
mod documents;
mod io;
mod menu;
mod recent;
mod windows;

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};

use documents::{DocId, DocumentStore};
use windows::DocWindow;

/// Application state shared by commands, menu and window handlers.
pub struct AppState {
    /// Every open document.
    pub store: Mutex<DocumentStore>,
    /// The document last used, for the Toolbar and menu bar.
    pub last_doc: Mutex<Option<DocId>>,
    /// File ▸ Open Recent.
    pub recent: Mutex<recent::Recent>,
    /// Submenus rebuilt at run time.
    pub menus: Mutex<Option<menu::MenuHandles>>,
    /// The bundled enzyme database.
    pub library: sc_enzyme::Library,
}

/// The enzyme database compiled in (also bundled as a resource for the
/// Enzyme Library window and `scripts/update-rebase.sh`).
const RELIBRARY: &str = include_str!("../../resources/RELibrary");

/// The document a menu command targets: the focused Sequence Window, else
/// the last one used.
fn current_doc(app: &AppHandle) -> Option<DocId> {
    for (label, w) in app.webview_windows() {
        if let Some((DocWindow::Sequence, id)) = windows::parse_label(&label)
            && w.is_focused().unwrap_or(false)
        {
            return Some(id);
        }
    }
    let state = app.state::<AppState>();
    let last = *state.last_doc.lock().unwrap();
    last.filter(|id| state.store.lock().unwrap().get(*id).is_some())
}

/// Quits after offering to save every dirty document.
fn request_quit(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let dirty = app.state::<AppState>().store.lock().unwrap().dirty_ids();
        for id in dirty {
            windows::open_sequence(&app, id);
            match io::confirm_close(&app, id) {
                io::CloseDecision::Save => match io::save(&app, id, false) {
                    Ok(true) => {}
                    _ => return,
                },
                io::CloseDecision::Discard => {
                    if let Some(d) = app.state::<AppState>().store.lock().unwrap().get_mut(id) {
                        d.dirty = false;
                    }
                }
                io::CloseDecision::Cancel => return,
            }
        }
        let ids: Vec<DocId> = app
            .state::<AppState>()
            .store
            .lock()
            .unwrap()
            .list()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        for id in ids {
            autosave::discard(&app, id);
        }
        app.exit(0);
    });
}

/// Runs a menu-bar or Toolbar action by id.
pub(crate) fn run_menu_action(app: &AppHandle, id: &str) {
    let doc = current_doc(app);
    let handle = app.clone();
    match id {
        "file.new" => {
            commands::new_sequence(app.clone(), false);
        }
        "file.new_degenerate" => {
            commands::new_sequence(app.clone(), true);
        }
        "file.open" => {
            std::thread::spawn(move || {
                io::open_dialog(&handle);
            });
        }
        "file.clear_recent" => {
            app.state::<AppState>().recent.lock().unwrap().clear();
            menu::refresh_recent(app);
        }
        "file.close_all" => {
            for (label, w) in app.webview_windows() {
                if matches!(windows::parse_label(&label), Some((DocWindow::Sequence, _))) {
                    let _ = w.close();
                }
            }
        }
        "file.export_multifasta" => {
            std::thread::spawn(move || {
                let _ = io::export_multifasta(&handle);
            });
        }
        "edit.copy_multifasta" => {
            let _ = commands::copy_multifasta(app.clone());
        }
        "app.quit" => request_quit(app),
        "window.toolbar" => {
            let _ = windows::toggle_toolbar(app);
        }
        "window.help" => {
            let _ = windows::toggle_help(app);
        }
        "window.bring_front" => windows::bring_all_to_front(app),
        "window.genetic_code" => windows::open_reference(app, "genetic-code"),
        "window.alphabet" => windows::open_reference(app, "alphabet"),
        "window.about" => windows::open_reference(app, "about"),
        _ if id.starts_with("file.recent.") => {
            let index: usize = id["file.recent.".len()..].parse().unwrap_or(usize::MAX);
            let path = app
                .state::<AppState>()
                .recent
                .lock()
                .unwrap()
                .paths
                .get(index)
                .cloned();
            if let Some(p) = path {
                std::thread::spawn(move || {
                    io::open_paths(&handle, &[p]);
                });
            }
        }
        _ if id.starts_with("window.doc.") => {
            if let Ok(id) = id["window.doc.".len()..].parse::<DocId>() {
                windows::open_sequence(app, id);
            }
        }
        // Everything else needs a document.
        _ => {
            let Some(doc_id) = doc else { return };
            match id {
                "file.save" | "file.save_as" => {
                    let save_as = id == "file.save_as";
                    std::thread::spawn(move || {
                        if io::save(&handle, doc_id, save_as).is_ok() {
                            commands::broadcast(&handle, doc_id);
                        }
                    });
                }
                "file.export_fasta" | "file.export_genbank" => {
                    let format = if id == "file.export_fasta" {
                        io::ExportFormat::Fasta
                    } else {
                        io::ExportFormat::GenBank
                    };
                    std::thread::spawn(move || {
                        let _ = io::export(&handle, doc_id, format);
                    });
                }
                "file.revert" => {
                    std::thread::spawn(move || {
                        if io::revert(&handle, doc_id).is_ok() {
                            commands::broadcast(&handle, doc_id);
                        }
                    });
                }
                "file.close" => commands::close_document(app.clone(), doc_id),
                "edit.find" => {
                    windows::toggle_find(app, doc_id);
                }
                "seq.reveal" => commands::reveal_file(app.clone(), doc_id),
                // Selection-dependent commands are executed by the window.
                _ => {
                    let _ = app.emit_to(windows::seq_label(doc_id).as_str(), "menu", id);
                }
            }
        }
    }
}

/// Builds and runs the application. Called from `main.rs`.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppState {
            store: Mutex::new(DocumentStore::default()),
            last_doc: Mutex::new(None),
            recent: Mutex::new(recent::Recent::default()),
            menus: Mutex::new(None),
            library: sc_enzyme::Library::parse(RELIBRARY),
        })
        .setup(|app| {
            let handle = app.handle().clone();
            *handle.state::<AppState>().recent.lock().unwrap() = recent::Recent::load(&handle);
            let menus = menu::build(&handle)?;
            *handle.state::<AppState>().menus.lock().unwrap() = Some(menus);
            menu::refresh_recent(&handle);
            windows::ensure_help(&handle)?;
            for id in autosave::recover(&handle) {
                windows::open_sequence(&handle, id);
                commands::broadcast(&handle, id);
            }
            menu::refresh_windows(&handle);
            autosave::start(handle.clone());
            // Files given on the command line.
            let paths: Vec<std::path::PathBuf> = std::env::args()
                .skip(1)
                .map(Into::into)
                .filter(|p: &std::path::PathBuf| p.is_file() && io::is_sequence_file(p))
                .collect();
            if !paths.is_empty() {
                std::thread::spawn(move || {
                    io::open_paths(&handle, &paths);
                });
            }
            Ok(())
        })
        .on_menu_event(|app, event| run_menu_action(app, event.id().as_ref()))
        .on_window_event(|window, event| {
            let app = window.app_handle().clone();
            let label = window.label().to_owned();
            match event {
                WindowEvent::CloseRequested { api, .. } => match windows::parse_label(&label) {
                    Some((DocWindow::Sequence, id)) => {
                        let dirty = app
                            .state::<AppState>()
                            .store
                            .lock()
                            .unwrap()
                            .get(id)
                            .is_some_and(|d| d.dirty);
                        if dirty {
                            api.prevent_close();
                            let w = window.clone();
                            std::thread::spawn(move || match io::confirm_close(&app, id) {
                                io::CloseDecision::Save => {
                                    if let Ok(true) = io::save(&app, id, false) {
                                        let _ = w.close();
                                    }
                                }
                                io::CloseDecision::Discard => {
                                    if let Some(d) =
                                        app.state::<AppState>().store.lock().unwrap().get_mut(id)
                                    {
                                        d.dirty = false;
                                    }
                                    let _ = w.close();
                                }
                                io::CloseDecision::Cancel => {}
                            });
                        }
                    }
                    Some((DocWindow::Find, _)) => {}
                    None if label == windows::HELP => {
                        // The Help window is permanent: closing only hides it.
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    None if label == windows::TOOLBAR => {
                        api.prevent_close();
                        request_quit(&app);
                    }
                    None => {}
                },
                WindowEvent::Destroyed => {
                    if let Some((DocWindow::Sequence, id)) = windows::parse_label(&label) {
                        app.state::<AppState>().store.lock().unwrap().remove(id);
                        autosave::discard(&app, id);
                        windows::close_find(&app, id);
                        menu::refresh_windows(&app);
                        let state = app.state::<AppState>();
                        let mut last = state.last_doc.lock().unwrap();
                        if *last == Some(id) {
                            *last = None;
                        }
                    }
                }
                WindowEvent::Focused(true) => {
                    if let Some((_, id)) = windows::parse_label(&label) {
                        *app.state::<AppState>().last_doc.lock().unwrap() = Some(id);
                    }
                }
                WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                    if let Some((DocWindow::Sequence, id)) = windows::parse_label(&label) {
                        windows::position_find(&app, id);
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::toggle_help_window,
            commands::new_sequence,
            commands::open_files,
            commands::open_paths,
            commands::get_document,
            commands::list_documents,
            commands::edit,
            commands::undo,
            commands::redo,
            commands::paste,
            commands::save,
            commands::export,
            commands::revert,
            commands::close_document,
            commands::toggle_find_window,
            commands::open_reference_window,
            commands::clipboard_write,
            commands::clipboard_read,
            commands::copy_multifasta,
            commands::selection_info,
            commands::find_text,
            commands::find_site,
            commands::find_orfs_cmd,
            commands::enzyme_list,
            commands::recent_files,
            commands::reveal_file,
            commands::note_focus,
            commands::toolbar_action,
            commands::autosave_now,
        ])
        .build(tauri::generate_context!())
        .expect("error while building SerialClon")
        .run(|app, event| {
            if let RunEvent::ExitRequested { api, code, .. } = event
                && code.is_none()
                && !app
                    .state::<AppState>()
                    .store
                    .lock()
                    .unwrap()
                    .dirty_ids()
                    .is_empty()
            {
                api.prevent_exit();
                request_quit(app);
            }
        });
}
