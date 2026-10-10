//! Snapping to the document grid, the baseline grid, pixels, guidelines,
//! the page and objects.

use crate::app::App;
use tracedraw_core::{
    document::GuideLine,
    geometry::{Point, Rect, Vec2},
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

    /// Candidate x and y values to snap to.
    fn snap_candidates(
        &self,
        exclude: &[tracedraw_core::ShapeId],
        guides: bool,
    ) -> (Vec<f64>, Vec<f64>) {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        let s = self.snap;
        let page = self.page_rect();
        if s.page {
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
        if s.objects {
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
        let (xs, _) = self.snap_candidates(&[], false);
        self.snap_axis(x, &xs, true).unwrap_or(x)
    }

    /// Snap a y value (a horizontal guideline being dragged) to everything
    /// but guidelines.
    pub fn snap_y_without_guides(&self, y: f64) -> f64 {
        if !self.snap_enabled() {
            return y;
        }
        let (_, ys) = self.snap_candidates(&[], false);
        self.snap_axis(y, &ys, false).unwrap_or(y)
    }

    /// Snap a point to everything but guidelines.
    pub fn snap_point_without_guides(&self, p: Point) -> Point {
        Point::new(
            self.snap_x_without_guides(p.x),
            self.snap_y_without_guides(p.y),
        )
    }

    /// Snap a pointer position (drawing tools, text placement).
    pub fn snap_point(&self, p: Point) -> Point {
        if !self.snap_enabled() {
            return p;
        }
        let (xs, ys) = self.snap_candidates(&[], true);
        let axis = Point::new(
            self.snap_axis(p.x, &xs, true).unwrap_or(p.x),
            self.snap_axis(p.y, &ys, false).unwrap_or(p.y),
        );
        if axis != p || !self.snap.guides {
            return axis;
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
        axis
    }

    /// Snap a translation of the selection so an edge or centre of its
    /// bounds lands on a candidate.
    pub fn snap_move(&self, bounds: Rect, d: Vec2) -> Vec2 {
        if !self.snap_enabled() {
            return d;
        }
        let (xs, ys) = self.snap_candidates(&self.selection, true);
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
