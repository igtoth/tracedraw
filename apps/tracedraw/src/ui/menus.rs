//! The menu bar. Items without an implementation are shown disabled so
//! the layout is complete; wiring them is tracked in docs/features.md.

use crate::app::{App, DockerTab};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::dialogs::Dialog;
use crate::ui::tips::RichTip;
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

/// The style of menu pop-ups: white, a one-pixel grey frame, square
/// corners, rows touching.
pub(crate) fn menu_popup_style(style: &mut egui::Style) {
    egui::containers::menu::menu_style(style);
    style.visuals.window_fill = crate::theme::Tokens::menu();
    style.visuals.window_stroke = egui::Stroke::new(1.0, crate::theme::Tokens::window_border());
    style.visuals.menu_corner_radius = egui::CornerRadius::ZERO;
    style.visuals.popup_shadow = crate::theme::shadow(egui::epaint::Shadow {
        offset: [2, 2],
        blur: 4,
        spread: 0,
        color: egui::Color32::from_black_alpha(30),
    });
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
/// first, sized once the rows are laid out. A body taller than the window
/// scrolls (wheel, scroll bar, keyboard focus) instead of running past the
/// window's edge; the pop-up then moves to fit inside the window.
pub(crate) fn body<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let limit = screen_body_limit(ui);
    body_within(ui, limit, add)
}

/// The tallest a menu body may be anywhere in the window: the window's
/// height less the pop-up frame around the rows.
fn screen_body_limit(ui: &Ui) -> f32 {
    let frame = egui::Frame::menu(ui.style()).total_margin().sum().y;
    body_limit(ui.ctx().content_rect().height() - frame)
}

/// The tallest the body of a menu-bar menu may be: the room from its
/// title's bottom edge (`title_bottom`, last frame's) down to the window's
/// bottom edge, so the menu stays under its title instead of moving up
/// over the bar. A pixel is kept spare so rounding never makes the menu
/// miss the room below its title.
fn below_body_limit(ui: &Ui, title_bottom: Option<f32>) -> f32 {
    let Some(title_bottom) = title_bottom else {
        return screen_body_limit(ui);
    };
    let frame = egui::Frame::menu(ui.style()).total_margin().sum().y;
    body_limit(ui.ctx().content_rect().bottom() - title_bottom - frame - 1.0)
}

/// At least a few rows, however small the window.
fn body_limit(room: f32) -> f32 {
    room.max(3.0 * ROW_HEIGHT)
}

/// A menu body at most `max_height` tall; taller rows scroll.
fn body_within<R>(ui: &mut Ui, max_height: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let width_id = ui.id().with("menu_width");
    let width = ui.ctx().data(|d| d.get_temp::<f32>(width_id));
    if let Some(w) = width {
        ui.set_min_width(w);
    }
    // The pop-up's room is last frame's size (or a default on its first
    // frame); the rows may need more, up to the limit.
    ui.set_max_height(max_height);
    egui::ScrollArea::vertical()
        .id_salt("menu_scroll")
        .max_height(max_height)
        .show(ui, |ui| {
            if let Some(w) = width {
                ui.set_min_width(w);
            }
            body_rows(ui, width_id, add)
        })
        .inner
}

fn body_rows<R>(ui: &mut Ui, width_id: egui::Id, add: impl FnOnce(&mut Ui) -> R) -> R {
    let slot = ui.painter().add(egui::Shape::Noop);
    let r = {
        let _keys = access::Scope::new();
        add(ui)
    };
    let rows = ui.min_rect();
    ui.ctx().data_mut(|d| d.insert_temp(width_id, rows.width()));
    let gutter = egui::Rect::from_min_max(
        egui::pos2(rows.left(), rows.top() - 2.0),
        egui::pos2(rows.left() + GUTTER, rows.bottom() + 2.0),
    );
    ui.painter().set(
        slot,
        egui::Shape::rect_filled(gutter, 0.0, crate::theme::Tokens::menu_gutter()),
    );
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
                crate::theme::Tokens::text(),
            )
            .size()
            .x;
        let s = if shortcut.is_empty() {
            0.0
        } else {
            f.layout_no_wrap(
                shortcut.to_string(),
                egui::FontId::proportional(FONT),
                crate::theme::Tokens::text(),
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
    let mut resp = resp;
    // The access key: underlined; pressed while this is the innermost
    // open menu, it works as a click on the row.
    let access = access::pick(label);
    if let Some((_, c)) = access {
        if enabled && access::pressed(ui, c) {
            resp.flags
                .insert(egui::response::Flags::FAKE_PRIMARY_CLICKED);
        }
    }
    let painter = ui.painter();
    let slot = painter.add(egui::Shape::Noop);
    if enabled && resp.hovered() {
        painter.set(slot, highlight(rect));
    }
    let color = if enabled && resp.hovered() {
        crate::theme::on_highlight()
    } else if enabled {
        crate::theme::Tokens::text()
    } else {
        crate::theme::Tokens::menu_text_off()
    };
    if let Some((at, _)) = access {
        access::underline(
            painter,
            egui::pos2(rect.left() + LABEL_X, rect.center().y),
            label,
            at,
            FONT,
            color,
        );
    }
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
                crate::theme::Tokens::text_dim()
            } else {
                crate::theme::Tokens::menu_text_off()
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
        crate::theme::Tokens::menu_hover(),
        egui::Stroke::new(1.0, crate::theme::Tokens::menu_hover_line()),
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
    if invoke::active() {
        return;
    }
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
        crate::theme::Tokens::border(),
    );
}

/// A menu item; `key` is an i18n key. Returns true when clicked. Items
/// with a toolbar counterpart show its icon in the gutter.
pub(crate) fn item(ui: &mut Ui, key: &str, shortcut: &str, enabled: bool) -> bool {
    item_text(ui, &tr(key), shortcut, enabled, menu_icon(key), key)
}

/// A menu item with a hover tip shown while it is greyed (why it cannot
/// run, such as an AI service that makes drawings only).
pub(crate) fn item_tip(
    ui: &mut Ui,
    key: &str,
    shortcut: &str,
    enabled: bool,
    tip: Option<&str>,
) -> bool {
    item_text_tip(ui, &tr(key), shortcut, enabled, menu_icon(key), key, tip)
}

/// A menu item with a ready label (a style name, a file).
pub(crate) fn item_label(ui: &mut Ui, label: &str, shortcut: &str, enabled: bool) -> bool {
    item_text(ui, label, shortcut, enabled, None, label)
}

/// A menu item with a ready label and an optional icon; `id` names it for
/// the test replay.
/// Help > Highlight What's New: the menu rows new in this version and the
/// submenus that lead to them, by their keys. Each version replaces the
/// list with its own.
pub const NEW_IN_THIS_VERSION: &[&str] = &[
    // Window > New Window.
    "menu.window",
    "menu.window.new_window",
    // Window > Dockers > Effects (the effect stack).
    "menu.window.dockers",
    "docker.effects",
    // Object > Clip to Frame > Opacity Mask, Invert Mask.
    "menu.object",
    "menu.object.clip_frame",
    "menu.object.clip_frame_opacity_mask",
    "menu.object.clip_frame_invert_mask",
    // Help > Updates checks for a newer release.
    "menu.help",
    "menu.help.updates",
];

thread_local! {
    /// Whether Help > Highlight What's New is on (set from the app each
    /// frame the menus draw).
    static HIGHLIGHT_NEW: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Centerline Trace's presets, as the Bitmaps menu and a bitmap's
/// right-click menu list them.
pub const CENTERLINE_TRACES: [(&str, crate::trace::Preset); 2] = [
    (
        "menu.bitmaps.trace_technical",
        crate::trace::Preset::Technical,
    ),
    (
        "menu.bitmaps.trace_line_drawing",
        crate::trace::Preset::LineDrawing,
    ),
];

/// Outline Trace's presets.
pub const OUTLINE_TRACES: [(&str, crate::trace::Preset); 6] = [
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
];

pub fn set_highlight_new(on: bool) {
    HIGHLIGHT_NEW.with(|h| h.set(on));
}

/// Whether the row `key` is marked as new now.
pub fn marked_new(key: &str) -> bool {
    HIGHLIGHT_NEW.with(|h| h.get()) && NEW_IN_THIS_VERSION.contains(&key)
}

/// The mark of a new row: a small accent disc at the row's right end.
fn paint_new_mark(ui: &Ui, row: egui::Rect) {
    let c = egui::pos2(row.right() - 8.0, row.center().y);
    ui.painter().circle_filled(c, 3.5, Tokens::ACCENT);
}

fn item_text(
    ui: &mut Ui,
    label: &str,
    shortcut: &str,
    enabled: bool,
    icon: Option<crate::ui::icons::Action>,
    id: &str,
) -> bool {
    item_text_tip(ui, label, shortcut, enabled, icon, id, None)
}

/// [`item_text`] with a hover tip for the greyed row.
fn item_text_tip(
    ui: &mut Ui,
    label: &str,
    shortcut: &str,
    enabled: bool,
    icon: Option<crate::ui::icons::Action>,
    id: &str,
    tip: Option<&str>,
) -> bool {
    if invoke::active() {
        return invoke::hit(id, shortcut, enabled);
    }
    let shortcut = invoke::shown_shortcut(id, shortcut);
    let (r, _) = menu_row(ui, label, &shortcut, enabled, Mark::None);
    let r = match tip {
        Some(t) if !enabled => r.rich_tip(t),
        _ => r,
    };
    if marked_new(id) {
        paint_new_mark(ui, r.rect);
    }
    #[cfg(test)]
    if replay::hit(id, enabled) {
        return true;
    }
    let _ = id;
    let ir = egui::Rect::from_center_size(
        egui::pos2(r.rect.left() + GUTTER / 2.0, r.rect.center().y),
        egui::vec2(16.0, 16.0),
    );
    let color = if enabled {
        Tokens::icon()
    } else {
        crate::theme::Tokens::menu_text_off()
    };
    if let Some(a) = icon {
        crate::ui::icons::draw_action(ui.painter(), ir, a, color);
    } else if let Some(p) = menu_pic(id) {
        crate::ui::propbar::draw_pic(ui.painter(), ir.expand(2.0), p, color);
    } else if let Some(g) = crate::ui::menu_glyphs::for_key(id) {
        crate::ui::menu_glyphs::draw(ui.painter(), ir, g, color);
    }
    if r.clicked() {
        ui.close();
        true
    } else {
        false
    }
}

/// A menu item with a ready label and a picture (a texture) in the
/// gutter, greyed with the row when it cannot run.
fn item_picture(
    ui: &mut Ui,
    label: &str,
    shortcut: &str,
    enabled: bool,
    picture: Option<egui::TextureId>,
) -> bool {
    if invoke::active() {
        return invoke::hit(label, shortcut, enabled);
    }
    let shortcut = invoke::shown_shortcut(label, shortcut);
    let (r, _) = menu_row(ui, label, &shortcut, enabled, Mark::None);
    #[cfg(test)]
    if replay::hit(label, enabled) {
        return true;
    }
    if let Some(tex) = picture {
        let ir = egui::Rect::from_center_size(
            egui::pos2(r.rect.left() + GUTTER / 2.0, r.rect.center().y),
            egui::vec2(16.0, 16.0),
        );
        let tint = if enabled {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_white_alpha(70)
        };
        ui.painter().image(
            tex,
            ir,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            tint,
        );
    }
    if enabled && r.clicked() {
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
        _ => return None,
    })
}

/// The top menus' title keys, in the menu bar's order.
pub(crate) fn menu_titles() -> Vec<&'static str> {
    all_menus().iter().map(|(k, _)| *k).collect()
}

/// Draw the picture the menu row of `key` shows into `r`; false when the
/// row has none.
pub(crate) fn draw_menu_picture(
    painter: &egui::Painter,
    r: egui::Rect,
    key: &str,
    color: egui::Color32,
) -> bool {
    if let Some(a) = menu_icon(key) {
        crate::ui::icons::draw_action(painter, r, a, color);
        true
    } else if let Some(p) = menu_pic(key) {
        crate::ui::propbar::draw_pic(painter, r.expand(2.0), p, color);
        true
    } else if let Some(g) = crate::ui::menu_glyphs::for_key(key) {
        crate::ui::menu_glyphs::draw(painter, r, g, color);
        true
    } else {
        false
    }
}

/// Whether the menu item `key` has a picture to draw.
pub(crate) fn has_menu_picture(key: &str) -> bool {
    menu_icon(key).is_some()
        || menu_pic(key).is_some()
        || crate::ui::menu_glyphs::for_key(key).is_some()
}

/// Property bar picture for a menu item, when it has one.
fn menu_pic(key: &str) -> Option<crate::ui::propbar::Pic> {
    use crate::ui::propbar::Pic;
    Some(match key {
        "menu.view.zoom_in" => Pic::ZoomIn,
        "menu.view.zoom_out" => Pic::ZoomOut,
        "menu.view.zoom_to_fit" => Pic::ZoomAll,
        "menu.object.group_group" => Pic::Group,
        "menu.object.group_ungroup" => Pic::Ungroup,
        "menu.object.group_ungroup_all" => Pic::UngroupAll,
        "menu.object.combine" => Pic::Combine,
        "menu.object.break_apart"
        | "menu.object.break_shadow_apart"
        | "menu.object.break_contour_apart"
        | "menu.object.break_blend_apart"
        | "menu.object.break_extrude_apart" => Pic::BreakApart,
        "menu.object.weld" => Pic::Weld,
        "menu.object.trim" => Pic::Trim,
        "menu.object.intersect" => Pic::Intersect,
        "menu.object.simplify" => Pic::Simplify,
        "menu.object.front_minus_back" => Pic::FrontMinusBack,
        "menu.object.back_minus_front" => Pic::BackMinusFront,
        "menu.object.boundary" => Pic::Boundary,
        "menu.object.to_front_of_page" => Pic::ToFront,
        "menu.object.to_front_of_layer" => Pic::ToFront,
        "menu.object.to_back_of_page" => Pic::ToBack,
        "menu.object.to_back_of_layer" => Pic::ToBack,
        "menu.object.convert_to_curves" => Pic::ToCurves,
        "menu.object.lock_object" | "menu.object.lock" => Pic::Lock(true),
        "menu.object.group" => Pic::Group,
        "menu.object.shaping" => Pic::Weld,
        "menu.object.transformations" => Pic::Position,
        "menu.object.tf_position" => Pic::Position,
        "menu.object.tf_rotate" => Pic::Rotate,
        "menu.object.tf_scale" => Pic::ScaleMirror,
        "menu.object.tf_size" => Pic::SizeBox,
        "menu.object.tf_skew" => Pic::SkewBox,
        "menu.object.align_distribute_docker" => Pic::AlignDistribute,
        "menu.object.convert_to_bitmap" => Pic::ConvertToBitmap,
        "menu.object.copy_envelope_from" => Pic::EnvelopeCopy,
        "menu.effects.envelope" => Pic::EnvelopeUnconstrained,
        "menu.object.shaping_docker" => Pic::Weld,
        "menu.text.tabs" => Pic::Tabs,
        "menu.text.columns" => Pic::Columns,
        "menu.text.bullets" => Pic::Bullets,
        "menu.text.drop_cap" => Pic::DropCap,
        "menu.text.text_statistics" => Pic::TextStatistics,
        "menu.text.edit_text" => Pic::EditText,
        "menu.text.convert" => Pic::ConvertText,
        "menu.text.fit_text_to_path" => Pic::FitTextToPath,
        "menu.text.straighten_text" => Pic::StraightenText,
        "menu.text.align_to_baseline" => Pic::AlignBaseline,
        "menu.text.change_case" => Pic::ChangeCase,
        "menu.text.encode" => Pic::Encode,
        "menu.bitmaps.convert_to_bitmap" => Pic::ConvertToBitmap,
        "menu.bitmaps.straighten_image" => Pic::StraightenImage,
        "menu.bitmaps.edit_bitmap" => Pic::EditBitmap,
        "menu.bitmaps.crop_bitmap" => Pic::CropBitmap,
        "menu.bitmaps.resample" => Pic::Resample,
        "menu.bitmaps.break_link" => Pic::BreakLink,
        "menu.bitmaps.update_from_link" => Pic::UpdateLink,
        "menu.bitmaps.quick_trace" => Pic::QuickTrace,
        "menu.window.new_window" => Pic::NewWindow,
        "menu.window.refresh_window" => Pic::RefreshWindow,
        "menu.window.close_window" => Pic::CloseWindow,
        "menu.window.close_all" => Pic::CloseAll,
        "menu.window.cascade" => Pic::Cascade,
        "menu.window.tile_horizontally" => Pic::TileHorizontally,
        "menu.window.tile_vertically" => Pic::TileVertically,
        "menu.window.combine_windows" => Pic::CombineWindows,
        "menu.window.dock_window" => Pic::DockWindow,
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

/// An exclusive item that can be unavailable (greyed, not clickable).
pub(crate) fn radio_if(ui: &mut Ui, key: &str, shortcut: &str, on: bool, enabled: bool) -> bool {
    mark_item_if(ui, key, shortcut, Mark::Radio(on), enabled)
}

fn mark_item(ui: &mut Ui, key: &str, shortcut: &str, mark: Mark) -> bool {
    mark_item_if(ui, key, shortcut, mark, true)
}

fn mark_item_if(ui: &mut Ui, key: &str, shortcut: &str, mark: Mark, enabled: bool) -> bool {
    if invoke::active() {
        return invoke::hit(key, shortcut, enabled);
    }
    let shortcut = invoke::shown_shortcut(key, shortcut);
    let (r, _) = menu_row(ui, &tr(key), &shortcut, enabled, mark);
    if marked_new(key) {
        paint_new_mark(ui, r.rect);
    }
    #[cfg(test)]
    if replay::hit(key, enabled) {
        return true;
    }
    if enabled && r.clicked() {
        ui.close();
        true
    } else {
        false
    }
}

/// A submenu that is not made yet: shown greyed, left out of the
/// commands that take shortcuts.
fn todo_sub(ui: &mut Ui, key: &str) {
    if invoke::active() {
        return;
    }
    let (r, _) = menu_row(ui, &tr(key), "", false, Mark::Submenu);
    row_picture(ui, r.rect, key, false);
}

/// The picture of the row of `key` in its gutter, when it has one.
fn row_picture(ui: &Ui, row: egui::Rect, key: &str, enabled: bool) {
    let ir = egui::Rect::from_center_size(
        egui::pos2(row.left() + GUTTER / 2.0, row.center().y),
        egui::vec2(16.0, 16.0),
    );
    let color = if enabled {
        Tokens::icon()
    } else {
        crate::theme::Tokens::menu_text_off()
    };
    draw_menu_picture(ui.painter(), ir, key, color);
}

/// A submenu that is greyed and does not open while `enabled` is false;
/// its items still count for shortcuts and the command list.
pub(crate) fn sub_if<R>(ui: &mut Ui, key: &str, enabled: bool, add: impl FnOnce(&mut Ui) -> R) {
    #[cfg(test)]
    let replaying = replay::active();
    #[cfg(not(test))]
    let replaying = false;
    if enabled || invoke::active() || replaying {
        sub(ui, key, add);
    } else {
        let (r, _) = menu_row(ui, &tr(key), "", false, Mark::Submenu);
        row_picture(ui, r.rect, key, false);
    }
}

/// A submenu row: the submenu opens beside it while it is hovered.
pub(crate) fn sub<R>(ui: &mut Ui, key: &str, add: impl FnOnce(&mut Ui) -> R) {
    if invoke::active() {
        // Walked without the menus on screen: the submenu's items count
        // as if it were open.
        invoke::enter(key);
        let _ = add(ui);
        invoke::leave();
        return;
    }
    #[cfg(test)]
    if replay::active() {
        // Replaying: submenus are laid out inline so their items count.
        let _ = add(ui);
        return;
    }
    let (resp, slot) = menu_row(ui, &tr(key), "", true, Mark::Submenu);
    row_picture(ui, resp.rect, key, true);
    if marked_new(key) {
        // Left of the submenu arrow.
        paint_new_mark(ui, resp.rect.translate(egui::vec2(-14.0, 0.0)));
    }
    let open = egui::containers::menu::MenuState::from_ui(ui, |state, _| {
        state.open_item == Some(egui::containers::menu::SubMenu::id_from_widget_id(resp.id))
    });
    if open {
        ui.painter().set(slot, highlight(resp.rect));
    }
    // A row scrolled out of a long menu's view does not open its submenu
    // from a pointer outside that view (over the menu bar, say).
    let view = ui.clip_rect();
    let pointer_in_view = ui
        .ctx()
        .pointer_hover_pos()
        .is_none_or(|p| view.contains(p));
    if !view.intersects(resp.rect) || !(open || pointer_in_view) {
        return;
    }
    egui::containers::menu::SubMenu::new().show(ui, &resp, |ui| {
        ui.set_min_width(MENU_WIDTH);
        body(ui, add)
    });
}

/// Access keys: one letter of each menu row, underlined, that picks the
/// row from the keyboard while its menu is open; the menu bar's titles
/// open with Alt and their letter.
pub(crate) mod access {
    use egui::Ui;
    use std::cell::RefCell;

    thread_local! {
        /// The letters taken in each menu body being drawn, innermost last.
        static TAKEN: RefCell<Vec<Vec<char>>> = const { RefCell::new(Vec::new()) };
    }

    /// The letters of one menu body, for as long as it is drawn.
    pub struct Scope;

    impl Scope {
        pub fn new() -> Scope {
            TAKEN.with(|t| t.borrow_mut().push(Vec::new()));
            Scope
        }
    }

    impl Drop for Scope {
        fn drop(&mut self) {
            TAKEN.with(|t| {
                t.borrow_mut().pop();
            });
        }
    }

    /// The access key of `label` among `taken`: the first letter or digit
    /// starting a word that is free, else the first free one anywhere.
    /// Returns its char index and the letter in lower case.
    pub fn choose(label: &str, taken: &[char]) -> Option<(usize, char)> {
        let chars: Vec<char> = label.chars().collect();
        // Letters and digits a key press can give.
        let free = |c: char| c.is_ascii_alphanumeric() && !taken.contains(&c.to_ascii_lowercase());
        let lower = |c: char| c.to_lowercase().next().unwrap_or(c);
        let word_start = |i: usize| i == 0 || !chars[i - 1].is_alphanumeric();
        (0..chars.len())
            .find(|&i| word_start(i) && free(chars[i]))
            .or_else(|| (0..chars.len()).find(|&i| free(chars[i])))
            .map(|i| (i, lower(chars[i])))
    }

    /// The access key of a row of the menu body being drawn (none outside
    /// a body).
    pub fn pick(label: &str) -> Option<(usize, char)> {
        TAKEN.with(|t| {
            let mut t = t.borrow_mut();
            let taken = t.last_mut()?;
            let picked = choose(label, taken)?;
            taken.push(picked.1);
            Some(picked)
        })
    }

    /// The key of an ASCII letter or digit.
    pub fn key_of(c: char) -> Option<egui::Key> {
        egui::Key::from_name(&c.to_ascii_uppercase().to_string())
    }

    /// `c` was pressed without Ctrl or Alt and this menu has no submenu
    /// open (the innermost open menu takes the keys).
    pub fn pressed(ui: &Ui, c: char) -> bool {
        let Some(key) = key_of(c) else {
            return false;
        };
        let hit = ui.input(|i| {
            i.events.iter().any(|e| {
                matches!(e, egui::Event::Key { key: k, pressed: true, modifiers, .. }
                    if *k == key && !(modifiers.ctrl || modifiers.alt || modifiers.command))
            })
        });
        hit && egui::containers::menu::MenuState::from_ui(ui, |state, _| state.open_item.is_none())
    }

    /// Underline the char at `at` of `label` drawn left-centred at `pos`.
    pub fn underline(
        painter: &egui::Painter,
        pos: egui::Pos2,
        label: &str,
        at: usize,
        size: f32,
        color: egui::Color32,
    ) {
        let font = egui::FontId::proportional(size);
        let before: String = label.chars().take(at).collect();
        let this: String = label.chars().skip(at).take(1).collect();
        let (x0, w, h) = painter.fonts_mut(|f| {
            let full = f.layout_no_wrap(label.to_string(), font.clone(), color);
            (
                f.layout_no_wrap(before, font.clone(), color).size().x,
                f.layout_no_wrap(this, font.clone(), color).size().x,
                full.size().y,
            )
        });
        let y = (pos.y + h / 2.0 - 2.0).round() + 0.5;
        painter.line_segment(
            [egui::pos2(pos.x + x0, y), egui::pos2(pos.x + x0 + w, y)],
            egui::Stroke::new(1.0, color),
        );
    }

    /// A menu bar title with its access key underlined.
    pub fn title(
        label: &str,
        at: Option<usize>,
        size: f32,
        color: egui::Color32,
    ) -> egui::WidgetText {
        let mut job = egui::text::LayoutJob::default();
        for (i, ch) in label.chars().enumerate() {
            job.append(
                &ch.to_string(),
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(size),
                    color,
                    underline: if Some(i) == at {
                        egui::Stroke::new(1.0, color)
                    } else {
                        egui::Stroke::NONE
                    },
                    ..Default::default()
                },
            );
        }
        job.into()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn access_keys_prefer_free_word_starts() {
            assert_eq!(choose("Save As...", &[]), Some((0, 's')));
            assert_eq!(choose("Save As...", &['s']), Some((5, 'a')));
            assert_eq!(choose("Save As...", &['s', 'a']), Some((2, 'v')));
            assert_eq!(choose("...", &[]), None);
            // Rows nest: each body keeps its own letters.
            let _outer = Scope::new();
            assert_eq!(pick("File"), Some((0, 'f')));
            {
                let _inner = Scope::new();
                assert_eq!(pick("Find"), Some((0, 'f')));
            }
            assert_eq!(pick("Find"), Some((1, 'i')));
        }
    }
}

/// Menu commands without the menus on screen. A shortcut chosen in
/// Options > Customization > Commands names a menu item by its key; the
/// menus are then walked invisibly and that item reports a click
/// ([`run_command`]). The same walk lists every command with its menus
/// and built-in shortcut ([`commands`]).
pub(crate) mod invoke {
    use std::cell::RefCell;

    /// A menu command: its i18n key, built-in shortcut and the keys of
    /// the menu and submenus it sits in.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Entry {
        pub key: String,
        pub shortcut: String,
        pub path: Vec<String>,
    }

    enum Mode {
        Collect { path: Vec<String>, out: Vec<Entry> },
        Trigger { key: String, fired: bool },
    }

    thread_local! {
        static MODE: RefCell<Option<Mode>> = const { RefCell::new(None) };
        /// The user's shortcuts (`Settings::shortcuts`), for the rows.
        static BINDINGS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    }

    pub fn active() -> bool {
        MODE.with(|m| m.borrow().is_some())
    }

    pub fn hit(key: &str, shortcut: &str, enabled: bool) -> bool {
        MODE.with(|m| match m.borrow_mut().as_mut() {
            Some(Mode::Collect { path, out }) => {
                if key.starts_with("menu.") && !out.iter().any(|e| e.key == key) {
                    out.push(Entry {
                        key: key.to_string(),
                        shortcut: shortcut.to_string(),
                        path: path.clone(),
                    });
                }
                false
            }
            Some(Mode::Trigger { key: wanted, fired }) if !*fired && enabled && wanted == key => {
                *fired = true;
                true
            }
            Some(Mode::Trigger { .. }) | None => false,
        })
    }

    pub fn enter(key: &str) {
        MODE.with(|m| {
            if let Some(Mode::Collect { path, .. }) = m.borrow_mut().as_mut() {
                path.push(key.to_string());
            }
        });
    }

    pub fn leave() {
        MODE.with(|m| {
            if let Some(Mode::Collect { path, .. }) = m.borrow_mut().as_mut() {
                path.pop();
            }
        });
    }

    pub fn fired() -> bool {
        MODE.with(|m| matches!(&*m.borrow(), Some(Mode::Trigger { fired: true, .. })))
    }

    pub(super) fn start_collect() {
        MODE.with(|m| {
            *m.borrow_mut() = Some(Mode::Collect {
                path: Vec::new(),
                out: Vec::new(),
            })
        });
    }

    pub(super) fn start_trigger(key: &str) {
        MODE.with(|m| {
            *m.borrow_mut() = Some(Mode::Trigger {
                key: key.to_string(),
                fired: false,
            })
        });
    }

    /// Ends a walk however it ends, so the menus never stay invisible.
    pub(super) struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            MODE.with(|m| m.borrow_mut().take());
        }
    }

    /// End the walk: the commands collected, or whether the item fired.
    pub(super) fn finish() -> (Vec<Entry>, bool) {
        MODE.with(|m| match m.borrow_mut().take() {
            Some(Mode::Collect { out, .. }) => (out, false),
            Some(Mode::Trigger { fired, .. }) => (Vec::new(), fired),
            None => (Vec::new(), false),
        })
    }

    pub fn set_bindings(bindings: &[(String, String)]) {
        BINDINGS.with(|b| {
            let mut b = b.borrow_mut();
            if b.as_slice() != bindings {
                *b = bindings.to_vec();
            }
        });
    }

    /// The shortcut a row shows: the user's for `key`, else `default`.
    pub fn shown_shortcut(key: &str, default: &str) -> String {
        // The first of its keys (a command may have several, #822).
        BINDINGS.with(|b| {
            crate::shortcuts::keys_for(&b.borrow(), key, Some(default))
                .into_iter()
                .next()
                .unwrap_or_default()
        })
    }
}

/// The menus of the menu bar in order, with their title keys.
fn all_menus() -> [(&'static str, fn(&mut App, &mut Ui)); 12] {
    [
        ("menu.file", file_menu),
        ("menu.edit", edit_menu),
        ("menu.view", view_menu),
        ("menu.layout", layout_menu),
        ("menu.object", object_menu),
        ("menu.effects", effects_menu),
        ("menu.bitmaps", bitmaps_menu),
        ("menu.text", text_menu),
        ("menu.table", table_menu),
        ("menu.tools", tools_menu),
        ("menu.window", window_menu),
        ("menu.help", help_menu),
    ]
}

/// The menus the bar shows with no drawing open.
const MENUS_WITHOUT_DRAWING: [&str; 4] = ["menu.file", "menu.tools", "menu.window", "menu.help"];

/// Walk the menus without showing them, in a Ui of their own far off
/// the screen, so nothing is drawn, hit or laid out beside the real
/// widgets. `shown_only`: only the menus the bar shows now.
fn walk_invisibly(app: &mut App, ctx: &egui::Context, shown_only: bool, until_fired: bool) {
    let far = egui::Rect::from_min_size(
        egui::pos2(-100_000.0, -100_000.0),
        egui::vec2(400.0, 50_000.0),
    );
    let mut ui = Ui::new(
        ctx.clone(),
        egui::Id::new("menu_commands_walk"),
        egui::UiBuilder::new()
            .invisible()
            .max_rect(far)
            .layer_id(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("menu_commands_walk"),
            )),
    );
    for (title, body) in all_menus() {
        if until_fired && invoke::fired() {
            break;
        }
        if shown_only && !app.has_document() && !MENUS_WITHOUT_DRAWING.contains(&title) {
            continue;
        }
        invoke::enter(title);
        body(app, &mut ui);
        invoke::leave();
    }
}

/// Every command of the menu bar: its key, its built-in shortcut and the
/// menus it sits in, in menu order.
pub fn commands(app: &mut App, ctx: &egui::Context) -> Vec<invoke::Entry> {
    let _guard = invoke::Guard;
    invoke::start_collect();
    walk_invisibly(app, ctx, false, false);
    invoke::finish().0
}

/// Run the menu command `key` as its item would when clicked; false when
/// the menus shown now have no enabled item with that key.
pub fn run_command(app: &mut App, ctx: &egui::Context, key: &str) -> bool {
    let _guard = invoke::Guard;
    invoke::start_trigger(key);
    walk_invisibly(app, ctx, true, true);
    invoke::finish().1
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
    const SKIP: [&str; 16] = [
        "menu.file.send_to_mail",
        "menu.tools.macros_load",
        "menu.help.updates",
        "menu.help.community",
        "menu.help.support",
        "menu.file.open",
        "menu.file.save",
        "menu.file.save_as",
        "menu.file.save_as_template",
        "menu.file.import",
        "menu.file.exit",
        "menu.bitmaps.edit_bitmap",
        "menu.tools.scripts_run",
        "menu.window.palette_open",
        "menu.window.workspace_export",
        "menu.window.workspace_import",
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
    #[test]
    fn the_panels_menu_lists_every_panel_once() {
        let mut listed: Vec<DockerTab> = DockerTab::MENU_GROUPS
            .iter()
            .flat_map(|g| g.iter().copied())
            .chain(DockerTab::EFFECTS_MENU)
            .collect();
        assert_eq!(listed.len(), DockerTab::ALL.len());
        listed.dedup();
        for t in DockerTab::ALL {
            assert_eq!(listed.iter().filter(|x| **x == t).count(), 1, "{t:?}");
        }
    }

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

    /// A frame of the whole window with `events` as its input.
    fn root_frame(ctx: &Context, app: &mut App, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
    }

    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    /// The menus list their commands with the menus they sit in and their
    /// built-in shortcuts, and run one by its key with nothing shown.
    #[test]
    fn menu_commands_are_listed_and_run_by_key() {
        let ctx = crate::theme::ui_context();
        let mut app = mixed_app();
        let mut entries = Vec::new();
        run_frame(&ctx, |ui| entries = commands(&mut app, ui.ctx()));
        assert!(!invoke::active());
        let find = |key: &str| entries.iter().find(|e| e.key == key).cloned();
        let export = find("menu.file.export").expect("export listed");
        assert_eq!(export.shortcut, "Ctrl+E");
        assert_eq!(export.path, ["menu.file"]);
        let all = find("menu.edit.select_all_objects").expect("submenu items listed");
        assert_eq!(all.path, ["menu.edit", "menu.edit.select_all"]);
        assert_eq!(all.shortcut, "Ctrl+A");
        assert!(find("menu.window.new_window").is_some());
        assert!(find("menu.window.cascade").is_some());
        // Every row Highlight What's New marks is a command, or a menu or
        // submenu on the way to one.
        // (Window > Dockers lists the panels, which are not commands.)
        for key in NEW_IN_THIS_VERSION {
            let docker =
                *key == "menu.window.dockers" || DockerTab::ALL.iter().any(|t| t.key() == *key);
            assert!(
                docker
                    || find(key).is_some()
                    || entries.iter().any(|e| e.path.iter().any(|p| p == key)),
                "{key} is not in the menus"
            );
        }
        assert!(!marked_new("menu.help.updates"));
        set_highlight_new(true);
        assert!(marked_new("menu.help.updates"));
        assert!(!marked_new("menu.file.export"));
        set_highlight_new(false);
        assert!(
            find("menu.edit.select_all").is_none(),
            "submenus are not commands"
        );
        assert!(entries.len() > 200, "{} commands", entries.len());
        for (i, e) in entries.iter().enumerate() {
            assert!(e.key.starts_with("menu."), "{e:?}");
            assert!(!entries[..i].iter().any(|o| o.key == e.key), "{e:?} twice");
        }
        // Run one: nothing selected, then Select All.
        app.selection.clear();
        let mut ran = false;
        run_frame(&ctx, |ui| {
            ran = run_command(&mut app, ui.ctx(), "menu.edit.select_all_objects")
        });
        assert!(ran && !app.selection.is_empty());
        // Unknown keys and commands not available now run nothing.
        run_frame(&ctx, |ui| {
            ran = run_command(&mut app, ui.ctx(), "no.such.key")
        });
        assert!(!ran);
        app.selection.clear();
        let shapes = app.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.shapes.len())
            .sum::<usize>();
        run_frame(&ctx, |ui| {
            ran = run_command(&mut app, ui.ctx(), "menu.edit.duplicate")
        });
        assert!(!ran, "Duplicate needs a selection");
        assert_eq!(
            app.doc().pages[0]
                .layers
                .iter()
                .map(|l| l.shapes.len())
                .sum::<usize>(),
            shapes
        );
        assert!(!invoke::active());
    }

    /// The Effects menu: Flatten Effects, the bitmap groups, then the
    /// effect panels, checked while open; Object > Create holds Arrowhead
    /// and Pattern Fill, which asks for the pattern's type.
    #[test]
    fn effects_menu_checks_the_open_effect_panels() {
        let ctx = crate::theme::ui_context();
        let mut app = mixed_app();
        let mut entries = Vec::new();
        run_frame(&ctx, |ui| entries = commands(&mut app, ui.ctx()));
        let find = |key: &str| entries.iter().find(|e| e.key == key).cloned();
        assert_eq!(
            find("menu.effects.flatten").expect("flatten").path,
            ["menu.effects"]
        );
        assert!(find("menu.effects.picker").is_none_or(|e| e.path == ["menu.window"]));
        assert_eq!(
            find("menu.effects.contour").expect("contour").shortcut,
            "Ctrl+F9"
        );
        let create: Vec<String> = entries
            .iter()
            .filter(|e| e.path == ["menu.object", "menu.object.create"])
            .map(|e| e.key.clone())
            .collect();
        assert_eq!(
            create,
            ["menu.object.create_arrowhead", "menu.object.create_pattern"]
        );
        assert!(find("menu.table.create_new_table").is_some());
        // The panels: checked while their tab is in the shown column.
        app.show_dockers = false;
        assert!(!effect_docker_open(&app, DockerTab::Blend));
        let mut ran = false;
        run_frame(&ctx, |ui| {
            ran = run_command(&mut app, ui.ctx(), "menu.effects.blend")
        });
        assert!(ran && app.show_dockers && app.docker_tab == DockerTab::Blend);
        assert!(effect_docker_open(&app, DockerTab::Blend));
        // Pattern Fill opens its dialog.
        let id = app.doc().pages[0]
            .layers
            .iter()
            .flat_map(|l| &l.shapes)
            .map(|s| s.id)
            .next()
            .expect("a shape");
        app.select(vec![id]);
        run_frame(&ctx, |ui| {
            ran = run_command(&mut app, ui.ctx(), "menu.object.create_pattern")
        });
        assert!(ran && matches!(app.dialog, Dialog::CreatePattern { vector: true }));
    }

    /// File > Close All closes every drawing, asking about unsaved ones.
    #[test]
    fn file_close_all_closes_every_drawing() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.new_document();
        app.new_document();
        assert_eq!(app.docs.len(), 3);
        let mut ran = false;
        run_frame(&ctx, |ui| {
            ran = run_command(&mut app, ui.ctx(), "menu.file.close_all")
        });
        assert!(ran && !app.has_document());
    }

    /// Window > New Window opens a second window of the active drawing.
    #[test]
    fn window_new_window_opens_a_second_window() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        let mut ran = false;
        run_frame(&ctx, |ui| {
            ran = run_command(&mut app, ui.ctx(), "menu.window.new_window")
        });
        assert!(ran);
        assert_eq!(app.docs.len(), 2);
        assert_eq!(app.active_doc, 1);
        assert!(app.document_tab_title(0).ends_with(":1"));
        assert!(app.document_tab_title(1).ends_with(":2"));
    }

    /// A shortcut of the user's runs its menu command, takes over a
    /// built-in shortcut and shows in the menu row.
    #[test]
    fn shortcuts_of_the_users_run_menu_commands() {
        use egui::{Key, Modifiers};
        let ctx = crate::theme::ui_context();
        let mut app = mixed_app();
        app.show_welcome = false;
        let count = |app: &App| {
            app.doc().pages[0]
                .layers
                .iter()
                .map(|l| l.shapes.len())
                .sum::<usize>()
        };
        app.settings
            .shortcuts
            .push(("menu.edit.select_all_objects".into(), "Alt+Shift+K".into()));
        app.selection.clear();
        root_frame(
            &ctx,
            &mut app,
            vec![key(Key::K, Modifiers::ALT | Modifiers::SHIFT)],
        );
        root_frame(&ctx, &mut app, vec![]);
        assert!(!app.selection.is_empty(), "{}", app.status);
        assert_eq!(
            invoke::shown_shortcut("menu.edit.select_all_objects", "Ctrl+A"),
            "Alt+Shift+K"
        );
        assert_eq!(invoke::shown_shortcut("menu.edit.cut", "Ctrl+X"), "Ctrl+X");
        // Ctrl+D (built in for Duplicate) given to Select All: no copy.
        app.settings.shortcuts[0].1 = "Ctrl+D".into();
        app.selection.clear();
        let before = count(&app);
        root_frame(&ctx, &mut app, vec![key(Key::D, Modifiers::COMMAND)]);
        assert_eq!(count(&app), before);
        assert!(!app.selection.is_empty());
        // The built-in Ctrl+A still selects everything.
        app.selection.clear();
        root_frame(&ctx, &mut app, vec![key(Key::A, Modifiers::COMMAND)]);
        assert!(!app.selection.is_empty());
        // A command not available now says so.
        app.settings
            .shortcuts
            .push(("menu.edit.duplicate".into(), "Alt+Shift+J".into()));
        app.selection.clear();
        root_frame(
            &ctx,
            &mut app,
            vec![key(Key::J, Modifiers::ALT | Modifiers::SHIFT)],
        );
        assert_eq!(count(&app), before);
        assert!(
            app.status.contains(&tr("menu.edit.duplicate")),
            "{}",
            app.status
        );
    }

    /// A frame of the menu bar alone in a window `height` tall.
    fn bar_frame(ctx: &Context, app: &mut App, height: f32, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, height),
            )),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| menu_bar(app, ui));
        out.textures_delta.clear();
    }

    /// The rects of the pop-ups on screen.
    fn popup_rects(ctx: &Context) -> Vec<egui::Rect> {
        ctx.memory(|m| {
            m.areas()
                .visible_layer_ids()
                .into_iter()
                .filter(|l| l.order == egui::Order::Foreground)
                .filter_map(|l| m.area_rect(l.id))
                .collect()
        })
    }

    /// The Object menu opened with Alt and its access key in a window
    /// `height` tall: the rects of the pop-ups shown.
    fn open_object_menu(height: f32) -> Vec<egui::Rect> {
        let ctx = crate::theme::ui_context();
        let mut app = mixed_app();
        let mut taken = Vec::new();
        let mut letter = ' ';
        for key in [
            "menu.file",
            "menu.edit",
            "menu.view",
            "menu.layout",
            "menu.object",
        ] {
            let c = access::choose(&tr(key), &taken).map(|a| a.1).unwrap_or(' ');
            taken.push(c);
            letter = c;
        }
        let k = access::key_of(letter).expect("object menu access key");
        bar_frame(&ctx, &mut app, height, Vec::new());
        bar_frame(&ctx, &mut app, height, vec![key(k, egui::Modifiers::ALT)]);
        for _ in 0..4 {
            bar_frame(&ctx, &mut app, height, Vec::new());
        }
        popup_rects(&ctx)
    }

    /// A long menu in a short window stays inside the window, under its
    /// title, and scrolls instead of running past the bottom edge (#1008).
    #[test]
    fn a_long_menu_fits_a_short_window() {
        let tall = open_object_menu(1200.0);
        assert_eq!(tall.len(), 1, "{tall:?}");
        let full = tall[0];
        let height = 360.0;
        assert!(full.height() > height, "the Object menu is {full:?}");
        let short = open_object_menu(height);
        assert_eq!(short.len(), 1, "{short:?}");
        let menu = short[0];
        assert!(
            menu.bottom() <= height + 0.5,
            "{menu:?} runs past the window"
        );
        assert!(
            (menu.top() - full.top()).abs() < 0.5,
            "{menu:?} left its title"
        );
        assert!(menu.height() > 3.0 * ROW_HEIGHT, "{menu:?}");
    }

    /// One frame of a menu pop-up with `rows` rows at the top left of a
    /// window `height` tall: the pop-up's rect and the last row's rect.
    fn long_menu_frame(
        ctx: &Context,
        height: f32,
        rows: usize,
        events: Vec<egui::Event>,
    ) -> (egui::Rect, egui::Rect) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, height),
            )),
            events,
            ..Default::default()
        };
        let mut last = egui::Rect::NOTHING;
        let mut area = egui::Rect::NOTHING;
        let mut out = ctx.run_ui(input, |ui| {
            let r = egui::Area::new(egui::Id::new("long menu"))
                .fixed_pos(egui::pos2(10.0, 10.0))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    ui.set_style(ui.ctx().global_style());
                    menu_popup_style(ui.style_mut());
                    egui::Frame::menu(ui.style()).show(ui, |ui| {
                        body(ui, |ui| {
                            for i in 0..rows {
                                item_label(ui, &format!("Row {i}"), "", true);
                            }
                            last = ui.label("last").rect;
                        })
                    })
                });
            area = r.response.rect;
        });
        out.textures_delta.clear();
        (area, last)
    }

    /// A menu that fits the window shows all its rows, however tall it
    /// is, without a scroll bar's limit.
    #[test]
    fn a_menu_that_fits_shows_every_row() {
        let ctx = crate::theme::ui_context();
        let mut shown = (egui::Rect::NOTHING, egui::Rect::NOTHING);
        for _ in 0..4 {
            shown = long_menu_frame(&ctx, 1000.0, 30, Vec::new());
        }
        let (area, last) = shown;
        assert!(area.height() > 30.0 * ROW_HEIGHT, "{area:?}");
        assert!(last.bottom() <= area.bottom(), "{last:?} in {area:?}");
    }

    /// The wheel scrolls a menu body taller than the window, so its last
    /// rows come into view.
    #[test]
    fn the_wheel_scrolls_a_long_menu_body() {
        let ctx = crate::theme::ui_context();
        let height = 300.0;
        let frame = |events: Vec<egui::Event>| long_menu_frame(&ctx, height, 40, events);
        for _ in 0..3 {
            frame(Vec::new());
        }
        let (area, before) = frame(vec![egui::Event::PointerMoved(egui::pos2(60.0, 100.0))]);
        assert!(area.bottom() <= height + 0.5, "{area:?}");
        assert!(
            before.top() > height,
            "the last row starts in view: {before:?}"
        );
        let wheel = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -2000.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        };
        frame(vec![wheel]);
        let mut after = before;
        for _ in 0..30 {
            after = frame(Vec::new()).1;
        }
        assert!(
            after.bottom() <= height,
            "the last row stays hidden: {after:?}"
        );
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

/// Menu bar titles: size and spacing (about 25 px between titles, the
/// first one 11 px from the edge).
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
        w.weak_bg_fill = crate::theme::Tokens::menu_hover();
        w.bg_fill = crate::theme::Tokens::menu_hover();
        w.bg_stroke = egui::Stroke::new(1.0, crate::theme::Tokens::menu_hover_line());
        w.corner_radius = egui::CornerRadius::ZERO;
        w.expansion = 0.0;
        w.fg_stroke.color = crate::theme::on_highlight();
    }
    style.visuals.widgets.inactive.fg_stroke.color = crate::theme::Tokens::text();
    style.spacing.button_padding = egui::vec2(10.0, 3.0);
    style.spacing.item_spacing.x = 3.0;
}

pub fn menu_bar(app: &mut App, ui: &mut Ui) {
    set_highlight_new(app.highlight_whats_new);
    // Alt and a letter the user gave a command keeps running that command
    // instead of opening a menu; while Options waits for a shortcut's keys
    // no menu opens either.
    let capturing = app.shortcut_editor.capture.is_some();
    let user_alt_keys: Vec<egui::Key> = app
        .settings
        .shortcuts
        .iter()
        .filter_map(|(_, v)| crate::shortcuts::parse(v))
        .filter(|(m, _)| m.alt && !(m.ctrl || m.command || m.shift))
        .map(|(_, k)| k)
        .collect();
    let blocked =
        move |c: char| capturing || access::key_of(c).is_some_and(|k| user_alt_keys.contains(&k));
    egui::MenuBar::new()
        .style(menu_bar_style)
        .config(egui::containers::menu::MenuConfig::new().style(menu_popup_style))
        .ui(ui, |ui| {
            // Each title's access key, for Alt and the letter.
            let mut keys: Vec<char> = Vec::new();
            let mut menu = |ui: &mut Ui, key: &str, f: &mut dyn FnMut(&mut Ui)| -> egui::Response {
                let label = tr(key);
                let access = access::choose(&label, &keys);
                keys.push(access.map(|a| a.1).unwrap_or(' '));
                let title_bottom_id = ui.id().with(key).with("title_bottom");
                let title_bottom = ui.ctx().data(|d| d.get_temp::<f32>(title_bottom_id));
                let r = ui
                    .menu_button(
                        access::title(
                            &label,
                            access.map(|a| a.0),
                            BAR_FONT,
                            crate::theme::Tokens::text(),
                        ),
                        |ui| {
                            ui.set_min_width(MENU_WIDTH);
                            let limit = below_body_limit(ui, title_bottom);
                            body_within(ui, limit, |ui| f(ui))
                        },
                    )
                    .response;
                ui.ctx()
                    .data_mut(|d| d.insert_temp(title_bottom_id, r.rect.bottom()));
                if marked_new(key) {
                    // A dot over the title's top right corner.
                    ui.painter().circle_filled(
                        egui::pos2(r.rect.right() - 3.0, r.rect.top() + 4.0),
                        3.0,
                        Tokens::ACCENT,
                    );
                }
                r
            };
            let mut titles = vec![menu(ui, "menu.file", &mut |ui| file_menu(app, ui))];
            // With no drawing open only File, Tools, Window and Help remain.
            if app.has_document() {
                titles.push(menu(ui, "menu.edit", &mut |ui| edit_menu(app, ui)));
                titles.push(menu(ui, "menu.view", &mut |ui| view_menu(app, ui)));
                titles.push(menu(ui, "menu.layout", &mut |ui| layout_menu(app, ui)));
                titles.push(menu(ui, "menu.object", &mut |ui| object_menu(app, ui)));
                titles.push(menu(ui, "menu.effects", &mut |ui| effects_menu(app, ui)));
                titles.push(menu(ui, "menu.bitmaps", &mut |ui| bitmaps_menu(app, ui)));
                titles.push(menu(ui, "menu.text", &mut |ui| text_menu(app, ui)));
                titles.push(menu(ui, "menu.table", &mut |ui| table_menu(app, ui)));
            }
            titles.push(menu(ui, "menu.tools", &mut |ui| tools_menu(app, ui)));
            titles.push(menu(ui, "menu.window", &mut |ui| window_menu(app, ui)));
            titles.push(menu(ui, "menu.help", &mut |ui| help_menu(app, ui)));
            switch_on_hover(ui.ctx(), &titles);
            open_with_alt(ui.ctx(), &titles, &keys, &blocked);
        });
}

/// Alt+letter used by commands (symmetry, Alt+Q, Alt+Z): they keep it
/// over opening a menu.
const ALT_TAKEN: [char; 5] = ['e', 's', 'x', 'q', 'z'];

/// Alt and a title's access key open that menu (closing another one),
/// unless `blocked` keeps that letter for something else.
fn open_with_alt(
    ctx: &egui::Context,
    titles: &[egui::Response],
    keys: &[char],
    blocked: &dyn Fn(char) -> bool,
) {
    let popup_id = egui::Popup::default_response_id;
    let hit = keys.iter().zip(titles).find(|(c, _)| {
        !ALT_TAKEN.contains(c)
            && !blocked(**c)
            && access::key_of(**c).is_some_and(|key| {
                ctx.input(|i| {
                    i.events.iter().any(|e| {
                        matches!(e, egui::Event::Key { key: k, pressed: true, modifiers, .. }
                            if *k == key && modifiers.alt && !modifiers.ctrl && !modifiers.shift)
                    })
                })
            })
    });
    if let Some((_, r)) = hit {
        for t in titles {
            egui::Popup::close_id(ctx, popup_id(t));
        }
        egui::Popup::open_id(ctx, popup_id(r));
    }
}

/// With one menu open, pointing at another title of the bar opens that
/// menu instead, as desktop menu bars do.
fn switch_on_hover(ctx: &egui::Context, titles: &[egui::Response]) {
    let popup_id = egui::Popup::default_response_id;
    let Some(open) = titles
        .iter()
        .position(|r| egui::Popup::is_id_open(ctx, popup_id(r)))
    else {
        return;
    };
    if let Some(hovered) = titles.iter().position(|r| r.hovered()) {
        if hovered != open {
            egui::Popup::close_id(ctx, popup_id(&titles[open]));
            egui::Popup::open_id(ctx, popup_id(&titles[hovered]));
        }
    }
}

fn file_menu(app: &mut App, ui: &mut Ui) {
    // Commands that work on a drawing are off on the Welcome Screen alone.
    let doc = app.has_document();
    if item(ui, "menu.file.new", "Ctrl+N", true) {
        app.request_new_document();
    }
    if item(ui, "menu.file.new_from_template", "", true) {
        app.open_template_dialog();
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
                .rich_tip(p.display().to_string())
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
                crate::thumbnails::clear();
                app.recent_thumbs.clear();
                app.settings.save();
            }
        }
    });
    sub(ui, "menu.file.github", |ui| {
        if item(ui, "menu.file.github_open", "", true) {
            app.github.status.clear();
            app.dialog = Dialog::GithubOpen(app.github_form(false));
        }
        if item(ui, "menu.file.github_save", "", doc) {
            app.github.status.clear();
            app.dialog = Dialog::GithubSave(app.github_form(true));
        }
        if item(ui, "menu.file.github_history", "", app.remote.is_some()) {
            app.github.status.clear();
            app.github.history = None;
            app.dialog = Dialog::GithubHistory;
        }
    });
    sep(ui);
    if item(ui, "menu.file.close", "", doc) {
        app.close_document();
    }
    if item(ui, "menu.file.close_all", "", doc) {
        app.close_all_documents();
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
            // Read again (not the open drawing shown, see `open_path`).
            app.open_path_with_password(p, "");
        }
    }
    sep(ui);
    todo_sub(ui, "menu.file.acquire_image");
    if item(ui, "menu.file.import", "Ctrl+I", doc) {
        app.import();
    }
    if item(ui, "menu.file.export", "Ctrl+E", doc) {
        app.export();
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
        app.publish_pdf();
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
                crate::print_merge::perform(app, &st);
            }
        }
    });
    sep(ui);
    if item(ui, "menu.file.print", "Ctrl+P", doc) {
        app.print();
    }
    if item(ui, "menu.file.print_preview", "", doc) {
        app.print_preview(None);
    }
    if item(ui, "menu.file.preflight", "", doc) {
        app.dialog = Dialog::Preflight(Default::default());
    }
    sep(ui);
    if item(ui, "menu.file.document_properties", "", doc) {
        app.dialog = Dialog::DocumentProperties(Box::default());
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
        app.last_repeatable().is_some(),
    ) {
        app.repeat_last();
    }
    sep(ui);
    // Guidelines selected (no objects) are cut and copied too.
    let guides_picked = !app.selected_guides.is_empty();
    if item(ui, "menu.edit.cut", "Ctrl+X", has || guides_picked) {
        app.cut();
    }
    if item(ui, "menu.edit.copy", "Ctrl+C", has || guides_picked) {
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
    // Guidelines selected (no objects) duplicate too.
    if item(ui, "menu.edit.duplicate", "Ctrl+D", has || guides_picked) {
        app.duplicate();
    }
    if item(ui, "menu.edit.clone", "", app.can_clone()) {
        app.clone_selection();
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
    // Right after the view modes, where the report asks for it (#615).
    if check(ui, "menu.view.mask_outside_page", "", app.mask_outside_page) {
        app.mask_outside_page = !app.mask_outside_page;
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
        if app.page_sorter {
            app.multipage = false;
        }
    }
    if check(ui, "menu.view.multipage", "", app.multipage) {
        app.set_multipage(!app.multipage);
    }
    if check(
        ui,
        "menu.view.draw_in_perspective",
        "",
        app.perspective.is_some(),
    ) {
        app.toggle_perspective_drawing();
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

pub(crate) fn layout_menu(app: &mut App, ui: &mut Ui) {
    if item(ui, "menu.layout.insert_page", "", true) {
        app.dialog = Dialog::InsertPage {
            count: 1,
            after: true,
        };
    }
    if item(ui, "menu.layout.duplicate_page", "", true) {
        app.dialog = Dialog::duplicate_page();
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
        app.ask_delete_page();
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
    if item(
        ui,
        "menu.layout.fit_page_selection",
        "",
        !app.selection.is_empty(),
    ) {
        app.fit_page_to_selection();
    }
    if item(
        ui,
        "menu.layout.fit_page_drawing",
        "",
        app.drawing_bounds().is_some(),
    ) {
        app.fit_page_to_drawing();
    }
    if item(ui, "menu.layout.page_background", "", true) {
        app.dialog = Dialog::Options;
        app.options_page = crate::ui::dialogs::OptionsPage::Background;
    }
}

fn object_menu(app: &mut App, ui: &mut Ui) {
    let has = !app.selection.is_empty();
    let many = app.selection.len() > 1;
    // AI Generate: the docker in each mode, and Remove Background.
    sub(ui, "menu.object.ai_generate", |ui| {
        let doc = app.has_document();
        for (key, mode) in [
            ("menu.object.ai_create_image", crate::ai_image::Mode::Create),
            ("menu.object.ai_remix_image", crate::ai_image::Mode::Remix),
            (
                "menu.object.ai_create_vector",
                crate::ai_image::Mode::Vector,
            ),
            ("menu.object.ai_draw_words", crate::ai_image::Mode::Draw),
        ] {
            if item(ui, key, "", doc) {
                app.ai_image.mode = mode;
                app.ai_image.info = false;
                app.open_ai_generate();
            }
        }
        let cutout =
            app.ai_image_ready() && app.ai_source_bitmap().is_some() && !app.ai_image.busy();
        let tip = app.ai_image_tip();
        if item_tip(
            ui,
            "menu.bitmaps.remove_background",
            "",
            cutout,
            tip.as_deref(),
        ) {
            app.ai_remove_background();
        }
    });
    sub(ui, "menu.object.create", |ui| {
        if item(
            ui,
            "menu.object.create_arrowhead",
            "",
            app.selection.len() == 1,
        ) {
            app.create_arrowhead_from_selection();
        }
        // Pattern Fill asks whether the pattern is vector or bitmap.
        if item(
            ui,
            "menu.object.create_pattern",
            "",
            !app.selection.is_empty(),
        ) {
            app.dialog = Dialog::CreatePattern { vector: true };
        }
    });
    sub(ui, "menu.object.insert", |ui| {
        if item(ui, "menu.object.insert_barcode", "", true) {
            app.dialog = Dialog::Barcode(Default::default());
        }
        if item(ui, "menu.object.insert_qr", "", true) {
            app.insert_qr_code();
            // Its settings are edited in Properties.
            app.show_dockers = true;
            app.docker_tab = DockerTab::Properties;
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
        let masks: Vec<_> = app
            .selected_shapes()
            .iter()
            .filter(|s| matches!(s.kind, ShapeKind::ClipFrame { .. }))
            .map(|s| s.opacity_mask)
            .collect();
        let masked = !masks.is_empty() && masks.iter().all(|m| m.is_some());
        if check(ui, "menu.object.clip_frame_opacity_mask", "", masked) && is_clip {
            app.set_opacity_masks(!masked, false);
        }
        let inverted = masked && masks.iter().all(|m| m.is_some_and(|m| m.invert));
        if check(ui, "menu.object.clip_frame_invert_mask", "", inverted) && masked {
            app.set_opacity_masks(true, !inverted);
        }
    });
    sub(ui, "menu.object.symmetry", |ui| {
        let has_sym = app.selected_symmetry().is_some();
        let editing = app.symmetry_edit.is_some();
        if item(
            ui,
            "menu.object.symmetry_create",
            "Alt+S",
            has && !has_sym && !editing,
        ) {
            app.create_symmetry();
        }
        if item(
            ui,
            "menu.object.symmetry_edit",
            "Alt+E",
            has_sym && !editing,
        ) {
            app.edit_symmetry();
        }
        if item(ui, "menu.object.symmetry_finish", "Alt+Shift+E", editing) {
            app.finish_symmetry_edit();
        }
        if item(ui, "menu.object.symmetry_remove", "Alt+Shift+S", has_sym) {
            app.remove_symmetry();
        }
        if item(ui, "menu.object.symmetry_break", "Alt+X", has_sym) {
            app.break_symmetry_link();
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
    sub(ui, "menu.object.transformations", |ui| {
        use crate::app::TransformTab;
        for (key, shortcut, tab) in [
            ("menu.object.tf_position", "Alt+F7", TransformTab::Position),
            ("menu.object.tf_rotate", "Alt+F8", TransformTab::Rotate),
            ("menu.object.tf_scale", "Alt+F9", TransformTab::Scale),
            ("menu.object.tf_size", "Alt+F10", TransformTab::Size),
            ("menu.object.tf_skew", "", TransformTab::Skew),
        ] {
            if item(ui, key, shortcut, true) {
                crate::ui::transform_docker::open(app, tab, false);
            }
        }
    });
    if item(ui, "menu.object.clear_transformations", "", has) {
        app.clear_transformations();
    }
    sep(ui);
    sub(ui, "menu.object.copy_effect", |ui| {
        use crate::copy_effect::CopyKind;
        for kind in [CopyKind::Shadow, CopyKind::Transparency] {
            if item(ui, kind.key(), "", has) {
                app.pending_copy_effect = Some(kind);
                app.status = tr("status.click_source_object");
            }
        }
        if item(ui, "menu.object.copy_envelope_from", "", has) {
            app.start_copy_envelope();
        }
        for kind in CopyKind::LIVE {
            if item(ui, kind.key(), "", has) {
                app.pending_copy_effect = Some(kind);
                app.status = tr("status.click_source_object");
            }
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
            ("menu.object.distribute_left", Distribute::Left),
            ("menu.object.distribute_centers_h", Distribute::CentersH),
            ("menu.object.distribute_spacing_h", Distribute::SpacingH),
            ("menu.object.distribute_right", Distribute::Right),
            ("menu.object.distribute_top", Distribute::Top),
            ("menu.object.distribute_centers_v", Distribute::CentersV),
            ("menu.object.distribute_spacing_v", Distribute::SpacingV),
            ("menu.object.distribute_bottom", Distribute::Bottom),
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
    if item(ui, "menu.object.fit_objects_to_path", "", true) {
        app.open_docker(crate::app::DockerTab::FitToPath);
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
            let on = if need_two {
                op.ready(app.selection.len())
            } else {
                has
            };
            if item(ui, key, "", on) {
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
    // Break Apart, or the drop shadow's or effect group's version of it.
    let label = app.break_apart_label();
    if item(ui, label, "Ctrl+K", has) {
        app.break_apart();
    }
    if item(ui, "menu.object.add_perspective", "", has) {
        app.add_perspective();
    }
    sep(ui);
    if item(
        ui,
        "menu.object.convert_to_curves",
        "Ctrl+Q",
        app.can_convert_to_curves(),
    ) {
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
        app.docker_shown(DockerTab::Properties),
    ) {
        app.open_docker(DockerTab::Properties);
    }
    if check(
        ui,
        "menu.object.objects",
        "",
        app.docker_shown(DockerTab::Objects),
    ) {
        app.open_docker(DockerTab::Objects);
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
    // Bitmap effect groups.
    for (key, group) in crate::fx::groups() {
        sub(ui, key, |ui| {
            for fx in group.iter() {
                let shortcut = match fx.id {
                    "brightness_contrast_intensity" => "Ctrl+B",
                    "color_balance" => "Ctrl+Shift+B",
                    "hue_saturation_lightness" => "Ctrl+Shift+U",
                    _ => "",
                };
                // Effects with settings open a dialog, marked "...".
                let dots = if fx.params.is_empty() && fx.id != "tone_curve" {
                    ""
                } else {
                    "..."
                };
                let label = format!("{}{dots}", tr(&format!("fx.{}", fx.id)));
                // The row's picture is the effect itself, on a small
                // landscape (made once, when the menu first shows it).
                let picture = if invoke::active() {
                    None
                } else {
                    crate::ui::fx_icons::texture(ui.ctx(), fx.id)
                };
                // Bitmaps and vector objects alike (a vector object is
                // drawn as the picture its effects make of it).
                if item_picture(ui, &label, shortcut, has_bitmap || has, picture) {
                    crate::ui::effect_dialog::choose_effect(app, fx.id);
                }
            }
        });
    }
    sep(ui);
    // The effect panels: each opens its docker, checked while its tab is
    // in the docker column.
    for (key, shortcut, tab) in [
        ("menu.effects.brush_strokes", "", DockerTab::BrushStrokes),
        ("menu.effects.bevel", "", DockerTab::Bevel),
        ("menu.effects.blend", "", DockerTab::Blend),
        ("menu.effects.contour", "Ctrl+F9", DockerTab::Contour),
        ("menu.effects.envelope", "Ctrl+F7", DockerTab::Envelope),
        ("menu.effects.extrude", "", DockerTab::Extrude),
        ("menu.effects.lens", "Alt+F3", DockerTab::Lens),
        ("menu.effects.picture_mosaic", "", DockerTab::PictureMosaic),
        ("menu.effects.vector_mosaic", "", DockerTab::VectorMosaic),
    ] {
        if check(ui, key, shortcut, effect_docker_open(app, tab)) {
            app.open_docker(tab);
        }
    }
}

/// An effect panel's tab is in an open docker column, or floating.
pub(crate) fn effect_docker_open(app: &App, tab: DockerTab) -> bool {
    match app.docker_place(tab) {
        Some(crate::app::DockerPlace::Right) => app.show_dockers,
        Some(crate::app::DockerPlace::Left) => app.show_left_dockers,
        Some(crate::app::DockerPlace::Floating) => true,
        // Brought to the front and joining the default column.
        None => app.show_dockers && app.docker_tab == tab,
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
        app.open_resample();
    }
    // On this computer, no AI service needed.
    if item(ui, "menu.bitmaps.remove_background_local", "", has_bitmap) {
        app.dialog = Dialog::RemoveBackground(Default::default());
    }
    // AI: greyed until the AI service makes pictures (Tools > Options >
    // AI > Service); the tip says why.
    let cutout = app.ai_image_ready() && app.ai_source_bitmap().is_some() && !app.ai_image.busy();
    let tip = app.ai_image_tip();
    if item_tip(
        ui,
        "menu.bitmaps.remove_background",
        "",
        cutout,
        tip.as_deref(),
    ) {
        app.ai_remove_background();
    }
    let styled = app.ai_image_ready() && !app.selection.is_empty() && !app.ai_image.busy();
    sub(ui, "menu.bitmaps.ai_art_styles", |ui| {
        for (i, (key, _)) in crate::ai_image::ART_STYLES.iter().enumerate() {
            if item_tip(ui, key, "", styled, tip.as_deref()) {
                app.ai_art_style(i);
            }
        }
    });
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
    let last = app.last_trace.is_some();
    if item(ui, "menu.bitmaps.repeat_last_trace", "", has_bitmap && last) {
        app.repeat_last_trace();
    }
    if item(
        ui,
        "menu.bitmaps.last_trace_settings",
        "",
        has_bitmap && last,
    ) {
        app.open_last_trace_settings();
    }
    sub_if(ui, "menu.bitmaps.centerline_trace", has_bitmap, |ui| {
        for (key, preset) in CENTERLINE_TRACES {
            if item(ui, key, "", has_bitmap) {
                app.open_trace(preset);
            }
        }
    });
    sub_if(ui, "menu.bitmaps.outline_trace", has_bitmap, |ui| {
        for (key, preset) in OUTLINE_TRACES {
            if item(ui, key, "", has_bitmap) {
                app.open_trace(preset);
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
    if item(ui, "menu.text.tabs", "", true) {
        app.dialog = Dialog::TextTabs;
    }
    if item(ui, "menu.text.columns", "", true) {
        app.dialog = Dialog::TextColumns;
    }
    if item(ui, "menu.text.bullets", "", true) {
        app.dialog = Dialog::TextBullets;
    }
    if item(ui, "menu.text.drop_cap", "", true) {
        app.dialog = Dialog::TextDropCap;
    }
    if item(ui, "menu.text.text_statistics", "", true) {
        app.dialog = Dialog::TextStatistics;
    }
    if item(ui, "menu.text.font_sampler", "", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::FontSampler;
    }
    sep(ui);
    // Vertical text, offered with a CJK interface language or for text
    // in those scripts (#257).
    if app.vertical_text_offered() {
        let on = app.vertical_text_on();
        if check(ui, "menu.text.vertical_text", "", on) {
            app.set_vertical_text(!on);
        }
    }
    if item(ui, "menu.text.edit_text", "Ctrl+Shift+T", is_text) {
        app.open_edit_text_dialog();
    }
    if item(ui, "menu.text.insert_placeholder_text", "", is_para) {
        app.insert_placeholder_text();
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
    let joinable = !app.joinable_text_lines().is_empty();
    if item(ui, "menu.text.join_lines", "", joinable) {
        app.join_text_lines();
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
    // With a text and a path selected they go together; otherwise the
    // next click picks the path.
    if item(ui, "menu.text.fit_text_to_path", "", app.has_document()) {
        app.begin_fit_text_to_path();
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
    if mark_item_if(
        ui,
        "menu.text.align_to_baseline_grid",
        "",
        Mark::Check(on_grid),
        is_para,
    ) {
        app.toggle_baseline_grid();
    }
    sep(ui);
    if check(ui, "menu.text.use_hyphenation", "", app.uses_hyphenation()) {
        app.toggle_use_hyphenation();
    }
    if item(ui, "menu.text.custom_hyphens", "", true) {
        app.dialog = Dialog::CustomHyphens(Default::default());
    }
    sub(ui, "menu.text.writing_tools", |ui| {
        if item(ui, "menu.text.spell_check", "Ctrl+F12", true) {
            app.dialog =
                Dialog::WritingTools(crate::ui::writing_tools::WritingToolsState::on_tab(0));
        }
        if item(ui, "menu.text.grammatik", "", true) {
            app.dialog =
                Dialog::WritingTools(crate::ui::writing_tools::WritingToolsState::on_tab(1));
        }
        if item(ui, "menu.text.thesaurus", "", true) {
            app.dialog =
                Dialog::WritingTools(crate::ui::writing_tools::WritingToolsState::on_tab(2));
        }
        if item(ui, "menu.text.autocorrect", "", true) {
            app.dialog = Dialog::Options;
            app.options_page = crate::ui::dialogs::OptionsPage::Text;
            app.options_text_tab = crate::ui::options::AUTOCORRECT_TAB;
        }
    });
    sep(ui);
    if item(ui, "menu.text.change_case", "Shift+F3", is_text) {
        app.dialog = Dialog::ChangeCase(crate::ops2::CaseMode::Sentence);
    }
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
            width: app.table_size.0,
            height: app.table_size.1,
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
        sep(ui);
        if item(ui, "menu.table.insert_rows", "", in_table) {
            app.open_table_count(crate::ui::table_bar::TableCountKind::InsertRows);
        }
        if item(ui, "menu.table.insert_columns", "", in_table) {
            app.open_table_count(crate::ui::table_bar::TableCountKind::InsertCols);
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
        app.open_table_count(crate::ui::table_bar::TableCountKind::SplitRows);
    }
    if item(ui, "menu.table.split_into_columns", "", in_table) {
        app.open_table_count(crate::ui::table_bar::TableCountKind::SplitCols);
    }
}

/// Tools > Macros: the Macro Manager, Run Macro, the editor, loading a
/// project, and every loaded macro by project (so each can get a
/// shortcut in Options > Customization).
fn macros_items(app: &mut App, ui: &mut Ui) {
    app.macros.ensure_loaded();
    let running = app.macros.run.is_some();
    if item(ui, "menu.tools.macro_manager", "", true) {
        app.show_macro_manager();
    }
    if item(ui, "menu.tools.macros_run", "Alt+Shift+F8", true) {
        app.open_run_macro();
    }
    if item(ui, "menu.tools.macro_editor", "Alt+F11", true) {
        app.open_macro_editor();
    }
    if item(ui, "menu.tools.macros_load", "", true) {
        app.load_macro_project();
    }
    if item(ui, "menu.tools.macros_stop", "", running) {
        app.stop_macro();
    }
    let all: Vec<(String, Vec<(String, String)>)> = app
        .macros
        .projects
        .iter()
        .map(|p| (p.name().to_string(), p.macros()))
        .collect();
    if !all.is_empty() {
        sep(ui);
    }
    let mut run = None;
    for (i, (project, macros)) in all.iter().enumerate() {
        let key = crate::macros::project_menu_key(project);
        sub(ui, &key, |ui| {
            for (m, n) in macros {
                let id = crate::macros::command_key(project, m, n);
                if item_text(ui, &format!("{m}.{n}"), "", !running, None, &id) {
                    run = Some((i, m.clone(), n.clone()));
                }
            }
        });
    }
    if let Some((i, m, n)) = run {
        app.request_macro(i, &m, &n);
    }
}

fn tools_menu(app: &mut App, ui: &mut Ui) {
    sub(ui, "menu.tools.options", |ui| options_items(app, ui));
    if item(ui, "menu.tools.save_settings_as_default", "", true) {
        app.save_defaults();
    }
    sep(ui);
    if item(ui, "menu.tools.color_management", "", true) {
        app.dialog = Dialog::ColorManagement(Default::default());
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
    sub(ui, "menu.tools.macros", |ui| macros_items(app, ui));
    if item(ui, "menu.tools.ai_generate", "", true) {
        app.open_ai_generate();
    }
    if item(ui, "menu.tools.font_manager", "", true) {
        app.dialog = Dialog::FontManager(Default::default());
    }
    if item(ui, "menu.tools.border_and_grommet", "", true) {
        app.dialog = Dialog::BorderGrommet(Default::default());
    }
    sep(ui);
    let doc = app.has_document() && !app.show_welcome;
    if item(ui, "menu.tools.time_tracker", "", doc) {
        app.dialog = Dialog::TimeTracker(Default::default());
    }
}
/// Every bar with a check at each one shown (Window > Toolbars and the
/// bars' menu): the menu bar (always there), the standard toolbar, the
/// property bar, the toolbox, the colour palette, the AI prompt bar, the
/// status bar, the Text, Zoom and Transform toolbars and the time
/// tracker's bar.
pub(crate) fn toolbars_list(app: &mut App, ui: &mut Ui) {
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
    let palette = app.settings.palette.show_palette;
    if check(ui, "bars.palette", "", palette) {
        app.settings.palette.show_palette = !palette;
    }
    // Shown while an AI service is set up (AI Drawing).
    let ai_bar = app.settings.ai.show_bar;
    if check(ui, "bars.ai_prompt", "", ai_bar) {
        app.settings.ai.show_bar = !ai_bar;
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
    let tracker = app.settings.time_tracker.show_bar;
    if check(ui, "menu.window.toolbar_time_tracker", "", tracker) {
        app.settings.time_tracker.show_bar = !tracker;
    }
}

fn window_menu(app: &mut App, ui: &mut Ui) {
    let doc = app.has_document() && !app.show_welcome;
    if item(ui, "menu.window.new_window", "", doc) {
        app.new_window();
    }
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
    {
        use crate::ui::windows::{set_layout, WindowLayout};
        let has = app.has_document();
        let arranged = app.window_layout != WindowLayout::Tabs;
        for (key, layout) in [
            ("menu.window.cascade", WindowLayout::Cascade),
            (
                "menu.window.tile_horizontally",
                WindowLayout::TileHorizontal,
            ),
            ("menu.window.tile_vertically", WindowLayout::TileVertical),
        ] {
            if item(ui, key, "", has) {
                app.show_welcome = false;
                set_layout(app, layout);
            }
        }
        for key in ["menu.window.combine_windows", "menu.window.dock_window"] {
            if item(ui, key, "", arranged) {
                set_layout(app, WindowLayout::Tabs);
            }
        }
    }
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
            ("menu.window.workspace_touch", crate::app::Workspace::Touch),
        ] {
            if check(ui, key, "", app.workspace == ws) {
                app.set_workspace(ws);
            }
        }
        sep(ui);
        // Every part, whatever the options chose.
        if item(ui, "menu.window.workspace_export", "", true) {
            app.export_workspace_dialog(crate::workspace_file::WorkspaceParts::ALL);
        }
        if item(ui, "menu.window.workspace_import", "", true) {
            app.import_workspace_dialog(crate::workspace_file::WorkspaceParts::ALL, true);
        }
    });
    sep(ui);
    sub(ui, "menu.window.dockers", |ui| {
        let row = |ui: &mut Ui, app: &mut App, tab: DockerTab| {
            if check(ui, tab.key(), tab.shortcut(), app.docker_shown(tab)) {
                app.open_docker(tab);
            }
        };
        for (i, group) in DockerTab::MENU_GROUPS.iter().enumerate() {
            if i > 0 {
                sep(ui);
            }
            for tab in group.iter() {
                row(ui, app, *tab);
            }
            // The effect panels follow the text group, as a submenu.
            if i == 5 {
                sep(ui);
                sub(ui, "menu.window.dockers_effects", |ui| {
                    for tab in DockerTab::EFFECTS_MENU {
                        row(ui, app, tab);
                    }
                });
            }
        }
    });
    if check(ui, "menu.effects.picker", "", app.effect_picker.open) {
        app.effect_picker.open = !app.effect_picker.open;
    }
    sub(ui, "menu.window.toolbars", |ui| {
        toolbars_list(app, ui);
        sep(ui);
        let locked = app.settings.bars.locked;
        if check(ui, "bars.lock", "", locked) {
            app.settings.bars.locked = !locked;
        }
        if item(ui, "bars.reset", "", true) {
            crate::ui::bars::reset(app);
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
    if radio(ui, "menu.window.welcome_screen", "", app.show_welcome) {
        app.show_welcome = true;
    }
    // One entry per open drawing; a dot marks the one in front.
    for i in 0..app.docs.len() {
        let title = app.document_tab_title(i);
        let on = !app.show_welcome && i == app.active_doc;
        if menu_row(ui, &title, "", true, Mark::Radio(on)).0.clicked() {
            ui.close();
            app.switch_document(i);
        }
    }
}

fn help_menu(app: &mut App, ui: &mut Ui) {
    if item(ui, "menu.help.help_topics", "", true) {
        app.open_help(crate::help::HOME);
    }
    // The page of the active docker, else of the active tool (F1).
    if item(ui, "menu.help.product_help", "F1", true) {
        let topic = crate::help::context_topic(app);
        app.open_help(topic);
    }
    sep(ui);
    if item(ui, "menu.help.welcome_screen", "", true) {
        app.show_welcome = true;
    }
    if item(ui, "menu.help.video_tutorials", "", true) {
        app.show_welcome = true;
        app.welcome_tab = crate::app::WelcomeTab::Learn;
    }
    let hints_open = app.docker_shown(DockerTab::Hints);
    if check(ui, "menu.help.hints", "", hints_open) {
        if hints_open {
            app.close_docker(DockerTab::Hints);
        } else {
            app.open_docker(DockerTab::Hints);
        }
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
    if check(
        ui,
        "menu.help.highlight_whats_new",
        "",
        app.highlight_whats_new,
    ) {
        app.highlight_whats_new = !app.highlight_whats_new;
    }
    sep(ui);
    if item(ui, "menu.help.updates", "", true) {
        app.check_for_updates(Some(ui.ctx().clone()));
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
        app.dialog = Dialog::About;
    }
}

/// Tools > Options and the standard bar's Options button: the option
/// pages to open.
pub(crate) fn options_items(app: &mut App, ui: &mut Ui) {
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
    if item(ui, "menu.tools.options_ai", "", true) {
        app.dialog = Dialog::Options;
        app.options_page = crate::ui::dialogs::OptionsPage::AiGeneral;
    }
}
