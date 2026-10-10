//! Snapping to the document grid, the baseline grid, pixels, guidelines,
//! the page and objects. Objects and the page offer points by the
//! snapping modes (see `snap_points`); a point wins over guidelines and
//! the grids, which win over edges and text baselines.

use crate::app::App;
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
            alignment_guides: true,
            dynamic_guides: false,
            off: false,
        }
    }
}

impl App {
    /// The snapping radius (Options > Snapping, screen pixels) in mm.
    fn snap_tolerance(&self) -> f64 {
        self.settings.snap.threshold_px.clamp(1.0, 100.0) / self.view.zoom.max(1e-6) as f64
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
            if let Ok(p) = self.doc().page(self.page) {
                for sh in p
                    .layers
                    .iter()
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
            let rulers = self.doc().metadata.rulers;
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
            if let Ok(page) = self.doc().page(self.page) {
                for sh in page
                    .layers
                    .iter()
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
        }
        g
    }

    /// The object or page point nearest `p` within the snapping radius:
    /// points (`points`), else edges and baselines (`lines`).
    fn object_snap(
        &self,
        p: Point,
        exclude: &[ShapeId],
        points: bool,
        lines: bool,
    ) -> Option<Target> {
        let tol = self.snap_tolerance();
        let geo = self.snap_geometry(p, tol, exclude);
        let found = snap_points::targets(&geo, p, tol, self.snap_from(), &self.settings.snap.modes);
        snap_points::best(&found, points, lines)
    }

    /// The selection's own snap point under `p` (nodes, midpoints,
    /// quadrants, centres), which a move carries onto other objects.
    pub fn selection_snap_source(&self, p: Point) -> Option<Point> {
        if !self.snap_enabled() || !self.snap.objects {
            return None;
        }
        let tol = self.snap_tolerance();
        let mut g = Vec::new();
        for s in self.selected_shapes() {
            snap_points::geometry(&s, Affine::IDENTITY, &text_baselines, &mut g);
        }
        let found = snap_points::targets(&g, p, tol, None, &self.settings.snap.modes);
        snap_points::best(&found, true, false).map(|t| t.point)
    }

    /// Snap a pointer position (drawing tools, text placement).
    pub fn snap_point(&self, p: Point) -> Point {
        if !self.snap_enabled() {
            return p;
        }
        if let Some(t) = self.object_snap(p, &[], true, false) {
            self.snap_mark.set(Some(t));
            return t.point;
        }
        let (xs, ys) = self.snap_candidates(&[], true, false);
        let axis = Point::new(
            self.snap_axis(p.x, &xs, true).unwrap_or(p.x),
            self.snap_axis(p.y, &ys, false).unwrap_or(p.y),
        );
        if axis != p {
            return axis;
        }
        if !self.snap.guides {
            return self.edge_snap(p);
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
                return q;
            }
        }
        self.edge_snap(p)
    }

    /// An object's or the page's edge, or a text baseline, near `p`.
    fn edge_snap(&self, p: Point) -> Point {
        match self.object_snap(p, &[], false, true) {
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
        if !self.snap_enabled() {
            return d;
        }
        if let Some(src) = source {
            if let Some(t) = self.object_snap(src + d, &self.selection, true, true) {
                self.snap_mark.set(Some(t));
                return t.point - src;
            }
        }
        self.snap_move(bounds, d)
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
            let (a, b) = (l.edges.first()?, l.edges.last()?);
            (b > a).then(|| {
                (
                    to_page * Point::new(*a, l.baseline),
                    to_page * Point::new(*b, l.baseline),
                )
            })
        })
        .collect()
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
    fn lines_snap_to_tangent_and_perpendicular_points_from_their_start() {
        let mut app = app();
        app.settings.snap.threshold_px = 2.0;
        rect(&mut app, 100.0, 100.0, 140.0, 120.0);
        app.curve = Some(crate::app::CurveInProgress {
            nodes: vec![(Point::new(125.0, 60.0), None)],
            smooth: false,
            dragging_handle: false,
        });
        let p = app.snap_point(Point::new(125.8, 99.5));
        assert_eq!(p, Point::new(125.0, 100.0));
        assert_eq!(
            app.snap_mark.get().map(|m| m.mode),
            Some(SnapMode::Perpendicular)
        );
    }
}
