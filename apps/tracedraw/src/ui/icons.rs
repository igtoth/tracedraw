//! Toolbox icons drawn with the painter, 16x16 design grid, no bitmaps.

use crate::theme::Tokens;
use crate::tools::Tool;
use egui::{epaint, Color32, Painter, Pos2, Rect, Stroke, Vec2};

fn p(r: Rect, x: f32, y: f32) -> Pos2 {
    r.min + Vec2::new(x, y) * (r.width() / 16.0)
}

pub fn draw(painter: &Painter, r: Rect, tool: Tool, color: Color32) {
    let s = Stroke::new(1.4, color);
    let thin = Stroke::new(1.0, color);
    match tool {
        Tool::Pick | Tool::FreeformPick => {
            let pts = vec![
                p(r, 4.0, 2.0),
                p(r, 4.0, 13.0),
                p(r, 7.0, 10.5),
                p(r, 9.0, 14.0),
                p(r, 11.0, 13.0),
                p(r, 9.0, 9.5),
                p(r, 12.5, 9.5),
            ];
            painter.add(epaint::PathShape::convex_polygon(pts, color, Stroke::NONE));
        }
        Tool::Shape | Tool::Smooth | Tool::Smear | Tool::Twirl => {
            let pts = vec![
                p(r, 4.0, 2.0),
                p(r, 4.0, 13.0),
                p(r, 7.0, 10.5),
                p(r, 9.0, 14.0),
                p(r, 11.0, 13.0),
                p(r, 9.0, 9.5),
                p(r, 12.5, 9.5),
            ];
            painter.add(epaint::PathShape::closed_line(pts, s));
            painter.rect_filled(
                Rect::from_center_size(p(r, 12.5, 3.5), Vec2::splat(r.width() / 16.0 * 4.0)),
                0.0,
                color,
            );
        }
        Tool::Crop | Tool::Knife | Tool::Eraser => {
            painter.line_segment([p(r, 5.0, 2.0), p(r, 5.0, 11.0)], s);
            painter.line_segment([p(r, 5.0, 11.0), p(r, 14.0, 11.0)], s);
            painter.line_segment([p(r, 2.0, 5.0), p(r, 11.0, 5.0)], s);
            painter.line_segment([p(r, 11.0, 5.0), p(r, 11.0, 14.0)], s);
        }
        Tool::Zoom | Tool::Pan => {
            painter.circle_stroke(p(r, 7.0, 7.0), r.width() / 16.0 * 4.5, s);
            painter.line_segment(
                [p(r, 10.3, 10.3), p(r, 14.5, 14.5)],
                Stroke::new(2.2, color),
            );
            painter.line_segment([p(r, 4.5, 7.0), p(r, 9.5, 7.0)], thin);
            painter.line_segment([p(r, 7.0, 4.5), p(r, 7.0, 9.5)], thin);
        }
        Tool::Freehand
        | Tool::TwoPointLine
        | Tool::Bezier
        | Tool::Pen
        | Tool::BSpline
        | Tool::Polyline
        | Tool::ThreePointCurve => {
            let mut pts = Vec::new();
            for i in 0..=16 {
                let t = i as f32 / 16.0;
                let x = 2.0 + 12.0 * t;
                let y = 8.0 + 4.5 * (t * std::f32::consts::TAU).sin() * (1.0 - t * 0.3);
                pts.push(p(r, x, y));
            }
            painter.add(epaint::PathShape::line(pts, s));
        }
        Tool::BrushStrokes => {
            let mut pts = Vec::new();
            for i in 0..=16 {
                let t = i as f32 / 16.0;
                pts.push(p(r, 2.0 + 12.0 * t, 11.0 - 7.0 * t + 2.5 * (t * 6.0).sin()));
            }
            painter.add(epaint::PathShape::line(pts, Stroke::new(3.0, color)));
        }
        Tool::Rectangle | Tool::ThreePointRectangle => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 2.5, 4.0), p(r, 13.5, 12.0)),
                0.0,
                s,
                epaint::StrokeKind::Middle,
            );
        }
        Tool::Ellipse | Tool::ThreePointEllipse => {
            let c = p(r, 8.0, 8.0);
            let (rx, ry) = (r.width() / 16.0 * 5.5, r.width() / 16.0 * 4.0);
            let pts: Vec<Pos2> = (0..40)
                .map(|i| {
                    let a = i as f32 / 40.0 * std::f32::consts::TAU;
                    Pos2::new(c.x + rx * a.cos(), c.y + ry * a.sin())
                })
                .collect();
            painter.add(epaint::PathShape::closed_line(pts, s));
        }
        Tool::Polygon | Tool::Star | Tool::Spiral | Tool::CommonShapes => {
            let c = p(r, 8.0, 8.5);
            let rr = r.width() / 16.0 * 6.0;
            let pts: Vec<Pos2> = (0..5)
                .map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / 5.0;
                    Pos2::new(c.x + rr * a.cos(), c.y + rr * a.sin())
                })
                .collect();
            painter.add(epaint::PathShape::closed_line(pts, s));
        }
        Tool::Text | Tool::Table => {
            painter.text(
                p(r, 8.0, 8.5),
                egui::Align2::CENTER_CENTER,
                "A",
                egui::FontId::proportional(r.width() * 0.85),
                color,
            );
        }
        Tool::ParallelDimension => {
            painter.line_segment([p(r, 2.0, 11.0), p(r, 14.0, 11.0)], s);
            painter.line_segment([p(r, 2.0, 8.0), p(r, 2.0, 14.0)], thin);
            painter.line_segment([p(r, 14.0, 8.0), p(r, 14.0, 14.0)], thin);
            painter.line_segment([p(r, 2.0, 11.0), p(r, 4.5, 9.5)], thin);
            painter.line_segment([p(r, 14.0, 11.0), p(r, 11.5, 9.5)], thin);
        }
        Tool::Connector => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 1.5, 1.5), p(r, 6.5, 6.5)),
                0.0,
                thin,
                epaint::StrokeKind::Middle,
            );
            painter.rect_stroke(
                Rect::from_min_max(p(r, 9.5, 9.5), p(r, 14.5, 14.5)),
                0.0,
                thin,
                epaint::StrokeKind::Middle,
            );
            painter.line_segment([p(r, 6.5, 4.0), p(r, 12.0, 4.0)], s);
            painter.line_segment([p(r, 12.0, 4.0), p(r, 12.0, 9.5)], s);
        }
        Tool::DropShadow
        | Tool::Contour
        | Tool::Blend
        | Tool::Distort
        | Tool::Envelope
        | Tool::Extrude => {
            painter.rect_filled(
                Rect::from_min_max(p(r, 5.0, 5.0), p(r, 14.0, 14.0)),
                0.0,
                color.gamma_multiply(0.35),
            );
            painter.rect_stroke(
                Rect::from_min_max(p(r, 2.5, 2.5), p(r, 11.5, 11.5)),
                0.0,
                s,
                epaint::StrokeKind::Middle,
            );
        }
        Tool::Transparency => {
            painter.rect_filled(
                Rect::from_min_max(p(r, 2.0, 4.0), p(r, 9.0, 12.0)),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_min_max(p(r, 7.0, 4.0), p(r, 14.0, 12.0)),
                0.0,
                color.gamma_multiply(0.4),
            );
        }
        Tool::ColorEyedropper | Tool::AttributesEyedropper => {
            painter.line_segment([p(r, 3.0, 13.0), p(r, 10.0, 6.0)], Stroke::new(2.2, color));
            painter.circle_filled(p(r, 11.5, 4.5), r.width() / 16.0 * 2.2, color);
            painter.circle_filled(p(r, 3.0, 13.0), r.width() / 16.0 * 1.2, color);
        }
        Tool::InteractiveFill | Tool::MeshFill | Tool::AreaFill => {
            let pts = vec![
                p(r, 3.0, 8.0),
                p(r, 8.0, 3.0),
                p(r, 13.0, 8.0),
                p(r, 8.0, 13.0),
            ];
            painter.add(epaint::PathShape::convex_polygon(pts, color, Stroke::NONE));
            painter.circle_filled(p(r, 13.0, 12.5), r.width() / 16.0 * 1.8, Tokens::ACCENT);
        }
    }
}

/// Standard toolbar actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    New,
    Open,
    Save,
    Print,
    Cut,
    Copy,
    Paste,
    Undo,
    Redo,
    Import,
    Export,
    Pdf,
    Snap,
    Options,
    Welcome,
    Mirror,
    Flip,
}

pub fn draw_action(painter: &Painter, r: Rect, a: Action, color: Color32) {
    let s = Stroke::new(1.3, color);
    let thin = Stroke::new(1.0, color);
    let page = |painter: &Painter, x0: f32, y0: f32, x1: f32, y1: f32| {
        painter.add(epaint::PathShape::closed_line(
            vec![
                p(r, x0, y0),
                p(r, x1 - 3.0, y0),
                p(r, x1, y0 + 3.0),
                p(r, x1, y1),
                p(r, x0, y1),
            ],
            s,
        ));
        painter.line_segment([p(r, x1 - 3.0, y0), p(r, x1 - 3.0, y0 + 3.0)], thin);
        painter.line_segment([p(r, x1 - 3.0, y0 + 3.0), p(r, x1, y0 + 3.0)], thin);
    };
    match a {
        Action::New => page(painter, 3.5, 1.5, 12.5, 14.5),
        Action::Open => {
            painter.add(epaint::PathShape::closed_line(
                vec![
                    p(r, 1.5, 4.0),
                    p(r, 6.0, 4.0),
                    p(r, 7.5, 5.5),
                    p(r, 14.5, 5.5),
                    p(r, 14.5, 13.5),
                    p(r, 1.5, 13.5),
                ],
                s,
            ));
            painter.line_segment([p(r, 1.5, 8.0), p(r, 14.5, 8.0)], thin);
        }
        Action::Save => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 2.0, 2.0), p(r, 14.0, 14.0)),
                1.0,
                s,
                epaint::StrokeKind::Middle,
            );
            painter.rect_filled(
                Rect::from_min_max(p(r, 5.0, 2.5), p(r, 11.0, 6.0)),
                0.0,
                color,
            );
            painter.rect_stroke(
                Rect::from_min_max(p(r, 4.5, 9.0), p(r, 11.5, 13.5)),
                0.0,
                thin,
                epaint::StrokeKind::Middle,
            );
        }
        Action::Print => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 2.0, 6.0), p(r, 14.0, 12.0)),
                1.0,
                s,
                epaint::StrokeKind::Middle,
            );
            painter.rect_stroke(
                Rect::from_min_max(p(r, 5.0, 2.0), p(r, 11.0, 6.0)),
                0.0,
                thin,
                epaint::StrokeKind::Middle,
            );
            painter.rect_stroke(
                Rect::from_min_max(p(r, 5.0, 10.0), p(r, 11.0, 14.5)),
                0.0,
                thin,
                epaint::StrokeKind::Middle,
            );
        }
        Action::Cut => {
            painter.circle_stroke(p(r, 5.0, 12.5), r.width() / 16.0 * 2.2, s);
            painter.circle_stroke(p(r, 11.0, 12.5), r.width() / 16.0 * 2.2, s);
            painter.line_segment([p(r, 6.5, 10.5), p(r, 12.0, 2.0)], s);
            painter.line_segment([p(r, 9.5, 10.5), p(r, 4.0, 2.0)], s);
        }
        Action::Copy => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 2.0, 2.0), p(r, 10.0, 11.0)),
                0.0,
                thin,
                epaint::StrokeKind::Middle,
            );
            painter.rect_filled(
                Rect::from_min_max(p(r, 6.0, 5.0), p(r, 14.0, 14.0)),
                0.0,
                Color32::WHITE,
            );
            painter.rect_stroke(
                Rect::from_min_max(p(r, 6.0, 5.0), p(r, 14.0, 14.0)),
                0.0,
                s,
                epaint::StrokeKind::Middle,
            );
        }
        Action::Paste => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 2.5, 3.0), p(r, 13.5, 14.5)),
                1.0,
                s,
                epaint::StrokeKind::Middle,
            );
            painter.rect_filled(
                Rect::from_min_max(p(r, 5.5, 1.5), p(r, 10.5, 4.5)),
                0.0,
                color,
            );
            painter.rect_stroke(
                Rect::from_min_max(p(r, 6.0, 7.0), p(r, 14.5, 15.0)),
                0.0,
                thin,
                epaint::StrokeKind::Middle,
            );
        }
        Action::Undo | Action::Redo => {
            let flip = a == Action::Redo;
            let fx = |x: f32| if flip { 16.0 - x } else { x };
            let mut pts = Vec::new();
            for i in 0..=12 {
                let t = i as f32 / 12.0;
                let ang = std::f32::consts::PI * (1.0 - t * 0.9);
                pts.push(p(r, fx(8.0 + 5.0 * ang.cos()), 9.0 - 4.5 * ang.sin()));
            }
            painter.add(epaint::PathShape::line(pts, s));
            painter.add(epaint::PathShape::convex_polygon(
                vec![p(r, fx(1.0), 6.5), p(r, fx(5.5), 5.0), p(r, fx(4.5), 10.0)],
                color,
                Stroke::NONE,
            ));
        }
        Action::Import | Action::Export => {
            page(painter, 3.5, 1.5, 12.5, 14.5);
            let up = a == Action::Export;
            let (y0, y1) = if up { (12.0, 6.0) } else { (6.0, 12.0) };
            painter.line_segment([p(r, 8.0, y0), p(r, 8.0, y1)], Stroke::new(2.0, color));
            let d = if up { -1.0 } else { 1.0 };
            painter.line_segment(
                [p(r, 8.0, y1), p(r, 5.5, y1 - 2.5 * d)],
                Stroke::new(2.0, color),
            );
            painter.line_segment(
                [p(r, 8.0, y1), p(r, 10.5, y1 - 2.5 * d)],
                Stroke::new(2.0, color),
            );
        }
        Action::Pdf => {
            painter.text(
                p(r, 8.0, 8.0),
                egui::Align2::CENTER_CENTER,
                "PDF",
                egui::FontId::proportional(r.width() * 0.42),
                color,
            );
        }
        Action::Snap => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 3.0, 3.0), p(r, 13.0, 13.0)),
                0.0,
                thin,
                epaint::StrokeKind::Middle,
            );
            painter.line_segment([p(r, 8.0, 1.0), p(r, 8.0, 15.0)], thin);
            painter.line_segment([p(r, 1.0, 8.0), p(r, 15.0, 8.0)], thin);
            painter.circle_filled(p(r, 8.0, 8.0), r.width() / 16.0 * 1.8, Tokens::ACCENT);
        }
        Action::Options => {
            for i in 0..8 {
                let ang = i as f32 * std::f32::consts::TAU / 8.0;
                painter.line_segment(
                    [
                        p(r, 8.0 + 3.0 * ang.cos(), 8.0 + 3.0 * ang.sin()),
                        p(r, 8.0 + 6.5 * ang.cos(), 8.0 + 6.5 * ang.sin()),
                    ],
                    Stroke::new(2.0, color),
                );
            }
            painter.circle_stroke(p(r, 8.0, 8.0), r.width() / 16.0 * 3.2, s);
        }
        Action::Welcome => {
            for y in [4.0, 8.0, 12.0] {
                painter.line_segment([p(r, 2.5, y), p(r, 13.5, y)], Stroke::new(1.8, color));
            }
        }
        Action::Mirror => {
            painter.line_segment([p(r, 8.0, 1.5), p(r, 8.0, 14.5)], thin);
            painter.add(epaint::PathShape::closed_line(
                vec![p(r, 2.0, 4.0), p(r, 6.5, 8.0), p(r, 2.0, 12.0)],
                s,
            ));
            painter.add(epaint::PathShape::convex_polygon(
                vec![p(r, 14.0, 4.0), p(r, 9.5, 8.0), p(r, 14.0, 12.0)],
                color,
                Stroke::NONE,
            ));
        }
        Action::Flip => {
            painter.line_segment([p(r, 1.5, 8.0), p(r, 14.5, 8.0)], thin);
            painter.add(epaint::PathShape::closed_line(
                vec![p(r, 4.0, 2.0), p(r, 8.0, 6.5), p(r, 12.0, 2.0)],
                s,
            ));
            painter.add(epaint::PathShape::convex_polygon(
                vec![p(r, 4.0, 14.0), p(r, 8.0, 9.5), p(r, 12.0, 14.0)],
                color,
                Stroke::NONE,
            ));
        }
    }
}
