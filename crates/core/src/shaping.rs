//! Shaping: weld, trim, intersect, simplify, front-minus-back, back-minus-
//! front and boundary. Implemented on flattened polygons with i_overlay
//! (0.02 mm tolerance); the curves of the sources are then put back into
//! the result ([`recover_curves`]), so a result keeps the Bezier segments
//! of the objects it came from, with new nodes only where their outlines
//! cross.

use crate::geometry::{BezPath, PathEl, Point};
use i_overlay::core::fill_rule::FillRule as IFill;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;
use kurbo::{ParamCurve as _, ParamCurveArclen as _};

const TOL: f64 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Weld,
    Intersect,
    /// Subject minus clip.
    Trim,
    /// Symmetric difference.
    Exclude,
}

/// Flatten a path into closed contours (one per subpath).
pub fn contours(path: &BezPath) -> Vec<Vec<[f64; 2]>> {
    let mut out: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut cur: Vec<[f64; 2]> = Vec::new();
    kurbo::flatten(path.elements().iter().copied(), TOL, &mut |el| match el {
        PathEl::MoveTo(p) => {
            if cur.len() >= 3 {
                out.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
            cur.push([p.x, p.y]);
        }
        PathEl::LineTo(p) => cur.push([p.x, p.y]),
        PathEl::ClosePath => {}
        _ => {}
    });
    if cur.len() >= 3 {
        out.push(cur);
    }
    out
}

/// Build a path from shapes (outer contour + holes each).
fn from_shapes(shapes: Vec<Vec<Vec<[f64; 2]>>>) -> BezPath {
    let mut path = BezPath::new();
    for shape in shapes {
        for contour in shape {
            let mut first = true;
            for p in contour {
                let pt = Point::new(p[0], p[1]);
                if first {
                    path.move_to(pt);
                    first = false;
                } else {
                    path.line_to(pt);
                }
            }
            path.close_path();
        }
    }
    path
}

/// Combine `subject` and `clip` (both in the same coordinate space).
pub fn overlay(subject: &BezPath, clip: &BezPath, op: Op) -> BezPath {
    let subj = contours(subject);
    let clp = contours(clip);
    let rule = match op {
        Op::Weld => OverlayRule::Union,
        Op::Intersect => OverlayRule::Intersect,
        Op::Trim => OverlayRule::Difference,
        Op::Exclude => OverlayRule::Xor,
    };
    let result = subj.overlay(&clp, rule, IFill::EvenOdd);
    recover_curves(&from_shapes(result), &[subject, clip])
}

/// Combine `subject` and `clip` as [`overlay`] does, each separate piece
/// of the result (its outline with its holes) a path of its own.
pub fn overlay_parts(subject: &BezPath, clip: &BezPath, op: Op) -> Vec<BezPath> {
    let subj = contours(subject);
    let clp = contours(clip);
    let rule = match op {
        Op::Weld => OverlayRule::Union,
        Op::Intersect => OverlayRule::Intersect,
        Op::Trim => OverlayRule::Difference,
        Op::Exclude => OverlayRule::Xor,
    };
    subj.overlay(&clp, rule, IFill::EvenOdd)
        .into_iter()
        .map(|shape| recover_curves(&from_shapes(vec![shape]), &[subject, clip]))
        .filter(|p| !p.elements().is_empty())
        .collect()
}

/// How near a vertex of a polygon result must lie to a source outline to
/// count as lying on it, mm: where two flattened outlines cross, the
/// crossing of the two chords lies up to [`TOL`] off either curve.
const ON_SOURCE: f64 = 1.5 * TOL;

/// A place on a source outline: the subpath, and its segment index plus
/// the parameter within the segment, one number that runs on across the
/// segment ends (a closed subpath's wraps back to 0).
#[derive(Clone, Copy, Debug)]
struct Place {
    sub: usize,
    u: f64,
}

/// A subpath of a source, ready for lookups.
struct SourceSub {
    segs: Vec<kurbo::PathSeg>,
    closed: bool,
    /// Each segment's box, grown by [`ON_SOURCE`].
    boxes: Vec<kurbo::Rect>,
}

impl SourceSub {
    fn len(&self) -> f64 {
        self.segs.len() as f64
    }

    /// The place of `p` on segment `i` of this subpath, added to `out`
    /// when it lies on it (two places coincide where it lies on a
    /// segment end; the second is dropped).
    fn place_on(&self, sub: usize, i: usize, p: Point, out: &mut Vec<Place>) {
        use kurbo::ParamCurveNearest as _;
        let (Some(seg), Some(bx)) = (self.segs.get(i), self.boxes.get(i)) else {
            return;
        };
        if !bx.contains(p) {
            return;
        }
        let near = seg.nearest(p, 1e-9);
        if near.distance_sq > ON_SOURCE * ON_SOURCE {
            return;
        }
        let n = self.len();
        let mut u = i as f64 + near.t.clamp(0.0, 1.0);
        // On a segment end: that end exactly, so no sliver of the
        // segment is left over.
        if (u - u.round()).abs() < 1e-6 {
            u = u.round();
        }
        if self.closed && u >= n - 1e-9 {
            u = 0.0;
        }
        if out
            .iter()
            .any(|q| q.sub == sub && self.delta(q.u, u).abs() < 1e-6)
        {
            return;
        }
        out.push(Place { sub, u });
    }

    /// From `a` to `b` along the subpath: the signed parameter distance,
    /// the shorter way round for a closed one.
    fn delta(&self, a: f64, b: f64) -> f64 {
        let d = b - a;
        if self.closed {
            let n = self.len();
            let d = d.rem_euclid(n);
            if d > n / 2.0 {
                d - n
            } else {
                d
            }
        } else {
            d
        }
    }

    /// The point `du` along from `u`.
    fn at(&self, u: f64, du: f64) -> Point {
        use kurbo::ParamCurve as _;
        let n = self.len();
        let mut v = u + du;
        if self.closed {
            v = v.rem_euclid(n);
        }
        let v = v.clamp(0.0, n);
        let i = (v.floor() as usize).min(self.segs.len() - 1);
        self.segs[i].eval(v - i as f64)
    }

    /// The pieces of the subpath from `u` on by `du` (back for a
    /// negative `du`), appended to `out`, which stands at the start.
    fn pieces(&self, u: f64, du: f64, out: &mut BezPath) {
        use kurbo::{ParamCurve as _, PathSeg};
        let n = self.len();
        let count = self.segs.len();
        let mut left = du.abs();
        let mut cur = if self.closed { u.rem_euclid(n) } else { u };
        let mut guard = 0;
        while left > 1e-9 && guard < count + 2 {
            guard += 1;
            if du < 0.0 && self.closed && cur <= 1e-12 {
                cur = n;
            }
            let (i, t0, t1, step) = if du > 0.0 {
                let i = (cur.floor() as usize).min(count - 1);
                let t0 = cur - i as f64;
                let t1 = (t0 + left).min(1.0);
                (i, t0, t1, t1 - t0)
            } else {
                let i = (cur.ceil() as usize).max(1).min(count) - 1;
                let t1 = cur - i as f64;
                let t0 = (t1 - left).max(0.0);
                (i, t0, t1, t1 - t0)
            };
            if step <= 1e-12 {
                break;
            }
            let mut piece = self.segs[i].subsegment(t0..t1);
            if du < 0.0 {
                piece = piece.reverse();
            }
            match piece {
                PathSeg::Line(l) => out.line_to(l.p1),
                PathSeg::Quad(q) => out.quad_to(q.p1, q.p2),
                PathSeg::Cubic(c) => out.curve_to(c.p1, c.p2, c.p3),
            }
            left -= step;
            cur = if du > 0.0 {
                i as f64 + t1
            } else {
                i as f64 + t0
            };
            if self.closed {
                cur = cur.rem_euclid(n);
            } else if (du > 0.0 && cur >= n - 1e-12) || (du < 0.0 && cur <= 1e-12) {
                break;
            }
        }
    }
}

/// A stretch of the result lying along one source subpath: where it
/// started on it, how far it has come, and which way.
struct Run {
    sub: usize,
    u_start: f64,
    du: f64,
}

/// A grid over the sources' segments, so the segments near a point are
/// found without a look at every one (a block shadow welds thousands).
struct SegmentGrid {
    x0: f64,
    y0: f64,
    cell: f64,
    nx: usize,
    ny: usize,
    /// Per cell: the (subpath, segment) pairs whose boxes cover it.
    cells: Vec<Vec<(usize, usize)>>,
}

impl SegmentGrid {
    fn new(subs: &[SourceSub]) -> Self {
        let mut bounds: Option<kurbo::Rect> = None;
        let mut count = 0usize;
        for sub in subs {
            for bx in &sub.boxes {
                bounds = Some(bounds.map_or(*bx, |b| b.union(*bx)));
                count += 1;
            }
        }
        let bounds = bounds.unwrap_or(kurbo::Rect::ZERO);
        // About one segment per cell, within reason.
        let side = ((count as f64).sqrt().ceil() as usize).clamp(1, 512);
        let cell = (bounds.width().max(bounds.height()) / side as f64).max(1e-6);
        let nx = ((bounds.width() / cell).ceil() as usize).max(1);
        let ny = ((bounds.height() / cell).ceil() as usize).max(1);
        let mut grid = SegmentGrid {
            x0: bounds.x0,
            y0: bounds.y0,
            cell,
            nx,
            ny,
            cells: vec![Vec::new(); nx * ny],
        };
        for (k, sub) in subs.iter().enumerate() {
            for (i, bx) in sub.boxes.iter().enumerate() {
                let (cx0, cy0) = grid.cell_of(Point::new(bx.x0, bx.y0));
                let (cx1, cy1) = grid.cell_of(Point::new(bx.x1, bx.y1));
                for cy in cy0..=cy1 {
                    for cx in cx0..=cx1 {
                        grid.cells[cy * nx + cx].push((k, i));
                    }
                }
            }
        }
        grid
    }

    fn cell_of(&self, p: Point) -> (usize, usize) {
        let cx = ((p.x - self.x0) / self.cell).floor().max(0.0) as usize;
        let cy = ((p.y - self.y0) / self.cell).floor().max(0.0) as usize;
        (cx.min(self.nx - 1), cy.min(self.ny - 1))
    }

    /// The segments whose boxes may hold `p`.
    fn near(&self, p: Point) -> &[(usize, usize)] {
        if p.x < self.x0 || p.y < self.y0 {
            return &[];
        }
        let (cx, cy) = self.cell_of(p);
        if (p.x - self.x0) / self.cell >= self.nx as f64
            || (p.y - self.y0) / self.cell >= self.ny as f64
        {
            return &[];
        }
        &self.cells[cy * self.nx + cx]
    }
}

/// `result`, a polygon path made from the flattened `sources`, with the
/// sources' curves put back: a vertex lying on a source outline is placed
/// on it, and a stretch of consecutive vertices along one source subpath
/// becomes that stretch of the subpath itself, segments, handles and all,
/// instead of its flattening. Vertices on no source (none, as a rule) and
/// edges that do not follow a source (a chord across an arc, a source
/// edge the result leaves early) stay straight lines, so the result is
/// never worse than the polygon.
fn recover_curves(result: &BezPath, sources: &[&BezPath]) -> BezPath {
    let curved = |p: &BezPath| {
        p.elements()
            .iter()
            .any(|e| matches!(e, PathEl::QuadTo(..) | PathEl::CurveTo(..)))
    };
    // Nothing to put back when the sources are polygons themselves.
    if curved(result) || !sources.iter().any(|p| curved(p)) {
        return result.clone();
    }
    let subs: Vec<SourceSub> = sources
        .iter()
        .flat_map(|p| subpath_segments(p))
        .filter(|(segs, _)| !segs.is_empty())
        .map(|(segs, closed)| {
            use kurbo::Shape as _;
            let boxes = segs
                .iter()
                .map(|s| s.bounding_box().inflate(ON_SOURCE, ON_SOURCE))
                .collect();
            SourceSub {
                segs,
                closed,
                boxes,
            }
        })
        .collect();
    if subs.is_empty() {
        return result.clone();
    }
    let grid = SegmentGrid::new(&subs);
    // The contours of the result.
    let mut contours: Vec<Vec<Point>> = Vec::new();
    for el in result.elements() {
        match *el {
            PathEl::MoveTo(p) => contours.push(vec![p]),
            PathEl::LineTo(p) => {
                if let Some(c) = contours.last_mut() {
                    c.push(p);
                }
            }
            _ => {}
        }
    }
    let mut out = BezPath::new();
    for mut pts in contours {
        if pts.len() > 1 && (pts[pts.len() - 1] - pts[0]).hypot() < 1e-9 {
            pts.pop();
        }
        if pts.len() < 3 {
            continue;
        }
        let n = pts.len();
        let mut places: Vec<Vec<Place>> = pts
            .iter()
            .map(|p| {
                let mut v = Vec::new();
                for (k, i) in grid.near(*p) {
                    subs[*k].place_on(*k, *i, *p, &mut v);
                }
                v
            })
            .collect();
        // The contour starts where a run would end anyway (a crossing, a
        // vertex on no source, or a source's own node), so a stretch of
        // one source is not cut in two at an arbitrary vertex.
        let boundary =
            |pl: &Vec<Place>| pl.len() != 1 || pl.iter().any(|q| (q.u - q.u.round()).abs() < 1e-6);
        if let Some(first) = (0..n).find(|k| boundary(&places[*k])) {
            pts.rotate_left(first);
            places.rotate_left(first);
        }
        out.move_to(pts[0]);
        // The run ending at the vertex before the one in hand; `None` when
        // that vertex was reached by a straight line.
        let mut run: Option<Run> = None;
        for k in 1..=n {
            let prev = pts[k - 1];
            let p = pts[k % n];
            let here = &places[k % n];
            let before = &places[(k - 1) % n];
            // The step from `prev` to `p` along a source: `(place of prev,
            // the parameter distance)`, when the source follows the edge.
            let follows = |sub_k: usize, ua: f64, ub: f64| -> Option<f64> {
                let sub = &subs[sub_k];
                let du = sub.delta(ua, ub);
                if du.abs() < 1e-9 {
                    return None;
                }
                // The flattening put each edge within TOL of the curve; a
                // chord across more of it lies farther from it.
                let chord = kurbo::Line::new(prev, p);
                for f in [0.25, 0.5, 0.75] {
                    use kurbo::ParamCurveNearest as _;
                    let q = sub.at(ua, du * f);
                    if chord.nearest(q, 1e-9).distance_sq > (2.0 * TOL) * (2.0 * TOL) {
                        return None;
                    }
                }
                Some(du)
            };
            // Extend the run in hand?
            let mut extended = false;
            if let Some(r) = run.as_mut() {
                let u_last = r.u_start + r.du;
                let same = here
                    .iter()
                    .filter(|c| c.sub == r.sub)
                    .find_map(|c| follows(r.sub, u_last, c.u));
                if let Some(du) = same {
                    if (du > 0.0) == (r.du > 0.0) {
                        r.du += du;
                        extended = true;
                    }
                }
            }
            if extended {
                continue;
            }
            // Not along the run: the run ends at `prev`, drawn as it is.
            if let Some(r) = run.take() {
                subs[r.sub].pieces(r.u_start, r.du, &mut out);
            }
            // A new run from `prev` to `p` along a source both lie on?
            let start = before.iter().find_map(|a| {
                here.iter()
                    .filter(|b| b.sub == a.sub)
                    .find_map(|b| follows(a.sub, a.u, b.u).map(|du| (a.sub, a.u, du)))
            });
            match start {
                Some((sub, u_start, du)) => run = Some(Run { sub, u_start, du }),
                None => out.line_to(p),
            }
        }
        if let Some(r) = run.take() {
            subs[r.sub].pieces(r.u_start, r.du, &mut out);
        }
        // The close does the last straight edge back to the start.
        if let Some(PathEl::LineTo(q)) = out.elements().last() {
            if (*q - pts[0]).hypot() < 1e-9 {
                out.pop();
            }
        }
        out.close_path();
    }
    out
}

/// What [`enclosed_area`] finds around a point.
#[derive(Debug, Clone, PartialEq)]
pub enum Enclosed {
    /// The area the outlines enclose, with the holes the outlines inside
    /// it make, its sides the outlines' own curves.
    Area(BezPath),
    /// The point lies in an area the outlines leave open to the outside.
    Open,
    /// The point lies on an outline.
    OnOutline,
}

/// The widest gap, mm, between the end of an open outline and another
/// outline (or its own start) that [`enclosed_area`] closes: the hairline
/// gaps files carry from rounding (a few ten thousandths of a millimetre,
/// #899), not a gap anyone drew.
const GAP: f64 = 0.01;

/// The short lines that close the gaps of at most [`GAP`] between the ends
/// of the open `lines` (`open[i]`: line `i` is open) and the lines
/// themselves, each running from the end to the nearest point of the
/// outline it nearly meets and a little beyond, so that it crosses it.
fn gap_bridges(lines: &[Vec<[f64; 2]>], open: &[bool]) -> Vec<Vec<[f64; 2]>> {
    // A grid over the segments, each in the cells its box (widened by the
    // gap) covers, so an end finds its neighbours without a look at all.
    let mut segs: Vec<(usize, usize)> = Vec::new();
    let mut bounds: Option<kurbo::Rect> = None;
    for (i, l) in lines.iter().enumerate() {
        for (k, w) in l.windows(2).enumerate() {
            let bx = kurbo::Rect::from_points((w[0][0], w[0][1]), (w[1][0], w[1][1]));
            bounds = Some(bounds.map_or(bx, |b| b.union(bx)));
            segs.push((i, k));
        }
    }
    let Some(bounds) = bounds else {
        return Vec::new();
    };
    if !open.iter().any(|o| *o) {
        return Vec::new();
    }
    let side = ((segs.len() as f64).sqrt().ceil() as usize).clamp(1, 512);
    let cell = (bounds.width().max(bounds.height()) / side as f64).max(4.0 * GAP);
    let nx = ((bounds.width() / cell).floor() as usize + 1).max(1);
    let ny = ((bounds.height() / cell).floor() as usize + 1).max(1);
    let cell_of = |x: f64, y: f64| {
        let cx = (((x - bounds.x0) / cell).floor().max(0.0) as usize).min(nx - 1);
        let cy = (((y - bounds.y0) / cell).floor().max(0.0) as usize).min(ny - 1);
        (cx, cy)
    };
    let mut cells: Vec<Vec<usize>> = vec![Vec::new(); nx * ny];
    for (n, (i, k)) in segs.iter().enumerate() {
        let (a, b) = (lines[*i][*k], lines[*i][*k + 1]);
        let (cx0, cy0) = cell_of(a[0].min(b[0]) - GAP, a[1].min(b[1]) - GAP);
        let (cx1, cy1) = cell_of(a[0].max(b[0]) + GAP, a[1].max(b[1]) + GAP);
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                cells[cy * nx + cx].push(n);
            }
        }
    }
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if !open[i] || l.len() < 2 {
            continue;
        }
        // The two ends, each with the segment it ends (that one is no
        // neighbour of it).
        for (end, own) in [(l[0], 0), (l[l.len() - 1], l.len() - 2)] {
            let p = Point::new(end[0], end[1]);
            let (cx, cy) = cell_of(end[0], end[1]);
            let mut best: Option<(f64, Point)> = None;
            for n in &cells[cy * nx + cx] {
                let (j, k) = segs[*n];
                if j == i && k == own {
                    continue;
                }
                let seg = kurbo::Line::new(
                    Point::new(lines[j][k][0], lines[j][k][1]),
                    Point::new(lines[j][k + 1][0], lines[j][k + 1][1]),
                );
                use kurbo::ParamCurveNearest as _;
                let nearest = seg.nearest(p, 1e-9);
                let d = nearest.distance_sq.sqrt();
                if d < GAP && best.is_none_or(|(b, _)| d < b) {
                    best = Some((d, seg.eval(nearest.t)));
                }
            }
            if let Some((d, q)) = best {
                // Already touching: nothing to close.
                if d > 1e-9 {
                    let past = q + (q - p) * 0.5;
                    out.push(vec![end, [past.x, past.y]]);
                }
            }
        }
    }
    out
}

/// The area around `p` that the outlines `paths` (page coordinates; open
/// and closed subpaths alike, a subpath crossing itself enclosing the
/// areas between its crossings) close off: the outlines, flattened, cut
/// a box around them all into faces, the face holding `p` is the area,
/// and the outlines' curves are put back into its sides
/// ([`recover_curves`]), so the area meets them exactly and has nodes
/// only where they cross.
pub fn enclosed_area(paths: &[BezPath], p: Point) -> Enclosed {
    use i_overlay::float::slice::FloatSlice as _;
    use kurbo::Shape as _;
    let mut lines: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut open_ends: Vec<bool> = Vec::new();
    for (pts, closed) in paths.iter().flat_map(|path| polylines(path, TOL)) {
        let l: Vec<[f64; 2]> = pts.iter().map(|q| [q.x, q.y]).collect();
        if l.iter().all(|q| q[0].is_finite() && q[1].is_finite()) {
            lines.push(l);
            open_ends.push(!closed);
        }
    }
    let bridges = gap_bridges(&lines, &open_ends);
    lines.extend(bridges);
    let Some(bounds) = lines
        .iter()
        .flatten()
        .map(|q| kurbo::Rect::from_points((q[0], q[1]), (q[0], q[1])))
        .reduce(|a, b| a.union(b))
    else {
        return Enclosed::Open;
    };
    // A margin all round: the faces reaching it are open.
    let margin = 2.0;
    let domain = bounds.inflate(margin, margin);
    if !domain.contains(p) {
        return Enclosed::Open;
    }
    let frame = vec![
        [domain.x0, domain.y0],
        [domain.x1, domain.y0],
        [domain.x1, domain.y1],
        [domain.x0, domain.y1],
    ];
    let faces = frame.slice_by(&lines, IFill::NonZero);
    for face in faces {
        let area = from_shapes(vec![face.clone()]);
        if !area.contains(p) {
            continue;
        }
        let open = face.iter().flatten().any(|q| {
            (q[0] - domain.x0).abs() < margin / 2.0
                || (q[0] - domain.x1).abs() < margin / 2.0
                || (q[1] - domain.y0).abs() < margin / 2.0
                || (q[1] - domain.y1).abs() < margin / 2.0
        });
        if open {
            return Enclosed::Open;
        }
        let sources: Vec<&BezPath> = paths.iter().collect();
        return Enclosed::Area(recover_curves(&area, &sources));
    }
    Enclosed::OnOutline
}

/// How an offset outline turns its corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Join {
    /// Sharp, the sides run on until they meet (very sharp corners, under
    /// about 9 degrees, are cut off).
    #[default]
    Miter,
    Round,
    /// Cut off straight.
    Bevel,
}

/// The smallest corner a mitred join keeps sharp, radians.
const MITER_MIN: f64 = 0.15;

/// Offset a closed path outward (positive) or inward (negative) by `d` mm,
/// with round corners. Used by Contour and outline-to-object.
pub fn offset(path: &BezPath, d: f64) -> BezPath {
    offset_with(path, d, Join::Round)
}

/// Offset a closed path outward (positive) or inward (negative) by `d` mm,
/// the corners turned as `join` says.
pub fn offset_with(path: &BezPath, d: f64, join: Join) -> BezPath {
    use i_overlay::float::simplify::SimplifyShape;
    use i_overlay::mesh::float::outline::offset::OutlineOffset;
    use i_overlay::mesh::float::style::{LineJoin, OutlineStyle};
    if !d.is_finite() {
        return BezPath::new();
    }
    let subj = contours(path);
    // Simplify guarantees orientation (outer CCW, holes CW) and no self-intersections.
    let clean = subj.simplify_shape(IFill::EvenOdd);
    let style = OutlineStyle::new(d).line_join(match join {
        Join::Miter => LineJoin::Miter(MITER_MIN),
        Join::Round => LineJoin::Round(0.05),
        Join::Bevel => LineJoin::Bevel,
    });
    let out = clean.outline(&style);
    fit_polygon_curves(&from_shapes(out), FIT_TOL)
}

/// How far the curves an offset is fitted with may stray from it, mm:
/// well under what shows or prints.
const FIT_TOL: f64 = 0.05;

/// A piece this many times as long as the one next to it is a straight
/// side, not part of a curve.
const STRAIGHT_RATIO: f64 = 4.0;

/// How sharply a polygon must turn at a point, in degrees, for the point
/// to be a corner [`fit_polygon_curves`] keeps.
const CORNER_DEG: f64 = 35.0;

/// A path of straight pieces (an offset, as the overlay library gives it)
/// as curves: each contour split at its corners, and every smooth stretch
/// between them fitted with as few curves as keep within `accuracy` mm
/// (#821). A contour of fewer than four points stays as it is.
pub fn fit_polygon_curves(path: &BezPath, accuracy: f64) -> BezPath {
    let mut out = BezPath::new();
    for mut pts in polygons(path) {
        while pts.len() > 1 && (pts[0] - pts[pts.len() - 1]).hypot() < 1e-9 {
            pts.pop();
        }
        // The points that matter within the tolerance: the overlay's
        // tiny joins between pieces would read as corners and sides.
        if pts.len() > 4 {
            let mut ring = pts.clone();
            ring.push(pts[0]);
            let mut few = crate::geometry::simplify(&ring, accuracy * 0.5);
            few.pop();
            if few.len() >= 3 {
                pts = few;
            }
        }
        let n = pts.len();
        if n < 4 {
            if let Some((first, rest)) = pts.split_first() {
                out.move_to(*first);
                rest.iter().for_each(|p| out.line_to(*p));
                out.close_path();
            }
            continue;
        }
        let limit = CORNER_DEG.to_radians();
        let corner = |i: usize| {
            let a = pts[i] - pts[(i + n - 1) % n];
            let b = pts[(i + 1) % n] - pts[i];
            let (la, lb) = (a.hypot(), b.hypot());
            // Where a long straight side meets short pieces (a curve), the
            // side stays a line.
            let side = la > STRAIGHT_RATIO * lb || lb > STRAIGHT_RATIO * la;
            la < 1e-12
                || lb < 1e-12
                || side
                || (a.dot(b) / (la * lb)).clamp(-1.0, 1.0).acos() > limit
        };
        let corners: Vec<usize> = (0..n).filter(|i| corner(*i)).collect();
        if corners.is_empty() {
            let fitted = fit_run(&pts, true, accuracy);
            out.extend(
                fitted
                    .elements()
                    .iter()
                    .copied()
                    .filter(|e| !matches!(e, PathEl::ClosePath)),
            );
            out.close_path();
            continue;
        }
        // From corner to corner, all the way round.
        let start = corners[0];
        out.move_to(pts[start]);
        for (k, &c) in corners.iter().enumerate() {
            let next = corners[(k + 1) % corners.len()];
            let mut run = vec![pts[c]];
            let mut i = (c + 1) % n;
            loop {
                run.push(pts[i]);
                if i == next {
                    break;
                }
                i = (i + 1) % n;
            }
            if run.len() <= 2 {
                out.line_to(run[run.len() - 1]);
                continue;
            }
            let fitted = fit_run(&run, false, accuracy);
            out.extend(
                fitted
                    .elements()
                    .iter()
                    .copied()
                    .filter(|e| !matches!(e, PathEl::MoveTo(_))),
            );
        }
        out.close_path();
    }
    out
}

/// The points of each subpath of a path of straight pieces.
fn polygons(path: &BezPath) -> Vec<Vec<Point>> {
    let mut out: Vec<Vec<Point>> = Vec::new();
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => out.push(vec![p]),
            PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => {
                if let Some(last) = out.last_mut() {
                    last.push(p);
                }
            }
            PathEl::ClosePath => {}
        }
    }
    out
}

/// A run of points (a whole ring when `closed`) as a smooth curve
/// through them, fitted with as few curves as keep within `accuracy`
/// (kurbo's fitter merges curves, not lines); straight pieces when the
/// fit fails.
fn fit_run(pts: &[Point], closed: bool, accuracy: f64) -> BezPath {
    use kurbo::simplify::{simplify_bezpath, SimplifyOptions};
    let smooth = crate::geometry::smooth_path(pts, closed);
    let options = SimplifyOptions::default();
    let fitted = simplify_bezpath(smooth.elements().iter().copied(), accuracy, &options);
    let sane = fitted.elements().len() > 1
        && fitted.elements().iter().all(|e| match *e {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => p.is_finite(),
            PathEl::QuadTo(a, p) => a.is_finite() && p.is_finite(),
            PathEl::CurveTo(a, b, p) => a.is_finite() && b.is_finite() && p.is_finite(),
            PathEl::ClosePath => true,
        });
    if sane {
        return fitted;
    }
    let mut line = BezPath::new();
    if let Some((first, rest)) = pts.split_first() {
        line.move_to(*first);
        rest.iter().for_each(|p| line.line_to(*p));
    }
    line
}

/// The area swept by a round or square nib of width `width` along a
/// polyline (the Eraser's band). A single point gives a dot.
pub fn stroke_band(points: &[Point], width: f64, square: bool) -> BezPath {
    use crate::geometry::Shape as _;
    use i_overlay::mesh::float::stroke::offset::StrokeOffset;
    use i_overlay::mesh::float::style::{LineCap, LineJoin, StrokeStyle};
    if points.is_empty() || width <= 0.0 {
        return BezPath::new();
    }
    let r = width / 2.0;
    if points.len() == 1 || points.iter().all(|p| (*p - points[0]).hypot() < 1e-9) {
        let c = points[0];
        return if square {
            crate::geometry::Rect::new(c.x - r, c.y - r, c.x + r, c.y + r).to_path(0.01)
        } else {
            crate::geometry::Circle::new(c, r).to_path(0.01)
        };
    }
    let pts: Vec<[f64; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
    let style = if square {
        StrokeStyle::new(width)
            .line_join(LineJoin::Miter(MITER_MIN))
            .start_cap(LineCap::Square)
            .end_cap(LineCap::Square)
    } else {
        StrokeStyle::new(width)
            .line_join(LineJoin::Round(0.05))
            .start_cap(LineCap::Round(0.05))
            .end_cap(LineCap::Round(0.05))
    };
    from_shapes(pts.stroke(style, false))
}

/// Area swept by a calligraphic nib along a path: an ellipse of width
/// `width` along `nib_angle_deg` and `width * stretch` across it. Each
/// flattened segment contributes the parallelogram the nib's support
/// sweeps, each vertex the nib itself; the union of them is the outline
/// (open subpaths get nib-shaped ends). `stretch` 1 is the round pen.
pub fn calligraphic_band(path: &BezPath, width: f64, stretch: f64, nib_angle_deg: f64) -> BezPath {
    if width <= 0.0 {
        return BezPath::new();
    }
    let a = width / 2.0;
    let b = a * stretch.clamp(0.01, 1.0);
    let ang = nib_angle_deg.to_radians();
    let e1 = [ang.cos(), ang.sin()];
    let e2 = [-ang.sin(), ang.cos()];
    // Polylines of every subpath, closed ones with the first point repeated.
    let mut lines: Vec<Vec<Point>> = Vec::new();
    let mut cur: Vec<Point> = Vec::new();
    kurbo::flatten(path.elements().iter().copied(), 0.05, &mut |el| match el {
        PathEl::MoveTo(p) => {
            if cur.len() >= 2 {
                lines.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
            cur.push(p);
        }
        PathEl::LineTo(p) => cur.push(p),
        PathEl::ClosePath => {
            if let Some(f) = cur.first().copied() {
                cur.push(f);
            }
        }
        _ => {}
    });
    if !cur.is_empty() {
        lines.push(cur);
    }
    let ellipse = |c: Point| -> Vec<[f64; 2]> {
        (0..24)
            .map(|i| {
                let t = i as f64 / 24.0 * std::f64::consts::TAU;
                let (x, y) = (a * t.cos(), b * t.sin());
                [c.x + x * e1[0] + y * e2[0], c.y + x * e1[1] + y * e2[1]]
            })
            .collect()
    };
    let mut polys: Vec<Vec<[f64; 2]>> = Vec::new();
    for line in &lines {
        for (i, p) in line.iter().enumerate() {
            polys.push(ellipse(*p));
            let Some(q) = line.get(i + 1) else {
                continue;
            };
            let d = *q - *p;
            let len = d.hypot();
            if len < 1e-9 {
                continue;
            }
            let n = [-d.y / len, d.x / len];
            // Support half-width of the ellipse along n.
            let h = ((a * (n[0] * e1[0] + n[1] * e1[1])).powi(2)
                + (b * (n[0] * e2[0] + n[1] * e2[1])).powi(2))
            .sqrt();
            if h < 1e-9 {
                continue;
            }
            polys.push(vec![
                [p.x + n[0] * h, p.y + n[1] * h],
                [q.x + n[0] * h, q.y + n[1] * h],
                [q.x - n[0] * h, q.y - n[1] * h],
                [p.x - n[0] * h, p.y - n[1] * h],
            ]);
        }
    }
    // Consistent orientation so the non-zero union adds up.
    for poly in polys.iter_mut() {
        let area: f64 = poly
            .iter()
            .zip(poly.iter().cycle().skip(1))
            .map(|(p, q)| p[0] * q[1] - q[0] * p[1])
            .sum();
        if area < 0.0 {
            poly.reverse();
        }
    }
    let empty: Vec<Vec<[f64; 2]>> = Vec::new();
    from_shapes(polys.overlay(&empty, OverlayRule::Subject, IFill::NonZero))
}

/// The polylines of every subpath (flattened to `tol`), closed ones with
/// the first point repeated at the end.
fn polylines(path: &BezPath, tol: f64) -> Vec<(Vec<Point>, bool)> {
    let mut lines: Vec<(Vec<Point>, bool)> = Vec::new();
    let mut cur: Vec<Point> = Vec::new();
    kurbo::flatten(path.elements().iter().copied(), tol, &mut |el| match el {
        PathEl::MoveTo(p) => {
            if cur.len() >= 2 {
                lines.push((std::mem::take(&mut cur), false));
            } else {
                cur.clear();
            }
            cur.push(p);
        }
        PathEl::LineTo(p) => cur.push(p),
        PathEl::ClosePath => {
            if let Some(f) = cur.first().copied() {
                cur.push(f);
            }
            if cur.len() >= 2 {
                lines.push((std::mem::take(&mut cur), true));
            } else {
                cur.clear();
            }
        }
        _ => {}
    });
    if cur.len() >= 2 {
        lines.push((cur, false));
    }
    lines
}

/// Total length of a path's flattened subpaths.
pub fn path_length(path: &BezPath) -> f64 {
    polylines(path, 0.05)
        .iter()
        .map(|(l, _)| l.windows(2).map(|w| (w[1] - w[0]).hypot()).sum::<f64>())
        .sum()
}

/// Where along the path (0 to 1, by length over all subpaths) the point
/// nearest to `p` is, its distance and which side of the path's direction
/// `p` is on (true: left).
pub fn locate_on_path(path: &BezPath, p: Point) -> Option<(f64, f64, bool)> {
    let lines = polylines(path, 0.05);
    let total: f64 = lines
        .iter()
        .map(|(l, _)| l.windows(2).map(|w| (w[1] - w[0]).hypot()).sum::<f64>())
        .sum();
    if total <= 1e-12 {
        return None;
    }
    let mut best: Option<(f64, f64, bool)> = None;
    let mut run = 0.0;
    for (line, _) in &lines {
        for w in line.windows(2) {
            let (a, b) = (w[0], w[1]);
            let d = b - a;
            let len2 = d.x * d.x + d.y * d.y;
            if len2 <= 1e-18 {
                continue;
            }
            let len = len2.sqrt();
            let u = (((p - a).x * d.x + (p - a).y * d.y) / len2).clamp(0.0, 1.0);
            let q = a + d * u;
            let dist = (p - q).hypot();
            if best.map(|(_, bd, _)| dist < bd).unwrap_or(true) {
                let cross = d.x * (p - a).y - d.y * (p - a).x;
                best = Some(((run + u * len) / total, dist, cross >= 0.0));
            }
            run += len;
        }
    }
    best
}

/// The point `t` of the way along the path (0 to 1, by length over all
/// subpaths) and the unit normal to the left of its direction there.
pub fn point_on_path(path: &BezPath, t: f64) -> Option<(Point, [f64; 2])> {
    let lines = polylines(path, 0.05);
    let total: f64 = lines
        .iter()
        .map(|(l, _)| l.windows(2).map(|w| (w[1] - w[0]).hypot()).sum::<f64>())
        .sum();
    if total <= 1e-12 {
        return None;
    }
    let want = t.clamp(0.0, 1.0) * total;
    let mut run = 0.0;
    let mut last = None;
    for (line, _) in &lines {
        for w in line.windows(2) {
            let d = w[1] - w[0];
            let len = d.hypot();
            if len <= 1e-12 {
                continue;
            }
            let n = [-d.y / len, d.x / len];
            if run + len >= want {
                let u = ((want - run) / len).clamp(0.0, 1.0);
                return Some((w[0] + d * u, n));
            }
            run += len;
            last = Some((w[1], n));
        }
    }
    last
}

/// The area of a variable-width outline: every segment of the flattened
/// path widened by the reach `width_at` gives at its ends (left and right
/// separately), joined with wedges at the corners; butt ends.
pub fn variable_band(path: &BezPath, base: f64, nodes: &[crate::style::WidthNode]) -> BezPath {
    variable_band_within(path, base, nodes, 0.05)
}

/// [`variable_band`] with the path flattened to `tol` (mm): finer for a
/// band that is kept as an object and fitted to curves, so the corners
/// at the joins of the short lines stay small (#627).
pub fn variable_band_within(
    path: &BezPath,
    base: f64,
    nodes: &[crate::style::WidthNode],
    tol: f64,
) -> BezPath {
    let lines = polylines(path, tol.max(1e-4));
    let total: f64 = lines
        .iter()
        .map(|(l, _)| l.windows(2).map(|w| (w[1] - w[0]).hypot()).sum::<f64>())
        .sum();
    if total <= 1e-12 {
        return BezPath::new();
    }
    let reach = |s: f64| crate::style::width_at(base, nodes, s / total);
    // Short pieces so the width can change along long straight segments,
    // split at the nodes so their reach is met exactly.
    let step = (total / 256.0).max(0.05);
    let mut marks: Vec<f64> = nodes.iter().map(|n| n.at.clamp(0.0, 1.0) * total).collect();
    marks.sort_by(|a, b| a.total_cmp(b));
    // Between neighbouring nodes whose reach differs the width eases in
    // and out: sampled finely there, however short the span, so the band
    // curves smoothly instead of running in a few straight pieces (#492).
    let mut ends = vec![0.0];
    ends.extend(marks.iter().copied());
    ends.push(total);
    let mut eased = Vec::new();
    for w in ends.windows(2) {
        let (a, b) = (w[0], w[1]);
        let ((la, ra), (lb, rb)) = (reach(a), reach(b));
        if b - a <= 1e-9 || (la, ra) == (lb, rb) {
            continue;
        }
        // Enough samples that the straight pieces stay within
        // `EASE_TOL` of the eased edge however far the reach changes: a
        // smoothstep from `0` to `dw` over a span bends at most
        // `6 dw / span^2`, and a piece `h` long strays `h^2 / 8` of that
        // (#776).
        const EASE_TOL: f64 = 0.005;
        let dw = (la - lb).abs().max((ra - rb).abs());
        let steps = ((6.0 * dw / (8.0 * EASE_TOL)).sqrt().ceil() as usize).clamp(48, 512);
        eased.extend((1..steps).map(|i| a + (b - a) * i as f64 / steps as f64));
    }
    marks.extend(eased);
    marks.sort_by(|a, b| a.total_cmp(b));
    let mut polys: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut run = 0.0;
    for (line, closed) in &lines {
        // Resampled points with their length along the whole path, and
        // the unit normal (to the left) of each piece. The pieces of one
        // segment share its normal: computed piece by piece, rounding
        // tilted them apart, and the union left cracks between them that
        // showed as spikes and holes (#627).
        let mut pts: Vec<(Point, f64)> = Vec::new();
        let mut normals: Vec<Option<[f64; 2]>> = Vec::new();
        for w in line.windows(2) {
            let (a, b) = (w[0], w[1]);
            let len = (b - a).hypot();
            if pts.is_empty() {
                pts.push((a, run));
            }
            if len <= 1e-12 {
                continue;
            }
            let d = b - a;
            let n = Some([-d.y / len, d.x / len]);
            let k = ((len / step).ceil() as usize).clamp(1, 4096);
            let mut cuts: Vec<f64> = (1..k).map(|i| i as f64 / k as f64).collect();
            cuts.extend(
                marks
                    .iter()
                    .filter(|m| **m > run + 1e-9 && **m < run + len - 1e-9)
                    .map(|m| (m - run) / len),
            );
            cuts.sort_by(|x, y| x.total_cmp(y));
            for u in cuts {
                pts.push((a + (b - a) * u, run + u * len));
                normals.push(n);
            }
            run += len;
            pts.push((b, run));
            normals.push(n);
        }
        for (i, w) in pts.windows(2).enumerate() {
            let Some(n) = normals.get(i).copied().flatten() else {
                continue;
            };
            let ((p, s0), (q, s1)) = (w[0], w[1]);
            let (l0, r0) = reach(s0);
            let (l1, r1) = reach(s1);
            // The ends of the piece on the path are corners too: a join's
            // fan starts there, and a point of it in the middle of an edge
            // left a crack after rounding.
            let mut poly = vec![
                [p.x + n[0] * l0, p.y + n[1] * l0],
                [q.x + n[0] * l1, q.y + n[1] * l1],
                [q.x, q.y],
                [q.x - n[0] * r1, q.y - n[1] * r1],
                [p.x - n[0] * r0, p.y - n[1] * r0],
                [p.x, p.y],
            ];
            // A side of no reach repeats the point on the path.
            poly.dedup();
            if poly.first() == poly.last() {
                poly.pop();
            }
            polys.push(poly);
        }
        // Round joins between consecutive pieces (and around the seam of
        // a closed subpath): a fan from one normal to the next on the
        // outer side of the turn. The inner side is covered by the two
        // pieces themselves; a fan there, as wide as the reach at the
        // corner, stood out past a reach that falls away after the
        // corner and left a notch where it met the band (#776).
        let valid: Vec<(usize, [f64; 2])> = normals
            .iter()
            .enumerate()
            .filter_map(|(i, n)| n.map(|n| (i, n)))
            .collect();
        let count = valid.len();
        let pairs = if *closed {
            count
        } else {
            count.saturating_sub(1)
        };
        for k in 0..pairs {
            let (Some(&(i, a)), Some(&(_, b))) = (valid.get(k), valid.get((k + 1) % count.max(1)))
            else {
                continue;
            };
            let Some(&(c, sc)) = pts.get(i + 1) else {
                continue;
            };
            let turn = (a[0] * b[1] - a[1] * b[0]).atan2(a[0] * b[0] + a[1] * b[1]);
            if turn.abs() < 1e-6 {
                continue;
            }
            let (l, r) = reach(sc);
            let a0 = a[1].atan2(a[0]);
            // A left turn has its outer side on the right.
            let (sign, len) = if turn > 0.0 { (-1.0, r) } else { (1.0, l) };
            if len <= 0.0 {
                continue;
            }
            // Fine enough that the fan's chords stay within `FAN_TOL` of
            // its arc, however wide the reach (#776).
            const FAN_TOL: f64 = 0.005;
            let per = 2.0 * (1.0 - (FAN_TOL / len).min(1.0)).acos();
            let steps = ((turn.abs() / per.max(1e-3)).ceil() as usize).clamp(1, 256);
            // The first and last points exactly the corners of the
            // pieces, so no crack opens between them and the fan.
            let mut fan = vec![[c.x, c.y]];
            for j in 0..=steps {
                let (cos, sin) = if j == 0 {
                    (a[0], a[1])
                } else if j == steps {
                    (b[0], b[1])
                } else {
                    let ang = a0 + turn * j as f64 / steps as f64;
                    (ang.cos(), ang.sin())
                };
                fan.push([c.x + sign * cos * len, c.y + sign * sin * len]);
            }
            polys.push(fan);
        }
    }
    for poly in polys.iter_mut() {
        let area: f64 = poly
            .iter()
            .zip(poly.iter().cycle().skip(1))
            .map(|(p, q)| p[0] * q[1] - q[0] * p[1])
            .sum();
        if area < 0.0 {
            poly.reverse();
        }
    }
    polys.retain(|p| {
        p.iter()
            .zip(p.iter().cycle().skip(1))
            .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
            .sum::<f64>()
            .abs()
            > 1e-12
    });
    let empty: Vec<Vec<[f64; 2]>> = Vec::new();
    from_shapes(polys.overlay(&empty, OverlayRule::Subject, IFill::NonZero))
}

/// The filled area that stands for an outline that cannot be drawn as a
/// plain stroke (a calligraphic nib, a variable width, an outline inside
/// or outside the path, or dashes fitted to the corners), for a path in
/// page space and the object's scale (1 when the outline does not scale
/// with the object). `None`: a plain stroke draws it.
pub fn outline_band(path: &BezPath, stroke: &crate::style::Stroke, scale: f64) -> Option<BezPath> {
    // Bands are kept: a page of brush strokes is drawn again on every
    // zoom and scroll, and working one out takes milliseconds (#829).
    let key = band_key(path, stroke, scale);
    if let Some(b) = band_cache().lock().ok().and_then(|c| c.get(&key).cloned()) {
        return b.as_deref().cloned();
    }
    let band = outline_band_uncached(path, stroke, scale).map(|b| thin_polygons(&b, BAND_THIN));
    if let Ok(mut c) = band_cache().lock() {
        let size: usize = band.as_ref().map_or(1, |b| b.elements().len());
        if c.size + size > BAND_CACHE_ELEMENTS {
            c.map.clear();
            c.size = 0;
        }
        c.size += size;
        c.map.insert(key, band.clone().map(std::sync::Arc::new));
    }
    band
}

/// How far an outline band's points may move when it is thinned, mm:
/// far below what shows or prints.
const BAND_THIN: f64 = 0.0005;

/// The straight-piece contours of `path` with the points that lie within
/// `tol` of the line through their neighbours left out; curved pieces
/// are kept as they are.
fn thin_polygons(path: &BezPath, tol: f64) -> BezPath {
    if path
        .elements()
        .iter()
        .any(|e| matches!(e, PathEl::QuadTo(..) | PathEl::CurveTo(..)))
    {
        return path.clone();
    }
    let mut out = BezPath::new();
    let mut ring: Vec<Point> = Vec::new();
    let mut closed = false;
    let flush = |ring: &mut Vec<Point>, closed: bool, out: &mut BezPath| {
        if ring.is_empty() {
            return;
        }
        let pts = if ring.len() > 3 {
            let mut r = ring.clone();
            if closed {
                r.push(ring[0]);
            }
            let mut few = crate::geometry::simplify(&r, tol);
            if closed {
                few.pop();
            }
            if few.len() >= 3 || !closed {
                few
            } else {
                ring.clone()
            }
        } else {
            ring.clone()
        };
        if let Some((first, rest)) = pts.split_first() {
            out.move_to(*first);
            rest.iter().for_each(|p| out.line_to(*p));
            if closed {
                out.close_path();
            }
        }
        ring.clear();
    };
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                flush(&mut ring, closed, &mut out);
                closed = false;
                ring.push(p);
            }
            PathEl::LineTo(p) => ring.push(p),
            PathEl::ClosePath => closed = true,
            _ => {}
        }
    }
    flush(&mut ring, closed, &mut out);
    out
}

/// Path elements the outline bands kept may hold, all told, before they
/// are let go (a few tens of megabytes at most).
const BAND_CACHE_ELEMENTS: usize = 500_000;

/// The outline bands worked out, by [`band_key`].
#[derive(Default)]
struct BandCache {
    map: std::collections::HashMap<u64, Option<std::sync::Arc<BezPath>>>,
    size: usize,
}

impl BandCache {
    fn get(&self, key: &u64) -> Option<&Option<std::sync::Arc<BezPath>>> {
        self.map.get(key)
    }
}

fn band_cache() -> &'static std::sync::Mutex<BandCache> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<BandCache>> = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// What an outline band depends on: the path, the outline and the scale.
fn band_key(path: &BezPath, stroke: &crate::style::Stroke, scale: f64) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for el in path.elements() {
        let pts: &[Point] = match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => std::slice::from_ref(p),
            PathEl::QuadTo(a, b) => &[*a, *b],
            PathEl::CurveTo(a, b, c) => &[*a, *b, *c],
            PathEl::ClosePath => &[],
        };
        std::mem::discriminant(el).hash(&mut h);
        for p in pts {
            p.x.to_bits().hash(&mut h);
            p.y.to_bits().hash(&mut h);
        }
    }
    format!("{stroke:?}").hash(&mut h);
    scale.to_bits().hash(&mut h);
    h.finish()
}

/// [`outline_band`], worked out.
fn outline_band_uncached(
    path: &BezPath,
    stroke: &crate::style::Stroke,
    scale: f64,
) -> Option<BezPath> {
    use crate::style::{OutlinePosition, Stroke};
    if !stroke.widths.is_empty() {
        let nodes: Vec<crate::style::WidthNode> = stroke
            .widths
            .iter()
            .map(|n| crate::style::WidthNode {
                at: n.at,
                left: n.left * scale,
                right: n.right * scale,
            })
            .collect();
        return Some(variable_band(path, stroke.width * scale, &nodes));
    }
    let hair = stroke.width <= Stroke::HAIRLINE + 1e-9;
    if stroke.stretch < 0.999 && !hair {
        return Some(calligraphic_band(
            path,
            stroke.width * scale,
            stroke.stretch,
            stroke.nib_angle,
        ));
    }
    if hair || !stroke.width.is_finite() {
        return None;
    }
    let scaled = Stroke {
        width: stroke.width * scale,
        ..stroke.clone()
    };
    let aligned = stroke.align_dashes && has_dashes(&stroke.dash);
    if stroke.position == OutlinePosition::Center {
        return aligned.then(|| stroke_area(path, &scaled));
    }
    Some(positioned_band(path, &scaled))
}

/// True when a dash pattern has something to draw.
fn has_dashes(dash: &[f64]) -> bool {
    !dash.is_empty() && dash.iter().all(|d| d.is_finite()) && dash.iter().sum::<f64>() > 1e-9
}

/// An outline wholly inside or outside the closed subpaths of `path`
/// (page space, width already scaled): the stroke at twice the width,
/// cut to the fill area (inside) or with the fill area cut away
/// (outside). Open subpaths keep a centred outline. Dash lengths stay
/// those of the outline's own width.
fn positioned_band(path: &BezPath, stroke: &crate::style::Stroke) -> BezPath {
    use crate::style::OutlinePosition;
    let (closed, open) = split_closed(path);
    let mut out = BezPath::new();
    if !closed.elements().is_empty() {
        let doubled = crate::style::Stroke {
            width: stroke.width * 2.0,
            dash: stroke.dash.iter().map(|d| d / 2.0).collect(),
            ..stroke.clone()
        };
        let area = simplify(&stroke_area(&closed, &doubled));
        let fill = simplify_even_odd(&closed);
        let op = match stroke.position {
            OutlinePosition::Inside => Op::Intersect,
            _ => Op::Trim,
        };
        out = overlay(&area, &fill, op);
    }
    if !open.elements().is_empty() {
        let centred = simplify(&stroke_area(&open, stroke));
        out = if out.elements().is_empty() {
            centred
        } else {
            overlay(&out, &centred, Op::Weld)
        };
    }
    out
}

/// Split a path into its closed subpaths (ended with a close command or
/// ending where they start) and its open ones.
pub fn split_closed(path: &BezPath) -> (BezPath, BezPath) {
    let mut closed = BezPath::new();
    let mut open = BezPath::new();
    let mut cur: Vec<PathEl> = Vec::new();
    let mut flush = |cur: &mut Vec<PathEl>| {
        if cur.is_empty() {
            return;
        }
        let start = match cur.first() {
            Some(PathEl::MoveTo(p)) => Some(*p),
            _ => None,
        };
        let end = match cur.last() {
            Some(PathEl::LineTo(p))
            | Some(PathEl::QuadTo(_, p))
            | Some(PathEl::CurveTo(_, _, p)) => Some(*p),
            _ => None,
        };
        let is_closed = cur.iter().any(|e| matches!(e, PathEl::ClosePath))
            || matches!((start, end), (Some(a), Some(b)) if (a - b).hypot() < 1e-6);
        let target = if is_closed { &mut closed } else { &mut open };
        for e in cur.drain(..) {
            target.push(e);
        }
    };
    for el in path.elements() {
        if matches!(el, PathEl::MoveTo(_)) {
            flush(&mut cur);
        }
        cur.push(*el);
    }
    flush(&mut cur);
    (closed, open)
}

/// The area of a path filled with the even-odd rule, as clean contours.
fn simplify_even_odd(path: &BezPath) -> BezPath {
    let subj = contours(path);
    let empty: Vec<Vec<[f64; 2]>> = Vec::new();
    from_shapes(subj.overlay(&empty, OverlayRule::Subject, IFill::EvenOdd))
}

/// The area a plain outline covers along `path` (page space), as a path
/// to fill with the non-zero rule: its width, caps, joins (with its mitre
/// limit) and dashes, fitted to the corners when it aligns them. A
/// hairline covers no area: the result is empty.
pub fn stroke_area(path: &BezPath, stroke: &crate::style::Stroke) -> BezPath {
    use kurbo::{Cap, Join};
    if !stroke.width.is_finite() || stroke.width <= crate::style::Stroke::HAIRLINE + 1e-9 {
        return BezPath::new();
    }
    let cap = match stroke.cap {
        crate::style::LineCap::Butt => Cap::Butt,
        crate::style::LineCap::Round => Cap::Round,
        crate::style::LineCap::Square => Cap::Square,
    };
    let join = match stroke.join {
        crate::style::LineJoin::Miter => Join::Miter,
        crate::style::LineJoin::Round => Join::Round,
        crate::style::LineJoin::Bevel => Join::Bevel,
    };
    let mut style = kurbo::Stroke::new(stroke.width)
        .with_caps(cap)
        .with_join(join)
        .with_miter_limit(stroke.miter_ratio());
    let dashes: Vec<f64> = stroke
        .dash
        .iter()
        .map(|d| (d * stroke.width).max(0.0))
        .collect();
    let tol = (stroke.width / 100.0).max(1e-3);
    if has_dashes(&dashes) {
        if stroke.align_dashes {
            let pieces = aligned_dashes(path, &dashes);
            return kurbo::stroke(pieces.iter(), &style, &kurbo::StrokeOpts::default(), tol);
        }
        style = style.with_dashes(0.0, dashes);
    }
    kurbo::stroke(path.iter(), &style, &kurbo::StrokeOpts::default(), tol)
}

/// Corners sharper than this (degrees between the tangents) split a path
/// into runs for dash alignment; smooth nodes do not.
const DASH_CORNER_DEG: f64 = 3.0;

/// The dashes of `path` with the pattern `dashes` (on, off, on, ...
/// lengths in mm) fitted to each run between corners: a run of length
/// `L` takes `n = max(1, round(L / P))` repeats of the pattern (sum `P`)
/// stretched by `L / (n P)`, starting and ending with half of the first
/// dash. Halves meeting at a corner or at the start of a closed subpath
/// join into one dash, so every corner and every line end lies inside a
/// dash. The result is the dashes as open subpaths, to be stroked
/// without a pattern.
pub fn aligned_dashes(path: &BezPath, dashes: &[f64]) -> BezPath {
    let mut pattern: Vec<f64> = dashes.iter().map(|d| d.max(0.0)).collect();
    if pattern.len() % 2 == 1 {
        pattern = [pattern.clone(), pattern].concat();
    }
    let period: f64 = pattern.iter().sum();
    let mut out = BezPath::new();
    if pattern.is_empty() || !(period.is_finite() && period > 1e-9) {
        return path.clone();
    }
    for (segs, closed) in subpath_segments(path) {
        let lens: Vec<f64> = segs.iter().map(|s| s.arclen(1e-4)).collect();
        let total: f64 = lens.iter().sum();
        if total <= 1e-9 {
            continue;
        }
        // Run boundaries (arc length) at the corners.
        let mut cuts = vec![0.0];
        let mut acc = 0.0;
        for (i, pair) in segs.windows(2).enumerate() {
            acc += lens[i];
            if is_corner(&pair[0], &pair[1]) {
                cuts.push(acc);
            }
        }
        cuts.push(total);
        // A closed subpath smooth where it starts runs on across the start.
        let wraps = closed && !segs.last().is_some_and(|l| is_corner(l, &segs[0]));
        let mut runs: Vec<(f64, f64)> = cuts.windows(2).map(|w| (w[0], w[1])).collect();
        if wraps && runs.len() > 1 {
            // The last run continues into the first.
            let (a, _) = runs.remove(runs.len() - 1);
            let (_, b) = runs[0];
            runs[0] = (a, total + b);
        }
        let mut intervals: Vec<(f64, f64)> = Vec::new();
        for (r0, r1) in runs {
            let len = r1 - r0;
            if len <= 1e-9 {
                continue;
            }
            let n = (len / period).round().max(1.0);
            let k = len / (n * period);
            let q: Vec<f64> = pattern.iter().map(|d| (d * k).max(1e-3)).collect();
            let p: f64 = q.iter().sum();
            let half = q[0] / 2.0;
            for rep in 0..=(n as usize) {
                let mut off = rep as f64 * p - half;
                for (i, d) in q.iter().enumerate() {
                    if i % 2 == 0 {
                        let a = off.max(0.0);
                        let b = (off + d).min(len);
                        if b > a + 1e-9 {
                            intervals.push((r0 + a, r0 + b));
                        }
                    }
                    off += d;
                }
            }
        }
        // Dashes of a run that crossed the start, measured from the start.
        for iv in intervals.iter_mut() {
            if iv.0 >= total - 1e-9 {
                *iv = (iv.0 - total, iv.1 - total);
            }
        }
        // Join dashes that meet (the halves at a corner).
        intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f64, f64)> = Vec::new();
        for (a, b) in intervals {
            match merged.last_mut() {
                Some(last) if a <= last.1 + 1e-6 => last.1 = last.1.max(b),
                _ => merged.push((a, b)),
            }
        }
        // A closed subpath: the dash ending at the start runs on into the
        // first one.
        if closed && merged.len() > 1 {
            let first = merged[0];
            if let Some(last) = merged.last().copied() {
                if first.0 <= 1e-6 && (last.1 - total).abs() <= 1e-6 {
                    merged.pop();
                    merged[0] = (last.0, total + first.1);
                }
            }
        }
        for (a, b) in merged {
            append_piece(&mut out, &segs, &lens, total, closed, a, b);
        }
    }
    out
}

/// The segments of each subpath of `path` (a close adds the closing
/// line), with whether it is closed. Zero-length segments are left out.
fn subpath_segments(path: &BezPath) -> Vec<(Vec<kurbo::PathSeg>, bool)> {
    use kurbo::{CubicBez, Line, PathSeg, QuadBez};
    let mut out = Vec::new();
    let mut segs: Vec<PathSeg> = Vec::new();
    let mut start = Point::ZERO;
    let mut cur = Point::ZERO;
    let push = |segs: &mut Vec<PathSeg>, s: PathSeg| {
        if s.arclen(1e-6) > 1e-9 {
            segs.push(s);
        }
    };
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                if !segs.is_empty() {
                    out.push((std::mem::take(&mut segs), false));
                }
                start = p;
                cur = p;
            }
            PathEl::LineTo(p) => {
                push(&mut segs, PathSeg::Line(Line::new(cur, p)));
                cur = p;
            }
            PathEl::QuadTo(a, p) => {
                push(&mut segs, PathSeg::Quad(QuadBez::new(cur, a, p)));
                cur = p;
            }
            PathEl::CurveTo(a, b, p) => {
                push(&mut segs, PathSeg::Cubic(CubicBez::new(cur, a, b, p)));
                cur = p;
            }
            PathEl::ClosePath => {
                push(&mut segs, PathSeg::Line(Line::new(cur, start)));
                cur = start;
                if !segs.is_empty() {
                    out.push((std::mem::take(&mut segs), true));
                }
            }
        }
    }
    if !segs.is_empty() {
        out.push((segs, false));
    }
    out
}

/// True when the path turns by more than [`DASH_CORNER_DEG`] where `a`
/// ends and `b` starts.
fn is_corner(a: &kurbo::PathSeg, b: &kurbo::PathSeg) -> bool {
    use kurbo::ParamCurve;
    let ta = a.eval(1.0) - a.eval(1.0 - 1e-3);
    let tb = b.eval(1e-3) - b.eval(0.0);
    let (la, lb) = (ta.hypot(), tb.hypot());
    if la <= 1e-12 || lb <= 1e-12 {
        return true;
    }
    let cos = (ta.dot(tb) / (la * lb)).clamp(-1.0, 1.0);
    cos.acos().to_degrees() > DASH_CORNER_DEG
}

/// Append the part of a subpath between arc lengths `a` and `b` as an open
/// subpath; on a closed subpath `b` may run past the end into the start.
fn append_piece(
    out: &mut BezPath,
    segs: &[kurbo::PathSeg],
    lens: &[f64],
    total: f64,
    closed: bool,
    a: f64,
    b: f64,
) {
    let mut ranges = vec![(a, b.min(total))];
    if closed && b > total + 1e-9 {
        ranges.push((0.0, b - total));
    }
    let mut started = false;
    for (ra, rb) in ranges {
        let mut s0 = 0.0;
        for (seg, len) in segs.iter().zip(lens) {
            let s1 = s0 + len;
            let lo = ra.max(s0);
            let hi = rb.min(s1);
            if hi > lo + 1e-9 && *len > 1e-12 {
                let t0 = seg.inv_arclen(lo - s0, 1e-5);
                let t1 = seg.inv_arclen(hi - s0, 1e-5);
                let piece = seg.subsegment(t0..t1);
                if !started {
                    out.move_to(piece.start());
                    started = true;
                }
                match piece {
                    kurbo::PathSeg::Line(l) => out.line_to(l.p1),
                    kurbo::PathSeg::Quad(q) => out.quad_to(q.p1, q.p2),
                    kurbo::PathSeg::Cubic(c) => out.curve_to(c.p1, c.p2, c.p3),
                }
            }
            s0 = s1;
        }
    }
}

/// Union of one path with itself: removes self-overlaps (Simplify).
pub fn simplify(path: &BezPath) -> BezPath {
    let subj = contours(path);
    let empty: Vec<Vec<[f64; 2]>> = Vec::new();
    let result = subj.overlay(&empty, OverlayRule::Subject, IFill::NonZero);
    recover_curves(&from_shapes(result), &[path])
}

/// The area a path covers under the nonzero rule (how text is filled), as
/// a path that covers the same under the even-odd rule (how curves are
/// filled). Only contours whose boxes overlap one turning the same way
/// (strokes of a glyph drawn as separate pieces, as many Chinese fonts do)
/// are joined; the rest are kept as they are, so a long text stays quick
/// and keeps its curves (#800).
pub fn nonzero_as_even_odd(path: &BezPath) -> BezPath {
    use kurbo::Shape as _;
    let mut subs: Vec<BezPath> = Vec::new();
    for el in path.elements() {
        if matches!(el, PathEl::MoveTo(_)) || subs.is_empty() {
            subs.push(BezPath::new());
        }
        if let Some(last) = subs.last_mut() {
            last.push(*el);
        }
    }
    let boxes: Vec<kurbo::Rect> = subs.iter().map(|s| s.bounding_box()).collect();
    let turns: Vec<bool> = subs.iter().map(|s| s.area() > 0.0).collect();
    // Union-find over contours whose boxes overlap.
    let mut parent: Vec<usize> = (0..subs.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut order: Vec<usize> = (0..subs.len()).collect();
    order.sort_by(|a, b| boxes[*a].x0.total_cmp(&boxes[*b].x0));
    let mut same_turn = vec![false; subs.len()];
    let mut active: Vec<usize> = Vec::new();
    for &i in &order {
        active.retain(|&j| boxes[j].x1 > boxes[i].x0);
        for &j in &active {
            let (a, b) = (boxes[i], boxes[j]);
            if a.y0 < b.y1 && b.y0 < a.y1 {
                let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                let joined = same_turn[ri] || same_turn[rj] || turns[i] == turns[j];
                parent[ri] = rj;
                same_turn[rj] = joined;
            }
        }
        active.push(i);
    }
    let mut clusters: Vec<Vec<usize>> = vec![Vec::new(); subs.len()];
    for i in 0..subs.len() {
        let r = root(&mut parent, i);
        clusters[r].push(i);
    }
    let mut out = BezPath::new();
    for (r, members) in clusters.iter().enumerate() {
        if members.is_empty() {
            continue;
        }
        let mut piece = BezPath::new();
        for &i in members {
            for el in subs[i].elements() {
                piece.push(*el);
            }
        }
        let piece = if same_turn[r] {
            simplify(&piece)
        } else {
            piece
        };
        for el in piece.elements() {
            out.push(*el);
        }
    }
    out
}

#[cfg(test)]
mod tests {

    /// A contour of a circle is a circle made of a few curves, not a
    /// row of short straight pieces (#821), and stays at its distance.
    #[test]
    fn an_offset_circle_is_a_few_curves() {
        use super::{offset_with, Join};
        use crate::geometry::{BezPath, PathEl, Point, Shape as _};
        let circle = kurbo::Circle::new(Point::new(50.0, 50.0), 20.0).to_path(0.001);
        for d in [5.0, -5.0] {
            let q: BezPath = offset_with(&circle, d, Join::Round);
            let pieces = q
                .elements()
                .iter()
                .filter(|e| !matches!(e, PathEl::MoveTo(_) | PathEl::ClosePath))
                .count();
            assert!(pieces <= 8, "{d}: {pieces} pieces");
            assert!(q
                .elements()
                .iter()
                .any(|e| matches!(e, PathEl::CurveTo(..))));
            for t in 0..100 {
                let a = t as f64 / 100.0 * std::f64::consts::TAU;
                let r = 20.0 + d;
                let on = Point::new(50.0 + r * a.cos(), 50.0 + r * a.sin());
                let near = q
                    .segments()
                    .map(|s| {
                        kurbo::ParamCurveNearest::nearest(&s, on, 1e-4)
                            .distance_sq
                            .sqrt()
                    })
                    .fold(f64::INFINITY, f64::min);
                assert!(near < 0.1, "{d}: {near} off at {a}");
            }
        }
    }

    /// Sharp corners stay sharp: a mitred square's contour is four
    /// straight sides.
    #[test]
    fn an_offset_square_keeps_its_corners() {
        use super::{offset_with, Join};
        use crate::geometry::{BezPath, PathEl, Point};
        let mut sq = BezPath::new();
        sq.move_to(Point::new(0.0, 0.0));
        sq.line_to(Point::new(10.0, 0.0));
        sq.line_to(Point::new(10.0, 10.0));
        sq.line_to(Point::new(0.0, 10.0));
        sq.close_path();
        let q = offset_with(&sq, 2.0, Join::Miter);
        let lines = q
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::LineTo(_)))
            .count();
        let curves = q
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::CurveTo(..) | PathEl::QuadTo(..)))
            .count();
        assert_eq!((lines, curves), (4, 0), "{:?}", q.elements());
        // Round corners: the four sides stay straight lines, the corners
        // are curves.
        let q = offset_with(&sq, 2.0, Join::Round);
        let lines = q
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::LineTo(_)))
            .count();
        let curves = q
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::CurveTo(..) | PathEl::QuadTo(..)))
            .count();
        assert!(
            lines >= 4 && (4..=12).contains(&curves),
            "{:?}",
            q.elements()
        );
    }

    fn glyph_square(path: &mut crate::geometry::BezPath, x: f64, y: f64, w: f64, clockwise: bool) {
        use crate::geometry::Point;
        let pts = [(x, y), (x + w, y), (x + w, y + w), (x, y + w)];
        path.move_to(Point::new(pts[0].0, pts[0].1));
        if clockwise {
            for p in &pts[1..] {
                path.line_to(Point::new(p.0, p.1));
            }
        } else {
            for p in pts[1..].iter().rev() {
                path.line_to(Point::new(p.0, p.1));
            }
        }
        path.close_path();
    }

    /// Even-odd: inside where the winding number is odd.
    fn even_odd_inside(path: &crate::geometry::BezPath, x: f64, y: f64) -> bool {
        use crate::geometry::Shape as _;
        path.winding(crate::geometry::Point::new(x, y)) % 2 != 0
    }

    #[test]
    /// Two strokes of a glyph that cross, turning the same way, stay
    /// filled where they cross once the text is curves (#800).
    fn overlapping_strokes_stay_filled_as_curves() {
        use super::nonzero_as_even_odd;
        let mut p = crate::geometry::BezPath::new();
        glyph_square(&mut p, 0.0, 0.0, 10.0, true);
        glyph_square(&mut p, 5.0, 5.0, 10.0, true);
        assert!(!even_odd_inside(&p, 7.0, 7.0), "the hole the report shows");
        let q = nonzero_as_even_odd(&p);
        assert!(even_odd_inside(&q, 7.0, 7.0));
        assert!(even_odd_inside(&q, 2.0, 2.0));
        assert!(even_odd_inside(&q, 13.0, 13.0));
        assert!(!even_odd_inside(&q, 13.0, 2.0));
    }

    #[test]
    /// A counter (the hole of an o) and glyphs apart from each other are
    /// kept as they are.
    fn counters_and_separate_glyphs_are_kept() {
        use super::nonzero_as_even_odd;
        let mut p = crate::geometry::BezPath::new();
        glyph_square(&mut p, 0.0, 0.0, 10.0, true);
        glyph_square(&mut p, 3.0, 3.0, 4.0, false);
        glyph_square(&mut p, 20.0, 0.0, 10.0, true);
        let q = nonzero_as_even_odd(&p);
        assert_eq!(q.elements(), p.elements());
        assert!(!even_odd_inside(&q, 5.0, 5.0));
    }

    /// Area Fill finds the exact area (#734): inside a circle, the circle
    /// itself, its four curves and no more nodes; where two squares
    /// overlap, their overlap; outside them, an open area.
    #[test]
    fn enclosed_area_keeps_the_outlines_curves() {
        use super::{enclosed_area, Enclosed};
        use crate::geometry::{BezPath, Point, Rect};
        let circle = crate::geometry::ellipse_path(Rect::new(0.0, 0.0, 40.0, 40.0));
        let Enclosed::Area(area) =
            enclosed_area(std::slice::from_ref(&circle), Point::new(20.0, 20.0))
        else {
            panic!("no area");
        };
        let curves = |p: &BezPath| p.segments().count();
        assert_eq!(curves(&area), curves(&circle), "{area:?}");
        use kurbo::Shape as _;
        assert!((area.area().abs() - circle.area().abs()).abs() < 1e-6);
        // Every point of the area's outline on the circle.
        let mut far: f64 = 0.0;
        kurbo::flatten(area.elements().iter().copied(), 0.01, |el| {
            if let super::PathEl::LineTo(q) | super::PathEl::MoveTo(q) = el {
                far = far.max(((q - Point::new(20.0, 20.0)).hypot() - 20.0).abs());
            }
        });
        assert!(far < 0.02, "{far}");
        // Two squares overlapping: the overlap, exactly.
        let a = crate::geometry::rect_path(Rect::new(0.0, 0.0, 30.0, 30.0), 0.0);
        let b = crate::geometry::rect_path(Rect::new(17.3, 11.1, 50.0, 50.0), 0.0);
        let Enclosed::Area(area) = enclosed_area(&[a.clone(), b.clone()], Point::new(20.0, 20.0))
        else {
            panic!("no overlap");
        };
        let r = area.bounding_box();
        for (got, want) in [(r.x0, 17.3), (r.y0, 11.1), (r.x1, 30.0), (r.y1, 30.0)] {
            assert!((got - want).abs() < 1e-6, "{r:?}");
        }
        assert_eq!(curves(&area), 4, "{area:?}");
        // Outside both, or nowhere near: open.
        assert_eq!(
            enclosed_area(&[a.clone(), b.clone()], Point::new(45.0, 5.0)),
            Enclosed::Open
        );
        assert_eq!(
            enclosed_area(&[a, b], Point::new(500.0, 5.0)),
            Enclosed::Open
        );
        assert_eq!(enclosed_area(&[], Point::new(5.0, 5.0)), Enclosed::Open);
    }

    /// Lines that stop a hair short of the next outline still close the
    /// area (#899): a frame cut in two by a line ending 0.0002 mm from
    /// the frame, as files carry from rounding, gives the half under the
    /// click; a gap anyone could see (0.2 mm) stays open, and so does a
    /// line that merely touches nothing.
    #[test]
    fn enclosed_area_closes_hairline_gaps() {
        use super::{enclosed_area, Enclosed};
        use crate::geometry::{BezPath, Point, Rect};
        use kurbo::Shape as _;
        let frame = crate::geometry::rect_path(Rect::new(0.0, 0.0, 40.0, 20.0), 0.0);
        let divider = |top: f64, bottom: f64| {
            let mut p = BezPath::new();
            p.move_to((20.0, top));
            p.line_to((20.0, bottom));
            p
        };
        let width_at = |gap: f64, at: Point| match enclosed_area(
            &[frame.clone(), divider(0.0002 + gap, 19.9998 - gap)],
            at,
        ) {
            Enclosed::Area(a) => a.bounding_box().width(),
            other => panic!("{other:?}"),
        };
        // Both ends a hair short of the frame: two halves.
        assert!((width_at(0.0, Point::new(10.0, 10.0)) - 20.0).abs() < 1e-3);
        assert!((width_at(0.0, Point::new(30.0, 10.0)) - 20.0).abs() < 1e-3);
        // Ends 0.2 mm short: the halves are one area around the line.
        assert!((width_at(0.2, Point::new(10.0, 10.0)) - 40.0).abs() < 1e-3);
        // A line that crosses nothing never divides the frame.
        let mut stray = BezPath::new();
        stray.move_to((10.0, 5.0));
        stray.line_to((10.0, 15.0));
        let Enclosed::Area(a) = enclosed_area(&[frame.clone(), stray], Point::new(30.0, 10.0))
        else {
            panic!("no area");
        };
        assert!((a.bounding_box().width() - 40.0).abs() < 1e-3);
        // An outline's own ends: a ring left open by a hairline closes
        // into an area of its own.
        let mut ring = BezPath::new();
        ring.move_to((5.0, 5.0));
        ring.line_to((15.0, 5.0));
        ring.line_to((15.0, 15.0));
        ring.line_to((5.0, 15.0));
        ring.line_to((5.0, 5.0002));
        let Enclosed::Area(a) = enclosed_area(&[ring], Point::new(10.0, 10.0)) else {
            panic!("ring not closed");
        };
        assert!((a.bounding_box().width() - 10.0).abs() < 1e-3);
    }

    #[test]
    /// The width eases between nodes along a smooth curve however short
    /// the span on a long path: half-way into the ease the reach is the
    /// smoothstep's, not that of a few straight pieces (#492).
    fn variable_band_eases_smoothly_between_close_nodes() {
        use super::variable_band;
        use crate::geometry::{BezPath, Point, Shape as _};
        use crate::style::WidthNode;
        let mut line = BezPath::new();
        line.move_to(Point::new(0.0, 0.0));
        line.line_to(Point::new(1000.0, 0.0));
        // 2 mm wide at 500, 10 mm at 510.
        let band = variable_band(
            &line,
            2.0,
            &[
                WidthNode::symmetric(0.5, 2.0),
                WidthNode::symmetric(0.51, 10.0),
            ],
        );
        // A quarter into the ease: 1 + 4 * smoothstep(0.25) = 1.625.
        assert!(band.contains(Point::new(502.5, 1.58)));
        assert!(!band.contains(Point::new(502.5, 1.67)));
        // Three quarters: 1 + 4 * 0.84375 = 4.375.
        assert!(band.contains(Point::new(507.5, 4.33)));
        assert!(!band.contains(Point::new(507.5, 4.42)));
    }

    /// Along a curve the pieces of the band meet without cracks: one
    /// outline, no slivers or spikes into it (#627).
    #[test]
    fn a_variable_band_along_a_curve_has_no_cracks() {
        use super::variable_band;
        use crate::geometry::{BezPath, PathEl, Point};
        use crate::style::WidthNode;
        let mut arch = BezPath::new();
        arch.move_to(Point::new(10.0, 10.0));
        arch.curve_to(
            Point::new(30.0, 60.0),
            Point::new(80.0, 60.0),
            Point::new(110.0, 10.0),
        );
        let band = variable_band(&arch, 2.0, &[WidthNode::symmetric(0.5, 10.0)]);
        let moves = band
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count();
        assert_eq!(moves, 1);
        // No vertex of the outline comes near the path itself, but for
        // the middle of each butt end.
        let ends = [Point::new(10.0, 10.0), Point::new(110.0, 10.0)];
        let mut near = 0;
        crate::geometry::flatten(&band, 0.05, &mut |el| {
            if let PathEl::LineTo(p) | PathEl::MoveTo(p) = el {
                let (_, d, _) = super::locate_on_path(&arch, p).unwrap_or((0.0, 9.0, true));
                if d < 0.5 && ends.iter().all(|e| (*e - p).hypot() > 1e-6) {
                    near += 1;
                }
            }
        });
        assert_eq!(near, 0);
    }

    #[test]
    fn variable_band_follows_the_width_nodes() {
        use super::{locate_on_path, outline_band, variable_band};
        use crate::geometry::{BezPath, Point, Shape as _};
        use crate::style::{Stroke, WidthNode};
        let mut line = BezPath::new();
        line.move_to(Point::new(0.0, 0.0));
        line.line_to(Point::new(100.0, 0.0));
        // No nodes: the base width everywhere, butt ends.
        let b = variable_band(&line, 2.0, &[]).bounding_box();
        assert!((b.height() - 2.0).abs() < 1e-6 && (b.width() - 100.0).abs() < 1e-6);
        // A 10 mm node in the middle: 10 mm there, 2 mm at the ends.
        let band = variable_band(&line, 2.0, &[WidthNode::symmetric(0.5, 10.0)]);
        let b = band.bounding_box();
        assert!((b.height() - 10.0).abs() < 1e-3, "{b:?}");
        assert!(band.contains(Point::new(50.0, 4.9)));
        assert!(!band.contains(Point::new(2.0, 1.5)));
        assert!(band.contains(Point::new(2.0, 0.9)));
        // Only to the left (up, for a line going right).
        let node = WidthNode {
            at: 0.5,
            left: 6.0,
            right: 0.0,
        };
        let b = variable_band(&line, 0.0, &[node]).bounding_box();
        assert!(b.y0 > -1e-6 && (b.y1 - 6.0).abs() < 1e-3, "{b:?}");
        // Through the stroke, scaled with the object.
        let mut s = Stroke::new(crate::color::Color::BLACK, 2.0);
        assert!(outline_band(&line, &s, 1.0).is_none());
        s.widths.push(WidthNode::symmetric(0.5, 10.0));
        let b = outline_band(&line, &s, 2.0)
            .map(|p| p.bounding_box())
            .unwrap_or_default();
        assert!((b.height() - 20.0).abs() < 1e-3, "{b:?}");
        // Locating: a quarter along, 3 mm to the right.
        let (t, d, left) = locate_on_path(&line, Point::new(25.0, -3.0)).unwrap_or_default();
        assert!((t - 0.25).abs() < 1e-9 && (d - 3.0).abs() < 1e-9 && !left);
    }

    #[test]
    fn closed_variable_bands_have_no_gaps_at_the_corners() {
        use super::variable_band;
        use crate::geometry::{Point, Rect, Shape as _};
        let sq = Rect::new(0.0, 0.0, 50.0, 50.0).to_path(0.01);
        let band = variable_band(&sq, 4.0, &[]);
        // The outer corner is covered on both sides of the seam.
        assert!(band.contains(Point::new(-1.2, -1.2)));
        assert!(band.contains(Point::new(51.2, 51.2)));
        assert!(!band.contains(Point::new(-1.6, -1.6)));
        assert!(!band.contains(Point::new(25.0, 25.0)));
    }

    /// #776: a wide node at a rectangle's corner, its reach falling away
    /// down the next side. The inside of the corner follows that fall
    /// with no round bulge as wide as the corner's reach standing out
    /// and a notch where it met the band; the outside of the corner
    /// stays round, and its arc is drawn within a hundredth of a
    /// millimetre.
    #[test]
    fn a_wide_corner_node_leaves_no_notch_inside_and_a_smooth_arc_outside() {
        use super::variable_band;
        use crate::geometry::{Point, Rect, Shape as _};
        use crate::style::WidthNode;
        // From the bottom-left corner: bottom, right side, top (leftwards)
        // and the left side down; the top-left corner 317 mm along.
        let path = Rect::new(0.0, 0.0, 113.0, 91.0).to_path(0.01);
        let total = 2.0 * (113.0 + 91.0);
        let c = (113.0 + 91.0 + 113.0) / total;
        let nodes = [
            WidthNode::symmetric(c - 9.0 / total, 0.2),
            WidthNode::symmetric(c - 0.3 / total, 118.0),
        ];
        let band = variable_band(&path, 0.2, &nodes);
        // 20 mm down the left side the reach has fallen to about 51.6 mm
        // inside; the old bulge reached 55.5 mm there.
        assert!(band.contains(Point::new(49.0, 71.0)));
        assert!(!band.contains(Point::new(53.5, 71.0)), "a bulge inside");
        // Outside, round about the corner with the corner's reach.
        let r = 59.0 - 0.01;
        for deg in [100.0f64, 135.0, 170.0] {
            let (sin, cos) = deg.to_radians().sin_cos();
            assert!(band.contains(Point::new(r * 0.98 * cos, 91.0 + r * 0.98 * sin)));
            assert!(!band.contains(Point::new(r * 1.01 * cos, 91.0 + r * 1.01 * sin)));
        }
        // The arc's chords stay close to it: no corner of the outline
        // between its points lies far inside the circle.
        let corner = Point::new(0.0, 91.0);
        let mut worst: f64 = 0.0;
        let mut last: Option<Point> = None;
        for el in band.elements() {
            let p = match el {
                crate::geometry::PathEl::MoveTo(p) | crate::geometry::PathEl::LineTo(p) => *p,
                crate::geometry::PathEl::CurveTo(_, _, p)
                | crate::geometry::PathEl::QuadTo(_, p) => *p,
                crate::geometry::PathEl::ClosePath => continue,
            };
            if let Some(q) = last {
                let on_arc = |p: Point| {
                    let d = p - corner;
                    (d.hypot() - 59.0).abs() < 0.05 && d.x < -5.0 && d.y > 5.0
                };
                if on_arc(p) && on_arc(q) {
                    let mid = Point::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
                    worst = worst.max(59.0 - (mid - corner).hypot());
                }
            }
            last = Some(p);
        }
        assert!(worst < 0.01, "chords {worst} mm inside the arc");
    }

    /// A reach wider than a curve's radius fills the curve's inside
    /// whole, with the joins on the outside only (#776).
    #[test]
    fn a_reach_wider_than_the_curve_fills_its_inside() {
        use super::variable_band;
        use crate::geometry::{Circle, Point, Shape as _};
        use crate::style::WidthNode;
        let circle = Circle::new((0.0, 0.0), 2.0).to_path(0.01);
        let band = variable_band(&circle, 10.0, &[WidthNode::symmetric(0.5, 10.0)]);
        for (x, y) in [
            (0.0, 0.0),
            (0.5, -0.3),
            (-1.0, 1.0),
            (6.5, 0.0),
            (0.0, -6.5),
        ] {
            assert!(band.contains(Point::new(x, y)), "({x}, {y})");
        }
        assert!(!band.contains(Point::new(7.3, 0.0)));
        let area = band.area().abs();
        let want = std::f64::consts::PI * 49.0;
        assert!((area - want).abs() < want * 0.02, "{area} against {want}");
    }

    #[test]
    fn calligraphic_band_is_thin_along_the_nib_and_full_across_it() {
        use super::calligraphic_band;
        use crate::geometry::{BezPath, Point, Shape as _};
        // Flat nib at 0 degrees, 2 mm wide, stretch 0.1.
        let mut horizontal = BezPath::new();
        horizontal.move_to(Point::new(0.0, 0.0));
        horizontal.line_to(Point::new(50.0, 0.0));
        let hb = calligraphic_band(&horizontal, 2.0, 0.1, 0.0).bounding_box();
        // Across a horizontal stroke the nib offers only its 0.2 mm side.
        assert!(hb.height() < 0.25 && hb.height() > 0.15, "{hb:?}");
        let mut vertical = BezPath::new();
        vertical.move_to(Point::new(0.0, 0.0));
        vertical.line_to(Point::new(0.0, 50.0));
        let vb = calligraphic_band(&vertical, 2.0, 0.1, 0.0).bounding_box();
        assert!((vb.width() - 2.0).abs() < 0.05, "{vb:?}");
        // The round pen (stretch 1) is a plain 2 mm stroke with round ends.
        let rb = calligraphic_band(&horizontal, 2.0, 1.0, 0.0).bounding_box();
        assert!(
            (rb.height() - 2.0).abs() < 0.05 && (rb.width() - 52.0).abs() < 0.05,
            "{rb:?}"
        );
    }

    use super::*;
    use crate::geometry::{Rect, Shape as _};

    fn square(x: f64) -> BezPath {
        Rect::new(x, 0.0, x + 10.0, 10.0).to_path(0.01)
    }

    fn square_side(side: f64) -> BezPath {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((side, 0.0));
        p.line_to((side, side));
        p.line_to((0.0, side));
        p.close_path();
        p
    }

    #[test]
    fn inside_and_outside_outlines_cover_one_side_of_the_path() {
        use crate::style::{OutlinePosition, Stroke};
        let sq = square_side(20.0);
        let mut st = Stroke::new(crate::Color::BLACK, 2.0);
        // Centred: a plain stroke, no band.
        assert!(outline_band(&sq, &st, 1.0).is_none());
        st.position = OutlinePosition::Inside;
        let inside = outline_band(&sq, &st, 1.0).expect("band");
        let b = inside.bounding_box();
        assert!(
            b.x0.abs() < 0.01 && (b.x1 - 20.0).abs() < 0.01,
            "inside stays in the path: {b:?}"
        );
        // A 2 mm frame inside a 20 mm square: 400 - 256.
        assert!(
            (inside.area().abs() - 144.0).abs() < 0.5,
            "{}",
            inside.area()
        );
        st.position = OutlinePosition::Outside;
        let outside = outline_band(&sq, &st, 1.0).expect("band");
        let b = outside.bounding_box();
        assert!(
            (b.x0 + 2.0).abs() < 0.01 && (b.x1 - 22.0).abs() < 0.01,
            "outside reaches the full width out: {b:?}"
        );
        // Mitred corners: 576 - 400.
        assert!(
            (outside.area().abs() - 176.0).abs() < 0.5,
            "{}",
            outside.area()
        );
        // The scale applies to the width.
        let scaled = outline_band(&sq, &st, 2.0).expect("band");
        assert!((scaled.bounding_box().x0 + 4.0).abs() < 0.01);
        // An open line keeps a centred outline.
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((30.0, 0.0));
        let open = outline_band(&line, &st, 1.0).expect("band");
        let b = open.bounding_box();
        assert!(
            (b.y0 + 1.0).abs() < 0.01 && (b.y1 - 1.0).abs() < 0.01,
            "{b:?}"
        );
        // Hairlines stay plain.
        let mut hair = Stroke::hairline(crate::Color::BLACK);
        hair.position = OutlinePosition::Inside;
        assert!(outline_band(&sq, &hair, 1.0).is_none());
    }

    #[test]
    fn the_mitre_limit_bevels_sharp_corners() {
        use crate::style::Stroke;
        // A narrow spike: about 11 degrees at the tip.
        let mut spike = BezPath::new();
        spike.move_to((0.0, 0.0));
        spike.line_to((50.0, 5.0));
        spike.line_to((0.0, 10.0));
        let mut st = Stroke::new(crate::Color::BLACK, 1.0);
        st.miter_limit = 5.0;
        let sharp = stroke_area(&spike, &st).bounding_box().x1;
        st.miter_limit = 30.0;
        let cut = stroke_area(&spike, &st).bounding_box().x1;
        assert!(sharp > 54.0, "{sharp}");
        assert!(cut < 51.0, "{cut}");
    }

    /// Whether `p` is on the drawn part of `dashes` (within `tol`).
    fn covered(dashes: &BezPath, p: Point, tol: f64) -> bool {
        use kurbo::ParamCurveNearest as _;
        dashes
            .segments()
            .any(|s| s.nearest(p, 1e-6).distance_sq <= tol * tol)
    }

    #[test]
    fn aligned_dashes_put_corners_and_ends_on_dashes() {
        use kurbo::ParamCurveArclen as _;
        let sq = square_side(20.0);
        // 3 on, 2 off: 20 / 5 = 4 repeats per side exactly.
        let d = aligned_dashes(&sq, &[3.0, 2.0]);
        for corner in [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)] {
            assert!(
                covered(&d, Point::new(corner.0, corner.1), 1e-6),
                "{corner:?}"
            );
        }
        // Every side has 4 dashes, the corner ones shared: 16 pieces.
        let pieces = d
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count();
        assert_eq!(pieces, 16);
        // Gaps fall between them: 2.5 mm from a corner is off.
        assert!(!covered(&d, Point::new(2.5, 0.0), 0.01));
        // A side that does not hold whole repeats is stretched: 23 mm with
        // a 5 mm pattern takes 5 repeats of 4.6 mm, still on the corners.
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((23.0, 0.0));
        let d = aligned_dashes(&line, &[3.0, 2.0]);
        assert!(covered(&d, Point::new(0.0, 0.0), 1e-6));
        assert!(covered(&d, Point::new(23.0, 0.0), 1e-6));
        let total: f64 = d.segments().map(|s| s.arclen(1e-6)).sum();
        // 5 dashes of 2.76 mm (two halves at the ends): 3/5 of the length.
        assert!((total - 23.0 * 0.6).abs() < 1e-3, "{total}");
        // A smooth closed curve has no corners: one run around it.
        let circle = kurbo::Circle::new((0.0, 0.0), 10.0).to_path(0.01);
        let d = aligned_dashes(&circle, &[3.0, 2.0]);
        assert!(!d.elements().is_empty());
        // In the outline: aligned dashes go through the band.
        let mut st = crate::style::Stroke::new(crate::Color::BLACK, 1.0);
        st.dash = vec![3.0, 2.0];
        assert!(outline_band(&sq, &st, 1.0).is_none());
        st.align_dashes = true;
        let band = outline_band(&sq, &st, 1.0).expect("band");
        assert!(band.area().abs() > 1.0);
        // Degenerate patterns leave the path as it is.
        assert_eq!(aligned_dashes(&sq, &[0.0, 0.0]).elements(), sq.elements());
        assert_eq!(aligned_dashes(&sq, &[]).elements(), sq.elements());
    }

    #[test]
    fn stroke_area_covers_the_outline_and_hairlines_cover_nothing() {
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((50.0, 0.0));
        let s = crate::style::Stroke::new(crate::Color::BLACK, 2.0);
        let a = stroke_area(&line, &s);
        let b = a.bounding_box();
        assert!(
            (b.width() - 50.0).abs() < 0.01 && (b.height() - 2.0).abs() < 0.01,
            "{b:?}"
        );
        assert!((a.area().abs() - 100.0).abs() < 0.5, "{}", a.area());
        // Round caps reach half the width past the ends.
        let mut round = s.clone();
        round.cap = crate::style::LineCap::Round;
        assert!((stroke_area(&line, &round).bounding_box().width() - 52.0).abs() < 0.05);
        // Dashes leave gaps: half on, half off.
        let mut dashed = s.clone();
        dashed.dash = vec![2.5, 2.5];
        let d = stroke_area(&line, &dashed).area().abs();
        assert!((d - 50.0).abs() < 2.0, "{d}");
        let hair = crate::style::Stroke::hairline(crate::Color::BLACK);
        assert!(stroke_area(&line, &hair).elements().is_empty());
    }

    #[test]
    fn weld_two_overlapping_squares() {
        let w = overlay(&square(0.0), &square(5.0), Op::Weld);
        let b = w.bounding_box();
        assert!((b.x0 - 0.0).abs() < 1e-6 && (b.x1 - 15.0).abs() < 1e-6);
        assert!((w.area().abs() - 150.0).abs() < 1e-3);
    }

    #[test]
    fn offset_grows_and_shrinks() {
        let grown = offset(&square(0.0), 2.0);
        let b = grown.bounding_box();
        assert!(
            (b.x0 + 2.0).abs() < 0.05 && (b.x1 - 12.0).abs() < 0.05,
            "{b:?}"
        );
        let shrunk = offset(&square(0.0), -2.0);
        let b = shrunk.bounding_box();
        assert!(
            (b.x0 - 2.0).abs() < 0.05 && (b.x1 - 8.0).abs() < 0.05,
            "{b:?}"
        );
    }

    #[test]
    fn offset_corners_are_mitred_round_or_bevelled() {
        let area = |j: Join| offset_with(&square(0.0), 2.0, j).area().abs();
        assert!(
            (area(Join::Miter) - 196.0).abs() < 0.2,
            "{}",
            area(Join::Miter)
        );
        let round = 180.0 + std::f64::consts::PI * 4.0;
        assert!(
            (area(Join::Round) - round).abs() < 0.3,
            "{}",
            area(Join::Round)
        );
        assert!(
            (area(Join::Bevel) - 188.0).abs() < 0.2,
            "{}",
            area(Join::Bevel)
        );
        assert!(offset_with(&square(0.0), f64::NAN, Join::Miter)
            .elements()
            .is_empty());
    }

    fn circle(cx: f64, cy: f64, r: f64) -> BezPath {
        use crate::geometry::Shape as _;
        kurbo::Circle::new((cx, cy), r).to_path(1e-4)
    }

    /// The end points of a path's elements.
    fn nodes(p: &BezPath) -> Vec<Point> {
        p.elements()
            .iter()
            .filter_map(|e| match *e {
                PathEl::MoveTo(q) | PathEl::LineTo(q) => Some(q),
                PathEl::QuadTo(_, q) => Some(q),
                PathEl::CurveTo(_, _, q) => Some(q),
                PathEl::ClosePath => None,
            })
            .collect()
    }

    fn count(p: &BezPath, curves: bool) -> usize {
        p.elements()
            .iter()
            .filter(|e| match e {
                PathEl::LineTo(_) => !curves,
                PathEl::QuadTo(..) | PathEl::CurveTo(..) => curves,
                _ => false,
            })
            .count()
    }

    /// Welding two circles keeps their arcs: the result is a few curve
    /// segments with nodes only at the circles' own nodes and where they
    /// cross, not hundreds of tiny lines.
    #[test]
    fn welding_circles_keeps_their_arcs() {
        let (a, b) = (circle(0.0, 0.0, 10.0), circle(12.0, 0.0, 10.0));
        let w = overlay(&a, &b, Op::Weld);
        assert_eq!(count(&w, false), 0, "{w:?}");
        let curves = count(&w, true);
        assert!((4..=12).contains(&curves), "{curves} curve segments");
        // The area of the union of two discs of radius 10, 12 apart.
        let lens = 200.0 * (0.6f64).acos() - 6.0 * (400.0f64 - 144.0).sqrt();
        let expect = 2.0 * std::f64::consts::PI * 100.0 - lens;
        assert!((w.area().abs() - expect).abs() < 0.3, "{}", w.area());
        for q in nodes(&w) {
            let on_a = ((q - Point::new(0.0, 0.0)).hypot() - 10.0).abs();
            let on_b = ((q - Point::new(12.0, 0.0)).hypot() - 10.0).abs();
            assert!(on_a < 2e-3 || on_b < 2e-3, "{q:?} off both circles");
        }
        // Trim and Intersect the same way.
        let t = overlay(&a, &b, Op::Trim);
        assert_eq!(count(&t, false), 0, "{t:?}");
        assert!((t.area().abs() - (std::f64::consts::PI * 100.0 - lens)).abs() < 0.3);
        let i = overlay(&a, &b, Op::Intersect);
        assert_eq!(count(&i, false), 0, "{i:?}");
        assert!((i.area().abs() - lens).abs() < 0.3);
        // Nodes at the two crossings and at the circles' own nodes on
        // the lens, nowhere else.
        assert!(count(&i, true) <= 7, "{} {i:?}", count(&i, true));
        // The last piece ends at the start: no node is repeated.
        assert_eq!(nodes(&i).len(), count(&i, true) + 1, "{i:?}");
    }

    /// A square and a circle: the result has the square's straight sides
    /// and the circle's arcs, and nothing else.
    #[test]
    fn lines_and_arcs_both_come_back() {
        let sq = square(0.0);
        let c = circle(10.0, 5.0, 4.0);
        let i = overlay(&sq, &c, Op::Intersect);
        // The left half of the disc: one straight edge, the arc in a few pieces.
        assert_eq!(count(&i, false), 1, "{i:?}");
        assert!(count(&i, true) <= 4, "{i:?}");
        assert!((i.area().abs() - std::f64::consts::PI * 8.0).abs() < 0.1);
        // Polygons stay polygons: two squares side by side weld to a
        // rectangle of four corners, none repeated.
        let w = overlay(&square(0.0), &square(5.0), Op::Weld);
        assert_eq!(count(&w, true), 0);
        assert_eq!(count(&w, false), 3, "{w:?}");
        assert_eq!(nodes(&w).len(), 4, "{w:?}");
    }

    /// Where the result cuts straight across a circle (a trim by a
    /// half-plane), the cut is a line, not the arc it replaces.
    #[test]
    fn a_chord_stays_a_line() {
        let c = circle(0.0, 0.0, 10.0);
        let mut half = BezPath::new();
        half.move_to((0.0, -20.0));
        half.line_to((20.0, -20.0));
        half.line_to((20.0, 20.0));
        half.line_to((0.0, 20.0));
        half.close_path();
        let t = overlay(&c, &half, Op::Trim);
        assert!(
            (t.area().abs() - std::f64::consts::PI * 50.0).abs() < 0.2,
            "{}",
            t.area()
        );
        assert_eq!(count(&t, false), 1, "{t:?}");
        let chord = t
            .elements()
            .iter()
            .find_map(|e| match e {
                PathEl::LineTo(q) => Some(*q),
                _ => None,
            })
            .unwrap();
        assert!(
            chord.x.abs() < 1e-3 && (chord.y.abs() - 10.0).abs() < 1e-3,
            "{chord:?}"
        );
        // Simplify keeps curves too.
        let s = simplify(&c);
        assert_eq!(count(&s, false), 0, "{s:?}");
    }

    /// Polygon sources skip the curve recovery (a block shadow welds
    /// thousands of them), and a curved source of thousands of segments
    /// is looked up through the grid, not segment by segment: both are
    /// quick.
    #[test]
    fn recovery_is_quick_on_big_inputs() {
        let t = std::time::Instant::now();
        // Many thin quads welded one by one, as a block shadow is made.
        let mut solid = square(0.0);
        for k in 0..300 {
            let x = k as f64 * 0.1;
            let mut quad = BezPath::new();
            quad.move_to((x, 0.0));
            quad.line_to((x + 0.3, 0.0));
            quad.line_to((x + 0.3 + 20.0, 30.0));
            quad.line_to((x + 20.0, 30.0));
            quad.close_path();
            solid = overlay(&solid, &quad, Op::Weld);
        }
        assert!(count(&solid, true) == 0);
        // A curve of 4,000 segments against a circle.
        let n = 4000;
        let mut wavy = BezPath::new();
        for k in 0..n {
            let a = k as f64 / n as f64 * std::f64::consts::TAU;
            let r = 50.0 + 3.0 * (a * 40.0).sin();
            let q = Point::new(r * a.cos(), r * a.sin());
            if k == 0 {
                wavy.move_to(q);
            } else {
                let b = (k as f64 - 0.5) / n as f64 * std::f64::consts::TAU;
                let rb = 50.0 + 3.0 * (b * 40.0).sin();
                let m = Point::new(rb * b.cos(), rb * b.sin());
                wavy.curve_to(m, m, q);
            }
        }
        wavy.close_path();
        let w = overlay(&wavy, &circle(30.0, 0.0, 40.0), Op::Weld);
        assert!(count(&w, true) > 100, "{}", count(&w, true));
        assert!(t.elapsed().as_secs() < 20, "{:?}", t.elapsed());
    }

    #[test]
    fn intersect_and_trim() {
        let i = overlay(&square(0.0), &square(5.0), Op::Intersect);
        assert!((i.area().abs() - 50.0).abs() < 1e-3);
        let t = overlay(&square(0.0), &square(5.0), Op::Trim);
        assert!((t.area().abs() - 50.0).abs() < 1e-3);
        assert!((t.bounding_box().x1 - 5.0).abs() < 1e-6);
    }
}

#[cfg(test)]
mod contour_timing {
    /// How long contours take on many curves (#821): `cargo test
    /// --release -p tracedraw-core -- --ignored frame_times_of_contours
    /// --nocapture`.
    #[test]
    #[ignore]
    fn frame_times_of_contours() {
        use crate::geometry::{BezPath, Point, Shape as _};
        let mut p = BezPath::new();
        for k in 0..40 {
            let c = kurbo::Circle::new(
                Point::new((k % 8) as f64 * 30.0, (k / 8) as f64 * 30.0),
                12.0,
            );
            p.extend(c.to_path(0.01).elements().iter().copied());
        }
        let t = std::time::Instant::now();
        let mut n = 0;
        for i in 1..=20 {
            n += super::offset_with(&p, i as f64 * 0.8, super::Join::Round)
                .elements()
                .len();
        }
        eprintln!("20 steps over 40 circles: {:?}, {n} elements", t.elapsed());
    }
}
