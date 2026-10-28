//! Drawing the page, objects, selection handles, nodes, rulers and grid with
//! the egui painter. Good enough for alpha; a real renderer comes later.

use crate::app::{App, Drag, Handle};
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::view::View;
use egui::{epaint, Color32, Painter, Pos2, Rect as ERect, Stroke as EStroke};
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Affine, BezPath, PathEl, Point, Rect},
    Color, Document,
};

pub fn to_color32(c: Color) -> Color32 {
    let [r, g, b] = c.to_rgb8();
    Color32::from_rgb(r, g, b)
}

pub fn flatten(path: &BezPath, view: &View) -> Vec<(Vec<Pos2>, bool)> {
    let mut out: Vec<(Vec<Pos2>, bool)> = Vec::new();
    let mut cur: Vec<Pos2> = Vec::new();
    let mut closed = false;
    let tol = 0.25 / view.zoom.max(0.05) as f64;
    kurbo::flatten(path.elements().iter().copied(), tol, &mut |el| match el {
        PathEl::MoveTo(p) => {
            if !cur.is_empty() {
                out.push((std::mem::take(&mut cur), closed));
            }
            closed = false;
            cur.push(view.to_screen(p));
        }
        PathEl::LineTo(p) => cur.push(view.to_screen(p)),
        PathEl::ClosePath => closed = true,
        _ => {}
    });
    if !cur.is_empty() {
        out.push((cur, closed));
    }
    out
}

/// Distance from a point to the nearest segment of a flattened path (mm).
pub fn distance_to_path(path: &BezPath, p: Point) -> f64 {
    let mut best = f64::MAX;
    let mut last: Option<Point> = None;
    let mut start: Option<Point> = None;
    kurbo::flatten(path.elements().iter().copied(), 0.05, &mut |el| match el {
        PathEl::MoveTo(q) => {
            last = Some(q);
            start = Some(q);
        }
        PathEl::LineTo(q) => {
            if let Some(a) = last {
                best = best.min(seg_dist(a, q, p));
            }
            last = Some(q);
        }
        PathEl::ClosePath => {
            if let (Some(a), Some(s)) = (last, start) {
                best = best.min(seg_dist(a, s, p));
            }
        }
        _ => {}
    });
    best
}

fn seg_dist(a: Point, b: Point, p: Point) -> f64 {
    let ab = b - a;
    let l2 = ab.hypot2();
    if l2 < 1e-12 {
        return (p - a).hypot();
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    (p - (a + ab * t)).hypot()
}

/// The page on screen, snapped to whole pixels.
pub fn page_screen_rect(app: &App) -> ERect {
    let r = app.view.rect_to_screen(app.page_rect());
    ERect::from_min_max(r.min.round(), r.max.round())
}

/// True when a screen point is on the page border (within 3 px) or on
/// its shadow: double-clicking there opens the page size options.
pub fn on_page_frame(app: &App, s: Pos2) -> bool {
    let paper = page_screen_rect(app);
    let near_border = paper.expand(3.0).contains(s) && !paper.shrink(3.0).contains(s);
    let shadow = paper.expand(1.0).translate(Tokens::PAGE_SHADOW_OFFSET);
    near_border || (shadow.contains(s) && !paper.contains(s))
}

pub fn draw_canvas(app: &App, painter: &Painter, rect: ERect) {
    let view = &app.view;
    let preview = match &app.drag {
        Drag::Move { total, .. } if total.hypot() > 0.0 => Some(Affine::translate(*total)),
        d @ (Drag::Scale { .. } | Drag::Rotate { .. } | Drag::Skew { .. }) => {
            app.preview_transform_of(d)
        }
        _ => None,
    };
    // While the selection is moved, scaled, turned or slanted, the
    // objects stay where they are and their outline shows where they
    // would go; the handles wait for the release.
    draw_page(app, painter, rect, None);
    let doc: &Document = app.doc();
    draw_text_frames(app, painter, preview);

    // Selection.
    match preview {
        Some(t) => draw_transform_outline(app, painter, t),
        None => draw_selection(app, painter, None),
    }
    draw_table_cells(app, painter);
    crate::table_tool::draw_table_drag(app, painter);
    crate::variable_outline::draw_width_nodes(app, painter);
    crate::comments::draw_comments(app, painter);
    crate::perspective_draw::draw_grid(app, painter, rect);
    draw_frame_links(app, painter);
    draw_drop_frame(app, painter);
    draw_symmetry_lines(app, painter, rect);
    draw_anchors(app, painter);
    draw_fountain_handles(app, painter);
    draw_shadow_handles(app, painter);
    draw_blend_controls(app, painter);

    // Fit Text to Path waiting for its path: the text on the one near.
    if app.pending_text_path {
        if let Some(p) = app.pointer_page {
            draw_text_path_preview(app, painter, p);
        }
    }
    // 3-point tools: base segment waiting for the third click, previewed to the pointer.
    if let (Some((a, b)), Some(c)) = (app.three_point_base, app.pointer_page) {
        // On a white line, as while it was dragged (#672).
        haloed_polyline(
            painter,
            &[view.to_screen(a), view.to_screen(b)],
            false,
            Color32::BLACK,
        );
        let mods = painter.ctx().input(|i| i.modifiers);
        let arc = app.tool == Tool::ThreePointCurve && app.constrain_held(&mods);
        let preview = three_point_preview(app.tool, a, b, c, arc);
        for (pts, closed) in flatten(&preview, view) {
            if pts.len() > 1 {
                haloed_polyline(painter, &pts, closed, Color32::BLACK);
            }
        }
    }

    // Rubber bands and in-progress tools.
    match &app.drag {
        Drag::Box {
            start,
            current,
            from_center,
        } => match app.creation_preview(*start, *current, *from_center) {
            // Shape tools: the shape itself follows the pointer.
            Some(path) => draw_creation_preview(app, painter, &path),
            None => {
                let page_rect = App::box_rect(*start, *current, *from_center);
                let r = view.rect_to_screen(page_rect);
                haloed_rect(painter, r, Tokens::SELECTION);
            }
        },
        Drag::Marquee { start, current, .. } => {
            // A dashed blue frame.
            let r = view.rect_to_screen(Rect::from_points(*start, *current));
            let pts = [
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
                r.left_top(),
            ];
            dashed_selection_line(painter, &pts);
        }
        Drag::Lasso { points, .. } => {
            let mut pts: Vec<Pos2> = points.iter().map(|p| view.to_screen(*p)).collect();
            if let Some(first) = pts.first().copied() {
                pts.push(first);
            }
            if pts.len() > 2 {
                dashed_selection_line(painter, &pts);
            }
        }
        Drag::AlongPath { start, current } => {
            draw_along_path(app, painter, *start, *current);
        }
        Drag::ZoomBox { start, current } => {
            let r = view.rect_to_screen(Rect::from_points(*start, *current));
            haloed_rect(painter, r, Tokens::SELECTION);
        }
        Drag::ThreePointBase { start, current } => {
            haloed_polyline(
                painter,
                &[view.to_screen(*start), view.to_screen(*current)],
                false,
                Color32::BLACK,
            );
        }
        Drag::Connector {
            from,
            start,
            current,
        } => {
            let origin = if app.tool == Tool::Blend {
                doc.shape(*from).ok().map(|(_, s)| s.bounds().center())
            } else {
                app.nearest_anchor(*from, *start)
            };
            if let Some(o) = origin {
                haloed_polyline(
                    painter,
                    &[view.to_screen(o), view.to_screen(*current)],
                    false,
                    Tokens::SELECTION,
                );
            }
        }
        Drag::TextFrame { start, current } => {
            let r = view.rect_to_screen(Rect::from_points(*start, *current));
            haloed_rect(painter, r, Color32::BLACK);
        }
        Drag::Freehand { points } => {
            // A black line on a white one, so the stroke shows over any
            // colour, a black fill too (#672).
            let pts: Vec<Pos2> = points.iter().map(|p| view.to_screen(*p)).collect();
            if pts.len() > 1 {
                haloed_polyline(painter, &pts, false, Color32::BLACK);
            }
        }
        Drag::FillGradient { start, current, .. } => {
            painter.line_segment(
                [view.to_screen(*start), view.to_screen(*current)],
                EStroke::new(1.0, Tokens::SELECTION),
            );
            painter.rect_filled(
                ERect::from_center_size(view.to_screen(*start), egui::vec2(8.0, 8.0)),
                0.0,
                Color32::BLACK,
            );
            painter.rect_filled(
                ERect::from_center_size(view.to_screen(*current), egui::vec2(8.0, 8.0)),
                0.0,
                Color32::WHITE,
            );
        }
        _ => {}
    }
    if let Some(c) = app.curve.as_ref().filter(|c| c.smooth && !c.bspline) {
        // Pen and Bezier: the segments placed in the outline the curve
        // will get, the Pen's dashed preview to the pointer, the nodes
        // and the last node's handles (see `pen_look`).
        let ctrl = painter.ctx().input(|i| app.constrain_held(&i.modifiers));
        let pointer = app
            .pointer_page
            .map(|p| app.curve_point(app.snap_point(p), ctrl));
        let preview = app.tool == crate::tools::Tool::Pen && app.settings.curves.pen_preview;
        let outline = app.default_stroke().map(|s| to_color32(s.color));
        let look = crate::pen_look::curve_look(c, pointer, preview, outline);
        // The placed segments in their outline's colour, on a white line
        // so a black outline shows over a black fill (#672).
        for (pts, _) in flatten(&look.placed, view) {
            if pts.len() > 1 {
                haloed_polyline(painter, &pts, false, look.placed_color);
            }
        }
        if let Some(seg) = &look.preview {
            // A solid line: white, and the preview colour on top at half
            // its width, so it shows over any colour (#489, #670); the
            // dashes are the control handles' look. Plain segments: a
            // path shape here cost a third of the Pen's frame and the
            // preview lagged behind the pointer (#560).
            for (pts, _) in flatten(seg, view) {
                for w in pts.windows(2) {
                    painter.line_segment([w[0], w[1]], EStroke::new(HALO_WIDTH, Color32::WHITE));
                }
                for w in pts.windows(2) {
                    painter
                        .line_segment([w[0], w[1]], EStroke::new(HALO_INNER, look.preview_color));
                }
            }
        }
        for (node, h) in &look.handles {
            draw_control_handle(
                painter,
                view.to_screen(*node),
                view.to_screen(*h),
                look.preview_color,
            );
        }
        for (p, mark) in &look.nodes {
            let r = ERect::from_center_size(view.to_screen(*p), egui::vec2(5.0, 5.0));
            match mark {
                crate::pen_look::NodeLook::Current => {
                    painter.rect_filled(r.expand(0.5), 0.0, look.preview_color);
                }
                crate::pen_look::NodeLook::Placed => {
                    painter.rect_filled(r, 0.0, Color32::WHITE);
                    painter.rect_stroke(
                        r,
                        0.0,
                        EStroke::new(1.0, look.placed_color),
                        epaint::StrokeKind::Middle,
                    );
                }
            }
        }
    } else if let Some(c) = &app.curve {
        // The pointer as the next node would take it (the constrain key:
        // in steps of the constrain angle).
        let ctrl = painter.ctx().input(|i| app.constrain_held(&i.modifiers));
        let preview = if c.dragging_handle {
            None
        } else {
            app.pointer_page
                .map(|p| app.curve_point(app.snap_point(p), ctrl))
        };
        let path = c.path(preview);
        if c.bspline {
            // The control polygon, dashed, as the curve is drawn.
            let mut poly: Vec<egui::Pos2> = c.nodes.iter().map(|n| view.to_screen(n.0)).collect();
            if let Some(p) = preview {
                poly.push(view.to_screen(p));
            }
            for w in poly.windows(2) {
                painter.extend(egui::Shape::dashed_line(
                    &[w[0], w[1]],
                    EStroke::new(1.0, Tokens::SELECTION),
                    4.0,
                    3.0,
                ));
            }
        }
        // A black line on a white one, as for the shapes being drawn, so
        // it shows over any colour (#672).
        for (pts, closed) in flatten(&path, view) {
            if pts.len() > 1 {
                haloed_polyline(painter, &pts, closed, Color32::BLACK);
            }
        }
        for (p, out) in &c.nodes {
            let sp = view.to_screen(*p);
            if let Some(o) = out {
                let so = view.to_screen(*o);
                let mirrored = sp - (so - sp);
                draw_control_handle(painter, sp, so, Tokens::SELECTION);
                draw_control_handle(painter, sp, mirrored, Tokens::SELECTION);
            }
            // A white edge round the hollow square, so it shows on a
            // black fill too (#672).
            let r = ERect::from_center_size(sp, egui::vec2(6.0, 6.0));
            painter.rect_stroke(
                r,
                0.0,
                EStroke::new(1.0, Color32::WHITE),
                epaint::StrokeKind::Outside,
            );
            painter.rect_stroke(
                r,
                0.0,
                EStroke::new(1.0, Tokens::text()),
                epaint::StrokeKind::Middle,
            );
        }
    }
    for p in &app.dimension_points {
        let sp = view.to_screen(*p);
        painter.circle_stroke(sp, 4.0, EStroke::new(1.0, Tokens::SELECTION));
    }
    // Text selection and caret.
    if app.text_edit.is_some() {
        if let Some((top, bottom, quads)) = app.text_caret_geometry() {
            for q in quads {
                let pts: Vec<Pos2> = q.iter().map(|p| view.to_screen(*p)).collect();
                painter.add(egui::Shape::convex_polygon(
                    pts,
                    TEXT_SELECTION,
                    EStroke::NONE,
                ));
            }
            draw_text_caret(painter, view.to_screen(top), view.to_screen(bottom));
            ime_at_caret(app, painter, view.to_screen(top), view.to_screen(bottom));
        }
    }
}

/// The caret of text being edited: black on a white edge, so it shows on
/// the white page and on dark fills alike (the interface's text colour
/// is light in the dark themes, #632).
pub(crate) fn draw_text_caret(painter: &Painter, top: Pos2, bottom: Pos2) {
    for stroke in TEXT_CARET {
        painter.line_segment([top, bottom], stroke);
    }
}

/// While text is typed on the canvas: let the system's input method
/// compose there (its candidate window follows the caret) and show the
/// text being composed, underlined, at the caret.
pub(crate) fn ime_at_caret(app: &App, painter: &Painter, top: Pos2, bottom: Pos2) {
    let ctx = painter.ctx();
    // A focused field of a docker or bar has the keyboard (and its own
    // input method position).
    if ctx.memory(|m| m.focused().is_some()) {
        return;
    }
    let caret = ERect::from_two_pos(top, bottom).expand2(egui::vec2(1.0, 0.0));
    ctx.output_mut(|o| {
        o.ime = Some(egui::output::IMEOutput {
            purpose: egui::IMEPurpose::Normal,
            rect: painter.clip_rect(),
            cursor_rect: caret,
            should_interrupt_composition: false,
        })
    });
    if app.ime_preedit.is_empty() {
        return;
    }
    let size = ((bottom.y - top.y).abs() * 0.8).clamp(10.0, 72.0);
    let galley = painter.layout_no_wrap(
        app.ime_preedit.clone(),
        egui::FontId::proportional(size),
        Tokens::text(),
    );
    let at = Pos2::new(caret.max.x + 1.0, caret.max.y - galley.size().y);
    let r = ERect::from_min_size(at, galley.size());
    painter.rect_filled(r.expand(1.0), 0.0, Color32::from_white_alpha(230));
    painter.galley(at, galley, Tokens::text());
    painter.line_segment(
        [Pos2::new(r.min.x, r.max.y), Pos2::new(r.max.x, r.max.y)],
        EStroke::new(1.0, Tokens::text()),
    );
}

/// The window's background, the page with its shadow and border, bleed
/// and printable area, the grids, and the objects rendered (`preview`
/// moves the selection while it is dragged).
fn draw_page(app: &App, painter: &Painter, rect: ERect, preview: Option<Affine>) {
    let view = &app.view;
    painter.rect_filled(rect, 0.0, app.desktop_color());
    // Objects, rasterized with tiny-skia into a texture; the live drag preview
    // is applied to the selection during the render. Rendered before the
    // other pages of the Multipage View so their renders know the time
    // this one took (#903); painted after the page and its grids.
    let started = web_time::Instant::now();
    let renders_before = app.raster.borrow().render_count();
    // Editing a symmetry with Show full preview off: drawn without its
    // copies (the canvas outlines them in red); a revision of its own so
    // the picture is not taken for the full one.
    let without_copies = app.symmetry_without_copies();
    let doc: &Document = without_copies.as_deref().unwrap_or(app.doc());
    let revision = app.engine.revision()
        ^ if without_copies.is_some() {
            1u64 << 62
        } else {
            0
        };
    // Paragraph text being resized keeps its letters while the frame
    // follows the drag (the text flows anew on release).
    let preview_ids = preview.map(|t| {
        let ids: Vec<_> = app
            .selection
            .iter()
            .copied()
            .filter(|id| !app.resizes_text_frame(*id, t))
            .collect();
        (ids, t)
    });
    let main = app.raster.borrow_mut().texture(
        painter.ctx(),
        doc,
        app.page,
        view,
        rect,
        preview_ids,
        revision,
        app.wireframe,
        app.simulate_overprints,
        app.rasterize_complex_effects,
        app.proof_colors,
        app.render_focus(),
        (app.view_mode == crate::app::ViewMode::Pixels).then(|| app.document_dpi()),
    );
    let main_rendered = app.raster.borrow().render_count() != renders_before;
    if app.multipage {
        crate::multipage::draw_other_pages(app, painter, rect, started, main_rendered);
    }

    // Page: a flat grey shadow offset right and down, the white page, a
    // one-pixel grey border (pixel-aligned so it stays crisp). While
    // Autofit Page's popup is open the page shows at the size it would
    // take, around the drawing (#594, #604).
    let shown = app
        .autofit_preview_rect(painter.ctx().cumulative_frame_nr())
        .unwrap_or_else(|| app.page_rect());
    let paper = {
        let r = view.rect_to_screen(shown);
        ERect::from_min_max(r.min.round(), r.max.round())
    };
    if app.show_page_border {
        painter.rect_filled(
            paper.expand(1.0).translate(Tokens::PAGE_SHADOW_OFFSET),
            0.0,
            Tokens::page_shadow(),
        );
    }
    painter.rect_filled(paper, 0.0, Tokens::PAGE);
    if app.show_page_border {
        painter.rect_stroke(
            paper,
            0.0,
            EStroke::new(1.0, Tokens::page_border()),
            epaint::StrokeKind::Outside,
        );
    }
    if app.show_bleed {
        let b = app.doc().metadata.bleed.max(0.0);
        if b > 0.0 {
            let r = view.rect_to_screen(shown.inflate(b, b));
            painter.rect_stroke(
                r,
                0.0,
                EStroke::new(1.0, Tokens::page_border()),
                epaint::StrokeKind::Outside,
            );
        }
    }
    if app.show_printable_area {
        // Printable area: the page minus the printer's typical 5 mm hardware margin.
        let r = view.rect_to_screen(shown.inflate(-5.0, -5.0));
        let dash = 4.0;
        for (a, b) in [
            (r.left_top(), r.right_top()),
            (r.right_top(), r.right_bottom()),
            (r.right_bottom(), r.left_bottom()),
            (r.left_bottom(), r.left_top()),
        ] {
            let len = (b - a).length();
            let n = (len / (dash * 2.0)).floor() as usize;
            for k in 0..n {
                let t0 = (k as f32 * dash * 2.0) / len;
                let t1 = ((k as f32 * dash * 2.0) + dash) / len;
                painter.line_segment(
                    [a + (b - a) * t0, a + (b - a) * t1],
                    EStroke::new(1.0, Tokens::text_dim()),
                );
            }
        }
    }

    if app.show_grid {
        draw_grid(app, painter, rect);
    }
    if app.show_baseline_grid {
        draw_baseline_grid(app, painter, rect, paper);
    }

    if let Some((tex, at)) = main {
        painter.image(
            tex,
            at,
            ERect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    if app.mask_outside_page {
        mask_outside_pages(app, painter, rect, paper);
    }
    draw_pixel_grid(app, painter, rect);
    draw_empty_clip_frames(app, painter);
}

/// The colour Mask Outside Page paints when the desktop is white or
/// nearly (the default in the light theme): a near-black that makes the
/// page stand out, as the reference program shows it (#615).
pub(crate) const OUTSIDE_PAGE_DARK: Color32 = Color32::from_gray(0x22);

/// The colour of Mask Outside Page: the desktop colour, or
/// [`OUTSIDE_PAGE_DARK`] when the desktop is as light as the paper, so
/// the page never melts into what surrounds it.
pub(crate) fn outside_page_mask_color(app: &App) -> Color32 {
    let d = app.desktop_color();
    if d.r() >= 0xE0 && d.g() >= 0xE0 && d.b() >= 0xE0 {
        OUTSIDE_PAGE_DARK
    } else {
        d.to_opaque()
    }
}

/// Mask Outside Page (#615): an opaque colour over the window outside
/// the page (every page in Multipage View), the page border over the
/// objects, and the selected objects' outlines in the selection blue over
/// the mask, so parts off the page can still be found and picked. A few
/// rectangles a frame and the outlines flattened once per change.
fn mask_outside_pages(app: &App, painter: &Painter, rect: ERect, paper: ERect) {
    let mut pages = vec![paper];
    if app.multipage {
        pages.extend(
            app.multipage_layout()
                .iter()
                .filter(|(id, _)| *id != app.page)
                .map(|(_, r)| {
                    let r = app.view.rect_to_screen(*r);
                    ERect::from_min_max(r.min.round(), r.max.round())
                })
                .filter(|r| r.intersects(rect)),
        );
    }
    let mask = outside_page_mask_color(app);
    for r in outside(rect, &pages) {
        painter.rect_filled(r, 0.0, mask);
    }
    for p in &pages {
        painter.rect_stroke(
            *p,
            0.0,
            EStroke::new(1.0, Tokens::page_border()),
            epaint::StrokeKind::Outside,
        );
    }
    let stroke = EStroke::new(1.0, Tokens::SELECTION);
    let view = &app.view;
    let seen = rect.expand(2.0);
    for line in app.selected_outlines().iter() {
        let pts: Vec<Pos2> = line.iter().map(|p| view.to_screen(*p)).collect();
        for w in pts.windows(2) {
            if seen.contains(w[0]) || seen.contains(w[1]) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
    }
}

/// The parts of `area` no rectangle of `holes` covers, as rectangles:
/// bands between the holes' top and bottom edges, each cut at the holes
/// crossing it.
pub(crate) fn outside(area: ERect, holes: &[ERect]) -> Vec<ERect> {
    let holes: Vec<ERect> = holes
        .iter()
        .map(|h| h.intersect(area))
        .filter(|h| h.width() > 0.0 && h.height() > 0.0)
        .collect();
    let mut ys: Vec<f32> = vec![area.top(), area.bottom()];
    for h in &holes {
        ys.push(h.top());
        ys.push(h.bottom());
    }
    ys.sort_by(f32::total_cmp);
    ys.dedup();
    let mut out = Vec::new();
    for band in ys.windows(2) {
        let (y0, y1) = (band[0], band[1]);
        if y1 <= y0 {
            continue;
        }
        let mut cuts: Vec<(f32, f32)> = holes
            .iter()
            .filter(|h| h.top() < y1 && h.bottom() > y0)
            .map(|h| (h.left(), h.right()))
            .collect();
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut x = area.left();
        for (l, r) in cuts {
            if l > x {
                out.push(ERect::from_min_max(Pos2::new(x, y0), Pos2::new(l, y1)));
            }
            x = x.max(r);
        }
        if x < area.right() {
            out.push(ERect::from_min_max(
                Pos2::new(x, y0),
                Pos2::new(area.right(), y1),
            ));
        }
    }
    out
}

/// The drawing of a window that is not the active one (Window > Cascade
/// or Tile): its page, grids and objects as its view shows them, and its
/// guidelines; nothing of the editing going on in the active window.
pub fn draw_parked(app: &App, painter: &Painter, rect: ERect) {
    draw_page(app, painter, rect, None);
    draw_text_frames(app, painter, None);
    draw_guides(app, painter, rect);
}

/// Liang-Barsky clip of a screen segment against a rectangle.
fn clip_segment(a: Pos2, b: Pos2, r: ERect) -> Option<(Pos2, Pos2)> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let mut t0 = 0.0f32;
    let mut t1 = 1.0f32;
    for (p, q) in [
        (-dx, a.x - r.left()),
        (dx, r.right() - a.x),
        (-dy, a.y - r.top()),
        (dy, r.bottom() - a.y),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            if t > t1 {
                return None;
            }
            t0 = t0.max(t);
        } else {
            if t < t0 {
                return None;
            }
            t1 = t1.min(t);
        }
    }
    Some((
        Pos2::new(a.x + dx * t0, a.y + dy * t0),
        Pos2::new(a.x + dx * t1, a.y + dy * t1),
    ))
}

/// The marquee's and the lasso's line: blue dashes on a white line so it
/// shows on any drawing.
fn dashed_selection_line(painter: &Painter, pts: &[Pos2]) {
    painter.add(epaint::PathShape::line(
        pts.to_vec(),
        EStroke::new(1.0, Color32::from_white_alpha(200)),
    ));
    painter.extend(egui::Shape::dashed_line(
        pts,
        EStroke::new(1.0, Tokens::SELECTION),
        4.0,
        3.0,
    ));
}

/// Outline of the shape a 3-point tool would create for base `a -> b` and third point `c`.
/// The outline of the object being drawn, in the colour and width of the
/// default outline (at least one pixel wide; the selection colour when new
/// objects get no outline), so the preview looks like the result.
/// Over it, a black line on a white one, so the shape's edge shows on
/// any outline and any colour under it (#670).
fn draw_creation_preview(app: &App, painter: &Painter, path: &BezPath) {
    let view = &app.view;
    let stroke = match &app.default_stroke() {
        Some(s) => EStroke::new((s.width as f32 * view.zoom).max(1.0), to_color32(s.color)),
        None => EStroke::new(1.0, Tokens::SELECTION),
    };
    for (pts, closed) in flatten(path, view) {
        if pts.len() < 2 {
            continue;
        }
        if closed {
            painter.add(epaint::PathShape::closed_line(pts.clone(), stroke));
        } else {
            painter.add(epaint::PathShape::line(pts.clone(), stroke));
        }
        haloed_polyline(painter, &pts, closed, Color32::BLACK);
    }
}

/// The frame `r` as a [`haloed_polyline`] in `inner`.
fn haloed_rect(painter: &Painter, r: ERect, inner: Color32) {
    let pts = [
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
    ];
    haloed_polyline(painter, &pts, true, inner);
}

/// The width of the white line under an overlay's coloured one, and of
/// the coloured one: half of it, as the reference program draws its
/// overlays (#670).
pub(crate) const HALO_WIDTH: f32 = 2.0;
pub(crate) const HALO_INNER: f32 = 1.0;

/// An overlay line that shows on any colour: `inner` on a white line
/// twice as wide. A short line is plain segments, which cost less than a
/// path shape in an overlay redrawn every frame (#560); a long one (a
/// freehand stroke, many placed segments) two path shapes, which cost
/// less than twice its segments (`frame_times_of_the_pen_preview`, #672).
/// Either way one call to the painter.
pub(crate) fn haloed_polyline(painter: &Painter, pts: &[Pos2], closed: bool, inner: Color32) {
    let segs = || {
        pts.windows(2).map(|w| [w[0], w[1]]).chain(
            closed
                .then(|| [pts[pts.len() - 1], pts[0]])
                .filter(|_| pts.len() > 2),
        )
    };
    if pts.len() > 8 {
        let shape = |stroke: EStroke| {
            if closed {
                epaint::PathShape::closed_line(pts.to_vec(), stroke)
            } else {
                epaint::PathShape::line(pts.to_vec(), stroke)
            }
        };
        painter.extend([
            egui::Shape::Path(shape(EStroke::new(HALO_WIDTH, Color32::WHITE))),
            egui::Shape::Path(shape(EStroke::new(HALO_INNER, inner))),
        ]);
        return;
    }
    // One call for every segment, the white ones first.
    let white = EStroke::new(HALO_WIDTH, Color32::WHITE);
    let line = EStroke::new(HALO_INNER, inner);
    painter.extend(
        segs()
            .map(|points| egui::Shape::LineSegment {
                points,
                stroke: white,
            })
            .chain(segs().map(|points| egui::Shape::LineSegment {
                points,
                stroke: line,
            })),
    );
}

fn three_point_preview(
    tool: Tool,
    a: Point,
    b: Point,
    c: Point,
    arc: bool,
) -> tracedraw_core::geometry::BezPath {
    use tracedraw_core::geometry::Vec2;
    let base = b - a;
    let len = base.hypot().max(1e-6);
    let dir = base / len;
    let normal = Vec2::new(-dir.y, dir.x);
    let height = (c - a).dot(normal);
    match tool {
        Tool::ThreePointCurve => crate::app::three_point_curve(a, b, c, arc),
        _ => {
            let local = Rect::new(0.0, height.min(0.0), len, height.max(0.0));
            let t = Affine::translate(a.to_vec2()) * Affine::rotate(base.atan2());
            let p = if tool == Tool::ThreePointEllipse {
                tracedraw_core::geometry::ellipse_path(local)
            } else {
                tracedraw_core::geometry::rect_path(local, 0.0)
            };
            t * p
        }
    }
}

/// Dashed screen-space line (guides, mirror lines).
fn dash(painter: &Painter, a: Pos2, b: Pos2, color: Color32) {
    let len = a.distance(b);
    let n = (len / 6.0).ceil().max(1.0) as usize;
    for i in (0..n).step_by(2) {
        let t0 = i as f32 / n as f32;
        let t1 = ((i + 1) as f32 / n as f32).min(1.0);
        painter.line_segment(
            [a + (b - a) * t0, a + (b - a) * t1],
            EStroke::new(1.0, color),
        );
    }
}

/// Mirror lines of selected objects with a Symmetry effect (dashed, with a
/// centre handle), shown in symmetry mode.
/// The clip frame the selection is being moved over: its outline filled
/// in the selection colour, and under the pointer what a drop does (as
/// it is, or with W or K held).
fn draw_drop_frame(app: &App, painter: &Painter) {
    let Some(frame) = app.drop_frame else {
        return;
    };
    let Some(s) = app.doc().find_shape(frame) else {
        return;
    };
    let accent = Tokens::ACCENT;
    for (pts, closed) in flatten(&s.page_path(), &app.view) {
        if pts.len() < 2 {
            continue;
        }
        if closed {
            painter.add(egui::Shape::convex_polygon(
                pts.clone(),
                accent.gamma_multiply(0.25),
                EStroke::NONE,
            ));
        }
        painter.add(egui::Shape::line(pts, EStroke::new(1.5, accent)));
    }
    let Some(p) = app.pointer_page else {
        return;
    };
    let at = app.view.to_screen(p) + egui::vec2(18.0, 24.0);
    let (w, k) = app.drop_keys;
    let lines = [
        ("hint.drop_frame_place", !w && !k),
        ("hint.drop_frame_add", w && !k),
        ("hint.drop_frame_replace", k),
    ];
    let font = egui::FontId::proportional(12.0);
    let galleys: Vec<_> = lines
        .iter()
        .map(|(key, on)| {
            let col = if *on {
                Tokens::text()
            } else {
                Tokens::text_dim()
            };
            painter.layout_no_wrap(
                format!("\u{2022} {}", crate::i18n::tr(key)),
                font.clone(),
                col,
            )
        })
        .collect();
    let width = galleys.iter().map(|g| g.size().x).fold(0.0f32, f32::max);
    let height: f32 = galleys.iter().map(|g| g.size().y + 2.0).sum();
    let bg = ERect::from_min_size(at, egui::vec2(width + 12.0, height + 8.0));
    painter.rect(
        bg,
        2.0,
        Tokens::panel(),
        EStroke::new(1.0, Tokens::border()),
        egui::StrokeKind::Inside,
    );
    let mut y = bg.top() + 4.0;
    for g in galleys {
        let h = g.size().y;
        painter.galley(egui::pos2(bg.left() + 6.0, y), g, Tokens::text());
        y += h + 2.0;
    }
}

fn draw_symmetry_lines(app: &App, painter: &Painter, rect: ERect) {
    let view = &app.view;
    let blue = Color32::from_rgb(40, 110, 220);
    let red = Color32::from_rgb(0xE0, 0x10, 0x10);
    // Editing a symmetry: its copies as red outlines while Show full
    // preview is off (the renderer leaves them out then).
    for c in app.symmetry_copies_outlined() {
        for (pts, closed) in flatten(&c.page_path(), view) {
            if pts.len() < 2 {
                continue;
            }
            let mut pts = pts;
            if closed {
                if let Some(first) = pts.first().copied() {
                    pts.push(first);
                }
            }
            painter.add(egui::Shape::line(pts, EStroke::new(1.0, red)));
        }
    }
    // The lines of the symmetry being edited, whatever is selected; a
    // symmetry merely selected shows none, so after Finish it reads as one
    // object, as in the reference program.
    let mut shown: Vec<(Point, f64, u8)> = Vec::new();
    if let Some((_, center, angle, lines)) = app.edited_symmetry() {
        if app.settings.symmetry_show_lines {
            shown.push((center, angle, lines));
        }
    }
    for (center, angle, lines) in shown {
        let n = lines.clamp(1, 12) as usize;
        let far = 100_000.0 / view.zoom.max(0.01) as f64;
        for k in 0..n {
            let a = (angle + 180.0 * k as f64 / n as f64).to_radians();
            let d = tracedraw_core::geometry::Vec2::new(a.cos(), a.sin());
            let s0 = view.to_screen(center - d * far);
            let s1 = view.to_screen(center + d * far);
            if let Some((c0, c1)) = clip_segment(s0, s1, rect) {
                dash(painter, c0, c1, blue);
            }
        }
        let c = view.to_screen(center);
        painter.circle_stroke(c, 5.0, EStroke::new(1.5, blue));
    }
    // The turning handles, after a click on a line.
    for h in app.symmetry_handles() {
        let q = view.to_screen(h);
        painter.circle_filled(q, 4.5, Color32::WHITE);
        painter.circle_stroke(q, 4.5, EStroke::new(1.5, blue));
    }
}

/// Interactive Fill: the fill's nodes as the reference shows them (#330):
/// the line from the start to the end (an arrow at the end), a thin line
/// to the round skew handle, the start and end nodes as squares in the
/// first and last colours, and for an elliptical or square fill its
/// outline dashed (the frame's unit circle or square).
fn draw_fountain_handles(app: &App, painter: &Painter) {
    // The Transparency tool shows its mask's nodes the same way (#448).
    if app.tool != Tool::InteractiveFill && app.tool != Tool::Transparency {
        return;
    }
    let Some((_, f, b)) = app.selected_fountain() else {
        return;
    };
    let view = &app.view;
    let handles = crate::fill_tool::fountain_handles(&f, b);
    let [(_, s), (_, e), (_, k)] = handles.as_slice() else {
        return;
    };
    let (ss, se, sk) = (view.to_screen(*s), view.to_screen(*e), view.to_screen(*k));
    let line = EStroke::new(1.0, Tokens::SELECTION);
    // The outline of an elliptical or square fill, dashed.
    if matches!(
        f.kind,
        tracedraw_core::FountainKind::Radial | tracedraw_core::FountainKind::Square
    ) {
        let (ev, kv) = (*e - *s, *k - *s);
        let at = |u: f64, v: f64| view.to_screen(*s + ev * u + kv * v);
        let pts: Vec<egui::Pos2> = if f.kind == tracedraw_core::FountainKind::Radial {
            (0..=64)
                .map(|i| {
                    let a = i as f64 / 64.0 * std::f64::consts::TAU;
                    at(a.cos(), a.sin())
                })
                .collect()
        } else {
            vec![
                at(1.0, 1.0),
                at(-1.0, 1.0),
                at(-1.0, -1.0),
                at(1.0, -1.0),
                at(1.0, 1.0),
            ]
        };
        dashed_polyline(painter, &pts, line, 4.0, 3.0);
    }
    // The axis, an arrow at the end.
    painter.line_segment([ss, se], line);
    let d = se - ss;
    if d.length() > 12.0 {
        let u = d.normalized();
        let n = egui::vec2(-u.y, u.x);
        let tip = se - u * 6.0;
        painter.add(epaint::PathShape::convex_polygon(
            vec![tip, tip - u * 8.0 + n * 4.0, tip - u * 8.0 - n * 4.0],
            Tokens::SELECTION,
            EStroke::NONE,
        ));
    }
    // The skew handle on its thin line.
    painter.line_segment(
        [ss, sk],
        EStroke::new(1.0, Tokens::SELECTION.gamma_multiply(0.6)),
    );
    painter.circle_filled(sk, 4.5, Color32::WHITE);
    painter.circle_stroke(sk, 4.5, line);
    // The midpoint sliders: a short bar across the line between each
    // pair of nodes.
    let d = se - ss;
    if d.length() > 24.0 {
        let n = egui::vec2(-d.y, d.x).normalized() * 5.0;
        for (_, t) in crate::fill_tool::midpoint_params(&f) {
            let m = ss + d * t as f32;
            painter.line_segment([m - n, m + n], EStroke::new(2.0, Tokens::SELECTION));
        }
    }
    // The colour nodes in their colours, the start and the end bigger;
    // the picked one drawn with a thick outline.
    let last = f.stops.len().saturating_sub(1);
    for (i, st) in f.stops.iter().enumerate() {
        let p = if i == 0 {
            ss
        } else if i == last {
            se
        } else {
            ss + d * st.pos as f32
        };
        let size = if i == 0 || i == last { 9.0 } else { 7.0 };
        let [r, g, bl] = st.color.to_rgb8();
        let rect = ERect::from_center_size(p, egui::vec2(size, size));
        painter.rect_filled(rect, 0.0, Color32::from_rgb(r, g, bl));
        let picked = app.fill_node == Some(i);
        let stroke = if picked {
            EStroke::new(2.0, Tokens::SELECTION)
        } else {
            line
        };
        painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Outside);
    }
}

/// Drop Shadow tool: the selected shadow's start handle (white), end
/// handle (the shadow colour) and the dashed line between them with the
/// opacity slider on it.
fn draw_shadow_handles(app: &App, painter: &Painter) {
    use crate::shadow_tool::{shadow_handles, ShadowHandle};
    if app.tool != Tool::DropShadow {
        return;
    }
    let Some((_, sh, b)) = app.selected_shadow() else {
        return;
    };
    let view = &app.view;
    let hs = shadow_handles(&sh, b);
    let pos = |h: ShadowHandle| {
        hs.iter()
            .find(|(k, _)| *k == h)
            .map(|(_, p)| view.to_screen(*p))
    };
    let (Some(a), Some(z), Some(o)) = (
        pos(ShadowHandle::Start),
        pos(ShadowHandle::End),
        pos(ShadowHandle::Opacity),
    ) else {
        return;
    };
    let d = z - a;
    let len = d.length();
    if len > 1e-3 {
        let step = 6.0;
        let mut t = 0.0;
        while t < len {
            let t1 = (t + step * 0.6).min(len);
            painter.line_segment(
                [a + d * (t / len), a + d * (t1 / len)],
                EStroke::new(1.0, Tokens::SELECTION),
            );
            t += step;
        }
        // The opacity slider: a short bar across the line.
        let n = egui::vec2(-d.y, d.x) / len * 5.0;
        painter.line_segment([o - n, o + n], EStroke::new(3.0, Tokens::SELECTION));
    }
    let [r, g, bl] = sh.color.to_rgb8();
    for (c, fill) in [(a, Color32::WHITE), (z, Color32::from_rgb(r, g, bl))] {
        let rect = ERect::from_center_size(c, egui::vec2(9.0, 9.0));
        painter.rect_filled(rect, 0.0, fill);
        painter.rect_stroke(
            rect,
            0.0,
            EStroke::new(1.0, Tokens::SELECTION),
            egui::StrokeKind::Outside,
        );
    }
}

/// Blend tool: the selected blend's start and end squares, the line
/// between them and the object (above) and colour (below) acceleration
/// sliders on it, plain segments and small shapes (#965).
fn draw_blend_controls(app: &App, painter: &Painter) {
    use crate::blend_tool::{screen_normal, BlendHandle, SLIDER_OFFSET_PX};
    // Not while a new blend is being dragged out.
    if app.tool != Tool::Blend || matches!(app.drag, Drag::Connector { .. }) {
        return;
    }
    let Some(c) = app.blend_controls() else {
        return;
    };
    let view = &app.view;
    let (a, z) = (view.to_screen(c.start), view.to_screen(c.end));
    haloed_polyline(painter, &[a, z], false, Tokens::SELECTION);
    let n = screen_normal(a, z);
    if (z - a).length() >= 14.0 {
        let along = (z - a).normalized();
        for (h, side) in [(BlendHandle::Objects, 1.0), (BlendHandle::Colors, -1.0)] {
            let o = view.to_screen(c.slider_point(h));
            // A triangle pointing at the line from its side.
            let tip = o + n * side * 1.0;
            let base = o + n * side * (SLIDER_OFFSET_PX + 5.0);
            let pts = vec![tip, base + along * 4.5, base - along * 4.5];
            let fill = if app.blend_handle_drag == Some(h) {
                Tokens::SELECTION
            } else {
                Color32::WHITE
            };
            painter.add(egui::Shape::convex_polygon(
                pts,
                fill,
                EStroke::new(1.0, Tokens::SELECTION),
            ));
        }
    }
    for (h, p) in [(BlendHandle::Start, a), (BlendHandle::End, z)] {
        let rect = ERect::from_center_size(p, egui::vec2(9.0, 9.0));
        // The square being dragged is filled in the selection colour.
        let fill = if app.blend_handle_drag == Some(h) {
            Tokens::SELECTION
        } else {
            Color32::WHITE
        };
        painter.rect_filled(rect, 0.0, fill);
        painter.rect_stroke(
            rect,
            0.0,
            EStroke::new(1.0, Tokens::SELECTION),
            egui::StrokeKind::Outside,
        );
    }
    // The end square is crossed, as the start one is plain.
    let r = 3.5;
    painter.line_segment(
        [z + egui::vec2(-r, -r), z + egui::vec2(r, r)],
        EStroke::new(1.0, Tokens::SELECTION),
    );
    painter.line_segment(
        [z + egui::vec2(-r, r), z + egui::vec2(r, -r)],
        EStroke::new(1.0, Tokens::SELECTION),
    );
}

fn draw_anchors(app: &App, painter: &Painter) {
    let view = &app.view;
    let mut shown: Vec<(tracedraw_core::ShapeId, bool)> = Vec::new();
    match (&app.tool, &app.drag) {
        (Tool::AnchorEditing, _) => {
            if let Some(id) = app.selection.first() {
                shown.push((*id, true));
            }
        }
        (
            Tool::Connector | Tool::RightAngleConnector | Tool::RoundedConnector,
            Drag::Connector { from, current, .. },
        ) => {
            shown.push((*from, false));
            if let Some(to) = app.hit_test(*current).filter(|t| t != from) {
                shown.push((to, false));
            }
        }
        _ => {}
    }
    for (id, editing) in shown {
        let n_side = crate::anchors::SIDE_ANCHORS.len();
        for (i, p) in app.anchors_of(id).into_iter().enumerate() {
            let c = view.to_screen(p);
            let custom = i >= n_side;
            let r = if custom { 5.0 } else { 4.0 };
            let pts = vec![
                c + egui::vec2(0.0, -r),
                c + egui::vec2(r, 0.0),
                c + egui::vec2(0.0, r),
                c + egui::vec2(-r, 0.0),
            ];
            let selected = editing && custom && app.anchor_sel == Some(i - n_side);
            let fill = if selected {
                Tokens::SELECTION
            } else if custom {
                Color32::WHITE
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 160)
            };
            painter.add(epaint::PathShape::convex_polygon(
                pts,
                fill,
                EStroke::new(1.0, Tokens::SELECTION),
            ));
        }
    }
}

/// Linked paragraph frames: a blue connector from the bottom of a selected
/// frame to the top of the next one.
fn draw_frame_links(app: &App, painter: &Painter) {
    let view = &app.view;
    let stroke = EStroke::new(1.0, Tokens::SELECTION);
    // Only linked paragraph text has a chain (asking for every selected
    // object searched the drawing for each, #367).
    let linked: Vec<tracedraw_core::ShapeId> = app
        .selected_rc()
        .iter()
        .filter(|s| matches!(s.kind, ShapeKind::Text { frame: Some(_), .. }) && !s.data.is_empty())
        .map(|s| s.id)
        .collect();
    for id in &linked {
        for pair in app.chain_of(*id).windows(2) {
            let (Some(a), Some(b)) = (app.doc().find_shape(pair[0]), app.doc().find_shape(pair[1]))
            else {
                continue;
            };
            let (ba, bb) = (a.bounds(), b.bounds());
            let from = view.to_screen(Point::new(ba.x1, ba.y0));
            let to = view.to_screen(Point::new(bb.x0, bb.y1));
            painter.line_segment([from, to], stroke);
            painter.circle_filled(from, 3.0, Tokens::SELECTION);
            let d = (to - from).normalized();
            let n = egui::vec2(-d.y, d.x);
            painter.add(epaint::PathShape::convex_polygon(
                vec![to, to - d * 8.0 + n * 4.0, to - d * 8.0 - n * 4.0],
                Tokens::SELECTION,
                EStroke::NONE,
            ));
        }
    }
}

/// Active table cells (Table tool): a filled highlight over each selected cell.
fn draw_table_cells(app: &App, painter: &Painter) {
    let Some(e) = &app.table_edit else {
        return;
    };
    let Ok((_, s)) = app.doc().shape(e.shape) else {
        return;
    };
    let tracedraw_core::document::ShapeKind::Table(t) = &s.kind else {
        return;
    };
    let view = &app.view;
    for cell in &t.cells {
        if !e.cells.contains(&(cell.row, cell.col)) {
            continue;
        }
        let r = s.transform.transform_rect_bbox(t.cell_rect(cell));
        let sr = view.rect_to_screen(r);
        painter.rect_filled(sr, 0.0, Tokens::SELECTION.gamma_multiply(0.25));
        painter.rect_stroke(
            sr,
            0.0,
            EStroke::new(1.5, Tokens::SELECTION),
            epaint::StrokeKind::Inside,
        );
    }
    // Caret and selection inside the active cell.
    if let Some((top, bottom, quads)) = app.cell_caret_geometry() {
        for q in quads {
            let pts: Vec<Pos2> = q.iter().map(|p| view.to_screen(*p)).collect();
            painter.add(egui::Shape::convex_polygon(
                pts,
                TEXT_SELECTION,
                EStroke::NONE,
            ));
        }
        draw_text_caret(painter, view.to_screen(top), view.to_screen(bottom));
        ime_at_caret(app, painter, view.to_screen(top), view.to_screen(bottom));
    }
}

/// The selection's outline moved by `t`: a blue line on a white halo, so it
/// reads on any background (two light lines on a dark one).
/// The outlines of the mirrored copies `s` (in page space) and the
/// objects inside it draw with their symmetries, but the one `edited`.
fn symmetry_copy_paths(
    s: &tracedraw_core::Shape,
    edited: Option<tracedraw_core::ShapeId>,
    out: &mut Vec<BezPath>,
) {
    use tracedraw_core::live::Effect;
    if edited != Some(s.id)
        && s.effects
            .iter()
            .any(|e| matches!(e, Effect::Symmetry { .. }))
    {
        let mut only = s.clone();
        only.effects
            .retain(|e| matches!(e, Effect::Symmetry { .. }));
        out.extend(
            tracedraw_core::live::evaluate(&only)
                .above
                .iter()
                .map(|c| c.page_path()),
        );
    }
    if let ShapeKind::Group { children } = &s.kind {
        for c in children {
            let mut c = c.clone();
            c.transform_by(s.transform);
            symmetry_copy_paths(&c, edited, out);
        }
    }
}

pub fn draw_transform_outline(app: &App, painter: &Painter, t: Affine) {
    let halo = EStroke::new(3.0, Color32::from_white_alpha(220));
    let line = EStroke::new(1.0, Tokens::DRAG_OUTLINE);
    for s in app.selected_rc().iter() {
        // What the release will make: rounded corners keep their size.
        let mut paths = vec![s.page_path_after(t)];
        // A symmetry out of edit mode goes whole: its copies too, also
        // those of one inside a group.
        let mut moved = s.clone();
        moved.transform_by(t);
        symmetry_copy_paths(&moved, app.symmetry_edit, &mut paths);
        for path in &paths {
            for (mut pts, closed) in flatten(path, &app.view) {
                if closed {
                    if let Some(first) = pts.first().copied() {
                        pts.push(first);
                    }
                }
                painter.add(egui::Shape::line(pts.clone(), halo));
                painter.add(egui::Shape::line(pts, line));
            }
        }
    }
}

/// A fitted text dragged along its path: its letters where they would go,
/// outlined in blue, the line they would stand on dashed, and the
/// distance from the path and offset along it by the pointer.
fn draw_along_path(app: &App, painter: &Painter, start: Point, current: Point) {
    let Some((letters, line)) = app.along_path_preview(start, current) else {
        return;
    };
    let halo = EStroke::new(3.0, Color32::from_white_alpha(220));
    let stroke = EStroke::new(1.0, Tokens::DRAG_OUTLINE);
    for (mut pts, closed) in flatten(&letters, &app.view) {
        if closed {
            if let Some(first) = pts.first().copied() {
                pts.push(first);
            }
        }
        painter.add(egui::Shape::line(pts.clone(), halo));
        painter.add(egui::Shape::line(pts, stroke));
    }
    for (pts, _) in flatten(&line, &app.view) {
        dashed_polyline(painter, &pts, stroke, 4.0, 3.0);
    }
    if let Some(ft) = app.along_path_settings(start, current) {
        let u = app.units;
        let text = format!(
            "{}: {:.2} {u}   {}: {:.2} {u}",
            crate::i18n::tr("toolbar.path_distance"),
            u.from_mm(ft.on_path.distance),
            crate::i18n::tr("toolbar.path_offset"),
            u.from_mm(ft.on_path.offset),
            u = u.short()
        );
        painter.text(
            app.view.to_screen(current) + egui::vec2(14.0, 14.0),
            egui::Align2::LEFT_TOP,
            text,
            egui::FontId::proportional(11.0),
            Tokens::DRAG_OUTLINE,
        );
    }
}

/// Fit Text to Path waiting for its path, the pointer near one: the
/// selected text where a click puts it, outlined in blue, the line it
/// stands on dashed, a dashed line from the path to the pointer with the
/// distance, and a red mark where the text begins.
fn draw_text_path_preview(app: &App, painter: &Painter, p: Point) {
    let Some(pv) = app.text_path_preview(p) else {
        return;
    };
    let view = &app.view;
    let halo = EStroke::new(3.0, Color32::from_white_alpha(220));
    let stroke = EStroke::new(1.0, Tokens::DRAG_OUTLINE);
    for (mut pts, closed) in flatten(&pv.letters, view) {
        if closed {
            if let Some(first) = pts.first().copied() {
                pts.push(first);
            }
        }
        painter.add(egui::Shape::line(pts.clone(), halo));
        painter.add(egui::Shape::line(pts, stroke));
    }
    for (pts, _) in flatten(&pv.line, view) {
        dashed_polyline(painter, &pts, stroke, 4.0, 3.0);
    }
    let (near, at) = (view.to_screen(pv.nearest), view.to_screen(p));
    dashed_polyline(painter, &[near, at], stroke, 4.0, 3.0);
    let u = app.units;
    painter.text(
        at + egui::vec2(6.0, 4.0),
        egui::Align2::LEFT_TOP,
        format!("{:.2} {}", u.from_mm(pv.distance), u.short()),
        egui::FontId::proportional(11.0),
        Tokens::DRAG_OUTLINE,
    );
    let (sp, dir) = pv.start;
    let c = view.to_screen(sp);
    // Across the line, as long either side.
    let n = egui::vec2(-dir.y as f32, dir.x as f32);
    painter.line_segment(
        [c - n * 9.0, c + n * 9.0],
        EStroke::new(2.0, Color32::from_rgb(0xE0, 0x20, 0x20)),
    );
}

fn draw_selection(app: &App, painter: &Painter, preview: Option<Affine>) {
    let shapes = app.selected_rc();
    if shapes.is_empty() {
        return;
    }
    let view = &app.view;
    // What the objects draw (live effects included), moved by the
    // transform being previewed.
    let bounds = app
        .selected_bounds()
        .iter()
        .map(|&b| match preview {
            Some(t) => t.transform_rect_bbox(b),
            None => b,
        })
        .reduce(|a, b| a.union(b))
        .unwrap_or(Rect::ZERO);
    let r = view.rect_to_screen(bounds);

    if app.tool == Tool::Shape {
        draw_nodes(app, painter);
        return;
    }
    // While text is typed or edited only the text and its cursor show;
    // the handles come back when editing ends.
    if app.text_edit.is_some() {
        return;
    }
    // The Envelope tool shows the frames instead (`ui::envelope_ui`), the
    // Mesh Fill tool the grids.
    if matches!(app.tool, Tool::Envelope | Tool::MeshFill) {
        return;
    }
    // Options > Display > Hide bounding box for curve tools.
    if app.settings.hide_bbox_curve_tools
        && matches!(
            app.tool,
            Tool::Freehand
                | Tool::TwoPointLine
                | Tool::Bezier
                | Tool::Pen
                | Tool::BSpline
                | Tool::Polyline
                | Tool::ThreePointCurve
                | Tool::ShapeRecognition
                | Tool::BrushStrokes
                | Tool::PainterlyBrush
        )
    {
        return;
    }

    // Every tool shows the selection handles and the centre marker right
    // after a shape is drawn; only the Pick tools switch to the rotate and
    // skew arrows.
    let pick = matches!(
        app.tool,
        Tool::Pick | Tool::FreeformPick | Tool::InteractiveFill | Tool::Text
    );
    if app.rotate_mode && pick {
        // Rotation arrows at corners, skew arrows at edges, the centre of
        // rotation (it can be moved).
        let c = app
            .rotation_center()
            .map(|c| app.view.to_screen(c))
            .unwrap_or(r.center());
        // The centre of rotation: a ring with a dot.
        painter.circle_stroke(c, 8.0, EStroke::new(2.0, Color32::BLACK));
        painter.circle_filled(c, 2.0, Color32::BLACK);
        for (h, p) in app.handle_positions(bounds) {
            draw_rotate_handle(painter, p, h);
        }
    } else {
        // Eight square handles just outside the selection box, which has
        // no frame of its own, so the objects' outlines stay in view.
        // Objects chosen inside a group show round handles.
        let nested = app.selection_nested();
        let size = crate::interaction::HANDLE_SIZE;
        // Black with a white edge, so they show on a dark drawing (#670).
        let edge = EStroke::new(1.0, Color32::WHITE);
        for (_, p) in app.handle_positions(bounds) {
            if nested {
                painter.circle(p, size / 2.0 + 0.5, Color32::BLACK, edge);
            } else {
                painter.rect(
                    ERect::from_center_size(p, egui::vec2(size, size)),
                    0.0,
                    Color32::BLACK,
                    edge,
                    epaint::StrokeKind::Outside,
                );
            }
        }
    }
    // Hint "x" in the centre: the move marker (rotate mode shows the
    // centre of rotation instead).
    if app.rotate_mode && pick {
        return;
    }
    let c = r.center();
    painter.line_segment(
        [c - egui::vec2(4.0, 4.0), c + egui::vec2(4.0, 4.0)],
        EStroke::new(1.0, Tokens::HANDLE),
    );
    painter.line_segment(
        [c - egui::vec2(4.0, -4.0), c + egui::vec2(4.0, -4.0)],
        EStroke::new(1.0, Tokens::HANDLE),
    );
}

/// A rotate mode handle at `p`: a curved double arrow around each
/// corner, a straight double arrow along each side (sideways at the top
/// and bottom, up and down at the left and right). Black, 2 points thick,
/// with solid heads.
fn draw_rotate_handle(painter: &Painter, p: Pos2, h: Handle) {
    let ink = Color32::BLACK;
    let stroke = EStroke::new(2.0, ink);
    // A solid head with its tip at `tip`, pointing along `dir`.
    let head = |tip: Pos2, dir: egui::Vec2| {
        let d = dir.normalized();
        let n = egui::vec2(-d.y, d.x);
        epaint::PathShape::convex_polygon(
            vec![tip, tip - d * 4.5 + n * 3.0, tip - d * 4.5 - n * 3.0],
            ink,
            EStroke::NONE,
        )
    };
    if h.is_corner() {
        // Which way the corner points (screen y grows downwards).
        let (sx, sy): (f32, f32) = match h {
            Handle::NE => (1.0, -1.0),
            Handle::NW => (-1.0, -1.0),
            Handle::SE => (1.0, 1.0),
            _ => (-1.0, 1.0),
        };
        // A quarter circle bulging out of the corner, its centre half a
        // radius in from `p` on each axis.
        let radius = 13.0;
        let centre = p - egui::vec2(sx, sy) * (radius / 2.0);
        let base = sy.atan2(sx);
        let (a0, a1) = (
            base - std::f32::consts::FRAC_PI_4,
            base + std::f32::consts::FRAC_PI_4,
        );
        let at = |a: f32| centre + egui::vec2(a.cos(), a.sin()) * radius;
        // The arc stops short of its ends, where the heads take over.
        let trim = 3.5 / radius;
        let arc: Vec<Pos2> = (0..=12)
            .map(|i| at(a0 + trim + (a1 - a0 - 2.0 * trim) * i as f32 / 12.0))
            .collect();
        painter.add(epaint::PathShape::line(arc, stroke));
        // Each head points on along the turn.
        let tangent = |a: f32| egui::vec2(-a.sin(), a.cos());
        painter.add(head(at(a1), tangent(a1)));
        painter.add(head(at(a0), -tangent(a0)));
    } else {
        let d = if matches!(h, Handle::N | Handle::S) {
            egui::vec2(8.0, 0.0)
        } else {
            egui::vec2(0.0, 8.0)
        };
        let u = d.normalized() * 4.0;
        painter.line_segment([p - d + u, p + d - u], stroke);
        painter.add(head(p + d, d));
        painter.add(head(p - d, -d));
    }
}

/// The shade over selected characters of a text being edited: a
/// see-through grey, so coloured letters keep their colour under it, dark
/// enough to stand out on the white page (#632).
pub(crate) const TEXT_SELECTION: Color32 = Color32::from_rgba_premultiplied(40, 40, 40, 150);

/// The caret's strokes, drawn in order: a white edge, then the black bar.
pub(crate) const TEXT_CARET: [EStroke; 2] = [
    EStroke {
        width: 3.5,
        color: Color32::WHITE,
    },
    EStroke {
        width: 1.5,
        color: Color32::BLACK,
    },
];

/// A text that does not fit: its frame is red.
const OVERFLOW_RED: Color32 = Color32::from_rgb(220, 30, 30);

/// The colour of a paragraph frame's dashed line: red when its text does
/// not fit, black while it is being edited, blue while selected, else
/// grey (Text > Show Text Frames).
pub(crate) fn text_frame_color(f: &crate::text_frames::ShownFrame) -> Color32 {
    if f.overflow {
        OVERFLOW_RED
    } else if f.editing {
        Color32::BLACK
    } else if f.chosen {
        Tokens::SELECTION
    } else {
        Color32::from_gray(150)
    }
}

/// The flow tab of a frame whose bottom edge runs from `a` to `b` on
/// screen: hanging under its middle, below the bottom handle of the
/// selection (it sat under that handle, so dragging the frame taller
/// started a text link instead, #503).
pub fn flow_tab_rect(a: Pos2, b: Pos2) -> ERect {
    let mid = a + (b - a) * 0.5;
    let top = crate::interaction::HANDLE_OFFSET + crate::interaction::HANDLE_SIZE / 2.0 + 2.0;
    ERect::from_min_size(mid + egui::vec2(-6.0, top), egui::vec2(12.0, 9.0))
}

/// Paragraph text frames: dashed lines in the colour `text_frame_color`
/// gives.
fn draw_text_frames(app: &App, painter: &Painter, preview: Option<Affine>) {
    // A text that does not fit: red, with a tab holding a down arrow
    // below the frame's middle.
    let red = OVERFLOW_RED;
    for f in app.shown_text_frames(preview) {
        let mut pts: Vec<Pos2> = f.corners.iter().map(|p| app.view.to_screen(*p)).collect();
        pts.push(pts[0]);
        let color = text_frame_color(&f);
        painter.extend(egui::Shape::dashed_line(
            &pts,
            EStroke::new(1.0, color),
            4.0,
            3.0,
        ));
        // The flow tab under the bottom edge's middle: a down arrow when
        // the text does not fit, two links when it flows on, empty on a
        // picked frame; a click on it says where the text flows on.
        if f.overflow || f.chosen {
            let tab = flow_tab_rect(pts[0], pts[1]);
            let ink = if f.overflow { red } else { color };
            painter.rect(
                tab,
                0.0,
                Color32::WHITE,
                EStroke::new(1.0, ink),
                epaint::StrokeKind::Inside,
            );
            let c = tab.center();
            if f.overflow {
                painter.add(epaint::PathShape::convex_polygon(
                    vec![
                        c + egui::vec2(-3.0, -2.0),
                        c + egui::vec2(3.0, -2.0),
                        c + egui::vec2(0.0, 2.5),
                    ],
                    red,
                    EStroke::NONE,
                ));
            } else if f.linked {
                for dx in [-2.0, 2.0] {
                    painter.rect_stroke(
                        ERect::from_center_size(c + egui::vec2(dx, 0.0), egui::vec2(4.0, 3.0)),
                        1.0,
                        EStroke::new(1.0, ink),
                        epaint::StrokeKind::Inside,
                    );
                }
            }
        }
    }
}

/// Empty clip frames show two grey diagonals on screen (Options >
/// Clip Frames > Show lines in empty clip frames).
fn draw_empty_clip_frames(app: &App, painter: &Painter) {
    if !app.settings.clip_frame.empty_lines {
        return;
    }
    let Ok(page) = app.doc().page(app.page) else {
        return;
    };
    let stroke = EStroke::new(1.0, Color32::from_gray(160));
    for s in page
        .layers
        .iter()
        .filter(|l| l.visible)
        .flat_map(|l| &l.shapes)
    {
        if let ShapeKind::ClipFrame { contents, .. } = &s.kind {
            if contents.is_empty() && s.visible {
                let r = app.view.rect_to_screen(s.bounds());
                painter.line_segment([r.left_top(), r.right_bottom()], stroke);
                painter.line_segment([r.right_top(), r.left_bottom()], stroke);
            }
        }
    }
}

/// Document grid line colour (lines) and dot colour (dots).
const GRID_LINE: Color32 = Color32::from_gray(0xDC);
const GRID_DOT: Color32 = Color32::from_gray(0x8C);

/// How many grid spacings apart drawn lines are, so they stay at least
/// `min_px` screen pixels apart: 1, 2, 5, 10, 20, 50... spacings.
pub fn grid_thinning(spacing_px: f64, min_px: f64) -> f64 {
    if !(spacing_px.is_finite() && spacing_px > 0.0) {
        return f64::INFINITY;
    }
    for k in 0..12 {
        for m in [1.0, 2.0, 5.0] {
            let f = m * 10f64.powi(k);
            if spacing_px * f >= min_px {
                return f;
            }
        }
    }
    f64::INFINITY
}

/// Grid positions (mm) along one axis between `a` and `b`: multiples of
/// `step` counted from `origin`.
pub fn grid_positions(a: f64, b: f64, origin: f64, step: f64) -> Vec<f64> {
    let (a, b) = (a.min(b), a.max(b));
    if !(step.is_finite() && step > 0.0) {
        return Vec::new();
    }
    let first = ((a - origin) / step).floor() as i64;
    let last = ((b - origin) / step).ceil() as i64;
    if last - first > 5000 {
        return Vec::new();
    }
    (first..=last).map(|i| origin + i as f64 * step).collect()
}

/// The document grid over the whole drawing window, as lines or dots,
/// passing through the ruler origin. Lines are thinned out when the zoom
/// would crowd them.
fn draw_grid(app: &App, painter: &Painter, rect: ERect) {
    let view = &app.view;
    let meta = &app.doc().metadata;
    let (grid, rulers) = (meta.grid, meta.rulers.clone());
    let zoom = view.zoom as f64;
    let dots = grid.display == tracedraw_core::document::GridDisplay::Dots;
    let min_px = if dots { 8.0 } else { 6.0 };
    let sx = grid.spacing_x.max(1e-6) * grid_thinning(grid.spacing_x * zoom, min_px);
    let sy = grid.spacing_y.max(1e-6) * grid_thinning(grid.spacing_y * zoom, min_px);
    let tl = view.to_page(rect.left_top());
    let br = view.to_page(rect.right_bottom());
    let xs: Vec<f32> = grid_positions(tl.x, br.x, rulers.origin_x, sx)
        .into_iter()
        .map(|x| view.to_screen(Point::new(x, 0.0)).x.round() + 0.5)
        .filter(|x| *x >= rect.left() && *x <= rect.right())
        .collect();
    let ys: Vec<f32> = grid_positions(br.y, tl.y, rulers.origin_y, sy)
        .into_iter()
        .map(|y| view.to_screen(Point::new(0.0, y)).y.round() + 0.5)
        .filter(|y| *y >= rect.top() && *y <= rect.bottom())
        .collect();
    let mut px = crate::ui::rulers::Pixels::default();
    if dots {
        if xs.len() * ys.len() > 200_000 {
            return;
        }
        for x in &xs {
            for y in &ys {
                px.dot(x.floor(), y.floor(), GRID_DOT);
            }
        }
    } else {
        for x in &xs {
            px.rect(vline_rect(*x, rect), GRID_LINE);
        }
        for y in &ys {
            px.rect(hline_rect(*y, rect), GRID_LINE);
        }
    }
    px.paint(painter);
}

/// A one-pixel vertical line through the pixel containing `x`, across `r`.
fn vline_rect(x: f32, r: ERect) -> ERect {
    let x = x.floor();
    ERect::from_min_max(Pos2::new(x, r.top()), Pos2::new(x + 1.0, r.bottom()))
}

/// A one-pixel horizontal line through the pixel containing `y`.
fn hline_rect(y: f32, r: ERect) -> ERect {
    let y = y.floor();
    ERect::from_min_max(Pos2::new(r.left(), y), Pos2::new(r.right(), y + 1.0))
}

/// The baseline grid: lines across the page like a ruled notebook, from
/// the "start from top" distance down, in the grid's colour.
fn draw_baseline_grid(app: &App, painter: &Painter, rect: ERect, paper: ERect) {
    let grid = app.doc().metadata.grid;
    if grid.baseline_spacing * (app.view.zoom as f64) < 2.0 {
        return;
    }
    let color = to_color32(grid.baseline_color);
    let mut px = crate::ui::rulers::Pixels::default();
    for y in app.baseline_ys() {
        let sy = app.view.to_screen(Point::new(0.0, y)).y.round() + 0.5;
        if sy >= rect.top() && sy <= rect.bottom() {
            px.rect(hline_rect(sy, paper.intersect(rect)), color);
        }
    }
    px.paint(painter);
}

/// Screen pixels per document pixel at the current zoom.
pub fn screen_px_per_doc_px(app: &App) -> f64 {
    app.view.zoom as f64 * 25.4 / app.document_dpi().max(1.0)
}

/// The pixel grid: one cell per document pixel, aligned with the page's
/// bottom-left corner, in the Pixels view (or with pixel rulers) once a
/// document pixel spans 8 screen pixels.
fn draw_pixel_grid(app: &App, painter: &Painter, rect: ERect) {
    let pixel_mode =
        app.view_mode == crate::app::ViewMode::Pixels || app.units == crate::app::Units::Pixels;
    if !app.show_pixel_grid || !pixel_mode || screen_px_per_doc_px(app) < 7.999 {
        return;
    }
    let grid = app.doc().metadata.grid;
    let [r, g, b] = grid.pixel_color.to_rgb8();
    let alpha = (grid.pixel_opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    let color = Color32::from_rgba_unmultiplied(r, g, b, alpha);
    if alpha == 0 {
        return;
    }
    let step = 25.4 / app.document_dpi().max(1.0);
    let view = &app.view;
    let tl = view.to_page(rect.left_top());
    let br = view.to_page(rect.right_bottom());
    let mut px = crate::ui::rulers::Pixels::default();
    for x in grid_positions(tl.x, br.x, 0.0, step) {
        let sx = view.to_screen(Point::new(x, 0.0)).x.round() + 0.5;
        if sx >= rect.left() && sx <= rect.right() {
            px.rect(vline_rect(sx, rect), color);
        }
    }
    for y in grid_positions(br.y, tl.y, 0.0, step) {
        let sy = view.to_screen(Point::new(0.0, y)).y.round() + 0.5;
        if sy >= rect.top() && sy <= rect.bottom() {
            px.rect(hline_rect(sy, rect), color);
        }
    }
    px.paint(painter);
}

/// One node marker: `shape` of side `size` around `p`.
/// A node's control handle: a dashed line from the node `node` to the
/// control point `ctrl`, which is a solid arrow head pointing away from
/// the node.
fn draw_control_handle(painter: &Painter, node: Pos2, ctrl: Pos2, color: Color32) {
    let d = ctrl - node;
    let len = d.length();
    if len < 0.5 {
        return;
    }
    let u = d / len;
    let n = egui::vec2(-u.y, u.x);
    let head = 7.0_f32.min(len);
    let base = ctrl - u * head;
    if len > head {
        // Dashes in the colour and white in turn, as wide as the Pen
        // preview's inner line, and a head with a white edge, so the
        // handle shows on any colour (#670).
        painter.line_segment([node, base], EStroke::new(HALO_INNER, Color32::WHITE));
        dashed_polyline(
            painter,
            &[node, base],
            EStroke::new(HALO_INNER, color),
            4.0,
            3.0,
        );
    }
    painter.add(epaint::PathShape::convex_polygon(
        vec![ctrl, base + n * 3.5, base - n * 3.5],
        color,
        EStroke::new(1.0, Color32::WHITE),
    ));
}

/// The first node of a subpath: a triangle standing on its base, `size`
/// across.
fn start_node_marker(
    painter: &Painter,
    p: Pos2,
    size: f32,
    fill: Option<Color32>,
    outline: Color32,
) {
    let h = size / 2.0;
    let pts = vec![
        p + egui::vec2(0.0, -h),
        p + egui::vec2(h, h * 0.8),
        p + egui::vec2(-h, h * 0.8),
    ];
    painter.add(epaint::PathShape::convex_polygon(
        pts,
        fill.unwrap_or(Color32::TRANSPARENT),
        EStroke::new(1.0, outline),
    ));
}

fn node_marker(
    painter: &Painter,
    p: Pos2,
    size: f32,
    shape: crate::settings::NodeShape,
    fill: Option<Color32>,
    outline: Color32,
) {
    use crate::settings::NodeShape;
    let h = size / 2.0;
    let stroke = EStroke::new(1.0, outline);
    match shape {
        NodeShape::Square => {
            let r = ERect::from_center_size(p, egui::vec2(size, size));
            if let Some(f) = fill {
                painter.rect_filled(r, 0.0, f);
            }
            painter.rect_stroke(r, 0.0, stroke, epaint::StrokeKind::Inside);
        }
        NodeShape::Circle => {
            if let Some(f) = fill {
                painter.circle_filled(p, h, f);
            }
            painter.circle_stroke(p, h - 0.5, stroke);
        }
        NodeShape::Diamond => {
            let pts = vec![
                p + egui::vec2(0.0, -h - 0.5),
                p + egui::vec2(h + 0.5, 0.0),
                p + egui::vec2(0.0, h + 0.5),
                p + egui::vec2(-h - 0.5, 0.0),
            ];
            painter.add(epaint::PathShape::convex_polygon(
                pts,
                fill.unwrap_or(Color32::TRANSPARENT),
                stroke,
            ));
        }
    }
}

/// A text's Interactive spacing arrow: a bar and an arrow pointing right
/// (character spacing) or down (line spacing), centred on `c`.
fn draw_spacing_arrow(painter: &Painter, c: Pos2, horizontal: bool, color: Color32) {
    let s = EStroke::new(1.5, color);
    let v = |x: f32, y: f32| {
        if horizontal {
            c + egui::vec2(x, y)
        } else {
            c + egui::vec2(y, x)
        }
    };
    painter.circle_filled(c, 8.0, Color32::from_white_alpha(200));
    painter.line_segment([v(-6.0, -4.0), v(-6.0, 4.0)], s);
    painter.line_segment([v(-6.0, 0.0), v(5.0, 0.0)], s);
    painter.add(epaint::PathShape::convex_polygon(
        vec![v(7.0, 0.0), v(3.0, -3.5), v(3.0, 3.5)],
        color,
        EStroke::NONE,
    ));
}

/// Shape tool overlay, as Options > Nodes and Handles sets it: nodes
/// shaped by type (cusp, smooth, symmetrical), the selected ones filled
/// with the main colour, the others white or hollow, the first node of a
/// curve larger, handles of the selected nodes, and an arrow in the
/// secondary colour showing the curve's direction.
fn draw_nodes(app: &App, painter: &Painter) {
    use tracedraw_core::nodes;
    let view = &app.view;
    let prefs = app.settings.nodes;
    let [r, g, b] = prefs.main_rgb;
    let main = Color32::from_rgb(r, g, b);
    // The segment chosen by a click: drawn over in the main colour, with
    // its handles.
    let chosen_segment = app.chosen_segment();
    if let Some((id, index)) = chosen_segment {
        if let Some(s) = app.doc().shape_on_page(id) {
            if let ShapeKind::Path { path, .. } = &s.kind {
                let pts: Vec<Pos2> = (0..=32)
                    .filter_map(|k| nodes::point_on_segment(path, index, k as f64 / 32.0))
                    .map(|q| view.to_screen(s.transform * q))
                    .collect();
                painter.add(egui::Shape::line(pts, EStroke::new(3.0, main)));
                if let Some(PathEl::CurveTo(c1, c2, _)) =
                    nodes::normalize(path).elements().get(index).copied()
                {
                    let ends = (
                        nodes::point_on_segment(path, index, 0.0),
                        nodes::point_on_segment(path, index, 1.0),
                    );
                    if let (Some(a), Some(b)) = ends {
                        let sp = |q| view.to_screen(s.transform * q);
                        draw_control_handle(painter, sp(a), sp(c1), main);
                        draw_control_handle(painter, sp(b), sp(c2), main);
                    }
                }
            }
        }
    }
    // The spot clicked on a segment: a small round mark where Add Node
    // puts the node.
    if let Some((id, index, t)) = app.segment_spot {
        let spot = app.doc().shape_on_page(id).and_then(|s| match &s.kind {
            ShapeKind::Path { path, .. } => {
                nodes::point_on_segment(path, index, t).map(|q| s.transform * q)
            }
            _ => None,
        });
        if let Some(q) = spot {
            let c = view.to_screen(q);
            painter.circle_filled(c, 3.5, Color32::WHITE);
            painter.circle_filled(c, 2.5, main);
        }
    }
    let [r, g, b] = prefs.secondary_rgb;
    let secondary = Color32::from_rgb(r, g, b);
    let size = prefs.size.px();
    // Several objects: the one picked last has its outline in the node
    // colour, the others in the second colour (#1007).
    let key = app.shape_tool_key_curve();
    let outline_of = |id: tracedraw_core::ShapeId| match key {
        Some(k) if k != id => secondary,
        _ => main,
    };
    // A clip frame shows its frame's nodes.
    for s in app.shape_tool_shapes() {
        // A picture shows a node at each corner; dragging one crops it.
        if let Some(corners) = crate::shape_tool::bitmap_corners(&s) {
            for p in corners {
                let fill = prefs.unselected_filled.then_some(Color32::WHITE);
                node_marker(painter, view.to_screen(p), size, prefs.cusp, fill, main);
            }
            continue;
        }
        // Text: a node per character and the two spacing arrows.
        if matches!(s.kind, ShapeKind::Text { .. }) {
            let chars = crate::text_nodes::char_nodes(&s);
            if !chars.is_empty() {
                for (c, p) in chars {
                    let chosen = app.node_selection.contains(&(s.id, c));
                    let r = ERect::from_center_size(view.to_screen(p), egui::vec2(6.0, 6.0));
                    painter.rect_filled(r, 0.0, if chosen { main } else { Color32::WHITE });
                    painter.rect_stroke(
                        r,
                        0.0,
                        EStroke::new(1.0, main),
                        epaint::StrokeKind::Inside,
                    );
                }
                let (h, v) = app.spacing_arrow_spots(&s);
                draw_spacing_arrow(painter, h, true, main);
                draw_spacing_arrow(painter, v, false, main);
                continue;
            }
        }
        // A B-spline: its outline, the control polygon dashed and the
        // control points as nodes; the curve's own nodes do not show.
        if s.spline.is_some() {
            for (pts, _) in flatten(&s.page_path(), view) {
                painter.add(egui::Shape::line(pts, EStroke::new(1.0, outline_of(s.id))));
            }
            let own = crate::kind_nodes::nodes(&s);
            let poly: Vec<Pos2> = own.iter().map(|(_, p)| view.to_screen(*p)).collect();
            let closed = matches!(s.kind, ShapeKind::Path { closed: true, .. });
            let lines = if closed {
                poly.len()
            } else {
                poly.len().saturating_sub(1)
            };
            for i in 0..lines {
                painter.extend(egui::Shape::dashed_line(
                    &[poly[i], poly[(i + 1) % poly.len()]],
                    EStroke::new(1.0, secondary),
                    4.0,
                    3.0,
                ));
            }
            for (index, p) in poly.into_iter().enumerate() {
                // Chosen control points are filled, as chosen nodes are.
                let fill = if app.node_selection.contains(&(s.id, index)) {
                    Some(main)
                } else {
                    prefs.unselected_filled.then_some(Color32::WHITE)
                };
                node_marker(painter, p, size, prefs.cusp, fill, main);
            }
            continue;
        }
        let ShapeKind::Path { path, .. } = &s.kind else {
            // Rectangles, ellipses and polygons show their own nodes.
            let own = crate::kind_nodes::nodes(&s);
            if own.is_empty() {
                painter.rect_stroke(
                    view.rect_to_screen(s.bounds()),
                    0.0,
                    EStroke::new(1.0, Tokens::SELECTION),
                    epaint::StrokeKind::Outside,
                );
            }
            for (node, p) in own {
                let chosen = matches!(
                    node,
                    crate::kind_nodes::KindNode::Corner { corner, .. }
                        if app.rect_corners_selected.contains(&(s.id, corner))
                );
                let fill = if chosen {
                    Some(main)
                } else if prefs.unselected_filled {
                    Some(Color32::WHITE)
                } else {
                    None
                };
                node_marker(painter, view.to_screen(p), size, prefs.cusp, fill, main);
            }
            continue;
        };
        // The curve being edited shows its outline in the node colour
        // (the second colour for the objects picked before the last one).
        let outline = outline_of(s.id);
        for (mut pts, closed) in flatten(&s.page_path(), view) {
            if closed {
                if let Some(first) = pts.first().copied() {
                    pts.push(first);
                }
            }
            painter.add(egui::Shape::line(pts, EStroke::new(1.0, outline)));
        }
        let all = nodes::nodes(path);
        if prefs.show_direction {
            // A small arrow head just after the first node, along the
            // first segment.
            if let Some(n0) = all.iter().find(|n| n.is_start) {
                let toward = n0
                    .ctrl_out
                    .or_else(|| all.iter().find(|n| !n.is_start).map(|n| n.pos));
                if let Some(t) = toward {
                    let a = view.to_screen(s.transform * n0.pos);
                    let bpt = view.to_screen(s.transform * t);
                    let d = bpt - a;
                    if d.length() > 1.0 {
                        let d = d.normalized();
                        let n = egui::vec2(-d.y, d.x);
                        let tip = a + d * (size + 9.0);
                        let base = a + d * (size + 3.0);
                        painter.add(epaint::PathShape::convex_polygon(
                            vec![tip, base + n * 3.5, base - n * 3.5],
                            secondary,
                            EStroke::NONE,
                        ));
                    }
                }
            }
        }
        for n in all {
            // A chosen segment shows as the segment, not as its end node.
            let selected = app.node_selection.contains(&(s.id, n.index))
                && chosen_segment != Some((s.id, n.index));
            let p = view.to_screen(s.transform * n.pos);
            if selected {
                for c in [n.ctrl_in, n.ctrl_out].into_iter().flatten() {
                    let cp = view.to_screen(s.transform * c);
                    draw_control_handle(painter, p, cp, main);
                }
            }
            let shape = match nodes::node_type_of_with(&n, &s.node_types) {
                nodes::NodeType::Cusp => prefs.cusp,
                nodes::NodeType::Smooth => prefs.smooth,
                nodes::NodeType::Symmetrical => prefs.symmetrical,
            };
            let fill = if selected {
                Some(main)
            } else if prefs.unselected_filled {
                Some(Color32::WHITE)
            } else {
                None
            };
            if n.is_start {
                // The first node of a subpath: a triangle a little larger
                // than the other nodes.
                start_node_marker(painter, p, size + 3.0, fill, main);
            } else {
                node_marker(painter, p, size, shape, fill, main);
            }
        }
    }
    // Node transform handles around the selected nodes.
    let handles = app.node_transform_handles();
    if let (false, Some(b)) = (handles.is_empty(), app.selected_nodes_bounds()) {
        let r = view.rect_to_screen(b);
        let corners = [
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
            r.left_top(),
        ];
        dashed_polyline(
            painter,
            &corners,
            EStroke::new(1.0, Tokens::SELECTION),
            4.0,
            3.0,
        );
        let stroke = EStroke::new(1.0, Tokens::HANDLE);
        for (h, p) in handles {
            let c = view.to_screen(p);
            match app.node_transform {
                crate::node_edit::NodeTransformMode::RotateSkew if h.is_corner() => {
                    painter.circle_filled(c, 3.5, Color32::WHITE);
                    painter.circle_stroke(c, 3.5, stroke);
                }
                crate::node_edit::NodeTransformMode::RotateSkew => {
                    // A double arrow along the side.
                    let d = if matches!(h, Handle::N | Handle::S) {
                        egui::vec2(5.0, 0.0)
                    } else {
                        egui::vec2(0.0, 5.0)
                    };
                    painter.line_segment([c - d, c + d], EStroke::new(1.5, Tokens::HANDLE));
                    painter.circle_filled(c, 1.5, Tokens::HANDLE);
                }
                _ => {
                    let rr = ERect::from_center_size(c, egui::vec2(6.0, 6.0));
                    painter.rect_filled(rr, 0.0, Tokens::HANDLE);
                }
            }
        }
        if app.node_transform == crate::node_edit::NodeTransformMode::RotateSkew {
            let c = view.to_screen(b.center());
            painter.circle_stroke(c, 4.0, stroke);
            painter.circle_filled(c, 1.2, Tokens::HANDLE);
        }
    }
    if let Drag::NodeLasso { points } = &app.drag {
        let pts: Vec<Pos2> = points.iter().map(|p| view.to_screen(*p)).collect();
        dashed_polyline(
            painter,
            &pts,
            EStroke::new(1.0, Tokens::SELECTION),
            4.0,
            3.0,
        );
    }
    if let Drag::NodeMarquee { start, current } = &app.drag {
        let r = view.rect_to_screen(Rect::from_points(*start, *current));
        painter.rect_stroke(
            r,
            0.0,
            EStroke::new(1.0, Tokens::SELECTION),
            epaint::StrokeKind::Outside,
        );
    }
    crate::corner_widgets::draw(app, painter, main);
}

/// A line of `pattern` (on and off lengths in pixels; empty for solid).
/// Horizontal and vertical lines are drawn on whole pixels.
fn pattern_line(painter: &Painter, a: Pos2, b: Pos2, color: Color32, pattern: &[f32]) {
    let stroke = EStroke::new(1.0, color);
    let len = a.distance(b);
    let axis = (a.x - b.x).abs() < 0.01 || (a.y - b.y).abs() < 0.01;
    if axis && len > 0.0 {
        let mut px = crate::ui::rulers::Pixels::default();
        let dir = (b - a) / len;
        let mut t = 0.0;
        let mut i = 0;
        while t < len && i < 40_000 {
            let l = if pattern.is_empty() {
                len
            } else {
                pattern[i % pattern.len()].max(0.5)
            };
            if i % 2 == 0 {
                let p0 = a + dir * t;
                let p1 = a + dir * (t + l).min(len);
                let r = ERect::from_two_pos(p0, p1);
                let r = if (a.x - b.x).abs() < 0.01 {
                    let x = r.min.x.floor();
                    ERect::from_min_max(Pos2::new(x, r.min.y), Pos2::new(x + 1.0, r.max.y))
                } else {
                    let y = r.min.y.floor();
                    ERect::from_min_max(Pos2::new(r.min.x, y), Pos2::new(r.max.x, y + 1.0))
                };
                px.rect(r, color);
            }
            t += l;
            i += 1;
        }
        px.paint(painter);
        return;
    }
    if pattern.is_empty() || len <= 0.0 {
        painter.line_segment([a, b], stroke);
        return;
    }
    let dir = (b - a) / len;
    let mut t = 0.0;
    let mut i = 0;
    let mut segments = 0;
    while t < len && segments < 20_000 {
        let l = pattern[i % pattern.len()].max(0.5);
        if i % 2 == 0 {
            painter.line_segment([a + dir * t, a + dir * (t + l).min(len)], stroke);
            segments += 1;
        }
        t += l;
        i += 1;
    }
}

/// Selected guidelines are red.
const GUIDE_SELECTED: Color32 = Color32::from_rgb(0xFF, 0x00, 0x00);

/// One guideline across the drawing window.
fn draw_guide_line(
    app: &App,
    painter: &Painter,
    rect: ERect,
    line: tracedraw_core::document::GuideLine,
    color: Color32,
    pattern: &[f32],
) {
    use tracedraw_core::document::GuideLine;
    let view = &app.view;
    match line {
        GuideLine::Horizontal { y } => {
            let sy = view.to_screen(Point::new(0.0, y)).y.round() + 0.5;
            if sy >= rect.top() && sy <= rect.bottom() {
                pattern_line(
                    painter,
                    Pos2::new(rect.left(), sy),
                    Pos2::new(rect.right(), sy),
                    color,
                    pattern,
                );
            }
        }
        GuideLine::Vertical { x } => {
            let sx = view.to_screen(Point::new(x, 0.0)).x.round() + 0.5;
            if sx >= rect.left() && sx <= rect.right() {
                pattern_line(
                    painter,
                    Pos2::new(sx, rect.top()),
                    Pos2::new(sx, rect.bottom()),
                    color,
                    pattern,
                );
            }
        }
        GuideLine::Angled { .. } => {
            // Extend far beyond the viewport in both directions.
            let (o, d) = line.point_and_direction();
            let far = 100_000.0 / view.zoom.max(0.01) as f64;
            let s0 = view.to_screen(o - d * far);
            let s1 = view.to_screen(o + d * far);
            if let Some((c0, c1)) = clip_segment(s0, s1, rect) {
                pattern_line(painter, c0, c1, color, pattern);
            }
        }
    }
}

/// Guidelines across the drawing window in their colour and style; the
/// selected ones red, one being dragged where it would go, the rotation
/// handles of a guideline in rotate mode.
pub fn draw_guides(app: &App, painter: &Painter, rect: ERect) {
    if !app.show_guides {
        return;
    }
    let Ok(page) = app.doc().page(app.page) else {
        return;
    };
    let moving = app.guide_drag_moves();
    for (i, g) in page.guides.iter().enumerate() {
        let moved = moving.iter().find(|(k, _)| *k == i).map(|(_, g)| g);
        let g = match &app.drag {
            Drag::RotateGuide { index, guide, .. } if *index == i => guide,
            // A copy leaves the guideline in place and shows the copy.
            _ if app.drag_copy => g,
            _ => moved.unwrap_or(g),
        };
        let color = if app.selected_guides.contains(&i) {
            GUIDE_SELECTED
        } else {
            to_color32(app.guide_color(g))
        };
        draw_guide_line(app, painter, rect, g.line, color, g.style.pattern());
    }
    // The copies a right click during the drag will leave.
    if app.drag_copy {
        for (_, g) in &moving {
            draw_guide_line(
                app,
                painter,
                rect,
                g.line,
                GUIDE_SELECTED,
                g.style.pattern(),
            );
        }
    }
    if let Drag::NewGuide { horizontal, pos } = &app.drag {
        use tracedraw_core::document::GuideLine;
        let line = if *horizontal {
            GuideLine::Horizontal { y: pos.y }
        } else {
            GuideLine::Vertical { x: pos.x }
        };
        let color = to_color32(app.doc().metadata.guides.color);
        draw_guide_line(
            app,
            painter,
            rect,
            line,
            color,
            tracedraw_core::document::GuideStyle::default().pattern(),
        );
    }
    if let Some((_, pivot, handles)) = app.guide_rotate_handles() {
        let c = app.view.to_screen(pivot);
        let s = EStroke::new(1.0, Tokens::HANDLE);
        painter.circle_filled(c, 4.0, Color32::WHITE);
        painter.circle_stroke(c, 4.0, s);
        painter.circle_filled(c, 1.2, Tokens::HANDLE);
        for h in handles {
            let p = app.view.to_screen(h);
            let d = (p - c).normalized();
            let n = egui::vec2(-d.y, d.x);
            // A curved double arrow across the line, as the skew and
            // rotate handles of objects.
            let a = p + n * 6.0 - d * 2.0;
            let b = p - n * 6.0 - d * 2.0;
            painter.add(epaint::PathShape::line(
                vec![a, p + d * 1.5, b],
                EStroke::new(1.5, Tokens::HANDLE),
            ));
            painter.line_segment([a, a + d * 3.0], s);
            painter.line_segment([b, b + d * 3.0], s);
        }
    }
}

/// Outlines a docker previews (the Corners docker's cut): dashed in the
/// selection colour over the drawing.
pub fn draw_docker_preview(app: &App, painter: &Painter) {
    let stroke = EStroke::new(1.0, Tokens::SELECTION);
    for path in &app.docker_preview {
        for (mut pts, closed) in flatten(path, &app.view) {
            if closed {
                if let Some(first) = pts.first().copied() {
                    pts.push(first);
                }
            }
            dashed_polyline(painter, &pts, stroke, 4.0, 3.0);
        }
    }
    // The marked point (the Coordinates docker's origin point): a blue
    // node.
    if let Some(p) = app.docker_preview_point {
        let c = app.view.to_screen(p);
        let r = ERect::from_center_size(c, egui::vec2(7.0, 7.0));
        painter.rect_filled(r, 0.0, Tokens::SELECTION);
        painter.rect_stroke(
            r,
            0.0,
            EStroke::new(1.0, Color32::WHITE),
            egui::StrokeKind::Outside,
        );
    }
}

/// A polyline drawn in dashes of `on` px with `off` px gaps.
fn dashed_polyline(painter: &Painter, pts: &[Pos2], stroke: EStroke, on: f32, off: f32) {
    let period = on + off;
    let mut walked = 0.0f32;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = a.distance(b);
        if len <= 0.0 {
            continue;
        }
        let dir = (b - a) / len;
        let mut t = 0.0f32;
        while t < len {
            let phase = (walked + t) % period;
            if phase < on {
                let end = (t + (on - phase)).min(len);
                painter.line_segment([a + dir * t, a + dir * end], stroke);
                t = end;
            } else {
                t += period - phase;
            }
        }
        walked += len;
    }
}

/// The mark of a snapping mode, about 11 px across, centred on `c`.
pub fn draw_snap_glyph(
    painter: &Painter,
    c: Pos2,
    mode: crate::snap_points::SnapMode,
    color: Color32,
) {
    draw_snap_glyph_width(painter, c, mode, color, 1.5);
}

fn draw_snap_glyph_width(
    painter: &Painter,
    c: Pos2,
    mode: crate::snap_points::SnapMode,
    color: Color32,
    width: f32,
) {
    use crate::snap_points::SnapMode;
    let s = EStroke::new(width, color);
    let h = 5.0;
    let v = egui::vec2;
    match mode {
        SnapMode::Node => {
            painter.rect_stroke(
                ERect::from_center_size(c, v(2.0 * h - 1.0, 2.0 * h - 1.0)),
                0.0,
                s,
                epaint::StrokeKind::Middle,
            );
        }
        SnapMode::Intersection => {
            painter.line_segment([c + v(-h, -h), c + v(h, h)], s);
            painter.line_segment([c + v(-h, h), c + v(h, -h)], s);
        }
        SnapMode::Midpoint => {
            painter.add(epaint::PathShape::closed_line(
                vec![c + v(0.0, -h), c + v(h, h - 1.0), c + v(-h, h - 1.0)],
                s,
            ));
        }
        SnapMode::Quadrant => {
            painter.add(epaint::PathShape::closed_line(
                vec![c + v(0.0, -h), c + v(h, 0.0), c + v(0.0, h), c + v(-h, 0.0)],
                s,
            ));
        }
        SnapMode::Tangent => {
            painter.circle_stroke(c + v(0.0, 1.5), h - 1.5, s);
            painter.line_segment([c + v(-h, -h + 2.0), c + v(h, -h + 2.0)], s);
        }
        SnapMode::Perpendicular => {
            painter.line_segment([c + v(-h, h), c + v(h, h)], s);
            painter.line_segment([c + v(0.0, h), c + v(0.0, -h)], s);
        }
        SnapMode::Edge => {
            // A square with a short tick out of the middle of each side.
            let e = h - 0.5;
            painter.rect_stroke(
                ERect::from_center_size(c, v(2.0 * e, 2.0 * e)),
                0.0,
                s,
                epaint::StrokeKind::Middle,
            );
            for d in [v(1.0, 0.0), v(-1.0, 0.0), v(0.0, 1.0), v(0.0, -1.0)] {
                painter.line_segment([c + d * e, c + d * (e + 2.0)], s);
            }
        }
        SnapMode::Center => {
            // A circle with a cross inside it.
            painter.circle_stroke(c, h + 0.5, s);
            painter.line_segment([c + v(-h, 0.0), c + v(h, 0.0)], s);
            painter.line_segment([c + v(0.0, -h), c + v(0.0, h)], s);
        }
        SnapMode::TextBaseline => {
            // A "T" standing on its baseline.
            painter.line_segment([c + v(-h - 1.0, h), c + v(h + 1.0, h)], s);
            let thin = EStroke::new(1.0, color);
            painter.line_segment([c + v(-h + 1.0, -h), c + v(h - 1.0, -h)], thin);
            painter.line_segment([c + v(0.0, -h), c + v(0.0, h - 2.0)], thin);
        }
        SnapMode::Grid => {
            // Two lines each way, the point where they would cross.
            let thin = EStroke::new(1.0, color);
            for d in [-h / 2.0, h / 2.0] {
                painter.line_segment([c + v(d, -h), c + v(d, h)], thin);
                painter.line_segment([c + v(-h, d), c + v(h, d)], thin);
            }
        }
        SnapMode::Guideline => {
            // A line through the point, broken around it.
            painter.line_segment([c + v(-h - 1.0, 0.0), c + v(-2.0, 0.0)], s);
            painter.line_segment([c + v(2.0, 0.0), c + v(h + 1.0, 0.0)], s);
            painter.circle_filled(c, 1.5, color);
        }
    }
}

/// The point the pointer snapped to: its mode's mark and, with screen
/// tips on, the mode's name beside it (Options > Snapping).
pub fn draw_snap_mark(app: &App, painter: &Painter) {
    let Some(t) = app.snap_mark.get() else {
        return;
    };
    if !app.settings.snap.show_marks {
        return;
    }
    let c = app.view.to_screen(t.point);
    // While dragging, the edge snapped to lights up in blue.
    if let (Some(seg), false) = (t.edge, matches!(app.drag, crate::app::Drag::None)) {
        let pts: Vec<Pos2> = (0..=24)
            .map(|i| {
                app.view
                    .to_screen(tracedraw_core::geometry::ParamCurve::eval(
                        &seg,
                        i as f64 / 24.0,
                    ))
            })
            .collect();
        painter.add(egui::Shape::line(
            pts,
            EStroke::new(1.5, Tokens::DRAG_OUTLINE),
        ));
    }
    // The mark in black on a white halo, so it reads on any colour.
    draw_snap_glyph_halo(painter, c, t.mode);
    if app.settings.snap.screen_tips {
        // The mode's name in blue beside it, haloed in white.
        let font = egui::FontId::proportional(11.0);
        let text = crate::i18n::tr(t.mode.tip_key());
        let at = c + egui::vec2(12.0, -7.0);
        let halo = painter.layout_no_wrap(text.clone(), font.clone(), Color32::WHITE);
        for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
            painter.galley(at + egui::vec2(dx, dy), halo.clone(), Color32::WHITE);
        }
        let galley = painter.layout_no_wrap(text, font, Tokens::SELECTION);
        painter.galley(at, galley, Tokens::SELECTION);
    }
}

/// A line in a guide's style.
fn guide_line(
    painter: &Painter,
    a: Pos2,
    b: Pos2,
    stroke: EStroke,
    style: crate::live_guides::GuideStyle,
) {
    use crate::live_guides::GuideStyle as S;
    match style {
        S::Solid => {
            painter.line_segment([a, b], stroke);
        }
        S::Dashed => {
            painter.extend(egui::Shape::dashed_line(&[a, b], stroke, 6.0, 4.0));
        }
        S::Dotted => {
            painter.extend(egui::Shape::dashed_line(&[a, b], stroke, 1.5, 3.0));
        }
    }
}

/// Text haloed in white, so it reads on any drawing; `at` is its bottom
/// left corner, or its bottom right with `right`.
fn haloed_text(painter: &Painter, at: Pos2, text: String, color: Color32, right: bool) {
    let font = egui::FontId::proportional(11.0);
    let halo = painter.layout_no_wrap(text.clone(), font.clone(), Color32::WHITE);
    let size = halo.size();
    let at = at - egui::vec2(if right { size.x } else { 0.0 }, size.y);
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        painter.galley(at + egui::vec2(dx, dy), halo.clone(), Color32::WHITE);
    }
    let galley = painter.layout_no_wrap(text, font, color);
    painter.galley(at, galley, color);
}

/// This frame's alignment lines, equal gaps and dynamic guide, one pixel
/// wide in screen space, clipped to the canvas.
pub fn draw_live_guides(app: &App, painter: &Painter, canvas: ERect) {
    let Ok(o) = app.live_guides.try_borrow() else {
        return;
    };
    if o.is_empty() {
        return;
    }
    let painter = painter.with_clip_rect(canvas);
    let a = &app.settings.snap.align;
    let [r, g, b] = a.color;
    let stroke = EStroke::new(1.0, Color32::from_rgb(r, g, b));
    for l in &o.lines {
        let (p, q) = (app.view.to_screen(l.a), app.view.to_screen(l.b));
        // A little past both ends.
        let d = q - p;
        let u = if d.length() > 0.5 {
            d.normalized()
        } else {
            egui::Vec2::ZERO
        };
        guide_line(&painter, p - u * 6.0, q + u * 6.0, stroke, a.style);
    }
    for l in &o.gaps {
        let (p, q) = (app.view.to_screen(l.a), app.view.to_screen(l.b));
        let d = q - p;
        if d.length() < 0.5 {
            continue;
        }
        let n = egui::vec2(-d.y, d.x).normalized() * 4.0;
        painter.line_segment([p, q], stroke);
        painter.line_segment([p - n, p + n], stroke);
        painter.line_segment([q - n, q + n], stroke);
    }
    for l in &o.sizes {
        // Outside the box: below a width, left of a height.
        let (p, q) = (app.view.to_screen(l.a), app.view.to_screen(l.b));
        let d = q - p;
        if d.length() < 0.5 {
            continue;
        }
        let off = if d.x.abs() >= d.y.abs() {
            egui::vec2(0.0, 7.0)
        } else {
            egui::vec2(-7.0, 0.0)
        };
        let (p, q) = (p + off, q + off);
        let u = d.normalized();
        let n = egui::vec2(-u.y, u.x) * 4.0;
        painter.line_segment([p, q], stroke);
        painter.line_segment([p - n, p + n], stroke);
        painter.line_segment([q - n, q + n], stroke);
        // Two strokes across the middle: the same size.
        let m = p + d * 0.5;
        for k in [-1.5, 1.5] {
            let c = m + u * k;
            painter.line_segment([c - n * 0.75, c + n * 0.75], stroke);
        }
    }
    if let Some(h) = o.dynamic {
        let dopt = &app.settings.snap.dynamic;
        let [r, g, b] = dopt.color;
        let color = Color32::from_rgb(r, g, b);
        let stroke = EStroke::new(1.0, color);
        let view = app.view.visible_page_rect(canvas);
        let mut guides = vec![(h.anchor, h.angle)];
        if let Some((c, ang)) = h.cross {
            // The other guide through the crossing.
            guides.push((c, ang));
        }
        for (anchor, angle) in guides {
            if let Some(s) = crate::live_guides::guide_in_view(anchor, angle, view) {
                guide_line(
                    &painter,
                    app.view.to_screen(s.a),
                    app.view.to_screen(s.b),
                    stroke,
                    dopt.style,
                );
            }
        }
        // The anchor: a small cross.
        let c = app.view.to_screen(h.anchor);
        for (dx, dy) in [(-4.0, -4.0), (4.0, -4.0)] {
            painter.line_segment([c + egui::vec2(dx, dy), c - egui::vec2(dx, dy)], stroke);
        }
        let to = app.view.to_screen(h.point);
        if dopt.ticks {
            let along = to - c;
            if along.length() > 0.5 {
                let n = egui::vec2(-along.y, along.x).normalized() * 3.0;
                // Ticks closer than 3 px would be a smear: skip them.
                let step_px = dopt.tick_mm.max(0.01) as f32 * app.view.zoom;
                if step_px >= 3.0 {
                    for t in crate::live_guides::ticks(h.anchor, h.point, dopt.tick_mm, 1000) {
                        let s = app.view.to_screen(t);
                        painter.line_segment([s - n, s + n], stroke);
                    }
                }
            }
        }
        if dopt.tooltip {
            let u = app.units;
            let text = format!(
                "{:.2} {}, {:.1}\u{00B0}",
                u.world_from_mm(h.distance()),
                u.short(),
                h.bearing()
            );
            // Beside the guide, on its upper side, clear of the pointer.
            let a = h.angle.to_radians() as f32;
            let mut n = egui::vec2(a.sin(), a.cos());
            if n.y > 0.0 || (n.y.abs() < 1e-3 && n.x < 0.0) {
                n = -n;
            }
            haloed_text(&painter, to + n * 14.0, text, color, n.x < -0.01);
        }
    }
}

/// A snap mark in black over a wider white copy of itself.
fn draw_snap_glyph_halo(painter: &Painter, c: Pos2, mode: crate::snap_points::SnapMode) {
    // The halo: the same mark, thicker, in white.
    draw_snap_glyph_width(painter, c, mode, Color32::WHITE, 3.5);
    draw_snap_glyph_width(painter, c, mode, Tokens::HANDLE, 1.3);
}

/// Effect nodes (perspective, mesh) of the selection with the Shape tool
/// (and the mesh with the Mesh Fill tool): small blue squares joined by
/// a frame.
pub fn draw_effect_nodes(app: &App, painter: &Painter) {
    if !matches!(
        app.tool,
        crate::tools::Tool::Shape | crate::tools::Tool::MeshFill
    ) {
        return;
    }
    for s in app.selected_rc().iter() {
        let nodes = crate::interaction2::effect_nodes(s);
        if nodes.is_empty() {
            continue;
        }
        let blue = egui::Color32::from_rgb(40, 110, 220);
        // The perspective frame (envelopes: `ui::envelope_ui`).
        let persp: Vec<egui::Pos2> = nodes
            .iter()
            .filter(|(i, _)| (100..200).contains(i))
            .map(|(_, p)| app.view.to_screen(*p))
            .collect();
        if persp.len() == 4 {
            let mut pts = persp.clone();
            pts.push(persp[0]);
            painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, blue)));
        }
        // Mesh grid lines.
        if let tracedraw_core::Fill::Mesh(m) = &s.fill {
            let cols = m.cols as usize + 1;
            for r in 0..=m.rows as usize {
                let pts: Vec<egui::Pos2> = (0..cols)
                    .filter_map(|c| m.nodes.get(r * cols + c))
                    .map(|n| app.view.to_screen(s.transform * n.pos))
                    .collect();
                painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, blue)));
            }
            for c in 0..cols {
                let pts: Vec<egui::Pos2> = (0..=m.rows as usize)
                    .filter_map(|r| m.nodes.get(r * cols + c))
                    .map(|n| app.view.to_screen(s.transform * n.pos))
                    .collect();
                painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, blue)));
            }
        }
        for (i, p) in nodes {
            let sp = app.view.to_screen(p);
            let r = egui::Rect::from_center_size(sp, egui::vec2(7.0, 7.0));
            let selected = app
                .effect_node_drag
                .map(|(id, k)| id == s.id && k == i)
                .unwrap_or(false)
                || app.selected_effect_node == Some((s.id, i));
            painter.rect_filled(r, 0.0, if selected { blue } else { egui::Color32::WHITE });
            painter.rect_stroke(
                r,
                0.0,
                egui::Stroke::new(1.0, blue),
                egui::StrokeKind::Outside,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parts of the window outside the pages: bands around one page,
    /// and around two side by side, with nothing left over or covered.
    #[test]
    fn the_mask_covers_everything_outside_the_pages() {
        let area = ERect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0));
        let page = ERect::from_min_max(Pos2::new(20.0, 10.0), Pos2::new(60.0, 70.0));
        let parts = outside(area, &[page]);
        let covered: f32 = parts.iter().map(|r| r.area()).sum();
        assert!(
            (covered - (area.area() - page.area())).abs() < 1e-3,
            "{parts:?}"
        );
        assert!(parts.iter().all(|r| !r.intersects(page.shrink(0.01))));
        let second = ERect::from_min_max(Pos2::new(70.0, 30.0), Pos2::new(120.0, 50.0));
        let parts = outside(area, &[page, second]);
        let covered: f32 = parts.iter().map(|r| r.area()).sum();
        let inside = page.area() + second.intersect(area).area();
        assert!((covered - (area.area() - inside)).abs() < 1e-3, "{parts:?}");
        assert!(parts
            .iter()
            .all(|r| !r.intersects(page.shrink(0.01)) && !r.intersects(second.shrink(0.01))));
        // No page in view: the whole window.
        assert_eq!(outside(area, &[]), vec![area]);
    }

    #[test]
    fn grid_lines_thin_out_and_follow_the_origin() {
        // 10 mm at 2 px per mm is 20 px: every line.
        assert_eq!(grid_thinning(20.0, 6.0), 1.0);
        // At 0.5 px per mm, 5 px apart: every second line.
        assert_eq!(grid_thinning(5.0, 6.0), 2.0);
        assert_eq!(grid_thinning(1.0, 6.0), 10.0);
        assert!(grid_thinning(0.0, 6.0).is_infinite());
        // Lines pass through the origin.
        let xs = grid_positions(-12.0, 25.0, 3.0, 10.0);
        assert_eq!(xs, vec![-17.0, -7.0, 3.0, 13.0, 23.0, 33.0]);
        assert!(grid_positions(0.0, 1e9, 0.0, 1e-3).is_empty());
        assert!(grid_positions(0.0, 1.0, 0.0, 0.0).is_empty());
    }

    #[test]
    fn baselines_start_from_the_top_and_stop_at_the_bottom() {
        let mut app = App::headless();
        let page = app.page_rect();
        let ys = app.baseline_ys();
        let g = app.doc().metadata.grid;
        assert!((ys[0] - (page.y1 - g.baseline_start)).abs() < 1e-9);
        assert!((ys[0] - ys[1] - g.baseline_spacing).abs() < 1e-9);
        assert!(*ys.last().unwrap() >= page.y0);
        let mut m = app.doc().metadata.clone();
        m.grid.baseline_start = 0.0;
        app.run(tracedraw_core::Command::SetMetadata { metadata: m });
        assert!((app.baseline_ys()[0] - page.y1).abs() < 1e-9);
    }
}
