//! Dialogs: pages, layers, Options (multi-page), Document Properties,
//! Export, Print, Print Merge, Find and Replace, tables, QR codes,
//! bitmaps (convert, straighten, resample, inflate, trace, effect amount),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptionsPage {
    #[default]
    General,
    Workspace,
    PageSize,
    Layout,
    Background,
    Guidelines,
    Grid,
    Rulers,
    Save,
    Shortcuts,
    Tools,
    Text,
}

impl OptionsPage {
    pub const ALL: [OptionsPage; 12] = [
        OptionsPage::General,
        OptionsPage::Workspace,
        OptionsPage::PageSize,
        OptionsPage::Layout,
        OptionsPage::Background,
        OptionsPage::Guidelines,
        OptionsPage::Grid,
        OptionsPage::Rulers,
        OptionsPage::Save,
        OptionsPage::Shortcuts,
        OptionsPage::Tools,
        OptionsPage::Text,
    ];
    pub fn key(self) -> &'static str {
        match self {
            OptionsPage::General => "options.general",
            OptionsPage::Workspace => "options.workspace",
            OptionsPage::PageSize => "options.page_size",
            OptionsPage::Layout => "options.layout",
            OptionsPage::Background => "options.background",
            OptionsPage::Guidelines => "options.guidelines",
            OptionsPage::Grid => "options.grid",
            OptionsPage::Rulers => "options.rulers",
            OptionsPage::Save => "options.save",
            OptionsPage::Shortcuts => "options.shortcuts",
            OptionsPage::Tools => "options.tools",
            OptionsPage::Text => "options.text",
        }
    }
}

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

pub const EXPORT_FORMATS: [(&str, &str); 12] = [
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
    FindReplace(FindReplaceState),
    CreateTable {
        rows: u32,
        cols: u32,
    },
    QrCode(QrState),
    PasteSpecial(PasteSpecialState),
    Barcode(BarcodeState),
    ConvertToBitmap {
        dpi: f64,
        transparent: bool,
    },
    StraightenImage {
        angle: f64,
    },
    Resample {
        dpi: f64,
    },
    InflateBitmap {
        px: u32,
    },
    Trace(TraceState),
    BitmapFx {
        fx: crate::bitmap_fx::Fx,
        amount: f32,
    },
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
    NewDocument {
        width: f64,
        height: f64,
        preset: usize,
        name: String,
    },
    About,
}

fn window<'a>(_ctx: &Context, title: String) -> egui::Window<'a> {
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
}

fn ok_cancel(ui: &mut Ui, close: &mut bool) -> bool {
    let mut ok = false;
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button(tr("dialog.ok")).clicked() {
            ok = true;
            *close = true;
        }
        if ui.button(tr("dialog.cancel")).clicked() {
            *close = true;
        }
    });
    ok
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
    let mut dialog = std::mem::replace(&mut app.dialog, Dialog::None);
    let mut close = false;
    match &mut dialog {
        Dialog::None => {}
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
        Dialog::Options => options_dialog(app, ctx, &mut close),
        Dialog::DocumentProperties => document_properties(app, ctx, &mut close),
        Dialog::ConfirmClose => {
            window(ctx, tr("dialog.close_document")).show(ctx, |ui| {
                ui.label(tr("dialog.save_changes_question"));
                ui.horizontal(|ui| {
                    if ui.button(tr("dialog.save")).clicked() {
                        app.save(false);
                        if !app.engine.is_dirty() {
                            app.new_document();
                            app.show_welcome = true;
                        }
                        close = true;
                    }
                    if ui.button(tr("dialog.dont_save")).clicked() {
                        app.new_document();
                        app.show_welcome = true;
                        close = true;
                    }
                    if ui.button(tr("dialog.cancel")).clicked() {
                        close = true;
                    }
                });
            });
        }
        Dialog::Export(st) => export_dialog(app, ctx, st, &mut close),
        Dialog::Print(st) => print_dialog(app, ctx, st, &mut close),
        Dialog::PrintMerge(st) => print_merge_dialog(app, ctx, st, &mut close),
        Dialog::FindReplace(st) => find_replace_dialog(app, ctx, st, &mut close),
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
                ui.text_edit_multiline(&mut st.text);
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
        Dialog::ConvertToBitmap { dpi, transparent } => {
            window(ctx, tr("dialog.convert_to_bitmap")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.resolution"));
                    ui.add(
                        egui::DragValue::new(dpi)
                            .range(36.0..=1200.0)
                            .suffix(" dpi"),
                    );
                    for d in [72.0, 150.0, 300.0, 600.0] {
                        if ui.small_button(format!("{d:.0}")).clicked() {
                            *dpi = d;
                        }
                    }
                });
                ui.checkbox(transparent, tr("dialog.transparent_background"));
                if let Some(b) = app.selection_bounds() {
                    let w = (b.width() / 25.4 * *dpi).round();
                    let h = (b.height() / 25.4 * *dpi).round();
                    ui.label(format!("{w} x {h} px"));
                }
                if ok_cancel(ui, &mut close) {
                    app.convert_to_bitmap(*dpi, *transparent);
                }
            });
        }
        Dialog::StraightenImage { angle } => {
            window(ctx, tr("dialog.straighten_image")).show(ctx, |ui| {
                ui.add(
                    egui::Slider::new(angle, -45.0..=45.0)
                        .suffix("°")
                        .text(tr("dialog.angle")),
                );
                if ok_cancel(ui, &mut close) {
                    app.straighten_bitmap(*angle);
                }
            });
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
        Dialog::InflateBitmap { px } => {
            window(ctx, tr("dialog.inflate_bitmap")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.pixels"));
                    ui.add(egui::DragValue::new(px).range(1..=500));
                });
                if ok_cancel(ui, &mut close) {
                    app.inflate_bitmap(Some(*px));
                }
            });
        }
        Dialog::Trace(st) => trace_dialog(app, ctx, st, &mut close),
        Dialog::BitmapFx { fx, amount } => {
            window(ctx, tr("dialog.effect")).show(ctx, |ui| {
                ui.label(format!("{fx:?}"));
                ui.add(egui::Slider::new(amount, 0.0..=100.0).text(tr("dialog.amount")));
                if ok_cancel(ui, &mut close) {
                    app.bitmap_fx_amount = *amount;
                    app.apply_bitmap_effect_amount(*fx, *amount);
                }
            });
        }
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
                if ui.button(tr("dialog.close")).clicked() {
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
        Dialog::NewDocument {
            width,
            height,
            preset,
            name,
        } => {
            window(ctx, tr("dialog.create_new_document")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.name"));
                    ui.text_edit_singleline(name);
                });
                let presets = paper_presets();
                egui::ComboBox::from_label(tr("dialog.preset"))
                    .selected_text(presets.get(*preset).map(|p| p.0).unwrap_or("Custom"))
                    .show_ui(ui, |ui| {
                        for (i, (n, s)) in presets.iter().enumerate() {
                            if ui.selectable_label(*preset == i, *n).clicked() {
                                *preset = i;
                                *width = s.width;
                                *height = s.height;
                            }
                        }
                    });
                unit_value(ui, &tr("dialog.width"), width, app.units);
                unit_value(ui, &tr("dialog.height"), height, app.units);
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(*width <= *height, tr("dialog.portrait"))
                        .clicked()
                        && *width > *height
                    {
                        std::mem::swap(width, height);
                    }
                    if ui
                        .selectable_label(*width > *height, tr("dialog.landscape"))
                        .clicked()
                        && *width <= *height
                    {
                        std::mem::swap(width, height);
                    }
                });
                if ok_cancel(ui, &mut close) {
                    let mut doc = App::localized_document(
                        name.clone(),
                        Size::new(width.max(1.0), height.max(1.0)),
                    );
                    doc.metadata.resolution_dpi = app.settings.default_dpi;
                    app.page = doc.pages[0].id;
                    app.engine.replace(doc);
                    app.selection.clear();
                    app.file = None;
                    app.fit_pending = true;
                    app.show_welcome = false;
                }
            });
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
                    if ui.button(tr("dialog.close")).clicked() {
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
                if ui.button(tr("dialog.close")).clicked() {
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
                if ui.button(tr("dialog.close")).clicked() {
                    close = true;
                }
            });
        }
    }
    app.dialog = if close { Dialog::None } else { dialog };
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

fn options_dialog(app: &mut App, ctx: &Context, close: &mut bool) {
    egui::Window::new(tr("dialog.options"))
        .collapsible(false)
        .resizable(true)
        .default_width(640.0)
        .default_height(440.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(150.0);
                    for p in OptionsPage::ALL {
                        if ui
                            .selectable_label(app.options_page == p, tr(p.key()))
                            .clicked()
                        {
                            app.options_page = p;
                        }
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_min_width(440.0);
                    match app.options_page {
                        OptionsPage::General => {
                            ui.strong(tr("options.general"));
                            ui.checkbox(
                                &mut app.settings.show_welcome_on_start,
                                tr("options.show_welcome"),
                            );
                            let u = app.units;
                            let mut nudge = u.from_mm(app.nudge_mm);
                            ui.horizontal(|ui| {
                                ui.label(tr("options.nudge"));
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut nudge)
                                            .speed(0.1)
                                            .suffix(format!(" {}", u.short())),
                                    )
                                    .changed()
                                    && nudge > 0.0
                                {
                                    app.nudge_mm = u.to_mm(nudge);
                                }
                            });
                            let mut dx = u.from_mm(app.duplicate_offset.x);
                            let mut dy = u.from_mm(app.duplicate_offset.y);
                            ui.horizontal(|ui| {
                                ui.label(tr("options.duplicate_offset"));
                                ui.add(
                                    egui::DragValue::new(&mut dx)
                                        .speed(0.1)
                                        .suffix(format!(" {}", u.short())),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut dy)
                                        .speed(0.1)
                                        .suffix(format!(" {}", u.short())),
                                );
                            });
                            app.duplicate_offset =
                                tracedraw_core::geometry::Vec2::new(u.to_mm(dx), u.to_mm(dy));
                            ui.horizontal(|ui| {
                                ui.label(tr("options.default_dpi"));
                                ui.add(
                                    egui::DragValue::new(&mut app.settings.default_dpi)
                                        .range(36.0..=2400.0),
                                );
                            });
                        }
                        OptionsPage::Workspace => {
                            ui.strong(tr("options.workspace"));
                            ui.horizontal(|ui| {
                                ui.label(tr("options.language"));
                                let current = crate::i18n::language();
                                let name = crate::i18n::LANGUAGES
                                    .iter()
                                    .find(|(c, _)| *c == current)
                                    .map(|(_, n)| *n)
                                    .unwrap_or("English");
                                egui::ComboBox::from_id_salt("opt_lang")
                                    .selected_text(name)
                                    .show_ui(ui, |ui| {
                                        for (code, n) in crate::i18n::LANGUAGES {
                                            if ui.selectable_label(current == code, n).clicked() {
                                                crate::i18n::set_language(code);
                                                app.settings.language = code.to_string();
                                            }
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label(tr("options.units"));
                                egui::ComboBox::from_id_salt("opt_units")
                                    .selected_text(app.units.label())
                                    .show_ui(ui, |ui| {
                                        for u in Units::ALL {
                                            if ui
                                                .selectable_label(app.units == u, u.label())
                                                .clicked()
                                            {
                                                app.units = u;
                                            }
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label(tr("options.workspace_layout"));
                                for ws in [
                                    crate::app::Workspace::Default,
                                    crate::app::Workspace::Lite,
                                    crate::app::Workspace::Classic,
                                    crate::app::Workspace::Illustration,
                                    crate::app::Workspace::PageLayout,
                                ] {
                                    if ui
                                        .selectable_label(
                                            app.workspace == ws,
                                            tr(&format!("menu.window.workspace_{}", ws.id())),
                                        )
                                        .clicked()
                                    {
                                        app.set_workspace(ws);
                                    }
                                }
                            });
                            ui.separator();
                            ui.strong(tr("options.display"));
                            ui.checkbox(&mut app.show_rulers, tr("menu.view.rulers"));
                            ui.checkbox(&mut app.show_grid, tr("menu.view.document_grid"));
                            ui.checkbox(&mut app.show_guides, tr("menu.view.guidelines"));
                            ui.checkbox(
                                &mut app.show_status_bar,
                                tr("menu.window.toolbar_status_bar"),
                            );
                            ui.checkbox(&mut app.show_page_border, tr("menu.view.page_border"));
                        }
                        OptionsPage::PageSize => {
                            ui.strong(tr("options.page_size"));
                            let s = app.page_size();
                            let (mut w, mut h) = (s.width, s.height);
                            let mut changed =
                                unit_value(ui, &tr("dialog.width"), &mut w, app.units);
                            changed |= unit_value(ui, &tr("dialog.height"), &mut h, app.units);
                            ui.horizontal_wrapped(|ui| {
                                for (n, ps) in paper_presets() {
                                    if ui.button(n).clicked() {
                                        w = ps.width;
                                        h = ps.height;
                                        changed = true;
                                    }
                                }
                            });
                            if changed {
                                let page = app.page;
                                app.run(Command::ResizePage {
                                    page,
                                    size: Size::new(w.max(1.0), h.max(1.0)),
                                });
                                app.fit_pending = true;
                            }
                            ui.horizontal(|ui| {
                                ui.label(tr("options.rendering_resolution"));
                                let mut md = app.doc().metadata.clone();
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut md.resolution_dpi)
                                            .range(36.0..=2400.0)
                                            .suffix(" dpi"),
                                    )
                                    .changed()
                                {
                                    app.run(Command::SetMetadata { metadata: md });
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label(tr("options.bleed"));
                                let mut md = app.doc().metadata.clone();
                                let mut b = app.units.from_mm(md.bleed);
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut b)
                                            .speed(0.1)
                                            .suffix(format!(" {}", app.units.short())),
                                    )
                                    .changed()
                                {
                                    md.bleed = app.units.to_mm(b).max(0.0);
                                    app.run(Command::SetMetadata { metadata: md });
                                }
                                ui.checkbox(&mut app.show_bleed, tr("options.show_bleed"));
                            });
                        }
                        OptionsPage::Layout => {
                            ui.strong(tr("options.layout"));
                            let n = app.doc().pages.len();
                            ui.label(trf("options.pages_count", &[("n", &n.to_string())]));
                            ui.checkbox(&mut app.page_sorter, tr("menu.view.page_sorter"));
                            if ui.button(tr("menu.layout.switch_orientation")).clicked() {
                                let s = app.page_size();
                                let page = app.page;
                                app.run(Command::ResizePage {
                                    page,
                                    size: Size::new(s.height, s.width),
                                });
                                app.fit_pending = true;
                            }
                            ui.separator();
                            ui.strong(tr("options.master_layers"));
                            let masters: Vec<(
                                tracedraw_core::LayerId,
                                String,
                                tracedraw_core::MasterScope,
                            )> = app
                                .doc()
                                .master
                                .iter()
                                .map(|l| (l.id, l.name.clone(), l.scope))
                                .collect();
                            for (id, name, scope) in masters {
                                ui.horizontal(|ui| {
                                    ui.label(&name);
                                    for (s, k) in [
                                        (tracedraw_core::MasterScope::All, "options.scope_all"),
                                        (tracedraw_core::MasterScope::Odd, "options.scope_odd"),
                                        (tracedraw_core::MasterScope::Even, "options.scope_even"),
                                    ] {
                                        if ui.selectable_label(scope == s, tr(k)).clicked() {
                                            app.run(Command::SetLayerScope {
                                                layer: id,
                                                scope: s,
                                            });
                                        }
                                    }
                                });
                            }
                            ui.horizontal(|ui| {
                                for (s, k) in [
                                    (tracedraw_core::MasterScope::All, "options.new_master_all"),
                                    (tracedraw_core::MasterScope::Odd, "options.new_master_odd"),
                                    (tracedraw_core::MasterScope::Even, "options.new_master_even"),
                                ] {
                                    if ui.button(tr(k)).clicked() {
                                        let n = app.doc().master.len() + 1;
                                        app.run(Command::AddMasterLayer {
                                            name: format!("Master {n}"),
                                            scope: s,
                                        });
                                    }
                                }
                            });
                        }
                        OptionsPage::Background => {
                            ui.strong(tr("options.background"));
                            let page = app.page;
                            let current =
                                app.doc().page(page).ok().and_then(|p| p.background.clone());
                            let mut kind = match &current {
                                None => 0,
                                Some(Fill::Solid(_)) => 1,
                                Some(_) => 2,
                            };
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut kind, 0, tr("options.no_background"));
                                ui.radio_value(&mut kind, 1, tr("options.solid"));
                                ui.radio_value(&mut kind, 2, tr("options.bitmap"));
                            });
                            match kind {
                                0 => {
                                    if current.is_some() {
                                        app.run(Command::SetPageBackground {
                                            page,
                                            background: None,
                                        });
                                    }
                                }
                                1 => {
                                    let mut c = match &current {
                                        Some(Fill::Solid(c)) => *c,
                                        _ => Color::WHITE,
                                    };
                                    let [r, g, b] = c.to_rgb8();
                                    let mut rgb = [r, g, b];
                                    let changed = ui.color_edit_button_srgb(&mut rgb).changed();
                                    if changed || !matches!(current, Some(Fill::Solid(_))) {
                                        c = Color::rgb8(rgb[0], rgb[1], rgb[2]);
                                        app.run(Command::SetPageBackground {
                                            page,
                                            background: Some(Fill::Solid(c)),
                                        });
                                    }
                                }
                                _ => {
                                    if ui.button(tr("options.choose_bitmap")).clicked() {
                                        if let Some(p) = rfd::FileDialog::new()
                                            .add_filter(
                                                "Images",
                                                &[
                                                    "png", "jpg", "jpeg", "bmp", "gif", "webp",
                                                    "tif", "tiff",
                                                ],
                                            )
                                            .pick_file()
                                        {
                                            if let Ok(img) = image::open(&p) {
                                                let img = img.to_rgba8();
                                                if let Some(png) = crate::bitmap_fx::encode(&img) {
                                                    let size = app.page_size();
                                                    app.run(Command::SetPageBackground {
                                                        page,
                                                        background: Some(Fill::Pattern(
                                                            tracedraw_core::Pattern::Bitmap {
                                                                png,
                                                                width_px: img.width(),
                                                                height_px: img.height(),
                                                                size_mm: size
                                                                    .width
                                                                    .max(size.height),
                                                            },
                                                        )),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        OptionsPage::Guidelines => {
                            ui.strong(tr("options.guidelines"));
                            crate::ui::dockers2::guidelines_editor(app, ui);
                        }
                        OptionsPage::Grid => {
                            ui.strong(tr("options.grid"));
                            ui.checkbox(&mut app.show_grid, tr("options.show_grid"));
                            ui.checkbox(&mut app.snap.grid, tr("options.snap_to_grid"));
                            let u = app.units;
                            let mut g = u.from_mm(app.snap.grid_mm);
                            ui.horizontal(|ui| {
                                ui.label(tr("options.grid_spacing"));
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut g)
                                            .speed(0.5)
                                            .suffix(format!(" {}", u.short())),
                                    )
                                    .changed()
                                    && g > 0.0
                                {
                                    app.snap.grid_mm = u.to_mm(g);
                                }
                            });
                            ui.checkbox(&mut app.show_pixel_grid, tr("menu.view.pixel_grid"));
                            ui.checkbox(&mut app.show_baseline_grid, tr("menu.view.baseline_grid"));
                            ui.horizontal(|ui| {
                                ui.label(tr("options.baseline_spacing"));
                                let mut b = u.from_mm(app.settings.baseline_grid_mm);
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut b)
                                            .speed(0.1)
                                            .suffix(format!(" {}", u.short())),
                                    )
                                    .changed()
                                    && b > 0.0
                                {
                                    app.settings.baseline_grid_mm = u.to_mm(b);
                                }
                            });
                        }
                        OptionsPage::Rulers => {
                            ui.strong(tr("options.rulers"));
                            ui.checkbox(&mut app.show_rulers, tr("options.show_rulers"));
                            ui.horizontal(|ui| {
                                ui.label(tr("options.units"));
                                for u in Units::ALL {
                                    if ui.selectable_label(app.units == u, u.label()).clicked() {
                                        app.units = u;
                                    }
                                }
                            });
                            ui.label(
                                egui::RichText::new(tr("options.rulers_origin_hint"))
                                    .color(Tokens::TEXT_DIM),
                            );
                        }
                        OptionsPage::Save => {
                            ui.strong(tr("options.save"));
                            ui.label(tr("options.save_hint"));
                            if let Some(p) = crate::settings::Settings::path() {
                                ui.label(
                                    egui::RichText::new(p.display().to_string())
                                        .color(Tokens::TEXT_DIM)
                                        .size(11.0),
                                );
                            }
                        }
                        OptionsPage::Shortcuts => {
                            ui.strong(tr("options.shortcuts"));
                            ui.label(
                                egui::RichText::new(tr("options.shortcuts_hint"))
                                    .color(Tokens::TEXT_DIM),
                            );
                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    egui::Grid::new("shortcuts").striped(true).show(ui, |ui| {
                                        for t in crate::tools::Tool::ALL {
                                            ui.label(t.name());
                                            let current = app
                                                .settings
                                                .shortcuts
                                                .iter()
                                                .find(|(k, _)| k == t.id())
                                                .map(|(_, v)| v.clone())
                                                .unwrap_or_else(|| {
                                                    t.shortcut().unwrap_or("").to_string()
                                                });
                                            let mut v = current.clone();
                                            if ui
                                                .add(
                                                    egui::TextEdit::singleline(&mut v)
                                                        .desired_width(100.0),
                                                )
                                                .lost_focus()
                                                && v != current
                                            {
                                                app.settings.shortcuts.retain(|(k, _)| k != t.id());
                                                if !v.is_empty() {
                                                    app.settings
                                                        .shortcuts
                                                        .push((t.id().to_string(), v));
                                                }
                                            }
                                            ui.end_row();
                                        }
                                    });
                                });
                            if ui.button(tr("options.reset_shortcuts")).clicked() {
                                app.settings.shortcuts.clear();
                            }
                        }
                        OptionsPage::Tools => {
                            ui.strong(tr("options.tools"));
                            ui.horizontal(|ui| {
                                ui.label(tr("tool.polygon"));
                                ui.add(
                                    egui::DragValue::new(&mut app.polygon_points).range(3..=500),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label(tr("tool.spiral"));
                                ui.add(
                                    egui::DragValue::new(&mut app.spiral_revolutions)
                                        .range(1..=100),
                                );
                            });
                            ui.checkbox(
                                &mut app.settings.show_outline_flyout,
                                tr("options.show_outline_flyout"),
                            );
                            ui.horizontal(|ui| {
                                ui.label(tr("options.zoom_wheel_hint"));
                            });
                        }
                        OptionsPage::Text => {
                            ui.strong(tr("options.text"));
                            ui.horizontal(|ui| {
                                ui.label(tr("options.default_font"));
                                egui::ComboBox::from_id_salt("opt_font")
                                    .selected_text(app.text_font.clone())
                                    .show_ui(ui, |ui| {
                                        for f in app.font_families.clone() {
                                            if ui.selectable_label(app.text_font == f, &f).clicked()
                                            {
                                                app.text_font = f;
                                            }
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label(tr("options.default_size"));
                                ui.add(
                                    egui::DragValue::new(&mut app.text_size_pt)
                                        .range(1.0..=999.0)
                                        .suffix(" pt"),
                                );
                            });
                            ui.checkbox(&mut app.text_hyphenation, tr("menu.text.use_hyphenation"));
                            ui.checkbox(
                                &mut app.show_non_printing,
                                tr("menu.text.show_non_printing"),
                            );
                        }
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(tr("dialog.ok")).clicked() {
                    app.save_settings();
                    *close = true;
                }
                if ui.button(tr("dialog.cancel")).clicked() {
                    *close = true;
                }
                if ui
                    .button(tr("menu.tools.save_settings_as_default"))
                    .clicked()
                {
                    app.save_defaults();
                }
            });
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
            changed |= ui.text_edit_multiline(&mut md.notes).lost_focus();
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
        if ui.button(tr("dialog.close")).clicked() {
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
        let raster = st.format >= 2 && st.format <= 7;
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
                ui.add(egui::Slider::new(&mut st.quality, 1..=100).text(tr("dialog.quality")));
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
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(name, &[ext])
                .set_file_name(format!("{stem}.{ext}"))
                .save_file()
            {
                match crate::export::export(app, &path, st) {
                    Ok(msg) => app.status = msg,
                    Err(e) => app.status = format!("{}: {e}", tr("dialog.export_failed")),
                }
            }
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
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("CSV", &["csv", "txt"])
                    .pick_file()
                {
                    if let Ok(text) = std::fs::read_to_string(&p) {
                        let (h, r) = crate::export::parse_csv(&text);
                        st.headers = h;
                        st.rows = r;
                        st.path = Some(p);
                    }
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

fn find_replace_dialog(app: &mut App, ctx: &Context, st: &mut FindReplaceState, close: &mut bool) {
    egui::Window::new(tr("dialog.find_replace"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-40.0, 80.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut st.kind, 0, tr("dialog.find_text"));
                ui.radio_value(&mut st.kind, 1, tr("dialog.find_objects"));
            });
            ui.horizontal(|ui| {
                ui.label(tr("dialog.find"));
                ui.text_edit_singleline(&mut st.find);
            });
            if st.kind == 0 {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.replace_with"));
                    ui.text_edit_singleline(&mut st.replace);
                });
                ui.checkbox(&mut st.match_case, tr("dialog.match_case"));
                ui.checkbox(&mut st.whole_word, tr("dialog.whole_word"));
            } else {
                ui.label(
                    egui::RichText::new(tr("dialog.find_objects_hint"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
            }
            ui.horizontal(|ui| {
                if ui.button(tr("dialog.find_next")).clicked() {
                    st.results = crate::export::find_shapes(
                        app,
                        &st.find,
                        st.kind,
                        st.match_case,
                        st.whole_word,
                    );
                    if !st.results.is_empty() {
                        st.cursor = (st.cursor + 1) % st.results.len();
                        let id = st.results[st.cursor];
                        app.select(vec![id]);
                        app.zoom_to_selection();
                    }
                }
                if ui.button(tr("dialog.find_all")).clicked() {
                    st.results = crate::export::find_shapes(
                        app,
                        &st.find,
                        st.kind,
                        st.match_case,
                        st.whole_word,
                    );
                    app.select(st.results.clone());
                }
                if st.kind == 0 {
                    if ui.button(tr("dialog.replace")).clicked() {
                        crate::export::replace_text(
                            app,
                            &st.find,
                            &st.replace,
                            st.match_case,
                            st.whole_word,
                            false,
                        );
                    }
                    if ui.button(tr("dialog.replace_all")).clicked() {
                        crate::export::replace_text(
                            app,
                            &st.find,
                            &st.replace,
                            st.match_case,
                            st.whole_word,
                            true,
                        );
                    }
                }
            });
            ui.label(trf(
                "dialog.found_n",
                &[("n", &st.results.len().to_string())],
            ));
            if ui.button(tr("dialog.close")).clicked() {
                *close = true;
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
        ui.add(egui::Slider::new(&mut s.detail, 0.0..=100.0).text(tr("dialog.detail")));
        ui.add(egui::Slider::new(&mut s.smoothing, 0.0..=100.0).text(tr("dialog.smoothing")));
        ui.add(
            egui::Slider::new(&mut s.corner_smoothness, 0.0..=100.0)
                .text(tr("dialog.corner_smoothness")),
        );
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
        if ui.button(tr("dialog.close")).clicked() {
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
        if ui.button(tr("dialog.close")).clicked() {
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
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("MyThes", &["dat"])
            .pick_file()
        {
            app.settings.thesaurus_file = Some(p);
            app.settings.save();
            thesaurus_lookup(app, st);
        }
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
            if ui.button(tr("dialog.ok")).clicked() {
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
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("Palette", &["tdpal", "gpl"])
                        .set_file_name(format!("{}.tdpal", pal.name))
                        .save_file()
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
