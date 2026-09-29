//! The menu bar (User Manual §III), with the original's shortcuts as
//! accelerators. Item ids are `menu.item`; document-scoped ones are forwarded
//! to the front-most Sequence Window as a `menu` event.

use tauri::menu::{
    AboutMetadataBuilder, CheckMenuItem, CheckMenuItemBuilder, Menu, MenuBuilder, MenuItem,
    MenuItemBuilder, PredefinedMenuItem, Submenu, SubmenuBuilder,
};
use tauri::{AppHandle, Manager, Wry};

use crate::AppState;

/// Submenus rebuilt at run time.
pub struct MenuHandles {
    /// File ▸ Open Recent.
    pub recent: Submenu<Wry>,
    /// The open-windows list at the end of the Window menu.
    pub windows: Submenu<Wry>,
    /// Protein ▸ Choose genetic code (check items, one per NCBI table).
    pub codes: Vec<CheckMenuItem<Wry>>,
    /// Protein ▸ pK table for pI (check items).
    pub pks: Vec<CheckMenuItem<Wry>>,
    /// Protein ▸ "Codon usage: …" (shows the selected table).
    pub codon_label: MenuItem<Wry>,
}

fn item(
    app: &AppHandle,
    id: &str,
    text: &str,
    accel: Option<&str>,
) -> tauri::Result<tauri::menu::MenuItem<Wry>> {
    let mut b = MenuItemBuilder::with_id(id, text);
    if let Some(a) = accel {
        b = b.accelerator(a);
    }
    b.build(app)
}

/// Builds and installs the menu bar.
pub fn build(app: &AppHandle) -> tauri::Result<MenuHandles> {
    let about = AboutMetadataBuilder::new()
        .name(Some("SerialClon.rs"))
        .version(Some(app.package_info().version.to_string()))
        .comments(Some(
            "An independent reimplementation of Serial Cloner. Not affiliated with SerialBasics.",
        ))
        .build();

    let recent = SubmenuBuilder::with_id(app, "file.recent", "Open Recent").build()?;
    let export = SubmenuBuilder::new(app, "Export")
        .item(&item(app, "file.export_fasta", "Export to FASTA…", None)?)
        .item(&item(
            app,
            "file.export_genbank",
            "Export to GenBank (with Features)…",
            None,
        )?)
        .item(&item(
            app,
            "file.export_snapgene",
            "Export to SnapGene (.dna)…",
            None,
        )?)
        .separator()
        .item(&item(
            app,
            "file.export_multifasta",
            "Save All Sequences To Multifasta…",
            None,
        )?)
        .build()?;
    let file = SubmenuBuilder::new(app, "File")
        .item(&item(app, "file.new", "New", Some("CmdOrCtrl+N"))?)
        .item(&item(
            app,
            "file.new_degenerate",
            "New Degenerate DNA",
            Some("CmdOrCtrl+Shift+N"),
        )?)
        .item(&item(app, "file.open", "Open…", Some("CmdOrCtrl+O"))?)
        .item(&recent)
        .separator()
        .item(&item(
            app,
            "file.new_protein",
            "New Protein Sequence",
            None,
        )?)
        .item(&item(
            app,
            "file.open_protein",
            "Open Protein Sequence…",
            None,
        )?)
        .separator()
        .item(&item(app, "file.revert", "Revert To Saved", None)?)
        .separator()
        .item(&item(app, "file.close", "Close", Some("CmdOrCtrl+W"))?)
        .item(&item(
            app,
            "file.close_all",
            "Close All",
            Some("CmdOrCtrl+Alt+W"),
        )?)
        .separator()
        .item(&item(app, "file.save", "Save", Some("CmdOrCtrl+S"))?)
        .item(&item(
            app,
            "file.save_as",
            "Save As…",
            Some("CmdOrCtrl+Shift+S"),
        )?)
        .item(&export)
        .separator()
        .item(&item(app, "file.page_setup", "Page Setup…", None)?)
        .item(&item(app, "file.print", "Print…", Some("CmdOrCtrl+P"))?)
        .build()?;

    let edit = SubmenuBuilder::new(app, "Edit")
        .item(&item(app, "edit.undo", "Undo", Some("CmdOrCtrl+Z"))?)
        .item(&item(app, "edit.redo", "Redo", Some("CmdOrCtrl+Y"))?)
        .separator()
        .item(&item(
            app,
            "edit.select_all",
            "Select All",
            Some("CmdOrCtrl+A"),
        )?)
        .item(&item(app, "edit.cut", "Cut", Some("CmdOrCtrl+X"))?)
        .item(&item(app, "edit.copy", "Copy", Some("CmdOrCtrl+C"))?)
        .item(&item(
            app,
            "edit.copy_fasta",
            "Copy as Fasta",
            Some("CmdOrCtrl+K"),
        )?)
        .item(&item(
            app,
            "edit.copy_formatted",
            "Copy Non Formatted",
            Some("CmdOrCtrl+Shift+K"),
        )?)
        .item(&item(
            app,
            "edit.copy_multifasta",
            "Copy All Sequences To Multifasta",
            Some("CmdOrCtrl+Shift+J"),
        )?)
        .item(&item(app, "edit.paste", "Paste", Some("CmdOrCtrl+V"))?)
        .separator()
        .item(&item(app, "edit.find", "Find…", Some("CmdOrCtrl+F"))?)
        .item(&item(
            app,
            "edit.find_orf",
            "Find ORF",
            Some("CmdOrCtrl+R"),
        )?)
        .build()?;

    let sequence = SubmenuBuilder::new(app, "Sequence")
        .item(&item(app, "seq.info", "Show Info…", Some("CmdOrCtrl+I"))?)
        .item(&item(
            app,
            "seq.copy_translation",
            "Copy Translation",
            Some("CmdOrCtrl+T"),
        )?)
        .separator()
        .item(&item(
            app,
            "seq.uppercase",
            "UPPERCASE",
            Some("CmdOrCtrl+U"),
        )?)
        .item(&item(
            app,
            "seq.lowercase",
            "Lowercase",
            Some("CmdOrCtrl+L"),
        )?)
        .item(&item(
            app,
            "seq.antiparallel",
            "Antiparallel",
            Some("CmdOrCtrl+/"),
        )?)
        .item(&item(app, "seq.reverse", "Reverse", None)?)
        .item(&item(app, "seq.complement", "Complement", None)?)
        .separator()
        .item(&item(app, "seq.topology", "Linearize / Circularize", None)?)
        .item(&item(app, "seq.set_origin", "Set Origin", None)?)
        .item(&item(app, "seq.read", "Read selection", None)?)
        .item(&item(
            app,
            "seq.degenerate",
            "Transform to / from Degenerate DNA",
            None,
        )?)
        .item(
            &SubmenuBuilder::new(app, "Bisulfite Conversion")
                .item(&item(
                    app,
                    "seq.bisulfite_sense",
                    "Sense Strand (C>>T)",
                    None,
                )?)
                .item(&item(
                    app,
                    "seq.bisulfite_antisense",
                    "Antisense Strand (C>>T)",
                    None,
                )?)
                .build()?,
        )
        .item(&item(app, "seq.blast", "BLAST at the NCBI…", None)?)
        .separator()
        .item(&item(app, "seq.rename", "Rename Sequence…", None)?)
        .item(&item(
            app,
            "seq.reveal",
            "Show File in Finder / Explorer",
            None,
        )?)
        .build()?;

    let restriction = SubmenuBuilder::new(app, "Restriction")
        .item(&item(
            app,
            "restriction.graphic_map",
            "Graphic Map",
            Some("CmdOrCtrl+G"),
        )?)
        .item(&item(
            app,
            "restriction.sequence_map",
            "Sequence Map",
            Some("CmdOrCtrl+M"),
        )?)
        .item(&item(
            app,
            "restriction.site_usage",
            "Site Usage (interactive)",
            None,
        )?)
        .item(&item(
            app,
            "restriction.site_usage_table",
            "Site Usage (table)",
            None,
        )?)
        .separator()
        .item(&item(app, "restriction.library", "Enzyme Library", None)?)
        .build()?;

    let generate = SubmenuBuilder::new(app, "Generate")
        .item(&item(
            app,
            "generate.construct",
            "Build a Construct…",
            Some("CmdOrCtrl+1"),
        )?)
        .item(&item(
            app,
            "generate.adaptor",
            "Create an Adaptor…",
            Some("CmdOrCtrl+2"),
        )?)
        .item(&item(
            app,
            "generate.pcr",
            "Run a PCR…",
            Some("CmdOrCtrl+3"),
        )?)
        .item(&item(
            app,
            "generate.shrna",
            "Make shRNA…",
            Some("CmdOrCtrl+4"),
        )?)
        .item(&item(
            app,
            "generate.gateway",
            "Gateway™ Cloning…",
            Some("CmdOrCtrl+5"),
        )?)
        .item(&item(
            app,
            "generate.extract",
            "Extract Fragment",
            Some("CmdOrCtrl+6"),
        )?)
        .item(&item(
            app,
            "generate.align",
            "Align two Sequences…",
            Some("CmdOrCtrl+7"),
        )?)
        .item(&item(
            app,
            "generate.align_multiple",
            "Align Multiple Sequences…",
            Some("CmdOrCtrl+8"),
        )?)
        .build()?;

    // Protein menu (changelog 2.5 / 2.6).
    let mut codes = Vec::new();
    let code_menu = SubmenuBuilder::new(app, "Choose Genetic Code");
    let mut code_menu = code_menu;
    for c in sc_seq::genetic_codes::GENETIC_CODES {
        let it = CheckMenuItemBuilder::with_id(
            format!("code.{}", c.id),
            format!("{} — {}", c.id, c.name),
        )
        .checked(c.id == 1)
        .build(app)?;
        code_menu = code_menu.item(&it);
        codes.push(it);
    }
    let code_menu = code_menu.build()?;
    let mut pks = Vec::new();
    let mut pk_menu = SubmenuBuilder::new(app, "pK Table for pI");
    for (i, t) in sc_seq::pi::PK_TABLES.iter().enumerate() {
        let it = CheckMenuItemBuilder::with_id(format!("pk.{}", t.name), t.name)
            .checked(i == 0)
            .build(app)?;
        pk_menu = pk_menu.item(&it);
        pks.push(it);
    }
    let pk_menu = pk_menu.build()?;
    let codon_label =
        MenuItemBuilder::with_id("protein.codon_usage", "Codon Usage: —").build(app)?;
    let protein = SubmenuBuilder::new(app, "Protein")
        .item(&item(
            app,
            "protein.reverse_translate",
            "Reverse Translate…",
            None,
        )?)
        .item(&item(
            app,
            "protein.scan",
            "Scan for Peptide Features",
            None,
        )?)
        .item(&item(app, "protein.align", "Protein Align…", None)?)
        .item(&item(
            app,
            "protein.blast",
            "BLAST Protein Sequence…",
            None,
        )?)
        .separator()
        .item(&codon_label)
        .item(&item(
            app,
            "protein.import_codon",
            "Import Codon Usage…",
            None,
        )?)
        .separator()
        .item(&code_menu)
        .item(&pk_menu)
        .build()?;

    let features = SubmenuBuilder::new(app, "Features")
        .item(&item(
            app,
            "features.from_selection",
            "New Feature From Selection",
            Some("CmdOrCtrl+Shift+2"),
        )?)
        .item(&item(
            app,
            "features.scan",
            "Scan Sequence",
            Some("CmdOrCtrl+Alt+S"),
        )?)
        .item(&item(
            app,
            "features.remove_duplicates",
            "Remove duplicated Features",
            None,
        )?)
        .separator()
        .item(&item(app, "features.manage", "Manage Features…", None)?)
        .build()?;

    let tools = SubmenuBuilder::new(app, "Tools")
        .item(&item(app, "tools.virtual_cutter", "Virtual Cutter…", None)?)
        .item(&item(app, "tools.silent_map", "Silent Map…", None)?)
        .separator()
        .item(&item(app, "tools.web", "Web Access…", None)?)
        .build()?;

    let windows = SubmenuBuilder::with_id(app, "window.list", "Open Windows").build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .item(&PredefinedMenuItem::minimize(app, None)?)
        .item(&item(
            app,
            "window.bring_front",
            "Bring All To Front",
            Some("CmdOrCtrl+B"),
        )?)
        .separator()
        .item(&item(app, "window.toolbar", "Show/Hide Toolbar", None)?)
        .item(&item(app, "window.help", "Show/Hide Help Window", None)?)
        .separator()
        .item(&item(
            app,
            "window.genetic_code",
            "Standard Genetic Code",
            None,
        )?)
        .item(&item(app, "window.alphabet", "DNA Alphabet", None)?)
        .item(&item(app, "window.about", "About SerialClon.rs…", None)?)
        .separator()
        .item(&windows)
        .build()?;

    let menu: Menu<Wry> = if cfg!(target_os = "macos") {
        let app_menu = SubmenuBuilder::new(app, "SerialClon.rs")
            .item(&PredefinedMenuItem::about(
                app,
                Some("About SerialClon.rs"),
                Some(about),
            )?)
            .separator()
            .item(&item(
                app,
                "app.preferences",
                "Preferences…",
                Some("CmdOrCtrl+,"),
            )?)
            .separator()
            .item(&PredefinedMenuItem::services(app, None)?)
            .separator()
            .item(&PredefinedMenuItem::hide(app, None)?)
            .item(&PredefinedMenuItem::hide_others(app, None)?)
            .item(&PredefinedMenuItem::show_all(app, None)?)
            .separator()
            .item(&item(
                app,
                "app.quit",
                "Quit SerialClon.rs",
                Some("CmdOrCtrl+Q"),
            )?)
            .build()?;
        MenuBuilder::new(app)
            .items(&[
                &app_menu,
                &file,
                &edit,
                &sequence,
                &restriction,
                &generate,
                &features,
                &protein,
                &tools,
                &window,
            ])
            .build()?
    } else {
        // Windows / Linux: Preferences and Quit live in File, About in Window.
        file.append(&PredefinedMenuItem::separator(app)?)?;
        file.append(&item(
            app,
            "app.preferences",
            "Preferences…",
            Some("CmdOrCtrl+,"),
        )?)?;
        file.append(&item(app, "app.quit", "Quit", Some("CmdOrCtrl+Q"))?)?;
        MenuBuilder::new(app)
            .items(&[
                &file,
                &edit,
                &sequence,
                &restriction,
                &generate,
                &features,
                &protein,
                &tools,
                &window,
            ])
            .build()?
    };
    app.set_menu(menu)?;
    Ok(MenuHandles {
        recent,
        windows,
        codes,
        pks,
        codon_label,
    })
}

/// Reflects the chosen genetic code, pK table and codon usage table in the
/// Protein menu.
pub fn refresh_protein(app: &AppHandle) {
    let state = app.state::<AppState>();
    let menus = state.menus.lock().unwrap();
    let Some(m) = menus.as_ref() else { return };
    let settings = app
        .state::<crate::protein::ProteinState>()
        .settings
        .lock()
        .unwrap()
        .clone();
    for it in &m.codes {
        let id = it
            .id()
            .as_ref()
            .strip_prefix("code.")
            .and_then(|s| s.parse::<u8>().ok());
        let _ = it.set_checked(id == Some(settings.genetic_code));
    }
    for it in &m.pks {
        let name = it.id().as_ref().strip_prefix("pk.").unwrap_or("");
        let _ = it.set_checked(name == settings.pk_table);
    }
    let _ = m.codon_label.set_text(format!(
        "Codon Usage: {}…",
        crate::protein::current_codon_table_name(app)
    ));
}

fn clear(submenu: &Submenu<Wry>) {
    while let Ok(Some(_)) = submenu.remove_at(0) {}
}

/// Rebuilds File ▸ Open Recent from the stored list.
pub fn refresh_recent(app: &AppHandle) {
    let state = app.state::<AppState>();
    let paths = state.recent.lock().unwrap().paths.clone();
    let menus = state.menus.lock().unwrap();
    let Some(m) = menus.as_ref() else { return };
    clear(&m.recent);
    for (i, p) in paths.iter().enumerate() {
        let text = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Ok(it) = item(app, &format!("file.recent.{i}"), &text, None) {
            let _ = m.recent.append(&it);
        }
    }
    if let Ok(sep) = PredefinedMenuItem::separator(app) {
        let _ = m.recent.append(&sep);
    }
    if let Ok(it) = MenuItemBuilder::with_id("file.clear_recent", "Clear Menu")
        .enabled(!paths.is_empty())
        .build(app)
    {
        let _ = m.recent.append(&it);
    }
}

/// Rebuilds the list of open Sequence Windows at the end of the Window menu.
pub fn refresh_windows(app: &AppHandle) {
    let state = app.state::<AppState>();
    let docs = state.store.lock().unwrap().list();
    let menus = state.menus.lock().unwrap();
    let Some(m) = menus.as_ref() else { return };
    clear(&m.windows);
    for (id, name) in docs {
        if let Ok(it) = item(app, &format!("window.doc.{id}"), &name, None) {
            let _ = m.windows.append(&it);
        }
    }
}
