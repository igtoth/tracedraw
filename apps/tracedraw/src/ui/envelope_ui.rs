//! The Envelope tool's property bar, the Envelope docker, the frames drawn
//! over the selected objects with the Envelope and Shape tools, and the
//! envelope pictures.
//!
//! Bar, left to right: the presets list, the four modes (Straight Line,
//! Single Arc, Double Arc, Unconstrained), in the Unconstrained mode Add
//! Node and Delete Node, then Add New Envelope, the mapping list, Keep
//! Lines, Copy Envelope Properties and Clear Envelope.

use crate::app::App;
use crate::envelope_edit::{last_envelope, Part, Preset};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::propbar::{pic_button, sep, Pic};
use crate::ui::tips::RichTip;
use egui::{epaint, Color32, Painter, Pos2, Rect, Stroke, Ui, Vec2};
use tracedraw_core::envelope::{Mapping, Mode};
use tracedraw_core::geometry::{CubicBez, ParamCurve};

/// The name of a mode.
pub fn mode_key(m: Mode) -> &'static str {
    match m {
        Mode::StraightLine => "envelope.straight_line",
        Mode::SingleArc => "envelope.single_arc",
        Mode::DoubleArc => "envelope.double_arc",
        Mode::Unconstrained => "envelope.unconstrained",
    }
}

/// What a mode does.
fn mode_hint_key(m: Mode) -> &'static str {
    match m {
        Mode::StraightLine => "envelope.straight_line_hint",
        Mode::SingleArc => "envelope.single_arc_hint",
        Mode::DoubleArc => "envelope.double_arc_hint",
        Mode::Unconstrained => "envelope.unconstrained_hint",
    }
}

fn mode_pic(m: Mode) -> Pic {
    match m {
        Mode::StraightLine => Pic::EnvelopeStraight,
        Mode::SingleArc => Pic::EnvelopeSingleArc,
        Mode::DoubleArc => Pic::EnvelopeDoubleArc,
        Mode::Unconstrained => Pic::EnvelopeUnconstrained,
    }
}

/// The name of a mapping.
pub fn mapping_key(m: Mapping) -> &'static str {
    match m {
        Mapping::Horizontal => "envelope.mapping_horizontal",
        Mapping::Original => "envelope.mapping_original",
        Mapping::Putty => "envelope.mapping_putty",
        Mapping::Vertical => "envelope.mapping_vertical",
    }
}

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// The mapping list; true when the user chose one.
fn mapping_list(app: &mut App, ui: &mut Ui, width: f32) {
    let (shown, _) = app.envelope_settings_shown();
    let mut chosen = None;
    egui::ComboBox::from_id_salt("envelope_mapping")
        .selected_text(tr(mapping_key(shown)))
        .width(width)
        .show_ui(ui, |ui| {
            for m in Mapping::ALL {
                if ui
                    .selectable_label(m == shown, tr(mapping_key(m)))
                    .clicked()
                {
                    chosen = Some(m);
                }
            }
        })
        .response
        .rich_tip(tr("envelope.mapping_tip"));
    if let Some(m) = chosen {
        app.set_envelope_mapping(m);
    }
}

/// The presets list.
fn preset_list(app: &mut App, ui: &mut Ui, enabled: bool) {
    let mut chosen = None;
    ui.add_enabled_ui(enabled, |ui| {
        egui::ComboBox::from_id_salt("envelope_presets")
            .selected_text(tr("docker.presets"))
            .width(110.0)
            .show_ui(ui, |ui| {
                for p in Preset::ALL {
                    if ui.selectable_label(false, tr(p.key())).clicked() {
                        chosen = Some(p);
                    }
                }
            })
            .response
            .rich_tip(tr("envelope.presets_tip"));
    });
    if let Some(p) = chosen {
        app.apply_envelope_preset(p);
    }
}

/// The Envelope tool's property bar.
pub fn envelope_bar(app: &mut App, ui: &mut Ui) {
    let selected = !app.selection.is_empty();
    let enveloped = app
        .selection
        .iter()
        .any(|id| app.envelope_of(*id).is_some());
    preset_list(app, ui, selected);
    sep(ui);
    for m in Mode::ALL {
        if pic_button(
            ui,
            mode_pic(m),
            &tr(mode_key(m)),
            true,
            app.envelope_mode == m,
        )
        .clicked()
        {
            app.envelope_mode = m;
        }
    }
    if app.envelope_mode == Mode::Unconstrained {
        sep(ui);
        let node = app.selected_envelope_node();
        let removable = node
            .and_then(|(id, i)| app.envelope_of(id).map(|(_, f)| !f.is_corner(i)))
            .unwrap_or(false);
        if pic_button(
            ui,
            Pic::AddNode,
            &tr("envelope.add_node"),
            node.is_some(),
            false,
        )
        .clicked()
        {
            app.add_node_after_selected();
        }
        if pic_button(
            ui,
            Pic::DeleteNode,
            &tr("envelope.delete_node"),
            removable,
            false,
        )
        .clicked()
        {
            app.delete_selected_envelope_node();
        }
    }
    sep(ui);
    if pic_button(
        ui,
        Pic::EnvelopeAddNew,
        &tr("envelope.add_new"),
        selected,
        false,
    )
    .clicked()
    {
        app.add_new_envelope();
    }
    dim(ui, tr("envelope.mapping"));
    mapping_list(app, ui, 100.0);
    let (_, keep) = app.envelope_settings_shown();
    if pic_button(ui, Pic::KeepLines, &tr("docker.keep_lines"), true, keep).clicked() {
        app.set_envelope_keep_lines(!keep);
    }
    sep(ui);
    if pic_button(
        ui,
        Pic::EnvelopeCopy,
        &tr("envelope.copy"),
        selected,
        app.pending_copy_envelope,
    )
    .clicked()
    {
        app.start_copy_envelope();
    }
    if pic_button(
        ui,
        Pic::EnvelopeClear,
        &tr("envelope.clear"),
        enveloped,
        false,
    )
    .clicked()
    {
        app.clear_envelope();
    }
    if !selected {
        sep(ui);
        dim(ui, tr("status_hint.envelope"));
    }
}

/// One preset's picture in the docker's grid; true when clicked.
fn preset_cell(ui: &mut Ui, i: usize, size: Vec2, enabled: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let resp = resp.rich_tip(tr("envelope.preset_tip"));
    let p = ui.painter_at(r);
    if resp.hovered() && enabled {
        p.rect_filled(r, 2.0, Tokens::tool_hover());
    }
    let ink = if enabled {
        Color32::from_gray(0x20)
    } else {
        Color32::from_gray(0xA8)
    };
    let o = crate::envelope_shapes::outline(i);
    let area = r.shrink2(Vec2::new(8.0, 7.0));
    let pts: Vec<Pos2> = o
        .points(12)
        .into_iter()
        .map(|q| {
            Pos2::new(
                area.left() + q.x as f32 * area.width(),
                area.bottom() - q.y as f32 * area.height(),
            )
        })
        .collect();
    if pts.len() > 2 {
        p.add(epaint::PathShape::closed_line(pts, Stroke::new(1.0, ink)));
    }
    enabled && resp.clicked()
}

/// The Envelope docker: Add New, the preset shapes, the editing modes
/// with the mapping, and Keep lines, Copy and Clear.
pub fn envelope_docker(app: &mut App, ui: &mut Ui) {
    let selected = !app.selection.is_empty();
    let enveloped = app
        .selection
        .iter()
        .any(|id| app.envelope_of(*id).is_some());
    let tip = crate::ui::tips::keyed("envelope.add_new");
    if crate::ui::docker_form::wide_button(ui, &tr("envelope.add_new"), selected, &tip) {
        app.add_new_envelope();
        app.set_tool(Tool::Envelope);
    }
    ui.add_space(4.0);
    let title = egui::RichText::new(tr("envelope.choose_preset"));
    ui.label(if selected {
        title
    } else {
        title.color(Tokens::text_dim())
    });
    let width = ui.available_width();
    let cell = Vec2::new(((width - 18.0) / 4.0).floor().max(40.0), 46.0);
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0, Tokens::border()))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("envelope_presets")
                .max_height(cell.y * 5.0)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    for row in 0..crate::envelope_shapes::COUNT.div_ceil(4) {
                        ui.horizontal(|ui| {
                            for col in 0..4 {
                                let i = row * 4 + col;
                                if i >= crate::envelope_shapes::COUNT {
                                    break;
                                }
                                if preset_cell(ui, i, cell, selected) {
                                    app.apply_envelope_frame(|b| {
                                        crate::envelope_shapes::frame(i, b).unwrap_or_else(|| {
                                            tracedraw_core::envelope::Frame::around(b)
                                        })
                                    });
                                }
                            }
                        });
                    }
                });
        });
    ui.add_space(4.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for m in Mode::ALL {
            let tip = format!("{}\n{}", tr(mode_key(m)), tr(mode_hint_key(m)));
            if pic_button(ui, mode_pic(m), &tip, true, app.envelope_mode == m).clicked() {
                app.envelope_mode = m;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            mapping_list(app, ui, 100.0);
        });
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let (_, keep) = app.envelope_settings_shown();
        if pic_button(ui, Pic::KeepLines, &tr("docker.keep_lines"), true, keep).clicked() {
            app.set_envelope_keep_lines(!keep);
        }
        if pic_button(
            ui,
            Pic::EnvelopeCopy,
            &tr("envelope.copy"),
            selected,
            app.pending_copy_envelope,
        )
        .clicked()
        {
            app.start_copy_envelope();
        }
        if pic_button(
            ui,
            Pic::EnvelopeClear,
            &tr("envelope.clear"),
            enveloped,
            false,
        )
        .clicked()
        {
            app.clear_envelope();
        }
    });
    if !selected {
        ui.add_space(4.0);
        dim(ui, tr("docker.envelope_hint"));
    }
}

/// How far a drawn frame may stray from its curves, in pixels.
const FRAME_TOLERANCE: f64 = 0.25;

/// How many equal steps draw the frame segment `c` (in screen pixels)
/// within [`FRAME_TOLERANCE`] of the curve: a step of `1/n` strays at most
/// `6 d / (8 n^2)` from a cubic whose control points' second differences
/// are at most `d`. A fixed 16 steps drew a large segment, zoomed in, as
/// a visible row of lines (#881); a short one takes few.
fn frame_steps(c: &CubicBez) -> usize {
    let d = (c.p0.to_vec2() - 2.0 * c.p1.to_vec2() + c.p2.to_vec2())
        .hypot()
        .max((c.p1.to_vec2() - 2.0 * c.p2.to_vec2() + c.p3.to_vec2()).hypot());
    let n = (0.75 * d / FRAME_TOLERANCE).sqrt().ceil();
    if n.is_finite() {
        (n as usize).clamp(1, 256)
    } else {
        1
    }
}

/// The frames of the selected objects' last envelopes, with the Envelope
/// and the Shape tools: the frame, its nodes (the selected one filled)
/// and, in the Unconstrained mode, the selected node's handles.
pub fn draw_envelopes(app: &App, painter: &Painter) {
    if !matches!(app.tool, Tool::Shape | Tool::Envelope) {
        return;
    }
    let blue = Color32::from_rgb(40, 110, 220);
    for s in app.selected_shapes() {
        let Some((_, f)) = last_envelope(&s.effects) else {
            continue;
        };
        let page = f.transformed(s.transform);
        let mut outline: Vec<Pos2> = Vec::new();
        for i in 0..page.len() {
            let screen = |p: tracedraw_core::geometry::Point| {
                let q = app.view.to_screen(p);
                tracedraw_core::geometry::Point::new(q.x as f64, q.y as f64)
            };
            let c = page.segment(i);
            let c = CubicBez::new(screen(c.p0), screen(c.p1), screen(c.p2), screen(c.p3));
            let n = frame_steps(&c);
            for k in 0..n {
                let p = c.eval(k as f64 / n as f64);
                outline.push(Pos2::new(p.x as f32, p.y as f32));
            }
        }
        if outline.len() > 2 {
            painter.add(epaint::PathShape::closed_line(
                outline,
                Stroke::new(1.0, blue),
            ));
        }
        for (_, _, h, node) in app.envelope_handles(s.id, &f) {
            let (hs, ns) = (
                app.view.to_screen(s.transform * h),
                app.view.to_screen(s.transform * node),
            );
            painter.line_segment([ns, hs], Stroke::new(1.0, blue));
            painter.circle_filled(hs, 3.5, Color32::WHITE);
            painter.circle_stroke(hs, 3.5, Stroke::new(1.0, blue));
        }
        let dragged = app.envelope_drag.as_ref().and_then(|d| match d.part {
            Part::Node(i) if d.shape == s.id => Some(i),
            _ => None,
        });
        for (i, n) in page.nodes.iter().enumerate() {
            let sp = app.view.to_screen(*n);
            let size = if page.is_corner(i) { 8.0 } else { 7.0 };
            let r = Rect::from_center_size(sp, Vec2::splat(size));
            let chosen = dragged == Some(i) || app.selected_effect_node == Some((s.id, i));
            painter.rect_filled(r, 0.0, if chosen { blue } else { Color32::WHITE });
            painter.rect_stroke(r, 0.0, Stroke::new(1.0, blue), egui::StrokeKind::Outside);
        }
    }
}

/// Draw an envelope picture (16 x 16 grid) in `r`.
pub fn draw_envelope_pic(painter: &Painter, r: Rect, pic: Pic, color: Color32) {
    let k = r.width() / 16.0;
    let q = |x: f32, y: f32| r.min + Vec2::new(x, y) * k;
    let s = Stroke::new(1.1 * k, color);
    let thin = Stroke::new(0.85 * k, color);
    let node = |x: f32, y: f32| {
        painter.rect_filled(
            Rect::from_center_size(q(x, y), Vec2::splat(2.6 * k)),
            0.0,
            color,
        );
    };
    // A frame whose top runs through `top` (x from 2 to 14), the rest a box
    // from y 5 down to 14.
    let frame = |top: &dyn Fn(f32) -> f32| {
        let mut pts = vec![q(2.0, 14.0), q(14.0, 14.0)];
        for i in 0..=24 {
            let x = 14.0 - 12.0 * i as f32 / 24.0;
            pts.push(q(x, top(x)));
        }
        painter.add(epaint::PathShape::closed_line(pts, s));
    };
    match pic {
        Pic::EnvelopeStraight => {
            // A tent: straight from the corners to the raised node.
            frame(&|x: f32| 8.5 - 7.0 * (1.0 - (x - 8.0).abs() / 6.0));
            node(8.0, 1.5);
        }
        Pic::EnvelopeSingleArc => {
            // A dome: one arc through the raised node.
            frame(&|x: f32| {
                let t = (x - 8.0) / 6.0;
                8.5 - 7.0 * (1.0 - t * t)
            });
            node(8.0, 1.5);
        }
        Pic::EnvelopeDoubleArc => {
            // A wave: up, down, up.
            frame(&|x: f32| 6.0 - 3.5 * (std::f32::consts::PI * (x - 2.0) / 6.0).sin());
            node(8.0, 6.0);
        }
        Pic::EnvelopeUnconstrained => {
            // A free curve through a node with its handle.
            frame(&|x: f32| {
                let t = (x - 2.0) / 12.0;
                8.0 - 6.5 * (std::f32::consts::PI * t).sin() * (1.0 - 0.45 * t)
            });
            painter.line_segment([q(4.0, 0.8), q(11.0, 3.6)], thin);
            node(7.5, 2.2);
            painter.circle_filled(q(4.0, 0.8), 1.2 * k, color);
            painter.circle_filled(q(11.0, 3.6), 1.2 * k, color);
        }
        Pic::EnvelopeAddNew => {
            frame(&|x: f32| {
                let t = (x - 8.0) / 6.0;
                7.0 - 3.0 * (1.0 - t * t)
            });
            painter.line_segment([q(8.0, 7.5), q(8.0, 12.5)], s);
            painter.line_segment([q(5.5, 10.0), q(10.5, 10.0)], s);
        }
        Pic::EnvelopeClear => {
            frame(&|x: f32| {
                let t = (x - 8.0) / 6.0;
                7.0 - 3.0 * (1.0 - t * t)
            });
            painter.line_segment([q(5.5, 7.5), q(10.5, 12.5)], s);
            painter.line_segment([q(10.5, 7.5), q(5.5, 12.5)], s);
        }
        Pic::EnvelopeCopy => {
            // A small enveloped frame, an arrow, a plain one taking it.
            let small = |x0: f32, arched: bool| {
                let mut pts = vec![q(x0, 14.0), q(x0 + 5.0, 14.0)];
                for i in 0..=10 {
                    let x = x0 + 5.0 - 5.0 * i as f32 / 10.0;
                    let t = (x - x0 - 2.5) / 2.5;
                    let y = if arched {
                        9.0 - 3.0 * (1.0 - t * t)
                    } else {
                        8.0
                    };
                    pts.push(q(x, y));
                }
                painter.add(epaint::PathShape::closed_line(pts, thin));
            };
            small(1.0, true);
            small(10.0, false);
            painter.line_segment([q(4.0, 4.0), q(12.0, 4.0)], thin);
            painter.add(epaint::PathShape::convex_polygon(
                vec![q(13.0, 4.0), q(10.5, 2.5), q(10.5, 5.5)],
                color,
                Stroke::NONE,
            ));
        }
        Pic::KeepLines => {
            frame(&|x: f32| {
                let t = (x - 8.0) / 6.0;
                6.0 - 4.0 * (1.0 - t * t)
            });
            for x in [5.0, 8.0, 11.0] {
                let t: f32 = (x - 8.0) / 6.0;
                let top = 6.0 - 4.0 * (1.0 - t * t) + 1.0;
                painter.line_segment([q(x, top), q(x, 13.0)], thin);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Point;

    /// A frame segment is drawn within a quarter pixel of its curve
    /// however large it is on screen, with few steps when it is small or
    /// straight (#881).
    #[test]
    fn the_frame_is_drawn_smooth_at_any_zoom() {
        // An arch, from a frame segment seen at growing zooms.
        for zoom in [0.5, 4.0, 40.0, 400.0] {
            let c = CubicBez::new(
                Point::new(0.0, 0.0),
                Point::new(10.0 * zoom, -30.0 * zoom),
                Point::new(30.0 * zoom, -30.0 * zoom),
                Point::new(40.0 * zoom, 0.0),
            );
            let n = frame_steps(&c);
            let pts: Vec<Point> = (0..=n).map(|k| c.eval(k as f64 / n as f64)).collect();
            // The curve's farthest point from the drawn line.
            let mut worst: f64 = 0.0;
            for k in 0..=4000 {
                let p = c.eval(k as f64 / 4000.0);
                let d = pts
                    .windows(2)
                    .map(|w| {
                        let ab = w[1] - w[0];
                        let t = ((p - w[0]).dot(ab) / ab.hypot2().max(1e-12)).clamp(0.0, 1.0);
                        (p - (w[0] + ab * t)).hypot()
                    })
                    .fold(f64::MAX, f64::min);
                worst = worst.max(d);
            }
            assert!(
                worst <= FRAME_TOLERANCE,
                "zoom {zoom}: {n} steps, {worst} px off"
            );
            if zoom >= 40.0 {
                // The 16 steps it used to take were not enough.
                assert!(n > 16, "zoom {zoom}: {n}");
            }
        }
        // A straight segment is one line; a tiny curve a few.
        let line = CubicBez::new(
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(200.0, 0.0),
            Point::new(300.0, 0.0),
        );
        assert_eq!(frame_steps(&line), 1);
        let nan = CubicBez::new(
            Point::new(f64::NAN, 0.0),
            Point::ZERO,
            Point::ZERO,
            Point::ZERO,
        );
        assert_eq!(frame_steps(&nan), 1);
    }

    /// The bar and the docker draw with nothing selected, with an
    /// enveloped object and in every mode.
    #[test]
    fn the_bar_and_the_docker_draw() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let id = app
            .new_shape(tracedraw_core::ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 40.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        for (i, m) in Mode::ALL.into_iter().enumerate() {
            if i == 2 {
                app.select(vec![id]);
                app.add_new_envelope();
                app.selected_effect_node = Some((id, 5));
            }
            app.envelope_mode = m;
            app.tool = Tool::Envelope;
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1400.0, 600.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                ui.horizontal(|ui| envelope_bar(&mut app, ui));
                envelope_docker(&mut app, ui);
                draw_envelopes(&app, ui.painter());
                for pic in [
                    Pic::EnvelopeStraight,
                    Pic::EnvelopeSingleArc,
                    Pic::EnvelopeDoubleArc,
                    Pic::EnvelopeUnconstrained,
                    Pic::EnvelopeAddNew,
                    Pic::EnvelopeClear,
                    Pic::EnvelopeCopy,
                    Pic::KeepLines,
                ] {
                    draw_envelope_pic(
                        ui.painter(),
                        Rect::from_min_size(Pos2::ZERO, Vec2::splat(18.0)),
                        pic,
                        Tokens::icon(),
                    );
                }
            });
            out.textures_delta.clear();
        }
        assert_eq!(mode_key(Mode::DoubleArc), "envelope.double_arc");
        assert_eq!(mapping_key(Mapping::Original), "envelope.mapping_original");
    }
}
