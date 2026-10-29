//! File > Print Preview: the sheets exactly as File > Print sends them,
//! with the imposition, marks, mirror, invert and each separation plate,
//! and the print options at hand beside them (see
//! `docs/behavior/printing.md`, Print Preview).

use crate::app::App;
use crate::export::{plate_page, plates, print_sheets};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::dialogs::PrintState;
use crate::ui::tips::RichTip;
use egui::Ui;
use tracedraw_core::Document;

/// The open preview: the settings it shows, the sheet and plate on screen
/// and what was last built from them.
pub struct PrintPreview {
    pub st: PrintState,
    /// Index of the sheet shown.
    pub sheet: usize,
    /// The inks shown while Print separations is on, one per plate of
    /// [`plates`] (all at first, #571).
    pub inks: Vec<bool>,
    /// Zoom over the fitted size (1 = the sheet fits the window).
    pub zoom: f32,
    /// A drag on the sheet moves the page information, not the artwork.
    pub move_info: bool,
    /// The margin line being dragged (0 left, 1 right, 2 top, 3 bottom).
    pub margin_drag: Option<usize>,
    /// Transform: handles round the page on the sheet that stretch it
    /// (#595), and the one being dragged (x and y sides, -1, 0 or 1).
    pub transform: bool,
    pub handle_drag: Option<(i8, i8)>,
    /// The width the options panel needs for its widest row, as last
    /// laid out (0 until a row did not fit, #862).
    options_need: f32,
    built: Option<(String, Document)>,
    texture: Option<(String, egui::TextureHandle)>,
}

impl PrintPreview {
    pub fn new(st: PrintState) -> Self {
        PrintPreview {
            st,
            sheet: 0,
            inks: Vec::new(),
            zoom: 1.0,
            move_info: false,
            margin_drag: None,
            transform: false,
            handle_drag: None,
            options_need: 0.0,
            built: None,
            texture: None,
        }
    }
}

/// What the sheets depend on: every print setting but the preflight list.
fn settings_key(st: &PrintState) -> String {
    let mut st = st.clone();
    st.preflight = None;
    st.copies = 1;
    format!("{st:?}")
}

/// The sheets for the settings, built again only when they changed.
fn sheets<'a>(app: &App, pv: &'a mut PrintPreview) -> &'a Document {
    let key = settings_key(&pv.st);
    if pv.built.as_ref().map(|(k, _)| k != &key).unwrap_or(true) {
        pv.built = Some((key, print_sheets(app, &pv.st)));
        pv.texture = None;
    }
    // Just built when missing.
    &pv.built
        .get_or_insert_with(|| (String::new(), Document::default()))
        .1
}

/// The page shown: the sheet, or one plate of it.
pub fn shown_page(
    doc: &Document,
    sheet: usize,
    plate: usize,
    separations: bool,
) -> Option<tracedraw_core::document::Page> {
    let page = doc.pages.get(sheet)?;
    if !separations || plate == 0 {
        return Some(page.clone());
    }
    let p = plates(doc).get(plate - 1)?.0;
    Some(plate_page(page, p))
}

/// The page shown with the inks ticked: every ink, the sheet as it is; one
/// ink, its plate in grey (black = full ink); some, the colours those
/// inks print (#571).
pub fn shown_page_inks(
    doc: &Document,
    sheet: usize,
    inks: &[bool],
    separations: bool,
) -> Option<tracedraw_core::document::Page> {
    let page = doc.pages.get(sheet)?;
    let all = plates(doc);
    let on: Vec<usize> = (0..all.len())
        .filter(|i| inks.get(*i).copied().unwrap_or(true))
        .collect();
    if !separations || on.len() == all.len() {
        return Some(page.clone());
    }
    if let [one] = on.as_slice() {
        return shown_page(doc, sheet, one + 1, true);
    }
    let mut process = [false; 4];
    let mut spots = Vec::new();
    for i in on {
        match all[i].0 {
            crate::export::Plate::Process(k) if k < 4 => process[k] = true,
            crate::export::Plate::Spot(id) => spots.push(id),
            _ => {}
        }
    }
    Some(crate::export::inks_page(page, process, &spots))
}

/// The names of the plates, translated for the process inks.
fn plate_names(doc: &Document) -> Vec<String> {
    let mut names = vec![tr("print.composite")];
    for (i, (_, name)) in plates(doc).into_iter().enumerate() {
        names.push(match i {
            0 => tr("fxp.cyan"),
            1 => tr("fxp.magenta"),
            2 => tr("fxp.yellow"),
            3 => tr("fxp.black"),
            _ => name,
        });
    }
    names
}

/// The id of the options panel at the left of Print Preview.
const PREVIEW_OPTIONS: &str = "print_preview_options";
/// The options panel's width at first, its narrowest and the widest its
/// edge drags to (wider still when a row needs it), points.
const OPTIONS_DEFAULT: f32 = 360.0;
const OPTIONS_MIN: f32 = 300.0;
const OPTIONS_MAX: f32 = 720.0;

pub fn show(app: &mut App, ui: &mut Ui) {
    let Some(mut pv) = app.print_preview.take() else {
        return;
    };
    crate::ui::dialogs::take_saved_sheet(app, &mut pv.st);
    // The printer list, once it has been read (#792).
    if pv.st.looking_for_printers && app.poll_printers(&mut pv.st) {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(200));
    }
    let mut close = false;
    let mut print = false;
    let ctx = ui.ctx().clone();

    egui::Panel::top("print_preview_bar")
        .frame(
            egui::Frame::new()
                .fill(Tokens::menu_bar())
                .inner_margin(egui::Margin::symmetric(8, 4)),
        )
        .show(ui, |ui| {
            // One row, wrapping onto a second in a narrow window.
            ui.horizontal_wrapped(|ui| {
                // Whole labels go to the next row, never half of one.
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                if ui.button(tr("dialog.print")).clicked() {
                    print = true;
                }
                ui.separator();
                let n = sheets(app, &mut pv).pages.len().max(1);
                pv.sheet = pv.sheet.min(n - 1);
                if ui
                    .add_enabled(pv.sheet > 0, egui::Button::new("<"))
                    .rich_tip(tr("status.previous_page"))
                    .clicked()
                {
                    pv.sheet -= 1;
                }
                ui.label(trf(
                    "print.sheet_of",
                    &[
                        ("n", &(pv.sheet + 1).to_string()),
                        ("total", &n.to_string()),
                    ],
                ));
                if ui
                    .add_enabled(pv.sheet + 1 < n, egui::Button::new(">"))
                    .rich_tip(tr("status.next_page"))
                    .clicked()
                {
                    pv.sheet += 1;
                }
                ui.separator();
                ui.label(tr("print.zoom"));
                // The zoom in percent on the slider, a factor in the view.
                let mut percent = pv.zoom * 100.0;
                if ui
                    .add(
                        crate::ui::Slider::new(&mut percent, 25.0..=400.0)
                            .logarithmic(true)
                            .suffix(" %")
                            .max_decimals(0),
                    )
                    .changed()
                {
                    pv.zoom = percent / 100.0;
                }
                if ui.button(tr("print.fit")).clicked() {
                    pv.zoom = 1.0;
                }
                // The printer with its Properties and status, and the
                // copies: in the bar, so the left panel keeps its room
                // for the options (#792, #953).
                ui.separator();
                let st = &mut pv.st;
                ui.label(tr("print.printer"));
                if let Some(p) = crate::ui::dialogs::destination_row(ui, st, 220.0) {
                    if let Err(e) = crate::printers::open_properties(&p) {
                        app.status = e;
                    }
                }
                let chosen = st.printers.iter().find(|p| {
                    st.destination == 0 && st.printer.as_deref() == Some(p.name.as_str())
                });
                if let Some(p) = chosen {
                    ui.label(
                        egui::RichText::new(tr(p.state.key()))
                            .color(Tokens::text_dim())
                            .size(11.0),
                    )
                    .rich_tip(tr("print.status"));
                }
                ui.separator();
                ui.label(tr("print.number_of_copies"));
                ui.add(crate::ui::field::NumField::new(&mut st.copies).range(1..=999));
                ui.add_enabled(
                    st.copies > 1,
                    egui::Checkbox::new(&mut st.collate, tr("print.collate")),
                );
                // Close at the bar's right end, where windows keep it
                // (#534).
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(tr("dialog.close"))
                        .rich_tip(crate::ui::tips::keyed("dialog.close"))
                        .clicked()
                    {
                        close = true;
                    }
                });
            });
        });

    // The inks while Print separations is on (built sheets are kept, so
    // this is a lookup on most frames).
    let ink_names = if pv.st.separations {
        plate_names(sheets(app, &mut pv))
    } else {
        Vec::new()
    };
    pv.inks.resize(ink_names.len().saturating_sub(1), true);

    // Wide enough for the longer languages; its edge drags wider still.
    // It is never narrower than its widest row as last laid out, up to
    // half the window, so no label is cut; past that what does not fit
    // scrolls sideways (#862).
    let page_count = app.doc().pages.len();
    let has_selection = !app.selection.is_empty();
    let widest = (ui.max_rect().width() * 0.5).max(OPTIONS_MIN);
    let need = pv.options_need.clamp(OPTIONS_MIN, widest);
    let panel = egui::Panel::left(PREVIEW_OPTIONS)
        .resizable(true)
        .default_size(OPTIONS_DEFAULT.max(need))
        .size_range(need..=OPTIONS_MAX.max(need))
        .show(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                let st = &mut pv.st;
                // Two options to a row, so the panel has room for more
                // (#951): which pages print, then the marks.
                egui::Grid::new("print_preview_range")
                    .num_columns(2)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        if ui
                            .radio_value(&mut st.range, 0, tr("dialog.all_pages"))
                            .clicked()
                        {
                            st.all_pages = true;
                        }
                        ui.radio_value(&mut st.range, 1, tr("print.range_page"));
                        ui.end_row();
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut st.range, 2, tr("print.range_pages"));
                            ui.add_enabled(
                                st.range == 2,
                                egui::TextEdit::singleline(&mut st.pages)
                                    .hint_text(format!("1-{page_count}"))
                                    .desired_width(70.0),
                            );
                        });
                        ui.add_enabled_ui(has_selection, |ui| {
                            ui.radio_value(&mut st.range, 3, tr("print.range_selection"));
                        });
                        ui.end_row();
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut st.fit_to_page, tr("dialog.fit_to_page"));
                            if !st.fit_to_page {
                                ui.add(
                                    crate::ui::field::NumField::new(&mut st.scale)
                                        .range(1.0..=1000.0)
                                        .suffix(" %"),
                                );
                            }
                        });
                        crate::ui::dialogs::parity_combo(ui, st, "print_preview_parity");
                        ui.end_row();
                    });
                // As the Print dialog keeps it: every range but the
                // current page prints more than one page.
                if st.range == 3 && !has_selection {
                    st.range = 0;
                }
                st.all_pages = st.range != 1;
                ui.separator();
                ui.strong(tr("dialog.prepress"));
                ui.checkbox(&mut st.separations, tr("dialog.separations"));
                if st.separations {
                    // Each ink with a box under it, always in view: one
                    // alone shows its plate, several print together
                    // (#571).
                    ui.indent("print_preview_inks", |ui| {
                        for (on, name) in pv.inks.iter_mut().zip(ink_names.iter().skip(1)) {
                            ui.checkbox(on, name.clone());
                        }
                    });
                }
                let st = &mut pv.st;
                egui::Grid::new("print_preview_marks")
                    .num_columns(2)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        ui.checkbox(&mut st.bleed, tr("dialog.print_bleed"));
                        ui.checkbox(&mut st.crop_marks, tr("dialog.crop_marks"));
                        ui.end_row();
                        ui.checkbox(&mut st.registration_marks, tr("dialog.registration_marks"));
                        ui.checkbox(&mut st.color_bars, tr("print.color_bars"));
                        ui.end_row();
                        ui.checkbox(&mut st.densitometer, tr("print.densitometer"));
                        ui.checkbox(&mut st.page_info, tr("print.page_info_mark"));
                        ui.end_row();
                        ui.checkbox(&mut st.mirror, tr("dialog.mirror"));
                        ui.checkbox(&mut st.invert, tr("dialog.invert"));
                        ui.end_row();
                    });
                if st.page_info {
                    ui.checkbox(&mut pv.move_info, tr("print.move_info"));
                }
                let st = &mut pv.st;
                if st.info_offset != (0.0, 0.0)
                    && ui
                        .small_button(tr("print.info_reset"))
                        .rich_tip(tr("print.info_drag_tip"))
                        .clicked()
                {
                    st.info_offset = (0.0, 0.0);
                }
                // Transform: the page stretched on the sheet by handles,
                // or by these percentages (#595).
                ui.checkbox(&mut pv.transform, tr("print.transform"))
                    .rich_tip(tr("print.transform_tip"));
                let st = &mut pv.st;
                ui.horizontal(|ui| {
                    let mut x = st.art_stretch.0 * 100.0;
                    let mut y = st.art_stretch.1 * 100.0;
                    let rx = ui.add(
                        crate::ui::field::NumField::new(&mut x)
                            .range(1.0..=1000.0)
                            .speed(0.5)
                            .prefix("x ")
                            .suffix(" %")
                            .width(70.0),
                    );
                    let ry = ui.add(
                        crate::ui::field::NumField::new(&mut y)
                            .range(1.0..=1000.0)
                            .speed(0.5)
                            .prefix("y ")
                            .suffix(" %")
                            .width(70.0),
                    );
                    if rx.changed() || ry.changed() {
                        st.art_stretch = (x / 100.0, y / 100.0);
                    }
                    if ui
                        .add_enabled(
                            st.art_stretch != (1.0, 1.0),
                            egui::Button::new(tr("print.art_reset")).small(),
                        )
                        .clicked()
                    {
                        st.art_stretch = (1.0, 1.0);
                    }
                });
                // Where the artwork sits on its page: a drag on the sheet
                // moves it too (#536).
                ui.horizontal(|ui| {
                    ui.label(tr("print.art_position"));
                    ui.add(
                        crate::ui::field::NumField::new(&mut st.art_offset.0)
                            .range(-1000.0..=1000.0)
                            .speed(0.5)
                            .prefix("x ")
                            .suffix(" mm")
                            .width(70.0),
                    );
                    ui.add(
                        crate::ui::field::NumField::new(&mut st.art_offset.1)
                            .range(-1000.0..=1000.0)
                            .speed(0.5)
                            .prefix("y ")
                            .suffix(" mm")
                            .width(70.0),
                    );
                    if ui
                        .add_enabled(
                            st.art_offset != (0.0, 0.0),
                            egui::Button::new(tr("print.art_reset")).small(),
                        )
                        .clicked()
                    {
                        st.art_offset = (0.0, 0.0);
                    }
                });
                ui.separator();
                crate::ui::dialogs::print_layout_fields(app, ui, st);
            })
        });
    // A row wider than the panel (a longer language, a setting shown):
    // the panel takes its width from the next frame on, this frame not
    // shown, so nothing is drawn cut.
    let scroll = &panel.inner;
    let chrome = panel.response.rect.width() - scroll.inner_rect.width();
    let want = scroll.content_size.x + chrome;
    if scroll.content_size.x > scroll.inner_rect.width() + 0.5 && want > pv.options_need + 0.5 {
        pv.options_need = want;
        if want <= widest {
            ctx.request_discard("print preview options wider");
        }
    }

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::from_gray(90)))
        .show(ui, |ui| {
            let area = ui.available_rect_before_wrap();
            let separations = pv.st.separations;
            let sheet = pv.sheet;
            let inks = pv.inks.clone();
            let Some(page) = shown_page_inks(sheets(app, &mut pv), sheet, &inks, separations)
            else {
                return;
            };
            let plate: String = inks.iter().map(|on| if *on { '1' } else { '0' }).collect();
            let margin = 24.0;
            let fit = ((area.width() - 2.0 * margin) as f64 / page.size.width.max(1e-6))
                .min((area.height() - 2.0 * margin) as f64 / page.size.height.max(1e-6))
                .max(0.01);
            // Keep the bitmap within what a texture holds.
            let zoom = (fit * pv.zoom as f64)
                .min(8000.0 / page.size.width.max(page.size.height).max(1e-6));
            let w = (page.size.width * zoom).ceil().max(1.0) as u32;
            let h = (page.size.height * zoom).ceil().max(1.0) as u32;
            let key = format!(
                "{}|{sheet}|{plate}|{w}x{h}",
                pv.built.as_ref().map(|(k, _)| k.as_str()).unwrap_or("")
            );
            if pv.texture.as_ref().map(|(k, _)| k != &key).unwrap_or(true) {
                if let Some(img) = render_sheet(&page, w, h, zoom) {
                    let tex = ctx.load_texture("print_preview", img, egui::TextureOptions::LINEAR);
                    pv.texture = Some((key, tex));
                }
            }
            let size = egui::vec2(w as f32, h as f32);
            // A drag moves the artwork, or the page information when
            // that is asked for (#388, #536).
            let info = pv.st.page_info && pv.move_info;
            let mut moved: Option<egui::Vec2> = None;
            egui::ScrollArea::both().show(ui, |ui| {
                let avail = ui
                    .available_size()
                    .max(size + egui::vec2(2.0 * margin, 2.0 * margin));
                let (r, resp) = ui.allocate_exact_size(avail, egui::Sense::drag());
                let resp = resp.rich_tip(if info {
                    tr("print.info_drag_tip")
                } else {
                    tr("print.art_drag_tip")
                });
                let sheet_rect = egui::Rect::from_center_size(r.center(), size);
                // The margins' lines on the sheet (mm, y up).
                let k = zoom as f32;
                let m = pv.st.margins.map(|v| v.max(0.0) as f32);
                let (left, right) = (sheet_rect.left() + m[0] * k, sheet_rect.right() - m[1] * k);
                let (top, bottom) = (sheet_rect.top() + m[2] * k, sheet_rect.bottom() - m[3] * k);
                let near = |p: egui::Pos2| -> Option<usize> {
                    let inside_y = p.y >= top - 6.0 && p.y <= bottom + 6.0;
                    let inside_x = p.x >= left - 6.0 && p.x <= right + 6.0;
                    [
                        (inside_y && (p.x - left).abs() <= 6.0, 0),
                        (inside_y && (p.x - right).abs() <= 6.0, 1),
                        (inside_x && (p.y - top).abs() <= 6.0, 2),
                        (inside_x && (p.y - bottom).abs() <= 6.0, 3),
                    ]
                    .into_iter()
                    .find(|(hit, _)| *hit)
                    .map(|(_, i)| i)
                };
                // The page on the sheet (its clip frame), for Transform's
                // handles: mm, y up, then on screen.
                let cell = page.layers.first().and_then(|l| {
                    l.shapes.iter().find_map(|s| match &s.kind {
                        tracedraw_core::ShapeKind::ClipFrame { frame, .. } => match &frame.kind {
                            tracedraw_core::ShapeKind::Rect { rect, .. } => {
                                Some(s.transform.transform_rect_bbox(*rect))
                            }
                            _ => None,
                        },
                        _ => None,
                    })
                });
                let to_screen = |x: f64, y: f64| {
                    egui::pos2(
                        sheet_rect.left() + (x * zoom) as f32,
                        sheet_rect.bottom() - (y * zoom) as f32,
                    )
                };
                let handles: Vec<((i8, i8), egui::Pos2)> = match (pv.transform, cell) {
                    (true, Some(c)) => {
                        let mut v = Vec::new();
                        for hy in [-1i8, 0, 1] {
                            for hx in [-1i8, 0, 1] {
                                if hx == 0 && hy == 0 {
                                    continue;
                                }
                                let x = match hx {
                                    -1 => c.x0,
                                    0 => (c.x0 + c.x1) / 2.0,
                                    _ => c.x1,
                                };
                                let y = match hy {
                                    -1 => c.y0,
                                    0 => (c.y0 + c.y1) / 2.0,
                                    _ => c.y1,
                                };
                                v.push(((hx, hy), to_screen(x, y)));
                            }
                        }
                        v
                    }
                    _ => Vec::new(),
                };
                let handle_at = |p: egui::Pos2| {
                    handles
                        .iter()
                        .find(|(_, h)| (p - *h).length() <= 7.0)
                        .map(|(k, _)| *k)
                };
                let over_handle = resp.hover_pos().and_then(handle_at);
                let over = if over_handle.is_some() {
                    None
                } else {
                    resp.hover_pos().and_then(near)
                };
                if resp.drag_started() {
                    pv.handle_drag = resp.interact_pointer_pos().and_then(handle_at);
                    pv.margin_drag = if pv.handle_drag.is_some() {
                        None
                    } else {
                        resp.interact_pointer_pos().and_then(near)
                    };
                }
                let edge = pv.margin_drag.or(over);
                let handle = pv.handle_drag.or(over_handle);
                if resp.hovered() || pv.margin_drag.is_some() || pv.handle_drag.is_some() {
                    ui.ctx().set_cursor_icon(match (handle, edge) {
                        (Some((0, _)), _) => egui::CursorIcon::ResizeVertical,
                        (Some((_, 0)), _) => egui::CursorIcon::ResizeHorizontal,
                        (Some((hx, hy)), _) if hx == hy => egui::CursorIcon::ResizeNeSw,
                        (Some(_), _) => egui::CursorIcon::ResizeNwSe,
                        (None, Some(0 | 1)) => egui::CursorIcon::ResizeHorizontal,
                        (None, Some(_)) => egui::CursorIcon::ResizeVertical,
                        (None, None) => egui::CursorIcon::Move,
                    });
                }
                if let (true, Some((hx, hy)), Some(c)) = (resp.dragged(), pv.handle_drag, cell) {
                    // A side follows the pointer, the opposite one stays:
                    // the stretch grows by the move, the middle by half.
                    let d = resp.drag_delta();
                    let (dx, dy) = (f64::from(d.x) / zoom, -f64::from(d.y) / zoom);
                    let st = &mut pv.st;
                    if hx != 0 && c.width() > 1e-6 {
                        let w = (c.width() + f64::from(hx) * dx).max(1.0);
                        st.art_stretch.0 = (st.art_stretch.0 * w / c.width()).clamp(0.01, 10.0);
                        st.art_offset.0 += dx / 2.0;
                    }
                    if hy != 0 && c.height() > 1e-6 {
                        let h = (c.height() + f64::from(hy) * dy).max(1.0);
                        st.art_stretch.1 = (st.art_stretch.1 * h / c.height()).clamp(0.01, 10.0);
                        st.art_offset.1 += dy / 2.0;
                    }
                } else if resp.dragged() {
                    let d = resp.drag_delta();
                    match pv.margin_drag {
                        Some(i) => {
                            // Inwards makes the margin larger.
                            let along = match i {
                                0 => d.x,
                                1 => -d.x,
                                2 => d.y,
                                _ => -d.y,
                            };
                            let v = &mut pv.st.margins[i];
                            *v = (*v + f64::from(along) / zoom.max(1e-6)).clamp(0.0, 200.0);
                        }
                        None => moved = Some(d),
                    }
                }
                if resp.drag_stopped() {
                    pv.margin_drag = None;
                    pv.handle_drag = None;
                }
                let p = ui.painter();
                p.rect_filled(
                    sheet_rect.translate(egui::vec2(4.0, 4.0)),
                    0.0,
                    egui::Color32::from_black_alpha(90),
                );
                if let Some((_, tex)) = &pv.texture {
                    p.image(
                        tex.id(),
                        sheet_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                // Transform's frame and handles round the page.
                if let (true, Some(c)) = (pv.transform, cell) {
                    let r = egui::Rect::from_two_pos(to_screen(c.x0, c.y0), to_screen(c.x1, c.y1));
                    p.rect_stroke(
                        r,
                        0.0,
                        egui::Stroke::new(1.0, Tokens::ACCENT),
                        egui::StrokeKind::Middle,
                    );
                    for (k, h) in &handles {
                        let hr = egui::Rect::from_center_size(*h, egui::vec2(8.0, 8.0));
                        p.rect_filled(
                            hr,
                            0.0,
                            if handle == Some(*k) {
                                Tokens::ACCENT
                            } else {
                                egui::Color32::WHITE
                            },
                        );
                        p.rect_stroke(
                            hr,
                            0.0,
                            egui::Stroke::new(1.0, Tokens::ACCENT),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
                // The margins dashed in blue, the one under the pointer or
                // dragged bolder.
                for (i, (a, b)) in [
                    (
                        egui::pos2(left, sheet_rect.top()),
                        egui::pos2(left, sheet_rect.bottom()),
                    ),
                    (
                        egui::pos2(right, sheet_rect.top()),
                        egui::pos2(right, sheet_rect.bottom()),
                    ),
                    (
                        egui::pos2(sheet_rect.left(), top),
                        egui::pos2(sheet_rect.right(), top),
                    ),
                    (
                        egui::pos2(sheet_rect.left(), bottom),
                        egui::pos2(sheet_rect.right(), bottom),
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    let width = if edge == Some(i) { 2.0 } else { 1.0 };
                    p.extend(egui::Shape::dashed_line(
                        &[a, b],
                        egui::Stroke::new(width, Tokens::ACCENT),
                        5.0,
                        4.0,
                    ));
                }
            });
            if let Some(d) = moved {
                // Screen points over the sheet are `zoom` per millimetre;
                // the sheet's y runs up.
                let k = zoom.max(1e-6);
                let target = if info {
                    &mut pv.st.info_offset
                } else {
                    &mut pv.st.art_offset
                };
                target.0 += f64::from(d.x) / k;
                target.1 -= f64::from(d.y) / k;
            }
        });

    let (prev, next, esc) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::PageUp),
            i.key_pressed(egui::Key::PageDown),
            i.key_pressed(egui::Key::Escape),
        )
    });
    if prev {
        pv.sheet = pv.sheet.saturating_sub(1);
    }
    if next {
        pv.sheet += 1;
    }
    if esc {
        close = true;
    }
    if print {
        // The inks unticked here do not print (#846).
        pv.st.plates_off = pv
            .inks
            .iter()
            .enumerate()
            .filter(|(_, on)| !**on)
            .map(|(i, _)| i)
            .collect();
        match crate::export::print(app, &pv.st) {
            Ok(msg) => app.status = msg,
            Err(e) => app.status = format!("{}: {e}", tr("dialog.print_failed")),
        }
        if app.settings.printing.remember_last {
            app.settings.printing.remember(&pv.st);
            app.settings.save();
        }
        close = true;
    }
    if !close {
        app.print_preview = Some(pv);
    }
}

/// The sheet as a bitmap on white paper.
fn render_sheet(
    page: &tracedraw_core::document::Page,
    w: u32,
    h: u32,
    zoom: f64,
) -> Option<egui::ColorImage> {
    let mut doc = Document::new("sheet", page.size);
    doc.pages = vec![page.clone()];
    let opts = tracedraw_render::RenderOptions {
        width: w,
        height: h,
        view: tracedraw_render::ViewTransform {
            zoom,
            origin_x: 0.0,
            origin_y: page.size.height * zoom,
        },
        preview: None,
        wireframe: false,
        ..tracedraw_render::RenderOptions::default()
    };
    let pm = tracedraw_render::render_page(&doc, page.id, &opts)?;
    let mut white = tiny_skia::Pixmap::new(pm.width(), pm.height())?;
    white.fill(tiny_skia::Color::WHITE);
    white.draw_pixmap(
        0,
        0,
        pm.as_ref(),
        &Default::default(),
        tiny_skia::Transform::identity(),
        None,
    );
    Some(egui::ColorImage::from_rgba_premultiplied(
        [white.width() as usize, white.height() as usize],
        white.data(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{Color, Fill, Shape, ShapeKind};

    #[test]
    fn plates_split_the_sheet_by_ink() {
        let mut doc = Document::new("t", tracedraw_core::geometry::Size::new(100.0, 100.0));
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Solid(Color::Cmyk {
            c: 1.0,
            m: 0.5,
            y: 0.0,
            k: 0.0,
        });
        doc.pages[0].layers[0].shapes.push(s);
        let grey = |plate| {
            let p = shown_page(&doc, 0, plate, true).expect("page");
            match p.layers[0].shapes[0].fill {
                Fill::Solid(Color::Gray { v }) => v,
                ref f => panic!("{f:?}"),
            }
        };
        assert!((grey(1) - 0.0).abs() < 1e-6, "cyan plate full");
        assert!((grey(2) - 0.5).abs() < 1e-6, "magenta plate half");
        assert!((grey(3) - 1.0).abs() < 1e-6, "yellow plate empty");
        // Composite, and separations off, show the sheet as it is.
        let p = shown_page(&doc, 0, 0, true).expect("page");
        assert!(matches!(
            p.layers[0].shapes[0].fill,
            Fill::Solid(Color::Cmyk { .. })
        ));
        let p = shown_page(&doc, 0, 2, false).expect("page");
        assert!(matches!(
            p.layers[0].shapes[0].fill,
            Fill::Solid(Color::Cmyk { .. })
        ));
        assert!(shown_page(&doc, 5, 0, false).is_none());
        // Ticked inks (#571): one alone is its plate in grey; cyan and
        // yellow together keep only those channels; every ink, the sheet.
        let fill = |inks: &[bool]| {
            shown_page_inks(&doc, 0, inks, true).expect("page").layers[0].shapes[0]
                .fill
                .clone()
        };
        assert!(
            matches!(fill(&[false, true, false, false]), Fill::Solid(Color::Gray { v }) if (v - 0.5).abs() < 1e-6)
        );
        match fill(&[true, false, true, false]) {
            Fill::Solid(Color::Cmyk { c, m, y, k }) => {
                assert!((c - 1.0).abs() < 1e-6 && m == 0.0 && y == 0.0 && k == 0.0)
            }
            f => panic!("{f:?}"),
        }
        assert_eq!(
            fill(&[true; 4]),
            Fill::Solid(Color::Cmyk {
                c: 1.0,
                m: 0.5,
                y: 0.0,
                k: 0.0
            })
        );
        assert_eq!(
            fill(&[]),
            Fill::Solid(Color::Cmyk {
                c: 1.0,
                m: 0.5,
                y: 0.0,
                k: 0.0
            }),
            "all at first"
        );
    }

    /// With Print separations on, a box for each ink stands under that
    /// option in the left panel, as the request's sketch has them; they
    /// were in the top bar, where many spot inks ran past the window
    /// (#571).
    #[test]
    fn the_ink_boxes_stand_under_print_separations() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let st = PrintState {
            separations: true,
            ..PrintState::default()
        };
        app.print_preview = Some(PrintPreview::new(st));
        let find = |shapes: &[egui::epaint::ClippedShape], text: &str| {
            shapes.iter().find_map(|c| match &c.shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == text => Some(t.pos),
                _ => None,
            })
        };
        let mut shapes = Vec::new();
        for _ in 0..2 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1400.0, 900.0),
                    )),
                    ..Default::default()
                },
                |ui| show(&mut app, ui),
            );
            out.textures_delta.clear();
            shapes = out.shapes;
        }
        assert_eq!(app.print_preview.as_ref().map(|p| p.inks.len()), Some(4));
        let option = find(&shapes, &tr("dialog.separations")).expect("the option");
        for ink in ["fxp.cyan", "fxp.magenta", "fxp.yellow", "fxp.black"] {
            let at = find(&shapes, &tr(ink)).expect(ink);
            assert!(at.x < 330.0, "{ink} in the left panel: {at:?}");
            assert!(at.y > option.y && at.x > option.x, "{ink} under it: {at:?}");
        }
    }

    /// Print Preview shows the printer choice, the Properties button of
    /// the printer chosen and the copies in its top bar, beside Print
    /// (they were at the top of the left panel, #953), the printers
    /// coming in when the system has listed them; the Print button prints
    /// to the printer chosen there (#792).
    #[test]
    fn the_preview_offers_the_printer_and_the_copies() {
        use crate::printers::{Printer, PrinterState};
        fn list() -> Vec<Printer> {
            vec![
                Printer {
                    name: "Studio Laser".into(),
                    default: true,
                    state: PrinterState::Ready,
                    ..Default::default()
                },
                Printer {
                    name: "Plotter".into(),
                    ..Default::default()
                },
            ]
        }
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.printers_seen.clear();
        app.print_preview = Some(PrintPreview::new(PrintState {
            looking_for_printers: true,
            ..PrintState::default()
        }));
        app.fetch_printers(list);
        let texts = |shapes: &[egui::epaint::ClippedShape]| -> Vec<(String, egui::Pos2)> {
            fn walk(s: &egui::Shape, out: &mut Vec<(String, egui::Pos2)>) {
                match s {
                    egui::Shape::Text(t) => out.push((t.galley.text().to_string(), t.pos)),
                    egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                    _ => {}
                }
            }
            let mut out = Vec::new();
            shapes.iter().for_each(|c| walk(&c.shape, &mut out));
            out
        };
        for _ in 0..200 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1400.0, 900.0),
                    )),
                    ..Default::default()
                },
                |ui| show(&mut app, ui),
            );
            out.textures_delta.clear();
            if app
                .print_preview
                .as_ref()
                .is_some_and(|p| !p.st.looking_for_printers)
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let st = &app.print_preview.as_ref().expect("preview").st;
        assert_eq!(st.printers.len(), 2, "the printers came in");
        assert_eq!(st.printer.as_deref(), Some("Studio Laser"), "the default");
        // One more frame to show them.
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                ..Default::default()
            },
            |ui| show(&mut app, ui),
        );
        out.textures_delta.clear();
        let t = texts(&out.shapes);
        let at = |label: &str| t.iter().find(|(s, _)| s == label).map(|(_, p)| *p);
        let printer = at("Studio Laser").expect("the printer chosen");
        let copies = at(&tr("print.number_of_copies")).expect("the copies");
        let all_pages = at(&tr("dialog.all_pages")).expect("the other options");
        let print = at(&tr("dialog.print")).expect("the Print button");
        // In the top bar, after Print, the left panel starting with the
        // pages to print (#953).
        assert!((printer.y - print.y).abs() < 8.0, "{printer:?} {print:?}");
        assert!((copies.y - print.y).abs() < 8.0, "{copies:?} {print:?}");
        assert!(
            printer.x > print.x && copies.x > printer.x,
            "{print:?} {printer:?} {copies:?}"
        );
        assert!(all_pages.x < 330.0 && all_pages.y > printer.y + 15.0);
        assert!(at(&tr("print.collate")).is_some());
        assert!(at(&tr("print.printer")).is_some_and(|p| p.y < all_pages.y));
    }

    #[test]
    fn the_settings_key_ignores_copies_and_preflight() {
        let a = PrintState::default();
        let mut b = a.clone();
        b.copies = 4;
        assert_eq!(settings_key(&a), settings_key(&b));
        b.crop_marks = true;
        assert_ne!(settings_key(&a), settings_key(&b));
    }

    /// A page smaller than the paper, repeated on it: every copy shows in
    /// the preview, not only the bottom row with white over the rest
    /// (#848).
    #[test]
    fn every_repeated_page_shows_in_the_preview() {
        let mut doc = Document::new("t", tracedraw_core::geometry::Size::new(90.0, 50.0));
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(0.0, 0.0, 90.0, 50.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(230, 30, 30));
        s.stroke = None;
        doc.pages[0].layers[0].shapes.push(s);
        let o = crate::imposition::Options {
            layout: crate::imposition::Layout::Repeat { cols: 2, rows: 5 },
            sheet: Some(tracedraw_core::geometry::Size::new(210.0, 297.0)),
            ..Default::default()
        };
        let sheets = crate::imposition::impose(&doc, &o);
        let page = &sheets.pages[0];
        let plan = crate::imposition::plan(&[doc.pages[0].size], &o);
        let zoom = 3.0;
        let (w, h) = (630usize, 891usize);
        let img = render_sheet(page, w as u32, h as u32, zoom).expect("image");
        assert_eq!(img.size, [w, h]);
        assert_eq!(plan[0].cells.len(), 10);
        for c in &plan[0].cells {
            let m = c.trim.center();
            let (x, y) = ((m.x * zoom) as usize, h - (m.y * zoom) as usize);
            let px = img.pixels[y * w + x];
            assert!(px.r() > 200 && px.g() < 80, "cell at {m:?}: {px:?}");
        }
    }

    /// The options at the left of Print Preview come wider than before
    /// and their edge drags wider, so the longer languages are not cut
    /// off (#862).
    #[test]
    fn the_options_panel_is_wider_and_its_edge_drags() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        app.print_preview(None);
        let run = |app: &mut App, events: Vec<egui::Event>| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1400.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| show(app, ui),
            );
            out.textures_delta.clear();
        };
        for _ in 0..3 {
            run(&mut app, Vec::new());
        }
        let id = egui::Id::new(PREVIEW_OPTIONS);
        let width = || {
            egui::containers::panel::PanelState::load(&ctx, id)
                .map(|s| s.outer_rect.width())
                .expect("the panel's state")
        };
        let start = width();
        assert!(start >= 355.0, "{start}");
        let right = egui::containers::panel::PanelState::load(&ctx, id)
            .map(|s| s.outer_rect.right())
            .expect("the panel's state");
        let edge = egui::pos2(right - 2.0, 450.0);
        let to = edge + egui::vec2(120.0, 0.0);
        run(&mut app, vec![egui::Event::PointerMoved(edge)]);
        run(
            &mut app,
            vec![egui::Event::PointerButton {
                pos: edge,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        for k in 1..=6 {
            run(
                &mut app,
                vec![egui::Event::PointerMoved(
                    edge + (to - edge) * (k as f32 / 6.0),
                )],
            );
        }
        run(
            &mut app,
            vec![egui::Event::PointerButton {
                pos: to,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        run(&mut app, Vec::new());
        assert!(width() > start + 80.0, "{start} -> {}", width());
        assert!(app.print_preview.is_some());
    }

    /// In every language, at the main window's default size, no label of
    /// the options panel is cut at its right: the panel takes the width of
    /// its widest row (Spanish in the report, #862), with the layout
    /// options that add rows (Repeat, a sheet size, page information) on.
    #[test]
    fn no_option_is_cut_off_in_any_language() {
        let size = crate::splash::DEFAULT_SIZE;
        for code in [
            "en", "es", "pt-BR", "fr", "de", "it", "nl", "pl", "cs", "ru", "tr", "id", "ar", "hi",
            "bn", "ja", "zh-CN", "zh-TW",
        ] {
            crate::i18n::with_thread_language(code, || {
                let ctx = crate::theme::ui_context();
                let mut app = App::headless();
                app.show_welcome = false;
                app.print_preview(None);
                if let Some(pv) = app.print_preview.as_mut() {
                    pv.st.looking_for_printers = false;
                    pv.st.layout_kind = 3;
                    pv.st.sheet = 1;
                    pv.st.page_info = true;
                    pv.st.copies = 2;
                }
                let mut shapes = Vec::new();
                for _ in 0..8 {
                    let mut out = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                            ..Default::default()
                        },
                        |ui| show(&mut app, ui),
                    );
                    out.textures_delta.clear();
                    shapes = out.shapes;
                }
                let panel =
                    egui::containers::panel::PanelState::load(&ctx, egui::Id::new(PREVIEW_OPTIONS))
                        .map(|s| s.outer_rect)
                        .expect("the panel's state");
                fn walk(
                    s: &egui::Shape,
                    clip: egui::Rect,
                    out: &mut Vec<(String, egui::Rect, egui::Rect)>,
                ) {
                    match s {
                        egui::Shape::Text(t) => {
                            out.push((t.galley.text().to_string(), t.visual_bounding_rect(), clip))
                        }
                        egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, clip, out)),
                        _ => {}
                    }
                }
                let mut texts = Vec::new();
                for c in &shapes {
                    walk(&c.shape, c.clip_rect, &mut texts);
                }
                // The top bar with the printer and the copies (#953)
                // stays inside the window, wrapping when it must.
                let close = tr("dialog.close");
                for (text, r, _) in &texts {
                    if r.top() < 80.0 {
                        assert!(
                            r.right() <= size.x + 1.0,
                            "{code}: {text:?} past the window {r:?}"
                        );
                    }
                }
                assert!(
                    texts
                        .iter()
                        .any(|(t, r, _)| *t == close && r.right() <= size.x),
                    "{code}: Close shows"
                );
                // The panel's texts: those drawn within its clip.
                let ours: Vec<_> = texts
                    .iter()
                    .filter(|(_, r, clip)| {
                        clip.right() <= panel.right() + 1.0 && r.left() < panel.right()
                    })
                    .collect();
                assert!(ours.len() > 20, "{code}: the options drew ({})", ours.len());
                for (text, r, clip) in ours {
                    if text.trim().is_empty() || r.bottom() < clip.top() || r.top() > clip.bottom()
                    {
                        continue;
                    }
                    assert!(
                        r.right() <= clip.right() + 1.0 && r.left() >= clip.left() - 1.0,
                        "{code}: {text:?} cut off: {r:?} in {clip:?} (panel {panel:?})"
                    );
                }
                assert!(panel.width() <= size.x * 0.5 + 1.0, "{code}: {panel:?}");
            });
        }
    }
    /// Texts drawn by one frame of the preview at `size`, with where.
    fn frame_texts(
        app: &mut App,
        ctx: &egui::Context,
        size: egui::Vec2,
    ) -> Vec<(String, egui::Rect)> {
        fn walk(s: &egui::Shape, out: &mut Vec<(String, egui::Rect)>) {
            match s {
                egui::Shape::Text(t) => {
                    out.push((t.galley.text().to_string(), t.visual_bounding_rect()))
                }
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            },
            |ui| show(app, ui),
        );
        out.textures_delta.clear();
        let mut texts = Vec::new();
        out.shapes.iter().for_each(|c| walk(&c.shape, &mut texts));
        texts
    }

    /// The options at the left stand two to a row, as the report's sketch
    /// has them: All pages beside Current page, Pages with its list beside
    /// Selection, Fit to page beside Odd and even pages, then Bleed beside
    /// Crop marks, Registration marks beside Color bars, Densitometer
    /// scale beside Page information and Mirror beside Invert (#951).
    #[test]
    fn the_options_stand_two_to_a_row() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        app.print_preview(None);
        let mut texts = Vec::new();
        for _ in 0..4 {
            texts = frame_texts(&mut app, &ctx, egui::vec2(1400.0, 900.0));
        }
        let at = |key: &str| {
            let label = tr(key);
            texts
                .iter()
                .find(|(t, _)| *t == label)
                .map(|(_, r)| r.left_top())
                .unwrap_or_else(|| panic!("{key} shown"))
        };
        for (left, right) in [
            ("dialog.all_pages", "print.range_page"),
            ("print.range_pages", "print.range_selection"),
            ("dialog.fit_to_page", "print.odd_and_even"),
            ("dialog.print_bleed", "dialog.crop_marks"),
            ("dialog.registration_marks", "print.color_bars"),
            ("print.densitometer", "print.page_info_mark"),
            ("dialog.mirror", "dialog.invert"),
        ] {
            let (l, r) = (at(left), at(right));
            assert!((l.y - r.y).abs() < 4.0, "{left} {l:?} beside {right} {r:?}");
            assert!(
                r.x > l.x + 40.0 && r.x < 480.0,
                "{right} {r:?} right of {left}"
            );
        }
        // The rows follow each other down the panel.
        assert!(at("print.range_pages").y > at("dialog.all_pages").y);
        assert!(at("dialog.print_bleed").y > at("dialog.fit_to_page").y);
        assert!(at("dialog.mirror").y > at("print.densitometer").y);
    }

    /// The pages printed are chosen in the preview: the current page, a
    /// list, odd or even pages; the preview's sheets follow (#951).
    #[test]
    fn the_preview_prints_the_pages_chosen() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        let size = app.doc().pages[0].size;
        for _ in 0..4 {
            app.run(tracedraw_core::Command::AddPage {
                name: None,
                size,
                layer: None,
            });
        }
        let n = app.doc().pages.len();
        assert_eq!(n, 5);
        app.print_preview(None);
        let count = |app: &mut App| {
            let _ = frame_texts(app, &ctx, egui::vec2(1400.0, 900.0));
            let pv = app.print_preview.as_mut().expect("preview");
            let st = pv.st.clone();
            print_sheets(app, &st).pages.len()
        };
        assert_eq!(count(&mut app), 5);
        let set = |app: &mut App, f: &dyn Fn(&mut PrintState)| {
            if let Some(pv) = app.print_preview.as_mut() {
                f(&mut pv.st);
            }
        };
        set(&mut app, &|st| st.parity = 1);
        assert_eq!(count(&mut app), 3, "pages 1, 3 and 5");
        set(&mut app, &|st| st.parity = 2);
        assert_eq!(count(&mut app), 2, "pages 2 and 4");
        set(&mut app, &|st| {
            st.range = 2;
            st.pages = "2-5".into();
            st.parity = 1;
        });
        assert_eq!(count(&mut app), 2, "pages 3 and 5");
        set(&mut app, &|st| st.range = 1);
        assert_eq!(count(&mut app), 1, "the current page, whatever the parity");
        let st = &app.print_preview.as_ref().expect("preview").st;
        assert!(!st.all_pages, "the current page alone");
    }
}
