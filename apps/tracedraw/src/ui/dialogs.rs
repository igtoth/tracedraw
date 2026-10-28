//! Dialogs: pages, layers, Options (multi-page), Document Properties,
//! Export, Print, Print Merge, Find and Replace, tables, QR codes,
//! bitmaps (convert, straighten, resample, inflate, trace),
//! text (tabs, columns, bullets, drop cap, statistics, spell check),
//! colour management, font manager, palette editor, confirmations.

use crate::app::{App, Units};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Context, Ui};
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Rect, Size},
    Color, Command,
};

pub use crate::ui::options::OptionsPage;

/// The lowest export resolution: an A4 page comes out 9 x 12 pixels.
pub const MIN_EXPORT_DPI: f64 = 1.0;

/// The pixel size a raster export of `w_mm` by `h_mm` has at `dpi` (the
/// renderer rounds up).
pub fn export_pixels(w_mm: f64, h_mm: f64, dpi: f64) -> (u32, u32) {
    let px = |mm: f64| (mm / 25.4 * dpi).ceil().clamp(1.0, f64::from(u32::MAX)) as u32;
    (px(w_mm), px(h_mm))
}

/// The resolution that makes `mm` come out `px` pixels wide, within the
/// export range; `None` for an empty side.
pub fn dpi_for_pixels(mm: f64, px: u32) -> Option<f64> {
    if !(mm > 0.0 && mm.is_finite()) {
        return None;
    }
    // A hair under, so rounding up gives exactly `px`.
    let dpi = f64::from(px) * 25.4 / mm * (1.0 - 1e-9);
    Some(dpi.clamp(MIN_EXPORT_DPI, 2400.0))
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportState {
    /// File > Export opened it: exporting keeps the choices for the next
    /// export (`App::keep_export_choices`).
    pub remember: bool,
    pub format: usize,
    pub dpi: f64,
    pub all_pages: bool,
    pub selection_only: bool,
    pub transparent: bool,
    pub quality: u8,
    pub web: bool,
    /// PDF only: 0 plain, 1 X-1a, 2 X-3, 3 X-4.
    pub pdf_standard: usize,
    pub bleed_mm: f64,
    pub output_condition: String,
    /// An installed script filter chosen instead of `format`, by name.
    pub script: Option<String>,
    /// The file's title and tags (keywords), for formats that keep them.
    pub title: String,
    pub tags: String,
    /// WebP without loss (else lossy at the quality, #592).
    pub webp_lossless: bool,
    /// The export preset chosen (its name), empty for none (#547).
    pub preset: String,
    /// A name being typed for a new preset (false) or for renaming the
    /// chosen one (true).
    pub preset_name: Option<(String, bool)>,
    /// The unit the bitmap size is typed in: an index into
    /// [`EXPORT_SIZE_UNITS`] (#617).
    pub size_unit: usize,
    /// Bitmaps: the artwork's printed size against the page's, the same
    /// across and down (1 = as drawn, #617).
    pub scale: f64,
}

impl Default for ExportState {
    fn default() -> Self {
        ExportState {
            remember: false,
            format: 0,
            dpi: 300.0,
            all_pages: false,
            selection_only: false,
            transparent: true,
            quality: 90,
            web: false,
            pdf_standard: 0,
            bleed_mm: 0.0,
            output_condition: "FOGRA39".into(),
            script: None,
            title: String::new(),
            tags: String::new(),
            webp_lossless: false,
            preset: String::new(),
            preset_name: None,
            size_unit: 0,
            scale: 1.0,
        }
    }
}

/// The units a bitmap export's size is shown and typed in, with their
/// length in mm (pixels have none: they follow the resolution).
pub const EXPORT_SIZE_UNITS: [(&str, f64); 4] =
    [("px", 0.0), ("mm", 1.0), ("cm", 10.0), ("in", 25.4)];

/// The smallest and largest bitmap export scale.
pub const EXPORT_SCALE_RANGE: std::ops::RangeInclusive<f64> = 0.01..=100.0;

/// What the bitmap size fields show for an artwork `w_mm` by `h_mm` with
/// `st`'s scale, resolution and unit.
pub fn export_size_in_unit(w_mm: f64, h_mm: f64, st: &ExportState) -> (f64, f64) {
    let unit = EXPORT_SIZE_UNITS[st.size_unit.min(EXPORT_SIZE_UNITS.len() - 1)].1;
    if unit > 0.0 {
        let s = st.scale_or_one();
        (w_mm * s / unit, h_mm * s / unit)
    } else {
        let (w, h) = export_pixels(w_mm, h_mm, st.render_dpi());
        (f64::from(w), f64::from(h))
    }
}

/// A size typed in a bitmap size field: `typed` (in `st`'s unit) for the
/// side that is `side_mm` long on the page. In pixels it sets the
/// resolution, as before; in a length it sets the scale, the resolution
/// staying. Either way the other side follows (#617).
pub fn set_export_size(st: &mut ExportState, side_mm: f64, typed: f64) {
    if !(side_mm > 0.0 && side_mm.is_finite() && typed > 0.0 && typed.is_finite()) {
        return;
    }
    let unit = EXPORT_SIZE_UNITS[st.size_unit.min(EXPORT_SIZE_UNITS.len() - 1)].1;
    if unit > 0.0 {
        // Not past what the highest resolution can draw.
        let most = (2400.0 / st.dpi.max(MIN_EXPORT_DPI)).min(*EXPORT_SCALE_RANGE.end());
        let least = EXPORT_SCALE_RANGE.start().min(most);
        st.scale = (typed * unit / side_mm).clamp(least, most);
    } else {
        let px = typed.round().clamp(1.0, 200_000.0) as u32;
        let scale = st.scale_or_one();
        if let Some(dpi) = dpi_for_pixels(side_mm * scale, px) {
            st.dpi = dpi;
        }
    }
}

/// Where a `.cdr` file that does not open right is sent in.
pub const CDR_ISSUE: &str = "https://github.com/igtoth/tracedraw/issues/new?template=cdr_file.yml";

pub const PDF_STANDARDS: [&str; 4] = ["PDF 1.4", "PDF/X-1a:2003", "PDF/X-3:2003", "PDF/X-4"];

impl ExportState {
    /// The scale, 1 when it is not a usable number.
    pub fn scale_or_one(&self) -> f64 {
        if self.scale.is_finite() && self.scale > 0.0 {
            self.scale
                .clamp(*EXPORT_SCALE_RANGE.start(), *EXPORT_SCALE_RANGE.end())
        } else {
            1.0
        }
    }

    /// The resolution bitmaps are drawn at: the one the file records,
    /// times the scale.
    pub fn render_dpi(&self) -> f64 {
        (self.dpi * self.scale_or_one()).clamp(MIN_EXPORT_DPI, 2400.0)
    }

    pub fn web() -> Self {
        ExportState {
            format: 2,
            dpi: 96.0,
            web: true,
            ..Default::default()
        }
    }
    pub fn office() -> Self {
        ExportState {
            format: 2,
            dpi: 150.0,
            ..Default::default()
        }
    }
}

pub const EXPORT_FORMATS: [(&str, &str); 18] = [
    ("SVG", "svg"),
    ("PDF", "pdf"),
    ("PNG", "png"),
    ("JPEG", "jpg"),
    ("BMP", "bmp"),
    ("TIFF", "tif"),
    ("WebP", "webp"),
    ("GIF", "gif"),
    ("EPS", "eps"),
    ("AI", "ai"),
    ("DXF", "dxf"),
    ("HTML", "html"),
    ("EMF", "emf"),
    ("WMF", "wmf"),
    ("PLT", "plt"),
    ("PSD", "psd"),
    ("PostScript", "ps"),
    ("PRN", "prn"),
];

/// The formats the Export list offers first, by index into
/// [`EXPORT_FORMATS`]: JPG, PDF, AI and PNG.
pub const EXPORT_COMMON: [usize; 4] = [3, 1, 9, 2];

/// The file name patterns of an export format.
pub fn export_patterns(ext: &str) -> &'static str {
    match ext {
        "jpg" => "*.jpg;*.jpeg",
        "tif" => "*.tif;*.tiff",
        "svg" => "*.svg",
        "pdf" => "*.pdf",
        "png" => "*.png",
        "bmp" => "*.bmp",
        "webp" => "*.webp",
        "gif" => "*.gif",
        "eps" => "*.eps",
        "ai" => "*.ai",
        "dxf" => "*.dxf",
        "html" => "*.html;*.htm",
        "emf" => "*.emf",
        "wmf" => "*.wmf",
        "plt" => "*.plt",
        "psd" => "*.psd",
        "ps" => "*.ps",
        "prn" => "*.prn",
        _ => "*.*",
    }
}

/// "JPG - JPEG Bitmaps (*.jpg;*.jpeg)": the Export list's line for the
/// format at `i`.
pub fn export_format_label(i: usize) -> String {
    let (_, ext) = EXPORT_FORMATS[i.min(EXPORT_FORMATS.len() - 1)];
    format!(
        "{} - {} ({})",
        ext.to_ascii_uppercase(),
        tr(&format!("export.fmt_{ext}")),
        export_patterns(ext)
    )
}

/// The Export list: the common formats, then every format by extension.
/// `None` marks the separator.
pub fn export_format_order() -> Vec<Option<usize>> {
    let mut all: Vec<usize> = (0..EXPORT_FORMATS.len()).collect();
    all.sort_by_key(|i| EXPORT_FORMATS[*i].1);
    let mut out: Vec<Option<usize>> = EXPORT_COMMON.iter().map(|i| Some(*i)).collect();
    out.push(None);
    out.extend(all.into_iter().map(Some));
    out
}

/// Formats whose files keep a title and keywords (tags).
pub fn export_keeps_properties(ext: &str) -> bool {
    matches!(ext, "pdf" | "ai")
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintState {
    /// With Print separations: the plates (in `export::plates` order)
    /// unticked in Print Preview, which do not print (#846).
    pub plates_off: Vec<usize>,
    pub copies: u32,
    pub all_pages: bool,
    pub scale: f64,
    pub fit_to_page: bool,
    pub separations: bool,
    pub crop_marks: bool,
    pub registration_marks: bool,
    pub bleed: bool,
    pub mirror: bool,
    /// Back side of double-sided sheets: columns right to left (#387).
    pub back_side: bool,
    /// Pages keep their size on a fixed sheet: never shrunk to make room
    /// for the marks (#388).
    pub keep_size: bool,
    /// Where the page information sits, moved from its place (mm, x and
    /// y) by a drag in Print Preview (#388).
    pub info_offset: (f64, f64),
    /// The artwork moved on its page before printing (mm, x and y up),
    /// by a drag in Print Preview or its fields (#536).
    pub art_offset: (f64, f64),
    /// The paper's margins kept clear (mm: left, right, top, bottom), set
    /// in Print Preview by value or by dragging their lines (#580, #582).
    pub margins: [f64; 4],
    /// Every page stretched on the sheet about its middle (x, y; 1 for
    /// none) by Print Preview's Transform (#595).
    pub art_stretch: (f64, f64),
    /// The pages stretch to fill the sheet inside the margins (#591).
    pub stretch_fit: bool,
    /// The Custom sheet's size, mm (#591).
    pub custom_sheet: Size,
    /// The name typed to save the Custom size, and a size saved that the
    /// preferences take (#591).
    pub sheet_name_typed: String,
    pub sheet_to_save: Option<(String, Size)>,
    pub invert: bool,
    pub color_bars: bool,
    pub densitometer: bool,
    pub page_info: bool,
    /// Layout: 0 as in the document, 1 n-up, 2 booklet, 3 step and repeat.
    pub layout_kind: usize,
    pub cols: u32,
    pub rows: u32,
    /// The sheet: an index into [`SHEETS`] (0 fits the sheet to the pages).
    pub sheet: usize,
    pub landscape: bool,
    /// Space between pages on the sheet, mm.
    pub gutter: f64,
    /// What the preflight found when the dialog opened.
    pub preflight: Option<Vec<crate::preflight::Issue>>,
    /// The dialog's tab: 0 General, 1 Color, 2 Composite, 3 Layout,
    /// 4 Prepress, 5 PostScript, 6 Issues.
    pub tab: usize,
    /// 0 a printer (`printer`, else the system viewer, which prints),
    /// 1 a PDF file, 2 a PostScript file.
    pub destination: usize,
    /// The system printer the print goes to (a name from `printers`),
    /// `None` for the system PDF viewer.
    pub printer: Option<String>,
    /// The printers the system has, as the dialog opened.
    pub printers: Vec<crate::printers::Printer>,
    /// The printer list is still being read (#614).
    pub looking_for_printers: bool,
    /// A destination was chosen in the dialog: the list read later keeps
    /// it (#614).
    pub printer_picked: bool,
    /// 0 as the pages are, 1 portrait, 2 landscape (fixed sheets).
    pub orientation: usize,
    /// 0 the whole drawing, 1 the current page, 2 the pages in `pages`,
    /// 3 the selection.
    pub range: usize,
    /// Pages to print, e.g. "1-3, 5".
    pub pages: String,
    /// Of the whole drawing or the pages listed: 0 every page, 1 the odd
    /// pages only, 2 the even ones (#951).
    pub parity: usize,
    /// Copies of a file come out as whole sets (on) or page by page.
    pub collate: bool,
    /// Every sheet printed as one picture of `bitmap_dpi`.
    pub as_bitmap: bool,
    pub bitmap_dpi: f64,
    /// Spot inks printed as their CMYK mix.
    pub spot_to_cmyk: bool,
    /// Text in black, whatever its colour.
    pub text_black: bool,
    /// Black fills and outlines overprint what lies below.
    pub overprint_black: bool,
    /// How far beyond the page objects print, mm (with Bleed on).
    pub bleed_limit: f64,
    /// The page information's text; empty = the drawing's name.
    pub info_text: String,
    pub page_numbers: bool,
    /// PostScript language level of a PostScript file (2 or 3).
    pub ps_level: u8,
}

/// The sheets the Print dialog offers: fitted to the pages, then paper
/// sizes (portrait, mm).
/// The sheet index of the Custom size (after the named ones, #591).
pub const CUSTOM_SHEET: usize = SHEETS.len();

pub const SHEETS: [(&str, Option<Size>); 10] = [
    ("print.sheet_fit", None),
    (
        "A5",
        Some(Size {
            width: 148.0,
            height: 210.0,
        }),
    ),
    (
        "A4",
        Some(Size {
            width: 210.0,
            height: 297.0,
        }),
    ),
    (
        "A3",
        Some(Size {
            width: 297.0,
            height: 420.0,
        }),
    ),
    (
        "SRA3",
        Some(Size {
            width: 320.0,
            height: 450.0,
        }),
    ),
    (
        "A2",
        Some(Size {
            width: 420.0,
            height: 594.0,
        }),
    ),
    (
        "Letter",
        Some(Size {
            width: 215.9,
            height: 279.4,
        }),
    ),
    (
        "Legal",
        Some(Size {
            width: 215.9,
            height: 355.6,
        }),
    ),
    (
        "Tabloid",
        Some(Size {
            width: 279.4,
            height: 431.8,
        }),
    ),
    // 16:9, 53.33 x 30 in, wide as it comes (#591).
    (
        "Smart Board 16:9",
        Some(Size {
            width: 1354.582,
            height: 762.0,
        }),
    ),
];

/// The name of the Print dialog's sheet `i` (the first one fits the
/// sheet to the pages).
pub fn sheet_name(i: usize) -> String {
    if i >= CUSTOM_SHEET {
        return tr("print.sheet_custom");
    }
    let (n, _) = SHEETS[i];
    if i == 0 {
        tr(n)
    } else {
        n.to_string()
    }
}

impl PrintState {
    /// The imposition chosen.
    pub fn layout(&self) -> crate::imposition::Layout {
        use crate::imposition::Layout;
        let (cols, rows) = (self.cols.clamp(1, 20), self.rows.clamp(1, 20));
        match self.layout_kind {
            1 => Layout::NUp { cols, rows },
            2 => Layout::Booklet,
            3 => Layout::Repeat { cols, rows },
            _ => Layout::AsIs,
        }
    }

    /// The sheet for pages like `page`: portrait or landscape as the
    /// General tab's Orientation says, or as the Layout tab's Landscape box
    /// when it matches the layout.
    pub fn sheet_size_for(&self, page: Size) -> Option<Size> {
        let s = if self.sheet >= CUSTOM_SHEET {
            let c = self.custom_sheet;
            (c.width.is_finite() && c.height.is_finite() && c.width >= 1.0 && c.height >= 1.0)
                .then_some(c)?
        } else {
            SHEETS.get(self.sheet).and_then(|(_, s)| *s)?
        };
        let _ = page;
        let landscape = match self.orientation {
            1 => false,
            2 => true,
            _ => self.landscape,
        };
        Some(if landscape {
            Size::new(s.height, s.width)
        } else {
            s
        })
    }

    /// The indexes of the pages to print among `count` pages, the current
    /// one being `current`.
    pub fn page_indexes(&self, count: usize, current: usize) -> Vec<usize> {
        let picked = self.range_indexes(count, current);
        // Odd or even pages of the drawing or of the list; the current
        // page and the selection stay as they are, and a choice that
        // leaves no page is let go rather than printing nothing (#951).
        if self.parity == 0 || matches!(self.range, 1 | 3) || self.range == 0 && !self.all_pages {
            return picked;
        }
        let odd = self.parity == 1;
        let kept: Vec<usize> = picked
            .iter()
            .copied()
            .filter(|i| (i % 2 == 0) == odd)
            .collect();
        if kept.is_empty() {
            picked
        } else {
            kept
        }
    }

    /// The pages of the range, before the odd or even choice.
    fn range_indexes(&self, count: usize, current: usize) -> Vec<usize> {
        match self.range {
            1 | 3 => vec![current.min(count.saturating_sub(1))],
            2 => {
                let picked = parse_page_list(&self.pages, count);
                if picked.is_empty() {
                    (0..count).collect()
                } else {
                    picked
                }
            }
            _ if !self.all_pages => vec![current.min(count.saturating_sub(1))],
            _ => (0..count).collect(),
        }
    }
}

/// Page numbers from a list like "1-3, 5, 8-" (1-based, ranges with a
/// dash, an open end running to the last page), as 0-based indexes below
/// `count`, in order, each once.
pub fn parse_page_list(text: &str, count: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for part in text.split([',', ';', ' ']).filter(|p| !p.trim().is_empty()) {
        let part = part.trim();
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (
                a.trim().parse::<usize>().unwrap_or(1),
                b.trim().parse::<usize>().unwrap_or(count),
            ),
            None => match part.parse::<usize>() {
                Ok(n) => (n, n),
                Err(_) => continue,
            },
        };
        for n in a.max(1)..=b.min(count) {
            if !out.contains(&(n - 1)) {
                out.push(n - 1);
            }
        }
    }
    out
}

impl Default for PrintState {
    fn default() -> Self {
        PrintState {
            plates_off: Vec::new(),
            copies: 1,
            all_pages: true,
            scale: 100.0,
            fit_to_page: false,
            separations: false,
            crop_marks: false,
            registration_marks: false,
            bleed: false,
            mirror: false,
            back_side: false,
            keep_size: false,
            info_offset: (0.0, 0.0),
            art_offset: (0.0, 0.0),
            margins: [0.0; 4],
            art_stretch: (1.0, 1.0),
            stretch_fit: false,
            custom_sheet: Size::new(210.0, 297.0),
            sheet_name_typed: String::new(),
            sheet_to_save: None,
            invert: false,
            color_bars: false,
            densitometer: false,
            page_info: false,
            layout_kind: 0,
            cols: 2,
            rows: 2,
            sheet: 0,
            landscape: false,
            gutter: 5.0,
            preflight: None,
            tab: 0,
            destination: 0,
            printer: None,
            printers: Vec::new(),
            looking_for_printers: false,
            printer_picked: false,
            orientation: 0,
            range: 0,
            pages: String::new(),
            parity: 0,
            collate: true,
            as_bitmap: false,
            bitmap_dpi: 300.0,
            spot_to_cmyk: false,
            text_black: false,
            overprint_black: false,
            bleed_limit: 4.0,
            info_text: String::new(),
            page_numbers: false,
            ps_level: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PrintMergeState {
    pub csv: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub path: Option<std::path::PathBuf>,
    /// What each field holds, by field; missing ones are text.
    pub kinds: Vec<crate::print_merge::FieldKind>,
    /// The record the navigation is on (0-based).
    pub current: usize,
    /// View Single Record: one record as a form instead of the grid.
    pub single: bool,
    /// Configure Columns: names, kinds and counter settings shown.
    pub configure: bool,
    /// The Add Column form while it is open: name and kind (0 text,
    /// 1 number, 2 picture, 3 QR code).
    pub adding: Option<(String, usize)>,
    /// The Add Column form's counter settings for a number field: start,
    /// step (0 when "continually increment" is off), digits, and the end
    /// value when one is set (#791).
    pub adding_number: NumberForm,
    /// The Go to Record field.
    pub go_to: String,
    /// The folder picture cells are looked for in first (the Print Merge
    /// docker's Pictures folder, #460).
    pub pictures: Option<std::path::PathBuf>,
    /// The block of cells selected in the grid (#501).
    pub cells: Option<crate::print_merge::CellRange>,
    /// A drag selecting in the grid: what it selects and where it began.
    pub grid_drag: Option<(crate::print_merge::GridDrag, (usize, usize))>,
}

/// The counter of a number field being added (see
/// [`PrintMergeState::adding_number`]).
#[derive(Debug, Clone, PartialEq)]
pub struct NumberForm {
    pub start: i64,
    pub increment: bool,
    pub step: i64,
    pub digits: u32,
    pub use_end: bool,
    pub end: i64,
    /// How the counter is written: numbers, letters or custom text around
    /// the number (#930).
    pub format: crate::print_merge::NumberFormat,
}

impl Default for NumberForm {
    fn default() -> Self {
        NumberForm {
            start: 1,
            increment: true,
            step: 1,
            digits: 1,
            use_end: false,
            end: 1,
            format: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FindReplaceState {
    pub find: String,
    pub replace: String,
    pub match_case: bool,
    pub whole_word: bool,
    pub kind: usize,
    pub results: Vec<tracedraw_core::ShapeId>,
    pub cursor: usize,
}

/// Paste Special: the system clipboard is read once when the dialog opens.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PasteSpecialState {
    pub loaded: bool,
    pub has_text: bool,
    pub is_svg: bool,
    pub has_image: bool,
    /// 0 objects, 1 svg, 2 text, 3 bitmap.
    pub choice: u8,
}

pub use crate::ui::barcode_wizard::BarcodeState;

#[derive(Debug, Clone, PartialEq)]
pub struct TraceState {
    pub preset: crate::trace::Preset,
    pub settings: crate::trace::Settings,
    pub preview_count: Option<usize>,
}

impl TraceState {
    /// The Trace dialog on `preset`, with the choices every trace starts
    /// with (Options > Global > Bitmap Tracing).
    pub fn new(preset: crate::trace::Preset, prefs: &crate::prefs::TracePrefs) -> Self {
        TraceState {
            preset,
            settings: prefs.settings(preset),
            preview_count: None,
        }
    }
}

/// What the message after opening a drawing says.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OpenNotes {
    /// The drawing's file name.
    pub name: String,
    /// Objects left out because they could not be read (0: none, or the
    /// warning is off).
    pub unread: usize,
    /// Fonts the drawing uses that are not installed (empty: none, or the
    /// warning is off).
    pub fonts: Vec<String>,
    pub dont_show: bool,
}

impl OpenNotes {
    /// The message: one paragraph per part.
    pub fn text(&self) -> String {
        let mut parts = Vec::new();
        if self.unread > 0 {
            parts.push(trf(
                "notes.unread_objects",
                &[("n", &self.unread.to_string()), ("name", &self.name)],
            ));
        }
        if !self.fonts.is_empty() {
            parts.push(trf(
                "notes.missing_fonts",
                &[
                    ("name", &self.name),
                    ("fonts", &crate::ops2::with_stand_ins(&self.fonts)),
                ],
            ));
        }
        parts.join("\n\n")
    }
}

/// Text > Encode.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EncodeState {
    pub from: crate::encode::Encoding,
    pub to: crate::encode::Encoding,
}

pub use crate::ui::font_manager::FontManagerState;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PaletteEditorState {
    pub palette: usize,
    pub selected: Option<usize>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Dialog {
    None,
    /// Edit Line Style: custom dash patterns.
    LineStyle(crate::ui::line_style::State),
    /// File > New From Template.
    NewFromTemplate(Box<crate::ui::template_dialog::TemplateState>),
    /// Bitmaps > Mode > Black and White, Duotone and Paletted.
    BitmapBw(crate::bitmap_modes::BwSettings),
    BitmapDuotone(crate::ui::bitmap_dialogs::DuotoneState),
    BitmapPaletted(crate::bitmap_modes::PalettedSettings),
    /// Bitmaps > Remove Background (on this computer, no AI service).
    RemoveBackground(crate::ui::bitmap_dialogs::BgRemoveState),
    /// The Shape tool's Align Nodes: on a common horizontal and/or
    /// vertical.
    NodeAlign {
        horizontal: bool,
        vertical: bool,
    },
    RenamePage {
        name: String,
    },
    GoToPage {
        page: usize,
    },
    InsertPage {
        count: u32,
        after: bool,
    },
    /// Layout > Delete Page: the page (1 = the first) and, with `through`,
    /// the last one of a range to delete with it (#500).
    DeletePage {
        from: usize,
        to: usize,
        through: bool,
    },
    /// Layout > Duplicate Page and the page tab menu: where the copy goes
    /// and whether the layers keep their objects.
    DuplicatePage {
        before: bool,
        contents: bool,
    },
    PageSize {
        width: f64,
        height: f64,
        all_pages: bool,
    },
    Options,
    PageNumberSettings,
    RenameLayer {
        layer: tracedraw_core::LayerId,
        name: String,
    },
    RenameObject {
        shape: tracedraw_core::ShapeId,
        name: String,
    },
    /// How a PDF's text is read in (#386).
    PdfText(crate::ui::pdf_import_dialog::PdfTextState),
    /// The Edit Fill dialog (F11, #361).
    EditFill(Box<crate::ui::edit_fill::EditFillState>),
    /// The Outline Pen dialog (F12, #357).
    OutlinePen(Box<crate::ui::outline_pen::OutlinePenState>),
    /// Text > Edit Text: the text of a text object in a box of its own
    /// (#355).
    EditText {
        shape: tracedraw_core::ShapeId,
        text: String,
    },
    DocumentProperties(Box<crate::ui::doc_properties::DocPropsState>),
    /// Save on a drawing opened from a `.cdr` version TraceDraw does not
    /// save in: choose another version first.
    OldCdrVersion {
        version: u16,
    },
    ConfirmClose,
    /// A font or size picked with no text selected: make it the default
    /// of new text? (as the reference program asks)
    TextDefaults {
        font: Option<String>,
        style: Option<(bool, bool)>,
        size: Option<f64>,
    },
    /// Reset every preference to the defaults? (F8 while TraceDraw
    /// starts, Options > Global.)
    ConfirmResetSettings,
    /// At start-up: drawings a session that ended without closing left in
    /// the auto-backup folder (see `recovery.rs`).
    Recover(Vec<crate::recovery::Found>),
    /// Help > Updates: where the check stands, and a newer release.
    Updates,
    /// After a drawing opens: objects that could not be read, fonts that
    /// are not installed (Options > Global > Warnings).
    OpenNotes(OpenNotes),
    /// Take an item off the Shelf? (Options > Global > Warnings.)
    ConfirmShelfRemove {
        uid: u64,
        name: String,
        dont_ask: bool,
    },
    Export(ExportState),
    Print(PrintState),
    PrintMerge(PrintMergeState),
    /// Object > Create > Pattern Fill: a vector or a bitmap pattern.
    CreatePattern {
        vector: bool,
    },
    /// Table > Insert > Rows / Columns, Split into Rows / Columns: how many.
    TableCount(crate::ui::table_bar::TableCountState),
    CreateTable {
        rows: u32,
        cols: u32,
        /// The table's size in millimetres.
        width: f64,
        height: f64,
    },
    PasteSpecial(PasteSpecialState),
    Barcode(BarcodeState),
    ConvertToBitmap(crate::bitmap_modes::ConvertOptions),
    StraightenImage(crate::ui::straighten_dialog::StraightenState),
    Resample(crate::resample::ResampleState),
    InflateBitmap(InflateState),
    Trace(TraceState),
    /// A bitmap effect's settings (Effects menu, FX section).
    Effect(crate::ui::effect_dialog::EffectState),
    TextTabs,
    TextColumns,
    TextBullets,
    TextDropCap,
    TextStatistics,
    /// Text > Change Case: the case picked, applied on OK.
    ChangeCase(crate::ops2::CaseMode),
    WritingTools(crate::ui::writing_tools::WritingToolsState),
    CustomHyphens(crate::ui::writing_tools::CustomHyphensState),
    Encode(EncodeState),
    BorderGrommet(crate::border_grommet::BorderGrommetState),
    ColorManagement(Box<crate::ui::color_management::ColorManagementState>),
    FontManager(FontManagerState),
    PaletteEditor(PaletteEditorState),
    NewDocument(crate::new_document::NewDocState),
    About,
    /// File > GitHub.
    GithubOpen(crate::github::GithubForm),
    GithubSave(crate::github::GithubForm),
    GithubHistory,
    /// The browser version, each time the page opens: the same program
    /// compiled for the browser, what is slower there, the desktop app.
    WebNotice,
    /// The browser version, at any macro command: macros run in the
    /// desktop app only, with a button to its downloads.
    MacrosDesktopOnly,
    TimeTracker(crate::ui::time_tracker::TrackerDialog),
    Preflight(crate::ui::preflight_dialog::PreflightState),
    PublishPdf(Box<crate::ui::publish_pdf::PublishState>),
    PdfPassword(crate::ui::pdf_password::PdfPasswordState),
    ChangeDefaults(crate::ui::defaults_dialog::State),
    /// File > Save As: folder, name, type, `.cdr` version, properties.
    SaveAs(crate::ui::save_as::SaveAsState),
}

/// Convert to Bitmap: Resolution (a list and a value), Color mode,
/// Dithered (256 colours or fewer), Always overprint black,
/// Anti-aliasing, Transparent background and the uncompressed size.
fn convert_fields(app: &App, ui: &mut Ui, o: &mut crate::bitmap_modes::ConvertOptions) {
    use crate::bitmap_modes::ConvertMode;
    egui::Grid::new("convert_grid")
        .num_columns(2)
        .spacing([10.0, 8.0])
        .show(ui, |ui| {
            ui.label(tr("dialog.resolution"));
            ui.horizontal(|ui| {
                let tip = crate::ui::tips::pair("dialog.resolution", "dialog.convert_dpi_desc");
                egui::ComboBox::from_id_salt("convert_dpi")
                    .selected_text(format!("{:.0}", o.dpi))
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        for d in [72.0, 96.0, 150.0, 200.0, 300.0, 400.0, 600.0] {
                            ui.selectable_value(&mut o.dpi, d, format!("{d:.0}"));
                        }
                    })
                    .response
                    .rich_tip(&tip);
                ui.add(
                    crate::ui::field::NumField::new(&mut o.dpi)
                        .range(36.0..=2400.0)
                        .suffix(" dpi"),
                )
                .rich_tip(&tip);
            });
            ui.end_row();
            ui.label(tr("dialog.color_mode"));
            egui::ComboBox::from_id_salt("convert_mode")
                .selected_text(tr(o.mode.key()))
                .width(190.0)
                .show_ui(ui, |ui| {
                    for m in ConvertMode::ALL {
                        ui.selectable_value(&mut o.mode, m, tr(m.key()));
                    }
                })
                .response
                .rich_tip(crate::ui::tips::pair(
                    "dialog.color_mode",
                    "dialog.convert_mode_desc",
                ));
            ui.end_row();
        });
    ui.add_space(4.0);
    let tip = crate::ui::tips::pair("dialog.dithered", "dialog.convert_dithered_desc");
    ui.add_enabled(
        o.mode.can_dither(),
        egui::Checkbox::new(&mut o.dithered, tr("dialog.dithered")),
    )
    .rich_tip(&tip)
    .rich_disabled_tip(&tip);
    ui.checkbox(&mut o.overprint_black, tr("dialog.overprint_black"))
        .rich_tip(crate::ui::tips::pair(
            "dialog.overprint_black",
            "dialog.convert_overprint_desc",
        ));
    ui.checkbox(&mut o.anti_alias, tr("dialog.anti_aliasing"))
        .rich_tip(crate::ui::tips::pair(
            "dialog.anti_aliasing",
            "dialog.convert_anti_alias_desc",
        ));
    ui.checkbox(&mut o.transparent, tr("dialog.transparent_background"))
        .rich_tip(crate::ui::tips::pair(
            "dialog.transparent_background",
            "dialog.convert_transparent_desc",
        ));
    if let Some(b) = app.selection_bounds() {
        let w = (b.width() / 25.4 * o.dpi).ceil().max(1.0) as u64;
        let h = (b.height() / 25.4 * o.dpi).ceil().max(1.0) as u64;
        let kb = o.bytes(w, h).div_ceil(1024);
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(trf(
                "dialog.uncompressed_size",
                &[
                    ("size", &format!("{kb} KB")),
                    ("w", &w.to_string()),
                    ("h", &h.to_string()),
                ],
            ))
            .color(Tokens::text_dim()),
        );
    }
}

/// Manually Inflate Bitmap: the first selected bitmap's pixel size and
/// the new size as percentages (Inflate by), shown in pixels too
/// (Inflate to).
#[derive(Debug, Clone, PartialEq)]
pub struct InflateState {
    pub base: (u32, u32),
    pub pw: f64,
    pub ph: f64,
    /// Maintain aspect ratio: both percentages move together.
    pub keep: bool,
}

impl InflateState {
    pub fn for_app(app: &App) -> Self {
        let base = app
            .selected_shapes()
            .into_iter()
            .find_map(|s| match s.kind {
                ShapeKind::Bitmap {
                    width_px,
                    height_px,
                    ..
                } => Some((width_px.max(1), height_px.max(1))),
                _ => None,
            })
            .unwrap_or((1, 1));
        InflateState {
            base,
            pw: 100.0,
            ph: 100.0,
            keep: true,
        }
    }

    /// Set the width percentage (the height follows when keeping the
    /// aspect ratio); at least 100.
    pub fn set_width(&mut self, pct: f64) {
        self.pw = pct.clamp(100.0, 1000.0);
        if self.keep {
            self.ph = self.pw;
        }
    }

    pub fn set_height(&mut self, pct: f64) {
        self.ph = pct.clamp(100.0, 1000.0);
        if self.keep {
            self.pw = self.ph;
        }
    }

    /// The new size in pixels.
    pub fn pixels(&self) -> (u32, u32) {
        (
            (self.base.0 as f64 * self.pw / 100.0).round() as u32,
            (self.base.1 as f64 * self.ph / 100.0).round() as u32,
        )
    }
}

/// The Width and Height rows (Inflate to pixels, Inflate by percent) and
/// Maintain aspect ratio.
fn inflate_fields(ui: &mut Ui, st: &mut InflateState) {
    let (px_w, px_h) = st.pixels();
    egui::Grid::new("inflate_grid")
        .num_columns(5)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("");
            ui.label(tr("dialog.inflate_to"));
            ui.label("");
            ui.label(tr("dialog.inflate_by"));
            ui.label("");
            ui.end_row();
            for (row, px, base) in [(0, px_w, st.base.0), (1, px_h, st.base.1)] {
                ui.label(tr(if row == 0 {
                    "dialog.width"
                } else {
                    "dialog.height"
                }));
                let mut p = px as f64;
                if ui
                    .add(
                        crate::ui::field::NumField::new(&mut p)
                            .range(base as f64..=base as f64 * 10.0)
                            .speed(1.0),
                    )
                    .rich_tip(crate::ui::tips::pair(
                        "dialog.inflate_to",
                        "dialog.inflate_to_desc",
                    ))
                    .changed()
                {
                    let pct = p / base.max(1) as f64 * 100.0;
                    if row == 0 {
                        st.set_width(pct);
                    } else {
                        st.set_height(pct);
                    }
                }
                ui.label(tr("dialog.pixels"));
                let mut pct = if row == 0 { st.pw } else { st.ph };
                if ui
                    .add(
                        crate::ui::field::NumField::new(&mut pct)
                            .range(100.0..=1000.0)
                            .speed(0.5)
                            .max_decimals(1),
                    )
                    .rich_tip(crate::ui::tips::pair(
                        "dialog.inflate_by",
                        "dialog.inflate_by_desc",
                    ))
                    .changed()
                {
                    if row == 0 {
                        st.set_width(pct);
                    } else {
                        st.set_height(pct);
                    }
                }
                ui.label("%");
                ui.end_row();
            }
        });
    if ui
        .checkbox(&mut st.keep, tr("dialog.maintain_aspect"))
        .rich_tip(crate::ui::tips::pair(
            "dialog.maintain_aspect",
            "dialog.keep_aspect_desc",
        ))
        .changed()
        && st.keep
    {
        st.ph = st.pw;
    }
}

impl Dialog {
    /// The Duplicate Page dialog with its defaults: after the selected
    /// page, layers with their contents.
    pub fn duplicate_page() -> Dialog {
        Dialog::DuplicatePage {
            before: false,
            contents: true,
        }
    }
}

/// A dialog window in the dialogs' chrome (white title bar with a close
/// button, grey body), sized to its content and centred.
pub(crate) fn window(_ctx: &Context, title: String) -> crate::ui::chrome::Window {
    crate::ui::chrome::window(title)
}

/// OK and Cancel, 100 x 27 and aligned right; OK is the default button.
/// Enter presses OK (unless a multi-line field has the focus); Esc is
/// handled by the keyboard handler, which closes any dialog.
pub(crate) fn ok_cancel(ui: &mut Ui, close: &mut bool) -> bool {
    let mut ok = false;
    let ok_label = tr("dialog.ok");
    let cancel_label = tr("dialog.cancel");
    match crate::ui::chrome::button_row(ui, &[&ok_label, &cancel_label], false) {
        Some(0) => {
            ok = true;
            *close = true;
        }
        Some(_) => *close = true,
        None => {}
    }
    if !ok && enter_pressed(ui) {
        ok = true;
        *close = true;
    }
    ok
}

/// "?" on the left, OK and Cancel on the right, as dialogs with help
/// have them. Returns (OK pressed, "?" pressed).
pub(crate) fn help_ok_cancel(ui: &mut Ui, close: &mut bool) -> (bool, bool) {
    let mut help = false;
    let mut ok = false;
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        help = ui
            .add_sized([27.0, 27.0], egui::Button::new("?"))
            .rich_tip(crate::ui::tips::keyed("dialog.help_button"))
            .clicked();
        ok = ok_cancel(ui, close);
    });
    (ok, help)
}

/// Names of the dialogs' multi-line fields, where Enter inserts a line.
const MULTILINE_NAMES: [&str; 2] = ["dialog_multiline_qr", "dialog_multiline_notes"];

pub(crate) fn multiline_id(i: usize) -> egui::Id {
    egui::Id::new(MULTILINE_NAMES[i.min(MULTILINE_NAMES.len() - 1)])
}

/// Enter was pressed this frame and no multi-line text field holds it.
pub(crate) fn enter_pressed(ui: &Ui) -> bool {
    ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift)
        && !ui.memory(|m| {
            m.focused()
                .is_some_and(|id| MULTILINE_NAMES.iter().any(|n| egui::Id::new(n) == id))
        })
}

fn unit_value(ui: &mut Ui, label: &str, tip: &str, mm: &mut f64, units: Units) -> bool {
    let mut v = units.from_mm(*mm);
    let r = crate::ui::tips::tip_over(ui, tip, |ui| {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(
                crate::ui::field::NumField::new(&mut v)
                    .speed(0.5)
                    .suffix(format!(" {}", units.short())),
            )
            .changed()
        })
        .inner
    });
    if r {
        *mm = units.to_mm(v);
    }
    r
}

pub fn show(app: &mut App, ctx: &Context) {
    crate::ui::chrome::set_center_dialogs(ctx, app.settings.center_dialogs);
    // An options dialog replaced by another dialog keeps its changes.
    if !matches!(app.dialog, Dialog::Options) {
        app.options_snapshot = None;
        app.shortcut_editor.capture = None;
    }
    crate::ui::line_style::apply_units_request(app, ctx);
    let line_style = crate::ui::line_style::take_request(ctx);
    if line_style && matches!(app.dialog, Dialog::OutlinePen(_)) {
        // Edit Line Style from the Outline Pen dialog: back to it after.
        let back = std::mem::replace(&mut app.dialog, Dialog::LineStyle(Default::default()));
        app.dialog_return = Some(Box::new(back));
    } else if line_style && matches!(app.dialog, Dialog::None) {
        app.dialog = Dialog::LineStyle(Default::default());
    } else if matches!(app.dialog, Dialog::None) {
        if let Some(back) = app.dialog_return.take() {
            app.dialog = *back;
        }
    }
    // The Edit Fill dialog closed some other way (Esc): its preview goes.
    if app.fill_preview && !matches!(app.dialog, Dialog::EditFill(_)) {
        app.end_edit_fill_preview();
    }
    // The same for the Outline Pen dialog's (#741); Edit Line Style opened
    // from it takes the preview back, which comes again on the return.
    if app.outline_preview && !matches!(app.dialog, Dialog::OutlinePen(_)) {
        app.end_outline_pen_preview();
    }
    let mut dialog = std::mem::replace(&mut app.dialog, Dialog::None);
    app.dialog_help = crate::help::dialog_topic(&dialog, app.options_page);
    if !matches!(dialog, Dialog::None) {
        // The Customization options leave the toolbars whose buttons they
        // let move open (#563).
        let holes: Vec<egui::Rect> = if matches!(dialog, Dialog::Options)
            && app.options_page.set() == crate::ui::options::OptionsSet::Customization
        {
            app.shown_bar_slots
                .iter()
                .map(|s| s.rect.expand(2.0))
                .collect()
        } else {
            Vec::new()
        };
        crate::ui::chrome::modal_blocker_except(ctx, &holes);
    }
    let mut close = false;
    match &mut dialog {
        Dialog::None => {}
        Dialog::LineStyle(st) => crate::ui::line_style::dialog(app, ctx, st, &mut close),
        Dialog::BitmapBw(s) => crate::ui::bitmap_dialogs::bw_dialog(app, ctx, s, &mut close),
        Dialog::RemoveBackground(st) => {
            crate::ui::bitmap_dialogs::remove_background_dialog(app, ctx, st, &mut close)
        }
        Dialog::TimeTracker(st) => crate::ui::time_tracker::dialog(app, ctx, st, &mut close),
        Dialog::TableCount(st) => crate::ui::table_bar::count_dialog(app, ctx, st, &mut close),
        Dialog::Preflight(st) => crate::ui::preflight_dialog::dialog(app, ctx, st, &mut close),
        Dialog::PublishPdf(st) => crate::ui::publish_pdf::dialog(app, ctx, st, &mut close),
        Dialog::PdfPassword(st) => crate::ui::pdf_password::dialog(app, ctx, st, &mut close),
        Dialog::ChangeDefaults(st) => crate::ui::defaults_dialog::dialog(app, ctx, st, &mut close),
        Dialog::SaveAs(st) => crate::ui::save_as::dialog(app, ctx, st, &mut close),
        Dialog::BitmapDuotone(st) => {
            crate::ui::bitmap_dialogs::duotone_dialog(app, ctx, st, &mut close)
        }
        Dialog::BitmapPaletted(s) => {
            crate::ui::bitmap_dialogs::paletted_dialog(app, ctx, s, &mut close)
        }
        Dialog::NodeAlign {
            horizontal,
            vertical,
        } => {
            window(ctx, tr("dialog.node_align")).show(ctx, |ui| {
                ui.checkbox(horizontal, tr("dialog.align_horizontal"))
                    .rich_tip(crate::ui::tips::keyed("dialog.align_horizontal"));
                ui.checkbox(vertical, tr("dialog.align_vertical"))
                    .rich_tip(crate::ui::tips::keyed("dialog.align_vertical"));
                if ok_cancel(ui, &mut close) && (*horizontal || *vertical) {
                    let (h, v) = (*horizontal, *vertical);
                    app.align_selected_nodes(h, v);
                }
            });
        }
        Dialog::RenamePage { name } => {
            window(ctx, tr("dialog.rename_page")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.page_name"));
                    ui.text_edit_singleline(name)
                        .rich_tip(crate::ui::tips::keyed("dialog.page_name"));
                });
                if ok_cancel(ui, &mut close) {
                    let page = app.page;
                    app.run(Command::RenamePage {
                        page,
                        name: name.clone(),
                    });
                }
            });
        }
        Dialog::GoToPage { page } => {
            let n = app.doc().pages.len();
            window(ctx, tr("dialog.go_to_page")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(trf("dialog.go_to_page_range", &[("n", &n.to_string())]));
                    ui.add(crate::ui::field::NumField::new(page).range(1..=n.max(1)))
                        .rich_tip(crate::ui::tips::pair(
                            "dialog.go_to_page",
                            "dialog.go_to_page_desc",
                        ));
                });
                if ok_cancel(ui, &mut close) {
                    app.goto_page(page.saturating_sub(1));
                }
            });
        }
        Dialog::InsertPage { count, after } => {
            window(ctx, tr("dialog.insert_page")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.number_of_pages"));
                    ui.add(crate::ui::field::NumField::new(count).range(1..=999))
                        .rich_tip(crate::ui::tips::keyed("dialog.number_of_pages"));
                });
                ui.horizontal(|ui| {
                    ui.radio_value(after, false, tr("dialog.before")).rich_tip(
                        crate::ui::tips::pair("dialog.before", "dialog.insert_before_desc"),
                    );
                    ui.radio_value(after, true, tr("dialog.after")).rich_tip(
                        crate::ui::tips::pair("dialog.after", "dialog.insert_after_desc"),
                    );
                });
                let s = app.page_size();
                ui.label(format!(
                    "{} x {} {}",
                    app.units.from_mm(s.width).round(),
                    app.units.from_mm(s.height).round(),
                    app.units.short()
                ));
                if ok_cancel(ui, &mut close) {
                    let size = app.page_size();
                    let idx = app.page_index();
                    let base = app.doc().pages.len();
                    let cmds: Vec<Command> = (0..*count)
                        .map(|k| Command::AddPage {
                            name: Some(trf(
                                "doc.page_n",
                                &[("n", &(base + k as usize + 1).to_string())],
                            )),
                            size,
                        })
                        .collect();
                    let _ = app.engine.run_batch("Insert Page", &cmds);
                    // Move the new pages next to the current one.
                    let total = app.doc().pages.len();
                    let target = if *after { idx + 1 } else { idx };
                    for k in 0..*count as usize {
                        let pid = app.doc().pages[total - *count as usize + k].id;
                        app.run(Command::MovePage {
                            page: pid,
                            to: target + k,
                        });
                    }
                    app.goto_page(target);
                }
            });
        }
        Dialog::DeletePage { from, to, through } => {
            let n = app.doc().pages.len();
            window(ctx, tr("dialog.delete_page")).show(ctx, |ui| {
                egui::Grid::new("delete_page")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(tr("dialog.delete_page_from"));
                        ui.add(crate::ui::field::NumField::new(from).range(1..=n.max(1)))
                            .rich_tip(crate::ui::tips::pair(
                                "dialog.delete_page_from",
                                "dialog.delete_from_desc",
                            ));
                        ui.end_row();
                        ui.checkbox(through, tr("dialog.delete_through"))
                            .rich_tip(crate::ui::tips::keyed("dialog.delete_through"));
                        let tip =
                            crate::ui::tips::pair("dialog.delete_through", "dialog.delete_to_desc");
                        ui.add_enabled(
                            *through,
                            crate::ui::field::NumField::new(to).range(1..=n.max(1)),
                        )
                        .rich_tip(&tip)
                        .rich_disabled_tip(&tip);
                        ui.end_row();
                    });
                let (a, b) = delete_page_range(*from, *to, *through, n);
                let all = b + 1 - a >= n;
                if all {
                    ui.colored_label(Tokens::text_dim(), tr("dialog.delete_keeps_one"));
                }
                if ok_cancel(ui, &mut close) && !all {
                    app.delete_pages(a, b);
                }
            });
        }
        Dialog::DuplicatePage { before, contents } => {
            let mut help = false;
            window(ctx, tr("dialog.duplicate_page")).show(ctx, |ui| {
                ui.label(tr("dialog.insert_new_page"));
                ui.indent("duplicate_page_where", |ui| {
                    ui.radio_value(before, true, tr("dialog.before_selected_page"))
                        .rich_tip(crate::ui::tips::keyed("dialog.before_selected_page"));
                    ui.radio_value(before, false, tr("dialog.after_selected_page"))
                        .rich_tip(crate::ui::tips::keyed("dialog.after_selected_page"));
                });
                ui.add_space(6.0);
                ui.radio_value(contents, false, tr("dialog.copy_layers_only"))
                    .rich_tip(crate::ui::tips::keyed("dialog.copy_layers_only"));
                ui.radio_value(contents, true, tr("dialog.copy_layers_and_contents"))
                    .rich_tip(crate::ui::tips::keyed("dialog.copy_layers_and_contents"));
                let (ok, h) = help_ok_cancel(ui, &mut close);
                help = h;
                if ok {
                    let (b, c) = (*before, *contents);
                    app.duplicate_page(b, c);
                }
            });
            if help {
                app.open_dialog_help();
            }
        }
        Dialog::PageSize {
            width,
            height,
            all_pages,
        } => {
            window(ctx, tr("dialog.page_size")).show(ctx, |ui| {
                let u = app.units;
                let tip = crate::ui::tips::pair("dialog.width", "dialog.page_width_desc");
                unit_value(ui, &tr("dialog.width"), &tip, width, u);
                let tip = crate::ui::tips::pair("dialog.height", "dialog.page_height_desc");
                unit_value(ui, &tr("dialog.height"), &tip, height, u);
                ui.horizontal(|ui| {
                    let saved = &app.settings.page_sizes;
                    let current = paper_name(Size::new(*width, *height), saved)
                        .unwrap_or_else(|| tr("toolbar.custom"));
                    crate::ui::tips::tip_over(
                        ui,
                        crate::ui::tips::pair("dialog.page_size", "toolbar.page_size_desc"),
                        |ui| {
                            egui::ComboBox::from_id_salt("page_size_dialog_preset")
                                .selected_text(current.clone())
                                .width(200.0)
                                .height(400.0)
                                .show_ui(ui, |ui| {
                                    if let Some((_, s)) = paper_preset_menu(ui, &current, saved) {
                                        *width = s.width;
                                        *height = s.height;
                                    }
                                });
                        },
                    );
                    if ui
                        .button(tr("dialog.swap"))
                        .rich_tip(crate::ui::tips::keyed("dialog.swap"))
                        .clicked()
                    {
                        std::mem::swap(width, height);
                    }
                });
                ui.checkbox(all_pages, tr("dialog.apply_all_pages"))
                    .rich_tip(crate::ui::tips::keyed("dialog.apply_all_pages"));
                if ok_cancel(ui, &mut close) {
                    let size = Size::new(width.max(1.0), height.max(1.0));
                    let pages: Vec<_> = if *all_pages {
                        app.doc().pages.iter().map(|p| p.id).collect()
                    } else {
                        vec![app.page]
                    };
                    let cmds: Vec<Command> = pages
                        .into_iter()
                        .map(|page| Command::ResizePage { page, size })
                        .collect();
                    let _ = app.engine.run_batch("Page Size", &cmds);
                    app.fit_pending = true;
                }
            });
        }
        Dialog::RenameLayer { layer, name } => {
            window(ctx, tr("dialog.rename_layer")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.layer_name"));
                    ui.text_edit_singleline(name)
                        .rich_tip(crate::ui::tips::keyed("dialog.layer_name"));
                });
                if ok_cancel(ui, &mut close) {
                    app.run(Command::RenameLayer {
                        layer: *layer,
                        name: name.clone(),
                    });
                }
            });
        }
        Dialog::RenameObject { shape, name } => {
            window(ctx, tr("dialog.rename_object")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.object_name"));
                    ui.text_edit_singleline(name);
                });
                if ok_cancel(ui, &mut close) {
                    let trimmed = name.trim();
                    app.run(Command::SetShapeName {
                        shape: *shape,
                        name: (!trimmed.is_empty()).then(|| trimmed.to_string()),
                    });
                }
            });
        }
        Dialog::EditText { shape, text } => {
            let title = tr("menu.text.edit_text");
            let title = title
                .trim_end_matches("...")
                .trim_end_matches('\u{2026}')
                .trim()
                .to_string();
            window(ctx, title).show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::multiline(text)
                        .id(egui::Id::new("edit_text_dialog_box"))
                        .desired_width(480.0)
                        .desired_rows(10),
                );
                if ok_cancel(ui, &mut close) {
                    app.set_text_content(*shape, text);
                }
            });
        }
        Dialog::Options => crate::ui::options::options_dialog(app, ctx, &mut close),
        Dialog::OutlinePen(st) => crate::ui::outline_pen::dialog(app, ctx, st, &mut close),
        Dialog::EditFill(st) => crate::ui::edit_fill::dialog(app, ctx, st, &mut close),
        Dialog::PdfText(st) => crate::ui::pdf_import_dialog::dialog(app, ctx, st, &mut close),
        Dialog::OldCdrVersion { version } => {
            let mut choose = false;
            window(ctx, tr("dialog.old_cdr_title")).show(ctx, |ui| {
                ui.set_max_width(420.0);
                ui.label(trf(
                    "dialog.old_cdr_text",
                    &[
                        ("version", &version.to_string()),
                        ("oldest", &tracedraw_cdr::write::CHECKED_SINCE.to_string()),
                        ("newest", &tracedraw_cdr::write::CHECKED_UNTIL.to_string()),
                    ],
                ));
                let a = tr("dialog.old_cdr_choose");
                let b = tr("dialog.cancel");
                match crate::ui::chrome::button_row(ui, &[&a, &b], false) {
                    Some(0) => choose = true,
                    Some(_) => close = true,
                    None => {}
                }
                if enter_pressed(ui) {
                    choose = true;
                }
            });
            if choose {
                app.save_as_from_old_cdr();
                return;
            }
        }
        Dialog::DocumentProperties(st) => {
            crate::ui::doc_properties::dialog(app, ctx, st, &mut close)
        }
        Dialog::ConfirmClose => {
            let text = trf("dialog.save_changes_to", &[("name", &app.document_name())]);
            let (yes, no, cancel) = (tr("dialog.yes"), tr("dialog.no"), tr("dialog.cancel"));
            match crate::ui::chrome::message_box(
                ctx,
                crate::ui::chrome::MessageIcon::Question,
                &text,
                &[&yes, &no, &cancel],
            ) {
                Some(0) => {
                    app.save(false);
                    let saved = !app.engine.is_dirty();
                    if saved {
                        app.close_active_document();
                    }
                    close = true;
                    app.after_close_question(saved);
                }
                Some(1) => {
                    app.close_active_document();
                    close = true;
                    app.after_close_question(true);
                }
                Some(_) => {
                    close = true;
                    app.after_close_question(false);
                }
                None => {}
            }
        }
        Dialog::Updates => {
            use crate::updates::UpdateState;
            let version = env!("CARGO_PKG_VERSION");
            let close_label = tr("dialog.close");
            let install = app.installer.get();
            match app.updates.get() {
                UpdateState::Available(r) if install != crate::updates::InstallState::Idle => {
                    use crate::updates::InstallState;
                    let (text, buttons): (String, Vec<String>) = match &install {
                        InstallState::Downloading { done, total } => {
                            ctx.request_repaint_after(std::time::Duration::from_millis(200));
                            let mb = |b: u64| format!("{:.1}", b as f64 / 1_048_576.0);
                            (
                                trf(
                                    "updates.downloading",
                                    &[
                                        ("new", &r.version),
                                        ("done", &mb(*done)),
                                        ("total", &mb(*total)),
                                    ],
                                ),
                                vec![close_label.clone()],
                            )
                        }
                        InstallState::Installed(v) => (
                            trf("updates.installed", &[("new", v)]),
                            vec![close_label.clone()],
                        ),
                        InstallState::Failed(e) => (
                            trf("updates.install_failed", &[("error", e)]),
                            vec![tr("updates.download"), close_label.clone()],
                        ),
                        InstallState::Idle => (String::new(), vec![close_label.clone()]),
                    };
                    let labels: Vec<&str> = buttons.iter().map(String::as_str).collect();
                    match crate::ui::chrome::message_box(
                        ctx,
                        crate::ui::chrome::MessageIcon::Info,
                        &text,
                        &labels,
                    ) {
                        Some(0) if matches!(install, InstallState::Failed(_)) => {
                            let url = r.download.clone().unwrap_or_else(|| r.page.clone());
                            app.open_url(&url);
                            close = true;
                        }
                        Some(_) => close = true,
                        None => {}
                    }
                }
                UpdateState::Idle | UpdateState::Checking => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(200));
                    if crate::ui::chrome::message_box(
                        ctx,
                        crate::ui::chrome::MessageIcon::Busy,
                        &tr("updates.checking"),
                        &[&tr("dialog.cancel")],
                    )
                    .is_some()
                    {
                        close = true;
                    }
                }
                UpdateState::Latest => {
                    let text = trf("updates.latest", &[("version", version)]);
                    if crate::ui::chrome::message_box(
                        ctx,
                        crate::ui::chrome::MessageIcon::Info,
                        &text,
                        &[&close_label],
                    )
                    .is_some()
                    {
                        close = true;
                    }
                }
                UpdateState::Available(r) => {
                    let text = trf(
                        "updates.available",
                        &[("new", &r.version), ("version", version)],
                    );
                    let (download, notes, skip) = (
                        tr("updates.download"),
                        tr("updates.notes"),
                        tr("updates.skip"),
                    );
                    // A running AppImage can be replaced in place.
                    let target = crate::updates::appimage_target();
                    let in_place = r.appimage.clone().zip(target);
                    let install_label = tr("updates.install");
                    let mut labels: Vec<&str> = Vec::new();
                    if in_place.is_some() {
                        labels.push(&install_label);
                    }
                    labels.extend([download.as_str(), &notes, &skip, &close_label]);
                    let first = usize::from(in_place.is_some());
                    match crate::ui::chrome::message_box(
                        ctx,
                        crate::ui::chrome::MessageIcon::Info,
                        &text,
                        &labels,
                    )
                    .map(|i| {
                        if in_place.is_some() && i == 0 {
                            usize::MAX
                        } else {
                            i - first
                        }
                    }) {
                        Some(usize::MAX) => {
                            if let Some((package, target)) = in_place {
                                app.installer.start(
                                    r.version.clone(),
                                    package,
                                    target,
                                    Some(ctx.clone()),
                                );
                            }
                        }
                        Some(0) => {
                            let url = r.download.clone().unwrap_or_else(|| r.page.clone());
                            app.open_url(&url);
                            close = true;
                        }
                        Some(1) => app.open_url(&r.page),
                        Some(2) => {
                            app.settings.skipped_update = r.version.clone();
                            close = true;
                        }
                        Some(_) => close = true,
                        None => {}
                    }
                }
                UpdateState::Failed(e) => {
                    let text = trf("updates.failed", &[("error", &e)]);
                    let open = tr("updates.open_releases");
                    match crate::ui::chrome::message_box(
                        ctx,
                        crate::ui::chrome::MessageIcon::Info,
                        &text,
                        &[&open, &close_label],
                    ) {
                        Some(0) => {
                            app.open_url(crate::updates::RELEASES_PAGE);
                            close = true;
                        }
                        Some(_) => close = true,
                        None => {}
                    }
                }
            }
        }
        Dialog::Recover(found) => {
            let n = found.iter().map(|f| f.backups.len()).sum::<usize>();
            let names: Vec<String> = found
                .iter()
                .flat_map(|f| &f.backups)
                .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                .map(|s| s.trim_start_matches("AutoBackup_of_").to_string())
                .collect();
            let text = trf(
                "dialog.recover_question",
                &[("n", &n.to_string()), ("names", &names.join(", "))],
            );
            let (recover, discard) = (tr("dialog.recover"), tr("dialog.discard"));
            match crate::ui::chrome::message_box(
                ctx,
                crate::ui::chrome::MessageIcon::Question,
                &text,
                &[&recover, &discard],
            ) {
                Some(0) => {
                    let found = found.clone();
                    app.recover(&found);
                    close = true;
                }
                Some(_) => {
                    crate::recovery::dismiss(found, false);
                    close = true;
                }
                None => {}
            }
        }
        Dialog::TextDefaults { font, style, size } => {
            let (ok, cancel) = (tr("dialog.ok"), tr("dialog.cancel"));
            match crate::ui::chrome::message_box(
                ctx,
                crate::ui::chrome::MessageIcon::Question,
                &tr("dialog.text_defaults_question"),
                &[&ok, &cancel],
            ) {
                Some(0) => {
                    app.set_text_defaults(font.take(), *style, *size);
                    close = true;
                }
                Some(_) => close = true,
                None => {}
            }
        }
        Dialog::ConfirmResetSettings => {
            let (reset, cancel) = (tr("dialog.reset"), tr("dialog.cancel"));
            match crate::ui::chrome::message_box(
                ctx,
                crate::ui::chrome::MessageIcon::Question,
                &tr("dialog.reset_settings_question"),
                &[&reset, &cancel],
            ) {
                Some(0) => {
                    app.reset_settings();
                    close = true;
                }
                Some(_) => close = true,
                None => {}
            }
        }
        Dialog::OpenNotes(notes) => {
            let ok = tr("dialog.ok");
            let report = tr("notes.report");
            // Unread objects: a way to send the file in.
            let buttons: Vec<&str> = if notes.unread > 0 {
                vec![&ok, &report]
            } else {
                vec![&ok]
            };
            let text = notes.text();
            let picked = crate::ui::chrome::message_box_check(
                ctx,
                crate::ui::chrome::MessageIcon::Info,
                &text,
                &buttons,
                &tr("notes.dont_show"),
                &mut notes.dont_show,
            );
            if let Some(i) = picked {
                if notes.dont_show {
                    if notes.unread > 0 {
                        app.settings.warnings.unread_objects = false;
                    }
                    if !notes.fonts.is_empty() {
                        app.settings.warnings.missing_fonts = false;
                    }
                    app.settings.save();
                }
                if i == 1 && notes.unread > 0 {
                    app.open_url(crate::ui::dialogs::CDR_ISSUE);
                }
                close = true;
            }
        }
        Dialog::ConfirmShelfRemove {
            uid,
            name,
            dont_ask,
        } => {
            let (remove, cancel) = (tr("shelf.remove"), tr("dialog.cancel"));
            match crate::ui::chrome::message_box_check(
                ctx,
                crate::ui::chrome::MessageIcon::Question,
                &trf("shelf.confirm_remove", &[("name", name)]),
                &[&remove, &cancel],
                &tr("shelf.dont_ask"),
                dont_ask,
            ) {
                Some(0) => {
                    if *dont_ask {
                        app.settings.warnings.shelf_remove = false;
                        app.settings.save();
                    }
                    let uid = *uid;
                    crate::ui::shelf_docker::remove_now(app, uid);
                    close = true;
                }
                Some(_) => close = true,
                None => {}
            }
        }
        Dialog::Export(st) => export_dialog(app, ctx, st, &mut close),
        Dialog::Print(st) => print_dialog(app, ctx, st, &mut close),
        Dialog::PrintMerge(st) => print_merge_dialog(app, ctx, st, &mut close),
        Dialog::GithubOpen(f) => crate::ui::github_dialogs::open_dialog(app, ctx, f, &mut close),
        Dialog::GithubSave(f) => crate::ui::github_dialogs::save_dialog(app, ctx, f, &mut close),
        Dialog::GithubHistory => crate::ui::github_dialogs::history_dialog(app, ctx, &mut close),
        Dialog::CreatePattern { vector } => {
            window(ctx, tr("dialog.create_pattern")).show(ctx, |ui| {
                ui.label(tr("dialog.create_pattern_type"));
                ui.radio_value(vector, true, tr("dialog.create_pattern_vector"))
                    .rich_tip(crate::ui::tips::keyed("dialog.create_pattern_vector"));
                ui.radio_value(vector, false, tr("dialog.create_pattern_bitmap"))
                    .rich_tip(crate::ui::tips::keyed("dialog.create_pattern_bitmap"));
                if ok_cancel(ui, &mut close) {
                    if *vector {
                        app.create_vector_pattern_from_selection();
                    } else {
                        app.create_pattern_from_selection();
                    }
                }
            });
        }
        Dialog::CreateTable {
            rows,
            cols,
            width,
            height,
        } => {
            let units = app.units;
            let mut help = false;
            window(ctx, tr("dialog.create_table")).show(ctx, |ui| {
                egui::Grid::new("create_table")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        let label = |ui: &mut Ui, key: &str| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(tr(key));
                                },
                            );
                        };
                        label(ui, "dialog.table_rows");
                        ui.add_sized(
                            [120.0, 22.0],
                            crate::ui::field::NumField::new(rows).range(1..=200),
                        )
                        .rich_tip(crate::ui::tips::pair(
                            "dialog.table_rows",
                            "dialog.new_table_rows_desc",
                        ));
                        ui.end_row();
                        label(ui, "dialog.table_columns");
                        ui.add_sized(
                            [120.0, 22.0],
                            crate::ui::field::NumField::new(cols).range(1..=200),
                        )
                        .rich_tip(crate::ui::tips::pair(
                            "dialog.table_columns",
                            "dialog.new_table_columns_desc",
                        ));
                        ui.end_row();
                        for (key, mm) in [
                            ("dialog.height", &mut *height),
                            ("dialog.width", &mut *width),
                        ] {
                            label(ui, key);
                            let mut v = units.from_mm(*mm);
                            if ui
                                .add_sized(
                                    [120.0, 22.0],
                                    crate::ui::field::NumField::new(&mut v)
                                        .speed(0.1)
                                        .max_decimals(3)
                                        .range(units.from_mm(1.0)..=units.from_mm(5000.0))
                                        .suffix(format!(" {}", units.short())),
                                )
                                .rich_tip_with(|| {
                                    crate::ui::tips::pair(key, "dialog.new_table_size_desc")
                                })
                                .changed()
                            {
                                *mm = units.to_mm(v);
                            }
                            ui.end_row();
                        }
                    });
                let (ok, h) = help_ok_cancel(ui, &mut close);
                help = h;
                if ok {
                    app.table_rows = *rows;
                    app.table_cols = *cols;
                    app.table_size = (*width, *height);
                    let page = app.page_rect();
                    let (w, h) = (*width, *height);
                    let c = page.center();
                    let rect =
                        Rect::new(c.x - w / 2.0, c.y - h / 2.0, c.x + w / 2.0, c.y + h / 2.0);
                    let table = tracedraw_core::Table::new(
                        rect,
                        *rows,
                        *cols,
                        Some(tracedraw_core::Stroke::hairline(Color::BLACK)),
                    );
                    if let Some(id) = app.new_shape(ShapeKind::Table(table)) {
                        app.select(vec![id]);
                    }
                }
            });
            if help {
                app.open_dialog_help();
            }
        }
        Dialog::PasteSpecial(st) => {
            if !st.loaded {
                let c = crate::clipboard::read_system();
                st.has_text = c.text.is_some();
                st.is_svg = c.is_svg();
                st.has_image = c.image.is_some();
                st.choice = if app.clipboard.is_some() {
                    0
                } else if st.is_svg {
                    1
                } else if st.has_image {
                    3
                } else {
                    2
                };
                st.loaded = true;
            }
            window(ctx, tr("dialog.paste_special")).show(ctx, |ui| {
                ui.label(tr("dialog.paste_as"));
                let options: [(u8, String, bool); 4] = [
                    (0, tr("dialog.paste_objects"), app.clipboard.is_some()),
                    (1, tr("dialog.paste_svg"), st.is_svg),
                    (2, tr("dialog.paste_text"), st.has_text),
                    (3, tr("dialog.paste_bitmap"), st.has_image),
                ];
                let any = options.iter().any(|(_, _, ok)| *ok);
                for (v, label, ok) in &options {
                    ui.add_enabled_ui(*ok, |ui| {
                        ui.radio_value(&mut st.choice, *v, label);
                    });
                }
                if !any {
                    ui.label(
                        egui::RichText::new(tr("dialog.clipboard_empty")).color(Tokens::text_dim()),
                    );
                }
                if ok_cancel(ui, &mut close) && any {
                    let c = crate::clipboard::read_system();
                    match st.choice {
                        0 => app.paste(),
                        1 => {
                            if let Some(t) = c.text {
                                app.paste_as_svg(&t);
                            }
                        }
                        2 => {
                            if let Some(t) = c.text {
                                app.paste_as_text(&t);
                            }
                        }
                        _ => {
                            if let Some(img) = c.image {
                                app.paste_as_bitmap(img);
                            }
                        }
                    }
                }
            });
        }
        Dialog::Barcode(st) => crate::ui::barcode_wizard::dialog(app, ctx, st, &mut close),
        Dialog::ConvertToBitmap(o) => {
            window(ctx, tr("dialog.convert_to_bitmap")).show(ctx, |ui| {
                convert_fields(app, ui, o);
                if ok_cancel(ui, &mut close) {
                    app.convert_to_bitmap_with(o);
                }
            });
        }
        Dialog::StraightenImage(st) => {
            crate::ui::straighten_dialog::straighten_dialog(app, ctx, st, &mut close)
        }
        Dialog::Resample(st) => {
            let mut apply = false;
            window(ctx, tr("dialog.resample")).show(ctx, |ui| {
                use crate::ui::field::NumField;
                egui::Grid::new("resample_grid")
                    .num_columns(3)
                    .min_col_width(96.0)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("");
                        ui.label(tr("resample.width"));
                        ui.label(tr("resample.height"));
                        ui.end_row();
                        ui.label(tr("resample.pixels_row"));
                        let (mut w, mut h) = (st.w as f64, st.h as f64);
                        if ui
                            .add(
                                NumField::new(&mut w)
                                    .range(1.0..=20000.0)
                                    .fixed_decimals(0)
                                    .suffix(" px")
                                    .width(90.0),
                            )
                            .rich_tip(crate::ui::tips::pair(
                                "resample.width",
                                "resample.width_px_desc",
                            ))
                            .changed()
                        {
                            st.set_w(w.round() as u32);
                        }
                        if ui
                            .add(
                                NumField::new(&mut h)
                                    .range(1.0..=20000.0)
                                    .fixed_decimals(0)
                                    .suffix(" px")
                                    .width(90.0),
                            )
                            .rich_tip(crate::ui::tips::pair(
                                "resample.height",
                                "resample.height_px_desc",
                            ))
                            .changed()
                        {
                            st.set_h(h.round() as u32);
                        }
                        ui.end_row();
                        ui.label(tr("resample.percent_row"));
                        let (mut px, mut py) = st.percent();
                        let cx = ui
                            .add(
                                NumField::new(&mut px)
                                    .range(0.1..=10000.0)
                                    .fixed_decimals(1)
                                    .suffix(" %")
                                    .width(90.0),
                            )
                            .rich_tip(crate::ui::tips::pair(
                                "resample.width",
                                "resample.width_pct_desc",
                            ))
                            .changed();
                        let cy = ui
                            .add(
                                NumField::new(&mut py)
                                    .range(0.1..=10000.0)
                                    .fixed_decimals(1)
                                    .suffix(" %")
                                    .width(90.0),
                            )
                            .rich_tip(crate::ui::tips::pair(
                                "resample.height",
                                "resample.height_pct_desc",
                            ))
                            .changed();
                        if cx {
                            st.set_percent(px, if st.keep_aspect { px } else { py });
                        } else if cy {
                            if st.keep_aspect {
                                st.set_percent(py, py);
                            } else {
                                st.set_percent(px, py);
                            }
                        }
                        ui.end_row();
                        ui.label(tr("dialog.resolution"));
                        let mut dpi = st.dpi();
                        if ui
                            .add(
                                NumField::new(&mut dpi)
                                    .range(1.0..=4800.0)
                                    .fixed_decimals(0)
                                    .suffix(" dpi")
                                    .width(90.0),
                            )
                            .rich_tip(crate::ui::tips::pair(
                                "dialog.resolution",
                                "resample.dpi_desc",
                            ))
                            .changed()
                        {
                            st.set_dpi(dpi);
                        }
                        ui.end_row();
                    });
                ui.checkbox(&mut st.keep_aspect, tr("resample.keep_aspect"))
                    .rich_tip(crate::ui::tips::pair(
                        "resample.keep_aspect",
                        "dialog.keep_aspect_desc",
                    ));
                ui.horizontal(|ui| {
                    ui.label(tr("resample.method"));
                    egui::ComboBox::from_id_salt("resample_method")
                        .selected_text(tr(st.method.key()))
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            for m in crate::resample::Method::ALL {
                                ui.selectable_value(&mut st.method, m, tr(m.key()))
                                    .rich_tip_with(|| {
                                        crate::ui::tips::pair(m.key(), &format!("{}_tip", m.key()))
                                    });
                            }
                        })
                        .response
                        .rich_tip(crate::ui::tips::keyed("resample.method"));
                });
                ui.label(
                    egui::RichText::new(tr(&format!("{}_tip", st.method.key())))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(trf(
                        "resample.sizes",
                        &[
                            ("ow", &st.orig.0.to_string()),
                            ("oh", &st.orig.1.to_string()),
                            ("w", &st.w.to_string()),
                            ("h", &st.h.to_string()),
                            ("mb", &format!("{:.1}", st.bytes() as f64 / 1_048_576.0)),
                        ],
                    ))
                    .color(Tokens::text_dim())
                    .size(11.0),
                );
                apply = ok_cancel(ui, &mut close);
            });
            if apply {
                let st = st.clone();
                app.resample_bitmaps(&st);
            }
        }
        Dialog::InflateBitmap(st) => {
            window(ctx, tr("dialog.inflate_bitmap")).show(ctx, |ui| {
                inflate_fields(ui, st);
                if ok_cancel(ui, &mut close) {
                    app.inflate_bitmaps_by(st.pw, st.ph);
                }
            });
        }
        Dialog::Trace(st) => trace_dialog(app, ctx, st, &mut close),
        Dialog::NewFromTemplate(st) => crate::ui::template_dialog::dialog(app, ctx, st, &mut close),
        Dialog::Effect(st) => crate::ui::effect_dialog::effect_dialog(app, ctx, st, &mut close),
        Dialog::TextTabs | Dialog::TextColumns | Dialog::TextBullets | Dialog::TextDropCap => {
            paragraph_dialog(app, ctx, &dialog, &mut close)
        }
        Dialog::ChangeCase(mode) => {
            let title = tr("menu.text.change_case");
            let title = title.trim_end_matches(['.', '\u{2026}']).to_string();
            let mut apply = false;
            window(ctx, title).show(ctx, |ui| {
                use crate::ops2::CaseMode;
                for (key, m) in [
                    ("menu.text.case_sentence", CaseMode::Sentence),
                    ("menu.text.case_lower", CaseMode::Lower),
                    ("menu.text.case_upper", CaseMode::Upper),
                    ("menu.text.case_title", CaseMode::Title),
                    ("menu.text.case_toggle", CaseMode::Toggle),
                ] {
                    ui.radio_value(mode, m, tr(key))
                        .rich_tip_with(|| crate::ui::tips::keyed(key));
                }
                ui.add_space(8.0);
                apply = ok_cancel(ui, &mut close);
            });
            if apply {
                app.change_case(*mode);
            }
        }
        Dialog::TextStatistics => {
            window(ctx, tr("dialog.text_statistics")).show(ctx, |ui| {
                let shapes = if app.selection.is_empty() {
                    app.doc()
                        .page(app.page)
                        .map(|p| {
                            p.layers
                                .iter()
                                .flat_map(|l| l.shapes.clone())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                } else {
                    app.selected_shapes()
                };
                let (mut objects, mut chars, mut words, mut lines, mut fonts) =
                    (0, 0, 0, 0, std::collections::BTreeSet::new());
                for s in &shapes {
                    if let ShapeKind::Text { spans, .. } = &s.kind {
                        objects += 1;
                        let t: String = spans.iter().map(|x| x.text.as_str()).collect();
                        chars += t.chars().count();
                        words += t.split_whitespace().count();
                        lines += t.lines().count().max(1);
                        for sp in spans {
                            fonts.insert(sp.font_family.clone());
                        }
                    }
                }
                ui.label(trf("dialog.stats_objects", &[("n", &objects.to_string())]));
                ui.label(trf("dialog.stats_lines", &[("n", &lines.to_string())]));
                ui.label(trf("dialog.stats_words", &[("n", &words.to_string())]));
                ui.label(trf("dialog.stats_chars", &[("n", &chars.to_string())]));
                ui.label(trf(
                    "dialog.stats_fonts",
                    &[("n", &fonts.iter().cloned().collect::<Vec<_>>().join(", "))],
                ));
                if crate::ui::chrome::button_row(ui, &[&tr("dialog.close")], false).is_some() {
                    close = true;
                }
            });
        }
        Dialog::WritingTools(st) => crate::ui::writing_tools::dialog(app, ctx, st, &mut close),
        Dialog::CustomHyphens(st) => {
            crate::ui::writing_tools::custom_hyphens_dialog(app, ctx, st, &mut close)
        }
        Dialog::Encode(st) => encode_dialog(app, ctx, st, &mut close),
        Dialog::BorderGrommet(st) => border_grommet_dialog(app, ctx, st, &mut close),
        Dialog::ColorManagement(st) => {
            crate::ui::color_management::dialog(app, ctx, st, &mut close)
        }
        Dialog::FontManager(st) => font_manager_dialog(app, ctx, st, &mut close),
        Dialog::PaletteEditor(st) => palette_editor_dialog(app, ctx, st, &mut close),
        Dialog::NewDocument(st) => {
            crate::new_document::new_document_dialog(app, ctx, st, &mut close)
        }
        Dialog::PageNumberSettings => {
            window(ctx, tr("dialog.page_number_settings")).show(ctx, |ui| {
                let pn = &mut app.page_numbers;
                egui::Grid::new("page_number_settings")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label(tr("dialog.start_at"));
                        ui.add(
                            crate::ui::field::NumField::new(&mut pn.start_at).range(-999..=9999),
                        )
                        .rich_tip(crate::ui::tips::pair(
                            "dialog.start_at",
                            "dialog.pn_start_desc",
                        ));
                        ui.end_row();
                        ui.label(tr("dialog.prefix"));
                        ui.text_edit_singleline(&mut pn.prefix)
                            .rich_tip(crate::ui::tips::pair(
                                "dialog.prefix",
                                "dialog.pn_prefix_desc",
                            ));
                        ui.end_row();
                        ui.label(tr("dialog.suffix"));
                        ui.text_edit_singleline(&mut pn.suffix)
                            .rich_tip(crate::ui::tips::pair(
                                "dialog.suffix",
                                "dialog.pn_suffix_desc",
                            ));
                        ui.end_row();
                        ui.label(tr("dialog.number_style"));
                        let styles = [
                            tr("dialog.style_arabic"),
                            tr("dialog.style_roman_upper"),
                            tr("dialog.style_roman_lower"),
                            tr("dialog.style_alpha_upper"),
                            tr("dialog.style_alpha_lower"),
                        ];
                        egui::ComboBox::from_id_salt("pn_style")
                            .selected_text(styles[(pn.style as usize).min(4)].clone())
                            .show_ui(ui, |ui| {
                                for (i, s) in styles.iter().enumerate() {
                                    ui.selectable_value(&mut pn.style, i as u8, s);
                                }
                            })
                            .response
                            .rich_tip(crate::ui::tips::pair(
                                "dialog.number_style",
                                "dialog.pn_style_desc",
                            ));
                        ui.end_row();
                        ui.label(tr("dialog.page_number_position"));
                        let positions = [
                            tr("dialog.pos_bottom_center"),
                            tr("dialog.pos_bottom_outer"),
                            tr("dialog.pos_top_center"),
                            tr("dialog.pos_top_outer"),
                        ];
                        egui::ComboBox::from_id_salt("pn_pos")
                            .selected_text(positions[(pn.position as usize).min(3)].clone())
                            .show_ui(ui, |ui| {
                                for (i, s) in positions.iter().enumerate() {
                                    ui.selectable_value(&mut pn.position, i as u8, s);
                                }
                            })
                            .response
                            .rich_tip(crate::ui::tips::pair(
                                "dialog.page_number_position",
                                "dialog.pn_position_desc",
                            ));
                        ui.end_row();
                        ui.label(tr("dialog.size"));
                        ui.add(
                            crate::ui::field::NumField::new(&mut pn.size_pt)
                                .range(4.0..=200.0)
                                .suffix(" pt"),
                        )
                        .rich_tip(crate::ui::tips::pair("dialog.size", "dialog.pn_size_desc"));
                        ui.end_row();
                    });
                ui.label(
                    egui::RichText::new(format!("{}  {}", pn.label(0), pn.label(1)))
                        .color(Tokens::text_dim()),
                );
                if crate::ui::chrome::button_row(ui, &[&tr("dialog.close")], false).is_some() {
                    close = true;
                }
            });
        }
        Dialog::About => crate::ui::about::about_dialog(app, ctx, &mut close),
        Dialog::WebNotice => {
            let text = format!("{}\n\n{}", tr("web.notice_title"), tr("web.notice_text"));
            match crate::ui::chrome::message_box(
                ctx,
                crate::ui::chrome::MessageIcon::Info,
                &text,
                &[&tr("dialog.ok"), &tr("web.notice_download")],
            ) {
                Some(1) => {
                    app.open_url(crate::updates::RELEASES_PAGE);
                    close = true;
                }
                Some(_) => close = true,
                None => {}
            }
        }
        Dialog::MacrosDesktopOnly => {
            let text = format!(
                "{}\n\n{}",
                tr("macros.desktop_only"),
                tr("macros.desktop_only_text")
            );
            match crate::ui::chrome::message_box(
                ctx,
                crate::ui::chrome::MessageIcon::Info,
                &text,
                &[&tr("dialog.ok"), &tr("web.notice_download")],
            ) {
                Some(1) => {
                    app.open_url(crate::updates::RELEASES_PAGE);
                    close = true;
                }
                Some(_) => close = true,
                None => {}
            }
        }
    }
    // A dialog's close button closes it like Esc.
    if crate::ui::chrome::take_close_request(ctx) {
        close = true;
    }
    if close {
        app.end_edit_fill_preview();
        app.end_outline_pen_preview();
    }
    // A closing dialog may have opened the next one (the next unsaved
    // drawing's question); keep that.
    if !close {
        app.dialog = dialog;
    }
}

/// Inches to millimetres.
fn inch(w: f64, h: f64) -> Size {
    Size::new(w * 25.4, h * 25.4)
}

/// Screen pixels (96 per inch) to millimetres.
fn px(w: f64, h: f64) -> Size {
    Size::new(w * 25.4 / 96.0, h * 25.4 / 96.0)
}

/// Centimetres to millimetres.
fn cm(w: f64, h: f64) -> Size {
    Size::new(w * 10.0, h * 10.0)
}

/// The sizes listed first in every page size list, the most used ones.
pub const FAVOURITE_PAPERS: [&str; 5] = ["A4", "Letter", "A5", "A3", "Legal"];

/// The page size presets under their group headings (i18n keys), in the
/// order the page size lists show them. No two presets share a size in
/// either orientation, so a page's size always finds one name. The names
/// of the presets that came first (A4 ... Square 1080) are kept: saved
/// New Document settings refer to them.
pub fn paper_preset_groups() -> Vec<(&'static str, Vec<(&'static str, Size)>)> {
    use tracedraw_core::document::paper;
    vec![
        (
            "paper.group_iso_a",
            vec![
                ("A0", Size::new(841.0, 1189.0)),
                ("A1", Size::new(594.0, 841.0)),
                ("A2", Size::new(420.0, 594.0)),
                ("A3", paper::A3),
                ("A4", paper::A4),
                ("A5", Size::new(148.0, 210.0)),
                ("A6", Size::new(105.0, 148.0)),
                ("A7", Size::new(74.0, 105.0)),
                ("RA2", Size::new(430.0, 610.0)),
                ("RA3", Size::new(305.0, 430.0)),
                ("RA4", Size::new(215.0, 305.0)),
                ("SRA2", Size::new(450.0, 640.0)),
                ("SRA3", Size::new(320.0, 450.0)),
                ("SRA4", Size::new(225.0, 320.0)),
                ("F4", Size::new(210.0, 330.0)),
            ],
        ),
        (
            "paper.group_iso_b",
            vec![
                ("B0", Size::new(1000.0, 1414.0)),
                ("B1", Size::new(707.0, 1000.0)),
                ("B2", Size::new(500.0, 707.0)),
                ("B3", Size::new(353.0, 500.0)),
                ("B4", Size::new(250.0, 353.0)),
                ("B5", Size::new(176.0, 250.0)),
                ("B6", Size::new(125.0, 176.0)),
                ("B0 (JIS)", Size::new(1030.0, 1456.0)),
                ("B1 (JIS)", Size::new(728.0, 1030.0)),
                ("B2 (JIS)", Size::new(515.0, 728.0)),
                ("B3 (JIS)", Size::new(364.0, 515.0)),
                ("B4 (JIS)", Size::new(257.0, 364.0)),
                ("B5 (JIS)", Size::new(182.0, 257.0)),
                ("B6 (JIS)", Size::new(128.0, 182.0)),
            ],
        ),
        (
            "paper.group_north_america",
            vec![
                ("Letter", paper::LETTER),
                ("Legal", Size::new(215.9, 355.6)),
                ("Tabloid", Size::new(279.4, 431.8)),
                ("Executive", inch(7.25, 10.5)),
                ("Half Letter", inch(5.5, 8.5)),
                ("Government Letter", inch(8.0, 10.5)),
                ("ANSI C", inch(17.0, 22.0)),
                ("ANSI D", inch(22.0, 34.0)),
                ("ANSI E", inch(34.0, 44.0)),
                ("Arch A", inch(9.0, 12.0)),
                ("Arch B", inch(12.0, 18.0)),
                ("Arch C", inch(18.0, 24.0)),
                ("Arch D", inch(24.0, 36.0)),
                ("Fanfold", inch(14.875, 11.0)),
                ("German Fanfold", inch(8.5, 12.0)),
            ],
        ),
        (
            "paper.group_print_shop",
            vec![
                ("Ofício 2", Size::new(216.0, 330.0)),
                ("2A 64x88 cm", cm(64.0, 88.0)),
                ("BB 66x96 cm", cm(66.0, 96.0)),
                ("AA 76x112 cm", cm(76.0, 112.0)),
                ("Flyer 15x21 cm", cm(15.0, 21.0)),
                ("Flyer 10x14 cm", cm(10.0, 14.0)),
                ("Folder 30x21 cm", cm(30.0, 21.0)),
                ("Menu 23x33 cm", cm(23.0, 33.0)),
                ("Sticker 5x5 cm", cm(5.0, 5.0)),
                ("Sticker 10x10 cm", cm(10.0, 10.0)),
            ],
        ),
        (
            "paper.group_posters",
            vec![
                ("Poster 30x40 cm", cm(30.0, 40.0)),
                ("Poster 50x70 cm", cm(50.0, 70.0)),
                ("Poster 70x100 cm", cm(70.0, 100.0)),
                ("Banner 60x90 cm", cm(60.0, 90.0)),
                ("Banner 90x120 cm", cm(90.0, 120.0)),
                ("Banner 100x200 cm", cm(100.0, 200.0)),
                ("Roll-up 85x200 cm", cm(85.0, 200.0)),
            ],
        ),
        (
            "paper.group_envelopes",
            vec![
                ("DL", Size::new(110.0, 220.0)),
                ("C3", Size::new(324.0, 458.0)),
                ("C4", Size::new(229.0, 324.0)),
                ("C5", Size::new(162.0, 229.0)),
                ("C6", Size::new(114.0, 162.0)),
                ("Envelope #9", inch(3.875, 8.875)),
                ("Envelope #10", inch(4.125, 9.5)),
                ("Envelope #11", inch(4.5, 10.375)),
                ("Envelope #12", inch(4.75, 11.0)),
                ("Envelope #14", inch(5.0, 11.5)),
                ("Envelope Monarch", inch(3.875, 7.5)),
                ("Envelope 11.4x22.9 cm", cm(11.4, 22.9)),
                ("Envelope 24x34 cm", cm(24.0, 34.0)),
            ],
        ),
        (
            "paper.group_cards",
            vec![
                ("Business card", Size::new(90.0, 50.0)),
                ("Business card 85x55 mm", Size::new(85.0, 55.0)),
                ("Business card 3.5x2 in", inch(3.5, 2.0)),
                ("Business card 91x55 mm", Size::new(91.0, 55.0)),
                ("ID card", Size::new(85.6, 53.98)),
                ("Postcard 4.25x6 in", inch(4.25, 6.0)),
                ("Postcard 6x9 in", inch(6.0, 9.0)),
                ("Postcard 100x148 mm", Size::new(100.0, 148.0)),
                ("Double postcard 200x148 mm", Size::new(200.0, 148.0)),
            ],
        ),
        (
            "paper.group_photo",
            vec![
                ("Photo 3.5x5 in", inch(3.5, 5.0)),
                ("Photo 4x6 in", inch(4.0, 6.0)),
                ("Photo 5x7 in", inch(5.0, 7.0)),
                ("Photo 8x10 in", inch(8.0, 10.0)),
                ("Photo 10x15 cm", cm(10.0, 15.0)),
                ("Photo 13x18 cm", cm(13.0, 18.0)),
                ("Photo 20x30 cm", cm(20.0, 30.0)),
            ],
        ),
        (
            "paper.group_screen",
            vec![
                ("Web 1280x720", px(1280.0, 720.0)),
                ("Web 1366x768", px(1366.0, 768.0)),
                ("Web 1920x1080", px(1920.0, 1080.0)),
                ("Web 2560x1440", px(2560.0, 1440.0)),
                ("4K 3840x2160", px(3840.0, 2160.0)),
                ("Web page 960x960", px(960.0, 960.0)),
                ("Square 1080", px(1080.0, 1080.0)),
                ("Social post 1080x1350", px(1080.0, 1350.0)),
                ("Social cover 820x312", px(820.0, 312.0)),
                ("Social banner 1500x500", px(1500.0, 500.0)),
                ("Slide 16:9", inch(10.0, 5.625)),
                ("Slide 4:3", inch(10.0, 7.5)),
            ],
        ),
        (
            "paper.group_web_ads",
            vec![
                ("Ad 120x60", px(120.0, 60.0)),
                ("Ad 120x90", px(120.0, 90.0)),
                ("Ad 125x125", px(125.0, 125.0)),
                ("Ad 250x250", px(250.0, 250.0)),
                ("Ad 468x60", px(468.0, 60.0)),
                ("Skyscraper 120x600", px(120.0, 600.0)),
                ("Wide skyscraper 160x600", px(160.0, 600.0)),
                ("Medium rectangle 300x250", px(300.0, 250.0)),
                ("Large rectangle 336x280", px(336.0, 280.0)),
                ("Leaderboard 728x90", px(728.0, 90.0)),
                ("Billboard 970x250", px(970.0, 250.0)),
            ],
        ),
    ]
}

/// Every built-in page size preset, in list order.
pub fn paper_presets() -> Vec<(&'static str, Size)> {
    paper_preset_groups()
        .into_iter()
        .flat_map(|(_, sizes)| sizes)
        .collect()
}

/// The sizes are the same in either orientation (to 0.05 mm).
fn same_size(a: Size, b: Size) -> bool {
    let close = |x: f64, y: f64| (x - y).abs() < 0.05;
    (close(a.width, b.width) && close(a.height, b.height))
        || (close(a.width, b.height) && close(a.height, b.width))
}

/// The name of the preset a page of `size` matches, the user's saved
/// sizes first, or None (a custom size).
pub fn paper_name(size: Size, saved: &[crate::settings::PageSizePreset]) -> Option<String> {
    saved
        .iter()
        .find(|p| same_size(Size::new(p.width_mm, p.height_mm), size))
        .map(|p| p.name.clone())
        .or_else(|| {
            paper_presets()
                .into_iter()
                .find(|(_, s)| same_size(*s, size))
                .map(|(n, _)| n.to_string())
        })
}

/// The page size presets as a dropdown's rows: the favourites, the
/// user's saved sizes, then every group under its heading; `ticked`
/// names the row shown selected. Returns the clicked one.
pub fn paper_preset_menu(
    ui: &mut Ui,
    ticked: &str,
    saved: &[crate::settings::PageSizePreset],
) -> Option<(String, Size)> {
    let mut pick = None;
    let groups = paper_preset_groups();
    let heading = |ui: &mut Ui, key: &str| {
        ui.label(
            egui::RichText::new(tr(key))
                .small()
                .color(Tokens::text_dim()),
        );
    };
    let mut row = |ui: &mut Ui, n: &str, s: Size| {
        if ui.selectable_label(ticked == n, n).clicked() {
            pick = Some((n.to_string(), s));
        }
    };
    for fav in FAVOURITE_PAPERS {
        if let Some(s) = groups
            .iter()
            .flat_map(|(_, sizes)| sizes)
            .find(|(n, _)| *n == fav)
            .map(|(_, s)| *s)
        {
            row(ui, fav, s);
        }
    }
    if !saved.is_empty() {
        ui.separator();
        heading(ui, "paper.group_saved");
        for p in saved {
            row(ui, &p.name, Size::new(p.width_mm, p.height_mm));
        }
    }
    for (group, sizes) in &groups {
        ui.separator();
        heading(ui, group);
        for (n, s) in sizes {
            row(ui, n, *s);
        }
    }
    pick
}

/// A QR code as one even-odd path of module squares.
pub(crate) fn qr_path(text: &str, size_mm: f64) -> Result<tracedraw_core::BezPath, String> {
    use tracedraw_core::geometry::Shape as _;
    if text.trim().is_empty() {
        return Err(tr("dialog.qr_empty"));
    }
    let code = qrcode::QrCode::new(text.as_bytes()).map_err(|e| e.to_string())?;
    let n = code.width();
    let module = size_mm / n as f64;
    let mut path = tracedraw_core::BezPath::new();
    for y in 0..n {
        for x in 0..n {
            if code[(x, y)] == qrcode::Color::Dark {
                // Y up: row 0 at the top.
                let r = Rect::new(
                    x as f64 * module,
                    size_mm - (y as f64 + 1.0) * module,
                    (x as f64 + 1.0) * module,
                    size_mm - y as f64 * module,
                );
                path.extend(r.to_path(0.01));
            }
        }
    }
    Ok(path)
}

// ----- Options ---------------------------------------------------------------

/// Label sheet presets: (name, sheet size, columns, rows, label size,
/// left/top margin, horizontal/vertical pitch), all in millimetres.
const LABEL_PRESETS: [(&str, Size, u32, u32, Size, f64, f64, f64, f64); 4] = [
    (
        "A4 3 x 7 (63.5 x 38.1 mm)",
        Size::new(210.0, 297.0),
        3,
        7,
        Size::new(63.5, 38.1),
        7.2,
        15.1,
        66.0,
        38.1,
    ),
    (
        "A4 2 x 4 (99.1 x 67.7 mm)",
        Size::new(210.0, 297.0),
        2,
        4,
        Size::new(99.1, 67.7),
        4.7,
        13.1,
        101.6,
        67.7,
    ),
    (
        "A4 2 x 8 (99.1 x 33.9 mm)",
        Size::new(210.0, 297.0),
        2,
        8,
        Size::new(99.1, 33.9),
        4.7,
        12.9,
        101.6,
        33.9,
    ),
    (
        "Letter 3 x 10 (66.7 x 25.4 mm)",
        Size::new(215.9, 279.4),
        3,
        10,
        Size::new(66.7, 25.4),
        4.8,
        12.7,
        69.9,
        25.4,
    ),
];

/// Saves the page size under `name` (replacing a saved size of that
/// name) so every page size list offers it.
pub(crate) fn save_page_size(app: &mut App, name: &str, size: Size) {
    let preset = crate::settings::PageSizePreset {
        name: name.to_string(),
        width_mm: size.width,
        height_mm: size.height,
    };
    match app.settings.page_sizes.iter_mut().find(|p| p.name == name) {
        Some(p) => *p = preset,
        None => app.settings.page_sizes.push(preset),
    }
    app.save_settings();
}

/// Forgets the saved page size `name`.
pub(crate) fn delete_page_size(app: &mut App, name: &str) {
    app.settings.page_sizes.retain(|p| p.name != name);
    app.save_settings();
}

/// Document Options > Page Size, laid out.
pub(crate) fn page_size_page(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.radio_value(
            &mut app.options_labels,
            false,
            tr("options.page_size_radio"),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.page_size_radio",
            "options.page_size_radio_tip",
        ));
        ui.radio_value(&mut app.options_labels, true, tr("options.label_presets"))
            .rich_tip(crate::ui::tips::pair(
                "options.label_presets",
                "options.label_presets_tip",
            ));
    });
    ui.add_space(4.0);
    if app.options_labels {
        ui.label(tr("options.size_and_orientation"));
        ui.separator();
        ui.label(
            egui::RichText::new(tr("options.label_hint"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
        ui.add_space(4.0);
        egui::ComboBox::from_id_salt("label_preset")
            .selected_text(LABEL_PRESETS[app.options_label_index.min(3)].0)
            .width(260.0)
            .show_ui(ui, |ui| {
                for (i, p) in LABEL_PRESETS.iter().enumerate() {
                    ui.selectable_value(&mut app.options_label_index, i, p.0);
                }
            })
            .response
            .rich_tip(crate::ui::tips::pair(
                "options.label_presets",
                "options.label_preset_tip",
            ));
        if ui
            .button(tr("options.apply_labels"))
            .rich_tip(crate::ui::tips::pair(
                "options.apply_labels",
                "options.apply_labels_tip",
            ))
            .clicked()
        {
            let (_, sheet, cols, rows, label, left, top, px, py) =
                LABEL_PRESETS[app.options_label_index.min(3)];
            let page = app.page;
            app.run(Command::ResizePage { page, size: sheet });
            for c in 0..cols {
                let x0 = left + c as f64 * px;
                app.add_guide(tracedraw_core::document::Guide::vertical(x0));
                app.add_guide(tracedraw_core::document::Guide::vertical(x0 + label.width));
            }
            for r in 0..rows {
                let y1 = sheet.height - (top + r as f64 * py);
                app.add_guide(tracedraw_core::document::Guide::horizontal(y1));
                app.add_guide(tracedraw_core::document::Guide::horizontal(
                    y1 - label.height,
                ));
            }
            app.fit_pending = true;
        }
        return;
    }
    ui.label(tr("options.size_and_orientation"));
    ui.separator();
    let s = app.page_size();
    let (mut w, mut h) = (s.width, s.height);
    let mut changed = false;
    let saved = app.settings.page_sizes.clone();
    let named = paper_name(s, &saved);
    let current = named.clone().unwrap_or_else(|| tr("options.custom_size"));
    egui::Grid::new("page_size_grid")
        .num_columns(3)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(tr("options.size"));
            });
            egui::ComboBox::from_id_salt("page_size_preset")
                .selected_text(current.clone())
                .width(230.0)
                .height(400.0)
                .show_ui(ui, |ui| {
                    if let Some((_, ps)) = paper_preset_menu(ui, &current, &saved) {
                        let landscape = w > h;
                        if landscape {
                            w = ps.width.max(ps.height);
                            h = ps.width.min(ps.height);
                        } else {
                            w = ps.width;
                            h = ps.height;
                        }
                        changed = true;
                    }
                })
                .response
                .rich_tip(crate::ui::tips::pair("options.size", "options.size_tip"));
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(named.is_none(), egui::Button::new("\u{1f4be}"))
                    .rich_tip_always(crate::ui::tips::pair(
                        "options.save_page_size",
                        "options.save_page_size_tip",
                    ))
                    .clicked()
                {
                    app.options_page_size_name = Some(String::new());
                }
                let user_size = saved.iter().any(|p| Some(&p.name) == named.as_ref());
                if ui
                    .add_enabled(user_size, egui::Button::new("\u{1f5d1}"))
                    .rich_tip_always(crate::ui::tips::pair(
                        "options.delete_page_size",
                        "options.delete_page_size_tip",
                    ))
                    .clicked()
                {
                    if let Some(n) = &named {
                        delete_page_size(app, n);
                    }
                }
            });
            ui.end_row();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(tr("dialog.width"));
            });
            let mut wv = app.units.from_mm(w);
            if ui
                .add_sized(
                    [100.0, 22.0],
                    crate::ui::field::NumField::new(&mut wv)
                        .speed(0.5)
                        .max_decimals(3),
                )
                .rich_tip(crate::ui::tips::pair(
                    "dialog.width",
                    "dialog.page_width_desc",
                ))
                .changed()
            {
                w = app.units.to_mm(wv);
                changed = true;
            }
            egui::ComboBox::from_id_salt("page_units")
                .selected_text(app.units.label())
                .width(120.0)
                .show_ui(ui, |ui| {
                    for u in Units::ALL {
                        if ui.selectable_label(app.units == u, u.label()).clicked() {
                            app.units = u;
                        }
                    }
                })
                .response
                .rich_tip(crate::ui::tips::pair(
                    "options.page_units",
                    "options.page_units_tip",
                ));
            ui.end_row();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(tr("dialog.height"));
            });
            let mut hv = app.units.from_mm(h);
            if ui
                .add_sized(
                    [100.0, 22.0],
                    crate::ui::field::NumField::new(&mut hv)
                        .speed(0.5)
                        .max_decimals(3),
                )
                .rich_tip(crate::ui::tips::pair(
                    "dialog.height",
                    "dialog.page_height_desc",
                ))
                .changed()
            {
                h = app.units.to_mm(hv);
                changed = true;
            }
            ui.horizontal(|ui| {
                let portrait = h >= w;
                if ui
                    .add(egui::Button::new("\u{25af}").selected(portrait))
                    .rich_tip(crate::ui::tips::pair(
                        "options.portrait",
                        "options.portrait_tip",
                    ))
                    .clicked()
                    && !portrait
                {
                    std::mem::swap(&mut w, &mut h);
                    changed = true;
                }
                if ui
                    .add(egui::Button::new("\u{25ad}").selected(!portrait))
                    .rich_tip(crate::ui::tips::pair(
                        "options.landscape",
                        "options.landscape_tip",
                    ))
                    .clicked()
                    && portrait
                {
                    std::mem::swap(&mut w, &mut h);
                    changed = true;
                }
            });
            ui.end_row();
        });
    if let Some(mut name) = app.options_page_size_name.take() {
        let mut keep = true;
        ui.horizontal(|ui| {
            ui.label(tr("options.page_size_name"));
            let r = ui
                .add(egui::TextEdit::singleline(&mut name).desired_width(180.0))
                .rich_tip(crate::ui::tips::pair(
                    "options.page_size_name",
                    "options.page_size_name_tip",
                ));
            if name.is_empty() && !r.has_focus() {
                r.request_focus();
            }
            let enter =
                (r.lost_focus() || r.has_focus()) && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui
                .button(tr("dialog.ok"))
                .rich_tip(crate::ui::tips::keyed("dialog.ok"))
                .clicked()
                || enter)
                && !name.trim().is_empty()
            {
                save_page_size(app, name.trim(), Size::new(w, h));
                keep = false;
            }
            if ui
                .button(tr("dialog.cancel"))
                .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                .clicked()
            {
                keep = false;
            }
        });
        if keep {
            app.options_page_size_name = Some(name);
        }
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(tr("options.fit_page"));
        if ui
            .add_enabled(
                !app.selection.is_empty(),
                egui::Button::new(tr("options.fit_page_selection")),
            )
            .rich_tip_always(crate::ui::tips::pair(
                "options.fit_page_selection",
                "options.fit_page_selection_tip",
            ))
            .clicked()
        {
            app.fit_page_to_selection();
        }
        if ui
            .add_enabled(
                app.drawing_bounds().is_some(),
                egui::Button::new(tr("options.fit_page_drawing")),
            )
            .rich_tip_always(crate::ui::tips::pair(
                "options.fit_page_drawing",
                "options.fit_page_drawing_tip",
            ))
            .clicked()
        {
            app.fit_page_to_drawing();
        }
    });
    ui.add_space(4.0);
    ui.indent("page_size_checks", |ui| {
        ui.checkbox(
            &mut app.options_current_only,
            tr("options.apply_current_only"),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.apply_current_only",
            "options.apply_current_only_tip",
        ));
        ui.checkbox(&mut app.show_page_border, tr("options.show_page_border"))
            .rich_tip(crate::ui::tips::pair(
                "options.show_page_border",
                "options.show_page_border_tip",
            ));
        ui.add_space(6.0);
        if ui
            .add_sized(
                [150.0, 26.0],
                egui::Button::new(tr("options.add_page_frame")),
            )
            .rich_tip(crate::ui::tips::pair(
                "options.add_page_frame",
                "options.add_page_frame_tip",
            ))
            .clicked()
        {
            let r = app.page_rect();
            if let Some(id) = app.new_shape(ShapeKind::Rect {
                rect: r,
                radius: 0.0,
                corners: None,
            }) {
                app.select(vec![id]);
            }
        }
    });
    if changed {
        let size = Size::new(w.max(1.0), h.max(1.0));
        let pages: Vec<tracedraw_core::PageId> = if app.options_current_only {
            vec![app.page]
        } else {
            app.doc().pages.iter().map(|p| p.id).collect()
        };
        let cmds: Vec<Command> = pages
            .into_iter()
            .map(|page| Command::ResizePage { page, size })
            .collect();
        let _ = app.engine.run_batch("Page Size", &cmds);
        app.fit_pending = true;
    }
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(tr("options.rendering_resolution"));
        let mut md = app.doc().metadata.clone();
        let mut dpi = md.resolution_dpi;
        egui::ComboBox::from_id_salt("render_dpi")
            .selected_text(format!("{}", dpi.round() as i64))
            .width(70.0)
            .show_ui(ui, |ui| {
                for d in [72.0, 96.0, 150.0, 200.0, 300.0, 600.0] {
                    if ui
                        .selectable_label((dpi - d).abs() < 0.5, format!("{}", d as i64))
                        .clicked()
                    {
                        dpi = d;
                    }
                }
            })
            .response
            .rich_tip(crate::ui::tips::pair(
                "options.rendering_resolution",
                "options.rendering_resolution_tip",
            ));
        ui.label("dpi");
        if (dpi - md.resolution_dpi).abs() > 0.5 {
            md.resolution_dpi = dpi;
            app.run(Command::SetMetadata { metadata: md });
        }
    });
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(tr("options.bleed"));
        let mut md = app.doc().metadata.clone();
        let mut b = app.units.from_mm(md.bleed);
        if ui
            .add_sized(
                [100.0, 22.0],
                crate::ui::field::NumField::new(&mut b)
                    .speed(0.1)
                    .max_decimals(2),
            )
            .rich_tip(crate::ui::tips::pair("options.bleed", "options.bleed_tip"))
            .changed()
        {
            md.bleed = app.units.to_mm(b).max(0.0);
            app.run(Command::SetMetadata { metadata: md });
        }
    });
    ui.indent("bleed_checks", |ui| {
        ui.add_space(2.0);
        ui.checkbox(&mut app.show_bleed, tr("options.show_bleed"))
            .rich_tip(crate::ui::tips::pair(
                "options.show_bleed",
                "options.show_bleed_tip",
            ));
    });
}

// ----- Export ------------------------------------------------------------------

/// The export preview, cached until the drawing or the export settings
/// change, with the estimated file size.
#[derive(Clone)]
struct ExportPreview {
    key: u64,
    texture: Option<egui::TextureHandle>,
    size: (u32, u32),
    bytes: Option<usize>,
}

/// What the export preview depends on.
fn export_preview_key(app: &App, st: &ExportState) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    app.engine.revision().hash(&mut h);
    app.page.raw().hash(&mut h);
    st.format.hash(&mut h);
    st.transparent.hash(&mut h);
    st.selection_only.hash(&mut h);
    st.all_pages.hash(&mut h);
    st.quality.hash(&mut h);
    st.dpi.to_bits().hash(&mut h);
    st.scale_or_one().to_bits().hash(&mut h);
    st.webp_lossless.hash(&mut h);
    if st.selection_only {
        for id in &app.selection {
            id.raw().hash(&mut h);
        }
    }
    h.finish()
}

/// Draw the preview box (`size` points) and return the estimated size.
fn export_preview_box(app: &App, ui: &mut Ui, st: &ExportState, size: egui::Vec2) -> Option<usize> {
    let id = egui::Id::new("export_preview");
    let key = export_preview_key(app, st);
    let cached = ui
        .ctx()
        .data(|d| d.get_temp::<ExportPreview>(id))
        .filter(|p| p.key == key);
    let preview = match cached {
        Some(p) => p,
        None => {
            let px = (size.x.max(size.y) * ui.ctx().pixels_per_point()).round() as u32;
            let pm = crate::export::preview(app, st, px);
            let bytes = pm
                .as_ref()
                .and_then(|pm| crate::export::estimated_size(app, st, pm));
            let texture = pm.as_ref().map(|pm| {
                let pixels: Vec<egui::Color32> = pm
                    .pixels()
                    .iter()
                    .map(|p| {
                        egui::Color32::from_rgba_premultiplied(
                            p.red(),
                            p.green(),
                            p.blue(),
                            p.alpha(),
                        )
                    })
                    .collect();
                let image = egui::ColorImage {
                    size: [pm.width() as usize, pm.height() as usize],
                    source_size: egui::vec2(pm.width() as f32, pm.height() as f32),
                    pixels,
                };
                ui.ctx()
                    .load_texture("export_preview", image, egui::TextureOptions::LINEAR)
            });
            let p = ExportPreview {
                key,
                texture,
                size: pm
                    .as_ref()
                    .map(|pm| (pm.width(), pm.height()))
                    .unwrap_or((1, 1)),
                bytes,
            };
            ui.ctx().data_mut(|d| d.insert_temp(id, p.clone()));
            p
        }
    };
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, crate::theme::Tokens::panel_dark());
    if let Some(tex) = &preview.texture {
        let (w, h) = (preview.size.0.max(1) as f32, preview.size.1.max(1) as f32);
        let scale = ((rect.width() - 16.0) / w).min((rect.height() - 16.0) / h);
        let img = egui::Rect::from_center_size(rect.center(), egui::vec2(w * scale, h * scale));
        // A checkerboard behind a transparent export.
        let (_, ext) = EXPORT_FORMATS[st.format.min(EXPORT_FORMATS.len() - 1)];
        if st.transparent && crate::export::keeps_alpha(ext) {
            let cell = 8.0;
            let mut y = img.top();
            let mut row = 0;
            while y < img.bottom() {
                let mut x = img.left();
                let mut col = row % 2;
                while x < img.right() {
                    let c = if col % 2 == 0 { 0xFF } else { 0xD9 };
                    let r = egui::Rect::from_min_max(
                        egui::pos2(x, y),
                        egui::pos2((x + cell).min(img.right()), (y + cell).min(img.bottom())),
                    );
                    painter.rect_filled(r, 0.0, egui::Color32::from_gray(c));
                    x += cell;
                    col += 1;
                }
                y += cell;
                row += 1;
            }
        }
        painter.add(egui::epaint::RectShape::filled(
            img.translate(egui::vec2(2.0, 3.0)),
            0.0,
            egui::Color32::from_black_alpha(40),
        ));
        painter.image(
            tex.id(),
            img,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(0xC8)),
        egui::StrokeKind::Inside,
    );
    preview.bytes
}

/// Ask for the file and export to it with `st`'s choices.
pub fn export_with_file_dialog(app: &mut App, st: &ExportState) {
    let (name, ext) = EXPORT_FORMATS[st.format.min(EXPORT_FORMATS.len() - 1)];
    let stem = app
        .file
        .as_ref()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "Graphic1".into());
    crate::files::Dialog::new()
        .add_filter(name, &[ext])
        .set_file_name(format!("{stem}.{ext}"))
        .save_file(app, |app, path| {
            match crate::export::export(app, &path, st) {
                Ok(msg) => app.status = msg,
                Err(e) => app.status = format!("{}: {e}", tr("dialog.export_failed")),
            }
        });
}

/// The Export dialog's presets (#547): a list of the saved ones of the
/// chosen format (#612), "+" to save the choices under a name, rename and
/// delete for the chosen one.
fn export_presets_row(app: &mut App, ui: &mut Ui, st: &mut ExportState) {
    // A preset of another format no longer describes the choices.
    if !st.preset.is_empty()
        && !app
            .settings
            .formats
            .export_presets
            .iter()
            .any(|p| p.name == st.preset && preset_fits(p, st))
    {
        st.preset.clear();
    }
    ui.horizontal(|ui| {
        ui.label(tr("dialog.export_preset"));
        let presets = presets_for(&app.settings.formats.export_presets, st);
        // Every preset listed is of the chosen format: its name says it.
        let label = |p: &crate::prefs::ExportPreset| p.name.clone();
        let shown = presets
            .iter()
            .find(|p| p.name == st.preset)
            .map(label)
            .unwrap_or_else(|| tr("dialog.export_preset_none"));
        egui::ComboBox::from_id_salt("export_preset")
            .selected_text(shown)
            .width(170.0)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(st.preset.is_empty(), tr("dialog.export_preset_none"))
                    .clicked()
                {
                    st.preset.clear();
                }
                for p in &presets {
                    if ui.selectable_label(p.name == st.preset, label(p)).clicked() {
                        p.apply(st);
                    }
                }
            })
            .response
            .rich_tip(crate::ui::tips::pair(
                "dialog.export_preset",
                "dialog.export_preset_desc",
            ));
        // A script filter has no presets: "+" would save under the last
        // built-in format, out of sight.
        if ui
            .add_enabled(st.script.is_none(), egui::Button::new("+").small())
            .rich_tip(crate::ui::tips::keyed("dialog.export_preset_add"))
            .clicked()
        {
            st.preset_name = Some((String::new(), false));
        }
        let chosen = !st.preset.is_empty();
        if ui
            .add_enabled(chosen, egui::Button::new("\u{270e}").small())
            .rich_tip(crate::ui::tips::keyed("dialog.export_preset_rename"))
            .clicked()
        {
            st.preset_name = Some((st.preset.clone(), true));
        }
        if ui
            .add_enabled(chosen, egui::Button::new("\u{2212}").small())
            .rich_tip(crate::ui::tips::keyed("dialog.export_preset_delete"))
            .clicked()
        {
            app.settings.formats.delete_preset(&st.preset, st.format);
            st.preset.clear();
        }
    });
    let mut done: Option<bool> = None;
    if let Some((name, _)) = st.preset_name.as_mut() {
        ui.horizontal(|ui| {
            let r = ui.add(
                egui::TextEdit::singleline(name)
                    .desired_width(170.0)
                    .hint_text(tr("dialog.export_preset_name")),
            );
            if !r.has_focus() && name.is_empty() {
                r.request_focus();
            }
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui
                .small_button(tr("dialog.ok"))
                .rich_tip(crate::ui::tips::keyed("dialog.ok"))
                .clicked()
                || enter
            {
                done = Some(true);
            }
            if ui
                .small_button(tr("dialog.cancel"))
                .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                .clicked()
            {
                done = Some(false);
            }
        });
    }
    // A script filter chosen while a name was being typed: nothing to save.
    if st.script.is_some() {
        st.preset_name = None;
        done = None;
    }
    if let Some(ok) = done {
        if let (true, Some((name, renaming))) = (ok, st.preset_name.clone()) {
            let name = name.trim().to_string();
            let saved = if renaming {
                app.settings
                    .formats
                    .rename_preset(&st.preset, &name, st.format)
            } else {
                app.settings.formats.save_preset(&name, st)
            };
            if saved {
                st.preset = name;
            }
        }
        st.preset_name = None;
    }
}

/// Whether the Export dialog lists `p`: it was saved for the format
/// chosen (a script filter has none).
fn preset_fits(p: &crate::prefs::ExportPreset, st: &ExportState) -> bool {
    st.script.is_none() && p.fits(st.format)
}

/// The presets the Export dialog lists for its chosen format.
pub(crate) fn presets_for(
    all: &[crate::prefs::ExportPreset],
    st: &ExportState,
) -> Vec<crate::prefs::ExportPreset> {
    all.iter().filter(|p| preset_fits(p, st)).cloned().collect()
}

fn export_dialog(app: &mut App, ctx: &Context, st: &mut ExportState, close: &mut bool) {
    window(
        ctx,
        if st.web {
            tr("menu.file.export_for_web")
        } else {
            tr("dialog.export")
        },
    )
    .show(ctx, |ui| {
        ui.horizontal_top(|ui| {
            let estimate = export_preview_box(app, ui, st, egui::vec2(260.0, 300.0));
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.set_width(300.0);
                export_presets_row(app, ui, st);
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.format"));
                    let scripted: Vec<crate::scripting::ScriptFilter> = app
                        .settings
                        .script_filters
                        .iter()
                        .filter(|f| f.export)
                        .cloned()
                        .collect();
                    if st
                        .script
                        .as_ref()
                        .is_some_and(|n| !scripted.iter().any(|f| &f.name == n))
                    {
                        st.script = None;
                    }
                    let shown = match &st.script {
                        Some(n) => n.clone(),
                        None => export_format_label(st.format),
                    };
                    egui::ComboBox::from_id_salt("exp_fmt")
                        .selected_text(shown)
                        .width(300.0)
                        .height(520.0)
                        .show_ui(ui, |ui| {
                            for entry in export_format_order() {
                                let Some(i) = entry else {
                                    ui.separator();
                                    continue;
                                };
                                if ui
                                    .selectable_label(
                                        st.script.is_none() && st.format == i,
                                        export_format_label(i),
                                    )
                                    .clicked()
                                    && (st.format != i || st.script.is_some())
                                {
                                    app.recall_export_format(st, i);
                                }
                            }
                            if !scripted.is_empty() {
                                ui.separator();
                            }
                            for f in &scripted {
                                let on = st.script.as_ref() == Some(&f.name);
                                if ui.selectable_label(on, f.label()).clicked() {
                                    st.script = Some(f.name.clone());
                                }
                            }
                        })
                        .response
                        .rich_tip(crate::ui::tips::pair(
                            "dialog.format",
                            "dialog.export_format_desc",
                        ));
                });
                let scripted = st.script.is_some();
                let raster = !scripted && ((2..=7).contains(&st.format) || st.format == 15);
                if raster {
                    ui.horizontal(|ui| {
                        ui.label(tr("dialog.resolution"));
                        ui.add(
                            crate::ui::field::NumField::new(&mut st.dpi)
                                .range(MIN_EXPORT_DPI..=2400.0)
                                .suffix(" dpi"),
                        )
                        .rich_tip(crate::ui::tips::pair(
                            "dialog.resolution",
                            "dialog.export_dpi_desc",
                        ));
                        for d in [72.0, 96.0, 150.0, 300.0] {
                            if ui
                                .small_button(format!("{d:.0}"))
                                .rich_tip_with(|| {
                                    format!("{d:.0} dpi\n{}", tr("dialog.export_dpi_preset_desc"))
                                })
                                .clicked()
                            {
                                st.dpi = d;
                            }
                        }
                    });
                    if st.format == 2 || st.format == 6 || st.format == 7 {
                        ui.checkbox(&mut st.transparent, tr("dialog.transparent_background"))
                            .rich_tip(crate::ui::tips::pair(
                                "dialog.transparent_background",
                                "dialog.export_transparent_desc",
                            ));
                    }
                    if st.format == 6 {
                        ui.checkbox(&mut st.webp_lossless, tr("dialog.webp_lossless"))
                            .rich_tip(crate::ui::tips::keyed("dialog.webp_lossless"));
                    }
                    if st.format == 3 || st.format == 6 {
                        ui.add(crate::ui::Rail(
                            egui::Slider::new(&mut st.quality, 1..=100).text(tr("dialog.quality")),
                        ))
                        .rich_tip(crate::ui::tips::keyed("dialog.quality"));
                    }
                    let b = if st.selection_only {
                        app.selection_bounds().unwrap_or(app.page_rect())
                    } else {
                        app.page_rect()
                    };
                    // The size in the chosen unit: typing either side sets
                    // the resolution (pixels) or the scale (lengths) that
                    // gives it, the other side following (#617).
                    let (mut w, mut h) = export_size_in_unit(b.width(), b.height(), st);
                    let unit = st.size_unit.min(EXPORT_SIZE_UNITS.len() - 1);
                    let (decimals, step) = if unit == 0 { (0, 1.0) } else { (2, 0.1) };
                    ui.horizontal(|ui| {
                        ui.label(tr("dialog.export_size"));
                        let wc = ui
                            .add(
                                crate::ui::field::NumField::new(&mut w)
                                    .range(0.01..=200_000.0)
                                    .max_decimals(decimals)
                                    .speed(step)
                                    .width(70.0),
                            )
                            .rich_tip(crate::ui::tips::pair(
                                "dialog.width",
                                "dialog.export_width_desc",
                            ))
                            .changed();
                        ui.label("x");
                        let hc = ui
                            .add(
                                crate::ui::field::NumField::new(&mut h)
                                    .range(0.01..=200_000.0)
                                    .max_decimals(decimals)
                                    .speed(step)
                                    .width(70.0),
                            )
                            .rich_tip(crate::ui::tips::pair(
                                "dialog.height",
                                "dialog.export_height_desc",
                            ))
                            .changed();
                        if wc {
                            set_export_size(st, b.width(), w);
                        } else if hc {
                            set_export_size(st, b.height(), h);
                        }
                        egui::ComboBox::from_id_salt("exp_size_unit")
                            .selected_text(EXPORT_SIZE_UNITS[unit].0)
                            .width(50.0)
                            .show_ui(ui, |ui| {
                                for (i, (name, _)) in EXPORT_SIZE_UNITS.iter().enumerate() {
                                    ui.selectable_value(&mut st.size_unit, i, *name);
                                }
                            })
                            .response
                            .rich_tip(crate::ui::tips::keyed("dialog.export_size_unit"));
                        let scale = st.scale_or_one();
                        if (scale - 1.0).abs() > 1e-9
                            && ui
                                .small_button(format!("{:.0} %", scale * 100.0))
                                .rich_tip(crate::ui::tips::keyed("dialog.export_scale_reset"))
                                .clicked()
                        {
                            st.scale = 1.0;
                        }
                    });
                    // What comes out: pixels, the resolution the file
                    // records and its colours.
                    let (pw, ph) = export_pixels(b.width(), b.height(), st.render_dpi());
                    let (_, ext) = EXPORT_FORMATS[st.format.min(EXPORT_FORMATS.len() - 1)];
                    let alpha = st.transparent && crate::export::keeps_alpha(ext);
                    ui.label(
                        egui::RichText::new(trf(
                            "dialog.export_output",
                            &[
                                ("w", &pw.to_string()),
                                ("h", &ph.to_string()),
                                ("dpi", &format!("{:.0}", st.dpi)),
                                ("mode", if alpha { "sRGB + alpha" } else { "sRGB" }),
                            ],
                        ))
                        .color(Tokens::text_dim()),
                    );
                }
                if !scripted && st.format == 1 {
                    ui.horizontal(|ui| {
                        ui.label(tr("dialog.pdf_standard"));
                        egui::ComboBox::from_id_salt("pdf_standard")
                            .selected_text(PDF_STANDARDS[st.pdf_standard.min(3)])
                            .show_ui(ui, |ui| {
                                for (i, n) in PDF_STANDARDS.iter().enumerate() {
                                    ui.selectable_value(&mut st.pdf_standard, i, *n);
                                }
                            })
                            .response
                            .rich_tip(crate::ui::tips::keyed("dialog.pdf_standard"));
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("dialog.bleed"));
                        ui.add(
                            crate::ui::field::NumField::new(&mut st.bleed_mm)
                                .range(0.0..=50.0)
                                .speed(0.5)
                                .suffix(" mm"),
                        )
                        .rich_tip(crate::ui::tips::keyed("dialog.bleed"));
                    });
                    if st.pdf_standard > 0 {
                        ui.horizontal(|ui| {
                            ui.label(tr("dialog.output_condition"));
                            ui.text_edit_singleline(&mut st.output_condition)
                                .rich_tip(crate::ui::tips::keyed("dialog.output_condition"));
                        });
                        ui.label(
                            egui::RichText::new(tr("dialog.pdfx_profile_note"))
                                .color(Tokens::text_dim())
                                .size(11.0),
                        );
                    }
                }
                ui.checkbox(&mut st.selection_only, tr("dialog.selected_only"))
                    .rich_tip(crate::ui::tips::pair(
                        "dialog.selected_only",
                        "dialog.export_selected_desc",
                    ));
                if !st.selection_only && st.format != 1 && st.format != 11 {
                    ui.checkbox(&mut st.all_pages, tr("dialog.all_pages"))
                        .rich_tip(crate::ui::tips::pair(
                            "dialog.all_pages",
                            "dialog.export_all_pages_desc",
                        ));
                }
                if !st.web {
                    ui.checkbox(
                        &mut app.settings.formats.skip_export_dialog,
                        tr("dialog.skip_export_dialog"),
                    )
                    .rich_tip(crate::ui::tips::pair(
                        "dialog.skip_export_dialog",
                        "dialog.skip_export_dialog_tip",
                    ));
                }
                if !scripted && export_keeps_properties(EXPORT_FORMATS[st.format].1) {
                    egui::Grid::new("exp_props").num_columns(2).show(ui, |ui| {
                        ui.label(tr("dialog.title"));
                        ui.add_sized([200.0, 20.0], egui::TextEdit::singleline(&mut st.title))
                            .rich_tip(crate::ui::tips::pair(
                                "dialog.title",
                                "dialog.export_title_desc",
                            ));
                        ui.end_row();
                        ui.label(tr("dialog.tags"));
                        ui.add_sized([200.0, 20.0], egui::TextEdit::singleline(&mut st.tags))
                            .rich_tip(crate::ui::tips::pair("dialog.tags", "dialog.tags_tip"));
                        ui.end_row();
                    });
                }
                if let Some(bytes) = estimate.filter(|_| !scripted) {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(trf(
                            "dialog.export_estimate",
                            &[("size", &crate::export::human_size(bytes))],
                        ))
                        .color(Tokens::text_dim()),
                    );
                }
            });
        });
        if ok_cancel(ui, close) {
            let filter = st.script.as_ref().and_then(|n| {
                app.settings
                    .script_filters
                    .iter()
                    .find(|f| &f.name == n && f.export)
                    .cloned()
            });
            if let Some(filter) = filter {
                let stem = app
                    .file
                    .as_ref()
                    .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                    .unwrap_or_else(|| "Graphic1".into());
                let ext = filter.extensions.first().cloned().unwrap_or_default();
                let exts: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
                let st2 = st.clone();
                crate::files::Dialog::new()
                    .add_filter(filter.label(), &exts)
                    .set_file_name(format!("{stem}.{ext}"))
                    .save_file(app, move |app, path| {
                        match app.export_with_script(&path, &filter, &st2) {
                            Ok(msg) => app.status = msg,
                            Err(e) => app.status = format!("{}: {e}", tr("dialog.export_failed")),
                        }
                    });
                return;
            }
            export_with_file_dialog(app, st);
            app.keep_export_choices(st);
        }
    });
}

/// The pages Delete Page deletes, as indices (0 = the first): `from`
/// alone, or `from` through `to` in either order, kept to the `n` pages.
pub(crate) fn delete_page_range(from: usize, to: usize, through: bool, n: usize) -> (usize, usize) {
    let last = n.saturating_sub(1);
    let a = from.saturating_sub(1).min(last);
    let b = if through {
        to.saturating_sub(1).min(last)
    } else {
        a
    };
    (a.min(b), a.max(b))
}

/// Print > Layout: the imposition, the sheet and the gap between pages,
/// with a sketch of the first sheet.
/// A Custom sheet size saved in the layout fields goes into the
/// preferences (replacing one of that name, #591).
pub(crate) fn take_saved_sheet(app: &mut App, st: &mut PrintState) {
    if let Some((name, size)) = st.sheet_to_save.take() {
        let list = &mut app.settings.printing.sheet_presets;
        list.retain(|(n, _)| *n != name);
        list.push((name, size));
    }
}

/// The paper's margins kept clear (left, right, top, bottom, mm), in a
/// row: the Print dialog's Layout and Print Preview share them.
pub(crate) fn margin_fields(ui: &mut Ui, margins: &mut [f64; 4]) {
    ui.horizontal(|ui| {
        for (m, key) in margins.iter_mut().zip([
            "print.margin_left",
            "print.margin_right",
            "print.margin_top",
            "print.margin_bottom",
        ]) {
            ui.add(
                crate::ui::field::NumField::new(m)
                    .range(0.0..=200.0)
                    .speed(0.5)
                    .suffix(" mm")
                    .width(62.0),
            )
            .rich_tip(tr(key));
        }
    });
}

pub(crate) fn print_layout_fields(app: &App, ui: &mut Ui, st: &mut PrintState) {
    ui.strong(tr("print.layout"));
    ui.vertical(|ui| {
        ui.vertical(|ui| {
            egui::Grid::new("print_layout")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label(tr("print.imposition"));
                    let kinds = [
                        "print.layout_as_is",
                        "print.layout_n_up",
                        "print.layout_booklet",
                        "print.layout_repeat",
                    ];
                    egui::ComboBox::from_id_salt("print_layout_kind")
                        .selected_text(tr(kinds[st.layout_kind.min(3)]))
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            for (i, k) in kinds.iter().enumerate() {
                                ui.selectable_value(&mut st.layout_kind, i, tr(k));
                            }
                        });
                    ui.end_row();
                    if matches!(st.layout_kind, 1 | 3) {
                        ui.label(tr("print.grid"));
                        ui.horizontal(|ui| {
                            ui.add(crate::ui::field::NumField::new(&mut st.cols).range(1..=99));
                            ui.label("x");
                            ui.add(crate::ui::field::NumField::new(&mut st.rows).range(1..=99));
                        });
                        ui.end_row();
                    }
                    ui.label(tr("print.sheet"));
                    ui.horizontal(|ui| {
                        let name = sheet_name;
                        egui::ComboBox::from_id_salt("print_sheet")
                            .selected_text(name(st.sheet))
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for i in 0..=CUSTOM_SHEET {
                                    ui.selectable_value(&mut st.sheet, i, name(i));
                                }
                                // Custom sizes saved before (#591).
                                let saved = app.settings.printing.sheet_presets.clone();
                                if !saved.is_empty() {
                                    ui.separator();
                                }
                                for (n, size) in saved {
                                    let on = st.sheet == CUSTOM_SHEET && st.custom_sheet == size;
                                    if ui.selectable_label(on, n).clicked() {
                                        st.sheet = CUSTOM_SHEET;
                                        st.custom_sheet = size;
                                    }
                                }
                            });
                        ui.add_enabled(
                            st.sheet > 0,
                            egui::Checkbox::new(&mut st.landscape, tr("print.landscape")),
                        );
                    });
                    ui.end_row();
                    // The Custom size, and saving it under a name (#591).
                    if st.sheet >= CUSTOM_SHEET {
                        ui.label("");
                        ui.horizontal(|ui| {
                            ui.add(
                                crate::ui::field::NumField::new(&mut st.custom_sheet.width)
                                    .range(10.0..=5000.0)
                                    .speed(1.0)
                                    .suffix(" mm")
                                    .width(80.0),
                            );
                            ui.label("x");
                            ui.add(
                                crate::ui::field::NumField::new(&mut st.custom_sheet.height)
                                    .range(10.0..=5000.0)
                                    .speed(1.0)
                                    .suffix(" mm")
                                    .width(80.0),
                            );
                        });
                        ui.end_row();
                        ui.label("");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut st.sheet_name_typed)
                                    .desired_width(120.0)
                                    .hint_text(tr("print.sheet_name")),
                            );
                            let name = st.sheet_name_typed.trim().to_string();
                            if ui
                                .add_enabled(
                                    !name.is_empty(),
                                    egui::Button::new(tr("print.sheet_save")),
                                )
                                .clicked()
                            {
                                st.sheet_to_save = Some((name, st.custom_sheet));
                                st.sheet_name_typed.clear();
                            }
                        });
                        ui.end_row();
                    }
                    if st.sheet > 0 {
                        ui.label("");
                        ui.checkbox(&mut st.stretch_fit, tr("print.stretch_fit"))
                            .rich_tip(tr("print.stretch_fit_tip"));
                        ui.end_row();
                    }
                    if matches!(st.layout_kind, 1 | 3) {
                        ui.label(tr("print.gutter"));
                        ui.add(
                            crate::ui::field::NumField::new(&mut st.gutter)
                                .range(0.0..=50.0)
                                .speed(0.5)
                                .suffix(" mm"),
                        );
                        ui.end_row();
                        ui.label("");
                        ui.checkbox(&mut st.back_side, tr("print.back_side"))
                            .rich_tip(tr("print.back_side_tip"));
                        ui.end_row();
                    }
                    if st.sheet > 0 {
                        ui.label("");
                        ui.checkbox(&mut st.keep_size, tr("print.keep_size"))
                            .rich_tip(tr("print.keep_size_tip"));
                        ui.end_row();
                    }
                    // The paper's margins kept clear: typed here, or in
                    // Print Preview their dashed lines dragged on the
                    // sheet (#580, #582, #909).
                    ui.label(tr("print.paper_margins"))
                        .rich_tip(tr("print.margins"));
                    margin_fields(ui, &mut st.margins);
                    ui.end_row();
                });
        });
        ui.add_space(8.0);
        // A sketch of the first sheet and the sheet count.
        let mut doc_pages: Vec<Size> = app.doc().pages.iter().map(|p| p.size).collect();
        if !st.all_pages {
            doc_pages = vec![app.page_size()];
        }
        let options = crate::export::print_layout(app, st, app.doc());
        let plan = crate::imposition::plan(&doc_pages, &options);
        ui.vertical_centered(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(220.0, 180.0), egui::Sense::hover());
            if let Some(sheet) = plan.first() {
                let k = (r.width() / sheet.size.width as f32)
                    .min(r.height() / sheet.size.height as f32);
                let w = sheet.size.width as f32 * k;
                let h = sheet.size.height as f32 * k;
                let o = r.center() - egui::vec2(w / 2.0, h / 2.0);
                // Pages kept at their size may run off the sheet: the
                // sketch shows the sheet's part of them.
                let p = ui
                    .painter()
                    .with_clip_rect(egui::Rect::from_min_size(o, egui::vec2(w, h)).expand(1.0));
                p.rect(
                    egui::Rect::from_min_size(o, egui::vec2(w, h)),
                    0.0,
                    crate::theme::Tokens::field(),
                    egui::Stroke::new(1.0, Tokens::control_border()),
                    egui::StrokeKind::Inside,
                );
                for c in &sheet.cells {
                    // Sheet y runs up; the sketch's down.
                    let cell = egui::Rect::from_min_max(
                        egui::pos2(o.x + c.trim.x0 as f32 * k, o.y + h - c.trim.y1 as f32 * k),
                        egui::pos2(o.x + c.trim.x1 as f32 * k, o.y + h - c.trim.y0 as f32 * k),
                    );
                    let fill = if c.page.is_some() {
                        egui::Color32::from_rgb(0xDD, 0xEB, 0xF7)
                    } else {
                        crate::theme::Tokens::control_fill()
                    };
                    p.rect(
                        cell,
                        0.0,
                        fill,
                        egui::Stroke::new(1.0, Tokens::ACCENT),
                        egui::StrokeKind::Inside,
                    );
                    if let Some(n) = c.page {
                        p.text(
                            cell.center(),
                            egui::Align2::CENTER_CENTER,
                            (n + 1).to_string(),
                            egui::FontId::proportional(11.0),
                            Tokens::text(),
                        );
                    }
                }
            }
            ui.label(
                egui::RichText::new(trf("print.sheets", &[("n", &plan.len().to_string())]))
                    .color(Tokens::text_dim()),
            );
            if crate::imposition::overflows(&plan) {
                ui.colored_label(ui.visuals().warn_fg_color, tr("print.keep_size_overflow"));
            }
        });
    });
}

/// A titled frame around a group of the Print dialog's options.
fn print_group(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.strong(title);
        add(ui);
    });
}

fn print_dialog(app: &mut App, ctx: &Context, st: &mut PrintState, close: &mut bool) {
    take_saved_sheet(app, st);
    if app.poll_printers(st) {
        // Look again shortly while the list is read.
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    window(ctx, tr("dialog.print")).show(ctx, |ui| {
        let issues = if app.settings.printing.preflight {
            st.preflight
                .get_or_insert_with(|| crate::ui::preflight_dialog::issues_of(app))
                .clone()
        } else {
            Vec::new()
        };
        let issues_tab = if issues.is_empty() {
            tr("print.tab_no_issues")
        } else {
            trf("print.tab_issues", &[("n", &issues.len().to_string())])
        };
        let tabs = [
            tr("print.tab_general"),
            tr("print.tab_color"),
            tr("print.tab_composite"),
            tr("print.tab_layout"),
            tr("print.tab_prepress"),
            tr("print.tab_postscript"),
            issues_tab,
        ];
        const TAB_DESCS: [&str; 7] = [
            "print.tab_general_desc",
            "print.tab_color_desc",
            "print.tab_composite_desc",
            "print.tab_layout_desc",
            "print.tab_prepress_desc",
            "print.tab_postscript_desc",
            "print.tab_issues_desc",
        ];
        ui.horizontal(|ui| {
            for (i, t) in tabs.iter().enumerate() {
                if i == 6 && !app.settings.printing.preflight {
                    continue;
                }
                ui.selectable_value(&mut st.tab, i, t.as_str())
                    .rich_tip_with(|| format!("{t}\n{}", tr(TAB_DESCS[i])));
            }
        });
        ui.separator();
        egui::Frame::new().show(ui, |ui| {
            ui.set_min_size(egui::vec2(560.0, 380.0));
            ui.set_max_width(560.0);
            match st.tab {
                1 => print_tab_color(ui, st),
                2 => print_tab_composite(ui, st),
                3 => print_tab_layout(app, ui, st),
                4 => print_tab_prepress(app, ui, st),
                5 => print_tab_postscript(ui, st),
                6 => print_tab_issues(app, ui, &issues, close),
                _ => {
                    if let Some(p) = print_tab_general(app, ui, st) {
                        if let Err(e) = crate::printers::open_properties(&p) {
                            app.status = e;
                        }
                    }
                }
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui
                .button(tr("menu.file.print_preview"))
                .rich_tip(crate::ui::tips::pair(
                    "menu.file.print_preview",
                    "print.preview_desc",
                ))
                .clicked()
            {
                app.print_preview(Some(st.clone()));
                *close = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(tr("print.apply"))
                    .rich_tip(crate::ui::tips::keyed("print.apply"))
                    .clicked()
                {
                    app.settings.printing.remember(st);
                    app.settings.save();
                }
                if ui
                    .button(tr("dialog.cancel"))
                    .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                    .clicked()
                {
                    *close = true;
                }
                // While the first printer list is read, a print to a
                // printer waits for it (#614).
                let waiting =
                    st.destination == 0 && st.looking_for_printers && st.printers.is_empty();
                if ui
                    .add_enabled(!waiting, egui::Button::new(tr("dialog.print")))
                    .rich_tip(crate::ui::tips::pair("dialog.print", "print.print_desc"))
                    .rich_disabled_tip(crate::ui::tips::pair("dialog.print", "print.print_desc"))
                    .clicked()
                {
                    match crate::export::print(app, st) {
                        Ok(msg) if !msg.is_empty() => app.status = msg,
                        Ok(_) => {}
                        Err(e) => app.status = format!("{}: {e}", tr("dialog.print_failed")),
                    }
                    if app.settings.printing.remember_last {
                        app.settings.printing.remember(st);
                        app.settings.save();
                    }
                    *close = true;
                }
            });
        });
    });
}

/// The destination list (the system's printers in name order, then the
/// PDF viewer and the file destinations), `width` wide, and the button
/// of the chosen printer's own settings; the printer whose Properties
/// button was clicked. The Print dialog's General tab and Print Preview
/// show it (#614, #792).
pub fn destination_row(ui: &mut Ui, st: &mut PrintState, width: f32) -> Option<String> {
    let mut properties = None;
    let dests = ["print.dest_viewer", "print.dest_pdf", "print.dest_ps"];
    let on_printer = st.destination == 0 && st.printer.is_some();
    let selected = match (&st.printer, st.destination) {
        (Some(p), 0) => p.clone(),
        _ => tr(dests[st.destination.min(2)]),
    };
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("print_destination")
            .selected_text(selected)
            .width(width)
            .show_ui(ui, |ui| {
                if st.looking_for_printers && st.printers.is_empty() {
                    ui.add_enabled(false, egui::Label::new(tr("print.looking")));
                }
                for p in st.printers.clone() {
                    let is = on_printer && st.printer.as_deref() == Some(p.name.as_str());
                    if ui.selectable_label(is, &p.name).clicked() {
                        st.destination = 0;
                        st.printer = Some(p.name.clone());
                        st.printer_picked = true;
                    }
                }
                for (i, d) in dests.iter().enumerate() {
                    let is = st.destination == i && (i != 0 || !on_printer);
                    if ui.selectable_label(is, tr(d)).clicked() {
                        st.destination = i;
                        st.printer_picked = true;
                        if i == 0 {
                            st.printer = None;
                        }
                    }
                }
            })
            .response
            .rich_tip(crate::ui::tips::keyed("print.printer"));
        // The printer's own settings (#614).
        let gear = ui
            .add_enabled(
                on_printer,
                egui::Button::new("").min_size(egui::vec2(24.0, 22.0)),
            )
            .rich_tip(crate::ui::tips::keyed("print.properties"))
            .rich_disabled_tip(crate::ui::tips::keyed("print.properties"));
        crate::ui::icons::draw_action(
            ui.painter(),
            egui::Rect::from_center_size(gear.rect.center(), egui::vec2(16.0, 16.0)),
            crate::ui::icons::Action::Options,
            if on_printer {
                Tokens::icon()
            } else {
                Tokens::icon_off()
            },
        );
        if gear.clicked() {
            properties = st.printer.clone();
        }
    });
    properties
}

/// The choice of every page, the odd ones or the even ones, for the
/// whole drawing or the pages listed (greyed out for the current page
/// and the selection), as wide as its longest choice (#951).
pub fn parity_combo(ui: &mut Ui, st: &mut PrintState, id: &str) {
    let keys = [
        "print.odd_and_even",
        "options.scope_odd",
        "options.scope_even",
    ];
    let names = keys.map(tr);
    let font = egui::TextStyle::Button.resolve(ui.style());
    let widest = names
        .iter()
        .map(|n| {
            ui.fonts_mut(|f| f.layout_no_wrap(n.clone(), font.clone(), egui::Color32::WHITE))
                .size()
                .x
        })
        .fold(0.0_f32, f32::max);
    ui.add_enabled_ui(matches!(st.range, 0 | 2), |ui| {
        egui::ComboBox::from_id_salt(id)
            .selected_text(names[st.parity.min(2)].clone())
            .width(widest + 2.0 * ui.spacing().button_padding.x + ui.spacing().icon_width)
            .show_ui(ui, |ui| {
                for (i, n) in names.iter().enumerate() {
                    ui.selectable_value(&mut st.parity, i, n.clone());
                }
            })
            .response
            .rich_tip(crate::ui::tips::pair("print.parity", "print.parity_desc"))
            .rich_disabled_tip(crate::ui::tips::pair("print.parity", "print.parity_desc"));
    });
}

/// The General tab; the printer whose Properties button was clicked.
fn print_tab_general(app: &App, ui: &mut Ui, st: &mut PrintState) -> Option<String> {
    let mut properties = None;
    print_group(ui, &tr("print.destination"), |ui| {
        egui::Grid::new("print_dest")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label(tr("print.printer"));
                if let Some(p) = destination_row(ui, st, 300.0) {
                    properties = Some(p);
                }
                ui.end_row();
                ui.label(tr("print.orientation"));
                let ors = [
                    "print.orient_match",
                    "print.orient_portrait",
                    "print.orient_landscape",
                ];
                egui::ComboBox::from_id_salt("print_orientation")
                    .selected_text(tr(ors[st.orientation.min(2)]))
                    .width(300.0)
                    .show_ui(ui, |ui| {
                        for (i, o) in ors.iter().enumerate() {
                            if ui.selectable_value(&mut st.orientation, i, tr(o)).clicked() && i > 0
                            {
                                st.landscape = i == 2;
                            }
                        }
                    })
                    .response
                    .rich_tip(crate::ui::tips::keyed("print.orientation"));
                ui.end_row();
                // What the system says of the printer chosen (#614).
                let on_printer = st.destination == 0 && st.printer.is_some();
                let chosen = st
                    .printers
                    .iter()
                    .find(|p| on_printer && st.printer.as_deref() == Some(p.name.as_str()));
                let rows = [
                    ("print.status", chosen.map(|p| tr(p.state.key()))),
                    ("print.location", chosen.map(|p| p.location.clone())),
                    ("print.comment", chosen.map(|p| p.comment.clone())),
                ];
                for (key, value) in rows {
                    ui.label(tr(key));
                    ui.label(value.unwrap_or_default());
                    ui.end_row();
                }
            });
    });
    ui.add_space(6.0);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(270.0);
            print_group(ui, &tr("print.range"), |ui| {
                ui.radio_value(&mut st.range, 0, tr("print.range_document"))
                    .rich_tip(crate::ui::tips::keyed("print.range_document"));
                ui.radio_value(&mut st.range, 1, tr("print.range_page"))
                    .rich_tip(crate::ui::tips::keyed("print.range_page"));
                ui.horizontal(|ui| {
                    ui.radio_value(&mut st.range, 2, tr("print.range_pages"))
                        .rich_tip(crate::ui::tips::keyed("print.range_pages"));
                    ui.add_enabled(
                        st.range == 2,
                        egui::TextEdit::singleline(&mut st.pages)
                            .hint_text(format!("1-{}", app.doc().pages.len()))
                            .desired_width(110.0),
                    )
                    .rich_tip(crate::ui::tips::pair(
                        "print.page_list",
                        "print.page_list_desc",
                    ))
                    .rich_disabled_tip(crate::ui::tips::pair(
                        "print.page_list",
                        "print.page_list_desc",
                    ));
                });
                ui.add_enabled_ui(!app.selection.is_empty(), |ui| {
                    ui.radio_value(&mut st.range, 3, tr("print.range_selection"))
                        .rich_tip(crate::ui::tips::keyed("print.range_selection"))
                        .rich_disabled_tip(crate::ui::tips::keyed("print.range_selection"));
                });
                parity_combo(ui, st, "print_parity");
            });
        });
        ui.vertical(|ui| {
            print_group(ui, &tr("print.copies"), |ui| {
                crate::ui::tips::tip_over(
                    ui,
                    crate::ui::tips::keyed("print.number_of_copies"),
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.label(tr("print.number_of_copies"));
                            ui.add(crate::ui::field::NumField::new(&mut st.copies).range(1..=999));
                        })
                    },
                );
                ui.add_enabled(
                    st.copies > 1,
                    egui::Checkbox::new(&mut st.collate, tr("print.collate")),
                )
                .rich_tip(crate::ui::tips::keyed("print.collate"))
                .rich_disabled_tip(crate::ui::tips::keyed("print.collate"));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.checkbox(&mut st.as_bitmap, tr("print.as_bitmap"))
                    .rich_tip(crate::ui::tips::keyed("print.as_bitmap"));
                ui.add_enabled(
                    st.as_bitmap,
                    crate::ui::field::NumField::new(&mut st.bitmap_dpi)
                        .range(36.0..=1200.0)
                        .suffix(" dpi"),
                )
                .rich_tip(crate::ui::tips::pair(
                    "print.bitmap_dpi",
                    "print.bitmap_dpi_desc",
                ))
                .rich_disabled_tip(crate::ui::tips::pair(
                    "print.bitmap_dpi",
                    "print.bitmap_dpi_desc",
                ));
            });
        });
    });
    // Keep the older "all pages" choice in step with the range.
    st.all_pages = st.range != 1;
    properties
}

fn print_tab_color(ui: &mut Ui, st: &mut PrintState) {
    egui::Grid::new("print_color")
        .num_columns(2)
        .spacing([24.0, 8.0])
        .show(ui, |ui| {
            ui.label(tr("print.color"));
            ui.vertical(|ui| {
                ui.radio_value(&mut st.separations, false, tr("print.composite_output"));
                ui.radio_value(&mut st.separations, true, tr("print.separations_output"));
            });
            ui.end_row();
            ui.label("");
            ui.checkbox(&mut st.spot_to_cmyk, tr("print.spot_to_cmyk"));
            ui.end_row();
        });
    ui.add_space(8.0);
    ui.label(egui::RichText::new(tr("print.color_hint")).color(Tokens::text_dim()));
}

fn print_tab_composite(ui: &mut Ui, st: &mut PrintState) {
    print_group(ui, &tr("print.options"), |ui| {
        ui.checkbox(&mut st.text_black, tr("print.text_black"));
        ui.checkbox(&mut st.overprint_black, tr("print.overprint_black"));
    });
}

fn print_tab_layout(app: &App, ui: &mut Ui, st: &mut PrintState) {
    print_group(ui, &tr("print.position_size"), |ui| {
        let mut mode = if st.fit_to_page {
            1
        } else if (st.scale - 100.0).abs() > 1e-9 {
            2
        } else {
            0
        };
        ui.radio_value(&mut mode, 0, tr("print.as_in_document"));
        ui.radio_value(&mut mode, 1, tr("dialog.fit_to_page"));
        ui.horizontal(|ui| {
            ui.radio_value(&mut mode, 2, tr("print.scale"));
            ui.add_enabled(
                mode == 2,
                crate::ui::field::NumField::new(&mut st.scale)
                    .range(1.0..=1000.0)
                    .suffix(" %"),
            );
        });
        match mode {
            0 => {
                st.fit_to_page = false;
                st.scale = 100.0;
            }
            1 => st.fit_to_page = true,
            _ => {
                st.fit_to_page = false;
                if (st.scale - 100.0).abs() < 1e-9 {
                    st.scale = 99.0;
                }
            }
        }
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.checkbox(&mut st.bleed, tr("print.bleed_limit"));
        ui.add_enabled(
            st.bleed,
            crate::ui::field::NumField::new(&mut st.bleed_limit)
                .range(0.0..=50.0)
                .speed(0.5)
                .suffix(" mm"),
        );
    });
    ui.add_space(6.0);
    print_layout_fields(app, ui, st);
}

fn print_tab_prepress(app: &App, ui: &mut Ui, st: &mut PrintState) {
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(270.0);
            print_group(ui, &tr("print.paper_film"), |ui| {
                ui.checkbox(&mut st.invert, tr("dialog.invert"));
                ui.checkbox(&mut st.mirror, tr("dialog.mirror"));
            });
            ui.add_space(6.0);
            print_group(ui, &tr("print.file_info"), |ui| {
                ui.checkbox(&mut st.page_info, tr("print.print_file_info"));
                ui.add_enabled(
                    st.page_info,
                    egui::TextEdit::singleline(&mut st.info_text)
                        .hint_text(app.document_name())
                        .desired_width(240.0),
                );
                ui.checkbox(&mut st.page_numbers, tr("print.page_numbers"));
            });
            ui.add_space(6.0);
            print_group(ui, &tr("print.finishing"), |ui| {
                ui.checkbox(&mut st.crop_marks, tr("print.crop_fold_marks"));
            });
        });
        ui.vertical(|ui| {
            print_group(ui, &tr("print.registration"), |ui| {
                ui.checkbox(&mut st.registration_marks, tr("dialog.registration_marks"));
            });
            ui.add_space(6.0);
            print_group(ui, &tr("print.calibration"), |ui| {
                ui.checkbox(&mut st.color_bars, tr("print.color_bars"));
                ui.checkbox(&mut st.densitometer, tr("print.densitometer"));
            });
        });
    });
}

fn print_tab_postscript(ui: &mut Ui, st: &mut PrintState) {
    if st.destination != 2 {
        ui.label(egui::RichText::new(tr("print.postscript_hint")).color(Tokens::text_dim()));
        ui.add_space(6.0);
    }
    ui.add_enabled_ui(st.destination == 2, |ui| {
        print_group(ui, &tr("print.compatibility"), |ui| {
            egui::ComboBox::from_id_salt("print_ps_level")
                .selected_text(trf("print.ps_level", &[("n", &st.ps_level.to_string())]))
                .show_ui(ui, |ui| {
                    for l in [2u8, 3] {
                        ui.selectable_value(
                            &mut st.ps_level,
                            l,
                            trf("print.ps_level", &[("n", &l.to_string())]),
                        );
                    }
                });
        });
    });
}

fn print_tab_issues(
    app: &mut App,
    ui: &mut Ui,
    issues: &[crate::preflight::Issue],
    close: &mut bool,
) {
    ui.label(crate::ui::preflight_dialog::summary(issues));
    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            for issue in issues {
                ui.horizontal(|ui| {
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                    crate::ui::preflight_dialog::paint_icon(
                        ui.painter(),
                        r.center(),
                        issue.check.severity(),
                    );
                    let mut text = tr(&issue.check.key());
                    if !issue.detail.is_empty() {
                        text = format!("{text}: {}", issue.detail);
                    }
                    ui.label(text);
                });
            }
        });
    ui.add_space(6.0);
    if ui.button(tr("preflight.open")).clicked() {
        app.dialog = Dialog::Preflight(Default::default());
        *close = true;
    }
}

fn merge_kind_names() -> [String; 4] {
    [
        tr("merge.kind_text"),
        tr("merge.kind_number"),
        tr("merge.kind_image"),
        tr("merge.kind_qr"),
    ]
}

fn merge_kind_of(index: usize) -> crate::print_merge::FieldKind {
    use crate::print_merge::FieldKind;
    match index {
        1 => FieldKind::Number {
            start: 1,
            step: 1,
            digits: 1,
            format: Default::default(),
        },
        2 => FieldKind::Image,
        3 => FieldKind::Qr,
        _ => FieldKind::Text,
    }
}

/// A number field's Format: 1, 2, 3 / A, B, C / a, b, c / Custom, and for
/// Custom the texts before and after the number (#930).
fn merge_number_format(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    format: &mut crate::print_merge::NumberFormat,
) {
    use crate::print_merge::NumberFormat;
    let cur = format.index();
    let mut pick = cur;
    ui.horizontal(|ui| {
        ui.label(tr("merge.format"));
        egui::ComboBox::from_id_salt(("merge_format", salt))
            .selected_text(tr(NumberFormat::KEYS[cur]))
            .width(110.0)
            .show_ui(ui, |ui| {
                for (i, key) in NumberFormat::KEYS.iter().enumerate() {
                    ui.selectable_value(&mut pick, i, tr(key));
                }
            })
            .response
            .rich_tip(crate::ui::tips::keyed("merge.format"));
    });
    if pick != cur {
        *format = NumberFormat::from_index(pick);
    }
    if let NumberFormat::Custom { prefix, suffix } = format {
        ui.horizontal(|ui| {
            ui.label(tr("merge.prefix"));
            ui.add(egui::TextEdit::singleline(prefix).desired_width(90.0))
                .rich_tip(crate::ui::tips::keyed("merge.prefix"));
        });
        ui.horizontal(|ui| {
            ui.label(tr("merge.suffix"));
            ui.add(egui::TextEdit::singleline(suffix).desired_width(90.0))
                .rich_tip(crate::ui::tips::keyed("merge.suffix"));
        });
    }
}

/// Whether a number format pads with zeros (letters have no digits).
fn merge_format_has_digits(format: &crate::print_merge::NumberFormat) -> bool {
    use crate::print_merge::NumberFormat;
    !matches!(format, NumberFormat::Upper | NumberFormat::Lower)
}

fn print_merge_dialog(app: &mut App, ctx: &Context, st: &mut PrintMergeState, close: &mut bool) {
    use crate::print_merge::{self as pm, FieldKind};
    window(ctx, tr("dialog.print_merge")).show(ctx, |ui| {
        ui.set_width(720.0);
        if st.kinds.len() < st.headers.len() {
            st.kinds.resize(st.headers.len(), FieldKind::Text);
        }
        ui.horizontal(|ui| {
            ui.label(tr("merge.hint"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(tr("merge.import_file"))
                    .rich_tip(crate::ui::tips::keyed("merge.import_file"))
                    .clicked()
                {
                    crate::files::Dialog::new()
                        .add_filter("CSV, Excel", &["csv", "txt", "xlsx"])
                        .add_filter("Excel", &["xlsx"])
                        .add_filter("CSV", &["csv", "txt"])
                        .pick_into("print_merge_csv");
                }
            });
        });
        if let Some(p) = crate::files::take_picked("print_merge_csv") {
            match crate::print_merge::read_data(&p) {
                Ok((h, r)) => {
                    st.kinds = crate::print_merge::guess_kinds(&h, &r);
                    st.headers = h;
                    st.rows = r;
                    st.current = 0;
                    st.path = Some(p);
                    // The fields' types show at once, to set each one.
                    st.configure = true;
                }
                Err(e) => app.status = e,
            }
        }
        ui.add_space(4.0);
        ui.strong(tr("merge.add_or_edit"));
        ui.horizontal(|ui| {
            if ui
                .button(tr("merge.add_column"))
                .rich_tip(crate::ui::tips::keyed("merge.add_column"))
                .clicked()
            {
                st.adding = Some((String::new(), 0));
                ui.memory_mut(|m| m.request_focus(egui::Id::new("merge_add_name")));
            }
            if ui
                .add_enabled(
                    !st.headers.is_empty(),
                    egui::Button::new(tr("merge.configure")).selected(st.configure),
                )
                .rich_tip_always(crate::ui::tips::keyed("merge.configure"))
                .clicked()
            {
                st.configure = !st.configure;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if st.single {
                    tr("merge.view_all")
                } else {
                    tr("merge.view_single")
                };
                let tip = if st.single {
                    crate::ui::tips::keyed("merge.view_all")
                } else {
                    crate::ui::tips::keyed("merge.view_single")
                };
                if ui
                    .add_enabled(!st.rows.is_empty(), egui::Button::new(label))
                    .rich_tip_always(tip)
                    .clicked()
                {
                    st.single = !st.single;
                }
            });
        });
        // Add Column: name and type.
        let mut add_done = None;
        if let Some((name, kind)) = &mut st.adding {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong(tr("merge.add_column_title"));
                ui.label(tr("merge.name"));
                ui.add(
                    egui::TextEdit::singleline(name)
                        .id(egui::Id::new("merge_add_name"))
                        .desired_width(320.0),
                )
                .rich_tip(crate::ui::tips::pair("merge.name", "merge.new_name_desc"));
                ui.label(tr("merge.type"));
                ui.horizontal(|ui| {
                    for (i, n) in merge_kind_names().iter().enumerate() {
                        ui.radio_value(kind, i, n.as_str())
                            .rich_tip(crate::ui::tips::pair("merge.type", "merge.type_desc"));
                    }
                });
                // A number field: its counter, and an end value that makes
                // the records from the start to it (#791).
                if *kind == 1 {
                    let f = &mut st.adding_number;
                    merge_number_format(ui, "add", &mut f.format);
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            merge_format_has_digits(&f.format),
                            crate::ui::field::NumField::new(&mut f.digits)
                                .range(1..=12)
                                .prefix(tr("merge.digits")),
                        )
                        .rich_tip_always(crate::ui::tips::keyed("merge.digits"));
                        ui.add(
                            crate::ui::field::NumField::new(&mut f.start).prefix(tr("merge.start")),
                        )
                        .rich_tip(crate::ui::tips::keyed("merge.start"));
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut f.increment, tr("merge.increment"))
                            .rich_tip(crate::ui::tips::keyed("merge.increment"));
                        ui.add_enabled(
                            f.increment,
                            crate::ui::field::NumField::new(&mut f.step).prefix(tr("merge.step")),
                        )
                        .rich_tip_always(crate::ui::tips::keyed("merge.step"));
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut f.use_end, tr("merge.end_value"))
                            .rich_tip(crate::ui::tips::keyed("merge.end_value"));
                        ui.add_enabled(
                            f.use_end && f.increment,
                            crate::ui::field::NumField::new(&mut f.end),
                        )
                        .rich_tip_always(crate::ui::tips::keyed("merge.end_value"));
                    });
                }
                ui.horizontal(|ui| {
                    let ok = !name.trim().is_empty();
                    if ui
                        .add_enabled(ok, egui::Button::new(tr("merge.add")))
                        .rich_tip_always(crate::ui::tips::keyed("merge.add"))
                        .clicked()
                    {
                        add_done = Some(Some((name.trim().to_string(), *kind)));
                    }
                    if ui
                        .button(tr("dialog.cancel"))
                        .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                        .clicked()
                    {
                        add_done = Some(None);
                    }
                });
            });
        }
        if let Some(done) = add_done {
            if let Some((name, kind)) = done {
                if kind == 1 {
                    let f = st.adding_number.clone();
                    pm::add_number_field(
                        st,
                        &name,
                        f.start,
                        if f.increment { f.step } else { 0 },
                        f.digits,
                        f.format,
                        (f.use_end && f.increment).then_some(f.end),
                    );
                } else {
                    pm::add_field(st, &name);
                    if let Some(k) = st.kinds.last_mut() {
                        *k = merge_kind_of(kind);
                    }
                }
                if st.rows.is_empty() {
                    pm::add_record(st);
                }
            }
            st.adding = None;
        }
        // Configure Columns: name, kind and counter settings of each field.
        let mut remove_field = None;
        if st.configure && !st.headers.is_empty() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("merge_cfg")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for i in 0..st.headers.len() {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut st.headers[i])
                                                .desired_width(110.0),
                                        )
                                        .rich_tip(
                                            crate::ui::tips::pair(
                                                "merge.name",
                                                "merge.field_name_desc",
                                            ),
                                        );
                                        if ui
                                            .small_button("x")
                                            .rich_tip(crate::ui::tips::keyed("merge.delete_field"))
                                            .clicked()
                                        {
                                            remove_field = Some(i);
                                        }
                                    });
                                    let names = merge_kind_names();
                                    let cur = match st.kinds[i] {
                                        FieldKind::Text => 0,
                                        FieldKind::Number { .. } => 1,
                                        FieldKind::Image => 2,
                                        FieldKind::Qr => 3,
                                    };
                                    let mut pick = cur;
                                    egui::ComboBox::from_id_salt(("merge_kind", i))
                                        .selected_text(names[cur].clone())
                                        .width(110.0)
                                        .show_ui(ui, |ui| {
                                            for (k, n) in names.iter().enumerate() {
                                                ui.selectable_value(&mut pick, k, n.as_str());
                                            }
                                        })
                                        .response
                                        .rich_tip(crate::ui::tips::pair(
                                            "merge.type",
                                            "merge.type_desc",
                                        ));
                                    if pick != cur {
                                        st.kinds[i] = merge_kind_of(pick);
                                    }
                                    if let FieldKind::Number {
                                        start,
                                        step,
                                        digits,
                                        format,
                                    } = &mut st.kinds[i]
                                    {
                                        merge_number_format(ui, ("cfg", i), format);
                                        ui.add(
                                            crate::ui::field::NumField::new(start)
                                                .prefix(tr("merge.start")),
                                        )
                                        .rich_tip(crate::ui::tips::keyed("merge.start"));
                                        ui.add(
                                            crate::ui::field::NumField::new(step)
                                                .prefix(tr("merge.step")),
                                        )
                                        .rich_tip(crate::ui::tips::keyed("merge.step"));
                                        ui.add_enabled(
                                            merge_format_has_digits(format),
                                            crate::ui::field::NumField::new(digits)
                                                .range(1..=12)
                                                .prefix(tr("merge.digits")),
                                        )
                                        .rich_tip_always(crate::ui::tips::keyed("merge.digits"));
                                    }
                                });
                                ui.separator();
                            }
                        });
                    });
            });
        }
        if let Some(i) = remove_field {
            pm::remove_field(st, i);
        }
        st.current = st.current.min(st.rows.len().saturating_sub(1));
        // The records: a grid, or the current one as a form.
        let values = pm::values(st);
        egui::Frame::new()
            .fill(Tokens::field())
            .stroke(egui::Stroke::new(1.0, Tokens::control_border()))
            .inner_margin(4.0)
            .show(ui, |ui| {
                ui.set_min_size(egui::vec2(700.0, 300.0));
                egui::ScrollArea::both()
                    .max_height(300.0)
                    .id_salt("merge_grid")
                    .show(ui, |ui| {
                        if st.headers.is_empty() {
                            return;
                        }
                        let cell = |ui: &mut Ui,
                                    st: &mut PrintMergeState,
                                    r: usize,
                                    c: usize,
                                    w: f32|
                         -> Option<egui::Response> {
                            if matches!(pm::kind_of(st, c), FieldKind::Number { .. }) {
                                let v = values
                                    .get(r)
                                    .and_then(|row| row.get(c))
                                    .cloned()
                                    .unwrap_or_default();
                                Some(
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(v).color(Tokens::text_dim()),
                                        )
                                        .sense(egui::Sense::click_and_drag()),
                                    ),
                                )
                            } else {
                                st.rows
                                    .get_mut(r)
                                    .and_then(|row| row.get_mut(c))
                                    .map(|text| {
                                        ui.add(egui::TextEdit::singleline(text).desired_width(w))
                                    })
                            }
                        };
                        for row in st.rows.iter_mut() {
                            row.resize(st.headers.len(), String::new());
                        }
                        if st.single {
                            let r = st.current;
                            egui::Grid::new("merge_single")
                                .num_columns(2)
                                .spacing([12.0, 6.0])
                                .show(ui, |ui| {
                                    for c in 0..st.headers.len() {
                                        ui.label(st.headers[c].clone());
                                        let _ = cell(ui, st, r, c, 420.0);
                                        ui.end_row();
                                    }
                                });
                        } else {
                            // Where each cell, record number and field name
                            // is, to select by dragging over them (#501).
                            let mut cells: Vec<((usize, usize), egui::Rect)> = Vec::new();
                            let mut heads: Vec<(pm::GridDrag, usize, egui::Rect)> = Vec::new();
                            let shift = ui.input(|i| i.modifiers.shift);
                            let last_col = st.headers.len() - 1;
                            egui::Grid::new("merge_grid")
                                .striped(true)
                                .spacing([6.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label("");
                                    for c in 0..st.headers.len() {
                                        let h = ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(st.headers[c].as_str())
                                                    .strong(),
                                            )
                                            .sense(egui::Sense::click_and_drag()),
                                        );
                                        heads.push((pm::GridDrag::Fields, c, h.rect));
                                        if h.drag_started() || h.clicked() {
                                            st.grid_drag = Some((pm::GridDrag::Fields, (0, c)));
                                            let rows = st.rows.len().saturating_sub(1);
                                            st.cells = Some(((0, c), (rows, c)));
                                        }
                                    }
                                    ui.end_row();
                                    for r in 0..st.rows.len() {
                                        let mark = if r == st.current { "\u{25b6}" } else { "" };
                                        let n = ui.add(
                                            egui::Button::selectable(
                                                r == st.current,
                                                format!("{mark}{}", r + 1),
                                            )
                                            .sense(egui::Sense::click_and_drag()),
                                        );
                                        heads.push((pm::GridDrag::Records, r, n.rect));
                                        if n.drag_started() || n.clicked() {
                                            st.current = r;
                                            st.grid_drag = Some((pm::GridDrag::Records, (r, 0)));
                                            st.cells = Some(((r, 0), (r, last_col)));
                                        }
                                        for c in 0..st.headers.len() {
                                            let Some(resp) = cell(ui, st, r, c, 130.0) else {
                                                continue;
                                            };
                                            cells.push(((r, c), resp.rect));
                                            if resp.drag_started() {
                                                st.grid_drag = Some((pm::GridDrag::Cells, (r, c)));
                                            }
                                            if resp.clicked() || resp.gained_focus() {
                                                let from = match st.cells {
                                                    Some((from, _)) if shift => from,
                                                    _ => (r, c),
                                                };
                                                st.cells = Some((from, (r, c)));
                                            }
                                        }
                                        ui.end_row();
                                    }
                                });
                            // A drag carried over other cells selects the
                            // block between and leaves the text alone.
                            let pointer = ui.input(|i| i.pointer.interact_pos());
                            let down = ui.input(|i| i.pointer.primary_down());
                            if let (Some((kind, from)), Some(p)) = (st.grid_drag, pointer) {
                                let to = match kind {
                                    pm::GridDrag::Cells => cells
                                        .iter()
                                        .find(|(_, rect)| rect.contains(p))
                                        .map(|(rc, _)| *rc),
                                    _ => heads
                                        .iter()
                                        .find(|(k, _, rect)| *k == kind && rect.contains(p))
                                        .map(|(_, i, _)| *i)
                                        .map(|i| match kind {
                                            pm::GridDrag::Records => (i, last_col),
                                            _ => (st.rows.len().saturating_sub(1), i),
                                        }),
                                };
                                if let Some(to) = to {
                                    if kind == pm::GridDrag::Cells && to != from {
                                        ui.memory_mut(|m| m.stop_text_input());
                                    }
                                    if kind != pm::GridDrag::Cells || to != from {
                                        st.cells = Some((from, to));
                                    }
                                }
                            }
                            if !down {
                                st.grid_drag = None;
                            }
                            // The block selected, tinted over its cells.
                            if let Some(sel) = st.cells {
                                let (rows, cols) = pm::range_bounds(sel);
                                let several =
                                    rows.end() > rows.start() || cols.end() > cols.start();
                                if several {
                                    let tint = ui.visuals().selection.bg_fill.gamma_multiply(0.35);
                                    for ((r, c), rect) in &cells {
                                        if rows.contains(r) && cols.contains(c) {
                                            ui.painter().rect_filled(rect.expand(1.0), 0.0, tint);
                                        }
                                    }
                                }
                            }
                        }
                    });
            });
        // Records: add and remove, and the navigation.
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!st.headers.is_empty(), egui::Button::new("+"))
                .rich_tip_always(crate::ui::tips::keyed("merge.add_record"))
                .clicked()
            {
                pm::add_record(st);
                st.current = st.rows.len() - 1;
            }
            if ui
                .add_enabled(!st.rows.is_empty(), egui::Button::new("\u{2212}"))
                .rich_tip_always(crate::ui::tips::keyed("merge.delete_record"))
                .clicked()
                && st.current < st.rows.len()
            {
                st.rows.remove(st.current);
                st.current = st.current.min(st.rows.len().saturating_sub(1));
                st.cells = None;
            }
            // The block selected in the grid: records added after it,
            // its records deleted, its cells emptied (#501).
            let sel = st.cells.filter(|_| !st.single && !st.rows.is_empty());
            ui.separator();
            if ui
                .add_enabled(sel.is_some(), egui::Button::new(tr("merge.sel_add")))
                .rich_tip_always(crate::ui::tips::pair("merge.sel_add", "merge.sel_add_tip"))
                .clicked()
            {
                if let Some(sel) = sel {
                    let (rows, cols) = pm::range_bounds(sel);
                    let n = rows.end() + 1 - rows.start();
                    pm::insert_records(st, rows.end() + 1, n);
                    let a = rows.end() + 1;
                    st.cells = Some(((a, *cols.start()), (a + n - 1, *cols.end())));
                    st.current = a;
                }
            }
            if ui
                .add_enabled(sel.is_some(), egui::Button::new(tr("merge.sel_delete")))
                .rich_tip_always(crate::ui::tips::pair(
                    "merge.sel_delete",
                    "merge.sel_delete_tip",
                ))
                .clicked()
            {
                if let Some(sel) = sel {
                    pm::delete_records(st, pm::range_bounds(sel).0);
                    st.cells = None;
                }
            }
            let clear_key = ui.memory(|m| m.focused().is_none())
                && ui.input(|i| i.key_pressed(egui::Key::Delete));
            if ui
                .add_enabled(sel.is_some(), egui::Button::new(tr("merge.sel_clear")))
                .rich_tip_always(crate::ui::tips::pair(
                    "merge.sel_clear",
                    "merge.sel_clear_tip",
                ))
                .clicked()
                || (clear_key && sel.is_some())
            {
                if let Some(sel) = sel {
                    pm::clear_cells(st, sel);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let n = st.rows.len();
                if ui
                    .add_enabled(n > 0, egui::Button::new(tr("merge.go_to")))
                    .rich_tip_always(crate::ui::tips::keyed("merge.go_to"))
                    .clicked()
                {
                    if let Ok(k) = st.go_to.trim().parse::<usize>() {
                        st.current = k.clamp(1, n.max(1)) - 1;
                    }
                }
                ui.add(
                    egui::TextEdit::singleline(&mut st.go_to)
                        .desired_width(60.0)
                        .hint_text(format!("{}", st.current + 1)),
                )
                .rich_tip(crate::ui::tips::pair(
                    "merge.go_to",
                    "merge.go_to_field_desc",
                ));
                if ui
                    .add_enabled(n > 0 && st.current + 1 < n, egui::Button::new(">|"))
                    .rich_tip_always(crate::ui::tips::keyed("merge.last_record"))
                    .clicked()
                {
                    st.current = n - 1;
                }
                if ui
                    .add_enabled(n > 0 && st.current + 1 < n, egui::Button::new(">"))
                    .rich_tip_always(crate::ui::tips::keyed("merge.next_record"))
                    .clicked()
                {
                    st.current += 1;
                }
                if ui
                    .add_enabled(st.current > 0, egui::Button::new("<"))
                    .rich_tip_always(crate::ui::tips::keyed("merge.previous_record"))
                    .clicked()
                {
                    st.current -= 1;
                }
                if ui
                    .add_enabled(st.current > 0, egui::Button::new("|<"))
                    .rich_tip_always(crate::ui::tips::keyed("merge.first_record"))
                    .clicked()
                {
                    st.current = 0;
                }
            });
        });
        if !st.headers.is_empty() {
            ui.label(
                egui::RichText::new(tr("dialog.print_merge_fields_hint"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
            ui.horizontal_wrapped(|ui| {
                for h in st.headers.clone() {
                    if ui
                        .small_button(format!("<{h}>"))
                        .rich_tip_with(|| format!("<{h}>\n{}", tr("merge.insert_field_desc")))
                        .clicked()
                    {
                        app.insert_text(&format!("<{h}>"));
                    }
                }
            });
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !st.headers.is_empty(),
                    egui::Button::new(tr("merge.save_data")),
                )
                .rich_tip_always(crate::ui::tips::keyed("merge.save_data"))
                .clicked()
            {
                let text = pm::to_csv(st);
                crate::files::Dialog::new()
                    .add_filter("CSV", &["csv"])
                    .set_file_name("merge.csv")
                    .save_file(app, move |app, path| {
                        if let Err(e) = crate::files::write(&path, text.into_bytes()) {
                            app.status = e.to_string();
                        }
                    });
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(tr("dialog.cancel"))
                    .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                    .clicked()
                {
                    *close = true;
                }
                // Finish keeps the data source and opens the Print Merge
                // docker, where the fields are put on the page and the
                // merge is made, as in the reference program (#437).
                if ui
                    .add_enabled(
                        !st.headers.is_empty(),
                        egui::Button::new(tr("merge.finish")),
                    )
                    .rich_tip_always(crate::ui::tips::keyed("merge.finish"))
                    .clicked()
                {
                    app.merge_state = Some(st.clone());
                    app.open_docker(crate::app::DockerTab::PrintMerge);
                    *close = true;
                }
            });
        });
        if !st.headers.is_empty() {
            app.merge_state = Some(st.clone());
        }
    });
}

fn trace_dialog(app: &mut App, ctx: &Context, st: &mut TraceState, close: &mut bool) {
    window(ctx, tr("dialog.trace")).show(ctx, |ui| {
        // Before and after: the bitmap and its trace, worked out on a small
        // copy whenever the settings change.
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            format!("{:?}", st.settings).hash(&mut h);
            h.finish()
        };
        let settings = st.settings.clone();
        crate::ui::bitmap_preview::before_after(ui, app, key, 200.0, |img, _| {
            let traced = crate::trace::trace(img, &settings);
            crate::trace::render_traced(
                &traced,
                img.width(),
                img.height(),
                crate::trace::last_stroke_width_px(),
            )
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for p in crate::trace::Preset::ALL {
                if ui
                    .selectable_label(st.preset == p, tr(p.key()))
                    .rich_tip(crate::ui::tips::keyed(p.key()))
                    .clicked()
                {
                    st.preset = p;
                    st.settings = app.settings.tracing.settings(p);
                    st.preview_count = None;
                }
            }
        });
        let s = &mut st.settings;
        crate::ui::tips::tip_over(ui, crate::ui::tips::keyed("dialog.detail"), |ui| {
            ui.add(crate::ui::Rail(
                egui::Slider::new(&mut s.detail, 0.0..=100.0).text(tr("dialog.detail")),
            ))
        });
        crate::ui::tips::tip_over(ui, crate::ui::tips::keyed("dialog.smoothing"), |ui| {
            ui.add(crate::ui::Rail(
                egui::Slider::new(&mut s.smoothing, 0.0..=100.0).text(tr("dialog.smoothing")),
            ))
        });
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::keyed("dialog.corner_smoothness"),
            |ui| {
                ui.add(crate::ui::Rail(
                    egui::Slider::new(&mut s.corner_smoothness, 0.0..=100.0)
                        .text(tr("dialog.corner_smoothness")),
                ))
            },
        );
        if !s.centerline {
            ui.horizontal(|ui| {
                ui.checkbox(&mut s.black_white, tr("dialog.black_and_white"))
                    .rich_tip(crate::ui::tips::pair(
                        "dialog.black_and_white",
                        "dialog.trace_bw_desc",
                    ));
                if !s.black_white {
                    crate::ui::tips::tip_over(
                        ui,
                        crate::ui::tips::pair("dialog.colors", "dialog.trace_colors_desc"),
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.label(tr("dialog.colors"));
                                ui.add(
                                    crate::ui::field::NumField::new(&mut s.colors).range(2..=256),
                                );
                            })
                        },
                    );
                }
            });
            ui.checkbox(&mut s.remove_background, tr("dialog.remove_background"))
                .rich_tip(crate::ui::tips::pair(
                    "dialog.remove_background",
                    "dialog.trace_remove_background_desc",
                ));
        }
        ui.checkbox(&mut s.delete_original, tr("dialog.delete_original"))
            .rich_tip(crate::ui::tips::keyed("dialog.delete_original"));
        ui.horizontal(|ui| {
            if ui
                .button(tr("dialog.preview"))
                .rich_tip(crate::ui::tips::pair(
                    "dialog.preview",
                    "dialog.trace_preview_desc",
                ))
                .clicked()
            {
                let mut count = 0;
                for sh in app.selected_shapes() {
                    if let ShapeKind::Bitmap { png, .. } = &sh.kind {
                        if let Some(img) = crate::bitmap_fx::decode(png) {
                            count += crate::trace::trace(&img, &st.settings).len();
                        }
                    }
                }
                st.preview_count = Some(count);
            }
            if let Some(n) = st.preview_count {
                ui.label(trf("dialog.trace_preview_n", &[("n", &n.to_string())]));
            }
        });
        if ok_cancel(ui, close) {
            let settings = st.settings.clone();
            app.trace_bitmaps(&settings);
            app.last_trace = Some((st.preset, settings));
        }
    });
}

fn paragraph_dialog(app: &mut App, ctx: &Context, which: &Dialog, close: &mut bool) {
    let title = match which {
        Dialog::TextTabs => tr("menu.text.tabs"),
        Dialog::TextColumns => tr("menu.text.columns"),
        Dialog::TextBullets => tr("menu.text.bullets"),
        _ => tr("menu.text.drop_cap"),
    };
    let title = title.trim_end_matches(['.', '\u{2026}']).to_string();
    // The selected text's paragraph, or with no text selected the
    // defaults new paragraph text gets.
    let target = app
        .selected_shapes()
        .into_iter()
        .find_map(|s| app.paragraph_style_of(&s).map(|(_, p)| (Some(s.id), p)));
    let (target, mut p) = target.unwrap_or_else(|| (None, app.doc().defaults.paragraph.clone()));
    let mut changed = false;
    let u = app.units;
    window(ctx, title).show(ctx, |ui| {
        match which {
            Dialog::TextTabs => {
                ui.label(tr("dialog.tab_stops"));
                let mut remove = None;
                for (i, t) in p.tabs.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        let tip = crate::ui::tips::pair("dialog.tab_stop", "dialog.tab_stop_desc");
                        changed |= unit_value(ui, &format!("{}", i + 1), &tip, t, u);
                        if ui
                            .small_button("x")
                            .rich_tip(crate::ui::tips::keyed("dialog.remove_tab"))
                            .clicked()
                        {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    p.tabs.remove(i);
                    changed = true;
                }
                if ui
                    .button(tr("dialog.add_tab"))
                    .rich_tip(crate::ui::tips::keyed("dialog.add_tab"))
                    .clicked()
                {
                    let last = p.tabs.last().copied().unwrap_or(0.0);
                    p.tabs.push(last + 12.7);
                    changed = true;
                }
            }
            Dialog::TextColumns => {
                crate::ui::tips::tip_over(ui, crate::ui::tips::keyed("dialog.columns"), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr("dialog.columns"));
                        changed |= ui
                            .add(crate::ui::field::NumField::new(&mut p.columns).range(1..=8))
                            .changed();
                    })
                });
                let tip = crate::ui::tips::keyed("dialog.gutter");
                changed |= unit_value(ui, &tr("dialog.gutter"), &tip, &mut p.gutter, u);
            }
            Dialog::TextBullets => {
                changed |= ui
                    .checkbox(&mut p.bullets, tr("dialog.use_bullets"))
                    .rich_tip(crate::ui::tips::keyed("dialog.use_bullets"))
                    .changed();
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.bullet_symbol"));
                    for b in [
                        "\u{2022}", "\u{25E6}", "\u{25AA}", "\u{2013}", "\u{2713}", "\u{27A4}",
                    ] {
                        if ui
                            .selectable_label(p.bullet_char == b, b)
                            .rich_tip(crate::ui::tips::keyed("dialog.bullet_symbol"))
                            .clicked()
                        {
                            p.bullet_char = b.to_string();
                            changed = true;
                        }
                    }
                });
                let tip = crate::ui::tips::keyed("dialog.bullet_indent");
                changed |= unit_value(
                    ui,
                    &tr("dialog.bullet_indent"),
                    &tip,
                    &mut p.bullet_indent,
                    u,
                );
            }
            _ => {
                let mut on = p.drop_cap_lines > 1;
                if ui
                    .checkbox(&mut on, tr("dialog.use_drop_cap"))
                    .rich_tip(crate::ui::tips::keyed("dialog.use_drop_cap"))
                    .changed()
                {
                    p.drop_cap_lines = if on { 3 } else { 0 };
                    changed = true;
                }
                if on {
                    crate::ui::tips::tip_over(
                        ui,
                        crate::ui::tips::keyed("dialog.lines_dropped"),
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.label(tr("dialog.lines_dropped"));
                                changed |= ui
                                    .add(
                                        crate::ui::field::NumField::new(&mut p.drop_cap_lines)
                                            .range(2..=10),
                                    )
                                    .changed();
                            })
                        },
                    );
                }
            }
        }
        if crate::ui::chrome::button_row(ui, &[&tr("dialog.close")], false).is_some() {
            *close = true;
        }
    });
    if changed {
        match target {
            Some(id) => app.set_paragraph_style(id, p),
            None => app.set_default_paragraph(p),
        }
    }
}

fn encode_dialog(app: &mut App, ctx: &Context, st: &mut EncodeState, close: &mut bool) {
    use crate::encode::Encoding;
    let original = app.selected_text_content();
    let mut apply = false;
    window(ctx, tr("dialog.encode")).show(ctx, |ui| {
        ui.set_min_width(420.0);
        ui.label(egui::RichText::new(tr("dialog.encode_hint")).color(Tokens::text_dim()));
        ui.add_space(4.0);
        let combo = |ui: &mut Ui, id: &str, id_key: &str, v: &mut Encoding| {
            ui.horizontal(|ui| {
                ui.label(tr(id_key));
                egui::ComboBox::from_id_salt(id)
                    .selected_text(v.name())
                    .show_ui(ui, |ui| {
                        for e in Encoding::ALL {
                            ui.selectable_value(v, e, e.name());
                        }
                    })
                    .response
                    .rich_tip(crate::ui::tips::keyed(id_key));
            });
        };
        combo(ui, "encode_from", "dialog.encode_from", &mut st.from);
        combo(ui, "encode_to", "dialog.encode_to", &mut st.to);
        ui.separator();
        ui.label(tr("dialog.preview"));
        let preview = crate::encode::reinterpret(&original, st.from, st.to);
        egui::ScrollArea::vertical()
            .max_height(160.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut preview.clone())
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let can = !original.is_empty() && st.from != st.to;
            if ui
                .add_enabled(can, egui::Button::new(tr("dialog.ok")))
                .rich_tip_always(crate::ui::tips::keyed("dialog.ok"))
                .clicked()
                || (can && enter_pressed(ui))
            {
                apply = true;
                *close = true;
            }
            if ui
                .button(tr("dialog.cancel"))
                .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                .clicked()
            {
                *close = true;
            }
        });
    });
    if apply {
        app.reencode_selected_text(st.from, st.to);
    }
}

fn border_grommet_dialog(
    app: &mut App,
    ctx: &Context,
    st: &mut crate::border_grommet::BorderGrommetState,
    close: &mut bool,
) {
    use crate::border_grommet::{BorderKind, Source};
    let units = app.units;
    let has_selection = crate::border_grommet::source_area(app, Source::Selection).is_some();
    if !has_selection {
        st.source = Source::Page;
    }
    let area = crate::border_grommet::source_area(app, st.source);
    let mut ok = false;
    let mut help = false;
    let length = |ui: &mut Ui, mm: &mut f64, min: f64, tip: &str| {
        let mut v = units.from_mm(*mm);
        if ui
            .add_sized(
                [90.0, 22.0],
                crate::ui::field::NumField::new(&mut v)
                    .speed(0.1)
                    .max_decimals(3)
                    .suffix(format!(" {}", units.short())),
            )
            .rich_tip_always(tip)
            .changed()
        {
            *mm = units.to_mm(v).max(min);
        }
    };
    window(ctx, tr("border_grommet.title")).show(ctx, |ui| {
        ui.horizontal_top(|ui| {
            border_grommet_preview(ui, st, area, units);
            ui.vertical(|ui| {
                ui.set_width(250.0);
                group(ui, &tr("border_grommet.source"), |ui| {
                    ui.horizontal(|ui| {
                        ui.radio_value(
                            &mut st.source,
                            Source::Page,
                            tr("border_grommet.source_page"),
                        )
                        .rich_tip(crate::ui::tips::keyed("border_grommet.source_page"));
                        ui.add_enabled_ui(has_selection, |ui| {
                            ui.radio_value(
                                &mut st.source,
                                Source::Selection,
                                tr("border_grommet.source_selection"),
                            )
                            .rich_tip_always(crate::ui::tips::keyed(
                                "border_grommet.source_selection",
                            ));
                        });
                    });
                });
                group(ui, &tr("border_grommet.border"), |ui| {
                    ui.checkbox(&mut st.add_border, tr("border_grommet.add_border"))
                        .rich_tip(crate::ui::tips::keyed("border_grommet.add_border"));
                    ui.add_enabled_ui(st.add_border, |ui| {
                        egui::Grid::new("bg_border").num_columns(2).show(ui, |ui| {
                            ui.label(tr("border_grommet.type"));
                            egui::ComboBox::from_id_salt("bg_kind")
                                .selected_text(tr(st.border.key()))
                                .show_ui(ui, |ui| {
                                    for k in BorderKind::ALL {
                                        ui.selectable_value(&mut st.border, k, tr(k.key()));
                                    }
                                })
                                .response
                                .rich_tip_always(crate::ui::tips::pair(
                                    "border_grommet.type",
                                    "border_grommet.border_type_desc",
                                ));
                            ui.end_row();
                            ui.label(tr("border_grommet.size"));
                            let tip = crate::ui::tips::pair(
                                "border_grommet.size",
                                "border_grommet.border_size_desc",
                            );
                            length(ui, &mut st.width_mm, 0.0, &tip);
                            ui.end_row();
                            if st.border == BorderKind::Solid {
                                ui.label(tr("border_grommet.border_color"));
                                crate::ui::color_picker::color_button(ui, &mut st.color);
                                ui.end_row();
                            }
                        });
                    });
                });
                group(ui, &tr("border_grommet.grommet"), |ui| {
                    ui.checkbox(&mut st.grommets, tr("border_grommet.add_grommets"))
                        .rich_tip(crate::ui::tips::keyed("border_grommet.add_grommets"));
                    ui.add_enabled_ui(st.grommets, |ui| {
                        let g = &mut st.grommet;
                        ui.add_enabled_ui(st.add_border, |ui| {
                            ui.checkbox(&mut g.use_border, tr("border_grommet.use_border"))
                                .rich_tip_always(crate::ui::tips::keyed(
                                    "border_grommet.use_border",
                                ));
                        });
                        egui::Grid::new("bg_grommet").num_columns(2).show(ui, |ui| {
                            ui.label(tr("border_grommet.size"));
                            let tip = crate::ui::tips::pair(
                                "border_grommet.size",
                                "border_grommet.grommet_size_desc",
                            );
                            length(ui, &mut g.diameter_mm, 0.1, &tip);
                            ui.end_row();
                            ui.label(tr("border_grommet.margin"));
                            ui.add_enabled_ui(!(g.use_border && st.add_border), |ui| {
                                let tip = crate::ui::tips::keyed("border_grommet.margin");
                                length(ui, &mut g.margin_mm, 0.0, &tip);
                            });
                            ui.end_row();
                        });
                        ui.separator();
                        ui.label(egui::RichText::new(tr("border_grommet.placement")).strong());
                        ui.horizontal(|ui| {
                            for (i, (on, key)) in [
                                (&mut g.corners, "border_grommet.place_corners"),
                                (&mut g.top, "border_grommet.place_top"),
                                (&mut g.bottom, "border_grommet.place_bottom"),
                                (&mut g.left, "border_grommet.place_left"),
                                (&mut g.right, "border_grommet.place_right"),
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                if placement_button(ui, i, *on)
                                    .rich_tip_always(crate::ui::tips::keyed(key))
                                    .clicked()
                                {
                                    *on = !*on;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut g.by_count, true, tr("border_grommet.number"))
                                .rich_tip_always(crate::ui::tips::keyed("border_grommet.number"));
                            ui.add_enabled_ui(g.by_count, |ui| {
                                ui.add(
                                    crate::ui::field::NumField::new(&mut g.count_per_edge)
                                        .range(2..=200),
                                )
                                .rich_tip_always(crate::ui::tips::keyed("border_grommet.number"));
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut g.by_count, false, tr("border_grommet.spacing"))
                                .rich_tip_always(crate::ui::tips::keyed("border_grommet.spacing"));
                            ui.add_enabled_ui(!g.by_count, |ui| {
                                let tip = crate::ui::tips::keyed("border_grommet.spacing");
                                length(ui, &mut g.spacing_mm, 1.0, &tip);
                            });
                        });
                    });
                });
            });
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            help = ui
                .add_sized([27.0, 27.0], egui::Button::new("?"))
                .rich_tip(crate::ui::tips::keyed("dialog.help_button"))
                .clicked();
            if ui
                .add_sized([100.0, 27.0], egui::Button::new(tr("dialog.reset")))
                .rich_tip(crate::ui::tips::keyed("dialog.reset"))
                .clicked()
            {
                *st = Default::default();
            }
            let can = st.does_something() && area.is_some();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_sized([100.0, 27.0], egui::Button::new(tr("dialog.cancel")))
                    .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                    .clicked()
                {
                    *close = true;
                }
                if ui
                    .add_enabled(
                        can,
                        egui::Button::new(tr("dialog.ok")).min_size(egui::vec2(100.0, 27.0)),
                    )
                    .rich_tip_always(crate::ui::tips::keyed("dialog.ok"))
                    .clicked()
                    || (can && enter_pressed(ui))
                {
                    ok = true;
                    *close = true;
                }
            });
        });
    });
    if help {
        app.open_dialog_help();
    }
    if ok {
        let n = crate::border_grommet::apply(app, st);
        app.status = trf("border_grommet.done", &[("n", &n.to_string())]);
    }
}

/// A titled box around a group of fields.
fn group(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(0xC8)))
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(title).strong());
            add(ui);
        });
    ui.add_space(4.0);
}

/// One of the placement buttons: a page with its corners (0), or its
/// top (1), bottom (2), left (3) or right (4) edge marked; pressed when on.
fn placement_button(ui: &mut Ui, which: usize, on: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
    let p = ui.painter();
    let fill = if on {
        crate::theme::Tokens::tool_hover()
    } else {
        egui::Color32::TRANSPARENT
    };
    let edge = if on || resp.hovered() {
        Tokens::ACCENT
    } else {
        egui::Color32::from_gray(0xB0)
    };
    p.rect(
        rect,
        2.0,
        fill,
        egui::Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    let page = rect.shrink(6.0);
    p.rect_stroke(
        page,
        0.0,
        egui::Stroke::new(1.0, Tokens::text()),
        egui::StrokeKind::Inside,
    );
    let dot = |p: &egui::Painter, x: f32, y: f32| {
        p.circle_filled(egui::pos2(x, y), 1.4, Tokens::text());
    };
    let (l, r, t, b) = (
        page.left() + 2.0,
        page.right() - 2.0,
        page.top() + 2.0,
        page.bottom() - 2.0,
    );
    let mid_x = (l + r) / 2.0;
    let mid_y = (t + b) / 2.0;
    match which {
        0 => [(l, t), (r, t), (l, b), (r, b)]
            .iter()
            .for_each(|&(x, y)| dot(p, x, y)),
        1 => [l, mid_x, r].iter().for_each(|&x| dot(p, x, t)),
        2 => [l, mid_x, r].iter().for_each(|&x| dot(p, x, b)),
        3 => [t, mid_y, b].iter().for_each(|&y| dot(p, l, y)),
        _ => [t, mid_y, b].iter().for_each(|&y| dot(p, r, y)),
    }
    resp
}

/// The preview: the final page, its border, the original area dashed and
/// the grommets, with the final size under it.
fn border_grommet_preview(
    ui: &mut Ui,
    st: &crate::border_grommet::BorderGrommetState,
    area: Option<Rect>,
    units: Units,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(560.0, 440.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect(
        rect,
        0.0,
        egui::Color32::WHITE,
        egui::Stroke::new(1.0, egui::Color32::BLACK),
        egui::StrokeKind::Inside,
    );
    let Some(area) = area else {
        return;
    };
    let b = st.border_mm();
    let size = st.final_size(area.size());
    let room = rect.shrink2(egui::vec2(30.0, 40.0));
    let scale = (room.width() as f64 / size.width.max(1e-6))
        .min(room.height() as f64 / size.height.max(1e-6)) as f32;
    let outer = egui::Rect::from_center_size(
        room.center(),
        egui::vec2(size.width as f32 * scale, size.height as f32 * scale),
    );
    // Final page millimetres (y up) to the preview.
    let at = |x: f64, y: f64| {
        egui::pos2(
            outer.left() + x as f32 * scale,
            outer.bottom() - y as f32 * scale,
        )
    };
    let band = match st.border {
        crate::border_grommet::BorderKind::Solid => {
            let [r, g, bl] = st.color.to_rgb8();
            egui::Color32::from_rgb(r, g, bl)
        }
        _ => egui::Color32::from_gray(0xE4),
    };
    p.rect(
        outer,
        0.0,
        if b > 0.0 { band } else { egui::Color32::WHITE },
        egui::Stroke::new(1.0, egui::Color32::from_gray(0x80)),
        egui::StrokeKind::Inside,
    );
    let inner = egui::Rect::from_two_pos(at(b, b), at(b + area.width(), b + area.height()));
    p.rect_filled(inner, 0.0, egui::Color32::WHITE);
    let blue = egui::Color32::from_rgb(0x33, 0x33, 0xFF);
    for (a, c) in [
        (inner.left_top(), inner.right_top()),
        (inner.right_top(), inner.right_bottom()),
        (inner.right_bottom(), inner.left_bottom()),
        (inner.left_bottom(), inner.left_top()),
    ] {
        p.extend(egui::Shape::dashed_line(
            &[a, c],
            egui::Stroke::new(1.0, blue),
            4.0,
            3.0,
        ));
    }
    let radius = (st.grommet.diameter_mm as f32 * scale / 2.0).max(2.0);
    for c in st.grommets_on(size) {
        p.circle_stroke(
            at(c.x, c.y),
            radius,
            egui::Stroke::new(1.0, egui::Color32::BLACK),
        );
    }
    let fmt = |mm: f64| format!("{:.2} {}", units.from_mm(mm), units.short());
    p.text(
        rect.left_bottom() + egui::vec2(10.0, -8.0),
        egui::Align2::LEFT_BOTTOM,
        trf(
            "border_grommet.size_text",
            &[("h", &fmt(size.height)), ("w", &fmt(size.width))],
        ),
        egui::FontId::proportional(12.0),
        Tokens::text(),
    );
}

fn font_manager_dialog(app: &mut App, ctx: &Context, st: &mut FontManagerState, close: &mut bool) {
    crate::ui::font_manager::dialog(app, ctx, st, close);
}

fn palette_editor_dialog(
    app: &mut App,
    ctx: &Context,
    st: &mut PaletteEditorState,
    close: &mut bool,
) {
    window(ctx, tr("menu.window.palette_editor")).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("dialog.palette"));
            let names: Vec<String> = app.palettes.iter().map(|p| p.display_name()).collect();
            egui::ComboBox::from_id_salt("pal_sel")
                .selected_text(names.get(st.palette).cloned().unwrap_or_default())
                .show_ui(ui, |ui| {
                    for (i, n) in names.iter().enumerate() {
                        if ui.selectable_label(st.palette == i, n).clicked() {
                            st.palette = i;
                            st.selected = None;
                        }
                    }
                })
                .response
                .rich_tip(crate::ui::tips::pair(
                    "dialog.palette",
                    "dialog.palette_pick_desc",
                ));
            if ui
                .button(tr("dialog.new_palette"))
                .rich_tip(crate::ui::tips::pair(
                    "dialog.new_palette_tip",
                    "dialog.new_palette_desc",
                ))
                .clicked()
            {
                app.palettes.push(crate::palette::Palette {
                    name: format!("Palette {}", app.palettes.len() + 1),
                    key: "",
                    colors: Vec::new(),
                    builtin: false,
                    path: None,
                });
                st.palette = app.palettes.len() - 1;
            }
        });
        let Some(pal) = app.palettes.get_mut(st.palette) else {
            *close = true;
            return;
        };
        if !pal.builtin {
            ui.horizontal(|ui| {
                ui.label(tr("dialog.name"));
                ui.text_edit_singleline(&mut pal.name)
                    .rich_tip(crate::ui::tips::pair(
                        "dialog.name",
                        "dialog.palette_name_desc",
                    ));
            });
        }
        let cols = 12;
        egui::ScrollArea::vertical()
            .max_height(180.0)
            .show(ui, |ui| {
                egui::Grid::new("pal_grid")
                    .spacing(egui::vec2(2.0, 2.0))
                    .show(ui, |ui| {
                        for (i, (_, c)) in pal.colors.iter().enumerate() {
                            let [r, g, b] = c.to_rgb8();
                            let (rect, resp) = ui
                                .allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
                            ui.painter()
                                .rect_filled(rect, 2.0, egui::Color32::from_rgb(r, g, b));
                            if st.selected == Some(i) {
                                ui.painter().rect_stroke(
                                    rect,
                                    2.0,
                                    egui::Stroke::new(2.0, egui::Color32::BLACK),
                                    egui::StrokeKind::Outside,
                                );
                            }
                            let resp = resp.rich_tip_with(|| {
                                format!("{}\n{}", pal.colors[i].0, tr("dialog.palette_swatch_desc"))
                            });
                            if resp.clicked() {
                                st.selected = Some(i);
                            }
                            if (i + 1) % cols == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
        if let Some(i) = st.selected {
            if let Some((name, c)) = pal.colors.get_mut(i) {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(name)
                        .rich_tip(crate::ui::tips::pair(
                            "dialog.color_name",
                            "dialog.color_name_desc",
                        ));
                    let mut edited = *c;
                    let changed = crate::ui::tips::tip_over(
                        ui,
                        crate::ui::tips::pair("dialog.color_value", "dialog.color_value_desc"),
                        |ui| crate::ui::color_picker::color_button(ui, &mut edited),
                    );
                    if changed && !pal.builtin {
                        *c = edited;
                    }
                    ui.label(crate::app::color_description(*c));
                });
            }
        }
        if !pal.builtin {
            ui.horizontal(|ui| {
                if ui
                    .button(tr("dialog.add_color"))
                    .rich_tip(crate::ui::tips::keyed("dialog.add_color"))
                    .clicked()
                {
                    let n = pal.colors.len() + 1;
                    pal.colors
                        .push((format!("Color {n}"), Color::rgb8(128, 128, 128)));
                    st.selected = Some(pal.colors.len() - 1);
                }
                if let Some(i) = st.selected {
                    if ui
                        .button(tr("dialog.remove_color"))
                        .rich_tip(crate::ui::tips::keyed("dialog.remove_color"))
                        .clicked()
                        && i < pal.colors.len()
                    {
                        pal.colors.remove(i);
                        st.selected = None;
                    }
                }
                if ui
                    .button(tr("dialog.save_palette"))
                    .rich_tip(crate::ui::tips::keyed("dialog.save_palette"))
                    .clicked()
                {
                    if let Some(p) = crate::files::Dialog::new()
                        .add_filter(
                            crate::palette::filter_label(&tr("file.palettes"), &["tdpal", "gpl"]),
                            &["tdpal", "gpl"],
                        )
                        .set_file_name(format!("{}.tdpal", pal.name))
                        .save_path()
                    {
                        if let Err(e) = crate::palette::save_palette(pal, &p) {
                            log::warn!("{e}");
                        } else {
                            pal.path = Some(p);
                        }
                    }
                }
            });
        } else {
            ui.label(
                egui::RichText::new(tr("dialog.builtin_palette_hint"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
        }
        if ui
            .button(tr("dialog.close"))
            .rich_tip(crate::ui::tips::keyed("dialog.close"))
            .clicked()
        {
            app.rebuild_palette();
            *close = true;
        }
    });
}

#[cfg(test)]
pub(crate) mod tests {

    /// Every page size preset has its own name and size (in either
    /// orientation), so a page's size names one preset; the favourites
    /// and the names saved settings refer to are all there.
    #[test]
    fn page_size_presets_are_distinct_and_keep_their_names() {
        use std::collections::HashSet;
        let all = paper_presets();
        assert!(all.len() > 100, "{}", all.len());
        let names: HashSet<_> = all.iter().map(|(n, _)| *n).collect();
        assert_eq!(names.len(), all.len(), "duplicate names");
        for (i, (a, sa)) in all.iter().enumerate() {
            assert!(sa.width > 0.0 && sa.height > 0.0, "{a}");
            for (b, sb) in &all[i + 1..] {
                assert!(!same_size(*sa, *sb), "{a} and {b} share a size");
            }
        }
        for n in FAVOURITE_PAPERS.iter().chain(&[
            "Tabloid",
            "Business card",
            "Web 1920x1080",
            "Web 1280x720",
            "Square 1080",
        ]) {
            assert!(names.contains(n), "{n}");
        }
        for (key, _) in paper_preset_groups() {
            assert_ne!(crate::i18n::tr(key), key.rsplit('.').next().unwrap_or(key));
        }
    }

    /// A page size finds its preset in either orientation, a saved size
    /// first, and a size no preset has is custom.
    #[test]
    fn a_page_size_finds_its_name() {
        let none: &[crate::settings::PageSizePreset] = &[];
        assert_eq!(
            paper_name(Size::new(210.0, 297.0), none).as_deref(),
            Some("A4")
        );
        assert_eq!(
            paper_name(Size::new(1189.0, 841.0), none).as_deref(),
            Some("A0")
        );
        assert_eq!(
            paper_name(Size::new(104.775, 241.3), none).as_deref(),
            Some("Envelope #10")
        );
        assert_eq!(paper_name(Size::new(123.0, 45.0), none), None);
        let saved = vec![crate::settings::PageSizePreset {
            name: "My label".into(),
            width_mm: 45.0,
            height_mm: 123.0,
        }];
        assert_eq!(
            paper_name(Size::new(123.0, 45.0), &saved).as_deref(),
            Some("My label")
        );
    }

    #[test]
    fn a_pixel_size_sets_the_resolution_that_gives_it() {
        // A 10 mm square at 300 dpi.
        assert_eq!(export_pixels(10.0, 10.0, 300.0), (119, 119));
        for px in [1u32, 16, 32, 48, 256, 900] {
            let dpi = dpi_for_pixels(10.0, px).expect("dpi");
            assert_eq!(export_pixels(10.0, 5.0, dpi).0, px, "{px}");
        }
        // An A4 page down to a few pixels: the lowest resolution.
        let dpi = dpi_for_pixels(210.0, 2).expect("dpi");
        assert_eq!(dpi, MIN_EXPORT_DPI);
        assert_eq!(export_pixels(210.0, 297.0, dpi), (9, 12));
        assert_eq!(dpi_for_pixels(0.0, 32), None);
        assert_eq!(dpi_for_pixels(f64::NAN, 32), None);
        assert_eq!(dpi_for_pixels(1.0, 1_000_000), Some(2400.0));
    }
    use super::*;

    /// Every dialog, as the menus open it.
    pub(crate) fn all_dialogs(app: &App) -> Vec<Dialog> {
        let layer = app.active_layer().unwrap_or(tracedraw_core::LayerId(1));
        vec![
            Dialog::NodeAlign {
                horizontal: true,
                vertical: false,
            },
            Dialog::BitmapBw(Default::default()),
            Dialog::BitmapDuotone(Default::default()),
            Dialog::BitmapPaletted(Default::default()),
            Dialog::RemoveBackground(Default::default()),
            Dialog::RenamePage {
                name: "Page".into(),
            },
            Dialog::GoToPage { page: 1 },
            Dialog::InsertPage {
                count: 1,
                after: true,
            },
            Dialog::DeletePage {
                from: 1,
                to: 1,
                through: true,
            },
            Dialog::duplicate_page(),
            Dialog::PageSize {
                width: 210.0,
                height: 297.0,
                all_pages: false,
            },
            Dialog::Options,
            Dialog::PageNumberSettings,
            Dialog::RenameLayer {
                layer,
                name: "Layer".into(),
            },
            Dialog::DocumentProperties(Box::default()),
            Dialog::OldCdrVersion { version: 9 },
            Dialog::ConfirmClose,
            Dialog::ConfirmResetSettings,
            Dialog::Updates,
            Dialog::Recover(vec![crate::recovery::Found {
                marker: "/nowhere/session-x.json".into(),
                backups: vec!["/nowhere/AutoBackup_of_art.tdraw".into()],
            }]),
            Dialog::Export(ExportState::default()),
            Dialog::Print(PrintState::default()),
            Dialog::PrintMerge(PrintMergeState::default()),
            Dialog::CreateTable {
                rows: 3,
                cols: 4,
                width: 75.0,
                height: 100.0,
            },
            Dialog::TableCount(crate::ui::table_bar::TableCountState {
                kind: crate::ui::table_bar::TableCountKind::InsertRows,
                count: 2,
                after: true,
            }),
            Dialog::TableCount(crate::ui::table_bar::TableCountState {
                kind: crate::ui::table_bar::TableCountKind::SplitCols,
                count: 3,
                after: false,
            }),
            Dialog::CreatePattern { vector: true },
            Dialog::CreatePattern { vector: false },
            Dialog::PasteSpecial(PasteSpecialState::default()),
            Dialog::Barcode(BarcodeState::default()),
            Dialog::ConvertToBitmap(crate::bitmap_modes::ConvertOptions {
                dpi: 150.0,
                mode: crate::bitmap_modes::ConvertMode::BlackWhite,
                dithered: true,
                ..Default::default()
            }),
            Dialog::StraightenImage(crate::ui::straighten_dialog::StraightenState {
                s: crate::straighten::Straighten {
                    angle: 5.0,
                    ..Default::default()
                },
                ..Default::default()
            }),
            Dialog::Resample(crate::resample::ResampleState::new(
                (400, 300),
                (100.0, 75.0),
            )),
            Dialog::InflateBitmap(InflateState {
                base: (40, 30),
                pw: 120.0,
                ph: 110.0,
                keep: false,
            }),
            Dialog::Trace(TraceState::new(
                crate::trace::Preset::Logo,
                &Default::default(),
            )),
            Dialog::Effect(crate::ui::effect_dialog::EffectState::new("emboss")),
            Dialog::Effect(crate::ui::effect_dialog::EffectState::new("tone_curve")),
            Dialog::Effect(crate::ui::effect_dialog::EffectState::new(
                "image_adjustments",
            )),
            Dialog::TextTabs,
            Dialog::TextColumns,
            Dialog::TextBullets,
            Dialog::TextDropCap,
            Dialog::TextStatistics,
            Dialog::ChangeCase(crate::ops2::CaseMode::Sentence),
            Dialog::WritingTools(Default::default()),
            Dialog::WritingTools(crate::ui::writing_tools::WritingToolsState::on_tab(1)),
            Dialog::WritingTools(crate::ui::writing_tools::WritingToolsState::on_tab(2)),
            Dialog::CustomHyphens(Default::default()),
            Dialog::Encode(EncodeState::default()),
            Dialog::BorderGrommet(crate::border_grommet::BorderGrommetState::default()),
            Dialog::ColorManagement(Default::default()),
            Dialog::FontManager(FontManagerState::default()),
            Dialog::PaletteEditor(PaletteEditorState::default()),
            Dialog::NewDocument(crate::new_document::NewDocState::from_settings(
                &Default::default(),
                "n".into(),
            )),
            Dialog::About,
            Dialog::WebNotice,
            Dialog::MacrosDesktopOnly,
            Dialog::GithubOpen(Default::default()),
            Dialog::GithubSave(Default::default()),
        ]
    }

    fn frame(ctx: &Context, app: &mut App, enter: bool) {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..Default::default()
        };
        if enter {
            input.events.push(egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
        let mut out = ctx.run_ui(input, |ui| show(app, &ui.ctx().clone()));
        out.textures_delta.clear();
    }

    /// One frame of `show` with `events`; where each text was drawn.
    fn frame_texts(
        ctx: &Context,
        app: &mut App,
        events: Vec<egui::Event>,
    ) -> Vec<(String, egui::Pos2)> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| show(app, &ui.ctx().clone()));
        out.textures_delta.clear();
        let mut texts = Vec::new();
        fn walk(s: &egui::Shape, texts: &mut Vec<(String, egui::Pos2)>) {
            match s {
                egui::Shape::Text(t) => texts.push((
                    t.galley.text().to_string(),
                    t.pos + t.galley.rect.center().to_vec2(),
                )),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, texts)),
                _ => {}
            }
        }
        for c in &out.shapes {
            walk(&c.shape, &mut texts);
        }
        texts
    }

    /// A drag from one cell of the Print Merge grid to another selects the
    /// block between; Delete empties it, Delete (the button) removes its
    /// records and Add puts as many empty ones after it (#501).
    #[test]
    fn the_merge_grid_selects_a_block_of_cells_by_dragging() {
        use egui::{Event, Modifiers, PointerButton};
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.dialog = Dialog::PrintMerge(PrintMergeState {
            headers: vec!["Name".into(), "City".into()],
            rows: vec![
                vec!["a1".into(), "b1".into()],
                vec!["a2".into(), "b2".into()],
                vec!["a3".into(), "b3".into()],
            ],
            ..Default::default()
        });
        let mut texts = Vec::new();
        for _ in 0..3 {
            texts = frame_texts(&ctx, &mut app, Vec::new());
        }
        let at = |texts: &[(String, egui::Pos2)], t: &str| {
            texts
                .iter()
                .find(|(s, _)| s == t)
                .map(|(_, p)| *p)
                .unwrap_or_else(|| panic!("{t} not drawn"))
        };
        let (from, to) = (at(&texts, "a1"), at(&texts, "b2"));
        let press = |p, pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        frame_texts(&ctx, &mut app, vec![Event::PointerMoved(from)]);
        frame_texts(&ctx, &mut app, vec![press(from, true)]);
        for i in 1..=6 {
            let p = from + (to - from) * (i as f32 / 6.0);
            frame_texts(&ctx, &mut app, vec![Event::PointerMoved(p)]);
        }
        frame_texts(&ctx, &mut app, vec![press(to, false)]);
        frame_texts(&ctx, &mut app, Vec::new());
        let st = |app: &App| match &app.dialog {
            Dialog::PrintMerge(st) => st.clone(),
            _ => panic!("the dialog closed"),
        };
        assert_eq!(st(&app).cells, Some(((0, 0), (1, 1))));
        let key = Event::Key {
            key: egui::Key::Delete,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        frame_texts(&ctx, &mut app, vec![key]);
        assert_eq!(
            st(&app).rows,
            vec![
                vec![String::new(), String::new()],
                vec![String::new(), String::new()],
                vec!["a3".to_string(), "b3".to_string()],
            ]
        );
        // The helpers the buttons run.
        let mut m = st(&app);
        crate::print_merge::insert_records(&mut m, 2, 2);
        assert_eq!(m.rows.len(), 5);
        assert_eq!(m.rows[4], vec!["a3".to_string(), "b3".to_string()]);
        crate::print_merge::delete_records(&mut m, 0..=3);
        assert_eq!(m.rows, vec![vec!["a3".to_string(), "b3".to_string()]]);
        crate::print_merge::delete_records(&mut m, 5..=9);
        assert_eq!(m.rows.len(), 1, "past the end deletes nothing");
    }

    #[test]
    fn create_new_table_takes_rows_columns_and_size() {
        let mut app = App::headless();
        app.dialog = Dialog::CreateTable {
            rows: 5,
            cols: 2,
            width: 80.0,
            height: 50.0,
        };
        let ctx = crate::theme::ui_context();
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, true);
        assert!(matches!(app.dialog, Dialog::None));
        let table = app
            .selection
            .first()
            .and_then(|id| app.doc().find_shape(*id))
            .and_then(|s| match &s.kind {
                ShapeKind::Table(t) => Some(t.clone()),
                _ => None,
            })
            .expect("a table");
        assert_eq!((table.row_heights.len(), table.col_widths.len()), (5, 2));
        let r = table.rect;
        assert!(
            (r.width() - 80.0).abs() < 1e-6 && (r.height() - 50.0).abs() < 1e-6,
            "{r:?}"
        );
        assert_eq!(app.table_size, (80.0, 50.0));
    }

    #[test]
    fn an_open_dialog_takes_the_clicks_outside_it() {
        let mut app = App::headless();
        app.dialog = Dialog::CreateTable {
            rows: 3,
            cols: 4,
            width: 75.0,
            height: 100.0,
        };
        let ctx = crate::theme::ui_context();
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, false);
        let blocker = egui::LayerId::new(egui::Order::Middle, egui::Id::new("modal_blocker"));
        assert_eq!(ctx.layer_id_at(egui::pos2(5.0, 5.0)), Some(blocker));
        assert_eq!(ctx.layer_id_at(egui::pos2(1390.0, 890.0)), Some(blocker));
        // The dialog itself is above it.
        let centre = ctx.layer_id_at(egui::pos2(700.0, 450.0)).expect("a layer");
        assert_eq!(centre.order, egui::Order::Foreground);
        app.dialog = Dialog::None;
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, false);
        assert_ne!(ctx.layer_id_at(egui::pos2(5.0, 5.0)), Some(blocker));
    }

    #[test]
    fn the_export_list_puts_the_common_formats_first() {
        let order = export_format_order();
        let ext = |i: Option<usize>| i.map(|i| EXPORT_FORMATS[i].1);
        let head: Vec<_> = order.iter().take(5).map(|i| ext(*i)).collect();
        assert_eq!(
            head,
            vec![Some("jpg"), Some("pdf"), Some("ai"), Some("png"), None]
        );
        let rest: Vec<&str> = order[5..].iter().filter_map(|i| ext(*i)).collect();
        let mut sorted = rest.clone();
        sorted.sort();
        assert_eq!(rest, sorted);
        assert_eq!(rest.len(), EXPORT_FORMATS.len());
        let jpg = export_format_label(3);
        assert!(
            jpg.starts_with("JPG - ") && jpg.ends_with("(*.jpg;*.jpeg)"),
            "{jpg}"
        );
        assert!(export_keeps_properties("pdf") && !export_keeps_properties("png"));
    }

    /// Every dialog can be drawn with a rectangle, text and a bitmap in the
    /// document, and Enter (OK) applies it without a panic.
    #[test]
    fn every_dialog_draws_and_accepts_enter() {
        let mut app = App::headless();
        let r = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.start_text(tracedraw_core::geometry::Point::new(20.0, 80.0), None);
        app.text_insert("Dialog test");
        app.finish_text();
        app.select(vec![r]);
        app.convert_to_bitmap(100.0, true);
        app.select_all();
        let ctx = crate::theme::ui_context();
        for dialog in all_dialogs(&app) {
            let name = format!("{dialog:?}");
            app.dialog = dialog;
            frame(&ctx, &mut app, false);
            frame(&ctx, &mut app, false);
            frame(&ctx, &mut app, true);
            frame(&ctx, &mut app, false);
            assert!(!app.doc().pages.is_empty(), "{name}: the document survived");
            app.dialog = Dialog::None;
            app.select_all();
        }
    }

    /// The Updates dialog draws in every state of a check and of an
    /// install in place.
    #[test]
    fn the_updates_dialog_draws_every_state() {
        use crate::updates::{InstallState, Package, Release, UpdateState};
        let mut app = App::headless();
        let ctx = crate::theme::ui_context();
        let release = Release {
            version: "9.0.0".into(),
            page: "https://example.org/r".into(),
            download: None,
            appimage: Some(Package {
                url: "https://example.org/a.AppImage".into(),
                sha256: "0".repeat(64),
                size: 3 << 20,
            }),
        };
        let checks = [
            UpdateState::Idle,
            UpdateState::Checking,
            UpdateState::Latest,
            UpdateState::Available(release),
            UpdateState::Failed("offline".into()),
        ];
        let installs = [
            InstallState::Idle,
            InstallState::Downloading {
                done: 1 << 20,
                total: 3 << 20,
            },
            InstallState::Installed("9.0.0".into()),
            InstallState::Failed("digest".into()),
        ];
        for check in &checks {
            for install in &installs {
                *app.updates.state.lock().expect("lock") = check.clone();
                *app.installer.state.lock().expect("lock") = install.clone();
                app.dialog = Dialog::Updates;
                frame(&ctx, &mut app, false);
                frame(&ctx, &mut app, false);
                assert!(
                    matches!(app.dialog, Dialog::Updates),
                    "{check:?} {install:?}: still open"
                );
            }
        }
    }
}
