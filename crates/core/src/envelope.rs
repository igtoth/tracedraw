//! Envelopes: a frame of nodes around an object's bounding box, joined by
//! cubic segments, that the object is mapped into.
//!
//! The nodes run from the bottom-left corner along the bottom side, up the
//! right side, back along the top and down the left side; four of them are
//! the corners. Each segment, from a node to the next, has two handles. How
//! the object fills the frame is the [`Mapping`]; how moving a node
//! reshapes the frame is the editing [`Mode`].

use crate::geometry::{
    BezPath, CubicBez, ParamCurve, ParamCurveNearest, PathEl, Point, Rect, Vec2,
};
use serde::{Deserialize, Serialize};

/// The most nodes a frame takes.
pub const MAX_NODES: usize = 96;

/// How an object is mapped into its envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mapping {
    /// Every point blends the four sides, the nodes of each side spread
    /// evenly along it.
    #[default]
    Putty,
    /// As Putty, with each side measured by its length: the object's
    /// edges spread evenly along the frame however its nodes are spaced.
    Original,
    /// Rows stay level: the object fills the frame's height and each row
    /// spans the frame from its left edge to its right edge.
    Horizontal,
    /// Columns stay upright: the object fills the frame's width and each
    /// column spans the frame from its bottom edge to its top edge.
    Vertical,
}

impl Mapping {
    /// In the order the lists show them.
    pub const ALL: [Mapping; 4] = [
        Mapping::Horizontal,
        Mapping::Original,
        Mapping::Putty,
        Mapping::Vertical,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Mapping::Putty => "putty",
            Mapping::Original => "original",
            Mapping::Horizontal => "horizontal",
            Mapping::Vertical => "vertical",
        }
    }

    /// The mapping named `id`; Putty for anything else.
    pub fn from_id(id: &str) -> Mapping {
        Mapping::ALL
            .into_iter()
            .find(|m| m.id() == id)
            .unwrap_or_default()
    }
}

/// How moving a node reshapes the sides it is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// The sides run straight from node to node.
    StraightLine,
    /// Each side is one smooth arc through its nodes.
    SingleArc,
    /// Each segment leaves and reaches its nodes along the side, so a node
    /// moved off the side makes an S on each segment it ends.
    DoubleArc,
    /// The node moves with its handles and nothing else changes; handles
    /// move on their own.
    #[default]
    Unconstrained,
}

impl Mode {
    pub const ALL: [Mode; 4] = [
        Mode::StraightLine,
        Mode::SingleArc,
        Mode::DoubleArc,
        Mode::Unconstrained,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Mode::StraightLine => "straight_line",
            Mode::SingleArc => "single_arc",
            Mode::DoubleArc => "double_arc",
            Mode::Unconstrained => "unconstrained",
        }
    }

    /// The mode named `id`; Unconstrained for anything else.
    pub fn from_id(id: &str) -> Mode {
        Mode::ALL
            .into_iter()
            .find(|m| m.id() == id)
            .unwrap_or_default()
    }
}

/// The eight nodes of a fresh frame around `b`: the corners and the middle
/// of each side, from the bottom-left corner: BL, BM, BR, RM, TR, TM, TL,
/// LM.
pub fn default_nodes(b: Rect) -> Vec<Point> {
    vec![
        Point::new(b.x0, b.y0),
        Point::new(b.center().x, b.y0),
        Point::new(b.x1, b.y0),
        Point::new(b.x1, b.center().y),
        Point::new(b.x1, b.y1),
        Point::new(b.center().x, b.y1),
        Point::new(b.x0, b.y1),
        Point::new(b.x0, b.center().y),
    ]
}

fn finite(p: &Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// An envelope's frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub nodes: Vec<Point>,
    /// The handles of the segment from node `i` to node `i + 1` (the last
    /// one back to node 0).
    pub handles: Vec<[Point; 2]>,
    /// The corner nodes: bottom-left, bottom-right, top-right, top-left,
    /// in increasing order.
    pub corners: [usize; 4],
}

impl Frame {
    /// The frame around a box: its corners and the middle of each side,
    /// the sides straight. Mapping through it leaves an object as it is.
    pub fn around(b: Rect) -> Frame {
        let mut f = Frame {
            nodes: default_nodes(b),
            handles: Vec::new(),
            corners: [0, 2, 4, 6],
        };
        f.handles = (0..f.nodes.len()).map(|i| f.straight_handles(i)).collect();
        f
    }

    /// The frame an envelope stores. Without handles (envelopes written
    /// before handles existed) each side is the arc through its nodes;
    /// corners that are not four increasing node numbers fall back to the
    /// nodes a quarter of the way round each. None with fewer than four
    /// nodes, more than [`MAX_NODES`] or a node that is not a number.
    pub fn from_parts(nodes: &[Point], handles: &[[Point; 2]], corners: &[usize]) -> Option<Frame> {
        let n = nodes.len();
        if !(4..=MAX_NODES).contains(&n) || !nodes.iter().all(finite) {
            return None;
        }
        let corners = match corners {
            &[a, b, c, d] if a < b && b < c && c < d && d < n => [a, b, c, d],
            _ => [0, n / 4, n / 2, 3 * n / 4],
        };
        let mut f = Frame {
            nodes: nodes.to_vec(),
            handles: Vec::new(),
            corners,
        };
        if handles.len() == n && handles.iter().flatten().all(finite) {
            f.handles = handles.to_vec();
        } else {
            f.handles = (0..n).map(|i| f.straight_handles(i)).collect();
            for side in 0..4 {
                f.reshape_side(side, Mode::SingleArc);
            }
        }
        Some(f)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// One pair of handles per node and corners that are nodes, whatever
    /// was put in the fields.
    fn fix(&mut self) {
        let n = self.nodes.len();
        self.handles.truncate(n);
        while self.handles.len() < n {
            let h = self.straight_handles(self.handles.len());
            self.handles.push(h);
        }
        let [a, b, c, d] = self.corners;
        if !(a < b && b < c && c < d && d < n) {
            self.corners = [0, n / 4, n / 2, 3 * n / 4];
        }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    fn node(&self, i: usize) -> Point {
        let n = self.nodes.len().max(1);
        self.nodes.get(i % n).copied().unwrap_or_default()
    }

    /// The segment from node `i` to the next.
    pub fn segment(&self, i: usize) -> CubicBez {
        let n = self.nodes.len().max(1);
        let [h1, h2] = self
            .handles
            .get(i % n)
            .copied()
            .unwrap_or_else(|| self.straight_handles(i));
        CubicBez::new(self.node(i), h1, h2, self.node(i + 1))
    }

    /// Handles a third and two thirds of the way along the chord.
    fn straight_handles(&self, i: usize) -> [Point; 2] {
        let (a, b) = (self.node(i), self.node(i + 1));
        [a.lerp(b, 1.0 / 3.0), a.lerp(b, 2.0 / 3.0)]
    }

    pub fn is_corner(&self, i: usize) -> bool {
        self.corners.contains(&i)
    }

    /// The nodes of side `s` (0 bottom, 1 right, 2 top, 3 left) in node
    /// order, both corners included.
    pub fn side_nodes(&self, s: usize) -> Vec<usize> {
        let n = self.nodes.len();
        if n == 0 {
            return Vec::new();
        }
        let from = self.corners[s % 4];
        let to = self.corners[(s + 1) % 4];
        let count = (to + n - from) % n;
        (0..=count).map(|k| (from + k) % n).collect()
    }

    /// The segments of side `s` in node order.
    pub fn side_segments(&self, s: usize) -> Vec<usize> {
        let nodes = self.side_nodes(s);
        nodes[..nodes.len().saturating_sub(1)].to_vec()
    }

    /// The sides node `i` is on: two for a corner, one for any other.
    pub fn sides_of(&self, i: usize) -> Vec<usize> {
        (0..4)
            .filter(|s| self.side_nodes(*s).contains(&i))
            .collect()
    }

    /// Give side `s` the shape `mode` gives it (Unconstrained leaves it).
    pub fn reshape_side(&mut self, s: usize, mode: Mode) {
        self.fix();
        let nodes = self.side_nodes(s);
        if nodes.len() < 2 {
            return;
        }
        let pts: Vec<Point> = nodes.iter().map(|i| self.node(*i)).collect();
        let m = pts.len() - 1;
        let mut set = |j: usize, h: [Point; 2]| {
            if let Some(slot) = nodes.get(j).and_then(|i| self.handles.get_mut(*i)) {
                *slot = h;
            }
        };
        match mode {
            Mode::Unconstrained => {}
            Mode::StraightLine => {
                for j in 0..m {
                    let (a, b) = (pts[j], pts[j + 1]);
                    set(j, [a.lerp(b, 1.0 / 3.0), a.lerp(b, 2.0 / 3.0)]);
                }
            }
            Mode::SingleArc => {
                // Tangents (per segment): Catmull-Rom inside, and at the
                // ends the ones that make three nodes one quadratic arc.
                let mut t = vec![Vec2::ZERO; m + 1];
                for (i, ti) in t.iter_mut().enumerate().take(m).skip(1) {
                    *ti = (pts[i + 1] - pts[i - 1]) * 0.5;
                }
                if m == 1 {
                    t[0] = pts[1] - pts[0];
                    t[1] = t[0];
                } else {
                    t[0] = (pts[1] - pts[0]) * 2.0 - t[1];
                    t[m] = (pts[m] - pts[m - 1]) * 2.0 - t[m - 1];
                }
                for j in 0..m {
                    set(j, [pts[j] + t[j] / 3.0, pts[j + 1] - t[j + 1] / 3.0]);
                }
            }
            Mode::DoubleArc => {
                // Along the side's own direction: across for the bottom and
                // the top, up for the right and the left.
                let axis = if s.is_multiple_of(2) {
                    Vec2::new(1.0, 0.0)
                } else {
                    Vec2::new(0.0, 1.0)
                };
                for j in 0..m {
                    let (a, b) = (pts[j], pts[j + 1]);
                    let l = (b - a).dot(axis) / 3.0;
                    set(j, [a + axis * l, b - axis * l]);
                }
            }
        }
    }

    /// Move node `i` by `d` the way `mode` moves it.
    pub fn move_node(&mut self, i: usize, d: Vec2, mode: Mode) {
        self.fix();
        let n = self.nodes.len();
        if i >= n || !(d.x.is_finite() && d.y.is_finite()) {
            return;
        }
        if let Some(p) = self.nodes.get_mut(i) {
            *p += d;
        }
        if mode == Mode::Unconstrained {
            if let Some(h) = self.handles.get_mut(i) {
                h[0] += d;
            }
            if let Some(h) = self.handles.get_mut((i + n - 1) % n) {
                h[1] += d;
            }
            return;
        }
        for s in self.sides_of(i) {
            self.reshape_side(s, mode);
        }
    }

    /// Move handle `end` (0 at the segment's start, 1 at its end) of
    /// segment `seg` by `d`.
    pub fn move_handle(&mut self, seg: usize, end: usize, d: Vec2) {
        self.fix();
        if let Some(h) = self.handles.get_mut(seg).and_then(|h| h.get_mut(end)) {
            if d.x.is_finite() && d.y.is_finite() {
                *h += d;
            }
        }
    }

    /// Split segment `seg` at `t` (0..1) with a new node where the curve
    /// was, keeping the frame's shape. Returns the new node's number.
    pub fn add_node(&mut self, seg: usize, t: f64) -> Option<usize> {
        self.fix();
        let n = self.nodes.len();
        if seg >= n || n >= MAX_NODES || !t.is_finite() {
            return None;
        }
        let t = t.clamp(0.05, 0.95);
        let c = self.segment(seg);
        let (a, b) = (c.subsegment(0.0..t), c.subsegment(t..1.0));
        let at = seg + 1;
        *self.handles.get_mut(seg)? = [a.p1, a.p2];
        self.nodes.insert(at, b.p0);
        self.handles.insert(at, [b.p1, b.p2]);
        for c in self.corners.iter_mut() {
            if *c >= at {
                *c += 1;
            }
        }
        Some(at)
    }

    /// Remove node `i` (not a corner); the segments either side become one
    /// keeping their outer handles. False when it cannot go.
    pub fn remove_node(&mut self, i: usize) -> bool {
        self.fix();
        let n = self.nodes.len();
        if i >= n || n <= 4 || self.is_corner(i) {
            return false;
        }
        let prev = (i + n - 1) % n;
        let Some(out) = self.handles.get(i).map(|h| h[1]) else {
            return false;
        };
        if let Some(h) = self.handles.get_mut(prev) {
            h[1] = out;
        }
        self.nodes.remove(i);
        self.handles.remove(i);
        for c in self.corners.iter_mut() {
            if *c > i {
                *c -= 1;
            }
        }
        true
    }

    /// The segment nearest `p`: (segment, parameter, distance).
    pub fn nearest_segment(&self, p: Point) -> Option<(usize, f64, f64)> {
        (0..self.nodes.len())
            .map(|i| {
                let near = self.segment(i).nearest(p, 1e-6);
                (i, near.t, near.distance_sq.sqrt())
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
    }

    /// The frame's outline.
    pub fn to_path(&self) -> BezPath {
        let mut path = BezPath::new();
        if self.nodes.is_empty() {
            return path;
        }
        path.move_to(self.node(0));
        for i in 0..self.nodes.len() {
            let c = self.segment(i);
            path.curve_to(c.p1, c.p2, c.p3);
        }
        path.close_path();
        path
    }

    /// The frame shaped like a closed outline (its largest closed part),
    /// so an object mapped into it fills that shape (#993). The corners
    /// are the outline's points nearest the corners of its bounding box;
    /// a node goes in where such a point falls inside a segment. None when
    /// the outline has no closed part with an area, more nodes than
    /// [`MAX_NODES`], or corners that do not run round it in order.
    pub fn from_outline(path: &BezPath) -> Option<Frame> {
        use crate::geometry::Shape as _;
        // The closed parts as cubic segments.
        let mut parts: Vec<Vec<CubicBez>> = Vec::new();
        let mut cur: Vec<CubicBez> = Vec::new();
        let (mut start, mut last) = (Point::ZERO, Point::ZERO);
        let mut closed = false;
        let mut finish = |cur: &mut Vec<CubicBez>, closed: bool, start: Point, last: Point| {
            if closed || (!cur.is_empty() && (last - start).hypot() < 1e-9) {
                if (last - start).hypot() > 1e-9 {
                    cur.push(CubicBez::new(
                        last,
                        last.lerp(start, 1.0 / 3.0),
                        last.lerp(start, 2.0 / 3.0),
                        start,
                    ));
                }
                if !cur.is_empty() {
                    parts.push(std::mem::take(cur));
                }
            }
            cur.clear();
        };
        for el in path.elements() {
            match *el {
                PathEl::MoveTo(p) => {
                    finish(&mut cur, closed, start, last);
                    closed = false;
                    start = p;
                    last = p;
                }
                PathEl::LineTo(p) => {
                    if (p - last).hypot() > 1e-12 {
                        cur.push(CubicBez::new(
                            last,
                            last.lerp(p, 1.0 / 3.0),
                            last.lerp(p, 2.0 / 3.0),
                            p,
                        ));
                    }
                    last = p;
                }
                PathEl::QuadTo(c, p) => {
                    cur.push(kurbo::QuadBez::new(last, c, p).raise());
                    last = p;
                }
                PathEl::CurveTo(a, b, p) => {
                    cur.push(CubicBez::new(last, a, b, p));
                    last = p;
                }
                PathEl::ClosePath => {
                    closed = true;
                    finish(&mut cur, closed, start, last);
                    closed = false;
                    last = start;
                }
            }
        }
        finish(&mut cur, closed, start, last);
        let area = |segs: &[CubicBez]| {
            let mut p = BezPath::new();
            if let Some(f) = segs.first() {
                p.move_to(f.p0);
            }
            for c in segs {
                p.curve_to(c.p1, c.p2, c.p3);
            }
            p.close_path();
            p.area()
        };
        let mut segs = parts
            .into_iter()
            .map(|s| (area(&s), s))
            .filter(|(a, _)| a.is_finite() && a.abs() > 1e-9)
            .max_by(|a, b| a.0.abs().total_cmp(&b.0.abs()))?;
        // Anticlockwise (y up), as the frame runs.
        if segs.0 < 0.0 {
            segs.1 = segs
                .1
                .iter()
                .rev()
                .map(|c| CubicBez::new(c.p3, c.p2, c.p1, c.p0))
                .collect();
        }
        let mut segs = segs.1;
        let mut outline = BezPath::new();
        if let Some(f) = segs.first() {
            outline.move_to(f.p0);
        }
        for c in &segs {
            outline.curve_to(c.p1, c.p2, c.p3);
        }
        let b = outline.bounding_box();
        if b.width() < 1e-9 || b.height() < 1e-9 {
            return None;
        }
        // Each corner of the box: the nearest point of the outline, as a
        // node (the segment it falls in split there).
        let corner_at = |q: Point, segs: &mut Vec<CubicBez>| -> Option<usize> {
            let (i, t, _) = segs
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let n = c.nearest(q, 1e-9);
                    (i, n.t, n.distance_sq)
                })
                .min_by(|a, b| a.2.total_cmp(&b.2))?;
            let c = *segs.get(i)?;
            let near = |p: Point| (p - c.eval(t)).hypot() < 1e-6 * b.width().max(b.height());
            if t < 1e-6 || near(c.p0) {
                Some(i)
            } else if t > 1.0 - 1e-6 || near(c.p3) {
                Some((i + 1) % segs.len())
            } else {
                *segs.get_mut(i)? = c.subsegment(0.0..t);
                segs.insert(i + 1, c.subsegment(t..1.0));
                Some(i + 1)
            }
        };
        let targets = [
            Point::new(b.x0, b.y0),
            Point::new(b.x1, b.y0),
            Point::new(b.x1, b.y1),
            Point::new(b.x0, b.y1),
        ];
        let mut points = Vec::new();
        for q in targets {
            let i = corner_at(q, &mut segs)?;
            points.push(segs.get(i)?.p0);
        }
        // Start at the bottom-left corner. Two box corners at one point
        // (the apex of a triangle) become two nodes there, joined by a
        // side of no length.
        let first = segs.iter().position(|c| c.p0 == points[0])?;
        segs.rotate_left(first);
        let mut corners = [0usize; 4];
        for k in 1..4 {
            let prev = corners[k - 1];
            let found = segs
                .iter()
                .enumerate()
                .skip(prev + 1)
                .find(|(_, c)| c.p0 == points[k])
                .map(|(i, _)| i);
            corners[k] = match found {
                Some(i) => i,
                None if points[k] == points[k - 1] => {
                    let p = points[k];
                    segs.insert(prev, CubicBez::new(p, p, p, p));
                    prev + 1
                }
                None => return None,
            };
        }
        if segs.len() > MAX_NODES {
            return None;
        }
        Some(Frame {
            nodes: segs.iter().map(|c| c.p0).collect(),
            handles: segs.iter().map(|c| [c.p1, c.p2]).collect(),
            corners,
        })
    }

    /// The frame moved through `m` (nodes and handles alike).
    pub fn transformed(&self, m: crate::geometry::Affine) -> Frame {
        Frame {
            nodes: self.nodes.iter().map(|p| m * *p).collect(),
            handles: self.handles.iter().map(|[a, b]| [m * *a, m * *b]).collect(),
            corners: self.corners,
        }
    }

    /// The frame's bounding box (of its outline).
    pub fn bounds(&self) -> Rect {
        use crate::geometry::Shape as _;
        self.to_path().bounding_box()
    }
}

/// One side of the frame as a curve from 0 to 1, evenly by segment or by
/// length.
struct Side {
    segs: Vec<CubicBez>,
    /// (fraction of the length, side parameter), for measuring by length.
    by_length: Option<Vec<(f64, f64)>>,
}

impl Side {
    fn new(frame: &Frame, s: usize, reversed: bool, by_length: bool) -> Side {
        let mut segs: Vec<CubicBez> = frame
            .side_segments(s)
            .into_iter()
            .map(|i| frame.segment(i))
            .collect();
        if reversed {
            segs.reverse();
            for c in segs.iter_mut() {
                *c = CubicBez::new(c.p3, c.p2, c.p1, c.p0);
            }
        }
        let mut side = Side {
            segs,
            by_length: None,
        };
        if by_length && !side.segs.is_empty() {
            const STEPS: usize = 32;
            let total_steps = STEPS * side.segs.len();
            let mut table = Vec::with_capacity(total_steps + 1);
            let mut len = 0.0;
            let mut last = side.at_param(0.0);
            table.push((0.0, 0.0));
            for k in 1..=total_steps {
                let u = k as f64 / total_steps as f64;
                let p = side.at_param(u);
                len += (p - last).hypot();
                last = p;
                table.push((len, u));
            }
            if len > 1e-12 {
                for e in table.iter_mut() {
                    e.0 /= len;
                }
                side.by_length = Some(table);
            }
        }
        side
    }

    /// The point at side parameter `u`, each segment taking an equal part.
    fn at_param(&self, u: f64) -> Point {
        let m = self.segs.len();
        if m == 0 {
            return Point::ZERO;
        }
        let x = u.clamp(0.0, 1.0) * m as f64;
        let j = (x.floor() as usize).min(m - 1);
        self.segs[j].eval(x - j as f64)
    }

    fn at(&self, u: f64) -> Point {
        match &self.by_length {
            Some(table) => {
                let f = u.clamp(0.0, 1.0);
                let i = table.partition_point(|e| e.0 < f);
                let param = match (i.checked_sub(1).and_then(|k| table.get(k)), table.get(i)) {
                    (Some(&(l0, p0)), Some(&(l1, p1))) if l1 - l0 > 1e-12 => {
                        p0 + (p1 - p0) * (f - l0) / (l1 - l0)
                    }
                    (_, Some(&(_, p))) | (Some(&(_, p)), None) => p,
                    (None, None) => f,
                };
                self.at_param(param)
            }
            None => self.at_param(u),
        }
    }
}

/// The roots of `c0 + c1 t + c2 t^2 + c3 t^3`, each polished with a
/// Newton step. A cubic term that is only rounding (a straight side's
/// handles a third of the way along make none) is left out: solved as a
/// cubic it gave roots far off.
fn cubic_roots(c0: f64, c1: f64, c2: f64, c3: f64) -> impl Iterator<Item = f64> {
    let roots: Vec<f64> = if c3.abs() <= 1e-9 * (c0.abs() + c1.abs() + c2.abs()) {
        kurbo::common::solve_quadratic(c0, c1, c2).to_vec()
    } else {
        kurbo::common::solve_cubic(c0, c1, c2, c3).to_vec()
    };
    roots.into_iter().map(move |t| {
        let f = c0 + t * (c1 + t * (c2 + t * c3));
        let d = c1 + t * (2.0 * c2 + t * 3.0 * c3);
        let polished = t - f / d;
        if polished.is_finite() && (polished - t).abs() < 1e-3 {
            polished
        } else {
            t
        }
    })
}

/// For each of `ROWS` levels across the frame (rows when `across_y`,
/// columns otherwise), where the frame's outline starts and ends there.
struct Spans {
    lo: f64,
    hi: f64,
    spans: Vec<(f64, f64)>,
}

const ROWS: usize = 1025;

impl Spans {
    fn new(frame: &Frame, rows: bool) -> Option<Spans> {
        // Rows across y read x; columns across x read y.
        let along = |p: Point| if rows { p.y } else { p.x };
        let across = |p: Point| if rows { p.x } else { p.y };
        let segs: Vec<CubicBez> = (0..frame.len()).map(|i| frame.segment(i)).collect();
        if segs.len() < 2 {
            return None;
        }
        use crate::geometry::Shape as _;
        let bb = frame.to_path().bounding_box();
        let (lo, hi) = if rows { (bb.y0, bb.y1) } else { (bb.x0, bb.x1) };
        if !(lo.is_finite() && hi.is_finite()) || hi - lo <= 1e-9 {
            return None;
        }
        // Each segment's height (or width) as a cubic in its parameter.
        let polys: Vec<[f64; 4]> = segs
            .iter()
            .map(|c| {
                let (a, b, d, e) = (along(c.p0), along(c.p1), along(c.p2), along(c.p3));
                [
                    a,
                    3.0 * (b - a),
                    3.0 * (a - 2.0 * b + d),
                    e - a + 3.0 * (b - d),
                ]
            })
            .collect();
        let eps = 1e-4;
        let mut spans: Vec<Option<(f64, f64)>> = (0..ROWS)
            .map(|k| {
                let f = eps + (1.0 - 2.0 * eps) * k as f64 / (ROWS - 1) as f64;
                let level = lo + (hi - lo) * f;
                let mut min = f64::INFINITY;
                let mut max = f64::NEG_INFINITY;
                // Where the outline itself crosses the level, not a
                // polygon through it: the edges of the mapped object
                // follow the frame's curves without facets (#881).
                for (c, [c0, c1, c2, c3]) in segs.iter().zip(&polys) {
                    for t in cubic_roots(c0 - level, *c1, *c2, *c3) {
                        if (-1e-9..=1.0 + 1e-9).contains(&t) {
                            let x = across(c.eval(t.clamp(0.0, 1.0)));
                            if x.is_finite() {
                                min = min.min(x);
                                max = max.max(x);
                            }
                        }
                    }
                }
                (min <= max).then_some((min, max))
            })
            .collect();
        // Levels the outline missed take their nearest neighbour's span.
        let first = spans.iter().position(Option::is_some)?;
        let mut last = spans[first];
        for s in spans.iter_mut() {
            match s {
                Some(v) => last = Some(*v),
                None => *s = last,
            }
        }
        Some(Spans {
            lo,
            hi,
            spans: spans.into_iter().map(|s| s.unwrap_or_default()).collect(),
        })
    }

    /// (level, start, end) at fraction `f` of the way across.
    fn at(&self, f: f64) -> (f64, f64, f64) {
        let f = f.clamp(0.0, 1.0);
        let r = f * (ROWS - 1) as f64;
        let k = (r.floor() as usize).min(ROWS - 2);
        let w = r - k as f64;
        let (a, b) = (self.spans[k], self.spans[k + 1]);
        (
            self.lo + (self.hi - self.lo) * f,
            a.0 + (b.0 - a.0) * w,
            a.1 + (b.1 - a.1) * w,
        )
    }
}

/// Map `path` through `frame`: the path's bounding box goes onto the
/// frame the way `mapping` says. With `keep_lines` only the nodes and
/// control points move, so straight segments stay straight; otherwise
/// the path is divided finely enough to bend.
pub fn apply(path: &BezPath, frame: &Frame, keep_lines: bool, mapping: Mapping) -> BezPath {
    use crate::geometry::Shape as _;
    apply_in(path, path.bounding_box(), frame, keep_lines, mapping)
}

/// As [`apply`], with the box `b` mapped onto the frame instead of the
/// path's own bounding box (objects that share one envelope, each mapped
/// from the box of them all).
pub fn apply_in(
    path: &BezPath,
    b: Rect,
    frame: &Frame,
    keep_lines: bool,
    mapping: Mapping,
) -> BezPath {
    if b.width() < 1e-9 || b.height() < 1e-9 {
        return path.clone();
    }
    let unit = |p: Point| {
        (
            ((p.x - b.x0) / b.width()).clamp(0.0, 1.0),
            ((p.y - b.y0) / b.height()).clamp(0.0, 1.0),
        )
    };
    let map: Box<dyn Fn(Point) -> Point> = match mapping {
        Mapping::Putty | Mapping::Original => {
            let by_length = mapping == Mapping::Original;
            let bottom = Side::new(frame, 0, false, by_length);
            let right = Side::new(frame, 1, false, by_length);
            // The top runs from the top-right corner in node order and the
            // left from the top-left: both turned round.
            let top = Side::new(frame, 2, true, by_length);
            let left = Side::new(frame, 3, true, by_length);
            let [c0, c1, c2, c3] = frame.corners;
            let (p00, p10, p11, p01) = (
                frame.node(c0),
                frame.node(c1),
                frame.node(c2),
                frame.node(c3),
            );
            Box::new(move |p: Point| {
                let (u, v) = unit(p);
                let (bu, tu, lv, rv) = (bottom.at(u), top.at(u), left.at(v), right.at(v));
                // The Coons patch: the sides blended, less the corners
                // counted twice.
                let sides = (1.0 - v) * bu.to_vec2()
                    + v * tu.to_vec2()
                    + (1.0 - u) * lv.to_vec2()
                    + u * rv.to_vec2();
                let corners = (1.0 - u) * (1.0 - v) * p00.to_vec2()
                    + u * (1.0 - v) * p10.to_vec2()
                    + (1.0 - u) * v * p01.to_vec2()
                    + u * v * p11.to_vec2();
                (sides - corners).to_point()
            })
        }
        Mapping::Horizontal => {
            let Some(rows) = Spans::new(frame, true) else {
                return path.clone();
            };
            Box::new(move |p: Point| {
                let (u, v) = unit(p);
                let (y, x0, x1) = rows.at(v);
                Point::new(x0 + (x1 - x0) * u, y)
            })
        }
        Mapping::Vertical => {
            let Some(cols) = Spans::new(frame, false) else {
                return path.clone();
            };
            Box::new(move |p: Point| {
                let (u, v) = unit(p);
                let (x, y0, y1) = cols.at(u);
                Point::new(x, y0 + (y1 - y0) * v)
            })
        }
    };
    if keep_lines {
        let mut out = BezPath::new();
        for el in path.elements() {
            out.push(match *el {
                PathEl::MoveTo(p) => PathEl::MoveTo(map(p)),
                PathEl::LineTo(p) => PathEl::LineTo(map(p)),
                PathEl::QuadTo(a, p) => PathEl::QuadTo(map(a), map(p)),
                PathEl::CurveTo(a, c, p) => PathEl::CurveTo(map(a), map(c), map(p)),
                PathEl::ClosePath => PathEl::ClosePath,
            });
        }
        return out;
    }
    crate::live::warp(path, b, map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Shape as _;

    fn square() -> BezPath {
        Rect::new(0.0, 0.0, 40.0, 20.0).to_path(0.01)
    }

    fn close(a: Point, b: Point) -> bool {
        (a - b).hypot() < 1e-6
    }

    /// A frame made from an outline follows it (#993): a rectangle drawn
    /// either way round gives its own box, a circle its circle with the
    /// corners at the diagonals, a triangle is filled by the box; an open
    /// line gives none.
    #[test]
    fn frames_are_made_from_outlines() {
        // A rectangle, clockwise: the frame of its box, mapping changes
        // nothing.
        let mut cw = BezPath::new();
        cw.move_to((0.0, 20.0));
        cw.line_to((40.0, 20.0));
        cw.line_to((40.0, 0.0));
        cw.line_to((0.0, 0.0));
        cw.close_path();
        let f = Frame::from_outline(&cw).expect("a frame");
        assert_eq!(f.nodes.len(), 4);
        assert!(close(f.nodes[f.corners[0]], Point::new(0.0, 0.0)));
        assert!(close(f.nodes[f.corners[1]], Point::new(40.0, 0.0)));
        assert!(close(f.nodes[f.corners[2]], Point::new(40.0, 20.0)));
        assert!(close(f.nodes[f.corners[3]], Point::new(0.0, 20.0)));
        let mapped = apply(&square(), &f, false, Mapping::Putty);
        assert!((mapped.area().abs() - 800.0).abs() < 1e-3);

        // A circle: the object's box fills the circle.
        let circle = crate::geometry::Circle::new((50.0, 50.0), 10.0).to_path(0.01);
        let f = Frame::from_outline(&circle).expect("a frame");
        let d = 10.0 / 2f64.sqrt();
        let near = |a: Point, b: Point| (a - b).hypot() < 0.01;
        assert!(near(f.nodes[f.corners[0]], Point::new(50.0 - d, 50.0 - d)));
        assert!(near(f.nodes[f.corners[2]], Point::new(50.0 + d, 50.0 + d)));
        let box40 = Rect::new(50.0 - d, 50.0 - d, 50.0 + d, 50.0 + d).to_path(0.01);
        let round = apply(&box40, &f, false, Mapping::Putty);
        let area = std::f64::consts::PI * 100.0;
        assert!(
            (round.area().abs() - area).abs() < area * 0.01,
            "{}",
            round.area()
        );
        assert!((round.bounding_box().width() - 20.0).abs() < 0.05);

        // A triangle: its box fills it.
        let mut tri = BezPath::new();
        tri.move_to((0.0, 0.0));
        tri.line_to((30.0, 0.0));
        tri.line_to((15.0, 30.0));
        tri.close_path();
        let f = Frame::from_outline(&tri).expect("a frame");
        let mapped = apply(
            &Rect::new(0.0, 0.0, 30.0, 30.0).to_path(0.01),
            &f,
            false,
            Mapping::Putty,
        );
        assert!(
            (mapped.area().abs() - 450.0).abs() < 1.0,
            "{}",
            mapped.area()
        );

        // An open line, or nothing: no frame.
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((10.0, 5.0));
        assert!(Frame::from_outline(&line).is_none());
        assert!(Frame::from_outline(&BezPath::new()).is_none());
    }

    /// A fresh frame leaves the object where it is, whatever the mapping.
    #[test]
    fn a_fresh_frame_changes_nothing() {
        let path = square();
        let f = Frame::around(path.bounding_box());
        for mapping in Mapping::ALL {
            for keep in [false, true] {
                let out = apply(&path, &f, keep, mapping);
                let (a, b) = (out.bounding_box(), path.bounding_box());
                for (x, y) in [(a.x0, b.x0), (a.y0, b.y0), (a.x1, b.x1), (a.y1, b.y1)] {
                    assert!((x - y).abs() < 1e-3, "{mapping:?} {keep}: {a:?} != {b:?}");
                }
            }
        }
    }

    /// Envelopes without handles (written before they existed) are single
    /// arcs: the side through its three nodes is the quadratic through them.
    #[test]
    fn older_envelopes_are_single_arcs() {
        let b = Rect::new(0.0, 0.0, 40.0, 20.0);
        let mut nodes = default_nodes(b);
        nodes[5].y += 10.0;
        let f = Frame::from_parts(&nodes, &[], &[]).expect("frame");
        assert_eq!(f.corners, [0, 2, 4, 6]);
        // The top's middle node is on the curve at the middle.
        let top = Side::new(&f, 2, true, false);
        assert!(close(top.at(0.5), Point::new(20.0, 30.0)));
        // A quarter of the way: the quadratic (0,20) (20,40) (40,20) at 1/4.
        let q = |t: f64| {
            let ctrl = Point::new(20.0, 40.0);
            let a = Point::new(0.0, 20.0);
            let c = Point::new(40.0, 20.0);
            let u = 1.0 - t;
            Point::new(
                u * u * a.x + 2.0 * u * t * ctrl.x + t * t * c.x,
                u * u * a.y + 2.0 * u * t * ctrl.y + t * t * c.y,
            )
        };
        assert!(close(top.at(0.25), q(0.25)), "{:?}", top.at(0.25));
        // The object's top centre follows.
        let out = apply(&square(), &f, false, Mapping::Putty);
        assert!((out.bounding_box().y1 - 30.0).abs() < 0.05);
        // Too few nodes or a node that is not a number: no frame.
        assert!(Frame::from_parts(&nodes[..3], &[], &[]).is_none());
        let mut bad = nodes.clone();
        bad[2].x = f64::NAN;
        assert!(Frame::from_parts(&bad, &[], &[]).is_none());
    }

    /// Each mode reshapes the sides a moved node is on.
    #[test]
    fn the_modes_shape_the_sides_a_node_moves() {
        let b = Rect::new(0.0, 0.0, 40.0, 20.0);
        let up = Vec2::new(0.0, 10.0);
        // Straight line: the top runs straight to the raised middle.
        let mut f = Frame::around(b);
        f.move_node(5, up, Mode::StraightLine);
        let quarter = f.segment(4).eval(0.5);
        assert!(close(quarter, Point::new(30.0, 25.0)), "{quarter:?}");
        // Single arc: one smooth arc, level at the raised middle.
        let mut f = Frame::around(b);
        f.move_node(5, up, Mode::SingleArc);
        let (into, out) = (f.handles[4][1], f.handles[5][0]);
        assert!((into.y - 30.0).abs() < 1e-9 && (out.y - 30.0).abs() < 1e-9);
        assert!(f.segment(4).eval(0.5).y > 25.0);
        // Double arc: level at both ends of each segment, an S between.
        let mut f = Frame::around(b);
        f.move_node(5, up, Mode::DoubleArc);
        let seg = f.segment(4);
        assert!((seg.p1.y - 20.0).abs() < 1e-9 && (seg.p2.y - 30.0).abs() < 1e-9);
        assert!(close(seg.eval(0.5), Point::new(30.0, 25.0)));
        assert!(seg.eval(0.25).y < 25.0 - 2.0);
        // Unconstrained: the node takes its handles along, nothing else moves.
        let mut f = Frame::around(b);
        let before = f.clone();
        f.move_node(5, up, Mode::Unconstrained);
        assert_eq!(f.handles[4][1], before.handles[4][1] + up);
        assert_eq!(f.handles[5][0], before.handles[5][0] + up);
        assert_eq!(f.handles[4][0], before.handles[4][0]);
        // A corner moved in a mode reshapes both its sides.
        let mut f = Frame::around(b);
        f.move_node(4, up, Mode::StraightLine);
        assert_eq!(f.sides_of(4), vec![1, 2]);
        assert!(close(f.segment(3).eval(0.5), Point::new(40.0, 20.0)));
        assert!(close(f.segment(4).eval(0.5), Point::new(30.0, 25.0)));
        // A handle moves alone.
        let mut f = Frame::around(b);
        f.move_handle(0, 1, up);
        assert_eq!(f.handles[0][1], Frame::around(b).handles[0][1] + up);
    }

    /// Nodes come and go keeping the frame's shape, corners stay.
    #[test]
    fn nodes_are_added_and_removed() {
        let b = Rect::new(0.0, 0.0, 40.0, 20.0);
        let mut f = Frame::around(b);
        f.move_node(5, Vec2::new(0.0, 10.0), Mode::SingleArc);
        let on_curve = f.segment(4).eval(0.5);
        let at = f.add_node(4, 0.5).expect("added");
        assert_eq!(at, 5);
        assert_eq!(f.len(), 9);
        assert!(close(f.nodes[5], on_curve));
        assert_eq!(f.corners, [0, 2, 4, 7]);
        // The halves are the curve as it was.
        assert!(close(f.segment(4).eval(1.0), on_curve));
        // The last segment, back to the first node, takes a node too.
        let n = f.len();
        assert_eq!(f.add_node(n - 1, 0.5), Some(n));
        assert_eq!(f.corners, [0, 2, 4, 7]);
        // Corners stay; other nodes go down to four.
        assert!(!f.remove_node(4));
        assert!(f.remove_node(5));
        assert_eq!(f.corners, [0, 2, 4, 6]);
        let mut f = Frame::around(b);
        for _ in 0..4 {
            let i = (0..f.len())
                .find(|i| !f.is_corner(*i))
                .expect("a middle node");
            assert!(f.remove_node(i));
        }
        assert_eq!(f.len(), 4);
        assert_eq!(f.corners, [0, 1, 2, 3]);
        assert!(!f.remove_node(1));
        // The nearest segment and its parameter.
        let (seg, t, d) = f.nearest_segment(Point::new(20.0, -1.0)).expect("near");
        assert_eq!(seg, 0);
        assert!((t - 0.5).abs() < 1e-6 && (d - 1.0).abs() < 1e-6);
    }

    /// The points of `path`'s lines and the farthest any point of `truth`
    /// lies from them.
    fn farthest_from(path: &BezPath, truth: &[Point]) -> f64 {
        let mut segs: Vec<(Point, Point)> = Vec::new();
        let (mut at, mut start) = (Point::ZERO, Point::ZERO);
        for el in path.elements() {
            match *el {
                PathEl::MoveTo(p) => (at, start) = (p, p),
                PathEl::LineTo(p) => {
                    segs.push((at, p));
                    at = p;
                }
                PathEl::ClosePath => {
                    segs.push((at, start));
                    at = start;
                }
                _ => {}
            }
        }
        truth
            .iter()
            .map(|p| {
                segs.iter()
                    .map(|(a, b)| {
                        let ab = *b - *a;
                        let t = ((*p - *a).dot(ab) / ab.hypot2().max(1e-18)).clamp(0.0, 1.0);
                        (*p - (*a + ab * t)).hypot()
                    })
                    .fold(f64::MAX, f64::min)
            })
            .fold(0.0, f64::max)
    }

    /// The largest turn, in degrees, between one line of `path` and the
    /// next, leaving out the turns at `corners` (and the close).
    fn largest_turn(path: &BezPath, corners: &[Point]) -> f64 {
        let pts: Vec<Point> = path
            .elements()
            .iter()
            .filter_map(|el| match el {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => Some(*p),
                _ => None,
            })
            .collect();
        pts.windows(3)
            .filter(|w| corners.iter().all(|c| (w[1] - *c).hypot() > 1e-6))
            .map(|w| {
                let (a, b) = (w[1] - w[0], w[2] - w[1]);
                a.cross(b).atan2(a.dot(b)).abs().to_degrees()
            })
            .fold(0.0, f64::max)
    }

    /// The outline of `b` in `n` lines a side, for the mapping's own
    /// points through `keep_lines`.
    fn dense_box(b: Rect, n: usize) -> BezPath {
        let c = [
            Point::new(b.x0, b.y0),
            Point::new(b.x1, b.y0),
            Point::new(b.x1, b.y1),
            Point::new(b.x0, b.y1),
            Point::new(b.x0, b.y0),
        ];
        let mut p = BezPath::new();
        p.move_to(c[0]);
        for w in c.windows(2) {
            for k in 1..=n {
                p.line_to(w[0].lerp(w[1], k as f64 / n as f64));
            }
        }
        p.close_path();
        p
    }

    fn points_of(p: &BezPath) -> Vec<Point> {
        p.elements()
            .iter()
            .filter_map(|el| match el {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => Some(*p),
                _ => None,
            })
            .collect()
    }

    /// A frame bent hard (#881): the enveloped object follows the warp
    /// within 1/50 000 of its size and turns smoothly along its edges.
    /// Pieces of 1/64 of its size showed as facets when zoomed in (the
    /// report: a 78 x 86 mm rectangle, a corner dragged in, at 7000 %).
    #[test]
    fn a_hard_bend_is_drawn_without_facets() {
        // The top's middle node pulled far up and its handles drawn in:
        // the top rises from each corner in a tight turn.
        let b = Rect::new(0.0, 0.0, 40.0, 20.0);
        let mut arch = Frame::around(b);
        arch.move_node(5, Vec2::new(0.0, 40.0), Mode::Unconstrained);
        arch.move_handle(4, 1, Vec2::new(5.0, 0.0));
        arch.move_handle(5, 0, Vec2::new(-5.0, 0.0));
        arch.move_handle(4, 0, Vec2::new(0.0, 10.0));
        arch.move_handle(5, 1, Vec2::new(0.0, 10.0));
        // As in the report: the top-left corner dragged in and down, the
        // top swept down to it.
        let tall = Rect::new(0.0, 0.0, 78.0, 86.0);
        let mut corner = Frame::around(tall);
        corner.move_node(6, Vec2::new(8.0, -25.0), Mode::Unconstrained);
        corner.move_handle(5, 1, Vec2::new(20.0, -15.0));
        for (b, f) in [(b, arch), (tall, corner)] {
            let size = b.width().max(b.height());
            let truth = points_of(&apply(&dense_box(b, 4000), &f, true, Mapping::Putty));
            let out = apply(&b.to_path(0.01), &f, false, Mapping::Putty);
            let off = farthest_from(&out, &truth);
            let corners: Vec<Point> = f.corners.iter().map(|c| f.nodes[*c]).collect();
            let turn = largest_turn(&out, &corners);
            assert!(off < size * 2e-5, "{b:?}: {off} mm off the warp");
            assert!(turn < 2.5, "{b:?}: turns {turn} degrees at once");
        }
    }

    /// Horizontal and Vertical take the frame's own curves, not a
    /// polygon through them: the object's sides lie on the frame (#881).
    #[test]
    fn the_rows_follow_the_frame_curves() {
        let path = square();
        let b = path.bounding_box();
        let mut f = Frame::around(b);
        f.move_node(3, Vec2::new(25.0, 0.0), Mode::SingleArc);
        f.move_node(5, Vec2::new(0.0, 15.0), Mode::SingleArc);
        let dense = dense_box(b, 400);
        for mapping in [Mapping::Horizontal, Mapping::Vertical] {
            let out = apply(&dense, &f, true, mapping);
            // The box's left and right sides (Horizontal), or its bottom
            // and top (Vertical), land on the frame.
            let on_side = |p: &Point| match mapping {
                Mapping::Horizontal => p.x == b.x0 || p.x == b.x1,
                _ => p.y == b.y0 || p.y == b.y1,
            };
            let worst = points_of(&dense)
                .iter()
                .zip(points_of(&out))
                .filter(|(src, _)| on_side(src))
                .map(|(_, p)| {
                    use crate::geometry::ParamCurveNearest as _;
                    (0..f.len())
                        .map(|i| f.segment(i).nearest(p, 1e-9).distance_sq.sqrt())
                        .fold(f64::MAX, f64::min)
                })
                .fold(0.0, f64::max);
            assert!(worst < 0.02, "{mapping:?}: {worst} mm off the frame");
        }
    }

    /// The mappings: Putty and Original differ once a side's nodes are
    /// uneven; Horizontal keeps rows level; Vertical keeps columns upright.
    #[test]
    fn the_mappings_fill_the_frame_differently() {
        let path = square();
        let b = path.bounding_box();
        // The bottom's middle node slid right, the side still straight.
        let mut f = Frame::around(b);
        f.move_node(1, Vec2::new(10.0, 0.0), Mode::StraightLine);
        let mid = Point::new(20.0, 0.0);
        let mut probe = BezPath::new();
        probe.move_to(Point::new(0.0, 0.0));
        probe.line_to(mid);
        probe.line_to(Point::new(40.0, 20.0));
        let putty = apply(&probe, &f, true, Mapping::Putty);
        let original = apply(&probe, &f, true, Mapping::Original);
        let second = |p: &BezPath| match p.elements().get(1) {
            Some(PathEl::LineTo(q)) => *q,
            _ => Point::ZERO,
        };
        // Putty takes the bottom's middle to the moved node, Original to
        // the middle of the bottom's length.
        assert!(
            close(second(&putty), Point::new(30.0, 0.0)),
            "{:?}",
            second(&putty)
        );
        assert!(close(second(&original), mid), "{:?}", second(&original));
        // A frame whose right side bulges: Horizontal stretches the rows,
        // Vertical leaves every column's x where the box puts it.
        let mut f = Frame::around(b);
        f.move_node(3, Vec2::new(10.0, 0.0), Mode::SingleArc);
        let row = apply(&probe, &f, true, Mapping::Horizontal);
        let p = second(&row);
        assert!((p.y - 0.0).abs() < 1e-6);
        let col = apply(&probe, &f, true, Mapping::Vertical);
        let q = second(&col);
        assert!(q.x > 20.0 && q.x < 30.0, "{q:?}");
        // Mid-height on the right edge goes out to the bulge with Horizontal.
        let mut edge = BezPath::new();
        edge.move_to(Point::new(0.0, 0.0));
        edge.line_to(Point::new(40.0, 10.0));
        edge.line_to(Point::new(40.0, 20.0));
        let out = apply(&edge, &f, true, Mapping::Horizontal);
        match out.elements().get(1) {
            Some(PathEl::LineTo(q)) => assert!((q.x - 50.0).abs() < 0.2, "{q:?}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(Mapping::from_id("vertical"), Mapping::Vertical);
        assert_eq!(Mapping::from_id("no.such.key"), Mapping::Putty);
        assert_eq!(Mode::from_id("double_arc"), Mode::DoubleArc);
        assert_eq!(Mode::from_id("no.such.key"), Mode::Unconstrained);
    }
}
