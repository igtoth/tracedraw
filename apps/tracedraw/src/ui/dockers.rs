//! Dockers: Properties, Objects (layer manager) and Hints.

use crate::app::{App, DockerTab};
use crate::theme::Tokens;
use crate::tools::Tool;
use egui::Ui;
use tracedraw_core::{document::ShapeKind, Color, Command, Fill, Stroke};

/// The vertical strip of docker tabs on the right edge of the window.
pub fn tab_strip(app: &mut App, ui: &mut Ui) {
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
    ui.add_space(4.0);
    for (tab, name) in [
        (DockerTab::Hints, "Hints"),
        (DockerTab::Properties, "Properties"),
        (DockerTab::Objects, "Objects"),
    ] {
        let active = app.show_dockers && app.docker_tab == tab;
        let h = 14.0 + name.len() as f32 * 7.0;
        let (r, resp) = ui.allocate_exact_size(egui::vec2(26.0, h), egui::Sense::click());
        if active {
            ui.painter().rect_filled(r, 2.0, Tokens::TOOL_ACTIVE);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, 2.0, Tokens::TOOL_HOVER);
        }
        let galley = ui.painter().layout_no_wrap(
            name.to_string(),
            egui::FontId::proportional(11.0),
            Tokens::TEXT,
        );
        let w = galley.size().x;
        let gh = galley.size().y;
        let mut ts = egui::epaint::TextShape::new(
            egui::pos2(r.center().x + gh / 2.0 - 1.0, r.center().y - w / 2.0),
            galley,
            Tokens::TEXT,
        );
        ts.angle = std::f32::consts::FRAC_PI_2;
        ui.painter().add(ts);
        if resp.clicked() {
            if active {
                app.show_dockers = false;
            } else {
                app.show_dockers = true;
                app.docker_tab = tab;
            }
        }
    }
    let _ = ui
        .add(
            egui::Button::new(egui::RichText::new("+").size(14.0).color(Tokens::TEXT_DIM))
                .frame(false),
        )
        .on_hover_text("Add docker");
}

pub fn dockers(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        let name = match app.docker_tab {
            DockerTab::Properties => "Properties",
            DockerTab::Objects => "Objects",
            DockerTab::Hints => "Hints",
        };
        ui.label(egui::RichText::new(name).size(12.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new("✕").frame(false))
                .on_hover_text("Close docker")
                .clicked()
            {
                app.show_dockers = false;
            }
            let _ = ui
                .add(egui::Button::new("»").frame(false))
                .on_hover_text("Collapse");
        });
    });
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| match app.docker_tab {
        DockerTab::Properties => properties(app, ui),
        DockerTab::Objects => objects(app, ui),
        DockerTab::Hints => hints(app, ui),
    });
}

fn color_row(ui: &mut Ui, label: &str, c: &mut Color) -> bool {
    let [r, g, b] = c.to_rgb8();
    let mut rgb = [r, g, b];
    let changed = ui
        .horizontal(|ui| {
            ui.label(label);
            let ch = ui.color_edit_button_srgb(&mut rgb).changed();
            ui.label(
                egui::RichText::new(crate::app::color_description(*c))
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
            );
            ch
        })
        .inner;
    if changed {
        *c = Color::rgb8(rgb[0], rgb[1], rgb[2]);
    }
    changed
}

fn properties(app: &mut App, ui: &mut Ui) {
    let shapes = app.selected_shapes();
    if shapes.is_empty() {
        ui.label(egui::RichText::new("No objects selected").color(Tokens::TEXT_DIM));
        ui.add_space(6.0);
        ui.strong("Default properties for new objects");
        let mut fill = app.default_fill.clone();
        if fill_editor(ui, &mut fill) {
            app.default_fill = fill;
        }
        ui.separator();
        let mut stroke = app.default_stroke.clone();
        if outline_editor(ui, &mut stroke) {
            app.default_stroke = stroke;
        }
        return;
    }
    let first = &shapes[0];
    ui.strong(format!(
        "{}{}",
        kind_name(&first.kind),
        if shapes.len() > 1 {
            format!(" and {} more", shapes.len() - 1)
        } else {
            String::new()
        }
    ));
    let b = app.selection_bounds().unwrap_or_default();
    ui.label(
        egui::RichText::new(format!(
            "x {:.2}  y {:.2}  w {:.2}  h {:.2} {}",
            app.units.from_mm(b.x0),
            app.units.from_mm(b.y0),
            app.units.from_mm(b.width()),
            app.units.from_mm(b.height()),
            app.units.short()
        ))
        .monospace()
        .size(11.0),
    );

    ui.separator();
    ui.collapsing("Fill", |ui| {
        let mut fill = first.fill.clone();
        if fill_editor(ui, &mut fill) {
            app.apply_fill(fill);
        }
    });
    ui.collapsing("Outline", |ui| {
        let mut stroke = first.stroke.clone();
        if outline_editor(ui, &mut stroke) {
            let shapes = app.selection.clone();
            app.run(Command::SetStroke { shapes, stroke });
        }
    });
    if let ShapeKind::Text { spans, .. } = &first.kind {
        ui.collapsing("Character", |ui| {
            if let Some(sp) = spans.first() {
                ui.label(format!(
                    "{} {} pt{}{}",
                    sp.font_family,
                    sp.size_pt,
                    if sp.bold { " Bold" } else { "" },
                    if sp.italic { " Italic" } else { "" }
                ));
            }
            ui.label(
                egui::RichText::new("Edit with the Text tool (F8)")
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
            );
        });
    }
    ui.collapsing("Summary", |ui| {
        for s in &shapes {
            ui.label(format!(
                "{}  id {}  {}",
                kind_name(&s.kind),
                s.id.raw(),
                s.name.clone().unwrap_or_default()
            ));
        }
    });
}

fn fill_editor(ui: &mut Ui, fill: &mut Fill) -> bool {
    let mut changed = false;
    let mut kind = match fill {
        Fill::None => 0,
        Fill::Solid(_) => 1,
        Fill::Linear { .. } => 2,
        Fill::Radial { .. } => 3,
    };
    ui.horizontal(|ui| {
        for (i, n) in ["None", "Uniform", "Linear", "Radial"].iter().enumerate() {
            if ui.selectable_label(kind == i, *n).clicked() && kind != i {
                kind = i;
                changed = true;
            }
        }
    });
    if changed {
        let base = match fill {
            Fill::Solid(c) => *c,
            Fill::Linear { from, .. } | Fill::Radial { from, .. } => *from,
            Fill::None => Color::cmyk_pct(0.0, 0.0, 0.0, 20.0),
        };
        *fill = match kind {
            0 => Fill::None,
            1 => Fill::Solid(base),
            2 => Fill::Linear {
                from: base,
                to: Color::WHITE,
                angle: 0.0,
            },
            _ => Fill::Radial {
                from: base,
                to: Color::WHITE,
                offset: tracedraw_core::geometry::Point::ZERO,
            },
        };
    }
    match fill {
        Fill::None => {}
        Fill::Solid(c) => changed |= color_row(ui, "Colour", c),
        Fill::Linear { from, to, angle } => {
            changed |= color_row(ui, "From", from);
            changed |= color_row(ui, "To", to);
            changed |= ui
                .add(
                    egui::Slider::new(angle, -180.0..=180.0)
                        .text("angle")
                        .suffix("°"),
                )
                .drag_stopped();
        }
        Fill::Radial { from, to, .. } => {
            changed |= color_row(ui, "From", from);
            changed |= color_row(ui, "To", to);
        }
    }
    changed
}

fn outline_editor(ui: &mut Ui, stroke: &mut Option<Stroke>) -> bool {
    let mut changed = false;
    let mut has = stroke.is_some();
    if ui.checkbox(&mut has, "Outline").changed() {
        *stroke = if has { Some(Stroke::default()) } else { None };
        changed = true;
    }
    if let Some(s) = stroke {
        changed |= color_row(ui, "Colour", &mut s.color);
        let mut hair = s.width <= Stroke::HAIRLINE + 1e-9;
        if ui.checkbox(&mut hair, "Hairline").changed() {
            s.width = if hair { Stroke::HAIRLINE } else { 0.5 };
            changed = true;
        }
        if !hair {
            changed |= ui
                .add(
                    egui::Slider::new(&mut s.width, 0.1..=25.0)
                        .text("width mm")
                        .logarithmic(true),
                )
                .drag_stopped();
        }
        ui.horizontal(|ui| {
            ui.label("Caps");
            for (c, n) in [
                (tracedraw_core::LineCap::Butt, "Butt"),
                (tracedraw_core::LineCap::Round, "Round"),
                (tracedraw_core::LineCap::Square, "Square"),
            ] {
                if ui.selectable_label(s.cap == c, n).clicked() {
                    s.cap = c;
                    changed = true;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Corners");
            for (j, n) in [
                (tracedraw_core::LineJoin::Miter, "Miter"),
                (tracedraw_core::LineJoin::Round, "Round"),
                (tracedraw_core::LineJoin::Bevel, "Bevel"),
            ] {
                if ui.selectable_label(s.join == j, n).clicked() {
                    s.join = j;
                    changed = true;
                }
            }
        });
        changed |= ui.checkbox(&mut s.behind_fill, "Behind fill").changed();
        changed |= ui
            .checkbox(&mut s.scale_with_object, "Scale with object")
            .changed();
    }
    changed
}

pub fn kind_name(k: &ShapeKind) -> String {
    match k {
        ShapeKind::Rect { .. } => "Rectangle".into(),
        ShapeKind::Ellipse { .. } => "Ellipse".into(),
        ShapeKind::Polygon {
            sharpness, points, ..
        } => {
            if *sharpness > 0.0 {
                format!("Star with {points} points")
            } else {
                format!("Polygon with {points} sides")
            }
        }
        ShapeKind::Path { path, .. } => format!(
            "Curve with {} nodes",
            path.elements()
                .iter()
                .filter(|e| !matches!(e, tracedraw_core::geometry::PathEl::ClosePath))
                .count()
        ),
        ShapeKind::Text { spans, .. } => format!(
            "Artistic Text: {}",
            spans
                .iter()
                .map(|s| s.text.as_str())
                .collect::<String>()
                .chars()
                .take(20)
                .collect::<String>()
        ),
        ShapeKind::Group { children } => format!("Group of {} objects", children.len()),
    }
}

fn objects(app: &mut App, ui: &mut Ui) {
    let doc = app.doc();
    let Ok(page) = doc.page(app.page) else { return };
    let mut toggles: Vec<(tracedraw_core::LayerId, bool, bool)> = Vec::new();
    let mut click: Option<tracedraw_core::ShapeId> = None;
    let mut add_layer = false;
    ui.horizontal(|ui| {
        ui.strong(&page.name);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("+ Layer").clicked() {
                add_layer = true;
            }
        });
    });
    for layer in page.layers.iter().rev() {
        let mut vis = layer.visible;
        let mut locked = layer.locked;
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut vis, "")
                .on_hover_text("Show/hide")
                .changed()
            {
                toggles.push((layer.id, vis, locked));
            }
            if ui
                .selectable_label(locked, "🔒")
                .on_hover_text("Lock")
                .clicked()
            {
                locked = !locked;
                toggles.push((layer.id, vis, locked));
            }
            ui.strong(&layer.name);
        });
        for s in layer.shapes.iter().rev() {
            let sel = app.selection.contains(&s.id);
            let fill = crate::app::fill_preview_color(&s.fill);
            ui.horizontal(|ui| {
                ui.add_space(18.0);
                let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                match fill {
                    Some(c) => {
                        ui.painter().rect_filled(r, 1.0, c);
                    }
                    None => {
                        ui.painter().rect_stroke(
                            r,
                            1.0,
                            egui::Stroke::new(1.0, Tokens::TEXT_DIM),
                            egui::epaint::StrokeKind::Inside,
                        );
                    }
                }
                if ui.selectable_label(sel, kind_name(&s.kind)).clicked() {
                    click = Some(s.id);
                }
            });
        }
    }
    for (layer, visible, locked) in toggles {
        app.run(Command::SetLayerVisible { layer, visible });
        app.run(Command::SetLayerLocked { layer, locked });
    }
    if add_layer {
        let page = app.page;
        let n = app
            .doc()
            .page(page)
            .map(|p| p.layers.len() + 1)
            .unwrap_or(1);
        app.run(Command::AddLayer {
            page,
            name: format!("Layer {n}"),
        });
    }
    if let Some(id) = click {
        if ui.input(|i| i.modifiers.shift) {
            if !app.selection.contains(&id) {
                app.selection.push(id);
            }
        } else {
            app.select(vec![id]);
        }
    }
}

fn hints(app: &mut App, ui: &mut Ui) {
    if app.tool == Tool::Pick && app.selection.is_empty() {
        ui.heading("Home");
        ui.add_space(4.0);
        ui.label("To display topics, perform an action with a tool or choose a topic from the following list.");
        ui.add_space(4.0);
        for t in [
            "Lines",
            "Connector lines",
            "Dimension lines",
            "Shapes",
            "Select objects",
            "Move, scale, and stretch objects",
            "Rotate and skew objects",
            "Shape objects",
            "Special effects",
            "Outline objects",
            "Fill objects",
            "Add text",
            "Get help",
        ] {
            ui.horizontal(|ui| {
                ui.label("•");
                let _ = ui.link(egui::RichText::new(t).color(Tokens::ACCENT));
            });
        }
        ui.add_space(12.0);
        ui.separator();
        ui.heading("Learn more");
        ui.label(egui::RichText::new("Help topic").strong());
        let _ = ui.link(egui::RichText::new("TraceDraw Help").color(Tokens::ACCENT));
        return;
    }
    ui.heading(app.tool.name());
    ui.add_space(4.0);
    let text = match app.tool {
        Tool::Pick => "Click an object to select it. Click again for rotate and skew handles. Drag the handles to size; Shift+click adds to the selection; drag on empty space for a marquee. Double-click a curve to switch to the Shape tool.",
        Tool::Shape => "Drag the nodes of a curve. Double-click a rectangle, ellipse or polygon to convert it to curves first.",
        Tool::Zoom => "Click to zoom in, Shift+click or right-click to zoom out, drag a box to zoom into it. F4 fits the drawing, Shift+F4 fits the page.",
        Tool::Pan => "Drag to move the view. The middle mouse button pans in any tool.",
        Tool::Freehand => "Drag to draw a freehand curve; it is smoothed when you release.",
        Tool::Bezier | Tool::Pen => "Click to place nodes. Double-click or press Enter to finish, Esc to cancel.",
        Tool::Polyline | Tool::TwoPointLine => "Click to place points. Double-click or Enter finishes.",
        Tool::Rectangle => "Drag to draw a rectangle. Ctrl constrains to a square. Double-click the tool for a page-sized rectangle.",
        Tool::Ellipse => "Drag to draw an ellipse. Ctrl constrains to a circle.",
        Tool::Polygon | Tool::Star => "Drag to draw. Set the number of points on the property bar.",
        Tool::Text => "Click on the page and type. Esc finishes. Click existing text to edit it.",
        Tool::InteractiveFill => "Click an object to apply the default fill; drag across it for a linear fountain fill.",
        Tool::ColorEyedropper => "Click an object to sample its fill, then click other objects to apply it. Esc cancels.",
        Tool::Eraser => "Click an object to delete it.",
        _ => "This tool is on the roadmap and not implemented yet.",
    };
    ui.label(text);
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "Palette: left click fills, right click outlines. Arrow keys nudge the selection.",
        )
        .color(Tokens::TEXT_DIM)
        .size(11.0),
    );
}
