//! Dimension lines. A dimension is a group of its lines, its arrowheads
//! and its label, with what it measures and its style in the group's
//! object data (`dimension`), so it can be drawn again: when its style
//! changes, and when it is moved, scaled, turned or skewed, which bakes
//! the transform into the measured points so the value follows the new
//! size (dynamic dimensioning) and the label stays upright text.
//!
//! Linear dimensions (Parallel, Horizontal or Vertical, Segment) measure
//! the distance between two points along their line; angular ones the
//! angle at a centre between two points. The label is the value in the
//! style's units through the drawing scale (degrees for angles), with the
//! chosen decimals, the units' sign when shown, a prefix and a suffix; it
//! sits above the dimension line, in a gap in it, or below.

use crate::app::{App, Units};
use crate::i18n::tr;
use serde::{Deserialize, Serialize};
use tracedraw_core::{
    document::{Shape, ShapeKind, TextAlign, TextSpan},
    geometry::{Affine, BezPath, Point, Rect, Shape as _, Vec2},
    Color, Command, Fill, ShapeId, Stroke,
};

/// The object data key holding a dimension.
pub const DATA_KEY: &str = "dimension";

/// Where the label sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextPosition {
    #[default]
    Above,
    Centered,
    Below,
}

impl TextPosition {
    pub const ALL: [TextPosition; 3] = [
        TextPosition::Above,
        TextPosition::Centered,
        TextPosition::Below,
    ];

    pub fn key(self) -> &'static str {
        match self {
            TextPosition::Above => "dimension.text_above",
            TextPosition::Centered => "dimension.text_centered",
            TextPosition::Below => "dimension.text_below",
        }
    }
}

/// How a dimension's label reads and sits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DimensionStyle {
    /// Decimals, 0 to 6.
    pub precision: u8,
    /// A unit id (`Units::id`); empty for the drawing's units.
    pub units: String,
    pub show_units: bool,
    pub prefix: String,
    pub suffix: String,
    pub text_position: TextPosition,
    /// Label size, points.
    pub text_size: f64,
}

impl Default for DimensionStyle {
    fn default() -> Self {
        DimensionStyle {
            precision: 2,
            units: String::new(),
            show_units: true,
            prefix: String::new(),
            suffix: String::new(),
            text_position: TextPosition::Above,
            text_size: 10.0,
        }
    }
}

/// What a dimension measures, in its group's space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Measure {
    /// From `a` to `b`, the dimension line through `at`.
    Linear { a: Point, b: Point, at: Point },
    /// The angle at `center` from `a` to `b` (counterclockwise), the arc
    /// through `at`.
    Angular {
        center: Point,
        a: Point,
        b: Point,
        at: Point,
    },
}

/// A dimension: what it measures and how it reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dimension {
    pub measure: Measure,
    #[serde(default)]
    pub style: DimensionStyle,
}

/// Arrowheads: length and half width, mm.
const ARROW_LEN: f64 = 3.0;
const ARROW_HALF: f64 = 1.0;
/// Gap between the label and its line, and how far extension lines run
/// past the dimension line, mm.
const GAP: f64 = 1.0;
const OVERSHOOT: f64 = 2.0;

fn finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

impl Dimension {
    /// The dimension a shape holds (a group made by the dimension tools).
    pub fn of(shape: &Shape) -> Option<Dimension> {
        if !matches!(shape.kind, ShapeKind::Group { .. }) {
            return None;
        }
        let (_, v) = shape.data.iter().find(|(k, _)| k == DATA_KEY)?;
        let d: Dimension = serde_json::from_str(v).ok()?;
        let ok = match d.measure {
            Measure::Linear { a, b, at } => finite(a) && finite(b) && finite(at),
            Measure::Angular { center, a, b, at } => {
                finite(center) && finite(a) && finite(b) && finite(at)
            }
        };
        ok.then_some(d)
    }

    /// The same dimension with its points moved through `t`.
    pub fn transformed(&self, t: Affine) -> Dimension {
        let measure = match self.measure {
            Measure::Linear { a, b, at } => Measure::Linear {
                a: t * a,
                b: t * b,
                at: t * at,
            },
            Measure::Angular { center, a, b, at } => Measure::Angular {
                center: t * center,
                a: t * a,
                b: t * b,
                at: t * at,
            },
        };
        Dimension {
            measure,
            style: self.style.clone(),
        }
    }

    /// The measured value: page millimetres for a length, degrees for an
    /// angle.
    pub fn value(&self) -> f64 {
        match self.measure {
            Measure::Linear { a, b, .. } => (b - a).hypot(),
            Measure::Angular { center, a, b, .. } => sweep(center, a, b).to_degrees(),
        }
    }

    /// The label: prefix, value, units, suffix.
    pub fn label(&self, drawing_units: Units) -> String {
        let s = &self.style;
        let decimals = s.precision.min(6) as usize;
        let number = match self.measure {
            Measure::Linear { .. } => {
                let units = if s.units.is_empty() {
                    drawing_units
                } else {
                    Units::from_id(&s.units)
                };
                let v = units.world_from_mm(self.value());
                let unit = if s.show_units {
                    format!(" {}", units.short())
                } else {
                    String::new()
                };
                format!("{v:.decimals$}{unit}")
            }
            Measure::Angular { .. } => {
                let unit = if s.show_units { "\u{b0}" } else { "" };
                format!("{:.decimals$}{unit}", self.value())
            }
        };
        format!("{}{}{}", s.prefix, number, s.suffix)
    }

    /// The group's children: lines, arrowheads, label (ids from `ids`).
    pub fn shapes(&self, ids: [ShapeId; 3], font: &str, drawing_units: Units) -> [Shape; 3] {
        let text = self.label(drawing_units);
        let mut label = Shape::new(
            ids[2],
            ShapeKind::Text {
                spans: vec![TextSpan::new(
                    text.clone(),
                    font.to_string(),
                    self.style.text_size.clamp(1.0, 500.0),
                )],
                origin: Point::ZERO,
                frame: None,
                align: TextAlign::Left,
                para: Default::default(),
                on_path: None,
            },
        );
        label.fill = Fill::Solid(Color::BLACK);
        label.stroke = None;
        // The label's own box: its width and how far it reaches below and
        // above its baseline.
        let tb = label.local_path().bounding_box();
        let tb = if tb.width().is_finite() && tb.height().is_finite() && tb.width() > 0.0 {
            tb
        } else {
            // No outlines (no font): a box from the size alone.
            let h = self.style.text_size * 25.4 / 72.0;
            Rect::new(0.0, 0.0, h * 0.55 * text.chars().count() as f64, h * 0.7)
        };
        let mut lines = BezPath::new();
        let mut arrows = BezPath::new();
        let mut arrow = |tip: Point, back_dir: Vec2| {
            let n = Vec2::new(-back_dir.y, back_dir.x);
            let back = tip + back_dir * ARROW_LEN;
            arrows.move_to(tip);
            arrows.line_to(back + n * ARROW_HALF);
            arrows.line_to(back - n * ARROW_HALF);
            arrows.close_path();
        };
        let pos = self.style.text_position;
        match self.measure {
            Measure::Linear { a, b, at } => {
                let len = (b - a).hypot();
                let dir = if len > 1e-9 {
                    (b - a) / len
                } else {
                    Vec2::new(1.0, 0.0)
                };
                let n = Vec2::new(-dir.y, dir.x);
                let off = (at - a).dot(n);
                let side = if off < 0.0 { -n } else { n };
                let (a2, b2) = (a + n * off, b + n * off);
                // Extension lines, past the dimension line.
                for (p, q) in [(a, a2), (b, b2)] {
                    lines.move_to(p);
                    lines.line_to(q + side * OVERSHOOT);
                }
                // The label reads left to right, upright.
                let read = if dir.x < -1e-9 || (dir.x.abs() <= 1e-9 && dir.y < 0.0) {
                    -dir
                } else {
                    dir
                };
                let up = Vec2::new(-read.y, read.x);
                let mid = a2.midpoint(b2);
                let half = tb.width() / 2.0;
                let baseline = match pos {
                    TextPosition::Above => mid + up * (GAP - tb.y0),
                    TextPosition::Below => mid - up * (GAP + tb.y1),
                    TextPosition::Centered => mid - up * ((tb.y0 + tb.y1) / 2.0),
                };
                // The dimension line, with a gap for a centred label.
                if pos == TextPosition::Centered && len > 2.0 * (half + GAP) {
                    let (g1, g2) = (mid - dir * (half + GAP), mid + dir * (half + GAP));
                    lines.move_to(a2);
                    lines.line_to(g1);
                    lines.move_to(g2);
                    lines.line_to(b2);
                } else {
                    lines.move_to(a2);
                    lines.line_to(b2);
                }
                arrow(a2, dir);
                arrow(b2, -dir);
                let origin = baseline - read * (tb.x0 + half);
                label.transform =
                    Affine::translate(origin.to_vec2()) * Affine::rotate(read.atan2());
            }
            Measure::Angular { center, a, b, at } => {
                let a0 = (a - center).atan2();
                let sw = sweep(center, a, b);
                let r = (at - center).hypot().max(1.0);
                let ray = |t: f64| Vec2::new(t.cos(), t.sin());
                for t in [a0, a0 + sw] {
                    lines.move_to(center);
                    lines.line_to(center + ray(t) * (r + OVERSHOOT));
                }
                let mid = a0 + sw / 2.0;
                let (w, h) = (tb.width(), tb.height());
                // A centred label leaves a gap in the arc.
                let gap = if pos == TextPosition::Centered {
                    ((w / 2.0 + GAP) / r).min(sw / 2.0)
                } else {
                    0.0
                };
                let steps = 48;
                let mut drawing = false;
                for i in 0..=steps {
                    let t = a0 + sw * i as f64 / steps as f64;
                    let q = center + ray(t) * r;
                    if gap > 0.0 && (t - mid).abs() < gap {
                        drawing = false;
                        continue;
                    }
                    if drawing {
                        lines.line_to(q);
                    } else {
                        lines.move_to(q);
                        drawing = true;
                    }
                }
                // Arrowheads along the arc at both ends.
                if sw > 1e-6 {
                    arrow(center + ray(a0) * r, Vec2::new(-a0.sin(), a0.cos()));
                    let a1 = a0 + sw;
                    arrow(center + ray(a1) * r, Vec2::new(a1.sin(), -a1.cos()));
                }
                // The label upright, its middle out from, on or inside the
                // arc.
                let reach = (w * mid.cos().abs() + h * mid.sin().abs()) / 2.0;
                let p = match pos {
                    TextPosition::Above => center + ray(mid) * (r + GAP + reach),
                    TextPosition::Centered => center + ray(mid) * r,
                    TextPosition::Below => center + ray(mid) * (r - GAP - reach).max(0.0),
                };
                let origin = p - Vec2::new(tb.x0 + w / 2.0, (tb.y0 + tb.y1) / 2.0);
                label.transform = Affine::translate(origin.to_vec2());
            }
        }
        let mut line = Shape::new(
            ids[0],
            ShapeKind::Path {
                path: lines,
                closed: false,
            },
        );
        line.fill = Fill::None;
        line.stroke = Some(Stroke::hairline(Color::BLACK));
        let mut heads = Shape::new(
            ids[1],
            ShapeKind::Path {
                path: arrows,
                closed: true,
            },
        );
        heads.fill = Fill::Solid(Color::BLACK);
        heads.stroke = None;
        [line, heads, label]
    }
}

/// The counterclockwise angle at `center` from `a` to `b`, 0 to a turn.
fn sweep(center: Point, a: Point, b: Point) -> f64 {
    let s = (b - center).atan2() - (a - center).atan2();
    if s < 0.0 {
        s + std::f64::consts::TAU
    } else {
        s
    }
}

fn with_data(mut data: Vec<(String, String)>, d: &Dimension) -> Vec<(String, String)> {
    data.retain(|(k, _)| k != DATA_KEY);
    if let Ok(v) = serde_json::to_string(d) {
        data.push((DATA_KEY.into(), v));
    }
    data
}

impl App {
    /// A new dimension on the active layer, in the bar's style, one undo
    /// step.
    pub fn add_dimension(&mut self, measure: Measure, label: &'static str) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let d = Dimension {
            measure,
            style: self.dimension_style.clone(),
        };
        let ids = [
            self.engine.new_shape_id(),
            self.engine.new_shape_id(),
            self.engine.new_shape_id(),
        ];
        let children = d.shapes(ids, &self.text_font, self.units);
        let group = self.engine.new_shape_id();
        let mut g = Shape::new(
            group,
            ShapeKind::Group {
                children: children.to_vec(),
            },
        );
        g.data = with_data(Vec::new(), &d);
        if let Err(e) = self
            .engine
            .run_batch(label, &[Command::AddShape { layer, shape: g }])
        {
            self.status = e.to_string();
            return;
        }
        self.select(vec![group]);
    }

    /// Parallel, Horizontal or Vertical and Segment dimensions: from `a` to
    /// `b`, the line through `offset_at`.
    pub fn create_dimension(&mut self, a: Point, b: Point, offset_at: Point) {
        self.add_dimension(
            Measure::Linear {
                a,
                b,
                at: offset_at,
            },
            "Dimension",
        );
    }

    /// An angular dimension: at `center` from `a` to `b`, the arc through
    /// `label_at`.
    pub fn create_angular_dimension(&mut self, center: Point, a: Point, b: Point, label_at: Point) {
        self.add_dimension(
            Measure::Angular {
                center,
                a,
                b,
                at: label_at,
            },
            "Angular Dimension",
        );
    }

    /// The commands that draw dimension group `g` (as it is in its
    /// parent) again from `d`, its own transform baked in.
    fn redraw_commands(&mut self, g: &Shape, d: &Dimension) -> Vec<Command> {
        let ids = match &g.kind {
            ShapeKind::Group { children } if children.len() == 3 => {
                [children[0].id, children[1].id, children[2].id]
            }
            _ => [
                self.engine.new_shape_id(),
                self.engine.new_shape_id(),
                self.engine.new_shape_id(),
            ],
        };
        let mut cmds = Vec::new();
        let t = g.transform;
        let d = if t != Affine::IDENTITY && t.determinant().abs() > 1e-12 {
            // The group's transform goes into the points, the group back to
            // none (in page space: about its parent).
            let parent = self
                .doc()
                .parent_transform(g.id)
                .unwrap_or(Affine::IDENTITY);
            cmds.push(Command::TransformShapes {
                shapes: vec![g.id],
                transform: parent * t.inverse() * parent.inverse(),
            });
            d.transformed(t)
        } else {
            d.clone()
        };
        let children = d.shapes(ids, &self.text_font, self.units);
        cmds.push(Command::SetShapeKind {
            shape: g.id,
            kind: ShapeKind::Group {
                children: children.to_vec(),
            },
        });
        cmds.push(Command::SetObjectData {
            shape: g.id,
            data: with_data(g.data.clone(), &d),
        });
        cmds
    }

    /// The selected dimensions.
    pub fn selected_dimensions(&self) -> Vec<(ShapeId, Dimension)> {
        self.selection
            .iter()
            .filter_map(|id| {
                let s = self.doc().find_shape(*id)?;
                Dimension::of(s).map(|d| (*id, d))
            })
            .collect()
    }

    /// Change the style new dimensions take and the selected ones have,
    /// one undo step for the selected ones.
    pub fn set_dimension_style(&mut self, f: impl Fn(&mut DimensionStyle)) {
        f(&mut self.dimension_style);
        let mut cmds = Vec::new();
        for (id, mut d) in self.selected_dimensions() {
            let before = d.style.clone();
            f(&mut d.style);
            if d.style == before {
                continue;
            }
            let Some(g) = self.doc().find_shape(id).cloned() else {
                continue;
            };
            cmds.extend(self.redraw_commands(&g, &d));
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Dimension Style", &cmds) {
            self.status = e.to_string();
        }
    }

    /// The style the bar shows: the first selected dimension's, else the
    /// one new dimensions take.
    pub fn dimension_style_shown(&self) -> DimensionStyle {
        self.selected_dimensions()
            .into_iter()
            .next()
            .map(|(_, d)| d.style)
            .unwrap_or_else(|| self.dimension_style.clone())
    }

    /// Dynamic dimensioning: dimensions moved, scaled, turned or skewed
    /// since the last look are drawn again with the transform in their
    /// points, as part of the step that moved them. Runs once per change
    /// of the drawing.
    pub fn bake_dimensions(&mut self) {
        let revision = self.engine.revision();
        if revision == self.dimensions_checked {
            return;
        }
        fn walk(s: &Shape, out: &mut Vec<Shape>) {
            if Dimension::of(s).is_some() {
                if s.transform != Affine::IDENTITY {
                    out.push(s.clone());
                }
                return;
            }
            match &s.kind {
                ShapeKind::Group { children } => children.iter().for_each(|c| walk(c, out)),
                ShapeKind::ClipFrame { frame, contents } => {
                    walk(frame, out);
                    contents.iter().for_each(|c| walk(c, out));
                }
                _ => {}
            }
        }
        let mut moved = Vec::new();
        for layer in self.doc().all_layers() {
            for s in &layer.shapes {
                walk(s, &mut moved);
            }
        }
        for g in moved {
            if let Some(d) = Dimension::of(&g) {
                for cmd in self.redraw_commands(&g, &d) {
                    if let Err(e) = self.engine.amend(&cmd) {
                        self.status = e.to_string();
                        break;
                    }
                }
            }
        }
        self.dimensions_checked = self.engine.revision();
    }
}

impl App {
    /// The nearest segment of an object on the page within 6 pixels of
    /// `p`: its two ends (page space).
    pub fn segment_ends_at(&self, p: Point) -> Option<(Point, Point)> {
        use tracedraw_core::geometry::{ParamCurve, ParamCurveNearest};
        let tol = 6.0 / self.view.zoom as f64;
        let mut best: Option<(Point, Point, f64)> = None;
        for layer in self.page_layers() {
            if !layer.visible || layer.locked {
                continue;
            }
            for s in &layer.shapes {
                if !s.visible || !self.visual_bounds(s).inflate(tol, tol).contains(p) {
                    continue;
                }
                for seg in s.page_path().segments() {
                    let d = seg.nearest(p, 1e-6).distance_sq.sqrt();
                    if d <= tol && best.is_none_or(|b| d < b.2) {
                        best = Some((seg.start(), seg.end(), d));
                    }
                }
            }
        }
        best.map(|(a, b, _)| (a, b))
    }

    /// The two points a linear dimension tool measures between, from the
    /// points clicked: Horizontal or Vertical keeps the larger of the two
    /// spans.
    fn measured_pair(&self, a: Point, b: Point) -> (Point, Point) {
        if self.tool == crate::tools::Tool::HorizontalVerticalDimension {
            if (b.x - a.x).abs() >= (b.y - a.y).abs() {
                (a, Point::new(b.x, a.y))
            } else {
                (a, Point::new(a.x, b.y))
            }
        } else {
            (a, b)
        }
    }

    /// The point the dimension tools take for `p`: with the constrain key
    /// (`ctrl`) held while the second point of a linear dimension, or a
    /// side of an angular one, is given, the leg from the first point (the
    /// vertex) keeps to steps of the constrain angle, as the curve tools
    /// do.
    pub fn dimension_point(&self, p: Point, ctrl: bool) -> Point {
        if !ctrl {
            return p;
        }
        let angular = self.tool == crate::tools::Tool::AngularDimension;
        let from = match (angular, &self.dimension_points[..]) {
            (false, [a]) => *a,
            (true, [center]) | (true, [center, _]) => *center,
            _ => return p,
        };
        crate::interaction::constrain_from(from, p, self.settings.constrain_angle)
    }

    /// The dimension the tool would make with the pointer at `at`, while
    /// its points are being given (for the preview).
    pub fn dimension_preview(&self, at: Point) -> Option<Dimension> {
        let pts = &self.dimension_points;
        let measure = match self.tool {
            crate::tools::Tool::AngularDimension => match pts[..] {
                [center, a, b] => Measure::Angular { center, a, b, at },
                _ => return None,
            },
            crate::tools::Tool::ParallelDimension
            | crate::tools::Tool::HorizontalVerticalDimension
            | crate::tools::Tool::SegmentDimension => match pts[..] {
                [a, b] => {
                    let (a, b) = self.measured_pair(a, b);
                    Measure::Linear { a, b, at }
                }
                _ => return None,
            },
            _ => return None,
        };
        Some(Dimension {
            measure,
            style: self.dimension_style.clone(),
        })
    }

    /// The dimension tools' clicks and drags. Linear tools: a drag from
    /// one point to the other (or two clicks), then a click where the
    /// dimension line goes; the Segment tool's first click on a segment
    /// takes its two ends. Angular: the vertex, a point on each side, then
    /// where the arc goes.
    pub fn dimension_input(&mut self, response: &egui::Response, p: Point) {
        use crate::tools::Tool;
        use egui::PointerButton;
        let p = self.snap_point(p);
        let ctrl = response
            .ctx
            .input(|i| i.modifiers.command || i.modifiers.ctrl);
        let p = self.dimension_point(p, ctrl);
        let angular = self.tool == Tool::AngularDimension;
        if !angular {
            if response.drag_started_by(PointerButton::Primary) && self.dimension_points.is_empty()
            {
                // Where the button went down, not where the drag was seen.
                let origin = response
                    .ctx
                    .input(|i| i.pointer.press_origin())
                    .map(|o| self.snap_point(self.view.to_page(o)))
                    .unwrap_or(p);
                self.dimension_points.push(origin);
                self.dimension_dragging = true;
                return;
            }
            if response.drag_stopped() && self.dimension_dragging {
                self.dimension_dragging = false;
                let start = self.dimension_points.first().copied();
                if start.is_some_and(|s| (p - s).hypot() > 1e-6) {
                    self.dimension_points.push(p);
                } else {
                    self.dimension_points.clear();
                }
                return;
            }
        }
        if !response.clicked_by(PointerButton::Primary) {
            return;
        }
        if self.tool == Tool::SegmentDimension && self.dimension_points.is_empty() {
            if let Some((a, b)) = self.segment_ends_at(p) {
                self.dimension_points = vec![a, b];
                return;
            }
        }
        self.dimension_points.push(p);
        let pts = self.dimension_points.clone();
        match (angular, &pts[..]) {
            (true, [center, a, b, at]) => {
                self.dimension_points.clear();
                self.create_angular_dimension(*center, *a, *b, *at);
            }
            (false, [a, b, at]) => {
                self.dimension_points.clear();
                let (a, b) = self.measured_pair(*a, *b);
                self.create_dimension(a, b, *at);
            }
            _ => {}
        }
    }
}

/// The units list of the dimension bar: the drawing's units first.
pub fn units_choices() -> Vec<(String, String)> {
    let mut out = vec![(String::new(), tr("dimension.drawing_units"))];
    out.extend(Units::ALL.iter().map(|u| (u.id().to_string(), u.label())));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label_of(app: &App, id: ShapeId) -> String {
        let g = app.doc().find_shape(id).expect("group");
        let ShapeKind::Group { children } = &g.kind else {
            panic!("not a group");
        };
        match &children[2].kind {
            ShapeKind::Text { spans, .. } => spans.iter().map(|s| s.text.as_str()).collect(),
            other => panic!("{other:?}"),
        }
    }

    /// The preview follows the pointer; Horizontal or Vertical keeps the
    /// larger span; a segment's ends are found under the pointer.
    #[test]
    fn previews_and_segments() {
        let mut app = App::headless();
        app.view.zoom = 1.0;
        app.tool = crate::tools::Tool::HorizontalVerticalDimension;
        assert!(app.dimension_preview(Point::ZERO).is_none());
        app.dimension_points = vec![Point::new(0.0, 0.0), Point::new(30.0, 4.0)];
        let d = app
            .dimension_preview(Point::new(10.0, 20.0))
            .expect("preview");
        assert_eq!(
            d.measure,
            Measure::Linear {
                a: Point::new(0.0, 0.0),
                b: Point::new(30.0, 0.0),
                at: Point::new(10.0, 20.0)
            }
        );
        app.dimension_points.clear();
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(10.0, 10.0, 50.0, 30.0),
            radius: 0.0,
            corners: None,
        })
        .expect("shape");
        let (a, b) = app
            .segment_ends_at(Point::new(30.0, 10.5))
            .expect("segment");
        let (lo, hi) = if a.x < b.x { (a, b) } else { (b, a) };
        assert_eq!((lo, hi), (Point::new(10.0, 10.0), Point::new(50.0, 10.0)));
        assert!(app.segment_ends_at(Point::new(30.0, 20.0)).is_none());
    }

    #[test]
    fn labels_follow_the_style() {
        let d = Dimension {
            measure: Measure::Linear {
                a: Point::new(0.0, 0.0),
                b: Point::new(25.4, 0.0),
                at: Point::new(0.0, 10.0),
            },
            style: DimensionStyle::default(),
        };
        assert_eq!(d.label(Units::Millimeters), "25.40 mm");
        let inches = Dimension {
            style: DimensionStyle {
                units: "in".into(),
                precision: 3,
                prefix: "W ".into(),
                suffix: " max".into(),
                ..Default::default()
            },
            ..d.clone()
        };
        assert_eq!(inches.label(Units::Millimeters), "W 1.000 \" max");
        let bare = Dimension {
            style: DimensionStyle {
                show_units: false,
                precision: 0,
                ..Default::default()
            },
            ..d
        };
        assert_eq!(bare.label(Units::Millimeters), "25");
        let angle = Dimension {
            measure: Measure::Angular {
                center: Point::ZERO,
                a: Point::new(10.0, 0.0),
                b: Point::new(0.0, 10.0),
                at: Point::new(5.0, 5.0),
            },
            style: DimensionStyle {
                precision: 1,
                ..Default::default()
            },
        };
        assert_eq!(angle.label(Units::Millimeters), "90.0\u{b0}");
    }

    /// A dimension is a group that knows what it measures; its style is
    /// changed from the bar; scaling it changes the value it shows and
    /// leaves the group without a transform; undo takes both back.
    #[test]
    fn dimensions_redraw_for_their_style_and_their_size() {
        let mut app = App::headless();
        app.create_dimension(
            Point::new(0.0, 0.0),
            Point::new(40.0, 0.0),
            Point::new(20.0, 10.0),
        );
        let id = app.selection[0];
        assert_eq!(label_of(&app, id), "40.00 mm");
        // The dimension line runs at y 10, the extension lines past it.
        let g = app.doc().find_shape(id).cloned().expect("group");
        assert!(g.bounds().y1 >= 12.0 - 1e-6, "{:?}", g.bounds());
        app.set_dimension_style(|s| {
            s.precision = 0;
            s.suffix = " long".into();
        });
        assert_eq!(label_of(&app, id), "40 mm long");
        assert_eq!(app.dimension_style.precision, 0);
        // Scale it by 2 about the origin: dynamic dimensioning.
        app.run(Command::TransformShapes {
            shapes: vec![id],
            transform: Affine::scale(2.0),
        });
        let depth = app.engine.history_labels().0.len();
        app.bake_dimensions();
        assert_eq!(app.engine.history_labels().0.len(), depth);
        assert_eq!(label_of(&app, id), "80 mm long");
        let g = app.doc().find_shape(id).cloned().expect("group");
        assert_eq!(g.transform, Affine::IDENTITY);
        match Dimension::of(&g).expect("dimension").measure {
            Measure::Linear { b, .. } => assert_eq!(b, Point::new(80.0, 0.0)),
            other => panic!("{other:?}"),
        }
        app.undo();
        app.bake_dimensions();
        assert_eq!(label_of(&app, id), "40 mm long");
        // An angular dimension and a centred label.
        app.dimension_style.text_position = TextPosition::Centered;
        app.create_angular_dimension(
            Point::new(100.0, 0.0),
            Point::new(110.0, 0.0),
            Point::new(100.0, 10.0),
            Point::new(108.0, 8.0),
        );
        let a = app.selection[0];
        assert_eq!(label_of(&app, a), "90\u{b0} long");
        // Something that is not a dimension is not one.
        assert!(Dimension::of(&Shape::new(
            ShapeId(1),
            ShapeKind::Group { children: vec![] }
        ))
        .is_none());
    }
}
