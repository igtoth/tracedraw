//! Fit flattened outlines back to few Bezier segments. Polygon operations
//! (stroking an outline, the band of a variable outline) return many short
//! lines; this turns them into lines where they run straight and cubic
//! curves where they bend, with corners kept as corners and as few nodes
//! as stay within a tolerance. Each smooth run is fitted by least squares
//! with chord-length parameters, refined by Newton steps and split in two
//! where one curve is not enough (the method of Schneider's "An Algorithm
//! for Automatically Fitting Digitized Curves", Graphics Gems, 1990).
//! It runs when a command runs, never per frame.

use crate::geometry::{BezPath, CubicBez, ParamCurve, PathEl, Point, Vec2};

/// A turn at a vertex above this (radians) is always a corner.
const CORNER: f64 = 0.6;
/// A smaller turn, above `MIN_SPIKE`, is a corner when it is `SPIKE` times
/// the turn at both neighbouring vertices (a bevel on a smooth side).
const SPIKE: f64 = 3.0;
const MIN_SPIKE: f64 = 0.12;
/// Deepest split of one run before it is kept as lines.
const MAX_DEPTH: usize = 24;

/// `path` as lines and cubic curves within `tolerance` (mm) of its
/// vertices. Curves in the input are flattened first.
pub fn fit_path(path: &BezPath, tolerance: f64) -> BezPath {
    fit_path_with_nodes(path, tolerance, &[])
}

/// A node the fitted path must have: where, the direction the path runs
/// there (either way round), and how far from it a vertex may lie to
/// take it (the tolerance when less).
pub type Node = (Point, Vec2, f64);

/// [`fit_path`], with a smooth node at the vertex nearest each of
/// `nodes`, its tangent along the node's direction: the places where a
/// stroke's round end meets its sides, which a curve would otherwise run
/// across. A node's reach lets it take a vertex further away than the
/// tolerance, when the polygon has no vertex right there (the tip of a
/// wide round end lies between two vertices of its fan).
pub fn fit_path_with_nodes(path: &BezPath, tolerance: f64, nodes: &[Node]) -> BezPath {
    let tol = if tolerance.is_finite() && tolerance > 0.0 {
        tolerance
    } else {
        0.01
    };
    let mut list: Vec<Node> = nodes
        .iter()
        .filter(|(p, t, _)| p.is_finite() && unit(*t).is_some())
        .map(|&(p, t, r)| (p, t, if r.is_finite() { r.max(tol) } else { tol }))
        .collect();
    list.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
    let reach = list.iter().map(|n| n.2).fold(tol, f64::max);
    let nodes = Nodes { list, reach };
    let mut out = BezPath::new();
    for (pts, closed) in subpaths(path, tol) {
        fit_subpath(&pts, closed, tol, &nodes, &mut out);
    }
    out
}

/// Given nodes sorted by x, and the largest reach among them.
struct Nodes {
    list: Vec<Node>,
    reach: f64,
}

impl Nodes {
    /// The nearest node within its reach of `p`: where it is and its
    /// direction.
    fn at(&self, p: Point) -> Option<(Point, Vec2)> {
        let from = self.list.partition_point(|n| n.0.x < p.x - self.reach);
        self.list[from..]
            .iter()
            .take_while(|n| n.0.x <= p.x + self.reach)
            .map(|n| ((n.0 - p).hypot(), n))
            .filter(|(d, n)| *d <= n.2)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .and_then(|(_, n)| Some((n.0, unit(n.1)?)))
    }
}

/// The subpaths of `path` as vertex lists without repeated points, with a
/// closed flag (the closing point not repeated).
fn subpaths(path: &BezPath, tol: f64) -> Vec<(Vec<Point>, bool)> {
    let same = tol * 1e-3;
    let mut out: Vec<(Vec<Point>, bool)> = Vec::new();
    let mut cur: Vec<Point> = Vec::new();
    let mut closed = false;
    let finish = |cur: &mut Vec<Point>, closed: bool, out: &mut Vec<(Vec<Point>, bool)>| {
        if closed
            && cur.len() > 1
            && cur
                .first()
                .zip(cur.last())
                .is_some_and(|(a, b)| (*a - *b).hypot() <= same)
        {
            cur.pop();
        }
        if cur.len() >= 2 {
            out.push((std::mem::take(cur), closed));
        }
        cur.clear();
    };
    kurbo::flatten(
        path.elements().iter().copied(),
        tol * 0.25,
        &mut |el| match el {
            PathEl::MoveTo(p) => {
                finish(&mut cur, closed, &mut out);
                closed = false;
                cur.push(p);
            }
            PathEl::LineTo(p) => {
                if cur.last().is_none_or(|q| (p - *q).hypot() > same) {
                    cur.push(p);
                }
            }
            PathEl::ClosePath => closed = true,
            _ => {}
        },
    );
    finish(&mut cur, closed, &mut out);
    out
}

/// Unit vector of `v`, or `None` when it has no length.
fn unit(v: Vec2) -> Option<Vec2> {
    let len = v.hypot();
    (len > 1e-12).then(|| v / len)
}

/// How a run between two breaks starts or ends.
#[derive(Clone, Copy)]
struct Break {
    /// Direction of travel arriving at the vertex and leaving it; `None`
    /// at a corner, where the run's own points give it.
    t_in: Option<Vec2>,
    t_out: Option<Vec2>,
}

/// `pts` with each run of vertices lying within `tol` of the run's first
/// one made one vertex, at a kept vertex of the run or else its middle:
/// the tiny steps polygon operations leave where pieces overlap, which
/// would read as corners. A run never spans more than twice `tol`, so the
/// fine fan of a thin outline's round end keeps its shape, and a second
/// kept vertex starts a run of its own.
fn merge_close(pts: &[Point], closed: bool, tol: f64, keep: &dyn Fn(Point) -> bool) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::with_capacity(pts.len());
    let mut group: Vec<Point> = Vec::new();
    let mut group_kept = false;
    let flush = |group: &mut Vec<Point>, out: &mut Vec<Point>| {
        let kept = group.iter().copied().find(|p| keep(*p));
        let mid = || {
            let sum = group.iter().fold(Vec2::ZERO, |s, p| s + p.to_vec2());
            (sum / group.len() as f64).to_point()
        };
        if !group.is_empty() {
            out.push(kept.unwrap_or_else(mid));
        }
        group.clear();
    };
    for (i, p) in pts.iter().enumerate() {
        // The ends of an open subpath stay where they are.
        let end = !closed && (i == 0 || i + 1 == pts.len());
        let kept = keep(*p);
        let near = group.first().is_some_and(|q| (*p - *q).hypot() < tol);
        if !near || end || (kept && group_kept) {
            flush(&mut group, &mut out);
            group_kept = false;
        }
        group.push(*p);
        group_kept |= kept;
        if end {
            flush(&mut group, &mut out);
            group_kept = false;
        }
    }
    flush(&mut group, &mut out);
    if closed && out.len() > 3 {
        if let (Some(a), Some(b)) = (out.first().copied(), out.last().copied()) {
            if (a - b).hypot() < tol && !keep(b) {
                out.pop();
            }
        }
    }
    if out.len() < if closed { 3 } else { 2 } {
        return pts.to_vec();
    }
    out
}

/// `pts` without the vertices that lie within `tol` of the lines between
/// their neighbours (the rounding of polygon operations along straight
/// runs), keeping those `keep` names. A closed loop is opened at a kept
/// vertex, or else at its sharpest turn.
fn thin(pts: &[Point], closed: bool, tol: f64, keep: &dyn Fn(Point) -> bool) -> Vec<Point> {
    let n = pts.len();
    if n < 4 {
        return pts.to_vec();
    }
    let kept: Vec<bool> = pts.iter().map(|p| keep(*p)).collect();
    let start = if closed {
        let turn = |i: usize| {
            let (a, b, c) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
            let (u, v) = (b - a, c - b);
            u.cross(v).atan2(u.dot(v)).abs()
        };
        kept.iter().position(|k| *k).unwrap_or_else(|| {
            (0..n)
                .max_by(|a, b| turn(*a).total_cmp(&turn(*b)))
                .unwrap_or(0)
        })
    } else {
        0
    };
    let count = if closed { n + 1 } else { n };
    let ring: Vec<Point> = (0..count).map(|i| pts[(start + i) % n]).collect();
    // Thin each stretch between kept vertices on its own.
    let mut out: Vec<Point> = Vec::with_capacity(ring.len());
    let mut from = 0;
    for i in 1..ring.len() {
        let last = i + 1 == ring.len();
        if last || kept[(start + i) % n] {
            let part = crate::geometry::simplify(&ring[from..=i], tol);
            out.extend(part.iter().skip(usize::from(!out.is_empty())));
            from = i;
        }
    }
    if closed {
        out.pop();
    }
    if out.len() < 3 {
        return pts.to_vec();
    }
    out
}

/// Fit one subpath; the vertex nearest each of `nodes`, within its
/// reach, becomes a smooth node.
fn fit_subpath(pts: &[Point], closed: bool, tol: f64, nodes: &Nodes, out: &mut BezPath) {
    let keep = |p: Point| nodes.at(p).is_some();
    let merged = merge_close(pts, closed, tol, &keep);
    let thinned = thin(&merged, closed, tol * 0.25, &keep);
    let pts = &thinned[..];
    let n = pts.len();
    let Some(&first) = pts.first() else {
        return;
    };
    if n < 3 {
        out.move_to(first);
        for p in &pts[1..] {
            out.line_to(*p);
        }
        if closed {
            out.close_path();
        }
        return;
    }
    let at = |i: usize| pts[i % n];
    let segs = if closed { n } else { n - 1 };
    let seg_dir = |s: usize| unit(at(s + 1) - at(s)).unwrap_or(Vec2::new(1.0, 0.0));
    // Segment before and after vertex `i`, when there is one.
    let before = |i: usize| -> Option<usize> {
        if i > 0 {
            Some(i - 1)
        } else if closed {
            Some(n - 1)
        } else {
            None
        }
    };
    let after = |i: usize| -> Option<usize> { (i < segs).then_some(i) };
    let turn: Vec<f64> = (0..n)
        .map(|i| match (before(i), after(i)) {
            (Some(a), Some(b)) => {
                let (u, v) = (seg_dir(a), seg_dir(b));
                u.cross(v).atan2(u.dot(v)).abs()
            }
            _ => 0.0,
        })
        .collect();
    let corner: Vec<bool> = (0..n)
        .map(|i| {
            let (Some(a), Some(b)) = (before(i), after(i)) else {
                // The ends of an open subpath.
                return true;
            };
            let t = turn[i];
            let prev = turn[a % n];
            let next = turn[(b + 1) % n];
            t > CORNER || (t > MIN_SPIKE && t > SPIKE * prev.max(next))
        })
        .collect();
    let free = Break {
        t_in: None,
        t_out: None,
    };
    let mut breaks: Vec<(usize, Break)> = (0..n)
        .filter_map(|i| {
            if corner[i] {
                return Some((i, free));
            }
            // A given node, at the vertex nearest to it: its direction,
            // the way the path runs.
            let (q, t) = nodes.at(at(i))?;
            let d = (at(i) - q).hypot();
            let nearer = |j: Option<usize>| j.is_some_and(|j| (at(j) - q).hypot() < d);
            let prev = (i > 0 || closed).then(|| (i + n - 1) % n);
            let next = (i + 1 < n || closed).then(|| (i + 1) % n);
            if nearer(prev) || nearer(next) {
                return None;
            }
            let along = tangent_at(pts, i, closed, tol * REACH);
            let t = Some(if t.dot(along) < 0.0 { -t } else { t });
            Some((i, Break { t_in: t, t_out: t }))
        })
        .collect();
    if breaks.is_empty() {
        // A smooth closed loop: split at its leftmost and rightmost
        // points, where an ellipse has its nodes.
        let key = |p: &Point| (p.x, p.y);
        let lo = (0..n)
            .min_by(|a, b| {
                key(&at(*a))
                    .partial_cmp(&key(&at(*b)))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(0);
        let hi = (0..n)
            .max_by(|a, b| {
                key(&at(*a))
                    .partial_cmp(&key(&at(*b)))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(n / 2);
        let (lo, hi) = if lo == hi {
            (0, n / 2)
        } else {
            (lo.min(hi), lo.max(hi))
        };
        for i in [lo, hi] {
            let t = Some(tangent_at(pts, i, closed, tol * REACH));
            breaks.push((i, Break { t_in: t, t_out: t }));
        }
    }
    let Some(&(b0, _)) = breaks.first() else {
        return;
    };
    out.move_to(at(b0));
    let runs = if closed {
        breaks.len()
    } else {
        breaks.len() - 1
    };
    for k in 0..runs {
        let (i, start) = breaks[k];
        let (j, end) = breaks[(k + 1) % breaks.len()];
        let j = if j <= i { j + n } else { j };
        let run: Vec<Point> = (i..=j).map(at).collect();
        fit_run(&run, start.t_out, end.t_in, tol, out);
    }
    if closed {
        out.close_path();
    }
}

/// How far (in tolerances) the points that give a tangent lie from the
/// vertex, so that short pieces (the corners a stroke leaves at the joins
/// of a flattened curve) do not tilt it.
const REACH: f64 = 10.0;
/// Most vertices walked to find them.
const REACH_STEPS: usize = 32;

/// Derivative at `b` of the parabola through `a`, `b` and `c` at their
/// chord lengths, as a unit vector.
fn parabola_tangent(a: Point, b: Point, c: Point) -> Option<Vec2> {
    let (d1, d2) = ((b - a).hypot(), (c - b).hypot());
    if d1 < 1e-12 || d2 < 1e-12 {
        return unit(c - a);
    }
    let v = (b - a) * (d2 / (d1 * (d1 + d2))) + (c - b) * (d1 / (d2 * (d1 + d2)));
    unit(v)
}

/// Tangent at vertex `i` (in the direction of travel) from the parabola
/// through it and a vertex on either side at least `reach` away.
fn tangent_at(pts: &[Point], i: usize, closed: bool, reach: f64) -> Vec2 {
    let n = pts.len();
    let p = pts[i % n];
    let mut prev = None;
    let mut next = None;
    // Round a loop, no further than halfway either way, so the walk never
    // comes back past the vertex itself.
    let steps = if closed {
        n.saturating_sub(1) / 2
    } else {
        n.saturating_sub(1)
    };
    for k in 1..=REACH_STEPS.min(steps) {
        if prev.is_none_or(|q: Point| (q - p).hypot() < reach) {
            if i >= k {
                prev = Some(pts[i - k]);
            } else if closed {
                prev = Some(pts[(i + n - k) % n]);
            }
        }
        if next.is_none_or(|q: Point| (q - p).hypot() < reach) {
            if i + k < n {
                next = Some(pts[i + k]);
            } else if closed {
                next = Some(pts[(i + k) % n]);
            }
        }
    }
    let fallback = Vec2::new(1.0, 0.0);
    match (prev, next) {
        (Some(a), Some(c)) => parabola_tangent(a, p, c).unwrap_or(fallback),
        (None, Some(c)) => unit(c - p).unwrap_or(fallback),
        (Some(a), None) => unit(p - a).unwrap_or(fallback),
        (None, None) => fallback,
    }
}

/// Tangent leaving `pts[0]`, from the parabola through it and two later
/// points, about `reach` and twice that away (the chord alone when there
/// are two points).
fn end_tangent(pts: &[Point], reach: f64) -> Vec2 {
    let fallback = || {
        pts.get(1)
            .zip(pts.first())
            .and_then(|(b, a)| unit(*b - *a))
            .unwrap_or(Vec2::new(1.0, 0.0))
    };
    let Some(&p0) = pts.first() else {
        return fallback();
    };
    let last = pts.len().saturating_sub(1).min(2 * REACH_STEPS);
    let far = |d: f64, from: usize| {
        (from..=last)
            .find(|j| (pts[*j] - p0).hypot() >= d)
            .unwrap_or(last)
    };
    let j1 = far(reach, 1).min(last.saturating_sub(1)).max(1);
    let j2 = far(reach * 2.0, j1 + 1).max(j1 + 1);
    let (Some(&p1), Some(&p2)) = (pts.get(j1), pts.get(j2)) else {
        return fallback();
    };
    let (d1, d2) = ((p1 - p0).hypot(), (p2 - p1).hypot());
    if d1 < 1e-12 || d2 < 1e-12 {
        return fallback();
    }
    let d = d1 + d2;
    let v = (p1 - p0) * (d / (d1 * d2)) - (p2 - p0) * (d1 / (d2 * d));
    // A parabola that turns back is no guide.
    unit(v)
        .filter(|t| t.dot(p1 - p0) > 0.0)
        .unwrap_or_else(fallback)
}

/// One fitted piece of a run: the points `from..=to` it covers, the
/// directions it leaves and arrives in, and what it is.
struct Piece {
    from: usize,
    to: usize,
    t1: Vec2,
    t2: Vec2,
    seg: Seg,
}

enum Seg {
    Line,
    Curve(CubicBez),
    /// The points as they are, where no curve fitted.
    Lines,
}

/// Fit the points of one smooth run (at least two), leaving `pts[0]`
/// along `t1` and arriving at its last point along `t2` (from the run's
/// own points when `None`), appending to `out` (which already stands at
/// `pts[0]`). The run is split in the middle until each piece fits, then
/// neighbouring pieces that fit as one are joined again.
fn fit_run(pts: &[Point], t1: Option<Vec2>, t2: Option<Vec2>, tol: f64, out: &mut BezPath) {
    if pts.len() < 2 {
        return;
    }
    let reach = tol * REACH;
    let t1 = t1.unwrap_or_else(|| end_tangent(pts, reach));
    let t2 = t2.unwrap_or_else(|| {
        let back: Vec<Point> = pts
            .iter()
            .rev()
            .take(2 * REACH_STEPS + 1)
            .copied()
            .collect();
        -end_tangent(&back, reach)
    });
    let mut pieces = Vec::new();
    split_run(pts, 0, pts.len() - 1, t1, t2, tol, 0, &mut pieces);
    // Join neighbours while one curve does for both.
    let mut i = 0;
    while i + 1 < pieces.len() {
        let (a, b) = (&pieces[i], &pieces[i + 1]);
        let joinable = !matches!(a.seg, Seg::Lines) && !matches!(b.seg, Seg::Lines);
        let joined = if joinable {
            try_fit(&pts[a.from..=b.to], a.t1, b.t2, tol)
        } else {
            None
        };
        if let Some(seg) = joined {
            let (from, to, t1, t2) = (a.from, b.to, a.t1, b.t2);
            pieces[i] = Piece {
                from,
                to,
                t1,
                t2,
                seg,
            };
            pieces.remove(i + 1);
            // The joined piece may now join the one before.
            i = i.saturating_sub(1);
        } else {
            i += 1;
        }
    }
    for p in pieces {
        match p.seg {
            Seg::Line => out.line_to(pts[p.to]),
            Seg::Curve(bez) => emit(bez, tol, out),
            Seg::Lines => {
                for q in &pts[p.from + 1..=p.to] {
                    out.line_to(*q);
                }
            }
        }
    }
}

/// Fit `pts[from..=to]` as one piece, or split it in the middle (so the
/// nodes fall evenly: the middle of a round end, the quarters of an
/// ellipse) and fit the halves; corners, where features need nodes of
/// their own, are breaks already.
#[allow(clippy::too_many_arguments)]
fn split_run(
    pts: &[Point],
    from: usize,
    to: usize,
    t1: Vec2,
    t2: Vec2,
    tol: f64,
    depth: usize,
    pieces: &mut Vec<Piece>,
) {
    let part = &pts[from..=to];
    let piece = |seg| Piece {
        from,
        to,
        t1,
        t2,
        seg,
    };
    if let Some(seg) = try_fit(part, t1, t2, tol) {
        pieces.push(piece(seg));
        return;
    }
    let n = part.len();
    if depth >= MAX_DEPTH || n < 3 {
        pieces.push(piece(Seg::Lines));
        return;
    }
    let u = chord_params(part);
    let split = u
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - 0.5).abs().total_cmp(&(b.1 - 0.5).abs()))
        .map(|(i, _)| i)
        .unwrap_or(n / 2)
        .clamp(1, n - 2);
    // The tangent from points well apart, so the small wiggles of a
    // polygon (the joins of a stroke along short lines) do not tilt it.
    let span = part
        .first()
        .zip(part.last())
        .map(|(a, b)| (*b - *a).hypot())
        .unwrap_or(0.0);
    let tc = tangent_at(part, split, false, (tol * REACH).max(span / 16.0));
    let mid = from + split;
    split_run(pts, from, mid, t1, tc, tol, depth + 1, pieces);
    split_run(pts, mid, to, tc, t2, tol, depth + 1, pieces);
}

/// `pts` as one line or one curve within `tol`, if they fit.
fn try_fit(pts: &[Point], t1: Vec2, t2: Vec2, tol: f64) -> Option<Seg> {
    let (Some(&a), Some(&b)) = (pts.first(), pts.last()) else {
        return None;
    };
    let flat = pts.len() == 2 || straight(pts, tol);
    // A line where the run leaves and arrives along it; a slightly bent
    // stretch of a curve stays a curve, so the curve stays smooth.
    let along = |t: Vec2| unit(b - a).is_some_and(|c| c.cross(t).abs() < 0.02 && c.dot(t) > 0.0);
    if flat && (pts.len() == 2 || (along(t1) && along(t2))) {
        return Some(Seg::Line);
    }
    let mut u = chord_params(pts);
    let mut bez = generate(pts, &u, t1, t2);
    let mut err = max_error(pts, &bez, &u);
    // Chord lengths overstate the error; move the parameters to the
    // nearest points of the curve before giving up on one curve.
    if err > tol && err < tol * 50.0 {
        for _ in 0..8 {
            u = reparameterize(pts, &bez, &u);
            bez = generate(pts, &u, t1, t2);
            err = max_error(pts, &bez, &u);
            if err <= tol {
                break;
            }
        }
    }
    if err <= tol {
        Some(Seg::Curve(bez))
    } else {
        flat.then_some(Seg::Line)
    }
}

/// Every point within `tol` of the chord from the first to the last.
fn straight(pts: &[Point], tol: f64) -> bool {
    let (Some(&a), Some(&b)) = (pts.first(), pts.last()) else {
        return true;
    };
    pts.iter().all(|p| dist_to_segment(*p, a, b) <= tol)
}

fn dist_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let len2 = ab.hypot2();
    if len2 < 1e-24 {
        return (p - a).hypot();
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p - (a + ab * t)).hypot()
}

/// Chord-length parameters of the points, 0 to 1.
fn chord_params(pts: &[Point]) -> Vec<f64> {
    let mut u = Vec::with_capacity(pts.len());
    let mut run = 0.0;
    u.push(0.0);
    for w in pts.windows(2) {
        run += (w[1] - w[0]).hypot();
        u.push(run);
    }
    if run > 1e-12 {
        for v in &mut u {
            *v /= run;
        }
    }
    u
}

/// The least-squares cubic from `pts[0]` along `t1` to the last point
/// along `t2`, the points at parameters `u`.
fn generate(pts: &[Point], u: &[f64], t1: Vec2, t2: Vec2) -> CubicBez {
    let p0 = pts[0];
    let p3 = pts[pts.len() - 1];
    let (mut c00, mut c01, mut c11, mut x0, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (p, &t) in pts.iter().zip(u) {
        let s = 1.0 - t;
        let (b0, b1, b2, b3) = (s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t);
        let a1 = t1 * b1;
        let a2 = -t2 * b2;
        c00 += a1.dot(a1);
        c01 += a1.dot(a2);
        c11 += a2.dot(a2);
        let base = p0.to_vec2() * (b0 + b1) + p3.to_vec2() * (b2 + b3);
        let rest = p.to_vec2() - base;
        x0 += a1.dot(rest);
        x1 += a2.dot(rest);
    }
    let chord = (p3 - p0).hypot();
    let det = c00 * c11 - c01 * c01;
    let (mut a1, mut a2) = if det.abs() > 1e-12 {
        ((x0 * c11 - x1 * c01) / det, (c00 * x1 - c01 * x0) / det)
    } else {
        (0.0, 0.0)
    };
    let eps = chord * 1e-6;
    if !(a1 > eps && a2 > eps && a1.is_finite() && a2.is_finite()) {
        a1 = chord / 3.0;
        a2 = chord / 3.0;
    }
    // Handles longer than the chord loop over.
    a1 = a1.min(chord);
    a2 = a2.min(chord);
    CubicBez::new(p0, p0 + t1 * a1, p3 - t2 * a2, p3)
}

/// Largest distance of a point from the curve at its parameter.
fn max_error(pts: &[Point], bez: &CubicBez, u: &[f64]) -> f64 {
    pts.iter()
        .zip(u)
        .map(|(p, &t)| (bez.eval(t) - *p).hypot())
        .fold(0.0, f64::max)
}

/// One Newton step towards the parameter of each point's nearest place
/// on the curve.
fn reparameterize(pts: &[Point], bez: &CubicBez, u: &[f64]) -> Vec<f64> {
    let d1 = [
        (bez.p1 - bez.p0) * 3.0,
        (bez.p2 - bez.p1) * 3.0,
        (bez.p3 - bez.p2) * 3.0,
    ];
    let d2 = [(d1[1] - d1[0]) * 2.0, (d1[2] - d1[1]) * 2.0];
    pts.iter()
        .zip(u)
        .map(|(p, &t)| {
            let s = 1.0 - t;
            let q = bez.eval(t) - *p;
            let q1 = d1[0] * (s * s) + d1[1] * (2.0 * s * t) + d1[2] * (t * t);
            let q2 = d2[0] * s + d2[1] * t;
            let den = q1.dot(q1) + q.dot(q2);
            if den.abs() < 1e-12 {
                t
            } else {
                (t - q.dot(q1) / den).clamp(0.0, 1.0)
            }
        })
        .collect()
}

/// Append `bez`, as a line when its handles lie on its chord.
fn emit(bez: CubicBez, tol: f64, out: &mut BezPath) {
    let flat = dist_to_segment(bez.p1, bez.p0, bez.p3) <= tol * 0.25
        && dist_to_segment(bez.p2, bez.p0, bez.p3) <= tol * 0.25;
    if flat {
        out.line_to(bez.p3);
    } else {
        out.curve_to(bez.p1, bez.p2, bez.p3);
    }
}

#[cfg(test)]
mod tests {
    use super::fit_path;
    use crate::geometry::{BezPath, Circle, PathEl, Point, Rect, Shape as _};

    /// The polygon of `path` flattened at `tol`.
    fn polygon(path: &BezPath, tol: f64) -> BezPath {
        let mut out = BezPath::new();
        crate::geometry::flatten(path, tol, &mut |el| out.push(el));
        out
    }

    fn count(path: &BezPath) -> (usize, usize) {
        let lines = path
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::LineTo(_)))
            .count();
        let curves = path
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::CurveTo(..)))
            .count();
        (lines, curves)
    }

    /// Largest distance of the fitted path's flattened points from the
    /// circle.
    fn off_circle(path: &BezPath, c: Point, r: f64) -> f64 {
        let mut worst: f64 = 0.0;
        crate::geometry::flatten(path, 0.001, &mut |el| {
            if let PathEl::LineTo(p) | PathEl::MoveTo(p) = el {
                worst = worst.max(((p - c).hypot() - r).abs());
            }
        });
        worst
    }

    #[test]
    fn a_flattened_circle_becomes_four_curves() {
        let c = Point::new(10.0, 20.0);
        let poly = polygon(&Circle::new(c, 15.0).to_path(1e-6), 0.01);
        assert!(count(&poly).0 > 50);
        let fit = fit_path(&poly, 0.01);
        assert_eq!(count(&fit), (0, 4), "{:?}", fit.elements());
        assert!(off_circle(&fit, c, 15.0) < 0.02);
    }

    #[test]
    fn a_rectangle_keeps_four_corners_and_lines() {
        let poly = Rect::new(0.0, 0.0, 30.0, 10.0).to_path(0.01);
        let fit = fit_path(&poly, 0.01);
        assert_eq!(count(&fit), (4, 0), "{:?}", fit.elements());
    }

    #[test]
    fn collinear_points_become_one_line() {
        let mut p = BezPath::new();
        p.move_to(Point::new(0.0, 0.0));
        for i in 1..=100 {
            p.line_to(Point::new(i as f64, 0.0));
        }
        let fit = fit_path(&p, 0.01);
        assert_eq!(count(&fit), (1, 0));
    }

    #[test]
    fn given_nodes_split_a_stadium_where_its_ends_meet_its_sides() {
        use crate::geometry::Vec2;
        // A 2 mm wide line from (0,0) to (40,0) with round ends, the ends
        // fanned in many short lines.
        let mut p = BezPath::new();
        p.move_to(Point::new(0.0, -1.0));
        p.line_to(Point::new(40.0, -1.0));
        let steps = 60;
        for i in 1..steps {
            let a = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * i as f64 / steps as f64;
            p.line_to(Point::new(40.0 + a.cos(), a.sin()));
        }
        p.line_to(Point::new(40.0, 1.0));
        p.line_to(Point::new(0.0, 1.0));
        for i in 1..steps {
            let a = std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * i as f64 / steps as f64;
            p.line_to(Point::new(a.cos(), a.sin()));
        }
        p.close_path();
        let x = Vec2::new(1.0, 0.0);
        let nodes = [
            (Point::new(0.0, -1.0), x, 0.0),
            (Point::new(0.0, 1.0), x, 0.0),
            (Point::new(40.0, -1.0), x, 0.0),
            (Point::new(40.0, 1.0), x, 0.0),
        ];
        let fit = super::fit_path_with_nodes(&p, 0.01, &nodes);
        // Two straight sides, each end two curves meeting at its tip.
        assert_eq!(count(&fit), (2, 4), "{:?}", fit.elements());
        let ends: Vec<Point> = fit
            .elements()
            .iter()
            .filter_map(|e| e.end_point())
            .collect();
        for q in [
            Point::new(41.0, 0.0),
            Point::new(-1.0, 0.0),
            Point::new(0.0, 1.0),
            Point::new(40.0, -1.0),
        ] {
            assert!(ends.iter().any(|p| (*p - q).hypot() < 0.02), "{q:?}");
        }
    }

    #[test]
    fn a_node_between_two_vertices_takes_the_nearest() {
        use crate::geometry::Vec2;
        // An open 240 degree arc of radius 10, a vertex every 3 degrees
        // from 1.5: none at 90 degrees, where the node is given.
        let at = |deg: f64| {
            let a = deg.to_radians();
            Point::new(10.0 * a.cos(), 10.0 * a.sin())
        };
        let mut p = BezPath::new();
        p.move_to(at(0.0));
        for k in 0..80 {
            p.line_to(at(1.5 + 3.0 * k as f64));
        }
        p.line_to(at(240.0));
        let node = |reach| [(at(90.0), Vec2::new(-1.0, 0.0), reach)];
        let near_90 = |fit: &BezPath| {
            fit.elements()
                .iter()
                .filter_map(|e| e.end_point())
                .any(|q| (q - at(90.0)).hypot() < 0.3)
        };
        // Within the tolerance of no vertex: no node there.
        assert!(!near_90(&super::fit_path_with_nodes(&p, 0.01, &node(0.0))));
        // Reaching half a step: the nearest vertex is the node.
        let fit = super::fit_path_with_nodes(&p, 0.01, &node(0.3));
        assert!(near_90(&fit), "{:?}", fit.elements());
        // Two nodes within reach of the same vertex: the nearer one takes
        // it, with its own direction.
        let nodes = [
            (at(89.0), Vec2::new(-1.0, 0.0), 0.3),
            (at(87.0), Vec2::new(0.0, 1.0), 0.3),
        ];
        let near = super::Nodes {
            list: nodes.to_vec(),
            reach: 0.3,
        };
        let (q, _) = near.at(at(88.4)).expect("a node");
        assert!((q - at(89.0)).hypot() < 1e-9);
    }

    #[test]
    fn open_runs_keep_their_ends() {
        let mut p = BezPath::new();
        p.move_to(Point::new(0.0, 0.0));
        for i in 1..=90 {
            let a = (i as f64).to_radians();
            p.line_to(Point::new(10.0 * a.sin(), 10.0 - 10.0 * a.cos()));
        }
        let fit = fit_path(&p, 0.01);
        assert_eq!(count(&fit).1, 1, "{:?}", fit.elements());
        let ends: Vec<Point> = fit
            .elements()
            .iter()
            .filter_map(|e| e.end_point())
            .collect();
        assert_eq!(ends.first(), Some(&Point::new(0.0, 0.0)));
        assert!((ends[ends.len() - 1] - Point::new(10.0, 10.0)).hypot() < 1e-9);
    }

    #[test]
    fn nothing_breaks_on_degenerate_input() {
        let mut p = BezPath::new();
        p.move_to(Point::new(1.0, 1.0));
        p.line_to(Point::new(1.0, 1.0));
        p.close_path();
        p.move_to(Point::new(f64::NAN, 0.0));
        p.line_to(Point::new(2.0, 0.0));
        p.line_to(Point::new(3.0, f64::INFINITY));
        let _ = fit_path(&p, 0.01);
        let _ = fit_path(&BezPath::new(), 0.0);
    }
}
