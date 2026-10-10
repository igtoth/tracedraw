//! File > New: the Create a New Document dialog and the settings it keeps
//! for the next drawing (see docs/behavior/new-document.md).
//!
//! Layout follows the target design: a General section (name,
//! destination preset with a menu to save or delete presets, number of
//! pages, primary colour mode), a Dimensions section (page size, width
//! with the drawing units, height with the orientation buttons,
//! resolution) and a collapsed Color settings section (profiles and
//! rendering intent). Under them a help button, "Do not show this dialog
//! again", OK and Cancel.

use crate::app::{App, Units};
use crate::i18n::tr;
use crate::theme::Tokens;
use egui::{Context, Ui};
use serde::{Deserialize, Serialize};
use tracedraw_core::geometry::Size;
use tracedraw_core::Document;

/// Built-in destination presets, in the order of the Preset list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    Default,
    Cmyk,
    Rgb,
    Web,
}

impl Destination {
    pub const ALL: [Destination; 4] = [
        Destination::Default,
        Destination::Cmyk,
        Destination::Rgb,
        Destination::Web,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Destination::Default => "default",
            Destination::Cmyk => "cmyk",
            Destination::Rgb => "rgb",
            Destination::Web => "web",
        }
    }

    pub fn label(self) -> String {
        tr(match self {
            Destination::Default => "newdoc.preset_default",
            Destination::Cmyk => "newdoc.preset_cmyk",
            Destination::Rgb => "newdoc.preset_rgb",
            Destination::Web => "newdoc.preset_web",
        })
    }

    /// The settings a preset stands for.
    pub fn values(self) -> PresetValues {
        let a4 = tracedraw_core::document::paper::A4;
        match self {
            Destination::Default | Destination::Cmyk => PresetValues {
                rgb: false,
                page_size: "A4".into(),
                width_mm: a4.width,
                height_mm: a4.height,
                units: "mm".into(),
                dpi: 300.0,
            },
            Destination::Rgb => PresetValues {
                rgb: true,
                page_size: "A4".into(),
                width_mm: a4.width,
                height_mm: a4.height,
                units: "mm".into(),
                dpi: 300.0,
            },
            Destination::Web => PresetValues {
                rgb: true,
                page_size: CUSTOM.into(),
                width_mm: 1920.0 * 25.4 / 72.0,
                height_mm: 1080.0 * 25.4 / 72.0,
                units: "px".into(),
                dpi: 72.0,
            },
        }
    }
}

/// Name used for a page size or preset that matches no list entry.
pub const CUSTOM: &str = "Custom";

/// What a destination preset sets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PresetValues {
    pub rgb: bool,
    pub page_size: String,
    pub width_mm: f64,
    pub height_mm: f64,
    pub units: String,
    pub dpi: f64,
}

impl Default for PresetValues {
    fn default() -> Self {
        Destination::Default.values()
    }
}

/// A preset saved by the user from the dialog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SavedPreset {
    pub name: String,
    pub values: PresetValues,
}

/// The dialog's last used settings, kept in the settings file so the next
/// drawing (with or without the dialog) starts from them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NewDocSettings {
    /// A built-in preset id, a saved preset's name, or [`CUSTOM`].
    pub preset: String,
    pub pages: u32,
    pub values: PresetValues,
    pub rgb_profile: String,
    pub cmyk_profile: String,
    pub gray_profile: String,
    pub intent: String,
    pub saved_presets: Vec<SavedPreset>,
}

impl Default for NewDocSettings {
    fn default() -> Self {
        NewDocSettings {
            preset: Destination::Default.id().into(),
            pages: 1,
            values: PresetValues::default(),
            rgb_profile: "sRGB IEC61966-2.1".into(),
            cmyk_profile: "Generic CMYK (open)".into(),
            gray_profile: GRAY_PROFILES[0].into(),
            intent: INTENTS[1].into(),
            saved_presets: Vec::new(),
        }
    }
}

const GRAY_PROFILES: [&str; 3] = ["Dot Gain 20%", "Dot Gain 15%", "Gray Gamma 2.2"];
const INTENTS: [&str; 4] = [
    "Perceptual",
    "Relative colorimetric",
    "Saturation",
    "Absolute colorimetric",
];
const RESOLUTIONS: [f64; 9] = [
    72.0, 96.0, 150.0, 200.0, 300.0, 400.0, 600.0, 1200.0, 2400.0,
];

impl NewDocSettings {
    /// A drawing with these settings.
    pub fn build(&self, name: String) -> Document {
        let size = Size::new(
            self.values.width_mm.max(1.0),
            self.values.height_mm.max(1.0),
        );
        let mut doc = App::localized_document(name, size);
        let extra = self.pages.clamp(1, 999) - 1;
        for _ in 0..extra {
            let n = doc.pages.len() + 1;
            let add = tracedraw_core::Command::AddPage {
                name: Some(crate::i18n::trf("doc.page_n", &[("n", &n.to_string())])),
                size,
            };
            if add.apply(&mut doc).is_err() {
                break;
            }
            if let Some(p) = doc.pages.last_mut() {
                for (k, l) in p.layers.iter_mut().enumerate() {
                    l.name = crate::i18n::trf("docker.layer_n", &[("n", &(k + 1).to_string())]);
                }
            }
        }
        let m = &mut doc.metadata;
        m.resolution_dpi = self.values.dpi;
        m.rgb_profile = self.rgb_profile.clone();
        m.cmyk_profile = self.cmyk_profile.clone();
        m.primary_color_mode = if self.values.rgb {
            tracedraw_core::document::PrimaryColorMode::Rgb
        } else {
            tracedraw_core::document::PrimaryColorMode::Cmyk
        };
        doc
    }

    /// The values a preset name stands for, if it names one.
    fn preset_values(&self, name: &str) -> Option<PresetValues> {
        Destination::ALL
            .iter()
            .find(|d| d.id() == name)
            .map(|d| d.values())
            .or_else(|| {
                self.saved_presets
                    .iter()
                    .find(|p| p.name == name)
                    .map(|p| p.values.clone())
            })
    }

    fn preset_label(&self, name: &str) -> String {
        Destination::ALL
            .iter()
            .find(|d| d.id() == name)
            .map(|d| d.label())
            .unwrap_or_else(|| {
                if name == CUSTOM {
                    tr("newdoc.preset_custom")
                } else {
                    name.to_string()
                }
            })
    }

    /// After an edit: the preset still applies only if every value matches.
    fn refresh_preset(&mut self) {
        if self.preset_values(&self.preset).as_ref() != Some(&self.values) {
            self.preset = CUSTOM.into();
        }
    }
}

/// The open dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct NewDocState {
    pub name: String,
    pub settings: NewDocSettings,
    pub dont_show: bool,
    pub color_settings_open: bool,
    /// Typing a name for Save Preset (Some while the field is shown).
    pub saving_preset: Option<String>,
}

impl NewDocState {
    pub fn from_settings(settings: &NewDocSettings, name: String) -> Self {
        NewDocState {
            name,
            settings: settings.clone(),
            dont_show: false,
            color_settings_open: false,
            saving_preset: None,
        }
    }

    fn units(&self) -> Units {
        Units::from_id(&self.settings.values.units)
    }

    fn set_page_size(&mut self, name: &str, size: Size) {
        let v = &mut self.settings.values;
        let landscape = v.width_mm > v.height_mm;
        v.page_size = name.to_string();
        // A new paper size keeps the chosen orientation.
        let (w, h) = (size.width.min(size.height), size.width.max(size.height));
        (v.width_mm, v.height_mm) = if landscape { (h, w) } else { (w, h) };
        self.settings.refresh_preset();
    }

    /// After a typed width or height: the page size name that matches, in
    /// either orientation, or Custom.
    fn match_page_size(&mut self) {
        let v = &mut self.settings.values;
        let (w, h) = (v.width_mm, v.height_mm);
        v.page_size = crate::ui::dialogs::paper_presets()
            .iter()
            .find(|(_, s)| {
                let close = |a: f64, b: f64| (a - b).abs() < 0.05;
                (close(s.width, w) && close(s.height, h))
                    || (close(s.width, h) && close(s.height, w))
            })
            .map(|(n, _)| n.to_string())
            .unwrap_or_else(|| CUSTOM.into());
        self.settings.refresh_preset();
    }

    fn apply_preset(&mut self, name: &str) {
        if let Some(values) = self.settings.preset_values(name) {
            self.settings.values = values;
        }
        self.settings.preset = name.to_string();
    }
}

/// Dialog geometry, in pixels from the dialog's left edge and from the
/// top of its body (under the title bar), as the target design lays it
/// out: labels end at 190, fields start at 195, rows 33 px apart.
const DIALOG_W: f32 = 480.0;
const LABEL_RIGHT: f32 = 190.0;
const FIELD_X: f32 = 195.0;
const WIDE: f32 = 231.0;
const NUMBER_W: f32 = 100.0;
const BODY_COLLAPSED: f32 = 474.0;
const COLOR_ROWS: f32 = 4.0;

/// Draw the dialog; `close` is set when it should go away.
pub fn new_document_dialog(app: &mut App, ctx: &Context, st: &mut NewDocState, close: &mut bool) {
    use crate::ui::chrome::{self, FIELD_H, ROW_PITCH};
    // Pixel sizes in the dialog are at the dialog's resolution.
    let outer_dpi = crate::app::pixel_dpi();
    crate::app::set_pixel_dpi(st.settings.values.dpi);
    let mut ok = false;
    let extra = if st.color_settings_open {
        COLOR_ROWS * ROW_PITCH
    } else {
        0.0
    };
    // Saving a preset adds the name row under the preset.
    let saving = if st.saving_preset.is_some() {
        ROW_PITCH
    } else {
        0.0
    };
    let height = chrome::TITLE_BAR + BODY_COLLAPSED + extra + saving;
    let x_closed = chrome::dialog(
        ctx,
        "new_document_dialog",
        &tr("dialog.create_new_document"),
        egui::vec2(DIALOG_W, height),
        |ui| {
            let body = ui.max_rect();
            let at = |x: f32, y: f32| body.min + egui::vec2(x, y);
            let field = |x: f32, y: f32, w: f32| {
                egui::Rect::from_min_size(at(x, y), egui::vec2(w, FIELD_H))
            };
            let mut y = 39.0;
            chrome::heading(ui, at(14.0, 23.0), &tr("newdoc.general"));
            // Name.
            chrome::label_right(
                ui,
                body.left() + LABEL_RIGHT,
                at(0.0, y + 14.0).y,
                &tr("dialog.name"),
            );
            chrome::text_field(ui, field(FIELD_X, y, WIDE), &mut st.name);
            y += ROW_PITCH;
            // Preset and its menu.
            chrome::label_right(
                ui,
                body.left() + LABEL_RIGHT,
                at(0.0, y + 14.0).y,
                &tr("newdoc.preset"),
            );
            preset_row(ui, st, field(FIELD_X, y, WIDE), at(FIELD_X + WIDE + 8.0, y));
            y += ROW_PITCH;
            if st.saving_preset.is_some() {
                chrome::label_right(
                    ui,
                    body.left() + LABEL_RIGHT,
                    at(0.0, y + 14.0).y,
                    &tr("newdoc.preset_name"),
                );
                let mut done = None;
                if let Some(name) = &mut st.saving_preset {
                    chrome::text_field(ui, field(FIELD_X, y, 160.0), name);
                    if chrome::button_at(ui, field(FIELD_X + 166.0, y, 65.0), &tr("dialog.ok"))
                        .clicked()
                        && !name.trim().is_empty()
                    {
                        done = Some(name.trim().to_string());
                    }
                }
                if let Some(n) = done {
                    let values = st.settings.values.clone();
                    st.settings.saved_presets.retain(|p| p.name != n);
                    st.settings.saved_presets.push(SavedPreset {
                        name: n.clone(),
                        values,
                    });
                    st.settings.preset = n;
                    st.saving_preset = None;
                }
                y += ROW_PITCH;
            }
            // Number of pages.
            chrome::label_right(
                ui,
                body.left() + LABEL_RIGHT,
                at(0.0, y + 14.0).y,
                &tr("newdoc.pages"),
            );
            if let Some(v) = chrome::spin_field(
                ui,
                "newdoc_pages",
                field(FIELD_X, y, 62.0),
                st.settings.pages as f64,
                0,
                "",
                1.0,
                1.0..=999.0,
            ) {
                st.settings.pages = v.round() as u32;
            }
            y += ROW_PITCH;
            // Primary colour mode.
            chrome::label_right(
                ui,
                body.left() + LABEL_RIGHT,
                at(0.0, y + 14.0).y,
                &tr("newdoc.color_mode"),
            );
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(field(FIELD_X, y, WIDE)),
                |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 16.0;
                        let v = &mut st.settings.values;
                        let mut rgb = v.rgb;
                        ui.radio_value(&mut rgb, false, "CMYK");
                        ui.radio_value(&mut rgb, true, "RGB");
                        if rgb != v.rgb {
                            v.rgb = rgb;
                            st.settings.refresh_preset();
                        }
                    });
                },
            );
            y += ROW_PITCH + 42.0;
            chrome::heading(ui, at(14.0, y - 16.0), &tr("newdoc.dimensions"));
            dimensions(ui, st, body, y);
            y += 5.0 * ROW_PITCH + 26.0;
            // Colour settings: a bold header with a triangle; open, the
            // profiles and the intent.
            let header = egui::Rect::from_min_size(at(14.0, y - 10.0), egui::vec2(200.0, 22.0));
            let resp = ui.interact(
                header,
                egui::Id::new("newdoc_color_header"),
                egui::Sense::click(),
            );
            let tri = at(29.0, y + 1.0);
            let pts = if st.color_settings_open {
                vec![
                    tri + egui::vec2(-4.0, -2.0),
                    tri + egui::vec2(4.0, -2.0),
                    tri + egui::vec2(0.0, 3.0),
                ]
            } else {
                vec![
                    tri + egui::vec2(-2.0, -4.0),
                    tri + egui::vec2(3.0, 0.0),
                    tri + egui::vec2(-2.0, 4.0),
                ]
            };
            ui.painter().add(egui::epaint::PathShape::convex_polygon(
                pts,
                Tokens::TEXT_DIM,
                egui::Stroke::NONE,
            ));
            chrome::heading(ui, at(45.0, y + 1.0), &tr("newdoc.color_settings"));
            if resp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                st.color_settings_open = !st.color_settings_open;
            }
            y += 29.0;
            if st.color_settings_open {
                color_settings(ui, st, body, y);
                y += COLOR_ROWS * ROW_PITCH;
            }
            // The bottom row.
            let help = egui::Rect::from_min_size(at(14.0, y), egui::vec2(FIELD_H, FIELD_H));
            if chrome::button_at(ui, help, "?")
                .on_hover_text(tr("menu.help.product_help"))
                .clicked()
            {
                app.open_url(HELP_URL);
            }
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    at(46.0, y),
                    egui::vec2(210.0, FIELD_H),
                )),
                |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.checkbox(&mut st.dont_show, tr("newdoc.dont_show"));
                    });
                },
            );
            if chrome::button_at(ui, field(263.0, y, 100.0), &tr("dialog.ok")).clicked() {
                ok = true;
            }
            if chrome::button_at(ui, field(366.0, y, 100.0), &tr("dialog.cancel")).clicked() {
                *close = true;
            }
            if crate::ui::dialogs::enter_pressed(ui) && st.saving_preset.is_none() {
                ok = true;
            }
        },
    );
    if x_closed {
        *close = true;
    }
    crate::app::set_pixel_dpi(outer_dpi);
    if ok {
        *close = true;
        create(app, st);
        app.save_settings();
    }
}

const HELP_URL: &str =
    "https://github.com/igtoth/tracedraw/blob/main/docs/behavior/new-document.md";

/// OK: open the drawing and keep the settings for next time (the caller
/// writes them to disk).
pub fn create(app: &mut App, st: &NewDocState) {
    let name = if st.name.trim().is_empty() {
        app.peek_untitled_name()
    } else {
        st.name.trim().to_string()
    };
    let mut doc = st.settings.build(name);
    app.settings.document_defaults.apply(&mut doc);
    // The name shown was the next Untitled-N: use it up.
    app.untitled_counter += 1;
    app.open_document(doc, None);
    app.units = Units::from_id(&st.settings.values.units);
    app.settings.new_document = st.settings.clone();
    if st.dont_show {
        app.settings.show_new_document_dialog = false;
    }
}

fn preset_row(ui: &mut Ui, st: &mut NewDocState, rect: egui::Rect, menu_at: egui::Pos2) {
    let current = st.settings.preset_label(&st.settings.preset);
    let mut chosen = None;
    crate::ui::chrome::combo(ui, "newdoc_preset", rect, current, |ui| {
        for d in Destination::ALL {
            if ui
                .selectable_label(st.settings.preset == d.id(), d.label())
                .clicked()
            {
                chosen = Some(d.id().to_string());
            }
        }
        for p in &st.settings.saved_presets {
            if ui
                .selectable_label(st.settings.preset == p.name, &p.name)
                .clicked()
            {
                chosen = Some(p.name.clone());
            }
        }
        ui.add_enabled(
            false,
            egui::Button::selectable(st.settings.preset == CUSTOM, tr("newdoc.preset_custom")),
        );
    });
    if let Some(name) = chosen {
        st.apply_preset(&name);
    }
    // The preset menu: three bold dots.
    let r = egui::Rect::from_min_size(menu_at, egui::vec2(24.0, crate::ui::chrome::FIELD_H));
    let resp = ui.interact(r, egui::Id::new("newdoc_preset_menu"), egui::Sense::click());
    if resp.hovered() {
        ui.painter()
            .rect_filled(r, 0.0, crate::ui::chrome::CONTROL_HOVER);
    }
    for i in -1..=1 {
        ui.painter().circle_filled(
            r.center() + egui::vec2(i as f32 * 6.0, 0.0),
            2.0,
            egui::Color32::from_gray(40),
        );
    }
    egui::Popup::menu(&resp)
        .id(egui::Id::new("newdoc_preset_menu_popup"))
        .style(crate::ui::menus::menu_popup_style)
        .show(|ui| {
            crate::ui::menus::body(ui, |ui| {
                if crate::ui::menus::item(ui, "newdoc.save_preset", "", true) {
                    st.saving_preset = Some(String::new());
                }
                let saved = st
                    .settings
                    .saved_presets
                    .iter()
                    .any(|p| p.name == st.settings.preset);
                if crate::ui::menus::item(ui, "newdoc.delete_preset", "", saved) {
                    let name = st.settings.preset.clone();
                    st.settings.saved_presets.retain(|p| p.name != name);
                    st.settings.preset = CUSTOM.into();
                }
            })
        });
}

fn dimensions(ui: &mut Ui, st: &mut NewDocState, body: egui::Rect, top: f32) {
    use crate::ui::chrome::{self, FIELD_H, ROW_PITCH};
    let units = st.units();
    let at = |x: f32, y: f32| body.min + egui::vec2(x, y);
    let field =
        |x: f32, y: f32, w: f32| egui::Rect::from_min_size(at(x, y), egui::vec2(w, FIELD_H));
    let label = |ui: &mut Ui, y: f32, key: &str| {
        chrome::label_right(ui, body.left() + LABEL_RIGHT, at(0.0, y + 14.0).y, &tr(key));
    };
    let mut y = top;
    // Page size.
    label(ui, y, "newdoc.page_size");
    let mut pick = None;
    let shown = if st.settings.values.page_size == CUSTOM {
        tr("newdoc.preset_custom")
    } else {
        st.settings.values.page_size.clone()
    };
    chrome::combo(
        ui,
        "newdoc_page_size",
        field(FIELD_X, y, WIDE),
        shown,
        |ui| {
            for (n, s) in crate::ui::dialogs::paper_presets() {
                if ui
                    .selectable_label(st.settings.values.page_size == n, n)
                    .clicked()
                {
                    pick = Some((n, s));
                }
            }
        },
    );
    if let Some((n, s)) = pick {
        st.set_page_size(n, s);
    }
    y += ROW_PITCH;
    // Width and the units.
    label(ui, y, "newdoc.width");
    if length(
        ui,
        "newdoc_width",
        field(FIELD_X, y, NUMBER_W),
        &mut st.settings.values.width_mm,
        units,
    ) {
        st.match_page_size();
    }
    let mut u = units;
    chrome::combo(
        ui,
        "newdoc_units",
        field(300.0, y, 123.0),
        u.label(),
        |ui| {
            for x in Units::ALL {
                ui.selectable_value(&mut u, x, x.label());
            }
        },
    );
    if u != units {
        st.settings.values.units = u.id().into();
        st.settings.refresh_preset();
    }
    y += ROW_PITCH;
    // Height.
    label(ui, y, "newdoc.height");
    if length(
        ui,
        "newdoc_height",
        field(FIELD_X, y, NUMBER_W),
        &mut st.settings.values.height_mm,
        units,
    ) {
        st.match_page_size();
    }
    y += ROW_PITCH;
    // Orientation.
    label(ui, y, "newdoc.orientation");
    let v = &mut st.settings.values;
    let landscape = v.width_mm > v.height_mm;
    if orientation_button(ui, field(FIELD_X, y, FIELD_H), false, !landscape)
        .on_hover_text(tr("dialog.portrait"))
        .clicked()
        && landscape
    {
        std::mem::swap(&mut v.width_mm, &mut v.height_mm);
        st.settings.refresh_preset();
    }
    let v = &mut st.settings.values;
    if orientation_button(ui, field(FIELD_X + 33.0, y, FIELD_H), true, landscape)
        .on_hover_text(tr("dialog.landscape"))
        .clicked()
        && !landscape
    {
        std::mem::swap(&mut v.width_mm, &mut v.height_mm);
        st.settings.refresh_preset();
    }
    y += ROW_PITCH;
    // Resolution: a list of the usual ones that also takes a typed value.
    label(ui, y, "newdoc.resolution");
    let v = &mut st.settings.values;
    let mut dpi = v.dpi;
    if let Some(d) = chrome::number_field(
        ui,
        "newdoc_dpi",
        field(FIELD_X, y, NUMBER_W - 22.0),
        dpi,
        0,
        "",
        36.0..=10000.0,
    ) {
        dpi = d;
    }
    chrome::combo(
        ui,
        "newdoc_dpi_list",
        field(FIELD_X + NUMBER_W - 23.0, y, 23.0),
        "",
        |ui| {
            for r in RESOLUTIONS {
                ui.selectable_value(&mut dpi, r, format!("{}", r as i64));
            }
        },
    );
    chrome::label_at(
        ui,
        at(FIELD_X + NUMBER_W + 6.0, y + 14.0),
        &tr("newdoc.dpi"),
    );
    if dpi != v.dpi && dpi > 0.0 {
        // In pixels the page keeps its pixel size at the new resolution.
        if units == Units::Pixels {
            v.width_mm *= v.dpi / dpi;
            v.height_mm *= v.dpi / dpi;
        }
        v.dpi = dpi;
        crate::app::set_pixel_dpi(dpi);
        st.settings.refresh_preset();
    }
}

fn color_settings(ui: &mut Ui, st: &mut NewDocState, body: egui::Rect, top: f32) {
    use crate::ui::chrome::{self, FIELD_H, ROW_PITCH};
    let at = |x: f32, y: f32| body.min + egui::vec2(x, y);
    let s = &mut st.settings;
    let rgb = [s.rgb_profile.clone(), "sRGB IEC61966-2.1".to_string()];
    let cmyk = [s.cmyk_profile.clone(), "Generic CMYK (open)".to_string()];
    let gray = GRAY_PROFILES.map(String::from);
    let intents = INTENTS.map(String::from);
    let rows: [(&str, &str, &mut String, &[String]); 4] = [
        ("newdoc_rgb", "newdoc.rgb_profile", &mut s.rgb_profile, &rgb),
        (
            "newdoc_cmyk",
            "newdoc.cmyk_profile",
            &mut s.cmyk_profile,
            &cmyk,
        ),
        (
            "newdoc_gray",
            "newdoc.gray_profile",
            &mut s.gray_profile,
            &gray,
        ),
        ("newdoc_intent", "newdoc.intent", &mut s.intent, &intents),
    ];
    for (i, (id, key, value, options)) in rows.into_iter().enumerate() {
        let y = top + i as f32 * ROW_PITCH;
        chrome::label_right(ui, body.left() + LABEL_RIGHT, at(0.0, y + 14.0).y, &tr(key));
        let rect = egui::Rect::from_min_size(at(FIELD_X, y), egui::vec2(WIDE, FIELD_H));
        chrome::combo(ui, id, rect, value.clone(), |ui| {
            let mut seen: Vec<&String> = Vec::new();
            for o in options {
                if seen.contains(&o) {
                    continue;
                }
                seen.push(o);
                ui.selectable_value(value, o.clone(), o);
            }
        });
    }
}

/// A length field in the drawing units with its spin arrows; true when
/// edited.
fn length(ui: &mut Ui, id: &str, rect: egui::Rect, mm: &mut f64, units: Units) -> bool {
    let v = units.from_mm(*mm);
    let decimals = if units == Units::Pixels { 0 } else { 1 };
    let step = if units == Units::Pixels { 1.0 } else { 0.1 };
    match crate::ui::chrome::spin_field(
        ui,
        id,
        rect,
        v,
        decimals,
        units.short(),
        step,
        0.001..=1.0e7,
    ) {
        Some(nv) => {
            *mm = units.to_mm(nv).max(0.01);
            true
        }
        None => false,
    }
}

/// Portrait or landscape button: a small page outline, framed when it is
/// the page's orientation.
fn orientation_button(
    ui: &mut Ui,
    rect: egui::Rect,
    landscape: bool,
    selected: bool,
) -> egui::Response {
    let id = egui::Id::new(("newdoc_orientation", landscape));
    let resp = ui.interact(rect, id, egui::Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect(
            rect,
            0.0,
            egui::Color32::WHITE,
            egui::Stroke::new(1.0, crate::ui::chrome::FIELD_BORDER),
            egui::StrokeKind::Inside,
        );
    } else if resp.hovered() {
        painter.rect_filled(rect, 0.0, crate::ui::chrome::CONTROL_HOVER);
    }
    let size = if landscape {
        egui::vec2(16.0, 12.0)
    } else {
        egui::vec2(10.0, 16.0)
    };
    let page = egui::Rect::from_center_size(rect.center(), size);
    painter.rect_stroke(
        page,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(40)),
        egui::StrokeKind::Inside,
    );
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_makes_the_pages_size_resolution_and_colour_mode() {
        let mut s = NewDocSettings {
            pages: 3,
            ..Default::default()
        };
        s.values.rgb = true;
        s.values.dpi = 150.0;
        let doc = s.build("Poster".into());
        assert_eq!(doc.title, "Poster");
        assert_eq!(doc.pages.len(), 3);
        assert!((doc.pages[2].size.width - 210.0).abs() < 1e-9);
        assert_eq!(doc.metadata.resolution_dpi, 150.0);
        assert_eq!(
            doc.metadata.primary_color_mode,
            tracedraw_core::document::PrimaryColorMode::Rgb
        );
    }

    #[test]
    fn editing_a_value_turns_the_preset_into_custom() {
        let mut st = NewDocState::from_settings(&NewDocSettings::default(), "Untitled-1".into());
        assert_eq!(st.settings.preset, "default");
        st.settings.values.dpi = 600.0;
        st.settings.refresh_preset();
        assert_eq!(st.settings.preset, CUSTOM);
        st.apply_preset("web");
        assert_eq!(st.settings.values.units, "px");
        assert!(st.settings.values.rgb);
        assert_eq!(st.settings.values.dpi, 72.0);
        st.settings.refresh_preset();
        assert_eq!(st.settings.preset, "web");
    }

    #[test]
    fn page_sizes_keep_the_orientation_and_typed_sizes_find_their_name() {
        let mut st = NewDocState::from_settings(&NewDocSettings::default(), "U".into());
        let v = &mut st.settings.values;
        std::mem::swap(&mut v.width_mm, &mut v.height_mm);
        st.set_page_size("A3", tracedraw_core::document::paper::A3);
        assert!((st.settings.values.width_mm - 420.0).abs() < 1e-9);
        st.settings.values.width_mm = 279.4;
        st.settings.values.height_mm = 215.9;
        st.match_page_size();
        assert_eq!(st.settings.values.page_size, "Letter");
        st.settings.values.width_mm = 100.0;
        st.match_page_size();
        assert_eq!(st.settings.values.page_size, CUSTOM);
    }

    #[test]
    fn ok_opens_a_new_tab_and_remembers_the_settings() {
        let mut app = App::headless();
        let before = app.docs.len();
        let mut st =
            NewDocState::from_settings(&app.settings.new_document, app.peek_untitled_name());
        st.apply_preset("web");
        st.dont_show = true;
        create(&mut app, &st);
        assert_eq!(app.docs.len(), before + 1);
        assert_eq!(app.units, Units::Pixels);
        assert_eq!(app.settings.new_document.preset, "web");
        assert!(!app.settings.show_new_document_dialog);
        // With the dialog off, New uses the remembered settings at once.
        app.request_new_document();
        assert_eq!(app.docs.len(), before + 2);
        assert!((app.page_size().width - 1920.0 * 25.4 / 72.0).abs() < 1e-6);
    }
}
