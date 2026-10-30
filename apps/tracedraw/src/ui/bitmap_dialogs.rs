//! Bitmaps > Mode dialogs: Black and White (1-bit), Duotone (8-bit) and
//! Paletted (8-bit), and Bitmaps > Remove Background, each a window the
//! user sizes with the shared Before and After preview of the selected
//! bitmap (`before_after.rs`) at the left and its settings at the right.
//! OK converts every selected bitmap in one undo step.

use crate::app::App;
use crate::bitmap_modes::{
    black_and_white, build_palette, duotone, paletted, BwMethod, BwSettings, Dither,
    DuotoneSettings, DuotoneType, PaletteType, PalettedSettings, Screen,
};
use crate::i18n::tr;
use crate::ui::before_after::{self, View, BOTTOM_H, RIGHT_W};
use crate::ui::bitmap_preview::{settings_key, Source};
use crate::ui::tips::RichTip;
use egui::{Color32, Context, Ui};
use image::RgbaImage;

/// The shared preview of the selected bitmap through `f`, in the dialog
/// `id` whose view state is kept between frames; the copy and the pixel
/// picked (while `picking`) come back.
fn preview(
    ui: &mut Ui,
    app: &App,
    id: &'static str,
    key: u64,
    picking: &mut bool,
    f: impl FnOnce(&RgbaImage, f32) -> RgbaImage,
) -> Option<before_after::Live> {
    let id = egui::Id::new(id);
    let ctx = ui.ctx().clone();
    let mut view = before_after::state(&ctx, id);
    let out = before_after::live(
        ui,
        app,
        id,
        &mut view,
        &Source::Shown,
        key,
        &View::PLAIN,
        picking,
        0.0,
        |_, _, _| {},
        f,
    );
    before_after::keep(&ctx, id, view);
    out
}

/// The Duotone dialog's state: the settings, the ink whose curve is shown
/// and whether every ink's curve is drawn.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DuotoneState {
    pub settings: DuotoneSettings,
    pub ink: usize,
    pub show_all: bool,
}

fn rgb32(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn combo<T: Copy + PartialEq>(
    ui: &mut Ui,
    id: &str,
    value: &mut T,
    all: &[T],
    key: impl Fn(T) -> &'static str,
    tip: &str,
) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .selected_text(tr(key(*value)))
        .width(180.0)
        .show_ui(ui, |ui| {
            for v in all {
                if ui.selectable_label(*value == *v, tr(key(*v))).clicked() {
                    *value = *v;
                    changed = true;
                }
            }
        })
        .response
        .rich_tip(crate::ui::tips::keyed(tip));
    changed
}

/// Black and White (1-bit): conversion method, threshold or intensity,
/// and the halftone screen.
pub fn bw_dialog(app: &mut App, ctx: &Context, s: &mut BwSettings, close: &mut bool) {
    before_after::window(ctx, tr("dialog.mode_bw")).show(ctx, |ui| {
        let dpi = crate::ui::bitmap_preview::selected_bitmap(app)
            .map(|b| b.1)
            .unwrap_or(300.0);
        let settings = *s;
        before_after::columns(ui, RIGHT_W, BOTTOM_H, |left, right| {
            preview(
                left,
                app,
                "bw_preview",
                settings_key(&settings),
                &mut false,
                |img, scale| black_and_white(img, &settings, dpi * scale),
            );
            before_after::panel(right, "bw_settings", before_after::STEPS_H, |ui| {
                bw_settings(ui, s)
            });
            before_after::steps_for(
                right,
                egui::Id::new("bw_steps"),
                s,
                BwSettings::default(),
                &crate::ui::tips::keyed("dialog.reset"),
            );
        });
        if super::dialogs::ok_cancel(ui, close) {
            let settings = *s;
            app.apply_to_bitmaps("Black and White", move |img, dpi| {
                black_and_white(img, &settings, dpi)
            });
        }
    });
}

/// A slider of the settings panel: its label above it, stretched to the
/// panel's width, the value box at the right; `name` is its place for
/// the tests.
fn panel_slider<N: egui::emath::Numeric>(
    ui: &mut Ui,
    name: &'static str,
    key: &str,
    slider: crate::ui::Slider<'_, N>,
) -> egui::Response {
    let r = crate::ui::tips::tip_over(ui, crate::ui::tips::keyed(key), |ui| {
        ui.add(slider.label(tr(key)))
    });
    before_after::mark(ui, name, r.rect);
    ui.add_space(2.0);
    r
}

/// The Black and White settings: the method and what it needs.
fn bw_settings(ui: &mut Ui, s: &mut BwSettings) {
    ui.horizontal(|ui| {
        ui.label(tr("dialog.conversion_method"));
        combo(
            ui,
            "bw_method",
            &mut s.method,
            &BwMethod::ALL,
            BwMethod::key,
            "dialog.conversion_method",
        );
    });
    before_after::rule(ui);
    match s.method {
        BwMethod::LineArt => {
            panel_slider(
                ui,
                "bw_slider",
                "dialog.threshold",
                crate::ui::Slider::new(&mut s.threshold, 0..=255),
            );
        }
        BwMethod::Halftone => {
            ui.horizontal(|ui| {
                ui.label(tr("dialog.screen_type"));
                egui::ComboBox::from_id_salt("bw_screen")
                    .selected_text(tr(screen_key(s.screen)))
                    .show_ui(ui, |ui| {
                        for sc in [Screen::Round, Screen::Line, Screen::Square] {
                            ui.selectable_value(&mut s.screen, sc, tr(screen_key(sc)));
                        }
                    })
                    .response
                    .rich_tip(crate::ui::tips::keyed("dialog.screen_type"));
            });
            ui.add_space(4.0);
            panel_slider(
                ui,
                "bw_slider",
                "dialog.angle",
                crate::ui::Slider::new(&mut s.angle, -180.0..=180.0).suffix(" \u{b0}"),
            );
            panel_slider(
                ui,
                "bw_lpi",
                "dialog.lines_per_inch",
                crate::ui::Slider::new(&mut s.lpi, 1.0..=300.0),
            );
        }
        _ => {
            panel_slider(
                ui,
                "bw_slider",
                "dialog.intensity",
                crate::ui::Slider::new(&mut s.intensity, 0.0..=100.0),
            );
        }
    }
}

fn screen_key(s: Screen) -> &'static str {
    match s {
        Screen::Round => "bmode.screen_round",
        Screen::Line => "bmode.screen_line",
        Screen::Square => "bmode.screen_square",
    }
}

/// Duotone (8-bit): type, inks with their colours and tone curves, Load
/// and Save of the ink settings.
pub fn duotone_dialog(app: &mut App, ctx: &Context, st: &mut DuotoneState, close: &mut bool) {
    if let Some(path) = crate::files::take_picked("duotone_inks") {
        if let Some(s) = crate::files::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<DuotoneSettings>(&t).ok())
        {
            st.settings = s;
            st.ink = 0;
        }
    }
    before_after::window(ctx, tr("dialog.mode_duotone")).show(ctx, |ui| {
        // Four inks always exist; the type uses the first ones.
        while st.settings.inks.len() < 4 {
            let d = DuotoneSettings::default();
            let i = st.settings.inks.len();
            st.settings.inks.push(d.inks[i].clone());
        }
        let settings = st.settings.clone();
        before_after::columns(ui, RIGHT_W, BOTTOM_H, |left, right| {
            preview(
                left,
                app,
                "duotone_preview",
                settings_key(&settings),
                &mut false,
                |img, _| duotone(img, &settings),
            );
            before_after::panel(right, "duotone_settings", before_after::STEPS_H, |ui| {
                duotone_settings(ui, app, st)
            });
            before_after::steps_for(
                right,
                egui::Id::new("duotone_steps"),
                &mut st.settings,
                DuotoneSettings::default(),
                &crate::ui::tips::keyed("dialog.reset"),
            );
        });
        if super::dialogs::ok_cancel(ui, close) {
            let settings = st.settings.clone();
            app.apply_to_bitmaps("Duotone", move |img, _| duotone(img, &settings));
        }
    });
}

/// The Duotone settings: the type, the inks and their curves, Load and
/// Save.
fn duotone_settings(ui: &mut Ui, app: &mut App, st: &mut DuotoneState) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(tr("dialog.type"));
            combo(
                ui,
                "duo_type",
                &mut st.settings.kind,
                &DuotoneType::ALL,
                DuotoneType::key,
                "dialog.duotone_type",
            );
        });
        let n = st.settings.kind.inks();
        st.ink = st.ink.min(n - 1);
        for i in 0..n {
            ui.horizontal(|ui| {
                let mut rgb = st.settings.inks[i].rgb;
                if ui
                    .color_edit_button_srgb(&mut rgb)
                    .rich_tip(crate::ui::tips::keyed("dialog.ink_color"))
                    .changed()
                {
                    st.settings.inks[i].rgb = rgb;
                }
                let name = crate::i18n::trf("dialog.ink_n", &[("n", &(i + 1).to_string())]);
                if ui
                    .selectable_label(st.ink == i, name)
                    .rich_tip_with(|| {
                        let name = crate::i18n::trf("dialog.ink_n", &[("n", &(i + 1).to_string())]);
                        format!("{name}\n{}", tr("dialog.ink_pick_desc"))
                    })
                    .clicked()
                {
                    st.ink = i;
                }
            });
        }
        ui.checkbox(&mut st.show_all, tr("dialog.show_all"))
            .rich_tip(crate::ui::tips::pair(
                "dialog.show_all",
                "dialog.duotone_show_all_desc",
            ));
        before_after::rule(ui);
        ui.horizontal(|ui| {
            if ui
                .button(tr("dialog.load"))
                .rich_tip(crate::ui::tips::pair(
                    "dialog.load",
                    "dialog.duotone_load_desc",
                ))
                .clicked()
            {
                crate::files::Dialog::new()
                    .add_filter(tr("dialog.duotone_inks"), &["json"])
                    .pick_into("duotone_inks");
            }
            if ui
                .button(tr("dialog.save"))
                .rich_tip(crate::ui::tips::pair(
                    "dialog.save",
                    "dialog.duotone_save_desc",
                ))
                .clicked()
            {
                if let Some(path) = crate::files::Dialog::new()
                    .add_filter(tr("dialog.duotone_inks"), &["json"])
                    .set_file_name("inks.json")
                    .save_path()
                {
                    if let Ok(json) = serde_json::to_string_pretty(&st.settings) {
                        if let Err(e) = crate::files::write(&path, json) {
                            app.status = e.to_string();
                        }
                    }
                }
            }
        });
    });
    before_after::rule(ui);
    ui.vertical(|ui| {
        let inks = st.settings.inks.clone();
        let n = st.settings.kind.inks();
        let others: Vec<(&crate::bitmap_modes::ToneCurve, Color32)> = if st.show_all {
            inks.iter()
                .take(n)
                .enumerate()
                .filter(|(i, _)| *i != st.ink)
                .map(|(_, ink)| (&ink.curve, rgb32(ink.rgb).gamma_multiply(0.6)))
                .collect()
        } else {
            Vec::new()
        };
        let ink = &mut st.settings.inks[st.ink];
        let col = if ink.rgb == [255, 255, 255] {
            Color32::GRAY
        } else {
            rgb32(ink.rgb)
        };
        crate::ui::curve_edit::curve_editor(
            ui,
            egui::Id::new("duotone_curve"),
            &mut ink.curve,
            200.0,
            col,
            &others,
            None,
        );
        ui.label(
            egui::RichText::new(tr("dialog.duotone_curve_hint"))
                .size(11.0)
                .color(crate::theme::Tokens::text_dim()),
        );
    });
}

/// Paletted (8-bit): palette type and colour count, dithering and its
/// intensity, and the processed palette.
pub fn paletted_dialog(app: &mut App, ctx: &Context, s: &mut PalettedSettings, close: &mut bool) {
    if s.palette == PaletteType::Custom && s.custom.is_empty() {
        s.custom = app.document_palette_rgb();
    }
    before_after::window(ctx, tr("dialog.mode_paletted")).show(ctx, |ui| {
        let settings = s.clone();
        before_after::columns(ui, RIGHT_W, BOTTOM_H, |left, right| {
            let thumb = preview(
                left,
                app,
                "paletted_preview",
                settings_key(&settings),
                &mut false,
                |img, _| paletted(img, &settings),
            )
            .map(|p| p.img);
            before_after::panel(right, "paletted_settings", before_after::STEPS_H, |ui| {
                paletted_settings(ui, s, thumb.as_deref())
            });
            before_after::steps_for(
                right,
                egui::Id::new("paletted_steps"),
                s,
                PalettedSettings::default(),
                &crate::ui::tips::keyed("dialog.reset"),
            );
        });
        if super::dialogs::ok_cancel(ui, close) {
            let settings = s.clone();
            app.apply_to_bitmaps("Paletted", move |img, _| paletted(img, &settings));
        }
    });
}

/// The Paletted settings and the processed palette of the copy `thumb`.
fn paletted_settings(ui: &mut Ui, s: &mut PalettedSettings, thumb: Option<&RgbaImage>) {
    {
        ui.horizontal(|ui| {
            ui.label(tr("dialog.palette"));
            combo(
                ui,
                "pal_type",
                &mut s.palette,
                &PaletteType::ALL,
                PaletteType::key,
                "dialog.palette",
            );
        });
        if s.palette.counts() {
            ui.add_space(4.0);
            panel_slider(
                ui,
                "pal_colors",
                "dialog.colors",
                crate::ui::Slider::new(&mut s.colors, 2..=256),
            );
        }
        before_after::rule(ui);
        ui.horizontal(|ui| {
            ui.label(tr("dialog.dithering"));
            combo(
                ui,
                "pal_dither",
                &mut s.dither,
                &Dither::ALL,
                Dither::key,
                "dialog.dithering",
            );
        });
        if s.dither != Dither::None {
            ui.add_space(4.0);
            panel_slider(
                ui,
                "pal_intensity",
                "dialog.dither_intensity",
                crate::ui::Slider::new(&mut s.intensity, 0.0..=100.0),
            );
        }
        before_after::rule(ui);
        // The processed palette, worked out again when the settings change.
        if let Some(t) = thumb {
            let key = settings_key(&*s);
            let id = egui::Id::new("paletted_processed");
            let pal = match ui.ctx().data(|d| d.get_temp::<(u64, Vec<[u8; 3]>)>(id)) {
                Some((k, p)) if k == key => p,
                _ => {
                    let p = build_palette(t, s);
                    ui.ctx().data_mut(|d| d.insert_temp(id, (key, p.clone())));
                    p
                }
            };
            ui.label(crate::i18n::trf(
                "dialog.processed_palette_n",
                &[("n", &pal.len().to_string())],
            ));
            let cell = 9.0;
            let per_row = 24usize;
            let rows = pal.len().div_ceil(per_row);
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(per_row as f32 * cell, rows as f32 * cell),
                egui::Sense::hover(),
            );
            for (i, c) in pal.iter().enumerate() {
                let r = egui::Rect::from_min_size(
                    rect.min + egui::vec2((i % per_row) as f32 * cell, (i / per_row) as f32 * cell),
                    egui::vec2(cell - 1.0, cell - 1.0),
                );
                ui.painter().rect_filled(r, 0.0, rgb32(*c));
            }
        }
    }
}

/// The Remove Background dialog's state: the settings, whether a click
/// on the preview picks a colour, and what a plain click picks: the
/// colour of list entry `row`, or a new entry while `adding`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BgRemoveState {
    pub settings: crate::bg_remove::BgSettings,
    pub picking: bool,
    pub adding: bool,
    pub row: usize,
}

/// Open Remove Background with this session's last settings (the list
/// of colours too), or the defaults the first time.
pub fn open_remove_background(app: &mut App) {
    let settings = app.last_bg_remove.clone().unwrap_or_default();
    app.dialog = crate::ui::dialogs::Dialog::RemoveBackground(BgRemoveState {
        settings,
        ..Default::default()
    });
}

/// What a click on the preview does with the eyedropper on: Shift adds
/// the colour under it to the list, Alt removes the listed colour
/// nearest to it (both keep the eyedropper on), a plain click sets the
/// entry the eyedropper was taken for (or adds one after "+") and puts
/// the eyedropper down. `at` is the point as fractions of the picture,
/// where the fill starts too (none for a colour picked on the drawing);
/// `img` is the copy, for Automatic's colours.
pub fn bg_pick_click(
    st: &mut BgRemoveState,
    c: [u8; 3],
    at: Option<[f32; 2]>,
    modifiers: egui::Modifiers,
    img: Option<&RgbaImage>,
) {
    use crate::bg_remove::BgPick;
    let s = &mut st.settings;
    if s.colors.is_empty() {
        s.colors.push(BgPick::AUTO);
    }
    if modifiers.alt {
        let learnt = match img {
            Some(img) if s.colors.iter().any(|p| p.color.is_none()) => {
                crate::bg_remove::learn_background(img)
            }
            _ => Vec::new(),
        };
        s.remove_nearest(c, &learnt);
        st.row = st.row.min(s.colors.len() - 1);
        return;
    }
    if modifiers.shift || st.adding {
        s.add_color(c, at);
        if !modifiers.shift {
            st.adding = false;
            st.picking = false;
        }
        return;
    }
    let row = st.row.min(s.colors.len() - 1);
    let tolerance = s.colors[row].tolerance;
    s.colors[row] = BgPick {
        color: Some(c),
        tolerance,
        seed: at,
    };
    st.picking = false;
}

/// Bitmaps > Remove Background: the list of colours to remove (each
/// Automatic or picked, with its own tolerance), tolerance, edge
/// softness, Contiguous, Remove specks and Clean edge colors.
pub fn remove_background_dialog(
    app: &mut App,
    ctx: &Context,
    st: &mut BgRemoveState,
    close: &mut bool,
) {
    use crate::bg_remove::remove_background;
    before_after::window(ctx, tr("dialog.remove_background_title")).show(ctx, |ui| {
        let settings = st.settings.clone();
        before_after::columns(ui, RIGHT_W, BOTTOM_H, |left, right| {
            let p = preview(
                left,
                app,
                "bg_remove_preview",
                settings_key(&settings),
                &mut st.picking,
                |img, scale| remove_background(img, &settings, scale),
            );
            // A click on a picture takes the colour under it from the
            // picture as it was, so a removed pixel can be picked too.
            if let (true, Some(p)) = (st.picking, &p) {
                if let Some((x, y)) = p.picked {
                    let (w, h) = p.img.dimensions();
                    let (x, y) = (x.min(w.saturating_sub(1)), y.min(h.saturating_sub(1)));
                    let c = p.img.get_pixel(x, y).0;
                    let at = [
                        (x as f32 + 0.5) / w.max(1) as f32,
                        (y as f32 + 0.5) / h.max(1) as f32,
                    ];
                    let modifiers = left.input(|i| i.modifiers);
                    bg_pick_click(st, [c[0], c[1], c[2]], Some(at), modifiers, Some(&p.img));
                }
            }
            // With the eyedropper on, a click on the drawing picks there
            // too (no seed point: it may be outside the bitmap).
            let on = st.picking;
            if let Some(c) =
                crate::ui::color_picker::pick_for(ctx, egui::Id::new("bg_remove_pick"), on)
            {
                let modifiers = ctx.input(|i| i.modifiers);
                let img = p.as_ref().map(|p| &*p.img);
                bg_pick_click(st, c.to_rgb8(), None, modifiers, img);
            }
            before_after::panel(right, "bg_remove_settings", before_after::STEPS_H, |ui| {
                bg_remove_settings(ui, st)
            });
            // A colour picked on the preview is a step like a slider's.
            let steps = before_after::steps_for(
                right,
                egui::Id::new("bg_remove_steps"),
                &mut st.settings,
                Default::default(),
                &crate::ui::tips::keyed("dialog.reset"),
            );
            if steps.reset {
                st.picking = false;
                st.adding = false;
                st.row = 0;
            }
        });
        if super::dialogs::ok_cancel(ui, close) {
            let settings = st.settings.clone();
            // Remembered for the session; the points picked belong to
            // this bitmap, so they are not.
            let mut kept = settings.clone();
            for p in &mut kept.colors {
                p.seed = None;
            }
            app.last_bg_remove = Some(kept);
            app.apply_to_bitmaps("Remove Background", move |img, _| {
                remove_background(img, &settings, 1.0)
            });
        }
    });
}

/// The test names of the list's controls, row by row.
const BG_PICK_MARKS: [&str; 4] = ["bg_pick_0", "bg_pick_1", "bg_pick_2", "bg_pick_3"];
const BG_DROP_MARKS: [&str; 4] = ["bg_drop_0", "bg_drop_1", "bg_drop_2", "bg_drop_3"];
const BG_TOL_MARKS: [&str; 4] = ["bg_tol_0", "bg_tol_1", "bg_tol_2", "bg_tol_3"];

/// A small square button drawing `glyph`, pressed while `on`.
fn bg_icon_button(
    ui: &mut Ui,
    on: bool,
    enabled: bool,
    glyph: impl FnOnce(&egui::Painter, egui::Rect, Color32),
) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), sense);
    crate::ui::propbar::frame(ui, rect, enabled && resp.hovered(), on);
    let colour = if enabled {
        crate::theme::Tokens::icon()
    } else {
        crate::theme::Tokens::icon().gamma_multiply(0.4)
    };
    glyph(
        ui.painter(),
        egui::Rect::from_center_size(rect.center(), egui::vec2(16.0, 16.0)),
        colour,
    );
    resp
}

/// One colour's swatch: the colour, or for Automatic a square half
/// light, half dark.
fn bg_swatch(ui: &mut Ui, color: Option<[u8; 3]>) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
    let p = ui.painter();
    match color {
        Some(c) => {
            p.rect_filled(rect, 2.0, rgb32(c));
        }
        None => {
            // Automatic: a diagonal half light, half dark.
            p.rect_filled(rect, 2.0, Color32::WHITE);
            p.add(egui::Shape::convex_polygon(
                vec![rect.right_top(), rect.right_bottom(), rect.left_bottom()],
                Color32::from_gray(150),
                egui::Stroke::NONE,
            ));
        }
    }
    p.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, crate::theme::Tokens::border()),
        egui::StrokeKind::Inside,
    );
    resp
}

/// The list of colours to remove: each row its swatch, its eyedropper,
/// Automatic for the first, its tolerance and a remove button; "+" adds
/// a colour picked on the preview.
fn bg_color_list(ui: &mut Ui, st: &mut BgRemoveState) {
    use crate::bg_remove::BgPick;
    if st.settings.colors.is_empty() {
        st.settings.colors.push(BgPick::AUTO);
    }
    let global = st.settings.tolerance;
    let alone = st.settings.colors.len() == 1;
    let mut gone = None;
    for i in 0..st.settings.colors.len() {
        let entry = st.settings.colors[i];
        ui.horizontal(|ui| {
            let swatch = bg_swatch(ui, entry.color);
            match entry.color {
                Some(c) => swatch.rich_tip(format!(
                    "{}\n#{:02X}{:02X}{:02X}",
                    tr("dialog.bg_color_remove"),
                    c[0],
                    c[1],
                    c[2]
                )),
                None => swatch.rich_tip(crate::ui::tips::pair(
                    "dialog.bg_colors_auto_button",
                    "dialog.bg_colors_auto",
                )),
            };
            let picking_here = st.picking && !st.adding && st.row == i;
            let pick = bg_icon_button(ui, picking_here, true, |p, r, c| {
                crate::ui::icons::draw(p, r, crate::tools::Tool::ColorEyedropper, c)
            });
            if let Some(m) = BG_PICK_MARKS.get(i) {
                before_after::mark(ui, m, pick.rect);
            }
            if pick
                .rich_tip(crate::ui::tips::pair(
                    "dialog.bg_pick",
                    "dialog.bg_pick_tip",
                ))
                .clicked()
            {
                st.picking = !picking_here;
                st.adding = false;
                st.row = i;
            }
            if i == 0 {
                if entry.color.is_none() {
                    ui.label(tr("dialog.bg_colors_auto_button"));
                } else if ui
                    .small_button(tr("dialog.bg_colors_auto_button"))
                    .rich_tip(crate::ui::tips::pair(
                        "dialog.bg_colors_auto_button",
                        "dialog.bg_colors_auto_tip",
                    ))
                    .clicked()
                {
                    st.settings.colors[0] = BgPick {
                        tolerance: entry.tolerance,
                        ..BgPick::AUTO
                    };
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let drop = bg_icon_button(ui, false, !alone, |p, r, c| {
                    let r = r.shrink(4.0);
                    let s = egui::Stroke::new(1.5, c);
                    p.line_segment([r.left_top(), r.right_bottom()], s);
                    p.line_segment([r.right_top(), r.left_bottom()], s);
                });
                if let Some(m) = BG_DROP_MARKS.get(i) {
                    before_after::mark(ui, m, drop.rect);
                }
                if drop
                    .rich_tip(crate::ui::tips::pair(
                        "dialog.bg_row_remove",
                        "dialog.bg_row_remove_tip",
                    ))
                    .clicked()
                {
                    gone = Some(i);
                }
                let mut t = entry.tolerance_or(global);
                let field = crate::ui::tips::tip_over(
                    ui,
                    crate::ui::tips::pair("dialog.tolerance", "dialog.bg_row_tolerance_tip"),
                    |ui| {
                        ui.add(
                            crate::ui::field::NumField::new(&mut t)
                                .range(0.0..=100.0)
                                .fixed_decimals(0)
                                .width(56.0),
                        )
                    },
                );
                if let Some(m) = BG_TOL_MARKS.get(i) {
                    before_after::mark(ui, m, field.rect);
                }
                if field.changed() {
                    st.settings.colors[i].tolerance = Some(t.round().clamp(0.0, 100.0));
                }
            });
        });
    }
    if let Some(i) = gone {
        st.settings.remove(i);
        st.row = st.row.min(st.settings.colors.len() - 1);
        st.picking &= st.adding;
    }
    let adding = st.picking && st.adding;
    ui.horizontal(|ui| {
        let add = bg_icon_button(ui, adding, true, |p, r, c| {
            let r = r.shrink(3.0);
            let s = egui::Stroke::new(1.5, c);
            p.line_segment([r.center_top(), r.center_bottom()], s);
            p.line_segment([r.left_center(), r.right_center()], s);
        });
        before_after::mark(ui, "bg_add", add.rect);
        if add
            .rich_tip(crate::ui::tips::pair(
                "dialog.bg_add_color",
                "dialog.bg_add_color_tip",
            ))
            .clicked()
        {
            st.picking = !adding;
            st.adding = !adding;
        }
        ui.label(tr("dialog.bg_add_color"));
    });
    ui.label(
        egui::RichText::new(tr("dialog.bg_colors_hint"))
            .weak()
            .small(),
    );
}

/// The Remove Background settings: the colours to remove, the
/// Tolerance and Edge softness sliders, then the three boxes.
fn bg_remove_settings(ui: &mut Ui, st: &mut BgRemoveState) {
    {
        ui.label(tr("dialog.bg_colors"));
        bg_color_list(ui, st);
        before_after::rule(ui);
        // Whole numbers, the box and the thumb alike.
        panel_slider(
            ui,
            "tolerance",
            "dialog.tolerance",
            crate::ui::Slider::new(&mut st.settings.tolerance, 0.0..=100.0).integer(),
        );
        panel_slider(
            ui,
            "edge_softness",
            "dialog.edge_softness",
            crate::ui::Slider::new(&mut st.settings.softness, 0.0..=20.0).suffix(" px"),
        );
        before_after::rule(ui);
        ui.checkbox(&mut st.settings.contiguous, tr("dialog.bg_contiguous"))
            .rich_tip(crate::ui::tips::pair(
                "dialog.bg_contiguous",
                "dialog.bg_contiguous_tip",
            ));
        ui.checkbox(&mut st.settings.despeckle, tr("dialog.bg_despeckle"))
            .rich_tip(crate::ui::tips::pair(
                "dialog.bg_despeckle",
                "dialog.bg_despeckle_tip",
            ));
        ui.checkbox(&mut st.settings.clean_edges, tr("dialog.bg_clean_edges"))
            .rich_tip(crate::ui::tips::pair(
                "dialog.bg_clean_edges",
                "dialog.bg_clean_edges_tip",
            ));
    }
}
