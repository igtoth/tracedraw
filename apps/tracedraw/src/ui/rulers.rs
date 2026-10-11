//! The rulers along the top and the left of the drawing window:
//! numbered marks centred over long
//! ticks, half and minor ticks, distances from the ruler origin without
//! sign, the unit name at the right end of the horizontal ruler and the
//! origin button in the corner where the rulers meet.
//!
//! Dragging out of a ruler adds a guideline, dragging out of the corner
//! moves the ruler origin, double-clicking a ruler or the corner opens the
//! ruler settings (Document Options, Rulers).

use crate::app::{App, Drag};
use crate::theme::Tokens;
use egui::{epaint, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke, Ui};
use tracedraw_core::geometry::Point;

/// Smallest distance between numbered marks, screen pixels.
const MIN_MAJOR_PX: f64 = 44.0;
/// Smallest distance between tick marks, screen pixels.
const MIN_TICK_PX: f64 = 3.0;
/// Lengths of the tick marks, pixels.
const MAJOR_TICK: f32 = 5.0;
const HALF_TICK: f32 = 2.0;
const MINOR_TICK: f32 = 1.0;
/// Gap between the ticks and the ruler's inner edge: one blank pixel row
/// and the one-pixel border.
const TICK_GAP: f32 = 2.0;
/// Size of the numbers.
const LABEL_SIZE: f32 = 10.0;
/// Offset of the number's text box from the ruler's outer edge.
const LABEL_OFFSET: f32 = 0.0;

pub const TICK_COLOR: Color32 = Color32::from_gray(140);
pub const LABEL_COLOR: Color32 = Color32::from_gray(118);
pub const BORDER_COLOR: Color32 = Color32::from_gray(216);
/// The pointer position marker: a dashed dark line across each ruler.
const MARKER_COLOR: Color32 = Color32::from_gray(11);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickKind {
    /// Numbered mark.
    Major,
    /// Halfway between two numbered marks.
    Half,
    Minor,
}

/// The step between numbered marks, in ruler units: 1, 2 or 5 times a
/// power of ten, the smallest that keeps the numbers apart.
pub fn major_step(px_per_unit: f64) -> f64 {
    if !(px_per_unit.is_finite() && px_per_unit > 0.0) {
        return 1.0;
    }
    for k in -6..=12 {
        for m in [1.0, 2.0, 5.0] {
            let step = m * 10f64.powi(k);
            if step * px_per_unit >= MIN_MAJOR_PX {
                return step;
            }
        }
    }
    1e12
}

/// Ticks per numbered step: the tick divisions setting, or fewer when the
/// ticks would crowd at this zoom.
pub fn divisions(major: f64, px_per_unit: f64, wanted: u32) -> u32 {
    let n = wanted.clamp(1, 100);
    let mut candidates = vec![n];
    if n.is_multiple_of(2) {
        candidates.push(n / 2);
    }
    for d in [5, 4, 2, 1] {
        if d < n && n.is_multiple_of(d) {
            candidates.push(d);
        }
    }
    candidates
        .into_iter()
        .find(|d| major / *d as f64 * px_per_unit >= MIN_TICK_PX)
        .unwrap_or(1)
}

/// The ticks whose values lie in `[v0, v1]` (ruler units), with their
/// kind. `wanted` is the tick divisions setting.
pub fn ticks(v0: f64, v1: f64, px_per_unit: f64, wanted: u32) -> Vec<(f64, TickKind)> {
    let (v0, v1) = (v0.min(v1), v0.max(v1));
    let major = major_step(px_per_unit);
    let d = divisions(major, px_per_unit, wanted);
    let minor = major / d as f64;
    let first = (v0 / minor).floor() as i64;
    let last = (v1 / minor).ceil() as i64;
    if last - first > 20_000 {
        return Vec::new();
    }
    let d = d as i64;
    (first..=last)
        .map(|i| {
            let kind = if i.rem_euclid(d) == 0 {
                TickKind::Major
            } else if d % 2 == 0 && i.rem_euclid(d / 2) == 0 {
                TickKind::Half
            } else {
                TickKind::Minor
            };
            (i as f64 * minor, kind)
        })
        .filter(|(v, _)| *v >= v0 - minor * 0.5 && *v <= v1 + minor * 0.5)
        .collect()
}

/// The number shown at a numbered mark: the distance from the origin,
/// without sign, with as many decimals as the step needs.
pub fn label(v: f64, major: f64) -> String {
    let decimals = if major >= 1.0 {
        0
    } else {
        (-major.log10()).ceil().max(0.0) as usize
    };
    let s = format!("{:.*}", decimals, v.abs());
    // No "-0" for values that round to zero.
    if s.chars().all(|c| c == '0' || c == '.') {
        "0".to_string()
    } else {
        s
    }
}

/// The ruler value (signed, in the ruler unit) of a page position along
/// one axis.
fn value_of(app: &App, mm: f64, origin: f64) -> f64 {
    app.units.from_mm(mm - origin)
}

/// The rulers, their corner, the pointer markers and their interaction.
/// `full` is the whole drawing window; the canvas starts one ruler width
/// in from its top-left corner.
pub fn rulers(app: &mut App, ui: &mut Ui, full: Rect) {
    let r = Tokens::RULER;
    let top = Rect::from_min_max(
        Pos2::new(full.min.x + r, full.min.y),
        Pos2::new(full.max.x, full.min.y + r),
    );
    let left = Rect::from_min_max(
        Pos2::new(full.min.x, full.min.y + r),
        Pos2::new(full.min.x + r, full.max.y),
    );
    let corner = Rect::from_min_size(full.min, egui::vec2(r, r));
    draw_horizontal(app, &ui.painter_at(top), top);
    draw_vertical(app, &ui.painter_at(left), left);
    draw_corner(&ui.painter_at(corner), corner, app.drag_is_origin());

    // Dragging out of a ruler creates a guideline; out of the corner, it
    // moves the ruler origin. A double-click opens the ruler settings.
    let top_resp = ui.interact(top, egui::Id::new("ruler_top"), Sense::click_and_drag());
    let left_resp = ui.interact(left, egui::Id::new("ruler_left"), Sense::click_and_drag());
    let corner_resp = ui
        .interact(
            corner,
            egui::Id::new("ruler_corner"),
            Sense::click_and_drag(),
        )
        .on_hover_text(crate::i18n::tr("rulers.origin_tip"));
    if top_resp.drag_started() {
        app.drag = Drag::NewGuide {
            horizontal: true,
            pos: Point::ZERO,
        };
    }
    if left_resp.drag_started() {
        app.drag = Drag::NewGuide {
            horizontal: false,
            pos: Point::ZERO,
        };
    }
    if corner_resp.drag_started() {
        app.drag = Drag::RulerOrigin { pos: Point::ZERO };
    }
    if top_resp.double_clicked() || left_resp.double_clicked() || corner_resp.double_clicked() {
        app.open_ruler_options();
    }
    if top_resp.hovered() || left_resp.hovered() || corner_resp.hovered() {
        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Default);
    }
}

/// Follow the pointer while a guideline or the ruler origin is dragged out
/// of the rulers, and finish on release. Returns true while such a drag is
/// active (the canvas then ignores the pointer).
pub fn ruler_drag(app: &mut App, ui: &Ui) -> bool {
    let pos = ui.input(|i| i.pointer.latest_pos());
    let released = ui.input(|i| i.pointer.primary_released());
    match app.drag {
        Drag::NewGuide { horizontal, .. } => {
            if let Some(s) = pos {
                let p = app.snap_point_without_guides(app.view.to_page(s));
                app.drag = Drag::NewGuide { horizontal, pos: p };
            }
            if released {
                app.finish_guide_drag();
            }
            true
        }
        Drag::RulerOrigin { .. } => {
            if let Some(s) = pos {
                let p = app.snap_point_without_guides(app.view.to_page(s));
                app.drag = Drag::RulerOrigin { pos: p };
            }
            if released {
                if let Drag::RulerOrigin { pos } = app.drag {
                    if app.canvas_rect.contains(app.view.to_screen(pos)) {
                        app.set_ruler_origin(pos);
                    }
                }
                app.drag = Drag::None;
            }
            true
        }
        _ => false,
    }
}

fn label_font() -> FontId {
    FontId::proportional(LABEL_SIZE)
}

/// Whole-pixel rectangles collected into one mesh: meshes are drawn
/// without anti-aliasing, so one-pixel ticks stay crisp and dark.
#[derive(Default)]
pub struct Pixels(epaint::Mesh);

impl Pixels {
    pub fn rect(&mut self, r: Rect, c: Color32) {
        self.0.add_colored_rect(r, c);
    }

    pub fn dot(&mut self, x: f32, y: f32, c: Color32) {
        self.rect(
            Rect::from_min_size(Pos2::new(x, y), egui::vec2(1.0, 1.0)),
            c,
        );
    }

    pub fn paint(self, painter: &Painter) {
        if !self.0.is_empty() {
            painter.add(egui::Shape::mesh(self.0));
        }
    }
}

/// A dashed axis-aligned line, 3 pixels on and 3 off (the pointer
/// markers); `a` and `b` share x or y.
fn dashed(painter: &Painter, a: Pos2, b: Pos2, color: Color32) {
    let mut pixels = Pixels::default();
    let (x0, y0) = (a.x.floor(), a.y.floor());
    if (a.x - b.x).abs() < 0.5 {
        let mut y = a.y.min(b.y).floor();
        let end = a.y.max(b.y);
        while y < end {
            pixels.rect(
                Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x0 + 1.0, (y + 3.0).min(end))),
                color,
            );
            y += 6.0;
        }
    } else {
        let mut x = a.x.min(b.x).floor();
        let end = a.x.max(b.x);
        while x < end {
            pixels.rect(
                Rect::from_min_max(Pos2::new(x, y0), Pos2::new((x + 3.0).min(end), y0 + 1.0)),
                color,
            );
            x += 6.0;
        }
    }
    pixels.paint(painter);
}

fn draw_horizontal(app: &App, painter: &Painter, top: Rect) {
    painter.rect_filled(top, 0.0, Tokens::RULER_BG);
    painter.hline(
        top.x_range(),
        top.bottom() - 0.5,
        Stroke::new(1.0, BORDER_COLOR),
    );
    let view = &app.view;
    let rulers = app.doc().metadata.rulers;
    let ppu = view.zoom as f64 * app.units.mm();
    let a = value_of(app, view.to_page(top.left_top()).x, rulers.origin_x);
    let b = value_of(app, view.to_page(top.right_top()).x, rulers.origin_x);
    let major = major_step(ppu);
    let font = label_font();

    // The unit name at the right end, over a patch of background so the
    // numbers do not run into it.
    let unit = painter.layout_no_wrap(app.units.label(), font.clone(), LABEL_COLOR);
    let unit_rect = Rect::from_min_size(
        Pos2::new(top.right() - unit.size().x - 4.0, top.top() + LABEL_OFFSET),
        unit.size(),
    );
    let base = top.bottom() - TICK_GAP;
    let mut pixels = Pixels::default();
    for (v, kind) in ticks(a, b, ppu, rulers.tick_divisions) {
        let px = view
            .to_screen(Point::new(app.units.to_mm(v) + rulers.origin_x, 0.0))
            .x
            .round();
        let x = px + 0.5;
        let h = match kind {
            TickKind::Major => MAJOR_TICK,
            TickKind::Half => HALF_TICK,
            TickKind::Minor => MINOR_TICK,
        };
        pixels.rect(
            Rect::from_min_max(Pos2::new(px, base - h), Pos2::new(px + 1.0, base)),
            TICK_COLOR,
        );
        if kind == TickKind::Major {
            let g = painter.layout_no_wrap(label(v, major), font.clone(), LABEL_COLOR);
            let rect = Rect::from_min_size(
                Pos2::new((x - g.size().x / 2.0).round(), top.top() + LABEL_OFFSET),
                g.size(),
            );
            if !rect.intersects(unit_rect.expand2(egui::vec2(3.0, 0.0))) {
                painter.galley(rect.min, g, LABEL_COLOR);
            }
        }
    }
    pixels.paint(painter);
    painter.rect_filled(
        unit_rect.expand2(egui::vec2(2.0, 0.0)),
        0.0,
        Tokens::RULER_BG,
    );
    painter.galley(unit_rect.min, unit, LABEL_COLOR);

    if let Some(x) = marker_x(app) {
        dashed(
            painter,
            Pos2::new(x, top.top() + 1.0),
            Pos2::new(x, top.bottom() - 1.0),
            MARKER_COLOR,
        );
    }
}

fn draw_vertical(app: &App, painter: &Painter, left: Rect) {
    painter.rect_filled(left, 0.0, Tokens::RULER_BG);
    painter.vline(
        left.right() - 0.5,
        left.y_range(),
        Stroke::new(1.0, BORDER_COLOR),
    );
    let view = &app.view;
    let rulers = app.doc().metadata.rulers;
    let ppu = view.zoom as f64 * app.units.mm();
    // Y up: the bottom of the ruler has the smaller values.
    let a = value_of(app, view.to_page(left.left_bottom()).y, rulers.origin_y);
    let b = value_of(app, view.to_page(left.left_top()).y, rulers.origin_y);
    let major = major_step(ppu);
    let font = label_font();
    let base = left.right() - TICK_GAP;
    let mut pixels = Pixels::default();
    for (v, kind) in ticks(a, b, ppu, rulers.tick_divisions) {
        let py = view
            .to_screen(Point::new(0.0, app.units.to_mm(v) + rulers.origin_y))
            .y
            .round();
        let y = py + 0.5;
        let w = match kind {
            TickKind::Major => MAJOR_TICK,
            TickKind::Half => HALF_TICK,
            TickKind::Minor => MINOR_TICK,
        };
        pixels.rect(
            Rect::from_min_max(Pos2::new(base - w, py), Pos2::new(base, py + 1.0)),
            TICK_COLOR,
        );
        if kind == TickKind::Major {
            // Numbers read upwards, centred on their tick.
            let g = painter.layout_no_wrap(label(v, major), font.clone(), LABEL_COLOR);
            let len = g.size().x;
            let mut ts = epaint::TextShape::new(
                Pos2::new(left.left() + LABEL_OFFSET, (y + len / 2.0).round()),
                g,
                LABEL_COLOR,
            );
            ts.angle = -std::f32::consts::FRAC_PI_2;
            painter.add(ts);
        }
    }
    pixels.paint(painter);
    if let Some(y) = marker_y(app) {
        dashed(
            painter,
            Pos2::new(left.left() + 1.0, y),
            Pos2::new(left.right() - 1.0, y),
            MARKER_COLOR,
        );
    }
}

/// Screen x of the pointer marker on the horizontal ruler.
fn marker_x(app: &App) -> Option<f32> {
    let p = marker_point(app)?;
    Some(app.view.to_screen(p).x.round() + 0.5)
}

fn marker_y(app: &App) -> Option<f32> {
    let p = marker_point(app)?;
    Some(app.view.to_screen(p).y.round() + 0.5)
}

fn marker_point(app: &App) -> Option<Point> {
    match app.drag {
        Drag::NewGuide { pos, .. } | Drag::RulerOrigin { pos } => Some(pos),
        _ => app.pointer_page,
    }
}

/// The origin button: a dotted cross with a handle pulled out of it.
fn draw_corner(painter: &Painter, corner: Rect, active: bool) {
    painter.rect_filled(corner, 0.0, Tokens::RULER_BG);
    if active {
        painter.rect_filled(corner.shrink(1.0), 0.0, Tokens::TOOL_HOVER);
    }
    painter.hline(
        corner.x_range(),
        corner.bottom() - 0.5,
        Stroke::new(1.0, BORDER_COLOR),
    );
    painter.vline(
        corner.right() - 0.5,
        corner.y_range(),
        Stroke::new(1.0, BORDER_COLOR),
    );
    // Pixel pattern (17 px corner): a
    // dotted vertical line at x 6, a dotted horizontal one at y 6, and a
    // diagonal from (7, 7) to (11, 11) ending in a 2x2 square.
    let mut pixels = Pixels::default();
    let (x0, y0) = (corner.min.x.round(), corner.min.y.round());
    let dot = Color32::from_gray(148);
    for i in (0..10).step_by(2) {
        pixels.dot(x0 + 6.0, y0 + 4.0 + i as f32, dot);
        pixels.dot(x0 + 4.0 + i as f32, y0 + 6.0, dot);
    }
    let d = Color32::from_gray(109);
    for i in 0..5 {
        pixels.dot(x0 + 7.0 + i as f32, y0 + 7.0 + i as f32, d);
    }
    for (x, y) in [(11.0, 12.0), (12.0, 11.0), (12.0, 12.0)] {
        pixels.dot(x0 + x, y0 + y, Color32::from_gray(136));
    }
    pixels.paint(painter);
}

/// While the origin is dragged, dashed lines across the drawing window
/// show where it will go.
pub fn draw_origin_drag(app: &App, painter: &Painter, canvas: Rect) {
    if let Drag::RulerOrigin { pos } = app.drag {
        let s = app.view.to_screen(pos);
        let (x, y) = (s.x.round() + 0.5, s.y.round() + 0.5);
        dashed(
            painter,
            Pos2::new(canvas.left(), y),
            Pos2::new(canvas.right(), y),
            Tokens::TEXT,
        );
        dashed(
            painter,
            Pos2::new(x, canvas.top()),
            Pos2::new(x, canvas.bottom()),
            Tokens::TEXT,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbered_marks_keep_their_distance() {
        // Half a screen pixel per unit: a mark every 100 units (50 px).
        assert_eq!(major_step(0.5), 100.0);
        assert_eq!(major_step(2.0), 50.0);
        assert_eq!(major_step(10.0), 5.0);
        assert_eq!(major_step(100.0), 0.5);
        for ppu in [0.001, 0.37, 1.0, 3.3, 47.0, 2000.0] {
            let s = major_step(ppu);
            assert!(s * ppu >= MIN_MAJOR_PX, "{ppu}");
            assert!(s * ppu < MIN_MAJOR_PX * 2.6, "{ppu}");
        }
        assert_eq!(major_step(0.0), 1.0);
        assert_eq!(major_step(f64::NAN), 1.0);
    }

    #[test]
    fn ticks_follow_the_tick_divisions_and_thin_out() {
        // 0.5 px per unit: majors of 100 with ten 10-unit ticks (5 px).
        let t = ticks(0.0, 100.0, 0.5, 10);
        assert_eq!(t.len(), 11);
        assert_eq!(t[0], (0.0, TickKind::Major));
        assert_eq!(t[5], (50.0, TickKind::Half));
        assert_eq!(t[1].1, TickKind::Minor);
        assert_eq!(t[10], (100.0, TickKind::Major));
        // When ten would crowd, five, then two.
        assert_eq!(divisions(100.0, 0.25, 10), 5);
        assert_eq!(divisions(100.0, 0.07, 10), 2);
        assert_eq!(divisions(100.0, 0.01, 10), 1);
        // Eight divisions (inches) keep a half mark.
        let t = ticks(0.0, 1.0, 50.0, 8);
        assert_eq!(t.iter().filter(|(_, k)| *k == TickKind::Half).count(), 1);
        // Absurd ranges give nothing rather than millions of ticks.
        assert!(ticks(-1e12, 1e12, 1000.0, 10).is_empty());
    }

    #[test]
    fn labels_have_no_sign_and_just_enough_decimals() {
        assert_eq!(label(-200.0, 100.0), "200");
        assert_eq!(label(0.0, 100.0), "0");
        assert_eq!(label(-0.0, 100.0), "0");
        assert_eq!(label(2.5, 0.5), "2.5");
        assert_eq!(label(0.05, 0.05), "0.05");
        assert_eq!(label(-0.001, 0.5), "0");
    }
}
