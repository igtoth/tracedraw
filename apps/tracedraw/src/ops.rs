//! Higher-level editing operations built from commands: align, distribute,
//! transformations, lock, combine, bitmap import.

use crate::app::App;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, Point, Rect, Vec2},
    Command,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
    Top,
    Bottom,
    CenterH,
    CenterV,
    CenterPage,
}

/// What Distribute spreads evenly: the left edges, centres or right
/// edges, or the gaps, along x; the same along y (Top and Bottom are the
/// top and bottom edges).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distribute {
    Left,
    CentersH,
    Right,
    SpacingH,
    Top,
    CentersV,
    Bottom,
    SpacingV,
}

/// Where Distribute spreads the objects: between the outermost ones, or
/// across the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DistributeTo {
    #[default]
    Selection,
    Page,
}

impl DistributeTo {
    /// The name kept in the preferences.
    pub fn id(self) -> &'static str {
        match self {
            DistributeTo::Selection => "selection",
            DistributeTo::Page => "page",
        }
    }

    /// The extent a name stands for (the selection for an unknown name).
    pub fn from_id(id: &str) -> DistributeTo {
        match id {
            "page" => DistributeTo::Page,
            _ => DistributeTo::Selection,
        }
    }
}

/// The nearest multiple of `step` from `origin` to `v`.
fn nearest_line(v: f64, origin: f64, step: f64) -> f64 {
    if !(step.is_finite() && step > 1e-9) {
        return v;
    }
    origin + ((v - origin) / step).round() * step
}

impl App {
    /// The box Align and Distribute use for `s`: its bounds, grown by
    /// half its outline width with Use Outline.
    fn align_box(&self, s: &Shape) -> Rect {
        let b = self.visual_bounds(s);
        match (&s.stroke, self.align_use_outline) {
            (Some(st), true) if st.width.is_finite() && st.width > 0.0 => {
                b.inflate(st.width / 2.0, st.width / 2.0)
            }
            _ => b,
        }
    }

    /// Align to the last-selected object; with one object, to the page
    /// (the menu, the L, R, T, B, E, C and P keys and scripts).
    pub fn align(&mut self, how: Align) {
        // With the Shape tool and nodes selected, the nodes align.
        if self.align_nodes_to(how) {
            return;
        }
        self.align_with(how, crate::ops2::AlignTo::ActiveObject);
    }

    /// Align the selected objects: to the last-selected object (to the
    /// page when there is only one), the page's edges, its centre, or each
    /// to the nearest grid line. Center to Page always takes the page
    /// centre.
    pub fn align_with(&mut self, how: Align, to: crate::ops2::AlignTo) {
        use crate::ops2::AlignTo;
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            return;
        }
        let page = self.page_rect();
        let c = page.center();
        let target: Rect = match to {
            _ if how == Align::CenterPage => page,
            AlignTo::ActiveObject if shapes.len() > 1 => {
                shapes.last().map(|s| self.align_box(s)).unwrap_or(page)
            }
            AlignTo::ActiveObject | AlignTo::PageEdge | AlignTo::Grid => page,
            AlignTo::PageCenter => Rect::new(c.x, c.y, c.x, c.y),
        };
        let grid = self.doc().metadata.grid;
        let rulers = self.doc().metadata.rulers.clone();
        let on_grid = to == AlignTo::Grid && how != Align::CenterPage;
        let gx = |v: f64| nearest_line(v, rulers.origin_x, grid.spacing_x);
        let gy = |v: f64| nearest_line(v, rulers.origin_y, grid.spacing_y);
        let mut cmds = Vec::new();
        for s in &shapes {
            let b = self.align_box(s);
            let d = if on_grid {
                match how {
                    Align::Left => Vec2::new(gx(b.x0) - b.x0, 0.0),
                    Align::Right => Vec2::new(gx(b.x1) - b.x1, 0.0),
                    Align::CenterV => Vec2::new(gx(b.center().x) - b.center().x, 0.0),
                    Align::Top => Vec2::new(0.0, gy(b.y1) - b.y1),
                    Align::Bottom => Vec2::new(0.0, gy(b.y0) - b.y0),
                    Align::CenterH => Vec2::new(0.0, gy(b.center().y) - b.center().y),
                    Align::CenterPage => Vec2::ZERO,
                }
            } else {
                match how {
                    Align::Left => Vec2::new(target.x0 - b.x0, 0.0),
                    Align::Right => Vec2::new(target.x1 - b.x1, 0.0),
                    Align::Top => Vec2::new(0.0, target.y1 - b.y1),
                    Align::Bottom => Vec2::new(0.0, target.y0 - b.y0),
                    Align::CenterH => Vec2::new(0.0, target.center().y - b.center().y),
                    Align::CenterV => Vec2::new(target.center().x - b.center().x, 0.0),
                    Align::CenterPage => target.center() - b.center(),
                }
            };
            if d.hypot() > 1e-9 {
                cmds.push(Command::TransformShapes {
                    shapes: vec![s.id],
                    transform: Affine::translate(d),
                });
            }
        }
        if !cmds.is_empty() {
            let _ = self.run_batch_keeping_lines("Align", &cmds);
        }
    }

    /// Distribute between the outermost selected objects (the menu).
    pub fn distribute(&mut self, how: Distribute) {
        self.distribute_with(how, DistributeTo::Selection);
    }

    /// How many objects `to` needs: three between the outermost ones, two
    /// across the page.
    pub fn can_distribute(&self, to: DistributeTo) -> bool {
        let need = match to {
            DistributeTo::Selection => 3,
            DistributeTo::Page => 2,
        };
        self.selection.len() >= need
    }

    /// Spread the selected objects evenly along x or y: their left edges,
    /// centres or right edges (top, centres, bottom), or the gaps between
    /// them. Between the outermost objects they stay put; across the page
    /// the first and last touch its edges.
    pub fn distribute_with(&mut self, how: Distribute, to: DistributeTo) {
        if !self.can_distribute(to) {
            return;
        }
        let mut shapes = self.selected_shapes();
        let horizontal = matches!(
            how,
            Distribute::Left | Distribute::CentersH | Distribute::Right | Distribute::SpacingH
        );
        // Along the axis: the low and high edge of a box.
        let lo = |b: &Rect| if horizontal { b.x0 } else { b.y0 };
        let hi = |b: &Rect| if horizontal { b.x1 } else { b.y1 };
        // 0: low edges, 1: centres, 2: high edges, 3: gaps. Top is the
        // high edge along y (y grows upwards).
        let kind = match how {
            Distribute::Left | Distribute::Bottom => 0,
            Distribute::CentersH | Distribute::CentersV => 1,
            Distribute::Right | Distribute::Top => 2,
            Distribute::SpacingH | Distribute::SpacingV => 3,
        };
        let key = |b: &Rect| match kind {
            0 => lo(b),
            2 => hi(b),
            _ => (lo(b) + hi(b)) / 2.0,
        };
        let mut boxes: Vec<(Shape, Rect)> = shapes
            .drain(..)
            .map(|s| {
                let b = self.align_box(&s);
                (s, b)
            })
            .collect();
        boxes.sort_by(|a, b| {
            key(&a.1)
                .partial_cmp(&key(&b.1))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let n = boxes.len();
        let (first, last) = (boxes[0].1, boxes[n - 1].1);
        let page = self.page_rect();
        let (e0, e1) = (lo(&page), hi(&page));
        let mut cmds = Vec::new();
        let mut shift = |s: &Shape, d: f64| {
            if d.abs() > 1e-9 {
                let v = if horizontal {
                    Vec2::new(d, 0.0)
                } else {
                    Vec2::new(0.0, d)
                };
                cmds.push(Command::TransformShapes {
                    shapes: vec![s.id],
                    transform: Affine::translate(v),
                });
            }
        };
        if kind == 3 {
            let total: f64 = boxes.iter().map(|(_, b)| hi(b) - lo(b)).sum();
            let (start, end) = match to {
                DistributeTo::Selection => (lo(&first), hi(&last)),
                DistributeTo::Page => (e0, e1),
            };
            let gap = (end - start - total) / (n - 1) as f64;
            let mut cursor = start;
            for (s, b) in &boxes {
                shift(s, cursor - lo(b));
                cursor += hi(b) - lo(b) + gap;
            }
        } else {
            let len = |b: &Rect| hi(b) - lo(b);
            let (k0, k1) = match to {
                DistributeTo::Selection => (key(&first), key(&last)),
                DistributeTo::Page => match kind {
                    0 => (e0, e1 - len(&last)),
                    1 => (e0 + len(&first) / 2.0, e1 - len(&last) / 2.0),
                    _ => (e0 + len(&first), e1),
                },
            };
            let step = (k1 - k0) / (n - 1) as f64;
            for (i, (s, b)) in boxes.iter().enumerate() {
                shift(s, k0 + step * i as f64 - key(b));
            }
        }
        if !cmds.is_empty() {
            let _ = self.run_batch_keeping_lines("Distribute", &cmds);
        }
    }

    /// Copies of the selection right on top of it, which become the
    /// selection (a transform that leaves the originals).
    pub(crate) fn duplicate_in_place(&mut self) {
        let saved = self.duplicate_offset;
        self.duplicate_offset = Vec2::ZERO;
        // Copies in place are not a Duplicate the next one repeats (#559).
        self.last_duplicate.clear();
        self.duplicate();
        self.last_duplicate.clear();
        self.duplicate_offset = saved;
    }

    pub fn set_locked(&mut self, locked: bool) {
        if self.selection.is_empty() {
            return;
        }
        let shapes = self.selection.clone();
        self.run(Command::SetLocked { shapes, locked });
        if locked {
            self.selection.clear();
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        if self.selection.is_empty() {
            return;
        }
        let shapes = self.selection.clone();
        self.run(Command::SetVisible { shapes, visible });
        if !visible {
            self.selection.clear();
        }
    }

    pub fn show_all(&mut self) {
        let ids: Vec<_> = self
            .doc()
            .page(self.page)
            .map(|p| {
                p.layers
                    .iter()
                    .flat_map(|l| &l.shapes)
                    .filter(|s| !s.visible)
                    .map(|s| s.id)
                    .collect()
            })
            .unwrap_or_default();
        if !ids.is_empty() {
            self.run(Command::SetVisible {
                shapes: ids,
                visible: true,
            });
        }
    }

    /// Unlock object `id` and select it.
    pub fn unlock_object(&mut self, id: tracedraw_core::ShapeId) {
        self.run(Command::SetLocked {
            shapes: vec![id],
            locked: false,
        });
        if self.doc().find_shape(id).is_some_and(|s| !s.locked) {
            self.select(vec![id]);
        }
    }

    pub fn unlock_all(&mut self) {
        let ids: Vec<_> = self
            .doc()
            .page(self.page)
            .map(|p| {
                p.layers
                    .iter()
                    .flat_map(|l| &l.shapes)
                    .filter(|s| s.locked)
                    .map(|s| s.id)
                    .collect()
            })
            .unwrap_or_default();
        if !ids.is_empty() {
            self.run(Command::SetLocked {
                shapes: ids,
                locked: false,
            });
        }
    }

    pub fn combine(&mut self) {
        if self.selection.len() < 2 {
            return;
        }
        let shapes = self.selection.clone();
        // Make everything a curve first so Combine has paths to merge.
        self.convert_to_curves();
        let keep = shapes.last().copied();
        self.run(Command::Combine { shapes });
        self.selection = keep.into_iter().collect();
    }

    /// The menu label of the group Break Apart breaks first when a
    /// selected object carries a contour, a blend or an extrusion (Break
    /// Contour Group Apart and the like), else `None`.
    pub fn effect_group_to_break(&self) -> Option<&'static str> {
        use tracedraw_core::live::Effect;
        self.selection.iter().find_map(|id| {
            let (_, s) = self.doc().shape(*id).ok()?;
            s.effects.iter().find_map(|e| match e {
                Effect::Contour { .. } => Some("menu.object.break_contour_apart"),
                Effect::Blend { .. } => Some("menu.object.break_blend_apart"),
                Effect::Extrude { .. } => Some("menu.object.break_extrude_apart"),
                _ => None,
            })
        })
    }

    /// The label and shortcut state of Break Apart for the menus: the
    /// drop shadow's, an effect group's, else plain Break Apart.
    pub fn break_apart_label(&self) -> &'static str {
        if self.can_break_shadow_apart() {
            "menu.object.break_shadow_apart"
        } else {
            self.effect_group_to_break()
                .unwrap_or("menu.object.break_apart")
        }
    }

    /// Break Apart: every subpath becomes an object; the pieces come out
    /// selected.
    pub fn break_apart(&mut self) {
        // A drop shadow breaks off its object first (Break Drop Shadow
        // Apart).
        if self.can_break_shadow_apart() {
            self.break_shadow_apart();
            return;
        }
        // A contour, blend or extrude group: its steps become objects of
        // their own beside the object (Break Contour Group Apart, #424).
        if self.effect_group_to_break().is_some() {
            self.break_effect_groups();
            return;
        }
        let ids = self.selection.clone();
        let before: std::collections::HashSet<tracedraw_core::ShapeId> = self
            .doc()
            .pages
            .iter()
            .flat_map(|p| &p.layers)
            .flat_map(|l| &l.shapes)
            .map(|s| s.id)
            .collect();
        // Artistic text breaks into its lines, a single line into its
        // words, a single word into its characters; curves into their
        // subpaths. One undo step for the whole selection.
        let mut cmds = Vec::new();
        // Top of the stack first: an object's pieces go right above it, so
        // the places read for the objects below it stay true.
        let mut order = ids.clone();
        order.sort_by_key(|id| std::cmp::Reverse(self.doc().locate(*id).map_or(0, |(_, i)| i)));
        for id in &order {
            // A dimension frees its lines, arrowheads and label (#644).
            if let Some(mut c) = self.dimension_break_commands(*id) {
                cmds.append(&mut c);
                continue;
            }
            // An envelope, perspective or distortion first becomes part of
            // the curves (#647).
            if let Some(mut c) = self.warp_break_commands(*id) {
                cmds.append(&mut c);
                continue;
            }
            match self.text_break_commands(*id) {
                Some(mut c) => cmds.append(&mut c),
                None => cmds.push(Command::BreakApart { shape: *id }),
            }
        }
        if let Err(e) = self.engine.run_batch("Break Apart", &cmds) {
            self.status = e.to_string();
        }
        let mut pieces: Vec<tracedraw_core::ShapeId> = self
            .doc()
            .pages
            .iter()
            .flat_map(|p| &p.layers)
            .flat_map(|l| &l.shapes)
            .map(|s| s.id)
            .filter(|id| !before.contains(id) || ids.contains(id))
            .collect();
        pieces.sort_by_key(|id| id.0);
        self.select(pieces);
    }

    /// The commands that break apart `id` when it has an outline warp a bake
    /// takes (`None` otherwise): the warp made part of its curves (#647),
    /// then a text in one curve per letter (each warped as the whole text
    /// is, holes kept), a curve in its parts. The first piece keeps the
    /// object; the others go right above it with its look and other
    /// effects. Groups, clip frames, tables and symbols have nothing to
    /// break and keep their warps (Convert to Curves bakes them), and so
    /// does a text inside a group.
    pub(crate) fn warp_break_commands(
        &mut self,
        id: tracedraw_core::ShapeId,
    ) -> Option<Vec<Command>> {
        let doc = self.doc();
        let shape = doc.find_shape(id)?.clone();
        if matches!(
            shape.kind,
            ShapeKind::Group { .. }
                | ShapeKind::ClipFrame { .. }
                | ShapeKind::Table(_)
                | ShapeKind::SymbolInstance { .. }
        ) {
            return None;
        }
        let place = doc.locate(id).ok();
        let letters = tracedraw_core::live::warped_letters(&shape);
        if letters.is_some() && place.is_none() {
            return None;
        }
        let baked = tracedraw_core::live::bake_outline_warps(&shape)?;
        let closed = |p: &tracedraw_core::geometry::BezPath| {
            p.elements()
                .iter()
                .any(|e| matches!(e, tracedraw_core::geometry::PathEl::ClosePath))
        };
        let mut cmds = vec![Command::SetEffectStack {
            shape: id,
            effects: baked.effects.clone(),
            parked: baked.parked_effects.clone(),
        }];
        match (place, letters) {
            (Some((layer, index)), Some(letters)) if !letters.is_empty() => {
                for (k, path) in letters.into_iter().enumerate() {
                    let kind = ShapeKind::Path {
                        closed: closed(&path),
                        path,
                    };
                    if k == 0 {
                        cmds.push(Command::SetShapeKind { shape: id, kind });
                    } else {
                        let mut s = baked.clone();
                        s.id = self.engine.new_shape_id();
                        s.uid = None;
                        s.kind = kind;
                        let new_id = s.id;
                        cmds.push(Command::AddShape { layer, shape: s });
                        cmds.push(Command::Reorder {
                            shape: new_id,
                            layer,
                            index: index + k,
                        });
                    }
                }
            }
            _ => {
                cmds.push(Command::SetShapeKind {
                    shape: id,
                    kind: baked.kind,
                });
                cmds.push(Command::BreakApart { shape: id });
            }
        }
        Some(cmds)
    }

    /// The commands that break the artistic text `id` apart (`None` when
    /// it is not artistic text, or holds a single character): each piece
    /// a text object of its own with the same look, placed where its
    /// letters were. The first piece keeps the object (its id, place in
    /// the stacking order and properties); the others go right above it.
    /// Whether Break Apart does something to `shape` beyond a curve's
    /// subpaths: an outline warp to bake into curves (#647), or artistic
    /// text of more than one line, word or character.
    pub(crate) fn breaks_apart(shape: &Shape) -> bool {
        if shape.has_outline_warp() {
            return true;
        }
        match &shape.kind {
            ShapeKind::Text {
                spans,
                frame: None,
                on_path: None,
                ..
            } => text_pieces(&tracedraw_core::document::spans_text(spans)).len() > 1,
            _ => false,
        }
    }

    pub(crate) fn text_break_commands(
        &mut self,
        id: tracedraw_core::ShapeId,
    ) -> Option<Vec<Command>> {
        let doc = self.doc();
        let (layer, index) = doc.locate(id).ok()?;
        let shape = doc.find_shape(id)?.clone();
        let ShapeKind::Text {
            spans,
            origin,
            frame: None,
            para,
            on_path: None,
            ..
        } = &shape.kind
        else {
            return None;
        };
        let pieces = text_pieces(&tracedraw_core::document::spans_text(spans));
        if pieces.len() < 2 {
            return None;
        }
        let (layout, _) = crate::text_editing::layout_of_text_shape(
            &shape,
            tracedraw_core::geometry::Affine::IDENTITY,
        )?;
        let mut cmds = Vec::new();
        for (k, (a, b)) in pieces.iter().enumerate() {
            let (head, _) = tracedraw_core::document::split_spans_at(spans, *b);
            let (_, piece) = tracedraw_core::document::split_spans_at(&head, *a);
            let (x, baseline, _, _) = layout.caret(*a)?;
            // Where the piece starts in the text's own space: the written
            // line's place turned into a column for vertical text, so its
            // pieces go down the column (and its lines to the left)
            // instead of side by side (#1303).
            let at = layout.line_frame() * tracedraw_core::geometry::Point::new(x, baseline);
            let kind = ShapeKind::Text {
                spans: piece,
                origin: tracedraw_core::geometry::Point::new(origin.x + at.x, origin.y + at.y),
                frame: None,
                align: tracedraw_core::document::TextAlign::Left,
                para: para.clone(),
                on_path: None,
            };
            if k == 0 {
                cmds.push(Command::SetShapeKind { shape: id, kind });
            } else {
                let mut s = shape.clone();
                s.id = self.engine.new_shape_id();
                s.uid = None;
                s.kind = kind;
                let new_id = s.id;
                cmds.push(Command::AddShape { layer, shape: s });
                cmds.push(Command::Reorder {
                    shape: new_id,
                    layer,
                    index: index + k,
                });
            }
        }
        Some(cmds)
    }

    /// Import a raster image as a bitmap object at the page centre (96 dpi).
    pub fn import_bitmap(&mut self, path: &std::path::Path) {
        let bytes = match crate::files::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.status = crate::i18n::trf(
                    "status.could_not_read",
                    &[("p", &path.display().to_string()), ("e", &e.to_string())],
                );
                return;
            }
        };
        let img = match image::load_from_memory(&bytes) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                self.status = crate::i18n::trf(
                    "status.could_not_read",
                    &[("p", &path.display().to_string()), ("e", &e.to_string())],
                );
                return;
            }
        };
        let name = path.file_name().and_then(|n| n.to_str()).map(String::from);
        // At the resolution the file records, else 96 dpi.
        let dpi = crate::image_dpi::resolution(&bytes);
        if let Some(id) = self.place_image_at(img, name, dpi) {
            // Remember where it came from: Bitmaps > Update from Link re-reads it.
            self.run(Command::SetObjectData {
                shape: id,
                data: vec![("bitmap.link".into(), path.display().to_string())],
            });
            self.select(vec![id]);
        }
    }

    /// Double-clicking the Rectangle tool: a rectangle exactly the size of
    /// the page (a page frame), selected.
    pub fn add_page_frame(&mut self) {
        let size = self.page_size();
        if let Some(id) = self.new_shape(tracedraw_core::ShapeKind::Rect {
            rect: tracedraw_core::geometry::Rect::new(0.0, 0.0, size.width, size.height),
            radius: 0.0,
            corners: None,
        }) {
            self.select(vec![id]);
        }
    }

    /// Shift + double-click on the Rectangle tool: a rectangle around the
    /// selection as it is drawn (its effects included), selected; with
    /// nothing selected, the page frame.
    pub fn add_selection_frame(&mut self) {
        let Some(b) = self.selection_bounds().filter(|b| {
            b.width().is_finite() && b.height().is_finite() && b.width() > 0.0 && b.height() > 0.0
        }) else {
            self.add_page_frame();
            return;
        };
        if let Some(id) = self.new_shape(tracedraw_core::ShapeKind::Rect {
            rect: b,
            radius: 0.0,
            corners: None,
        }) {
            self.select(vec![id]);
        }
    }

    /// Layout > Page Background > Bitmap: an image file as the page's
    /// background, scaled to cover the page.
    pub fn set_page_background_image(
        &mut self,
        page: tracedraw_core::PageId,
        path: &std::path::Path,
    ) {
        let bytes = match crate::files::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.status = crate::i18n::trf(
                    "status.could_not_read",
                    &[("p", &path.display().to_string()), ("e", &e.to_string())],
                );
                return;
            }
        };
        let img = match image::load_from_memory(&bytes) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                self.status = crate::i18n::trf(
                    "status.could_not_read",
                    &[("p", &path.display().to_string()), ("e", &e.to_string())],
                );
                return;
            }
        };
        let Some(png) = crate::bitmap_fx::encode(&img) else {
            return;
        };
        let size = self.page_size();
        self.run(Command::SetPageBackground {
            page,
            background: Some(tracedraw_core::Fill::Pattern(
                tracedraw_core::Pattern::Bitmap {
                    png: png.into(),
                    width_px: img.width(),
                    height_px: img.height(),
                    size_mm: size.width.max(size.height),
                },
            )),
        });
    }

    /// Place an RGBA image at the page centre at 96 dpi; returns the new shape.
    pub fn place_image(
        &mut self,
        img: image::RgbaImage,
        name: Option<String>,
    ) -> Option<tracedraw_core::ShapeId> {
        self.place_image_at(img, name, None)
    }

    /// Place an RGBA image at the page centre at `dpi` (horizontal,
    /// vertical), or 96 dpi when `None`; returns the new shape.
    pub fn place_image_at(
        &mut self,
        img: image::RgbaImage,
        name: Option<String>,
        dpi: Option<(f64, f64)>,
    ) -> Option<tracedraw_core::ShapeId> {
        let (dpi_x, dpi_y) =
            dpi.unwrap_or((crate::image_dpi::DEFAULT_DPI, crate::image_dpi::DEFAULT_DPI));
        let (w, h) = img.dimensions();
        let mut png = Vec::new();
        if let Err(e) = image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        {
            self.status =
                crate::i18n::trf("status.could_not_encode_image", &[("e", &e.to_string())]);
            return None;
        }
        let mm_w = w as f64 * 25.4 / dpi_x;
        let mm_h = h as f64 * 25.4 / dpi_y;
        let page = self.page_rect();
        let c = page.center();
        let rect = Rect::new(
            c.x - mm_w / 2.0,
            c.y - mm_h / 2.0,
            c.x + mm_w / 2.0,
            c.y + mm_h / 2.0,
        );
        let layer = self.active_layer()?;
        let id = self.engine.new_shape_id();
        let mut shape = Shape::new(
            id,
            ShapeKind::Bitmap {
                rect,
                width_px: w,
                height_px: h,
                png: png.into(),
                fx: None,
            },
        );
        shape.stroke = None;
        shape.name = name;
        self.run(Command::AddShape { layer, shape });
        self.select(vec![id]);
        let _ = Point::ZERO;
        Some(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shaping {
    Weld,
    Trim,
    Intersect,
    Simplify,
    FrontMinusBack,
    BackMinusFront,
    Boundary,
}

impl Shaping {
    pub const ALL: [Shaping; 7] = [
        Shaping::Weld,
        Shaping::Trim,
        Shaping::Intersect,
        Shaping::Simplify,
        Shaping::FrontMinusBack,
        Shaping::BackMinusFront,
        Shaping::Boundary,
    ];

    /// Whether the operation can leave its source objects or its target
    /// as they were (Weld, Trim and Intersect).
    pub fn can_leave_originals(self) -> bool {
        matches!(self, Shaping::Weld | Shaping::Trim | Shaping::Intersect)
    }

    /// What the menus and the property bar leave: Trim keeps the objects
    /// that cut, Intersect keeps both, the others replace them (Boundary
    /// draws around them and keeps them all).
    pub fn default_leave(self) -> (bool, bool) {
        match self {
            Shaping::Trim => (true, false),
            Shaping::Intersect | Shaping::Boundary => (true, true),
            _ => (false, false),
        }
    }

    /// The name kept in the preferences.
    pub fn id(self) -> &'static str {
        match self {
            Shaping::Weld => "weld",
            Shaping::Trim => "trim",
            Shaping::Intersect => "intersect",
            Shaping::Simplify => "simplify",
            Shaping::FrontMinusBack => "front_minus_back",
            Shaping::BackMinusFront => "back_minus_front",
            Shaping::Boundary => "boundary",
        }
    }

    pub fn from_id(id: &str) -> Shaping {
        Shaping::ALL
            .into_iter()
            .find(|s| s.id() == id)
            .unwrap_or(Shaping::Weld)
    }

    /// With one object selected it waits for a click on the target, as
    /// the other drawing programs do (#562).
    pub fn picks_target(self) -> bool {
        matches!(self, Shaping::Weld | Shaping::Trim | Shaping::Intersect)
    }

    /// Whether it can start with `selected` objects selected: enough of
    /// them, or one when it then waits for the target.
    pub fn ready(self, selected: usize) -> bool {
        selected >= self.needs() || (selected == 1 && self.picks_target())
    }

    /// How many objects it needs selected.
    pub fn needs(self) -> usize {
        match self {
            Shaping::Simplify | Shaping::Boundary => 1,
            _ => 2,
        }
    }
}

/// The objects of a group (groups inside opened), their transforms
/// composed with the group's so each `page_path` is in page space;
/// empty for anything but a group.
fn group_leaves(s: &Shape) -> Vec<Shape> {
    fn walk(s: &Shape, parent: tracedraw_core::geometry::Affine, out: &mut Vec<Shape>) {
        let t = parent * s.transform;
        match &s.kind {
            ShapeKind::Group { children } => children.iter().for_each(|c| walk(c, t, out)),
            _ => {
                let mut leaf = s.clone();
                leaf.transform = t;
                out.push(leaf);
            }
        }
    }
    let mut out = Vec::new();
    if let ShapeKind::Group { children } = &s.kind {
        children.iter().for_each(|c| walk(c, s.transform, &mut out));
    }
    out
}

/// What a picture target shows, for a shaping result to hold: the picture
/// itself, or what a cropped picture (a clip frame holding one) holds,
/// placed on the page. `None` for other objects.
fn picture_contents(s: &Shape) -> Option<Vec<Shape>> {
    match &s.kind {
        ShapeKind::Bitmap { .. } => Some(vec![s.clone()]),
        ShapeKind::ClipFrame { contents, .. }
            if contents
                .iter()
                .any(|c| matches!(c.kind, ShapeKind::Bitmap { .. })) =>
        {
            Some(
                contents
                    .iter()
                    .map(|c| {
                        let mut c = c.clone();
                        c.transform = s.transform * c.transform;
                        c
                    })
                    .collect(),
            )
        }
        _ => None,
    }
}

impl App {
    /// The operation as the menus and the property bar do it (see
    /// `Shaping::default_leave`).
    pub fn shaping(&mut self, op: Shaping) {
        let (source, target) = op.default_leave();
        self.begin_shaping(op, source, target);
    }

    /// Start an operation: with two or more objects selected it runs at
    /// once (the last selected the target); Weld, Trim and Intersect with
    /// one selected wait for a click on the target, the selected object
    /// acting on it (#562).
    pub fn begin_shaping(&mut self, op: Shaping, leave_source: bool, leave_target: bool) {
        if self.selection.len() == 1 && op.picks_target() {
            self.pending_shaping = Some((op, leave_source, leave_target));
            self.status = crate::i18n::tr("status.click_shaping_target");
            return;
        }
        self.shaping_with(op, leave_source, leave_target);
    }

    /// The target of the waiting operation was clicked: the selection
    /// acts on it. A click on the selection itself or on nothing cancels.
    pub fn shaping_onto(&mut self, target: Option<tracedraw_core::ShapeId>) {
        let Some((op, leave_source, leave_target)) = self.pending_shaping.take() else {
            return;
        };
        let sources = self.selection.clone();
        let Some(target) = target.filter(|t| !sources.contains(t)) else {
            self.status = crate::i18n::tr("status.cancelled");
            return;
        };
        let rev = self.engine.revision();
        let mut picked = sources.clone();
        picked.push(target);
        self.select(picked);
        self.shaping_with(op, leave_source, leave_target);
        if self.engine.revision() == rev {
            // Nothing made (no overlap): the selection is as it was.
            let status = std::mem::take(&mut self.status);
            self.select(sources);
            self.status = status;
        }
    }

    /// Convention: the last selected object is the target and
    /// keeps its attributes; the other selected objects act on it. With
    /// `leave_source` the other objects stay, with `leave_target` the
    /// target stays (Weld, Trim and Intersect; Boundary keeps them all,
    /// the others replace them).
    pub fn shaping_with(&mut self, op: Shaping, leave_source: bool, leave_target: bool) {
        use tracedraw_core::shaping::{overlay, simplify, Op};
        let shapes = self.selected_shapes_target_last();
        if shapes.is_empty()
            || (shapes.len() < 2 && !matches!(op, Shaping::Simplify | Shaping::Boundary))
        {
            return;
        }
        let doc = self.doc();
        // Z-order: lowest index first within the layer.
        let mut ordered = shapes.clone();
        ordered.sort_by_key(|s| doc.locate(s.id).map(|(_, i)| i).unwrap_or(0));
        let target = shapes.last().cloned().unwrap_or_else(|| ordered[0].clone());
        // The object that keeps its attributes and is cut: the target, the
        // front one (Front Minus Back) or the back one (Back Minus Front).
        let kept = match op {
            Shaping::FrontMinusBack => ordered.last().cloned().unwrap_or(target.clone()),
            Shaping::BackMinusFront => ordered[0].clone(),
            _ => target.clone(),
        };
        let open = |s: &Shape| matches!(s.kind, ShapeKind::Path { closed: false, .. });
        let subtracts = matches!(
            op,
            Shaping::Trim | Shaping::FrontMinusBack | Shaping::BackMinusFront
        );
        // An open curve that is cut (the target of Weld, Trim or
        // Intersect, the kept one of the Minus operations) is cut by the
        // closed objects and stays open (#828).
        let open_target = (op.picks_target() || subtracts) && open(&kept);
        // An open curve acting on a closed object: Trim and the Minus
        // operations split the object along it, into pieces (#1005);
        // Weld and Intersect leave it out, and it stays as it is.
        let mut slicers: Vec<tracedraw_core::BezPath> = Vec::new();
        let mut slicer_ids: Vec<tracedraw_core::ShapeId> = Vec::new();
        if op.picks_target() || subtracts {
            if subtracts && !open_target {
                for s in ordered.iter().filter(|s| s.id != kept.id && open(s)) {
                    slicers.push(s.page_path());
                    slicer_ids.push(s.id);
                }
            }
            ordered.retain(|s| s.id == kept.id || !open(s));
            if ordered.len() < 2 && slicers.is_empty() {
                self.status = crate::i18n::tr("status.shaping_open_cutter");
                return;
            }
        } else if op == Shaping::Simplify {
            // An open curve covers nothing and is not a region to cut.
            ordered.retain(|s| !open(s));
            if ordered.is_empty() {
                return;
            }
        }
        let target = kept;
        let mut used: Vec<tracedraw_core::ShapeId> = ordered.iter().map(|s| s.id).collect();
        used.extend(&slicer_ids);
        let paths: Vec<tracedraw_core::BezPath> = ordered.iter().map(Shape::page_path).collect();
        let open_result = open_target.then(|| {
            use tracedraw_core::shaping::{cut_open, Keep};
            let regions: Vec<tracedraw_core::BezPath> = ordered
                .iter()
                .filter(|s| s.id != target.id)
                .map(Shape::page_path)
                .collect();
            let line = target.page_path();
            match op {
                Shaping::Intersect => cut_open(&line, &regions, Keep::Inside),
                Shaping::Weld => {
                    // The closed objects welded, with the curve's parts
                    // outside them, as one open curve.
                    let mut welded = regions
                        .iter()
                        .skip(1)
                        .fold(regions[0].clone(), |acc, p| overlay(&acc, p, Op::Weld));
                    for el in cut_open(&line, &regions, Keep::Outside).elements() {
                        welded.push(*el);
                    }
                    welded
                }
                _ => cut_open(&line, &regions, Keep::Outside),
            }
        });
        let result = if let Some(r) = open_result {
            r
        } else {
            match op {
                Shaping::Weld | Shaping::Boundary => paths
                    .iter()
                    .skip(1)
                    .fold(paths[0].clone(), |acc, p| overlay(&acc, p, Op::Weld)),
                Shaping::Intersect => paths
                    .iter()
                    .skip(1)
                    .fold(paths[0].clone(), |acc, p| overlay(&acc, p, Op::Intersect)),
                Shaping::Simplify => {
                    // Each object loses what is covered by objects above it.
                    let mut cmds = Vec::new();
                    for (i, s) in ordered.iter().enumerate() {
                        let mut p = s.page_path();
                        for above in &paths[i + 1..] {
                            p = overlay(&p, above, Op::Trim);
                        }
                        let p = simplify(&p);
                        let local = s.transform.inverse() * p;
                        cmds.push(Command::SetShapeKind {
                            shape: s.id,
                            kind: ShapeKind::Path {
                                path: local,
                                closed: true,
                            },
                        });
                    }
                    let _ = self.engine.run_batch("Simplify", &cmds);
                    return;
                }
                Shaping::Trim => {
                    let mut p = target.page_path();
                    for s in &ordered {
                        if s.id != target.id {
                            p = overlay(&p, &s.page_path(), Op::Trim);
                        }
                    }
                    p
                }
                Shaping::FrontMinusBack => {
                    let front = paths.last().cloned().unwrap_or_default();
                    paths
                        .iter()
                        .take(paths.len() - 1)
                        .fold(front, |acc, p| overlay(&acc, p, Op::Trim))
                }
                Shaping::BackMinusFront => paths
                    .iter()
                    .skip(1)
                    .fold(paths[0].clone(), |acc, p| overlay(&acc, p, Op::Trim)),
            }
        };
        if result.elements().is_empty() {
            self.status = crate::i18n::tr("status.shaping_empty");
            return;
        }
        // The open curves acting on it split what is left into pieces,
        // each an object of its own (#1005).
        let pieces: Vec<tracedraw_core::BezPath> = if slicers.is_empty() {
            Vec::new()
        } else {
            use tracedraw_core::shaping::slice_open;
            let whole = slice_open(&result, &[]).len();
            let pieces = slice_open(&result, &slicers);
            if pieces.len() > whole {
                pieces
            } else if ordered.len() < 2 {
                // Only open curves, none crossing it from edge to edge.
                self.status = crate::i18n::tr("status.shaping_open_no_cut");
                return;
            } else {
                Vec::new()
            }
        };
        let (leave_source, leave_target) = match op {
            Shaping::Boundary => (true, true),
            _ if op.can_leave_originals() => (leave_source, leave_target),
            _ => (false, false),
        };
        let attrs = match op {
            Shaping::FrontMinusBack => ordered.last().cloned().unwrap_or(target.clone()),
            Shaping::BackMinusFront => ordered[0].clone(),
            _ => target.clone(),
        };
        let Some(layer) = self.active_layer() else {
            return;
        };
        let id = self.engine.new_shape_id();
        let leaves = group_leaves(&attrs);
        let pictures = picture_contents(&attrs);
        // The further pieces of a split object, after the first.
        let mut extra: Vec<Shape> = Vec::new();
        let shape = if let Some(contents) = pictures {
            // A picture keeps showing inside the result: its frame takes
            // the result's shape, as a crop with the Shape tool does
            // (#827).
            let path = if pieces.is_empty() {
                result
            } else {
                let mut all = tracedraw_core::BezPath::new();
                for p in &pieces {
                    all.extend(p.elements().iter().copied());
                }
                all
            };
            let mut frame = Shape::new(
                self.engine.new_shape_id(),
                ShapeKind::Path { path, closed: true },
            );
            frame.fill = tracedraw_core::Fill::None;
            frame.stroke = attrs
                .stroke
                .clone()
                .filter(|_| matches!(attrs.kind, ShapeKind::ClipFrame { .. }));
            let contents = contents
                .into_iter()
                .map(|c| {
                    let nid = self.engine.new_shape_id();
                    crate::app::reid_pub(c, nid, &mut self.engine)
                })
                .collect();
            let mut shape = Shape::new(
                id,
                ShapeKind::ClipFrame {
                    frame: Box::new(frame),
                    contents,
                },
            );
            shape.opacity = attrs.opacity;
            shape
        } else if leaves.is_empty() {
            let mut paths = pieces.into_iter();
            let first = paths.next().unwrap_or(result);
            let piece = |id, path| {
                let mut shape = Shape::new(
                    id,
                    ShapeKind::Path {
                        path,
                        closed: !open_target,
                    },
                );
                shape.fill = attrs.fill.clone();
                shape.stroke = attrs.stroke.clone();
                shape.opacity = attrs.opacity;
                shape
            };
            for path in paths {
                extra.push(piece(self.engine.new_shape_id(), path));
            }
            piece(id, first)
        } else if matches!(
            op,
            Shaping::Trim | Shaping::FrontMinusBack | Shaping::BackMinusFront
        ) {
            // A group keeping its attributes is cut object by object, each
            // keeping its own fill and outline, the pieces a group; the
            // result took the group's (none) and showed only in Wireframe
            // (#539).
            let cutters: Vec<tracedraw_core::BezPath> = ordered
                .iter()
                .filter(|s| s.id != attrs.id)
                .map(Shape::page_path)
                .collect();
            let mut children = Vec::new();
            for leaf in &leaves {
                let p = cutters
                    .iter()
                    .fold(leaf.page_path(), |acc, c| overlay(&acc, c, Op::Trim));
                if p.elements().is_empty() {
                    continue;
                }
                let parts = if slicers.is_empty() {
                    vec![p]
                } else {
                    tracedraw_core::shaping::slice_open(&p, &slicers)
                };
                for p in parts {
                    let mut piece = Shape::new(
                        self.engine.new_shape_id(),
                        ShapeKind::Path {
                            path: p,
                            closed: true,
                        },
                    );
                    piece.fill = leaf.fill.clone();
                    piece.stroke = leaf.stroke.clone();
                    piece.opacity = leaf.opacity;
                    children.push(piece);
                }
            }
            if children.is_empty() {
                self.status = crate::i18n::tr("status.shaping_empty");
                return;
            }
            Shape::new(id, ShapeKind::Group { children })
        } else {
            // Weld and Intersect with a group: the attributes of its
            // first object with a fill (else its first).
            let first = leaves
                .iter()
                .find(|l| !matches!(l.fill, tracedraw_core::Fill::None))
                .unwrap_or(&leaves[0]);
            let mut shape = Shape::new(
                id,
                ShapeKind::Path {
                    path: result,
                    closed: true,
                },
            );
            shape.fill = first.fill.clone();
            shape.stroke = first.stroke.clone();
            shape.opacity = first.opacity;
            shape
        };
        let mut ids = vec![id];
        let mut cmds = vec![Command::AddShape { layer, shape }];
        for shape in extra {
            ids.push(shape.id);
            cmds.push(Command::AddShape { layer, shape });
        }
        let gone: Vec<_> = shapes
            .iter()
            .map(|s| s.id)
            .filter(|id| used.contains(id))
            .filter(|id| {
                if *id == target.id {
                    !leave_target
                } else {
                    !leave_source
                }
            })
            .collect();
        if !gone.is_empty() {
            cmds.push(Command::DeleteShapes { shapes: gone });
        }
        let label = match op {
            Shaping::Weld => "Weld",
            Shaping::Trim => "Trim",
            Shaping::Intersect => "Intersect",
            Shaping::FrontMinusBack => "Front Minus Back",
            Shaping::BackMinusFront => "Back Minus Front",
            Shaping::Boundary => "Boundary",
            Shaping::Simplify => "Simplify",
        };
        if let Err(e) = self.engine.run_batch(label, &cmds) {
            self.status = e.to_string();
            return;
        }
        self.select(ids);
    }
}

impl App {
    pub fn set_shadow(&mut self, shadow: Option<tracedraw_core::Shadow>, collapse: bool) {
        if self.selection.is_empty() {
            return;
        }
        let shapes = self.selection.clone();
        if collapse && matches!(self.engine.undo_label(), Some("Drop Shadow")) {
            let _ = self.engine.undo();
        }
        self.run(Command::SetShadow { shapes, shadow });
    }
}

impl App {
    /// Tab / Shift+Tab: select the next or previous object in z-order.
    pub fn cycle_selection(&mut self, forward: bool) {
        let ids: Vec<tracedraw_core::ShapeId> = self
            .page_layers()
            .into_iter()
            .filter(|l| !l.locked)
            .flat_map(|l| &l.shapes)
            .filter(|s| !s.locked && !self.outside_clip_edit(s.id))
            .map(|s| s.id)
            .collect();
        if ids.is_empty() {
            return;
        }
        let current = self
            .selection
            .first()
            .and_then(|id| ids.iter().position(|i| i == id));
        let next = match current {
            None => {
                if forward {
                    ids.len() - 1
                } else {
                    0
                }
            }
            Some(i) => {
                if forward {
                    (i + ids.len() - 1) % ids.len()
                } else {
                    (i + 1) % ids.len()
                }
            }
        };
        self.select(vec![ids[next]]);
    }
}

#[cfg(test)]
mod smoke_tests {
    use crate::app::App;
    use crate::tools::Tool;
    use tracedraw_core::{
        document::{ParagraphStyle, ShapeKind, TextSpan},
        geometry::{Point, Rect, Shape as _},
        TextAlign,
    };

    /// Back Minus Front with a group at the back: each of its objects is
    /// cut and keeps its own fill, the pieces a group; the result took the
    /// group's attributes (none) and showed only in Wireframe (#539).
    #[test]
    fn back_minus_front_with_a_group_keeps_each_objects_colours() {
        use tracedraw_core::{Color, Fill};
        let mut app = App::headless();
        let mut square = |app: &mut App, r: Rect, c: Color| {
            let id = app
                .new_shape(ShapeKind::Rect {
                    rect: r,
                    radius: 0.0,
                    corners: None,
                })
                .expect("rect");
            app.select(vec![id]);
            app.apply_fill(Fill::Solid(c));
            id
        };
        let orange = Color::rgb8(240, 130, 30);
        let grey = Color::rgb8(160, 190, 210);
        let blue = Color::rgb8(0, 150, 220);
        let a = square(&mut app, Rect::new(0.0, 0.0, 20.0, 20.0), orange);
        let b = square(&mut app, Rect::new(22.0, 0.0, 42.0, 20.0), grey);
        let c = square(&mut app, Rect::new(10.0, -6.0, 30.0, 14.0), blue);
        app.select(vec![a, b, c]);
        app.group_selection();
        let g = app.selection[0];
        // The front: a frame over their top halves, no fill of its own.
        let front = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(-5.0, 10.0, 50.0, 30.0),
                radius: 0.0,
                corners: None,
            })
            .expect("front");
        app.select(vec![g, front]);
        app.shaping(crate::ops::Shaping::BackMinusFront);
        let result = app
            .doc()
            .find_shape(app.selection[0])
            .cloned()
            .expect("result");
        let ShapeKind::Group { children } = &result.kind else {
            panic!("a group: {:?}", result.kind);
        };
        let fills: Vec<Fill> = children.iter().map(|s| s.fill.clone()).collect();
        assert_eq!(
            fills,
            vec![Fill::Solid(orange), Fill::Solid(grey), Fill::Solid(blue)]
        );
        // Each piece below the front only.
        assert!(children
            .iter()
            .all(|s| s.page_path().bounding_box().y1 <= 10.0 + 1e-6));
        assert!(app.doc().find_shape(g).is_none() && app.doc().find_shape(front).is_none());
    }

    /// The sequence a user runs through: draw one of everything, then
    /// group, ungroup, combine, break apart and convert to curves.
    #[test]
    fn group_ungroup_combine_break_convert_survives_mixed_objects() {
        let mut app = App::headless();
        app.tool = Tool::Rectangle;
        app.create_box_shape(Point::new(10.0, 200.0), Point::new(40.0, 230.0));
        app.tool = Tool::Ellipse;
        app.create_box_shape(Point::new(50.0, 200.0), Point::new(80.0, 230.0));
        app.tool = Tool::Polygon;
        app.create_box_shape(Point::new(90.0, 200.0), Point::new(120.0, 230.0));
        app.tool = Tool::GraphPaper;
        app.create_box_shape(Point::new(50.0, 150.0), Point::new(80.0, 180.0));
        app.tool = Tool::Spiral;
        let r = Rect::new(10.0, 150.0, 40.0, 180.0);
        let path = crate::tools2::spiral_path(r, app.spiral_revolutions, app.spiral_logarithmic);
        app.new_shape(ShapeKind::Path {
            path,
            closed: false,
        });
        let mut line = tracedraw_core::geometry::BezPath::new();
        line.move_to((10.0, 100.0));
        line.line_to((60.0, 80.0));
        app.new_shape(ShapeKind::Path {
            path: line,
            closed: false,
        });
        app.new_shape(ShapeKind::Text {
            spans: vec![TextSpan::new("Smoke test", "Arial", 24.0)],
            origin: Point::new(20.0, 50.0),
            frame: None,
            align: TextAlign::Left,
            para: ParagraphStyle::default(),
            on_path: None,
        });
        app.select_all();
        assert_eq!(app.selection.len(), 7);
        let render = |app: &App, step: &str| {
            let pm = tracedraw_render::render_page_image(app.doc(), app.page, 25.4);
            assert!(pm.is_some(), "render after {step}");
            eprintln!(
                "{step}: {} selected, {} objects",
                app.selection.len(),
                app.doc().pages[0]
                    .layers
                    .iter()
                    .map(|l| l.shapes.len())
                    .sum::<usize>()
            );
        };
        app.group_selection();
        render(&app, "group");
        app.ungroup_selection();
        render(&app, "ungroup");
        app.combine();
        render(&app, "combine");
        app.break_apart();
        render(&app, "break apart");
        app.convert_to_curves();
        render(&app, "convert");
        assert!(!app.selection.is_empty());
        for s in app.selected_shapes() {
            assert!(s.bounds().width().is_finite());
        }
    }

    #[test]
    fn page_frame_is_a_selected_rectangle_the_size_of_the_page() {
        let mut app = App::headless();
        app.add_page_frame();
        let size = app.page_size();
        let shapes = app.selected_shapes();
        assert_eq!(shapes.len(), 1);
        let b = shapes[0].bounds();
        assert!((b.width() - size.width).abs() < 1e-6);
        assert!((b.height() - size.height).abs() < 1e-6);
        assert!(b.x0.abs() < 1e-6 && b.y0.abs() < 1e-6);
    }

    #[test]
    fn save_bitmap_as_takes_the_selected_bitmaps_pixels() {
        let mut app = App::headless();
        assert_eq!(app.selected_bitmap_png(), None);
        let png = vec![0x89, b'P', b'N', b'G', 1, 2, 3];
        let id = app
            .new_shape(tracedraw_core::ShapeKind::Bitmap {
                rect: tracedraw_core::geometry::Rect::new(0.0, 0.0, 10.0, 10.0),
                width_px: 1,
                height_px: 1,
                png: png.clone().into(),
                fx: None,
            })
            .expect("bitmap");
        app.select(vec![id]);
        assert_eq!(app.selected_bitmap_png().as_deref(), Some(&png[..]));
        let other = app
            .new_shape(tracedraw_core::ShapeKind::Ellipse {
                rect: tracedraw_core::geometry::Rect::new(0.0, 0.0, 5.0, 5.0),
                arc: None,
            })
            .expect("ellipse");
        app.select(vec![other]);
        assert_eq!(app.selected_bitmap_png(), None);
    }

    #[test]
    fn shift_double_click_frames_the_selection() {
        let mut app = App::headless();
        let id = app
            .new_shape(tracedraw_core::ShapeKind::Ellipse {
                rect: tracedraw_core::geometry::Rect::new(10.0, 20.0, 50.0, 40.0),
                arc: None,
            })
            .expect("ellipse");
        app.select(vec![id]);
        let want = app.selection_bounds().expect("bounds");
        app.add_selection_frame();
        let shapes = app.selected_shapes();
        assert_eq!(shapes.len(), 1);
        assert!(matches!(
            shapes[0].kind,
            tracedraw_core::ShapeKind::Rect { .. }
        ));
        let b = shapes[0].bounds();
        assert!((b.x0 - want.x0).abs() < 1e-6 && (b.x1 - want.x1).abs() < 1e-6);
        assert!((b.y0 - want.y0).abs() < 1e-6 && (b.y1 - want.y1).abs() < 1e-6);
        // Nothing selected: the page frame.
        app.select(Vec::new());
        app.add_selection_frame();
        let size = app.page_size();
        let b = app.selected_shapes()[0].bounds();
        assert!((b.width() - size.width).abs() < 1e-6);
    }

    #[test]
    fn toolbox_groups_remember_the_last_tool_used() {
        let mut app = App::headless();
        let gi = crate::tools::GROUPS
            .iter()
            .position(|g| g.tools.contains(&Tool::Spiral))
            .expect("spiral is in a group");
        app.set_tool(Tool::Spiral);
        app.set_tool(Tool::Pick);
        assert_eq!(app.toolbox_last[gi], Tool::Spiral);
        app.sync_settings();
        let saved = app.settings.toolbox.clone();
        let mut other = App::headless();
        let mut s = other.settings.clone();
        s.toolbox = saved;
        other.apply_settings(s);
        assert_eq!(other.toolbox_last[gi], Tool::Spiral);
    }
}

/// How Break Apart cuts a text: its lines when it has more than one
/// (empty lines dropped), else its words, else its characters (spaces
/// dropped). Character ranges of the concatenated text, in order.
pub(crate) fn text_pieces(text: &str) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let runs = |is_break: &dyn Fn(char) -> bool| -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut start = None;
        for (i, c) in chars.iter().enumerate() {
            if is_break(*c) {
                if let Some(s) = start.take() {
                    out.push((s, i));
                }
            } else if start.is_none() {
                start = Some(i);
            }
        }
        if let Some(s) = start {
            out.push((s, chars.len()));
        }
        out
    };
    let lines = runs(&|c| c == '\n' || c == '\u{2028}' || c == '\r');
    if lines.len() > 1 {
        return lines;
    }
    let words = runs(&|c: char| c.is_whitespace());
    if words.len() > 1 {
        return words;
    }
    // A single word (or a run without spaces, like CJK text): characters.
    runs(&|c: char| c.is_whitespace())
        .into_iter()
        .flat_map(|(a, b)| (a..b).map(|i| (i, i + 1)))
        .collect()
}

#[cfg(test)]
mod curves_and_target_tests {
    use super::Shaping;
    use crate::app::App;
    use tracedraw_core::{
        document::TextSpan,
        geometry::{Affine, Rect},
        Color, Fill, Shape, ShapeId, ShapeKind, Stroke, Table,
    };

    fn leaves(s: &Shape, out: &mut Vec<Shape>) {
        match &s.kind {
            ShapeKind::Group { children } => children.iter().for_each(|c| leaves(c, out)),
            _ => out.push(s.clone()),
        }
    }

    fn ids(s: &Shape, out: &mut Vec<ShapeId>) {
        out.push(s.id);
        if let ShapeKind::Group { children } = &s.kind {
            children.iter().for_each(|c| ids(c, out));
        }
    }

    fn converted(app: &App, id: ShapeId) -> Vec<Shape> {
        let s = app.doc().find_shape(id).expect("still there").clone();
        assert!(matches!(s.kind, ShapeKind::Group { .. }), "{:?}", s.kind);
        let mut all = Vec::new();
        ids(&s, &mut all);
        let mut unique = all.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), all.len(), "every piece has its own id");
        assert!(!all.contains(&ShapeId(0)));
        let mut out = Vec::new();
        leaves(&s, &mut out);
        out
    }

    #[test]
    fn a_table_becomes_curves_with_its_cells_borders_and_text() {
        let mut app = App::headless();
        let border = Stroke::hairline(Color::BLACK);
        let mut t = Table::new(
            Rect::new(10.0, 10.0, 70.0, 30.0),
            2,
            2,
            Some(border.clone()),
        );
        let red = Fill::Solid(Color::rgb8(255, 0, 0));
        t.cell_fill = red.clone();
        t.cells[0].text = vec![TextSpan::new("Hello", "Sans", 12.0)];
        let id = app.new_shape(ShapeKind::Table(t)).expect("a table");
        app.select(vec![id]);
        assert!(app.can_convert_to_curves());
        app.convert_to_curves();
        let pieces = converted(&app, id);
        assert!(
            pieces
                .iter()
                .all(|s| matches!(s.kind, ShapeKind::Path { .. })),
            "{pieces:?}"
        );
        // Four cell fills, one text, one border.
        let fills = pieces.iter().filter(|s| s.fill == red).count();
        assert_eq!(fills, 4);
        let borders: Vec<_> = pieces
            .iter()
            .filter(|s| s.stroke.as_ref() == Some(&border))
            .collect();
        assert_eq!(borders.len(), 1);
        assert_eq!(borders[0].fill, Fill::None);
        assert_eq!(pieces.len(), 6, "{pieces:?}");
        // The group covers the table's cells.
        let b = app.doc().find_shape(id).expect("group").bounds();
        assert!(
            (b.x0 - 10.0).abs() < 1e-6 && (b.x1 - 70.0).abs() < 1e-6,
            "{b:?}"
        );
        app.undo();
        assert!(matches!(
            app.doc().find_shape(id).map(|s| &s.kind),
            Some(ShapeKind::Table(_))
        ));
    }

    #[test]
    fn a_symbol_instance_becomes_the_curves_of_its_symbol() {
        let mut app = App::headless();
        let a = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(20.0, 20.0, 40.0, 30.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        let b = app
            .new_shape(ShapeKind::Ellipse {
                rect: Rect::new(50.0, 20.0, 60.0, 30.0),
                arc: None,
            })
            .expect("ellipse");
        app.select(vec![a, b]);
        app.create_symbol_from_selection();
        let inst = app.selection[0];
        app.run(tracedraw_core::Command::TransformShapes {
            shapes: vec![inst],
            transform: Affine::translate((5.0, 0.0)),
        });
        let before = app.doc().find_shape(inst).expect("instance").clone();
        let before = before.local_path_with(&app.doc().symbols).elements().len();
        app.convert_to_curves();
        let pieces = converted(&app, inst);
        assert_eq!(pieces.len(), 2, "{pieces:?}");
        assert!(pieces
            .iter()
            .all(|s| matches!(s.kind, ShapeKind::Path { .. })));
        let g = app.doc().find_shape(inst).expect("group").clone();
        assert_eq!(g.local_path().elements().len(), before);
        // Where the instance was: the rectangle moved with it.
        let bounds = g.bounds();
        assert!((bounds.x0 - 25.0).abs() < 1e-6, "{bounds:?}");
        assert!((bounds.x1 - 65.0).abs() < 1e-6, "{bounds:?}");
    }

    #[test]
    fn a_clip_frame_keeps_its_contents_as_curves() {
        let mut app = App::headless();
        let fid = app.engine.new_shape_id();
        let rid = app.engine.new_shape_id();
        let bid = app.engine.new_shape_id();
        let frame = Shape::new(
            fid,
            ShapeKind::Ellipse {
                rect: Rect::new(10.0, 10.0, 50.0, 40.0),
                arc: None,
            },
        );
        let mut inner = Shape::new(
            rid,
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 30.0, 30.0),
                radius: 2.0,
                corners: None,
            },
        );
        inner.fill = Fill::Solid(Color::rgb8(0, 128, 0));
        let bitmap = Shape::new(
            bid,
            ShapeKind::Bitmap {
                rect: Rect::new(20.0, 20.0, 40.0, 30.0),
                width_px: 1,
                height_px: 1,
                png: Vec::new().into(),
                fx: None,
            },
        );
        let id = app
            .new_shape(ShapeKind::ClipFrame {
                frame: Box::new(frame),
                contents: vec![inner, bitmap],
            })
            .expect("a clip frame");
        app.select(vec![id]);
        assert!(app.can_convert_to_curves());
        app.convert_to_curves();
        let s = app.doc().find_shape(id).expect("still there").clone();
        let ShapeKind::ClipFrame { frame, contents } = &s.kind else {
            panic!("still a clip frame: {:?}", s.kind);
        };
        assert_eq!(frame.id, fid);
        assert!(matches!(frame.kind, ShapeKind::Path { .. }));
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0].id, rid);
        assert!(matches!(contents[0].kind, ShapeKind::Path { .. }));
        assert_eq!(contents[0].fill, Fill::Solid(Color::rgb8(0, 128, 0)));
        // A bitmap inside stays a bitmap.
        assert!(matches!(contents[1].kind, ShapeKind::Bitmap { .. }));
        // Converting again changes nothing, and the command is greyed.
        assert!(!app.can_convert_to_curves());
        let rev = app.engine.revision();
        app.convert_to_curves();
        assert_eq!(app.engine.revision(), rev);
    }

    /// An ellipse converts to one closed curve of four nodes, at its
    /// right, top, left and bottom, each smooth (#625).
    #[test]
    fn an_ellipse_becomes_a_closed_curve_of_four_nodes() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Ellipse {
                rect: Rect::new(10.0, 10.0, 70.0, 50.0),
                arc: None,
            })
            .expect("an ellipse");
        app.select(vec![id]);
        app.convert_to_curves();
        let Some(ShapeKind::Path { path, closed }) = app.doc().find_shape(id).map(|s| &s.kind)
        else {
            panic!("a curve");
        };
        assert!(*closed, "closed");
        let nodes = tracedraw_core::nodes::nodes(path);
        assert_eq!(nodes.len(), 4, "{path:?}");
        let at: Vec<_> = nodes.iter().map(|n| (n.pos.x, n.pos.y)).collect();
        assert_eq!(
            at,
            vec![(70.0, 30.0), (40.0, 50.0), (10.0, 30.0), (40.0, 10.0)]
        );
        for i in 0..4 {
            assert_eq!(
                tracedraw_core::nodes::node_type(path, nodes[i].index),
                tracedraw_core::nodes::NodeType::Symmetrical,
                "{path:?}"
            );
        }
    }

    #[test]
    fn a_bitmap_is_not_converted() {
        let mut app = App::headless();
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 255]));
        let id = app.place_image(img, None).expect("bitmap");
        assert!(!app.can_convert_to_curves());
        app.convert_to_curves();
        assert!(matches!(
            app.doc().find_shape(id).map(|s| &s.kind),
            Some(ShapeKind::Bitmap { .. })
        ));
        assert_eq!(app.status, crate::i18n::tr("status.bitmap_not_curves"));
    }

    /// A red rectangle under a blue one, overlapping.
    fn two_rects(app: &mut App) -> (ShapeId, ShapeId) {
        let mut add = |r: Rect, c: Color| {
            let id = app
                .new_shape(ShapeKind::Rect {
                    rect: r,
                    radius: 0.0,
                    corners: None,
                })
                .expect("rect");
            app.run(tracedraw_core::Command::SetFill {
                shapes: vec![id],
                fill: Fill::Solid(c),
            });
            id
        };
        let bottom = add(Rect::new(10.0, 10.0, 40.0, 40.0), Color::rgb8(255, 0, 0));
        let top = add(Rect::new(30.0, 30.0, 60.0, 60.0), Color::rgb8(0, 0, 255));
        (bottom, top)
    }

    fn result_fill(app: &App, gone: &[ShapeId]) -> Fill {
        let s = app
            .doc()
            .all_layers()
            .flat_map(|l| &l.shapes)
            .find(|s| !gone.contains(&s.id) && matches!(s.kind, ShapeKind::Path { .. }))
            .expect("a result")
            .clone();
        s.fill
    }

    /// #1005: Trim with an open line across a circle cuts the circle in
    /// two along the line, two pieces with the circle's fill and outline
    /// whose areas add up to the circle's; the line stays. Two lines make
    /// three pieces, a line that ends inside cuts nothing and says so, a
    /// closed object and a line together cut and split, and Back Minus
    /// Front splits the back object along a line in front.
    #[test]
    fn open_curves_split_closed_targets_into_pieces() {
        use tracedraw_core::geometry::{BezPath, Shape as _};
        let red = Fill::Solid(Color::rgb8(255, 0, 0));
        let line_at = |app: &mut App, pts: &[(f64, f64)]| {
            let mut p = BezPath::new();
            p.move_to(pts[0]);
            for q in &pts[1..] {
                p.line_to(*q);
            }
            app.new_shape(ShapeKind::Path {
                path: p,
                closed: false,
            })
            .expect("line")
        };
        let circle_app = || {
            let mut app = App::headless();
            let circle = app
                .new_shape(ShapeKind::Ellipse {
                    rect: Rect::new(30.0, 30.0, 70.0, 70.0),
                    arc: None,
                })
                .expect("circle");
            app.run(tracedraw_core::Command::SetFill {
                shapes: vec![circle],
                fill: red.clone(),
            });
            (app, circle)
        };
        let pieces = |app: &App| -> Vec<Shape> {
            app.selection
                .iter()
                .map(|id| app.doc().shape_on_page(*id).expect("piece").clone())
                .collect()
        };
        let area = |s: &Shape| s.page_path().area().abs();
        // The issue: a slanted line past the outline at both ends.
        let (mut app, circle) = circle_app();
        let full = area(app.doc().find_shape(circle).expect("circle"));
        let stroke = app.doc().find_shape(circle).expect("circle").stroke.clone();
        let line = line_at(&mut app, &[(20.0, 35.0), (80.0, 75.0)]);
        app.select(vec![line, circle]);
        app.shaping(Shaping::Trim);
        let got = pieces(&app);
        assert_eq!(got.len(), 2, "{:?}", app.status);
        for p in &got {
            assert!(matches!(p.kind, ShapeKind::Path { closed: true, .. }));
            assert_eq!(p.fill, red);
            assert_eq!(p.stroke, stroke);
            assert!(area(p) > 100.0 && area(p) < full - 100.0, "{}", area(p));
        }
        let sum: f64 = got.iter().map(area).sum();
        assert!((sum - full).abs() < 1e-3, "{sum} against {full}");
        assert!(app.doc().find_shape(circle).is_none(), "the circle is cut");
        assert!(app.doc().find_shape(line).is_some(), "the line stays");
        // One undo step brings the circle back.
        app.undo();
        assert!(app.doc().find_shape(circle).is_some());
        // A horizontal line through the centre: two equal halves.
        let (mut app, circle) = circle_app();
        let line = line_at(&mut app, &[(20.0, 50.0), (80.0, 50.0)]);
        app.select(vec![line, circle]);
        app.shaping(Shaping::Trim);
        let got = pieces(&app);
        assert_eq!(got.len(), 2);
        for p in &got {
            assert!((area(p) - full / 2.0).abs() < 1e-3, "{}", area(p));
        }
        // Two lines: three pieces.
        let (mut app, circle) = circle_app();
        let a = line_at(&mut app, &[(20.0, 45.0), (80.0, 45.0)]);
        let b = line_at(&mut app, &[(20.0, 55.0), (80.0, 55.0)]);
        app.select(vec![a, b, circle]);
        app.shaping(Shaping::Trim);
        let got = pieces(&app);
        assert_eq!(got.len(), 3);
        let sum: f64 = got.iter().map(area).sum();
        assert!((sum - full).abs() < 1e-3);
        // A line that ends inside: nothing cut, a message.
        let (mut app, circle) = circle_app();
        let line = line_at(&mut app, &[(20.0, 50.0), (50.0, 50.0)]);
        let before = app.doc().clone();
        app.select(vec![line, circle]);
        app.shaping(Shaping::Trim);
        assert_eq!(app.doc().pages, before.pages);
        assert_eq!(app.status, crate::i18n::tr("status.shaping_open_no_cut"));
        // A 40 by 20 rectangle, a closed square taking 10 by 10 off a
        // corner and a polyline cutting another 10 by 10 corner off: two
        // pieces, 100 and 600 mm2.
        let mut app = App::headless();
        let rect = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 40.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        let square = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(30.0, 10.0, 50.0, 30.0),
                radius: 0.0,
                corners: None,
            })
            .expect("square");
        let poly = line_at(&mut app, &[(-10.0, 10.0), (10.0, 10.0), (10.0, 30.0)]);
        app.select(vec![square, poly, rect]);
        app.shaping(Shaping::Trim);
        let mut areas: Vec<f64> = pieces(&app).iter().map(area).collect();
        areas.sort_by(f64::total_cmp);
        assert_eq!(areas.len(), 2, "{areas:?}");
        assert!((areas[0] - 100.0).abs() < 1e-6, "{areas:?}");
        assert!((areas[1] - 600.0).abs() < 1e-6, "{areas:?}");
        // Back Minus Front: the circle at the back, a line in front.
        let (mut app, circle) = circle_app();
        let line = line_at(&mut app, &[(20.0, 50.0), (80.0, 50.0)]);
        app.select(vec![circle, line]);
        app.shaping(Shaping::BackMinusFront);
        let got = pieces(&app);
        assert_eq!(got.len(), 2);
        assert!(got.iter().all(|p| p.fill == red));
        assert!(app.doc().find_shape(line).is_none());
        assert!(app.doc().find_shape(circle).is_none());
    }

    /// #828: an open line as the target of Trim, Intersect and Weld is
    /// cut by the closed rectangle and stays open (Trim keeps the parts
    /// outside, Intersect the part inside, Weld the rectangle with the
    /// parts outside); an open line acting on a closed target in Weld or
    /// Intersect does not cut it, and says so.
    #[test]
    fn open_targets_are_cut_and_open_cutters_leave_shapes_alone() {
        use tracedraw_core::geometry::{BezPath, Shape as _};
        let setup = || {
            let mut app = App::headless();
            let rect = app
                .new_shape(ShapeKind::Rect {
                    rect: Rect::new(20.0, 40.0, 60.0, 60.0),
                    radius: 0.0,
                    corners: None,
                })
                .expect("rect");
            let mut p = BezPath::new();
            p.move_to((0.0, 50.0));
            p.line_to((80.0, 50.0));
            let line = app
                .new_shape(ShapeKind::Path {
                    path: p,
                    closed: false,
                })
                .expect("line");
            (app, rect, line)
        };
        let path_of = |app: &App| -> (BezPath, bool) {
            let s = app.doc().shape_on_page(app.selection[0]).expect("result");
            let closed = matches!(s.kind, ShapeKind::Path { closed: true, .. });
            (s.page_path(), closed)
        };
        let length = |p: &BezPath| -> f64 {
            use tracedraw_core::geometry::ParamCurveArclen as _;
            p.segments().map(|s| s.arclen(1e-6)).sum()
        };
        // Trim: the line target keeps its two parts outside, 40 mm.
        let (mut app, rect, line) = setup();
        app.select(vec![rect, line]);
        app.shaping(Shaping::Trim);
        let (p, closed) = path_of(&app);
        assert!(!closed);
        assert!((length(&p) - 40.0).abs() < 1e-6, "{p:?}");
        assert!(
            app.doc().find_shape(line).is_none(),
            "the target is replaced"
        );
        assert!(app.doc().find_shape(rect).is_some(), "the cutter stays");
        // Intersect: the part inside.
        let (mut app, rect, line) = setup();
        app.select(vec![rect, line]);
        app.shaping(Shaping::Intersect);
        let (p, closed) = path_of(&app);
        assert!(!closed);
        let b = p.bounding_box();
        assert!(
            (b.x0 - 20.0).abs() < 1e-6 && (b.x1 - 60.0).abs() < 1e-6,
            "{b:?}"
        );
        // Weld: the rectangle's outline and the parts outside, open.
        let (mut app, rect, line) = setup();
        app.select(vec![rect, line]);
        app.shaping(Shaping::Weld);
        let (p, closed) = path_of(&app);
        assert!(!closed);
        assert!((length(&p) - (40.0 + 120.0)).abs() < 0.1, "{}", length(&p));
        // An open line acting on the rectangle in Weld or Intersect:
        // nothing cut, a message (Trim splits it, #1005).
        for op in [Shaping::Weld, Shaping::Intersect] {
            let (mut app, rect, line) = setup();
            let before = app.doc().clone();
            app.select(vec![line, rect]);
            app.shaping(op);
            assert_eq!(app.doc().pages, before.pages);
            assert_eq!(app.status, crate::i18n::tr("status.shaping_open_cutter"));
        }
        // With a closed cutter as well, the open one takes no part and
        // stays.
        let (mut app, rect, line) = setup();
        let small = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(50.0, 45.0, 70.0, 55.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![line, small, rect]);
        app.shaping(Shaping::Weld);
        assert!(app.doc().find_shape(line).is_some(), "the open line stays");
        let (p, closed) = path_of(&app);
        assert!(closed);
        let b = p.bounding_box();
        assert!(
            (b.x1 - 70.0).abs() < 1e-6 && (b.y0 - 40.0).abs() < 1e-6,
            "{b:?}"
        );
    }

    #[test]
    fn shaping_targets_the_last_clicked_object_but_the_bottom_one_of_a_marquee() {
        let blue = Fill::Solid(Color::rgb8(0, 0, 255));
        let red = Fill::Solid(Color::rgb8(255, 0, 0));
        for op in [Shaping::Trim, Shaping::Weld, Shaping::Intersect] {
            // Picked one by one: the one picked last is the target.
            let mut app = App::headless();
            let (bottom, top) = two_rects(&mut app);
            app.select(vec![bottom, top]);
            app.shaping_with(op, false, false);
            assert_eq!(result_fill(&app, &[]), blue, "{op:?} clicked");

            let mut app = App::headless();
            let (bottom, top) = two_rects(&mut app);
            app.select(vec![top, bottom]);
            app.shaping_with(op, false, false);
            assert_eq!(result_fill(&app, &[]), red, "{op:?} clicked in reverse");

            // Marquee: the bottom-most object is the target.
            let mut app = App::headless();
            let (bottom, top) = two_rects(&mut app);
            app.select(Vec::new());
            app.select_in_region(
                crate::select_region::Region::Rect(Rect::new(0.0, 0.0, 70.0, 70.0)),
                false,
                false,
            );
            assert_eq!(app.selection, vec![bottom, top]);
            app.shaping_with(op, false, false);
            assert_eq!(result_fill(&app, &[]), red, "{op:?} marquee");
        }
        // A marquee selection changed by a click no longer counts.
        let mut app = App::headless();
        let (bottom, top) = two_rects(&mut app);
        app.select_in_region(
            crate::select_region::Region::Rect(Rect::new(0.0, 0.0, 70.0, 70.0)),
            false,
            false,
        );
        app.select(vec![bottom, top]);
        assert_eq!(
            app.selected_shapes_target_last().last().map(|s| s.id),
            Some(top)
        );
    }

    #[test]
    fn join_curves_keeps_the_bottom_curve_of_a_marquee() {
        let mut app = App::headless();
        let line = |app: &mut App, a: (f64, f64), b: (f64, f64)| {
            let mut p = tracedraw_core::BezPath::new();
            p.move_to(a);
            p.line_to(b);
            app.new_shape(ShapeKind::Path {
                path: p,
                closed: false,
            })
            .expect("a line")
        };
        let a = line(&mut app, (100.0, 100.0), (108.0, 100.0));
        let b = line(&mut app, (110.0, 102.0), (110.0, 110.0));
        app.select(Vec::new());
        app.select_in_region(
            crate::select_region::Region::Rect(Rect::new(90.0, 90.0, 120.0, 120.0)),
            false,
            false,
        );
        app.join_settings.gap = 5.0;
        assert_eq!(app.join_curves(), 1);
        assert_eq!(app.selection, vec![a]);
        assert!(app.doc().find_shape(b).is_none());
    }
}
