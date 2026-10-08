//! Mapping between page space (mm, Y up, origin bottom-left) and screen
//! space (pixels, Y down).

use egui::{Pos2, Rect as ERect, Vec2};
use tracedraw_core::geometry::{Point, Rect};

#[derive(Debug, Clone, Copy)]
pub struct View {
    /// Pixels per millimetre.
    pub zoom: f32,
    /// Screen position of the page origin (bottom-left corner).
    pub origin: Pos2,
}

impl Default for View {
    fn default() -> Self {
        View {
            zoom: 3.0,
            origin: Pos2::new(100.0, 800.0),
        }
    }
}

impl View {
    pub fn to_screen(&self, p: Point) -> Pos2 {
        Pos2::new(
            self.origin.x + p.x as f32 * self.zoom,
            self.origin.y - p.y as f32 * self.zoom,
        )
    }

    pub fn to_page(&self, s: Pos2) -> Point {
        Point::new(
            ((s.x - self.origin.x) / self.zoom) as f64,
            ((self.origin.y - s.y) / self.zoom) as f64,
        )
    }

    pub fn rect_to_screen(&self, r: Rect) -> ERect {
        let a = self.to_screen(Point::new(r.x0, r.y1));
        let b = self.to_screen(Point::new(r.x1, r.y0));
        ERect::from_min_max(a, b)
    }

    /// Zoom around a screen point, keeping it fixed.
    pub fn zoom_at(&mut self, anchor: Pos2, factor: f32) {
        let page = self.to_page(anchor);
        self.zoom = (self.zoom * factor).clamp(0.05, 400.0);
        let after = self.to_screen(page);
        self.origin += anchor - after;
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.origin += delta;
    }

    /// Fit a page rect into the available screen rect with a margin.
    pub fn fit(&mut self, page: Rect, screen: ERect) {
        let margin = 40.0;
        let zx = (screen.width() - 2.0 * margin) / page.width() as f32;
        let zy = (screen.height() - 2.0 * margin) / page.height() as f32;
        self.zoom = zx.min(zy).clamp(0.05, 400.0);
        let w = page.width() as f32 * self.zoom;
        let h = page.height() as f32 * self.zoom;
        self.origin = Pos2::new(screen.center().x - w / 2.0, screen.center().y + h / 2.0);
    }
}
