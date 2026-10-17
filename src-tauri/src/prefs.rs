//! Preferences (⌘,): every knob the original had (User Manual, changelog),
//! persisted to `prefs.json` in the app config directory and applied live —
//! every change is broadcast as a `prefs:changed` event and the windows
//! re-read. The original's "auto-check for update availability" lives in
//! `update.rs`.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::{AppState, menu, protein, windows};

/// Every preference. Unknown fields in an old `prefs.json` fall back to
/// their defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    // --- colours (Graphic Map, selection, features) ---
    /// Unique sites (yellow).
    pub color_unique: String,
    /// Double sites (orange).
    pub color_double: String,
    /// Particular sites (blue).
    pub color_particular: String,
    /// BEGIN site of a selection (green).
    pub color_begin: String,
    /// END site of a selection (red).
    pub color_end: String,
    /// Colour of a new feature.
    pub color_feature: String,
    // --- character sizes ---
    /// Sequence Window field.
    pub seq_font_size: u32,
    /// Graphic Map restriction-site names.
    pub gmap_re_font_size: u32,
    /// Graphic Map feature names.
    pub gmap_feature_font_size: u32,
    /// Sequence Map on screen.
    pub smap_char_size: u32,
    /// Sequence Map when printing.
    pub smap_print_size: u32,
    // --- Sequence Map defaults ---
    /// Nucleotides per line.
    pub smap_per_line: usize,
    /// Show the reverse strand.
    pub smap_show_reverse: bool,
    /// Amino acid under the middle base of the codon rather than the first.
    pub smap_aa_middle: bool,
    // --- Graphic Map ---
    /// Box the restriction-site names.
    pub gmap_box_re_names: bool,
    /// Box and colour unique-site names.
    pub gmap_box_unique: bool,
    // --- general ---
    /// Lock newly opened sequences.
    pub autolock: bool,
    /// Entries in File ▸ Open Recent (max 25).
    pub recent_files: usize,
    /// Warn when opening a file that is not in Serial Cloner format.
    pub warn_import: bool,
    /// Minimum ORF size for Find ▸ ORFs, in codons (amino acids); 0 = no
    /// limit, the original's default.
    pub orf_min_codons: usize,
    /// Start codons that open an ORF, `|`-separated: one of the original's
    /// four choices (`ATG`, `GTG`, `ATG|GTG`, `ATG|CTG|GTG|TTG`).
    pub orf_start_codons: String,
    /// Run a PCR: 3' A overhang by default.
    pub pcr_ta_overhang: bool,
    /// Gateway: "Use minimal att consensus" (the original's attDefCheckBox):
    /// sites are located with the short core consensus instead of the full
    /// definitions.
    pub gateway_minimal_att: bool,
    /// Autosave interval in seconds.
    pub autosave_secs: u64,
    /// Check GitHub for a new release at start (at most once a day).
    pub check_updates: bool,
    /// Manual proxy URL; empty = system settings.
    pub proxy: String,
    /// Vertical shift of the Toolbar, in pixels.
    pub toolbar_shift: i32,
    /// Limit the width of Sequence and Protein windows (2.6.1).
    pub limit_window_width: bool,
    /// `modern` or `aqua`.
    pub theme: String,
    /// Read-out speed for Sequence ▸ Read selection (0.5–2).
    pub read_speed: f64,
    // --- printing (File ▸ Page Setup) ---
    /// Landscape pages.
    pub page_landscape: bool,
    /// Page margin in mm.
    pub page_margin_mm: u32,
    // --- protein ---
    /// Use the selected codon-usage table as the genetic code (2.6).
    pub codon_as_code: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            color_unique: "#ffffaf".into(),
            color_double: "#ff9c07".into(),
            color_particular: "#7fbfff".into(),
            color_begin: "#54ba7f".into(),
            color_end: "#ff3f00".into(),
            color_feature: "#84a4c0".into(),
            seq_font_size: 12,
            gmap_re_font_size: 9,
            gmap_feature_font_size: 9,
            smap_char_size: 11,
            smap_print_size: 8,
            smap_per_line: 100,
            smap_show_reverse: true,
            smap_aa_middle: false,
            gmap_box_re_names: true,
            gmap_box_unique: true,
            autolock: true,
            recent_files: 10,
            warn_import: true,
            orf_min_codons: 0,
            orf_start_codons: "ATG".into(),
            pcr_ta_overhang: true,
            gateway_minimal_att: false,
            autosave_secs: 30,
            check_updates: true,
            proxy: String::new(),
            toolbar_shift: 0,
            limit_window_width: true,
            theme: "modern".into(),
            read_speed: 1.0,
            page_landscape: false,
            page_margin_mm: 12,
            codon_as_code: false,
        }
    }
}

impl Prefs {
    /// Clamps every numeric knob to a sane range.
    fn sanitised(mut self) -> Self {
        self.recent_files = self.recent_files.clamp(1, 25);
        self.seq_font_size = self.seq_font_size.clamp(8, 32);
        self.gmap_re_font_size = self.gmap_re_font_size.clamp(7, 24);
        self.gmap_feature_font_size = self.gmap_feature_font_size.clamp(7, 24);
        self.smap_char_size = self.smap_char_size.clamp(6, 24);
        self.smap_print_size = self.smap_print_size.clamp(4, 16);
        self.smap_per_line = self.smap_per_line.clamp(10, 500);
        self.orf_min_codons = self.orf_min_codons.clamp(0, 10_000);
        if !sc_seq::translate::ORF_START_OPTIONS.contains(&self.orf_start_codons.as_str()) {
            self.orf_start_codons = "ATG".into();
        }
        self.autosave_secs = self.autosave_secs.clamp(5, 3600);
        self.toolbar_shift = self.toolbar_shift.clamp(-200, 400);
        self.read_speed = self.read_speed.clamp(0.5, 2.0);
        self.page_margin_mm = self.page_margin_mm.clamp(0, 50);
        if self.theme != "aqua" {
            self.theme = "modern".into();
        }
        self
    }
}

/// Preferences state.
#[derive(Default)]
pub struct PrefsState {
    /// Current values.
    pub prefs: Mutex<Prefs>,
}

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("prefs.json"))
}

/// The current preferences (cloned).
pub fn get(app: &AppHandle) -> Prefs {
    app.state::<PrefsState>().prefs.lock().unwrap().clone()
}

/// Loads `prefs.json` at startup.
pub fn load(app: &AppHandle) {
    if let Some(f) = file(app)
        && let Ok(text) = std::fs::read_to_string(f)
        && let Ok(p) = serde_json::from_str::<Prefs>(&text)
    {
        *app.state::<PrefsState>().prefs.lock().unwrap() = p.sanitised();
    }
}

fn save(app: &AppHandle) {
    let Some(f) = file(app) else { return };
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let p = app.state::<PrefsState>().prefs.lock().unwrap().clone();
    let _ = std::fs::write(f, serde_json::to_string_pretty(&p).unwrap_or_default());
}

/// Replaces the preferences, persists them, applies the side effects and
/// tells every window.
pub fn apply(app: &AppHandle, new: Prefs) {
    let new = new.sanitised();
    let old = {
        let state = app.state::<PrefsState>();
        let mut p = state.prefs.lock().unwrap();
        std::mem::replace(&mut *p, new.clone())
    };
    save(app);
    if old.recent_files != new.recent_files {
        app.state::<AppState>()
            .recent
            .lock()
            .unwrap()
            .set_cap(new.recent_files);
        menu::refresh_recent(app);
    }
    if old.toolbar_shift != new.toolbar_shift {
        windows::apply_toolbar_shift(app, new.toolbar_shift);
    }
    if old.limit_window_width != new.limit_window_width {
        windows::apply_width_limit(app, new.limit_window_width);
    }
    if old.proxy != new.proxy {
        protein::reset_net(app);
    }
    if old.codon_as_code != new.codon_as_code {
        menu::refresh_protein(app);
    }
    let _ = app.emit("prefs:changed", &new);
}

/// Applies the startup-time effects (Toolbar position) and starts the
/// update check.
pub fn apply_startup(app: &AppHandle) {
    let p = get(app);
    if p.toolbar_shift != 0 {
        windows::apply_toolbar_shift(app, p.toolbar_shift);
    }
    if p.check_updates {
        crate::update::start_daily_check(app);
    }
}

/// Preferences window: the current values.
#[tauri::command]
pub fn get_prefs(app: AppHandle) -> Prefs {
    get(&app)
}

/// Preferences window: applies new values.
#[tauri::command]
pub fn set_prefs(app: AppHandle, prefs: Prefs) -> Prefs {
    apply(&app, prefs);
    get(&app)
}

/// Reset Preferences.
#[tauri::command]
pub fn reset_prefs(app: AppHandle) -> Prefs {
    apply(&app, Prefs::default());
    get(&app)
}

/// The Preferences window's Reset button has a command-line twin,
/// `--reset-prefs`, standing in for the original's Ctrl+Shift-at-launch
/// (modifier state is not readable before the event loop starts).
pub fn reset_from_argv(app: &AppHandle) -> bool {
    if std::env::args().any(|a| a == "--reset-prefs") {
        apply(app, Prefs::default());
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_survive_a_partial_file_and_are_clamped() {
        let p: Prefs =
            serde_json::from_str(r#"{"recent_files": 99, "theme": "weird", "seq_font_size": 2}"#)
                .unwrap();
        let p = p.sanitised();
        assert_eq!(p.recent_files, 25);
        assert_eq!(p.theme, "modern");
        assert_eq!(p.seq_font_size, 8);
        // The original's unique-site colour, RGB(255,255,175).
        assert_eq!(p.color_unique, "#ffffaf");
        assert!(p.warn_import);
        // ORF preferences: no size limit by default; an unknown start-codon
        // choice falls back to ATG.
        assert_eq!(p.orf_min_codons, 0);
        let q: Prefs = serde_json::from_str(r#"{"orf_start_codons": "TTG"}"#).unwrap();
        assert_eq!(q.sanitised().orf_start_codons, "ATG");
        let q: Prefs = serde_json::from_str(r#"{"orf_start_codons": "ATG|GTG"}"#).unwrap();
        assert_eq!(q.sanitised().orf_start_codons, "ATG|GTG");
    }
}
