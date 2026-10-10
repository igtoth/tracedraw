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
    /// Shift+Arrow distance (super nudge), mm.
    #[serde(default = "default_super_nudge")]
    pub super_nudge_mm: f64,
    /// Ctrl+Arrow distance (micro nudge), mm.
    #[serde(default = "default_micro_nudge")]
    pub micro_nudge_mm: f64,
    pub duplicate_offset_mm: [f64; 2],
    pub default_dpi: f64,
    pub snap: SnapPrefs,
    pub shortcuts: Vec<(String, String)>,
    pub last_dir: Option<PathBuf>,
    pub color: ColorPrefs,
    /// Show the Outline flyout in the toolbox (hidden by default).
    pub show_outline_flyout: bool,
    /// Tool shown by each toolbox group (tool ids, one per group): the one
    /// last used from its flyout.
    #[serde(default)]
    pub toolbox: Vec<String>,
    /// Default action of the mouse wheel: zoom (the target design's
    /// default) or scroll.
    #[serde(default = "default_true")]
    pub wheel_zooms: bool,
    /// Arrowheads created with Object > Create > Arrowhead.
    #[serde(default)]
    pub custom_arrowheads: Vec<tracedraw_core::Arrowhead>,
    /// Grid, ruler and guideline settings new drawings start with
    /// (Document Options > Save as Default).
    #[serde(default)]
    pub document_defaults: DocumentDefaults,
    /// Text > Writing Tools > Autocorrect options.
    #[serde(default)]
    pub autocorrect: crate::autocorrect::AutocorrectPrefs,
    /// Thesaurus file picked by the user (Text > Writing Tools > Thesaurus).
    #[serde(default)]
    pub thesaurus_file: Option<PathBuf>,
    /// Options > General: show the Create a New Document dialog for File >
    /// New (off: new drawings use the last settings at once).
    #[serde(default = "default_true")]
    pub show_new_document_dialog: bool,
    /// The Create a New Document dialog's last used settings.
    #[serde(default)]
    pub new_document: crate::new_document::NewDocSettings,
    /// Colour of the area around the page (Customization > Appearance).
    #[serde(default = "default_desktop")]
    pub desktop_rgb: [u8; 3],
    /// What the status bar's first field shows.
    #[serde(default)]
    pub status_info: crate::ui::status::StatusInfo,
    /// Toolbox flyouts hidden with the toolbox's "+" button (the id of
    /// each flyout's first tool).
    #[serde(default)]
    pub toolbox_hidden: Vec<String>,
    /// Options > General: what opens at start-up.
    #[serde(default)]
    pub startup: Startup,
    /// Options > General: undo levels.
    #[serde(default = "default_undo_levels")]
    pub undo_levels: usize,
    /// Options > Display.
    #[serde(default = "default_true")]
    pub show_tooltips: bool,
    #[serde(default)]
    pub hide_bbox_curve_tools: bool,
    #[serde(default = "default_true")]
    pub preview_page_border: bool,
    /// Options > Edit: Ctrl constrains rotation and line angles to
    /// multiples of this angle (degrees).
    #[serde(default = "default_constrain_angle")]
    pub constrain_angle: f64,
    /// Options > Edit: decimal places of distances in the property bar
    /// and the status bar.
    #[serde(default = "default_precision")]
    pub precision: u32,
    /// Options > Nodes and Handles.
    #[serde(default)]
    pub nodes: NodePrefs,
    /// Options > ClipFrame.
    #[serde(default)]
    pub clip_frame: ClipFramePrefs,
    /// Options > Save.
    #[serde(default)]
    pub backup: BackupPrefs,
    /// Options > Text: Ctrl+Numpad 8 and 2 change the font size by this
    /// many points.
    #[serde(default = "default_text_increment")]
    pub text_increment_pt: f64,
    /// Options > Tools > Pick: unfilled objects can be picked inside.
    #[serde(default)]
    pub treat_all_filled: bool,
    /// Options > Tools > Pick: the Pick tool's pointer is a cross hair.
    #[serde(default)]
    pub crosshair_cursor: bool,
    /// Options > Tools > Zoom/Pan: a right click with the Zoom tool zooms
    /// out (off: it opens the context menu).
    #[serde(default = "default_true")]
    pub zoom_right_click_out: bool,
    /// Options > Customization > Color Palette.
    #[serde(default)]
    pub palette: PalettePrefs,
}

fn default_desktop() -> [u8; 3] {
    [255, 255, 255]
}

fn default_true() -> bool {
    true
}

fn default_undo_levels() -> usize {
    150
}

fn default_constrain_angle() -> f64 {
    15.0
}

fn default_precision() -> u32 {
    3
}

fn default_text_increment() -> f64 {
    1.0
}

/// What opens when the application starts (Options > General).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Startup {
    #[default]
    WelcomeScreen,
    NewDocument,
    LastDocument,
}

impl Startup {
    pub const ALL: [Startup; 3] = [
        Startup::WelcomeScreen,
        Startup::NewDocument,
        Startup::LastDocument,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Startup::WelcomeScreen => "options.startup_welcome",
            Startup::NewDocument => "options.startup_new",
            Startup::LastDocument => "options.startup_last",
        }
    }
}

/// How nodes are drawn: size, shape per node type, colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NodeSize {
    #[default]
    Small,
    Medium,
    Large,
}

impl NodeSize {
    pub const ALL: [NodeSize; 3] = [NodeSize::Small, NodeSize::Medium, NodeSize::Large];

    /// Side of a node square, screen pixels.
    pub fn px(self) -> f32 {
        match self {
            NodeSize::Small => 7.0,
            NodeSize::Medium => 9.0,
            NodeSize::Large => 11.0,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            NodeSize::Small => "options.node_small",
            NodeSize::Medium => "options.node_medium",
            NodeSize::Large => "options.node_large",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeShape {
    Square,
    Circle,
    Diamond,
}

impl NodeShape {
    pub const ALL: [NodeShape; 3] = [NodeShape::Square, NodeShape::Circle, NodeShape::Diamond];
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NodePrefs {
    pub size: NodeSize,
    pub cusp: NodeShape,
    pub smooth: NodeShape,
    pub symmetrical: NodeShape,
    /// Selected nodes and handles.
    pub main_rgb: [u8; 3],
    /// The first node of a curve.
    pub secondary_rgb: [u8; 3],
    /// Unselected nodes are filled white (Ctrl+Shift+G).
    pub unselected_filled: bool,
    /// An arrow at the start node shows the curve's direction.
    pub show_direction: bool,
}

impl Default for NodePrefs {
    fn default() -> Self {
        NodePrefs {
            size: NodeSize::Small,
            cusp: NodeShape::Square,
            smooth: NodeShape::Circle,
            symmetrical: NodeShape::Diamond,
            main_rgb: [0x00, 0x8C, 0xFF],
            secondary_rgb: [0xFF, 0x00, 0x00],
            unselected_filled: true,
            show_direction: true,
        }
    }
}

/// When new ClipFrame content is centred in its frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AutoCenter {
    /// When the content lies completely outside the frame.
    #[default]
    WhenOutside,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClipFramePrefs {
    pub auto_center: AutoCenter,
    /// An empty frame shows crossing lines on screen.
    pub empty_lines: bool,
}

impl Default for ClipFramePrefs {
    fn default() -> Self {
        ClipFramePrefs {
            auto_center: AutoCenter::WhenOutside,
            empty_lines: true,
        }
    }
}

/// The colour palettes docked under the drawing window (Options >
/// Customization > Color Palette).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PalettePrefs {
    /// Colours applied to objects join the document palette.
    pub auto_update_document: bool,
    /// A right click on a swatch sets the outline colour (off: it opens
    /// the palette menu).
    pub right_click_outline: bool,
    /// The "No Color" well at the start of the palette.
    pub show_no_color: bool,
    /// The document palette row under the default palette.
    pub show_document: bool,
}

impl Default for PalettePrefs {
    fn default() -> Self {
        PalettePrefs {
            auto_update_document: true,
            right_click_outline: true,
            show_no_color: true,
            show_document: true,
        }
    }
}

/// Backups (Options > Save).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BackupPrefs {
    /// Copy the file being replaced to `backup_of_<name>` before saving.
    pub before_save: bool,
    /// Where those copies go; none: next to the original.
    pub before_save_dir: Option<PathBuf>,
    /// Save open drawings with unsaved changes every `minutes`.
    pub auto: bool,
    pub minutes: u32,
    /// Where auto-backups go; none: the temporary folder.
    pub auto_dir: Option<PathBuf>,
}

impl Default for BackupPrefs {
    fn default() -> Self {
        BackupPrefs {
            before_save: true,
            before_save_dir: None,
            auto: true,
            minutes: 20,
            auto_dir: None,
        }
    }
}

fn default_super_nudge() -> f64 {
    0.2 * 25.4
}

fn default_micro_nudge() -> f64 {
    0.01 * 25.4
}

/// Document settings saved as the default for new drawings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DocumentDefaults {
    pub grid: tracedraw_core::document::GridSettings,
    pub rulers: tracedraw_core::document::RulerSettings,
    pub guides: tracedraw_core::document::GuideSettings,
}

impl DocumentDefaults {
    pub fn of(doc: &tracedraw_core::Document) -> Self {
        DocumentDefaults {
            grid: doc.metadata.grid,
            rulers: doc.metadata.rulers,
            guides: doc.metadata.guides,
        }
    }

    pub fn apply(&self, doc: &mut tracedraw_core::Document) {
        doc.metadata.grid = self.grid;
        doc.metadata.rulers = self.rulers;
        doc.metadata.guides = self.guides;
    }
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
    /// Paths of ICC files loaded by the user (empty = built-in model).
    #[serde(default)]
    pub rgb_profile_path: String,
    #[serde(default)]
    pub cmyk_profile_path: String,
}

impl Default for ColorPrefs {
    fn default() -> Self {
        ColorPrefs {
            rgb_profile: "sRGB IEC61966-2.1".into(),
            cmyk_profile: "Generic CMYK (open)".into(),
            intent: "Relative colorimetric".into(),
            black_point_compensation: true,
            proof_profile: "Generic CMYK (open)".into(),
            rgb_profile_path: String::new(),
            cmyk_profile_path: String::new(),
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
            super_nudge_mm: default_super_nudge(),
            micro_nudge_mm: default_micro_nudge(),
            duplicate_offset_mm: [6.35, 6.35],
            default_dpi: 300.0,
            snap: SnapPrefs::default(),
            shortcuts: Vec::new(),
            last_dir: None,
            color: ColorPrefs::default(),
            show_outline_flyout: false,
            wheel_zooms: true,
            toolbox: Vec::new(),
            custom_arrowheads: Vec::new(),
            document_defaults: DocumentDefaults::default(),
            startup: Startup::default(),
            undo_levels: default_undo_levels(),
            show_tooltips: true,
            hide_bbox_curve_tools: false,
            preview_page_border: true,
            constrain_angle: default_constrain_angle(),
            precision: default_precision(),
            nodes: NodePrefs::default(),
            clip_frame: ClipFramePrefs::default(),
            backup: BackupPrefs::default(),
            text_increment_pt: default_text_increment(),
            treat_all_filled: false,
            crosshair_cursor: false,
            zoom_right_click_out: true,
            palette: PalettePrefs::default(),
            autocorrect: crate::autocorrect::AutocorrectPrefs::default(),
            thesaurus_file: None,
            show_new_document_dialog: true,
            new_document: crate::new_document::NewDocSettings::default(),
            desktop_rgb: default_desktop(),
            status_info: crate::ui::status::StatusInfo::default(),
            toolbox_hidden: Vec::new(),
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

/// Local storage key of the settings in the browser build.
#[cfg(target_arch = "wasm32")]
const WEB_KEY: &str = "tracedraw.settings";

impl Settings {
    pub fn path() -> Option<PathBuf> {
        config_dir().map(|d| d.join("settings.json"))
    }

    pub fn load() -> Settings {
        #[cfg(target_arch = "wasm32")]
        {
            // The browser build keeps settings in local storage.
            return crate::web::storage_get(WEB_KEY)
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
        }
        #[allow(unreachable_code)]
        let Some(p) = Self::path() else {
            return Settings::default();
        };
        match std::fs::read_to_string(&p) {
            Ok(s) => {
                let mut settings: Settings = serde_json::from_str(&s).unwrap_or_else(|e| {
                    log::warn!("settings unreadable ({e}); using defaults");
                    Settings::default()
                });
                // Files written before the start-up choice existed.
                if !s.contains("\"startup\"") && !settings.show_welcome_on_start {
                    settings.startup = Startup::NewDocument;
                }
                settings
            }
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            if let Ok(s) = serde_json::to_string(self) {
                crate::web::storage_set(WEB_KEY, &s);
            }
            return;
        }
        #[allow(unreachable_code)]
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
        // Browser uploads and downloads have no path to reopen.
        if crate::files::WEB {
            return;
        }
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
        assert!(
            partial.wheel_zooms,
            "old settings files keep the zoom default"
        );
        assert!(partial.snap.guides);
    }
}
