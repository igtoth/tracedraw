//! Distortions of an outline about a centre: Push and Pull, Zipper and
//! Twister. They work on the path's own nodes and segments, so every
//! subpath stays a subpath and an open curve stays open.
//!
//! - Push and Pull moves each node along the line from the centre by
//!   `amount` percent of its distance (away for a positive amount, in for
//!   a negative one) and the control points of the segments the other way
//!   by the same share, so the segments bow against the nodes: pushed
//!   nodes give spikes with hollow sides, pulled nodes give petals.
//!   Straight segments become curves with their control points at a third
//!   and two thirds.
//! - Zipper puts `frequency` teeth on every segment: the segment is cut
//!   into `2 * frequency` equal steps of its parameter, the inner points
//!   alternate `amplitude` to one side of the segment and to the other,
//!   the nodes stay. Random scales each tooth by a fixed pseudo-random
//!   share; Local scales them by how near they are to the centre (full at
//!   the centre, none at the farthest node); Smooth rounds the teeth.
//! - Twister turns every point about the centre by `angle` times its
//!   distance over the farthest distance, so the centre stays and the
//!   farthest point turns by the whole angle. The outline is first cut
//!   into short steps so the turn bends it smoothly.

use crate::geometry::{
    BezPath, CubicBez, Line, ParamCurve, ParamCurveArclen, ParamCurveDeriv, PathEl, PathSeg, Point,
    Rect, Shape as _, Vec2,
};
use kurbo::QuadBez;

/// A distortion and its centre (in the space of the path it is applied
/// to; `None` for the middle of the path's bounds).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum Distort {
    /// Nodes pushed away from the centre (positive) or pulled in
    /// (negative), percent of their distance, -200 to 200.
    PushPull {
        amount: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        center: Option<Point>,
    },
    /// Teeth `amplitude` mm high, `frequency` on each segment.
    Zipper {
        amplitude: f64,
        frequency: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        center: Option<Point>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        random: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        smooth: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        local: bool,
    },
    /// Turned about the centre, `angle_deg` at the farthest point
    /// (counterclockwise when positive).
    Twister {
        angle_deg: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        center: Option<Point>,
    },
}

impl Distort {
    /// The centre it was given, if any.
    pub fn center(&self) -> Option<Point> {
        match *self {
            Distort::PushPull { center, .. }
            | Distort::Zipper { center, .. }
            | Distort::Twister { center, .. } => center,
        }
    }

    /// The same distortion about another centre.
    pub fn with_center(self, at: Option<Point>) -> Distort {
        match self {
            Distort::PushPull { amount, .. } => Distort::PushPull { amount, center: at },
            Distort::Zipper {
                amplitude,
                frequency,
                random,
                smooth,
                local,
                ..
            } => Distort::Zipper {
                amplitude,
                frequency,
                center: at,
                random,
                smooth,
                local,
            },
            Distort::Twister { angle_deg, .. } => Distort::Twister {
                angle_deg,
                center: at,
            },
        }
    }
}

/// The highest Zipper frequency and amplitude, and the largest Twister
/// angle (degrees) and Push and Pull amount taken.
pub const MAX_FREQUENCY: u32 = 100;
pub const MAX_AMPLITUDE: f64 = 1000.0;
pub const MAX_ANGLE: f64 = 3600.0;
pub const MAX_AMOUNT: f64 = 200.0;

fn finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// Apply `how` to `path`.
pub fn distort(path: &BezPath, how: Distort) -> BezPath {
    distort_in(path, path.bounding_box(), how)
}

/// As [`distort`], about the box `b` instead of the path's own (the
/// objects of a group, each distorted about the box of them all).
pub fn distort_in(path: &BezPath, b: Rect, how: Distort) -> BezPath {
    if !(b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite()) {
        return path.clone();
    }
    // A centre far off the outline (a damaged file) is not taken.
    let reach = b.inflate(1e5, 1e5);
    let c = how
        .center()
        .filter(|p| finite(*p) && reach.contains(*p))
        .unwrap_or(b.center());
    let out = match how {
        Distort::PushPull { amount, .. } => {
            let s = if amount.is_finite() {
                amount.clamp(-MAX_AMOUNT, MAX_AMOUNT) / 100.0
            } else {
                0.0
            };
            push_pull(path, c, s)
        }
        Distort::Zipper {
            amplitude,
            frequency,
            random,
            smooth,
            local,
            ..
        } => {
            let a = if amplitude.is_finite() {
                amplitude.clamp(0.0, MAX_AMPLITUDE)
            } else {
                0.0
            };
            zipper(
                path,
                c,
                a,
                frequency.min(MAX_FREQUENCY),
                random,
                smooth,
                local,
            )
        }
        Distort::Twister { angle_deg, .. } => {
            let a = if angle_deg.is_finite() {
                angle_deg.clamp(-MAX_ANGLE, MAX_ANGLE)
            } else {
                0.0
            };
            twister(path, c, a.to_radians())
        }
    };
    let ob = out.bounding_box();
    if ob.x0.is_finite() && ob.y0.is_finite() && ob.x1.is_finite() && ob.y1.is_finite() {
        out
    } else {
        path.clone()
    }
}

/// One subpath: its segments and whether it closes.
struct Sub {
    segs: Vec<PathSeg>,
    closed: bool,
    start: Point,
}

/// The subpaths of `path` as segments, the closing line of a closed one
/// included as a segment when its last point is not its first.
fn subpaths(path: &BezPath) -> Vec<Sub> {
    let mut out: Vec<Sub> = Vec::new();
    let mut cur: Option<Sub> = None;
    let mut last = Point::ZERO;
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                if let Some(s) = cur.take() {
                    out.push(s);
                }
                cur = Some(Sub {
                    segs: Vec::new(),
                    closed: false,
                    start: p,
                });
                last = p;
            }
            PathEl::LineTo(p) => {
                let s = cur.get_or_insert(Sub {
                    segs: Vec::new(),
                    closed: false,
                    start: last,
                });
                s.segs.push(PathSeg::Line(Line::new(last, p)));
                last = p;
            }
            PathEl::QuadTo(q, p) => {
                let s = cur.get_or_insert(Sub {
                    segs: Vec::new(),
                    closed: false,
                    start: last,
                });
                s.segs.push(PathSeg::Quad(QuadBez::new(last, q, p)));
                last = p;
            }
            PathEl::CurveTo(a, b, p) => {
                let s = cur.get_or_insert(Sub {
                    segs: Vec::new(),
                    closed: false,
                    start: last,
                });
                s.segs.push(PathSeg::Cubic(CubicBez::new(last, a, b, p)));
                last = p;
            }
            PathEl::ClosePath => {
                if let Some(mut s) = cur.take() {
                    if (last - s.start).hypot() > 1e-9 {
                        s.segs.push(PathSeg::Line(Line::new(last, s.start)));
                    }
                    s.closed = true;
                    last = s.start;
                    out.push(s);
                }
            }
        }
    }
    if let Some(s) = cur.take() {
        out.push(s);
    }
    // A subpath that ends where it started is closed (an ellipse has no
    // closing element).
    for s in &mut out {
        if !s.closed
            && s.segs.len() > 1
            && s.segs
                .last()
                .is_some_and(|g| (g.end() - s.start).hypot() < 1e-6)
        {
            s.closed = true;
        }
    }
    out
}

/// A segment as a cubic: lines with their control points at a third and
/// two thirds, quadratics raised.
fn as_cubic(seg: &PathSeg) -> (Point, Point, Point, Point) {
    match *seg {
        PathSeg::Line(l) => (
            l.p0,
            l.p0.lerp(l.p1, 1.0 / 3.0),
            l.p0.lerp(l.p1, 2.0 / 3.0),
            l.p1,
        ),
        PathSeg::Quad(q) => (
            q.p0,
            q.p0 + (q.p1 - q.p0) * (2.0 / 3.0),
            q.p2 + (q.p1 - q.p2) * (2.0 / 3.0),
            q.p2,
        ),
        PathSeg::Cubic(c) => (c.p0, c.p1, c.p2, c.p3),
    }
}

fn push_pull(path: &BezPath, c: Point, s: f64) -> BezPath {
    let node = |p: Point| c + (p - c) * (1.0 + s);
    let handle = |p: Point| c + (p - c) * (1.0 - s);
    let mut out = BezPath::new();
    for sub in subpaths(path) {
        out.move_to(node(sub.start));
        for seg in &sub.segs {
            let (_, a, b, p) = as_cubic(seg);
            out.curve_to(handle(a), handle(b), node(p));
        }
        if sub.closed {
            out.close_path();
        }
    }
    out
}

/// A pseudo-random share in 0..1, the same for the same tooth.
fn share(seg: usize, k: usize) -> f64 {
    let mut h = (seg as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((k as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// The unit normal of a segment at `t` (left of its direction).
fn normal(seg: &PathSeg, t: f64) -> Vec2 {
    let e = 1e-4;
    let (t0, t1) = ((t - e).max(0.0), (t + e).min(1.0));
    let d = seg.eval(t1) - seg.eval(t0);
    let len = d.hypot();
    if len < 1e-12 {
        return Vec2::ZERO;
    }
    Vec2::new(-d.y / len, d.x / len)
}

/// The unit normal between two normals (the direction a node moves
/// where two segments meet); `b` alone when they cancel out.
fn between(a: Vec2, b: Vec2) -> Vec2 {
    let m = a + b;
    let len = m.hypot();
    if len < 1e-9 {
        b
    } else {
        m / len
    }
}

fn zipper(
    path: &BezPath,
    c: Point,
    amplitude: f64,
    frequency: u32,
    random: bool,
    smooth: bool,
    local: bool,
) -> BezPath {
    if frequency == 0 || amplitude <= 0.0 {
        return path.clone();
    }
    let subs = subpaths(path);
    // The farthest node from the centre, for Local.
    let far = subs
        .iter()
        .flat_map(|s| std::iter::once(s.start).chain(s.segs.iter().map(|g| g.end())))
        .map(|p| (p - c).hypot())
        .fold(0.0f64, f64::max)
        .max(1e-9);
    // `frequency + 1` lines on every segment, so a line with frequency 5
    // is six lines between seven tips (#1122).
    let steps = frequency as usize + 1;
    let mut out = BezPath::new();
    let mut seg_index = 0usize;
    for sub in subs {
        let count = sub.segs.len();
        if count == 0 {
            out.move_to(sub.start);
            continue;
        }
        // A closed outline needs an even number of tips for the last to
        // alternate with the first: its last segment takes one line more
        // when the count would be odd.
        let odd_closed = sub.closed && (count * steps) % 2 == 1;
        let mut pts = Vec::with_capacity(count * steps + 2);
        for (i, seg) in sub.segs.iter().enumerate() {
            let last_seg = i + 1 == count;
            let steps = if odd_closed && last_seg {
                steps + 1
            } else {
                steps
            };
            // The teeth run through the nodes: every point of the zigzag,
            // the nodes included, is a tooth's tip, so the ends of an open
            // curve are tips too and the pattern goes on unbroken from
            // one segment to the next (#1108). An open curve's last node
            // comes with its last segment; a closed one's start is its
            // end.
            let last = if !sub.closed && last_seg {
                steps
            } else {
                steps - 1
            };
            for k in 0..=last {
                let t = k as f64 / steps as f64;
                let p = seg.eval(t);
                let mut n = normal(seg, t);
                if k == 0 {
                    // A node moves between the normals of its two segments.
                    let before = if i > 0 {
                        sub.segs.get(i - 1)
                    } else if sub.closed {
                        sub.segs.last()
                    } else {
                        None
                    };
                    if let Some(b) = before {
                        n = between(normal(b, 1.0), n);
                    }
                }
                // Tips alternate along the whole subpath.
                let sign = if pts.len() % 2 == 1 { 1.0 } else { -1.0 };
                let mut a = amplitude;
                if random {
                    a *= share(seg_index, k);
                }
                if local {
                    a *= (1.0 - (p - c).hypot() / far).clamp(0.0, 1.0);
                }
                pts.push(p + n * (sign * a));
            }
            seg_index += 1;
        }
        if smooth {
            out.extend(
                crate::geometry::smooth_path(&pts, sub.closed)
                    .elements()
                    .iter()
                    .copied(),
            );
        } else {
            out.extend(
                crate::geometry::polyline_path(&pts, sub.closed)
                    .elements()
                    .iter()
                    .copied(),
            );
        }
    }
    out
}

/// The most a point's turn changes from one step of a twisted segment
/// to the next (radians): 24 steps to a turn keep a cubic through the
/// steps, with their true directions, within a few hundredths of a
/// percent of the size of the true curve.
const TWIST_STEP: f64 = std::f64::consts::PI / 12.0;

/// The most steps one segment is cut into.
const MAX_STEPS: usize = 2048;

fn twister(path: &BezPath, c: Point, angle: f64) -> BezPath {
    if angle == 0.0 {
        return path.clone();
    }
    let subs = subpaths(path);
    let b = path.bounding_box();
    let size = b.width().max(b.height()).max(1e-6);
    let step = size / 64.0;
    // The farthest point from the centre turns by the whole angle
    // (measured on points along the segments; the turn is clamped at
    // the whole angle for the little a curve may reach beyond them).
    let far = subs
        .iter()
        .flat_map(|s| {
            std::iter::once(s.start).chain(s.segs.iter().flat_map(|g| {
                let n = ((g.arclen(1e-3) / step).ceil() as usize).clamp(1, 64);
                (1..=n).map(move |k| g.eval(k as f64 / n as f64))
            }))
        })
        .map(|p| (p - c).hypot())
        .fold(0.0f64, f64::max)
        .max(1e-9);
    let rate = angle / far;
    // A point and the direction a curve through it goes after the turn:
    // `p` turned about `c` by `angle * |p - c| / far`, and its tangent
    // `d` turned the same with the extra swing of the turn growing with
    // the distance (the derivative of the turn along the curve).
    let turn = |p: Point, d: Vec2| -> (Point, Vec2) {
        let v = p - c;
        let r = v.hypot();
        let a = (rate * r).clamp(-angle.abs(), angle.abs());
        let (sin, cos) = a.sin_cos();
        let rot = |w: Vec2| Vec2::new(w.x * cos - w.y * sin, w.x * sin + w.y * cos);
        let swing = if r < 1e-12 || r > far {
            Vec2::ZERO
        } else {
            Vec2::new(-v.y, v.x) * (rate * v.dot(d) / r)
        };
        (c + rot(v), rot(d + swing))
    };
    let mut out = BezPath::new();
    for sub in &subs {
        let (p0, _) = turn(sub.start, Vec2::ZERO);
        out.move_to(p0);
        for seg in &sub.segs {
            let cubic = match *seg {
                PathSeg::Line(l) => {
                    let (a, b, c2, d) = as_cubic(&PathSeg::Line(l));
                    CubicBez::new(a, b, c2, d)
                }
                PathSeg::Quad(q) => q.raise(),
                PathSeg::Cubic(cb) => cb,
            };
            // Steps along which the turn changes by at most `TWIST_STEP`
            // (the distance from the centre, so the turn, changes by at
            // most the length walked). A cubic through each step's ends
            // with their true directions follows the turned curve; where
            // the turn does not change (a circle about its middle) the
            // segment turns whole, exactly.
            let len = seg.arclen(1e-3);
            let n = ((rate.abs() * len / TWIST_STEP).ceil() as usize).clamp(1, MAX_STEPS);
            let h = 1.0 / n as f64;
            let deriv = cubic.deriv();
            let mut prev = turn(cubic.eval(0.0), deriv.eval(0.0).to_vec2());
            for k in 1..=n {
                let t = k as f64 * h;
                let next = turn(cubic.eval(t), deriv.eval(t).to_vec2());
                // A cubic through the two points with their directions
                // (Hermite), the handles a third of the step along them.
                let c1 = prev.0 + prev.1 * (h / 3.0);
                let c2 = next.0 - next.1 * (h / 3.0);
                out.curve_to(c1, c2, next.0);
                prev = next;
            }
        }
        if sub.closed {
            out.close_path();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{ParamCurveNearest, Rect};

    fn square(x: f64, y: f64, s: f64) -> BezPath {
        let mut p = BezPath::new();
        p.move_to((x, y));
        p.line_to((x + s, y));
        p.line_to((x + s, y + s));
        p.line_to((x, y + s));
        p.close_path();
        p
    }

    fn nodes(p: &BezPath) -> Vec<Point> {
        p.elements()
            .iter()
            .filter_map(|e| match e {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => Some(*p),
                PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => Some(*p),
                PathEl::ClosePath => None,
            })
            .collect()
    }

    fn subpath_count(p: &BezPath) -> usize {
        p.elements()
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count()
    }

    fn closes(p: &BezPath) -> usize {
        p.elements()
            .iter()
            .filter(|e| matches!(e, PathEl::ClosePath))
            .count()
    }

    #[test]
    fn push_and_pull_move_nodes_and_bow_the_sides() {
        let sq = square(-10.0, -10.0, 20.0);
        // Pushed 50 %: the corners go out by half their distance, the
        // sides bow in: the middle of a side comes nearer the centre.
        let pushed = distort(
            &sq,
            Distort::PushPull {
                amount: 50.0,
                center: None,
            },
        );
        let n = nodes(&pushed);
        assert!(n
            .iter()
            .any(|p| (p.x - 15.0).abs() < 1e-9 && (p.y - 15.0).abs() < 1e-9));
        let side_mid = |p: &BezPath| {
            p.segments()
                .map(|s| s.eval(0.5))
                .map(|q| q.to_vec2().hypot())
                .fold(f64::INFINITY, f64::min)
        };
        assert!(side_mid(&pushed) < 10.0, "{}", side_mid(&pushed));
        // Pulled 50 %: corners in, sides bow out past the old sides.
        let pulled = distort(
            &sq,
            Distort::PushPull {
                amount: -50.0,
                center: None,
            },
        );
        assert!(nodes(&pulled)
            .iter()
            .any(|p| (p.x - 5.0).abs() < 1e-9 && (p.y - 5.0).abs() < 1e-9));
        let reach = pulled.bounding_box();
        assert!(reach.width() > 10.0);
        assert_eq!((subpath_count(&pulled), closes(&pulled)), (1, 1));
        // About a centre of its own: a node at the centre stays.
        let off = distort(
            &sq,
            Distort::PushPull {
                amount: 80.0,
                center: Some(Point::new(-10.0, -10.0)),
            },
        );
        assert!(nodes(&off)
            .iter()
            .any(|p| (p.x + 10.0).abs() < 1e-9 && (p.y + 10.0).abs() < 1e-9));
        // An open curve stays open; two squares stay two subpaths.
        let mut open = BezPath::new();
        open.move_to((0.0, 0.0));
        open.line_to((10.0, 0.0));
        open.line_to((10.0, 10.0));
        let o = distort(
            &open,
            Distort::PushPull {
                amount: 30.0,
                center: None,
            },
        );
        assert_eq!((subpath_count(&o), closes(&o)), (1, 0));
        let mut two = square(0.0, 0.0, 10.0);
        two.extend(square(20.0, 0.0, 10.0).elements().iter().copied());
        let t = distort(
            &two,
            Distort::PushPull {
                amount: 30.0,
                center: None,
            },
        );
        assert_eq!((subpath_count(&t), closes(&t)), (2, 2));
    }

    fn zip(random: bool, smooth: bool, local: bool) -> Distort {
        Distort::Zipper {
            amplitude: 2.0,
            frequency: 3,
            center: None,
            random,
            smooth,
            local,
        }
    }

    #[test]
    fn zipper_teeth_on_every_segment() {
        let sq = square(0.0, 0.0, 20.0);
        let out = distort(&sq, zip(false, false, false));
        // 4 sides of 4 lines (frequency 3 plus one), one tip each, closed.
        assert_eq!(nodes(&out).len(), 16);
        assert_eq!(closes(&out), 1);
        // The teeth reach 2 mm out and in along the sides; the corners
        // are tips too, moved between the normals of their sides.
        let b = out.bounding_box();
        assert!(
            (b.x0 + 2.0).abs() < 1e-6 && (b.x1 - 22.0).abs() < 1e-6,
            "{b:?}"
        );
        let corner = Point::new(20.0, 0.0);
        assert!(!nodes(&out).contains(&corner));
        let moved = nodes(&out)
            .into_iter()
            .map(|p| (p - corner).hypot())
            .fold(f64::INFINITY, f64::min);
        assert!((moved - 2.0).abs() < 1e-6, "{moved}");
        // No frequency: unchanged.
        let none = Distort::Zipper {
            amplitude: 2.0,
            frequency: 0,
            center: None,
            random: false,
            smooth: false,
            local: false,
        };
        assert_eq!(distort(&sq, none), sq);
        // Random: smaller teeth, the same each time.
        let a = distort(&sq, zip(true, false, false));
        assert_eq!(a, distort(&sq, zip(true, false, false)));
        let ab = a.bounding_box();
        assert!(ab.width() <= 24.0 + 1e-9 && ab != b);
        // Local: smaller teeth away from the centre (none at the corners).
        let lb = distort(&sq, zip(false, false, true)).bounding_box();
        assert!(lb.width() < b.width());
        // Smooth: curves through the teeth.
        assert!(distort(&sq, zip(false, true, false))
            .elements()
            .iter()
            .any(|e| matches!(e, PathEl::CurveTo(..))));
    }

    /// A straight line with the Zipper (#1108, #1122): every point of
    /// the zigzag, the two ends included, is a tooth's tip, alternately
    /// on each side; no node is left on the line; frequency 6 gives
    /// seven lines.
    #[test]
    fn a_zipped_line_starts_and_ends_on_tips() {
        let mut line = BezPath::new();
        line.move_to((10.0, 0.0));
        line.line_to((10.0, 70.0));
        let out = distort(
            &line,
            Distort::Zipper {
                amplitude: 3.0,
                frequency: 6,
                center: None,
                random: false,
                smooth: false,
                local: false,
            },
        );
        let n = nodes(&out);
        assert_eq!(n.len(), 8, "seven lines");
        assert_eq!((subpath_count(&out), closes(&out)), (1, 0));
        for (i, p) in n.iter().enumerate() {
            assert!(((p.x - 10.0).abs() - 3.0).abs() < 1e-9, "{i}: {p:?}");
            assert!((p.y - 10.0 * i as f64).abs() < 1e-9, "{i}: {p:?}");
            if i > 0 {
                assert!((p.x - 10.0) * (n[i - 1].x - 10.0) < 0.0, "alternate");
            }
        }
        // Two segments: the pattern goes on through the middle node, no
        // two neighbouring tips on the same side.
        let mut bent = BezPath::new();
        bent.move_to((0.0, 0.0));
        bent.line_to((30.0, 0.0));
        bent.line_to((30.0, 30.0));
        let out = distort(
            &bent,
            Distort::Zipper {
                amplitude: 1.0,
                frequency: 3,
                center: None,
                random: false,
                smooth: true,
                local: false,
            },
        );
        let n = nodes(&out);
        assert_eq!(n.len(), 9, "two segments of four lines");
        let side = |p: &Point| -> f64 {
            // Signed distance from the nearer side of the bent line,
            // outside positive.
            if p.y.abs() <= (p.x - 30.0).abs() {
                -p.y
            } else {
                p.x - 30.0
            }
        };
        for i in 1..n.len() {
            assert!(
                side(&n[i]) * side(&n[i - 1]) < 0.0,
                "{i}: {:?}",
                &n[i - 1..=i]
            );
        }
        // A closed outline with an odd count of tips (a triangle with
        // frequency 2: three sides of three lines) takes one line more
        // on its last side, so the last tip alternates with the first.
        let mut tri = BezPath::new();
        tri.move_to((0.0, 0.0));
        tri.line_to((40.0, 0.0));
        tri.line_to((20.0, 30.0));
        tri.close_path();
        let out = distort(
            &tri,
            Distort::Zipper {
                amplitude: 1.0,
                frequency: 2,
                center: None,
                random: false,
                smooth: false,
                local: false,
            },
        );
        let n = nodes(&out);
        assert_eq!(n.len(), 10);
        let c = Point::new(20.0, 10.0);
        let outward = |p: &Point| -> bool {
            // Farther from the middle than the nearest point of the
            // triangle's own outline: the tip points out.
            let (on, _) = tri
                .segments()
                .map(|s| {
                    let q = s.nearest(*p, 1e-6);
                    (s.eval(q.t), q.distance_sq)
                })
                .fold((Point::ZERO, f64::INFINITY), |best, cur| {
                    if cur.1 < best.1 {
                        cur
                    } else {
                        best
                    }
                });
            (*p - c).hypot() > (on - c).hypot()
        };
        for i in 0..n.len() {
            let j = (i + 1) % n.len();
            assert_ne!(outward(&n[i]), outward(&n[j]), "{i}: {:?} {:?}", n[i], n[j]);
        }
    }

    /// A straight line with the Twister (#1108) is a smooth spiral from
    /// end to end: every step lies on the true turned curve and the ends
    /// leave along its direction, not along a straight tail.
    #[test]
    fn a_twisted_line_is_a_smooth_spiral_to_its_ends() {
        let mut line = BezPath::new();
        line.move_to((0.0, -50.0));
        line.line_to((0.0, 50.0));
        let turns = 4.0;
        let out = distort(
            &line,
            Distort::Twister {
                angle_deg: -360.0 * turns,
                center: None,
            },
        );
        assert_eq!((subpath_count(&out), closes(&out)), (1, 0));
        // Fine enough: a step at most a 24th of a turn, each half of
        // the line turning four times.
        let segs: Vec<PathSeg> = out.segments().collect();
        assert!(segs.len() >= 2 * 24 * turns as usize, "{}", segs.len());
        // The true curve: a point at distance r from the middle turned
        // by r / 50 of the angle.
        let angle = (-360.0f64 * turns).to_radians();
        let truth = |y: f64| -> Point {
            let a = angle * y.abs() / 50.0;
            Point::new(-y * a.sin(), y * a.cos())
        };
        for seg in &segs {
            for k in 0..=4 {
                let p = seg.eval(k as f64 / 4.0);
                let r = p.to_vec2().hypot();
                // The point is on the spiral: at its distance the curve
                // passes through the same angle.
                let want = truth(r);
                let want2 = truth(-r);
                let off = (p - want).hypot().min((p - want2).hypot());
                assert!(off < 0.03, "{p:?} off by {off}");
            }
        }
        // The end leaves tangent to the spiral (the derivative of the
        // turn), not along the chord to the next step.
        let first = segs[0];
        let d0 = (first.eval(1e-3) - first.eval(0.0)).normalize();
        let want = (truth(-50.0 + 1e-4) - truth(-50.0)).normalize();
        assert!(d0.dot(want) > 0.999, "{d0:?} vs {want:?}");
        let last = segs[segs.len() - 1];
        let d1 = (last.eval(1.0) - last.eval(1.0 - 1e-3)).normalize();
        let want = (truth(50.0) - truth(50.0 - 1e-4)).normalize();
        assert!(d1.dot(want) > 0.999, "{d1:?} vs {want:?}");
        // No two neighbouring steps meet at a corner.
        for w in segs.windows(2) {
            let a = (w[0].eval(1.0) - w[0].eval(0.999)).normalize();
            let b = (w[1].eval(0.001) - w[1].eval(0.0)).normalize();
            assert!(a.dot(b) > 0.99, "corner between steps: {a:?} {b:?}");
        }
    }

    #[test]
    fn twister_turns_the_far_points_by_the_angle() {
        let sq = square(-10.0, -10.0, 20.0);
        let t = distort(
            &sq,
            Distort::Twister {
                angle_deg: 90.0,
                center: None,
            },
        );
        // A corner (farthest) turned a quarter: (10, 10) to (-10, 10).
        let n = nodes(&t);
        assert!(
            n.iter()
                .any(|p| (p.x + 10.0).abs() < 1e-6 && (p.y - 10.0).abs() < 1e-6),
            "{n:?}"
        );
        assert_eq!((subpath_count(&t), closes(&t)), (1, 1));
        // The bounds of a quarter turn of a square about its middle stay
        // near the square's.
        let b = t.bounding_box();
        assert!(b.width() < 30.0 && b.width() > 18.0, "{b:?}");
        // An ellipse (no closing element, but closed) stays closed.
        let e = crate::geometry::ellipse_path(Rect::new(0.0, 0.0, 30.0, 20.0));
        let te = distort(
            &e,
            Distort::Twister {
                angle_deg: 60.0,
                center: None,
            },
        );
        assert_eq!((subpath_count(&te), closes(&te)), (1, 1));
        // Zero: unchanged; an odd centre falls back to the middle.
        let z = Distort::Twister {
            angle_deg: 0.0,
            center: None,
        };
        assert_eq!(distort(&sq, z), sq);
        let odd = Distort::Twister {
            angle_deg: 90.0,
            center: Some(Point::new(f64::NAN, 0.0)),
        };
        assert_eq!(distort(&sq, odd), t);
    }

    #[test]
    fn older_files_and_odd_values_read() {
        let d: Distort = serde_json::from_str(r#"{"mode":"twister","angle_deg":140.0}"#)
            .expect("an older twister");
        assert_eq!(
            d,
            Distort::Twister {
                angle_deg: 140.0,
                center: None
            }
        );
        let z: Distort =
            serde_json::from_str(r#"{"mode":"zipper","amplitude":2.0,"frequency":12}"#)
                .expect("an older zipper");
        assert!(matches!(
            z,
            Distort::Zipper {
                random: false,
                smooth: false,
                local: false,
                center: None,
                ..
            }
        ));
        // Written without the defaults.
        let s = serde_json::to_string(&z).expect("json");
        assert!(!s.contains("random") && !s.contains("center"), "{s}");
        // Out of range and not a number: no panic, finite output.
        let sq = square(0.0, 0.0, 10.0);
        for how in [
            Distort::PushPull {
                amount: f64::NAN,
                center: None,
            },
            Distort::PushPull {
                amount: 1e9,
                center: None,
            },
            Distort::Zipper {
                amplitude: f64::INFINITY,
                frequency: u32::MAX,
                center: None,
                random: true,
                smooth: true,
                local: true,
            },
            Distort::Twister {
                angle_deg: 1e12,
                center: Some(Point::new(1e300, 0.0)),
            },
        ] {
            let out = distort(&sq, how);
            let b = out.bounding_box();
            assert!(b.x0.is_finite() && b.y1.is_finite(), "{how:?} {b:?}");
        }
        assert_eq!(
            distort(
                &BezPath::new(),
                Distort::PushPull {
                    amount: 10.0,
                    center: None
                }
            ),
            BezPath::new()
        );
        let _ = Rect::ZERO;
    }
}
