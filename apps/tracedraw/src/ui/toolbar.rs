//! Standard toolbar and the context-sensitive property bar.

use crate::app::{App, Units};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::icons;
use egui::{Ui, Vec2};
use tracedraw_core::{
    document::{paper, ShapeKind},
    geometry::{Affine, Rect, Size},
    Command, Fill,
};

fn tb_button(ui: &mut Ui, action: icons::Action, tip: &str, enabled: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(24.0, 22.0),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    if enabled && resp.hovered() {
        ui.painter().rect_filled(rect, 2.0, Tokens::TOOL_HOVER);
    }
    let color = if enabled {
        Tokens::ICON
    } else {
        Tokens::BORDER
    };
    icons::draw_action(
        ui.painter(),
        egui::Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
        action,
        color,
    );
    resp.on_hover_text(tip).clicked() && enabled
}

/// Toolbar toggle: drawn pressed while `on`; returns true when clicked.
fn tb_toggle(ui: &mut Ui, action: icons::Action, tip: &str, on: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(24.0, 22.0), egui::Sense::click());
    if on {
        ui.painter().rect_filled(rect, 2.0, Tokens::TOOL_ACTIVE);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 2.0, Tokens::TOOL_HOVER);
    }
    icons::draw_action(
        ui.painter(),
        egui::Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
        action,
        Tokens::ICON,
    );
    resp.on_hover_text(tip).clicked()
}

fn vsep(ui: &mut Ui) {
    ui.add(egui::Separator::default().vertical().spacing(6.0));
}

pub fn standard_toolbar(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        if tb_button(ui, icons::Action::New, &tr("toolbar.new"), true) {
            app.new_document();
        }
        if tb_button(ui, icons::Action::Open, &tr("toolbar.open"), true) {
            app.open_dialog();
        }
        if tb_button(ui, icons::Action::Save, &tr("toolbar.save"), true) {
            app.save(false);
        }
        if tb_button(ui, icons::Action::Print, &tr("toolbar.print"), true) {
            app.dialog = crate::ui::dialogs::Dialog::Print(Default::default());
        }
        vsep(ui);
        let has = !app.selection.is_empty();
        if tb_button(ui, icons::Action::Cut, &tr("toolbar.cut"), has) {
            app.cut();
        }
        if tb_button(ui, icons::Action::Copy, &tr("toolbar.copy"), has) {
            app.copy_with_system();
        }
        if tb_button(
            ui,
            icons::Action::Paste,
            &tr("toolbar.paste"),
            app.clipboard.is_some(),
        ) {
            app.paste_any();
        }
        vsep(ui);
        if tb_button(
            ui,
            icons::Action::Undo,
            &tr("toolbar.undo"),
            app.engine.undo_label().is_some(),
        ) {
            app.undo();
        }
        if tb_button(
            ui,
            icons::Action::Redo,
            &tr("toolbar.redo"),
            app.engine.redo_label().is_some(),
        ) {
            app.redo();
        }
        vsep(ui);
        if tb_button(ui, icons::Action::Import, &tr("toolbar.import"), true) {
            app.import();
        }
        if tb_button(ui, icons::Action::Export, &tr("toolbar.export"), true) {
            app.export();
        }
        if tb_button(
            ui,
            icons::Action::Pdf,
            &tr("menu.file.publish_to_pdf"),
            true,
        ) {
            app.export_pdf();
        }
        vsep(ui);
        // Zoom level combo.
        let mut pct = app.zoom_percent();
        let levels = [
            tr("toolbar.zoom_to_page"),
            tr("toolbar.zoom_to_fit"),
            tr("toolbar.zoom_to_selected"),
            "25%".to_string(),
            "50%".to_string(),
            "75%".to_string(),
            "100%".to_string(),
            "150%".to_string(),
            "200%".to_string(),
            "400%".to_string(),
            "800%".to_string(),
        ];
        egui::ComboBox::from_id_salt("zoom_levels")
            .selected_text(format!("{:.0}%", pct))
            .width(80.0)
            .show_ui(ui, |ui| {
                for (i, l) in levels.iter().enumerate() {
                    if ui.selectable_label(false, l).clicked() {
                        match i {
                            0 => app.zoom_to_page(),
                            1 => app.zoom_to_fit(),
                            2 => app.zoom_to_selection(),
                            _ => {
                                pct = l.trim_end_matches('%').parse().unwrap_or(100.0);
                                app.set_zoom_percent(pct);
                            }
                        }
                    }
                }
            });
        vsep(ui);
        if tb_button(
            ui,
            icons::Action::Fullscreen,
            &tr("toolbar.fullscreen_preview"),
            true,
        ) {
            app.fullscreen_preview = true;
        }
        if tb_toggle(
            ui,
            icons::Action::Snap,
            &tr("menu.view.snap_off"),
            app.snap.off,
        ) {
            app.snap.off = !app.snap.off;
        }
        ui.menu_button(tr("menu.view.snap_to"), |ui| {
            ui.checkbox(&mut app.snap.pixels, tr("menu.view.snap_pixels"));
            ui.checkbox(&mut app.snap.grid, tr("menu.view.snap_document_grid"));
            ui.checkbox(
                &mut app.snap.baseline_grid,
                tr("menu.view.snap_baseline_grid"),
            );
            ui.checkbox(&mut app.snap.guides, tr("menu.view.snap_guidelines"));
            ui.checkbox(&mut app.snap.objects, tr("menu.view.snap_objects"));
            ui.checkbox(&mut app.snap.page, tr("menu.view.snap_page"));
            ui.separator();
            ui.checkbox(&mut app.snap.off, tr("toolbar.snap_off_shortcut"));
        });
        if tb_button(ui, icons::Action::Options, &tr("toolbar.options"), true) {
            app.dialog = crate::ui::dialogs::Dialog::Options;
        }
        vsep(ui);
        if tb_button(
            ui,
            icons::Action::Welcome,
            &tr("menu.window.welcome_screen"),
            true,
        ) {
            app.show_welcome = true;
        }
    });
}

/// Window > Toolbars > Text: font controls as a standalone bar.
pub fn text_toolbar(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        text_properties(app, ui);
    });
}

/// Window > Toolbars > Zoom.
pub fn zoom_toolbar(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        for (label, pct) in [
            ("25%", 25.0),
            ("50%", 50.0),
            ("100%", 100.0),
            ("200%", 200.0),
            ("400%", 400.0),
        ] {
            if ui.button(label).clicked() {
                app.set_zoom_percent(pct);
            }
        }
        vsep(ui);
        if ui.button(tr("menu.view.zoom_in")).clicked() {
            app.zoom_step(true);
        }
        if ui.button(tr("menu.view.zoom_out")).clicked() {
            app.zoom_step(false);
        }
        if ui.button(tr("toolbar.zoom_to_page")).clicked() {
            app.zoom_to_page();
        }
        if ui.button(tr("toolbar.zoom_to_fit")).clicked() {
            app.zoom_to_fit();
        }
        if ui.button(tr("toolbar.zoom_to_selected")).clicked() {
            app.zoom_to_selection();
        }
    });
}

/// Window > Toolbars > Transform.
pub fn transform_toolbar(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        let has = !app.selection.is_empty();
        let mut b = app.selection_bounds().unwrap_or_default();
        let (x0, y0, w, h) = (b.x0, b.y0, b.width(), b.height());
        let mut changed = false;
        changed |= unit_value(ui, app, "x", &mut b.x0, 0.5);
        changed |= unit_value(ui, app, "y", &mut b.y0, 0.5);
        let mut nw = w;
        let mut nh = h;
        changed |= unit_value(ui, app, "w", &mut nw, 0.5);
        changed |= unit_value(ui, app, "h", &mut nh, 0.5);
        if changed && has && w > 1e-9 && h > 1e-9 {
            let t = tracedraw_core::Affine::translate((b.x0, b.y0))
                * tracedraw_core::Affine::scale_non_uniform(nw / w, nh / h)
                * tracedraw_core::Affine::translate((-x0, -y0));
            app.transform_selection(t);
        }
        vsep(ui);
        if ui
            .add_enabled(has, egui::Button::new(tr("toolbar.mirror_h_short")))
            .clicked()
        {
            app.mirror(true);
        }
        if ui
            .add_enabled(has, egui::Button::new(tr("toolbar.mirror_v_short")))
            .clicked()
        {
            app.mirror(false);
        }
        if ui
            .add_enabled(has, egui::Button::new(tr("toolbar.rotate_90")))
            .clicked()
        {
            let c = app.selection_bounds().unwrap_or_default().center();
            app.transform_selection(tracedraw_core::Affine::rotate_about(
                std::f64::consts::FRAC_PI_2,
                c,
            ));
        }
    });
}

fn unit_value(ui: &mut Ui, app: &App, label: &str, mm: &mut f64, speed: f64) -> bool {
    ui.label(
        egui::RichText::new(label)
            .color(Tokens::TEXT_DIM)
            .size(11.0),
    );
    let mut v = app.units.from_mm(*mm);
    let r = ui.add(
        egui::DragValue::new(&mut v)
            .speed(speed)
            .fixed_decimals(3)
            .suffix(format!(" {}", app.units.short())),
    );
    if r.changed() {
        *mm = app.units.to_mm(v);
    }
    r.drag_stopped() || r.lost_focus()
}

pub fn property_bar(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let shapes = app.selected_shapes();
        match app.tool {
            Tool::Shape => shape_tool_bar(app, ui),
            Tool::Pick | Tool::FreeformPick if shapes.is_empty() => page_properties(app, ui),
            Tool::Pick | Tool::FreeformPick
                if shapes
                    .iter()
                    .all(|s| matches!(s.kind, ShapeKind::Text { .. })) =>
            {
                // Text selected with the Pick tool: the object fields, then
                // the text fields, as the target design shows them.
                object_properties(app, ui);
                vsep(ui);
                if let Some(first) = shapes.first() {
                    let id = first.id;
                    app.sync_text_defaults_from(id);
                }
                text_properties(app, ui);
            }
            Tool::Pick | Tool::FreeformPick => object_properties(app, ui),
            Tool::Zoom | Tool::Pan => {
                ui.label(
                    egui::RichText::new(tr("toolbar.zoom_levels"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                for (l, pct) in [
                    ("25%", 25.0),
                    ("50%", 50.0),
                    ("100%", 100.0),
                    ("200%", 200.0),
                    ("400%", 400.0),
                ] {
                    if ui.button(l).clicked() {
                        app.set_zoom_percent(pct);
                    }
                }
                vsep(ui);
                if ui.button(tr("menu.view.zoom_to_page")).clicked() {
                    app.zoom_to_page();
                }
                if ui.button(tr("menu.view.zoom_to_fit")).clicked() {
                    app.zoom_to_fit();
                }
                if ui.button(tr("menu.view.zoom_to_selected")).clicked() {
                    app.zoom_to_selection();
                }
            }
            Tool::Rectangle | Tool::ThreePointRectangle => {
                object_properties(app, ui);
                vsep(ui);
                ui.label(
                    egui::RichText::new(tr("toolbar.corner_radius"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                let mut r = app.rect_radius;
                if ui
                    .add(
                        egui::DragValue::new(&mut r)
                            .speed(0.1)
                            .range(0.0..=500.0)
                            .suffix(" mm"),
                    )
                    .changed()
                {
                    app.rect_radius = r;
                    let cmds: Vec<Command> = shapes
                        .iter()
                        .filter_map(|s| match &s.kind {
                            ShapeKind::Rect { rect, .. } => Some(Command::SetShapeKind {
                                shape: s.id,
                                kind: ShapeKind::Rect {
                                    rect: *rect,
                                    radius: r,
                                },
                            }),
                            _ => None,
                        })
                        .collect();
                    if !cmds.is_empty() {
                        let _ = app.engine.run_batch("Corner Radius", &cmds);
                    }
                }
            }
            Tool::Ellipse | Tool::ThreePointEllipse => {
                object_properties(app, ui);
                vsep(ui);
                let mode = match app.ellipse_arc {
                    None => 0,
                    Some(a) if a.pie => 1,
                    Some(_) => 2,
                };
                let mut new_mode = mode;
                for (i, n, tip) in [
                    (0, tr("toolbar.ellipse"), tr("toolbar.ellipse_tip")),
                    (1, tr("toolbar.pie"), tr("toolbar.pie_tip")),
                    (2, tr("toolbar.arc"), tr("toolbar.arc_tip")),
                ] {
                    if ui
                        .selectable_label(mode == i, n)
                        .on_hover_text(tip)
                        .clicked()
                    {
                        new_mode = i;
                    }
                }
                let mut arc = app.ellipse_arc.unwrap_or(tracedraw_core::EllipseArc {
                    start_deg: 0.0,
                    end_deg: 270.0,
                    pie: true,
                });
                let mut changed = new_mode != mode;
                if new_mode == 0 {
                    app.ellipse_arc = None;
                } else {
                    arc.pie = new_mode == 1;
                    ui.label(
                        egui::RichText::new(tr("toolbar.start"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut arc.start_deg)
                                .speed(1.0)
                                .suffix("°"),
                        )
                        .changed();
                    ui.label(
                        egui::RichText::new(tr("toolbar.end"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut arc.end_deg)
                                .speed(1.0)
                                .suffix("°"),
                        )
                        .changed();
                    app.ellipse_arc = Some(arc);
                }
                if changed {
                    let new_arc = app.ellipse_arc;
                    let cmds: Vec<Command> = shapes
                        .iter()
                        .filter_map(|s| match &s.kind {
                            ShapeKind::Ellipse { rect, .. } => Some(Command::SetShapeKind {
                                shape: s.id,
                                kind: ShapeKind::Ellipse {
                                    rect: *rect,
                                    arc: new_arc,
                                },
                            }),
                            _ => None,
                        })
                        .collect();
                    if !cmds.is_empty() {
                        let _ = app.engine.run_batch("Ellipse", &cmds);
                    }
                }
            }
            Tool::Polygon | Tool::Star => {
                object_properties(app, ui);
                vsep(ui);
                ui.label(
                    egui::RichText::new(tr("toolbar.points"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                let mut n = app.polygon_points;
                let mut changed = ui
                    .add(egui::DragValue::new(&mut n).range(3..=500))
                    .changed();
                let mut sh = app.star_sharpness;
                if app.tool == Tool::Star {
                    ui.label(
                        egui::RichText::new(tr("toolbar.sharpness"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                    changed |= ui
                        .add(egui::DragValue::new(&mut sh).speed(0.01).range(0.0..=0.95))
                        .changed();
                }
                if changed {
                    app.polygon_points = n;
                    app.star_sharpness = sh;
                    let star = app.tool == Tool::Star;
                    let cmds: Vec<Command> = shapes
                        .iter()
                        .filter_map(|s| match &s.kind {
                            ShapeKind::Polygon { rect, .. } => Some(Command::SetShapeKind {
                                shape: s.id,
                                kind: ShapeKind::Polygon {
                                    rect: *rect,
                                    points: n,
                                    sharpness: if star { sh } else { 0.0 },
                                },
                            }),
                            _ => None,
                        })
                        .collect();
                    if !cmds.is_empty() {
                        let _ = app.engine.run_batch("Polygon", &cmds);
                    }
                }
            }
            Tool::Text => text_properties(app, ui),
            Tool::Freehand
            | Tool::Bezier
            | Tool::Pen
            | Tool::Polyline
            | Tool::TwoPointLine
            | Tool::BSpline
            | Tool::ThreePointCurve => {
                object_properties(app, ui);
                vsep(ui);
                let hint = if app.tool == Tool::ThreePointCurve {
                    tr("toolbar.three_point_hint")
                } else {
                    tr("toolbar.curve_finish_hint")
                };
                ui.label(egui::RichText::new(hint).color(Tokens::TEXT_DIM).size(11.0));
            }
            Tool::Blend => {
                ui.label(
                    egui::RichText::new(tr("toolbar.blend_steps"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(egui::DragValue::new(&mut app.blend_steps).range(1..=200));
                ui.label(
                    egui::RichText::new(tr("toolbar.drag_between_objects"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
            }
            Tool::Extrude => {
                ui.label(
                    egui::RichText::new(tr("toolbar.extrude_depth"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                let mut dx = app.extrude_depth.x;
                let mut dy = app.extrude_depth.y;
                ui.add(egui::DragValue::new(&mut dx).speed(0.2).suffix(" mm"));
                ui.add(egui::DragValue::new(&mut dy).speed(0.2).suffix(" mm"));
                app.extrude_depth = tracedraw_core::geometry::Vec2::new(dx, dy);
                vsep(ui);
                if ui
                    .add_enabled(!shapes.is_empty(), egui::Button::new(tr("docker.apply")))
                    .clicked()
                {
                    app.apply_extrude();
                }
            }
            Tool::Distort | Tool::Envelope => {
                use crate::tools2::DistortMode;
                for (m, n) in [
                    (DistortMode::PushPull, tr("toolbar.push_pull")),
                    (DistortMode::Zipper, tr("toolbar.zipper")),
                    (DistortMode::Twister, tr("toolbar.twister")),
                ] {
                    if ui.selectable_label(app.distort_mode == m, n).clicked() {
                        app.distort_mode = m;
                    }
                }
                ui.label(
                    egui::RichText::new(tr("toolbar.amplitude"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(
                    egui::DragValue::new(&mut app.distort_amount)
                        .speed(1.0)
                        .range(-200.0..=200.0),
                );
                if app.distort_mode == DistortMode::Zipper {
                    ui.label(
                        egui::RichText::new(tr("toolbar.frequency"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                    ui.add(egui::DragValue::new(&mut app.distort_frequency).range(1..=100));
                }
                vsep(ui);
                if ui
                    .add_enabled(!shapes.is_empty(), egui::Button::new(tr("docker.apply")))
                    .clicked()
                {
                    app.apply_distort();
                }
                if app.tool == Tool::Envelope {
                    ui.label(
                        egui::RichText::new(tr("toolbar.envelope_presets_hint"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                }
            }
            Tool::Eraser => {
                ui.label(
                    egui::RichText::new(tr("toolbar.eraser_thickness"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                let mut w = app.units.from_mm(app.eraser_width);
                if ui
                    .add(
                        egui::DragValue::new(&mut w)
                            .speed(0.1)
                            .range(0.01..=1000.0)
                            .suffix(format!(" {}", app.units.short())),
                    )
                    .changed()
                {
                    app.eraser_width = app.units.to_mm(w).max(0.01);
                }
                if ui
                    .add(egui::Button::new("\u{25cf}").selected(!app.eraser_square))
                    .on_hover_text(tr("toolbar.eraser_round"))
                    .clicked()
                {
                    app.eraser_square = false;
                }
                if ui
                    .add(egui::Button::new("\u{25a0}").selected(app.eraser_square))
                    .on_hover_text(tr("toolbar.eraser_square"))
                    .clicked()
                {
                    app.eraser_square = true;
                }
                ui.label(
                    egui::RichText::new(tr("hint.bar_eraser"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
            }
            Tool::Smooth | Tool::Smear | Tool::Twirl => {
                ui.label(
                    egui::RichText::new(tr("toolbar.nib_size"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(
                    egui::DragValue::new(&mut app.brush_radius)
                        .speed(0.5)
                        .range(1.0..=200.0)
                        .suffix(" mm"),
                );
                ui.label(
                    egui::RichText::new(tr("toolbar.drag_over_selected"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
            }
            Tool::Contour => {
                use crate::tools2::ContourDirection;
                ui.label(
                    egui::RichText::new(tr("docker.contour"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                for (d, n) in [
                    (ContourDirection::Inside, tr("docker.inside")),
                    (ContourDirection::Outside, tr("docker.outside")),
                ] {
                    if ui.selectable_label(app.contour_direction == d, n).clicked() {
                        app.contour_direction = d;
                    }
                }
                ui.label(
                    egui::RichText::new(tr("docker.steps"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(egui::DragValue::new(&mut app.contour_steps).range(1..=50));
                ui.label(
                    egui::RichText::new(tr("docker.offset"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(
                    egui::DragValue::new(&mut app.contour_offset)
                        .speed(0.1)
                        .range(0.05..=200.0)
                        .suffix(" mm"),
                );
                let [r, g, b] = app.contour_color.to_rgb8();
                let mut rgb = [r, g, b];
                if ui
                    .color_edit_button_srgb(&mut rgb)
                    .on_hover_text(tr("toolbar.outermost_contour_color"))
                    .changed()
                {
                    app.contour_color = tracedraw_core::Color::rgb8(rgb[0], rgb[1], rgb[2]);
                }
                vsep(ui);
                if ui
                    .add_enabled(!shapes.is_empty(), egui::Button::new(tr("docker.apply")))
                    .clicked()
                {
                    app.apply_contour();
                }
                if shapes.is_empty() {
                    ui.label(
                        egui::RichText::new(tr("toolbar.contour_hint"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                }
            }
            Tool::Crop => {
                ui.label(
                    egui::RichText::new(tr("toolbar.crop_hint"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
            }
            Tool::Knife => {
                ui.label(
                    egui::RichText::new(tr("toolbar.knife_hint"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
            }
            Tool::Spiral => {
                ui.label(
                    egui::RichText::new(tr("toolbar.revolutions"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(egui::DragValue::new(&mut app.spiral_revolutions).range(1..=100));
                if ui
                    .selectable_label(!app.spiral_logarithmic, tr("toolbar.symmetrical"))
                    .clicked()
                {
                    app.spiral_logarithmic = false;
                }
                if ui
                    .selectable_label(app.spiral_logarithmic, tr("toolbar.logarithmic"))
                    .clicked()
                {
                    app.spiral_logarithmic = true;
                }
            }
            Tool::GraphPaper => {
                ui.label(
                    egui::RichText::new(tr("toolbar.rows"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(egui::DragValue::new(&mut app.graph_rows).range(1..=99));
                ui.label(
                    egui::RichText::new(tr("toolbar.columns"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(egui::DragValue::new(&mut app.graph_cols).range(1..=99));
            }
            Tool::ActionLines => {
                ui.label(
                    egui::RichText::new(tr("toolbar.lines"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(egui::DragValue::new(&mut app.action_lines_count).range(2..=500));
                if ui
                    .selectable_label(!app.action_lines_radial, tr("toolbar.parallel"))
                    .clicked()
                {
                    app.action_lines_radial = false;
                }
                if ui
                    .selectable_label(app.action_lines_radial, tr("toolbar.radial"))
                    .clicked()
                {
                    app.action_lines_radial = true;
                }
            }
            Tool::CommonShapes => {
                ui.label(
                    egui::RichText::new(tr("toolbar.shape"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                egui::ComboBox::from_id_salt("common_shape")
                    .selected_text(app.common_shape.name())
                    .show_ui(ui, |ui| {
                        for cs in crate::tools2::CommonShape::ALL {
                            if ui
                                .selectable_label(app.common_shape == cs, cs.name())
                                .clicked()
                            {
                                app.common_shape = cs;
                            }
                        }
                    });
                ui.label(
                    egui::RichText::new(tr("toolbar.drag_to_draw"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
            }
            Tool::Table => {
                ui.label(
                    egui::RichText::new(tr("toolbar.rows"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(egui::DragValue::new(&mut app.table_rows).range(1..=100));
                ui.label(
                    egui::RichText::new(tr("toolbar.columns"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(egui::DragValue::new(&mut app.table_cols).range(1..=100));
                vsep(ui);
                // Cell fill, border width and colour: edit the selected table,
                // else the defaults for the next one.
                let selected = app.selected_table();
                let mut fill = selected
                    .as_ref()
                    .map(|(_, t)| t.cell_fill.clone())
                    .unwrap_or_else(|| app.table_fill.clone());
                let mut border = selected
                    .as_ref()
                    .map(|(_, t)| t.border.clone())
                    .unwrap_or_else(|| app.table_border.clone());
                let mut changed = false;
                ui.label(
                    egui::RichText::new(tr("toolbar.attr_fill"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                let mut has_fill = !matches!(fill, Fill::None);
                if ui.checkbox(&mut has_fill, "").changed() {
                    fill = if has_fill {
                        Fill::Solid(tracedraw_core::Color::WHITE)
                    } else {
                        Fill::None
                    };
                    changed = true;
                }
                if let Fill::Solid(c) = fill {
                    let mut rgb = c.to_rgb8();
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        fill = Fill::Solid(tracedraw_core::Color::rgb8(rgb[0], rgb[1], rgb[2]));
                        changed = true;
                    }
                }
                vsep(ui);
                ui.label(
                    egui::RichText::new(tr("toolbar.border"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                let mut width = border.as_ref().map(|b| b.width).unwrap_or(0.0);
                let r = ui.add(
                    egui::DragValue::new(&mut width)
                        .range(0.0..=20.0)
                        .speed(0.05)
                        .fixed_decimals(3)
                        .suffix(" mm"),
                );
                if r.changed() {
                    border = if width <= 0.0 {
                        None
                    } else {
                        let mut b = border.clone().unwrap_or_else(|| {
                            tracedraw_core::Stroke::hairline(tracedraw_core::Color::BLACK)
                        });
                        b.width = width;
                        Some(b)
                    };
                    changed = true;
                }
                if let Some(b) = &mut border {
                    let mut rgb = b.color.to_rgb8();
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        b.color = tracedraw_core::Color::rgb8(rgb[0], rgb[1], rgb[2]);
                        changed = true;
                    }
                }
                if changed {
                    match selected {
                        Some((id, mut t)) => {
                            t.cell_fill = fill;
                            t.border = border;
                            app.run(Command::SetShapeKind {
                                shape: id,
                                kind: ShapeKind::Table(t),
                            });
                        }
                        None => {
                            app.table_fill = fill;
                            app.table_border = border;
                        }
                    }
                }
                vsep(ui);
                ui.label(
                    egui::RichText::new(tr("toolbar.drag_to_draw_table"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
            }
            Tool::BrushStrokes => {
                ui.label(
                    egui::RichText::new(tr("media.calligraphic"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.label(
                    egui::RichText::new(tr("toolbar.width"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(
                    egui::DragValue::new(&mut app.media_width)
                        .speed(0.1)
                        .range(0.2..=50.0)
                        .suffix(" mm"),
                );
                ui.label(
                    egui::RichText::new(tr("docker.nib_angle"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                ui.add(
                    egui::DragValue::new(&mut app.media_angle)
                        .speed(1.0)
                        .range(0.0..=180.0)
                        .suffix("°"),
                );
            }
            Tool::ParallelDimension => {
                let n = app.dimension_points.len();
                let msg = match n {
                    0 => tr("toolbar.dimension_first_point"),
                    1 => tr("toolbar.dimension_second_point"),
                    _ => tr("toolbar.dimension_place_line"),
                };
                ui.label(egui::RichText::new(msg).color(Tokens::TEXT_DIM).size(11.0));
            }
            Tool::Connector => {
                ui.label(
                    egui::RichText::new(tr("toolbar.drag_between_objects"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
            }
            Tool::AnchorEditing => {
                let id = app.selection.first().copied();
                let n = id
                    .and_then(|id| app.doc().find_shape(id))
                    .map(|s| crate::anchors::custom_anchors(&s.data).len())
                    .unwrap_or(0);
                ui.label(
                    egui::RichText::new(trf("toolbar.anchors_n", &[("n", &n.to_string())]))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                if ui
                    .add_enabled(
                        app.anchor_sel.is_some(),
                        egui::Button::new(tr("toolbar.anchor_delete")),
                    )
                    .clicked()
                {
                    app.delete_selected_anchor();
                }
                if ui
                    .add_enabled(n > 0, egui::Button::new(tr("toolbar.anchor_clear")))
                    .clicked()
                {
                    if let Some(id) = id {
                        app.clear_anchors(id);
                    }
                }
                ui.label(
                    egui::RichText::new(tr("status.anchor_hint"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
            }
            Tool::DropShadow => {
                let first = shapes.first().and_then(|s| s.shadow);
                let mut sh = first.unwrap_or(app.shadow_default);
                let mut changed = false;
                ui.label(
                    egui::RichText::new(tr("toolbar.drop_shadow"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                if ui
                    .add_enabled(
                        !shapes.is_empty() && first.is_none(),
                        egui::Button::new(tr("docker.apply")),
                    )
                    .clicked()
                {
                    app.set_shadow(Some(sh), false);
                }
                let mut opacity = (sh.opacity * 100.0).round();
                ui.label(
                    egui::RichText::new(tr("toolbar.opacity"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                if ui
                    .add(
                        egui::DragValue::new(&mut opacity)
                            .range(0.0..=100.0)
                            .suffix(" %"),
                    )
                    .changed()
                {
                    sh.opacity = opacity / 100.0;
                    changed = true;
                }
                ui.label(
                    egui::RichText::new(tr("toolbar.feathering"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                let mut blur = sh.blur;
                if ui
                    .add(
                        egui::DragValue::new(&mut blur)
                            .speed(0.1)
                            .range(0.0..=50.0)
                            .suffix(" mm"),
                    )
                    .changed()
                {
                    sh.blur = blur;
                    changed = true;
                }
                let mut dx = sh.offset.x;
                let mut dy = sh.offset.y;
                ui.label(
                    egui::RichText::new(tr("docker.offset"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                changed |= ui
                    .add(egui::DragValue::new(&mut dx).speed(0.1).suffix(" mm"))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut dy).speed(0.1).suffix(" mm"))
                    .changed();
                sh.offset = tracedraw_core::geometry::Vec2::new(dx, dy);
                let [r, g, b] = sh.color.to_rgb8();
                let mut rgb = [r, g, b];
                if ui.color_edit_button_srgb(&mut rgb).changed() {
                    sh.color = tracedraw_core::Color::rgb8(rgb[0], rgb[1], rgb[2]);
                    changed = true;
                }
                if changed {
                    app.shadow_default = sh;
                    if first.is_some() {
                        app.set_shadow(Some(sh), true);
                    }
                }
                vsep(ui);
                if ui
                    .add_enabled(
                        first.is_some(),
                        egui::Button::new(tr("toolbar.clear_shadow")),
                    )
                    .clicked()
                {
                    app.set_shadow(None, false);
                }
                if shapes.is_empty() {
                    ui.label(
                        egui::RichText::new(tr("toolbar.shadow_hint"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                }
            }
            Tool::Transparency => {
                let kinds = [
                    tr("transparency.none"),
                    tr("transparency.uniform"),
                    tr("transparency.fountain"),
                    tr("transparency.pattern"),
                    tr("transparency.texture"),
                ];
                // Reflect the first selected object in the bar.
                let has_mask = shapes
                    .first()
                    .map(|s| {
                        s.effects
                            .iter()
                            .any(|e| matches!(e, tracedraw_core::live::Effect::Transparency { .. }))
                    })
                    .unwrap_or(false);
                let first_opacity = shapes.first().map(|s| s.opacity).unwrap_or(1.0);
                let current = if has_mask {
                    app.transparency_default.kind.max(1) as usize + 1
                } else if first_opacity < 1.0 {
                    1
                } else {
                    0
                };
                let mut changed = false;
                egui::ComboBox::from_id_salt("transparency_kind")
                    .selected_text(kinds[current].clone())
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        for (i, k) in kinds.iter().enumerate() {
                            if ui.selectable_label(current == i, k).clicked() {
                                if i == 0 {
                                    app.clear_transparency();
                                } else {
                                    app.transparency_default.set_kind((i - 1) as u8);
                                    changed = true;
                                }
                            }
                        }
                    });
                if app.transparency_default.kind == 0 {
                    let mut amount = if first_opacity < 1.0 {
                        ((1.0 - first_opacity) * 100.0).round()
                    } else {
                        app.transparency_default.amount
                    };
                    if ui
                        .add_enabled(
                            !shapes.is_empty(),
                            egui::Slider::new(&mut amount, 0.0..=100.0).suffix(" %"),
                        )
                        .changed()
                    {
                        app.transparency_default.amount = amount;
                        changed = true;
                    }
                } else if app.transparency_default.kind == 1 {
                    if let Fill::Fountain(f) = &mut app.transparency_default.mask {
                        let mut angle = f.angle;
                        if ui
                            .add(egui::DragValue::new(&mut angle).suffix("°").speed(1.0))
                            .changed()
                        {
                            f.angle = angle;
                            changed = true;
                        }
                        let kinds = [
                            (tracedraw_core::style::FountainKind::Linear, "fill.linear"),
                            (tracedraw_core::style::FountainKind::Radial, "fill.radial"),
                            (tracedraw_core::style::FountainKind::Conical, "fill.conical"),
                            (tracedraw_core::style::FountainKind::Square, "fill.square"),
                        ];
                        let name = kinds
                            .iter()
                            .find(|(k, _)| *k == f.kind)
                            .map(|(_, n)| tr(n))
                            .unwrap_or_default();
                        egui::ComboBox::from_id_salt("transparency_fountain_kind")
                            .selected_text(name)
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                for (k, n) in kinds {
                                    if ui.selectable_label(f.kind == k, tr(n)).clicked() {
                                        f.kind = k;
                                        changed = true;
                                    }
                                }
                            });
                    }
                }
                vsep(ui);
                ui.label(
                    egui::RichText::new(tr("transparency.merge_mode"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                egui::ComboBox::from_id_salt("transparency_merge")
                    .selected_text(app.transparency_default.merge.name())
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        for m in tracedraw_core::live::MergeMode::ALL {
                            if ui
                                .selectable_label(app.transparency_default.merge == m, m.name())
                                .clicked()
                            {
                                app.transparency_default.merge = m;
                                changed = true;
                            }
                        }
                    });
                let targets = [
                    tr("transparency.target_fill"),
                    tr("transparency.target_outline"),
                    tr("transparency.target_all"),
                ];
                egui::ComboBox::from_id_salt("transparency_target")
                    .selected_text(
                        targets[(app.transparency_default.target as usize).min(2)].clone(),
                    )
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        for (i, t) in targets.iter().enumerate() {
                            if ui
                                .selectable_label(app.transparency_default.target as usize == i, t)
                                .clicked()
                            {
                                app.transparency_default.target = i as u8;
                                changed = true;
                            }
                        }
                    });
                if changed && !shapes.is_empty() {
                    let s = app.transparency_default.clone();
                    app.apply_transparency(&s);
                }
                if shapes.is_empty() {
                    ui.label(
                        egui::RichText::new(tr("transparency.hint"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                }
            }
            Tool::FreeTransform => {
                for m in crate::app::FreeTransformMode::ALL {
                    if ui
                        .selectable_label(app.free_transform_mode == m, tr(m.label_key()))
                        .clicked()
                    {
                        app.free_transform_mode = m;
                    }
                }
                vsep(ui);
                ui.checkbox(
                    &mut app.free_transform_duplicate,
                    tr("toolbar.apply_to_duplicate"),
                );
                vsep(ui);
                if !shapes.is_empty() {
                    object_properties(app, ui);
                } else {
                    ui.label(
                        egui::RichText::new(tr("hint.bar_free_transform"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                }
            }
            Tool::ColorEyedropper => {
                let has = app.eyedropper_color.is_some();
                if ui
                    .selectable_label(!app.eyedropper_apply, tr("toolbar.select_color"))
                    .clicked()
                {
                    app.eyedropper_apply = false;
                }
                if ui
                    .add_enabled(
                        has,
                        egui::Button::selectable(app.eyedropper_apply, tr("toolbar.apply_color")),
                    )
                    .clicked()
                {
                    app.eyedropper_apply = true;
                }
                vsep(ui);
                ui.label(
                    egui::RichText::new(tr("toolbar.sample_size"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                for n in [1u32, 2, 5] {
                    if ui
                        .selectable_label(app.eyedropper_sample == n, format!("{n}x{n}"))
                        .clicked()
                    {
                        app.eyedropper_sample = n;
                    }
                }
                vsep(ui);
                if let Some(c) = app.eyedropper_color {
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                    let [cr, cg, cb] = c.to_rgb8();
                    ui.painter()
                        .rect_filled(r, 2.0, egui::Color32::from_rgb(cr, cg, cb));
                    ui.painter().rect_stroke(
                        r,
                        2.0,
                        egui::Stroke::new(1.0, Tokens::BORDER),
                        egui::StrokeKind::Inside,
                    );
                    ui.label(
                        egui::RichText::new(crate::app::color_description(c))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(tr("hint.bar_eyedropper"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                }
            }
            Tool::AttributesEyedropper => {
                let has = app.eyedropper_attrs.is_some();
                if ui
                    .selectable_label(!app.eyedropper_apply, tr("toolbar.select_attrs"))
                    .clicked()
                {
                    app.eyedropper_apply = false;
                }
                if ui
                    .add_enabled(
                        has,
                        egui::Button::selectable(app.eyedropper_apply, tr("toolbar.apply_attrs")),
                    )
                    .clicked()
                {
                    app.eyedropper_apply = true;
                }
                vsep(ui);
                let g = &mut app.eyedropper_groups;
                ui.menu_button(tr("toolbar.attr_properties"), |ui| {
                    ui.checkbox(&mut g.outline, tr("toolbar.attr_outline"));
                    ui.checkbox(&mut g.fill, tr("toolbar.attr_fill"));
                    ui.checkbox(&mut g.text, tr("toolbar.attr_text"));
                });
                ui.menu_button(tr("toolbar.attr_transformations"), |ui| {
                    ui.checkbox(&mut g.size, tr("toolbar.attr_size"));
                    ui.checkbox(&mut g.rotation, tr("toolbar.attr_rotation"));
                    ui.checkbox(&mut g.position, tr("toolbar.attr_position"));
                });
                ui.checkbox(&mut g.effects, tr("toolbar.attr_effects"));
                vsep(ui);
                ui.label(
                    egui::RichText::new(if has {
                        tr("hint.bar_attrs_apply")
                    } else {
                        tr("hint.bar_attrs")
                    })
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
                );
            }
            Tool::BlockShadow => {
                use tracedraw_core::{geometry::Vec2 as CVec2, live::Effect};
                // The selected object's block shadow, else the defaults.
                let current = shapes.first().and_then(|s| {
                    s.effects.iter().find_map(|e| match e {
                        Effect::BlockShadow { offset, color, gap } => Some((*offset, *color, *gap)),
                        _ => None,
                    })
                });
                let (mut offset, mut color, mut gap) = current.unwrap_or((
                    CVec2::new(5.0, -5.0),
                    app.block_shadow_color,
                    app.block_shadow_gap,
                ));
                let mut changed = false;
                let mut depth = offset.hypot();
                let mut dir = offset.y.atan2(offset.x).to_degrees();
                ui.label(
                    egui::RichText::new(tr("toolbar.depth"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut depth)
                            .speed(0.1)
                            .range(0.0..=500.0)
                            .fixed_decimals(2)
                            .suffix(" mm"),
                    )
                    .changed();
                ui.label(
                    egui::RichText::new(tr("toolbar.direction"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut dir)
                            .speed(1.0)
                            .range(-180.0..=180.0)
                            .fixed_decimals(1)
                            .suffix("°"),
                    )
                    .changed();
                if changed {
                    let a = dir.to_radians();
                    offset = CVec2::new(depth * a.cos(), depth * a.sin());
                }
                let mut rgb = color.to_rgb8();
                if ui.color_edit_button_srgb(&mut rgb).changed() {
                    color = tracedraw_core::Color::rgb8(rgb[0], rgb[1], rgb[2]);
                    changed = true;
                }
                ui.label(
                    egui::RichText::new(tr("toolbar.gap"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut gap)
                            .speed(0.05)
                            .range(0.0..=50.0)
                            .fixed_decimals(2)
                            .suffix(" mm"),
                    )
                    .changed();
                if changed {
                    app.block_shadow_color = color;
                    app.block_shadow_gap = gap;
                    if current.is_some() {
                        app.push_effect(Effect::BlockShadow { offset, color, gap }, true);
                    }
                }
                vsep(ui);
                if ui
                    .add_enabled(
                        current.is_some(),
                        egui::Button::new(tr("toolbar.clear_block_shadow")),
                    )
                    .clicked()
                {
                    app.remove_effects_of_kind(&Effect::BlockShadow {
                        offset: CVec2::ZERO,
                        color,
                        gap,
                    });
                }
                if current.is_none() {
                    ui.label(
                        egui::RichText::new(tr("hint.bar_block_shadow"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                }
            }
            Tool::InteractiveFill => interactive_fill_bar(app, ui, &shapes),
            Tool::AreaFill | Tool::MeshFill => {
                ui.label(
                    egui::RichText::new(tr("toolbar.fill_hint"))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
            }
            t => {
                ui.label(
                    egui::RichText::new(trf("toolbar.tool_n", &[("t", &t.name())]))
                        .color(Tokens::TEXT_DIM)
                        .size(11.0),
                );
                if !t.implemented() {
                    ui.label(
                        egui::RichText::new(tr("toolbar.not_implemented"))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                }
            }
        }
    });
}

fn page_properties(app: &mut App, ui: &mut Ui) {
    let size = app.page_size();
    let custom = tr("toolbar.custom");
    let presets: [(&str, Size); 4] = [
        ("A4", paper::A4),
        ("A3", paper::A3),
        ("Letter", paper::LETTER),
        (custom.as_str(), size),
    ];
    let current = presets
        .iter()
        .find(|(_, s)| {
            ((s.width - size.width).abs() < 0.01 && (s.height - size.height).abs() < 0.01)
                || ((s.height - size.width).abs() < 0.01 && (s.width - size.height).abs() < 0.01)
        })
        .map(|(n, _)| *n)
        .unwrap_or(custom.as_str());
    egui::ComboBox::from_id_salt("page_size")
        .selected_text(current)
        .width(90.0)
        .show_ui(ui, |ui| {
            for (n, s) in presets.iter().take(3) {
                if ui.selectable_label(current == *n, *n).clicked() {
                    let page = app.page;
                    let landscape = size.width > size.height;
                    let s = if landscape {
                        Size::new(s.height, s.width)
                    } else {
                        *s
                    };
                    app.run(Command::ResizePage { page, size: s });
                    app.fit_pending = true;
                }
            }
        });
    let mut w = size.width;
    let mut h = size.height;
    let cw = unit_value(ui, app, "W", &mut w, 0.5);
    let ch = unit_value(ui, app, "H", &mut h, 0.5);
    if (cw || ch) && w > 1.0 && h > 1.0 {
        let page = app.page;
        app.run(Command::ResizePage {
            page,
            size: Size::new(w, h),
        });
    }
    vsep(ui);
    let portrait = size.height >= size.width;
    if ui
        .selectable_label(portrait, "▯")
        .on_hover_text(tr("dialog.portrait"))
        .clicked()
        && !portrait
    {
        let page = app.page;
        app.run(Command::ResizePage {
            page,
            size: Size::new(size.height, size.width),
        });
        app.fit_pending = true;
    }
    if ui
        .selectable_label(!portrait, "▭")
        .on_hover_text(tr("dialog.landscape"))
        .clicked()
        && portrait
    {
        let page = app.page;
        app.run(Command::ResizePage {
            page,
            size: Size::new(size.height, size.width),
        });
        app.fit_pending = true;
    }
    vsep(ui);
    let _ = ui
        .selectable_label(true, tr("dialog.all_pages"))
        .on_hover_text(tr("toolbar.all_pages_tip"));
    let _ = ui
        .selectable_label(false, tr("toolbar.current_page"))
        .on_hover_text(tr("toolbar.current_page_tip"));
    vsep(ui);
    ui.label(
        egui::RichText::new(tr("toolbar.units"))
            .color(Tokens::TEXT_DIM)
            .size(11.0),
    );
    egui::ComboBox::from_id_salt("units")
        .selected_text(app.units.label())
        .width(100.0)
        .show_ui(ui, |ui| {
            for u in Units::ALL {
                if ui.selectable_label(app.units == u, u.label()).clicked() {
                    app.units = u;
                }
            }
        });
    vsep(ui);
    let mut nudge = app.nudge_mm;
    if unit_value(ui, app, &tr("toolbar.nudge"), &mut nudge, 0.1) && nudge > 0.0 {
        app.nudge_mm = nudge;
    }
    let mut dx = app.duplicate_offset.x;
    let mut dy = app.duplicate_offset.y;
    unit_value(ui, app, &tr("toolbar.dup_x"), &mut dx, 0.1);
    unit_value(ui, app, "y", &mut dy, 0.1);
    app.duplicate_offset = tracedraw_core::geometry::Vec2::new(dx, dy);
}

fn object_properties(app: &mut App, ui: &mut Ui) {
    let Some(b) = app.selection_bounds() else {
        page_properties(app, ui);
        return;
    };
    let mut x = b.x0;
    let mut y = b.y0;
    let mut w = b.width();
    let mut h = b.height();
    let cx = unit_value(ui, app, "X", &mut x, 0.5);
    let cy = unit_value(ui, app, "Y", &mut y, 0.5);
    if cx || cy {
        app.transform_selection(Affine::translate((x - b.x0, y - b.y0)));
    }
    vsep(ui);
    let cw = unit_value(ui, app, "W", &mut w, 0.5);
    let ch = unit_value(ui, app, "H", &mut h, 0.5);
    if (cw || ch) && w > 0.01 && h > 0.01 {
        let sx = w / b.width().max(1e-9);
        let sy = h / b.height().max(1e-9);
        let anchor = tracedraw_core::geometry::Point::new(b.x0, b.y0);
        app.transform_selection(
            Affine::translate(anchor.to_vec2())
                * Affine::scale_non_uniform(sx, sy)
                * Affine::translate(-anchor.to_vec2()),
        );
    }
    ui.label(
        egui::RichText::new(tr("toolbar.scale"))
            .color(Tokens::TEXT_DIM)
            .size(11.0),
    );
    let mut sx = 100.0;
    let mut sy = 100.0;
    let rx = ui.add(egui::DragValue::new(&mut sx).speed(1.0).suffix(" %"));
    let ry = ui.add(egui::DragValue::new(&mut sy).speed(1.0).suffix(" %"));
    if (rx.drag_stopped() || rx.lost_focus() || ry.drag_stopped() || ry.lost_focus())
        && (sx != 100.0 || sy != 100.0)
    {
        let c = b.center();
        app.transform_selection(
            Affine::translate(c.to_vec2())
                * Affine::scale_non_uniform(sx / 100.0, sy / 100.0)
                * Affine::translate(-c.to_vec2()),
        );
    }
    vsep(ui);
    ui.label(
        egui::RichText::new(tr("toolbar.angle"))
            .color(Tokens::TEXT_DIM)
            .size(11.0),
    );
    let mut angle = 0.0f64;
    let ra = ui.add(egui::DragValue::new(&mut angle).speed(1.0).suffix("°"));
    if (ra.drag_stopped() || ra.lost_focus()) && angle != 0.0 {
        let c = b.center();
        app.transform_selection(
            Affine::translate(c.to_vec2())
                * Affine::rotate(angle.to_radians())
                * Affine::translate(-c.to_vec2()),
        );
    }
    if tb_button(ui, icons::Action::Mirror, &tr("toolbar.mirror_h"), true) {
        app.mirror(true);
    }
    if tb_button(ui, icons::Action::Flip, &tr("toolbar.mirror_v"), true) {
        app.mirror(false);
    }
    vsep(ui);
    // Outline width.
    let first = app.selected_shapes().into_iter().next();
    let width = first
        .as_ref()
        .and_then(|s| s.stroke.as_ref())
        .map(|s| s.width);
    ui.label(
        egui::RichText::new(tr("docker.outline"))
            .color(Tokens::TEXT_DIM)
            .size(11.0),
    );
    let label = match width {
        None => tr("toolbar.none"),
        Some(w) if w <= tracedraw_core::Stroke::HAIRLINE + 1e-9 => tr("toolbar.hairline"),
        Some(w) => format!("{:.2} mm", w),
    };
    egui::ComboBox::from_id_salt("outline_width")
        .selected_text(label)
        .width(90.0)
        .show_ui(ui, |ui| {
            if ui.selectable_label(false, tr("toolbar.none")).clicked() {
                let shapes = app.selection.clone();
                app.run(Command::SetStroke {
                    shapes,
                    stroke: None,
                });
            }
            if ui.selectable_label(false, tr("toolbar.hairline")).clicked() {
                app.apply_outline_width(tracedraw_core::Stroke::HAIRLINE);
            }
            for w in [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0, 8.0] {
                if ui.selectable_label(false, format!("{w:.2} mm")).clicked() {
                    app.apply_outline_width(w);
                }
            }
        });
    vsep(ui);
    if ui.button(tr("toolbar.to_front")).clicked() {
        app.order(0);
    }
    if ui.button(tr("toolbar.to_back")).clicked() {
        app.order(3);
    }
    vsep(ui);
    if ui
        .button(tr("menu.object.convert_to_curves"))
        .on_hover_text("Ctrl+Q")
        .clicked()
    {
        app.convert_to_curves();
    }
    let _ = Rect::ZERO;
}

fn text_properties(app: &mut App, ui: &mut Ui) {
    ui.label(
        egui::RichText::new(tr("toolbar.font"))
            .color(Tokens::TEXT_DIM)
            .size(11.0),
    );
    let mut changed = false;
    egui::ComboBox::from_id_salt("font")
        .selected_text(app.text_font.clone())
        .width(140.0)
        .show_ui(ui, |ui| {
            let families = app.font_families.clone();
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for f in &families {
                        if ui.selectable_label(&app.text_font == f, f).clicked() {
                            app.text_font = f.clone();
                            changed = true;
                        }
                    }
                });
        });
    let mut size = app.text_size_pt;
    if ui
        .add(
            egui::DragValue::new(&mut size)
                .speed(0.5)
                .range(1.0..=999.0)
                .suffix(" pt"),
        )
        .changed()
    {
        app.text_size_pt = size;
        changed = true;
    }
    changed |= ui
        .selectable_label(app.text_bold, egui::RichText::new("B").strong())
        .clicked()
        .then(|| app.text_bold = !app.text_bold)
        .is_some();
    changed |= ui
        .selectable_label(app.text_italic, egui::RichText::new("I").italics())
        .clicked()
        .then(|| app.text_italic = !app.text_italic)
        .is_some();
    changed |= ui
        .selectable_label(app.text_underline, egui::RichText::new("U").underline())
        .clicked()
        .then(|| app.text_underline = !app.text_underline)
        .is_some();
    vsep(ui);
    for (a, label, tip) in [
        (
            tracedraw_core::TextAlign::Left,
            tr("docker.align_left_short"),
            tr("toolbar.align_left"),
        ),
        (
            tracedraw_core::TextAlign::Center,
            tr("docker.align_center_short"),
            tr("toolbar.align_center"),
        ),
        (
            tracedraw_core::TextAlign::Right,
            tr("docker.align_right_short"),
            tr("toolbar.align_right"),
        ),
        (
            tracedraw_core::TextAlign::Justify,
            tr("docker.align_justify_short"),
            tr("toolbar.full_justify"),
        ),
    ] {
        if ui
            .selectable_label(app.text_align == a, label)
            .on_hover_text(tip)
            .clicked()
        {
            app.text_align = a;
            changed = true;
        }
    }
    if changed {
        app.apply_text_style();
    }
}

fn shape_tool_bar(app: &mut App, ui: &mut Ui) {
    use tracedraw_core::nodes::NodeType;
    let has_nodes = !app.node_selection.is_empty();
    let has_curve = app
        .selected_shapes()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Path { .. }));
    let b = |ui: &mut Ui, label: &str, tip: &str, enabled: bool| -> bool {
        ui.add_enabled(
            enabled,
            egui::Button::new(egui::RichText::new(label).size(11.0)),
        )
        .on_hover_text(tip)
        .clicked()
    };
    if b(
        ui,
        &tr("toolbar.add_node"),
        &tr("toolbar.add_node_tip"),
        has_nodes,
    ) {
        app.add_node_midpoints();
    }
    if b(
        ui,
        &tr("toolbar.delete_node"),
        &tr("toolbar.delete_node_tip"),
        has_nodes,
    ) {
        app.delete_selected_nodes();
    }
    vsep(ui);
    if b(
        ui,
        &tr("toolbar.break"),
        &tr("toolbar.break_tip"),
        has_nodes,
    ) {
        app.break_selected_nodes();
    }
    if b(
        ui,
        &tr("toolbar.close"),
        &tr("toolbar.close_tip"),
        has_curve,
    ) {
        app.close_selected_curves();
    }
    vsep(ui);
    if b(
        ui,
        &tr("toolbar.to_line"),
        &tr("toolbar.to_line_tip"),
        has_nodes,
    ) {
        app.selected_segments_to_line();
    }
    if b(
        ui,
        &tr("toolbar.to_curve"),
        &tr("toolbar.to_curve_tip"),
        has_nodes,
    ) {
        app.selected_segments_to_curve();
    }
    vsep(ui);
    let current = app.current_node_type();
    for (ty, name, tip) in [
        (
            NodeType::Cusp,
            tr("context.node_cusp"),
            tr("toolbar.cusp_tip"),
        ),
        (
            NodeType::Smooth,
            tr("context.node_smooth"),
            tr("toolbar.smooth_tip"),
        ),
        (
            NodeType::Symmetrical,
            tr("toolbar.symm_short"),
            tr("toolbar.symm_tip"),
        ),
    ] {
        if ui
            .add_enabled(
                has_nodes,
                egui::Button::selectable(current == Some(ty), name),
            )
            .on_hover_text(tip)
            .clicked()
        {
            app.set_selected_node_type(ty);
        }
    }
    vsep(ui);
    if b(
        ui,
        &tr("toolbar.reverse"),
        &tr("toolbar.reverse_tip"),
        has_curve,
    ) {
        app.reverse_selected_curves();
    }
    if b(
        ui,
        &tr("toolbar.select_all"),
        &tr("toolbar.select_all_nodes_tip"),
        has_curve,
    ) {
        app.select_all_nodes();
    }
    vsep(ui);
    if b(
        ui,
        &tr("toolbar.reduce_nodes"),
        &tr("toolbar.reduce_nodes_tip"),
        has_curve,
    ) {
        app.reduce_selected_nodes();
    }
    let two_nodes = app.node_selection.len() >= 2;
    if b(ui, "⟷", &tr("toolbar.align_nodes_h"), two_nodes) {
        app.align_selected_nodes(true, false);
    }
    if b(ui, "↕", &tr("toolbar.align_nodes_v"), two_nodes) {
        app.align_selected_nodes(false, true);
    }
    vsep(ui);
    if b(
        ui,
        &tr("menu.object.convert_to_curves"),
        "Ctrl+Q",
        !app.selection.is_empty(),
    ) {
        app.convert_to_curves();
    }
    if !has_curve && !app.selection.is_empty() {
        ui.label(
            egui::RichText::new(tr("toolbar.convert_to_curves_hint"))
                .color(Tokens::TEXT_DIM)
                .size(11.0),
        );
    }
}

/// Interactive Fill property bar: fill type, then the fields of that type
/// (colour; fountain kind, start and end colours, angle, edge pad). With a
/// selection the fields edit it; otherwise they set the default fill.
fn interactive_fill_bar(app: &mut App, ui: &mut Ui, shapes: &[tracedraw_core::document::Shape]) {
    use tracedraw_core::style::FountainKind;
    let mut fill = shapes
        .first()
        .map(|s| s.fill.clone())
        .unwrap_or_else(|| app.default_fill.clone());
    let kind = match &fill {
        Fill::None => 0,
        Fill::Solid(_) => 1,
        Fill::Fountain(_) => 2,
        Fill::Pattern(_) => 3,
        Fill::Texture(_) => 4,
        Fill::Mesh(_) => 5,
    };
    let mut changed = false;
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
        if ui.selectable_label(kind == i, tr(key)).clicked() && kind != i {
            let base = fill
                .preview_color()
                .unwrap_or(tracedraw_core::Color::cmyk_pct(0.0, 0.0, 0.0, 20.0));
            fill = match i {
                0 => Fill::None,
                1 => Fill::Solid(base),
                2 => Fill::linear(base, tracedraw_core::Color::WHITE, 0.0),
                3 => Fill::Pattern(tracedraw_core::Pattern::TwoColor {
                    tile: tracedraw_core::style::PatternTile::Checker,
                    front: base,
                    back: tracedraw_core::Color::WHITE,
                    size_mm: 10.0,
                }),
                4 => Fill::Texture(tracedraw_core::style::Texture {
                    kind: tracedraw_core::style::TextureKind::Clouds,
                    color_a: base,
                    color_b: tracedraw_core::Color::WHITE,
                    scale: 20.0,
                    seed: 1,
                }),
                _ => Fill::Mesh(tracedraw_core::Mesh::new(
                    shapes
                        .first()
                        .map(|s| s.bounds())
                        .unwrap_or(Rect::new(0.0, 0.0, 100.0, 100.0)),
                    2,
                    2,
                    base,
                )),
            };
            changed = true;
        }
    }
    vsep(ui);
    let color_button = |ui: &mut Ui, c: &mut tracedraw_core::Color| -> bool {
        let mut rgb = c.to_rgb8();
        if ui.color_edit_button_srgb(&mut rgb).changed() {
            *c = tracedraw_core::Color::rgb8(rgb[0], rgb[1], rgb[2]);
            true
        } else {
            false
        }
    };
    match &mut fill {
        Fill::Solid(c) => {
            ui.label(
                egui::RichText::new(tr("docker.colour"))
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
            );
            changed |= color_button(ui, c);
        }
        Fill::Fountain(f) => {
            for (k, key) in [
                (FountainKind::Linear, "fill.linear"),
                (FountainKind::Radial, "fill.radial"),
                (FountainKind::Conical, "fill.conical"),
                (FountainKind::Square, "fill.square"),
            ] {
                if ui.selectable_label(f.kind == k, tr(key)).clicked() && f.kind != k {
                    f.kind = k;
                    changed = true;
                }
            }
            vsep(ui);
            if let Some(first) = f.stops.first_mut() {
                changed |= color_button(ui, &mut first.color);
            }
            if let Some(last) = f.stops.last_mut() {
                changed |= color_button(ui, &mut last.color);
            }
            ui.label(
                egui::RichText::new(tr("toolbar.angle"))
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
            );
            changed |= ui
                .add(
                    egui::DragValue::new(&mut f.angle)
                        .speed(1.0)
                        .range(-360.0..=360.0)
                        .fixed_decimals(1)
                        .suffix("°"),
                )
                .changed();
            ui.label(
                egui::RichText::new(tr("docker.edge_pad"))
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
            );
            let mut pad = f.edge_pad * 100.0;
            if ui
                .add(
                    egui::DragValue::new(&mut pad)
                        .speed(1.0)
                        .range(0.0..=49.0)
                        .suffix(" %"),
                )
                .changed()
            {
                f.edge_pad = pad / 100.0;
                changed = true;
            }
        }
        _ => {}
    }
    if changed {
        app.apply_fill_or_default(fill);
    }
    vsep(ui);
    ui.label(
        egui::RichText::new(tr("toolbar.fill_hint"))
            .color(Tokens::TEXT_DIM)
            .size(11.0),
    );
}
