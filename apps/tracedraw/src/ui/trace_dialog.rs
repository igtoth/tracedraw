//! Bitmaps > Trace Bitmap: a window the user sizes, with the picture and
//! its trace large at the left (Before and After, Before, After or the
//! wireframe over the picture; zoom and pan), the counts of the trace
//! under them, and the Settings and Colors tabs at the right. The trace
//! follows every change by itself (see `trace_preview.rs`): no Preview
//! button. See docs/behavior/bitmap-tracing.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::trace::{ColorMode, Preset, Remove, Settings};
use crate::trace_preview::{Request, Stage};
use crate::ui::bitmap_preview::{copy_of, selected_bitmap, settings_key, Source};
use crate::ui::tips::RichTip;
use egui::{Color32, Context, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// The longest side of the copy the preview shows, pixels.
pub const PREVIEW_SIDE: u32 = 1024;

/// The window's first size and its smallest.
const DEFAULT_SIZE: Vec2 = Vec2::new(1040.0, 700.0);
const MIN_SIZE: Vec2 = Vec2::new(780.0, 540.0);
/// The settings column's width.
const RIGHT_W: f32 = 320.0;

/// What the preview shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TracePreview {
    /// The picture above, its trace below.
    #[default]
    BeforeAfter,
    Before,
    After,
    /// The trace's curves as lines over the picture.
    Wireframe,
}

impl TracePreview {
    pub const ALL: [TracePreview; 4] = [
        TracePreview::BeforeAfter,
        TracePreview::Before,
        TracePreview::After,
        TracePreview::Wireframe,
    ];
    pub fn key(self) -> &'static str {
        match self {
            TracePreview::BeforeAfter => "dialog.trace_view_before_after",
            TracePreview::Before => "dialog.before",
            TracePreview::After => "dialog.after",
            TracePreview::Wireframe => "dialog.trace_view_wireframe",
        }
    }
}

/// The dialog's state.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceState {
    pub preset: Preset,
    pub settings: Settings,
    pub preview: TracePreview,
    /// 0 Settings, 1 Colors.
    pub tab: usize,
    /// Screen points per preview pixel; `None` fits the picture.
    pub zoom: Option<f32>,
    /// The point of the picture (as fractions of its size) at the middle
    /// of the view; `None` its middle.
    pub centre: Option<Vec2>,
    /// A click on the preview picks the background colour.
    pub picking: bool,
    /// Every settings state passed through, for the undo and redo arrows;
    /// `cursor` is the one in use.
    pub history: Vec<Settings>,
    pub cursor: usize,
    /// The colours ticked in the Colors tab.
    pub selected_colors: Vec<[u8; 3]>,
}

impl TraceState {
    /// The Trace dialog on `preset`, with the choices every trace starts
    /// with (Options > Global > Bitmap Tracing).
    pub fn new(preset: Preset, prefs: &crate::prefs::TracePrefs) -> Self {
        Self::with_settings(preset, prefs.settings(preset))
    }

    /// The dialog on `preset` showing `settings` (the last trace's).
    pub fn with_settings(preset: Preset, settings: Settings) -> Self {
        TraceState {
            preset,
            history: vec![settings.clone()],
            settings,
            preview: TracePreview::BeforeAfter,
            tab: 0,
            zoom: None,
            centre: None,
            picking: false,
            cursor: 0,
            selected_colors: Vec::new(),
        }
    }

    /// Remember the settings as they stand when they changed since the
    /// last step; a step after an undo drops the steps undone.
    pub fn commit(&mut self) {
        if self.history.get(self.cursor) == Some(&self.settings) {
            return;
        }
        self.history.truncate(self.cursor + 1);
        self.history.push(self.settings.clone());
        self.cursor = self.history.len() - 1;
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }
    pub fn can_redo(&self) -> bool {
        self.cursor + 1 < self.history.len()
    }
    pub fn undo(&mut self) {
        if self.can_undo() {
            self.cursor -= 1;
            self.settings = self.history[self.cursor].clone();
        }
    }
    pub fn redo(&mut self) {
        if self.can_redo() {
            self.cursor += 1;
            self.settings = self.history[self.cursor].clone();
        }
    }

    /// Back to the preset's settings (with the choices every trace starts
    /// with), as a step the arrows can undo.
    pub fn reset(&mut self, prefs: &crate::prefs::TracePrefs) {
        self.settings = prefs.settings(self.preset);
        self.selected_colors.clear();
        self.picking = false;
    }

    /// Switch to `preset`: its settings, the trace type following.
    pub fn choose_preset(&mut self, preset: Preset, prefs: &crate::prefs::TracePrefs) {
        self.preset = preset;
        self.settings = prefs.settings(preset);
        self.selected_colors.clear();
    }

    /// The key of the trace the settings ask for on the copy `copy_key`
    /// of `shape`: only the settings that change the traced curves
    /// count, so ticking Group by color or Delete original image, or
    /// editing a colour, traces nothing again.
    fn request_key(&self, shape: tracedraw_core::ShapeId, copy_key: u64) -> u64 {
        settings_key(&self.settings.traced_part())
            ^ copy_key.rotate_left(13)
            ^ shape.raw().rotate_left(29)
    }
}

/// Where the dialog's controls are this frame, for the tests that drive
/// it with the pointer.
#[cfg(test)]
#[derive(Clone, Default)]
pub struct Rects(pub std::collections::HashMap<&'static str, Rect>);

#[cfg(test)]
pub fn rects(ctx: &Context) -> Rects {
    ctx.data(|d| d.get_temp::<Rects>(egui::Id::new("trace_dialog_rects")))
        .unwrap_or_default()
}

fn mark(ui: &Ui, name: &'static str, rect: Rect) {
    #[cfg(test)]
    ui.ctx().data_mut(|d| {
        let r: &mut Rects = d.get_temp_mut_or_default(egui::Id::new("trace_dialog_rects"));
        r.0.insert(name, rect);
    });
    #[cfg(not(test))]
    let _ = (ui, name, rect);
}

fn icon_button(
    ui: &mut Ui,
    tip: &str,
    pressed: bool,
    enabled: bool,
    draw: impl FnOnce(&Painter, Rect, Color32),
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
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

/// The view's picture: which texture, drawn where the picture is.
struct Pane<'a> {
    tex: Option<egui::TextureId>,
    /// Drawn over `tex`: the wireframe.
    over: Option<egui::TextureId>,
    label: &'a str,
}

/// What a frame of the preview did.
#[derive(Default)]
struct ViewOut {
    /// A click (while picking) on the picture, as a fraction of its size.
    picked: Option<Vec2>,
}

/// Draw `panes` stacked in `area` with the one zoom and centre; drags pan,
/// the wheel zooms, a click while `picking` picks a point.
fn view(
    ui: &mut Ui,
    st: &mut TraceState,
    area: Rect,
    size: Vec2,
    panes: &[Pane],
    picking: bool,
) -> ViewOut {
    let mut out = ViewOut::default();
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, Color32::from_gray(0xC8));
    let n = panes.len().max(1) as f32;
    let gap = 4.0;
    let pane_h = (area.height() - gap * (n - 1.0)) / n;
    let fit = ((area.width() - 2.0) / size.x.max(1.0)).min((pane_h - 2.0) / size.y.max(1.0));
    let k = st.zoom.unwrap_or(fit).clamp(fit.min(1.0) * 0.1, 64.0);
    let centre = st.centre.unwrap_or(Vec2::splat(0.5));
    let resp = ui.interact(area, ui.id().with("trace_view"), Sense::click_and_drag());
    let mut first_rect = None;
    for (i, pane) in panes.iter().enumerate() {
        let top = area.top() + i as f32 * (pane_h + gap);
        let pane_rect = Rect::from_min_size(
            area.left_top() + Vec2::new(0.0, top - area.top()),
            Vec2::new(area.width(), pane_h),
        );
        let p = ui.painter_at(pane_rect);
        p.rect_filled(pane_rect, 0.0, Color32::from_gray(0xC8));
        let r = Rect::from_min_size(pane_rect.center() - centre * size * k, size * k);
        if first_rect.is_none() {
            first_rect = Some((pane_rect, r));
        }
        // A checkerboard shows transparency.
        checkerboard(&p, r.intersect(pane_rect));
        if let Some(tex) = pane.tex {
            p.image(
                tex,
                r,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        if let Some(over) = pane.over {
            p.image(
                over,
                r,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        if !pane.label.is_empty() {
            let galley = p.layout_no_wrap(
                pane.label.to_string(),
                egui::FontId::proportional(11.0),
                Tokens::text(),
            );
            let tag = Rect::from_min_size(
                pane_rect.left_top() + Vec2::new(4.0, 4.0),
                galley.size() + Vec2::new(8.0, 4.0),
            );
            p.rect_filled(tag, 2.0, Color32::from_white_alpha(200));
            p.galley(tag.min + Vec2::new(4.0, 2.0), galley, Tokens::text());
        }
        p.rect_stroke(
            pane_rect,
            0.0,
            Stroke::new(1.0, Tokens::border()),
            egui::StrokeKind::Inside,
        );
    }
    if let Some((pane_rect, r)) = first_rect {
        mark(ui, "view_pane", pane_rect);
        mark(ui, "view_image", r);
    }
    // Pan by dragging, zoom with the wheel about the pointer.
    if resp.dragged() && !picking {
        let d = resp.drag_delta();
        if d != Vec2::ZERO {
            st.centre = Some((centre - d / (size * k)).clamp(Vec2::ZERO, Vec2::splat(1.0)));
            st.zoom = Some(k);
        }
    }
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
            st.zoom = Some((k * f).clamp(fit.min(1.0) * 0.1, 64.0));
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
        if let Some(pos) = resp.interact_pointer_pos() {
            // Whichever pane was clicked: the same point of the picture.
            for i in 0..panes.len() {
                let top = area.top() + i as f32 * (pane_h + gap);
                let pane_rect = Rect::from_min_size(
                    Pos2::new(area.left(), top),
                    Vec2::new(area.width(), pane_h),
                );
                let r = Rect::from_min_size(pane_rect.center() - centre * size * k, size * k);
                if pane_rect.contains(pos) && r.contains(pos) {
                    out.picked = Some((pos - r.min) / r.size());
                }
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
fn checkerboard(p: &Painter, r: Rect) {
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
    let id = egui::Id::new("trace_checkerboard");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let (l, d) = (255u8, 0xE6u8);
    let px = |v: u8| [v, v, v, 255];
    let rgba: Vec<u8> = [px(l), px(d), px(d), px(l)].concat();
    let img = egui::ColorImage::from_rgba_unmultiplied([2, 2], &rgba);
    let t = ctx.load_texture(
        "trace_checkerboard",
        img,
        egui::TextureOptions::NEAREST_REPEAT,
    );
    ctx.data_mut(|m| m.insert_temp(id, t.clone()));
    t
}

/// A slider with the label above it, a colon after the label, tick marks
/// under the rail, a slim upright thumb and the value box at the right.
fn slider(ui: &mut Ui, name: &'static str, key: &str, v: &mut f64) {
    ui.label(format!("{}:", tr(key)));
    crate::ui::tips::tip_over(ui, crate::ui::tips::keyed(key), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = (ui.available_width() - 70.0).max(100.0);
            let resp = ui.add(crate::ui::Rail(
                egui::Slider::new(v, 0.0..=100.0)
                    .show_value(true)
                    .step_by(1.0)
                    .max_decimals(0)
                    .handle_shape(egui::style::HandleShape::Rect { aspect_ratio: 0.4 }),
            ));
            let rail = Rect::from_min_size(
                resp.rect.left_top(),
                Vec2::new(ui.spacing().slider_width, resp.rect.height()),
            );
            mark(ui, name, rail);
            let p = ui.painter();
            let y = rail.bottom() - 1.0;
            let inset = 6.0;
            for i in 0..=10 {
                let x = rail.left() + inset + (rail.width() - 2.0 * inset) * i as f32 / 10.0;
                let h = if i % 5 == 0 { 4.0 } else { 2.5 };
                p.line_segment(
                    [Pos2::new(x, y), Pos2::new(x, y + h)],
                    Stroke::new(1.0, Tokens::text_dim()),
                );
            }
        });
    });
    ui.add_space(4.0);
}

fn combo<T: PartialEq + Copy>(
    ui: &mut Ui,
    id: &'static str,
    width: f32,
    value: &mut T,
    all: &[T],
    name: impl Fn(T) -> String,
) -> bool {
    let mut changed = false;
    let resp = egui::ComboBox::from_id_salt(id)
        .selected_text(name(*value))
        .width(width)
        .show_ui(ui, |ui| {
            for v in all {
                if ui.selectable_value(value, *v, name(*v)).clicked() {
                    changed = true;
                }
            }
        })
        .response;
    mark(ui, id, resp.rect);
    changed
}

/// The Trace Bitmap dialog.
pub fn trace_dialog(app: &mut App, ctx: &Context, st: &mut TraceState, close: &mut bool) {
    let before = st.settings.clone();
    // The undo and redo arrows move the cursor (nothing else does in a
    // frame): their settings are a step already taken, kept as they are.
    let cursor = st.cursor;
    let mut ok = false;
    let mut help = false;
    let mut options = false;
    let dragging = ctx.input(|i| i.pointer.any_down());
    super::dialogs::window(ctx, tr("dialog.trace"))
        .resizable(DEFAULT_SIZE, MIN_SIZE)
        .show(ctx, |ui| {
            let total = ui.available_size();
            let bottom_h = 46.0;
            let body_h = (total.y - bottom_h).max(200.0);
            let left_w = (total.x - RIGHT_W - 12.0).max(300.0);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(left_w, body_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(left_w);
                        ui.set_height(body_h);
                        preview_column(app, ui, st, dragging);
                    },
                );
                ui.add_space(8.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(RIGHT_W, body_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(RIGHT_W);
                        settings_column(app, ui, st);
                    },
                );
            });
            ui.horizontal(|ui| {
                help = ui
                    .add_sized([27.0, 27.0], egui::Button::new("?"))
                    .rich_tip(crate::ui::tips::keyed("dialog.help_button"))
                    .clicked();
                let r = ui
                    .add_sized([100.0, 27.0], egui::Button::new(tr("dialog.trace_options")))
                    .rich_tip(crate::ui::tips::keyed("dialog.trace_options"));
                mark(ui, "options", r.rect);
                options = r.clicked();
                ok = super::dialogs::ok_cancel(ui, close);
            });
        });
    // Settings the user changes (not a slider mid-drag) are steps the
    // arrows undo; a change to what is traced forgets the colours merged
    // or edited on the Colors tab, which belonged to the trace before.
    let retraced = |a: &Settings, b: &Settings| {
        (
            a.detail,
            a.smoothing,
            a.corner_smoothness,
            a.colors,
            a.mode,
            a.centerline,
        ) != (
            b.detail,
            b.smoothing,
            b.corner_smoothness,
            b.colors,
            b.mode,
            b.centerline,
        )
    };
    let arrowed = st.cursor != cursor;
    if !arrowed && retraced(&before, &st.settings) {
        st.settings.merged.clear();
        st.settings.recolored.clear();
        st.selected_colors.clear();
    }
    // A step of the arrows is in the history already: committing nothing
    // keeps the steps to redo.
    if !dragging && !arrowed {
        st.commit();
    }
    if help {
        app.open_dialog_help();
    }
    if options {
        // Options > Global > Bitmap Tracing, then back here.
        app.options_page = crate::ui::options::OptionsPage::BitmapTracing;
        app.dialog_return = Some(Box::new(super::dialogs::Dialog::Trace(st.clone())));
        app.dialog = super::dialogs::Dialog::Options;
        *close = true;
        return;
    }
    if ok {
        let settings = st.settings.clone();
        // The whole picture's trace from the preview, when it is in.
        let ready = selected_bitmap(app).and_then(|(shape, _, _)| {
            let copy = copy_of(app, ctx, &Source::Shown, PREVIEW_SIDE)?;
            let key = st.request_key(shape, copy.key);
            app.trace_live
                .full(key, crate::trace_preview::look_key(&st.settings))
                .filter(|s| s.shape == shape)
                .map(|s| (s.shape, s.traced.clone(), s.stroke_px))
        });
        app.trace_bitmaps_using(&settings, ready);
        app.last_trace = Some((st.preset, settings));
    }
}

/// The preview's row of controls, the view and the counts.
fn preview_column(app: &mut App, ui: &mut Ui, st: &mut TraceState, dragging: bool) {
    let ctx = ui.ctx().clone();
    ui.horizontal(|ui| {
        ui.label(tr("dialog.trace_preview_label"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::keyed("dialog.trace_preview_label"),
            |ui| {
                combo(
                    ui,
                    "trace_view",
                    170.0,
                    &mut st.preview,
                    &TracePreview::ALL,
                    |v| tr(v.key()),
                );
            },
        );
        ui.add_space(8.0);
        if icon_button(
            ui,
            &crate::ui::tips::keyed("dialog.lab_zoom_fit"),
            st.zoom.is_none(),
            true,
            |p, r, c| {
                p.rect_stroke(
                    r.shrink(3.0),
                    0.0,
                    Stroke::new(1.2, c),
                    egui::StrokeKind::Middle,
                );
            },
        )
        .clicked()
        {
            st.zoom = None;
            st.centre = None;
        }
        for (zoom_in, key) in [(false, "dialog.lab_zoom_out"), (true, "dialog.lab_zoom_in")] {
            let r = icon_button(ui, &crate::ui::tips::keyed(key), false, true, |p, r, c| {
                crate::ui::icons::draw(p, r, crate::tools::Tool::Zoom, c);
                let m = r.min + Vec2::new(6.8, 6.8) * (r.width() / 16.0);
                let s = Stroke::new(1.2, c);
                p.line_segment([m - Vec2::new(2.4, 0.0), m + Vec2::new(2.4, 0.0)], s);
                if zoom_in {
                    p.line_segment([m - Vec2::new(0.0, 2.4), m + Vec2::new(0.0, 2.4)], s);
                }
            });
            mark(ui, if zoom_in { "zoom_in" } else { "zoom_out" }, r.rect);
            if r.clicked() {
                let k = st
                    .zoom
                    .or_else(|| ctx.data(|d| d.get_temp::<f32>(egui::Id::new("trace_view_fit"))))
                    .unwrap_or(1.0);
                st.zoom = Some((if zoom_in { k * 1.25 } else { k / 1.25 }).clamp(0.02, 64.0));
            }
        }
        icon_button(
            ui,
            &crate::ui::tips::keyed("dialog.lab_pan"),
            !st.picking,
            true,
            |p, r, c| crate::ui::icons::draw(p, r, crate::tools::Tool::Pan, c),
        );
        if app.trace_live.busy() {
            ui.add(egui::Spinner::new().size(16.0));
        }
    });
    let status_h = 22.0;
    let area_h = (ui.available_height() - status_h - 4.0).max(100.0);
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), area_h), Sense::hover());
    let Some((shape, _dpi, png)) = selected_bitmap(app) else {
        ui.painter()
            .rect_filled(area, 0.0, Color32::from_gray(0xC8));
        ui.painter().text(
            area.center(),
            egui::Align2::CENTER_CENTER,
            tr("dialog.no_bitmap_preview"),
            egui::FontId::proportional(13.0),
            Tokens::text(),
        );
        return;
    };
    let Some(copy) = copy_of(app, &ctx, &Source::Shown, PREVIEW_SIDE) else {
        return;
    };
    // The trace of these settings, asked for once; results come in as
    // they are ready.
    let key = st.request_key(shape, copy.key);
    if let Some(image) = app.trace_live.image(shape, &png) {
        app.trace_live.want(
            &ctx,
            Request {
                shape,
                key,
                look: crate::trace_preview::look_key(&st.settings),
                settings: st.settings.clone(),
                image,
                copy_scale: copy.scale,
            },
            dragging,
        );
    }
    app.trace_live.poll(&ctx);
    let shown = app
        .trace_live
        .shown
        .as_ref()
        .filter(|s| s.shape == shape)
        .cloned();
    let size = Vec2::new(copy.img.width() as f32, copy.img.height() as f32);
    let after_tex = shown.as_ref().map(|s| s.after.id());
    let wire_tex = shown.as_ref().map(|s| s.wire.id());
    let (before_l, after_l) = (tr("dialog.before"), tr("dialog.after"));
    let wire_l = tr("dialog.trace_view_wireframe");
    let panes: Vec<Pane> = match st.preview {
        TracePreview::BeforeAfter => vec![
            Pane {
                tex: Some(copy.tex.id()),
                over: None,
                label: &before_l,
            },
            Pane {
                tex: after_tex,
                over: None,
                label: &after_l,
            },
        ],
        TracePreview::Before => vec![Pane {
            tex: Some(copy.tex.id()),
            over: None,
            label: "",
        }],
        TracePreview::After => vec![Pane {
            tex: after_tex,
            over: None,
            label: "",
        }],
        TracePreview::Wireframe => vec![Pane {
            tex: Some(copy.tex.id()),
            over: wire_tex,
            label: &wire_l,
        }],
    };
    let n = panes.len() as f32;
    let fit = ((area.width() - 2.0) / size.x.max(1.0))
        .min(((area.height() - 4.0 * (n - 1.0)) / n - 2.0) / size.y.max(1.0));
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("trace_view_fit"), fit));
    let out = view(ui, st, area, size, &panes, st.picking);
    if let Some(at) = out.picked {
        let (w, h) = copy.img.dimensions();
        let x = ((at.x * w as f32) as u32).min(w.saturating_sub(1));
        let y = ((at.y * h as f32) as u32).min(h.saturating_sub(1));
        let c = copy.img.get_pixel(x, y).0;
        st.settings.background = Some([c[0], c[1], c[2]]);
        if st.settings.remove == Remove::None {
            st.settings.remove = Remove::Background;
        }
        st.picking = false;
    }
    // The counts, of the whole picture's trace; the quick look's while it
    // is being made.
    ui.horizontal(|ui| match &shown {
        Some(s) => {
            let text = trf(
                "dialog.trace_stats",
                &[
                    ("curves", &s.stats.curves.to_string()),
                    ("nodes", &s.stats.nodes.to_string()),
                    ("colors", &s.stats.colors.to_string()),
                ],
            );
            let text = if s.stage == Stage::Quick {
                format!("{text} \u{2026}")
            } else {
                text
            };
            ui.label(text);
        }
        None => {
            ui.label(tr("dialog.trace_tracing"));
        }
    });
}

/// The Settings and Colors tabs, and Reset with the undo and redo arrows.
fn settings_column(app: &mut App, ui: &mut Ui, st: &mut TraceState) {
    let labels = [
        tr("dialog.trace_tab_settings"),
        tr("dialog.trace_tab_colors"),
    ];
    crate::ui::color_management::tab_header(ui, &mut st.tab, &labels);
    let tabs = ui.min_rect();
    mark(
        ui,
        "tab_settings",
        Rect::from_min_size(
            tabs.left_bottom() - Vec2::new(0.0, 26.0),
            Vec2::new(80.0, 26.0),
        ),
    );
    mark(
        ui,
        "tab_colors",
        Rect::from_min_size(
            tabs.left_bottom() - Vec2::new(-80.0, 26.0),
            Vec2::new(80.0, 26.0),
        ),
    );
    let body_h = (ui.available_height() - 40.0).max(100.0);
    egui::Frame::new()
        .fill(Tokens::field())
        .stroke(Stroke::new(1.0, Tokens::border()))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(RIGHT_W - 22.0, body_h));
            ui.set_max_height(body_h);
            egui::ScrollArea::vertical()
                .id_salt("trace_settings_scroll")
                .max_height(body_h - 20.0)
                .show(ui, |ui| {
                    ui.set_width(RIGHT_W - 42.0);
                    if st.tab == 0 {
                        settings_tab(app, ui, st);
                    } else {
                        colors_tab(app, ui, st);
                    }
                });
        });
    ui.horizontal(|ui| {
        let r = ui
            .add_sized([80.0, 26.0], egui::Button::new(tr("dialog.reset")))
            .rich_tip(crate::ui::tips::pair(
                "dialog.reset",
                "dialog.trace_reset_desc",
            ));
        mark(ui, "reset", r.rect);
        if r.clicked() {
            st.reset(&app.settings.tracing);
        }
        let undo = icon_button(
            ui,
            &crate::ui::tips::pair("menu.edit.undo", "dialog.trace_undo_desc"),
            false,
            st.can_undo(),
            |p, r, c| crate::ui::icons::draw_action(p, r, crate::ui::icons::Action::Undo, c),
        );
        mark(ui, "undo", undo.rect);
        if undo.clicked() {
            st.undo();
        }
        let redo = icon_button(
            ui,
            &crate::ui::tips::pair("menu.edit.redo", "dialog.trace_redo_desc"),
            false,
            st.can_redo(),
            |p, r, c| crate::ui::icons::draw_action(p, r, crate::ui::icons::Action::Redo, c),
        );
        mark(ui, "redo", redo.rect);
        if redo.clicked() {
            st.redo();
        }
    });
}

fn preset_name(p: Preset) -> String {
    tr(p.key())
        .trim_end_matches("...")
        .trim_end_matches('\u{2026}')
        .to_string()
}

fn settings_tab(app: &mut App, ui: &mut Ui, st: &mut TraceState) {
    let prefs = app.settings.tracing.clone();
    let w = 180.0;
    egui::Grid::new("trace_type_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label(tr("dialog.trace_type"));
            let mut centerline = st.settings.centerline;
            crate::ui::tips::tip_over(ui, crate::ui::tips::keyed("dialog.trace_type"), |ui| {
                if combo(ui, "trace_type", w, &mut centerline, &[false, true], |c| {
                    tr(if c {
                        "dialog.trace_type_centerline"
                    } else {
                        "dialog.trace_type_outline"
                    })
                }) && centerline != st.settings.centerline
                {
                    // The first image type of the other kind.
                    if let Some(p) = Preset::of_type(centerline).first() {
                        st.choose_preset(*p, &prefs);
                    }
                }
            });
            ui.end_row();
            ui.label(tr("dialog.trace_image_type"));
            let mut preset = st.preset;
            crate::ui::tips::tip_over(
                ui,
                crate::ui::tips::keyed("dialog.trace_image_type"),
                |ui| {
                    if combo(
                        ui,
                        "image_type",
                        w,
                        &mut preset,
                        Preset::of_type(st.settings.centerline),
                        preset_name,
                    ) && preset != st.preset
                    {
                        st.choose_preset(preset, &prefs);
                    }
                },
            );
            ui.end_row();
        });
    ui.add_space(6.0);
    slider(ui, "detail", "dialog.detail", &mut st.settings.detail);
    slider(
        ui,
        "smoothing",
        "dialog.smoothing",
        &mut st.settings.smoothing,
    );
    slider(
        ui,
        "corner",
        "dialog.corner_smoothness",
        &mut st.settings.corner_smoothness,
    );
    if !st.settings.centerline {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(tr("dialog.trace_remove"));
            crate::ui::tips::tip_over(ui, crate::ui::tips::keyed("dialog.trace_remove"), |ui| {
                combo(
                    ui,
                    "remove",
                    w,
                    &mut st.settings.remove,
                    &Remove::ALL,
                    |r| tr(r.key()),
                );
            });
        });
        if st.settings.remove != Remove::None {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                let auto = st.settings.background.is_none();
                let r = ui
                    .radio(auto, tr("dialog.trace_remove_auto"))
                    .rich_tip(crate::ui::tips::keyed("dialog.trace_remove_auto"));
                mark(ui, "remove_auto", r.rect);
                if r.clicked() {
                    st.settings.background = None;
                    st.picking = false;
                }
                let r = ui
                    .radio(!auto, tr("dialog.trace_remove_specify"))
                    .rich_tip(crate::ui::tips::keyed("dialog.trace_remove_specify"));
                mark(ui, "remove_specify", r.rect);
                if r.clicked() && auto {
                    st.settings.background = Some([255, 255, 255]);
                }
            });
            if let Some(mut bg) = st.settings.background {
                ui.horizontal(|ui| {
                    ui.add_space(30.0);
                    let r = ui
                        .color_edit_button_srgb(&mut bg)
                        .rich_tip(crate::ui::tips::keyed("dialog.trace_background_swatch"));
                    if r.changed() {
                        st.settings.background = Some(bg);
                    }
                    mark(ui, "background_swatch", r.rect);
                    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                    crate::ui::propbar::frame(ui, rect, resp.hovered(), st.picking);
                    crate::ui::icons::draw(
                        ui.painter(),
                        Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
                        crate::tools::Tool::ColorEyedropper,
                        Tokens::icon(),
                    );
                    mark(ui, "eyedropper", rect);
                    if resp
                        .rich_tip(crate::ui::tips::keyed("dialog.trace_pick_background"))
                        .clicked()
                    {
                        st.picking = !st.picking;
                    }
                    ui.label(
                        egui::RichText::new(format!("#{:02X}{:02X}{:02X}", bg[0], bg[1], bg[2]))
                            .color(Tokens::text_dim())
                            .size(11.0),
                    );
                });
            }
        }
        ui.separator();
        ui.label(tr("dialog.trace_objects"));
        let r = ui
            .checkbox(
                &mut st.settings.merge_adjacent,
                tr("dialog.trace_merge_adjacent"),
            )
            .rich_tip(crate::ui::tips::keyed("dialog.trace_merge_adjacent"));
        mark(ui, "merge_adjacent", r.rect);
        let r = ui
            .checkbox(
                &mut st.settings.remove_overlap,
                tr("dialog.trace_remove_overlap"),
            )
            .rich_tip(crate::ui::tips::keyed("dialog.trace_remove_overlap"));
        mark(ui, "remove_overlap", r.rect);
        let r = ui
            .checkbox(
                &mut st.settings.group_by_color,
                tr("dialog.trace_group_by_color"),
            )
            .rich_tip(crate::ui::tips::keyed("dialog.trace_group_by_color"));
        mark(ui, "group_by_color", r.rect);
    }
    ui.separator();
    ui.label(tr("dialog.trace_image"));
    let r = ui
        .checkbox(
            &mut st.settings.delete_original,
            tr("dialog.delete_original"),
        )
        .rich_tip(crate::ui::tips::keyed("dialog.delete_original"));
    mark(ui, "delete_original", r.rect);
}

fn colors_tab(app: &mut App, ui: &mut Ui, st: &mut TraceState) {
    let w = 160.0;
    if st.settings.centerline {
        ui.label(tr("dialog.trace_colors_centerline"));
        return;
    }
    egui::Grid::new("trace_colors_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label(tr("dialog.color_mode"));
            crate::ui::tips::tip_over(
                ui,
                crate::ui::tips::keyed("dialog.trace_color_mode"),
                |ui| {
                    combo(
                        ui,
                        "color_mode",
                        w,
                        &mut st.settings.mode,
                        &ColorMode::ALL,
                        |m| tr(m.key()),
                    );
                },
            );
            ui.end_row();
            if st.settings.mode != ColorMode::BlackWhite {
                ui.label(tr("dialog.trace_number_of_colors"));
                crate::ui::tips::tip_over(
                    ui,
                    crate::ui::tips::pair(
                        "dialog.trace_number_of_colors",
                        "dialog.trace_colors_desc",
                    ),
                    |ui| {
                        let r = ui.add(
                            crate::ui::field::NumField::new(&mut st.settings.colors).range(2..=256),
                        );
                        mark(ui, "number_of_colors", r.rect);
                    },
                );
                ui.end_row();
            }
        });
    ui.separator();
    let shown = app.trace_live.shown.as_ref();
    let Some(shown) = shown else {
        ui.label(tr("dialog.trace_tracing"));
        return;
    };
    // Worked out once with the trace, not every frame.
    let counts = shown.colors.clone();
    ui.label(trf(
        "dialog.trace_colors_in_trace",
        &[("n", &counts.len().to_string())],
    ));
    // The colours and their curves; a tick picks, the swatch edits.
    let mut edit: Option<([u8; 3], [u8; 3])> = None;
    egui::ScrollArea::vertical()
        .id_salt("trace_colors_list")
        .max_height(220.0)
        .show(ui, |ui| {
            for (i, (c, n)) in counts.iter().enumerate() {
                ui.horizontal(|ui| {
                    let mut on = st.selected_colors.contains(c);
                    let r = ui.checkbox(&mut on, "");
                    if i == 0 {
                        mark(ui, "color_0", r.rect);
                    } else if i == 1 {
                        mark(ui, "color_1", r.rect);
                    }
                    if r.changed() {
                        if on {
                            st.selected_colors.push(*c);
                        } else {
                            st.selected_colors.retain(|k| k != c);
                        }
                    }
                    let mut rgb = *c;
                    if ui
                        .color_edit_button_srgb(&mut rgb)
                        .rich_tip(crate::ui::tips::keyed("dialog.trace_edit_color"))
                        .changed()
                        && rgb != *c
                    {
                        edit = Some((*c, rgb));
                    }
                    ui.label(format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]));
                    ui.label(
                        egui::RichText::new(trf("dialog.trace_curves_n", &[("n", &n.to_string())]))
                            .color(Tokens::text_dim()),
                    );
                });
            }
        });
    if let Some((from, to)) = edit {
        // The list shows the colours as drawn: an edited colour's key is
        // the colour it was traced as.
        let key = st
            .settings
            .recolored
            .iter()
            .find(|(_, shown)| *shown == from)
            .map(|(k, _)| *k)
            .unwrap_or(from);
        st.settings.recolored.retain(|(k, _)| *k != key);
        if to != key {
            st.settings.recolored.push((key, to));
        }
    }
    ui.add_space(4.0);
    let can_merge = st.selected_colors.len() >= 2;
    let r = ui
        .add_enabled(
            can_merge,
            egui::Button::new(tr("dialog.trace_merge_colors")),
        )
        .rich_tip(crate::ui::tips::keyed("dialog.trace_merge_colors"));
    mark(ui, "merge_colors", r.rect);
    if r.clicked() && can_merge {
        // In the list's order: the first stays, the others join it.
        let group: Vec<[u8; 3]> = counts
            .iter()
            .map(|(c, _)| *c)
            .filter(|c| st.selected_colors.contains(c))
            .map(|c| {
                // Colours shown as another colour merge as what they
                // were traced as.
                st.settings
                    .recolored
                    .iter()
                    .find(|(_, s)| *s == c)
                    .map(|(k, _)| *k)
                    .unwrap_or(c)
            })
            .collect();
        st.settings.merged.push(group);
        st.selected_colors.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only the settings that change the traced curves change the
    /// request's key: grouping, deleting the original, a colour edit and
    /// the background of a trace that removes none trace nothing again.
    #[test]
    fn only_settings_that_change_the_trace_change_its_key() {
        let st = TraceState::with_settings(Preset::Logo, Preset::Logo.settings());
        let id = tracedraw_core::ShapeId(3);
        let key = st.request_key(id, 11);
        let mut same = st.clone();
        same.settings.delete_original = !same.settings.delete_original;
        same.settings.group_by_color = !same.settings.group_by_color;
        same.settings.recolored = vec![([0, 0, 0], [10, 20, 30])];
        same.settings.remove = Remove::None;
        let key_none = same.request_key(id, 11);
        same.settings.background = Some([1, 2, 3]);
        assert_eq!(same.request_key(id, 11), key_none);
        same.settings.remove = st.settings.remove;
        same.settings.background = st.settings.background;
        assert_eq!(same.request_key(id, 11), key);
        for change in [
            |s: &mut Settings| s.detail += 5.0,
            |s: &mut Settings| s.colors += 1,
            |s: &mut Settings| s.merge_adjacent = !s.merge_adjacent,
            |s: &mut Settings| s.merged.push(vec![[0, 0, 0], [9, 9, 9]]),
        ] {
            let mut other = st.clone();
            change(&mut other.settings);
            assert_ne!(other.request_key(id, 11), key);
        }
        assert_ne!(st.request_key(id, 12), key, "another copy");
        assert_ne!(st.request_key(tracedraw_core::ShapeId(4), 11), key);
    }
}
