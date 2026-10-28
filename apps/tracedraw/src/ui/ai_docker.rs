//! The AI Drawing docker: the connection to the AI service (service,
//! address, model, key), the request in words, Draw, and what came back.
//! The work is in `ai.rs`; see docs/behavior/ai-drawing.md.

use crate::ai::{AiStatus, Provider};
use crate::app::{App, DockerTab};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::options::{combo, grid, row_label};
use crate::ui::tips::RichTip;
use egui::{RichText, Ui};

const ERROR: egui::Color32 = egui::Color32::from_rgb(0xC4, 0x2B, 0x1C);
/// Width of the connection fields.
const FIELD_W: f32 = 170.0;

fn dim(ui: &mut Ui, text: String) {
    ui.label(RichText::new(text).color(Tokens::text_dim()).size(11.0));
}

pub fn ai_drawing(app: &mut App, ui: &mut Ui) {
    if !app.ai_features_on() {
        crate::ui::ai_ui::banner(app, ui, crate::ui::ai_ui::Need::Drawing);
    }
    // The connection, open until a key is set where one is needed.
    let provider = app.settings.ai.provider;
    let needs_setup = provider.needs_key() && app.ai.key.trim().is_empty();
    egui::CollapsingHeader::new(tr("ai.connection"))
        .id_salt("ai_connection")
        .default_open(needs_setup)
        .show(ui, |ui| connection(app, ui));
    ui.add_space(4.0);
    ui.label(tr("ai.request"));
    let field = ui.add(
        egui::TextEdit::multiline(&mut app.ai.request)
            .id(egui::Id::new("ai_request"))
            .hint_text(tr("ai.request_hint"))
            .desired_rows(4)
            .desired_width(f32::INFINITY),
    );
    // Ctrl+Enter in the request draws.
    let ctrl_enter =
        field.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command);
    ui.horizontal(|ui| {
        if app.ai.busy() {
            ui.spinner();
            if ui
                .button(tr("dialog.cancel"))
                .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                .clicked()
            {
                app.ai_cancel();
            }
        } else if ui
            .add_enabled(app.ai_features_on(), egui::Button::new(tr("ai.draw")))
            .clicked()
            || (ctrl_enter && app.ai_features_on())
        {
            app.ai_draw();
        }
        if ui
            .add_enabled(
                !app.ai.history.is_empty(),
                egui::Button::new(tr("ai.new_conversation")),
            )
            .clicked()
        {
            app.ai_new_conversation();
        }
    });
    match &app.ai.status {
        AiStatus::Idle => {}
        AiStatus::Waiting => dim(ui, tr("ai.waiting")),
        AiStatus::Retrying => dim(ui, tr("ai.retrying")),
        AiStatus::Done(n) => {
            ui.label(trf("ai.done", &[("n", &n.to_string())]));
        }
        AiStatus::Listed(n) => dim(ui, trf("ai.models_found", &[("n", &n.to_string())])),
        AiStatus::Failed(e) => {
            ui.label(RichText::new(e.as_str()).color(ERROR));
        }
    }
    for line in app.ai.output.clone() {
        dim(ui, line);
    }
    if !app.ai.script.is_empty() {
        egui::CollapsingHeader::new(tr("ai.script"))
            .id_salt("ai_script")
            .show(ui, |ui| {
                let mut shown = app.ai.script.as_str();
                ui.add(
                    egui::TextEdit::multiline(&mut shown)
                        .code_editor()
                        .desired_rows(6)
                        .desired_width(f32::INFINITY),
                );
                if ui.button(tr("ai.open_in_scripts")).clicked() {
                    app.script_source = app.ai.script.clone();
                    app.open_docker(DockerTab::Scripts);
                }
            });
    }
    ui.add_space(6.0);
    ui.separator();
    dim(ui, tr("ai.privacy"));
}

/// Service, address, model and key, and whether the key is remembered.
pub fn connection(app: &mut App, ui: &mut Ui) {
    // The models to pick from: the service's own once listed, else the
    // usual ones of its kind.
    let choices = app.ai_model_choices();
    let mut fetch = false;
    let prefs = &mut app.settings.ai;
    grid("ai_connection_grid").show(ui, |ui| {
        row_label(ui, &tr("aipreset.label"));
        let current = crate::ai::preset_of(&*prefs);
        let shown = current
            .and_then(|i| crate::ai::PRESETS.get(i))
            .map(|(k, _, _)| tr(k))
            .unwrap_or_else(|| tr("aipreset.custom"));
        egui::ComboBox::from_id_salt("ai_preset")
            .width(FIELD_W)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for (i, (key, provider, endpoint)) in crate::ai::PRESETS.iter().enumerate() {
                    if ui.selectable_label(current == Some(i), tr(key)).clicked() {
                        prefs.provider = *provider;
                        prefs.endpoint = endpoint.to_string();
                        prefs.model.clear();
                        prefs.image_model.clear();
                        app.ai.models.clear();
                        app.ai_image.models.clear();
                    }
                }
            })
            .response
            .rich_tip(tr("aipreset.tip"));
        ui.end_row();
        row_label(ui, &tr("ai.provider"));
        let mut provider = prefs.provider;
        let tip = crate::ui::tips::pair("ai.provider", "ai.provider_tip");
        if crate::ui::tips::tip_over(ui, tip, |ui| {
            combo(
                ui,
                "ai_provider",
                &mut provider,
                &Provider::ALL,
                |p| tr(p.key()),
                FIELD_W,
            )
        }) {
            // Another kind of service: its own address and model.
            prefs.provider = provider;
            prefs.endpoint.clear();
            prefs.model.clear();
            prefs.image_model.clear();
            app.ai_image.models.clear();
        }
        ui.end_row();
        row_label(ui, &tr("ai.endpoint"));
        ui.add(
            egui::TextEdit::singleline(&mut prefs.endpoint)
                .hint_text(prefs.provider.default_endpoint())
                .desired_width(FIELD_W),
        )
        .rich_tip(crate::ui::tips::pair("ai.endpoint", "ai.endpoint_tip"));
        ui.end_row();
        row_label(ui, &tr("ai.model"));
        let model_hint = match prefs.provider.default_model() {
            "" => tr("ai.model_hint"),
            m => m.to_string(),
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.add(
                egui::TextEdit::singleline(&mut prefs.model)
                    .hint_text(model_hint)
                    .desired_width(FIELD_W - 26.0),
            )
            .rich_tip(crate::ui::tips::pair("ai.model", "ai.model_tip"));
            ui.menu_button("\u{25BE}", |ui| {
                for m in &choices {
                    if ui.selectable_label(prefs.model == *m, m).clicked() {
                        prefs.model = m.clone();
                        ui.close();
                    }
                }
                if !choices.is_empty() {
                    ui.separator();
                }
                if ui.button(tr("ai.refresh_models")).clicked() {
                    fetch = true;
                    ui.close();
                }
            })
            .response
            .rich_tip(crate::ui::tips::pair("ai.model", "ai.model_list_tip"));
        });
        ui.end_row();
        row_label(ui, &tr("ai.key"));
        let key = ui
            .add(
                egui::TextEdit::singleline(&mut app.ai.key)
                    .password(true)
                    .desired_width(FIELD_W),
            )
            .rich_tip(crate::ui::tips::pair("ai.key", "ai.key_tip"));
        if key.changed() && prefs.remember_key {
            prefs.key = app.ai.key.clone();
        }
        ui.end_row();
        // Pictures: only for a service that can make them.
        if prefs.can_make_pictures() {
            row_label(ui, &tr("ai.image_model"));
            image_model_field(prefs, &app.ai_image.models, ui);
            ui.end_row();
        }
    });
    if !prefs.can_make_pictures() {
        dim(ui, tr("aiui.drawings_only"));
    }
    ui.checkbox(&mut prefs.show_bar, tr("ai.show_bar"))
        .rich_tip(crate::ui::tips::pair("ai.show_bar", "ai.show_bar_tip"));
    if ui
        .checkbox(&mut prefs.remember_key, tr("ai.remember_key"))
        .rich_tip(crate::ui::tips::pair(
            "ai.remember_key",
            "ai.remember_key_tip",
        ))
        .changed()
    {
        prefs.key = if prefs.remember_key {
            app.ai.key.clone()
        } else {
            String::new()
        };
        app.settings.save();
    }
    dim(ui, tr("ai.local_hint"));
    if fetch {
        app.ai_fetch_models();
    }
}

/// The Image model field: typed, or picked from the service's image
/// models (once tested) or the usual ones.
fn image_model_field(prefs: &mut crate::ai::AiPrefs, listed: &[String], ui: &mut Ui) {
    use crate::ai::ImageSupport;
    use crate::ai_image::ImageProvider;
    let support = prefs.image_support();
    let (hint, usual): (String, &[&str]) = match support {
        ImageSupport::OpenAi => (
            ImageProvider::OpenAi.default_model().to_string(),
            ImageProvider::OpenAi.preset_models(),
        ),
        ImageSupport::Gemini => (
            ImageProvider::Gemini.default_model().to_string(),
            ImageProvider::Gemini.preset_models(),
        ),
        _ => (tr("ai.image_model_none"), &[]),
    };
    let choices: Vec<String> = if listed.is_empty() {
        usual.iter().map(|m| m.to_string()).collect()
    } else {
        listed.to_vec()
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.add(
            egui::TextEdit::singleline(&mut prefs.image_model)
                .hint_text(hint)
                .desired_width(FIELD_W - 26.0),
        )
        .rich_tip(tr("ai.image_model_tip"));
        ui.add_enabled_ui(!choices.is_empty(), |ui| {
            ui.menu_button("\u{25BE}", |ui| {
                for m in &choices {
                    if ui.selectable_label(prefs.image_model == *m, m).clicked() {
                        prefs.image_model = m.clone();
                        ui.close();
                    }
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{vec2, Pos2, Rect};

    fn frame(ctx: &egui::Context, app: &mut App) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| ai_drawing(app, ui));
        out.textures_delta.clear();
    }

    #[test]
    fn the_docker_draws_in_every_state() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        frame(&ctx, &mut app);
        app.settings.ai.provider = Provider::OpenAiCompatible;
        app.ai.status = AiStatus::Waiting;
        frame(&ctx, &mut app);
        app.ai.status = AiStatus::Failed("no".into());
        app.ai.script = "print(1)".into();
        app.ai.output = vec!["1".into()];
        frame(&ctx, &mut app);
        app.ai.status = AiStatus::Done(3);
        frame(&ctx, &mut app);
        assert_eq!(app.ai.script, "print(1)", "the script is shown, not edited");
    }

    #[test]
    fn the_key_is_kept_only_when_asked() {
        let mut app = App::headless();
        app.ai.key = "secret".into();
        assert!(app.settings.ai.key.is_empty());
        let json = serde_json::to_string(&app.settings).expect("settings");
        assert!(!json.contains("secret"));
        // Remembered: it goes with the preferences, and comes back.
        app.settings.ai.remember_key = true;
        app.settings.ai.key = app.ai.key.clone();
        let json = serde_json::to_string(&app.settings).expect("settings");
        assert!(json.contains("secret"));
        let back: crate::settings::Settings = serde_json::from_str(&json).expect("read");
        assert_eq!(crate::ai::remembered_key(&back.ai), "secret");
        // A key in a file that does not say to remember it is not used.
        let mut odd = back.ai.clone();
        odd.remember_key = false;
        assert_eq!(crate::ai::remembered_key(&odd), "");
    }
}
