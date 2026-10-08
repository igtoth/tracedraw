//! Page navigator (left of the palette) and status bar (bottom): tool hint,
//! object information, fill and outline swatches, as in the editor 2019.

use crate::app::App;
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::dockers::kind_name;
use egui::{Sense, Stroke, Ui, Vec2};

pub fn navigator(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        let n = app.doc().pages.len();
        let idx = app.page_index();
        let nav = |ui: &mut Ui, g: &str, tip: &str| -> bool {
            let (r, resp) = ui.allocate_exact_size(Vec2::new(14.0, 16.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r, 2.0, Tokens::TOOL_HOVER);
            }
            let c = r.center();
            let s = Stroke::new(1.3, Tokens::ICON);
            let p = ui.painter();
            match g {
                "⊞" => {
                    p.rect_stroke(
                        egui::Rect::from_center_size(c, Vec2::new(8.0, 10.0)),
                        0.0,
                        Stroke::new(1.0, Tokens::ICON),
                        egui::epaint::StrokeKind::Middle,
                    );
                    p.line_segment([c + Vec2::new(-2.5, 0.0), c + Vec2::new(2.5, 0.0)], s);
                    p.line_segment([c + Vec2::new(0.0, -2.5), c + Vec2::new(0.0, 2.5)], s);
                }
                "◀" => {
                    p.add(egui::epaint::PathShape::convex_polygon(
                        vec![
                            c + Vec2::new(-3.0, 0.0),
                            c + Vec2::new(2.0, -4.0),
                            c + Vec2::new(2.0, 4.0),
                        ],
                        Tokens::ICON,
                        Stroke::NONE,
                    ));
                }
                "▶" => {
                    p.add(egui::epaint::PathShape::convex_polygon(
                        vec![
                            c + Vec2::new(3.0, 0.0),
                            c + Vec2::new(-2.0, -4.0),
                            c + Vec2::new(-2.0, 4.0),
                        ],
                        Tokens::ICON,
                        Stroke::NONE,
                    ));
                }
                "⏮" => {
                    p.line_segment([c + Vec2::new(-4.0, -4.0), c + Vec2::new(-4.0, 4.0)], s);
                    p.add(egui::epaint::PathShape::convex_polygon(
                        vec![
                            c + Vec2::new(-3.0, 0.0),
                            c + Vec2::new(3.0, -4.0),
                            c + Vec2::new(3.0, 4.0),
                        ],
                        Tokens::ICON,
                        Stroke::NONE,
                    ));
                }
                _ => {
                    p.line_segment([c + Vec2::new(4.0, -4.0), c + Vec2::new(4.0, 4.0)], s);
                    p.add(egui::epaint::PathShape::convex_polygon(
                        vec![
                            c + Vec2::new(3.0, 0.0),
                            c + Vec2::new(-3.0, -4.0),
                            c + Vec2::new(-3.0, 4.0),
                        ],
                        Tokens::ICON,
                        Stroke::NONE,
                    ));
                }
            }
            resp.on_hover_text(tip).clicked()
        };
        if nav(ui, "⊞", "Insert page before") {
            app.add_page();
        }
        if nav(ui, "⏮", "First page") {
            app.goto_page(0);
        }
        if nav(ui, "◀", "Previous page") && idx > 0 {
            app.goto_page(idx - 1);
        }
        ui.label(egui::RichText::new(format!("{} of {}", idx + 1, n)).size(11.0));
        if nav(ui, "▶", "Next page") {
            app.goto_page(idx + 1);
        }
        if nav(ui, "⏭", "Last page") {
            app.goto_page(n.saturating_sub(1));
        }
        if nav(ui, "⊞", "Insert page after") {
            app.add_page();
        }
        let names: Vec<(usize, String)> = app
            .doc()
            .pages
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.name.clone()))
            .collect();
        for (i, name) in names {
            let selected = i == idx;
            let r = ui.add(
                egui::Button::new(egui::RichText::new(name).size(11.0))
                    .fill(if selected {
                        Tokens::PAGE
                    } else {
                        Tokens::PANEL_DARK
                    })
                    .stroke(Stroke::new(1.0, Tokens::BORDER)),
            );
            if r.clicked() {
                app.goto_page(i);
            }
        }
    });
}

fn tool_hint(tool: Tool) -> &'static str {
    match tool {
        Tool::Pick | Tool::FreeformPick => "Next click for Drag/Scale; Second click for Rotate/Skew; Dbl-clicking tool selects all objects; Shift+click multi-selects; Alt+click digs",
        Tool::Shape => "Click or drag nodes; Dbl-click an object to convert it to curves",
        Tool::Zoom => "Click to zoom in; Shift+click or right-click to zoom out; drag to zoom to an area",
        Tool::Pan => "Drag to pan the view",
        Tool::Rectangle | Tool::ThreePointRectangle => "Drag to draw; Ctrl constrains to a square; Dbl-click the tool for a page frame",
        Tool::Ellipse | Tool::ThreePointEllipse => "Drag to draw; Ctrl constrains to a circle",
        Tool::Polygon | Tool::Star => "Drag to draw; set points on the property bar",
        Tool::Text => "Click for artistic text, then type; Esc finishes",
        Tool::Freehand => "Drag to draw a freehand curve",
        Tool::Bezier | Tool::Pen | Tool::Polyline | Tool::TwoPointLine | Tool::BSpline => "Click to add nodes; Dbl-click or Enter finishes; Esc cancels",
        Tool::InteractiveFill | Tool::AreaFill => "Click an object to apply the default fill; drag across it for a fountain fill",
        Tool::ColorEyedropper | Tool::AttributesEyedropper => "Click an object to sample its fill; click other objects to apply",
        Tool::Eraser => "Click an object to delete it",
        _ => "This tool is not implemented yet",
    }
}

pub fn status_bar(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let (r, resp) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::click());
        crate::ui::icons::draw_action(
            ui.painter(),
            r,
            crate::ui::icons::Action::Options,
            Tokens::ICON,
        );
        let _ = resp.on_hover_text("Status bar options");
        let shapes = app.selected_shapes();
        let info = if app.text_edit.is_some() {
            "Editing text".to_string()
        } else if shapes.is_empty() {
            if app.status.is_empty() {
                tool_hint(app.tool).to_string()
            } else {
                format!("{}    {}", app.status, tool_hint(app.tool))
            }
        } else if shapes.len() == 1 {
            let layer = app
                .doc()
                .shape(shapes[0].id)
                .map(|(l, _)| l.name.clone())
                .unwrap_or_default();
            format!("{} on {}", kind_name(&shapes[0].kind), layer)
        } else {
            format!("{} objects selected", shapes.len())
        };
        ui.label(egui::RichText::new(info).size(11.0));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (fill, stroke) = match shapes.first() {
                Some(s) => (s.fill.clone(), s.stroke.clone()),
                None => (app.default_fill.clone(), app.default_stroke.clone()),
            };
            let width_label = match &stroke {
                None => String::new(),
                Some(s) if s.width <= tracedraw_core::Stroke::HAIRLINE + 1e-9 => {
                    "Hairline".to_string()
                }
                Some(s) => format!("{:.2} {}", app.units.from_mm(s.width), app.units.short()),
            };
            let outline_text = match &stroke {
                None => "None".to_string(),
                Some(s) => format!(
                    "{}  {}",
                    crate::app::color_description(s.color),
                    width_label
                ),
            };
            ui.add_space(12.0);
            ui.label(egui::RichText::new(outline_text).size(11.0));
            swatch(
                ui,
                stroke.as_ref().map(|s| crate::canvas::to_color32(s.color)),
                "Outline",
            );
            let (r, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
            crate::ui::icons::draw(
                ui.painter(),
                r,
                crate::tools::Tool::Freehand,
                Tokens::TEXT_DIM,
            );
            ui.add_space(24.0);
            ui.label(egui::RichText::new(crate::app::fill_description(&fill)).size(11.0));
            swatch(ui, crate::app::fill_preview_color(&fill), "Fill");
            let (r, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
            crate::ui::icons::draw(
                ui.painter(),
                r,
                crate::tools::Tool::InteractiveFill,
                Tokens::TEXT_DIM,
            );
        });
    });
}

fn swatch(ui: &mut Ui, color: Option<egui::Color32>, tip: &str) {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(18.0, 14.0), Sense::hover());
    match color {
        Some(c) => {
            ui.painter().rect_filled(r, 1.0, c);
        }
        None => {
            ui.painter().rect_stroke(
                r,
                1.0,
                Stroke::new(1.0, Tokens::TEXT_DIM),
                egui::epaint::StrokeKind::Inside,
            );
            ui.painter().line_segment(
                [r.left_bottom(), r.right_top()],
                Stroke::new(1.0, Tokens::TEXT_DIM),
            );
        }
    }
    resp.on_hover_text(tip);
}
