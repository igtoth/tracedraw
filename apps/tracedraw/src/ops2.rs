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

/// Step and Repeat docker state.
#[derive(Debug, Clone, PartialEq)]
pub struct StepRepeat {
    pub copies: u32,
    pub dx: f64,
    pub dy: f64,
    /// 0 = offset, 1 = spacing between copies (uses object size).
    pub mode_x: u8,
    pub mode_y: u8,
}

impl Default for StepRepeat {
    fn default() -> Self {
        StepRepeat {
            copies: 1,
            dx: 6.35,
            dy: 0.0,
            mode_x: 0,
            mode_y: 0,
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
    // ----- layout ----------------------------------------------------------------

    /// Layout > Insert Page Number: an artistic text "1", "2"... on the
    /// chosen pages (bottom centre), updated through the symbol-like
    /// name "page-number" so future renumbering can find it.
    pub fn insert_page_number(&mut self, where_: PageNumberWhere) {
        let pages: Vec<(usize, tracedraw_core::PageId)> = self
            .doc()
            .pages
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.id))
            .collect();
        let active = self.page;
        let mut cmds = Vec::new();
        for (i, pid) in pages {
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
            cmds.push(Command::AddShape { layer, shape: s });
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Insert Page Number", &cmds) {
                self.status = e.to_string();
            }
        }
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
        let shapes = self.selected_shapes();
        let mut cmds = Vec::new();
        let mut new_sel = Vec::new();
        for s in shapes {
            // A vector object with bitmap effects becomes the bitmap of its
            // picture, in its place.
            if let Some(pic) = s.drawn_as() {
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
            for p in parts {
                new_sel.push(p.id);
                cmds.push(Command::AddShape { layer, shape: p });
            }
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Flatten Effects", &cmds) {
                self.status = e.to_string();
            }
            self.selection = new_sel;
        }
    }

    /// Object > Symmetry > Create New Symmetry (Alt+S): one vertical mirror
    /// line at the right edge of the selection.
    pub fn create_symmetry(&mut self) {
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            return;
        }
        let Some(b) = self.selection_bounds() else {
            return;
        };
        let center = Point::new(b.x1, b.center().y);
        for s in shapes {
            let mut effects = s.effects.clone();
            effects.retain(|e| !matches!(e, Effect::Symmetry { .. }));
            effects.push(Effect::Symmetry {
                center,
                angle: 90.0,
                lines: 1,
            });
            self.run(Command::SetEffects {
                shape: s.id,
                effects,
            });
        }
        // Straight into editing it, as Edit Symmetry does.
        self.edit_symmetry();
    }

    /// Object > Symmetry > Edit Symmetry (Alt+E): the symmetry editing
    /// mode for the first selected object with a symmetry. The original is
    /// outlined in red, its mirror lines show across the window and the
    /// symmetry bar (Finish, mirror lines, centre, angle) floats at the
    /// top left of the drawing window.
    pub fn edit_symmetry(&mut self) {
        if let Some((id, ..)) = self.selected_symmetry() {
            self.symmetry_edit = Some(id);
            self.select(vec![id]);
            self.status = tr("status.editing_symmetry");
        }
    }

    /// Object > Symmetry > Finish Editing Symmetry (Alt+Shift+E), and the
    /// symmetry bar's Finish.
    pub fn finish_symmetry_edit(&mut self) {
        if self.symmetry_edit.take().is_some() {
            self.status.clear();
        }
    }

    /// The object whose symmetry is being edited, while it still has one;
    /// the mode ends when it is gone or lost its symmetry.
    pub fn symmetry_editing(&mut self) -> Option<ShapeId> {
        let id = self.symmetry_edit?;
        let alive = self.doc().find_shape(id).is_some_and(|s| {
            s.effects
                .iter()
                .any(|e| matches!(e, Effect::Symmetry { .. }))
        });
        if !alive {
            self.symmetry_edit = None;
            return None;
        }
        Some(id)
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

    /// Update the symmetry parameters of every selected object that has one.
    pub fn set_symmetry(&mut self, center: Point, angle: f64, lines: u8) {
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
                .map(|e| match e {
                    Effect::Symmetry { .. } => Effect::Symmetry {
                        center,
                        angle,
                        lines,
                    },
                    other => other.clone(),
                })
                .collect();
            if self.engine.undo_label() == Some("Symmetry") {
                let _ = self.engine.undo();
            }
            let _ = self.engine.run_with_label(
                &Command::SetEffects {
                    shape: s.id,
                    effects,
                },
                "Symmetry",
            );
        }
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

    /// Apply the Transparency tool settings to the selection. Uniform
    /// transparency is plain opacity; the other kinds use a greyscale mask
    /// (white opaque, black transparent) blended with the merge mode.
    pub fn apply_transparency(&mut self, settings: &crate::effects_ui::TransparencySettings) {
        let shapes = self.selection.clone();
        if shapes.is_empty() {
            return;
        }
        let mut cmds = Vec::new();
        for id in &shapes {
            let Some(s) = self.doc().find_shape(*id).cloned() else {
                continue;
            };
            let mut effects: Vec<Effect> = s
                .effects
                .iter()
                .filter(|e| !matches!(e, Effect::Transparency { .. }))
                .cloned()
                .collect();
            if settings.kind != 0 {
                effects.push(Effect::Transparency {
                    mask: settings.mask.clone(),
                    merge: settings.merge,
                    target: settings.target,
                });
            }
            cmds.push(Command::SetEffects {
                shape: *id,
                effects,
            });
        }
        let opacity = if settings.kind == 0 {
            1.0 - (settings.amount / 100.0).clamp(0.0, 1.0)
        } else {
            1.0
        };
        cmds.push(Command::SetOpacity {
            shapes: shapes.clone(),
            opacity,
        });
        if self.engine.undo_label() == Some("Transparency") {
            let _ = self.engine.undo();
        }
        let _ = self.engine.run_batch("Transparency", &cmds);
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
        self.run(Command::ExtractContents { clip: clip.id });
        self.clip_frame_edit = Some((clip.id, ids.clone()));
        self.select(ids);
        self.status = tr("status.editing_clip_frame");
    }

    /// Object > Clip to Frame > Finish Editing Contents: place the (surviving) contents
    /// back inside the frame.
    pub fn finish_clip_frame_edit(&mut self) {
        let Some((frame, ids)) = self.clip_frame_edit.take() else {
            return;
        };
        if self.doc().find_shape(frame).is_none() {
            return;
        }
        let contents: Vec<ShapeId> = ids
            .into_iter()
            .filter(|id| self.doc().find_shape(*id).is_some())
            .collect();
        if contents.is_empty() {
            return;
        }
        self.place_inside(contents, frame);
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
    /// the others are spread evenly along it.
    pub fn fit_objects_to_path(&mut self) {
        let shapes = self.selected_shapes();
        if shapes.len() < 2 {
            return;
        }
        let Some(path_shape) = shapes.last() else {
            return;
        };
        let path = path_shape.page_path();
        let mut pts: Vec<Point> = Vec::new();
        kurbo::flatten(path.elements().iter().copied(), 0.05, |el| match el {
            kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) => pts.push(p),
            _ => {}
        });
        if pts.len() < 2 {
            return;
        }
        let mut cum = vec![0.0];
        for w in pts.windows(2) {
            cum.push(cum.last().copied().unwrap_or(0.0) + (w[1] - w[0]).hypot());
        }
        let total = *cum.last().unwrap_or(&0.0);
        let objects = &shapes[..shapes.len() - 1];
        let n = objects.len();
        let mut cmds = Vec::new();
        for (i, s) in objects.iter().enumerate() {
            let t = if n == 1 {
                0.5
            } else {
                i as f64 / (n as f64 - 1.0)
            };
            let target = t * total;
            let k = cum
                .partition_point(|c| *c <= target)
                .saturating_sub(1)
                .min(pts.len() - 2);
            let seg = pts[k + 1] - pts[k];
            let len = seg.hypot().max(1e-9);
            let u = ((target - cum[k]) / len).clamp(0.0, 1.0);
            let pos = pts[k] + seg * u;
            let c = s.bounds().center();
            cmds.push(Command::TransformShapes {
                shapes: vec![s.id],
                transform: Affine::translate(pos - c),
            });
        }
        if let Err(e) = self.engine.run_batch("Fit Objects to Path", &cmds) {
            self.status = e.to_string();
        }
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
        // goes back to none.
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

    pub(crate) fn bitmap_shapes(&self) -> Vec<(ShapeId, Rect, u32, u32, Vec<u8>)> {
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
                png,
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
                    png,
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
            c.transform = Affine::translate((-bounds.x0, -bounds.y0)) * c.transform;
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
                png,
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
            png,
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
        let settings = prefs.settings(prefs.quick_preset);
        self.trace_bitmaps(&settings);
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

    /// Text > Edit Text: start in-place editing of the selected text.
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
        self.apply_paragraph_edit("Text Alignment", move |f| f.align = a);
    }

    /// Set which way the selected text runs (the paragraphs at the caret
    /// while text in a frame is typed), as [`App::apply_text_align`] does
    /// for the alignment. Right to left also aligns right what was left
    /// aligned, and left to right aligns left what was right aligned.
    pub fn apply_text_direction(&mut self, d: tracedraw_core::TextDirection) {
        use tracedraw_core::{TextAlign, TextDirection};
        self.apply_paragraph_edit("Text Direction", move |f| {
            f.direction = d;
            match (d, f.align) {
                (TextDirection::Rtl, TextAlign::Left | TextAlign::None) => {
                    f.align = TextAlign::Right
                }
                (TextDirection::Ltr, TextAlign::Right) => f.align = TextAlign::Left,
                _ => {}
            }
        });
    }

    /// Whether the text `s` (the paragraph at the caret while it is
    /// typed, else its first one) runs right to left: as set, or from its
    /// first strong letter. `None` for other shapes.
    pub fn text_runs_rtl(&self, s: &Shape) -> Option<bool> {
        use tracedraw_core::TextDirection;
        let (_, style) = self.paragraph_style_of(s)?;
        Some(match style.direction {
            TextDirection::Rtl => true,
            TextDirection::Ltr => false,
            TextDirection::Auto => {
                let ShapeKind::Text { spans, .. } = &s.kind else {
                    return None;
                };
                let text: String = spans.iter().map(|sp| sp.text.as_str()).collect();
                let caret = self
                    .text_edit
                    .as_ref()
                    .filter(|te| te.shape == s.id)
                    .map_or(0, |te| te.caret);
                // The paragraph at the caret.
                let mut start = 0;
                let mut para = "";
                for piece in text.split('\n') {
                    let n = piece.chars().count();
                    para = piece;
                    if caret <= start + n {
                        break;
                    }
                    start += n + 1;
                }
                first_strong_rtl(para)
            }
        })
    }

    /// Change the paragraph formatting of the selected text with `edit`,
    /// as one undo step named `label`: text typed at a point as a whole,
    /// text in a frame in the paragraphs at the caret. Text typed at a
    /// point moves back so the reference point of the X and Y fields stays
    /// where it was.
    pub fn apply_paragraph_edit(
        &mut self,
        label: &'static str,
        edit: impl Fn(&mut tracedraw_core::paragraphs::ParagraphFormat),
    ) {
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
            let point_text = matches!(&s.kind, ShapeKind::Text { frame: None, .. });
            let range = if point_text {
                None
            } else {
                self.paragraph_range(s.id)
            };
            tracedraw_core::paragraphs::edit_paragraph_formats(&mut kind, range, &edit);
            if kind == s.kind {
                continue;
            }
            // Text typed at a point lines up about that point, so a new
            // alignment would move it: it moves back so the reference
            // point of the X and Y fields stays where it was. Text in a
            // frame or on a path keeps its frame or path; its lines
            // realign inside.
            let anchored = matches!(
                &kind,
                ShapeKind::Text {
                    frame: None,
                    on_path: None,
                    ..
                }
            );
            let shift = if anchored {
                let before = s.bounds();
                let mut after = s.clone();
                after.kind = kind.clone();
                let after = after.bounds();
                let o = self.object_origin;
                let d = crate::ui::propbar::origin_point(before, o)
                    - crate::ui::propbar::origin_point(after, o);
                // The move in the object's parent's space (a group's).
                let parent = doc.parent_transform(s.id).unwrap_or(Affine::IDENTITY);
                let local = parent.inverse() * (Point::ZERO + d) - parent.inverse() * Point::ZERO;
                (local.hypot() > 1e-9).then_some(local)
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
            if let Err(e) = self.engine.run_batch(label, &cmds) {
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
    /// point.
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
        let ShapeKind::Text {
            spans,
            origin,
            align,
            para,
            ..
        } = text.kind.clone()
        else {
            return;
        };
        // The path turned and started as text sits on it (in page space,
        // where outside and top are seen), then in the text's local space.
        let (page_path, closed) = path_for_text_at(&curve.page_path(), at);
        let path = Affine::translate(-origin.to_vec2()) * (text.transform.inverse() * page_path);
        let (align, offset) = placement_on(&path, closed, align);
        self.run(Command::SetShapeKind {
            shape: text.id,
            kind: ShapeKind::Text {
                spans,
                origin,
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
            },
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
            let cmds = vec![
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

    /// The bottom palette strip shows the first visible palette.
    pub fn rebuild_palette(&mut self) {
        if let Some(i) = self.visible_palettes.first() {
            if let Some(p) = self.palettes.get(*i) {
                self.palette = p.colors.clone();
                self.palette_scroll = 0;
            }
        }
    }

    pub fn open_palette_file(&mut self) {
        crate::files::Dialog::new()
            .add_filter(tr("file.palettes"), &["tdpal", "gpl", "ase", "aco", "json"])
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

    pub fn step_and_repeat_apply(&mut self) {
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
                c.transform = Affine::translate((dx * k as f64, dy * k as f64)) * c.transform;
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
        def.transform = Affine::translate((-b.x0, -b.y0)) * def.transform;
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
    /// Fonts used by the document (in groups, tables and symbols too) that
    /// are neither installed nor carried by a drawing (Font Manager,
    /// missing-font substitution notice).
    pub fn check_missing_fonts(&mut self) {
        let missing: Vec<String> = self
            .doc()
            .font_families_used()
            .into_iter()
            .filter(|f| !self.font_families.contains(f))
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

#[cfg(test)]
mod tests {
    use super::*;

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
                png,
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
}

/// Whether the first letter of `text` with a direction of its own is a
/// right-to-left one (Hebrew, Arabic and the like); left to right when
/// there is none.
fn first_strong_rtl(text: &str) -> bool {
    for c in text.chars() {
        let r = matches!(c as u32,
            0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF);
        if r && c.is_alphabetic() {
            return true;
        }
        if c.is_alphabetic() {
            return false;
        }
    }
    false
}
