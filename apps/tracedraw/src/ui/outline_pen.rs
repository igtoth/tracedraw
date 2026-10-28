//! The Outline Pen dialog (F12, the Outline Pen tool, #357): every part of
//! the selected objects' outlines in one window, laid out in groups:
//! General (colour, width and its unit, style, mitre limit, dashes,
//! corners, line caps, position), Arrowheads, Calligraphy (stretch, angle,
//! a picture of the nib), Behind fill and Scale with object, Variable
//! Outline (the width nodes along the outline) and Overprint outline. OK
//! applies what was changed, in one undo step; with nothing selected it
//! sets the outline new objects get. See docs/behavior/outline-pen.md.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Color32, Pos2, Rect, Sense, Stroke as EStroke, Ui, Vec2};
use tracedraw_core::patch::{StrokeField, StrokeMix};
use tracedraw_core::{LineCap, LineJoin, OutlinePosition, Stroke, WidthNode};

/// What the dialog keeps while it is open.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlinePenState {
    /// The outline as it was (the first object's, or the default).
    pub old: Option<Stroke>,
    /// The outline being edited.
    pub stroke: Stroke,
    /// The objects have an outline (unticked: OK takes it away).
    pub has: bool,
    /// How the selection's outlines differ.
    pub mix: StrokeMix,
    /// Fields set in the dialog (they apply even when they equal the
    /// value shown).
    pub touched: Vec<StrokeField>,
    /// Overprint outline as shown (`None`: the objects differ) and as
    /// it was.
    pub overprint: Option<bool>,
    pub overprint_old: Option<bool>,
    /// The width node picked in the Variable Outline group.
    pub node: Option<usize>,
    /// Nothing is selected: the dialog edits the default outline.
    pub defaults: bool,
    /// What the drawing shows of the dialog (outline, fields set,
    /// overprint), so the preview is made again only when it changes.
    pub shown: Option<(Option<Stroke>, Vec<StrokeField>, Option<bool>)>,
}

/// The widths the width list offers in unit `u`, as (millimetres, label):
/// the same presets the property bar's width list offers, so both lists
/// match in every unit.
pub(crate) fn width_choices(u: crate::app::Units) -> Vec<(f64, String)> {
    crate::ui::propbar::outline_width_presets(u)
        .into_iter()
        .map(|mm| (mm, crate::ui::propbar::outline_width_number(mm, u)))
        .collect()
}

/// The dash styles the style list offers before the custom ones.
fn preset_dashes() -> [Vec<f64>; 4] {
    [
        vec![],
        vec![4.0, 2.0],
        vec![1.0, 1.0],
        vec![6.0, 2.0, 1.0, 2.0],
    ]
}

impl App {
    /// Open the Outline Pen dialog on the selection's outline (or the
    /// default outline when nothing is selected).
    pub fn open_outline_pen(&mut self) {
        let targets = self.paint_targets();
        let defaults = targets.is_empty();
        let old = match targets.first() {
            Some(s) => s.stroke.clone(),
            None => self.default_stroke(),
        };
        let mix = if defaults {
            StrokeMix::default()
        } else {
            self.selection_stroke_mix()
        };
        let overprint_old = if defaults {
            Some(false)
        } else {
            let on = targets.iter().filter(|s| s.overprint_outline).count();
            if on == 0 {
                Some(false)
            } else if on == targets.len() {
                Some(true)
            } else {
                None
            }
        };
        let lib = crate::ui::line_style::OutlineLib::of(self);
        let stroke = old.clone().unwrap_or(Stroke {
            color: lib.black,
            ..Stroke::default()
        });
        self.dialog = crate::ui::dialogs::Dialog::OutlinePen(Box::new(OutlinePenState {
            has: old.is_some(),
            old,
            stroke,
            mix,
            touched: Vec::new(),
            overprint: overprint_old,
            overprint_old,
            node: None,
            defaults,
            shown: None,
        }));
    }

    /// Show what the dialog holds on the selection while it is open, as
    /// the Edit Fill dialog does: the change is taken back and made anew
    /// only when a value in the dialog changes, OK keeps it as one step
    /// and Cancel, Esc or the close button take it back (#741).
    pub fn preview_outline_pen(&mut self, st: &mut OutlinePenState) {
        if st.defaults {
            return;
        }
        let new = st.has.then(|| st.stroke.clone());
        let unchanged = new == st.old && st.touched.is_empty() && st.overprint == st.overprint_old;
        let now = (new, st.touched.clone(), st.overprint);
        // Shown already, unless something else took the preview back.
        if st.shown.as_ref() == Some(&now) && self.outline_preview != unchanged {
            return;
        }
        self.end_outline_pen_preview();
        if !unchanged {
            self.engine.begin_preview();
            self.outline_preview = true;
            self.apply_outline_pen(st);
        }
        st.shown = Some(now);
    }

    /// Take back the outline the Outline Pen dialog shows on the
    /// selection.
    pub fn end_outline_pen_preview(&mut self) {
        if std::mem::take(&mut self.outline_preview) {
            self.engine.withdraw_preview();
        }
    }

    /// OK in the Outline Pen dialog.
    pub fn apply_outline_pen(&mut self, st: &OutlinePenState) {
        let new = st.has.then(|| st.stroke.clone());
        if st.defaults {
            if new != st.old {
                self.request_default_change(crate::defaults::DefaultChange::Outline(new));
            }
            return;
        }
        let before = self.engine.last_step_id();
        self.patch_selection_stroke(&st.old, &new, &st.touched);
        if let Some(on) = st.overprint.filter(|_| st.overprint != st.overprint_old) {
            let ids: Vec<_> = self.paint_targets().iter().map(|s| s.id).collect();
            let cmd = tracedraw_core::Command::SetOverprint {
                shapes: ids,
                fill: None,
                outline: Some(on),
            };
            // One step with the outline change when there was one.
            if self.engine.last_step_id() != before {
                let _ = self.engine.amend(&cmd);
            } else {
                self.run(cmd);
            }
        }
    }
}

/// The dialog's window. True when it is to close.
pub fn dialog(app: &mut App, ctx: &egui::Context, st: &mut OutlinePenState, close: &mut bool) {
    let lib = crate::ui::line_style::OutlineLib::of(app);
    let mut ok = false;
    let mut help = false;
    crate::ui::dialogs::window(ctx, tr("outline_pen.title")).show(ctx, |ui| {
        ui.set_min_width(640.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(330.0);
                group(ui, &tr("outline_pen.general"), |ui| general(ui, st, &lib));
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.set_width(290.0);
                group(ui, &tr("outline_pen.arrowheads"), |ui| {
                    arrowheads(ui, st, &lib)
                });
                group(ui, &tr("outline_pen.calligraphy"), |ui| calligraphy(ui, st));
            });
        });
        group(ui, &tr("outline_pen.variable_outline"), |ui| {
            variable(ui, st, &lib)
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            help = ui
                .add_sized([27.0, 27.0], egui::Button::new("?"))
                .rich_tip(tr("dialog.help"))
                .clicked();
            if !st.defaults {
                let mut on = st.overprint.unwrap_or(false);
                if ui
                    .add(
                        egui::Checkbox::new(&mut on, tr("outline_pen.overprint"))
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
        app.end_outline_pen_preview();
        app.apply_outline_pen(st);
    } else if *close {
        app.end_outline_pen_preview();
    } else {
        app.preview_outline_pen(st);
    }
}

/// A titled group with a light frame around it.
fn group(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    crate::ui::chrome::section(ui, title);
    egui::Frame::new()
        .stroke(EStroke::new(1.0, Tokens::border()))
        .inner_margin(egui::Margin::same(8))
        .corner_radius(3.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui)
        });
}

/// Mark `field` as set and the outline as present.
fn touch(st: &mut OutlinePenState, field: StrokeField) {
    if !st.touched.contains(&field) {
        st.touched.push(field);
    }
    st.has = true;
}

fn row_label(ui: &mut Ui, key: &str) {
    ui.add_sized(
        [86.0, 20.0],
        egui::Label::new(egui::RichText::new(tr(key)).color(Tokens::text())),
    );
}

fn general(ui: &mut Ui, st: &mut OutlinePenState, lib: &crate::ui::line_style::OutlineLib) {
    let mixed = |st: &OutlinePenState, f: StrokeField| st.mix.has(f) && !st.touched.contains(&f);
    // Colour.
    ui.horizontal(|ui| {
        row_label(ui, "outline_pen.color");
        let mut c = st.stroke.color;
        if crate::ui::color_picker::color_button(ui, &mut c) {
            st.stroke.color = c;
            st.stroke.fill = None;
            touch(st, StrokeField::Paint);
        }
    });
    // Width: a list with None, hairline and common widths, a box for any
    // width, and the unit.
    ui.horizontal(|ui| {
        row_label(ui, "outline_pen.width");
        let u = lib.units;
        let shown = if !st.has {
            tr("outline_pen.none")
        } else if mixed(st, StrokeField::Width) {
            String::new()
        } else if st.stroke.width <= Stroke::HAIRLINE + 1e-9 {
            tr("docker.hairline")
        } else {
            crate::ui::propbar::outline_width_number(st.stroke.width, u)
        };
        egui::ComboBox::from_id_salt("outline_pen_width")
            .selected_text(shown)
            .width(110.0)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(!st.has, tr("outline_pen.none"))
                    .clicked()
                {
                    st.has = false;
                }
                if ui.selectable_label(false, tr("docker.hairline")).clicked() {
                    st.stroke.width = Stroke::HAIRLINE;
                    touch(st, StrokeField::Width);
                }
                for (mm, label) in width_choices(u) {
                    if ui.selectable_label(false, label).clicked() {
                        st.stroke.width = mm;
                        touch(st, StrokeField::Width);
                    }
                }
            });
        let mut v = u.from_mm(st.stroke.width);
        if ui
            .add(
                crate::ui::field::NumField::new(&mut v)
                    .range(u.from_mm(0.001)..=u.from_mm(1000.0))
                    .unit_mm(u.mm())
                    .max_decimals(4)
                    .width(60.0)
                    .mixed(mixed(st, StrokeField::Width) || !st.has),
            )
            .changed()
        {
            st.stroke.width = u.to_mm(v).clamp(0.001, 1000.0);
            touch(st, StrokeField::Width);
        }
        if let Some(nu) = crate::ui::propbar::outline_unit_combo(ui, "outline_pen_units", u) {
            crate::ui::line_style::request_units(ui, nu);
        }
    });
    // Style: the dash patterns drawn, then Edit Line Style.
    ui.horizontal(|ui| {
        row_label(ui, "outline_pen.style");
        let dashes: Vec<Vec<f64>> = preset_dashes()
            .into_iter()
            .chain(lib.dashes.iter().cloned())
            .collect();
        let resp = egui::ComboBox::from_id_salt("outline_pen_style")
            .selected_text("")
            .width(170.0)
            .show_ui(ui, |ui| {
                for d in &dashes {
                    let r = crate::ui::line_style::preview(ui, d, 150.0);
                    if st.stroke.dash == *d {
                        ui.painter().rect_stroke(
                            r.rect.expand(2.0),
                            0.0,
                            EStroke::new(1.0, Tokens::ACCENT),
                            egui::StrokeKind::Outside,
                        );
                    }
                    if r.clicked() {
                        st.stroke.dash = d.clone();
                        touch(st, StrokeField::Dash);
                    }
                }
            });
        // The style in use, drawn over the closed list.
        if !mixed(st, StrokeField::Dash) {
            let r = resp.response.rect.shrink2(Vec2::new(8.0, 0.0));
            let r = Rect::from_min_max(r.min, Pos2::new(r.max.x - 18.0, r.max.y));
            dash_line(ui.painter(), r, &st.stroke.dash);
        }
        if ui
            .button("\u{2026}")
            .rich_tip(tr("docker.edit_line_style"))
            .clicked()
        {
            crate::ui::line_style::request(ui);
        }
    });
    // Mitre limit.
    ui.horizontal(|ui| {
        row_label(ui, "docker.miter_limit");
        let limit_mixed = mixed(st, StrokeField::MiterLimit);
        if ui
            .add_enabled(
                st.stroke.join == LineJoin::Miter,
                crate::ui::field::NumField::new(&mut st.stroke.miter_limit)
                    .range(Stroke::MITER_LIMIT_RANGE)
                    .speed(0.5)
                    .max_decimals(1)
                    .suffix("°")
                    .width(70.0)
                    .mixed(limit_mixed),
            )
            .rich_tip(tr("docker.miter_limit_tip"))
            .changed()
        {
            touch(st, StrokeField::MiterLimit);
        }
    });
    // Dashes: as they come, or fitted to the corners and ends.
    ui.horizontal(|ui| {
        row_label(ui, "outline_pen.dashes");
        let shown = !mixed(st, StrokeField::AlignDashes);
        for (align, pic, key) in [
            (false, Pic::DashDefault, "outline_pen.dashes_default"),
            (true, Pic::DashAlign, "docker.align_dashes"),
        ] {
            let on = shown && st.stroke.align_dashes == align;
            if pic_button(ui, pic, on, !st.stroke.dash.is_empty(), key) {
                st.stroke.align_dashes = align;
                touch(st, StrokeField::AlignDashes);
            }
        }
    });
    ui.horizontal(|ui| {
        row_label(ui, "docker.corners");
        let shown = !mixed(st, StrokeField::Join);
        for (j, pic, key) in [
            (LineJoin::Miter, Pic::JoinMiter, "docker.join_miter"),
            (LineJoin::Round, Pic::JoinRound, "docker.join_round"),
            (LineJoin::Bevel, Pic::JoinBevel, "docker.join_bevel"),
        ] {
            if pic_button(ui, pic, shown && st.stroke.join == j, true, key) {
                st.stroke.join = j;
                touch(st, StrokeField::Join);
            }
        }
    });
    ui.horizontal(|ui| {
        row_label(ui, "outline_pen.line_caps");
        let shown = !mixed(st, StrokeField::Cap);
        for (c, pic, key) in [
            (LineCap::Butt, Pic::CapButt, "docker.cap_butt"),
            (LineCap::Round, Pic::CapRound, "docker.cap_round"),
            (LineCap::Square, Pic::CapSquare, "docker.cap_square"),
        ] {
            if pic_button(ui, pic, shown && st.stroke.cap == c, true, key) {
                st.stroke.cap = c;
                touch(st, StrokeField::Cap);
            }
        }
    });
    ui.horizontal(|ui| {
        row_label(ui, "outline_pen.position");
        let shown = !mixed(st, StrokeField::Position);
        for (p, pic, key) in [
            (
                OutlinePosition::Outside,
                Pic::Outside,
                "docker.position_outside",
            ),
            (
                OutlinePosition::Center,
                Pic::Center,
                "docker.position_center",
            ),
            (
                OutlinePosition::Inside,
                Pic::Inside,
                "docker.position_inside",
            ),
        ] {
            if pic_button(ui, pic, shown && st.stroke.position == p, true, key) {
                st.stroke.position = p;
                touch(st, StrokeField::Position);
            }
        }
    });
}

fn arrowheads(ui: &mut Ui, st: &mut OutlinePenState, lib: &crate::ui::line_style::OutlineLib) {
    for (field, id, key) in [
        (
            StrokeField::StartArrow,
            "outline_pen_start",
            "docker.arrow_start",
        ),
        (StrokeField::EndArrow, "outline_pen_end", "docker.arrow_end"),
    ] {
        let mixed = st.mix.has(field) && !st.touched.contains(&field);
        let a = if field == StrokeField::StartArrow {
            &mut st.stroke.start_arrow
        } else {
            &mut st.stroke.end_arrow
        };
        if crate::ui::dockers::arrow_combo(ui, id, &tr(key), a, &lib.arrows, mixed) {
            touch(st, field);
        }
    }
}

fn calligraphy(ui: &mut Ui, st: &mut OutlinePenState) {
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                row_label(ui, "outline_pen.stretch");
                let mut pct = st.stroke.stretch * 100.0;
                if ui
                    .add(
                        crate::ui::field::NumField::new(&mut pct)
                            .range(10.0..=100.0)
                            .speed(1.0)
                            .max_decimals(0)
                            .suffix(" %")
                            .width(60.0)
                            .mixed(
                                st.mix.has(StrokeField::Stretch)
                                    && !st.touched.contains(&StrokeField::Stretch),
                            ),
                    )
                    .changed()
                {
                    st.stroke.stretch = (pct / 100.0).clamp(0.1, 1.0);
                    touch(st, StrokeField::Stretch);
                }
            });
            ui.horizontal(|ui| {
                row_label(ui, "outline_pen.angle");
                if ui
                    .add(
                        crate::ui::field::NumField::new(&mut st.stroke.nib_angle)
                            .range(-90.0..=90.0)
                            .speed(1.0)
                            .max_decimals(1)
                            .suffix("°")
                            .width(60.0)
                            .mixed(
                                st.mix.has(StrokeField::NibAngle)
                                    && !st.touched.contains(&StrokeField::NibAngle),
                            ),
                    )
                    .changed()
                {
                    touch(st, StrokeField::NibAngle);
                }
            });
            if ui.button(tr("outline_pen.default")).clicked() {
                st.stroke.stretch = 1.0;
                st.stroke.nib_angle = 0.0;
                touch(st, StrokeField::Stretch);
                touch(st, StrokeField::NibAngle);
            }
        });
        // The nib: a square stretched and turned as set; a drag on it
        // turns and stretches it.
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(64.0), Sense::drag());
        let p = ui.painter();
        p.rect_filled(rect, 2.0, Color32::WHITE);
        p.rect_stroke(
            rect,
            2.0,
            EStroke::new(1.0, Tokens::border()),
            egui::StrokeKind::Inside,
        );
        if let Some(at) = resp.interact_pointer_pos().filter(|_| resp.dragged()) {
            let d = at - rect.center();
            if d.length() > 2.0 {
                let a = (-d.y).atan2(d.x).to_degrees() as f64;
                let a = ((a + 90.0).rem_euclid(180.0)) - 90.0;
                st.stroke.nib_angle = a.round();
                st.stroke.stretch = f64::from(d.length() / 28.0).clamp(0.1, 1.0);
                touch(st, StrokeField::NibAngle);
                touch(st, StrokeField::Stretch);
            }
        }
        let corners = nib_corners(rect.center(), 24.0, st.stroke.stretch, st.stroke.nib_angle);
        p.add(egui::Shape::convex_polygon(
            corners.to_vec(),
            Color32::BLACK,
            EStroke::NONE,
        ));
    });
    for (field, key) in [
        (StrokeField::BehindFill, "docker.behind_fill"),
        (StrokeField::ScaleWithObject, "docker.scale_with_object"),
    ] {
        let mixed = st.mix.has(field) && !st.touched.contains(&field);
        let v = if field == StrokeField::BehindFill {
            &mut st.stroke.behind_fill
        } else {
            &mut st.stroke.scale_with_object
        };
        if ui
            .add(egui::Checkbox::new(v, tr(key)).indeterminate(mixed))
            .changed()
        {
            touch(st, field);
        }
    }
}

/// The corners of the nib picture: a square of half side `r` at `c`,
/// its height times `stretch`, turned by `angle` degrees.
pub fn nib_corners(c: Pos2, r: f32, stretch: f64, angle: f64) -> [Pos2; 4] {
    let (s, co) = (angle.to_radians() as f32).sin_cos();
    let h = r * stretch.clamp(0.1, 1.0) as f32;
    [(-r, -h), (r, -h), (r, h), (-r, h)].map(|(x, y)| {
        // Screen y runs down: a positive angle turns counter-clockwise.
        Pos2::new(c.x + x * co + y * s, c.y - x * s + y * co)
    })
}

/// The Variable Outline bar's id (tests find the bar by it).
pub(crate) const VO_BAR: &str = "outline_pen_vo_bar";

/// The node of `nodes` nearest to `x` on the bar (`x_of` placing a node),
/// within 6 points.
fn nearest_node(nodes: &[WidthNode], x: f32, x_of: impl Fn(f64) -> f32) -> Option<usize> {
    nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (i, (x_of(n.at) - x).abs()))
        .filter(|(_, d)| *d <= 6.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// Add a node at `t` with the width the outline has there; its index.
fn add_node(st: &mut OutlinePenState, t: f64) -> usize {
    let (l, r) = tracedraw_core::style::width_at(st.stroke.width, &st.stroke.widths, t);
    st.stroke.widths.push(WidthNode {
        at: t,
        left: l,
        right: r,
    });
    let last = st.stroke.widths.len() - 1;
    let (nodes, at) = crate::variable_outline::sorted(std::mem::take(&mut st.stroke.widths), last);
    st.stroke.widths = nodes;
    touch(st, StrokeField::Widths);
    at
}

/// Slide node `i` to `t`, kept between its neighbours and within the ends,
/// as on the canvas (#479); true when it moved.
fn slide_node(nodes: &mut [WidthNode], i: usize, t: f64) -> bool {
    let lo = i
        .checked_sub(1)
        .and_then(|k| nodes.get(k))
        .map_or(0.0, |m| m.at + 1e-4);
    let hi = nodes.get(i + 1).map_or(1.0, |m| m.at - 1e-4);
    let Some(m) = nodes.get_mut(i) else {
        return false;
    };
    let t = t.clamp(lo.min(hi), hi.max(lo));
    if (m.at - t).abs() < 1e-9 {
        return false;
    }
    m.at = t;
    true
}

/// The Variable Outline group: the width nodes as dots along a bar (a
/// click on the bar adds one, on a dot picks it, a drag slides it between
/// its neighbours, #652), the picked node's position and its width on
/// each side and in all.
fn variable(ui: &mut Ui, st: &mut OutlinePenState, lib: &crate::ui::line_style::OutlineLib) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(360.0, 34.0), Sense::hover());
    // Clicks and drags: a drag on the bar slides a node, not the dialog.
    let id = egui::Id::new(VO_BAR);
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let x_of = |t: f64| rect.left() + rect.width() * t.clamp(0.0, 1.0) as f32;
    let t_of = |x: f32| f64::from((x - rect.left()) / rect.width()).clamp(0.0, 1.0);
    if resp.clicked() {
        if let Some(at) = resp.interact_pointer_pos() {
            st.node = Some(match nearest_node(&st.stroke.widths, at.x, x_of) {
                Some(i) => i,
                None => add_node(st, t_of(at.x)),
            });
        }
    }
    let key = id.with("drag");
    if resp.drag_started() {
        // Where the button went down (the pointer moved on since).
        if let Some(p) = ui.input(|i| i.pointer.press_origin()) {
            let i = nearest_node(&st.stroke.widths, p.x, x_of)
                .unwrap_or_else(|| add_node(st, t_of(p.x)));
            st.node = Some(i);
            let grab = p.x - x_of(st.stroke.widths.get(i).map_or(0.0, |n| n.at));
            ui.data_mut(|d| d.insert_temp(key, (i, grab)));
        }
    }
    if resp.dragged() {
        let held = ui.data(|d| d.get_temp::<(usize, f32)>(key));
        if let (Some((i, grab)), Some(p)) = (held, resp.interact_pointer_pos()) {
            if slide_node(&mut st.stroke.widths, i, t_of(p.x - grab)) {
                touch(st, StrokeField::Widths);
            }
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if resp
        .hover_pos()
        .is_some_and(|h| nearest_node(&st.stroke.widths, h.x, x_of).is_some())
    {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    if resp.drag_stopped() {
        ui.data_mut(|d| d.remove::<(usize, f32)>(key));
    }
    // Drawn after the input, so the dots are where this frame left them.
    let p = ui.painter();
    let y = rect.center().y;
    p.line_segment(
        [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
        EStroke::new(2.0, Tokens::text_dim()),
    );
    for k in 0..=4 {
        let x = rect.left() + rect.width() * k as f32 / 4.0;
        p.line_segment(
            [Pos2::new(x, y - 9.0), Pos2::new(x, y + 9.0)],
            EStroke::new(1.0, Tokens::text_dim()),
        );
    }
    for (i, n) in st.stroke.widths.iter().enumerate() {
        let c = Pos2::new(x_of(n.at), y);
        let picked = st.node == Some(i);
        p.circle_filled(
            c,
            5.0,
            if picked {
                Tokens::ACCENT
            } else {
                Color32::WHITE
            },
        );
        p.circle_stroke(c, 5.0, EStroke::new(1.2, Tokens::ACCENT));
    }
    let u = lib.units;
    let node = st.node.filter(|i| *i < st.stroke.widths.len());
    ui.horizontal(|ui| {
        let n = node.map(|i| st.stroke.widths[i]);
        let mut pos = n.map_or(0.0, |n| n.at * 100.0);
        let mut side1 = u.from_mm(n.map_or(0.0, |n| n.left));
        let mut side2 = u.from_mm(n.map_or(0.0, |n| n.right));
        ui.label(tr("outline_pen.node_position"));
        let a = ui.add_enabled(
            n.is_some(),
            crate::ui::field::NumField::new(&mut pos)
                .range(0.0..=100.0)
                .max_decimals(1)
                .suffix(" %")
                .width(56.0),
        );
        ui.label(tr("outline_pen.side1"));
        let b = ui.add_enabled(
            n.is_some(),
            crate::ui::field::NumField::new(&mut side1)
                .range(0.0..=u.from_mm(500.0))
                .max_decimals(3)
                .width(56.0),
        );
        ui.label(tr("outline_pen.side2"));
        let c = ui.add_enabled(
            n.is_some(),
            crate::ui::field::NumField::new(&mut side2)
                .range(0.0..=u.from_mm(500.0))
                .max_decimals(3)
                .width(56.0),
        );
        let mut total = side1 + side2;
        ui.label(tr("outline_pen.total"));
        let d = ui.add_enabled(
            n.is_some(),
            crate::ui::field::NumField::new(&mut total)
                .range(0.0..=u.from_mm(1000.0))
                .max_decimals(3)
                .width(56.0),
        );
        ui.label(u.short());
        if let Some(i) = node {
            let m = &mut st.stroke.widths[i];
            let mut changed = false;
            if a.changed() {
                m.at = pos / 100.0;
                changed = true;
            }
            if b.changed() {
                m.left = u.to_mm(side1);
                changed = true;
            }
            if c.changed() {
                m.right = u.to_mm(side2);
                changed = true;
            }
            if d.changed() {
                // The total shared as the two sides were.
                let old = m.left + m.right;
                let new = u.to_mm(total);
                if old > 1e-9 {
                    m.left *= new / old;
                    m.right *= new / old;
                } else {
                    m.left = new / 2.0;
                    m.right = new / 2.0;
                }
                changed = true;
            }
            if changed {
                touch(st, StrokeField::Widths);
            }
            // A position typed past a neighbour: the nodes in order again
            // (the outline's widths read them in order).
            if a.changed() {
                let (nodes, j) =
                    crate::variable_outline::sorted(std::mem::take(&mut st.stroke.widths), i);
                st.stroke.widths = nodes;
                st.node = Some(j);
            }
        }
        if ui
            .add_enabled(node.is_some(), egui::Button::new(tr("vo.delete_node")))
            .clicked()
        {
            if let Some(i) = node {
                st.stroke.widths.remove(i);
                st.node = None;
                touch(st, StrokeField::Widths);
            }
        }
    });
}

/// The pictures of the dialog's buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pic {
    DashDefault,
    DashAlign,
    JoinMiter,
    JoinRound,
    JoinBevel,
    CapButt,
    CapRound,
    CapSquare,
    Outside,
    Center,
    Inside,
}

/// A 28 x 24 button with a picture; true when clicked.
fn pic_button(ui: &mut Ui, pic: Pic, on: bool, enabled: bool, tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(28.0, 24.0), Sense::click());
    let resp = resp.rich_tip(tr(tip));
    let p = ui.painter();
    if on {
        p.rect_filled(rect, 3.0, Tokens::ACCENT.gamma_multiply(0.35));
    } else if resp.hovered() && enabled {
        p.rect_filled(rect, 3.0, Tokens::tool_hover());
    }
    let ink = if enabled {
        Tokens::text()
    } else {
        Tokens::text_dim().gamma_multiply(0.5)
    };
    draw_pic(p, rect.shrink(4.0), pic, ink);
    enabled && resp.clicked()
}

fn draw_pic(p: &egui::Painter, r: Rect, pic: Pic, ink: Color32) {
    let thin = EStroke::new(1.0, ink);
    let thick = |w: f32| EStroke::new(w, ink);
    let (l, t, rt, b) = (r.left(), r.top(), r.right(), r.bottom());
    let c = r.center();
    match pic {
        Pic::DashDefault | Pic::DashAlign => {
            // An L of dashes; aligned ones meet in the corner.
            let pts = [Pos2::new(l, b), Pos2::new(l, t), Pos2::new(rt, t)];
            let seg = |a: Pos2, z: Pos2, from: f32, to: f32| [a + (z - a) * from, a + (z - a) * to];
            for (a, z) in [(pts[0], pts[1]), (pts[1], pts[2])] {
                let runs: &[(f32, f32)] = if pic == Pic::DashAlign {
                    &[(0.0, 0.2), (0.4, 0.6), (0.8, 1.0)]
                } else {
                    &[(0.1, 0.35), (0.55, 0.8)]
                };
                for (f, g) in runs {
                    p.line_segment(seg(a, z, *f, *g), thick(2.0));
                }
            }
        }
        Pic::JoinMiter | Pic::JoinRound | Pic::JoinBevel => {
            // A thick corner, its outer edge as the join makes it.
            let w = 6.0;
            let inner = Pos2::new(l + w, t + w);
            let mut outer = vec![Pos2::new(l, b)];
            match pic {
                Pic::JoinMiter => outer.push(Pos2::new(l, t)),
                Pic::JoinBevel => {
                    outer.push(Pos2::new(l, t + w));
                    outer.push(Pos2::new(l + w, t));
                }
                _ => {
                    for k in 0..=6 {
                        let a = std::f32::consts::PI + std::f32::consts::FRAC_PI_2 * k as f32 / 6.0;
                        outer.push(Pos2::new(l + w + w * a.cos(), t + w + w * a.sin()));
                    }
                }
            }
            outer.push(Pos2::new(rt, t));
            let mut shape = outer.clone();
            shape.push(Pos2::new(rt, t + w));
            shape.push(inner);
            shape.push(Pos2::new(l + w, b));
            p.add(egui::Shape::closed_line(shape, thin));
            p.line_segment(
                [
                    Pos2::new(l + w / 2.0, b),
                    Pos2::new(l + w / 2.0, t + w / 2.0),
                ],
                EStroke::new(1.0, Tokens::ACCENT),
            );
            p.line_segment(
                [
                    Pos2::new(l + w / 2.0, t + w / 2.0),
                    Pos2::new(rt, t + w / 2.0),
                ],
                EStroke::new(1.0, Tokens::ACCENT),
            );
        }
        Pic::CapButt | Pic::CapRound | Pic::CapSquare => {
            // A thick line ending at the path's end (a thin upright).
            let end = c.x + 2.0;
            let h = 4.0;
            let reach = match pic {
                Pic::CapButt => end,
                _ => end + h,
            };
            let body =
                Rect::from_min_max(Pos2::new(l, c.y - h), Pos2::new(reach.min(end), c.y + h));
            p.rect_stroke(body, 0.0, thin, egui::StrokeKind::Middle);
            match pic {
                Pic::CapRound => {
                    let pts: Vec<Pos2> = (0..=8)
                        .map(|k| {
                            let a = -std::f32::consts::FRAC_PI_2
                                + std::f32::consts::PI * k as f32 / 8.0;
                            Pos2::new(end + h * a.cos(), c.y + h * a.sin())
                        })
                        .collect();
                    p.add(egui::Shape::line(pts, thin));
                }
                Pic::CapSquare => {
                    p.rect_stroke(
                        Rect::from_min_max(Pos2::new(end, c.y - h), Pos2::new(end + h, c.y + h)),
                        0.0,
                        thin,
                        egui::StrokeKind::Middle,
                    );
                }
                _ => {}
            }
            p.line_segment(
                [Pos2::new(l, c.y), Pos2::new(end, c.y)],
                EStroke::new(1.0, Tokens::ACCENT),
            );
            p.line_segment([Pos2::new(end, t), Pos2::new(end, b)], thin);
        }
        Pic::Outside | Pic::Center | Pic::Inside => {
            // The path (a thin line) and the outline's band against it.
            let band = 6.0;
            let y = c.y;
            let (y0, y1) = match pic {
                Pic::Outside => (y - band, y),
                Pic::Center => (y - band / 2.0, y + band / 2.0),
                _ => (y, y + band),
            };
            p.rect_filled(
                Rect::from_min_max(Pos2::new(l, y0), Pos2::new(rt, y1)),
                0.0,
                ink,
            );
            p.line_segment(
                [Pos2::new(l, y), Pos2::new(rt, y)],
                EStroke::new(1.0, Tokens::ACCENT),
            );
            // The object's inside, light, below the path.
            p.rect_stroke(
                Rect::from_min_max(Pos2::new(l, y), Pos2::new(rt, b)),
                0.0,
                EStroke::new(1.0, Tokens::text_dim()),
                egui::StrokeKind::Inside,
            );
        }
    }
}

/// A dash pattern drawn as a line across `r`.
fn dash_line(p: &egui::Painter, r: Rect, dash: &[f64]) {
    let y = r.center().y;
    let ink = EStroke::new(2.0, Tokens::text());
    if dash.is_empty() {
        p.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], ink);
        return;
    }
    let unit = 2.0f32;
    let mut x = r.left();
    let mut i = 0;
    while x < r.right() {
        let len = dash[i % dash.len()] as f32 * unit;
        if i % 2 == 0 {
            p.line_segment(
                [Pos2::new(x, y), Pos2::new((x + len).min(r.right()), y)],
                ink,
            );
        }
        x += len.max(0.5);
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect as PRect;
    use tracedraw_core::ShapeKind;

    /// The dialog's width list and the property bar's width list offer
    /// the same widths in every outline unit, and the dialog's labels are
    /// the property bar's labels without the unit (#967). In millimetres
    /// both start 0.1, 0.2, 0.25 rather than points turned into mm.
    #[test]
    fn the_width_list_matches_the_property_bar() {
        use crate::ui::propbar::{outline_width_label, outline_width_presets, OUTLINE_UNITS};
        for u in OUTLINE_UNITS {
            let bar = outline_width_presets(u);
            let dialog = width_choices(u);
            assert_eq!(dialog.len(), bar.len(), "{u:?}");
            for ((mm, label), want) in dialog.iter().zip(&bar) {
                assert!((mm - want).abs() < 1e-12, "{u:?}: {mm} vs {want}");
                assert_eq!(
                    format!("{label} {}", u.short()),
                    outline_width_label(*want, u),
                    "{u:?}"
                );
            }
        }
        let mm: Vec<String> = width_choices(crate::app::Units::Millimeters)
            .into_iter()
            .map(|(_, l)| l)
            .collect();
        assert_eq!(&mm[..3], &["0.1", "0.2", "0.25"]);
    }

    fn rect(app: &mut App, x: f64) -> tracedraw_core::ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: PRect::new(x, 10.0, x + 30.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect")
    }

    /// Run the dialog for one frame with `events`.
    fn run_dialog(
        ctx: &egui::Context,
        app: &mut App,
        st: &mut OutlinePenState,
        events: Vec<egui::Event>,
    ) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1200.0),
            )),
            events,
            ..Default::default()
        };
        let mut close = false;
        let mut out = ctx.run_ui(input, |ui| {
            let c = ui.ctx().clone();
            dialog(app, &c, st, &mut close);
        });
        out.textures_delta.clear();
    }

    fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// A drag on the bar from `from` to `to`, then a frame with nothing.
    fn drag_on(
        ctx: &egui::Context,
        app: &mut App,
        st: &mut OutlinePenState,
        from: egui::Pos2,
        to: egui::Pos2,
    ) {
        run_dialog(ctx, app, st, vec![egui::Event::PointerMoved(from)]);
        run_dialog(ctx, app, st, vec![press(from, true)]);
        for k in 1..=4 {
            let p = from + (to - from) * (k as f32 / 4.0);
            run_dialog(ctx, app, st, vec![egui::Event::PointerMoved(p)]);
        }
        run_dialog(ctx, app, st, vec![press(to, false)]);
        run_dialog(ctx, app, st, Vec::new());
    }

    /// The Variable Outline bar's width nodes drag along it, kept between
    /// their neighbours and the ends, the fields following; a drag there
    /// no longer moves the dialog (#652). A click on empty bar adds a
    /// node, on a node picks it, a drag on empty bar adds one and slides
    /// it; the drawing shows the change at once and OK keeps it as one
    /// step.
    #[test]
    fn a_width_node_drags_along_the_bar_between_its_neighbours() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let a = rect(&mut app, 10.0);
        app.select(vec![a]);
        app.open_outline_pen();
        let crate::ui::dialogs::Dialog::OutlinePen(mut st) =
            std::mem::replace(&mut app.dialog, crate::ui::dialogs::Dialog::None)
        else {
            panic!("the dialog");
        };
        st.stroke.widths = vec![
            WidthNode::symmetric(0.25, 1.0),
            WidthNode::symmetric(0.75, 2.0),
        ];
        let steps = app.engine.history_labels().0.len();
        for _ in 0..4 {
            run_dialog(&ctx, &mut app, &mut st, Vec::new());
        }
        let bar = ctx
            .read_response(egui::Id::new(VO_BAR))
            .map(|r| r.rect)
            .expect("the bar");
        let window_id = egui::Id::new(("chrome_dialog", tr("outline_pen.title")));
        let window = ctx.memory(|m| m.area_rect(window_id)).expect("the window");
        let x = |t: f32| egui::pos2(bar.left() + bar.width() * t, bar.center().y);
        drag_on(&ctx, &mut app, &mut st, x(0.25), x(0.5));
        assert!(
            (st.stroke.widths[0].at - 0.5).abs() < 0.01,
            "{:?}",
            st.stroke.widths
        );
        assert!(
            (st.stroke.widths[0].left - 0.5).abs() < 1e-9,
            "the width stays"
        );
        assert_eq!(st.node, Some(0));
        assert!(st.touched.contains(&StrokeField::Widths));
        let moved = ctx.memory(|m| m.area_rect(window_id)).expect("the window");
        assert!(
            (moved.min - window.min).length() < 0.5,
            "the dialog stays: {moved:?}"
        );
        // Shown on the drawing at once, as a preview OK keeps (#741).
        assert_eq!(
            app.doc()
                .find_shape(a)
                .and_then(|s| s.stroke.as_ref())
                .map(|s| s.widths.clone()),
            Some(st.stroke.widths.clone())
        );
        // Not past its neighbour, nor past the start.
        drag_on(&ctx, &mut app, &mut st, x(0.5), x(0.95));
        let w = &st.stroke.widths;
        assert!(w[0].at < w[1].at && w[0].at > 0.74, "{w:?}");
        // Just left of the first node (the second sits right of it).
        let first = x(st.stroke.widths[0].at as f32 - 0.002);
        drag_on(
            &ctx,
            &mut app,
            &mut st,
            first,
            egui::pos2(bar.left() - 50.0, bar.center().y),
        );
        assert_eq!(st.stroke.widths[0].at, 0.0);
        // A click on empty bar adds a node there and picks it.
        run_dialog(
            &ctx,
            &mut app,
            &mut st,
            vec![egui::Event::PointerMoved(x(0.6))],
        );
        run_dialog(&ctx, &mut app, &mut st, vec![press(x(0.6), true)]);
        run_dialog(&ctx, &mut app, &mut st, vec![press(x(0.6), false)]);
        assert_eq!(st.stroke.widths.len(), 3);
        assert_eq!(st.node, Some(1));
        assert!((st.stroke.widths[1].at - 0.6).abs() < 0.01);
        // A drag on empty bar adds a node and slides it.
        drag_on(&ctx, &mut app, &mut st, x(0.3), x(0.4));
        assert_eq!(st.stroke.widths.len(), 4);
        assert!(
            (st.stroke.widths[1].at - 0.4).abs() < 0.01,
            "{:?}",
            st.stroke.widths
        );
        assert!(st.stroke.widths.windows(2).all(|p| p[0].at <= p[1].at));
        app.end_outline_pen_preview();
        assert_eq!(app.engine.history_labels().0.len(), steps, "taken back");
        app.apply_outline_pen(&st);
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        let got = app
            .doc()
            .find_shape(a)
            .and_then(|s| s.stroke.clone())
            .expect("outline");
        assert_eq!(got.widths, st.stroke.widths);
    }

    fn frame(ctx: &egui::Context, app: &mut App) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
    }

    /// The dialog's outline shows on the selection while the dialog is
    /// open, made again only when a value changes (not every frame);
    /// Esc takes it back, history and all, and OK keeps it as one step
    /// (#741).
    #[test]
    fn the_outline_shows_live_while_the_dialog_is_open() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        let a = rect(&mut app, 10.0);
        app.select(vec![a]);
        let width = |app: &App| {
            app.doc()
                .find_shape(a)
                .and_then(|s| s.stroke.as_ref())
                .map(|s| s.width)
        };
        let before = width(&app);
        let steps = app.engine.history_labels();
        let edit = |app: &mut App, w: f64| {
            let crate::ui::dialogs::Dialog::OutlinePen(st) = &mut app.dialog else {
                panic!("the dialog");
            };
            st.stroke.width = w;
            if !st.touched.contains(&StrokeField::Width) {
                st.touched.push(StrokeField::Width);
            }
        };
        app.open_outline_pen();
        frame(&ctx, &mut app);
        assert_eq!(width(&app), before, "nothing changed yet");
        assert!(!app.outline_preview);
        edit(&mut app, 2.0);
        frame(&ctx, &mut app);
        assert_eq!(width(&app), Some(2.0), "shown at once");
        // Idle frames leave the preview as it is.
        let rev = app.engine.revision();
        for _ in 0..3 {
            frame(&ctx, &mut app);
        }
        assert_eq!(app.engine.revision(), rev, "made again only on a change");
        edit(&mut app, 3.0);
        frame(&ctx, &mut app);
        assert_eq!(width(&app), Some(3.0));
        // Esc (the keyboard closes any dialog): taken back.
        app.dialog = crate::ui::dialogs::Dialog::None;
        frame(&ctx, &mut app);
        assert_eq!(width(&app), before);
        assert_eq!(app.engine.history_labels(), steps);
        assert!(!app.outline_preview);
        // OK: one step.
        app.open_outline_pen();
        frame(&ctx, &mut app);
        edit(&mut app, 2.5);
        frame(&ctx, &mut app);
        let crate::ui::dialogs::Dialog::OutlinePen(st) = &app.dialog else {
            panic!("the dialog");
        };
        let st = st.as_ref().clone();
        app.end_outline_pen_preview();
        app.apply_outline_pen(&st);
        assert_eq!(app.engine.history_labels().0.len(), steps.0.len() + 1);
        assert_eq!(width(&app), Some(2.5));
    }

    /// A node slides between its neighbours and the ends only.
    #[test]
    fn a_slid_node_stops_at_its_neighbours_and_the_ends() {
        let mut w = vec![
            WidthNode::symmetric(0.2, 1.0),
            WidthNode::symmetric(0.6, 1.0),
        ];
        assert!(slide_node(&mut w, 0, 0.9));
        assert!((w[0].at - (0.6 - 1e-4)).abs() < 1e-12);
        assert!(slide_node(&mut w, 1, 2.0));
        assert_eq!(w[1].at, 1.0);
        assert!(slide_node(&mut w, 0, -1.0));
        assert_eq!(w[0].at, 0.0);
        assert!(!slide_node(&mut w, 0, 0.0), "already there");
        assert!(!slide_node(&mut w, 5, 0.5), "no such node");
    }

    /// The dialog opens on the selection's outline; OK sends only what was
    /// changed to every selected object, one undo step with overprint;
    /// with nothing selected it sets the default outline (#357).
    #[test]
    fn the_outline_pen_applies_what_was_changed() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0);
        let b = rect(&mut app, 60.0);
        app.select(vec![a]);
        app.apply_outline_color(Some(tracedraw_core::Color::rgb8(255, 0, 0)));
        app.select(vec![a, b]);
        app.open_outline_pen();
        let crate::ui::dialogs::Dialog::OutlinePen(mut st) =
            std::mem::replace(&mut app.dialog, crate::ui::dialogs::Dialog::None)
        else {
            panic!("the dialog");
        };
        assert!(st.has);
        assert!(st.mix.has(StrokeField::Paint));
        // Round corners and overprint; the colours stay each object's.
        st.stroke.join = LineJoin::Round;
        st.touched.push(StrokeField::Join);
        st.overprint = Some(true);
        let steps = app.engine.history_labels().0.len();
        app.apply_outline_pen(&st);
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        let sa = app.doc().find_shape(a).cloned().expect("a");
        let sb = app.doc().find_shape(b).cloned().expect("b");
        assert_eq!(sa.stroke.as_ref().map(|s| s.join), Some(LineJoin::Round));
        assert_eq!(sb.stroke.as_ref().map(|s| s.join), Some(LineJoin::Round));
        assert_ne!(
            sa.stroke.as_ref().map(|s| s.color),
            sb.stroke.as_ref().map(|s| s.color)
        );
        assert!(sa.overprint_outline && sb.overprint_outline);
        // Nothing selected: the default outline.
        app.select(Vec::new());
        app.open_outline_pen();
        let crate::ui::dialogs::Dialog::OutlinePen(mut st) =
            std::mem::replace(&mut app.dialog, crate::ui::dialogs::Dialog::None)
        else {
            panic!("the dialog");
        };
        assert!(st.defaults);
        st.stroke.width = 2.0;
        st.has = true;
        app.apply_outline_pen(&st);
        assert!(
            matches!(app.dialog, crate::ui::dialogs::Dialog::ChangeDefaults(_))
                || app
                    .default_stroke()
                    .is_some_and(|s| (s.width - 2.0).abs() < 1e-9)
        );
    }

    #[test]
    fn the_nib_picture_stretches_and_turns() {
        let c = Pos2::new(50.0, 50.0);
        let flat = nib_corners(c, 20.0, 0.5, 0.0);
        assert!((flat[0].y - 40.0).abs() < 1e-4 && (flat[1].x - 70.0).abs() < 1e-4);
        let turned = nib_corners(c, 20.0, 1.0, 90.0);
        // A quarter turn: the right side's corner goes up.
        assert!((turned[1].x - 30.0).abs() < 1e-3 || (turned[1].y - 30.0).abs() < 1e-3);
    }
}
