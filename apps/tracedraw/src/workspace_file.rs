//! Workspace files (`.tdws`): the user's own setup of the window, kept in
//! a file to carry to another computer or to go back to later. A file
//! holds the parts chosen when it was written (keyboard shortcuts; where
//! the bars sit and which show, with the toolbox; the colour palette
//! options; the interface size, desktop colour and page border), and an
//! import applies the parts chosen then that the file has. Files are
//! JSON; anything odd in one is dropped or set back to its default, never
//! trusted.

use crate::app::{App, Workspace};
use crate::i18n::{tr, trf};
use crate::settings::{PalettePrefs, ShownBars};
use crate::ui::bars::BarLayout;
use crate::ui::status::StatusInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// What every workspace file says it is.
pub const KIND: &str = "tracedraw-workspace";
/// The version written. Files of later versions are read as far as their
/// parts are understood.
pub const VERSION: u32 = 1;
/// The file name extension.
pub const EXTENSION: &str = "tdws";
/// Larger files are not workspace files.
const MAX_FILE: usize = 4 << 20;
/// At most this many shortcuts are taken from a file.
const MAX_SHORTCUTS: usize = 4096;
/// Ids, keys and tool names are short.
const MAX_TEXT: usize = 64;
/// Toolbox lists are short.
const MAX_LIST: usize = 256;
/// Floating bars lie within this distance of the origin (points).
const MAX_POSITION: f32 = 100_000.0;

/// The parts a workspace file can hold; kept in the preferences so the
/// choice is remembered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceParts {
    pub shortcuts: bool,
    pub bars: bool,
    pub palette: bool,
    pub appearance: bool,
}

impl Default for WorkspaceParts {
    fn default() -> Self {
        WorkspaceParts::ALL
    }
}

impl WorkspaceParts {
    pub const ALL: WorkspaceParts = WorkspaceParts {
        shortcuts: true,
        bars: true,
        palette: true,
        appearance: true,
    };

    /// Each part with the i18n key of its name, in the order they are
    /// listed.
    pub fn flags_mut(&mut self) -> [(&mut bool, &'static str); 4] {
        [
            (&mut self.shortcuts, PART_SHORTCUTS),
            (&mut self.bars, PART_BARS),
            (&mut self.palette, PART_PALETTE),
            (&mut self.appearance, PART_APPEARANCE),
        ]
    }

    pub fn any(&self) -> bool {
        self.shortcuts || self.bars || self.palette || self.appearance
    }
}

const PART_SHORTCUTS: &str = "workspace.part_shortcuts";
const PART_BARS: &str = "workspace.part_bars";
const PART_PALETTE: &str = "workspace.part_palette";
const PART_APPEARANCE: &str = "workspace.part_appearance";

/// Where the bars sit and which show, and the toolbox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BarsPart {
    pub layout: BarLayout,
    pub shown: ShownBars,
    /// The tool each toolbox group shows (tool ids, in group order).
    pub toolbox: Vec<String>,
    /// Toolbox flyouts hidden with the toolbox's "+" button.
    pub toolbox_hidden: Vec<String>,
    pub show_outline_flyout: bool,
    pub status_info: StatusInfo,
    /// The buttons of the Standard and Zoom toolbars, when changed.
    pub bar_items: std::collections::BTreeMap<String, Vec<String>>,
}

/// The interface's look.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearancePart {
    /// The interface size relative to the system's scaling (1 = 100 %);
    /// 0 is 100 % too.
    pub ui_size: f32,
    pub desktop_rgb: [u8; 3],
    /// The desktop colour was picked (white is then white, #530).
    pub desktop_custom: bool,
    pub page_border: bool,
}

impl Default for AppearancePart {
    fn default() -> Self {
        AppearancePart {
            ui_size: 0.0,
            desktop_rgb: [255, 255, 255],
            desktop_custom: false,
            page_border: true,
        }
    }
}

/// A workspace file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WorkspaceFile {
    /// Always [`KIND`].
    pub kind: String,
    pub version: u32,
    /// The built-in workspace the bars start from (Window > Workspace).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// Menu command keys and tool ids with their shortcuts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortcuts: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bars: Option<BarsPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub palette: Option<PalettePrefs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<AppearancePart>,
}

/// Why a file was not taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// Too large, not JSON, or a part not shaped as it should be.
    Unreadable(String),
    /// JSON, but not a workspace file.
    NotAWorkspace,
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Unreadable(why) => write!(f, "unreadable workspace file: {why}"),
            ReadError::NotAWorkspace => write!(f, "not a workspace file"),
        }
    }
}

impl WorkspaceFile {
    /// The file as pretty JSON.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap_or_default()
    }

    /// Read a workspace file, dropping or resetting whatever is out of
    /// bounds in it.
    pub fn from_bytes(bytes: &[u8]) -> Result<WorkspaceFile, ReadError> {
        if bytes.len() > MAX_FILE {
            return Err(ReadError::Unreadable(format!(
                "{} bytes, more than a workspace holds",
                bytes.len()
            )));
        }
        let mut file: WorkspaceFile =
            serde_json::from_slice(bytes).map_err(|e| ReadError::Unreadable(e.to_string()))?;
        if file.kind != KIND || file.version == 0 {
            return Err(ReadError::NotAWorkspace);
        }
        file.sanitize();
        Ok(file)
    }

    /// The parts this file has.
    #[cfg(test)]
    pub fn parts(&self) -> WorkspaceParts {
        WorkspaceParts {
            shortcuts: self.shortcuts.is_some(),
            bars: self.bars.is_some(),
            palette: self.palette.is_some(),
            appearance: self.appearance.is_some(),
        }
    }

    fn sanitize(&mut self) {
        if self.base.as_deref().is_some_and(|b| !short_text(b)) {
            self.base = None;
        }
        if let Some(list) = self.shortcuts.take() {
            self.shortcuts = Some(clean_shortcuts(list));
        }
        if let Some(bars) = &mut self.bars {
            let d = BarLayout::default();
            let l = &mut bars.layout;
            for (pos, default) in [
                (&mut l.toolbox_pos, d.toolbox_pos),
                (&mut l.palette_pos, d.palette_pos),
                (&mut l.standard_pos, d.standard_pos),
                (&mut l.property_pos, d.property_pos),
            ] {
                if !pos.iter().all(|v| v.is_finite() && v.abs() <= MAX_POSITION) {
                    *pos = default;
                }
            }
            // A pick out of bounds keeps its group's place, as nothing.
            bars.toolbox.truncate(MAX_LIST);
            for id in &mut bars.toolbox {
                if !short_text(id) {
                    id.clear();
                }
            }
            bars.toolbox_hidden.truncate(MAX_LIST);
            bars.toolbox_hidden.retain(|id| short_text(id));
            bars.bar_items.retain(|bar, _| {
                crate::ui::bar_items::ItemBar::ALL
                    .iter()
                    .any(|b| b.id() == bar)
            });
            for list in bars.bar_items.values_mut() {
                list.truncate(MAX_LIST);
                list.retain(|id| short_text(id));
            }
        }
        if let Some(a) = &mut self.appearance {
            if !(a.ui_size == 0.0 || (0.5..=4.0).contains(&a.ui_size)) {
                a.ui_size = 0.0;
            }
        }
    }
}

/// Not empty and not too long.
fn short_text(s: &str) -> bool {
    !s.trim().is_empty() && s.chars().count() <= MAX_TEXT
}

/// The usable shortcuts of a file: the last one given for each id, at most
/// [`MAX_SHORTCUTS`], in the file's order.
fn clean_shortcuts(list: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    let mut kept = Vec::new();
    for (id, keys) in list.into_iter().rev() {
        if kept.len() >= MAX_SHORTCUTS {
            break;
        }
        if short_text(&id) && short_text(&keys) && seen.insert(id.clone()) {
            kept.push((id, keys));
        }
    }
    kept.reverse();
    kept
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

impl App {
    /// The chosen `parts` of the workspace as it is now.
    pub fn export_workspace(&mut self, parts: WorkspaceParts) -> WorkspaceFile {
        self.sync_settings();
        let s = &self.settings;
        WorkspaceFile {
            kind: KIND.into(),
            version: VERSION,
            base: parts.bars.then(|| s.workspace.clone()),
            shortcuts: parts.shortcuts.then(|| s.shortcuts.clone()),
            bars: parts.bars.then(|| BarsPart {
                layout: s.bars.clone(),
                shown: self.shown_bars(),
                toolbox: s.toolbox.clone(),
                toolbox_hidden: s.toolbox_hidden.clone(),
                show_outline_flyout: s.show_outline_flyout,
                status_info: s.status_info,
                bar_items: s.bar_items.clone(),
            }),
            palette: parts.palette.then_some(s.palette),
            appearance: parts.appearance.then_some(AppearancePart {
                ui_size: s.ui_size,
                desktop_rgb: s.desktop_rgb,
                desktop_custom: s.desktop_custom,
                page_border: self.show_page_border,
            }),
        }
    }

    /// Apply the chosen `parts` that `file` has (the preferences are not
    /// written; the caller saves them or, in the options, OK does). The
    /// i18n keys of the parts applied.
    pub fn import_workspace(
        &mut self,
        file: &WorkspaceFile,
        parts: WorkspaceParts,
    ) -> Vec<&'static str> {
        let mut applied = Vec::new();
        if let (true, Some(b)) = (parts.bars, &file.bars) {
            if let Some(base) = &file.base {
                self.set_workspace(Workspace::from_id(base));
            }
            self.set_shown_bars(b.shown);
            self.settings.bars = b.layout.clone();
            self.apply_toolbox_picks(&b.toolbox);
            self.settings.toolbox_hidden = b.toolbox_hidden.clone();
            self.settings.show_outline_flyout = b.show_outline_flyout;
            self.settings.status_info = b.status_info;
            self.settings.bar_items = b.bar_items.clone();
            applied.push(PART_BARS);
        }
        if let (true, Some(list)) = (parts.shortcuts, &file.shortcuts) {
            self.settings.shortcuts = list.clone();
            self.shortcut_editor.note = None;
            self.shortcut_editor.capture = None;
            applied.push(PART_SHORTCUTS);
        }
        if let (true, Some(p)) = (parts.palette, file.palette) {
            self.settings.palette = p;
            applied.push(PART_PALETTE);
        }
        if let (true, Some(a)) = (parts.appearance, &file.appearance) {
            self.settings.ui_size = a.ui_size;
            self.settings.desktop_rgb = a.desktop_rgb;
            self.settings.desktop_custom = a.desktop_custom;
            self.show_page_border = a.page_border;
            applied.push(PART_APPEARANCE);
        }
        applied
    }

    /// Write the chosen parts of the workspace to `path`.
    pub fn export_workspace_to(&mut self, path: &Path, parts: WorkspaceParts) {
        let bytes = self.export_workspace(parts).to_bytes();
        let name = file_name(path);
        self.status = match crate::files::write(path, bytes) {
            Ok(()) => trf("status.workspace_exported", &[("name", &name)]),
            Err(e) => {
                log::warn!("could not write {}: {e}", path.display());
                trf("status.workspace_not_written", &[("name", &name)])
            }
        };
    }

    /// Read the workspace file at `path` and apply the chosen parts it
    /// has; `save` writes the preferences at once (the menu; the options
    /// leave it to OK).
    pub fn import_workspace_from(&mut self, path: &Path, parts: WorkspaceParts, save: bool) {
        let name = file_name(path);
        let file = match crate::files::read(path) {
            Ok(bytes) => WorkspaceFile::from_bytes(&bytes),
            Err(e) => Err(ReadError::Unreadable(e.to_string())),
        };
        let file = match file {
            Ok(f) => f,
            Err(e) => {
                log::warn!("{}: {e}", path.display());
                self.status = trf("status.workspace_not_read", &[("name", &name)]);
                return;
            }
        };
        let applied = self.import_workspace(&file, parts);
        if applied.is_empty() {
            self.status = trf("status.workspace_nothing", &[("name", &name)]);
            return;
        }
        if save {
            self.save_settings();
        }
        let parts: Vec<String> = applied.iter().map(|k| tr(k)).collect();
        self.status = trf(
            "status.workspace_imported",
            &[("name", &name), ("parts", &parts.join(", "))],
        );
    }

    /// Choose where to export the workspace (all the parts chosen in the
    /// options).
    pub fn export_workspace_dialog(&mut self, parts: WorkspaceParts) {
        crate::files::Dialog::new()
            .add_filter(tr("workspace.file_filter"), &[EXTENSION])
            .set_file_name(format!("workspace.{EXTENSION}"))
            .save_file(self, move |app, path| {
                let path = with_extension(path);
                app.export_workspace_to(&path, parts);
            });
    }

    /// Choose a workspace file and import the chosen parts it has.
    pub fn import_workspace_dialog(&mut self, parts: WorkspaceParts, save: bool) {
        crate::files::Dialog::new()
            .add_filter(tr("workspace.file_filter"), &[EXTENSION])
            .pick_file(self, move |app, path| {
                app.import_workspace_from(&path, parts, save);
            });
    }
}

/// `path` with the workspace extension when it has none.
fn with_extension(path: PathBuf) -> PathBuf {
    if path.extension().is_some() {
        path
    } else {
        path.with_extension(EXTENSION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::bars::Dock;

    /// A workspace changed in every part.
    fn customized() -> App {
        let mut app = App::headless();
        app.settings.shortcuts = vec![
            ("menu.file.export".into(), "Ctrl+Shift+X".into()),
            ("pick".into(), "Q".into()),
        ];
        app.settings.bars.toolbox = Dock::Right;
        app.settings.bars.palette_pos = [400.0, 500.0];
        app.settings.bars.locked = true;
        app.show_status_bar = false;
        app.show_zoom_toolbar = true;
        app.show_dockers = false;
        app.settings.toolbox_hidden = vec!["eraser".into()];
        app.settings.show_outline_flyout = true;
        app.settings.status_info = StatusInfo::CursorCoordinates;
        app.settings.palette.show_document = false;
        app.settings.palette.right_click_outline = false;
        app.settings.ui_size = 1.25;
        app.settings.desktop_rgb = [200, 210, 220];
        app.show_page_border = false;
        app.set_workspace(Workspace::Illustration);
        app.set_shown_bars(ShownBars {
            dockers: false,
            status_bar: false,
            zoom_toolbar: true,
            ..ShownBars::default()
        });
        app
    }

    #[test]
    fn a_workspace_comes_back_part_by_part() {
        let mut from = customized();
        let bytes = from.export_workspace(WorkspaceParts::ALL).to_bytes();
        let file = WorkspaceFile::from_bytes(&bytes).expect("reads back");
        assert_eq!(file.parts(), WorkspaceParts::ALL);
        let mut to = App::headless();
        let applied = to.import_workspace(&file, WorkspaceParts::ALL);
        assert_eq!(applied.len(), 4);
        assert_eq!(to.settings.shortcuts, from.settings.shortcuts);
        assert_eq!(to.settings.bars, from.settings.bars);
        assert_eq!(to.shown_bars(), from.shown_bars());
        assert!(!to.show_status_bar && to.show_zoom_toolbar && !to.show_dockers);
        assert_eq!(to.workspace, Workspace::Illustration);
        assert_eq!(to.settings.toolbox_hidden, vec!["eraser".to_string()]);
        assert!(to.settings.show_outline_flyout);
        assert_eq!(to.settings.status_info, StatusInfo::CursorCoordinates);
        assert_eq!(to.settings.palette, from.settings.palette);
        assert_eq!(to.settings.ui_size, 1.25);
        assert_eq!(to.settings.desktop_rgb, [200, 210, 220]);
        assert!(!to.show_page_border);
        // Through a file on disk, too.
        let dir = std::env::temp_dir().join(format!("tracedraw-ws-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("mine.tdws");
        from.export_workspace_to(&path, WorkspaceParts::ALL);
        let mut again = App::headless();
        again.import_workspace_from(&path, WorkspaceParts::ALL, false);
        assert_eq!(again.settings.shortcuts, from.settings.shortcuts);
        assert!(again.status.contains("mine.tdws"), "{}", again.status);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_the_parts_chosen_come_across() {
        let mut from = customized();
        // Exported with the shortcuts and the palette only.
        let parts = WorkspaceParts {
            bars: false,
            appearance: false,
            ..WorkspaceParts::ALL
        };
        let file = from.export_workspace(parts);
        assert!(file.bars.is_none() && file.appearance.is_none() && file.base.is_none());
        // Imported with the shortcuts only.
        let mut to = App::headless();
        let palette = to.settings.palette;
        let bars = to.settings.bars.clone();
        let only_shortcuts = WorkspaceParts {
            shortcuts: true,
            bars: false,
            palette: false,
            appearance: false,
        };
        let applied = to.import_workspace(&file, only_shortcuts);
        assert_eq!(applied, vec![PART_SHORTCUTS]);
        assert_eq!(to.settings.shortcuts, from.settings.shortcuts);
        assert_eq!(to.settings.palette, palette);
        assert_eq!(to.settings.bars, bars);
        // Parts the file lacks are not applied even when chosen.
        let applied = to.import_workspace(&file, WorkspaceParts::ALL);
        assert_eq!(applied, vec![PART_SHORTCUTS, PART_PALETTE]);
    }

    #[test]
    fn odd_files_are_refused_or_tamed() {
        for bytes in [
            &b""[..],
            b"\x00\xff\xfe garbage",
            b"[1, 2, 3]",
            b"{\"kind\": \"something else\", \"version\": 1}",
            b"{\"version\": 1}",
            b"{\"kind\": \"tracedraw-workspace\", \"version\": 0}",
            b"{\"kind\": \"tracedraw-workspace\", \"version\": 1, \"shortcuts\": 5}",
            b"{\"kind\": \"tracedraw-workspace\", \"version\": 1, \"bars\": {\"layout\": {\"toolbox\": \"diagonal\"}}}",
        ] {
            assert!(WorkspaceFile::from_bytes(bytes).is_err(), "{:?}", String::from_utf8_lossy(bytes));
        }
        assert_eq!(
            WorkspaceFile::from_bytes(&vec![b' '; MAX_FILE + 1])
                .err()
                .map(|e| matches!(e, ReadError::Unreadable(_))),
            Some(true)
        );
        // A later version with parts of its own still gives what it can.
        let long = "x".repeat(MAX_TEXT + 1);
        let mut shortcuts: Vec<(String, String)> = (0..MAX_SHORTCUTS + 50)
            .map(|i| (format!("cmd{i}"), "Ctrl+K".to_string()))
            .collect();
        shortcuts.push(("menu.file.export".into(), String::new()));
        shortcuts.push((long.clone(), "F2".into()));
        shortcuts.push(("cmd7".into(), "F3".into()));
        let json = serde_json::json!({
            "kind": KIND,
            "version": 7,
            "future_part": {"anything": [1, 2, 3]},
            "base": long,
            "shortcuts": shortcuts,
            "bars": {
                "layout": {"palette_pos": [1e30, 5.0], "standard_pos": [-3.0e5, 0.0]},
                "toolbox": ["pick", long, ""],
                "toolbox_hidden": ["", "eraser", "x".repeat(500)]
            },
            "appearance": {"ui_size": 99.0}
        });
        let file = WorkspaceFile::from_bytes(json.to_string().as_bytes()).expect("tamed");
        let list = file.shortcuts.clone().unwrap_or_default();
        assert_eq!(list.len(), MAX_SHORTCUTS);
        // The last keys given for an id win and keep their place; the
        // oldest entries beyond the limit go.
        assert_eq!(list.last(), Some(&("cmd7".to_string(), "F3".to_string())));
        assert_eq!(list.iter().filter(|(id, _)| id == "cmd7").count(), 1);
        assert!(list.iter().any(|(id, _)| id == "cmd51"));
        assert!(!list.iter().any(|(id, _)| id == "cmd50"));
        assert!(list.iter().all(|(id, k)| short_text(id) && short_text(k)));
        assert!(file.base.is_none());
        let bars = file.bars.clone().unwrap_or_default();
        let d = BarLayout::default();
        assert_eq!(
            (bars.layout.palette_pos, bars.layout.standard_pos),
            (d.palette_pos, d.standard_pos)
        );
        assert_eq!(
            bars.toolbox,
            vec!["pick".to_string(), String::new(), String::new()]
        );
        assert_eq!(bars.toolbox_hidden, vec!["eraser".to_string()]);
        assert_eq!(file.appearance.as_ref().map(|a| a.ui_size), Some(0.0));
        // Applying it is harmless.
        let mut app = App::headless();
        app.import_workspace(&file, WorkspaceParts::ALL);
        assert_eq!(app.settings.ui_size, 0.0);
        // A missing file is a message, not a panic.
        app.import_workspace_from(Path::new("/no/such/dir/x.tdws"), WorkspaceParts::ALL, false);
        assert!(app.status.contains("x.tdws"));
    }

    #[test]
    fn bars_shown_survive_a_restart() {
        let mut app = App::headless();
        app.show_status_bar = false;
        app.show_transform_toolbar = true;
        app.sync_settings();
        let saved = app.settings.clone();
        assert_eq!(
            saved
                .shown_bars
                .map(|s| (s.status_bar, s.transform_toolbar)),
            Some((false, true))
        );
        let mut next = App::headless();
        next.apply_settings(saved);
        assert!(!next.show_status_bar && next.show_transform_toolbar);
        // Preferences from before bars were kept: the workspace's own.
        let mut old = crate::settings::Settings::default();
        old.shown_bars = None;
        old.workspace = "lite".into();
        next.apply_settings(old);
        assert!(!next.show_dockers && next.show_status_bar && !next.show_rulers);
    }
}
