//! Toolbox icons drawn with the painter, 16x16 design grid, no bitmaps.

use crate::theme::Tokens;
use crate::tools::Tool;
use egui::{epaint, Color32, Painter, Pos2, Rect, Stroke, Vec2};

fn p(r: Rect, x: f32, y: f32) -> Pos2 {
    r.min + Vec2::new(x, y) * (r.width() / 16.0)
}

/// Small drawing context for one icon: all coordinates are on the 16x16 design grid.
struct Ic<'a> {
    painter: &'a Painter,
    r: Rect,
    color: Color32,
    s: Stroke,
    thin: Stroke,
}

impl Ic<'_> {
    fn at(&self, x: f32, y: f32) -> Pos2 {
        p(self.r, x, y)
    }

    fn u(&self, v: f32) -> f32 {
        self.r.width() / 16.0 * v
    }

    fn line(&self, x0: f32, y0: f32, x1: f32, y1: f32, st: Stroke) {
        self.painter
            .line_segment([self.at(x0, y0), self.at(x1, y1)], st);
    }

    fn open(&self, pts: &[(f32, f32)], st: Stroke) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.at(x, y)).collect();
        self.painter.add(epaint::PathShape::line(v, st));
    }

    fn closed(&self, pts: &[(f32, f32)], st: Stroke) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.at(x, y)).collect();
        self.painter.add(epaint::PathShape::closed_line(v, st));
    }

    fn fill(&self, pts: &[(f32, f32)], color: Color32) {
        let v: Vec<Pos2> = pts.iter().map(|&(x, y)| self.at(x, y)).collect();
        self.painter
            .add(epaint::PathShape::convex_polygon(v, color, Stroke::NONE));
    }

    fn rect_s(&self, x0: f32, y0: f32, x1: f32, y1: f32, round: f32, st: Stroke) {
        self.painter.rect_stroke(
            Rect::from_min_max(self.at(x0, y0), self.at(x1, y1)),
            self.u(round),
            st,
            epaint::StrokeKind::Middle,
        );
    }

    fn rect_f(&self, x0: f32, y0: f32, x1: f32, y1: f32, color: Color32) {
        self.painter.rect_filled(
            Rect::from_min_max(self.at(x0, y0), self.at(x1, y1)),
            0.0,
            color,
        );
    }

    fn circle_s(&self, cx: f32, cy: f32, rad: f32, st: Stroke) {
        self.painter.circle_stroke(self.at(cx, cy), self.u(rad), st);
    }

    fn circle_f(&self, cx: f32, cy: f32, rad: f32, color: Color32) {
        self.painter
            .circle_filled(self.at(cx, cy), self.u(rad), color);
    }

    /// Filled dot used for curve nodes and dimension points.
    fn dot(&self, cx: f32, cy: f32) {
        self.circle_f(cx, cy, 1.2, self.color);
    }

    /// Small filled square used as a node handle.
    fn node(&self, cx: f32, cy: f32) {
        self.rect_f(cx - 1.2, cy - 1.2, cx + 1.2, cy + 1.2, self.color);
    }

    /// Small hollow square used as a control point.
    fn node_hollow(&self, cx: f32, cy: f32) {
        self.rect_s(cx - 1.2, cy - 1.2, cx + 1.2, cy + 1.2, 0.0, self.thin);
    }

    /// Points along an elliptical arc, angles in radians.
    fn arc(
        &self,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        a0: f32,
        a1: f32,
        n: usize,
    ) -> Vec<(f32, f32)> {
        (0..=n)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f32 / n as f32;
                (cx + rx * a.cos(), cy + ry * a.sin())
            })
            .collect()
    }

    fn ellipse_s(&self, cx: f32, cy: f32, rx: f32, ry: f32, st: Stroke) {
        let pts = self.arc(cx, cy, rx, ry, 0.0, std::f32::consts::TAU, 40);
        self.closed(&pts[..40], st);
    }

    /// Filled arrowhead pointing from (fx, fy) toward (tx, ty), tip at (tx, ty).
    fn arrow_head(&self, fx: f32, fy: f32, tx: f32, ty: f32, size: f32) {
        let d = Vec2::new(tx - fx, ty - fy).normalized();
        let n = Vec2::new(-d.y, d.x);
        let base_x = tx - d.x * size;
        let base_y = ty - d.y * size;
        let w = size * 0.55;
        self.fill(
            &[
                (tx, ty),
                (base_x + n.x * w, base_y + n.y * w),
                (base_x - n.x * w, base_y - n.y * w),
            ],
            self.color,
        );
    }

    /// Dashed straight line with the given dash length on the design grid.
    fn dashed(&self, x0: f32, y0: f32, x1: f32, y1: f32, dash: f32, st: Stroke) {
        let d = Vec2::new(x1 - x0, y1 - y0);
        let len = d.length();
        if len <= 0.0 {
            return;
        }
        let dir = d / len;
        let mut t = 0.0;
        while t < len {
            let e = (t + dash).min(len);
            self.line(
                x0 + dir.x * t,
                y0 + dir.y * t,
                x0 + dir.x * e,
                y0 + dir.y * e,
                st,
            );
            t += dash * 2.0;
        }
    }

    /// The classic pick cursor arrow, filled or hollow, offset by (dx, dy).
    fn cursor(&self, dx: f32, dy: f32, scale: f32, filled: bool) {
        let base = [
            (4.0, 2.0),
            (4.0, 13.0),
            (7.0, 10.5),
            (9.0, 14.0),
            (11.0, 13.0),
            (9.0, 9.5),
            (12.5, 9.5),
        ];
        let pts: Vec<(f32, f32)> = base
            .iter()
            .map(|&(x, y)| (dx + (x - 4.0) * scale, dy + (y - 2.0) * scale))
            .collect();
        if filled {
            self.fill(&pts, self.color);
        } else {
            self.closed(&pts, self.s);
        }
    }

    /// A pencil drawn diagonally from the top right down to the tip at (tx, ty).
    fn pencil(&self, tx: f32, ty: f32, len: f32) {
        let d = Vec2::new(1.0, -1.0).normalized();
        let n = Vec2::new(-d.y, d.x) * 1.6;
        let tip = Vec2::new(tx, ty);
        let neck = tip + d * 2.8;
        let end = tip + d * len;
        let body = [
            (neck.x + n.x, neck.y + n.y),
            (end.x + n.x, end.y + n.y),
            (end.x - n.x, end.y - n.y),
            (neck.x - n.x, neck.y - n.y),
        ];
        self.closed(&body, self.thin);
        self.closed(
            &[
                (tip.x, tip.y),
                (neck.x + n.x, neck.y + n.y),
                (neck.x - n.x, neck.y - n.y),
            ],
            self.thin,
        );
        self.fill(
            &[
                (tip.x, tip.y),
                (
                    tip.x + d.x * 1.2 + n.x * 0.45,
                    tip.y + d.y * 1.2 + n.y * 0.45,
                ),
                (
                    tip.x + d.x * 1.2 - n.x * 0.45,
                    tip.y + d.y * 1.2 - n.y * 0.45,
                ),
            ],
            self.color,
        );
        let cap = end - d * 1.6;
        self.line(
            cap.x + n.x,
            cap.y + n.y,
            cap.x - n.x,
            cap.y - n.y,
            self.thin,
        );
    }

    /// Four-point sparkle centred at (cx, cy).
    fn sparkle(&self, cx: f32, cy: f32, rad: f32) {
        let w = rad * 0.3;
        self.fill(
            &[
                (cx, cy - rad),
                (cx + w, cy - w),
                (cx + rad, cy),
                (cx + w, cy + w),
                (cx, cy + rad),
                (cx - w, cy + w),
                (cx - rad, cy),
                (cx - w, cy - w),
            ],
            self.color,
        );
    }

    /// Paint bucket tilted to the right with a pouring drop.
    fn bucket(&self) {
        self.closed(&[(3.0, 7.0), (8.0, 2.0), (13.0, 7.0), (8.0, 12.0)], self.s);
        self.fill(
            &[(3.0, 7.0), (8.0, 12.0), (8.0, 14.5), (3.0, 9.5)],
            self.color,
        );
        self.line(5.5, 4.5, 5.5, 2.0, self.thin);
        self.circle_f(13.2, 10.5, 1.4, self.color);
        self.fill(&[(13.2, 7.8), (14.6, 10.5), (11.8, 10.5)], self.color);
    }

    /// Eyedropper pointing down-left with the tip at (3, 13).
    fn dropper(&self) {
        self.painter.line_segment(
            [self.at(3.0, 13.0), self.at(9.5, 6.5)],
            Stroke::new(2.2, self.color),
        );
        self.line(8.0, 5.0, 11.0, 8.0, self.thin);
        self.circle_f(11.5, 4.5, 2.2, self.color);
        self.circle_f(3.0, 13.0, 1.2, self.color);
    }

    /// Hollow pen nib pointing down with the tip at (tx, ty).
    fn nib(&self, tx: f32, ty: f32) {
        self.closed(
            &[
                (tx, ty),
                (tx - 3.5, ty - 5.0),
                (tx - 2.5, ty - 9.0),
                (tx + 2.5, ty - 9.0),
                (tx + 3.5, ty - 5.0),
            ],
            self.s,
        );
        self.line(tx, ty - 1.5, tx, ty - 5.0, self.thin);
        self.circle_f(tx, ty - 5.5, 1.0, self.color);
    }

    /// Dimension line between two points with filled arrowheads at both ends.
    fn dim_line(&self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.line(x0, y0, x1, y1, self.thin);
        self.arrow_head(x1, y1, x0, y0, 2.6);
        self.arrow_head(x0, y0, x1, y1, 2.6);
    }
}

pub fn draw(painter: &Painter, r: Rect, tool: Tool, color: Color32) {
    // Stroke weights follow the icon size (1.25 and 0.9 px at 16 px).
    let k = r.width() / 16.0;
    let s = Stroke::new(1.25 * k, color);
    let thin = Stroke::new((0.9 * k).max(1.0), color);
    // Colour accents (connector ends, the dropper's tip, the bucket's paint)
    // only on the normal icon colour; dimmed icons stay monochrome.
    let accent = if color == Tokens::ICON {
        Color32::from_rgb(0xE8, 0x5A, 0x1C)
    } else {
        color
    };
    let paint = if color == Tokens::ICON {
        Color32::from_rgb(0xE0, 0x10, 0x1A)
    } else {
        color
    };
    let faint = color.gamma_multiply(0.35);
    let faint_stroke = Stroke::new(1.0, faint);
    let ic = Ic {
        painter,
        r,
        color,
        s,
        thin,
    };
    use std::f32::consts::{FRAC_PI_2, PI, TAU};
    match tool {
        // Pick group: arrow, arrow with lasso, arrow with rotate arc.
        Tool::Pick => ic.cursor(4.0, 2.0, 1.0, true),
        Tool::FreeformPick => {
            ic.cursor(2.5, 1.5, 0.75, true);
            let loop_pts = ic.arc(10.5, 11.0, 4.0, 3.0, 0.0, TAU, 24);
            ic.closed(&loop_pts[..24], thin);
            ic.line(7.0, 13.0, 9.5, 14.5, thin);
        }
        Tool::FreeTransform => {
            ic.cursor(2.5, 3.5, 0.8, true);
            let a = ic.arc(10.0, 6.0, 4.5, 4.5, PI * 1.05, PI * 1.95, 12);
            ic.open(&a, thin);
            ic.arrow_head(13.2, 1.9, 14.6, 5.0, 2.4);
        }

        // Shape edit group.
        Tool::Shape => {
            // A curve with a hollow node and the pointer reshaping it.
            let a = ic.arc(11.0, 9.0, 8.0, 7.5, PI * 0.78, PI * 1.32, 14);
            ic.open(&a, s);
            ic.node_hollow(a[5].0, a[5].1);
            ic.cursor(8.0, 6.5, 0.62, true);
        }
        Tool::Smooth => {
            let pts: Vec<(f32, f32)> = (0..=20)
                .map(|i| {
                    let t = i as f32 / 20.0;
                    let amp = 3.5 * (1.0 - t).powi(2);
                    (2.0 + 12.0 * t, 8.0 + amp * (t * 14.0).sin())
                })
                .collect();
            ic.open(&pts, s);
        }
        Tool::Smear => {
            ic.circle_f(5.5, 8.0, 3.0, color);
            ic.fill(&[(5.5, 5.0), (14.5, 7.4), (14.5, 8.6), (5.5, 11.0)], color);
        }
        Tool::Twirl => {
            // Three curved blades spun around the centre.
            for k in 0..3 {
                let off = k as f32 * TAU / 3.0;
                let pts: Vec<(f32, f32)> = (0..=10)
                    .map(|i| {
                        let t = i as f32 / 10.0;
                        let a = off + t * 2.2;
                        let rad = 1.0 + 5.5 * t;
                        (8.0 + rad * a.cos(), 8.0 + rad * a.sin())
                    })
                    .collect();
                ic.open(&pts, s);
            }
            ic.circle_f(8.0, 8.0, 1.1, color);
        }
        Tool::AttractRepel => {
            ic.circle_s(8.0, 8.0, 2.6, s);
            for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let (fx, fy) = (8.0 + 6.3 * dx, 8.0 + 6.3 * dy);
                let (tx, ty) = (8.0 + 3.4 * dx, 8.0 + 3.4 * dy);
                ic.line(fx, fy, tx, ty, thin);
                ic.arrow_head(fx, fy, tx, ty, 2.2);
            }
        }
        Tool::Smudge => {
            // Brush handle with a bristle tip and a trailing smear.
            ic.painter
                .line_segment([ic.at(13.5, 2.5), ic.at(8.5, 7.5)], Stroke::new(2.2, color));
            ic.fill(&[(8.5, 6.0), (10.0, 7.5), (6.5, 11.0), (5.0, 9.5)], color);
            ic.open(&[(5.0, 11.0), (4.0, 13.0), (2.0, 14.0)], s);
            ic.open(&[(6.5, 12.0), (6.0, 14.5)], thin);
        }
        Tool::Roughen => {
            // A rectangle whose top edge has been roughened into a zigzag.
            let mut pts = vec![(2.0, 13.5), (2.0, 7.0)];
            for i in 0..6 {
                let x = 2.0 + i as f32 * 2.0;
                pts.push((x + 1.0, 4.5));
                pts.push((x + 2.0, 7.0));
            }
            pts.push((14.0, 13.5));
            ic.closed(&pts, s);
        }

        // Crop group.
        Tool::Crop => {
            // Two crop marks.
            let w = Stroke::new(1.9 * k, color);
            ic.line(5.0, 1.5, 5.0, 11.0, w);
            ic.line(5.0, 11.0, 14.5, 11.0, w);
            ic.line(1.5, 5.0, 11.0, 5.0, w);
            ic.line(11.0, 5.0, 11.0, 14.5, w);
        }
        Tool::Knife => {
            // Blade with a straight spine and a curved cutting edge, handle at lower left.
            ic.fill(&[(2.0, 14.0), (4.5, 11.5), (6.5, 13.5), (4.0, 15.0)], color);
            let mut blade = vec![(5.5, 10.5), (13.5, 2.5), (14.5, 3.5)];
            blade.extend(ic.arc(14.5, 10.5, 7.0, 7.0, -FRAC_PI_2, -PI * 0.85, 6));
            blade.push((7.5, 12.5));
            ic.closed(&blade, s);
        }
        Tool::SegmentDelete => {
            // Curve with its middle segment faded out and marked with an X.
            let a = ic.arc(8.0, 14.0, 9.0, 8.0, PI * 1.08, PI * 1.35, 6);
            let b = ic.arc(8.0, 14.0, 9.0, 8.0, PI * 1.35, PI * 1.65, 6);
            let c = ic.arc(8.0, 14.0, 9.0, 8.0, PI * 1.65, PI * 1.92, 6);
            ic.open(&a, s);
            ic.open(&b, faint_stroke);
            ic.open(&c, s);
            ic.dot(a[a.len() - 1].0, a[a.len() - 1].1);
            ic.dot(c[0].0, c[0].1);
            ic.line(6.0, 10.0, 10.0, 14.0, s);
            ic.line(10.0, 10.0, 6.0, 14.0, s);
        }
        Tool::Eraser => {
            // Tilted eraser block with a darker rubbing end and the surface line.
            ic.closed(&[(3.0, 10.0), (10.0, 3.0), (14.0, 7.0), (7.0, 14.0)], s);
            ic.fill(&[(3.0, 10.0), (6.0, 7.0), (10.0, 11.0), (7.0, 14.0)], color);
            ic.line(2.0, 15.0, 14.0, 15.0, thin);
        }

        // Zoom group.
        Tool::Zoom => {
            // A magnifying glass.
            ic.circle_s(6.8, 6.8, 5.0, s);
            ic.painter.line_segment(
                [ic.at(10.4, 10.4), ic.at(14.8, 14.8)],
                Stroke::new(1.9 * k, color),
            );
        }
        Tool::Pan => {
            // Open hand: four fingers, thumb and palm.
            let mut pts = vec![(4.0, 14.5), (4.0, 9.0), (2.0, 7.0), (3.5, 6.0), (5.0, 7.5)];
            pts.extend([(5.0, 2.5), (6.5, 2.5), (6.5, 7.0)]);
            pts.extend([(7.5, 1.5), (9.0, 1.5), (9.0, 7.0)]);
            pts.extend([(10.0, 2.5), (11.5, 2.5), (11.5, 7.5)]);
            pts.extend([(12.5, 4.0), (14.0, 4.0), (14.0, 10.0), (12.0, 14.5)]);
            ic.closed(&pts, thin);
        }

        // Curve group.
        Tool::Freehand => {
            // The drawing cursor and a freehand scribble.
            ic.line(1.0, 3.5, 6.0, 3.5, thin);
            ic.line(3.5, 1.0, 3.5, 6.0, thin);
            let pts: Vec<(f32, f32)> = (0..=28)
                .map(|i| {
                    let t = i as f32 / 28.0;
                    let x = 3.5 + 11.5 * t;
                    let y = if t < 0.18 {
                        7.5 + 30.0 * t
                    } else {
                        13.0 - 3.0 * ((t - 0.18) * 26.0).sin().abs()
                    };
                    (x, y)
                })
                .collect();
            ic.open(&pts, s);
        }
        Tool::TwoPointLine => {
            ic.line(3.0, 13.0, 13.0, 3.0, s);
            ic.dot(3.0, 13.0);
            ic.dot(13.0, 3.0);
        }
        Tool::Bezier => {
            let pts: Vec<(f32, f32)> = (0..=16)
                .map(|i| {
                    let t = i as f32 / 16.0;
                    let x = 3.0 + 10.0 * t;
                    let y = 12.0 - 8.0 * (3.0 * t * t - 2.0 * t * t * t);
                    (x, y)
                })
                .collect();
            ic.open(&pts, s);
            ic.line(3.0, 12.0, 7.5, 12.0, thin);
            ic.line(13.0, 4.0, 8.5, 4.0, thin);
            ic.node(3.0, 12.0);
            ic.node(13.0, 4.0);
            ic.node_hollow(7.5, 12.0);
            ic.node_hollow(8.5, 4.0);
        }
        Tool::Pen => {
            ic.nib(8.0, 14.0);
        }
        Tool::BSpline => {
            // Smooth curve hugging a dashed control polygon with hollow control points.
            let ctrl = [(2.0, 12.0), (5.0, 3.0), (11.0, 13.0), (14.0, 4.0)];
            for w in ctrl.windows(2) {
                ic.dashed(w[0].0, w[0].1, w[1].0, w[1].1, 1.0, faint_stroke);
            }
            let pts: Vec<(f32, f32)> = (0..=20)
                .map(|i| {
                    let t = i as f32 / 20.0;
                    // Quadratic B-spline evaluated through the two inner spans.
                    let seg = if t < 0.5 { 0 } else { 1 };
                    let u = if seg == 0 { t * 2.0 } else { (t - 0.5) * 2.0 };
                    let (p0, p1, p2) = (ctrl[seg], ctrl[seg + 1], ctrl[seg + 2]);
                    let b0 = (1.0 - u) * (1.0 - u) * 0.5;
                    let b1 = -u * u + u + 0.5;
                    let b2 = u * u * 0.5;
                    (
                        p0.0 * b0 + p1.0 * b1 + p2.0 * b2,
                        p0.1 * b0 + p1.1 * b1 + p2.1 * b2,
                    )
                })
                .collect();
            ic.open(&pts, s);
            for &(x, y) in &ctrl {
                ic.node_hollow(x, y);
            }
        }
        Tool::Polyline => {
            let pts = [
                (2.0, 12.0),
                (5.0, 4.0),
                (8.5, 11.0),
                (11.0, 5.0),
                (14.0, 12.0),
            ];
            ic.open(&pts, s);
            for &(x, y) in &pts {
                ic.node(x, y);
            }
        }
        Tool::ThreePointCurve => {
            let a = ic.arc(8.0, 12.0, 6.0, 8.0, PI * 1.1, PI * 1.9, 14);
            ic.open(&a, s);
            ic.dot(a[0].0, a[0].1);
            ic.dot(a[7].0, a[7].1);
            ic.dot(a[14].0, a[14].1);
        }
        Tool::ShapeRecognition => {
            ic.pencil(2.5, 13.5, 11.0);
            ic.sparkle(12.5, 3.5, 3.0);
        }
        Tool::Sketch => {
            // Several overlapping sketchy strokes that read as one line.
            for (k, off) in [(0usize, -1.2f32), (1, 0.0), (2, 1.2)] {
                let pts: Vec<(f32, f32)> = (0..=12)
                    .map(|i| {
                        let t = i as f32 / 12.0;
                        let wob = ((t * 9.0) + k as f32).sin() * 0.6;
                        (2.0 + 12.0 * t, 11.0 - 6.0 * t + off + wob)
                    })
                    .collect();
                ic.open(&pts, if k == 1 { s } else { faint_stroke });
            }
        }

        // Brush strokes: a tapered brush stroke.
        Tool::BrushStrokes => {
            // A brush stroke shaped like an S, thickening towards a heavy,
            // rounded end at the bottom left.
            let mut pts = ic.arc(7.5, 5.0, 3.0, 3.2, -PI * 0.15, -PI * 1.5, 12);
            pts.extend(ic.arc(8.0, 11.4, 4.2, 3.2, -PI * 0.5, PI * 0.85, 14));
            for (i, w) in pts.windows(2).enumerate() {
                let t = i as f32 / pts.len() as f32;
                ic.painter.line_segment(
                    [ic.at(w[0].0, w[0].1), ic.at(w[1].0, w[1].1)],
                    Stroke::new(ic.u(0.9 + 1.4 * t), color),
                );
            }
            if let Some(&(x, y)) = pts.last() {
                ic.circle_f(x, y, 1.5, color);
            }
        }

        // Rectangle group.
        Tool::Rectangle => ic.rect_s(1.8, 1.8, 14.2, 14.2, 0.0, s),
        Tool::ThreePointRectangle => {
            let pts = [(3.0, 6.0), (9.5, 2.0), (13.5, 8.5), (7.0, 12.5)];
            ic.closed(&pts, s);
            ic.dot(3.0, 6.0);
            ic.dot(9.5, 2.0);
            ic.dot(13.5, 8.5);
        }

        // Ellipse group.
        Tool::Ellipse => ic.ellipse_s(8.0, 8.0, 6.4, 6.4, s),
        Tool::ThreePointEllipse => {
            // Tilted ellipse built from a rotated parametric curve.
            let ang = -0.6f32;
            let (rx, ry) = (5.5f32, 3.2f32);
            let pts: Vec<(f32, f32)> = (0..40)
                .map(|i| {
                    let a = i as f32 / 40.0 * TAU;
                    let (ex, ey) = (rx * a.cos(), ry * a.sin());
                    (
                        8.0 + ex * ang.cos() - ey * ang.sin(),
                        8.0 + ex * ang.sin() + ey * ang.cos(),
                    )
                })
                .collect();
            ic.closed(&pts, s);
            let tip = |a: f32| {
                let (ex, ey) = (rx * a.cos(), ry * a.sin());
                (
                    8.0 + ex * ang.cos() - ey * ang.sin(),
                    8.0 + ex * ang.sin() + ey * ang.cos(),
                )
            };
            for a in [0.0, PI, FRAC_PI_2] {
                let (x, y) = tip(a);
                ic.dot(x, y);
            }
        }

        // Polygon group.
        Tool::Polygon => {
            // A hexagon with flat top and bottom.
            let pts: Vec<(f32, f32)> = (0..6)
                .map(|i| {
                    let a = i as f32 * TAU / 6.0;
                    (8.0 + 6.8 * a.cos(), 8.0 + 6.2 * a.sin())
                })
                .collect();
            ic.closed(&pts, s);
        }
        Tool::Star => {
            let pts: Vec<(f32, f32)> = (0..10)
                .map(|i| {
                    let a = -FRAC_PI_2 + i as f32 * TAU / 10.0;
                    let rad = if i % 2 == 0 { 6.5 } else { 2.8 };
                    (8.0 + rad * a.cos(), 8.5 + rad * a.sin())
                })
                .collect();
            ic.closed(&pts, s);
        }
        Tool::Spiral => {
            let pts: Vec<(f32, f32)> = (0..=48)
                .map(|i| {
                    let t = i as f32 / 48.0;
                    let a = t * TAU * 2.5;
                    let rad = 0.5 + 6.0 * t;
                    (8.0 + rad * a.cos(), 8.0 + rad * a.sin())
                })
                .collect();
            ic.open(&pts, s);
        }
        Tool::CommonShapes => {
            // Heart: two arcs meeting at the bottom point.
            let mut pts = ic.arc(5.0, 6.0, 3.2, 3.2, PI, TAU, 10);
            pts.extend(ic.arc(11.0, 6.0, 3.2, 3.2, PI, TAU, 10));
            pts.push((13.8, 7.5));
            pts.push((8.0, 14.0));
            pts.push((2.2, 7.5));
            ic.closed(&pts, s);
        }
        Tool::ActionLines => {
            // A small filled shape rushing right with speed lines trailing left.
            ic.fill(&[(10.0, 5.0), (14.5, 8.0), (10.0, 11.0)], color);
            ic.line(2.0, 5.5, 8.5, 5.5, s);
            ic.line(4.0, 8.0, 8.5, 8.0, s);
            ic.line(2.0, 10.5, 8.5, 10.5, s);
        }
        Tool::GraphPaper => {
            ic.rect_s(2.5, 2.5, 13.5, 13.5, 0.0, s);
            for v in [6.17, 9.83] {
                ic.line(v, 2.5, v, 13.5, thin);
                ic.line(2.5, v, 13.5, v, thin);
            }
        }

        // Text group.
        Tool::Text => {
            // A capital A.
            ic.open(&[(2.0, 15.0), (8.0, 1.0), (14.0, 15.0)], s);
            ic.line(4.6, 10.2, 11.4, 10.2, s);
        }
        Tool::Table => {
            ic.rect_s(2.0, 3.0, 14.0, 13.0, 0.0, s);
            ic.rect_f(2.0, 3.0, 14.0, 6.0, color);
            ic.line(2.0, 9.5, 14.0, 9.5, thin);
            ic.line(6.0, 6.0, 6.0, 13.0, thin);
            ic.line(10.0, 6.0, 10.0, 13.0, thin);
        }

        // Dimension group.
        Tool::ParallelDimension => {
            // A slanted dimension line with ticks across both ends.
            ic.line(2.5, 13.5, 13.5, 2.5, s);
            ic.line(0.8, 11.8, 4.2, 15.2, s);
            ic.line(11.8, 0.8, 15.2, 4.2, s);
        }
        Tool::HorizontalVerticalDimension => {
            ic.dim_line(4.5, 13.0, 14.5, 13.0);
            ic.dim_line(3.0, 11.5, 3.0, 1.5);
            ic.line(4.5, 11.0, 4.5, 14.5, thin);
            ic.line(14.5, 11.0, 14.5, 14.5, thin);
        }
        Tool::AngularDimension => {
            ic.line(2.5, 13.5, 14.0, 13.5, s);
            ic.line(2.5, 13.5, 11.5, 3.0, s);
            let a = ic.arc(2.5, 13.5, 8.0, 8.0, -0.85, 0.0, 8);
            ic.open(&a, thin);
            ic.arrow_head(a[1].0, a[1].1, a[0].0, a[0].1, 2.2);
            ic.arrow_head(a[7].0, a[7].1, a[8].0, a[8].1, 2.2);
        }
        Tool::SegmentDimension => {
            ic.line(2.5, 11.0, 13.5, 11.0, s);
            ic.dot(2.5, 11.0);
            ic.dot(8.0, 11.0);
            ic.dot(13.5, 11.0);
            ic.line(2.5, 8.5, 2.5, 4.5, thin);
            ic.line(8.0, 8.5, 8.0, 4.5, thin);
            ic.dim_line(2.5, 5.5, 8.0, 5.5);
        }
        Tool::Callout => {
            ic.rect_s(6.0, 2.0, 14.5, 7.5, 0.5, s);
            ic.line(7.5, 4.0, 13.0, 4.0, thin);
            ic.line(7.5, 5.7, 11.0, 5.7, thin);
            ic.line(7.5, 7.5, 3.0, 13.0, s);
            ic.dot(3.0, 13.0);
        }

        // Connector group.
        Tool::Connector => {
            // A line joining two coloured anchor squares.
            ic.line(3.5, 3.5, 12.5, 12.5, s);
            for (x, y) in [(3.5, 3.5), (12.5, 12.5)] {
                ic.rect_f(x - 2.2, y - 2.2, x + 2.2, y + 2.2, accent);
                ic.rect_s(x - 2.2, y - 2.2, x + 2.2, y + 2.2, 0.0, thin);
            }
        }
        Tool::RightAngleConnector => {
            ic.rect_s(1.5, 1.5, 6.5, 6.5, 0.0, thin);
            ic.rect_s(9.5, 9.5, 14.5, 14.5, 0.0, thin);
            ic.line(6.5, 4.0, 12.0, 4.0, s);
            ic.line(12.0, 4.0, 12.0, 9.5, s);
        }
        Tool::RoundedConnector => {
            ic.rect_s(1.5, 1.5, 6.5, 6.5, 0.0, thin);
            ic.rect_s(9.5, 9.5, 14.5, 14.5, 0.0, thin);
            ic.line(6.5, 4.0, 9.5, 4.0, s);
            let a = ic.arc(9.5, 6.5, 2.5, 2.5, -FRAC_PI_2, 0.0, 6);
            ic.open(&a, s);
            ic.line(12.0, 6.5, 12.0, 9.5, s);
        }
        Tool::AnchorEditing => {
            // Box with its anchor points on the edges, one being dragged.
            ic.rect_s(3.0, 3.0, 12.0, 12.0, 0.0, s);
            ic.node_hollow(7.5, 3.0);
            ic.node_hollow(3.0, 7.5);
            ic.node_hollow(7.5, 12.0);
            ic.node(12.0, 7.5);
            ic.line(12.0, 7.5, 15.0, 7.5, thin);
            ic.line(13.5, 6.0, 13.5, 9.0, thin);
        }

        // Effects group.
        Tool::DropShadow => {
            // A square casting a shadow down and to the right.
            let shade = if color == Tokens::ICON {
                Color32::from_rgb(0x55, 0x55, 0x55)
            } else {
                color
            };
            ic.rect_f(4.0, 4.0, 14.5, 14.5, shade);
            ic.rect_f(1.5, 1.5, 12.0, 12.0, Color32::WHITE);
            ic.rect_s(1.5, 1.5, 12.0, 12.0, 0.0, s);
        }
        Tool::Contour => {
            ic.rect_s(2.0, 2.0, 14.0, 14.0, 0.0, thin);
            ic.rect_s(4.5, 4.5, 11.5, 11.5, 0.0, thin);
            ic.rect_f(6.75, 6.75, 9.25, 9.25, color);
        }
        Tool::Blend => {
            // Square at the left morphing into a circle at the right via a faint midstep.
            ic.rect_s(1.5, 5.0, 6.5, 11.0, 0.0, s);
            ic.rect_s(6.0, 5.5, 10.5, 10.5, 1.3, faint_stroke);
            ic.circle_s(12.0, 8.0, 2.6, s);
        }
        Tool::Distort => {
            // Square with its four sides pushed inward into a pinched shape.
            let mut pts = Vec::new();
            let corners = [(2.5, 2.5), (13.5, 2.5), (13.5, 13.5), (2.5, 13.5)];
            for i in 0..4 {
                let (ax, ay) = corners[i];
                let (bx, by) = corners[(i + 1) % 4];
                let horizontal = ay == by;
                for k in 0..6 {
                    let t = k as f32 / 6.0;
                    let (x, y) = (ax + (bx - ax) * t, ay + (by - ay) * t);
                    let pinch = (PI * t).sin() * 2.2;
                    if horizontal {
                        pts.push((x, y + (8.0 - y).signum() * pinch));
                    } else {
                        pts.push((x + (8.0 - x).signum() * pinch, y));
                    }
                }
            }
            ic.closed(&pts, s);
        }
        Tool::Envelope => {
            // Square outline with one corner pulled by a node, others still square.
            ic.closed(
                &[
                    (3.0, 3.0),
                    (12.0, 3.0),
                    (14.5, 10.0),
                    (12.0, 13.0),
                    (3.0, 13.0),
                ],
                s,
            );
            ic.dashed(12.0, 3.0, 12.0, 13.0, 1.0, faint_stroke);
            ic.node(3.0, 3.0);
            ic.node(12.0, 3.0);
            ic.node(3.0, 13.0);
            ic.node(12.0, 13.0);
            ic.node(14.5, 10.0);
        }
        Tool::Extrude => {
            // Isometric cube.
            ic.rect_s(2.5, 5.5, 10.5, 13.5, 0.0, s);
            ic.open(
                &[
                    (2.5, 5.5),
                    (5.5, 2.5),
                    (13.5, 2.5),
                    (13.5, 10.5),
                    (10.5, 13.5),
                ],
                s,
            );
            ic.line(10.5, 5.5, 13.5, 2.5, s);
        }
        Tool::BlockShadow => {
            // Square with a solid block extruded behind it.
            ic.fill(
                &[(10.5, 3.5), (14.0, 7.0), (14.0, 14.5), (10.5, 11.0)],
                faint,
            );
            ic.fill(
                &[(3.0, 11.0), (10.5, 11.0), (14.0, 14.5), (6.5, 14.5)],
                faint,
            );
            ic.rect_f(3.0, 3.5, 10.5, 11.0, color);
        }

        Tool::Transparency => {
            // A checkerboard.
            let n = 6;
            let cell = 13.0 / n as f32;
            for i in 0..n {
                for j in 0..n {
                    if (i + j) % 2 == 0 {
                        let (x, y) = (1.5 + i as f32 * cell, 1.5 + j as f32 * cell);
                        ic.rect_f(x, y, x + cell, y + cell, color);
                    }
                }
            }
            ic.rect_s(1.5, 1.5, 14.5, 14.5, 0.0, thin);
        }

        // Eyedropper group.
        Tool::ColorEyedropper => {
            ic.dropper();
            // The tip holds the picked colour.
            ic.painter.line_segment(
                [ic.at(3.0, 13.0), ic.at(6.5, 9.5)],
                Stroke::new(2.4 * k, accent),
            );
        }
        Tool::AttributesEyedropper => {
            ic.dropper();
            ic.rect_s(10.0, 10.0, 14.5, 14.5, 0.0, s);
            ic.line(10.0, 12.25, 14.5, 12.25, thin);
        }

        // Fill group.
        Tool::InteractiveFill => {
            ic.bucket();
            // Paint in the bucket and a drop of it at the corner.
            ic.fill(&[(4.2, 8.2), (11.8, 8.2), (8.0, 12.0)], paint);
            ic.fill(&[(15.0, 11.0), (15.0, 15.0), (11.0, 15.0)], paint);
        }
        Tool::AreaFill => {
            ic.bucket();
            ic.sparkle(13.0, 3.0, 2.6);
        }
        Tool::MeshFill => {
            ic.rect_s(2.5, 2.5, 13.5, 13.5, 0.0, s);
            let hor: Vec<(f32, f32)> = (0..=10)
                .map(|i| {
                    let t = i as f32 / 10.0;
                    (2.5 + 11.0 * t, 8.0 + 1.8 * (t * PI).sin())
                })
                .collect();
            let ver: Vec<(f32, f32)> = (0..=10)
                .map(|i| {
                    let t = i as f32 / 10.0;
                    (8.0 - 1.8 * (t * PI).sin(), 2.5 + 11.0 * t)
                })
                .collect();
            ic.open(&hor, thin);
            ic.open(&ver, thin);
            ic.node(hor[5].0, ver[5].1);
            ic.node_hollow(2.5, 8.0);
            ic.node_hollow(13.5, 8.0);
            ic.node_hollow(8.0, 2.5);
            ic.node_hollow(8.0, 13.5);
        }

        // Outline group.
        Tool::OutlinePen => {
            ic.nib(8.0, 11.5);
            ic.line(3.0, 14.5, 13.0, 14.5, Stroke::new(1.8, color));
        }
        Tool::OutlineColor => {
            ic.rect_s(2.5, 2.5, 11.5, 11.5, 0.0, Stroke::new(1.8, color));
            ic.rect_f(8.5, 8.5, 14.5, 14.5, Tokens::ACCENT);
            ic.rect_s(8.5, 8.5, 14.5, 14.5, 0.0, thin);
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
    Fullscreen,
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
        Action::Fullscreen => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 2.0, 3.0), p(r, 14.0, 13.0)),
                0.0,
                s,
                epaint::StrokeKind::Middle,
            );
            painter.line_segment([p(r, 5.0, 6.0), p(r, 5.0, 10.0)], thin);
            painter.line_segment([p(r, 11.0, 6.0), p(r, 11.0, 10.0)], thin);
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
