//! File ▸ Open Recent, persisted in the app config directory.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

/// Upper bound of the Preferences setting.
pub const MAX_RECENT: usize = 25;

/// The recent-files list.
#[derive(Default)]
pub struct Recent {
    /// Most recent first.
    pub paths: Vec<PathBuf>,
    file: Option<PathBuf>,
}

impl Recent {
    /// Loads the list from `recent.json`, or an empty list.
    pub fn load(app: &AppHandle) -> Recent {
        let file = app
            .path()
            .app_config_dir()
            .ok()
            .map(|d| d.join("recent.json"));
        let paths = file
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|s| serde_json::from_str::<Vec<PathBuf>>(&s).ok())
            .unwrap_or_default();
        Recent { paths, file }
    }

    /// Moves `path` to the front, keeping `cap` entries.
    pub fn push(&mut self, path: &Path, cap: usize) {
        self.paths.retain(|p| p != path);
        self.paths.insert(0, path.to_path_buf());
        self.paths.truncate(cap.clamp(1, MAX_RECENT));
        self.save();
    }

    /// Preferences changed the number of entries.
    pub fn set_cap(&mut self, cap: usize) {
        self.paths.truncate(cap.clamp(1, MAX_RECENT));
        self.save();
    }

    /// Open Recent ▸ Clear Menu.
    pub fn clear(&mut self) {
        self.paths.clear();
        self.save();
    }

    fn save(&self) {
        if let Some(file) = &self.file {
            if let Some(dir) = file.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_string_pretty(&self.paths) {
                let _ = std::fs::write(file, json);
            }
        }
    }
}
