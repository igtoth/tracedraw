//! Geometry primitives. We re-export kurbo so the rest of the workspace
//! shares one set of types, and add the few helpers the editor needs.

pub use kurbo::{
    Affine, Arc, BezPath, Circle, CubicBez, Ellipse, Line, ParamCurve, ParamCurveArclen,
    ParamCurveDeriv, ParamCurveNearest, PathEl, PathSeg, Point, Rect, RoundedRect, Shape, Size,
    Vec2,
};

/// Flatten a path into lines within `tolerance`, calling `f` with each
/// move, line and close element.
pub fn flatten(path: &BezPath, tolerance: f64, f: &mut dyn FnMut(PathEl)) {
    kurbo::flatten(path.elements().iter().copied(), tolerance, f);
}

/// Build a closed rectangle path, optionally with rounded corners.
pub fn rect_path(rect: Rect, radius: f64) -> BezPath {
    if radius > 0.0 {
        RoundedRect::from_rect(rect, radius).to_path(0.01)
    } else {
        rect.to_path(0.01)
    }
}

/// Build a closed ellipse path inscribed in `rect`.
pub fn ellipse_path(rect: Rect) -> BezPath {
    Ellipse::new(
        rect.center(),
        (rect.width() / 2.0, rect.height() / 2.0),
        0.0,
    )
    .to_path(0.01)
}

/// Arc or pie of the ellipse inscribed in `rect`, angles in degrees CCW from +x.
pub fn ellipse_arc_path(rect: Rect, start_deg: f64, end_deg: f64, pie: bool) -> BezPath {
    let c = rect.center();
    let radii = kurbo::Vec2::new(rect.width() / 2.0, rect.height() / 2.0);
    let mut sweep = (end_deg - start_deg).rem_euclid(360.0);
    if sweep == 0.0 {
        sweep = 360.0;
    }
    let arc = kurbo::Arc::new(c, radii, start_deg.to_radians(), sweep.to_radians(), 0.0);
    let mut path = BezPath::new();
    let start = Point::new(
        c.x + radii.x * start_deg.to_radians().cos(),
        c.y + radii.y * start_deg.to_radians().sin(),
    );
    if pie {
        path.move_to(c);
        path.line_to(start);
    } else {
        path.move_to(start);
    }
    arc.append_iter(0.1).for_each(|el| path.push(el));
    if pie {
        path.close_path();
    }
    path
}

/// Build a regular polygon or star inscribed in the ellipse that fills
/// `rect`, with its first vertex at the top (as the target design
/// draws them).
///
/// `points` is the number of vertices; `sharpness` in `0.0..1.0` pulls every
/// other vertex toward the centre, turning a polygon into a star.
pub fn polygon_path(rect: Rect, points: u32, sharpness: f64) -> BezPath {
    let c = rect.center();
    let rx = rect.width() / 2.0;
    let ry = rect.height() / 2.0;
    let mut path = BezPath::new();
    for (i, (x, y)) in polygon_unit_vertices(points, sharpness).enumerate() {
        let p = Point::new(c.x + rx * x, c.y + ry * y);
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    path
}

/// The fewest points a complex star has.
pub const COMPLEX_STAR_MIN_POINTS: u32 = 5;

/// The largest sharpness a complex star of `points` points takes: its
/// sides join vertices at most (points - 1) / 2 apart.
pub fn complex_star_max_sharpness(points: u32) -> u32 {
    let n = points.max(COMPLEX_STAR_MIN_POINTS);
    ((n - 1) / 2).saturating_sub(1).max(1)
}

/// A complex star: `points` vertices on the ellipse in `rect` (the first
/// at the top, then clockwise), each joined to the one `sharpness + 1`
/// further on, so the sides cross. When that step shares a factor with
/// the count the star is several closed subpaths (two triangles for six
/// points). Filled even-odd, the middle stays empty.
pub fn complex_star_path(rect: Rect, points: u32, sharpness: u32) -> BezPath {
    let n = points.clamp(COMPLEX_STAR_MIN_POINTS, 500) as usize;
    let step = sharpness.clamp(1, complex_star_max_sharpness(n as u32)) as usize + 1;
    let verts: Vec<Point> = {
        let c = rect.center();
        let (rx, ry) = (rect.width() / 2.0, rect.height() / 2.0);
        polygon_unit_vertices(n as u32, 0.0)
            .map(|(x, y)| Point::new(c.x + rx * x, c.y + ry * y))
            .collect()
    };
    let g = gcd(n, step);
    let mut path = BezPath::new();
    for start in 0..g {
        let mut i = start;
        path.move_to(verts[i]);
        for _ in 1..n / g {
            i = (i + step) % n;
            path.line_to(verts[i]);
        }
        path.close_path();
    }
    path
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a.max(1)
    } else {
        gcd(b, a % b)
    }
}

/// Vertices of a polygon or star on the unit circle, first at the top
/// (y up), then clockwise.
fn polygon_unit_vertices(points: u32, sharpness: f64) -> impl Iterator<Item = (f64, f64)> {
    let points = points.max(3) as usize;
    let star = sharpness > 0.0;
    let inner = 1.0 - sharpness.clamp(0.0, 1.0);
    let n = if star { points * 2 } else { points };
    (0..n).map(move |i| {
        let t = std::f64::consts::FRAC_PI_2 - i as f64 * std::f64::consts::TAU / n as f64;
        let k = if star && i % 2 == 1 { inner } else { 1.0 };
        (k * t.cos(), k * t.sin())
    })
}

/// The ellipse rectangle (the `rect` of a polygon shape) whose polygon or
/// star exactly fills `bounds`. Drawing tools drag the visible box of the
/// polygon; the model keeps the ellipse its outer vertices lie on.
pub fn polygon_rect_for_bounds(bounds: Rect, points: u32, sharpness: f64) -> Rect {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (x, y) in polygon_unit_vertices(points, sharpness) {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let rx = bounds.width() / (x1 - x0).max(1e-9);
    let ry = bounds.height() / (y1 - y0).max(1e-9);
    let cx = bounds.x0 - x0 * rx;
    let cy = bounds.y0 - y0 * ry;
    Rect::new(cx - rx, cy - ry, cx + rx, cy + ry)
}

/// Straight segments through the points.
pub fn polyline_path(points: &[Point], closed: bool) -> BezPath {
    let mut path = BezPath::new();
    for (i, p) in points.iter().enumerate() {
        if i == 0 {
            path.move_to(*p);
        } else {
            path.line_to(*p);
        }
    }
    if closed && points.len() > 2 {
        path.close_path();
    }
    path
}

/// Smooth cubic curve through the points (Catmull-Rom converted to Bezier),
/// the way a freehand stroke is turned into a curve.
pub fn smooth_path(points: &[Point], closed: bool) -> BezPath {
    let n = points.len();
    if n < 3 {
        return polyline_path(points, closed);
    }
    let mut path = BezPath::new();
    path.move_to(points[0]);
    let get = |i: isize| -> Point {
        let idx = i.clamp(0, n as isize - 1) as usize;
        points[idx]
    };
    for i in 0..n - 1 {
        let p0 = get(i as isize - 1);
        let p1 = get(i as isize);
        let p2 = get(i as isize + 1);
        let p3 = get(i as isize + 2);
        let c1 = p1 + (p2 - p0) / 6.0;
        let c2 = p2 - (p3 - p1) / 6.0;
        path.curve_to(c1, c2, p2);
    }
    if closed {
        path.close_path();
    }
    path
}

/// Drop points closer than `tolerance` to the line between their
/// neighbours (Ramer-Douglas-Peucker), used to thin freehand input.
pub fn simplify(points: &[Point], tolerance: f64) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }
    fn rdp(pts: &[Point], tol: f64, out: &mut Vec<Point>) {
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        let ab = b - a;
        let len2 = ab.hypot2();
        let mut max_d = 0.0;
        let mut idx = 0;
        for (i, p) in pts.iter().enumerate().skip(1).take(pts.len() - 2) {
            let d = if len2 < 1e-12 {
                (*p - a).hypot()
            } else {
                let t = ((*p - a).dot(ab) / len2).clamp(0.0, 1.0);
                (*p - (a + ab * t)).hypot()
            };
            if d > max_d {
                max_d = d;
                idx = i;
            }
        }
        if max_d > tol && idx > 0 {
            rdp(&pts[..=idx], tol, out);
            out.pop();
            rdp(&pts[idx..], tol, out);
        } else {
            out.push(a);
            out.push(b);
        }
    }
    let mut out = Vec::new();
    rdp(points, tolerance, &mut out);
    out
}

/// Axis-aligned bounds of a path after an affine transform.
pub fn transformed_bounds(path: &BezPath, transform: Affine) -> Rect {
    (transform * path.clone()).bounding_box()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_path_has_bounds() {
        let r = Rect::new(10.0, 20.0, 110.0, 70.0);
        let p = rect_path(r, 0.0);
        let b = p.bounding_box();
        assert!((b.x0 - 10.0).abs() < 1e-9 && (b.y1 - 70.0).abs() < 1e-9);
    }

    #[test]
    fn simplify_drops_collinear_points() {
        let pts: Vec<Point> = (0..10).map(|i| Point::new(i as f64, 0.0)).collect();
        assert_eq!(simplify(&pts, 0.1).len(), 2);
        let smooth = smooth_path(&pts, false);
        assert_eq!(smooth.elements().len(), 10);
    }

    #[test]
    fn star_has_twice_the_points() {
        let p = polygon_path(Rect::new(0.0, 0.0, 10.0, 10.0), 5, 0.5);
        let n = p
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::LineTo(_)))
            .count();
        assert_eq!(n, 9);
    }

    #[test]
    fn polygons_point_up() {
        // First vertex at the top centre (y up), as drawn by the target design.
        let p = polygon_path(Rect::new(0.0, 0.0, 10.0, 10.0), 5, 0.0);
        match p.elements()[0] {
            PathEl::MoveTo(q) => assert!((q.x - 5.0).abs() < 1e-9 && (q.y - 10.0).abs() < 1e-9),
            ref e => panic!("{e:?}"),
        }
    }

    #[test]
    fn complex_stars_cross_their_sides() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        let count =
            |p: &BezPath, f: fn(&PathEl) -> bool| p.elements().iter().filter(|e| f(e)).count();
        // A pentagram: one subpath of five vertices, every second one.
        let p = complex_star_path(r, 5, 1);
        assert_eq!(count(&p, |e| matches!(e, PathEl::MoveTo(_))), 1);
        assert_eq!(count(&p, |e| matches!(e, PathEl::LineTo(_))), 4);
        let pts: Vec<Point> = p
            .elements()
            .iter()
            .filter_map(|e| match e {
                PathEl::MoveTo(q) | PathEl::LineTo(q) => Some(*q),
                _ => None,
            })
            .collect();
        let verts = polygon_path(r, 5, 0.0);
        let PathEl::LineTo(v2) = verts.elements()[2] else {
            panic!("{verts:?}");
        };
        assert!((pts[1] - v2).hypot() < 1e-9, "{pts:?}");
        // Filled even-odd, the middle pentagon is a hole.
        assert!(p.winding(r.center()).abs() == 2);
        // Six points: two triangles.
        let p = complex_star_path(r, 6, 1);
        assert_eq!(count(&p, |e| matches!(e, PathEl::MoveTo(_))), 2);
        // Sharpness follows the point count.
        assert_eq!(complex_star_max_sharpness(5), 1);
        assert_eq!(complex_star_max_sharpness(9), 3);
        assert_eq!(complex_star_max_sharpness(12), 4);
        assert_eq!(complex_star_max_sharpness(0), 1);
        // Out of range values are brought in.
        assert_eq!(complex_star_path(r, 9, 50), complex_star_path(r, 9, 3));
        assert_eq!(complex_star_path(r, 1, 1), complex_star_path(r, 5, 1));
    }

    #[test]
    fn polygon_fills_the_dragged_box() {
        let drag = Rect::new(20.0, 10.0, 70.0, 40.0);
        for (n, sharp) in [(3, 0.0), (5, 0.0), (6, 0.0), (5, 0.5), (3, 0.2), (12, 0.3)] {
            let r = polygon_rect_for_bounds(drag, n, sharp);
            let b = polygon_path(r, n, sharp).bounding_box();
            assert!(
                (b.x0 - drag.x0).abs() < 1e-9
                    && (b.y0 - drag.y0).abs() < 1e-9
                    && (b.x1 - drag.x1).abs() < 1e-9
                    && (b.y1 - drag.y1).abs() < 1e-9,
                "{n} {sharp}: {b:?}"
            );
        }
    }
}
