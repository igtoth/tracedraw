//! Operations added with the full menu structure: page numbers, order
//! relative to another object, effects on the stack, bitmap effects and
//! tracing, text conversions, scripts, palettes.

use crate::app::App;
use crate::corners::JoinKind;
use crate::i18n::{tr, trf};
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, BezPath, Point, Rect, Shape as _, Size},
    live::Effect,
    Color, Command, Fill, ShapeId, TextSpan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageNumberWhere {
    Active,
    All,
    Odd,
    Even,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMode {
    Sentence,
    Lower,
    Upper,
    Title,
    Toggle,
}

/// An undo step that began (`begins`) or finished editing the contents
/// of the clip frame `frame`, with the objects that were on the page
/// besides the frame (see [`App::clip_edit_contents`]).
#[derive(Debug, Clone, PartialEq)]
pub struct ClipEditMark {
    /// The drawing's engine (step ids are its own).
    pub engine: u64,
    pub step: u64,
    pub frame: ShapeId,
    pub outside: std::collections::HashSet<ShapeId>,
    pub begins: bool,
}

/// Step and Repeat docker state.
#[derive(Debug, Clone, PartialEq)]
pub struct StepRepeat {
    pub copies: u32,
    pub dx: f64,
    pub dy: f64,
    /// 0 = offset, 1 = spacing between copies (uses object size).
    pub mode_x: u8,
    pub mode_y: u8,
    /// The objects these values were set for, in id order. Another
    /// selection, the copies an Apply made among them, starts again from
    /// the defaults (#1032).
    pub for_selection: Vec<ShapeId>,
}

impl Default for StepRepeat {
    fn default() -> Self {
        StepRepeat {
            copies: 1,
            dx: 6.35,
            dy: 0.0,
            mode_x: 0,
            mode_y: 0,
            for_selection: Vec::new(),
        }
    }
}

/// Align and Distribute docker: what to align to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignTo {
    #[default]
    ActiveObject,
    PageEdge,
    PageCenter,
    Grid,
}

impl AlignTo {
    /// The name kept in the preferences.
    pub fn id(self) -> &'static str {
        match self {
            AlignTo::ActiveObject => "active",
            AlignTo::PageEdge => "page_edge",
            AlignTo::PageCenter => "page_center",
            AlignTo::Grid => "grid",
        }
    }

    /// The target a name stands for (the active object for an unknown
    /// name).
    pub fn from_id(id: &str) -> AlignTo {
        match id {
            "page_edge" => AlignTo::PageEdge,
            "page_center" => AlignTo::PageCenter,
            "grid" => AlignTo::Grid,
            _ => AlignTo::ActiveObject,
        }
    }
}

impl App {
    /// The bounds of everything drawn on the current page (its layers'
    /// objects, effects included), page mm.
    pub fn drawing_bounds(&self) -> Option<tracedraw_core::geometry::Rect> {
        let doc = self.doc();
        let page = doc.page(self.page).ok()?;
        page.layers
            .iter()
            .flat_map(|l| l.shapes.iter())
            .filter(|s| s.visible)
            .map(|s| self.visual_bounds(s))
            .reduce(|a, b| a.union(b))
    }

    /// Layout > Fit Page to Selection: the current page takes the size
    /// of the selection's bounds and the page's objects move so the
    /// selection sits on the page from its bottom-left corner; one undo
    /// step. Nothing happens without a selection.
    pub fn fit_page_to_selection(&mut self) {
        if let Some(b) = self.selection_bounds() {
            self.fit_page_to(b, "Fit Page to Selection");
        }
    }

    /// Layout > Fit Page to Drawing: as [`Self::fit_page_to_selection`],
    /// for everything on the page.
    pub fn fit_page_to_drawing(&mut self) {
        self.fit_page_to_drawing_with_margin(0.0);
    }

    /// The page Autofit Page would make while its popup is open (#594):
    /// the drawing's bounds with the margin around them. The popup sets
    /// it each frame; one set more than a frame before `frame` is stale.
    pub fn autofit_preview_rect(&self, frame: u64) -> Option<tracedraw_core::geometry::Rect> {
        let (margin, set) = self.autofit_preview?;
        if frame.saturating_sub(set) > 1 {
            return None;
        }
        let m = if margin.is_finite() {
            margin.max(0.0)
        } else {
            0.0
        };
        self.drawing_bounds().map(|b| b.inflate(m, m))
    }

    /// The property bar's Autofit Page: [`Self::fit_page_to_drawing`]
    /// with `margin` mm left free around the drawing on every side.
    pub fn fit_page_to_drawing_with_margin(&mut self, margin: f64) {
        let margin = if margin.is_finite() {
            margin.max(0.0)
        } else {
            0.0
        };
        if let Some(b) = self.drawing_bounds() {
            self.fit_page_to(b.inflate(margin, margin), "Fit Page to Drawing");
        }
    }

    fn fit_page_to(&mut self, b: tracedraw_core::geometry::Rect, label: &'static str) {
        use tracedraw_core::geometry::{Affine, Size, Vec2};
        if !(b.width().is_finite() && b.height().is_finite()) || b.width() < 0.1 || b.height() < 0.1
        {
            self.status = crate::i18n::tr("status.fit_page_nothing");
            return;
        }
        let page = self.page;
        let shapes: Vec<ShapeId> = self
            .doc()
            .page(page)
            .map(|p| {
                p.layers
                    .iter()
                    .flat_map(|l| l.shapes.iter().map(|s| s.id))
                    .collect()
            })
            .unwrap_or_default();
        let mut cmds = vec![Command::ResizePage {
            page,
            size: Size::new(b.width(), b.height()),
        }];
        let shift = Vec2::new(-b.x0, -b.y0);
        if shift.hypot() > 1e-9 && !shapes.is_empty() {
            cmds.push(Command::TransformShapes {
                shapes,
                transform: Affine::translate(shift),
            });
        }
        if let Err(e) = self.engine.run_batch(label, &cmds) {
            self.status = e.to_string();
        }
    }

    // ----- layout ----------------------------------------------------------------

    /// Layout > Insert Page Number: an artistic text "1", "2"... on the
    /// chosen pages (bottom centre), named "page-number". With a master
    /// layer active it is one field on that layer that shows the number
    /// of each page it is drawn on (#807).
    pub fn insert_page_number(&mut self, where_: PageNumberWhere) {
        let pages: Vec<(usize, tracedraw_core::PageId)> = self
            .doc()
            .pages
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.id))
            .collect();
        let active = self.page;
        let master = self
            .active_layer()
            .and_then(|l| self.doc().master.iter().find(|m| m.id == l))
            .map(|m| (m.id, m.scope));
        let mut cmds = Vec::new();
        if let Some((layer, scope)) = master {
            // The field lines up as on the page in view, or on the first
            // page the layer shows on when that is not one of them.
            let i = pages
                .iter()
                .position(|(_, p)| *p == active)
                .filter(|i| scope.applies(*i))
                .or_else(|| (0..pages.len()).find(|i| scope.applies(*i)))
                .unwrap_or(0);
            let size = pages
                .get(i)
                .and_then(|(_, p)| self.doc().page(*p).ok())
                .map(|p| p.size);
            if let Some(size) = size {
                let mut s = self.page_number_text(i, size);
                s.page_number = Some(self.page_numbers.field());
                cmds.push(Command::AddShape { layer, shape: s });
            }
        }
        for (i, pid) in pages.into_iter().filter(|_| master.is_none()) {
            let wanted = match where_ {
                PageNumberWhere::Active => pid == active,
                PageNumberWhere::All => true,
                PageNumberWhere::Odd => i % 2 == 0,
                PageNumberWhere::Even => i % 2 == 1,
            };
            if !wanted {
                continue;
            }
            let Ok(page) = self.doc().page(pid) else {
                continue;
            };
            let Some(layer) = page.layers.last().map(|l| l.id) else {
                continue;
            };
            let size = page.size;
            let s = self.page_number_text(i, size);
            cmds.push(Command::AddShape { layer, shape: s });
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Insert Page Number", &cmds) {
                self.status = e.to_string();
            }
        }
    }

    /// The page number text for the page at `i` of `size`, placed as the
    /// page number settings say.
    fn page_number_text(&mut self, i: usize, size: tracedraw_core::geometry::Size) -> Shape {
        let id = self.engine.new_shape_id();
        let pn = &self.page_numbers;
        let text = pn.label(i);
        let em = pn.size_pt * 25.4 / 72.0;
        let margin = 10.0;
        // Outer corner: left on even (left-hand) pages, right on odd pages.
        let left_page = i % 2 == 1;
        let (origin, align) = match pn.position {
            1 | 3 => {
                let y = if pn.position == 1 {
                    margin
                } else {
                    size.height - margin - em
                };
                if left_page {
                    (Point::new(margin, y), tracedraw_core::TextAlign::Left)
                } else {
                    (
                        Point::new(size.width - margin, y),
                        tracedraw_core::TextAlign::Right,
                    )
                }
            }
            2 => (
                Point::new(size.width / 2.0, size.height - margin - em),
                tracedraw_core::TextAlign::Center,
            ),
            _ => (
                Point::new(size.width / 2.0, margin),
                tracedraw_core::TextAlign::Center,
            ),
        };
        let mut s = Shape::new(
            id,
            ShapeKind::Text {
                spans: vec![TextSpan::new(text, self.text_font.clone(), pn.size_pt)],
                origin,
                frame: None,
                align,
                para: Default::default(),
                on_path: None,
            },
        );
        s.name = Some("page-number".into());
        s.fill = Fill::Solid(Color::BLACK);
        s.stroke = None;
        s
    }

    // ----- order -------------------------------------------------------------------

    pub fn reverse_order(&mut self) {
        if self.selection.len() < 2 {
            return;
        }
        let shapes = self.selection.clone();
        self.run(Command::ReverseOrder { shapes });
    }

    /// Finish Object > Order > In Front Of / Behind after the click.
    pub fn order_relative_to(&mut self, reference: ShapeId, in_front: bool) {
        let shapes: Vec<ShapeId> = self
            .selection
            .iter()
            .copied()
            .filter(|s| *s != reference)
            .collect();
        if shapes.is_empty() {
            return;
        }
        self.run(Command::OrderRelative {
            shapes,
            reference,
            in_front,
        });
    }

    pub fn ungroup_all(&mut self) {
        // Repeat ungroup until no selected object is a group.
        for _ in 0..64 {
            let groups: Vec<ShapeId> = self
                .selected_shapes()
                .iter()
                .filter(|s| matches!(s.kind, ShapeKind::Group { .. }))
                .map(|s| s.id)
                .collect();
            if groups.is_empty() {
                break;
            }
            let mut new_sel: Vec<ShapeId> = self
                .selection
                .iter()
                .copied()
                .filter(|s| !groups.contains(s))
                .collect();
            for g in groups {
                let children: Vec<ShapeId> = self
                    .doc()
                    .find_shape(g)
                    .and_then(|s| match &s.kind {
                        ShapeKind::Group { children } => {
                            Some(children.iter().map(|c| c.id).collect())
                        }
                        _ => None,
                    })
                    .unwrap_or_default();
                self.run(Command::Ungroup { group: g });
                new_sel.extend(children);
            }
            self.selection = new_sel;
        }
    }

    // ----- effects on the stack ------------------------------------------------------

    /// Add or replace a live effect on the selected objects.
    pub fn push_effect(&mut self, effect: Effect, replace_same_kind: bool) {
        let shapes = self.selected_shapes();
        let ids: Vec<ShapeId> = shapes.iter().map(|s| s.id).collect();
        let mut cmds = Vec::new();
        for s in shapes {
            let mut effects = s.effects.clone();
            let same = |e: &Effect| std::mem::discriminant(e) == std::mem::discriminant(&effect);
            match effects.iter().position(same).filter(|_| replace_same_kind) {
                // Replaced where it was, so the effects keep their order.
                Some(i) => {
                    effects[i] = effect.clone();
                    let mut first = true;
                    effects.retain(|e| {
                        if !same(e) {
                            return true;
                        }
                        std::mem::replace(&mut first, false)
                    });
                }
                None => effects.push(effect.clone()),
            }
            cmds.push(Command::SetEffects {
                shape: s.id,
                effects,
            });
        }
        if cmds.is_empty() {
            return;
        }
        // A slider dragged or a field typed in a panel or on the bar: the
        // changes of one effect on the same objects, with nothing else in
        // between, are one undo step.
        let key = (
            self.engine.revision(),
            ids,
            format!("{:?}", std::mem::discriminant(&effect)),
        );
        if replace_same_kind
            && self.engine.undo_label() == Some("Effect")
            && self.effect_edit_step.as_ref() == Some(&key)
        {
            let _ = self.engine.undo();
        }
        if let Err(e) = self.engine.run_batch("Effect", &cmds) {
            self.status = e.to_string();
            return;
        }
        self.effect_edit_step = Some((self.engine.revision(), key.1, key.2));
    }

    pub fn remove_effects_of_kind(&mut self, like: &Effect) {
        let shapes = self.selected_shapes();
        let mut cmds = Vec::new();
        for s in shapes {
            let effects: Vec<Effect> = s
                .effects
                .iter()
                .filter(|e| std::mem::discriminant(*e) != std::mem::discriminant(like))
                .cloned()
                .collect();
            if effects.len() != s.effects.len() {
                cmds.push(Command::SetEffects {
                    shape: s.id,
                    effects,
                });
            }
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Clear Effect", &cmds) {
                self.status = e.to_string();
            }
        }
    }

    /// Object > Clear Effect: remove every live effect, shadow and opacity.
    pub fn clear_effects(&mut self) {
        let shapes = self.selected_shapes();
        let mut cmds = Vec::new();
        for s in shapes {
            if !s.effects.is_empty() {
                cmds.push(Command::SetEffects {
                    shape: s.id,
                    effects: Vec::new(),
                });
            }
            if s.shadow.is_some() {
                cmds.push(Command::SetShadow {
                    shapes: vec![s.id],
                    shadow: None,
                });
            }
            if s.opacity < 1.0 {
                cmds.push(Command::SetOpacity {
                    shapes: vec![s.id],
                    opacity: 1.0,
                });
            }
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Clear Effect", &cmds) {
                self.status = e.to_string();
            }
        }
    }

    /// Effects > Flatten Effects: bake the stack into ordinary objects.
    pub fn flatten_effects(&mut self) {
        self.flatten_effects_with(false);
    }

    /// Break Contour, Blend or Extrude Group Apart (Ctrl+K, the docker's
    /// Break apart): as Flatten Effects, the steps then grouped beside the
    /// object.
    pub fn break_effect_groups(&mut self) {
        self.flatten_effects_with(true);
    }

    fn flatten_effects_with(&mut self, group_steps: bool) {
        let shapes = self.selected_shapes();
        let mut cmds = Vec::new();
        let mut new_sel = Vec::new();
        let mut grouped = Vec::new();
        for s in shapes {
            // A vector object with bitmap effects becomes the bitmap of its
            // picture, in its place.
            if let Some(pic) = s.picture() {
                let id = vec![s.id];
                cmds.extend([
                    Command::SetShapeKind {
                        shape: s.id,
                        kind: pic.kind,
                    },
                    Command::SetFill {
                        shapes: id.clone(),
                        fill: tracedraw_core::Fill::None,
                    },
                    Command::SetStroke {
                        shapes: id.clone(),
                        stroke: None,
                    },
                    Command::SetExtraPaint {
                        shapes: id.clone(),
                        fills: Vec::new(),
                        strokes: Vec::new(),
                    },
                    Command::SetShadow {
                        shapes: id.clone(),
                        shadow: None,
                    },
                    Command::SetEffectStack {
                        shape: s.id,
                        effects: Vec::new(),
                        parked: Vec::new(),
                    },
                    Command::SetOpacity {
                        shapes: id,
                        opacity: 1.0,
                    },
                    Command::SetRasterFx {
                        shape: s.id,
                        fx: None,
                    },
                ]);
                new_sel.push(s.id);
                continue;
            }
            // A bitmap's effects become its pixels.
            if let ShapeKind::Bitmap {
                rect,
                width_px,
                height_px,
                png,
                fx: Some(_),
            } = &s.kind
            {
                cmds.push(Command::SetShapeKind {
                    shape: s.id,
                    kind: ShapeKind::Bitmap {
                        rect: *rect,
                        width_px: *width_px,
                        height_px: *height_px,
                        png: png.clone(),
                        fx: None,
                    },
                });
            }
            if s.effects.is_empty() {
                new_sel.push(s.id);
                continue;
            }
            let Ok((layer, _)) = self.doc().shape(s.id) else {
                continue;
            };
            let layer = layer.id;
            let parts = tracedraw_core::live::break_apart(&s, || self.engine.new_shape_id());
            cmds.push(Command::DeleteShapes { shapes: vec![s.id] });
            let mut steps = Vec::new();
            for p in parts {
                if p.id == s.id {
                    new_sel.push(p.id);
                } else {
                    steps.push(p.id);
                }
                cmds.push(Command::AddShape { layer, shape: p });
            }
            // The steps (contour rings, blend steps, extrusion faces) come
            // out as one group beside the object, so they move together
            // (#424).
            match steps.len() {
                0 => {}
                n if n == 1 || !group_steps => new_sel.extend(steps),
                _ => {
                    cmds.push(Command::Group {
                        shapes: steps.clone(),
                    });
                    grouped.push(steps[0]);
                }
            }
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Flatten Effects", &cmds) {
                self.status = e.to_string();
            }
            // Each group of steps, found by a step it holds.
            for step in grouped {
                let group = self
                    .page_layers()
                    .into_iter()
                    .flat_map(|l| l.shapes.iter())
                    .find(|g| match &g.kind {
                        ShapeKind::Group { children } => children.iter().any(|c| c.id == step),
                        _ => false,
                    })
                    .map(|g| g.id);
                new_sel.extend(group);
            }
            self.selection = new_sel;
        }
    }

    /// Object > Symmetry > Create New Symmetry (Alt+S): one vertical mirror
    /// line at the right edge of the selection. Several selected objects
    /// become one symmetry group (grouped first), so they are mirrored
    /// together and each can still be picked inside it with Ctrl+click.
    /// One undo step.
    pub fn create_symmetry(&mut self) {
        let mut shapes = self.selected_shapes();
        if shapes.is_empty() {
            return;
        }
        let Some(b) = self.selection_bounds() else {
            return;
        };
        let center = Point::new(b.x1, b.center().y);
        let mut first = true;
        if shapes.len() > 1 {
            let ids: Vec<ShapeId> = shapes.iter().map(|s| s.id).collect();
            if let Err(e) = self
                .engine
                .run_with_label(&Command::Group { shapes: ids }, "Symmetry")
            {
                self.status = e.to_string();
                return;
            }
            first = false;
            let Some(g) = self.newest_group() else {
                return;
            };
            self.selection = vec![g];
            shapes = self.selected_shapes();
        }
        for s in shapes {
            let mut effects = s.effects.clone();
            effects.retain(|e| !matches!(e, Effect::Symmetry { .. }));
            effects.push(Effect::Symmetry {
                center,
                angle: 90.0,
                lines: 1,
            });
            let cmd = Command::SetEffects {
                shape: s.id,
                effects,
            };
            let r = if first {
                first = false;
                self.engine.run_with_label(&cmd, "Symmetry")
            } else {
                self.engine.amend(&cmd)
            };
            if let Err(e) = r {
                self.status = e.to_string();
            }
        }
        // Straight into editing it, as Edit Symmetry does.
        self.edit_symmetry();
    }

    /// The group added last on the page's layers (the one a Group
    /// command just made).
    fn newest_group(&self) -> Option<ShapeId> {
        self.page_layers()
            .into_iter()
            .flat_map(|l| &l.shapes)
            .filter(|s| matches!(s.kind, ShapeKind::Group { .. }))
            .max_by_key(|s| s.id)
            .map(|g| g.id)
    }

    /// Objects made while a symmetry is edited (drawn, pasted,
    /// duplicated) join it: the symmetry group takes them in (an object
    /// that was alone with the symmetry becomes a group of it and them,
    /// the symmetry moving to the group). Folded into the step that made
    /// the objects, so one Undo takes both back. The new objects stay
    /// selected, inside the group.
    pub fn join_symmetry(&mut self, ids: &[ShapeId]) {
        let Some(g) = self.symmetry_editing() else {
            return;
        };
        let ids: Vec<ShapeId> = ids
            .iter()
            .copied()
            .filter(|id| self.doc().top_level_of(*id).is_some_and(|t| t != g))
            .collect();
        if ids.is_empty() {
            return;
        }
        let Some(group) = self.doc().find_shape(g).cloned() else {
            return;
        };
        let symmetry: Vec<Effect> = group
            .effects
            .iter()
            .filter(|e| matches!(e, Effect::Symmetry { .. }))
            .cloned()
            .collect();
        let mut members: Vec<ShapeId> = Vec::new();
        let mut cmds: Vec<Command> = Vec::new();
        match &group.kind {
            ShapeKind::Group { children } => {
                // The members come out of the group and into a new one
                // with the newcomers; the group itself goes.
                members.extend(children.iter().map(|c| c.id));
                cmds.push(Command::Ungroup { group: g });
            }
            _ => {
                // The lone object keeps its other effects; the symmetry
                // moves to the group of it and the newcomers.
                members.push(g);
                let rest: Vec<Effect> = group
                    .effects
                    .iter()
                    .filter(|e| !matches!(e, Effect::Symmetry { .. }))
                    .cloned()
                    .collect();
                cmds.push(Command::SetEffects {
                    shape: g,
                    effects: rest,
                });
            }
        }
        // A copy of the lone object came with a symmetry of its own: it
        // goes, the group's mirrors the copy (two would mirror it twice).
        for id in &ids {
            if let Some(n) = self.doc().find_shape(*id) {
                if n.effects
                    .iter()
                    .any(|e| matches!(e, Effect::Symmetry { .. }))
                {
                    cmds.push(Command::SetEffects {
                        shape: *id,
                        effects: n
                            .effects
                            .iter()
                            .filter(|e| !matches!(e, Effect::Symmetry { .. }))
                            .cloned()
                            .collect(),
                    });
                }
            }
        }
        members.extend(ids.iter().copied());
        cmds.push(Command::Group { shapes: members });
        for cmd in &cmds {
            if let Err(e) = self.engine.amend(cmd) {
                self.status = e.to_string();
                return;
            }
        }
        let Some(new) = self.newest_group() else {
            return;
        };
        if let Err(e) = self.engine.amend(&Command::SetEffects {
            shape: new,
            effects: symmetry,
        }) {
            self.status = e.to_string();
        }
        self.symmetry_edit = Some(new);
        self.selection = ids;
    }

    /// Object > Symmetry > Edit Symmetry (Alt+E): the symmetry editing
    /// mode for the first selected object with a symmetry. The original is
    /// outlined in red, its mirror lines show across the window and the
    /// symmetry bar (Finish, mirror lines, centre, angle) floats at the
    /// top left of the drawing window.
    pub fn edit_symmetry(&mut self) {
        if let Some((id, ..)) = self.selected_symmetry() {
            self.symmetry_edit = Some(id);
            // Measured from now on without the copies (as picked).
            self.visual_cache.borrow_mut().1.clear();
            self.select(vec![id]);
            self.status = tr("status.editing_symmetry");
        }
    }

    /// Object > Symmetry > Finish Editing Symmetry (Alt+Shift+E), and the
    /// symmetry bar's Finish. The symmetry is then selected as one object:
    /// the object picked inside it while editing stayed selected, so the
    /// next drag moved that object alone, as if inside a group.
    pub fn finish_symmetry_edit(&mut self) {
        self.symmetry_turning = false;
        self.symmetry_step = None;
        *self.symmetry_doc.borrow_mut() = None;
        if let Some(id) = self.symmetry_edit.take() {
            self.status.clear();
            self.raster.borrow_mut().invalidate();
            // Nothing of the editing stays. The boxes measured while it was
            // edited are the object's own (without the copies): measured
            // again. A group of the symmetry entered by a double click on a
            // member, or the Shape tool that switched to, would still pick
            // a member instead of the whole.
            self.visual_cache.borrow_mut().1.clear();
            let inside = self
                .focus_target()
                .is_some_and(|f| f == id || self.doc().ancestors(f).contains(&id));
            if inside {
                self.exit_focus();
            }
            if self.tool == crate::tools::Tool::Shape {
                self.set_tool(crate::tools::Tool::Pick);
            }
            if self.doc().find_shape(id).is_some() {
                self.select(vec![id]);
            }
        }
    }

    /// The object whose symmetry is being edited, while it still has one;
    /// the mode ends when it is gone or lost its symmetry. A group
    /// remade by an Undo (see [`App::join_symmetry`]) hands the mode to
    /// the page's symmetry object when there is one alone.
    pub fn symmetry_editing(&mut self) -> Option<ShapeId> {
        let id = self.symmetry_edit?;
        let has = |s: &tracedraw_core::Shape| {
            s.effects
                .iter()
                .any(|e| matches!(e, Effect::Symmetry { .. }))
        };
        if self.doc().find_shape(id).is_some_and(has) {
            return Some(id);
        }
        let mut with: Vec<ShapeId> = self
            .page_layers()
            .into_iter()
            .flat_map(|l| &l.shapes)
            .filter(|s| has(s))
            .map(|s| s.id)
            .collect();
        if with.len() == 1 {
            let only = with.remove(0);
            self.symmetry_edit = Some(only);
            return Some(only);
        }
        self.symmetry_edit = None;
        None
    }

    /// Object > Symmetry > Break Symmetry Link (Alt+X): the copies become
    /// objects of their own, no longer following the original.
    pub fn break_symmetry_link(&mut self) {
        self.finish_symmetry_edit();
        self.flatten_effects();
    }

    /// The symmetry effect of the first selected object.
    pub fn selected_symmetry(&self) -> Option<(ShapeId, Point, f64, u8)> {
        self.selected_shapes().iter().find_map(|s| {
            s.effects.iter().find_map(|e| match e {
                Effect::Symmetry {
                    center,
                    angle,
                    lines,
                } => Some((s.id, *center, *angle, *lines)),
                _ => None,
            })
        })
    }

    /// Object > Symmetry > Remove Symmetry: the copies disappear.
    pub fn remove_symmetry(&mut self) {
        for s in self.selected_shapes() {
            if !s
                .effects
                .iter()
                .any(|e| matches!(e, Effect::Symmetry { .. }))
            {
                continue;
            }
            let effects = s
                .effects
                .iter()
                .filter(|e| !matches!(e, Effect::Symmetry { .. }))
                .cloned()
                .collect();
            self.run(Command::SetEffects {
                shape: s.id,
                effects,
            });
        }
    }

    /// Object > Add Perspective: a perspective effect with the corners at
    /// the bounding box (edit them with the Shape tool).
    pub fn add_perspective(&mut self) {
        for s in self.selected_shapes() {
            let b = s.local_path().bounding_box();
            let corners = [
                Point::new(b.x0, b.y0),
                Point::new(b.x1, b.y0),
                Point::new(b.x1, b.y1),
                Point::new(b.x0, b.y1),
            ];
            let mut effects = s.effects.clone();
            effects.retain(|e| !matches!(e, Effect::Perspective { .. }));
            effects.push(Effect::Perspective { corners });
            self.run(Command::SetEffects {
                shape: s.id,
                effects,
            });
        }
        self.set_tool(crate::tools::Tool::Shape);
    }

    /// Remove any transparency from the selection.
    pub fn clear_transparency(&mut self) {
        let shapes = self.selection.clone();
        let mut cmds = Vec::new();
        for id in &shapes {
            let Some(s) = self.doc().find_shape(*id).cloned() else {
                continue;
            };
            let effects: Vec<Effect> = s
                .effects
                .iter()
                .filter(|e| !matches!(e, Effect::Transparency { .. }))
                .cloned()
                .collect();
            cmds.push(Command::SetEffects {
                shape: *id,
                effects,
            });
        }
        cmds.push(Command::SetOpacity {
            shapes,
            opacity: 1.0,
        });
        let _ = self.engine.run_batch("Clear Transparency", &cmds);
    }

    /// Object > Clip to Frame > Edit Contents: take the contents out so they can
    /// be moved and edited; Finish Editing puts them back in the frame.
    pub fn edit_clip_frame(&mut self) {
        let Some(clip) = self
            .selected_shapes()
            .into_iter()
            .find(|s| matches!(s.kind, ShapeKind::ClipFrame { .. }))
        else {
            return;
        };
        let ShapeKind::ClipFrame { contents, .. } = &clip.kind else {
            return;
        };
        let ids: Vec<ShapeId> = contents.iter().map(|c| c.id).collect();
        // What is on the page besides: everything else that comes along
        // while editing (the members of a content ungrouped, objects
        // drawn or pasted) is contents too (#537).
        let outside: std::collections::HashSet<ShapeId> = self
            .page_layers()
            .iter()
            .flat_map(|l| &l.shapes)
            .map(|s| s.id)
            .filter(|id| *id != clip.id)
            .collect();
        let before = self.engine.last_step_id();
        self.run(Command::ExtractContents { clip: clip.id });
        self.mark_clip_edit_step(before, clip.id, &outside, true);
        self.clip_frame_edit = Some((clip.id, outside));
        self.select(ids);
        self.status = tr("status.editing_clip_frame");
    }

    /// Note the step just run (when one was) as the one that began
    /// (`begins`) or finished editing `frame`'s contents.
    fn mark_clip_edit_step(
        &mut self,
        before: Option<u64>,
        frame: ShapeId,
        outside: &std::collections::HashSet<ShapeId>,
        begins: bool,
    ) {
        let Some(step) = self.engine.last_step_id().filter(|s| Some(*s) != before) else {
            return;
        };
        // Marks of steps gone from the history (undone and replaced, or
        // too old) are of no more use.
        let engine = &self.engine;
        self.clip_edit_marks
            .retain(|m| m.engine != engine.instance() || engine.step_in_effect(m.step));
        self.clip_edit_marks.push(ClipEditMark {
            engine: engine.instance(),
            step,
            frame,
            outside: outside.clone(),
            begins,
        });
    }

    /// After undo (`undone`) or redo of `step`: undoing the step that
    /// began editing a clip frame's contents leaves the edit, with the
    /// frame as it was; redoing it enters the edit again. Undoing the
    /// step that finished the edit goes back into it, redoing it leaves.
    /// So undo inside the edit never reaches the drawing behind it while
    /// the edit still shows (#851).
    pub fn clip_edit_across(&mut self, step: Option<u64>, undone: bool) {
        let Some(step) = step else {
            return;
        };
        let engine = self.engine.instance();
        let Some(mark) = self
            .clip_edit_marks
            .iter()
            .find(|m| m.engine == engine && m.step == step)
            .cloned()
        else {
            return;
        };
        if self.doc().find_shape(mark.frame).is_none() {
            return;
        }
        if mark.begins != undone {
            self.clip_frame_edit = Some((mark.frame, mark.outside));
            let contents = self.clip_edit_contents();
            self.select(contents);
            self.status = tr("status.editing_clip_frame");
        } else {
            self.clip_frame_edit = None;
            self.select(vec![mark.frame]);
        }
    }

    /// The object is out of reach while a clip frame's contents are
    /// edited: the frame itself and everything that was on the page
    /// besides it (#851).
    pub fn outside_clip_edit(&self, id: ShapeId) -> bool {
        match &self.clip_frame_edit {
            Some((frame, outside)) => id == *frame || outside.contains(&id),
            None => false,
        }
    }

    /// Object > Clip to Frame > Finish Editing Contents: place the (surviving) contents
    /// back inside the frame.
    pub fn finish_clip_frame_edit(&mut self) {
        let contents = self.clip_edit_contents();
        let Some((frame, outside)) = self.clip_frame_edit.take() else {
            return;
        };
        if self.doc().find_shape(frame).is_none() {
            return;
        }
        if contents.is_empty() {
            return;
        }
        let before = self.engine.last_step_id();
        self.place_inside(contents, frame);
        self.mark_clip_edit_step(before, frame, &outside, false);
        self.select(vec![frame]);
    }

    /// The contents of the clip frame being edited: the page's objects
    /// that were not there besides it when editing began, so a content
    /// ungrouped stays in as its members, and objects drawn or pasted
    /// meanwhile go in (#537).
    pub fn clip_edit_contents(&self) -> Vec<ShapeId> {
        let Some((frame, outside)) = &self.clip_frame_edit else {
            return Vec::new();
        };
        self.page_layers()
            .iter()
            .flat_map(|l| &l.shapes)
            .map(|s| s.id)
            .filter(|id| id != frame && !outside.contains(id))
            .collect()
    }

    /// The clip frame bar's Select Contents: the frame's contents become
    /// the selection (as Ctrl+click picks them), with their round
    /// handles.
    pub fn select_clip_contents(&mut self, clip: ShapeId) {
        let ids: Vec<ShapeId> = match self.doc().find_shape(clip).map(|s| &s.kind) {
            Some(ShapeKind::ClipFrame { contents, .. }) => contents.iter().map(|c| c.id).collect(),
            _ => Vec::new(),
        };
        if !ids.is_empty() {
            self.select(ids);
        }
    }

    /// The clip frame bar's Fit Contents: the contents moved (and
    /// scaled) to the frame's box as `mode` says; one undo step.
    pub fn fit_clip_contents(&mut self, clip: ShapeId, mode: crate::ui::clip_bar::FitContents) {
        let (frame_bounds, ids) = match self.doc().find_shape(clip) {
            Some(s) => match &s.kind {
                ShapeKind::ClipFrame { contents, .. } => (
                    self.doc()
                        .shape_on_page(clip)
                        .map(|c| c.bounds())
                        .unwrap_or_default(),
                    contents.iter().map(|c| c.id).collect::<Vec<_>>(),
                ),
                _ => return,
            },
            None => return,
        };
        let Some(contents_bounds) = ids
            .iter()
            .filter_map(|id| self.doc().shape_on_page(*id).map(|s| s.bounds()))
            .reduce(|a, b| a.union(b))
        else {
            return;
        };
        let t = fit_contents_transform(contents_bounds, frame_bounds, mode);
        if t == Affine::IDENTITY {
            return;
        }
        if let Err(e) = self.engine.run_batch(
            "Fit Contents",
            &[Command::TransformShapes {
                shapes: ids,
                transform: t,
            }],
        ) {
            self.status = e.to_string();
        }
    }

    /// What the renderer draws in full while the rest fades: the object
    /// in focus (focus mode), or the frame and the contents being edited
    /// (Edit Contents).
    pub fn render_focus(&self) -> Option<(Vec<ShapeId>, f32)> {
        if let Some((frame, _)) = &self.clip_frame_edit {
            let mut all = vec![*frame];
            all.extend(self.clip_edit_contents());
            return Some((all, CLIP_EDIT_VISIBILITY));
        }
        self.focus_target()
            .map(|id| (vec![id], self.focus_visibility()))
    }

    /// The clip frame under `p` (page) that is not part of the selection
    /// being moved, for a drop into it.
    pub fn frame_under_pointer(&self, p: Point) -> Option<ShapeId> {
        // Without a clip frame on the page there is nothing to drop on;
        // this runs every frame of a move, so the hit test below (which
        // asks each object for its top-level object) is skipped.
        let frames = self
            .page_layers()
            .iter()
            .flat_map(|l| &l.shapes)
            .any(|s| matches!(s.kind, ShapeKind::ClipFrame { .. }));
        if !frames {
            return None;
        }
        let doc = self.doc();
        let moved = |id: ShapeId| {
            self.selection.contains(&id)
                || doc
                    .top_level_of(id)
                    .is_some_and(|t| self.selection.contains(&t))
        };
        let hit = self.hit_test_where(p, moved)?;
        let top = doc.top_level_of(hit)?;
        matches!(
            doc.find_shape(top).map(|s| &s.kind),
            Some(ShapeKind::ClipFrame { .. })
        )
        .then_some(top)
    }

    /// The selection dropped on a clip frame with K held: the frame's
    /// contents give way to it (they are deleted); one undo step.
    pub fn replace_clip_contents(&mut self, contents: Vec<ShapeId>, frame: ShapeId) {
        let old: Vec<ShapeId> = match self.doc().find_shape(frame).map(|s| &s.kind) {
            Some(ShapeKind::ClipFrame { contents, .. }) => contents.iter().map(|c| c.id).collect(),
            _ => return,
        };
        let mut cmds = Vec::new();
        if !old.is_empty() {
            cmds.push(Command::ExtractContents { clip: frame });
            cmds.push(Command::DeleteShapes { shapes: old });
        }
        cmds.push(Command::PlaceInside { contents, frame });
        if let Err(e) = self.engine.run_batch("Place Inside Frame", &cmds) {
            self.status = format!("ClipFrame: {e}");
        }
        self.select(vec![frame]);
    }

    /// Whether a clip frame's contents move with the frame (the default).
    pub fn clip_frame_locked(&self, id: ShapeId) -> bool {
        self.doc()
            .find_shape(id)
            .map(|s| {
                !s.data
                    .iter()
                    .any(|(k, v)| k == "clip_frame.unlocked" && v == "1")
            })
            .unwrap_or(true)
    }

    /// Object > Clip to Frame > Lock Contents to Frame: toggle for the selected clips.
    pub fn toggle_clip_frame_lock(&mut self) {
        for s in self.selected_shapes() {
            if !matches!(s.kind, ShapeKind::ClipFrame { .. }) {
                continue;
            }
            let locked = self.clip_frame_locked(s.id);
            let mut data = s.data.clone();
            data.retain(|(k, _)| k != "clip_frame.unlocked");
            if locked {
                data.push(("clip_frame.unlocked".into(), "1".into()));
            }
            self.run(Command::SetObjectData { shape: s.id, data });
        }
    }

    /// Object > Clip Frame > Opacity Mask and Invert Mask: the selected
    /// clip frames as opacity masks (`on`), inverted or not, or as hard
    /// clips again; one undo step.
    pub fn set_opacity_masks(&mut self, on: bool, invert: bool) {
        let cmds: Vec<Command> = self
            .selected_shapes()
            .iter()
            .filter(|s| matches!(s.kind, ShapeKind::ClipFrame { .. }))
            .map(|s| Command::SetOpacityMask {
                shape: s.id,
                mask: on.then_some(tracedraw_core::document::OpacityMask { invert }),
            })
            .collect();
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Opacity Mask", &cmds) {
            self.status = e.to_string();
        }
    }

    /// After moving unlocked clip frames by `t`, keep their contents in place.
    pub fn compensate_unlocked_clip_frames(&mut self, ids: &[ShapeId], t: Affine) {
        let inv = t.inverse();
        for id in ids {
            if self.clip_frame_locked(*id) {
                continue;
            }
            let Some(s) = self.doc().find_shape(*id).cloned() else {
                continue;
            };
            let ShapeKind::ClipFrame { frame, contents } = s.kind else {
                continue;
            };
            // The clip already carries t * T; contents need T^-1 t^-1 T applied.
            let new_t = s.transform;
            let old_t = inv * new_t;
            let fix = new_t.inverse() * old_t;
            let contents = contents
                .into_iter()
                .map(|mut c| {
                    c.absorb(fix);
                    c
                })
                .collect();
            self.run(Command::SetShapeKind {
                shape: *id,
                kind: ShapeKind::ClipFrame { frame, contents },
            });
        }
    }

    /// Object > Create > Arrowhead: one selected closed curve becomes a custom
    /// arrowhead, listed in the Outline editors and kept in the settings.
    pub fn create_arrowhead_from_selection(&mut self) {
        let shapes = self.selected_shapes();
        let [s] = shapes.as_slice() else {
            self.status = tr("status.arrowhead_needs_curve");
            return;
        };
        let path = s.page_path();
        if path.elements().len() < 3 {
            self.status = tr("status.arrowhead_needs_curve");
            return;
        }
        let n = self.settings.custom_arrowheads.len() + 1;
        let name = s
            .name
            .clone()
            .unwrap_or_else(|| trf("docker.custom_arrowhead_n", &[("n", &n.to_string())]));
        let head = tracedraw_core::Arrowhead::from_shape_path(&path, name);
        if matches!(head, tracedraw_core::Arrowhead::None) {
            self.status = tr("status.arrowhead_needs_curve");
            return;
        }
        self.settings.custom_arrowheads.push(head);
        self.save_settings();
        self.status = tr("status.arrowhead_created");
    }

    /// Text > Align to Baseline Grid: toggle for the selected paragraph frames.
    pub fn toggle_baseline_grid(&mut self) {
        let g = self.doc().metadata.grid.baseline_spacing.max(0.1);
        for s in self.text_shapes() {
            if let ShapeKind::Text { frame: Some(_), .. } = &s.kind {
                self.edit_paragraph_style(s.id, |p| {
                    p.baseline_grid_mm = if p.baseline_grid_mm > 0.0 { 0.0 } else { g };
                });
            }
        }
    }

    /// Object > Create > Vector Pattern Fill: the selected objects become a
    /// vector tile, set as the default fill for new objects.
    pub fn create_vector_pattern_from_selection(&mut self) {
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            return;
        }
        let fill = Fill::Pattern(tracedraw_core::Pattern::vector_from_shapes(&shapes));
        self.set_default_properties(tracedraw_core::DefaultKind::Graphic, |p| p.fill = fill);
        self.status = tr("status.pattern_created");
    }

    /// Edit > Copy Properties From: fill and outline of `source` to the selection.
    pub fn copy_properties_from(&mut self, source: ShapeId) {
        let Some(src) = self.doc().find_shape(source).cloned() else {
            return;
        };
        let shapes: Vec<ShapeId> = self
            .selection
            .iter()
            .copied()
            .filter(|s| *s != source)
            .collect();
        if shapes.is_empty() {
            return;
        }
        let cmds = vec![
            Command::SetFill {
                shapes: shapes.clone(),
                fill: src.fill.clone(),
            },
            Command::SetStroke {
                shapes,
                stroke: src.stroke.clone(),
            },
        ];
        if let Err(e) = self.engine.run_batch("Copy Properties", &cmds) {
            self.status = e.to_string();
        }
    }

    /// Object > Fit Objects to Path: the last selected object is the path;
    /// the others are placed along it with the Fit Objects to Path panel's
    /// settings (#1027), with the copies the panel asks for (#1091,
    /// #1196). Copies go into their original's layer and, with the
    /// objects placed, become the selection.
    pub fn fit_objects_to_path(&mut self) {
        let shapes = self.selected_shapes();
        let plan = self.fit_path_plan(&shapes);
        if plan.is_empty() {
            return;
        }
        let mut cmds = Vec::with_capacity(plan.len());
        let mut placed = Vec::with_capacity(plan.len());
        let mut copied = false;
        for p in &plan {
            if !p.copy {
                cmds.push(Command::TransformShapes {
                    shapes: vec![p.source],
                    transform: p.transform,
                });
                placed.push(p.source);
                continue;
            }
            let Some(src) = shapes.iter().find(|s| s.id == p.source) else {
                continue;
            };
            let top = self
                .doc()
                .ancestors(src.id)
                .first()
                .copied()
                .unwrap_or(src.id);
            let Some(layer) = self
                .doc()
                .locate(top)
                .ok()
                .map(|(l, _)| l)
                .or_else(|| self.active_layer())
            else {
                continue;
            };
            let id = self.engine.new_shape_id();
            let mut c = crate::app::reid_pub(src.clone(), id, &mut self.engine);
            c.transform_by(p.transform);
            cmds.push(Command::AddShape { layer, shape: c });
            placed.push(id);
            copied = true;
        }
        // Group all objects: the ones placed that are not inside a group
        // already, in the same step (#1091).
        let grouped: Vec<ShapeId> = placed
            .iter()
            .copied()
            .filter(|id| {
                !shapes.iter().any(|s| s.id == *id) || self.doc().ancestors(*id).is_empty()
            })
            .collect();
        let group = self.fit_path.group && grouped.len() >= 2;
        if group {
            cmds.push(Command::Group {
                shapes: grouped.clone(),
            });
        }
        if let Err(e) = self.engine.run_batch("Fit Objects to Path", &cmds) {
            self.status = e.to_string();
            return;
        }
        if group {
            if let Some(g) = self.doc().ancestors(grouped[0]).first().copied() {
                self.select(vec![g]);
                return;
            }
        }
        if copied {
            self.select(placed);
        }
    }

    /// [`crate::fit_path::fit_to_path_plan_with`] for `shapes` with the panel's settings and the
    /// centres of rotation moved with the Pick tool.
    fn fit_path_plan(&self, shapes: &[Shape]) -> Vec<crate::fit_path::Placement> {
        let pivot = |s: &Shape| {
            self.pivots
                .single
                .get(&s.id)
                .map(|local| s.transform * *local)
        };
        crate::fit_path::fit_to_path_plan_with(shapes, &self.fit_path, &pivot)
    }

    /// The Fit Objects to Path panel for the objects selected now: values
    /// set for other objects (or in another drawing) are not carried
    /// over; the panel starts again from the defaults, with no preview
    /// until a value is entered (#1136).
    pub fn fit_path_follow_selection(&mut self) {
        let drawing = self.docs.get(self.active_doc).map(|d| d.drawing);
        let same = self
            .fit_path_for
            .as_ref()
            .is_some_and(|(d, sel)| *d == drawing && *sel == self.selection);
        if !same {
            self.fit_path = FitPathSettings::default();
            self.fit_path_edited = false;
            self.fit_path_for = Some((drawing, self.selection.clone()));
        }
    }

    /// The outlines the selected objects (and the copies) would have
    /// after [`Self::fit_objects_to_path`], for the panel's preview; empty
    /// when the selection cannot be fitted or is too large to preview
    /// cheaply, and until a value was entered for these objects (#1136).
    /// Worked out again only when the settings, the selection or the
    /// drawing change; past [`FIT_PREVIEW_OUTLINES`] objects each shows
    /// as its box.
    pub fn fit_objects_to_path_preview(&mut self) -> &[BezPath] {
        if !self.fit_path_edited || self.selection.len() < 2 || self.selection.len() > 200 {
            self.fit_path_preview = None;
            return &[];
        }
        let key = FitPreviewKey {
            settings: self.fit_path,
            revision: self.engine.revision(),
            drawing: self.docs.get(self.active_doc).map(|d| d.drawing),
            selection: self.selection.clone(),
        };
        if self
            .fit_path_preview
            .as_ref()
            .is_none_or(|(k, _)| *k != key)
        {
            let shapes = self.selected_shapes();
            let plan = self.fit_path_plan(&shapes);
            let boxes = plan.len() > FIT_PREVIEW_OUTLINES;
            let paths = plan
                .into_iter()
                .filter_map(|p| {
                    let s = shapes.iter().find(|s| s.id == p.source)?;
                    Some(if boxes {
                        p.transform * kurbo::Shape::to_path(&s.bounds(), 0.1)
                    } else {
                        p.transform * s.page_path()
                    })
                })
                .collect();
            self.fit_path_preview = Some((key, paths));
        }
        self.fit_path_preview
            .as_ref()
            .map_or(&[], |(_, p)| p.as_slice())
    }

    /// The Join Curves docker: join the open ends of the selected curves
    /// (and the open subpaths inside them) that lie within the gap
    /// tolerance, nearest first, with the chosen joint. The result takes
    /// the place and properties of the curve selected last. Returns the
    /// number of joints made.
    pub fn join_curves(&mut self) -> usize {
        let shapes: Vec<Shape> = self
            .selected_shapes_target_last()
            .into_iter()
            .filter(|s| matches!(s.kind, ShapeKind::Path { .. }))
            .collect();
        let Some(last) = shapes.last() else {
            return 0;
        };
        let keep = last.id;
        let keep_transform = last.transform;
        let paths: Vec<BezPath> = shapes.iter().map(|s| s.page_path()).collect();
        let j = self.join_settings;
        let mode = match j.mode {
            JoinKind::Extend => tracedraw_core::join::JoinMode::Extend,
            JoinKind::Chamfer => tracedraw_core::join::JoinMode::Chamfer,
            JoinKind::Fillet => tracedraw_core::join::JoinMode::Fillet(j.radius),
            JoinKind::Bezier => tracedraw_core::join::JoinMode::Bezier,
        };
        let (joined, n) = tracedraw_core::join::join_curves(&paths, mode, j.gap);
        if n == 0 {
            self.status = tr("status.join_gap_too_small");
            return 0;
        }
        let closed = joined
            .elements()
            .iter()
            .filter(|e| matches!(e, kurbo::PathEl::MoveTo(_)))
            .count()
            == joined
                .elements()
                .iter()
                .filter(|e| matches!(e, kurbo::PathEl::ClosePath))
                .count();
        let others: Vec<ShapeId> = shapes
            .iter()
            .map(|s| s.id)
            .filter(|id| *id != keep)
            .collect();
        // The joined outline is in page space: the kept curve's transform
        // goes back to none (its effects stay where they are).
        let mut cmds = vec![
            Command::TransformShapes {
                shapes: vec![keep],
                transform: keep_transform.inverse(),
            },
            Command::SetShapeKind {
                shape: keep,
                kind: ShapeKind::Path {
                    path: joined,
                    closed,
                },
            },
        ];
        cmds.extend(self.effects_as_now(keep));
        if !others.is_empty() {
            cmds.push(Command::DeleteShapes { shapes: others });
        }
        if let Err(e) = self.engine.run_batch("Join Curves", &cmds) {
            self.status = e.to_string();
            return 0;
        }
        self.selection = vec![keep];
        n
    }

    pub fn toggle_overprint(&mut self, fill: bool) {
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            return;
        }
        let current = if fill {
            shapes.iter().any(|s| s.overprint_fill)
        } else {
            shapes.iter().any(|s| s.overprint_outline)
        };
        let ids: Vec<ShapeId> = shapes.iter().map(|s| s.id).collect();
        self.run(Command::SetOverprint {
            shapes: ids,
            fill: fill.then_some(!current),
            outline: (!fill).then_some(!current),
        });
    }

    // ----- bitmaps -------------------------------------------------------------------

    pub(crate) fn bitmap_shapes(
        &self,
    ) -> Vec<(ShapeId, Rect, u32, u32, tracedraw_core::SharedBytes)> {
        self.selected_shapes()
            .into_iter()
            .filter_map(|s| match s.kind {
                ShapeKind::Bitmap {
                    rect,
                    width_px,
                    height_px,
                    png,
                    fx: _,
                } => Some((s.id, rect, width_px, height_px, png)),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn replace_bitmap(&mut self, id: ShapeId, rect: Rect, img: &image::RgbaImage) {
        let Some(png) = crate::bitmap_fx::encode(img) else {
            return;
        };
        self.run(Command::SetShapeKind {
            shape: id,
            kind: ShapeKind::Bitmap {
                rect,
                width_px: img.width(),
                height_px: img.height(),
                png: png.into(),
                fx: None,
            },
        });
    }

    pub fn set_bitmap_mode(&mut self, mode: crate::bitmap_fx::ColorMode) {
        for (id, rect, _, _, png) in self.bitmap_shapes() {
            let Some(img) = crate::bitmap_fx::decode(&png) else {
                continue;
            };
            let out = crate::bitmap_fx::convert_mode(&img, mode);
            self.replace_bitmap(id, rect, &out);
        }
    }

    /// Bitmaps > Inflate Bitmap > Manually Inflate Bitmap: grow every
    /// selected bitmap's canvas to `pw` by `ph` percent of its size (100
    /// or more), centred, with transparent margins; one undo step. The
    /// bitmap keeps its resolution, so it grows on the page too.
    pub fn inflate_bitmaps_by(&mut self, pw: f64, ph: f64) {
        let mut cmds = Vec::new();
        for (id, rect, w, h, png) in self.bitmap_shapes() {
            let Some(img) = crate::bitmap_fx::decode(&png) else {
                continue;
            };
            let grow = |n: u32, pct: f64| -> u32 {
                let pct = if pct.is_finite() {
                    pct.clamp(100.0, 1000.0)
                } else {
                    100.0
                };
                ((n as f64 * pct / 100.0).round() as u32).max(n)
            };
            let (nw, nh) = (grow(w, pw), grow(h, ph));
            if (nw, nh) == (w, h) {
                continue;
            }
            let out = crate::bitmap_fx::inflate_to(&img, nw, nh);
            let Some(png) = crate::bitmap_fx::encode(&out) else {
                continue;
            };
            let (mx, my) = (
                rect.width() / w.max(1) as f64,
                rect.height() / h.max(1) as f64,
            );
            let (left, top) = ((nw - w) / 2, (nh - h) / 2);
            let x0 = rect.x0 - left as f64 * mx;
            let y0 = rect.y0 - top as f64 * my;
            cmds.push(Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Bitmap {
                    rect: Rect::new(x0, y0, x0 + nw as f64 * mx, y0 + nh as f64 * my),
                    width_px: nw,
                    height_px: nh,
                    png: png.into(),
                    fx: None,
                },
            });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Inflate Bitmap", &cmds) {
            self.status = e.to_string();
        }
    }

    /// The Bitmap Mask docker's Apply: hide (or keep only) the colours of
    /// the slots in use in the selected bitmaps. True when a bitmap changed.
    pub fn apply_bitmap_mask(&mut self) -> bool {
        let slots = self.mask.active();
        if slots.is_empty() {
            return false;
        }
        let hide = self.mask.hide;
        let mut changed = false;
        for (id, rect, _, _, png) in self.bitmap_shapes() {
            let Some(img) = crate::bitmap_fx::decode(&png) else {
                continue;
            };
            let out = crate::bitmap_fx::mask_bitmap(&img, &slots, hide);
            self.replace_bitmap(id, rect, &out);
            changed = true;
        }
        changed
    }

    /// "Select colours from bitmap": fill the slots with the colours the
    /// first selected bitmap uses most, each in use.
    pub fn mask_colors_from_bitmap(&mut self) -> bool {
        let Some((_, _, _, _, png)) = self.bitmap_shapes().into_iter().next() else {
            return false;
        };
        let Some(img) = crate::bitmap_fx::decode(&png) else {
            return false;
        };
        let colors = crate::bitmap_fx::main_colors(&img, crate::bitmap_fx::MASK_SLOTS);
        for (slot, c) in self.mask.slots.iter_mut().zip(colors) {
            slot.on = true;
            slot.color = Color::rgb8(c[0], c[1], c[2]);
        }
        true
    }

    /// Object > Convert to Bitmap: rasterise the selection at `dpi`.
    /// Rasterise the selection: PNG bytes, pixel size and the bounds in page space.
    pub fn render_selection_png(
        &self,
        dpi: f64,
        transparent: bool,
    ) -> Option<(Vec<u8>, u32, u32, Rect)> {
        self.render_selection_png_with(dpi, transparent, true)
    }

    /// [`App::render_selection_png`] with or without smoothed edges.
    pub fn render_selection_png_with(
        &self,
        dpi: f64,
        transparent: bool,
        anti_alias: bool,
    ) -> Option<(Vec<u8>, u32, u32, Rect)> {
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            return None;
        }
        let bounds = self.selection_bounds()?;
        // Render only the selection: a temporary document with those shapes.
        let mut tmp = tracedraw_core::Document::new(
            "tmp",
            Size::new(bounds.width().max(0.1), bounds.height().max(0.1)),
        );
        tmp.symbols = self.doc().symbols.clone();
        let l = tmp.pages[0].layers[0].id;
        for s in &shapes {
            let mut c = s.clone();
            c.transform_by(Affine::translate((-bounds.x0, -bounds.y0)));
            if let Ok(layer) = tmp.layer_mut(l) {
                layer.shapes.push(c);
            }
        }
        let page = tmp.pages[0].id;
        let zoom = dpi / 25.4;
        let w = (bounds.width() * zoom).ceil().max(1.0) as u32;
        let h = (bounds.height() * zoom).ceil().max(1.0) as u32;
        let view = tracedraw_render::ViewTransform {
            zoom,
            origin_x: 0.0,
            origin_y: h as f64,
        };
        let mut pm = tracedraw_render::render_page(
            &tmp,
            page,
            &tracedraw_render::RenderOptions {
                width: w,
                height: h,
                view,
                preview: None,
                wireframe: false,
                anti_alias,
                draft: false,
                focus: None,
                ..tracedraw_render::RenderOptions::default()
            },
        )?;
        if !transparent {
            let mut white = tiny_skia_white(w, h);
            white.draw_pixmap(
                0,
                0,
                pm.as_ref(),
                &Default::default(),
                tiny_skia::Transform::identity(),
                None,
            );
            pm = white;
        }
        let png = pm.encode_png().ok()?;
        Some((png, w, h, bounds))
    }

    #[cfg(test)]
    pub fn convert_to_bitmap(&mut self, dpi: f64, transparent: bool) {
        self.convert_to_bitmap_with(&crate::bitmap_modes::ConvertOptions {
            dpi,
            transparent,
            ..Default::default()
        });
    }

    /// Bitmaps > Convert to Bitmap: the selection becomes one bitmap with
    /// the dialog's resolution, colour mode, dithering, anti-aliasing,
    /// background and overprint, in one undo step.
    pub fn convert_to_bitmap_with(&mut self, o: &crate::bitmap_modes::ConvertOptions) {
        let shapes = self.selected_shapes();
        let Some(layer) = self.active_layer() else {
            return;
        };
        let dpi = if o.dpi.is_finite() {
            o.dpi.clamp(36.0, 2400.0)
        } else {
            300.0
        };
        let Some((png, w, h, bounds)) =
            self.render_selection_png_with(dpi, o.transparent, o.anti_alias)
        else {
            return;
        };
        let png = match o.mode {
            crate::bitmap_modes::ConvertMode::Rgb | crate::bitmap_modes::ConvertMode::Cmyk => png,
            _ => match crate::bitmap_fx::decode(&png)
                .map(|img| crate::bitmap_modes::convert_pixels(&img, o))
                .and_then(|img| crate::bitmap_fx::encode(&img))
            {
                Some(p) => p,
                None => return,
            },
        };
        let id = self.engine.new_shape_id();
        let mut bitmap = Shape::new(
            id,
            ShapeKind::Bitmap {
                rect: bounds,
                width_px: w,
                height_px: h,
                png: png.into(),
                fx: None,
            },
        );
        bitmap.fill = Fill::None;
        bitmap.stroke = None;
        bitmap.overprint_fill = o.overprint_black;
        let ids: Vec<ShapeId> = shapes.iter().map(|s| s.id).collect();
        let cmds = vec![
            Command::DeleteShapes { shapes: ids },
            Command::AddShape {
                layer,
                shape: bitmap,
            },
        ];
        if let Err(e) = self.engine.run_batch("Convert to Bitmap", &cmds) {
            self.status = e.to_string();
        }
        self.selection = vec![id];
    }

    /// Object > Create > Pattern Fill: the selection becomes a bitmap pattern
    /// tile, set as the default fill for new objects.
    pub fn create_pattern_from_selection(&mut self) {
        let Some((png, width_px, height_px, bounds)) = self.render_selection_png(300.0, true)
        else {
            return;
        };
        let fill = Fill::Pattern(tracedraw_core::style::Pattern::Bitmap {
            png: png.into(),
            width_px,
            height_px,
            size_mm: bounds.width().max(0.1),
        });
        self.set_default_properties(tracedraw_core::DefaultKind::Graphic, |p| p.fill = fill);
        self.status = tr("status.pattern_created");
    }

    // ----- tracing ---------------------------------------------------------------------

    /// Bitmaps > Quick Trace: the preset and choices of Options > Global >
    /// Bitmap Tracing.
    pub fn quick_trace(&mut self) {
        let prefs = &self.settings.tracing;
        let preset = prefs.quick_preset;
        let settings = prefs.settings(preset);
        self.trace_bitmaps(&settings);
        self.last_trace = Some((preset, settings));
    }

    /// The Trace dialog on `preset` (Centerline and Outline Trace).
    pub fn open_trace(&mut self, preset: crate::trace::Preset) {
        self.dialog = crate::ui::dialogs::Dialog::Trace(crate::ui::dialogs::TraceState::new(
            preset,
            &self.settings.tracing,
        ));
    }

    /// Repeat Last Trace: the selected bitmaps traced as the last trace
    /// was, without the dialog.
    pub fn repeat_last_trace(&mut self) {
        if let Some((_, settings)) = self.last_trace.clone() {
            self.trace_bitmaps(&settings);
        }
    }

    /// Open Last Trace Settings: the Trace dialog with the last trace's
    /// preset and settings.
    pub fn open_last_trace_settings(&mut self) {
        let Some((preset, settings)) = self.last_trace.clone() else {
            return;
        };
        let mut st = crate::ui::dialogs::TraceState::new(preset, &self.settings.tracing);
        st.settings = settings;
        self.dialog = crate::ui::dialogs::Dialog::Trace(st);
    }

    pub fn trace_bitmaps(&mut self, settings: &crate::trace::Settings) {
        let bitmaps = self.bitmap_shapes();
        if bitmaps.is_empty() {
            return;
        }
        let Some(layer) = self.active_layer() else {
            return;
        };
        let mut cmds = Vec::new();
        let mut new_sel = Vec::new();
        for (id, rect, w, h, png) in bitmaps {
            let Some(img) = crate::bitmap_fx::decode(&png) else {
                continue;
            };
            let Ok((_, src)) = self.doc().shape(id) else {
                continue;
            };
            let transform = src.transform;
            let traced = crate::trace::trace(&img, settings);
            let stroke_px = crate::trace::last_stroke_width_px();
            let mut children = Vec::new();
            for t in traced {
                let sid = self.engine.new_shape_id();
                let path = crate::trace::to_rect(&t.path, w, h, rect);
                let mut s = Shape::new(
                    sid,
                    ShapeKind::Path {
                        path,
                        closed: t.closed,
                    },
                );
                if t.closed {
                    s.fill = Fill::Solid(t.color);
                    s.stroke = None;
                } else {
                    s.fill = Fill::None;
                    let mm = stroke_px * rect.width() / w.max(1) as f64;
                    s.stroke = Some(tracedraw_core::Stroke::new(t.color, mm.max(0.1)));
                }
                children.push(s);
            }
            if children.is_empty() {
                continue;
            }
            let gid = self.engine.new_shape_id();
            let mut group = Shape::new(gid, ShapeKind::Group { children });
            group.transform = transform;
            group.name = Some(tr("trace.result_name"));
            cmds.push(Command::AddShape {
                layer,
                shape: group,
            });
            new_sel.push(gid);
            if settings.delete_original {
                cmds.push(Command::DeleteShapes { shapes: vec![id] });
            }
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Trace Bitmap", &cmds) {
                self.status = e.to_string();
            }
            self.selection = new_sel;
        }
    }

    // ----- text ------------------------------------------------------------------------

    pub fn text_shapes(&self) -> Vec<Shape> {
        self.selected_shapes()
            .into_iter()
            .filter(|s| matches!(s.kind, ShapeKind::Text { .. }))
            .collect()
    }

    /// Text > Edit Text (Ctrl+Shift+T, the text bar's button, the object's
    /// menu): the Edit Text dialog with the first selected text's
    /// characters (#355). A double click edits the text in place instead.
    pub fn open_edit_text_dialog(&mut self) {
        if let Some(s) = self.text_shapes().first() {
            let ShapeKind::Text { spans, .. } = &s.kind else {
                return;
            };
            self.finish_text();
            self.dialog = crate::ui::dialogs::Dialog::EditText {
                shape: s.id,
                text: tracedraw_core::spans_text(spans),
            };
        }
    }

    /// The text object `id` gets the characters `text`, as one undo step
    /// ("Edit Text"): the characters before and after the part that
    /// changed keep their own formatting, and the new part takes the
    /// formatting of the character before it.
    pub fn set_text_content(&mut self, id: ShapeId, text: &str) {
        let Some(ShapeKind::Text {
            spans,
            origin,
            frame,
            align,
            para,
            on_path,
        }) = self.doc().find_shape(id).map(|s| s.kind.clone())
        else {
            return;
        };
        let old: Vec<char> = tracedraw_core::spans_text(&spans).chars().collect();
        let new: Vec<char> = text.chars().collect();
        if old == new {
            return;
        }
        let mut spans = spans;
        // Each run of characters removed or added, in place: what stays
        // keeps its formatting and an addition takes the formatting of
        // the character before it.
        let mut pos = 0;
        for op in char_edits(&old, &new) {
            match op {
                CharEdit::Keep(n) => pos += n,
                CharEdit::Remove(n) => tracedraw_core::spans_delete(&mut spans, pos, pos + n),
                CharEdit::Add(s) => {
                    tracedraw_core::spans_insert(&mut spans, pos, &s);
                    pos += s.chars().count();
                }
            }
        }
        let kind = ShapeKind::Text {
            spans,
            origin,
            frame,
            align,
            para,
            on_path,
        };
        if let Err(e) = self
            .engine
            .run_with_label(&Command::SetShapeKind { shape: id, kind }, "Edit Text")
        {
            self.status = e.to_string();
            return;
        }
        if self.is_linked_frame(id) {
            self.reflow_chain(id);
        }
    }

    /// A double click on a text: start in-place editing of the selected text.
    pub fn edit_selected_text(&mut self) {
        if let Some(s) = self.text_shapes().first() {
            let id = s.id;
            self.set_tool(crate::tools::Tool::Text);
            self.begin_text_edit(id);
        }
    }

    /// Insert a string at the caret of the text being edited.
    pub fn insert_text(&mut self, code: &str) {
        if self.text_edit.is_some() {
            self.text_insert(code);
        }
    }

    /// Text > Convert: artistic to paragraph text or back.
    pub fn toggle_text_kind(&mut self) {
        for s in self.text_shapes() {
            let ShapeKind::Text {
                spans,
                origin,
                frame,
                align,
                para,
                on_path,
            } = s.kind.clone()
            else {
                continue;
            };
            let b = s.bounds();
            let new_frame = match frame {
                Some(_) => None,
                None => Some(Size::new(
                    b.width().max(10.0) + 2.0,
                    b.height().max(5.0) + 2.0,
                )),
            };
            let new_origin = match (frame, new_frame) {
                (None, Some(_)) => {
                    Point::new(origin.x, origin.y - (b.height() - (b.y1 - origin.y)))
                }
                (Some(_), None) => Point::new(
                    origin.x,
                    b.y1 - spans
                        .first()
                        .map(|s| s.size_pt * 25.4 / 72.0)
                        .unwrap_or(5.0),
                ),
                _ => origin,
            };
            self.run(Command::SetShapeKind {
                shape: s.id,
                kind: ShapeKind::Text {
                    spans,
                    origin: new_origin,
                    frame: new_frame,
                    align,
                    para,
                    on_path,
                },
            });
        }
    }

    /// The selected texts Join Lines into Paragraph takes: artistic text,
    /// not on a path nor vertical, at least two.
    pub fn joinable_text_lines(&self) -> Vec<Shape> {
        let lines: Vec<Shape> = self
            .text_shapes()
            .into_iter()
            .filter(|s| {
                matches!(&s.kind, ShapeKind::Text { frame: None, on_path: None, para, .. }
                    if !para.vertical)
            })
            .collect();
        if lines.len() < 2 {
            Vec::new()
        } else {
            lines
        }
    }

    /// Text > Join Lines into Paragraph (#599): the selected lines of
    /// artistic text (a PDF's text comes one object per line) become one
    /// paragraph text in a frame as wide as the widest line, the lines in
    /// reading order (top to bottom) joined by spaces, so typing flows the
    /// words on to the next lines. The first baseline stays where it was
    /// and the lines keep their spacing (the usual distance between their
    /// baselines, in points); a gap of more than half again as much starts
    /// a new paragraph. The lines must be turned and sized alike. One undo
    /// step; the paragraph comes out selected.
    pub fn join_text_lines(&mut self) {
        use tracedraw_core::document::text_outline::TextRequest;
        let lines = self.joinable_text_lines();
        let Some(first) = lines.first() else {
            return;
        };
        let doc = self.doc();
        let parent = doc.parent_transform(first.id).unwrap_or(Affine::IDENTITY);
        let base = first.transform;
        let linear = |a: Affine| {
            let c = a.as_coeffs();
            [c[0], c[1], c[2], c[3]]
        };
        let alike = |a: Affine, b: Affine| {
            let (a, b) = (linear(a), linear(b));
            let scale = a.iter().fold(1e-9_f64, |m, v| m.max(v.abs()));
            a.iter().zip(b).all(|(x, y)| (x - y).abs() <= scale * 1e-6)
        };
        let to_base = base.inverse();
        // Each line in the first line's own space: where its first
        // baseline starts, its right end, and its text.
        struct Line {
            start: Point,
            right: f64,
            spans: Vec<TextSpan>,
        }
        let mut found: Vec<Line> = Vec::new();
        for s in &lines {
            let same_parent = doc
                .parent_transform(s.id)
                .is_some_and(|p| (p.as_coeffs()) == parent.as_coeffs());
            if !same_parent || !alike(s.transform, base) {
                self.status = tr("status.join_lines_unlike");
                return;
            }
            let Some((layout, to_parent)) =
                crate::text_editing::layout_of_text_shape(s, Affine::IDENTITY)
            else {
                continue;
            };
            let Some(&(bx, by)) = layout.baselines.first() else {
                continue;
            };
            let ShapeKind::Text { spans, .. } = &s.kind else {
                continue;
            };
            let m = to_base * to_parent;
            found.push(Line {
                start: m * Point::new(bx, by),
                right: (m * Point::new(layout.bounds.x1, by)).x,
                spans: spans.clone(),
            });
        }
        if found.len() < 2 {
            return;
        }
        found.sort_by(|a, b| {
            b.start
                .y
                .total_cmp(&a.start.y)
                .then(a.start.x.total_cmp(&b.start.x))
        });
        let gaps: Vec<f64> = found
            .windows(2)
            .map(|w| w[0].start.y - w[1].start.y)
            .filter(|g| *g > 1e-6)
            .collect();
        let mut sorted = gaps.clone();
        sorted.sort_by(f64::total_cmp);
        let size_mm = found[0]
            .spans
            .first()
            .map(|s| s.size_pt * tracedraw_text::PT_MM)
            .unwrap_or(5.0);
        let leading = sorted
            .get(sorted.len() / 2)
            .copied()
            .unwrap_or(size_mm * 1.2);
        let mut spans: Vec<TextSpan> = Vec::new();
        for (i, line) in found.iter().enumerate() {
            if i > 0 {
                let gap = found[i - 1].start.y - line.start.y;
                let sep = if gap > leading * 1.5 { "\n" } else { " " };
                if let Some(last) = spans.last_mut() {
                    if !last.text.ends_with(char::is_whitespace) {
                        last.text.push_str(sep);
                    }
                }
            }
            spans.extend(line.spans.iter().cloned());
        }
        let left = found.iter().map(|l| l.start.x).fold(f64::MAX, f64::min);
        let right = found.iter().map(|l| l.right).fold(f64::MIN, f64::max);
        // A little room so the lines this program measured still fit.
        let width = ((right - left) * 1.02 + 0.5).max(5.0);
        let ShapeKind::Text { align, para, .. } = &first.kind else {
            return;
        };
        let mut para = para.clone();
        para.leading_unit = tracedraw_core::LeadingUnit::Points;
        para.leading_pct = leading / tracedraw_text::PT_MM;
        let top_base = found[0].start.y;
        let low_base = found.iter().map(|l| l.start.y).fold(f64::MAX, f64::min);
        // Room for the lines there are and two more as the text grows.
        let height = (top_base - low_base) + size_mm + leading * 2.0;
        let frame = Size::new(width, height);
        let laid = tracedraw_text::fonts().layout(&TextRequest {
            spans: &spans,
            frame: Some(frame),
            align: *align,
            para: &para,
            on_path: None,
        });
        // The first baseline where the top line's was.
        let (bx, by) = laid.baselines.first().copied().unwrap_or((0.0, -size_mm));
        let origin = Point::new(left - bx, top_base - by - height);
        let kind = ShapeKind::Text {
            spans,
            origin,
            frame: Some(frame),
            align: *align,
            para,
            on_path: None,
        };
        let keep = first.id;
        let rest: Vec<ShapeId> = lines.iter().skip(1).map(|s| s.id).collect();
        let cmds = vec![
            Command::SetShapeKind { shape: keep, kind },
            Command::DeleteShapes { shapes: rest },
        ];
        match self.engine.run_batch("Join Lines into Paragraph", &cmds) {
            Ok(()) => self.selection = vec![keep],
            Err(e) => self.status = e.to_string(),
        }
    }

    pub fn fit_text_to_frame(&mut self) {
        for s in self.text_shapes() {
            if let ShapeKind::Text { frame: Some(_), .. } = &s.kind {
                self.edit_paragraph_style(s.id, |p| p.fit_to_frame = !p.fit_to_frame);
            }
        }
    }

    /// The characters the paragraph settings act on in text `id`: the
    /// selection (or the caret) while it is being edited; `None`, every
    /// paragraph, when the whole object is selected.
    pub fn paragraph_range(&self, id: ShapeId) -> Option<(usize, usize)> {
        self.text_edit
            .as_ref()
            .filter(|te| te.shape == id)
            .map(|te| te.selection())
    }

    /// The alignment and paragraph style the paragraph controls show for a
    /// text: the paragraph at the caret (the first selected one) while it
    /// is being edited, else its first paragraph. `None` for other shapes.
    pub fn paragraph_style_of(
        &self,
        s: &Shape,
    ) -> Option<(tracedraw_core::TextAlign, tracedraw_core::ParagraphStyle)> {
        let ShapeKind::Text {
            spans, align, para, ..
        } = &s.kind
        else {
            return None;
        };
        let index = self.paragraph_range(s.id).map_or(0, |(a, _)| {
            tracedraw_core::paragraphs::paragraph_of(spans, a)
        });
        let f = tracedraw_core::paragraphs::paragraph_format(spans, *align, para, index);
        let mut p = para.clone();
        let a = f.apply_to(&mut p);
        Some((a, p))
    }

    /// Give text `id` the paragraph style `para`, as edited from what
    /// [`App::paragraph_style_of`] showed: the frame settings (columns,
    /// fitting, baseline grid, vertical alignment, background) go to the
    /// object, the paragraph settings that changed to the paragraphs of
    /// [`App::paragraph_range`] (each keeps the others). One undo step.
    pub fn set_paragraph_style(&mut self, id: ShapeId, para: tracedraw_core::ParagraphStyle) {
        let Some(s) = self.doc().find_shape(id).cloned() else {
            return;
        };
        let Some((align, old)) = self.paragraph_style_of(&s) else {
            return;
        };
        let mut kind = s.kind.clone();
        if let ShapeKind::Text { para: own, .. } = &mut kind {
            own.columns = para.columns;
            own.gutter = para.gutter;
            own.fit_to_frame = para.fit_to_frame;
            own.baseline_grid_mm = para.baseline_grid_mm;
            own.vertical_align = para.vertical_align;
            own.vertical = para.vertical;
            own.frame_fill = para.frame_fill.clone();
        }
        let before = tracedraw_core::ParagraphFormat::of(align, &old);
        let after = tracedraw_core::ParagraphFormat::of(align, &para);
        if before != after {
            tracedraw_core::paragraphs::edit_paragraph_formats(
                &mut kind,
                self.paragraph_range(id),
                |f| f.take_changes(&before, &after),
            );
        }
        if kind != s.kind {
            self.run(Command::SetShapeKind { shape: id, kind });
        }
    }

    /// Change the paragraph style of text `id` with `f`, starting from
    /// what [`App::paragraph_style_of`] shows (see
    /// [`App::set_paragraph_style`]).
    pub fn edit_paragraph_style(
        &mut self,
        id: ShapeId,
        f: impl FnOnce(&mut tracedraw_core::ParagraphStyle),
    ) {
        let Some(s) = self.doc().find_shape(id).cloned() else {
            return;
        };
        let Some((_, mut p)) = self.paragraph_style_of(&s) else {
            return;
        };
        f(&mut p);
        self.set_paragraph_style(id, p);
    }

    /// The texts Text > Use Hyphenation acts on: the one being edited,
    /// else the selected ones.
    fn hyphenation_targets(&self) -> Vec<ShapeId> {
        if let Some(te) = &self.text_edit {
            return vec![te.shape];
        }
        self.selected_shapes()
            .iter()
            .filter(|s| matches!(s.kind, ShapeKind::Text { .. }))
            .map(|s| s.id)
            .collect()
    }

    /// Whether Text > Use Hyphenation shows checked: the text being
    /// edited, or every selected text, hyphenates; with no text, whether
    /// new text does (#470).
    pub fn uses_hyphenation(&self) -> bool {
        let ids = self.hyphenation_targets();
        if ids.is_empty() {
            return self.text_hyphenation;
        }
        ids.iter().all(|id| {
            self.doc()
                .find_shape(*id)
                .and_then(|s| self.paragraph_style_of(s))
                .is_some_and(|(_, p)| p.hyphenate)
        })
    }

    /// Text > Use Hyphenation: the selected texts hyphenate (or stop, when
    /// they all did), as in the reference program; before, only text made
    /// afterwards did, so turning it on for a selected text changed
    /// nothing (#255). While a text is edited it is that text's setting
    /// (#470); with no text selected, the setting new text takes.
    pub fn toggle_use_hyphenation(&mut self) {
        let ids = self.hyphenation_targets();
        if ids.is_empty() {
            self.text_hyphenation = !self.text_hyphenation;
            return;
        }
        let on = !self.uses_hyphenation();
        for id in ids {
            self.edit_paragraph_style(id, |p| p.hyphenate = on);
        }
    }

    /// Whether any selected object wraps paragraph text, and the wrap
    /// style of the first one.
    pub fn wrap_state(&self) -> (bool, tracedraw_core::TextWrap) {
        let shapes = self.selected_shapes();
        (
            shapes.iter().any(|s| s.wrap_text),
            shapes.first().map(|s| s.wrap_style).unwrap_or_default(),
        )
    }

    /// Wrap paragraph text around the selected objects with this style,
    /// keeping the offset of the first one.
    pub fn set_wrap_style(
        &mut self,
        mode: tracedraw_core::WrapMode,
        side: tracedraw_core::WrapSide,
    ) {
        let (_, current) = self.wrap_state();
        let style = tracedraw_core::TextWrap {
            mode,
            side,
            ..current
        };
        let shapes = self.selection.clone();
        self.run(Command::SetWrapStyle { shapes, style });
    }

    /// Wrap paragraph text around the selected objects this far from them
    /// (mm), keeping the style of the first one.
    pub fn set_wrap_offset(&mut self, offset: f64) {
        let (_, current) = self.wrap_state();
        let style = tracedraw_core::TextWrap {
            offset: offset.max(0.0),
            ..current
        };
        let shapes = self.selection.clone();
        self.run(Command::SetWrapStyle { shapes, style });
    }

    /// Align the paragraphs of the selected text: those of the selection
    /// while a text is edited, else every paragraph of every selected
    /// text object. One undo step.
    pub fn apply_text_align(&mut self, a: tracedraw_core::TextAlign) {
        self.text_align = a;
        let doc = self.doc();
        let mut cmds: Vec<Command> = Vec::new();
        for s in self
            .selected_shapes()
            .iter()
            .filter(|s| matches!(s.kind, ShapeKind::Text { .. }))
        {
            let mut kind = s.kind.clone();
            // Text typed at a point lines up as a whole; paragraph text
            // takes the alignment in the paragraphs at the caret.
            let point_text = matches!(
                &s.kind,
                ShapeKind::Text {
                    frame: None,
                    on_path: None,
                    ..
                }
            );
            let range = if point_text {
                None
            } else {
                self.paragraph_range(s.id)
            };
            tracedraw_core::paragraphs::edit_paragraph_formats(&mut kind, range, |f| f.align = a);
            if kind == s.kind {
                continue;
            }
            // Text typed at a point lines up about that point, so a new
            // alignment would move it: it moves back so the reference
            // point of the X and Y fields stays where it was. Text in a
            // frame or on a path keeps its frame or path; its lines
            // realign inside.
            let shift = if point_text {
                let before = s.bounds();
                let mut after = s.clone();
                after.kind = kind.clone();
                let after = after.bounds();
                let o = self.object_origin;
                let d = crate::ui::propbar::origin_point(before, o)
                    - crate::ui::propbar::origin_point(after, o);
                // The move in the object's parent's space (a group's).
                let parent = doc.parent_transform(s.id).unwrap_or(Affine::IDENTITY);
                let inv = parent.inverse();
                let local = inv * (Point::ZERO + d) - inv * Point::ZERO;
                (local.hypot() > 1e-9 && local.x.is_finite() && local.y.is_finite())
                    .then_some(local)
            } else {
                None
            };
            cmds.push(Command::SetShapeKind { shape: s.id, kind });
            if let Some(v) = shift {
                cmds.push(Command::TransformShapes {
                    shapes: vec![s.id],
                    transform: Affine::translate(v),
                });
            }
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Text Alignment", &cmds) {
                self.status = e.to_string();
            }
        }
    }

    /// Text > Fit Text to Path from the menu: a text and a path selected
    /// go together at once; otherwise the next click picks the path.
    pub fn begin_fit_text_to_path(&mut self) {
        // A text being typed is finished first, so it goes on the path as
        // typed and the typing does not carry on into a stale layout.
        self.finish_text();
        let shapes = self.selected_shapes();
        let texts = shapes
            .iter()
            .filter(|s| matches!(s.kind, ShapeKind::Text { .. }))
            .count();
        if texts > 0 && texts < shapes.len() {
            self.fit_text_to_path();
            return;
        }
        self.pending_text_path = true;
        self.status = crate::i18n::tr("status.click_text_path");
    }

    /// The path picked for Fit Text to Path: the selected text goes on it,
    /// or with no text selected a new empty text starts on it, ready to
    /// type. On a closed path the text is centred on the point of the
    /// path nearest to `at` (the click), else on its highest point.
    pub fn fit_text_to_picked_path(&mut self, path_id: ShapeId, at: Option<Point>) {
        self.text_onto_path(path_id, at, false);
    }

    /// [`Self::fit_text_to_picked_path`]; with `begin` a new text on a
    /// closed path begins at `at` instead of being centred there (the
    /// Text tool clicked on the path).
    fn text_onto_path(&mut self, path_id: ShapeId, at: Option<Point>, begin: bool) {
        let Ok(curve) = self.doc().shape(path_id).map(|(_, s)| s.clone()) else {
            return;
        };
        if matches!(curve.kind, ShapeKind::Text { .. }) {
            self.status = crate::i18n::tr("status.click_text_path");
            self.pending_text_path = true;
            return;
        }
        let text = self
            .selected_shapes()
            .iter()
            .find(|s| s.id != path_id && matches!(s.kind, ShapeKind::Text { .. }))
            .map(|s| s.id);
        if let Some(t) = text {
            self.selection = vec![t, path_id];
            self.fit_text_to_path_at(at);
            return;
        }
        self.finish_text();
        let (path, closed) = path_for_text_at(&curve.page_path(), at);
        let (align, offset) = if begin && closed && at.is_some() {
            (tracedraw_core::TextAlign::Left, 0.0)
        } else {
            placement_on(&path, closed, self.text_align)
        };
        let span = TextSpan {
            bold: self.text_bold,
            italic: self.text_italic,
            ..TextSpan::new("", self.text_font.clone(), self.text_size_pt)
        };
        let para = tracedraw_core::ParagraphStyle {
            hyphenate: self.text_hyphenation,
            ..Default::default()
        };
        if let Some(id) = self.new_shape(ShapeKind::Text {
            spans: vec![span],
            origin: Point::ZERO,
            frame: None,
            align,
            para,
            on_path: Some(tracedraw_core::TextOnPath {
                path,
                offset,
                distance: 0.0,
                mirror: false,
                ..Default::default()
            }),
        }) {
            self.text_edit = Some(crate::app::TextEdit::at_end(id, 0));
            self.select(vec![id]);
        }
    }

    /// Text > Fit Text to Path: select a text and a curve.
    pub fn fit_text_to_path(&mut self) {
        self.fit_text_to_path_at(None);
    }

    /// Fit the selected text to the selected curve; on a closed curve the
    /// text is centred on the point nearest to `at`, else on its highest
    /// point, and as far from the curve as `at` is (as the preview while
    /// picking the curve showed it).
    fn fit_text_to_path_at(&mut self, at: Option<Point>) {
        let shapes = self.selected_shapes();
        let Some(text) = shapes
            .iter()
            .find(|s| matches!(s.kind, ShapeKind::Text { .. }))
        else {
            return;
        };
        let Some(curve) = shapes
            .iter()
            .find(|s| !matches!(s.kind, ShapeKind::Text { .. }))
        else {
            return;
        };
        let Some(kind) = crate::text_path::fit_kind(text, curve, at) else {
            return;
        };
        self.run(Command::SetShapeKind {
            shape: text.id,
            kind,
        });
        self.selection = vec![text.id];
    }

    /// The Text tool clicked on the outline of a path (not a text): a new
    /// text starts on that path. True when it did.
    pub fn start_text_on_path_at(&mut self, p: Point) -> bool {
        let tolerance = 4.0 / f64::from(self.view.zoom).max(1e-6);
        let Some(id) = self.hit_test(p) else {
            return false;
        };
        let Ok((_, s)) = self.doc().shape(id) else {
            return false;
        };
        if matches!(
            s.kind,
            ShapeKind::Text { .. } | ShapeKind::Bitmap { .. } | ShapeKind::Group { .. }
        ) {
            return false;
        }
        let path = s.page_path();
        let near = path.segments().any(|seg| {
            kurbo::ParamCurveNearest::nearest(&seg, p, 1e-3).distance_sq <= tolerance * tolerance
        });
        if !near {
            return false;
        }
        self.selection.clear();
        self.text_onto_path(id, Some(p), true);
        true
    }

    /// Text > Straighten Text on text fitted to a path: it comes off the
    /// path, unrotated, where the path's text began.
    pub fn straighten_text(&mut self) {
        for s in self.text_shapes() {
            let ShapeKind::Text {
                spans,
                origin,
                frame,
                align,
                para,
                on_path: Some(_),
            } = s.kind.clone()
            else {
                continue;
            };
            let mut cmds = vec![
                Command::SetShapeKind {
                    shape: s.id,
                    kind: ShapeKind::Text {
                        spans,
                        origin,
                        frame,
                        align,
                        para,
                        on_path: None,
                    },
                },
                Command::TransformShapes {
                    shapes: vec![s.id],
                    transform: Affine::translate(s.bounds().origin().to_vec2())
                        * s.transform.inverse(),
                },
            ];
            cmds.extend(self.effects_as_now(s.id));
            if let Err(e) = self.engine.run_batch("Straighten Text", &cmds) {
                self.status = e.to_string();
            }
        }
    }

    pub fn change_case(&mut self, mode: CaseMode) {
        for s in self.text_shapes() {
            let ShapeKind::Text {
                mut spans,
                origin,
                frame,
                align,
                para,
                on_path,
            } = s.kind.clone()
            else {
                continue;
            };
            for sp in spans.iter_mut() {
                sp.text = apply_case(&sp.text, mode);
            }
            self.run(Command::SetShapeKind {
                shape: s.id,
                kind: ShapeKind::Text {
                    spans,
                    origin,
                    frame,
                    align,
                    para,
                    on_path,
                },
            });
        }
    }

    /// Apply the property bar text style (font, size, bold, italic) to
    /// every selected text object (the alignment goes through
    /// [`App::apply_text_align`]).
    pub fn apply_text_style(&mut self) {
        // With table cells selected, their text takes the style.
        if self.table_edit.is_some() {
            self.cells_apply_text_style();
            return;
        }
        // While editing with a selection, only the selected characters
        // change, and only in what the bar changed from the character it
        // shows (a new size keeps the selection's fonts).
        if self.text_edit.as_ref().is_some_and(|te| te.has_selection()) {
            let shown = self.text_span_at_caret();
            // The bar shows the size as seen; the characters keep it before
            // the text's own scale.
            let scale = self
                .text_edit
                .as_ref()
                .map_or(1.0, |te| self.text_scale(te.shape));
            let (font, size, bold, italic, underline) = (
                self.text_font.clone(),
                self.text_size_pt / scale,
                self.text_bold,
                self.text_italic,
                self.text_underline,
            );
            let differs = |f: &dyn Fn(&TextSpan) -> bool| shown.as_ref().is_none_or(f);
            let set_font = differs(&|s| s.font_family != font);
            let set_size = differs(&|s| ((s.size_pt - size) * scale).abs() > 0.005);
            let set_bold = differs(&|s| s.bold != bold);
            let set_italic = differs(&|s| s.italic != italic);
            let set_underline = differs(&|s| s.underline != underline);
            self.text_apply_style(move |sp| {
                if set_font {
                    sp.font_family = font.clone();
                }
                if set_size {
                    sp.size_pt = size;
                }
                if set_bold {
                    sp.bold = bold;
                }
                if set_italic {
                    sp.italic = italic;
                }
                if set_underline {
                    sp.underline = underline;
                }
            });
            return;
        }
        let cmds: Vec<Command> = self
            .selected_shapes()
            .iter()
            .filter_map(|s| match &s.kind {
                ShapeKind::Text {
                    spans,
                    origin,
                    frame,
                    align,
                    para,
                    on_path,
                } => {
                    let mut spans = spans.clone();
                    // The bar's size is the size as seen: before the text's
                    // own scale it is smaller (or larger). A size the bar
                    // shows unchanged leaves each character's as it was.
                    let scale = self.text_scale(s.id);
                    for sp in spans.iter_mut() {
                        sp.font_family = self.text_font.clone();
                        if (sp.size_pt * scale - self.text_size_pt).abs() > 0.005 {
                            sp.size_pt = self.text_size_pt / scale;
                        }
                        sp.bold = self.text_bold;
                        sp.italic = self.text_italic;
                        sp.underline = self.text_underline;
                    }
                    Some(Command::SetShapeKind {
                        shape: s.id,
                        kind: ShapeKind::Text {
                            spans,
                            origin: *origin,
                            frame: *frame,
                            align: *align,
                            para: para.clone(),
                            on_path: on_path.clone(),
                        },
                    })
                }
                _ => None,
            })
            .collect();
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Text Style", &cmds) {
                self.status = e.to_string();
            }
        }
    }

    pub fn apply_text_font(&mut self, family: &str) {
        self.text_font = family.to_string();
        self.apply_text_style();
    }

    // ----- scripts -----------------------------------------------------------------------

    pub fn run_script_file(&mut self) {
        crate::files::Dialog::new()
            .add_filter("JavaScript (*.js)", &["js"])
            .pick_file(self, |app, path| {
                match crate::files::read_to_string(&path) {
                    Ok(src) => app.run_script(&src),
                    Err(e) => app.status = format!("{}: {e}", path.display()),
                }
            });
    }

    pub fn run_script(&mut self, src: &str) {
        let result = crate::scripting::run(self, src);
        match result {
            Ok(out) => {
                self.scripts_output.extend(out);
                self.status = tr("status.script_done");
            }
            Err(e) => {
                self.scripts_output.push(format!("Error: {e}"));
                self.status = trf("status.script_error", &[("e", &e.to_string())]);
            }
        }
    }

    pub fn toggle_recording(&mut self) {
        match self.recording.take() {
            Some(cmds) => {
                let script = crate::scripting::commands_to_script(&cmds);
                self.script_source = script;
                self.show_dockers = true;
                self.docker_tab = crate::app::DockerTab::Scripts;
                self.status = tr("status.recording_stopped");
            }
            None => {
                self.recording = Some(Vec::new());
                self.status = tr("status.recording");
            }
        }
    }

    // ----- palettes -----------------------------------------------------------------------

    pub fn toggle_palette(&mut self, i: usize) {
        if let Some(pos) = self.visible_palettes.iter().position(|p| *p == i) {
            if self.visible_palettes.len() > 1 {
                self.visible_palettes.remove(pos);
            }
        } else {
            self.visible_palettes.push(i);
        }
        self.rebuild_palette();
    }

    /// The main palette strip shows the first visible palette, and each
    /// further visible palette gets a strip of its own (#1164), keeping
    /// how far it was scrolled.
    pub fn rebuild_palette(&mut self) {
        if let Some(i) = self.visible_palettes.first() {
            if let Some(p) = self.palettes.get(*i) {
                self.palette = p.colors.clone();
                self.palette_rgb = crate::ui::palette::as_rgb(&self.palette);
                self.palette_scroll = 0;
            }
        }
        let old = std::mem::take(&mut self.extra_palettes);
        self.extra_palettes = self
            .visible_palettes
            .iter()
            .skip(1)
            .filter_map(|&index| {
                let p = self.palettes.get(index)?;
                let scroll = old
                    .iter()
                    .find(|r| r.index == index)
                    .map_or(0, |r| r.scroll);
                Some(crate::ui::palette::ExtraRow {
                    index,
                    rgb: crate::ui::palette::as_rgb(&p.colors),
                    colors: p.colors.clone(),
                    scroll,
                })
            })
            .collect();
    }

    pub fn open_palette_file(&mut self) {
        crate::files::Dialog::new()
            .add_filter(
                crate::palette::filter_label(
                    &tr("file.palettes"),
                    crate::palette::PALETTE_EXTENSIONS,
                ),
                crate::palette::PALETTE_EXTENSIONS,
            )
            .pick_file(self, |app, path| {
                match crate::palette::load_palette(&path) {
                    Ok(p) => {
                        app.palettes.push(p);
                        let i = app.palettes.len() - 1;
                        app.visible_palettes.insert(0, i);
                        app.rebuild_palette();
                    }
                    Err(e) => app.status = format!("{}: {e}", path.display()),
                }
            });
    }

    fn colors_of(shapes: &[Shape], out: &mut Vec<Color>) {
        for s in shapes {
            if let Some(c) = s.fill.preview_color() {
                if !out.contains(&c) {
                    out.push(c);
                }
            }
            if let Some(st) = &s.stroke {
                if !out.contains(&st.color) {
                    out.push(st.color);
                }
            }
            match &s.kind {
                ShapeKind::Group { children } => Self::colors_of(children, out),
                ShapeKind::ClipFrame { frame, contents } => {
                    Self::colors_of(std::slice::from_ref(frame), out);
                    Self::colors_of(contents, out);
                }
                _ => {}
            }
        }
    }

    pub fn palette_from_document(&mut self) {
        let mut colors = Vec::new();
        let all: Vec<Shape> = self
            .doc()
            .all_layers()
            .flat_map(|l| l.shapes.iter().cloned())
            .collect();
        Self::colors_of(&all, &mut colors);
        self.run(Command::SetDocumentPalette { colors });
    }

    /// Add from document: every colour used in the drawing joins the
    /// document palette (the colours already there stay first).
    pub fn add_document_colors_to_palette(&mut self) {
        let mut colors = self.doc().palette.clone();
        let all: Vec<Shape> = self
            .doc()
            .all_layers()
            .flat_map(|l| l.shapes.iter().cloned())
            .collect();
        let before = colors.len();
        Self::colors_of(&all, &mut colors);
        if colors.len() != before {
            self.run(Command::SetDocumentPalette { colors });
        }
    }

    /// Reset palette: the document palette keeps only the colours the
    /// drawing still uses.
    pub fn reset_document_palette(&mut self) {
        let mut used = Vec::new();
        let all: Vec<Shape> = self
            .doc()
            .all_layers()
            .flat_map(|l| l.shapes.iter().cloned())
            .collect();
        Self::colors_of(&all, &mut used);
        let colors: Vec<Color> = self
            .doc()
            .palette
            .iter()
            .copied()
            .filter(|c| used.contains(c))
            .collect();
        if colors != self.doc().palette {
            self.run(Command::SetDocumentPalette { colors });
            self.doc_palette_current = None;
        }
    }

    /// Ctrl+click on a palette colour: a tenth of it goes into the uniform
    /// fill of each selected object.
    pub fn mix_into_fill(&mut self, c: Color) {
        let cmds: Vec<Command> = self
            .selected_shapes()
            .iter()
            .filter_map(|s| match s.fill {
                Fill::Solid(old) => Some(Command::SetFill {
                    shapes: vec![s.id],
                    fill: Fill::Solid(crate::ui::palette::mix(old, c, 0.1)),
                }),
                _ => None,
            })
            .collect();
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Fill", &cmds) {
            self.status = e.to_string();
        }
    }

    pub fn palette_from_selection(&mut self) {
        let mut colors = self.doc().palette.clone();
        Self::colors_of(&self.selected_shapes(), &mut colors);
        self.run(Command::SetDocumentPalette { colors });
    }

    // ----- step and repeat ---------------------------------------------------------------

    /// The Step and Repeat values belong to the objects they were set
    /// for: when the selection is other objects (a new object picked, or
    /// the copies the last Apply made), the copies, distances and modes go
    /// back to their defaults, so a count or a distance typed for one
    /// object is never used again on the next one (#1032).
    pub fn step_repeat_follow_selection(&mut self) {
        let mut sel = self.selection.clone();
        sel.sort();
        sel.dedup();
        if self.step_repeat.for_selection != sel {
            self.step_repeat = StepRepeat {
                for_selection: sel,
                ..StepRepeat::default()
            };
        }
    }

    pub fn step_and_repeat_apply(&mut self) {
        self.step_repeat_follow_selection();
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            return;
        }
        let Some(b) = self.selection_bounds() else {
            return;
        };
        let sr = self.step_repeat.clone();
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
        let Some(layer) = self.active_layer() else {
            return;
        };
        let mut cmds = Vec::new();
        let mut new_sel = Vec::new();
        for k in 1..=sr.copies.max(1) {
            for s in &shapes {
                let mut c = s.clone();
                c.id = self.engine.new_shape_id();
                c.transform_by(Affine::translate((dx * k as f64, dy * k as f64)));
                new_sel.push(c.id);
                cmds.push(Command::AddShape { layer, shape: c });
            }
        }
        if let Err(e) = self.engine.run_batch("Step and Repeat", &cmds) {
            self.status = e.to_string();
        }
        self.selection = new_sel;
    }
}

fn tiny_skia_white(w: u32, h: u32) -> tiny_skia::Pixmap {
    let mut pm = tiny_skia::Pixmap::new(w.max(1), h.max(1))
        .unwrap_or_else(|| tiny_skia::Pixmap::new(1, 1).expect("1x1 pixmap"));
    pm.fill(tiny_skia::Color::WHITE);
    pm
}

/// The path text is fitted to, in page space: its first subpath, and for a
/// closed one turned to run clockwise (so the letters stand on its
/// outside) and started at the point nearest to `at` (where the path was
/// clicked), or at its highest point without one. Returns the path and
/// whether it is closed.
pub fn path_for_text_at(path: &BezPath, at: Option<Point>) -> (BezPath, bool) {
    use kurbo::{ParamCurve, ParamCurveExtrema, ParamCurveNearest, PathSeg};
    let mut first = BezPath::new();
    let mut closed = false;
    for el in path.elements() {
        match el {
            tracedraw_core::geometry::PathEl::MoveTo(_) if !first.elements().is_empty() => break,
            tracedraw_core::geometry::PathEl::ClosePath => {
                closed = true;
                break;
            }
            e => first.push(*e),
        }
    }
    let mut segs: Vec<PathSeg> = first.segments().collect();
    if let (Some(a), Some(b)) = (segs.first(), segs.last()) {
        if (a.start() - b.end()).hypot() < 1e-6 && segs.len() > 1 {
            closed = true;
        }
    }
    if !closed || segs.is_empty() {
        return (first, false);
    }
    if let (Some(a), Some(b)) = (segs.first(), segs.last()) {
        if (a.start() - b.end()).hypot() >= 1e-6 {
            segs.push(PathSeg::Line(tracedraw_core::geometry::Line::new(
                b.end(),
                a.start(),
            )));
        }
    }
    // Shoelace over the flattened outline: positive runs anticlockwise
    // (y up), whose left side, where letters stand, is the inside.
    let mut area = 0.0;
    let mut prev: Option<Point> = None;
    let mut start: Option<Point> = None;
    tracedraw_core::geometry::flatten(&first, 0.1, &mut |el| match el {
        tracedraw_core::geometry::PathEl::MoveTo(p) => {
            start = Some(p);
            prev = Some(p);
        }
        tracedraw_core::geometry::PathEl::LineTo(p) => {
            if let Some(q) = prev {
                area += q.x * p.y - p.x * q.y;
            }
            prev = Some(p);
        }
        _ => {}
    });
    if let (Some(q), Some(p)) = (prev, start) {
        area += q.x * p.y - p.x * q.y;
    }
    if area > 0.0 {
        segs = segs.iter().rev().map(|s| s.reverse()).collect();
    }
    // The start: the point nearest to `at`, or the highest point (the
    // segment and parameter where y is largest).
    let mut best = (0usize, 0.0f64, f64::NEG_INFINITY);
    match at.filter(|p| p.x.is_finite() && p.y.is_finite()) {
        Some(p) => {
            for (i, seg) in segs.iter().enumerate() {
                let n = seg.nearest(p, 1e-6);
                if -n.distance_sq > best.2 {
                    best = (i, n.t, -n.distance_sq);
                }
            }
        }
        None => {
            for (i, seg) in segs.iter().enumerate() {
                let mut ts = vec![0.0, 1.0];
                ts.extend(seg.extrema());
                for t in ts {
                    let y = seg.eval(t).y;
                    if y > best.2 + 1e-9 {
                        best = (i, t, y);
                    }
                }
            }
        }
    }
    let (k, t, _) = best;
    let mut order: Vec<PathSeg> = Vec::new();
    if t < 1.0 - 1e-9 {
        order.push(segs[k].subsegment(t..1.0));
    }
    order.extend(segs[k + 1..].iter().copied());
    order.extend(segs[..k].iter().copied());
    if t > 1e-9 {
        order.push(segs[k].subsegment(0.0..t));
    }
    let mut out = BezPath::new();
    if let Some(s0) = order.first() {
        out.move_to(s0.start());
    }
    for seg in order {
        match seg {
            PathSeg::Line(l) => out.line_to(l.p1),
            PathSeg::Quad(q) => out.quad_to(q.p1, q.p2),
            PathSeg::Cubic(c) => out.curve_to(c.p1, c.p2, c.p3),
        }
    }
    out.close_path();
    (out, true)
}

/// Where text fitted to `path` (its local space) is placed: on a closed
/// path centred on the start (the highest point), on an open one with the
/// text's own alignment from the start. Returns the alignment and offset.
pub fn placement_on(
    path: &BezPath,
    closed: bool,
    align: tracedraw_core::TextAlign,
) -> (tracedraw_core::TextAlign, f64) {
    if !closed {
        return (align, 0.0);
    }
    let total = tracedraw_core::TextOnPath {
        path: path.clone(),
        offset: 0.0,
        distance: 0.0,
        mirror: false,
        ..Default::default()
    }
    .baseline()
    .length();
    // Centred text starts (total - width) / 2 along; half the length more
    // brings its middle round to the start.
    (tracedraw_core::TextAlign::Center, total / 2.0)
}

pub fn apply_case(text: &str, mode: CaseMode) -> String {
    match mode {
        CaseMode::Lower => text.to_lowercase(),
        CaseMode::Upper => text.to_uppercase(),
        CaseMode::Toggle => text
            .chars()
            .map(|c| {
                if c.is_uppercase() {
                    c.to_lowercase().collect::<String>()
                } else {
                    c.to_uppercase().collect::<String>()
                }
            })
            .collect(),
        CaseMode::Title => text
            .split(' ')
            .map(|w| {
                let mut cs = w.chars();
                match cs.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + &cs.as_str().to_lowercase(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
        CaseMode::Sentence => {
            let mut out = String::new();
            let mut start = true;
            for c in text.chars() {
                if start && c.is_alphabetic() {
                    out.extend(c.to_uppercase());
                    start = false;
                } else {
                    out.extend(c.to_lowercase());
                }
                if matches!(c, '.' | '!' | '?' | '\n') {
                    start = true;
                }
            }
            out
        }
    }
}

impl App {
    /// Clear Blend: the selection's blends go (both objects stay).
    pub fn clear_blend(&mut self) {
        self.change_blends("Clear Blend", |_| None);
    }

    /// Blend the two selected objects: the first becomes the start with a
    /// live Blend effect, the second stays an object of its own, the
    /// blend's end (see `blend_link`).
    pub fn blend_selection(
        &mut self,
        steps: u32,
        accel_objects: f64,
        accel_colors: f64,
        rotation: f64,
    ) {
        let shapes = self.selected_shapes();
        if shapes.len() != 2 {
            return;
        }
        let start = &shapes[0];
        let Some(end) = self.doc().shape_on_page(shapes[1].id) else {
            return;
        };
        let mut effects = start.effects.clone();
        effects.retain(|e| !matches!(e, Effect::Blend { .. }));
        // The steps lie between the two objects in the stacking order.
        let mut options = self.blend_options;
        options.steps_below = self.stacked_below(end.id, start.id);
        effects.push(Effect::Blend {
            end: Box::new(crate::blend_link::end_copy(&end)),
            steps,
            accel_objects,
            accel_colors,
            rotation,
            path: None,
            rotate_on_path: false,
            end_id: Some(end.id),
            options,
        });
        let cmds = vec![Command::SetEffects {
            shape: start.id,
            effects,
        }];
        if let Err(e) = self.engine.run_batch("Blend", &cmds) {
            self.status = e.to_string();
        }
        self.selection = vec![start.id];
    }

    /// Attach a path to the blend of the selected object (click on a curve).
    pub fn set_blend_path(&mut self, curve: ShapeId) {
        let Some(path_shape) = self.doc().find_shape(curve).cloned() else {
            return;
        };
        let path = path_shape.page_path();
        for s in self.selected_shapes() {
            if s.id == curve {
                continue;
            }
            let mut effects = s.effects.clone();
            for e in effects.iter_mut() {
                if let Effect::Blend { path: p, .. } = e {
                    *p = Some(path.clone());
                }
            }
            self.run(Command::SetEffects {
                shape: s.id,
                effects,
            });
        }
    }

    pub fn create_symbol_from_selection(&mut self) {
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            return;
        }
        let Some(layer) = self.active_layer() else {
            return;
        };
        // One symbol per selection: group the shapes when there are several.
        let b = self.selection_bounds().unwrap_or_default();
        let mut def = if shapes.len() == 1 {
            shapes[0].clone()
        } else {
            let mut g = Shape::new(
                ShapeId(0),
                ShapeKind::Group {
                    children: shapes.clone(),
                },
            );
            g.fill = Fill::None;
            g
        };
        // Symbol definition is placed at the origin.
        def.transform_by(Affine::translate((-b.x0, -b.y0)));
        def.id = ShapeId(0);
        let index = self.doc().symbols.len();
        let name = format!("Symbol {}", index + 1);
        let id = self.engine.new_shape_id();
        let mut inst = Shape::new(id, ShapeKind::SymbolInstance { index });
        inst.transform = Affine::translate((b.x0, b.y0));
        inst.fill = Fill::None;
        inst.stroke = None;
        let ids: Vec<ShapeId> = shapes.iter().map(|s| s.id).collect();
        let cmds = vec![
            Command::AddSymbol {
                symbol: tracedraw_core::Symbol { name, shape: def },
            },
            Command::DeleteShapes { shapes: ids },
            Command::AddShape { layer, shape: inst },
        ];
        if let Err(e) = self.engine.run_batch("New Symbol", &cmds) {
            self.status = e.to_string();
        }
        self.selection = vec![id];
    }

    pub fn insert_symbol_instance(&mut self, index: usize) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let c = self.page_rect().center();
        let id = self.engine.new_shape_id();
        let mut inst = Shape::new(id, ShapeKind::SymbolInstance { index });
        inst.transform = Affine::translate(c.to_vec2());
        inst.fill = Fill::None;
        inst.stroke = None;
        self.run(Command::AddShape { layer, shape: inst });
        self.selection = vec![id];
    }

    /// Symbols docker > Revert to objects: replace selected instances by
    /// copies of their definition.
    pub fn revert_symbol_instances(&mut self) {
        let symbols = self.doc().symbols.clone();
        let mut cmds = Vec::new();
        let mut new_sel = Vec::new();
        for s in self.selected_shapes() {
            if let ShapeKind::SymbolInstance { index } = s.kind {
                let Some(def) = symbols.get(index) else {
                    continue;
                };
                let Ok((layer, _)) = self.doc().shape(s.id) else {
                    continue;
                };
                let layer = layer.id;
                let mut copy = def.shape.clone();
                copy.id = self.engine.new_shape_id();
                copy.absorb(s.transform);
                new_sel.push(copy.id);
                cmds.push(Command::DeleteShapes { shapes: vec![s.id] });
                cmds.push(Command::AddShape { layer, shape: copy });
            }
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Revert Symbol", &cmds);
            self.selection = new_sel;
        }
    }
}

impl App {
    /// Whether text set in `family` draws with an installed or carried
    /// font of that family. A face named with its weight ("Titillium Web
    /// SemiBold") is drawn by its family's face of that weight, so it is
    /// not missing when the family is there (#892).
    pub fn font_installed(&self, family: &str) -> bool {
        self.font_families.iter().any(|g| g == family)
            || tracedraw_text::split_weight(family)
                .is_some_and(|(base, _)| self.font_families.iter().any(|g| g == base))
    }

    /// Fonts used by the document (in groups, tables and symbols too) that
    /// are neither installed nor carried by a drawing (Font Manager,
    /// missing-font substitution notice).
    pub fn check_missing_fonts(&mut self) {
        let missing: Vec<String> = self
            .doc()
            .font_families_used()
            .into_iter()
            .filter(|f| !self.font_installed(f))
            .collect();
        if !missing.is_empty() {
            self.status = trf("status.missing_fonts", &[("f", &with_stand_ins(&missing))]);
        }
        self.missing_fonts = missing;
    }
}

/// Missing families with the installed font that draws their text, for
/// messages: "Arial (Liberation Sans), Futura (DejaVu Sans)".
pub(crate) fn with_stand_ins(missing: &[String]) -> String {
    missing
        .iter()
        .map(|f| match tracedraw_text::fonts().substitute(f) {
            Some(s) if s != *f => format!("{f} ({s})"),
            _ => f.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Past this many objects (copies included) the Fit Objects to Path
/// preview shows each object as its box.
pub const FIT_PREVIEW_OUTLINES: usize = 200;

/// What the Fit Objects to Path preview was worked out for.
#[derive(Debug, Clone, PartialEq)]
pub struct FitPreviewKey {
    settings: FitPathSettings,
    revision: u64,
    drawing: Option<u64>,
    selection: Vec<ShapeId>,
}

pub use crate::fit_path::{
    FitAlign, FitCopies, FitMeasure, FitOrder, FitPathSettings, FitPlace, FitReference,
    FitRotation, FitSpacing, FitSpin,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol_of(app: &mut App, rect: kurbo::Rect) -> ShapeId {
        let a = app
            .new_shape(ShapeKind::Rect {
                rect,
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![a]);
        app.create_symbol_from_selection();
        app.selection[0]
    }

    fn instance_index(app: &App, id: ShapeId) -> usize {
        match app.doc().find_shape(id).expect("instance").kind {
            ShapeKind::SymbolInstance { index } => index,
            ref k => panic!("{k:?}"),
        }
    }

    /// Step and Repeat values set for one object are not used on the next
    /// one: a new selection, or the copies an Apply made, starts again
    /// from the defaults, so the next Apply makes one copy (#1032).
    #[test]
    fn step_and_repeat_starts_fresh_for_each_object() {
        let mut app = App::headless();
        let rect = |x: f64| ShapeKind::Rect {
            rect: kurbo::Rect::new(x, 0.0, x + 10.0, 10.0),
            radius: 0.0,
            corners: None,
        };
        let a = app.new_shape(rect(0.0)).expect("a");
        let b = app.new_shape(rect(100.0)).expect("b");
        let count = |app: &App| {
            app.doc()
                .all_layers()
                .map(|l| l.shapes.len())
                .sum::<usize>()
        };
        let before = count(&app);
        app.select(vec![a]);
        app.step_repeat_follow_selection();
        app.step_repeat.copies = 3;
        app.step_repeat.dx = 20.0;
        app.step_repeat.mode_x = 1;
        app.step_and_repeat_apply();
        assert_eq!(count(&app), before + 3);
        assert_eq!(app.selection.len(), 3);
        // The copies are selected: the values went back to the defaults.
        app.step_repeat_follow_selection();
        assert_eq!(
            app.step_repeat,
            StepRepeat {
                for_selection: {
                    let mut s = app.selection.clone();
                    s.sort();
                    s
                },
                ..StepRepeat::default()
            }
        );
        // A new object picked gets one copy at the default offset.
        app.select(vec![b]);
        app.step_and_repeat_apply();
        assert_eq!(count(&app), before + 4);
        let copy = app
            .doc()
            .find_shape(app.selection[0])
            .expect("copy")
            .bounds();
        let orig = app.doc().find_shape(b).expect("b").bounds();
        assert!((copy.x0 - orig.x0 - StepRepeat::default().dx).abs() < 1e-9);
        assert!((copy.y0 - orig.y0).abs() < 1e-9);
    }

    /// New Symbol is one step to undo: the symbol goes with its instance.
    #[test]
    fn undoing_new_symbol_removes_the_symbol() {
        let mut app = App::headless();
        symbol_of(&mut app, kurbo::Rect::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(app.doc().symbols.len(), 1);
        app.undo();
        assert!(app.doc().symbols.is_empty());
        app.redo();
        assert_eq!(app.doc().symbols.len(), 1);
    }

    /// An instance copied in one drawing and pasted in another brings its
    /// symbol along, and shows the same objects there.
    #[test]
    fn an_instance_pasted_in_another_drawing_brings_its_symbol() {
        let mut app = App::headless();
        let inst = symbol_of(&mut app, kurbo::Rect::new(0.0, 0.0, 10.0, 10.0));
        app.select(vec![inst]);
        app.copy();
        let expected = app.doc().symbols[0].shape.clone();
        // The other drawing has a symbol of its own first.
        app.open_document(
            tracedraw_core::Document::new("Second", tracedraw_core::Size::new(100.0, 50.0)),
            None,
        );
        symbol_of(&mut app, kurbo::Rect::new(0.0, 0.0, 30.0, 5.0));
        app.paste();
        let pasted = app.selection[0];
        let index = instance_index(&app, pasted);
        let sym = app.doc().symbols.get(index).expect("the symbol is there");
        assert_eq!(sym.shape.bounds(), expected.bounds());
        assert_eq!(app.doc().symbols.len(), 2);
        // Pasted again: the same symbol, not another copy of it.
        app.paste();
        assert_eq!(instance_index(&app, app.selection[0]), index);
        assert_eq!(app.doc().symbols.len(), 2);
        // In the drawing it came from, it stays an instance of its symbol.
        app.switch_document(0);
        app.paste();
        assert_eq!(instance_index(&app, app.selection[0]), 0);
        assert_eq!(app.doc().symbols.len(), 1);
    }

    /// Insert Page Number with a master layer active puts one field on
    /// that layer, which shows each page's own number where pages are
    /// drawn (screen, print, export) and stays a field when saved (#807);
    /// the pages get nothing of their own.
    #[test]
    fn a_page_number_on_the_master_layer_numbers_every_page() {
        let mut app = App::headless();
        app.add_page();
        app.add_page();
        crate::ui::objects_docker::new_master_layer(&mut app, tracedraw_core::MasterScope::All);
        app.page_numbers.prefix = "p. ".into();
        app.insert_page_number(PageNumberWhere::Active);
        let doc = app.doc().clone();
        assert_eq!(doc.master[0].shapes.len(), 1);
        assert!(doc
            .pages
            .iter()
            .all(|p| p.layers.iter().all(|l| l.shapes.is_empty())));
        let shown: Vec<String> = doc
            .pages
            .iter()
            .map(|p| {
                let layers = doc.layers_drawn_for_page(p.id).expect("layers");
                match &layers[0].shapes[0].kind {
                    ShapeKind::Text { spans, .. } => tracedraw_core::document::spans_text(spans),
                    k => panic!("{k:?}"),
                }
            })
            .collect();
        assert_eq!(shown, ["p. 1", "p. 2", "p. 3"]);
        // Saved and opened again, it is still a field.
        let json = doc.to_json().expect("save");
        let back = tracedraw_core::Document::from_json(&json).expect("open");
        assert_eq!(
            back.master[0].shapes[0].page_number,
            Some(app.page_numbers.field())
        );
    }

    /// While Autofit Page's popup is open the page it would make is the
    /// drawing's bounds and the margin; a value left from frames before
    /// shows nothing (#594).
    /// Lines of artistic text (as a PDF brings them, one object a line)
    /// join into one paragraph text: in reading order, joined by spaces
    /// and a wide gap by a new paragraph, the first baseline where it
    /// was, the lines' spacing kept, every line fitting the frame; text
    /// typed into it flows on to more lines (#599).
    #[test]
    fn lines_of_text_join_into_a_paragraph_that_flows() {
        let mut app = App::headless();
        let mut ids = Vec::new();
        // Made out of order: the joined text reads top to bottom.
        for (text, y) in [
            ("third line", 190.0),
            ("first line here", 200.0),
            ("second line", 195.0),
            ("new paragraph", 180.0),
        ] {
            let id = app
                .new_shape(ShapeKind::Text {
                    spans: vec![TextSpan::new(text, "DejaVu Sans", 10.0)],
                    origin: Point::new(20.0, y),
                    frame: None,
                    align: tracedraw_core::TextAlign::Left,
                    para: tracedraw_core::ParagraphStyle::default(),
                    on_path: None,
                })
                .expect("text");
            ids.push(id);
        }
        app.select(ids.clone());
        app.join_text_lines();
        assert_eq!(app.selection.len(), 1);
        let id = app.selection[0];
        let left = ids
            .iter()
            .filter(|i| app.doc().find_shape(**i).is_some())
            .count();
        assert_eq!(left, 1, "the other lines are gone");
        let s = app.doc().find_shape(id).expect("paragraph").clone();
        let ShapeKind::Text {
            spans, frame, para, ..
        } = &s.kind
        else {
            panic!("{:?}", s.kind);
        };
        assert!(frame.is_some(), "paragraph text");
        assert_eq!(
            tracedraw_core::document::spans_text(spans),
            "first line here second line third line\nnew paragraph"
        );
        assert_eq!(para.leading_unit, tracedraw_core::LeadingUnit::Points);
        assert!((para.leading_pct - 5.0 / tracedraw_text::PT_MM).abs() < 1e-6);
        let (layout, to_page) =
            crate::text_editing::layout_of_text_shape(&s, Affine::IDENTITY).expect("layout");
        assert!(!layout.overflow);
        assert_eq!(
            layout.lines.len(),
            4,
            "each line fits: {:?}",
            layout.baselines
        );
        let (bx, by) = layout.baselines[0];
        let first = to_page * Point::new(bx, by);
        assert!(
            (first.x - 20.0).abs() < 1e-6 && (first.y - 200.0).abs() < 1e-6,
            "{first:?}"
        );
        let second = to_page * Point::new(layout.baselines[1].0, layout.baselines[1].1);
        assert!((first.y - second.y - 5.0).abs() < 1e-6, "{second:?}");
        // More words in the first line push the rest down a line.
        let mut kind = s.kind.clone();
        if let ShapeKind::Text { spans, .. } = &mut kind {
            spans[0]
                .text
                .insert_str(0, "a few more words at the start of ");
        }
        app.run(Command::SetShapeKind { shape: id, kind });
        let s = app.doc().find_shape(id).expect("paragraph").clone();
        let (layout, _) =
            crate::text_editing::layout_of_text_shape(&s, Affine::IDENTITY).expect("layout");
        assert!(layout.lines.len() > 4, "{:?}", layout.baselines);
        // A line on its own does not join.
        app.select(vec![id]);
        assert!(app.joinable_text_lines().is_empty());
    }

    #[test]
    fn the_autofit_preview_is_the_drawing_and_its_margin() {
        let mut app = App::headless();
        app.new_shape(ShapeKind::Rect {
            rect: tracedraw_core::geometry::Rect::new(10.0, 20.0, 60.0, 50.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
        assert!(app.autofit_preview_rect(5).is_none());
        app.autofit_preview = Some((5.0, 5));
        let r = app.autofit_preview_rect(6).expect("page");
        assert!(
            (r.width() - 60.0).abs() < 1e-6 && (r.height() - 40.0).abs() < 1e-6,
            "{r:?}"
        );
        assert!(app.autofit_preview_rect(8).is_none(), "stale");
    }

    /// Repeat Last Trace and Open Last Trace Settings follow the last
    /// trace, whichever way it was made (#452).
    #[test]
    fn the_last_trace_can_be_repeated_and_reopened() {
        let mut app = App::headless();
        let img = image::RgbaImage::from_fn(60, 60, |x, y| {
            let d = ((x as f32 - 30.0).powi(2) + (y as f32 - 30.0).powi(2)).sqrt();
            if d < 18.0 {
                image::Rgba([0, 0, 0, 255])
            } else {
                image::Rgba([255, 255, 255, 255])
            }
        });
        let bmp = app.place_image(img, Some("Disc".into())).expect("bitmap");
        assert!(app.last_trace.is_none());
        app.select(vec![bmp]);
        app.quick_trace();
        let (preset, settings) = app.last_trace.clone().expect("remembered");
        assert_eq!(preset, app.settings.tracing.quick_preset);
        let count = |app: &App| {
            app.doc().pages[0]
                .layers
                .iter()
                .map(|l| l.shapes.len())
                .sum::<usize>()
        };
        let after_first = count(&app);
        app.select(vec![bmp]);
        app.repeat_last_trace();
        assert!(count(&app) > after_first, "traced again");
        app.open_last_trace_settings();
        match &app.dialog {
            crate::ui::dialogs::Dialog::Trace(st) => {
                assert_eq!(st.preset, preset);
                assert_eq!(st.settings, settings);
            }
            _ => panic!("the Trace dialog"),
        }
    }

    #[test]
    fn fit_text_to_path_waits_for_a_path_and_starts_text_on_it() {
        let mut app = App::headless();
        let mut line = tracedraw_core::geometry::BezPath::new();
        line.move_to((10.0, 50.0));
        line.line_to((90.0, 50.0));
        let path = app
            .new_shape(ShapeKind::Path {
                path: line,
                closed: false,
            })
            .expect("path");
        app.selection.clear();
        app.begin_fit_text_to_path();
        assert!(app.pending_text_path);
        app.fit_text_to_picked_path(path, None);
        let id = app.selection[0];
        let on_path = app
            .doc()
            .shape(id)
            .ok()
            .and_then(|(_, s)| match &s.kind {
                ShapeKind::Text { on_path, .. } => on_path.clone(),
                _ => None,
            })
            .expect("text on the path");
        assert_eq!(on_path.path.elements().len(), 2);
        assert!(app.text_edit.is_some());
        // A selected text and path go together at once.
        app.insert_text("abc");
        app.finish_text();
        let text = id;
        app.straighten_text();
        app.selection = vec![text, path];
        app.pending_text_path = false;
        app.begin_fit_text_to_path();
        assert!(!app.pending_text_path);
        let fitted = app.doc().shape(text).ok().is_some_and(|(_, s)| {
            matches!(
                &s.kind,
                ShapeKind::Text {
                    on_path: Some(_),
                    ..
                }
            )
        });
        assert!(fitted);
    }

    use kurbo::ParamCurve as _;

    fn fitted(app: &App, id: ShapeId) -> tracedraw_core::TextOnPath {
        app.doc()
            .shape(id)
            .ok()
            .and_then(|(_, s)| match &s.kind {
                ShapeKind::Text { on_path, .. } => on_path.clone(),
                _ => None,
            })
            .expect("text on a path")
    }

    #[test]
    fn closed_paths_turn_clockwise_and_start_at_the_top() {
        // A kurbo ellipse runs anticlockwise from its right end.
        let e =
            tracedraw_core::geometry::Ellipse::new((90.0, 130.0), (50.0, 30.0), 0.0).to_path(0.01);
        let (path, closed) = path_for_text_at(&e, None);
        assert!(closed);
        let seg = path.segments().next().expect("a segment");
        let start = seg.start();
        assert!(
            (start.x - 90.0).abs() < 0.05 && (start.y - 160.0).abs() < 0.05,
            "{start:?}"
        );
        // Clockwise (y up): it heads right from the top.
        assert!(seg.eval(0.01).x > start.x, "{:?}", seg.eval(0.01));
        // An open path is kept as it runs.
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((50.0, 0.0));
        let (open, closed) = path_for_text_at(&line, None);
        assert!(!closed);
        assert_eq!(open, line);
    }

    #[test]
    fn text_fitted_to_an_ellipse_sits_centred_on_its_top_outside() {
        tracedraw_text::install();
        let mut app = App::headless();
        let e = app
            .new_shape(ShapeKind::Ellipse {
                rect: tracedraw_core::geometry::Rect::new(40.0, 100.0, 140.0, 160.0),
                arc: None,
            })
            .expect("ellipse");
        app.start_text(Point::new(60.0, 200.0), None);
        app.text_insert("sasadsa");
        app.finish_text();
        let t = app.selection[0];
        app.selection = vec![t, e];
        app.begin_fit_text_to_path();
        assert_eq!(app.selection, vec![t]);
        // Still one text and the ellipse: the text itself went on the path.
        let count: usize = app.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.shapes.len())
            .sum();
        assert_eq!(count, 2);
        let tp = fitted(&app, t);
        assert!(tp.baseline().is_closed());
        if tracedraw_text::fonts().families().is_empty() {
            return;
        }
        let s = app.doc().shape(t).expect("text").1.clone();
        let b = app.visual_bounds(&s);
        // Centred over the top of the ellipse, on its outside, upright.
        assert!((b.center().x - 90.0).abs() < 1.0, "{b:?}");
        assert!(b.y0 > 155.0 && b.y0 < 161.0, "{b:?}");
        assert!(b.height() < 10.0, "{b:?}");
    }

    #[test]
    fn text_on_a_straight_line_stands_on_it_from_the_start() {
        tracedraw_text::install();
        let mut app = App::headless();
        let mut line = BezPath::new();
        line.move_to((10.0, 50.0));
        line.line_to((90.0, 50.0));
        let l = app
            .new_shape(ShapeKind::Path {
                path: line,
                closed: false,
            })
            .expect("line");
        app.start_text(Point::new(10.0, 120.0), None);
        app.text_insert("Line");
        app.finish_text();
        let t = app.selection[0];
        app.selection = vec![t, l];
        app.begin_fit_text_to_path();
        let tp = fitted(&app, t);
        assert!(!tp.baseline().is_closed());
        assert_eq!(tp.offset, 0.0);
        if tracedraw_text::fonts().families().is_empty() {
            return;
        }
        let s = app.doc().shape(t).expect("text").1.clone();
        let b = app.visual_bounds(&s);
        assert!((b.x0 - 10.0).abs() < 1.5, "{b:?}");
        assert!(b.y0 > 49.0 && b.y0 < 51.0, "{b:?}");
        assert!(b.height() < 8.0 && b.width() > b.height(), "{b:?}");
    }

    #[test]
    fn the_text_tool_on_a_paths_outline_starts_text_on_it() {
        let mut app = App::headless();
        let e = app
            .new_shape(ShapeKind::Ellipse {
                rect: tracedraw_core::geometry::Rect::new(40.0, 100.0, 140.0, 160.0),
                arc: None,
            })
            .expect("ellipse");
        let _ = e;
        // Inside, far from the outline: not on the path.
        app.selection.clear();
        assert!(!app.start_text_on_path_at(Point::new(90.0, 130.0)));
        // On the outline at the top.
        assert!(app.start_text_on_path_at(Point::new(90.0, 160.0)));
        let t = app.selection[0];
        assert!(app.text_edit.is_some());
        let tp = fitted(&app, t);
        let start = tp.path.segments().next().expect("segment").start();
        assert!((start.y - 160.0).abs() < 0.05, "{start:?}");
        // The text begins where the path was clicked.
        assert_eq!(tp.offset, 0.0);
        let s = app.doc().shape(t).expect("text").1.clone();
        let ShapeKind::Text { align, .. } = s.kind else {
            panic!("a text");
        };
        assert_eq!(align, tracedraw_core::TextAlign::Left);
        tracedraw_text::install();
        app.text_insert("HOME");
        app.finish_text();
        if tracedraw_text::fonts().families().is_empty() {
            return;
        }
        let s = app.doc().shape(t).expect("text").1.clone();
        let b = app.visual_bounds(&s);
        assert!((b.x0 - 90.0).abs() < 2.0 && b.x1 > 100.0, "{b:?}");
    }

    /// A click on the bottom of an ellipse puts the text there, centred
    /// on the point clicked, outside; both mirrors keep it there, inside
    /// and reading the other way; one mirror keeps it in place too.
    #[test]
    fn text_goes_where_the_path_is_clicked_and_mirrors_keep_it_there() {
        tracedraw_text::install();
        let mut app = App::headless();
        let e = app
            .new_shape(ShapeKind::Ellipse {
                rect: tracedraw_core::geometry::Rect::new(40.0, 100.0, 140.0, 160.0),
                arc: None,
            })
            .expect("ellipse");
        app.selection.clear();
        app.begin_fit_text_to_path();
        app.fit_text_to_picked_path(e, Some(Point::new(90.0, 100.0)));
        let t = app.selection[0];
        let tp = fitted(&app, t);
        let start = tp.path.segments().next().expect("segment").start();
        assert!(
            (start.y - 100.0).abs() < 0.05 && (start.x - 90.0).abs() < 0.05,
            "{start:?}"
        );
        // Clicked off the extremes: the path starts at the nearest point.
        let (path, closed) = path_for_text_at(&tp.path, Some(Point::new(150.0, 130.0)));
        assert!(closed);
        let s0 = path.segments().next().expect("segment").start();
        assert!(
            (s0.x - 140.0).abs() < 0.05 && (s0.y - 130.0).abs() < 0.05,
            "{s0:?}"
        );
        app.text_insert("HOME");
        app.finish_text();
        app.selection = vec![t];
        if tracedraw_text::fonts().families().is_empty() {
            return;
        }
        let bounds = |app: &App| {
            let s = app.doc().shape(t).expect("text").1.clone();
            app.visual_bounds(&s)
        };
        let b = bounds(&app);
        // Centred under the bottom, outside the ellipse.
        assert!((b.center().x - 90.0).abs() < 1.0, "{b:?}");
        assert!(b.y1 < 101.0 && b.y1 > 95.0, "{b:?}");
        // Both mirrors: still centred on the bottom, now inside.
        app.mirror_fitted_text(true, true);
        let both = bounds(&app);
        assert!((both.center().x - 90.0).abs() < 1.0, "{both:?}");
        assert!(both.y0 > 99.0 && both.y0 < 105.0, "{both:?}");
        assert_eq!(fitted(&app, t).mirrors(), (true, true));
        // Back, then the horizontal mirror alone: in place.
        app.mirror_fitted_text(false, false);
        let back = bounds(&app);
        assert!((back.center().x - b.center().x).abs() < 0.01, "{back:?}");
        assert!((back.y1 - b.y1).abs() < 0.01, "{back:?}");
        app.mirror_fitted_text(true, false);
        let h = bounds(&app);
        assert!((h.center().x - 90.0).abs() < 1.0, "{h:?}");
    }

    /// On an open path the horizontal mirror keeps the letters over the
    /// same stretch of the path: the start placement becomes an end one
    /// with the offset turned round.
    #[test]
    fn the_horizontal_mirror_keeps_text_on_a_line_in_place() {
        tracedraw_text::install();
        let mut app = App::headless();
        let mut line = BezPath::new();
        line.move_to((10.0, 50.0));
        line.line_to((210.0, 50.0));
        let l = app
            .new_shape(ShapeKind::Path {
                path: line,
                closed: false,
            })
            .expect("line");
        app.start_text(Point::new(10.0, 120.0), None);
        app.text_insert("Card");
        app.finish_text();
        let t = app.selection[0];
        app.selection = vec![t, l];
        app.begin_fit_text_to_path();
        app.edit_fitted_text(|tp, _| tp.offset = 30.0);
        app.mirror_fitted_text(true, false);
        let ft = app.fitted_text().expect("fitted");
        assert_eq!(ft.align, tracedraw_core::TextAlign::Right);
        assert_eq!(ft.on_path.offset, -30.0);
        app.mirror_fitted_text(false, false);
        let ft = app.fitted_text().expect("fitted");
        assert_eq!(ft.align, tracedraw_core::TextAlign::Left);
        assert_eq!(ft.on_path.offset, 30.0);
        if tracedraw_text::fonts().families().is_empty() {
            return;
        }
        let x = |app: &App| {
            let s = app.doc().shape(t).expect("text").1.clone();
            app.visual_bounds(&s)
        };
        let before = x(&app);
        assert!((before.x0 - 40.0).abs() < 3.0, "{before:?}");
        app.mirror_fitted_text(true, false);
        let after = x(&app);
        assert!(
            (after.center().x - before.center().x).abs() < 0.5,
            "{after:?}"
        );
    }

    #[test]
    fn case_modes() {
        assert_eq!(
            apply_case("hello world. bye", CaseMode::Sentence),
            "Hello world. Bye"
        );
        assert_eq!(apply_case("hello world", CaseMode::Title), "Hello World");
        assert_eq!(apply_case("aBc", CaseMode::Toggle), "AbC");
    }

    #[test]
    fn convert_to_bitmap_applies_mode_dither_and_overprint() {
        use crate::bitmap_modes::{ConvertMode, ConvertOptions};
        let make = || {
            let mut app = App::headless();
            let id = app
                .new_shape(ShapeKind::Ellipse {
                    rect: Rect::new(0.0, 0.0, 30.0, 20.0),
                    arc: None,
                })
                .expect("ellipse");
            app.run(Command::SetFill {
                shapes: vec![id],
                fill: Fill::Solid(Color::rgb8(200, 120, 40)),
            });
            app.select(vec![id]);
            app
        };
        let pixels = |app: &App| -> image::RgbaImage {
            let s = app.selected_shapes().into_iter().next().expect("bitmap");
            match s.kind {
                ShapeKind::Bitmap { png, .. } => crate::bitmap_fx::decode(&png).expect("png"),
                _ => panic!("not a bitmap"),
            }
        };
        // Black and white: only black and white where it is opaque.
        let mut app = make();
        app.convert_to_bitmap_with(&ConvertOptions {
            dpi: 72.0,
            mode: ConvertMode::BlackWhite,
            transparent: false,
            ..Default::default()
        });
        let bw = pixels(&app);
        assert!(bw
            .pixels()
            .all(|p| (p[0] == 0 || p[0] == 255) && p[0] == p[1] && p[1] == p[2]));
        // Grayscale: equal channels; overprint marks the bitmap.
        let mut app = make();
        app.convert_to_bitmap_with(&ConvertOptions {
            dpi: 72.0,
            mode: ConvertMode::Grayscale,
            overprint_black: true,
            ..Default::default()
        });
        assert!(pixels(&app).pixels().all(|p| p[0] == p[1] && p[1] == p[2]));
        assert!(app.selected_shapes()[0].overprint_fill);
        // Without anti-aliasing the edge has no half-covered pixels.
        let mut app = make();
        app.convert_to_bitmap_with(&ConvertOptions {
            dpi: 150.0,
            anti_alias: false,
            ..Default::default()
        });
        assert!(pixels(&app).pixels().all(|p| p[3] == 0 || p[3] == 255));
        // The uncompressed size: 1 bit a pixel rounds up per row.
        let o = ConvertOptions {
            mode: ConvertMode::BlackWhite,
            ..Default::default()
        };
        assert_eq!(o.bytes(10, 3), 6);
        assert_eq!(ConvertOptions::default().bytes(10, 3), 90);
        assert!(!ConvertMode::Rgb.can_dither() && ConvertMode::Paletted.can_dither());
    }

    #[test]
    fn manual_inflate_grows_by_percent_around_the_middle() {
        let mut app = App::headless();
        let img: image::RgbaImage =
            image::ImageBuffer::from_pixel(40, 20, image::Rgba([9, 9, 9, 255]));
        let png = crate::bitmap_fx::encode(&img).expect("png");
        let id = app
            .new_shape(ShapeKind::Bitmap {
                rect: Rect::new(0.0, 0.0, 40.0, 20.0),
                width_px: 40,
                height_px: 20,
                png: png.into(),
                fx: None,
            })
            .expect("bitmap");
        app.select(vec![id]);
        let mut st = crate::ui::dialogs::InflateState::for_app(&app);
        assert_eq!((st.base, st.pixels()), ((40, 20), (40, 20)));
        st.set_width(150.0);
        assert_eq!(st.pixels(), (60, 30));
        st.keep = false;
        st.set_height(200.0);
        assert_eq!(st.pixels(), (60, 40));
        st.set_width(50.0);
        assert_eq!(st.pw, 100.0);
        let depth = app.engine.history_labels().0.len();
        app.inflate_bitmaps_by(150.0, 200.0);
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let ShapeKind::Bitmap {
            rect,
            width_px,
            height_px,
            png,
            ..
        } = &app.doc().find_shape(id).expect("bitmap").kind
        else {
            panic!("not a bitmap");
        };
        assert_eq!((*width_px, *height_px), (60, 40));
        // The original stays in the middle, one millimetre per pixel.
        assert_eq!(
            (rect.x0, rect.y0, rect.x1, rect.y1),
            (-10.0, -10.0, 50.0, 30.0)
        );
        let out = crate::bitmap_fx::decode(png).expect("png");
        assert_eq!((out.get_pixel(0, 0)[3], out.get_pixel(30, 20)[3]), (0, 255));
        // Less than the original size does nothing.
        app.inflate_bitmaps_by(100.0, 100.0);
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
    }

    fn square_at(app: &mut App, x: f64, y: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x, y, x + 4.0, y + 2.0),
            radius: 0.0,
            corners: None,
        })
        .expect("square")
    }

    fn line_path(app: &mut App, pts: &[(f64, f64)], closed: bool) -> ShapeId {
        let mut path = BezPath::new();
        path.move_to(pts[0]);
        for p in &pts[1..] {
            path.line_to(*p);
        }
        if closed {
            path.close_path();
        }
        app.new_shape(ShapeKind::Path { path, closed })
            .expect("path")
    }

    fn centre(app: &App, id: ShapeId) -> Point {
        app.doc().find_shape(id).expect("shape").bounds().center()
    }

    /// Object > Fit Objects to Path opens its panel instead of moving the
    /// objects at once, and the panel's settings decide the placing (#1027).
    #[test]
    fn fit_objects_to_path_opens_its_panel_and_uses_its_settings() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let a = square_at(&mut app, 50.0, 50.0);
        let b = square_at(&mut app, 60.0, 60.0);
        let c = square_at(&mut app, 70.0, 70.0);
        let line = line_path(&mut app, &[(0.0, 0.0), (100.0, 0.0)], false);
        app.select(vec![a, b, c, line]);
        app.open_docker(crate::app::DockerTab::FitToPath);
        assert!(app.open_dockers.contains(&crate::app::DockerTab::FitToPath));
        assert_eq!(app.docker_tab, crate::app::DockerTab::FitToPath);
        // The panel draws; it previews the three outlines once a value is
        // entered for these objects (#1136).
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::ui::dockers2::show(&mut app, ui, crate::app::DockerTab::FitToPath);
        });
        out.textures_delta.clear();
        assert!(app.docker_preview.is_empty());
        app.fit_path_edited = true;
        assert_eq!(app.fit_objects_to_path_preview().len(), 3);
        assert!(
            (centre(&app, a) - Point::new(52.0, 51.0)).hypot() < 1e-9,
            "nothing moved yet"
        );

        // Evenly: both ends and the middle.
        app.fit_objects_to_path();
        for (id, x) in [(a, 0.0), (b, 50.0), (c, 100.0)] {
            assert!(
                (centre(&app, id) - Point::new(x, 0.0)).hypot() < 1e-6,
                "{x}"
            );
        }
        // A fixed distance after an offset, from the other end.
        app.fit_path = FitPathSettings {
            spacing: FitSpacing::Distance,
            distance: 20.0,
            offset: 10.0,
            reverse: true,
            ..FitPathSettings::default()
        };
        app.fit_objects_to_path();
        for (id, x) in [(a, 90.0), (b, 70.0), (c, 50.0)] {
            assert!(
                (centre(&app, id) - Point::new(x, 0.0)).hypot() < 1e-6,
                "{x}"
            );
        }
        assert_eq!(app.engine.undo_label(), Some("Fit Objects to Path"));
    }

    /// The panel starts from the defaults, with no preview, for objects it
    /// was not used on: values entered for other objects or in another
    /// drawing are not carried over; for the same objects they stay, and
    /// the preview shows once a value is entered (#1136).
    #[test]
    fn fit_objects_to_path_starts_from_the_defaults_for_other_objects() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let show = |app: &mut App| {
            app.docker_preview.clear();
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                crate::ui::dockers2::show(app, ui, crate::app::DockerTab::FitToPath);
            });
            out.textures_delta.clear();
        };
        let a = square_at(&mut app, 50.0, 50.0);
        let line = line_path(&mut app, &[(0.0, 0.0), (100.0, 0.0)], false);
        app.select(vec![a, line]);
        app.open_docker(crate::app::DockerTab::FitToPath);
        show(&mut app);
        assert!(app.docker_preview.is_empty(), "no preview before a value");
        // Values entered for these objects (as the panel's fields do).
        let entered = FitPathSettings {
            spacing: FitSpacing::Distance,
            distance: 20.0,
            offset: 30.0,
            rotation: FitRotation::Follow,
            reverse: true,
            ..FitPathSettings::default()
        };
        app.fit_path = entered;
        app.fit_path_edited = true;
        show(&mut app);
        assert_eq!(app.fit_path, entered);
        assert_eq!(app.docker_preview.len(), 1);
        app.fit_objects_to_path();
        // Another object on the path: the defaults, no preview.
        let b = square_at(&mut app, 80.0, 80.0);
        app.select(vec![b, line]);
        show(&mut app);
        assert_eq!(app.fit_path, FitPathSettings::default());
        assert!(app.docker_preview.is_empty());
        // Back to the first objects: the defaults too (the panel follows
        // the objects selected, it does not remember every selection).
        app.select(vec![a, line]);
        show(&mut app);
        assert_eq!(app.fit_path, FitPathSettings::default());
        // Another drawing with objects of the same ids.
        app.fit_path = entered;
        app.fit_path_edited = true;
        show(&mut app);
        assert_eq!(app.fit_path, entered);
        app.new_document();
        let a2 = square_at(&mut app, 50.0, 50.0);
        let line2 = line_path(&mut app, &[(0.0, 0.0), (100.0, 0.0)], false);
        app.select(vec![a2, line2]);
        show(&mut app);
        assert_eq!(app.fit_path, FitPathSettings::default());
        assert!(app.docker_preview.is_empty());
    }

    /// With Rotate the objects turn with the path; on a closed path they go
    /// all the way round, the closing side included, without the first and
    /// last meeting (#1027).
    #[test]
    fn fit_objects_to_path_turns_and_goes_round_closed_paths() {
        let mut app = App::headless();
        let ids: Vec<ShapeId> = (0..4)
            .map(|i| square_at(&mut app, 200.0 + i as f64, 0.0))
            .collect();
        // A 40 x 40 square, so the objects land on its four corners.
        let sq = line_path(
            &mut app,
            &[(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)],
            true,
        );
        let mut sel = ids.clone();
        sel.push(sq);
        app.select(sel);
        app.fit_objects_to_path();
        for (id, p) in ids
            .iter()
            .zip([(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)])
        {
            assert!((centre(&app, *id) - Point::from(p)).hypot() < 1e-6, "{p:?}");
        }
        // Turned: on the right side (going down) the 4 x 2 object stands up.
        app.fit_path.rotation = FitRotation::Follow;
        app.fit_path.offset = 50.0;
        app.fit_objects_to_path();
        let s = app.doc().find_shape(ids[0]).expect("shape").clone();
        let r = s.bounds();
        assert!(
            (r.center() - Point::new(40.0, 10.0)).hypot() < 1e-6,
            "{r:?}"
        );
        assert!(
            (r.width() - 2.0).abs() < 1e-6 && (r.height() - 4.0).abs() < 1e-6,
            "{r:?}"
        );
    }

    /// Keep originals leaves the objects where they were and places
    /// copies; Group all objects puts what was placed in one group, which
    /// is selected; one undo takes it all back (#1091).
    #[test]
    fn fit_objects_to_path_keeps_originals_and_groups() {
        let mut app = App::headless();
        let a = square_at(&mut app, 200.0, 0.0);
        let b = square_at(&mut app, 210.0, 0.0);
        let line = line_path(&mut app, &[(0.0, 0.0), (100.0, 0.0)], false);
        let count = |app: &App| {
            app.doc()
                .all_layers()
                .map(|l| l.shapes.len())
                .sum::<usize>()
        };
        let before = count(&app);
        app.select(vec![a, b, line]);
        app.fit_path_follow_selection();
        app.fit_path = FitPathSettings {
            keep_originals: true,
            group: true,
            copies: FitCopies::Count,
            count: 4,
            ..FitPathSettings::default()
        };
        app.fit_objects_to_path();
        // The originals stay; four copies in one group.
        assert!((centre(&app, a) - Point::new(202.0, 1.0)).hypot() < 1e-9);
        assert!((centre(&app, b) - Point::new(212.0, 1.0)).hypot() < 1e-9);
        assert_eq!(count(&app), before + 1);
        assert_eq!(app.selection.len(), 1);
        let g = app
            .doc()
            .find_shape(app.selection[0])
            .expect("group")
            .clone();
        match &g.kind {
            ShapeKind::Group { children } => {
                assert_eq!(children.len(), 4);
                let xs: Vec<f64> = children.iter().map(|c| c.bounds().center().x).collect();
                for (x, w) in xs.iter().zip([0.0, 100.0 / 3.0, 200.0 / 3.0, 100.0]) {
                    assert!((x - w).abs() < 1e-6, "{xs:?}");
                }
            }
            k => panic!("{k:?}"),
        }
        assert_eq!(app.engine.undo_label(), Some("Fit Objects to Path"));
        app.undo();
        assert_eq!(count(&app), before);
        // Grouping the objects themselves.
        app.select(vec![a, b, line]);
        app.fit_path = FitPathSettings {
            group: true,
            ..FitPathSettings::default()
        };
        app.fit_objects_to_path();
        assert_eq!(count(&app), before - 1);
        let g = app.selection[0];
        assert!(app.doc().ancestors(a).contains(&g));
    }

    /// Copies along the path: a count makes the missing objects as
    /// copies in the original's layer, below the path's selection; the
    /// objects placed become the selection and one undo takes it all
    /// back. The preview is worked out once while nothing changes, and
    /// a long run previews as boxes (#1091, #1196).
    #[test]
    fn fit_objects_to_path_makes_copies_in_one_step() {
        let mut app = App::headless();
        let a = square_at(&mut app, 200.0, 0.0);
        let line = line_path(&mut app, &[(0.0, 0.0), (100.0, 0.0)], false);
        let before = app
            .doc()
            .all_layers()
            .map(|l| l.shapes.len())
            .sum::<usize>();
        app.select(vec![a, line]);
        app.fit_path_follow_selection();
        app.fit_path = FitPathSettings {
            copies: FitCopies::Count,
            count: 5,
            place: FitPlace::Above,
            side: 1.0,
            ..FitPathSettings::default()
        };
        app.fit_path_edited = true;
        assert_eq!(app.fit_objects_to_path_preview().len(), 5);
        let first = app.fit_objects_to_path_preview().as_ptr();
        assert_eq!(app.fit_objects_to_path_preview().as_ptr(), first, "kept");
        app.fit_objects_to_path();
        let after = app
            .doc()
            .all_layers()
            .map(|l| l.shapes.len())
            .sum::<usize>();
        assert_eq!(after, before + 4);
        assert_eq!(app.selection.len(), 5);
        assert_eq!(app.selection[0], a);
        for (id, x) in app
            .selection
            .clone()
            .iter()
            .zip([0.0, 25.0, 50.0, 75.0, 100.0])
        {
            // 2 mm high, 1 mm above the line.
            assert!(
                (centre(&app, *id) - Point::new(x, 2.0)).hypot() < 1e-6,
                "{x}"
            );
        }
        assert_eq!(app.engine.undo_label(), Some("Fit Objects to Path"));
        app.undo();
        let undone = app
            .doc()
            .all_layers()
            .map(|l| l.shapes.len())
            .sum::<usize>();
        assert_eq!(undone, before);
        assert!((centre(&app, a) - Point::new(202.0, 1.0)).hypot() < 1e-6);
        // Filling a long path: boxes instead of outlines in the preview.
        app.select(vec![a, line]);
        app.fit_path_follow_selection();
        app.fit_path = FitPathSettings {
            spacing: FitSpacing::Distance,
            distance: 0.25,
            copies: FitCopies::Fill,
            ..FitPathSettings::default()
        };
        app.fit_path_edited = true;
        let n = app.fit_objects_to_path_preview().len();
        assert_eq!(n, 401);
        assert!(app
            .fit_objects_to_path_preview()
            .iter()
            .all(|p| p.elements().len() <= 6));
    }
}

/// How much of the rest of the page shows while a clip frame's contents
/// are edited (the frame and its contents are drawn in full).
pub const CLIP_EDIT_VISIBILITY: f32 = 0.25;

/// The page transform that takes `contents` (its box) to `frame` as the
/// Fit Contents `mode` says: centred without scaling, scaled alike to fit
/// inside or to cover, or stretched to the box. The identity for an
/// empty box.
pub fn fit_contents_transform(
    contents: Rect,
    frame: Rect,
    mode: crate::ui::clip_bar::FitContents,
) -> Affine {
    use crate::ui::clip_bar::FitContents;
    if contents.width() < 1e-9 || contents.height() < 1e-9 {
        return Affine::IDENTITY;
    }
    let (sx, sy) = match mode {
        FitContents::Center => (1.0, 1.0),
        FitContents::FitProportionally => {
            let s = (frame.width() / contents.width()).min(frame.height() / contents.height());
            (s, s)
        }
        FitContents::FillProportionally => {
            let s = (frame.width() / contents.width()).max(frame.height() / contents.height());
            (s, s)
        }
        FitContents::Stretch => (
            frame.width() / contents.width(),
            frame.height() / contents.height(),
        ),
    };
    let c = contents.center();
    let f = frame.center();
    Affine::translate(f.to_vec2())
        * Affine::scale_non_uniform(sx, sy)
        * Affine::translate(-c.to_vec2())
}

#[cfg(test)]
mod clip_frame_tests {
    use super::fit_contents_transform;
    use crate::app::App;
    use crate::ui::clip_bar::FitContents;
    use tracedraw_core::geometry::{Rect, Shape as _};
    use tracedraw_core::{ShapeId, ShapeKind};

    fn rect(app: &mut App, r: Rect) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: r,
            radius: 0.0,
            corners: None,
        })
        .expect("a layer")
    }

    /// The four ways of fitting contents to a frame, and the bar's
    /// commands on a clip frame: the contents picked, fitted as one undo
    /// step.
    #[test]
    fn contents_fit_the_frame_four_ways() {
        let frame = Rect::new(0.0, 0.0, 100.0, 50.0);
        let contents = Rect::new(200.0, 200.0, 240.0, 220.0);
        let moved = |m: FitContents| -> Rect {
            (fit_contents_transform(contents, frame, m) * contents.to_path(0.01)).bounding_box()
        };
        let near = |a: Rect, b: Rect| {
            (a.x0 - b.x0).abs() < 1e-9
                && (a.y0 - b.y0).abs() < 1e-9
                && (a.x1 - b.x1).abs() < 1e-9
                && (a.y1 - b.y1).abs() < 1e-9
        };
        assert!(near(
            moved(FitContents::Center),
            Rect::new(30.0, 15.0, 70.0, 35.0)
        ));
        // Fit: 40 x 20 scaled by 2.5 (the height's limit) to 100 x 50.
        assert!(near(moved(FitContents::FitProportionally), frame));
        // A taller box: fit by the height, fill by the width.
        let tall = Rect::new(0.0, 0.0, 10.0, 20.0);
        let fit = (fit_contents_transform(tall, frame, FitContents::FitProportionally)
            * tall.to_path(0.01))
        .bounding_box();
        assert!(near(fit, Rect::new(37.5, 0.0, 62.5, 50.0)), "{fit:?}");
        let fill = (fit_contents_transform(tall, frame, FitContents::FillProportionally)
            * tall.to_path(0.01))
        .bounding_box();
        assert!(near(fill, Rect::new(0.0, -75.0, 100.0, 125.0)), "{fill:?}");
        assert!(near(moved(FitContents::Stretch), frame));
        assert_eq!(
            fit_contents_transform(Rect::ZERO, frame, FitContents::Stretch),
            tracedraw_core::geometry::Affine::IDENTITY
        );

        let mut app = App::headless();
        let f = rect(&mut app, Rect::new(10.0, 10.0, 110.0, 60.0));
        let a = rect(&mut app, Rect::new(200.0, 200.0, 220.0, 210.0));
        let b = rect(&mut app, Rect::new(220.0, 200.0, 240.0, 220.0));
        app.settings.clip_frame.auto_center = crate::settings::AutoCenter::Never;
        app.place_inside(vec![a, b], f);
        let clip = app.doc().top_level_of(a).expect("clip");
        app.select_clip_contents(clip);
        assert_eq!(app.selection, vec![a, b]);
        assert!(app.selection_nested());
        let depth = app.engine.history_labels().0.len();
        app.fit_clip_contents(clip, FitContents::Stretch);
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let bounds = [a, b]
            .iter()
            .map(|id| app.doc().shape_on_page(*id).expect("inside").bounds())
            .reduce(|x, y| x.union(y))
            .expect("bounds");
        assert!(
            near(bounds, Rect::new(10.0, 10.0, 110.0, 60.0)),
            "{bounds:?}"
        );
    }
}

#[cfg(test)]
mod symmetry_tests {
    use crate::app::App;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::live::Effect;
    use tracedraw_core::{ShapeId, ShapeKind};

    fn rect(app: &mut App, r: Rect) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: r,
            radius: 0.0,
            corners: None,
        })
        .expect("a layer")
    }

    fn has_symmetry(app: &App, id: ShapeId) -> bool {
        app.doc().find_shape(id).is_some_and(|s| {
            s.effects
                .iter()
                .any(|e| matches!(e, Effect::Symmetry { .. }))
        })
    }

    fn members(app: &App, g: ShapeId) -> Vec<ShapeId> {
        match app.doc().find_shape(g).map(|s| &s.kind) {
            Some(ShapeKind::Group { children }) => children.iter().map(|c| c.id).collect(),
            _ => Vec::new(),
        }
    }

    /// Several objects become one symmetry group in one step, each still
    /// to be picked inside it; an object drawn, pasted or duplicated
    /// while the symmetry is edited joins the group, folded into the step
    /// that made it; a lone object with a symmetry becomes a group when
    /// another joins it.
    #[test]
    fn a_symmetry_takes_several_objects_and_those_made_while_editing() {
        let mut app = App::headless();
        let a = rect(&mut app, Rect::new(10.0, 10.0, 30.0, 30.0));
        let b = rect(&mut app, Rect::new(40.0, 10.0, 60.0, 30.0));
        app.select(vec![a, b]);
        let depth = app.engine.history_labels().0.len();
        app.create_symmetry();
        assert_eq!(app.engine.history_labels().0.len(), depth + 1, "one step");
        let g = app.symmetry_edit.expect("editing the symmetry");
        assert!(has_symmetry(&app, g));
        assert_eq!(members(&app, g), vec![a, b]);
        assert!(!has_symmetry(&app, a) && !has_symmetry(&app, b));
        // Picked inside: nested, so the handles are round.
        app.select(vec![a]);
        assert!(app.selection_nested());
        // Drawn while editing: in the group, one step with the drawing.
        let depth = app.engine.history_labels().0.len();
        let c = rect(&mut app, Rect::new(10.0, 40.0, 30.0, 60.0));
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let g2 = app.symmetry_edit.expect("still editing");
        assert_eq!(members(&app, g2), vec![a, b, c]);
        assert!(has_symmetry(&app, g2));
        assert_eq!(app.selection, vec![c]);
        assert_eq!(
            app.doc()
                .all_layers()
                .flat_map(|l| &l.shapes)
                .filter(|s| matches!(s.kind, ShapeKind::Group { .. }))
                .count(),
            1,
            "one group on the page"
        );
        // Undo takes the drawing and its joining back together; the mode
        // stays on the group as remade.
        app.undo();
        let g = app.symmetry_editing().expect("editing the symmetry");
        assert_eq!(members(&app, g), vec![a, b]);
        assert!(app.doc().find_shape(c).is_none());
        // Duplicated while editing: the copy joins too.
        app.select(vec![a]);
        app.duplicate();
        let g3 = app.symmetry_edit.expect("still editing");
        assert_eq!(members(&app, g3).len(), 3);
        app.finish_symmetry_edit();

        // A lone object with a symmetry: another drawn while editing it
        // makes the two a group holding the symmetry.
        let mut app = App::headless();
        let a = rect(&mut app, Rect::new(10.0, 10.0, 30.0, 30.0));
        app.select(vec![a]);
        app.create_symmetry();
        assert_eq!(app.symmetry_edit, Some(a));
        let b = rect(&mut app, Rect::new(10.0, 40.0, 30.0, 60.0));
        let g = app.symmetry_edit.expect("editing");
        assert_ne!(g, a);
        assert_eq!(members(&app, g), vec![a, b]);
        assert!(has_symmetry(&app, g) && !has_symmetry(&app, a));
        // Not editing: a new object stays on its own.
        app.finish_symmetry_edit();
        let c = rect(&mut app, Rect::new(70.0, 10.0, 80.0, 20.0));
        assert_eq!(members(&app, g), vec![a, b]);
        assert!(app.doc().top_level_of(c) == Some(c));
    }
}

#[cfg(test)]
mod fit_page_tests {
    use crate::app::App;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::ShapeKind;

    fn rect(app: &mut App, r: Rect) -> tracedraw_core::ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: r,
            radius: 0.0,
            corners: None,
        })
        .expect("rect")
    }

    /// Fit Page to Selection sizes the page to the selection and moves
    /// the page's objects so the selection sits on it from the corner;
    /// Fit Page to Drawing does the same for everything on the page. One
    /// undo step each.
    #[test]
    fn the_page_fits_the_selection_or_the_drawing() {
        let mut app = App::headless();
        let a = rect(&mut app, Rect::new(30.0, 40.0, 80.0, 90.0));
        let b = rect(&mut app, Rect::new(100.0, 100.0, 120.0, 110.0));
        let bounds = |app: &App, id| app.doc().shape_on_page(id).expect("shape").bounds();
        app.select(vec![a]);
        let depth = app.engine.history_labels().0.len();
        app.fit_page_to_selection();
        let size = app.page_size();
        assert!((size.width - 50.0).abs() < 1e-9 && (size.height - 50.0).abs() < 1e-9);
        let ba = bounds(&app, a);
        assert!(ba.x0.abs() < 1e-9 && ba.y0.abs() < 1e-9, "{ba:?}");
        let bb = bounds(&app, b);
        assert!(
            (bb.x0 - 70.0).abs() < 1e-9 && (bb.y0 - 60.0).abs() < 1e-9,
            "{bb:?}"
        );
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        app.undo();
        assert!((bounds(&app, a).x0 - 30.0).abs() < 1e-9);
        assert!((app.page_size().width - 210.0).abs() < 1e-6);
        // The whole drawing: 30..120 by 40..110.
        app.select(Vec::new());
        app.fit_page_to_drawing();
        let size = app.page_size();
        assert!((size.width - 90.0).abs() < 1e-9 && (size.height - 70.0).abs() < 1e-9);
        assert!(bounds(&app, a).x0.abs() < 1e-9 && bounds(&app, a).y0.abs() < 1e-9);
        let bb = bounds(&app, b);
        assert!(
            (bb.x1 - 90.0).abs() < 1e-9 && (bb.y1 - 70.0).abs() < 1e-9,
            "{bb:?}"
        );
        // Autofit Page with a margin: the page is larger by the margin on
        // every side and the drawing sits that far in; a margin that is
        // not a number counts as none.
        app.undo();
        app.fit_page_to_drawing_with_margin(5.0);
        let size = app.page_size();
        assert!(
            (size.width - 100.0).abs() < 1e-9 && (size.height - 80.0).abs() < 1e-9,
            "{size:?}"
        );
        let ba = bounds(&app, a);
        assert!(
            (ba.x0 - 5.0).abs() < 1e-9 && (ba.y0 - 5.0).abs() < 1e-9,
            "{ba:?}"
        );
        app.undo();
        app.fit_page_to_drawing_with_margin(f64::NAN);
        assert!((app.page_size().width - 90.0).abs() < 1e-9);
        // Nothing selected: Fit Page to Selection does nothing.
        let before = app.engine.history_labels().0.len();
        app.fit_page_to_selection();
        assert_eq!(app.engine.history_labels().0.len(), before);
    }
}

/// One step of turning one text into another (see [`char_edits`]).
#[derive(Debug, Clone, PartialEq)]
enum CharEdit {
    Keep(usize),
    Remove(usize),
    Add(String),
}

/// The steps that turn `old` into `new`, keeping the longest common
/// subsequence of characters (beyond 4 million comparisons, only the
/// common start and end are kept).
fn char_edits(old: &[char], new: &[char]) -> Vec<CharEdit> {
    let head = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let tail = old[head..]
        .iter()
        .rev()
        .zip(new[head..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let (a, b) = (&old[head..old.len() - tail], &new[head..new.len() - tail]);
    let mut out = Vec::new();
    let mut push = |e: CharEdit| match (out.last_mut(), e) {
        (Some(CharEdit::Keep(n)), CharEdit::Keep(m)) => *n += m,
        (Some(CharEdit::Remove(n)), CharEdit::Remove(m)) => *n += m,
        (Some(CharEdit::Add(s)), CharEdit::Add(t)) => s.push_str(&t),
        (_, e) => out.push(e),
    };
    if head > 0 {
        push(CharEdit::Keep(head));
    }
    if a.len().saturating_mul(b.len()) > 4_000_000 {
        if !a.is_empty() {
            push(CharEdit::Remove(a.len()));
        }
        if !b.is_empty() {
            push(CharEdit::Add(b.iter().collect()));
        }
    } else {
        // lcs[i][j]: the longest common subsequence of a[i..] and b[j..].
        let w = b.len() + 1;
        let mut lcs = vec![0u32; (a.len() + 1) * w];
        for i in (0..a.len()).rev() {
            for j in (0..b.len()).rev() {
                lcs[i * w + j] = if a[i] == b[j] {
                    lcs[(i + 1) * w + j + 1] + 1
                } else {
                    lcs[(i + 1) * w + j].max(lcs[i * w + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < a.len() || j < b.len() {
            if i < a.len() && j < b.len() && a[i] == b[j] {
                push(CharEdit::Keep(1));
                i += 1;
                j += 1;
            } else if j < b.len() && (i == a.len() || lcs[i * w + j + 1] >= lcs[(i + 1) * w + j]) {
                push(CharEdit::Add(b[j].to_string()));
                j += 1;
            } else {
                push(CharEdit::Remove(1));
                i += 1;
            }
        }
    }
    if tail > 0 {
        push(CharEdit::Keep(tail));
    }
    out
}

#[cfg(test)]
mod text_dialog_tests {
    use crate::app::App;
    use crate::ui::dialogs::Dialog;
    use tracedraw_core::{document::ShapeKind, geometry::Point, TextSpan};

    #[test]
    fn char_edits_keep_what_both_texts_share() {
        use super::{char_edits, CharEdit};
        let c = |s: &str| s.chars().collect::<Vec<_>>();
        assert_eq!(
            char_edits(&c("Hello world"), &c("Hello big world!")),
            vec![
                CharEdit::Keep(6),
                CharEdit::Add("big ".into()),
                CharEdit::Keep(5),
                CharEdit::Add("!".into())
            ]
        );
        assert_eq!(
            char_edits(&c("abc"), &c("ac")),
            vec![CharEdit::Keep(1), CharEdit::Remove(1), CharEdit::Keep(1)]
        );
        assert_eq!(char_edits(&c(""), &c("x")), vec![CharEdit::Add("x".into())]);
        assert_eq!(char_edits(&c("x"), &c("")), vec![CharEdit::Remove(1)]);
    }

    /// Text > Edit Text opens the dialog with the text; OK keeps the
    /// formatting of the characters that did not change (#355).
    #[test]
    fn the_edit_text_dialog_keeps_the_formatting_of_what_did_not_change() {
        let mut app = App::headless();
        let mut bold = TextSpan::new("Hello ", "Sans", 20.0);
        bold.bold = true;
        let plain = TextSpan::new("world", "Sans", 12.0);
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![bold, plain],
                origin: Point::new(10.0, 10.0),
                frame: None,
                align: tracedraw_core::document::TextAlign::Left,
                para: Default::default(),
                on_path: None,
            })
            .expect("text");
        app.select(vec![id]);
        app.open_edit_text_dialog();
        let Dialog::EditText { shape, text } = &app.dialog else {
            panic!("no dialog: {:?}", std::mem::discriminant(&app.dialog));
        };
        assert_eq!((*shape, text.as_str()), (id, "Hello world"));
        let depth = app.engine.history_labels().0.len();
        app.set_text_content(id, "Hello big world!");
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        assert_eq!(app.engine.undo_label(), Some("Edit Text"));
        let ShapeKind::Text { spans, .. } = app.doc().find_shape(id).expect("text").kind.clone()
        else {
            panic!("text");
        };
        // "Hello " stays bold at 20 pt; "big " (typed after the bold
        // space) takes its style; "world" stays plain, "!" follows it.
        let styled: Vec<(String, bool, f64)> = spans
            .iter()
            .map(|s| (s.text.clone(), s.bold, s.size_pt))
            .collect();
        assert_eq!(
            styled,
            vec![
                ("Hello big ".to_string(), true, 20.0),
                ("world!".to_string(), false, 12.0)
            ]
        );
        // The same text: nothing to do.
        app.set_text_content(id, "Hello big world!");
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        // The dialog draws.
        let ctx = crate::theme::ui_context();
        app.open_edit_text_dialog();
        app.show_welcome = false;
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::root(&mut app, ui));
            out.textures_delta.clear();
        }
        assert!(matches!(app.dialog, Dialog::EditText { .. }));
    }
}
