//! The buttons of the Standard and Zoom toolbars as lists the user can
//! change. In customize mode (Tools > Options > Customization open, or Alt
//! held) a button dragged along a bar moves, dragged onto the other bar
//! moves there, and dragged off the bars goes away; a command or a tool
//! dragged from the Commands page onto a bar joins it (#563). Bars Reset
//! (Options > Command Bars, or a bar's menu) brings the first buttons back.

use crate::app::App;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// The bars whose buttons can be changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemBar {
    Standard,
    Zoom,
}

/// A separator between buttons.
pub const SEP: &str = "sep";

/// Prefix of a toolbox tool's button: "tool:" and the tool's id.
pub const TOOL: &str = "tool:";

const STANDARD: [&str; 33] = [
    "new",
    "open",
    "save",
    "git_pull",
    "git_push",
    "print",
    SEP,
    "cut",
    "copy",
    "paste",
    SEP,
    "undo",
    "redo",
    SEP,
    "import",
    "export",
    "publish_pdf",
    SEP,
    "zoom_levels",
    "fullscreen",
    SEP,
    "rulers",
    "grid",
    "guidelines",
    SEP,
    "snap_off",
    "snap_to",
    SEP,
    "options",
    SEP,
    "launch",
    SEP,
    "ai",
];

const ZOOM: [&str; 10] = [
    "zoom_levels",
    "zoom_in",
    "zoom_out",
    SEP,
    "zoom_selected",
    "zoom_fit",
    SEP,
    "zoom_page",
    "zoom_width",
    "zoom_height",
];

impl ItemBar {
    pub const ALL: [ItemBar; 2] = [ItemBar::Standard, ItemBar::Zoom];

    /// The key the bar's list is kept under in the settings.
    pub fn id(self) -> &'static str {
        match self {
            ItemBar::Standard => "standard",
            ItemBar::Zoom => "zoom",
        }
    }

    /// The buttons the bar comes with.
    pub fn defaults(self) -> &'static [&'static str] {
        match self {
            ItemBar::Standard => &STANDARD,
            ItemBar::Zoom => &ZOOM,
        }
    }
}

/// Every built-in button of the bars (each once, the separator first):
/// what the Commands page offers to drag back onto a bar, so a button
/// taken off can always be put back (#836).
pub fn all_buttons() -> Vec<&'static str> {
    let mut out = vec![SEP];
    for b in ItemBar::ALL {
        for id in b.defaults() {
            if !out.contains(id) {
                out.push(id);
            }
        }
    }
    out
}

/// Whether `id` is one of [`all_buttons`].
pub fn is_button(id: &str) -> bool {
    all_buttons().contains(&id)
}

/// The bar's buttons, in order.
pub fn items(app: &App, bar: ItemBar) -> Vec<String> {
    match app.settings.bar_items.get(bar.id()) {
        Some(list) => list.clone(),
        None => bar.defaults().iter().map(|s| s.to_string()).collect(),
    }
}

/// Keep the bar's buttons; the first ones are kept as nothing.
pub fn set_items(app: &mut App, bar: ItemBar, list: Vec<String>) {
    if list
        .iter()
        .map(String::as_str)
        .eq(bar.defaults().iter().copied())
    {
        app.settings.bar_items.remove(bar.id());
    } else {
        app.settings.bar_items.insert(bar.id().to_string(), list);
    }
}

/// Every bar with its first buttons.
pub fn reset(app: &mut App) {
    app.settings.bar_items.clear();
}

/// A button being dragged: what it is, and the bar and place it left
/// (none when it comes from the Commands page).
#[derive(Clone, Debug, PartialEq)]
pub struct ItemDrag {
    pub item: String,
    pub from: Option<(ItemBar, usize)>,
}

/// Where a bar and its buttons were drawn this frame.
#[derive(Clone, Debug)]
pub struct Slot {
    pub bar: ItemBar,
    pub rect: Rect,
    pub items: Vec<Rect>,
}

/// Buttons can be moved now: the Customization pages of the options are
/// open, or Alt is held.
pub fn customizing(app: &App, ctx: &egui::Context) -> bool {
    let options = matches!(app.dialog, crate::ui::dialogs::Dialog::Options)
        && app.options_page.set() == crate::ui::options::OptionsSet::Customization;
    options || ctx.input(|i| i.modifiers.alt)
}

/// Draw `bar`'s buttons with `draw` (each one's id, the bar's id as a
/// salt), and in customize mode let each be dragged.
pub fn draw_bar(app: &mut App, ui: &mut Ui, bar: ItemBar, draw: fn(&mut App, &mut Ui, &str, &str)) {
    let custom = customizing(app, ui.ctx());
    // The first buttons are static: no copy of the list each frame.
    let changed = app.settings.bar_items.get(bar.id()).cloned();
    let list: Vec<&str> = match &changed {
        Some(v) => v.iter().map(String::as_str).collect(),
        None => bar.defaults().to_vec(),
    };
    let gap = ui.spacing().item_spacing.x;
    let mut rects = Vec::with_capacity(list.len());
    for (i, id) in list.iter().enumerate() {
        // The button spans what the cursor moved by (a row of buttons).
        let x0 = ui.cursor().min.x;
        draw(app, ui, bar.id(), id);
        let x1 = (ui.cursor().min.x - gap).max(x0);
        let rows = ui.min_rect();
        let rect = Rect::from_min_max(Pos2::new(x0, rows.top()), Pos2::new(x1, rows.bottom()));
        if custom && rect.width() > 0.0 {
            // Over the button, so a press moves it rather than runs it.
            let resp = ui.interact(
                rect,
                egui::Id::new(("bar_item", bar.id(), i)),
                Sense::click_and_drag(),
            );
            let resp = crate::ui::tips::RichTip::rich_tip(
                resp,
                crate::ui::tips::pair("options.bar_item_move", "options.bar_item_move_tip"),
            );
            if resp.hovered() {
                ui.painter().rect_stroke(
                    rect.shrink(0.5),
                    0.0,
                    Stroke::new(1.0, crate::theme::Tokens::ACCENT),
                    egui::StrokeKind::Inside,
                );
            }
            if resp.drag_started() {
                app.bar_item_drag = Some(ItemDrag {
                    item: id.to_string(),
                    from: Some((bar, i)),
                });
            }
        }
        rects.push(rect);
    }
    let mut rect = ui.min_rect();
    if rect.width() < 40.0 {
        rect.set_width(40.0);
    }
    app.bar_slots.push(Slot {
        bar,
        rect,
        items: rects,
    });
}

/// The bar and place a button dropped at `p` goes to, and where to mark
/// it (the x of the gap).
pub fn drop_target(slots: &[Slot], p: Pos2) -> Option<(ItemBar, usize, f32)> {
    let slot = slots.iter().find(|s| s.rect.expand(4.0).contains(p))?;
    let index = slot.items.iter().filter(|r| r.center().x < p.x).count();
    let x = match (
        index.checked_sub(1).and_then(|i| slot.items.get(i)),
        slot.items.get(index),
    ) {
        (Some(a), Some(b)) => (a.right() + b.left()) / 2.0,
        (Some(a), None) => a.right() + 1.0,
        (None, Some(b)) => b.left() - 1.0,
        (None, None) => slot.rect.left() + 2.0,
    };
    Some((slot.bar, index, x))
}

/// Put the dragged button at `to` (removed from where it was), or drop it
/// when `to` is none. A command already on the bar it joins only moves.
pub fn apply_drop(app: &mut App, drag: &ItemDrag, to: Option<(ItemBar, usize)>) {
    let mut to = to;
    if let Some((bar, i)) = drag.from {
        let mut list = items(app, bar);
        if list.get(i) != Some(&drag.item) {
            return;
        }
        list.remove(i);
        set_items(app, bar, list);
        if let Some((tb, j)) = to.as_mut() {
            if *tb == bar && *j > i {
                *j -= 1;
            }
        }
    }
    let Some((bar, j)) = to else {
        return;
    };
    let mut list = items(app, bar);
    let mut j = j.min(list.len());
    if drag.from.is_none() && drag.item != SEP {
        if let Some(k) = list.iter().position(|x| *x == drag.item) {
            list.remove(k);
            if j > k {
                j -= 1;
            }
        }
    }
    list.insert(j, drag.item.clone());
    set_items(app, bar, list);
}

/// While a button is dragged: it follows the pointer, a line marks where
/// it would go; on release it goes there (or off the bar). Call once a
/// frame after the bars are drawn.
pub fn drop_overlay(app: &mut App, ctx: &egui::Context) {
    let slots = std::mem::take(&mut app.bar_slots);
    if let Some(drag) = app.bar_item_drag.clone() {
        drag_frame(app, ctx, &slots, &drag);
    }
    app.shown_bar_slots = slots;
}

fn drag_frame(app: &mut App, ctx: &egui::Context, slots: &[Slot], drag: &ItemDrag) {
    let (pointer, down) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down()));
    let target = pointer.and_then(|p| drop_target(slots, p));
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("bar_item_drag"),
    ));
    if let Some(p) = pointer {
        let r = Rect::from_center_size(p + Vec2::new(14.0, 14.0), Vec2::splat(26.0));
        painter.rect_filled(r, 2.0, crate::theme::Tokens::panel().gamma_multiply(0.9));
        painter.rect_stroke(
            r,
            2.0,
            Stroke::new(1.0, crate::theme::Tokens::border()),
            egui::StrokeKind::Inside,
        );
        picture(
            &painter,
            r.shrink(4.0),
            &drag.item,
            crate::theme::Tokens::icon(),
        );
    }
    if let Some((bar, _, x)) = target {
        if let Some(s) = slots.iter().find(|s| s.bar == bar) {
            painter.line_segment(
                [
                    Pos2::new(x, s.rect.top() + 2.0),
                    Pos2::new(x, s.rect.bottom() - 2.0),
                ],
                Stroke::new(2.0, crate::theme::Tokens::ACCENT),
            );
        }
    }
    if !down {
        app.bar_item_drag = None;
        apply_drop(app, drag, target.map(|(b, i, _)| (b, i)));
    }
}

/// The picture of a button by its id, for the dragged copy: a command's
/// menu picture or a tool's icon; a separator is a bar.
pub fn picture(painter: &egui::Painter, r: Rect, item: &str, color: Color32) {
    if item == SEP {
        painter.line_segment([r.center_top(), r.center_bottom()], Stroke::new(1.0, color));
    } else if let Some(t) = tool_of(item) {
        crate::ui::icons::draw(painter, r, t, color);
    } else if !crate::ui::menus::draw_menu_picture(painter, r, item, color) {
        if let Some(pic) = crate::ui::toolbar::item_pic(item) {
            crate::ui::propbar::draw_pic(painter, r, pic, color);
        } else if let Some(a) = crate::ui::toolbar::item_action(item) {
            crate::ui::icons::draw_action(painter, r, a, color);
        } else {
            painter.rect_stroke(
                r.shrink(3.0),
                1.0,
                Stroke::new(1.0, color),
                egui::StrokeKind::Inside,
            );
        }
    }
}

/// The tool of a "tool:" button.
pub fn tool_of(item: &str) -> Option<crate::tools::Tool> {
    let id = item.strip_prefix(TOOL)?;
    crate::tools::Tool::ALL.into_iter().find(|t| t.id() == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(bar: ItemBar, n: usize, y: f32) -> Slot {
        let items: Vec<Rect> = (0..n)
            .map(|i| Rect::from_min_size(Pos2::new(i as f32 * 30.0, y), Vec2::splat(28.0)))
            .collect();
        Slot {
            bar,
            rect: Rect::from_min_size(Pos2::new(0.0, y), Vec2::new(n as f32 * 30.0, 28.0)),
            items,
        }
    }

    /// A button moves along its bar, moves to the other bar, goes away
    /// when dropped off them; a command from the Commands page joins a
    /// bar once; the first buttons are kept as nothing (#563).
    #[test]
    fn buttons_move_join_and_leave_the_bars() {
        let mut app = App::headless();
        let first = items(&app, ItemBar::Zoom);
        assert_eq!(first[1], "zoom_in");
        // zoom_in to the end of its bar.
        let drag = ItemDrag {
            item: "zoom_in".into(),
            from: Some((ItemBar::Zoom, 1)),
        };
        apply_drop(&mut app, &drag, Some((ItemBar::Zoom, first.len())));
        let now = items(&app, ItemBar::Zoom);
        assert_eq!(now.last().map(String::as_str), Some("zoom_in"));
        assert_eq!(now.len(), first.len());
        // Dropped back where it was: nothing changes.
        let n = now.len();
        let drag = ItemDrag {
            item: "zoom_in".into(),
            from: Some((ItemBar::Zoom, n - 1)),
        };
        apply_drop(&mut app, &drag, Some((ItemBar::Zoom, n)));
        assert_eq!(items(&app, ItemBar::Zoom), now);
        // To the Standard bar, first.
        apply_drop(&mut app, &drag, Some((ItemBar::Standard, 0)));
        assert_eq!(items(&app, ItemBar::Standard)[0], "zoom_in");
        assert!(!items(&app, ItemBar::Zoom).contains(&"zoom_in".to_string()));
        // Off the bars: gone.
        let drag = ItemDrag {
            item: "zoom_in".into(),
            from: Some((ItemBar::Standard, 0)),
        };
        apply_drop(&mut app, &drag, None);
        assert!(!items(&app, ItemBar::Standard).contains(&"zoom_in".to_string()));
        // A command joins, once.
        let cmd = ItemDrag {
            item: "menu.object.group_group".into(),
            from: None,
        };
        apply_drop(&mut app, &cmd, Some((ItemBar::Zoom, 0)));
        apply_drop(&mut app, &cmd, Some((ItemBar::Zoom, 3)));
        let z = items(&app, ItemBar::Zoom);
        assert_eq!(
            z.iter().filter(|x| *x == "menu.object.group_group").count(),
            1
        );
        assert_eq!(z[2], "menu.object.group_group");
        // Dropped on nothing from the Commands page: nothing.
        let before = app.settings.bar_items.clone();
        apply_drop(&mut app, &cmd, None);
        assert_eq!(app.settings.bar_items, before);
        reset(&mut app);
        assert_eq!(items(&app, ItemBar::Zoom), first);
        assert!(app.settings.bar_items.is_empty());
    }

    #[test]
    fn the_drop_place_is_the_gap_under_the_pointer() {
        let slots = vec![
            slot(ItemBar::Standard, 3, 0.0),
            slot(ItemBar::Zoom, 2, 40.0),
        ];
        assert_eq!(
            drop_target(&slots, Pos2::new(5.0, 10.0)).map(|t| (t.0, t.1)),
            Some((ItemBar::Standard, 0))
        );
        assert_eq!(
            drop_target(&slots, Pos2::new(50.0, 10.0)).map(|t| (t.0, t.1)),
            Some((ItemBar::Standard, 2))
        );
        assert_eq!(
            drop_target(&slots, Pos2::new(50.0, 50.0)).map(|t| (t.0, t.1)),
            Some((ItemBar::Zoom, 2))
        );
        assert!(drop_target(&slots, Pos2::new(500.0, 300.0)).is_none());
        assert_eq!(tool_of("tool:rectangle").map(|t| t.id()), Some("rectangle"));
    }
}
