//! Drop Shadow tool: the drag that makes a flat or perspective shadow,
//! the handles on the canvas (start, end with the shadow colour, and the
//! opacity slider between them), presets and Break Drop Shadow Apart.
//! See docs/behavior/drop-shadow.md.

use crate::app::App;
use crate::i18n::tr;
use tracedraw_core::{
    geometry::{Point, Rect, Vec2},
    live::{Effect, MergeMode},
    shadow::edge_near,
    Color, Command, Fill, Shadow, Shape, ShapeId, ShapeKind,
};

/// Which shadow handle a drag holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowHandle {
    /// Where the shadow starts: the object's centre or the edge it stands on.
    Start,
    /// Where it goes to; colours dropped on it become the shadow colour.
    End,
    /// The opacity slider on the line between them.
    Opacity,
}

/// How far from a handle a press still takes it, screen pixels.
const HANDLE_PX: f64 = 6.0;

/// The resolution Break Drop Shadow Apart makes the bitmap at.
pub const BREAK_APART_DPI: f64 = 150.0;

/// The handles of `sh` on an object with page path bounds `b`, end first
/// (it wins when they overlap).
pub fn shadow_handles(sh: &Shadow, b: Rect) -> [(ShadowHandle, Point); 3] {
    let a = sh.anchor(b);
    let e = sh.end_point(b);
    let t = sh.opacity.clamp(0.0, 1.0);
    let o = a + (e - a) * t;
    [
        (ShadowHandle::End, e),
        (ShadowHandle::Opacity, o),
        (ShadowHandle::Start, a),
    ]
}

/// The bounds the shadow geometry is measured on: the object's path.
pub fn shadow_box(s: &Shape) -> Rect {
    use tracedraw_core::geometry::Shape as _;
    s.page_path().bounding_box()
}

/// `p` (page space) lies on the drop shadow of `s`: inside its
/// silhouette moved or sheared as the shadow is drawn, the feathered
/// fringe left out. Text, groups, tables, bitmaps, clip frames and
/// symbols cast it from anywhere in their box, as they are picked
/// themselves. A shadow with no opacity is not there to pick.
pub fn shadow_covers(s: &Shape, p: Point) -> bool {
    use tracedraw_core::geometry::Shape as _;
    let Some(sh) = s.shadow.filter(|sh| sh.opacity > 0.0) else {
        return false;
    };
    let path = s.page_path();
    let b = path.bounding_box();
    if !(Shadow { blur: 0.0, ..sh }).bounds(b).contains(p) {
        return false;
    }
    let t = sh.silhouette_transform(b);
    let boxed = matches!(
        s.kind,
        ShapeKind::Text { .. }
            | ShapeKind::Group { .. }
            | ShapeKind::Table(_)
            | ShapeKind::Bitmap { .. }
            | ShapeKind::ClipFrame { .. }
            | ShapeKind::SymbolInstance { .. }
    );
    if boxed {
        (t * b.to_path(0.1)).contains(p)
    } else {
        (t * path).contains(p)
    }
}

impl App {
    /// The first selected object's shadow, with its id and box.
    pub fn selected_shadow(&self) -> Option<(ShapeId, Shadow, Rect)> {
        let s = self.selected_shapes().into_iter().next()?;
        let sh = s.shadow?;
        Some((s.id, sh, shadow_box(&s)))
    }

    /// The shadow handle under page point `p`, with the Drop Shadow tool.
    pub fn shadow_handle_at(&self, p: Point) -> Option<ShadowHandle> {
        if self.tool != crate::tools::Tool::DropShadow {
            return None;
        }
        let (_, sh, b) = self.selected_shadow()?;
        let tol = HANDLE_PX / f64::from(self.view.zoom.max(1e-6));
        shadow_handles(&sh, b)
            .into_iter()
            .find(|(_, hp)| (*hp - p).hypot() <= tol)
            .map(|(h, _)| h)
    }

    /// A press at `p` on an object's drop shadow away from the object
    /// itself (Drop Shadow tool): the object becomes the selection, and
    /// what is returned is the way from `p` to the shadow's end, so that
    /// dragging the end by it moves the whole shadow with the pointer
    /// (#935). `None` anywhere else.
    pub fn grab_shadow(&mut self, p: Point) -> Option<Vec2> {
        let id = self.pick_at(p)?;
        if self.hit_test(p) == Some(id) {
            return None;
        }
        self.select(vec![id]);
        let (_, sh, b) = self.selected_shadow()?;
        Some(sh.end_point(b) - p)
    }

    /// A drag on `shape` started at `start` and is now at `p`. The first
    /// frame (`first`) picks a flat shadow or, next to an edge, a
    /// perspective one; later frames move its end with the pointer.
    pub fn drag_new_shadow(&mut self, shape: ShapeId, start: Point, p: Point, first: bool) {
        let Some(s) = self.doc().shape_on_page(shape) else {
            return;
        };
        let b = shadow_box(&s);
        let base = s.shadow.unwrap_or(self.shadow_default);
        let mut sh = if first {
            Shadow::for_drag_start(&base, b, start)
        } else {
            base
        };
        if sh.perspective.is_some() {
            sh.set_end_point(b, p);
        } else {
            sh.offset = p - start;
        }
        self.select(vec![shape]);
        self.set_shadow(Some(sh), true);
    }

    /// Move a shadow handle of the selected shadow to `p`.
    pub fn drag_shadow_handle(&mut self, handle: ShadowHandle, p: Point) {
        let Some((_, mut sh, b)) = self.selected_shadow() else {
            return;
        };
        match handle {
            ShadowHandle::End => sh.set_end_point(b, p),
            ShadowHandle::Opacity => {
                let a = sh.anchor(b);
                let d = sh.end_point(b) - a;
                let len2 = d.hypot2();
                if len2 > 1e-12 {
                    let t = ((p - a).dot(d) / len2).clamp(0.0, 1.0);
                    sh.opacity = (t * 100.0).round() / 100.0;
                }
            }
            ShadowHandle::Start => {
                // Moving the start to another edge stands the shadow there;
                // to the middle makes it flat.
                let end = sh.end_point(b);
                match edge_near(b, p) {
                    Some(e) if sh.perspective.map(|q| q.edge) != Some(e) => {
                        let mut per = sh
                            .perspective
                            .unwrap_or_else(|| tracedraw_core::shadow::Perspective::new(e));
                        per.edge = e;
                        sh.perspective = Some(per);
                        sh.set_end_point(b, end);
                    }
                    None if sh.perspective.is_some() => {
                        sh.perspective = None;
                        sh.set_end_point(b, end);
                    }
                    _ => {}
                }
            }
        }
        self.shadow_default = sh;
        self.set_shadow(Some(sh), true);
    }

    /// A colour dropped on the end handle: the shadow takes it (one step).
    pub fn set_shadow_color(&mut self, shape: ShapeId, color: Color) {
        let Some(mut sh) = self.doc().find_shape(shape).and_then(|s| s.shadow) else {
            return;
        };
        sh.color = color;
        self.run(Command::SetShadow {
            shapes: vec![shape],
            shadow: Some(sh),
        });
    }

    /// Apply built-in preset `id` to the selection's shadows (and make it
    /// the default for new ones), keeping each shadow's colour.
    pub fn apply_shadow_preset(&mut self, id: &str) {
        let Some((_, preset)) = Shadow::presets().into_iter().find(|(k, _)| *k == id) else {
            return;
        };
        let color = self
            .selected_shadow()
            .map(|(_, s, _)| s.color)
            .unwrap_or(self.shadow_default.color);
        let sh = Shadow { color, ..preset };
        self.shadow_default = sh;
        self.set_shadow(Some(sh), false);
    }

    /// True when Break Apart would break a drop shadow off.
    pub fn can_break_shadow_apart(&self) -> bool {
        self.selection.iter().any(|id| {
            self.doc()
                .shape(*id)
                .is_ok_and(|(_, s)| s.shadow.is_some_and(|sh| sh.opacity > 0.0))
        })
    }

    /// Break Drop Shadow Apart: each selected object's shadow becomes a
    /// bitmap of its own just behind the object (merged as the shadow
    /// was), and the object loses its shadow, in one undo step. The new
    /// bitmaps come out selected. Objects inside groups are left alone.
    pub fn break_shadow_apart(&mut self) {
        let mut cmds = Vec::new();
        let mut made = Vec::new();
        for id in self.selection.clone() {
            let Ok((layer, s)) = self.doc().shape(id) else {
                continue;
            };
            let (layer, s) = (layer.id, s.clone());
            if s.shadow.is_none() {
                continue;
            }
            let Some(bitmap) = self.shadow_bitmap(&s) else {
                continue;
            };
            let new_id = bitmap.id;
            cmds.push(Command::AddShape {
                layer,
                shape: bitmap,
            });
            cmds.push(Command::OrderRelative {
                shapes: vec![new_id],
                reference: id,
                in_front: false,
            });
            cmds.push(Command::SetShadow {
                shapes: vec![id],
                shadow: None,
            });
            made.push(new_id);
        }
        if cmds.is_empty() {
            return;
        }
        match self.engine.run_batch("Break Drop Shadow Apart", &cmds) {
            Ok(()) => self.select(made),
            Err(e) => self.status = e.to_string(),
        }
    }

    /// The bitmap object that stands for the shadow of `s`.
    fn shadow_bitmap(&mut self, s: &Shape) -> Option<Shape> {
        let sh = s.shadow?;
        let (rect, pm) = tracedraw_render::render_shadow_image(s, BREAK_APART_DPI)?;
        let (w, h) = (pm.width(), pm.height());
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for p in pm.pixels() {
            let c = p.demultiply();
            rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
        }
        let img = image::RgbaImage::from_raw(w, h, rgba)?;
        let mut png = Vec::new();
        if let Err(e) = image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        {
            self.status =
                crate::i18n::trf("status.could_not_encode_image", &[("e", &e.to_string())]);
            return None;
        }
        let mut shape = Shape::new(
            self.engine.new_shape_id(),
            ShapeKind::Bitmap {
                rect,
                width_px: w,
                height_px: h,
                png: png.into(),
                fx: None,
            },
        );
        shape.fill = Fill::None;
        shape.stroke = None;
        shape.name = Some(tr("shadow.broken_name"));
        if sh.merge != MergeMode::Normal {
            // An opaque transparency that only carries the merge mode.
            shape.effects = vec![Effect::Transparency {
                mask: Fill::Solid(Color::WHITE),
                merge: sh.merge,
                target: 2,
            }];
        }
        Some(shape)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::shadow::ShadowEdge;

    fn app_with_rect() -> (App, ShapeId) {
        let mut app = App::headless();
        app.canvas_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let page = app.page_rect();
        app.view.fit(page, app.canvas_rect);
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(20.0, 20.0, 60.0, 80.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.apply_fill(Fill::Solid(Color::rgb8(0, 0, 255)));
        app.tool = crate::tools::Tool::DropShadow;
        (app, id)
    }

    fn shadow(app: &App, id: ShapeId) -> Option<Shadow> {
        app.doc().find_shape(id).and_then(|s| s.shadow)
    }

    #[test]
    fn a_drag_from_the_bottom_edge_makes_a_perspective_shadow_that_multiplies() {
        let (mut app, id) = app_with_rect();
        let start = Point::new(40.0, 22.0);
        app.drag_new_shadow(id, start, Point::new(41.0, 23.0), true);
        app.drag_new_shadow(id, start, Point::new(90.0, 40.0), false);
        let sh = shadow(&app, id).expect("shadow");
        assert_eq!(sh.merge, MergeMode::Multiply);
        let per = sh.perspective.expect("perspective");
        assert_eq!(per.edge, ShadowEdge::Bottom);
        let b = Rect::new(20.0, 20.0, 60.0, 80.0);
        assert!((sh.end_point(b) - Point::new(90.0, 40.0)).hypot() < 1e-9);
        // One drag, one undo step.
        app.undo();
        assert!(shadow(&app, id).is_none());
        // From the middle: flat, moved by the drag.
        let mid = Point::new(40.0, 50.0);
        app.drag_new_shadow(id, mid, mid, true);
        app.drag_new_shadow(id, mid, Point::new(43.0, 47.0), false);
        let sh = shadow(&app, id).expect("shadow");
        assert!(sh.perspective.is_none());
        assert!((sh.offset.x - 3.0).abs() < 1e-9 && (sh.offset.y + 3.0).abs() < 1e-9);
    }

    /// A rectangle with an ellipse placed inside it: a drag on it with
    /// the Drop Shadow tool gives the clip frame its shadow, which draws
    /// beside the frame with its contents (#873).
    #[test]
    fn a_clip_frame_takes_a_shadow_that_shows() {
        let (mut app, rect) = app_with_rect();
        let ellipse = app
            .new_shape(ShapeKind::Ellipse {
                rect: Rect::new(30.0, 30.0, 50.0, 70.0),
                arc: None,
            })
            .expect("ellipse");
        app.select(vec![ellipse]);
        app.apply_fill(Fill::Solid(Color::rgb8(255, 0, 0)));
        app.place_inside(vec![ellipse], rect);
        app.tool = crate::tools::Tool::DropShadow;
        let mid = Point::new(40.0, 50.0);
        let hit = app.hit_test(mid).expect("the clip frame");
        assert!(matches!(
            app.doc().find_shape(hit).map(|s| &s.kind),
            Some(ShapeKind::ClipFrame { .. })
        ));
        app.drag_new_shadow(hit, mid, mid, true);
        app.drag_new_shadow(hit, mid, Point::new(70.0, 50.0), false);
        let sh = shadow(&app, hit).expect("shadow");
        assert!((sh.offset.x - 30.0).abs() < 1e-9);
        // Drawn: right of the frame, where only its shadow falls.
        let dpi = 25.4;
        let pm = tracedraw_render::render_page_image(app.doc(), app.page, dpi).expect("page");
        let page = app.page_rect();
        let px = |x: f64, y: f64| {
            pm.pixel((x - page.x0) as u32, (page.y1 - y) as u32)
                .expect("pixel")
                .demultiply()
        };
        let under = px(85.0, 50.0);
        assert!(under.red() < 200, "{under:?}");
        let clear = px(120.0, 50.0);
        assert!(clear.red() > 240, "{clear:?}");
    }

    #[test]
    fn handles_move_the_end_set_the_opacity_and_take_dropped_colours() {
        let (mut app, id) = app_with_rect();
        app.set_shadow(Some(Shadow::default()), false);
        let b = Rect::new(20.0, 20.0, 60.0, 80.0);
        let sh = shadow(&app, id).expect("shadow");
        let [(_, end), (_, slider), (_, start)] = shadow_handles(&sh, b);
        assert_eq!(start, b.center());
        assert_eq!(app.shadow_handle_at(end), Some(ShadowHandle::End));
        app.drag_shadow_handle(ShadowHandle::End, Point::new(50.0, 40.0));
        let sh = shadow(&app, id).expect("shadow");
        assert!((sh.offset.x - 10.0).abs() < 1e-9 && (sh.offset.y + 10.0).abs() < 1e-9);
        let _ = slider;
        // The slider three quarters of the way along: 75 %.
        let a = sh.anchor(b);
        let e = sh.end_point(b);
        app.drag_shadow_handle(ShadowHandle::Opacity, a + (e - a) * 0.75);
        assert!((shadow(&app, id).expect("shadow").opacity - 0.75).abs() < 1e-9);
        // The start moved to the top edge stands the shadow there.
        app.drag_shadow_handle(ShadowHandle::Start, Point::new(40.0, 79.0));
        let per = shadow(&app, id)
            .and_then(|s| s.perspective)
            .expect("perspective");
        assert_eq!(per.edge, ShadowEdge::Top);
        // A colour dropped on the end handle colours the shadow.
        let sh = shadow(&app, id).expect("shadow");
        let at = app.view.to_screen(sh.end_point(b));
        let red = Color::rgb8(200, 0, 0);
        assert!(app.drop_color(Some(red), at));
        assert_eq!(shadow(&app, id).map(|s| s.color), Some(red));
        app.undo();
        assert_eq!(shadow(&app, id).map(|s| s.color), Some(Color::BLACK));
    }

    /// A shadow is picked with its object (#935): where it lies over an
    /// object beneath, it wins, as it is drawn over that object; outside
    /// its silhouette (the feathered fringe included) nothing is picked;
    /// a perspective shadow is picked where it falls; and a shadow with
    /// no opacity is not there.
    #[test]
    fn a_shadow_is_picked_with_its_object_above_what_lies_beneath() {
        let (mut app, below) = app_with_rect();
        let top = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(100.0, 20.0, 120.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![top]);
        app.apply_fill(Fill::Solid(Color::rgb8(255, 0, 0)));
        app.set_shadow(
            Some(Shadow {
                offset: Vec2::new(-60.0, 20.0),
                ..Shadow::default()
            }),
            false,
        );
        app.select(Vec::new());
        // The shadow covers 40 to 60 by 40 to 60, over the rectangle below.
        let on_both = Point::new(50.0, 50.0);
        assert_eq!(app.hit_test(on_both), Some(below));
        assert_eq!(app.pick_at(on_both), Some(top));
        // The rectangle below where no shadow is, and the top object itself.
        assert_eq!(app.pick_at(Point::new(30.0, 70.0)), Some(below));
        assert_eq!(app.pick_at(Point::new(110.0, 30.0)), Some(top));
        // Just outside the silhouette, in the feathering: nothing new.
        assert_eq!(app.pick_at(Point::new(50.0, 61.0)), Some(below));
        assert_eq!(app.pick_at(Point::new(80.0, 50.0)), None);
        // A click there with a tool that selects on click.
        app.click_select(on_both, egui::Modifiers::NONE);
        assert_eq!(app.selection, vec![top]);
        // A perspective shadow on the floor, falling to the right.
        let mut sh = Shadow {
            perspective: Some(tracedraw_core::shadow::Perspective::new(ShadowEdge::Bottom)),
            ..Shadow::default()
        };
        let b = Rect::new(100.0, 20.0, 120.0, 40.0);
        sh.set_end_point(b, Point::new(150.0, 30.0));
        app.set_shadow(Some(sh), false);
        app.select(Vec::new());
        let s = app.doc().find_shape(top).expect("top").clone();
        let far = sh.silhouette_transform(b) * Point::new(110.0, 39.0);
        assert!(shadow_covers(&s, far), "{far:?}");
        assert_eq!(app.pick_at(far), Some(top));
        // No opacity: nothing to pick.
        app.select(vec![top]);
        app.set_shadow(Some(Shadow { opacity: 0.0, ..sh }), false);
        let s = app.doc().find_shape(top).expect("top").clone();
        assert!(!shadow_covers(&s, far));
    }

    #[test]
    fn presets_apply_and_keep_the_colour() {
        let (mut app, id) = app_with_rect();
        let green = Color::rgb8(0, 120, 0);
        app.set_shadow(
            Some(Shadow {
                color: green,
                ..Shadow::default()
            }),
            false,
        );
        for (key, p) in Shadow::presets() {
            app.apply_shadow_preset(key);
            let sh = shadow(&app, id).expect("shadow");
            assert_eq!(sh.color, green, "{key}");
            assert_eq!(sh.perspective, p.perspective, "{key}");
            assert_eq!(sh.merge, p.merge, "{key}");
        }
    }

    #[test]
    fn break_apart_makes_a_bitmap_behind_the_object_in_one_step() {
        let (mut app, id) = app_with_rect();
        app.set_shadow(Some(Shadow::default()), false);
        let count = |app: &App| {
            app.doc().pages[0]
                .layers
                .iter()
                .map(|l| l.shapes.len())
                .sum::<usize>()
        };
        let before = count(&app);
        assert!(app.can_break_shadow_apart());
        app.break_apart();
        assert_eq!(count(&app), before + 1);
        assert!(shadow(&app, id).is_none());
        let bitmap_id = app.selection[0];
        let (layer, bmp) = app.doc().shape(bitmap_id).expect("bitmap");
        assert!(matches!(bmp.kind, ShapeKind::Bitmap { .. }));
        // Merged as the shadow was (multiply).
        assert!(matches!(
            bmp.effects.as_slice(),
            [Effect::Transparency {
                merge: MergeMode::Multiply,
                ..
            }]
        ));
        // Just behind the object.
        let ib = layer.shapes.iter().position(|s| s.id == bitmap_id);
        let io = layer.shapes.iter().position(|s| s.id == id);
        assert_eq!(ib.map(|i| i + 1), io);
        // It covers the shadow: right of and below the object.
        let b = bmp.bounds();
        assert!(b.x1 > 60.0 && b.y0 < 20.0, "{b:?}");
        // One undo brings the shadow back and takes the bitmap away.
        app.undo();
        assert_eq!(count(&app), before);
        assert!(shadow(&app, id).is_some());
    }
}
