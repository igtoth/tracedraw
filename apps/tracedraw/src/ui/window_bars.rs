//! The drawing window's edges, laid out: the
//! vertical scrollbar on the right; along the bottom the document
//! navigator (insert page before, first, previous, "1 of N", next, last,
//! insert page after), the page tabs, a splitter, the horizontal
//! scrollbar; and the Navigator button in the corner.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// Height of the bottom row and width of the vertical scrollbar.
pub const BAR: f32 = 17.0;
/// One click on a scroll arrow moves the view this many pixels.
const LINE: f32 = 40.0;

/// Draw the scrollbars, the navigator row and the corner button around
/// `canvas` inside `full`.
pub fn window_bars(app: &mut App, ui: &mut Ui, full: Rect, canvas: Rect, ruler: f32) {
    let p = ui.painter_at(full);
    let row = Rect::from_min_max(
        Pos2::new(full.min.x, full.max.y - BAR),
        Pos2::new(full.max.x - BAR, full.max.y),
    );
    let vbar = Rect::from_min_max(
        Pos2::new(full.max.x - BAR, full.min.y + ruler),
        Pos2::new(full.max.x, full.max.y - BAR),
    );
    let corner = Rect::from_min_max(Pos2::new(full.max.x - BAR, full.max.y - BAR), full.max);
    p.rect_filled(row, 0.0, Tokens::panel());
    p.rect_filled(vbar, 0.0, crate::theme::Tokens::panel());
    p.rect_filled(corner, 0.0, Tokens::panel());
    p.hline(row.x_range(), row.min.y, Stroke::new(1.0, Tokens::border()));

    // Bottom row: navigator, page tabs, splitter, horizontal scrollbar.
    let nav_w = 200.0_f32.min(row.width() * 0.4);
    let nav = Rect::from_min_max(
        Pos2::new(row.min.x + ruler, row.min.y),
        Pos2::new(row.min.x + ruler + nav_w, row.max.y),
    );
    let rest = Rect::from_min_max(Pos2::new(nav.max.x, row.min.y), row.max);
    let split_x = rest.min.x + rest.width() * app.page_tabs_fraction.clamp(0.1, 0.9);
    let tabs = Rect::from_min_max(rest.min, Pos2::new(split_x - 4.0, rest.max.y));
    let grip = Rect::from_min_max(
        Pos2::new(split_x - 4.0, rest.min.y),
        Pos2::new(split_x + 4.0, rest.max.y),
    );
    let hbar = Rect::from_min_max(Pos2::new(split_x + 4.0, rest.min.y + 1.0), rest.max);
    ui.scope_builder(egui::UiBuilder::new().max_rect(nav), |ui| {
        page_navigator(app, ui)
    });
    ui.scope_builder(egui::UiBuilder::new().max_rect(tabs), |ui| {
        ui.set_clip_rect(tabs);
        page_tabs(app, ui, tabs)
    });
    splitter(app, ui, grip, rest);
    horizontal(app, ui, hbar, canvas);
    vertical(app, ui, vbar, canvas);
    super::view_navigator(app, ui, corner, canvas);
}

/// The extent the scrollbars cover: the page plus one page size around
/// it, grown to include whatever is visible.
fn extent(app: &App, canvas: Rect) -> (f64, f64, f64, f64, f64, f64, f64, f64) {
    let page = app.page_rect();
    let desktop = page.inflate(page.width(), page.height());
    let v = app.view;
    let x0 = v.to_page(canvas.left_top()).x;
    let x1 = v.to_page(canvas.right_top()).x;
    let y1 = v.to_page(canvas.left_top()).y;
    let y0 = v.to_page(canvas.left_bottom()).y;
    (
        desktop.x0.min(x0),
        desktop.x1.max(x1),
        desktop.y0.min(y0),
        desktop.y1.max(y1),
        x0,
        x1,
        y0,
        y1,
    )
}

fn horizontal(app: &mut App, ui: &mut Ui, bar: Rect, canvas: Rect) {
    if bar.width() < BAR * 3.0 {
        return;
    }
    let left = Rect::from_min_size(bar.min, Vec2::new(BAR, bar.height()));
    let right = Rect::from_min_size(
        Pos2::new(bar.max.x - BAR, bar.min.y),
        Vec2::new(BAR, bar.height()),
    );
    let track = Rect::from_min_max(
        Pos2::new(left.max.x, bar.min.y),
        Pos2::new(right.min.x, bar.max.y),
    );
    let p = ui.painter_at(bar);
    p.rect_filled(bar, 0.0, crate::theme::Tokens::panel());
    if arrow_button(ui, left, "hscroll_left", Dir::Left) {
        app.view.pan(Vec2::new(LINE, 0.0));
    }
    if arrow_button(ui, right, "hscroll_right", Dir::Right) {
        app.view.pan(Vec2::new(-LINE, 0.0));
    }
    let (ex0, ex1, _, _, vx0, vx1, _, _) = extent(app, canvas);
    let w = (ex1 - ex0).max(1e-6);
    let t0 = ((vx0 - ex0) / w) as f32;
    let t1 = ((vx1 - ex0) / w) as f32;
    let thumb = Rect::from_min_max(
        Pos2::new(track.min.x + track.width() * t0, track.min.y + 2.0),
        Pos2::new(
            track.min.x + track.width() * t1.max(t0 + 0.03),
            track.max.y - 2.0,
        ),
    );
    let resp = ui.interact(track, egui::Id::new("hscroll"), Sense::click_and_drag());
    let hot = resp.dragged() || resp.hover_pos().is_some_and(|h| thumb.contains(h));
    p.rect_filled(
        thumb,
        0.0,
        if hot {
            crate::theme::Tokens::control_border()
        } else {
            crate::theme::Tokens::slider_rail()
        },
    );
    if resp.dragged() {
        let d = resp.drag_delta().x / track.width() * w as f32;
        app.view.pan(Vec2::new(-d * app.view.zoom, 0.0));
    } else if resp.clicked() {
        // A click on the track pages towards the click.
        if let Some(pos) = resp.interact_pointer_pos() {
            let page = canvas.width() * 0.9;
            if pos.x < thumb.min.x {
                app.view.pan(Vec2::new(page, 0.0));
            } else if pos.x > thumb.max.x {
                app.view.pan(Vec2::new(-page, 0.0));
            }
        }
    }
}

fn vertical(app: &mut App, ui: &mut Ui, bar: Rect, canvas: Rect) {
    if bar.height() < BAR * 3.0 {
        return;
    }
    let up = Rect::from_min_size(bar.min, Vec2::new(bar.width(), BAR));
    let down = Rect::from_min_size(
        Pos2::new(bar.min.x, bar.max.y - BAR),
        Vec2::new(bar.width(), BAR),
    );
    let track = Rect::from_min_max(
        Pos2::new(bar.min.x, up.max.y),
        Pos2::new(bar.max.x, down.min.y),
    );
    let p = ui.painter_at(bar);
    if arrow_button(ui, up, "vscroll_up", Dir::Up) {
        app.view.pan(Vec2::new(0.0, LINE));
    }
    if arrow_button(ui, down, "vscroll_down", Dir::Down) {
        app.view.pan(Vec2::new(0.0, -LINE));
    }
    let (_, _, ey0, ey1, _, _, vy0, vy1) = extent(app, canvas);
    let h = (ey1 - ey0).max(1e-6);
    // Page y grows upwards: the top of the bar is ey1.
    let s0 = ((ey1 - vy1) / h) as f32;
    let s1 = ((ey1 - vy0) / h) as f32;
    let thumb = Rect::from_min_max(
        Pos2::new(track.min.x + 2.0, track.min.y + track.height() * s0),
        Pos2::new(
            track.max.x - 2.0,
            track.min.y + track.height() * s1.max(s0 + 0.03),
        ),
    );
    let resp = ui.interact(track, egui::Id::new("vscroll"), Sense::click_and_drag());
    let hot = resp.dragged() || resp.hover_pos().is_some_and(|h| thumb.contains(h));
    p.rect_filled(
        thumb,
        0.0,
        if hot {
            crate::theme::Tokens::control_border()
        } else {
            crate::theme::Tokens::slider_rail()
        },
    );
    if resp.dragged() {
        let d = resp.drag_delta().y / track.height() * h as f32;
        app.view.pan(Vec2::new(0.0, -d * app.view.zoom));
    } else if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let page = canvas.height() * 0.9;
            if pos.y < thumb.min.y {
                app.view.pan(Vec2::new(0.0, page));
            } else if pos.y > thumb.max.y {
                app.view.pan(Vec2::new(0.0, -page));
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Dir {
    Left,
    Right,
    Up,
    Down,
}

/// A scroll arrow; repeats while held (after a short delay).
fn arrow_button(ui: &mut Ui, r: Rect, id: &str, dir: Dir) -> bool {
    let key = match dir {
        Dir::Left => "view.scroll_left",
        Dir::Right => "view.scroll_right",
        Dir::Up => "view.scroll_up",
        Dir::Down => "view.scroll_down",
    };
    let resp = ui
        .interact(r, egui::Id::new(id), Sense::click_and_drag())
        .rich_tip(crate::ui::tips::keyed(key));
    let p = ui.painter();
    if resp.is_pointer_button_down_on() {
        p.rect_filled(r, 0.0, crate::theme::Tokens::control_border());
    } else if resp.hovered() {
        p.rect_filled(r, 0.0, Tokens::tool_hover());
    }
    chevron(p, r.center(), dir, crate::theme::Tokens::text_dim());
    // Held: fire on press, then every 50 ms after 0.4 s.
    let held_id = egui::Id::new((id, "held"));
    let now = ui.input(|i| i.time);
    if resp.is_pointer_button_down_on() {
        let start = ui.data_mut(|d| *d.get_temp_mut_or_insert_with(held_id, || now));
        ui.ctx().request_repaint();
        if now == start {
            return true;
        }
        let since = now - start;
        if since > 0.4 {
            let last_id = egui::Id::new((id, "last"));
            let last = ui.data(|d| d.get_temp::<f64>(last_id)).unwrap_or(start);
            if now - last >= 0.05 {
                ui.data_mut(|d| d.insert_temp(last_id, now));
                return true;
            }
        }
        false
    } else {
        ui.data_mut(|d| d.remove::<f64>(held_id));
        false
    }
}

fn chevron(p: &egui::Painter, c: Pos2, dir: Dir, color: Color32) {
    let s = Stroke::new(1.4, color);
    let (a, b, d) = match dir {
        Dir::Left => (
            Vec2::new(2.0, -4.0),
            Vec2::new(2.0, 4.0),
            Vec2::new(-2.0, 0.0),
        ),
        Dir::Right => (
            Vec2::new(-2.0, -4.0),
            Vec2::new(-2.0, 4.0),
            Vec2::new(2.0, 0.0),
        ),
        Dir::Up => (
            Vec2::new(-4.0, 2.0),
            Vec2::new(4.0, 2.0),
            Vec2::new(0.0, -2.0),
        ),
        Dir::Down => (
            Vec2::new(-4.0, -2.0),
            Vec2::new(4.0, -2.0),
            Vec2::new(0.0, 2.0),
        ),
    };
    p.line_segment([c + a, c + d], s);
    p.line_segment([c + d, c + b], s);
}

/// The dotted grip between the page tabs and the horizontal scrollbar;
/// dragging it shares the row between them.
fn splitter(app: &mut App, ui: &mut Ui, grip: Rect, rest: Rect) {
    let resp = ui
        .interact(grip, egui::Id::new("page_tabs_splitter"), Sense::drag())
        .rich_tip(crate::ui::tips::keyed("view.page_tabs_splitter"));
    if resp.hovered() || resp.dragged() {
        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
    }
    if resp.dragged() {
        if let Some(pos) = resp.interact_pointer_pos() {
            app.page_tabs_fraction = ((pos.x - rest.min.x) / rest.width().max(1.0)).clamp(0.1, 0.9);
        }
    }
    let p = ui.painter();
    for row in 0..5 {
        for col in 0..2 {
            let c = Pos2::new(
                grip.center().x - 1.5 + col as f32 * 3.0,
                grip.min.y + 3.0 + row as f32 * 2.6,
            );
            p.rect_filled(
                Rect::from_center_size(c, Vec2::splat(1.0)),
                0.0,
                Tokens::text_dim(),
            );
        }
    }
}

/// Insert before, first, previous, "1 of N", next, last, insert after.
pub fn page_navigator(app: &mut App, ui: &mut Ui) {
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        let n = app.doc().pages.len();
        let idx = app.page_index();
        if nav_button(ui, Glyph::AddPage, &tr("status.insert_page_before")) {
            app.insert_page(false);
        }
        if nav_button(ui, Glyph::First, &tr("status.first_page")) {
            app.goto_page(0);
        }
        if nav_button(ui, Glyph::Previous, &tr("status.previous_page")) && idx > 0 {
            app.goto_page(idx - 1);
        }
        ui.label(
            egui::RichText::new(trf(
                "status.page_n_of_m",
                &[("n", &(idx + 1).to_string()), ("m", &n.to_string())],
            ))
            .size(11.0)
            .color(Tokens::text_dim()),
        );
        if nav_button(ui, Glyph::Next, &tr("status.next_page")) {
            app.goto_page(idx + 1);
        }
        if nav_button(ui, Glyph::Last, &tr("status.last_page")) {
            app.goto_page(n.saturating_sub(1));
        }
        if nav_button(ui, Glyph::AddPage, &tr("status.insert_page_after")) {
            app.insert_page(true);
        }
    });
}

#[derive(Clone, Copy)]
enum Glyph {
    AddPage,
    First,
    Previous,
    Next,
    Last,
}

fn nav_button(ui: &mut Ui, g: Glyph, tip: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(16.0, BAR - 1.0), Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(r, 0.0, Tokens::tool_hover());
    }
    let c = r.center();
    let color = Tokens::text_dim();
    let s = Stroke::new(1.0, color);
    let tri = |p: &egui::Painter, tip: f32| {
        // A filled triangle pointing left (tip < 0) or right.
        p.add(egui::epaint::PathShape::convex_polygon(
            vec![
                c + Vec2::new(tip * 3.5, 0.0),
                c + Vec2::new(-tip * 2.5, -4.5),
                c + Vec2::new(-tip * 2.5, 4.5),
            ],
            color,
            Stroke::NONE,
        ));
    };
    match g {
        Glyph::AddPage => {
            let page = Rect::from_center_size(c, Vec2::new(9.0, 11.0));
            p.rect_stroke(page, 0.0, s, egui::StrokeKind::Inside);
            p.line_segment([c + Vec2::new(-2.5, 0.5), c + Vec2::new(2.5, 0.5)], s);
            p.line_segment([c + Vec2::new(0.0, -2.0), c + Vec2::new(0.0, 3.0)], s);
        }
        Glyph::Previous => tri(p, -1.0),
        Glyph::Next => tri(p, 1.0),
        Glyph::First => {
            p.line_segment([c + Vec2::new(-4.5, -4.5), c + Vec2::new(-4.5, 4.5)], s);
            tri(p, -1.0);
        }
        Glyph::Last => {
            p.line_segment([c + Vec2::new(4.5, -4.5), c + Vec2::new(4.5, 4.5)], s);
            tri(p, 1.0);
        }
    }
    resp.rich_tip(tip).clicked()
}

/// One tab per page, slanted at the sides; the current page's tab is white.
/// The tabs that fit are shown from `App::page_tabs_first`: they slide so
/// the current page's tab shows whenever the page changes, and the mouse
/// wheel over them slides them one tab a notch (#794).
fn page_tabs(app: &mut App, ui: &mut Ui, area: Rect) {
    const SLANT: f32 = 5.0;
    let names: Vec<String> = app.doc().pages.iter().map(|p| p.name.clone()).collect();
    let current = app.page_index();
    let n = names.len();
    if n == 0 {
        return;
    }
    let font = egui::FontId::proportional(12.0);
    let width = |ui: &Ui, name: &str| {
        ui.painter()
            .layout_no_wrap(name.to_string(), font.clone(), Tokens::text())
            .size()
            .x
            + 24.0
    };
    let mut first = app.page_tabs_first.min(n.saturating_sub(1));
    // A new current page: slide just enough for its tab to show whole.
    if app.page_tabs_followed != Some(app.page) {
        app.page_tabs_followed = Some(app.page);
        if current < first {
            first = current;
        } else {
            let room = area.width() - 4.0;
            let widths: Vec<f32> = names[first..=current.min(n - 1).max(first)]
                .iter()
                .map(|name| width(ui, name) - SLANT)
                .collect();
            let mut used: f32 = widths.iter().sum::<f32>() + SLANT;
            let mut k = 0;
            while used > room && first + k < current {
                used -= widths[k];
                k += 1;
            }
            first += k;
        }
    }
    // The wheel over the tabs slides them, one tab a notch.
    if ui.rect_contains_pointer(area) {
        let step = ui.input(|i| {
            i.raw.events.iter().rev().find_map(|e| match e {
                egui::Event::MouseWheel { delta, .. } if delta.y != 0.0 => Some(delta.y),
                egui::Event::MouseWheel { delta, .. } if delta.x != 0.0 => Some(-delta.x),
                _ => None,
            })
        });
        if let Some(d) = step {
            // The scrollbars and the drawing do not scroll as well.
            crate::ui::field::take_wheel(ui);
            if d > 0.0 {
                first = first.saturating_sub(1);
            } else {
                first = (first + 1).min(n.saturating_sub(1));
            }
        }
    }
    app.page_tabs_first = first;
    let mut x = area.min.x + 4.0;
    let mut clicked = None;
    let mut double = None;
    for (i, name) in names.iter().enumerate().skip(first) {
        let galley = ui
            .painter()
            .layout_no_wrap(name.clone(), font.clone(), Tokens::text());
        let w = galley.size().x + 24.0;
        let r = Rect::from_min_max(Pos2::new(x, area.min.y), Pos2::new(x + w, area.max.y - 1.0));
        if r.min.x > area.max.x {
            break;
        }
        let resp = ui
            .interact(r, egui::Id::new(("page_tab", i)), Sense::click())
            .rich_tip(format!("{name}\n{}", tr("view.page_tab_desc")));
        let selected = i == current;
        let slant = SLANT;
        let shape = vec![
            Pos2::new(r.min.x, r.min.y),
            Pos2::new(r.max.x, r.min.y),
            Pos2::new(r.max.x - slant, r.max.y),
            Pos2::new(r.min.x + slant, r.max.y),
        ];
        let fill = if selected {
            crate::theme::Tokens::field()
        } else if resp.hovered() {
            Tokens::tool_hover()
        } else {
            Tokens::panel_dark()
        };
        let p = ui.painter();
        p.add(egui::epaint::PathShape::convex_polygon(
            shape.clone(),
            fill,
            Stroke::NONE,
        ));
        let edge = Stroke::new(1.0, Tokens::text_dim());
        p.line_segment([shape[0], shape[3]], edge);
        p.line_segment([shape[3], shape[2]], edge);
        p.line_segment([shape[2], shape[1]], edge);
        p.galley(
            Pos2::new(
                r.center().x - galley.size().x / 2.0,
                r.center().y - galley.size().y / 2.0,
            ),
            galley,
            Tokens::text(),
        );
        if resp.double_clicked() {
            double = Some(i);
        } else if resp.clicked() {
            clicked = Some(i);
        }
        // A right click on a tab makes its page the current one and opens
        // the page menu.
        if resp.secondary_clicked() {
            app.goto_page(i);
        }
        let mut action = None;
        resp.context_menu(|ui| action = page_tab_menu(app, ui));
        if let Some(a) = action {
            apply_page_tab_action(app, a);
        }
        x += w - slant;
    }
    if let Some(i) = clicked {
        app.goto_page(i);
    }
    if let Some(i) = double {
        // Double-clicking a page tab renames the page.
        app.goto_page(i);
        let name = app
            .doc()
            .pages
            .get(i)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        app.dialog = crate::ui::dialogs::Dialog::RenamePage { name };
    }
}

/// What the page tab menu asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageTabAction {
    Rename,
    InsertAfter,
    InsertBefore,
    Duplicate,
    Delete,
    SwitchOrientation,
}

/// The rows of the page tab menu, in order, by their label keys. Every
/// row has a picture and the menu has no separators; the page size and
/// background stay in the Layout menu. An ellipsis marks the rows that
/// open a dialog.
pub(crate) const PAGE_TAB_MENU: [(&str, PageTabAction); 6] = [
    ("menu.layout.rename_page", PageTabAction::Rename),
    ("page_tab.insert_after", PageTabAction::InsertAfter),
    ("page_tab.insert_before", PageTabAction::InsertBefore),
    ("menu.layout.duplicate_page", PageTabAction::Duplicate),
    ("page_tab.delete", PageTabAction::Delete),
    (
        "menu.layout.switch_orientation",
        PageTabAction::SwitchOrientation,
    ),
];

/// The menu of a page tab, for the current page.
pub(crate) fn page_tab_menu(app: &App, ui: &mut Ui) -> Option<PageTabAction> {
    let mut out = None;
    for (key, action) in PAGE_TAB_MENU {
        let enabled = action != PageTabAction::Delete || app.doc().pages.len() > 1;
        // A clicked row closes the menu itself.
        if crate::ui::menus::item(ui, key, "", enabled) {
            out = Some(action);
        }
    }
    out
}

/// Carry out a page tab menu command on the current page.
pub(crate) fn apply_page_tab_action(app: &mut App, action: PageTabAction) {
    use crate::ui::dialogs::Dialog;
    use tracedraw_core::Command;
    match action {
        PageTabAction::Rename => {
            let name = app
                .doc()
                .page(app.page)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            app.dialog = Dialog::RenamePage { name };
        }
        PageTabAction::InsertBefore => app.insert_page(false),
        PageTabAction::InsertAfter => app.insert_page(true),
        PageTabAction::Duplicate => app.dialog = Dialog::duplicate_page(),
        PageTabAction::Delete => {
            if app.doc().pages.len() > 1 {
                app.delete_page();
            }
        }
        PageTabAction::SwitchOrientation => {
            let s = app.page_size();
            let page = app.page;
            app.run(Command::ResizePage {
                page,
                size: tracedraw_core::geometry::Size::new(s.height, s.width),
            });
            app.fit_pending = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;

    #[test]
    fn the_page_tab_menu_inserts_duplicates_turns_and_deletes_pages() {
        use super::{apply_page_tab_action, PageTabAction as A};
        use crate::ui::dialogs::Dialog;
        let mut app = App::headless();
        let first = app.page;
        apply_page_tab_action(&mut app, A::InsertAfter);
        assert_eq!(app.doc().pages.len(), 2);
        assert_eq!(app.page_index(), 1);
        app.goto_page(0);
        apply_page_tab_action(&mut app, A::InsertBefore);
        assert_eq!(app.doc().pages.len(), 3);
        assert_eq!(app.page_index(), 0);
        assert_eq!(app.doc().pages[1].id, first);
        app.goto_page(1);
        // Duplicate opens its dialog with the defaults and changes nothing yet.
        apply_page_tab_action(&mut app, A::Duplicate);
        assert_eq!(app.doc().pages.len(), 3);
        assert_eq!(
            app.dialog,
            Dialog::DuplicatePage {
                before: false,
                contents: true
            }
        );
        app.dialog = Dialog::None;
        app.duplicate_page(false, true);
        assert_eq!(app.doc().pages.len(), 4);
        assert_eq!(app.page_index(), 2);
        let s = app.page_size();
        apply_page_tab_action(&mut app, A::SwitchOrientation);
        let t = app.page_size();
        assert!((s.width - t.height).abs() < 1e-9 && (s.height - t.width).abs() < 1e-9);
        apply_page_tab_action(&mut app, A::Delete);
        assert_eq!(app.doc().pages.len(), 3);
        apply_page_tab_action(&mut app, A::Rename);
        assert!(matches!(app.dialog, Dialog::RenamePage { .. }));
        // The last page is never deleted.
        let mut one = App::headless();
        apply_page_tab_action(&mut one, A::Delete);
        assert_eq!(one.doc().pages.len(), 1);
    }

    fn frame(ctx: &egui::Context, f: impl FnOnce(&mut egui::Ui)) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
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

    /// The rows as the menu draws them, found by clicking each in turn.
    fn replayed_rows(app: &App) -> Vec<(String, super::PageTabAction)> {
        use crate::ui::menus::replay;
        let ctx = crate::theme::ui_context();
        let mut out = Vec::new();
        for i in 0.. {
            replay::start(i);
            let mut action = None;
            frame(&ctx, |ui| action = super::page_tab_menu(app, ui));
            let (hit, rows) = replay::finish();
            if let (Some(k), Some(a)) = (hit, action) {
                out.push((k, a));
            }
            if i + 1 >= rows {
                break;
            }
        }
        out
    }

    #[test]
    fn the_page_tab_menu_has_six_rows_in_order_each_with_a_picture() {
        use super::PageTabAction as A;
        use crate::i18n::tr;
        let mut app = App::headless();
        app.insert_page(true);
        let rows = replayed_rows(&app);
        let keys: Vec<&str> = rows.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            [
                "menu.layout.rename_page",
                "page_tab.insert_after",
                "page_tab.insert_before",
                "menu.layout.duplicate_page",
                "page_tab.delete",
                "menu.layout.switch_orientation",
            ]
        );
        let actions: Vec<A> = rows.iter().map(|(_, a)| *a).collect();
        assert_eq!(
            actions,
            [
                A::Rename,
                A::InsertAfter,
                A::InsertBefore,
                A::Duplicate,
                A::Delete,
                A::SwitchOrientation
            ]
        );
        // Every row has a picture; the two insert rows have their own.
        use crate::ui::menu_glyphs::{for_key, Glyph};
        let ctx = crate::theme::ui_context();
        frame(&ctx, |ui| {
            let r = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(16.0, 16.0));
            for k in &keys {
                assert!(
                    crate::ui::menus::draw_menu_picture(ui.painter(), r, k, egui::Color32::GRAY),
                    "{k} has no picture"
                );
            }
        });
        assert_eq!(
            for_key("page_tab.insert_after"),
            Some(Glyph::PageInsertAfter)
        );
        assert_eq!(
            for_key("page_tab.insert_before"),
            Some(Glyph::PageInsertBefore)
        );
        assert_eq!(for_key("page_tab.delete"), Some(Glyph::PageDelete));
        assert_eq!(
            for_key("menu.layout.duplicate_page"),
            Some(Glyph::PageDuplicate)
        );
        // An ellipsis only where a dialog opens.
        for (k, a) in &rows {
            let dialog = matches!(a, A::Rename | A::Duplicate);
            assert_eq!(tr(k).ends_with("..."), dialog, "{k}: {}", tr(k));
        }
        // With one page, Delete is greyed and cannot be hit.
        let one = App::headless();
        let keys: Vec<String> = replayed_rows(&one).into_iter().map(|(k, _)| k).collect();
        assert_eq!(keys.len(), 5);
        assert!(!keys.iter().any(|k| k == "page_tab.delete"));
    }

    #[test]
    fn the_page_tab_menu_has_no_separators() {
        // The menu is exactly as tall as six rows stacked.
        let app = App::headless();
        let ctx = crate::theme::ui_context();
        let mut heights = [0.0f32; 3];
        frame(&ctx, |ui| {
            let measure = |ui: &mut egui::Ui, f: &mut dyn FnMut(&mut egui::Ui)| {
                ui.vertical(|ui| f(ui)).response.rect.height()
            };
            heights[0] = measure(ui, &mut |ui| {
                crate::ui::menus::item(ui, "menu.layout.rename_page", "", true);
            });
            heights[1] = measure(ui, &mut |ui| {
                crate::ui::menus::item(ui, "menu.layout.rename_page", "", true);
                crate::ui::menus::item(ui, "menu.layout.rename_page", "", true);
            });
            heights[2] = measure(ui, &mut |ui| {
                super::page_tab_menu(&app, ui);
            });
        });
        let step = heights[1] - heights[0];
        let six = heights[0] + 5.0 * step;
        assert!(
            (heights[2] - six).abs() < 0.5,
            "menu {} vs six rows {six}",
            heights[2]
        );
    }

    #[test]
    fn the_duplicate_page_dialog_puts_the_copy_where_asked_and_undoes() {
        use crate::ui::dialogs::Dialog;
        let mut app = App::headless();
        let first = app.page;
        app.new_shape(tracedraw_core::ShapeKind::Rect {
            rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        })
        .expect("shape");
        let count = |app: &App, i: usize| -> usize {
            app.doc().pages[i]
                .layers
                .iter()
                .map(|l| l.shapes.len())
                .sum()
        };
        assert_eq!(
            Dialog::duplicate_page(),
            Dialog::DuplicatePage {
                before: false,
                contents: true
            }
        );
        // Before, layers only: the copy is first, current and empty.
        app.duplicate_page(true, false);
        assert_eq!(app.doc().pages.len(), 2);
        assert_eq!(app.page_index(), 0);
        assert_ne!(app.page, first);
        assert_eq!(app.doc().pages[1].id, first);
        assert_eq!(count(&app, 0), 0);
        assert_eq!(count(&app, 1), 1);
        assert_eq!(
            app.doc().pages[0].layers.len(),
            app.doc().pages[1].layers.len()
        );
        app.undo();
        assert_eq!(app.doc().pages.len(), 1);
        // After, with contents: the copy follows and is current.
        app.page = first;
        app.duplicate_page(false, true);
        assert_eq!(app.doc().pages.len(), 2);
        assert_eq!(app.page_index(), 1);
        assert_eq!(app.doc().pages[0].id, first);
        assert_eq!(count(&app, 1), 1);
    }

    #[test]
    fn layout_duplicate_page_opens_the_dialog() {
        use crate::ui::dialogs::Dialog;
        use crate::ui::menus::replay;
        let mut app = App::headless();
        let ctx = crate::theme::ui_context();
        for i in 0.. {
            replay::start(i);
            frame(&ctx, |ui| crate::ui::menus::layout_menu(&mut app, ui));
            let (hit, rows) = replay::finish();
            if hit.as_deref() == Some("menu.layout.duplicate_page") {
                break;
            }
            assert!(i + 1 < rows, "no Duplicate Page row");
            app.dialog = Dialog::None;
        }
        assert_eq!(app.dialog, Dialog::duplicate_page());
        assert_eq!(app.doc().pages.len(), 1);
    }

    #[test]
    fn insert_page_before_and_after_the_current_one() {
        let mut app = App::headless();
        let first = app.page;
        app.insert_page(true);
        assert_eq!(app.page_index(), 1);
        let second = app.page;
        app.goto_page(0);
        app.insert_page(false);
        assert_eq!(app.page_index(), 0);
        let ids: Vec<_> = app.doc().pages.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[1], first);
        assert_eq!(ids[2], second);
    }

    #[test]
    fn the_page_border_and_shadow_are_hit_but_not_the_page_inside() {
        let app = App::headless();
        let paper = crate::canvas::page_screen_rect(&app);
        let right_mid = egui::pos2(paper.max.x, paper.center().y);
        assert!(crate::canvas::on_page_frame(&app, right_mid));
        // In the shadow band to the right.
        assert!(crate::canvas::on_page_frame(
            &app,
            right_mid + egui::vec2(4.0, 0.0)
        ));
        // Well inside the page, and far outside it: no.
        assert!(!crate::canvas::on_page_frame(&app, paper.center()));
        assert!(!crate::canvas::on_page_frame(
            &app,
            right_mid + egui::vec2(40.0, 0.0)
        ));
    }
}
