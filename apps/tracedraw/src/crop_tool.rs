//! The Crop tool. A drag draws the crop area; it stays on the page with
//! eight handles until it is applied (a double-click inside it, Enter, or
//! the bar's Crop button) or cleared (Escape, a click outside it, Clear
//! Crop Area). A drag inside the area moves it, a drag on a handle
//! resizes it, a drag outside it draws a new one.
//!
//! Cropping keeps what lies inside the area, of the selection or (with
//! nothing selected) of every object on the page's unlocked, visible
//! layers. Closed outlines are intersected with the area and open curves
//! are cut where they cross its edge, keeping the pieces inside; text
//! becomes its outlines first. Bitmaps, tables and symbols are put in a
//! clip frame of the area, so their content stays whole. Groups and clip
//! frames are cropped member by member (a clip frame's frame only).
//! Objects wholly inside are left as they are; objects wholly outside are
//! deleted. One undo step.

use crate::app::App;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, BezPath, ParamCurve, PathEl, Point, Rect, Shape as _, Vec2},
    shaping, Command, Fill, ShapeId,
};

/// A drag of the Crop tool under way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CropDrag {
    /// A new area from `start`.
    New { start: Point },
    /// The area moved from where it was.
    Move { start: Point, from: Rect },
    /// Handle `which` (0 to 7: corners and side middles, counterclockwise
    /// from the bottom left) dragged.
    Handle { which: usize, from: Rect },
}

/// What cropping does to one object.
#[derive(Debug, Clone, PartialEq)]
pub enum Cropped {
    Keep,
    Gone,
    Into(Box<Shape>),
}

/// The eight handles of an area: corners and side middles,
/// counterclockwise from the bottom left.
pub fn handles(r: Rect) -> [Point; 8] {
    let c = r.center();
    [
        Point::new(r.x0, r.y0),
        Point::new(c.x, r.y0),
        Point::new(r.x1, r.y0),
        Point::new(r.x1, c.y),
        Point::new(r.x1, r.y1),
        Point::new(c.x, r.y1),
        Point::new(r.x0, r.y1),
        Point::new(r.x0, c.y),
    ]
}

/// `from` with handle `which` moved to `p`.
pub fn resized(from: Rect, which: usize, p: Point) -> Rect {
    let (mut x0, mut y0, mut x1, mut y1) = (from.x0, from.y0, from.x1, from.y1);
    match which {
        0 => (x0, y0) = (p.x, p.y),
        1 => y0 = p.y,
        2 => (x1, y0) = (p.x, p.y),
        3 => x1 = p.x,
        4 => (x1, y1) = (p.x, p.y),
        5 => y1 = p.y,
        6 => (x0, y1) = (p.x, p.y),
        _ => x0 = p.x,
    }
    Rect::new(x0, y0, x1, y1).abs()
}

fn finite_rect(r: Rect) -> bool {
    r.x0.is_finite() && r.y0.is_finite() && r.x1.is_finite() && r.y1.is_finite()
}

/// Is the subpath closed (a closing element, or it ends where it started)?
fn closes(sub: &BezPath, closed: bool) -> bool {
    if closed {
        return true;
    }
    let els = sub.elements();
    let start = match els.first() {
        Some(PathEl::MoveTo(p)) => *p,
        _ => return false,
    };
    let end = match els.last() {
        Some(PathEl::LineTo(p)) | Some(PathEl::QuadTo(_, p)) | Some(PathEl::CurveTo(_, _, p)) => *p,
        _ => return false,
    };
    els.len() > 2 && (end - start).hypot() < 1e-6
}

/// The part of `page` (page space) inside `rect`, and whether any of it is
/// closed.
fn clip_path(page: &BezPath, rect: Rect) -> (BezPath, bool) {
    let mut closed_parts = BezPath::new();
    let mut out = BezPath::new();
    let edge = [
        Point::new(rect.x0, rect.y0),
        Point::new(rect.x1, rect.y0),
        Point::new(rect.x1, rect.y1),
        Point::new(rect.x0, rect.y1),
        Point::new(rect.x0, rect.y0),
    ];
    for (sub, closed) in tracedraw_core::effects::subpaths(page) {
        if closes(&sub, closed) {
            closed_parts.extend(sub.elements().iter().copied());
            if !closed {
                closed_parts.close_path();
            }
            continue;
        }
        let cuts = crate::knife::crossings(&sub, &edge);
        let pieces = if cuts.is_empty() {
            vec![sub]
        } else {
            crate::knife::split_open(&sub, &cuts)
        };
        for piece in pieces {
            // A piece lies wholly inside or outside: its middle tells.
            let segs: Vec<_> = piece.segments().collect();
            let Some(mid) = segs.get(segs.len() / 2).map(|s| s.eval(0.5)) else {
                continue;
            };
            if rect.contains(mid) {
                out.extend(piece.elements().iter().copied());
            }
        }
    }
    let mut any_closed = false;
    if !closed_parts.elements().is_empty() {
        let clipped = shaping::overlay(&closed_parts, &rect.to_path(0.01), shaping::Op::Intersect);
        any_closed = !clipped.elements().is_empty();
        out.extend(clipped.elements().iter().copied());
    }
    (out, any_closed)
}

/// Crop `s` (whose parent maps to the page by `parent`) to `rect` (page
/// space); `new_id` gives ids for the parts it adds.
pub fn crop_shape(
    s: &Shape,
    parent: Affine,
    rect: Rect,
    new_id: &mut dyn FnMut() -> ShapeId,
) -> Cropped {
    let to_page = parent * s.transform;
    match &s.kind {
        ShapeKind::Group { children } => {
            let mut out = Vec::with_capacity(children.len());
            let mut changed = false;
            for c in children {
                match crop_shape(c, to_page, rect, new_id) {
                    Cropped::Keep => out.push(c.clone()),
                    Cropped::Gone => changed = true,
                    Cropped::Into(n) => {
                        changed = true;
                        out.push(*n);
                    }
                }
            }
            if out.is_empty() {
                Cropped::Gone
            } else if !changed {
                Cropped::Keep
            } else {
                let mut g = s.clone();
                g.kind = ShapeKind::Group { children: out };
                Cropped::Into(Box::new(g))
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            // The frame clips the contents: cropping it is enough.
            match crop_shape(frame, to_page, rect, new_id) {
                Cropped::Keep => Cropped::Keep,
                Cropped::Gone => Cropped::Gone,
                Cropped::Into(f) => {
                    let mut c = s.clone();
                    c.kind = ShapeKind::ClipFrame {
                        frame: f,
                        contents: contents.clone(),
                    };
                    Cropped::Into(Box::new(c))
                }
            }
        }
        _ => {
            if to_page.determinant().abs() < 1e-12 {
                return Cropped::Keep;
            }
            let local = s.local_path();
            let page = to_page * local;
            let b = page.bounding_box();
            if !finite_rect(b) {
                return Cropped::Keep;
            }
            if rect.contains(Point::new(b.x0, b.y0)) && rect.contains(Point::new(b.x1, b.y1)) {
                return Cropped::Keep;
            }
            if b.intersect(rect).area() <= 0.0 && (b.width() > 0.0 && b.height() > 0.0) {
                return Cropped::Gone;
            }
            match &s.kind {
                ShapeKind::Bitmap { .. }
                | ShapeKind::Table(_)
                | ShapeKind::SymbolInstance { .. } => {
                    // A clip frame of the area, in the object's own space.
                    let mut frame = Shape::new(
                        s.id,
                        ShapeKind::Path {
                            path: to_page.inverse() * rect.to_path(0.01),
                            closed: true,
                        },
                    );
                    frame.fill = Fill::None;
                    frame.stroke = None;
                    let mut inner = s.clone();
                    inner.id = new_id();
                    inner.transform = Affine::IDENTITY;
                    let mut clip = s.clone();
                    clip.effects.clear();
                    clip.kind = ShapeKind::ClipFrame {
                        frame: Box::new(frame),
                        contents: vec![inner],
                    };
                    Cropped::Into(Box::new(clip))
                }
                _ => {
                    let (clipped, closed) = clip_path(&page, rect);
                    if clipped.elements().is_empty() {
                        return Cropped::Gone;
                    }
                    let mut n = s.clone();
                    n.kind = ShapeKind::Path {
                        path: to_page.inverse() * clipped,
                        closed,
                    };
                    Cropped::Into(Box::new(n))
                }
            }
        }
    }
}

impl App {
    /// Crop to the area and clear it.
    pub fn apply_crop(&mut self) {
        if let Some(r) = self.crop_area.take() {
            self.crop_to(r);
        }
        self.crop_drag = None;
    }

    /// Keep what lies inside `rect` (page space) of the selection, or of
    /// everything on the page's unlocked, visible layers.
    pub fn crop_to(&mut self, rect: Rect) {
        if !finite_rect(rect) || rect.width() < 0.01 || rect.height() < 0.01 {
            return;
        }
        let targets: Vec<Shape> = if self.selection.is_empty() {
            self.doc()
                .page(self.page)
                .map(|p| {
                    p.layers
                        .iter()
                        .filter(|l| !l.locked && l.visible)
                        .flat_map(|l| l.shapes.iter().filter(|s| !s.locked).cloned())
                        .collect()
                })
                .unwrap_or_default()
        } else {
            self.selected_shapes()
        };
        let mut cmds = Vec::new();
        let mut removed = Vec::new();
        for s in targets {
            let parent = self
                .doc()
                .parent_transform(s.id)
                .unwrap_or(Affine::IDENTITY);
            let engine = &mut self.engine;
            let mut new_id = || engine.new_shape_id();
            match crop_shape(&s, parent, rect, &mut new_id) {
                Cropped::Keep => {}
                Cropped::Gone => removed.push(s.id),
                Cropped::Into(n) => cmds.push(Command::SetShapeKind {
                    shape: s.id,
                    kind: n.kind,
                }),
            }
        }
        if !removed.is_empty() {
            cmds.push(Command::DeleteShapes {
                shapes: removed.clone(),
            });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Crop", &cmds) {
            self.status = e.to_string();
            return;
        }
        self.selection.retain(|id| !removed.contains(id));
    }

    /// The handle of the crop area near `p` (page space), if any.
    fn crop_handle_at(&self, p: Point) -> Option<usize> {
        let r = self.crop_area?;
        let tol = 6.0 / self.view.zoom.max(1e-6) as f64;
        handles(r).iter().position(|h| (*h - p).hypot() <= tol)
    }

    /// The Crop tool's clicks and drags.
    pub fn crop_input(&mut self, response: &egui::Response, p: Point) {
        use egui::PointerButton;
        if response.drag_started_by(PointerButton::Primary) {
            let origin = response
                .ctx
                .input(|i| i.pointer.press_origin())
                .map(|o| self.view.to_page(o))
                .unwrap_or(p);
            self.crop_drag = Some(match (self.crop_handle_at(origin), self.crop_area) {
                (Some(which), Some(from)) => CropDrag::Handle { which, from },
                (None, Some(from)) if from.contains(origin) => CropDrag::Move {
                    start: origin,
                    from,
                },
                _ => CropDrag::New {
                    start: self.snap_point(origin),
                },
            });
        }
        if response.dragged_by(PointerButton::Primary) {
            self.crop_drag_to(p);
        }
        if response.drag_stopped() {
            self.crop_drag = None;
            if self
                .crop_area
                .is_some_and(|r| r.width() < 0.1 || r.height() < 0.1)
            {
                self.crop_area = None;
            }
        }
        if response.double_clicked_by(PointerButton::Primary) {
            if self.crop_area.is_some_and(|r| r.contains(p)) {
                self.apply_crop();
            }
        } else if response.clicked_by(PointerButton::Primary)
            && self.crop_area.is_some_and(|r| !r.contains(p))
        {
            self.crop_area = None;
        }
    }

    /// The pointer moved to `p` (page space) during a drag.
    pub fn crop_drag_to(&mut self, p: Point) {
        let Some(d) = self.crop_drag else {
            return;
        };
        let r = match d {
            CropDrag::New { start } => Rect::from_points(start, self.snap_point(p)),
            CropDrag::Move { start, from } => from + (p - start),
            CropDrag::Handle { which, from } => resized(from, which, self.snap_point(p)),
        };
        if finite_rect(r) {
            self.crop_area = Some(r);
        }
    }
}

/// `r` moved by `v`, for the bar's fields.
pub fn moved(r: Rect, v: Vec2) -> Rect {
    r + v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;
    use tracedraw_core::Color;

    fn rect_shape(app: &mut App, r: Rect) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: r,
                radius: 0.0,
                corners: None,
            })
            .expect("rectangle");
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(Color::BLACK),
        });
        id
    }

    /// #1491: cropping an object with a gradient transparency keeps the
    /// part of the transparency over what is left, as cropping a picture
    /// does: the ramp is cut, not started again across the new box.
    #[test]
    fn crop_cuts_the_gradient_transparency() {
        use tracedraw_core::live::Effect;
        use tracedraw_core::style::{Fountain, FountainKind};
        let mut app = App::headless();
        let id = rect_shape(&mut app, Rect::new(0.0, 0.0, 100.0, 50.0));
        let ramp = Fill::Fountain(Fountain::two(
            FountainKind::Linear,
            Color::BLACK,
            Color::WHITE,
            0.0,
        ));
        app.run(Command::SetEffects {
            shape: id,
            effects: vec![Effect::Transparency {
                mask: ramp,
                merge: Default::default(),
                target: 2,
            }],
        });
        app.select(vec![id]);
        app.crop_to(Rect::new(60.0, 10.0, 90.0, 40.0));
        let s = app.doc().find_shape(id).cloned().expect("cropped");
        let b = s.bounds();
        assert!(
            (b.x0 - 60.0).abs() < 1e-6 && (b.x1 - 90.0).abs() < 1e-6,
            "{b:?}"
        );
        let Some(Effect::Transparency {
            mask: Fill::Fountain(f),
            ..
        }) = s.effects.first()
        else {
            panic!("a gradient transparency");
        };
        // Where the ramp was 0.6 and 0.9 of the way, it still is.
        for x in [60.0, 75.0, 90.0] {
            let t = f.param_at(b, Point::new(x, 25.0)).expect("placed");
            assert!((t - x / 100.0).abs() < 1e-9, "{x}: {t}");
        }
    }

    #[test]
    fn the_area_waits_and_moves_until_it_is_applied() {
        let mut app = App::headless();
        app.tool = Tool::Crop;
        let id = rect_shape(&mut app, Rect::new(0.0, 0.0, 40.0, 40.0));
        // A drag draws the area; nothing is cropped yet.
        app.crop_drag = Some(CropDrag::New {
            start: Point::new(10.0, 10.0),
        });
        app.crop_drag_to(Point::new(30.0, 30.0));
        app.crop_drag = None;
        assert_eq!(app.crop_area, Some(Rect::new(10.0, 10.0, 30.0, 30.0)));
        assert!(matches!(
            app.doc().find_shape(id).map(|s| &s.kind),
            Some(ShapeKind::Rect { .. })
        ));
        // Moved, then a handle dragged.
        let from = app.crop_area.expect("area");
        app.crop_drag = Some(CropDrag::Move {
            start: Point::new(20.0, 20.0),
            from,
        });
        app.crop_drag_to(Point::new(25.0, 20.0));
        app.crop_drag = Some(CropDrag::Handle {
            which: 4,
            from: app.crop_area.expect("area"),
        });
        app.crop_drag_to(Point::new(50.0, 32.0));
        assert_eq!(app.crop_area, Some(Rect::new(15.0, 10.0, 50.0, 32.0)));
        assert_eq!(
            handles(Rect::new(0.0, 0.0, 2.0, 2.0))[3],
            Point::new(2.0, 1.0)
        );
        // Applied: the square is cut to the area, one undo step.
        let steps = app.engine.history_labels().0.len();
        app.apply_crop();
        assert!(app.crop_area.is_none());
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        let b = app.doc().find_shape(id).expect("square").bounds();
        assert!(
            (b.x0 - 15.0).abs() < 1e-6 && (b.x1 - 40.0).abs() < 1e-6,
            "{b:?}"
        );
        assert!(
            (b.y0 - 10.0).abs() < 1e-6 && (b.y1 - 32.0).abs() < 1e-6,
            "{b:?}"
        );
    }

    #[test]
    fn open_curves_groups_bitmaps_and_text() {
        let mut app = App::headless();
        // An open line across the area: the part inside stays, open.
        let mut line = BezPath::new();
        line.move_to((-10.0, 5.0));
        line.line_to((30.0, 5.0));
        let lid = app
            .new_shape(ShapeKind::Path {
                path: line,
                closed: false,
            })
            .expect("line");
        // A group of a square inside and one outside the area.
        let a = rect_shape(&mut app, Rect::new(1.0, 1.0, 4.0, 4.0));
        let b = rect_shape(&mut app, Rect::new(50.0, 50.0, 60.0, 60.0));
        app.run(Command::Group { shapes: vec![a, b] });
        let gid = app
            .doc()
            .all_layers()
            .flat_map(|l| &l.shapes)
            .find(|s| matches!(s.kind, ShapeKind::Group { .. }))
            .map(|s| s.id)
            .expect("group");
        // A bitmap half in the area.
        let bid = app
            .new_shape(ShapeKind::Bitmap {
                rect: Rect::new(10.0, -10.0, 30.0, 10.0),
                width_px: 2,
                height_px: 2,
                png: Vec::new().into(),
                fx: None,
            })
            .expect("bitmap");
        app.crop_to(Rect::new(0.0, 0.0, 20.0, 20.0));
        let s = app.doc().find_shape(lid).expect("line");
        let ShapeKind::Path { path, closed } = &s.kind else {
            panic!("a path");
        };
        assert!(!closed);
        let lb = path.bounding_box();
        assert!(
            (lb.x0 - 0.0).abs() < 1e-6 && (lb.x1 - 20.0).abs() < 1e-6,
            "{lb:?}"
        );
        // The group keeps the square inside, as it was.
        let Some(ShapeKind::Group { children }) = app.doc().find_shape(gid).map(|s| s.kind.clone())
        else {
            panic!("the group");
        };
        assert_eq!(children.len(), 1);
        assert!(matches!(children[0].kind, ShapeKind::Rect { .. }));
        // The bitmap is in a clip frame of the area, whole.
        let Some(ShapeKind::ClipFrame { frame, contents }) =
            app.doc().find_shape(bid).map(|s| s.kind.clone())
        else {
            panic!("a clip frame");
        };
        assert!(matches!(contents[0].kind, ShapeKind::Bitmap { .. }));
        let fb = frame.local_path().bounding_box();
        assert!(
            (fb.x1 - 20.0).abs() < 1e-6 && (fb.y0 - 0.0).abs() < 1e-6,
            "{fb:?}"
        );
        // Text half in the area becomes its outlines, cut (when a font
        // gives outlines here).
        let tid = app
            .new_shape(ShapeKind::Text {
                spans: vec![tracedraw_core::TextSpan::new(
                    "Crop me".to_string(),
                    app.text_font.clone(),
                    24.0,
                )],
                origin: Point::new(10.0, 5.0),
                frame: None,
                align: tracedraw_core::TextAlign::Left,
                para: Default::default(),
                on_path: None,
            })
            .expect("text");
        let outlines = app
            .doc()
            .find_shape(tid)
            .map(|s| !s.local_path().elements().is_empty())
            .unwrap_or(false);
        app.select(vec![tid]);
        app.crop_to(Rect::new(0.0, 0.0, 20.0, 20.0));
        if outlines {
            let t = app.doc().find_shape(tid).expect("cropped text");
            assert!(matches!(t.kind, ShapeKind::Path { closed: true, .. }));
            let tb = t.bounds();
            assert!(tb.x1 <= 20.0 + 1e-6, "{tb:?}");
        }
        // Wholly outside: deleted.
        let far = rect_shape(&mut app, Rect::new(100.0, 100.0, 110.0, 110.0));
        app.select(vec![far]);
        app.crop_to(Rect::new(0.0, 0.0, 20.0, 20.0));
        assert!(app.doc().find_shape(far).is_none());
        assert!(app.selection.is_empty());
    }

    #[test]
    fn handles_and_resizing_stay_sane() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        // Dragging a corner past the opposite one flips the area round.
        assert_eq!(
            resized(r, 0, Point::new(20.0, 15.0)),
            Rect::new(10.0, 10.0, 20.0, 15.0)
        );
        assert_eq!(
            moved(r, Vec2::new(1.0, 2.0)),
            Rect::new(1.0, 2.0, 11.0, 12.0)
        );
        // Odd areas crop nothing.
        let mut app = App::headless();
        let id = rect_shape(&mut app, r);
        app.crop_to(Rect::new(f64::NAN, 0.0, 1.0, 1.0));
        app.crop_to(Rect::new(0.0, 0.0, 0.0, 5.0));
        assert!(matches!(
            app.doc().find_shape(id).map(|s| &s.kind),
            Some(ShapeKind::Rect { .. })
        ));
    }
}
