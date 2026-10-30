//! The Before and After preview every dialog that changes a picture
//! shares (Trace Bitmap, Remove Background, the colour mode dialogs, the
//! bitmap effects and Image Adjustments): a Preview list (Before and
//! After stacked, side by side, Before only, After only, and what the
//! dialog adds: the split view, the wireframe over the picture), zoom to
//! fit, zoom out, zoom in and a pan button, the panes with one zoom and
//! place (a drag pans, the wheel zooms about the pointer), a
//! checkerboard under transparent parts, and room for a status line.
//! The panes fill the dialog and grow with it. A frame draws textures
//! made once per change: nothing is worked out again while the settings
//! stand (see [`live`]).

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::bitmap_preview::{copy_of, Source, Thumb, THUMB};
use crate::ui::tips::RichTip;
use egui::{epaint, Color32, Context, Painter, Pos2, Rect, Sense, Stroke, TextureId, Ui, Vec2};
use image::RgbaImage;
use std::sync::Arc;

/// What the preview shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    /// The picture above, the result below.
    #[default]
    Stacked,
    /// The picture at the left, the result at the right.
    SideBySide,
    Before,
    After,
    /// The result alone, taking the whole preview (Trace Bitmap's name
    /// for it).
    Large,
    /// One picture: the original left of a divider, the result right.
    Split,
    /// The dialog's overlay (the trace's curves) over the picture, which
    /// shows through at [`ViewState::overlay`] percent.
    Overlay,
}

impl View {
    /// The views every dialog offers.
    pub const PLAIN: [View; 4] = [View::Stacked, View::SideBySide, View::Before, View::After];

    pub fn key(self) -> &'static str {
        match self {
            View::Stacked => "dialog.trace_view_before_after",
            View::SideBySide => "dialog.view_side_by_side",
            View::Before => "dialog.before",
            View::After => "dialog.after",
            View::Large => "dialog.trace_view_large",
            View::Split => "dialog.view_split",
            View::Overlay => "dialog.trace_view_wireframe",
        }
    }
}

/// How much of the picture shows under the wireframe at first, percent.
pub const OVERLAY_DEFAULT: f32 = 80.0;

/// How the preview is looked at; part of each dialog's state.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewState {
    pub view: View,
    /// Screen points per preview pixel; `None` fits the picture.
    pub zoom: Option<f32>,
    /// The point of the picture (as fractions of its turned size) at the
    /// middle of each pane; `None` its middle.
    pub centre: Option<Vec2>,
    /// Quarter turns clockwise of the view.
    pub turns: u8,
    /// The split view's divider, 0 (left) to 1 (right).
    pub split: f32,
    /// How strongly the picture shows under the wireframe, 0 to 100.
    pub overlay: f32,
}

impl Default for ViewState {
    fn default() -> Self {
        ViewState {
            view: View::Stacked,
            zoom: None,
            centre: None,
            turns: 0,
            split: 0.5,
            overlay: OVERLAY_DEFAULT,
        }
    }
}

impl ViewState {
    pub fn with_view(view: View) -> Self {
        ViewState {
            view,
            ..Default::default()
        }
    }

    /// Back to the whole picture in the middle.
    pub fn fit(&mut self) {
        self.zoom = None;
        self.centre = None;
    }
}

/// The smallest and largest zoom, as factors of the fit (and absolute
/// points per pixel at the top).
const ZOOM_MIN_OF_FIT: f32 = 0.1;
const ZOOM_MAX: f32 = 64.0;
/// A zoom button's step.
const ZOOM_STEP: f32 = 1.25;
/// The gap between panes, points.
const GAP: f32 = 4.0;
/// The toolbar's height and the status line's, points.
const BAR_H: f32 = 26.0;
pub const STATUS_H: f32 = 22.0;

/// Where the preview's controls are this frame, for the tests that
/// drive a dialog with the pointer.
#[cfg(test)]
#[derive(Clone, Default)]
pub struct Rects(pub std::collections::HashMap<&'static str, Rect>);

#[cfg(test)]
pub fn rects(ctx: &Context) -> Rects {
    ctx.data(|d| d.get_temp::<Rects>(egui::Id::new("preview_rects")))
        .unwrap_or_default()
}

/// Forget the places remembered: a dialog's frame starts (tests only).
fn clear_marks(ctx: &Context) {
    #[cfg(test)]
    ctx.data_mut(|d| d.remove::<Rects>(egui::Id::new("preview_rects")));
    #[cfg(not(test))]
    let _ = ctx;
}

/// Remember where `name` is drawn this frame (tests only).
pub fn mark(ui: &Ui, name: &'static str, rect: Rect) {
    #[cfg(test)]
    ui.ctx().data_mut(|d| {
        let r: &mut Rects = d.get_temp_mut_or_default(egui::Id::new("preview_rects"));
        r.0.insert(name, rect);
    });
    #[cfg(not(test))]
    let _ = (ui, name, rect);
}

/// An icon button 26 points square; `pressed` shows it pushed in.
pub fn icon_button(
    ui: &mut Ui,
    tip: &str,
    pressed: bool,
    enabled: bool,
    draw: impl FnOnce(&Painter, Rect, Color32),
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::splat(BAR_H),
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
        Tokens::text_dim()
    };
    draw(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
        color,
    );
    resp.rich_tip(tip)
}

fn at(r: Rect, x: f32, y: f32) -> Pos2 {
    r.min + Vec2::new(x, y) * (r.width() / 16.0)
}

/// The zoom glass with a minus, or a plus.
pub fn draw_zoom(p: &Painter, r: Rect, c: Color32, plus: bool) {
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

/// A frame with the four corners marked: the whole picture in view.
pub fn draw_fit(p: &Painter, r: Rect, c: Color32) {
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

/// What the toolbar's buttons did.
#[derive(Default)]
pub struct BarOut {
    /// The pan button was clicked: a dialog picking a colour on the
    /// preview goes back to panning.
    pub pan: bool,
}

/// The Preview list over `views`, zoom to fit, zoom out, zoom in, the pan
/// button (pressed unless `picking`), a spinner while `busy`, and `extra`
/// buttons of the dialog's own after them.
#[allow(clippy::too_many_arguments)]
pub fn toolbar(
    ui: &mut Ui,
    id: egui::Id,
    st: &mut ViewState,
    views: &[View],
    busy: bool,
    picking: bool,
    extra: impl FnOnce(&mut Ui, &mut ViewState),
) -> BarOut {
    let mut out = BarOut::default();
    clear_marks(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(tr("dialog.trace_preview_label"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::keyed("dialog.trace_preview_label"),
            |ui| {
                let resp = egui::ComboBox::from_id_salt(id.with("view"))
                    .selected_text(tr(st.view.key()))
                    .width(170.0)
                    .show_ui(ui, |ui| {
                        for v in views {
                            ui.selectable_value(&mut st.view, *v, tr(v.key()));
                        }
                    })
                    .response;
                mark(ui, "trace_view", resp.rect);
            },
        );
        if st.view == View::Overlay {
            // How much the picture shows under the wireframe.
            ui.add_space(8.0);
            ui.label(tr("dialog.trace_overlay"));
            let r = crate::ui::tips::tip_over(
                ui,
                crate::ui::tips::keyed("dialog.trace_overlay"),
                |ui| {
                    ui.add(
                        crate::ui::Slider::new(&mut st.overlay, 0.0..=100.0)
                            .integer()
                            .width(150.0),
                    )
                },
            );
            // The groove alone (the number box sits at its right).
            mark(
                ui,
                "overlay",
                Rect::from_min_size(r.rect.left_top(), Vec2::new(150.0, r.rect.height())),
            );
        }
        ui.add_space(8.0);
        let r = icon_button(
            ui,
            &crate::ui::tips::keyed("dialog.lab_zoom_fit"),
            st.zoom.is_none(),
            true,
            draw_fit,
        );
        mark(ui, "zoom_fit", r.rect);
        if r.clicked() {
            st.fit();
        }
        let fit = ui
            .ctx()
            .data(|d| d.get_temp::<f32>(id.with("fit")))
            .unwrap_or(1.0);
        for (zoom_in, key) in [(false, "dialog.lab_zoom_out"), (true, "dialog.lab_zoom_in")] {
            let r = icon_button(ui, &crate::ui::tips::keyed(key), false, true, |p, r, c| {
                draw_zoom(p, r, c, zoom_in)
            });
            mark(ui, if zoom_in { "zoom_in" } else { "zoom_out" }, r.rect);
            if r.clicked() {
                let k = st.zoom.unwrap_or(fit);
                let k = if zoom_in {
                    k * ZOOM_STEP
                } else {
                    k / ZOOM_STEP
                };
                st.zoom = Some(k.clamp((fit * ZOOM_MIN_OF_FIT).min(0.02), ZOOM_MAX));
            }
        }
        let r = icon_button(
            ui,
            &crate::ui::tips::keyed("dialog.lab_pan"),
            !picking,
            true,
            |p, r, c| crate::ui::icons::draw(p, r, crate::tools::Tool::Pan, c),
        );
        mark(ui, "pan", r.rect);
        out.pan = r.clicked();
        if busy {
            let r = ui.add(egui::Spinner::new().size(16.0));
            mark(ui, "spinner", r.rect);
        }
        extra(ui, st);
    });
    out
}

/// The pictures a frame of the preview draws, all at the copy's size.
pub struct Pictures {
    pub before: Option<TextureId>,
    pub after: Option<TextureId>,
    /// Drawn over the picture in the overlay view.
    pub overlay: Option<TextureId>,
    /// The copy's size, pixels.
    pub size: Vec2,
}

/// What a frame of the panes did.
#[derive(Default)]
pub struct ViewOut {
    /// A click (while picking) on a picture, as fractions of the
    /// picture's own size (turns undone).
    pub picked: Option<Vec2>,
    /// Points per pixel that fit the picture in a pane.
    pub fit: f32,
}

/// The size of the picture after `turns` quarter turns.
fn turned_size(size: Vec2, turns: u8) -> Vec2 {
    if turns % 2 == 1 {
        Vec2::new(size.y, size.x)
    } else {
        size
    }
}

/// A point of the turned picture (fractions) back to the picture's own
/// fractions.
pub fn unturn(d: Vec2, turns: u8) -> Vec2 {
    match turns % 4 {
        1 => Vec2::new(d.y, 1.0 - d.x),
        2 => Vec2::new(1.0 - d.x, 1.0 - d.y),
        3 => Vec2::new(1.0 - d.y, d.x),
        _ => d,
    }
}

/// Draw the texture into `r` turned by quarter turns clockwise, through
/// `tint` (white draws it as it is; a translucent white fades it).
fn paint_turned(painter: &Painter, tex: TextureId, r: Rect, turns: u8, tint: Color32) {
    if turns.is_multiple_of(4) {
        painter.image(
            tex,
            r,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            tint,
        );
        return;
    }
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
            color: tint,
        });
    }
    mesh.indices = vec![0, 1, 2, 0, 2, 3];
    painter.add(egui::Shape::mesh(mesh));
}

/// The pane rectangles of `view` in `area`.
fn pane_rects(area: Rect, view: View) -> Vec<Rect> {
    match view {
        View::Stacked => {
            let h = (area.height() - GAP) / 2.0;
            vec![
                Rect::from_min_size(area.min, Vec2::new(area.width(), h)),
                Rect::from_min_size(
                    area.min + Vec2::new(0.0, h + GAP),
                    Vec2::new(area.width(), h),
                ),
            ]
        }
        View::SideBySide => {
            let w = (area.width() - GAP) / 2.0;
            vec![
                Rect::from_min_size(area.min, Vec2::new(w, area.height())),
                Rect::from_min_size(
                    area.min + Vec2::new(w + GAP, 0.0),
                    Vec2::new(w, area.height()),
                ),
            ]
        }
        _ => vec![area],
    }
}

/// The points per pixel that fit a picture of `size` (turned) in a pane
/// of `view` in `area`.
pub fn fit_zoom(area: Rect, view: View, size: Vec2, turns: u8) -> f32 {
    let pane = pane_rects(area, view)[0];
    let size = turned_size(size, turns);
    ((pane.width() - 2.0) / size.x.max(1.0)).min((pane.height() - 2.0) / size.y.max(1.0))
}

/// Draw the panes of `st.view` in `area` with the one zoom and centre;
/// a drag pans (or moves the split view's divider), the wheel zooms
/// about the pointer, a click while `picking` picks a point.
pub fn view(
    ui: &mut Ui,
    id: egui::Id,
    st: &mut ViewState,
    area: Rect,
    pics: &Pictures,
    picking: bool,
) -> ViewOut {
    let mut out = ViewOut::default();
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, Color32::from_gray(0xC8));
    let panes = pane_rects(area, st.view);
    let size = turned_size(pics.size, st.turns);
    let fit = fit_zoom(area, st.view, pics.size, st.turns);
    out.fit = fit;
    ui.ctx().data_mut(|d| d.insert_temp(id.with("fit"), fit));
    let lo = (fit * ZOOM_MIN_OF_FIT).min(0.02);
    let k = st.zoom.unwrap_or(fit).clamp(lo, ZOOM_MAX);
    let centre = st.centre.unwrap_or(Vec2::splat(0.5));
    let resp = ui.interact(area, id.with("panes"), Sense::click_and_drag());
    let img_rect = |pane: Rect| Rect::from_min_size(pane.center() - centre * size * k, size * k);
    let (before_l, after_l) = (tr("dialog.before"), tr("dialog.after"));
    let pane_pics: Vec<(Rect, Option<TextureId>, Option<TextureId>, &str)> = match st.view {
        View::Stacked | View::SideBySide => vec![
            (panes[0], pics.before, None, before_l.as_str()),
            (panes[1], pics.after, None, after_l.as_str()),
        ],
        View::Before => vec![(panes[0], pics.before, None, "")],
        View::After | View::Large => vec![(panes[0], pics.after, None, "")],
        View::Split => vec![(panes[0], pics.before, None, "")],
        View::Overlay => vec![(panes[0], pics.before, pics.overlay, "")],
    };
    // Under the wireframe the picture shows at the Overlay slider's
    // strength: a tint on the one textured rectangle, nothing worked out.
    let tint = if st.view == View::Overlay {
        Color32::from_white_alpha((st.overlay.clamp(0.0, 100.0) * 2.55).round() as u8)
    } else {
        Color32::WHITE
    };
    for (i, (pane, tex, over, label)) in pane_pics.iter().enumerate() {
        let p = ui.painter_at(*pane);
        p.rect_filled(*pane, 0.0, Color32::from_gray(0xC8));
        let r = img_rect(*pane);
        checkerboard(&p, r.intersect(*pane));
        if st.view == View::Split {
            // The original left of the divider, the result right of it.
            let x = pane.left() + pane.width() * st.split;
            let left = Rect::from_min_max(pane.min, Pos2::new(x, pane.bottom()));
            let right = Rect::from_min_max(Pos2::new(x, pane.top()), pane.max);
            if let Some(t) = pics.before {
                paint_turned(&ui.painter_at(left), t, r, st.turns, Color32::WHITE);
            }
            if let Some(t) = pics.after {
                paint_turned(&ui.painter_at(right), t, r, st.turns, Color32::WHITE);
            }
            let mut y = pane.top();
            while y < pane.bottom() {
                p.line_segment(
                    [Pos2::new(x, y), Pos2::new(x, (y + 6.0).min(pane.bottom()))],
                    Stroke::new(1.0, Color32::BLACK),
                );
                p.line_segment(
                    [
                        Pos2::new(x + 1.0, y + 6.0),
                        Pos2::new(x + 1.0, (y + 12.0).min(pane.bottom())),
                    ],
                    Stroke::new(1.0, Color32::WHITE),
                );
                y += 12.0;
            }
        } else {
            if let Some(t) = tex {
                paint_turned(&p, *t, r, st.turns, tint);
            }
            if let Some(t) = over {
                paint_turned(&p, *t, r, st.turns, Color32::WHITE);
            }
        }
        if !label.is_empty() {
            let galley = p.layout_no_wrap(
                label.to_string(),
                egui::FontId::proportional(11.0),
                Tokens::text(),
            );
            let tag = Rect::from_min_size(
                pane.left_top() + Vec2::new(4.0, 4.0),
                galley.size() + Vec2::new(8.0, 4.0),
            );
            p.rect_filled(tag, 2.0, Color32::from_white_alpha(200));
            p.galley(tag.min + Vec2::new(4.0, 2.0), galley, Tokens::text());
        }
        p.rect_stroke(
            *pane,
            0.0,
            Stroke::new(1.0, Tokens::border()),
            egui::StrokeKind::Inside,
        );
        if i == 0 {
            mark(ui, "view_pane", *pane);
            mark(ui, "view_image", r);
        }
    }
    // Dragging the split view's divider wins over panning.
    let divider_id = id.with("divider");
    let (split_view, split, first) = (st.view == View::Split, st.split, panes[0]);
    let near_divider =
        move |pos: Pos2| split_view && (pos.x - (first.left() + first.width() * split)).abs() < 6.0;
    let mut on_divider = ui.data(|d| d.get_temp::<bool>(divider_id)).unwrap_or(false);
    if resp.drag_started() {
        // Where the button went down, not where the pointer is once the
        // drag counts as one.
        let origin = ui
            .input(|i| i.pointer.press_origin())
            .or_else(|| resp.interact_pointer_pos());
        on_divider = origin.is_some_and(near_divider);
        ui.data_mut(|d| d.insert_temp(divider_id, on_divider));
    }
    if resp.dragged() && on_divider {
        if let Some(p) = resp.interact_pointer_pos() {
            st.split = ((p.x - panes[0].left()) / panes[0].width()).clamp(0.0, 1.0);
        }
    } else if resp.dragged() && !picking {
        // Pan by dragging.
        let d = resp.drag_delta();
        if d != Vec2::ZERO {
            st.centre = Some((centre - d / (size * k)).clamp(Vec2::ZERO, Vec2::splat(1.0)));
            st.zoom = Some(k);
        }
    }
    if resp.hovered() && !resp.dragged() && resp.hover_pos().is_some_and(near_divider) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    // Where a point is in the picture: the pane under it, the point as
    // fractions of the turned picture.
    let locate = |pos: Pos2| -> Option<Vec2> {
        let pane = panes.iter().find(|p| p.contains(pos))?;
        let r = img_rect(*pane);
        Some((pos - r.min) / r.size())
    };
    // The wheel zooms about the pointer, keeping the point under it.
    if resp.hovered() {
        let (scroll, zoom_delta) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        let f = if zoom_delta != 1.0 {
            zoom_delta
        } else if scroll != 0.0 {
            (scroll / 120.0).exp2()
        } else {
            1.0
        };
        if f != 1.0 {
            let nk = (k * f).clamp(lo, ZOOM_MAX);
            if let Some(d) = resp.hover_pos().and_then(locate) {
                let c = d + (centre - d) * (k / nk);
                st.centre = Some(c.clamp(Vec2::ZERO, Vec2::splat(1.0)));
            }
            st.zoom = Some(nk);
        }
    }
    if picking && resp.hovered() {
        crate::ui::cursors::show(
            ui.ctx(),
            crate::ui::cursors::ToolCursor::Cross,
            resp.hover_pos(),
        );
    }
    if picking && resp.clicked() {
        if let Some(d) = resp.interact_pointer_pos().and_then(locate) {
            if (0.0..1.0).contains(&d.x) && (0.0..1.0).contains(&d.y) {
                out.picked = Some(unturn(d, st.turns));
            }
        }
    }
    out
}

/// The checkerboard's cell, points.
const CHECKER_CELL: f32 = 8.0;

/// A checkerboard showing transparency over `r`: one 2 x 2 texture made
/// once and repeated across the rectangle, so a frame draws a single
/// textured rectangle however large the view (the view is redrawn every
/// frame).
pub fn checkerboard(p: &Painter, r: Rect) {
    if r.width() <= 0.0 || r.height() <= 0.0 {
        return;
    }
    let tex = checker_texture(p.ctx());
    let uv = Rect::from_min_max(
        Pos2::ZERO,
        Pos2::new(
            r.width() / (2.0 * CHECKER_CELL),
            r.height() / (2.0 * CHECKER_CELL),
        ),
    );
    p.image(tex.id(), r, uv, Color32::WHITE);
}

/// The checkerboard's texture, loaded on first use and kept.
fn checker_texture(ctx: &Context) -> egui::TextureHandle {
    let id = egui::Id::new("preview_checkerboard");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let (l, d) = (255u8, 0xE6u8);
    let px = |v: u8| [v, v, v, 255];
    let rgba: Vec<u8> = [px(l), px(d), px(d), px(l)].concat();
    let img = egui::ColorImage::from_rgba_unmultiplied([2, 2], &rgba);
    let t = ctx.load_texture(
        "preview_checkerboard",
        img,
        egui::TextureOptions::NEAREST_REPEAT,
    );
    ctx.data_mut(|m| m.insert_temp(id, t.clone()));
    t
}

/// The dialog's body in two columns: the preview at the left taking the
/// room, the settings `right_w` wide at the right, both `body_h` tall
/// (the row of buttons under them, `bottom_h` tall, is the caller's).
/// `body` gets the two columns, left then right.
pub fn columns(ui: &mut Ui, right_w: f32, bottom_h: f32, body: impl FnOnce(&mut Ui, &mut Ui)) {
    let total = ui.available_rect_before_wrap();
    let body_h = (total.height() - bottom_h).max(200.0);
    let left_w = (total.width() - right_w - 12.0).max(300.0);
    let left_rect = Rect::from_min_size(total.min, Vec2::new(left_w, body_h));
    let right_rect = Rect::from_min_size(
        total.min + Vec2::new(left_w + 8.0, 0.0),
        Vec2::new(right_w, body_h),
    );
    let column = |r: Rect| {
        egui::UiBuilder::new()
            .max_rect(r)
            .layout(egui::Layout::top_down(egui::Align::Min))
    };
    let mut left = ui.new_child(column(left_rect));
    left.set_width(left_w);
    let mut right = ui.new_child(column(right_rect));
    right.set_width(right_w);
    body(&mut left, &mut right);
    ui.allocate_rect(
        Rect::from_min_size(total.min, Vec2::new(total.width(), body_h)),
        Sense::hover(),
    );
}

/// The room left for the panes under the toolbar, and over the status
/// line when the dialog has one.
pub fn pane_area(ui: &mut Ui, status: bool) -> Rect {
    let status_h = if status { STATUS_H + 4.0 } else { 0.0 };
    let area_h = (ui.available_height() - status_h).max(100.0);
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), area_h), Sense::hover());
    area
}

/// "Select a bitmap to see a preview" in `area`.
pub fn no_bitmap(ui: &Ui, area: Rect) {
    ui.painter()
        .rect_filled(area, 0.0, Color32::from_gray(0xC8));
    ui.painter().text(
        area.center(),
        egui::Align2::CENTER_CENTER,
        tr("dialog.no_bitmap_preview"),
        egui::FontId::proportional(13.0),
        Tokens::text(),
    );
}

/// The copy's side for panes of `view` in `area`: the pane's longer side
/// in pixels, rounded up to a multiple of 256 so a resize by a few
/// points makes no new copy, at most `cap`.
pub fn side_for(ctx: &Context, area: Rect, view: View, cap: u32) -> u32 {
    let pane = pane_rects(area, view)[0];
    let px = pane.width().max(pane.height()) * ctx.pixels_per_point();
    let side = ((px / 256.0).ceil() as u32 * 256).max(256);
    side.min(cap)
}

/// The largest copy the shared live previews work on.
pub const LIVE_CAP: u32 = 768;

/// What a live preview gives back: the copy the result was made from
/// (for histograms and eyedroppers), the result, and the copy's pixel
/// under a click on a picture while picking.
pub struct Live {
    pub img: Arc<RgbaImage>,
    pub after: Arc<RgbaImage>,
    pub picked: Option<(u32, u32)>,
}

/// The before and after preview of `src` through `f` (given the copy and
/// its scale against the bitmap), with the toolbar over the panes: `f`
/// runs when `key` (the settings) or the copy changes, never per frame.
/// The copy is as large as the panes show (a multiple of 256 pixels, at
/// most [`LIVE_CAP`]); while the pointer is held outside the panes (a
/// slider dragged) a copy at most [`THUMB`] across is worked on instead,
/// so the preview follows the slider at once, and the shown size
/// follows when the pointer is released. `views` is the Preview list;
/// `extra` adds the dialog's own buttons to the toolbar.
#[allow(clippy::too_many_arguments)]
pub fn live(
    ui: &mut Ui,
    app: &App,
    id: egui::Id,
    st: &mut ViewState,
    src: &Source,
    key: u64,
    views: &[View],
    picking: &mut bool,
    below: f32,
    extra: impl FnOnce(&mut Ui, &mut ViewState, Option<&Thumb>),
    f: impl FnOnce(&RgbaImage, f32) -> RgbaImage,
) -> Option<Live> {
    let ctx = ui.ctx().clone();
    // The panes' room, as it will be under the toolbar (and over the
    // dialog's own rows, `below` points of them).
    let gap = ui.spacing().item_spacing.y;
    let area_h = (ui.available_height() - BAR_H - gap - below).max(100.0);
    let area = Rect::from_min_size(
        ui.cursor().min + Vec2::new(0.0, BAR_H + gap),
        Vec2::new(ui.available_width(), area_h),
    );
    let full_side = side_for(&ctx, area, st.view, LIVE_CAP);
    let dragging = ctx.input(|i| i.pointer.any_down()) && !ctx.is_being_dragged(id.with("panes"));
    let full = copy_of(app, &ctx, src, full_side);
    let shown: Option<(Thumb, egui::TextureHandle, Arc<RgbaImage>)> = full.and_then(|copy| {
        let full_key = key ^ copy.shape.raw().rotate_left(17) ^ copy.key.rotate_left(41);
        let name = id.with("after");
        if let Some((tex, img)) = peek(&ctx, name, full_key) {
            return Some((copy, tex, img));
        }
        if dragging && full_side > THUMB {
            // Mid-drag: the small copy, made in a moment.
            let small = copy_of(app, &ctx, src, THUMB)?;
            let small_key = key ^ small.shape.raw().rotate_left(17) ^ small.key.rotate_left(41);
            let (tex, img) = cached(&ctx, id.with("after_quick"), small_key, || {
                f(&small.img, small.scale)
            });
            // The result is drawn at the panes' size; the copy returned
            // is the one it was made from.
            return Some((small, tex, img));
        }
        let (tex, img) = cached(&ctx, name, full_key, || f(&copy.img, copy.scale));
        Some((copy, tex, img))
    });
    let bar = toolbar(ui, id, st, views, false, *picking, |ui, st| {
        extra(ui, st, shown.as_ref().map(|(t, _, _)| t))
    });
    if bar.pan {
        *picking = false;
    }
    let (area, _) = ui.allocate_exact_size(area.size(), Sense::hover());
    let Some((copy, after_tex, after)) = shown else {
        no_bitmap(ui, area);
        return None;
    };
    let pics = Pictures {
        before: Some(copy.tex.id()),
        after: Some(after_tex.id()),
        overlay: None,
        size: Vec2::new(copy.img.width() as f32, copy.img.height() as f32),
    };
    let out = view(ui, id, st, area, &pics, *picking);
    let (w, h) = copy.img.dimensions();
    let picked = out.picked.filter(|_| w > 0 && h > 0).map(|u| {
        (
            ((u.x * w as f32) as u32).min(w - 1),
            ((u.y * h as f32) as u32).min(h - 1),
        )
    });
    Some(Live {
        img: copy.img,
        after,
        picked,
    })
}

#[derive(Clone)]
struct After {
    key: u64,
    img: Arc<RgbaImage>,
    tex: egui::TextureHandle,
}

fn color_image(img: &RgbaImage) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied(
        [img.width() as usize, img.height() as usize],
        img.as_raw(),
    )
}

/// The image and texture kept under `name` when they are for `key`.
fn peek(ctx: &Context, name: egui::Id, key: u64) -> Option<(egui::TextureHandle, Arc<RgbaImage>)> {
    ctx.data(|d| d.get_temp::<After>(name))
        .filter(|a| a.key == key)
        .map(|a| (a.tex, a.img))
}

/// An image and its texture kept under `name` until `key` changes;
/// `make` runs only then.
pub fn cached(
    ctx: &Context,
    name: egui::Id,
    key: u64,
    make: impl FnOnce() -> RgbaImage,
) -> (egui::TextureHandle, Arc<RgbaImage>) {
    if let Some(a) = peek(ctx, name, key) {
        return a;
    }
    let img = Arc::new(make());
    let tex = ctx.load_texture(
        format!("{name:?}"),
        color_image(&img),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| {
        d.insert_temp(
            name,
            After {
                key,
                img: img.clone(),
                tex: tex.clone(),
            },
        )
    });
    (tex, img)
}

/// The view state of the dialog `id`, kept between frames (and between
/// openings) for the dialogs whose state is their settings alone.
pub fn state(ctx: &Context, id: egui::Id) -> ViewState {
    ctx.data(|d| d.get_temp::<ViewState>(id.with("view_state")))
        .unwrap_or_default()
}

pub fn keep(ctx: &Context, id: egui::Id, st: ViewState) {
    ctx.data_mut(|d| d.insert_temp(id.with("view_state"), st));
}

/// The window every dialog with the shared preview opens in: sized by
/// the user, 960 x 640 points at first and never under 720 x 480.
pub fn window(ctx: &Context, title: String) -> crate::ui::chrome::Window {
    crate::ui::dialogs::window(ctx, title).resizable(DEFAULT_SIZE, MIN_SIZE)
}

pub const DEFAULT_SIZE: Vec2 = Vec2::new(960.0, 640.0);
pub const MIN_SIZE: Vec2 = Vec2::new(720.0, 480.0);
/// The settings column's width in those dialogs.
pub const RIGHT_W: f32 = 340.0;
/// The row of buttons' height.
pub const BOTTOM_H: f32 = 46.0;
/// The height of the Reset, undo and redo row under the settings panel.
pub const STEPS_H: f32 = 34.0;

/// The settings panel every dialog with the shared preview draws at the
/// right: a framed panel in the field colour (white in the light theme),
/// `below` points of room left under it for the dialog's own rows, its
/// contents scrolling when they do not fit. Sliders inside stretch to
/// its width with their label above and the value box at the right
/// (`crate::ui::Slider` in a column does that by itself).
pub fn panel(ui: &mut Ui, id: &str, below: f32, body: impl FnOnce(&mut Ui)) {
    let body_h = (ui.available_height() - below).max(100.0);
    let w = ui.available_width();
    // The frame's margin, both sides.
    let m = 20.0;
    egui::Frame::new()
        .fill(Tokens::field())
        .stroke(Stroke::new(1.0, Tokens::border()))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(w - m, body_h - m));
            ui.set_max_height(body_h - m);
            egui::ScrollArea::vertical()
                .id_salt(id)
                .max_height(body_h - m)
                .show(ui, |ui| {
                    // The width the scroll area leaves: less the scroll
                    // bar when the contents do not fit, so the value
                    // boxes at the right are never cut off (their spin
                    // arrows were, and lost their tips).
                    ui.set_width(ui.available_width().min(w - m));
                    body(ui);
                });
        });
    mark(
        ui,
        "settings_panel",
        Rect::from_min_size(
            ui.min_rect().left_bottom() - Vec2::new(0.0, body_h),
            Vec2::new(w, body_h),
        ),
    );
}

/// A thin line between groups of settings in the panel.
pub fn rule(ui: &mut Ui) {
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);
}

/// What the Reset, undo and redo row was asked for.
#[derive(Default, Clone, Copy)]
pub struct Steps {
    pub reset: bool,
    pub undo: bool,
    pub redo: bool,
}

/// Reset with the undo and redo arrows under the settings panel; `reset`
/// is the tip of Reset (its title and what it puts back).
pub fn steps_row(ui: &mut Ui, can_undo: bool, can_redo: bool, reset: &str) -> Steps {
    let mut out = Steps::default();
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let r = ui
            .add_sized([80.0, BAR_H], egui::Button::new(tr("dialog.reset")))
            .rich_tip(reset);
        mark(ui, "reset", r.rect);
        out.reset = r.clicked();
        let undo = icon_button(
            ui,
            &crate::ui::tips::pair("menu.edit.undo", "dialog.trace_undo_desc"),
            false,
            can_undo,
            |p, r, c| crate::ui::icons::draw_action(p, r, crate::ui::icons::Action::Undo, c),
        );
        mark(ui, "undo", undo.rect);
        out.undo = undo.clicked();
        let redo = icon_button(
            ui,
            &crate::ui::tips::pair("menu.edit.redo", "dialog.trace_redo_desc"),
            false,
            can_redo,
            |p, r, c| crate::ui::icons::draw_action(p, r, crate::ui::icons::Action::Redo, c),
        );
        mark(ui, "redo", redo.rect);
        out.redo = redo.clicked();
    });
    out
}

/// The settings states a dialog passed through, for its undo and redo
/// arrows: each change (a slider's drag once released, a click, a list,
/// Reset) is one step; a step after an undo drops the steps undone.
#[derive(Debug, Clone, PartialEq)]
pub struct History<T> {
    steps: Vec<T>,
    pos: usize,
}

impl<T: Clone + PartialEq> History<T> {
    /// A history starting at `first`.
    pub fn new(first: T) -> Self {
        History {
            steps: vec![first],
            pos: 0,
        }
    }

    /// Remember `now` as a new step when it differs from the step in use
    /// and `settled` (the pointer is up: a slider is not mid-drag).
    pub fn record(&mut self, now: &T, settled: bool) {
        if settled && self.steps.get(self.pos) != Some(now) {
            self.steps.truncate(self.pos + 1);
            self.steps.push(now.clone());
            self.pos = self.steps.len() - 1;
        }
    }

    pub fn can_undo(&self) -> bool {
        self.pos > 0
    }

    pub fn can_redo(&self) -> bool {
        self.pos + 1 < self.steps.len()
    }

    /// One step back; the settings to show.
    pub fn undo(&mut self) -> Option<T> {
        if !self.can_undo() {
            return None;
        }
        self.pos -= 1;
        self.steps.get(self.pos).cloned()
    }

    pub fn redo(&mut self) -> Option<T> {
        if !self.can_redo() {
            return None;
        }
        self.pos += 1;
        self.steps.get(self.pos).cloned()
    }
}

/// The history of the dialog `id` whose state is its settings alone,
/// kept between frames; a dialog that was not drawn last frame has just
/// opened and starts afresh at `first`.
pub fn history<T: Clone + PartialEq + Send + Sync + 'static>(
    ctx: &Context,
    id: egui::Id,
    first: T,
) -> History<T> {
    let frame = ctx.cumulative_frame_nr();
    ctx.data(|d| d.get_temp::<(u64, History<T>)>(id.with("history")))
        .filter(|(last, _)| frame <= last + 1)
        .map(|(_, h)| h)
        .unwrap_or_else(|| History::new(first))
}

pub fn keep_history<T: Clone + PartialEq + Send + Sync + 'static>(
    ctx: &Context,
    id: egui::Id,
    h: History<T>,
) {
    let frame = ctx.cumulative_frame_nr();
    ctx.data_mut(|d| d.insert_temp(id.with("history"), (frame, h)));
}

/// The settings `value` of a dialog whose state is its settings alone,
/// with the Reset, undo and redo row under the panel: the history is
/// kept between frames under `id`, a change (once the pointer is up) is
/// one step, Reset puts `default` (a step too), the arrows step through
/// the history.
pub fn steps_for<T: Clone + PartialEq + Send + Sync + 'static>(
    ui: &mut Ui,
    id: egui::Id,
    value: &mut T,
    default: T,
    reset: &str,
) -> Steps {
    let ctx = ui.ctx().clone();
    let settled = !ctx.input(|i| i.pointer.any_down());
    let mut h = history(&ctx, id, value.clone());
    h.record(value, settled);
    let steps = steps_row(ui, h.can_undo(), h.can_redo(), reset);
    if steps.reset {
        *value = default;
    }
    if steps.undo {
        if let Some(v) = h.undo() {
            *value = v;
        }
    }
    if steps.redo {
        if let Some(v) = h.redo() {
            *value = v;
        }
    }
    keep_history(&ctx, id, h);
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turned_points_map_back() {
        for turns in 0..4u8 {
            let d = Vec2::new(0.25, 0.75);
            // Turn forward, as the view draws it, then back.
            let fwd = match turns {
                1 => Vec2::new(1.0 - d.y, d.x),
                2 => Vec2::new(1.0 - d.x, 1.0 - d.y),
                3 => Vec2::new(d.y, 1.0 - d.x),
                _ => d,
            };
            let back = unturn(fwd, turns);
            assert!((back - d).length() < 1e-5, "{turns}: {back:?}");
        }
        assert_eq!(turned_size(Vec2::new(40.0, 30.0), 1), Vec2::new(30.0, 40.0));
        assert_eq!(turned_size(Vec2::new(40.0, 30.0), 2), Vec2::new(40.0, 30.0));
    }

    /// The copy's side follows the pane shown in steps of 256, capped.
    #[test]
    fn the_copy_side_follows_the_pane() {
        let ctx = Context::default();
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(700.0, 600.0));
        assert_eq!(side_for(&ctx, area, View::Stacked, LIVE_CAP), 768);
        assert_eq!(side_for(&ctx, area, View::SideBySide, LIVE_CAP), 768);
        assert_eq!(side_for(&ctx, area, View::After, LIVE_CAP), 768);
        let small = Rect::from_min_size(Pos2::ZERO, Vec2::new(300.0, 500.0));
        assert_eq!(side_for(&ctx, small, View::Stacked, LIVE_CAP), 512);
        assert_eq!(side_for(&ctx, small, View::SideBySide, LIVE_CAP), 512);
        let tiny = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 100.0));
        assert_eq!(side_for(&ctx, tiny, View::After, LIVE_CAP), 256);
        let huge = Rect::from_min_size(Pos2::ZERO, Vec2::new(3000.0, 3000.0));
        assert_eq!(side_for(&ctx, huge, View::After, 1024), 1024);
    }

    /// Two panes share the area, stacked or side by side; the fit zoom
    /// is the one that shows the whole picture in a pane.
    #[test]
    fn panes_share_the_area_and_fit_the_picture() {
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(604.0, 404.0));
        let s = pane_rects(area, View::Stacked);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].height(), 200.0);
        assert_eq!(s[1].top(), 204.0);
        let h = pane_rects(area, View::SideBySide);
        assert_eq!(h[0].width(), 300.0);
        assert_eq!(h[1].left(), 304.0);
        assert_eq!(pane_rects(area, View::Split).len(), 1);
        let size = Vec2::new(400.0, 100.0);
        let fit = fit_zoom(area, View::Stacked, size, 0);
        assert!((fit - 602.0 / 400.0).abs() < 1e-4, "{fit}");
        let fit = fit_zoom(area, View::Stacked, size, 1);
        assert!((fit - 198.0 / 400.0).abs() < 1e-4, "{fit}");
    }

    /// A cached result is made once per key and found again without
    /// being made; another key makes it again.
    #[test]
    fn results_are_made_once_per_key() {
        let ctx = Context::default();
        let name = egui::Id::new("test_after");
        let made = std::cell::Cell::new(0);
        let make = || {
            made.set(made.get() + 1);
            RgbaImage::new(4, 4)
        };
        let _ = cached(&ctx, name, 1, make);
        let _ = cached(&ctx, name, 1, make);
        assert_eq!(made.get(), 1);
        assert!(peek(&ctx, name, 1).is_some());
        assert!(peek(&ctx, name, 2).is_none());
        let _ = cached(&ctx, name, 2, make);
        assert_eq!(made.get(), 2);
    }
}
