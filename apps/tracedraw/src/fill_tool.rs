//! Interactive Fill tool: the fountain handles on the canvas and the
//! property bar that edits the selected object's fill in place.
//!
//! A linear fountain shows its axis as a dashed line between two square
//! nodes (start and end colour); dragging a node turns the axis around
//! the object's centre and the fill angle follows. Radial, conical and
//! square fountains show one node at the centre, which drags the centre
//! offset. The formulas match `docs/behavior/fountain-fill.md`.

use crate::app::App;
use tracedraw_core::{
    geometry::{Point, Rect, Vec2},
    style::{Fountain, FountainKind},
    Command, Fill, ShapeId,
};

/// Which fountain handle a drag holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FountainHandle {
    Start,
    End,
    Center,
}

/// Handle positions of a fountain on the object's page bounds.
pub fn fountain_handles(f: &Fountain, b: Rect) -> Vec<(FountainHandle, Point)> {
    let c = b.center();
    match f.kind {
        FountainKind::Linear => {
            let a = f.angle.to_radians();
            let (ca, sa) = (a.cos(), a.sin());
            let h = (b.width() * ca.abs() + b.height() * sa.abs()) / 2.0;
            let d = Vec2::new(ca, sa) * h;
            vec![(FountainHandle::Start, c - d), (FountainHandle::End, c + d)]
        }
        _ => {
            let p = Point::new(
                c.x + f.offset.x * b.width() / 2.0,
                c.y + f.offset.y * b.height() / 2.0,
            );
            vec![(FountainHandle::Center, p)]
        }
    }
}

impl App {
    /// The selected object's fountain, with its id and bounds.
    pub fn selected_fountain(&self) -> Option<(ShapeId, Fountain, Rect)> {
        let s = self.selected_shapes().into_iter().next()?;
        match &s.fill {
            Fill::Fountain(f) => Some((s.id, f.clone(), s.bounds())),
            _ => None,
        }
    }

    /// The handle under a page point, within 6 screen pixels.
    pub fn fountain_handle_at(&self, p: Point) -> Option<FountainHandle> {
        let (_, f, b) = self.selected_fountain()?;
        let tol = 6.0 / self.view.zoom as f64;
        fountain_handles(&f, b)
            .into_iter()
            .find(|(_, hp)| (*hp - p).hypot() <= tol)
            .map(|(h, _)| h)
    }

    /// Move a fountain handle to `p`: the axis angle for linear fills, the
    /// centre offset (clamped to the bounds) for the others.
    pub fn drag_fountain_handle(&mut self, handle: FountainHandle, p: Point) {
        let Some((id, mut f, b)) = self.selected_fountain() else {
            return;
        };
        let c = b.center();
        match handle {
            FountainHandle::Start | FountainHandle::End => {
                let v = if handle == FountainHandle::End {
                    p - c
                } else {
                    c - p
                };
                if v.hypot() > 1e-6 {
                    f.angle = v.atan2().to_degrees();
                }
            }
            FountainHandle::Center => {
                if b.width() > 1e-9 && b.height() > 1e-9 {
                    f.offset = Point::new(
                        ((p.x - c.x) / (b.width() / 2.0)).clamp(-1.0, 1.0),
                        ((p.y - c.y) / (b.height() / 2.0)).clamp(-1.0, 1.0),
                    );
                }
            }
        }
        self.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Fountain(f),
        });
    }

    /// Apply a fill to the selection, or make it the default when nothing
    /// is selected.
    pub fn apply_fill_or_default(&mut self, fill: Fill) {
        if self.selection.is_empty() {
            self.default_fill = fill;
        } else {
            self.apply_fill(fill);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{document::ShapeKind, Color};

    #[test]
    fn linear_handles_sit_on_the_axis_and_dragging_sets_the_angle() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 40.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        app.select(vec![id]);
        app.apply_fill(Fill::linear(Color::BLACK, Color::WHITE, 0.0));
        let (_, f, b) = app.selected_fountain().expect("fountain");
        let hs = fountain_handles(&f, b);
        assert_eq!(hs[0].0, FountainHandle::Start);
        assert!((hs[0].1.x - 0.0).abs() < 1e-9 && (hs[0].1.y - 10.0).abs() < 1e-9);
        assert!((hs[1].1.x - 40.0).abs() < 1e-9);
        // Drag the end node straight up: the fill runs bottom to top.
        app.drag_fountain_handle(FountainHandle::End, Point::new(20.0, 30.0));
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert!((f.angle - 90.0).abs() < 1e-6, "{}", f.angle);
        assert_eq!(
            app.fountain_handle_at(Point::new(20.0, 20.0)),
            Some(FountainHandle::End)
        );
        assert_eq!(app.fountain_handle_at(Point::new(5.0, 5.0)), None);
    }

    #[test]
    fn radial_centre_handle_sets_the_offset() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 40.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        app.select(vec![id]);
        app.apply_fill(Fill::radial(Color::BLACK, Color::WHITE));
        app.drag_fountain_handle(FountainHandle::Center, Point::new(30.0, 15.0));
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert!((f.offset.x - 0.5).abs() < 1e-9 && (f.offset.y - 0.5).abs() < 1e-9);
        // Past the edge the offset clamps.
        app.drag_fountain_handle(FountainHandle::Center, Point::new(100.0, 15.0));
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert!((f.offset.x - 1.0).abs() < 1e-9);
    }
}
