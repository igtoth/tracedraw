//! The 3-point callout tool's property bar (also shown when callouts are
//! selected with the Pick tool, and while a new callout's text is typed)
//! and the preview of the callout being placed. Bar: outline width, line
//! style, the head at the tip, the shape round the text and the gap,
//! then what to click next. Changes apply to the selected callouts and
//! to the ones made next.

use crate::app::App;
use crate::callouts::CalloutShape;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::field::NumField;
use crate::ui::propbar::{dash_label, label_pic, outline_text, sep, Pic, DASHES};
use crate::ui::tips::RichTip;
use egui::{Color32, Painter, Stroke, Ui};
use tracedraw_core::Arrowhead;

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// A head at the start of a line, pointing left.
fn head_label(a: &Arrowhead) -> String {
    match a {
        Arrowhead::None => "\u{2500}".into(),
        Arrowhead::Arrow => "\u{25C0}\u{2500}".into(),
        Arrowhead::OpenArrow => "<\u{2500}".into(),
        Arrowhead::Circle => "\u{25CF}\u{2500}".into(),
        Arrowhead::Square => "\u{25A0}\u{2500}".into(),
        Arrowhead::Bar => "|\u{2500}".into(),
        Arrowhead::Diamond => "\u{25C6}\u{2500}".into(),
        Arrowhead::Custom { name, .. } => name.clone(),
    }
}

/// The bar.
pub fn callout_bar(app: &mut App, ui: &mut Ui) {
    let style = app.callout_style_shown();
    label_pic(ui, Pic::Pen, &tr("toolbar.outline_width"));
    let mut width = None;
    egui::ComboBox::from_id_salt("callout_width")
        .selected_text(outline_text(app, Some(style.width)))
        .width(88.0)
        .height(400.0)
        .show_ui(ui, |ui| {
            let hair = tracedraw_core::Stroke::HAIRLINE;
            if ui
                .selectable_label((style.width - hair).abs() < 1e-9, tr("toolbar.hairline"))
                .clicked()
            {
                width = Some(hair);
            }
            for w in crate::ui::propbar::outline_width_presets(app.outline_units()) {
                if ui
                    .selectable_label((style.width - w).abs() < 1e-9, outline_text(app, Some(w)))
                    .clicked()
                {
                    width = Some(w);
                }
            }
        })
        .response
        .rich_tip(crate::ui::tips::keyed("toolbar.outline_width"));
    if let Some(w) = width {
        app.set_callout_style(|s| s.width = w);
    }
    let mut dash = None;
    egui::ComboBox::from_id_salt("callout_dash")
        .selected_text(dash_label(&style.dash))
        .width(96.0)
        .show_ui(ui, |ui| {
            for d in DASHES {
                if ui
                    .selectable_label(style.dash.as_slice() == *d, dash_label(d))
                    .clicked()
                {
                    dash = Some(d.to_vec());
                }
            }
        })
        .response
        .rich_tip(tr("callout.line_style"));
    if let Some(d) = dash {
        app.set_callout_style(|s| s.dash = d.clone());
    }
    let mut head = None;
    egui::ComboBox::from_id_salt("callout_head")
        .selected_text(head_label(&style.head))
        .width(56.0)
        .show_ui(ui, |ui| {
            for a in Arrowhead::ALL {
                if ui
                    .selectable_label(style.head == a, head_label(&a))
                    .clicked()
                {
                    head = Some(a.clone());
                }
            }
        })
        .response
        .rich_tip(tr("callout.head"));
    if let Some(h) = head {
        app.set_callout_style(|s| s.head = h.clone());
    }
    sep(ui);
    dim(ui, tr("callout.shape"));
    let mut shape = style.shape;
    egui::ComboBox::from_id_salt("callout_shape")
        .selected_text(tr(shape.key()))
        .width(120.0)
        .show_ui(ui, |ui| {
            for s in CalloutShape::ALL {
                ui.selectable_value(&mut shape, s, tr(s.key()));
            }
        })
        .response
        .rich_tip(crate::ui::tips::keyed("callout.shape"));
    if shape != style.shape {
        app.set_callout_style(|s| s.shape = shape);
    }
    dim(ui, tr("callout.gap"));
    let mut gap = style.gap;
    let r = ui
        .add(
            NumField::new(&mut gap)
                .range(0.0..=100.0)
                .unit_mm(1.0)
                .suffix(" mm"),
        )
        .rich_tip(tr("callout.gap_tip"));
    if r.changed() {
        app.set_callout_style(|s| s.gap = gap);
    }
    if app.tool == Tool::Callout {
        sep(ui);
        dim(ui, next_step(app));
    }
}

/// What the tool wants clicked next.
fn next_step(app: &App) -> String {
    tr(match app.dimension_points.len() {
        0 => "callout.tip",
        1 => "callout.elbow",
        _ => "callout.text",
    })
}

/// The leader being placed, following the pointer, drawn thin.
pub fn draw_preview(app: &App, painter: &Painter) {
    if app.tool != Tool::Callout || app.dimension_points.is_empty() {
        return;
    }
    let Some(at) = app.pointer_page else {
        return;
    };
    let blue = Color32::from_rgb(40, 110, 220);
    let mut pts: Vec<egui::Pos2> = app
        .dimension_points
        .iter()
        .map(|p| app.view.to_screen(*p))
        .collect();
    pts.push(app.view.to_screen(at));
    painter.add(egui::Shape::line(pts, Stroke::new(1.0, blue)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Point;

    #[test]
    fn the_bar_and_the_preview_draw() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.tool = Tool::Callout;
        for pts in [
            vec![],
            vec![Point::new(0.0, 0.0)],
            vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)],
        ] {
            app.dimension_points = pts;
            app.pointer_page = Some(Point::new(15.0, 12.0));
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600.0, 400.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                ui.horizontal(|ui| callout_bar(&mut app, ui));
                draw_preview(&app, ui.painter());
            });
            out.textures_delta.clear();
        }
        assert_eq!(head_label(&Arrowhead::Arrow), "\u{25C0}\u{2500}");
        assert_eq!(next_step(&app), tr("callout.text"));
    }
}
