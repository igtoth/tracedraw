//! Geometry primitives. We re-export kurbo so the rest of the workspace
//! shares one set of types, and add the few helpers the editor needs.

pub use kurbo::{Affine, BezPath, Circle, Ellipse, Line, PathEl, Point, Rect, RoundedRect, Shape, Size, Vec2};

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
    Ellipse::new(rect.center(), (rect.width() / 2.0, rect.height() / 2.0), 0.0).to_path(0.01)
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
        let k = if sharpness > 0.0 && i % 2 == 1 { inner } else { 1.0 };
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
    fn star_has_twice_the_points() {
        let p = polygon_path(Rect::new(0.0, 0.0, 10.0, 10.0), 5, 0.5);
        let n = p.elements().iter().filter(|e| matches!(e, PathEl::LineTo(_))).count();
        assert_eq!(n, 9);
    }
}
