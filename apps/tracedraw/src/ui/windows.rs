//! Drawings side by side: Window > Cascade, Tile Horizontally and Tile
//! Vertically give every open drawing a window in the drawing area;
//! Combine Windows and Dock Window bring the tabs back. A window has a
//! title bar naming its drawing (an asterisk while it has unsaved
//! changes) with buttons to go back to tabs (as a double click on the
//! bar does) and to close the drawing. The active drawing's window is
//! drawn on top with its rulers, scrollbars and tools as usual; the
//! others show their page as their own view left it, and a click on one
//! makes it the active drawing. A new arrangement, or another number of
//! windows, fits every page into its window.

use crate::app::{App, Drag};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// How the drawings' windows are arranged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowLayout {
    /// One drawing at a time, chosen with the tabs.
    #[default]
    Tabs,
    /// Overlapping windows, each a step down and to the right.
    Cascade,
    /// One above the other, each as wide as the area.
    TileHorizontal,
    /// Side by side, each as tall as the area.
    TileVertical,
}

/// Height of a window's title bar.
pub const TITLE_H: f32 = 24.0;
/// How far each cascaded window sits from the one before.
const CASCADE_STEP: f32 = 28.0;

/// What a click in the windows asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Activate(usize),
    Close(usize),
    Maximize(usize),
}

/// The windows of `n` drawings in `area`, in the drawings' order.
pub fn window_rects(layout: WindowLayout, n: usize, area: Rect) -> Vec<Rect> {
    let n = n.max(1);
    match layout {
        WindowLayout::Tabs => vec![area; n],
        WindowLayout::TileHorizontal => {
            let h = area.height() / n as f32;
            (0..n)
                .map(|i| {
                    Rect::from_min_size(
                        area.min + Vec2::new(0.0, h * i as f32),
                        Vec2::new(area.width(), h),
                    )
                })
                .collect()
        }
        WindowLayout::TileVertical => {
            let w = area.width() / n as f32;
            (0..n)
                .map(|i| {
                    Rect::from_min_size(
                        area.min + Vec2::new(w * i as f32, 0.0),
                        Vec2::new(w, area.height()),
                    )
                })
                .collect()
        }
        WindowLayout::Cascade => {
            let size = Vec2::new(
                (area.width() * 0.72).max(area.width().min(320.0)),
                (area.height() * 0.72).max(area.height().min(240.0)),
            );
            // As many steps as keep a window inside the area; the next
            // windows start again at the top left.
            let room = (area.size() - size).max(Vec2::ZERO);
            let steps = (room.x.min(room.y) / CASCADE_STEP).floor() as usize + 1;
            (0..n)
                .map(|i| {
                    Rect::from_min_size(
                        area.min + Vec2::splat(CASCADE_STEP * (i % steps) as f32),
                        size,
                    )
                })
                .collect()
        }
    }
}

/// Fit every drawing's page into its window on the next frame.
fn fit_all(app: &mut App) {
    app.fit_pending = true;
    for slot in &mut app.docs {
        slot.fit_pending = true;
    }
}

/// Arrange the windows as `layout` says (Window > Cascade, Tile; Tabs for
/// Combine Windows and Dock Window): every page is fitted again.
pub fn set_layout(app: &mut App, layout: WindowLayout) {
    if app.window_layout != layout {
        app.window_layout = layout;
        app.window_layout_seen = None;
        fit_all(app);
    }
}

/// A title bar button: a square with `paint` drawn in it.
fn title_button(
    ui: &mut Ui,
    rect: Rect,
    id: egui::Id,
    tip: &str,
    paint: impl Fn(&egui::Painter, Rect, Color32),
) -> bool {
    let resp = ui.interact(rect, id, Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 2.0, Tokens::tool_hover());
    }
    paint(ui.painter(), rect.shrink(6.0), Tokens::icon());
    resp.rich_tip(tip).clicked()
}

/// A window's frame and title bar; returns the rectangle left for its
/// drawing.
fn frame(
    ui: &mut Ui,
    r: Rect,
    i: usize,
    title: &str,
    active: bool,
    act: &mut Option<Action>,
) -> Rect {
    let p = ui.painter();
    let bar = Rect::from_min_size(r.min, Vec2::new(r.width(), TITLE_H));
    p.rect_filled(
        bar,
        0.0,
        if active {
            crate::theme::Tokens::tab_active()
        } else {
            crate::theme::Tokens::tab_strip()
        },
    );
    let title_font = if active {
        crate::theme::bold(12.5)
    } else {
        egui::FontId::proportional(12.5)
    };
    let text_clip = Rect::from_min_max(bar.min, Pos2::new(bar.max.x - 2.0 * TITLE_H, bar.max.y));
    ui.painter_at(text_clip).text(
        Pos2::new(bar.left() + 8.0, bar.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        title_font,
        Tokens::text(),
    );
    let bar_resp = ui.interact(
        bar,
        egui::Id::new(("document_window_bar", i)),
        Sense::click(),
    );
    if bar_resp.double_clicked() {
        *act = Some(Action::Maximize(i));
    } else if bar_resp.clicked() && !active {
        *act = Some(Action::Activate(i));
    }
    let close = Rect::from_min_size(
        Pos2::new(bar.right() - TITLE_H, bar.top()),
        Vec2::splat(TITLE_H),
    );
    let max = close.translate(Vec2::new(-TITLE_H, 0.0));
    if title_button(
        ui,
        max,
        egui::Id::new(("document_window_max", i)),
        &tr("menu.window.combine_windows"),
        |p, r, c| {
            p.rect_stroke(r, 0.0, Stroke::new(1.2, c), egui::StrokeKind::Inside);
            p.line_segment(
                [
                    r.left_top() + Vec2::new(0.0, 2.0),
                    r.right_top() + Vec2::new(0.0, 2.0),
                ],
                Stroke::new(1.5, c),
            );
        },
    ) {
        *act = Some(Action::Maximize(i));
    }
    if title_button(
        ui,
        close,
        egui::Id::new(("document_window_close", i)),
        &tr("menu.window.close_window"),
        |p, r, c| {
            let s = Stroke::new(1.3, c);
            p.line_segment([r.left_top(), r.right_bottom()], s);
            p.line_segment([r.right_top(), r.left_bottom()], s);
        },
    ) {
        *act = Some(Action::Close(i));
    }
    let content = Rect::from_min_max(
        Pos2::new(r.left() + 1.0, bar.bottom()),
        r.max - Vec2::splat(1.0),
    );
    ui.painter().rect_stroke(
        r,
        0.0,
        Stroke::new(
            1.0,
            if active {
                Tokens::ACCENT
            } else {
                crate::theme::Tokens::window_border()
            },
        ),
        egui::StrokeKind::Inside,
    );
    content
}

/// Draw drawing `i`, which is not the active one, into `rect`: its page
/// as its own view shows it, fitted when its window is new.
fn paint_parked(app: &mut App, ui: &Ui, i: usize, rect: Rect) {
    let holder = app.peek(i);
    // What the active window is doing is not drawn here.
    let drag = std::mem::replace(&mut app.drag, Drag::None);
    let guides = std::mem::take(&mut app.selected_guides);
    let rotate = app.guide_rotate.take();
    let preview = std::mem::take(&mut app.docker_preview);
    app.anchor_view(rect.min);
    if app.fit_pending {
        app.view.fit(app.page_rect(), rect);
        app.fit_pending = false;
    }
    crate::canvas::draw_parked(app, &ui.painter_at(rect), rect);
    app.drag = drag;
    app.selected_guides = guides;
    app.guide_rotate = rotate;
    app.docker_preview = preview;
    app.end_peek(i, holder);
}

/// The drawing area with every drawing in a window (see the module notes).
pub fn arranged(app: &mut App, ui: &mut Ui, area: Rect) {
    let n = app.docs.len();
    if n == 0 {
        return;
    }
    let seen = (app.window_layout, n);
    if app.window_layout_seen != Some(seen) {
        app.window_layout_seen = Some(seen);
        fit_all(app);
    }
    ui.painter()
        .rect_filled(area, 0.0, crate::theme::Tokens::page_shadow());
    let rects = window_rects(app.window_layout, n, area);
    let active = app.active_doc.min(n - 1);
    let mut act = None;
    for i in (0..n).filter(|i| *i != active) {
        let r = rects[i];
        // Under the title bar's buttons, so these win.
        let resp = ui.interact(r, egui::Id::new(("document_window", i)), Sense::click());
        let title = app.document_tab_title(i);
        let content = frame(ui, r, i, &title, false, &mut act);
        paint_parked(app, ui, i, content);
        if resp.clicked() && act.is_none() {
            act = Some(Action::Activate(i));
        }
    }
    let r = rects[active];
    let title = app.document_tab_title(active);
    let content = frame(ui, r, active, &title, true, &mut act);
    crate::ui::drawing_window(app, ui, content);
    match act {
        Some(Action::Activate(i)) => app.switch_document(i),
        Some(Action::Close(i)) => {
            app.switch_document(i);
            app.close_document();
        }
        Some(Action::Maximize(i)) => {
            app.switch_document(i);
            set_layout(app, WindowLayout::Tabs);
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_tile_and_cascade_inside_the_area() {
        let area = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(900.0, 600.0));
        let h = window_rects(WindowLayout::TileHorizontal, 3, area);
        assert_eq!(h.len(), 3);
        assert!(h
            .iter()
            .all(|r| (r.width() - 900.0).abs() < 1e-3 && (r.height() - 200.0).abs() < 1e-3));
        assert!((h[2].bottom() - area.bottom()).abs() < 1e-3);
        let v = window_rects(WindowLayout::TileVertical, 2, area);
        assert!((v[1].left() - 460.0).abs() < 1e-3 && (v[1].right() - area.right()).abs() < 1e-3);
        let c = window_rects(WindowLayout::Cascade, 30, area);
        assert!(c.iter().all(|r| area.contains_rect(*r)));
        assert_eq!(c[1].min - c[0].min, Vec2::splat(CASCADE_STEP));
        // A small area keeps the windows at its size.
        let small = Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 150.0));
        let s = window_rects(WindowLayout::Cascade, 3, small);
        assert!(s.iter().all(|r| small.contains_rect(*r)));
    }

    /// Drawing the windows leaves every drawing as it was; a click on
    /// another window makes it active, and Combine Windows goes back to
    /// tabs with every page fitted again.
    #[test]
    fn tiled_windows_draw_every_drawing_and_switch() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let first = app.new_shape(tracedraw_core::ShapeKind::Rect {
            rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        });
        app.new_document();
        app.new_document();
        assert_eq!(app.docs.len(), 3);
        let active = app.active_doc;
        let page = app.page;
        let draw = |app: &mut App| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
            out.textures_delta.clear();
        };
        for layout in [
            WindowLayout::TileVertical,
            WindowLayout::TileHorizontal,
            WindowLayout::Cascade,
        ] {
            set_layout(&mut app, layout);
            draw(&mut app);
            draw(&mut app);
            assert_eq!(app.active_doc, active);
            assert_eq!(app.page, page);
            assert!(!app.fit_pending);
            assert!(app
                .docs
                .iter()
                .enumerate()
                .all(|(i, d)| i == active || !d.fit_pending));
        }
        // The first drawing still has its rectangle when it comes back.
        app.switch_document(0);
        let id = first.expect("shape");
        assert!(app.doc().find_shape(id).is_some());
        set_layout(&mut app, WindowLayout::Tabs);
        assert!(app.fit_pending && app.docs.iter().all(|d| d.fit_pending));
        draw(&mut app);
    }

    #[test]
    fn the_view_stays_put_in_a_window_that_moves() {
        let mut app = App::headless();
        app.anchor_view(Pos2::new(100.0, 50.0));
        let before = app.view.to_screen(tracedraw_core::Point::new(10.0, 10.0));
        app.anchor_view(Pos2::new(400.0, 80.0));
        let after = app.view.to_screen(tracedraw_core::Point::new(10.0, 10.0));
        assert_eq!(after - before, Vec2::new(300.0, 30.0));
    }
}
