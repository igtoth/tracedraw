//! Page navigator (left of the palette) and status bar (bottom): tool hint,
//! object information, fill and outline swatches.

use crate::app::App;
use crate::i18n::{tr, trf};
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
        if nav(ui, "⊞", &tr("status.insert_page_before")) {
            app.add_page();
        }
        if nav(ui, "⏮", &tr("status.first_page")) {
            app.goto_page(0);
        }
        if nav(ui, "◀", &tr("status.previous_page")) && idx > 0 {
            app.goto_page(idx - 1);
        }
        ui.label(
            egui::RichText::new(trf(
                "status.page_n_of_m",
                &[("n", &(idx + 1).to_string()), ("m", &n.to_string())],
            ))
            .size(11.0),
        );
        if nav(ui, "▶", &tr("status.next_page")) {
            app.goto_page(idx + 1);
        }
        if nav(ui, "⏭", &tr("status.last_page")) {
            app.goto_page(n.saturating_sub(1));
        }
        if nav(ui, "⊞", &tr("status.insert_page_after")) {
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

fn tool_hint(tool: Tool) -> String {
    let key = match tool {
        Tool::Pick | Tool::FreeformPick => "hint.bar_pick",
        Tool::Shape => "hint.bar_shape",
        Tool::Zoom => "hint.bar_zoom",
        Tool::Pan => "hint.bar_pan",
        Tool::Rectangle | Tool::ThreePointRectangle => "hint.bar_rectangle",
        Tool::Ellipse | Tool::ThreePointEllipse => "hint.bar_ellipse",
        Tool::Polygon | Tool::Star => "hint.bar_polygon",
        Tool::Text => "hint.bar_text",
        Tool::Freehand => "hint.bar_freehand",
        Tool::Bezier | Tool::Pen | Tool::Polyline | Tool::TwoPointLine | Tool::BSpline => {
            "hint.bar_bezier"
        }
        Tool::InteractiveFill | Tool::AreaFill => "hint.bar_fill",
        Tool::ColorEyedropper | Tool::AttributesEyedropper => "hint.bar_eyedropper",
        Tool::Eraser => "hint.bar_eraser",
        Tool::Contour => "hint.bar_contour",
        Tool::Crop => "hint.bar_crop",
        Tool::Knife => "hint.bar_knife",
        Tool::Spiral => "hint.bar_spiral",
        Tool::CommonShapes => "hint.bar_common_shapes",
        Tool::Table => "hint.bar_table",
        Tool::BrushStrokes => "hint.bar_brush_strokes",
        Tool::ParallelDimension => "hint.bar_dimension",
        Tool::Connector => "hint.bar_connector",
        Tool::DropShadow => "hint.bar_drop_shadow",
        Tool::Transparency => "hint.bar_transparency",
        _ => "hint.bar_not_implemented",
    };
    tr(key)
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
        let _ = resp.on_hover_text(tr("status.bar_options"));
        let shapes = app.selected_shapes();
        let info = if app.text_edit.is_some() {
            tr("status.editing_text")
        } else if shapes.is_empty() {
            if app.status.is_empty() {
                tool_hint(app.tool)
            } else {
                format!("{}    {}", app.status, tool_hint(app.tool))
            }
        } else if shapes.len() == 1 {
            let layer = app
                .doc()
                .shape(shapes[0].id)
                .map(|(l, _)| l.name.clone())
                .unwrap_or_default();
            trf(
                "status.object_on_layer",
                &[("k", &kind_name(&shapes[0].kind)), ("l", &layer)],
            )
        } else {
            trf(
                "status.n_objects_selected",
                &[("n", &shapes.len().to_string())],
            )
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
                    tr("status.hairline")
                }
                Some(s) => format!("{:.2} {}", app.units.from_mm(s.width), app.units.short()),
            };
            let outline_text = match &stroke {
                None => tr("status.none"),
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
                &tr("status.swatch_outline"),
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
            swatch(
                ui,
                crate::app::fill_preview_color(&fill),
                &tr("status.swatch_fill"),
            );
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
