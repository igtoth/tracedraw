//! Bitmaps > Trace Bitmap: a window the user sizes, with the picture and
//! its trace large at the left (the shared Before and After preview of
//! `before_after.rs`, with the wireframe over the picture as a view of
//! its own; zoom and pan), the counts of the trace under them, and the
//! Settings and Colors tabs at the right. The trace follows every change
//! by itself (see `trace_preview.rs`): no Preview button. See
//! docs/behavior/bitmap-tracing.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::trace::{ColorMode, Preset, Remove, Settings};
use crate::trace_preview::Request;
use crate::ui::before_after::{self, mark, Pictures, View, ViewState};
use crate::ui::bitmap_preview::{copy_of, selected_bitmap, settings_key, Source};
use crate::ui::tips::RichTip;
use egui::{Context, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// The longest side of the copy the preview shows, pixels.
pub const PREVIEW_SIDE: u32 = 1024;

/// The window's first size and its smallest.
const DEFAULT_SIZE: Vec2 = Vec2::new(1040.0, 700.0);
const MIN_SIZE: Vec2 = Vec2::new(780.0, 540.0);
/// The settings column's width.
const RIGHT_W: f32 = 320.0;

/// The dialog's state.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceState {
    pub preset: Preset,
    pub settings: Settings,
    /// How the preview is looked at (the view, zoom and place).
    pub view: ViewState,
    /// 0 Settings, 1 Colors.
    pub tab: usize,
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
            view: ViewState::default(),
            tab: 0,
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

/// The views the Trace dialog offers: Before and After (stacked, and side
/// by side), Before, Large Preview (the trace alone, taking the whole
/// preview) and Wireframe Overlay (the curves over the picture, which
/// shows through at the Overlay slider's strength).
pub const VIEWS: [View; 5] = [
    View::Stacked,
    View::SideBySide,
    View::Before,
    View::Large,
    View::Overlay,
];

#[cfg(test)]
pub use crate::ui::before_after::rects;

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
            before_after::columns(ui, RIGHT_W, before_after::BOTTOM_H, |left, right| {
                preview_column(app, left, st, dragging);
                settings_column(app, right, st);
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
    let id = egui::Id::new("trace_preview");
    let bar = before_after::toolbar(
        ui,
        id,
        &mut st.view,
        &VIEWS,
        app.trace_live.busy(),
        st.picking,
        |_, _| {},
    );
    if bar.pan {
        st.picking = false;
    }
    let area = before_after::pane_area(ui, true);
    let Some((shape, _dpi, png)) = selected_bitmap(app) else {
        before_after::no_bitmap(ui, area);
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
    let pics = Pictures {
        before: Some(copy.tex.id()),
        after: shown.as_ref().map(|s| s.after.id()),
        overlay: shown.as_ref().map(|s| s.wire.id()),
        size: Vec2::new(copy.img.width() as f32, copy.img.height() as f32),
    };
    let out = before_after::view(ui, id, &mut st.view, area, &pics, st.picking);
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
            let text = if s.done {
                text
            } else {
                format!("{text} \u{2026}")
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
    before_after::panel(ui, "trace_settings_scroll", before_after::STEPS_H, |ui| {
        if st.tab == 0 {
            settings_tab(app, ui, st);
        } else {
            colors_tab(app, ui, st);
        }
    });
    // The history is the dialog's own (a step keeps the Colors tab's
    // merges and edits, see `trace_dialog`): the shared row only draws.
    let steps = before_after::steps_row(
        ui,
        st.can_undo(),
        st.can_redo(),
        &crate::ui::tips::pair("dialog.reset", "dialog.trace_reset_desc"),
    );
    if steps.reset {
        st.reset(&app.settings.tracing);
    }
    if steps.undo {
        st.undo();
    }
    if steps.redo {
        st.redo();
    }
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
    before_after::rule(ui);
    slider(ui, "detail", "dialog.detail", &mut st.settings.detail);
    slider(
        ui,
        "smoothing",
        "dialog.smoothing",
        &mut st.settings.smoothing,
    );
    // A centreline trace reads the detail and the smoothing only: the
    // other controls are shown greyed, as they do not apply.
    let outline = !st.settings.centerline;
    ui.add_enabled_ui(outline, |ui| {
        slider(
            ui,
            "corner",
            "dialog.corner_smoothness",
            &mut st.settings.corner_smoothness,
        );
    });
    before_after::rule(ui);
    ui.add_enabled_ui(outline, |ui| {
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
    });
    before_after::rule(ui);
    ui.label(tr("dialog.trace_objects"));
    ui.add_enabled_ui(outline, |ui| {
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
    });
    // With every region of a colour one curve, a group per colour would
    // hold one curve: the box is greyed and the grouping left out.
    ui.add_enabled_ui(outline && !st.settings.merge_adjacent, |ui| {
        let r = ui
            .checkbox(
                &mut st.settings.group_by_color,
                tr("dialog.trace_group_by_color"),
            )
            .rich_tip(crate::ui::tips::keyed("dialog.trace_group_by_color"));
        mark(ui, "group_by_color", r.rect);
    });
    before_after::rule(ui);
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
