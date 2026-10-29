//! The Contour tool's property bar and the controls it shares with the
//! Contour panel: To centre, Inside, Outside; steps and offset; the
//! corners; the colour direction and the last step's fill and outline
//! colours; the object and colour accelerations. They show the selected
//! contour and edit the selected contours live.

use crate::app::App;
use crate::contour_tool::{blend_key, join_key, ContourDirection};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::field::NumField;
use crate::ui::fill_tool_bars::color_pick;
use crate::ui::propbar::sep;
use crate::ui::tips::RichTip;
use egui::Ui;
use tracedraw_core::shaping::Join;

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// An acceleration slider, -100 to 100 %.
fn accel(ui: &mut Ui, key: &str, v: &mut f64) -> bool {
    let mut p = (*v * 100.0).round() as i32;
    let r = crate::ui::toolbar::dim_labelled(ui, key, |ui| {
        ui.add(crate::ui::Slider::new(&mut p, -100..=100).suffix(" %"))
    });
    if r.changed() {
        *v = p as f64 / 100.0;
    }
    r.changed()
}

/// The controls, a group to a row.
pub fn contour_controls(app: &mut App, ui: &mut Ui) {
    app.sync_contour_settings();
    let palette = app.palette.clone();
    let mut s = app.contour;
    let mut changed = false;
    ui.horizontal(|ui| {
        for d in ContourDirection::ALL {
            if ui
                .selectable_label(s.direction == d, tr(d.key()))
                .rich_tip(crate::ui::tips::keyed(d.key()))
                .clicked()
                && s.direction != d
            {
                s.direction = d;
                changed = true;
            }
        }
    });
    ui.horizontal(|ui| {
        crate::ui::toolbar::dim_labelled_as(ui, "docker.steps", "contour.steps_desc", |ui| {
            ui.add_enabled_ui(s.direction != ContourDirection::ToCenter, |ui| {
                changed |= ui
                    .add(NumField::new(&mut s.steps).range(1..=999).width(44.0))
                    .changed();
            });
        });
        let mut o = app.units.world_from_mm(s.offset);
        if crate::ui::toolbar::dim_labelled_as(ui, "docker.offset", "contour.offset_desc", |ui| {
            ui.add(
                NumField::new(&mut o)
                    .range(0.0..=app.units.world_from_mm(1000.0))
                    .fixed_decimals(app.settings.precision.min(10) as usize)
                    .unit_mm(app.units.world_to_mm(1.0))
                    .suffix(format!(" {}", app.units.short()))
                    .width(76.0),
            )
        })
        .changed()
        {
            s.offset = app.units.world_to_mm(o);
            changed = true;
        }
        egui::ComboBox::from_id_salt("contour_corners")
            .selected_text(tr(join_key(s.corners)))
            .width(80.0)
            .show_ui(ui, |ui| {
                for j in [Join::Miter, Join::Round, Join::Bevel] {
                    if ui
                        .selectable_label(s.corners == j, tr(join_key(j)))
                        .clicked()
                        && s.corners != j
                    {
                        s.corners = j;
                        changed = true;
                    }
                }
            })
            .response
            .rich_tip(tr("contour.corners"));
    });
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("contour_blend")
            .selected_text(tr(blend_key(s.color_blend)))
            .width(110.0)
            .show_ui(ui, |ui| {
                for b in 0..3u8 {
                    if ui
                        .selectable_label(s.color_blend == b, tr(blend_key(b)))
                        .clicked()
                        && s.color_blend != b
                    {
                        s.color_blend = b;
                        changed = true;
                    }
                }
            })
            .response
            .rich_tip(tr("docker.color_blend"));
        changed |= color_pick(
            ui,
            &palette,
            "contour_fill_to",
            &mut s.fill_to,
            &tr("docker.fill_color"),
            true,
        );
        changed |= color_pick(
            ui,
            &palette,
            "contour_outline_to",
            &mut s.outline_to,
            &tr("docker.outline_color"),
            true,
        );
    });
    ui.horizontal(|ui| {
        changed |= accel(ui, "docker.object_acceleration", &mut s.accel_objects);
        changed |= accel(ui, "docker.color_acceleration", &mut s.accel_colors);
    });
    if changed {
        app.contour = s;
        app.apply_contour_settings();
    }
}

/// Apply (for selected objects with no contour), Clear Contour and Break
/// Apart.
pub fn contour_actions(app: &mut App, ui: &mut Ui) {
    let has = app.selected_contour().is_some();
    let bare = app.selected_shapes().iter().any(|s| {
        !s.effects
            .iter()
            .any(|e| matches!(e, tracedraw_core::live::Effect::Contour { .. }))
            && !matches!(
                s.kind,
                tracedraw_core::ShapeKind::Bitmap { .. } | tracedraw_core::ShapeKind::Group { .. }
            )
    });
    ui.horizontal(|ui| {
        if bare
            && ui
                .button(tr("docker.apply"))
                .rich_tip(crate::ui::tips::pair("docker.apply", "contour.apply_desc"))
                .clicked()
        {
            app.add_contour();
        }
        if ui
            .add_enabled(has, egui::Button::new(tr("contour.clear")))
            .rich_tip(crate::ui::tips::keyed("contour.clear"))
            .clicked()
        {
            app.clear_contour();
        }
        if ui
            .add_enabled(has, egui::Button::new(tr("docker.break_apart")))
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
pub fn contour_bar(app: &mut App, ui: &mut Ui) {
    contour_controls(app, ui);
    sep(ui);
    contour_actions(app, ui);
    if app.selection.is_empty() {
        sep(ui);
        dim(ui, tr("toolbar.contour_hint"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{geometry::Rect, ShapeKind};

    #[test]
    fn the_bar_and_the_panel_draw() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.tool = crate::tools::Tool::Contour;
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
                        contour_controls(app, ui);
                        contour_actions(app, ui);
                    });
                } else {
                    ui.horizontal(|ui| contour_bar(app, ui));
                }
            });
            out.textures_delta.clear();
        };
        for d in ContourDirection::ALL {
            app.contour.direction = d;
            frame(&mut app, false);
        }
        app.select(vec![id]);
        app.contour.direction = ContourDirection::Inside;
        app.add_contour();
        app.contour.direction = ContourDirection::Outside;
        frame(&mut app, true);
        assert_eq!(app.contour.direction, ContourDirection::Inside);
    }
}
