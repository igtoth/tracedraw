//! Operations added with the full menu structure: page numbers, order
//! relative to another object, effects on the stack, bitmap effects and
//! tracing, text conversions, scripts, palettes.

use crate::app::{App, EffectKind};
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
        let mut cmds = Vec::new();
        for s in shapes {
            let mut effects = s.effects.clone();
            if replace_same_kind {
                effects.retain(|e| std::mem::discriminant(e) != std::mem::discriminant(&effect));
            }
            effects.push(effect.clone());
            cmds.push(Command::SetEffects {
                shape: s.id,
                effects,
            });
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Effect", &cmds) {
                self.status = e.to_string();
            }
        }
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
        self.dialog = crate::ui::dialogs::Dialog::Symmetry;
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

    /// Copy the shadow or transparency of `source` to the selection.
    pub fn copy_effect_from(&mut self, source: ShapeId, kind: EffectKind) {
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
        match kind {
            EffectKind::Shadow => self.run(Command::SetShadow {
                shapes,
                shadow: src.shadow,
            }),
            EffectKind::Transparency => {
                self.run(Command::SetOpacity {
                    shapes: shapes.clone(),
                    opacity: src.opacity,
                });
                let t = src
                    .effects
                    .iter()
                    .find(|e| matches!(e, Effect::Transparency { .. }))
                    .cloned();
                for id in shapes {
                    let Some(s) = self.doc().find_shape(id).cloned() else {
                        continue;
                    };
                    let mut effects: Vec<Effect> = s
                        .effects
                        .iter()
                        .filter(|e| !matches!(e, Effect::Transparency { .. }))
                        .cloned()
                        .collect();
                    if let Some(t) = &t {
                        effects.push(t.clone());
                    }
                    self.run(Command::SetEffects { shape: id, effects });
                }
            }
        }
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

    /// Object > ClipFrame > Edit ClipFrame: take the contents out so they can
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

    /// Object > ClipFrame > Finish Editing: place the (surviving) contents
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

    /// Whether a ClipFrame's contents move with the frame (the default).
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

    /// Object > ClipFrame > Lock Contents: toggle for the selected clips.
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

    /// After moving unlocked ClipFrames by `t`, keep their contents in place.
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
            if let ShapeKind::Text {
                frame: Some(_),
                para,
                ..
            } = &s.kind
            {
                let mut p = para.clone();
                p.baseline_grid_mm = if p.baseline_grid_mm > 0.0 { 0.0 } else { g };
                self.set_paragraph_style(s.id, p);
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
        self.default_fill = Fill::Pattern(tracedraw_core::Pattern::vector_from_shapes(&shapes));
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

    /// The Shape tool's Join of two end nodes on different curves: one
    /// curve, the second one's path drawn on from the first one's end.
    pub fn join_two_curves(&mut self) {
        let shapes: Vec<Shape> = self
            .selected_shapes()
            .into_iter()
            .filter(|s| matches!(s.kind, ShapeKind::Path { closed: false, .. }))
            .collect();
        if shapes.len() < 2 {
            return;
        }
        let mut joined = BezPath::new();
        let mut first = true;
        for s in &shapes {
            let p = s.page_path();
            for el in p.elements() {
                match el {
                    kurbo::PathEl::MoveTo(pt) => {
                        if first {
                            joined.move_to(*pt);
                            first = false;
                        } else {
                            joined.line_to(*pt);
                        }
                    }
                    other => joined.push(*other),
                }
            }
        }
        let keep = shapes[0].id;
        let others: Vec<ShapeId> = shapes[1..].iter().map(|s| s.id).collect();
        let mut cmds = vec![
            Command::TransformShapes {
                shapes: vec![keep],
                transform: shapes[0].transform.inverse() * Affine::IDENTITY,
            },
            Command::SetShapeKind {
                shape: keep,
                kind: ShapeKind::Path {
                    path: joined,
                    closed: false,
                },
            },
        ];
        // The joined path is in page space: reset the kept shape's transform.
        cmds[0] = Command::TransformShapes {
            shapes: vec![keep],
            transform: shapes[0].transform.inverse(),
        };
        cmds.push(Command::DeleteShapes { shapes: others });
        if let Err(e) = self.engine.run_batch("Join Curves", &cmds) {
            self.status = e.to_string();
        }
        self.selection = vec![keep];
    }

    /// The Join Curves docker: join the open ends of the selected curves
    /// (and the open subpaths inside them) that lie within the gap
    /// tolerance, nearest first, with the chosen joint. The result takes
    /// the place and properties of the curve selected last. Returns the
    /// number of joints made.
    pub fn join_curves(&mut self) -> usize {
        let shapes: Vec<Shape> = self
            .selected_shapes()
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

    fn bitmap_shapes(&self) -> Vec<(ShapeId, Rect, u32, u32, Vec<u8>)> {
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

    fn replace_bitmap(&mut self, id: ShapeId, rect: Rect, img: &image::RgbaImage) {
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

    pub fn inflate_bitmap(&mut self, px: Option<u32>) {
        for (id, rect, w, h, png) in self.bitmap_shapes() {
            let Some(img) = crate::bitmap_fx::decode(&png) else {
                continue;
            };
            let pad = px.unwrap_or((w.max(h) / 10).max(4));
            let out = crate::bitmap_fx::inflate(&img, pad);
            let mm_x = rect.width() / w.max(1) as f64 * pad as f64;
            let mm_y = rect.height() / h.max(1) as f64 * pad as f64;
            let r = Rect::new(
                rect.x0 - mm_x,
                rect.y0 - mm_y,
                rect.x1 + mm_x,
                rect.y1 + mm_y,
            );
            self.replace_bitmap(id, r, &out);
        }
    }

    pub fn resample_bitmap(&mut self, dpi: f64) {
        for (id, rect, _, _, png) in self.bitmap_shapes() {
            let Some(img) = crate::bitmap_fx::decode(&png) else {
                continue;
            };
            let w = (rect.width() / 25.4 * dpi).round().max(1.0) as u32;
            let h = (rect.height() / 25.4 * dpi).round().max(1.0) as u32;
            let out = crate::bitmap_fx::resample(&img, w, h);
            self.replace_bitmap(id, rect, &out);
        }
    }

    pub fn straighten_bitmap(&mut self, degrees: f64) {
        for (id, rect, w, h, png) in self.bitmap_shapes() {
            let Some(img) = crate::bitmap_fx::decode(&png) else {
                continue;
            };
            let out = crate::bitmap_fx::rotate(&img, degrees as f32);
            let sx = rect.width() / w.max(1) as f64;
            let sy = rect.height() / h.max(1) as f64;
            let c = rect.center();
            let nw = out.width() as f64 * sx;
            let nh = out.height() as f64 * sy;
            let r = Rect::new(
                c.x - nw / 2.0,
                c.y - nh / 2.0,
                c.x + nw / 2.0,
                c.y + nh / 2.0,
            );
            self.replace_bitmap(id, r, &out);
        }
    }

    /// Bitmaps > Bitmap Mask: make the given colours transparent.
    pub fn bitmap_color_mask(&mut self, colors: &[Color], tolerance: u8) {
        let rgb: Vec<[u8; 3]> = colors.iter().map(|c| c.to_rgb8()).collect();
        for (id, rect, _, _, png) in self.bitmap_shapes() {
            let Some(img) = crate::bitmap_fx::decode(&png) else {
                continue;
            };
            let out = crate::bitmap_fx::color_mask(&img, &rgb, tolerance);
            self.replace_bitmap(id, rect, &out);
        }
    }

    /// Object > Convert to Bitmap: rasterise the selection at `dpi`.
    /// Rasterise the selection: PNG bytes, pixel size and the bounds in page space.
    pub fn render_selection_png(
        &self,
        dpi: f64,
        transparent: bool,
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

    pub fn convert_to_bitmap(&mut self, dpi: f64, transparent: bool) {
        let shapes = self.selected_shapes();
        let Some(layer) = self.active_layer() else {
            return;
        };
        let Some((png, w, h, bounds)) = self.render_selection_png(dpi, transparent) else {
            return;
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
        self.default_fill = Fill::Pattern(tracedraw_core::style::Pattern::Bitmap {
            png,
            width_px,
            height_px,
            size_mm: bounds.width().max(0.1),
        });
        self.status = tr("status.pattern_created");
    }

    // ----- tracing ---------------------------------------------------------------------

    pub fn quick_trace(&mut self) {
        let settings = crate::trace::Preset::Clipart.settings();
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
            if let ShapeKind::Text {
                frame: Some(_),
                para,
                ..
            } = &s.kind
            {
                let mut p = para.clone();
                p.fit_to_frame = !p.fit_to_frame;
                self.set_paragraph_style(s.id, p);
            }
        }
    }

    pub fn set_paragraph_style(&mut self, id: ShapeId, para: tracedraw_core::ParagraphStyle) {
        let Some(s) = self.doc().find_shape(id).cloned() else {
            return;
        };
        if let ShapeKind::Text {
            spans,
            origin,
            frame,
            align,
            on_path,
            ..
        } = s.kind
        {
            self.run(Command::SetShapeKind {
                shape: id,
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

    /// Text > Fit Text to Path: select a text and a curve.
    pub fn fit_text_to_path(&mut self) {
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
        // Path in the text's local space.
        let path = text.transform.inverse() * curve.page_path();
        let path = Affine::translate(-origin.to_vec2()) * path;
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
                    offset: 0.0,
                    distance: 0.0,
                    mirror: false,
                }),
            },
        });
        self.selection = vec![text.id];
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

    /// Apply the property bar text style (font, size, bold, italic, align)
    /// to every selected text object.
    pub fn apply_text_style(&mut self) {
        // While editing with a selection, only the selected characters change.
        if self.text_edit.as_ref().is_some_and(|te| te.has_selection()) {
            let (font, size, bold, italic, underline) = (
                self.text_font.clone(),
                self.text_size_pt,
                self.text_bold,
                self.text_italic,
                self.text_underline,
            );
            self.text_apply_style(move |sp| {
                sp.font_family = font.clone();
                sp.size_pt = size;
                sp.bold = bold;
                sp.italic = italic;
                sp.underline = underline;
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
                    para,
                    on_path,
                    ..
                } => {
                    let mut spans = spans.clone();
                    for sp in spans.iter_mut() {
                        sp.font_family = self.text_font.clone();
                        sp.size_pt = self.text_size_pt;
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
                            align: self.text_align,
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

    /// Replace the spans of a text object (Text docker edits).
    pub fn set_text_spans(&mut self, id: ShapeId, spans: Vec<TextSpan>) {
        let Some(s) = self.doc().find_shape(id).cloned() else {
            return;
        };
        if let ShapeKind::Text {
            origin,
            frame,
            align,
            para,
            on_path,
            ..
        } = s.kind
        {
            self.run(Command::SetShapeKind {
                shape: id,
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
    /// Blend the two selected objects: the first becomes the start with a
    /// live Blend effect, the second is removed (it lives inside the effect).
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
        let end = shapes[1].clone();
        let mut effects = start.effects.clone();
        effects.retain(|e| !matches!(e, Effect::Blend { .. }));
        effects.push(Effect::Blend {
            end: Box::new(end.clone()),
            steps,
            accel_objects,
            accel_colors,
            rotation,
            path: None,
            rotate_on_path: false,
        });
        let cmds = vec![
            Command::SetEffects {
                shape: start.id,
                effects,
            },
            Command::DeleteShapes {
                shapes: vec![end.id],
            },
        ];
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

    /// Envelope presets applied to the selection: 0 arch, 1 bulge, 2 flag,
    /// 3 perspective, 4 circle.
    pub fn apply_envelope_preset(&mut self, preset: usize) {
        for s in self.selected_shapes() {
            let b = s.local_path().bounding_box();
            let mut n = tracedraw_core::live::envelope_default(b);
            let h = b.height();
            let w = b.width();
            match preset {
                0 => n[5].y += h * 0.3,
                1 => {
                    n[1].y -= h * 0.2;
                    n[5].y += h * 0.2;
                    n[3].x += w * 0.2;
                    n[7].x -= w * 0.2;
                }
                2 => {
                    n[1].y -= h * 0.2;
                    n[5].y -= h * 0.2;
                    n[0].y += h * 0.1;
                    n[6].y += h * 0.1;
                }
                3 => {
                    n[4].x -= w * 0.2;
                    n[6].x += w * 0.2;
                }
                _ => {
                    let k = 0.4142; // (sqrt2 - 1) so midpoints sit on the circle
                    n[1].y -= h * k * 0.5;
                    n[5].y += h * k * 0.5;
                    n[3].x += w * k * 0.5;
                    n[7].x -= w * k * 0.5;
                }
            }
            let mut effects = s.effects.clone();
            effects.retain(|e| !matches!(e, Effect::Envelope { .. }));
            effects.push(Effect::Envelope {
                nodes: n,
                keep_lines: self.envelope_keep_lines,
            });
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
    /// Fonts used by the document that are not installed (Font Manager,
    /// missing-font substitution notice).
    pub fn check_missing_fonts(&mut self) {
        let mut missing = Vec::new();
        for s in self.doc().all_layers().flat_map(|l| &l.shapes) {
            if let ShapeKind::Text { spans, .. } = &s.kind {
                for sp in spans {
                    if !self.font_families.contains(&sp.font_family)
                        && !missing.contains(&sp.font_family)
                    {
                        missing.push(sp.font_family.clone());
                    }
                }
            }
        }
        if !missing.is_empty() {
            self.status = trf("status.missing_fonts", &[("f", &missing.join(", "))]);
        }
        self.missing_fonts = missing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_modes() {
        assert_eq!(
            apply_case("hello world. bye", CaseMode::Sentence),
            "Hello world. Bye"
        );
        assert_eq!(apply_case("hello world", CaseMode::Title), "Hello World");
        assert_eq!(apply_case("aBc", CaseMode::Toggle), "AbC");
    }
}
