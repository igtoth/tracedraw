//! Bitmaps > Straighten Image: the preview with a grid and the cropping
//! area, zoom and pan, quarter turns, and the lens, rotation and
//! perspective sliders, Crop image and Crop and resample to original
//! size.

use crate::app::App;
use crate::i18n::tr;
use crate::straighten::{straighten, turned, Straighten};
use crate::theme::Tokens;
use crate::ui::bitmap_preview::{cached, copy_of, settings_key, Source};
use egui::{Color32, Context, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// The preview copy's largest side, pixels.
const SIDE: u32 = 560;
const VIEW: Vec2 = Vec2::new(520.0, 400.0);

/// What a drag or click in the preview does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewTool {
    #[default]
    Pan,
    ZoomIn,
    ZoomOut,
}

/// The dialog's state: the settings and how the preview shows them.
#[derive(Debug, Clone, PartialEq)]
pub struct StraightenState {
    pub s: Straighten,
    pub grid: bool,
    /// Grid cells, screen points.
    pub grid_size: f32,
    pub grid_color: [u8; 3],
    pub tool: ViewTool,
    /// Screen points per preview pixel; `None` fits the image.
    pub zoom: Option<f32>,
    /// The preview point at the window's middle; `None` its middle.
    pub centre: Option<Vec2>,
}

impl Default for StraightenState {
    fn default() -> Self {
        StraightenState {
            s: Straighten::default(),
            grid: true,
            grid_size: 24.0,
            grid_color: [0x80, 0x80, 0x80],
            tool: ViewTool::Pan,
            zoom: None,
            centre: None,
        }
    }
}

fn icon_button(
    ui: &mut Ui,
    tip: &str,
    pressed: bool,
    draw: impl FnOnce(&Painter, Rect, Color32),
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
    crate::ui::propbar::frame(ui, rect, resp.hovered(), pressed);
    draw(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
        Tokens::ICON,
    );
    let clicked = resp.clicked();
    resp.on_hover_text(tip);
    clicked
}

/// The preview: the corrected copy (uncropped), the grid over it and the
/// cropping area, with zoom and pan.
fn preview(
    ui: &mut Ui,
    st: &mut StraightenState,
    tex: egui::TextureId,
    size: Vec2,
    crop: Option<Vec2>,
) {
    let (area, resp) = ui.allocate_exact_size(VIEW, Sense::click_and_drag());
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, Color32::from_gray(0xC8));
    let fit = (area.width() / size.x.max(1.0)).min(area.height() / size.y.max(1.0));
    let k = st.zoom.unwrap_or(fit).clamp(fit.min(1.0) * 0.25, 32.0);
    let centre = st.centre.unwrap_or(size / 2.0);
    let r = Rect::from_min_size(area.center() - centre * k, size * k);
    painter.image(
        tex,
        r,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    // The cropping area: everything outside it dimmed.
    if let Some(c) = crop {
        let cr = Rect::from_center_size(r.center(), c * k);
        let dim = Color32::from_black_alpha(110);
        painter.rect_filled(
            Rect::from_min_max(area.min, Pos2::new(area.right(), cr.top())),
            0.0,
            dim,
        );
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(area.left(), cr.bottom()), area.max),
            0.0,
            dim,
        );
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(area.left(), cr.top()),
                Pos2::new(cr.left(), cr.bottom()),
            ),
            0.0,
            dim,
        );
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(cr.right(), cr.top()),
                Pos2::new(area.right(), cr.bottom()),
            ),
            0.0,
            dim,
        );
        painter.rect_stroke(
            cr,
            0.0,
            Stroke::new(1.0, Color32::WHITE),
            egui::StrokeKind::Middle,
        );
    }
    // The grid, fixed to the window, to line the image up against.
    if st.grid {
        let [cr, cg, cb] = st.grid_color;
        let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(cr, cg, cb, 200));
        let step = st.grid_size.clamp(4.0, 200.0);
        let mut x = area.center().x - (area.width() / 2.0 / step).floor() * step;
        while x <= area.right() {
            painter.vline(x, area.y_range(), stroke);
            x += step;
        }
        let mut y = area.center().y - (area.height() / 2.0 / step).floor() * step;
        while y <= area.bottom() {
            painter.hline(area.x_range(), y, stroke);
            y += step;
        }
    }
    painter.rect_stroke(
        area,
        0.0,
        Stroke::new(1.0, Tokens::BORDER),
        egui::StrokeKind::Inside,
    );
    match st.tool {
        ViewTool::Pan => {
            if resp.dragged() {
                st.centre = Some((centre - resp.drag_delta() / k).clamp(Vec2::ZERO, size));
                st.zoom = Some(k);
            }
        }
        ViewTool::ZoomIn | ViewTool::ZoomOut => {
            if resp.clicked() {
                if let Some(p) = resp.interact_pointer_pos() {
                    let d = (p - r.min) / k;
                    let f = if st.tool == ViewTool::ZoomIn {
                        1.5
                    } else {
                        1.0 / 1.5
                    };
                    st.zoom = Some((k * f).clamp(fit.min(1.0) * 0.25, 32.0));
                    st.centre = Some(d.clamp(Vec2::ZERO, size));
                }
            }
        }
    }
}

/// The Straighten Image dialog; OK straightens the selected bitmaps.
pub fn straighten_dialog(app: &mut App, ctx: &Context, st: &mut StraightenState, close: &mut bool) {
    super::dialogs::window(ctx, tr("dialog.straighten_image")).show(ctx, |ui| {
        let Some(copy) = copy_of(app, ctx, &Source::Shown, SIDE) else {
            ui.label(tr("dialog.no_bitmap_preview"));
            super::dialogs::ok_cancel(ui, close);
            return;
        };
        // The preview shows the whole corrected copy; the crop is drawn.
        let view = Straighten {
            crop: false,
            resample: false,
            ..st.s
        };
        let key = settings_key(&format!("{view:?}")) ^ copy.key.rotate_left(7);
        let (tex, img) = cached(ctx, "straighten_preview", key, || {
            straighten(&copy.img, &view)
        });
        let size = Vec2::new(img.width() as f32, img.height() as f32);
        let crop = st.s.crop.then(|| {
            let t = turned(&copy.img, st.s.turns);
            let (w, h) = (t.width() as f32, t.height() as f32);
            let k = st.s.clamped().crop_scale(w, h);
            Vec2::new(w * k, h * k)
        });
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    for (tool, key) in [
                        (ViewTool::ZoomIn, "dialog.lab_zoom_in"),
                        (ViewTool::ZoomOut, "dialog.lab_zoom_out"),
                    ] {
                        let on = st.tool == tool;
                        if icon_button(ui, &tr(key), on, |p, r, c| {
                            crate::ui::icons::draw(p, r, crate::tools::Tool::Zoom, c);
                            let m = r.min + Vec2::new(6.8, 6.8) * (r.width() / 16.0);
                            let s = Stroke::new(1.2, c);
                            p.line_segment([m - Vec2::new(2.4, 0.0), m + Vec2::new(2.4, 0.0)], s);
                            if tool == ViewTool::ZoomIn {
                                p.line_segment(
                                    [m - Vec2::new(0.0, 2.4), m + Vec2::new(0.0, 2.4)],
                                    s,
                                );
                            }
                        }) {
                            st.tool = tool;
                        }
                    }
                    if icon_button(
                        ui,
                        &tr("dialog.lab_zoom_fit"),
                        st.zoom.is_none(),
                        |p, r, c| {
                            p.rect_stroke(
                                r.shrink(3.0),
                                0.0,
                                Stroke::new(1.2, c),
                                egui::StrokeKind::Middle,
                            );
                        },
                    ) {
                        st.zoom = None;
                        st.centre = None;
                    }
                    let actual = 1.0 / copy.scale.max(1e-6);
                    if icon_button(
                        ui,
                        &tr("dialog.lab_actual_size"),
                        st.zoom == Some(actual),
                        |p, r, c| {
                            p.text(
                                r.center(),
                                egui::Align2::CENTER_CENTER,
                                "1:1",
                                egui::FontId::proportional(11.0),
                                c,
                            );
                        },
                    ) {
                        st.zoom = Some(actual);
                    }
                    if icon_button(
                        ui,
                        &tr("dialog.lab_pan"),
                        st.tool == ViewTool::Pan,
                        |p, r, c| crate::ui::icons::draw(p, r, crate::tools::Tool::Pan, c),
                    ) {
                        st.tool = ViewTool::Pan;
                    }
                    ui.add_space(8.0);
                    if icon_button(ui, &tr("dialog.rotate_ccw"), false, |p, r, c| {
                        crate::ui::lab_dialog::draw_rotate(p, r, c, true)
                    }) {
                        st.s.turns = (st.s.turns + 3) % 4;
                        st.centre = None;
                    }
                    if icon_button(ui, &tr("dialog.rotate_cw"), false, |p, r, c| {
                        crate::ui::lab_dialog::draw_rotate(p, r, c, false)
                    }) {
                        st.s.turns = (st.s.turns + 1) % 4;
                        st.centre = None;
                    }
                });
                preview(ui, st, tex.id(), size, crop);
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.set_width(250.0);
                ui.spacing_mut().slider_width = 150.0;
                let slider = |ui: &mut Ui,
                              key: &str,
                              v: &mut f32,
                              range: std::ops::RangeInclusive<f32>,
                              step: f64| {
                    ui.label(tr(key));
                    ui.add(egui::Slider::new(v, range).step_by(step).max_decimals(1));
                };
                slider(
                    ui,
                    "dialog.straighten_lens",
                    &mut st.s.lens,
                    -100.0..=100.0,
                    1.0,
                );
                ui.add_space(4.0);
                slider(
                    ui,
                    "dialog.straighten_rotate",
                    &mut st.s.angle,
                    -15.0..=15.0,
                    0.1,
                );
                ui.add_space(4.0);
                slider(
                    ui,
                    "dialog.straighten_vertical",
                    &mut st.s.vertical,
                    -100.0..=100.0,
                    1.0,
                );
                ui.add_space(4.0);
                slider(
                    ui,
                    "dialog.straighten_horizontal",
                    &mut st.s.horizontal,
                    -100.0..=100.0,
                    1.0,
                );
                ui.separator();
                ui.horizontal(|ui| {
                    ui.checkbox(&mut st.grid, tr("dialog.straighten_grid"));
                    ui.color_edit_button_srgb(&mut st.grid_color)
                        .on_hover_text(tr("dialog.straighten_grid_color"));
                });
                ui.add_enabled(
                    st.grid,
                    egui::Slider::new(&mut st.grid_size, 8.0..=80.0).integer(),
                );
                ui.separator();
                ui.checkbox(&mut st.s.crop, tr("dialog.straighten_crop"));
                ui.add_enabled_ui(st.s.crop, |ui| {
                    ui.checkbox(&mut st.s.resample, tr("dialog.straighten_resample"));
                });
                ui.add_space(6.0);
                if ui.button(tr("dialog.reset")).clicked() {
                    st.s = Straighten::default();
                    st.zoom = None;
                    st.centre = None;
                }
            });
        });
        if super::dialogs::ok_cancel(ui, close) {
            let s = st.s;
            app.straighten_image(&s);
        }
    });
}
