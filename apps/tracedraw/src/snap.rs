//! Snapping (grid, guidelines, page, objects) and guideline handling.

use crate::app::{App, Drag};
use tracedraw_core::{
    document::Guide,
    geometry::{Point, Rect, Vec2},
    Command,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapSettings {
    pub grid: bool,
    pub guides: bool,
    pub objects: bool,
    pub page: bool,
}

impl Default for SnapSettings {
    fn default() -> Self {
        SnapSettings {
            grid: false,
            guides: true,
            objects: true,
            page: true,
        }
    }
}

pub const GRID_MM: f64 = 10.0;

impl App {
    fn snap_tolerance(&self) -> f64 {
        6.0 / self.view.zoom as f64
    }

    /// Candidate x and y values to snap to.
    fn snap_candidates(&self, exclude: &[tracedraw_core::ShapeId]) -> (Vec<f64>, Vec<f64>) {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        let s = self.snap;
        let page = self.page_rect();
        if s.page {
            xs.extend([page.x0, page.x1, page.center().x]);
            ys.extend([page.y0, page.y1, page.center().y]);
        }
        if s.guides {
            if let Ok(p) = self.doc().page(self.page) {
                for g in &p.guides {
                    match g {
                        Guide::Horizontal { y } => ys.push(*y),
                        Guide::Vertical { x } => xs.push(*x),
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

    fn snap_axis(&self, v: f64, candidates: &[f64]) -> Option<f64> {
        let tol = self.snap_tolerance();
        let mut best: Option<(f64, f64)> = None;
        for c in candidates {
            let d = (c - v).abs();
            if d <= tol && best.map(|b| d < b.1).unwrap_or(true) {
                best = Some((*c, d));
            }
        }
        if best.is_none() && self.snap.grid {
            let g = (v / GRID_MM).round() * GRID_MM;
            if (g - v).abs() <= tol {
                return Some(g);
            }
        }
        best.map(|b| b.0)
    }

    /// Snap a pointer position (drawing tools, text placement).
    pub fn snap_point(&self, p: Point) -> Point {
        if !self.snap_enabled() {
            return p;
        }
        let (xs, ys) = self.snap_candidates(&[]);
        Point::new(
            self.snap_axis(p.x, &xs).unwrap_or(p.x),
            self.snap_axis(p.y, &ys).unwrap_or(p.y),
        )
    }

    /// Snap a translation of the selection so an edge or centre of its
    /// bounds lands on a candidate.
    pub fn snap_move(&self, bounds: Rect, d: Vec2) -> Vec2 {
        if !self.snap_enabled() {
            return d;
        }
        let (xs, ys) = self.snap_candidates(&self.selection);
        let moved = bounds + d;
        let mut best_dx: Option<f64> = None;
        for edge in [moved.x0, moved.x1, moved.center().x] {
            if let Some(t) = self.snap_axis(edge, &xs) {
                let dx = t - edge;
                if best_dx.map(|b| dx.abs() < b.abs()).unwrap_or(true) {
                    best_dx = Some(dx);
                }
            }
        }
        let mut best_dy: Option<f64> = None;
        for edge in [moved.y0, moved.y1, moved.center().y] {
            if let Some(t) = self.snap_axis(edge, &ys) {
                let dy = t - edge;
                if best_dy.map(|b| dy.abs() < b.abs()).unwrap_or(true) {
                    best_dy = Some(dy);
                }
            }
        }
        Vec2::new(d.x + best_dx.unwrap_or(0.0), d.y + best_dy.unwrap_or(0.0))
    }

    pub fn snap_enabled(&self) -> bool {
        self.snap.grid || self.snap.guides || self.snap.objects || self.snap.page
    }

    // ----- guidelines ---------------------------------------------------------

    pub fn guide_at(&self, p: Point) -> Option<usize> {
        let tol = 4.0 / self.view.zoom as f64;
        let page = self.doc().page(self.page).ok()?;
        page.guides.iter().position(|g| match g {
            Guide::Horizontal { y } => (y - p.y).abs() <= tol,
            Guide::Vertical { x } => (x - p.x).abs() <= tol,
        })
    }

    pub fn add_guide(&mut self, guide: Guide) {
        let page = self.page;
        self.run(Command::AddGuide { page, guide });
    }

    pub fn move_guide(&mut self, index: usize, guide: Guide) {
        let page = self.page;
        if self.engine.undo_label() == Some("Move Guideline") {
            let _ = self.engine.undo();
        }
        let _ = self
            .engine
            .run_with_label(&Command::MoveGuide { page, index, guide }, "Move Guideline");
    }

    pub fn delete_guide(&mut self, index: usize) {
        let page = self.page;
        self.run(Command::DeleteGuide { page, index });
        self.selected_guide = None;
    }

    pub fn finish_guide_drag(&mut self) {
        if let Drag::NewGuide { horizontal, pos } = self.drag.clone() {
            if self.canvas_rect.contains(self.view.to_screen(pos)) {
                let guide = if horizontal {
                    Guide::Horizontal { y: pos.y }
                } else {
                    Guide::Vertical { x: pos.x }
                };
                self.add_guide(guide);
            }
            self.drag = Drag::None;
        }
    }
}
