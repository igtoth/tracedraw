//! The points objects offer for snapping, by snapping mode: nodes, intersections, segment midpoints, the quadrants
//! of ellipses and arcs, tangent and perpendicular points from the point a
//! line starts at, edges, centres and text baselines. Everything here is
//! page space.

use serde::{Deserialize, Serialize};
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{
        Affine, BezPath, Line, ParamCurve, ParamCurveArclen, ParamCurveNearest, PathSeg, Point,
        Rect, Shape as _, Vec2,
    },
};

/// What a snap point is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SnapMode {
    Node,
    Intersection,
    Midpoint,
    Quadrant,
    Tangent,
    Perpendicular,
    Edge,
    Center,
    TextBaseline,
}

impl SnapMode {
    /// In the order Options > Snapping lists them.
    pub const ALL: [SnapMode; 9] = [
        SnapMode::Node,
        SnapMode::Intersection,
        SnapMode::Midpoint,
        SnapMode::Quadrant,
        SnapMode::Tangent,
        SnapMode::Perpendicular,
        SnapMode::Edge,
        SnapMode::Center,
        SnapMode::TextBaseline,
    ];

    pub fn key(self) -> &'static str {
        match self {
            SnapMode::Node => "snap.node",
            SnapMode::Intersection => "snap.intersection",
            SnapMode::Midpoint => "snap.midpoint",
            SnapMode::Quadrant => "snap.quadrant",
            SnapMode::Tangent => "snap.tangent",
            SnapMode::Perpendicular => "snap.perpendicular",
            SnapMode::Edge => "snap.edge",
            SnapMode::Center => "snap.center",
            SnapMode::TextBaseline => "snap.text_baseline",
        }
    }

    /// A single point; edges and baselines are lines the pointer slides
    /// along and give way to points.
    pub fn is_point(self) -> bool {
        !matches!(self, SnapMode::Edge | SnapMode::TextBaseline)
    }
}

/// Which modes are on (Options > Snapping); all of them by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SnapModes {
    pub node: bool,
    pub intersection: bool,
    pub midpoint: bool,
    pub quadrant: bool,
    pub tangent: bool,
    pub perpendicular: bool,
    pub edge: bool,
    pub center: bool,
    pub text_baseline: bool,
}

impl Default for SnapModes {
    fn default() -> Self {
        SnapModes {
            node: true,
            intersection: true,
            midpoint: true,
            quadrant: true,
            tangent: true,
            perpendicular: true,
            edge: true,
            center: true,
            text_baseline: true,
        }
    }
}

impl SnapModes {
    pub fn get(&self, m: SnapMode) -> bool {
        match m {
            SnapMode::Node => self.node,
            SnapMode::Intersection => self.intersection,
            SnapMode::Midpoint => self.midpoint,
            SnapMode::Quadrant => self.quadrant,
            SnapMode::Tangent => self.tangent,
            SnapMode::Perpendicular => self.perpendicular,
            SnapMode::Edge => self.edge,
            SnapMode::Center => self.center,
            SnapMode::TextBaseline => self.text_baseline,
        }
    }

    pub fn set(&mut self, m: SnapMode, on: bool) {
        let v = match m {
            SnapMode::Node => &mut self.node,
            SnapMode::Intersection => &mut self.intersection,
            SnapMode::Midpoint => &mut self.midpoint,
            SnapMode::Quadrant => &mut self.quadrant,
            SnapMode::Tangent => &mut self.tangent,
            SnapMode::Perpendicular => &mut self.perpendicular,
            SnapMode::Edge => &mut self.edge,
            SnapMode::Center => &mut self.center,
            SnapMode::TextBaseline => &mut self.text_baseline,
        };
        *v = on;
    }

    pub fn none() -> Self {
        let mut m = SnapModes::default();
        for k in SnapMode::ALL {
            m.set(k, false);
        }
        m
    }
}

/// A point to snap to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub point: Point,
    pub mode: SnapMode,
}

/// The snapping geometry of one object, on the page.
#[derive(Debug, Clone)]
pub enum Geo {
    /// An outline: edges, intersections and perpendiculars; with
    /// `points` also its nodes, segment midpoints and tangents (an
    /// ellipse's outline is only an approximation of it: it brings those
    /// itself).
    Outline {
        path: BezPath,
        points: bool,
    },
    /// Nodes that are not the outline's own (an ellipse's).
    Nodes(Vec<Point>),
    /// An ellipse or arc: the centre and the page images of its x and y
    /// radii; the arc's angles (degrees) when it is not whole.
    Ellipse {
        center: Point,
        rx: Vec2,
        ry: Vec2,
        arc: Option<(f64, f64)>,
    },
    Center(Point),
    /// One line of text: the start and end of its baseline.
    Baseline(Point, Point),
}

/// Baselines of a text object on the page; supplied by the app, which
/// lays text out.
pub type BaselinesOf<'a> = &'a dyn Fn(&Shape, Affine) -> Vec<(Point, Point)>;

/// Collect the snapping geometry of `s` under `parent`.
pub fn geometry(s: &Shape, parent: Affine, baselines: BaselinesOf, out: &mut Vec<Geo>) {
    if !s.visible {
        return;
    }
    let t = parent * s.transform;
    match &s.kind {
        ShapeKind::Group { children } => {
            for c in children {
                geometry(c, t, baselines, out);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            geometry(frame, t, baselines, out);
            for c in contents {
                geometry(c, t, baselines, out);
            }
        }
        ShapeKind::Text { .. } => {
            for (a, b) in baselines(s, parent) {
                out.push(Geo::Baseline(a, b));
            }
        }
        ShapeKind::SymbolInstance { .. } => {}
        ShapeKind::Ellipse { rect, arc } => {
            let c = rect.center();
            let center = t * c;
            let o = t * Point::ZERO;
            let rx = (t * Point::new(rect.width() / 2.0, 0.0)) - o;
            let ry = (t * Point::new(0.0, rect.height() / 2.0)) - o;
            out.push(Geo::Outline {
                path: parent * s.page_path(),
                points: false,
            });
            let at = |deg: f64| {
                let a = deg.to_radians();
                center + rx * a.cos() + ry * a.sin()
            };
            out.push(Geo::Nodes(match arc {
                None => vec![at(90.0)],
                Some(a) => vec![at(a.start_deg), at(a.end_deg)],
            }));
            out.push(Geo::Ellipse {
                center,
                rx,
                ry,
                arc: arc.map(|a| (a.start_deg, a.end_deg)),
            });
            out.push(Geo::Center(center));
        }
        _ => {
            let path = parent * s.page_path();
            let center = match &s.kind {
                ShapeKind::Rect { rect, .. }
                | ShapeKind::Polygon { rect, .. }
                | ShapeKind::Bitmap { rect, .. } => Some(t * rect.center()),
                ShapeKind::Table(table) => Some(t * table.rect.center()),
                ShapeKind::Path { closed: true, .. } => centroid(&path),
                _ => None,
            };
            out.push(Geo::Outline { path, points: true });
            if let Some(c) = center {
                out.push(Geo::Center(c));
            }
        }
    }
}

/// The page as an object: corners, edge midpoints, centre and edges.
pub fn page_geometry(page: Rect, out: &mut Vec<Geo>) {
    out.push(Geo::Outline {
        path: page.to_path(0.01),
        points: true,
    });
    out.push(Geo::Center(page.center()));
}

/// Area centroid of a closed outline (flattened), if it has an area.
fn centroid(path: &BezPath) -> Option<Point> {
    let mut pts: Vec<Point> = Vec::new();
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    let mut close = |pts: &mut Vec<Point>| {
        if pts.len() >= 3 {
            for i in 0..pts.len() {
                let p = pts[i];
                let q = pts[(i + 1) % pts.len()];
                let cross = p.x * q.y - q.x * p.y;
                a += cross;
                cx += (p.x + q.x) * cross;
                cy += (p.y + q.y) * cross;
            }
        }
        pts.clear();
    };
    tracedraw_core::geometry::flatten(path, 0.05, &mut |el| match el {
        tracedraw_core::geometry::PathEl::MoveTo(p) => {
            close(&mut pts);
            pts.push(p);
        }
        tracedraw_core::geometry::PathEl::LineTo(p) => pts.push(p),
        _ => {}
    });
    close(&mut pts);
    if a.abs() < 1e-9 {
        return None;
    }
    Some(Point::new(cx / (3.0 * a), cy / (3.0 * a)))
}

/// Segments of an outline.
fn segments(path: &BezPath) -> Vec<PathSeg> {
    path.segments().collect()
}

/// End points of the outline's segments, without repeats.
fn nodes_of(path: &BezPath) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    for el in path.elements() {
        use tracedraw_core::geometry::PathEl;
        let p = match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => *p,
            PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => *p,
            PathEl::ClosePath => continue,
        };
        if !out.iter().any(|q| (*q - p).hypot() < 1e-9) {
            out.push(p);
        }
    }
    out
}

/// The point halfway along a segment.
fn midpoint(seg: &PathSeg) -> Point {
    match seg {
        PathSeg::Line(l) => l.eval(0.5),
        _ => {
            let len = seg.arclen(1e-4);
            seg.eval(seg.inv_arclen(len / 2.0, 1e-4))
        }
    }
}

fn deriv(seg: &PathSeg, t: f64) -> Vec2 {
    match seg {
        PathSeg::Line(l) => l.p1 - l.p0,
        PathSeg::Quad(q) => {
            let a = q.p1 - q.p0;
            let b = q.p2 - q.p1;
            (a * (1.0 - t) + b * t) * 2.0
        }
        PathSeg::Cubic(c) => {
            let a = c.p1 - c.p0;
            let b = c.p2 - c.p1;
            let d = c.p3 - c.p2;
            let mt = 1.0 - t;
            (a * (mt * mt) + b * (2.0 * mt * t) + d * (t * t)) * 3.0
        }
    }
}

/// Parameters in [0, 1] where `f` changes sign, refined by bisection.
fn roots(f: impl Fn(f64) -> f64) -> Vec<f64> {
    const N: usize = 48;
    let mut out = Vec::new();
    let mut prev = f(0.0);
    for i in 1..=N {
        let t1 = i as f64 / N as f64;
        let v1 = f(t1);
        if prev == 0.0 {
            out.push((i - 1) as f64 / N as f64);
        } else if prev * v1 < 0.0 {
            let (mut lo, mut hi, mut flo) = ((i - 1) as f64 / N as f64, t1, prev);
            for _ in 0..40 {
                let mid = (lo + hi) / 2.0;
                let fm = f(mid);
                if fm * flo <= 0.0 {
                    hi = mid;
                } else {
                    lo = mid;
                    flo = fm;
                }
            }
            out.push((lo + hi) / 2.0);
        }
        prev = v1;
    }
    out
}

fn add(out: &mut Vec<(Target, f64)>, p: Point, point: Point, mode: SnapMode, tol: f64) {
    let d = (point - p).hypot();
    if d <= tol && point.x.is_finite() && point.y.is_finite() {
        out.push((Target { point, mode }, d));
    }
}

/// Every snap target of `geo` within `tol` of `p`, with its distance, for
/// the modes on. `from` is where the line being drawn starts (for tangent
/// and perpendicular points).
pub fn targets(
    geo: &[Geo],
    p: Point,
    tol: f64,
    from: Option<Point>,
    modes: &SnapModes,
) -> Vec<(Target, f64)> {
    let mut out = Vec::new();
    // Segments near the pointer, for edges, intersections, tangents and
    // perpendiculars; with their outline's index.
    let mut near: Vec<(usize, usize, PathSeg)> = Vec::new();
    for (gi, g) in geo.iter().enumerate() {
        match g {
            Geo::Outline { path, points } => {
                let points = *points;
                if !path.bounding_box().inflate(tol, tol).contains(p) {
                    continue;
                }
                if modes.node && points {
                    for n in nodes_of(path) {
                        add(&mut out, p, n, SnapMode::Node, tol);
                    }
                }
                for (si, seg) in segments(path).into_iter().enumerate() {
                    let nearest = seg.nearest(p, 1e-6);
                    if nearest.distance_sq.sqrt() > tol {
                        continue;
                    }
                    if modes.midpoint && points {
                        add(&mut out, p, midpoint(&seg), SnapMode::Midpoint, tol);
                    }
                    if modes.edge {
                        add(&mut out, p, seg.eval(nearest.t), SnapMode::Edge, tol);
                    }
                    if let Some(f) = from {
                        if modes.perpendicular {
                            for t in roots(|t| (seg.eval(t) - f).dot(deriv(&seg, t))) {
                                add(&mut out, p, seg.eval(t), SnapMode::Perpendicular, tol);
                            }
                        }
                        if modes.tangent && points && !matches!(seg, PathSeg::Line(_)) {
                            for t in roots(|t| (seg.eval(t) - f).cross(deriv(&seg, t))) {
                                add(&mut out, p, seg.eval(t), SnapMode::Tangent, tol);
                            }
                        }
                    }
                    near.push((gi, si, seg));
                }
            }
            Geo::Nodes(ns) => {
                if modes.node {
                    for n in ns {
                        add(&mut out, p, *n, SnapMode::Node, tol);
                    }
                }
            }
            Geo::Ellipse {
                center,
                rx,
                ry,
                arc,
            } => {
                let inside = |deg: f64| match arc {
                    None => true,
                    Some((a, b)) => {
                        let sweep = (b - a).rem_euclid(360.0);
                        (deg - a).rem_euclid(360.0) <= sweep + 1e-9
                    }
                };
                let at = |deg: f64| {
                    let a = deg.to_radians();
                    *center + *rx * a.cos() + *ry * a.sin()
                };
                if modes.quadrant {
                    for deg in [0.0, 90.0, 180.0, 270.0] {
                        if inside(deg) {
                            add(&mut out, p, at(deg), SnapMode::Quadrant, tol);
                        }
                    }
                }
                if let (Some(f), true) = (from, modes.tangent) {
                    // In the ellipse's own frame it is the unit circle;
                    // tangency survives the affine map.
                    let m = Affine::new([rx.x, rx.y, ry.x, ry.y, center.x, center.y]);
                    if m.determinant().abs() > 1e-12 {
                        let u = m.inverse() * f;
                        let d = u.to_vec2().hypot();
                        if d > 1.0 {
                            let base = u.y.atan2(u.x);
                            let off = (1.0 / d).acos();
                            for a in [base + off, base - off] {
                                let deg = a.to_degrees();
                                if inside(deg) {
                                    add(&mut out, p, at(deg), SnapMode::Tangent, tol);
                                }
                            }
                        }
                    }
                }
            }
            Geo::Center(c) => {
                if modes.center {
                    add(&mut out, p, *c, SnapMode::Center, tol);
                }
            }
            Geo::Baseline(a, b) => {
                if modes.text_baseline {
                    let l = Line::new(*a, *b);
                    let n = l.nearest(p, 1e-9);
                    add(&mut out, p, l.eval(n.t), SnapMode::TextBaseline, tol);
                }
            }
        }
    }
    if modes.intersection {
        for i in 0..near.len() {
            for j in i + 1..near.len() {
                let (_, _, a) = &near[i];
                let (_, _, b) = &near[j];
                // Segments joined end to end meet at a node, not a crossing.
                let at_end = |s: &PathSeg, q: Point| {
                    (s.eval(0.0) - q).hypot() < 1e-6 || (s.eval(1.0) - q).hypot() < 1e-6
                };
                for q in intersections(a, b) {
                    if at_end(a, q) && at_end(b, q) {
                        continue;
                    }
                    add(&mut out, p, q, SnapMode::Intersection, tol);
                }
            }
        }
    }
    out
}

/// Where two segments cross.
fn intersections(a: &PathSeg, b: &PathSeg) -> Vec<Point> {
    let lines_of = |s: &PathSeg| -> Vec<Line> {
        match s {
            PathSeg::Line(l) => vec![*l],
            _ => {
                let mut pts = Vec::new();
                let mut path = BezPath::new();
                path.move_to(s.eval(0.0));
                path.push(match s {
                    PathSeg::Quad(q) => tracedraw_core::geometry::PathEl::QuadTo(q.p1, q.p2),
                    PathSeg::Cubic(c) => {
                        tracedraw_core::geometry::PathEl::CurveTo(c.p1, c.p2, c.p3)
                    }
                    PathSeg::Line(l) => tracedraw_core::geometry::PathEl::LineTo(l.p1),
                });
                tracedraw_core::geometry::flatten(&path, 1e-4, &mut |el| match el {
                    tracedraw_core::geometry::PathEl::MoveTo(p)
                    | tracedraw_core::geometry::PathEl::LineTo(p) => pts.push(p),
                    _ => {}
                });
                pts.windows(2).map(|w| Line::new(w[0], w[1])).collect()
            }
        }
    };
    let mut out = Vec::new();
    for l in lines_of(b) {
        for hit in a.intersect_line(l) {
            if (0.0..=1.0).contains(&hit.line_t) {
                out.push(l.eval(hit.line_t));
            }
        }
    }
    out
}

/// The best target: the nearest point (node, intersection, midpoint,
/// quadrant, tangent, perpendicular, centre); without one, the nearest
/// edge or baseline. Equal distances go to the mode listed first.
pub fn best(found: &[(Target, f64)], points: bool, lines: bool) -> Option<Target> {
    let rank = |m: SnapMode| SnapMode::ALL.iter().position(|x| *x == m).unwrap_or(99);
    let pick = |want_point: bool| {
        found
            .iter()
            .filter(|(t, _)| t.mode.is_point() == want_point)
            .min_by(|a, b| {
                a.1.total_cmp(&b.1)
                    .then_with(|| rank(a.0.mode).cmp(&rank(b.0.mode)))
            })
            .map(|(t, _)| *t)
    };
    let p = if points { pick(true) } else { None };
    p.or_else(|| if lines { pick(false) } else { None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{EllipseArc, ShapeId};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Shape {
        Shape::new(
            ShapeId(1),
            ShapeKind::Rect {
                rect: Rect::new(x0, y0, x1, y1),
                radius: 0.0,
                corners: None,
            },
        )
    }

    fn geo_of(shapes: &[Shape]) -> Vec<Geo> {
        let mut g = Vec::new();
        let none = |_: &Shape, _: Affine| Vec::new();
        for s in shapes {
            geometry(s, Affine::IDENTITY, &none, &mut g);
        }
        g
    }

    fn snap(shapes: &[Shape], p: Point, from: Option<Point>) -> Option<Target> {
        let g = geo_of(shapes);
        best(
            &targets(&g, p, 1.0, from, &SnapModes::default()),
            true,
            true,
        )
    }

    #[test]
    fn nodes_midpoints_centres_and_edges() {
        let r = rect(0.0, 0.0, 40.0, 20.0);
        let t = snap(std::slice::from_ref(&r), Point::new(39.6, 19.5), None).unwrap();
        assert_eq!((t.mode, t.point), (SnapMode::Node, Point::new(40.0, 20.0)));
        let t = snap(std::slice::from_ref(&r), Point::new(20.4, 0.3), None).unwrap();
        assert_eq!(
            (t.mode, t.point),
            (SnapMode::Midpoint, Point::new(20.0, 0.0))
        );
        let t = snap(std::slice::from_ref(&r), Point::new(20.5, 10.2), None).unwrap();
        assert_eq!(
            (t.mode, t.point),
            (SnapMode::Center, Point::new(20.0, 10.0))
        );
        // On the edge away from points: the nearest point of the edge.
        let t = snap(std::slice::from_ref(&r), Point::new(30.0, 0.4), None).unwrap();
        assert_eq!(t.mode, SnapMode::Edge);
        assert!((t.point - Point::new(30.0, 0.0)).hypot() < 1e-6);
        // Far from everything: nothing.
        assert!(snap(&[r], Point::new(10.0, 6.0), None).is_none());
    }

    #[test]
    fn modes_turn_off() {
        let r = rect(0.0, 0.0, 40.0, 20.0);
        let g = geo_of(&[r]);
        let mut m = SnapModes::default();
        m.node = false;
        let t = best(
            &targets(&g, Point::new(39.6, 19.5), 1.0, None, &m),
            true,
            true,
        )
        .unwrap();
        assert_eq!(t.mode, SnapMode::Edge);
        assert!(best(
            &targets(&g, Point::new(39.6, 19.5), 1.0, None, &SnapModes::none()),
            true,
            true
        )
        .is_none());
        let mut back = SnapModes::none();
        back.set(SnapMode::Quadrant, true);
        assert!(back.get(SnapMode::Quadrant) && !back.get(SnapMode::Node));
    }

    #[test]
    fn crossing_outlines_snap_to_their_intersection() {
        let a = rect(0.0, 0.0, 40.0, 20.0);
        let b = rect(10.0, 10.0, 30.0, 30.0);
        let t = snap(&[a, b], Point::new(10.3, 19.6), None).unwrap();
        assert_eq!(t.mode, SnapMode::Intersection);
        assert!((t.point - Point::new(10.0, 20.0)).hypot() < 1e-6, "{t:?}");
    }

    #[test]
    fn ellipses_offer_quadrants_tangents_and_their_node() {
        let mut e = Shape::new(
            ShapeId(2),
            ShapeKind::Ellipse {
                rect: Rect::new(-10.0, -5.0, 10.0, 5.0),
                arc: None,
            },
        );
        e.transform = Affine::translate((50.0, 50.0));
        let t = snap(std::slice::from_ref(&e), Point::new(60.5, 50.2), None).unwrap();
        assert_eq!(t.mode, SnapMode::Quadrant);
        assert!((t.point - Point::new(60.0, 50.0)).hypot() < 1e-9);
        // The top is the node and a quadrant: the node comes first.
        let t = snap(std::slice::from_ref(&e), Point::new(50.2, 55.3), None).unwrap();
        assert_eq!(t.mode, SnapMode::Node);
        // A line from (50, 70): tangent points where y = 50 + 5 * 5/20.
        let from = Point::new(50.0, 70.0);
        let y = 50.0 + 5.0 * 5.0 / 20.0;
        let x = 50.0 + 10.0 * (1.0f64 - (5.0 / 20.0) * (5.0 / 20.0)).sqrt();
        let t = snap(
            std::slice::from_ref(&e),
            Point::new(x + 0.4, y + 0.2),
            Some(from),
        )
        .unwrap();
        assert_eq!(t.mode, SnapMode::Tangent, "{t:?}");
        assert!((t.point - Point::new(x, y)).hypot() < 1e-6, "{t:?}");
        // An arc keeps the quadrants it covers only.
        e.kind = ShapeKind::Ellipse {
            rect: Rect::new(-10.0, -5.0, 10.0, 5.0),
            arc: Some(EllipseArc {
                start_deg: 0.0,
                end_deg: 90.0,
                pie: false,
            }),
        };
        let g = geo_of(&[e]);
        let q: Vec<Point> = targets(&g, Point::new(40.0, 50.0), 1.0, None, &SnapModes::default())
            .into_iter()
            .filter(|(t, _)| t.mode == SnapMode::Quadrant)
            .map(|(t, _)| t.point)
            .collect();
        assert!(q.is_empty(), "{q:?}");
    }

    #[test]
    fn perpendicular_from_the_start_of_a_line() {
        let r = rect(0.0, 0.0, 40.0, 20.0);
        let from = Point::new(25.0, 40.0);
        let t = snap(&[r], Point::new(25.6, 20.3), Some(from)).unwrap();
        assert_eq!(t.mode, SnapMode::Perpendicular);
        assert!((t.point - Point::new(25.0, 20.0)).hypot() < 1e-6);
    }

    #[test]
    fn closed_curves_snap_to_their_centroid() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((30.0, 0.0));
        path.line_to((0.0, 30.0));
        path.close_path();
        let s = Shape::new(ShapeId(3), ShapeKind::Path { path, closed: true });
        let t = snap(&[s], Point::new(10.4, 10.3), None).unwrap();
        assert_eq!(t.mode, SnapMode::Center);
        assert!((t.point - Point::new(10.0, 10.0)).hypot() < 1e-6, "{t:?}");
    }

    #[test]
    fn text_baselines_come_from_the_app() {
        let s = Shape::new(
            ShapeId(4),
            ShapeKind::Text {
                spans: vec![tracedraw_core::TextSpan::new("abc", "Sans", 12.0)],
                origin: Point::new(0.0, 0.0),
                frame: None,
                align: tracedraw_core::TextAlign::Left,
                para: Default::default(),
                on_path: None,
            },
        );
        let lines = |_: &Shape, _: Affine| vec![(Point::new(0.0, 0.0), Point::new(12.0, 0.0))];
        let mut g = Vec::new();
        geometry(&s, Affine::IDENTITY, &lines, &mut g);
        let t = best(
            &targets(&g, Point::new(6.0, 0.5), 1.0, None, &SnapModes::default()),
            true,
            true,
        )
        .unwrap();
        assert_eq!(
            (t.mode, t.point),
            (SnapMode::TextBaseline, Point::new(6.0, 0.0))
        );
    }

    #[test]
    fn the_page_is_an_object_too() {
        let mut g = Vec::new();
        page_geometry(Rect::new(0.0, 0.0, 210.0, 297.0), &mut g);
        let t = best(
            &targets(
                &g,
                Point::new(105.3, 296.6),
                1.0,
                None,
                &SnapModes::default(),
            ),
            true,
            true,
        )
        .unwrap();
        assert_eq!(
            (t.mode, t.point),
            (SnapMode::Midpoint, Point::new(105.0, 297.0))
        );
        let t = best(
            &targets(
                &g,
                Point::new(105.2, 148.0),
                1.0,
                None,
                &SnapModes::default(),
            ),
            true,
            true,
        )
        .unwrap();
        assert_eq!(t.mode, SnapMode::Center);
    }
}
