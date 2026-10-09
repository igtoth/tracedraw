//! Static effects on paths: blend (interpolate between two shapes), extrude
//! (parallel), distort (push/pull, zipper, twister) and brush-style
//! deformations (smear, twirl, smooth). All work on flattened polylines
//! and return new paths; the target design keeps these live, we bake them.

use crate::geometry::{BezPath, PathEl, Point, Vec2};

/// Resample the first subpath of a path into `n` evenly spaced points.
/// Vertices of the flattened first subpath (0.05 mm tolerance), with the
/// first point repeated at the end when the subpath is closed. Corners are
/// kept exactly, unlike `resample`.
pub fn flat_points(path: &BezPath) -> Vec<Point> {
    let mut pts: Vec<Point> = Vec::new();
    kurbo::flatten(path.elements().iter().copied(), 0.05, &mut |el| match el {
        PathEl::MoveTo(p) => {
            if pts.is_empty() {
                pts.push(p)
            }
        }
        PathEl::LineTo(p) => pts.push(p),
        PathEl::ClosePath => {
            if let Some(f) = pts.first().copied() {
                pts.push(f);
            }
        }
        _ => {}
    });
    pts
}

pub fn resample(path: &BezPath, n: usize) -> Vec<Point> {
    let pts = flat_points(path);
    if pts.len() < 2 || n < 2 {
        return pts;
    }
    let total: f64 = pts.windows(2).map(|w| (w[1] - w[0]).hypot()).sum();
    if total <= 0.0 {
        return vec![pts[0]; n];
    }
    let mut out = Vec::with_capacity(n);
    let mut seg = 0;
    let mut seg_start = 0.0;
    for i in 0..n {
        let target = total * i as f64 / (n - 1) as f64;
        while seg + 1 < pts.len() - 1 && seg_start + (pts[seg + 1] - pts[seg]).hypot() < target {
            seg_start += (pts[seg + 1] - pts[seg]).hypot();
            seg += 1;
        }
        let len = (pts[seg + 1] - pts[seg]).hypot().max(1e-12);
        let t = ((target - seg_start) / len).clamp(0.0, 1.0);
        out.push(pts[seg].lerp(pts[seg + 1], t));
    }
    out
}

/// Closed polygon through points.
pub fn polygon(pts: &[Point]) -> BezPath {
    let mut p = BezPath::new();
    for (i, pt) in pts.iter().enumerate() {
        if i == 0 {
            p.move_to(*pt);
        } else {
            p.line_to(*pt);
        }
    }
    if !pts.is_empty() {
        p.close_path();
    }
    p
}

/// Intermediate shapes between `a` and `b` (excluding both), `steps` of them.
pub fn blend(a: &BezPath, b: &BezPath, steps: usize, samples: usize) -> Vec<BezPath> {
    let pa = resample(a, samples);
    let pb = resample(b, samples);
    if pa.len() != pb.len() || pa.is_empty() {
        return Vec::new();
    }
    // Match the two outlines by shape, not by position: wind b the way a
    // is wound, then start b at the point whose direction from b's centre
    // is nearest the direction of a's start from a's centre. Otherwise the
    // intermediate steps twist through the middle.
    let centroid = |pts: &[Point]| {
        let n = pts.len().max(1) as f64;
        Point::new(
            pts.iter().map(|p| p.x).sum::<f64>() / n,
            pts.iter().map(|p| p.y).sum::<f64>() / n,
        )
    };
    let area = |pts: &[Point]| -> f64 {
        pts.iter()
            .zip(pts.iter().cycle().skip(1))
            .map(|(p, q)| p.x * q.y - q.x * p.y)
            .sum()
    };
    let mut pb = pb;
    if area(&pa) * area(&pb) < 0.0 {
        pb.reverse();
    }
    let (ca, cb) = (centroid(&pa), centroid(&pb));
    let da = pa[0] - ca;
    let start = (0..pb.len())
        .min_by(|i, j| {
            let di = (pb[*i] - cb) - da;
            let dj = (pb[*j] - cb) - da;
            di.hypot()
                .partial_cmp(&dj.hypot())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0);
    let pb: Vec<Point> = (0..pb.len()).map(|i| pb[(i + start) % pb.len()]).collect();
    (1..=steps)
        .map(|k| {
            let t = k as f64 / (steps + 1) as f64;
            polygon(
                &pa.iter()
                    .zip(&pb)
                    .map(|(x, y)| x.lerp(*y, t))
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

/// Parallel extrusion: side faces from each edge toward `depth` offset.
/// Returns the side polygons (back to front order) and the back face.
pub fn extrude(path: &BezPath, depth: Vec2) -> (Vec<BezPath>, BezPath) {
    let pts = resample(path, 64);
    let mut sides = Vec::new();
    for w in pts.windows(2) {
        sides.push(polygon(&[w[0], w[1], w[1] + depth, w[0] + depth]));
    }
    let back = polygon(&pts.iter().map(|p| *p + depth).collect::<Vec<_>>());
    (sides, back)
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum Distort {
    /// Push (negative) or pull (positive) points toward/away from the centre.
    PushPull { amount: f64 },
    /// Zigzag the outline with the given amplitude and frequency.
    Zipper { amplitude: f64, frequency: u32 },
    /// Rotate points around the centre proportionally to their distance.
    Twister { angle_deg: f64 },
}

pub fn distort(path: &BezPath, how: Distort) -> BezPath {
    use crate::geometry::Shape as _;
    let b = path.bounding_box();
    let c = b.center();
    let r_max = (b.width().max(b.height()) / 2.0).max(1e-9);
    let samples = 160;
    let pts = resample(path, samples);
    let out: Vec<Point> = pts
        .iter()
        .enumerate()
        .map(|(i, p)| match how {
            Distort::PushPull { amount } => {
                let v = *p - c;
                *p + v * (amount / 100.0)
            }
            Distort::Zipper {
                amplitude,
                frequency,
            } => {
                let prev = pts[(i + pts.len() - 1) % pts.len()];
                let next = pts[(i + 1) % pts.len()];
                let tangent = (next - prev).normalize();
                let n = Vec2::new(-tangent.y, tangent.x);
                let phase = (i as f64 / samples as f64) * frequency as f64 * std::f64::consts::TAU;
                *p + n * (amplitude * phase.sin())
            }
            Distort::Twister { angle_deg } => {
                let v = *p - c;
                let a = angle_deg.to_radians() * (v.hypot() / r_max);
                c + Vec2::new(v.x * a.cos() - v.y * a.sin(), v.x * a.sin() + v.y * a.cos())
            }
        })
        .collect();
    crate::geometry::smooth_path(&out[..out.len().saturating_sub(1)], true)
}

/// Push points within `radius` of `at` along `delta` (Smear tool).
pub fn smear(path: &BezPath, at: Point, delta: Vec2, radius: f64) -> BezPath {
    let closed = path
        .elements()
        .iter()
        .any(|e| matches!(e, PathEl::ClosePath));
    let pts = densify(path, radius / 3.0);
    let out: Vec<Point> = pts
        .iter()
        .map(|p| {
            let d = (*p - at).hypot();
            if d < radius {
                let w = 1.0 - d / radius;
                *p + delta * (w * w)
            } else {
                *p
            }
        })
        .collect();
    crate::geometry::smooth_path(&out, closed)
}

/// Rotate points within `radius` of `at` by up to `angle` (Twirl tool).
pub fn twirl(path: &BezPath, at: Point, angle: f64, radius: f64) -> BezPath {
    let closed = path
        .elements()
        .iter()
        .any(|e| matches!(e, PathEl::ClosePath));
    let pts = densify(path, radius / 3.0);
    let out: Vec<Point> = pts
        .iter()
        .map(|p| {
            let v = *p - at;
            let d = v.hypot();
            if d < radius {
                let a = angle * (1.0 - d / radius);
                at + Vec2::new(v.x * a.cos() - v.y * a.sin(), v.x * a.sin() + v.y * a.cos())
            } else {
                *p
            }
        })
        .collect();
    crate::geometry::smooth_path(&out, closed)
}

/// Reduce nodes near `at` (Smooth tool): simplify and re-smooth.
pub fn smooth(path: &BezPath, at: Point, radius: f64, tolerance: f64) -> BezPath {
    let closed = path
        .elements()
        .iter()
        .any(|e| matches!(e, PathEl::ClosePath));
    let pts = densify(path, radius / 4.0);
    // Only simplify the run of points near the brush; keep others exact.
    let mut out: Vec<Point> = Vec::new();
    let mut run: Vec<Point> = Vec::new();
    let flush = |run: &mut Vec<Point>, out: &mut Vec<Point>| {
        if run.len() > 2 {
            out.extend(crate::geometry::simplify(run, tolerance));
        } else {
            out.append(run);
        }
        run.clear();
    };
    for p in pts {
        if (p - at).hypot() < radius {
            run.push(p);
        } else {
            flush(&mut run, &mut out);
            out.push(p);
        }
    }
    flush(&mut run, &mut out);
    crate::geometry::smooth_path(&out, closed)
}

/// Flatten and insert points so no segment is longer than `max_len`.
fn densify(path: &BezPath, max_len: f64) -> Vec<Point> {
    let mut pts: Vec<Point> = Vec::new();
    kurbo::flatten(path.elements().iter().copied(), 0.05, &mut |el| match el {
        PathEl::MoveTo(p) => {
            if pts.is_empty() {
                pts.push(p)
            }
        }
        PathEl::LineTo(p) => pts.push(p),
        _ => {}
    });
    let mut out = Vec::new();
    for w in pts.windows(2) {
        let len = (w[1] - w[0]).hypot();
        let n = (len / max_len.max(0.05)).ceil().max(1.0) as usize;
        for k in 0..n {
            out.push(w[0].lerp(w[1], k as f64 / n as f64));
        }
    }
    if let Some(l) = pts.last() {
        out.push(*l);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Rect, Shape as _};

    #[test]
    fn blend_between_squares_is_in_between() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.01);
        let b = Rect::new(20.0, 0.0, 30.0, 10.0).to_path(0.01);
        let mid = blend(&a, &b, 1, 32);
        assert_eq!(mid.len(), 1);
        let c = mid[0].bounding_box().center();
        assert!((c.x - 15.0).abs() < 0.5, "{c:?}");
    }

    #[test]
    fn blend_between_offset_ellipses_keeps_their_size_midway() {
        // Two equal ellipses, one down and to the right of the other: the
        // middle step must be the same size, not collapsed by a twisted
        // point correspondence.
        let a = crate::geometry::Ellipse::from_rect(Rect::new(0.0, 40.0, 30.0, 60.0)).to_path(0.01);
        let b = crate::geometry::Ellipse::from_rect(Rect::new(100.0, 0.0, 130.0, 20.0)).to_path(0.01);
        let steps = blend(&a, &b, 5, 64);
        assert_eq!(steps.len(), 5);
        for s in &steps {
            let bb = s.bounding_box();
            assert!((bb.width() - 30.0).abs() < 1.0, "{bb:?}");
            assert!((bb.height() - 20.0).abs() < 1.0, "{bb:?}");
        }
        // A reversed second outline is matched the same way.
        let mut rb = crate::nodes::reverse(&b);
        if rb.elements().is_empty() {
            rb = b.clone();
        }
        for s in blend(&a, &rb, 3, 64) {
            let bb = s.bounding_box();
            assert!((bb.width() - 30.0).abs() < 1.0, "{bb:?}");
        }
    }

    #[test]
    fn twister_keeps_bounds_roughly() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.01);
        let t = distort(&a, Distort::Twister { angle_deg: 45.0 });
        let b = t.bounding_box();
        assert!(b.width() > 8.0 && b.width() < 16.0);
    }

    #[test]
    fn extrude_makes_side_faces() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.01);
        let (sides, back) = extrude(&a, Vec2::new(5.0, 5.0));
        assert!(!sides.is_empty());
        assert!((back.bounding_box().x0 - 5.0).abs() < 1e-6);
    }
}
