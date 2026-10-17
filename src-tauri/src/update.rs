//! The original's "auto-check for update availability", now with in-app
//! installs through the Tauri updater plugin.
//!
//! Once a day at start (Preferences ▸ Check for a new version) and on the
//! Preferences ▸ Check now button, the updater reads `latest.json` from the
//! latest GitHub Release. A newer release with the *same major version* is
//! offered for download and install (signature-checked by the plugin),
//! then the app relaunches. A new major version, a `.deb` install on Linux
//! (which cannot replace itself), or a release without `latest.json` gets
//! the old notice with a link to the release page instead.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::{prefs, protein, windows};

/// The GitHub repository queried for new releases.
pub const REPO: &str = "luksgrin/SerialClon.rs";
/// Minimum time between two automatic update checks.
const UPDATE_CHECK_SECS: u64 = 24 * 3600;
/// Release notes longer than this are cut in the dialog.
const NOTES_MAX_CHARS: usize = 1200;
/// The event carrying download progress to the Preferences window.
pub const PROGRESS_EVENT: &str = "update:progress";

/// What an update check found.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateReport {
    /// The version running.
    pub current: String,
    /// The latest release, if the check succeeded.
    pub latest: Option<String>,
    /// True if `latest` is newer than `current`.
    pub newer: bool,
    /// The release page.
    pub url: Option<String>,
    /// Why the check (or the install) failed, if it did.
    pub error: Option<String>,
}

/// Download progress, for the Preferences window's status line.
#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    /// What is happening, in words.
    pub text: String,
    /// 0..=1 while downloading with a known size.
    pub fraction: Option<f64>,
    /// True once the flow is over (installed, failed or cancelled).
    pub done: bool,
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

/// Startup: checks in the background if the last check is a day old.
pub fn start_daily_check(app: &AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || {
        if due(&handle) {
            let _ = check_for_updates(&handle, false);
        }
    });
}

/// Preferences ▸ Check now.
#[tauri::command]
pub async fn check_for_updates_now(app: AppHandle) -> UpdateReport {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || check_for_updates(&handle, true))
        .await
        .unwrap_or_else(|e| UpdateReport {
            current: app.package_info().version.to_string(),
            latest: None,
            newer: false,
            url: None,
            error: Some(e.to_string()),
        })
}

/// The major version of `MAJOR.MINOR.PATCH` (an optional leading `v`).
fn major(v: &str) -> Option<u64> {
    v.trim()
        .trim_start_matches('v')
        .split('.')
        .next()?
        .parse()
        .ok()
}

/// The policy: only a release of the running major version installs itself.
pub fn same_major(candidate: &str, current: &str) -> bool {
    matches!((major(candidate), major(current)), (Some(a), Some(b)) if a == b)
}

/// The release page of a version.
fn release_url(version: &str) -> String {
    format!(
        "https://github.com/{REPO}/releases/tag/v{}",
        version.trim_start_matches('v')
    )
}

/// Whether this install can replace itself. A Linux `.deb` cannot (the
/// package manager owns it); only the AppImage can.
fn can_self_update() -> bool {
    if cfg!(target_os = "linux") {
        std::env::var_os("APPIMAGE").is_some()
    } else {
        true
    }
}

/// Blocking: runs on a worker thread, never on the main thread. Failures
/// are silent unless `interactive`, in which case a dialog reports the
/// outcome either way.
pub fn check_for_updates(app: &AppHandle, interactive: bool) -> UpdateReport {
    if let Some(f) = stamp_file(app) {
        let _ = std::fs::write(f, now().to_string());
    }
    let current = app.package_info().version.to_string();

    let updater_error = if can_self_update() {
        match tauri::async_runtime::block_on(updater_check(app)) {
            Ok(Some(update)) => return found(app, update, current),
            Ok(None) => {
                if interactive {
                    info(app, format!("SerialClon.rs {current} is up to date."));
                }
                return UpdateReport {
                    current,
                    latest: None,
                    newer: false,
                    url: None,
                    error: None,
                };
            }
            // No `latest.json` (an older release) or the network: fall back
            // to the release page so a new version is still announced.
            Err(e) => Some(e),
        }
    } else {
        None
    };

    let report = legacy_check(app, current, updater_error);
    if report.newer {
        notice(app, &report);
    } else if interactive {
        match &report.error {
            Some(e) => error(app, format!("Could not check for updates:\n{e}")),
            None => info(
                app,
                format!("SerialClon.rs {} is up to date.", report.current),
            ),
        }
    }
    report
}

async fn updater_check(app: &AppHandle) -> Result<Option<Update>, String> {
    let mut builder = app.updater_builder();
    let proxy = prefs::get(app).proxy;
    if !proxy.trim().is_empty() {
        match proxy.trim().parse() {
            Ok(url) => builder = builder.proxy(url),
            Err(e) => return Err(format!("the proxy URL is not valid: {e}")),
        }
    }
    let updater = builder.build().map_err(|e| e.to_string())?;
    updater.check().await.map_err(|e| e.to_string())
}

/// The GitHub API check of 1.0.0: finds the latest release and its page.
fn legacy_check(app: &AppHandle, current: String, updater_error: Option<String>) -> UpdateReport {
    let result =
        protein::client(app).and_then(|c| c.latest_github_release(REPO).map_err(|e| e.to_string()));
    match result {
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
            error: Some(updater_error.unwrap_or(e)),
        },
    }
}

/// The updater found a newer release: install it, or announce it.
fn found(app: &AppHandle, update: Update, current: String) -> UpdateReport {
    let mut report = UpdateReport {
        latest: Some(update.version.clone()),
        newer: true,
        url: Some(release_url(&update.version)),
        current,
        error: None,
    };
    if !same_major(&update.version, &report.current) {
        notice(app, &report);
        return report;
    }
    if let Err(e) = offer_install(app, &update, &report.current) {
        progress(app, format!("Update failed: {e}"), None, true);
        error(app, format!("Could not install the update:\n{e}"));
        report.error = Some(e);
    }
    report
}

/// The old behaviour: a notice offering to open the release page.
fn notice(app: &AppHandle, report: &UpdateReport) {
    let latest = report.latest.clone().unwrap_or_default();
    let open = app
        .dialog()
        .message(format!(
            "SerialClon.rs {latest} is available (you have {}).\nOpen the download page?",
            report.current
        ))
        .title("A new version is available")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Open".into(),
            "Later".into(),
        ))
        .blocking_show();
    if open && let Some(url) = &report.url {
        let _ = app.opener().open_url(url, None::<&str>);
    }
}

/// Release notes for a dialog: trimmed and cut to a readable length.
fn short_notes(notes: Option<&str>) -> String {
    let notes = notes.unwrap_or("").trim();
    if notes.is_empty() {
        return String::new();
    }
    let mut out: String = notes.chars().take(NOTES_MAX_CHARS).collect();
    if notes.chars().count() > NOTES_MAX_CHARS {
        out.push_str("\n…");
    }
    format!("\n\nWhat's new:\n{out}")
}

/// Asks, downloads with progress, installs and relaunches. `Ok` if the user
/// declined or cancelled; `Err` with a plain reason if something failed.
fn offer_install(app: &AppHandle, update: &Update, current: &str) -> Result<(), String> {
    let yes = app
        .dialog()
        .message(format!(
            "SerialClon.rs {} is available (you have {current}). Download and install?{}",
            update.version,
            short_notes(update.body.as_deref()),
        ))
        .title("A new version is available")
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Download and Install".into(),
            "Later".into(),
        ))
        .blocking_show();
    if !yes {
        return Ok(());
    }
    // Progress shows in the Preferences window's status line.
    windows::open_reference(app, "prefs");
    progress(
        app,
        format!("Downloading {}…", update.version),
        Some(0.0),
        false,
    );

    let mut received: u64 = 0;
    let mut shown: i64 = -1;
    let bytes = tauri::async_runtime::block_on(update.download(
        |chunk, total| {
            received += chunk as u64;
            let fraction = total.filter(|t| *t > 0).map(|t| received as f64 / t as f64);
            let percent = fraction.map(|f| (f * 100.0) as i64).unwrap_or(-1);
            if percent != shown || fraction.is_none() {
                shown = percent;
                let text = match fraction {
                    Some(f) => format!("Downloading {}… {:.0} %", update.version, f * 100.0),
                    None => format!("Downloading {}… {} kB", update.version, received / 1024),
                };
                progress(app, text, fraction, false);
            }
        },
        || {},
    ))
    .map_err(|e| e.to_string())?;

    // Unsaved documents would be lost by the relaunch: settle them first.
    progress(
        app,
        "Downloaded. Waiting for unsaved documents…".into(),
        Some(1.0),
        false,
    );
    if !crate::settle_documents(app) {
        progress(app, "Update cancelled.".into(), None, true);
        return Ok(());
    }
    progress(
        app,
        format!("Installing {}…", update.version),
        Some(1.0),
        false,
    );
    // Windows: this launches the installer and exits the app.
    update.install(&bytes).map_err(|e| e.to_string())?;
    progress(app, "Installed. Restarting…".into(), Some(1.0), true);
    app.restart();
}

fn progress(app: &AppHandle, text: String, fraction: Option<f64>, done: bool) {
    let _ = app.emit(
        PROGRESS_EVENT,
        Progress {
            text,
            fraction,
            done,
        },
    );
}

fn info(app: &AppHandle, text: String) {
    app.dialog()
        .message(text)
        .title("Check for updates")
        .blocking_show();
}

fn error(app: &AppHandle, text: String) {
    app.dialog()
        .message(text)
        .title("Check for updates")
        .kind(MessageDialogKind::Error)
        .blocking_show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_same_major_installs_itself() {
        assert!(same_major("1.2.0", "1.0.0"));
        assert!(same_major("v1.0.1", "1.0.0"));
        assert!(!same_major("2.0.0", "1.9.9"));
        assert!(!same_major("garbage", "1.0.0"));
    }

    #[test]
    fn release_page_and_notes() {
        assert_eq!(
            release_url("1.1.0"),
            "https://github.com/luksgrin/SerialClon.rs/releases/tag/v1.1.0"
        );
        assert_eq!(short_notes(None), "");
        assert_eq!(short_notes(Some("  ")), "");
        let long = "x".repeat(NOTES_MAX_CHARS + 10);
        assert!(short_notes(Some(&long)).ends_with('…'));
    }
}
