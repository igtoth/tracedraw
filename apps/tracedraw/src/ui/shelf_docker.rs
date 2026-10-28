//! The Shelf docker (Window > Dockers > Shelf, or Add to Shelf on an
//! object's menu): Add Selection, Place and Remove, then the items as
//! tiles with a picture and a name. A click chooses an item; a double
//! click places it in the middle of the view; dragging it onto the
//! drawing places it under the pointer, onto another tile moves it there.
//! An item's menu (right click) places, renames and removes it.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Color32, CursorIcon, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// The picture's side, points.
const THUMB: f32 = 64.0;
const TILE_W: f32 = 80.0;
const TILE_H: f32 = THUMB + 26.0;
/// New pictures drawn per frame (a large shelf fills in over a few).
const THUMBS_PER_FRAME: usize = 6;

pub fn shelf(app: &mut App, ui: &mut Ui) {
    app.shelf.ensure_loaded();
    let has_selection = !app.selection.is_empty();
    let drawing = app.has_document() && !app.show_welcome;
    let chosen = app
        .shelf
        .selected
        .filter(|uid| app.shelf.item(*uid).is_some());
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(has_selection, egui::Button::new(tr("shelf.add_selection")))
            .rich_tip(tr("shelf.add_selection_tip"))
            .clicked()
        {
            app.shelf_add_selection();
        }
        if ui
            .add_enabled(drawing, egui::Button::new(tr("shelf.add_colors")))
            .rich_tip(tr("shelf.add_colors_tip"))
            .clicked()
        {
            app.shelf_add_colors();
        }
        if ui
            .add_enabled(
                chosen.is_some() && drawing,
                egui::Button::new(tr("shelf.place")),
            )
            .clicked()
        {
            if let Some(uid) = chosen {
                app.shelf_place(uid, None);
            }
        }
        if ui
            .add_enabled(chosen.is_some(), egui::Button::new(tr("shelf.remove")))
            .clicked()
        {
            if let Some(uid) = chosen {
                ask_to_remove(app, uid);
            }
        }
    });
    ui.add_space(4.0);
    if app.shelf.items.is_empty() {
        ui.label(egui::RichText::new(tr("shelf.empty_hint")).color(Tokens::text_dim()));
        return;
    }
    let items: Vec<(u64, String)> = app
        .shelf
        .items
        .iter()
        .map(|i| (i.uid, i.name.clone()))
        .collect();
    let mut tiles: Vec<(u64, Rect)> = Vec::with_capacity(items.len());
    let mut budget = THUMBS_PER_FRAME;
    let mut released: Option<u64> = None;
    egui::ScrollArea::vertical()
        .id_salt("shelf_items")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                for (uid, name) in &items {
                    let (rect, released_here) = tile(app, ui, *uid, name, &mut budget);
                    tiles.push((*uid, rect));
                    if released_here {
                        released = Some(*uid);
                    }
                }
            });
        });
    if let Some(uid) = released {
        drop_item(app, ui.ctx(), uid, &tiles);
    }
    if budget == 0 {
        ui.ctx().request_repaint();
    }
}

/// Remove (button or menu): asks first unless Options > Global >
/// Warnings says not to.
pub fn ask_to_remove(app: &mut App, uid: u64) {
    let Some(name) = app.shelf.item(uid).map(|i| i.name.clone()) else {
        return;
    };
    if app.settings.warnings.shelf_remove {
        app.dialog = crate::ui::dialogs::Dialog::ConfirmShelfRemove {
            uid,
            name,
            dont_ask: false,
        };
    } else {
        remove_now(app, uid);
    }
}

/// Take the item off the shelf and say so.
pub fn remove_now(app: &mut App, uid: u64) {
    let name = app.shelf.item(uid).map(|i| i.name.clone());
    if app.shelf_remove(uid) {
        if let Some(name) = name {
            app.status = trf("shelf.removed", &[("name", &name)]);
        }
    }
}

/// The picture of an item, drawn once and kept; none while the frame's
/// budget of new pictures is spent.
fn picture(app: &mut App, ui: &Ui, uid: u64, budget: &mut usize) -> Option<egui::TextureHandle> {
    let px = (THUMB * ui.ctx().pixels_per_point()).round().max(8.0) as u32;
    let cached = app
        .shelf
        .thumbs
        .get(&uid)
        .is_some_and(|(size, _)| *size == px);
    if !cached {
        if *budget == 0 {
            return None;
        }
        *budget -= 1;
    }
    app.shelf_thumbnail(ui.ctx(), uid, px)
}

/// Where a picture of `size` pixels sits, whole and centred, in `frame`.
fn fit(frame: Rect, size: [usize; 2]) -> Rect {
    let (w, h) = (size[0].max(1) as f32, size[1].max(1) as f32);
    let scale = (frame.width() / w).min(frame.height() / h);
    Rect::from_center_size(frame.center(), egui::vec2(w * scale, h * scale))
}

/// One item: its picture and name, chosen or not. Returns its rectangle
/// and whether a drag of it ended this frame.
fn tile(app: &mut App, ui: &mut Ui, uid: u64, name: &str, budget: &mut usize) -> (Rect, bool) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(TILE_W, TILE_H), Sense::click_and_drag());
    let chosen = app.shelf.selected == Some(uid);
    let painter = ui.painter().clone();
    if chosen {
        painter.rect_filled(rect, 2.0, Tokens::tool_active());
    } else if resp.hovered() {
        painter.rect_filled(rect, 2.0, Tokens::tool_hover());
    }
    let frame = Rect::from_min_size(
        rect.min + egui::vec2((TILE_W - THUMB) / 2.0, 4.0),
        Vec2::splat(THUMB),
    );
    painter.rect_filled(frame, 0.0, Color32::WHITE);
    painter.rect_stroke(
        frame,
        0.0,
        Stroke::new(1.0, crate::theme::Tokens::border()),
        egui::StrokeKind::Inside,
    );
    let texture = picture(app, ui, uid, budget);
    if let Some(t) = &texture {
        painter.image(
            t.id(),
            fit(frame.shrink(3.0), t.size()),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    let name_rect = Rect::from_min_max(
        egui::pos2(rect.left() + 2.0, frame.bottom() + 3.0),
        egui::pos2(rect.right() - 2.0, rect.bottom() - 2.0),
    );
    if !rename_field(app, ui, uid, name_rect) {
        let mut job = egui::text::LayoutJob::simple_singleline(
            name.to_string(),
            egui::FontId::proportional(11.5),
            Tokens::text(),
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(name_rect.width());
        let galley = ui.painter().layout_job(job);
        let at = egui::pos2(
            name_rect.center().x - galley.size().x / 2.0,
            name_rect.top(),
        );
        painter.galley(at, galley, Tokens::text());
    }
    let resp = resp.rich_tip(trf("shelf.item_tip", &[("name", name)]));
    if resp.clicked() || resp.secondary_clicked() || resp.drag_started() {
        app.shelf.selected = Some(uid);
    }
    if resp.double_clicked() {
        app.shelf_place(uid, None);
    }
    if resp.drag_started() {
        app.shelf.dragging = Some(uid);
    }
    if app.shelf.dragging == Some(uid) && resp.dragged() {
        ghost(app, ui, texture.as_ref(), frame.size());
    }
    let released = resp.drag_stopped() && app.shelf.dragging == Some(uid);
    egui::Popup::context_menu(&resp)
        .id(egui::Id::new(("shelf_item_menu", uid)))
        .style(crate::ui::menus::menu_popup_style)
        .show(|ui| {
            crate::ui::menus::body(ui, |ui| {
                if crate::ui::menus::item(ui, "shelf.place", "", true) {
                    app.shelf_place(uid, None);
                }
                if crate::ui::menus::item(ui, "shelf.rename", "", true) {
                    app.shelf.selected = Some(uid);
                    app.shelf.renaming = Some((uid, name.to_string()));
                }
                crate::ui::menus::sep(ui);
                if crate::ui::menus::item(ui, "shelf.remove", "", true) {
                    ask_to_remove(app, uid);
                }
            })
        });
    (rect, released)
}

/// The name field while the item is renamed: Enter or leaving the field
/// keeps the name, Esc keeps the old one. False when it is not renamed.
fn rename_field(app: &mut App, ui: &mut Ui, uid: u64, at: Rect) -> bool {
    let Some((who, mut text)) = app.shelf.renaming.take() else {
        return false;
    };
    if who != uid {
        app.shelf.renaming = Some((who, text));
        return false;
    }
    let id = egui::Id::new(("shelf_rename", uid));
    // A child at the name's place: the tile has its room already.
    let mut field = ui.new_child(egui::UiBuilder::new().max_rect(at));
    let resp = field.add_sized(
        at.size(),
        egui::TextEdit::singleline(&mut text)
            .id(id)
            .font(egui::FontId::proportional(11.5))
            .margin(egui::Margin::symmetric(2, 0)),
    );
    if !resp.has_focus() && !resp.lost_focus() {
        resp.request_focus();
    }
    let (enter, esc) = ui.input(|i| {
        (
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
        )
    });
    if esc {
        return true;
    }
    if resp.lost_focus() || enter {
        app.shelf_rename(uid, &text);
        return true;
    }
    app.shelf.renaming = Some((uid, text));
    true
}

/// The item's picture under the pointer while it is dragged, and the
/// pointer saying whether letting go places it.
fn ghost(app: &App, ui: &Ui, texture: Option<&egui::TextureHandle>, size: Vec2) {
    let Some(pos) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    let over_drawing = app.canvas_rect.contains(pos);
    ui.ctx().set_cursor_icon(if over_drawing {
        CursorIcon::Copy
    } else {
        CursorIcon::Grabbing
    });
    let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("shelf_drag"));
    let painter = ui.ctx().layer_painter(layer);
    let r = Rect::from_center_size(pos + egui::vec2(size.x * 0.25, size.y * 0.25), size * 0.75);
    painter.rect_filled(r, 0.0, Color32::from_white_alpha(170));
    if let Some(t) = texture {
        painter.image(
            t.id(),
            fit(r, t.size()),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::from_white_alpha(170),
        );
    }
}

/// A drag of the item ended: on the drawing it is placed under the
/// pointer, on another tile it moves there.
fn drop_item(app: &mut App, ctx: &egui::Context, uid: u64, tiles: &[(u64, Rect)]) {
    app.shelf.dragging = None;
    let Some(pos) = ctx.pointer_latest_pos() else {
        return;
    };
    if app.canvas_rect.contains(pos) {
        let at = app.view.to_page(pos);
        app.shelf_place(uid, Some(at));
        return;
    }
    let target = tiles
        .iter()
        .find(|(other, r)| *other != uid && r.contains(pos))
        .map(|(other, _)| *other);
    if let Some(index) = target.and_then(|other| app.shelf.index_of(other)) {
        app.shelf_move(uid, index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect as PageRect;
    use tracedraw_core::{ShapeId, ShapeKind};

    fn frame(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
    }

    fn add_rect(app: &mut App, x: f64) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: PageRect::new(x, 0.0, x + 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        id
    }

    #[test]
    fn the_docker_lists_items_and_places_them() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        app.show_dockers = true;
        app.docker_tab = crate::app::DockerTab::Shelf;
        // Empty: the hint.
        frame(&ctx, &mut app, vec![]);
        frame(&ctx, &mut app, vec![]);
        add_rect(&mut app, 0.0);
        assert!(app.shelf_add_selection());
        add_rect(&mut app, 30.0);
        assert!(app.shelf_add_selection());
        let first = app.shelf.items[0].uid;
        let second = app.shelf.items[1].uid;
        // Pictures are drawn and kept.
        for _ in 0..3 {
            frame(&ctx, &mut app, vec![]);
        }
        assert!(app.shelf.thumbs.contains_key(&first));
        assert!(app.shelf.thumbs.contains_key(&second));
        // A drop on the drawing places the item under the pointer.
        let count = |app: &App| {
            app.doc()
                .all_layers()
                .map(|l| l.shapes.len())
                .sum::<usize>()
        };
        let before = count(&app);
        let target = app.canvas_rect.center();
        let tiles = vec![(first, egui::Rect::NOTHING), (second, egui::Rect::NOTHING)];
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(target)]);
        drop_item(&mut app, &ctx, second, &tiles);
        assert_eq!(count(&app), before + 1);
        let placed = app.selection_bounds().expect("placed");
        let at = app.view.to_page(target);
        assert!((placed.center().x - at.x).abs() < 1e-3, "{placed:?} {at:?}");
        // A drop on another tile moves the item there.
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(Pos2::new(5.0, 5.0))],
        );
        let tiles = vec![
            (
                first,
                egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(10.0, 10.0)),
            ),
            (second, egui::Rect::NOTHING),
        ];
        drop_item(&mut app, &ctx, second, &tiles);
        assert_eq!(app.shelf.index_of(second), Some(0));
        // Renaming through the field: Enter keeps the name.
        app.shelf.renaming = Some((first, "Badge".into()));
        frame(&ctx, &mut app, vec![]);
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(
            app.shelf.item(first).map(|i| i.name.as_str()),
            Some("Badge")
        );
        assert!(app.shelf.renaming.is_none());
        assert_eq!(
            fit(
                egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(64.0, 64.0)),
                [128, 64]
            )
            .height(),
            32.0
        );
    }
}
