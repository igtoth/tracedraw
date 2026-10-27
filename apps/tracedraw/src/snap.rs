//! Snapping to the document grid, the baseline grid, pixels, guidelines,
//! the page and objects. Objects and the page offer points by the
//! snapping modes (see `snap_points`); a point wins over guidelines and
//! the grids, which win over edges and text baselines.

use crate::app::App;
use crate::live_guides;
use crate::snap_points::{self, Geo, Target};
use tracedraw_core::{
    document::GuideLine,
    geometry::{Affine, Point, Rect, Vec2},
    Shape, ShapeId,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapSettings {
    pub grid: bool,
    pub guides: bool,
    pub objects: bool,
    pub page: bool,
    pub pixels: bool,
    pub baseline_grid: bool,
    pub alignment_guides: bool,
    pub dynamic_guides: bool,
    /// Alt+Q: all snapping off.
    pub off: bool,
}

impl Default for SnapSettings {
    fn default() -> Self {
        SnapSettings {
            grid: false,
            guides: true,
            objects: true,
            page: true,
            pixels: false,
            baseline_grid: false,
            alignment_guides: false,
            dynamic_guides: false,
            off: false,
        }
    }
}

/// At most this many acquired anchors of dynamic guides.
const MAX_ANCHORS: usize = 4;
/// At most this many object boxes for alignment guides.
const MAX_BOXES: usize = 20_000;

/// What the alignment targets were built from.
#[derive(Debug, Clone, PartialEq)]
pub struct AlignKey {
    doc: usize,
    revision: u64,
    page: tracedraw_core::PageId,
    exclude: Vec<ShapeId>,
    view: Option<[f64; 4]>,
    multipage: bool,
    options: live_guides::AlignOptions,
}

/// What stays put of the curves whose nodes are being dragged, on the
/// page (#673): their other nodes and the segments between two of them,
/// in pieces of at most [`STILL_PIECE`] segments (or nodes not on such a
/// segment), each with its bounds. Built once a drag; a move looks only
/// at the pieces near the pointer, so a long curve costs no more a frame
/// than a short one.
#[derive(Debug, Clone, Default)]
pub struct StillGeometry(Vec<(Rect, Geo)>);

/// Segments (or lone nodes) in one piece of [`StillGeometry`].
const STILL_PIECE: usize = 16;

impl StillGeometry {
    pub fn of(start: &[crate::node_edit::StartCurve]) -> Self {
        use tracedraw_core::geometry::{BezPath, PathEl, Shape as _};
        let mut out = Vec::new();
        for c in start {
            let moving: std::collections::HashSet<usize> = c.indices.iter().copied().collect();
            let path = c.transform
                * tracedraw_core::nodes::segments_where(&c.path, |i| !moving.contains(&i));
            // The segments in pieces, each starting where the last ended.
            let mut ends: std::collections::HashSet<(i64, i64)> = Default::default();
            let key = |p: Point| ((p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64);
            let mut piece = BezPath::new();
            let mut count = 0;
            let mut last = Point::ZERO;
            let mut flush = |piece: &mut BezPath, count: &mut usize, at: Point| {
                if *count > 0 {
                    let done = std::mem::take(piece);
                    out.push((
                        done.bounding_box(),
                        Geo::Outline {
                            path: done,
                            points: true,
                        },
                    ));
                }
                *count = 0;
                piece.move_to(at);
            };
            for el in path.elements() {
                match *el {
                    PathEl::MoveTo(p) => {
                        flush(&mut piece, &mut count, p);
                        last = p;
                    }
                    PathEl::ClosePath => {}
                    el => {
                        if count == STILL_PIECE {
                            flush(&mut piece, &mut count, last);
                        }
                        ends.insert(key(last));
                        piece.push(el);
                        count += 1;
                        if let Some(p) = el.end_point() {
                            ends.insert(key(p));
                            last = p;
                        }
                    }
                }
            }
            flush(&mut piece, &mut count, last);
            // Nodes on no still segment, in pieces of their own.
            let lone: Vec<Point> = tracedraw_core::nodes::nodes(&c.path)
                .iter()
                .filter(|n| !moving.contains(&n.index))
                .map(|n| c.transform * n.pos)
                .filter(|p| !ends.contains(&key(*p)))
                .collect();
            for chunk in lone.chunks(STILL_PIECE) {
                let b = chunk
                    .iter()
                    .fold(Rect::from_points(chunk[0], chunk[0]), |r, q| r.union_pt(*q));
                out.push((b, Geo::Nodes(chunk.to_vec())));
            }
        }
        StillGeometry(out)
    }
}

/// The alignment targets of the last frame and what they were built from.
#[derive(Debug, Clone)]
pub struct AlignCache {
    key: AlignKey,
    targets: std::rc::Rc<live_guides::AlignTargets>,
}

impl App {
    /// The snapping radius (Options > Snapping, screen pixels) in mm.
    pub(crate) fn snap_tolerance(&self) -> f64 {
        self.settings.snap.threshold_px.clamp(1.0, 100.0) / self.view.zoom.max(1e-6) as f64
    }

    /// The pieces of `still` within the snapping radius of `p`: what a
    /// node dragged there snaps to of its own curves.
    pub(crate) fn still_near<'a>(&self, still: &'a StillGeometry, p: Point) -> Vec<&'a Geo> {
        if self.snap.off || !self.snap.objects || !self.snap_enabled() {
            return Vec::new();
        }
        let tol = self.snap_tolerance();
        still
            .0
            .iter()
            .filter(|(b, _)| b.inflate(tol, tol).contains(p))
            .map(|(_, g)| g)
            .collect()
    }

    /// Candidate x and y values to snap to; `bounds` adds the page's and
    /// the objects' edges and centres.
    fn snap_candidates(
        &self,
        exclude: &[ShapeId],
        guides: bool,
        bounds: bool,
    ) -> (Vec<f64>, Vec<f64>) {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        let s = self.snap;
        let page = self.page_rect();
        if s.page && bounds {
            xs.extend([page.x0, page.x1, page.center().x]);
            ys.extend([page.y0, page.y1, page.center().y]);
        }
        if s.baseline_grid {
            ys.extend(self.baseline_ys());
        }
        if s.guides && guides {
            if let Ok(p) = self.doc().page(self.page) {
                for g in &p.guides {
                    match g.line {
                        GuideLine::Horizontal { y } => ys.push(y),
                        GuideLine::Vertical { x } => xs.push(x),
                        GuideLine::Angled { .. } => {}
                    }
                }
            }
        }
        if s.objects && bounds {
            for sh in self
                .page_layers()
                .into_iter()
                .filter(|l| l.visible)
                .flat_map(|l| &l.shapes)
            {
                if exclude.contains(&sh.id) {
                    continue;
                }
                let b = sh.bounds();
                xs.extend([b.x0, b.x1, b.center().x]);
                ys.extend([b.y0, b.y1, b.center().y]);
            }
        }
        (xs, ys)
    }

    /// Baselines of the baseline grid, from the first one under the page
    /// top downwards (page coordinates, Y up).
    pub fn baseline_ys(&self) -> Vec<f64> {
        let g = self.doc().metadata.grid;
        let page = self.page_rect();
        let step = g.baseline_spacing.max(0.1);
        let mut out = Vec::new();
        let mut y = page.y1 - g.baseline_start.max(0.0);
        while y >= page.y0 - 1e-9 && out.len() < 100_000 {
            out.push(y);
            y -= step;
        }
        out
    }

    fn snap_axis(&self, v: f64, candidates: &[f64], horizontal: bool) -> Option<f64> {
        let tol = self.snap_tolerance();
        let mut best: Option<(f64, f64)> = None;
        for c in candidates {
            let d = (c - v).abs();
            if d <= tol && best.map(|b| d < b.1).unwrap_or(true) {
                best = Some((*c, d));
            }
        }
        if best.is_none() && self.snap.grid {
            // Grid lines pass through the ruler origin.
            let grid = self.doc().metadata.grid;
            let rulers = self.doc().metadata.rulers.clone();
            let (step, origin) = if horizontal {
                (grid.spacing_x, rulers.origin_x)
            } else {
                (grid.spacing_y, rulers.origin_y)
            };
            let step = step.max(0.001);
            let g = origin + ((v - origin) / step).round() * step;
            if (g - v).abs() <= tol {
                return Some(g);
            }
        }
        if best.is_none() && self.snap.pixels {
            // Whole pixels of the document resolution from the page corner.
            let step = 25.4 / self.document_dpi().max(1.0);
            let g = (v / step).round() * step;
            if (g - v).abs() <= tol {
                return Some(g);
            }
        }
        best.map(|b| b.0)
    }

    /// Snap an x value (a vertical guideline being dragged) to everything
    /// but guidelines.
    pub fn snap_x_without_guides(&self, x: f64) -> f64 {
        if !self.snap_enabled() {
            return x;
        }
        let (xs, _) = self.snap_candidates(&[], false, true);
        self.snap_axis(x, &xs, true).unwrap_or(x)
    }

    /// Snap a y value (a horizontal guideline being dragged) to everything
    /// but guidelines.
    pub fn snap_y_without_guides(&self, y: f64) -> f64 {
        if !self.snap_enabled() {
            return y;
        }
        let (_, ys) = self.snap_candidates(&[], false, true);
        self.snap_axis(y, &ys, false).unwrap_or(y)
    }

    /// Snap a point to everything but guidelines.
    pub fn snap_point_without_guides(&self, p: Point) -> Point {
        Point::new(
            self.snap_x_without_guides(p.x),
            self.snap_y_without_guides(p.y),
        )
    }

    /// Where a line being drawn starts: the last node of the curve in
    /// progress or the last point a dimension or callout tool placed
    /// (tangent and perpendicular points are seen from there).
    pub fn snap_from(&self) -> Option<Point> {
        self.curve
            .as_ref()
            .and_then(|c| c.nodes.last().map(|n| n.0))
            .or_else(|| self.dimension_points.last().copied())
    }

    /// Snapping geometry of the page and of the objects whose bounds come
    /// within `tol` of `p`, leaving out `exclude`.
    fn snap_geometry(&self, p: Point, tol: f64, exclude: &[ShapeId]) -> Vec<Geo> {
        let mut g = Vec::new();
        if self.snap.page {
            snap_points::page_geometry(self.page_rect(), &mut g);
        }
        if self.snap.objects {
            for sh in self
                .page_layers()
                .into_iter()
                .filter(|l| l.visible)
                .flat_map(|l| &l.shapes)
            {
                if exclude.contains(&sh.id) || !sh.visible {
                    continue;
                }
                if !sh.bounds().inflate(tol, tol).contains(p) {
                    continue;
                }
                snap_points::geometry(sh, Affine::IDENTITY, &text_baselines, &mut g);
            }
        }
        g
    }

    /// The object or page point nearest `p` within the snapping radius:
    /// points (`points`), else edges and baselines (`lines`).
    fn object_snap(
        &self,
        p: Point,
        exclude: &[ShapeId],
        own: &[&Geo],
        points: bool,
        lines: bool,
    ) -> Option<Target> {
        let tol = self.snap_tolerance();
        let mut geo = self.snap_geometry(p, tol, exclude);
        // What stays put of the curves being edited (#673): only its few
        // pieces near the pointer, each of at most 16 segments.
        geo.extend(own.iter().map(|g| (*g).clone()));
        let found = snap_points::targets(&geo, p, tol, self.snap_from(), &self.settings.snap.modes);
        snap_points::best(&found, points, lines)
    }

    /// The selection's own snap point under `p` (a node, midpoint,
    /// quadrant or centre, else the point of an edge under it): the one
    /// the pointer marks, which a move carries onto other objects' points
    /// and edges.
    pub fn selection_snap_source(&self, p: Point) -> Option<Point> {
        self.selection_hover_target(p).map(|t| t.point)
    }

    /// The selection's own snap point under `p`, or its edge when no
    /// point is near: what the Pick tool marks, with its name, while the
    /// pointer rests over the selection.
    pub fn selection_hover_target(&self, p: Point) -> Option<Target> {
        if !self.snap_enabled() || !self.snap.objects {
            return None;
        }
        let tol = self.snap_tolerance();
        let mut g = Vec::new();
        let shapes = self.selected_rc();
        let bounds = self.selected_bounds();
        for (s, b) in shapes.iter().zip(bounds.iter()) {
            // Only the objects whose bounds come near the pointer.
            if !b.inflate(tol, tol).contains(p) {
                continue;
            }
            snap_points::geometry(s, Affine::IDENTITY, &text_baselines, &mut g);
        }
        let found = snap_points::targets(&g, p, tol, None, &self.settings.snap.modes);
        snap_points::best(&found, true, true)
    }

    /// Snap a pointer position (drawing tools, text placement).
    pub fn snap_point(&self, p: Point) -> Point {
        self.snap_point_excluding(p, &[])
    }

    /// Snap a pointer position to everything but `exclude` (a handle of
    /// the selection snaps to other objects, not to the selection's own
    /// points).
    pub fn snap_point_excluding(&self, p: Point, exclude: &[ShapeId]) -> Point {
        self.snap_point_with(p, exclude, &[])
    }

    /// Snap the pointer turning the selection: onto a point of another
    /// object or of the page (a node, a midpoint, a centre) only, so that
    /// the turn aims at it. Lines are not taken (page edges and centre
    /// lines, guidelines, grid lines, the bounds of other objects): the
    /// turning handle stands off the object, so lining it up with a line
    /// means nothing, and the turn stuck behind the pointer while it
    /// crossed one, anywhere on the canvas (#933).
    pub fn snap_turn_point(&self, p: Point, exclude: &[ShapeId]) -> Point {
        if !self.snap_enabled() {
            return p;
        }
        match self.object_snap(p, exclude, &[], true, false) {
            Some(t) => {
                self.snap_mark.set(Some(t));
                t.point
            }
            None => p,
        }
    }

    /// Snap the selection's own point `grab` turned about `center` by the
    /// pointer at `p` (rotate mode, a node dragged, #933): the point goes
    /// round its circle towards the pointer; near another object's point
    /// the turn aims at that point, and near an edge the point lands
    /// where its circle crosses the edge, so a node turned onto a line
    /// ends on the line. Returns where the turn aims (only its direction
    /// from `center` counts).
    pub fn snap_turn_grab(
        &self,
        center: Point,
        grab: Point,
        p: Point,
        exclude: &[ShapeId],
    ) -> Point {
        let r = (grab - center).hypot();
        let d = p - center;
        let len = d.hypot();
        if !(r > 1e-9 && len > 1e-9 && r.is_finite() && len.is_finite()) {
            return p;
        }
        let q = center + d * (r / len);
        if !self.snap_enabled() {
            return q;
        }
        let Some(t) = self.object_snap(q, exclude, &[], true, true) else {
            return q;
        };
        if t.mode.is_point() {
            self.snap_mark.set(Some(t));
            return t.point;
        }
        let reach = 2.0 * self.snap_tolerance();
        match t.edge.and_then(|seg| circle_crossing(seg, center, r, q)) {
            Some(x) if (x - q).hypot() <= reach => {
                self.snap_mark.set(Some(Target { point: x, ..t }));
                x
            }
            _ => q,
        }
    }

    /// [`App::snap_point_excluding`], with `own` snapping geometry of the
    /// excluded objects that stays put meanwhile (a node being dragged
    /// snaps to the other nodes and segments of its own curve, #673).
    pub fn snap_point_with(&self, p: Point, exclude: &[ShapeId], own: &[&Geo]) -> Point {
        if self.snap.off {
            return p;
        }
        let enabled = self.snap_enabled();
        if enabled {
            if let Some(t) = self.object_snap(p, exclude, own, true, false) {
                self.acquire_anchor(t);
                self.snap_mark.set(Some(t));
                return t.point;
            }
        }
        if let Some(q) = self.live_guide_point(p, exclude, enabled) {
            return q;
        }
        if !enabled {
            return p;
        }
        let (xs, ys) = self.snap_candidates(exclude, true, false);
        let axis = Point::new(
            self.snap_axis(p.x, &xs, true).unwrap_or(p.x),
            self.snap_axis(p.y, &ys, false).unwrap_or(p.y),
        );
        if axis != p {
            self.mark_axis_snap(p, axis);
            return axis;
        }
        if !self.snap.guides {
            return self.edge_snap(p, exclude, own);
        }
        // Angled guides: project onto the nearest one within tolerance.
        let tol = self.snap_tolerance();
        if let Ok(page) = self.doc().page(self.page) {
            let mut best: Option<(f64, Point)> = None;
            for g in &page.guides {
                if let GuideLine::Angled { .. } = g.line {
                    let d = g.distance(p).abs();
                    if d <= tol && best.map(|b| d < b.0).unwrap_or(true) {
                        let (o, dir) = g.line.point_and_direction();
                        let t = (p - o).dot(dir);
                        best = Some((d, o + dir * t));
                    }
                }
            }
            if let Some((_, q)) = best {
                self.snap_mark.set(Some(crate::snap_points::Target {
                    point: q,
                    mode: crate::snap_points::SnapMode::Guideline,
                    edge: None,
                }));
                return q;
            }
        }
        self.edge_snap(p, exclude, own)
    }

    /// The pointer at `p` snapped to `q` along the axes: mark it, with its
    /// screen tip, when a guideline or the grid took it (guidelines first);
    /// the page's and objects' bounds are not marked.
    fn mark_axis_snap(&self, p: Point, q: Point) {
        use crate::snap_points::{SnapMode, Target};
        let moved_x = (q.x - p.x).abs() > 1e-12;
        let moved_y = (q.y - p.y).abs() > 1e-12;
        let on_guide = self.snap.guides
            && self.doc().page(self.page).is_ok_and(|pg| {
                pg.guides.iter().any(|g| match g.line {
                    GuideLine::Horizontal { y } => moved_y && (y - q.y).abs() < 1e-9,
                    GuideLine::Vertical { x } => moved_x && (x - q.x).abs() < 1e-9,
                    GuideLine::Angled { .. } => false,
                })
            });
        let on_grid = || {
            if !self.snap.grid {
                return false;
            }
            let grid = self.doc().metadata.grid;
            let rulers = &self.doc().metadata.rulers;
            let on = |v: f64, step: f64, origin: f64| {
                let step = step.max(0.001);
                let k = (v - origin) / step;
                (k - k.round()).abs() * step < 1e-9
            };
            (moved_x && on(q.x, grid.spacing_x, rulers.origin_x))
                || (moved_y && on(q.y, grid.spacing_y, rulers.origin_y))
        };
        let mode = if on_guide {
            SnapMode::Guideline
        } else if on_grid() {
            SnapMode::Grid
        } else {
            return;
        };
        self.snap_mark.set(Some(Target {
            point: q,
            mode,
            edge: None,
        }));
    }

    /// Where the centre of rotation goes for the pointer at `p`: when
    /// snapping is on, the nearest within the snap radius of the
    /// selection's box (corners, middles of the sides, centre) and of the
    /// points the pointer snaps to (nodes and points of every object, the
    /// selection's own included, guidelines, the grid, the page).
    pub fn snap_pivot(&self, p: Point) -> Point {
        if !self.snap_enabled() {
            return p;
        }
        let snapped = self.snap_point(p);
        let tol = self.snap_tolerance();
        let boxed = self.selection_bounds().and_then(|b| {
            let (x0, x1, y0, y1) = (b.x0, b.x1, b.y0, b.y1);
            let c = b.center();
            [
                Point::new(x0, y0),
                Point::new(x1, y0),
                Point::new(x0, y1),
                Point::new(x1, y1),
                Point::new(c.x, y0),
                Point::new(c.x, y1),
                Point::new(x0, c.y),
                Point::new(x1, c.y),
                c,
            ]
            .into_iter()
            .map(|q| (q, (q - p).hypot()))
            .filter(|(_, d)| *d <= tol)
            .min_by(|a, b| a.1.total_cmp(&b.1))
        });
        match boxed {
            Some((q, d)) if snapped == p || d < (snapped - p).hypot() => {
                self.snap_mark.set(None);
                q
            }
            _ => snapped,
        }
    }

    /// An object's or the page's edge, or a text baseline, near `p`.
    fn edge_snap(&self, p: Point, exclude: &[ShapeId], own: &[&Geo]) -> Point {
        match self.object_snap(p, exclude, own, false, true) {
            Some(t) => {
                self.snap_mark.set(Some(t));
                t.point
            }
            None => p,
        }
    }

    /// Snap a translation of the selection: its snap point `source`
    /// (when the move began on one) onto another object's point or edge;
    /// otherwise an edge or the centre of its bounds onto a guideline, the
    /// grid, the page or another object's bounds.
    pub fn snap_move_from(&self, source: Option<Point>, bounds: Rect, d: Vec2) -> Vec2 {
        if self.snap.off {
            return d;
        }
        let enabled = self.snap_enabled();
        if let (true, Some(src)) = (enabled, source) {
            if let Some(t) = self.object_snap(src + d, &self.selection, &[], true, true) {
                self.acquire_anchor(t);
                self.snap_mark.set(Some(t));
                return t.point - src;
            }
        }
        let snapped = if enabled {
            self.snap_move(bounds, d)
        } else {
            d
        };
        self.live_guide_move(source, bounds, d, snapped)
    }

    /// Alignment guides on a move (edges, centres, margins and equal
    /// gaps, which win over the grid and guidelines on their axis), else
    /// a dynamic guide for the grabbed point (the selection's snap point,
    /// else its centre); the guides to draw at the place reached.
    fn live_guide_move(&self, source: Option<Point>, bounds: Rect, d: Vec2, snapped: Vec2) -> Vec2 {
        let o = &self.settings.snap.align;
        let tol = self.snap_tolerance();
        let mut out = snapped;
        let mut aligned = false;
        let moving: Vec<Rect> = if o.each_object {
            self.selected_shapes()
                .iter()
                .map(|s| self.visual_bounds(s))
                .collect()
        } else {
            vec![bounds]
        };
        if self.alignment_guides_on() && !moving.is_empty() {
            let t = self.align_targets(&self.selection);
            let m = live_guides::align_move(&moving, d, &t, o, tol);
            if let Some(dx) = m.dx {
                out.x = d.x + dx;
                aligned = true;
            }
            if let Some(dy) = m.dy {
                out.y = d.y + dy;
                aligned = true;
            }
        }
        if !aligned && self.dynamic_guides_on() {
            let grab = source.unwrap_or_else(|| bounds.center());
            let mut anchors = self.dynamic_anchors();
            live_guides::acquire(&mut anchors, grab, MAX_ANCHORS + 1);
            if let Some(h) = self.dynamic_hit(grab + d, &anchors) {
                self.set_overlay(Vec::new(), Vec::new(), Some(h));
                return h.point - grab;
            }
        }
        if self.alignment_guides_on() {
            // The lines of what lines up where the move ends, whichever
            // snapping put it there.
            let t = self.align_targets(&self.selection);
            let at: Vec<Rect> = moving.iter().map(|r| *r + out).collect();
            let lines = live_guides::lines_for(&at, &t, o);
            let gaps = match (
                o.equal_spacing,
                at.iter().copied().reduce(|a, b| a.union(b)),
            ) {
                (true, Some(w)) => {
                    let mut g = live_guides::equal_gaps(w, &t.boxes, true);
                    g.extend(live_guides::equal_gaps(w, &t.boxes, false));
                    g
                }
                _ => Vec::new(),
            };
            self.set_overlay(lines, gaps, None);
        }
        out
    }

    pub fn alignment_guides_on(&self) -> bool {
        self.snap.alignment_guides && !self.snap.off
    }

    pub fn dynamic_guides_on(&self) -> bool {
        self.snap.dynamic_guides && !self.snap.off
    }

    fn set_overlay(
        &self,
        lines: Vec<live_guides::Seg>,
        gaps: Vec<live_guides::Seg>,
        dynamic: Option<live_guides::DynHit>,
    ) {
        self.put_overlay(live_guides::Overlay {
            lines,
            gaps,
            sizes: Vec::new(),
            dynamic,
        });
    }

    fn put_overlay(&self, overlay: live_guides::Overlay) {
        if let Ok(mut o) = self.live_guides.try_borrow_mut() {
            *o = overlay;
        }
    }

    /// Forget this frame's guides (each frame computes them again).
    pub fn clear_live_guides(&self) {
        self.set_overlay(Vec::new(), Vec::new(), None);
    }

    /// The page area in view, when the canvas has been laid out.
    fn view_area(&self) -> Option<Rect> {
        self.canvas_rect
            .is_positive()
            .then(|| self.view.visible_page_rect(self.canvas_rect))
    }

    /// The boxes of the visible objects on the page in view (leaving out
    /// `exclude`) and the values they offer, from the cache while nothing
    /// changed.
    fn align_targets(&self, exclude: &[ShapeId]) -> std::rc::Rc<live_guides::AlignTargets> {
        let o = &self.settings.snap.align;
        let view = self.view_area();
        let key = AlignKey {
            doc: self.doc() as *const _ as usize,
            revision: self.engine.revision(),
            page: self.page,
            exclude: exclude.to_vec(),
            view: view.map(|v| [v.x0, v.y0, v.x1, v.y1]),
            multipage: self.multipage,
            options: o.clone(),
        };
        if let Ok(c) = self.align_cache.try_borrow() {
            if let Some(c) = c.as_ref().filter(|c| c.key == key) {
                return c.targets.clone();
            }
        }
        let outside = |b: Rect| {
            view.map(|v| b.x1 < v.x0 || b.x0 > v.x1 || b.y1 < v.y0 || b.y0 > v.y1)
                .unwrap_or(false)
        };
        // The current page at its place, and in the Multipage View the
        // other pages in view, each moved to where it is displayed.
        let mut pages = vec![(self.page, self.page_rect())];
        if self.multipage {
            pages.extend(
                self.multipage_layout()
                    .into_iter()
                    .filter(|(id, r)| *id != self.page && !outside(*r)),
            );
        }
        let mut boxes = Vec::new();
        let doc = self.doc();
        'pages: for (id, r) in &pages {
            let shift = if *id == self.page {
                Vec2::ZERO
            } else {
                Vec2::new(r.x0, r.y0)
            };
            let layers = doc.layers_for_page(*id).unwrap_or_default();
            for sh in layers
                .into_iter()
                .filter(|l| l.visible)
                .flat_map(|l| &l.shapes)
            {
                if !sh.visible || exclude.contains(&sh.id) {
                    continue;
                }
                let b = self.visual_bounds(sh) + shift;
                if outside(b) {
                    continue;
                }
                boxes.push(b);
                if boxes.len() >= MAX_BOXES {
                    break 'pages;
                }
            }
        }
        let page_rects: Vec<Rect> = if o.page {
            pages.iter().map(|(_, r)| *r).collect()
        } else {
            Vec::new()
        };
        let targets = std::rc::Rc::new(live_guides::AlignTargets::new(boxes, &page_rects, o));
        if let Ok(mut c) = self.align_cache.try_borrow_mut() {
            *c = Some(AlignCache {
                key,
                targets: targets.clone(),
            });
        }
        targets
    }

    /// Remember a point snapped to (a node, a centre, a midpoint...) as
    /// an anchor of dynamic guides.
    fn acquire_anchor(&self, t: Target) {
        use snap_points::SnapMode as M;
        if !self.dynamic_guides_on()
            || !matches!(
                t.mode,
                M::Node | M::Intersection | M::Midpoint | M::Quadrant | M::Center
            )
        {
            return;
        }
        if let Ok(mut a) = self.guide_anchors.try_borrow_mut() {
            live_guides::acquire(&mut a, t.point, MAX_ANCHORS);
        }
    }

    /// The anchors of dynamic guides: the points acquired that are in
    /// view, and the start of the line being drawn.
    fn dynamic_anchors(&self) -> Vec<Point> {
        let view = self.view_area();
        let mut out: Vec<Point> = self
            .guide_anchors
            .try_borrow()
            .map(|a| a.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|p| view.map(|v| v.contains(*p)).unwrap_or(true))
            .collect();
        if let Some(from) = self.snap_from() {
            live_guides::acquire(&mut out, from, MAX_ANCHORS + 1);
        }
        out
    }

    /// The dynamic guide `p` snaps to (and its tick, when ticks snap).
    fn dynamic_hit(&self, p: Point, anchors: &[Point]) -> Option<live_guides::DynHit> {
        let o = &self.settings.snap.dynamic;
        let tol = self.snap_tolerance();
        let h = live_guides::dynamic_snap(p, anchors, &o.clean_angles(), tol)?;
        Some(if o.ticks && o.snap_ticks {
            live_guides::snap_to_tick(h, o.tick_mm.max(0.01), tol)
        } else {
            h
        })
    }

    /// The box being drawn or resized: its fixed point, how many times the
    /// pointer's distance its size is (2 from the centre) and the axes the
    /// handle changes.
    fn size_context(&self) -> Option<(Point, f64, (bool, bool))> {
        use crate::app::{Drag, Handle};
        match &self.drag {
            Drag::Box {
                start, from_center, ..
            } => Some((*start, if *from_center { 2.0 } else { 1.0 }, (true, true))),
            Drag::Scale {
                handle,
                anchor,
                start_bounds,
                mods,
                ..
            } => {
                let (anchor, factor) = if mods.from_center {
                    (start_bounds.center(), 2.0)
                } else {
                    (*anchor, 1.0)
                };
                let axes = match handle {
                    Handle::E | Handle::W => (true, false),
                    Handle::N | Handle::S => (false, true),
                    _ => (true, true),
                };
                Some((anchor, factor, axes))
            }
            _ => None,
        }
    }

    /// Alignment guides for a point (a drawing tool's pointer, a scale
    /// handle): its x or y onto other objects' edges or centres (the other
    /// coordinate still snaps to guidelines and the grids), else onto a
    /// dynamic guide. Not while turning the selection.
    fn live_guide_point(&self, p: Point, exclude: &[ShapeId], enabled: bool) -> Option<Point> {
        if matches!(
            self.drag,
            crate::app::Drag::Rotate { .. } | crate::app::Drag::Pivot
        ) {
            return None;
        }
        let tol = self.snap_tolerance();
        if self.alignment_guides_on() {
            let t = self.align_targets(exclude);
            let (mut x, mut y, mut lines) = live_guides::align_point(p, &t, tol);
            // Equal sizes while a box is drawn or resized: on each axis the
            // nearer of an alignment and a size match wins.
            let mut sizes = Vec::new();
            if let (true, Some((anchor, factor, axes))) =
                (self.settings.snap.align.equal_size, self.size_context())
            {
                let (sx, sy, _) = live_guides::size_snap(p, anchor, factor, axes, &t, tol);
                let wins = |s: Option<f64>, a: Option<f64>, v: f64| match (s, a) {
                    (Some(s), Some(a)) => (s - v).abs() <= (a - v).abs(),
                    (Some(_), None) => true,
                    _ => false,
                };
                let (wx, wy) = (wins(sx, x, p.x), wins(sy, y, p.y));
                if wx {
                    x = sx;
                    // Vertical lines belong to the x axis.
                    lines.retain(|l| (l.a.x - l.b.x).abs() > 1e-9);
                }
                if wy {
                    y = sy;
                    lines.retain(|l| (l.a.y - l.b.y).abs() > 1e-9);
                }
                if wx || wy {
                    // The marks of the place reached.
                    let q = Point::new(x.unwrap_or(p.x), y.unwrap_or(p.y));
                    let (_, _, marks) =
                        live_guides::size_snap(q, anchor, factor, (wx, wy), &t, 1e-6);
                    sizes = marks;
                }
            }
            if x.is_some() || y.is_some() {
                let (xs, ys) = if enabled {
                    self.snap_candidates(exclude, true, false)
                } else {
                    (Vec::new(), Vec::new())
                };
                let q = Point::new(
                    x.or_else(|| self.snap_axis(p.x, &xs, true).filter(|_| enabled))
                        .unwrap_or(p.x),
                    y.or_else(|| self.snap_axis(p.y, &ys, false).filter(|_| enabled))
                        .unwrap_or(p.y),
                );
                self.put_overlay(live_guides::Overlay {
                    lines,
                    gaps: Vec::new(),
                    sizes,
                    dynamic: None,
                });
                return Some(q);
            }
        }
        if self.dynamic_guides_on() {
            let anchors = self.dynamic_anchors();
            if let Some(h) = self.dynamic_hit(p, &anchors) {
                self.set_overlay(Vec::new(), Vec::new(), Some(h));
                return Some(h.point);
            }
        }
        None
    }

    /// Snap a translation of the selection so an edge or centre of its
    /// bounds lands on a candidate.
    pub fn snap_move(&self, bounds: Rect, d: Vec2) -> Vec2 {
        if !self.snap_enabled() {
            return d;
        }
        let (xs, ys) = self.snap_candidates(&self.selection, true, true);
        let moved = bounds + d;
        let mut best_dx: Option<f64> = None;
        for edge in [moved.x0, moved.x1, moved.center().x] {
            if let Some(t) = self.snap_axis(edge, &xs, true) {
                let dx = t - edge;
                if best_dx.map(|b| dx.abs() < b.abs()).unwrap_or(true) {
                    best_dx = Some(dx);
                }
            }
        }
        let mut best_dy: Option<f64> = None;
        for edge in [moved.y0, moved.y1, moved.center().y] {
            if let Some(t) = self.snap_axis(edge, &ys, false) {
                let dy = t - edge;
                if best_dy.map(|b| dy.abs() < b.abs()).unwrap_or(true) {
                    best_dy = Some(dy);
                }
            }
        }
        Vec2::new(d.x + best_dx.unwrap_or(0.0), d.y + best_dy.unwrap_or(0.0))
    }

    pub fn snap_enabled(&self) -> bool {
        !self.snap.off
            && (self.snap.grid
                || self.snap.guides
                || self.snap.objects
                || self.snap.page
                || self.snap.pixels)
    }
}

/// The baselines of a text object on the page, one per laid-out line.
fn text_baselines(s: &Shape, parent: Affine) -> Vec<(Point, Point)> {
    let Some((layout, to_page)) = crate::text_editing::layout_of_text_shape(s, parent) else {
        return Vec::new();
    };
    layout
        .lines
        .iter()
        .filter_map(|l| {
            let (a, b) = l.x_range();
            (b > a).then(|| {
                (
                    to_page * Point::new(a, l.baseline),
                    to_page * Point::new(b, l.baseline),
                )
            })
        })
        .collect()
}

/// Where the circle about `center` of radius `r` crosses the segment
/// `seg` (a curve taken as short straight pieces), the crossing nearest
/// `near`; `None` when it does not cross.
fn circle_crossing(
    seg: tracedraw_core::geometry::PathSeg,
    center: Point,
    r: f64,
    near: Point,
) -> Option<Point> {
    use tracedraw_core::geometry::{ParamCurve, PathSeg};
    let pieces = match seg {
        PathSeg::Line(_) => 1,
        _ => 64,
    };
    let mut best: Option<(f64, Point)> = None;
    let mut a = seg.eval(0.0);
    for i in 1..=pieces {
        let b = seg.eval(i as f64 / pieces as f64);
        // |a + t (b - a) - center|² = r², for t in [0, 1].
        let d = b - a;
        let f = a - center;
        let qa = d.hypot2();
        if qa > 1e-18 {
            let qb = 2.0 * f.dot(d);
            let qc = f.hypot2() - r * r;
            let disc = qb * qb - 4.0 * qa * qc;
            if disc >= 0.0 {
                let s = disc.sqrt();
                for t in [(-qb - s) / (2.0 * qa), (-qb + s) / (2.0 * qa)] {
                    if (-1e-9..=1.0 + 1e-9).contains(&t) {
                        let x = a + d * t;
                        let dist = (x - near).hypot();
                        if best.is_none_or(|(bd, _)| dist < bd) {
                            best = Some((dist, x));
                        }
                    }
                }
            }
        }
        a = b;
    }
    best.map(|(_, x)| x)
        .filter(|x| x.x.is_finite() && x.y.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snap_points::SnapMode;
    use tracedraw_core::{document::Guide, Command, ShapeKind};

    /// A headless app at 1 px per mm: the 10 px radius is 10 mm.
    fn app() -> App {
        let mut app = App::headless();
        app.view.zoom = 1.0;
        app.snap = SnapSettings::default();
        app
    }

    fn rect(app: &mut App, x0: f64, y0: f64, x1: f64, y1: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x0, y0, x1, y1),
            radius: 0.0,
            corners: None,
        })
        .expect("a rectangle")
    }

    #[test]
    fn the_pick_tool_names_the_selections_point_under_the_pointer() {
        let mut app = app();
        // Snapping radius 3 mm at 1 px per mm.
        app.settings.snap.threshold_px = 3.0;
        let id = rect(&mut app, 100.0, 100.0, 160.0, 140.0);
        // Nothing selected: nothing to name.
        assert!(app
            .selection_hover_target(Point::new(160.0, 120.0))
            .is_none());
        app.select(vec![id]);
        let mode = |app: &App, x: f64, y: f64| {
            app.selection_hover_target(Point::new(x, y)).map(|t| t.mode)
        };
        assert_eq!(mode(&app, 130.0, 120.5), Some(SnapMode::Center));
        assert_eq!(mode(&app, 101.0, 139.0), Some(SnapMode::Node));
        assert_eq!(mode(&app, 100.5, 120.0), Some(SnapMode::Midpoint));
        // On a side, away from its points: the edge, at the pointer.
        let t = app
            .selection_hover_target(Point::new(160.5, 108.0))
            .expect("an edge");
        assert_eq!(t.mode, SnapMode::Edge);
        assert!((t.point.x - 160.0).abs() < 1e-9 && (t.point.y - 108.0).abs() < 1e-9);
        // Far from everything, or with snapping to objects off: nothing.
        assert!(mode(&app, 130.0, 110.0).is_none());
        app.snap.objects = false;
        assert!(mode(&app, 130.0, 120.5).is_none());
    }

    #[test]
    fn a_guideline_catches_an_object_moved_near_it() {
        let mut app = app();
        let page = app.page;
        app.run(Command::AddGuide {
            page,
            guide: Guide::vertical(50.0),
        });
        let id = rect(&mut app, 20.0, 20.0, 40.0, 40.0);
        app.select(vec![id]);
        let b = app.selection_bounds().unwrap();
        // The right edge dragged to x = 51 lands on the guideline.
        let d = app.snap_move_from(None, b, Vec2::new(11.0, 0.0));
        assert!((b.x1 + d.x - 50.0).abs() < 1e-9, "{d:?}");
    }

    #[test]
    fn drawing_tools_snap_to_object_points_and_show_the_mark() {
        let mut app = app();
        rect(&mut app, 100.0, 100.0, 140.0, 120.0);
        let p = app.snap_point(Point::new(103.0, 118.0));
        assert_eq!(p, Point::new(100.0, 120.0));
        let mark = app.snap_mark.get().expect("a mark");
        assert_eq!(mark.mode, SnapMode::Node);
        // A point wins over a guideline near it.
        let page = app.page;
        app.run(Command::AddGuide {
            page,
            guide: Guide::horizontal(116.0),
        });
        assert_eq!(
            app.snap_point(Point::new(101.0, 117.0)),
            Point::new(100.0, 120.0)
        );
        // A guideline wins over an edge (2 mm radius).
        app.settings.snap.threshold_px = 2.0;
        let q = app.snap_point(Point::new(141.0, 115.5));
        assert_eq!(q, Point::new(141.0, 116.0));
        // Without the guideline: the edge.
        app.snap.guides = false;
        let q = app.snap_point(Point::new(141.0, 115.5));
        assert_eq!(q, Point::new(140.0, 115.5));
        assert_eq!(app.snap_mark.get().map(|m| m.mode), Some(SnapMode::Edge));
        app.snap.guides = true;
        app.settings.snap.threshold_px = 10.0;
        // Modes can be turned off one by one.
        app.settings.snap.modes.node = false;
        app.snap_mark.set(None);
        let q = app.snap_point(Point::new(103.0, 118.0));
        assert_ne!(q, Point::new(100.0, 120.0));
    }

    #[test]
    fn snapping_to_objects_turns_off_but_the_page_stays() {
        let mut app = app();
        rect(&mut app, 100.0, 100.0, 140.0, 120.0);
        app.snap.objects = false;
        app.snap.guides = false;
        let p = Point::new(103.0, 118.0);
        assert_eq!(app.snap_point(p), p);
        let page = app.page_rect();
        let corner = Point::new(page.x1 - 2.0, page.y1 - 3.0);
        assert_eq!(app.snap_point(corner), Point::new(page.x1, page.y1));
        app.snap.off = true;
        assert_eq!(app.snap_point(corner), corner);
    }

    #[test]
    fn a_move_carries_its_snap_point_onto_another_object() {
        let mut app = app();
        rect(&mut app, 100.0, 100.0, 140.0, 120.0);
        let moving = rect(&mut app, 20.0, 20.0, 30.0, 30.0);
        app.select(vec![moving]);
        // Grabbed at its top right corner node.
        let src = app.selection_snap_source(Point::new(29.0, 29.0));
        assert_eq!(src, Some(Point::new(30.0, 30.0)));
        let b = app.selection_bounds().unwrap();
        // Dropped near the other rectangle's top right corner.
        let d = app.snap_move_from(src, b, Vec2::new(107.0, 88.0));
        assert_eq!(Point::new(30.0, 30.0) + d, Point::new(140.0, 120.0));
        assert_eq!(app.snap_mark.get().map(|m| m.mode), Some(SnapMode::Node));
    }

    #[test]
    fn a_move_grabbed_on_an_edge_follows_another_objects_edge() {
        let mut app = app();
        app.settings.snap.threshold_px = 3.0;
        // A tall rectangle whose left edge is the line x = 100.
        rect(&mut app, 100.0, 50.0, 140.0, 150.0);
        let moving = rect(&mut app, 20.0, 20.0, 40.0, 60.0);
        app.select(vec![moving]);
        // Grabbed on its right edge, away from the nodes and the midpoint.
        let src = app.selection_snap_source(Point::new(40.5, 30.0));
        assert_eq!(src, Some(Point::new(40.0, 30.0)));
        let b = app.selection_bounds().unwrap();
        // Dragged near the other rectangle's left edge at two heights:
        // the grabbed point lands on that edge each time and slides along
        // it with the pointer.
        for dy in [60.0, 85.0] {
            let d = app.snap_move_from(src, b, Vec2::new(61.5, dy));
            assert!((40.0 + d.x - 100.0).abs() < 1e-9, "{d:?}");
            assert!((d.y - dy).abs() < 1e-9, "{d:?}");
            assert_eq!(app.snap_mark.get().map(|m| m.mode), Some(SnapMode::Edge));
            // The mark knows the edge it lies on, to light it up.
            match app.snap_mark.get().and_then(|m| m.edge) {
                Some(tracedraw_core::geometry::PathSeg::Line(l)) => {
                    assert!((l.p0.x - 100.0).abs() < 1e-9 && (l.p1.x - 100.0).abs() < 1e-9);
                }
                other => panic!("not the left edge: {other:?}"),
            }
        }
        // Far from it, it lets go.
        let d = app.snap_move_from(src, b, Vec2::new(50.0, 60.0));
        assert!((d.x - 50.0).abs() < 1e-9, "{d:?}");
    }

    #[test]
    fn alignment_guides_pull_a_drawing_point_and_respect_snapping_off() {
        let mut app = app();
        app.settings.snap.threshold_px = 2.0;
        // Only the alignment guides act.
        app.snap.objects = false;
        app.snap.guides = false;
        rect(&mut app, 100.0, 100.0, 140.0, 120.0);
        let p = Point::new(141.5, 170.0);
        // Off by default: nothing moves.
        assert!(!SnapSettings::default().alignment_guides);
        assert_eq!(app.snap_point(p), p);
        assert!(app.live_guides.borrow().is_empty());
        app.snap.alignment_guides = true;
        assert_eq!(app.snap_point(p), Point::new(140.0, 170.0));
        {
            let o = app.live_guides.borrow();
            assert_eq!(o.lines.len(), 1, "{o:?}");
            let l = o.lines[0];
            assert!((l.a.x - 140.0).abs() < 1e-9 && (l.b.x - 140.0).abs() < 1e-9);
            assert!((l.a.y - 100.0).abs() < 1e-9 && (l.b.y - 170.0).abs() < 1e-9);
        }
        // The centre lines up too, on both axes at once.
        assert_eq!(
            app.snap_point(Point::new(119.0, 121.0)),
            Point::new(120.0, 120.0)
        );
        // Alt+Q: no snapping, no guides.
        app.clear_live_guides();
        app.snap.off = true;
        assert_eq!(app.snap_point(p), p);
        assert!(app.live_guides.borrow().is_empty());
        // A move aligns the moved box's edges with the other box.
        app.snap.off = false;
        let moving = rect(&mut app, 10.0, 10.0, 30.0, 30.0);
        app.select(vec![moving]);
        let b = app.selection_bounds().unwrap();
        let d = app.snap_move_from(None, b, Vec2::new(0.0, 91.0));
        assert!((b.y1 + d.y - 120.0).abs() < 1e-9, "{d:?}");
        assert!(!app.live_guides.borrow().lines.is_empty());
        // The boxes are measured once while nothing changes.
        let first = app.align_cache.borrow().as_ref().map(|c| c.targets.clone());
        app.snap_move_from(None, b, Vec2::new(0.0, 90.5));
        let second = app.align_cache.borrow().as_ref().map(|c| c.targets.clone());
        assert!(matches!((first, second), (Some(a), Some(b)) if std::rc::Rc::ptr_eq(&a, &b)));
    }

    #[test]
    fn equal_sizes_snap_while_drawing_and_resizing() {
        use crate::app::{Drag, Handle};
        let mut app = app();
        app.settings.snap.threshold_px = 2.0;
        app.snap.objects = false;
        app.snap.guides = false;
        app.snap.alignment_guides = true;
        // 40 x 20 mm.
        rect(&mut app, 100.0, 100.0, 140.0, 120.0);
        // Drawing a box from (10, 200): 41 x 21.5 becomes 40 x 20.
        app.drag = Drag::Box {
            start: Point::new(10.0, 200.0),
            current: Point::new(10.0, 200.0),
            from_center: false,
        };
        assert_eq!(
            app.snap_point(Point::new(51.0, 221.5)),
            Point::new(50.0, 220.0)
        );
        assert_eq!(app.live_guides.borrow().sizes.len(), 4);
        // Not while hovering before the press.
        app.drag = Drag::None;
        app.clear_live_guides();
        let p = Point::new(51.0, 221.5);
        assert_eq!(app.snap_point(p), p);
        // Resizing with the right side handle: only the width.
        let id = rect(&mut app, 10.0, 10.0, 30.0, 30.0);
        app.select(vec![id]);
        let b = app.selection_bounds().unwrap();
        app.drag = Drag::Scale {
            handle: Handle::E,
            anchor: Point::new(b.x0, b.center().y),
            start_bounds: b,
            current: Point::new(b.x1, b.center().y),
            grab: Vec2::ZERO,
            mods: Default::default(),
        };
        let q = app.snap_point_excluding(Point::new(49.0, 21.0), &[id]);
        assert_eq!(q, Point::new(50.0, 21.0));
        assert_eq!(app.live_guides.borrow().sizes.len(), 2);
        // The option turns it off.
        app.settings.snap.align.equal_size = false;
        let p = Point::new(49.0, 21.0);
        assert_eq!(app.snap_point_excluding(p, &[id]), p);
    }

    #[test]
    fn the_multipage_view_aligns_with_objects_of_the_other_pages() {
        let mut app = app();
        app.settings.snap.threshold_px = 2.0;
        app.snap.objects = false;
        app.snap.guides = false;
        app.snap.alignment_guides = true;
        let first = app.page;
        app.add_page();
        let second = app.doc().pages[1].id;
        app.page = second;
        rect(&mut app, 10.0, 10.0, 50.0, 30.0);
        app.page = first;
        app.set_multipage(true);
        let r = app
            .multipage_layout()
            .into_iter()
            .find(|(id, _)| *id == second)
            .map(|(_, r)| r)
            .expect("the second page");
        // The rectangle's right side as displayed.
        let right = r.x0 + 50.0;
        let p = Point::new(right + 1.5, r.y0 + 170.0);
        let q = app.snap_point(p);
        assert!((q.x - right).abs() < 1e-9, "{q:?}");
        // Without the Multipage View the other page is not in sight.
        app.set_multipage(false);
        assert_eq!(app.snap_point(p), p);
    }

    #[test]
    fn dynamic_guides_come_from_acquired_points() {
        let mut app = app();
        app.settings.snap.threshold_px = 2.0;
        app.snap.dynamic_guides = true;
        rect(&mut app, 100.0, 100.0, 140.0, 120.0);
        // Resting on the corner acquires it.
        assert_eq!(
            app.snap_point(Point::new(100.5, 100.5)),
            Point::new(100.0, 100.0)
        );
        assert_eq!(*app.guide_anchors.borrow(), vec![Point::new(100.0, 100.0)]);
        // Away from the object, near the 45 degree guide from it.
        app.clear_live_guides();
        let q = app.snap_point(Point::new(150.5, 149.0));
        assert!(
            (q.x - 149.75).abs() < 1e-9 && (q.y - 149.75).abs() < 1e-9,
            "{q:?}"
        );
        let h = app.live_guides.borrow().dynamic.expect("a guide");
        assert_eq!(h.anchor, Point::new(100.0, 100.0));
        assert!((h.angle - 45.0).abs() < 1e-9);
        // Only the angles chosen: without 45 the point stays.
        app.settings.snap.dynamic.angles = vec![0.0, 90.0];
        let p = Point::new(150.5, 149.0);
        assert_eq!(app.snap_point(p), p);
        // Dynamic guides off, or snapping off: nothing.
        app.settings.snap.dynamic.angles = vec![45.0];
        app.snap.off = true;
        assert_eq!(app.snap_point(p), p);
        app.snap.off = false;
        app.snap.dynamic_guides = false;
        assert_eq!(app.snap_point(p), p);
    }

    #[test]
    fn lines_snap_to_tangent_and_perpendicular_points_from_their_start() {
        let mut app = app();
        app.settings.snap.threshold_px = 2.0;
        rect(&mut app, 100.0, 100.0, 140.0, 120.0);
        app.curve = Some(crate::app::CurveInProgress {
            nodes: vec![(Point::new(125.0, 60.0), None)],
            smooth: false,
            dragging_handle: false,
            bspline: false,
            ..Default::default()
        });
        let p = app.snap_point(Point::new(125.8, 99.5));
        assert_eq!(p, Point::new(125.0, 100.0));
        assert_eq!(
            app.snap_mark.get().map(|m| m.mode),
            Some(SnapMode::Perpendicular)
        );
    }
}
