//! User settings persisted between sessions: language, recent files,
//! workspace flags, tool defaults. Stored as JSON in the platform config
//! directory (`%APPDATA%\TraceDraw`, `~/Library/Application Support/TraceDraw`,
//! `$XDG_CONFIG_HOME/tracedraw`).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub recent_files: Vec<PathBuf>,
    pub workspace: String,
    pub show_welcome_on_start: bool,
    pub units: String,
    pub nudge_mm: f64,
    pub duplicate_offset_mm: [f64; 2],
    pub default_dpi: f64,
    pub snap: SnapPrefs,
    pub shortcuts: Vec<(String, String)>,
    pub last_dir: Option<PathBuf>,
    pub color: ColorPrefs,
    /// Show the Outline flyout in the toolbox (hidden by default).
    pub show_outline_flyout: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SnapPrefs {
    pub grid: bool,
    pub guides: bool,
    pub objects: bool,
    pub page: bool,
    pub threshold_px: f64,
}

impl Default for SnapPrefs {
    fn default() -> Self {
        SnapPrefs {
            grid: false,
            guides: true,
            objects: true,
            page: true,
            threshold_px: 10.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorPrefs {
    pub rgb_profile: String,
    pub cmyk_profile: String,
    pub intent: String,
    pub black_point_compensation: bool,
    pub proof_profile: String,
}

impl Default for ColorPrefs {
    fn default() -> Self {
        ColorPrefs {
            rgb_profile: "sRGB IEC61966-2.1".into(),
            cmyk_profile: "Generic CMYK (open)".into(),
            intent: "Relative colorimetric".into(),
            black_point_compensation: true,
            proof_profile: "Generic CMYK (open)".into(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: String::new(),
            recent_files: Vec::new(),
            workspace: "default".into(),
            show_welcome_on_start: true,
            units: "mm".into(),
            nudge_mm: 2.54,
            duplicate_offset_mm: [6.35, 6.35],
            default_dpi: 300.0,
            snap: SnapPrefs::default(),
            shortcuts: Vec::new(),
            last_dir: None,
            color: ColorPrefs::default(),
            show_outline_flyout: false,
        }
    }
}

pub fn config_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("TRACEDRAW_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("TraceDraw"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|p| PathBuf::from(p).join("Library/Application Support/TraceDraw"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
            .map(|p| p.join("tracedraw"))
    }
}

impl Settings {
    pub fn path() -> Option<PathBuf> {
        config_dir().map(|d| d.join("settings.json"))
    }

    pub fn load() -> Settings {
        let Some(p) = Self::path() else {
            return Settings::default();
        };
        match std::fs::read_to_string(&p) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
                log::warn!("settings unreadable ({e}); using defaults");
                Settings::default()
            }),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self) {
        let Some(p) = Self::path() else {
            return;
        };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match serde_json::to_string_pretty(self) {
            Ok(s) => {
                if let Err(e) = std::fs::write(&p, s) {
                    log::warn!("could not save settings: {e}");
                }
            }
            Err(e) => log::warn!("could not serialise settings: {e}"),
        }
    }

    /// Move or insert `path` at the front of the recent list (max 10).
    pub fn touch_recent(&mut self, path: &std::path::Path) {
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_path_buf());
        self.recent_files.truncate(10);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_list_dedups_and_caps() {
        let mut s = Settings::default();
        for i in 0..12 {
            s.touch_recent(std::path::Path::new(&format!("/f{i}.tdraw")));
        }
        s.touch_recent(std::path::Path::new("/f11.tdraw"));
        assert_eq!(s.recent_files.len(), 10);
        assert_eq!(s.recent_files[0], PathBuf::from("/f11.tdraw"));
    }

    #[test]
    fn round_trip_json() {
        let s = Settings::default();
        let j = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&j).unwrap();
        assert_eq!(back.nudge_mm, s.nudge_mm);
        let partial: Settings = serde_json::from_str("{\"language\":\"de\"}").unwrap();
        assert_eq!(partial.language, "de");
        assert!(partial.snap.guides);
    }
}
