//! The AI prompt bar: one line above the colour palette, shown while an
//! AI service is set up for AI Drawing (and the bar is not turned off in
//! Window > Toolbars). It holds the model, the request (Enter draws),
//! Draw or Cancel, what the last request did, the panel with the settings
//! and a button that hides the bar. See docs/behavior/ai-drawing.md.

use crate::ai::AiStatus;
use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{RichText, Ui};

/// Height of the bar.
pub const BAR_H: f32 = 32.0;
const ERROR: egui::Color32 = egui::Color32::from_rgb(0xC4, 0x2B, 0x1C);
/// Width of the model list.
const MODEL_W: f32 = 170.0;
/// Room kept on the right of the request for the buttons and the state.
const RIGHT_W: f32 = 330.0;

/// The bar shows: a drawing is open, a service is set up and the bar is
/// not turned off.
pub fn visible(app: &App) -> bool {
    app.settings.ai.show_bar && app.has_document() && !app.show_welcome && app.ai_drawing_ready()
}

/// The state of the last request, short, and its colour.
fn state(app: &App) -> Option<(String, egui::Color32)> {
    match &app.ai.status {
        AiStatus::Idle => None,
        AiStatus::Waiting => Some((tr("ai.waiting"), Tokens::text_dim())),
        AiStatus::Retrying => Some((tr("ai.retrying"), Tokens::text_dim())),
        AiStatus::Done(n) => Some((trf("ai.done", &[("n", &n.to_string())]), Tokens::text_dim())),
        AiStatus::Listed(n) => Some((
            trf("ai.models_found", &[("n", &n.to_string())]),
            Tokens::text_dim(),
        )),
        AiStatus::Failed(e) => Some((e.clone(), ERROR)),
    }
}

pub fn ai_bar(app: &mut App, ui: &mut Ui) {
    ui.horizontal_centered(|ui| {
        ui.add_space(6.0);
        ui.label(RichText::new(tr("ai.bar_label")).strong());
        model_list(app, ui);
        let busy = app.ai.busy();
        let width = (ui.available_width() - RIGHT_W).max(120.0);
        let field = ui.add_enabled(
            !busy,
            egui::TextEdit::singleline(&mut app.ai.request)
                .id(egui::Id::new("ai_bar_request"))
                .hint_text(tr("ai.bar_hint"))
                .desired_width(width),
        );
        let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if busy {
            ui.spinner();
            if ui
                .button(tr("dialog.cancel"))
                .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                .clicked()
            {
                app.ai_cancel();
            }
        } else if ui.button(tr("ai.draw")).clicked() || enter {
            app.ai_draw();
            // Ready for the next request.
            field.request_focus();
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(4.0);
            if ui.small_button("x").rich_tip(tr("ai.hide_bar")).clicked() {
                app.settings.ai.show_bar = false;
            }
            if ui.small_button("...").rich_tip(tr("ai.settings")).clicked() {
                app.open_ai_drawing();
            }
            if let Some((text, colour)) = state(app) {
                ui.add(
                    egui::Label::new(RichText::new(text.clone()).color(colour).size(11.0))
                        .truncate(),
                )
                .rich_tip(text);
            }
        });
    });
}

/// The model list: the service's models once fetched (else the usual
/// ones), and an entry that fetches them.
fn model_list(app: &mut App, ui: &mut Ui) {
    let current = app.settings.ai.model();
    let choices = app.ai_model_choices();
    let mut fetch = false;
    egui::ComboBox::from_id_salt("ai_bar_model")
        .selected_text(RichText::new(current.clone()).size(12.0))
        .width(MODEL_W)
        .show_ui(ui, |ui| {
            for m in &choices {
                if ui.selectable_label(*m == current, m.as_str()).clicked() {
                    app.settings.ai.model = m.clone();
                }
            }
            ui.separator();
            if ui.button(tr("ai.refresh_models")).clicked() {
                fetch = true;
            }
        })
        .response
        .rich_tip(tr("ai.model"));
    if fetch {
        app.ai_fetch_models();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{HttpResponse, Provider};
    use egui::{vec2, Pos2, Rect};

    fn frame(ctx: &egui::Context, app: &mut App) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
    }

    #[test]
    fn the_bar_shows_while_a_service_is_set_up() {
        let mut app = App::headless();
        app.settings.ai = Default::default();
        assert!(!visible(&app), "no key yet");
        app.ai.key = "k".into();
        assert!(visible(&app));
        app.settings.ai.show_bar = false;
        assert!(!visible(&app), "turned off");
        app.settings.ai.show_bar = true;
        // A local server: an address of its own and a model, no key.
        app.ai.key.clear();
        app.settings.ai.provider = Provider::OpenAiCompatible;
        assert!(!visible(&app), "no model");
        app.settings.ai.model = "llama3".into();
        assert!(!visible(&app), "the default address needs a key");
        app.settings.ai.endpoint = "http://localhost:11434/v1/chat/completions".into();
        assert!(visible(&app));
        // The whole window draws with the bar, in every state.
        let ctx = crate::theme::ui_context();
        for status in [
            AiStatus::Idle,
            AiStatus::Waiting,
            AiStatus::Done(4),
            AiStatus::Listed(2),
            AiStatus::Failed("a long message from the service".repeat(5)),
        ] {
            app.ai.status = status;
            frame(&ctx, &mut app);
        }
    }

    #[test]
    fn the_model_list_offers_the_services_models() {
        let mut app = App::headless();
        app.settings.ai = Default::default();
        app.ai.key = "k".into();
        let choices = app.ai_model_choices();
        assert_eq!(
            choices.first().map(String::as_str),
            Some("claude-sonnet-5-5")
        );
        assert!(choices.len() > 1);
        // Fetching asks the models address and lists what came.
        app.ai.fake = Some(|req| {
            assert_eq!(req.method, "GET");
            assert_eq!(req.url, "https://api.anthropic.com/v1/models");
            Ok(HttpResponse {
                status: 200,
                body: r#"{"data":[{"id":"model-a"},{"id":"model-b"}],"has_more":false}"#.into(),
            })
        });
        app.ai_fetch_models();
        assert_eq!(app.ai.status, AiStatus::Listed(2));
        // The model in use stays first when the service does not list it.
        assert_eq!(
            app.ai_model_choices(),
            vec![
                "claude-sonnet-5-5".to_string(),
                "model-a".into(),
                "model-b".into()
            ]
        );
        app.settings.ai.model = "model-b".into();
        assert_eq!(
            app.ai_model_choices(),
            vec!["model-a".to_string(), "model-b".into()]
        );
        // A chat completions service: models next to chat/completions.
        let mut prefs = crate::ai::AiPrefs {
            provider: Provider::OpenAiCompatible,
            endpoint: "http://localhost:11434/v1/chat/completions/".into(),
            ..Default::default()
        };
        assert_eq!(prefs.models_endpoint(), "http://localhost:11434/v1/models");
        prefs.endpoint = "https://example.org/api".into();
        assert_eq!(prefs.models_endpoint(), "https://example.org/api/models");
        // Errors are shown.
        app.ai.fake = Some(|_| {
            Ok(HttpResponse {
                status: 403,
                body: r#"{"error":{"message":"no access"}}"#.into(),
            })
        });
        app.ai_fetch_models();
        assert!(matches!(&app.ai.status, AiStatus::Failed(e) if e.contains("no access")));
    }
}
