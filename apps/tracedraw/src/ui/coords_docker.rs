//! The Coordinates docker (Window > Dockers > Coordinates): the object
//! buttons (rectangle and square, ellipse and circle, polygon and regular
//! polygon behind flyouts; star, complex star, 2-point line, multipoint
//! curve), the fields of the chosen object with their "Set ...
//! interactively" buttons, and Create object / Replace object. The
//! drawing previews the object with its origin point marked.

use crate::app::App;
use crate::coords::{CoordKind, CoordPick};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::field::NumField;
use crate::ui::propbar::{draw_pic, Pic};
use egui::{epaint, Color32, Key, Modifiers, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use tracedraw_core::geometry::{
    complex_star_max_sharpness, BezPath, Point, COMPLEX_STAR_MIN_POINTS,
};

const BTN: f32 = 28.0;
const FIELD_W: f32 = 84.0;

/// The picture of an object button.
fn draw_kind(painter: &egui::Painter, r: Rect, kind: CoordKind, color: Color32) {
    use crate::tools::Tool;
    let s = Stroke::new(1.2, color);
    let c = r.center();
    match kind {
        CoordKind::Rectangle => crate::ui::icons::draw(painter, r, Tool::Rectangle, color),
        CoordKind::Ellipse => crate::ui::icons::draw(painter, r, Tool::Ellipse, color),
        CoordKind::Polygon => crate::ui::icons::draw(painter, r, Tool::Polygon, color),
        CoordKind::Line => crate::ui::icons::draw(painter, r, Tool::TwoPointLine, color),
        CoordKind::MultiPoint => crate::ui::icons::draw(painter, r, Tool::Polyline, color),
        CoordKind::Star => draw_pic(painter, r, Pic::Star, color),
        CoordKind::ComplexStar => draw_pic(painter, r, Pic::ComplexStar, color),
        CoordKind::Square => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::splat(r.width() * 0.7)),
                0.0,
                s,
                egui::StrokeKind::Middle,
            );
        }
        CoordKind::Circle => {
            painter.circle_stroke(c, r.width() * 0.38, s);
        }
        CoordKind::RegularPolygon => {
            let rad = r.width() * 0.4;
            let pts: Vec<Pos2> = (0..6)
                .map(|i| {
                    let a = std::f32::consts::FRAC_PI_2 - i as f32 * std::f32::consts::TAU / 6.0;
                    c + Vec2::new(rad * a.cos(), -rad * a.sin())
                })
                .collect();
            painter.add(epaint::PathShape::closed_line(pts, s));
        }
    }
}

/// An object button: pressed while it is the docker's object; a flyout
/// button has a small triangle in its corner.
fn kind_button(ui: &mut Ui, kind: CoordKind, pressed: bool, flyout: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(BTN), Sense::click());
    if pressed {
        ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            0.0,
            Stroke::new(1.0, Tokens::CONTROL_BORDER),
            egui::StrokeKind::Middle,
        );
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(rect, 0.0, Color32::from_rgb(0xE0, 0xF0, 0xFF));
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            0.0,
            Stroke::new(1.0, Color32::from_rgb(0x00, 0xAD, 0xFE)),
            egui::StrokeKind::Middle,
        );
    }
    draw_kind(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(18.0)),
        kind,
        Tokens::ICON,
    );
    if flyout {
        let b = rect.right_bottom() + Vec2::new(-3.0, -3.0);
        ui.painter().add(epaint::PathShape::convex_polygon(
            vec![b, b + Vec2::new(-4.0, 0.0), b + Vec2::new(0.0, -4.0)],
            Tokens::ICON,
            Stroke::NONE,
        ));
    }
    resp.on_hover_text(tr(kind.key()))
}

/// A flyout of two objects: the button shows the chosen one (or the
/// first), a click lists both.
fn kind_flyout(app: &mut App, ui: &mut Ui, pair: [CoordKind; 2]) {
    let on = pair.contains(&app.coords.kind);
    let shown = if on { app.coords.kind } else { pair[0] };
    let resp = kind_button(ui, shown, on, true);
    egui::Popup::menu(&resp)
        .id(egui::Id::new(("coords_flyout", pair[0].key())))
        .style(crate::ui::menus::menu_popup_style)
        .show(|ui| {
            crate::ui::menus::body(ui, |ui| {
                for k in pair {
                    if crate::ui::menus::item(ui, k.key(), "", true) {
                        app.coords.set_kind(k);
                        app.coord_pick = None;
                    }
                }
            })
        });
}

fn kind_single(app: &mut App, ui: &mut Ui, kind: CoordKind) {
    if kind_button(ui, kind, app.coords.kind == kind, false).clicked() {
        app.coords.set_kind(kind);
        app.coord_pick = None;
    }
}

/// A heading over a group of fields.
fn section(ui: &mut Ui, key: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(tr(key)).strong());
}

/// A "Set ... interactively" button: pressed while its pick waits.
fn pick_button(
    ui: &mut Ui,
    waiting: Option<CoordPick>,
    pick: CoordPick,
    out: &mut Option<CoordPick>,
) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(22.0, 20.0), Sense::click());
    let pressed = waiting == Some(pick);
    if pressed || resp.hovered() {
        ui.painter().rect_filled(
            rect,
            0.0,
            if pressed {
                Color32::WHITE
            } else {
                Color32::from_rgb(0xE0, 0xF0, 0xFF)
            },
        );
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            0.0,
            Stroke::new(1.0, Tokens::CONTROL_BORDER),
            egui::StrokeKind::Middle,
        );
    }
    // A pointer with a small target: click or drag on the drawing.
    let c = rect.center();
    let s = Stroke::new(1.1, Tokens::ICON);
    ui.painter()
        .circle_stroke(c + Vec2::new(-3.0, -2.0), 3.5, s);
    ui.painter().line_segment(
        [c + Vec2::new(-3.0, -7.0), c + Vec2::new(-3.0, 3.0)],
        Stroke::new(0.8, Tokens::ICON),
    );
    ui.painter().line_segment(
        [c + Vec2::new(-8.0, -2.0), c + Vec2::new(2.0, -2.0)],
        Stroke::new(0.8, Tokens::ICON),
    );
    let tip = c + Vec2::new(1.5, 1.0);
    ui.painter().add(epaint::PathShape::convex_polygon(
        vec![
            tip,
            tip + Vec2::new(0.0, 8.0),
            tip + Vec2::new(2.2, 6.0),
            tip + Vec2::new(5.5, 6.0),
        ],
        Tokens::ICON,
        Stroke::NONE,
    ));
    let key = if pick.is_drag() {
        "coords.set_drag_tip"
    } else {
        "coords.set_click_tip"
    };
    if resp.on_hover_text(tr(key)).clicked() {
        *out = Some(pick);
    }
}

/// A labelled length field in the document unit; true when it changed.
fn length_row(ui: &mut Ui, app_units: crate::app::Units, key: &str, mm: &mut f64) -> bool {
    let u = app_units;
    let mut v = u.from_mm(*mm);
    let mut changed = false;
    ui.label(tr(key));
    if ui
        .add_sized(
            [FIELD_W, 20.0],
            NumField::new(&mut v)
                .range(0.0..=100_000.0)
                .max_decimals(3)
                .suffix(format!(" {}", u.short()))
                .unit_mm(u.mm()),
        )
        .changed()
    {
        *mm = u.to_mm(v);
        changed = true;
    }
    changed
}

/// x and y of a page point, counted from the ruler origin `o`.
fn point_fields(ui: &mut Ui, u: crate::app::Units, o: Point, p: &mut Point) -> bool {
    let mut changed = false;
    for (axis, label) in [(0, "x:"), (1, "y:")] {
        ui.label(label);
        let mm = if axis == 0 { p.x - o.x } else { p.y - o.y };
        let mut v = u.from_mm(mm);
        if ui
            .add_sized(
                [FIELD_W, 20.0],
                NumField::new(&mut v)
                    .range(-1_000_000.0..=1_000_000.0)
                    .max_decimals(3)
                    .suffix(format!(" {}", u.short()))
                    .unit_mm(u.mm()),
            )
            .changed()
        {
            let mm = u.to_mm(v);
            if axis == 0 {
                p.x = mm + o.x;
            } else {
                p.y = mm + o.y;
            }
            changed = true;
        }
    }
    changed
}

fn angle_field(ui: &mut Ui, deg: &mut f64) -> bool {
    ui.add_sized(
        [FIELD_W, 20.0],
        NumField::new(deg)
            .range(-360.0..=360.0)
            .max_decimals(1)
            .suffix(" °"),
    )
    .changed()
}

/// The 3 x 3 origin point selector; the chosen point when clicked.
fn origin_grid(ui: &mut Ui, origin: usize) -> Option<usize> {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(34.0, 34.0), Sense::click());
    let resp = resp.on_hover_text(tr("coords.origin_tip"));
    let frame = rect.shrink(6.0);
    ui.painter().rect_stroke(
        frame,
        0.0,
        Stroke::new(1.0, Tokens::BORDER),
        egui::StrokeKind::Middle,
    );
    let mut out = None;
    for i in 0..9usize {
        let (col, row) = ((i % 3) as f32, (i / 3) as f32);
        let p = frame.min + Vec2::new(col * frame.width() / 2.0, row * frame.height() / 2.0);
        let r = Rect::from_center_size(p, Vec2::splat(6.0));
        if i == origin {
            ui.painter().rect_filled(r, 0.0, Tokens::SELECTION);
        } else {
            ui.painter().rect_filled(r, 0.0, Color32::WHITE);
            ui.painter().rect_stroke(
                r,
                0.0,
                Stroke::new(1.0, Tokens::ICON),
                egui::StrokeKind::Inside,
            );
        }
        if resp.clicked() {
            if let Some(q) = resp.interact_pointer_pos() {
                if r.expand(3.0).contains(q) {
                    out = Some(i);
                }
            }
        }
    }
    out
}

pub fn coordinates(app: &mut App, ui: &mut Ui) {
    if let Ok(page) = app.doc().page(app.page) {
        let r = tracedraw_core::geometry::Rect::new(0.0, 0.0, page.size.width, page.size.height);
        app.coords.place_on(r);
    }
    app.coords_follow_selection();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        kind_flyout(app, ui, [CoordKind::Rectangle, CoordKind::Square]);
        kind_flyout(app, ui, [CoordKind::Ellipse, CoordKind::Circle]);
        kind_flyout(app, ui, [CoordKind::Polygon, CoordKind::RegularPolygon]);
        kind_single(app, ui, CoordKind::Star);
        kind_single(app, ui, CoordKind::ComplexStar);
        kind_single(app, ui, CoordKind::Line);
        kind_single(app, ui, CoordKind::MultiPoint);
    });
    ui.label(
        egui::RichText::new(tr(app.coords.kind.key()))
            .color(Tokens::TEXT_DIM)
            .size(11.0),
    );
    let u = app.units;
    let o = app.ruler_origin();
    let waiting = app.coord_pick;
    let mut pick = None;
    let mut c = app.coords.clone();
    let grid = |ui: &mut Ui, id: &str, add: &mut dyn FnMut(&mut Ui)| {
        egui::Grid::new(id)
            .num_columns(3)
            .spacing([6.0, 4.0])
            .show(ui, |ui| add(ui));
    };
    match c.kind {
        k if k.is_boxed() => {
            section(ui, "coords.position");
            ui.horizontal(|ui| {
                if let Some(i) = origin_grid(ui, c.origin) {
                    c.set_origin(i);
                }
                ui.vertical(|ui| {
                    grid(ui, "coords_at", &mut |ui| {
                        point_fields_grid(ui, u, o, &mut c.at);
                    });
                });
                pick_button(ui, waiting, CoordPick::Origin, &mut pick);
            });
            section(ui, "coords.object_size");
            grid(ui, "coords_size", &mut |ui| {
                if k.is_uniform() {
                    let key = if k == CoordKind::Square {
                        "coords.side"
                    } else {
                        "coords.diameter"
                    };
                    let mut w = c.w;
                    if length_row(ui, u, key, &mut w) {
                        c.set_width(w);
                    }
                    pick_button(ui, waiting, CoordPick::Width, &mut pick);
                    ui.end_row();
                } else {
                    let (wk, hk) = if k == CoordKind::Rectangle {
                        ("coords.width", "coords.height")
                    } else {
                        ("coords.diameter_h", "coords.diameter_v")
                    };
                    let mut w = c.w;
                    if length_row(ui, u, wk, &mut w) {
                        c.set_width(w);
                    }
                    pick_button(ui, waiting, CoordPick::Width, &mut pick);
                    ui.end_row();
                    let mut h = c.h;
                    if length_row(ui, u, hk, &mut h) {
                        c.set_height(h);
                    }
                    pick_button(ui, waiting, CoordPick::Height, &mut pick);
                    ui.end_row();
                }
            });
            if !k.is_uniform() {
                ui.checkbox(&mut c.proportional, tr("coords.proportional"));
            }
            section(ui, "coords.angle");
            ui.horizontal(|ui| {
                angle_field(ui, &mut c.angle);
                pick_button(ui, waiting, CoordPick::Angle, &mut pick);
            });
            if !k.is_uniform() {
                section(ui, "coords.bounding_box");
                match c.bounding_box() {
                    Some(b) => {
                        let (mut ll, mut ur) = (Point::new(b.x0, b.y0), Point::new(b.x1, b.y1));
                        let (ll0, ur0) = (ll, ur);
                        ui.label(tr("coords.lower_left"));
                        ui.horizontal(|ui| {
                            point_fields(ui, u, o, &mut ll);
                            pick_button(ui, waiting, CoordPick::LowerLeft, &mut pick);
                        });
                        ui.label(tr("coords.upper_right"));
                        ui.horizontal(|ui| {
                            point_fields(ui, u, o, &mut ur);
                            pick_button(ui, waiting, CoordPick::UpperRight, &mut pick);
                        });
                        if ll != ll0 || ur != ur0 {
                            c.set_bounding_box(ll, ur);
                        }
                    }
                    None => {
                        ui.label(
                            egui::RichText::new(tr("coords.bounding_box_rotated"))
                                .color(Tokens::TEXT_DIM)
                                .size(11.0),
                        );
                    }
                }
            }
        }
        k if k.is_polygon() => {
            section(ui, "coords.object_size");
            grid(ui, "coords_poly", &mut |ui| {
                ui.label(tr("toolbar.points_sides"));
                let min = if k == CoordKind::ComplexStar {
                    COMPLEX_STAR_MIN_POINTS
                } else {
                    3
                };
                ui.add_sized(
                    [FIELD_W, 20.0],
                    NumField::new(&mut c.points).range(min..=500),
                );
                ui.end_row();
                match k {
                    CoordKind::Star => {
                        ui.label(tr("toolbar.sharpness"));
                        let mut pct = (c.sharpness * 100.0).round();
                        if ui
                            .add_sized([FIELD_W, 20.0], NumField::new(&mut pct).range(1.0..=99.0))
                            .changed()
                        {
                            c.sharpness = (pct / 100.0).clamp(0.01, 0.99);
                        }
                        ui.end_row();
                    }
                    CoordKind::ComplexStar => {
                        ui.label(tr("toolbar.sharpness"));
                        let max = complex_star_max_sharpness(c.points);
                        ui.add_sized(
                            [FIELD_W, 20.0],
                            NumField::new(&mut c.complex_sharpness).range(1..=max),
                        );
                        c.complex_sharpness = c.complex_sharpness.clamp(1, max);
                        ui.end_row();
                    }
                    CoordKind::RegularPolygon => {
                        let mut side = c.side_length();
                        if length_row(ui, u, "coords.side_length", &mut side) && side > 0.0 {
                            c.set_side_length(side);
                        }
                        pick_button(ui, waiting, CoordPick::SideLength, &mut pick);
                        ui.end_row();
                    }
                    _ => {}
                }
            });
            section(ui, "coords.angle");
            ui.horizontal(|ui| {
                angle_field(ui, &mut c.angle);
                pick_button(ui, waiting, CoordPick::Angle, &mut pick);
            });
            section(ui, "coords.bounding_circle");
            ui.horizontal(|ui| {
                point_fields(ui, u, o, &mut c.at);
                pick_button(ui, waiting, CoordPick::Origin, &mut pick);
            });
            grid(ui, "coords_circle", &mut |ui| {
                if k.is_uniform() {
                    let mut w = c.w;
                    if length_row(ui, u, "coords.diameter", &mut w) {
                        c.set_width(w);
                    }
                    pick_button(ui, waiting, CoordPick::Width, &mut pick);
                    ui.end_row();
                } else {
                    let mut w = c.w;
                    if length_row(ui, u, "coords.diameter_h", &mut w) {
                        c.set_width(w);
                    }
                    pick_button(ui, waiting, CoordPick::Width, &mut pick);
                    ui.end_row();
                    let mut h = c.h;
                    if length_row(ui, u, "coords.diameter_v", &mut h) {
                        c.set_height(h);
                    }
                    pick_button(ui, waiting, CoordPick::Height, &mut pick);
                    ui.end_row();
                }
            });
            if !k.is_uniform() {
                ui.checkbox(&mut c.proportional, tr("coords.proportional"));
            }
        }
        CoordKind::Line => {
            section(ui, "coords.points");
            ui.label(tr("coords.start_point"));
            ui.horizontal(|ui| {
                point_fields(ui, u, o, &mut c.start);
                pick_button(ui, waiting, CoordPick::Start, &mut pick);
            });
            ui.label(tr("coords.end_point"));
            ui.horizontal(|ui| {
                point_fields(ui, u, o, &mut c.end);
                pick_button(ui, waiting, CoordPick::End, &mut pick);
            });
            section(ui, "coords.object_size");
            grid(ui, "coords_line", &mut |ui| {
                let mut l = c.line_length();
                if length_row(ui, u, "coords.line_length", &mut l) {
                    c.set_line_length(l);
                }
                pick_button(ui, waiting, CoordPick::LineLength, &mut pick);
                ui.end_row();
                ui.label(tr("coords.angle"));
                let mut a = c.line_angle();
                if angle_field(ui, &mut a) {
                    c.set_line_angle(a);
                }
                pick_button(ui, waiting, CoordPick::Angle, &mut pick);
                ui.end_row();
            });
        }
        _ => multipoint(ui, &mut c, u, o, waiting, &mut pick),
    }
    app.coords = c;
    if let Some(p) = pick {
        app.coord_pick = if app.coord_pick == Some(p) {
            None
        } else {
            Some(p)
        };
        app.coord_drag = None;
    }
    // The preview: the object, its origin point and a pick's drag.
    if let Some(path) = app.coords.preview() {
        app.docker_preview.push(path);
    }
    app.docker_preview_point = app.coords.marked_point();
    if let (Some(a), Some(b)) = (app.coord_drag, app.pointer_page) {
        let mut p = BezPath::new();
        p.move_to(a);
        p.line_to(b);
        app.docker_preview.push(p);
    }
    if let Some(p) = app.coord_pick {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(tr(if p.is_drag() {
                "coords.drag_hint"
            } else {
                "coords.click_hint"
            }))
            .color(Tokens::SELECTION)
            .size(11.0),
        );
    }
    ui.add_space(10.0);
    let can = app.coords.shape_kind().is_some();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(can, egui::Button::new(tr("coords.create")))
            .clicked()
        {
            app.create_from_coords();
        }
        if ui
            .add_enabled(
                can && app.coords_target().is_some(),
                egui::Button::new(tr("coords.replace")),
            )
            .clicked()
        {
            app.replace_from_coords();
        }
    });
}

/// x and y as two grid rows.
fn point_fields_grid(ui: &mut Ui, u: crate::app::Units, o: Point, p: &mut Point) -> bool {
    let mut changed = false;
    for axis in 0..2 {
        ui.label(if axis == 0 { "x:" } else { "y:" });
        let mm = if axis == 0 { p.x - o.x } else { p.y - o.y };
        let mut v = u.from_mm(mm);
        if ui
            .add_sized(
                [FIELD_W, 20.0],
                NumField::new(&mut v)
                    .range(-1_000_000.0..=1_000_000.0)
                    .max_decimals(3)
                    .suffix(format!(" {}", u.short()))
                    .unit_mm(u.mm()),
            )
            .changed()
        {
            let mm = u.to_mm(v);
            if axis == 0 {
                p.x = mm + o.x;
            } else {
                p.y = mm + o.y;
            }
            changed = true;
        }
        ui.end_row();
    }
    changed
}

/// The multipoint curve's points: a list (click chooses, double-click
/// waits for a click on the drawing; with the list focused Insert adds a
/// point and Delete removes the chosen one), the chosen point's x and y,
/// Add point, Delete point, Set point interactively, Auto-close/open.
fn multipoint(
    ui: &mut Ui,
    c: &mut crate::coords::CoordsState,
    u: crate::app::Units,
    o: Point,
    waiting: Option<CoordPick>,
    pick: &mut Option<CoordPick>,
) {
    section(ui, "coords.points");
    let row_h = 20.0;
    let rows = c.curve.len().max(1);
    let height = (rows as f32 * row_h).min(8.0 * row_h);
    let (area, area_resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().min(260.0), height + 22.0),
        Sense::click(),
    );
    let id = egui::Id::new("coords_points_list");
    let focus = ui.interact(area, id, Sense::click());
    if focus.clicked() || area_resp.clicked() {
        focus.request_focus();
    }
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, Color32::WHITE);
    painter.rect_stroke(
        area.shrink(0.5),
        0.0,
        Stroke::new(
            1.0,
            if focus.has_focus() {
                Tokens::SELECTION
            } else {
                Tokens::BORDER
            },
        ),
        egui::StrokeKind::Middle,
    );
    let font = egui::FontId::proportional(12.0);
    let head = area.min + Vec2::new(6.0, 4.0);
    for (x, t) in [(0.0, "#"), (40.0, "X"), (140.0, "Y")] {
        painter.text(
            head + Vec2::new(x, 0.0),
            egui::Align2::LEFT_TOP,
            t,
            font.clone(),
            Tokens::TEXT_DIM,
        );
    }
    let top = area.min.y + 22.0;
    let mut clicked_row = None;
    let mut double_row = None;
    if let Some(pos) = focus.interact_pointer_pos() {
        if pos.y >= top {
            let i = ((pos.y - top) / row_h) as usize;
            if i < c.curve.len() {
                if focus.double_clicked() {
                    double_row = Some(i);
                } else if focus.clicked() {
                    clicked_row = Some(i);
                }
            }
        }
    }
    for (i, p) in c.curve.iter().enumerate().take(8) {
        let y = top + i as f32 * row_h;
        let row = Rect::from_min_size(
            Pos2::new(area.min.x + 1.0, y),
            Vec2::new(area.width() - 2.0, row_h),
        );
        if c.current == Some(i) {
            painter.rect_filled(row, 0.0, Color32::from_rgb(0xCC, 0xE8, 0xFF));
        }
        let r = Point::new(u.from_mm(p.x - o.x), u.from_mm(p.y - o.y));
        for (x, t) in [
            (0.0, (i + 1).to_string()),
            (
                40.0,
                crate::ui::field::format_number(r.x, false, 0, Some(3)),
            ),
            (
                140.0,
                crate::ui::field::format_number(r.y, false, 0, Some(3)),
            ),
        ] {
            painter.text(
                Pos2::new(area.min.x + 6.0 + x, y + 3.0),
                egui::Align2::LEFT_TOP,
                t,
                font.clone(),
                Tokens::TEXT,
            );
        }
    }
    if let Some(i) = clicked_row {
        c.current = Some(i);
    }
    if let Some(i) = double_row {
        c.current = Some(i);
        *pick = Some(CoordPick::Point);
    }
    if focus.has_focus() {
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Insert)) {
            c.add_point();
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete)) {
            c.delete_point();
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
            if let Some(i) = c.current {
                c.current = Some((i + 1).min(c.curve.len().saturating_sub(1)));
            }
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
            if let Some(i) = c.current {
                c.current = Some(i.saturating_sub(1));
            }
        }
    }
    if let Some(i) = c.current.filter(|i| *i < c.curve.len()) {
        ui.horizontal(|ui| {
            let mut p = c.curve[i];
            if point_fields(ui, u, o, &mut p) {
                c.curve[i] = p;
            }
        });
    }
    ui.horizontal(|ui| {
        if ui
            .button("+")
            .on_hover_text(tr("coords.add_point"))
            .clicked()
        {
            c.add_point();
        }
        if ui
            .add_enabled(c.current.is_some(), egui::Button::new("-"))
            .on_hover_text(tr("coords.delete_point"))
            .clicked()
        {
            c.delete_point();
        }
        pick_button(ui, waiting, CoordPick::Point, pick);
        if ui
            .selectable_label(c.closed, tr("coords.auto_close"))
            .clicked()
        {
            c.closed = !c.closed;
        }
    });
}
