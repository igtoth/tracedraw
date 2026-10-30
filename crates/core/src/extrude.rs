//! The extruded body of an object: side faces between its outline and a
//! second copy of it, and that copy as the far or the near face.
//!
//! - Back parallel: the copy is the outline moved by `vector`, behind the
//!   object. Front parallel: the same copy in front, the object behind.
//! - Small back and Big back: the copy is the outline drawn toward the
//!   vanishing point `v` by `depth` (a share 0 to 0.99 of the way: each
//!   point `p` goes to `p + (v - p) * depth`), or away from it
//!   (`p + (p - v) * depth`), behind the object. Small front and Big
//!   front: the same copies in front, the object behind.
//!
//! Every subpath of the outline gives faces: one quad for each flattened
//! edge (the closing edge of a closed subpath included) to the copy of
//! that edge. A face shows when it faces the viewer: its normal (away from
//! the filled side of the edge) points along the way the body runs from
//! the object at that face (back types) or against it (front types).
//! Faces are drawn farthest first: along the vector for parallel bodies,
//! by their distance from the vanishing point for the others.
//!
//! Colour: the object's fill, one solid colour, or a shade from one
//! colour at the object's side of each face to another at the copy's
//! side; each face is darkened by how far its normal turns from the light
//! (`0.55 + 0.45 * k * (n . l + 1) / 2 + 0.45 * (1 - k)`, `k` the
//! intensity over 100). The far face takes the darkest colour; a near
//! face looks like the object (its fill and outline).

use crate::document::{Shape, ShapeKind};
use crate::geometry::{Affine, BezPath, PathEl, Point, Vec2};
use crate::style::{lerp_color, Fill};
use crate::Color;
use serde::{Deserialize, Serialize};

/// How the body runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtrudeKind {
    #[default]
    BackParallel,
    FrontParallel,
    SmallBack,
    BigBack,
    SmallFront,
    BigFront,
}

impl ExtrudeKind {
    pub const ALL: [ExtrudeKind; 6] = [
        ExtrudeKind::SmallBack,
        ExtrudeKind::SmallFront,
        ExtrudeKind::BigBack,
        ExtrudeKind::BigFront,
        ExtrudeKind::BackParallel,
        ExtrudeKind::FrontParallel,
    ];

    /// Runs along a vector (no vanishing point).
    pub fn parallel(self) -> bool {
        matches!(self, ExtrudeKind::BackParallel | ExtrudeKind::FrontParallel)
    }

    /// The copy is in front of the object.
    pub fn front(self) -> bool {
        matches!(
            self,
            ExtrudeKind::FrontParallel | ExtrudeKind::SmallFront | ExtrudeKind::BigFront
        )
    }
}

/// How the body is coloured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Coloring {
    /// The object's fill colour.
    Object,
    Solid(Color),
    /// From the first colour at the object's side to the second at the
    /// copy's.
    Shade(Color, Color),
}

/// What a body is built from, in the object's parent space.
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub kind: ExtrudeKind,
    /// Parallel bodies: from the object to the copy.
    pub vector: Vec2,
    /// The others: the vanishing point and the share of the way, 0 to
    /// 0.99.
    pub vanishing: Point,
    pub depth: f64,
    pub coloring: Coloring,
    /// Degrees (0 = from the right, counterclockwise) and 0 to 100.
    pub light_angle: f64,
    pub light_intensity: f64,
    /// Width of the bevel on the near cap's edge, mm (0: none).
    pub bevel: f64,
    /// Turn in space and further lights (see [`crate::extrude3d`]).
    pub solid: crate::extrude3d::Solid,
}

/// The faces behind the object and in front of it, and what the object
/// itself becomes when the body is turned or bevelled.
#[derive(Debug, Clone, Default)]
pub struct Faces {
    pub below: Vec<Shape>,
    pub above: Vec<Shape>,
    pub main: Option<Shape>,
}

/// Points per subpath before an outline is resampled.
const MAX_POINTS: usize = 400;

fn finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// The flattened subpaths of `path` and whether each closes.
fn polylines(path: &BezPath) -> Vec<(Vec<Point>, bool)> {
    let mut out: Vec<(Vec<Point>, bool)> = Vec::new();
    let mut cur: Vec<Point> = Vec::new();
    let flush = |cur: &mut Vec<Point>, closed: bool, out: &mut Vec<(Vec<Point>, bool)>| {
        let mut pts = std::mem::take(cur);
        // Points that repeat (a curve ending where it started) would make
        // edges with no direction.
        pts.dedup_by(|b, a| (*b - *a).hypot() < 1e-6);
        // An outline that ends where it started is closed (an ellipse has
        // no closing element).
        let ends_at_start = pts.len() > 2 && (pts[0] - pts[pts.len() - 1]).hypot() < 1e-6;
        let closed = closed || ends_at_start;
        if ends_at_start {
            pts.pop();
        }
        if pts.len() >= 2 {
            if pts.len() > MAX_POINTS {
                let step = pts.len() as f64 / MAX_POINTS as f64;
                pts = (0..MAX_POINTS)
                    .map(|i| pts[((i as f64 * step) as usize).min(pts.len() - 1)])
                    .collect();
            }
            out.push((pts, closed));
        }
    };
    crate::geometry::flatten(path, 0.05, &mut |el| match el {
        PathEl::MoveTo(p) => {
            flush(&mut cur, false, &mut out);
            cur.push(p);
        }
        PathEl::LineTo(p) => cur.push(p),
        PathEl::ClosePath => flush(&mut cur, true, &mut out),
        _ => {}
    });
    flush(&mut cur, false, &mut out);
    out.retain(|(pts, _)| pts.iter().all(|p| finite(*p)));
    out
}

/// Twice the signed area (counterclockwise positive).
fn area2(pts: &[Point]) -> f64 {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum()
}

/// Even-odd: is `p` inside the closed polyline `pts`?
fn inside(pts: &[Point], p: Point) -> bool {
    let n = pts.len();
    let mut c = false;
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (pts[i], pts[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            c = !c;
        }
        j = i;
    }
    c
}

fn scale(c: Color, k: f64) -> Color {
    let [r, g, b] = c.to_rgb8();
    Color::rgb8(
        (r as f64 * k).round().clamp(0.0, 255.0) as u8,
        (g as f64 * k).round().clamp(0.0, 255.0) as u8,
        (b as f64 * k).round().clamp(0.0, 255.0) as u8,
    )
}

impl Body {
    /// Where the copy of `p` goes.
    pub fn copy_of(&self, p: Point) -> Point {
        let d = self.depth.clamp(0.0, 0.99);
        match self.kind {
            ExtrudeKind::BackParallel | ExtrudeKind::FrontParallel => p + self.vector,
            ExtrudeKind::SmallBack | ExtrudeKind::SmallFront => p + (self.vanishing - p) * d,
            ExtrudeKind::BigBack | ExtrudeKind::BigFront => p + (p - self.vanishing) * d,
        }
    }

    /// How deep a face with its middle at `m` lies (the larger, the
    /// farther from the viewer).
    fn depth_key(&self, m: Point) -> f64 {
        let to_v = (m - self.vanishing).hypot();
        match self.kind {
            ExtrudeKind::BackParallel => m.to_vec2().dot(self.vector),
            ExtrudeKind::FrontParallel => -m.to_vec2().dot(self.vector),
            ExtrudeKind::SmallBack | ExtrudeKind::BigFront => -to_v,
            ExtrudeKind::BigBack | ExtrudeKind::SmallFront => to_v,
        }
    }

    /// The face over the edge from `a` to `b` and how deep it lies, when
    /// it shows.
    #[allow(clippy::too_many_arguments)]
    fn face(
        &self,
        a: Point,
        b: Point,
        out_right: bool,
        closed: bool,
        sign: f64,
        (from, to, k, light): (Color, Color, f64, Vec2),
        template: &dyn Fn() -> Shape,
    ) -> Option<(f64, Shape)> {
        let (a2, b2) = (self.copy_of(a), self.copy_of(b));
        let e = b - a;
        let len = e.hypot();
        if len < 1e-9 {
            return None;
        }
        let mut normal = Vec2::new(e.y, -e.x) / len;
        if !out_right {
            normal = -normal;
        }
        let run = a2.midpoint(b2) - a.midpoint(b);
        // Open subpaths show both sides.
        if closed && sign * normal.dot(run) <= 0.0 {
            return None;
        }
        let lit =
            (0.55 + 0.45 * k * (normal.dot(light) + 1.0) / 2.0 + 0.45 * (1.0 - k)).clamp(0.2, 1.0);
        let mut s = template();
        let mut quad = BezPath::new();
        quad.move_to(a);
        quad.line_to(b);
        quad.line_to(b2);
        quad.line_to(a2);
        quad.close_path();
        s.kind = ShapeKind::Path {
            path: quad,
            closed: true,
        };
        let (fill, edge) = match self.coloring {
            Coloring::Shade(..) => {
                let ang = run.y.atan2(run.x).to_degrees();
                (
                    Fill::linear(scale(from, lit), scale(to, lit), ang),
                    scale(lerp_color(from, to, 0.5), lit),
                )
            }
            _ => (Fill::Solid(scale(from, lit)), scale(from, lit)),
        };
        s.fill = fill;
        // A hairline of its own colour closes the seams between faces.
        s.stroke = Some(crate::Stroke::hairline(edge));
        Some((self.depth_key(a.midpoint(b2)), s))
    }

    /// The fill of a surface made of `faces` (edge index, depth, face) of
    /// the outline `pts`: the faces' one colour, or a fountain across the
    /// surface through their colours where the light changes along a
    /// curved side; a shade keeps the middle face's fountain.
    fn surface_fill(
        &self,
        faces: &[(usize, f64, Shape)],
        pts: &[Point],
        outline: &BezPath,
        all_round: bool,
        light: Vec2,
    ) -> Fill {
        let middle = faces
            .get(faces.len() / 2)
            .map(|(_, _, s)| s.fill.clone())
            .unwrap_or(Fill::None);
        if matches!(self.coloring, Coloring::Shade(..)) {
            return middle;
        }
        let colors: Vec<(Point, Color)> = faces
            .iter()
            .filter_map(|(i, _, s)| {
                let a = *pts.get(*i)?;
                let b = *pts.get((*i + 1) % pts.len().max(1))?;
                match s.fill {
                    Fill::Solid(c) => Some((a.midpoint(b), c)),
                    _ => None,
                }
            })
            .collect();
        let (Some(first), Some(last)) = (colors.first(), colors.last()) else {
            return middle;
        };
        if colors.iter().all(|(_, c)| c.to_rgb8() == first.1.to_rgb8()) {
            return Fill::Solid(first.1);
        }
        // Across the surface: from the first face to the last, or along
        // the light all round.
        let along = if all_round { light } else { last.0 - first.0 };
        if along.hypot() < 1e-9 {
            return middle;
        }
        let dir = along.normalize();
        let angle = dir.y.atan2(dir.x).to_degrees();
        // Where a point falls on the fountain's span over the outline's
        // box (as the renderer lays a linear fountain out).
        let b = kurbo::Shape::bounding_box(outline);
        let half = (b.width() * dir.x.abs() + b.height() * dir.y.abs()) / 2.0;
        if !(half.is_finite() && half > 1e-9) {
            return middle;
        }
        let at = |p: Point| (((p - b.center()).dot(dir) + half) / (2.0 * half)).clamp(0.0, 1.0);
        // At most a dozen stops, evenly through the faces.
        let step = (colors.len() / 12).max(1);
        let mut stops: Vec<crate::style::Stop> = colors
            .iter()
            .step_by(step)
            .chain(std::iter::once(last))
            .map(|(p, c)| crate::style::Stop::new(at(*p), *c))
            .collect();
        stops.sort_by(|a, b| {
            a.pos
                .partial_cmp(&b.pos)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        stops.dedup_by(|a, b| (a.pos - b.pos).abs() < 1e-6);
        if stops.len() < 2 {
            return Fill::Solid(first.1);
        }
        let mut f =
            crate::style::Fountain::two(crate::style::FountainKind::Linear, first.1, last.1, angle);
        f.stops = stops;
        Fill::Fountain(f)
    }

    /// The body of `shape` (its outline in its parent's space).
    pub fn faces(&self, shape: &Shape) -> Faces {
        let mut out = Faces::default();
        if !(finite(self.vanishing)
            && self.vector.x.is_finite()
            && self.vector.y.is_finite()
            && self.depth.is_finite())
        {
            return out;
        }
        let subs = polylines(&shape.page_path());
        if subs.is_empty() {
            return out;
        }
        if !self.solid.is_plain() || (self.bevel.is_finite() && self.bevel > 0.0) {
            // Each subpath with the outside on the right of travel.
            let oriented: Vec<(Vec<Point>, bool, bool)> = subs
                .iter()
                .enumerate()
                .map(|(i, (pts, c))| {
                    let hole = *c
                        && subs
                            .iter()
                            .enumerate()
                            .filter(|(j, (o, oc))| {
                                *j != i && *oc && o.len() >= 3 && inside(o, pts[0])
                            })
                            .count()
                            % 2
                            == 1;
                    let mut pts = pts.clone();
                    if *c && (area2(&pts) >= 0.0) == hole {
                        pts.reverse();
                    }
                    (pts, *c, hole)
                })
                .collect();
            return crate::extrude3d::faces(self, shape, &oriented);
        }
        let base = shape.fill.preview_color().unwrap_or(Color::WHITE);
        let (from, to) = match self.coloring {
            Coloring::Object => (base, lerp_color(base, Color::BLACK, 0.5)),
            Coloring::Solid(c) => (c, lerp_color(c, Color::BLACK, 0.5)),
            Coloring::Shade(a, b) => (a, b),
        };
        let k = if self.light_intensity.is_finite() {
            (self.light_intensity / 100.0).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let la = if self.light_angle.is_finite() {
            self.light_angle.to_radians()
        } else {
            0.0
        };
        let light = Vec2::new(la.cos(), la.sin());
        let sign = if self.kind.front() { -1.0 } else { 1.0 };
        let template = || {
            let mut s = shape.clone();
            s.effects.clear();
            s.shadow = None;
            s.transform = Affine::IDENTITY;
            if let Some(st) = &mut s.stroke {
                st.width = st.width.min(0.1);
                st.start_arrow = Default::default();
                st.end_arrow = Default::default();
            }
            s
        };
        // Which closed subpaths are holes (inside an odd number of others).
        let holes: Vec<bool> = subs
            .iter()
            .enumerate()
            .map(|(i, (pts, c))| {
                *c && subs
                    .iter()
                    .enumerate()
                    .filter(|(j, (o, oc))| *j != i && *oc && o.len() >= 3 && inside(o, pts[0]))
                    .count()
                    % 2
                    == 1
            })
            .collect();
        let outline_stroke = template().stroke;
        // One face for each flattened edge that shows; the faces between
        // corners make a surface, outlined as one (a curved side is not
        // drawn as a row of slats).
        let corner = 20f64.to_radians().cos();
        let mut surfaces: Vec<(f64, Shape)> = Vec::new();
        for ((pts, is_closed), hole) in subs.iter().zip(holes) {
            let closed = *is_closed;
            // The right of travel is outside a counterclockwise outline.
            let out_right = (area2(pts) >= 0.0) != hole;
            let n = pts.len();
            let edges = if closed { n } else { n - 1 };
            let shown: Vec<Option<(f64, Shape)>> = (0..edges)
                .map(|i| {
                    self.face(
                        pts[i],
                        pts[(i + 1) % n],
                        out_right,
                        closed,
                        sign,
                        (from, to, k, light),
                        &template,
                    )
                })
                .collect();
            let dir = |i: usize| (pts[(i + 1) % n] - pts[i]).normalize();
            let starts = |i: usize| -> bool {
                if shown[i].is_none() {
                    return false;
                }
                let prev = match (i, closed) {
                    (0, true) => Some(edges - 1),
                    (0, false) => None,
                    _ => Some(i - 1),
                };
                match prev {
                    None => true,
                    Some(p) => shown[p].is_none() || dir(p).dot(dir(i)) < corner,
                }
            };
            let runs: Vec<Vec<usize>> = match (0..edges).find(|&i| starts(i)) {
                // A smooth closed outline that shows all round.
                None if edges > 0 && shown.iter().all(|f| f.is_some()) => {
                    vec![(0..edges).collect()]
                }
                None => Vec::new(),
                Some(f) => {
                    let order: Vec<usize> = if closed {
                        (0..edges).map(|k| (f + k) % edges).collect()
                    } else {
                        (f..edges).collect()
                    };
                    let mut runs: Vec<Vec<usize>> = Vec::new();
                    let mut cur: Vec<usize> = Vec::new();
                    for i in order {
                        if (shown[i].is_none() || starts(i)) && !cur.is_empty() {
                            runs.push(std::mem::take(&mut cur));
                        }
                        if shown[i].is_some() {
                            cur.push(i);
                        }
                    }
                    if !cur.is_empty() {
                        runs.push(cur);
                    }
                    runs
                }
            };
            for run in runs {
                let all_round = closed && run.len() == edges;
                let front: Vec<Point> = if all_round {
                    pts.clone()
                } else {
                    let mut v: Vec<Point> = run.iter().map(|&i| pts[i]).collect();
                    if let Some(&last) = run.last() {
                        v.push(pts[(last + 1) % n]);
                    }
                    v
                };
                let mut outline = BezPath::new();
                if all_round {
                    for loop_pts in [
                        front.clone(),
                        front.iter().map(|p| self.copy_of(*p)).collect::<Vec<_>>(),
                    ] {
                        for (j, p) in loop_pts.iter().enumerate() {
                            if j == 0 {
                                outline.move_to(*p);
                            } else {
                                outline.line_to(*p);
                            }
                        }
                        outline.close_path();
                    }
                } else {
                    for (j, p) in front.iter().enumerate() {
                        if j == 0 {
                            outline.move_to(*p);
                        } else {
                            outline.line_to(*p);
                        }
                    }
                    for p in front.iter().rev() {
                        outline.line_to(self.copy_of(*p));
                    }
                    outline.close_path();
                }
                // One object for the surface (#1281): its outline filled
                // with the faces' colours, so a side is one part to paint
                // or colour, not a row of slats.
                let faces: Vec<(usize, f64, Shape)> = run
                    .iter()
                    .filter_map(|&i| shown[i].clone().map(|(k, s)| (i, k, s)))
                    .collect();
                let Some((_, _, middle)) = faces.get(faces.len() / 2) else {
                    continue;
                };
                let key = faces.iter().map(|(_, k, _)| *k).sum::<f64>() / faces.len() as f64;
                let mut surface = middle.clone();
                surface.fill = self.surface_fill(&faces, pts, &outline, all_round, light);
                surface.stroke = match &outline_stroke {
                    Some(st) => Some(st.clone()),
                    None => surface.fill.preview_color().map(crate::Stroke::hairline),
                };
                surface.kind = ShapeKind::Path {
                    path: outline,
                    closed: true,
                };
                surfaces.push((key, surface));
            }
        }
        surfaces.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let faces: Vec<Shape> = surfaces.into_iter().map(|(_, s)| s).collect();
        // The copy: the far face, or the near face looking like the object.
        let mut copy_path = BezPath::new();
        for (pts, is_closed) in &subs {
            for (i, p) in pts.iter().enumerate() {
                let q = self.copy_of(*p);
                if i == 0 {
                    copy_path.move_to(q);
                } else {
                    copy_path.line_to(q);
                }
            }
            if *is_closed {
                copy_path.close_path();
            }
        }
        let any_closed = subs.iter().any(|(_, c)| *c);
        if self.kind.front() {
            let mut near = shape.clone();
            near.effects.clear();
            near.shadow = None;
            near.transform = Affine::IDENTITY;
            near.kind = ShapeKind::Path {
                path: copy_path,
                closed: any_closed,
            };
            out.above.extend(faces);
            out.above.push(near);
        } else {
            // The far face lies behind the object and the sides: where the
            // object does not cover it, a side does. It is drawn only when
            // it can show, through an object without a fill or an open
            // outline, so a filled object's body is just its sides (#1281).
            let opaque_fill = match &shape.fill {
                Fill::Solid(_) => true,
                Fill::Fountain(f) => f.stops.iter().all(|s| s.opacity >= 1.0),
                _ => false,
            };
            let hidden = any_closed
                && subs.iter().all(|(_, c)| *c)
                && opaque_fill
                && shape.opacity >= 1.0
                && shape.opacity_mask.is_none()
                && !shape.effects.iter().any(|e| {
                    matches!(
                        e,
                        crate::live::Effect::Transparency { .. } | crate::live::Effect::Lens(_)
                    )
                });
            if !hidden {
                let mut far = template();
                far.kind = ShapeKind::Path {
                    path: copy_path,
                    closed: any_closed,
                };
                far.fill = if any_closed {
                    Fill::Solid(to)
                } else {
                    Fill::None
                };
                out.below.push(far);
            }
            out.below.extend(faces);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Rect, Shape as _};
    use crate::ShapeId;

    fn square(fill: Fill) -> Shape {
        let mut s = Shape::new(
            ShapeId(1),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = fill;
        s
    }

    fn filled(shapes: &[Shape]) -> Vec<&Shape> {
        shapes.iter().filter(|s| s.fill != Fill::None).collect()
    }

    fn body(kind: ExtrudeKind) -> Body {
        Body {
            kind,
            vector: Vec2::new(10.0, 10.0),
            vanishing: Point::new(60.0, 60.0),
            depth: 0.25,
            coloring: Coloring::Object,
            light_angle: 135.0,
            light_intensity: 50.0,
            bevel: 0.0,
            solid: Default::default(),
        }
    }

    #[test]
    fn back_parallel_shows_the_faces_toward_the_vector() {
        let s = square(Fill::Solid(Color::rgb8(200, 0, 0)));
        let f = body(ExtrudeKind::BackParallel).faces(&s);
        assert!(f.above.is_empty());
        // The top and right faces only (the others face away), each one
        // object with its outline; the far face is hidden behind them.
        assert_eq!(f.below.len(), 2, "two sides");
        assert_eq!(filled(&f.below).len(), 2);
        assert!(f.below.iter().all(|s| s.stroke.is_some()));
        // Lit from the top left: the top face is lighter than the right.
        let shade = |s: &Shape| match s.fill {
            Fill::Solid(c) => c.to_rgb8()[0],
            _ => 0,
        };
        let (top, right): (Vec<&Shape>, Vec<&Shape>) = filled(&f.below)
            .into_iter()
            .partition(|s| s.bounds().y1 > 25.0 && s.bounds().height() < 15.0);
        assert_eq!((top.len(), right.len()), (1, 1));
        assert!(shade(top[0]) > shade(right[0]));
        // Without a fill the far face shows through: first, at (10, 10).
        let hollow = square(Fill::None);
        let f = body(ExtrudeKind::BackParallel).faces(&hollow);
        let far = f.below[0].bounds();
        assert!(
            (far.x0 - 10.0).abs() < 1e-6 && (far.y0 - 10.0).abs() < 1e-6,
            "{far:?}"
        );
        assert_eq!(f.below.len(), 3);
    }

    /// #1281: a square's body is its two sides, a circle's its one
    /// curved side: parts to paint whole, not slats.
    #[test]
    fn a_body_is_one_object_per_side() {
        let s = square(Fill::Solid(Color::rgb8(200, 0, 0)));
        for kind in [
            ExtrudeKind::BackParallel,
            ExtrudeKind::SmallBack,
            ExtrudeKind::BigBack,
        ] {
            let f = body(kind).faces(&s);
            assert_eq!(f.below.len(), 2, "{kind:?}");
        }
        let mut c = s.clone();
        c.kind = ShapeKind::Ellipse {
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            arc: None,
        };
        let f = body(ExtrudeKind::BackParallel).faces(&c);
        assert_eq!(f.below.len(), 1, "one curved side");
        // Its colours follow the light round the side.
        let Fill::Fountain(fill) = &f.below[0].fill else {
            panic!("a fountain across the side: {:?}", f.below[0].fill);
        };
        assert!(fill.stops.len() >= 3, "{}", fill.stops.len());
        assert!(fill.stops.windows(2).all(|w| w[0].pos <= w[1].pos));
        // It covers the half of the side toward the vector.
        let b = f.below[0].bounds();
        assert!(
            b.x1 > 29.0 && b.y1 > 29.0 && b.x0 < 4.0 && b.y0 < 4.0,
            "{b:?}"
        );
    }

    #[test]
    fn front_and_vanishing_bodies() {
        let s = square(Fill::Solid(Color::rgb8(0, 0, 200)));
        // Front parallel: the near face looks like the object, on top;
        // the bottom and left faces show.
        let f = body(ExtrudeKind::FrontParallel).faces(&s);
        assert!(f.below.is_empty());
        assert_eq!(filled(&f.above).len(), 3);
        let near = f.above.last().expect("near face");
        assert_eq!(near.fill, s.fill);
        assert!((near.bounds().x0 - 10.0).abs() < 1e-6);
        // Small back: the far face a quarter of the way to (60, 60)
        // (shown through an object without a fill).
        let hollow = square(Fill::None);
        let f = body(ExtrudeKind::SmallBack).faces(&hollow);
        let far = f.below[0].bounds();
        assert!(
            (far.x0 - 15.0).abs() < 1e-6 && (far.width() - 15.0).abs() < 1e-6,
            "{far:?}"
        );
        // Big back: a quarter farther from it, bigger.
        let f = body(ExtrudeKind::BigBack).faces(&hollow);
        let far = f.below[0].bounds();
        assert!((far.width() - 25.0).abs() < 1e-6, "{far:?}");
        // Shade: fountains from the first colour to the second.
        let mut b = body(ExtrudeKind::BackParallel);
        b.coloring = Coloring::Shade(Color::WHITE, Color::BLACK);
        b.light_intensity = 0.0;
        let f = b.faces(&s);
        assert!(matches!(f.below[0].fill, Fill::Fountain(_)));
        let f = b.faces(&hollow);
        assert_eq!(f.below[0].fill, Fill::Solid(Color::BLACK));
    }

    #[test]
    fn every_subpath_extrudes_and_holes_show_their_far_walls() {
        // A square with a square hole (even-odd), both counterclockwise.
        let mut path = Rect::new(0.0, 0.0, 30.0, 30.0).to_path(0.01);
        path.extend(Rect::new(10.0, 10.0, 20.0, 20.0).to_path(0.01));
        let mut s = square(Fill::Solid(Color::BLACK));
        s.kind = ShapeKind::Path { path, closed: true };
        let f = body(ExtrudeKind::BackParallel).faces(&s);
        // Two faces of the outer square and two of the hole (its bottom
        // and left walls); the far face hidden.
        assert_eq!(filled(&f.below).len(), 4);
        let walls: Vec<_> = filled(&f.below)
            .into_iter()
            .filter(|s| s.bounds().x0 >= 9.9 && s.bounds().x1 <= 30.1 && s.bounds().y1 <= 30.1)
            .collect();
        assert_eq!(walls.len(), 2);
        // Without a fill: the far face with both subpaths comes first.
        s.fill = Fill::None;
        let f = body(ExtrudeKind::BackParallel).faces(&s);
        assert_eq!(filled(&f.below).len(), 5);
        let moves = f.below[0]
            .local_path()
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count();
        assert_eq!(moves, 2);
    }

    #[test]
    fn a_curved_side_is_one_outlined_surface() {
        let mut s = square(Fill::Solid(Color::rgb8(0, 120, 0)));
        s.kind = ShapeKind::Ellipse {
            rect: Rect::new(0.0, 0.0, 30.0, 20.0),
            arc: None,
        };
        let f = body(ExtrudeKind::BackParallel).faces(&s);
        assert_eq!(f.below.len(), 1);
        let side = &f.below[0];
        assert_ne!(side.fill, Fill::None);
        assert_eq!(
            side.stroke.as_ref().map(|x| x.color),
            s.stroke.as_ref().map(|x| x.color),
            "outlined like the object"
        );
        // An ellipse has no closing element but is closed: only the half
        // toward the vector shows.
        let b = side.bounds();
        assert!(b.center().x + b.center().y > 20.0, "{b:?}");
        // Not even where the outline starts and ends (an ellipse's first
        // point), turned any way.
        for t in [0.3, 1.7, 3.0] {
            let mut r = s.clone();
            r.transform = Affine::rotate(t);
            let f = body(ExtrudeKind::BackParallel).faces(&r);
            assert_eq!(f.below.len(), 1, "{t}");
        }
        // No outline on the object: a hairline of the side's own colour
        // closes its seams.
        s.stroke = None;
        let f = body(ExtrudeKind::BackParallel).faces(&s);
        let side = &f.below[0];
        let st = side.stroke.as_ref().expect("a hairline");
        assert_eq!(Some(st.color), side.fill.preview_color());
    }

    #[test]
    fn odd_values_give_no_faces() {
        let s = square(Fill::None);
        let mut b = body(ExtrudeKind::SmallBack);
        b.vanishing = Point::new(f64::NAN, 0.0);
        let f = b.faces(&s);
        assert!(f.below.is_empty() && f.above.is_empty());
        let mut b = body(ExtrudeKind::BackParallel);
        b.light_intensity = f64::NAN;
        b.depth = 50.0;
        assert!(!b.faces(&s).below.is_empty());
        let empty = Shape::new(
            ShapeId(2),
            ShapeKind::Path {
                path: BezPath::new(),
                closed: false,
            },
        );
        assert!(body(ExtrudeKind::BackParallel)
            .faces(&empty)
            .below
            .is_empty());
    }
}
