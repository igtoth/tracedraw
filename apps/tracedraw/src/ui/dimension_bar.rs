//! The dimension tools' property bar (also shown when dimensions are
//! selected with the Pick tool) and the preview of the dimension being
//! placed. Bar: Precision, Units, Show units, Prefix, Suffix, Text
//! position, Text size; the look; more options (extension lines,
//! horizontal text); then what to click next. Changes
//! apply to the selected dimensions and to the ones made next.

use crate::app::App;
use crate::dimensions::{units_choices, ArrowStyle, DimensionStyle, TextPosition};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::field::NumField;
use crate::ui::tips::RichTip;
use egui::{Color32, Painter, Stroke, Ui};
use tracedraw_core::geometry::PathEl;

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// What a precision looks like: 1.23 with that many decimals.
fn precision_sample(p: u8) -> String {
    let p = p.min(6) as usize;
    format!("{:.p$}", 1.234_567)
}

/// A text field that applies when it loses focus (or Enter).
fn text_field(
    app: &mut App,
    ui: &mut Ui,
    key: &str,
    shown: &str,
    apply: impl Fn(&mut DimensionStyle, String),
) {
    let id = ui.id().with(("dimension_text", key));
    let mut buf = ui
        .data_mut(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| shown.to_string());
    let r = ui
        .add(egui::TextEdit::singleline(&mut buf).desired_width(56.0))
        .rich_tip(tr(key));
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, buf.clone()));
    }
    if r.lost_focus() {
        ui.data_mut(|d| d.remove::<String>(id));
        if buf != shown {
            app.set_dimension_style(|s| apply(s, buf.clone()));
        }
    }
}

/// The look of the dimension (#475): the label's font and colour, the
/// lines' colour and width, the arrowheads and rounded ends.
fn style_part(app: &mut App, ui: &mut Ui, style: &DimensionStyle) {
    crate::ui::propbar::sep(ui);
    let current = if style.font.trim().is_empty() {
        app.text_default_font.clone()
    } else {
        style.font.clone()
    };
    let (pick, _) = crate::ui::tips::tip_over(
        ui,
        crate::ui::tips::pair("toolbar.font", "dimension.font_desc"),
        |ui| crate::ui::font_list::font_list_live(app, ui, "dimension_font", &current, 130.0),
    );
    if let Some(p) = pick {
        app.set_dimension_style(|s| s.font = p.family.clone());
    }
    let mut text_color = style.text_color;
    if crate::ui::color_picker::color_button(ui, &mut text_color) && text_color != style.text_color
    {
        app.set_dimension_style(|s| s.text_color = text_color);
    }
    crate::ui::propbar::sep(ui);
    dim(ui, tr("dimension.line"));
    let mut line_color = style.line_color;
    if crate::ui::color_picker::color_button(ui, &mut line_color) && line_color != style.line_color
    {
        app.set_dimension_style(|s| s.line_color = line_color);
    }
    let u = app.outline_units();
    let mut width = u.from_mm(style.line_width);
    let r = ui
        .add(
            NumField::new(&mut width)
                .speed(u.from_mm(0.05))
                .range(0.0..=u.from_mm(50.0))
                .max_decimals(3)
                .unit_mm(u.mm())
                .suffix(format!(" {}", u.short()))
                .width(64.0),
        )
        .rich_tip(tr("dimension.line_width"));
    let width_mm = u.to_mm(width);
    if r.changed() && (width_mm - style.line_width).abs() > 1e-9 {
        app.set_dimension_style(|s| s.line_width = width_mm);
    }
    let mut arrows = style.arrows;
    egui::ComboBox::from_id_salt("dimension_arrows")
        .selected_text(tr(arrows.key()))
        .width(90.0)
        .show_ui(ui, |ui| {
            for a in ArrowStyle::ALL {
                ui.selectable_value(&mut arrows, a, tr(a.key()));
            }
        })
        .response
        .rich_tip(tr("dimension.arrows"));
    if arrows != style.arrows {
        app.set_dimension_style(|s| s.arrows = arrows);
    }
    let mut round = style.round_ends;
    if ui
        .checkbox(&mut round, tr("dimension.round_ends"))
        .rich_tip(crate::ui::tips::keyed("dimension.round_ends"))
        .changed()
    {
        app.set_dimension_style(|s| s.round_ends = round);
    }
}

/// A length as printed, mm, in the drawing's units; applied when it
/// changes.
fn length_field(
    app: &mut App,
    ui: &mut Ui,
    key: &str,
    mm: f64,
    max: f64,
    apply: impl Fn(&mut DimensionStyle, f64),
) {
    let u = app.units;
    ui.horizontal(|ui| {
        ui.label(tr(key));
        let mut v = u.from_mm(mm);
        let r = ui.add(
            NumField::new(&mut v)
                .speed(u.from_mm(0.1))
                .range(0.0..=u.from_mm(max))
                .max_decimals(3)
                .unit_mm(u.mm())
                .suffix(format!(" {}", u.short()))
                .width(64.0),
        );
        let next = u.to_mm(v);
        if r.changed() && next.is_finite() && (next - mm).abs() > 1e-9 {
            app.set_dimension_style(|s| apply(s, next));
        }
    });
}

/// More options (#644): the extension lines' gap from the object and how
/// far they run past the dimension line, a horizontal label, and the
/// lines joined in rounded corners (#641).
fn options_part(app: &mut App, ui: &mut Ui, style: &DimensionStyle) {
    let button = ui
        .button(format!("{} \u{25BE}", tr("dimension.options")))
        .rich_tip(tr("dimension.options"));
    // The menu stays open while its fields are typed in and stepped; a
    // click outside closes it.
    egui::Popup::menu(&button)
        .id(egui::Id::new("dimension_options_menu"))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            length_field(
                app,
                ui,
                "dimension.extension_gap",
                style.extension_gap,
                100.0,
                |s, v| s.extension_gap = v,
            );
            length_field(
                app,
                ui,
                "dimension.extension_overshoot",
                style.extension_overshoot,
                100.0,
                |s, v| s.extension_overshoot = v,
            );
            let mut horizontal = style.horizontal_text;
            if ui
                .checkbox(&mut horizontal, tr("dimension.horizontal_text"))
                .changed()
            {
                app.set_dimension_style(|s| s.horizontal_text = horizontal);
            }
            // Lines joined in corners, rounded (#641).
            let mut joined = style.joined;
            if ui
                .checkbox(&mut joined, tr("dimension.join_lines"))
                .changed()
            {
                app.set_dimension_style(|s| s.joined = joined);
            }
            ui.add_enabled_ui(style.joined, |ui| {
                length_field(
                    app,
                    ui,
                    "dimension.corner_radius",
                    style.corner_radius,
                    100.0,
                    |s, v| s.corner_radius = v,
                );
            });
        });
}

/// The bar.
pub fn dimension_bar(app: &mut App, ui: &mut Ui) {
    // The first selected dimension, read once a frame.
    let first = app.first_selected_dimension();
    let style = first
        .map(|d| d.style)
        .unwrap_or_else(|| app.dimension_style.clone());
    dim(ui, tr("dimension.precision"));
    let mut precision = style.precision.min(6);
    egui::ComboBox::from_id_salt("dimension_precision")
        .selected_text(precision_sample(precision))
        .width(80.0)
        .show_ui(ui, |ui| {
            for p in 0..=6u8 {
                ui.selectable_value(&mut precision, p, precision_sample(p));
            }
        })
        .response
        .rich_tip(crate::ui::tips::keyed("dimension.precision"));
    if precision != style.precision.min(6) {
        app.set_dimension_style(|s| s.precision = precision);
    }
    dim(ui, tr("dimension.units"));
    let choices = units_choices();
    let mut units = style.units.clone();
    let shown_units = choices
        .iter()
        .find(|(id, _)| *id == units)
        .map(|(_, l)| l.clone())
        .unwrap_or_default();
    egui::ComboBox::from_id_salt("dimension_units")
        .selected_text(shown_units)
        .width(120.0)
        .show_ui(ui, |ui| {
            for (id, label) in &choices {
                ui.selectable_value(&mut units, id.clone(), label);
            }
        })
        .response
        .rich_tip(crate::ui::tips::keyed("dimension.units"));
    if units != style.units {
        app.set_dimension_style(|s| s.units = units.clone());
    }
    let mut show = style.show_units;
    if ui
        .checkbox(&mut show, tr("dimension.show_units"))
        .rich_tip(crate::ui::tips::keyed("dimension.show_units"))
        .changed()
    {
        app.set_dimension_style(|s| s.show_units = show);
    }
    crate::ui::propbar::sep(ui);
    dim(ui, tr("dimension.prefix"));
    text_field(app, ui, "dimension.prefix", &style.prefix, |s, v| {
        s.prefix = v
    });
    dim(ui, tr("dimension.suffix"));
    text_field(app, ui, "dimension.suffix", &style.suffix, |s, v| {
        s.suffix = v
    });
    crate::ui::propbar::sep(ui);
    dim(ui, tr("dimension.text_position"));
    let mut pos = style.text_position;
    egui::ComboBox::from_id_salt("dimension_text_position")
        .selected_text(tr(pos.key()))
        .width(90.0)
        .show_ui(ui, |ui| {
            for p in TextPosition::ALL {
                ui.selectable_value(&mut pos, p, tr(p.key()));
            }
        })
        .response
        .rich_tip(crate::ui::tips::keyed("dimension.text_position"));
    if pos != style.text_position {
        app.set_dimension_style(|s| s.text_position = pos);
    }
    let mut size = style.text_size;
    let r = ui
        .add(
            NumField::new(&mut size)
                .speed(0.5)
                .range(1.0..=500.0)
                .suffix(" pt"),
        )
        .rich_tip(tr("dimension.text_size"));
    if r.changed() && (size - style.text_size).abs() > 1e-9 {
        app.set_dimension_style(|s| s.text_size = size);
    }
    style_part(app, ui, &style);
    options_part(app, ui, &style);
    if matches!(
        app.tool,
        Tool::ParallelDimension
            | Tool::HorizontalVerticalDimension
            | Tool::SegmentDimension
            | Tool::AngularDimension
    ) {
        crate::ui::propbar::sep(ui);
        dim(ui, next_step(app));
    }
}

/// What the tool wants clicked next.
fn next_step(app: &App) -> String {
    let n = app.dimension_points.len();
    if app.tool == Tool::AngularDimension {
        return tr(match n {
            0 => "dimension.angular_vertex",
            1 => "dimension.angular_first",
            2 => "dimension.angular_second",
            _ => "dimension.angular_place",
        });
    }
    tr(match (n, app.tool) {
        (0, Tool::SegmentDimension) => "dimension.segment_first",
        (0, _) => "toolbar.dimension_first_point",
        (1, _) => "toolbar.dimension_second_point",
        _ => "toolbar.dimension_place_line",
    })
}

/// The dimension being placed, following the pointer, drawn thin.
pub fn draw_preview(app: &App, painter: &Painter) {
    let Some(at) = app.pointer_page else {
        return;
    };
    // Ctrl keeps the leg being given to steps of the constrain angle.
    let ctrl = painter
        .ctx()
        .input(|i| i.modifiers.command || i.modifiers.ctrl);
    let at = app.dimension_point(at, ctrl);
    let blue = Color32::from_rgb(40, 110, 220);
    // The first leg follows the pointer while the second point is wanted.
    if let (
        [first],
        Tool::ParallelDimension
        | Tool::HorizontalVerticalDimension
        | Tool::SegmentDimension
        | Tool::AngularDimension,
    ) = (&app.dimension_points[..], app.tool)
    {
        painter.line_segment(
            [app.view.to_screen(*first), app.view.to_screen(at)],
            Stroke::new(1.0, blue),
        );
        return;
    }
    let Some(d) = app.dimension_preview(at) else {
        return;
    };
    let ids = [tracedraw_core::ShapeId(0); 3];
    for s in d.shapes(ids, &app.text_font, app.units) {
        let path = s.page_path();
        let mut line: Vec<egui::Pos2> = Vec::new();
        let flush = |line: &mut Vec<egui::Pos2>| {
            if line.len() > 1 {
                painter.add(egui::Shape::line(
                    std::mem::take(line),
                    Stroke::new(1.0, blue),
                ));
            }
            line.clear();
        };
        tracedraw_core::geometry::flatten(&path, 0.05, &mut |el| match el {
            PathEl::MoveTo(p) => {
                flush(&mut line);
                line.push(app.view.to_screen(p));
            }
            PathEl::LineTo(p) => line.push(app.view.to_screen(p)),
            PathEl::ClosePath => {
                if let Some(first) = line.first().copied() {
                    line.push(first);
                }
            }
            _ => {}
        });
        flush(&mut line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Point;

    #[test]
    fn the_bar_and_the_preview_draw() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for (tool, pts) in [
            (Tool::ParallelDimension, vec![]),
            (Tool::ParallelDimension, vec![Point::new(0.0, 0.0)]),
            (
                Tool::HorizontalVerticalDimension,
                vec![Point::new(0.0, 0.0), Point::new(30.0, 0.0)],
            ),
            (
                Tool::AngularDimension,
                vec![
                    Point::new(0.0, 0.0),
                    Point::new(10.0, 0.0),
                    Point::new(0.0, 10.0),
                ],
            ),
        ] {
            app.tool = tool;
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
                ui.horizontal(|ui| dimension_bar(&mut app, ui));
                draw_preview(&app, ui.painter());
            });
            out.textures_delta.clear();
        }
        assert_eq!(precision_sample(0), "1");
        assert_eq!(precision_sample(3), "1.235");
    }
}
