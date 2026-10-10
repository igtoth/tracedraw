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
                width_mm: 1920.0 * Units::Pixels.mm(),
                height_mm: 1080.0 * Units::Pixels.mm(),
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

/// Draw the dialog; `close` is set when it should go away.
pub fn new_document_dialog(app: &mut App, ctx: &Context, st: &mut NewDocState, close: &mut bool) {
    let mut ok = false;
    egui::Window::new(tr("dialog.create_new_document"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .min_width(430.0)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            section(ui, &tr("newdoc.general"));
            egui::Grid::new("newdoc_general")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .min_col_width(140.0)
                .show(ui, |ui| {
                    ui.label(tr("dialog.name"));
                    ui.add(egui::TextEdit::singleline(&mut st.name).desired_width(240.0));
                    ui.end_row();
                    ui.label(tr("newdoc.preset"));
                    ui.horizontal(|ui| preset_row(ui, st));
                    ui.end_row();
                    if let Some(name) = &mut st.saving_preset {
                        ui.label(tr("newdoc.preset_name"));
                        let mut done = None;
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(name).desired_width(150.0));
                            if ui.button(tr("dialog.ok")).clicked() && !name.trim().is_empty() {
                                done = Some(name.trim().to_string());
                            }
                        });
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
                        ui.end_row();
                    }
                    ui.label(tr("newdoc.pages"));
                    ui.add(egui::DragValue::new(&mut st.settings.pages).range(1..=999));
                    ui.end_row();
                    ui.label(tr("newdoc.color_mode"));
                    ui.horizontal(|ui| {
                        let v = &mut st.settings.values;
                        let mut rgb = v.rgb;
                        ui.radio_value(&mut rgb, false, "CMYK");
                        ui.radio_value(&mut rgb, true, "RGB");
                        if rgb != v.rgb {
                            v.rgb = rgb;
                            st.settings.refresh_preset();
                        }
                    });
                    ui.end_row();
                });
            ui.add_space(4.0);
            section(ui, &tr("newdoc.dimensions"));
            egui::Grid::new("newdoc_dimensions")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .min_col_width(140.0)
                .show(ui, |ui| dimensions(ui, st));
            ui.add_space(4.0);
            let header = egui::CollapsingHeader::new(tr("newdoc.color_settings"))
                .id_salt("newdoc_color")
                .default_open(st.color_settings_open)
                .show(ui, |ui| color_settings(ui, st));
            st.color_settings_open = header.body_returned.is_some();
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new("?").corner_radius(10.0))
                    .on_hover_text(tr("menu.help.product_help"))
                    .clicked()
                {
                    app.open_url(HELP_URL);
                }
                ui.checkbox(&mut st.dont_show, tr("newdoc.dont_show"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(tr("dialog.cancel")).min_size(egui::vec2(72.0, 0.0)))
                        .clicked()
                    {
                        *close = true;
                    }
                    if ui
                        .add(egui::Button::new(tr("dialog.ok")).min_size(egui::vec2(72.0, 0.0)))
                        .clicked()
                    {
                        ok = true;
                    }
                });
            });
            if crate::ui::dialogs::enter_pressed(ui) && st.saving_preset.is_none() {
                ok = true;
            }
        });
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
    let doc = st.settings.build(name);
    // The name shown was the next Untitled-N: use it up.
    app.untitled_counter += 1;
    app.open_document(doc, None);
    app.units = Units::from_id(&st.settings.values.units);
    app.settings.new_document = st.settings.clone();
    if st.dont_show {
        app.settings.show_new_document_dialog = false;
    }
}

fn section(ui: &mut Ui, title: &str) {
    ui.label(egui::RichText::new(title).size(13.0).color(Tokens::TEXT));
}

fn preset_row(ui: &mut Ui, st: &mut NewDocState) {
    let current = st.settings.preset_label(&st.settings.preset);
    let mut chosen = None;
    egui::ComboBox::from_id_salt("newdoc_preset")
        .width(200.0)
        .selected_text(current)
        .show_ui(ui, |ui| {
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
    ui.menu_button("...", |ui| {
        if ui.button(tr("newdoc.save_preset")).clicked() {
            st.saving_preset = Some(String::new());
            ui.close();
        }
        let saved = st
            .settings
            .saved_presets
            .iter()
            .any(|p| p.name == st.settings.preset);
        if ui
            .add_enabled(saved, egui::Button::new(tr("newdoc.delete_preset")))
            .clicked()
        {
            let name = st.settings.preset.clone();
            st.settings.saved_presets.retain(|p| p.name != name);
            st.settings.preset = CUSTOM.into();
            ui.close();
        }
    });
}

fn dimensions(ui: &mut Ui, st: &mut NewDocState) {
    let units = st.units();
    ui.label(tr("newdoc.page_size"));
    let mut pick = None;
    let shown = if st.settings.values.page_size == CUSTOM {
        tr("newdoc.preset_custom")
    } else {
        st.settings.values.page_size.clone()
    };
    egui::ComboBox::from_id_salt("newdoc_page_size")
        .width(200.0)
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for (n, s) in crate::ui::dialogs::paper_presets() {
                if ui
                    .selectable_label(st.settings.values.page_size == n, n)
                    .clicked()
                {
                    pick = Some((n, s));
                }
            }
        });
    if let Some((n, s)) = pick {
        st.set_page_size(n, s);
    }
    ui.end_row();

    ui.label(tr("newdoc.width"));
    ui.horizontal(|ui| {
        if length(ui, &mut st.settings.values.width_mm, units) {
            st.match_page_size();
        }
        let mut u = units;
        egui::ComboBox::from_id_salt("newdoc_units")
            .width(110.0)
            .selected_text(u.label())
            .show_ui(ui, |ui| {
                for x in Units::ALL {
                    ui.selectable_value(&mut u, x, x.label());
                }
            });
        if u != units {
            st.settings.values.units = u.id().into();
            st.settings.refresh_preset();
        }
    });
    ui.end_row();

    ui.label(tr("newdoc.height"));
    ui.horizontal(|ui| {
        if length(ui, &mut st.settings.values.height_mm, units) {
            st.match_page_size();
        }
        let v = &mut st.settings.values;
        let landscape = v.width_mm > v.height_mm;
        if orientation_button(ui, false, !landscape)
            .on_hover_text(tr("dialog.portrait"))
            .clicked()
            && landscape
        {
            std::mem::swap(&mut v.width_mm, &mut v.height_mm);
            st.settings.refresh_preset();
        }
        let v = &mut st.settings.values;
        if orientation_button(ui, true, landscape)
            .on_hover_text(tr("dialog.landscape"))
            .clicked()
            && !landscape
        {
            std::mem::swap(&mut v.width_mm, &mut v.height_mm);
            st.settings.refresh_preset();
        }
    });
    ui.end_row();

    ui.label(tr("newdoc.resolution"));
    ui.horizontal(|ui| {
        let v = &mut st.settings.values;
        let mut dpi = v.dpi;
        egui::ComboBox::from_id_salt("newdoc_dpi")
            .width(80.0)
            .selected_text(format!("{}", dpi.round() as i64))
            .show_ui(ui, |ui| {
                for r in RESOLUTIONS {
                    ui.selectable_value(&mut dpi, r, format!("{}", r as i64));
                }
            });
        ui.add(
            egui::DragValue::new(&mut dpi)
                .range(36.0..=10000.0)
                .speed(1.0),
        );
        ui.label(tr("newdoc.dpi"));
        if dpi != v.dpi {
            v.dpi = dpi;
            st.settings.refresh_preset();
        }
    });
    ui.end_row();
}

fn color_settings(ui: &mut Ui, st: &mut NewDocState) {
    egui::Grid::new("newdoc_colors")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .min_col_width(128.0)
        .show(ui, |ui| {
            let s = &mut st.settings;
            let rgb = [s.rgb_profile.clone(), "sRGB IEC61966-2.1".to_string()];
            combo(
                ui,
                "newdoc_rgb",
                &tr("newdoc.rgb_profile"),
                &mut s.rgb_profile,
                &rgb,
            );
            let cmyk = [s.cmyk_profile.clone(), "Generic CMYK (open)".to_string()];
            combo(
                ui,
                "newdoc_cmyk",
                &tr("newdoc.cmyk_profile"),
                &mut s.cmyk_profile,
                &cmyk,
            );
            let gray = GRAY_PROFILES.map(String::from);
            combo(
                ui,
                "newdoc_gray",
                &tr("newdoc.gray_profile"),
                &mut s.gray_profile,
                &gray,
            );
            let intents = INTENTS.map(String::from);
            combo(
                ui,
                "newdoc_intent",
                &tr("newdoc.intent"),
                &mut s.intent,
                &intents,
            );
        });
}

fn combo(ui: &mut Ui, id: &str, label: &str, value: &mut String, options: &[String]) {
    ui.label(label);
    egui::ComboBox::from_id_salt(id)
        .width(230.0)
        .selected_text(value.clone())
        .show_ui(ui, |ui| {
            let mut seen: Vec<&String> = Vec::new();
            for o in options {
                if seen.contains(&o) {
                    continue;
                }
                seen.push(o);
                ui.selectable_value(value, o.clone(), o);
            }
        });
    ui.end_row();
}

/// A length field in the drawing units; true when edited.
fn length(ui: &mut Ui, mm: &mut f64, units: Units) -> bool {
    let mut v = units.from_mm(*mm);
    let decimals = if units == Units::Pixels { 0 } else { 3 };
    let changed = ui
        .add(
            egui::DragValue::new(&mut v)
                .speed(0.5)
                .range(0.001..=f64::MAX)
                .max_decimals(decimals)
                .suffix(format!(" {}", units.short())),
        )
        .changed();
    if changed {
        *mm = units.to_mm(v).max(0.01);
    }
    changed
}

/// Portrait or landscape button: a small page outline.
fn orientation_button(ui: &mut Ui, landscape: bool, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 22.0), egui::Sense::click());
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, 2.0, Tokens::TOOL_ACTIVE);
        painter.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, Tokens::SELECTION),
            egui::StrokeKind::Inside,
        );
    } else if resp.hovered() {
        painter.rect_filled(rect, 2.0, Tokens::TOOL_HOVER);
    }
    let size = if landscape {
        egui::vec2(14.0, 10.0)
    } else {
        egui::vec2(10.0, 14.0)
    };
    let page = egui::Rect::from_center_size(rect.center(), size);
    painter.rect_filled(page, 0.0, egui::Color32::WHITE);
    painter.rect_stroke(
        page,
        0.0,
        egui::Stroke::new(1.0, Tokens::TEXT),
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
        assert!((app.page_size().width - 1920.0 * Units::Pixels.mm()).abs() < 1e-6);
    }
}
