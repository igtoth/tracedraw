//! Step and Repeat's live preview (#1197): while the Step and Repeat
//! panel is being worked in (the pointer on it, one of its fields being
//! typed in, or the last click was in it), the copies an Apply would make
//! are outlined on the canvas, dashed in the selection colour as the other
//! panels' previews. Nothing is added to the drawing until Apply, which
//! stays one undo step; the preview goes when the panel is closed or
//! hidden, when the work moves elsewhere (a click outside the panel) and
//! when nothing is selected.
//!
//! Cost: the selected objects' outlines are flattened once, in page
//! space, and kept until the selection, the drawing, the values or the
//! zoom change. Each frame the outlines are moved to the screen once and
//! every copy is drawn by adding its screen offset, as line segments;
//! copies outside the drawing window are skipped, and past
//! [`OUTLINE_BUDGET`] points in all the copies show as their boxes.

use crate::app::App;
use crate::ops2::StepRepeat;
use crate::theme::Tokens;
use egui::{Painter, Pos2};
use tracedraw_core::geometry::{PathEl, Point, Rect, Vec2};
use tracedraw_core::ShapeId;

/// The most outline points drawn for all the copies in a frame; past it
/// each copy shows as its box.
pub const OUTLINE_BUDGET: usize = 20_000;

/// What the cached outlines were worked out for.
#[derive(Debug, Clone, PartialEq)]
pub struct StepPreviewKey {
    settings: StepRepeat,
    revision: u64,
    drawing: Option<u64>,
    selection: Vec<ShapeId>,
    zoom: f32,
}

/// The selection's outlines (page space, flattened), their box and the
/// step from one copy to the next.
#[derive(Debug, Clone)]
pub struct StepPreview {
    key: StepPreviewKey,
    pub outlines: Vec<(Vec<Point>, bool)>,
    pub bounds: Rect,
    pub step: Vec2,
    pub copies: u32,
    points: usize,
}

impl StepPreview {
    /// Each copy as its box rather than its outlines.
    pub fn as_boxes(&self) -> bool {
        self.points.saturating_mul(self.copies as usize) > OUTLINE_BUDGET
    }

    /// The box of copy `k` (1 is the first copy).
    pub fn copy_bounds(&self, k: u32) -> Rect {
        let d = self.step * k as f64;
        Rect::new(
            self.bounds.x0 + d.x,
            self.bounds.y0 + d.y,
            self.bounds.x1 + d.x,
            self.bounds.y1 + d.y,
        )
    }
}

impl App {
    /// The distance from one copy to the next for the selection: the
    /// offsets as typed, or with Spacing the selection's size plus them.
    pub fn step_and_repeat_step(&self) -> Option<Vec2> {
        let b = self.selection_bounds()?;
        let sr = &self.step_repeat;
        let dx = if sr.mode_x == 1 {
            b.width() + sr.dx
        } else {
            sr.dx
        };
        let dy = if sr.mode_y == 1 {
            b.height() + sr.dy
        } else {
            sr.dy
        };
        Some(Vec2::new(dx, dy))
    }

    /// The preview of the copies an Apply would make, worked out again
    /// only when its key changed; `None` with nothing selected.
    pub fn step_and_repeat_preview(&mut self) -> Option<&StepPreview> {
        if self.selection.is_empty() {
            self.step_preview = None;
            return None;
        }
        let key = StepPreviewKey {
            settings: self.step_repeat.clone(),
            revision: self.engine.revision(),
            drawing: self.docs.get(self.active_doc).map(|d| d.drawing),
            selection: self.selection.clone(),
            zoom: self.view.zoom,
        };
        if self.step_preview.as_ref().is_none_or(|p| p.key != key) {
            self.step_preview = self.build_step_preview(key);
        }
        self.step_preview.as_ref()
    }

    fn build_step_preview(&self, key: StepPreviewKey) -> Option<StepPreview> {
        let step = self.step_and_repeat_step()?;
        let bounds = self.selection_bounds()?;
        // A quarter of a screen pixel, as the canvas flattens.
        let tol = 0.25 / self.view.zoom.max(0.05) as f64;
        let mut outlines: Vec<(Vec<Point>, bool)> = Vec::new();
        for s in self.selected_shapes() {
            let path = s.page_path();
            let mut cur: Vec<Point> = Vec::new();
            let mut closed = false;
            tracedraw_core::geometry::flatten(&path, tol, &mut |el| match el {
                PathEl::MoveTo(p) => {
                    if cur.len() > 1 {
                        outlines.push((std::mem::take(&mut cur), closed));
                    }
                    cur.clear();
                    closed = false;
                    cur.push(p);
                }
                PathEl::LineTo(p) => cur.push(p),
                PathEl::ClosePath => closed = true,
                _ => {}
            });
            if cur.len() > 1 {
                outlines.push((cur, closed));
            }
        }
        let points = outlines.iter().map(|(p, c)| p.len() + *c as usize).sum();
        Some(StepPreview {
            copies: key.settings.copies.max(1),
            key,
            outlines,
            bounds,
            step,
            points,
        })
    }
}

/// Draw the preview of the copies when the panel asked for it this frame.
pub fn draw_preview(app: &App, painter: &Painter) {
    if !app.step_preview_shown {
        return;
    }
    let Some(pv) = app.step_preview.as_ref() else {
        return;
    };
    let view = &app.view;
    let stroke = egui::Stroke::new(1.0, Tokens::SELECTION);
    let clip = painter.clip_rect();
    let origin = view.to_screen(Point::ZERO);
    let step = view.to_screen(Point::new(pv.step.x, pv.step.y)) - origin;
    let boxes = pv.as_boxes();
    // The outlines on screen once; each copy adds its offset.
    let screen: Vec<(Vec<Pos2>, bool)> = if boxes {
        Vec::new()
    } else {
        pv.outlines
            .iter()
            .map(|(pts, closed)| (pts.iter().map(|p| view.to_screen(*p)).collect(), *closed))
            .collect()
    };
    let mut buf: Vec<Pos2> = Vec::new();
    for k in 1..=pv.copies {
        let b = pv.copy_bounds(k);
        let r = egui::Rect::from_two_pos(
            view.to_screen(Point::new(b.x0, b.y0)),
            view.to_screen(Point::new(b.x1, b.y1)),
        );
        if !r.expand(1.0).intersects(clip) {
            continue;
        }
        if boxes {
            buf.clear();
            buf.extend([
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
                r.left_top(),
            ]);
            crate::canvas::dashed_polyline(painter, &buf, stroke, 4.0, 3.0);
            continue;
        }
        let d = step * k as f32;
        for (pts, closed) in &screen {
            buf.clear();
            buf.extend(pts.iter().map(|p| *p + d));
            if *closed {
                if let Some(first) = buf.first().copied() {
                    buf.push(first);
                }
            }
            crate::canvas::dashed_polyline(painter, &buf, stroke, 4.0, 3.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::ShapeKind;

    fn app_with_rect() -> (App, ShapeId) {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 30.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("a rectangle");
        app.select(vec![id]);
        (app, id)
    }

    /// #1197: the preview has a copy for each one Apply makes, at the
    /// same places, and adds nothing to the drawing; Apply is one undo
    /// step and makes exactly the copies previewed.
    #[test]
    fn the_preview_shows_the_copies_apply_makes() {
        let (mut app, _) = app_with_rect();
        app.step_repeat.copies = 3;
        app.step_repeat.dx = 25.0;
        app.step_repeat.dy = 0.0;
        let before = app.engine.revision();
        let shapes = app.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.shapes.len())
            .sum::<usize>();
        let pv = app.step_and_repeat_preview().expect("a preview").clone();
        assert_eq!(app.engine.revision(), before, "the preview changes nothing");
        assert_eq!(pv.copies, 3);
        assert!(!pv.as_boxes());
        assert_eq!(pv.outlines.len(), 1);
        let previewed: Vec<Rect> = (1..=3).map(|k| pv.copy_bounds(k)).collect();
        app.step_and_repeat_apply();
        let after = app.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.shapes.len())
            .sum::<usize>();
        assert_eq!(after, shapes + 3);
        let made: Vec<Rect> = app
            .selected_shapes()
            .iter()
            .map(|s| tracedraw_core::geometry::Shape::bounding_box(&s.page_path()))
            .collect();
        for (p, m) in previewed.iter().zip(&made) {
            assert!(
                (p.x0 - m.x0).abs() < 1e-6 && (p.y0 - m.y0).abs() < 1e-6,
                "{p:?} {m:?}"
            );
        }
        app.undo();
        let undone = app.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.shapes.len())
            .sum::<usize>();
        assert_eq!(undone, shapes, "one undo takes every copy away");
    }

    /// Spacing mode steps by the selection's size plus the distance; the
    /// preview follows a changed value at once and is kept while nothing
    /// changes.
    #[test]
    fn the_preview_follows_the_values_and_is_cached() {
        let (mut app, _) = app_with_rect();
        app.step_repeat.copies = 2;
        app.step_repeat.mode_x = 1;
        app.step_repeat.dx = 5.0;
        app.step_repeat.dy = 0.0;
        let step = app.step_and_repeat_preview().expect("a preview").step;
        assert!((step.x - 25.0).abs() < 1e-9, "{step:?}");
        let ptr = app.step_preview.as_ref().map(|p| p.outlines.as_ptr());
        app.step_and_repeat_preview();
        assert_eq!(
            app.step_preview.as_ref().map(|p| p.outlines.as_ptr()),
            ptr,
            "kept while nothing changes"
        );
        app.step_repeat.dy = -7.0;
        let step = app.step_and_repeat_preview().expect("a preview").step;
        assert!((step.y + 7.0).abs() < 1e-9);
        app.select(Vec::new());
        assert!(app.step_and_repeat_preview().is_none());
    }

    /// Many copies of a detailed outline show as boxes.
    #[test]
    fn many_copies_show_as_boxes() {
        let (mut app, _) = app_with_rect();
        app.step_repeat.copies = 999;
        let pv = app.step_and_repeat_preview().expect("a preview");
        // A rectangle has 5 points: 999 copies stay under the budget.
        assert!(!pv.as_boxes());
        let mut big = pv.clone();
        big.points = OUTLINE_BUDGET;
        big.copies = 2;
        assert!(big.as_boxes());
    }
}
