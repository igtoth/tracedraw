//! The Blend tool's property bar: Steps, Rotation, Object acceleration and
//! Color acceleration, for the next blend and, live, for the selected one;
//! Clear Blend and Break Apart for a selected blend.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::field::NumField;
use crate::ui::tips::RichTip;
use egui::Ui;
use tracedraw_core::live::Effect;

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// The selection's blend (of a selected start or end object).
fn selected_blend(app: &App) -> Option<Effect> {
    app.selected_blends().into_iter().next().map(|(_, e)| e)
}

/// The bar.
pub fn blend_bar(app: &mut App, ui: &mut Ui) {
    let cur = selected_blend(app);
    let (mut steps, mut rotation, mut accel_o, mut accel_c) = match &cur {
        Some(Effect::Blend {
            steps,
            rotation,
            accel_objects,
            accel_colors,
            ..
        }) => (*steps, *rotation, *accel_objects, *accel_colors),
        _ => (
            app.blend_steps,
            app.blend_rotation,
            app.blend_accel_objects,
            app.blend_accel_colors,
        ),
    };
    let mut changed = false;
    changed |=
        crate::ui::toolbar::dim_labelled_as(ui, "docker.steps", "toolbar.blend_steps_desc", |ui| {
            ui.add(NumField::new(&mut steps).range(1..=999))
        })
        .changed();
    changed |= crate::ui::toolbar::dim_labelled_as(
        ui,
        "docker.rotation",
        "toolbar.blend_rotation_desc",
        |ui| {
            ui.add(
                NumField::new(&mut rotation)
                    .speed(1.0)
                    .range(-3600.0..=3600.0)
                    .suffix("\u{b0}"),
            )
        },
    )
    .changed();
    crate::ui::propbar::sep(ui);
    let pct = |ui: &mut Ui, key: &str, v: &mut f64| {
        let mut p = (*v * 100.0).round() as i32;
        let r = crate::ui::toolbar::dim_labelled(ui, key, |ui| {
            ui.add(crate::ui::Slider::new(&mut p, -100..=100).suffix(" %"))
        });
        if r.changed() {
            *v = p as f64 / 100.0;
        }
        r.changed()
    };
    changed |= pct(ui, "docker.object_acceleration", &mut accel_o);
    changed |= pct(ui, "docker.color_acceleration", &mut accel_c);
    app.blend_steps = steps.clamp(1, 999);
    app.blend_rotation = rotation;
    app.blend_accel_objects = accel_o;
    app.blend_accel_colors = accel_c;
    crate::ui::propbar::sep(ui);
    match cur {
        Some(Effect::Blend { .. }) => {
            if changed {
                let steps = steps.clamp(1, 999);
                app.change_blends("Blend", |e| {
                    let mut e = e.clone();
                    if let Effect::Blend {
                        steps: s,
                        accel_objects,
                        accel_colors,
                        rotation: r,
                        ..
                    } = &mut e
                    {
                        *s = steps;
                        *accel_objects = accel_o;
                        *accel_colors = accel_c;
                        *r = rotation;
                    }
                    Some(e)
                });
            }
            if ui
                .button(tr("docker.clear_blend"))
                .rich_tip(crate::ui::tips::keyed("docker.clear_blend"))
                .clicked()
            {
                app.clear_blend();
            }
            if ui
                .button(tr("docker.break_apart"))
                .rich_tip(crate::ui::tips::pair(
                    "docker.break_apart",
                    "toolbar.effect_break_apart_desc",
                ))
                .clicked()
            {
                app.break_effect_groups();
            }
        }
        _ => dim(ui, tr("toolbar.drag_between_objects")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{Rect, ShapeKind};

    /// The bar draws with and without a blend; a blend's settings change
    /// live as one undo step for a run of changes.
    #[test]
    fn the_blend_bar_edits_the_selected_blend() {
        let mut app = App::headless();
        let a = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("a");
        let b = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(50.0, 0.0, 60.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("b");
        let ctx = crate::theme::ui_context();
        let draw = |app: &mut App| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 80.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                ui.horizontal(|ui| blend_bar(app, ui));
            });
            out.textures_delta.clear();
        };
        app.tool = crate::tools::Tool::Blend;
        draw(&mut app);
        app.select(vec![a, b]);
        app.blend_selection(5, 0.0, 0.0, 0.0);
        app.select(vec![a]);
        draw(&mut app);
        let depth = app.engine.history_labels().0.len();
        // Three changes in a row from the bar: one step.
        for n in [6, 7, 8] {
            app.change_blends("Blend", |e| {
                let mut e = e.clone();
                if let Effect::Blend { steps, .. } = &mut e {
                    *steps = n;
                }
                Some(e)
            });
        }
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        assert!(matches!(
            selected_blend(&app),
            Some(Effect::Blend { steps: 8, .. })
        ));
        app.clear_blend();
        assert!(selected_blend(&app).is_none());
    }
}
