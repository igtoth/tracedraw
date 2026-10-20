//! Node editing on a `BezPath`, the model behind the Shape tool.
//!
//! A node is the end point of a path element. Its incoming control point
//! is `c2` of its own `CurveTo`; its outgoing control point is `c1` of the
//! next `CurveTo`. Quadratics are elevated to cubics on first edit so there
//! is one kind of curve to deal with.

use crate::geometry::{BezPath, PathEl, Point, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NodeType {
    /// Handles move independently.
    Cusp,
    /// Handles stay collinear, lengths independent.
    Smooth,
    /// Handles stay collinear and equal in length.
    Symmetrical,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    /// Index of the element that ends at this node.
    pub index: usize,
    pub pos: Point,
    pub ctrl_in: Option<Point>,
    pub ctrl_out: Option<Point>,
    /// First node of a subpath.
    pub is_start: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    In,
    Out,
}

/// For every closed subpath whose last segment ends back on its start,
/// the pair (start index, index of that last segment). The two are one
/// node: the start, whose incoming handle is the last segment's `c2`.
fn closing_twins(els: &[PathEl]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    for i in 0..els.len() {
        match els[i] {
            PathEl::MoveTo(_) => start = i,
            PathEl::ClosePath if i > start + 1 => {
                if let (PathEl::MoveTo(s), Some(end)) = (els[start], end_point(&els[i - 1])) {
                    if (s - end).hypot() < 1e-9 {
                        out.push((start, i - 1));
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// The closing twin of the node at `index`: the last segment for a start,
/// or the start for that last segment.
fn twin_of(els: &[PathEl], index: usize) -> Option<usize> {
    closing_twins(els).into_iter().find_map(|(a, b)| {
        if a == index {
            Some(b)
        } else if b == index {
            Some(a)
        } else {
            None
        }
    })
}

/// Elevate quadratics to cubics so every curve has two control points.
pub fn normalize(path: &BezPath) -> BezPath {
    let mut out = BezPath::new();
    let mut last = Point::ZERO;
    let mut start = Point::ZERO;
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                out.move_to(p);
                last = p;
                start = p;
            }
            PathEl::LineTo(p) => {
                out.line_to(p);
                last = p;
            }
            PathEl::QuadTo(c, p) => {
                let c1 = last + (c - last) * (2.0 / 3.0);
                let c2 = p + (c - p) * (2.0 / 3.0);
                out.curve_to(c1, c2, p);
                last = p;
            }
            PathEl::CurveTo(c1, c2, p) => {
                out.curve_to(c1, c2, p);
                last = p;
            }
            PathEl::ClosePath => {
                out.close_path();
                last = start;
            }
        }
    }
    out
}

/// The nodes of `path`. The start of a closed subpath whose last segment
/// ends on it is one node (that segment's end is not listed again); its
/// incoming handle is the last segment's.
pub fn nodes(path: &BezPath) -> Vec<Node> {
    let els = path.elements();
    let twins = closing_twins(els);
    let mut out = Vec::new();
    for (i, el) in els.iter().enumerate() {
        if twins.iter().any(|&(_, last)| last == i) {
            continue;
        }
        let (pos, mut ctrl_in, is_start) = match *el {
            PathEl::MoveTo(p) => (p, None, true),
            PathEl::LineTo(p) => (p, None, false),
            PathEl::QuadTo(c, p) => (p, Some(c), false),
            PathEl::CurveTo(_, c2, p) => (p, Some(c2), false),
            PathEl::ClosePath => continue,
        };
        if let Some(&(_, last)) = twins.iter().find(|&&(start, _)| start == i) {
            ctrl_in = match els[last] {
                PathEl::CurveTo(_, c2, _) => Some(c2),
                PathEl::QuadTo(c, _) => Some(c),
                _ => None,
            };
        }
        let ctrl_out = match els.get(i + 1) {
            Some(PathEl::CurveTo(c1, _, _)) => Some(*c1),
            Some(PathEl::QuadTo(c, _)) => Some(*c),
            _ => None,
        };
        out.push(Node {
            index: i,
            pos,
            ctrl_in,
            ctrl_out,
            is_start,
        });
    }
    out
}

/// The type of node `index`: the one chosen for it in `chosen` (a curve's
/// `Shape::node_types`) when the node exists, else as its handles read.
pub fn node_type_with(
    path: &BezPath,
    index: usize,
    chosen: &std::collections::BTreeMap<usize, NodeType>,
) -> NodeType {
    match chosen.get(&index) {
        Some(t) if nodes(path).iter().any(|n| n.index == index) => *t,
        _ => node_type(path, index),
    }
}

/// [`node_type_with`] for a node already found by [`nodes`] (no search of
/// the path: callers going over every node stay linear).
pub fn node_type_of_with(
    n: &Node,
    chosen: &std::collections::BTreeMap<usize, NodeType>,
) -> NodeType {
    chosen
        .get(&n.index)
        .copied()
        .unwrap_or_else(|| node_type_of(n))
}

/// The type of node `index` as its handles read: aligned handles are
/// smooth, aligned ones of equal length symmetrical, the rest cusp.
pub fn node_type(path: &BezPath, index: usize) -> NodeType {
    match nodes(path).into_iter().find(|n| n.index == index) {
        Some(n) => node_type_of(&n),
        None => NodeType::Cusp,
    }
}

/// The type a node's handles show (see [`node_type`]), for a node already
/// found by [`nodes`]; a curve's nodes classified one by one this way are
/// worked out once instead of once a node.
pub fn node_type_of(n: &Node) -> NodeType {
    let (Some(a), Some(b)) = (n.ctrl_in, n.ctrl_out) else {
        return NodeType::Cusp;
    };
    let va = a - n.pos;
    let vb = b - n.pos;
    if va.hypot() < 1e-9 || vb.hypot() < 1e-9 {
        return NodeType::Cusp;
    }
    let cross = va.cross(vb).abs() / (va.hypot() * vb.hypot());
    let opposite = va.dot(vb) < 0.0;
    if cross < 1e-3 && opposite {
        if (va.hypot() - vb.hypot()).abs() < 1e-6 {
            NodeType::Symmetrical
        } else {
            NodeType::Smooth
        }
    } else {
        NodeType::Cusp
    }
}

/// Move a node (and its two handles) by `d`.
pub fn move_node(path: &BezPath, index: usize, d: Vec2) -> BezPath {
    let mut els: Vec<PathEl> = normalize(path).elements().to_vec();
    // Found before anything moves: the twin is the end lying on the start.
    let twin = twin_of(&els, index);
    if let Some(el) = els.get_mut(index) {
        *el = match *el {
            PathEl::MoveTo(p) => PathEl::MoveTo(p + d),
            PathEl::LineTo(p) => PathEl::LineTo(p + d),
            PathEl::CurveTo(c1, c2, p) => PathEl::CurveTo(c1, c2 + d, p + d),
            other => other,
        };
    }
    if let Some(PathEl::CurveTo(c1, _, _)) = els.get_mut(index + 1) {
        *c1 += d;
    }
    // The start of a closed subpath and the end of its last segment are one
    // node: the other half moves too, with its handle.
    if let Some(t) = twin {
        if let Some(el) = els.get_mut(t) {
            *el = match *el {
                PathEl::MoveTo(p) => PathEl::MoveTo(p + d),
                PathEl::LineTo(p) => PathEl::LineTo(p + d),
                PathEl::CurveTo(c1, c2, p) => PathEl::CurveTo(c1, c2 + d, p + d),
                other => other,
            };
        }
        if t < index {
            // `t` is the start: its outgoing handle moves as well.
            if let Some(PathEl::CurveTo(c1, _, _)) = els.get_mut(t + 1) {
                *c1 += d;
            }
        }
    }
    BezPath::from_vec(els)
}

/// Move one control handle of a node, honouring the node type.
pub fn move_handle(
    path: &BezPath,
    index: usize,
    which: Which,
    new_pos: Point,
    ty: NodeType,
) -> BezPath {
    let mut els: Vec<PathEl> = normalize(path).elements().to_vec();
    let node_pos = match els.get(index) {
        Some(PathEl::MoveTo(p)) | Some(PathEl::LineTo(p)) | Some(PathEl::CurveTo(_, _, p)) => *p,
        _ => return path.clone(),
    };
    // A closed subpath's start takes its incoming handle from the last
    // segment, which ends on it.
    let in_index = match els.get(index) {
        Some(PathEl::MoveTo(_)) => twin_of(&els, index).unwrap_or(index),
        _ => index,
    };
    let set_in = |els: &mut Vec<PathEl>, p: Point| {
        if let Some(PathEl::CurveTo(_, c2, _)) = els.get_mut(in_index) {
            *c2 = p;
        }
    };
    let set_out = |els: &mut Vec<PathEl>, p: Point| {
        if let Some(PathEl::CurveTo(c1, _, _)) = els.get_mut(index + 1) {
            *c1 = p;
        }
    };
    let other_len = |els: &[PathEl]| -> Option<f64> {
        match which {
            Which::In => match els.get(index + 1) {
                Some(PathEl::CurveTo(c1, _, _)) => Some((*c1 - node_pos).hypot()),
                _ => None,
            },
            Which::Out => match els.get(in_index) {
                Some(PathEl::CurveTo(_, c2, _)) => Some((*c2 - node_pos).hypot()),
                _ => None,
            },
        }
    };
    let dir = new_pos - node_pos;
    match which {
        Which::In => set_in(&mut els, new_pos),
        Which::Out => set_out(&mut els, new_pos),
    }
    if ty != NodeType::Cusp && dir.hypot() > 1e-9 {
        if let Some(len) = other_len(&els) {
            let len = if ty == NodeType::Symmetrical {
                dir.hypot()
            } else {
                len
            };
            let mirrored = node_pos - dir / dir.hypot() * len;
            match which {
                Which::In => set_out(&mut els, mirrored),
                Which::Out => set_in(&mut els, mirrored),
            }
        }
    }
    BezPath::from_vec(els)
}

/// Make the node cusp/smooth/symmetrical by adjusting its handles. With a
/// curve on both sides, the handles line up (and for symmetrical take
/// one length). With a straight segment on one side, the curve's handle
/// lines up with it, keeping its length, so the join is smooth and the
/// line stays straight. With straight segments on both sides they become
/// curves first (as To Curve makes them), then the node is smoothed; a
/// closing line that is only a close stays straight. An end node of an
/// open subpath has one side only and stays as it is.
pub fn set_node_type(path: &BezPath, index: usize, ty: NodeType) -> BezPath {
    let n = normalize(path);
    let Some(node) = nodes(&n).into_iter().find(|n| n.index == index) else {
        return n;
    };
    if ty == NodeType::Cusp {
        return n;
    }
    let els = n.elements();
    match (node.ctrl_in, node.ctrl_out) {
        (Some(a), Some(b)) => {
            let va = a - node.pos;
            let vb = b - node.pos;
            let dir = (va - vb).normalize();
            if !dir.x.is_finite() {
                return n;
            }
            let (la, lb) = if ty == NodeType::Symmetrical {
                let l = (va.hypot() + vb.hypot()) / 2.0;
                (l, l)
            } else {
                (va.hypot(), vb.hypot())
            };
            let n2 = move_handle(&n, index, Which::In, node.pos + dir * la, NodeType::Cusp);
            move_handle(&n2, index, Which::Out, node.pos - dir * lb, NodeType::Cusp)
        }
        // Leaving on a straight segment: the arriving handle points back
        // along it.
        (Some(a), None) => {
            let Some(dir) = line_out_point(els, index).and_then(|q| unit(q - node.pos)) else {
                return n;
            };
            let len = (a - node.pos).hypot();
            move_handle(&n, index, Which::In, node.pos - dir * len, NodeType::Cusp)
        }
        // Arriving on a straight segment: the leaving handle carries on
        // along it.
        (None, Some(b)) => {
            let Some(dir) = line_in_point(els, index).and_then(|p| unit(node.pos - p)) else {
                return n;
            };
            let len = (b - node.pos).hypot();
            move_handle(&n, index, Which::Out, node.pos + dir * len, NodeType::Cusp)
        }
        (None, None) => {
            // An end node of an open subpath has one side only.
            let has_out = matches!(
                els.get(index + 1),
                Some(PathEl::LineTo(_) | PathEl::CurveTo(..) | PathEl::ClosePath)
            );
            if !has_out || line_in_point(els, index).is_none() {
                return n;
            }
            let mut m = n.clone();
            let mut curved = false;
            let in_index = match els.get(index) {
                Some(PathEl::MoveTo(_)) => twin_of(els, index),
                _ => Some(index),
            };
            if let Some(i) = in_index.filter(|i| matches!(els.get(*i), Some(PathEl::LineTo(_)))) {
                m = to_curve(&m, i);
                curved = true;
            }
            if matches!(els.get(index + 1), Some(PathEl::LineTo(_))) {
                m = to_curve(&m, index + 1);
                curved = true;
            }
            if curved {
                set_node_type(&m, index, ty)
            } else {
                n
            }
        }
    }
}

/// Where the straight segment leaving node `index` goes: a line's end, or
/// the subpath's start for a closing line.
fn line_out_point(els: &[PathEl], index: usize) -> Option<Point> {
    match els.get(index + 1)? {
        PathEl::LineTo(q) => Some(*q),
        PathEl::ClosePath => Some(subpath_start(els, index)),
        _ => None,
    }
}

/// Where the straight segment arriving at node `index` comes from; for a
/// closed subpath's start, the last segment's start, or the last point
/// when a closing line arrives.
fn line_in_point(els: &[PathEl], index: usize) -> Option<Point> {
    match els.get(index)? {
        PathEl::MoveTo(_) => {
            if let Some(t) = twin_of(els, index) {
                return Some(prev_point(els, t));
            }
            let mut j = index + 1;
            while let Some(e) = els.get(j) {
                match e {
                    PathEl::MoveTo(_) => return None,
                    PathEl::ClosePath => return end_point(els.get(j - 1)?),
                    _ => j += 1,
                }
            }
            None
        }
        _ => Some(prev_point(els, index)),
    }
}

/// Delete a node; the neighbouring segments are joined.
/// The cubic that element `el`, starting at `start`, draws. Lines are raised
/// to cubics with evenly spaced control points; only segments that carry a
/// point come back.
fn element_cubic(start: Point, el: PathEl) -> Option<kurbo::CubicBez> {
    use kurbo::CubicBez;
    match el {
        PathEl::LineTo(p) => Some(CubicBez::new(
            start,
            start.lerp(p, 1.0 / 3.0),
            start.lerp(p, 2.0 / 3.0),
            p,
        )),
        PathEl::CurveTo(c1, c2, p) => Some(CubicBez::new(start, c1, c2, p)),
        _ => None,
    }
}

/// A unit vector, or `None` when the input is too short to have a direction.
fn unit(v: Vec2) -> Option<Vec2> {
    let len = v.hypot();
    (len > 1e-9).then(|| v / len)
}

/// Fit one cubic from `p0` to `p3` to the ordered sample points `pts`,
/// holding the end tangents `t1` (unit, leaving `p0`) and `t2` (unit,
/// leaving `p3` back along the curve). The control points come out as
/// `p0 + a1 * t1` and `p3 + a2 * t2`, the two lengths from a least-squares
/// fit with fixed endpoints and tangents (Graphics Gems). A degenerate fit
/// falls back to a third of the chord.
fn fit_cubic(p0: Point, p3: Point, t1: Vec2, t2: Vec2, pts: &[Point]) -> (Point, Point) {
    use kurbo::ParamCurveNearest;
    let fallback = ((p3 - p0).hypot() / 3.0).max(1e-6);
    if pts.len() < 2 {
        return (p0 + t1 * fallback, p3 + t2 * fallback);
    }
    // Chord-length parameters in [0, 1].
    let mut u = Vec::with_capacity(pts.len());
    let mut acc = 0.0;
    u.push(0.0);
    for w in pts.windows(2) {
        acc += (w[1] - w[0]).hypot();
        u.push(acc);
    }
    if acc <= 1e-9 {
        return (p0 + t1 * fallback, p3 + t2 * fallback);
    }
    for x in u.iter_mut() {
        *x /= acc;
    }
    // Solve for the two handle lengths, then reparameterize the samples onto
    // the fitted curve and solve again: a couple of passes pull the fit close
    // to the samples where plain chord length is off.
    let mut handles = (p0 + t1 * fallback, p3 + t2 * fallback);
    for pass in 0..3 {
        let (mut c00, mut c01, mut c11, mut x0, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for (pt, &ui) in pts.iter().zip(u.iter()) {
            let b1 = 3.0 * ui * (1.0 - ui).powi(2);
            let b2 = 3.0 * ui * ui * (1.0 - ui);
            let a0 = t1 * b1;
            let a1 = t2 * b2;
            c00 += a0.dot(a0);
            c01 += a0.dot(a1);
            c11 += a1.dot(a1);
            // The curve with both control points pinned to the endpoints,
            // which the two handle lengths then correct towards the samples.
            let b03 = (1.0 - ui).powi(3) + b1;
            let b23 = b2 + ui.powi(3);
            let base = p0.to_vec2() * b03 + p3.to_vec2() * b23;
            let tmp = pt.to_vec2() - base;
            x0 += a0.dot(tmp);
            x1 += a1.dot(tmp);
        }
        let det = c00 * c11 - c01 * c01;
        let (mut a1, mut a2) = if det.abs() < 1e-12 {
            (fallback, fallback)
        } else {
            ((x0 * c11 - c01 * x1) / det, (c00 * x1 - x0 * c01) / det)
        };
        if !(a1.is_finite() && a1 > 1e-6) {
            a1 = fallback;
        }
        if !(a2.is_finite() && a2 > 1e-6) {
            a2 = fallback;
        }
        handles = (p0 + t1 * a1, p3 + t2 * a2);
        if pass == 2 {
            break;
        }
        // Reparameterize: move each u towards the nearest point on the fit.
        let cubic = kurbo::CubicBez::new(p0, handles.0, handles.1, p3);
        for (pt, ui) in pts.iter().zip(u.iter_mut()) {
            *ui = cubic.nearest(*pt, 1e-6).t;
        }
    }
    handles
}

/// Sample `cubic` at `n` parameters, skipping `t = 0` when `skip_start` so a
/// shared join is not sampled twice.
fn sample_cubic(cubic: kurbo::CubicBez, n: usize, skip_start: bool, out: &mut Vec<Point>) {
    use kurbo::ParamCurve;
    let start = if skip_start { 1 } else { 0 };
    for i in start..=n {
        out.push(cubic.eval(i as f64 / n as f64));
    }
}

pub fn delete_node(path: &BezPath, index: usize) -> BezPath {
    let mut els: Vec<PathEl> = normalize(path).elements().to_vec();
    if index >= els.len() {
        return path.clone();
    }
    match els[index] {
        PathEl::MoveTo(_) if twin_of(&els, index).is_some_and(|t| t > index + 1) => {
            // A closed subpath's start: the next node becomes the start and
            // the last segment runs on to it, keeping the outer handles.
            let last = twin_of(&els, index).unwrap_or(index);
            let first = els[index + 1];
            let next = match first {
                PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) | PathEl::QuadTo(_, p) => p,
                _ => return path.clone(),
            };
            els[last] = match (els[last], first) {
                (PathEl::CurveTo(c1, _, _), PathEl::CurveTo(_, c2, _)) => {
                    PathEl::CurveTo(c1, c2, next)
                }
                (PathEl::CurveTo(c1, _, _), _) => PathEl::CurveTo(c1, next, next),
                (_, PathEl::CurveTo(_, c2, _)) => {
                    let from = prev_point(&els, last);
                    PathEl::CurveTo(from, c2, next)
                }
                _ => PathEl::LineTo(next),
            };
            els[index + 1] = PathEl::MoveTo(next);
            els.remove(index);
        }
        PathEl::MoveTo(_) => {
            // Promote the next element to the subpath start.
            if let Some(next) = els.get(index + 1).copied() {
                let p = match next {
                    PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) | PathEl::QuadTo(_, p) => p,
                    _ => return path.clone(),
                };
                els[index + 1] = PathEl::MoveTo(p);
                els.remove(index);
            }
        }
        PathEl::LineTo(_) | PathEl::CurveTo(..) | PathEl::QuadTo(..) => {
            // The node to delete (B) ends segment `index` (A to B); segment
            // `index + 1` (B to C) carries on. When either side is a curve,
            // refit one cubic A to C to the old A-B-C shape so the curve is
            // kept, instead of collapsing to the far handles (which cut a
            // corner, or a straight line when a neighbour was a line).
            let seg_a = els[index];
            let a = prev_point(&els, index);
            let next = els.get(index + 1).copied();
            let refit = match (seg_a, next) {
                (PathEl::LineTo(_), Some(PathEl::LineTo(_))) => None,
                (_, Some(nx)) => {
                    let b = end_point(&seg_a);
                    match (
                        b,
                        element_cubic(a, seg_a),
                        b.and_then(|b| element_cubic(b, nx)),
                    ) {
                        (Some(_), Some(ca), Some(cb)) => Some((ca, cb)),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some((ca, cb)) = refit {
                let c = cb.p3;
                let t1 = unit(ca.p1 - ca.p0)
                    .or_else(|| unit(ca.p2 - ca.p0))
                    .or_else(|| unit(ca.p3 - ca.p0))
                    .or_else(|| unit(c - a))
                    .unwrap_or(Vec2::new(1.0, 0.0));
                let t2 = unit(cb.p2 - cb.p3)
                    .or_else(|| unit(cb.p1 - cb.p3))
                    .or_else(|| unit(cb.p0 - cb.p3))
                    .or_else(|| unit(a - c))
                    .unwrap_or(Vec2::new(-1.0, 0.0));
                let mut pts = Vec::new();
                sample_cubic(ca, 8, false, &mut pts);
                sample_cubic(cb, 8, true, &mut pts);
                let (h1, h2) = fit_cubic(a, c, t1, t2, &pts);
                els[index + 1] = PathEl::CurveTo(h1, h2, c);
                els.remove(index);
            } else {
                // Two lines, or no following segment: drop the node and keep
                // a straight join, carrying the next curve's end handle.
                let removed = els.remove(index);
                if let (PathEl::CurveTo(c1, _, _), Some(PathEl::CurveTo(_, c2, p))) =
                    (removed, els.get(index).copied())
                {
                    els[index] = PathEl::CurveTo(c1, c2, p);
                }
            }
        }
        PathEl::ClosePath => {}
    }
    BezPath::from_vec(els)
}

/// Split the segment ending at `index` at parameter `t`, inserting a node.
pub fn add_node(path: &BezPath, index: usize, t: f64) -> BezPath {
    let n = normalize(path);
    let els = n.elements();
    let Some(el) = els.get(index).copied() else {
        return n;
    };
    let prev = prev_point(els, index);
    let t = t.clamp(0.01, 0.99);
    let mut out: Vec<PathEl> = els[..index].to_vec();
    match el {
        PathEl::LineTo(p) => {
            out.push(PathEl::LineTo(prev.lerp(p, t)));
            out.push(PathEl::LineTo(p));
        }
        PathEl::CurveTo(c1, c2, p) => {
            // De Casteljau split.
            let p01 = prev.lerp(c1, t);
            let p12 = c1.lerp(c2, t);
            let p23 = c2.lerp(p, t);
            let p012 = p01.lerp(p12, t);
            let p123 = p12.lerp(p23, t);
            let mid = p012.lerp(p123, t);
            out.push(PathEl::CurveTo(p01, p012, mid));
            out.push(PathEl::CurveTo(p123, p23, p));
        }
        PathEl::ClosePath => {
            let start = subpath_start(els, index);
            out.push(PathEl::LineTo(prev.lerp(start, t)));
            out.push(PathEl::ClosePath);
        }
        other => out.push(other),
    }
    out.extend_from_slice(&els[index + 1..]);
    BezPath::from_vec(out)
}

/// Bend the segment ending at element `index` so that its point at
/// parameter `t` moves by `d`, its end nodes staying put: a straight
/// segment (or a closing one) becomes a curve first; both control points
/// move, the nearer one more (by `k (1 - t) d` and `k t d`, `k` making the
/// point follow `d` exactly); an end node that was smooth or symmetrical
/// stays so (its other handle turns along). Paths whose segment cannot be
/// bent come back unchanged.
pub fn bend_segment(path: &BezPath, index: usize, t: f64, d: Vec2) -> BezPath {
    let n = normalize(path);
    let mut els: Vec<PathEl> = n.elements().to_vec();
    if !(d.x.is_finite() && d.y.is_finite()) || index == 0 || index >= els.len() {
        return n;
    }
    let types_before = (node_type(&n, index - 1), node_type(&n, index));
    let prev = prev_point(&els, index);
    // The node the segment ends at: itself, or for a closing segment the
    // subpath's start.
    let mut end_node = index;
    match els[index] {
        PathEl::LineTo(p) => {
            els[index] = PathEl::CurveTo(prev.lerp(p, 1.0 / 3.0), prev.lerp(p, 2.0 / 3.0), p);
        }
        PathEl::CurveTo(..) => {}
        PathEl::ClosePath => {
            let start = subpath_start(&els, index);
            if (start - prev).hypot() < 1e-9 {
                return n;
            }
            els.insert(
                index,
                PathEl::CurveTo(
                    prev.lerp(start, 1.0 / 3.0),
                    prev.lerp(start, 2.0 / 3.0),
                    start,
                ),
            );
            end_node = (0..index)
                .rev()
                .find(|&i| matches!(els[i], PathEl::MoveTo(_)))
                .unwrap_or(0);
        }
        _ => return n,
    }
    let PathEl::CurveTo(c1, c2, _) = els[index] else {
        return n;
    };
    let t = t.clamp(0.02, 0.98);
    let u = 1.0 - t;
    let k = 1.0 / (3.0 * t * u * (u * u + t * t));
    let (n1, n2) = (c1 + d * (k * u), c2 + d * (k * t));
    let bent = BezPath::from_vec(els);
    // Through move_handle so smooth and symmetrical ends stay so (the
    // types as they were before the segment became a curve).
    let start_type = types_before.0;
    let end_type = if end_node == index {
        types_before.1
    } else {
        node_type(&n, end_node)
    };
    let bent = move_handle(&bent, index - 1, Which::Out, n1, start_type);
    move_handle(&bent, end_node, Which::In, n2, end_type)
}

/// Turn the segment ending at `index` into a straight line.
pub fn to_line(path: &BezPath, index: usize) -> BezPath {
    let mut els: Vec<PathEl> = normalize(path).elements().to_vec();
    if let Some(PathEl::CurveTo(_, _, p)) = els.get(index).copied() {
        els[index] = PathEl::LineTo(p);
    }
    BezPath::from_vec(els)
}

/// Turn the segment ending at `index` into a curve with handles at thirds.
pub fn to_curve(path: &BezPath, index: usize) -> BezPath {
    let mut els: Vec<PathEl> = normalize(path).elements().to_vec();
    if let Some(PathEl::LineTo(p)) = els.get(index).copied() {
        let prev = prev_point(&els, index);
        els[index] = PathEl::CurveTo(prev.lerp(p, 1.0 / 3.0), prev.lerp(p, 2.0 / 3.0), p);
    }
    BezPath::from_vec(els)
}

/// Close the subpath containing `index` (adds a ClosePath if missing).
pub fn close_subpath(path: &BezPath, index: usize) -> BezPath {
    let mut els: Vec<PathEl> = normalize(path).elements().to_vec();
    let mut end = index;
    while end + 1 < els.len() && !matches!(els[end + 1], PathEl::MoveTo(_)) {
        end += 1;
    }
    if !matches!(els[end], PathEl::ClosePath) {
        els.insert(end + 1, PathEl::ClosePath);
    }
    BezPath::from_vec(els)
}

/// Break the path at a node: the node becomes the end of one subpath and
/// the start of another.
pub fn break_at(path: &BezPath, index: usize) -> BezPath {
    let mut els: Vec<PathEl> = normalize(path).elements().to_vec();
    // At a closed subpath's start, whose last segment ends on it: open the
    // subpath there (its two ends stay where they are).
    if let (Some(PathEl::MoveTo(_)), Some(last)) = (els.get(index), twin_of(&els, index)) {
        if matches!(els.get(last + 1), Some(PathEl::ClosePath)) {
            els.remove(last + 1);
            return BezPath::from_vec(els);
        }
    }
    let p = match els.get(index) {
        Some(PathEl::LineTo(p)) | Some(PathEl::CurveTo(_, _, p)) => *p,
        _ => return path.clone(),
    };
    // Remove a ClosePath for this subpath: breaking opens it.
    let mut end = index;
    while end + 1 < els.len() && !matches!(els[end + 1], PathEl::MoveTo(_)) {
        end += 1;
    }
    if matches!(els[end], PathEl::ClosePath) {
        els.remove(end);
    }
    els.insert(index + 1, PathEl::MoveTo(p));
    BezPath::from_vec(els)
}

/// Reverse the direction of the whole path.
/// Reduce nodes: drop every node whose removal moves the curve by less
/// than `tolerance` (sampled at the removed node's position). Subpath
/// starts and ends are kept.
pub fn reduce_nodes(path: &BezPath, tolerance: f64) -> BezPath {
    use kurbo::{ParamCurveNearest, PathSeg};
    let mut cur = normalize(path);
    loop {
        let ns = nodes(&cur);
        let mut removed = false;
        // Walk from the end so indices stay valid.
        for n in ns.iter().rev() {
            if n.is_start {
                continue;
            }
            let next_is_end = match cur.elements().get(n.index + 1) {
                None | Some(PathEl::MoveTo(_)) => true,
                Some(PathEl::ClosePath) => false,
                _ => false,
            };
            if next_is_end {
                continue;
            }
            let candidate = delete_node(&cur, n.index);
            if candidate.elements().len() == cur.elements().len() {
                continue;
            }
            // Distance from the dropped node to the new curve.
            let d = candidate
                .segments()
                .map(|seg: PathSeg| seg.nearest(n.pos, 1e-3).distance_sq.sqrt())
                .fold(f64::INFINITY, f64::min);
            if d <= tolerance {
                cur = candidate;
                removed = true;
                break;
            }
        }
        if !removed {
            break;
        }
    }
    cur
}

/// Reduce nodes among `chosen` only (element indices): like
/// `reduce_nodes`, but other nodes stay. Indices are tracked as nodes go.
pub fn reduce_chosen_nodes(path: &BezPath, chosen: &[usize], tolerance: f64) -> BezPath {
    use kurbo::{ParamCurveNearest, PathSeg};
    let mut cur = normalize(path);
    let mut chosen: Vec<usize> = chosen.to_vec();
    loop {
        let ns = nodes(&cur);
        let mut removed = false;
        for n in ns.iter().rev() {
            if n.is_start || !chosen.contains(&n.index) {
                continue;
            }
            let next_is_end = matches!(
                cur.elements().get(n.index + 1),
                None | Some(PathEl::MoveTo(_))
            );
            if next_is_end {
                continue;
            }
            let candidate = delete_node(&cur, n.index);
            if candidate.elements().len() == cur.elements().len() {
                continue;
            }
            let d = candidate
                .segments()
                .map(|seg: PathSeg| seg.nearest(n.pos, 1e-3).distance_sq.sqrt())
                .fold(f64::INFINITY, f64::min);
            if d <= tolerance {
                cur = candidate;
                // Elements after the removed one shift down by one.
                chosen.retain(|i| *i != n.index);
                for i in chosen.iter_mut() {
                    if *i > n.index {
                        *i -= 1;
                    }
                }
                removed = true;
                break;
            }
        }
        if !removed {
            break;
        }
    }
    cur
}

/// Align the given nodes on a common x and/or y (their average).
pub fn align_nodes(path: &BezPath, indices: &[usize], horizontal: bool, vertical: bool) -> BezPath {
    let ns = nodes(path);
    let sel: Vec<&Node> = ns.iter().filter(|n| indices.contains(&n.index)).collect();
    if sel.len() < 2 {
        return path.clone();
    }
    let cx = sel.iter().map(|n| n.pos.x).sum::<f64>() / sel.len() as f64;
    let cy = sel.iter().map(|n| n.pos.y).sum::<f64>() / sel.len() as f64;
    let mut out = path.clone();
    for n in sel {
        let target = Point::new(
            if vertical { cx } else { n.pos.x },
            if horizontal { cy } else { n.pos.y },
        );
        out = move_node(&out, n.index, target - n.pos);
    }
    out
}

pub fn reverse(path: &BezPath) -> BezPath {
    let n = normalize(path);
    let mut out = BezPath::new();
    let mut subpaths: Vec<Vec<PathEl>> = Vec::new();
    for el in n.elements() {
        if let PathEl::MoveTo(_) = el {
            subpaths.push(Vec::new());
        }
        if let Some(sp) = subpaths.last_mut() {
            sp.push(*el);
        }
    }
    for sp in subpaths {
        let closed = matches!(sp.last(), Some(PathEl::ClosePath));
        let pts: Vec<(Point, Option<(Point, Point)>)> = {
            let mut v = Vec::new();
            let mut last = Point::ZERO;
            for el in &sp {
                match *el {
                    PathEl::MoveTo(p) => {
                        v.push((p, None));
                        last = p;
                    }
                    PathEl::LineTo(p) => {
                        v.push((p, None));
                        last = p;
                    }
                    PathEl::CurveTo(c1, c2, p) => {
                        v.push((p, Some((c1, c2))));
                        last = p;
                    }
                    _ => {}
                }
            }
            let _ = last;
            v
        };
        if pts.is_empty() {
            continue;
        }
        out.move_to(pts[pts.len() - 1].0);
        for i in (1..pts.len()).rev() {
            let target = pts[i - 1].0;
            match pts[i].1 {
                Some((c1, c2)) => out.curve_to(c2, c1, target),
                None => out.line_to(target),
            }
        }
        if closed {
            out.close_path();
        }
    }
    out
}

/// Apply `t` to the given nodes and to their handles (a node's incoming
/// handle and its outgoing one). The start node of a closed subpath whose
/// last segment ends on it takes that end along.
pub fn transform_nodes(path: &BezPath, indices: &[usize], t: crate::geometry::Affine) -> BezPath {
    let mut els: Vec<PathEl> = normalize(path).elements().to_vec();
    let mut chosen: Vec<usize> = indices.to_vec();
    // A closed subpath's start node and a last segment ending on it are
    // one node.
    let mut start = 0;
    for i in 0..els.len() {
        match els[i] {
            PathEl::MoveTo(_) => start = i,
            PathEl::ClosePath if i > start + 1 => {
                let (PathEl::MoveTo(s), Some(end)) = (els[start], end_point(&els[i - 1])) else {
                    continue;
                };
                if (s - end).hypot() < 1e-9 {
                    let (a, b) = (start, i - 1);
                    if chosen.contains(&a) && !chosen.contains(&b) {
                        chosen.push(b);
                    } else if chosen.contains(&b) && !chosen.contains(&a) {
                        chosen.push(a);
                    }
                }
            }
            _ => {}
        }
    }
    let n = els.len();
    for &i in &chosen {
        if i >= n {
            continue;
        }
        els[i] = match els[i] {
            PathEl::MoveTo(p) => PathEl::MoveTo(t * p),
            PathEl::LineTo(p) => PathEl::LineTo(t * p),
            PathEl::CurveTo(c1, c2, p) => PathEl::CurveTo(c1, t * c2, t * p),
            other => other,
        };
        if let Some(PathEl::CurveTo(c1, _, _)) = els.get_mut(i + 1) {
            *c1 = t * *c1;
        }
    }
    BezPath::from_vec(els)
}

fn end_point(el: &PathEl) -> Option<Point> {
    match el {
        PathEl::MoveTo(p) | PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) | PathEl::QuadTo(_, p) => {
            Some(*p)
        }
        PathEl::ClosePath => None,
    }
}

/// A subpath as segments: each with the element that draws it (a line or
/// curve to its end) and the node index at each end. A closing segment's
/// end node is the subpath's start.
/// A segment, its start and end nodes, and the element drawing it (its
/// own index, or the `ClosePath` for a closing line).
type Seg = (PathEl, usize, usize, usize);

struct Segs {
    start: Point,
    start_index: usize,
    segs: Vec<Seg>,
    closed: bool,
}

fn split_segments(path: &BezPath) -> Vec<Segs> {
    let n = normalize(path);
    let mut out: Vec<Segs> = Vec::new();
    let mut last_index = 0;
    let mut last = Point::ZERO;
    for (i, el) in n.elements().iter().enumerate() {
        match *el {
            PathEl::MoveTo(p) => {
                out.push(Segs {
                    start: p,
                    start_index: i,
                    segs: Vec::new(),
                    closed: false,
                });
                last_index = i;
                last = p;
            }
            PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) => {
                if let Some(s) = out.last_mut() {
                    s.segs.push((*el, last_index, i, i));
                }
                last_index = i;
                last = p;
            }
            PathEl::QuadTo(..) => {}
            PathEl::ClosePath => {
                if let Some(s) = out.last_mut() {
                    if (last - s.start).hypot() > 1e-9 {
                        s.segs
                            .push((PathEl::LineTo(s.start), last_index, s.start_index, i));
                    } else if let Some(seg) = s.segs.last_mut() {
                        // The last segment ends on the start node.
                        seg.2 = s.start_index;
                    }
                    s.closed = true;
                }
            }
        }
    }
    out
}

fn seg_end(el: &PathEl) -> Point {
    end_point(el).unwrap_or(Point::ZERO)
}

/// The segments whose two end nodes are both among `indices`, as open
/// subpaths: what the Shape tool copies.
pub fn selected_segments(path: &BezPath, indices: &[usize]) -> BezPath {
    let mut out = BezPath::new();
    for sub in split_segments(path) {
        let mut at = sub.start;
        let mut open = false;
        let mut prev_end = usize::MAX;
        for (el, from, to, _) in &sub.segs {
            let picked = indices.contains(from) && indices.contains(to);
            if picked {
                if !open || prev_end != *from {
                    out.move_to(at);
                }
                out.push(*el);
                open = true;
                prev_end = *to;
            } else {
                open = false;
            }
            at = seg_end(el);
        }
    }
    out
}

/// The path without the segments `selected_segments` picks (the Shape
/// tool's Cut): what is left becomes open subpaths.
pub fn without_segments(path: &BezPath, indices: &[usize]) -> BezPath {
    cut_segments(path, &|&(_, from, to, _)| {
        indices.contains(&from) && indices.contains(&to)
    })
}

/// The path without the one segment drawn by element `index` (a line, a
/// curve, or the `ClosePath` of a closing line): an open subpath splits in
/// two there (or loses an end), a closed one opens there. A path without
/// that segment comes back unchanged.
pub fn delete_segment(path: &BezPath, index: usize) -> BezPath {
    let found = split_segments(path)
        .iter()
        .any(|sub| sub.segs.iter().any(|seg| seg.3 == index));
    if !found {
        return path.clone();
    }
    cut_segments(path, &|&(_, _, _, elem)| elem == index)
}

/// The path without the segments `picked` chooses; what is left of each
/// touched subpath becomes open subpaths.
fn cut_segments(path: &BezPath, picked: &dyn Fn(&Seg) -> bool) -> BezPath {
    let mut out = BezPath::new();
    for sub in split_segments(path) {
        let n = sub.segs.len();
        let first_cut = sub.segs.iter().position(picked);
        let Some(first_cut) = first_cut else {
            // Untouched.
            out.move_to(sub.start);
            for (el, _, _, _) in &sub.segs {
                out.push(*el);
            }
            if sub.closed {
                out.close_path();
            }
            continue;
        };
        // Walk the segments; a closed subpath starts right after a cut one.
        let order: Vec<usize> = if sub.closed {
            (1..=n).map(|k| (first_cut + k) % n).collect()
        } else {
            (0..n).collect()
        };
        let start_of = |k: usize| -> Point {
            if k == 0 {
                sub.start
            } else {
                seg_end(&sub.segs[k - 1].0)
            }
        };
        let mut open = false;
        for k in order {
            let seg = &sub.segs[k];
            if picked(seg) {
                open = false;
                continue;
            }
            if !open {
                out.move_to(start_of(k));
                open = true;
            }
            out.push(seg.0);
        }
    }
    out
}

/// The nodes from `a` to `b` along their subpath, both included and in
/// that order, and which way it went: `Some(true)` follows the path's
/// direction (wrapping round a closed subpath), `Some(false)` goes against
/// it, `None` takes the shorter way (on an open subpath, the only one).
/// `None` when they lie on different subpaths or the way asked for runs
/// off the end of an open one.
pub fn node_run(
    path: &BezPath,
    a: usize,
    b: usize,
    forward: Option<bool>,
) -> Option<(Vec<usize>, bool)> {
    for sub in split_segments(path) {
        let mut ring: Vec<usize> = vec![sub.start_index];
        for (_, _, to, _) in &sub.segs {
            if *to != sub.start_index {
                ring.push(*to);
            }
        }
        let (Some(ia), Some(ib)) = (
            ring.iter().position(|x| *x == a),
            ring.iter().position(|x| *x == b),
        ) else {
            continue;
        };
        let n = ring.len();
        let ahead = if ib >= ia {
            Some(ib - ia)
        } else if sub.closed {
            Some(n - ia + ib)
        } else {
            None
        };
        let behind = if ia >= ib {
            Some(ia - ib)
        } else if sub.closed {
            Some(ia + n - ib)
        } else {
            None
        };
        let dir = match forward {
            Some(d) => d,
            None => match (ahead, behind) {
                (Some(f), Some(r)) => f <= r,
                (Some(_), None) => true,
                _ => false,
            },
        };
        let len = if dir { ahead } else { behind }?;
        let run = (0..=len)
            .map(|k| {
                if dir {
                    ring[(ia + k) % n]
                } else {
                    ring[(ia + n - k) % n]
                }
            })
            .collect();
        return Some((run, dir));
    }
    None
}

/// Join two end nodes with a straight segment (the Shape tool's Extend
/// Curve to Close): the two ends of one open subpath close it; ends of two
/// open subpaths make one subpath. `None` when either is not an end of an
/// open subpath.
pub fn connect_ends(path: &BezPath, a: usize, b: usize) -> Option<BezPath> {
    link_ends(path, a, b, false)
}

/// Join two end nodes into one (the Shape tool's Join): both move to the
/// point halfway between them, their handles along, and become a single
/// node with no segment added. The two ends of one open subpath close it
/// there; ends of two open subpaths make one subpath running through the
/// joined node. `None` when either is not an end of an open subpath, or
/// both are the same end.
pub fn join_ends(path: &BezPath, a: usize, b: usize) -> Option<BezPath> {
    let n = normalize(path);
    let els = n.elements();
    let (pa, pb) = (end_point(els.get(a)?)?, end_point(els.get(b)?)?);
    let mid = pa.lerp(pb, 0.5);
    let moved = move_node(&move_node(&n, a, mid - pa), b, mid - pb);
    link_ends(&moved, a, b, true)
}

/// Shared by [`connect_ends`] and [`join_ends`]: with `fuse` the ends meet
/// without a segment between them (they lie on one point already).
fn link_ends(path: &BezPath, a: usize, b: usize, fuse: bool) -> Option<BezPath> {
    let subs = split_segments(path);
    // (subpath, is the end rather than the start)
    let find = |x: usize| -> Option<(usize, bool)> {
        subs.iter().enumerate().find_map(|(k, s)| {
            if s.closed || s.segs.is_empty() {
                return None;
            }
            if s.start_index == x {
                Some((k, false))
            } else if s.segs.last().map(|seg| seg.2) == Some(x) {
                Some((k, true))
            } else {
                None
            }
        })
    };
    let (sa, ea) = find(a)?;
    let (sb, eb) = find(b)?;
    let to_path = |s: &Segs| -> BezPath {
        let mut p = BezPath::new();
        p.move_to(s.start);
        for (el, _, _, _) in &s.segs {
            p.push(*el);
        }
        p
    };
    let mut out = BezPath::new();
    if sa == sb {
        if ea == eb {
            return None;
        }
        for (k, s) in subs.iter().enumerate() {
            let mut p = to_path(s);
            if s.closed || k == sa {
                p.close_path();
            }
            out.extend(p);
        }
        return Some(out);
    }
    // The first subpath ends at `a`, the second starts at `b`.
    let first = to_path(&subs[sa]);
    let first = if ea { first } else { reverse(&first) };
    let second = to_path(&subs[sb]);
    let second = if eb { reverse(&second) } else { second };
    let mut joined = first;
    for el in second.elements() {
        match *el {
            PathEl::MoveTo(_) if fuse => {}
            PathEl::MoveTo(p) => joined.line_to(p),
            other => joined.push(other),
        }
    }
    for (k, s) in subs.iter().enumerate() {
        if k == sa {
            out.extend(joined.clone());
        } else if k != sb {
            let mut p = to_path(s);
            if s.closed {
                p.close_path();
            }
            out.extend(p);
        }
    }
    Some(out)
}

/// The point at parameter `t` of the segment ending at element `index`
/// (a line, a curve or a closing line).
pub fn point_on_segment(path: &BezPath, index: usize, t: f64) -> Option<Point> {
    use kurbo::{CubicBez, ParamCurve};
    let n = normalize(path);
    let els = n.elements();
    if index == 0 || index >= els.len() {
        return None;
    }
    let prev = prev_point(els, index);
    let t = t.clamp(0.0, 1.0);
    match els[index] {
        PathEl::LineTo(p) => Some(prev.lerp(p, t)),
        PathEl::CurveTo(c1, c2, p) => Some(CubicBez::new(prev, c1, c2, p).eval(t)),
        PathEl::ClosePath => Some(prev.lerp(subpath_start(els, index), t)),
        _ => None,
    }
}

/// Nearest segment to a point: (element index, t, distance).
pub fn nearest_segment(path: &BezPath, p: Point) -> Option<(usize, f64, f64)> {
    use kurbo::{CubicBez, Line, ParamCurve, ParamCurveNearest};
    let n = normalize(path);
    let els = n.elements();
    let mut best: Option<(usize, f64, f64)> = None;
    for (i, el) in els.iter().enumerate() {
        let prev = prev_point(els, i);
        let (t, d) = match *el {
            PathEl::LineTo(q) => {
                let l = Line::new(prev, q);
                let r = l.nearest(p, 1e-6);
                (r.t, r.distance_sq.sqrt())
            }
            PathEl::CurveTo(c1, c2, q) => {
                let c = CubicBez::new(prev, c1, c2, q);
                let r = c.nearest(p, 1e-6);
                let _ = c.eval(r.t);
                (r.t, r.distance_sq.sqrt())
            }
            PathEl::ClosePath => {
                let l = Line::new(prev, subpath_start(els, i));
                let r = l.nearest(p, 1e-6);
                (r.t, r.distance_sq.sqrt())
            }
            _ => continue,
        };
        if best.map(|b| d < b.2).unwrap_or(true) {
            best = Some((i, t, d));
        }
    }
    best
}

fn prev_point(els: &[PathEl], index: usize) -> Point {
    if index == 0 {
        return Point::ZERO;
    }
    let i = index - 1;
    match els[i] {
        PathEl::MoveTo(p) | PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) | PathEl::QuadTo(_, p) => {
            p
        }
        PathEl::ClosePath => subpath_start(els, i),
    }
}

fn subpath_start(els: &[PathEl], index: usize) -> Point {
    let mut i = index;
    loop {
        if let PathEl::MoveTo(p) = els[i] {
            return p;
        }
        if i == 0 {
            return Point::ZERO;
        }
        i -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A closed smooth curve through four points: four curves, the last one
    /// back onto the start, then the close.
    fn round() -> BezPath {
        crate::geometry::smooth_path(
            &[
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
            ],
            true,
        )
    }

    fn last_curve_end(p: &BezPath) -> Point {
        p.elements()
            .iter()
            .rev()
            .find_map(|e| match e {
                PathEl::CurveTo(_, _, q) | PathEl::LineTo(q) => Some(*q),
                _ => None,
            })
            .unwrap_or(Point::ZERO)
    }

    #[test]
    fn a_closed_curve_ending_on_its_start_has_one_start_node() {
        let p = round();
        let ns = nodes(&p);
        assert_eq!(ns.len(), 4);
        let start = ns[0];
        assert!(start.is_start);
        // The start shows both handles: it is a smooth node.
        assert!(start.ctrl_in.is_some() && start.ctrl_out.is_some());
        assert_ne!(node_type(&p, start.index), NodeType::Cusp);
    }

    #[test]
    fn moving_the_start_node_takes_the_closing_end_along() {
        let p = round();
        let moved = move_node(&p, 0, Vec2::new(-3.0, -2.0));
        let start = match moved.elements()[0] {
            PathEl::MoveTo(q) => q,
            _ => Point::ZERO,
        };
        assert_eq!(start, Point::new(-3.0, -2.0));
        assert_eq!(last_curve_end(&moved), start);
        // Its incoming handle moved with it.
        let before = nodes(&p)[0].ctrl_in.unwrap_or(Point::ZERO);
        let after = nodes(&moved)[0].ctrl_in.unwrap_or(Point::ZERO);
        let d = after - before;
        assert!(
            (d.x + 3.0).abs() < 1e-9 && (d.y + 2.0).abs() < 1e-9,
            "{d:?}"
        );
    }

    #[test]
    fn the_start_nodes_incoming_handle_is_the_last_segments() {
        let p = round();
        let target = Point::new(-4.0, 3.0);
        let edited = move_handle(&p, 0, Which::In, target, NodeType::Cusp);
        assert_eq!(nodes(&edited)[0].ctrl_in, Some(target));
        // Smooth: the outgoing handle turns to stay opposite.
        let smooth = move_handle(&p, 0, Which::In, target, NodeType::Smooth);
        let n = nodes(&smooth)[0];
        let (a, b) = (
            n.ctrl_in.unwrap_or(Point::ZERO) - n.pos,
            n.ctrl_out.unwrap_or(Point::ZERO) - n.pos,
        );
        assert!(a.cross(b).abs() < 1e-6 && a.dot(b) < 0.0);
    }

    #[test]
    fn deleting_or_breaking_at_the_start_of_a_closed_curve() {
        let p = round();
        let deleted = delete_node(&p, 0);
        let ns = nodes(&deleted);
        assert_eq!(ns.len(), 3);
        assert_eq!(ns[0].pos, Point::new(10.0, 0.0));
        assert_eq!(last_curve_end(&deleted), ns[0].pos);
        assert!(matches!(deleted.elements().last(), Some(PathEl::ClosePath)));
        // Breaking opens the curve at its start; both ends stay put.
        let broken = break_at(&p, 0);
        assert!(!broken
            .elements()
            .iter()
            .any(|e| matches!(e, PathEl::ClosePath)));
        assert_eq!(nodes(&broken).len(), 5);
        assert_eq!(last_curve_end(&broken), Point::new(0.0, 0.0));
    }

    fn sample() -> BezPath {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        p.curve_to((15.0, 0.0), (20.0, 5.0), (20.0, 10.0));
        p.close_path();
        p
    }

    #[test]
    fn lists_nodes_with_handles() {
        let n = nodes(&sample());
        assert_eq!(n.len(), 3);
        assert!(n[0].is_start);
        assert_eq!(n[1].ctrl_out, Some(Point::new(15.0, 0.0)));
        assert_eq!(n[2].ctrl_in, Some(Point::new(20.0, 5.0)));
    }

    /// Dragging a segment: the point under the pointer follows exactly,
    /// the ends stay, lines become curves, closing lines too, and a smooth
    /// end node stays smooth.
    #[test]
    fn bending_a_segment_moves_its_point_and_keeps_the_ends() {
        use kurbo::{CubicBez, ParamCurve};
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((30.0, 0.0));
        let bent = bend_segment(&line, 1, 0.5, Vec2::new(0.0, 12.0));
        let PathEl::CurveTo(c1, c2, p) = bent.elements()[1] else {
            panic!("not a curve: {bent:?}");
        };
        assert_eq!(p, Point::new(30.0, 0.0));
        let mid = CubicBez::new(Point::new(0.0, 0.0), c1, c2, p).eval(0.5);
        assert!((mid - Point::new(15.0, 12.0)).hypot() < 1e-9, "{mid:?}");
        // Off the middle: the point at t moves by d all the same.
        let b2 = bend_segment(&line, 1, 0.2, Vec2::new(3.0, -4.0));
        let PathEl::CurveTo(a1, a2, q) = b2.elements()[1] else {
            panic!("not a curve");
        };
        let at = CubicBez::new(Point::new(0.0, 0.0), a1, a2, q).eval(0.2);
        assert!((at - Point::new(9.0, -4.0)).hypot() < 1e-9, "{at:?}");
        // A closing line: a curve is put in before the close.
        let mut tri = BezPath::new();
        tri.move_to((0.0, 0.0));
        tri.line_to((20.0, 0.0));
        tri.line_to((10.0, 20.0));
        tri.close_path();
        let b3 = bend_segment(&tri, 3, 0.5, Vec2::new(-5.0, 0.0));
        let els = b3.elements();
        assert!(matches!(els[3], PathEl::CurveTo(_, _, e) if e == Point::new(0.0, 0.0)));
        assert!(matches!(els[4], PathEl::ClosePath));
        assert_eq!(nodes(&b3).len(), 3);
        // A smooth node at the end of the bent segment stays smooth.
        let mut s = BezPath::new();
        s.move_to((0.0, 0.0));
        s.curve_to((5.0, 0.0), (10.0, -5.0), (20.0, 0.0));
        s.curve_to((25.0, 2.5), (35.0, 0.0), (40.0, 0.0));
        assert_eq!(node_type(&s, 1), NodeType::Smooth);
        let b4 = bend_segment(&s, 1, 0.5, Vec2::new(0.0, 6.0));
        assert_eq!(node_type(&b4, 1), NodeType::Smooth);
        assert_eq!(nodes(&b4)[1].pos, Point::new(20.0, 0.0));
        // Nonsense leaves the path as it was.
        assert_eq!(bend_segment(&s, 0, 0.5, Vec2::new(1.0, 1.0)), normalize(&s));
        assert_eq!(bend_segment(&s, 9, 0.5, Vec2::new(1.0, 1.0)), normalize(&s));
        assert_eq!(
            bend_segment(&s, 1, 0.5, Vec2::new(f64::NAN, 1.0)),
            normalize(&s)
        );
    }

    #[test]
    fn move_node_drags_handles_along() {
        let p = move_node(&sample(), 1, Vec2::new(0.0, 5.0));
        let n = nodes(&p);
        assert_eq!(n[1].pos, Point::new(10.0, 5.0));
        assert_eq!(n[1].ctrl_out, Some(Point::new(15.0, 5.0)));
    }

    #[test]
    fn add_and_delete_node_round_trip() {
        let p = add_node(&sample(), 1, 0.5);
        assert_eq!(nodes(&p).len(), 4);
        assert_eq!(nodes(&p)[1].pos, Point::new(5.0, 0.0));
        let q = delete_node(&p, 1);
        assert_eq!(nodes(&q).len(), 3);
        let c = add_node(&sample(), 2, 0.5);
        assert_eq!(
            c.elements()
                .iter()
                .filter(|e| matches!(e, PathEl::CurveTo(..)))
                .count(),
            2
        );
    }

    #[test]
    fn deleting_a_node_on_a_curve_keeps_the_shape() {
        // Split a curve, then delete the inserted node: the single curve
        // comes back, matching the original closely, not cutting a corner.
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.curve_to((10.0, 30.0), (30.0, 30.0), (40.0, 0.0));
        let split = add_node(&p, 1, 0.5);
        assert_eq!(nodes(&split).len(), 3);
        let back = delete_node(&split, 1);
        assert_eq!(nodes(&back).len(), 2);
        assert!(
            matches!(back.elements()[1], PathEl::CurveTo(..)),
            "got {:?}",
            back.elements()[1]
        );
        // The rebuilt curve follows the original within a small tolerance
        // (chord-length fitting, not an exact inverse of the split).
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            let a = point_on_segment(&p, 1, t).unwrap();
            let d = nearest_segment(&back, a).unwrap().2;
            assert!(d < 0.3, "round trip off by {d} at t={t}");
        }
        // It still bulges towards the old apex, nowhere near the straight
        // chord between the two remaining nodes.
        let mid = point_on_segment(&back, 1, 0.5).unwrap();
        assert!(mid.y > 15.0, "curve collapsed towards the chord: {mid:?}");
    }

    #[test]
    fn deleting_a_node_between_a_line_and_a_curve_keeps_a_curve() {
        // sample() is a line then a curve, closed. Deleting the node where
        // they meet must not turn the join into a straight line.
        let q = delete_node(&sample(), 1);
        assert_eq!(nodes(&q).len(), 2);
        assert!(
            matches!(q.elements()[1], PathEl::CurveTo(..)),
            "got {:?}",
            q.elements()[1]
        );
    }

    #[test]
    fn deleting_a_corner_of_a_polyline_stays_straight() {
        // With a line on both sides there is no curve to keep: a plain join.
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        p.line_to((20.0, 8.0));
        p.line_to((30.0, 0.0));
        let q = delete_node(&p, 2);
        assert_eq!(nodes(&q).len(), 3);
        assert!(
            q.elements()
                .iter()
                .all(|e| !matches!(e, PathEl::CurveTo(..))),
            "a line join turned into a curve: {q:?}"
        );
    }

    #[test]
    fn smooth_handles_stay_collinear() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.curve_to((0.0, 5.0), (5.0, 10.0), (10.0, 10.0));
        p.curve_to((15.0, 10.0), (20.0, 5.0), (20.0, 0.0));
        let q = move_handle(
            &p,
            1,
            Which::Out,
            Point::new(10.0, 15.0),
            NodeType::Symmetrical,
        );
        let n = nodes(&q)[1];
        assert_eq!(n.ctrl_out, Some(Point::new(10.0, 15.0)));
        assert_eq!(n.ctrl_in, Some(Point::new(10.0, 5.0)));
        assert_eq!(node_type(&q, 1), NodeType::Symmetrical);
    }

    #[test]
    fn nearest_segment_finds_the_line() {
        let (i, t, d) = nearest_segment(&sample(), Point::new(5.0, 1.0)).unwrap();
        assert_eq!(i, 1);
        assert!((t - 0.5).abs() < 1e-3);
        assert!((d - 1.0).abs() < 1e-6);
    }

    #[test]
    fn reverse_keeps_geometry() {
        let r = reverse(&sample());
        let n = nodes(&r);
        assert_eq!(n[0].pos, Point::new(20.0, 10.0));
        assert_eq!(n.last().unwrap().pos, Point::new(0.0, 0.0));
    }
}

#[cfg(test)]
mod reduce_tests {
    use super::*;

    #[test]
    fn reduce_drops_collinear_nodes_and_align_levels_them() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((5.0, 0.01));
        p.line_to((10.0, 0.0));
        p.line_to((10.0, 10.0));
        let r = reduce_nodes(&p, 0.1);
        assert_eq!(nodes(&r).len(), 3, "{r:?}");

        let a = align_nodes(&p, &[1, 2], true, false);
        let ns = nodes(&a);
        assert!((ns[1].pos.y - ns[2].pos.y).abs() < 1e-9);
    }
}

#[cfg(test)]
mod segment_tests {
    use super::*;
    use crate::geometry::Affine;

    fn zigzag() -> BezPath {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        p.line_to((10.0, 10.0));
        p.curve_to((5.0, 15.0), (0.0, 15.0), (0.0, 10.0));
        p
    }

    #[test]
    fn nodes_transform_with_their_handles() {
        // Nodes 2 and 3 (the curve's ends) doubled about the origin.
        let q = transform_nodes(&zigzag(), &[2, 3], Affine::scale(2.0));
        let ns = nodes(&q);
        assert_eq!(ns[2].pos, Point::new(20.0, 20.0));
        assert_eq!(ns[2].ctrl_out, Some(Point::new(10.0, 30.0)));
        assert_eq!(ns[3].pos, Point::new(0.0, 20.0));
        assert_eq!(ns[3].ctrl_in, Some(Point::new(0.0, 30.0)));
        // A closed square's start moves its closing end too.
        let mut sq = BezPath::new();
        sq.move_to((0.0, 0.0));
        sq.line_to((10.0, 0.0));
        sq.line_to((10.0, 10.0));
        sq.line_to((0.0, 0.0));
        sq.close_path();
        let q = transform_nodes(&sq, &[0], Affine::translate((1.0, 1.0)));
        assert_eq!(q.elements()[0], PathEl::MoveTo(Point::new(1.0, 1.0)));
        assert_eq!(q.elements()[3], PathEl::LineTo(Point::new(1.0, 1.0)));
    }

    #[test]
    fn segments_between_chosen_nodes_copy_and_cut() {
        let p = zigzag();
        let copied = selected_segments(&p, &[1, 2, 3]);
        let ns = nodes(&copied);
        assert_eq!(ns.len(), 3);
        assert_eq!(ns[0].pos, Point::new(10.0, 0.0));
        let left = without_segments(&p, &[1, 2, 3]);
        assert_eq!(nodes(&left).len(), 2);
        // Cutting one side of a closed square leaves one open three-sided path.
        let sq = crate::geometry::Shape::to_path(
            &crate::geometry::Rect::new(0.0, 0.0, 10.0, 10.0),
            0.01,
        );
        let left = without_segments(&sq, &[1, 2]);
        assert_eq!(
            left.elements()
                .iter()
                .filter(|e| matches!(e, PathEl::MoveTo(_)))
                .count(),
            1
        );
        assert!(!left.elements().contains(&PathEl::ClosePath));
        assert_eq!(nodes(&left).len(), 4);
        // Nothing chosen: nothing copied.
        assert!(selected_segments(&p, &[1]).elements().is_empty());
    }

    #[test]
    fn runs_follow_the_subpath() {
        let p = zigzag();
        assert_eq!(node_run(&p, 3, 1, None), Some((vec![3, 2, 1], false)));
        // An open subpath has one way only.
        assert_eq!(node_run(&p, 3, 1, Some(true)), None);
        let sq = crate::geometry::Shape::to_path(
            &crate::geometry::Rect::new(0.0, 0.0, 10.0, 10.0),
            0.01,
        );
        // Round the start is shorter from 3 to 0.
        assert_eq!(node_run(&sq, 3, 0, None), Some((vec![3, 0], true)));
        assert_eq!(node_run(&sq, 1, 2, None), Some((vec![1, 2], true)));
        // The other way round.
        assert_eq!(
            node_run(&sq, 1, 2, Some(false)),
            Some((vec![1, 0, 3, 2], false))
        );
        assert_eq!(node_run(&sq, 2, 2, None), Some((vec![2], true)));
        // Different subpaths: no run.
        let mut two = p.clone();
        two.move_to((50.0, 50.0));
        two.line_to((60.0, 50.0));
        assert_eq!(node_run(&two, 1, 4, None), None);
    }

    #[test]
    fn extending_ends_closes_or_joins_subpaths() {
        let p = zigzag();
        let closed = connect_ends(&p, 0, 3).expect("its two ends");
        assert_eq!(closed.elements().last(), Some(&PathEl::ClosePath));
        let mut two = BezPath::new();
        two.move_to((0.0, 0.0));
        two.line_to((10.0, 0.0));
        two.move_to((10.0, 5.0));
        two.line_to((0.0, 5.0));
        let joined = connect_ends(&two, 1, 2).expect("ends of two subpaths");
        assert_eq!(
            joined
                .elements()
                .iter()
                .filter(|e| matches!(e, PathEl::MoveTo(_)))
                .count(),
            1
        );
        assert_eq!(nodes(&joined).len(), 4);
        // A middle node is not an end.
        assert!(connect_ends(&p, 1, 3).is_none());
    }

    fn moves(p: &BezPath) -> usize {
        p.elements()
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count()
    }

    fn drawn(p: &BezPath) -> usize {
        p.elements()
            .iter()
            .filter(|e| matches!(e, PathEl::LineTo(_) | PathEl::CurveTo(..)))
            .count()
    }

    #[test]
    fn deleting_a_middle_segment_splits_an_open_curve() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        p.line_to((20.0, 0.0));
        p.line_to((30.0, 0.0));
        let q = delete_segment(&p, 2);
        assert_eq!(moves(&q), 2, "{q:?}");
        assert_eq!(drawn(&q), 2);
        // Every node stays where it was.
        assert_eq!(nodes(&q).len(), 4);
        // An end segment just goes: one subpath is left.
        let e = delete_segment(&p, 3);
        assert_eq!(moves(&e), 1);
        assert_eq!(drawn(&e), 2);
    }

    #[test]
    fn deleting_a_segment_opens_a_closed_curve_there() {
        // A square whose closing side is a ClosePath (element 4).
        let mut sq = BezPath::new();
        sq.move_to((0.0, 0.0));
        sq.line_to((10.0, 0.0));
        sq.line_to((10.0, 10.0));
        sq.line_to((0.0, 10.0));
        sq.close_path();
        let open = delete_segment(&sq, 4);
        assert!(!open
            .elements()
            .iter()
            .any(|e| matches!(e, PathEl::ClosePath)));
        assert_eq!(moves(&open), 1);
        assert_eq!(drawn(&open), 3);
        assert_eq!(open.elements()[0], PathEl::MoveTo(Point::new(0.0, 0.0)));
        // A side in the middle: the curve opens there and runs round.
        let side = delete_segment(&sq, 2);
        assert_eq!(moves(&side), 1);
        assert_eq!(drawn(&side), 3);
        assert_eq!(side.elements()[0], PathEl::MoveTo(Point::new(10.0, 10.0)));
        // A closed smooth curve ending on its start loses one of its four.
        let round = crate::geometry::smooth_path(
            &[
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
            ],
            true,
        );
        let r = delete_segment(&round, 1);
        assert_eq!(drawn(&r), 3);
        assert!(!r.elements().iter().any(|e| matches!(e, PathEl::ClosePath)));
    }

    #[test]
    fn deleting_a_missing_segment_changes_nothing() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        p.curve_to((15.0, 0.0), (20.0, 5.0), (20.0, 10.0));
        p.close_path();
        assert_eq!(delete_segment(&p, 0), p);
        assert_eq!(delete_segment(&p, 99), p);
    }

    #[test]
    fn joining_ends_of_two_subpaths_fuses_them_without_a_segment() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.curve_to((3.0, 5.0), (7.0, 5.0), (10.0, 0.0));
        p.move_to((12.0, 0.0));
        p.curve_to((15.0, -5.0), (18.0, -5.0), (20.0, 0.0));
        let j = join_ends(&p, 1, 2).expect("joined");
        // One subpath, still two curves: no straight segment between.
        assert_eq!(
            j.elements(),
            &[
                PathEl::MoveTo(Point::new(0.0, 0.0)),
                PathEl::CurveTo(
                    Point::new(3.0, 5.0),
                    Point::new(8.0, 5.0),
                    Point::new(11.0, 0.0)
                ),
                PathEl::CurveTo(
                    Point::new(14.0, -5.0),
                    Point::new(18.0, -5.0),
                    Point::new(20.0, 0.0)
                ),
            ]
        );
        // Either order, and a start joined to a start (the first subpath
        // runs the other way round).
        assert_eq!(nodes(&join_ends(&p, 2, 1).expect("joined")).len(), 3);
        assert_eq!(nodes(&join_ends(&p, 0, 2).expect("joined")).len(), 3);
    }

    #[test]
    fn joining_the_two_ends_of_one_subpath_closes_it_at_one_node() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        p.line_to((10.0, 10.0));
        p.line_to((1.0, 9.0));
        let j = join_ends(&p, 0, 3).expect("joined");
        assert!(matches!(j.elements().last(), Some(PathEl::ClosePath)));
        // Three nodes: the two ends are one now, and no line was added.
        let ns = nodes(&j);
        assert_eq!(ns.len(), 3);
        assert_eq!(ns[0].pos, Point::new(0.5, 4.5));
        let drawn = j
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::LineTo(_) | PathEl::CurveTo(..)))
            .count();
        assert_eq!(drawn, 3);
    }

    #[test]
    fn joining_needs_two_ends_of_open_subpaths() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        p.line_to((20.0, 0.0));
        // A middle node is not an end; one end with itself is nothing.
        assert!(join_ends(&p, 0, 1).is_none());
        assert!(join_ends(&p, 2, 2).is_none());
        assert!(join_ends(&p, 0, 99).is_none());
        let mut c = p.clone();
        c.close_path();
        assert!(join_ends(&c, 0, 2).is_none());
    }

    fn close_pt(a: Point, b: Point) -> bool {
        (a - b).hypot() < 1e-9
    }

    #[test]
    fn smoothing_a_join_of_a_line_and_a_curve_lines_the_handle_up() {
        // A line in, a curve out: the leaving handle carries on along the
        // line, keeping its length; the line stays straight.
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        p.curve_to((15.0, 5.0), (20.0, 5.0), (20.0, 10.0));
        let s = set_node_type(&p, 1, NodeType::Smooth);
        let len = 50f64.sqrt();
        assert!(matches!(s.elements()[1], PathEl::LineTo(q) if close_pt(q, Point::new(10.0, 0.0))));
        let PathEl::CurveTo(c1, _, _) = s.elements()[2] else {
            panic!("{s:?}");
        };
        assert!(close_pt(c1, Point::new(10.0 + len, 0.0)), "{c1:?}");
        // A curve in, a line out: the arriving handle points back along it.
        let mut q = BezPath::new();
        q.move_to((0.0, 0.0));
        q.curve_to((2.0, 5.0), (5.0, 5.0), (10.0, 0.0));
        q.line_to((20.0, 0.0));
        let t = set_node_type(&q, 1, NodeType::Smooth);
        let PathEl::CurveTo(_, c2, _) = t.elements()[1] else {
            panic!("{t:?}");
        };
        assert!(close_pt(c2, Point::new(10.0 - len, 0.0)), "{c2:?}");
        // Symmetrical does the same there (the line has no handle).
        assert_eq!(set_node_type(&q, 1, NodeType::Symmetrical), t);
    }

    #[test]
    fn smoothing_a_corner_of_two_lines_makes_them_curves() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        p.line_to((10.0, 10.0));
        let s = set_node_type(&p, 1, NodeType::Smooth);
        assert!(matches!(s.elements()[1], PathEl::CurveTo(..)));
        assert!(matches!(s.elements()[2], PathEl::CurveTo(..)));
        // The handles line up (equal here, so they read symmetrical).
        assert_ne!(node_type(&s, 1), NodeType::Cusp);
        assert_eq!(nodes(&s)[1].pos, Point::new(10.0, 0.0));
        assert_eq!(nodes(&s).len(), 3);
        // An end node has one side: nothing to smooth.
        assert_eq!(set_node_type(&p, 0, NodeType::Smooth), normalize(&p));
        assert_eq!(set_node_type(&p, 2, NodeType::Smooth), normalize(&p));
    }

    #[test]
    fn smoothing_beside_a_closing_line_lines_up_with_it() {
        // A curve arrives at (5, 10); the close goes straight back to (0, 0).
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        p.curve_to((12.0, 6.0), (9.0, 10.0), (5.0, 10.0));
        p.close_path();
        let s = set_node_type(&p, 2, NodeType::Smooth);
        let PathEl::CurveTo(_, c2, _) = s.elements()[2] else {
            panic!("{s:?}");
        };
        // The handle points away from (0, 0), through (5, 10).
        let away = (Point::new(5.0, 10.0) - Point::new(0.0, 0.0)).normalize();
        let h = (c2 - Point::new(5.0, 10.0)).normalize();
        assert!((h - away).hypot() < 1e-9, "{c2:?}");
        assert!(matches!(s.elements()[3], PathEl::ClosePath));
        // The start, with the closing line arriving and a line leaving:
        // the close stays a close, the line out becomes a curve.
        let t = set_node_type(&p, 0, NodeType::Smooth);
        assert!(matches!(t.elements()[1], PathEl::CurveTo(..)));
        assert!(matches!(t.elements()[3], PathEl::ClosePath));
        assert_eq!(t.elements().len(), p.elements().len());
    }
}
