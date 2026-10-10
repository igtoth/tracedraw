//! The menu bar, item for item as in the target design's 2019
//! workspace. Items without an implementation are shown disabled so the
//! layout is complete; wiring them is tracked in docs/parity.md.

use crate::app::{App, DockerTab};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::dialogs::Dialog;
use egui::Ui;
use tracedraw_core::document::ShapeKind;
use tracedraw_core::Command;

const MENU_WIDTH: f32 = 250.0;
/// Menu rows: 26 px tall, a 30 px grey gutter
/// for icons and check marks, labels from 38 px, shortcuts right-aligned
/// 28 px from the edge, 13 px text.
const ROW_HEIGHT: f32 = 26.0;
const GUTTER: f32 = 30.0;
const LABEL_X: f32 = 38.0;
const SHORTCUT_PAD: f32 = 28.0;
const FONT: f32 = 13.0;
const SEPARATOR_HEIGHT: f32 = 7.0;
const MENU_TEXT: egui::Color32 = egui::Color32::BLACK;
const MENU_TEXT_OFF: egui::Color32 = egui::Color32::from_gray(0xC8);
const MENU_GUTTER: egui::Color32 = egui::Color32::from_gray(0xF4);
const MENU_BORDER: egui::Color32 = egui::Color32::from_gray(0xB2);
const MENU_LINE: egui::Color32 = egui::Color32::from_gray(0xD8);
/// Hovered rows and open menu titles: a light blue box with a blue frame.
const MENU_HOVER: egui::Color32 = egui::Color32::from_rgb(0xE0, 0xF0, 0xFF);
const MENU_HOVER_LINE: egui::Color32 = egui::Color32::from_rgb(0x00, 0xAD, 0xFE);
const SUBMENU_ARROW: egui::Color32 = egui::Color32::from_gray(120);

/// The style of menu pop-ups: white, a one-pixel grey frame, square
/// corners, rows touching.
pub(crate) fn menu_popup_style(style: &mut egui::Style) {
    egui::containers::menu::menu_style(style);
    style.visuals.window_fill = egui::Color32::WHITE;
    style.visuals.window_stroke = egui::Stroke::new(1.0, MENU_BORDER);
    style.visuals.menu_corner_radius = egui::CornerRadius::ZERO;
    style.visuals.popup_shadow = egui::epaint::Shadow {
        offset: [2, 2],
        blur: 4,
        spread: 0,
        color: egui::Color32::from_black_alpha(30),
    };
    style.spacing.menu_margin = egui::Margin {
        left: 0,
        right: 0,
        top: 2,
        bottom: 2,
    };
    style.spacing.item_spacing.y = 0.0;
    style.override_font_id = Some(egui::FontId::proportional(FONT));
}

/// The grey gutter down the left of a menu body, under its rows: reserved
/// first, sized once the rows are laid out.
pub(crate) fn body<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let width_id = ui.id().with("menu_width");
    if let Some(w) = ui.ctx().data(|d| d.get_temp::<f32>(width_id)) {
        ui.set_min_width(w);
    }
    let slot = ui.painter().add(egui::Shape::Noop);
    let r = add(ui);
    let rows = ui.min_rect();
    ui.ctx().data_mut(|d| d.insert_temp(width_id, rows.width()));
    let gutter = egui::Rect::from_min_max(
        egui::pos2(rows.left(), rows.top() - 2.0),
        egui::pos2(rows.left() + GUTTER, rows.bottom() + 2.0),
    );
    ui.painter()
        .set(slot, egui::Shape::rect_filled(gutter, 0.0, MENU_GUTTER));
    r
}

/// One menu row drawn: icon or check mark in the
/// gutter, label, shortcut right-aligned, hover box.
fn menu_row(
    ui: &mut Ui,
    label: &str,
    shortcut: &str,
    enabled: bool,
    mark: Mark,
) -> (egui::Response, egui::layers::ShapeIdx) {
    // Fixed width like a native menu; longer translations widen their row.
    let needed = ui.fonts_mut(|f| {
        let l = f
            .layout_no_wrap(
                label.to_string(),
                egui::FontId::proportional(FONT),
                MENU_TEXT,
            )
            .size()
            .x;
        let s = if shortcut.is_empty() {
            0.0
        } else {
            f.layout_no_wrap(
                shortcut.to_string(),
                egui::FontId::proportional(FONT),
                MENU_TEXT,
            )
            .size()
            .x + 32.0
        };
        LABEL_X + l + s + SHORTCUT_PAD
    });
    // As wide as the widest row (remembered by `body` from the last frame).
    let width = MENU_WIDTH.max(needed).max(ui.min_rect().width());
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(width, ROW_HEIGHT),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let painter = ui.painter();
    let slot = painter.add(egui::Shape::Noop);
    if enabled && resp.hovered() {
        painter.set(slot, highlight(rect));
    }
    let color = if enabled { MENU_TEXT } else { MENU_TEXT_OFF };
    let mid = egui::pos2(rect.left() + GUTTER / 2.0, rect.center().y);
    match mark {
        Mark::Check(true) => {
            // A bold check mark.
            let s = egui::Stroke::new(2.0, color);
            painter.line_segment(
                [mid + egui::vec2(-5.0, 0.0), mid + egui::vec2(-1.5, 3.5)],
                s,
            );
            painter.line_segment(
                [mid + egui::vec2(-1.5, 3.5), mid + egui::vec2(5.0, -4.5)],
                s,
            );
        }
        Mark::Radio(true) => {
            painter.circle_filled(mid, 3.5, color);
        }
        _ => {}
    }
    painter.text(
        egui::pos2(rect.left() + LABEL_X, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(FONT),
        color,
    );
    if !shortcut.is_empty() {
        painter.text(
            egui::pos2(rect.right() - SHORTCUT_PAD, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            shortcut,
            egui::FontId::proportional(FONT),
            color,
        );
    }
    if mark == Mark::Submenu {
        let c = egui::pos2(rect.right() - 13.0, rect.center().y);
        painter.add(egui::epaint::PathShape::convex_polygon(
            vec![
                c + egui::vec2(-2.0, -4.0),
                c + egui::vec2(2.0, 0.0),
                c + egui::vec2(-2.0, 4.0),
            ],
            if enabled {
                SUBMENU_ARROW
            } else {
                MENU_TEXT_OFF
            },
            egui::Stroke::NONE,
        ));
    }
    (resp, slot)
}

/// The box behind a hovered row or an open submenu's row.
fn highlight(rect: egui::Rect) -> egui::Shape {
    egui::Shape::Rect(egui::epaint::RectShape::new(
        rect.shrink2(egui::vec2(2.0, 0.0)),
        0.0,
        MENU_HOVER,
        egui::Stroke::new(1.0, MENU_HOVER_LINE),
        egui::StrokeKind::Inside,
    ))
}

/// What a row shows in its gutter and at its right end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    None,
    Check(bool),
    Radio(bool),
    Submenu,
}

/// A separator: a light line from the gutter to the right edge.
pub(crate) fn sep(ui: &mut Ui) {
    let width = ui.min_rect().width().max(MENU_WIDTH);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, SEPARATOR_HEIGHT), egui::Sense::hover());
    let y = rect.center().y.round() - 0.5;
    ui.painter().rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(rect.left() + GUTTER + 2.0, y),
            egui::pos2(rect.right(), y + 1.0),
        ),
        0.0,
        MENU_LINE,
    );
}

/// A menu item; `key` is an i18n key. Returns true when clicked. Items
/// with a toolbar counterpart show its icon in the gutter.
pub(crate) fn item(ui: &mut Ui, key: &str, shortcut: &str, enabled: bool) -> bool {
    item_text(ui, &tr(key), shortcut, enabled, menu_icon(key), key)
}

/// A menu item with a ready label (a style name, a file).
pub(crate) fn item_label(ui: &mut Ui, label: &str, shortcut: &str, enabled: bool) -> bool {
    item_text(ui, label, shortcut, enabled, None, label)
}

/// A menu item with a ready label and an optional icon; `id` names it for
/// the test replay.
fn item_text(
    ui: &mut Ui,
    label: &str,
    shortcut: &str,
    enabled: bool,
    icon: Option<crate::ui::icons::Action>,
    id: &str,
) -> bool {
    let (r, _) = menu_row(ui, label, shortcut, enabled, Mark::None);
    #[cfg(test)]
    if replay::hit(id, enabled) {
        return true;
    }
    let _ = id;
    let ir = egui::Rect::from_center_size(
        egui::pos2(r.rect.left() + GUTTER / 2.0, r.rect.center().y),
        egui::vec2(16.0, 16.0),
    );
    let color = if enabled { Tokens::ICON } else { MENU_TEXT_OFF };
    if let Some(a) = icon {
        crate::ui::icons::draw_action(ui.painter(), ir, a, color);
    } else if let Some(p) = menu_pic(id) {
        crate::ui::propbar::draw_pic(ui.painter(), ir.expand(2.0), p, color);
    }
    if r.clicked() {
        ui.close();
        true
    } else {
        false
    }
}

/// Toolbar icon for a menu item, when it has one.
fn menu_icon(key: &str) -> Option<crate::ui::icons::Action> {
    use crate::ui::icons::Action;
    Some(match key {
        "menu.file.new" => Action::New,
        "menu.file.open" => Action::Open,
        "menu.file.save" => Action::Save,
        "menu.file.print" => Action::Print,
        "menu.file.import" => Action::Import,
        "menu.file.export" => Action::Export,
        "menu.file.publish_to_pdf" => Action::Pdf,
        "menu.edit.cut" => Action::Cut,
        "menu.edit.copy" => Action::Copy,
        "menu.edit.paste" => Action::Paste,
        "menu.edit.undo" => Action::Undo,
        "menu.edit.redo" => Action::Redo,
        "menu.tools.options_app" => Action::Options,
        "menu.view.fullscreen_preview" => Action::Fullscreen,
        "menu.help.welcome_screen" => Action::Welcome,
        _ => return None,
    })
}

/// Property bar picture for a menu item, when it has one.
fn menu_pic(key: &str) -> Option<crate::ui::propbar::Pic> {
    use crate::ui::propbar::Pic;
    Some(match key {
        "menu.view.zoom_in" => Pic::ZoomIn,
        "menu.view.zoom_out" => Pic::ZoomOut,
        "menu.view.zoom_to_fit" => Pic::ZoomAll,
        _ => return None,
    })
}

/// A checkable item (check mark in the gutter when `on`).
pub(crate) fn check(ui: &mut Ui, key: &str, shortcut: &str, on: bool) -> bool {
    mark_item(ui, key, shortcut, Mark::Check(on))
}

/// One of a set of exclusive items (a dot in the gutter when `on`).
pub(crate) fn radio(ui: &mut Ui, key: &str, shortcut: &str, on: bool) -> bool {
    mark_item(ui, key, shortcut, Mark::Radio(on))
}

fn mark_item(ui: &mut Ui, key: &str, shortcut: &str, mark: Mark) -> bool {
    let (r, _) = menu_row(ui, &tr(key), shortcut, true, mark);
    #[cfg(test)]
    if replay::hit(key, true) {
        return true;
    }
    if r.clicked() {
        ui.close();
        true
    } else {
        false
    }
}

fn todo(ui: &mut Ui, key: &str, shortcut: &str) {
    let _ = item(ui, key, shortcut, false);
}

fn todo_sub(ui: &mut Ui, key: &str) {
    menu_row(ui, &tr(key), "", false, Mark::Submenu);
}

/// A submenu row: the submenu opens beside it while it is hovered.
pub(crate) fn sub<R>(ui: &mut Ui, key: &str, add: impl FnOnce(&mut Ui) -> R) {
    #[cfg(test)]
    if replay::active() {
        // Replaying: submenus are laid out inline so their items count.
        let _ = add(ui);
        return;
    }
    let (resp, slot) = menu_row(ui, &tr(key), "", true, Mark::Submenu);
    let open = egui::containers::menu::MenuState::from_ui(ui, |state, _| {
        state.open_item == Some(egui::containers::menu::SubMenu::id_from_widget_id(resp.id))
    });
    if open {
        ui.painter().set(slot, highlight(resp.rect));
    }
    egui::containers::menu::SubMenu::new().show(ui, &resp, |ui| {
        ui.set_min_width(MENU_WIDTH);
        body(ui, add)
    });
}

/// Test hook: "click" the n-th enabled menu row of a menu body without a
/// pointer, so every menu command can be exercised headlessly.
#[cfg(test)]
pub(crate) mod replay {
    use std::cell::{Cell, RefCell};

    thread_local! {
        static TARGET: Cell<Option<usize>> = const { Cell::new(None) };
        static COUNTER: Cell<usize> = const { Cell::new(0) };
        static HIT: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    /// Items that would block on a native file chooser, start another
    /// program (an editor or the web browser) or close the window.
    const SKIP: [&str; 14] = [
        "menu.file.send_to_mail",
        "menu.help.updates",
        "menu.help.community",
        "menu.help.support",
        "menu.file.open",
        "menu.file.save",
        "menu.file.save_as",
        "menu.file.save_as_template",
        "menu.file.import",
        "menu.file.publish_to_pdf",
        "menu.file.exit",
        "menu.bitmaps.edit_bitmap",
        "menu.tools.scripts_run",
        "menu.window.palette_open",
    ];

    pub fn start(target: usize) {
        TARGET.set(Some(target));
        COUNTER.set(0);
        HIT.replace(None);
    }

    /// Stop replaying; returns the key that was hit and how many rows were seen.
    pub fn finish() -> (Option<String>, usize) {
        TARGET.set(None);
        (HIT.take(), COUNTER.get())
    }

    pub fn active() -> bool {
        TARGET.get().is_some()
    }

    pub fn hit(key: &str, enabled: bool) -> bool {
        let Some(t) = TARGET.get() else {
            return false;
        };
        let i = COUNTER.get();
        COUNTER.set(i + 1);
        if i == t && enabled && !SKIP.contains(&key) {
            HIT.replace(Some(key.to_string()));
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Context;
    use tracedraw_core::geometry::{Point, Rect};
    use tracedraw_core::ShapeKind;

    /// A document with one of the object kinds the menus act on.
    fn mixed_app() -> App {
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        let r = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.new_shape(ShapeKind::Ellipse {
            rect: Rect::new(70.0, 10.0, 120.0, 40.0),
            arc: None,
        });
        app.start_text(Point::new(20.0, 80.0), None);
        app.text_insert("Menu test");
        app.finish_text();
        app.select(vec![r]);
        app.duplicate();
        app.convert_to_bitmap(50.0, true);
        app.create_table(Rect::new(10.0, 120.0, 80.0, 160.0));
        app.select_all();
        app
    }

    fn menus() -> Vec<(&'static str, fn(&mut App, &mut Ui))> {
        vec![
            ("file", file_menu),
            ("edit", edit_menu),
            ("view", view_menu),
            ("layout", layout_menu),
            ("object", object_menu),
            ("effects", effects_menu),
            ("bitmaps", bitmaps_menu),
            ("text", text_menu),
            ("table", table_menu),
            ("tools", tools_menu),
            ("window", window_menu),
            ("help", help_menu),
        ]
    }

    fn run_frame(ctx: &Context, f: impl FnOnce(&mut Ui)) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..Default::default()
        };
        let mut f = Some(f);
        let mut out = ctx.run_ui(input, |ui| {
            if let Some(f) = f.take() {
                f(ui);
            }
        });
        out.textures_delta.clear();
    }

    /// Every enabled row of every menu is clicked once against a mixed
    /// document; the application survives each command.
    #[test]
    fn every_menu_item_runs_without_panicking() {
        let ctx = crate::theme::ui_context();
        let mut hits = 0usize;
        for (name, body) in menus() {
            let mut app = mixed_app();
            let mut target = 0usize;
            loop {
                replay::start(target);
                run_frame(&ctx, |ui| body(&mut app, ui));
                let (hit, rows) = replay::finish();
                if let Some(key) = &hit {
                    hits += 1;
                    // Draw the whole window once so a dialog or docker the
                    // command opened is laid out too.
                    run_frame(&ctx, |ui| crate::ui::root(&mut app, ui));
                    assert!(!app.doc().pages.is_empty(), "{name}: {key} left no page");
                    app.dialog = crate::ui::dialogs::Dialog::None;
                    app.finish_text();
                    if app.doc().pages[0]
                        .layers
                        .iter()
                        .all(|l| l.shapes.is_empty())
                        || app.show_welcome
                    {
                        app = mixed_app();
                    }
                    app.show_welcome = false;
                    app.select_all();
                }
                target += 1;
                if target >= rows {
                    break;
                }
            }
        }
        assert!(hits > 150, "only {hits} menu items were exercised");
    }
}

/// Menu bar titles: the target design's size and spacing (about 25 px
/// between titles, the first one 11 px from the edge; within 2 px of it
/// across the bar).
const BAR_FONT: f32 = 13.0;

fn menu_bar_style(style: &mut egui::Style) {
    egui::containers::menu::menu_style(style);
    style.override_font_id = Some(egui::FontId::proportional(BAR_FONT));
    // An open or hovered title: light blue with a blue frame, square.
    for w in [
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        w.weak_bg_fill = MENU_HOVER;
        w.bg_fill = MENU_HOVER;
        w.bg_stroke = egui::Stroke::new(1.0, MENU_HOVER_LINE);
        w.corner_radius = egui::CornerRadius::ZERO;
        w.expansion = 0.0;
        w.fg_stroke.color = MENU_TEXT;
    }
    style.visuals.widgets.inactive.fg_stroke.color = MENU_TEXT;
    style.spacing.button_padding = egui::vec2(10.0, 3.0);
    style.spacing.item_spacing.x = 3.0;
}

pub fn menu_bar(app: &mut App, ui: &mut Ui) {
    egui::MenuBar::new()
        .style(menu_bar_style)
        .config(egui::containers::menu::MenuConfig::new().style(menu_popup_style))
        .ui(ui, |ui| {
            let menu = |ui: &mut Ui, key: &str, f: &mut dyn FnMut(&mut Ui)| {
                ui.menu_button(tr(key), |ui| {
                    ui.set_min_width(MENU_WIDTH);
                    body(ui, |ui| f(ui))
                });
            };
            menu(ui, "menu.file", &mut |ui| file_menu(app, ui));
            // With no drawing open only File, Tools, Window and Help remain.
            if app.has_document() {
                menu(ui, "menu.edit", &mut |ui| edit_menu(app, ui));
                menu(ui, "menu.view", &mut |ui| view_menu(app, ui));
                menu(ui, "menu.layout", &mut |ui| layout_menu(app, ui));
                menu(ui, "menu.object", &mut |ui| object_menu(app, ui));
                menu(ui, "menu.effects", &mut |ui| effects_menu(app, ui));
                menu(ui, "menu.bitmaps", &mut |ui| bitmaps_menu(app, ui));
                menu(ui, "menu.text", &mut |ui| text_menu(app, ui));
                menu(ui, "menu.table", &mut |ui| table_menu(app, ui));
            }
            menu(ui, "menu.tools", &mut |ui| tools_menu(app, ui));
            menu(ui, "menu.window", &mut |ui| window_menu(app, ui));
            menu(ui, "menu.help", &mut |ui| help_menu(app, ui));
        });
}

fn file_menu(app: &mut App, ui: &mut Ui) {
    // Commands that work on a drawing are off on the Welcome Screen alone.
    let doc = app.has_document();
    if item(ui, "menu.file.new", "Ctrl+N", true) {
        app.request_new_document();
    }
    if item(ui, "menu.file.new_from_template", "", true) {
        app.show_welcome = true;
        app.welcome_tab = crate::app::WelcomeTab::Templates;
    }
    if item(ui, "menu.file.open", "Ctrl+O", true) {
        app.open_dialog();
    }
    let recent = app.settings.recent_files.clone();
    sub(ui, "menu.file.open_recent", |ui| {
        if recent.is_empty() {
            ui.add_enabled(
                false,
                egui::Button::new(tr("menu.file.no_recent")).frame(false),
            );
        }
        for p in recent.iter().take(10) {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if menu_row(ui, &name, "", true, Mark::None)
                .0
                .on_hover_text(p.display().to_string())
                .clicked()
            {
                ui.close();
                app.open_path(p.clone());
            }
        }
        if !recent.is_empty() {
            sep(ui);
            if item(ui, "menu.file.clear_recent", "", true) {
                app.settings.recent_files.clear();
                app.settings.save();
            }
        }
    });
    sep(ui);
    if item(ui, "menu.file.close", "", doc) {
        app.close_document();
    }
    if item(ui, "menu.file.close_all", "", doc) {
        app.close_document();
    }
    sep(ui);
    if item(ui, "menu.file.save", "Ctrl+S", doc) {
        app.save(false);
    }
    if item(ui, "menu.file.save_as", "Ctrl+Shift+S", doc) {
        app.save(true);
    }
    if item(ui, "menu.file.save_as_template", "", doc) {
        app.save_as_template();
    }
    if item(
        ui,
        "menu.file.revert",
        "",
        app.file.is_some() && app.engine.is_dirty(),
    ) {
        if let Some(p) = app.file.clone() {
            app.open_path(p);
        }
    }
    sep(ui);
    todo_sub(ui, "menu.file.acquire_image");
    if item(ui, "menu.file.import", "Ctrl+I", doc) {
        app.import();
    }
    if item(ui, "menu.file.export", "Ctrl+E", doc) {
        app.dialog = Dialog::Export(crate::ui::dialogs::ExportState::default());
    }
    sub(ui, "menu.file.export_for", |ui| {
        if item(ui, "menu.file.export_for_web", "", doc) {
            app.dialog = Dialog::Export(crate::ui::dialogs::ExportState::web());
        }
        if item(ui, "menu.file.export_for_office", "", doc) {
            app.dialog = Dialog::Export(crate::ui::dialogs::ExportState::office());
        }
    });
    sub(ui, "menu.file.send_to", |ui| {
        use crate::export::SendTarget;
        for (key, t) in [
            ("menu.file.send_to_desktop", SendTarget::Desktop),
            ("menu.file.send_to_documents", SendTarget::Documents),
            ("menu.file.send_to_mail", SendTarget::Mail),
        ] {
            if item(ui, key, "", doc) {
                app.send_to(t);
            }
        }
    });
    if item(ui, "menu.file.publish_to_pdf", "", doc) {
        app.export_pdf();
    }
    sep(ui);
    sub(ui, "menu.file.print_merge", |ui| {
        if item(ui, "menu.file.print_merge_create", "", doc) {
            app.dialog = Dialog::PrintMerge(Default::default());
        }
        let loaded = app.merge_state.is_some();
        if item(ui, "menu.file.print_merge_edit", "", loaded) {
            if let Some(st) = app.merge_state.clone() {
                app.dialog = Dialog::PrintMerge(st);
            }
        }
        if item(ui, "menu.file.print_merge_perform", "", loaded) {
            if let Some(st) = app.merge_state.clone() {
                crate::export::perform_merge(app, &st.headers, &st.rows);
            }
        }
    });
    sep(ui);
    if item(ui, "menu.file.print", "Ctrl+P", doc) {
        app.dialog = Dialog::Print(Default::default());
    }
    if item(ui, "menu.file.print_preview", "", doc) {
        app.fullscreen_preview = true;
    }
    sep(ui);
    if item(ui, "menu.file.document_properties", "", doc) {
        app.dialog = Dialog::DocumentProperties;
    }
    sep(ui);
    if item(ui, "menu.file.exit", "Alt+F4", true) {
        app.request_exit();
    }
}

fn edit_menu(app: &mut App, ui: &mut Ui) {
    let has = !app.selection.is_empty();
    let undo = app
        .engine
        .undo_label()
        .map(|l| format!("{} {l}", tr("menu.edit.undo")))
        .unwrap_or_else(|| tr("menu.edit.undo"));
    let redo = app
        .engine
        .redo_label()
        .map(|l| format!("{} {l}", tr("menu.edit.redo")))
        .unwrap_or_else(|| tr("menu.edit.redo"));
    if item_text(
        ui,
        &undo,
        "Ctrl+Z",
        app.engine.undo_label().is_some(),
        Some(crate::ui::icons::Action::Undo),
        "menu.edit.undo",
    ) {
        app.undo();
    }
    if item_text(
        ui,
        &redo,
        "Ctrl+Shift+Z",
        app.engine.redo_label().is_some(),
        Some(crate::ui::icons::Action::Redo),
        "menu.edit.redo",
    ) {
        app.redo();
    }
    if item(
        ui,
        "menu.edit.repeat",
        "Ctrl+R",
        app.last_repeatable.is_some(),
    ) {
        app.repeat_last();
    }
    sep(ui);
    if item(ui, "menu.edit.cut", "Ctrl+X", has) {
        app.cut();
    }
    if item(ui, "menu.edit.copy", "Ctrl+C", has) {
        app.copy_with_system();
    }
    if item(ui, "menu.edit.copy_properties_from", "", has) {
        app.pending_copy_properties = true;
        app.status = tr("status.click_source_object");
    }
    sep(ui);
    if item(ui, "menu.edit.paste", "Ctrl+V", true) {
        app.paste_any();
    }
    if item(
        ui,
        "menu.edit.paste_in_view",
        "Ctrl+Shift+V",
        app.clipboard.is_some(),
    ) {
        app.paste_in_view();
    }
    if item(ui, "menu.edit.paste_special", "", true) {
        app.dialog = Dialog::PasteSpecial(Default::default());
    }
    sep(ui);
    if item(ui, "menu.edit.delete", "Delete", has) {
        app.delete_selection();
    }
    sep(ui);
    if item(ui, "menu.edit.duplicate", "Ctrl+D", has) {
        app.duplicate();
    }
    if item(ui, "menu.edit.clone", "", has) {
        app.duplicate();
    }
    sep(ui);
    sub(ui, "menu.edit.select_all", |ui| {
        if item(ui, "menu.edit.select_all_objects", "Ctrl+A", true) {
            app.select_all();
        }
        if item(ui, "menu.edit.select_all_text", "", true) {
            app.select_all_of(|s| matches!(s.kind, ShapeKind::Text { .. }));
        }
        if item(ui, "menu.edit.select_all_guidelines", "", true) {
            app.select_all_guides();
        }
        if item(ui, "menu.edit.select_all_nodes", "", has) {
            app.select_all_nodes();
        }
    });
    sep(ui);
    if item(ui, "menu.edit.find_and_replace", "Ctrl+F", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::FindReplace;
    }
    if item(ui, "menu.edit.step_and_repeat", "Ctrl+Shift+D", has) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::StepAndRepeat;
    }
}

fn view_menu(app: &mut App, ui: &mut Ui) {
    for (key, mode) in [
        ("menu.view.wireframe", crate::app::ViewMode::Wireframe),
        ("menu.view.normal", crate::app::ViewMode::Normal),
        ("menu.view.enhanced", crate::app::ViewMode::Enhanced),
        ("menu.view.pixels", crate::app::ViewMode::Pixels),
    ] {
        if radio(ui, key, "", app.view_mode == mode) {
            app.view_mode = mode;
            app.wireframe = mode == crate::app::ViewMode::Wireframe;
        }
    }
    sep(ui);
    if item(ui, "menu.view.fullscreen_preview", "F9", true) {
        app.fullscreen_preview = true;
    }
    if item(
        ui,
        "menu.view.preview_selected_only",
        "",
        !app.selection.is_empty(),
    ) {
        app.preview_selected_only = true;
        app.fullscreen_preview = true;
    }
    if check(ui, "menu.view.page_sorter", "", app.page_sorter) {
        app.page_sorter = !app.page_sorter;
    }
    sep(ui);
    if item(ui, "menu.view.zoom_in", "Ctrl++", true) {
        app.zoom_step(true);
    }
    if item(ui, "menu.view.zoom_out", "Ctrl+-", true) {
        app.zoom_step(false);
    }
    if item(ui, "menu.view.zoom_to_fit", "F4", true) {
        app.zoom_to_fit();
    }
    sep(ui);
    if check(ui, "menu.view.proof_colors", "", app.proof_colors) {
        app.proof_colors = !app.proof_colors;
    }
    if check(
        ui,
        "menu.view.simulate_overprints",
        "",
        app.simulate_overprints,
    ) {
        app.simulate_overprints = !app.simulate_overprints;
        app.raster.borrow_mut().invalidate();
    }
    if check(
        ui,
        "menu.view.rasterize_complex_effects",
        "",
        app.rasterize_complex_effects,
    ) {
        app.rasterize_complex_effects = !app.rasterize_complex_effects;
        app.raster.borrow_mut().invalidate();
    }
    sep(ui);
    sub(ui, "menu.view.page", |ui| {
        if check(ui, "menu.view.page_border", "", app.show_page_border) {
            app.show_page_border = !app.show_page_border;
        }
        if check(ui, "menu.view.bleed", "", app.show_bleed) {
            app.show_bleed = !app.show_bleed;
        }
        if check(ui, "menu.view.printable_area", "", app.show_printable_area) {
            app.show_printable_area = !app.show_printable_area;
        }
    });
    sub(ui, "menu.view.grid", |ui| {
        if check(ui, "menu.view.document_grid", "", app.show_grid) {
            app.show_grid = !app.show_grid;
        }
        if check(ui, "menu.view.pixel_grid", "", app.show_pixel_grid) {
            app.show_pixel_grid = !app.show_pixel_grid;
        }
        if check(ui, "menu.view.baseline_grid", "", app.show_baseline_grid) {
            app.show_baseline_grid = !app.show_baseline_grid;
        }
    });
    if check(ui, "menu.view.rulers", "Alt+Shift+R", app.show_rulers) {
        app.show_rulers = !app.show_rulers;
    }
    if check(ui, "menu.view.guidelines", "", app.show_guides) {
        app.show_guides = !app.show_guides;
    }
    if check(
        ui,
        "menu.view.alignment_guides",
        "Alt+Shift+A",
        app.snap.alignment_guides,
    ) {
        app.snap.alignment_guides = !app.snap.alignment_guides;
    }
    if check(
        ui,
        "menu.view.dynamic_guides",
        "Alt+Shift+D",
        app.snap.dynamic_guides,
    ) {
        app.snap.dynamic_guides = !app.snap.dynamic_guides;
    }
    sep(ui);
    sub(ui, "menu.view.snap_to", |ui| {
        if check(ui, "menu.view.snap_pixels", "", app.snap.pixels) {
            app.snap.pixels = !app.snap.pixels;
        }
        if check(ui, "menu.view.snap_document_grid", "Ctrl+Y", app.snap.grid) {
            app.snap.grid = !app.snap.grid;
        }
        if check(
            ui,
            "menu.view.snap_baseline_grid",
            "",
            app.snap.baseline_grid,
        ) {
            app.snap.baseline_grid = !app.snap.baseline_grid;
        }
        if check(ui, "menu.view.snap_guidelines", "", app.snap.guides) {
            app.snap.guides = !app.snap.guides;
        }
        if check(ui, "menu.view.snap_objects", "Alt+Z", app.snap.objects) {
            app.snap.objects = !app.snap.objects;
        }
        if check(ui, "menu.view.snap_page", "", app.snap.page) {
            app.snap.page = !app.snap.page;
        }
    });
    if check(ui, "menu.view.snap_off", "Alt+Q", app.snap.off) {
        app.snap.off = !app.snap.off;
    }
}

fn layout_menu(app: &mut App, ui: &mut Ui) {
    if item(ui, "menu.layout.insert_page", "", true) {
        app.dialog = Dialog::InsertPage {
            count: 1,
            after: true,
        };
    }
    if item(ui, "menu.layout.duplicate_page", "", true) {
        let page = app.page;
        app.run(Command::DuplicatePage { page });
    }
    if item(ui, "menu.layout.rename_page", "", true) {
        let name = app
            .doc()
            .page(app.page)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        app.dialog = Dialog::RenamePage { name };
    }
    if item(ui, "menu.layout.delete_page", "", app.doc().pages.len() > 1) {
        app.delete_page();
    }
    if item(ui, "menu.layout.go_to_page", "", true) {
        app.dialog = Dialog::GoToPage {
            page: app.page_index() + 1,
        };
    }
    sep(ui);
    sub(ui, "menu.layout.insert_page_number", |ui| {
        for (key, where_) in [
            (
                "menu.layout.page_number_active",
                crate::ops2::PageNumberWhere::Active,
            ),
            (
                "menu.layout.page_number_all",
                crate::ops2::PageNumberWhere::All,
            ),
            (
                "menu.layout.page_number_odd",
                crate::ops2::PageNumberWhere::Odd,
            ),
            (
                "menu.layout.page_number_even",
                crate::ops2::PageNumberWhere::Even,
            ),
        ] {
            if item(ui, key, "", true) {
                app.insert_page_number(where_);
            }
        }
    });
    if item(ui, "menu.layout.page_number_settings", "", true) {
        app.dialog = Dialog::PageNumberSettings;
    }
    sep(ui);
    if item(ui, "menu.layout.switch_orientation", "", true) {
        let s = app.page_size();
        let page = app.page;
        app.run(Command::ResizePage {
            page,
            size: tracedraw_core::geometry::Size::new(s.height, s.width),
        });
        app.fit_pending = true;
    }
    sep(ui);
    if item(ui, "menu.layout.document_options", "", true) {
        app.dialog = Dialog::Options;
        app.options_page = crate::ui::dialogs::OptionsPage::DocumentGeneral;
    }
    if item(ui, "menu.layout.page_size", "", true) {
        let s = app.page_size();
        app.dialog = Dialog::PageSize {
            width: s.width,
            height: s.height,
            all_pages: true,
        };
    }
    if item(ui, "menu.layout.page_layout", "", true) {
        app.dialog = Dialog::Options;
        app.options_page = crate::ui::dialogs::OptionsPage::Layout;
    }
    if item(ui, "menu.layout.page_background", "", true) {
        app.dialog = Dialog::Options;
        app.options_page = crate::ui::dialogs::OptionsPage::Background;
    }
}

fn object_menu(app: &mut App, ui: &mut Ui) {
    let has = !app.selection.is_empty();
    let many = app.selection.len() > 1;
    sub(ui, "menu.object.create", |ui| {
        if item(ui, "menu.object.create_table", "", true) {
            app.dialog = Dialog::CreateTable {
                rows: app.table_rows,
                cols: app.table_cols,
            };
        }
        if item(
            ui,
            "menu.object.create_arrowhead",
            "",
            app.selection.len() == 1,
        ) {
            app.create_arrowhead_from_selection();
        }
        if item(
            ui,
            "menu.object.create_pattern",
            "",
            !app.selection.is_empty(),
        ) {
            app.create_pattern_from_selection();
        }
        if item(
            ui,
            "menu.object.create_vector_pattern",
            "",
            !app.selection.is_empty(),
        ) {
            app.create_vector_pattern_from_selection();
        }
        if item(
            ui,
            "menu.object.create_symbol",
            "",
            !app.selection.is_empty(),
        ) {
            app.create_symbol_from_selection();
        }
    });
    sub(ui, "menu.object.insert", |ui| {
        if item(ui, "menu.object.insert_barcode", "", true) {
            app.dialog = Dialog::Barcode(Default::default());
        }
        if item(ui, "menu.object.insert_qr", "", true) {
            app.dialog = Dialog::QrCode(Default::default());
        }
        if item(ui, "menu.object.insert_page_number", "", true) {
            app.insert_page_number(crate::ops2::PageNumberWhere::Active);
        }
    });
    sep(ui);
    sub(ui, "menu.object.clip_frame", |ui| {
        let is_clip = app
            .selected_shapes()
            .iter()
            .any(|s| matches!(s.kind, ShapeKind::ClipFrame { .. }));
        if item(ui, "menu.object.clip_frame_place_inside", "", has) {
            app.pending_clip_frame = true;
            app.status = tr("status.click_frame");
        }
        if item(ui, "menu.object.clip_frame_extract", "", is_clip) {
            let clips: Vec<_> = app
                .selected_shapes()
                .iter()
                .filter(|s| matches!(s.kind, ShapeKind::ClipFrame { .. }))
                .map(|s| s.id)
                .collect();
            for c in clips {
                app.run(Command::ExtractContents { clip: c });
            }
        }
        if app.clip_frame_edit.is_some() {
            if item(ui, "menu.object.clip_frame_finish", "", true) {
                app.finish_clip_frame_edit();
            }
        } else if item(ui, "menu.object.clip_frame_edit", "", is_clip) {
            app.edit_clip_frame();
        }
        let locked = app
            .selected_shapes()
            .iter()
            .filter(|s| matches!(s.kind, ShapeKind::ClipFrame { .. }))
            .all(|s| app.clip_frame_locked(s.id));
        if check(ui, "menu.object.clip_frame_lock", "", is_clip && locked) && is_clip {
            app.toggle_clip_frame_lock();
        }
    });
    sub(ui, "menu.object.symmetry", |ui| {
        let has_sym = app.selected_symmetry().is_some();
        if item(ui, "menu.object.symmetry_create", "Alt+S", has && !has_sym) {
            app.create_symmetry();
        }
        if item(ui, "menu.object.symmetry_edit", "", has_sym) {
            app.dialog = Dialog::Symmetry;
        }
        if item(ui, "menu.object.symmetry_remove", "", has_sym) {
            app.remove_symmetry();
        }
    });
    sub(ui, "menu.object.symbol", |ui| {
        let has_symbols = !app.doc().symbols.is_empty();
        let has_instance = app
            .selected_shapes()
            .iter()
            .any(|s| matches!(s.kind, ShapeKind::SymbolInstance { .. }));
        if item(ui, "menu.object.symbol_new", "", has) {
            app.create_symbol_from_selection();
        }
        if item(ui, "menu.object.symbol_insert", "", has_symbols) {
            let last = app.doc().symbols.len().saturating_sub(1);
            app.insert_symbol_instance(last);
        }
        if item(ui, "menu.object.symbol_revert", "", has_instance) {
            app.revert_symbol_instances();
        }
        if item(ui, "menu.object.symbol_manager", "Ctrl+F3", true) {
            app.show_dockers = true;
            app.docker_tab = DockerTab::Symbols;
        }
    });
    todo_sub(ui, "menu.object.rollover");
    sep(ui);
    if item(ui, "menu.object.clear_transformations", "", has) {
        app.clear_transformations();
    }
    sep(ui);
    sub(ui, "menu.object.copy_effect", |ui| {
        if item(ui, "menu.object.copy_shadow_from", "", has) {
            app.pending_copy_effect = Some(crate::app::EffectKind::Shadow);
            app.status = tr("status.click_source_object");
        }
        if item(ui, "menu.object.copy_transparency_from", "", has) {
            app.pending_copy_effect = Some(crate::app::EffectKind::Transparency);
            app.status = tr("status.click_source_object");
        }
    });
    sub(ui, "menu.object.clone_effect", |ui| {
        if item(ui, "menu.object.clone_shadow_from", "", has) {
            app.pending_clone_effect = Some(crate::app::EffectKind::Shadow);
            app.status = tr("status.click_source_object");
        }
        if item(ui, "menu.object.clone_transparency_from", "", has) {
            app.pending_clone_effect = Some(crate::app::EffectKind::Transparency);
            app.status = tr("status.click_source_object");
        }
    });
    if item(ui, "menu.object.clear_effect", "", has) {
        app.clear_effects();
    }
    sep(ui);
    sub(ui, "menu.object.align_distribute", |ui| {
        use crate::ops::{Align, Distribute};
        for (key, shortcut, a) in [
            ("menu.object.align_left", "L", Align::Left),
            ("menu.object.align_right", "R", Align::Right),
            ("menu.object.align_top", "T", Align::Top),
            ("menu.object.align_bottom", "B", Align::Bottom),
            ("menu.object.align_centers_h", "E", Align::CenterH),
            ("menu.object.align_centers_v", "C", Align::CenterV),
            ("menu.object.center_to_page", "P", Align::CenterPage),
        ] {
            if item(ui, key, shortcut, has) {
                app.align(a);
            }
        }
        sep(ui);
        let three = app.selection.len() >= 3;
        for (key, d) in [
            ("menu.object.distribute_centers_h", Distribute::CentersH),
            ("menu.object.distribute_centers_v", Distribute::CentersV),
            ("menu.object.distribute_spacing_h", Distribute::SpacingH),
            ("menu.object.distribute_spacing_v", Distribute::SpacingV),
        ] {
            if item(ui, key, "", three) {
                app.distribute(d);
            }
        }
        sep(ui);
        if item(
            ui,
            "menu.object.align_distribute_docker",
            "Ctrl+Shift+A",
            true,
        ) {
            app.show_dockers = true;
            app.docker_tab = DockerTab::AlignDistribute;
        }
    });
    if item(ui, "menu.object.fit_objects_to_path", "", many) {
        app.fit_objects_to_path();
    }
    sub(ui, "menu.object.order", |ui| {
        if item(ui, "menu.object.to_front_of_page", "Ctrl+Home", has) {
            app.order(0);
        }
        if item(ui, "menu.object.to_back_of_page", "Ctrl+End", has) {
            app.order(3);
        }
        if item(ui, "menu.object.to_front_of_layer", "Shift+PgUp", has) {
            app.order(0);
        }
        if item(ui, "menu.object.to_back_of_layer", "Shift+PgDn", has) {
            app.order(3);
        }
        if item(ui, "menu.object.forward_one", "Ctrl+PgUp", has) {
            app.order(1);
        }
        if item(ui, "menu.object.back_one", "Ctrl+PgDn", has) {
            app.order(2);
        }
        if item(ui, "menu.object.in_front_of", "", has) {
            app.pending_order = Some(true);
            app.status = tr("status.click_reference_object");
        }
        if item(ui, "menu.object.behind", "", has) {
            app.pending_order = Some(false);
            app.status = tr("status.click_reference_object");
        }
        if item(ui, "menu.object.reverse_order", "", many) {
            app.reverse_order();
        }
    });
    sub(ui, "menu.object.group", |ui| {
        if item(ui, "menu.object.group_group", "Ctrl+G", many) {
            app.group_selection();
        }
        if item(ui, "menu.object.group_ungroup", "Ctrl+U", has) {
            app.ungroup_selection();
        }
        if item(ui, "menu.object.group_ungroup_all", "", has) {
            app.ungroup_all();
        }
    });
    sep(ui);
    sub(ui, "menu.object.hide", |ui| {
        if item(ui, "menu.object.hide_object", "", has) {
            app.set_visible(false);
        }
        if item(ui, "menu.object.show_all", "", true) {
            app.show_all();
        }
    });
    sub(ui, "menu.object.lock", |ui| {
        if item(ui, "menu.object.lock_object", "", has) {
            app.set_locked(true);
        }
        if item(ui, "menu.object.unlock_object", "", has) {
            app.set_locked(false);
        }
        if item(ui, "menu.object.unlock_all", "", true) {
            app.unlock_all();
        }
    });
    sep(ui);
    sub(ui, "menu.object.shaping", |ui| {
        use crate::ops::Shaping;
        let two = app.selection.len() >= 2;
        for (key, op, need_two) in [
            ("menu.object.weld", Shaping::Weld, true),
            ("menu.object.trim", Shaping::Trim, true),
            ("menu.object.intersect", Shaping::Intersect, true),
            ("menu.object.simplify", Shaping::Simplify, false),
            (
                "menu.object.front_minus_back",
                Shaping::FrontMinusBack,
                true,
            ),
            (
                "menu.object.back_minus_front",
                Shaping::BackMinusFront,
                true,
            ),
            ("menu.object.boundary", Shaping::Boundary, false),
        ] {
            if item(ui, key, "", if need_two { two } else { has }) {
                app.shaping(op);
            }
        }
        sep(ui);
        if item(ui, "menu.object.shaping_docker", "", true) {
            app.show_dockers = true;
            app.docker_tab = DockerTab::Shaping;
        }
    });
    if item(ui, "menu.object.combine", "Ctrl+L", many) {
        app.combine();
    }
    if item(ui, "menu.object.break_apart", "Ctrl+K", has) {
        app.break_apart();
    }
    if item(ui, "menu.object.add_perspective", "", has) {
        app.add_perspective();
    }
    sep(ui);
    if item(ui, "menu.object.convert_to_curves", "Ctrl+Q", has) {
        app.convert_to_curves();
    }
    if item(ui, "menu.object.convert_to_bitmap", "", has) {
        app.dialog = Dialog::ConvertToBitmap(Default::default());
    }
    if item(
        ui,
        "menu.object.convert_outline_to_object",
        "Ctrl+Shift+Q",
        has,
    ) {
        app.convert_outline_to_object();
    }
    if item(ui, "menu.object.join_curves", "", true) {
        app.show_dockers = true;
        app.docker_tab = crate::app::DockerTab::JoinCurves;
    }
    sep(ui);
    let has_fill = app
        .selected_shapes()
        .iter()
        .any(|s| !matches!(s.fill, tracedraw_core::Fill::None));
    if check(
        ui,
        "menu.object.overprint_fill",
        "",
        app.selected_shapes().iter().any(|s| s.overprint_fill),
    ) {
        app.toggle_overprint(true);
    }
    if check(
        ui,
        "menu.object.overprint_outline",
        "",
        app.selected_shapes().iter().any(|s| s.overprint_outline),
    ) {
        app.toggle_overprint(false);
    }
    let _ = has_fill;
    let has_bitmap = app
        .selected_shapes()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Bitmap { .. }));
    if check(
        ui,
        "menu.object.overprint_bitmap",
        "",
        has_bitmap && app.selected_shapes().iter().any(|s| s.overprint_fill),
    ) && has_bitmap
    {
        app.toggle_overprint(true);
    }
    let hinted = has && app.selection.iter().all(|id| app.object_hinted(*id));
    if check(ui, "menu.object.object_hinting", "", hinted) && has {
        app.toggle_object_hinting();
    }
    sep(ui);
    if check(
        ui,
        "menu.object.properties",
        "Alt+Enter",
        app.show_dockers && app.docker_tab == DockerTab::Properties,
    ) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Properties;
    }
    if check(
        ui,
        "menu.object.objects",
        "",
        app.show_dockers && app.docker_tab == DockerTab::Objects,
    ) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Objects;
    }
}

fn effects_menu(app: &mut App, ui: &mut Ui) {
    let has = !app.selection.is_empty();
    let has_bitmap = app
        .selected_shapes()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Bitmap { .. }));
    if item(ui, "menu.effects.flatten", "", has) {
        app.flatten_effects();
    }
    sep(ui);
    // Bitmap effect groups, as in the target design's Effects menu.
    for (key, group) in crate::fx::groups() {
        sub(ui, key, |ui| {
            for fx in group.iter() {
                let shortcut = match fx.id {
                    "brightness_contrast_intensity" => "Ctrl+B",
                    "color_balance" => "Ctrl+Shift+B",
                    "hue_saturation_lightness" => "Ctrl+Shift+U",
                    _ => "",
                };
                // Effects with settings open a dialog: "..." as in the
                // target design's menus.
                let dots = if fx.params.is_empty() && fx.id != "tone_curve" {
                    ""
                } else {
                    "..."
                };
                let label = format!("{}{dots}", tr(&format!("fx.{}", fx.id)));
                if item_label(ui, &label, shortcut, has_bitmap) {
                    crate::ui::effect_dialog::choose_effect(app, fx.id);
                }
            }
        });
    }
    sep(ui);
    if item(ui, "menu.effects.brush_strokes", "", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::BrushStrokes;
    }
    if item(ui, "menu.effects.bevel", "", has) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Bevel;
    }
    if item(ui, "menu.effects.blend", "", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Blend;
    }
    if item(ui, "menu.effects.contour", "Ctrl+F9", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Contour;
    }
    if item(ui, "menu.effects.envelope", "Ctrl+F7", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Envelope;
    }
    if item(ui, "menu.effects.extrude", "", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Extrude;
    }
    if item(ui, "menu.effects.lens", "Alt+F3", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Lens;
    }
    sep(ui);
    if item(ui, "menu.effects.vector_mosaic", "", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::VectorMosaic;
    }
}

fn bitmaps_menu(app: &mut App, ui: &mut Ui) {
    let has = !app.selection.is_empty();
    let has_bitmap = app
        .selected_shapes()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Bitmap { .. }));
    if item(ui, "menu.bitmaps.convert_to_bitmap", "", has) {
        app.dialog = Dialog::ConvertToBitmap(Default::default());
    }
    if item(ui, "menu.bitmaps.straighten_image", "", has_bitmap) {
        app.dialog = Dialog::StraightenImage(Default::default());
    }
    if item(
        ui,
        "menu.bitmaps.edit_bitmap",
        "",
        app.selected_bitmap().is_some(),
    ) {
        app.edit_bitmap_externally();
    }
    if item(ui, "menu.bitmaps.crop_bitmap", "", has_bitmap) {
        app.set_tool(crate::tools::Tool::Crop);
    }
    if item(ui, "menu.bitmaps.resample", "", has_bitmap) {
        app.dialog = Dialog::Resample { dpi: 300.0 };
    }
    // Black and white, duotone and paletted open their dialogs; the
    // others convert at once.
    sub(ui, "menu.bitmaps.mode", |ui| {
        if item(ui, "menu.bitmaps.mode_bw", "", has_bitmap) {
            app.dialog = Dialog::BitmapBw(Default::default());
        }
        if item(ui, "menu.bitmaps.mode_grayscale", "", has_bitmap) {
            app.set_bitmap_mode(crate::bitmap_fx::ColorMode::Grayscale);
        }
        if item(ui, "menu.bitmaps.mode_duotone", "", has_bitmap) {
            app.dialog = Dialog::BitmapDuotone(Default::default());
        }
        if item(ui, "menu.bitmaps.mode_paletted", "", has_bitmap) {
            app.dialog = Dialog::BitmapPaletted(Default::default());
        }
        if item(ui, "menu.bitmaps.mode_rgb", "", has_bitmap) {
            app.set_bitmap_mode(crate::bitmap_fx::ColorMode::Rgb);
        }
        if item(ui, "menu.bitmaps.mode_lab", "", has_bitmap) {
            app.apply_to_bitmaps("Lab Color", |img, _| crate::bitmap_modes::lab_mode(img));
        }
        if item(ui, "menu.bitmaps.mode_cmyk", "", has_bitmap) {
            app.set_bitmap_mode(crate::bitmap_fx::ColorMode::Cmyk);
        }
    });
    sub(ui, "menu.bitmaps.inflate", |ui| {
        // The same switch as Document Options > General: effects that
        // spread past the edges grow the bitmap first.
        let auto = app.doc().metadata.auto_inflate_bitmaps();
        if check(ui, "menu.bitmaps.inflate_auto", "", auto) {
            app.set_auto_inflate_bitmaps(!auto);
        }
        if item(ui, "menu.bitmaps.inflate_manual", "", has_bitmap) {
            app.dialog = Dialog::InflateBitmap(crate::ui::dialogs::InflateState::for_app(app));
        }
    });
    sep(ui);
    let linked = app.selected_bitmap_is_linked();
    if item(ui, "menu.bitmaps.break_link", "", linked) {
        app.break_bitmap_link();
    }
    if item(ui, "menu.bitmaps.update_from_link", "", linked) {
        app.update_bitmap_from_link();
    }
    sep(ui);
    if item(ui, "menu.bitmaps.quick_trace", "", has_bitmap) {
        app.quick_trace();
    }
    sub(ui, "menu.bitmaps.centerline_trace", |ui| {
        for (key, preset) in [
            (
                "menu.bitmaps.trace_technical",
                crate::trace::Preset::Technical,
            ),
            (
                "menu.bitmaps.trace_line_drawing",
                crate::trace::Preset::LineDrawing,
            ),
        ] {
            if item(ui, key, "", has_bitmap) {
                app.dialog = Dialog::Trace(crate::ui::dialogs::TraceState::new(preset));
            }
        }
    });
    sub(ui, "menu.bitmaps.outline_trace", |ui| {
        for (key, preset) in [
            ("menu.bitmaps.trace_line_art", crate::trace::Preset::LineArt),
            ("menu.bitmaps.trace_logo", crate::trace::Preset::Logo),
            (
                "menu.bitmaps.trace_detailed_logo",
                crate::trace::Preset::DetailedLogo,
            ),
            ("menu.bitmaps.trace_clipart", crate::trace::Preset::Clipart),
            (
                "menu.bitmaps.trace_low_quality",
                crate::trace::Preset::LowQualityImage,
            ),
            (
                "menu.bitmaps.trace_high_quality",
                crate::trace::Preset::HighQualityImage,
            ),
        ] {
            if item(ui, key, "", has_bitmap) {
                app.dialog = Dialog::Trace(crate::ui::dialogs::TraceState::new(preset));
            }
        }
    });
    sep(ui);
    todo_sub(ui, "menu.bitmaps.plugins");
    sep(ui);
    if item(ui, "menu.bitmaps.bitmap_mask", "", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::BitmapMask;
    }
}

fn text_menu(app: &mut App, ui: &mut Ui) {
    let is_text = app
        .selected_shapes()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Text { .. }));
    let is_para = app
        .selected_shapes()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Text { frame: Some(_), .. }));
    if item(ui, "menu.text.tabs", "", is_para) {
        app.dialog = Dialog::TextTabs;
    }
    if item(ui, "menu.text.columns", "", is_para) {
        app.dialog = Dialog::TextColumns;
    }
    if item(ui, "menu.text.bullets", "", is_para) {
        app.dialog = Dialog::TextBullets;
    }
    if item(ui, "menu.text.drop_cap", "", is_para) {
        app.dialog = Dialog::TextDropCap;
    }
    if item(ui, "menu.text.text_statistics", "", true) {
        app.dialog = Dialog::TextStatistics;
    }
    sep(ui);
    if item(ui, "menu.text.edit_text", "Ctrl+Shift+T", is_text) {
        app.edit_selected_text();
    }
    sub(ui, "menu.text.insert_formatting_code", |ui| {
        for (key, code) in [
            ("menu.text.code_em_space", "\u{2003}"),
            ("menu.text.code_en_space", "\u{2002}"),
            ("menu.text.code_nonbreaking_space", "\u{00A0}"),
            ("menu.text.code_em_dash", "\u{2014}"),
            ("menu.text.code_en_dash", "\u{2013}"),
            ("menu.text.code_optional_hyphen", "\u{00AD}"),
            ("menu.text.code_column_break", "\u{000C}"),
            ("menu.text.code_line_break", "\u{2028}"),
        ] {
            if item(ui, key, "", app.text_edit.is_some()) {
                app.insert_text(code);
            }
        }
    });
    sep(ui);
    if item(ui, "menu.text.convert", "Ctrl+F8", is_text) {
        app.toggle_text_kind();
    }
    if check(ui, "menu.text.show_non_printing", "", app.show_non_printing) {
        app.show_non_printing = !app.show_non_printing;
    }
    sep(ui);
    sub(ui, "menu.text.paragraph_text_frame", |ui| {
        if item(ui, "menu.text.frame_fit_text", "", is_para) {
            app.fit_text_to_frame();
        }
        if item(ui, "menu.text.frame_link", "", app.selection.len() == 2) {
            app.link_text_frames();
        }
        let linked = app.selection.iter().any(|id| app.is_linked_frame(*id));
        if item(ui, "menu.text.frame_unlink", "", linked) {
            app.unlink_text_frames();
        }
    });
    sep(ui);
    if item(
        ui,
        "menu.text.fit_text_to_path",
        "",
        is_text && app.selection.len() == 2,
    ) {
        app.fit_text_to_path();
    }
    // Shifted and rotated characters go back (the selected characters
    // while editing, the chosen character nodes with the Shape tool, else
    // all); Straighten Text also takes text off its path.
    if item(ui, "menu.text.straighten_text", "", is_text) {
        app.straighten_chars();
    }
    if item(ui, "menu.text.align_to_baseline", "Alt+F12", is_text) {
        app.align_chars_to_baseline();
    }
    let on_grid = app
        .text_shapes()
        .iter()
        .any(|s| matches!(&s.kind, ShapeKind::Text { para, .. } if para.baseline_grid_mm > 0.0));
    if check(ui, "menu.text.align_to_baseline_grid", "", on_grid) {
        app.toggle_baseline_grid();
    }
    sep(ui);
    if check(ui, "menu.text.use_hyphenation", "", app.text_hyphenation) {
        app.text_hyphenation = !app.text_hyphenation;
        if let Some(spans) = app.edit_spans() {
            app.set_edit_spans(spans);
        }
    }
    sub(ui, "menu.text.writing_tools", |ui| {
        if item(ui, "menu.text.spell_check", "Ctrl+F12", true) {
            app.dialog = Dialog::SpellCheck(Default::default());
        }
        if item(ui, "menu.text.grammatik", "", true) {
            app.dialog = Dialog::Grammar(Default::default());
        }
        if item(ui, "menu.text.thesaurus", "", true) {
            app.dialog = Dialog::Thesaurus(Default::default());
        }
        if item(ui, "menu.text.autocorrect", "", true) {
            app.dialog = Dialog::Autocorrect(Default::default());
        }
    });
    sep(ui);
    sub(ui, "menu.text.change_case", |ui| {
        for (key, mode) in [
            ("menu.text.case_sentence", crate::ops2::CaseMode::Sentence),
            ("menu.text.case_lower", crate::ops2::CaseMode::Lower),
            ("menu.text.case_upper", crate::ops2::CaseMode::Upper),
            ("menu.text.case_title", crate::ops2::CaseMode::Title),
            ("menu.text.case_toggle", crate::ops2::CaseMode::Toggle),
        ] {
            if item(ui, key, "", is_text) {
                app.change_case(mode);
            }
        }
    });
    if item(ui, "menu.text.make_web_compatible", "", is_text) {
        app.make_text_web_compatible();
    }
    sep(ui);
    if item(ui, "menu.text.encode", "", is_text) {
        app.dialog = Dialog::Encode(Default::default());
    }
    sep(ui);
    if item(ui, "menu.text.text_docker", "Ctrl+T", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Text;
    }
    if item(ui, "menu.text.glyphs", "Ctrl+F11", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Glyphs;
    }
}

fn table_menu(app: &mut App, ui: &mut Ui) {
    let in_table = app.selected_table().is_some();
    if item(ui, "menu.table.create_new_table", "", true) {
        app.dialog = Dialog::CreateTable {
            rows: app.table_rows,
            cols: app.table_cols,
        };
    }
    if item(
        ui,
        "menu.table.convert_text_to_table",
        "",
        app.selected_shapes()
            .iter()
            .any(|s| matches!(s.kind, ShapeKind::Text { .. })),
    ) {
        app.convert_text_to_table();
    }
    if item(ui, "menu.table.convert_table_to_text", "", in_table) {
        app.convert_table_to_text();
    }
    sep(ui);
    sub(ui, "menu.table.insert", |ui| {
        for (key, op) in [
            (
                "menu.table.insert_row_above",
                crate::table::TableOp::RowAbove,
            ),
            (
                "menu.table.insert_row_below",
                crate::table::TableOp::RowBelow,
            ),
            (
                "menu.table.insert_column_left",
                crate::table::TableOp::ColLeft,
            ),
            (
                "menu.table.insert_column_right",
                crate::table::TableOp::ColRight,
            ),
        ] {
            if item(ui, key, "", in_table) {
                app.table_op(op);
            }
        }
    });
    sub(ui, "menu.table.select", |ui| {
        for (key, op) in [
            ("menu.table.select_cell", crate::table::TableOp::SelectCell),
            ("menu.table.select_row", crate::table::TableOp::SelectRow),
            ("menu.table.select_column", crate::table::TableOp::SelectCol),
            (
                "menu.table.select_table",
                crate::table::TableOp::SelectTable,
            ),
        ] {
            if item(ui, key, "", in_table) {
                app.table_op(op);
            }
        }
    });
    sub(ui, "menu.table.delete", |ui| {
        for (key, op) in [
            ("menu.table.delete_row", crate::table::TableOp::DeleteRow),
            ("menu.table.delete_column", crate::table::TableOp::DeleteCol),
            (
                "menu.table.delete_table",
                crate::table::TableOp::DeleteTable,
            ),
        ] {
            if item(ui, key, "", in_table) {
                app.table_op(op);
            }
        }
    });
    sub(ui, "menu.table.distribute", |ui| {
        for (key, op) in [
            (
                "menu.table.distribute_rows",
                crate::table::TableOp::DistributeRows,
            ),
            (
                "menu.table.distribute_columns",
                crate::table::TableOp::DistributeCols,
            ),
        ] {
            if item(ui, key, "", in_table) {
                app.table_op(op);
            }
        }
    });
    sep(ui);
    if item(ui, "menu.table.merge_cells", "Ctrl+M", in_table) {
        app.table_op(crate::table::TableOp::Merge);
    }
    if item(ui, "menu.table.unmerge_cells", "", in_table) {
        app.table_op(crate::table::TableOp::Unmerge);
    }
    if item(ui, "menu.table.split_into_rows", "", in_table) {
        app.table_op(crate::table::TableOp::SplitRows);
    }
    if item(ui, "menu.table.split_into_columns", "", in_table) {
        app.table_op(crate::table::TableOp::SplitCols);
    }
}

fn tools_menu(app: &mut App, ui: &mut Ui) {
    sub(ui, "menu.tools.options", |ui| {
        if item(ui, "menu.tools.options_app", "Ctrl+J", true) {
            app.dialog = Dialog::Options;
            app.options_page = crate::ui::dialogs::OptionsPage::General;
        }
        if item(ui, "menu.tools.options_customization", "", true) {
            app.dialog = Dialog::Options;
            app.options_page = crate::ui::dialogs::OptionsPage::Appearance;
        }
        if item(ui, "menu.tools.options_tools", "", true) {
            app.dialog = Dialog::Options;
            app.options_page = crate::ui::dialogs::OptionsPage::Pick;
        }
        if item(ui, "menu.tools.options_global", "", true) {
            app.dialog = Dialog::Options;
            app.options_page = crate::ui::dialogs::OptionsPage::GlobalGeneral;
        }
        if item(ui, "menu.tools.options_workspaces", "", true) {
            app.dialog = Dialog::Options;
            app.options_page = crate::ui::dialogs::OptionsPage::Workspace;
        }
    });
    if item(ui, "menu.tools.save_settings_as_default", "", true) {
        app.save_defaults();
    }
    sep(ui);
    if item(ui, "menu.tools.color_management", "", true) {
        app.dialog = Dialog::ColorManagement;
    }
    sep(ui);
    sub(ui, "menu.tools.scripts", |ui| {
        if item(ui, "menu.tools.scripts_docker", "", true) {
            app.show_dockers = true;
            app.docker_tab = DockerTab::Scripts;
        }
        if item(ui, "menu.tools.scripts_run", "", true) {
            app.run_script_file();
        }
        if check(ui, "menu.tools.scripts_record", "", app.recording.is_some()) {
            app.toggle_recording();
        }
    });
    if item(ui, "menu.tools.font_manager", "", true) {
        app.dialog = Dialog::FontManager(Default::default());
    }
    if item(ui, "menu.tools.border_and_grommet", "", true) {
        app.dialog = Dialog::BorderGrommet(Default::default());
    }
}

fn window_menu(app: &mut App, ui: &mut Ui) {
    todo(ui, "menu.window.new_window", "");
    if item(ui, "menu.window.refresh_window", "Ctrl+W", true) {
        app.raster.borrow_mut().invalidate();
    }
    if item(ui, "menu.window.close_window", "Ctrl+F4", true) {
        app.close_document();
    }
    if item(ui, "menu.window.close_all", "", true) {
        app.close_all_documents();
    }
    sep(ui);
    todo(ui, "menu.window.cascade", "");
    todo(ui, "menu.window.tile_horizontally", "");
    todo(ui, "menu.window.tile_vertically", "");
    todo(ui, "menu.window.combine_windows", "");
    todo(ui, "menu.window.dock_window", "");
    sep(ui);
    sub(ui, "menu.window.workspace", |ui| {
        for (key, ws) in [
            (
                "menu.window.workspace_default",
                crate::app::Workspace::Default,
            ),
            ("menu.window.workspace_lite", crate::app::Workspace::Lite),
            (
                "menu.window.workspace_classic",
                crate::app::Workspace::Classic,
            ),
            (
                "menu.window.workspace_illustration",
                crate::app::Workspace::Illustration,
            ),
            (
                "menu.window.workspace_page_layout",
                crate::app::Workspace::PageLayout,
            ),
        ] {
            if check(ui, key, "", app.workspace == ws) {
                app.set_workspace(ws);
            }
        }
    });
    sep(ui);
    sub(ui, "menu.window.dockers", |ui| {
        for tab in DockerTab::ALL {
            if check(
                ui,
                tab.key(),
                tab.shortcut(),
                app.show_dockers && app.docker_tab == tab,
            ) {
                app.show_dockers = true;
                app.docker_tab = tab;
            }
        }
    });
    sub(ui, "menu.window.toolbars", |ui| {
        check(ui, "menu.window.toolbar_menu_bar", "", true);
        if check(
            ui,
            "menu.window.toolbar_standard",
            "",
            app.show_standard_toolbar,
        ) {
            app.show_standard_toolbar = !app.show_standard_toolbar;
        }
        if check(
            ui,
            "menu.window.toolbar_property_bar",
            "",
            app.show_property_bar,
        ) {
            app.show_property_bar = !app.show_property_bar;
        }
        if check(ui, "menu.window.toolbar_toolbox", "", app.show_toolbox) {
            app.show_toolbox = !app.show_toolbox;
        }
        if check(
            ui,
            "menu.window.toolbar_status_bar",
            "",
            app.show_status_bar,
        ) {
            app.show_status_bar = !app.show_status_bar;
        }
        if check(ui, "menu.window.toolbar_text", "", app.show_text_toolbar) {
            app.show_text_toolbar = !app.show_text_toolbar;
        }
        if check(ui, "menu.window.toolbar_zoom", "", app.show_zoom_toolbar) {
            app.show_zoom_toolbar = !app.show_zoom_toolbar;
        }
        if check(
            ui,
            "menu.window.toolbar_transform",
            "",
            app.show_transform_toolbar,
        ) {
            app.show_transform_toolbar = !app.show_transform_toolbar;
        }
    });
    sub(ui, "menu.window.color_palettes", |ui| {
        let entries: Vec<(usize, String, bool)> = app
            .palettes
            .iter()
            .enumerate()
            .map(|(i, p)| {
                (
                    i,
                    p.name_key().to_string(),
                    app.visible_palettes.contains(&i),
                )
            })
            .collect();
        if check(
            ui,
            "palette.show_document",
            "",
            app.settings.palette.show_document,
        ) {
            app.settings.palette.show_document = !app.settings.palette.show_document;
            app.settings.save();
        }
        for (i, key, on) in entries {
            if check(ui, &key, "", on) {
                app.toggle_palette(i);
            }
        }
        sep(ui);
        if item(ui, "menu.window.palette_open", "", true) {
            app.open_palette_file();
        }
        if item(ui, "menu.window.palette_editor", "", true) {
            app.dialog = Dialog::PaletteEditor(Default::default());
        }
        if item(ui, "menu.window.palette_manager", "", true) {
            app.show_dockers = true;
            app.docker_tab = DockerTab::Palettes;
        }
        if item(ui, "menu.window.palette_from_document", "", true) {
            app.palette_from_document();
        }
        if item(
            ui,
            "menu.window.palette_from_selection",
            "",
            !app.selection.is_empty(),
        ) {
            app.palette_from_selection();
        }
    });
    sep(ui);
    if check(ui, "menu.window.welcome_screen", "", app.show_welcome) {
        app.show_welcome = true;
    }
    // One entry per open drawing, numbered.
    for i in 0..app.docs.len() {
        let title = format!("{} {}", i + 1, app.document_tab_title(i));
        let on = !app.show_welcome && i == app.active_doc;
        if menu_row(ui, &title, "", true, Mark::Check(on)).0.clicked() {
            ui.close();
            app.switch_document(i);
        }
    }
}

fn help_menu(app: &mut App, ui: &mut Ui) {
    if item(ui, "menu.help.product_help", "F1", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Hints;
    }
    if item(ui, "menu.help.welcome_screen", "", true) {
        app.show_welcome = true;
    }
    if item(ui, "menu.help.video_tutorials", "", true) {
        app.show_welcome = true;
        app.welcome_tab = crate::app::WelcomeTab::Learn;
    }
    if check(
        ui,
        "menu.help.hints",
        "",
        app.show_dockers && app.docker_tab == DockerTab::Hints,
    ) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Hints;
    }
    if item(ui, "menu.help.quick_start_guide", "", true) {
        app.show_welcome = true;
        app.welcome_tab = crate::app::WelcomeTab::Learn;
    }
    if item(ui, "menu.help.user_guide", "", true) {
        app.show_welcome = true;
        app.welcome_tab = crate::app::WelcomeTab::Learn;
    }
    sep(ui);
    if item(ui, "menu.help.whats_new", "", true) {
        app.show_welcome = true;
        app.welcome_tab = crate::app::WelcomeTab::News;
    }
    todo_sub(ui, "menu.help.highlight_whats_new");
    sep(ui);
    if item(ui, "menu.help.updates", "", true) {
        app.open_url("https://github.com/igtoth/tracedraw/releases");
    }
    if item(ui, "menu.help.message_settings", "", true) {
        app.dialog = Dialog::Options;
        app.options_page = crate::ui::dialogs::OptionsPage::General;
    }
    if item(ui, "menu.help.community", "", true) {
        app.open_url("https://github.com/igtoth/tracedraw/discussions");
    }
    if item(ui, "menu.help.support", "", true) {
        app.open_url("https://github.com/igtoth/tracedraw/issues");
    }
    sep(ui);
    if item(ui, "menu.help.about", "", true) {
        app.about_open = true;
    }
}
