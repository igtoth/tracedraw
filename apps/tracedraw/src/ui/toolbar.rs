//! Standard toolbar and the context-sensitive property bar.

use crate::app::{App, Units};
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::icons;
use egui::{Ui, Vec2};
use tracedraw_core::{
    document::{paper, ShapeKind},
    geometry::{Affine, Rect, Size},
    Command,
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

fn vsep(ui: &mut Ui) {
    ui.add(egui::Separator::default().vertical().spacing(6.0));
}

pub fn standard_toolbar(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        if tb_button(ui, icons::Action::New, "New (Ctrl+N)", true) {
            app.new_document();
        }
        if tb_button(ui, icons::Action::Open, "Open (Ctrl+O)", true) {
            app.open_dialog();
        }
        if tb_button(ui, icons::Action::Save, "Save (Ctrl+S)", true) {
            app.save(false);
        }
        let _ = tb_button(ui, icons::Action::Print, "Print (Ctrl+P)", false);
        vsep(ui);
        let has = !app.selection.is_empty();
        if tb_button(ui, icons::Action::Cut, "Cut (Ctrl+X)", has) {
            app.cut();
        }
        if tb_button(ui, icons::Action::Copy, "Copy (Ctrl+C)", has) {
            app.copy();
        }
        if tb_button(
            ui,
            icons::Action::Paste,
            "Paste (Ctrl+V)",
            app.clipboard.is_some(),
        ) {
            app.paste();
        }
        vsep(ui);
        if tb_button(
            ui,
            icons::Action::Undo,
            "Undo (Ctrl+Z)",
            app.engine.undo_label().is_some(),
        ) {
            app.undo();
        }
        if tb_button(
            ui,
            icons::Action::Redo,
            "Redo (Ctrl+Shift+Z)",
            app.engine.redo_label().is_some(),
        ) {
            app.redo();
        }
        vsep(ui);
        if tb_button(ui, icons::Action::Import, "Import (Ctrl+I)", true) {
            app.import();
        }
        if tb_button(ui, icons::Action::Export, "Export (Ctrl+E)", true) {
            app.export();
        }
        let _ = tb_button(ui, icons::Action::Pdf, "Publish to PDF", false);
        vsep(ui);
        // Zoom level combo.
        let mut pct = app.zoom_percent();
        let levels = [
            "To Page",
            "To Fit",
            "To Selected",
            "25%",
            "50%",
            "75%",
            "100%",
            "150%",
            "200%",
            "400%",
            "800%",
        ];
        egui::ComboBox::from_id_salt("zoom_levels")
            .selected_text(format!("{:.0}%", pct))
            .width(80.0)
            .show_ui(ui, |ui| {
                for l in levels {
                    if ui.selectable_label(false, l).clicked() {
                        match l {
                            "To Page" => app.zoom_to_page(),
                            "To Fit" => app.zoom_to_fit(),
                            "To Selected" => app.zoom_to_selection(),
                            _ => {
                                pct = l.trim_end_matches('%').parse().unwrap_or(100.0);
                                app.set_zoom_percent(pct);
                            }
                        }
                    }
                }
            });
        vsep(ui);
        let _ = tb_button(ui, icons::Action::Snap, "Snap To", false);
        let _ = tb_button(ui, icons::Action::Options, "Options (Ctrl+J)", false);
        vsep(ui);
        let _ = tb_button(ui, icons::Action::Welcome, "Welcome Screen", false);
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
            Tool::Pick | Tool::FreeformPick => object_properties(app, ui),
            Tool::Zoom | Tool::Pan => {
                ui.label(egui::RichText::new("Zoom levels").color(Tokens::TEXT_DIM).size(11.0));
                for (l, pct) in [("25%", 25.0), ("50%", 50.0), ("100%", 100.0), ("200%", 200.0), ("400%", 400.0)] {
                    if ui.button(l).clicked() {
                        app.set_zoom_percent(pct);
                    }
                }
                vsep(ui);
                if ui.button("Zoom to page").clicked() {
                    app.zoom_to_page();
                }
                if ui.button("Zoom to fit").clicked() {
                    app.zoom_to_fit();
                }
                if ui.button("Zoom to selected").clicked() {
                    app.zoom_to_selection();
                }
            }
            Tool::Rectangle | Tool::ThreePointRectangle => {
                object_properties(app, ui);
                vsep(ui);
                ui.label(egui::RichText::new("Corner radius").color(Tokens::TEXT_DIM).size(11.0));
                let mut r = app.rect_radius;
                if ui.add(egui::DragValue::new(&mut r).speed(0.1).range(0.0..=500.0).suffix(" mm")).changed() {
                    app.rect_radius = r;
                    let cmds: Vec<Command> = shapes
                        .iter()
                        .filter_map(|s| match &s.kind {
                            ShapeKind::Rect { rect, .. } => Some(Command::SetShapeKind { shape: s.id, kind: ShapeKind::Rect { rect: *rect, radius: r } }),
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
                for (i, n, tip) in [(0, "Ellipse", "Whole ellipse"), (1, "Pie", "Pie wedge"), (2, "Arc", "Open arc")] {
                    if ui.selectable_label(mode == i, n).on_hover_text(tip).clicked() {
                        new_mode = i;
                    }
                }
                let mut arc = app.ellipse_arc.unwrap_or(tracedraw_core::EllipseArc { start_deg: 0.0, end_deg: 270.0, pie: true });
                let mut changed = new_mode != mode;
                if new_mode == 0 {
                    app.ellipse_arc = None;
                } else {
                    arc.pie = new_mode == 1;
                    ui.label(egui::RichText::new("Start").color(Tokens::TEXT_DIM).size(11.0));
                    changed |= ui.add(egui::DragValue::new(&mut arc.start_deg).speed(1.0).suffix("°")).changed();
                    ui.label(egui::RichText::new("End").color(Tokens::TEXT_DIM).size(11.0));
                    changed |= ui.add(egui::DragValue::new(&mut arc.end_deg).speed(1.0).suffix("°")).changed();
                    app.ellipse_arc = Some(arc);
                }
                if changed {
                    let new_arc = app.ellipse_arc;
                    let cmds: Vec<Command> = shapes
                        .iter()
                        .filter_map(|s| match &s.kind {
                            ShapeKind::Ellipse { rect, .. } => Some(Command::SetShapeKind { shape: s.id, kind: ShapeKind::Ellipse { rect: *rect, arc: new_arc } }),
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
                ui.label(egui::RichText::new("Points").color(Tokens::TEXT_DIM).size(11.0));
                let mut n = app.polygon_points;
                let mut changed = ui.add(egui::DragValue::new(&mut n).range(3..=500)).changed();
                let mut sh = app.star_sharpness;
                if app.tool == Tool::Star {
                    ui.label(egui::RichText::new("Sharpness").color(Tokens::TEXT_DIM).size(11.0));
                    changed |= ui.add(egui::DragValue::new(&mut sh).speed(0.01).range(0.0..=0.95)).changed();
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
                                kind: ShapeKind::Polygon { rect: *rect, points: n, sharpness: if star { sh } else { 0.0 } },
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
            Tool::Freehand | Tool::Bezier | Tool::Pen | Tool::Polyline | Tool::TwoPointLine | Tool::BSpline | Tool::ThreePointCurve => {
                object_properties(app, ui);
                vsep(ui);
                ui.label(egui::RichText::new("Enter or double-click finishes the curve, Esc cancels").color(Tokens::TEXT_DIM).size(11.0));
            }
            Tool::Blend => {
                ui.label(egui::RichText::new("Blend steps").color(Tokens::TEXT_DIM).size(11.0));
                ui.add(egui::DragValue::new(&mut app.blend_steps).range(1..=200));
                ui.label(egui::RichText::new("Drag from one object to another").color(Tokens::TEXT_DIM).size(11.0));
            }
            Tool::Extrude => {
                ui.label(egui::RichText::new("Extrude depth").color(Tokens::TEXT_DIM).size(11.0));
                let mut dx = app.extrude_depth.x;
                let mut dy = app.extrude_depth.y;
                ui.add(egui::DragValue::new(&mut dx).speed(0.2).suffix(" mm"));
                ui.add(egui::DragValue::new(&mut dy).speed(0.2).suffix(" mm"));
                app.extrude_depth = tracedraw_core::geometry::Vec2::new(dx, dy);
                vsep(ui);
                if ui.add_enabled(!shapes.is_empty(), egui::Button::new("Apply")).clicked() {
                    app.apply_extrude();
                }
            }
            Tool::Distort | Tool::Envelope => {
                use crate::tools2::DistortMode;
                for (m, n) in [(DistortMode::PushPull, "Push and Pull"), (DistortMode::Zipper, "Zipper"), (DistortMode::Twister, "Twister")] {
                    if ui.selectable_label(app.distort_mode == m, n).clicked() {
                        app.distort_mode = m;
                    }
                }
                ui.label(egui::RichText::new("Amplitude").color(Tokens::TEXT_DIM).size(11.0));
                ui.add(egui::DragValue::new(&mut app.distort_amount).speed(1.0).range(-200.0..=200.0));
                if app.distort_mode == DistortMode::Zipper {
                    ui.label(egui::RichText::new("Frequency").color(Tokens::TEXT_DIM).size(11.0));
                    ui.add(egui::DragValue::new(&mut app.distort_frequency).range(1..=100));
                }
                vsep(ui);
                if ui.add_enabled(!shapes.is_empty(), egui::Button::new("Apply")).clicked() {
                    app.apply_distort();
                }
                if app.tool == Tool::Envelope {
                    ui.label(egui::RichText::new("Envelope presets are not implemented yet; distortions are available here").color(Tokens::TEXT_DIM).size(11.0));
                }
            }
            Tool::Smooth | Tool::Smear | Tool::Twirl => {
                ui.label(egui::RichText::new("Nib size").color(Tokens::TEXT_DIM).size(11.0));
                ui.add(egui::DragValue::new(&mut app.brush_radius).speed(0.5).range(1.0..=200.0).suffix(" mm"));
                ui.label(egui::RichText::new("Drag over a selected object").color(Tokens::TEXT_DIM).size(11.0));
            }
            Tool::Contour => {
                use crate::tools2::ContourDirection;
                ui.label(egui::RichText::new("Contour").color(Tokens::TEXT_DIM).size(11.0));
                for (d, n) in [(ContourDirection::Inside, "Inside"), (ContourDirection::Outside, "Outside")] {
                    if ui.selectable_label(app.contour_direction == d, n).clicked() {
                        app.contour_direction = d;
                    }
                }
                ui.label(egui::RichText::new("Steps").color(Tokens::TEXT_DIM).size(11.0));
                ui.add(egui::DragValue::new(&mut app.contour_steps).range(1..=50));
                ui.label(egui::RichText::new("Offset").color(Tokens::TEXT_DIM).size(11.0));
                ui.add(egui::DragValue::new(&mut app.contour_offset).speed(0.1).range(0.05..=200.0).suffix(" mm"));
                let [r, g, b] = app.contour_color.to_rgb8();
                let mut rgb = [r, g, b];
                if ui.color_edit_button_srgb(&mut rgb).on_hover_text("Outermost contour colour").changed() {
                    app.contour_color = tracedraw_core::Color::rgb8(rgb[0], rgb[1], rgb[2]);
                }
                vsep(ui);
                if ui.add_enabled(!shapes.is_empty(), egui::Button::new("Apply")).clicked() {
                    app.apply_contour();
                }
                if shapes.is_empty() {
                    ui.label(egui::RichText::new("Select an object, drag to set the offset, then Apply").color(Tokens::TEXT_DIM).size(11.0));
                }
            }
            Tool::Crop => {
                ui.label(egui::RichText::new("Drag a rectangle to crop the selection (or everything) to it").color(Tokens::TEXT_DIM).size(11.0));
            }
            Tool::Knife => {
                ui.label(egui::RichText::new("Drag a line across objects to cut them in two").color(Tokens::TEXT_DIM).size(11.0));
            }
            Tool::Spiral => {
                ui.label(egui::RichText::new("Revolutions").color(Tokens::TEXT_DIM).size(11.0));
                ui.add(egui::DragValue::new(&mut app.spiral_revolutions).range(1..=100));
                if ui.selectable_label(!app.spiral_logarithmic, "Symmetrical").clicked() {
                    app.spiral_logarithmic = false;
                }
                if ui.selectable_label(app.spiral_logarithmic, "Logarithmic").clicked() {
                    app.spiral_logarithmic = true;
                }
            }
            Tool::CommonShapes => {
                ui.label(egui::RichText::new("Shape").color(Tokens::TEXT_DIM).size(11.0));
                egui::ComboBox::from_id_salt("common_shape").selected_text(app.common_shape.name()).show_ui(ui, |ui| {
                    for cs in crate::tools2::CommonShape::ALL {
                        if ui.selectable_label(app.common_shape == cs, cs.name()).clicked() {
                            app.common_shape = cs;
                        }
                    }
                });
                ui.label(egui::RichText::new("Drag to draw").color(Tokens::TEXT_DIM).size(11.0));
            }
            Tool::Table => {
                ui.label(egui::RichText::new("Rows").color(Tokens::TEXT_DIM).size(11.0));
                ui.add(egui::DragValue::new(&mut app.table_rows).range(1..=100));
                ui.label(egui::RichText::new("Columns").color(Tokens::TEXT_DIM).size(11.0));
                ui.add(egui::DragValue::new(&mut app.table_cols).range(1..=100));
                ui.label(egui::RichText::new("Drag to draw the table").color(Tokens::TEXT_DIM).size(11.0));
            }
            Tool::BrushStrokes => {
                ui.label(egui::RichText::new("Calligraphic").color(Tokens::TEXT_DIM).size(11.0));
                ui.label(egui::RichText::new("Width").color(Tokens::TEXT_DIM).size(11.0));
                ui.add(egui::DragValue::new(&mut app.media_width).speed(0.1).range(0.2..=50.0).suffix(" mm"));
                ui.label(egui::RichText::new("Nib angle").color(Tokens::TEXT_DIM).size(11.0));
                ui.add(egui::DragValue::new(&mut app.media_angle).speed(1.0).range(0.0..=180.0).suffix("°"));
            }
            Tool::ParallelDimension => {
                let n = app.dimension_points.len();
                let msg = match n {
                    0 => "Click the first point",
                    1 => "Click the second point",
                    _ => "Click where the dimension line should sit",
                };
                ui.label(egui::RichText::new(msg).color(Tokens::TEXT_DIM).size(11.0));
            }
            Tool::Connector => {
                ui.label(egui::RichText::new("Drag from one object to another").color(Tokens::TEXT_DIM).size(11.0));
            }
            Tool::DropShadow => {
                let first = shapes.first().and_then(|s| s.shadow);
                let mut sh = first.unwrap_or(app.shadow_default);
                let mut changed = false;
                ui.label(egui::RichText::new("Drop shadow").color(Tokens::TEXT_DIM).size(11.0));
                if ui.add_enabled(!shapes.is_empty() && first.is_none(), egui::Button::new("Apply")).clicked() {
                    app.set_shadow(Some(sh), false);
                }
                let mut opacity = (sh.opacity * 100.0).round();
                ui.label(egui::RichText::new("Opacity").color(Tokens::TEXT_DIM).size(11.0));
                if ui.add(egui::DragValue::new(&mut opacity).range(0.0..=100.0).suffix(" %")).changed() {
                    sh.opacity = opacity / 100.0;
                    changed = true;
                }
                ui.label(egui::RichText::new("Feathering").color(Tokens::TEXT_DIM).size(11.0));
                let mut blur = sh.blur;
                if ui.add(egui::DragValue::new(&mut blur).speed(0.1).range(0.0..=50.0).suffix(" mm")).changed() {
                    sh.blur = blur;
                    changed = true;
                }
                let mut dx = sh.offset.x;
                let mut dy = sh.offset.y;
                ui.label(egui::RichText::new("Offset").color(Tokens::TEXT_DIM).size(11.0));
                changed |= ui.add(egui::DragValue::new(&mut dx).speed(0.1).suffix(" mm")).changed();
                changed |= ui.add(egui::DragValue::new(&mut dy).speed(0.1).suffix(" mm")).changed();
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
                if ui.add_enabled(first.is_some(), egui::Button::new("Clear shadow")).clicked() {
                    app.set_shadow(None, false);
                }
                if shapes.is_empty() {
                    ui.label(egui::RichText::new("Drag from an object to set the shadow offset").color(Tokens::TEXT_DIM).size(11.0));
                }
            }
            Tool::Transparency => {
                let first = shapes.first().map(|s| s.opacity).unwrap_or(1.0);
                let mut transparency = ((1.0 - first) * 100.0).round();
                ui.label(egui::RichText::new("Uniform transparency").color(Tokens::TEXT_DIM).size(11.0));
                let r = ui.add_enabled(!shapes.is_empty(), egui::Slider::new(&mut transparency, 0.0..=100.0).suffix(" %"));
                if r.changed() {
                    app.set_opacity(1.0 - transparency / 100.0);
                }
                if shapes.is_empty() {
                    ui.label(egui::RichText::new("Click an object first").color(Tokens::TEXT_DIM).size(11.0));
                }
            }
            Tool::InteractiveFill | Tool::AreaFill | Tool::MeshFill => {
                ui.label(egui::RichText::new("Click an object to fill it with the default fill; drag across it for a fountain fill").color(Tokens::TEXT_DIM).size(11.0));
            }
            t => {
                ui.label(egui::RichText::new(format!("{} tool", t.name())).color(Tokens::TEXT_DIM).size(11.0));
                if !t.implemented() {
                    ui.label(egui::RichText::new("(not implemented yet)").color(Tokens::TEXT_DIM).size(11.0));
                }
            }
        }
    });
}

fn page_properties(app: &mut App, ui: &mut Ui) {
    let size = app.page_size();
    let presets: [(&str, Size); 4] = [
        ("A4", paper::A4),
        ("A3", paper::A3),
        ("Letter", paper::LETTER),
        ("Custom", size),
    ];
    let current = presets
        .iter()
        .find(|(_, s)| {
            ((s.width - size.width).abs() < 0.01 && (s.height - size.height).abs() < 0.01)
                || ((s.height - size.width).abs() < 0.01 && (s.width - size.height).abs() < 0.01)
        })
        .map(|(n, _)| *n)
        .unwrap_or("Custom");
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
        .on_hover_text("Portrait")
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
        .on_hover_text("Landscape")
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
        .selectable_label(true, "All pages")
        .on_hover_text("Apply page size to all pages");
    let _ = ui
        .selectable_label(false, "Current page")
        .on_hover_text("Apply page size to the current page only");
    vsep(ui);
    ui.label(
        egui::RichText::new("Units")
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
    if unit_value(ui, app, "Nudge", &mut nudge, 0.1) && nudge > 0.0 {
        app.nudge_mm = nudge;
    }
    let mut dx = app.duplicate_offset.x;
    let mut dy = app.duplicate_offset.y;
    unit_value(ui, app, "Dup. x", &mut dx, 0.1);
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
        egui::RichText::new("Scale")
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
        egui::RichText::new("Angle")
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
    if tb_button(ui, icons::Action::Mirror, "Mirror horizontally", true) {
        app.mirror(true);
    }
    if tb_button(ui, icons::Action::Flip, "Mirror vertically", true) {
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
        egui::RichText::new("Outline")
            .color(Tokens::TEXT_DIM)
            .size(11.0),
    );
    let label = match width {
        None => "None".to_string(),
        Some(w) if w <= tracedraw_core::Stroke::HAIRLINE + 1e-9 => "Hairline".into(),
        Some(w) => format!("{:.2} mm", w),
    };
    egui::ComboBox::from_id_salt("outline_width")
        .selected_text(label)
        .width(90.0)
        .show_ui(ui, |ui| {
            if ui.selectable_label(false, "None").clicked() {
                let shapes = app.selection.clone();
                app.run(Command::SetStroke {
                    shapes,
                    stroke: None,
                });
            }
            if ui.selectable_label(false, "Hairline").clicked() {
                app.apply_outline_width(tracedraw_core::Stroke::HAIRLINE);
            }
            for w in [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0, 8.0] {
                if ui.selectable_label(false, format!("{w:.2} mm")).clicked() {
                    app.apply_outline_width(w);
                }
            }
        });
    vsep(ui);
    if ui.button("To front").clicked() {
        app.order(0);
    }
    if ui.button("To back").clicked() {
        app.order(3);
    }
    vsep(ui);
    if ui
        .button("Convert to curves")
        .on_hover_text("Ctrl+Q")
        .clicked()
    {
        app.convert_to_curves();
    }
    let _ = Rect::ZERO;
}

fn text_properties(app: &mut App, ui: &mut Ui) {
    ui.label(
        egui::RichText::new("Font")
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
    vsep(ui);
    for (a, label, tip) in [
        (tracedraw_core::TextAlign::Left, "Left", "Align left"),
        (tracedraw_core::TextAlign::Center, "Center", "Align center"),
        (tracedraw_core::TextAlign::Right, "Right", "Align right"),
        (
            tracedraw_core::TextAlign::Justify,
            "Justify",
            "Full justify",
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
        let cmds: Vec<Command> = app
            .selected_shapes()
            .iter()
            .filter_map(|s| match &s.kind {
                ShapeKind::Text {
                    spans,
                    origin,
                    frame,
                    ..
                } => {
                    let mut spans = spans.clone();
                    for sp in spans.iter_mut() {
                        sp.font_family = app.text_font.clone();
                        sp.size_pt = app.text_size_pt;
                        sp.bold = app.text_bold;
                        sp.italic = app.text_italic;
                    }
                    Some(Command::SetShapeKind {
                        shape: s.id,
                        kind: ShapeKind::Text {
                            spans,
                            origin: *origin,
                            frame: *frame,
                            align: app.text_align,
                        },
                    })
                }
                _ => None,
            })
            .collect();
        if !cmds.is_empty() {
            let _ = app.engine.run_batch("Text Properties", &cmds);
        }
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
        "+ Node",
        "Add nodes (double-click a segment)",
        has_nodes,
    ) {
        app.add_node_midpoints();
    }
    if b(ui, "- Node", "Delete nodes (Delete)", has_nodes) {
        app.delete_selected_nodes();
    }
    vsep(ui);
    if b(ui, "Break", "Break curve at nodes", has_nodes) {
        app.break_selected_nodes();
    }
    if b(ui, "Close", "Close curve", has_curve) {
        app.close_selected_curves();
    }
    vsep(ui);
    if b(ui, "To line", "Convert segment to line", has_nodes) {
        app.selected_segments_to_line();
    }
    if b(ui, "To curve", "Convert segment to curve", has_nodes) {
        app.selected_segments_to_curve();
    }
    vsep(ui);
    let current = app.current_node_type();
    for (ty, name, tip) in [
        (
            NodeType::Cusp,
            "Cusp",
            "Cusp node: handles move independently",
        ),
        (
            NodeType::Smooth,
            "Smooth",
            "Smooth node: handles stay in line",
        ),
        (
            NodeType::Symmetrical,
            "Symm.",
            "Symmetrical node: handles equal and in line",
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
    if b(ui, "Reverse", "Reverse direction", has_curve) {
        app.reverse_selected_curves();
    }
    if b(ui, "Select all", "Select all nodes (Ctrl+A)", has_curve) {
        app.select_all_nodes();
    }
    vsep(ui);
    if b(ui, "Convert to curves", "Ctrl+Q", !app.selection.is_empty()) {
        app.convert_to_curves();
    }
    if !has_curve && !app.selection.is_empty() {
        ui.label(
            egui::RichText::new(
                "Double-click an object to convert it to curves and edit its nodes",
            )
            .color(Tokens::TEXT_DIM)
            .size(11.0),
        );
    }
}
