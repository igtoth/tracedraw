//! Welcome Screen: Get Started (new, open, recent documents), Workspace,
//! What's New, Learn, Templates.

use crate::app::{App, WelcomeTab};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::icons;
use crate::ui::tips::RichTip;
use egui::Ui;

const NEWS: &[(&str, &str)] = &[
    ("welcome.news_1_title", "welcome.news_1_body"),
    ("welcome.news_2_title", "welcome.news_2_body"),
    ("welcome.news_3_title", "welcome.news_3_body"),
];

pub fn welcome_screen(app: &mut App, ui: &mut Ui) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        ui.heading(egui::RichText::new("TraceDraw").size(26.0));
        ui.add_space(30.0);
        for (tab, key) in [
            (WelcomeTab::GetStarted, "welcome.get_started"),
            (WelcomeTab::Workspace, "welcome.workspace"),
            (WelcomeTab::News, "welcome.whats_new"),
            (WelcomeTab::Learn, "welcome.learn"),
            (WelcomeTab::Templates, "welcome.templates"),
        ] {
            if ui
                .selectable_label(
                    app.welcome_tab == tab,
                    egui::RichText::new(tr(key)).size(14.0),
                )
                .clicked()
            {
                app.welcome_tab = tab;
            }
        }
    });
    ui.separator();
    ui.add_space(12.0);
    match app.welcome_tab {
        WelcomeTab::GetStarted => get_started(app, ui),
        WelcomeTab::Workspace => workspace(app, ui),
        WelcomeTab::News => news(ui),
        WelcomeTab::Learn => learn(ui),
        WelcomeTab::Templates => templates(app, ui),
    }
}

fn big_button(ui: &mut Ui, icon: icons::Action, title: String, subtitle: String) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(240.0, 64.0), egui::Sense::click());
    let bg = if resp.hovered() {
        Tokens::tool_hover()
    } else {
        Tokens::panel()
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, bg);
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, Tokens::border()),
        egui::epaint::StrokeKind::Inside,
    );
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 32.0, rect.center().y),
        egui::vec2(28.0, 28.0),
    );
    icons::draw_action(painter, icon_rect, icon, Tokens::ACCENT);
    let text_x = rect.left() + 64.0;
    painter.text(
        egui::pos2(
            text_x,
            rect.center().y - if subtitle.is_empty() { 0.0 } else { 9.0 },
        ),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(14.0),
        Tokens::text(),
    );
    if !subtitle.is_empty() {
        painter.text(
            egui::pos2(text_x, rect.center().y + 10.0),
            egui::Align2::LEFT_CENTER,
            subtitle,
            egui::FontId::proportional(11.0),
            Tokens::text_dim(),
        );
    }
    resp.clicked()
}

fn get_started(app: &mut App, ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.set_width(260.0);
            if big_button(
                ui,
                icons::Action::New,
                tr("welcome.new_document"),
                "Ctrl+N".into(),
            ) {
                app.request_new_document();
            }
            ui.add_space(8.0);
            if big_button(
                ui,
                icons::Action::Welcome,
                tr("welcome.new_from_template"),
                String::new(),
            ) {
                app.open_template_dialog();
            }
            ui.add_space(8.0);
            if big_button(
                ui,
                icons::Action::Open,
                tr("welcome.open_document"),
                "Ctrl+O".into(),
            ) {
                app.open_dialog();
                if app.file.is_some() || app.doc().all_layers().any(|l| !l.shapes.is_empty()) {
                    app.show_welcome = false;
                }
            }
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new(tr("welcome.shortcuts_hint"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
        });
        // A browser keeps no recent documents.
        if crate::files::WEB {
            return;
        }
        ui.add_space(40.0);
        ui.vertical(|ui| {
            ui.strong(tr("welcome.recent_documents"));
            let recent = app.settings.recent_files.clone();
            if recent.is_empty() {
                ui.label(egui::RichText::new(tr("welcome.no_recent")).color(Tokens::text_dim()));
            }
            ui.add_space(4.0);
            for p in recent {
                if recent_row(app, ui, &p) {
                    app.open_path(p.clone());
                    app.show_welcome = false;
                }
            }
            if !app.settings.recent_files.is_empty()
                && ui.small_button(tr("menu.file.clear_recent")).clicked()
            {
                app.settings.recent_files.clear();
                app.settings.save();
                crate::thumbnails::clear();
                app.recent_thumbs.clear();
            }
        });
    });
}

/// One recent drawing: its thumbnail (a blank page when it has none), its
/// name and folder. True when clicked; a missing file is greyed out.
fn recent_row(app: &mut App, ui: &mut Ui, path: &std::path::Path) -> bool {
    const ROW: egui::Vec2 = egui::vec2(380.0, 56.0);
    const THUMB: egui::Vec2 = egui::vec2(64.0, 48.0);
    let exists = path.exists();
    let thumb = if exists {
        app.recent_thumbnail(ui.ctx(), path)
    } else {
        None
    };
    let sense = if exists {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(ROW, sense);
    let painter = ui.painter();
    if exists && resp.hovered() {
        painter.rect_filled(rect, 2.0, Tokens::tool_hover());
    }
    let frame = egui::Rect::from_min_size(rect.min + egui::vec2(4.0, 4.0), THUMB);
    painter.rect_filled(frame, 0.0, egui::Color32::WHITE);
    if let Some(t) = &thumb {
        let size = t.size_vec2();
        let scale = (THUMB.x / size.x.max(1.0)).min(THUMB.y / size.y.max(1.0));
        let r = egui::Rect::from_center_size(frame.center(), size * scale);
        painter.image(
            t.id(),
            r,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    painter.rect_stroke(
        frame,
        0.0,
        egui::Stroke::new(1.0, Tokens::border()),
        egui::StrokeKind::Inside,
    );
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let folder = path
        .parent()
        .map(|d| d.display().to_string())
        .unwrap_or_default();
    let text_x = frame.right() + 10.0;
    let color = if exists {
        Tokens::text()
    } else {
        Tokens::text_dim()
    };
    painter.text(
        egui::pos2(text_x, frame.top() + 6.0),
        egui::Align2::LEFT_TOP,
        name,
        egui::FontId::proportional(13.0),
        color,
    );
    painter.text(
        egui::pos2(text_x, frame.top() + 26.0),
        egui::Align2::LEFT_TOP,
        folder,
        egui::FontId::proportional(11.0),
        Tokens::text_dim(),
    );
    resp.rich_tip(path.display().to_string()).clicked()
}

fn workspace(app: &mut App, ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.strong(tr("welcome.choose_workspace"));
            for (ws, key, desc) in [
                (
                    crate::app::Workspace::Default,
                    "menu.window.workspace_default",
                    "welcome.ws_default_desc",
                ),
                (
                    crate::app::Workspace::Lite,
                    "menu.window.workspace_lite",
                    "welcome.ws_lite_desc",
                ),
                (
                    crate::app::Workspace::Classic,
                    "menu.window.workspace_classic",
                    "welcome.ws_classic_desc",
                ),
                (
                    crate::app::Workspace::Illustration,
                    "menu.window.workspace_illustration",
                    "welcome.ws_illustration_desc",
                ),
                (
                    crate::app::Workspace::PageLayout,
                    "menu.window.workspace_page_layout",
                    "welcome.ws_page_layout_desc",
                ),
                (
                    crate::app::Workspace::Touch,
                    "menu.window.workspace_touch",
                    "welcome.ws_touch_desc",
                ),
            ] {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            app.workspace == ws,
                            egui::RichText::new(tr(key)).size(13.0),
                        )
                        .clicked()
                    {
                        app.set_workspace(ws);
                        app.save_settings();
                    }
                    ui.label(
                        egui::RichText::new(tr(desc))
                            .color(Tokens::text_dim())
                            .size(11.0),
                    );
                });
            }
            ui.add_space(12.0);
            ui.strong(tr("options.language"));
            let current = crate::i18n::language();
            ui.horizontal_wrapped(|ui| {
                for (code, name) in crate::i18n::LANGUAGES {
                    if ui.selectable_label(current == code, name).clicked() {
                        crate::i18n::set_language(code);
                        app.settings.language = code.to_string();
                        app.settings.save();
                    }
                }
            });
        });
    });
}

fn news(ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.set_max_width(640.0);
            ui.strong(trf(
                "welcome.version_n",
                &[("v", env!("CARGO_PKG_VERSION"))],
            ));
            for (t, b) in NEWS {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(tr(t)).strong().size(14.0));
                ui.label(tr(b));
            }
        });
    });
}

fn learn(ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.set_max_width(640.0);
            for (t, b) in [
                ("welcome.learn_1_title", "welcome.learn_1_body"),
                ("welcome.learn_2_title", "welcome.learn_2_body"),
                ("welcome.learn_3_title", "welcome.learn_3_body"),
                ("welcome.learn_4_title", "welcome.learn_4_body"),
                ("welcome.learn_5_title", "welcome.learn_5_body"),
            ] {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(tr(t)).strong().size(14.0));
                ui.label(tr(b));
            }
        });
    });
}

fn templates(app: &mut App, ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.strong(tr("welcome.templates"));
            let list = crate::templates::built_in();
            if let Some(i) = crate::ui::template_dialog::strip(ui, &list) {
                if let Some(t) = list.get(i) {
                    let name = app.next_untitled_name();
                    if let Some(doc) = crate::templates::document(t, &name) {
                        app.open_document(doc, None);
                    }
                }
            }
            ui.add_space(6.0);
            if ui.button(tr("tpl.more")).clicked() {
                app.open_template_dialog();
            }
            ui.add_space(12.0);
            ui.strong(tr("welcome.my_templates"));
            if ui.button(tr("welcome.open_template")).clicked() {
                crate::files::Dialog::new()
                    .add_filter("TraceDraw template", &["tdt", "tdraw"])
                    .pick_file(app, |app, p| app.open_as_template(p));
            }
            let dir = crate::settings::config_dir().map(|d| d.join("templates"));
            if let Some(dir) = dir {
                if let Ok(rd) = std::fs::read_dir(&dir) {
                    for e in rd.flatten() {
                        let p = e.path();
                        if p.extension().and_then(|x| x.to_str()) == Some("tdt") {
                            let name = p
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default();
                            if ui.button(name).clicked() {
                                app.open_as_template(p.clone());
                            }
                        }
                    }
                }
                ui.label(
                    egui::RichText::new(trf(
                        "welcome.templates_folder",
                        &[("d", &dir.display().to_string())],
                    ))
                    .color(Tokens::text_dim())
                    .size(11.0),
                );
            }
        });
    });
}
