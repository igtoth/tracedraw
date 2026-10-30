//! The dialog of a bitmap effect: a window the user sizes with the
//! shared Before and After preview (`before_after.rs`) at the left and
//! the settings (sliders, direction dials, lists, check boxes, colours)
//! at the right. OK adds the effect to the selected bitmaps, or changes
//! the one being edited from the Properties docker's FX section. The
//! Tone Curve filter shows its curves instead of sliders.

use crate::app::App;
use crate::fx::{new_effect, ParamKind, ParamSpec};
use crate::i18n::tr;
use crate::ui::before_after::{self, View, BOTTOM_H, RIGHT_W};
use crate::ui::bitmap_preview::{settings_key, Source};
use crate::ui::tips::RichTip;
use egui::{Color32, Context, Rect, Sense, Stroke, Ui, Vec2};
use std::collections::BTreeMap;
use tracedraw_core::{BitmapEffect, ShapeId};

/// The dialog's state: the effect being set up and, when editing, the
/// bitmap and its place in the list; the Tone Curve's shown channel.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectState {
    pub effect: BitmapEffect,
    pub edit: Option<(ShapeId, usize)>,
    pub channel: usize,
    /// The Image Adjustments's view, history and snapshots.
    pub lab: crate::ui::lab_dialog::LabState,
    /// The colour setting an eyedropper is picking from the preview.
    pub pick: Option<String>,
    /// The Preview box: the effect shows on the drawing while the dialog
    /// is open (see `fx::preview`).
    pub preview: bool,
}

impl EffectState {
    pub fn new(id: &str) -> Self {
        let mut lab = crate::ui::lab_dialog::LabState::default();
        // Image Adjustments shows the result alone at first; the other
        // dialogs the picture over the result.
        if id != "image_adjustments" {
            lab.view = Default::default();
        }
        EffectState {
            effect: new_effect(id),
            edit: None,
            channel: 0,
            lab,
            pick: None,
            preview: PREVIEW_AT_START,
        }
    }
}

/// The Preview box as an effect dialog opens: ticked on the desktop; in
/// the browser the preview is made on the interface thread, so it starts
/// cleared there and opening an effect never stalls the page.
pub const PREVIEW_AT_START: bool = !crate::files::WEB;

/// A direction dial: click or drag around it to point it.
fn dial(ui: &mut Ui, deg: &mut f64) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click_and_drag());
    let c = rect.center();
    let r = rect.width() / 2.0 - 2.0;
    ui.painter().circle_filled(c, r, Color32::WHITE);
    ui.painter()
        .circle_stroke(c, r, Stroke::new(1.0, crate::theme::Tokens::border()));
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

/// Colour settings that are colours of the image: they get an
/// eyedropper that picks them from the preview.
pub const PICK_PARAMS: [&str; 4] = ["low_sample", "mid_sample", "high_sample", "old_color"];

/// One setting's control; true when it changed. `pick` is the colour
/// setting whose eyedropper is on, if the dialog has eyedroppers.
/// The control of `spec` in the settings panel's column: a slider with
/// its label above it, stretched to the column; a list, a colour or a
/// dial with its label at the left; a check box alone.
pub fn param_stacked(
    ui: &mut Ui,
    spec: &ParamSpec,
    values: &mut BTreeMap<String, f64>,
    pick: Option<&mut Option<String>>,
) -> bool {
    let key = format!("fxp.{}", spec.name);
    // The setting's own description, or what every setting of its kind does.
    let tip = || {
        let own = format!("{key}_desc");
        let desc = if crate::i18n::has_english(&own) {
            tr(&own)
        } else {
            tr(match spec.kind {
                ParamKind::Range { .. } => "fx.param_range_desc",
                ParamKind::Angle { .. } => "fx.param_angle_desc",
                ParamKind::Choice { .. } => "fx.param_choice_desc",
                ParamKind::Check { .. } => "fx.param_check_desc",
                ParamKind::Color { .. } => "fx.param_color_desc",
            })
        };
        format!("{}\n{desc}", tr(&key))
    };
    let mut changed = false;
    match spec.kind {
        ParamKind::Range {
            min,
            max,
            default,
            step,
        } => {
            ui.label(format!("{}:", tr(&key)));
            let mut v = values.get(spec.name).copied().unwrap_or(default);
            let mut s = crate::ui::Slider::new(&mut v, min..=max);
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
            let resp = crate::ui::tips::tip_over(ui, tip(), |ui| ui.add(s));
            if resp.changed() {
                values.insert(spec.name.to_string(), v);
                changed = true;
            }
        }
        ParamKind::Angle { default } => {
            let mut v = values.get(spec.name).copied().unwrap_or(default);
            let draw = |ui: &mut Ui| {
                ui.label(tr(&key));
                crate::ui::tips::tip_over(ui, tip(), |ui| {
                    ui.horizontal(|ui| {
                        changed |= dial(ui, &mut v);
                        changed |= ui
                            .add(
                                crate::ui::field::NumField::new(&mut v)
                                    .range(-360.0..=360.0)
                                    .suffix(" \u{b0}"),
                            )
                            .changed();
                    })
                });
            };
            ui.horizontal(draw);
            if changed {
                values.insert(spec.name.to_string(), v);
            }
        }
        ParamKind::Choice { options, default } => {
            let cur = values
                .get(spec.name)
                .map(|v| v.max(0.0) as usize)
                .unwrap_or(default)
                .min(options.len().saturating_sub(1));
            let draw = |ui: &mut Ui| {
                ui.label(tr(&key));
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
                    })
                    .response
                    .rich_tip_with(tip);
            };
            ui.horizontal(draw);
        }
        ParamKind::Check { default } => {
            let mut on = values.get(spec.name).map(|v| *v != 0.0).unwrap_or(default);
            if ui.checkbox(&mut on, tr(&key)).rich_tip_with(tip).changed() {
                values.insert(spec.name.to_string(), if on { 1.0 } else { 0.0 });
                changed = true;
            }
        }
        ParamKind::Color { default } => {
            let c = values
                .get(spec.name)
                .copied()
                .unwrap_or(default as f64)
                .max(0.0) as u32;
            let mut rgb = crate::fx::util::rgb(c);
            let draw = |ui: &mut Ui| {
                ui.label(tr(&key));
                ui.horizontal(|ui| {
                    if ui
                        .color_edit_button_srgb(&mut rgb)
                        .rich_tip_with(tip)
                        .changed()
                    {
                        let v = ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | rgb[2] as u32;
                        values.insert(spec.name.to_string(), v as f64);
                        changed = true;
                    }
                    if let Some(pick) = pick {
                        if PICK_PARAMS.contains(&spec.name) {
                            let on = pick.as_deref() == Some(spec.name);
                            let (rect, resp) =
                                ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
                            crate::ui::propbar::frame(ui, rect, resp.hovered(), on);
                            crate::ui::icons::draw(
                                ui.painter(),
                                Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
                                crate::tools::Tool::ColorEyedropper,
                                crate::theme::Tokens::icon(),
                            );
                            if resp.rich_tip(tr("dialog.pick_from_image")).clicked() {
                                *pick = if on {
                                    None
                                } else {
                                    Some(spec.name.to_string())
                                };
                            }
                        }
                    }
                });
            };
            ui.horizontal(draw);
        }
    }
    changed
}

/// The Tone Curve eyedropper's name in [`EffectState::pick`]: a click on
/// the preview adds a node at that pixel's level.
const NODE_PICK: &str = "tone_curve_node";

/// Where a picked Tone Curve preset file arrives.
const PRESET_PICK: &str = "tone_curve_preset";

/// The settings in a Tone Curve preset file: its numbers by name (other
/// values are left out).
pub fn read_preset(text: &str) -> Option<BTreeMap<String, f64>> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let obj = v.as_object()?;
    Some(
        obj.iter()
            .filter_map(|(k, v)| v.as_f64().filter(|x| x.is_finite()).map(|x| (k.clone(), x)))
            .collect(),
    )
}

const CURVE_CHANNELS: [(&str, &str); 4] = [
    ("rgb", "fxo.master"),
    ("r", "fxo.red"),
    ("g", "fxo.green"),
    ("b", "fxo.blue"),
];

fn channel_color(ch: &str) -> Color32 {
    match ch {
        "r" => Color32::from_rgb(220, 40, 40),
        "g" => Color32::from_rgb(30, 160, 60),
        "b" => Color32::from_rgb(40, 80, 220),
        _ => Color32::from_gray(40),
    }
}

/// The Tone Curve dialog's controls: channel and style lists, the curve
/// (drawn as its style edits), the eyedropper that adds nodes, Smooth,
/// Invert, Reset Active Channel, Reset, Auto Balance Tone and Display
/// all channels. `picked` is the preview pixel the eyedropper took.
fn tone_curve_part(
    ui: &mut Ui,
    st: &mut EffectState,
    hist: Option<&[u32; 256]>,
    image: Option<&image::RgbaImage>,
    picked: Option<image::Rgba<u8>>,
    status: &mut String,
) {
    use crate::bitmap_modes::ToneCurve;
    use crate::fx::adjust::{
        balance_curves, curve_from, curve_gamma, curve_into, curve_style, freehand_points,
        set_curve_style, smoothed, CURVE_STYLES,
    };
    let params = &mut st.effect.params;
    let idx = st.channel.min(3);
    let (ch, _) = CURVE_CHANNELS[idx];
    let mut style = curve_style(params, ch);
    ui.horizontal(|ui| {
        ui.label(tr("fxp.channel"));
        egui::ComboBox::from_id_salt("tone_curve_channel")
            .selected_text(tr(CURVE_CHANNELS[idx].1))
            .width(110.0)
            .show_ui(ui, |ui| {
                for (i, (_, key)) in CURVE_CHANNELS.iter().enumerate() {
                    ui.selectable_value(&mut st.channel, i, tr(key));
                }
            })
            .response
            .rich_tip(crate::ui::tips::pair(
                "fxp.channel",
                "dialog.curve_channel_desc",
            ));
        ui.add_space(10.0);
        ui.label(tr("dialog.curve_style"));
        egui::ComboBox::from_id_salt("tone_curve_style")
            .selected_text(tr(&format!("dialog.curve_style_{}", CURVE_STYLES[style])))
            .width(110.0)
            .show_ui(ui, |ui| {
                for (i, name) in CURVE_STYLES.iter().enumerate() {
                    if ui
                        .selectable_label(i == style, tr(&format!("dialog.curve_style_{name}")))
                        .clicked()
                    {
                        // Freehand and gamma start from what the curve is now.
                        if i == 2 && style != 2 {
                            let c = curve_from(params, ch);
                            let lut: [f32; 256] = std::array::from_fn(|x| c.eval(x as f32));
                            curve_into(
                                params,
                                ch,
                                &ToneCurve {
                                    points: freehand_points(&lut),
                                    smooth: false,
                                },
                            );
                        }
                        set_curve_style(params, ch, i);
                        style = i;
                    }
                }
            })
            .response
            .rich_tip(crate::ui::tips::keyed("dialog.curve_style"));
    });
    // The pixel the eyedropper took becomes a node (curve and straight).
    if let Some(px) = picked {
        if style < 2 {
            let level = if ch == "rgb" {
                crate::fx::util::luma(&px)
            } else {
                px[["r", "g", "b"].iter().position(|c| *c == ch).unwrap_or(0)] as f32
            };
            let mut c = curve_from(params, ch);
            let y = c.eval(level);
            c.points.push((level, y));
            curve_into(params, ch, &c);
            set_curve_style(params, ch, style);
        }
    }
    let show_all = ui
        .data(|d| d.get_temp::<bool>(egui::Id::new("tone_curve_all")))
        .unwrap_or(false);
    let others: Vec<(ToneCurve, Color32)> = if show_all {
        CURVE_CHANNELS
            .iter()
            .filter(|(c, _)| *c != ch)
            .map(|(c, _)| (curve_from(params, c), channel_color(c).gamma_multiply(0.6)))
            .collect()
    } else {
        Vec::new()
    };
    let others_ref: Vec<(&ToneCurve, Color32)> = others.iter().map(|(c, col)| (c, *col)).collect();
    let color = channel_color(ch);
    ui.horizontal_top(|ui| {
        let id = egui::Id::new(("tone_curve", ch));
        match style {
            3 => {
                let mut g = curve_gamma(params, ch);
                if crate::ui::curve_edit::gamma_editor(ui, &mut g, 220.0, color, &others_ref, hist)
                {
                    params.insert(format!("{ch}gamma"), g);
                }
            }
            2 => {
                let mut c = curve_from(params, ch);
                if crate::ui::curve_edit::freehand_editor(
                    ui,
                    id,
                    &mut c,
                    220.0,
                    color,
                    &others_ref,
                    hist,
                ) {
                    curve_into(params, ch, &c);
                    set_curve_style(params, ch, 2);
                }
            }
            _ => {
                let mut c = curve_from(params, ch);
                if crate::ui::curve_edit::curve_editor(
                    ui,
                    id,
                    &mut c,
                    220.0,
                    color,
                    &others_ref,
                    hist,
                ) {
                    curve_into(params, ch, &c);
                    set_curve_style(params, ch, style);
                }
            }
        }
        ui.vertical(|ui| {
            let on = st.pick.as_deref() == Some(NODE_PICK);
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
            crate::ui::propbar::frame(ui, rect, resp.hovered(), on);
            crate::ui::icons::draw(
                ui.painter(),
                Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
                crate::tools::Tool::ColorEyedropper,
                if style < 2 {
                    crate::theme::Tokens::icon()
                } else {
                    crate::theme::Tokens::border()
                },
            );
            if resp.rich_tip(tr("dialog.curve_eyedropper")).clicked() && style < 2 {
                st.pick = if on {
                    None
                } else {
                    Some(NODE_PICK.to_string())
                };
            }
            if ui
                .add_enabled(style == 2, egui::Button::new(tr("dialog.smooth")))
                .rich_tip(crate::ui::tips::keyed("dialog.smooth"))
                .rich_disabled_tip(crate::ui::tips::keyed("dialog.smooth"))
                .clicked()
            {
                let lut = smoothed(&curve_from(params, ch));
                curve_into(
                    params,
                    ch,
                    &ToneCurve {
                        points: freehand_points(&lut),
                        smooth: false,
                    },
                );
                set_curve_style(params, ch, 2);
            }
            if ui
                .add_enabled(style != 3, egui::Button::new(tr("dialog.invert")))
                .rich_tip(crate::ui::tips::keyed("dialog.invert"))
                .rich_disabled_tip(crate::ui::tips::keyed("dialog.invert"))
                .clicked()
            {
                let c = curve_from(params, ch);
                let inverted = ToneCurve {
                    points: c
                        .normalized()
                        .iter()
                        .map(|(x, y)| (*x, 255.0 - *y))
                        .collect(),
                    smooth: c.smooth,
                };
                curve_into(params, ch, &inverted);
                set_curve_style(params, ch, style);
            }
            if ui
                .button(tr("dialog.reset_active_channel"))
                .rich_tip(crate::ui::tips::keyed("dialog.reset_active_channel"))
                .clicked()
            {
                params.retain(|k, _| !(k.starts_with(ch) && channel_key_of(k, ch)));
            }
            if ui
                .button(tr("dialog.reset"))
                .rich_tip(crate::ui::tips::keyed("dialog.reset"))
                .clicked()
            {
                params.clear();
            }
            if ui
                .add_enabled(
                    image.is_some(),
                    egui::Button::new(tr("dialog.auto_balance_tone")),
                )
                .rich_tip(tr("dialog.auto_balance_tone_tip"))
                .clicked()
            {
                if let Some(img) = image {
                    let curves = balance_curves(img, 0.005);
                    for (c, curve) in ["r", "g", "b"].iter().zip(curves.iter()) {
                        curve_into(params, c, curve);
                        set_curve_style(params, c, 1);
                    }
                }
            }
            let mut all = show_all;
            if ui
                .checkbox(&mut all, tr("dialog.display_all_channels"))
                .rich_tip(crate::ui::tips::keyed("dialog.display_all_channels"))
                .changed()
            {
                ui.data_mut(|d| d.insert_temp(egui::Id::new("tone_curve_all"), all));
            }
            // Presets: the curves of every channel in a JSON file.
            ui.horizontal(|ui| {
                if ui
                    .button(tr("dialog.load"))
                    .rich_tip(crate::ui::tips::pair(
                        "dialog.load",
                        "dialog.curve_load_desc",
                    ))
                    .clicked()
                {
                    crate::files::Dialog::new()
                        .add_filter(tr("dialog.tone_curve_preset"), &["json"])
                        .pick_into(PRESET_PICK);
                }
                if ui
                    .button(tr("dialog.save"))
                    .rich_tip(crate::ui::tips::pair(
                        "dialog.save",
                        "dialog.curve_save_desc",
                    ))
                    .clicked()
                {
                    if let Some(path) = crate::files::Dialog::new()
                        .add_filter(tr("dialog.tone_curve_preset"), &["json"])
                        .set_file_name("tone curve.json")
                        .save_path()
                    {
                        if let Ok(json) = serde_json::to_string_pretty(&*params) {
                            if let Err(e) = crate::files::write(&path, json) {
                                *status = e.to_string();
                            }
                        }
                    }
                }
            });
        });
    });
}

/// Contrast Enhancement's eyedroppers: they set the input clipping
/// levels from a pixel's brightness.
const LEVEL_PICKS: [&str; 2] = ["input_low", "input_high"];

/// A triangle marker under a bar at `x`, filled with `fill`.
fn marker(painter: &egui::Painter, x: f32, top: f32, fill: Color32) {
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(x, top),
            egui::pos2(x + 5.0, top + 8.0),
            egui::pos2(x - 5.0, top + 8.0),
        ],
        fill,
        Stroke::new(1.0, Color32::from_gray(0x40)),
    ));
}

/// Drag the nearer of two level markers along a bar of `r`; returns the
/// changed (low, high) in 0..255, keeping low below high.
fn drag_levels(
    ui: &mut Ui,
    id: egui::Id,
    r: Rect,
    lo: f64,
    hi: f64,
    ordered: bool,
) -> Option<(f64, f64)> {
    let resp = ui.interact(r.expand2(Vec2::new(6.0, 0.0)), id, Sense::click_and_drag());
    let to_x = |v: f64| r.left() + (v / 255.0) as f32 * r.width();
    let to_v = |x: f32| {
        (((x - r.left()) / r.width()) * 255.0)
            .round()
            .clamp(0.0, 255.0) as f64
    };
    if resp.drag_started() || resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let which = (p.x - to_x(hi)).abs() < (p.x - to_x(lo)).abs();
            ui.data_mut(|d| d.insert_temp(id, which));
        }
    }
    if resp.dragged() || resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let high = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
            let v = to_v(p.x);
            return Some(if high {
                (lo, if ordered { v.max(lo + 1.0) } else { v })
            } else {
                (if ordered { v.min(hi - 1.0) } else { v }, hi)
            });
        }
    }
    None
}

/// Contrast Enhancement: the image's histogram with the input clipping
/// markers, the output range bar with its markers, the eyedroppers that
/// pick the input levels from the preview, and Auto-adjust.
fn levels_part(
    ui: &mut Ui,
    st: &mut EffectState,
    hist: Option<&[u32; 256]>,
    image: Option<&image::RgbaImage>,
) {
    let params = &mut st.effect.params;
    let get = |p: &BTreeMap<String, f64>, k: &str, d: f64| p.get(k).copied().unwrap_or(d);
    let (il, ih) = (
        get(params, "input_low", 0.0),
        get(params, "input_high", 255.0),
    );
    let (ol, oh) = (
        get(params, "output_low", 0.0),
        get(params, "output_high", 255.0),
    );
    let width = 300.0;
    // The histogram and the input markers.
    let (r, _) = ui.allocate_exact_size(Vec2::new(width, 90.0), Sense::hover());
    let painter = ui.painter_at(r.expand2(Vec2::new(6.0, 12.0)));
    painter.rect_filled(r, 0.0, Color32::WHITE);
    if let Some(h) = hist {
        let max = h.iter().map(|v| (*v as f32).sqrt()).fold(1.0, f32::max);
        for (i, v) in h.iter().enumerate() {
            let x = r.left() + (i as f32 + 0.5) / 256.0 * r.width();
            let top = r.bottom() - (*v as f32).sqrt() / max * (r.height() - 2.0);
            painter.line_segment(
                [egui::pos2(x, r.bottom()), egui::pos2(x, top)],
                Stroke::new(r.width() / 256.0 + 0.2, Color32::from_gray(0x60)),
            );
        }
    }
    // Clipped areas shaded.
    let x_of = |v: f64| r.left() + (v / 255.0) as f32 * r.width();
    painter.rect_filled(
        Rect::from_min_max(r.min, egui::pos2(x_of(il), r.bottom())),
        0.0,
        Color32::from_black_alpha(40),
    );
    painter.rect_filled(
        Rect::from_min_max(egui::pos2(x_of(ih), r.top()), r.max),
        0.0,
        Color32::from_black_alpha(40),
    );
    painter.rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, crate::theme::Tokens::border()),
        egui::StrokeKind::Inside,
    );
    let (mr, _) = ui.allocate_exact_size(Vec2::new(width, 10.0), Sense::hover());
    marker(
        &ui.painter_at(mr.expand(6.0)),
        x_of(il),
        mr.top(),
        Color32::BLACK,
    );
    marker(
        &ui.painter_at(mr.expand(6.0)),
        x_of(ih),
        mr.top(),
        Color32::WHITE,
    );
    if let Some((lo, hi)) = drag_levels(
        ui,
        egui::Id::new("levels_in"),
        Rect::from_min_max(r.min, mr.max),
        il,
        ih,
        true,
    ) {
        params.insert("input_low".into(), lo);
        params.insert("input_high".into(), hi);
    }
    ui.add_space(4.0);
    // The output range: a gradient bar and its markers.
    let (br, _) = ui.allocate_exact_size(Vec2::new(width, 12.0), Sense::hover());
    let mut mesh = egui::epaint::Mesh::default();
    mesh.colored_vertex(br.left_top(), Color32::BLACK);
    mesh.colored_vertex(br.right_top(), Color32::WHITE);
    mesh.colored_vertex(br.right_bottom(), Color32::WHITE);
    mesh.colored_vertex(br.left_bottom(), Color32::BLACK);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(egui::Shape::mesh(mesh));
    ui.painter().rect_stroke(
        br,
        0.0,
        Stroke::new(1.0, crate::theme::Tokens::border()),
        egui::StrokeKind::Inside,
    );
    let (or, _) = ui.allocate_exact_size(Vec2::new(width, 10.0), Sense::hover());
    let ox = |v: f64| br.left() + (v / 255.0) as f32 * br.width();
    marker(
        &ui.painter_at(or.expand(6.0)),
        ox(ol),
        or.top(),
        Color32::BLACK,
    );
    marker(
        &ui.painter_at(or.expand(6.0)),
        ox(oh),
        or.top(),
        Color32::WHITE,
    );
    if let Some((lo, hi)) = drag_levels(
        ui,
        egui::Id::new("levels_out"),
        Rect::from_min_max(br.min, or.max),
        ol,
        oh,
        false,
    ) {
        params.insert("output_low".into(), lo);
        params.insert("output_high".into(), hi);
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for (name, white) in [("input_low", false), ("input_high", true)] {
            let on = st.pick.as_deref() == Some(name);
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
            crate::ui::propbar::frame(ui, rect, resp.hovered(), on);
            let ir = Rect::from_center_size(rect.center(), Vec2::splat(16.0));
            crate::ui::icons::draw(
                ui.painter(),
                ir,
                crate::tools::Tool::ColorEyedropper,
                crate::theme::Tokens::icon(),
            );
            let sw = Rect::from_min_size(ir.left_bottom() - Vec2::new(0.0, 5.0), Vec2::splat(5.0));
            ui.painter().rect_filled(
                sw,
                0.0,
                if white {
                    Color32::WHITE
                } else {
                    Color32::BLACK
                },
            );
            ui.painter().rect_stroke(
                sw,
                0.0,
                Stroke::new(1.0, Color32::from_gray(0x40)),
                egui::StrokeKind::Middle,
            );
            let tip = if white {
                "dialog.pick_input_high"
            } else {
                "dialog.pick_input_low"
            };
            if resp.rich_tip(tr(tip)).clicked() {
                st.pick = if on { None } else { Some(name.to_string()) };
            }
        }
        if ui
            .add_enabled(
                image.is_some(),
                egui::Button::new(tr("dialog.lab_auto_adjust")),
            )
            .rich_tip(crate::ui::tips::pair(
                "dialog.lab_auto_adjust",
                "dialog.levels_auto_desc",
            ))
            .rich_disabled_tip(crate::ui::tips::pair(
                "dialog.lab_auto_adjust",
                "dialog.levels_auto_desc",
            ))
            .clicked()
        {
            if let Some(img) = image {
                let (bp, wp) = crate::ui::lab_dialog::auto_points(img);
                st.effect.params.insert("input_low".into(), bp);
                st.effect.params.insert("input_high".into(), wp);
            }
        }
    });
    ui.add_space(4.0);
}

/// Whether `key` is one of channel `ch`'s curve settings (and not, for
/// `r`, one of `rgb`'s).
fn channel_key_of(key: &str, ch: &str) -> bool {
    let rest = &key[ch.len()..];
    rest == "n"
        || rest == "linear"
        || rest == "style"
        || rest == "gamma"
        || (rest.ends_with(['x', 'y'])
            && rest[..rest.len() - 1].chars().all(|c| c.is_ascii_digit())
            && rest.len() > 1)
}

/// A histogram of one channel: 0 brightness, 1 to 3 red, green, blue.
fn channel_histogram(img: &image::RgbaImage, ch: usize) -> [u32; 256] {
    if ch == 0 {
        return histogram(img);
    }
    let mut h = [0u32; 256];
    for p in img.pixels().filter(|p| p[3] > 0) {
        h[p[(ch - 1).min(2)] as usize] += 1;
    }
    h
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
    if spec.id == "image_adjustments" {
        crate::ui::lab_dialog::lab_dialog(app, ctx, st, close);
        return;
    }
    if spec.id == "tone_curve" {
        if let Some(path) = crate::files::take_picked(PRESET_PICK) {
            match crate::files::read_to_string(&path)
                .ok()
                .and_then(|t| read_preset(&t))
            {
                Some(p) => st.effect.params = p,
                None => app.status = tr("status.bad_preset"),
            }
        }
    }
    let title = tr(&format!("fx.{}", spec.id));
    let mut ok = false;
    before_after::window(ctx, title).show(ctx, |ui| {
        let effect = st.effect.clone();
        // Adding starts from what the bitmap shows without this effect
        // (its effects so far; the live preview puts this one last);
        // editing, from what the effects before this one made, and the
        // after view runs the ones after it too.
        let stack = match st.edit {
            Some((id, i)) => app.bitmap_fx_stack(id).map(|s| (id, i, s)),
            None => crate::ui::bitmap_preview::selected_bitmap(app).and_then(|(id, _, _)| {
                let s = app.bitmap_fx_stack(id)?;
                let shown = usize::from(app.fx_preview.on_drawing());
                let i = s.effects.len().saturating_sub(shown);
                Some((id, i, s))
            }),
        };
        let after: Vec<BitmapEffect> = match &stack {
            Some((_, index, stack)) => stack.effects.iter().skip(index + 1).cloned().collect(),
            None => Vec::new(),
        };
        let src = match &stack {
            Some((shape, index, stack)) => Source::Stack {
                shape: *shape,
                stack,
                index: *index,
            },
            None => Source::Shown,
        };
        before_after::columns(ui, RIGHT_W, BOTTOM_H, |left, right| {
            let mut picking = st.pick.is_some();
            let preview = before_after::live(
                left,
                app,
                egui::Id::new("effect_preview"),
                &mut st.lab.view,
                &src,
                settings_key(&(&effect, &after)),
                &View::PLAIN,
                &mut picking,
                0.0,
                |_, _, _| {},
                |img, scale| {
                    let mut out = crate::fx::apply_effect(img, &crate::fx::scaled(&effect, scale));
                    for e in after.iter().filter(|e| e.visible) {
                        out = crate::fx::apply_effect(&out, &crate::fx::scaled(e, scale));
                    }
                    out
                },
            );
            if !picking {
                st.pick = None;
            }
            effect_settings(right, app, st, spec, preview);
        });
        ok = super::dialogs::ok_cancel(ui, close);
    });
    if ok {
        app.commit_bitmap_effect(st);
    } else if *close {
        app.end_bitmap_effect_preview();
    } else {
        app.preview_bitmap_effect(ctx, st);
    }
}

/// The settings column: the effect's controls (the Tone Curve's curves,
/// Contrast Enhancement's histogram), Reset and the Preview box.
/// `preview` is what the shared preview made this frame: the copy for
/// histograms and the pixel an eyedropper picked.
fn effect_settings(
    ui: &mut Ui,
    app: &mut App,
    st: &mut EffectState,
    spec: &crate::fx::EffectSpec,
    preview: Option<before_after::Live>,
) {
    {
        // An eyedropper picks the colour under a click on the preview (the
        // Tone Curve's adds a node instead).
        let mut node_pick = None;
        if let (Some(name), Some(p)) = (st.pick.clone(), preview.as_ref()) {
            if let Some((x, y)) = p.picked {
                let c = *p.img.get_pixel(x, y);
                if name == NODE_PICK {
                    node_pick = Some(c);
                } else if LEVEL_PICKS.contains(&name.as_str()) {
                    // Contrast Enhancement's eyedroppers take a brightness.
                    let l = crate::fx::util::luma(&c).round() as f64;
                    let (lo, hi) = (
                        st.effect.params.get("input_low").copied().unwrap_or(0.0),
                        st.effect.params.get("input_high").copied().unwrap_or(255.0),
                    );
                    let v = if name == "input_low" {
                        l.min(hi - 1.0)
                    } else {
                        l.max(lo + 1.0)
                    };
                    st.effect.params.insert(name, v.clamp(0.0, 255.0));
                } else {
                    let v = ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32;
                    st.effect.params.insert(name, v as f64);
                }
                st.pick = None;
            }
        }
        let thumb = preview.map(|p| p.img);
        if spec.id == "tone_curve" {
            let hist = thumb.as_ref().map(|t| histogram(t));
            tone_curve_part(
                ui,
                st,
                hist.as_ref(),
                thumb.as_deref(),
                node_pick,
                &mut app.status,
            );
        } else if spec.params.is_empty() {
            ui.label(tr("dialog.no_settings"));
        } else {
            // The settings panel, with the Reset, undo and redo row and
            // the Preview box under it.
            let below = before_after::STEPS_H + 30.0;
            before_after::panel(ui, "fx_params_scroll", below, |ui| {
                if spec.id == "contrast_enhancement" {
                    // The chosen channel's histogram (brightness for all).
                    let ch = st
                        .effect
                        .params
                        .get("channel")
                        .map(|v| v.max(0.0) as usize)
                        .unwrap_or(0)
                        .min(3);
                    let hist = thumb.as_ref().map(|t| channel_histogram(t, ch));
                    levels_part(ui, st, hist.as_ref(), thumb.as_deref());
                }
                for p in spec.params {
                    let r = ui
                        .vertical(|ui| {
                            param_stacked(ui, p, &mut st.effect.params, Some(&mut st.pick))
                        })
                        .response;
                    before_after::mark(ui, p.name, r.rect);
                    ui.add_space(2.0);
                }
            });
            before_after::steps_for(
                ui,
                egui::Id::new("fx_steps"),
                &mut st.effect.params,
                BTreeMap::new(),
                &crate::ui::tips::keyed("dialog.reset"),
            );
        }
        preview_row(ui, app, &mut st.preview);
    }
}

/// The Preview box (the effect on the drawing while the dialog is open)
/// and, while a preview is being made, a busy mark beside it.
pub fn preview_row(ui: &mut Ui, app: &App, preview: &mut bool) {
    ui.horizontal(|ui| {
        ui.checkbox(preview, tr("dialog.preview"))
            .rich_tip(crate::ui::tips::pair(
                "dialog.preview",
                "dialog.live_preview_desc",
            ));
        if *preview && app.fx_preview.busy() {
            ui.spinner();
            ui.label(
                egui::RichText::new(tr("dialog.preview_busy"))
                    .size(11.0)
                    .color(crate::theme::Tokens::text_dim()),
            );
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
                .color(crate::theme::Tokens::text_dim())
                .size(11.0),
        );
    }
    let n = effects.len();
    for (i, e) in effects.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut vis = e.visible;
            if ui
                .checkbox(&mut vis, "")
                .rich_tip(tr("docker.show_hide_effect"))
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
                .rich_tip(tr("docker.edit_effect"))
                .clicked()
            {
                app.dialog = crate::ui::dialogs::Dialog::Effect(EffectState {
                    effect: e.clone(),
                    edit: Some((id, i)),
                    channel: 0,
                    lab: Default::default(),
                    pick: None,
                    preview: PREVIEW_AT_START,
                });
            }
            if ui
                .add_enabled(i > 0, egui::Button::new("\u{25B2}").small())
                .rich_tip(tr("docker.move_up"))
                .clicked()
            {
                app.edit_bitmap_effects(id, "Reorder Effects", move |l| l.swap(i, i - 1));
            }
            if ui
                .add_enabled(i + 1 < n, egui::Button::new("\u{25BC}").small())
                .rich_tip(tr("docker.move_down"))
                .clicked()
            {
                app.edit_bitmap_effects(id, "Reorder Effects", move |l| l.swap(i, i + 1));
            }
            if ui
                .small_button("\u{00D7}")
                .rich_tip(tr("docker.delete_effect"))
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
    let resp = ui
        .button(tr("docker.add_effect"))
        .rich_tip(crate::ui::tips::keyed("docker.add_effect"));
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

#[cfg(test)]
mod tests {
    use super::channel_key_of;

    #[test]
    fn presets_keep_numbers_only() {
        let p = super::read_preset(
            r#"{"rgbn": 2, "rgb0x": 0, "rgb0y": 10.5, "name": "x", "bad": null}"#,
        )
        .expect("preset");
        assert_eq!(p.len(), 3);
        assert_eq!(p["rgb0y"], 10.5);
        assert!(super::read_preset("[1, 2]").is_none());
        assert!(super::read_preset("not json").is_none());
    }

    #[test]
    fn resetting_a_channel_leaves_the_others() {
        for k in ["r0x", "r12y", "rn", "rlinear", "rstyle", "rgamma"] {
            assert!(channel_key_of(k, "r"), "{k}");
        }
        for k in ["rgb0x", "rgbn", "rgbstyle", "rx", "red"] {
            assert!(!channel_key_of(k, "r"), "{k}");
        }
        assert!(channel_key_of("rgb3y", "rgb"));
    }
}
