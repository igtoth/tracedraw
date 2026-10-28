//! The Transformations docker: Position, Rotate, Scale and Mirror, Size
//! and Skew, chosen with the buttons at its top (Alt+F7, Alt+F8, Alt+F9
//! and Alt+F10 open it on the first four). Each page shows its fields
//! beside a 3 x 3 reference point: the point placed, the centre of
//! rotation, the point a scale, a size or a skew keeps still. Under them
//! come the number of copies and Apply. With no copies Apply transforms
//! the selection; with copies it leaves the selection as it is and makes
//! that many copies, each the one before transformed again, in one undo
//! step, and the last copies become the selection.
//!
//! The absolute fields (the position with Relative Position off, the
//! centre with Relative Center off, the size) follow the selection: they
//! show where it is and how large it is whenever it changes.
//!
//! On the Position page with Relative Position on, the 3 x 3 boxes pick a
//! direction instead of a point: a side or a corner fills X and Y with the
//! selection's width and height that way, so Apply puts the selection (or
//! each copy) right next to the last one; the centre fills 0. The offset
//! follows the selection until a field is typed into.

use crate::app::{App, TransformTab, Units};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::propbar::{origin_point, pic_button, Pic};
use crate::ui::tips::RichTip;
use egui::{Color32, Rect, Sense, Stroke, Ui, Vec2};
use tracedraw_core::geometry::{Affine, Point, Rect as PageRect, Vec2 as PageVec};
use tracedraw_core::{Command, ShapeId};

/// At most this many copies come from one Apply.
pub const MAX_COPIES: u32 = 500;

/// The docker's fields, kept while TraceDraw runs. Distances are in
/// millimetres, angles in degrees, scales in percent; absolute positions
/// count from the ruler origin, as the rulers do.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformForm {
    pub tab: TransformTab,
    pub position: [f64; 2],
    pub relative_position: bool,
    pub angle: f64,
    pub center: [f64; 2],
    pub relative_center: bool,
    pub scale: [f64; 2],
    /// Mirror horizontally (left and right swap) and vertically.
    pub mirror: [bool; 2],
    pub scale_proportional: bool,
    pub size: [f64; 2],
    pub size_proportional: bool,
    pub skew: [f64; 2],
    /// Skew about the reference point instead of the centre.
    pub use_anchor: bool,
    /// (column, row) of the 3 x 3 reference point, (1, 1) the centre.
    pub anchor: (u8, u8),
    pub copies: u32,
    /// The position fields hold the direction offset of the reference
    /// point (Position page, Relative Position on) and follow the
    /// selection's size; typing into them clears it.
    direction: bool,
    /// The centre of rotation the user moved for the selection on the
    /// canvas (page space), set before each use: the Rotate page turns
    /// about it instead of the reference point.
    pub pivot: Option<Point>,
    /// The selection, its bounds and its moved centre of rotation the
    /// absolute fields were last filled from.
    synced: Option<(Vec<ShapeId>, PageRect, Option<Point>)>,
}

impl Default for TransformForm {
    fn default() -> Self {
        TransformForm {
            tab: TransformTab::Position,
            position: [0.0; 2],
            relative_position: true,
            angle: 0.0,
            center: [0.0; 2],
            relative_center: true,
            scale: [100.0; 2],
            mirror: [false; 2],
            scale_proportional: true,
            size: [0.0; 2],
            size_proportional: true,
            skew: [0.0; 2],
            use_anchor: false,
            anchor: (1, 1),
            copies: 0,
            direction: false,
            pivot: None,
            synced: None,
        }
    }
}

/// `m` about the point `p`.
fn about(p: Point, m: Affine) -> Affine {
    Affine::translate(p.to_vec2()) * m * Affine::translate(-p.to_vec2())
}

/// The offset of a side or corner of `b` from its centre, picked by the
/// reference point `(column, row)`; the top row is the top of `b`.
pub fn direction_offset(b: PageRect, anchor: (u8, u8)) -> [f64; 2] {
    let step = |k: u8| k.min(2) as f64 - 1.0;
    // `+ 0.0` turns a negative zero into 0, so the field never shows -0.
    [
        step(anchor.0) * b.width() + 0.0,
        -step(anchor.1) * b.height() + 0.0,
    ]
}

impl TransformForm {
    /// Whether the 3 x 3 boxes pick a direction (Position page, Relative
    /// Position on) rather than a point.
    pub fn picks_direction(&self) -> bool {
        self.tab == TransformTab::Position && self.relative_position
    }

    /// The reference point was clicked: on the Position page with
    /// Relative Position on, fill X and Y with the offset that way.
    pub fn pick_anchor(&mut self, bounds: Option<PageRect>) {
        if self.picks_direction() {
            self.direction = true;
            if let Some(b) = bounds {
                self.position = direction_offset(b, self.anchor);
            }
        }
    }

    /// A position field was typed into: it no longer follows the
    /// selection's size.
    pub fn typed_position(&mut self) {
        self.direction = false;
    }

    /// Fill the absolute fields from the selection when it or its bounds
    /// changed since the last time.
    pub fn sync(&mut self, selection: &[ShapeId], bounds: Option<PageRect>, origin: Point) {
        let Some(b) = bounds else {
            self.synced = None;
            return;
        };
        if self.synced.as_ref().is_some_and(|(ids, sb, pv)| {
            ids.as_slice() == selection && *sb == b && *pv == self.pivot
        }) {
            return;
        }
        self.synced = Some((selection.to_vec(), b, self.pivot));
        let a = origin_point(b, self.anchor);
        if !self.relative_position {
            self.position = [a.x - origin.x, a.y - origin.y];
        } else if self.direction {
            self.position = direction_offset(b, self.anchor);
        }
        if !self.relative_center {
            let c = self.rotation_base(b);
            self.center = [c.x - origin.x, c.y - origin.y];
        }
        self.size = [b.width(), b.height()];
    }

    /// Fill the absolute fields again on the next frame (the reference
    /// point or a Relative box changed).
    pub fn resync(&mut self) {
        self.synced = None;
    }

    /// The point the Rotate page turns about before its offset: the
    /// centre of rotation moved on the canvas, else the reference point.
    fn rotation_base(&self, b: PageRect) -> Point {
        self.pivot.unwrap_or_else(|| origin_point(b, self.anchor))
    }

    /// The centre the Rotate page turns a selection with bounds `b` about.
    pub fn rotation_center(&self, b: PageRect, origin: Point) -> Point {
        let c = PageVec::new(self.center[0], self.center[1]);
        if self.relative_center {
            self.rotation_base(b) + c
        } else {
            origin + c
        }
    }

    /// The transformation of the current page for a selection with bounds
    /// `b`, the ruler origin at `origin` (page space, millimetres).
    pub fn affine(&self, b: PageRect, origin: Point) -> Affine {
        let a = origin_point(b, self.anchor);
        match self.tab {
            TransformTab::Position => {
                let d = PageVec::new(self.position[0], self.position[1]);
                if self.relative_position {
                    Affine::translate(d)
                } else {
                    Affine::translate(origin.to_vec2() + d - a.to_vec2())
                }
            }
            TransformTab::Rotate => about(
                self.rotation_center(b, origin),
                Affine::rotate(self.angle.to_radians()),
            ),
            TransformTab::Scale => {
                let sign = |m: bool| if m { -1.0 } else { 1.0 };
                about(
                    a,
                    Affine::scale_non_uniform(
                        self.scale[0] / 100.0 * sign(self.mirror[0]),
                        self.scale[1] / 100.0 * sign(self.mirror[1]),
                    ),
                )
            }
            TransformTab::Size => {
                let factor = |want: f64, have: f64| {
                    if have > 1e-9 && want.is_finite() && want > 0.0 {
                        want / have
                    } else {
                        1.0
                    }
                };
                about(
                    a,
                    Affine::scale_non_uniform(
                        factor(self.size[0], b.width()),
                        factor(self.size[1], b.height()),
                    ),
                )
            }
            TransformTab::Skew => {
                let p = if self.use_anchor { a } else { b.center() };
                about(
                    p,
                    Affine::skew(
                        self.skew[0].to_radians().tan(),
                        self.skew[1].to_radians().tan(),
                    ),
                )
            }
        }
    }
}

/// The name of a page of the docker and its undo step.
fn tab_info(tab: TransformTab) -> (Pic, &'static str, &'static str) {
    match tab {
        TransformTab::Position => (Pic::Position, "docker.position", "Position"),
        TransformTab::Rotate => (Pic::Rotate, "docker.rotate", "Rotate"),
        TransformTab::Scale => (
            Pic::ScaleMirror,
            "docker.scale_and_mirror",
            "Scale and Mirror",
        ),
        TransformTab::Size => (Pic::SizeBox, "docker.size", "Size"),
        TransformTab::Skew => (Pic::SkewBox, "docker.skew", "Skew"),
    }
}

const TABS: [TransformTab; 5] = [
    TransformTab::Position,
    TransformTab::Rotate,
    TransformTab::Scale,
    TransformTab::Size,
    TransformTab::Skew,
];

/// What the docker works on: the selection's bounds, else (no objects
/// selected) the selected guidelines' box (#376).
fn target_bounds(app: &App) -> Option<PageRect> {
    app.selection_bounds().or_else(|| {
        app.selection
            .is_empty()
            .then(|| app.selected_guides_bounds())
            .flatten()
    })
}

/// Apply the docker's page to the selection (see the module notes), or
/// with no objects selected to the selected guidelines.
pub fn apply(app: &mut App) {
    let Some(b) = target_bounds(app) else {
        return;
    };
    if app.selection.is_empty() {
        let origin = app.ruler_origin();
        let t = app.transform.affine(b, origin);
        let copies = app.transform.copies.min(MAX_COPIES);
        let label = tab_info(app.transform.tab).2;
        app.transform_selected_guides(t, copies, label);
        return;
    }
    app.transform.pivot = app.moved_rotation_center();
    let origin = app.ruler_origin();
    let t = app.transform.affine(b, origin);
    // Rotating keeps its centre where it is for what comes out of it, so
    // a next Apply turns about the same point (copies round a centre).
    let keep_centre = (app.transform.tab == TransformTab::Rotate)
        .then(|| app.transform.rotation_center(b, origin));
    let copies = app.transform.copies.min(MAX_COPIES);
    if copies == 0 {
        app.transform_selection(t);
        if let Some(c) = keep_centre {
            app.set_pivot(c);
        }
        return;
    }
    let shapes = app.selected_shapes();
    let Some(layer) = app.active_layer() else {
        return;
    };
    let mut cmds = Vec::new();
    let mut last = Vec::new();
    let mut step = Affine::IDENTITY;
    for _ in 0..copies {
        step = t * step;
        last.clear();
        for s in &shapes {
            let id = app.engine.new_shape_id();
            let mut shape = crate::app::reid_pub(s.clone(), id, &mut app.engine);
            shape.transform_by(step);
            last.push(id);
            cmds.push(Command::AddShape { layer, shape });
        }
    }
    let label = tab_info(app.transform.tab).2;
    if let Err(e) = app.engine.run_batch(label, &cmds) {
        app.status = e.to_string();
        return;
    }
    app.select(last);
    if let Some(c) = keep_centre {
        app.set_pivot(c);
    }
}

/// Open the docker on `tab`; with `toggle` (the shortcuts), close the
/// docker column instead when it already shows that page.
pub fn open(app: &mut App, tab: TransformTab, toggle: bool) {
    let shown =
        app.docker_shown(crate::app::DockerTab::Transformations) && app.transform.tab == tab;
    if shown && toggle {
        app.toggle_docker(crate::app::DockerTab::Transformations);
    } else {
        app.open_docker(crate::app::DockerTab::Transformations);
        app.transform.tab = tab;
    }
}

/// A distance in a spin box (up and down arrows at its right), the
/// unit's name after it.
fn length(ui: &mut Ui, id: &str, units: Units, decimals: usize, mm: &mut f64) -> bool {
    let unit = units.world_mm();
    let unit = if unit.is_finite() && unit > 0.0 {
        unit
    } else {
        1.0
    };
    // A tenth of an inch or a whole millimetre (or pixel) per click.
    let step = if units.mm() >= 10.0 { 0.1 } else { 1.0 };
    let v = *mm / unit;
    let got = spin_with_unit(ui, id, v, decimals.min(3), step, units.short());
    match got {
        Some(n) if (n * unit - *mm).abs() > 1e-12 => {
            *mm = n * unit;
            true
        }
        _ => false,
    }
}

/// A number in a spin box with `suffix` (degrees, percent) after it.
fn number(ui: &mut Ui, id: &str, value: &mut f64, suffix: &str, step: f64) -> bool {
    let got = spin_with_unit(ui, id, *value, 1, step, suffix.trim());
    match got {
        Some(n) if (n - *value).abs() > 1e-12 => {
            *value = n;
            true
        }
        _ => false,
    }
}

/// A spin box and, right after it, the unit's name.
fn spin_with_unit(
    ui: &mut Ui,
    id: &str,
    value: f64,
    decimals: usize,
    step: f64,
    unit: &str,
) -> Option<f64> {
    let (cell, _) = ui.allocate_exact_size(Vec2::new(FIELD_W + 34.0, 22.0), Sense::hover());
    let rect = Rect::from_min_size(cell.min, Vec2::new(FIELD_W, 22.0));
    let got =
        crate::ui::chrome::spin_field(ui, id, rect, value, decimals, "", step, -1.0e6..=1.0e6);
    ui.painter().text(
        egui::pos2(rect.right() + 6.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        unit,
        egui::TextStyle::Body.resolve(ui.style()),
        ui.visuals().text_color(),
    );
    got
}

/// Width of the docker's spin boxes.
const FIELD_W: f32 = 96.0;

/// The 3 x 3 reference point: nine boxes on a frame, the chosen one
/// filled. True when another one was chosen.
fn anchor_grid(ui: &mut Ui, anchor: &mut (u8, u8), direction: bool) -> bool {
    let cell = 12.0;
    let step = 22.0;
    let size = Vec2::splat(step * 2.0 + cell + 2.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let resp = resp.rich_tip(tr(if direction {
        "docker.direction"
    } else {
        "docker.reference_point"
    }));
    let o = rect.min + Vec2::splat(1.0 + cell / 2.0);
    let centre = |i: u8, j: u8| o + Vec2::new(i as f32 * step, j as f32 * step);
    let p = ui.painter();
    p.rect_stroke(
        Rect::from_min_max(centre(0, 0), centre(2, 2)),
        0.0,
        Stroke::new(1.0, Tokens::control_border()),
        egui::StrokeKind::Middle,
    );
    let mut changed = false;
    for j in 0..3u8 {
        for i in 0..3u8 {
            let r = Rect::from_center_size(centre(i, j), Vec2::splat(cell));
            if direction && (i, j) == (1, 1) {
                // No direction: a ring, filled when chosen.
                let rad = cell / 2.0;
                if *anchor == (i, j) {
                    p.circle_filled(r.center(), rad, Tokens::icon());
                } else {
                    p.circle_filled(r.center(), rad, Color32::WHITE);
                    p.circle_stroke(r.center(), rad - 0.5, Stroke::new(1.0, Tokens::icon()));
                }
            } else if *anchor == (i, j) {
                p.rect_filled(r, 1.0, Tokens::icon());
            } else {
                p.rect_filled(r, 1.0, Color32::WHITE);
                p.rect_stroke(
                    r,
                    1.0,
                    Stroke::new(1.0, Tokens::icon()),
                    egui::StrokeKind::Inside,
                );
            }
            if resp.clicked() {
                if let Some(q) = resp.interact_pointer_pos() {
                    if r.expand((step - cell) / 2.0).contains(q) && (direction || *anchor != (i, j))
                    {
                        *anchor = (i, j);
                        changed = true;
                    }
                }
            }
        }
    }
    changed
}

/// The turning arrow before the angle box (Rotate).
fn rotate_pic(ui: &mut Ui) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::hover());
    let p = ui.painter();
    let c = r.center();
    let rad = 6.0;
    let pts: Vec<egui::Pos2> = (0..=20)
        .map(|k| {
            let a = (40.0 + k as f32 * 13.5_f32).to_radians();
            c + Vec2::new(a.cos() * rad, -a.sin() * rad)
        })
        .collect();
    let end = *pts.last().unwrap_or(&c);
    p.add(egui::Shape::line(pts, Stroke::new(1.4, Tokens::icon())));
    p.add(egui::epaint::PathShape::convex_polygon(
        vec![
            end + Vec2::new(-3.0, -3.0),
            end + Vec2::new(3.5, -1.0),
            end + Vec2::new(0.0, 3.5),
        ],
        Tokens::icon(),
        Stroke::NONE,
    ));
}

/// The direction picture beside the reference point when the offset is
/// relative: a small grid with arrows out to the four sides.
fn direction_pic(ui: &mut Ui) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(56.0, 56.0), Sense::hover());
    let p = ui.painter();
    let c = r.center();
    let inner = Rect::from_center_size(c, Vec2::splat(28.0));
    p.rect_filled(inner, 0.0, Color32::from_rgb(0xB8, 0xB4, 0xE0));
    let grid = Stroke::new(1.0, Color32::from_rgb(0x8C, 0x86, 0xC8));
    for k in 1..4 {
        let t = k as f32 * 7.0;
        p.line_segment(
            [
                inner.left_top() + Vec2::new(t, 0.0),
                inner.left_bottom() + Vec2::new(t, 0.0),
            ],
            grid,
        );
        p.line_segment(
            [
                inner.left_top() + Vec2::new(0.0, t),
                inner.right_top() + Vec2::new(0.0, t),
            ],
            grid,
        );
    }
    let ink = Stroke::new(1.4, Tokens::icon());
    for d in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
        let tip = c + d * 27.0;
        p.line_segment([c, tip], ink);
        let side = Vec2::new(-d.y, d.x) * 3.0;
        p.add(egui::epaint::PathShape::convex_polygon(
            vec![tip, tip - d * 4.0 + side, tip - d * 4.0 - side],
            Tokens::icon(),
            Stroke::NONE,
        ));
    }
}

/// A label for a field, in the first column of the grid.
fn label(ui: &mut Ui, key: &str) {
    ui.label(tr(key));
}

/// Draw the docker.
pub fn transformations(app: &mut App, ui: &mut Ui) {
    let bounds = target_bounds(app);
    let origin = app.ruler_origin();
    let selection = app.selection.clone();
    app.transform.pivot = app.moved_rotation_center();
    app.transform.sync(&selection, bounds, origin);
    let has = bounds.is_some();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for tab in TABS {
            let (pic, key, _) = tab_info(tab);
            if pic_button(ui, pic, &tr(key), true, app.transform.tab == tab).clicked() {
                app.transform.tab = tab;
            }
        }
    });
    ui.add_space(4.0);
    ui.separator();
    let (_, key, _) = tab_info(app.transform.tab);
    if app.transform.tab != TransformTab::Rotate {
        ui.label(format!("{}:", tr(key)));
    }
    ui.add_space(4.0);
    if !has {
        // Nothing to fill the fields from or to apply to: all pale, as
        // the page buttons above stay usable.
        ui.disable();
    }
    let units = app.units;
    let decimals = app.settings.precision.min(6) as usize;
    let form = &mut app.transform;
    let mut resync = false;
    // A reference point picked on the Rotate page: the centre of rotation
    // goes there (a centre moved on the canvas is let go).
    let mut reset_pivot = false;
    {
        {
            egui::Grid::new("transform_fields")
                .num_columns(2)
                .spacing([4.0, 6.0])
                .show(ui, |ui| match form.tab {
                    TransformTab::Position => {
                        label(ui, "docker.x");
                        let mut typed =
                            length(ui, "tf_pos_x", units, decimals, &mut form.position[0]);
                        ui.end_row();
                        label(ui, "docker.y");
                        typed |= length(ui, "tf_pos_y", units, decimals, &mut form.position[1]);
                        ui.end_row();
                        if typed {
                            form.typed_position();
                        }
                    }
                    TransformTab::Rotate => {
                        rotate_pic(ui);
                        number(ui, "tf_angle", &mut form.angle, "°", 1.0);
                        ui.end_row();
                        ui.label(tr("docker.center_colon"));
                        ui.end_row();
                        label(ui, "docker.x");
                        length(ui, "tf_cx", units, decimals, &mut form.center[0]);
                        ui.end_row();
                        label(ui, "docker.y");
                        length(ui, "tf_cy", units, decimals, &mut form.center[1]);
                        ui.end_row();
                    }
                    TransformTab::Scale => {
                        label(ui, "docker.x");
                        if number(ui, "tf_sx", &mut form.scale[0], "%", 1.0)
                            && form.scale_proportional
                        {
                            form.scale[1] = form.scale[0];
                        }
                        ui.end_row();
                        label(ui, "docker.y");
                        if number(ui, "tf_sy", &mut form.scale[1], "%", 1.0)
                            && form.scale_proportional
                        {
                            form.scale[0] = form.scale[1];
                        }
                        ui.end_row();
                    }
                    TransformTab::Size => {
                        let ratio = bounds
                            .filter(|b| b.width() > 1e-9 && b.height() > 1e-9)
                            .map(|b| b.height() / b.width());
                        label(ui, "docker.w");
                        if length(ui, "tf_w", units, decimals, &mut form.size[0])
                            && form.size_proportional
                        {
                            if let Some(r) = ratio {
                                form.size[1] = form.size[0] * r;
                            }
                        }
                        ui.end_row();
                        label(ui, "docker.h");
                        if length(ui, "tf_h", units, decimals, &mut form.size[1])
                            && form.size_proportional
                        {
                            if let Some(r) = ratio {
                                form.size[0] = form.size[1] / r;
                            }
                        }
                        ui.end_row();
                    }
                    TransformTab::Skew => {
                        label(ui, "docker.x");
                        number(ui, "tf_kx", &mut form.skew[0], "°", 1.0);
                        ui.end_row();
                        label(ui, "docker.y");
                        number(ui, "tf_ky", &mut form.skew[1], "°", 1.0);
                        ui.end_row();
                    }
                });
            ui.add_space(18.0);
            match form.tab {
                TransformTab::Position => {
                    resync |= ui
                        .checkbox(&mut form.relative_position, tr("docker.relative_position"))
                        .changed();
                    if resync && form.relative_position {
                        form.position = [0.0; 2];
                        form.typed_position();
                    }
                }
                TransformTab::Rotate => {
                    resync |= ui
                        .checkbox(&mut form.relative_center, tr("docker.relative_center"))
                        .changed();
                    if resync && form.relative_center {
                        form.center = [0.0; 2];
                    }
                }
                TransformTab::Scale => {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for (i, pic, key) in [
                            (0, Pic::MirrorH, "toolbar.mirror_h"),
                            (1, Pic::MirrorV, "toolbar.mirror_v"),
                        ] {
                            if pic_button(ui, pic, &tr(key), true, form.mirror[i]).clicked() {
                                form.mirror[i] = !form.mirror[i];
                            }
                        }
                    });
                    if ui
                        .checkbox(&mut form.scale_proportional, tr("docker.proportional"))
                        .changed()
                        && form.scale_proportional
                    {
                        form.scale[1] = form.scale[0];
                    }
                }
                TransformTab::Size => {
                    ui.checkbox(&mut form.size_proportional, tr("docker.proportional"));
                }
                TransformTab::Skew => {
                    ui.checkbox(&mut form.use_anchor, tr("docker.use_anchor_point"));
                }
            }
        }
        // The reference point under the check box, indented; for an
        // offset, the direction picture
        // beside it.
        ui.add_space(4.0);
        let anchor_matters = !matches!(form.tab, TransformTab::Skew) || form.use_anchor;
        let direction = form.picks_direction();
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            ui.add_enabled_ui(anchor_matters, |ui| {
                if anchor_grid(ui, &mut form.anchor, direction) {
                    resync = true;
                    form.pick_anchor(bounds);
                    if form.tab == TransformTab::Rotate {
                        reset_pivot = true;
                    }
                }
            });
            if direction {
                ui.add_space(22.0);
                direction_pic(ui);
            }
        });
    }
    if resync {
        form.resync();
    }
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(tr("dialog.copies"));
        ui.add_space(6.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(FIELD_W, 22.0), Sense::hover());
        if let Some(n) = crate::ui::chrome::spin_field(
            ui,
            "tf_copies",
            rect,
            form.copies as f64,
            0,
            "",
            1.0,
            0.0..=MAX_COPIES as f64,
        ) {
            form.copies = n.round().clamp(0.0, MAX_COPIES as f64) as u32;
        }
    });
    ui.add_space(8.0);
    let width = ui.available_width();
    let clicked = ui
        .add_enabled_ui(has, |ui| {
            ui.add_sized([width, 26.0], egui::Button::new(tr("docker.apply")))
                .clicked()
        })
        .inner;
    if reset_pivot {
        app.reset_pivot();
        app.transform.pivot = None;
    }
    if clicked {
        apply(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::ShapeKind;

    fn square(app: &mut App) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(10.0, 10.0, 30.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        app.select(vec![id]);
        id
    }

    fn bounds(app: &App) -> PageRect {
        app.selection_bounds().expect("bounds")
    }

    fn close(a: PageRect, b: (f64, f64, f64, f64)) {
        for (x, y) in [(a.x0, b.0), (a.y0, b.1), (a.x1, b.2), (a.y1, b.3)] {
            assert!((x - y).abs() < 1e-6, "{a:?} != {b:?}");
        }
    }

    #[test]
    fn position_moves_by_an_offset_or_to_a_point() {
        let mut app = App::headless();
        square(&mut app);
        app.transform.position = [5.0, -2.0];
        apply(&mut app);
        close(bounds(&app), (15.0, 8.0, 35.0, 18.0));
        // Absolute: the bottom-left corner goes to (0, 0) of the rulers.
        app.transform.relative_position = false;
        app.transform.anchor = (0, 2);
        app.transform.position = [0.0, 0.0];
        let origin = app.ruler_origin();
        apply(&mut app);
        let b = bounds(&app);
        assert!((b.x0 - origin.x).abs() < 1e-6 && (b.y0 - origin.y).abs() < 1e-6);
    }

    #[test]
    fn rotate_scale_size_and_skew_keep_the_reference_point_still() {
        let mut app = App::headless();
        square(&mut app);
        // A quarter turn about the centre (20, 15): 20 x 10 becomes 10 x 20.
        app.transform.tab = TransformTab::Rotate;
        app.transform.angle = 90.0;
        apply(&mut app);
        close(bounds(&app), (15.0, 5.0, 25.0, 25.0));
        app.undo();
        // Twice as large about the top-left corner (10, 20).
        app.transform.tab = TransformTab::Scale;
        app.transform.anchor = (0, 0);
        app.transform.scale = [200.0, 200.0];
        apply(&mut app);
        close(bounds(&app), (10.0, 0.0, 50.0, 20.0));
        app.undo();
        // Mirrored about the right edge: it flips to the right of it.
        app.transform.anchor = (2, 1);
        app.transform.scale = [100.0, 100.0];
        app.transform.mirror = [true, false];
        apply(&mut app);
        close(bounds(&app), (30.0, 10.0, 50.0, 20.0));
        app.undo();
        // A size of 40 x 5 about the bottom-right corner (30, 10).
        app.transform.tab = TransformTab::Size;
        app.transform.anchor = (2, 2);
        app.transform.size = [40.0, 5.0];
        apply(&mut app);
        close(bounds(&app), (-10.0, 10.0, 30.0, 15.0));
        app.undo();
        // A 45 degree horizontal skew about the bottom edge keeps it.
        app.transform.tab = TransformTab::Skew;
        app.transform.use_anchor = true;
        app.transform.anchor = (1, 2);
        app.transform.skew = [45.0, 0.0];
        apply(&mut app);
        let b = bounds(&app);
        assert!((b.y0 - 10.0).abs() < 1e-6 && (b.width() - 30.0).abs() < 1e-6);
    }

    /// What the docker does every frame before it draws: take the moved
    /// centre of rotation, then fill the absolute fields.
    fn frame_sync(app: &mut App) {
        app.transform.pivot = app.moved_rotation_center();
        let sel = app.selection.clone();
        let (b, o) = (app.selection_bounds(), app.ruler_origin());
        app.transform.sync(&sel, b, o);
    }

    #[test]
    fn rotated_copies_turn_about_the_centre_moved_on_the_canvas() {
        for relative in [true, false] {
            let mut app = App::headless();
            square(&mut app);
            // The centre of rotation dragged off the square, to its right.
            let pivot = Point::new(40.0, 15.0);
            app.set_pivot(pivot);
            app.transform.tab = TransformTab::Rotate;
            app.transform.relative_center = relative;
            app.transform.center = [0.0; 2];
            app.transform.angle = 90.0;
            app.transform.copies = 1;
            // Apply again and again: every copy lies at the same distance
            // from the moved centre (the square's middle was 20 mm from
            // it), each a quarter turn on.
            let mut middles = Vec::new();
            for _ in 0..3 {
                frame_sync(&mut app);
                apply(&mut app);
                middles.push(bounds(&app).center());
            }
            for m in &middles {
                assert!(
                    ((*m - pivot).hypot() - 20.0).abs() < 1e-6,
                    "relative {relative}: {middles:?}"
                );
            }
            assert!((middles[0] - middles[2]).hypot() > 39.0, "{middles:?}");
            // The absolute fields show the moved centre.
            if !relative {
                let o = app.ruler_origin();
                assert!((app.transform.center[0] - (pivot.x - o.x)).abs() < 1e-6);
                assert!((app.transform.center[1] - (pivot.y - o.y)).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn copies_repeat_the_transformation_in_one_step() {
        let mut app = App::headless();
        let original = square(&mut app);
        app.transform.position = [25.0, 0.0];
        app.transform.copies = 3;
        apply(&mut app);
        let layer = &app.doc().pages[0].layers[0];
        assert_eq!(layer.shapes.len(), 4);
        // The original stays; the last copy is the selection.
        let first = app.doc().shape_on_page(original).expect("original");
        close(first.bounds(), (10.0, 10.0, 30.0, 20.0));
        close(bounds(&app), (85.0, 10.0, 105.0, 20.0));
        // One Undo takes them all back.
        app.undo();
        assert_eq!(app.doc().pages[0].layers[0].shapes.len(), 1);
    }

    #[test]
    fn absolute_fields_follow_the_selection() {
        let mut app = App::headless();
        square(&mut app);
        let origin = app.ruler_origin();
        let mut form = TransformForm {
            relative_position: false,
            anchor: (0, 0),
            ..Default::default()
        };
        form.sync(&app.selection, app.selection_bounds(), origin);
        assert_eq!(form.position, [10.0 - origin.x, 20.0 - origin.y]);
        assert_eq!(form.size, [20.0, 10.0]);
        // Typed values stay until the selection changes.
        form.position = [1.0, 1.0];
        form.sync(&app.selection, app.selection_bounds(), origin);
        assert_eq!(form.position, [1.0, 1.0]);
        form.resync();
        form.sync(&app.selection, app.selection_bounds(), origin);
        assert_eq!(form.position, [10.0 - origin.x, 20.0 - origin.y]);
    }

    #[test]
    fn relative_position_boxes_pick_a_direction_the_size_of_the_selection() {
        let mut app = App::headless();
        square(&mut app); // 20 x 10 at (10, 10)
        let origin = app.ruler_origin();
        assert!(app.transform.picks_direction());
        // The right middle box: one width to the right.
        app.transform.anchor = (2, 1);
        app.transform.pick_anchor(app.selection_bounds());
        assert_eq!(app.transform.position, [20.0, 0.0]);
        // Two copies side by side, as the boxes said.
        app.transform.copies = 2;
        apply(&mut app);
        close(bounds(&app), (50.0, 10.0, 70.0, 20.0));
        // The bottom left box: one width left and one height down.
        app.transform.anchor = (0, 2);
        app.transform.pick_anchor(app.selection_bounds());
        assert_eq!(app.transform.position, [-20.0, -10.0]);
        app.transform.anchor = (1, 1);
        app.transform.pick_anchor(app.selection_bounds());
        assert_eq!(app.transform.position, [0.0, 0.0]);
        // The offset follows another selection's size...
        app.transform.anchor = (1, 0);
        app.transform.pick_anchor(app.selection_bounds());
        let tall = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 5.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        app.select(vec![tall]);
        let selection = app.selection.clone();
        app.transform
            .sync(&selection, app.selection_bounds(), origin);
        assert_eq!(app.transform.position, [0.0, 40.0]);
        // ...until a value is typed.
        app.transform.position = [3.0, 3.0];
        app.transform.typed_position();
        square(&mut app);
        let selection = app.selection.clone();
        app.transform
            .sync(&selection, app.selection_bounds(), origin);
        assert_eq!(app.transform.position, [3.0, 3.0]);
        // With Relative position off the boxes are points again.
        app.transform.relative_position = false;
        assert!(!app.transform.picks_direction());
        app.transform.anchor = (2, 1);
        app.transform.pick_anchor(app.selection_bounds());
        assert_eq!(app.transform.position, [3.0, 3.0]);
    }

    #[test]
    fn the_docker_draws_with_nothing_selected() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.select(vec![]);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 600.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| transformations(&mut app, ui));
        out.textures_delta.clear();
    }

    #[test]
    fn the_shortcuts_open_a_page_and_close_it_again() {
        let mut app = App::headless();
        app.show_dockers = false;
        open(&mut app, TransformTab::Rotate, true);
        assert!(app.show_dockers);
        assert_eq!(app.docker_tab, crate::app::DockerTab::Transformations);
        assert_eq!(app.transform.tab, TransformTab::Rotate);
        open(&mut app, TransformTab::Size, true);
        assert!(app.show_dockers);
        assert_eq!(app.transform.tab, TransformTab::Size);
        // The menu opens it whatever it showed.
        open(&mut app, TransformTab::Size, false);
        assert!(app.show_dockers);
        open(&mut app, TransformTab::Size, true);
        assert!(!app.show_dockers);
    }

    #[test]
    fn the_docker_draws_every_page() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        square(&mut app);
        for tab in TABS {
            app.transform.tab = tab;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 600.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| transformations(&mut app, ui));
            out.textures_delta.clear();
        }
    }
}
