//! The focus bar: in focus mode, a bar at the top left of the drawing
//! window with the way in (an exit button, the drawing, then each group
//! down to the object in focus); a click on a step goes back out to it.
//! See docs/behavior/focus-mode.md.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Context, RichText};

pub fn show(app: &mut App, ctx: &Context) {
    let path = app.focus_path();
    if path.is_empty() {
        return;
    }
    let at = app.canvas_rect.min + egui::vec2(14.0, 14.0);
    let mut go: Option<Option<usize>> = None;
    egui::Area::new(egui::Id::new("focus_bar"))
        .fixed_pos(at)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(Tokens::panel())
                .stroke(egui::Stroke::new(1.0, Tokens::border()))
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .button(RichText::new("\u{00D7}").size(14.0))
                            .rich_tip(tr("focus.exit"))
                            .clicked()
                        {
                            go = Some(None);
                        }
                        let title = app.doc().title.clone();
                        if ui
                            .link(RichText::new(title).size(15.0).color(Tokens::text()))
                            .rich_tip(tr("focus.exit"))
                            .clicked()
                        {
                            go = Some(None);
                        }
                        let last = path.len() - 1;
                        for (i, id) in path.iter().enumerate() {
                            ui.label(
                                RichText::new("\u{203A}")
                                    .size(15.0)
                                    .color(Tokens::text_dim()),
                            );
                            let name = app
                                .doc()
                                .find_shape(*id)
                                .map(crate::ui::objects_docker::object_label)
                                .unwrap_or_default();
                            if i == last {
                                ui.label(RichText::new(name).size(15.0).strong());
                            } else if ui
                                .link(RichText::new(name).size(15.0).color(Tokens::text()))
                                .clicked()
                            {
                                go = Some(Some(i));
                            }
                        }
                    });
                });
        });
    match go {
        Some(None) => {
            let target = app.focus_target();
            app.exit_focus();
            if let Some(id) = target {
                app.select(vec![id]);
            }
        }
        Some(Some(i)) => {
            if let Some(id) = path.get(i) {
                app.focus_on(*id);
            }
        }
        None => {}
    }
}
