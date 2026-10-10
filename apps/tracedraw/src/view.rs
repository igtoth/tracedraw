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

    /// Page-space rectangle currently visible in the canvas.
    pub fn visible_page_rect(&self, screen: ERect) -> Rect {
        let a = self.to_page(screen.min);
        let b = self.to_page(screen.max);
        Rect::from_points(a, b)
    }

    /// Fit a page-space rectangle (the page, the drawing, the selection or
    /// a zoom box anywhere on the page) into the screen rect with a margin,
    /// centred.
    pub fn fit(&mut self, r: Rect, screen: ERect) {
        let margin = 40.0;
        let w = (r.width() as f32).max(1e-3);
        let h = (r.height() as f32).max(1e-3);
        let zx = (screen.width() - 2.0 * margin).max(1.0) / w;
        let zy = (screen.height() - 2.0 * margin).max(1.0) / h;
        self.zoom = zx.min(zy).clamp(0.05, 400.0);
        let c = r.center();
        self.origin = Pos2::new(
            screen.center().x - c.x as f32 * self.zoom,
            screen.center().y + c.y as f32 * self.zoom,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rectangle away from the page origin ends up centred on screen:
    /// zoom to fit, to the selection and the zoom box all rely on this.
    #[test]
    fn fit_centres_rectangles_anywhere_on_the_page() {
        let screen = ERect::from_min_size(Pos2::new(50.0, 80.0), Vec2::new(800.0, 600.0));
        let mut v = View::default();
        let r = Rect::new(41.6, 168.6, 124.7, 226.8);
        v.fit(r, screen);
        let c = v.to_screen(r.center());
        assert!((c.x - screen.center().x).abs() < 0.01, "{c:?}");
        assert!((c.y - screen.center().y).abs() < 0.01, "{c:?}");
        let s = v.rect_to_screen(r);
        assert!(screen.contains_rect(s), "{s:?} inside {screen:?}");
        // The whole page as before: centred too.
        v.fit(Rect::new(0.0, 0.0, 210.0, 297.0), screen);
        let pc = v.to_screen(Point::new(105.0, 148.5));
        assert!((pc - screen.center()).length() < 0.01);
    }

    #[test]
    fn zoom_at_keeps_the_anchor_fixed() {
        let mut v = View::default();
        let anchor = Pos2::new(400.0, 300.0);
        let before = v.to_page(anchor);
        v.zoom_at(anchor, 3.0);
        let after = v.to_page(anchor);
        assert!((before - after).hypot() < 1e-3);
    }
}
