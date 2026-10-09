//! Brush Strokes modes: Preset (tapered strokes), Brush (textured
//! strokes), Sprayer (objects along the path), Calligraphic, Expression
//! (pressure-like variable width). Each mode turns a freehand polyline
//! into geometry; "Apply to curve" does the same to an existing curve.

use crate::app::App;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{BezPath, Point, Vec2},
    Color, Command, Fill,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaMode {
    Preset,
    Brush,
    Sprayer,
    #[default]
    Calligraphic,
    Expression,
}

impl MediaMode {
    pub const ALL: [MediaMode; 5] = [
        MediaMode::Preset,
        MediaMode::Brush,
        MediaMode::Sprayer,
        MediaMode::Calligraphic,
        MediaMode::Expression,
    ];
    pub fn key(self) -> &'static str {
        match self {
            MediaMode::Preset => "media.preset",
            MediaMode::Brush => "media.brush",
            MediaMode::Sprayer => "media.sprayer",
            MediaMode::Calligraphic => "media.calligraphic",
            MediaMode::Expression => "media.expression",
        }
    }
}

pub const BRUSH_KEYS: [&str; 6] = [
    "media.taper_both",
    "media.taper_end",
    "media.taper_start",
    "media.wave",
    "media.bulge",
    "media.flat",
];

pub const SPRAY_KEYS: [&str; 5] = [
    "media.spray_circles",
    "media.spray_stars",
    "media.spray_squares",
    "media.spray_hearts",
    "media.spray_leaves",
];

/// Width profile along the stroke for the preset and brush modes: t in 0..1.
fn profile(preset: usize, t: f64) -> f64 {
    match preset {
        0 => (t * std::f64::consts::PI).sin(),
        1 => 1.0 - t,
        2 => t,
        3 => 0.5 + 0.5 * (t * std::f64::consts::TAU * 2.0).sin().abs(),
        4 => 0.3 + 0.7 * (t * std::f64::consts::PI).sin(),
        _ => 1.0,
    }
    .max(0.02)
}

/// Variable-width stroke around a polyline.
pub fn variable_width(points: &[Point], width: f64, w_at: impl Fn(f64) -> f64) -> BezPath {
    if points.len() < 2 {
        return BezPath::new();
    }
    let mut cum = vec![0.0];
    for w in points.windows(2) {
        cum.push(cum.last().copied().unwrap_or(0.0) + (w[1] - w[0]).hypot());
    }
    let total = cum.last().copied().unwrap_or(1.0).max(1e-9);
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (i, p) in points.iter().enumerate() {
        let prev = points[i.saturating_sub(1)];
        let next = points[(i + 1).min(points.len() - 1)];
        let tangent = next - prev;
        let n = if tangent.hypot() > 1e-9 {
            Vec2::new(-tangent.y, tangent.x).normalize()
        } else {
            Vec2::new(0.0, 1.0)
        };
        let w = width / 2.0 * w_at(cum[i] / total);
        left.push(*p + n * w);
        right.push(*p - n * w);
    }
    right.reverse();
    left.extend(right);
    let p = tracedraw_core::geometry::smooth_path(&left, true);
    tracedraw_core::shaping::simplify(&p)
}

fn spray_object(preset: usize, c: Point, size: f64, k: usize) -> BezPath {
    let r = size / 2.0;
    let rect = tracedraw_core::geometry::Rect::new(c.x - r, c.y - r, c.x + r, c.y + r);
    match preset {
        0 => tracedraw_core::geometry::ellipse_path(rect),
        1 => tracedraw_core::geometry::polygon_path(rect, 5, 0.5),
        2 => tracedraw_core::geometry::rect_path(rect, 0.0),
        3 => {
            // Heart: two circles and a triangle, welded.
            let top = tracedraw_core::geometry::ellipse_path(tracedraw_core::geometry::Rect::new(
                c.x - r,
                c.y,
                c.x,
                c.y + r,
            ));
            let top2 = tracedraw_core::geometry::ellipse_path(tracedraw_core::geometry::Rect::new(
                c.x,
                c.y,
                c.x + r,
                c.y + r,
            ));
            let tri = tracedraw_core::effects::polygon(&[
                Point::new(c.x - r, c.y + r * 0.5),
                Point::new(c.x + r, c.y + r * 0.5),
                Point::new(c.x, c.y - r),
            ]);
            let a =
                tracedraw_core::shaping::overlay(&top, &top2, tracedraw_core::shaping::Op::Weld);
            tracedraw_core::shaping::overlay(&a, &tri, tracedraw_core::shaping::Op::Weld)
        }
        _ => {
            // Leaf: a lens shape rotated with the index.
            let mut p = BezPath::new();
            p.move_to((c.x - r, c.y));
            p.curve_to(
                (c.x - r * 0.3, c.y + r),
                (c.x + r * 0.3, c.y + r),
                (c.x + r, c.y),
            );
            p.curve_to(
                (c.x + r * 0.3, c.y - r),
                (c.x - r * 0.3, c.y - r),
                (c.x - r, c.y),
            );
            p.close_path();
            tracedraw_core::geometry::Affine::rotate_about((k as f64 * 0.7).sin() * 0.8, c) * p
        }
    }
}

impl App {
    /// Geometry for a stroke in the current media mode. Returns shapes to
    /// add (several for the sprayer).
    pub fn media_shapes(&mut self, points: &[Point]) -> Vec<Shape> {
        let pts =
            tracedraw_core::geometry::simplify(points, 0.1 + self.media_smoothing / 100.0 * 1.5);
        if pts.len() < 2 {
            return Vec::new();
        }
        let color = match &self.default_stroke {
            Some(st) => st.color,
            None => Color::BLACK,
        };
        let width = self.media_width;
        let mut out = Vec::new();
        let mut make = |path: BezPath, name: &str| {
            if path.elements().is_empty() {
                return;
            }
            let id = self.engine.new_shape_id();
            let mut s = Shape::new(id, ShapeKind::Path { path, closed: true });
            s.fill = Fill::Solid(color);
            s.stroke = None;
            s.name = Some(name.into());
            out.push(s);
        };
        match self.media_mode {
            MediaMode::Calligraphic => make(
                crate::tools2::calligraphic_path(&pts, width, self.media_angle),
                "Calligraphic",
            ),
            MediaMode::Preset => {
                let preset = self.media_preset;
                make(
                    variable_width(&pts, width, |t| profile(preset, t)),
                    "Preset stroke",
                )
            }
            MediaMode::Brush => {
                // Brush: the preset profile plus a jittered edge.
                let preset = self.media_preset;
                make(
                    variable_width(&pts, width, |t| {
                        profile(preset, t) * (0.85 + 0.15 * (t * 40.0).sin())
                    }),
                    "Brush stroke",
                )
            }
            MediaMode::Expression => {
                // Pressure: speed between samples stands in for pen pressure
                // (slow = wide), scaled by the docker's pressure setting.
                let mut dists = vec![0.0];
                for w in pts.windows(2) {
                    dists.push((w[1] - w[0]).hypot());
                }
                let max = dists.iter().cloned().fold(0.0, f64::max).max(1e-9);
                let n = pts.len();
                let pressure = self.media_pressure;
                let profile_pts: Vec<f64> =
                    dists.iter().map(|d| (1.0 - d / max) * 0.7 + 0.3).collect();
                make(
                    variable_width(&pts, width, |t| {
                        let i = ((t * (n as f64 - 1.0)).round() as usize).min(n - 1);
                        profile_pts[i] * (0.5 + 0.5 * pressure)
                    }),
                    "Expression stroke",
                )
            }
            MediaMode::Sprayer => {
                let spacing = self.media_spacing.max(0.5);
                let preset = self.media_preset;
                let mut cum = vec![0.0];
                for w in pts.windows(2) {
                    cum.push(cum.last().copied().unwrap_or(0.0) + (w[1] - w[0]).hypot());
                }
                let total = cum.last().copied().unwrap_or(0.0);
                let mut s = 0.0;
                let mut k = 0;
                while s <= total {
                    let i = cum
                        .partition_point(|c| *c <= s)
                        .saturating_sub(1)
                        .min(pts.len() - 2);
                    let seg = pts[i + 1] - pts[i];
                    let len = seg.hypot().max(1e-9);
                    let u = ((s - cum[i]) / len).clamp(0.0, 1.0);
                    let c = pts[i] + seg * u;
                    let size = width * (0.7 + 0.3 * ((k as f64 * 1.3).sin().abs()));
                    make(spray_object(preset, c, size, k), "Sprayed object");
                    s += spacing;
                    k += 1;
                }
            }
        }
        out
    }

    /// Brush Strokes docker > Apply to curve: use the selected curves as
    /// the path of a stroke in the current mode.
    pub fn apply_media_to_selection(&mut self) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let curves: Vec<Shape> = self
            .selected_shapes()
            .into_iter()
            .filter(|s| matches!(s.kind, ShapeKind::Path { .. }))
            .collect();
        let mut cmds = Vec::new();
        let mut new_sel = Vec::new();
        for c in curves {
            let path = c.page_path();
            let pts = tracedraw_core::effects::resample(&path, 120);
            for s in self.media_shapes(&pts) {
                new_sel.push(s.id);
                cmds.push(Command::AddShape { layer, shape: s });
            }
            cmds.push(Command::DeleteShapes { shapes: vec![c.id] });
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Brush Strokes", &cmds);
            self.selection = new_sel;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Shape as _;

    #[test]
    fn tapered_stroke_is_thin_at_the_ends() {
        let pts: Vec<Point> = (0..=20).map(|i| Point::new(i as f64 * 5.0, 0.0)).collect();
        let p = variable_width(&pts, 10.0, |t| profile(0, t));
        let b = p.bounding_box();
        assert!(b.height() > 8.0 && b.height() <= 10.5, "{b:?}");
        assert!(b.width() > 95.0);
    }
}
