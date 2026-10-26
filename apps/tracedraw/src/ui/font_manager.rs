//! Tools > Font Manager: a window of its own with a menu row and a
//! toolbar, the libraries (installed fonts, folders the user adds,
//! Favorites and collections) and filters on the left, and on the right a
//! sample text, a search field and every face of the chosen library drawn
//! in itself, grouped by family, with how it is built and whether it is
//! installed. Fonts of added folders can be installed for TraceDraw and
//! uninstalled; folders, installed files, Favorites and collections are
//! kept in the settings. See docs/behavior/font-manager.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use egui::{Color32, Context, Rect, Sense, Stroke, Ui, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::mpsc;
use tracedraw_text::catalog::{self, Embedding, FaceRecord, Script, FEATURES};
use tracedraw_text::FontKind;

/// The sample size's range and default, in pixels to the em.
pub const SAMPLE_PX: std::ops::RangeInclusive<f32> = 16.0..=120.0;
pub const DEFAULT_SAMPLE_PX: f32 = 44.0;
/// Height of a family's header row and the space of a face's card
/// outside its sample.
const HEADER_H: f32 = 76.0;
const CARD_EXTRA: f32 = 40.0;
/// Width of the libraries and filters, and of the details pane.
const LEFT_W: f32 = 290.0;
const DETAILS_W: f32 = 270.0;
/// Room for the separator before the details pane (its line and the
/// spacing on both sides).
const DETAILS_GAP: f32 = 20.0;
/// Pictures kept before the oldest go.
const MAX_PICTURES: usize = 600;
/// The green of installed fonts' bar.
const INSTALLED: Color32 = Color32::from_rgb(0x66, 0xBB, 0x6A);
const NOT_INSTALLED: Color32 = Color32::from_rgb(0xBD, 0xBD, 0xBD);
const SECTION_BLUE: Color32 = Color32::from_rgb(0x1E, 0x9B, 0xE0);

/// What the Font Manager keeps between sessions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FontManagerPrefs {
    /// Folders added to the libraries.
    pub folders: Vec<PathBuf>,
    /// Font files installed for TraceDraw (loaded at every start).
    pub installed: Vec<PathBuf>,
    /// Keys ([`FaceRecord::key`]) of the favourite faces.
    pub favorites: Vec<String>,
    pub collections: Vec<FontCollection>,
    /// The sample text (empty: each face shows its family's name).
    pub sample: String,
    /// The sample size, pixels to the em (0: the default).
    pub sample_px: f32,
    /// The details pane is shown.
    pub details: bool,
}

/// A collection of faces the user made.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FontCollection {
    pub name: String,
    pub fonts: Vec<String>,
}

impl FontManagerPrefs {
    pub fn sample_px(&self) -> f32 {
        if self.sample_px <= 0.0 {
            DEFAULT_SAMPLE_PX
        } else {
            self.sample_px.clamp(*SAMPLE_PX.start(), *SAMPLE_PX.end())
        }
    }
}

/// Load the font files installed for TraceDraw and those of the added
/// folders (at start).
pub fn install_saved(prefs: &FontManagerPrefs) -> Vec<String> {
    install_saved_in(tracedraw_text::fonts(), prefs)
}

fn install_saved_in(fonts: &tracedraw_text::FontSystem, prefs: &FontManagerPrefs) -> Vec<String> {
    let mut paths = prefs.installed.clone();
    paths.extend(folder_files(prefs));
    if paths.is_empty() {
        return Vec::new();
    }
    fonts.install_paths(&paths)
}

/// The font files of the added folders: usable in drawings for as long as
/// their folder is in the libraries (#866).
fn folder_files(prefs: &FontManagerPrefs) -> Vec<PathBuf> {
    prefs
        .folders
        .iter()
        .flat_map(|d| catalog::font_files(d))
        .collect()
}

/// Take out the fonts of `dir`, a folder just removed from the
/// libraries, except those installed on their own or in another folder
/// still added (the system's fonts are never taken out). Returns how many
/// files went.
fn remove_folder_fonts(
    fonts: &tracedraw_text::FontSystem,
    prefs: &FontManagerPrefs,
    dir: &std::path::Path,
) -> usize {
    let keep: BTreeSet<PathBuf> = prefs
        .installed
        .iter()
        .cloned()
        .chain(folder_files(prefs))
        .collect();
    catalog::font_files(dir)
        .into_iter()
        .filter(|p| !keep.contains(p))
        .filter(|p| fonts.uninstall_path(p))
        .count()
}

/// The library the list shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Library {
    /// The installed fonts.
    #[default]
    MyFonts,
    Folder(PathBuf),
    Favorites,
    Collection(usize),
}

/// One check box of the filters. Within a section any checked box lets a
/// face through; every section with a checked box must.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Filter {
    Installed,
    NotInstalled,
    Protected,
    Duplicates,
    Embedding(Embedding),
    Tech(FontKind),
    /// A weight class, 100 to 900.
    Weight(u16),
    /// 0 condensed, 1 normal, 2 expanded.
    Width(u8),
    Italic(bool),
    Script(Script),
    /// An index into [`FEATURES`].
    Feature(usize),
}

impl Filter {
    /// The section the filter sits in (0 font status to 7 OpenType).
    fn section(self) -> usize {
        match self {
            Filter::Installed | Filter::NotInstalled | Filter::Protected | Filter::Duplicates => 0,
            Filter::Embedding(_) => 1,
            Filter::Tech(_) => 2,
            Filter::Weight(_) => 3,
            Filter::Width(_) => 4,
            Filter::Italic(_) => 5,
            Filter::Script(_) => 6,
            Filter::Feature(_) => 7,
        }
    }

    /// Whether `r` passes this box (`duplicate`: its key appears twice).
    fn passes(self, r: &FaceRecord, duplicate: bool) -> bool {
        match self {
            Filter::Installed => r.installed,
            Filter::NotInstalled => !r.installed,
            Filter::Protected => r.system,
            Filter::Duplicates => duplicate,
            Filter::Embedding(e) => r.embedding == e,
            Filter::Tech(k) => r.kind == k,
            Filter::Weight(w) => weight_class(r.weight) == w,
            Filter::Width(w) => width_class(r.width) == w,
            Filter::Italic(i) => r.italic == i,
            Filter::Script(s) => r.scripts.contains(&s),
            Filter::Feature(i) => FEATURES
                .get(i)
                .is_some_and(|f| r.features.iter().any(|g| g == f)),
        }
    }
}

/// The weight rounded to its class, 100 to 900.
pub fn weight_class(weight: u16) -> u16 {
    (((u32::from(weight) + 50) / 100) * 100).clamp(100, 900) as u16
}

/// 0 condensed, 1 normal, 2 expanded.
pub fn width_class(width: u16) -> u8 {
    match width {
        0..=4 => 0,
        5 => 1,
        _ => 2,
    }
}

/// Whether `r` passes every section of `filters` and the search text.
pub fn passes(r: &FaceRecord, filters: &BTreeSet<Filter>, search: &str, duplicate: bool) -> bool {
    let search = search.trim().to_lowercase();
    if !search.is_empty()
        && ![&r.family, &r.style, &r.postscript]
            .iter()
            .any(|s| s.to_lowercase().contains(&search))
    {
        return false;
    }
    (0..8).all(|section| {
        let mut in_section = filters.iter().filter(|f| f.section() == section).peekable();
        in_section.peek().is_none() || in_section.any(|f| f.passes(r, duplicate))
    })
}

/// A row of the list: a family's header, or a face (index into the
/// faces).
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Family(String),
    Face(usize),
}

/// The rows for `faces`, sorted by family, each family's header first,
/// then its faces upright first, by width and weight.
pub fn rows(faces: &[FaceRecord]) -> Vec<Row> {
    let mut order: Vec<usize> = (0..faces.len()).collect();
    order.sort_by(|&a, &b| {
        let (x, y) = (&faces[a], &faces[b]);
        x.family
            .to_lowercase()
            .cmp(&y.family.to_lowercase())
            .then(x.italic.cmp(&y.italic))
            .then(x.width.cmp(&y.width))
            .then(x.weight.cmp(&y.weight))
            .then(x.style.cmp(&y.style))
    });
    let mut out = Vec::new();
    let mut last: Option<&str> = None;
    for i in order {
        let fam = faces[i].family.as_str();
        if last != Some(fam) {
            out.push(Row::Family(fam.to_string()));
            last = Some(fam);
        }
        out.push(Row::Face(i));
    }
    out
}

/// The dialog's state between frames.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FontManagerState {
    pub library: Library,
    pub filters: BTreeSet<Filter>,
    pub search: String,
    /// Keys of the selected faces.
    pub selected: BTreeSet<String>,
    /// The face whose details show.
    pub focus: Option<String>,
    /// Collapsed sections, by name.
    pub collapsed: BTreeSet<&'static str>,
    /// A collection being renamed, and the name typed.
    pub renaming: Option<(usize, String)>,
    /// A line for the status bar after an action.
    pub note: Option<String>,
}

/// A picture of a sample and where its baseline is, in pixels.
#[derive(Clone)]
struct Picture {
    texture: egui::TextureHandle,
    baseline: f32,
}

/// What the Font Manager loads and draws, kept in the application.
#[derive(Default)]
pub struct Catalog {
    generation: Option<u64>,
    installed: Option<Vec<FaceRecord>>,
    loading: Option<mpsc::Receiver<Vec<FaceRecord>>>,
    folders: HashMap<PathBuf, Vec<FaceRecord>>,
    folder_jobs: HashMap<PathBuf, mpsc::Receiver<Vec<FaceRecord>>>,
    pictures: HashMap<(String, String, u16), Option<Picture>>,
}

/// Run `job` off the interface thread where there are threads.
fn spawn<T: Send + 'static>(job: impl FnOnce() -> T + Send + 'static) -> mpsc::Receiver<T> {
    let (tx, rx) = mpsc::channel();
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || {
        let _ = tx.send(job());
    });
    #[cfg(target_arch = "wasm32")]
    {
        let _ = tx.send(job());
    }
    rx
}

impl Catalog {
    /// Start again when fonts changed; take the results of loads that
    /// finished. True while something is still loading.
    fn refresh(&mut self, folders: &[PathBuf]) -> bool {
        let g = tracedraw_text::fonts().generation();
        if self.generation != Some(g) {
            self.generation = Some(g);
            self.pictures.clear();
            self.loading = Some(spawn(|| tracedraw_text::fonts().catalog()));
        }
        if let Some(rx) = &self.loading {
            if let Ok(faces) = rx.try_recv() {
                self.installed = Some(faces);
                self.loading = None;
            }
        }
        for f in folders {
            if !self.folders.contains_key(f) && !self.folder_jobs.contains_key(f) {
                let dir = f.clone();
                self.folder_jobs
                    .insert(f.clone(), spawn(move || catalog::scan_folder(&dir)));
            }
        }
        let done: Vec<PathBuf> = self
            .folder_jobs
            .iter()
            .filter_map(|(k, rx)| rx.try_recv().ok().map(|faces| (k.clone(), faces)))
            .map(|(k, faces)| {
                self.folders.insert(k.clone(), faces);
                k
            })
            .collect();
        for k in done {
            self.folder_jobs.remove(&k);
        }
        if self.pictures.len() > MAX_PICTURES {
            self.pictures.clear();
        }
        self.loading.is_some() || !self.folder_jobs.is_empty()
    }

    /// Scan a folder again (after fonts in it were installed).
    fn rescan(&mut self, dir: &PathBuf) {
        self.folders.remove(dir);
    }

    /// Every face known: the installed ones, then those of the folders
    /// not installed.
    fn all(&self) -> Vec<FaceRecord> {
        let mut out = self.installed.clone().unwrap_or_default();
        let paths: BTreeSet<PathBuf> = out.iter().filter_map(|r| r.path.clone()).collect();
        for faces in self.folders.values() {
            out.extend(
                faces
                    .iter()
                    .filter(|r| r.path.as_ref().is_none_or(|p| !paths.contains(p)))
                    .cloned(),
            );
        }
        out
    }

    /// The faces of `library`.
    fn faces(&self, library: &Library, prefs: &FontManagerPrefs) -> Vec<FaceRecord> {
        let installed_paths: BTreeSet<PathBuf> = self
            .installed
            .iter()
            .flatten()
            .filter_map(|r| r.path.clone())
            .collect();
        match library {
            Library::MyFonts => self.installed.clone().unwrap_or_default(),
            Library::Folder(dir) => self
                .folders
                .get(dir)
                .map(|faces| {
                    faces
                        .iter()
                        .map(|r| {
                            let mut r = r.clone();
                            r.installed =
                                r.path.as_ref().is_some_and(|p| installed_paths.contains(p));
                            r
                        })
                        .collect()
                })
                .unwrap_or_default(),
            Library::Favorites => keep(self.all(), &prefs.favorites),
            Library::Collection(i) => prefs
                .collections
                .get(*i)
                .map(|c| keep(self.all(), &c.fonts))
                .unwrap_or_default(),
        }
    }

    /// `text` in face `r` at `em_px`, cached.
    fn picture(
        &mut self,
        ctx: &Context,
        r: &FaceRecord,
        text: &str,
        em_px: f32,
    ) -> Option<Picture> {
        let px = em_px.round().clamp(4.0, 400.0) as u16;
        let id = format!(
            "{}|{}|{}",
            r.key(),
            r.path
                .as_deref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            r.index
        );
        let key = (id.clone(), text.to_string(), px);
        if let Some(p) = self.pictures.get(&key) {
            return p.clone();
        }
        let outline = if let Some(h) = r.handle {
            tracedraw_text::fonts()
                .with_face(h, |d, i| catalog::sample_outline(d, i, text))
                .flatten()
        } else {
            r.path
                .as_ref()
                .and_then(|p| crate::files::read(p).ok())
                .and_then(|d| catalog::sample_outline(&d, r.index, text))
        };
        let picture = outline
            .and_then(|(path, advance, ascent, descent)| {
                draw_sample(&path, advance, ascent, descent, f32::from(px))
            })
            .map(|(image, baseline)| Picture {
                texture: ctx.load_texture(
                    format!("font_manager:{id}:{px}"),
                    image,
                    egui::TextureOptions::LINEAR,
                ),
                baseline,
            });
        self.pictures.insert(key, picture.clone());
        picture
    }
}

/// The faces of `all` whose key is in `keys`, each once.
fn keep(all: Vec<FaceRecord>, keys: &[String]) -> Vec<FaceRecord> {
    let mut seen = BTreeSet::new();
    all.into_iter()
        .filter(|r| keys.contains(&r.key()) && seen.insert(r.key()))
        .collect()
}

/// A sample outline (em 1.0, y up) drawn dark on transparent at `em_px`
/// pixels to the em, and its baseline's distance from the top.
fn draw_sample(
    path: &kurbo::BezPath,
    advance: f64,
    ascent: f64,
    descent: f64,
    em_px: f32,
) -> Option<(egui::ColorImage, f32)> {
    let scale = f64::from(em_px);
    let pad = 2.0;
    let w = ((advance * scale).ceil() + 2.0 * pad).clamp(1.0, 4096.0) as u32;
    let h = (((ascent + descent) * scale).ceil() + 2.0 * pad).clamp(1.0, 1024.0) as u32;
    let sk = tracedraw_render::to_sk_path(path)?;
    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    let mut paint = tiny_skia::Paint::default();
    // A white mask, tinted with the theme's text colour where it is
    // painted (#380).
    paint.set_color_rgba8(0xff, 0xff, 0xff, 0xff);
    paint.anti_alias = true;
    let baseline = (ascent * scale + pad) as f32;
    let t = tiny_skia::Transform::from_row(
        scale as f32,
        0.0,
        0.0,
        -(scale as f32),
        pad as f32,
        baseline,
    );
    pixmap.fill_path(&sk, &paint, tiny_skia::FillRule::Winding, t, None);
    Some((
        egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], pixmap.data()),
        baseline,
    ))
}

/// The sample texts offered.
fn sample_presets() -> Vec<String> {
    vec![
        tr("dialog.font_sample"),
        "AaBbCcDdEeFfGg 0123456789".into(),
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ".into(),
        "abcdefghijklmnopqrstuvwxyz".into(),
        "0123456789 !?&@#%$€".into(),
    ]
}

/// The library's name for the status line.
fn library_name(library: &Library, prefs: &FontManagerPrefs) -> String {
    match library {
        Library::MyFonts => tr("font_manager.my_fonts"),
        Library::Folder(p) => p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| p.display().to_string()),
        Library::Favorites => tr("font_manager.favorites"),
        Library::Collection(i) => prefs
            .collections
            .get(*i)
            .map(|c| c.name.clone())
            .unwrap_or_default(),
    }
}

/// The keys of the faces being dragged to a collection.
#[derive(Debug, Clone)]
struct DragFonts(Vec<String>);

/// Things to do after drawing, with the application free.
#[derive(Debug, Clone, PartialEq)]
enum Action {
    AddFolder,
    AddFont,
    Install(Vec<FaceRecord>),
    Uninstall(Vec<FaceRecord>),
    RemoveFolder(PathBuf),
    UseInDrawing(String),
    ToFavorites(Vec<String>),
    FromFavorites(Vec<String>),
    ToCollection(usize, Vec<String>),
    FromCollection(usize, Vec<String>),
    NewCollection,
    DeleteCollection(usize),
    Help,
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut FontManagerState, close: &mut bool) {
    let busy = app
        .font_catalog
        .refresh(&app.settings.font_manager.folders.clone());
    if busy {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    // A library that went away (a removed folder or collection).
    let prefs = app.settings.font_manager.clone();
    let gone = match &st.library {
        Library::Folder(p) => !prefs.folders.contains(p),
        Library::Collection(i) => *i >= prefs.collections.len(),
        _ => false,
    };
    if gone {
        st.library = Library::MyFonts;
    }
    let faces = app.font_catalog.faces(&st.library, &prefs);
    let mut counts: HashMap<String, usize> = HashMap::new();
    for r in &faces {
        *counts.entry(r.key()).or_default() += 1;
    }
    let shown: Vec<FaceRecord> = faces
        .iter()
        .filter(|r| {
            passes(
                r,
                &st.filter_set(),
                &st.search,
                counts.get(&r.key()).copied().unwrap_or(0) > 1,
            )
        })
        .cloned()
        .collect();
    let selected: Vec<FaceRecord> = shown
        .iter()
        .filter(|r| st.selected.contains(&r.key()))
        .cloned()
        .collect();
    let mut actions: Vec<Action> = Vec::new();
    let mut open = true;
    let mut prefs = prefs;
    egui::Window::new(tr("font_manager.title"))
        .id(egui::Id::new("font_manager_window"))
        .order(egui::Order::Foreground)
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size([1000.0, 720.0])
        .min_size([760.0, 480.0])
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .show(ctx, |ui| {
            menu_row(ui, st, &selected, &prefs, &mut actions);
            toolbar(ui, st, &selected, &mut prefs, &mut actions);
            ui.separator();
            let body = ui.available_size();
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(LEFT_W, body.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(LEFT_W);
                        left_panes(ui, st, &mut prefs, &app.font_catalog, &mut actions);
                    },
                );
                ui.separator();
                // The details pane takes its room from the list, never from
                // outside the window: with the separator counted short the
                // window grew a little on every frame (#381).
                let right_w = (ui.available_width()
                    - if prefs.details {
                        DETAILS_W + DETAILS_GAP
                    } else {
                        0.0
                    })
                .max(200.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(right_w, body.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(right_w);
                        ui.vertical(|ui| {
                            sample_and_search(ui, st, &mut prefs);
                            // Fonts the drawing names that are not installed.
                            if !app.missing_fonts.is_empty() {
                                ui.colored_label(
                                    Color32::from_rgb(180, 80, 0),
                                    trf(
                                        "dialog.missing_fonts",
                                        &[("f", &crate::ops2::with_stand_ins(&app.missing_fonts))],
                                    ),
                                );
                            }
                            let list_h = (ui.available_height() - 34.0).max(80.0);
                            let rows = rows(&shown);
                            font_list(ui, app, st, &prefs, &shown, &rows, list_h, &mut actions);
                            let name = library_name(&st.library, &prefs);
                            status_line(ui, st, &mut prefs, &name, shown.len(), busy);
                        });
                    },
                );
                if prefs.details {
                    ui.separator();
                    let details_w = ui.available_width().clamp(0.0, DETAILS_W);
                    ui.allocate_ui_with_layout(
                        Vec2::new(details_w, body.y),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(details_w);
                            let focus = st
                                .focus
                                .as_ref()
                                .and_then(|k| shown.iter().find(|r| &r.key() == k))
                                .or_else(|| selected.first());
                            details(ui, focus);
                        },
                    );
                }
            });
        });
    if !open {
        *close = true;
    }
    let changed = prefs != app.settings.font_manager;
    app.settings.font_manager = prefs;
    for a in actions {
        run(app, st, a);
    }
    if changed {
        app.save_settings();
    }
}

impl FontManagerState {
    fn filter_set(&self) -> BTreeSet<Filter> {
        self.filters.clone()
    }

    fn open(&self, section: &'static str) -> bool {
        !self.collapsed.contains(section)
    }
}

/// Carry out an action.
fn run(app: &mut App, st: &mut FontManagerState, a: Action) {
    let fonts = tracedraw_text::fonts();
    let prefs = &mut app.settings.font_manager;
    match a {
        Action::AddFolder => {
            if let Some(dir) = crate::files::pick_folder() {
                if !prefs.folders.contains(&dir) {
                    prefs.folders.push(dir.clone());
                }
                // Its fonts are usable at once, without Install (#866).
                let files = catalog::font_files(&dir);
                let added = fonts.install_paths(&files);
                log::info!("fonts of {}: {}", dir.display(), added.join(", "));
                st.note = Some(trf(
                    "font_manager.installed_n",
                    &[("n", &files.len().to_string())],
                ));
                app.font_families = fonts.available_families();
                st.library = Library::Folder(dir);
                app.save_settings();
            }
        }
        Action::AddFont => {
            crate::files::Dialog::new()
                .add_filter(tr("font_manager.font_files"), &["ttf", "otf", "ttc", "otc"])
                .pick_file(app, |app, path| {
                    let added = if crate::files::WEB {
                        crate::files::read(&path)
                            .map(|d| tracedraw_text::fonts().install_data(vec![d]))
                            .unwrap_or_default()
                    } else {
                        let p = &mut app.settings.font_manager.installed;
                        if !p.contains(&path) {
                            p.push(path.clone());
                        }
                        tracedraw_text::fonts().install_paths(&[path])
                    };
                    app.font_families = tracedraw_text::fonts().available_families();
                    app.status = trf(
                        "font_manager.installed_n",
                        &[("n", &added.len().to_string())],
                    );
                    app.save_settings();
                });
        }
        Action::Install(faces) => {
            let paths: Vec<PathBuf> = faces
                .iter()
                .filter(|r| !r.installed)
                .filter_map(|r| r.path.clone())
                .collect();
            for p in &paths {
                if !prefs.installed.contains(p) {
                    prefs.installed.push(p.clone());
                }
            }
            let added = fonts.install_paths(&paths);
            st.note = Some(trf(
                "font_manager.installed_n",
                &[("n", &paths.len().to_string())],
            ));
            log::info!("installed for TraceDraw: {}", added.join(", "));
            app.font_families = fonts.available_families();
            app.save_settings();
        }
        Action::Uninstall(faces) => {
            let mut n = 0;
            for r in &faces {
                let Some(p) = &r.path else { continue };
                if r.system || !prefs.installed.contains(p) {
                    continue;
                }
                prefs.installed.retain(|q| q != p);
                if fonts.uninstall_path(p) {
                    n += 1;
                }
            }
            st.note = Some(trf("font_manager.uninstalled_n", &[("n", &n.to_string())]));
            app.font_families = fonts.available_families();
            app.save_settings();
        }
        Action::RemoveFolder(dir) => {
            prefs.folders.retain(|f| f != &dir);
            if remove_folder_fonts(fonts, prefs, &dir) > 0 {
                app.font_families = fonts.available_families();
            }
            app.font_catalog.rescan(&dir);
            if st.library == Library::Folder(dir) {
                st.library = Library::MyFonts;
            }
            app.save_settings();
        }
        Action::UseInDrawing(family) => {
            app.text_font = family.clone();
            app.apply_text_font(&family);
            st.note = Some(trf("font_manager.used", &[("f", &family)]));
        }
        Action::ToFavorites(keys) => {
            for k in keys {
                if !prefs.favorites.contains(&k) {
                    prefs.favorites.push(k);
                }
            }
            app.save_settings();
        }
        Action::FromFavorites(keys) => {
            prefs.favorites.retain(|k| !keys.contains(k));
            app.save_settings();
        }
        Action::ToCollection(i, keys) => {
            if let Some(c) = prefs.collections.get_mut(i) {
                for k in keys {
                    if !c.fonts.contains(&k) {
                        c.fonts.push(k);
                    }
                }
            }
            app.save_settings();
        }
        Action::FromCollection(i, keys) => {
            if let Some(c) = prefs.collections.get_mut(i) {
                c.fonts.retain(|k| !keys.contains(k));
            }
            app.save_settings();
        }
        Action::NewCollection => {
            let n = prefs.collections.len() + 1;
            let name = trf("font_manager.collection_n", &[("n", &n.to_string())]);
            prefs.collections.push(FontCollection {
                name: name.clone(),
                fonts: Vec::new(),
            });
            st.renaming = Some((prefs.collections.len() - 1, name));
            app.save_settings();
        }
        Action::DeleteCollection(i) => {
            if i < prefs.collections.len() {
                prefs.collections.remove(i);
            }
            st.library = Library::MyFonts;
            st.renaming = None;
            app.save_settings();
        }
        Action::Help => app.open_dialog_help(),
    }
}

/// File, Edit, Font, View and Help.
fn menu_row(
    ui: &mut Ui,
    st: &mut FontManagerState,
    selected: &[FaceRecord],
    prefs: &FontManagerPrefs,
    actions: &mut Vec<Action>,
) {
    let keys: Vec<String> = selected.iter().map(|r| r.key()).collect();
    let any = !selected.is_empty();
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button(tr("font_manager.menu_file"), |ui| {
            if ui
                .add_enabled(
                    !crate::files::WEB,
                    egui::Button::new(tr("font_manager.add_folder")),
                )
                .clicked()
            {
                actions.push(Action::AddFolder);
                ui.close();
            }
            if ui.button(tr("font_manager.add_font")).clicked() {
                actions.push(Action::AddFont);
                ui.close();
            }
        });
        ui.menu_button(tr("font_manager.menu_edit"), |ui| {
            ui.add_enabled_ui(any, |ui| {
                ui.menu_button(tr("font_manager.add_to_collection"), |ui| {
                    if ui.button(tr("font_manager.favorites")).clicked() {
                        actions.push(Action::ToFavorites(keys.clone()));
                        ui.close();
                    }
                    for (i, c) in prefs.collections.iter().enumerate() {
                        if ui.button(&c.name).clicked() {
                            actions.push(Action::ToCollection(i, keys.clone()));
                            ui.close();
                        }
                    }
                });
            });
            let from = match st.library {
                Library::Favorites => Some(Action::FromFavorites(keys.clone())),
                Library::Collection(i) => Some(Action::FromCollection(i, keys.clone())),
                _ => None,
            };
            if ui
                .add_enabled(
                    any && from.is_some(),
                    egui::Button::new(tr("font_manager.remove_from_collection")),
                )
                .clicked()
            {
                actions.extend(from);
                ui.close();
            }
            if ui.button(tr("font_manager.select_none")).clicked() {
                st.selected.clear();
                ui.close();
            }
        });
        ui.menu_button(tr("font_manager.menu_font"), |ui| {
            let can_install = selected.iter().any(|r| !r.installed && r.path.is_some());
            if ui
                .add_enabled(can_install, egui::Button::new(tr("font_manager.install")))
                .clicked()
            {
                actions.push(Action::Install(selected.to_vec()));
                ui.close();
            }
            let can_uninstall = selected.iter().any(|r| uninstallable(r, prefs));
            if ui
                .add_enabled(
                    can_uninstall,
                    egui::Button::new(tr("font_manager.uninstall")),
                )
                .clicked()
            {
                actions.push(Action::Uninstall(selected.to_vec()));
                ui.close();
            }
            let one = selected.first().filter(|r| r.installed);
            if ui
                .add_enabled(
                    one.is_some(),
                    egui::Button::new(tr("font_manager.use_in_drawing")),
                )
                .clicked()
            {
                if let Some(r) = one {
                    actions.push(Action::UseInDrawing(r.family.clone()));
                }
                ui.close();
            }
        });
        ui.menu_button(tr("font_manager.menu_view"), |ui| {
            let mut details = prefs.details;
            if ui
                .checkbox(&mut details, tr("font_manager.show_details"))
                .changed()
            {
                // Applied by the toolbar, which owns the settings copy.
                st.collapsed.insert("__toggle_details");
                ui.close();
            }
        });
        ui.menu_button(tr("font_manager.menu_help"), |ui| {
            if ui.button(tr("font_manager.help")).clicked() {
                actions.push(Action::Help);
                ui.close();
            }
        });
    });
}

/// A face installed for TraceDraw (not one of the system's).
fn uninstallable(r: &FaceRecord, prefs: &FontManagerPrefs) -> bool {
    r.installed && !r.system && r.path.as_ref().is_some_and(|p| prefs.installed.contains(p))
}

/// The toolbar's icons.
#[derive(Clone, Copy, PartialEq)]
enum Icon {
    AddFolder,
    AddFont,
    Install,
    Delete,
    AddToCollection,
    RemoveFromCollection,
    Uninstall,
    Details,
    SampleOptions,
}

fn icon_button(ui: &mut Ui, icon: Icon, tip: &str, enabled: bool, on: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::splat(30.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let p = ui.painter();
    if on || (enabled && resp.hovered()) {
        p.rect(
            rect.shrink(1.0),
            2.0,
            Tokens::tool_hover(),
            Stroke::new(1.0, Tokens::ACCENT),
            egui::StrokeKind::Inside,
        );
    }
    let c = if enabled {
        Tokens::icon()
    } else {
        Tokens::icon_off()
    };
    let s = Stroke::new(1.4, c);
    let r = rect.shrink(7.0);
    let letter = |p: &egui::Painter, at: egui::Pos2| {
        p.text(
            at,
            egui::Align2::CENTER_CENTER,
            "A",
            crate::theme::bold(13.0),
            c,
        );
    };
    let plus = |p: &egui::Painter, at: egui::Pos2, color: Color32| {
        p.line_segment(
            [at - Vec2::new(4.0, 0.0), at + Vec2::new(4.0, 0.0)],
            Stroke::new(2.0, color),
        );
        p.line_segment(
            [at - Vec2::new(0.0, 4.0), at + Vec2::new(0.0, 4.0)],
            Stroke::new(2.0, color),
        );
    };
    let green = if enabled {
        Color32::from_rgb(0x2E, 0x9E, 0x3C)
    } else {
        c
    };
    let red = if enabled {
        Color32::from_rgb(0xD0, 0x30, 0x30)
    } else {
        c
    };
    match icon {
        Icon::AddFolder => {
            p.rect_stroke(
                Rect::from_min_max(r.left_top() + Vec2::new(0.0, 3.0), r.right_bottom()),
                1.0,
                s,
                egui::StrokeKind::Inside,
            );
            letter(p, r.center() + Vec2::new(-1.0, 2.0));
            plus(p, r.right_top() + Vec2::new(0.0, 1.0), green);
        }
        Icon::AddFont => {
            p.rect_stroke(r, 1.0, s, egui::StrokeKind::Inside);
            letter(p, r.center());
            plus(p, r.right_top() + Vec2::new(0.0, 1.0), green);
        }
        Icon::Install => {
            let top = r.center_top();
            let tip_y = r.bottom() - 4.0;
            p.line_segment([top, egui::pos2(top.x, tip_y)], Stroke::new(2.0, c));
            p.line_segment(
                [
                    egui::pos2(top.x - 5.0, tip_y - 5.0),
                    egui::pos2(top.x, tip_y),
                ],
                Stroke::new(2.0, c),
            );
            p.line_segment(
                [
                    egui::pos2(top.x + 5.0, tip_y - 5.0),
                    egui::pos2(top.x, tip_y),
                ],
                Stroke::new(2.0, c),
            );
            p.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(2.0, c));
            plus(p, r.right_top() + Vec2::new(0.0, 1.0), green);
        }
        Icon::Delete => {
            let body = Rect::from_min_max(
                r.left_top() + Vec2::new(2.0, 4.0),
                r.right_bottom() - Vec2::new(2.0, 0.0),
            );
            p.rect_stroke(body, 1.0, s, egui::StrokeKind::Inside);
            p.line_segment(
                [
                    r.left_top() + Vec2::new(0.0, 2.0),
                    r.right_top() + Vec2::new(0.0, 2.0),
                ],
                s,
            );
            for dx in [-3.0, 0.0, 3.0] {
                p.line_segment(
                    [
                        body.center_top() + Vec2::new(dx, 3.0),
                        body.center_bottom() + Vec2::new(dx, -3.0),
                    ],
                    Stroke::new(1.0, c),
                );
            }
        }
        Icon::AddToCollection | Icon::RemoveFromCollection | Icon::Uninstall => {
            p.rect_stroke(r, 1.0, s, egui::StrokeKind::Inside);
            letter(p, r.center());
            let at = r.right_top() + Vec2::new(0.0, 1.0);
            match icon {
                Icon::AddToCollection => plus(p, at, c),
                Icon::RemoveFromCollection => {
                    p.line_segment(
                        [at - Vec2::new(4.0, 0.0), at + Vec2::new(4.0, 0.0)],
                        Stroke::new(2.0, c),
                    );
                }
                _ => {
                    p.line_segment(
                        [at - Vec2::new(3.5, 3.5), at + Vec2::new(3.5, 3.5)],
                        Stroke::new(2.0, red),
                    );
                    p.line_segment(
                        [at + Vec2::new(-3.5, 3.5), at + Vec2::new(3.5, -3.5)],
                        Stroke::new(2.0, red),
                    );
                }
            }
        }
        Icon::Details => {
            p.rect_stroke(r, 1.0, s, egui::StrokeKind::Inside);
            p.line_segment(
                [
                    r.center_top() + Vec2::new(2.0, 0.0),
                    r.center_bottom() + Vec2::new(2.0, 0.0),
                ],
                s,
            );
            for dy in [3.0, 7.0, 11.0] {
                p.line_segment(
                    [
                        r.left_top() + Vec2::new(3.0, dy),
                        r.left_top() + Vec2::new(7.0, dy),
                    ],
                    Stroke::new(1.0, c),
                );
            }
        }
        Icon::SampleOptions => {
            p.text(
                r.left_top() + Vec2::new(0.0, -1.0),
                egui::Align2::LEFT_TOP,
                "A",
                crate::theme::bold(11.0),
                c,
            );
            for dy in [8.0, 11.0, 14.0] {
                p.line_segment(
                    [
                        r.left_top() + Vec2::new(6.0, dy),
                        r.right_top() + Vec2::new(0.0, dy),
                    ],
                    Stroke::new(1.2, c),
                );
            }
        }
    }
    resp.on_hover_text(tip)
}

fn toolbar(
    ui: &mut Ui,
    st: &mut FontManagerState,
    selected: &[FaceRecord],
    prefs: &mut FontManagerPrefs,
    actions: &mut Vec<Action>,
) {
    if st.collapsed.remove("__toggle_details") {
        prefs.details = !prefs.details;
    }
    let keys: Vec<String> = selected.iter().map(|r| r.key()).collect();
    let any = !selected.is_empty();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if icon_button(
            ui,
            Icon::AddFolder,
            &tr("font_manager.add_folder"),
            !crate::files::WEB,
            false,
        )
        .clicked()
        {
            actions.push(Action::AddFolder);
        }
        if icon_button(ui, Icon::AddFont, &tr("font_manager.add_font"), true, false).clicked() {
            actions.push(Action::AddFont);
        }
        let can_install = selected.iter().any(|r| !r.installed && r.path.is_some());
        if icon_button(
            ui,
            Icon::Install,
            &tr("font_manager.install"),
            can_install,
            false,
        )
        .clicked()
        {
            actions.push(Action::Install(selected.to_vec()));
        }
        ui.separator();
        // Delete: out of the collection shown, or the folder out of the
        // libraries.
        let delete = match &st.library {
            Library::Favorites if any => Some(Action::FromFavorites(keys.clone())),
            Library::Collection(i) if any => Some(Action::FromCollection(*i, keys.clone())),
            Library::Folder(dir) => Some(Action::RemoveFolder(dir.clone())),
            _ => None,
        };
        if icon_button(
            ui,
            Icon::Delete,
            &tr("font_manager.delete"),
            delete.is_some(),
            false,
        )
        .clicked()
        {
            actions.extend(delete);
        }
        ui.separator();
        let collection = match st.library {
            Library::Collection(i) => Some(i),
            _ => None,
        };
        let add_to = icon_button(
            ui,
            Icon::AddToCollection,
            &tr("font_manager.add_to_collection"),
            any,
            false,
        );
        egui::Popup::menu(&add_to)
            .id(egui::Id::new("font_manager_add_to"))
            .show(|ui| {
                if ui.button(tr("font_manager.favorites")).clicked() {
                    actions.push(Action::ToFavorites(keys.clone()));
                    ui.close();
                }
                for (i, c) in prefs.collections.iter().enumerate() {
                    if ui.button(&c.name).clicked() {
                        actions.push(Action::ToCollection(i, keys.clone()));
                        ui.close();
                    }
                }
                if ui.button(tr("font_manager.new_collection")).clicked() {
                    actions.push(Action::NewCollection);
                    ui.close();
                }
            });
        let from = match (st.library.clone(), collection) {
            (Library::Favorites, _) => Some(Action::FromFavorites(keys.clone())),
            (_, Some(i)) => Some(Action::FromCollection(i, keys.clone())),
            _ => None,
        };
        if icon_button(
            ui,
            Icon::RemoveFromCollection,
            &tr("font_manager.remove_from_collection"),
            any && from.is_some(),
            false,
        )
        .clicked()
        {
            actions.extend(from);
        }
        let can_uninstall = selected.iter().any(|r| uninstallable(r, prefs));
        if icon_button(
            ui,
            Icon::Uninstall,
            &tr("font_manager.uninstall"),
            can_uninstall,
            false,
        )
        .clicked()
        {
            actions.push(Action::Uninstall(selected.to_vec()));
        }
        ui.separator();
        if icon_button(
            ui,
            Icon::Details,
            &tr("font_manager.show_details"),
            true,
            prefs.details,
        )
        .clicked()
        {
            prefs.details = !prefs.details;
        }
        let opts = icon_button(
            ui,
            Icon::SampleOptions,
            &tr("font_manager.sample_options"),
            true,
            false,
        );
        egui::Popup::menu(&opts)
            .id(egui::Id::new("font_manager_sample_opts"))
            .show(|ui| {
                if ui.button(tr("font_manager.sample_family")).clicked() {
                    prefs.sample.clear();
                    ui.close();
                }
                for s in sample_presets() {
                    if ui.button(&s).clicked() {
                        prefs.sample = s;
                        ui.close();
                    }
                }
            });
    });
}

/// A blue section title with a line and an arrow that folds it.
fn section(
    ui: &mut Ui,
    st: &mut FontManagerState,
    id: &'static str,
    title: &str,
    add: impl FnOnce(&mut Ui, &mut FontManagerState),
) {
    let open = st.open(id);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::click());
    let p = ui.painter();
    let galley = p.layout_no_wrap(
        title.to_string(),
        egui::FontId::proportional(13.0),
        SECTION_BLUE,
    );
    let text_w = galley.size().x;
    p.galley(
        egui::pos2(rect.left() + 2.0, rect.center().y - galley.size().y / 2.0),
        galley,
        SECTION_BLUE,
    );
    let line_y = rect.center().y;
    p.line_segment(
        [
            egui::pos2(rect.left() + text_w + 8.0, line_y),
            egui::pos2(rect.right() - 22.0, line_y),
        ],
        Stroke::new(1.0, SECTION_BLUE),
    );
    let a = egui::pos2(rect.right() - 11.0, line_y);
    let pts = if open {
        vec![
            a + Vec2::new(-5.0, 3.0),
            a + Vec2::new(0.0, -3.0),
            a + Vec2::new(5.0, 3.0),
        ]
    } else {
        vec![
            a + Vec2::new(-5.0, -3.0),
            a + Vec2::new(0.0, 3.0),
            a + Vec2::new(5.0, -3.0),
        ]
    };
    p.add(egui::Shape::line(pts, Stroke::new(1.5, SECTION_BLUE)));
    if resp.clicked() {
        if open {
            st.collapsed.insert(id);
        } else {
            st.collapsed.remove(id);
        }
    }
    if open {
        add(ui, st);
    }
}

/// A row of the libraries: picked when it is the one shown.
fn library_row(ui: &mut Ui, label: &str, picked: bool) -> egui::Response {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::click());
    let p = ui.painter();
    if picked {
        p.rect(
            rect,
            0.0,
            Tokens::selected_row(),
            Stroke::NONE,
            egui::StrokeKind::Inside,
        );
    } else if resp.hovered() {
        p.rect_filled(rect, 0.0, Tokens::hovered_row());
    }
    p.text(
        rect.left_center() + Vec2::new(6.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        Tokens::text(),
    );
    resp
}

fn left_panes(
    ui: &mut Ui,
    st: &mut FontManagerState,
    prefs: &mut FontManagerPrefs,
    cat: &Catalog,
    actions: &mut Vec<Action>,
) {
    let h = ui.available_height();
    // Libraries.
    egui::Frame::new()
        .fill(Tokens::field())
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ui.set_width(LEFT_W - 12.0);
            ui.set_height(h * 0.38);
            ui.label(
                egui::RichText::new(tr("font_manager.libraries")).font(crate::theme::bold(13.0)),
            );
            egui::ScrollArea::vertical()
                .id_salt("fm_libraries")
                .show(ui, |ui| {
                    section(ui, st, "folders", &tr("font_manager.folders"), |ui, st| {
                        if library_row(
                            ui,
                            &tr("font_manager.my_fonts"),
                            st.library == Library::MyFonts,
                        )
                        .clicked()
                        {
                            st.library = Library::MyFonts;
                        }
                        for dir in prefs.folders.clone() {
                            let name = dir
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| dir.display().to_string());
                            let lib = Library::Folder(dir.clone());
                            let busy = !cat.folders.contains_key(&dir);
                            let label = if busy {
                                format!("{name} \u{2026}")
                            } else {
                                name
                            };
                            let r = library_row(ui, &label, st.library == lib)
                                .on_hover_text(dir.display().to_string());
                            if r.clicked() {
                                st.library = lib;
                            }
                            r.context_menu(|ui| {
                                if ui.button(tr("font_manager.remove_folder")).clicked() {
                                    actions.push(Action::RemoveFolder(dir.clone()));
                                    ui.close();
                                }
                            });
                        }
                    });
                    section(
                        ui,
                        st,
                        "collections",
                        &tr("font_manager.my_collections"),
                        |ui, st| {
                            let fav = library_row(
                                ui,
                                &tr("font_manager.favorites"),
                                st.library == Library::Favorites,
                            );
                            if fav.clicked() {
                                st.library = Library::Favorites;
                            }
                            if let Some(DragFonts(keys)) =
                                fav.dnd_release_payload::<DragFonts>().map(|p| (*p).clone())
                            {
                                actions.push(Action::ToFavorites(keys));
                            }
                            for i in 0..prefs.collections.len() {
                                if let Some((at, name)) = &mut st.renaming {
                                    if *at == i {
                                        let r = ui.add(
                                            egui::TextEdit::singleline(name)
                                                .desired_width(f32::INFINITY),
                                        );
                                        r.request_focus();
                                        if r.lost_focus() {
                                            let n = name.trim().to_string();
                                            if !n.is_empty() {
                                                prefs.collections[i].name = n;
                                            }
                                            st.renaming = None;
                                        }
                                        continue;
                                    }
                                }
                                let lib = Library::Collection(i);
                                let r =
                                    library_row(ui, &prefs.collections[i].name, st.library == lib);
                                if r.clicked() {
                                    st.library = lib;
                                }
                                if r.double_clicked() {
                                    st.renaming = Some((i, prefs.collections[i].name.clone()));
                                }
                                if let Some(DragFonts(keys)) =
                                    r.dnd_release_payload::<DragFonts>().map(|p| (*p).clone())
                                {
                                    actions.push(Action::ToCollection(i, keys));
                                }
                                r.context_menu(|ui| {
                                    if ui.button(tr("font_manager.rename")).clicked() {
                                        st.renaming = Some((i, prefs.collections[i].name.clone()));
                                        ui.close();
                                    }
                                    if ui.button(tr("font_manager.delete_collection")).clicked() {
                                        actions.push(Action::DeleteCollection(i));
                                        ui.close();
                                    }
                                });
                            }
                            if ui.small_button(tr("font_manager.new_collection")).clicked() {
                                actions.push(Action::NewCollection);
                            }
                        },
                    );
                });
        });
    ui.add_space(6.0);
    // Filters.
    egui::Frame::new()
        .fill(Tokens::field())
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ui.set_width(LEFT_W - 12.0);
            ui.set_min_height((ui.available_height() - 6.0).max(60.0));
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(tr("font_manager.filters")).font(crate::theme::bold(13.0)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            !st.filters.is_empty(),
                            egui::Button::new(tr("font_manager.clear_filters")).small(),
                        )
                        .clicked()
                    {
                        st.filters.clear();
                    }
                });
            });
            egui::ScrollArea::vertical()
                .id_salt("fm_filters")
                .show(ui, |ui| {
                    let boxes =
                        |ui: &mut Ui, st: &mut FontManagerState, items: Vec<(Filter, String)>| {
                            for (f, label) in items {
                                let mut on = st.filters.contains(&f);
                                if ui.checkbox(&mut on, label).changed() {
                                    if on {
                                        st.filters.insert(f);
                                    } else {
                                        st.filters.remove(&f);
                                    }
                                }
                            }
                        };
                    section(
                        ui,
                        st,
                        "status",
                        &tr("font_manager.font_status"),
                        |ui, st| {
                            boxes(
                                ui,
                                st,
                                vec![
                                    (Filter::Installed, tr("font_manager.installed")),
                                    (Filter::NotInstalled, tr("font_manager.not_installed")),
                                    (Filter::Protected, tr("font_manager.protected")),
                                    (Filter::Duplicates, tr("font_manager.duplicates")),
                                ],
                            );
                        },
                    );
                    section(
                        ui,
                        st,
                        "embedding",
                        &tr("font_manager.embedding_rights"),
                        |ui, st| {
                            boxes(
                                ui,
                                st,
                                Embedding::ALL
                                    .iter()
                                    .map(|e| (Filter::Embedding(*e), embedding_name(*e)))
                                    .collect(),
                            );
                        },
                    );
                    section(
                        ui,
                        st,
                        "technology",
                        &tr("font_manager.technology"),
                        |ui, st| {
                            boxes(
                                ui,
                                st,
                                vec![
                                    (
                                        Filter::Tech(FontKind::TrueType),
                                        tr("font_manager.tech_truetype"),
                                    ),
                                    (
                                        Filter::Tech(FontKind::OpenType),
                                        tr("font_manager.tech_opentype"),
                                    ),
                                    (
                                        Filter::Tech(FontKind::Variable),
                                        tr("font_manager.tech_variable"),
                                    ),
                                ],
                            );
                        },
                    );
                    section(ui, st, "weight", &tr("font_manager.weight"), |ui, st| {
                        boxes(
                            ui,
                            st,
                            (1..=9)
                                .map(|w| (Filter::Weight(w * 100), weight_name(w * 100)))
                                .collect(),
                        );
                    });
                    section(ui, st, "width", &tr("font_manager.width"), |ui, st| {
                        boxes(
                            ui,
                            st,
                            vec![
                                (Filter::Width(0), tr("font_manager.width_condensed")),
                                (Filter::Width(1), tr("font_manager.width_normal")),
                                (Filter::Width(2), tr("font_manager.width_expanded")),
                            ],
                        );
                    });
                    section(ui, st, "style", &tr("font_manager.style"), |ui, st| {
                        boxes(
                            ui,
                            st,
                            vec![
                                (Filter::Italic(false), tr("font_manager.upright")),
                                (Filter::Italic(true), tr("font_manager.italic")),
                            ],
                        );
                    });
                    section(
                        ui,
                        st,
                        "scripts",
                        &tr("font_manager.character_range"),
                        |ui, st| {
                            boxes(
                                ui,
                                st,
                                Script::ALL
                                    .iter()
                                    .map(|s| (Filter::Script(*s), script_name(*s)))
                                    .collect(),
                            );
                        },
                    );
                    section(
                        ui,
                        st,
                        "features",
                        &tr("font_manager.opentype"),
                        |ui, st| {
                            boxes(
                                ui,
                                st,
                                FEATURES
                                    .iter()
                                    .enumerate()
                                    .map(|(i, f)| (Filter::Feature(i), feature_name(f)))
                                    .collect(),
                            );
                        },
                    );
                });
        });
}

pub fn embedding_name(e: Embedding) -> String {
    tr(match e {
        Embedding::Installable => "font_manager.emb_installable",
        Embedding::Editable => "font_manager.emb_editable",
        Embedding::PreviewPrint => "font_manager.emb_preview",
        Embedding::Restricted => "font_manager.emb_restricted",
        Embedding::BitmapOnly => "font_manager.emb_bitmap",
    })
}

fn weight_name(w: u16) -> String {
    tr(match w {
        100 => "font_manager.w_thin",
        200 => "font_manager.w_extralight",
        300 => "font_manager.w_light",
        400 => "font_manager.w_regular",
        500 => "font_manager.w_medium",
        600 => "font_manager.w_semibold",
        700 => "font_manager.w_bold",
        800 => "font_manager.w_extrabold",
        _ => "font_manager.w_black",
    })
}

pub fn script_name(s: Script) -> String {
    tr(match s {
        Script::Latin => "font_manager.s_latin",
        Script::Greek => "font_manager.s_greek",
        Script::Cyrillic => "font_manager.s_cyrillic",
        Script::Hebrew => "font_manager.s_hebrew",
        Script::Arabic => "font_manager.s_arabic",
        Script::Devanagari => "font_manager.s_devanagari",
        Script::Bengali => "font_manager.s_bengali",
        Script::Thai => "font_manager.s_thai",
        Script::Korean => "font_manager.s_korean",
        Script::Japanese => "font_manager.s_japanese",
        Script::Chinese => "font_manager.s_chinese",
        Script::Symbols => "font_manager.s_symbols",
    })
}

fn feature_name(tag: &str) -> String {
    let key = format!("font_manager.f_{tag}");
    format!("{} ({tag})", tr(&key))
}

fn kind_name(k: FontKind) -> String {
    tr(match k {
        FontKind::TrueType => "font_manager.tech_truetype",
        FontKind::Variable => "font_manager.tech_variable",
        _ => "font_manager.tech_opentype",
    })
}

/// The sample text box (with presets) and the search field.
fn sample_and_search(ui: &mut Ui, st: &mut FontManagerState, prefs: &mut FontManagerPrefs) {
    ui.horizontal(|ui| {
        let w = (ui.available_width() * 0.6).max(160.0);
        ui.add(
            egui::TextEdit::singleline(&mut prefs.sample)
                .hint_text(tr("font_manager.sample_hint"))
                .desired_width(w - 28.0),
        );
        let arrow = ui.add_sized([22.0, 20.0], egui::Button::new("\u{25BE}"));
        egui::Popup::menu(&arrow)
            .id(egui::Id::new("font_manager_samples"))
            .show(|ui| {
                for s in sample_presets() {
                    if ui.button(&s).clicked() {
                        prefs.sample = s;
                        ui.close();
                    }
                }
            });
        ui.add(
            egui::TextEdit::singleline(&mut st.search)
                .hint_text(tr("font_manager.search"))
                .desired_width(ui.available_width()),
        );
    });
}

#[allow(clippy::too_many_arguments)]
fn font_list(
    ui: &mut Ui,
    app: &mut App,
    st: &mut FontManagerState,
    prefs: &FontManagerPrefs,
    faces: &[FaceRecord],
    rows: &[Row],
    height: f32,
    actions: &mut Vec<Action>,
) {
    let em = prefs.sample_px();
    let card_h = em * 1.45 + CARD_EXTRA;
    let header_h = HEADER_H.min(em * 1.2 + 26.0).max(40.0);
    let heights: Vec<f32> = rows
        .iter()
        .map(|r| match r {
            Row::Family(_) => header_h,
            Row::Face(_) => card_h,
        })
        .collect();
    let mut tops = Vec::with_capacity(rows.len() + 1);
    let mut y = 0.0;
    for h in &heights {
        tops.push(y);
        y += h;
    }
    let total = y;
    // The search field sits on the sample row's right.
    let ctx = ui.ctx().clone();
    egui::Frame::new().fill(Tokens::field()).show(ui, |ui| {
        ui.set_height(height);
        if faces.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new(tr("font_manager.nothing")).color(Tokens::text_dim()));
            });
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("fm_list")
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                let width = ui.available_width();
                let (area, _) = ui.allocate_exact_size(Vec2::new(width, total), Sense::hover());
                let first = tops.partition_point(|t| *t + card_h.max(header_h) < viewport.min.y);
                for (i, row) in rows.iter().enumerate().skip(first) {
                    if tops[i] > viewport.max.y {
                        break;
                    }
                    let rect = Rect::from_min_size(
                        egui::pos2(area.left(), area.top() + tops[i]),
                        Vec2::new(width, heights[i]),
                    );
                    match row {
                        Row::Family(name) => {
                            let picture = faces
                                .iter()
                                .find(|r| &r.family == name)
                                .cloned()
                                .and_then(|r| {
                                    app.font_catalog.picture(
                                        &ctx,
                                        &r,
                                        name,
                                        (header_h - 22.0).max(16.0),
                                    )
                                });
                            let p = ui.painter_at(rect);
                            match picture {
                                Some(pic) => {
                                    let size = pic.texture.size_vec2();
                                    let at = egui::pos2(
                                        rect.left() + 4.0,
                                        rect.center().y - size.y / 2.0,
                                    );
                                    p.image(
                                        pic.texture.id(),
                                        Rect::from_min_size(at, size),
                                        Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        // The family's name, faded.
                                        Tokens::text().gamma_multiply(120.0 / 255.0),
                                    );
                                }
                                None => {
                                    p.text(
                                        rect.left_center() + Vec2::new(4.0, 0.0),
                                        egui::Align2::LEFT_CENTER,
                                        name,
                                        egui::FontId::proportional(28.0),
                                        Tokens::text_dim(),
                                    );
                                }
                            }
                        }
                        Row::Face(fi) => {
                            let r = &faces[*fi];
                            card(ui, app, st, prefs, r, rect, em, actions);
                        }
                    }
                }
            });
    });
}

/// The colour a face's sample is painted in: the theme's text colour
/// (light on the dark themes, #380), the highlight's text colour on a
/// chosen card in the high contrast theme.
fn sample_ink(picked: bool) -> Color32 {
    if picked && crate::theme::theme() == crate::theme::Theme::HighContrast {
        crate::theme::on_highlight()
    } else {
        Tokens::text()
    }
}

#[allow(clippy::too_many_arguments)]
fn card(
    ui: &mut Ui,
    app: &mut App,
    st: &mut FontManagerState,
    prefs: &FontManagerPrefs,
    r: &FaceRecord,
    rect: Rect,
    em: f32,
    actions: &mut Vec<Action>,
) {
    let key = r.key();
    let picked = st.selected.contains(&key);
    let card = rect.shrink2(Vec2::new(2.0, 2.0));
    let id = egui::Id::new(("fm_card", &key, r.path.clone(), r.index));
    let resp = ui.interact(card, id, Sense::click_and_drag());
    let p = ui.painter_at(rect);
    let (fill, line) = if picked {
        (Tokens::selected_row(), Tokens::ACCENT)
    } else if resp.hovered() {
        (Tokens::hovered_row(), Tokens::border())
    } else {
        (Tokens::field(), Tokens::border())
    };
    p.rect(
        card,
        0.0,
        fill,
        Stroke::new(1.0, line),
        egui::StrokeKind::Inside,
    );
    let title = format!("{} ({}, {})", r.family, r.style, r.width_name());
    p.text(
        card.left_top() + Vec2::new(46.0, 8.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(13.0),
        Tokens::text_dim(),
    );
    let sample = if prefs.sample.trim().is_empty() {
        r.family.clone()
    } else {
        prefs.sample.clone()
    };
    let ctx = ui.ctx().clone();
    if let Some(pic) = app.font_catalog.picture(&ctx, r, &sample, em) {
        let size = pic.texture.size_vec2();
        let top = card.top() + 28.0 + em * 1.05 - pic.baseline;
        let max_w = card.width() - 46.0 - 60.0;
        let shown = Vec2::new(size.x.min(max_w), size.y);
        let uv_w = if size.x > 0.0 { shown.x / size.x } else { 1.0 };
        p.image(
            pic.texture.id(),
            Rect::from_min_size(egui::pos2(card.left() + 46.0, top), shown),
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(uv_w, 1.0)),
            sample_ink(picked),
        );
    }
    // How it is built, then the installed bar.
    let bar = Rect::from_min_max(
        egui::pos2(card.right() - 16.0, card.top() + 3.0),
        egui::pos2(card.right() - 3.0, card.bottom() - 3.0),
    );
    p.rect_filled(
        bar,
        0.0,
        if r.installed {
            INSTALLED
        } else {
            NOT_INSTALLED
        },
    );
    if r.system {
        // A small lock on the bar.
        let c = bar.center();
        p.rect_filled(
            Rect::from_center_size(c + Vec2::new(0.0, 2.0), Vec2::new(8.0, 6.0)),
            1.0,
            Color32::WHITE,
        );
        p.circle_stroke(
            c + Vec2::new(0.0, -2.0),
            2.6,
            Stroke::new(1.3, Color32::WHITE),
        );
    }
    let glyph = if r.kind == FontKind::TrueType {
        "TT"
    } else {
        "O"
    };
    let mut job = egui::text::LayoutJob::default();
    job.append(
        glyph,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(24.0),
            color: Tokens::text_dim(),
            italics: true,
            ..Default::default()
        },
    );
    let galley = ui.painter().layout_job(job);
    let gs = galley.size();
    p.galley(
        egui::pos2(bar.left() - 14.0 - gs.x, card.center().y - gs.y / 2.0),
        galley,
        Tokens::text_dim(),
    );
    // Picking, using, dragging and the menu.
    if resp.clicked() {
        let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
        if ctrl {
            if !st.selected.remove(&key) {
                st.selected.insert(key.clone());
            }
        } else {
            st.selected.clear();
            st.selected.insert(key.clone());
        }
        st.focus = Some(key.clone());
    }
    if resp.double_clicked() && r.installed {
        actions.push(Action::UseInDrawing(r.family.clone()));
    }
    if resp.drag_started() {
        let keys: Vec<String> = if picked {
            st.selected.iter().cloned().collect()
        } else {
            vec![key.clone()]
        };
        resp.dnd_set_drag_payload(DragFonts(keys));
    }
    let status = if r.installed {
        tr("font_manager.installed")
    } else {
        tr("font_manager.not_installed")
    };
    let resp = resp.on_hover_ui(|ui| {
        ui.label(egui::RichText::new(&r.family).font(crate::theme::bold(12.0)));
        ui.horizontal(|ui| {
            let (sq, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
            ui.painter().rect_filled(
                sq,
                0.0,
                if r.installed {
                    INSTALLED
                } else {
                    NOT_INSTALLED
                },
            );
            ui.label(&status);
        });
    });
    let keys: Vec<String> = if picked {
        st.selected.iter().cloned().collect()
    } else {
        vec![key.clone()]
    };
    let one = vec![r.clone()];
    resp.context_menu(|ui| {
        if !r.installed && r.path.is_some() && ui.button(tr("font_manager.install")).clicked() {
            actions.push(Action::Install(one.clone()));
            ui.close();
        }
        if uninstallable(r, prefs) && ui.button(tr("font_manager.uninstall")).clicked() {
            actions.push(Action::Uninstall(one.clone()));
            ui.close();
        }
        if r.installed && ui.button(tr("font_manager.use_in_drawing")).clicked() {
            actions.push(Action::UseInDrawing(r.family.clone()));
            ui.close();
        }
        if prefs.favorites.contains(&key) {
            if ui.button(tr("font_manager.remove_favorite")).clicked() {
                actions.push(Action::FromFavorites(keys.clone()));
                ui.close();
            }
        } else if ui.button(tr("font_manager.add_favorite")).clicked() {
            actions.push(Action::ToFavorites(keys.clone()));
            ui.close();
        }
        if !prefs.collections.is_empty() {
            ui.menu_button(tr("font_manager.add_to_collection"), |ui| {
                for (i, c) in prefs.collections.iter().enumerate() {
                    if ui.button(&c.name).clicked() {
                        actions.push(Action::ToCollection(i, keys.clone()));
                        ui.close();
                    }
                }
            });
        }
        if let Library::Collection(i) = st.library {
            if ui
                .button(tr("font_manager.remove_from_collection"))
                .clicked()
            {
                actions.push(Action::FromCollection(i, keys.clone()));
                ui.close();
            }
        }
        if ui.button(tr("font_manager.show_details")).clicked() {
            st.focus = Some(key.clone());
            st.collapsed.insert("__toggle_details");
            ui.close();
        }
    });
}

/// The count of fonts shown and the sample size slider.
fn status_line(
    ui: &mut Ui,
    st: &mut FontManagerState,
    prefs: &mut FontManagerPrefs,
    library: &str,
    n: usize,
    busy: bool,
) {
    ui.horizontal(|ui| {
        ui.set_height(30.0);
        let text = if busy {
            tr("font_manager.loading")
        } else {
            st.note.clone().unwrap_or_else(|| {
                trf(
                    "font_manager.count",
                    &[("library", library), ("n", &n.to_string())],
                )
            })
        };
        ui.label(text);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new("A").size(18.0));
            let mut px = prefs.sample_px();
            if ui
                .add(egui::Slider::new(&mut px, SAMPLE_PX).show_value(false))
                .on_hover_text(tr("font_manager.sample_size"))
                .changed()
            {
                prefs.sample_px = px;
            }
            ui.label(egui::RichText::new("A").size(11.0));
        });
    });
}

/// The details of the focused face.
fn details(ui: &mut Ui, r: Option<&FaceRecord>) {
    ui.label(egui::RichText::new(tr("font_manager.details")).font(crate::theme::bold(13.0)));
    ui.separator();
    let Some(r) = r else {
        ui.label(egui::RichText::new(tr("font_manager.details_none")).color(Tokens::text_dim()));
        return;
    };
    egui::ScrollArea::vertical()
        .id_salt("fm_details")
        .show(ui, |ui| {
            egui::Grid::new("fm_details_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    let mut row = |k: &str, v: String| {
                        ui.label(egui::RichText::new(tr(k)).color(Tokens::text_dim()));
                        ui.add(egui::Label::new(v).wrap());
                        ui.end_row();
                    };
                    row("font_manager.d_family", r.family.clone());
                    row("font_manager.d_style", r.style.clone());
                    row("font_manager.d_postscript", r.postscript.clone());
                    row("font_manager.d_version", r.version.clone());
                    row(
                        "font_manager.d_file",
                        r.path
                            .as_deref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| tr("font_manager.d_memory")),
                    );
                    row("font_manager.d_technology", kind_name(r.kind));
                    row("font_manager.d_embedding", embedding_name(r.embedding));
                    row(
                        "font_manager.d_weight",
                        format!("{} ({})", weight_name(weight_class(r.weight)), r.weight),
                    );
                    row("font_manager.d_width", r.width_name().to_string());
                    row("font_manager.d_glyphs", r.glyphs.to_string());
                    row(
                        "font_manager.d_scripts",
                        r.scripts
                            .iter()
                            .map(|s| script_name(*s))
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    row("font_manager.d_features", r.features.join(", "));
                    row(
                        "font_manager.d_status",
                        if r.installed {
                            tr("font_manager.installed")
                        } else {
                            tr("font_manager.not_installed")
                        },
                    );
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(family: &str, style: &str, weight: u16, italic: bool) -> FaceRecord {
        FaceRecord {
            family: family.into(),
            style: style.into(),
            weight,
            width: 5,
            italic,
            kind: FontKind::TrueType,
            embedding: Embedding::Installable,
            glyphs: 200,
            scripts: vec![Script::Latin],
            features: vec!["kern".into()],
            version: "Version 1.0".into(),
            postscript: format!("{}-{}", family.replace(' ', ""), style.replace(' ', "")),
            path: None,
            index: 0,
            installed: true,
            system: false,
            handle: None,
        }
    }

    #[test]
    fn rows_group_faces_by_family_upright_first() {
        let faces = vec![
            face("Zeta", "Regular", 400, false),
            face("Alpha", "Bold Italic", 700, true),
            face("Alpha", "Bold", 700, false),
            face("Alpha", "Regular", 400, false),
        ];
        let rows = rows(&faces);
        assert_eq!(
            rows,
            vec![
                Row::Family("Alpha".into()),
                Row::Face(3),
                Row::Face(2),
                Row::Face(1),
                Row::Family("Zeta".into()),
                Row::Face(0),
            ]
        );
    }

    #[test]
    fn filters_combine_within_and_across_sections() {
        let mut light = face("Alpha", "Light", 300, false);
        light.kind = FontKind::OpenType;
        light.installed = false;
        let bold_italic = face("Alpha", "Bold Italic", 700, true);
        let mut f = BTreeSet::new();
        assert!(passes(&light, &f, "", false));
        // One section: either weight.
        f.insert(Filter::Weight(300));
        f.insert(Filter::Weight(700));
        assert!(passes(&light, &f, "", false) && passes(&bold_italic, &f, "", false));
        // Another section narrows it.
        f.insert(Filter::Italic(true));
        assert!(!passes(&light, &f, "", false) && passes(&bold_italic, &f, "", false));
        let mut g = BTreeSet::new();
        g.insert(Filter::NotInstalled);
        assert!(passes(&light, &g, "", false) && !passes(&bold_italic, &g, "", false));
        g.clear();
        g.insert(Filter::Tech(FontKind::OpenType));
        assert!(passes(&light, &g, "", false) && !passes(&bold_italic, &g, "", false));
        g.clear();
        g.insert(Filter::Duplicates);
        assert!(passes(&light, &g, "", true) && !passes(&light, &g, "", false));
        g.clear();
        g.insert(Filter::Feature(1));
        assert!(passes(&light, &g, "", false));
        g.clear();
        g.insert(Filter::Script(Script::Arabic));
        assert!(!passes(&light, &g, "", false));
        // The search matches family, style or PostScript name.
        let none = BTreeSet::new();
        assert!(passes(&bold_italic, &none, "bold it", false));
        assert!(passes(&bold_italic, &none, "alpha-", false));
        assert!(!passes(&bold_italic, &none, "zeta", false));
        assert_eq!(weight_class(350), 400);
        assert_eq!(weight_class(949), 900);
        assert_eq!((width_class(3), width_class(5), width_class(7)), (0, 1, 2));
    }

    #[test]
    fn collections_and_favorites_list_the_faces_they_keep() {
        let mut cat = Catalog::default();
        cat.installed = Some(vec![
            face("Alpha", "Regular", 400, false),
            face("Beta", "Regular", 400, false),
        ]);
        let mut prefs = FontManagerPrefs {
            favorites: vec!["Beta-Regular".into(), "Gone-Regular".into()],
            ..Default::default()
        };
        prefs.collections.push(FontCollection {
            name: "Mine".into(),
            fonts: vec!["Alpha-Regular".into()],
        });
        let fav = cat.faces(&Library::Favorites, &prefs);
        assert_eq!(fav.len(), 1);
        assert_eq!(fav[0].family, "Beta");
        let col = cat.faces(&Library::Collection(0), &prefs);
        assert_eq!(col[0].family, "Alpha");
        assert!(cat.faces(&Library::Collection(5), &prefs).is_empty());
        assert_eq!(cat.faces(&Library::MyFonts, &prefs).len(), 2);
        // Kept in the settings file.
        let json = serde_json::to_string(&prefs).expect("json");
        let back: FontManagerPrefs = serde_json::from_str(&json).expect("back");
        assert_eq!(back, prefs);
        assert_eq!(back.sample_px(), DEFAULT_SAMPLE_PX);
    }

    #[test]
    fn a_folder_lists_its_fonts_installed_or_not() {
        let dir = std::env::temp_dir().join(format!("tracedraw-fm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(
            dir.join("s.ttf"),
            include_bytes!("../../assets/fonts/Selawik-Regular.ttf"),
        )
        .expect("write");
        let mut cat = Catalog::default();
        cat.installed = Some(Vec::new());
        cat.folders.insert(dir.clone(), catalog::scan_folder(&dir));
        let prefs = FontManagerPrefs::default();
        let faces = cat.faces(&Library::Folder(dir.clone()), &prefs);
        assert_eq!(faces.len(), 1);
        assert!(!faces[0].installed);
        // Once the file is installed it shows as installed.
        let mut inst = faces[0].clone();
        inst.installed = true;
        cat.installed = Some(vec![inst]);
        assert!(cat.faces(&Library::Folder(dir.clone()), &prefs)[0].installed);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The fonts of an added folder are usable in drawings, also after a
    /// restart, without installing them one by one; removing the folder
    /// takes them out again, except a file installed on its own (#866).
    #[test]
    fn the_fonts_of_an_added_folder_are_usable() {
        let dir = std::env::temp_dir().join(format!("tracedraw-fm-use-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).expect("dir");
        let file = dir.join("sub").join("s.ttf");
        std::fs::write(
            &file,
            include_bytes!("../../assets/fonts/Selawik-Regular.ttf"),
        )
        .expect("write");
        let fonts = tracedraw_text::FontSystem::empty();
        let mut prefs = FontManagerPrefs {
            folders: vec![dir.clone()],
            ..Default::default()
        };
        assert_eq!(
            install_saved_in(&fonts, &prefs),
            vec!["Selawik".to_string()]
        );
        assert!(fonts.has_family("Selawik"));
        prefs.folders.clear();
        assert_eq!(remove_folder_fonts(&fonts, &prefs, &dir), 1);
        assert!(!fonts.has_family("Selawik"));
        // Installed on its own as well: it stays when the folder goes.
        prefs.installed = vec![file.clone()];
        prefs.folders = vec![dir.clone()];
        install_saved_in(&fonts, &prefs);
        prefs.folders.clear();
        assert_eq!(remove_folder_fonts(&fonts, &prefs, &dir), 0);
        assert!(fonts.has_family("Selawik"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_window_draws_with_faces_and_details() {
        let mut app = App::headless();
        app.settings.font_manager.details = true;
        app.font_catalog.installed = Some(vec![face("Alpha", "Regular", 400, false)]);
        app.font_catalog.generation = Some(tracedraw_text::fonts().generation());
        let mut st = FontManagerState::default();
        st.selected.insert("Alpha-Regular".into());
        let ctx = crate::theme::ui_context();
        let mut close = false;
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1400.0, 900.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                dialog(&mut app, &ui.ctx().clone(), &mut st, &mut close)
            });
            out.textures_delta.clear();
        }
        assert!(!close);
    }

    #[test]
    fn probe_381() {
        let mut app = App::headless();
        app.font_catalog.installed = Some(vec![face("Alpha", "Regular", 400, false)]);
        app.font_catalog.generation = Some(tracedraw_text::fonts().generation());
        let mut st = FontManagerState::default();
        st.selected.insert("Alpha-Regular".into());
        st.focus = Some("Alpha-Regular".into());
        let ctx = crate::theme::ui_context();
        let mut close = false;
        let mut width = |app: &mut App, st: &mut FontManagerState| {
            for _ in 0..4 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1400.0, 900.0),
                    )),
                    ..Default::default()
                };
                let mut out =
                    ctx.run_ui(input, |ui| dialog(app, &ui.ctx().clone(), st, &mut close));
                out.textures_delta.clear();
            }
            ctx.memory(|m| m.area_rect(egui::Id::new("font_manager_window")))
                .map(|r| r.width())
        };
        app.settings.font_manager.details = false;
        let a = width(&mut app, &mut st);
        app.settings.font_manager.details = true;
        let b = width(&mut app, &mut st);
        eprintln!("PROBE without {a:?} with {b:?}");
    }

    /// Turning the details pane on takes its room from the list: the
    /// window keeps its width (#381).
    #[test]
    fn the_details_pane_does_not_widen_the_window() {
        let mut app = App::headless();
        app.font_catalog.installed = Some(vec![face("Alpha", "Regular", 400, false)]);
        app.font_catalog.generation = Some(tracedraw_text::fonts().generation());
        let mut st = FontManagerState::default();
        st.selected.insert("Alpha-Regular".into());
        st.focus = Some("Alpha-Regular".into());
        let ctx = crate::theme::ui_context();
        let mut close = false;
        let mut width = |app: &mut App, st: &mut FontManagerState| {
            for _ in 0..6 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1400.0, 900.0),
                    )),
                    ..Default::default()
                };
                let mut out =
                    ctx.run_ui(input, |ui| dialog(app, &ui.ctx().clone(), st, &mut close));
                out.textures_delta.clear();
            }
            ctx.memory(|m| m.area_rect(egui::Id::new("font_manager_window")))
                .map(|r| r.width())
                .expect("the window")
        };
        app.settings.font_manager.details = false;
        let without = width(&mut app, &mut st);
        app.settings.font_manager.details = true;
        let with = width(&mut app, &mut st);
        assert!((with - without).abs() < 0.5, "{without} then {with}");
        // And it stays so.
        assert!((width(&mut app, &mut st) - without).abs() < 0.5);
    }

    /// The samples are drawn in the theme's text colour: light on the
    /// dark themes, dark on the light ones (#380).
    #[test]
    fn samples_take_the_theme_text_colour() {
        // The theme's text colour, whatever the theme (not switched here:
        // the theme is shared by the tests running alongside).
        assert_eq!(sample_ink(false), Tokens::text());
        // The mask itself is white, so the tint is the colour seen.
        let path = {
            let mut p = kurbo::BezPath::new();
            p.move_to((0.0, 0.0));
            p.line_to((0.5, 0.0));
            p.line_to((0.5, 0.7));
            p.close_path();
            p
        };
        let (img, _) = draw_sample(&path, 0.6, 0.8, 0.2, 40.0).expect("a picture");
        let max = img.pixels.iter().map(|c| c.r()).max().unwrap_or(0);
        assert_eq!(max, 255);
    }
}
