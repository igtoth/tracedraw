//! The colour palettes docked under the drawing window: the default
//! palette, then the document palette.
//!
//! Each row: a grip, the palette menu button, the eyedropper (it adds
//! colours sampled from the drawing; only the document palette can be
//! changed), the scroll arrow, then 20 px swatches in a one-pixel dark
//! frame, and at the right the other scroll arrow and the button that shows
//! every colour at once. A click sets the fill of the selection, a right
//! click its outline, Ctrl+click mixes a tenth of the colour into the fill,
//! a click and hold shows the colour's shades; the wheel scrolls.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::rulers::Pixels;
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui};
use tracedraw_core::{Color, Command, Fill};

/// Height of a palette row: a separator line, the swatches in their frame
/// and a pixel of space on each side.
pub const ROW_H: f32 = 25.0;
const SWATCH: f32 = 20.0;
const STRIDE: f32 = 21.0;
/// From a row's left edge to the frame of the first swatch.
const LEAD: f32 = 77.0;
/// Room for the scroll and expand buttons at the right.
const TAIL: f32 = 51.0;
const FRAME: Color32 = Color32::from_gray(90);
const LINE: Color32 = Color32::from_gray(0xD8);
const GRIP: Color32 = Color32::from_gray(178);
const MENU_ARROW: Color32 = Color32::from_gray(120);
const ARROW_ON: Color32 = Color32::from_gray(150);
const ARROW_OFF: Color32 = Color32::from_gray(214);
const NO_COLOR_LINE: Color32 = Color32::from_rgb(255, 64, 64);
/// Seconds a swatch is held before its shades pop up.
const HOLD: f64 = 0.5;

/// Which palette a row shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Main,
    Document,
}

/// The default palette row.
pub fn palette_row(app: &mut App, ui: &mut Ui) {
    row(app, ui, Row::Main);
}

/// The document palette row.
pub fn document_palette_row(app: &mut App, ui: &mut Ui) {
    row(app, ui, Row::Document);
}

/// The colours a row shows, with their names (the document palette borrows
/// names from the open palettes).
fn colors_of(app: &App, which: Row) -> Vec<(String, Color)> {
    match which {
        Row::Main => app.palette.clone(),
        Row::Document => app
            .doc()
            .palette
            .iter()
            .map(|c| (name_of(app, *c), *c))
            .collect(),
    }
}

fn name_of(app: &App, c: Color) -> String {
    app.palette
        .iter()
        .chain(app.palettes.iter().flat_map(|p| p.colors.iter()))
        .find(|(_, pc)| *pc == c)
        .map(|(n, _)| n.clone())
        .unwrap_or_default()
}

/// Tooltip of a swatch: the colour's name, then its values.
pub fn swatch_tip(name: &str, c: Color) -> String {
    let values = color_values(c);
    if name.is_empty() {
        values
    } else {
        format!("{name}\n{values}")
    }
}

/// Values as the palette tooltips show them.
pub fn color_values(c: Color) -> String {
    let pct = |v: f32| (v * 100.0).round() as i32;
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as i32;
    match c {
        Color::Cmyk { c, m, y, k } => {
            format!("C: {} M: {} Y: {} K: {}", pct(c), pct(m), pct(y), pct(k))
        }
        Color::Rgb { r, g, b } => format!("R: {} G: {} B: {}", byte(r), byte(g), byte(b)),
        Color::Gray { v } => crate::i18n::trf("palette.gray_value", &[("v", &byte(v).to_string())]),
        other => crate::app::color_description(other),
    }
}

fn scroll_of(app: &mut App, which: Row) -> &mut usize {
    match which {
        Row::Main => &mut app.palette_scroll,
        Row::Document => &mut app.doc_palette_scroll,
    }
}

/// How many swatches fit in a row of this width (the No Color well
/// counts as one).
pub fn visible_slots(width: f32) -> usize {
    ((width - LEAD - TAIL - 1.0) / STRIDE).floor().max(1.0) as usize
}

fn row(app: &mut App, ui: &mut Ui, which: Row) {
    let (rect, bg) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
    let painter = ui.painter_at(rect);
    let mut px = Pixels::default();
    let x0 = rect.left().round();
    let top = rect.top().round();
    px.rect(
        Rect::from_min_size(pos2(x0, top), vec2(rect.width(), 1.0)),
        LINE,
    );
    let frame_top = top + 2.0;
    // Grip: dots in a zigzag.
    for i in 0..5 {
        let y = frame_top + 1.0 + 4.0 * i as f32;
        px.dot(x0 + 3.0, y, GRIP);
        px.dot(x0 + 7.0, y, GRIP);
        px.dot(x0 + 5.0, y + 2.0, GRIP);
    }
    px.paint(&painter);

    let colors = colors_of(app, which);
    let no_color = app.settings.palette.show_no_color;
    let count = colors.len() + usize::from(no_color);
    let slots = visible_slots(rect.width());
    let max_scroll = count.saturating_sub(slots);
    let scroll = (*scroll_of(app, which)).min(max_scroll);
    *scroll_of(app, which) = scroll;

    // The palette menu button.
    let menu_rect = Rect::from_min_size(pos2(x0 + 13.0, frame_top), vec2(15.0, 22.0));
    let menu_resp = ui.interact(
        menu_rect,
        ui.id().with(("palette_menu", which as u8)),
        Sense::click(),
    );
    let c = menu_rect.center();
    painter.add(egui::epaint::PathShape::convex_polygon(
        vec![
            pos2(c.x - 2.0, c.y - 4.5),
            pos2(c.x + 3.0, c.y + 0.5),
            pos2(c.x - 2.0, c.y + 5.5),
        ],
        MENU_ARROW,
        Stroke::NONE,
    ));
    let menu_resp = menu_resp.on_hover_text(tr("palette.menu_tip"));
    egui::Popup::menu(&menu_resp)
        .id(egui::Id::new(("palette_menu_popup", which as u8)))
        .style(crate::ui::menus::menu_popup_style)
        .show(|ui| crate::ui::menus::body(ui, |ui| palette_menu(app, ui, which)));
    // A right click on the row outside the swatches opens the same menu.
    egui::Popup::context_menu(&bg)
        .id(egui::Id::new(("palette_context_popup", which as u8)))
        .style(crate::ui::menus::menu_popup_style)
        .show(|ui| crate::ui::menus::body(ui, |ui| palette_menu(app, ui, which)));

    // The eyedropper: only the document palette takes new colours.
    let drop_rect = Rect::from_min_size(pos2(x0 + 34.0, frame_top + 2.0), vec2(18.0, 18.0));
    let can_sample = which == Row::Document && app.has_document();
    let drop_resp = ui.interact(
        drop_rect,
        ui.id().with(("palette_dropper", which as u8)),
        if can_sample {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let active = which == Row::Document && app.pending_palette_sample;
    if active || (can_sample && drop_resp.hovered()) {
        painter.rect(
            drop_rect.expand(1.0),
            0.0,
            if active {
                Tokens::TOOL_ACTIVE
            } else {
                Tokens::TOOL_HOVER
            },
            Stroke::new(1.0, Color32::from_rgb(0x99, 0xD1, 0xFF)),
            egui::StrokeKind::Inside,
        );
    }
    crate::ui::icons::draw(
        &painter,
        drop_rect.shrink(1.0),
        crate::tools::Tool::ColorEyedropper,
        if can_sample {
            Tokens::TEXT
        } else {
            Color32::from_gray(186)
        },
    );
    if can_sample {
        let drop_resp = drop_resp.on_hover_text(tr("palette.eyedropper_tip"));
        if drop_resp.clicked() {
            app.pending_palette_sample = !app.pending_palette_sample;
            if app.pending_palette_sample {
                app.status = tr("status.palette_sample");
            }
        }
    }

    // Scroll arrows.
    let left = Rect::from_min_size(pos2(x0 + 58.0, frame_top), vec2(16.0, 22.0));
    if chevron(app, ui, left, -1, scroll > 0, which) {
        *scroll_of(app, which) = scroll.saturating_sub(1);
    }
    let right_edge = rect.right().round();
    let right = Rect::from_min_size(pos2(right_edge - 49.0, frame_top), vec2(16.0, 22.0));
    if chevron(app, ui, right, 1, scroll < max_scroll, which) {
        *scroll_of(app, which) = (scroll + 1).min(max_scroll);
    }
    let expand = Rect::from_min_size(pos2(right_edge - 29.0, frame_top), vec2(18.0, 22.0));
    let expand_resp = ui
        .interact(
            expand,
            ui.id().with(("palette_expand", which as u8)),
            Sense::click(),
        )
        .on_hover_text(tr("palette.expand_tip"));
    double_chevron(
        &painter,
        expand.center(),
        if count > 0 { ARROW_ON } else { ARROW_OFF },
    );
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new(("palette_expand_rect", which as u8)), expand));
    if expand_resp.clicked() && count > 0 {
        app.palette_expanded = if app.palette_expanded == Some(which) {
            None
        } else {
            Some(which)
        };
    }

    // The swatches.
    let sx = x0 + LEAD;
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new(("palette_strip", which as u8)), pos2(sx, top)));
    let shown = slots.min(count.saturating_sub(scroll));
    if shown == 0 {
        if which == Row::Document {
            painter.text(
                pos2(sx + 8.0, frame_top + 11.0),
                Align2::LEFT_CENTER,
                tr("palette.document_hint"),
                FontId::proportional(11.0),
                Tokens::TEXT_DIM,
            );
        }
        return;
    }
    let mut px = Pixels::default();
    px.rect(
        Rect::from_min_size(pos2(sx, frame_top), vec2(shown as f32 * STRIDE + 1.0, 22.0)),
        FRAME,
    );
    px.paint(&painter);
    let notch: f32 = if ui.rect_contains_pointer(rect) {
        ui.input(|i| {
            i.events
                .iter()
                .map(|e| match e {
                    egui::Event::MouseWheel { unit, delta, .. } => {
                        let d = delta.y + delta.x;
                        match unit {
                            egui::MouseWheelUnit::Line => d,
                            egui::MouseWheelUnit::Point => d / 50.0,
                            egui::MouseWheelUnit::Page => d * 10.0,
                        }
                    }
                    _ => 0.0,
                })
                .sum()
        })
    } else {
        0.0
    };
    if notch < -0.5 {
        *scroll_of(app, which) = (scroll + 1).min(max_scroll);
    } else if notch > 0.5 {
        *scroll_of(app, which) = scroll.saturating_sub(1);
    }
    for slot in 0..shown {
        let index = scroll + slot;
        let cell = Rect::from_min_size(
            pos2(sx + 1.0 + slot as f32 * STRIDE, frame_top + 1.0),
            vec2(SWATCH, SWATCH),
        );
        let color_index = if no_color {
            index.checked_sub(1)
        } else {
            Some(index)
        };
        let entry = color_index.and_then(|i| colors.get(i));
        swatch(app, ui, &painter, cell, entry, which, index, color_index);
    }
    if which == Row::Document && shown < slots && colors.is_empty() {
        painter.text(
            pos2(sx + shown as f32 * STRIDE + 9.0, frame_top + 11.0),
            Align2::LEFT_CENTER,
            tr("palette.document_hint"),
            FontId::proportional(11.0),
            Tokens::TEXT_DIM,
        );
    }
}

/// One swatch; `entry` is `None` for the No Color well.
#[allow(clippy::too_many_arguments)]
fn swatch(
    app: &mut App,
    ui: &mut Ui,
    painter: &egui::Painter,
    cell: Rect,
    entry: Option<&(String, Color)>,
    which: Row,
    index: usize,
    color_index: Option<usize>,
) {
    let id = ui.id().with(("swatch", which as u8, index));
    let resp = ui.interact(cell, id, Sense::click_and_drag());
    match entry {
        None => {
            painter.rect_filled(cell, 0.0, Color32::WHITE);
            painter.line_segment(
                [
                    cell.left_bottom() + vec2(2.5, -2.5),
                    cell.right_top() + vec2(-2.5, 2.5),
                ],
                Stroke::new(1.0, NO_COLOR_LINE),
            );
        }
        Some((_, c)) => {
            painter.rect_filled(cell, 0.0, crate::canvas::to_color32(*c));
        }
    }
    let current =
        which == Row::Document && color_index.is_some() && app.doc_palette_current == color_index;
    if resp.hovered() || resp.is_pointer_button_down_on() || current {
        // A white frame around the swatch under the pointer.
        let mut px = Pixels::default();
        let o = cell.expand(1.0);
        px.rect(
            Rect::from_min_size(o.min, vec2(o.width(), 1.0)),
            Color32::WHITE,
        );
        px.rect(
            Rect::from_min_size(pos2(o.left(), o.bottom() - 1.0), vec2(o.width(), 1.0)),
            Color32::WHITE,
        );
        px.rect(
            Rect::from_min_size(o.min, vec2(1.0, o.height())),
            Color32::WHITE,
        );
        px.rect(
            Rect::from_min_size(pos2(o.right() - 1.0, o.top()), vec2(1.0, o.height())),
            Color32::WHITE,
        );
        px.paint(painter);
    }
    let tip = match entry {
        None => tr("palette.no_color_tip"),
        Some((name, c)) => swatch_tip(name, *c),
    };
    let resp = resp.on_hover_text(tip);

    // Click and hold: the colour's shades.
    if let Some((_, c)) = entry {
        if resp.is_pointer_button_down_on() && app.palette_shades.is_none() {
            let held = ui.input(|i| {
                i.pointer
                    .press_start_time()
                    .map(|t| i.time - t)
                    .unwrap_or(0.0)
            });
            if held >= HOLD && !resp.dragged() {
                app.palette_shades = Some((*c, cell.center_top()));
                ui.ctx()
                    .data_mut(|d| d.insert_temp(shades_fired_id(), true));
            } else {
                ui.ctx().request_repaint();
            }
        }
    }
    let fired = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(shades_fired_id()))
        .unwrap_or(false);
    if fired && (resp.clicked() || resp.drag_stopped()) {
        // The release that ends a hold does not apply the colour.
        ui.ctx().data_mut(|d| d.remove::<bool>(shades_fired_id()));
        return;
    }

    if resp.clicked() {
        if which == Row::Document {
            app.doc_palette_current = color_index;
        }
        match entry {
            None => app.apply_fill(Fill::None),
            Some((_, c)) => {
                if ui.input(|i| i.modifiers.ctrl) {
                    app.mix_into_fill(*c);
                } else {
                    app.apply_fill(Fill::Solid(*c));
                }
            }
        }
    }
    if app.settings.palette.right_click_outline {
        if resp.secondary_clicked() {
            app.apply_outline_color(entry.map(|(_, c)| *c));
        }
    } else {
        egui::Popup::context_menu(&resp)
            .id(egui::Id::new(("palette_swatch_menu", which as u8, index)))
            .style(crate::ui::menus::menu_popup_style)
            .show(|ui| crate::ui::menus::body(ui, |ui| palette_menu(app, ui, which)));
    }
}

fn shades_fired_id() -> egui::Id {
    egui::Id::new("palette_shades_fired")
}

/// A scroll chevron; returns true when clicked while enabled. Holding it
/// keeps scrolling.
fn chevron(app: &App, ui: &mut Ui, r: Rect, dir: i32, enabled: bool, which: Row) -> bool {
    let id = ui.id().with(("palette_chevron", which as u8, dir));
    let resp = ui.interact(r, id, Sense::click());
    let color = if enabled { ARROW_ON } else { ARROW_OFF };
    let c = r.center();
    let d = dir as f32;
    let pts = [
        pos2(c.x - 2.5 * d, c.y - 5.0),
        pos2(c.x + 2.5 * d, c.y),
        pos2(c.x - 2.5 * d, c.y + 5.0),
    ];
    ui.painter().add(egui::epaint::PathShape::line(
        pts.to_vec(),
        Stroke::new(2.0, color),
    ));
    let _ = app;
    if !enabled {
        return false;
    }
    if resp.clicked() {
        return true;
    }
    // Repeat while held.
    if resp.is_pointer_button_down_on() {
        let held = ui.input(|i| {
            i.pointer
                .press_start_time()
                .map(|t| i.time - t)
                .unwrap_or(0.0)
        });
        ui.ctx().request_repaint();
        if held > 0.4 {
            let step = ui.input(|i| {
                (i.time * 12.0).floor() != ((i.time - i.unstable_dt as f64) * 12.0).floor()
            });
            return step;
        }
    }
    false
}

fn double_chevron(painter: &egui::Painter, c: Pos2, color: Color32) {
    for dx in [-2.5f32, 2.5] {
        let pts = vec![
            pos2(c.x + dx - 2.0, c.y - 3.5),
            pos2(c.x + dx + 1.5, c.y),
            pos2(c.x + dx - 2.0, c.y + 3.5),
        ];
        painter.add(egui::epaint::PathShape::line(pts, Stroke::new(1.3, color)));
    }
}

/// The palette menu (the arrow button, or a right click on the row).
fn palette_menu(app: &mut App, ui: &mut Ui, which: Row) {
    use crate::ui::menus::{check, item, sep, sub};
    ui.set_min_width(220.0);
    match which {
        Row::Main => {
            if item(ui, "menu.window.palette_open", "", true) {
                app.open_palette_file();
            }
            if item(ui, "menu.window.palette_editor", "", true) {
                app.dialog = crate::ui::dialogs::Dialog::PaletteEditor(Default::default());
            }
            if item(ui, "menu.window.palette_manager", "", true) {
                app.show_dockers = true;
                app.docker_tab = crate::app::DockerTab::Palettes;
            }
            sep(ui);
            if check(
                ui,
                "palette.show_document",
                "",
                app.settings.palette.show_document,
            ) {
                app.settings.palette.show_document = !app.settings.palette.show_document;
                app.settings.save();
            }
        }
        Row::Document => {
            let doc = app.has_document();
            if check(
                ui,
                "palette.auto_update",
                "",
                app.settings.palette.auto_update_document,
            ) {
                app.settings.palette.auto_update_document =
                    !app.settings.palette.auto_update_document;
                app.settings.save();
            }
            sep(ui);
            if item(
                ui,
                "palette.add_from_selection",
                "",
                doc && !app.selection.is_empty(),
            ) {
                app.palette_from_selection();
            }
            if item(ui, "palette.add_from_document", "", doc) {
                app.add_document_colors_to_palette();
            }
            let current = app
                .doc_palette_current
                .filter(|i| *i < app.doc().palette.len());
            if item(ui, "palette.delete_color", "", doc && current.is_some()) {
                if let Some(i) = current {
                    let mut colors = app.doc().palette.clone();
                    colors.remove(i);
                    app.run(Command::SetDocumentPalette { colors });
                    app.doc_palette_current = None;
                }
            }
            sep(ui);
            sub(ui, "palette.palette", |ui| {
                if item(ui, "palette.reset", "", doc) {
                    app.reset_document_palette();
                }
            });
        }
    }
}

/// Pop-ups drawn over the window: a colour's shades and the expanded
/// palette. Called once per frame after the rows.
pub fn popups(app: &mut App, ctx: &egui::Context) {
    if let Some((color, anchor)) = app.palette_shades {
        let id = egui::Id::new("palette_shades");
        let grid = shades(color);
        let side = 5.0 * STRIDE + 1.0;
        let pos = pos2(anchor.x - side / 2.0, anchor.y - side - 6.0);
        let mut picked = None;
        let area = egui::Area::new(id)
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                let (r, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
                let painter = ui.painter();
                painter.rect_filled(r, 0.0, FRAME);
                for (i, c) in grid.iter().enumerate() {
                    let (col, row) = ((i % 5) as f32, (i / 5) as f32);
                    let cell = Rect::from_min_size(
                        r.min + vec2(1.0 + col * STRIDE, 1.0 + row * STRIDE),
                        vec2(SWATCH, SWATCH),
                    );
                    painter.rect_filled(cell, 0.0, crate::canvas::to_color32(*c));
                    let resp = ui
                        .interact(cell, id.with(i), Sense::click())
                        .on_hover_text(color_values(*c));
                    if resp.hovered() {
                        painter.rect_stroke(
                            cell.expand(1.0),
                            0.0,
                            Stroke::new(1.0, Color32::WHITE),
                            egui::StrokeKind::Inside,
                        );
                    }
                    if resp.clicked() {
                        picked = Some((*c, false));
                    }
                    if resp.secondary_clicked() {
                        picked = Some((*c, true));
                    }
                }
            });
        if let Some((c, outline)) = picked {
            app.palette_shades = None;
            if outline {
                app.apply_outline_color(Some(c));
            } else {
                app.apply_fill(Fill::Solid(c));
            }
        } else if clicked_outside(ctx, area.response.rect) {
            app.palette_shades = None;
        }
    }
    if let Some(which) = app.palette_expanded {
        let colors = colors_of(app, which);
        let id = egui::Id::new("palette_expanded");
        let screen = ctx.content_rect();
        // Above the row, from its first swatch.
        let strip = ctx
            .data(|d| d.get_temp::<Pos2>(egui::Id::new(("palette_strip", which as u8))))
            .unwrap_or(pos2(screen.left() + LEAD, screen.bottom() - 3.0 * ROW_H));
        let cols = (((screen.right() - strip.x - 24.0) / STRIDE).floor() as usize).clamp(1, 33);
        let rows = colors.len().div_ceil(cols).max(1);
        let size = vec2(cols as f32 * STRIDE + 1.0, rows as f32 * STRIDE + 1.0);
        let pos = pos2(strip.x - 7.0, (strip.y - size.y - 16.0).max(screen.top()));
        let mut picked = None;
        let area = egui::Area::new(id)
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    let (r, _) = ui.allocate_exact_size(size, Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(r, 0.0, FRAME);
                    for (i, (name, c)) in colors.iter().enumerate() {
                        let (col, row) = ((i % cols) as f32, (i / cols) as f32);
                        let cell = Rect::from_min_size(
                            r.min + vec2(1.0 + col * STRIDE, 1.0 + row * STRIDE),
                            vec2(SWATCH, SWATCH),
                        );
                        painter.rect_filled(cell, 0.0, crate::canvas::to_color32(*c));
                        let resp = ui
                            .interact(cell, id.with(i), Sense::click())
                            .on_hover_text(swatch_tip(name, *c));
                        if resp.hovered() {
                            painter.rect_stroke(
                                cell.expand(1.0),
                                0.0,
                                Stroke::new(1.0, Color32::WHITE),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if resp.clicked() {
                            picked = Some((*c, false));
                        }
                        if resp.secondary_clicked() {
                            picked = Some((*c, true));
                        }
                    }
                });
            });
        if let Some((c, outline)) = picked {
            app.palette_expanded = None;
            if outline {
                app.apply_outline_color(Some(c));
            } else {
                app.apply_fill(Fill::Solid(c));
            }
        } else if clicked_outside(ctx, area.response.rect) && !on_expand_button(ctx, which) {
            app.palette_expanded = None;
        }
    }
    // The release that ends a click and hold is over.
    if ctx.input(|i| i.pointer.any_released()) {
        ctx.data_mut(|d| d.remove::<bool>(shades_fired_id()));
    }
}

/// The expand button toggles the pop-up itself.
fn on_expand_button(ctx: &egui::Context, which: Row) -> bool {
    let r = ctx.data(|d| d.get_temp::<Rect>(egui::Id::new(("palette_expand_rect", which as u8))));
    let p = ctx.input(|i| i.pointer.interact_pos());
    matches!((r, p), (Some(r), Some(p)) if r.contains(p))
}

fn clicked_outside(ctx: &egui::Context, r: Rect) -> bool {
    ctx.input(|i| {
        i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !r.contains(p))
    })
}

/// A 5 x 5 grid around a colour: lighter tints upwards, more black
/// downwards, less colourful to the left, more to the right; the colour
/// itself in the middle.
pub fn shades(c: Color) -> Vec<Color> {
    let mut out = Vec::with_capacity(25);
    for row in -2i32..=2 {
        for col in -2i32..=2 {
            out.push(shade(c, row, col));
        }
    }
    out
}

fn shade(c: Color, row: i32, col: i32) -> Color {
    match c {
        Color::Cmyk { c, m, y, k } => {
            let mut v = [c, m, y];
            // Chroma: spread the inks away from (or towards) their mean.
            let mean = (v[0] + v[1] + v[2]) / 3.0;
            let s = 1.0 + 0.25 * col as f32;
            for x in &mut v {
                *x = mean + (*x - mean) * s;
            }
            let mut k = k;
            if row < 0 {
                // Tints: less of every ink.
                let f = 1.0 + 0.3 * row as f32;
                for x in &mut v {
                    *x *= f;
                }
                k *= f;
            } else {
                k += 0.2 * row as f32;
            }
            Color::Cmyk {
                c: round_pct(v[0]),
                m: round_pct(v[1]),
                y: round_pct(v[2]),
                k: round_pct(k),
            }
        }
        other => {
            let (h, s, b) = other.to_hsb();
            let s = (s * (1.0 + 0.25 * col as f64)).clamp(0.0, 1.0);
            let b = if row < 0 {
                b + (1.0 - b) * 0.35 * (-row) as f64
            } else {
                b * (1.0 - 0.25 * row as f64)
            };
            let tint = if row < 0 {
                s * (1.0 + 0.3 * row as f64)
            } else {
                s
            };
            Color::from_hsb(h, tint.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
        }
    }
}

fn round_pct(v: f32) -> f32 {
    (v.clamp(0.0, 1.0) * 100.0).round() / 100.0
}

/// Mix `share` of colour `b` into colour `a`, in `a`'s model when both are
/// CMYK or both RGB, else by way of RGB.
pub fn mix(a: Color, b: Color, share: f32) -> Color {
    let l = |x: f32, y: f32| x + (y - x) * share;
    match (a, b) {
        (
            Color::Cmyk { c, m, y, k },
            Color::Cmyk {
                c: c2,
                m: m2,
                y: y2,
                k: k2,
            },
        ) => Color::Cmyk {
            c: l(c, c2),
            m: l(m, m2),
            y: l(y, y2),
            k: l(k, k2),
        },
        _ => {
            let [r, g, bb] = a.to_rgb_f32();
            let [r2, g2, b2] = b.to_rgb_f32();
            Color::Rgb {
                r: l(r, r2),
                g: l(g, g2),
                b: l(bb, b2),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltips_name_the_colour_and_its_values() {
        let c = Color::cmyk_pct(60.0, 0.0, 40.0, 20.0);
        assert_eq!(
            swatch_tip("Light Green", c),
            "Light Green\nC: 60 M: 0 Y: 40 K: 20"
        );
        assert_eq!(swatch_tip("", Color::rgb8(255, 0, 10)), "R: 255 G: 0 B: 10");
    }

    #[test]
    fn the_default_palette_has_greys_process_colours_and_tints() {
        let app = App::headless();
        assert_eq!(app.palette.len(), 99);
        let names: Vec<&str> = app.palette.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(&names[..3], &["Black", "90% Black", "80% Black"]);
        assert_eq!(names[10], "White");
        assert_eq!(
            &names[11..17],
            &["Blue", "Cyan", "Green", "Yellow", "Red", "Magenta"]
        );
        let (_, mint) = app
            .palette
            .iter()
            .find(|(n, _)| n == "Mint Green")
            .expect("mint");
        assert_eq!(*mint, Color::cmyk_pct(40.0, 0.0, 40.0, 0.0));
    }

    #[test]
    fn shades_keep_the_colour_in_the_middle() {
        let c = Color::cmyk_pct(0.0, 60.0, 100.0, 0.0);
        let g = shades(c);
        assert_eq!(g.len(), 25);
        assert_eq!(g[12], c);
        // Lighter above, darker below.
        assert!(g[2].luminance() > c.luminance());
        assert!(g[22].luminance() < c.luminance());
    }

    #[test]
    fn mixing_moves_a_tenth_of_the_way() {
        let a = Color::cmyk_pct(0.0, 0.0, 0.0, 0.0);
        let b = Color::cmyk_pct(100.0, 0.0, 0.0, 50.0);
        assert_eq!(mix(a, b, 0.1), Color::cmyk_pct(10.0, 0.0, 0.0, 5.0));
    }

    #[test]
    fn applying_a_colour_adds_it_to_the_document_palette_in_the_same_step() {
        use tracedraw_core::{geometry::Rect as R, ShapeKind};
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: R::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        app.select(vec![id]);
        let steps = app.engine.history_labels().0.len();
        let green = Color::cmyk_pct(100.0, 0.0, 100.0, 0.0);
        app.apply_fill(Fill::Solid(green));
        assert_eq!(app.doc().palette, vec![green]);
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        // Applying it again does not add it twice; the outline adds black.
        app.apply_fill(Fill::Solid(green));
        let black = Color::cmyk_pct(0.0, 0.0, 0.0, 100.0);
        app.apply_outline_color(Some(black));
        assert_eq!(app.doc().palette, vec![green, black]);
        app.undo();
        assert_eq!(app.doc().palette, vec![green]);
        // Off in the options: nothing is added.
        app.settings.palette.auto_update_document = false;
        app.apply_fill(Fill::Solid(Color::cmyk_pct(0.0, 100.0, 0.0, 0.0)));
        assert_eq!(app.doc().palette, vec![green]);
    }

    #[test]
    fn rows_fit_whole_swatches() {
        assert_eq!(visible_slots(LEAD + TAIL + 1.0 + 21.0 * 10.0), 10);
        assert_eq!(visible_slots(10.0), 1);
    }
}
