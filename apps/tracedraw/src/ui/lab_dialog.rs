//! Effects > Adjust > Image Adjustments: a window the user sizes with
//! the shared Before and After preview (`before_after.rs`, with the
//! split view and quarter turns besides), Auto adjust and the white and
//! black point pickers; the colour and tone sliders; a histogram of the
//! result; undo, redo and reset inside the dialog; numbered snapshots to
//! compare versions. OK adds (or changes) one Image Adjustments effect
//! with the chosen settings.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::before_after::{self, View, ViewState, BOTTOM_H, RIGHT_W};
use crate::ui::bitmap_preview::{copy_of, settings_key, Source};
use crate::ui::effect_dialog::{param_widget, EffectState};
use crate::ui::tips::RichTip;
use egui::{epaint, Color32, Context, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use image::RgbaImage;
use std::collections::BTreeMap;
use tracedraw_core::BitmapEffect;

/// Snapshot thumbnails' largest side.
const SNAP: u32 = 96;
/// The views the dialog offers: the shared ones and the split view.
const VIEWS: [View; 5] = [
    View::After,
    View::Stacked,
    View::SideBySide,
    View::Before,
    View::Split,
];

/// The settings the sliders show, in groups.
const SLIDERS: [&[&str]; 3] = [
    &["temperature", "tint", "saturation"],
    &["brightness", "contrast"],
    &["highlights", "shadows", "midtones"],
];

/// What a click in the preview does besides panning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabTool {
    #[default]
    Pan,
    WhitePoint,
    BlackPoint,
}

/// The dialog's own state besides the settings (the shared preview's
/// view state serves every effect dialog).
#[derive(Debug, Clone, PartialEq)]
pub struct LabState {
    pub tool: LabTool,
    /// How the preview is looked at: the view, zoom, place and turns.
    pub view: ViewState,
    /// The settings after each correction; `pos` is the one shown.
    pub history: Vec<BTreeMap<String, f64>>,
    pub pos: usize,
    /// Numbered snapshots of the settings.
    pub snapshots: Vec<(u32, BTreeMap<String, f64>)>,
    pub next: u32,
}

impl Default for LabState {
    fn default() -> Self {
        LabState {
            tool: LabTool::Pan,
            view: ViewState::with_view(View::After),
            history: Vec::new(),
            pos: 0,
            snapshots: Vec::new(),
            next: 1,
        }
    }
}

impl LabState {
    /// Record the settings as a new step when they differ from the
    /// current one and nothing is being dragged (a slider let go, a pick
    /// or a button). The first call keeps the starting settings.
    pub fn record(&mut self, params: &BTreeMap<String, f64>, settled: bool) {
        if self.history.is_empty() {
            self.history.push(params.clone());
            self.pos = 0;
            return;
        }
        if settled && self.history.get(self.pos) != Some(params) {
            self.history.truncate(self.pos + 1);
            self.history.push(params.clone());
            self.pos = self.history.len() - 1;
        }
    }

    pub fn can_undo(&self) -> bool {
        self.pos > 0
    }

    pub fn can_redo(&self) -> bool {
        self.pos + 1 < self.history.len()
    }

    /// One step back; the settings to show.
    pub fn undo(&mut self) -> Option<BTreeMap<String, f64>> {
        if !self.can_undo() {
            return None;
        }
        self.pos -= 1;
        self.history.get(self.pos).cloned()
    }

    pub fn redo(&mut self) -> Option<BTreeMap<String, f64>> {
        if !self.can_redo() {
            return None;
        }
        self.pos += 1;
        self.history.get(self.pos).cloned()
    }

    /// Keep the settings as the next numbered snapshot.
    pub fn snapshot(&mut self, params: &BTreeMap<String, f64>) -> u32 {
        let n = self.next;
        self.snapshots.push((n, params.clone()));
        self.next += 1;
        n
    }
}

/// The black and white points that cut 0.5 % of the opaque pixels'
/// brightness at either end (Auto adjust).
pub fn auto_points(img: &RgbaImage) -> (f64, f64) {
    let mut hist = [0u32; 256];
    let mut n = 0u32;
    for p in img.pixels().filter(|p| p[3] > 0) {
        hist[crate::fx::util::clamp8(crate::fx::util::luma(p)) as usize] += 1;
        n += 1;
    }
    if n == 0 {
        return (0.0, 255.0);
    }
    let cut = (n as f32 * 0.005) as u32;
    let mut acc = 0;
    let mut lo = 0usize;
    for (i, v) in hist.iter().enumerate() {
        acc += v;
        if acc > cut {
            lo = i;
            break;
        }
    }
    acc = 0;
    let mut hi = 255usize;
    for i in (0..256).rev() {
        acc += hist[i];
        if acc > cut {
            hi = i;
            break;
        }
    }
    if hi <= lo {
        return (0.0, 255.0);
    }
    (lo as f64, hi as f64)
}

/// Set the white (or black) point from a pixel's brightness, keeping the
/// other point at least one level away.
pub fn pick_point(params: &mut BTreeMap<String, f64>, pixel: &image::Rgba<u8>, white: bool) {
    let l = crate::fx::util::luma(pixel).round() as f64;
    let bp = params.get("black_point").copied().unwrap_or(0.0);
    let wp = params.get("white_point").copied().unwrap_or(255.0);
    if white {
        params.insert("white_point".into(), l.max(bp + 1.0).clamp(1.0, 255.0));
    } else {
        params.insert("black_point".into(), l.min(wp - 1.0).clamp(0.0, 254.0));
    }
}

/// An icon button 26 points square; `pressed` shows it pushed in.
fn icon_button(
    ui: &mut Ui,
    tip: &str,
    enabled: bool,
    pressed: bool,
    draw: impl FnOnce(&Painter, Rect, Color32),
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::splat(26.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    crate::ui::propbar::frame(ui, rect, enabled && resp.hovered(), pressed);
    let color = if enabled {
        Tokens::icon()
    } else {
        Tokens::border()
    };
    draw(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
        color,
    );
    let clicked = enabled && resp.clicked();
    resp.rich_tip(tip);
    clicked
}

fn at(r: Rect, x: f32, y: f32) -> Pos2 {
    r.min + Vec2::new(x, y) * (r.width() / 16.0)
}

/// A circular arrow, counter-clockwise when `left`.
pub(crate) fn draw_rotate(p: &Painter, r: Rect, c: Color32, left: bool) {
    let k = r.width() / 16.0;
    let centre = at(r, 8.0, 8.5);
    let rad = 5.2 * k;
    let pts: Vec<Pos2> = (0..=24)
        .map(|i| {
            let t = (30.0 + 270.0 * i as f32 / 24.0).to_radians();
            let x = if left { -t.cos() } else { t.cos() };
            centre + Vec2::new(x, -t.sin()) * rad
        })
        .collect();
    let end = pts[0];
    p.add(epaint::PathShape::line(pts, Stroke::new(1.4 * k, c)));
    let s = if left { -1.0 } else { 1.0 };
    p.add(epaint::PathShape::convex_polygon(
        vec![
            end + Vec2::new(s * 2.6, -1.0) * k,
            end + Vec2::new(-s * 0.6, -3.4) * k,
            end + Vec2::new(-s * 1.4, 1.6) * k,
        ],
        c,
        Stroke::NONE,
    ));
}

fn draw_actual(p: &Painter, r: Rect, c: Color32) {
    p.text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        "1:1",
        egui::FontId::proportional(11.0),
        c,
    );
}

fn draw_picker(p: &Painter, r: Rect, c: Color32, white: bool) {
    crate::ui::icons::draw(p, r, crate::tools::Tool::ColorEyedropper, c);
    let sw = Rect::from_min_max(at(r, 0.5, 10.5), at(r, 5.5, 15.5));
    p.rect_filled(
        sw,
        0.0,
        if white {
            Color32::WHITE
        } else {
            Color32::BLACK
        },
    );
    p.rect_stroke(sw, 0.0, Stroke::new(1.0, c), egui::StrokeKind::Middle);
}

fn histogram(img: &RgbaImage) -> [u32; 256] {
    let mut h = [0u32; 256];
    for p in img.pixels().filter(|p| p[3] > 0) {
        h[crate::fx::util::clamp8(crate::fx::util::luma(p)) as usize] += 1;
    }
    h
}

fn draw_histogram(ui: &mut Ui, hist: &[u32; 256], size: Vec2) {
    let (r, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(r);
    painter.rect_filled(r, 0.0, Color32::WHITE);
    // Square roots keep a big spike from flattening the rest.
    let max = hist.iter().map(|v| (*v as f32).sqrt()).fold(1.0, f32::max);
    let bw = r.width() / 256.0;
    for (i, v) in hist.iter().enumerate() {
        let hgt = (*v as f32).sqrt() / max * (r.height() - 2.0);
        if hgt > 0.0 {
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(r.left() + i as f32 * bw, r.bottom() - hgt),
                    Pos2::new(r.left() + (i + 1) as f32 * bw, r.bottom()),
                ),
                0.0,
                Color32::from_gray(0x50),
            );
        }
    }
    painter.rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Tokens::border()),
        egui::StrokeKind::Inside,
    );
}

fn lab_effect(params: &BTreeMap<String, f64>) -> BitmapEffect {
    BitmapEffect {
        id: "image_adjustments".into(),
        params: params.clone(),
        visible: true,
    }
}

/// The Image Adjustments dialog.
pub fn lab_dialog(app: &mut App, ctx: &Context, st: &mut EffectState, close: &mut bool) {
    let Some(spec) = crate::fx::spec("image_adjustments") else {
        *close = true;
        return;
    };
    let title = tr("fx.image_adjustments");
    before_after::window(ctx, title).show(ctx, |ui| {
        let stack = st
            .edit
            .and_then(|(id, i)| app.bitmap_fx_stack(id).map(|s| (id, i, s)));
        let src = match &stack {
            Some((shape, index, stack)) => Source::Stack {
                shape: *shape,
                stack,
                index: *index,
            },
            None => Source::Shown,
        };
        let settled = !ui.input(|i| i.pointer.any_down());
        st.lab.record(&st.effect.params, settled);
        let params = st.effect.params.clone();
        before_after::columns(ui, RIGHT_W, BOTTOM_H, |left, right| {
            // Under the panes: the undo, redo, reset and snapshot row and
            // the snapshots.
            let below = 32.0
                + if st.lab.snapshots.is_empty() {
                    0.0
                } else {
                    SNAP as f32 + 40.0
                };
            let mut picking = st.lab.tool != LabTool::Pan;
            let id = egui::Id::new("effect_preview");
            let preview = before_after::live(
                left,
                app,
                id,
                &mut st.lab.view,
                &src,
                settings_key(&params),
                &VIEWS,
                &mut picking,
                below,
                |ui, view, copy| {
                    ui.add_space(6.0);
                    if icon_button(
                        ui,
                        &crate::ui::tips::keyed("dialog.lab_rotate_left"),
                        true,
                        false,
                        |p, r, c| draw_rotate(p, r, c, true),
                    ) {
                        view.turns = (view.turns + 3) % 4;
                        view.centre = None;
                    }
                    if icon_button(
                        ui,
                        &crate::ui::tips::keyed("dialog.lab_rotate_right"),
                        true,
                        false,
                        |p, r, c| draw_rotate(p, r, c, false),
                    ) {
                        view.turns = (view.turns + 1) % 4;
                        view.centre = None;
                    }
                    let actual = 1.0 / copy.map(|c| c.scale).unwrap_or(1.0).max(1e-6);
                    if icon_button(
                        ui,
                        &crate::ui::tips::keyed("dialog.lab_actual_size"),
                        copy.is_some(),
                        view.zoom == Some(actual),
                        draw_actual,
                    ) {
                        view.zoom = Some(actual);
                    }
                },
                |img, _| crate::fx::apply_effect(img, &lab_effect(&params)),
            );
            if !picking {
                st.lab.tool = LabTool::Pan;
            }
            let Some(preview) = preview else {
                return;
            };
            // A white or black point picked on a picture.
            if let Some((x, y)) = preview.picked {
                pick_point(
                    &mut st.effect.params,
                    preview.img.get_pixel(x, y),
                    st.lab.tool == LabTool::WhitePoint,
                );
                st.lab.tool = LabTool::Pan;
            }
            left.add_space(4.0);
            left.horizontal(|ui| {
                let lab = &mut st.lab;
                if icon_button(
                    ui,
                    &crate::ui::tips::keyed("dialog.lab_undo"),
                    lab.can_undo(),
                    false,
                    |p, r, c| {
                        crate::ui::icons::draw_action(p, r, crate::ui::icons::Action::Undo, c)
                    },
                ) {
                    if let Some(v) = lab.undo() {
                        st.effect.params = v;
                    }
                }
                let lab = &mut st.lab;
                if icon_button(
                    ui,
                    &crate::ui::tips::keyed("dialog.lab_redo"),
                    lab.can_redo(),
                    false,
                    |p, r, c| {
                        crate::ui::icons::draw_action(p, r, crate::ui::icons::Action::Redo, c)
                    },
                ) {
                    if let Some(v) = lab.redo() {
                        st.effect.params = v;
                    }
                }
                if ui
                    .button(tr("dialog.lab_reset"))
                    .rich_tip(crate::ui::tips::keyed("dialog.lab_reset"))
                    .clicked()
                {
                    st.effect.params.clear();
                }
                ui.add_space(12.0);
                if ui
                    .add(egui::Button::new(tr("dialog.lab_snapshot")))
                    .rich_tip(crate::ui::tips::pair(
                        "dialog.lab_snapshot",
                        "dialog.lab_snapshot_tip",
                    ))
                    .clicked()
                {
                    st.lab.snapshot(&st.effect.params);
                }
            });
            snapshots(left, app, ctx, st, &src);
            right.horizontal(|ui| {
                if ui
                    .button(tr("dialog.lab_auto_adjust"))
                    .rich_tip(crate::ui::tips::keyed("dialog.lab_auto_adjust"))
                    .clicked()
                {
                    let (bp, wp) = auto_points(&preview.img);
                    st.effect.params.insert("black_point".into(), bp);
                    st.effect.params.insert("white_point".into(), wp);
                }
                let lab = &mut st.lab;
                for (tool, key, white) in [
                    (LabTool::WhitePoint, "dialog.lab_white_point", true),
                    (LabTool::BlackPoint, "dialog.lab_black_point", false),
                ] {
                    if icon_button(
                        ui,
                        &crate::ui::tips::keyed(key),
                        true,
                        lab.tool == tool,
                        |p, r, c| draw_picker(p, r, c, white),
                    ) {
                        lab.tool = if lab.tool == tool { LabTool::Pan } else { tool };
                    }
                }
            });
            right.add_space(6.0);
            right.spacing_mut().slider_width = 170.0;
            // One grid, so every slider lines up; a rule between groups.
            egui::Grid::new("lab_sliders")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(right, |ui| {
                    for (g, group) in SLIDERS.iter().enumerate() {
                        if g > 0 {
                            ui.separator();
                            ui.separator();
                            ui.end_row();
                        }
                        for name in group.iter() {
                            if let Some(p) = spec.params.iter().find(|p| p.name == *name) {
                                param_widget(ui, p, &mut st.effect.params, None);
                                ui.end_row();
                            }
                        }
                    }
                });
            right.separator();
            right.label(tr("dialog.lab_histogram"));
            let hist = histogram(&preview.after);
            draw_histogram(right, &hist, Vec2::new(RIGHT_W - 20.0, 96.0));
        });
        if super::dialogs::ok_cancel(ui, close) {
            let e = lab_effect(&st.effect.params);
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

/// The snapshot strip: numbered thumbnails; a click shows that version,
/// the cross deletes it.
fn snapshots(ui: &mut Ui, app: &App, ctx: &Context, st: &mut EffectState, src: &Source) {
    if st.lab.snapshots.is_empty() {
        return;
    }
    let Some(small) = copy_of(app, ctx, src, SNAP) else {
        return;
    };
    let mut restore = None;
    let mut remove = None;
    egui::ScrollArea::horizontal()
        .max_width(ui.available_width())
        .id_salt("lab_snapshots")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (n, params) in st.lab.snapshots.iter() {
                    let key = settings_key(params) ^ small.key;
                    let (tex, img) =
                        before_after::cached(ctx, egui::Id::new(("lab_snapshot", *n)), key, || {
                            crate::fx::apply_effect(&small.img, &lab_effect(params))
                        });
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(trf("dialog.lab_snapshot_n", &[("n", &n.to_string())]));
                            if ui
                                .small_button("\u{00D7}")
                                .rich_tip(crate::ui::tips::keyed("dialog.lab_delete_snapshot"))
                                .clicked()
                            {
                                remove = Some(*n);
                            }
                        });
                        let size = Vec2::new(img.width() as f32, img.height() as f32);
                        let (r, resp) =
                            ui.allocate_exact_size(Vec2::splat(SNAP as f32), Sense::click());
                        before_after::checkerboard(ui.painter(), r);
                        let fit = (r.width() / size.x.max(1.0)).min(r.height() / size.y.max(1.0));
                        ui.painter().image(
                            tex.id(),
                            Rect::from_center_size(r.center(), size * fit),
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        ui.painter().rect_stroke(
                            r,
                            0.0,
                            Stroke::new(1.0, Tokens::border()),
                            egui::StrokeKind::Inside,
                        );
                        if resp
                            .rich_tip(crate::ui::tips::keyed("dialog.lab_show_snapshot"))
                            .clicked()
                        {
                            restore = Some(params.clone());
                        }
                    });
                }
            });
        });
    if let Some(p) = restore {
        st.effect.params = p;
    }
    if let Some(n) = remove {
        st.lab.snapshots.retain(|(m, _)| *m != n);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn history_steps_back_and_forth_and_forgets_redo_after_a_change() {
        let mut lab = LabState::default();
        let mut p = BTreeMap::new();
        lab.record(&p, true);
        p.insert("brightness".to_string(), 20.0);
        lab.record(&p, true);
        lab.record(&p, true);
        p.insert("contrast".to_string(), 10.0);
        lab.record(&p, true);
        assert_eq!(lab.history.len(), 3);
        assert_eq!(lab.undo().and_then(|v| v.get("contrast").copied()), None);
        assert!(lab.undo().is_some_and(|v| v.is_empty()));
        assert!(lab.undo().is_none());
        assert!(lab.redo().is_some());
        // A new change drops the steps that were undone.
        let mut q = BTreeMap::new();
        q.insert("tint".to_string(), 5.0);
        lab.record(&q, true);
        assert!(!lab.can_redo());
        assert_eq!(lab.history.len(), 3);
        assert_eq!(lab.snapshot(&q), 1);
        assert_eq!(lab.snapshot(&p), 2);
        assert_eq!(lab.snapshots[1].1, p);
    }

    #[test]
    fn auto_points_cut_the_extremes_and_pickers_keep_order() {
        // Columns of grays from 60 to 200, and two outliers.
        let img: RgbaImage = ImageBuffer::from_fn(100, 100, |x, y| {
            let v = if x == 0 && y == 0 {
                0
            } else if x == 1 && y == 0 {
                255
            } else {
                (60 + x * 140 / 99) as u8
            };
            Rgba([v, v, v, 255])
        });
        let (bp, wp) = auto_points(&img);
        assert_eq!((bp, wp), (60.0, 200.0));
        let mut p = BTreeMap::new();
        p.insert("black_point".to_string(), 100.0);
        pick_point(&mut p, &Rgba([50, 50, 50, 255]), true);
        assert_eq!(p["white_point"], 101.0);
        pick_point(&mut p, &Rgba([200, 200, 200, 255]), false);
        assert_eq!(p["black_point"], 100.0);
        pick_point(&mut p, &Rgba([30, 30, 30, 255]), false);
        assert_eq!(p["black_point"], 30.0);
        // After Auto adjust the image spans the whole range.
        let mut q = BTreeMap::new();
        q.insert("black_point".to_string(), bp);
        q.insert("white_point".to_string(), wp);
        let out = crate::fx::apply_effect(&img, &lab_effect(&q));
        let hist = histogram(&out);
        assert!(hist[..4].iter().sum::<u32>() > 0 && hist[252..].iter().sum::<u32>() > 0);
    }
}
