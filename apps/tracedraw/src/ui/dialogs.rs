//! Dialogs: pages, layers, Options (multi-page), Document Properties,
//! Export, Print, Print Merge, Find and Replace, tables, QR codes,
//! bitmaps (convert, straighten, resample, inflate, trace),
//! text (tabs, columns, bullets, drop cap, statistics, spell check),
//! colour management, font manager, palette editor, confirmations.

use crate::app::{App, Units};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use egui::{Context, Ui};
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Rect, Size},
    Color, Command, Fill,
};

pub use crate::ui::options::OptionsPage;

#[derive(Debug, Clone, PartialEq)]
pub struct ExportState {
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
}

impl Default for ExportState {
    fn default() -> Self {
        ExportState {
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
        }
    }
}

pub const PDF_STANDARDS: [&str; 4] = ["PDF 1.4", "PDF/X-1a:2003", "PDF/X-3:2003", "PDF/X-4"];

impl ExportState {
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

pub const EXPORT_FORMATS: [(&str, &str); 16] = [
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
];

#[derive(Debug, Clone, PartialEq)]
pub struct PrintState {
    pub copies: u32,
    pub all_pages: bool,
    pub scale: f64,
    pub fit_to_page: bool,
    pub separations: bool,
    pub crop_marks: bool,
    pub registration_marks: bool,
    pub bleed: bool,
    pub mirror: bool,
    pub invert: bool,
}

impl Default for PrintState {
    fn default() -> Self {
        PrintState {
            copies: 1,
            all_pages: true,
            scale: 100.0,
            fit_to_page: false,
            separations: false,
            crop_marks: false,
            registration_marks: false,
            bleed: false,
            mirror: false,
            invert: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PrintMergeState {
    pub csv: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub path: Option<std::path::PathBuf>,
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct QrState {
    pub text: String,
    pub size_mm: f64,
    pub error: String,
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

#[derive(Debug, Clone, PartialEq)]
pub struct BarcodeState {
    pub symbology: crate::barcode::Symbology,
    pub text: String,
    pub height_mm: f64,
    pub module_mm: f64,
    pub show_text: bool,
    pub error: String,
}

impl Default for BarcodeState {
    fn default() -> Self {
        BarcodeState {
            symbology: crate::barcode::Symbology::Code128,
            text: "TRACEDRAW-2026".into(),
            height_mm: 15.0,
            module_mm: 0.33,
            show_text: true,
            error: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceState {
    pub preset: crate::trace::Preset,
    pub settings: crate::trace::Settings,
    pub preview_count: Option<usize>,
}

impl TraceState {
    pub fn new(preset: crate::trace::Preset) -> Self {
        TraceState {
            preset,
            settings: preset.settings(),
            preview_count: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpellState {
    pub unknown: Vec<(tracedraw_core::ShapeId, String)>,
    pub cursor: usize,
    pub replacement: String,
    pub checked: bool,
    pub dictionary_words: usize,
}

/// Text > Encode.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EncodeState {
    pub from: crate::encode::Encoding,
    pub to: crate::encode::Encoding,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThesaurusState {
    /// Word typed or taken from the selection.
    pub word: String,
    /// Word as it appears in the text (keeps its capitalisation on replace).
    pub original: String,
    pub target: Option<tracedraw_core::ShapeId>,
    pub looked_up: Option<String>,
    pub meanings: Vec<crate::thesaurus::Meaning>,
    pub selected: Option<String>,
    pub started: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GrammarState {
    pub findings: Vec<(tracedraw_core::ShapeId, crate::grammar::Finding)>,
    pub checked: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutocorrectState {
    pub new_from: String,
    pub new_to: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FontManagerState {
    pub filter: String,
    pub sample: String,
    pub selected: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PaletteEditorState {
    pub palette: usize,
    pub selected: Option<usize>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Dialog {
    None,
    /// Bitmaps > Mode > Black and White, Duotone and Paletted.
    BitmapBw(crate::bitmap_modes::BwSettings),
    BitmapDuotone(crate::ui::bitmap_dialogs::DuotoneState),
    BitmapPaletted(crate::bitmap_modes::PalettedSettings),
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
    PageSize {
        width: f64,
        height: f64,
        all_pages: bool,
    },
    Options,
    PageNumberSettings,
    Symmetry,
    RenameLayer {
        layer: tracedraw_core::LayerId,
        name: String,
    },
    DocumentProperties,
    ConfirmClose,
    Export(ExportState),
    Print(PrintState),
    PrintMerge(PrintMergeState),
    CreateTable {
        rows: u32,
        cols: u32,
    },
    QrCode(QrState),
    PasteSpecial(PasteSpecialState),
    Barcode(BarcodeState),
    ConvertToBitmap(crate::bitmap_modes::ConvertOptions),
    StraightenImage(crate::ui::straighten_dialog::StraightenState),
    Resample {
        dpi: f64,
    },
    InflateBitmap(InflateState),
    Trace(TraceState),
    /// A bitmap effect's settings (Effects menu, FX section).
    Effect(crate::ui::effect_dialog::EffectState),
    TextTabs,
    TextColumns,
    TextBullets,
    TextDropCap,
    TextStatistics,
    SpellCheck(SpellState),
    Thesaurus(ThesaurusState),
    Encode(EncodeState),
    Grammar(GrammarState),
    Autocorrect(AutocorrectState),
    BorderGrommet(crate::border_grommet::BorderGrommetState),
    ColorManagement,
    FontManager(FontManagerState),
    PaletteEditor(PaletteEditorState),
    NewDocument(crate::new_document::NewDocState),
    About,
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
                egui::ComboBox::from_id_salt("convert_dpi")
                    .selected_text(format!("{:.0}", o.dpi))
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        for d in [72.0, 96.0, 150.0, 200.0, 300.0, 400.0, 600.0] {
                            ui.selectable_value(&mut o.dpi, d, format!("{d:.0}"));
                        }
                    });
                ui.add(
                    egui::DragValue::new(&mut o.dpi)
                        .range(36.0..=2400.0)
                        .suffix(" dpi"),
                );
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
                });
            ui.end_row();
        });
    ui.add_space(4.0);
    ui.add_enabled(
        o.mode.can_dither(),
        egui::Checkbox::new(&mut o.dithered, tr("dialog.dithered")),
    );
    ui.checkbox(&mut o.overprint_black, tr("dialog.overprint_black"));
    ui.checkbox(&mut o.anti_alias, tr("dialog.anti_aliasing"));
    ui.checkbox(&mut o.transparent, tr("dialog.transparent_background"));
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
            .color(Tokens::TEXT_DIM),
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
                        egui::DragValue::new(&mut p)
                            .range(base as f64..=base as f64 * 10.0)
                            .speed(1.0),
                    )
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
                        egui::DragValue::new(&mut pct)
                            .range(100.0..=1000.0)
                            .speed(0.5)
                            .max_decimals(1),
                    )
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
        .changed()
        && st.keep
    {
        st.ph = st.pw;
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

/// Names of the dialogs' multi-line fields, where Enter inserts a line.
const MULTILINE_NAMES: [&str; 2] = ["dialog_multiline_qr", "dialog_multiline_notes"];

fn multiline_id(i: usize) -> egui::Id {
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

fn unit_value(ui: &mut Ui, label: &str, mm: &mut f64, units: Units) -> bool {
    let mut v = units.from_mm(*mm);
    let r = ui
        .horizontal(|ui| {
            ui.label(label);
            ui.add(
                egui::DragValue::new(&mut v)
                    .speed(0.5)
                    .suffix(format!(" {}", units.short())),
            )
            .changed()
        })
        .inner;
    if r {
        *mm = units.to_mm(v);
    }
    r
}

pub fn show(app: &mut App, ctx: &Context) {
    // An options dialog replaced by another dialog keeps its changes.
    if !matches!(app.dialog, Dialog::Options) {
        app.options_snapshot = None;
    }
    let mut dialog = std::mem::replace(&mut app.dialog, Dialog::None);
    let mut close = false;
    match &mut dialog {
        Dialog::None => {}
        Dialog::BitmapBw(s) => crate::ui::bitmap_dialogs::bw_dialog(app, ctx, s, &mut close),
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
                ui.checkbox(horizontal, tr("dialog.align_horizontal"));
                ui.checkbox(vertical, tr("dialog.align_vertical"));
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
                    ui.text_edit_singleline(name);
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
                    ui.add(egui::DragValue::new(page).range(1..=n.max(1)));
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
                    ui.add(egui::DragValue::new(count).range(1..=999));
                });
                ui.horizontal(|ui| {
                    ui.radio_value(after, false, tr("dialog.before"));
                    ui.radio_value(after, true, tr("dialog.after"));
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
        Dialog::PageSize {
            width,
            height,
            all_pages,
        } => {
            window(ctx, tr("dialog.page_size")).show(ctx, |ui| {
                let u = app.units;
                unit_value(ui, &tr("dialog.width"), width, u);
                unit_value(ui, &tr("dialog.height"), height, u);
                ui.horizontal_wrapped(|ui| {
                    for (n, s) in paper_presets() {
                        if ui.button(n).clicked() {
                            *width = s.width;
                            *height = s.height;
                        }
                    }
                    if ui.button(tr("dialog.swap")).clicked() {
                        std::mem::swap(width, height);
                    }
                });
                ui.checkbox(all_pages, tr("dialog.apply_all_pages"));
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
                    ui.text_edit_singleline(name);
                });
                if ok_cancel(ui, &mut close) {
                    app.run(Command::RenameLayer {
                        layer: *layer,
                        name: name.clone(),
                    });
                }
            });
        }
        Dialog::Options => crate::ui::options::options_dialog(app, ctx, &mut close),
        Dialog::DocumentProperties => document_properties(app, ctx, &mut close),
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
        Dialog::Export(st) => export_dialog(app, ctx, st, &mut close),
        Dialog::Print(st) => print_dialog(app, ctx, st, &mut close),
        Dialog::PrintMerge(st) => print_merge_dialog(app, ctx, st, &mut close),
        Dialog::CreateTable { rows, cols } => {
            window(ctx, tr("dialog.create_table")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.rows"));
                    ui.add(egui::DragValue::new(rows).range(1..=200));
                    ui.label(tr("dialog.columns"));
                    ui.add(egui::DragValue::new(cols).range(1..=200));
                });
                if ok_cancel(ui, &mut close) {
                    app.table_rows = *rows;
                    app.table_cols = *cols;
                    let page = app.page_rect();
                    let w = (page.width() * 0.6).max(10.0);
                    let h = (*rows as f64 * 10.0).min(page.height() * 0.6);
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
                        egui::RichText::new(tr("dialog.clipboard_empty")).color(Tokens::TEXT_DIM),
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
        Dialog::Barcode(st) => {
            window(ctx, tr("dialog.barcode")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.barcode_type"));
                    egui::ComboBox::from_id_salt("barcode_sym")
                        .selected_text(st.symbology.name())
                        .show_ui(ui, |ui| {
                            for s in crate::barcode::Symbology::ALL {
                                ui.selectable_value(&mut st.symbology, s, s.name());
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.barcode_text"));
                    ui.text_edit_singleline(&mut st.text);
                });
                unit_value(ui, &tr("dialog.bar_height"), &mut st.height_mm, app.units);
                unit_value(ui, &tr("dialog.module_width"), &mut st.module_mm, app.units);
                ui.checkbox(&mut st.show_text, tr("dialog.show_text"));
                if !st.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &st.error);
                }
                if ok_cancel(ui, &mut close) {
                    match crate::barcode::encode(st.symbology, &st.text) {
                        Ok(modules) => {
                            let m = st.module_mm.max(0.05);
                            let h = st.height_mm.max(1.0);
                            let total_w = modules.len() as f64 * m;
                            let page = app.page_rect();
                            let c = page.center();
                            let x0 = c.x - total_w / 2.0;
                            let y0 = c.y - h / 2.0;
                            let mut path = tracedraw_core::geometry::BezPath::new();
                            for (start, width) in crate::barcode::bars(&modules) {
                                let r = Rect::new(
                                    x0 + start as f64 * m,
                                    y0,
                                    x0 + (start + width) as f64 * m,
                                    y0 + h,
                                );
                                path.extend(tracedraw_core::geometry::rect_path(r, 0.0));
                            }
                            let Some(layer) = app.active_layer() else {
                                return;
                            };
                            let mut cmds = Vec::new();
                            let mut ids = Vec::new();
                            let bars_id = app.engine.new_shape_id();
                            let mut bars = tracedraw_core::document::Shape::new(
                                bars_id,
                                ShapeKind::Path { path, closed: true },
                            );
                            bars.fill = Fill::Solid(Color::BLACK);
                            bars.stroke = None;
                            bars.name = Some(st.symbology.name().to_string());
                            ids.push(bars_id);
                            cmds.push(Command::AddShape { layer, shape: bars });
                            if st.show_text {
                                let text_id = app.engine.new_shape_id();
                                let size_pt = (h * 0.2 * 72.0 / 25.4).clamp(5.0, 12.0);
                                let mut label = tracedraw_core::document::Shape::new(
                                    text_id,
                                    ShapeKind::Text {
                                        spans: vec![tracedraw_core::TextSpan::new(
                                            st.text.clone(),
                                            app.text_font.clone(),
                                            size_pt,
                                        )],
                                        origin: tracedraw_core::geometry::Point::new(
                                            c.x,
                                            y0 - size_pt * 25.4 / 72.0 * 1.1,
                                        ),
                                        frame: None,
                                        align: tracedraw_core::TextAlign::Center,
                                        para: Default::default(),
                                        on_path: None,
                                    },
                                );
                                label.fill = Fill::Solid(Color::BLACK);
                                label.stroke = None;
                                ids.push(text_id);
                                cmds.push(Command::AddShape {
                                    layer,
                                    shape: label,
                                });
                                cmds.push(Command::Group {
                                    shapes: ids.clone(),
                                });
                            }
                            if let Err(e) = app.engine.run_batch("Insert Barcode", &cmds) {
                                app.status = e.to_string();
                            }
                            let last = app
                                .doc()
                                .page(app.page)
                                .ok()
                                .and_then(|p| p.layers.iter().find(|l| l.id == layer))
                                .and_then(|l| l.shapes.last())
                                .map(|s| s.id);
                            if let Some(id) = last {
                                app.select(vec![id]);
                            }
                        }
                        Err(key) => {
                            st.error = tr(key);
                            close = false;
                        }
                    }
                }
            });
        }
        Dialog::QrCode(st) => {
            window(ctx, tr("dialog.qr_code")).show(ctx, |ui| {
                ui.label(tr("dialog.qr_text"));
                ui.add(egui::TextEdit::multiline(&mut st.text).id(multiline_id(0)));
                if st.size_mm <= 0.0 {
                    st.size_mm = 30.0;
                }
                unit_value(ui, &tr("dialog.size"), &mut st.size_mm, app.units);
                if !st.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &st.error);
                }
                if ok_cancel(ui, &mut close) {
                    match qr_path(&st.text, st.size_mm) {
                        Ok(path) => {
                            let page = app.page_rect();
                            let c = page.center();
                            let p = tracedraw_core::geometry::Affine::translate((
                                c.x - st.size_mm / 2.0,
                                c.y - st.size_mm / 2.0,
                            )) * path;
                            if let Some(id) = app.new_shape(ShapeKind::Path {
                                path: p,
                                closed: true,
                            }) {
                                app.run(Command::SetFill {
                                    shapes: vec![id],
                                    fill: Fill::Solid(Color::BLACK),
                                });
                                app.run(Command::SetStroke {
                                    shapes: vec![id],
                                    stroke: None,
                                });
                                app.run(Command::SetShapeName {
                                    shape: id,
                                    name: Some("QR code".into()),
                                });
                                app.select(vec![id]);
                            }
                        }
                        Err(e) => {
                            st.error = e;
                            close = false;
                        }
                    }
                }
            });
        }
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
        Dialog::Resample { dpi } => {
            window(ctx, tr("dialog.resample")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.resolution"));
                    ui.add(
                        egui::DragValue::new(dpi)
                            .range(10.0..=2400.0)
                            .suffix(" dpi"),
                    );
                });
                if ok_cancel(ui, &mut close) {
                    app.resample_bitmap(*dpi);
                }
            });
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
        Dialog::Effect(st) => crate::ui::effect_dialog::effect_dialog(app, ctx, st, &mut close),
        Dialog::TextTabs | Dialog::TextColumns | Dialog::TextBullets | Dialog::TextDropCap => {
            paragraph_dialog(app, ctx, &dialog, &mut close)
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
        Dialog::SpellCheck(st) => spell_dialog(app, ctx, st, &mut close),
        Dialog::Thesaurus(st) => thesaurus_dialog(app, ctx, st, &mut close),
        Dialog::Encode(st) => encode_dialog(app, ctx, st, &mut close),
        Dialog::Grammar(st) => grammar_dialog(app, ctx, st, &mut close),
        Dialog::Autocorrect(st) => autocorrect_dialog(app, ctx, st, &mut close),
        Dialog::BorderGrommet(st) => border_grommet_dialog(app, ctx, st, &mut close),
        Dialog::ColorManagement => color_management_dialog(app, ctx, &mut close),
        Dialog::FontManager(st) => font_manager_dialog(app, ctx, st, &mut close),
        Dialog::PaletteEditor(st) => palette_editor_dialog(app, ctx, st, &mut close),
        Dialog::NewDocument(st) => {
            crate::new_document::new_document_dialog(app, ctx, st, &mut close)
        }
        Dialog::Symmetry => {
            window(ctx, tr("dialog.symmetry")).show(ctx, |ui| match app.selected_symmetry() {
                Some((_, center, angle, lines)) => {
                    let (mut c, mut a, mut n) = (center, angle, lines);
                    let u = app.units;
                    let mut changed = false;
                    egui::Grid::new("symmetry").num_columns(2).show(ui, |ui| {
                        ui.label(tr("dialog.mirror_lines"));
                        changed |= ui.add(egui::DragValue::new(&mut n).range(1..=12)).changed();
                        ui.end_row();
                        ui.label(tr("dialog.mirror_angle"));
                        changed |= ui
                            .add(egui::DragValue::new(&mut a).speed(1.0).suffix("°"))
                            .changed();
                        ui.end_row();
                        ui.label(tr("dialog.mirror_center"));
                        ui.horizontal(|ui| {
                            let (mut x, mut y) = (u.from_mm(c.x), u.from_mm(c.y));
                            let cx = ui
                                .add(egui::DragValue::new(&mut x).speed(0.5).prefix("x "))
                                .changed();
                            let cy = ui
                                .add(egui::DragValue::new(&mut y).speed(0.5).prefix("y "))
                                .changed();
                            if cx || cy {
                                c = tracedraw_core::geometry::Point::new(u.to_mm(x), u.to_mm(y));
                                changed = true;
                            }
                        });
                        ui.end_row();
                    });
                    if changed {
                        app.set_symmetry(c, a, n);
                    }
                    ui.label(
                        egui::RichText::new(tr("dialog.symmetry_hint"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                    ui.horizontal(|ui| {
                        if ui.button(tr("dialog.symmetry_bake")).clicked() {
                            app.flatten_effects();
                            close = true;
                        }
                        if ui.button(tr("menu.object.symmetry_remove")).clicked() {
                            app.remove_symmetry();
                            close = true;
                        }
                        if ui.button(tr("dialog.close")).clicked() {
                            close = true;
                        }
                    });
                }
                None => {
                    ui.label(tr("docker.no_objects_selected"));
                    if crate::ui::chrome::button_row(ui, &[&tr("dialog.close")], false).is_some() {
                        close = true;
                    }
                }
            });
        }
        Dialog::PageNumberSettings => {
            window(ctx, tr("dialog.page_number_settings")).show(ctx, |ui| {
                let pn = &mut app.page_numbers;
                egui::Grid::new("page_number_settings")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label(tr("dialog.start_at"));
                        ui.add(egui::DragValue::new(&mut pn.start_at).range(-999..=9999));
                        ui.end_row();
                        ui.label(tr("dialog.prefix"));
                        ui.text_edit_singleline(&mut pn.prefix);
                        ui.end_row();
                        ui.label(tr("dialog.suffix"));
                        ui.text_edit_singleline(&mut pn.suffix);
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
                            });
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
                            });
                        ui.end_row();
                        ui.label(tr("dialog.size"));
                        ui.add(
                            egui::DragValue::new(&mut pn.size_pt)
                                .range(4.0..=200.0)
                                .suffix(" pt"),
                        );
                        ui.end_row();
                    });
                ui.label(
                    egui::RichText::new(format!("{}  {}", pn.label(0), pn.label(1)))
                        .color(Tokens::TEXT_DIM),
                );
                if crate::ui::chrome::button_row(ui, &[&tr("dialog.close")], false).is_some() {
                    close = true;
                }
            });
        }
        Dialog::About => {
            window(ctx, tr("dialog.about")).show(ctx, |ui| {
                ui.heading("TraceDraw");
                ui.label(format!(
                    "{} {}",
                    tr("dialog.version"),
                    env!("CARGO_PKG_VERSION")
                ));
                ui.label(tr("dialog.about_text"));
                ui.add_space(6.0);
                ui.label(trf(
                    "dialog.about_copyright",
                    &[("name", env!("CARGO_PKG_AUTHORS"))],
                ));
                ui.label(tr("dialog.about_ai"));
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.about_source"));
                    let repo = env!("CARGO_PKG_REPOSITORY");
                    if ui.link(repo).clicked() {
                        app.open_url(repo);
                    }
                });
                ui.add_space(6.0);
                if crate::ui::chrome::button_row(ui, &[&tr("dialog.close")], false).is_some() {
                    close = true;
                }
            });
        }
    }
    // A dialog's close button closes it like Esc.
    if crate::ui::chrome::take_close_request(ctx) {
        close = true;
    }
    // A closing dialog may have opened the next one (the next unsaved
    // drawing's question); keep that.
    if !close {
        app.dialog = dialog;
    }
}

pub fn paper_presets() -> Vec<(&'static str, Size)> {
    vec![
        ("A4", tracedraw_core::document::paper::A4),
        ("A3", tracedraw_core::document::paper::A3),
        ("A5", Size::new(148.0, 210.0)),
        ("Letter", tracedraw_core::document::paper::LETTER),
        ("Legal", Size::new(215.9, 355.6)),
        ("Tabloid", Size::new(279.4, 431.8)),
        ("Business card", Size::new(90.0, 50.0)),
        (
            "Web 1920x1080",
            Size::new(1920.0 * 25.4 / 96.0, 1080.0 * 25.4 / 96.0),
        ),
        (
            "Web 1280x720",
            Size::new(1280.0 * 25.4 / 96.0, 720.0 * 25.4 / 96.0),
        ),
        (
            "Square 1080",
            Size::new(1080.0 * 25.4 / 96.0, 1080.0 * 25.4 / 96.0),
        ),
    ]
}

/// A QR code as one even-odd path of module squares.
fn qr_path(text: &str, size_mm: f64) -> Result<tracedraw_core::BezPath, String> {
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

/// Document Options > Page Size, laid out.
pub(crate) fn page_size_page(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.radio_value(
            &mut app.options_labels,
            false,
            tr("options.page_size_radio"),
        );
        ui.radio_value(&mut app.options_labels, true, tr("options.label_presets"));
    });
    ui.add_space(4.0);
    if app.options_labels {
        ui.label(tr("options.size_and_orientation"));
        ui.separator();
        ui.label(
            egui::RichText::new(tr("options.label_hint"))
                .color(Tokens::TEXT_DIM)
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
            });
        if ui.button(tr("options.apply_labels")).clicked() {
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
    let presets = paper_presets();
    let current = presets
        .iter()
        .find(|(_, ps)| {
            ((ps.width - w).abs() < 0.05 && (ps.height - h).abs() < 0.05)
                || ((ps.width - h).abs() < 0.05 && (ps.height - w).abs() < 0.05)
        })
        .map(|(n, _)| n.to_string())
        .unwrap_or_else(|| tr("options.custom_size"));
    egui::Grid::new("page_size_grid")
        .num_columns(3)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(tr("options.size"));
            });
            egui::ComboBox::from_id_salt("page_size_preset")
                .selected_text(current)
                .width(230.0)
                .show_ui(ui, |ui| {
                    for (n, ps) in &presets {
                        if ui.selectable_label(false, *n).clicked() {
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
                    }
                });
            ui.label("");
            ui.end_row();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(tr("dialog.width"));
            });
            let mut wv = app.units.from_mm(w);
            if ui
                .add_sized(
                    [100.0, 22.0],
                    egui::DragValue::new(&mut wv).speed(0.5).max_decimals(3),
                )
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
                });
            ui.end_row();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(tr("dialog.height"));
            });
            let mut hv = app.units.from_mm(h);
            if ui
                .add_sized(
                    [100.0, 22.0],
                    egui::DragValue::new(&mut hv).speed(0.5).max_decimals(3),
                )
                .changed()
            {
                h = app.units.to_mm(hv);
                changed = true;
            }
            ui.horizontal(|ui| {
                let portrait = h >= w;
                if ui
                    .add(egui::Button::new("\u{25af}").selected(portrait))
                    .on_hover_text(tr("options.portrait"))
                    .clicked()
                    && !portrait
                {
                    std::mem::swap(&mut w, &mut h);
                    changed = true;
                }
                if ui
                    .add(egui::Button::new("\u{25ad}").selected(!portrait))
                    .on_hover_text(tr("options.landscape"))
                    .clicked()
                    && portrait
                {
                    std::mem::swap(&mut w, &mut h);
                    changed = true;
                }
            });
            ui.end_row();
        });
    ui.add_space(4.0);
    ui.indent("page_size_checks", |ui| {
        ui.checkbox(
            &mut app.options_current_only,
            tr("options.apply_current_only"),
        );
        ui.checkbox(&mut app.show_page_border, tr("options.show_page_border"));
        ui.add_space(6.0);
        if ui
            .add_sized(
                [150.0, 26.0],
                egui::Button::new(tr("options.add_page_frame")),
            )
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
            });
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
                egui::DragValue::new(&mut b).speed(0.1).max_decimals(2),
            )
            .changed()
        {
            md.bleed = app.units.to_mm(b).max(0.0);
            app.run(Command::SetMetadata { metadata: md });
        }
    });
    ui.indent("bleed_checks", |ui| {
        ui.add_space(2.0);
        ui.checkbox(&mut app.show_bleed, tr("options.show_bleed"));
    });
}

fn document_properties(app: &mut App, ctx: &Context, close: &mut bool) {
    window(ctx, tr("menu.file.document_properties")).show(ctx, |ui| {
        let mut md = app.doc().metadata.clone();
        let mut title = app.doc().title.clone();
        let mut changed = false;
        egui::Grid::new("docprops").num_columns(2).show(ui, |ui| {
            ui.label(tr("dialog.title"));
            if ui.text_edit_singleline(&mut title).lost_focus() {
                app.run(Command::SetTitle {
                    title: title.clone(),
                });
            }
            ui.end_row();
            for (k, v) in [
                ("dialog.author", &mut md.author),
                ("dialog.subject", &mut md.subject),
                ("dialog.keywords", &mut md.keywords),
                ("dialog.copyright", &mut md.copyright),
            ] {
                ui.label(tr(k));
                changed |= ui.text_edit_singleline(v).lost_focus();
                ui.end_row();
            }
            ui.label(tr("dialog.notes"));
            changed |= ui
                .add(egui::TextEdit::multiline(&mut md.notes).id(multiline_id(1)))
                .lost_focus();
            ui.end_row();
            ui.label(tr("dialog.rating"));
            ui.horizontal(|ui| {
                for i in 1..=5u8 {
                    if ui.selectable_label(md.rating >= i, "\u{2605}").clicked() {
                        md.rating = i;
                        changed = true;
                    }
                }
            });
            ui.end_row();
        });
        if changed {
            app.run(Command::SetMetadata {
                metadata: md.clone(),
            });
        }
        ui.separator();
        let doc = app.doc();
        ui.label(trf(
            "dialog.pages_n",
            &[("n", &doc.pages.len().to_string())],
        ));
        let objects: usize = doc.all_layers().map(|l| l.shapes.len()).sum();
        ui.label(trf("dialog.objects_n", &[("n", &objects.to_string())]));
        if let Some(f) = &app.file {
            ui.label(format!("{}: {}", tr("dialog.file"), f.display()));
        }
        ui.label(format!(
            "{}: {} / {}",
            tr("dialog.color_profiles"),
            app.settings.color.rgb_profile,
            app.settings.color.cmyk_profile
        ));
        let mut fonts = std::collections::BTreeSet::new();
        for s in doc.all_layers().flat_map(|l| &l.shapes) {
            if let ShapeKind::Text { spans, .. } = &s.kind {
                for sp in spans {
                    fonts.insert(sp.font_family.clone());
                }
            }
        }
        if !fonts.is_empty() {
            ui.label(format!(
                "{}: {}",
                tr("dialog.fonts"),
                fonts.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
        if crate::ui::chrome::button_row(ui, &[&tr("dialog.close")], false).is_some() {
            *close = true;
        }
    });
}

// ----- Export ------------------------------------------------------------------

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
        ui.horizontal(|ui| {
            ui.label(tr("dialog.format"));
            egui::ComboBox::from_id_salt("exp_fmt")
                .selected_text(EXPORT_FORMATS[st.format].0)
                .show_ui(ui, |ui| {
                    for (i, (n, _)) in EXPORT_FORMATS.iter().enumerate() {
                        if ui.selectable_label(st.format == i, *n).clicked() {
                            st.format = i;
                        }
                    }
                });
        });
        let raster = (2..=7).contains(&st.format) || st.format == 15;
        if raster {
            ui.horizontal(|ui| {
                ui.label(tr("dialog.resolution"));
                ui.add(
                    egui::DragValue::new(&mut st.dpi)
                        .range(36.0..=2400.0)
                        .suffix(" dpi"),
                );
                for d in [72.0, 96.0, 150.0, 300.0] {
                    if ui.small_button(format!("{d:.0}")).clicked() {
                        st.dpi = d;
                    }
                }
            });
            if st.format == 2 || st.format == 6 || st.format == 7 {
                ui.checkbox(&mut st.transparent, tr("dialog.transparent_background"));
            }
            if st.format == 3 || st.format == 6 {
                ui.add(crate::ui::Rail(
                    egui::Slider::new(&mut st.quality, 1..=100).text(tr("dialog.quality")),
                ));
            }
            let b = if st.selection_only {
                app.selection_bounds().unwrap_or(app.page_rect())
            } else {
                app.page_rect()
            };
            ui.label(format!(
                "{} x {} px",
                (b.width() / 25.4 * st.dpi).round(),
                (b.height() / 25.4 * st.dpi).round()
            ));
        }
        if st.format == 1 {
            ui.horizontal(|ui| {
                ui.label(tr("dialog.pdf_standard"));
                egui::ComboBox::from_id_salt("pdf_standard")
                    .selected_text(PDF_STANDARDS[st.pdf_standard.min(3)])
                    .show_ui(ui, |ui| {
                        for (i, n) in PDF_STANDARDS.iter().enumerate() {
                            ui.selectable_value(&mut st.pdf_standard, i, *n);
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label(tr("dialog.bleed"));
                ui.add(
                    egui::DragValue::new(&mut st.bleed_mm)
                        .range(0.0..=50.0)
                        .speed(0.5)
                        .suffix(" mm"),
                );
            });
            if st.pdf_standard > 0 {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.output_condition"));
                    ui.text_edit_singleline(&mut st.output_condition);
                });
                ui.label(
                    egui::RichText::new(tr("dialog.pdfx_profile_note"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
            }
        }
        ui.checkbox(&mut st.selection_only, tr("dialog.selected_only"));
        if !st.selection_only && st.format != 1 && st.format != 11 {
            ui.checkbox(&mut st.all_pages, tr("dialog.all_pages"));
        }
        if ok_cancel(ui, close) {
            let (name, ext) = EXPORT_FORMATS[st.format];
            let _ = name;
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
    });
}

fn print_dialog(app: &mut App, ctx: &Context, st: &mut PrintState, close: &mut bool) {
    window(ctx, tr("dialog.print")).show(ctx, |ui| {
        ui.label(egui::RichText::new(tr("dialog.print_hint")).color(Tokens::TEXT_DIM));
        ui.horizontal(|ui| {
            ui.label(tr("dialog.copies"));
            ui.add(egui::DragValue::new(&mut st.copies).range(1..=999));
        });
        ui.checkbox(&mut st.all_pages, tr("dialog.all_pages"));
        ui.horizontal(|ui| {
            ui.checkbox(&mut st.fit_to_page, tr("dialog.fit_to_page"));
            if !st.fit_to_page {
                ui.add(
                    egui::DragValue::new(&mut st.scale)
                        .range(1.0..=1000.0)
                        .suffix(" %"),
                );
            }
        });
        ui.separator();
        ui.strong(tr("dialog.prepress"));
        ui.checkbox(&mut st.separations, tr("dialog.separations"));
        ui.checkbox(&mut st.crop_marks, tr("dialog.crop_marks"));
        ui.checkbox(&mut st.registration_marks, tr("dialog.registration_marks"));
        ui.checkbox(&mut st.bleed, tr("dialog.print_bleed"));
        ui.checkbox(&mut st.mirror, tr("dialog.mirror"));
        ui.checkbox(&mut st.invert, tr("dialog.invert"));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button(tr("dialog.print")).clicked() {
                match crate::export::print(app, st) {
                    Ok(msg) => app.status = msg,
                    Err(e) => app.status = format!("{}: {e}", tr("dialog.print_failed")),
                }
                *close = true;
            }
            if ui.button(tr("menu.file.print_preview")).clicked() {
                app.fullscreen_preview = true;
                *close = true;
            }
            if ui.button(tr("dialog.cancel")).clicked() {
                *close = true;
            }
        });
    });
}

fn print_merge_dialog(app: &mut App, ctx: &Context, st: &mut PrintMergeState, close: &mut bool) {
    window(ctx, tr("dialog.print_merge")).show(ctx, |ui| {
        ui.label(tr("dialog.print_merge_hint"));
        ui.horizontal(|ui| {
            if ui.button(tr("dialog.load_csv")).clicked() {
                crate::files::Dialog::new()
                    .add_filter("CSV", &["csv", "txt"])
                    .pick_into("print_merge_csv");
            }
            if let Some(p) = crate::files::take_picked("print_merge_csv") {
                if let Ok(text) = crate::files::read_to_string(&p) {
                    let (h, r) = crate::export::parse_csv(&text);
                    st.headers = h;
                    st.rows = r;
                    st.path = Some(p);
                }
            }
            if let Some(p) = &st.path {
                ui.label(
                    p.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                );
            }
        });
        if !st.headers.is_empty() {
            ui.label(trf(
                "dialog.fields_rows",
                &[
                    ("f", &st.headers.join(", ")),
                    ("n", &st.rows.len().to_string()),
                ],
            ));
            ui.label(
                egui::RichText::new(tr("dialog.print_merge_fields_hint"))
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
            );
            ui.horizontal_wrapped(|ui| {
                for h in st.headers.clone() {
                    if ui.small_button(format!("<{h}>")).clicked() {
                        app.insert_text(&format!("<{h}>"));
                    }
                }
            });
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !st.rows.is_empty(),
                    egui::Button::new(tr("dialog.perform_merge")),
                )
                .clicked()
            {
                crate::export::perform_merge(app, &st.headers, &st.rows);
                *close = true;
            }
            if ui.button(tr("dialog.close")).clicked() {
                *close = true;
            }
        });
        if !st.rows.is_empty() {
            app.merge_state = Some(st.clone());
        }
    });
}

fn trace_dialog(app: &mut App, ctx: &Context, st: &mut TraceState, close: &mut bool) {
    window(ctx, tr("dialog.trace")).show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            for p in [
                crate::trace::Preset::Technical,
                crate::trace::Preset::LineDrawing,
                crate::trace::Preset::LineArt,
                crate::trace::Preset::Logo,
                crate::trace::Preset::DetailedLogo,
                crate::trace::Preset::Clipart,
                crate::trace::Preset::LowQualityImage,
                crate::trace::Preset::HighQualityImage,
            ] {
                if ui.selectable_label(st.preset == p, tr(p.key())).clicked() {
                    st.preset = p;
                    st.settings = p.settings();
                    st.preview_count = None;
                }
            }
        });
        let s = &mut st.settings;
        ui.add(crate::ui::Rail(
            egui::Slider::new(&mut s.detail, 0.0..=100.0).text(tr("dialog.detail")),
        ));
        ui.add(crate::ui::Rail(
            egui::Slider::new(&mut s.smoothing, 0.0..=100.0).text(tr("dialog.smoothing")),
        ));
        ui.add(crate::ui::Rail(
            egui::Slider::new(&mut s.corner_smoothness, 0.0..=100.0)
                .text(tr("dialog.corner_smoothness")),
        ));
        if !s.centerline {
            ui.horizontal(|ui| {
                ui.checkbox(&mut s.black_white, tr("dialog.black_and_white"));
                if !s.black_white {
                    ui.label(tr("dialog.colors"));
                    ui.add(egui::DragValue::new(&mut s.colors).range(2..=256));
                }
            });
            ui.checkbox(&mut s.remove_background, tr("dialog.remove_background"));
        }
        ui.checkbox(&mut s.delete_original, tr("dialog.delete_original"));
        ui.horizontal(|ui| {
            if ui.button(tr("dialog.preview")).clicked() {
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
    let Some(s) = app
        .selected_shapes()
        .into_iter()
        .find(|s| matches!(s.kind, ShapeKind::Text { .. }))
    else {
        *close = true;
        return;
    };
    let ShapeKind::Text { para, .. } = &s.kind else {
        *close = true;
        return;
    };
    let mut p = para.clone();
    let mut changed = false;
    let u = app.units;
    window(ctx, title).show(ctx, |ui| {
        match which {
            Dialog::TextTabs => {
                ui.label(tr("dialog.tab_stops"));
                let mut remove = None;
                for (i, t) in p.tabs.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        changed |= unit_value(ui, &format!("{}", i + 1), t, u);
                        if ui.small_button("x").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    p.tabs.remove(i);
                    changed = true;
                }
                if ui.button(tr("dialog.add_tab")).clicked() {
                    let last = p.tabs.last().copied().unwrap_or(0.0);
                    p.tabs.push(last + 12.7);
                    changed = true;
                }
            }
            Dialog::TextColumns => {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.columns"));
                    changed |= ui
                        .add(egui::DragValue::new(&mut p.columns).range(1..=8))
                        .changed();
                });
                changed |= unit_value(ui, &tr("dialog.gutter"), &mut p.gutter, u);
            }
            Dialog::TextBullets => {
                changed |= ui
                    .checkbox(&mut p.bullets, tr("dialog.use_bullets"))
                    .changed();
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.bullet_symbol"));
                    for b in [
                        "\u{2022}", "\u{25E6}", "\u{25AA}", "\u{2013}", "\u{2713}", "\u{27A4}",
                    ] {
                        if ui.selectable_label(p.bullet_char == b, b).clicked() {
                            p.bullet_char = b.to_string();
                            changed = true;
                        }
                    }
                });
                changed |= unit_value(ui, &tr("dialog.bullet_indent"), &mut p.bullet_indent, u);
            }
            _ => {
                let mut on = p.drop_cap_lines > 1;
                if ui.checkbox(&mut on, tr("dialog.use_drop_cap")).changed() {
                    p.drop_cap_lines = if on { 3 } else { 0 };
                    changed = true;
                }
                if on {
                    ui.horizontal(|ui| {
                        ui.label(tr("dialog.lines_dropped"));
                        changed |= ui
                            .add(egui::DragValue::new(&mut p.drop_cap_lines).range(2..=10))
                            .changed();
                    });
                }
            }
        }
        if crate::ui::chrome::button_row(ui, &[&tr("dialog.close")], false).is_some() {
            *close = true;
        }
    });
    if changed {
        app.set_paragraph_style(s.id, p);
    }
}

fn spell_dialog(app: &mut App, ctx: &Context, st: &mut SpellState, close: &mut bool) {
    if !st.checked {
        let (unknown, words) = crate::spell::check_document(app);
        st.unknown = unknown;
        st.dictionary_words = words;
        st.checked = true;
        st.cursor = 0;
        st.replacement = st
            .unknown
            .first()
            .map(|(_, w)| w.clone())
            .unwrap_or_default();
    }
    window(ctx, tr("dialog.spell_check")).show(ctx, |ui| {
        if st.dictionary_words == 0 {
            ui.colored_label(
                egui::Color32::from_rgb(180, 80, 0),
                tr("dialog.no_dictionary"),
            );
        }
        if st.unknown.is_empty() {
            ui.label(tr("dialog.spell_done"));
        } else if let Some((id, word)) = st.unknown.get(st.cursor).cloned() {
            ui.label(trf("dialog.not_in_dictionary", &[("w", &word)]));
            ui.horizontal(|ui| {
                ui.label(tr("dialog.replace_with"));
                ui.text_edit_singleline(&mut st.replacement);
            });
            let suggestions = crate::spell::suggest(&word);
            ui.horizontal_wrapped(|ui| {
                for s in suggestions {
                    if ui.small_button(&s).clicked() {
                        st.replacement = s;
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui.button(tr("dialog.replace")).clicked() {
                    crate::spell::replace_word(app, id, &word, &st.replacement);
                    st.unknown.remove(st.cursor);
                    st.replacement = st
                        .unknown
                        .get(st.cursor)
                        .map(|(_, w)| w.clone())
                        .unwrap_or_default();
                }
                if ui.button(tr("dialog.skip")).clicked() {
                    st.cursor += 1;
                    st.replacement = st
                        .unknown
                        .get(st.cursor)
                        .map(|(_, w)| w.clone())
                        .unwrap_or_default();
                }
                if ui.button(tr("dialog.add_to_dictionary")).clicked() {
                    crate::spell::add_user_word(&word);
                    let w = word.clone();
                    st.unknown.retain(|(_, x)| *x != w);
                    st.replacement = st
                        .unknown
                        .get(st.cursor)
                        .map(|(_, w)| w.clone())
                        .unwrap_or_default();
                }
            });
            if st.cursor >= st.unknown.len() {
                st.unknown.clear();
            }
        }
        if crate::ui::chrome::button_row(ui, &[&tr("dialog.close")], false).is_some() {
            *close = true;
        }
    });
}

fn thesaurus_lookup(app: &App, st: &mut ThesaurusState) {
    let th = crate::thesaurus::get(app.settings.thesaurus_file.as_deref());
    st.meanings = th.lookup(&st.word).to_vec();
    st.looked_up = Some(st.word.trim().to_string());
    st.selected = None;
}

fn encode_dialog(app: &mut App, ctx: &Context, st: &mut EncodeState, close: &mut bool) {
    use crate::encode::Encoding;
    let original = app.selected_text_content();
    let mut apply = false;
    window(ctx, tr("dialog.encode")).show(ctx, |ui| {
        ui.set_min_width(420.0);
        ui.label(egui::RichText::new(tr("dialog.encode_hint")).color(Tokens::TEXT_DIM));
        ui.add_space(4.0);
        let combo = |ui: &mut Ui, id: &str, label: &str, v: &mut Encoding| {
            ui.horizontal(|ui| {
                ui.label(label);
                egui::ComboBox::from_id_salt(id)
                    .selected_text(v.name())
                    .show_ui(ui, |ui| {
                        for e in Encoding::ALL {
                            ui.selectable_value(v, e, e.name());
                        }
                    });
            });
        };
        combo(ui, "encode_from", &tr("dialog.encode_from"), &mut st.from);
        combo(ui, "encode_to", &tr("dialog.encode_to"), &mut st.to);
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
                .clicked()
                || (can && enter_pressed(ui))
            {
                apply = true;
                *close = true;
            }
            if ui.button(tr("dialog.cancel")).clicked() {
                *close = true;
            }
        });
    });
    if apply {
        app.reencode_selected_text(st.from, st.to);
    }
}

fn thesaurus_dialog(app: &mut App, ctx: &Context, st: &mut ThesaurusState, close: &mut bool) {
    if !st.started {
        st.started = true;
        if let Some(s) = app.text_shapes().first() {
            if let ShapeKind::Text { spans, .. } = &s.kind {
                let text: String = spans.iter().map(|x| x.text.as_str()).collect();
                if let Some(w) = crate::thesaurus::first_word(&text) {
                    st.word = w.clone();
                    st.original = w;
                    st.target = Some(s.id);
                }
            }
        }
        if !st.word.is_empty() {
            thesaurus_lookup(app, st);
        }
    }
    let mut pick_file = false;
    let mut relookup: Option<String> = None;
    let mut replace: Option<(tracedraw_core::ShapeId, String)> = None;
    window(ctx, tr("dialog.thesaurus")).show(ctx, |ui| {
        ui.set_min_width(360.0);
        let th = crate::thesaurus::get(app.settings.thesaurus_file.as_deref());
        match &th.source {
            crate::thesaurus::Source::BuiltIn => {
                ui.colored_label(
                    egui::Color32::from_rgb(180, 80, 0),
                    tr("thesaurus.built_in_note"),
                );
            }
            crate::thesaurus::Source::File(p) => {
                ui.label(trf(
                    "thesaurus.source_file",
                    &[
                        ("path", &p.display().to_string()),
                        ("n", &th.len().to_string()),
                    ],
                ));
            }
        }
        ui.horizontal(|ui| {
            ui.label(tr("thesaurus.word"));
            let r = ui.text_edit_singleline(&mut st.word);
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button(tr("thesaurus.look_up")).clicked() || enter {
                thesaurus_lookup(app, st);
            }
        });
        ui.separator();
        if let Some(w) = &st.looked_up {
            if st.meanings.is_empty() {
                ui.label(trf("thesaurus.no_entry", &[("w", w)]));
            } else {
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for m in &st.meanings {
                            ui.label(
                                egui::RichText::new(format!("({})", m.pos))
                                    .italics()
                                    .color(Tokens::TEXT_DIM),
                            );
                            ui.horizontal_wrapped(|ui| {
                                for syn in &m.synonyms {
                                    let on = st.selected.as_deref() == Some(syn.as_str());
                                    let r = ui.selectable_label(on, syn);
                                    if r.clicked() {
                                        st.selected = Some(syn.clone());
                                    }
                                    if r.double_clicked() {
                                        relookup = Some(syn.clone());
                                    }
                                }
                            });
                        }
                    });
            }
        }
        if st.target.is_none() {
            ui.label(egui::RichText::new(tr("thesaurus.no_text_selected")).color(Tokens::TEXT_DIM));
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let can_replace = st.target.is_some() && st.selected.is_some();
            if ui
                .add_enabled(can_replace, egui::Button::new(tr("dialog.replace")))
                .clicked()
            {
                if let (Some(id), Some(sel)) = (st.target, st.selected.clone()) {
                    replace = Some((id, sel));
                }
            }
            if ui.button(tr("thesaurus.pick_file")).clicked() {
                pick_file = true;
            }
            if ui.button(tr("dialog.close")).clicked() {
                *close = true;
            }
        });
    });
    if let Some(w) = relookup {
        st.word = w;
        thesaurus_lookup(app, st);
    }
    if let Some((id, sel)) = replace {
        let repl = crate::thesaurus::match_case(&st.original, &sel);
        crate::spell::replace_word(app, id, &st.original, &repl);
        app.status = trf("thesaurus.replaced", &[("a", &st.original), ("b", &repl)]);
        st.original = repl.clone();
        st.word = repl;
        thesaurus_lookup(app, st);
    }
    if pick_file {
        crate::files::Dialog::new()
            .add_filter("MyThes", &["dat"])
            .pick_into("thesaurus_file");
    }
    if let Some(p) = crate::files::take_picked("thesaurus_file") {
        app.settings.thesaurus_file = Some(p);
        app.settings.save();
        thesaurus_lookup(app, st);
    }
}

fn grammar_options() -> crate::grammar::Options {
    crate::grammar::Options::for_language(&crate::i18n::language())
}

fn grammar_dialog(app: &mut App, ctx: &Context, st: &mut GrammarState, close: &mut bool) {
    if !st.checked {
        st.findings = crate::grammar::check_document(app, grammar_options());
        st.checked = true;
    }
    let mut fix: Option<(tracedraw_core::ShapeId, crate::grammar::Fix)> = None;
    let mut fix_all = false;
    window(ctx, tr("dialog.grammar")).show(ctx, |ui| {
        ui.set_min_width(420.0);
        ui.label(egui::RichText::new(tr("grammar.scope_note")).color(Tokens::TEXT_DIM));
        if st.findings.is_empty() {
            ui.label(tr("grammar.no_issues"));
        } else {
            ui.label(trf(
                "grammar.issues_found",
                &[("n", &st.findings.len().to_string())],
            ));
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for (i, (id, f)) in st.findings.iter().enumerate() {
                        ui.push_id(i, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    let rule = if f.rule == crate::grammar::Rule::LongSentence {
                                        trf(
                                            f.rule.key(),
                                            &[(
                                                "n",
                                                &crate::grammar::LONG_SENTENCE_WORDS.to_string(),
                                            )],
                                        )
                                    } else {
                                        tr(f.rule.key())
                                    };
                                    ui.label(egui::RichText::new(rule).strong());
                                    let mut s: String = f.sentence.chars().take(90).collect();
                                    if s.chars().count() < f.sentence.chars().count() {
                                        s.push_str("...");
                                    }
                                    ui.label(egui::RichText::new(s).color(Tokens::TEXT_DIM));
                                });
                                if let Some(x) = &f.fix {
                                    if ui.button(tr("grammar.fix")).clicked() {
                                        fix = Some((*id, x.clone()));
                                    }
                                }
                            });
                            ui.separator();
                        });
                    }
                });
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button(tr("grammar.check_again")).clicked() {
                st.checked = false;
            }
            let any_fix = st.findings.iter().any(|(_, f)| f.fix.is_some());
            if ui
                .add_enabled(any_fix, egui::Button::new(tr("grammar.fix_all")))
                .clicked()
            {
                fix_all = true;
            }
            if ui.button(tr("dialog.close")).clicked() {
                *close = true;
            }
        });
    });
    if let Some((id, x)) = fix {
        crate::grammar::apply_fix_to_shape(app, id, &x);
        st.checked = false;
    }
    if fix_all {
        // One fix at a time, re-checking in between so offsets stay valid.
        for _ in 0..500 {
            let findings = crate::grammar::check_document(app, grammar_options());
            let Some((id, f)) = findings.into_iter().find(|(_, f)| f.fix.is_some()) else {
                break;
            };
            if let Some(x) = f.fix {
                crate::grammar::apply_fix_to_shape(app, id, &x);
            }
        }
        st.checked = false;
    }
}

fn autocorrect_dialog(app: &mut App, ctx: &Context, st: &mut AutocorrectState, close: &mut bool) {
    let mut changed = false;
    let mut apply = false;
    let style = crate::autocorrect::QuoteStyle::for_language(&crate::i18n::language());
    window(ctx, tr("dialog.autocorrect")).show(ctx, |ui| {
        ui.set_min_width(380.0);
        let p = &mut app.settings.autocorrect;
        changed |= ui
            .checkbox(&mut p.enabled, tr("autocorrect.enabled"))
            .changed();
        ui.separator();
        changed |= ui
            .checkbox(&mut p.capitalize_sentences, tr("autocorrect.capitalize"))
            .changed();
        changed |= ui
            .checkbox(
                &mut p.fix_two_initial_capitals,
                tr("autocorrect.two_capitals"),
            )
            .changed();
        changed |= ui
            .checkbox(&mut p.typographic_quotes, tr("autocorrect.quotes"))
            .changed();
        let (o, c) = style.double();
        let (so, sc) = style.single();
        ui.label(
            egui::RichText::new(trf(
                "autocorrect.quotes_example",
                &[("q", &format!("{o}abc{c}  {so}abc{sc}"))],
            ))
            .color(Tokens::TEXT_DIM),
        );
        ui.separator();
        ui.label(egui::RichText::new(tr("autocorrect.replacements")).strong());
        let mut remove: Option<usize> = None;
        egui::ScrollArea::vertical()
            .max_height(180.0)
            .show(ui, |ui| {
                egui::Grid::new("qc_table")
                    .num_columns(3)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(tr("autocorrect.replace_col"));
                        ui.label(tr("autocorrect.with_col"));
                        ui.label("");
                        ui.end_row();
                        for (i, (from, to)) in p.replacements.iter_mut().enumerate() {
                            changed |= ui
                                .add_sized([130.0, 20.0], egui::TextEdit::singleline(from))
                                .changed();
                            changed |= ui
                                .add_sized([130.0, 20.0], egui::TextEdit::singleline(to))
                                .changed();
                            if ui.small_button(tr("autocorrect.remove")).clicked() {
                                remove = Some(i);
                            }
                            ui.end_row();
                        }
                        ui.add_sized([130.0, 20.0], egui::TextEdit::singleline(&mut st.new_from));
                        ui.add_sized([130.0, 20.0], egui::TextEdit::singleline(&mut st.new_to));
                        let can_add = !st.new_from.trim().is_empty()
                            && !st.new_from.contains(char::is_whitespace);
                        if ui
                            .add_enabled(can_add, egui::Button::new(tr("autocorrect.add")).small())
                            .clicked()
                        {
                            p.replacements
                                .push((st.new_from.trim().to_string(), st.new_to.clone()));
                            st.new_from.clear();
                            st.new_to.clear();
                            changed = true;
                        }
                        ui.end_row();
                    });
            });
        if let Some(i) = remove {
            if i < p.replacements.len() {
                p.replacements.remove(i);
                changed = true;
            }
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button(tr("autocorrect.apply_selection")).clicked() {
                apply = true;
            }
            if ui.button(tr("dialog.close")).clicked() {
                *close = true;
            }
        });
    });
    if changed {
        app.settings.save();
    }
    if apply {
        if app.text_shapes().is_empty() {
            app.status = tr("autocorrect.no_selection");
        } else {
            let n = crate::autocorrect::apply_to_selection(app);
            app.status = trf("autocorrect.applied", &[("n", &n.to_string())]);
        }
    }
}

fn border_grommet_dialog(
    app: &mut App,
    ctx: &Context,
    st: &mut crate::border_grommet::BorderGrommetState,
    close: &mut bool,
) {
    use crate::border_grommet::BorderKind;
    let units = app.units;
    let page = app.page_size();
    let mut ok = false;
    window(ctx, tr("border_grommet.title")).show(ctx, |ui| {
        ui.set_min_width(360.0);
        ui.horizontal(|ui| {
            ui.label(tr("border_grommet.border_type"));
            egui::ComboBox::from_id_salt("bg_kind")
                .selected_text(tr(st.border.key()))
                .show_ui(ui, |ui| {
                    for k in BorderKind::ALL {
                        ui.selectable_value(&mut st.border, k, tr(k.key()));
                    }
                });
        });
        if st.border != BorderKind::None {
            unit_value(
                ui,
                &tr("border_grommet.border_width"),
                &mut st.width_mm,
                units,
            );
            st.width_mm = st.width_mm.max(0.0);
        }
        if st.border == BorderKind::Solid {
            ui.horizontal(|ui| {
                ui.label(tr("border_grommet.border_color"));
                let [r, g, b] = st.color.to_rgb8();
                let mut rgb = [r, g, b];
                if ui.color_edit_button_srgb(&mut rgb).changed() {
                    st.color = Color::rgb8(rgb[0], rgb[1], rgb[2]);
                }
            });
        }
        ui.separator();
        ui.checkbox(&mut st.grommets, tr("border_grommet.grommets"));
        if st.grommets {
            let g = &mut st.grommet;
            unit_value(
                ui,
                &tr("border_grommet.diameter"),
                &mut g.diameter_mm,
                units,
            );
            unit_value(ui, &tr("border_grommet.margin"), &mut g.margin_mm, units);
            g.diameter_mm = g.diameter_mm.max(0.1);
            g.margin_mm = g.margin_mm.max(0.0);
            ui.horizontal(|ui| {
                ui.radio_value(&mut g.by_count, false, tr("border_grommet.by_spacing"));
                ui.add_enabled_ui(!g.by_count, |ui| {
                    let mut v = units.from_mm(g.spacing_mm);
                    if ui
                        .add(
                            egui::DragValue::new(&mut v)
                                .speed(1.0)
                                .suffix(format!(" {}", units.short())),
                        )
                        .changed()
                    {
                        g.spacing_mm = units.to_mm(v).max(1.0);
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.radio_value(&mut g.by_count, true, tr("border_grommet.by_count"));
                ui.add_enabled_ui(g.by_count, |ui| {
                    ui.add(egui::DragValue::new(&mut g.count_per_edge).range(2..=200));
                });
            });
            ui.checkbox(&mut g.corners_only, tr("border_grommet.corners_only"));
        }
        ui.separator();
        let final_size = st.final_size(page);
        let n = if st.grommets {
            crate::border_grommet::grommet_centers(final_size, &st.grommet).len()
        } else {
            0
        };
        ui.label(trf(
            "border_grommet.preview",
            &[
                ("n", &n.to_string()),
                ("w", &format!("{:.1}", units.from_mm(final_size.width))),
                ("h", &format!("{:.1}", units.from_mm(final_size.height))),
                ("u", units.short()),
            ],
        ));
        ok = ok_cancel(ui, close);
    });
    if ok {
        let n = crate::border_grommet::apply(app, st);
        app.status = trf("border_grommet.done", &[("n", &n.to_string())]);
    }
}

fn color_management_dialog(app: &mut App, ctx: &Context, close: &mut bool) {
    let mut load_rgb = false;
    let mut load_cmyk = false;
    window(ctx, tr("menu.tools.color_management")).show(ctx, |ui| {
        let c = &mut app.settings.color;
        egui::Grid::new("cm").num_columns(2).show(ui, |ui| {
            ui.label(tr("dialog.rgb_profile"));
            egui::ComboBox::from_id_salt("rgbp")
                .selected_text(c.rgb_profile.clone())
                .show_ui(ui, |ui| {
                    for p in [
                        "sRGB IEC61966-2.1",
                        "Adobe RGB (1998)",
                        "Display P3",
                        "ProPhoto RGB",
                    ] {
                        if ui.selectable_label(c.rgb_profile == p, p).clicked() {
                            c.rgb_profile = p.into();
                            c.rgb_profile_path.clear();
                        }
                    }
                });
            ui.end_row();
            ui.label("");
            ui.horizontal(|ui| {
                if ui.button(tr("dialog.load_icc")).clicked() {
                    load_rgb = true;
                }
                ui.label(
                    egui::RichText::new(if c.rgb_profile_path.is_empty() {
                        tr("dialog.builtin_profile")
                    } else {
                        c.rgb_profile_path.clone()
                    })
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
                );
            });
            ui.end_row();
            ui.label(tr("dialog.cmyk_profile"));
            egui::ComboBox::from_id_salt("cmykp")
                .selected_text(c.cmyk_profile.clone())
                .show_ui(ui, |ui| {
                    for p in [
                        "Generic CMYK (open)",
                        "Coated FOGRA39 (user)",
                        "US Web Coated SWOP (user)",
                        "Japan Color 2001 Coated (user)",
                    ] {
                        if ui.selectable_label(c.cmyk_profile == p, p).clicked() {
                            c.cmyk_profile = p.into();
                            c.cmyk_profile_path.clear();
                        }
                    }
                });
            ui.end_row();
            ui.label("");
            ui.horizontal(|ui| {
                if ui.button(tr("dialog.load_icc")).clicked() {
                    load_cmyk = true;
                }
                ui.label(
                    egui::RichText::new(if c.cmyk_profile_path.is_empty() {
                        tr("dialog.builtin_profile")
                    } else {
                        c.cmyk_profile_path.clone()
                    })
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
                );
            });
            ui.end_row();
            ui.label(tr("dialog.rendering_intent"));
            egui::ComboBox::from_id_salt("intent")
                .selected_text(c.intent.clone())
                .show_ui(ui, |ui| {
                    for p in [
                        "Relative colorimetric",
                        "Absolute colorimetric",
                        "Perceptual",
                        "Saturation",
                    ] {
                        if ui.selectable_label(c.intent == p, p).clicked() {
                            c.intent = p.into();
                        }
                    }
                });
            ui.end_row();
            ui.label(tr("dialog.black_point"));
            ui.checkbox(&mut c.black_point_compensation, "");
            ui.end_row();
            ui.label(tr("dialog.proof_profile"));
            ui.text_edit_singleline(&mut c.proof_profile);
            ui.end_row();
        });
        ui.label(
            egui::RichText::new(tr("dialog.color_management_hint"))
                .color(Tokens::TEXT_DIM)
                .size(11.0),
        );
        ui.checkbox(&mut app.proof_colors, tr("menu.view.proof_colors"));
        ui.horizontal(|ui| {
            if ui.button(tr("dialog.ok")).clicked() || enter_pressed(ui) {
                app.apply_color_settings();
                app.save_settings();
                *close = true;
            }
            if ui.button(tr("dialog.cancel")).clicked() {
                *close = true;
            }
        });
    });
    if load_rgb {
        app.load_icc_profile(false);
    }
    if load_cmyk {
        app.load_icc_profile(true);
    }
}

fn font_manager_dialog(app: &mut App, ctx: &Context, st: &mut FontManagerState, close: &mut bool) {
    egui::Window::new(tr("menu.tools.font_manager"))
        .collapsible(false)
        .resizable(true)
        .default_width(560.0)
        .default_height(420.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            if st.sample.is_empty() {
                st.sample = tr("dialog.font_sample");
            }
            ui.horizontal(|ui| {
                ui.label(tr("dialog.filter"));
                ui.text_edit_singleline(&mut st.filter);
                ui.label(tr("dialog.sample_text"));
                ui.text_edit_singleline(&mut st.sample);
            });
            let missing: Vec<String> = app.missing_fonts.clone();
            if !missing.is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(180, 80, 0),
                    trf("dialog.missing_fonts", &[("f", &missing.join(", "))]),
                );
            }
            let filter = st.filter.to_lowercase();
            let families: Vec<String> = app
                .font_families
                .iter()
                .filter(|f| filter.is_empty() || f.to_lowercase().contains(&filter))
                .cloned()
                .collect();
            ui.label(trf("dialog.fonts_n", &[("n", &families.len().to_string())]));
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for f in families {
                        let selected = st.selected.as_deref() == Some(f.as_str());
                        let r = ui.selectable_label(selected, format!("{f}    {}", st.sample));
                        if r.clicked() {
                            st.selected = Some(f.clone());
                        }
                        if r.double_clicked() {
                            app.text_font = f.clone();
                            app.apply_text_font(&f);
                        }
                    }
                });
            ui.horizontal(|ui| {
                if let Some(f) = st.selected.clone() {
                    if ui.button(tr("dialog.use_font")).clicked() {
                        app.text_font = f.clone();
                        app.apply_text_font(&f);
                    }
                }
                if ui.button(tr("dialog.close")).clicked() {
                    *close = true;
                }
            });
        });
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
                });
            if ui.button(tr("dialog.new_palette")).clicked() {
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
                ui.text_edit_singleline(&mut pal.name);
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
                    ui.text_edit_singleline(name);
                    let [r, g, b] = c.to_rgb8();
                    let mut rgb = [r, g, b];
                    if ui.color_edit_button_srgb(&mut rgb).changed() && !pal.builtin {
                        *c = Color::rgb8(rgb[0], rgb[1], rgb[2]);
                    }
                    ui.label(crate::app::color_description(*c));
                });
            }
        }
        if !pal.builtin {
            ui.horizontal(|ui| {
                if ui.button(tr("dialog.add_color")).clicked() {
                    let n = pal.colors.len() + 1;
                    pal.colors
                        .push((format!("Color {n}"), Color::rgb8(128, 128, 128)));
                    st.selected = Some(pal.colors.len() - 1);
                }
                if let Some(i) = st.selected {
                    if ui.button(tr("dialog.remove_color")).clicked() && i < pal.colors.len() {
                        pal.colors.remove(i);
                        st.selected = None;
                    }
                }
                if ui.button(tr("dialog.save_palette")).clicked() {
                    if let Some(p) = crate::files::Dialog::new()
                        .add_filter("Palette", &["tdpal", "gpl"])
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
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
            );
        }
        if ui.button(tr("dialog.close")).clicked() {
            app.rebuild_palette();
            *close = true;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every dialog, as the menus open it.
    fn all_dialogs(app: &App) -> Vec<Dialog> {
        let layer = app.active_layer().unwrap_or(tracedraw_core::LayerId(1));
        vec![
            Dialog::NodeAlign {
                horizontal: true,
                vertical: false,
            },
            Dialog::BitmapBw(Default::default()),
            Dialog::BitmapDuotone(Default::default()),
            Dialog::BitmapPaletted(Default::default()),
            Dialog::RenamePage {
                name: "Page".into(),
            },
            Dialog::GoToPage { page: 1 },
            Dialog::InsertPage {
                count: 1,
                after: true,
            },
            Dialog::PageSize {
                width: 210.0,
                height: 297.0,
                all_pages: false,
            },
            Dialog::Options,
            Dialog::PageNumberSettings,
            Dialog::Symmetry,
            Dialog::RenameLayer {
                layer,
                name: "Layer".into(),
            },
            Dialog::DocumentProperties,
            Dialog::ConfirmClose,
            Dialog::Export(ExportState::default()),
            Dialog::Print(PrintState::default()),
            Dialog::PrintMerge(PrintMergeState::default()),
            Dialog::CreateTable { rows: 3, cols: 4 },
            Dialog::QrCode(QrState::default()),
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
            Dialog::Resample { dpi: 100.0 },
            Dialog::InflateBitmap(InflateState {
                base: (40, 30),
                pw: 120.0,
                ph: 110.0,
                keep: false,
            }),
            Dialog::Trace(TraceState::new(crate::trace::Preset::Logo)),
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
            Dialog::SpellCheck(SpellState::default()),
            Dialog::Thesaurus(ThesaurusState::default()),
            Dialog::Encode(EncodeState::default()),
            Dialog::Grammar(GrammarState::default()),
            Dialog::Autocorrect(AutocorrectState::default()),
            Dialog::BorderGrommet(crate::border_grommet::BorderGrommetState::default()),
            Dialog::ColorManagement,
            Dialog::FontManager(FontManagerState::default()),
            Dialog::PaletteEditor(PaletteEditorState::default()),
            Dialog::NewDocument(crate::new_document::NewDocState::from_settings(
                &Default::default(),
                "n".into(),
            )),
            Dialog::About,
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
}
