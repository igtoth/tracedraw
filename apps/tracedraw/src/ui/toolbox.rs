//! The vertical toolbox: one button
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
use crate::ui::tips::RichTip;
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
const GROUP_BREAKS: [usize; 7] = [1, 3, 8, 9, 11, 13, 15];

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

/// Buttons per row in a floating toolbox.
pub const FLOATING_COLUMNS: usize = 2;

/// The width of a floating toolbox's grid of buttons.
pub fn floating_width(app: &App) -> f32 {
    FLOATING_COLUMNS as f32 * button_width(app) + (FLOATING_COLUMNS - 1) as f32
}

/// The room a caption takes on each side of its text.
const CAPTION_PAD: f32 = 6.0;

/// The width of a toolbox button: its side, plus the widest tool name
/// (as measured when the toolbox was last drawn) when captions show
/// (the bars' menu > Customize > Toolbox Toolbar); a caption alone
/// takes the width of the name.
pub fn button_width(app: &App) -> f32 {
    let captions = app.settings.toolbox_captions;
    let side = Tokens::tool_button();
    if !captions.caption() {
        return side;
    }
    let text = app.toolbox_caption_w + 2.0 * CAPTION_PAD;
    if captions.image() {
        side + text
    } else {
        text.max(side)
    }
}

/// The width of a docked toolbox column (its buttons and the margins).
pub fn column_width(app: &App) -> f32 {
    button_width(app) + 6.0
}

/// The widest tool name among the flyouts shown (any tool of each, so
/// the buttons keep their width whichever tool they show), in the
/// caption font.
fn widest_caption(app: &App, ui: &Ui) -> f32 {
    let font = caption_font();
    ui.ctx().fonts_mut(|f| {
        GROUPS
            .iter()
            .enumerate()
            .filter(|(gi, _)| group_visible(app, *gi))
            .flat_map(|(_, g)| g.tools.iter())
            .map(|t| {
                f.layout_no_wrap(t.name(), font.clone(), egui::Color32::BLACK)
                    .size()
                    .x
            })
            .fold(0.0, f32::max)
    })
}

fn caption_font() -> egui::FontId {
    egui::FontId::proportional(12.0)
}

/// The toolbox, docked at `dock`: a row along the top or the bottom, a
/// column on a side, or floating as a grid two buttons wide.
pub fn toolbox(app: &mut App, ui: &mut Ui, dock: crate::ui::bars::Dock) {
    use crate::ui::bars::Dock;
    // Nothing to draw on the Welcome Screen: the tools show pale.
    if !app.drawing_shown() {
        ui.disable();
    }
    // A right click on the toolbox's empty part opens its menu, as on the
    // other bars.
    crate::ui::bars::background(app, ui, crate::ui::bars::Bar::Toolbox);
    match dock {
        Dock::Top | Dock::Bottom => {
            crate::ui::overflow::row(ui, "toolbox_row", |ui| tools(app, ui, dock, None));
        }
        Dock::Floating => {
            ui.set_max_width(floating_width(app));
            ui.horizontal_wrapped(|ui| tools(app, ui, dock, None));
        }
        Dock::Left | Dock::Right => {
            // Buttons shorter, down to a floor, so the column fits the
            // window without scrolling (#494).
            let room = ui.available_rect_before_wrap().height();
            let side = fitted_side(Tokens::tool_button(), visible_groups(app), room);
            crate::ui::overflow::column(ui, "toolbox_column", |ui| {
                tools(app, ui, dock, Some(side))
            });
        }
    }
}

/// The height a column toolbox's buttons get so `n` of them (with the
/// grip) fit `room`: their side, less when that does not fit, never under
/// [`MIN_FITTED_SIDE`] (the overflow arrows take over from there).
pub fn fitted_side(side: f32, n: usize, room: f32) -> f32 {
    const GRIP: f32 = 9.0;
    if n == 0 || !room.is_finite() {
        return side;
    }
    let n = n as f32;
    let fit = ((room - GRIP - (n - 1.0)) / n).floor();
    fit.clamp(MIN_FITTED_SIDE.min(side), side)
}

/// The shortest a column toolbox's buttons get to fit the window.
pub const MIN_FITTED_SIDE: f32 = 22.0;

/// How many buttons the toolbox shows.
fn visible_groups(app: &App) -> usize {
    (0..GROUPS.len())
        .filter(|gi| group_visible(app, *gi))
        .count()
}

fn tools(app: &mut App, ui: &mut Ui, dock: crate::ui::bars::Dock, fitted: Option<f32>) {
    use crate::ui::bars::{Bar, Dock};
    let horizontal = dock.horizontal();
    let floating = dock == Dock::Floating;
    ui.spacing_mut().item_spacing = if floating {
        Vec2::new(1.0, 1.0)
    } else if horizontal {
        Vec2::new(1.0, 0.0)
    } else {
        Vec2::new(0.0, 1.0)
    };
    if !floating {
        crate::ui::bars::grip(app, ui, Bar::Toolbox, horizontal);
    }
    // Captions: the icon, the tool's name, or both on each button; the
    // names measured now size the buttons from the next frame.
    let captions = app.settings.toolbox_captions;
    if captions.caption() {
        app.toolbox_caption_w = widest_caption(app, ui);
    }
    let side = fitted.unwrap_or_else(Tokens::tool_button);
    let size = Vec2::new(button_width(app), side);
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
        // The active tool's button is white with a grey frame; hovering
        // tints a button blue.
        let bg = if active {
            crate::theme::Tokens::field()
        } else if resp.hovered() {
            Tokens::tool_hover()
        } else {
            Tokens::panel()
        };
        ui.painter().rect_filled(rect, 0.0, bg);
        if active {
            ui.painter().rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, Tokens::control_border()),
                egui::StrokeKind::Inside,
            );
        }
        let icon_side = (side * 22.0 / 32.0).round();
        if captions.image() {
            let icon_rect = Rect::from_center_size(
                egui::pos2(rect.left() + side / 2.0, rect.center().y),
                Vec2::splat(icon_side),
            );
            icons::draw(
                ui.painter(),
                icon_rect,
                shown,
                icons::tool_color(shown, Tokens::icon()),
            );
        }
        if captions.caption() {
            let left = if captions.image() {
                rect.left() + side + CAPTION_PAD
            } else {
                rect.left() + CAPTION_PAD
            };
            ui.painter().text(
                egui::pos2(left, rect.center().y),
                egui::Align2::LEFT_CENTER,
                shown.name(),
                caption_font(),
                Tokens::text(),
            );
        }
        if has_flyout {
            // The flyout arrow: a small triangle at the bottom-right.
            let p = rect.right_bottom() - Vec2::new(2.0, 2.0);
            ui.painter().add(egui::epaint::PathShape::convex_polygon(
                vec![p, p - Vec2::new(5.0, 0.0), p - Vec2::new(0.0, 5.0)],
                Tokens::icon(),
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

        // Opening the flyout: the arrow, a drag, or a press
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
                || resp.drag_started()
                || resp.long_touched()
                || held
                || switch
            {
                app.flyout_open = Some(gi);
            }
        }
        // A right click on a tool button opens the toolbox's menu
        // (customize, button size, the bars), as on the other bars.
        crate::ui::bars::bar_menu(app, &resp, crate::ui::bars::Bar::Toolbox);
        // A plain click picks the shown tool (not when it opened the flyout).
        if resp.clicked() && !(has_flyout && on_arrow) && app.flyout_open != Some(gi) {
            app.choose_tool(shown);
        }
        // Double-clicking a tool button: Pick selects every object, Shape
        // every node, Zoom fits the drawing, Rectangle adds a frame around
        // the page (with Shift, around the selection).
        if resp.double_clicked() {
            match shown {
                Tool::Pick => app.select_all(),
                Tool::Zoom => app.zoom_to_fit(),
                Tool::Rectangle if ui.input(|i| i.modifiers.shift) => app.add_selection_frame(),
                Tool::Rectangle => app.add_page_frame(),
                // The Shape tool selects every node of the selected curves.
                Tool::Shape => app.select_all_nodes(),
                _ => {}
            }
        }
        if app.flyout_open == Some(gi) {
            flyout(app, ui, gi, rect, dock);
        }
        if GROUP_BREAKS.contains(&gi) && floating {
            // A line across the grid, on a row of its own.
            ui.end_row();
            let (r, _) =
                ui.allocate_exact_size(Vec2::new(floating_width(app), 3.0), Sense::hover());
            ui.painter().hline(
                r.x_range().shrink(4.0),
                r.center().y,
                Stroke::new(1.0, Tokens::border()),
            );
            ui.end_row();
        } else if GROUP_BREAKS.contains(&gi) {
            ui.add_space(2.0);
            if horizontal {
                let (r, _) = ui.allocate_exact_size(Vec2::new(1.0, side), Sense::hover());
                ui.painter().vline(
                    r.center().x,
                    r.y_range().shrink(4.0),
                    Stroke::new(1.0, Tokens::border()),
                );
            } else {
                let (r, _) =
                    ui.allocate_exact_size(Vec2::new(button_width(app), 1.0), Sense::hover());
                ui.painter().hline(
                    r.x_range().shrink(4.0),
                    r.center().y,
                    Stroke::new(1.0, Tokens::border()),
                );
            }
            ui.add_space(2.0);
        }
    }
    if floating {
        ui.end_row();
    }
    quick_customize(app, ui, horizontal);
}

/// The flyout of group `gi`, opened beside its button: to the right of a
/// toolbox on the left or floating, to the left of one on the right,
/// below one along the top and above one along the bottom.
fn flyout(app: &mut App, ui: &mut Ui, gi: usize, button: Rect, dock: crate::ui::bars::Dock) {
    use crate::ui::bars::Dock;
    let Some(group) = GROUPS.get(gi) else {
        return;
    };
    let (pos, pivot) = match dock {
        Dock::Right => (
            button.left_top() - Vec2::new(2.0, 0.0),
            egui::Align2::RIGHT_TOP,
        ),
        Dock::Top => (
            button.left_bottom() + Vec2::new(0.0, 2.0),
            egui::Align2::LEFT_TOP,
        ),
        Dock::Bottom => (
            button.left_top() - Vec2::new(0.0, 2.0),
            egui::Align2::LEFT_BOTTOM,
        ),
        Dock::Left | Dock::Floating => (
            button.right_top() + Vec2::new(2.0, 0.0),
            egui::Align2::LEFT_TOP,
        ),
    };
    let shown = app.toolbox_last.get(gi).copied().unwrap_or(group.tools[0]);
    let mut close = false;
    let area = egui::Area::new(egui::Id::new(("flyout", gi)))
        .pivot(pivot)
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
                                Tokens::text(),
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
                            ui.painter().rect_filled(r, 0.0, Tokens::tool_hover());
                        }
                        // A check mark beside the tool the button shows.
                        if current {
                            let c = Pos2::new(r.min.x + 9.0, r.center().y);
                            let s = Stroke::new(1.4, Tokens::icon());
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
                        icons::draw(
                            ui.painter(),
                            icon,
                            *t,
                            icons::tool_color(*t, Tokens::icon()),
                        );
                        ui.painter().text(
                            Pos2::new(r.min.x + 46.0, r.center().y),
                            egui::Align2::LEFT_CENTER,
                            t.name(),
                            egui::FontId::proportional(12.0),
                            Tokens::text(),
                        );
                        if !t.shortcut_label().is_empty() {
                            ui.painter().text(
                                Pos2::new(r.max.x - 8.0, r.center().y),
                                egui::Align2::RIGHT_CENTER,
                                t.shortcut_label(),
                                egui::FontId::proportional(12.0),
                                Tokens::text(),
                            );
                        }
                        if rr.on_hover_ui(|ui| tool_tooltip(ui, *t)).clicked() {
                            app.choose_tool(*t);
                            close = true;
                        }
                        if SEPARATE_AFTER.contains(t) {
                            let (sr, _) =
                                ui.allocate_exact_size(Vec2::new(row_w, 7.0), Sense::hover());
                            ui.painter().hline(
                                sr.x_range().shrink(6.0),
                                sr.center().y,
                                Stroke::new(1.0, Tokens::border()),
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
    let over_toolbox = ui.input(|i| i.pointer.latest_pos()).is_some_and(|p| {
        if dock == Dock::Floating {
            // The grid: any button of the floating toolbox.
            let reach = button_width(app) + 4.0;
            p.x <= button.max.x + reach && p.x >= button.min.x - reach
        } else if dock.horizontal() {
            p.y <= button.max.y + 2.0 && p.y >= button.min.y - 4.0
        } else {
            p.x <= button.max.x + 2.0 && p.x >= button.min.x - 4.0
        }
    });
    if close || clicked_elsewhere || (away && !over_toolbox) {
        app.flyout_open = None;
    }
}

/// The "+" at the end of the toolbox: tick the flyouts to show.
fn quick_customize(app: &mut App, ui: &mut Ui, horizontal: bool) {
    ui.add_space(4.0);
    let layout = if horizontal {
        egui::Layout::left_to_right(egui::Align::Center)
    } else {
        egui::Layout::top_down(egui::Align::Center)
    };
    ui.with_layout(layout, |ui| {
        ui.visuals_mut().widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
        ui.menu_button(
            egui::RichText::new("+")
                .size(18.0)
                .color(Tokens::text_dim()),
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
        .rich_tip(tr("toolbox.quick_customize"));
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

/// The bars' menu > Customize > Toolbox Toolbar > Reset to Default: every
/// flyout shown (the Outline flyout hidden, as at first), each button
/// showing its first tool, the default button size and icons only.
pub fn reset_toolbox(app: &mut App) {
    app.settings.toolbox_hidden.clear();
    app.settings.show_outline_flyout = false;
    app.settings.toolbox_button_size = None;
    app.settings.toolbox_captions = crate::settings::Captions::ImageOnly;
    app.toolbox_last = GROUPS.iter().map(|g| g.tools[0]).collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The buttons follow the default button size, or the toolbox's
    /// own; captions widen them by the widest tool name.
    #[test]
    fn buttons_follow_the_sizes_and_the_captions() {
        use crate::settings::{ButtonSize, Captions};
        let mut app = App::headless();
        crate::theme::set_button_sizes(ButtonSize::Medium, None);
        assert_eq!(button_width(&app), 32.0);
        crate::theme::set_button_sizes(ButtonSize::Large, None);
        assert_eq!(button_width(&app), 40.0);
        crate::theme::set_button_sizes(ButtonSize::Large, Some(ButtonSize::Small));
        assert_eq!(button_width(&app), 24.0);
        assert_eq!(crate::theme::Tokens::bar_button(), 40.0);
        assert_eq!(crate::theme::Tokens::prop_button(), 35.0);
        app.toolbox_caption_w = 50.0;
        app.settings.toolbox_captions = Captions::CaptionRight;
        assert_eq!(button_width(&app), 24.0 + 50.0 + 2.0 * CAPTION_PAD);
        app.settings.toolbox_captions = Captions::CaptionOnly;
        assert_eq!(button_width(&app), 50.0 + 2.0 * CAPTION_PAD);
        assert_eq!(floating_width(&app), 2.0 * (50.0 + 2.0 * CAPTION_PAD) + 1.0);
        // Reset to Default: icons only, the default size, every flyout.
        app.settings.toolbox_button_size = Some(ButtonSize::Small);
        set_group_visible(&mut app, 2, false);
        reset_toolbox(&mut app);
        assert_eq!(app.settings.toolbox_captions, Captions::ImageOnly);
        assert_eq!(app.settings.toolbox_button_size, None);
        assert!(group_visible(&app, 2));
        crate::theme::set_button_sizes(ButtonSize::Medium, None);
    }

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
    fn separators_split_the_toolbox_subgroups() {
        // Each separator tool sits inside a flyout and is not its last tool.
        for t in SEPARATE_AFTER {
            let g = GROUPS
                .iter()
                .find(|g| g.tools.contains(&t))
                .expect("in a group");
            assert_ne!(g.tools.last(), Some(&t), "{t:?}");
        }
    }

    /// A column toolbox fits the window: its buttons get shorter as the
    /// room shrinks, down to a floor, and keep their size when they fit
    /// (#494).
    #[test]
    fn a_column_toolbox_fits_the_window() {
        // 20 buttons of 32 px with the grip need 9 + 640 + 19 = 668.
        assert_eq!(fitted_side(32.0, 20, 900.0), 32.0);
        assert_eq!(fitted_side(32.0, 20, 668.0), 32.0);
        assert_eq!(fitted_side(32.0, 20, 600.0), 28.0);
        assert_eq!(fitted_side(32.0, 20, 300.0), MIN_FITTED_SIDE);
        // A small toolbox never grows, and nothing to fit changes nothing.
        assert_eq!(fitted_side(20.0, 20, 300.0), 20.0);
        assert_eq!(fitted_side(32.0, 0, 10.0), 32.0);
    }
}
