//! Static effects on paths: blend (interpolate between two shapes), extrude
//! (parallel), distort (push/pull, zipper, twister) and brush-style
//! deformations (smear, twirl, smooth). All work on flattened polylines
//! and return new paths; the results are baked, not kept live.

use crate::geometry::{Affine, BezPath, PathEl, Point, Vec2};

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
    // Outlines of several parts (letters, shapes with holes) morph part by
    // part: the n-th part of one into the n-th part of the other, in
    // order; a part without a partner grows from, or shrinks to, a point
    // at its own centre. One polygon through every part would join them
    // into a tangle.
    let parts = |p: &BezPath| -> Vec<BezPath> {
        subpaths(p)
            .into_iter()
            .map(|(sp, _)| sp)
            .filter(|sp| sp.elements().len() > 1)
            .collect()
    };
    let (ka, kb) = (parts(a), parts(b));
    if ka.len() > 1 || kb.len() > 1 {
        let n = ka.len().max(kb.len());
        let per = (samples / 2).clamp(24, 96);
        let mut out = vec![BezPath::new(); steps];
        for i in 0..n {
            let (x, y) = match (ka.get(i), kb.get(i)) {
                (Some(x), Some(y)) => (x.clone(), y.clone()),
                (Some(x), None) => (x.clone(), point_path(x)),
                (None, Some(y)) => (point_path(y), y.clone()),
                (None, None) => continue,
            };
            for (k, step) in blend_one(&x, &y, steps, per).into_iter().enumerate() {
                if let Some(o) = out.get_mut(k) {
                    o.extend(step.elements().iter().copied());
                }
            }
        }
        return out;
    }
    blend_one(a, b, steps, samples)
}

/// A closed path collapsed to the centre of `p`'s outline (a part that
/// appears or vanishes in a blend).
fn point_path(p: &BezPath) -> BezPath {
    let pts = resample(p, 32);
    let n = pts.len().max(1) as f64;
    let c = Point::new(
        pts.iter().map(|q| q.x).sum::<f64>() / n,
        pts.iter().map(|q| q.y).sum::<f64>() / n,
    );
    let mut out = BezPath::new();
    out.move_to(c);
    out.line_to(c + Vec2::new(1e-6, 0.0));
    out.line_to(c + Vec2::new(0.0, 1e-6));
    out.close_path();
    out
}

/// The morph of one outline into another (each taken as one closed run of
/// points).
fn blend_one(a: &BezPath, b: &BezPath, steps: usize, samples: usize) -> Vec<BezPath> {
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

pub use crate::distort::{distort, Distort};

/// The subpaths of `path`, each with whether it is closed.
pub fn subpaths(path: &BezPath) -> Vec<(BezPath, bool)> {
    let mut out = Vec::new();
    let mut cur = BezPath::new();
    let mut closed = false;
    for el in path.elements() {
        match el {
            PathEl::MoveTo(_) => {
                if !cur.elements().is_empty() {
                    out.push((std::mem::take(&mut cur), closed));
                }
                closed = false;
                cur.push(*el);
            }
            PathEl::ClosePath => {
                closed = true;
                cur.push(*el);
            }
            _ => cur.push(*el),
        }
    }
    if !cur.elements().is_empty() {
        out.push((cur, closed));
    }
    out
}

/// The fit tolerance of the curves a brush step leaves (mm, on the page):
/// as few nodes as hold the moved points within it.
const FIT_TOLERANCE: f64 = 0.05;

/// A brush step on the parts of `path` the nib (`radius` around `at`,
/// both in page space) reaches. `to_page` takes the path's own space to
/// the page, so the nib is the circle drawn on screen whatever the
/// object's scale, stretch or turn. Only the runs of segments that come
/// within the nib are measured into points (`max_len` apart on the page),
/// passed through `f` in page space and fitted back into curves (straight
/// lines with `sharp`); every other segment, node and corner stays
/// exactly as it was, and so does every subpath the nib does not reach.
/// `f` gets the points and whether they are a whole closed subpath.
fn brush_local(
    path: &BezPath,
    to_page: Affine,
    at: Point,
    radius: f64,
    max_len: f64,
    sharp: bool,
    mut f: impl FnMut(Vec<Point>, bool) -> Vec<Point>,
) -> BezPath {
    use kurbo::{ParamCurve, ParamCurveArclen, ParamCurveNearest, PathSeg, Shape as _};
    let from_page = to_page.inverse();
    let reach = radius;
    let reached = |seg: &PathSeg| -> bool {
        let page = to_page * *seg;
        // A quick test on the bounds first.
        let b = page.bounding_box().inflate(reach, reach);
        if !b.contains(at) {
            return false;
        }
        page.nearest(at, 1e-6).distance_sq < reach * reach
    };
    // Where a page-space segment is inside the nib: the first and last
    // parameter (0 to 1) within reach, found on 64 samples and refined by
    // halving, so the cuts sit on the nib's edge.
    let reach_span = |page: &PathSeg| -> (f64, f64) {
        let inside = |t: f64| (page.eval(t) - at).hypot() < reach;
        const N: usize = 64;
        let ts: Vec<f64> = (0..=N).map(|k| k as f64 / N as f64).collect();
        let first = ts.iter().position(|t| inside(*t));
        let last = ts.iter().rposition(|t| inside(*t));
        let (Some(a), Some(b)) = (first, last) else {
            return (0.0, 1.0);
        };
        let refine = |mut out_t: f64, mut in_t: f64| {
            for _ in 0..40 {
                let m = 0.5 * (out_t + in_t);
                if inside(m) {
                    in_t = m;
                } else {
                    out_t = m;
                }
            }
            out_t
        };
        let t_in = if a == 0 {
            0.0
        } else {
            refine(ts[a - 1], ts[a])
        };
        let t_out = if b == N {
            1.0
        } else {
            refine(ts[b + 1], ts[b])
        };
        (t_in, t_out)
    };
    // Points along the page-space segments, `max_len` apart, first and
    // last exact.
    let sample = |segs: &[PathSeg]| -> Vec<Point> {
        let mut pts = Vec::new();
        for seg in segs {
            let page = to_page * *seg;
            let len = page.arclen(1e-3);
            let n = (len / max_len.max(0.05)).ceil().clamp(1.0, 10_000.0) as usize;
            if pts.is_empty() {
                pts.push(page.eval(0.0));
            }
            for k in 1..=n {
                pts.push(page.eval(k as f64 / n as f64));
            }
        }
        pts
    };
    // Moved points become curves (or straight lines with `sharp`); the
    // stretches `f` left alone are fitted as curves either way, so a
    // jagged edge does not leave a trail of straight bits beside it.
    let fit = |orig: &[Point], pts: &[Point], closed: bool| -> BezPath {
        let simplify = |poly: BezPath| {
            kurbo::simplify::simplify_bezpath(
                poly,
                FIT_TOLERANCE,
                &kurbo::simplify::SimplifyOptions::default(),
            )
        };
        if pts.len() < 3 {
            return crate::geometry::polyline_path(pts, closed);
        }
        if !sharp {
            return simplify(crate::geometry::smooth_path(pts, closed));
        }
        let still = |i: usize| orig.get(i).is_some_and(|o| (*o - pts[i]).hypot() < 1e-9);
        let mut out = BezPath::new();
        out.move_to(pts[0]);
        let mut i = 0;
        while i + 1 < pts.len() {
            if still(i) && still(i + 1) {
                let mut j = i + 1;
                while j + 1 < pts.len() && still(j + 1) {
                    j += 1;
                }
                let part = simplify(crate::geometry::smooth_path(&pts[i..=j], false));
                out.extend(part.elements().iter().skip(1).copied());
                i = j;
            } else {
                out.line_to(pts[i + 1]);
                i += 1;
            }
        }
        if closed {
            out.close_path();
        }
        out
    };
    let push_seg = |out: &mut BezPath, seg: &PathSeg| match seg {
        PathSeg::Line(l) => out.line_to(l.p1),
        PathSeg::Quad(q) => out.quad_to(q.p1, q.p2),
        PathSeg::Cubic(c) => out.curve_to(c.p1, c.p2, c.p3),
    };
    let mut out = BezPath::new();
    for (sub, closed) in subpaths(path) {
        let segs: Vec<PathSeg> = sub.segments().collect();
        let hit: Vec<bool> = segs.iter().map(reached).collect();
        if !hit.iter().any(|h| *h) {
            out.extend(sub.elements().iter().copied());
            continue;
        }
        if hit.iter().all(|h| *h) {
            // The whole subpath is in reach.
            let mut pts = sample(&segs);
            if closed && pts.len() > 1 {
                pts.pop();
            }
            let moved = f(pts.clone(), closed);
            if moved.is_empty() {
                out.extend(sub.elements().iter().copied());
                continue;
            }
            out.extend(
                (from_page * fit(&pts, &moved, closed))
                    .elements()
                    .iter()
                    .copied(),
            );
            continue;
        }
        // Start a closed subpath at a segment out of reach, so a run of
        // reached segments never wraps around the start.
        let n = segs.len();
        let start = if closed && hit[0] {
            hit.iter().position(|h| !*h).unwrap_or(0)
        } else {
            0
        };
        let order: Vec<usize> = (0..n).map(|i| (i + start) % n).collect();
        let first = order[0];
        out.move_to(segs[first].eval(0.0));
        let mut i = 0;
        while i < n {
            let k = order[i];
            if !hit[k] {
                push_seg(&mut out, &segs[k]);
                i += 1;
                continue;
            }
            let mut j = i;
            while j < n && hit[order[j]] {
                j += 1;
            }
            let mut run: Vec<PathSeg> = order[i..j].iter().map(|k| segs[*k]).collect();
            // Cut the run's first and last segments where they enter and
            // leave the nib: the parts outside stay exactly as they were.
            let last = run.len() - 1;
            let (t_in, _) = reach_span(&(to_page * run[0]));
            let (_, t_out) = reach_span(&(to_page * run[last]));
            let tail = (t_out < 1.0).then(|| run[last].subsegment(t_out..1.0));
            if t_out < 1.0 {
                let t0 = if last == 0 { t_in } else { 0.0 };
                run[last] = run[last].subsegment(0.0..t_out);
                if last == 0 && t0 > 0.0 {
                    // Both cuts on one segment: t_out is on the whole one.
                    let head = segs[order[i]].subsegment(0.0..t0);
                    push_seg(&mut out, &head);
                    run[0] = segs[order[i]].subsegment(t0..t_out);
                }
            }
            if !(last == 0 && t_out < 1.0) && t_in > 0.0 {
                let head = run[0].subsegment(0.0..t_in);
                push_seg(&mut out, &head);
                run[0] = run[0].subsegment(t_in..1.0);
            }
            let pts = sample(&run);
            let (a, b) = (pts[0], pts[pts.len() - 1]);
            let mut moved = f(pts.clone(), false);
            // The run's ends are nodes the nib does not reach; they stay,
            // so the run joins its neighbours exactly.
            if let Some(p) = moved.first_mut() {
                *p = a;
            }
            if let Some(p) = moved.last_mut() {
                *p = b;
            }
            if moved.len() < 2 {
                for s in &run {
                    push_seg(&mut out, s);
                }
            } else {
                let fitted = from_page * fit(&pts, &moved, false);
                let mut els = fitted.elements().iter().copied();
                els.next(); // its MoveTo: the pen is already there
                let mut pen = from_page * a;
                for el in els {
                    match el {
                        PathEl::MoveTo(_) | PathEl::ClosePath => {}
                        PathEl::LineTo(p) => {
                            out.line_to(p);
                            pen = p;
                        }
                        PathEl::QuadTo(c, p) => {
                            out.quad_to(c, p);
                            pen = p;
                        }
                        PathEl::CurveTo(c1, c2, p) => {
                            out.curve_to(c1, c2, p);
                            pen = p;
                        }
                    }
                }
                // Land exactly on the cut point.
                let end = run[run.len() - 1].eval(1.0);
                if (pen - end).hypot() > 1e-9 {
                    out.line_to(end);
                }
            }
            if let Some(t) = tail {
                push_seg(&mut out, &t);
            }
            i = j;
        }
        if closed {
            out.close_path();
        }
    }
    out
}

/// The nib's weight at distance `d`: 1 at the middle, 0 at the edge.
fn falloff(d: f64, radius: f64) -> f64 {
    if d >= radius {
        0.0
    } else {
        1.0 - d / radius
    }
}

/// Push points within `radius` of `at` along `delta` (Smear tool). `at`,
/// `delta` and `radius` are in page space; `to_page` takes the path there.
pub fn smear(path: &BezPath, to_page: Affine, at: Point, delta: Vec2, radius: f64) -> BezPath {
    brush_local(path, to_page, at, radius, radius / 6.0, false, |pts, _| {
        pts.into_iter()
            .map(|p| {
                let w = falloff((p - at).hypot(), radius);
                p + delta * (w * w)
            })
            .collect()
    })
}

/// Rotate points within `radius` of `at` by up to `angle` (Twirl tool).
pub fn twirl(path: &BezPath, to_page: Affine, at: Point, angle: f64, radius: f64) -> BezPath {
    brush_local(path, to_page, at, radius, radius / 6.0, false, |pts, _| {
        pts.into_iter()
            .map(|p| {
                let v = p - at;
                let w = falloff(v.hypot(), radius);
                if w <= 0.0 {
                    return p;
                }
                let a = angle * w;
                at + Vec2::new(v.x * a.cos() - v.y * a.sin(), v.x * a.sin() + v.y * a.cos())
            })
            .collect()
    })
}

/// Reduce nodes near `at` (Smooth tool): simplify and re-smooth the run of
/// points within the nib.
pub fn smooth(path: &BezPath, to_page: Affine, at: Point, radius: f64, tolerance: f64) -> BezPath {
    brush_local(path, to_page, at, radius, radius / 4.0, false, |pts, _| {
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
        out
    })
}

/// Pull (`amount` > 0) or push (< 0) the nodes and control points within
/// `radius` of `at` towards or away from it (Attract and Repel tools);
/// nearer ones move more, none past the middle. Distances are measured on
/// the page; points out of reach stay exactly as they were.
pub fn attract(path: &BezPath, to_page: Affine, at: Point, radius: f64, amount: f64) -> BezPath {
    let from_page = to_page.inverse();
    let f = |p: Point| -> Point {
        let q = to_page * p;
        let v = at - q;
        let d = v.hypot();
        if d > radius || d < 1e-9 {
            return p;
        }
        let k = (1.0 - d / radius) * amount;
        from_page * (q + v * (k / d).min(0.9))
    };
    let mut out = BezPath::new();
    for el in path.elements() {
        out.push(match *el {
            PathEl::MoveTo(p) => PathEl::MoveTo(f(p)),
            PathEl::LineTo(p) => PathEl::LineTo(f(p)),
            PathEl::QuadTo(a, p) => PathEl::QuadTo(f(a), f(p)),
            PathEl::CurveTo(a, b, p) => PathEl::CurveTo(f(a), f(b), f(p)),
            PathEl::ClosePath => PathEl::ClosePath,
        });
    }
    out
}

/// A jagged edge within `radius` of `at` (Roughen tool): the curve in
/// points `spacing` apart, each within the nib pushed off the curve by up
/// to `amount` (less towards the nib's edge), joined by straight lines.
/// The spikes come from the points' places, so a step over the same spot
/// gives the same spikes.
pub fn roughen(
    path: &BezPath,
    to_page: Affine,
    at: Point,
    radius: f64,
    amount: f64,
    spacing: f64,
) -> BezPath {
    brush_local(
        path,
        to_page,
        at,
        radius,
        spacing.max(0.05),
        true,
        |pts, closed| {
            let n = pts.len();
            if n < 3 {
                return pts;
            }
            (0..n)
                .map(|i| {
                    let p = pts[i];
                    let w = falloff((p - at).hypot(), radius);
                    if w <= 0.0 {
                        return p;
                    }
                    let (prev, next) = if closed {
                        (pts[(i + n - 1) % n], pts[(i + 1) % n])
                    } else {
                        (pts[i.saturating_sub(1)], pts[(i + 1).min(n - 1)])
                    };
                    let t = next - prev;
                    let len = t.hypot();
                    if len < 1e-12 {
                        return p;
                    }
                    let normal = Vec2::new(-t.y / len, t.x / len);
                    // A number from -1 to 1 that depends on the place only.
                    let h = ((p.x * 12.9898 + p.y * 78.233).sin() * 43758.5453).fract();
                    let r = 2.0 * h.abs() - 1.0;
                    p + normal * (r * amount * w)
                })
                .collect()
        },
    )
}

/// One step of a Smudge stroke: the outline within the nib around the
/// pointer's last place `at` is pulled along the pointer's move `delta`
/// with a soft, rounded falloff (`(1 - (d/r)^2)^2`), so a drag that starts
/// inside a closed shape and goes out pulls a rounded extension out of
/// it, and one that starts outside and goes in pushes a dent. `radius` is
/// the nib at the start of the stroke; it narrows along the stroke by the
/// distance `travelled` so far, faster at a higher `pressure` (0 to 1):
/// `r = radius * max(0.6, 1 - pressure * travelled / (12 radius))`.
pub fn smudge(
    path: &BezPath,
    to_page: Affine,
    at: Point,
    delta: Vec2,
    radius: f64,
    travelled: f64,
    pressure: f64,
) -> BezPath {
    let r = smudge_radius(radius, travelled, pressure);
    brush_local(path, to_page, at, r, r / 8.0, false, |pts, _| {
        pts.into_iter()
            .map(|p| {
                let d = (p - at).hypot() / r;
                if d >= 1.0 {
                    return p;
                }
                let w = (1.0 - d * d) * (1.0 - d * d);
                p + delta * w
            })
            .collect()
    })
}

/// The Smudge nib after `travelled` mm of the stroke (see [`smudge`]).
pub fn smudge_radius(radius: f64, travelled: f64, pressure: f64) -> f64 {
    let p = pressure.clamp(0.0, 1.0);
    radius * (1.0 - p * travelled / (12.0 * radius.max(1e-6))).max(0.6)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Rect, Shape as _};

    /// The brushes change only the subpaths the nib reaches, keep open
    /// curves open and closed ones closed.
    #[test]
    fn brushes_touch_only_the_subpaths_they_reach() {
        let mut two = Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.01);
        let far = Rect::new(100.0, 0.0, 110.0, 10.0).to_path(0.01);
        two.extend(far.elements().iter().copied());
        let at = Point::new(10.0, 5.0);
        for out in [
            smear(&two, Affine::IDENTITY, at, Vec2::new(3.0, 0.0), 4.0),
            twirl(&two, Affine::IDENTITY, at, 0.5, 4.0),
            smooth(&two, Affine::IDENTITY, at, 4.0, 0.3),
            roughen(&two, Affine::IDENTITY, at, 4.0, 1.0, 0.5),
        ] {
            let subs = subpaths(&out);
            assert_eq!(subs.len(), 2);
            assert!(subs.iter().all(|(_, closed)| *closed));
            // The far square is exactly as it was.
            assert_eq!(subs[1].0, far);
            assert_ne!(subs[0].0.bounding_box(), Rect::ZERO);
        }
        // Smear pushed the near square's right side out.
        let pushed = smear(&two, Affine::IDENTITY, at, Vec2::new(3.0, 0.0), 4.0);
        assert!(subpaths(&pushed)[0].0.bounding_box().x1 > 11.0);
        // An open curve stays open.
        let mut line = BezPath::new();
        line.move_to(Point::new(0.0, 0.0));
        line.line_to(Point::new(20.0, 0.0));
        let rough = roughen(
            &line,
            Affine::IDENTITY,
            Point::new(10.0, 0.0),
            5.0,
            1.0,
            0.5,
        );
        assert_eq!(subpaths(&rough).len(), 1);
        assert!(!subpaths(&rough)[0].1);
        let b = rough.bounding_box();
        assert!(b.height() > 0.2 && b.height() <= 2.0, "{b:?}");
        // Attract pulls the corners near the nib inwards; repel pushes.
        let sq = Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.01);
        let c = Point::new(8.0, 8.0);
        // How far out the corner nearest the nib (10, 10) now is.
        let reach = |p: &BezPath| {
            p.elements()
                .iter()
                .filter_map(|e| match e {
                    PathEl::MoveTo(q) | PathEl::LineTo(q) => Some(q.x + q.y),
                    _ => None,
                })
                .fold(f64::MIN, f64::max)
        };
        let (pulled, pushed) = (
            attract(&sq, Affine::IDENTITY, c, 5.0, 1.0),
            attract(&sq, Affine::IDENTITY, c, 5.0, -1.0),
        );
        assert!(reach(&pulled) < 20.0 && reach(&pushed) > 20.0);
    }

    /// The nodes of `p` (ends of every segment), rounded to 1e-6.
    fn nodes(p: &BezPath) -> Vec<(i64, i64)> {
        p.elements()
            .iter()
            .filter_map(|e| match e {
                PathEl::MoveTo(q) | PathEl::LineTo(q) | PathEl::CurveTo(_, _, q) => Some(*q),
                PathEl::QuadTo(_, q) => Some(*q),
                PathEl::ClosePath => None,
            })
            .map(|q| ((q.x * 1e6).round() as i64, (q.y * 1e6).round() as i64))
            .collect()
    }

    /// A stroke near one corner of a rectangle leaves the other three
    /// corners, and the sides the nib does not reach, exactly as they were,
    /// and adds only a few nodes.
    #[test]
    fn brushes_change_only_the_outline_under_the_nib() {
        let sq = Rect::new(0.0, 0.0, 100.0, 60.0).to_path(0.01);
        let at = Point::new(100.0, 60.0);
        let far = [(0, 0), (100_000_000, 0), (0, 60_000_000)];
        for out in [
            smear(&sq, Affine::IDENTITY, at, Vec2::new(4.0, 3.0), 10.0),
            twirl(&sq, Affine::IDENTITY, at, 0.4, 10.0),
            smooth(&sq, Affine::IDENTITY, at, 10.0, 1.0),
            roughen(&sq, Affine::IDENTITY, at, 10.0, 1.0, 1.0),
            smudge(
                &sq,
                Affine::IDENTITY,
                at,
                Vec2::new(4.0, 3.0),
                10.0,
                0.0,
                1.0,
            ),
        ] {
            assert_ne!(out, sq);
            let n = nodes(&out);
            for c in far {
                assert!(n.contains(&c), "corner {c:?} kept: {n:?}");
            }
            assert!(n.len() < 40, "few nodes: {}", n.len());
            // Nothing moved outside the nib.
            let pts = resample(&out, 400);
            for p in pts {
                let on_old = p.x.abs() < 1e-6
                    || p.y.abs() < 1e-6
                    || (p.x - 100.0).abs() < 1e-6
                    || (p.y - 60.0).abs() < 1e-6;
                assert!(on_old || (p - at).hypot() < 10.0 + 6.0, "{p:?}");
            }
        }
        // The nib inside a big shape, away from its outline: no change.
        let big = Rect::new(0.0, 0.0, 200.0, 200.0).to_path(0.01);
        let mid = Point::new(100.0, 100.0);
        assert_eq!(
            smear(&big, Affine::IDENTITY, mid, Vec2::new(5.0, 0.0), 10.0),
            big
        );
        assert_eq!(twirl(&big, Affine::IDENTITY, mid, 0.5, 10.0), big);
        assert_eq!(smooth(&big, Affine::IDENTITY, mid, 10.0, 1.0), big);
        assert_eq!(roughen(&big, Affine::IDENTITY, mid, 10.0, 1.0, 1.0), big);
        assert_eq!(attract(&big, Affine::IDENTITY, mid, 10.0, 1.0), big);
    }

    /// The nib is measured on the page: on an object stretched three times
    /// wide, a smear only moves what lies inside the circle on the page.
    #[test]
    fn the_nib_is_the_circle_on_the_page() {
        let sq = Rect::new(0.0, 0.0, 20.0, 20.0).to_path(0.01);
        let stretch = Affine::scale_non_uniform(3.0, 1.0);
        // The top side on the page runs from x 0 to 60 at y 20.
        let at = Point::new(30.0, 20.0);
        let out = smear(&sq, stretch, at, Vec2::new(0.0, 4.0), 5.0);
        let page = stretch * out;
        for p in resample(&page, 600) {
            if p.y > 20.0 + 1e-6 {
                assert!((p.x - 30.0).abs() < 5.0 + 1e-6, "{p:?}");
            }
        }
        assert!(page.bounding_box().y1 > 23.0);
    }

    /// Smudge out of a shape adds area; into it takes area away. The nib
    /// narrows along the stroke.
    #[test]
    fn smudge_pulls_out_and_pushes_in() {
        fn area(p: &BezPath) -> f64 {
            let pts = resample(p, 800);
            let n = pts.len();
            (0..n)
                .map(|i| {
                    let (a, b) = (pts[i], pts[(i + 1) % n]);
                    a.x * b.y - b.x * a.y
                })
                .sum::<f64>()
                .abs()
                / 2.0
        }
        let sq = Rect::new(0.0, 0.0, 40.0, 40.0).to_path(0.01);
        let a0 = area(&sq);
        let mut out = sq.clone();
        let mut at = Point::new(38.0, 20.0);
        let mut travelled = 0.0;
        for _ in 0..6 {
            let d = Vec2::new(2.0, 0.0);
            out = smudge(&out, Affine::IDENTITY, at, d, 6.0, travelled, 1.0);
            at += d;
            travelled += 2.0;
        }
        assert!(area(&out) > a0 + 5.0, "{} vs {a0}", area(&out));
        let mut inn = sq.clone();
        let mut at = Point::new(42.0, 20.0);
        let mut travelled = 0.0;
        for _ in 0..6 {
            let d = Vec2::new(-2.0, 0.0);
            inn = smudge(&inn, Affine::IDENTITY, at, d, 6.0, travelled, 1.0);
            at += d;
            travelled += 2.0;
        }
        assert!(area(&inn) < a0 - 5.0, "{} vs {a0}", area(&inn));
        assert!(smudge_radius(6.0, 30.0, 1.0) < smudge_radius(6.0, 0.0, 1.0));
        assert!(smudge_radius(6.0, 30.0, 0.2) > smudge_radius(6.0, 30.0, 1.0));
    }

    #[test]
    fn blends_of_several_parts_morph_part_by_part() {
        // Two "letters" side by side into two "letters" further right,
        // and a third part that appears from a point.
        let two = |x: f64| {
            let mut p = Rect::new(x, 0.0, x + 10.0, 10.0).to_path(0.01);
            p.extend(Rect::new(x + 20.0, 0.0, x + 30.0, 10.0).to_path(0.01));
            p
        };
        let a = two(0.0);
        let mut b = two(100.0);
        b.extend(Rect::new(140.0, 0.0, 150.0, 10.0).to_path(0.01));
        let steps = blend(&a, &b, 1, 64);
        assert_eq!(steps.len(), 1);
        let parts = subpaths(&steps[0]);
        assert_eq!(parts.len(), 3, "one part per pair");
        let first = parts[0].0.bounding_box();
        let second = parts[1].0.bounding_box();
        // Each part keeps its size halfway and sits halfway.
        assert!(
            (first.width() - 10.0).abs() < 1.0 && (first.center().x - 55.0).abs() < 1.0,
            "{first:?}"
        );
        assert!(
            (second.width() - 10.0).abs() < 1.0 && (second.center().x - 75.0).abs() < 1.0,
            "{second:?}"
        );
        // The new part grows from its own centre: half its size there.
        let third = parts[2].0.bounding_box();
        assert!(
            (third.center().x - 145.0).abs() < 1.0 && third.width() < 6.0,
            "{third:?}"
        );
    }

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
        let b =
            crate::geometry::Ellipse::from_rect(Rect::new(100.0, 0.0, 130.0, 20.0)).to_path(0.01);
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
        let t = distort(
            &a,
            Distort::Twister {
                angle_deg: 45.0,
                center: None,
            },
        );
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
