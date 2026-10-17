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
mod cloning;
mod collections;
mod commands;
mod documents;
mod io;
mod menu;
mod msa;
mod prefs;
mod protein;
mod recent;
mod update;
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
    /// The document window last focused (label), so Toolbar buttons such as
    /// Print and Extract reach a map when that is what the user was using.
    pub last_window: Mutex<Option<String>>,
    /// File ▸ Open Recent.
    pub recent: Mutex<recent::Recent>,
    /// Submenus rebuilt at run time.
    pub menus: Mutex<Option<menu::MenuHandles>>,
    /// The enzyme database: the bundled REBASE file plus the user's additions.
    pub library: std::sync::RwLock<sc_enzyme::Library>,
    /// Names of the enzymes the user added (Enzyme Library).
    pub user_enzymes: Mutex<Vec<String>>,
    /// Enzymes unchecked in the Enzyme Library: hidden from maps and lists.
    pub hidden_enzymes: Mutex<std::collections::HashSet<String>>,
    /// Feature collections (Manage Features).
    pub collections: Mutex<Vec<sc_features::Collection>>,
}

impl AppState {
    /// Read access to the enzyme library.
    pub fn library(&self) -> std::sync::RwLockReadGuard<'_, sc_enzyme::Library> {
        self.library.read().unwrap()
    }
}

/// The enzyme database compiled in (also bundled as a resource for the
/// Enzyme Library window and `scripts/update-rebase.sh`).
const RELIBRARY: &str = include_str!("../../resources/RELibrary");

/// The focused document window, else the one last focused (the Toolbar and
/// the menu bar steal focus when clicked).
fn focused_doc_window(app: &AppHandle) -> Option<(DocWindow, DocId)> {
    let focused = app
        .webview_windows()
        .iter()
        .filter(|(_, w)| w.is_focused().unwrap_or(false))
        .find_map(|(label, _)| windows::parse_label(label));
    focused.or_else(|| {
        let last = app.state::<AppState>().last_window.lock().unwrap().clone();
        last.and_then(|l| windows::parse_label(&l))
            .filter(|(_, id)| {
                app.state::<AppState>()
                    .store
                    .lock()
                    .unwrap()
                    .get(*id)
                    .is_some()
            })
    })
}

/// The document a menu command targets: the focused document window's,
/// else the last one used.
fn current_doc(app: &AppHandle) -> Option<DocId> {
    if let Some((_, id)) = focused_doc_window(app) {
        return Some(id);
    }
    let state = app.state::<AppState>();
    let last = *state.last_doc.lock().unwrap();
    last.filter(|id| state.store.lock().unwrap().get(*id).is_some())
}

/// Quits after offering to save every dirty document.
fn request_quit(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        if settle_documents(&app) {
            app.exit(0);
        }
    });
}

/// Offers to save every dirty document, then drops the autosave copies.
/// `false` if the user cancelled (or a save failed): nothing may quit.
/// Blocks on dialogs, so never call it on the main thread.
pub(crate) fn settle_documents(app: &AppHandle) -> bool {
    let dirty = app.state::<AppState>().store.lock().unwrap().dirty_ids();
    for id in dirty {
        windows::open_sequence(app, id);
        match io::confirm_close(app, id) {
            io::CloseDecision::Save => match io::save(app, id, false) {
                Ok(true) => {}
                _ => return false,
            },
            io::CloseDecision::Discard => {
                if let Some(d) = app.state::<AppState>().store.lock().unwrap().get_mut(id) {
                    d.dirty = false;
                }
            }
            io::CloseDecision::Cancel => return false,
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
        autosave::discard(app, id);
    }
    true
}

/// Runs a menu-bar or Toolbar action by id.
/// The label of the window that has keyboard focus, else the last document
/// window used (the menu bar and the Toolbar take focus when clicked).
fn focused_window_label(app: &AppHandle) -> Option<String> {
    app.webview_windows()
        .iter()
        .find(|(_, w)| w.is_focused().unwrap_or(false))
        .map(|(label, _)| label.clone())
        .or_else(|| app.state::<AppState>().last_window.lock().unwrap().clone())
}

pub(crate) fn run_menu_action(app: &AppHandle, id: &str) {
    let doc = current_doc(app);
    let handle = app.clone();
    // Edit ▸ Undo / Redo / Select All / Cut / Copy / Paste act on the window
    // that has focus, as the original's standard Edit menu did: a Sequence
    // Window handles them itself, every other window applies them to its
    // focused text field (`src/lib/edit-menu.ts`).
    if matches!(
        id,
        "edit.undo" | "edit.redo" | "edit.select_all" | "edit.cut" | "edit.copy" | "edit.paste"
    ) && let Some(label) = focused_window_label(app)
        && !matches!(windows::parse_label(&label), Some((DocWindow::Sequence, _)))
        && label != windows::TOOLBAR
    {
        let _ = app.emit_to(label.as_str(), "menu", id);
        return;
    }
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
        "app.preferences" => windows::open_reference(app, "prefs"),
        "file.page_setup" => windows::open_reference(app, "page-setup"),
        "features.manage" => windows::open_reference(app, "features"),
        "generate.construct" => windows::open_reference(app, "construct"),
        "generate.adaptor" => windows::open_reference(app, "adaptor"),
        "generate.pcr" => windows::open_reference(app, "pcr"),
        "generate.shrna" => windows::open_reference(app, "shrna"),
        "generate.gateway" => windows::open_reference(app, "gateway"),
        "restriction.library" => windows::open_reference(app, "enzyme-library"),
        "generate.align" | "protein.align" => windows::open_reference(app, "align"),
        "generate.align_multiple" => windows::open_reference(app, "msa"),
        "tools.web" => protein::open_web(app, None),
        "file.new_protein" => {
            protein::new_protein(app.clone());
        }
        "file.open_protein" => {
            tauri::async_runtime::spawn(protein::open_protein_files(handle));
        }
        "protein.codon_usage" | "protein.import_codon" => {
            windows::open_reference(app, "codon-usage")
        }
        _ if id.starts_with("code.") => {
            if let Ok(code) = id["code.".len()..].parse::<u8>() {
                let _ = protein::set_genetic_code(app.clone(), code);
            }
        }
        _ if id.starts_with("pk.") => {
            protein::set_pk_table(app.clone(), id["pk.".len()..].to_owned())
        }
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
            let focused = focused_doc_window(app).map(|(k, _)| k);
            // Map windows handle their own Save As (picture), Print and Extract.
            if matches!(
                focused,
                Some(DocWindow::GraphicMap | DocWindow::SequenceMap)
            ) && matches!(
                id,
                "file.save_as"
                    | "file.print"
                    | "generate.extract"
                    | "seq.topology"
                    | "edit.copy"
                    | "edit.select_all"
            ) && let Some((kind, _)) = focused_doc_window(app)
            {
                let label = if kind == DocWindow::GraphicMap {
                    windows::gmap_label(doc_id)
                } else {
                    windows::smap_label(doc_id)
                };
                let _ = app.emit_to(label.as_str(), "menu", id);
                return;
            }
            match id {
                "restriction.graphic_map" => windows::open_graphic_map(app, doc_id),
                "restriction.site_usage" | "restriction.site_usage_table" => {
                    windows::open_usage(app, doc_id)
                }
                "tools.virtual_cutter" => windows::open_cutter(app, doc_id),
                "seq.bisulfite_sense" | "seq.bisulfite_antisense" => {
                    let _ = protein::bisulfite(app.clone(), doc_id, id.ends_with("antisense"));
                }
                "protein.blast" | "seq.blast" => {
                    let protein = app
                        .state::<AppState>()
                        .store
                        .lock()
                        .unwrap()
                        .get(doc_id)
                        .is_some_and(|d| d.seq.seq_type == sc_seq::SeqType::Protein);
                    protein::open_web(
                        app,
                        Some((
                            doc_id,
                            0,
                            0,
                            if protein {
                                "blastp".into()
                            } else {
                                "blastn".into()
                            },
                        )),
                    );
                }
                "features.scan" | "protein.scan" => {
                    let _ = commands::scan_features(app.clone(), doc_id);
                }
                "features.remove_duplicates" => {
                    let _ =
                        commands::edit(app.clone(), doc_id, documents::EditOp::RemoveDuplicates);
                }
                "features.from_selection" if matches!(focused, Some(DocWindow::GraphicMap)) => {
                    let _ = app.emit_to(windows::gmap_label(doc_id).as_str(), "menu", id);
                }
                "file.save" | "file.save_as" => {
                    let save_as = id == "file.save_as";
                    std::thread::spawn(move || {
                        if io::save(&handle, doc_id, save_as).is_ok() {
                            commands::broadcast(&handle, doc_id);
                        }
                    });
                }
                "file.export_fasta" | "file.export_genbank" | "file.export_snapgene" => {
                    let format = match id {
                        "file.export_fasta" => io::ExportFormat::Fasta,
                        "file.export_genbank" => io::ExportFormat::GenBank,
                        _ => io::ExportFormat::SnapGene,
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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            store: Mutex::new(DocumentStore::default()),
            last_doc: Mutex::new(None),
            last_window: Mutex::new(None),
            recent: Mutex::new(recent::Recent::default()),
            menus: Mutex::new(None),
            library: std::sync::RwLock::new(sc_enzyme::Library::parse(RELIBRARY)),
            user_enzymes: Mutex::new(Vec::new()),
            hidden_enzymes: Mutex::new(std::collections::HashSet::new()),
            collections: Mutex::new(Vec::new()),
        })
        .manage(protein::ProteinState::default())
        .manage(prefs::PrefsState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            prefs::load(&handle);
            prefs::reset_from_argv(&handle);
            *handle.state::<AppState>().recent.lock().unwrap() = recent::Recent::load(&handle);
            *handle.state::<AppState>().collections.lock().unwrap() = collections::load(&handle);
            cloning::load_user_settings(&handle);
            protein::load(&handle);
            let menus = menu::build(&handle)?;
            *handle.state::<AppState>().menus.lock().unwrap() = Some(menus);
            menu::refresh_recent(&handle);
            menu::refresh_protein(&handle);
            windows::ensure_help(&handle)?;
            // The Toolbar is the window the session starts in.
            if let Some(toolbar) = handle.get_webview_window(windows::TOOLBAR) {
                let _ = toolbar.set_focus();
            }
            for id in autosave::recover(&handle) {
                windows::open_sequence(&handle, id);
                commands::broadcast(&handle, id);
            }
            menu::refresh_windows(&handle);
            autosave::start(handle.clone());
            prefs::apply_startup(&handle);
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
                    Some((
                        DocWindow::Find
                        | DocWindow::GraphicMap
                        | DocWindow::SequenceMap
                        | DocWindow::Usage
                        | DocWindow::Cutter
                        | DocWindow::Silent,
                        _,
                    )) => {}
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
                        *app.state::<AppState>().last_window.lock().unwrap() = None;
                        autosave::discard(&app, id);
                        windows::close_find(&app, id);
                        windows::close_maps(&app, id);
                        menu::refresh_windows(&app);
                        let state = app.state::<AppState>();
                        let mut last = state.last_doc.lock().unwrap();
                        if *last == Some(id) {
                            *last = None;
                        }
                    }
                }
                WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                    let paths: Vec<std::path::PathBuf> = paths
                        .iter()
                        .filter(|p| p.is_file() && io::is_sequence_file(p))
                        .cloned()
                        .collect();
                    if !paths.is_empty() {
                        std::thread::spawn(move || {
                            io::open_paths(&app, &paths);
                        });
                    }
                }
                WindowEvent::Focused(true) => {
                    if let Some((_, id)) = windows::parse_label(&label) {
                        let state = app.state::<AppState>();
                        *state.last_doc.lock().unwrap() = Some(id);
                        *state.last_window.lock().unwrap() = Some(label.clone());
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
            prefs::get_prefs,
            prefs::set_prefs,
            prefs::reset_prefs,
            update::check_for_updates_now,
            commands::print_window,
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
            commands::map_data,
            commands::sequence_map,
            commands::extract_fragment,
            commands::extract_selection,
            commands::open_graphic_map,
            commands::open_sequence_map,
            commands::save_png,
            commands::show_parent,
            cloning::preview_fragments,
            cloning::ligate_construct,
            cloning::run_pcr,
            cloning::check_primer,
            cloning::pcr_window_open,
            cloning::make_adaptor,
            cloning::shrna_layouts,
            cloning::save_shrna_layouts,
            cloning::make_shrna,
            cloning::gateway,
            cloning::site_usage,
            cloning::library_entries,
            cloning::isoschizomers,
            cloning::compatible_ends,
            cloning::add_enzyme,
            cloning::remove_enzyme,
            cloning::set_enzymes_hidden,
            cloning::virtual_cut,
            cloning::silent_map,
            cloning::apply_silent_mutation,
            cloning::open_tool_window,
            cloning::visible_enzymes,
            protein::genetic_codes,
            protein::set_genetic_code,
            protein::pk_tables,
            protein::set_pk_table,
            protein::codon_tables,
            protein::codon_table_rows,
            protein::select_codon_table,
            protein::import_codon_table,
            protein::delete_codon_table,
            protein::new_protein,
            protein::open_protein_files,
            protein::peptide_to_document,
            protein::reverse_translate,
            protein::bisulfite,
            protein::translate_frames,
            protein::align_documents,
            protein::align_consensus,
            msa::msa_candidates,
            msa::align_multiple,
            msa::msa_text,
            msa::extract_msa_consensus,
            msa::export_msa,
            protein::web_fetch,
            protein::web_create_document,
            protein::blast_submit,
            protein::blast_status,
            protein::blast_result,
            commands::scan_features,
            commands::new_feature_from_selection,
            commands::new_feature_from_sites,
            commands::list_collections,
            commands::save_collections,
            commands::restore_builtin_collections,
            commands::import_collection,
            commands::export_collection,
        ])
        .build(tauri::generate_context!())
        .expect("error while building SerialClon")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let RunEvent::Opened { urls } = &event {
                let paths: Vec<std::path::PathBuf> = urls
                    .iter()
                    .filter_map(|u| u.to_file_path().ok())
                    .filter(|p| p.is_file() && io::is_sequence_file(p))
                    .collect();
                if !paths.is_empty() {
                    let handle = app.clone();
                    std::thread::spawn(move || {
                        io::open_paths(&handle, &paths);
                    });
                }
            }
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
