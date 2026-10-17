//! Preferences (⌘,): every knob the original had (User Manual, changelog),
//! persisted to `prefs.json` in the app config directory and applied live —
//! every change is broadcast as a `prefs:changed` event and the windows
//! re-read. Also the original's "auto-check for update availability".

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;

use crate::{AppState, menu, protein, windows};

/// The GitHub repository queried for new releases.
pub const REPO: &str = "luksgrin/SerialClon.rs";
/// Minimum time between two update checks.
const UPDATE_CHECK_SECS: u64 = 24 * 3600;

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
            color_unique: "#f2e14c".into(),
            color_double: "#f0a53a".into(),
            color_particular: "#7cb3e8".into(),
            color_begin: "#2ea043".into(),
            color_end: "#d1242f".into(),
            color_feature: "#84a4c0".into(),
            seq_font_size: 13,
            gmap_re_font_size: 11,
            gmap_feature_font_size: 10,
            smap_char_size: 12,
            smap_print_size: 8,
            smap_per_line: 100,
            smap_show_reverse: true,
            smap_aa_middle: false,
            gmap_box_re_names: true,
            gmap_box_unique: true,
            autolock: false,
            recent_files: 10,
            warn_import: true,
            orf_min_codons: 0,
            orf_start_codons: "ATG".into(),
            pcr_ta_overhang: false,
            autosave_secs: 30,
            check_updates: true,
            proxy: String::new(),
            toolbar_shift: 0,
            limit_window_width: false,
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
        let handle = app.clone();
        std::thread::spawn(move || {
            if due(&handle) {
                let _ = check_for_updates(&handle, false);
            }
        });
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

fn stamp_file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("update-check.json"))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// True if no check ran in the last day.
fn due(app: &AppHandle) -> bool {
    let last: u64 = stamp_file(app)
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(0);
    now().saturating_sub(last) >= UPDATE_CHECK_SECS
}

/// What an update check found.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateReport {
    /// The version running.
    pub current: String,
    /// The latest release tag, if the check succeeded.
    pub latest: Option<String>,
    /// True if `latest` is newer than `current`.
    pub newer: bool,
    /// The release page.
    pub url: Option<String>,
    /// Why the check failed, if it did.
    pub error: Option<String>,
}

/// Queries GitHub for the latest release. Failures are silent unless
/// `interactive` (the Preferences button), in which case a dialog reports
/// the outcome either way. A newer release offers to open its page.
pub fn check_for_updates(app: &AppHandle, interactive: bool) -> UpdateReport {
    if let Some(f) = stamp_file(app) {
        let _ = std::fs::write(f, now().to_string());
    }
    let current = app.package_info().version.to_string();
    let result =
        protein::client(app).and_then(|c| c.latest_github_release(REPO).map_err(|e| e.to_string()));
    let report = match result {
        Ok(r) => UpdateReport {
            newer: sc_net::is_newer(&r.tag_name, &current),
            latest: Some(r.tag_name),
            url: Some(r.html_url),
            current,
            error: None,
        },
        Err(e) => UpdateReport {
            current,
            latest: None,
            newer: false,
            url: None,
            error: Some(e),
        },
    };
    if report.newer {
        let latest = report.latest.clone().unwrap_or_default();
        let ok = app
            .dialog()
            .message(format!(
                "SerialClon {latest} is available (you have {}).\nOpen the download page?",
                report.current
            ))
            .title("A new version is available")
            .kind(MessageDialogKind::Info)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Open".into(),
                "Later".into(),
            ))
            .blocking_show();
        if ok && let Some(url) = &report.url {
            let _ = app.opener().open_url(url, None::<&str>);
        }
    } else if interactive {
        let text = match &report.error {
            Some(e) => format!("Could not check for updates:\n{e}"),
            None => format!("SerialClon {} is up to date.", report.current),
        };
        app.dialog()
            .message(text)
            .title("Check for updates")
            .blocking_show();
    }
    report
}

/// Preferences ▸ Check now.
#[tauri::command]
pub async fn check_for_updates_now(app: AppHandle) -> UpdateReport {
    check_for_updates(&app, true)
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
        assert_eq!(p.color_unique, "#f2e14c");
        assert!(p.warn_import);
    }
}
