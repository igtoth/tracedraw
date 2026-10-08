//! The vertical toolbox with flyouts.

use crate::app::App;
use crate::theme::Tokens;
use crate::tools::{Tool, GROUPS};
use crate::ui::icons;
use egui::{Rect, Sense, Ui, Vec2};

pub fn toolbox(app: &mut App, ui: &mut Ui) {
    ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
    ui.add_space(2.0);
    for (gi, group) in GROUPS.iter().enumerate() {
        // The button shows the tool last used in this group, as the editor does.
        let shown = if group.tools.contains(&app.tool) {
            app.tool
        } else {
            group.tools[0]
        };
        let active = group.tools.contains(&app.tool);
        let size = Vec2::new(Tokens::TOOL_BUTTON, Tokens::TOOL_BUTTON);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let hovered = resp.hovered();
        let bg = if active {
            Tokens::TOOL_ACTIVE
        } else if hovered {
            Tokens::TOOL_HOVER
        } else {
            Tokens::PANEL
        };
        ui.painter().rect_filled(rect, 2.0, bg);
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(18.0));
        let color = if shown.implemented() {
            Tokens::ICON
        } else {
            Tokens::TEXT_DIM
        };
        icons::draw(ui.painter(), icon_rect, shown, color);
        if group.tools.len() > 1 {
            // Flyout marker: small triangle at the bottom-right.
            let p = rect.right_bottom() - Vec2::new(3.0, 3.0);
            ui.painter().add(egui::epaint::PathShape::convex_polygon(
                vec![p, p - Vec2::new(4.0, 0.0), p - Vec2::new(0.0, 4.0)],
                Tokens::TEXT_DIM,
                egui::Stroke::NONE,
            ));
        }
        let tip = if shown.shortcut_label().is_empty() {
            shown.name().to_string()
        } else {
            format!("{} ({})", shown.name(), shown.shortcut_label())
        };
        let resp = resp.on_hover_text(tip);
        if resp.clicked() {
            app.set_tool(shown);
            app.flyout_open = None;
        }
        if group.tools.len() > 1
            && (resp.secondary_clicked() || resp.drag_started() || resp.long_touched())
        {
            app.flyout_open = Some(gi);
        }
        if app.flyout_open == Some(gi) {
            let pos = rect.right_top() + Vec2::new(4.0, 0.0);
            let mut close = false;
            egui::Area::new(egui::Id::new(("flyout", gi)))
                .fixed_pos(pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for t in group.tools {
                                let (r, rr) = ui.allocate_exact_size(size, Sense::click());
                                let bg = if *t == app.tool {
                                    Tokens::TOOL_ACTIVE
                                } else if rr.hovered() {
                                    Tokens::TOOL_HOVER
                                } else {
                                    Tokens::PANEL
                                };
                                ui.painter().rect_filled(r, 2.0, bg);
                                let color = if t.implemented() {
                                    Tokens::ICON
                                } else {
                                    Tokens::TEXT_DIM
                                };
                                icons::draw(
                                    ui.painter(),
                                    Rect::from_center_size(r.center(), Vec2::splat(18.0)),
                                    *t,
                                    color,
                                );
                                let tip = if t.shortcut_label().is_empty() {
                                    t.name().to_string()
                                } else {
                                    format!("{} ({})", t.name(), t.shortcut_label())
                                };
                                if rr.on_hover_text(tip).clicked() {
                                    app.set_tool(*t);
                                    close = true;
                                }
                            }
                        });
                    });
                });
            if close
                || ui.input(|i| {
                    i.pointer.any_click()
                        && i.pointer
                            .latest_pos()
                            .map(|p| {
                                !Rect::from_min_size(
                                    pos,
                                    Vec2::new(
                                        size.x * group.tools.len() as f32 + 16.0,
                                        size.y + 12.0,
                                    ),
                                )
                                .contains(p)
                                    && !rect.contains(p)
                            })
                            .unwrap_or(false)
                })
            {
                app.flyout_open = None;
            }
        }
        // Thin separators between groups, as in the original toolbox.
        if matches!(gi, 3 | 5 | 9 | 11 | 13) {
            ui.add_space(2.0);
            let (r, _) =
                ui.allocate_exact_size(Vec2::new(Tokens::TOOL_BUTTON, 1.0), Sense::hover());
            ui.painter().hline(
                r.x_range(),
                r.center().y,
                egui::Stroke::new(1.0, Tokens::BORDER),
            );
            ui.add_space(2.0);
        }
    }
    let _ = Tool::Pick;
}
