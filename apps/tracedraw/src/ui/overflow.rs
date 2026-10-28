//! Bars longer than the room they get. The bar shows what fits; a "»"
//! button at its end brings the next items into view (the bar slides,
//! cut between items) and a "«" button at its start brings the first
//! ones back; the mouse wheel over the bar slides it too. Every item stays
//! where it is in the bar and works as there: the bar is drawn once. The
//! toolbox on a side does the same up and down. See
//! docs/behavior/toolbars.md.

use crate::ui::tips::RichTip;
use egui::{Align, Layout, Pos2, Rect, Sense, Ui, UiBuilder, Vec2};

/// The size of the « and » buttons across the bar.
pub const BUTTON: f32 = 18.0;

/// What a bar measured in the frame before.
#[derive(Debug, Clone, Default, PartialEq)]
struct State {
    /// The bar's whole length.
    content: f32,
    /// How far the bar is slid.
    offset: f32,
    /// The items' extents along the bar, from its start.
    items: Vec<(f32, f32)>,
}

/// Areas that hold other items (a bar's own hover area, a group) are not
/// items: their children are.
fn items_only(items: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut out: Vec<(f32, f32)> = items
        .iter()
        .copied()
        .filter(|&(a, b)| {
            !items
                .iter()
                .any(|&(c, d)| (c, d) != (a, b) && c >= a - 0.5 && d <= b + 0.5 && d - c < b - a)
        })
        .collect();
    out.sort_by(|x, y| x.0.total_cmp(&y.0));
    out
}

/// The room the items get when the bar is longer than `room`: less the
/// buttons at both ends.
fn view(room: f32) -> f32 {
    (room - 2.0 * BUTTON).max(1.0)
}

/// The largest slide: the bar's end just inside the room.
pub fn max_offset(content: f32, room: f32) -> f32 {
    if content <= room + 0.5 {
        0.0
    } else {
        (content - view(room)).max(0.0)
    }
}

/// The slide after "»": the first item not wholly in view comes first.
pub fn next_offset(items: &[(f32, f32)], content: f32, room: f32, offset: f32) -> f32 {
    let end = offset + view(room);
    let items = items_only(items);
    let next = items
        .iter()
        .find(|&&(a, b)| b > end + 0.5 && a > offset + 0.5)
        .map(|&(a, _)| a)
        .unwrap_or(end);
    next.min(max_offset(content, room))
}

/// The slide after "«": as far back as keeps the first item now in view
/// at the end of the room, cut between items.
pub fn previous_offset(items: &[(f32, f32)], room: f32, offset: f32) -> f32 {
    let target = (offset - view(room)).max(0.0);
    if target <= 0.5 {
        return 0.0;
    }
    let items = items_only(items);
    items
        .iter()
        .map(|&(a, _)| a)
        .find(|&a| a >= target - 0.5)
        .unwrap_or(target)
        .min(offset)
}

/// Draw a horizontal bar with `add`, sliding when it is longer than the
/// room it gets; the bar's whole length.
pub fn row(ui: &mut Ui, id_salt: &str, add: impl FnOnce(&mut Ui)) -> f32 {
    strip(ui, id_salt, true, add)
}

/// The vertical counterpart, for a column of tool buttons.
pub fn column(ui: &mut Ui, id_salt: &str, add: impl FnOnce(&mut Ui)) -> f32 {
    strip(ui, id_salt, false, add)
}

fn strip(ui: &mut Ui, id_salt: &str, horizontal: bool, add: impl FnOnce(&mut Ui)) -> f32 {
    let id = ui.make_persistent_id(("bar_overflow", id_salt));
    let mut st: State = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    let full = ui.available_rect_before_wrap();
    let along = |v: Vec2| if horizontal { v.x } else { v.y };
    let room = along(full.size()).max(0.0);
    let long = st.content > room + 0.5;
    st.offset = st.offset.clamp(0.0, max_offset(st.content, room));
    let lead = if long { BUTTON } else { 0.0 };
    let shown = if long { view(room) } else { room };
    // Laid out as the bar was, slid by the offset, clipped to the room
    // left between the buttons across the whole panel.
    let panel = ui.clip_rect();
    let (origin, size, clip, layout) = if horizontal {
        (
            Pos2::new(full.min.x + lead - st.offset, full.min.y),
            Vec2::new(100_000.0, full.height().max(1.0)),
            Rect::from_min_max(
                Pos2::new(full.min.x + lead, panel.min.y),
                Pos2::new(full.min.x + lead + shown, panel.max.y),
            ),
            Layout::left_to_right(Align::Min),
        )
    } else {
        (
            Pos2::new(full.min.x, full.min.y + lead - st.offset),
            Vec2::new(full.width().max(1.0), 100_000.0),
            Rect::from_min_max(
                Pos2::new(panel.min.x, full.min.y + lead),
                Pos2::new(panel.max.x, full.min.y + lead + shown),
            ),
            Layout::top_down(Align::Center),
        )
    };
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_size(origin, size))
            .layout(layout)
            .id_salt(id.with("bar")),
    );
    child.set_clip_rect(clip.intersect(panel));
    add(&mut child);
    let used = child.min_rect();
    let content = if horizontal {
        used.right() - origin.x
    } else {
        used.bottom() - origin.y
    };
    let layer = child.layer_id();
    let items: Vec<(f32, f32)> = ui.ctx().viewport(|v| {
        v.this_pass
            .widgets
            .get_layer(layer)
            .filter(|w| {
                let c = w.rect.center();
                if horizontal {
                    w.rect.min.x >= origin.x - 0.5
                        && c.y >= used.min.y - 0.5
                        && c.y <= used.max.y + 0.5
                } else {
                    w.rect.min.y >= origin.y - 0.5
                        && c.x >= used.min.x - 0.5
                        && c.x <= used.max.x + 0.5
                }
            })
            .map(|w| {
                if horizontal {
                    (w.rect.min.x - origin.x, w.rect.max.x - origin.x)
                } else {
                    (w.rect.min.y - origin.y, w.rect.max.y - origin.y)
                }
            })
            .collect()
    });
    // The space the bar takes in its panel.
    let taken = if horizontal {
        Rect::from_min_size(
            Pos2::new(full.min.x, used.min.y),
            Vec2::new(content.min(room), used.height().max(1.0)),
        )
    } else {
        Rect::from_min_size(
            Pos2::new(used.min.x, full.min.y),
            Vec2::new(used.width().max(1.0), content.min(room)),
        )
    };
    let bar_resp = ui.allocate_rect(taken, Sense::hover());
    let mut offset = st.offset;
    if content > room + 0.5 {
        if long != (content > room + 0.5) {
            ui.ctx().request_repaint();
        }
        let tr = crate::i18n::tr;
        let (back, forward, back_tip, forward_tip) = if horizontal {
            (
                "\u{ab}",
                "\u{bb}",
                tr("bars.earlier_items"),
                tr("bars.more_items"),
            )
        } else {
            (
                "\u{25b4}",
                "\u{25be}",
                tr("bars.earlier_items"),
                tr("bars.more_items"),
            )
        };
        let cross = if horizontal {
            used.height()
        } else {
            used.width()
        }
        .clamp(16.0, 28.0);
        let (back_rect, forward_rect) = if horizontal {
            (
                Rect::from_center_size(
                    Pos2::new(full.min.x + BUTTON / 2.0, used.center().y),
                    Vec2::new(BUTTON, cross),
                ),
                Rect::from_center_size(
                    Pos2::new(full.min.x + room - BUTTON / 2.0, used.center().y),
                    Vec2::new(BUTTON, cross),
                ),
            )
        } else {
            (
                Rect::from_center_size(
                    Pos2::new(used.center().x, full.min.y + BUTTON / 2.0),
                    Vec2::new(cross, BUTTON),
                ),
                Rect::from_center_size(
                    Pos2::new(used.center().x, full.min.y + room - BUTTON / 2.0),
                    Vec2::new(cross, BUTTON),
                ),
            )
        };
        let can_back = offset > 0.5;
        let can_forward = offset < max_offset(content, room) - 0.5;
        if ui
            .put(
                back_rect,
                egui::Button::new(back).frame(false).sense(if can_back {
                    Sense::click()
                } else {
                    Sense::hover()
                }),
            )
            .rich_tip(back_tip)
            .clicked()
            && can_back
        {
            offset = previous_offset(&items, room, offset);
        }
        if ui
            .put(
                forward_rect,
                egui::Button::new(forward)
                    .frame(false)
                    .sense(if can_forward {
                        Sense::click()
                    } else {
                        Sense::hover()
                    }),
            )
            .rich_tip(forward_tip)
            .clicked()
            && can_forward
        {
            offset = next_offset(&items, content, room, offset);
        }
        // The mouse wheel over the bar slides it.
        let over = ui.rect_contains_pointer(full.intersect(panel)) || bar_resp.hovered();
        if over {
            let wheel = ui.input(|i| i.smooth_scroll_delta);
            let delta = if horizontal {
                if wheel.x != 0.0 {
                    wheel.x
                } else {
                    wheel.y
                }
            } else {
                wheel.y
            };
            if delta != 0.0 {
                offset -= delta;
            }
        }
        offset = offset.clamp(0.0, max_offset(content, room));
        if offset != st.offset {
            ui.ctx().request_repaint();
        }
    } else {
        offset = 0.0;
    }
    ui.data_mut(|d| {
        d.insert_temp(
            id,
            State {
                content,
                offset,
                items,
            },
        )
    });
    content
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<(f32, f32)> {
        (0..10)
            .map(|i| (i as f32 * 40.0, i as f32 * 40.0 + 40.0))
            .collect()
    }

    #[test]
    fn a_bar_that_fits_does_not_slide() {
        assert_eq!(max_offset(300.0, 300.0), 0.0);
        assert_eq!(max_offset(300.0, 400.0), 0.0);
    }

    #[test]
    fn forward_brings_the_first_hidden_item_to_the_start() {
        // 400 long in 200 of room: 164 between the buttons, items 40
        // apart. In view at first: 0..164, the item at 160 is cut, so
        // it comes first next.
        let it = items();
        assert_eq!(next_offset(&it, 400.0, 200.0, 0.0), 160.0);
        // Never past the end.
        assert_eq!(max_offset(400.0, 200.0), 400.0 - 164.0);
        assert_eq!(next_offset(&it, 400.0, 200.0, 160.0), 236.0);
    }

    #[test]
    fn back_returns_between_items_and_to_the_start() {
        let it = items();
        assert_eq!(previous_offset(&it, 200.0, 236.0), 80.0);
        assert_eq!(previous_offset(&it, 200.0, 80.0), 0.0);
    }

    #[test]
    fn areas_holding_items_are_not_cut_points() {
        let mut it = items();
        it.push((0.0, 400.0));
        it.push((120.0, 240.0));
        assert_eq!(next_offset(&it, 400.0, 200.0, 0.0), 160.0);
    }
}
