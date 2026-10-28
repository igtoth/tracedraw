//! File > GitHub's dialogs: Open from GitHub, Save to GitHub (a commit)
//! and History.

use crate::app::App;
use crate::github::GithubForm;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::dialogs::{ok_cancel, window};
use crate::ui::tips::RichTip;
use egui::{Color32, Context, RichText, Ui};

const ERROR: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);

/// Repository, branch, path and token; `message` adds the commit message.
fn fields(ui: &mut Ui, f: &mut GithubForm, message: bool) {
    egui::Grid::new("github_form")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label(tr("github.repository"));
            ui.add(
                egui::TextEdit::singleline(&mut f.repo)
                    .hint_text("owner/name")
                    .desired_width(280.0),
            )
            .rich_tip(crate::ui::tips::keyed("github.repository"));
            ui.end_row();
            ui.label(tr("github.branch"));
            ui.add(egui::TextEdit::singleline(&mut f.branch).desired_width(280.0))
                .rich_tip(crate::ui::tips::keyed("github.branch"));
            ui.end_row();
            ui.label(tr("github.path"));
            ui.add(
                egui::TextEdit::singleline(&mut f.path)
                    .hint_text("drawings/logo.tdraw")
                    .desired_width(280.0),
            )
            .rich_tip(crate::ui::tips::keyed("github.path"));
            ui.end_row();
            if message {
                ui.label(tr("github.message"));
                ui.add(egui::TextEdit::singleline(&mut f.message).desired_width(280.0))
                    .rich_tip(crate::ui::tips::keyed("github.message"));
                ui.end_row();
            }
            ui.label(tr("github.token"));
            ui.add(
                egui::TextEdit::singleline(&mut f.token)
                    .password(true)
                    .desired_width(280.0),
            )
            .rich_tip(crate::ui::tips::pair("github.token", "github.token_hint"));
            ui.end_row();
        });
    ui.checkbox(&mut f.remember_token, tr("github.remember_token"))
        .rich_tip(crate::ui::tips::keyed("github.remember_token"));
    ui.label(
        RichText::new(tr("github.token_hint"))
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// What is going on, or what went wrong.
fn status(app: &App, ui: &mut Ui) {
    if app.github.busy() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(tr("github.working"));
        });
    } else if !app.github.status.is_empty() {
        ui.label(RichText::new(&app.github.status).color(ERROR));
    }
}

pub fn open_dialog(app: &mut App, ctx: &Context, f: &mut GithubForm, close: &mut bool) {
    window(ctx, tr("github.open_title")).show(ctx, |ui| {
        ui.set_max_width(420.0);
        fields(ui, f, false);
        status(app, ui);
        let mut keep_open = false;
        if ok_cancel(ui, close) && !app.github.busy() {
            // The dialog stays until the file came (or the reason why not).
            keep_open = true;
            if let Err(e) = app.github_open(f) {
                app.github.status = e;
            }
        }
        if keep_open {
            *close = false;
        }
    });
}

pub fn save_dialog(app: &mut App, ctx: &Context, f: &mut GithubForm, close: &mut bool) {
    window(ctx, tr("github.save_title")).show(ctx, |ui| {
        ui.set_max_width(420.0);
        fields(ui, f, true);
        status(app, ui);
        let mut keep_open = false;
        if ok_cancel(ui, close) && !app.github.busy() {
            keep_open = true;
            if let Err(e) = app.github_save(f) {
                app.github.status = e;
            }
        }
        if keep_open {
            *close = false;
        }
    });
}

pub fn history_dialog(app: &mut App, ctx: &Context, close: &mut bool) {
    window(ctx, tr("github.history_title")).show(ctx, |ui| {
        ui.set_max_width(520.0);
        let history = app.github.history.clone();
        match &history {
            Some((remote, list)) => {
                ui.label(
                    RichText::new(format!(
                        "{} : {} ({})",
                        remote.repo_name(),
                        remote.path,
                        remote.branch
                    ))
                    .strong(),
                );
                if list.is_empty() {
                    ui.label(tr("github.no_commits"));
                }
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        egui::Grid::new("github_history")
                            .num_columns(4)
                            .spacing([10.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for c in list {
                                    ui.monospace(&c.sha[..c.sha.len().min(7)]);
                                    ui.label(c.date.replace('T', " ").trim_end_matches('Z'));
                                    ui.label(format!("{}  {}", c.author, c.message));
                                    if ui
                                        .add_enabled(
                                            !app.github.busy(),
                                            egui::Button::new(tr("github.open_version")),
                                        )
                                        .rich_tip_always(crate::ui::tips::keyed(
                                            "github.open_version",
                                        ))
                                        .clicked()
                                    {
                                        app.github_open_version(remote, &c.sha);
                                    }
                                    ui.end_row();
                                }
                            });
                    });
            }
            None => {
                if !app.github.busy() && app.github.status.is_empty() {
                    app.github_history();
                }
            }
        }
        status(app, ui);
        if ui
            .button(tr("dialog.close"))
            .rich_tip(crate::ui::tips::keyed("dialog.close"))
            .clicked()
        {
            *close = true;
        }
    });
}
