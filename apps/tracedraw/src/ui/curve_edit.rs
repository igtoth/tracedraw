//! A tone curve editor (Tone Curve filter, duotone inks): a square grid
//! with the curve on it. Dragging on empty space adds a point there,
//! dragging a point moves it (the end points move up and down only),
//! double-clicking a point or dragging it out of the grid removes it.

use crate::bitmap_modes::ToneCurve;
use egui::{epaint, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

const PICK: f32 = 7.0;

fn to_screen(r: Rect, x: f32, y: f32) -> Pos2 {
    Pos2::new(
        r.left() + x / 255.0 * r.width(),
        r.bottom() - y / 255.0 * r.height(),
    )
}

fn to_curve(r: Rect, p: Pos2) -> (f32, f32) {
    (
        ((p.x - r.left()) / r.width() * 255.0).clamp(0.0, 255.0),
        ((r.bottom() - p.y) / r.height() * 255.0).clamp(0.0, 255.0),
    )
}

/// The grid, the histogram (grey bars), the diagonal and other curves
/// behind the curve being edited.
fn paint_frame(
    painter: &egui::Painter,
    rect: Rect,
    others: &[(&ToneCurve, Color32)],
    hist: Option<&[u32; 256]>,
) {
    painter.rect_filled(rect, 0.0, Color32::WHITE);
    if let Some(h) = hist {
        let max = h.iter().copied().max().unwrap_or(1).max(1) as f32;
        for (i, v) in h.iter().enumerate() {
            if *v == 0 {
                continue;
            }
            let x = rect.left() + (i as f32 + 0.5) / 256.0 * rect.width();
            let top = rect.bottom() - (*v as f32 / max).sqrt() * rect.height() * 0.9;
            painter.line_segment(
                [Pos2::new(x, rect.bottom()), Pos2::new(x, top)],
                Stroke::new(rect.width() / 256.0 + 0.2, Color32::from_gray(0xDD)),
            );
        }
    }
    let grid = Stroke::new(1.0, Color32::from_gray(0xE4));
    for k in 1..4 {
        let t = k as f32 / 4.0;
        painter.line_segment(
            [
                Pos2::new(rect.left() + t * rect.width(), rect.top()),
                Pos2::new(rect.left() + t * rect.width(), rect.bottom()),
            ],
            grid,
        );
        painter.line_segment(
            [
                Pos2::new(rect.left(), rect.top() + t * rect.height()),
                Pos2::new(rect.right(), rect.top() + t * rect.height()),
            ],
            grid,
        );
    }
    painter.line_segment(
        [rect.left_bottom(), rect.right_top()],
        Stroke::new(1.0, Color32::from_gray(0xC8)),
    );
    for (c, col) in others {
        paint_curve(painter, rect, c, Stroke::new(1.0, *col));
    }
}

fn paint_curve(painter: &egui::Painter, rect: Rect, c: &ToneCurve, stroke: Stroke) {
    let pts: Vec<Pos2> = (0..=64)
        .map(|i| {
            let x = i as f32 * 255.0 / 64.0;
            to_screen(rect, x, c.eval(x))
        })
        .collect();
    painter.add(epaint::PathShape::line(pts, stroke));
}

/// The border and the levels under the pointer (input and output).
fn paint_readout(painter: &egui::Painter, rect: Rect, hover: Option<Pos2>, curve: &ToneCurve) {
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, crate::theme::Tokens::BORDER),
        egui::StrokeKind::Inside,
    );
    if let Some(p) = hover {
        let (x, _) = to_curve(rect, p);
        let text = format!("{} \u{2192} {}", x.round(), curve.eval(x).round());
        painter.text(
            rect.left_top() + Vec2::new(4.0, 2.0),
            egui::Align2::LEFT_TOP,
            text,
            egui::FontId::proportional(11.0),
            crate::theme::Tokens::TEXT_DIM,
        );
    }
}

/// The editor in a `size` px square; `others` are drawn thin behind the
/// curve, `hist` as grey bars. True when the curve changed.
pub fn curve_editor(
    ui: &mut Ui,
    id: egui::Id,
    curve: &mut ToneCurve,
    size: f32,
    color: Color32,
    others: &[(&ToneCurve, Color32)],
    hist: Option<&[u32; 256]>,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
    let painter = ui.painter_at(rect.expand(4.0));
    paint_frame(&painter, rect, others, hist);

    // Interaction: which point is being dragged lives in the memory.
    let drag_id = id.with("drag");
    let mut changed = false;
    let mut points = curve.normalized();
    let nearest = |pts: &[(f32, f32)], p: Pos2| -> Option<usize> {
        pts.iter()
            .enumerate()
            .map(|(i, (x, y))| (i, to_screen(rect, *x, *y).distance(p)))
            .filter(|(_, d)| *d <= PICK)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    };
    if resp.double_clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            if let Some(i) = nearest(&points, p) {
                if i != 0 && i != points.len() - 1 {
                    points.remove(i);
                    changed = true;
                }
            }
        }
    }
    if resp.drag_started() {
        if let Some(p) = ui.input(|i| i.pointer.press_origin()) {
            let i = match nearest(&points, p) {
                Some(i) => i,
                None => {
                    let (x, _) = to_curve(rect, p);
                    let at = points.partition_point(|q| q.0 < x);
                    points.insert(at, (x, curve.eval(x)));
                    changed = true;
                    at
                }
            };
            ui.memory_mut(|m| m.data.insert_temp(drag_id, i));
        }
    }
    if resp.dragged() {
        if let (Some(i), Some(p)) = (
            ui.memory(|m| m.data.get_temp::<usize>(drag_id)),
            resp.interact_pointer_pos(),
        ) {
            if i < points.len() {
                let (mut x, y) = to_curve(rect, p);
                let last = points.len() - 1;
                if i == 0 {
                    x = points[0].0;
                } else if i == last {
                    x = points[last].0;
                } else {
                    let lo = points[i - 1].0 + 1.0;
                    let hi = points[i + 1].0 - 1.0;
                    x = x.clamp(lo.min(hi), hi.max(lo));
                }
                points[i] = (x, y);
                changed = true;
            }
        }
    }
    if resp.drag_stopped() {
        if let (Some(i), Some(p)) = (
            ui.memory(|m| m.data.get_temp::<usize>(drag_id)),
            resp.interact_pointer_pos(),
        ) {
            // Dragged out of the grid: gone.
            if i != 0 && i + 1 < points.len() && !rect.expand(10.0).contains(p) {
                points.remove(i);
                changed = true;
            }
        }
        ui.memory_mut(|m| m.data.remove::<usize>(drag_id));
    }
    if changed {
        curve.points = points.clone();
    }
    paint_curve(&painter, rect, curve, Stroke::new(1.6, color));
    for (x, y) in &points {
        let c = to_screen(rect, *x, *y);
        let r = Rect::from_center_size(c, Vec2::splat(6.0));
        painter.rect_filled(r, 0.0, Color32::WHITE);
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, color), egui::StrokeKind::Inside);
    }
    paint_readout(&painter, rect, resp.hover_pos(), curve);
    changed
}

/// The Freehand style: dragging draws the curve under the pointer. True
/// when it changed; the curve becomes straight segments every 4 levels.
pub fn freehand_editor(
    ui: &mut Ui,
    id: egui::Id,
    curve: &mut ToneCurve,
    size: f32,
    color: Color32,
    others: &[(&ToneCurve, Color32)],
    hist: Option<&[u32; 256]>,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
    let painter = ui.painter_at(rect.expand(4.0));
    paint_frame(&painter, rect, others, hist);
    let lut_id = id.with("freehand");
    let mut changed = false;
    if resp.drag_started() {
        let lut: Vec<f32> = (0..256).map(|x| curve.eval(x as f32)).collect();
        let start = ui
            .input(|i| i.pointer.press_origin())
            .map(|p| to_curve(rect, p));
        ui.memory_mut(|m| m.data.insert_temp(lut_id, (lut, start)));
    }
    if resp.dragged() {
        let state = ui.memory(|m| m.data.get_temp::<(Vec<f32>, Option<(f32, f32)>)>(lut_id));
        if let (Some((mut lut, prev)), Some(p)) = (state, resp.interact_pointer_pos()) {
            let (x1, y1) = to_curve(rect, p);
            let (x0, y0) = prev.unwrap_or((x1, y1));
            let (a, b) = (x0.min(x1).round() as usize, x0.max(x1).round() as usize);
            for x in a.min(255)..=b.min(255) {
                let t = if (x1 - x0).abs() < 1e-3 {
                    1.0
                } else {
                    ((x as f32 - x0) / (x1 - x0)).clamp(0.0, 1.0)
                };
                if let Some(v) = lut.get_mut(x) {
                    *v = y0 + (y1 - y0) * t;
                }
            }
            if let Ok(arr) = <[f32; 256]>::try_from(lut.as_slice()) {
                curve.points = crate::fx::adjust::freehand_points(&arr);
                curve.smooth = false;
                changed = true;
            }
            ui.memory_mut(|m| m.data.insert_temp(lut_id, (lut, Some((x1, y1)))));
        }
    }
    if resp.drag_stopped() {
        ui.memory_mut(|m| m.data.remove::<(Vec<f32>, Option<(f32, f32)>)>(lut_id));
    }
    paint_curve(&painter, rect, curve, Stroke::new(1.6, color));
    paint_readout(&painter, rect, resp.hover_pos(), curve);
    changed
}

/// The Gamma style: one handle at the middle level; dragging it up or
/// down sets the gamma. True when it changed.
pub fn gamma_editor(
    ui: &mut Ui,
    gamma: &mut f64,
    size: f32,
    color: Color32,
    others: &[(&ToneCurve, Color32)],
    hist: Option<&[u32; 256]>,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
    let painter = ui.painter_at(rect.expand(4.0));
    paint_frame(&painter, rect, others, hist);
    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let (_, y) = to_curve(rect, p);
            let y = y.clamp(1.0, 254.0) as f64;
            // y / 255 = (128 / 255)^(1 / gamma)
            let g = (128.0f64 / 255.0).ln() / (y / 255.0).ln();
            *gamma = (g * 100.0).round().clamp(10.0, 1000.0) / 100.0;
            changed = true;
        }
    }
    let curve = crate::fx::adjust::gamma_curve(*gamma);
    paint_curve(&painter, rect, &curve, Stroke::new(1.6, color));
    let h = to_screen(rect, 128.0, curve.eval(128.0));
    let r = Rect::from_center_size(h, Vec2::splat(7.0));
    painter.rect_filled(r, 0.0, Color32::WHITE);
    painter.rect_stroke(r, 0.0, Stroke::new(1.0, color), egui::StrokeKind::Inside);
    paint_readout(&painter, rect, resp.hover_pos(), &curve);
    changed
}
