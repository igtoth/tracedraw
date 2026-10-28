//! Brush Strokes modes: Preset (tapered strokes), Brush (textured
//! strokes), Sprayer (objects along the path), Calligraphic, Expression
//! (pressure-like variable width). Each mode turns a freehand polyline
//! into geometry; "Apply to curve" does the same to an existing curve.

use crate::app::App;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{BezPath, Point},
    style::{Stroke, WidthNode},
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
/// How thick a calligraphic nib is across, as a share of its width.
const CALLIGRAPHIC_STRETCH: f64 = 0.1;
/// Width nodes a preset stroke gets along its path: the width eases
/// between nodes, and each one more costs drawing time.
const PROFILE_NODES: usize = 8;
/// Width nodes an expression stroke's speed is sampled at.
const EXPRESSION_NODES: usize = 16;

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
        self.media_shapes_pen(points, None)
    }

    /// `media_shapes` for a stroke drawn with a pen: `pen` is its
    /// pressure along `points`, which sets the width (#850).
    pub fn media_shapes_pen(
        &mut self,
        points: &[Point],
        pen: Option<&crate::pen::PressureProfile>,
    ) -> Vec<Shape> {
        let pts =
            tracedraw_core::geometry::simplify(points, 0.1 + self.media_smoothing / 100.0 * 1.5);
        if pts.len() < 2 {
            return Vec::new();
        }
        let color = match &self.default_stroke() {
            Some(st) => st.color,
            None => Color::BLACK,
        };
        let width = self.media_width;
        // A stroke stays the path drawn, with the brush as its live
        // outline: its nodes edit like any curve's and the brush follows
        // (#829). Only the sprayer makes objects of their own.
        if let Some(stroke) = self.media_stroke_pen(&pts, color, pen) {
            let id = self.engine.new_shape_id();
            let path = tracedraw_core::geometry::smooth_path(&pts, false);
            let mut s = Shape::new(
                id,
                ShapeKind::Path {
                    path,
                    closed: false,
                },
            );
            s.fill = Fill::None;
            s.stroke = Some(stroke);
            s.name = Some(self.media_mode_name().into());
            return vec![s];
        }
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
        if self.media_mode == MediaMode::Sprayer {
            {
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

    /// The live outline the current mode draws along `pts` in `color`:
    /// a calligraphic nib, or widths along the path from the preset's
    /// profile (with a jittered edge for Brush, the drawing speed for
    /// Expression). `None` for the sprayer.
    pub fn media_stroke(&self, pts: &[Point], color: Color) -> Option<Stroke> {
        self.media_stroke_pen(pts, color, None)
    }

    /// `media_stroke` for a stroke drawn with a pen (#850): its pressure
    /// along the drag scales the width of the Preset, Brush and
    /// Expression modes, from `1 - pressure setting` of it at no pressure
    /// (half, by default) to the full width at full pressure; in
    /// Expression mode it takes the place of the drawing speed. The
    /// calligraphic nib and the sprayer keep their size.
    pub fn media_stroke_pen(
        &self,
        pts: &[Point],
        color: Color,
        pen: Option<&crate::pen::PressureProfile>,
    ) -> Option<Stroke> {
        let width = self.media_width;
        let mut st = Stroke::new(color, width);
        st.cap = tracedraw_core::style::LineCap::Butt;
        let preset = self.media_preset;
        let pen_floor = 1.0 - self.media_pressure.clamp(0.0, 1.0);
        let widths = |n: usize, w_at: &dyn Fn(f64) -> f64| -> Vec<WidthNode> {
            if let Some(pen) = pen {
                return pen.width_nodes(n, width, pen_floor, w_at);
            }
            (0..=n)
                .map(|i| {
                    let t = i as f64 / n as f64;
                    WidthNode::symmetric(t, width * w_at(t).max(0.0))
                })
                .collect()
        };
        match self.media_mode {
            MediaMode::Calligraphic => {
                st.stretch = CALLIGRAPHIC_STRETCH;
                st.nib_angle = self.media_angle;
            }
            MediaMode::Preset => st.widths = widths(PROFILE_NODES, &|t| profile(preset, t)),
            MediaMode::Expression if pen.is_some() => {
                // A real pen: its pressure is the width.
                st.widths = widths(EXPRESSION_NODES, &|_| 1.0);
            }
            MediaMode::Brush => {
                // Nodes at the crests and troughs of the jittered edge: the
                // width eases between them, so they draw the whole wave.
                let pen_scale = |t: f64| match pen {
                    Some(pen) => pen_floor + (1.0 - pen_floor) * pen.at(t, 0.5 / 40.0),
                    None => 1.0,
                };
                let jitter =
                    |t: f64| profile(preset, t) * (0.85 + 0.15 * (t * 40.0).sin()) * pen_scale(t);
                let mut at = vec![0.0];
                at.extend(
                    (0..)
                        .map(|k| {
                            (std::f64::consts::FRAC_PI_2 + k as f64 * std::f64::consts::PI) / 40.0
                        })
                        .take_while(|t| *t < 1.0),
                );
                at.push(1.0);
                st.widths = at
                    .into_iter()
                    .map(|t| WidthNode::symmetric(t, width * jitter(t).max(0.0)))
                    .collect();
            }
            MediaMode::Expression => {
                // Pressure: speed between samples stands in for pen
                // pressure (slow = wide), scaled by the docker's setting.
                let mut cum = vec![0.0];
                let mut dists = vec![0.0];
                for w in pts.windows(2) {
                    let d = (w[1] - w[0]).hypot();
                    dists.push(d);
                    cum.push(cum.last().copied().unwrap_or(0.0) + d);
                }
                let total = cum.last().copied().unwrap_or(0.0).max(1e-9);
                let max = dists.iter().cloned().fold(0.0, f64::max).max(1e-9);
                let pressure = self.media_pressure;
                let at_point = |t: f64| {
                    let i = cum.partition_point(|c| *c < t * total).min(dists.len() - 1);
                    ((1.0 - dists[i] / max) * 0.7 + 0.3) * (0.5 + 0.5 * pressure)
                };
                st.widths = widths(EXPRESSION_NODES, &at_point);
            }
            MediaMode::Sprayer => return None,
        }
        Some(st)
    }

    /// The name a stroke of the current mode gets.
    fn media_mode_name(&self) -> &'static str {
        match self.media_mode {
            MediaMode::Calligraphic => "Calligraphic",
            MediaMode::Preset => "Preset stroke",
            MediaMode::Brush => "Brush stroke",
            MediaMode::Expression => "Expression stroke",
            MediaMode::Sprayer => "Sprayed object",
        }
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
            // The curve keeps its nodes and takes the brush as its outline.
            let color = c.stroke.as_ref().map_or(Color::BLACK, |st| st.color);
            let pts = tracedraw_core::effects::resample(&c.page_path(), 120);
            if let Some(stroke) = self.media_stroke(&pts, color) {
                new_sel.push(c.id);
                cmds.push(Command::SetStroke {
                    shapes: vec![c.id],
                    stroke: Some(stroke),
                });
                continue;
            }
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

    /// A brush stroke stays the path drawn, open and editable, with the
    /// brush as its live outline (#829): its nodes are few curves, and it
    /// is drawn as wide as the brush.
    #[test]
    fn a_brush_stroke_is_a_live_outline_on_its_path() {
        let mut app = App::headless();
        let pts: Vec<Point> = (0..=200)
            .map(|i| {
                let t = i as f64 / 200.0 * std::f64::consts::PI;
                Point::new(20.0 + 60.0 * t.cos(), 100.0 + 40.0 * t.sin())
            })
            .collect();
        app.media_width = 8.0;
        for mode in [
            MediaMode::Preset,
            MediaMode::Brush,
            MediaMode::Calligraphic,
            MediaMode::Expression,
        ] {
            app.media_mode = mode;
            let shapes = app.media_shapes(&pts);
            assert_eq!(shapes.len(), 1, "{mode:?}");
            let s = &shapes[0];
            let ShapeKind::Path { path, closed } = &s.kind else {
                panic!("{mode:?}: {:?}", s.kind);
            };
            assert!(!closed, "{mode:?}");
            assert!(
                path.elements().len() < 60,
                "{mode:?}: {}",
                path.elements().len()
            );
            let st = s.stroke.as_ref().expect("outline");
            match mode {
                MediaMode::Calligraphic => assert!(st.stretch < 0.5),
                _ => assert!(!st.widths.is_empty(), "{mode:?}"),
            }
            let band = tracedraw_core::shaping::outline_band(path, st, 1.0).expect("band");
            let reach = band.bounding_box().height() - path.bounding_box().height();
            assert!(reach > 1.0 && reach < 9.0, "{mode:?}: {reach}");
        }
        // The sprayer still sprays objects.
        app.media_mode = MediaMode::Sprayer;
        assert!(app.media_shapes(&pts).len() > 1);
    }

    /// Apply to curve keeps the curve, with the brush as its outline.
    #[test]
    fn apply_to_curve_keeps_the_curve() {
        let mut app = App::headless();
        let mut line = BezPath::new();
        line.move_to(Point::new(10.0, 10.0));
        line.curve_to(
            Point::new(40.0, 0.0),
            Point::new(60.0, 40.0),
            Point::new(90.0, 20.0),
        );
        let id = app
            .new_shape(ShapeKind::Path {
                path: line.clone(),
                closed: false,
            })
            .expect("curve");
        app.select(vec![id]);
        app.media_mode = MediaMode::Preset;
        app.apply_media_to_selection();
        let s = app.doc().find_shape(id).expect("the curve stays");
        assert!(
            matches!(&s.kind, ShapeKind::Path { path, .. } if path.elements() == line.elements())
        );
        assert!(!s.stroke.as_ref().expect("outline").widths.is_empty());
        assert_eq!(app.selection, vec![id]);
    }
}

/// How long a page of brush strokes takes to draw (#829): `cargo test
/// --release -p tracedraw -- --ignored frame_times_of_brush_strokes
/// --nocapture`.
#[cfg(test)]
#[test]
#[ignore]
fn frame_times_of_brush_strokes() {
    for mode in [
        MediaMode::Brush,
        MediaMode::Preset,
        MediaMode::Expression,
        MediaMode::Calligraphic,
    ] {
        let mut app = App::headless();
        app.media_mode = mode;
        app.media_width = 6.0;
        let layer = app.active_layer().expect("layer");
        let mut cmds = Vec::new();
        for k in 0..200 {
            let pts: Vec<Point> = (0..=150)
                .map(|i| {
                    let t = i as f64 / 150.0;
                    Point::new(
                        10.0 + 180.0 * t,
                        10.0 + (k as f64) * 1.4 + 6.0 * (t * 9.0).sin(),
                    )
                })
                .collect();
            for s in app.media_shapes(&pts) {
                cmds.push(Command::AddShape { layer, shape: s });
            }
        }
        app.engine.run_batch("strokes", &cmds).expect("add");
        let doc = app.doc().clone();
        let page = doc.pages[0].id;
        let opts = tracedraw_render::RenderOptions {
            width: 840,
            height: 1188,
            view: tracedraw_render::ViewTransform {
                zoom: 4.0,
                origin_x: 0.0,
                origin_y: 1188.0,
            },
            ..Default::default()
        };
        for pass in 0..2 {
            let t = std::time::Instant::now();
            let mut n = 0;
            for s in doc.pages[0].layers.iter().flat_map(|l| &l.shapes) {
                if let Some(st) = &s.stroke {
                    n += tracedraw_core::shaping::outline_band(&s.page_path(), st, 1.0)
                        .map_or(0, |b| b.elements().len());
                }
            }
            eprintln!(
                "{mode:?} bands pass {pass}: {:?}, {n} elements",
                t.elapsed()
            );
        }
        let t = std::time::Instant::now();
        for _ in 0..3 {
            tracedraw_render::render_page(&doc, page, &opts);
        }
        eprintln!(
            "{mode:?}: 200 strokes drawn in {:?} a page",
            t.elapsed() / 3
        );
    }
}
