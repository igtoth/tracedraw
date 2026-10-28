//! The AI parts shared by the dockers, menus and Tools > Options > AI:
//! the banner shown while AI features are off or not set up, and the two
//! pages of the AI options (General and Service: the one AI service).
//! See docs/behavior/ai-options.md.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::dialogs::{Dialog, OptionsPage};
use crate::ui::options::{grid, row_label};
use crate::ui::tips::RichTip;
use egui::{Color32, RichText, Ui};
const OK: Color32 = Color32::from_rgb(0x2E, 0x7D, 0x32);
const ERROR: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);

/// Which AI service a part of the interface needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    /// AI Drawing (a text service).
    Drawing,
    /// AI Generate and Remove Background (the service's image model).
    Images,
}

/// The banner's message, or None when the service is ready.
pub fn banner_text(app: &App, need: Need) -> Option<String> {
    if !app.ai_features_on() {
        return Some(tr("aiui.banner_off"));
    }
    let ready = match need {
        Need::Drawing => app.ai_drawing_ready(),
        Need::Images => app.ai_image_ready(),
    };
    (!ready).then(|| match need {
        Need::Drawing => tr("aiui.banner_drawing"),
        Need::Images if !app.settings.ai.can_make_pictures() => tr("aiui.banner_drawings_only"),
        Need::Images => tr("aiui.banner_pictures"),
    })
}

/// Open Tools > Options > AI at `page`.
pub fn open_options(app: &mut App, page: OptionsPage) {
    app.dialog = Dialog::Options;
    app.options_page = page;
}

/// The banner: what AI features need and a button to set them up. Returns
/// true when it showed (the service is not ready).
pub fn banner(app: &mut App, ui: &mut Ui, need: Need) -> bool {
    let Some(text) = banner_text(app, need) else {
        return false;
    };
    let dark = crate::theme::theme().is_dark();
    let (fill, line) = if dark {
        (
            Color32::from_rgb(0x1C, 0x33, 0x4A),
            Color32::from_rgb(0x3D, 0x7A, 0xB8),
        )
    } else {
        (Color32::from_rgb(0xE8, 0xF2, 0xFC), Tokens::ACCENT)
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, line))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.label(RichText::new("\u{2726}").size(18.0).color(line));
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr("aiui.banner_title")).strong());
                    ui.label(text);
                    ui.add_space(4.0);
                    if ui.button(tr("aiui.set_up")).clicked() {
                        let page = if !app.ai_features_on() {
                            OptionsPage::AiGeneral
                        } else {
                            OptionsPage::AiService
                        };
                        open_options(app, page);
                    }
                });
            });
        });
    ui.add_space(6.0);
    true
}

fn dim(ui: &mut Ui, key: &str) {
    ui.label(RichText::new(tr(key)).color(Tokens::text_dim()));
}

fn state_line(ui: &mut Ui, ready: bool) {
    if ready {
        ui.label(RichText::new(tr("aiui.ready")).color(OK));
    } else {
        ui.label(RichText::new(tr("aiui.not_ready")).color(Tokens::text_dim()));
    }
}

/// Where drawings and pictures stand: Ready, Not set up, or (pictures)
/// not offered by the chosen service.
fn service_state(app: &App, ui: &mut Ui) {
    grid("ai_services").show(ui, |ui| {
        row_label(ui, &tr("aiui.drawings"));
        state_line(ui, app.ai_drawing_ready());
        ui.end_row();
        row_label(ui, &tr("aiui.pictures"));
        if app.settings.ai.can_make_pictures() {
            state_line(ui, app.ai_image_ready());
        } else {
            ui.label(RichText::new(tr("aiui.drawings_only_state")).color(Tokens::text_dim()));
        }
        ui.end_row();
    });
}

/// Tools > Options > AI > General: the switch for all AI features and
/// where the service stands.
pub fn general_page(app: &mut App, ui: &mut Ui) {
    ui.checkbox(&mut app.settings.ai_enabled, tr("aiui.enable"));
    dim(ui, "aiui.enable_hint");
    crate::ui::chrome::section(ui, &tr("aiui.services"));
    service_state(app, ui);
    crate::ui::chrome::section(ui, &tr("aiui.features"));
    for (key, need) in [
        ("aiui.feature_drawing", Need::Drawing),
        ("aiui.feature_generate", Need::Images),
        ("aiui.feature_remix", Need::Images),
        ("aiui.feature_vectors", Need::Images),
        ("aiui.feature_cutout", Need::Images),
    ] {
        let on = match need {
            Need::Drawing => app.ai_drawing_ready(),
            Need::Images => app.ai_image_ready(),
        };
        ui.add_enabled(on, egui::Label::new(format!("\u{2022} {}", tr(key))));
    }
    ui.add_space(8.0);
    crate::ui::chrome::section(ui, &tr("aiui.privacy_title"));
    dim(ui, "aiui.privacy");
}

/// Tools > Options > AI > Service: the one AI service. Its text model
/// writes drawings; its image model, when the service can make pictures,
/// serves AI Generate and Remove Background.
pub fn service_page(app: &mut App, ui: &mut Ui) {
    if !app.ai_features_on() {
        banner(app, ui, Need::Drawing);
    }
    dim(ui, "aiui.service_hint");
    ui.add_space(6.0);
    crate::ui::ai_docker::connection(app, ui);
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let busy = app.ai.busy() || app.ai_image.busy();
        if ui
            .add_enabled(!busy, egui::Button::new(tr("aiui.test")))
            .rich_tip(tr("aiui.test_hint"))
            .clicked()
        {
            app.ai_fetch_models();
            // The image models too, for the Image model list.
            app.ai_image.models.clear();
            app.ai_image_fetch_models();
        }
        if busy {
            ui.spinner();
        }
        match &app.ai.status {
            crate::ai::AiStatus::Listed(n) => {
                ui.label(
                    RichText::new(crate::i18n::trf("aiui.test_ok", &[("n", &n.to_string())]))
                        .color(OK),
                );
            }
            crate::ai::AiStatus::Failed(e) => {
                ui.label(RichText::new(e.as_str()).color(ERROR));
            }
            _ => {}
        }
    });
    if app.settings.ai.can_make_pictures() {
        if let crate::ai_image::ImageStatus::Failed(e) = &app.ai_image.status {
            ui.label(RichText::new(e.as_str()).color(ERROR));
        }
    }
    service_state(app, ui);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, app: &mut App, f: impl Fn(&mut App, &mut Ui)) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| f(app, ui));
        out.textures_delta.clear();
    }

    /// The one service at preset `key`.
    fn use_preset(app: &mut App, key: &str) {
        let (_, provider, endpoint) = crate::ai::PRESETS
            .iter()
            .find(|(k, _, _)| *k == key)
            .copied()
            .expect("preset");
        app.settings.ai.provider = provider;
        app.settings.ai.endpoint = endpoint.to_string();
    }

    #[test]
    fn the_banner_shows_until_the_service_is_set_up() {
        let mut app = App::headless();
        app.settings.ai_enabled = false;
        assert_eq!(banner_text(&app, Need::Images), Some(tr("aiui.banner_off")));
        app.settings.ai_enabled = true;
        // The usual service makes drawings only.
        assert_eq!(
            banner_text(&app, Need::Images),
            Some(tr("aiui.banner_drawings_only"))
        );
        assert_eq!(
            banner_text(&app, Need::Drawing),
            Some(tr("aiui.banner_drawing"))
        );
        app.ai.key = "k".into();
        assert_eq!(banner_text(&app, Need::Drawing), None);
        assert!(banner_text(&app, Need::Images).is_some());
        // OpenAI makes pictures with the same key.
        use_preset(&mut app, "aipreset.openai");
        app.ai.key.clear();
        assert_eq!(
            banner_text(&app, Need::Images),
            Some(tr("aiui.banner_pictures"))
        );
        app.ai.key = "k".into();
        assert_eq!(banner_text(&app, Need::Images), None);
        // Turning AI off hides every feature again.
        app.settings.ai_enabled = false;
        assert!(!app.ai_image_ready() && !app.ai_drawing_ready());
    }

    #[test]
    fn the_pages_and_the_banner_draw_in_every_state() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for on in [false, true] {
            app.settings.ai_enabled = on;
            for (key, _, _) in crate::ai::PRESETS {
                use_preset(&mut app, key);
                frame(&ctx, &mut app, general_page);
                frame(&ctx, &mut app, service_page);
                frame(&ctx, &mut app, |a, ui| {
                    banner(a, ui, Need::Images);
                });
            }
            app.settings.ai.endpoint = "http://my-box:9000/v1/chat/completions".into();
            frame(&ctx, &mut app, service_page);
        }
        app.ai_image.status = crate::ai_image::ImageStatus::Failed("bad key".into());
        frame(&ctx, &mut app, service_page);
    }

    #[test]
    fn test_connection_asks_for_the_models_and_the_image_models() {
        let mut app = App::headless();
        use_preset(&mut app, "aipreset.gemini");
        app.ai.key = "k".into();
        app.ai.fake = Some(|_| {
            Ok(crate::ai::HttpResponse {
                status: 200,
                body: r#"{"data":[{"id":"gemini-2.5-flash"}]}"#.into(),
            })
        });
        app.ai_image.fake = Some(|_| {
            Ok(crate::ai::HttpResponse {
                status: 200,
                body: r#"{"models":[{"name":"models/gemini-2.5-flash-image"}]}"#.into(),
            })
        });
        app.ai_fetch_models();
        app.ai_image_fetch_models();
        assert_eq!(app.ai.models, vec!["gemini-2.5-flash".to_string()]);
        assert_eq!(
            app.ai_image.models,
            vec!["gemini-2.5-flash-image".to_string()]
        );
        // Both with the one key, the images at Gemini's own API.
        let img = app.ai_image.sent.last().expect("sent");
        assert!(img
            .url
            .starts_with("https://generativelanguage.googleapis.com/v1beta/models"));
        assert!(img
            .headers
            .iter()
            .any(|(h, v)| h == "x-goog-api-key" && v == "k"));
        // A service that makes drawings only asks for no image models.
        use_preset(&mut app, "aipreset.ollama");
        let n = app.ai_image.sent.len();
        app.ai_image_fetch_models();
        assert_eq!(app.ai_image.sent.len(), n);
    }

    #[test]
    fn presets_fill_the_address_and_typed_ones_stay_custom() {
        let mut d = crate::ai::AiPrefs::default();
        assert_eq!(crate::ai::preset_of(&d), Some(0), "the usual service");
        let (_, provider, endpoint) = crate::ai::PRESETS[3];
        d.provider = provider;
        d.endpoint = endpoint.to_string();
        assert_eq!(crate::ai::preset_of(&d), Some(3));
        assert_eq!(d.endpoint(), "http://localhost:11434/v1/chat/completions");
        // A local server needs no key to be ready.
        let mut app = App::headless();
        app.settings.ai = d.clone();
        app.settings.ai.model = "llama3.2".into();
        assert!(app.ai_drawing_ready());
        d.endpoint = "http://my-box:9000/v1/chat/completions".into();
        assert_eq!(crate::ai::preset_of(&d), None);
        // Older preference files turn AI features on.
        let old: crate::settings::Settings = serde_json::from_str("{}").expect("read");
        assert!(old.ai_enabled);
    }
}
