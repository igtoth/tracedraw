//! Bitmaps > Mode dialogs: Black and White (1-bit), Duotone (8-bit) and
//! Paletted (8-bit), each with a before and after preview of the selected
//! bitmap. OK converts every selected bitmap in one undo step.

use crate::app::App;
use crate::bitmap_modes::{
    black_and_white, build_palette, duotone, paletted, BwMethod, BwSettings, Dither,
    DuotoneSettings, DuotoneType, PaletteType, PalettedSettings, Screen,
};
use crate::i18n::tr;
use crate::ui::bitmap_preview::{before_after, settings_key};
use egui::{Color32, Context, Ui};

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
        });
    changed
}

/// Black and White (1-bit): conversion method, threshold or intensity,
/// and the halftone screen.
pub fn bw_dialog(app: &mut App, ctx: &Context, s: &mut BwSettings, close: &mut bool) {
    super::dialogs::window(ctx, tr("dialog.mode_bw")).show(ctx, |ui| {
        let dpi = crate::ui::bitmap_preview::selected_bitmap(app)
            .map(|b| b.1)
            .unwrap_or(300.0);
        let settings = *s;
        before_after(ui, app, settings_key(&settings), 220.0, |img, scale| {
            black_and_white(img, &settings, dpi * scale)
        });
        ui.add_space(6.0);
        egui::Grid::new("bw_grid").num_columns(2).show(ui, |ui| {
            ui.label(tr("dialog.conversion_method"));
            combo(
                ui,
                "bw_method",
                &mut s.method,
                &BwMethod::ALL,
                BwMethod::key,
            );
            ui.end_row();
            match s.method {
                BwMethod::LineArt => {
                    ui.label(tr("dialog.threshold"));
                    ui.add(crate::ui::Rail(egui::Slider::new(
                        &mut s.threshold,
                        0..=255,
                    )));
                    ui.end_row();
                }
                BwMethod::Halftone => {
                    ui.label(tr("dialog.screen_type"));
                    egui::ComboBox::from_id_salt("bw_screen")
                        .selected_text(tr(screen_key(s.screen)))
                        .show_ui(ui, |ui| {
                            for sc in [Screen::Round, Screen::Line, Screen::Square] {
                                ui.selectable_value(&mut s.screen, sc, tr(screen_key(sc)));
                            }
                        });
                    ui.end_row();
                    ui.label(tr("dialog.angle"));
                    ui.add(crate::ui::Rail(
                        egui::Slider::new(&mut s.angle, -180.0..=180.0).suffix(" \u{b0}"),
                    ));
                    ui.end_row();
                    ui.label(tr("dialog.lines_per_inch"));
                    ui.add(crate::ui::Rail(egui::Slider::new(&mut s.lpi, 1.0..=300.0)));
                    ui.end_row();
                }
                _ => {
                    ui.label(tr("dialog.intensity"));
                    ui.add(crate::ui::Rail(egui::Slider::new(
                        &mut s.intensity,
                        0.0..=100.0,
                    )));
                    ui.end_row();
                }
            }
        });
        if super::dialogs::ok_cancel(ui, close) {
            let settings = *s;
            app.apply_to_bitmaps("Black and White", move |img, dpi| {
                black_and_white(img, &settings, dpi)
            });
        }
    });
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
    super::dialogs::window(ctx, tr("dialog.mode_duotone")).show(ctx, |ui| {
        // Four inks always exist; the type uses the first ones.
        while st.settings.inks.len() < 4 {
            let d = DuotoneSettings::default();
            let i = st.settings.inks.len();
            st.settings.inks.push(d.inks[i].clone());
        }
        let settings = st.settings.clone();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.type"));
                    combo(
                        ui,
                        "duo_type",
                        &mut st.settings.kind,
                        &DuotoneType::ALL,
                        DuotoneType::key,
                    );
                });
                let n = st.settings.kind.inks();
                st.ink = st.ink.min(n - 1);
                for i in 0..n {
                    ui.horizontal(|ui| {
                        let mut rgb = st.settings.inks[i].rgb;
                        if ui.color_edit_button_srgb(&mut rgb).changed() {
                            st.settings.inks[i].rgb = rgb;
                        }
                        let name = crate::i18n::trf("dialog.ink_n", &[("n", &(i + 1).to_string())]);
                        if ui.selectable_label(st.ink == i, name).clicked() {
                            st.ink = i;
                        }
                    });
                }
                ui.checkbox(&mut st.show_all, tr("dialog.show_all"));
                ui.horizontal(|ui| {
                    if ui.button(tr("dialog.load")).clicked() {
                        crate::files::Dialog::new()
                            .add_filter(tr("dialog.duotone_inks"), &["json"])
                            .pick_into("duotone_inks");
                    }
                    if ui.button(tr("dialog.save")).clicked() {
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
                        .color(crate::theme::Tokens::TEXT_DIM),
                );
            });
        });
        before_after(ui, app, settings_key(&settings), 200.0, |img, _| {
            duotone(img, &settings)
        });
        if super::dialogs::ok_cancel(ui, close) {
            let settings = st.settings.clone();
            app.apply_to_bitmaps("Duotone", move |img, _| duotone(img, &settings));
        }
    });
}

/// Paletted (8-bit): palette type and colour count, dithering and its
/// intensity, and the processed palette.
pub fn paletted_dialog(app: &mut App, ctx: &Context, s: &mut PalettedSettings, close: &mut bool) {
    if s.palette == PaletteType::Custom && s.custom.is_empty() {
        s.custom = app.document_palette_rgb();
    }
    super::dialogs::window(ctx, tr("dialog.mode_paletted")).show(ctx, |ui| {
        let settings = s.clone();
        let thumb = before_after(ui, app, settings_key(&settings), 220.0, |img, _| {
            paletted(img, &settings)
        });
        ui.add_space(6.0);
        egui::Grid::new("pal_grid").num_columns(2).show(ui, |ui| {
            ui.label(tr("dialog.palette"));
            combo(
                ui,
                "pal_type",
                &mut s.palette,
                &PaletteType::ALL,
                PaletteType::key,
            );
            ui.end_row();
            if s.palette.counts() {
                ui.label(tr("dialog.colors"));
                ui.add(crate::ui::Rail(egui::Slider::new(&mut s.colors, 2..=256)));
                ui.end_row();
            }
            ui.label(tr("dialog.dithering"));
            combo(ui, "pal_dither", &mut s.dither, &Dither::ALL, Dither::key);
            ui.end_row();
            if s.dither != Dither::None {
                ui.label(tr("dialog.dither_intensity"));
                ui.add(crate::ui::Rail(egui::Slider::new(
                    &mut s.intensity,
                    0.0..=100.0,
                )));
                ui.end_row();
            }
        });
        // The processed palette, worked out again when the settings change.
        if let Some(t) = thumb {
            let key = settings_key(&*s);
            let id = egui::Id::new("paletted_processed");
            let pal = match ui.ctx().data(|d| d.get_temp::<(u64, Vec<[u8; 3]>)>(id)) {
                Some((k, p)) if k == key => p,
                _ => {
                    let p = build_palette(&t, s);
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
        if super::dialogs::ok_cancel(ui, close) {
            let settings = s.clone();
            app.apply_to_bitmaps("Paletted", move |img, _| paletted(img, &settings));
        }
    });
}
