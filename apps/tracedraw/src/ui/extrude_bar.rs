//! The Extrude tool's property bar, the controls it shares with the
//! Extrude panel, and the handles of the selected extrusions. Controls:
//! the kind of body; the vector (parallel bodies) or the depth, the
//! vanishing point and what it is locked to; the colour (the object's
//! fill, solid, or a shade between two colours); the light's angle,
//! intensity and elevation, two further lights, the turn in space and the
//! bevel. They show the selected extrusion and edit the selected
//! extrusions live.

use crate::app::App;
use crate::extrude_tool::{kind_key, ExtrudeColor};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::field::NumField;
use crate::ui::fill_tool_bars::color_pick;
use crate::ui::propbar::sep;
use crate::ui::tips::RichTip;
use egui::{Color32, Painter, Stroke, Ui};
use tracedraw_core::extrude::ExtrudeKind;

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// A length in the document's units; true when it was applied.
fn length(ui: &mut Ui, app: &App, mm: &mut f64, tip: &str) -> bool {
    let mut v = app.units.world_from_mm(*mm);
    let r = ui
        .add(
            NumField::new(&mut v)
                .fixed_decimals(app.settings.precision.min(10) as usize)
                .unit_mm(app.units.world_to_mm(1.0))
                .suffix(format!(" {}", app.units.short()))
                .width(76.0),
        )
        .rich_tip(tip);
    if r.changed() {
        *mm = app.units.world_to_mm(v);
        return true;
    }
    false
}

/// The controls, each group in a row of its own (a bar lays the rows
/// side by side, the panel one under the other).
pub fn extrude_controls(app: &mut App, ui: &mut Ui) {
    app.sync_extrude_settings();
    let palette = app.palette.clone();
    let mut s = app.extrude;
    let mut changed = false;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("extrude_kind")
            .selected_text(tr(kind_key(s.kind)))
            .width(110.0)
            .show_ui(ui, |ui| {
                for k in ExtrudeKind::ALL {
                    if ui.selectable_label(s.kind == k, tr(kind_key(k))).clicked() && s.kind != k {
                        s.kind = k;
                        changed = true;
                    }
                }
            })
            .response
            .rich_tip(tr("extrude.kind"));
    });
    ui.horizontal(|ui| {
        if s.kind.parallel() {
            dim(ui, tr("extrude.vector"));
            let (tx, ty) = (
                crate::ui::tips::pair("extrude.vector", "extrude.vector_x_desc"),
                crate::ui::tips::pair("extrude.vector", "extrude.vector_y_desc"),
            );
            changed |= length(ui, app, &mut s.vector.x, &tx);
            changed |= length(ui, app, &mut s.vector.y, &ty);
        } else {
            dim(ui, tr("docker.depth"));
            changed |= ui
                .add(
                    NumField::new(&mut s.depth)
                        .range(1.0..=99.0)
                        .max_decimals(0)
                        .width(44.0),
                )
                .rich_tip(crate::ui::tips::pair("docker.depth", "extrude.depth_tip"))
                .changed();
            dim(ui, tr("docker.vanishing_point"));
            let (tx, ty) = (
                crate::ui::tips::pair("docker.vanishing_point", "extrude.vanishing_x_desc"),
                crate::ui::tips::pair("docker.vanishing_point", "extrude.vanishing_y_desc"),
            );
            changed |= length(ui, app, &mut s.vanishing.x, &tx);
            changed |= length(ui, app, &mut s.vanishing.y, &ty);
            egui::ComboBox::from_id_salt("extrude_locked")
                .selected_text(tr(if s.on_object {
                    "extrude.locked_object"
                } else {
                    "extrude.locked_page"
                }))
                .width(110.0)
                .show_ui(ui, |ui| {
                    for (on, key) in [
                        (true, "extrude.locked_object"),
                        (false, "extrude.locked_page"),
                    ] {
                        if ui.selectable_label(s.on_object == on, tr(key)).clicked()
                            && s.on_object != on
                        {
                            s.on_object = on;
                            changed = true;
                        }
                    }
                })
                .response
                .rich_tip(crate::ui::tips::pair(
                    "extrude.locked_tip",
                    "extrude.locked_desc",
                ));
        }
    });
    ui.horizontal(|ui| {
        dim(ui, tr("extrude.color"));
        egui::ComboBox::from_id_salt("extrude_color")
            .selected_text(tr(s.color.key()))
            .width(96.0)
            .show_ui(ui, |ui| {
                for c in ExtrudeColor::ALL {
                    if ui.selectable_label(s.color == c, tr(c.key())).clicked() && s.color != c {
                        s.color = c;
                        changed = true;
                    }
                }
            })
            .response
            .rich_tip(crate::ui::tips::keyed("extrude.color"));
        match s.color {
            ExtrudeColor::Object => {}
            ExtrudeColor::Solid => {
                changed |= color_pick(
                    ui,
                    &palette,
                    "extrude_solid",
                    &mut s.solid,
                    &tr("extrude.color_solid"),
                    true,
                );
            }
            ExtrudeColor::Shade => {
                changed |= color_pick(
                    ui,
                    &palette,
                    "extrude_from",
                    &mut s.shade_from,
                    &tr("docker.from"),
                    true,
                );
                changed |= color_pick(
                    ui,
                    &palette,
                    "extrude_to",
                    &mut s.shade_to,
                    &tr("docker.to"),
                    true,
                );
            }
        }
    });
    ui.horizontal(|ui| {
        dim(ui, tr("docker.light_angle"));
        changed |= ui
            .add(
                NumField::new(&mut s.light_angle)
                    .range(0.0..=360.0)
                    .max_decimals(1)
                    .suffix("\u{b0}")
                    .width(52.0),
            )
            .rich_tip(crate::ui::tips::pair(
                "docker.light_angle",
                "extrude.light_angle_desc",
            ))
            .changed();
        dim(ui, tr("docker.light_intensity"));
        changed |= ui
            .add(
                NumField::new(&mut s.light_intensity)
                    .range(0.0..=100.0)
                    .max_decimals(0)
                    .width(40.0),
            )
            .rich_tip(crate::ui::tips::pair(
                "docker.light_intensity",
                "extrude.light_intensity_desc",
            ))
            .changed();
    });
    if changed {
        app.extrude = s;
        app.apply_extrude_settings();
    }
}

/// The turn in space, the first light's elevation, the further lights and
/// the bevel (the panel shows them; the bar in a menu).
pub fn extrude_space(app: &mut App, ui: &mut Ui) {
    app.sync_extrude_settings();
    let mut s = app.extrude;
    let mut changed = false;
    ui.horizontal(|ui| {
        dim(ui, tr("extrude.elevation"));
        changed |= degrees(
            ui,
            &mut s.elevation,
            0.0..=90.0,
            &tr("extrude.elevation_tip"),
        );
    });
    ui.horizontal(|ui| {
        dim(ui, tr("extrude.rotation"));
        for (i, axis) in ["x", "y", "z"].into_iter().enumerate() {
            changed |= degrees(
                ui,
                &mut s.rotation[i],
                -360.0..=360.0,
                &format!("{} ({axis})", tr("extrude.rotation_tip")),
            );
        }
        if ui
            .add_enabled(
                s.rotation != [0.0; 3],
                egui::Button::new(tr("docker.reset")),
            )
            .clicked()
        {
            s.rotation = [0.0; 3];
            changed = true;
        }
    });
    for i in 0..s.lights.len() {
        ui.horizontal(|ui| {
            let (on, light) = &mut s.lights[i];
            changed |= ui
                .checkbox(
                    on,
                    crate::i18n::trf("extrude.light_n", &[("n", &(i + 2).to_string())]),
                )
                .changed();
            ui.add_enabled_ui(*on, |ui| {
                changed |= degrees(
                    ui,
                    &mut light.angle,
                    0.0..=360.0,
                    &crate::ui::tips::pair("docker.light_angle", "extrude.light_angle_desc"),
                );
                changed |= degrees(
                    ui,
                    &mut light.elevation,
                    0.0..=90.0,
                    &tr("extrude.elevation"),
                );
                changed |= ui
                    .add(
                        NumField::new(&mut light.intensity)
                            .range(0.0..=100.0)
                            .max_decimals(0)
                            .width(40.0),
                    )
                    .rich_tip(crate::ui::tips::pair(
                        "docker.light_intensity",
                        "extrude.light_intensity_desc",
                    ))
                    .changed();
            });
        });
    }
    ui.horizontal(|ui| {
        dim(ui, tr("docker.bevel"));
        changed |= length(
            ui,
            app,
            &mut s.bevel,
            &crate::ui::tips::pair("docker.bevel", "extrude.bevel_tip"),
        );
    });
    if changed {
        app.extrude = s;
        app.apply_extrude_settings();
    }
}

/// An angle field in degrees; true when it changed.
fn degrees(ui: &mut Ui, v: &mut f64, range: std::ops::RangeInclusive<f64>, tip: &str) -> bool {
    ui.add(
        NumField::new(v)
            .range(range)
            .max_decimals(1)
            .suffix("\u{b0}")
            .width(52.0),
    )
    .rich_tip(tip)
    .changed()
}

/// Apply (for selected objects with no extrusion), Clear Extrude and
/// Break Apart.
pub fn extrude_actions(app: &mut App, ui: &mut Ui) {
    let extruded = app.selected_extrusion().is_some();
    let bare = app.selected_shapes().iter().any(|s| {
        !s.effects
            .iter()
            .any(|e| matches!(e, tracedraw_core::live::Effect::Extrude { .. }))
            && !matches!(
                s.kind,
                tracedraw_core::ShapeKind::Bitmap { .. } | tracedraw_core::ShapeKind::Group { .. }
            )
    });
    ui.horizontal(|ui| {
        if bare
            && ui
                .button(tr("docker.apply"))
                .rich_tip(crate::ui::tips::pair("docker.apply", "extrude.apply_desc"))
                .clicked()
        {
            app.add_extrusion();
        }
        if ui
            .add_enabled(extruded, egui::Button::new(tr("extrude.clear")))
            .rich_tip(crate::ui::tips::keyed("extrude.clear"))
            .clicked()
        {
            app.clear_extrusion();
        }
        if ui
            .add_enabled(extruded, egui::Button::new(tr("docker.break_apart")))
            .rich_tip(crate::ui::tips::pair(
                "docker.break_apart",
                "toolbar.effect_break_apart_desc",
            ))
            .clicked()
        {
            app.break_effect_groups();
        }
    });
}

/// The bar.
pub fn extrude_bar(app: &mut App, ui: &mut Ui) {
    extrude_controls(app, ui);
    ui.menu_button(tr("extrude.space_menu"), |ui| {
        ui.vertical(|ui| extrude_space(app, ui));
    })
    .response
    .rich_tip(tr("extrude.space_menu_tip"));
    sep(ui);
    extrude_actions(app, ui);
    sep(ui);
    dim(ui, tr("extrude.hint"));
}

/// Each selected extrusion's handle: a line from the object's middle to
/// its vanishing point or its copy's middle, and a cross there.
pub fn draw_handles(app: &App, painter: &Painter) {
    if app.tool != crate::tools::Tool::Extrude {
        return;
    }
    let blue = Color32::from_rgb(40, 110, 220);
    for (_, h, from) in app.extrusion_handles() {
        let (h, from) = (app.view.to_screen(h), app.view.to_screen(from));
        painter.add(egui::Shape::dashed_line(
            &[from, h],
            Stroke::new(1.0, blue),
            4.0,
            3.0,
        ));
        let r = 5.0;
        painter.rect_filled(
            egui::Rect::from_center_size(h, egui::vec2(2.0 * r, 2.0 * r)),
            0.0,
            Color32::WHITE,
        );
        painter.rect_stroke(
            egui::Rect::from_center_size(h, egui::vec2(2.0 * r, 2.0 * r)),
            0.0,
            Stroke::new(1.0, blue),
            egui::StrokeKind::Middle,
        );
        painter.line_segment(
            [h + egui::vec2(-r, -r), h + egui::vec2(r, r)],
            Stroke::new(1.0, blue),
        );
        painter.line_segment(
            [h + egui::vec2(-r, r), h + egui::vec2(r, -r)],
            Stroke::new(1.0, blue),
        );
    }
}

/// The Extrude docker's pages.
pub const EXTRUDE_PAGES: [&str; 5] = [
    "extrude.page_camera",
    "extrude.page_rotation",
    "extrude.page_lighting",
    "extrude.page_color",
    "extrude.page_bevel",
];

/// A page button's picture: the body, a turn, a light, colours, a bevel.
fn page_picture(p: &Painter, r: egui::Rect, page: usize, ink: Color32) {
    let s = Stroke::new(1.2, ink);
    let c = r.center();
    match page {
        0 => {
            let front = egui::Rect::from_min_size(
                r.left_top() + egui::vec2(0.0, 5.0),
                egui::vec2(10.0, 10.0),
            );
            let back = front.translate(egui::vec2(5.0, -5.0));
            p.rect_stroke(front, 0.0, s, egui::StrokeKind::Inside);
            p.rect_stroke(back, 0.0, s, egui::StrokeKind::Inside);
            for (a, b) in [
                (front.left_top(), back.left_top()),
                (front.right_top(), back.right_top()),
                (front.right_bottom(), back.right_bottom()),
            ] {
                p.line_segment([a, b], s);
            }
        }
        1 => {
            let pts: Vec<egui::Pos2> = (0..=14)
                .map(|k| {
                    let a = std::f32::consts::PI * (0.2 + 1.5 * k as f32 / 14.0);
                    c + egui::vec2(a.cos(), a.sin()) * 7.0
                })
                .collect();
            let end = *pts.last().unwrap_or(&c);
            p.add(egui::Shape::line(pts, s));
            p.circle_filled(end, 2.0, ink);
        }
        2 => {
            p.circle_stroke(c - egui::vec2(0.0, 2.0), 5.0, s);
            p.line_segment([c + egui::vec2(-2.5, 5.0), c + egui::vec2(2.5, 5.0)], s);
            p.line_segment([c + egui::vec2(-2.0, 7.5), c + egui::vec2(2.0, 7.5)], s);
        }
        3 => {
            for (i, col) in [
                Color32::from_rgb(0xE0, 0x40, 0x30),
                Color32::from_rgb(0x30, 0x90, 0xE0),
                Color32::from_rgb(0xF0, 0xB0, 0x20),
                Color32::from_rgb(0x40, 0xB0, 0x60),
            ]
            .into_iter()
            .enumerate()
            {
                let o = egui::vec2((i % 2) as f32 * 7.0 - 3.5, (i / 2) as f32 * 7.0 - 3.5);
                p.rect_filled(
                    egui::Rect::from_center_size(c + o, egui::vec2(5.5, 5.5)),
                    1.0,
                    col,
                );
            }
        }
        _ => {
            let outer = egui::Rect::from_center_size(c, egui::vec2(14.0, 14.0));
            let inner = outer.shrink(4.0);
            p.rect_stroke(outer, 0.0, s, egui::StrokeKind::Inside);
            p.rect_stroke(inner, 0.0, s, egui::StrokeKind::Inside);
            for (a, b) in [
                (outer.left_top(), inner.left_top()),
                (outer.right_top(), inner.right_top()),
                (outer.left_bottom(), inner.left_bottom()),
                (outer.right_bottom(), inner.right_bottom()),
            ] {
                p.line_segment([a, b], s);
            }
        }
    }
}

/// The picture of an extrusion type: the object (bold), its copy and the
/// lines joining them, arrows on the far ones.
fn type_picture(ui: &mut Ui, kind: ExtrudeKind) {
    let w = ui.available_width().max(120.0);
    let (r, _) = ui.allocate_exact_size(egui::vec2(w, 100.0), egui::Sense::hover());
    let p = ui.painter_at(r);
    p.rect_filled(r, 0.0, Color32::WHITE);
    p.rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Tokens::border()),
        egui::StrokeKind::Inside,
    );
    let ink = Color32::from_gray(0x10);
    let side = 44.0;
    let obj =
        egui::Rect::from_center_size(r.center() + egui::vec2(-22.0, 12.0), egui::vec2(side, side));
    let offset = egui::vec2(48.0, -30.0);
    let scale = match kind {
        ExtrudeKind::SmallBack | ExtrudeKind::SmallFront => 0.55,
        ExtrudeKind::BigBack | ExtrudeKind::BigFront => 1.45,
        _ => 1.0,
    };
    let copy = egui::Rect::from_center_size(obj.center() + offset, obj.size() * scale);
    let front = kind.front();
    let (near, far) = if front { (copy, obj) } else { (obj, copy) };
    p.rect_stroke(far, 0.0, Stroke::new(1.0, ink), egui::StrokeKind::Inside);
    p.rect_stroke(near, 0.0, Stroke::new(3.0, ink), egui::StrokeKind::Inside);
    let arrow = |from: egui::Pos2, to: egui::Pos2| {
        p.line_segment([from, to], Stroke::new(1.0, ink));
        let d = (to - from).normalized();
        let n = egui::vec2(-d.y, d.x);
        p.add(egui::Shape::convex_polygon(
            vec![to, to - d * 8.0 + n * 3.5, to - d * 8.0 - n * 3.5],
            ink,
            Stroke::NONE,
        ));
    };
    let (a, b) = if front { (copy, obj) } else { (obj, copy) };
    arrow(a.left_top(), b.left_top());
    arrow(a.right_bottom(), b.right_bottom());
    p.line_segment([a.left_bottom(), a.right_top()], Stroke::new(1.0, ink));
}

/// The Extrude docker: page buttons, then the page; Apply, Clear Extrude
/// and Break Apart under it.
pub fn extrude_docker(app: &mut App, ui: &mut Ui) {
    app.sync_extrude_settings();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (i, key) in EXTRUDE_PAGES.iter().enumerate() {
            let (r, resp) = ui.allocate_exact_size(egui::vec2(28.0, 26.0), egui::Sense::click());
            let resp = resp.rich_tip(tr(key));
            let on = app.extrude_page == i;
            let p = ui.painter();
            if on {
                p.rect_filled(r, 2.0, Tokens::tool_hover());
                p.rect_stroke(
                    r,
                    2.0,
                    Stroke::new(1.0, Tokens::ACCENT),
                    egui::StrokeKind::Inside,
                );
            }
            page_picture(p, r.shrink(5.0), i, Tokens::text());
            if resp.clicked() {
                app.extrude_page = i;
            }
        }
    });
    let page = app.extrude_page.min(EXTRUDE_PAGES.len() - 1);
    ui.label(egui::RichText::new(tr(EXTRUDE_PAGES[page])).strong());
    let mut s = app.extrude;
    let mut changed = false;
    match page {
        0 => {
            ui.label(tr("extrude.type"));
            type_picture(ui, s.kind);
            egui::ComboBox::from_id_salt("extrude_docker_kind")
                .selected_text(tr(kind_key(s.kind)))
                .width(ui.available_width() - 8.0)
                .show_ui(ui, |ui| {
                    for k in ExtrudeKind::ALL {
                        if ui.selectable_label(s.kind == k, tr(kind_key(k))).clicked()
                            && s.kind != k
                        {
                            s.kind = k;
                            changed = true;
                        }
                    }
                })
                .response
                .rich_tip(crate::ui::tips::pair("extrude.kind", "extrude.kind_desc"));
            let vp = !s.kind.parallel();
            ui.label(tr("docker.vanishing_point"));
            ui.add_enabled_ui(vp, |ui| {
                egui::ComboBox::from_id_salt("extrude_docker_locked")
                    .selected_text(tr(if s.on_object {
                        "extrude.locked_object"
                    } else {
                        "extrude.locked_page"
                    }))
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        for (on, key) in [
                            (true, "extrude.locked_object"),
                            (false, "extrude.locked_page"),
                        ] {
                            if ui.selectable_label(s.on_object == on, tr(key)).clicked()
                                && s.on_object != on
                            {
                                s.on_object = on;
                                changed = true;
                            }
                        }
                    })
                    .response
                    .rich_tip(crate::ui::tips::pair(
                        "extrude.locked_tip",
                        "extrude.locked_desc",
                    ))
                    .rich_disabled_tip(crate::ui::tips::pair(
                        "extrude.locked_tip",
                        "extrude.locked_desc",
                    ));
            });
            ui.label(tr("docker.depth"));
            ui.add_enabled_ui(vp, |ui| {
                changed |= ui
                    .add(
                        NumField::new(&mut s.depth)
                            .range(1.0..=99.0)
                            .max_decimals(0)
                            .width(80.0),
                    )
                    .rich_tip(crate::ui::tips::pair("docker.depth", "extrude.depth_tip"))
                    .changed();
            });
            // X and Y: the vanishing point, from the page origin or the
            // object's centre; for parallel bodies the vector.
            let origin = if vp && app.extrude_measure_object {
                app.selection_bounds()
                    .map(|b| b.center())
                    .unwrap_or_default()
            } else {
                tracedraw_core::geometry::Point::ZERO
            };
            let (mut x, mut y) = if vp {
                (s.vanishing.x - origin.x, s.vanishing.y - origin.y)
            } else {
                (s.vector.x, s.vector.y)
            };
            ui.label("X:");
            let cx = length(ui, app, &mut x, "x");
            ui.label("Y:");
            let cy = length(ui, app, &mut y, "y");
            if cx || cy {
                if vp {
                    s.vanishing = tracedraw_core::geometry::Point::new(x + origin.x, y + origin.y);
                } else {
                    s.vector = tracedraw_core::geometry::Vec2::new(x, y);
                }
                changed = true;
            }
            ui.label(tr("extrude.measure_from"));
            ui.add_enabled_ui(vp, |ui| {
                ui.radio_value(
                    &mut app.extrude_measure_object,
                    false,
                    tr("extrude.measure_page"),
                )
                .rich_tip(crate::ui::tips::keyed("extrude.measure_page"));
                ui.radio_value(
                    &mut app.extrude_measure_object,
                    true,
                    tr("extrude.measure_object"),
                )
                .rich_tip(crate::ui::tips::keyed("extrude.measure_object"));
            });
        }
        1 => {
            for (i, axis) in ["X", "Y", "Z"].into_iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{axis}:"));
                    changed |= degrees(
                        ui,
                        &mut s.rotation[i],
                        -360.0..=360.0,
                        &tr("extrude.rotation_tip"),
                    );
                });
            }
            if ui
                .add_enabled(
                    s.rotation != [0.0; 3],
                    egui::Button::new(tr("docker.reset")),
                )
                .clicked()
            {
                s.rotation = [0.0; 3];
                changed = true;
            }
        }
        2 => {
            ui.horizontal(|ui| {
                ui.label(tr("docker.light_angle"));
                changed |= degrees(
                    ui,
                    &mut s.light_angle,
                    0.0..=360.0,
                    &crate::ui::tips::pair("docker.light_angle", "extrude.light_angle_desc"),
                );
            });
            ui.horizontal(|ui| {
                ui.label(tr("docker.light_intensity"));
                changed |= ui
                    .add(
                        NumField::new(&mut s.light_intensity)
                            .range(0.0..=100.0)
                            .max_decimals(0)
                            .width(52.0),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label(tr("extrude.elevation"));
                changed |= degrees(
                    ui,
                    &mut s.elevation,
                    0.0..=90.0,
                    &tr("extrude.elevation_tip"),
                );
            });
            for i in 0..s.lights.len() {
                let (on, light) = &mut s.lights[i];
                changed |= ui
                    .checkbox(
                        on,
                        crate::i18n::trf("extrude.light_n", &[("n", &(i + 2).to_string())]),
                    )
                    .changed();
                ui.add_enabled_ui(*on, |ui| {
                    ui.horizontal(|ui| {
                        changed |= degrees(
                            ui,
                            &mut light.angle,
                            0.0..=360.0,
                            &crate::ui::tips::pair(
                                "docker.light_angle",
                                "extrude.light_angle_desc",
                            ),
                        );
                        changed |= degrees(
                            ui,
                            &mut light.elevation,
                            0.0..=90.0,
                            &tr("extrude.elevation"),
                        );
                        changed |= ui
                            .add(
                                NumField::new(&mut light.intensity)
                                    .range(0.0..=100.0)
                                    .max_decimals(0)
                                    .width(40.0),
                            )
                            .rich_tip(crate::ui::tips::pair(
                                "docker.light_intensity",
                                "extrude.light_intensity_desc",
                            ))
                            .changed();
                    });
                });
            }
        }
        3 => {
            let palette = app.palette.clone();
            for c in ExtrudeColor::ALL {
                if ui.radio(s.color == c, tr(c.key())).clicked() && s.color != c {
                    s.color = c;
                    changed = true;
                }
            }
            match s.color {
                ExtrudeColor::Object => {}
                ExtrudeColor::Solid => {
                    changed |= color_pick(
                        ui,
                        &palette,
                        "extrude_docker_solid",
                        &mut s.solid,
                        &tr("extrude.color_solid"),
                        true,
                    );
                }
                ExtrudeColor::Shade => {
                    ui.horizontal(|ui| {
                        changed |= color_pick(
                            ui,
                            &palette,
                            "extrude_docker_from",
                            &mut s.shade_from,
                            &tr("docker.from"),
                            true,
                        );
                        changed |= color_pick(
                            ui,
                            &palette,
                            "extrude_docker_to",
                            &mut s.shade_to,
                            &tr("docker.to"),
                            true,
                        );
                    });
                }
            }
        }
        _ => {
            ui.horizontal(|ui| {
                ui.label(tr("docker.bevel"));
                changed |= length(
                    ui,
                    app,
                    &mut s.bevel,
                    &crate::ui::tips::pair("docker.bevel", "extrude.bevel_tip"),
                );
            });
        }
    }
    if changed {
        app.extrude = s;
        app.apply_extrude_settings();
    }
    ui.add_space(6.0);
    ui.separator();
    extrude_actions(app, ui);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{geometry::Rect, live::Effect, ShapeKind};

    #[test]
    fn the_bar_and_the_panel_draw_for_every_kind() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.tool = crate::tools::Tool::Extrude;
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("square");
        let frame = |app: &mut App, panel: bool| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600.0, 400.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                if panel {
                    ui.vertical(|ui| {
                        extrude_controls(app, ui);
                        extrude_space(app, ui);
                        extrude_actions(app, ui);
                    });
                } else {
                    ui.horizontal(|ui| extrude_bar(app, ui));
                }
                draw_handles(app, ui.painter());
            });
            out.textures_delta.clear();
        };
        for k in ExtrudeKind::ALL {
            for c in ExtrudeColor::ALL {
                app.extrude.kind = k;
                app.extrude.color = c;
                frame(&mut app, false);
            }
        }
        // A selected extrusion shows as the controls' values.
        app.select(vec![id]);
        app.extrude.kind = ExtrudeKind::BigBack;
        app.add_extrusion();
        app.extrude.kind = ExtrudeKind::FrontParallel;
        frame(&mut app, true);
        assert_eq!(app.extrude.kind, ExtrudeKind::BigBack);
        assert_eq!(app.extrusion_handles().len(), 1);
        // The turn, the lights and the bevel go to the effect and come
        // back from it.
        app.extrude.rotation = [10.0, 20.0, 30.0];
        app.extrude.elevation = 40.0;
        app.extrude.lights[1] = (true, crate::extrude_tool::NEW_LIGHT);
        app.extrude.bevel = 1.5;
        app.apply_extrude_settings();
        frame(&mut app, true);
        frame(&mut app, false);
        let Some(Effect::Extrude { space, bevel, .. }) = app
            .selected_shapes()
            .first()
            .and_then(|s| {
                s.effects
                    .iter()
                    .find(|e| matches!(e, Effect::Extrude { .. }))
            })
            .cloned()
        else {
            panic!("no extrusion");
        };
        assert_eq!(space.rotation, [10.0, 20.0, 30.0]);
        assert_eq!(space.lights.len(), 1);
        assert!((bevel - 1.5).abs() < 1e-9);
        let mut back = crate::extrude_tool::ExtrudeSettings::default();
        back.take(
            &Effect::Extrude {
                depth: Default::default(),
                vanishing: None,
                amount: 0.0,
                shade_from: None,
                shade_to: None,
                light_angle: 0.0,
                light_intensity: 0.0,
                bevel,
                kind: None,
                on_object: false,
                solid: None,
                space,
            },
            Default::default(),
            Default::default(),
        );
        assert!(back.lights[0].0);
        assert!(!back.lights[1].0);
        assert_eq!(back.elevation, 40.0);
    }
}
