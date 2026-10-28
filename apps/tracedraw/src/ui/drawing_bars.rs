//! The property bars of Shape Recognition (how readily strokes become
//! shapes, how much free curves are smoothed), Sketch (whether and how
//! near a stroke joins the selected curve, how much it is smoothed) and the
//! Knife (a straight or freehand cut, a gap along it, the pieces kept as
//! one object).

use crate::app::App;
use crate::i18n::tr;
use crate::recognize::LEVEL_KEYS;
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::field::NumField;
use crate::ui::tips::RichTip;
use egui::Ui;

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// A level list, off to highest.
fn level(ui: &mut Ui, id: &str, tip: &str, v: &mut u8) {
    let mut cur = (*v).min(5);
    egui::ComboBox::from_id_salt(id)
        .selected_text(tr(LEVEL_KEYS[cur as usize]))
        .width(90.0)
        .show_ui(ui, |ui| {
            for (i, key) in LEVEL_KEYS.iter().enumerate() {
                ui.selectable_value(&mut cur, i as u8, tr(key));
            }
        })
        .response
        .rich_tip(tr(tip));
    *v = cur;
}

/// The bar of Shape Recognition or Sketch.
pub fn drawing_bar(app: &mut App, ui: &mut Ui) {
    match app.tool {
        Tool::ShapeRecognition => {
            dim(ui, tr("recognize.recognition"));
            level(
                ui,
                "recognize_level",
                "recognize.recognition_tip",
                &mut app.recognize.recognition,
            );
            dim(ui, tr("recognize.smoothing"));
            level(
                ui,
                "recognize_smoothing",
                "recognize.smoothing_tip",
                &mut app.recognize.smoothing,
            );
            crate::ui::propbar::sep(ui);
            dim(ui, tr("status_hint.shape_recognition"));
        }
        Tool::Sketch => {
            ui.checkbox(&mut app.sketch.join, tr("sketch.join"))
                .rich_tip(tr("sketch.join_tip"));
            let mut d = app.units.from_mm(app.sketch.join_distance);
            let r = ui.add_enabled(
                app.sketch.join,
                NumField::new(&mut d)
                    .speed(0.1)
                    .range(0.0..=1000.0)
                    .suffix(format!(" {}", app.units.short())),
            );
            let r = r
                .rich_tip(crate::ui::tips::keyed("sketch.join_distance"))
                .rich_disabled_tip(crate::ui::tips::keyed("sketch.join_distance"));
            if r.changed() {
                app.sketch.join_distance = app.units.to_mm(d).clamp(0.0, 1000.0);
            }
            dim(ui, tr("recognize.smoothing"));
            level(
                ui,
                "sketch_smoothing",
                "recognize.smoothing_tip",
                &mut app.sketch.smoothing,
            );
            crate::ui::propbar::sep(ui);
            dim(ui, tr("status_hint.sketch"));
        }
        _ => {}
    }
}

/// The Knife's bar.
pub fn knife_bar(app: &mut App, ui: &mut Ui) {
    use crate::knife::KnifeMode;
    for (mode, key) in [
        (KnifeMode::TwoPoint, "knife.two_point"),
        (KnifeMode::Freehand, "knife.freehand"),
    ] {
        if ui
            .selectable_label(app.knife.mode == mode, tr(key))
            .rich_tip(crate::ui::tips::keyed(key))
            .clicked()
        {
            app.knife.mode = mode;
        }
    }
    crate::ui::propbar::sep(ui);
    let split = app.knife.gap <= 0.0;
    if ui
        .selectable_label(split, tr("knife.split"))
        .rich_tip(tr("knife.split_tip"))
        .clicked()
    {
        app.knife.gap = 0.0;
    }
    if ui
        .selectable_label(!split, tr("knife.gap"))
        .rich_tip(tr("knife.gap_tip"))
        .clicked()
        && split
    {
        app.knife.gap = 1.0;
    }
    let mut w = app.units.from_mm(app.knife.gap);
    let r = ui.add_enabled(
        !split,
        NumField::new(&mut w)
            .speed(0.05)
            .range(0.0..=1000.0)
            .suffix(format!(" {}", app.units.short())),
    );
    let r = r
        .rich_tip(crate::ui::tips::keyed("knife.gap_width"))
        .rich_disabled_tip(crate::ui::tips::keyed("knife.gap_width"));
    if r.changed() {
        app.knife.gap = app.units.to_mm(w).clamp(0.0, 1000.0);
    }
    ui.checkbox(&mut app.knife.one_object, tr("knife.one_object"))
        .rich_tip(tr("knife.one_object_tip"));
    crate::ui::propbar::sep(ui);
    dim(
        ui,
        tr(match app.knife.mode {
            KnifeMode::TwoPoint => "toolbar.knife_hint",
            KnifeMode::Freehand => "knife.freehand_hint",
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_bars_draw() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for t in [
            Tool::ShapeRecognition,
            Tool::Sketch,
            Tool::Knife,
            Tool::Knife,
        ] {
            app.tool = t;
            app.knife.gap = 2.0;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 80.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                ui.horizontal(|ui| {
                    if app.tool == Tool::Knife {
                        knife_bar(&mut app, ui)
                    } else {
                        drawing_bar(&mut app, ui)
                    }
                });
            });
            out.textures_delta.clear();
        }
    }
}
