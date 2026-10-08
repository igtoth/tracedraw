//! Node editing on a `BezPath`, the model behind the Shape tool.
//!
//! A node is the end point of a path element. Its incoming control point
//! is `c2` of its own `CurveTo`; its outgoing control point is `c1` of the
//! next `CurveTo`. Quadratics are elevated to cubics on first edit so there
//! is one kind of curve to deal with.

use crate::geometry::{BezPath, PathEl, Point, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

pub fn nodes(path: &BezPath) -> Vec<Node> {
    let els = path.elements();
    let mut out = Vec::new();
    for (i, el) in els.iter().enumerate() {
        let (pos, ctrl_in, is_start) = match *el {
            PathEl::MoveTo(p) => (p, None, true),
            PathEl::LineTo(p) => (p, None, false),
            PathEl::QuadTo(c, p) => (p, Some(c), false),
            PathEl::CurveTo(_, c2, p) => (p, Some(c2), false),
            PathEl::ClosePath => continue,
        };
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

pub fn node_type(path: &BezPath, index: usize) -> NodeType {
    let Some(n) = nodes(path).into_iter().find(|n| n.index == index) else {
        return NodeType::Cusp;
    };
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
    // Closing a subpath back onto a moved start node: keep the closing segment attached.
    if let Some(PathEl::MoveTo(_)) = els.get(index) {
        // Nothing else to move: ClosePath implicitly returns to the MoveTo.
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
    let set_in = |els: &mut Vec<PathEl>, p: Point| {
        if let Some(PathEl::CurveTo(_, c2, _)) = els.get_mut(index) {
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
            Which::Out => match els.get(index) {
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

/// Make the node cusp/smooth/symmetrical by adjusting its handles.
pub fn set_node_type(path: &BezPath, index: usize, ty: NodeType) -> BezPath {
    let n = normalize(path);
    let Some(node) = nodes(&n).into_iter().find(|n| n.index == index) else {
        return n;
    };
    match (node.ctrl_in, node.ctrl_out, ty) {
        (_, _, NodeType::Cusp) => n,
        (Some(a), Some(b), _) => {
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
        _ => n,
    }
}

/// Delete a node; the neighbouring segments are joined.
pub fn delete_node(path: &BezPath, index: usize) -> BezPath {
    let mut els: Vec<PathEl> = normalize(path).elements().to_vec();
    if index >= els.len() {
        return path.clone();
    }
    match els[index] {
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
            // If the next segment is a curve, keep its end handle.
            let removed = els.remove(index);
            if let (PathEl::CurveTo(c1, _, _), Some(PathEl::CurveTo(_, c2, p))) =
                (removed, els.get(index).copied())
            {
                els[index] = PathEl::CurveTo(c1, c2, p);
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
    let mut i = index;
    while i > 0 {
        i -= 1;
        match els[i] {
            PathEl::MoveTo(p)
            | PathEl::LineTo(p)
            | PathEl::CurveTo(_, _, p)
            | PathEl::QuadTo(_, p) => return p,
            PathEl::ClosePath => return subpath_start(els, i),
        }
    }
    Point::ZERO
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
