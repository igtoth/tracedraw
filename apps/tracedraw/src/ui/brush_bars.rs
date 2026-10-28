//! The property bars of the shaping brushes (Smooth, Smear, Twirl,
//! Attract and Repel, Smudge, Roughen) and their settings. Every bar
//! starts with the nib size; then:
//!
//! - Smooth: Rate, how far one pass simplifies.
//! - Smear and Smudge: Pressure, how far the curve follows the pointer.
//! - Twirl: Speed and the turn, counterclockwise or clockwise.
//! - Attract and Repel: Speed and the mode (Shift while dragging swaps it).
//! - Roughen: Frequency (spikes per 10 mm) and Spike size.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::field::NumField;
use crate::ui::tips::RichTip;
use egui::Ui;
use serde::{Deserialize, Serialize};

/// The brushes' settings, kept in the preferences.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrushSettings {
    /// Smooth: 1 to 100.
    pub smooth_rate: u32,
    /// Smear: 1 to 100.
    pub smear_pressure: u32,
    /// Twirl: 1 to 100, and the turn.
    pub twirl_speed: u32,
    pub twirl_clockwise: bool,
    /// Attract and Repel: 1 to 100, and the mode.
    pub attract_speed: u32,
    pub repel: bool,
    /// Smudge: 1 to 100.
    pub smudge_pressure: u32,
    /// Roughen: spikes per 10 mm, 1 to 10.
    pub roughen_frequency: u32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        BrushSettings {
            smooth_rate: 20,
            smear_pressure: 100,
            twirl_speed: 50,
            twirl_clockwise: false,
            attract_speed: 50,
            repel: false,
            smudge_pressure: 100,
            roughen_frequency: 5,
        }
    }
}

impl BrushSettings {
    /// Every value in its range (a preferences file edited by hand).
    pub fn clamped(self) -> Self {
        BrushSettings {
            smooth_rate: self.smooth_rate.clamp(1, 100),
            smear_pressure: self.smear_pressure.clamp(1, 100),
            twirl_speed: self.twirl_speed.clamp(1, 100),
            attract_speed: self.attract_speed.clamp(1, 100),
            smudge_pressure: self.smudge_pressure.clamp(1, 100),
            roughen_frequency: self.roughen_frequency.clamp(1, 10),
            ..self
        }
    }

    /// Smooth: how far one pass may simplify (mm): 0.05 to 1.5.
    pub fn smooth_tolerance(&self) -> f64 {
        0.05 + 1.45 * (self.smooth_rate.clamp(1, 100) - 1) as f64 / 99.0
    }

    /// Smear and Smudge: the share of the pointer's move the curve takes.
    pub fn pressure(p: u32) -> f64 {
        p.clamp(1, 100) as f64 / 100.0
    }

    /// Twirl: radians per mm the pointer moves, signed by the turn
    /// (counterclockwise positive): 50 is 0.05.
    pub fn twirl_rate(&self) -> f64 {
        let r = self.twirl_speed.clamp(1, 100) as f64 / 1000.0;
        if self.twirl_clockwise {
            -r
        } else {
            r
        }
    }

    /// Attract and Repel: the pull per mm the pointer moves (50 is 0.3),
    /// negative to push; Shift swaps the mode.
    pub fn attract_rate(&self, shift: bool) -> f64 {
        let r = self.attract_speed.clamp(1, 100) as f64 * 0.006;
        if self.repel != shift {
            -r
        } else {
            r
        }
    }

    /// Roughen: the spacing of the spikes along the curve (mm).
    pub fn roughen_spacing(&self) -> f64 {
        10.0 / self.roughen_frequency.clamp(1, 10) as f64
    }
}

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

fn percent(ui: &mut Ui, label: &str, tip: &str, v: &mut u32) {
    dim(ui, tr(label));
    ui.add(NumField::new(v).range(1..=100).suffix(" %"))
        .rich_tip(tr(tip));
}

/// The bar of the brush tool in use.
pub fn brush_bar(app: &mut App, ui: &mut Ui) {
    dim(ui, tr("toolbar.nib_size"));
    let mut w = app.units.from_mm(app.brush_radius);
    if ui
        .add(
            NumField::new(&mut w)
                .speed(0.5)
                .range(0.5..=500.0)
                .suffix(format!(" {}", app.units.short())),
        )
        .rich_tip(tr("toolbar.nib_size_tip"))
        .changed()
    {
        app.brush_radius = app.units.to_mm(w).clamp(0.5, 500.0);
    }
    crate::ui::propbar::sep(ui);
    let b = &mut app.brush;
    match app.tool {
        Tool::Smooth => percent(
            ui,
            "toolbar.rate",
            "toolbar.smooth_rate_tip",
            &mut b.smooth_rate,
        ),
        Tool::Smear => percent(
            ui,
            "toolbar.pressure",
            "toolbar.pressure_tip",
            &mut b.smear_pressure,
        ),
        Tool::Smudge => percent(
            ui,
            "toolbar.pressure",
            "toolbar.pressure_tip",
            &mut b.smudge_pressure,
        ),
        Tool::Twirl => {
            percent(
                ui,
                "toolbar.speed",
                "toolbar.twirl_speed_tip",
                &mut b.twirl_speed,
            );
            if ui
                .selectable_label(!b.twirl_clockwise, tr("toolbar.counterclockwise"))
                .rich_tip(crate::ui::tips::keyed("toolbar.counterclockwise"))
                .clicked()
            {
                b.twirl_clockwise = false;
            }
            if ui
                .selectable_label(b.twirl_clockwise, tr("toolbar.clockwise"))
                .rich_tip(crate::ui::tips::keyed("toolbar.clockwise"))
                .clicked()
            {
                b.twirl_clockwise = true;
            }
        }
        Tool::AttractRepel => {
            percent(
                ui,
                "toolbar.speed",
                "toolbar.attract_speed_tip",
                &mut b.attract_speed,
            );
            if ui
                .selectable_label(!b.repel, tr("toolbar.attract"))
                .rich_tip(tr("toolbar.attract_tip"))
                .clicked()
            {
                b.repel = false;
            }
            if ui
                .selectable_label(b.repel, tr("toolbar.repel"))
                .rich_tip(tr("toolbar.attract_tip"))
                .clicked()
            {
                b.repel = true;
            }
        }
        Tool::Roughen => {
            dim(ui, tr("toolbar.frequency"));
            ui.add(NumField::new(&mut b.roughen_frequency).range(1..=10))
                .rich_tip(tr("toolbar.roughen_frequency_tip"));
            dim(ui, tr("toolbar.spike_size"));
            let mut a = app.units.from_mm(app.roughen_amount);
            if ui
                .add(
                    NumField::new(&mut a)
                        .speed(0.05)
                        .range(0.05..=50.0)
                        .suffix(format!(" {}", app.units.short())),
                )
                .rich_tip(crate::ui::tips::keyed("toolbar.spike_size"))
                .changed()
            {
                app.roughen_amount = app.units.to_mm(a).clamp(0.05, 50.0);
            }
        }
        _ => {}
    }
    crate::ui::propbar::sep(ui);
    dim(ui, tr("toolbar.drag_over_selected"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brush_settings_scale_and_stay_in_range() {
        let b = BrushSettings::default();
        assert!((b.twirl_rate() - 0.05).abs() < 1e-12);
        assert!((b.attract_rate(false) - 0.3).abs() < 1e-12);
        assert!(b.attract_rate(true) < 0.0);
        let r = BrushSettings {
            repel: true,
            twirl_clockwise: true,
            ..b
        };
        assert!(r.attract_rate(false) < 0.0 && r.attract_rate(true) > 0.0);
        assert!(r.twirl_rate() < 0.0);
        assert!((b.roughen_spacing() - 2.0).abs() < 1e-12);
        assert!(b.smooth_tolerance() > 0.3 && b.smooth_tolerance() < 0.4);
        let odd = BrushSettings {
            smooth_rate: 0,
            roughen_frequency: 99,
            ..b
        }
        .clamped();
        assert_eq!((odd.smooth_rate, odd.roughen_frequency), (1, 10));
    }

    /// Each brush's bar draws.
    #[test]
    fn every_brush_bar_draws() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for t in [
            Tool::Smooth,
            Tool::Smear,
            Tool::Twirl,
            Tool::AttractRepel,
            Tool::Smudge,
            Tool::Roughen,
        ] {
            app.tool = t;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 80.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                ui.horizontal(|ui| brush_bar(&mut app, ui));
            });
            out.textures_delta.clear();
        }
    }
}
