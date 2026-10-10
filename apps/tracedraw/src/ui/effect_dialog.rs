//! The dialog of a bitmap effect: its settings (sliders, direction dials,
//! lists, check boxes, colours) with a before and after preview. OK adds
//! the effect to the selected bitmaps, or changes the one being edited
//! from the Properties docker's FX section. The Tone Curve filter shows
//! its curves instead of sliders.

use crate::app::App;
use crate::fx::{new_effect, ParamKind, ParamSpec};
use crate::i18n::tr;
use crate::ui::bitmap_preview::{before_after, before_after_from, settings_key, Source};
use egui::{Color32, Context, Sense, Stroke, Ui, Vec2};
use std::collections::BTreeMap;
use tracedraw_core::{BitmapEffect, ShapeId};

/// The dialog's state: the effect being set up and, when editing, the
/// bitmap and its place in the list; the Tone Curve's shown channel.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectState {
    pub effect: BitmapEffect,
    pub edit: Option<(ShapeId, usize)>,
    pub channel: usize,
}

impl EffectState {
    pub fn new(id: &str) -> Self {
        EffectState {
            effect: new_effect(id),
            edit: None,
            channel: 0,
        }
    }
}

/// A direction dial: click or drag around it to point it.
fn dial(ui: &mut Ui, deg: &mut f64) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click_and_drag());
    let c = rect.center();
    let r = rect.width() / 2.0 - 2.0;
    ui.painter().circle_filled(c, r, Color32::WHITE);
    ui.painter()
        .circle_stroke(c, r, Stroke::new(1.0, crate::theme::Tokens::BORDER));
    let a = (*deg as f32).to_radians();
    let tip = c + Vec2::new(a.cos(), -a.sin()) * (r - 2.0);
    ui.painter()
        .line_segment([c, tip], Stroke::new(2.0, crate::theme::Tokens::SELECTION));
    ui.painter()
        .circle_filled(c, 2.0, crate::theme::Tokens::SELECTION);
    let mut changed = false;
    if resp.clicked() || resp.dragged() {
        if let Some(p) = resp.interact_pointer_pos() {
            let d = p - c;
            if d.length() > 2.0 {
                *deg = (-(d.y) as f64)
                    .atan2(d.x as f64)
                    .to_degrees()
                    .rem_euclid(360.0)
                    .round();
                changed = true;
            }
        }
    }
    changed
}

/// One setting's control; true when it changed.
pub fn param_widget(ui: &mut Ui, spec: &ParamSpec, values: &mut BTreeMap<String, f64>) -> bool {
    let key = format!("fxp.{}", spec.name);
    let mut changed = false;
    match spec.kind {
        ParamKind::Range {
            min,
            max,
            default,
            step,
        } => {
            ui.label(tr(&key));
            let mut v = values.get(spec.name).copied().unwrap_or(default);
            let mut s = egui::Slider::new(&mut v, min..=max);
            if step >= 1.0 {
                s = s.integer();
            } else {
                s = s.step_by(step).max_decimals(2);
            }
            // Wide ranges (a blur radius of 0.1 to 250 pixels) would leave
            // the small values in the first few pixels of the track.
            if min > 0.0 && max / min >= 100.0 {
                s = s.logarithmic(true);
            }
            if ui.add(s).changed() {
                values.insert(spec.name.to_string(), v);
                changed = true;
            }
        }
        ParamKind::Angle { default } => {
            ui.label(tr(&key));
            let mut v = values.get(spec.name).copied().unwrap_or(default);
            ui.horizontal(|ui| {
                changed |= dial(ui, &mut v);
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut v)
                            .range(-360.0..=360.0)
                            .suffix(" \u{b0}"),
                    )
                    .changed();
            });
            if changed {
                values.insert(spec.name.to_string(), v);
            }
        }
        ParamKind::Choice { options, default } => {
            ui.label(tr(&key));
            let cur = values
                .get(spec.name)
                .map(|v| v.max(0.0) as usize)
                .unwrap_or(default)
                .min(options.len().saturating_sub(1));
            egui::ComboBox::from_id_salt(("fx_choice", spec.name))
                .selected_text(tr(&format!("fxo.{}", options[cur])))
                .width(170.0)
                .show_ui(ui, |ui| {
                    for (i, o) in options.iter().enumerate() {
                        if ui
                            .selectable_label(i == cur, tr(&format!("fxo.{o}")))
                            .clicked()
                        {
                            values.insert(spec.name.to_string(), i as f64);
                            changed = true;
                        }
                    }
                });
        }
        ParamKind::Check { default } => {
            ui.label("");
            let mut on = values.get(spec.name).map(|v| *v != 0.0).unwrap_or(default);
            if ui.checkbox(&mut on, tr(&key)).changed() {
                values.insert(spec.name.to_string(), if on { 1.0 } else { 0.0 });
                changed = true;
            }
        }
        ParamKind::Color { default } => {
            ui.label(tr(&key));
            let c = values
                .get(spec.name)
                .copied()
                .unwrap_or(default as f64)
                .max(0.0) as u32;
            let mut rgb = crate::fx::util::rgb(c);
            if ui.color_edit_button_srgb(&mut rgb).changed() {
                let v = ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | rgb[2] as u32;
                values.insert(spec.name.to_string(), v as f64);
                changed = true;
            }
        }
    }
    changed
}

fn tone_curve_part(ui: &mut Ui, st: &mut EffectState, hist: Option<&[u32; 256]>) {
    use crate::fx::adjust::{curve_from, curve_into};
    let channels = [
        ("rgb", "fxo.master"),
        ("r", "fxo.red"),
        ("g", "fxo.green"),
        ("b", "fxo.blue"),
    ];
    ui.horizontal(|ui| {
        ui.label(tr("fxp.channel"));
        for (i, (_, key)) in channels.iter().enumerate() {
            ui.selectable_value(&mut st.channel, i, tr(key));
        }
    });
    let (ch, _) = channels[st.channel.min(3)];
    let mut curve = curve_from(&st.effect.params, ch);
    let color = match ch {
        "r" => Color32::from_rgb(220, 40, 40),
        "g" => Color32::from_rgb(30, 160, 60),
        "b" => Color32::from_rgb(40, 80, 220),
        _ => Color32::from_gray(40),
    };
    if crate::ui::curve_edit::curve_editor(
        ui,
        egui::Id::new(("tone_curve", ch)),
        &mut curve,
        220.0,
        color,
        &[],
        hist,
    ) {
        curve_into(&mut st.effect.params, ch, &curve);
    }
    ui.horizontal(|ui| {
        let mut linear = !curve.smooth;
        if ui
            .checkbox(&mut linear, tr("dialog.curve_linear"))
            .changed()
        {
            curve.smooth = !linear;
            curve_into(&mut st.effect.params, ch, &curve);
        }
        if ui.button(tr("dialog.reset")).clicked() {
            curve_into(
                &mut st.effect.params,
                ch,
                &crate::bitmap_modes::ToneCurve::identity(),
            );
        }
        if ui.button(tr("dialog.invert")).clicked() {
            let pts: Vec<(f32, f32)> = curve
                .normalized()
                .iter()
                .map(|(x, y)| (*x, 255.0 - *y))
                .collect();
            curve.points = pts;
            curve_into(&mut st.effect.params, ch, &curve);
        }
    });
}

fn histogram(img: &image::RgbaImage) -> [u32; 256] {
    let mut h = [0u32; 256];
    for p in img.pixels().filter(|p| p[3] > 0) {
        let l = crate::fx::util::luma(p).round() as usize;
        h[l.min(255)] += 1;
    }
    h
}

pub fn effect_dialog(app: &mut App, ctx: &Context, st: &mut EffectState, close: &mut bool) {
    let Some(spec) = crate::fx::spec(&st.effect.id) else {
        *close = true;
        return;
    };
    let title = tr(&format!("fx.{}", spec.id));
    super::dialogs::window(ctx, title).show(ctx, |ui| {
        let effect = st.effect.clone();
        // Adding starts from what the bitmap shows now; editing, from what
        // the effects before this one made, and the after view runs the
        // ones after it too.
        let stack = st
            .edit
            .and_then(|(id, i)| app.bitmap_fx_stack(id).map(|s| (id, i, s)));
        let thumb = match &stack {
            Some((shape, index, stack)) => {
                let after: Vec<BitmapEffect> =
                    stack.effects.iter().skip(index + 1).cloned().collect();
                let src = Source::Stack {
                    shape: *shape,
                    stack,
                    index: *index,
                };
                before_after_from(
                    ui,
                    app,
                    &src,
                    settings_key(&(&effect, &after)),
                    220.0,
                    |img, scale| {
                        let mut out =
                            crate::fx::apply_effect(img, &crate::fx::scaled(&effect, scale));
                        for e in after.iter().filter(|e| e.visible) {
                            out = crate::fx::apply_effect(&out, &crate::fx::scaled(e, scale));
                        }
                        out
                    },
                )
            }
            None => before_after(
                ui,
                app,
                settings_key(&effect.params),
                220.0,
                |img, scale| crate::fx::apply_effect(img, &crate::fx::scaled(&effect, scale)),
            ),
        };
        ui.add_space(6.0);
        if spec.id == "tone_curve" {
            let hist = thumb.as_ref().map(|t| histogram(t));
            tone_curve_part(ui, st, hist.as_ref());
        } else if spec.params.is_empty() {
            ui.label(tr("dialog.no_settings"));
        } else {
            ui.spacing_mut().slider_width = 200.0;
            egui::ScrollArea::vertical()
                .max_height(340.0)
                .show(ui, |ui| {
                    egui::Grid::new("fx_params")
                        .num_columns(2)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
                            for p in spec.params {
                                param_widget(ui, p, &mut st.effect.params);
                                ui.end_row();
                            }
                        });
                });
            if ui.button(tr("dialog.reset")).clicked() {
                st.effect.params.clear();
            }
        }
        if super::dialogs::ok_cancel(ui, close) {
            let e = st.effect.clone();
            match st.edit {
                Some((id, i)) => app.edit_bitmap_effects(id, "Edit Effect", move |list| {
                    if let Some(slot) = list.get_mut(i) {
                        *slot = e;
                    }
                }),
                None => app.add_bitmap_effect(e),
            }
        }
    });
}

/// Choose an effect from the menu: effects without settings apply at
/// once, the others open their dialog.
pub fn choose_effect(app: &mut App, id: &str) {
    let Some(spec) = crate::fx::spec(id) else {
        return;
    };
    if spec.params.is_empty() && id != "tone_curve" {
        app.add_bitmap_effect(new_effect(id));
    } else {
        app.dialog = crate::ui::dialogs::Dialog::Effect(EffectState::new(id));
    }
}

/// The Properties docker's FX section for a bitmap: its effects with
/// show/hide, edit, move up and down and delete, and Add effect.
pub fn fx_section(app: &mut App, ui: &mut Ui, id: ShapeId) {
    let effects = app.bitmap_effects(id);
    if effects.is_empty() {
        ui.label(
            egui::RichText::new(tr("docker.no_effects"))
                .color(crate::theme::Tokens::TEXT_DIM)
                .size(11.0),
        );
    }
    let n = effects.len();
    for (i, e) in effects.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut vis = e.visible;
            if ui
                .checkbox(&mut vis, "")
                .on_hover_text(tr("docker.show_hide_effect"))
                .changed()
            {
                app.edit_bitmap_effects(id, "Show or Hide Effect", move |l| {
                    if let Some(x) = l.get_mut(i) {
                        x.visible = vis;
                    }
                });
            }
            ui.label(tr(&format!("fx.{}", e.id)));
            if ui
                .small_button("\u{270E}")
                .on_hover_text(tr("docker.edit_effect"))
                .clicked()
            {
                app.dialog = crate::ui::dialogs::Dialog::Effect(EffectState {
                    effect: e.clone(),
                    edit: Some((id, i)),
                    channel: 0,
                });
            }
            if ui
                .add_enabled(i > 0, egui::Button::new("\u{25B2}").small())
                .on_hover_text(tr("docker.move_up"))
                .clicked()
            {
                app.edit_bitmap_effects(id, "Reorder Effects", move |l| l.swap(i, i - 1));
            }
            if ui
                .add_enabled(i + 1 < n, egui::Button::new("\u{25BC}").small())
                .on_hover_text(tr("docker.move_down"))
                .clicked()
            {
                app.edit_bitmap_effects(id, "Reorder Effects", move |l| l.swap(i, i + 1));
            }
            if ui
                .small_button("\u{2715}")
                .on_hover_text(tr("docker.delete_effect"))
                .clicked()
            {
                app.edit_bitmap_effects(id, "Delete Effect", move |l| {
                    if i < l.len() {
                        l.remove(i);
                    }
                });
            }
        });
    }
    let resp = ui.button(tr("docker.add_effect"));
    egui::Popup::menu(&resp)
        .id(egui::Id::new("fx_add_menu"))
        .style(crate::ui::menus::menu_popup_style)
        .show(|ui| {
            crate::ui::menus::body(ui, |ui| {
                for (key, group) in crate::fx::groups() {
                    ui.menu_button(tr(key), |ui| {
                        for s in group {
                            if ui.button(tr(&format!("fx.{}", s.id))).clicked() {
                                choose_effect(app, s.id);
                                ui.close();
                            }
                        }
                    });
                }
            })
        });
}
