//! A new line continues the selected curve: drawn with the Freehand,
//! Bezier, Pen, B-spline, Polyline or 2-point line tool, a line that
//! starts or ends on an end node of the selected open curve (within the
//! join distance on screen, Options > Tools > Curves) is added to that
//! curve instead of becoming a curve of its own; one that ends on the
//! other end too closes it, and one between the ends of two of its
//! subpaths makes them one. See docs/behavior/curve-tools.md.

use crate::app::App;
use tracedraw_core::geometry::{BezPath, PathEl, Point};
use tracedraw_core::{Command, ShapeId, ShapeKind};

/// One segment of a subpath, by its end point.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Seg {
    Line(Point),
    Quad(Point, Point),
    Cubic(Point, Point, Point),
}

impl Seg {
    fn end(&self) -> Point {
        match *self {
            Seg::Line(p) | Seg::Quad(_, p) | Seg::Cubic(_, _, p) => p,
        }
    }

    fn with_end(self, e: Point) -> Seg {
        match self {
            Seg::Line(_) => Seg::Line(e),
            Seg::Quad(c, _) => Seg::Quad(c, e),
            Seg::Cubic(a, b, _) => Seg::Cubic(a, b, e),
        }
    }
}

/// A subpath: where it starts, its segments, whether it is closed.
#[derive(Debug, Clone, PartialEq)]
struct Sub {
    start: Point,
    segs: Vec<Seg>,
    closed: bool,
}

impl Sub {
    fn end(&self) -> Point {
        self.segs.last().map_or(self.start, Seg::end)
    }

    /// The same subpath run the other way.
    fn reversed(&self) -> Sub {
        let mut segs = Vec::with_capacity(self.segs.len());
        let mut from = self.start;
        let mut starts = Vec::with_capacity(self.segs.len());
        for s in &self.segs {
            starts.push(from);
            from = s.end();
        }
        for (s, a) in self.segs.iter().zip(starts).rev() {
            segs.push(match *s {
                Seg::Line(_) => Seg::Line(a),
                Seg::Quad(c, _) => Seg::Quad(c, a),
                Seg::Cubic(c1, c2, _) => Seg::Cubic(c2, c1, a),
            });
        }
        Sub {
            start: self.end(),
            segs,
            closed: self.closed,
        }
    }
}

fn subpaths(path: &BezPath) -> Vec<Sub> {
    let mut out: Vec<Sub> = Vec::new();
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => out.push(Sub {
                start: p,
                segs: Vec::new(),
                closed: false,
            }),
            PathEl::LineTo(p) => current(&mut out).segs.push(Seg::Line(p)),
            PathEl::QuadTo(c, p) => current(&mut out).segs.push(Seg::Quad(c, p)),
            PathEl::CurveTo(a, b, p) => current(&mut out).segs.push(Seg::Cubic(a, b, p)),
            PathEl::ClosePath => {
                if let Some(s) = out.last_mut() {
                    s.closed = true;
                }
            }
        }
    }
    out.retain(|s| !s.segs.is_empty());
    out
}

/// The subpath being written to (one is begun when a path lacks its
/// first move).
fn current(out: &mut Vec<Sub>) -> &mut Sub {
    if out.is_empty() {
        out.push(Sub {
            start: Point::ZERO,
            segs: Vec::new(),
            closed: false,
        });
    }
    let n = out.len() - 1;
    &mut out[n]
}

fn to_path(subs: &[Sub]) -> BezPath {
    let mut p = BezPath::new();
    for s in subs {
        p.move_to(s.start);
        for seg in &s.segs {
            match *seg {
                Seg::Line(e) => p.line_to(e),
                Seg::Quad(c, e) => p.quad_to(c, e),
                Seg::Cubic(a, b, e) => p.curve_to(a, b, e),
            }
        }
        if s.closed {
            p.close_path();
        }
    }
    p
}

/// The open subpath end nearest to `p` within `tol`: its index and
/// whether it is the subpath's end (else its start).
fn end_near(subs: &[Sub], p: Point, tol: f64) -> Option<(usize, bool)> {
    let mut best: Option<(usize, bool, f64)> = None;
    for (i, s) in subs.iter().enumerate().filter(|(_, s)| !s.closed) {
        for (at_end, q) in [(false, s.start), (true, s.end())] {
            let d = (q - p).hypot();
            if d <= tol && best.is_none_or(|(_, _, bd)| d < bd) {
                best = Some((i, at_end, d));
            }
        }
    }
    best.map(|(i, e, _)| (i, e))
}

/// `curve` (page space) with `line` (page space, one open subpath) added
/// where its ends meet the curve's end nodes within `tol`; `None` when
/// neither end of the line is on one.
fn joined(curve: &BezPath, line: &BezPath, tol: f64) -> Option<BezPath> {
    let mut subs = subpaths(curve);
    let mut line = subpaths(line).into_iter().next()?;
    if line.closed {
        return None;
    }
    let mut first = end_near(&subs, line.start, tol);
    let mut last = end_near(&subs, line.end(), tol);
    // A line starting and ending on the same end node joins there once.
    if first.is_some() && first == last {
        last = None;
    }
    if first.is_none() {
        line = line.reversed();
        std::mem::swap(&mut first, &mut last);
    }
    let (i, at_end) = first?;
    let mut sub = subs.remove(i);
    if !at_end {
        sub = sub.reversed();
    }
    // The line runs on from the end node itself.
    sub.segs.extend(line.segs);
    match last {
        Some((j, _)) if j == i => {
            // The line ends on the curve's other end: closed.
            let start = sub.start;
            if let Some(s) = sub.segs.last_mut() {
                *s = s.with_end(start);
            }
            sub.closed = true;
        }
        Some((j, j_end)) => {
            let j = if j > i { j - 1 } else { j };
            if j < subs.len() {
                let mut other = subs.remove(j);
                if j_end {
                    other = other.reversed();
                }
                let meet = other.start;
                if let Some(s) = sub.segs.last_mut() {
                    *s = s.with_end(meet);
                }
                sub.segs.extend(other.segs);
            }
        }
        None => {}
    }
    subs.insert(i.min(subs.len()), sub);
    Some(to_path(&subs))
}

/// The handle a new segment from the open end of `sub` (its end when
/// `at_end`, else its start) leaves along, so it goes on smoothly from the
/// segment arriving there: the mirror of that segment's handle at the end
/// node, `None` after a straight segment.
fn mirrored_handle(sub: &Sub, at_end: bool) -> Option<Point> {
    let (node, h) = if at_end {
        let e = sub.end();
        match *sub.segs.last()? {
            Seg::Cubic(_, b, _) => (e, b),
            Seg::Quad(c, _) => (e, e + (c - e) * (2.0 / 3.0)),
            Seg::Line(_) => return None,
        }
    } else {
        let s = sub.start;
        match *sub.segs.first()? {
            Seg::Cubic(a, _, _) => (s, a),
            Seg::Quad(c, _) => (s, s + (c - s) * (2.0 / 3.0)),
            Seg::Line(_) => return None,
        }
    };
    let d = node - h;
    (d.hypot() > 1e-9).then_some(node + d)
}

impl App {
    /// The end node of an open curve under `p` (page) that a new Pen or
    /// Bezier curve starts from to continue that curve: one of the
    /// selected curve's, else one of the topmost visible, unlocked open
    /// curve on the page that has one there. The curve, the end node
    /// (page) and the handle the next segment leaves along, the mirror of
    /// the end's own (#630).
    pub(crate) fn open_end_at(&self, p: Point) -> Option<(ShapeId, Point, Option<Point>)> {
        let tol = self.join_tolerance();
        let selected = self.join_candidate();
        let layers = self.page_layers();
        let others = layers
            .iter()
            .rev()
            .filter(|l| l.visible && !l.locked)
            .flat_map(|l| l.shapes.iter().rev())
            .filter(|s| s.visible && !s.locked && Some(s.id) != selected.as_ref().map(|c| c.id));
        selected
            .iter()
            .chain(others)
            .filter(|s| matches!(s.kind, ShapeKind::Path { .. }))
            .filter(|s| s.transform.determinant().abs() > 1e-12)
            .find_map(|s| {
                let subs = subpaths(&s.page_path());
                let (i, at_end) = end_near(&subs, p, tol)?;
                let sub = subs.get(i)?;
                let node = if at_end { sub.end() } else { sub.start };
                Some((s.id, node, mirrored_handle(sub, at_end)))
            })
    }

    /// Take the curve `id` up again with the Pen or Bezier `tool` as if
    /// it were still being drawn (#631): the tool is chosen, the curve
    /// selected (when it is still there), and a line goes on from `to`,
    /// the node it stood at with the handle it had; without one (the step
    /// closed the curve) the curve is only selected.
    pub(crate) fn resume_curve(
        &mut self,
        id: ShapeId,
        tool: crate::tools::Tool,
        to: Option<crate::app::PenEnd>,
    ) {
        if self.tool != tool {
            self.set_tool(tool);
        }
        if self.doc().shape_on_page(id).is_some() {
            self.select(vec![id]);
        } else {
            self.select(Vec::new());
        }
        if let Some(node) = to {
            self.curve = Some(crate::app::CurveInProgress {
                nodes: vec![node],
                smooth: true,
                resumed: true,
                ..Default::default()
            });
        }
    }

    /// The curve being drawn goes on from an end of the selected curve:
    /// the other end of that open subpath when `p` (page) is within
    /// `reach` of it, where a click closes the curve.
    pub(crate) fn continuation_close_at(&self, p: Point, reach: f64) -> Option<Point> {
        let c = self.curve.as_ref()?;
        if !c.smooth || c.bspline {
            return None;
        }
        let from = c.nodes.first()?.0;
        let curve = self.join_candidate()?;
        let subs = subpaths(&curve.page_path());
        subs.iter().filter(|s| !s.closed).find_map(|s| {
            let (a, b) = (s.start, s.end());
            let other = if (a - from).hypot() < 1e-6 {
                b
            } else if (b - from).hypot() < 1e-6 {
                a
            } else {
                return None;
            };
            ((other - p).hypot() <= reach && (other - from).hypot() > 1e-6).then_some(other)
        })
    }

    /// The selected curve a new line may join: one open curve lying on a
    /// layer.
    pub(crate) fn join_candidate(&self) -> Option<tracedraw_core::Shape> {
        let [id] = self.selection.as_slice() else {
            return None;
        };
        let (_, s) = self.doc().shape(*id).ok()?;
        match &s.kind {
            ShapeKind::Path { path, .. } if subpaths(path).iter().any(|s| !s.closed) => {
                Some(s.clone())
            }
            _ => None,
        }
    }

    /// The join distance in page millimetres.
    fn join_tolerance(&self) -> f64 {
        self.settings.curves.join_px() / f64::from(self.view.zoom.max(1e-6))
    }

    /// `p` (page) is on an end node of the selected open curve, where a new
    /// line would continue it.
    pub fn on_join_end(&self, p: Point) -> bool {
        self.join_candidate().is_some_and(|s| {
            end_near(&subpaths(&s.page_path()), p, self.join_tolerance()).is_some()
        })
    }

    /// Add a newly drawn open line (page space): it continues the selected
    /// curve when one of its ends is on one of the curve's end nodes,
    /// otherwise it becomes a curve of its own. The result is selected.
    /// A line drawn now would continue the selected curve (see
    /// [`Self::add_drawn_line`]).
    pub(crate) fn would_join(&self, line: &BezPath) -> bool {
        self.join_candidate().is_some_and(|curve| {
            joined(&curve.page_path(), line, self.join_tolerance()).is_some()
                && curve.transform.determinant().abs() > 1e-12
        })
    }

    pub fn add_drawn_line(&mut self, line: BezPath) -> Option<ShapeId> {
        self.add_drawn_line_with_widths(line, Vec::new())
    }

    /// `add_drawn_line`, with `widths` along the new line's outline (a
    /// pen's pressure, #850); a line that continues a curve keeps that
    /// curve's outline.
    pub fn add_drawn_line_with_widths(
        &mut self,
        line: BezPath,
        widths: Vec<tracedraw_core::style::WidthNode>,
    ) -> Option<ShapeId> {
        if let Some(curve) = self.join_candidate() {
            if let Some(path) = joined(&curve.page_path(), &line, self.join_tolerance()) {
                let det = curve.transform.determinant();
                if det.is_finite() && det.abs() > 1e-12 {
                    let local = curve.transform.inverse() * path;
                    let closed = subpaths(&local).iter().all(|s| s.closed);
                    self.run(Command::SetShapeKind {
                        shape: curve.id,
                        kind: ShapeKind::Path {
                            path: local,
                            closed,
                        },
                    });
                    self.select(vec![curve.id]);
                    return Some(curve.id);
                }
            }
        }
        let id = self.new_shape_styled(
            ShapeKind::Path {
                path: line,
                closed: false,
            },
            |shape| {
                if let Some(st) = shape.stroke.as_mut().filter(|_| !widths.is_empty()) {
                    st.widths = widths;
                }
            },
        )?;
        self.select(vec![id]);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;

    fn line(a: (f64, f64), b: (f64, f64)) -> BezPath {
        let mut p = BezPath::new();
        p.move_to(a);
        p.line_to(b);
        p
    }

    fn ends(path: &BezPath) -> Vec<(Point, Point, bool)> {
        subpaths(path)
            .iter()
            .map(|s| (s.start, s.end(), s.closed))
            .collect()
    }

    #[test]
    fn a_line_from_an_end_continues_the_curve() {
        let curve = line((0.0, 0.0), (10.0, 0.0));
        // From the end.
        let j = joined(&curve, &line((10.2, 0.1), (10.0, 10.0)), 0.5).expect("joined");
        assert_eq!(
            ends(&j),
            vec![(Point::new(0.0, 0.0), Point::new(10.0, 10.0), false)]
        );
        assert_eq!(subpaths(&j)[0].segs.len(), 2);
        // From the start: the curve runs the other way and goes on.
        let j = joined(&curve, &line((0.0, 0.2), (-5.0, 5.0)), 0.5).expect("joined");
        assert_eq!(
            ends(&j),
            vec![(Point::new(10.0, 0.0), Point::new(-5.0, 5.0), false)]
        );
        // Ending on an end node counts too.
        let j = joined(&curve, &line((5.0, 9.0), (10.0, 0.3)), 0.5).expect("joined");
        assert_eq!(
            ends(&j),
            vec![(Point::new(0.0, 0.0), Point::new(5.0, 9.0), false)]
        );
        // Away from both ends: nothing.
        assert!(joined(&curve, &line((5.0, 5.0), (8.0, 8.0)), 0.5).is_none());
    }

    #[test]
    fn a_line_between_the_two_ends_closes_the_curve() {
        let mut curve = line((0.0, 0.0), (10.0, 0.0));
        curve.line_to((10.0, 10.0));
        let j = joined(&curve, &line((10.0, 10.1), (0.1, 0.0)), 0.5).expect("joined");
        let e = ends(&j);
        assert_eq!(e.len(), 1);
        assert!(e[0].2, "closed");
        assert_eq!(e[0].1, Point::new(0.0, 0.0));
    }

    #[test]
    fn a_line_between_two_subpaths_makes_them_one() {
        let mut curve = line((0.0, 0.0), (10.0, 0.0));
        curve.extend(line((20.0, 0.0), (30.0, 0.0)));
        let j = joined(&curve, &line((10.0, 0.0), (20.0, 0.2)), 0.5).expect("joined");
        assert_eq!(
            ends(&j),
            vec![(Point::new(0.0, 0.0), Point::new(30.0, 0.0), false)]
        );
        assert_eq!(subpaths(&j)[0].segs.len(), 3);
    }

    #[test]
    fn reversing_keeps_curves_on_their_way() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.curve_to((1.0, 2.0), (3.0, 2.0), (4.0, 0.0));
        p.quad_to((5.0, -1.0), (6.0, 0.0));
        let s = &subpaths(&p)[0];
        let r = s.reversed();
        assert_eq!(r.start, Point::new(6.0, 0.0));
        assert_eq!(
            r.segs[0],
            Seg::Quad(Point::new(5.0, -1.0), Point::new(4.0, 0.0))
        );
        assert_eq!(
            r.segs[1],
            Seg::Cubic(
                Point::new(3.0, 2.0),
                Point::new(1.0, 2.0),
                Point::new(0.0, 0.0)
            )
        );
        assert_eq!(&r.reversed(), s);
    }

    #[test]
    fn drawn_lines_join_the_selected_curve_in_its_own_space() {
        let mut app = App::headless();
        app.view.zoom = 1.0;
        let id = app
            .add_drawn_line(line((10.0, 10.0), (50.0, 10.0)))
            .expect("line");
        // The curve is moved, so its space is not the page's.
        app.run(Command::TransformShapes {
            shapes: vec![id],
            transform: tracedraw_core::geometry::Affine::translate((5.0, 5.0)),
        });
        assert!(app.on_join_end(Point::new(55.0, 15.0)));
        assert!(!app.on_join_end(Point::new(30.0, 15.0)));
        let same = app
            .add_drawn_line(line((55.0, 15.0), (55.0, 60.0)))
            .expect("joined");
        assert_eq!(same, id);
        let s = app.doc().shape(id).expect("shape").1.clone();
        let b = s.bounds();
        assert!(
            (b.x0 - 15.0).abs() < 1e-9 && (b.y1 - 60.0).abs() < 1e-9,
            "{b:?}"
        );
        // One undo step takes the new part off again.
        app.undo();
        let b = app.doc().shape(id).expect("shape").1.bounds();
        assert!((b.y1 - 15.0).abs() < 1e-9);
        // Nothing selected: a curve of its own.
        app.select(Vec::new());
        let other = app
            .add_drawn_line(line((55.0, 15.0), (90.0, 15.0)))
            .expect("line");
        assert_ne!(other, id);
        // A closed curve has no ends to join.
        let r = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 100.0, 10.0, 110.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![r]);
        assert!(!app.on_join_end(Point::new(0.0, 100.0)));
    }
}
