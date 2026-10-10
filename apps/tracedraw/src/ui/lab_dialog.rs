//! Effects > Adjust > Image Adjustments: a
//! large preview with rotate, pan and zoom tools and three preview modes;
//! Auto adjust and the white and black point pickers; the colour and tone
//! sliders; a histogram of the result; undo, redo and reset inside the
//! dialog; numbered snapshots to compare versions. OK adds (or changes)
//! one Image Adjustments effect with the chosen settings.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::bitmap_preview::{cached, copy_of, settings_key, Source};
use crate::ui::effect_dialog::{param_widget, EffectState};
use egui::{epaint, Color32, Context, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use image::RgbaImage;
use std::collections::BTreeMap;
use tracedraw_core::BitmapEffect;

/// The preview copy's largest side, pixels.
const SIDE: u32 = 720;
/// The preview window, screen points.
const VIEW: Vec2 = Vec2::new(600.0, 420.0);
/// Snapshot thumbnails' largest side.
const SNAP: u32 = 96;

/// The settings the sliders show, in the target design's groups.
const SLIDERS: [&[&str]; 3] = [
    &["temperature", "tint", "saturation"],
    &["brightness", "contrast"],
    &["highlights", "shadows", "midtones"],
];

/// What a click or drag in the preview does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabTool {
    #[default]
    Pan,
    ZoomIn,
    ZoomOut,
    WhitePoint,
    BlackPoint,
}

/// How the preview shows the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabView {
    /// The corrected image only.
    #[default]
    Full,
    /// The original and the corrected image side by side.
    BeforeAfter,
    /// One image, the original left of a divider and the result right.
    Split,
}

/// The dialog's own state besides the settings.
#[derive(Debug, Clone, PartialEq)]
pub struct LabState {
    pub tool: LabTool,
    pub view: LabView,
    /// Screen points per preview pixel; `None` fits the image.
    pub zoom: Option<f32>,
    /// The point of the turned image at the window's middle, preview
    /// pixels; `None` is the image's middle.
    pub centre: Option<Vec2>,
    /// Quarter turns clockwise of the view.
    pub turns: u8,
    /// The split view's divider, 0 (left) to 1 (right).
    pub split: f32,
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
            view: LabView::Full,
            zoom: None,
            centre: None,
            turns: 0,
            split: 0.5,
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

/// The image's size after `turns` quarter turns.
fn turned_size(w: f32, h: f32, turns: u8) -> Vec2 {
    if turns % 2 == 1 {
        Vec2::new(h, w)
    } else {
        Vec2::new(w, h)
    }
}

/// A point of the turned image back to the image's own pixels.
pub fn unturn(d: Vec2, w: f32, h: f32, turns: u8) -> Vec2 {
    match turns % 4 {
        1 => Vec2::new(d.y, h - d.x),
        2 => Vec2::new(w - d.x, h - d.y),
        3 => Vec2::new(w - d.y, d.x),
        _ => d,
    }
}

/// Draw the texture into `r` turned by quarter turns clockwise.
fn paint_turned(painter: &Painter, tex: egui::TextureId, r: Rect, turns: u8) {
    let base = [
        Pos2::new(0.0, 0.0),
        Pos2::new(1.0, 0.0),
        Pos2::new(1.0, 1.0),
        Pos2::new(0.0, 1.0),
    ];
    let corners = [
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
    ];
    let k = (turns % 4) as usize;
    let mut mesh = epaint::Mesh::with_texture(tex);
    for (i, c) in corners.iter().enumerate() {
        mesh.vertices.push(epaint::Vertex {
            pos: *c,
            uv: base[(i + 4 - k) % 4],
            color: Color32::WHITE,
        });
    }
    mesh.indices = vec![0, 1, 2, 0, 2, 3];
    painter.add(egui::Shape::mesh(mesh));
}

fn checker(painter: &Painter, r: Rect) {
    painter.rect_filled(r, 0.0, Color32::from_gray(0xFF));
    let cell = 8.0;
    let (nx, ny) = (
        (r.width() / cell).ceil() as i32,
        (r.height() / cell).ceil() as i32,
    );
    for j in 0..ny {
        for i in 0..nx {
            if (i + j) % 2 == 1 {
                let c = Rect::from_min_size(
                    r.min + Vec2::new(i as f32 * cell, j as f32 * cell),
                    Vec2::splat(cell),
                )
                .intersect(r);
                painter.rect_filled(c, 0.0, Color32::from_gray(0xE6));
            }
        }
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
        Tokens::ICON
    } else {
        Tokens::BORDER
    };
    draw(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
        color,
    );
    let clicked = enabled && resp.clicked();
    resp.on_hover_text(tip);
    clicked
}

fn at(r: Rect, x: f32, y: f32) -> Pos2 {
    r.min + Vec2::new(x, y) * (r.width() / 16.0)
}

/// A circular arrow, counter-clockwise when `left`.
fn draw_rotate(p: &Painter, r: Rect, c: Color32, left: bool) {
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

fn draw_zoom(p: &Painter, r: Rect, c: Color32, plus: bool) {
    crate::ui::icons::draw(p, r, crate::tools::Tool::Zoom, c);
    let k = r.width() / 16.0;
    let m = at(r, 6.8, 6.8);
    let st = Stroke::new(1.2 * k, c);
    p.line_segment(
        [m - Vec2::new(2.4 * k, 0.0), m + Vec2::new(2.4 * k, 0.0)],
        st,
    );
    if plus {
        p.line_segment(
            [m - Vec2::new(0.0, 2.4 * k), m + Vec2::new(0.0, 2.4 * k)],
            st,
        );
    }
}

fn draw_fit(p: &Painter, r: Rect, c: Color32) {
    let k = r.width() / 16.0;
    let st = Stroke::new(1.2 * k, c);
    p.rect_stroke(
        Rect::from_min_max(at(r, 4.0, 4.5), at(r, 12.0, 11.5)),
        0.0,
        Stroke::new(0.9 * k, c),
        egui::StrokeKind::Middle,
    );
    for (x, y, dx, dy) in [
        (1.0, 1.0, 1.0, 1.0),
        (15.0, 1.0, -1.0, 1.0),
        (1.0, 15.0, 1.0, -1.0),
        (15.0, 15.0, -1.0, -1.0),
    ] {
        let o = at(r, x, y);
        p.line_segment([o, o + Vec2::new(3.0 * dx, 0.0) * k], st);
        p.line_segment([o, o + Vec2::new(0.0, 3.0 * dy) * k], st);
    }
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

fn draw_view(p: &Painter, r: Rect, c: Color32, v: LabView) {
    let k = r.width() / 16.0;
    let st = Stroke::new(1.0 * k, c);
    match v {
        LabView::Full => {
            p.rect_stroke(
                Rect::from_min_max(at(r, 2.0, 3.0), at(r, 14.0, 13.0)),
                0.0,
                st,
                egui::StrokeKind::Middle,
            );
        }
        LabView::BeforeAfter => {
            p.rect_stroke(
                Rect::from_min_max(at(r, 1.0, 4.0), at(r, 7.5, 12.0)),
                0.0,
                st,
                egui::StrokeKind::Middle,
            );
            p.rect_stroke(
                Rect::from_min_max(at(r, 8.5, 4.0), at(r, 15.0, 12.0)),
                0.0,
                st,
                egui::StrokeKind::Middle,
            );
        }
        LabView::Split => {
            p.rect_stroke(
                Rect::from_min_max(at(r, 2.0, 3.0), at(r, 14.0, 13.0)),
                0.0,
                st,
                egui::StrokeKind::Middle,
            );
            for i in 0..4 {
                let y = 3.5 + i as f32 * 2.6;
                p.line_segment([at(r, 8.0, y), at(r, 8.0, y + 1.4)], st);
            }
        }
    }
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

/// The preview pixel under a screen point of an image drawn at
/// `img_rect` (`k` screen points per preview pixel), or `None` outside.
fn pixel_at(pos: Pos2, img_rect: Rect, k: f32, w: u32, h: u32, turns: u8) -> Option<(u32, u32)> {
    let d = (pos - img_rect.min) / k.max(1e-6);
    let q = unturn(d, w as f32, h as f32, turns);
    if q.x < 0.0 || q.y < 0.0 || q.x >= w as f32 || q.y >= h as f32 {
        return None;
    }
    Some((q.x as u32, q.y as u32))
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
        Stroke::new(1.0, Tokens::BORDER),
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

/// The preview window: the image (or both) drawn with the view's zoom,
/// centre and turns, and the tools' clicks and drags.
fn preview(
    ui: &mut Ui,
    st: &mut EffectState,
    before: &RgbaImage,
    before_tex: egui::TextureId,
    after_tex: egui::TextureId,
) {
    let (area, resp) = ui.allocate_exact_size(VIEW, Sense::click_and_drag());
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, Color32::from_gray(0xC8));
    let (w, h) = before.dimensions();
    let lab = &mut st.lab;
    let size = turned_size(w as f32, h as f32, lab.turns);
    // Each picture's window: the whole area, or a half of it.
    let panes: Vec<Rect> = match lab.view {
        LabView::BeforeAfter => {
            let half = (area.width() - 6.0) / 2.0;
            vec![
                Rect::from_min_size(area.min, Vec2::new(half, area.height())),
                Rect::from_min_size(
                    area.min + Vec2::new(half + 6.0, 0.0),
                    Vec2::new(half, area.height()),
                ),
            ]
        }
        _ => vec![area],
    };
    let pane = panes[0];
    let fit = (pane.width() / size.x.max(1.0)).min(pane.height() / size.y.max(1.0));
    let k = lab.zoom.unwrap_or(fit).clamp(fit.min(1.0) * 0.25, 32.0);
    let centre = lab.centre.unwrap_or(size / 2.0);
    let img_rect = |pane: Rect| Rect::from_min_size(pane.center() - centre * k, size * k);
    match lab.view {
        LabView::Full => {
            let r = img_rect(pane);
            checker(&painter, r.intersect(pane));
            paint_turned(&painter, after_tex, r, lab.turns);
        }
        LabView::BeforeAfter => {
            for (i, p) in panes.iter().enumerate() {
                let r = img_rect(*p);
                let pp = ui.painter_at(*p);
                checker(&pp, r.intersect(*p));
                paint_turned(
                    &pp,
                    if i == 0 { before_tex } else { after_tex },
                    r,
                    lab.turns,
                );
            }
        }
        LabView::Split => {
            let r = img_rect(pane);
            checker(&painter, r.intersect(pane));
            let x = pane.left() + pane.width() * lab.split;
            let left = Rect::from_min_max(pane.min, Pos2::new(x, pane.bottom()));
            let right = Rect::from_min_max(Pos2::new(x, pane.top()), pane.max);
            paint_turned(&ui.painter_at(left), before_tex, r, lab.turns);
            paint_turned(&ui.painter_at(right), after_tex, r, lab.turns);
            let mut y = pane.top();
            while y < pane.bottom() {
                painter.line_segment(
                    [Pos2::new(x, y), Pos2::new(x, (y + 6.0).min(pane.bottom()))],
                    Stroke::new(1.0, Color32::BLACK),
                );
                painter.line_segment(
                    [
                        Pos2::new(x + 1.0, y + 6.0),
                        Pos2::new(x + 1.0, (y + 12.0).min(pane.bottom())),
                    ],
                    Stroke::new(1.0, Color32::WHITE),
                );
                y += 12.0;
            }
        }
    }
    painter.rect_stroke(
        area,
        0.0,
        Stroke::new(1.0, Tokens::BORDER),
        egui::StrokeKind::Inside,
    );

    // Dragging the split view's divider wins over the tools.
    let divider_id = ui.id().with("lab_divider");
    let mut on_divider = ui.data(|d| d.get_temp::<bool>(divider_id)).unwrap_or(false);
    if resp.drag_started() {
        on_divider = lab.view == LabView::Split
            && resp
                .interact_pointer_pos()
                .is_some_and(|p| (p.x - (pane.left() + pane.width() * lab.split)).abs() < 6.0);
        ui.data_mut(|d| d.insert_temp(divider_id, on_divider));
    }
    if resp.dragged() && on_divider {
        if let Some(p) = resp.interact_pointer_pos() {
            lab.split = ((p.x - pane.left()) / pane.width()).clamp(0.0, 1.0);
        }
        return;
    }
    if resp.hovered() && lab.view == LabView::Split {
        if let Some(p) = resp.hover_pos() {
            if (p.x - (pane.left() + pane.width() * lab.split)).abs() < 6.0 {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
        }
    }
    // Which picture a point is over, and where in the turned image.
    let locate = |pos: Pos2| -> Option<(Rect, Vec2)> {
        let p = panes.iter().find(|p| p.contains(pos))?;
        let r = img_rect(*p);
        Some((r, (pos - r.min) / k))
    };
    match lab.tool {
        LabTool::Pan => {
            if resp.dragged() {
                let d = resp.drag_delta() / k;
                let c = (centre - d).clamp(Vec2::ZERO, size);
                lab.centre = Some(c);
                lab.zoom = Some(k);
            }
        }
        LabTool::ZoomIn | LabTool::ZoomOut => {
            if resp.clicked() {
                if let Some((_, d)) = resp.interact_pointer_pos().and_then(locate) {
                    let f = if lab.tool == LabTool::ZoomIn {
                        1.5
                    } else {
                        1.0 / 1.5
                    };
                    lab.zoom = Some((k * f).clamp(fit.min(1.0) * 0.25, 32.0));
                    lab.centre = Some(d.clamp(Vec2::ZERO, size));
                }
            }
        }
        LabTool::WhitePoint | LabTool::BlackPoint => {
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
            if resp.clicked() {
                if let Some(pos) = resp.interact_pointer_pos() {
                    if let Some((r, _)) = locate(pos) {
                        if let Some((x, y)) = pixel_at(pos, r, k, w, h, lab.turns) {
                            pick_point(
                                &mut st.effect.params,
                                before.get_pixel(x, y),
                                lab.tool == LabTool::WhitePoint,
                            );
                        }
                    }
                }
            }
        }
    }
    // The wheel zooms about the pointer, any tool.
    let wheel = ui.input(|i| i.smooth_scroll_delta.y);
    if resp.hovered() && wheel.abs() > 0.5 {
        if let Some(d) = resp.hover_pos().and_then(locate).map(|(_, d)| d) {
            let f = (wheel / 200.0).exp();
            let nk = (k * f).clamp(fit.min(1.0) * 0.25, 32.0);
            // Keep the point under the pointer where it is.
            let c = d + (centre - d) * (k / nk);
            st.lab.zoom = Some(nk);
            st.lab.centre = Some(c.clamp(Vec2::ZERO, size));
        }
    }
}

/// The Image Adjustments dialog.
pub fn lab_dialog(app: &mut App, ctx: &Context, st: &mut EffectState, close: &mut bool) {
    let Some(spec) = crate::fx::spec("image_adjustments") else {
        *close = true;
        return;
    };
    let title = tr("fx.image_adjustments");
    super::dialogs::window(ctx, title).show(ctx, |ui| {
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
        let Some(copy) = copy_of(app, ctx, &src, SIDE) else {
            ui.label(tr("dialog.no_bitmap_preview"));
            super::dialogs::ok_cancel(ui, close);
            return;
        };
        let settled = !ui.input(|i| i.pointer.any_down());
        st.lab.record(&st.effect.params, settled);
        let params = st.effect.params.clone();
        let key = settings_key(&params) ^ copy.key.rotate_left(13);
        let (after_tex, after) = cached(ctx, "lab_after", key, || {
            crate::fx::apply_effect(&copy.img, &lab_effect(&params))
        });
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let lab = &mut st.lab;
                    if icon_button(ui, &tr("dialog.lab_rotate_left"), true, false, |p, r, c| {
                        draw_rotate(p, r, c, true)
                    }) {
                        lab.turns = (lab.turns + 3) % 4;
                        lab.centre = None;
                    }
                    if icon_button(
                        ui,
                        &tr("dialog.lab_rotate_right"),
                        true,
                        false,
                        |p, r, c| draw_rotate(p, r, c, false),
                    ) {
                        lab.turns = (lab.turns + 1) % 4;
                        lab.centre = None;
                    }
                    ui.add_space(6.0);
                    for (tool, key) in [
                        (LabTool::Pan, "dialog.lab_pan"),
                        (LabTool::ZoomIn, "dialog.lab_zoom_in"),
                        (LabTool::ZoomOut, "dialog.lab_zoom_out"),
                    ] {
                        let on = lab.tool == tool;
                        if icon_button(ui, &tr(key), true, on, |p, r, c| match tool {
                            LabTool::Pan => {
                                crate::ui::icons::draw(p, r, crate::tools::Tool::Pan, c)
                            }
                            _ => draw_zoom(p, r, c, tool == LabTool::ZoomIn),
                        }) {
                            lab.tool = tool;
                        }
                    }
                    if icon_button(
                        ui,
                        &tr("dialog.lab_zoom_fit"),
                        true,
                        lab.zoom.is_none(),
                        draw_fit,
                    ) {
                        lab.zoom = None;
                        lab.centre = None;
                    }
                    let actual = 1.0 / copy.scale.max(1e-6);
                    if icon_button(
                        ui,
                        &tr("dialog.lab_actual_size"),
                        true,
                        lab.zoom == Some(actual),
                        draw_actual,
                    ) {
                        lab.zoom = Some(actual);
                    }
                    ui.add_space(6.0);
                    for (view, key) in [
                        (LabView::Full, "dialog.lab_full_preview"),
                        (LabView::BeforeAfter, "dialog.lab_before_after"),
                        (LabView::Split, "dialog.lab_split_preview"),
                    ] {
                        if icon_button(ui, &tr(key), true, lab.view == view, |p, r, c| {
                            draw_view(p, r, c, view)
                        }) {
                            lab.view = view;
                        }
                    }
                });
                preview(ui, st, &copy.img, copy.tex.id(), after_tex.id());
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let lab = &mut st.lab;
                    if icon_button(
                        ui,
                        &tr("dialog.lab_undo"),
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
                        &tr("dialog.lab_redo"),
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
                    if ui.button(tr("dialog.lab_reset")).clicked() {
                        st.effect.params.clear();
                    }
                    ui.add_space(12.0);
                    if ui
                        .add(egui::Button::new(tr("dialog.lab_snapshot")))
                        .on_hover_text(tr("dialog.lab_snapshot_tip"))
                        .clicked()
                    {
                        st.lab.snapshot(&st.effect.params);
                    }
                });
                snapshots(ui, app, ctx, st, &src);
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.set_width(300.0);
                ui.horizontal(|ui| {
                    if ui.button(tr("dialog.lab_auto_adjust")).clicked() {
                        let (bp, wp) = auto_points(&copy.img);
                        st.effect.params.insert("black_point".into(), bp);
                        st.effect.params.insert("white_point".into(), wp);
                    }
                    let lab = &mut st.lab;
                    for (tool, key, white) in [
                        (LabTool::WhitePoint, "dialog.lab_white_point", true),
                        (LabTool::BlackPoint, "dialog.lab_black_point", false),
                    ] {
                        if icon_button(ui, &tr(key), true, lab.tool == tool, |p, r, c| {
                            draw_picker(p, r, c, white)
                        }) {
                            lab.tool = if lab.tool == tool { LabTool::Pan } else { tool };
                        }
                    }
                });
                ui.add_space(6.0);
                ui.spacing_mut().slider_width = 170.0;
                // One grid, so every slider lines up; a rule between groups.
                egui::Grid::new("lab_sliders")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
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
                ui.separator();
                ui.label(tr("dialog.lab_histogram"));
                let hist = histogram(&after);
                draw_histogram(ui, &hist, Vec2::new(290.0, 96.0));
            });
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
        .max_width(VIEW.x)
        .id_salt("lab_snapshots")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (n, params) in st.lab.snapshots.iter() {
                    let key = settings_key(params) ^ small.key;
                    let (tex, img) = cached(ctx, &format!("lab_snapshot_{n}"), key, || {
                        crate::fx::apply_effect(&small.img, &lab_effect(params))
                    });
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(trf("dialog.lab_snapshot_n", &[("n", &n.to_string())]));
                            if ui
                                .small_button("\u{2715}")
                                .on_hover_text(tr("dialog.lab_delete_snapshot"))
                                .clicked()
                            {
                                remove = Some(*n);
                            }
                        });
                        let size = Vec2::new(img.width() as f32, img.height() as f32);
                        let (r, resp) =
                            ui.allocate_exact_size(Vec2::splat(SNAP as f32), Sense::click());
                        checker(ui.painter(), r);
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
                            Stroke::new(1.0, Tokens::BORDER),
                            egui::StrokeKind::Inside,
                        );
                        if resp.on_hover_text(tr("dialog.lab_show_snapshot")).clicked() {
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

    #[test]
    fn turned_views_map_back_to_the_same_pixels() {
        let (w, h) = (40.0, 30.0);
        let pixel = Vec2::new(5.5, 7.5);
        for turns in 0..4u8 {
            // Turn the point forward, then back.
            let size = turned_size(w, h, turns);
            let fwd = match turns {
                1 => Vec2::new(h - pixel.y, pixel.x),
                2 => Vec2::new(w - pixel.x, h - pixel.y),
                3 => Vec2::new(pixel.y, w - pixel.x),
                _ => pixel,
            };
            assert!(fwd.x <= size.x && fwd.y <= size.y);
            let back = unturn(fwd, w, h, turns);
            assert!((back - pixel).length() < 1e-4, "{turns}: {back:?}");
        }
        let r = Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(80.0, 60.0));
        assert_eq!(
            pixel_at(Pos2::new(111.0, 65.0), r, 2.0, 40, 30, 0),
            Some((5, 7))
        );
        assert_eq!(pixel_at(Pos2::new(99.0, 65.0), r, 2.0, 40, 30, 0), None);
    }
}
