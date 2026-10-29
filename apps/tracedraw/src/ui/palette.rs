//! The colour palettes docked under the drawing window: the default
//! palette, a strip for each further palette shown (#1164), then the
//! document palette.
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
use crate::ui::tips::RichTip;
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
const NO_COLOR_LINE: Color32 = Color32::from_rgb(255, 64, 64);
/// Seconds a swatch is held before its shades pop up.
const HOLD: f64 = 0.5;
/// Seconds the right button is held on a swatch to open its menu when a
/// right click sets the outline.
const RIGHT_HOLD: f64 = 1.0;

/// Which palette a row shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Row {
    Main,
    Document,
    /// The `k`th palette shown after the first ([`App::extra_palettes`]).
    Extra(usize),
}

impl Row {
    /// A small number telling the rows apart in widget ids.
    fn key(self) -> u8 {
        match self {
            Row::Main => 0,
            Row::Document => 1,
            Row::Extra(k) => u8::try_from(k.saturating_add(2)).unwrap_or(u8::MAX),
        }
    }
}

/// A further palette shown in a strip of its own: which palette, its
/// colours (and as an RGB drawing shows them, worked out once when the
/// palettes change), and how far the strip is scrolled.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtraRow {
    pub index: usize,
    pub colors: Vec<(String, Color)>,
    pub rgb: Vec<(String, Color)>,
    pub scroll: usize,
}

/// The strips of the further palettes shown, in order.
pub fn extra_rows(app: &App) -> Vec<Row> {
    (0..app.extra_palettes.len()).map(Row::Extra).collect()
}

/// A further palette as a row.
pub fn extra_palette_row(app: &mut App, ui: &mut Ui, which: Row) {
    row(app, ui, which, false);
}

/// A further palette as a column.
pub fn extra_palette_column(app: &mut App, ui: &mut Ui, which: Row) {
    row(app, ui, which, true);
}

/// The default palette row.
pub fn palette_row(app: &mut App, ui: &mut Ui) {
    row(app, ui, Row::Main, false);
}

/// The document palette row.
pub fn document_palette_row(app: &mut App, ui: &mut Ui) {
    row(app, ui, Row::Document, false);
}

/// The default palette as a column (docked on the left or the right).
pub fn palette_column(app: &mut App, ui: &mut Ui) {
    row(app, ui, Row::Main, true);
}

/// A strip's geometry: the row's design (left to right, the swatches
/// along it, 25 px across) laid along x for a row or along y for a
/// column.
#[derive(Clone, Copy)]
struct Axis {
    vertical: bool,
    origin: Pos2,
}

impl Axis {
    /// The rectangle `len` long and `thick` across, starting `along` the
    /// strip and `across` from its edge.
    fn rect(&self, along: f32, across: f32, len: f32, thick: f32) -> Rect {
        if self.vertical {
            Rect::from_min_size(
                pos2(self.origin.x + across, self.origin.y + along),
                vec2(thick, len),
            )
        } else {
            Rect::from_min_size(
                pos2(self.origin.x + along, self.origin.y + across),
                vec2(len, thick),
            )
        }
    }

    fn point(&self, along: f32, across: f32) -> Pos2 {
        self.rect(along, across, 0.0, 0.0).min
    }

    /// A vector `along` the strip and `across` it.
    fn vec(&self, along: f32, across: f32) -> egui::Vec2 {
        if self.vertical {
            vec2(across, along)
        } else {
            vec2(along, across)
        }
    }
}

fn colors_of(app: &App, which: Row) -> Vec<(String, Color)> {
    match which {
        // An RGB drawing gets the palette's colours as RGB, so what it
        // applies is RGB too; spot inks and registration stay (#483).
        // Worked out when the palette changes, not every frame (#620).
        Row::Main if rgb_drawing(app) => app.palette_rgb.clone(),
        Row::Main => app.palette.clone(),
        Row::Document => app
            .doc()
            .palette
            .iter()
            .map(|c| (name_of(app, *c), *c))
            .collect(),
        Row::Extra(k) => match app.extra_palettes.get(k) {
            Some(r) if rgb_drawing(app) => r.rgb.clone(),
            Some(r) => r.colors.clone(),
            None => Vec::new(),
        },
    }
}

/// `palette` as an RGB drawing shows it: the default palette's colours
/// take the RGB default palette's values, defined directly, found by name
/// and colour so a palette edited in the Palette Editor keeps them (#620);
/// other colours their inks as RGB ([`in_rgb`]).
pub fn as_rgb(palette: &[(String, Color)]) -> Vec<(String, Color)> {
    palette
        .iter()
        .map(|(n, c)| match DEFAULTS.get(n.as_str()) {
            Some((cmyk, rgb)) if cmyk == c => (n.clone(), *rgb),
            _ => (n.clone(), in_rgb(*c)),
        })
        .collect()
}

/// The default palette's colours by name: their CMYK value and the RGB
/// one they show as in an RGB drawing.
static DEFAULTS: std::sync::LazyLock<std::collections::HashMap<String, (Color, Color)>> =
    std::sync::LazyLock::new(|| {
        crate::palette::default_cmyk()
            .into_iter()
            .zip(crate::palette::default_rgb())
            .map(|((n, cmyk), (_, rgb))| (n, (cmyk, rgb)))
            .collect()
    });

/// Whether the drawing's primary colour mode is RGB.
fn rgb_drawing(app: &App) -> bool {
    app.has_document()
        && app.doc().metadata.primary_color_mode == tracedraw_core::document::PrimaryColorMode::Rgb
}

/// `c` as an RGB colour, a spot ink or registration as it is. A CMYK
/// colour takes the plain RGB of its inks, not the press simulation: each
/// channel is 255 (1 - min(1, ink + K)), so the palette's black is 0 0 0,
/// its red 255 0 0, its tints the matching RGB tints and C20 M80 K20
/// 153 0 204 (#620).
fn in_rgb(c: Color) -> Color {
    match c {
        Color::Spot { .. } | Color::Registration => c,
        Color::Cmyk { c, m, y, k } => {
            let ink = |v: f32| 1.0 - (v.clamp(0.0, 1.0) + k.clamp(0.0, 1.0)).min(1.0);
            let byte = |v: f32| (ink(v) * 255.0).round() as u8;
            Color::rgb8(byte(c), byte(m), byte(y))
        }
        _ => c.convert_to("RGB"),
    }
}

/// The name of `c` in the default palette or the palettes shown; empty
/// when it has none.
pub fn name_of(app: &App, c: Color) -> String {
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

/// A swatch's tooltip: the colour's name in bold, then its values one
/// per line (#1082).
pub fn swatch_tip_ui(ui: &mut Ui, name: &str, c: Color) {
    ui.spacing_mut().item_spacing.y = 1.0;
    let size = ui.style().text_styles[&egui::TextStyle::Body].size;
    if !name.is_empty() {
        ui.label(egui::RichText::new(name).font(crate::theme::bold(size)));
    }
    ui.label(color_lines(c));
}

/// A colour's values one per line ("C: 0", "M: 60", ...).
pub fn color_lines(c: Color) -> String {
    match c {
        Color::Cmyk { .. } | Color::Rgb { .. } => color_values(c)
            .split(' ')
            .collect::<Vec<_>>()
            .chunks(2)
            .map(|p| p.join(" "))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => color_values(c),
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
        Row::Extra(k) if k < app.extra_palettes.len() => &mut app.extra_palettes[k].scroll,
        // A strip gone since: nothing scrolls it.
        Row::Extra(_) => &mut app.palette_scroll_spare,
    }
}

/// How thick the strip of `which` is: one row of swatches, and for the
/// default palette one more swatch for each further row (#610).
pub fn thickness(app: &App, which: Row) -> f32 {
    ROW_H + (lines_of(app, which) - 1) as f32 * STRIDE
}

/// Rows of swatches in the strip of `which` (the document palette and
/// the further palettes have one).
fn lines_of(app: &App, which: Row) -> usize {
    match which {
        Row::Main => app.settings.palette.rows(),
        Row::Document | Row::Extra(_) => 1,
    }
}

/// How many swatches fit in a row of this width (the No Color well
/// counts as one).
pub fn visible_slots(width: f32) -> usize {
    ((width - LEAD - TAIL - 1.0) / STRIDE).floor().max(1.0) as usize
}

fn row(app: &mut App, ui: &mut Ui, which: Row, vertical: bool) {
    let lines = lines_of(app, which);
    let thick = thickness(app, which);
    let size = if vertical {
        vec2(thick, ui.available_height())
    } else {
        vec2(ui.available_width(), thick)
    };
    let (rect, bg) = ui.allocate_exact_size(size, Sense::click());
    if which == Row::Document {
        app.doc_palette_rect = Some(rect);
    }
    let painter = ui.painter_at(rect);
    let ax = Axis {
        vertical,
        origin: pos2(rect.left().round(), rect.top().round()),
    };
    let len = if vertical {
        rect.height()
    } else {
        rect.width()
    }
    .round();
    let mut px = Pixels::default();
    px.rect(ax.rect(0.0, 0.0, len, 1.0), crate::theme::Tokens::border());
    // Swatch frames start 2 px from the strip's edge.
    const EDGE: f32 = 2.0;
    // Grip: dots in a zigzag; dragging it moves the palette. Locked bars
    // show no grip, and a floating palette moves by its title.
    let floating =
        app.settings.bars.dock(crate::ui::bars::Bar::Palette) == crate::ui::bars::Dock::Floating;
    if !app.settings.bars.locked && !floating {
        for i in 0..5 {
            let c = EDGE + 1.0 + 4.0 * i as f32;
            let a = ax.point(3.0, c);
            px.dot(a.x, a.y, crate::theme::Tokens::text_dim());
            let b = ax.point(7.0, c);
            px.dot(b.x, b.y, crate::theme::Tokens::text_dim());
            let m = ax.point(5.0, c + 2.0);
            px.dot(m.x, m.y, crate::theme::Tokens::text_dim());
        }
    }
    px.paint(&painter);
    crate::ui::bars::grip_zone(
        app,
        ui,
        crate::ui::bars::Bar::Palette,
        ax.rect(0.0, 0.0, 11.0, thick),
        which.key(),
    );

    let colors = colors_of(app, which);
    let no_color = app.settings.palette.show_no_color;
    let count = colors.len() + usize::from(no_color);
    // Several rows fill line by line: each line holds the next run of the
    // palette, and the strip scrolls a swatch at a time (#610).
    let cols = visible_slots(len);
    let slots = cols * lines;
    let max_scroll = count.saturating_sub(slots);
    let scroll = (*scroll_of(app, which)).min(max_scroll);
    *scroll_of(app, which) = scroll;

    // The palette menu button.
    let menu_rect = ax.rect(13.0, EDGE, 15.0, 22.0);
    let menu_resp = ui.interact(
        menu_rect,
        ui.id().with(("palette_menu", which.key())),
        Sense::click(),
    );
    let c = menu_rect.center();
    painter.add(egui::epaint::PathShape::convex_polygon(
        vec![
            c + ax.vec(-2.0, -4.5),
            c + ax.vec(3.0, 0.5),
            c + ax.vec(-2.0, 5.5),
        ],
        crate::theme::Tokens::text_dim(),
        Stroke::NONE,
    ));
    let menu_resp = menu_resp.rich_tip(tr("palette.menu_tip"));
    egui::Popup::menu(&menu_resp)
        .id(egui::Id::new(("palette_menu_popup", which.key())))
        .style(crate::ui::menus::menu_popup_style)
        .show(|ui| crate::ui::menus::body(ui, |ui| palette_menu(app, ui, which)));
    // A right click on the row outside the swatches opens the same menu.
    egui::Popup::context_menu(&bg)
        .id(egui::Id::new(("palette_context_popup", which.key())))
        .style(crate::ui::menus::menu_popup_style)
        .show(|ui| crate::ui::menus::body(ui, |ui| palette_menu(app, ui, which)));

    // The eyedropper: only the document palette takes new colours.
    let drop_rect = ax.rect(34.0, EDGE + 2.0, 18.0, 18.0);
    let can_sample = which == Row::Document && app.drawing_shown();
    let drop_resp = ui.interact(
        drop_rect,
        ui.id().with(("palette_dropper", which.key())),
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
                Tokens::tool_active()
            } else {
                Tokens::tool_hover()
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
            Tokens::text()
        } else {
            crate::theme::Tokens::icon_off()
        },
    );
    if can_sample {
        let drop_resp = drop_resp.rich_tip(tr("palette.eyedropper_tip"));
        if drop_resp.clicked() {
            app.pending_palette_sample = !app.pending_palette_sample;
            if app.pending_palette_sample {
                app.status = tr("status.palette_sample");
            }
        }
    }

    // Scroll arrows.
    let left = ax.rect(58.0, EDGE, 16.0, 22.0);
    if chevron(app, ui, left, -1, scroll > 0, which, vertical) {
        *scroll_of(app, which) = scroll.saturating_sub(1);
    }
    let right = ax.rect(len - 49.0, EDGE, 16.0, 22.0);
    if chevron(app, ui, right, 1, scroll < max_scroll, which, vertical) {
        *scroll_of(app, which) = (scroll + 1).min(max_scroll);
    }
    let expand = ax.rect(len - 29.0, EDGE, 18.0, 22.0);
    let expand_resp = ui
        .interact(
            expand,
            ui.id().with(("palette_expand", which.key())),
            Sense::click(),
        )
        .rich_tip(tr("palette.expand_tip"));
    double_chevron(
        &painter,
        expand.center(),
        if count > 0 {
            crate::theme::Tokens::text_dim()
        } else {
            crate::theme::Tokens::icon_off()
        },
        vertical,
    );
    ui.ctx()
        .data_mut(|d| d.insert_temp(egui::Id::new(("palette_expand_rect", which.key())), expand));
    if expand_resp.clicked() && count > 0 {
        app.palette_expanded = if app.palette_expanded == Some(which) {
            None
        } else {
            Some(which)
        };
    }

    // The swatches.
    let strip = ax.point(LEAD, 0.0);
    ui.ctx().data_mut(|d| {
        d.insert_temp(egui::Id::new(("palette_strip", which.key())), strip);
        d.insert_temp(
            egui::Id::new(("palette_strip_vertical", which.key())),
            vertical,
        );
    });
    let shown = slots.min(count.saturating_sub(scroll));
    if shown == 0 {
        if which == Row::Document && !vertical {
            painter.text(
                ax.point(LEAD + 8.0, EDGE + 11.0),
                Align2::LEFT_CENTER,
                tr("palette.document_hint"),
                FontId::proportional(11.0),
                Tokens::text_dim(),
            );
        }
        return;
    }
    let mut px = Pixels::default();
    // One frame per line, as long as the swatches it holds.
    for line in 0..shown.div_ceil(cols) {
        let n = cols.min(shown - line * cols);
        px.rect(
            ax.rect(
                LEAD,
                EDGE + line as f32 * STRIDE,
                n as f32 * STRIDE + 1.0,
                STRIDE + 1.0,
            ),
            FRAME,
        );
    }
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
        let cell = ax.rect(
            LEAD + 1.0 + (slot % cols) as f32 * STRIDE,
            EDGE + 1.0 + (slot / cols) as f32 * STRIDE,
            SWATCH,
            SWATCH,
        );
        let color_index = if no_color {
            index.checked_sub(1)
        } else {
            Some(index)
        };
        let entry = color_index.and_then(|i| colors.get(i));
        swatch(app, ui, &painter, cell, entry, which, index, color_index);
    }
    if which == Row::Document && shown < slots && colors.is_empty() && !vertical {
        painter.text(
            ax.point(LEAD + shown as f32 * STRIDE + 9.0, EDGE + 11.0),
            Align2::LEFT_CENTER,
            tr("palette.document_hint"),
            FontId::proportional(11.0),
            Tokens::text_dim(),
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
    let id = ui.id().with(("swatch", which.key(), index));
    // Nothing to colour on the Welcome Screen or with no drawing open:
    // the swatches keep their colours and their tooltips but take no
    // clicks or drags (#482, #1082).
    let sense = if app.drawing_shown() {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let resp = ui.interact(cell, id, sense);
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
    let resp = match entry {
        None => resp.rich_tip(tr("palette.no_color_tip")),
        Some((name, c)) => resp.on_hover_ui(|ui| swatch_tip_ui(ui, name, *c)),
    };

    // Dragging the colour: inside an object it fills it, on its outline
    // it colours the outline, on the document palette it joins it.
    if resp.drag_started() && app.palette_shades.is_none() {
        app.palette_drag = Some(entry.map(|(_, c)| *c));
    }
    if app.palette_drag.is_some() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.palette_drag = None;
    }
    if resp.dragged() {
        if let (Some(color), Some(pos)) = (app.palette_drag, ui.ctx().pointer_latest_pos()) {
            drag_chip(app, ui.ctx(), color, pos);
        }
    }
    if resp.drag_stopped() {
        if let (Some(color), Some(pos)) = (app.palette_drag.take(), ui.ctx().pointer_latest_pos()) {
            app.drop_color(color, pos);
        }
    }

    // Click and hold: the colour's shades.
    if let Some((_, c)) = entry {
        if resp.is_pointer_button_down_on()
            && ui.input(|i| i.pointer.primary_down())
            && app.palette_shades.is_none()
        {
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
                    app.palette_fill(*c);
                }
            }
        }
    }
    let menu_id = egui::Id::new(("palette_swatch_menu", which.key(), index));
    if app.settings.palette.right_click_outline {
        // A right click sets the outline; held for a second it opens the
        // palette menu instead (the release then does nothing).
        if resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.secondary_down()) {
            let held = ui.input(|i| {
                i.pointer
                    .press_start_time()
                    .map(|t| i.time - t)
                    .unwrap_or(0.0)
            });
            let open = egui::Popup::is_id_open(ui.ctx(), menu_id);
            if held >= RIGHT_HOLD && !open {
                ui.ctx().data_mut(|d| d.insert_temp(menu_id, true));
            } else if !open {
                ui.ctx().request_repaint();
            }
        }
        let hold_open = ui
            .ctx()
            .data_mut(|d| d.remove_temp::<bool>(menu_id))
            .unwrap_or(false);
        // Not a right click that makes a copy during a drag on the canvas.
        if resp.secondary_clicked() && !hold_open && !app.transforming() {
            app.apply_outline_color(entry.map(|(_, c)| *c));
        }
        egui::Popup::menu(&resp)
            .id(menu_id)
            .open_memory(hold_open.then_some(egui::SetOpenCommand::Bool(true)))
            .at_pointer_fixed()
            .style(crate::ui::menus::menu_popup_style)
            .show(|ui| crate::ui::menus::body(ui, |ui| palette_menu(app, ui, which)));
    } else {
        egui::Popup::context_menu(&resp)
            .id(menu_id)
            .style(crate::ui::menus::menu_popup_style)
            .show(|ui| crate::ui::menus::body(ui, |ui| palette_menu(app, ui, which)));
    }
}

/// The colour under the pointer while it is dragged: a filled chip over
/// a fill, a ring over an outline, and a pointer saying whether letting
/// go changes anything.
fn drag_chip(app: &App, ctx: &egui::Context, color: Option<Color>, pos: Pos2) {
    use crate::color_drop::ColorTarget;
    let target = app.color_target(pos);
    ctx.set_cursor_icon(if target == ColorTarget::Nothing {
        egui::CursorIcon::NoDrop
    } else {
        egui::CursorIcon::Copy
    });
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("palette_drag_chip"),
    ));
    let r = Rect::from_min_size(pos + vec2(12.0, 12.0), vec2(16.0, 16.0));
    let fill = color.map(crate::canvas::to_color32);
    painter.rect_filled(r.expand(1.0), 0.0, Color32::WHITE);
    match (target, fill) {
        (ColorTarget::Outline(_), Some(c)) => {
            painter.rect_stroke(
                r.shrink(2.0),
                0.0,
                Stroke::new(4.0, c),
                egui::StrokeKind::Inside,
            );
        }
        (_, Some(c)) => {
            painter.rect_filled(r, 0.0, c);
        }
        (_, None) => {
            painter.line_segment(
                [r.left_bottom(), r.right_top()],
                Stroke::new(1.5, NO_COLOR_LINE),
            );
        }
    }
    painter.rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Color32::from_gray(0x5A)),
        egui::StrokeKind::Outside,
    );
}

fn shades_fired_id() -> egui::Id {
    egui::Id::new("palette_shades_fired")
}

/// A scroll chevron; returns true when clicked while enabled. Holding it
/// keeps scrolling.
#[allow(clippy::too_many_arguments)]
fn chevron(
    app: &App,
    ui: &mut Ui,
    r: Rect,
    dir: i32,
    enabled: bool,
    which: Row,
    vertical: bool,
) -> bool {
    let id = ui.id().with(("palette_chevron", which.key(), dir));
    let key = if dir < 0 {
        "palette.scroll_back"
    } else {
        "palette.scroll_forward"
    };
    let resp = ui
        .interact(r, id, Sense::click())
        .rich_tip(crate::ui::tips::keyed(key));
    let color = if enabled {
        crate::theme::Tokens::text_dim()
    } else {
        crate::theme::Tokens::icon_off()
    };
    let c = r.center();
    let d = dir as f32;
    let at = |along: f32, across: f32| {
        if vertical {
            pos2(c.x + across, c.y + along)
        } else {
            pos2(c.x + along, c.y + across)
        }
    };
    let pts = [at(-2.5 * d, -5.0), at(2.5 * d, 0.0), at(-2.5 * d, 5.0)];
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

fn double_chevron(painter: &egui::Painter, c: Pos2, color: Color32, vertical: bool) {
    let at = |along: f32, across: f32| {
        if vertical {
            pos2(c.x + across, c.y + along)
        } else {
            pos2(c.x + along, c.y + across)
        }
    };
    for d in [-2.5f32, 2.5] {
        let pts = vec![at(d - 2.0, -3.5), at(d + 1.5, 0.0), at(d - 2.0, 3.5)];
        painter.add(egui::epaint::PathShape::line(pts, Stroke::new(1.3, color)));
    }
}

/// The palette menu (the arrow button, or a right click on the row).
fn palette_menu(app: &mut App, ui: &mut Ui, which: Row) {
    use crate::ui::menus::{check, item, radio, sep, sub};
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
            // Rows of swatches, as in Options > Customization > Color
            // Palette (#610).
            sub(ui, "palette.rows", |ui| {
                let rows = app.settings.palette.rows();
                for (n, key) in [
                    (1, "palette.rows_1"),
                    (2, "palette.rows_2"),
                    (3, "palette.rows_3"),
                ] {
                    if radio(ui, key, "", rows == n) {
                        app.settings.palette.rows = n as u8;
                        app.settings.save();
                    }
                }
            });
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
        Row::Extra(k) => {
            let index = app.extra_palettes.get(k).map(|r| r.index);
            if item(ui, "menu.window.palette_editor", "", true) {
                app.dialog = crate::ui::dialogs::Dialog::PaletteEditor(Default::default());
            }
            if item(ui, "menu.window.palette_manager", "", true) {
                app.show_dockers = true;
                app.docker_tab = crate::app::DockerTab::Palettes;
            }
            sep(ui);
            if item(ui, "palette.close", "", index.is_some()) {
                if let Some(i) = index {
                    app.toggle_palette(i);
                }
            }
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
        let screen = ctx.content_rect();
        // Above the swatch, kept on the screen (a palette docked at the
        // top or on a side has no room above some swatches).
        let mut pos = pos2(anchor.x - side / 2.0, anchor.y - side - 6.0);
        if pos.y < screen.top() {
            pos.y = anchor.y + SWATCH + 6.0;
        }
        pos.x = pos.x.clamp(
            screen.left(),
            (screen.right() - side - 4.0).max(screen.left()),
        );
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
                        .rich_tip(color_values(*c));
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
                app.palette_fill(c);
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
            .data(|d| d.get_temp::<Pos2>(egui::Id::new(("palette_strip", which.key()))))
            .unwrap_or(pos2(screen.left() + LEAD, screen.bottom() - 3.0 * ROW_H));
        // Clear of every row of the strip.
        let thick = thickness(app, which);
        let vertical = ctx
            .data(|d| d.get_temp::<bool>(egui::Id::new(("palette_strip_vertical", which.key()))))
            .unwrap_or(false);
        let (cols, size, pos) = if vertical {
            // Beside a column: towards the middle of the window.
            let cols = 12usize;
            let rows = colors.len().div_ceil(cols).max(1);
            let size = vec2(cols as f32 * STRIDE + 1.0, rows as f32 * STRIDE + 1.0);
            let x = if strip.x > screen.center().x {
                strip.x - size.x - 24.0
            } else {
                strip.x + thick + 8.0
            };
            let y = strip
                .y
                .min(screen.bottom() - size.y - 24.0)
                .max(screen.top());
            (cols, size, pos2(x, y))
        } else {
            let cols = (((screen.right() - strip.x - 24.0) / STRIDE).floor() as usize).clamp(1, 33);
            let rows = colors.len().div_ceil(cols).max(1);
            let size = vec2(cols as f32 * STRIDE + 1.0, rows as f32 * STRIDE + 1.0);
            // Above the row, or below it when it is docked at the top.
            let above = strip.y - size.y - 16.0;
            let y = if above < screen.top() {
                strip.y + thick + 8.0
            } else {
                above
            };
            (cols, size, pos2(strip.x - 7.0, y))
        };
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
                            .rich_tip(swatch_tip(name, *c));
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
                app.palette_fill(c);
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
    let r = ctx.data(|d| d.get_temp::<Rect>(egui::Id::new(("palette_expand_rect", which.key()))));
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
        assert_eq!(
            color_lines(Color::cmyk_pct(0.0, 60.0, 100.0, 0.0)),
            "C: 0\nM: 60\nY: 100\nK: 0"
        );
        assert_eq!(color_lines(Color::rgb8(255, 0, 10)), "R: 255\nG: 0\nB: 10");
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

    /// The eyedropper of the document palette picking a colour it has
    /// already frames that swatch, as the last colour used; a new colour
    /// joins and takes the frame, and applying a palette colour moves it
    /// there (#1125).
    #[test]
    fn a_colour_picked_again_frames_its_swatch() {
        let mut app = App::headless();
        let red = Color::rgb8(255, 0, 0);
        let blue = Color::rgb8(0, 0, 255);
        app.add_to_document_palette(red);
        app.add_to_document_palette(blue);
        let n = app.doc().palette.len();
        let at = |app: &App, c: Color| app.doc().palette.iter().position(|x| *x == c);
        assert_eq!(app.doc_palette_current, at(&app, blue));
        app.add_to_document_palette(red);
        assert_eq!(app.doc().palette.len(), n, "no second red swatch");
        assert_eq!(app.doc_palette_current, at(&app, red));
        app.remember_color(blue);
        assert_eq!(app.doc_palette_current, at(&app, blue));
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

    /// On the Welcome Screen the palette takes no clicks: a click on a
    /// swatch fills the selection of the drawing behind only while the
    /// drawing shows (#482).
    #[test]
    fn the_palette_rests_on_the_welcome_screen() {
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
        let ctx = crate::theme::ui_context();
        // A second apart, so no two clicks make a double click.
        let clock = std::cell::Cell::new(0.0);
        let click = |app: &mut App, x: f32| {
            clock.set(clock.get() + 1.0);
            // Over the swatch first, then pressed and let go.
            for (i, pressed) in [(0, false), (1, true), (2, false), (3, false)] {
                let mut events = vec![egui::Event::PointerMoved(egui::pos2(x, 10.0))];
                if i == 1 || i == 2 {
                    events.push(egui::Event::PointerButton {
                        pos: egui::pos2(x, 10.0),
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 100.0),
                    )),
                    events,
                    time: Some({
                        clock.set(clock.get() + 0.05);
                        clock.get()
                    }),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| palette_row(app, ui));
                out.textures_delta.clear();
            }
        };
        let fill = |app: &App| app.doc().find_shape(id).expect("shape").fill.clone();
        // A swatch that fills while the drawing shows...
        let before = fill(&app);
        let x = (0..30)
            .map(|k| 30.0 + k as f32 * 21.0)
            .find(|x| {
                click(&mut app, *x);
                let changed = fill(&app) != before;
                if changed {
                    app.undo();
                }
                changed
            })
            .expect("a swatch");
        // ...does nothing on the Welcome Screen.
        app.show_welcome = true;
        click(&mut app, x);
        assert_eq!(fill(&app), before);
        app.show_welcome = false;
        click(&mut app, x);
        assert_ne!(fill(&app), before);
        // The swatches keep their full colours on the Welcome Screen
        // (#1082): the row paints the same colours as with the drawing.
        let fills = |app: &mut App| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 100.0),
                )),
                ..Default::default()
            };
            let out = ctx.run_ui(input, |ui| palette_row(app, ui));
            out.shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Rect(r) => Some(r.fill),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let shown = fills(&mut app);
        app.show_welcome = true;
        let welcome = fills(&mut app);
        assert!(shown.len() > 10, "{shown:?}");
        assert_eq!(welcome, shown);
    }

    /// Every colour at once, for a palette of three rows along the top:
    /// the window opens below all three rows, not over them (#610).
    #[test]
    fn every_colour_opens_clear_of_all_the_rows() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.settings.palette.rows = 3;
        let run = |app: &mut App| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 700.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                palette_row(app, ui);
                popups(app, ui.ctx());
            });
            out.textures_delta.clear();
        };
        run(&mut app);
        app.palette_expanded = Some(Row::Main);
        run(&mut app);
        run(&mut app);
        let area = ctx
            .memory(|m| m.area_rect(egui::Id::new("palette_expanded")))
            .expect("every colour shown");
        assert!(
            area.top() >= thickness(&app, Row::Main),
            "{area:?} over the rows"
        );
    }

    /// With two rows the default palette is twice as thick and fills line
    /// by line: Black sits beside the No Color well with 90% Black next to
    /// it, and the second line goes on where the first ends; scrolling
    /// moves one swatch (#610).
    #[test]
    fn the_default_palette_can_show_several_rows() {
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
        let ctx = crate::theme::ui_context();
        let clock = std::cell::Cell::new(0.0);
        let height = std::cell::Cell::new(0.0);
        let click = |app: &mut App, p: egui::Pos2| {
            clock.set(clock.get() + 1.0);
            for (i, pressed) in [(0, false), (1, true), (2, false), (3, false)] {
                let mut events = vec![egui::Event::PointerMoved(p)];
                if i == 1 || i == 2 {
                    events.push(egui::Event::PointerButton {
                        pos: p,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 100.0),
                    )),
                    events,
                    time: Some({
                        clock.set(clock.get() + 0.05);
                        clock.get()
                    }),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    let top = ui.cursor().top();
                    palette_row(app, ui);
                    height.set(ui.cursor().top() - top);
                });
                out.textures_delta.clear();
            }
        };
        let fill = |app: &App| app.doc().find_shape(id).expect("shape").fill.clone();
        let before = fill(&app);
        // The first column, one swatch down.
        let x = LEAD + 1.0 + SWATCH / 2.0;
        let second_line = egui::pos2(x, 3.0 + STRIDE + SWATCH / 2.0);
        click(&mut app, second_line);
        assert_eq!(fill(&app), before, "one row: nothing there");
        assert_eq!(thickness(&app, Row::Main), ROW_H);
        app.settings.palette.rows = 2;
        assert_eq!(thickness(&app, Row::Main), ROW_H + STRIDE);
        assert_eq!(thickness(&app, Row::Document), ROW_H);
        click(&mut app, second_line);
        let solid = |app: &App| match fill(app) {
            Fill::Solid(c) => c,
            other => panic!("{other:?}"),
        };
        // The second line starts with the swatch after the first line's
        // last (the No Color well takes the first slot).
        let cols = visible_slots(800.0);
        assert_eq!(solid(&app), app.palette[cols - 1].1);
        assert!(height.get() >= ROW_H + STRIDE, "{}", height.get());
        click(&mut app, egui::pos2(x + STRIDE, 3.0 + SWATCH / 2.0));
        assert_eq!(solid(&app), Color::cmyk_pct(0.0, 0.0, 0.0, 100.0));
        click(&mut app, egui::pos2(x + 2.0 * STRIDE, 3.0 + SWATCH / 2.0));
        assert_eq!(solid(&app), Color::cmyk_pct(0.0, 0.0, 0.0, 90.0));
        // The scroll arrow moves one swatch: Black comes under the pointer
        // of the first swatch.
        click(&mut app, egui::pos2(800.0 - 49.0 + 8.0, 12.0));
        assert_eq!(app.palette_scroll, 1);
        click(&mut app, egui::pos2(x, 3.0 + SWATCH / 2.0));
        assert_eq!(solid(&app), Color::cmyk_pct(0.0, 0.0, 0.0, 100.0));
        // More than three rows is not kept.
        app.settings.palette.rows = 9;
        assert_eq!(thickness(&app, Row::Main), ROW_H + 2.0 * STRIDE);
    }

    /// The palette's own menu has Rows > 1 Row, 2 Rows, 3 Rows: picking
    /// one changes the strip's rows and keeps it in the preferences
    /// (#610).
    #[test]
    fn the_palette_menu_picks_the_rows() {
        let mut app = App::headless();
        let ctx = crate::theme::ui_context();
        App::setup_context(&ctx);
        let clock = std::cell::Cell::new(0.0);
        // One frame with `events`; the place of each text drawn.
        let mut frame = |app: &mut App, events: Vec<egui::Event>| -> Vec<(String, egui::Rect)> {
            clock.set(clock.get() + 0.1);
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                events,
                time: Some(clock.get()),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                ui.add_space(400.0);
                palette_row(app, ui)
            });
            out.textures_delta.clear();
            fn walk(s: &egui::Shape, texts: &mut Vec<(String, egui::Rect)>) {
                match s {
                    egui::Shape::Text(t) => texts.push((
                        t.galley.text().to_string(),
                        t.galley.rect.translate(t.pos.to_vec2()),
                    )),
                    egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, texts)),
                    _ => {}
                }
            }
            let mut texts = Vec::new();
            out.shapes.iter().for_each(|c| walk(&c.shape, &mut texts));
            texts
        };
        let press = |p: egui::Pos2| {
            vec![
                egui::Event::PointerMoved(p),
                egui::Event::PointerButton {
                    pos: p,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        let release = |p: egui::Pos2| {
            vec![egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }]
        };
        let texts = frame(&mut app, Vec::new());
        assert!(texts.iter().all(|(t, _)| t != &tr("palette.rows")));
        // The menu button at the start of the strip.
        let button = egui::pos2(13.0 + 7.5, 400.0 + 2.0 + 11.0);
        frame(&mut app, press(button));
        frame(&mut app, release(button));
        let mut texts = frame(&mut app, Vec::new());
        let at = |texts: &[(String, egui::Rect)], key: &str| {
            texts
                .iter()
                .find(|(t, _)| t == &tr(key))
                .map(|(_, r)| r.center())
        };
        let rows = at(&texts, "palette.rows").expect("Rows in the palette menu");
        // Hovering Rows opens its submenu.
        for _ in 0..4 {
            texts = frame(&mut app, vec![egui::Event::PointerMoved(rows)]);
        }
        let two = at(&texts, "palette.rows_2").expect("2 Rows in the submenu");
        assert!(at(&texts, "palette.rows_1").is_some());
        assert!(at(&texts, "palette.rows_3").is_some());
        assert_eq!(app.settings.palette.rows(), 1);
        frame(
            &mut app,
            vec![egui::Event::PointerMoved(egui::pos2(two.x, rows.y))],
        );
        frame(&mut app, vec![egui::Event::PointerMoved(two)]);
        frame(&mut app, press(two));
        frame(&mut app, release(two));
        frame(&mut app, Vec::new());
        assert_eq!(app.settings.palette.rows(), 2);
        assert_eq!(thickness(&app, Row::Main), ROW_H + STRIDE);
    }

    /// An RGB drawing's palette holds the palette's colours as RGB (named
    /// as before); a CMYK drawing's keeps them CMYK (#483).
    #[test]
    fn an_rgb_drawing_gets_an_rgb_palette() {
        let mut app = App::headless();
        app.palette = vec![
            ("Cyan".into(), Color::cmyk_pct(100.0, 0.0, 0.0, 0.0)),
            ("Mine".into(), Color::rgb8(10, 20, 30)),
        ];
        app.palette_rgb = as_rgb(&app.palette);
        let cmyk = colors_of(&app, Row::Main);
        assert_eq!(cmyk, app.palette);
        let mut metadata = app.doc().metadata.clone();
        metadata.primary_color_mode = tracedraw_core::document::PrimaryColorMode::Rgb;
        app.run(tracedraw_core::Command::SetMetadata { metadata });
        let rgb = colors_of(&app, Row::Main);
        assert_eq!(rgb.len(), 2);
        assert!(rgb.iter().all(|(_, c)| c.model_name() == "RGB"), "{rgb:?}");
        assert_eq!(rgb[0].0, "Cyan");
        assert_eq!(rgb[0].1, Color::rgb8(0, 255, 255));
        assert_eq!(rgb[1].1, Color::rgb8(10, 20, 30));
    }

    /// The default palette in an RGB drawing holds clean RGB values: black
    /// 0 0 0, white 255 255 255, the primaries at full, the tints as RGB
    /// tints (#620).
    #[test]
    fn the_default_palette_of_an_rgb_drawing_is_clean_rgb() {
        let mut app = App::headless();
        let mut metadata = app.doc().metadata.clone();
        metadata.primary_color_mode = tracedraw_core::document::PrimaryColorMode::Rgb;
        app.run(tracedraw_core::Command::SetMetadata { metadata });
        let rgb = colors_of(&app, Row::Main);
        let of = |name: &str| {
            rgb.iter()
                .find(|(n, _)| n == name)
                .map(|(_, c)| *c)
                .expect(name)
        };
        assert_eq!(rgb.len(), 99);
        assert_eq!(of("Black"), Color::rgb8(0, 0, 0));
        assert_eq!(of("White"), Color::rgb8(255, 255, 255));
        assert_eq!(of("50% Black"), Color::rgb8(128, 128, 128));
        assert_eq!(of("Red"), Color::rgb8(255, 0, 0));
        assert_eq!(of("Green"), Color::rgb8(0, 255, 0));
        assert_eq!(of("Blue"), Color::rgb8(0, 0, 255));
        assert_eq!(of("Yellow"), Color::rgb8(255, 255, 0));
        assert_eq!(of("Magenta"), Color::rgb8(255, 0, 255));
        assert_eq!(of("Orange"), Color::rgb8(255, 102, 0));
        assert!(rgb.iter().all(|(_, c)| c.model_name() == "RGB"));
        // Swatches of the reference RGB palette (#620): the colours with
        // black in them are clean RGB values too.
        assert_eq!(of("Navy Blue"), Color::rgb8(0, 51, 153));
        assert_eq!(of("Purple"), Color::rgb8(153, 0, 204));
        assert_eq!(of("Dark Brown"), Color::rgb8(102, 51, 51));
        assert_eq!(of("Ruby Red"), Color::rgb8(153, 0, 0));
        assert_eq!(of("Twilight Blue"), Color::rgb8(102, 102, 204));
        assert_eq!(of("Forest Green"), Color::rgb8(0, 102, 51));
        assert_eq!(of("Walnut"), Color::rgb8(102, 51, 0));
        assert_eq!(of("90% Black"), Color::rgb8(26, 26, 26));
        assert_eq!(of("Crimson"), Color::rgb8(153, 51, 102));
        assert_eq!(of("Plum"), Color::rgb8(102, 0, 102));
        assert_eq!(of("Deep Blue"), Color::rgb8(51, 0, 204));
        // Every channel is a multiple of 51 or a 10% grey.
        for (n, c) in &rgb {
            let [r, g, b] = c.to_rgb8();
            let grey = r == g && g == b;
            assert!(
                grey || [r, g, b].iter().all(|v| v % 51 == 0),
                "{n}: {r} {g} {b}"
            );
        }
    }

    /// A colour added to the default palette in the Palette Editor moves
    /// the others along, and they keep their RGB values; the Palette
    /// Manager's built-in RGB default palette is the same table (#620).
    #[test]
    fn edited_default_palettes_keep_their_rgb_values() {
        let mut palette = crate::palette::default_cmyk();
        palette.insert(0, ("Mine".into(), Color::cmyk_pct(10.0, 0.0, 0.0, 0.0)));
        palette.retain(|(n, _)| n != "Black");
        let rgb = as_rgb(&palette);
        let of = |name: &str| rgb.iter().find(|(n, _)| n == name).map(|(_, c)| *c);
        assert_eq!(of("Crimson"), Some(Color::rgb8(0x99, 0x33, 0x66)));
        assert_eq!(of("Navy Blue"), Some(Color::rgb8(0, 51, 153)));
        assert_eq!(of("Mine"), Some(Color::rgb8(230, 255, 255)));
        // A default name with another colour is converted from its inks.
        let changed = vec![("Crimson".to_string(), Color::cmyk_pct(0.0, 100.0, 0.0, 0.0))];
        assert_eq!(as_rgb(&changed)[0].1, Color::rgb8(255, 0, 255));
        let builtin = crate::palette::builtin_palettes()
            .into_iter()
            .find(|p| p.key == "palette.default_rgb")
            .expect("the RGB default palette");
        assert_eq!(builtin.colors, crate::palette::default_rgb());
    }

    /// A CMYK colour outside the default palette takes 255 (1 - min(1,
    /// ink + K)) per channel in an RGB drawing (#620).
    #[test]
    fn inks_with_black_become_rgb_by_adding_the_black() {
        assert_eq!(
            in_rgb(Color::cmyk_pct(60.0, 40.0, 0.0, 40.0)),
            Color::rgb8(0, 51, 153)
        );
        assert_eq!(
            in_rgb(Color::cmyk_pct(0.0, 20.0, 20.0, 60.0)),
            Color::rgb8(102, 51, 51)
        );
        assert_eq!(
            in_rgb(Color::cmyk_pct(0.0, 0.0, 0.0, 100.0)),
            Color::rgb8(0, 0, 0)
        );
    }

    #[test]
    fn rows_fit_whole_swatches() {
        assert_eq!(visible_slots(LEAD + TAIL + 1.0 + 21.0 * 10.0), 10);
        assert_eq!(visible_slots(10.0), 1);
    }
}
