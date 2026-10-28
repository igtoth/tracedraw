//! The Edit Fill dialog (F11, #361): the fill types as pictures across the
//! top, then the chosen type's settings. A gradient shows a preview of the
//! fill, the fill picker with the built-in gradients, its type, how it
//! repeats, the colour band with its nodes (a click picks one, a drag
//! moves it, a double click on the band adds one, on a node removes it),
//! the picked node's colour, transparency, position and midpoint, Reverse,
//! and the transform (rotation, centre, edge pad). The preview has the
//! start, end and skew nodes the Interactive Fill tool shows on the
//! canvas: a drag there moves, turns and stretches the gradient, in step
//! with the rotation and centre fields (#939). Uniform colours get the
//! colour viewer; patterns, textures and meshes their editors. The
//! selection shows the fill as it is edited; OK applies what was changed,
//! with Overprint fill, in one undo step, and Cancel takes it back; with
//! nothing selected it sets the fill of new objects. See
//! docs/behavior/gradient-fill.md.

use crate::app::App;
use crate::fill_tool::{fountain_handles, moved_nodes, FountainHandle};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Color32, Pos2, Rect, Sense, Stroke as EStroke, Ui, Vec2};
use tracedraw_core::geometry::{Point as GPoint, Rect as GRect, Vec2 as GVec2};
use tracedraw_core::patch::{FillField, FillMix};
use tracedraw_core::style::{Fountain, FountainHandles, FountainKind, FountainRepeat, Stop};
use tracedraw_core::{Color, Fill};

/// What the dialog keeps while it is open.
#[derive(Debug, Clone, PartialEq)]
pub struct EditFillState {
    /// The fill as it was (the first object's, or the default).
    pub old: Fill,
    /// The fill being edited.
    pub fill: Fill,
    pub mix: FillMix,
    /// Fields set in the dialog that apply even when equal to the value
    /// shown (a type picked for objects of other types).
    pub touched: Vec<FillField>,
    /// The picked node of a gradient (an index into its stops).
    pub node: usize,
    /// Overprint fill as shown (`None`: the objects differ) and as it was.
    pub overprint: Option<bool>,
    pub overprint_old: Option<bool>,
    /// Nothing is selected: the dialog edits the default fill.
    pub defaults: bool,
    /// The hue the colour viewer shows (kept for greys).
    pub hue: f32,
    /// What the selection shows meanwhile: the fill, the fields set and
    /// Overprint fill last previewed.
    pub shown: Option<(Fill, Vec<FillField>, Option<bool>)>,
    /// The box the preview draws the fill on (see [`preview_bounds`]).
    pub bounds: GRect,
    /// The gradient node a drag in the preview holds, and how far it was
    /// from the press (it keeps that distance from the pointer).
    pub drag: Option<(FountainHandle, Vec2)>,
}

/// Width of the band of colours.
const BAND_W: f32 = 320.0;

/// The room the chosen kind's settings get under the kinds row: the same
/// for every kind, so the window keeps its size and OK and Cancel their
/// place as the kind changes (#654). Sized for the gradient page, the
/// largest; a page that needs more (a long translation) scrolls in it.
pub(crate) const PAGE: egui::Vec2 = egui::Vec2::new(724.0, 330.0);

impl App {
    /// Open the Edit Fill dialog on the selection's fill (or the default
    /// fill when nothing is selected).
    pub fn open_edit_fill(&mut self) {
        use tracedraw_core::geometry::Shape as _;
        let targets = self.paint_targets();
        let defaults = targets.is_empty();
        let bounds = preview_bounds(targets.first().map(|s| s.page_path().bounding_box()));
        let old = match targets.first() {
            Some(s) => s.fill.clone(),
            None => self.default_fill(),
        };
        let mix = if defaults {
            FillMix::default()
        } else {
            self.selection_fill_mix()
        };
        let overprint_old = if defaults {
            Some(false)
        } else {
            let on = targets.iter().filter(|s| s.overprint_fill).count();
            if on == 0 {
                Some(false)
            } else if on == targets.len() {
                Some(true)
            } else {
                None
            }
        };
        let hue = old
            .preview_color()
            .map_or(0.0, |c| crate::ui::color_picker::viewer_point(c).0);
        self.dialog = crate::ui::dialogs::Dialog::EditFill(Box::new(EditFillState {
            fill: old.clone(),
            old,
            mix,
            touched: Vec::new(),
            node: 0,
            overprint: overprint_old,
            overprint_old,
            defaults,
            hue,
            shown: None,
            bounds,
            drag: None,
        }));
    }

    /// Show what the dialog holds on the selection while it is open, as
    /// the reference program does: the change is taken back and made
    /// anew each time the dialog changes, OK keeps it as one step and
    /// Cancel, Esc or the close button take it back (#436).
    pub fn preview_edit_fill(&mut self, st: &mut EditFillState) {
        if st.defaults {
            return;
        }
        let now = (st.fill.clone(), st.touched.clone(), st.overprint);
        if st.shown.as_ref() == Some(&now) {
            return;
        }
        self.end_edit_fill_preview();
        let unchanged =
            st.fill == st.old && st.touched.is_empty() && st.overprint == st.overprint_old;
        if !unchanged {
            self.engine.begin_preview();
            self.fill_preview = true;
            self.apply_edit_fill(st);
        }
        st.shown = Some(now);
    }

    /// Take back the fill the Edit Fill dialog shows on the selection.
    pub fn end_edit_fill_preview(&mut self) {
        if std::mem::take(&mut self.fill_preview) {
            self.engine.withdraw_preview();
        }
    }

    /// OK in the Edit Fill dialog.
    pub fn apply_edit_fill(&mut self, st: &EditFillState) {
        if st.defaults {
            if st.fill != st.old {
                self.request_default_change(crate::defaults::DefaultChange::Fill(st.fill.clone()));
            }
            return;
        }
        let before = self.engine.last_step_id();
        if st.fill != st.old || !st.touched.is_empty() {
            self.patch_selection_fill(&st.old, &st.fill, &st.touched);
        }
        if let Some(on) = st.overprint.filter(|_| st.overprint != st.overprint_old) {
            let ids: Vec<_> = self.paint_targets().iter().map(|s| s.id).collect();
            let cmd = tracedraw_core::Command::SetOverprint {
                shapes: ids,
                fill: Some(on),
                outline: None,
            };
            if self.engine.last_step_id() != before {
                let _ = self.engine.amend(&cmd);
            } else {
                self.run(cmd);
            }
        }
    }
}

/// The dialog's window.
pub fn dialog(app: &mut App, ctx: &egui::Context, st: &mut EditFillState, close: &mut bool) {
    let mut ok = false;
    let mut help = false;
    crate::ui::dialogs::window(ctx, tr("edit_fill.title")).show(ctx, |ui| {
        kinds(ui, st);
        ui.separator();
        page(ui, st);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            help = ui
                .add_sized([27.0, 27.0], egui::Button::new("?"))
                .rich_tip(tr("dialog.help"))
                .clicked();
            if !st.defaults {
                let mut on = st.overprint.unwrap_or(false);
                if ui
                    .add(
                        egui::Checkbox::new(&mut on, tr("edit_fill.overprint"))
                            .indeterminate(st.overprint.is_none()),
                    )
                    .changed()
                {
                    st.overprint = Some(on);
                }
            }
            ok = crate::ui::dialogs::ok_cancel(ui, close);
        });
    });
    if help {
        app.open_dialog_help();
    }
    if ok {
        app.end_edit_fill_preview();
        app.apply_edit_fill(st);
    } else if *close {
        app.end_edit_fill_preview();
    } else {
        app.preview_edit_fill(st);
    }
}

/// The chosen kind's settings in a box of [`PAGE`] size.
fn page(ui: &mut Ui, st: &mut EditFillState) {
    let kind = tracedraw_core::patch::fill_kind(&st.fill);
    ui.allocate_ui_with_layout(PAGE, egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_min_size(PAGE);
        // Both ways: a page too wide would widen a scroll area that only
        // scrolls up and down, and the window with it.
        egui::ScrollArea::both()
            .id_salt(("edit_fill_page", kind))
            .max_width(PAGE.x)
            .max_height(PAGE.y)
            .auto_shrink([false, false])
            .show(ui, |ui| page_contents(ui, st));
    });
}

/// The settings of the fill's kind (what [`page`] boxes).
fn page_contents(ui: &mut Ui, st: &mut EditFillState) {
    match &mut st.fill {
        Fill::None => {
            ui.label(egui::RichText::new(tr("edit_fill.no_fill")).color(Tokens::text_dim()));
        }
        Fill::Solid(_) => uniform(ui, st),
        Fill::Fountain(_) => fountain(ui, st),
        Fill::Pattern(p) => {
            ui.horizontal_top(|ui| {
                preview(ui, &Fill::Pattern(p.clone()));
                ui.vertical(|ui| crate::ui::dockers::pattern_editor(ui, p));
            });
        }
        Fill::Texture(t) => {
            ui.horizontal_top(|ui| {
                preview(ui, &Fill::Texture(t.clone()));
                ui.vertical(|ui| crate::ui::dockers::texture_editor(ui, t));
            });
        }
        Fill::Mesh(_) => {
            ui.label(egui::RichText::new(tr("docker.mesh_hint")).color(Tokens::text_dim()));
        }
    }
}

/// The fill types as pictures: none, uniform, gradient, pattern, texture,
/// mesh.
fn kinds(ui: &mut Ui, st: &mut EditFillState) {
    let kind = tracedraw_core::patch::fill_kind(&st.fill);
    let shown = (!st.mix.kind || !st.touched.is_empty()).then_some(kind);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (i, key) in [
            "docker.fill_none",
            "docker.fill_uniform",
            "docker.fill_fountain",
            "docker.fill_pattern",
            "docker.fill_texture",
            "docker.fill_mesh",
        ]
        .iter()
        .enumerate()
        {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(34.0, 30.0), Sense::click());
            if shown == Some(i) {
                ui.painter().rect_filled(rect, 3.0, Tokens::tool_active());
            } else if resp.hovered() {
                ui.painter().rect_filled(rect, 3.0, Tokens::tool_hover());
            }
            crate::ui::dockers::fill_kind_picture(
                ui.painter(),
                Rect::from_center_size(rect.center(), Vec2::splat(18.0)),
                i,
            );
            if resp.rich_tip(tr(key)).clicked() && shown != Some(i) {
                st.fill = new_of_kind(&st.fill, i);
                st.touched = vec![FillField::All];
                st.node = 0;
            }
        }
    });
}

/// A fill of kind `i` (as [`kinds`] lists them) from `from`'s colour.
pub fn new_of_kind(from: &Fill, i: usize) -> Fill {
    use tracedraw_core::style::{Pattern, PatternTile, Texture, TextureKind};
    let base = from
        .preview_color()
        .unwrap_or(Color::cmyk_pct(0.0, 0.0, 0.0, 20.0));
    match i {
        0 => Fill::None,
        1 => Fill::Solid(base),
        2 => Fill::linear(base, Color::WHITE, 0.0),
        3 => Fill::Pattern(Pattern::TwoColor {
            tile: PatternTile::Checker,
            front: base,
            back: Color::WHITE,
            size_mm: 10.0,
        }),
        4 => Fill::Texture(Texture {
            kind: TextureKind::Clouds,
            color_a: base,
            color_b: Color::WHITE,
            scale: 20.0,
            seed: 1,
        }),
        _ => Fill::Mesh(tracedraw_core::Mesh::new(
            tracedraw_core::geometry::Rect::new(0.0, 0.0, 100.0, 100.0),
            2,
            2,
            base,
        )),
    }
}

/// A uniform colour: the viewer, the colour's model and its values.
fn uniform(ui: &mut Ui, st: &mut EditFillState) {
    let Fill::Solid(c) = &mut st.fill else {
        return;
    };
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(260.0);
            if let Some((h, s, b)) = crate::ui::color_docker::viewer(ui, st.hue, *c) {
                st.hue = h;
                *c = crate::ui::color_picker::from_viewer(
                    h,
                    s,
                    b,
                    crate::ui::color_picker::edit_model(*c),
                );
            }
        });
        ui.vertical(|ui| {
            // The colour's model and its values to type or drag, as the
            // colour picker has them (#468).
            ui.horizontal(|ui| {
                ui.label(tr("edit_fill.color"));
                crate::ui::color_picker::color_button(ui, c);
                // A colour picked from the drawing (#814).
                crate::ui::color_picker::eyedropper(ui, egui::Id::new("edit_fill_pick"), c);
            });
            ui.horizontal(|ui| {
                ui.label(tr("docker.model"));
                crate::ui::color_picker::model_list(ui, egui::Id::new("edit_fill_model"), c);
            });
            let mut m = c.convert_to(crate::ui::color_picker::edit_model(*c));
            if crate::ui::color_docker::sliders(ui, &mut m) {
                *c = m;
            }
            ui.label(
                egui::RichText::new(c.to_hex())
                    .monospace()
                    .color(Tokens::text_dim()),
            );
        });
    });
}

/// A gradient: preview and fill picker on the left; type, repeat, band,
/// node and transform on the right.
fn fountain(ui: &mut Ui, st: &mut EditFillState) {
    let Fill::Fountain(f) = &mut st.fill else {
        return;
    };
    st.node = st.node.min(f.stops.len().saturating_sub(1));
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(200.0);
            fountain_preview(ui, f, st.bounds, &mut st.drag);
            ui.add_space(4.0);
            ui.label(tr("edit_fill.fill_picker"));
            picker(ui, f);
        });
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("docker.type"));
                for k in FountainKind::ALL {
                    if ui
                        .selectable_label(f.kind == k, tr(crate::ui::dockers::fountain_kind_key(k)))
                        .clicked()
                    {
                        f.kind = k;
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("docker.repeat"));
                for (m, key) in [
                    (FountainRepeat::None, "edit_fill.repeat_default"),
                    (FountainRepeat::Mirror, "edit_fill.repeat_mirror"),
                    (FountainRepeat::Repeat, "edit_fill.repeat"),
                ] {
                    if ui.selectable_label(f.repeat == m, tr(key)).clicked() {
                        f.repeat = m;
                        if m != FountainRepeat::None && f.repeats < 2 {
                            f.repeats = 2;
                        }
                    }
                }
                if f.repeat != FountainRepeat::None {
                    ui.add(
                        crate::ui::field::NumField::new(&mut f.repeats)
                            .range(1..=Fountain::MAX_REPEATS),
                    );
                }
            });
            ui.add_space(4.0);
            band(ui, f, &mut st.node);
            node_fields(ui, f, st.node);
            ui.add_space(4.0);
            crate::ui::chrome::section(ui, &tr("edit_fill.transform"));
            transform(ui, f, st.bounds);
        });
    });
}

/// The fill drawn on a square, with a checker behind what is clear.
fn preview(ui: &mut Ui, fill: &Fill) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(180.0), Sense::hover());
    draw_fill(ui, rect, fill, GRect::new(0.0, 0.0, PREVIEW_MM, PREVIEW_MM));
}

/// `fill` rendered on the page box `b` and drawn on `rect`, with a
/// checker behind what is clear.
fn draw_fill(ui: &mut Ui, rect: Rect, fill: &Fill, b: GRect) {
    let size = rect.size();
    let key = egui::Id::new((
        "edit_fill_preview",
        format!("{fill:?}"),
        b.width().to_bits(),
        b.height().to_bits(),
    ));
    // One texture kept, made anew when the fill or the box changes (a
    // drag in the preview changes it every frame).
    let slot = egui::Id::new("edit_fill_preview_tex");
    let kept: Option<(egui::Id, egui::TextureHandle)> = ui.data(|d| d.get_temp(slot));
    let tex = kept.filter(|(k, _)| *k == key).map(|(_, t)| t);
    let tex = tex.or_else(|| {
        let pm = tracedraw_render::render_fill_image(fill, b, 90.0)?;
        let img = egui::ColorImage::from_rgba_premultiplied(
            [pm.width() as usize, pm.height() as usize],
            pm.data(),
        );
        let t = ui
            .ctx()
            .load_texture("edit_fill_preview", img, egui::TextureOptions::LINEAR);
        ui.data_mut(|d| d.insert_temp(slot, (key, t.clone())));
        Some(t)
    });
    let p = ui.painter();
    // A checker for what the fill leaves clear.
    let cell = 10.0;
    for y in 0..(size.y / cell).ceil() as i32 {
        for x in 0..(size.x / cell).ceil() as i32 {
            let c = if (x + y) % 2 == 0 {
                Color32::from_gray(255)
            } else {
                Color32::from_gray(220)
            };
            p.rect_filled(
                Rect::from_min_size(
                    rect.min + Vec2::new(x as f32 * cell, y as f32 * cell),
                    Vec2::splat(cell),
                )
                .intersect(rect),
                0.0,
                c,
            );
        }
    }
    if let Some(t) = tex {
        p.image(
            t.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    p.rect_stroke(
        rect,
        0.0,
        EStroke::new(1.0, Tokens::border()),
        egui::StrokeKind::Inside,
    );
}

/// Longest side of the box the gradient preview is drawn on (mm).
const PREVIEW_MM: f64 = 50.0;

/// Room around the gradient preview's box for nodes just outside it.
const PREVIEW_MARGIN: f32 = 14.0;

/// Screen pixels within which a press in the preview takes a node.
const PREVIEW_PICK: f32 = 8.0;

/// The box the gradient preview draws the fill on: the object's bounds
/// scaled to a longest side of 50 mm, so the nodes, the angle and the
/// centre are where they are on the object (they are relative to its
/// box); a square for no object or a flat one. A side is kept at least an
/// eighth of the other so a thin object still shows a preview.
pub fn preview_bounds(b: Option<GRect>) -> GRect {
    let square = GRect::new(0.0, 0.0, PREVIEW_MM, PREVIEW_MM);
    let Some(b) = b else {
        return square;
    };
    let (w, h) = (b.width(), b.height());
    if !(w.is_finite() && h.is_finite() && w > 1e-9 && h > 1e-9) {
        return square;
    }
    let m = w.max(h);
    GRect::new(
        0.0,
        0.0,
        (w / m).max(0.125) * PREVIEW_MM,
        (h / m).max(0.125) * PREVIEW_MM,
    )
}

/// Where the preview's page box `page` is on the screen (`screen`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewMap {
    pub screen: Rect,
    pub page: GRect,
}

impl PreviewMap {
    /// The box `page` as large as it fits in `area`, centred.
    pub fn fit(area: Rect, page: GRect) -> Self {
        let (w, h) = (page.width().max(1e-9), page.height().max(1e-9));
        let k = (f64::from(area.width()) / w).min(f64::from(area.height()) / h);
        let size = Vec2::new((w * k) as f32, (h * k) as f32);
        PreviewMap {
            screen: Rect::from_center_size(area.center(), size),
            page,
        }
    }

    /// A page point on the screen (the page's y runs up).
    pub fn to_screen(&self, p: GPoint) -> Pos2 {
        let (s, b) = (self.screen, self.page);
        Pos2::new(
            s.left() + ((p.x - b.x0) / b.width().max(1e-9)) as f32 * s.width(),
            s.top() + ((b.y1 - p.y) / b.height().max(1e-9)) as f32 * s.height(),
        )
    }

    /// A screen point on the page.
    pub fn to_page(&self, q: Pos2) -> GPoint {
        let (s, b) = (self.screen, self.page);
        let u = f64::from((q.x - s.left()) / s.width().max(1e-6));
        let v = f64::from((q.y - s.top()) / s.height().max(1e-6));
        GPoint::new(b.x0 + u * b.width(), b.y1 - v * b.height())
    }
}

/// Move node `h` of the gradient to `p` (a point of the preview's box
/// `b`), as the Interactive Fill tool moves it on the canvas: the start
/// moves the fill (the centre of an elliptical, conical or square one
/// moves it whole, which is its offset while its nodes are not placed
/// freely), the end turns and stretches the axis, the skew handle tilts
/// it. With `snap` the end turns in steps of 15 degrees. A shaped fill
/// has no nodes.
pub fn drag_preview_node(f: &mut Fountain, b: GRect, h: FountainHandle, p: GPoint, snap: bool) {
    if f.kind == FountainKind::Shaped || !(p.x.is_finite() && p.y.is_finite()) {
        return;
    }
    let centred = matches!(
        f.kind,
        FountainKind::Radial | FountainKind::Conical | FountainKind::Square
    );
    if h == FountainHandle::Start && centred && f.handles.is_none() {
        let r = Fountain::page_to_rel(b, p);
        f.offset = GPoint::new(r.x.clamp(-1.0, 1.0), r.y.clamp(-1.0, 1.0));
        return;
    }
    let pts = fountain_handles(f, b);
    let [(_, s), (_, e), (_, k)] = pts.as_slice() else {
        return;
    };
    let mut p = p;
    let mut snapped = None;
    if snap && h == FountainHandle::End {
        let d = p - *s;
        let len = d.hypot();
        if len > 1e-9 {
            let step = 15.0;
            let a = (d.y.atan2(d.x).to_degrees() / step).round() * step;
            let r = a.to_radians();
            p = *s + GVec2::new(r.cos(), r.sin()) * len;
            snapped = Some(if a > 180.0 { a - 360.0 } else { a });
        }
    }
    let (s, e, k) = moved_nodes(f.kind, (*s, *e, *k), h, p);
    let rel = |q: GPoint| Fountain::page_to_rel(b, q);
    f.set_handles(
        b,
        FountainHandles {
            start: rel(s),
            end: rel(e),
            skew: rel(k),
        },
    );
    if let Some(a) = snapped {
        f.angle = a;
    }
}

/// Turn nodes placed freely by `deg` about the start (on the box `b`), as
/// a change of the rotation field turns the fill; nothing without them.
pub fn turn_nodes(f: &mut Fountain, b: GRect, deg: f64) {
    let Some(h) = f.handles else {
        return;
    };
    if !deg.is_finite() {
        return;
    }
    let s = Fountain::rel_to_page(b, h.start);
    let (c, sn) = (deg.to_radians().cos(), deg.to_radians().sin());
    let turn = |q: GPoint| {
        let v = Fountain::rel_to_page(b, q) - s;
        Fountain::page_to_rel(b, s + GVec2::new(v.x * c - v.y * sn, v.x * sn + v.y * c))
    };
    f.handles = Some(FountainHandles {
        start: h.start,
        end: turn(h.end),
        skew: turn(h.skew),
    });
}

/// Move nodes placed freely by `(dx, dy)` of the box's half sides, as a
/// change of the centre fields moves the fill; nothing without them.
pub fn shift_nodes(f: &mut Fountain, dx: f64, dy: f64) {
    let Some(h) = &mut f.handles else {
        return;
    };
    if !(dx.is_finite() && dy.is_finite()) {
        return;
    }
    for q in [&mut h.start, &mut h.end, &mut h.skew] {
        *q = GPoint::new(q.x + dx, q.y + dy);
    }
}

/// The gradient preview with its nodes on it: a drag on a node moves it
/// (see [`drag_preview_node`]); a drag anywhere else moves the start (the
/// centre of the fill) by as much as the pointer moves.
fn fountain_preview(
    ui: &mut Ui,
    f: &mut Fountain,
    b: GRect,
    drag: &mut Option<(FountainHandle, Vec2)>,
) {
    let (area, resp) = ui.allocate_exact_size(Vec2::new(200.0, 180.0), Sense::drag());
    let map = PreviewMap::fit(area.shrink(PREVIEW_MARGIN), b);
    ui.data_mut(|d| d.insert_temp(egui::Id::new("edit_fill_preview_box"), map.screen));
    let nearest = |f: &Fountain, at: Pos2| {
        fountain_handles(f, b)
            .into_iter()
            .map(|(h, p)| (h, map.to_screen(p).distance(at)))
            .filter(|(_, d)| *d <= PREVIEW_PICK)
            .min_by(|x, y| x.1.total_cmp(&y.1))
            .map(|(h, _)| h)
    };
    if resp.drag_started() && f.kind != FountainKind::Shaped {
        *drag = ui.input(|i| i.pointer.press_origin()).and_then(|at| {
            let h = nearest(f, at).unwrap_or(FountainHandle::Start);
            let node = fountain_handles(f, b)
                .into_iter()
                .find(|(n, _)| *n == h)
                .map(|(_, p)| map.to_screen(p))?;
            Some((h, node - at))
        });
    }
    if resp.dragged() {
        if let (Some((h, grab)), Some(at)) = (*drag, resp.interact_pointer_pos()) {
            let snap = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
            drag_preview_node(f, b, h, map.to_page(at + grab), snap);
        }
    }
    if resp.drag_stopped() {
        *drag = None;
    }
    if drag.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if resp.hover_pos().and_then(|at| nearest(f, at)).is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    draw_fill(ui, map.screen, &Fill::Fountain(f.clone()), b);
    let pts = fountain_handles(f, b);
    let [(_, s), (_, e), (_, k)] = pts.as_slice() else {
        resp.rich_tip(tr("edit_fill.preview_tip"));
        return;
    };
    let (s, e, k) = (map.to_screen(*s), map.to_screen(*e), map.to_screen(*k));
    // Plain line segments, a light one under a dark one, so they show on
    // any colour.
    let p = ui.painter_at(area);
    let light = EStroke::new(3.0, Color32::WHITE);
    let dark = EStroke::new(1.0, Color32::from_gray(40));
    for (a, z) in [(s, k), (s, e)] {
        p.line_segment([a, z], light);
        p.line_segment([a, z], dark);
    }
    let rgb = |c: Option<&Stop>| {
        let [r, g, bl] = c.map_or([255, 255, 255], |s| s.color.to_rgb8());
        Color32::from_rgb(r, g, bl)
    };
    let ring = |h: FountainHandle| {
        if drag.is_some_and(|(d, _)| d == h) {
            EStroke::new(2.0, Tokens::ACCENT)
        } else {
            EStroke::new(1.0, Color32::from_gray(40))
        }
    };
    for (at, h, c) in [
        (s, FountainHandle::Start, rgb(f.stops.first())),
        (e, FountainHandle::End, rgb(f.stops.last())),
    ] {
        let r = Rect::from_center_size(at, Vec2::splat(9.0));
        p.rect_filled(r.expand(1.0), 0.0, Color32::WHITE);
        p.rect_filled(r, 0.0, c);
        p.rect_stroke(r, 0.0, ring(h), egui::StrokeKind::Outside);
    }
    p.circle_filled(k, 4.5, Color32::WHITE);
    p.circle_stroke(k, 4.5, ring(FountainHandle::Skew));
    resp.rich_tip(tr("edit_fill.preview_tip"));
}

/// The built-in gradients; a click takes its colours.
fn picker(ui: &mut Ui, f: &mut Fountain) {
    egui::ScrollArea::vertical()
        .id_salt("edit_fill_picker")
        .max_height(110.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(3.0);
                for (i, (key, _)) in crate::gradient_presets::PRESETS.iter().enumerate() {
                    let Some(stops) = crate::gradient_presets::stops(i) else {
                        continue;
                    };
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(36.0, 22.0), Sense::click());
                    crate::ui::dockers::gradient_swatch(ui.painter(), r, &stops);
                    let line = if resp.hovered() {
                        Tokens::ACCENT
                    } else {
                        Tokens::border()
                    };
                    ui.painter().rect_stroke(
                        r,
                        2.0,
                        EStroke::new(1.0, line),
                        egui::StrokeKind::Inside,
                    );
                    if resp.rich_tip(tr(key)).clicked() {
                        f.stops = stops;
                        if let Some(a) = crate::gradient_presets::angle(i)
                            .filter(|_| f.kind == FountainKind::Linear)
                        {
                            f.angle = a;
                        }
                    }
                }
            });
        });
}

/// Where stop position `t` is across the band `r`.
fn band_x(r: Rect, t: f64) -> f32 {
    r.left() + r.width() * t.clamp(0.0, 1.0) as f32
}

/// The band of colours with the nodes under it.
fn band(ui: &mut Ui, f: &mut Fountain, node: &mut usize) {
    let (area, resp) = ui.allocate_exact_size(Vec2::new(BAND_W, 40.0), Sense::click_and_drag());
    let band = Rect::from_min_size(
        area.min + Vec2::new(6.0, 2.0),
        Vec2::new(BAND_W - 12.0, 22.0),
    );
    let p = ui.painter();
    crate::ui::dockers::gradient_swatch(p, band, &f.blended_stops());
    p.rect_stroke(
        band,
        0.0,
        EStroke::new(1.0, Tokens::border()),
        egui::StrokeKind::Inside,
    );
    // The nodes: small squares in their colour below the band, the
    // picked one with a mark above it.
    let node_rect = |t: f64| {
        Rect::from_center_size(
            Pos2::new(band_x(band, t), band.bottom() + 8.0),
            Vec2::splat(9.0),
        )
    };
    for (i, s) in f.stops.iter().enumerate() {
        let r = node_rect(s.pos);
        let [cr, cg, cb] = s.color.to_rgb8();
        p.rect_filled(r, 0.0, Color32::from_rgb(cr, cg, cb));
        let stroke = if i == *node {
            EStroke::new(2.0, Tokens::ACCENT)
        } else {
            EStroke::new(1.0, Tokens::text_dim())
        };
        p.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Outside);
    }
    let at = resp.interact_pointer_pos();
    let hit = |at: Pos2| {
        f.stops
            .iter()
            .position(|s| node_rect(s.pos).expand(3.0).contains(at))
    };
    let t_at = |at: Pos2| f64::from((at.x - band.left()) / band.width()).clamp(0.0, 1.0);
    if resp.double_clicked() {
        if let Some(at) = at {
            match hit(at) {
                Some(i) if i > 0 && i + 1 < f.stops.len() => {
                    f.stops.remove(i);
                    *node = i - 1;
                }
                Some(_) => {}
                None => {
                    if let Some(i) = add_node(f, t_at(at)) {
                        *node = i;
                    }
                }
            }
        }
    } else if resp.drag_started() || resp.clicked() {
        if let Some(i) = at.and_then(hit) {
            *node = i;
        }
    }
    if resp.dragged() {
        if let Some(at) = at {
            move_node(f, *node, t_at(at));
        }
    }
    ui.horizontal(|ui| {
        if ui
            .button(tr("edit_fill.reverse"))
            .rich_tip(tr("edit_fill.reverse_tip"))
            .clicked()
        {
            reverse(f);
            *node = f.stops.len().saturating_sub(1) - (*node).min(f.stops.len().saturating_sub(1));
        }
        // Wrapped: a long translation must not widen the page.
        ui.add(
            egui::Label::new(
                egui::RichText::new(tr("edit_fill.band_hint"))
                    .size(11.0)
                    .color(Tokens::text_dim()),
            )
            .wrap(),
        );
    });
}

/// Add a node at `t` in the colour and opacity the gradient has there;
/// its index (none when a node is there already).
pub fn add_node(f: &mut Fountain, t: f64) -> Option<usize> {
    let i = f.stops.iter().position(|s| s.pos > t)?;
    if i == 0 || f.stops[i].pos - t < 1e-3 || t - f.stops[i - 1].pos < 1e-3 {
        return None;
    }
    let at = tracedraw_core::style::sample_stop(&f.blended_stops(), t);
    f.stops.insert(
        i,
        Stop {
            opacity: at.opacity,
            ..Stop::new(t, at.color)
        },
    );
    Some(i)
}

/// Move node `i` to `t`, kept between its neighbours (the ends stay).
pub fn move_node(f: &mut Fountain, i: usize, t: f64) {
    let n = f.stops.len();
    if i == 0 || i + 1 >= n {
        return;
    }
    let lo = f.stops[i - 1].pos + 1e-3;
    let hi = f.stops[i + 1].pos - 1e-3;
    if lo <= hi {
        f.stops[i].pos = t.clamp(lo, hi);
    }
}

/// The colours the other way round: each node at `1 - pos`, the
/// midpoints mirrored with them.
pub fn reverse(f: &mut Fountain) {
    let n = f.stops.len();
    let mids: Vec<f64> = f.stops.iter().map(|s| s.mid).collect();
    f.stops.reverse();
    for (k, s) in f.stops.iter_mut().enumerate() {
        s.pos = 1.0 - s.pos;
        // The blend after this node was the one before the old node.
        s.mid = if k + 1 < n {
            1.0 - mids[n - 2 - k]
        } else {
            0.5
        };
    }
}

/// The picked node's colour, transparency, position and midpoint.
fn node_fields(ui: &mut Ui, f: &mut Fountain, node: usize) {
    let n = f.stops.len();
    let Some(s) = f.stops.get_mut(node) else {
        return;
    };
    ui.horizontal(|ui| {
        ui.label(tr("edit_fill.node_color"));
        crate::ui::color_picker::color_button(ui, &mut s.color);
        ui.label(tr("toolbar.node_transparency"));
        let mut tr_pct = f64::from((1.0 - s.opacity) * 100.0).round();
        if ui
            .add(
                crate::ui::field::NumField::new(&mut tr_pct)
                    .range(0.0..=100.0)
                    .speed(1.0)
                    .suffix(" %")
                    .width(54.0),
            )
            .changed()
        {
            s.opacity = (1.0 - tr_pct / 100.0).clamp(0.0, 1.0) as f32;
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("toolbar.node_position"));
        let mut pos = f.stops[node].pos * 100.0;
        if ui
            .add_enabled(
                node > 0 && node + 1 < n,
                crate::ui::field::NumField::new(&mut pos)
                    .range(0.0..=100.0)
                    .speed(1.0)
                    .suffix(" %")
                    .width(54.0),
            )
            .changed()
        {
            move_node(f, node, pos / 100.0);
        }
        ui.label(tr("edit_fill.midpoint"));
        let mut mid = f.stops[node].mid * 100.0;
        if ui
            .add_enabled(
                node + 1 < n,
                crate::ui::field::NumField::new(&mut mid)
                    .range(1.0..=99.0)
                    .speed(1.0)
                    .suffix(" %")
                    .width(54.0),
            )
            .rich_tip(tr("edit_fill.midpoint_tip"))
            .changed()
        {
            f.stops[node].mid = (mid / 100.0).clamp(0.01, 0.99);
        }
    });
}

/// Rotation, centre and edge pad, mirror and inversion. Nodes placed
/// freely turn and move with the rotation and the centre (preview box
/// `b`), so the fields and the preview's nodes stay in step (#939).
fn transform(ui: &mut Ui, f: &mut Fountain, b: GRect) {
    ui.horizontal(|ui| {
        ui.label(tr("edit_fill.rotate"));
        let before = f.angle;
        if ui
            .add(
                crate::ui::field::NumField::new(&mut f.angle)
                    .range(-360.0..=360.0)
                    .speed(1.0)
                    .max_decimals(1)
                    .suffix("°")
                    .width(60.0),
            )
            .changed()
        {
            turn_nodes(f, b, f.angle - before);
        }
        if !matches!(f.kind, FountainKind::Linear | FountainKind::Shaped) {
            ui.label(tr("edit_fill.x"));
            let mut x = f.offset.x * 100.0;
            if ui
                .add(
                    crate::ui::field::NumField::new(&mut x)
                        .range(-100.0..=100.0)
                        .suffix(" %")
                        .width(54.0),
                )
                .changed()
            {
                shift_nodes(f, x / 100.0 - f.offset.x, 0.0);
                f.offset.x = x / 100.0;
            }
            ui.label(tr("edit_fill.y"));
            let mut y = f.offset.y * 100.0;
            if ui
                .add(
                    crate::ui::field::NumField::new(&mut y)
                        .range(-100.0..=100.0)
                        .suffix(" %")
                        .width(54.0),
                )
                .changed()
            {
                shift_nodes(f, 0.0, y / 100.0 - f.offset.y);
                f.offset.y = y / 100.0;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("edit_fill.edge_pad"));
        let mut pad = f.edge_pad * 100.0;
        if ui
            .add(
                crate::ui::field::NumField::new(&mut pad)
                    .range(0.0..=49.0)
                    .suffix(" %")
                    .width(54.0),
            )
            .changed()
        {
            f.edge_pad = pad / 100.0;
        }
        ui.checkbox(&mut f.bilinear, tr("docker.bilinear"))
            .rich_tip(tr("docker.bilinear_tip"));
        ui.checkbox(&mut f.invert, tr("docker.invert_colours"))
            .rich_tip(tr("docker.invert_colours_tip"));
    });
    if f.handles.is_some() {
        // Nodes placed with Interactive Fill: back to the angle and centre.
        if ui
            .button(tr("edit_fill.reset_nodes"))
            .rich_tip(tr("edit_fill.reset_nodes_tip"))
            .clicked()
        {
            f.handles = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dialogs::Dialog;
    use tracedraw_core::ShapeKind;

    fn two() -> Fountain {
        Fountain::two(FountainKind::Linear, Color::BLACK, Color::WHITE, 0.0)
    }

    #[test]
    fn nodes_are_added_moved_and_reversed() {
        let mut f = two();
        assert_eq!(add_node(&mut f, 0.25), Some(1));
        assert_eq!(add_node(&mut f, 0.25), None);
        move_node(&mut f, 1, 0.9);
        assert!((f.stops[1].pos - 0.9).abs() < 1e-9);
        move_node(&mut f, 1, 2.0);
        assert!(f.stops[1].pos < 1.0);
        move_node(&mut f, 0, 0.5);
        assert_eq!(f.stops[0].pos, 0.0);
        f.stops[0].mid = 0.2;
        let first = f.stops[0].color;
        reverse(&mut f);
        assert_eq!(f.stops.last().map(|s| s.color), Some(first));
        assert_eq!(f.stops[0].pos, 0.0);
        // The blend that was first is last now, its midpoint mirrored.
        let k = f.stops.len() - 2;
        assert!((f.stops[k].mid - 0.8).abs() < 1e-9, "{:?}", f.stops);
        reverse(&mut f);
        assert!((f.stops[0].mid - 0.2).abs() < 1e-9);
    }

    /// OK changes only what was changed on every selected object, with
    /// Overprint fill in the same step (#361).
    #[test]
    fn edit_fill_applies_what_was_changed() {
        let mut app = App::headless();
        let r = |app: &mut App, x: f64| {
            app.new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(x, 10.0, x + 20.0, 30.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect")
        };
        let a = r(&mut app, 10.0);
        let b = r(&mut app, 40.0);
        app.select(vec![a, b]);
        app.apply_fill(Fill::Fountain(two()));
        app.open_edit_fill();
        let crate::ui::dialogs::Dialog::EditFill(mut st) =
            std::mem::replace(&mut app.dialog, crate::ui::dialogs::Dialog::None)
        else {
            panic!("the dialog");
        };
        let Fill::Fountain(f) = &mut st.fill else {
            panic!("a gradient");
        };
        f.kind = FountainKind::Radial;
        add_node(f, 0.5);
        st.overprint = Some(true);
        let steps = app.engine.history_labels().0.len();
        app.apply_edit_fill(&st);
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        for id in [a, b] {
            let s = app.doc().find_shape(id).cloned().expect("shape");
            let Fill::Fountain(f) = &s.fill else {
                panic!("a gradient");
            };
            assert_eq!(f.kind, FountainKind::Radial);
            assert_eq!(f.stops.len(), 3);
            assert!(s.overprint_fill);
        }
        // A new type for both.
        app.open_edit_fill();
        let crate::ui::dialogs::Dialog::EditFill(mut st) =
            std::mem::replace(&mut app.dialog, crate::ui::dialogs::Dialog::None)
        else {
            panic!("the dialog");
        };
        st.fill = new_of_kind(&st.fill, 1);
        st.touched = vec![FillField::All];
        app.apply_edit_fill(&st);
        let s = app.doc().find_shape(b).cloned().expect("shape");
        assert!(matches!(s.fill, Fill::Solid(_)));
    }

    fn frame(ctx: &egui::Context, app: &mut App) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
    }

    /// Uniform fill shows the colour's model and a field for each of its
    /// values, as the colour picker does, not just the hex code (#468).
    #[test]
    fn a_uniform_fill_shows_its_model_and_values() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 30.0, 30.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.apply_fill(Fill::Solid(tracedraw_core::Color::cmyk_pct(
            0.0, 40.0, 100.0, 0.0,
        )));
        // Only the dialog may show a colour's values.
        app.show_dockers = false;
        app.open_edit_fill();
        let texts = |app: &mut App| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
            out.textures_delta.clear();
            let mut all = Vec::new();
            fn walk(s: &egui::Shape, all: &mut Vec<String>) {
                match s {
                    egui::Shape::Text(t) => all.push(t.galley.text().to_string()),
                    egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, all)),
                    _ => {}
                }
            }
            out.shapes.iter().for_each(|c| walk(&c.shape, &mut all));
            all
        };
        texts(&mut app);
        let all = texts(&mut app);
        for want in ["CMYK", "C", "M", "Y", "K"] {
            assert!(all.iter().any(|t| t == want), "{want} in {all:?}");
        }
    }

    /// The dialog keeps its place and size, and OK its place, whichever
    /// kind of fill is picked: each kind's settings get the same room
    /// (#654). Every kind's page fits that room.
    #[test]
    fn the_window_keeps_its_size_for_every_kind() {
        let ctx = crate::theme::ui_context();
        App::setup_context(&ctx);
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 30.0, 30.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.apply_fill(Fill::Fountain(two()));
        app.show_dockers = false;
        app.open_edit_fill();
        let run = |app: &mut App| -> (egui::Rect, Option<egui::Pos2>) {
            let mut ok = None;
            for _ in 0..4 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1400.0, 900.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
                out.textures_delta.clear();
                fn walk(s: &egui::Shape, ok: &mut Option<egui::Pos2>) {
                    match s {
                        egui::Shape::Text(t) if t.galley.text() == "OK" => *ok = Some(t.pos),
                        egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, ok)),
                        _ => {}
                    }
                }
                ok = None;
                out.shapes.iter().for_each(|c| walk(&c.shape, &mut ok));
            }
            let id = egui::Id::new(("chrome_dialog", tr("edit_fill.title")));
            let rect = ctx.memory(|m| m.area_rect(id)).expect("the dialog");
            (rect, ok)
        };
        let (rect0, ok0) = run(&mut app);
        let ok0 = ok0.expect("OK shown");
        for i in [0, 1, 2, 3, 4, 5, 2, 0] {
            let Dialog::EditFill(st) = &mut app.dialog else {
                panic!("the dialog stays open");
            };
            st.fill = new_of_kind(&st.fill, i);
            st.touched = vec![FillField::All];
            st.node = 0;
            let (rect, ok) = run(&mut app);
            assert!(
                (rect.min - rect0.min).length() < 0.5
                    && (rect.size() - rect0.size()).length() < 0.5,
                "kind {i}: {rect:?} vs {rect0:?}"
            );
            let ok = ok.expect("OK shown");
            assert!(
                (ok - ok0).length() < 0.5,
                "kind {i}: OK at {ok:?} vs {ok0:?}"
            );
        }
        // Every page fits the room (no scroll bar in English).
        let Dialog::EditFill(st) = std::mem::replace(&mut app.dialog, Dialog::None) else {
            panic!("the dialog");
        };
        let mut st = st;
        for i in 0..6 {
            st.fill = new_of_kind(&st.fill, i);
            st.node = 0;
            let mut size = egui::Vec2::ZERO;
            for _ in 0..2 {
                let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                    crate::ui::chrome::form_style(ui);
                    ui.spacing_mut().item_spacing.y = 6.0;
                    size = ui
                        .allocate_ui(egui::vec2(PAGE.x, 2000.0), |ui| page_contents(ui, &mut st))
                        .response
                        .rect
                        .size();
                });
                out.textures_delta.clear();
            }
            assert!(
                size.x <= PAGE.x + 0.5 && size.y <= PAGE.y + 0.5,
                "kind {i}: {size:?} in {PAGE:?}"
            );
        }
    }

    fn fill_of(app: &App, id: tracedraw_core::ShapeId) -> Fill {
        app.doc().find_shape(id).expect("shape").fill.clone()
    }

    /// The dialog's fill shows on the object while the dialog is open;
    /// closing it without OK takes it back, history and all, and OK
    /// keeps it as one step (#436).
    #[test]
    fn the_fill_shows_live_while_the_dialog_is_open() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 30.0, 30.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.apply_fill(Fill::Fountain(two()));
        let before = fill_of(&app, id);
        let steps = app.engine.history_labels();
        let edit = |app: &mut App| {
            let crate::ui::dialogs::Dialog::EditFill(st) = &mut app.dialog else {
                panic!("the dialog");
            };
            let Fill::Fountain(f) = &mut st.fill else {
                panic!("a gradient");
            };
            f.kind = FountainKind::Radial;
        };
        app.open_edit_fill();
        frame(&ctx, &mut app);
        assert_eq!(fill_of(&app, id), before, "nothing changed yet");
        edit(&mut app);
        frame(&ctx, &mut app);
        let Fill::Fountain(shown) = fill_of(&app, id) else {
            panic!("a gradient");
        };
        assert_eq!(shown.kind, FountainKind::Radial, "shown at once");
        // Esc (the keyboard closes any dialog): taken back.
        app.dialog = crate::ui::dialogs::Dialog::None;
        frame(&ctx, &mut app);
        assert_eq!(fill_of(&app, id), before);
        assert_eq!(app.engine.history_labels(), steps);
        // OK: one step.
        app.open_edit_fill();
        frame(&ctx, &mut app);
        edit(&mut app);
        frame(&ctx, &mut app);
        let crate::ui::dialogs::Dialog::EditFill(st) = &app.dialog else {
            panic!("the dialog");
        };
        let st = st.as_ref().clone();
        app.end_edit_fill_preview();
        app.apply_edit_fill(&st);
        assert!(!app.fill_preview);
        assert_eq!(app.engine.history_labels().0.len(), steps.0.len() + 1);
        let Fill::Fountain(kept) = fill_of(&app, id) else {
            panic!("a gradient");
        };
        assert_eq!(kept.kind, FountainKind::Radial);
    }

    fn near(a: GPoint, b: GPoint) -> bool {
        (a - b).hypot() < 1e-6
    }

    /// The preview's box has the object's proportions and maps the page
    /// (y up) to the screen and back (#939).
    #[test]
    fn the_preview_box_has_the_objects_proportions() {
        let b = preview_bounds(Some(GRect::new(10.0, 10.0, 110.0, 60.0)));
        assert!((b.width() - 50.0).abs() < 1e-9 && (b.height() - 25.0).abs() < 1e-9);
        assert_eq!(
            preview_bounds(None).size(),
            GRect::new(0.0, 0.0, 50.0, 50.0).size()
        );
        let flat = preview_bounds(Some(GRect::new(0.0, 0.0, 10.0, 0.0)));
        assert!((flat.width() - flat.height()).abs() < 1e-9, "{flat:?}");
        let thin = preview_bounds(Some(GRect::new(0.0, 0.0, 100.0, 1.0)));
        assert!((thin.height() - 50.0 / 8.0).abs() < 1e-9, "{thin:?}");
        let map = PreviewMap::fit(
            Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(172.0, 152.0)),
            b,
        );
        assert!((map.screen.width() / map.screen.height() - 2.0).abs() < 1e-3);
        let top_left = map.to_screen(GPoint::new(b.x0, b.y1));
        assert!((top_left - map.screen.min).length() < 1e-3);
        let p = GPoint::new(12.5, 20.0);
        assert!((map.to_page(map.to_screen(p)) - p).hypot() < 1e-3);
    }

    /// The centre of an elliptical fill dragged in the preview is its
    /// offset; the end of a linear one turns it, Ctrl in 15 degree steps,
    /// and places its nodes as on the canvas (#939).
    #[test]
    fn preview_nodes_move_and_turn_the_gradient() {
        let b = GRect::new(0.0, 0.0, 50.0, 50.0);
        let mut f = Fountain::two(FountainKind::Radial, Color::BLACK, Color::WHITE, 0.0);
        drag_preview_node(
            &mut f,
            b,
            FountainHandle::Start,
            GPoint::new(37.5, 12.5),
            false,
        );
        assert!(near(f.offset, GPoint::new(0.5, -0.5)), "{:?}", f.offset);
        assert!(f.handles.is_none(), "an offset still places it");
        drag_preview_node(
            &mut f,
            b,
            FountainHandle::Start,
            GPoint::new(500.0, 25.0),
            false,
        );
        assert!(near(f.offset, GPoint::new(1.0, 0.0)), "kept in the box");

        let mut f = two();
        let start = fountain_handles(&f, b)[0].1;
        assert!(near(start, GPoint::new(0.0, 25.0)));
        drag_preview_node(
            &mut f,
            b,
            FountainHandle::End,
            GPoint::new(25.0, 50.0),
            false,
        );
        assert!((f.angle - 45.0).abs() < 1e-9, "{}", f.angle);
        let h = f.handles.expect("nodes placed");
        assert!(near(
            h.end,
            Fountain::page_to_rel(b, GPoint::new(25.0, 50.0))
        ));
        // Turned in steps, the length kept.
        drag_preview_node(
            &mut f,
            b,
            FountainHandle::End,
            GPoint::new(30.0, 50.0),
            true,
        );
        assert_eq!(f.angle, 45.0);
        let e = fountain_handles(&f, b)[1].1;
        let len = (GPoint::new(30.0, 50.0) - start).hypot();
        assert!(((e - start).hypot() - len).abs() < 1e-9);
        // The start of a linear fill moves it.
        drag_preview_node(
            &mut f,
            b,
            FountainHandle::Start,
            GPoint::new(5.0, 30.0),
            false,
        );
        assert!(near(fountain_handles(&f, b)[0].1, GPoint::new(5.0, 30.0)));
        // A shaped fill has no nodes; nonsense points change nothing.
        let mut sh = Fountain::two(FountainKind::Shaped, Color::BLACK, Color::WHITE, 0.0);
        let was = sh.clone();
        drag_preview_node(
            &mut sh,
            b,
            FountainHandle::End,
            GPoint::new(1.0, 1.0),
            false,
        );
        assert_eq!(sh, was);
        let was = f.clone();
        drag_preview_node(
            &mut f,
            b,
            FountainHandle::End,
            GPoint::new(f64::NAN, 1.0),
            false,
        );
        assert_eq!(f, was);
    }

    /// With nodes placed, the rotation and centre fields turn and move
    /// them, so the preview and the fields agree (#939).
    #[test]
    fn the_fields_turn_and_move_placed_nodes() {
        let b = GRect::new(0.0, 0.0, 50.0, 25.0);
        let mut f = two();
        drag_preview_node(
            &mut f,
            b,
            FountainHandle::End,
            GPoint::new(40.0, 12.5),
            false,
        );
        let before = f.handles.expect("placed");
        let s = Fountain::rel_to_page(b, before.start);
        turn_nodes(&mut f, b, 90.0);
        let h = f.handles.expect("placed");
        assert!(near(h.start, before.start));
        let e = Fountain::rel_to_page(b, h.end) - s;
        assert!(e.x.abs() < 1e-9 && (e.y - 40.0).abs() < 1e-9, "{e:?}");
        // What the fill now shows has the angle the field says.
        let mut g = f.clone();
        g.set_handles(b, h);
        assert!((g.angle - 90.0).abs() < 1e-9, "{}", g.angle);
        shift_nodes(&mut f, 0.25, -0.5);
        let m = f.handles.expect("placed");
        assert!(near(
            m.start,
            GPoint::new(h.start.x + 0.25, h.start.y - 0.5)
        ));
        assert!(near(m.end, GPoint::new(h.end.x + 0.25, h.end.y - 0.5)));
        // Without placed nodes the fields alone place the fill.
        let mut plain = two();
        turn_nodes(&mut plain, b, 30.0);
        shift_nodes(&mut plain, 0.5, 0.5);
        assert_eq!(plain, two());
    }

    /// A drag on the preview's end node in the open dialog turns the
    /// gradient, and OK puts the placed nodes on the object; Reset nodes
    /// takes them off it again (#939).
    #[test]
    fn a_drag_in_the_preview_turns_the_objects_gradient() {
        let ctx = crate::theme::ui_context();
        App::setup_context(&ctx);
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 50.0, 30.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.apply_fill(Fill::Fountain(two()));
        app.show_dockers = false;
        app.open_edit_fill();
        let run = |app: &mut App, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
            out.textures_delta.clear();
        };
        for _ in 0..3 {
            run(&mut app, Vec::new());
        }
        let Dialog::EditFill(st) = &app.dialog else {
            panic!("the dialog");
        };
        let b = st.bounds;
        assert!((b.width() / b.height() - 2.0).abs() < 1e-9, "{b:?}");
        let screen: Rect = ctx
            .data(|d| d.get_temp(egui::Id::new("edit_fill_preview_box")))
            .expect("the preview drawn");
        let map = PreviewMap { screen, page: b };
        let Fill::Fountain(f) = &st.fill else {
            panic!("a gradient");
        };
        let from = map.to_screen(fountain_handles(f, b)[1].1);
        // Up to the top right corner of the box.
        let to = map.to_screen(GPoint::new(b.x1, b.y1));
        let press = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(&mut app, vec![egui::Event::PointerMoved(from)]);
        run(&mut app, vec![press(from, true)]);
        for i in 1..=5 {
            let p = from + (to - from) * (i as f32 / 5.0);
            run(&mut app, vec![egui::Event::PointerMoved(p)]);
        }
        run(&mut app, vec![press(to, false)]);
        run(&mut app, Vec::new());
        let Dialog::EditFill(st) = &app.dialog else {
            panic!("the dialog stays open");
        };
        let st = st.as_ref().clone();
        assert_eq!(st.drag, None, "let go");
        let Fill::Fountain(f) = &st.fill else {
            panic!("a gradient");
        };
        let h = f.handles.expect("the end moved");
        assert!(
            (h.end.x - 1.0).abs() < 0.05 && (h.end.y - 1.0).abs() < 0.05,
            "{h:?}"
        );
        assert!(f.angle > 5.0, "turned up: {}", f.angle);
        // Shown on the object meanwhile, kept with OK.
        let Fill::Fountain(shown) = fill_of(&app, id) else {
            panic!("a gradient");
        };
        assert_eq!(shown.handles, f.handles);
        app.end_edit_fill_preview();
        app.apply_edit_fill(&st);
        let Fill::Fountain(kept) = fill_of(&app, id) else {
            panic!("a gradient");
        };
        assert_eq!(kept.handles, f.handles);
        // Reset nodes: back to the angle alone.
        app.open_edit_fill();
        let Dialog::EditFill(mut st) = std::mem::replace(&mut app.dialog, Dialog::None) else {
            panic!("the dialog");
        };
        let Fill::Fountain(f) = &mut st.fill else {
            panic!("a gradient");
        };
        f.handles = None;
        app.apply_edit_fill(&st);
        let Fill::Fountain(reset) = fill_of(&app, id) else {
            panic!("a gradient");
        };
        assert_eq!(reset.handles, None);
        // A drag off the nodes of an elliptical fill moves its centre.
        app.select(vec![id]);
        app.apply_fill(Fill::Fountain(Fountain::two(
            FountainKind::Radial,
            Color::BLACK,
            Color::WHITE,
            0.0,
        )));
        app.open_edit_fill();
        for _ in 0..3 {
            run(&mut app, Vec::new());
        }
        let from = screen.center() + egui::vec2(20.0, 10.0);
        let to = from + egui::vec2(screen.width() / 4.0, 0.0);
        run(&mut app, vec![egui::Event::PointerMoved(from)]);
        run(&mut app, vec![press(from, true)]);
        for i in 1..=5 {
            let p = from + (to - from) * (i as f32 / 5.0);
            run(&mut app, vec![egui::Event::PointerMoved(p)]);
        }
        run(&mut app, vec![press(to, false)]);
        run(&mut app, Vec::new());
        let Dialog::EditFill(st) = &app.dialog else {
            panic!("the dialog stays open");
        };
        let Fill::Fountain(f) = &st.fill else {
            panic!("a gradient");
        };
        assert!((f.offset.x - 0.5).abs() < 0.05, "{:?}", f.offset);
        assert!(f.offset.y.abs() < 0.05, "{:?}", f.offset);
        assert_eq!(f.handles, None);
    }
}
