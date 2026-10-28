//! Static effects on paths: blend (interpolate between two shapes), extrude
//! (parallel), distort (push/pull, zipper, twister) and brush-style
//! deformations (smear, twirl, smooth). All work on flattened polylines
//! and return new paths; the results are baked, not kept live.

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

/// [`blend`], but with one-part outlines paired from given nodes:
/// `nodes` holds, in page space, the first node of `a` and a point after
/// it along `a`, then the same for `b`. Each outline is run from the
/// node nearest its first node, the way that reaches the second point
/// sooner, and the two are paired node for node without matching them
/// by shape: when the nodes sit at different places on the two outlines
/// the steps turn, and when the outlines run opposite ways they fold
/// through the middle, as blends read from `.cdr` files draw (#597).
/// Each step's nodes lie on the straight lines between paired nodes, so
/// a step between a pentagon and a rectangle is a pentagon with sharp
/// corners (see [`paired_nodes`] for outlines with different node
/// counts). Open or very large outlines are paired point by point along
/// their length instead. Outlines of several parts, or `None`, blend as
/// [`blend`] does.
pub fn blend_from_nodes(
    a: &BezPath,
    b: &BezPath,
    steps: usize,
    samples: usize,
    nodes: Option<[Point; 4]>,
) -> Vec<BezPath> {
    let one_part = |p: &BezPath| {
        p.elements()
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count()
            == 1
    };
    let Some([a0, a1, b0, b1]) = nodes else {
        return blend(a, b, steps, samples);
    };
    if !one_part(a) || !one_part(b) {
        return blend(a, b, steps, samples);
    }
    if let Some((sa, sb)) = paired_nodes(a, b, [a0, a1, b0, b1]) {
        return node_steps(&sa, &sb, steps);
    }
    let (Some(pa), Some(pb)) = (
        run_from(&resample(a, samples), a0, a1),
        run_from(&resample(b, samples), b0, b1),
    ) else {
        return blend(a, b, steps, samples);
    };
    if pa.len() != pb.len() {
        return blend(a, b, steps, samples);
    }
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

/// The `steps` steps between two runs of segments paired one for one:
/// each node and control point of a step lies on the straight line
/// between its two paired points, and a step's segment is a line where
/// both paired segments are lines.
fn node_steps(sa: &[Seg], sb: &[Seg], steps: usize) -> Vec<BezPath> {
    let mut out = open_node_steps(sa, sb, steps);
    out.iter_mut().for_each(|p| p.close_path());
    out
}

/// [`node_steps`] left open: the steps of two open outlines.
fn open_node_steps(sa: &[Seg], sb: &[Seg], steps: usize) -> Vec<BezPath> {
    let (Some(fa), Some(fb)) = (sa.first(), sb.first()) else {
        return Vec::new();
    };
    (1..=steps)
        .map(|k| {
            let t = k as f64 / (steps + 1) as f64;
            let mut out = BezPath::with_capacity(sa.len() + 2);
            out.move_to(fa.p0.lerp(fb.p0, t));
            for (x, y) in sa.iter().zip(sb) {
                let end = x.p3.lerp(y.p3, t);
                if x.line && y.line {
                    out.line_to(end);
                } else {
                    out.curve_to(x.c1.lerp(y.c1, t), x.c2.lerp(y.c2, t), end);
                }
            }
            out
        })
        .collect()
}

/// The chords a curved segment is measured over (see [`Seg::lengths`]).
const CHORDS: usize = 8;

/// One segment of an outline as a cubic (a line has its control points
/// on itself, a third of the way from each end).
#[derive(Debug, Clone, Copy)]
struct Seg {
    p0: Point,
    c1: Point,
    c2: Point,
    p3: Point,
    line: bool,
}

impl Seg {
    fn line(p0: Point, p3: Point) -> Seg {
        Seg {
            p0,
            c1: p0.lerp(p3, 1.0 / 3.0),
            c2: p0.lerp(p3, 2.0 / 3.0),
            p3,
            line: true,
        }
    }

    fn reversed(self) -> Seg {
        Seg {
            p0: self.p3,
            c1: self.c2,
            c2: self.c1,
            p3: self.p0,
            line: self.line,
        }
    }

    /// The lengths along the segment at the parameters 0, 1/8, ...,
    /// 1, measured over chords (close enough to pair nodes by share of
    /// length, and much cheaper than solving for exact lengths).
    fn lengths(self) -> [f64; CHORDS + 1] {
        let mut out = [0.0; CHORDS + 1];
        if self.line {
            let l = (self.p3 - self.p0).hypot();
            for (k, o) in out.iter_mut().enumerate() {
                *o = l * k as f64 / CHORDS as f64;
            }
            return out;
        }
        use kurbo::ParamCurve;
        let c = kurbo::CubicBez::new(self.p0, self.c1, self.c2, self.p3);
        let mut prev = self.p0;
        for k in 1..=CHORDS {
            let q = c.eval(k as f64 / CHORDS as f64);
            out[k] = out[k - 1] + (q - prev).hypot();
            prev = q;
        }
        out
    }

    /// The parameter `len` along a segment whose [`Seg::lengths`] are
    /// `table`.
    fn param_at(table: &[f64; CHORDS + 1], len: f64) -> f64 {
        let k = table.partition_point(|l| *l < len).clamp(1, CHORDS);
        let (l0, l1) = (table[k - 1], table[k]);
        let u = if l1 > l0 { (len - l0) / (l1 - l0) } else { 0.0 };
        let t = (k as f64 - 1.0 + u) / CHORDS as f64;
        if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// The part of the segment between parameters `t0` and `t1`.
    fn part(self, t0: f64, t1: f64) -> Seg {
        if self.line {
            return Seg::line(self.p0.lerp(self.p3, t0), self.p0.lerp(self.p3, t1));
        }
        use kurbo::ParamCurve;
        let c = kurbo::CubicBez::new(self.p0, self.c1, self.c2, self.p3).subsegment(t0..t1);
        Seg {
            p0: c.p0,
            c1: c.p1,
            c2: c.p2,
            p3: c.p3,
            line: false,
        }
    }
}

/// The most nodes an outline may have to be paired node for node; larger
/// ones are paired point by point along their length, so a blend of huge
/// outlines with many steps stays small.
const MAX_PAIRED_NODES: usize = 4096;

/// The segments of a closed one-part outline (the closing line included,
/// segments of no length left out; an outline ending on its first node
/// is closed). `None` for an open outline, one of
/// more than [`MAX_PAIRED_NODES`] nodes or of fewer than two segments.
fn closed_segments(path: &BezPath) -> Option<Vec<Seg>> {
    path_segments(path).and_then(|(segs, closed)| (closed && segs.len() >= 2).then_some(segs))
}

/// The segments of an open one-part line (segments of no length left
/// out). `None` for a closed outline, or one of more than
/// [`MAX_PAIRED_NODES`] nodes or with no segment.
fn open_segments(path: &BezPath) -> Option<Vec<Seg>> {
    path_segments(path).and_then(|(segs, closed)| (!closed && !segs.is_empty()).then_some(segs))
}

/// The segments of a one-part outline and whether it is closed; `None`
/// for several parts, too many nodes or points that are not finite.
fn path_segments(path: &BezPath) -> Option<(Vec<Seg>, bool)> {
    let els = path.elements();
    if els.len() > MAX_PAIRED_NODES + 2 {
        return None;
    }
    let mut segs = Vec::new();
    let (mut start, mut cur, mut closed) = (None, Point::ZERO, false);
    for el in els {
        let seg = match *el {
            PathEl::MoveTo(p) => {
                if start.is_some() {
                    return None;
                }
                start = Some(p);
                cur = p;
                continue;
            }
            PathEl::LineTo(p) => Seg::line(cur, p),
            PathEl::QuadTo(c, p) => Seg {
                p0: cur,
                c1: cur.lerp(c, 2.0 / 3.0),
                c2: p.lerp(c, 2.0 / 3.0),
                p3: p,
                line: false,
            },
            PathEl::CurveTo(c1, c2, p) => Seg {
                p0: cur,
                c1,
                c2,
                p3: p,
                line: false,
            },
            PathEl::ClosePath => {
                closed = true;
                Seg::line(cur, start?)
            }
        };
        cur = seg.p3;
        let still = |q: Point| (q - seg.p0).hypot() <= 1e-9;
        if !(still(seg.c1) && still(seg.c2) && still(seg.p3)) {
            segs.push(seg);
        }
        if closed {
            break;
        }
    }
    // A run that comes back to its first node without a close is closed.
    if let (false, Some(s)) = (closed, start) {
        closed = (cur - s).hypot() <= 1e-9;
    }
    let finite = segs.iter().all(|s| {
        [s.p0, s.c1, s.c2, s.p3]
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite())
    });
    finite.then_some((segs, closed))
}

/// Two open lines as runs of segments to pair one for one: the second
/// runs the other way when that brings each end nearer its partner (a
/// line drawn upwards blends with one drawn downwards end to end, not
/// crossed over, #1024), and the line with fewer nodes has its longest
/// segments halved until the counts match.
fn open_paired_nodes(a: &BezPath, b: &BezPath) -> Option<(Vec<Seg>, Vec<Seg>)> {
    let mut sa = open_segments(a)?;
    let mut sb = open_segments(b)?;
    if sa.len() + sb.len() > MAX_SHAPE_NODES {
        return None;
    }
    let ends = |s: &[Seg]| Some((s.first()?.p0, s.last()?.p3));
    let ((a0, a1), (b0, b1)) = (ends(&sa)?, ends(&sb)?);
    if (a0 - b1).hypot() + (a1 - b0).hypot() < (a0 - b0).hypot() + (a1 - b1).hypot() {
        sb = sb.iter().rev().map(|s| s.reversed()).collect();
    }
    let want = sa.len().max(sb.len());
    for short in [&mut sa, &mut sb] {
        while short.len() < want {
            let longest = (0..short.len())
                .max_by(|i, j| {
                    let l = |k: usize| short[k].lengths()[CHORDS];
                    l(*i)
                        .partial_cmp(&l(*j))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0);
            let seg = short[longest];
            short.splice(longest..=longest, [seg.part(0.0, 0.5), seg.part(0.5, 1.0)]);
        }
    }
    Some((sa, sb))
}

/// The segments of `segs` run from the node nearest `first`, the way that
/// reaches the node nearest `next` sooner.
fn run_segments_from(segs: &[Seg], first: Point, next: Point) -> Vec<Seg> {
    let n = segs.len();
    let nearest = |q: Point| {
        (0..n)
            .min_by(|i, j| {
                (segs[*i].p0 - q)
                    .hypot()
                    .partial_cmp(&(segs[*j].p0 - q).hypot())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(0)
    };
    let (i0, i1) = (nearest(first), nearest(next));
    if (i1 + n - i0) % n <= n / 2 {
        (0..n).map(|k| segs[(i0 + k) % n]).collect()
    } else {
        // Backward: the segment ending at node i0 first, reversed.
        (0..n)
            .map(|k| segs[(i0 + n - 1 - k) % n].reversed())
            .collect()
    }
}

/// The two closed one-part outlines `a` and `b` as runs of segments to
/// pair one for one, from the nodes given as in [`blend_from_nodes`].
/// When one has fewer nodes, its last segment (the one closing the run
/// back to its first node) is cut into equal parts until the counts
/// match: a rectangle blended with a pentagon gains a node halfway along
/// that segment, as the steps of the two blends attached to issue #597
/// show. `None` when either outline cannot be paired node for node.
fn paired_nodes(a: &BezPath, b: &BezPath, nodes: [Point; 4]) -> Option<(Vec<Seg>, Vec<Seg>)> {
    if !nodes.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
        return None;
    }
    let [a0, a1, b0, b1] = nodes;
    let mut sa = run_segments_from(&closed_segments(a)?, a0, a1);
    let mut sb = run_segments_from(&closed_segments(b)?, b0, b1);
    let fewer = if sa.len() < sb.len() {
        Some((&mut sa, sb.len()))
    } else if sb.len() < sa.len() {
        Some((&mut sb, sa.len()))
    } else {
        None
    };
    if let Some((short, want)) = fewer {
        let parts = want - short.len() + 1;
        let last = short.pop()?;
        for k in 0..parts {
            short.push(last.part(k as f64 / parts as f64, (k + 1) as f64 / parts as f64));
        }
    }
    Some((sa, sb))
}

/// The closed run of `pts` (the last point repeating the first) started at
/// the point nearest `first` and turned the way that reaches the point
/// nearest `next` sooner; the repeated point is dropped. `None` when there
/// are too few points or a node is not finite.
fn run_from(pts: &[Point], first: Point, next: Point) -> Option<Vec<Point>> {
    if ![first.x, first.y, next.x, next.y]
        .iter()
        .all(|v| v.is_finite())
    {
        return None;
    }
    let mut ring = pts.to_vec();
    if ring.len() > 1 && (ring[0] - ring[ring.len() - 1]).hypot() < 1e-9 {
        ring.pop();
    }
    let n = ring.len();
    if n < 3 {
        return None;
    }
    let nearest = |q: Point| {
        (0..n)
            .min_by(|i, j| {
                (ring[*i] - q)
                    .hypot()
                    .partial_cmp(&(ring[*j] - q).hypot())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(0)
    };
    let (i0, i1) = (nearest(first), nearest(next));
    let forward = (i1 + n - i0) % n <= n / 2;
    Some(
        (0..n)
            .map(|k| {
                let i = if forward { i0 + k } else { i0 + n - k };
                ring[i % n]
            })
            .collect(),
    )
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

/// The most nodes a step of a blend paired by shape may have (both
/// outlines' nodes together); with more, the steps are polygons through
/// points spread along the outlines, so huge outlines keep blends light.
const MAX_SHAPE_NODES: usize = 512;

/// The morph of one outline into another. Closed outlines are paired
/// node for node (see [`shape_paired_nodes`]), so the steps keep few
/// nodes, straight sides where both outlines are straight and smooth
/// curves elsewhere (#1020). Two open lines are paired end to end and
/// node for node, and their steps stay open lines (#1024). Very large
/// outlines, or an open line with a closed outline, are taken as one
/// closed run of points each.
fn blend_one(a: &BezPath, b: &BezPath, steps: usize, samples: usize) -> Vec<BezPath> {
    let pa = resample(a, samples);
    let pb = resample(b, samples);
    if pa.len() != pb.len() || pa.is_empty() {
        return Vec::new();
    }
    if let Some((sa, sb)) = shape_paired_nodes(a, b, &pa, &pb) {
        return node_steps(&sa, &sb, steps);
    }
    // Two open lines morph end to end and stay open lines.
    if let Some((sa, sb)) = open_paired_nodes(a, b) {
        return open_node_steps(&sa, &sb, steps);
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

/// Twice the signed area of the closed run of points `pts`.
fn ring_area(pts: &[Point]) -> f64 {
    pts.iter()
        .zip(pts.iter().cycle().skip(1))
        .map(|(p, q)| p.x * q.y - q.x * p.y)
        .sum()
}

/// A run of segments measured: where each one starts, as a share of
/// their whole length (the first 0), that length, and each one's
/// [`Seg::lengths`].
struct Measured {
    shares: Vec<f64>,
    total: f64,
    tables: Vec<[f64; CHORDS + 1]>,
}

fn measure(segs: &[Seg]) -> Measured {
    let tables: Vec<[f64; CHORDS + 1]> = segs.iter().map(|s| s.lengths()).collect();
    let total: f64 = tables.iter().map(|t| t[CHORDS]).sum();
    let mut at = 0.0;
    let shares = tables
        .iter()
        .map(|t| {
            let s = at / total;
            at += t[CHORDS];
            s
        })
        .collect();
    Measured {
        shares,
        total,
        tables,
    }
}

/// `segs` (measured as `m`) cut at each of `cuts` (shares of the whole
/// length, rising, from 0 to below 1) that is not one of its nodes
/// already.
fn cut_at(segs: &[Seg], m: &Measured, cuts: &[f64]) -> Vec<Seg> {
    let mut out = Vec::with_capacity(segs.len() + cuts.len());
    let mut c = 0;
    for (i, seg) in segs.iter().enumerate() {
        let from = m.shares.get(i).copied().unwrap_or(0.0);
        let to = m.shares.get(i + 1).copied().unwrap_or(1.0);
        let mut ts = vec![0.0];
        while let Some(&f) = cuts.get(c) {
            if f >= to {
                break;
            }
            if let (true, Some(table)) = (f > from, m.tables.get(i)) {
                ts.push(Seg::param_at(table, (f - from) * m.total));
            }
            c += 1;
        }
        ts.push(1.0);
        for w in ts.windows(2) {
            out.push(seg.part(w[0], w[1].max(w[0])));
        }
    }
    out
}

/// The two closed one-part outlines `a` and `b` as runs of segments to
/// pair one for one, matched by shape: `b` wound the way `a` is and
/// started at the place of its outline that best fits `a`'s first node
/// (the two outlines compared as point runs `pa` and `pb`, the resampled
/// outlines with the first point repeated last, each about its own
/// centre and at its own size, so the steps do not twist). Then each
/// outline gains a node where the other has one at the same share of
/// its length and has none itself (nodes closer than half a sample
/// apart pair up), so a square and a circle pair corner with the
/// circle's point halfway between its nodes, and the steps are rounded
/// squares of eight nodes. `None` for open outlines, outlines of no
/// length, or more than [`MAX_SHAPE_NODES`] nodes together.
fn shape_paired_nodes(
    a: &BezPath,
    b: &BezPath,
    pa: &[Point],
    pb: &[Point],
) -> Option<(Vec<Seg>, Vec<Seg>)> {
    let sa = closed_segments(a)?;
    let mut sb = closed_segments(b)?;
    if sa.len() + sb.len() > MAX_SHAPE_NODES {
        return None;
    }
    // The resampled runs without the repeated point.
    let ring = |p: &[Point]| -> Vec<Point> {
        let mut r = p.to_vec();
        if r.len() > 1 && (r[0] - r[r.len() - 1]).hypot() < 1e-9 {
            r.pop();
        }
        r
    };
    let (ra, mut rb) = (ring(pa), ring(pb));
    let m = ra.len();
    if m < 3 || rb.len() != m {
        return None;
    }
    if ring_area(&ra) * ring_area(&rb) < 0.0 {
        sb = sb.iter().rev().map(|s| s.reversed()).collect();
        // Reversed, the run still starts at the first node.
        rb[1..].reverse();
    }
    // Each run about its centre, at unit size.
    let normal = |r: &[Point]| -> Option<Vec<Vec2>> {
        let n = r.len() as f64;
        let c = Point::new(
            r.iter().map(|p| p.x).sum::<f64>() / n,
            r.iter().map(|p| p.y).sum::<f64>() / n,
        );
        let size = (r.iter().map(|p| (*p - c).hypot2()).sum::<f64>() / n).sqrt();
        let size = if size.is_finite() && size > 1e-12 {
            size
        } else {
            1.0
        };
        let v: Vec<Vec2> = r.iter().map(|p| (*p - c) / size).collect();
        v.iter()
            .all(|q| q.x.is_finite() && q.y.is_finite())
            .then_some(v)
    };
    let (na, nb) = (normal(&ra)?, normal(&rb)?);
    // The best shift: every fourth one on every fourth point first, then
    // the shifts around that one on all points (cheap enough to run on
    // every move of a drag).
    const STRIDE: usize = 4;
    let cost = |k: usize, by: usize| -> f64 {
        (0..m)
            .step_by(by)
            .map(|i| (na[i] - nb[(i + k) % m]).hypot2())
            .sum()
    };
    let best = |ks: &mut dyn Iterator<Item = usize>, by: usize| {
        ks.map(|k| (k, cost(k, by)))
            .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, _)| k)
            .unwrap_or(0)
    };
    let rough = best(&mut (0..m).step_by(STRIDE), STRIDE);
    let shift = best(
        &mut (0..=2 * STRIDE).map(|d| (rough + m + d - STRIDE) % m),
        1,
    );
    let snap = 0.5 / m as f64;
    // Start b at the share `shift / m` of its length: at its node there
    // when one is that close, otherwise at a new node.
    let mb = measure(&sb);
    let ma = measure(&sa);
    let long = |m: &Measured| m.total.is_finite() && m.total > 1e-9;
    if !(long(&ma) && long(&mb)) {
        return None;
    }
    let start = shift as f64 / m as f64;
    let near = mb
        .shares
        .iter()
        .enumerate()
        .map(|(i, s)| (i, (s - start).abs().min(1.0 - (s - start).abs())))
        .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal));
    let first = match near {
        Some((i, d)) if d < snap => i,
        _ => {
            sb = cut_at(&sb, &mb, &[start]);
            // The new node is the start of the segment after the cut.
            mb.shares.iter().take_while(|s| **s < start).count()
        }
    };
    let first = first.min(sb.len().saturating_sub(1));
    sb.rotate_left(first);
    let mb = measure(&sb);
    let (shares_a, shares_b) = (&ma.shares, &mb.shares);
    // Pair the nodes by share of length; a node without a partner within
    // `snap` gets one cut into the other outline.
    let (mut cut_a, mut cut_b) = (Vec::new(), Vec::new());
    let (mut i, mut j) = (0, 0);
    while i < shares_a.len() || j < shares_b.len() {
        match (shares_a.get(i), shares_b.get(j)) {
            (Some(x), Some(y)) if (x - y).abs() < snap => {
                i += 1;
                j += 1;
            }
            (Some(x), Some(y)) if x < y => {
                cut_b.push(*x);
                i += 1;
            }
            (Some(x), None) => {
                cut_b.push(*x);
                i += 1;
            }
            (_, Some(y)) => {
                cut_a.push(*y);
                j += 1;
            }
            (None, None) => break,
        }
    }
    let sa = cut_at(&sa, &ma, &cut_a);
    let sb = cut_at(&sb, &mb, &cut_b);
    (sa.len() == sb.len()).then_some((sa, sb))
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

pub use crate::distort::{distort, distort_in, Distort};

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

/// How far past the nib's edge a brush's rebuilt stretch runs, as a share
/// of the nib: the points there do not move, so the rebuilt stretch
/// joins the kept curve smoothly.
const REACH_MARGIN: f64 = 0.25;
/// A piece of a segment out of reach shorter than this share of the
/// reach is rebuilt along with the piece in reach, so a stroke does not
/// leave a node at every step.
const LEFTOVER_SHARE: f64 = 1.5;

/// The pieces of `seg` by whether they lie within `reach` of `at`: the
/// stretch within reach (one piece, from the first place in reach to the
/// last) and what lies before and after it, kept as it is.
fn pieces_in_reach(seg: kurbo::PathSeg, at: Point, reach: f64) -> Vec<(kurbo::PathSeg, bool)> {
    use kurbo::{ParamCurve as _, ParamCurveArclen as _};
    const SAMPLES: usize = 32;
    let inside = |t: f64| (seg.eval(t) - at).hypot() <= reach;
    let ts: Vec<f64> = (0..=SAMPLES).map(|k| k as f64 / SAMPLES as f64).collect();
    let first = ts.iter().position(|t| inside(*t));
    let last = ts.iter().rposition(|t| inside(*t));
    let (Some(a), Some(b)) = (first, last) else {
        return vec![(seg, false)];
    };
    // One sample out on each side, so the piece holds the crossing.
    let mut t0 = ts[a.saturating_sub(1)];
    let mut t1 = ts[(b + 1).min(SAMPLES)];
    // A leftover shorter than [`LEFTOVER_SHARE`] of the reach joins the
    // touched piece: otherwise a stroke that advances step by step leaves
    // a cut node, and a sliver of the old curve, at every step.
    let min_leftover = reach * LEFTOVER_SHARE;
    if t0 > 0.0 && seg.subsegment(0.0..t0).arclen(0.1) < min_leftover {
        t0 = 0.0;
    }
    if t1 < 1.0 && seg.subsegment(t1..1.0).arclen(0.1) < min_leftover {
        t1 = 1.0;
    }
    let mut out = Vec::new();
    if t0 > 0.0 {
        out.push((seg.subsegment(0.0..t0), false));
    }
    out.push((seg.subsegment(t0..t1), true));
    if t1 < 1.0 {
        out.push((seg.subsegment(t1..1.0), false));
    }
    out
}

/// A brush stroke step on each stretch of `path` the nib (`radius` around
/// `at`) reaches: each run of segment pieces within the nib and a margin
/// ([`REACH_MARGIN`]) goes through `rebuild` (with whether it is a whole
/// closed subpath) and comes back in its place; the pieces out of reach,
/// and the subpaths out of reach, stay exactly as they were. A closed
/// subpath touched in part starts from a piece out of reach, so a run
/// does not straddle its start.
fn brush_runs(
    path: &BezPath,
    at: Point,
    radius: f64,
    mut rebuild: impl FnMut(&BezPath, bool) -> BezPath,
) -> BezPath {
    use crate::geometry::Shape as _;
    use kurbo::ParamCurve as _;
    let reach = radius * (1.0 + REACH_MARGIN);
    let mut out = BezPath::new();
    for (sub, closed) in subpaths(path) {
        if !sub.bounding_box().inflate(reach, reach).contains(at) {
            out.extend(sub.elements().iter().copied());
            continue;
        }
        let mut pieces: Vec<(kurbo::PathSeg, bool)> = sub
            .segments()
            .flat_map(|seg| pieces_in_reach(seg, at, reach))
            .collect();
        if pieces.is_empty() {
            out.extend(sub.elements().iter().copied());
            continue;
        }
        let Some(first_free) = pieces.iter().position(|(_, touched)| !touched) else {
            // The whole subpath is within reach.
            let rebuilt = rebuild(&sub, closed);
            out.extend(rebuilt.elements().iter().copied());
            continue;
        };
        if !pieces.iter().any(|(_, touched)| *touched) {
            out.extend(sub.elements().iter().copied());
            continue;
        }
        if closed {
            pieces.rotate_left(first_free);
        }
        out.move_to(pieces[0].0.start());
        let mut i = 0;
        while i < pieces.len() {
            if !pieces[i].1 {
                out.push(pieces[i].0.as_path_el());
                i += 1;
                continue;
            }
            let mut run = BezPath::new();
            run.move_to(pieces[i].0.start());
            while i < pieces.len() && pieces[i].1 {
                run.push(pieces[i].0.as_path_el());
                i += 1;
            }
            let rebuilt = rebuild(&run, false);
            out.extend(
                rebuilt
                    .elements()
                    .iter()
                    .filter(|e| !matches!(e, PathEl::MoveTo(_)))
                    .copied(),
            );
        }
        if closed {
            out.close_path();
        }
    }
    out
}

/// A brush stroke step on each stretch the nib (`radius` around `at`)
/// reaches (see [`brush_runs`]): its points `max_len` apart go through
/// `f` and come back as smooth curves (or straight lines with `sharp`).
fn brush_each(
    path: &BezPath,
    at: Point,
    radius: f64,
    max_len: f64,
    sharp: bool,
    mut f: impl FnMut(Vec<Point>, bool) -> Vec<Point>,
) -> BezPath {
    brush_runs(path, at, radius, |run, closed| {
        let pts = f(densify(run, max_len), closed);
        if sharp {
            crate::geometry::polyline_path(&pts, closed)
        } else {
            fewer_curves(&crate::geometry::smooth_path(&pts, closed))
        }
    })
}

/// How far a brush's rebuilt curve may move when its many small curves
/// are merged into few, mm.
const MERGE_ACCURACY: f64 = 0.1;

/// `path` (smooth curves through many close points) with its curves
/// merged into as few as keep it within [`MERGE_ACCURACY`] (kurbo's curve
/// fitter), so a brush stroke leaves a handful of nodes, not one every
/// few millimetres. The path itself when the fit fails.
fn fewer_curves(path: &BezPath) -> BezPath {
    fit_within(path, MERGE_ACCURACY)
}

/// Push points within `radius` of `at` along `delta` (Smear tool).
pub fn smear(path: &BezPath, at: Point, delta: Vec2, radius: f64) -> BezPath {
    brush_each(path, at, radius, radius / 6.0, false, |pts, _| {
        pts.into_iter()
            .map(|p| {
                let d = (p - at).hypot();
                if d < radius {
                    let w = 1.0 - d / radius;
                    p + delta * (w * w)
                } else {
                    p
                }
            })
            .collect()
    })
}

/// Rotate points within `radius` of `at` by up to `angle` (Twirl tool).
pub fn twirl(path: &BezPath, at: Point, angle: f64, radius: f64) -> BezPath {
    brush_each(path, at, radius, radius / 6.0, false, |pts, _| {
        pts.into_iter()
            .map(|p| {
                let v = p - at;
                let d = v.hypot();
                if d < radius {
                    let a = angle * (1.0 - d / radius);
                    at + Vec2::new(v.x * a.cos() - v.y * a.sin(), v.x * a.sin() + v.y * a.cos())
                } else {
                    p
                }
            })
            .collect()
    })
}

/// Smooth tool: the stretch of the curve within the nib's reach is
/// measured into close points (half a millimetre apart), each eased
/// towards the middle of its neighbours by the nib's weight (full at
/// the centre, none at the edge; the ends stay put) a few times over,
/// so jags and wobbles melt away a little with every step of the
/// stroke; then the points that matter within `tolerance` mm are kept
/// (Ramer-Douglas-Peucker at half of it), a smooth curve is fitted
/// through them and merged into as few curves as stay within the
/// tolerance (kurbo's fitter), so the stretch comes back with a handful
/// of nodes rather than one every few millimetres. The rest of the
/// curve is exactly as it was.
pub fn smooth(path: &BezPath, at: Point, radius: f64, tolerance: f64) -> BezPath {
    let tolerance = tolerance.max(1e-3);
    brush_runs(path, at, radius, |run, closed| {
        let mut pts = densify(run, SMOOTH_STEP);
        let n = pts.len();
        if n < 3 {
            return run.clone();
        }
        let weight = |p: Point| (1.0 - (p - at).hypot() / radius).clamp(0.0, 1.0);
        let w: Vec<f64> = pts.iter().map(|p| weight(*p)).collect();
        // Each point goes towards the average of the stretch around it
        // (a span of the nib's size, nearer points counting more), by
        // the nib's weight at the point.
        let span = ((radius * SMOOTH_SPAN) / SMOOTH_STEP).ceil().max(1.0) as usize;
        let before = pts.clone();
        for i in 0..n {
            if w[i] <= 0.0 || (!closed && (i == 0 || i == n - 1)) {
                continue;
            }
            let mut sum = Vec2::ZERO;
            let mut total = 0.0;
            for k in 0..=span {
                let share = 1.0 - k as f64 / (span + 1) as f64;
                let both = if k == 0 { 1 } else { 2 };
                for side in 0..both {
                    let j = if closed {
                        if side == 0 {
                            (i + k) % n
                        } else {
                            (i + n - k) % n
                        }
                    } else if side == 0 {
                        (i + k).min(n - 1)
                    } else {
                        i.saturating_sub(k)
                    };
                    sum += before[j].to_vec2() * share;
                    total += share;
                }
            }
            let mean = (sum / total).to_point();
            pts[i] = before[i] + (mean - before[i]) * (SMOOTH_EASE * w[i]);
        }
        // The points that matter within the tolerance, a smooth curve
        // through them, and that curve in as few pieces as stay within
        // the tolerance (the fitter merges curves, not lines).
        let pts = crate::geometry::simplify(&pts, tolerance * 0.5);
        fit_within(&crate::geometry::smooth_path(&pts, closed), tolerance)
    })
}

/// How close the points Smooth works on are, mm.
const SMOOTH_STEP: f64 = 0.5;
/// How far along the curve, as a share of the nib's radius, the stretch
/// a point is eased towards reaches on each side.
const SMOOTH_SPAN: f64 = 0.25;
/// How far a point goes towards the average of the stretch around it
/// each step (times the nib's weight).
const SMOOTH_EASE: f64 = 0.75;

/// `path` fitted with as few curves as keep within `accuracy` mm of it
/// (kurbo's fitter); the path itself when the fit fails.
fn fit_within(path: &BezPath, accuracy: f64) -> BezPath {
    use kurbo::simplify::{simplify_bezpath, SimplifyOptLevel, SimplifyOptions};
    let options = SimplifyOptions::default().opt_level(SimplifyOptLevel::Optimize);
    let fitted = simplify_bezpath(path.elements().iter().copied(), accuracy, &options);
    let sane = !fitted.elements().is_empty()
        && fitted.elements().iter().all(|e| match *e {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => p.is_finite(),
            PathEl::QuadTo(a, p) => a.is_finite() && p.is_finite(),
            PathEl::CurveTo(a, b, p) => a.is_finite() && b.is_finite() && p.is_finite(),
            PathEl::ClosePath => true,
        });
    if sane {
        fitted
    } else {
        path.clone()
    }
}

/// Pull (`amount` > 0) or push (< 0) the nodes and control points within
/// `radius` of `at` towards or away from it (Attract and Repel tools);
/// nearer ones move more, none past the middle.
pub fn attract(path: &BezPath, at: Point, radius: f64, amount: f64) -> BezPath {
    let f = |p: Point| -> Point {
        let v = at - p;
        let d = v.hypot();
        if d > radius || d < 1e-9 {
            return p;
        }
        let k = (1.0 - d / radius) * amount;
        p + v * (k / d).min(0.9)
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
pub fn roughen(path: &BezPath, at: Point, radius: f64, amount: f64, spacing: f64) -> BezPath {
    brush_each(path, at, radius, spacing.max(0.05), true, |pts, closed| {
        let n = pts.len();
        if n < 3 {
            return pts;
        }
        (0..n)
            .map(|i| {
                let p = pts[i];
                let d = (p - at).hypot();
                if d > radius {
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
                p + normal * (r * amount * (1.0 - d / radius))
            })
            .collect()
    })
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

    /// The brushes change only the subpaths the nib reaches, keep open
    /// curves open and closed ones closed.
    #[test]
    fn brushes_touch_only_the_subpaths_they_reach() {
        let mut two = Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.01);
        let far = Rect::new(100.0, 0.0, 110.0, 10.0).to_path(0.01);
        two.extend(far.elements().iter().copied());
        let at = Point::new(10.0, 5.0);
        for out in [
            smear(&two, at, Vec2::new(3.0, 0.0), 4.0),
            twirl(&two, at, 0.5, 4.0),
            smooth(&two, at, 4.0, 0.3),
            roughen(&two, at, 4.0, 1.0, 0.5),
        ] {
            let subs = subpaths(&out);
            assert_eq!(subs.len(), 2);
            assert!(subs.iter().all(|(_, closed)| *closed));
            // The far square is exactly as it was.
            assert_eq!(subs[1].0, far);
            assert_ne!(subs[0].0.bounding_box(), Rect::ZERO);
        }
        // Smear pushed the near square's right side out.
        let pushed = smear(&two, at, Vec2::new(3.0, 0.0), 4.0);
        assert!(subpaths(&pushed)[0].0.bounding_box().x1 > 11.0);
        // An open curve stays open.
        let mut line = BezPath::new();
        line.move_to(Point::new(0.0, 0.0));
        line.line_to(Point::new(20.0, 0.0));
        let rough = roughen(&line, Point::new(10.0, 0.0), 5.0, 1.0, 0.5);
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
        let (pulled, pushed) = (attract(&sq, c, 5.0, 1.0), attract(&sq, c, 5.0, -1.0));
        assert!(reach(&pulled) < 20.0 && reach(&pushed) > 20.0);
    }

    /// The nodes of a path's elements (end points).
    fn node_points(p: &BezPath) -> Vec<Point> {
        p.elements()
            .iter()
            .filter_map(|e| match *e {
                PathEl::MoveTo(q) | PathEl::LineTo(q) => Some(q),
                PathEl::QuadTo(_, q) | PathEl::CurveTo(_, _, q) => Some(q),
                PathEl::ClosePath => None,
            })
            .collect()
    }

    /// A brush reshapes only the stretch within its reach: the segments
    /// out of reach keep their nodes and handles exactly, so a stroke does
    /// not turn a curve of a few nodes into hundreds (#227).
    #[test]
    fn brushes_leave_the_segments_out_of_reach_exactly() {
        // A blob of eight nodes, as the Freehand tool fits one.
        let pts: Vec<Point> = (0..8)
            .map(|k| {
                let a = k as f64 / 8.0 * std::f64::consts::TAU;
                Point::new(100.0 + 40.0 * a.cos(), 100.0 + 25.0 * a.sin())
            })
            .collect();
        for closed in [true, false] {
            let blob = crate::geometry::smooth_path(&pts, closed);
            let at = Point::new(140.0, 100.0);
            for (name, out) in [
                ("smear", smear(&blob, at, Vec2::new(2.0, 0.0), 6.0)),
                ("twirl", twirl(&blob, at, 0.3, 6.0)),
                ("smooth", smooth(&blob, at, 6.0, 0.3)),
                ("roughen", roughen(&blob, at, 6.0, 1.0, 1.0)),
            ] {
                assert_eq!(subpaths(&out)[0].1, closed, "{name}");
                // The elements lying more than 21.5 mm from the nib (start
                // and end; the stretch in reach ends 21.2 mm out) are the
                // originals.
                let far = |p: &BezPath| -> Vec<PathEl> {
                    let mut cur = Point::ZERO;
                    let mut out = Vec::new();
                    for e in p.elements() {
                        let end = match *e {
                            PathEl::MoveTo(q) | PathEl::LineTo(q) => q,
                            PathEl::QuadTo(_, q) | PathEl::CurveTo(_, _, q) => q,
                            PathEl::ClosePath => continue,
                        };
                        let keep = !matches!(e, PathEl::MoveTo(_))
                            && (cur - at).hypot() > 21.5
                            && (end - at).hypot() > 21.5;
                        if keep {
                            out.push(*e);
                        }
                        cur = end;
                    }
                    out
                };
                let kept = far(&out);
                assert!(kept.len() >= 4, "{name}: {}", kept.len());
                assert!(
                    kept.iter().all(|e| blob.elements().contains(e)),
                    "{name}: {kept:?}"
                );
                // Roughen's spikes are a millimetre apart across the nib.
                let room = if name == "roughen" { 20 } else { 10 };
                assert!(
                    node_points(&out).len() <= node_points(&blob).len() + room,
                    "{name}: {} nodes",
                    node_points(&out).len()
                );
            }
        }
    }

    /// Smooth fits the jagged stretch within the nib with a few curves,
    /// within its tolerance; the rest of the line and the corner stay.
    /// A wobbly spiral drawn by hand (300 jittered points), smoothed by
    /// a stroke of 150 steps along it: the stroke leaves a few nodes
    /// per turn, not one every few millimetres, and the curve stays on
    /// the spiral instead of wandering (the owner's report: the lines
    /// came out broken and full of nodes).
    #[test]
    fn a_smooth_stroke_along_a_wobbly_spiral_leaves_few_nodes_on_it() {
        let spiral = |k: usize| -> Point {
            let t = k as f64 / 300.0;
            let a = t * 4.0 * std::f64::consts::TAU;
            let r = 10.0 + 60.0 * t;
            Point::new(100.0 + r * a.cos(), 100.0 + r * a.sin())
        };
        let mut hand = BezPath::new();
        for k in 0..=300 {
            let p = spiral(k);
            // Jitter of up to 0.4 mm, as a shaky hand gives.
            let j = ((k * 7919) % 17) as f64 / 16.0 - 0.5;
            let q = Point::new(p.x + 0.8 * j, p.y - 0.8 * j);
            if k == 0 {
                hand.move_to(q);
            } else {
                hand.line_to(q);
            }
        }
        // A stroke along the spiral: a step every 7 mm or so, as a
        // pointer moves.
        let mut out = hand.clone();
        for k in (0..=300).step_by(2) {
            out = smooth(&out, spiral(k), 10.0, 0.33);
        }
        let nodes = node_points(&out);
        assert!(nodes.len() < 60, "{} nodes", nodes.len());
        // On the spiral: every node within a millimetre of it.
        let off = |p: Point| -> f64 {
            (0..=3000)
                .map(|k| {
                    let t = k as f64 / 3000.0;
                    let a = t * 4.0 * std::f64::consts::TAU;
                    let r = 10.0 + 60.0 * t;
                    (Point::new(100.0 + r * a.cos(), 100.0 + r * a.sin()) - p).hypot()
                })
                .fold(f64::MAX, f64::min)
        };
        let worst = nodes.iter().map(|p| off(*p)).fold(0.0, f64::max);
        assert!(worst < 1.0, "a node {worst} mm off the spiral");
        // The ends, out of the strokes' reach at the start, are as drawn.
        assert_eq!(nodes[0], node_points(&hand)[0]);
    }

    #[test]
    fn smooth_fits_jagged_lines_within_the_nib() {
        // A jagged line of 60 nodes along y = 0, then a right-angle turn.
        let mut jagged = BezPath::new();
        jagged.move_to(Point::new(0.0, 0.0));
        for k in 1..=60 {
            let y = if k % 2 == 0 { 0.1 } else { -0.1 };
            jagged.line_to(Point::new(k as f64, y));
        }
        jagged.line_to(Point::new(60.0, -30.0));
        let out = smooth(&jagged, Point::new(30.0, 0.0), 8.0, 0.3);
        let nodes = node_points(&out);
        // The stretch within reach (x within 22..38, 16 nodes) is a few now.
        let within = |p: &BezPath| {
            node_points(p)
                .iter()
                .filter(|q| q.x > 22.5 && q.x < 37.5)
                .count()
        };
        assert!(
            within(&out) <= 4,
            "{} nodes left in the nib: {out:?}",
            within(&out)
        );
        assert!(nodes.len() < node_points(&jagged).len() - 8);
        // Still within tolerance of the line, and the corner is there.
        for q in &nodes {
            if q.x < 59.9 {
                assert!(q.y.abs() <= 0.5, "{q:?}");
            }
        }
        assert!(nodes.contains(&Point::new(60.0, -30.0)));
        // The jags out of reach are as they were.
        assert!(nodes.contains(&Point::new(10.0, 0.1)) && nodes.contains(&Point::new(50.0, 0.1)));
        // The nib over the corner: the turn is still there, smoothed, and
        // the far end of the line out of reach is as it was.
        let out = smooth(&jagged, Point::new(60.0, 0.0), 8.0, 0.3);
        let nodes = node_points(&out);
        assert!(nodes.contains(&Point::new(60.0, -30.0)), "{out:?}");
        assert!(nodes.contains(&Point::new(10.0, 0.1)), "{out:?}");
        let b = out.bounding_box();
        assert!(b.x1 < 62.0 && b.y0 > -30.1 && b.y1 < 1.5, "{b:?}");
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

    /// A square blended into a circle (#1020): each step has eight nodes
    /// (the square's four corners and the circle's four nodes, each
    /// paired with the other outline's point at the same share of its
    /// length), not a polygon of a hundred points, and its outline is
    /// even: convex, and between the two shapes in size.
    #[test]
    fn blend_of_a_square_and_a_circle_has_few_nodes_and_even_outlines() {
        let square = Rect::new(0.0, 0.0, 40.0, 40.0).to_path(0.01);
        let circle =
            crate::geometry::Ellipse::from_rect(Rect::new(100.0, 0.0, 140.0, 40.0)).to_path(0.01);
        let steps = blend(&square, &circle, 5, 120);
        assert_eq!(steps.len(), 5);
        let mut last_area = square.area().abs();
        for s in &steps {
            let els = s.elements();
            let nodes = els
                .iter()
                .filter(|e| !matches!(e, PathEl::MoveTo(_) | PathEl::ClosePath))
                .count();
            assert_eq!(nodes, 8, "{els:?}");
            // Convex all round: every turn of its flattened outline the
            // same way, no lumps or dents.
            let pts = flat_points(s);
            let turns: Vec<f64> = pts
                .windows(3)
                .map(|w| (w[1] - w[0]).cross(w[2] - w[1]))
                .filter(|c| c.abs() > 1e-9)
                .collect();
            assert!(
                turns.iter().all(|c| *c > 0.0) || turns.iter().all(|c| *c < 0.0),
                "{turns:?}"
            );
            // Rounder step by step: the area falls from the square's to
            // the circle's, the size stays the same.
            let area = s.area().abs();
            assert!(area < last_area + 1e-6, "{area} after {last_area}");
            last_area = area;
            let bb = s.bounding_box();
            assert!((bb.width() - 40.0).abs() < 0.5, "{bb:?}");
            assert!((bb.height() - 40.0).abs() < 0.5, "{bb:?}");
        }
        assert!(last_area > circle.area().abs());
    }

    /// Two squares of different size blend into squares: four straight
    /// sides each, the corners kept exactly where they lie between the
    /// two squares' corners.
    #[test]
    fn blend_of_two_squares_keeps_straight_sides_and_corners() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.01);
        let b = Rect::new(50.0, 0.0, 80.0, 30.0).to_path(0.01);
        let mid = blend(&a, &b, 1, 120);
        let els = mid[0].elements();
        assert_eq!(els.len(), 6, "{els:?}");
        assert!(els[1..5].iter().all(|e| matches!(e, PathEl::LineTo(_))));
        let bb = mid[0].bounding_box();
        assert!((bb.x0 - 25.0).abs() < 1e-6 && (bb.x1 - 45.0).abs() < 1e-6);
        assert!((bb.y0).abs() < 1e-6 && (bb.y1 - 20.0).abs() < 1e-6);
        assert!((mid[0].area().abs() - 400.0).abs() < 1e-6);
    }

    /// Paired from given nodes, a square blends into the same square as
    /// they are paired: from matching nodes the way they run, the middle
    /// step is that square; from the next corner it turns (a quarter turn
    /// halfway is a smaller, turned square); run the other way it folds
    /// flat (#597). Without nodes it is the plain blend.
    #[test]
    fn blend_from_nodes_pairs_the_outlines_from_their_nodes() {
        let sq = Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.01);
        let area = |p: &BezPath| p.area().abs();
        let (o, x, y, xy) = (
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
        );
        let mid = |nodes| blend_from_nodes(&sq, &sq, 1, 64, Some(nodes))[0].clone();
        assert!((area(&mid([o, x, o, x])) - 100.0).abs() < 1.0);
        let turned = mid([o, x, x, xy]);
        assert!(
            area(&turned) < 80.0 && area(&turned) > 20.0,
            "{}",
            area(&turned)
        );
        let c = turned.bounding_box().center();
        assert!((c.x - 5.0).abs() < 0.5 && (c.y - 5.0).abs() < 0.5, "{c:?}");
        assert!(area(&mid([o, x, o, y])) < 5.0);
        let plain = blend(&sq, &sq, 1, 64);
        assert_eq!(blend_from_nodes(&sq, &sq, 1, 64, None), plain);
        // Nodes that are not numbers: the plain blend, no panic.
        let nan = Point::new(f64::NAN, 0.0);
        assert_eq!(
            blend_from_nodes(&sq, &sq, 1, 64, Some([nan, x, o, x])),
            plain
        );
    }

    /// Paired from given nodes, a rectangle and a pentagon blend node for
    /// node: every step is a pentagon with sharp corners (straight sides
    /// between the paired nodes, nothing cut off), the rectangle gaining
    /// its fifth node halfway along the side that closes its run back to
    /// its first node, as the blends attached to #597 draw.
    #[test]
    fn blend_from_nodes_pairs_a_rectangle_and_a_pentagon_node_for_node() {
        let rect = Rect::new(0.0, 0.0, 70.0, 60.0).to_path(0.01);
        // A pentagon away to the left, from its top vertex clockwise.
        let pent: Vec<Point> = (0..5)
            .map(|i| {
                let a = std::f64::consts::FRAC_PI_2 - i as f64 * std::f64::consts::TAU / 5.0;
                Point::new(-200.0 + 50.0 * a.cos(), 50.0 * a.sin())
            })
            .collect();
        let pentagon = polygon(&pent);
        // The rectangle from its upper right corner, down its height first.
        let (r0, r1) = (Point::new(70.0, 60.0), Point::new(70.0, 0.0));
        let steps = blend_from_nodes(&rect, &pentagon, 3, 64, Some([r0, r1, pent[0], pent[1]]));
        assert_eq!(steps.len(), 3);
        for s in &steps {
            let els = s.elements();
            assert_eq!(els.len(), 7, "{els:?}");
            assert!(els[1..6].iter().all(|e| matches!(e, PathEl::LineTo(_))));
        }
        let nodes: Vec<Point> = steps[1]
            .elements()
            .iter()
            .filter_map(|e| match e {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => Some(*p),
                _ => None,
            })
            .collect();
        // The rectangle runs (70,60), (70,0), (0,0), (0,60), then the
        // middle of its closing side, (35,60), paired with the pentagon's
        // fifth vertex.
        let want = [
            Point::new(70.0, 60.0),
            Point::new(70.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(0.0, 60.0),
            Point::new(35.0, 60.0),
        ];
        for (i, (got, r)) in nodes.iter().zip(want).enumerate() {
            let w = r.midpoint(pent[i]);
            assert!((*got - w).hypot() < 1e-9, "node {i}: {got:?} vs {w:?}");
        }
        // Run the other way, the rectangle starts down its other side.
        let back = blend_from_nodes(
            &rect,
            &pentagon,
            1,
            64,
            Some([r0, Point::new(0.0, 60.0), pent[0], pent[1]]),
        );
        let second = match back[0].elements()[1] {
            PathEl::LineTo(p) => p,
            ref e => panic!("{e:?}"),
        };
        assert!((second - Point::new(0.0, 60.0).midpoint(pent[1])).hypot() < 1e-9);
    }

    /// Curved outlines pair node for node too, their control points
    /// blended: an ellipse (four curves) and a triangle give steps of four
    /// curves, the triangle's closing side cut in two. Open outlines and
    /// outlines of very many nodes are paired along their length.
    #[test]
    fn blend_from_nodes_pairs_curves_and_falls_back_for_others() {
        let ellipse =
            crate::geometry::Ellipse::from_rect(Rect::new(0.0, 0.0, 40.0, 20.0)).to_path(0.01);
        let tri = polygon(&[
            Point::new(100.0, 0.0),
            Point::new(140.0, 0.0),
            Point::new(120.0, 30.0),
        ]);
        let first = match ellipse.elements()[0] {
            PathEl::MoveTo(p) => p,
            ref e => panic!("{e:?}"),
        };
        let next = match ellipse.elements()[1] {
            PathEl::CurveTo(_, _, p) => p,
            ref e => panic!("{e:?}"),
        };
        let nodes = [first, next, Point::new(100.0, 0.0), Point::new(140.0, 0.0)];
        let steps = blend_from_nodes(&ellipse, &tri, 2, 64, Some(nodes));
        assert_eq!(steps.len(), 2);
        let curves = steps[0]
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::CurveTo(..)))
            .count();
        assert_eq!(curves, 4, "{:?}", steps[0].elements());
        for s in &steps {
            let c = s.bounding_box().center();
            assert!(c.x > 20.0 && c.x < 120.0, "{c:?}");
        }
        // An open outline: paired along its length, still steps.
        let mut open = BezPath::new();
        open.move_to((0.0, 0.0));
        open.line_to((10.0, 0.0));
        open.line_to((10.0, 10.0));
        assert_eq!(blend_from_nodes(&open, &tri, 2, 64, Some(nodes)).len(), 2);
        // Many nodes: paired along the length (no blow-up), still steps.
        let many: Vec<Point> = (0..MAX_PAIRED_NODES + 10)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / (MAX_PAIRED_NODES + 10) as f64;
                Point::new(10.0 * a.cos(), 10.0 * a.sin())
            })
            .collect();
        let big = blend_from_nodes(&polygon(&many), &tri, 2, 64, Some(nodes));
        assert_eq!(big.len(), 2);
        assert!(big[0].elements().len() < 200);
    }

    /// Two arcs, the second turned half round so it runs the other way
    /// (the drawing in #1024): the steps are open arcs joining top to top
    /// and bottom to bottom, not closed outlines crossed through the middle.
    #[test]
    fn blend_of_two_open_arcs_runs_end_to_end() {
        let mut a = BezPath::new();
        a.move_to((0.0, 100.0));
        a.curve_to((-30.0, 70.0), (-30.0, 30.0), (0.0, 0.0));
        // The mirror image, drawn from its bottom end.
        let mut b = BezPath::new();
        b.move_to((40.0, 0.0));
        b.curve_to((70.0, 30.0), (70.0, 70.0), (40.0, 100.0));
        let steps = blend(&a, &b, 3, 120);
        assert_eq!(steps.len(), 3);
        for (k, p) in steps.iter().enumerate() {
            let els = p.elements();
            assert!(!els.iter().any(|e| matches!(e, PathEl::ClosePath)), "{p:?}");
            assert_eq!(els.len(), 2, "one curve, as the arcs: {p:?}");
            let t = (k + 1) as f64 / 4.0;
            let (PathEl::MoveTo(top), PathEl::CurveTo(_, _, bottom)) = (els[0], els[1]) else {
                panic!("{p:?}");
            };
            assert!(
                (top - Point::new(40.0 * t, 100.0)).hypot() < 1e-9,
                "{top:?}"
            );
            assert!(
                (bottom - Point::new(40.0 * t, 0.0)).hypot() < 1e-9,
                "{bottom:?}"
            );
        }
        // The middle step is straight up and down: the two bows cancel.
        let mid = steps[1].bounding_box();
        assert!(mid.width() < 1e-9, "{mid:?}");
        // A line with more nodes pairs with one of fewer.
        let mut c = BezPath::new();
        c.move_to((40.0, 100.0));
        c.line_to((60.0, 50.0));
        c.line_to((40.0, 0.0));
        let steps = blend(&a, &c, 1, 120);
        assert_eq!(steps[0].elements().len(), 3, "{:?}", steps[0]);
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
