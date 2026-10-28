//! File > Save As (and Save of a drawing without a file): the Save
//! Drawing dialog. One window chooses the folder (path bar, Up, New folder,
//! shortcuts and the folder's contents), the file name, the type
//! (TraceDraw or `.cdr`), the version of either (the newest first), whether
//! the CMYK profile goes along, and the document's title,
//! subject and rating, and asks before replacing a file. In a browser,
//! where every save is a download, it keeps the name, type, version and
//! properties. See docs/behavior/files.md.

use crate::app::App;
use crate::files::{self, DirEntry, Place};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::dialogs::{enter_pressed, window};
use crate::ui::options::{combo, grid, row_label};
use crate::ui::tips::RichTip;
use egui::{Context, RichText, Ui};
use std::path::{Path, PathBuf};
use tracedraw_core::document::{TDRAW_FORMAT_VERSION, TDRAW_RELEASES};
use tracedraw_core::Command;

const ERROR: egui::Color32 = egui::Color32::from_rgb(0xC4, 0x2B, 0x1C);
/// The dialog's width and the height of the folder list.
const WIDTH: f32 = 760.0;
const LIST_H: f32 = 250.0;
const PLACES_W: f32 = 150.0;
/// Letters typed in the list this many seconds apart start a new search.
const TYPE_AHEAD_PAUSE: f64 = 1.0;

/// The file types the drawing can be saved as.
pub const TDRAW: &str = "tdraw";
pub const CDR: &str = "cdr";

/// The dialog's state.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveAsState {
    /// The folder shown (desktop).
    pub dir: PathBuf,
    /// The path bar (a folder typed there opens with Enter).
    pub dir_text: String,
    /// The file name, with or without its extension.
    pub name: String,
    /// `.cdr` rather than TraceDraw.
    pub cdr: bool,
    pub version: u16,
    /// The `.tdraw` format version.
    pub tdraw_version: u32,
    /// The drawing's CMYK profile (name, size in bytes), when it has one.
    pub profile: Option<(String, usize)>,
    /// The profile goes into the file (`.cdr` from version 16).
    pub embed_profile: bool,
    /// What embedding the text's fonts adds, in bytes (0: none may be).
    pub fonts_size: u64,
    /// The fonts go into the file (`.cdr` from version 16).
    pub embed_fonts: bool,
    /// What embedding the installed fonts the text uses adds to a
    /// `.tdraw` file, in bytes (0: none may be).
    pub tdraw_fonts_size: u64,
    /// The installed fonts the text uses go into the `.tdraw` file.
    pub tdraw_embed_fonts: bool,
    pub title: String,
    pub subject: String,
    pub rating: u8,
    /// What `dir` holds.
    pub entries: Vec<DirEntry>,
    /// A file that exists, waiting for Replace.
    pub replace: Option<PathBuf>,
    /// Why nothing was saved (no name, no such folder).
    pub note: Option<String>,
    /// The entry picked in the list (a folder or a file).
    pub selected: Option<PathBuf>,
    /// The entry being renamed and the name typed so far.
    pub renaming: Option<(PathBuf, String)>,
    /// The list has the keyboard: letters jump to entries, F2 renames,
    /// Enter opens a folder, the arrows move.
    pub list_keys: bool,
    /// The letters typed in the list, and when the last came (seconds).
    pub typed: String,
    pub typed_at: f64,
    /// The list scrolls to the picked entry (the keys moved it).
    pub scroll_to: bool,
}

impl SaveAsState {
    /// Start in the drawing's folder (else the last one used, else the
    /// documents folder) with its name, its type (`.cdr` for a drawing
    /// that came from or went to one, or a new one when the options say
    /// so), the version the options set and its properties.
    pub fn new(app: &App) -> Self {
        let doc = app.doc();
        let file = app.file.as_ref();
        let dir = file
            .and_then(|f| f.parent())
            .map(Path::to_path_buf)
            .filter(|d| files::is_dir(d))
            .or_else(|| app.settings.last_dir.clone().filter(|d| files::is_dir(d)))
            .or_else(|| {
                files::places()
                    .into_iter()
                    .find(|(p, _)| *p == Place::Documents)
                    .map(|(_, d)| d)
            })
            .or_else(files::home_dir)
            .unwrap_or_default();
        let stem = file
            .and_then(|f| f.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                let t = doc.title.trim();
                if t.is_empty() {
                    "Graphic1".to_string()
                } else {
                    t.to_string()
                }
            });
        // A drawing without a file of its own starts at the type Options >
        // Global > File Formats chooses for new drawings (#613).
        let cdr = file.is_some_and(|f| crate::app::is_cdr_path(f))
            || app.cdr_version.is_some()
            || (file.is_none() && app.settings.formats.save_new_as_cdr);
        let mut st = SaveAsState {
            dir_text: dir.display().to_string(),
            dir: dir.clone(),
            name: String::new(),
            cdr,
            // A drawing opened from a .cdr file starts at its own version,
            // as Save keeps it; any other at the version Options > Global >
            // File Formats sets (#613).
            version: app
                .cdr_version
                .unwrap_or(app.settings.formats.cdr_version)
                .clamp(
                    tracedraw_cdr::write::CHECKED_SINCE,
                    tracedraw_cdr::write::NEWEST_VERSION,
                ),
            tdraw_version: TDRAW_FORMAT_VERSION,
            // The drawing's CMYK profile, else the one Colour Management
            // loaded (it goes into the file when the drawing has none).
            profile: doc
                .cmyk_profile
                .as_ref()
                .map(|p| (p.name.clone(), p.data.len()))
                .or_else(|| app.settings_cmyk_profile().map(|p| (p.name, p.data.len()))),
            embed_profile: app.cdr_embed_profile,
            fonts_size: tracedraw_cdr::write::embedded_size(&crate::app::cdr_font_faces(doc)),
            embed_fonts: app.cdr_embed_fonts,
            tdraw_fonts_size: tracedraw_text::fonts()
                .fonts_to_embed(doc)
                .iter()
                .map(|f| f.data.len() as u64)
                .sum(),
            tdraw_embed_fonts: app.tdraw_embed_fonts,
            title: doc.title.clone(),
            subject: doc.metadata.subject.clone(),
            rating: doc.metadata.rating,
            entries: Vec::new(),
            replace: None,
            note: None,
            selected: None,
            renaming: None,
            list_keys: false,
            typed: String::new(),
            typed_at: 0.0,
            scroll_to: false,
        };
        st.name = format!("{stem}.{}", st.extension());
        st.open_dir(dir);
        st
    }

    /// The chosen type's extension.
    pub fn extension(&self) -> &'static str {
        if self.cdr {
            CDR
        } else {
            TDRAW
        }
    }

    /// The name with the chosen type's extension (one it already has of
    /// either type is replaced).
    pub fn file_name(&self) -> String {
        let name = self.name.trim();
        let lower = name.to_ascii_lowercase();
        let stem = [TDRAW, CDR]
            .iter()
            .find_map(|ext| {
                lower
                    .strip_suffix(&format!(".{ext}"))
                    .map(|s| &name[..s.len()])
            })
            .unwrap_or(name);
        format!("{stem}.{}", self.extension())
    }

    /// The file to write: in the folder (desktop) or a download named so
    /// (browser).
    pub fn target(&self) -> PathBuf {
        if files::WEB {
            PathBuf::from("/download").join(self.file_name())
        } else {
            self.dir.join(self.file_name())
        }
    }

    /// Show `dir` (a folder that cannot be read shows empty).
    pub fn open_dir(&mut self, dir: PathBuf) {
        self.entries = files::list_entries(&dir);
        self.dir_text = dir.display().to_string();
        self.dir = dir;
        self.replace = None;
        self.note = None;
        self.selected = None;
        self.renaming = None;
        self.typed.clear();
    }

    /// New folder: make "New folder" (or "New folder (2)" ...) here, pick
    /// it and start renaming it, staying in this folder.
    pub fn new_folder(&mut self) {
        let base = tr("dialog.new_folder");
        let mut path = self.dir.join(&base);
        let mut n = 2;
        while path.exists() {
            path = self.dir.join(format!("{base} ({n})"));
            n += 1;
        }
        match files::create_dir(&path) {
            Ok(()) => {
                let dir = self.dir.clone();
                self.open_dir(dir);
                self.list_keys = true;
                self.start_rename(path);
            }
            Err(e) => self.note = Some(e.to_string()),
        }
    }

    /// Pick an entry; a file's name becomes the name to save as.
    pub fn select(&mut self, path: PathBuf) {
        if let Some(e) = self.entries.iter().find(|e| e.path == path) {
            if !e.is_dir {
                self.name = e.name.clone();
                self.replace = None;
            }
        }
        self.selected = Some(path);
    }

    /// Start renaming an entry of the list.
    pub fn start_rename(&mut self, path: PathBuf) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        self.selected = Some(path.clone());
        self.renaming = Some((path, name));
    }

    /// Give the entry being renamed the name typed. A name that is empty,
    /// has a folder separator or is taken leaves it as it was and says so;
    /// false then.
    pub fn finish_rename(&mut self) -> bool {
        let Some((from, typed)) = self.renaming.take() else {
            return true;
        };
        let name = typed.trim();
        let old = from.file_name().map(|n| n.to_string_lossy().to_string());
        if old.as_deref() == Some(name) {
            return true;
        }
        if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
            self.note = Some(tr("dialog.bad_name"));
            return false;
        }
        let to = self.dir.join(name);
        match files::rename(&from, &to) {
            Ok(()) => {
                let dir = self.dir.clone();
                self.open_dir(dir);
                self.select(to);
                true
            }
            Err(e) => {
                self.note = Some(if e.kind() == std::io::ErrorKind::AlreadyExists {
                    trf("dialog.name_taken", &[("name", name)])
                } else {
                    e.to_string()
                });
                false
            }
        }
    }

    /// Letters typed in the list pick the first entry whose name starts
    /// with them; letters a second or more apart start again, and one
    /// letter typed again goes on to the next entry starting with it.
    pub fn type_ahead(&mut self, text: &str, now: f64) {
        if !(0.0..=TYPE_AHEAD_PAUSE).contains(&(now - self.typed_at)) {
            self.typed.clear();
        }
        self.typed_at = now;
        self.typed.push_str(&text.to_lowercase());
        let shown: Vec<PathBuf> = self
            .shown()
            .iter()
            .filter(|e| {
                let name = e.name.to_lowercase();
                name.starts_with(&self.typed)
            })
            .map(|e| e.path.clone())
            .collect();
        let mut chars = self.typed.chars();
        let first = chars.next();
        let repeated = first.is_some_and(|c| chars.all(|d| d == c));
        let pick = if repeated && self.typed.chars().count() > 1 && shown.is_empty() {
            // "ss": the next entry starting with "s" after the picked one.
            let c = first.unwrap_or_default().to_string();
            let same: Vec<PathBuf> = self
                .shown()
                .iter()
                .filter(|e| e.name.to_lowercase().starts_with(&c))
                .map(|e| e.path.clone())
                .collect();
            let at = same.iter().position(|p| Some(p) == self.selected.as_ref());
            match at {
                Some(i) => same.get((i + 1) % same.len()).cloned(),
                None => same.first().cloned(),
            }
        } else {
            shown.first().cloned()
        };
        if let Some(path) = pick {
            self.select(path);
            self.scroll_to = true;
        }
    }

    /// The arrow keys: the entry `by` places down (up when negative).
    pub fn step(&mut self, by: i32) {
        let shown: Vec<PathBuf> = self.shown().iter().map(|e| e.path.clone()).collect();
        if shown.is_empty() {
            return;
        }
        let at = shown.iter().position(|p| Some(p) == self.selected.as_ref());
        let next = match at {
            Some(i) => (i as i64 + by as i64).clamp(0, shown.len() as i64 - 1) as usize,
            None if by < 0 => shown.len() - 1,
            None => 0,
        };
        self.select(shown[next].clone());
        self.scroll_to = true;
    }

    /// The picked entry when it is a folder.
    fn selected_folder(&self) -> Option<PathBuf> {
        let path = self.selected.as_ref()?;
        self.entries
            .iter()
            .find(|e| &e.path == path && e.is_dir)
            .map(|e| e.path.clone())
    }

    /// The type changed: the name takes its extension.
    pub fn set_cdr(&mut self, cdr: bool) {
        self.cdr = cdr;
        self.name = self.file_name();
        self.replace = None;
    }

    /// The entries shown: folders, and files of the chosen type.
    pub fn shown(&self) -> Vec<&DirEntry> {
        let ext = format!(".{}", self.extension());
        self.entries
            .iter()
            .filter(|e| e.is_dir || e.name.to_ascii_lowercase().ends_with(&ext))
            .collect()
    }
}

/// "v22 (2020)" for the version lists: the file version and the release
/// that writes it; versions whose files are not yet known to open in other
/// programs say they are experimental.
pub fn version_label(version: u16) -> String {
    let checked = tracedraw_cdr::write::CHECKED_SINCE..=tracedraw_cdr::write::CHECKED_UNTIL;
    let key = if !checked.contains(&version) {
        "dialog.cdr_version_item_experimental"
    } else {
        "dialog.cdr_version_item"
    };
    trf(
        key,
        &[("v", &version.to_string()), ("name", release_name(version))],
    )
}

/// The release that saves files of `version` by default: numbers up to
/// 12, X3 to X8 for 13 to 18, then years (version 24 was saved by two
/// yearly releases).
pub fn release_name(version: u16) -> &'static str {
    match version {
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        9 => "9",
        10 => "10",
        11 => "11",
        12 => "12",
        13 => "X3",
        14 => "X4",
        15 => "X5",
        16 => "X6",
        17 => "X7",
        18 => "X8",
        19 => "2017",
        20 => "2018",
        21 => "2019",
        22 => "2020",
        23 => "2021",
        24 => "2022/2023",
        25 => "2024",
        26 => "2025",
        27 => "2026",
        _ => "?",
    }
}

/// The versions offered, newest first.
/// The `.tdraw` versions offered, newest first.
pub fn tdraw_version_choices() -> Vec<u32> {
    (0..=TDRAW_FORMAT_VERSION).rev().collect()
}

/// "v2 (TraceDraw 1.0)": the format version and the release that first
/// wrote it; just "v0" while it is the only one.
pub fn tdraw_version_label(version: u32) -> String {
    if TDRAW_FORMAT_VERSION == 0 {
        return format!("v{version}");
    }
    let release = TDRAW_RELEASES.get(version as usize).copied().unwrap_or("?");
    format!("v{version} (TraceDraw {release})")
}

/// The `.cdr` versions offered, newest first: from the oldest checked to
/// open in other programs (14; the other program's own list stops at 15).
/// Older versions are still written by the command line and the MCP
/// server, and Save keeps the version of a drawing opened from one.
pub fn version_choices() -> Vec<u16> {
    (tracedraw_cdr::write::CHECKED_SINCE..=tracedraw_cdr::write::NEWEST_VERSION)
        .rev()
        .collect()
}

impl App {
    /// File > Save As: the Save Drawing dialog.
    pub fn save_as_dialog(&mut self) {
        self.dialog = crate::ui::dialogs::Dialog::SaveAs(SaveAsState::new(self));
    }

    /// The dialog's Save: check the name and the folder, ask before
    /// replacing a file, keep the properties, then write. True when the
    /// dialog is done.
    pub fn finish_save_as(&mut self, st: &mut SaveAsState) -> bool {
        let stem_empty = st
            .file_name()
            .strip_suffix(&format!(".{}", st.extension()))
            .is_none_or(|s| s.trim().is_empty());
        if stem_empty {
            st.note = Some(tr("dialog.need_file_name"));
            return false;
        }
        if !files::WEB && !files::is_dir(&st.dir) {
            st.note = Some(tr("dialog.no_such_folder"));
            return false;
        }
        let path = st.target();
        if files::exists(&path) && st.replace.as_ref() != Some(&path) {
            st.replace = Some(path);
            return false;
        }
        // The properties go with the file (one undo step each).
        if st.title.trim() != self.doc().title {
            self.run(Command::SetTitle {
                title: st.title.trim().to_string(),
            });
        }
        let md = &self.doc().metadata;
        if md.subject != st.subject || md.rating != st.rating {
            let mut metadata = md.clone();
            metadata.subject = st.subject.clone();
            metadata.rating = st.rating.min(5);
            self.run(Command::SetMetadata { metadata });
        }
        if !files::WEB {
            self.settings.last_dir = Some(st.dir.clone());
        }
        if st.cdr {
            self.cdr_embed_profile = st.embed_profile;
            self.cdr_embed_fonts = st.embed_fonts;
            self.save_cdr_to(path, st.version);
        } else {
            self.tdraw_embed_fonts = st.tdraw_embed_fonts;
            self.tdraw_version = Some(st.tdraw_version);
            self.save_to(path);
        }
        true
    }
}

/// The shortcut's name.
fn place_name(place: Place, path: &Path) -> String {
    match place {
        Place::Home => tr("dialog.place_home"),
        Place::Desktop => tr("dialog.place_desktop"),
        Place::Documents => tr("dialog.place_documents"),
        Place::Downloads => tr("dialog.place_downloads"),
        Place::Pictures => tr("dialog.place_pictures"),
        Place::Root => path.display().to_string(),
    }
}

/// A size as the list shows it: KB from 1 KB, MB from 1 MB.
pub fn size_text(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut SaveAsState, close: &mut bool) {
    window(ctx, tr("dialog.save_drawing")).show(ctx, |ui| {
        ui.set_width(WIDTH);
        if !files::WEB {
            folder_bar(ui, st);
            ui.add_space(4.0);
            // The shortcuts and the list keep a fixed height (a vertical
            // separator would take all there is).
            ui.allocate_ui(egui::vec2(WIDTH, LIST_H), |ui| {
                ui.set_min_height(LIST_H);
                ui.set_max_height(LIST_H);
                ui.horizontal_top(|ui| {
                    places(ui, st);
                    ui.separator();
                    listing(ui, st);
                });
            });
            ui.add_space(6.0);
        }
        fields(ui, st);
        if let Some(note) = &st.note {
            ui.label(RichText::new(note.as_str()).color(ERROR));
        }
        if files::WEB {
            ui.label(RichText::new(tr("dialog.download_hint")).color(Tokens::text_dim()));
        }
        ui.add_space(6.0);
        if let Some(path) = st.replace.clone() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            ui.label(trf("dialog.replace_question", &[("name", &name)]));
            let replace = tr("dialog.replace");
            let cancel = tr("dialog.cancel");
            match crate::ui::chrome::button_row(ui, &[&replace, &cancel], false) {
                Some(0) => {
                    if app.finish_save_as(st) {
                        *close = true;
                    }
                }
                Some(_) => st.replace = None,
                None => {}
            }
            return;
        }
        let save = tr("dialog.save");
        let cancel = tr("dialog.cancel");
        match crate::ui::chrome::button_row(ui, &[&save, &cancel], false) {
            Some(0) => {
                if app.finish_save_as(st) {
                    *close = true;
                }
            }
            Some(_) => *close = true,
            None => {
                if enter_pressed(ui) && app.finish_save_as(st) {
                    *close = true;
                }
            }
        }
    });
}

/// Up, the path bar and New folder.
fn folder_bar(ui: &mut Ui, st: &mut SaveAsState) {
    ui.horizontal(|ui| {
        let up = st.dir.parent().map(Path::to_path_buf);
        let resp = ui
            .add_enabled(
                up.is_some(),
                egui::Button::new("  ").min_size(egui::vec2(24.0, 20.0)),
            )
            .rich_tip(tr("dialog.up_folder"));
        // An arrow pointing up.
        let c = resp.rect.center();
        let colour = if up.is_some() {
            Tokens::icon()
        } else {
            Tokens::text_dim()
        };
        let stroke = egui::Stroke::new(1.5, colour);
        ui.painter().line_segment(
            [c + egui::vec2(0.0, 5.0), c + egui::vec2(0.0, -5.0)],
            stroke,
        );
        ui.painter().line_segment(
            [c + egui::vec2(-4.0, -1.0), c + egui::vec2(0.0, -5.0)],
            stroke,
        );
        ui.painter().line_segment(
            [c + egui::vec2(4.0, -1.0), c + egui::vec2(0.0, -5.0)],
            stroke,
        );
        if resp.clicked() {
            if let Some(up) = up {
                st.open_dir(up);
            }
        }
        let width = ui.available_width() - 120.0;
        let bar = ui.add(
            egui::TextEdit::singleline(&mut st.dir_text)
                .id(egui::Id::new("save_as_path"))
                .desired_width(width.max(200.0)),
        );
        if bar.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let typed = PathBuf::from(st.dir_text.trim());
            if files::is_dir(&typed) {
                st.open_dir(typed);
            } else {
                st.note = Some(tr("dialog.no_such_folder"));
            }
        }
        if ui.button(tr("dialog.new_folder")).clicked() {
            st.new_folder();
        }
    });
}

/// The shortcuts down the left.
fn places(ui: &mut Ui, st: &mut SaveAsState) {
    ui.vertical(|ui| {
        ui.set_width(PLACES_W);
        for (place, path) in files::places() {
            let name = place_name(place, &path);
            if ui
                .selectable_label(st.dir == path, name)
                .rich_tip(path.display().to_string())
                .clicked()
            {
                st.open_dir(path);
            }
        }
    });
}

/// The folder's contents: a click picks an entry (a file's name becomes
/// the name), a double click on a folder opens it, a right click offers
/// Open and Rename. With the list picked, letters jump to entries, F2
/// renames, Enter opens a folder and the arrows move.
fn listing(ui: &mut Ui, st: &mut SaveAsState) {
    let tz = jiff::tz::TimeZone::system();
    let mut open: Option<PathBuf> = None;
    let mut pick: Option<PathBuf> = None;
    let mut rename: Option<PathBuf> = None;
    let mut rename_done: Option<bool> = None;
    list_keys(ui, st, &mut open, &mut rename);
    ui.vertical(|ui| {
        egui::ScrollArea::vertical()
            .id_salt("save_as_list")
            .max_height(LIST_H)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("save_as_grid")
                    .num_columns(4)
                    .striped(true)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        for head in [
                            "dialog.col_name",
                            "dialog.col_modified",
                            "dialog.col_type",
                            "dialog.col_size",
                        ] {
                            ui.label(RichText::new(tr(head)).color(Tokens::text_dim()));
                        }
                        ui.end_row();
                        let picked = st.selected.clone();
                        let name = st.name.clone();
                        let entries: Vec<DirEntry> = st.shown().into_iter().cloned().collect();
                        for e in &entries {
                            let selected = picked.as_ref() == Some(&e.path)
                                || (picked.is_none() && !e.is_dir && e.name == name);
                            let editing = st
                                .renaming
                                .as_mut()
                                .filter(|(path, _)| path == &e.path)
                                .map(|(_, typed)| typed);
                            if let Some(typed) = editing {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    entry_icon(ui, e.is_dir);
                                    let field = ui.add(
                                        egui::TextEdit::singleline(typed)
                                            .id(egui::Id::new("save_as_rename"))
                                            .desired_width(220.0),
                                    );
                                    if !field.has_focus() && !field.lost_focus() {
                                        field.request_focus();
                                        field.scroll_to_me(None);
                                    }
                                    if field.lost_focus() {
                                        let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
                                        rename_done = Some(!cancel);
                                    }
                                });
                            } else {
                                let r = ui
                                    .horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        entry_icon(ui, e.is_dir);
                                        ui.selectable_label(selected, e.name.as_str())
                                    })
                                    .inner;
                                if selected && st.scroll_to {
                                    r.scroll_to_me(None);
                                }
                                if r.double_clicked() && e.is_dir {
                                    open = Some(e.path.clone());
                                } else if r.clicked() || r.secondary_clicked() {
                                    pick = Some(e.path.clone());
                                }
                                r.context_menu(|ui| {
                                    if e.is_dir && ui.button(tr("dialog.open_entry")).clicked() {
                                        open = Some(e.path.clone());
                                        ui.close();
                                    }
                                    if ui.button(tr("dialog.rename_entry")).clicked() {
                                        rename = Some(e.path.clone());
                                        ui.close();
                                    }
                                });
                            }
                            ui.label(
                                e.modified
                                    .map(|s| crate::time_tracker::date_time(s, &tz))
                                    .unwrap_or_default(),
                            );
                            ui.label(if e.is_dir {
                                tr("dialog.file_folder")
                            } else {
                                let ext = Path::new(&e.name)
                                    .extension()
                                    .map(|x| x.to_string_lossy().to_uppercase())
                                    .unwrap_or_default();
                                trf("dialog.file_of_type", &[("ext", &ext)])
                            });
                            ui.label(if e.is_dir {
                                String::new()
                            } else {
                                size_text(e.size)
                            });
                            ui.end_row();
                        }
                    });
            });
    });
    st.scroll_to = false;
    match rename_done {
        Some(true) => {
            // The Enter that ends a rename does not also save.
            ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
            st.finish_rename();
        }
        Some(false) => st.renaming = None,
        None => {}
    }
    if let Some(path) = pick {
        st.list_keys = true;
        st.select(path);
    }
    if let Some(path) = rename {
        st.list_keys = true;
        st.start_rename(path);
    }
    if let Some(dir) = open {
        st.list_keys = true;
        st.open_dir(dir);
    }
}

/// The keys of the list, while it is picked and no field takes typing.
fn list_keys(
    ui: &mut Ui,
    st: &mut SaveAsState,
    open: &mut Option<PathBuf>,
    rename: &mut Option<PathBuf>,
) {
    if st.renaming.is_some() {
        return;
    }
    if ui.ctx().egui_wants_keyboard_input() {
        // Typing in the name or the path bar hands the keys back.
        st.list_keys = false;
        return;
    }
    if !st.list_keys {
        return;
    }
    let now = ui.input(|i| i.time);
    let (text, f2, down, up) = ui.input(|i| {
        let text: String = i
            .events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        (
            text,
            i.key_pressed(egui::Key::F2),
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::ArrowUp),
        )
    });
    let text = text.trim();
    if !text.is_empty() {
        st.type_ahead(text, now);
    }
    if down {
        st.step(1);
    }
    if up {
        st.step(-1);
    }
    if f2 {
        if let Some(path) = st.selected.clone() {
            *rename = Some(path);
        }
    }
    if let Some(dir) = st.selected_folder() {
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            *open = Some(dir);
        }
    }
}

/// A small folder (a tab over a box), or a page with a folded corner.
fn entry_icon(ui: &mut Ui, folder: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
    let p = ui.painter();
    let stroke = egui::Stroke::new(1.0, Tokens::icon());
    if folder {
        let fill = egui::Color32::from_rgb(0xF3, 0xC6, 0x5F);
        let body = egui::Rect::from_min_max(
            rect.min + egui::vec2(1.0, 4.0),
            rect.max - egui::vec2(1.0, 2.0),
        );
        let tab = egui::Rect::from_min_size(rect.min + egui::vec2(1.0, 2.0), egui::vec2(5.0, 3.0));
        p.rect_filled(tab, 1.0, fill);
        p.rect_filled(body, 1.0, fill);
        p.rect_stroke(body, 1.0, stroke, egui::StrokeKind::Inside);
    } else {
        let page = egui::Rect::from_min_max(
            rect.min + egui::vec2(3.0, 1.0),
            rect.max - egui::vec2(3.0, 1.0),
        );
        p.rect_filled(page, 0.0, egui::Color32::WHITE);
        p.rect_stroke(page, 0.0, stroke, egui::StrokeKind::Inside);
        let corner = page.right_top();
        p.line_segment(
            [
                corner + egui::vec2(-3.0, 0.0),
                corner + egui::vec2(-3.0, 3.0),
            ],
            stroke,
        );
        p.line_segment(
            [
                corner + egui::vec2(-3.0, 3.0),
                corner + egui::vec2(0.0, 3.0),
            ],
            stroke,
        );
    }
}

/// File name, type, version and the document's properties.
fn fields(ui: &mut Ui, st: &mut SaveAsState) {
    grid("save_as_fields").show(ui, |ui| {
        row_label(ui, &tr("dialog.file_name"));
        let name = ui.add(
            egui::TextEdit::singleline(&mut st.name)
                .id(egui::Id::new("save_as_name"))
                .desired_width(WIDTH - 140.0),
        );
        if name.changed() {
            st.replace = None;
            st.note = None;
        }
        ui.end_row();
        row_label(ui, &tr("dialog.save_as_type"));
        let mut cdr = st.cdr;
        if combo(
            ui,
            "save_as_type",
            &mut cdr,
            &[false, true],
            |cdr| {
                if cdr {
                    tr("file.cdr_files")
                } else {
                    "TraceDraw (*.tdraw)".to_string()
                }
            },
            260.0,
        ) {
            st.set_cdr(cdr);
        }
        ui.end_row();
        // With one .tdraw format version there is nothing to choose or to
        // tell: the row stays, empty, so the dialog keeps its size.
        let one_tdraw = !st.cdr && tdraw_version_choices().len() == 1;
        if one_tdraw {
            ui.label("");
        } else {
            row_label(ui, &tr("dialog.cdr_version"));
        }
        ui.horizontal(|ui| {
            if st.cdr {
                combo(
                    ui,
                    "save_as_version",
                    &mut st.version,
                    &version_choices(),
                    version_label,
                    260.0,
                );
            } else if !one_tdraw {
                combo(
                    ui,
                    "save_as_tdraw_version",
                    &mut st.tdraw_version,
                    &tdraw_version_choices(),
                    tdraw_version_label,
                    260.0,
                );
            }
        });
        ui.end_row();
        if st.cdr {
            // Profiles are stored in the package of version 16 and later.
            let can = st.version >= 16;
            ui.label("");
            ui.horizontal(|ui| {
                ui.add_enabled_ui(can, |ui| {
                    let text = match &st.profile {
                        Some((_, size)) => trf(
                            "dialog.embed_color_profile",
                            &[("size", &size_text(*size as u64))],
                        ),
                        None => tr("dialog.embed_color_profile_none"),
                    };
                    let mut on = st.embed_profile && can;
                    if ui.checkbox(&mut on, text).changed() {
                        st.embed_profile = on;
                    }
                });
                let name = st
                    .profile
                    .as_ref()
                    .map(|(n, _)| n.clone())
                    .unwrap_or_else(|| tr("dialog.color_profile_missing"));
                ui.label(
                    RichText::new(trf("dialog.color_profile", &[("name", &name)]))
                        .color(Tokens::text_dim()),
                );
                ui.add_space(16.0);
                // Fonts are stored in the package of version 16 and later.
                let can = st.version >= 16;
                ui.add_enabled_ui(can, |ui| {
                    let text = if st.fonts_size > 0 {
                        trf(
                            "dialog.embed_fonts_size",
                            &[("size", &size_text(st.fonts_size))],
                        )
                    } else {
                        tr("dialog.embed_fonts")
                    };
                    let mut on = st.embed_fonts && can;
                    if ui
                        .checkbox(&mut on, text)
                        .rich_disabled_tip(tr("dialog.embed_fonts_later"))
                        .changed()
                    {
                        st.embed_fonts = on;
                    }
                });
            });
            ui.end_row();
        } else {
            // The installed fonts the text uses go into the file, where
            // their licence allows it (fonts that came with a drawing go
            // either way).
            let can = st.tdraw_fonts_size > 0;
            ui.label("");
            ui.add_enabled_ui(can, |ui| {
                let text = if can {
                    trf(
                        "dialog.embed_fonts_size",
                        &[("size", &size_text(st.tdraw_fonts_size))],
                    )
                } else {
                    tr("dialog.embed_fonts")
                };
                let mut on = st.tdraw_embed_fonts && can;
                if ui
                    .checkbox(&mut on, text)
                    .rich_disabled_tip(tr("dialog.embed_fonts_none"))
                    .changed()
                {
                    st.tdraw_embed_fonts = on;
                }
            });
            ui.end_row();
        }
        row_label(ui, &tr("dialog.title"));
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut st.title).desired_width(220.0));
            ui.label(tr("dialog.subject"));
            ui.add(egui::TextEdit::singleline(&mut st.subject).desired_width(200.0));
            ui.label(tr("dialog.rating"));
            for i in 1..=5u8 {
                let colour = if st.rating >= i {
                    egui::Color32::from_rgb(0xE8, 0xA3, 0x17)
                } else {
                    egui::Color32::from_gray(0xC0)
                };
                let star = egui::RichText::new("\u{2605}").color(colour).size(15.0);
                if ui
                    .add(egui::Label::new(star).sense(egui::Sense::click()))
                    .clicked()
                {
                    st.rating = if st.rating == i { i - 1 } else { i };
                }
            }
        });
        ui.end_row();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dialogs::Dialog;
    use egui::{vec2, Pos2, Rect};
    use tracedraw_core::document::ShapeKind;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tracedraw-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Draw the open dialog for a frame, pressing Enter when `enter`.
    fn frame(ctx: &Context, app: &mut App, enter: bool) {
        let keys = if enter {
            vec![key(egui::Key::Enter)]
        } else {
            Vec::new()
        };
        frame_with(ctx, app, keys);
    }

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// Draw the open dialog for a frame with these events.
    fn frame_with(ctx: &Context, app: &mut App, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::dialogs::show(app, ui.ctx()));
        out.textures_delta.clear();
    }

    fn version_of(path: &Path) -> Option<u16> {
        let bytes = std::fs::read(path).ok()?;
        let (_, report) = tracedraw_cdr::open_bytes(&bytes, "t").ok()?;
        report.version.map(|v| v.0)
    }

    fn drawing() -> App {
        let mut app = App::headless();
        app.new_shape(ShapeKind::Rect {
            rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 60.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer");
        app
    }

    fn state(app: &mut App) -> &mut SaveAsState {
        match &mut app.dialog {
            Dialog::SaveAs(st) => st,
            other => panic!("no Save Drawing dialog: {other:?}"),
        }
    }

    #[test]
    fn save_as_chooses_folder_name_type_version_and_properties() {
        let dir = temp_dir("save-as");
        std::fs::create_dir_all(dir.join("sub")).expect("folder");
        std::fs::write(dir.join("old.cdr"), b"x").expect("file");
        std::fs::write(dir.join("notes.txt"), b"x").expect("file");
        let mut app = drawing();
        app.save_as_dialog();
        let st = state(&mut app);
        st.open_dir(dir.clone());
        // Folders and files of the type, folders first.
        let names = |st: &SaveAsState| {
            st.shown()
                .iter()
                .map(|e| e.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(st), vec!["sub".to_string()], "TraceDraw files only");
        st.set_cdr(true);
        assert_eq!(names(st), vec!["sub".to_string(), "old.cdr".into()]);
        assert!(st.name.ends_with(".cdr"));
        st.name = "poster".into();
        st.version = 15;
        st.title = "Poster".into();
        st.subject = "Fair".into();
        st.rating = 4;
        let ctx = crate::theme::ui_context();
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, true);
        assert!(matches!(app.dialog, Dialog::None), "{:?}", app.dialog);
        let path = dir.join("poster.cdr");
        assert_eq!(version_of(&path), Some(15));
        assert_eq!(app.doc().title, "Poster");
        assert_eq!(
            (
                app.doc().metadata.subject.as_str(),
                app.doc().metadata.rating
            ),
            ("Fair", 4)
        );
        assert_eq!(app.file.as_deref(), Some(path.as_path()));
        assert_eq!(app.settings.last_dir.as_deref(), Some(dir.as_path()));
        // The next Save As starts there, as .cdr, at the version the
        // drawing now has, as Save keeps it (#613).
        app.save_as_dialog();
        let st = state(&mut app);
        assert_eq!(
            (st.dir.as_path(), st.name.as_str(), st.cdr, st.version),
            (dir.as_path(), "poster.cdr", true, 15)
        );
        st.version = 15;
        // An existing file asks first; Replace writes it.
        st.name = "old".into();
        frame(&ctx, &mut app, true);
        let st = state(&mut app);
        assert_eq!(st.replace.as_deref(), Some(dir.join("old.cdr").as_path()));
        assert_eq!(std::fs::read(dir.join("old.cdr")).unwrap_or_default(), b"x");
        let mut st = st.clone();
        assert!(app.finish_save_as(&mut st));
        assert_eq!(version_of(&dir.join("old.cdr")), Some(15));
        // TraceDraw files.
        app.save_as_dialog();
        let st = state(&mut app);
        st.set_cdr(false);
        st.name = "native.cdr".into();
        assert_eq!(st.file_name(), "native.tdraw");
        assert_eq!(st.tdraw_version, TDRAW_FORMAT_VERSION);
        assert_eq!(tdraw_version_choices()[0], TDRAW_FORMAT_VERSION);
        assert_eq!(tdraw_version_choices(), vec![0]);
        st.tdraw_version = 0;
        let mut st = st.clone();
        assert!(app.finish_save_as(&mut st));
        let text = std::fs::read_to_string(dir.join("native.tdraw")).unwrap_or_default();
        assert_eq!(tracedraw_core::document::tdraw_text_version(&text), 0);
        // Ctrl+S keeps the version; opening the file brings it back.
        app.engine.mark_saved();
        app.save(false);
        let text = std::fs::read_to_string(dir.join("native.tdraw")).unwrap_or_default();
        assert_eq!(tracedraw_core::document::tdraw_text_version(&text), 0);
        let mut other = App::headless();
        other.open_path(dir.join("native.tdraw"));
        assert_eq!(other.tdraw_version, Some(0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn names_and_folders_are_checked() {
        let dir = temp_dir("save-as-checks");
        let mut app = drawing();
        app.save_as_dialog();
        let mut st = state(&mut app).clone();
        st.open_dir(dir.clone());
        st.name = "  ".into();
        assert!(!app.finish_save_as(&mut st));
        assert_eq!(st.note, Some(tr("dialog.need_file_name")));
        st.name = ".cdr".into();
        assert!(!app.finish_save_as(&mut st));
        st.name = "ok".into();
        st.dir = dir.join("missing");
        assert!(!app.finish_save_as(&mut st));
        assert_eq!(st.note, Some(tr("dialog.no_such_folder")));
        // Every way of typing the extension ends with the type's.
        st.cdr = true;
        for (typed, saved) in [
            ("a", "a.cdr"),
            ("a.CDR", "a.cdr"),
            ("a.tdraw", "a.cdr"),
            ("a.b", "a.b.cdr"),
        ] {
            st.name = typed.into();
            assert_eq!(st.file_name(), saved);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_dialog_draws_and_opens_folders() {
        let dir = temp_dir("save-as-draw");
        std::fs::create_dir_all(dir.join("inner")).expect("folder");
        let mut app = drawing();
        app.save_as_dialog();
        state(&mut app).open_dir(dir.clone());
        let ctx = crate::theme::ui_context();
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, false);
        assert!(matches!(app.dialog, Dialog::SaveAs(_)));
        // New folder makes one, stays here and starts renaming it (#913).
        let st = state(&mut app);
        st.new_folder();
        let made = dir.join(tr("dialog.new_folder"));
        assert!(made.is_dir());
        assert_eq!(st.dir, dir);
        assert_eq!(st.renaming, Some((made.clone(), tr("dialog.new_folder"))));
        st.renaming = None;
        st.open_dir(made.clone());
        assert_eq!(st.dir, made);
        st.open_dir(dir.clone());
        assert_eq!(st.shown().len(), 2);
        assert_eq!(size_text(10), "1 KB");
        assert_eq!(size_text(3 * 1024 * 1024 + 1), "3.0 MB");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A new folder can be renamed at once, and so can any entry; a bad or
    /// taken name leaves it as it was (#913).
    #[test]
    fn entries_of_the_folder_list_are_renamed() {
        let dir = temp_dir("save-as-rename");
        std::fs::create_dir_all(dir.join("taken")).expect("folder");
        let mut app = drawing();
        app.save_as_dialog();
        let st = state(&mut app);
        st.open_dir(dir.clone());
        st.new_folder();
        let made = dir.join(tr("dialog.new_folder"));
        // A second New folder is "New folder (2)".
        st.renaming = None;
        st.new_folder();
        assert_eq!(
            st.selected,
            Some(dir.join(format!("{} (2)", tr("dialog.new_folder"))))
        );
        st.renaming = None;
        st.start_rename(made.clone());
        st.renaming.as_mut().expect("renaming").1 = "a/b".into();
        assert!(!st.finish_rename());
        assert_eq!(st.note, Some(tr("dialog.bad_name")));
        st.start_rename(made.clone());
        st.renaming.as_mut().expect("renaming").1 = "taken".into();
        assert!(!st.finish_rename());
        assert!(made.is_dir());
        st.start_rename(made.clone());
        st.renaming.as_mut().expect("renaming").1 = " Drawings ".into();
        assert!(st.finish_rename());
        assert!(!made.exists());
        assert!(dir.join("Drawings").is_dir());
        assert_eq!(st.selected, Some(dir.join("Drawings")));
        assert_eq!(st.renaming, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Letters typed in the list jump to the entries starting with them;
    /// F2 renames, Enter opens the folder picked rather than saving (#913).
    #[test]
    fn letters_jump_through_the_folder_list() {
        let dir = temp_dir("save-as-keys");
        for d in ["apple", "banana", "berry"] {
            std::fs::create_dir_all(dir.join(d)).expect("folder");
        }
        std::fs::write(dir.join("bird.tdraw"), b"{}").expect("file");
        let mut app = drawing();
        app.save_as_dialog();
        let st = state(&mut app);
        st.set_cdr(false);
        st.open_dir(dir.clone());
        st.type_ahead("b", 0.0);
        assert_eq!(st.selected, Some(dir.join("banana")));
        st.type_ahead("E", 0.4);
        assert_eq!(st.selected, Some(dir.join("berry")));
        // After a pause the search starts again.
        st.type_ahead("b", 5.0);
        assert_eq!(st.selected, Some(dir.join("banana")));
        // The same letter again goes on to the next entry with it.
        st.type_ahead("b", 5.2);
        assert_eq!(st.selected, Some(dir.join("berry")));
        st.type_ahead("b", 5.4);
        assert_eq!(st.selected, Some(dir.join("bird.tdraw")));
        assert_eq!(st.name, "bird.tdraw");
        st.step(-3);
        assert_eq!(st.selected, Some(dir.join("apple")));

        // Through the dialog: with the list picked, letters, F2 and Enter.
        st.list_keys = true;
        st.selected = None;
        let ctx = crate::theme::ui_context();
        frame(&ctx, &mut app, false);
        frame_with(&ctx, &mut app, vec![egui::Event::Text("a".into())]);
        assert_eq!(state(&mut app).selected, Some(dir.join("apple")));
        frame_with(&ctx, &mut app, vec![key(egui::Key::F2)]);
        assert_eq!(
            state(&mut app).renaming,
            Some((dir.join("apple"), "apple".to_string()))
        );
        state(&mut app).renaming = None;
        frame_with(&ctx, &mut app, vec![key(egui::Key::Enter)]);
        assert!(matches!(app.dialog, Dialog::SaveAs(_)));
        assert_eq!(state(&mut app).dir, dir.join("apple"));
        assert!(!dir.join("apple").join("bird.tdraw").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A template written in another version starts a new drawing, which
    /// Save As offers in the version Options sets (#613).
    #[test]
    fn a_drawing_from_a_template_starts_at_the_options_version() {
        let dir = temp_dir("save-as-template");
        let doc = drawing().engine.document().clone();
        let template = dir.join("card.cdt");
        std::fs::write(
            &template,
            tracedraw_cdr::write::document_to_cdr_version(&doc, 20),
        )
        .expect("written");
        let mut app = App::headless();
        app.settings.formats.cdr_version = 15;
        app.open_path(template);
        assert_eq!(app.cdr_version, None);
        app.save_as_dialog();
        let st = state(&mut app);
        st.set_cdr(true);
        assert_eq!(st.version, 15);
    }

    #[test]
    fn a_drawing_opened_from_a_cdr_file_keeps_its_version() {
        let dir = temp_dir("save-as-keep");
        let doc = drawing().engine.document().clone();
        // A version TraceDraw saves in is kept: Save asks where (an opened
        // .cdr is never overwritten without Save As) and offers it.
        let kept = dir.join("kept.cdr");
        std::fs::write(
            &kept,
            tracedraw_cdr::write::document_to_cdr_version(&doc, 21),
        )
        .expect("written");
        let mut app = App::headless();
        app.open_path(kept.clone());
        assert_eq!(app.cdr_version, Some(21));
        assert_eq!(app.cdr_unsaved_version, None);
        app.engine.mark_unsaved();
        app.save(false);
        assert!(state(&mut app).cdr);
        app.dialog = Dialog::None;
        // An older version: Save does not write it, it says so and Save As
        // opens on the oldest version saved.
        let old = dir.join("old.cdr");
        std::fs::write(&old, tracedraw_cdr::write::document_to_cdr_version(&doc, 9))
            .expect("written");
        app.open_path(old.clone());
        assert_eq!(app.cdr_unsaved_version, Some(9));
        assert_eq!(app.cdr_version, Some(tracedraw_cdr::write::CHECKED_SINCE));
        assert!(app.status.contains("14"), "{}", app.status);
        app.save(false);
        assert!(
            matches!(app.dialog, Dialog::OldCdrVersion { version: 9 }),
            "{:?}",
            app.dialog
        );
        assert_eq!(version_of(&old), Some(9), "the file is not written");
        app.save_as_from_old_cdr();
        let st = state(&mut app);
        assert!(st.cdr && st.version == 14, "{st:?}");
        st.open_dir(dir.clone());
        st.name = "copy".into();
        let mut st = st.clone();
        assert!(app.finish_save_as(&mut st));
        app.dialog = Dialog::None;
        assert_eq!(version_of(&dir.join("copy.cdr")), Some(14));
        // From then on Save writes that version without asking.
        assert_eq!(app.cdr_unsaved_version, None);
        app.engine.mark_unsaved();
        app.save(false);
        assert!(matches!(app.dialog, Dialog::None));
        // Versions too old or too new become the nearest saved.
        app.cdr_version = Some(3);
        assert_eq!(app.cdr_save_version(), tracedraw_cdr::write::CHECKED_SINCE);
        app.cdr_version = Some(99);
        assert_eq!(app.cdr_save_version(), tracedraw_cdr::write::CHECKED_UNTIL);
        assert_eq!(version_choices().first(), Some(&27));
        assert_eq!(version_choices().last(), Some(&14));
        assert!(version_label(15).contains("15"));
        assert!(version_label(13).contains("X3"));
        assert!(version_label(27).contains("2026"));
        // Versions not yet checked in other programs say so.
        let plain = |v: u16| {
            version_label(21)
                .replace("21", &v.to_string())
                .replace("2019", release_name(v))
        };
        assert_eq!(version_label(27), plain(27));
        assert_eq!(version_label(16), plain(16));
        assert_eq!(version_label(15), plain(15));
        assert_eq!(version_label(14), plain(14));
        assert_ne!(version_label(13), plain(13));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn linux_folder_names_come_from_the_users_settings() {
        let home = Path::new("/home/ana");
        let dirs = files::parse_user_dirs(
            "# comment\nXDG_DESKTOP_DIR=\"$HOME/Área de Trabalho\"\nXDG_DOWNLOAD_DIR=\"/data/dl\"\n",
            home,
        );
        assert_eq!(
            dirs,
            vec![
                ("XDG_DESKTOP_DIR".to_string(), home.join("Área de Trabalho")),
                ("XDG_DOWNLOAD_DIR".to_string(), PathBuf::from("/data/dl")),
            ]
        );
    }

    #[test]
    fn the_cmyk_profile_goes_along_unless_left_out() {
        let dir = temp_dir("save-as-profile");
        let mut doc = drawing().engine.document().clone();
        doc.cmyk_profile = Some(tracedraw_core::DocumentProfile::new(
            "Synthetic CMYK",
            tracedraw_core::icc::test_profiles::lut16_cmyk_profile(),
        ));
        let src = dir.join("src.cdr");
        std::fs::write(
            &src,
            tracedraw_cdr::write::document_to_cdr_version(&doc, 27),
        )
        .expect("written");
        let mut app = App::headless();
        app.open_path(src);
        let has_profile = |path: &Path| {
            let bytes = std::fs::read(path).unwrap_or_default();
            bytes
                .windows(b"color/profiles/cmyk/".len())
                .any(|w| w == b"color/profiles/cmyk/")
        };
        for (embed, name) in [(true, "with"), (false, "without")] {
            app.save_as_dialog();
            let st = state(&mut app);
            let (shown, size) = st.profile.clone().expect("the profile is offered");
            assert_eq!(shown, "Synthetic CMYK");
            assert!(size > 0);
            st.embed_profile = embed;
            st.open_dir(dir.clone());
            st.name = name.into();
            let mut st = st.clone();
            assert!(app.finish_save_as(&mut st));
            app.dialog = Dialog::None;
            assert_eq!(
                has_profile(&dir.join(format!("{name}.cdr"))),
                embed,
                "{name}"
            );
        }
        // The choice stays for the next Save As.
        app.save_as_dialog();
        assert!(!state(&mut app).embed_profile);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Save As for `.tdraw` offers Embed fonts, off by default (#886): the
    /// installed fonts the text uses go into the file, when their licence
    /// allows it, only when the box is ticked; the choice stays for the
    /// next Save As (#275).
    #[test]
    fn installed_fonts_go_into_tdraw_files_unless_left_out() {
        let fonts = tracedraw_text::fonts();
        // A family of this machine whose face may be embedded.
        let family = fonts.families().into_iter().find(|family| {
            let mut doc = tracedraw_core::Document::default();
            doc.pages[0].layers[0]
                .shapes
                .push(tracedraw_core::Shape::new(
                    tracedraw_core::ShapeId(1),
                    ShapeKind::Text {
                        spans: vec![tracedraw_core::TextSpan::new("Hi", family, 12.0)],
                        origin: tracedraw_core::geometry::Point::ZERO,
                        frame: None,
                        align: Default::default(),
                        para: Default::default(),
                        on_path: None,
                    },
                ));
            !fonts.fonts_to_embed(&doc).is_empty()
        });
        let Some(family) = family else {
            eprintln!("no installed font may be embedded here; nothing to check");
            return;
        };
        let dir = temp_dir("save-as-fonts");
        let mut app = App::headless();
        app.new_shape(ShapeKind::Text {
            spans: vec![tracedraw_core::TextSpan::new("Hi", &family, 12.0)],
            origin: tracedraw_core::geometry::Point::ZERO,
            frame: None,
            align: Default::default(),
            para: Default::default(),
            on_path: None,
        })
        .expect("a layer");
        assert!(!app.tdraw_embed_fonts, "off by default");
        let fonts_in = |path: &Path| -> usize {
            let doc = tracedraw_io::load_native(path).expect("the file opens");
            doc.fonts.len()
        };
        for (embed, name) in [(true, "with"), (false, "without")] {
            app.save_as_dialog();
            let st = state(&mut app);
            assert!(!st.cdr);
            assert!(st.tdraw_fonts_size > 0, "the size is shown");
            // Off the first time, then what the last Save As chose.
            assert_eq!(st.tdraw_embed_fonts, name == "without", "{name}");
            st.tdraw_embed_fonts = embed;
            st.open_dir(dir.clone());
            st.name = name.into();
            let mut st = st.clone();
            assert!(app.finish_save_as(&mut st));
            app.dialog = Dialog::None;
            let path = dir.join(format!("{name}.tdraw"));
            assert_eq!(fonts_in(&path), usize::from(embed), "{name}");
            // The drawing itself carries nothing: the fonts are added to
            // the file alone.
            assert!(app.doc().fonts.is_empty());
        }
        // The choice stays for the next Save As.
        app.save_as_dialog();
        assert!(!state(&mut app).tdraw_embed_fonts);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
