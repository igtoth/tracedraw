//! Shaping: weld, trim, intersect, simplify, front-minus-back, back-minus-
//! front and boundary. Implemented on flattened polygons with i_overlay,
//! so curves become dense polylines (0.02 mm tolerance). A curve-preserving
//! version is on the roadmap.

use crate::geometry::{BezPath, PathEl, Point};
use i_overlay::core::fill_rule::FillRule as IFill;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

const TOL: f64 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Weld,
    Intersect,
    /// Subject minus clip.
    Trim,
    /// Symmetric difference.
    Exclude,
}

/// Flatten a path into closed contours (one per subpath).
pub fn contours(path: &BezPath) -> Vec<Vec<[f64; 2]>> {
    let mut out: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut cur: Vec<[f64; 2]> = Vec::new();
    kurbo::flatten(path.elements().iter().copied(), TOL, &mut |el| match el {
        PathEl::MoveTo(p) => {
            if cur.len() >= 3 {
                out.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
            cur.push([p.x, p.y]);
        }
        PathEl::LineTo(p) => cur.push([p.x, p.y]),
        PathEl::ClosePath => {}
        _ => {}
    });
    if cur.len() >= 3 {
        out.push(cur);
    }
    out
}

/// Build a path from shapes (outer contour + holes each).
fn from_shapes(shapes: Vec<Vec<Vec<[f64; 2]>>>) -> BezPath {
    let mut path = BezPath::new();
    for shape in shapes {
        for contour in shape {
            let mut first = true;
            for p in contour {
                let pt = Point::new(p[0], p[1]);
                if first {
                    path.move_to(pt);
                    first = false;
                } else {
                    path.line_to(pt);
                }
            }
            path.close_path();
        }
    }
    path
}

/// Combine `subject` and `clip` (both in the same coordinate space).
pub fn overlay(subject: &BezPath, clip: &BezPath, op: Op) -> BezPath {
    let subj = contours(subject);
    let clp = contours(clip);
    let rule = match op {
        Op::Weld => OverlayRule::Union,
        Op::Intersect => OverlayRule::Intersect,
        Op::Trim => OverlayRule::Difference,
        Op::Exclude => OverlayRule::Xor,
    };
    let result = subj.overlay(&clp, rule, IFill::EvenOdd);
    from_shapes(result)
}

/// Union of one path with itself: removes self-overlaps (Simplify).
pub fn simplify(path: &BezPath) -> BezPath {
    let subj = contours(path);
    let empty: Vec<Vec<[f64; 2]>> = Vec::new();
    let result = subj.overlay(&empty, OverlayRule::Subject, IFill::NonZero);
    from_shapes(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Rect, Shape as _};

    fn square(x: f64) -> BezPath {
        Rect::new(x, 0.0, x + 10.0, 10.0).to_path(0.01)
    }

    #[test]
    fn weld_two_overlapping_squares() {
        let w = overlay(&square(0.0), &square(5.0), Op::Weld);
        let b = w.bounding_box();
        assert!((b.x0 - 0.0).abs() < 1e-6 && (b.x1 - 15.0).abs() < 1e-6);
        assert!((w.area().abs() - 150.0).abs() < 1e-3);
    }

    #[test]
    fn intersect_and_trim() {
        let i = overlay(&square(0.0), &square(5.0), Op::Intersect);
        assert!((i.area().abs() - 50.0).abs() < 1e-3);
        let t = overlay(&square(0.0), &square(5.0), Op::Trim);
        assert!((t.area().abs() - 50.0).abs() < 1e-3);
        assert!((t.bounding_box().x1 - 5.0).abs() < 1e-6);
    }
}
