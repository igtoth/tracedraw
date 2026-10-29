//! File > Publish to PDF: the PDF Settings dialog. Its tabs choose the
//! file, what goes in (the drawing, every open drawing, the selection,
//! the current page or a list of pages), the page size, a preset and the
//! compatibility (General), the output intent (Color), the document
//! information and bookmarks (Document), transparency flattening and
//! how text and fonts go in (Objects), the bleed and printers' marks
//! (Prepress), password
//! protection (Security), and list what the preflight found (Issues). OK writes the file. The PDF
//! itself comes from `tracedraw_io::pdf`. See docs/behavior/publish-to-pdf.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::dialogs::{window, ExportState, PDF_STANDARDS};
use crate::ui::tips::RichTip;
use egui::{Color32, Context, RichText, Ui};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracedraw_core::Document;
use tracedraw_io::pdf::{PdfOptions, PdfStandard};
use tracedraw_io::pdf_crypt::{Permissions, Security};

/// Width of the dialog's pages.
const WIDTH: f32 = 560.0;
/// Height of the dialog's pages, so switching pages does not move the
/// buttons.
const HEIGHT: f32 = 380.0;
/// Width of the labels in the first column.
const LABEL_W: f32 = 140.0;
const ERROR: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);

/// What goes into the file (Export range).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pages {
    /// Every page of the active drawing ("Current document").
    #[default]
    All,
    /// Every page of every open drawing, one after the other.
    Documents,
    Current,
    /// The pages listed in the Pages field ("1-3, 5").
    Range,
    Selection,
}

/// How a protected file may be printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Printing {
    No,
    LowResolution,
    #[default]
    Full,
}

impl Printing {
    const ALL: [Printing; 3] = [Printing::No, Printing::LowResolution, Printing::Full];
    fn key(self) -> &'static str {
        match self {
            Printing::No => "publish.printing_no",
            Printing::LowResolution => "publish.printing_low",
            Printing::Full => "publish.printing_full",
        }
    }
}

/// How bitmaps go into the file (Objects tab, #550); presets carry them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BitmapPrefs {
    pub downsample_color: bool,
    pub color_dpi: f64,
    pub downsample_gray: bool,
    pub gray_dpi: f64,
    pub downsample_mono: bool,
    pub mono_dpi: f64,
    /// JPEG (lossy) rather than ZIP for colour and gray pictures.
    pub jpeg: bool,
    pub jpeg_quality: u8,
    /// Compress the pages' text and line art.
    pub compress_content: bool,
}

impl Default for BitmapPrefs {
    fn default() -> Self {
        BitmapPrefs {
            downsample_color: false,
            color_dpi: 300.0,
            downsample_gray: false,
            gray_dpi: 300.0,
            downsample_mono: false,
            mono_dpi: 1200.0,
            jpeg: false,
            jpeg_quality: 85,
            compress_content: true,
        }
    }
}

impl BitmapPrefs {
    /// The writer's bitmap options.
    pub fn options(&self) -> tracedraw_io::pdf::PdfImages {
        let dpi = |on: bool, v: f64| (on && v.is_finite()).then(|| v.clamp(10.0, 9600.0));
        tracedraw_io::pdf::PdfImages {
            color_dpi: dpi(self.downsample_color, self.color_dpi),
            gray_dpi: dpi(self.downsample_gray, self.gray_dpi),
            mono_dpi: dpi(self.downsample_mono, self.mono_dpi),
            jpeg: self.jpeg,
            jpeg_quality: self.jpeg_quality.clamp(1, 100),
            compress_content: self.compress_content,
        }
    }
}

/// The printers' marks around every page (Prepress tab, #789): drawn by
/// the same code as the Print dialog's, in a margin outside the bleed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MarkPrefs {
    pub crop: bool,
    pub registration: bool,
    pub color_bars: bool,
    /// The drawing's name, the page and the date under the page.
    pub file_info: bool,
}

impl MarkPrefs {
    /// The print layout's marks for these choices.
    pub fn marks(self) -> crate::imposition::Marks {
        crate::imposition::Marks {
            crop: self.crop,
            registration: self.registration,
            color_bars: self.color_bars,
            page_info: self.file_info,
            ..Default::default()
        }
    }
}

/// A named set of choices (PDF preset): the compatibility, the output
/// condition, the flattening resolution, the bookmarks, the bleed and the
/// printers' marks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PdfPreset {
    pub name: String,
    pub standard: usize,
    pub output_condition: String,
    pub flatten_dpi: f64,
    pub bookmarks: bool,
    pub show_bookmarks: bool,
    pub bleed_mm: f64,
    pub bitmaps: BitmapPrefs,
    pub marks: MarkPrefs,
}

impl Default for PdfPreset {
    fn default() -> Self {
        PdfPreset {
            name: String::new(),
            standard: 0,
            output_condition: "FOGRA39".into(),
            flatten_dpi: 300.0,
            bookmarks: true,
            show_bookmarks: false,
            bleed_mm: 0.0,
            bitmaps: BitmapPrefs::default(),
            marks: MarkPrefs::default(),
        }
    }
}

/// The presets that come with TraceDraw: (i18n key of the name, preset).
pub fn builtin_presets() -> Vec<(&'static str, PdfPreset)> {
    let base = PdfPreset::default();
    vec![
        (
            "publish.preset_distribution",
            PdfPreset {
                bookmarks: true,
                ..base.clone()
            },
        ),
        (
            "publish.preset_web",
            PdfPreset {
                bookmarks: false,
                flatten_dpi: 150.0,
                // Small files for the screen (#550).
                bitmaps: BitmapPrefs {
                    downsample_color: true,
                    color_dpi: 150.0,
                    downsample_gray: true,
                    gray_dpi: 150.0,
                    downsample_mono: true,
                    mono_dpi: 300.0,
                    jpeg: true,
                    jpeg_quality: 75,
                    compress_content: true,
                },
                ..base.clone()
            },
        ),
        (
            "publish.preset_prepress",
            PdfPreset {
                standard: 3,
                bookmarks: false,
                bleed_mm: 3.0,
                marks: MarkPrefs {
                    crop: true,
                    ..Default::default()
                },
                ..base.clone()
            },
        ),
        (
            "publish.preset_x1a",
            PdfPreset {
                standard: 1,
                bookmarks: false,
                bleed_mm: 3.0,
                ..base.clone()
            },
        ),
        (
            "publish.preset_x3",
            PdfPreset {
                standard: 2,
                bookmarks: false,
                bleed_mm: 3.0,
                ..base
            },
        ),
    ]
}

/// The dialog's choices kept in the preferences (`publish_pdf`); never
/// the passwords.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PublishPrefs {
    /// Index in `PDF_STANDARDS`: 0 plain PDF, then the PDF/X levels.
    pub standard: usize,
    pub output_condition: String,
    /// Resolution transparency is flattened at for PDF/X-1a and X-3.
    pub flatten_dpi: f64,
    pub bookmarks: bool,
    pub show_bookmarks: bool,
    pub printing: Printing,
    pub copy: bool,
    pub modify: bool,
    pub annotate: bool,
    pub fill_forms: bool,
    pub assemble: bool,
    pub accessibility: bool,
    /// The preset chosen last (a built-in preset's key or a saved name).
    pub preset: String,
    /// Presets saved with "+".
    pub user_presets: Vec<PdfPreset>,
    /// Export all text as curves (no fonts in the file).
    pub text_as_curves: bool,
    /// Put the fonts in the file.
    pub embed_fonts: bool,
    /// Put the standard fonts (Helvetica, Times, Courier) in too.
    pub embed_base14: bool,
    /// Embed only the glyphs used...
    pub subset_fonts: bool,
    /// ...when a font uses fewer than this share of its glyphs (percent).
    pub subset_below_pct: f64,
    /// Bitmap resolution and compression (#550).
    pub bitmaps: BitmapPrefs,
    /// Printers' marks around the pages (#789).
    pub marks: MarkPrefs,
}

impl Default for PublishPrefs {
    fn default() -> Self {
        PublishPrefs {
            standard: 0,
            output_condition: "FOGRA39".into(),
            flatten_dpi: 300.0,
            bookmarks: true,
            show_bookmarks: false,
            printing: Printing::Full,
            copy: true,
            modify: true,
            annotate: true,
            fill_forms: true,
            assemble: true,
            accessibility: true,
            preset: "publish.preset_distribution".into(),
            user_presets: Vec::new(),
            text_as_curves: false,
            embed_fonts: true,
            embed_base14: false,
            subset_fonts: true,
            subset_below_pct: 80.0,
            bitmaps: BitmapPrefs::default(),
            marks: MarkPrefs::default(),
        }
    }
}

impl PublishPrefs {
    pub fn standard(&self) -> PdfStandard {
        match self.standard {
            1 => PdfStandard::X1a,
            2 => PdfStandard::X3,
            3 => PdfStandard::X4,
            _ => PdfStandard::None,
        }
    }

    /// How text goes into the file.
    pub fn text(&self) -> tracedraw_io::pdf::PdfText {
        tracedraw_io::pdf::PdfText {
            as_curves: self.text_as_curves,
            embed_fonts: self.embed_fonts,
            embed_base14: self.embed_base14,
            subset: self.subset_fonts,
            subset_below_pct: self.subset_below_pct.clamp(0.0, 100.0),
        }
    }

    /// What a file protected with a permissions password allows.
    pub fn permissions(&self) -> Permissions {
        Permissions {
            print: self.printing != Printing::No,
            print_high_quality: self.printing == Printing::Full,
            copy: self.copy,
            modify: self.modify,
            annotate: self.annotate,
            fill_forms: self.fill_forms,
            assemble: self.assemble,
            accessibility: self.accessibility,
        }
    }
}

/// The dialog's tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    General,
    Color,
    Document,
    Objects,
    Prepress,
    Security,
    Issues,
}

/// The dialog's state.
#[derive(Clone, PartialEq)]
pub struct PublishState {
    pub tab: Tab,
    pub prefs: PublishPrefs,
    /// The file written by OK.
    pub file: PathBuf,
    pub pages: Pages,
    /// The Pages field: "1-3, 5".
    pub range: String,
    /// With Selection: the page takes the selection's size ("Defined by
    /// selected objects") instead of the drawing's.
    pub fit_selection: bool,
    /// The name typed for a new preset, while "+" asks for it.
    pub new_preset: Option<String>,
    pub bleed_mm: f64,
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
    /// A password is needed to open the file.
    pub lock_open: bool,
    pub open_password: String,
    /// A second password guards printing, copying and changes.
    pub lock_permissions: bool,
    pub permissions_password: String,
    pub show_passwords: bool,
}

impl std::fmt::Debug for PublishState {
    // The passwords stay out of any log.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PublishState")
            .field("tab", &self.tab)
            .field("prefs", &self.prefs)
            .field("file", &self.file)
            .field("pages", &self.pages)
            .field("range", &self.range)
            .field("bleed_mm", &self.bleed_mm)
            .field("lock_open", &self.lock_open)
            .field("lock_permissions", &self.lock_permissions)
            .finish_non_exhaustive()
    }
}

impl PublishState {
    /// The dialog for the active drawing: its name, its properties and
    /// bleed, and the choices used last time.
    pub fn new(app: &App) -> PublishState {
        let m = &app.doc().metadata;
        let stem = app.document_name();
        let dir = app
            .file
            .as_ref()
            .and_then(|f| f.parent())
            .map(Path::to_path_buf)
            .or_else(|| app.settings.last_dir.clone())
            .unwrap_or_default();
        let file = if crate::files::WEB {
            PathBuf::from("/download").join(format!("{stem}.pdf"))
        } else {
            dir.join(format!("{stem}.pdf"))
        };
        PublishState {
            tab: Tab::General,
            prefs: app.settings.publish_pdf.clone(),
            file,
            pages: Pages::All,
            range: "1".into(),
            fit_selection: true,
            new_preset: None,
            bleed_mm: m.bleed.max(0.0),
            title: app.document_name(),
            author: m.author.clone(),
            subject: m.subject.clone(),
            keywords: m.keywords.clone(),
            lock_open: false,
            open_password: String::new(),
            lock_permissions: false,
            permissions_password: String::new(),
            show_passwords: false,
        }
    }

    /// PDF/X files cannot be protected: the standards forbid encryption.
    pub fn can_protect(&self) -> bool {
        self.prefs.standard() == PdfStandard::None
    }

    /// Every preset offered: the built-in ones, then the saved ones, as
    /// (key or name, shown name, preset).
    pub fn presets(&self) -> Vec<(String, String, PdfPreset)> {
        let mut out: Vec<(String, String, PdfPreset)> = builtin_presets()
            .into_iter()
            .map(|(k, p)| (k.to_string(), tr(k), p))
            .collect();
        for p in &self.prefs.user_presets {
            out.push((p.name.clone(), p.name.clone(), p.clone()));
        }
        out
    }

    /// Take the choices of the preset `key`.
    pub fn apply_preset(&mut self, key: &str) {
        if let Some((_, _, p)) = self.presets().into_iter().find(|(k, _, _)| k == key) {
            self.prefs.standard = p.standard.min(PDF_STANDARDS.len() - 1);
            self.prefs.output_condition = p.output_condition;
            self.prefs.flatten_dpi = p.flatten_dpi;
            self.prefs.bookmarks = p.bookmarks;
            self.prefs.show_bookmarks = p.show_bookmarks;
            self.bleed_mm = p.bleed_mm.max(0.0);
            self.prefs.bitmaps = p.bitmaps;
            self.prefs.marks = p.marks;
            self.prefs.preset = key.to_string();
        }
    }

    /// Save the current choices as a preset named `name` (replacing one
    /// of that name); built-in names are not taken.
    pub fn save_preset(&mut self, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || builtin_presets().iter().any(|(k, _)| tr(k) == name) {
            return false;
        }
        let p = PdfPreset {
            name: name.to_string(),
            standard: self.prefs.standard,
            output_condition: self.prefs.output_condition.clone(),
            flatten_dpi: self.prefs.flatten_dpi,
            bookmarks: self.prefs.bookmarks,
            show_bookmarks: self.prefs.show_bookmarks,
            bleed_mm: self.bleed_mm,
            bitmaps: self.prefs.bitmaps,
            marks: self.prefs.marks,
        };
        self.prefs.user_presets.retain(|q| q.name != name);
        self.prefs.user_presets.push(p);
        self.prefs.preset = name.to_string();
        true
    }

    /// Delete the chosen preset when it is a saved one.
    pub fn delete_preset(&mut self) -> bool {
        let before = self.prefs.user_presets.len();
        let name = self.prefs.preset.clone();
        self.prefs.user_presets.retain(|q| q.name != name);
        if self.prefs.user_presets.len() == before {
            return false;
        }
        self.prefs.preset = "publish.preset_distribution".into();
        true
    }

    /// Why the file cannot be written yet (an i18n key).
    pub fn problem(&self) -> Option<&'static str> {
        if self.pages == Pages::Range && parse_pages(&self.range, usize::MAX).is_empty() {
            return Some("publish.need_pages");
        }
        if self.file.file_name().is_none() {
            return Some("publish.need_file");
        }
        if !self.can_protect() {
            return None;
        }
        if self.lock_open && self.open_password.is_empty() {
            return Some("publish.need_open_password");
        }
        if self.lock_permissions && self.permissions_password.is_empty() {
            return Some("publish.need_permissions_password");
        }
        if self.lock_open
            && self.lock_permissions
            && self.open_password == self.permissions_password
        {
            return Some("publish.passwords_differ");
        }
        None
    }

    /// The protection asked for, if any. Without a permissions password
    /// whoever opens the file may do everything.
    pub fn security(&self) -> Option<Security> {
        if !self.can_protect() || !(self.lock_open || self.lock_permissions) {
            return None;
        }
        let user = if self.lock_open {
            self.open_password.clone()
        } else {
            String::new()
        };
        let (owner, permissions) = if self.lock_permissions {
            (self.permissions_password.clone(), self.prefs.permissions())
        } else {
            (user.clone(), Permissions::default())
        };
        Some(Security {
            user_password: user,
            owner_password: owner,
            permissions,
        })
    }

    /// The writer's options; `output_profile` is the PDF/X output intent.
    pub fn options(&self, output_profile: Option<Vec<u8>>) -> PdfOptions {
        PdfOptions {
            standard: self.prefs.standard(),
            output_profile,
            output_condition: self.prefs.output_condition.clone(),
            bleed_mm: self.bleed_mm.max(0.0),
            title: Some(self.title.clone()),
            flatten_dpi: self.prefs.flatten_dpi.max(72.0),
            author: Some(self.author.clone()),
            subject: Some(self.subject.clone()),
            keywords: Some(self.keywords.clone()),
            bookmarks: self.prefs.bookmarks,
            show_bookmarks: self.prefs.bookmarks && self.prefs.show_bookmarks,
            security: self.security(),
            text: self.prefs.text(),
            images: self.prefs.bitmaps.options(),
            marks_margin_mm: 0.0,
            page_marks: Vec::new(),
        }
    }

    /// The writer's options for `doc`: [`Self::options`] with the
    /// printers' marks of each page and the room they need, made by the
    /// print layout's code (#789).
    pub fn options_for(
        &self,
        doc: &Document,
        title: &str,
        when: &str,
        output_profile: Option<Vec<u8>>,
    ) -> PdfOptions {
        let mut opts = self.options(output_profile);
        let marks = self.prefs.marks.marks();
        if marks.any() {
            let sizes: Vec<_> = doc.pages.iter().map(|p| p.size).collect();
            let names: Vec<String> = doc
                .pages
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let n = p.name.trim();
                    if n.is_empty() {
                        (i + 1).to_string()
                    } else {
                        n.to_string()
                    }
                })
                .collect();
            let layout = crate::imposition::Options {
                bleed: opts.bleed_mm,
                marks,
                title: title.to_string(),
                when: when.to_string(),
                ..Default::default()
            };
            opts.marks_margin_mm = crate::imposition::MARK_MARGIN;
            opts.page_marks = crate::imposition::page_marks(&sizes, &names, &layout);
        }
        opts
    }
}

/// The pages of `range` ("1-3, 5", 1-based) that exist among `count`,
/// 0-based, in order, each once.
pub fn parse_pages(range: &str, count: usize) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for part in range.split([',', ';']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (a.trim().parse::<usize>(), b.trim().parse::<usize>()),
            None => (part.parse::<usize>(), part.parse::<usize>()),
        };
        let (Ok(a), Ok(b)) = (a, b) else {
            return Vec::new();
        };
        if a == 0 || b == 0 {
            return Vec::new();
        }
        for n in a.min(b)..=a.max(b) {
            if n <= count && !out.contains(&(n - 1)) {
                out.push(n - 1);
            }
        }
    }
    out
}

/// The drawing that goes into the file.
pub fn document_for(app: &App, st: &PublishState) -> Document {
    match st.pages {
        Pages::Selection if !app.selection.is_empty() => {
            if st.fit_selection {
                let export = ExportState {
                    selection_only: true,
                    ..Default::default()
                };
                crate::export::export_document(app, &export).0
            } else {
                // The page as it is, with the selected objects only.
                let mut doc = app.doc().clone();
                doc.pages.retain(|p| p.id == app.page);
                for page in &mut doc.pages {
                    for layer in &mut page.layers {
                        layer.shapes.retain(|s| app.selection.contains(&s.id));
                    }
                }
                doc
            }
        }
        Pages::Current => {
            let mut doc = app.doc().clone();
            doc.pages.retain(|p| p.id == app.page);
            doc
        }
        Pages::Range => {
            let mut doc = app.doc().clone();
            let keep = parse_pages(&st.range, doc.pages.len());
            let pages = std::mem::take(&mut doc.pages);
            doc.pages = keep.into_iter().map(|i| pages[i].clone()).collect();
            doc
        }
        Pages::Documents => {
            let mut doc = app.doc().clone();
            for other in other_documents(app) {
                // The other drawing's symbols go after these; its symbol
                // instances are renumbered to match.
                let offset = doc.symbols.len();
                doc.symbols.extend(other.symbols.iter().cloned());
                for page in &other.pages {
                    let mut page = page.clone();
                    for layer in &mut page.layers {
                        renumber_symbols(&mut layer.shapes, offset);
                    }
                    doc.pages.push(page);
                }
                for f in &other.fonts {
                    if !doc.fonts.iter().any(|g| g.family == f.family) {
                        doc.fonts.push(f.clone());
                    }
                }
            }
            doc
        }
        _ => app.doc().clone(),
    }
}

/// Add `offset` to the symbol numbers of the instances in `shapes`.
fn renumber_symbols(shapes: &mut [tracedraw_core::Shape], offset: usize) {
    use tracedraw_core::document::ShapeKind;
    for s in shapes {
        match &mut s.kind {
            ShapeKind::SymbolInstance { index } => *index += offset,
            ShapeKind::Group { children } => renumber_symbols(children, offset),
            ShapeKind::ClipFrame { frame, contents } => {
                renumber_symbols(std::slice::from_mut(frame.as_mut()), offset);
                renumber_symbols(contents, offset);
            }
            _ => {}
        }
    }
}

/// The drawings open besides the active one, each once.
fn other_documents(app: &App) -> Vec<&Document> {
    let active = app.docs.get(app.active_doc).map(|d| d.drawing);
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for (i, slot) in app.docs.iter().enumerate() {
        if i == app.active_doc || Some(slot.drawing) == active || seen.contains(&slot.drawing) {
            continue;
        }
        // A drawing with several windows keeps its document in the slot
        // that holds it.
        let holder = app
            .docs
            .iter()
            .find(|s| s.drawing == slot.drawing && s.holds)
            .unwrap_or(slot);
        seen.push(slot.drawing);
        out.push(holder.engine.document());
    }
    out
}

/// Write the PDF `st` describes to `path`; the status line's message.
pub fn publish(app: &App, st: &PublishState, path: &Path) -> Result<String, String> {
    let doc = document_for(app, st);
    if doc.pages.is_empty() {
        return Err(tr("publish.need_pages"));
    }
    let tz = crate::time_tracker::local_zone();
    let when = crate::time_tracker::date_time(crate::time_tracker::unix_now(), &tz);
    let opts = st.options_for(
        &doc,
        &app.document_name(),
        &when,
        crate::export::pdfx_profile(app, st.prefs.standard()),
    );
    let (pdf, report) = tracedraw_io::pdf::document_to_pdf_report(&doc, &opts);
    crate::files::write(path, pdf).map_err(|e| e.to_string())?;
    let mut msg = trf("status.exported", &[("path", &path.display().to_string())]);
    // Fonts whose licence forbids embedding: their text went as curves.
    if !report.restricted.is_empty() {
        msg.push(' ');
        msg.push_str(&trf(
            "publish.restricted_fonts",
            &[("f", &report.restricted.join(", "))],
        ));
    }
    Ok(msg)
}

/// The issues tab's title: "No Issues", or how many were found.
fn issues_title(n: usize) -> String {
    if n == 0 {
        tr("publish.tab_no_issues")
    } else {
        trf("publish.tab_issues", &[("n", &n.to_string())])
    }
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut PublishState, close: &mut bool) {
    let issues = crate::ui::preflight_dialog::issues_of(app);
    let mut help = false;
    window(ctx, tr("publish.settings_title")).show(ctx, |ui| {
        ui.horizontal(|ui| {
            for (tab, title) in [
                (Tab::General, tr("publish.tab_general")),
                (Tab::Color, tr("publish.tab_color")),
                (Tab::Document, tr("publish.tab_document")),
                (Tab::Objects, tr("publish.tab_objects")),
                (Tab::Prepress, tr("publish.tab_prepress")),
                (Tab::Security, tr("publish.tab_security")),
                (Tab::Issues, issues_title(issues.len())),
            ] {
                if ui.selectable_label(st.tab == tab, title).clicked() {
                    st.tab = tab;
                }
            }
        });
        ui.separator();
        ui.vertical(|ui| {
            ui.set_width(WIDTH);
            ui.set_min_height(HEIGHT);
            match st.tab {
                Tab::General => general(app, ui, st),
                Tab::Color => color(ui, st),
                Tab::Document => document(ui, st),
                Tab::Objects => objects(ui, st),
                Tab::Prepress => prepress(ui, st),
                Tab::Security => security(ui, st),
                Tab::Issues => issues_tab(ui, &issues),
            }
        });
        // The line is there even when empty, so the dialog keeps its size.
        let problem = st.problem();
        let message = problem.map(tr).unwrap_or_else(|| " ".into());
        ui.label(RichText::new(message).color(ERROR));
        let (mut ok, h) = crate::ui::dialogs::help_ok_cancel(ui, close);
        help = h;
        if ok && problem.is_some() {
            // Not closed: show what needs fixing.
            *close = false;
            ok = false;
            st.tab = match problem {
                Some("publish.need_pages") | Some("publish.need_file") => Tab::General,
                _ => Tab::Security,
            };
        }
        if !ok {
            return;
        }
        app.settings.publish_pdf = st.prefs.clone();
        // On Android the system's picker asks where the PDF goes, with
        // the file name of the form; it answers on a later frame.
        if crate::files::ANDROID {
            let name = st
                .file
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Graphic1.pdf".into());
            let st = st.clone();
            crate::files::Dialog::new()
                .add_filter("PDF (*.pdf)", &["pdf"])
                .set_file_name(name)
                .save_file(app, move |app, path| {
                    app.status = match publish(app, &st, &path) {
                        Ok(msg) => msg,
                        Err(e) => trf("status.export_failed", &[("e", &e)]),
                    };
                });
            return;
        }
        if !crate::files::WEB {
            if let Some(dir) = st.file.parent() {
                app.settings.last_dir = Some(dir.to_path_buf());
            }
        }
        app.status = match publish(app, st, &st.file) {
            Ok(msg) => msg,
            Err(e) => trf("status.export_failed", &[("e", &e)]),
        };
    });
    if help {
        app.open_dialog_help();
    }
}

/// A row of the form: a label of fixed width, then the field.
fn row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        label_cell(ui, label, LABEL_W);
        add(ui);
    });
}

/// A label on the left of a cell `width` wide.
fn label_cell(ui: &mut Ui, label: &str, width: f32) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 20.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(width);
            ui.label(label);
        },
    );
}

fn note(ui: &mut Ui, key: &str) {
    ui.label(RichText::new(tr(key)).color(Tokens::text_dim()).size(11.0));
}

/// General: the file, what goes in, the page size, the preset and the
/// compatibility.
fn general(app: &App, ui: &mut Ui, st: &mut PublishState) {
    let has_selection = !app.selection.is_empty();
    if st.pages == Pages::Selection && !has_selection {
        st.pages = Pages::All;
    }
    let others = other_documents(app).len();
    if st.pages == Pages::Documents && others == 0 {
        st.pages = Pages::All;
    }
    row(ui, &tr("publish.file_name"), |ui| {
        ui.label(st.file.display().to_string());
        // On Android the system's picker asks where the file goes when OK
        // is pressed, so there is nothing to browse first.
        if !crate::files::WEB && !crate::files::ANDROID && ui.button(tr("publish.browse")).clicked()
        {
            let name = st
                .file
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Graphic1.pdf".into());
            if let Some(p) = crate::files::Dialog::new()
                .add_filter("PDF (*.pdf)", &["pdf"])
                .set_file_name(name)
                .save_path()
            {
                st.file = p;
            }
        }
    });
    ui.add_space(4.0);
    group(ui, &tr("publish.export_range"), |ui| {
        egui::Grid::new("publish_range")
            .num_columns(2)
            .show(ui, |ui| {
                ui.radio_value(&mut st.pages, Pages::All, tr("publish.range_document"));
                ui.radio_value(&mut st.pages, Pages::Current, tr("publish.pages_current"));
                ui.end_row();
                ui.add_enabled_ui(others > 0, |ui| {
                    ui.radio_value(
                        &mut st.pages,
                        Pages::Documents,
                        tr("publish.range_documents"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut st.pages, Pages::Range, tr("publish.range_pages"));
                    ui.add_enabled_ui(st.pages == Pages::Range, |ui| {
                        ui.add_sized([120.0, 20.0], egui::TextEdit::singleline(&mut st.range));
                    });
                });
                ui.end_row();
                ui.add_enabled_ui(has_selection, |ui| {
                    ui.radio_value(
                        &mut st.pages,
                        Pages::Selection,
                        tr("publish.pages_selection"),
                    );
                });
                ui.end_row();
            });
    });
    group(ui, &tr("publish.page_size"), |ui| {
        ui.add_enabled_ui(st.pages == Pages::Selection, |ui| {
            ui.radio_value(&mut st.fit_selection, false, tr("publish.size_document"));
            ui.radio_value(&mut st.fit_selection, true, tr("publish.size_selection"));
        });
    });
    row(ui, &tr("publish.preset"), |ui| {
        let presets = st.presets();
        let shown = presets
            .iter()
            .find(|(k, _, _)| *k == st.prefs.preset)
            .map(|(_, n, _)| n.clone())
            .unwrap_or_default();
        let mut pick = None;
        egui::ComboBox::from_id_salt("publish_preset")
            .selected_text(shown)
            .width(220.0)
            .show_ui(ui, |ui| {
                for (k, n, _) in &presets {
                    if ui.selectable_label(*k == st.prefs.preset, n).clicked() {
                        pick = Some(k.clone());
                    }
                }
            });
        if let Some(k) = pick {
            st.apply_preset(&k);
        }
        if ui
            .button(RichText::new("+").size(16.0))
            .rich_tip(tr("publish.preset_add"))
            .clicked()
        {
            st.new_preset = Some(String::new());
        }
        let user = st
            .prefs
            .user_presets
            .iter()
            .any(|p| p.name == st.prefs.preset);
        if ui
            .add_enabled(user, egui::Button::new(RichText::new("-").size(16.0)))
            .rich_tip(tr("publish.preset_delete"))
            .clicked()
        {
            st.delete_preset();
        }
    });
    if let Some(name) = st.new_preset.as_mut() {
        let mut done = None;
        row(ui, &tr("publish.preset_name"), |ui| {
            let r = ui.add_sized([180.0, 20.0], egui::TextEdit::singleline(name));
            if ui
                .button(tr("dialog.ok"))
                .rich_tip(crate::ui::tips::keyed("dialog.ok"))
                .clicked()
                || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                done = Some(true);
            }
            if ui
                .button(tr("dialog.cancel"))
                .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                .clicked()
            {
                done = Some(false);
            }
        });
        match done {
            Some(true) => {
                let n = name.clone();
                if st.save_preset(&n) {
                    st.new_preset = None;
                }
            }
            Some(false) => st.new_preset = None,
            None => {}
        }
    }
    row(ui, &tr("publish.compatibility"), |ui| {
        egui::ComboBox::from_id_salt("publish_standard")
            .selected_text(PDF_STANDARDS[st.prefs.standard.min(PDF_STANDARDS.len() - 1)])
            .width(220.0)
            .show_ui(ui, |ui| {
                for (i, n) in PDF_STANDARDS.iter().enumerate() {
                    ui.selectable_value(&mut st.prefs.standard, i, *n);
                }
            });
    });
}

/// A titled box around a group of fields.
fn group(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, Color32::from_gray(0xC8)))
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(title);
            add(ui);
        });
    ui.add_space(4.0);
}

/// Color: the output intent of the PDF/X standards.
fn color(ui: &mut Ui, st: &mut PublishState) {
    let standard = st.prefs.standard();
    if standard == PdfStandard::None {
        note(ui, "publish.color_plain");
        return;
    }
    row(ui, &tr("dialog.output_condition"), |ui| {
        ui.add_sized(
            [220.0, 20.0],
            egui::TextEdit::singleline(&mut st.prefs.output_condition),
        );
    });
    note(ui, "dialog.pdfx_profile_note");
}

/// Objects: how transparency is flattened for PDF/X-1a and X-3.
fn objects(ui: &mut Ui, st: &mut PublishState) {
    let flattens = matches!(st.prefs.standard(), PdfStandard::X1a | PdfStandard::X3);
    ui.add_enabled_ui(flattens, |ui| {
        row(ui, &tr("publish.flatten_resolution"), |ui| {
            ui.add(
                crate::ui::field::NumField::new(&mut st.prefs.flatten_dpi)
                    .range(72.0..=1200.0)
                    .speed(5.0)
                    .suffix(" dpi"),
            );
        });
    });
    note(ui, "publish.objects_note");
    ui.add_space(8.0);
    bitmaps(ui, &mut st.prefs.bitmaps);
    text_and_fonts(ui, &mut st.prefs);
}

/// Objects: the pictures' resolution and compression (#550).
fn bitmaps(ui: &mut Ui, b: &mut BitmapPrefs) {
    group(ui, &tr("publish.bitmaps"), |ui| {
        for (on, dpi, key) in [
            (
                &mut b.downsample_color,
                &mut b.color_dpi,
                "publish.downsample_color",
            ),
            (
                &mut b.downsample_gray,
                &mut b.gray_dpi,
                "publish.downsample_gray",
            ),
            (
                &mut b.downsample_mono,
                &mut b.mono_dpi,
                "publish.downsample_mono",
            ),
        ] {
            ui.horizontal(|ui| {
                ui.checkbox(on, tr(key));
                ui.add_enabled_ui(*on, |ui| {
                    ui.add(
                        crate::ui::field::NumField::new(dpi)
                            .range(10.0..=9600.0)
                            .speed(5.0)
                            .max_decimals(0)
                            .suffix(" dpi"),
                    );
                });
            });
        }
        ui.horizontal(|ui| {
            ui.label(tr("publish.compression"));
            egui::ComboBox::from_id_salt("publish_compression")
                .selected_text(if b.jpeg {
                    tr("publish.compression_jpeg")
                } else {
                    tr("publish.compression_zip")
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut b.jpeg, false, tr("publish.compression_zip"));
                    ui.selectable_value(&mut b.jpeg, true, tr("publish.compression_jpeg"));
                });
            ui.add_enabled_ui(b.jpeg, |ui| {
                ui.label(tr("publish.jpeg_quality"));
                ui.add(crate::ui::Slider::new(&mut b.jpeg_quality, 1..=100));
            });
        });
        ui.checkbox(&mut b.compress_content, tr("publish.compress_content"));
        note(ui, "publish.bitmaps_note");
    });
}

/// Objects: how text and fonts go into the file.
fn text_and_fonts(ui: &mut Ui, p: &mut PublishPrefs) {
    group(ui, &tr("publish.text_fonts"), |ui| {
        ui.checkbox(&mut p.text_as_curves, tr("publish.text_as_curves"));
        ui.add_enabled_ui(!p.text_as_curves, |ui| {
            ui.checkbox(&mut p.embed_fonts, tr("publish.embed_fonts"));
            ui.add_enabled_ui(p.embed_fonts, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(22.0);
                    ui.checkbox(&mut p.embed_base14, tr("publish.embed_base14"));
                });
                ui.horizontal(|ui| {
                    ui.add_space(22.0);
                    ui.checkbox(&mut p.subset_fonts, tr("publish.subset_fonts"));
                    ui.add_enabled_ui(p.subset_fonts, |ui| {
                        ui.label(tr("publish.subset_below"));
                        ui.add(
                            crate::ui::field::NumField::new(&mut p.subset_below_pct)
                                .range(0.0..=100.0)
                                .speed(1.0)
                                .suffix(" %"),
                        );
                    });
                });
            });
        });
        note(ui, "publish.fonts_note");
    });
}

/// Prepress: the bleed around every page and the printers' marks.
fn prepress(ui: &mut Ui, st: &mut PublishState) {
    row(ui, &tr("dialog.bleed"), |ui| {
        ui.add(
            crate::ui::field::NumField::new(&mut st.bleed_mm)
                .range(0.0..=50.0)
                .speed(0.5)
                .suffix(" mm"),
        );
    });
    note(ui, "publish.bleed_note");
    ui.add_space(8.0);
    group(ui, &tr("publish.marks"), |ui| {
        let m = &mut st.prefs.marks;
        egui::Grid::new("publish_marks")
            .num_columns(2)
            .spacing([24.0, 4.0])
            .show(ui, |ui| {
                ui.checkbox(&mut m.crop, tr("dialog.crop_marks"))
                    .rich_tip(tr("dialog.crop_marks_tip"));
                ui.checkbox(&mut m.registration, tr("dialog.registration_marks"))
                    .rich_tip(tr("dialog.registration_marks_tip"));
                ui.end_row();
                ui.checkbox(&mut m.color_bars, tr("print.color_bars"))
                    .rich_tip(tr("print.color_bars_tip"));
                ui.checkbox(&mut m.file_info, tr("publish.file_info"))
                    .rich_tip(tr("publish.file_info_tip"));
                ui.end_row();
            });
        note(ui, "publish.marks_note");
    });
}

/// Issues: what the preflight found in the drawing.
fn issues_tab(ui: &mut Ui, issues: &[crate::preflight::Issue]) {
    if issues.is_empty() {
        ui.label(tr("preflight.none"));
        return;
    }
    ui.label(crate::ui::preflight_dialog::summary(issues));
    egui::ScrollArea::vertical()
        .max_height(HEIGHT - 30.0)
        .show(ui, |ui| {
            for i in issues {
                ui.horizontal(|ui| {
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                    crate::ui::preflight_dialog::paint_icon(
                        ui.painter(),
                        r.center(),
                        i.check.severity(),
                    );
                    let page = i
                        .page
                        .map(|p| trf("publish.issue_page", &[("n", &(p + 1).to_string())]))
                        .unwrap_or_default();
                    let mut text = tr(&i.check.key());
                    if !i.detail.is_empty() {
                        text = format!("{text}: {}", i.detail);
                    }
                    ui.label(format!("{text} {page}"));
                });
            }
        });
}

/// Document: the document information and the bookmarks.
fn document(ui: &mut Ui, st: &mut PublishState) {
    for (key, value) in [
        ("publish.document_title", &mut st.title),
        ("publish.author", &mut st.author),
        ("publish.subject", &mut st.subject),
        ("publish.keywords", &mut st.keywords),
    ] {
        row(ui, &tr(key), |ui| {
            ui.add_sized(
                [WIDTH - LABEL_W - 12.0, 20.0],
                egui::TextEdit::singleline(value),
            );
        });
    }
    note(ui, "publish.info_from_properties");
    ui.add_space(8.0);
    ui.checkbox(&mut st.prefs.bookmarks, tr("publish.bookmarks"));
    ui.add_enabled_ui(st.prefs.bookmarks, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(22.0);
            ui.checkbox(&mut st.prefs.show_bookmarks, tr("publish.show_bookmarks"));
        });
    });
}

/// Security: the two passwords and what the second one guards.
fn security(ui: &mut Ui, st: &mut PublishState) {
    let can = st.can_protect();
    if !can {
        ui.label(RichText::new(tr("publish.pdfx_no_security")).color(ERROR));
        ui.add_space(4.0);
    }
    ui.add_enabled_ui(can, |ui| {
        let masked = !st.show_passwords;
        ui.checkbox(&mut st.lock_open, tr("publish.lock_open"));
        ui.add_enabled_ui(st.lock_open, |ui| {
            password_row(ui, &mut st.open_password, masked);
        });
        ui.checkbox(&mut st.lock_permissions, tr("publish.lock_permissions"));
        ui.add_enabled_ui(st.lock_permissions, |ui| {
            password_row(ui, &mut st.permissions_password, masked);
        });
        ui.checkbox(&mut st.show_passwords, tr("publish.show_passwords"));
        ui.add_space(4.0);
        ui.add_enabled_ui(st.lock_permissions, |ui| {
            ui.strong(tr("publish.permissions"));
            let p = &mut st.prefs;
            ui.horizontal(|ui| {
                ui.add_space(22.0);
                ui.label(tr("publish.printing"));
                egui::ComboBox::from_id_salt("publish_printing")
                    .selected_text(tr(p.printing.key()))
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for v in Printing::ALL {
                            ui.selectable_value(&mut p.printing, v, tr(v.key()));
                        }
                    });
            });
            egui::Grid::new("publish_permissions")
                .num_columns(2)
                .spacing([12.0, 2.0])
                .show(ui, |ui| {
                    let boxes = [
                        (&mut p.copy, "publish.allow_copy"),
                        (&mut p.modify, "publish.allow_modify"),
                        (&mut p.annotate, "publish.allow_annotate"),
                        (&mut p.fill_forms, "publish.allow_fill_forms"),
                        (&mut p.assemble, "publish.allow_assemble"),
                        (&mut p.accessibility, "publish.allow_accessibility"),
                    ];
                    for (n, (value, key)) in boxes.into_iter().enumerate() {
                        if n % 2 == 0 {
                            ui.horizontal(|ui| {
                                ui.add_space(22.0);
                                ui.checkbox(value, tr(key));
                            });
                        } else {
                            ui.checkbox(value, tr(key));
                            ui.end_row();
                        }
                    }
                });
        });
        ui.add_space(4.0);
        note(ui, "publish.encryption_note");
    });
}

fn password_row(ui: &mut Ui, value: &mut String, masked: bool) {
    ui.horizontal(|ui| {
        ui.add_space(22.0);
        label_cell(ui, &tr("publish.password"), LABEL_W - 22.0);
        ui.add_sized(
            [200.0, 20.0],
            egui::TextEdit::singleline(value).password(masked),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{vec2, Pos2, Rect};
    use tracedraw_core::{geometry::Size, Document};

    fn app_with_two_pages() -> App {
        let mut app = App::headless();
        let mut doc = Document::new("Leaflet", Size::new(100.0, 50.0));
        doc.metadata.author = "Ana".into();
        doc.metadata.keywords = "sale".into();
        doc.metadata.bleed = 3.0;
        let mut ids = doc.ids().clone();
        let mut second = doc.pages[0].clone();
        second.id = ids.page();
        second.name = "Back".into();
        doc.set_ids(ids);
        doc.pages.push(second);
        app.open_document(doc, None);
        app
    }

    #[test]
    fn the_dialog_starts_from_the_drawing_and_checks_the_passwords() {
        let app = app_with_two_pages();
        let mut st = PublishState::new(&app);
        assert_eq!(st.author, "Ana");
        assert_eq!(st.keywords, "sale");
        assert_eq!(st.bleed_mm, 3.0);
        assert!(st.problem().is_none() && st.security().is_none());
        // A password to open the file, nothing else: everything allowed.
        st.lock_open = true;
        assert_eq!(st.problem(), Some("publish.need_open_password"));
        st.open_password = "abc".into();
        assert!(st.problem().is_none());
        let sec = st.security().expect("protected");
        assert_eq!(sec.user_password, "abc");
        assert_eq!(sec.owner_password, "abc");
        assert_eq!(sec.permissions, Permissions::default());
        // A permissions password: the choices made apply.
        st.lock_permissions = true;
        assert_eq!(st.problem(), Some("publish.need_permissions_password"));
        st.permissions_password = "abc".into();
        assert_eq!(st.problem(), Some("publish.passwords_differ"));
        st.permissions_password = "owner".into();
        st.prefs.printing = Printing::LowResolution;
        st.prefs.copy = false;
        let sec = st.security().expect("protected");
        assert_eq!(sec.owner_password, "owner");
        assert!(sec.permissions.print && !sec.permissions.print_high_quality);
        assert!(!sec.permissions.copy && sec.permissions.modify);
        // Only a permissions password: the file opens without one.
        st.lock_open = false;
        let sec = st.security().expect("protected");
        assert!(sec.user_password.is_empty());
        // PDF/X: never protected, and no complaint about the passwords.
        st.lock_open = true;
        st.open_password.clear();
        st.prefs.standard = 3;
        assert!(st.problem().is_none() && st.security().is_none());
        assert!(st.options(None).security.is_none());
        // The passwords never reach a log.
        st.permissions_password = "hunter2".into();
        assert!(!format!("{st:?}").contains("hunter2"));
    }

    #[test]
    fn publishing_writes_the_pages_with_bookmarks_and_protection() {
        let mut app = app_with_two_pages();
        let mut st = PublishState::new(&app);
        st.prefs.show_bookmarks = true;
        st.subject = "Spring".into();
        let dir = std::env::temp_dir().join(format!("tracedraw-publish-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("leaflet.pdf");
        let msg = publish(&app, &st, &path).expect("published");
        assert!(msg.contains("leaflet.pdf"));
        // Every page, a bookmark each, the panel open, the information.
        let bytes = std::fs::read(&path).expect("written");
        let doc = lopdf::Document::load_mem(&bytes).expect("parse");
        assert_eq!(doc.get_pages().len(), 2);
        let titles: Vec<String> = doc
            .get_toc()
            .map(|toc| toc.toc.into_iter().map(|t| t.title).collect())
            .unwrap_or_default();
        assert_eq!(titles, vec!["Page 1".to_string(), "Back".to_string()]);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        assert!(text.contains("/PageMode /UseOutlines"));
        assert!(text.contains("/Subject (Spring)") && text.contains("/Author (Ana)"));
        // Permissions only: protected, yet it opens without a password.
        st.lock_permissions = true;
        st.permissions_password = "owner".into();
        st.prefs.copy = false;
        publish(&app, &st, &path).expect("published");
        let bytes = std::fs::read(&path).expect("written");
        let text = String::from_utf8_lossy(&bytes).into_owned();
        assert!(text.starts_with("%PDF-1.7") && text.contains("/Encrypt "));
        assert!(!text.contains("Spring"));
        let mut ids = tracedraw_core::id::IdSource::default();
        let imp = tracedraw_io::pdf_import::parse(&bytes, &mut ids).expect("opens");
        assert_eq!(imp.pages.len(), 2);
        assert_eq!(imp.title.as_deref(), Some("Leaflet"));
        // A password to open it, and the current page alone.
        st.lock_open = true;
        st.open_password = "abc".into();
        st.pages = Pages::Current;
        publish(&app, &st, &path).expect("published");
        let bytes = std::fs::read(&path).expect("written");
        assert!(tracedraw_io::pdf_import::parse(&bytes, &mut ids).is_err());
        let imp = tracedraw_io::pdf_import::parse_with_password(&bytes, &mut ids, "abc")
            .expect("opens with its password");
        assert_eq!(imp.pages.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
        // The dialog draws on each page.
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 1000.0))),
            ..Default::default()
        };
        st.prefs.standard = 1;
        for tab in [
            Tab::General,
            Tab::Color,
            Tab::Document,
            Tab::Objects,
            Tab::Prepress,
            Tab::Security,
            Tab::Issues,
        ] {
            st.tab = tab;
            for _ in 0..2 {
                let mut out = ctx.run_ui(input.clone(), |ui| {
                    let mut close = false;
                    dialog(&mut app, ui.ctx(), &mut st, &mut close);
                    assert!(!close);
                });
                out.textures_delta.clear();
            }
        }
    }

    #[test]
    fn publishing_keeps_text_as_text_unless_curves_are_asked_for() {
        tracedraw_text::install();
        let fonts = tracedraw_text::fonts();
        let Some(family) = ["DejaVu Sans", "Liberation Sans", "Noto Sans"]
            .into_iter()
            .find(|f| fonts.has_family(f))
        else {
            return;
        };
        // Real text with fonts is the default.
        let defaults = PublishPrefs::default();
        assert!(!defaults.text_as_curves && defaults.embed_fonts && defaults.subset_fonts);
        let mut app = app_with_two_pages();
        app.new_shape(tracedraw_core::document::ShapeKind::Text {
            spans: vec![tracedraw_core::TextSpan::new(
                "Searchable words",
                family,
                18.0,
            )],
            origin: tracedraw_core::geometry::Point::new(10.0, 20.0),
            frame: None,
            align: tracedraw_core::TextAlign::Left,
            para: Default::default(),
            on_path: None,
        })
        .expect("a text");
        let mut st = PublishState::new(&app);
        st.pages = Pages::Current;
        let dir = std::env::temp_dir().join(format!("tracedraw-pdftext-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("words.pdf");
        publish(&app, &st, &path).expect("published");
        let bytes = std::fs::read(&path).expect("written");
        let text = String::from_utf8_lossy(&bytes).into_owned();
        assert!(text.contains("/Type0") && text.contains("/FontDescriptor"));
        let pdf = lopdf::Document::load_mem(&bytes).expect("parse");
        let words = pdf.extract_text(&[1]).expect("text");
        assert!(words.contains("Searchable words"), "{words:?}");
        // Curves when asked: no font, no text to find.
        st.prefs.text_as_curves = true;
        publish(&app, &st, &path).expect("published");
        let bytes = std::fs::read(&path).expect("written");
        assert!(!String::from_utf8_lossy(&bytes).contains("/Font"));
        let pdf = lopdf::Document::load_mem(&bytes).expect("parse");
        let words = pdf.extract_text(&[1]).unwrap_or_default();
        assert!(!words.contains("Searchable"), "{words:?}");
        // A font with CFF outlines, when one is installed: a CID font of
        // CFF data (FontFile3), and the text still comes back.
        if let Some(cff) = ["Nimbus Sans", "Loma", "Source Sans 3"]
            .into_iter()
            .find(|f| fonts.has_family(f))
        {
            let mut app = app_with_two_pages();
            app.new_shape(tracedraw_core::document::ShapeKind::Text {
                spans: vec![tracedraw_core::TextSpan::new("Searchable words", cff, 18.0)],
                origin: tracedraw_core::geometry::Point::new(10.0, 20.0),
                frame: None,
                align: tracedraw_core::TextAlign::Left,
                para: Default::default(),
                on_path: None,
            })
            .expect("a text");
            let mut st = PublishState::new(&app);
            st.pages = Pages::Current;
            publish(&app, &st, &path).expect("published");
            let bytes = std::fs::read(&path).expect("written");
            let text = String::from_utf8_lossy(&bytes).into_owned();
            assert!(text.contains("/CIDFontType0C") && text.contains("/FontFile3"));
            let pdf = lopdf::Document::load_mem(&bytes).expect("parse");
            let words = pdf.extract_text(&[1]).expect("text");
            assert!(words.contains("Searchable words"), "{words:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Texts one frame of the dialog drew, with where.
    fn dialog_texts(
        app: &mut App,
        ctx: &egui::Context,
        st: &mut PublishState,
        events: Vec<egui::Event>,
    ) -> Vec<(String, Rect)> {
        fn walk(s: &egui::Shape, out: &mut Vec<(String, Rect)>) {
            match s {
                egui::Shape::Text(t) => {
                    out.push((t.galley.text().to_string(), t.visual_bounding_rect()))
                }
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            let mut close = false;
            dialog(app, ui.ctx(), st, &mut close);
        });
        out.textures_delta.clear();
        let mut texts = Vec::new();
        out.shapes.iter().for_each(|c| walk(&c.shape, &mut texts));
        texts
    }

    /// Prepress: the printers' marks are ticked in the dialog, and the
    /// file gets them around every page in a margin outside the bleed,
    /// with the trim, bleed and media boxes set to match (#789).
    #[test]
    fn prepress_marks_go_around_the_pages_and_grow_the_media_box() {
        tracedraw_text::install();
        let mut app = app_with_two_pages();
        let mut st = PublishState::new(&app);
        st.tab = Tab::Prepress;
        let ctx = crate::theme::ui_context();
        let mut texts = Vec::new();
        for _ in 0..3 {
            texts = dialog_texts(&mut app, &ctx, &mut st, Vec::new());
        }
        for key in [
            "publish.marks",
            "dialog.crop_marks",
            "dialog.registration_marks",
            "print.color_bars",
            "publish.file_info",
        ] {
            assert!(texts.iter().any(|(t, _)| *t == tr(key)), "{key} shown");
        }
        // Click Crop marks and File information, as a user would.
        for key in ["dialog.crop_marks", "publish.file_info"] {
            let at = texts
                .iter()
                .find(|(t, _)| *t == tr(key))
                .map(|(_, r)| r.center())
                .expect("shown");
            let press = |pressed| egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            dialog_texts(&mut app, &ctx, &mut st, vec![egui::Event::PointerMoved(at)]);
            dialog_texts(&mut app, &ctx, &mut st, vec![press(true)]);
            texts = dialog_texts(&mut app, &ctx, &mut st, vec![press(false)]);
        }
        assert!(st.prefs.marks.crop && st.prefs.marks.file_info);
        assert!(!st.prefs.marks.registration && !st.prefs.marks.color_bars);
        // The file: 100 x 50 mm pages, 3 mm of bleed, 12 mm for the marks.
        let dir = std::env::temp_dir().join(format!("tracedraw-pdfmarks-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("marks.pdf");
        st.prefs.bitmaps.compress_content = false;
        publish(&app, &st, &path).expect("published");
        let bytes = std::fs::read(&path).expect("written");
        let pdf = lopdf::Document::load_mem(&bytes).expect("parse");
        let pt = |mm: f64| (mm * 72.0 / 25.4) as f32;
        let near = |a: f32, b: f32| (a - b).abs() < 0.01;
        let pages = pdf.get_pages();
        assert_eq!(pages.len(), 2);
        for id in pages.values() {
            let page = pdf.get_dictionary(*id).expect("page");
            let rect = |name: &[u8]| -> Vec<f32> {
                page.get(name)
                    .and_then(|o| o.as_array())
                    .expect("box")
                    .iter()
                    .map(|v| v.as_float().expect("number"))
                    .collect()
            };
            let media = rect(b"MediaBox");
            assert!(
                near(media[2], pt(130.0)) && near(media[3], pt(80.0)),
                "{media:?}"
            );
            let trim = rect(b"TrimBox");
            assert!(
                near(trim[0], pt(15.0)) && near(trim[2], pt(115.0)),
                "{trim:?}"
            );
            let bleed = rect(b"BleedBox");
            assert!(
                near(bleed[0], pt(12.0)) && near(bleed[3], pt(68.0)),
                "{bleed:?}"
            );
        }
        // The page information names the drawing and each page (in
        // DejaVu Sans, when it is installed).
        if tracedraw_text::fonts().has_family("DejaVu Sans") {
            let words = pdf.extract_text(&[2]).unwrap_or_default();
            assert!(
                words.contains("Leaflet") && words.contains("Back"),
                "{words:?}"
            );
        }
        // No marks: the boxes are the page and its bleed, as before.
        st.prefs.marks = MarkPrefs::default();
        publish(&app, &st, &path).expect("published");
        let bytes = std::fs::read(&path).expect("written");
        let text = String::from_utf8_lossy(&bytes).into_owned();
        assert!(!text.contains("/CSAll"), "no registration marks");
        let pdf = lopdf::Document::load_mem(&bytes).expect("parse");
        let first = pdf.get_pages()[&1];
        let media = pdf
            .get_dictionary(first)
            .and_then(|d| d.get(b"MediaBox"))
            .and_then(|o| o.as_array())
            .expect("box")
            .iter()
            .map(|v| v.as_float().expect("number"))
            .collect::<Vec<_>>();
        assert!(near(media[2], pt(106.0)), "{media:?}");
        let _ = std::fs::remove_dir_all(&dir);
        // The prepress preset asks for crop marks; a saved preset keeps
        // the marks chosen.
        st.apply_preset("publish.preset_prepress");
        assert!(st.prefs.marks.crop);
        st.prefs.marks.color_bars = true;
        assert!(st.save_preset("Marks"));
        st.apply_preset("publish.preset_distribution");
        assert_eq!(st.prefs.marks, MarkPrefs::default());
        st.apply_preset("Marks");
        assert!(st.prefs.marks.color_bars);
    }

    #[test]
    fn page_lists_presets_and_what_goes_in() {
        assert_eq!(parse_pages("1-3, 5", 10), vec![0, 1, 2, 4]);
        assert_eq!(parse_pages("3-1;2", 10), vec![0, 1, 2]);
        assert_eq!(parse_pages("2, 9", 3), vec![1]);
        assert!(parse_pages("a-b", 3).is_empty() && parse_pages("0", 3).is_empty());
        let mut app = app_with_two_pages();
        let mut st = PublishState::new(&app);
        assert_eq!(
            st.file.file_name().and_then(|n| n.to_str()),
            Some("Leaflet.pdf")
        );
        // Pages 2 alone, then a list that names no page: refused.
        st.pages = Pages::Range;
        st.range = "2".into();
        let doc = document_for(&app, &st);
        assert_eq!(doc.pages.len(), 1);
        assert_eq!(doc.pages[0].name, "Back");
        st.range = "x".into();
        assert_eq!(st.problem(), Some("publish.need_pages"));
        // The prepress preset sets PDF/X-4 and a bleed; a saved preset
        // comes back, a built-in one cannot be deleted.
        st.apply_preset("publish.preset_prepress");
        assert_eq!(st.prefs.standard, 3);
        assert_eq!(st.bleed_mm, 3.0);
        assert!(!st.delete_preset());
        st.prefs.standard = 1;
        assert!(st.save_preset("Press A"));
        st.apply_preset("publish.preset_distribution");
        assert_eq!(st.prefs.standard, 0);
        st.apply_preset("Press A");
        assert_eq!(st.prefs.standard, 1);
        assert!(!st.save_preset(" "));
        assert!(st.delete_preset());
        assert!(st.prefs.user_presets.is_empty());
        // The selection, at its own size or on the page.
        let id = app
            .new_shape(tracedraw_core::document::ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 30.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("a rectangle");
        app.select(vec![id]);
        st.pages = Pages::Selection;
        st.fit_selection = true;
        let doc = document_for(&app, &st);
        assert!(doc.pages[0].size.width < 50.0);
        st.fit_selection = false;
        let doc = document_for(&app, &st);
        assert_eq!(doc.pages[0].size.width, 100.0);
        let shapes: usize = doc.pages[0].layers.iter().map(|l| l.shapes.len()).sum();
        assert_eq!(shapes, 1);
        // Every open drawing: the pages of both.
        app.new_document();
        st.pages = Pages::Documents;
        let doc = document_for(&app, &st);
        let others: usize = other_documents(&app).iter().map(|d| d.pages.len()).sum();
        assert!(others >= 2, "the leaflet is among the others");
        assert_eq!(doc.pages.len(), app.doc().pages.len() + others);
    }
}
