//! The vertical toolbox, laid out like the target design's: one button
//! per flyout showing the tool last used from it, a small arrow at the
//! bottom-right of the buttons that have a flyout, tooltips with the tool
//! name, shortcut and what the tool does, and vertical flyouts listing
//! each tool's icon, name and shortcut with separators between the kinds
//! of tools. Once a flyout is open, hovering another flyout button opens
//! that one instead. The button at the end shows or hides flyouts.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::{self, Tokens};
use crate::tools::{Tool, GROUPS};
use crate::ui::icons;
use egui::{Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// Tools after which a flyout draws a separator.
const SEPARATE_AFTER: [Tool; 6] = [
    Tool::FreeformPick,
    Tool::SegmentDelete,
    Tool::CommonShapes,
    Tool::SegmentDimension,
    Tool::RoundedConnector,
    Tool::AreaFill,
];

/// Toolbox groups followed by a separator line.
const GROUP_BREAKS: [usize; 5] = [3, 5, 9, 11, 13];

/// Whether group `gi` is shown (the Outline flyout is hidden by default).
pub fn group_visible(app: &App, gi: usize) -> bool {
    let Some(group) = GROUPS.get(gi) else {
        return false;
    };
    if group.hidden {
        return app.settings.show_outline_flyout;
    }
    !app.settings
        .toolbox_hidden
        .iter()
        .any(|id| id == group.tools[0].id())
}

/// The tooltip of a tool: its name and shortcut in bold, then what it does.
pub fn tool_tooltip(ui: &mut Ui, tool: Tool) {
    let head = if tool.shortcut_label().is_empty() {
        crate::i18n::trf("toolbox.tool_title", &[("t", &tool.name())])
    } else {
        format!(
            "{} ({})",
            crate::i18n::trf("toolbox.tool_title", &[("t", &tool.name())]),
            tool.shortcut_label()
        )
    };
    ui.set_max_width(260.0);
    ui.label(egui::RichText::new(head).font(theme::bold(12.0)));
    ui.label(egui::RichText::new(tr(&format!("tooldesc.{}", tool.id()))).size(12.0));
}

pub fn toolbox(app: &mut App, ui: &mut Ui) {
    ui.spacing_mut().item_spacing = Vec2::new(0.0, 1.0);
    ui.add_space(2.0);
    let size = Vec2::new(Tokens::TOOL_BUTTON, Tokens::TOOL_BUTTON);
    for (gi, group) in GROUPS.iter().enumerate() {
        if !group_visible(app, gi) {
            continue;
        }
        // The button shows the tool last used in this group, even while a
        // tool of another group is active.
        let shown = if group.tools.contains(&app.tool) {
            app.tool
        } else {
            app.toolbox_last
                .get(gi)
                .copied()
                .filter(|t| group.tools.contains(t))
                .unwrap_or(group.tools[0])
        };
        let active = group.tools.contains(&app.tool);
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let has_flyout = group.tools.len() > 1;
        let arrow_zone = Rect::from_min_max(rect.right_bottom() - Vec2::splat(10.0), rect.max);
        let bg = if active {
            Tokens::TOOL_ACTIVE
        } else if resp.hovered() {
            Tokens::TOOL_HOVER
        } else {
            Tokens::PANEL
        };
        ui.painter().rect_filled(rect, 2.0, bg);
        if active {
            ui.painter().rect_stroke(
                rect,
                2.0,
                Stroke::new(1.0, Tokens::SELECTION.gamma_multiply(0.5)),
                egui::StrokeKind::Inside,
            );
        }
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(20.0));
        icons::draw(ui.painter(), icon_rect, shown, Tokens::ICON);
        if has_flyout {
            // The flyout arrow: a small triangle at the bottom-right.
            let p = rect.right_bottom() - Vec2::new(2.0, 2.0);
            ui.painter().add(egui::epaint::PathShape::convex_polygon(
                vec![p, p - Vec2::new(5.0, 0.0), p - Vec2::new(0.0, 5.0)],
                Tokens::ICON,
                Stroke::NONE,
            ));
        }
        // No tooltip while a flyout is open: it would cover the list.
        let resp = if app.flyout_open.is_none() {
            resp.on_hover_ui(|ui| tool_tooltip(ui, shown))
        } else {
            resp
        };
        let on_arrow = resp
            .interact_pointer_pos()
            .is_some_and(|p| arrow_zone.contains(p));

        // Opening the flyout: the arrow, a right click, a drag, or a press
        // held for 0.4 s; with a flyout already open, hovering opens this
        // one instead.
        if has_flyout {
            let held = resp.is_pointer_button_down_on()
                && ui.input(|i| {
                    let started = i.pointer.press_start_time().unwrap_or(i.time);
                    i.time - started >= 0.4
                });
            if resp.is_pointer_button_down_on() && !held {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
            }
            let switch = resp.hovered() && app.flyout_open.is_some_and(|o| o != gi);
            if (resp.clicked() && on_arrow)
                || resp.secondary_clicked()
                || resp.drag_started()
                || resp.long_touched()
                || held
                || switch
            {
                app.flyout_open = Some(gi);
            }
        }
        // A plain click picks the shown tool (not when it opened the flyout).
        if resp.clicked() && !(has_flyout && on_arrow) && app.flyout_open != Some(gi) {
            app.set_tool(shown);
        }
        // Double-clicking a tool button: Pick selects every object, Zoom
        // fits the drawing, Rectangle adds a frame around the page.
        if resp.double_clicked() {
            match shown {
                Tool::Pick => app.select_all(),
                Tool::Zoom => app.zoom_to_fit(),
                Tool::Rectangle => app.add_page_frame(),
                _ => {}
            }
        }
        if app.flyout_open == Some(gi) {
            flyout(app, ui, gi, rect);
        }
        if GROUP_BREAKS.contains(&gi) {
            ui.add_space(2.0);
            let (r, _) =
                ui.allocate_exact_size(Vec2::new(Tokens::TOOL_BUTTON, 1.0), Sense::hover());
            ui.painter().hline(
                r.x_range().shrink(4.0),
                r.center().y,
                Stroke::new(1.0, Tokens::BORDER),
            );
            ui.add_space(2.0);
        }
    }
    quick_customize(app, ui);
}

/// The flyout of group `gi`, opened beside its button.
fn flyout(app: &mut App, ui: &mut Ui, gi: usize, button: Rect) {
    let Some(group) = GROUPS.get(gi) else {
        return;
    };
    let pos = button.right_top() + Vec2::new(2.0, 0.0);
    let shown = app.toolbox_last.get(gi).copied().unwrap_or(group.tools[0]);
    let mut close = false;
    let area = egui::Area::new(egui::Id::new(("flyout", gi)))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .fade_in(false)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::symmetric(2, 3))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);
                    let row_w = group
                        .tools
                        .iter()
                        .map(|t| {
                            let name = ui.painter().layout_no_wrap(
                                t.name(),
                                egui::FontId::proportional(12.0),
                                Tokens::TEXT,
                            );
                            name.size().x
                        })
                        .fold(0.0_f32, f32::max)
                        + 22.0
                        + 24.0
                        + 70.0;
                    for t in group.tools {
                        let (r, rr) =
                            ui.allocate_exact_size(Vec2::new(row_w, 24.0), Sense::click());
                        let current =
                            *t == app.tool || (*t == shown && !group.tools.contains(&app.tool));
                        if rr.hovered() {
                            ui.painter().rect_filled(r, 0.0, Tokens::TOOL_HOVER);
                        }
                        // A check mark beside the tool the button shows.
                        if current {
                            let c = Pos2::new(r.min.x + 9.0, r.center().y);
                            let s = Stroke::new(1.4, Tokens::ICON);
                            ui.painter().line_segment(
                                [c + Vec2::new(-3.5, 0.0), c + Vec2::new(-1.0, 3.0)],
                                s,
                            );
                            ui.painter().line_segment(
                                [c + Vec2::new(-1.0, 3.0), c + Vec2::new(4.0, -3.5)],
                                s,
                            );
                        }
                        let icon = Rect::from_center_size(
                            Pos2::new(r.min.x + 30.0, r.center().y),
                            Vec2::splat(18.0),
                        );
                        icons::draw(ui.painter(), icon, *t, Tokens::ICON);
                        ui.painter().text(
                            Pos2::new(r.min.x + 46.0, r.center().y),
                            egui::Align2::LEFT_CENTER,
                            t.name(),
                            egui::FontId::proportional(12.0),
                            Tokens::TEXT,
                        );
                        if !t.shortcut_label().is_empty() {
                            ui.painter().text(
                                Pos2::new(r.max.x - 8.0, r.center().y),
                                egui::Align2::RIGHT_CENTER,
                                t.shortcut_label(),
                                egui::FontId::proportional(12.0),
                                Tokens::TEXT,
                            );
                        }
                        if rr.on_hover_ui(|ui| tool_tooltip(ui, *t)).clicked() {
                            app.set_tool(*t);
                            close = true;
                        }
                        if SEPARATE_AFTER.contains(t) {
                            let (sr, _) =
                                ui.allocate_exact_size(Vec2::new(row_w, 7.0), Sense::hover());
                            ui.painter().hline(
                                sr.x_range().shrink(6.0),
                                sr.center().y,
                                Stroke::new(1.0, Tokens::BORDER),
                            );
                        }
                    }
                });
        });
    // Close when the pointer wanders away from the flyout and its button
    // (no click needed, so the next press reaches the canvas untouched),
    // or with a click elsewhere.
    let zone = area.response.rect.union(button).expand(24.0);
    let (away, clicked_elsewhere) = ui.input(|i| {
        let pos = i.pointer.latest_pos();
        let away = pos.map(|p| !zone.contains(p)).unwrap_or(false);
        let click = i.pointer.any_pressed() && pos.is_some_and(|p| !zone.contains(p));
        (away, click)
    });
    // Moving onto another toolbox button keeps a flyout open (it switches).
    let over_toolbox = ui
        .input(|i| i.pointer.latest_pos())
        .is_some_and(|p| p.x <= button.max.x + 2.0 && p.x >= button.min.x - 4.0);
    if close || clicked_elsewhere || (away && !over_toolbox) {
        app.flyout_open = None;
    }
}

/// The "+" at the end of the toolbox: tick the flyouts to show.
fn quick_customize(app: &mut App, ui: &mut Ui) {
    ui.add_space(4.0);
    ui.vertical_centered(|ui| {
        ui.menu_button(
            egui::RichText::new("+").size(16.0).color(Tokens::TEXT_DIM),
            |ui| {
                for (gi, group) in GROUPS.iter().enumerate() {
                    let mut on = group_visible(app, gi);
                    let label = group
                        .tools
                        .iter()
                        .map(|t| t.name())
                        .collect::<Vec<_>>()
                        .join(", ");
                    if ui.checkbox(&mut on, label).changed() {
                        set_group_visible(app, gi, on);
                    }
                }
                ui.separator();
                if ui.button(tr("toolbox.reset")).clicked() {
                    app.settings.toolbox_hidden.clear();
                    app.settings.show_outline_flyout = false;
                    ui.close();
                }
            },
        )
        .response
        .on_hover_text(tr("toolbox.quick_customize"));
    });
}

fn set_group_visible(app: &mut App, gi: usize, on: bool) {
    let Some(group) = GROUPS.get(gi) else {
        return;
    };
    if group.hidden {
        app.settings.show_outline_flyout = on;
        return;
    }
    let id = group.tools[0].id().to_string();
    app.settings.toolbox_hidden.retain(|h| *h != id);
    if !on {
        app.settings.toolbox_hidden.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flyout_groups_can_be_hidden_and_reset() {
        let mut app = App::headless();
        assert!(group_visible(&app, 0));
        let outline = GROUPS.iter().position(|g| g.hidden).expect("outline group");
        assert!(!group_visible(&app, outline));
        set_group_visible(&mut app, 2, false);
        assert!(!group_visible(&app, 2));
        set_group_visible(&mut app, outline, true);
        assert!(group_visible(&app, outline));
        set_group_visible(&mut app, 2, true);
        assert!(group_visible(&app, 2));
    }

    #[test]
    fn separators_split_the_reference_subgroups() {
        // Each separator tool sits inside a flyout and is not its last tool.
        for t in SEPARATE_AFTER {
            let g = GROUPS
                .iter()
                .find(|g| g.tools.contains(&t))
                .expect("in a group");
            assert_ne!(g.tools.last(), Some(&t), "{t:?}");
        }
    }
}
