//! The drawing window's edges, laid out: the
//! vertical scrollbar on the right; along the bottom the document
//! navigator (insert page before, first, previous, "1 of N", next, last,
//! insert page after), the page tabs, a splitter, the horizontal
//! scrollbar; and the Navigator button in the corner.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// Height of the bottom row and width of the vertical scrollbar.
pub const BAR: f32 = 17.0;

const ARROW: Color32 = Color32::from_rgb(0x60, 0x60, 0x60);
const TRACK: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF0);
const THUMB: Color32 = Color32::from_rgb(0xCD, 0xCD, 0xCD);
const THUMB_HOT: Color32 = Color32::from_rgb(0xA6, 0xA6, 0xA6);
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
    p.rect_filled(row, 0.0, Tokens::PANEL);
    p.rect_filled(vbar, 0.0, TRACK);
    p.rect_filled(corner, 0.0, Tokens::PANEL);
    p.hline(row.x_range(), row.min.y, Stroke::new(1.0, Tokens::BORDER));

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
    p.rect_filled(bar, 0.0, TRACK);
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
    p.rect_filled(thumb, 0.0, if hot { THUMB_HOT } else { THUMB });
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
    p.rect_filled(thumb, 0.0, if hot { THUMB_HOT } else { THUMB });
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
    let resp = ui.interact(r, egui::Id::new(id), Sense::click_and_drag());
    let p = ui.painter();
    if resp.is_pointer_button_down_on() {
        p.rect_filled(r, 0.0, THUMB_HOT);
    } else if resp.hovered() {
        p.rect_filled(r, 0.0, Tokens::TOOL_HOVER);
    }
    chevron(p, r.center(), dir, ARROW);
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
    let resp = ui.interact(grip, egui::Id::new("page_tabs_splitter"), Sense::drag());
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
                Tokens::TEXT_DIM,
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
            .color(Tokens::TEXT_DIM),
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
        p.rect_filled(r, 0.0, Tokens::TOOL_HOVER);
    }
    let c = r.center();
    let color = Tokens::TEXT_DIM;
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
    resp.on_hover_text(tip).clicked()
}

/// One tab per page, slanted at the sides; the current page's tab is white.
fn page_tabs(app: &mut App, ui: &mut Ui, area: Rect) {
    let names: Vec<String> = app.doc().pages.iter().map(|p| p.name.clone()).collect();
    let current = app.page_index();
    let mut x = area.min.x + 4.0;
    let mut clicked = None;
    let mut double = None;
    for (i, name) in names.iter().enumerate() {
        let galley = ui.painter().layout_no_wrap(
            name.clone(),
            egui::FontId::proportional(12.0),
            Tokens::TEXT,
        );
        let w = galley.size().x + 24.0;
        let r = Rect::from_min_max(Pos2::new(x, area.min.y), Pos2::new(x + w, area.max.y - 1.0));
        if r.min.x > area.max.x {
            break;
        }
        let resp = ui.interact(r, egui::Id::new(("page_tab", i)), Sense::click());
        let selected = i == current;
        let slant = 5.0;
        let shape = vec![
            Pos2::new(r.min.x, r.min.y),
            Pos2::new(r.max.x, r.min.y),
            Pos2::new(r.max.x - slant, r.max.y),
            Pos2::new(r.min.x + slant, r.max.y),
        ];
        let fill = if selected {
            Color32::WHITE
        } else if resp.hovered() {
            Tokens::TOOL_HOVER
        } else {
            Tokens::PANEL_DARK
        };
        let p = ui.painter();
        p.add(egui::epaint::PathShape::convex_polygon(
            shape.clone(),
            fill,
            Stroke::NONE,
        ));
        let edge = Stroke::new(1.0, Tokens::TEXT_DIM);
        p.line_segment([shape[0], shape[3]], edge);
        p.line_segment([shape[3], shape[2]], edge);
        p.line_segment([shape[2], shape[1]], edge);
        p.galley(
            Pos2::new(
                r.center().x - galley.size().x / 2.0,
                r.center().y - galley.size().y / 2.0,
            ),
            galley,
            Tokens::TEXT,
        );
        if resp.double_clicked() {
            double = Some(i);
        } else if resp.clicked() {
            clicked = Some(i);
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

#[cfg(test)]
mod tests {
    use crate::app::App;

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
