//! Page navigator (left of the palette) and status bar (bottom): tool hint,
//! object information, fill and outline swatches.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::dockers::kind_name;
use egui::{Sense, Stroke, Ui, Vec2};

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
            let mut kind = kind_name(&shapes[0].kind);
            if app.is_effect_clone(shapes[0].id) {
                kind = format!("{kind} ({})", tr("status.effect_clone"));
            }
            trf("status.object_on_layer", &[("k", &kind), ("l", &layer)])
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
