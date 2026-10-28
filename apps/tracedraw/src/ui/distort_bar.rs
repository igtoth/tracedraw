//! The Distort tool's property bar and the handles of the selected
//! distortions. Bar: Push and Pull, Zipper, Twister; the values of the
//! kind shown (the selected object's distortion, else the tool's); Center
//! Distortion, Clear Distortion, Convert to Curves; Apply for selected
//! objects without a distortion.

use crate::app::App;
use crate::distort_tool::{DistortMode, MAX_TURNS};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::field::NumField;
use crate::ui::propbar::sep;
use crate::ui::tips::RichTip;
use egui::{Color32, Painter, Stroke, Ui};

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// The bar.
pub fn distort_bar(app: &mut App, ui: &mut Ui) {
    // The selected distortion's values are the ones shown and edited.
    if let Some(d) = app.selected_distortion() {
        let mut s = app.distort;
        s.take(&d);
        app.distort = s;
    }
    let shown = app.distort.mode;
    for m in DistortMode::ALL {
        if ui
            .selectable_label(shown == m, tr(m.key()))
            .rich_tip(crate::ui::tips::keyed(m.key()))
            .clicked()
            && m != shown
        {
            app.set_distort_mode(m);
        }
    }
    sep(ui);
    let mut s = app.distort;
    let mut changed = false;
    match shown {
        DistortMode::PushPull => {
            dim(ui, tr("toolbar.amplitude"));
            changed |= ui
                .add(
                    NumField::new(&mut s.amount)
                        .range(-200.0..=200.0)
                        .max_decimals(1),
                )
                .rich_tip(tr("distort.amount_tip"))
                .changed();
        }
        DistortMode::Zipper => {
            dim(ui, tr("toolbar.amplitude"));
            changed |= ui
                .add(
                    NumField::new(&mut s.amplitude)
                        .range(0.0..=100.0)
                        .unit_mm(1.0)
                        .suffix(" mm"),
                )
                .rich_tip(crate::ui::tips::pair(
                    "toolbar.amplitude",
                    "distort.zipper_amplitude_desc",
                ))
                .changed();
            dim(ui, tr("toolbar.frequency"));
            changed |= ui
                .add(
                    NumField::new(&mut s.frequency)
                        .range(0..=tracedraw_core::distort::MAX_FREQUENCY)
                        .width(44.0),
                )
                .rich_tip(tr("distort.frequency_tip"))
                .changed();
            for (flag, key, tip) in [
                (&mut s.random, "distort.random", "distort.random_tip"),
                (&mut s.smooth, "distort.smooth", "distort.smooth_tip"),
                (&mut s.local, "distort.local", "distort.local_tip"),
            ] {
                if ui
                    .selectable_label(*flag, tr(key))
                    .rich_tip(tr(tip))
                    .clicked()
                {
                    *flag = !*flag;
                    changed = true;
                }
            }
        }
        DistortMode::Twister => {
            for (cw, key) in [
                (true, "distort.clockwise"),
                (false, "distort.counterclockwise"),
            ] {
                if ui
                    .selectable_label(s.clockwise == cw, tr(key))
                    .rich_tip(crate::ui::tips::keyed(key))
                    .clicked()
                    && s.clockwise != cw
                {
                    s.clockwise = cw;
                    changed = true;
                }
            }
            dim(ui, tr("distort.rotations"));
            changed |= ui
                .add(NumField::new(&mut s.turns).range(0..=MAX_TURNS).width(36.0))
                .rich_tip(crate::ui::tips::keyed("distort.rotations"))
                .changed();
            dim(ui, tr("distort.degrees"));
            changed |= ui
                .add(
                    NumField::new(&mut s.degrees)
                        .range(0.0..=359.0)
                        .max_decimals(0)
                        .suffix("\u{b0}")
                        .width(52.0),
                )
                .rich_tip(crate::ui::tips::keyed("distort.degrees"))
                .changed();
        }
    }
    if changed {
        app.distort = s;
        app.apply_distort_settings();
    }
    sep(ui);
    let distorted = app.selected_distortion().is_some();
    if ui
        .add_enabled(distorted, egui::Button::new(tr("distort.center")))
        .clicked()
    {
        app.center_distortion();
    }
    if ui
        .add_enabled(distorted, egui::Button::new(tr("distort.clear")))
        .clicked()
    {
        app.clear_distortion();
    }
    if ui
        .add_enabled(distorted, egui::Button::new(tr("distort.convert")))
        .clicked()
    {
        app.bake_distortion();
    }
    let undistorted = app.selected_shapes().iter().any(|sh| {
        crate::distort_tool::distortion_of(&sh.effects).is_none()
            && !matches!(
                sh.kind,
                tracedraw_core::ShapeKind::Bitmap { .. } | tracedraw_core::ShapeKind::Group { .. }
            )
    });
    if undistorted
        && ui
            .button(tr("docker.apply"))
            .rich_tip(tr("distort.apply_tip"))
            .clicked()
    {
        app.add_distortion();
    }
    sep(ui);
    dim(ui, tr("distort.hint"));
}

/// The handles of each selected distortion: the centre (a diamond), a
/// line to the end handle (a square for the amount or amplitude, a
/// circle for the Twister's angle) and the Zipper's frequency slider on
/// the line (#1122).
pub fn draw_handles(app: &App, painter: &Painter) {
    if app.tool != crate::tools::Tool::Distort {
        return;
    }
    let stroke = Stroke::new(1.0, Tokens::SELECTION);
    let r = 5.0;
    for h in app.distortion_handles() {
        let c = app.view.to_screen(h.center);
        let e = app.view.to_screen(h.end);
        if (e - c).length() > 1.0 {
            painter.line_segment([c, e], stroke);
            // The arrow head toward the end.
            let dir = (e - c).normalized();
            let side = egui::vec2(-dir.y, dir.x);
            let tip = e - dir * r;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    e - dir * 1.0,
                    tip - dir * 6.0 + side * 3.5,
                    tip - dir * 6.0 - side * 3.5,
                ],
                Tokens::SELECTION,
                Stroke::NONE,
            ));
        }
        if let Some(sl) = h.slider {
            let p = app.view.to_screen(sl);
            painter.rect_filled(
                egui::Rect::from_center_size(p, egui::vec2(3.0, 2.0 * r + 2.0)),
                0.0,
                Color32::WHITE,
            );
            painter.rect_stroke(
                egui::Rect::from_center_size(p, egui::vec2(3.0, 2.0 * r + 2.0)),
                0.0,
                stroke,
                egui::StrokeKind::Middle,
            );
        }
        match h.mode {
            DistortMode::Twister => {
                painter.circle(e, r, Color32::WHITE, stroke);
            }
            DistortMode::PushPull | DistortMode::Zipper => {
                let sq = egui::Rect::from_center_size(e, egui::vec2(2.0 * r, 2.0 * r));
                painter.rect_filled(sq, 0.0, Color32::WHITE);
                painter.rect_stroke(sq, 0.0, stroke, egui::StrokeKind::Middle);
            }
        }
        let pts = vec![
            c + egui::vec2(0.0, -r),
            c + egui::vec2(r, 0.0),
            c + egui::vec2(0.0, r),
            c + egui::vec2(-r, 0.0),
        ];
        painter.add(egui::Shape::convex_polygon(pts, Color32::WHITE, stroke));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{distort::Distort, geometry::Rect, ShapeKind};

    #[test]
    fn the_bar_draws_for_every_kind_and_shows_the_selection() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.tool = crate::tools::Tool::Distort;
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("square");
        let frame = |app: &mut App| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600.0, 300.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                ui.horizontal(|ui| distort_bar(app, ui));
                draw_handles(app, ui.painter());
            });
            out.textures_delta.clear();
        };
        for m in DistortMode::ALL {
            app.distort.mode = m;
            frame(&mut app);
        }
        // A selected twister shows as the bar's values.
        app.select(vec![id]);
        app.distort.mode = DistortMode::Twister;
        app.add_distortion();
        app.distort.mode = DistortMode::PushPull;
        frame(&mut app);
        assert_eq!(app.distort.mode, DistortMode::Twister);
        assert!(matches!(
            app.selected_distortion(),
            Some(Distort::Twister { .. })
        ));
        // The handles draw for every kind.
        for m in DistortMode::ALL {
            app.set_distort_mode(m);
            frame(&mut app);
            assert_eq!(app.distortion_handles().len(), 1);
        }
    }
}
