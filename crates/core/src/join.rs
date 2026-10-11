//! Join curves (the Join Curves panel): the open
//! subpaths of the curves given are joined end to end, nearest ends
//! first, as long as the ends are within the gap tolerance. The two ends
//! of one subpath close it. The joint takes the mode's shape: the ends
//! extended to where they meet, a straight line between them, a fillet,
//! or a Bezier curve that carries on both directions.

use crate::corner_cut::{cut_corners, CornerCut};
use crate::geometry::{BezPath, PathEl, Point, Vec2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoinMode {
    /// Extend both ends to their intersection (a straight line when they
    /// do not meet ahead of both ends).
    Extend,
    /// A straight line between the ends.
    Chamfer,
    /// Extend, then round the corner with this radius.
    Fillet(f64),
    /// A curve that leaves and arrives along the ends' directions.
    Bezier,
}

/// A subpath as its elements after the move-to.
#[derive(Debug, Clone)]
struct Open {
    start: Point,
    els: Vec<PathEl>,
}

impl Open {
    fn end(&self) -> Point {
        match self.els.last() {
            Some(PathEl::LineTo(p))
            | Some(PathEl::CurveTo(_, _, p))
            | Some(PathEl::QuadTo(_, p)) => *p,
            _ => self.start,
        }
    }

    /// Point before the end, for the direction the subpath arrives in.
    fn before_end(&self) -> Option<Point> {
        let end = self.end();
        let prev = |i: usize| -> Point {
            if i == 0 {
                return self.start;
            }
            match self.els[i - 1] {
                PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) | PathEl::QuadTo(_, p) => p,
                _ => self.start,
            }
        };
        let i = self.els.len().checked_sub(1)?;
        let candidates = match self.els[i] {
            PathEl::CurveTo(c1, c2, _) => vec![c2, c1, prev(i)],
            PathEl::QuadTo(c, _) => vec![c, prev(i)],
            _ => vec![prev(i)],
        };
        candidates.into_iter().find(|q| (end - *q).hypot() > 1e-12)
    }

    /// Direction of travel at the end.
    fn end_dir(&self) -> Option<Vec2> {
        let q = self.before_end()?;
        let d = self.end() - q;
        Some(d / d.hypot())
    }

    /// Direction of travel at the start.
    fn start_dir(&self) -> Option<Vec2> {
        self.reversed().end_dir().map(|d| -d)
    }

    fn reversed(&self) -> Open {
        let mut pts = vec![self.start];
        for el in &self.els {
            match el {
                PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) | PathEl::QuadTo(_, p) => pts.push(*p),
                _ => {}
            }
        }
        let mut els = Vec::new();
        for (k, el) in self.els.iter().enumerate().rev() {
            let to = pts[k];
            els.push(match *el {
                PathEl::CurveTo(c1, c2, _) => PathEl::CurveTo(c2, c1, to),
                PathEl::QuadTo(c, _) => PathEl::QuadTo(c, to),
                _ => PathEl::LineTo(to),
            });
        }
        Open {
            start: self.end(),
            els,
        }
    }
}

/// Where the ray from `a` along `da` meets the ray back from `b` against
/// `db` (both ahead of their ends).
fn meet(a: Point, da: Vec2, b: Point, db: Vec2) -> Option<Point> {
    let denom = da.cross(db);
    if denom.abs() < 1e-12 {
        return None;
    }
    // a + da s = b - db t, with s and t not behind the ends.
    let w = b - a;
    let s = w.cross(db) / denom;
    let t = -w.cross(da) / denom;
    (s >= -1e-9 && t >= -1e-9).then(|| a + da * s)
}

/// Append to `a` the joint to `b`'s start and `b` itself; returns the
/// joined subpath and the element index (in its own elements, move-to
/// counted) of a corner made by extending, for a fillet.
fn joint(a: &Open, b: &Open, mode: JoinMode) -> (Open, Option<usize>) {
    let mut out = a.clone();
    let (pa, pb) = (a.end(), b.start);
    let mut corner = None;
    let gap = (pb - pa).hypot();
    let straight = |out: &mut Open| {
        if gap > 1e-12 {
            out.els.push(PathEl::LineTo(pb));
        }
    };
    match mode {
        JoinMode::Chamfer => straight(&mut out),
        JoinMode::Bezier => match (a.end_dir(), b.start_dir()) {
            (Some(da), Some(db)) if gap > 1e-12 => {
                let h = gap / 3.0;
                out.els.push(PathEl::CurveTo(pa + da * h, pb - db * h, pb));
            }
            _ => straight(&mut out),
        },
        JoinMode::Extend | JoinMode::Fillet(_) => {
            let x = match (a.end_dir(), b.start_dir()) {
                (Some(da), Some(db)) if gap > 1e-12 => meet(pa, da, pb, db),
                _ => None,
            };
            match x {
                Some(x) => {
                    // A straight end moves to the meeting point; a curved
                    // one gets a straight piece to it.
                    match out.els.last_mut() {
                        Some(PathEl::LineTo(p)) => *p = x,
                        _ => {
                            if (x - pa).hypot() > 1e-12 {
                                out.els.push(PathEl::LineTo(x));
                            }
                        }
                    }
                    corner = Some(out.els.len());
                    let b_line = matches!(b.els.first(), Some(PathEl::LineTo(_)));
                    if !b_line && (pb - x).hypot() > 1e-12 {
                        out.els.push(PathEl::LineTo(pb));
                    }
                    if b_line {
                        // B's straight first segment now starts at the meeting point.
                        out.els.extend(b.els.iter().copied());
                        return (out, corner);
                    }
                }
                None => straight(&mut out),
            }
        }
    }
    out.els.extend(b.els.iter().copied());
    (out, corner)
}

/// Close `a` onto its own start with the mode's joint (the close itself
/// draws a straight line); returns the closed subpath and the element
/// index of a corner made by extending.
fn close(a: &Open, mode: JoinMode) -> (Open, Option<usize>) {
    let mut out = a.clone();
    let (pe, ps) = (a.end(), a.start);
    let gap = (ps - pe).hypot();
    let mut corner = None;
    match mode {
        JoinMode::Chamfer => {}
        JoinMode::Bezier => {
            if let (Some(da), Some(db), true) = (a.end_dir(), a.start_dir(), gap > 1e-12) {
                let h = gap / 3.0;
                out.els.push(PathEl::CurveTo(pe + da * h, ps - db * h, ps));
            }
        }
        JoinMode::Extend | JoinMode::Fillet(_) => {
            let x = match (a.end_dir(), a.start_dir()) {
                (Some(da), Some(db)) if gap > 1e-12 => meet(pe, da, ps, db),
                _ => None,
            };
            if let Some(x) = x {
                match out.els.last_mut() {
                    Some(PathEl::LineTo(p)) => *p = x,
                    _ => out.els.push(PathEl::LineTo(x)),
                }
                if matches!(a.els.first(), Some(PathEl::LineTo(_))) {
                    // The first side now starts at the meeting point, which
                    // is the start node; the close draws the last side.
                    out.start = x;
                    if let Some(PathEl::LineTo(p)) = out.els.last() {
                        if (*p - x).hypot() < 1e-12 {
                            out.els.pop();
                        }
                    }
                    corner = Some(0);
                } else {
                    corner = Some(out.els.len());
                }
            }
        }
    }
    (out, corner)
}

/// Round the corner at element `corner` of one subpath.
fn fillet_at(o: Open, closed: bool, corner: usize, r: f64) -> Open {
    let single = to_path(&[(o.clone(), closed)]);
    let (cut, n) = cut_corners(&single, CornerCut::Fillet(r), Some(&[corner]));
    if n == 0 {
        return o;
    }
    let Some(PathEl::MoveTo(start)) = cut.elements().first().copied() else {
        return o;
    };
    let mut els: Vec<PathEl> = cut.elements()[1..].to_vec();
    if els.last() == Some(&PathEl::ClosePath) {
        els.pop();
    }
    Open { start, els }
}

fn to_path(subs: &[(Open, bool)]) -> BezPath {
    let mut p = BezPath::new();
    for (o, closed) in subs {
        p.move_to(o.start);
        for el in &o.els {
            p.push(*el);
        }
        if *closed {
            p.close_path();
        }
    }
    p
}

/// Join the open subpaths of `paths` (all in one space) whose ends lie
/// within `gap` of each other; returns the joined outline and the number
/// of joints made.
pub fn join_curves(paths: &[BezPath], mode: JoinMode, gap: f64) -> (BezPath, usize) {
    let mut subs: Vec<(Open, bool)> = Vec::new();
    for path in paths {
        let mut cur: Option<Open> = None;
        for el in path.elements() {
            match *el {
                PathEl::MoveTo(p) => {
                    if let Some(o) = cur.take() {
                        subs.push((o, false));
                    }
                    cur = Some(Open {
                        start: p,
                        els: Vec::new(),
                    });
                }
                PathEl::ClosePath => {
                    if let Some(o) = cur.take() {
                        subs.push((o, true));
                    }
                }
                other => {
                    if let Some(o) = cur.as_mut() {
                        o.els.push(other);
                    }
                }
            }
        }
        if let Some(o) = cur.take() {
            subs.push((o, false));
        }
    }
    let gap = if gap.is_finite() { gap.max(0.0) } else { 0.0 };
    let mut joints = 0;
    loop {
        // The nearest pair of free ends within the gap.
        let mut best: Option<(f64, usize, bool, usize, bool)> = None;
        for i in 0..subs.len() {
            if subs[i].1 || subs[i].0.els.is_empty() {
                continue;
            }
            for j in i..subs.len() {
                if subs[j].1 || subs[j].0.els.is_empty() {
                    continue;
                }
                for (ei, pi) in [(false, subs[i].0.start), (true, subs[i].0.end())] {
                    for (ej, pj) in [(false, subs[j].0.start), (true, subs[j].0.end())] {
                        // One subpath: its end against its start only.
                        if i == j && (ei == ej || ei) {
                            continue;
                        }
                        let d = (pi - pj).hypot();
                        if d <= gap && best.is_none_or(|b| d < b.0) {
                            best = Some((d, i, ei, j, ej));
                        }
                    }
                }
            }
        }
        let Some((_, i, ei, j, ej)) = best else {
            break;
        };
        joints += 1;
        if i == j {
            let (mut o, corner) = close(&subs[i].0, mode);
            if let (JoinMode::Fillet(r), Some(c)) = (mode, corner) {
                o = fillet_at(o, true, c, r);
            }
            subs[i] = (o, true);
            continue;
        }
        // A ends at its chosen end, B starts at its chosen end.
        let a = if ei {
            subs[i].0.clone()
        } else {
            subs[i].0.reversed()
        };
        let b = if ej {
            subs[j].0.reversed()
        } else {
            subs[j].0.clone()
        };
        let (mut o, corner) = joint(&a, &b, mode);
        if let (JoinMode::Fillet(r), Some(c)) = (mode, corner) {
            o = fillet_at(o, false, c, r);
        }
        subs[i] = (o, false);
        subs.remove(j);
    }
    (to_path(&subs), joints)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Shape as _;

    fn line(a: (f64, f64), b: (f64, f64)) -> BezPath {
        let mut p = BezPath::new();
        p.move_to(a);
        p.line_to(b);
        p
    }

    fn nodes(p: &BezPath) -> Vec<Point> {
        crate::nodes::nodes(p).into_iter().map(|n| n.pos).collect()
    }

    #[test]
    fn extend_meets_at_the_corner() {
        // Two lines that would meet at (10, 0).
        let a = line((0.0, 0.0), (8.0, 0.0));
        let b = line((10.0, 2.0), (10.0, 10.0));
        let (p, n) = join_curves(&[a, b], JoinMode::Extend, 5.0);
        assert_eq!(n, 1);
        assert_eq!(
            nodes(&p),
            vec![
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0)
            ]
        );
    }

    #[test]
    fn chamfer_and_bezier_bridge_the_gap() {
        let a = line((0.0, 0.0), (8.0, 0.0));
        let b = line((10.0, 2.0), (10.0, 10.0));
        let (p, _) = join_curves(&[a.clone(), b.clone()], JoinMode::Chamfer, 5.0);
        assert_eq!(nodes(&p).len(), 4);
        let (p, _) = join_curves(&[a, b], JoinMode::Bezier, 5.0);
        assert!(p
            .elements()
            .iter()
            .any(|e| matches!(e, PathEl::CurveTo(..))));
        assert_eq!(nodes(&p).len(), 4);
    }

    #[test]
    fn fillet_rounds_the_extended_corner() {
        let a = line((0.0, 0.0), (8.0, 0.0));
        let b = line((10.0, 2.0), (10.0, 10.0));
        let (p, _) = join_curves(&[a, b], JoinMode::Fillet(3.0), 5.0);
        let ns = nodes(&p);
        assert!(ns.contains(&Point::new(7.0, 0.0)), "{ns:?}");
        assert!(
            ns.iter()
                .any(|q| (*q - Point::new(10.0, 3.0)).hypot() < 1e-9),
            "{ns:?}"
        );
    }

    #[test]
    fn ends_too_far_apart_stay_open_and_one_subpath_closes() {
        let a = line((0.0, 0.0), (8.0, 0.0));
        let b = line((30.0, 2.0), (30.0, 10.0));
        let (p, n) = join_curves(&[a, b], JoinMode::Extend, 5.0);
        assert_eq!(n, 0);
        assert_eq!(
            p.elements()
                .iter()
                .filter(|e| matches!(e, PathEl::MoveTo(_)))
                .count(),
            2
        );
        // An open triangle: its own two ends close it.
        let mut t = BezPath::new();
        t.move_to((0.0, 1.0));
        t.line_to((10.0, 0.0));
        t.line_to((5.0, 8.0));
        t.line_to((0.5, 1.5));
        let (p, n) = join_curves(&[t], JoinMode::Chamfer, 2.0);
        assert_eq!(n, 1);
        assert_eq!(p.elements().last(), Some(&PathEl::ClosePath));
        assert!(p.area().abs() > 30.0);
    }

    #[test]
    fn several_pieces_join_nearest_first_into_one_closed_outline() {
        // Four sides of a square with small gaps at each corner.
        let sides = [
            line((1.0, 0.0), (9.0, 0.0)),
            line((10.0, 1.0), (10.0, 9.0)),
            line((9.0, 10.0), (1.0, 10.0)),
            line((0.0, 9.0), (0.0, 1.0)),
        ];
        let (p, n) = join_curves(&sides, JoinMode::Extend, 3.0);
        assert_eq!(n, 4);
        assert_eq!(
            p.elements()
                .iter()
                .filter(|e| matches!(e, PathEl::MoveTo(_)))
                .count(),
            1
        );
        assert_eq!(p.elements().last(), Some(&PathEl::ClosePath));
        assert!((p.area().abs() - 100.0).abs() < 1e-9, "{p:?}");
    }

    #[test]
    fn closed_and_empty_input_is_kept() {
        let sq = crate::geometry::Rect::new(0.0, 0.0, 5.0, 5.0).to_path(0.01);
        let (p, n) = join_curves(std::slice::from_ref(&sq), JoinMode::Extend, 100.0);
        assert_eq!(n, 0);
        assert!((p.area() - sq.area()).abs() < 1e-9);
        let (p, n) = join_curves(&[], JoinMode::Bezier, f64::NAN);
        assert_eq!(n, 0);
        assert!(p.elements().is_empty());
    }
}
