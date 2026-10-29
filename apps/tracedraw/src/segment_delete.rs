//! Segment Delete: removes the stretch of an outline between the nearest
//! places it crosses another outline (or itself), its virtual segment;
//! where nothing crosses, between its open ends, or a whole closed
//! subpath. A click removes the virtual segment under the pointer; a drag
//! removes every virtual segment the rectangle touches, whole (#528); a
//! freehand drag (the bar's Freehand, or Alt held at the press) removes
//! every virtual segment the stroke crosses, whole (#1150).
//!
//! Crossings are found against the outlines of every visible object on
//! the page's unlocked layers (flattened to 0.02 mm), the object's other
//! subpaths and its own segments that are not next to the one cut. The
//! object becomes a curve; a closed subpath with a stretch removed opens;
//! an object with nothing left is deleted. One undo step.

use crate::app::App;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{
        BezPath, Line, ParamCurve, ParamCurveNearest, PathEl, PathSeg, Point, Rect, Shape as _,
    },
    Command, ShapeId,
};

/// One subpath as segments.
#[derive(Debug, Clone)]
pub struct Sub {
    pub segs: Vec<PathSeg>,
    pub closed: bool,
}

/// The subpaths of `path`; one that ends where it started counts as
/// closed.
pub fn subs_of(path: &BezPath) -> Vec<Sub> {
    tracedraw_core::effects::subpaths(path)
        .into_iter()
        .filter_map(|(sub, closed)| {
            let mut segs: Vec<PathSeg> = sub.segments().collect();
            if segs.is_empty() {
                return None;
            }
            // `segments()` adds the closing line of a closed subpath; one
            // with no length is dropped.
            if segs
                .last()
                .is_some_and(|s| (s.end() - s.start()).hypot() < 1e-9 && segs.len() > 1)
            {
                segs.pop();
            }
            let ends_at_start =
                segs.len() > 1 && (segs[segs.len() - 1].end() - segs[0].start()).hypot() < 1e-6;
            Some(Sub {
                segs,
                closed: closed || ends_at_start,
            })
        })
        .collect()
}

/// `path` flattened into lines (a closed subpath's closing line too).
/// How far a cutter reaches past its ends, mm: enough for rounding, far
/// too little to see.
const CUTTER_REACH: f64 = 1e-3;

/// `line` lengthened by [`CUTTER_REACH`] at both ends (a point stays).
fn stretched(line: Line) -> Line {
    let d = line.p1 - line.p0;
    let len = d.hypot();
    if len < 1e-12 {
        return line;
    }
    let e = d * (CUTTER_REACH / len);
    Line::new(line.p0 - e, line.p1 + e)
}

/// How near the node two neighbouring segments share a crossing of theirs
/// must be to be that node rather than a crossing, mm (past the
/// flattening error).
const NODE_REACH: f64 = 0.05;

/// Where a segment crosses itself (a cubic with a loop): the two
/// parameters of the crossing, else nothing. Found between chords of
/// the curve and refined on the curve itself.
pub fn self_crossings(seg: &PathSeg) -> Vec<f64> {
    let PathSeg::Cubic(_) = seg else {
        return Vec::new();
    };
    const N: usize = 48;
    let pts: Vec<Point> = (0..=N).map(|k| seg.eval(k as f64 / N as f64)).collect();
    let mut out = Vec::new();
    for a in 0..N {
        let la = Line::new(pts[a], pts[a + 1]);
        for b in a + 2..N {
            let lb = Line::new(pts[b], pts[b + 1]);
            let crossing = PathSeg::Line(la).intersect_line(lb).into_iter().any(|h| {
                (-1e-9..=1.0 + 1e-9).contains(&h.line_t)
                    && (-1e-9..=1.0 + 1e-9).contains(&h.segment_t)
            });
            if !crossing {
                continue;
            }
            // The curve's own parameters: where it meets each chord, near
            // the other chord's stretch of it.
            let near = |t: f64, k: usize| {
                let lo = (k as f64 - 1.0) / N as f64;
                let hi = (k as f64 + 2.0) / N as f64;
                (lo..=hi).contains(&t)
            };
            for (chord, k) in [(lb, a), (la, b)] {
                for h in seg.intersect_line(stretched(chord)) {
                    let t = h.segment_t.clamp(0.0, 1.0);
                    if (-1e-9..=1.0 + 1e-9).contains(&h.line_t) && near(t, k) {
                        out.push(t);
                    }
                }
            }
        }
    }
    out.sort_by(f64::total_cmp);
    out.dedup_by(|x, y| (*x - *y).abs() < 1e-7);
    // The two parameters refined together onto the crossing point:
    // Newton steps on c(t1) - c(t2) = 0.
    if let [t1, t2] = out[..] {
        let (mut a, mut b) = (t1, t2);
        for _ in 0..12 {
            let f = seg.eval(a) - seg.eval(b);
            let (da, db) = (deriv(seg, a), deriv(seg, b));
            // The Jacobian [da, -db] and its inverse.
            let det = da.x * (-db.y) - (-db.x) * da.y;
            if det.abs() < 1e-12 {
                break;
            }
            let sa = (f.x * (-db.y) - (-db.x) * f.y) / det;
            let sb = (da.x * f.y - da.y * f.x) / det;
            a -= sa;
            b -= sb;
            if !(0.0..=1.0).contains(&a) || !(0.0..=1.0).contains(&b) {
                return out;
            }
            if sa.abs() < 1e-13 && sb.abs() < 1e-13 {
                break;
            }
        }
        if (seg.eval(a) - seg.eval(b)).hypot() < (seg.eval(t1) - seg.eval(t2)).hypot() {
            out = vec![a.min(b), a.max(b)];
        }
    }
    out
}

/// The derivative of `seg` at `t`.
fn deriv(seg: &PathSeg, t: f64) -> tracedraw_core::geometry::Vec2 {
    use tracedraw_core::geometry::ParamCurveDeriv;
    match seg {
        PathSeg::Line(l) => l.deriv().eval(t).to_vec2(),
        PathSeg::Quad(q) => q.deriv().eval(t).to_vec2(),
        PathSeg::Cubic(c) => c.deriv().eval(t).to_vec2(),
    }
}

fn lines_of(path: &BezPath) -> Vec<Line> {
    let mut out = Vec::new();
    let mut start = Point::ZERO;
    let mut last = Point::ZERO;
    tracedraw_core::geometry::flatten(path, 0.02, &mut |el| match el {
        PathEl::MoveTo(p) => {
            start = p;
            last = p;
        }
        PathEl::LineTo(p) => {
            out.push(Line::new(last, p));
            last = p;
        }
        PathEl::ClosePath => {
            if (last - start).hypot() > 1e-9 {
                out.push(Line::new(last, start));
            }
            last = start;
        }
        _ => {}
    });
    out
}

/// A segment as a path of its own.
fn seg_path(s: &PathSeg) -> BezPath {
    let mut p = BezPath::new();
    p.move_to(s.start());
    match *s {
        PathSeg::Line(l) => p.line_to(l.p1),
        PathSeg::Quad(q) => p.quad_to(q.p1, q.p2),
        PathSeg::Cubic(c) => p.curve_to(c.p1, c.p2, c.p3),
    }
    p
}

/// One segment of an outline that cuts others: the segment itself and
/// its flattening (found against first, then refined on the segment).
#[derive(Debug, Clone)]
pub struct Cutter {
    pub seg: PathSeg,
    lines: Vec<Line>,
    bounds: Rect,
}

impl Cutter {
    pub fn new(seg: PathSeg) -> Self {
        let lines = lines_of(&seg_path(&seg));
        let bounds =
            tracedraw_core::geometry::Shape::bounding_box(&seg_path(&seg)).inflate(0.05, 0.05);
        Cutter { seg, lines, bounds }
    }
}

/// The cutters of every segment of `path`.
pub fn cutters_of(path: &BezPath) -> Vec<Cutter> {
    subs_of(path)
        .iter()
        .flat_map(|s| s.segs.iter().copied())
        .map(Cutter::new)
        .collect()
}

/// `t` on `seg`, found against the flattening of `other`, moved onto the
/// true crossing of the two: Newton steps on seg(a) - other(b) = 0. A
/// crossing found against a flattened curve lies up to the flattening
/// error off the true one, so a cut there would leave a piece ending a
/// little short of the outline it crossed, and the next cut would miss
/// it (#354). `t` stays when the steps do not settle near it.
fn refine(seg: &PathSeg, t: f64, other: &PathSeg) -> f64 {
    let mut a = t;
    let mut b = other.nearest(seg.eval(t), 1e-9).t;
    for _ in 0..16 {
        let f = seg.eval(a) - other.eval(b);
        if f.hypot() < 1e-12 {
            break;
        }
        let (da, db) = (deriv(seg, a), deriv(other, b));
        // The Jacobian [da, -db] and its inverse.
        let det = da.x * (-db.y) - (-db.x) * da.y;
        if det.abs() < 1e-12 {
            return t;
        }
        a -= (f.x * (-db.y) - (-db.x) * f.y) / det;
        b -= (da.x * f.y - da.y * f.x) / det;
        // A cutter may end a hair short of the outline (a piece left by
        // an earlier cut), so its parameter may run a little past its ends.
        if !(-0.02..=1.02).contains(&a) || !(-0.02..=1.02).contains(&b) {
            return t;
        }
    }
    let a = a.clamp(0.0, 1.0);
    let settled = (seg.eval(a) - other.eval(b.clamp(0.0, 1.0))).hypot() < 1e-6;
    if settled && (seg.eval(a) - seg.eval(t)).hypot() < 0.1 {
        a
    } else {
        t
    }
}

/// Where subpath `which` of `subs` crosses `cutters` and the rest of its
/// own object: parameters `u` = segment index + t, sorted, in `0..n`
/// (`n` the number of segments).
pub fn crossings(subs: &[Sub], which: usize, cutters: &[Cutter]) -> Vec<f64> {
    let Some(sub) = subs.get(which) else {
        return Vec::new();
    };
    let n = sub.segs.len();
    // The object's other subpaths cut too.
    let mut own: Vec<Cutter> = Vec::new();
    for (k, other) in subs.iter().enumerate() {
        if k != which {
            own.extend(other.segs.iter().copied().map(Cutter::new));
        }
    }
    let mine: Vec<Cutter> = sub.segs.iter().copied().map(Cutter::new).collect();
    let mut out = Vec::new();
    for (i, seg) in sub.segs.iter().enumerate() {
        let near = mine[i].bounds;
        for c in cutters.iter().chain(own.iter()) {
            if c.bounds.intersect(near).area() <= 0.0 {
                continue;
            }
            // A cutter ending on the outline (a piece left by an earlier
            // cut) must count: the crossing lies at its very end, where
            // rounding puts it a hair short as often as not, and the
            // intersection drops a cutter parameter past its ends.
            for l in &c.lines {
                for h in seg.intersect_line(stretched(*l)) {
                    if (-1e-9..=1.0 + 1e-9).contains(&h.line_t) {
                        out.push(i as f64 + refine(seg, h.segment_t.clamp(0.0, 1.0), &c.seg));
                    }
                }
            }
        }
        // Its own other segments, the next ones included: those cross it
        // anywhere but at the node they share (a curve looping back over
        // the segment before it, #314).
        for (j, other) in mine.iter().enumerate() {
            if j == i {
                continue;
            }
            let mut shared: Vec<Point> = Vec::new();
            if (i + 1) % n == j && (sub.closed || j == i + 1) {
                shared.push(seg.end());
            }
            if (j + 1) % n == i && (sub.closed || i == j + 1) {
                shared.push(seg.start());
            }
            for l in &other.lines {
                for h in seg.intersect_line(stretched(*l)) {
                    if !(-1e-9..=1.0 + 1e-9).contains(&h.line_t) {
                        continue;
                    }
                    let t = h.segment_t.clamp(0.0, 1.0);
                    let at = seg.eval(t);
                    if shared.iter().any(|node| (at - *node).hypot() < NODE_REACH) {
                        continue;
                    }
                    out.push(i as f64 + refine(seg, t, &other.seg));
                }
            }
        }
        // The segment crossing itself (a loop within one segment).
        for t in self_crossings(seg) {
            out.push(i as f64 + t);
        }
    }
    out.retain(|u| u.is_finite());
    out.sort_by(f64::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() < 1e-7);
    // On a closed subpath the end is the start.
    if sub.closed {
        for u in &mut out {
            if *u >= n as f64 - 1e-9 {
                *u = 0.0;
            }
        }
        out.sort_by(f64::total_cmp);
        out.dedup_by(|a, b| (*a - *b).abs() < 1e-7);
    }
    out
}

/// The stretch `a..b` of an open run of segments as a path.
fn stretch(segs: &[PathSeg], a: f64, b: f64, into: &mut BezPath) {
    let n = segs.len();
    let mut started = !into.elements().is_empty();
    let first = (a.floor().max(0.0) as usize).min(n.saturating_sub(1));
    for (i, seg) in segs.iter().enumerate().skip(first) {
        let (t0, t1) = ((a - i as f64).max(0.0), (b - i as f64).min(1.0));
        if t0 >= 1.0 {
            continue;
        }
        if t1 <= 0.0 {
            break;
        }
        if t1 - t0 < 1e-9 {
            continue;
        }
        let part = seg.subsegment(t0..t1);
        if !started {
            into.move_to(part.start());
            started = true;
        }
        match part {
            PathSeg::Line(l) => into.line_to(l.p1),
            PathSeg::Quad(q) => into.quad_to(q.p1, q.p2),
            PathSeg::Cubic(c) => into.curve_to(c.p1, c.p2, c.p3),
        }
    }
}

/// What is left of `sub` with the stretches `cut` removed (sorted,
/// in `0..n`; on a closed subpath a stretch with `a > b` wraps past the
/// start). `None` when the subpath is kept whole.
pub fn without(sub: &Sub, cut: &[(f64, f64)]) -> Option<Vec<BezPath>> {
    if cut.is_empty() {
        return None;
    }
    let n = sub.segs.len() as f64;
    // Removed stretches as plain ranges of 0..n.
    let mut removed: Vec<(f64, f64)> = Vec::new();
    for &(a, b) in cut {
        if a <= b {
            removed.push((a, b));
        } else {
            removed.push((a, n));
            removed.push((0.0, b));
        }
    }
    removed.sort_by(|x, y| x.0.total_cmp(&y.0));
    // What stays.
    let mut keep: Vec<(f64, f64)> = Vec::new();
    let mut at = 0.0;
    for (a, b) in removed {
        if a > at + 1e-9 {
            keep.push((at, a));
        }
        at = at.max(b);
    }
    if at < n - 1e-9 {
        keep.push((at, n));
    }
    // A closed subpath: the stretch over the start is one piece.
    if sub.closed && keep.len() > 1 {
        let first = keep[0];
        let last = keep[keep.len() - 1];
        if first.0 <= 1e-9 && last.1 >= n - 1e-9 {
            keep.pop();
            keep.remove(0);
            let mut p = BezPath::new();
            stretch(&sub.segs, last.0, n, &mut p);
            stretch(&sub.segs, 0.0, first.1, &mut p);
            let mut pieces = vec![p];
            for (a, b) in keep {
                let mut q = BezPath::new();
                stretch(&sub.segs, a, b, &mut q);
                pieces.push(q);
            }
            pieces.retain(|p| p.elements().len() > 1);
            return Some(pieces);
        }
    }
    let mut pieces = Vec::new();
    for (a, b) in keep {
        let mut q = BezPath::new();
        stretch(&sub.segs, a, b, &mut q);
        pieces.push(q);
    }
    pieces.retain(|p| p.elements().len() > 1);
    Some(pieces)
}

/// The virtual segment of `sub` holding parameter `u`, given its
/// crossings: `(a, b)` (wrapping when `a > b`); the whole subpath when a
/// closed one has fewer than two crossings.
pub fn around(sub: &Sub, bounds: &[f64], u: f64) -> (f64, f64) {
    let n = sub.segs.len() as f64;
    if sub.closed {
        if bounds.len() < 2 {
            return (0.0, n);
        }
        let lo = bounds
            .iter()
            .rev()
            .find(|b| **b <= u)
            .or(bounds.last())
            .copied()
            .unwrap_or(0.0);
        let hi = bounds
            .iter()
            .find(|b| **b > u)
            .or(bounds.first())
            .copied()
            .unwrap_or(n);
        (lo, hi)
    } else {
        let lo = bounds
            .iter()
            .rev()
            .find(|b| **b <= u)
            .copied()
            .unwrap_or(0.0);
        let hi = bounds.iter().find(|b| **b > u).copied().unwrap_or(n);
        (lo.max(0.0), hi.min(n))
    }
}

/// Every virtual segment of `sub`: consecutive crossings (and the open
/// ends).
pub fn pieces(sub: &Sub, bounds: &[f64]) -> Vec<(f64, f64)> {
    let n = sub.segs.len() as f64;
    if sub.closed {
        if bounds.len() < 2 {
            return vec![(0.0, n)];
        }
        let mut out: Vec<(f64, f64)> = bounds.windows(2).map(|w| (w[0], w[1])).collect();
        out.push((bounds[bounds.len() - 1], bounds[0]));
        out
    } else {
        let mut b = vec![0.0];
        b.extend(
            bounds
                .iter()
                .copied()
                .filter(|u| *u > 1e-9 && *u < n - 1e-9),
        );
        b.push(n);
        b.windows(2).map(|w| (w[0], w[1])).collect()
    }
}

/// `p` inside `r`, its edges included (a rectangle of no height or width
/// is a line).
fn within(r: Rect, p: Point) -> bool {
    p.x >= r.x0 && p.x <= r.x1 && p.y >= r.y0 && p.y <= r.y1
}

/// Some part of the curve `seg` lies inside `r` (edges included): an end
/// inside, or the curve crossing one of its edges.
fn seg_touches(seg: &PathSeg, r: Rect) -> bool {
    let b = seg.bounding_box();
    if b.x0 > r.x1 || b.x1 < r.x0 || b.y0 > r.y1 || b.y1 < r.y0 {
        return false;
    }
    if within(r, seg.start()) || within(r, seg.end()) {
        return true;
    }
    let (a, c) = (Point::new(r.x0, r.y0), Point::new(r.x1, r.y1));
    let (b2, d) = (Point::new(r.x1, r.y0), Point::new(r.x0, r.y1));
    // A curve through a corner, or lying along a side of a box of no
    // height or width, which the crossing test misses (parallel lines).
    if [a, b2, c, d]
        .iter()
        .any(|k| seg.nearest(*k, 1e-9).distance_sq <= 1e-12)
    {
        return true;
    }
    [
        Line::new(a, b2),
        Line::new(b2, c),
        Line::new(c, d),
        Line::new(d, a),
    ]
    .into_iter()
    .filter(|l| l.p0 != l.p1)
    .any(|l| !seg.intersect_line(l).is_empty())
}

/// The virtual segment `a..b` of `sub` (wrapping when `a > b`) has some
/// part inside `r`: a drag takes it whole (#528).
pub fn stretch_touches(sub: &Sub, a: f64, b: f64, r: Rect) -> bool {
    let n = sub.segs.len() as f64;
    let ranges = if a <= b {
        vec![(a, b)]
    } else {
        vec![(a, n), (0.0, b)]
    };
    ranges.into_iter().any(|(a, b)| {
        sub.segs.iter().enumerate().any(|(i, seg)| {
            let (t0, t1) = ((a - i as f64).max(0.0), (b - i as f64).min(1.0));
            t1 - t0 > 1e-9 && seg_touches(&seg.subsegment(t0..t1), r)
        })
    })
}

/// The curve `seg` crosses or touches one of the stroke's `lines`.
fn seg_crosses(seg: &PathSeg, lines: &[Line], stroke_box: Rect) -> bool {
    let b = seg.bounding_box();
    if b.x0 > stroke_box.x1 || b.x1 < stroke_box.x0 || b.y0 > stroke_box.y1 || b.y1 < stroke_box.y0
    {
        return false;
    }
    lines.iter().any(|l| {
        let lb = Rect::from_points(l.p0, l.p1);
        if b.x0 > lb.x1 || b.x1 < lb.x0 || b.y0 > lb.y1 || b.y1 < lb.y0 {
            return false;
        }
        if l.p0 == l.p1 {
            return seg.nearest(l.p0, 1e-9).distance_sq <= 1e-12;
        }
        !seg.intersect_line(*l).is_empty()
    })
}

/// The virtual segment `a..b` of `sub` (wrapping when `a > b`) is crossed
/// by the freehand stroke `lines` (its box `stroke_box`): the stroke takes
/// it whole (#1150).
pub fn stretch_crossed(sub: &Sub, a: f64, b: f64, lines: &[Line], stroke_box: Rect) -> bool {
    let n = sub.segs.len() as f64;
    let ranges = if a <= b {
        vec![(a, b)]
    } else {
        vec![(a, n), (0.0, b)]
    };
    ranges.into_iter().any(|(a, b)| {
        sub.segs.iter().enumerate().any(|(i, seg)| {
            let (t0, t1) = ((a - i as f64).max(0.0), (b - i as f64).min(1.0));
            t1 - t0 > 1e-9 && seg_crosses(&seg.subsegment(t0..t1), lines, stroke_box)
        })
    })
}

/// The lines of a freehand stroke through `points` (one point: a line of
/// no length there).
pub fn stroke_lines(points: &[Point]) -> Vec<Line> {
    match points {
        [] => Vec::new(),
        [p] => vec![Line::new(*p, *p)],
        _ => points.windows(2).map(|w| Line::new(w[0], w[1])).collect(),
    }
}

fn cuttable(s: &Shape) -> bool {
    matches!(
        s.kind,
        ShapeKind::Path { .. }
            | ShapeKind::Rect { .. }
            | ShapeKind::Ellipse { .. }
            | ShapeKind::Polygon { .. }
    )
}

/// An object's outline on the page and its box, worked out once for a
/// click or a drag.
struct Outline {
    id: ShapeId,
    path: BezPath,
    bounds: Rect,
}

fn outlines_of(all: &[Shape]) -> Vec<Outline> {
    all.iter()
        .map(|s| {
            let path = s.page_path();
            let bounds = path.bounding_box();
            Outline {
                id: s.id,
                path,
                bounds,
            }
        })
        .collect()
}

impl App {
    /// The visible objects of the page's unlocked layers.
    fn segment_targets(&self) -> Vec<Shape> {
        self.doc()
            .page(self.page)
            .map(|pg| {
                pg.layers
                    .iter()
                    .filter(|l| l.visible && !l.locked)
                    .flat_map(|l| l.shapes.iter().filter(|s| s.visible && !s.locked).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The targets with every group opened (not linked groups such as
    /// text or clip frames): the objects inside take their place, their
    /// transforms composed with the group's, so a curve in a group is cut
    /// as one on its own (#427).
    fn segment_leaves(&self) -> Vec<Shape> {
        fn open(s: Shape, out: &mut Vec<Shape>) {
            match &s.kind {
                ShapeKind::Group { children } => {
                    for c in children.iter().filter(|c| c.visible && !c.locked) {
                        let mut c = c.clone();
                        c.transform = s.transform * c.transform;
                        open(c, out);
                    }
                }
                _ => out.push(s),
            }
        }
        let mut out = Vec::new();
        for s in self.segment_targets() {
            open(s, &mut out);
        }
        out
    }

    /// The outlines of the objects other than `id` that can cross
    /// `reach` (page space): only those whose box meets it, so a click or
    /// a drag on a crowded page flattens the neighbours, not every object.
    fn cutters_near(outlines: &[Outline], id: ShapeId, reach: Rect) -> Vec<Cutter> {
        let reach = reach.inflate(0.1, 0.1);
        let mut out = Vec::new();
        for o in outlines {
            let b = o.bounds;
            if o.id == id
                || b.x0 > reach.x1
                || b.x1 < reach.x0
                || b.y0 > reach.y1
                || b.y1 < reach.y0
            {
                continue;
            }
            out.extend(cutters_of(&o.path));
        }
        out
    }

    /// The command that leaves `s` (page path `subs`) with the stretches
    /// `cut[i]` removed from subpath `i`.
    fn segment_delete_command(
        &self,
        s: &Shape,
        subs: &[Sub],
        cut: &[Vec<(f64, f64)>],
    ) -> Option<Command> {
        let mut path = BezPath::new();
        let mut any_closed = false;
        let mut changed = false;
        for (sub, c) in subs.iter().zip(cut) {
            match without(sub, c) {
                None => {
                    for (k, seg) in sub.segs.iter().enumerate() {
                        if k == 0 {
                            path.move_to(seg.start());
                        }
                        match *seg {
                            PathSeg::Line(l) => path.line_to(l.p1),
                            PathSeg::Quad(q) => path.quad_to(q.p1, q.p2),
                            PathSeg::Cubic(cb) => path.curve_to(cb.p1, cb.p2, cb.p3),
                        }
                    }
                    if sub.closed {
                        path.close_path();
                        any_closed = true;
                    }
                }
                Some(pieces) => {
                    changed = true;
                    for p in pieces {
                        path.extend(p.elements().iter().copied());
                    }
                }
            }
        }
        if !changed {
            return None;
        }
        if path.elements().is_empty() {
            return Some(Command::DeleteShapes { shapes: vec![s.id] });
        }
        if s.transform.determinant().abs() < 1e-12 {
            return None;
        }
        Some(Command::SetShapeKind {
            shape: s.id,
            kind: ShapeKind::Path {
                path: s.transform.inverse() * path,
                closed: any_closed,
            },
        })
    }

    /// Remove the virtual segment under `p` (page space). False when no
    /// outline is near.
    pub fn delete_virtual_segment_at(&mut self, p: Point) -> bool {
        let tol = 5.0 / self.view.zoom.max(1e-6) as f64;
        let all = self.segment_leaves();
        // The nearest outline within reach (the topmost of equals).
        let mut found: Option<(Shape, Vec<Sub>, usize, f64, f64)> = None;
        for s in all.iter().rev().filter(|s| cuttable(s)) {
            if !s.bounds().inflate(tol, tol).contains(p) {
                continue;
            }
            let subs = subs_of(&s.page_path());
            let mut best: Option<(usize, f64, f64)> = None;
            for (k, sub) in subs.iter().enumerate() {
                for (i, seg) in sub.segs.iter().enumerate() {
                    let n = seg.nearest(p, 1e-6);
                    let d = n.distance_sq.sqrt();
                    if d <= tol && best.is_none_or(|b| d < b.2) {
                        best = Some((k, i as f64 + n.t, d));
                    }
                }
            }
            if let Some((k, u, d)) = best {
                if found.as_ref().is_none_or(|f| d < f.4 - 1e-9) {
                    found = Some((s.clone(), subs, k, u, d));
                }
            }
        }
        let Some((s, subs, k, u, _)) = found else {
            return false;
        };
        let reach = s.page_path().bounding_box();
        let cutters = Self::cutters_near(&outlines_of(&all), s.id, reach);
        let bounds = crossings(&subs, k, &cutters);
        let mut cut = vec![Vec::new(); subs.len()];
        cut[k].push(around(&subs[k], &bounds, u));
        if let Some(cmd) = self.segment_delete_command(&s, &subs, &cut) {
            if let Err(e) = self.engine.run_batch("Delete Segment", &[cmd]) {
                self.status = e.to_string();
            }
            self.selection
                .retain(|id| self.engine.document().find_shape(*id).is_some());
        }
        true
    }

    /// Remove every virtual segment the freehand stroke through `points`
    /// (page space) crosses, whole, as far as the crossings at its ends,
    /// on all objects at once (#1150). True when something was removed.
    pub fn delete_virtual_segments_along(&mut self, points: &[Point]) -> bool {
        let points: Vec<Point> = points
            .iter()
            .copied()
            .filter(|p| p.x.is_finite() && p.y.is_finite())
            .collect();
        let lines = stroke_lines(&points);
        let Some(stroke_box) = points
            .iter()
            .map(|p| Rect::from_points(*p, *p))
            .reduce(|a, b| a.union(b))
        else {
            return false;
        };
        let all = self.segment_leaves();
        let outlines = outlines_of(&all);
        let mut cmds = Vec::new();
        for (s, o) in all.iter().zip(&outlines).filter(|(s, _)| cuttable(s)) {
            let b = o.bounds;
            if b.x0 > stroke_box.x1
                || b.x1 < stroke_box.x0
                || b.y0 > stroke_box.y1
                || b.y1 < stroke_box.y0
            {
                continue;
            }
            let subs = subs_of(&o.path);
            let reached: Vec<bool> = subs
                .iter()
                .map(|sub| {
                    sub.segs
                        .iter()
                        .any(|seg| seg_crosses(seg, &lines, stroke_box))
                })
                .collect();
            if !reached.contains(&true) {
                continue;
            }
            let cutters = Self::cutters_near(&outlines, s.id, b);
            let mut cut = vec![Vec::new(); subs.len()];
            for (k, sub) in subs.iter().enumerate() {
                if !reached[k] {
                    continue;
                }
                let bounds = crossings(&subs, k, &cutters);
                for (a, b) in pieces(sub, &bounds) {
                    if stretch_crossed(sub, a, b, &lines, stroke_box) {
                        cut[k].push((a, b));
                    }
                }
            }
            if let Some(cmd) = self.segment_delete_command(s, &subs, &cut) {
                cmds.push(cmd);
            }
        }
        if cmds.is_empty() {
            return false;
        }
        if let Err(e) = self.engine.run_batch("Delete Segment", &cmds) {
            self.status = e.to_string();
            return false;
        }
        self.selection
            .retain(|id| self.engine.document().find_shape(*id).is_some());
        true
    }

    /// Remove every virtual segment `rect` touches (page space), whole:
    /// a stretch with any part inside the rectangle goes, as far as the
    /// crossings at its ends, and a closed outline nothing crosses goes
    /// whole (#528). True when something was removed.
    pub fn delete_virtual_segments_in(&mut self, rect: Rect) -> bool {
        if !(rect.width().is_finite() && rect.height().is_finite()) {
            return false;
        }
        let all = self.segment_leaves();
        let outlines = outlines_of(&all);
        let mut cmds = Vec::new();
        for (s, o) in all.iter().zip(&outlines).filter(|(s, _)| cuttable(s)) {
            // Overlap, not area: a level or upright line has a box of no
            // height or width.
            let b = o.bounds;
            if b.x0 > rect.x1 || b.x1 < rect.x0 || b.y0 > rect.y1 || b.y1 < rect.y0 {
                continue;
            }
            let subs = subs_of(&o.path);
            // Only subpaths the rectangle reaches need their crossings.
            let reached: Vec<bool> = subs
                .iter()
                .map(|sub| sub.segs.iter().any(|seg| seg_touches(seg, rect)))
                .collect();
            if !reached.contains(&true) {
                continue;
            }
            let cutters = Self::cutters_near(&outlines, s.id, b);
            let mut cut = vec![Vec::new(); subs.len()];
            for (k, sub) in subs.iter().enumerate() {
                if !reached[k] {
                    continue;
                }
                let bounds = crossings(&subs, k, &cutters);
                for (a, b) in pieces(sub, &bounds) {
                    if stretch_touches(sub, a, b, rect) {
                        cut[k].push((a, b));
                    }
                }
            }
            if let Some(cmd) = self.segment_delete_command(s, &subs, &cut) {
                cmds.push(cmd);
            }
        }
        if cmds.is_empty() {
            return false;
        }
        if let Err(e) = self.engine.run_batch("Delete Segment", &cmds) {
            self.status = e.to_string();
            return false;
        }
        self.selection
            .retain(|id| self.engine.document().find_shape(*id).is_some());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(app: &mut App, a: (f64, f64), b: (f64, f64)) -> ShapeId {
        let mut p = BezPath::new();
        p.move_to(a);
        p.line_to(b);
        app.new_shape(ShapeKind::Path {
            path: p,
            closed: false,
        })
        .expect("line")
    }

    fn path_of(app: &App, id: ShapeId) -> BezPath {
        app.doc().find_shape(id).expect("object").page_path()
    }

    #[test]
    fn a_click_removes_the_stretch_between_crossings() {
        let mut app = App::headless();
        // A horizontal line crossed by two vertical ones.
        let h = line(&mut app, (0.0, 0.0), (30.0, 0.0));
        line(&mut app, (10.0, -5.0), (10.0, 5.0));
        line(&mut app, (20.0, -5.0), (20.0, 5.0));
        assert!(app.delete_virtual_segment_at(Point::new(15.0, 0.0)));
        let p = path_of(&app, h);
        let moves = p
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count();
        assert_eq!(moves, 2, "{p:?}");
        let b = p.bounding_box();
        assert!((b.x0 - 0.0).abs() < 1e-6 && (b.x1 - 30.0).abs() < 1e-6);
        // Nothing between 10 and 20 is left.
        assert!(p
            .segments()
            .all(|s| s.eval(0.5).x < 10.0 + 1e-6 || s.eval(0.5).x > 20.0 - 1e-6));
        // An end stretch: from the open end to the first crossing.
        assert!(app.delete_virtual_segment_at(Point::new(25.0, 0.0)));
        let b = path_of(&app, h).bounding_box();
        assert!((b.x1 - 10.0).abs() < 1e-6, "{b:?}");
        // The last stretch: the object goes.
        assert!(app.delete_virtual_segment_at(Point::new(5.0, 0.0)));
        assert!(app.doc().find_shape(h).is_none());
        // Far from any outline: nothing.
        assert!(!app.delete_virtual_segment_at(Point::new(500.0, 500.0)));
    }

    /// Two lines crossing: a stretch of one goes, then a stretch of the
    /// other, which now crosses two pieces ending exactly on it. The
    /// crossing must still be found (#227: the whole second line went).
    #[test]
    fn a_crossing_at_the_end_of_a_cut_piece_still_counts() {
        let mut app = App::headless();
        let a = line(&mut app, (0.0, 0.0), (30.0, 30.0));
        let b = line(&mut app, (0.0, 30.0), (30.0, 0.0));
        // The upper-left arm of a, up to the crossing at (15, 15).
        assert!(app.delete_virtual_segment_at(Point::new(5.0, 5.0)));
        let pa = path_of(&app, a);
        assert!((pa.bounding_box().x0 - 15.0).abs() < 1e-6, "{pa:?}");
        // The lower-left arm of b: only up to the crossing.
        assert!(app.delete_virtual_segment_at(Point::new(5.0, 25.0)));
        assert!(app.doc().find_shape(b).is_some(), "the whole of b went");
        let pb = path_of(&app, b);
        let bb = pb.bounding_box();
        assert!(
            (bb.x0 - 15.0).abs() < 1e-6 && (bb.x1 - 30.0).abs() < 1e-6,
            "{pb:?}"
        );
    }

    /// The same with lines drawn by hand (curves, as the Freehand tool
    /// makes them): the crossing lies at the very end of the cut piece.
    #[test]
    fn a_crossing_at_the_end_of_a_cut_curve_piece_still_counts() {
        let mut app = App::headless();
        let curve = |app: &mut App, a: (f64, f64), b: (f64, f64)| -> ShapeId {
            let mut p = BezPath::new();
            p.move_to(a);
            let (c1, c2) = (
                (a.0 + (b.0 - a.0) / 3.0 + 0.3, a.1 + (b.1 - a.1) / 3.0 - 0.2),
                (
                    a.0 + (b.0 - a.0) * 2.0 / 3.0 - 0.1,
                    a.1 + (b.1 - a.1) * 2.0 / 3.0 + 0.4,
                ),
            );
            p.curve_to(c1, c2, b);
            app.new_shape(ShapeKind::Path {
                path: p,
                closed: false,
            })
            .expect("curve")
        };
        let a = curve(&mut app, (11.3, 7.1), (87.9, 83.2));
        let b = curve(&mut app, (9.2, 84.6), (88.1, 6.4));
        assert!(app.delete_virtual_segment_at(Point::new(16.0, 11.8)));
        assert!(app.doc().find_shape(a).is_some(), "the whole of a went");
        assert!(app.delete_virtual_segment_at(Point::new(14.0, 80.0)));
        assert!(app.doc().find_shape(b).is_some(), "the whole of b went");
        let bb = path_of(&app, b).bounding_box();
        assert!(bb.x0 > 40.0 && bb.x1 > 87.0, "{bb:?}");
    }

    /// A curve crossing itself: a loop inside one cubic segment, and the
    /// next segment looping back over the one before it. Only the
    /// stretch between the crossings goes, not the whole curve (#314).
    #[test]
    fn a_curve_crossing_itself_loses_only_the_stretch_clicked() {
        // One cubic with a loop: from (0, 0) up and over itself to (30, 0).
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.curve_to((40.0, 30.0), (-10.0, 30.0), (30.0, 0.0));
        let subs = subs_of(&p);
        let seg = subs[0].segs[0];
        let ts = self_crossings(&seg);
        assert_eq!(ts.len(), 2, "{ts:?}");
        let (a, b) = (seg.eval(ts[0]), seg.eval(ts[1]));
        assert!((a - b).hypot() < 1e-6, "one point: {a:?} {b:?}");
        assert_eq!(crossings(&subs, 0, &[]).len(), 2);
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Path {
                path: p.clone(),
                closed: false,
            })
            .expect("curve");
        // The loop's top is the stretch between the crossings.
        let top = seg.eval(0.5);
        assert!(app.delete_virtual_segment_at(top));
        let left = path_of(&app, id);
        assert!(app.doc().find_shape(id).is_some(), "the whole curve went");
        let pieces = left
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count();
        assert_eq!(pieces, 2, "{left:?}");
        assert!(left.bounding_box().y1 < top.y - 1.0, "{left:?}");
        assert!(left
            .segments()
            .any(|s| (s.start() - Point::new(0.0, 0.0)).hypot() < 1e-6));
        assert!(left
            .segments()
            .any(|s| (s.end() - Point::new(30.0, 0.0)).hypot() < 1e-6));

        // A straight segment, then a curve that comes back over it.
        let mut q = BezPath::new();
        q.move_to((0.0, 0.0));
        q.line_to((20.0, 0.0));
        q.curve_to((20.0, 15.0), (5.0, 15.0), (10.0, -5.0));
        let subs = subs_of(&q);
        let c = crossings(&subs, 0, &[]);
        assert_eq!(c.len(), 2, "{c:?}");
        assert!(c[0] > 0.3 && c[0] < 0.7, "on the line's middle: {c:?}");
        assert!(c[1] > 1.5 && c[1] < 2.0, "late in the curve: {c:?}");
        // The node they share is no crossing.
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Path {
                path: q,
                closed: false,
            })
            .expect("curve");
        // The tail past the crossing goes; the line and the arc stay.
        assert!(app.delete_virtual_segment_at(Point::new(10.0, -4.0)));
        let left = path_of(&app, id);
        let b = left.bounding_box();
        assert!(
            b.y0 > -0.5 && b.y1 > 8.0 && (b.x1 - 20.0).abs() < 1e-6,
            "{left:?}"
        );
        assert!(left
            .segments()
            .any(|s| (s.start() - Point::new(0.0, 0.0)).hypot() < 1e-6));
    }

    /// A curve of several segments whose tail comes back across an earlier
    /// segment: the stretch from the start to the crossing goes, then the
    /// tail past the crossing, and the loop between stays (#354: the whole
    /// curve went, as the first cut ended a little off the tail's true
    /// outline and the second found no crossing).
    #[test]
    fn a_tail_crossing_a_cut_curve_loses_only_itself() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.curve_to((15.0, 15.0), (35.0, 25.0), (45.0, 15.0));
        p.curve_to((55.0, 5.0), (60.0, -15.0), (75.0, -10.0));
        p.curve_to((90.0, -5.0), (80.0, 20.0), (65.0, 15.0));
        p.curve_to((55.0, 12.0), (50.0, 0.0), (40.0, -10.0));
        let subs = subs_of(&p);
        let c = crossings(&subs, 0, &[]);
        assert_eq!(c.len(), 2, "{c:?}");
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Path {
                path: p.clone(),
                closed: false,
            })
            .expect("curve");
        assert!(app.delete_virtual_segment_at(subs[0].segs[0].eval(0.3)));
        let x = subs[0].segs[1].eval(c[0] - 1.0);
        assert!(app.delete_virtual_segment_at(Point::new(40.5, -9.4)));
        assert!(app.doc().find_shape(id).is_some(), "the whole curve went");
        let left = path_of(&app, id);
        let b = left.bounding_box();
        // The loop: from the crossing round to the crossing again.
        assert!(b.x0 > x.x - 0.01 && b.x1 > 80.0, "{left:?}");
        assert!(b.y0 > -12.0 && b.y1 > 15.0, "{left:?}");
        let ends: Vec<Point> = left.segments().map(|s| s.end()).collect();
        let last = *ends.last().expect("segments");
        assert!((last - x).hypot() < 1e-6, "{last:?} {x:?}");
    }

    /// Curves inside a group (an imported ornament is one) are cut as
    /// curves on their own, by a click and by a drag, the group moved
    /// first so its transform counts (#427: nothing happened).
    #[test]
    fn curves_inside_a_group_are_cut() {
        let mut app = App::headless();
        let h = line(&mut app, (0.0, 0.0), (30.0, 0.0));
        let v1 = line(&mut app, (10.0, -5.0), (10.0, 5.0));
        let v2 = line(&mut app, (20.0, -5.0), (20.0, 5.0));
        app.select(vec![h, v1, v2]);
        app.group_selection();
        let g = app.selection[0];
        app.run(Command::TransformShapes {
            shapes: vec![g],
            transform: tracedraw_core::geometry::Affine::translate((100.0, 50.0)),
        });
        // The line in page space: the group's move applied.
        let page = |app: &App| {
            let gt = app.doc().find_shape(g).expect("group").transform;
            gt * path_of(app, h)
        };
        // A click between the crossings, where the group now is.
        assert!(app.delete_virtual_segment_at(Point::new(115.0, 50.0)));
        let p = page(&app);
        assert_eq!(
            p.elements()
                .iter()
                .filter(|e| matches!(e, PathEl::MoveTo(_)))
                .count(),
            2,
            "{p:?}"
        );
        assert!(
            p.segments()
                .all(|s| s.eval(0.5).x < 110.0 + 1e-6 || s.eval(0.5).x > 120.0 - 1e-6),
            "{p:?}"
        );
        let b = p.bounding_box();
        assert!(
            (b.x0 - 100.0).abs() < 1e-6 && (b.x1 - 130.0).abs() < 1e-6,
            "{b:?}"
        );
        // A drag round the right end of the line: that stretch goes, and
        // the vertical through it, both of whose halves the box touches.
        app.delete_virtual_segments_in(Rect::new(119.0, 48.0, 131.0, 52.0));
        let b = page(&app).bounding_box();
        assert!((b.x1 - 110.0).abs() < 1e-6, "{b:?}");
        assert!(
            app.doc().find_shape(v2).is_none(),
            "the vertical it touched"
        );
        assert!(app.doc().find_shape(v1).is_some());
        assert!(matches!(
            app.doc().find_shape(g).map(|s| &s.kind),
            Some(ShapeKind::Group { .. })
        ));
    }

    /// A cutter that stops a hair short of the outline (as a piece left by
    /// an earlier cut does, by rounding) still makes a crossing; one that
    /// stops well short does not.
    #[test]
    fn a_cutter_ending_a_hair_short_of_the_outline_still_cuts() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((30.0, 30.0));
        let subs = subs_of(&p);
        let short = |gap: f64| {
            vec![Cutter::new(PathSeg::Line(Line::new(
                (0.0, 30.0),
                (15.0 - gap, 15.0 + gap),
            )))]
        };
        assert_eq!(crossings(&subs, 0, &short(1e-7)).len(), 1, "a hair short");
        assert_eq!(crossings(&subs, 0, &short(-1e-7)).len(), 1, "a hair past");
        assert_eq!(crossings(&subs, 0, &short(0.0)).len(), 1, "exactly on it");
        assert!(crossings(&subs, 0, &short(0.5)).is_empty(), "well short");
    }

    #[test]
    fn closed_outlines_open_where_they_lose_a_stretch() {
        let mut app = App::headless();
        let sq = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("square");
        // A line across the square's bottom side at x = 5 and x = 15.
        line(&mut app, (5.0, -5.0), (5.0, 5.0));
        line(&mut app, (15.0, -5.0), (15.0, 5.0));
        assert!(app.delete_virtual_segment_at(Point::new(10.0, 0.0)));
        let s = app.doc().find_shape(sq).expect("square");
        let ShapeKind::Path { closed, .. } = &s.kind else {
            panic!("a curve");
        };
        assert!(!closed);
        let p = s.page_path();
        // One open piece, from (15, 0) round the top to (5, 0).
        let moves = p
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count();
        assert_eq!(moves, 1);
        assert!(p.segments().all(|g| !(g.eval(0.5).y.abs() < 1e-9
            && g.eval(0.5).x > 5.0 + 1e-6
            && g.eval(0.5).x < 15.0 - 1e-6)));
        // An ellipse with nothing crossing it goes whole.
        let e = app
            .new_shape(ShapeKind::Ellipse {
                rect: Rect::new(100.0, 0.0, 120.0, 10.0),
                arc: None,
            })
            .expect("ellipse");
        assert!(app.delete_virtual_segment_at(Point::new(120.0, 5.0)));
        assert!(app.doc().find_shape(e).is_none());
    }

    fn closed(app: &mut App, pts: &[(f64, f64)]) -> ShapeId {
        let mut p = BezPath::new();
        p.move_to(pts[0]);
        for q in &pts[1..] {
            p.line_to(*q);
        }
        p.close_path();
        app.new_shape(ShapeKind::Path {
            path: p,
            closed: true,
        })
        .expect("closed")
    }

    /// A drag takes every virtual segment its rectangle touches, whole,
    /// as far as the crossings at its ends (#528): a stretch reaching
    /// into the rectangle goes as well as one lying inside it, a closed
    /// outline nothing crosses goes whole even where it reaches past the
    /// rectangle, and what the rectangle does not reach stays.
    #[test]
    fn a_drag_removes_every_stretch_it_touches() {
        let mut app = App::headless();
        // A level line crossed by a band and by a ring around its middle.
        let h = line(&mut app, (0.0, 0.0), (100.0, 0.0));
        let band = closed(
            &mut app,
            &[(20.0, -50.0), (80.0, -50.0), (80.0, 50.0), (20.0, 50.0)],
        );
        let ring = app
            .new_shape(ShapeKind::Ellipse {
                rect: Rect::new(45.0, -5.0, 55.0, 5.0),
                arc: None,
            })
            .expect("ring");
        // Petals above the line: one inside the rectangle, one reaching
        // past its right side; a shape above it out of reach.
        let inside = closed(
            &mut app,
            &[(30.0, 10.0), (40.0, 10.0), (40.0, 20.0), (30.0, 20.0)],
        );
        let past = closed(
            &mut app,
            &[(60.0, 10.0), (75.0, 10.0), (75.0, 20.0), (60.0, 20.0)],
        );
        let above = closed(
            &mut app,
            &[(30.0, 40.0), (40.0, 40.0), (40.0, 45.0), (30.0, 45.0)],
        );
        let steps = app.engine.history_labels().0.len();
        assert!(app.delete_virtual_segments_in(Rect::new(25.0, 2.0, 70.0, 30.0)));
        assert_eq!(
            app.engine.history_labels().0.len(),
            steps + 1,
            "one undo step"
        );
        assert!(app.doc().find_shape(inside).is_none());
        assert!(
            app.doc().find_shape(past).is_none(),
            "touched, it goes whole"
        );
        // The ring loses its upper half (it reaches 3 mm into the
        // rectangle) between its crossings with the line, and keeps the
        // lower one.
        let r = path_of(&app, ring).bounding_box();
        assert!(r.y1 < 1e-6 && (r.y0 + 5.0).abs() < 1e-6, "{r:?}");
        assert!(
            (r.x0 - 45.0).abs() < 1e-6 && (r.x1 - 55.0).abs() < 1e-6,
            "{r:?}"
        );
        // Out of reach: the band, the line and the shape above.
        for id in [band, h, above] {
            let s = app.doc().find_shape(id).expect("kept");
            assert!(matches!(&s.kind, ShapeKind::Path { closed, .. } if *closed == (id != h)));
        }
        assert_eq!(
            path_of(&app, h)
                .elements()
                .iter()
                .filter(|e| matches!(e, PathEl::MoveTo(_)))
                .count(),
            1
        );
        // A rectangle that reaches nothing removes nothing.
        let steps = app.engine.history_labels().0.len();
        assert!(!app.delete_virtual_segments_in(Rect::new(85.0, 20.0, 95.0, 30.0)));
        assert_eq!(app.engine.history_labels().0.len(), steps);
    }

    /// A rectangle between two crossings takes only that stretch; a level
    /// drag (a rectangle of no height) takes what it crosses.
    #[test]
    fn a_drag_between_crossings_takes_only_that_stretch() {
        let mut app = App::headless();
        let h = line(&mut app, (0.0, 0.0), (30.0, 0.0));
        let v1 = line(&mut app, (10.0, -5.0), (10.0, 5.0));
        let v2 = line(&mut app, (20.0, -5.0), (20.0, 5.0));
        assert!(app.delete_virtual_segments_in(Rect::new(12.0, -1.0, 18.0, 1.0)));
        let p = path_of(&app, h);
        assert_eq!(
            p.elements()
                .iter()
                .filter(|e| matches!(e, PathEl::MoveTo(_)))
                .count(),
            2
        );
        assert!(p
            .segments()
            .all(|s| s.eval(0.5).x < 10.0 + 1e-6 || s.eval(0.5).x > 20.0 - 1e-6));
        for v in [v1, v2] {
            let b = path_of(&app, v).bounding_box();
            assert!(
                (b.y0 + 5.0).abs() < 1e-6 && (b.y1 - 5.0).abs() < 1e-6,
                "{b:?}"
            );
        }
        // Level across the upper halves of both verticals.
        assert!(app.delete_virtual_segments_in(Rect::new(5.0, 3.0, 25.0, 3.0)));
        for v in [v1, v2] {
            let b = path_of(&app, v).bounding_box();
            assert!((b.y0 + 5.0).abs() < 1e-6 && b.y1.abs() < 1e-6, "{b:?}");
        }
    }

    /// A click whose pointer slipped a few pixels makes a little box; when
    /// it reaches no outline, the outline near the press goes as a click
    /// would take it.
    #[test]
    fn a_click_that_slips_into_a_little_drag_still_deletes() {
        let mut app = App::headless();
        let h = line(&mut app, (0.0, 0.0), (30.0, 0.0));
        line(&mut app, (10.0, -5.0), (10.0, 5.0));
        app.tool = crate::tools::Tool::SegmentDelete;
        let px = 1.0 / app.view.zoom as f64;
        // Pressed 3 px off the line, slipped 7 px away from it (past the
        // distance that makes a drag) and sideways: the press is what
        // counts, whichever way the pointer went (y goes up on the page).
        for (press, slip) in [((20.0, -3.0), (7.0, -7.0)), ((5.0, 3.0), (-7.0, 7.0))] {
            let start = Point::new(press.0, press.1 * px);
            let before = app.engine.history_labels().0.len();
            app.drag = crate::app::Drag::Box {
                start,
                current: start + (slip.0 * px, slip.1 * px),
                from_center: false,
            };
            app.end_drag();
            assert_eq!(app.engine.history_labels().0.len(), before + 1, "{press:?}");
        }
        // The stretch right of the crossing went, then the one left of it.
        assert!(app.doc().find_shape(h).is_none());
        // A real drag that reaches nothing removes nothing.
        let before = app.engine.history_labels().0.len();
        let start = Point::new(40.0, 10.0);
        app.finish_tools2_box(start, start + (60.0 * px, 60.0 * px));
        assert_eq!(app.engine.history_labels().0.len(), before);
    }

    /// A freehand stroke removes every stretch it crosses, whole, and
    /// nothing it passes beside (#1150).
    #[test]
    fn a_freehand_stroke_removes_the_stretches_it_crosses() {
        let mut app = App::headless();
        let h = line(&mut app, (0.0, 0.0), (30.0, 0.0));
        let v1 = line(&mut app, (10.0, -5.0), (10.0, 5.0));
        let v2 = line(&mut app, (20.0, -5.0), (20.0, 5.0));
        let before = app.engine.history_labels().0.len();
        // Down across the middle of h only: one stretch, one undo step.
        assert!(app.delete_virtual_segments_along(&[
            Point::new(15.0, 3.0),
            Point::new(15.5, 0.5),
            Point::new(15.0, -3.0),
        ]));
        assert_eq!(app.engine.history_labels().0.len(), before + 1);
        let p = path_of(&app, h);
        assert!(p
            .segments()
            .all(|s| s.eval(0.5).x < 10.0 + 1e-6 || s.eval(0.5).x > 20.0 - 1e-6));
        assert_eq!(path_of(&app, h).bounding_box().x1, 30.0);
        // Across the upper arms of both uprights at y = 3: each loses its
        // arm above h, as far as the crossing.
        assert!(app.delete_virtual_segments_along(&[Point::new(5.0, 3.0), Point::new(25.0, 3.0)]));
        for v in [v1, v2] {
            let b = path_of(&app, v).bounding_box();
            assert!(
                (b.y1 - 0.0).abs() < 1e-6 && (b.y0 + 5.0).abs() < 1e-6,
                "{b:?}"
            );
        }
        // A stroke passing beside every outline removes nothing.
        let before = app.engine.history_labels().0.len();
        assert!(
            !app.delete_virtual_segments_along(&[Point::new(40.0, 10.0), Point::new(60.0, 12.0)])
        );
        assert!(!app.delete_virtual_segments_along(&[]));
        assert_eq!(app.engine.history_labels().0.len(), before);
    }

    /// What a drag's box touches: an end inside, a curve dipping in with
    /// both ends outside, a curve through a corner, a line along a box of
    /// no height; and a stretch wrapping over a closed outline's start.
    #[test]
    fn what_a_box_touches() {
        let r = Rect::new(10.0, 10.0, 20.0, 20.0);
        let line = |a: (f64, f64), b: (f64, f64)| PathSeg::Line(Line::new(a, b));
        assert!(
            seg_touches(&line((15.0, 15.0), (40.0, 40.0)), r),
            "an end inside"
        );
        assert!(seg_touches(&line((0.0, 15.0), (30.0, 15.0)), r), "across");
        assert!(!seg_touches(&line((0.0, 25.0), (30.0, 25.0)), r), "above");
        assert!(
            seg_touches(&line((0.0, 0.0), (30.0, 30.0)), r),
            "through two corners"
        );
        assert!(
            seg_touches(&line((0.0, 30.0), (30.0, 0.0)), r),
            "the other diagonal"
        );
        assert!(
            seg_touches(&line((0.0, 20.0), (40.0, 0.0)), r),
            "through a corner"
        );
        // A cubic dipping 2 mm into the box from below, both ends outside.
        let dip = PathSeg::Cubic(tracedraw_core::geometry::CubicBez::new(
            (0.0, 0.0),
            (10.0, 16.0),
            (20.0, 16.0),
            (30.0, 0.0),
        ));
        assert!(seg_touches(&dip, r));
        assert!(
            !seg_touches(&dip, Rect::new(10.0, 13.0, 20.0, 20.0)),
            "it stops at 12"
        );
        // A box of no height along a level line.
        let level = line((0.0, 0.0), (30.0, 0.0));
        assert!(seg_touches(&level, Rect::new(10.0, 0.0, 20.0, 0.0)));
        assert!(!seg_touches(&level, Rect::new(10.0, 1.0, 20.0, 1.0)));
        // A square with two crossings on its bottom side: the stretch
        // over the start (the bottom's ends, left and right sides, top)
        // is touched by a box on the top side only.
        let sub = Sub {
            segs: vec![
                line((0.0, 0.0), (10.0, 0.0)),
                line((10.0, 0.0), (10.0, 10.0)),
                line((10.0, 10.0), (0.0, 10.0)),
                line((0.0, 10.0), (0.0, 0.0)),
            ],
            closed: true,
        };
        let top = Rect::new(4.0, 9.0, 6.0, 11.0);
        assert!(
            stretch_touches(&sub, 0.6, 0.4, top),
            "wrapping over the start"
        );
        assert!(!stretch_touches(&sub, 0.4, 0.6, top), "the bottom's middle");
    }

    #[test]
    fn pieces_and_stretches() {
        let sub = Sub {
            segs: vec![
                PathSeg::Line(Line::new((0.0, 0.0), (10.0, 0.0))),
                PathSeg::Line(Line::new((10.0, 0.0), (10.0, 10.0))),
                PathSeg::Line(Line::new((10.0, 10.0), (0.0, 0.0))),
            ],
            closed: true,
        };
        assert_eq!(pieces(&sub, &[]), vec![(0.0, 3.0)]);
        assert_eq!(pieces(&sub, &[0.5, 1.5]), vec![(0.5, 1.5), (1.5, 0.5)]);
        assert_eq!(around(&sub, &[0.5, 1.5], 2.0), (1.5, 0.5));
        assert_eq!(around(&sub, &[0.5, 1.5], 1.0), (0.5, 1.5));
        // The wrapping stretch removed: one piece from 0.5 to 1.5.
        let left = without(&sub, &[(1.5, 0.5)]).expect("changed");
        assert_eq!(left.len(), 1);
        let b = left[0].bounding_box();
        assert!(
            (b.x0 - 5.0).abs() < 1e-9 && (b.y1 - 5.0).abs() < 1e-9,
            "{b:?}"
        );
        // The whole loop removed: nothing left.
        assert!(without(&sub, &[(0.0, 3.0)]).expect("changed").is_empty());
        assert!(without(&sub, &[]).is_none());
    }
}
