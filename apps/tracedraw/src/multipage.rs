//! Multipage View (View > Multipage View): every page of the drawing laid
//! out in a grid on the desktop. The current page is edited as always;
//! the others are drawn beside it, and a click on one of them makes it
//! the current page without moving anything on screen.

use crate::app::App;
use crate::view::View;
use egui::{Color32, Painter, Pos2, Rect as ERect};
use tracedraw_core::geometry::{Rect, Size};
use tracedraw_core::PageId;

/// Space between pages, in millimetres.
pub const GAP_MM: f64 = 30.0;

/// Columns of the grid: always this many (fewer pages fill one row), so
/// a page added goes at the next place and the pages before it stay
/// where they were (#1018).
pub const COLUMNS: usize = 4;

/// The pages' places in the current page's coordinates (mm, Y up): a grid
/// of [`COLUMNS`] columns, left to right then down, each page at the top
/// left of its cell.
/// The current page is at the origin; an unknown `active` keeps the grid
/// where the first page is at the origin.
pub fn layout(pages: &[(PageId, Size)], active: PageId) -> Vec<(PageId, Rect)> {
    let n = pages.len();
    if n == 0 {
        return Vec::new();
    }
    let cols = n.clamp(1, COLUMNS);
    let rows = n.div_ceil(cols);
    let mut col_w = vec![0.0f64; cols];
    let mut row_h = vec![0.0f64; rows];
    for (i, (_, size)) in pages.iter().enumerate() {
        let (r, c) = (i / cols, i % cols);
        if let Some(w) = col_w.get_mut(c) {
            *w = w.max(size.width.max(0.0));
        }
        if let Some(h) = row_h.get_mut(r) {
            *h = h.max(size.height.max(0.0));
        }
    }
    let mut placed: Vec<(PageId, Rect)> = Vec::with_capacity(n);
    for (i, (id, size)) in pages.iter().enumerate() {
        let (r, c) = (i / cols, i % cols);
        let x0: f64 = col_w.iter().take(c).sum::<f64>() + c as f64 * GAP_MM;
        let top: f64 = -(row_h.iter().take(r).sum::<f64>() + r as f64 * GAP_MM);
        let w = size.width.max(0.0);
        let h = size.height.max(0.0);
        placed.push((*id, Rect::new(x0, top - h, x0 + w, top)));
    }
    let (dx, dy) = placed
        .iter()
        .find(|(id, _)| *id == active)
        .or(placed.first())
        .map(|(_, r)| (r.x0, r.y0))
        .unwrap_or((0.0, 0.0));
    placed
        .into_iter()
        .map(|(id, r)| (id, Rect::new(r.x0 - dx, r.y0 - dy, r.x1 - dx, r.y1 - dy)))
        .collect()
}

/// The page under a page-space point, if any.
pub fn page_at(placed: &[(PageId, Rect)], p: tracedraw_core::geometry::Point) -> Option<PageId> {
    placed
        .iter()
        .find(|(_, r)| p.x >= r.x0 && p.x <= r.x1 && p.y >= r.y0 && p.y <= r.y1)
        .map(|(id, _)| *id)
}

/// Everything the grid covers, for Zoom to Fit in this view.
pub fn bounds(placed: &[(PageId, Rect)]) -> Option<Rect> {
    let mut it = placed.iter().map(|(_, r)| *r);
    let first = it.next()?;
    Some(it.fold(first, |a, b| a.union(b)))
}

impl App {
    /// The grid of pages around the current one.
    pub fn multipage_layout(&self) -> Vec<(PageId, Rect)> {
        let pages: Vec<(PageId, Size)> = self.doc().pages.iter().map(|p| (p.id, p.size)).collect();
        layout(&pages, self.page)
    }

    /// Turn the Multipage View on or off; turning it on shows every page.
    pub fn set_multipage(&mut self, on: bool) {
        self.multipage = on;
        if !on {
            self.page_rasters.borrow_mut().clear();
            self.page_snapshot.borrow_mut().take();
        }
        if on {
            self.page_sorter = false;
        }
        self.fit_pending = true;
    }

    /// Make `page` the current page and keep every page where it is on
    /// screen: the view moves by the page's place in the grid.
    pub fn multipage_activate(&mut self, page: PageId) {
        if page == self.page {
            return;
        }
        let Some(r) = self
            .multipage_layout()
            .into_iter()
            .find(|(id, _)| *id == page)
            .map(|(_, r)| r)
        else {
            return;
        };
        self.view.origin = self
            .view
            .to_screen(tracedraw_core::geometry::Point::new(r.x0, r.y0));
        self.page = page;
        self.selection.clear();
        self.focus.clear();
        self.picked_layer = None;
    }

    /// In the Multipage View, a press on another page makes it current
    /// before the tool sees the press. True when the page changed.
    pub fn multipage_press(&mut self, s: Pos2) -> bool {
        if !self.multipage {
            return false;
        }
        let p = self.view.to_page(s);
        match page_at(&self.multipage_layout(), p) {
            Some(id) if id != self.page => {
                self.multipage_activate(id);
                true
            }
            _ => false,
        }
    }

    /// What Zoom to Fit and a new view fit: the page, or every page in the
    /// Multipage View.
    pub fn fit_rect(&self) -> Rect {
        if self.multipage {
            if let Some(b) = bounds(&self.multipage_layout()) {
                return b;
            }
        }
        self.page_rect()
    }
}

/// How long one frame may spend rendering the page and the pages beside
/// it, seconds; the pages left wait for the next frames, showing their
/// last picture meanwhile (#903). One render a frame always runs.
pub const RENDER_BUDGET: f64 = 0.010;

/// The part of a page to render, on screen: the whole paper while it
/// covers no more of the screen than the canvas does, so the picture
/// stays whole while the view moves; else only its visible part.
pub fn render_rect(paper: ERect, canvas: ERect) -> ERect {
    let clip = paper.intersect(canvas);
    let area = |r: ERect| r.width().max(0.0) * r.height().max(0.0);
    if area(paper) <= area(canvas) {
        paper
    } else {
        clip
    }
}

/// Paints the pages other than the current one: shadow, paper, objects
/// (each page kept as its own texture) and the page name below. While
/// the view moves each page shows its last picture scaled into place, and
/// the pages render anew once it rests, as many a frame as
/// [`RENDER_BUDGET`] allows, counted from `started` (before the current
/// page's render; `rendered` when that one rendered) (#902, #903).
pub fn draw_other_pages(
    app: &App,
    painter: &Painter,
    rect: ERect,
    started: web_time::Instant,
    rendered: bool,
) {
    let placed = app.multipage_layout();
    let doc = app.doc();
    let ctx = painter.ctx();
    let mut rasters = app.page_rasters.borrow_mut();
    rasters.retain(|id, _| doc.pages.iter().any(|p| p.id == *id));
    let mut rendered = rendered;
    for (index, (id, r)) in placed.iter().enumerate() {
        let paper = app.view.rect_to_screen(*r);
        let paper = ERect::from_min_max(paper.min.round(), paper.max.round());
        if *id != app.page && paper.expand(30.0).intersects(rect) {
            if app.show_page_border {
                painter.rect_filled(
                    paper
                        .expand(1.0)
                        .translate(crate::theme::Tokens::PAGE_SHADOW_OFFSET),
                    0.0,
                    crate::theme::Tokens::page_shadow(),
                );
            }
            painter.rect_filled(paper, 0.0, crate::theme::Tokens::PAGE);
            let clip = paper.intersect(rect);
            if clip.width() >= 1.0 && clip.height() >= 1.0 {
                let view = View {
                    zoom: app.view.zoom,
                    origin: paper.left_bottom(),
                };
                let raster = rasters.entry(*id).or_default();
                raster.follows_zoom = true;
                raster.defer = rendered && started.elapsed().as_secs_f64() > RENDER_BUDGET;
                // A render on its thread takes the drawing shared by all
                // the pages, copied once a change (the browser build draws
                // on the one thread).
                if cfg!(not(target_arch = "wasm32")) && !raster.defer && raster.has_picture() {
                    let revision = app.engine.revision();
                    let mut snap = app.page_snapshot.borrow_mut();
                    if snap.as_ref().is_none_or(|(r, _)| *r != revision) {
                        *snap = Some((revision, std::sync::Arc::new(doc.clone())));
                    }
                    raster.snapshot = snap.as_ref().map(|(_, d)| d.clone());
                }
                let before = raster.render_count();
                let shown = raster.texture(
                    ctx,
                    doc,
                    *id,
                    &view,
                    render_rect(paper, rect),
                    None,
                    app.engine.revision(),
                    app.wireframe,
                    app.simulate_overprints,
                    app.rasterize_complex_effects,
                    app.proof_colors,
                    None,
                    None,
                );
                rendered |= raster.render_count() != before;
                raster.defer = false;
                raster.snapshot = None;
                // The picture where its part of the page is, cut to the
                // page's visible part: an older picture shown while the
                // view moves covers another part of the page than now.
                if let Some((tex, at)) = shown {
                    painter.with_clip_rect(clip).image(
                        tex,
                        at,
                        ERect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
            }
            if app.show_page_border {
                painter.rect_stroke(
                    paper,
                    0.0,
                    egui::Stroke::new(1.0, crate::theme::Tokens::page_border()),
                    egui::StrokeKind::Outside,
                );
            }
        }
        if paper.expand(30.0).intersects(rect) {
            let name = doc
                .pages
                .get(index)
                .map(|p| p.name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| {
                    crate::i18n::trf("doc.page_n", &[("n", &(index + 1).to_string())])
                });
            let color = if *id == app.page {
                crate::theme::Tokens::text()
            } else {
                crate::theme::Tokens::text_dim()
            };
            painter.text(
                paper.left_bottom() + egui::vec2(0.0, 6.0),
                egui::Align2::LEFT_TOP,
                format!("{}: {name}", index + 1),
                egui::FontId::proportional(12.0),
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Point;

    fn pages(n: usize) -> Vec<(PageId, Size)> {
        (0..n)
            .map(|i| (PageId(i as u64 + 1), Size::new(100.0, 50.0)))
            .collect()
    }

    #[test]
    fn pages_sit_in_a_grid_with_the_current_one_at_the_origin() {
        let ps = pages(6);
        let placed = layout(&ps, PageId(1));
        // Four columns, 30 mm apart, then the next row going down.
        assert_eq!(placed[0].1, Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(placed[1].1, Rect::new(130.0, 0.0, 230.0, 50.0));
        assert_eq!(placed[3].1, Rect::new(390.0, 0.0, 490.0, 50.0));
        assert_eq!(placed[4].1, Rect::new(0.0, -80.0, 100.0, -30.0));
        // Another page current: the same grid, moved so it is at the origin.
        let placed = layout(&ps, PageId(6));
        assert_eq!(placed[5].1, Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(placed[0].1, Rect::new(-130.0, 80.0, -30.0, 130.0));
        assert_eq!(page_at(&placed, Point::new(-50.0, 100.0)), Some(PageId(1)));
        assert_eq!(page_at(&placed, Point::new(-20.0, 100.0)), None);
        assert_eq!(bounds(&placed), Some(Rect::new(-130.0, 0.0, 360.0, 130.0)));
        assert!(layout(&[], PageId(1)).is_empty());
    }

    /// Pages added one by one go at the next place of a grid of four
    /// columns; the pages before never move (#1018). The grid used to have
    /// as many columns as the square root of the page count, so the 4th
    /// and the 10th page rearranged every page.
    #[test]
    fn adding_pages_never_moves_the_pages_before() {
        for n in 1..=13 {
            let before = layout(&pages(n), PageId(1));
            let after = layout(&pages(n + 1), PageId(1));
            assert_eq!(after[..n], before[..], "page {} added", n + 1);
            let (r, c) = (n / COLUMNS, n % COLUMNS);
            let x0 = c as f64 * 130.0;
            let y0 = -(r as f64) * 80.0;
            assert_eq!(after[n].1, Rect::new(x0, y0, x0 + 100.0, y0 + 50.0));
        }
    }

    #[test]
    fn pages_of_other_sizes_take_the_widest_and_tallest_cell() {
        let ps = vec![
            (PageId(1), Size::new(100.0, 50.0)),
            (PageId(2), Size::new(40.0, 80.0)),
            (PageId(3), Size::new(150.0, 30.0)),
        ];
        let placed = layout(&ps, PageId(1));
        // One row: each column as wide as its page, every top lined up
        // with page 1's; the next row (page 5 on) would start below the
        // tallest, page 2.
        assert_eq!(placed[1].1, Rect::new(130.0, -30.0, 170.0, 50.0));
        assert_eq!(placed[2].1, Rect::new(200.0, 20.0, 350.0, 50.0));
        let mut five = ps.clone();
        five.push((PageId(4), Size::new(10.0, 10.0)));
        five.push((PageId(5), Size::new(150.0, 30.0)));
        let placed = layout(&five, PageId(1));
        // Column 0 is now as wide as page 5, which sits a row below page
        // 2's bottom.
        assert_eq!(placed[1].1, Rect::new(180.0, -30.0, 220.0, 50.0));
        assert_eq!(placed[4].1, Rect::new(0.0, -90.0, 150.0, -60.0));
    }

    #[test]
    fn a_small_page_renders_whole_and_a_large_one_its_visible_part() {
        let canvas = ERect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        // Small and partly off the canvas: the whole paper, so the picture
        // stays whole while the view moves.
        let small = ERect::from_min_size(egui::pos2(-50.0, 100.0), egui::vec2(200.0, 150.0));
        assert_eq!(render_rect(small, canvas), small);
        // Larger than the canvas: only what shows.
        let large = ERect::from_min_size(egui::pos2(-500.0, -500.0), egui::vec2(2000.0, 1500.0));
        assert_eq!(render_rect(large, canvas), canvas);
    }

    /// One frame of the whole window at egui time `time`.
    fn frame_at(ctx: &egui::Context, app: &mut App, time: f64) {
        let input = egui::RawInput {
            screen_rect: Some(ERect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 1000.0))),
            time: Some(time),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
    }

    fn renders(app: &App) -> u64 {
        app.page_rasters
            .borrow()
            .values()
            .map(|r| r.render_count())
            .sum()
    }

    /// While the view zooms, the pages beside the current one show their
    /// last pictures scaled into place and render nothing; once the view
    /// rests they render anew (#902, #903).
    #[test]
    fn other_pages_render_once_the_zoom_rests() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        for _ in 0..3 {
            app.add_page();
            app.new_shape(tracedraw_core::ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            });
        }
        app.page = app.doc().pages[0].id;
        app.set_multipage(true);
        let mut t = 0.0;
        for _ in 0..20 {
            t += 0.02;
            frame_at(&ctx, &mut app, t);
        }
        let at_rest = renders(&app);
        assert!(at_rest >= 3, "every other page rendered: {at_rest}");
        // Still at rest: nothing rendered again.
        t += 0.02;
        frame_at(&ctx, &mut app, t);
        assert_eq!(renders(&app), at_rest);
        // A zoom, frame by frame: nothing rendered.
        for _ in 0..5 {
            app.view.zoom_at(egui::pos2(800.0, 500.0), 0.9);
            t += 0.016;
            frame_at(&ctx, &mut app, t);
        }
        assert_eq!(renders(&app), at_rest, "no render while zooming");
        // At rest: each page rendered again, one render on a thread at a
        // time in the whole program (other tests' renders may hold the
        // turn for a moment).
        for _ in 0..400 {
            t += 0.02;
            frame_at(&ctx, &mut app, t);
            if renders(&app) >= at_rest + 3 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(renders(&app) >= at_rest + 3);
    }

    #[test]
    fn a_click_on_another_page_makes_it_current_without_moving_the_view() {
        let mut app = App::headless();
        app.add_page();
        let first = app.doc().pages[0].id;
        let second = app.doc().pages[1].id;
        app.page = first;
        app.set_multipage(true);
        app.fit_pending = false;
        let placed = app.multipage_layout();
        let r = placed[1].1;
        let on_second = app.view.to_screen(Point::new(r.x0 + 5.0, r.y0 + 5.0));
        let before = app.view.to_screen(Point::new(r.x0, r.y0));
        // A press on the current page changes nothing.
        let on_first = app.view.to_screen(Point::new(5.0, 5.0));
        assert!(!app.multipage_press(on_first));
        assert!(app.multipage_press(on_second));
        assert_eq!(app.page, second);
        // The second page's corner is now the origin, at the same place.
        let after = app.view.to_screen(Point::new(0.0, 0.0));
        assert!((after - before).length() < 1e-3);
        let p = app.view.to_page(on_second);
        assert!((p.x - 5.0).abs() < 1e-3 && (p.y - 5.0).abs() < 1e-3);
        // Fit covers both pages.
        assert!(app.fit_rect().width() > app.page_rect().width());
        // Off: a press does nothing, Fit is the page again.
        app.set_multipage(false);
        assert!(!app.multipage_press(on_first));
        assert_eq!(app.fit_rect(), app.page_rect());
    }
}
