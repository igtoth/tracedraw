//! The property bar's pieces, drawn like the target design's: values in
//! white fields stacked two to a column with a small icon before each,
//! spin arrows, icon buttons that stay pressed, the object origin selector,
//! and the page, object, outline and zoom bars built from them.

use crate::app::{App, Units};
use crate::i18n::tr;
use crate::theme::Tokens;
use egui::{epaint, Color32, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Affine, Point, Size},
    Command, CornerKind, Corners,
};

/// Height of the bar's content: two stacked rows of fields.
/// Two fields stacked: 19 px each, sharing their middle border.
pub const BAR_H: f32 = 37.0;
const ROW: f32 = 19.0;
/// Icon buttons are 28 px square with an 18 px icon.
const BTN: f32 = 28.0;
const HOVER_FILL: Color32 = Color32::from_rgb(0xE5, 0xF3, 0xFF);
const HOVER_EDGE: Color32 = Color32::from_rgb(0xCC, 0xE8, 0xFF);

/// The small pictures before fields and on the bar's buttons, drawn on a
/// 16 x 16 grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pic {
    Width,
    Height,
    PageWidth,
    PageHeight,
    Portrait,
    Landscape,
    AllPages,
    CurrentPage,
    Nudge,
    DupX,
    DupY,
    TreatFilled,
    Rotate,
    Lock(bool),
    MirrorH,
    MirrorV,
    Pen,
    ZoomIn,
    ZoomOut,
    ZoomSelected,
    ZoomAll,
    ZoomPage,
    ZoomWidth,
    ZoomHeight,
    Ellipse,
    Pie,
    Arc,
    ArcDirection,
    CornerRound,
    CornerScallop,
    CornerChamfer,
    /// Relative corner scaling: a small corner, a larger one, an arrow.
    CornerScaling,
    ToFront,
    ToBack,
    ToCurves,
}

fn q(r: Rect, x: f32, y: f32) -> Pos2 {
    r.min + Vec2::new(x, y) * (r.width() / 16.0)
}

pub fn draw_pic(painter: &Painter, r: Rect, pic: Pic, color: Color32) {
    let k = r.width() / 16.0;
    let s = Stroke::new(1.1 * k, color);
    let thin = Stroke::new(0.85 * k, color);
    let line = |a: (f32, f32), b: (f32, f32), st: Stroke| {
        painter.line_segment([q(r, a.0, a.1), q(r, b.0, b.1)], st);
    };
    let poly = |pts: &[(f32, f32)], fill: Color32, st: Stroke| {
        painter.add(epaint::PathShape::convex_polygon(
            pts.iter().map(|(x, y)| q(r, *x, *y)).collect(),
            fill,
            st,
        ));
    };
    let outline = |pts: &[(f32, f32)], st: Stroke| {
        painter.add(epaint::PathShape::closed_line(
            pts.iter().map(|(x, y)| q(r, *x, *y)).collect(),
            st,
        ));
    };
    let arrow_h = |y: f32, x0: f32, x1: f32| {
        line((x0, y), (x1, y), thin);
        poly(
            &[(x0, y), (x0 + 2.5, y - 2.0), (x0 + 2.5, y + 2.0)],
            color,
            Stroke::NONE,
        );
        poly(
            &[(x1, y), (x1 - 2.5, y - 2.0), (x1 - 2.5, y + 2.0)],
            color,
            Stroke::NONE,
        );
    };
    let arrow_v = |x: f32, y0: f32, y1: f32| {
        line((x, y0), (x, y1), thin);
        poly(
            &[(x, y0), (x - 2.0, y0 + 2.5), (x + 2.0, y0 + 2.5)],
            color,
            Stroke::NONE,
        );
        poly(
            &[(x, y1), (x - 2.0, y1 - 2.5), (x + 2.0, y1 - 2.5)],
            color,
            Stroke::NONE,
        );
    };
    let magnifier = |painter: &Painter| {
        painter.circle_stroke(q(r, 6.5, 6.5), 4.6 * k, s);
        painter.line_segment(
            [q(r, 9.8, 9.8), q(r, 14.5, 14.5)],
            Stroke::new(2.0 * k, color),
        );
    };
    match pic {
        Pic::Width => {
            line((2.0, 3.0), (2.0, 13.0), thin);
            line((14.0, 3.0), (14.0, 13.0), thin);
            arrow_h(8.0, 3.0, 13.0);
        }
        Pic::Height => {
            line((3.0, 2.0), (13.0, 2.0), thin);
            line((3.0, 14.0), (13.0, 14.0), thin);
            arrow_v(8.0, 3.0, 13.0);
        }
        Pic::PageWidth => {
            outline(&[(2.0, 1.5), (14.0, 1.5), (14.0, 14.5), (2.0, 14.5)], thin);
            line((2.0, 5.0), (14.0, 5.0), thin);
            arrow_h(9.5, 3.0, 13.0);
        }
        Pic::PageHeight => {
            outline(&[(2.0, 1.5), (14.0, 1.5), (14.0, 14.5), (2.0, 14.5)], thin);
            line((5.5, 1.5), (5.5, 14.5), thin);
            arrow_v(10.0, 3.0, 13.0);
        }
        Pic::Portrait => outline(&[(4.0, 1.5), (12.0, 1.5), (12.0, 14.5), (4.0, 14.5)], s),
        Pic::Landscape => outline(&[(1.5, 4.0), (14.5, 4.0), (14.5, 12.0), (1.5, 12.0)], s),
        Pic::AllPages => {
            outline(&[(1.5, 1.5), (9.5, 1.5), (9.5, 11.5), (1.5, 11.5)], s);
            poly(
                &[(6.0, 4.5), (14.5, 4.5), (14.5, 14.5), (6.0, 14.5)],
                Color32::WHITE,
                s,
            );
        }
        Pic::CurrentPage => {
            outline(&[(1.5, 1.5), (9.5, 1.5), (9.5, 14.5), (1.5, 14.5)], s);
            for x in [11.5, 13.5] {
                line((x, 5.0), (x, 14.5), Stroke::new(1.4 * k, color));
            }
        }
        Pic::Nudge => {
            arrow_h(8.0, 1.0, 15.0);
            arrow_v(8.0, 1.0, 15.0);
        }
        Pic::DupX | Pic::DupY => {
            outline(&[(1.5, 1.5), (9.0, 1.5), (9.0, 9.0), (1.5, 9.0)], thin);
            outline(&[(5.0, 5.0), (12.5, 5.0), (12.5, 12.5), (5.0, 12.5)], thin);
            painter.text(
                q(r, 14.5, 13.5),
                egui::Align2::CENTER_CENTER,
                if pic == Pic::DupX { "x" } else { "y" },
                egui::FontId::proportional(7.5 * k),
                color,
            );
        }
        Pic::TreatFilled => {
            for (a, b) in [
                ((2.0, 2.0), (14.0, 2.0)),
                ((14.0, 2.0), (14.0, 14.0)),
                ((14.0, 14.0), (2.0, 14.0)),
                ((2.0, 14.0), (2.0, 2.0)),
            ] {
                // Dashed frame.
                for i in 0..4 {
                    let t0 = i as f32 / 4.0;
                    let t1 = t0 + 0.14;
                    line(
                        (a.0 + (b.0 - a.0) * t0, a.1 + (b.1 - a.1) * t0),
                        (a.0 + (b.0 - a.0) * t1, a.1 + (b.1 - a.1) * t1),
                        s,
                    );
                }
            }
            for (x, y) in [(2.0, 2.0), (14.0, 2.0), (2.0, 14.0), (14.0, 14.0)] {
                painter.rect_filled(
                    Rect::from_center_size(q(r, x, y), Vec2::splat(2.6 * k)),
                    0.0,
                    color,
                );
            }
        }
        Pic::Rotate => {
            let mut pts = Vec::new();
            for i in 0..=14 {
                let t = std::f32::consts::PI * (0.15 + 1.5 * i as f32 / 14.0);
                pts.push(q(r, 8.0 + 5.5 * t.cos(), 8.0 - 5.5 * t.sin()));
            }
            painter.add(epaint::PathShape::line(pts, s));
            poly(
                &[(13.5, 5.0), (15.5, 9.5), (11.0, 8.5)],
                color,
                Stroke::NONE,
            );
        }
        Pic::Lock(locked) => {
            painter.rect_filled(
                Rect::from_min_max(q(r, 3.5, 7.5), q(r, 12.5, 14.5)),
                k,
                color,
            );
            let arm = if locked {
                vec![
                    q(r, 5.5, 7.5),
                    q(r, 5.5, 4.0),
                    q(r, 8.0, 2.0),
                    q(r, 10.5, 4.0),
                    q(r, 10.5, 7.5),
                ]
            } else {
                vec![
                    q(r, 5.5, 7.5),
                    q(r, 5.5, 4.0),
                    q(r, 8.0, 2.0),
                    q(r, 10.5, 3.5),
                ]
            };
            painter.add(epaint::PathShape::line(arm, Stroke::new(1.4 * k, color)));
        }
        Pic::MirrorH => {
            line((8.0, 1.0), (8.0, 15.0), Stroke::new(0.8 * k, color));
            outline(&[(1.5, 3.5), (6.0, 8.0), (1.5, 12.5)], s);
            poly(
                &[(14.5, 3.5), (10.0, 8.0), (14.5, 12.5)],
                color,
                Stroke::NONE,
            );
        }
        Pic::MirrorV => {
            line((1.0, 8.0), (15.0, 8.0), Stroke::new(0.8 * k, color));
            outline(&[(3.5, 1.5), (8.0, 6.0), (12.5, 1.5)], s);
            poly(
                &[(3.5, 14.5), (8.0, 10.0), (12.5, 14.5)],
                color,
                Stroke::NONE,
            );
        }
        Pic::Pen => {
            // A fountain pen nib.
            poly(
                &[
                    (8.0, 1.0),
                    (12.0, 7.0),
                    (10.0, 12.5),
                    (6.0, 12.5),
                    (4.0, 7.0),
                ],
                Color32::TRANSPARENT,
                s,
            );
            line((8.0, 4.0), (8.0, 9.0), thin);
            painter.circle_filled(q(r, 8.0, 9.0), 1.0 * k, color);
            line((5.5, 14.5), (10.5, 14.5), s);
        }
        Pic::ZoomIn | Pic::ZoomOut => {
            magnifier(painter);
            line((4.5, 6.5), (8.5, 6.5), s);
            if pic == Pic::ZoomIn {
                line((6.5, 4.5), (6.5, 8.5), s);
            }
        }
        Pic::ZoomSelected => {
            magnifier(painter);
            outline(&[(4.5, 4.5), (8.5, 4.5), (8.5, 8.5), (4.5, 8.5)], thin);
        }
        Pic::ZoomAll => {
            magnifier(painter);
            painter.circle_stroke(q(r, 5.5, 7.0), 1.6 * k, thin);
            painter.circle_stroke(q(r, 8.0, 6.0), 1.6 * k, thin);
        }
        Pic::ZoomPage => {
            outline(
                &[
                    (2.0, 1.5),
                    (9.0, 1.5),
                    (11.0, 3.5),
                    (11.0, 14.5),
                    (2.0, 14.5),
                ],
                thin,
            );
            painter.circle_stroke(q(r, 10.5, 10.5), 3.0 * k, s);
            line((12.7, 12.7), (15.0, 15.0), Stroke::new(1.6 * k, color));
        }
        Pic::ZoomWidth => {
            arrow_h(2.0, 1.0, 15.0);
            painter.circle_stroke(q(r, 7.5, 9.0), 3.5 * k, s);
            line((10.0, 11.5), (13.5, 15.0), Stroke::new(1.6 * k, color));
        }
        Pic::ZoomHeight => {
            arrow_v(2.0, 1.0, 15.0);
            painter.circle_stroke(q(r, 9.0, 7.5), 3.5 * k, s);
            line((11.5, 10.0), (15.0, 13.5), Stroke::new(1.6 * k, color));
        }
        Pic::Ellipse => {
            painter.circle_stroke(q(r, 8.0, 8.0), 6.5 * k, s);
        }
        Pic::Pie | Pic::Arc => {
            let mut pts = Vec::new();
            for i in 0..=20 {
                let t = std::f32::consts::FRAC_PI_2 + 1.5 * std::f32::consts::PI * i as f32 / 20.0;
                pts.push(q(r, 8.0 + 6.5 * t.cos(), 8.0 - 6.5 * t.sin()));
            }
            if pic == Pic::Pie {
                pts.push(q(r, 8.0, 8.0));
                painter.add(epaint::PathShape::closed_line(pts, s));
            } else {
                painter.add(epaint::PathShape::line(pts, s));
            }
        }
        Pic::ArcDirection => {
            let mut pts = Vec::new();
            for i in 0..=12 {
                let t = std::f32::consts::PI * (1.0 - i as f32 / 12.0);
                pts.push(q(r, 8.0 + 6.0 * t.cos(), 10.0 - 6.0 * t.sin()));
            }
            painter.add(epaint::PathShape::line(pts, s));
            poly(
                &[(12.0, 10.0), (15.5, 7.5), (15.5, 12.0)],
                color,
                Stroke::NONE,
            );
            poly(&[(4.0, 10.0), (0.5, 7.5), (0.5, 12.0)], color, Stroke::NONE);
        }
        Pic::CornerRound => {
            line((2.0, 15.0), (2.0, 8.0), s);
            line((8.0, 2.0), (15.0, 2.0), s);
            let mut pts = Vec::new();
            for i in 0..=8 {
                let t = std::f32::consts::PI * (1.0 - 0.5 * i as f32 / 8.0);
                pts.push(q(r, 8.0 + 6.0 * t.cos(), 8.0 - 6.0 * t.sin()));
            }
            painter.add(epaint::PathShape::line(pts, s));
        }
        Pic::CornerScallop => {
            line((2.0, 15.0), (2.0, 8.0), s);
            line((8.0, 2.0), (15.0, 2.0), s);
            // A quarter circle centred on the corner, bending inwards.
            let mut pts = Vec::new();
            for i in 0..=8 {
                let t = std::f32::consts::FRAC_PI_2 * (1.0 - i as f32 / 8.0);
                pts.push(q(r, 2.0 + 6.0 * t.cos(), 2.0 + 6.0 * t.sin()));
            }
            painter.add(epaint::PathShape::line(pts, s));
        }
        Pic::CornerChamfer => {
            painter.add(epaint::PathShape::line(
                vec![
                    q(r, 2.0, 15.0),
                    q(r, 2.0, 8.0),
                    q(r, 8.0, 2.0),
                    q(r, 15.0, 2.0),
                ],
                s,
            ));
        }
        Pic::CornerScaling => {
            // The corner at two sizes and an arrow from the small to the
            // large one.
            for (o, rad) in [(1.5f32, 6.0f32), (8.0, 3.5)] {
                let c = o + rad;
                let mut pts = vec![q(r, o, 15.0)];
                for i in 0..=8 {
                    let t = std::f32::consts::PI * (1.0 - 0.5 * i as f32 / 8.0);
                    pts.push(q(r, c + rad * t.cos(), c - rad * t.sin()));
                }
                pts.push(q(r, 15.0, o));
                painter.add(epaint::PathShape::line(pts, thin));
            }
            line((8.6, 8.6), (5.4, 5.4), thin);
            poly(&[(4.6, 4.6), (7.6, 5.4), (5.4, 7.6)], color, Stroke::NONE);
        }
        Pic::ToFront | Pic::ToBack => {
            let front = pic == Pic::ToFront;
            let (back, top) = if front {
                ((1.5, 1.5, 9.5, 9.5), (6.5, 6.5, 14.5, 14.5))
            } else {
                ((6.5, 6.5, 14.5, 14.5), (1.5, 1.5, 9.5, 9.5))
            };
            outline(
                &[
                    (back.0, back.1),
                    (back.2, back.1),
                    (back.2, back.3),
                    (back.0, back.3),
                ],
                thin,
            );
            poly(
                &[
                    (top.0, top.1),
                    (top.2, top.1),
                    (top.2, top.3),
                    (top.0, top.3),
                ],
                color,
                Stroke::NONE,
            );
        }
        Pic::ToCurves => {
            let mut pts = Vec::new();
            for i in 0..=12 {
                let t = i as f32 / 12.0;
                pts.push(q(
                    r,
                    1.5 + 13.0 * t,
                    12.0 - 9.0 * (t * std::f32::consts::PI).sin(),
                ));
            }
            painter.add(epaint::PathShape::line(pts, s));
            for (x, y) in [(1.5, 12.0), (8.0, 3.0), (14.5, 12.0)] {
                painter.rect_filled(
                    Rect::from_center_size(q(r, x, y), Vec2::splat(2.6 * k)),
                    0.0,
                    color,
                );
            }
        }
    }
}

fn frame(ui: &Ui, rect: Rect, hovered: bool, pressed: bool) {
    if pressed {
        ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            0.0,
            Stroke::new(1.0, Tokens::CONTROL_BORDER),
            egui::StrokeKind::Middle,
        );
    } else if hovered {
        ui.painter().rect_filled(rect, 0.0, HOVER_FILL);
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            0.0,
            Stroke::new(1.0, HOVER_EDGE),
            egui::StrokeKind::Middle,
        );
    }
}

/// An icon button of the bar; `pressed` draws it pushed in.
pub fn pic_button(ui: &mut Ui, pic: Pic, tip: &str, enabled: bool, pressed: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::splat(BTN),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    frame(ui, rect, enabled && resp.hovered(), pressed);
    draw_pic(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(18.0)),
        pic,
        if enabled {
            Tokens::ICON
        } else {
            Tokens::BORDER
        },
    );
    resp.on_hover_text(tip)
}

/// A thin separator between groups.
pub fn sep(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(9.0, BAR_H), Sense::hover());
    let x = rect.center().x.round() + 0.5;
    ui.painter().vline(
        x,
        (rect.top() + 2.0)..=(rect.bottom() - 2.0),
        Stroke::new(1.0, Color32::from_gray(0xD9)),
    );
}

/// A small label picture before a field.
fn label_pic(ui: &mut Ui, pic: Pic, tip: &str) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(16.0, ROW), Sense::hover());
    draw_pic(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(13.0)),
        pic,
        Tokens::ICON,
    );
    resp.on_hover_text(tip);
}

/// A text label before a field ("X:", "Units:").
fn label_text(ui: &mut Ui, text: &str) {
    ui.add_sized(
        [16.0, ROW],
        egui::Label::new(egui::RichText::new(text).size(12.0)),
    );
}

/// A distance field in the ruler unit; with spin arrows when `step` is
/// given (in the unit). Returns the new value in mm once committed.
fn distance(ui: &mut Ui, app: &App, mm: f64, width: f32, step: Option<f64>) -> Option<f64> {
    let u = app.units;
    let d = app.settings.precision.min(6) as usize;
    let mut v = u.from_mm(mm);
    let r = ui.add_sized(
        [width, ROW],
        crate::ui::field::NumField::new(&mut v)
            .fixed_decimals(d.min(3))
            .max_decimals(d)
            .suffix(format!(" {}", u.short()))
            .unit_mm(u.mm()),
    );
    let mut out = None;
    // Typed values apply with Enter or when the field is left.
    if r.changed() && (u.to_mm(v) - mm).abs() > 1e-12 {
        out = Some(u.to_mm(v));
    }
    if let Some(step) = step {
        if let Some(delta) = spin(ui) {
            out = Some(u.to_mm(u.from_mm(mm) + delta * step));
        }
    }
    out
}

/// Two tiny arrows side by side; returns -1 or +1 when clicked.
fn spin(ui: &mut Ui) -> Option<f64> {
    let mut out = None;
    for (dir, up) in [(-1.0, false), (1.0, true)] {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(10.0, ROW), Sense::click());
        frame(ui, rect, resp.hovered(), false);
        let c = rect.center();
        let pts = if up {
            vec![
                c + Vec2::new(-3.0, 1.5),
                c + Vec2::new(3.0, 1.5),
                c + Vec2::new(0.0, -2.0),
            ]
        } else {
            vec![
                c + Vec2::new(-3.0, -1.5),
                c + Vec2::new(3.0, -1.5),
                c + Vec2::new(0.0, 2.0),
            ]
        };
        ui.painter().add(epaint::PathShape::convex_polygon(
            pts,
            Tokens::TEXT_DIM,
            Stroke::NONE,
        ));
        if resp.clicked() {
            out = Some(dir);
        }
    }
    out
}

/// Two rows stacked in the bar's height.
fn stacked<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.allocate_ui_with_layout(
        Vec2::new(0.0, BAR_H),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            // The second field's top border is the first one's bottom.
            ui.spacing_mut().item_spacing.y = -1.0;
            ui.spacing_mut().interact_size.y = ROW;
            ui.spacing_mut().button_padding.y = 0.0;
            add(ui)
        },
    )
    .inner
}

fn row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.spacing_mut().button_padding.y = 0.0;
        ui.set_height(ROW);
        add(ui)
    })
    .inner
}

// ----- the page bar -------------------------------------------------------

/// Pick tool with nothing selected: page size, orientation, the pages it
/// applies to, units, nudge and duplicate distances, "treat as filled".
pub fn page_bar(app: &mut App, ui: &mut Ui) {
    let size = app.page_size();
    let presets = crate::ui::dialogs::paper_presets();
    let current = presets
        .iter()
        .find(|(_, s)| {
            ((s.width - size.width).abs() < 0.05 && (s.height - size.height).abs() < 0.05)
                || ((s.height - size.width).abs() < 0.05 && (s.width - size.height).abs() < 0.05)
        })
        .map(|(n, _)| n.to_string())
        .unwrap_or_else(|| tr("toolbar.custom"));
    ui.allocate_ui_with_layout(
        Vec2::new(200.0, BAR_H),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            egui::ComboBox::from_id_salt("page_size_bar")
                .selected_text(current.clone())
                .width(192.0)
                .height(400.0)
                .show_ui(ui, |ui| {
                    for (n, s) in &presets {
                        if ui.selectable_label(current == *n, *n).clicked() {
                            let landscape = size.width > size.height;
                            let s = if landscape {
                                Size::new(s.height.max(s.width), s.height.min(s.width))
                            } else {
                                Size::new(s.width.min(s.height), s.width.max(s.height))
                            };
                            resize_pages(app, s);
                            app.fit_pending = true;
                        }
                    }
                });
        },
    );
    let mut new_size = None;
    stacked(ui, |ui| {
        row(ui, |ui| {
            label_pic(ui, Pic::PageWidth, &tr("toolbar.page_width"));
            if let Some(w) = distance(ui, app, size.width, 92.0, Some(1.0)) {
                new_size = Some(Size::new(w, size.height));
            }
        });
        row(ui, |ui| {
            label_pic(ui, Pic::PageHeight, &tr("toolbar.page_height"));
            if let Some(h) = distance(ui, app, size.height, 92.0, Some(1.0)) {
                new_size = Some(Size::new(size.width, h));
            }
        });
    });
    if let Some(s) = new_size {
        if s.width > 0.1 && s.height > 0.1 {
            resize_pages(app, s);
        }
    }
    sep(ui);
    let portrait = size.height >= size.width;
    if pic_button(ui, Pic::Portrait, &tr("dialog.portrait"), true, portrait).clicked() && !portrait
    {
        resize_pages(app, Size::new(size.height, size.width));
        app.fit_pending = true;
    }
    if pic_button(ui, Pic::Landscape, &tr("dialog.landscape"), true, !portrait).clicked()
        && portrait
    {
        resize_pages(app, Size::new(size.height, size.width));
        app.fit_pending = true;
    }
    sep(ui);
    if pic_button(
        ui,
        Pic::AllPages,
        &tr("toolbar.all_pages_tip"),
        true,
        app.page_size_all,
    )
    .clicked()
    {
        app.page_size_all = true;
    }
    if pic_button(
        ui,
        Pic::CurrentPage,
        &tr("toolbar.current_page_tip"),
        true,
        !app.page_size_all,
    )
    .clicked()
    {
        app.page_size_all = false;
    }
    sep(ui);
    ui.label(tr("toolbar.units_label"));
    let mut u = app.units;
    egui::ComboBox::from_id_salt("units_bar")
        .selected_text(u.label())
        .width(120.0)
        .height(400.0)
        .show_ui(ui, |ui| {
            for v in Units::ALL {
                ui.selectable_value(&mut u, v, v.label());
            }
        });
    app.units = u;
    sep(ui);
    label_pic(ui, Pic::Nudge, &tr("toolbar.nudge_tip"));
    if let Some(n) = distance(ui, app, app.nudge_mm, 84.0, Some(0.1)) {
        if n > 0.0 {
            app.nudge_mm = n;
        }
    }
    sep(ui);
    let dup = app.duplicate_offset;
    let mut new_dup = None;
    stacked(ui, |ui| {
        row(ui, |ui| {
            label_pic(ui, Pic::DupX, &tr("toolbar.duplicate_x_tip"));
            if let Some(x) = distance(ui, app, dup.x, 92.0, Some(0.1)) {
                new_dup = Some(tracedraw_core::geometry::Vec2::new(x, dup.y));
            }
        });
        row(ui, |ui| {
            label_pic(ui, Pic::DupY, &tr("toolbar.duplicate_y_tip"));
            if let Some(y) = distance(ui, app, dup.y, 92.0, Some(0.1)) {
                new_dup = Some(tracedraw_core::geometry::Vec2::new(dup.x, y));
            }
        });
    });
    if let Some(d) = new_dup {
        app.duplicate_offset = d;
    }
    sep(ui);
    let filled = app.settings.treat_all_filled;
    if pic_button(
        ui,
        Pic::TreatFilled,
        &tr("options.treat_all_filled"),
        true,
        filled,
    )
    .clicked()
    {
        app.settings.treat_all_filled = !filled;
    }
}

/// Resize the current page, or every page when "all pages" is on, as one
/// undo step.
pub fn resize_pages(app: &mut App, size: Size) {
    let pages: Vec<_> = if app.page_size_all {
        app.doc().pages.iter().map(|p| p.id).collect()
    } else {
        vec![app.page]
    };
    let cmds: Vec<Command> = pages
        .into_iter()
        .map(|page| Command::ResizePage { page, size })
        .collect();
    if let Err(e) = app.engine.run_batch("Page Size", &cmds) {
        app.status = e.to_string();
    }
}

// ----- the object bar ---------------------------------------------------

/// The reference point the X and Y fields show: (column, row) of the 3 x 3
/// object origin selector, (1, 1) the centre.
pub fn origin_point(b: tracedraw_core::geometry::Rect, origin: (u8, u8)) -> Point {
    let x = match origin.0 {
        0 => b.x0,
        1 => b.center().x,
        _ => b.x1,
    };
    // Row 0 is the top (largest y).
    let y = match origin.1 {
        0 => b.y1,
        1 => b.center().y,
        _ => b.y0,
    };
    Point::new(x, y)
}

/// The 3 x 3 object origin selector.
fn origin_selector(ui: &mut Ui, app: &mut App) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(22.0, BAR_H), Sense::click());
    let resp = resp.on_hover_text(tr("toolbar.object_origin"));
    let cell = 5.0;
    let gap = 2.0;
    let total = cell * 3.0 + gap * 2.0;
    let o = Pos2::new(
        (rect.center().x - total / 2.0).round(),
        (rect.center().y - total / 2.0).round(),
    );
    for j in 0..3u8 {
        for i in 0..3u8 {
            let r = Rect::from_min_size(
                o + Vec2::new(i as f32 * (cell + gap), j as f32 * (cell + gap)),
                Vec2::splat(cell),
            );
            if app.object_origin == (i, j) {
                ui.painter().rect_filled(r, 0.0, Tokens::ICON);
            } else {
                ui.painter().rect_filled(r, 0.0, Color32::WHITE);
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    Stroke::new(1.0, Tokens::ICON),
                    egui::StrokeKind::Inside,
                );
            }
            if resp.clicked() {
                if let Some(p) = resp.interact_pointer_pos() {
                    if r.expand(gap / 2.0).contains(p) {
                        app.object_origin = (i, j);
                    }
                }
            }
        }
    }
}

/// The rotation of the single selected object (degrees), from its
/// transform.
fn selection_angle(app: &App) -> f64 {
    let shapes = app.selected_shapes();
    if shapes.len() != 1 {
        return 0.0;
    }
    let c = shapes[0].transform.as_coeffs();
    let a = c[1].atan2(c[0]).to_degrees();
    if a.abs() < 1e-9 {
        0.0
    } else {
        a
    }
}

/// Position, size, scale, rotation and mirroring of the selection.
pub fn object_bar(app: &mut App, ui: &mut Ui) {
    let Some(b) = app.selection_bounds() else {
        return;
    };
    origin_selector(ui, app);
    let o = app.ruler_origin();
    let refp = origin_point(b, app.object_origin);
    let mut moved: Option<Point> = None;
    stacked(ui, |ui| {
        row(ui, |ui| {
            label_text(ui, "X:");
            if let Some(x) = distance(ui, app, refp.x - o.x, 100.0, None) {
                moved = Some(Point::new(x + o.x, refp.y));
            }
        });
        row(ui, |ui| {
            label_text(ui, "Y:");
            if let Some(y) = distance(ui, app, refp.y - o.y, 100.0, None) {
                moved = Some(Point::new(refp.x, y + o.y));
            }
        });
    });
    if let Some(p) = moved {
        app.transform_selection(Affine::translate(p - refp));
    }
    // Size, about the reference point; the lock keeps proportions.
    let (w, h) = (b.width(), b.height());
    let mut size: Option<(f64, f64)> = None;
    let locked = app.scale_locked;
    stacked(ui, |ui| {
        row(ui, |ui| {
            label_pic(ui, Pic::Width, &tr("toolbar.object_width"));
            if let Some(nw) = distance(ui, app, w, 92.0, None) {
                let nh = if locked && w > 1e-9 { h * nw / w } else { h };
                size = Some((nw, nh));
            }
        });
        row(ui, |ui| {
            label_pic(ui, Pic::Height, &tr("toolbar.object_height"));
            if let Some(nh) = distance(ui, app, h, 92.0, None) {
                let nw = if locked && h > 1e-9 { w * nh / h } else { w };
                size = Some((nw, nh));
            }
        });
    });
    // Scale factors (relative: the field shows 100% and applies the change).
    let mut scale: Option<(f64, f64)> = None;
    stacked(ui, |ui| {
        for horizontal in [true, false] {
            row(ui, |ui| {
                let mut pct = 100.0f64;
                let r = ui.add_sized(
                    [64.0, ROW],
                    crate::ui::field::NumField::new(&mut pct)
                        .speed(0.5)
                        .fixed_decimals(1)
                        .suffix(" %"),
                );
                if (r.lost_focus() || r.drag_stopped()) && (pct - 100.0).abs() > 1e-9 && pct > 0.0 {
                    let f = pct / 100.0;
                    scale = Some(if locked {
                        (f, f)
                    } else if horizontal {
                        (f, 1.0)
                    } else {
                        (1.0, f)
                    });
                }
            });
        }
    });
    if pic_button(
        ui,
        Pic::Lock(locked),
        &tr("toolbar.lock_ratio"),
        true,
        locked,
    )
    .clicked()
    {
        app.scale_locked = !locked;
    }
    if let Some((nw, nh)) = size {
        if nw > 1e-6 && nh > 1e-6 && w > 1e-9 && h > 1e-9 {
            scale = Some((nw / w, nh / h));
        }
    }
    if let Some((sx, sy)) = scale {
        app.transform_selection(
            Affine::translate(refp.to_vec2())
                * Affine::scale_non_uniform(sx, sy)
                * Affine::translate(-refp.to_vec2()),
        );
    }
    sep(ui);
    // Rotation angle: the object's own, about the reference point.
    let angle = selection_angle(app);
    let mut a = angle;
    label_pic(ui, Pic::Rotate, &tr("toolbar.rotation_angle"));
    let r = ui.add_sized(
        [64.0, ROW + 4.0],
        crate::ui::field::NumField::new(&mut a)
            .speed(1.0)
            .fixed_decimals(1)
            .suffix(" \u{00B0}"),
    );
    if (r.lost_focus() || r.drag_stopped()) && (a - angle).abs() > 1e-9 {
        let c = refp;
        app.transform_selection(
            Affine::translate(c.to_vec2())
                * Affine::rotate((a - angle).to_radians())
                * Affine::translate(-c.to_vec2()),
        );
    }
    sep(ui);
    if pic_button(ui, Pic::MirrorH, &tr("toolbar.mirror_h"), true, false).clicked() {
        app.mirror(true);
    }
    if pic_button(ui, Pic::MirrorV, &tr("toolbar.mirror_v"), true, false).clicked() {
        app.mirror(false);
    }
}

/// Outline width, as the target design lists them.
const OUTLINE_WIDTHS_MM: [f64; 10] = [0.1, 0.2, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 2.5, 3.0];

/// Text for an outline width in the ruler unit ("Hairline", "None").
pub fn outline_text(app: &App, width: Option<f64>) -> String {
    match width {
        None => tr("toolbar.none"),
        Some(w) if w <= tracedraw_core::Stroke::HAIRLINE + 1e-9 => tr("toolbar.hairline"),
        Some(w) => {
            let u = app.units;
            let d = if u == Units::Pixels { 2 } else { 3 };
            format!("{:.*} {}", d, u.from_mm(w), u.short())
        }
    }
}

/// Outline width and the line style and arrowheads of the selection's
/// outline.
pub fn outline_part(app: &mut App, ui: &mut Ui) {
    let first = app.selected_shapes().into_iter().next();
    let stroke = first.as_ref().and_then(|s| s.stroke.clone());
    label_pic(ui, Pic::Pen, &tr("toolbar.outline_width"));
    egui::ComboBox::from_id_salt("outline_width_bar")
        .selected_text(outline_text(app, stroke.as_ref().map(|s| s.width)))
        .width(88.0)
        .height(400.0)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(stroke.is_none(), tr("toolbar.none"))
                .clicked()
            {
                let shapes = app.selection.clone();
                app.run(Command::SetStroke {
                    shapes,
                    stroke: None,
                });
            }
            if ui.selectable_label(false, tr("toolbar.hairline")).clicked() {
                app.apply_outline_width(tracedraw_core::Stroke::HAIRLINE);
            }
            for w in OUTLINE_WIDTHS_MM {
                let text = outline_text(app, Some(w));
                if ui.selectable_label(false, text).clicked() {
                    app.apply_outline_width(w);
                }
            }
        });
    // Line style: solid or one of the dash presets.
    let dash = stroke.as_ref().map(|s| s.dash.clone()).unwrap_or_default();
    egui::ComboBox::from_id_salt("line_style_bar")
        .selected_text(dash_label(&dash))
        .width(96.0)
        .show_ui(ui, |ui| {
            for d in DASHES {
                if ui
                    .selectable_label(dash.as_slice() == *d, dash_label(d))
                    .clicked()
                {
                    let cmds: Vec<Command> = app
                        .selected_shapes()
                        .iter()
                        .filter_map(|s| {
                            let mut st = s.stroke.clone()?;
                            st.dash = d.to_vec();
                            Some(Command::SetStroke {
                                shapes: vec![s.id],
                                stroke: Some(st),
                            })
                        })
                        .collect();
                    if let Err(e) = app.engine.run_batch("Line Style", &cmds) {
                        app.status = e.to_string();
                    }
                }
            }
        });
    // Arrowheads at the start and the end.
    for start in [true, false] {
        let current = stroke
            .as_ref()
            .map(|s| {
                if start {
                    s.start_arrow.clone()
                } else {
                    s.end_arrow.clone()
                }
            })
            .unwrap_or_default();
        egui::ComboBox::from_id_salt(if start {
            "arrow_start_bar"
        } else {
            "arrow_end_bar"
        })
        .selected_text(arrow_label(&current))
        .width(56.0)
        .show_ui(ui, |ui| {
            for a in tracedraw_core::Arrowhead::ALL {
                if ui.selectable_label(current == a, arrow_label(&a)).clicked() {
                    let cmds: Vec<Command> = app
                        .selected_shapes()
                        .iter()
                        .filter_map(|s| {
                            let mut st = s.stroke.clone()?;
                            if start {
                                st.start_arrow = a.clone();
                            } else {
                                st.end_arrow = a.clone();
                            }
                            Some(Command::SetStroke {
                                shapes: vec![s.id],
                                stroke: Some(st),
                            })
                        })
                        .collect();
                    if let Err(e) = app.engine.run_batch("Arrowheads", &cmds) {
                        app.status = e.to_string();
                    }
                }
            }
        });
    }
}

/// Dash presets (multiples of the outline width); the first is solid.
const DASHES: &[&[f64]] = &[
    &[],
    &[4.0, 2.0],
    &[1.0, 2.0],
    &[8.0, 3.0],
    &[6.0, 2.0, 1.0, 2.0],
    &[6.0, 2.0, 1.0, 2.0, 1.0, 2.0],
];

fn dash_label(d: &[f64]) -> String {
    if d.is_empty() {
        return "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".into();
    }
    // A few characters drawing the pattern.
    let mut s = String::new();
    for (i, v) in d.iter().cycle().take(d.len() * 3).enumerate() {
        let n = (*v / 2.0).round().clamp(1.0, 4.0) as usize;
        let c = if i % 2 == 0 { '\u{2500}' } else { ' ' };
        for _ in 0..n {
            s.push(c);
        }
        if s.chars().count() > 12 {
            break;
        }
    }
    s
}

fn arrow_label(a: &tracedraw_core::Arrowhead) -> String {
    use tracedraw_core::Arrowhead as A;
    match a {
        A::None => "\u{2500}".into(),
        A::Arrow => "\u{2500}\u{25B6}".into(),
        A::OpenArrow => "\u{2500}>".into(),
        A::Circle => "\u{2500}\u{25CF}".into(),
        A::Square => "\u{2500}\u{25A0}".into(),
        A::Bar => "\u{2500}|".into(),
        A::Diamond => "\u{2500}\u{25C6}".into(),
        A::Custom { name, .. } => name.clone(),
    }
}

// ----- shape parts ------------------------------------------------------

/// Ellipse, pie or arc, the start and end angles and the direction, for
/// new ellipses and the selected ones.
pub fn ellipse_part(app: &mut App, ui: &mut Ui) {
    // The selection's arc when an ellipse is selected, else the tool's.
    let selected: Vec<(tracedraw_core::ShapeId, tracedraw_core::geometry::Rect)> = app
        .selected_shapes()
        .iter()
        .filter_map(|s| match &s.kind {
            ShapeKind::Ellipse { rect, .. } => Some((s.id, *rect)),
            _ => None,
        })
        .collect();
    let current = app
        .selected_shapes()
        .iter()
        .find_map(|s| match &s.kind {
            ShapeKind::Ellipse { arc, .. } => Some(*arc),
            _ => None,
        })
        .unwrap_or(app.ellipse_arc);
    let mode = match current {
        None => 0,
        Some(a) if a.pie => 1,
        Some(_) => 2,
    };
    let mut arc = current.unwrap_or(tracedraw_core::EllipseArc {
        start_deg: 90.0,
        end_deg: 90.0,
        pie: true,
    });
    let mut new = current;
    for (i, pic, key) in [
        (0, Pic::Ellipse, "toolbar.ellipse_tip"),
        (1, Pic::Pie, "toolbar.pie_tip"),
        (2, Pic::Arc, "toolbar.arc_tip"),
    ] {
        if pic_button(ui, pic, &tr(key), true, mode == i).clicked() && mode != i {
            new = match i {
                0 => None,
                k => {
                    arc.pie = k == 1;
                    Some(arc)
                }
            };
        }
    }
    let angles_on = mode != 0;
    stacked(ui, |ui| {
        for start in [true, false] {
            row(ui, |ui| {
                let mut v = if start { arc.start_deg } else { arc.end_deg };
                let r = ui.add_enabled(
                    angles_on,
                    crate::ui::field::NumField::new(&mut v)
                        .speed(1.0)
                        .fixed_decimals(1)
                        .suffix(" \u{00B0}"),
                );
                if angles_on && (r.lost_focus() || r.drag_stopped()) {
                    if start {
                        arc.start_deg = v;
                    } else {
                        arc.end_deg = v;
                    }
                    new = Some(arc);
                }
            });
        }
    });
    if pic_button(
        ui,
        Pic::ArcDirection,
        &tr("toolbar.arc_direction"),
        angles_on,
        false,
    )
    .clicked()
    {
        std::mem::swap(&mut arc.start_deg, &mut arc.end_deg);
        new = Some(arc);
    }
    if new != current {
        app.ellipse_arc = new;
        let cmds: Vec<Command> = selected
            .iter()
            .map(|(id, rect)| Command::SetShapeKind {
                shape: *id,
                kind: ShapeKind::Ellipse {
                    rect: *rect,
                    arc: new,
                },
            })
            .collect();
        if !cmds.is_empty() {
            if let Err(e) = app.engine.run_batch("Ellipse", &cmds) {
                app.status = e.to_string();
            }
        }
    }
}

/// Points and sharpness of polygons and stars (sharpness 1 to 99, as
/// the target design counts it).
pub fn polygon_part(app: &mut App, ui: &mut Ui, star: bool) {
    let selected: Vec<(
        tracedraw_core::ShapeId,
        tracedraw_core::geometry::Rect,
        u32,
        f64,
    )> = app
        .selected_shapes()
        .iter()
        .filter_map(|s| match &s.kind {
            ShapeKind::Polygon {
                rect,
                points,
                sharpness,
            } => Some((s.id, *rect, *points, *sharpness)),
            _ => None,
        })
        .collect();
    let (mut n, mut sharp) = selected
        .first()
        .map(|(_, _, n, s)| (*n, *s))
        .unwrap_or((app.polygon_points, app.star_sharpness));
    let star = star || selected.first().is_some_and(|s| s.3 > 0.0);
    let (n0, s0) = (n, sharp);
    ui.label(tr("toolbar.points_sides"));
    ui.add_sized(
        [56.0, ROW + 4.0],
        crate::ui::field::NumField::new(&mut n).range(3..=500),
    );
    if star {
        ui.label(tr("toolbar.sharpness"));
        let mut pct = (sharp * 100.0).round();
        if ui
            .add_sized(
                [56.0, ROW + 4.0],
                crate::ui::field::NumField::new(&mut pct).range(1.0..=99.0),
            )
            .changed()
        {
            sharp = (pct / 100.0).clamp(0.01, 0.99);
        }
    }
    if n != n0 || (sharp - s0).abs() > 1e-9 {
        app.polygon_points = n;
        app.star_sharpness = sharp;
        let cmds: Vec<Command> = selected
            .iter()
            .map(|(id, rect, _, old)| Command::SetShapeKind {
                shape: *id,
                kind: ShapeKind::Polygon {
                    rect: *rect,
                    points: n,
                    sharpness: if *old > 0.0 || star { sharp } else { 0.0 },
                },
            })
            .collect();
        if !cmds.is_empty() {
            if let Err(e) = app.engine.run_batch("Polygon", &cmds) {
                app.status = e.to_string();
            }
        }
    }
}

/// A change the rectangle part of the bar makes to corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CornerEdit {
    Kind(CornerKind),
    /// One corner's size (page mm); with the lock every corner gets it.
    Size(usize, f64),
    /// Relative corner scaling on or off.
    Scaling(bool),
}

impl CornerEdit {
    /// `c` (measured on the page) after the edit.
    pub fn apply(self, mut c: Corners, together: bool) -> Corners {
        match self {
            CornerEdit::Kind(k) => c.kind = k,
            CornerEdit::Size(i, v) => {
                let v = if v.is_finite() { v.max(0.0) } else { 0.0 };
                if together {
                    c.radii = [v; 4];
                } else if let Some(r) = c.radii.get_mut(i) {
                    *r = v;
                }
            }
            CornerEdit::Scaling(relative) => c.fixed = !relative,
        }
        c
    }
}

/// Apply a corner edit to the selected rectangles, or to the defaults for
/// new ones when none is selected.
pub fn edit_corners(app: &mut App, edit: CornerEdit) {
    let together = app.corners_together;
    let cmds: Vec<Command> = app
        .selected_shapes()
        .iter()
        .filter_map(|s| {
            let kind = s.with_page_corners(edit.apply(s.page_corners()?, together))?;
            (kind != s.kind).then_some(Command::SetShapeKind { shape: s.id, kind })
        })
        .collect();
    let any_rect = app
        .selected_shapes()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Rect { .. }));
    if !any_rect {
        app.rect_corners = edit.apply(app.rect_corners, together);
        return;
    }
    if cmds.is_empty() {
        return;
    }
    let label = match edit {
        CornerEdit::Kind(_) => "Corner Style",
        CornerEdit::Size(..) => "Corner Radius",
        CornerEdit::Scaling(_) => "Corner Scaling",
    };
    if let Err(e) = app.engine.run_batch(label, &cmds) {
        app.status = e.to_string();
    }
}

/// Rectangle corners: the corner style, the four sizes (top left over
/// bottom left, then top right over bottom right) with the lock that edits
/// them together, and relative corner scaling.
pub fn rectangle_part(app: &mut App, ui: &mut Ui) {
    let current = app
        .selected_shapes()
        .iter()
        .find_map(|s| s.page_corners())
        .unwrap_or(app.rect_corners);
    let mut edit = None;
    for (kind, pic, tip) in [
        (CornerKind::Round, Pic::CornerRound, "toolbar.corner_round"),
        (
            CornerKind::Scallop,
            Pic::CornerScallop,
            "toolbar.corner_scallop",
        ),
        (
            CornerKind::Chamfer,
            Pic::CornerChamfer,
            "toolbar.corner_chamfer",
        ),
    ] {
        if pic_button(ui, pic, &tr(tip), true, current.kind == kind).clicked() {
            edit = Some(CornerEdit::Kind(kind));
        }
    }
    sep(ui);
    for column in [
        [Corners::TOP_LEFT, Corners::BOTTOM_LEFT],
        [Corners::TOP_RIGHT, Corners::BOTTOM_RIGHT],
    ] {
        stacked(ui, |ui| {
            for i in column {
                row(ui, |ui| {
                    if let Some(v) = distance(ui, app, current.radii[i], 66.0, Some(1.0)) {
                        edit = Some(CornerEdit::Size(i, v));
                    }
                });
            }
        });
    }
    let together = app.corners_together;
    if pic_button(
        ui,
        Pic::Lock(together),
        &tr("toolbar.corners_together"),
        true,
        together,
    )
    .clicked()
    {
        app.corners_together = !together;
    }
    sep(ui);
    let relative = !current.fixed;
    if pic_button(
        ui,
        Pic::CornerScaling,
        &tr("toolbar.relative_corners"),
        true,
        relative,
    )
    .clicked()
    {
        edit = Some(CornerEdit::Scaling(!relative));
    }
    if let Some(e) = edit {
        edit_corners(app, e);
    }
}

/// The part that follows the object bar for the selected kind of object
/// (rectangle corners, ellipse arcs, polygon points); true when one was
/// drawn.
pub fn kind_part(app: &mut App, ui: &mut Ui) -> bool {
    let kinds: Vec<u8> = app
        .selected_shapes()
        .iter()
        .map(|s| match s.kind {
            ShapeKind::Rect { .. } => 1,
            ShapeKind::Ellipse { .. } => 2,
            ShapeKind::Polygon { .. } => 3,
            _ => 0,
        })
        .collect();
    let Some(first) = kinds.first().copied() else {
        return false;
    };
    if first == 0 || kinds.iter().any(|k| *k != first) {
        return false;
    }
    match first {
        1 => rectangle_part(app, ui),
        2 => ellipse_part(app, ui),
        _ => polygon_part(app, ui, false),
    }
    true
}

// ----- the Zoom tool's bar ---------------------------------------------

pub fn zoom_bar(app: &mut App, ui: &mut Ui) {
    crate::ui::toolbar::zoom_box_pub(app, ui, "bar_zoom", 107.0);
    for (pic, key, f) in [
        (Pic::ZoomIn, "menu.view.zoom_in", 0),
        (Pic::ZoomOut, "menu.view.zoom_out", 1),
    ] {
        if pic_button(ui, pic, &tr(key), true, false).clicked() {
            app.zoom_step(f == 0);
        }
    }
    sep(ui);
    let has = !app.selection.is_empty();
    if pic_button(
        ui,
        Pic::ZoomSelected,
        &tr("toolbar.zoom_to_selected"),
        has,
        false,
    )
    .clicked()
    {
        app.zoom_to_selection();
    }
    if pic_button(ui, Pic::ZoomAll, &tr("toolbar.zoom_to_fit"), true, false).clicked() {
        app.zoom_to_fit();
    }
    sep(ui);
    if pic_button(ui, Pic::ZoomPage, &tr("toolbar.zoom_to_page"), true, false).clicked() {
        app.zoom_to_page();
    }
    if pic_button(
        ui,
        Pic::ZoomWidth,
        &tr("toolbar.zoom_to_width"),
        true,
        false,
    )
    .clicked()
    {
        app.zoom_to_page_width();
    }
    if pic_button(
        ui,
        Pic::ZoomHeight,
        &tr("toolbar.zoom_to_height"),
        true,
        false,
    )
    .clicked()
    {
        app.zoom_to_page_height();
    }
}

/// Order and conversion buttons after an object's properties.
pub fn order_part(app: &mut App, ui: &mut Ui) {
    if pic_button(ui, Pic::ToFront, &tr("toolbar.to_front"), true, false).clicked() {
        app.order(0);
    }
    if pic_button(ui, Pic::ToBack, &tr("toolbar.to_back"), true, false).clicked() {
        app.order(3);
    }
    let curves_ok = app
        .selected_shapes()
        .iter()
        .any(|s| !matches!(s.kind, ShapeKind::Bitmap { .. } | ShapeKind::Path { .. }));
    if pic_button(
        ui,
        Pic::ToCurves,
        &format!("{} (Ctrl+Q)", tr("menu.object.convert_to_curves")),
        curves_ok,
        false,
    )
    .clicked()
    {
        app.convert_to_curves();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect as R;

    #[test]
    fn the_object_origin_picks_corners_edges_and_centre() {
        let b = R::new(10.0, 20.0, 30.0, 60.0);
        assert_eq!(origin_point(b, (1, 1)), Point::new(20.0, 40.0));
        assert_eq!(origin_point(b, (0, 0)), Point::new(10.0, 60.0));
        assert_eq!(origin_point(b, (2, 2)), Point::new(30.0, 20.0));
        assert_eq!(origin_point(b, (1, 2)), Point::new(20.0, 20.0));
    }

    #[test]
    fn page_size_changes_one_page_or_all_in_one_step() {
        let mut app = App::headless();
        app.add_page();
        let depth = app.engine.history_labels().0.len();
        app.page_size_all = true;
        resize_pages(&mut app, Size::new(100.0, 50.0));
        assert!(app
            .doc()
            .pages
            .iter()
            .all(|p| p.size == Size::new(100.0, 50.0)));
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        app.page_size_all = false;
        resize_pages(&mut app, Size::new(70.0, 70.0));
        let sizes: Vec<Size> = app.doc().pages.iter().map(|p| p.size).collect();
        assert_eq!(
            sizes
                .iter()
                .filter(|s| **s == Size::new(70.0, 70.0))
                .count(),
            1
        );
    }

    #[test]
    fn outline_widths_read_in_the_ruler_unit() {
        let mut app = App::headless();
        app.units = Units::Millimeters;
        assert_eq!(outline_text(&app, Some(0.2)), "0.200 mm");
        assert_eq!(outline_text(&app, None), tr("toolbar.none"));
        assert_eq!(
            outline_text(&app, Some(tracedraw_core::Stroke::HAIRLINE)),
            tr("toolbar.hairline")
        );
    }

    #[test]
    fn corner_edits_change_the_selection_in_one_step_or_the_defaults() {
        let mut app = App::headless();
        // Nothing selected: the defaults for new rectangles.
        edit_corners(&mut app, CornerEdit::Size(Corners::TOP_LEFT, 3.0));
        assert_eq!(app.rect_corners.radii, [3.0; 4]);
        edit_corners(&mut app, CornerEdit::Kind(CornerKind::Chamfer));
        assert_eq!(app.rect_corners.kind, CornerKind::Chamfer);
        app.rect_corners = Corners::default();
        let id = app
            .new_shape(ShapeKind::rect_with_corners(
                R::new(0.0, 0.0, 40.0, 20.0),
                Corners::default(),
            ))
            .expect("a rectangle");
        app.select(vec![id]);
        app.transform_selection(Affine::scale(2.0));
        let depth = app.engine.history_labels().0.len();
        // Together: every corner; the sizes are page millimetres.
        edit_corners(&mut app, CornerEdit::Size(Corners::TOP_RIGHT, 6.0));
        let s = app.selected_shapes()[0].clone();
        assert_eq!(s.page_corners().unwrap().radii, [6.0; 4]);
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        assert_eq!(app.engine.undo_label(), Some("Corner Radius"));
        // Unlocked: one corner.
        app.corners_together = false;
        edit_corners(&mut app, CornerEdit::Size(Corners::BOTTOM_LEFT, 1.0));
        let s = app.selected_shapes()[0].clone();
        assert_eq!(s.page_corners().unwrap().radii, [6.0, 6.0, 1.0, 6.0]);
        // Turning relative scaling off keeps the sizes on the page.
        edit_corners(&mut app, CornerEdit::Scaling(false));
        let s = app.selected_shapes()[0].clone();
        let c = s.page_corners().unwrap();
        assert!(c.fixed);
        assert_eq!(c.radii, [6.0, 6.0, 1.0, 6.0]);
        // ... and scaling the rectangle again leaves them alone.
        app.transform_selection(Affine::scale(0.5));
        let s = app.selected_shapes()[0].clone();
        assert_eq!(s.page_corners().unwrap().radii, [6.0, 6.0, 1.0, 6.0]);
        assert_eq!(app.rect_corners, Corners::default());
        // Undo goes back one edit at a time.
        app.undo();
        app.undo();
        let s = app.selected_shapes()[0].clone();
        assert!(!s.page_corners().unwrap().fixed);
    }

    #[test]
    fn the_rectangle_tool_draws_with_the_default_corners() {
        let mut app = App::headless();
        app.rect_corners = Corners::uniform(2.0, CornerKind::Scallop);
        app.tool = crate::tools::Tool::Rectangle;
        app.create_box_shape(Point::new(10.0, 10.0), Point::new(50.0, 30.0));
        let s = app.selected_shapes()[0].clone();
        let (_, c) = s.kind.rect_corners().unwrap();
        assert_eq!(c, Corners::uniform(2.0, CornerKind::Scallop));
    }
}
