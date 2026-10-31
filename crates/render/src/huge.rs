//! Paths far larger than any target. The scan converter works in fixed
//! point with integer bounds, and a path billions of pixels across (an
//! object scaled by a degenerate step, a deep zoom into a large shape)
//! overflowed them and panicked inside it. Such paths are cut to a square
//! much larger than any pixmap before they reach it: what lies inside the
//! square, and so everything visible, is unchanged.

use tracedraw_core::geometry::{BezPath, CubicBez, ParamCurve as _, PathEl, Point, Rect};

/// Half the side of the square, in pixels: far past any pixmap, and small
/// enough that the rasteriser's fixed-point arithmetic cannot overflow.
pub const LIMIT: f64 = (1u32 << 20) as f64;

/// Halvings of a curve that crosses the square's side before the part
/// still crossing it is drawn as a straight line.
const MAX_DEPTH: u32 = 40;

/// A vertex of a clipped subpath with the curve that reaches it: `None`
/// for a straight line, the two control points of a cubic otherwise.
type Vertex = (Point, Option<(Point, Point)>);

/// One side of the square: whether a point is on its inner side, and
/// where the segment between two points crosses it.
type Side = (fn(Point) -> bool, fn(Point, Point) -> Point);

/// What `path` needs before it is rasterised: `None` when a coordinate
/// is not finite (nothing can be drawn), the path itself borrowed when it
/// fits in the square, otherwise its part inside the square.
pub fn fit(path: &BezPath) -> Option<std::borrow::Cow<'_, BezPath>> {
    let mut finite = true;
    let mut inside = true;
    for el in path.elements() {
        let pts: &[Point] = match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => std::slice::from_ref(p),
            PathEl::QuadTo(a, b) => &[*a, *b],
            PathEl::CurveTo(a, b, c) => &[*a, *b, *c],
            PathEl::ClosePath => &[],
        };
        for p in pts {
            finite &= p.x.is_finite() && p.y.is_finite();
            inside &= p.x.abs() <= LIMIT && p.y.abs() <= LIMIT;
        }
    }
    if !finite {
        return None;
    }
    if inside {
        return Some(std::borrow::Cow::Borrowed(path));
    }
    Some(std::borrow::Cow::Owned(clip(path)))
}

/// The part of `path` inside the square. Closed subpaths are clipped as
/// polygons, so fills keep their winding inside it; open ones as
/// polylines, so outlines draw no closing edge they did not have.
fn clip(path: &BezPath) -> BezPath {
    let mut out = BezPath::new();
    let mut verts: Vec<Vertex> = Vec::new();
    let flush = |verts: &mut Vec<Vertex>, closed: bool, out: &mut BezPath| {
        if verts.len() > 1 {
            emit(clip_subpath(std::mem::take(verts), closed), closed, out);
        }
        verts.clear();
    };
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                flush(&mut verts, false, &mut out);
                verts.push((p, None));
            }
            PathEl::LineTo(p) => verts.push((p, None)),
            PathEl::QuadTo(q, p) => {
                if let Some(&(p0, _)) = verts.last() {
                    // Raised to a cubic: the same curve exactly.
                    let c1 = p0 + (q - p0) * (2.0 / 3.0);
                    let c2 = p + (q - p) * (2.0 / 3.0);
                    reduce(CubicBez::new(p0, c1, c2, p), MAX_DEPTH, &mut verts);
                }
            }
            PathEl::CurveTo(c1, c2, p) => {
                if let Some(&(p0, _)) = verts.last() {
                    reduce(CubicBez::new(p0, c1, c2, p), MAX_DEPTH, &mut verts);
                }
            }
            PathEl::ClosePath => {
                let start = verts.first().map(|v| v.0);
                flush(&mut verts, true, &mut out);
                // A path going on after a close starts from the same point.
                if let Some(p) = start {
                    verts.push((p, None));
                }
            }
        }
    }
    flush(&mut verts, false, &mut out);
    out
}

/// Append cubic `c` to `verts` so that every curve kept lies wholly
/// inside the square: a curve whose control points are all outside it on
/// the far side of nothing visible becomes its chord, one that crosses
/// the square's side is halved until it does not.
fn reduce(c: CubicBez, depth: u32, verts: &mut Vec<Vertex>) {
    let hull = Rect::from_points(c.p0, c.p1).union_pt(c.p2).union_pt(c.p3);
    let within = hull.x0 >= -LIMIT && hull.x1 <= LIMIT && hull.y0 >= -LIMIT && hull.y1 <= LIMIT;
    let apart = hull.x1 < -LIMIT || hull.x0 > LIMIT || hull.y1 < -LIMIT || hull.y0 > LIMIT;
    if within {
        verts.push((c.p3, Some((c.p1, c.p2))));
    } else if apart || depth == 0 {
        // The region between the curve and its chord lies in the hull,
        // outside the square: the winding inside it does not change.
        verts.push((c.p3, None));
    } else {
        let (a, b) = c.subdivide();
        reduce(a, depth - 1, verts);
        reduce(b, depth - 1, verts);
    }
}

/// Clip one subpath to the square, one side at a time. Curves are inside
/// the square already (see [`reduce`]) and pass through; straight edges
/// are cut where they cross a side, and the cuts are joined along it.
fn clip_subpath(mut verts: Vec<Vertex>, closed: bool) -> Vec<Vertex> {
    // Inside test and crossing point for each side of the square.
    let sides: [Side; 4] = [
        (|p| p.x >= -LIMIT, |a, b| cross_x(a, b, -LIMIT)),
        (|p| p.x <= LIMIT, |a, b| cross_x(a, b, LIMIT)),
        (|p| p.y >= -LIMIT, |a, b| cross_y(a, b, -LIMIT)),
        (|p| p.y <= LIMIT, |a, b| cross_y(a, b, LIMIT)),
    ];
    for (inside, cross) in sides {
        let n = verts.len();
        if n == 0 {
            break;
        }
        let mut next = Vec::with_capacity(n + 4);
        for i in 0..n {
            let cur = verts[i];
            let prev = if i > 0 {
                Some(verts[i - 1].0)
            } else if closed {
                // The closing edge, a straight line, reaches the first vertex.
                verts.last().map(|v| v.0)
            } else {
                None
            };
            let Some(prev) = prev else {
                if inside(cur.0) {
                    next.push((cur.0, None));
                }
                continue;
            };
            // Curves lie inside the square; a closed subpath's first
            // vertex is reached by its closing line.
            if i == 0 || cur.1.is_none() {
                match (inside(prev), inside(cur.0)) {
                    (true, true) => next.push((cur.0, None)),
                    (false, true) => {
                        next.push((cross(prev, cur.0), None));
                        next.push((cur.0, None));
                    }
                    (true, false) => next.push((cross(prev, cur.0), None)),
                    (false, false) => {}
                }
            } else {
                next.push(cur);
            }
        }
        verts = next;
    }
    verts
}

fn cross_x(a: Point, b: Point, x: f64) -> Point {
    let t = (x - a.x) / (b.x - a.x);
    Point::new(x, (a.y + (b.y - a.y) * t).clamp(-LIMIT, LIMIT))
}

fn cross_y(a: Point, b: Point, y: f64) -> Point {
    let t = (y - a.y) / (b.y - a.y);
    Point::new((a.x + (b.x - a.x) * t).clamp(-LIMIT, LIMIT), y)
}

/// Write a clipped subpath to `out`. Its first vertex is always reached
/// by a straight edge (the closing line), see [`clip_subpath`].
fn emit(verts: Vec<Vertex>, closed: bool, out: &mut BezPath) {
    if verts.len() < 2 {
        return;
    }
    let mut it = verts.into_iter();
    if let Some((first, _)) = it.next() {
        out.move_to(first);
    }
    for (p, curve) in it {
        match curve {
            Some((c1, c2)) => out.curve_to(c1, c2, p),
            None => out.line_to(p),
        }
    }
    if closed {
        out.close_path();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Shape as _;

    fn square() -> Rect {
        Rect::new(-LIMIT, -LIMIT, LIMIT, LIMIT)
    }

    fn rect(r: Rect) -> BezPath {
        r.to_path(0.1)
    }

    #[test]
    fn small_paths_are_left_alone() {
        let p = rect(Rect::new(0.0, 0.0, 100.0, 50.0));
        assert!(matches!(fit(&p), Some(std::borrow::Cow::Borrowed(_))));
    }

    #[test]
    fn non_finite_paths_are_dropped() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((f64::NAN, 3.0));
        p.line_to((5.0, 5.0));
        p.close_path();
        assert!(fit(&p).is_none());
        let mut q = BezPath::new();
        q.move_to((0.0, 0.0));
        q.line_to((f64::INFINITY, 3.0));
        assert!(fit(&q).is_none());
    }

    #[test]
    fn a_huge_rectangle_becomes_the_square() {
        let p = rect(Rect::new(-3.4e9, -2.0e9, 3.4e9, 2.0e9));
        let c = fit(&p).expect("finite");
        let b = c.bounding_box();
        assert_eq!(b, square());
        // The inside is still inside.
        assert!(c.contains(Point::new(500.0, 100.0)));
    }

    #[test]
    fn a_huge_triangle_keeps_its_edges_inside_the_square() {
        // Edge from (-1e10, 0) to (1e10, 1e10): it crosses the visible area
        // near the origin and must stay where it was there.
        let mut p = BezPath::new();
        p.move_to((-1e10, 0.0));
        p.line_to((1e10, 1e10));
        p.line_to((1e10, -1e10));
        p.close_path();
        let c = fit(&p).expect("finite");
        for &(x, y) in &[(0.0, 0.0), (100.0, 40.0), (100.0, 60.0), (-200.0, -50.0)] {
            let pt = Point::new(x, y);
            assert_eq!(c.contains(pt), p.contains(pt), "{pt:?}");
        }
        for el in c.elements() {
            if let PathEl::MoveTo(q) | PathEl::LineTo(q) = el {
                assert!(q.x.abs() <= LIMIT && q.y.abs() <= LIMIT);
            }
        }
    }

    #[test]
    fn a_huge_circle_keeps_its_curve_near_the_view() {
        // A circle of radius 1e9 through the origin: its arc near the
        // origin is kept as curves, its far side is gone.
        let r = 1e9;
        let p = tracedraw_core::geometry::Circle::new((r, 0.0), r).to_path(0.1);
        let c = fit(&p).expect("finite");
        for &(x, y) in &[(10.0, 0.0), (-10.0, 0.0), (1000.0, 30000.0), (1.0, 40000.0)] {
            let pt = Point::new(x, y);
            assert_eq!(c.contains(pt), p.contains(pt), "{pt:?}");
        }
        let b = c.bounding_box();
        assert!(b.x0 >= -LIMIT && b.x1 <= LIMIT && b.y0 >= -LIMIT && b.y1 <= LIMIT);
    }

    #[test]
    fn an_open_line_is_cut_without_a_closing_edge() {
        let mut p = BezPath::new();
        p.move_to((-1e9, 10.0));
        p.line_to((1e9, 10.0));
        let c = fit(&p).expect("finite");
        let els = c.elements();
        assert_eq!(els.len(), 2);
        assert_eq!(els[0], PathEl::MoveTo(Point::new(-LIMIT, 10.0)));
        assert_eq!(els[1], PathEl::LineTo(Point::new(LIMIT, 10.0)));
    }

    #[test]
    fn a_path_wholly_outside_draws_nothing_visible() {
        let p = rect(Rect::new(5e9, 5e9, 6e9, 6e9));
        let c = fit(&p).expect("finite");
        assert!(!c.contains(Point::new(0.0, 0.0)));
        assert!(c.bounding_box().area().abs() < 1.0);
    }
}
