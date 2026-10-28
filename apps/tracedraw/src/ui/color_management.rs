//! Tools > Color Management: the Color Management Settings dialog. The
//! Document tab shows the drawing's profiles and assigns or converts new
//! ones, with its primary colour mode and rendering intent; the Default
//! tab holds the defaults of new drawings, the colour engine and the open
//! and import policies. Nothing changes until OK. See
//! docs/behavior/colour-management.md.

use crate::app::App;
use crate::color_policy::{FoundOrName, FoundProfile, Space, PRESETS};
use crate::i18n::{tr, trf};
use crate::settings::{ColorPrefs, ColorPreset};
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Context, RichText, Ui};

const INTENTS: [&str; 4] = [
    "Relative colorimetric",
    "Absolute colorimetric",
    "Perceptual",
    "Saturation",
];

/// The dialog's state: the tab and the choices waiting for OK.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorManagementState {
    pub loaded: bool,
    /// 0 Document, 1 Default.
    pub tab: usize,
    /// Convert (preserve colour values) rather than assign.
    pub convert: bool,
    pub doc_rgb: Option<FoundOrName>,
    pub doc_cmyk: Option<FoundOrName>,
    pub doc_gray: Option<FoundOrName>,
    pub doc_primary_rgb: bool,
    pub doc_intent: String,
    /// The defaults being edited.
    pub defaults: ColorPrefs,
    /// The profiles found on this computer.
    pub found: Vec<FoundProfile>,
    /// A preset name being typed after the save button.
    pub new_preset: Option<String>,
}

impl ColorManagementState {
    fn load(&mut self, app: &App) {
        let md = &app.doc().metadata;
        let c = &app.settings.color;
        let name_or = |s: &str, d: &str| {
            if s.trim().is_empty() {
                d.to_string()
            } else {
                s.to_string()
            }
        };
        self.found = crate::color_policy::system_profiles();
        let pick = |name: String, space: Space, found: &[FoundProfile]| {
            found
                .iter()
                .find(|p| p.space == space && p.name == name)
                .cloned()
                .map(FoundOrName::Found)
                .unwrap_or(FoundOrName::Name(name))
        };
        let cmyk_name = app
            .doc()
            .cmyk_profile
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| name_or(&md.cmyk_profile, &c.cmyk_profile));
        self.doc_rgb = Some(pick(
            name_or(&md.rgb_profile, &c.rgb_profile),
            Space::Rgb,
            &self.found,
        ));
        self.doc_cmyk = Some(pick(cmyk_name, Space::Cmyk, &self.found));
        self.doc_gray = Some(pick(
            name_or(&md.gray_profile, &c.gray_profile),
            Space::Gray,
            &self.found,
        ));
        self.doc_primary_rgb =
            md.primary_color_mode == tracedraw_core::document::PrimaryColorMode::Rgb;
        self.doc_intent = name_or(&md.rendering_intent, &c.intent);
        self.defaults = c.clone();
        self.loaded = true;
    }
}

/// Classic tabs: one button per page, the chosen one joined to the body.
pub(crate) fn tab_header(ui: &mut Ui, tab: &mut usize, labels: &[String]) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (i, l) in labels.iter().enumerate() {
            let on = *tab == i;
            let text = RichText::new(l.as_str()).size(13.0);
            let b = egui::Button::new(text)
                .fill(if on { Tokens::field() } else { Tokens::panel() })
                .stroke(egui::Stroke::new(1.0, Tokens::border()))
                .corner_radius(0.0)
                .min_size(egui::vec2(80.0, 26.0));
            if ui
                .add(b)
                .rich_tip_with(|| format!("{l}\n{}", tr("dialog.tab_desc")))
                .clicked()
            {
                *tab = i;
            }
        }
    });
}

/// The tip of the combo `id`: what it sets.
fn combo_tip(id: &str) -> String {
    let (title, desc) = match id {
        "cm_doc_rgb" | "cm_def_rgb" => ("color_mgmt.rgb_profile", "color_mgmt.rgb_profile_desc"),
        "cm_doc_cmyk" | "cm_def_cmyk" => {
            ("color_mgmt.cmyk_profile", "color_mgmt.cmyk_profile_desc")
        }
        "cm_doc_gray" | "cm_def_gray" => {
            ("color_mgmt.gray_profile", "color_mgmt.gray_profile_desc")
        }
        "cm_doc_mode" | "cm_def_mode" => {
            ("color_mgmt.primary_mode", "color_mgmt.primary_mode_desc")
        }
        "cm_doc_intent" | "cm_def_intent" => ("color_mgmt.intent", "color_mgmt.intent_desc"),
        "cm_open_rgb" | "cm_open_cmyk" | "cm_open_gray" => {
            ("color_mgmt.open_policy", "color_mgmt.open_policy_desc")
        }
        "cm_imp_rgb" | "cm_imp_cmyk" | "cm_imp_gray" => {
            ("color_mgmt.import_policy", "color_mgmt.import_policy_desc")
        }
        _ => (
            "color_mgmt.spot_definition",
            "color_mgmt.spot_definition_desc",
        ),
    };
    crate::ui::tips::pair(title, desc)
}

/// A titled divider: the title, then a line to the right edge.
fn section(ui: &mut Ui, title: &str) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(title);
        let r = ui.available_rect_before_wrap();
        let y = r.center().y;
        ui.painter().line_segment(
            [egui::pos2(r.left() + 6.0, y), egui::pos2(r.right(), y)],
            egui::Stroke::new(1.0, Tokens::border()),
        );
    });
}

/// The page under a dialog's tabs: the colour of fields in the theme
/// (white in the light ones), so its labels read in every theme (#904).
pub(crate) fn tab_page() -> egui::Frame {
    let (fill, line) = crate::theme::theme().dialog_page();
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, line))
}

/// A profile combo: the default one first under "Default profiles", then
/// "Load color profiles..." and every profile of `space` found here under
/// "Other profiles". Returns true when "Load..." was chosen.
fn profile_combo(
    ui: &mut Ui,
    id: &str,
    space: Space,
    default_name: &str,
    found: &[FoundProfile],
    choice: &mut Option<FoundOrName>,
) -> bool {
    let mut load = false;
    let shown = choice
        .as_ref()
        .map(|c| c.name().to_string())
        .unwrap_or_default();
    let label = if shown == default_name {
        trf("color_mgmt.default_profile", &[("name", &shown)])
    } else {
        shown.clone()
    };
    egui::ComboBox::from_id_salt(id)
        .width(360.0)
        .selected_text(label)
        .show_ui(ui, |ui| {
            ui.label(RichText::new(tr("color_mgmt.default_profiles")).color(Tokens::ACCENT));
            let def = found
                .iter()
                .find(|p| p.space == space && p.name == default_name)
                .cloned()
                .map(FoundOrName::Found)
                .unwrap_or(FoundOrName::Name(default_name.to_string()));
            if ui
                .selectable_label(
                    shown == default_name,
                    trf("color_mgmt.default_profile", &[("name", default_name)]),
                )
                .clicked()
            {
                *choice = Some(def);
            }
            ui.label(RichText::new(tr("color_mgmt.other_profiles")).color(Tokens::ACCENT));
            if ui
                .selectable_label(false, tr("color_mgmt.load_profiles"))
                .clicked()
            {
                load = true;
            }
            for p in found.iter().filter(|p| p.space == space) {
                if ui
                    .selectable_label(shown == p.name, p.name.as_str())
                    .rich_tip(p.path.as_str())
                    .clicked()
                {
                    *choice = Some(FoundOrName::Found(p.clone()));
                }
            }
        })
        .response
        .rich_tip(combo_tip(id));
    load
}

fn combo_u8(ui: &mut Ui, id: &str, v: &mut u8, keys: &[&str]) {
    egui::ComboBox::from_id_salt(id)
        .width(360.0)
        .selected_text(tr(keys[(*v as usize).min(keys.len() - 1)]))
        .show_ui(ui, |ui| {
            for (i, k) in keys.iter().enumerate() {
                ui.selectable_value(v, i as u8, tr(k));
            }
        })
        .response
        .rich_tip(combo_tip(id));
}

fn intent_combo(ui: &mut Ui, id: &str, v: &mut String) {
    egui::ComboBox::from_id_salt(id)
        .width(360.0)
        .selected_text(intent_label(v))
        .show_ui(ui, |ui| {
            for i in INTENTS {
                if ui.selectable_label(v == i, intent_label(i)).clicked() {
                    *v = i.to_string();
                }
            }
        })
        .response
        .rich_tip(combo_tip(id));
}

fn intent_label(name: &str) -> String {
    match name {
        "Absolute colorimetric" => tr("color_mgmt.intent_absolute"),
        "Perceptual" => tr("color_mgmt.intent_perceptual"),
        "Saturation" => tr("color_mgmt.intent_saturation"),
        _ => tr("color_mgmt.intent_relative"),
    }
}

fn mode_combo(ui: &mut Ui, id: &str, rgb: &mut bool) {
    egui::ComboBox::from_id_salt(id)
        .width(360.0)
        .selected_text(if *rgb { "RGB" } else { "CMYK" })
        .show_ui(ui, |ui| {
            ui.selectable_value(rgb, false, "CMYK");
            ui.selectable_value(rgb, true, "RGB");
        })
        .response
        .rich_tip(combo_tip(id));
}

/// Right-aligned labels in a two-column grid.
fn row(ui: &mut Ui, label: &str) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(label);
    });
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut ColorManagementState, close: &mut bool) {
    if !st.loaded {
        st.load(app);
    }
    let mut ok = false;
    let mut help = false;
    let mut load: Option<Space> = None;
    let body = egui::vec2(660.0, 500.0);
    let size = egui::vec2(body.x + 30.0, body.y + 110.0);
    let closed = crate::ui::chrome::dialog(
        ctx,
        "color_management_settings",
        &tr("color_mgmt.title"),
        size,
        |ui| {
            let area = ui.max_rect();
            let inner = egui::Rect::from_min_max(
                area.min + egui::vec2(15.0, 12.0),
                egui::pos2(area.max.x - 15.0, area.max.y - 56.0),
            );
            // The page under the tabs fills what the tabs leave.
            let page_h = inner.height() - 26.0 - 24.0;
            ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
                tab_header(
                    ui,
                    &mut st.tab,
                    &[tr("color_mgmt.tab_document"), tr("color_mgmt.tab_default")],
                );
                tab_page()
                    .inner_margin(egui::Margin::symmetric(40, 12))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(inner.width() - 82.0, page_h));
                        ui.set_max_height(page_h);
                        if st.tab == 0 {
                            document_tab(app, ui, st, &mut load);
                        } else {
                            egui::ScrollArea::vertical()
                                .max_height(page_h)
                                .show(ui, |ui| default_tab(ui, st, &mut load));
                        }
                    });
            });
            let buttons = egui::Rect::from_min_size(
                egui::pos2(area.min.x + 15.0, area.max.y - 42.0),
                egui::vec2(area.width() - 30.0, 27.0),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(buttons), |ui| {
                ui.horizontal(|ui| {
                    help = ui
                        .add_sized([27.0, 27.0], egui::Button::new("?"))
                        .rich_tip(crate::ui::tips::keyed("dialog.help_button"))
                        .clicked();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_sized([100.0, 27.0], egui::Button::new(tr("dialog.cancel")))
                            .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                            .clicked()
                        {
                            *close = true;
                        }
                        if ui
                            .add_sized([100.0, 27.0], egui::Button::new(tr("dialog.ok")))
                            .rich_tip(crate::ui::tips::keyed("dialog.ok"))
                            .clicked()
                            || crate::ui::dialogs::enter_pressed(ui)
                        {
                            ok = true;
                            *close = true;
                        }
                    });
                });
            });
        },
    );
    if closed {
        *close = true;
    }
    if help {
        app.open_dialog_help();
    }
    if let Some(space) = load {
        load_profile_file(app, space);
        st.loaded = false;
    }
    if ok {
        apply(app, st);
    }
}

fn document_tab(app: &App, ui: &mut Ui, st: &mut ColorManagementState, load: &mut Option<Space>) {
    let md = &app.doc().metadata;
    let c = &app.settings.color;
    let current = |s: &str, d: &str| {
        if s.trim().is_empty() {
            d.to_string()
        } else {
            s.to_string()
        }
    };
    let cmyk_now = app
        .doc()
        .cmyk_profile
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| current(&md.cmyk_profile, &c.cmyk_profile));
    section(ui, &tr("color_mgmt.current_profiles"));
    egui::Grid::new("cm_current").num_columns(2).show(ui, |ui| {
        row(ui, "RGB:");
        ui.label(current(&md.rgb_profile, &c.rgb_profile));
        ui.end_row();
        row(ui, "CMYK:");
        ui.label(cmyk_now);
        ui.end_row();
        row(ui, &tr("color_mgmt.grayscale"));
        ui.label(current(&md.gray_profile, &c.gray_profile));
        ui.end_row();
    });
    ui.separator();
    ui.vertical_centered(|ui| {
        ui.radio_value(&mut st.convert, false, tr("color_mgmt.assign"))
            .rich_tip(crate::ui::tips::keyed("color_mgmt.assign"));
        ui.radio_value(&mut st.convert, true, tr("color_mgmt.convert"))
            .rich_tip(crate::ui::tips::keyed("color_mgmt.convert"));
    });
    egui::Grid::new("cm_doc")
        .num_columns(2)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            row(ui, "RGB:");
            if profile_combo(
                ui,
                "cm_doc_rgb",
                Space::Rgb,
                &c.rgb_profile,
                &st.found,
                &mut st.doc_rgb,
            ) {
                *load = Some(Space::Rgb);
            }
            ui.end_row();
            row(ui, "CMYK:");
            if profile_combo(
                ui,
                "cm_doc_cmyk",
                Space::Cmyk,
                &c.cmyk_profile,
                &st.found,
                &mut st.doc_cmyk,
            ) {
                *load = Some(Space::Cmyk);
            }
            ui.end_row();
            row(ui, &tr("color_mgmt.grayscale"));
            if profile_combo(
                ui,
                "cm_doc_gray",
                Space::Gray,
                &c.gray_profile,
                &st.found,
                &mut st.doc_gray,
            ) {
                *load = Some(Space::Gray);
            }
            ui.end_row();
        });
    ui.separator();
    egui::Grid::new("cm_doc2")
        .num_columns(2)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            row(ui, &tr("color_mgmt.primary_mode"));
            mode_combo(ui, "cm_doc_mode", &mut st.doc_primary_rgb);
            ui.end_row();
            row(ui, &tr("color_mgmt.intent"));
            intent_combo(ui, "cm_doc_intent", &mut st.doc_intent);
            ui.end_row();
        });
}

fn default_tab(ui: &mut Ui, st: &mut ColorManagementState, load: &mut Option<Space>) {
    let found = st.found.clone();
    let d = &mut st.defaults;
    ui.horizontal(|ui| {
        ui.label(tr("color_mgmt.preset"));
        let custom: Vec<String> = d.presets.iter().map(|p| p.name.clone()).collect();
        let shown = if d.preset.is_empty() {
            tr("color_mgmt.preset_custom")
        } else {
            preset_label(&d.preset)
        };
        egui::ComboBox::from_id_salt("cm_preset")
            .width(300.0)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for p in PRESETS {
                    if ui.selectable_label(d.preset == p.key, tr(p.key)).clicked() {
                        d.preset = p.key.to_string();
                        set_names(d, p.rgb, p.cmyk, p.gray, &found);
                    }
                }
                for name in &custom {
                    if ui
                        .selectable_label(&d.preset == name, name.as_str())
                        .clicked()
                    {
                        if let Some(p) = d.presets.iter().find(|p| &p.name == name).cloned() {
                            d.preset = p.name.clone();
                            d.rgb_profile = p.rgb_profile;
                            d.rgb_profile_path = p.rgb_profile_path;
                            d.cmyk_profile = p.cmyk_profile;
                            d.cmyk_profile_path = p.cmyk_profile_path;
                            d.gray_profile = p.gray_profile;
                            d.gray_profile_path = p.gray_profile_path;
                            d.primary_color_mode = p.primary_color_mode;
                            d.intent = p.intent;
                        }
                    }
                }
            })
            .response
            .rich_tip(crate::ui::tips::keyed("color_mgmt.preset"));
        if ui
            .button("\u{1F4BE}")
            .rich_tip(crate::ui::tips::keyed("color_mgmt.save_preset"))
            .clicked()
        {
            st.new_preset = Some(String::new());
        }
        let user = d.presets.iter().any(|p| p.name == d.preset);
        if ui
            .add_enabled(user, egui::Button::new("\u{1F5D1}"))
            .rich_tip_always(crate::ui::tips::keyed("color_mgmt.delete_preset"))
            .clicked()
        {
            let name = d.preset.clone();
            d.presets.retain(|p| p.name != name);
            d.preset.clear();
        }
    });
    if let Some(name) = &mut st.new_preset {
        let mut done = false;
        ui.horizontal(|ui| {
            ui.label(tr("color_mgmt.preset_name"));
            ui.text_edit_singleline(name)
                .rich_tip(crate::ui::tips::keyed("color_mgmt.preset_name"));
            if ui
                .button(tr("dialog.ok"))
                .rich_tip(crate::ui::tips::keyed("dialog.ok"))
                .clicked()
                && !name.trim().is_empty()
            {
                done = true;
            }
        });
        if done {
            let name = name.trim().to_string();
            d.presets.retain(|p| p.name != name);
            d.presets.push(ColorPreset {
                name: name.clone(),
                rgb_profile: d.rgb_profile.clone(),
                rgb_profile_path: d.rgb_profile_path.clone(),
                cmyk_profile: d.cmyk_profile.clone(),
                cmyk_profile_path: d.cmyk_profile_path.clone(),
                gray_profile: d.gray_profile.clone(),
                gray_profile_path: d.gray_profile_path.clone(),
                primary_color_mode: d.primary_color_mode.clone(),
                intent: d.intent.clone(),
            });
            d.preset = name;
            st.new_preset = None;
        }
    }
    ui.separator();
    egui::Grid::new("cm_def")
        .num_columns(2)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            for (space, key, id) in [
                (Space::Rgb, "color_mgmt.rgb_profile", "cm_def_rgb"),
                (Space::Cmyk, "color_mgmt.cmyk_profile", "cm_def_cmyk"),
                (Space::Gray, "color_mgmt.gray_profile", "cm_def_gray"),
            ] {
                row(ui, &tr(key));
                let (name, path) = match space {
                    Space::Rgb => (d.rgb_profile.clone(), d.rgb_profile_path.clone()),
                    Space::Cmyk => (d.cmyk_profile.clone(), d.cmyk_profile_path.clone()),
                    Space::Gray => (d.gray_profile.clone(), d.gray_profile_path.clone()),
                };
                let mut choice = Some(if path.is_empty() {
                    FoundOrName::Name(name.clone())
                } else {
                    FoundOrName::Found(FoundProfile {
                        name: name.clone(),
                        path,
                        space,
                    })
                });
                let before = choice.clone();
                if profile_combo(ui, id, space, &name, &found, &mut choice) {
                    *load = Some(space);
                }
                if choice != before {
                    if let Some(c) = choice {
                        let (n, p) = (c.name().to_string(), c.path().unwrap_or("").to_string());
                        match space {
                            Space::Rgb => (d.rgb_profile, d.rgb_profile_path) = (n, p),
                            Space::Cmyk => (d.cmyk_profile, d.cmyk_profile_path) = (n, p),
                            Space::Gray => (d.gray_profile, d.gray_profile_path) = (n, p),
                        }
                        d.preset.clear();
                    }
                }
                ui.end_row();
            }
            row(ui, &tr("color_mgmt.primary_mode"));
            let mut rgb = d.primary_color_mode == "RGB";
            mode_combo(ui, "cm_def_mode", &mut rgb);
            d.primary_color_mode = if rgb { "RGB" } else { "CMYK" }.into();
            ui.end_row();
            row(ui, &tr("color_mgmt.intent"));
            intent_combo(ui, "cm_def_intent", &mut d.intent);
            ui.end_row();
        });
    section(ui, &tr("color_mgmt.conversion"));
    egui::Grid::new("cm_conv")
        .num_columns(2)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            row(ui, &tr("color_mgmt.engine"));
            egui::ComboBox::from_id_salt("cm_engine")
                .width(360.0)
                .selected_text(if d.builtin_engine {
                    tr("color_mgmt.engine_builtin")
                } else {
                    tr("color_mgmt.engine_icc")
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut d.builtin_engine, false, tr("color_mgmt.engine_icc"));
                    ui.selectable_value(
                        &mut d.builtin_engine,
                        true,
                        tr("color_mgmt.engine_builtin"),
                    );
                })
                .response
                .rich_tip(crate::ui::tips::keyed("color_mgmt.engine"));
            ui.end_row();
            ui.label("");
            ui.vertical(|ui| {
                ui.checkbox(&mut d.preserve_pure_black, tr("color_mgmt.preserve_black"))
                    .rich_tip(crate::ui::tips::keyed("color_mgmt.preserve_black"));
                ui.checkbox(&mut d.map_gray_to_black, tr("color_mgmt.map_gray"))
                    .rich_tip(crate::ui::tips::keyed("color_mgmt.map_gray"));
                ui.checkbox(&mut d.black_point_compensation, tr("dialog.black_point"))
                    .rich_tip(crate::ui::tips::pair(
                        "dialog.black_point",
                        "color_mgmt.black_point_desc",
                    ));
            });
            ui.end_row();
        });
    let open_keys = [
        "color_mgmt.open_use_embedded",
        "color_mgmt.open_assign_default",
        "color_mgmt.open_convert_default",
    ];
    section(ui, &tr("color_mgmt.open_policy"));
    egui::Grid::new("cm_open")
        .num_columns(2)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            row(ui, "RGB:");
            combo_u8(ui, "cm_open_rgb", &mut d.open_rgb, &open_keys);
            ui.end_row();
            row(ui, "CMYK:");
            combo_u8(ui, "cm_open_cmyk", &mut d.open_cmyk, &open_keys);
            ui.end_row();
            row(ui, &tr("color_mgmt.grayscale"));
            combo_u8(ui, "cm_open_gray", &mut d.open_gray, &open_keys);
            ui.end_row();
            ui.label("");
            ui.vertical(|ui| {
                ui.checkbox(
                    &mut d.warn_open_mismatch,
                    tr("color_mgmt.warn_mismatch_box"),
                )
                .rich_tip(crate::ui::tips::keyed("color_mgmt.warn_mismatch_box"));
                ui.checkbox(&mut d.warn_open_missing, tr("color_mgmt.warn_missing_box"))
                    .rich_tip(crate::ui::tips::keyed("color_mgmt.warn_missing_box"));
            });
            ui.end_row();
        });
    let import_keys = ["color_mgmt.import_convert", "color_mgmt.import_assign"];
    section(ui, &tr("color_mgmt.import_policy"));
    egui::Grid::new("cm_import")
        .num_columns(2)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            row(ui, "RGB:");
            combo_u8(ui, "cm_imp_rgb", &mut d.import_rgb, &import_keys);
            ui.end_row();
            row(ui, "CMYK:");
            combo_u8(ui, "cm_imp_cmyk", &mut d.import_cmyk, &import_keys);
            ui.end_row();
            row(ui, &tr("color_mgmt.grayscale"));
            combo_u8(ui, "cm_imp_gray", &mut d.import_gray, &import_keys);
            ui.end_row();
            ui.label("");
            ui.vertical(|ui| {
                ui.checkbox(
                    &mut d.warn_import_mismatch,
                    tr("color_mgmt.warn_mismatch_box"),
                )
                .rich_tip(crate::ui::tips::keyed("color_mgmt.warn_mismatch_box"));
                ui.checkbox(
                    &mut d.warn_import_missing,
                    tr("color_mgmt.warn_missing_box"),
                )
                .rich_tip(crate::ui::tips::keyed("color_mgmt.warn_missing_box"));
            });
            ui.end_row();
        });
    ui.separator();
    egui::Grid::new("cm_spot")
        .num_columns(2)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            row(ui, &tr("color_mgmt.spot_definition"));
            combo_u8(
                ui,
                "cm_spot",
                &mut d.spot_definition,
                &[
                    "color_mgmt.spot_lab",
                    "color_mgmt.spot_cmyk",
                    "color_mgmt.spot_rgb",
                ],
            );
            ui.end_row();
        });
}

fn preset_label(key: &str) -> String {
    if key.starts_with("color_mgmt.") {
        tr(key)
    } else {
        key.to_string()
    }
}

/// The profiles of a preset, with their files where found here.
fn set_names(d: &mut ColorPrefs, rgb: &str, cmyk: &str, gray: &str, found: &[FoundProfile]) {
    let path = |name: &str, space: Space| {
        found
            .iter()
            .find(|p| p.space == space && p.name == name)
            .map(|p| p.path.clone())
            .unwrap_or_default()
    };
    d.rgb_profile = rgb.into();
    d.rgb_profile_path = path(rgb, Space::Rgb);
    d.cmyk_profile = cmyk.into();
    d.cmyk_profile_path = path(cmyk, Space::Cmyk);
    d.gray_profile = gray.into();
    d.gray_profile_path = path(gray, Space::Gray);
}

/// "Load color profiles...": an ICC file becomes the default profile of
/// its colour space (the dialog then lists it).
fn load_profile_file(app: &mut App, space: Space) {
    match space {
        Space::Rgb => app.load_icc_profile(false),
        Space::Cmyk => app.load_icc_profile(true),
        Space::Gray => app.load_gray_profile(),
    }
}

/// OK: the defaults become the settings, then the drawing takes its
/// profiles (assigned or converted), primary mode and intent.
fn apply(app: &mut App, st: &ColorManagementState) {
    app.settings.color = st.defaults.clone();
    app.save_settings();
    app.apply_color_settings();
    let (Some(rgb), Some(cmyk), Some(gray)) = (&st.doc_rgb, &st.doc_cmyk, &st.doc_gray) else {
        return;
    };
    let md = app.doc().metadata.clone();
    let carried = app.doc().cmyk_profile.as_ref().map(|p| p.name.clone());
    let cmyk_now = carried.unwrap_or_else(|| md.cmyk_profile.clone());
    let changed =
        rgb.name() != md.rgb_profile || cmyk.name() != cmyk_now || gray.name() != md.gray_profile;
    if changed {
        app.set_document_profiles(rgb, cmyk, gray, st.convert);
    }
    let mut md = app.doc().metadata.clone();
    let mode = if st.doc_primary_rgb {
        tracedraw_core::document::PrimaryColorMode::Rgb
    } else {
        tracedraw_core::document::PrimaryColorMode::Cmyk
    };
    if md.primary_color_mode != mode || md.rendering_intent != st.doc_intent {
        md.primary_color_mode = mode;
        md.rendering_intent = st.doc_intent.clone();
        app.run(tracedraw_core::Command::SetMetadata { metadata: md });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dialogs::Dialog;

    fn frame(ctx: &Context, app: &mut App, enter: bool) {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..Default::default()
        };
        if enter {
            input.events.push(egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
        let mut out = ctx.run_ui(input, |ui| crate::ui::dialogs::show(app, ui.ctx()));
        out.textures_delta.clear();
    }

    #[test]
    fn ok_sets_the_defaults_and_the_drawings_profiles_and_mode() {
        let mut app = App::headless();
        let mut st = ColorManagementState::default();
        st.load(&app);
        st.defaults.preserve_pure_black = true;
        st.defaults.open_cmyk = 1;
        st.doc_gray = Some(FoundOrName::Name("Dot Gain 20%".into()));
        st.doc_primary_rgb = true;
        st.doc_intent = "Perceptual".into();
        apply(&mut app, &st);
        assert!(app.settings.color.preserve_pure_black);
        assert_eq!(app.settings.color.open_cmyk, 1);
        let md = &app.doc().metadata;
        assert_eq!(md.gray_profile, "Dot Gain 20%");
        assert_eq!(md.rendering_intent, "Perceptual");
        assert_eq!(
            md.primary_color_mode,
            tracedraw_core::document::PrimaryColorMode::Rgb
        );
        // Both tabs draw, and Enter is OK.
        for tab in [0, 1] {
            app.dialog = Dialog::ColorManagement(Box::new(ColorManagementState {
                tab,
                ..Default::default()
            }));
            let ctx = crate::theme::ui_context();
            frame(&ctx, &mut app, false);
            frame(&ctx, &mut app, true);
            assert!(matches!(app.dialog, Dialog::None));
        }
    }

    #[test]
    fn presets_fill_the_three_default_profiles() {
        let mut d = ColorPrefs::default();
        set_names(
            &mut d,
            PRESETS[1].rgb,
            PRESETS[1].cmyk,
            PRESETS[1].gray,
            &[],
        );
        assert_eq!(d.cmyk_profile, PRESETS[1].cmyk);
        assert_eq!(d.gray_profile, PRESETS[1].gray);
        assert!(d.cmyk_profile_path.is_empty());
    }
}
