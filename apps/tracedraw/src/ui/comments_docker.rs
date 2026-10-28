//! The Comments docker (Window > Panels > Comments): the name comments
//! are signed with, buttons that place a pin, a box or an arrow on the
//! page, Show resolved and Hide marks, then every comment of the drawing
//! as a card: number, author, date, text (editable when picked),
//! replies, a reply field, Resolve or Reopen, and Delete.

use crate::app::App;
use crate::comments::{MarkKind, MARK};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{RichText, Ui};

fn dim(ui: &mut Ui, text: String) {
    ui.label(RichText::new(text).color(Tokens::text_dim()).size(11.0));
}

pub fn comments_docker(app: &mut App, ui: &mut Ui) {
    let drawing = app.has_document() && !app.show_welcome;
    ui.horizontal(|ui| {
        dim(ui, tr("comments.name"));
        let hint = app.comment_author();
        ui.add(
            egui::TextEdit::singleline(&mut app.settings.comment_author)
                .hint_text(hint)
                .desired_width(140.0),
        );
    });
    ui.horizontal_wrapped(|ui| {
        dim(ui, tr("comments.add"));
        for (kind, key, tip) in [
            (MarkKind::Pin, "comments.pin", "comments.pin_tip"),
            (MarkKind::Rect, "comments.rect", "comments.rect_tip"),
            (MarkKind::Arrow, "comments.arrow", "comments.arrow_tip"),
        ] {
            let on = app.comments.placing == Some(kind);
            if ui
                .add_enabled(drawing, egui::Button::new(tr(key)).selected(on))
                .rich_tip(tr(tip))
                .clicked()
            {
                app.comments.placing = if on { None } else { Some(kind) };
            }
        }
    });
    if app.comments.placing.is_some() {
        ui.label(RichText::new(tr("comments.placing")).color(MARK).size(11.0));
    }
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(
            &mut app.comments.show_resolved,
            tr("comments.show_resolved"),
        );
        ui.checkbox(&mut app.comments.hidden, tr("comments.hide_marks"));
    });
    ui.separator();

    let tz = crate::time_tracker::local_zone();
    let comments = app.doc().comments.clone();
    let page_no = |page| {
        app.doc()
            .pages
            .iter()
            .position(|p| p.id == page)
            .map(|i| i + 1)
            .unwrap_or(0)
    };
    let pages: Vec<usize> = comments.iter().map(|c| page_no(c.page)).collect();
    let open = comments.iter().filter(|c| !c.resolved).count();
    dim(
        ui,
        trf(
            "comments.count",
            &[
                ("n", &open.to_string()),
                ("all", &comments.len().to_string()),
            ],
        ),
    );
    if comments.is_empty() {
        dim(ui, tr("comments.empty"));
        return;
    }
    let mut go = None;
    let mut edit: Option<(u32, String)> = None;
    let mut resolve: Option<(u32, bool)> = None;
    let mut delete = None;
    let mut reply: Option<u32> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (n, c) in comments.iter().enumerate() {
                if c.resolved && !app.comments.show_resolved {
                    continue;
                }
                let picked = app.comments.selected == Some(c.id);
                let frame = egui::Frame::new()
                    .stroke(egui::Stroke::new(
                        if picked { 1.5 } else { 1.0 },
                        if picked { MARK } else { Tokens::border() },
                    ))
                    .corner_radius(4.0)
                    .inner_margin(6.0);
                let card = frame.show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(" {} ", n + 1))
                                .background_color(if c.resolved {
                                    MARK.gamma_multiply(0.45)
                                } else {
                                    MARK
                                })
                                .color(egui::Color32::WHITE)
                                .strong(),
                        );
                        ui.label(RichText::new(&c.author).strong());
                        dim(ui, crate::time_tracker::date_time(c.time, &tz));
                        dim(
                            ui,
                            trf(
                                "comments.page_n",
                                &[("n", &pages.get(n).copied().unwrap_or(0).to_string())],
                            ),
                        );
                    });
                    if picked {
                        let mut text = app
                            .comments
                            .drafts
                            .get(&c.id)
                            .cloned()
                            .unwrap_or_else(|| c.text.clone());
                        let r = ui.add(
                            egui::TextEdit::multiline(&mut text)
                                .hint_text(tr("comments.text_hint"))
                                .desired_rows(2)
                                .desired_width(f32::INFINITY),
                        );
                        if r.changed() {
                            app.comments.drafts.insert(c.id, text);
                        }
                        // What was typed is recorded once the field is left,
                        // as one undo step.
                        if !r.has_focus() && !app.comments.focus_text {
                            if let Some(d) = app.comments.drafts.remove(&c.id) {
                                if d != c.text {
                                    edit = Some((c.id, d));
                                }
                            }
                        }
                        if app.comments.focus_text {
                            r.request_focus();
                            app.comments.focus_text = false;
                        }
                    } else if c.text.is_empty() {
                        dim(ui, tr("comments.no_text"));
                    } else {
                        ui.label(&c.text);
                    }
                    for r in &c.replies {
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(12.0);
                            ui.label(RichText::new(&r.author).strong().size(12.0));
                            dim(ui, crate::time_tracker::date_time(r.time, &tz));
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(12.0);
                            ui.label(RichText::new(&r.text).size(12.0));
                        });
                    }
                    if picked {
                        ui.horizontal(|ui| {
                            let r = ui.add(
                                egui::TextEdit::singleline(&mut app.comments.reply)
                                    .hint_text(tr("comments.reply_hint"))
                                    .desired_width(ui.available_width() - 60.0),
                            );
                            let enter =
                                r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if (ui.button(tr("comments.reply")).clicked() || enter)
                                && !app.comments.reply.trim().is_empty()
                            {
                                reply = Some(c.id);
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        let key = if c.resolved {
                            "comments.reopen"
                        } else {
                            "comments.resolve"
                        };
                        if ui.small_button(tr(key)).clicked() {
                            resolve = Some((c.id, !c.resolved));
                        }
                        if ui.small_button(tr("comments.delete")).clicked() {
                            delete = Some(c.id);
                        }
                    });
                });
                let click = card.response.interact(egui::Sense::click());
                if click.clicked() && !picked {
                    go = Some(c.id);
                }
                ui.add_space(4.0);
            }
        });
    if let Some((id, text)) = edit {
        app.edit_comment(id, |c| c.text = text);
    }
    if let Some(id) = reply {
        let text = std::mem::take(&mut app.comments.reply);
        app.reply_to_comment(id, &text);
    }
    if let Some((id, resolved)) = resolve {
        app.edit_comment(id, |c| c.resolved = resolved);
    }
    if let Some(id) = delete {
        app.delete_comment(id);
    }
    if let Some(id) = go {
        app.comments.reply.clear();
        app.go_to_comment(id);
    }
}
