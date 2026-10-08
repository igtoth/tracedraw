//! Geometry primitives. We re-export kurbo so the rest of the workspace
//! shares one set of types, and add the few helpers the editor needs.

pub use kurbo::{
    Affine, BezPath, Circle, Ellipse, Line, PathEl, Point, Rect, RoundedRect, Shape, Size, Vec2,
};

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

/// Build a regular polygon or star inscribed in `rect`.
///
/// `points` is the number of vertices; `sharpness` in `0.0..1.0` pulls every
/// other vertex toward the centre, turning a polygon into a star.
pub fn polygon_path(rect: Rect, points: u32, sharpness: f64) -> BezPath {
    let points = points.max(3) as usize;
    let c = rect.center();
    let rx = rect.width() / 2.0;
    let ry = rect.height() / 2.0;
    let inner = 1.0 - sharpness.clamp(0.0, 1.0);
    let mut path = BezPath::new();
    let n = if sharpness > 0.0 { points * 2 } else { points };
    for i in 0..n {
        let t = -std::f64::consts::FRAC_PI_2 + i as f64 * std::f64::consts::TAU / n as f64;
        let k = if sharpness > 0.0 && i % 2 == 1 {
            inner
        } else {
            1.0
        };
        let p = Point::new(c.x + rx * k * t.cos(), c.y + ry * k * t.sin());
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    path
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
}
