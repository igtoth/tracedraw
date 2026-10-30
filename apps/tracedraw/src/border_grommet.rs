//! Tools > Border and Grommet: finishing for large-format prints. Takes
//! the page (or the selection's box), grows it by a border on every side
//! (mirrored or stretched edges, or a solid colour), keeps the design
//! inside, and places grommet marks (hairline circles) along the chosen
//! edges on a new layer. See docs/behavior/border-and-grommet.md.

use crate::app::App;
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Affine, BezPath, Point, Rect, Size},
    Color, Command, Fill, Shape, Stroke,
};

/// What the border and grommets go around.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Page,
    /// The selection's box becomes the page.
    Selection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderKind {
    Mirror,
    Stretch,
    Solid,
}

impl BorderKind {
    pub const ALL: [BorderKind; 3] = [BorderKind::Mirror, BorderKind::Stretch, BorderKind::Solid];

    pub fn key(self) -> &'static str {
        match self {
            BorderKind::Mirror => "border_grommet.border_mirror",
            BorderKind::Stretch => "border_grommet.border_stretch",
            BorderKind::Solid => "border_grommet.border_solid",
        }
    }
}

/// Grommet layout, all lengths in millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct GrommetParams {
    pub diameter_mm: f64,
    /// Distance from the page edge to the grommet's rim.
    pub margin_mm: f64,
    /// Place a fixed number per edge instead of one every `spacing_mm`.
    pub by_count: bool,
    pub spacing_mm: f64,
    /// Grommets per edge in count mode, corners included.
    pub count_per_edge: u32,
    /// Where they go: the four corners, and along each edge.
    pub corners: bool,
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
    /// Centre them in the border (when there is one) instead of at the
    /// margin from the outer edge.
    pub use_border: bool,
}

impl Default for GrommetParams {
    fn default() -> Self {
        GrommetParams {
            diameter_mm: 6.35,
            margin_mm: 31.75,
            by_count: true,
            spacing_mm: 500.0,
            count_per_edge: 4,
            corners: true,
            top: true,
            bottom: true,
            left: true,
            right: true,
            use_border: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BorderGrommetState {
    pub source: Source,
    pub add_border: bool,
    pub border: BorderKind,
    pub width_mm: f64,
    pub color: Color,
    pub grommets: bool,
    pub grommet: GrommetParams,
}

impl Default for BorderGrommetState {
    fn default() -> Self {
        BorderGrommetState {
            source: Source::Page,
            add_border: false,
            border: BorderKind::Mirror,
            width_mm: 31.75,
            color: Color::WHITE,
            grommets: false,
            grommet: GrommetParams::default(),
        }
    }
}

impl BorderGrommetState {
    /// Border width actually added (zero when the border is off).
    pub fn border_mm(&self) -> f64 {
        if self.add_border {
            self.width_mm.max(0.0)
        } else {
            0.0
        }
    }

    /// Something to add: a border or grommets.
    pub fn does_something(&self) -> bool {
        self.border_mm() > 0.0 || self.grommets
    }

    /// The grommet centres on the final page of `size`.
    pub fn grommets_on(&self, size: Size) -> Vec<Point> {
        if self.grommets {
            grommet_centers(size, &self.grommet, self.border_mm())
        } else {
            Vec::new()
        }
    }

    /// Page size after the border is added.
    pub fn final_size(&self, page: Size) -> Size {
        let b = self.border_mm();
        Size::new(page.width + 2.0 * b, page.height + 2.0 * b)
    }
}

/// Grommet centres for a page of `size` with a border of `border_mm`,
/// corners first, then the chosen edges, without duplicates. Empty when
/// the page is too small for the margin.
pub fn grommet_centers(size: Size, g: &GrommetParams, border_mm: f64) -> Vec<Point> {
    let m = if g.use_border && border_mm > 0.0 {
        border_mm / 2.0
    } else {
        g.margin_mm.max(0.0) + g.diameter_mm.max(0.0) / 2.0
    };
    let (x0, x1) = (m, size.width - m);
    let (y0, y1) = (m, size.height - m);
    if x1 < x0 || y1 < y0 {
        return Vec::new();
    }
    let mut out: Vec<Point> = Vec::new();
    let mut push = |p: Point| {
        if !out
            .iter()
            .any(|q| (q.x - p.x).abs() < 1e-6 && (q.y - p.y).abs() < 1e-6)
        {
            out.push(p);
        }
    };
    if g.corners {
        for p in [
            Point::new(x0, y1),
            Point::new(x1, y1),
            Point::new(x0, y0),
            Point::new(x1, y0),
        ] {
            push(p);
        }
    }
    let count_along = |len: f64| -> usize {
        if g.by_count {
            (g.count_per_edge as usize).max(2)
        } else if g.spacing_mm > 1e-6 {
            ((len / g.spacing_mm).floor() as usize + 1).max(2)
        } else {
            2
        }
    };
    let nx = count_along(x1 - x0);
    let ny = count_along(y1 - y0);
    // Page coordinates run upwards: y1 is the top edge.
    for i in 0..nx {
        let x = x0 + (x1 - x0) * i as f64 / (nx - 1) as f64;
        if g.top {
            push(Point::new(x, y1));
        }
        if g.bottom {
            push(Point::new(x, y0));
        }
    }
    for i in 0..ny {
        let y = y0 + (y1 - y0) * i as f64 / (ny - 1) as f64;
        if g.left {
            push(Point::new(x0, y));
        }
        if g.right {
            push(Point::new(x1, y));
        }
    }
    out
}

/// A frame path: the outer rectangle with the inner one cut out
/// (non-zero winding, so the inner ring runs the other way).
pub fn frame_path(outer: Rect, inner: Rect) -> BezPath {
    let mut p = BezPath::new();
    p.move_to((outer.x0, outer.y0));
    p.line_to((outer.x1, outer.y0));
    p.line_to((outer.x1, outer.y1));
    p.line_to((outer.x0, outer.y1));
    p.close_path();
    p.move_to((inner.x0, inner.y0));
    p.line_to((inner.x0, inner.y1));
    p.line_to((inner.x1, inner.y1));
    p.line_to((inner.x1, inner.y0));
    p.close_path();
    p
}

/// Rectangles (in final page coordinates) of the four edge strips and the
/// four corner squares of a border of width `b` around a page of `page`
/// (the original size). Order: top, bottom, left, right, then the corners
/// top-left, top-right, bottom-left, bottom-right.
pub fn border_rects(page: Size, b: f64) -> [Rect; 8] {
    let (w, h) = (page.width, page.height);
    [
        Rect::new(b, h + b, w + b, h + 2.0 * b),
        Rect::new(b, 0.0, w + b, b),
        Rect::new(0.0, b, b, h + b),
        Rect::new(w + b, b, w + 2.0 * b, h + b),
        Rect::new(0.0, h + b, b, h + 2.0 * b),
        Rect::new(w + b, h + b, w + 2.0 * b, h + 2.0 * b),
        Rect::new(0.0, 0.0, b, b),
        Rect::new(w + b, 0.0, w + 2.0 * b, b),
    ]
}

/// Edge strips of the page image for a mirrored or stretched border, in
/// the order of `border_rects`. `bpx` is the border width in pixels.
fn edge_images(img: &image::RgbaImage, bpx: u32, mirror: bool) -> Vec<image::RgbaImage> {
    use image::imageops::{crop_imm, flip_horizontal, flip_vertical, resize, FilterType};
    if img.width() == 0 || img.height() == 0 {
        return Vec::new();
    }
    let (w, h) = (img.width(), img.height());
    let bx = bpx.clamp(1, w);
    let by = bpx.clamp(1, h);
    // Mirror: the strip just inside the edge, flipped outwards.
    let mirrored = |x: u32, y: u32, cw: u32, ch: u32, fh: bool, fv: bool| -> image::RgbaImage {
        let mut c = crop_imm(img, x, y, cw, ch).to_image();
        if fh {
            c = flip_horizontal(&c);
        }
        if fv {
            c = flip_vertical(&c);
        }
        c
    };
    // Stretch: the outermost row, column or pixel, scaled to the strip.
    let stretched = |x: u32, y: u32, sw: u32, sh: u32, cw: u32, ch: u32| -> image::RgbaImage {
        let c = crop_imm(img, x, y, sw, sh).to_image();
        resize(&c, cw, ch, FilterType::Nearest)
    };
    if mirror {
        vec![
            mirrored(0, 0, w, by, false, true),
            mirrored(0, h - by, w, by, false, true),
            mirrored(0, 0, bx, h, true, false),
            mirrored(w - bx, 0, bx, h, true, false),
            mirrored(0, 0, bx, by, true, true),
            mirrored(w - bx, 0, bx, by, true, true),
            mirrored(0, h - by, bx, by, true, true),
            mirrored(w - bx, h - by, bx, by, true, true),
        ]
    } else {
        vec![
            stretched(0, 0, w, 1, w, by),
            stretched(0, h - 1, w, 1, w, by),
            stretched(0, 0, 1, h, bx, h),
            stretched(w - 1, 0, 1, h, bx, h),
            stretched(0, 0, 1, 1, bx, by),
            stretched(w - 1, 0, 1, 1, bx, by),
            stretched(0, h - 1, 1, 1, bx, by),
            stretched(w - 1, h - 1, 1, 1, bx, by),
        ]
    }
}

/// Rasterise `area` of the current page (page millimetres, y up) on
/// white, at a resolution that keeps its longest side at or under about
/// 4000 pixels. The area is drawn as it is, also where it reaches past
/// the page: a selection larger than the page (a big imported photo)
/// mirrors its own edges, not what the page shows of it (#1283).
fn render_page(app: &App, area: Rect) -> Option<(image::RgbaImage, f64)> {
    let longest = area.width().max(area.height());
    if !(longest.is_finite() && longest > 0.0) {
        return None;
    }
    let dpi = (4000.0 * 25.4 / longest).clamp(10.0, 150.0);
    let zoom = dpi / 25.4;
    let w = (area.width() * zoom).ceil().max(1.0) as u32;
    let h = (area.height() * zoom).ceil().max(1.0) as u32;
    let objects = tracedraw_render::render_page(
        app.doc(),
        app.page,
        &tracedraw_render::RenderOptions {
            width: w,
            height: h,
            view: tracedraw_render::ViewTransform {
                zoom,
                origin_x: -area.x0 * zoom,
                origin_y: area.y1 * zoom,
            },
            ..Default::default()
        },
    )?;
    let mut img = image::RgbaImage::from_pixel(w, h, image::Rgba([255, 255, 255, 255]));
    // Premultiplied colours over white.
    for (px, src) in img.pixels_mut().zip(objects.pixels()) {
        let a = src.alpha() as u32;
        let over = |c: u8| (c as u32 + (255 - a) * 255 / 255).min(255) as u8;
        *px = image::Rgba([over(src.red()), over(src.green()), over(src.blue()), 255]);
    }
    Some((img, dpi))
}

/// What the border goes around, in page millimetres: the page, or the
/// selection's box (`None` with nothing selected).
pub fn source_area(app: &App, source: Source) -> Option<Rect> {
    match source {
        Source::Page => {
            let s = app.page_size();
            Some(Rect::new(0.0, 0.0, s.width, s.height))
        }
        Source::Selection => app
            .selection_bounds()
            .filter(|r| r.width() > 1e-6 && r.height() > 1e-6),
    }
}

fn bitmap_shape(app: &mut App, rect: Rect, img: &image::RgbaImage, name: &str) -> Option<Shape> {
    let png = crate::bitmap_fx::encode(img)?;
    let id = app.engine.new_shape_id();
    let mut s = Shape::new(
        id,
        ShapeKind::Bitmap {
            rect,
            width_px: img.width(),
            height_px: img.height(),
            png: png.into(),
            fx: None,
        },
    );
    s.fill = Fill::None;
    s.stroke = None;
    s.name = Some(name.into());
    Some(s)
}

/// Apply the dialog: resize the page, recentre the design and add the
/// border and grommets on a new layer. Returns the number of grommets.
pub fn apply(app: &mut App, st: &BorderGrommetState) -> usize {
    let page = app.page;
    let Some(area) = source_area(app, st.source) else {
        return 0;
    };
    let old = area.size();
    let b = st.border_mm();
    let new_size = st.final_size(old);
    let page_rect = {
        let s = app.page_size();
        Rect::new(0.0, 0.0, s.width, s.height)
    };
    let reshaped = b > 0.0 || area != page_rect;

    // Edge images come from the page as it is now, before anything moves.
    let edges = match st.border {
        BorderKind::Mirror | BorderKind::Stretch if b > 0.0 => {
            render_page(app, area).map(|(img, dpi)| {
                let bpx = (b * dpi / 25.4).round().max(1.0) as u32;
                edge_images(&img, bpx, st.border == BorderKind::Mirror)
            })
        }
        _ => None,
    };

    let mut cmds = Vec::new();
    if reshaped {
        cmds.push(Command::ResizePage {
            page,
            size: new_size,
        });
        let ids: Vec<_> = app
            .doc()
            .page(page)
            .map(|p| {
                p.layers
                    .iter()
                    .flat_map(|l| &l.shapes)
                    .map(|s| s.id)
                    .collect()
            })
            .unwrap_or_default();
        if !ids.is_empty() {
            cmds.push(Command::TransformShapes {
                shapes: ids,
                transform: Affine::translate((b - area.x0, b - area.y0)),
            });
        }
    }
    cmds.push(Command::AddLayer {
        page,
        name: crate::i18n::tr("border_grommet.layer_name"),
    });
    if let Err(e) = app.engine.run_batch("Border and Grommet", &cmds) {
        app.status = format!("Border and Grommet: {e}");
        return 0;
    }
    let Some(layer) = app
        .doc()
        .page(page)
        .ok()
        .and_then(|p| p.layers.last())
        .map(|l| l.id)
    else {
        return 0;
    };

    let mut shapes: Vec<Shape> = Vec::new();
    let border_name = crate::i18n::tr("border_grommet.object_border");
    match st.border {
        _ if b <= 0.0 => {}
        BorderKind::Solid => {
            let outer = Rect::new(0.0, 0.0, new_size.width, new_size.height);
            let inner = Rect::new(b, b, old.width + b, old.height + b);
            let id = app.engine.new_shape_id();
            let mut s = Shape::new(
                id,
                ShapeKind::Path {
                    path: frame_path(outer, inner),
                    closed: true,
                },
            );
            s.fill = Fill::Solid(st.color);
            s.stroke = None;
            s.name = Some(border_name.clone());
            shapes.push(s);
        }
        _ => {
            if let Some(imgs) = edges {
                for (rect, img) in border_rects(old, b).iter().zip(imgs.iter()) {
                    if let Some(s) = bitmap_shape(app, *rect, img, &border_name) {
                        shapes.push(s);
                    }
                }
            }
        }
    }

    let mut count = 0;
    if st.grommets {
        let d = st.grommet.diameter_mm.max(0.1);
        let name = crate::i18n::tr("border_grommet.object_grommet");
        for c in st.grommets_on(new_size) {
            let id = app.engine.new_shape_id();
            let mut s = Shape::new(
                id,
                ShapeKind::Ellipse {
                    rect: Rect::from_center_size(c, Size::new(d, d)),
                    arc: None,
                },
            );
            s.fill = Fill::None;
            s.stroke = Some(Stroke::hairline(Color::BLACK));
            s.name = Some(name.clone());
            shapes.push(s);
            count += 1;
        }
    }
    if !shapes.is_empty() {
        let cmds: Vec<Command> = shapes
            .into_iter()
            .map(|shape| Command::AddShape { layer, shape })
            .collect();
        if let Err(e) = app.engine.run_batch("Border and Grommet", &cmds) {
            app.status = format!("Border and Grommet: {e}");
        }
    }
    app.selection.clear();
    app.raster.borrow_mut().invalidate();
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(by_count: bool) -> GrommetParams {
        GrommetParams {
            diameter_mm: 10.0,
            margin_mm: 5.0,
            by_count,
            spacing_mm: 100.0,
            count_per_edge: 3,
            ..Default::default()
        }
    }

    #[test]
    fn corners_only_gives_four_inset_by_margin_and_radius() {
        let g = GrommetParams {
            top: false,
            bottom: false,
            left: false,
            right: false,
            ..params(false)
        };
        let pts = grommet_centers(Size::new(300.0, 200.0), &g, 0.0);
        assert_eq!(pts.len(), 4);
        assert!(pts.contains(&Point::new(10.0, 190.0)));
        assert!(pts.contains(&Point::new(290.0, 10.0)));
    }

    #[test]
    fn spacing_mode_fills_each_edge() {
        // 300 x 200 page, usable span 280 x 180 at 100 mm spacing:
        // 3 along the top and bottom (0, 140, 280), 2 along the sides.
        let pts = grommet_centers(Size::new(300.0, 200.0), &params(false), 0.0);
        assert_eq!(pts.len(), 6, "{pts:?}");
        assert!(pts.contains(&Point::new(150.0, 190.0)));
        assert!(pts.contains(&Point::new(150.0, 10.0)));
        // Corners are not duplicated.
        let corners = pts.iter().filter(|p| p.x == 10.0 && p.y == 10.0).count();
        assert_eq!(corners, 1);
    }

    #[test]
    fn count_mode_places_n_per_edge() {
        let pts = grommet_centers(Size::new(300.0, 200.0), &params(true), 0.0);
        // 3 per edge, 4 edges, 4 shared corners: 8.
        assert_eq!(pts.len(), 8, "{pts:?}");
        assert!(pts.contains(&Point::new(10.0, 100.0)));
        assert!(pts.contains(&Point::new(290.0, 100.0)));
    }

    #[test]
    fn too_small_a_page_gives_nothing() {
        assert!(grommet_centers(Size::new(10.0, 10.0), &params(true), 0.0).is_empty());
        let zero = GrommetParams {
            spacing_mm: 0.0,
            ..params(false)
        };
        assert_eq!(
            grommet_centers(Size::new(300.0, 200.0), &zero, 0.0).len(),
            4
        );
    }

    #[test]
    fn final_size_and_border_rects() {
        let st = BorderGrommetState {
            add_border: true,
            border: BorderKind::Solid,
            width_mm: 20.0,
            ..Default::default()
        };
        assert_eq!(
            st.final_size(Size::new(100.0, 50.0)),
            Size::new(140.0, 90.0)
        );
        let off = BorderGrommetState {
            add_border: false,
            ..st.clone()
        };
        assert_eq!(
            off.final_size(Size::new(100.0, 50.0)),
            Size::new(100.0, 50.0)
        );
        let r = border_rects(Size::new(100.0, 50.0), 20.0);
        assert_eq!(r[0], Rect::new(20.0, 70.0, 120.0, 90.0));
        assert_eq!(r[2], Rect::new(0.0, 20.0, 20.0, 70.0));
        assert_eq!(r[7], Rect::new(120.0, 0.0, 140.0, 20.0));
        let fp = frame_path(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(2.0, 2.0, 8.0, 8.0),
        );
        assert_eq!(fp.elements().len(), 10);
    }

    #[test]
    fn grommets_go_on_the_chosen_edges_or_in_the_border() {
        let size = Size::new(300.0, 200.0);
        // Top edge only, 3 per edge: three along the top, none elsewhere.
        let top = GrommetParams {
            corners: false,
            bottom: false,
            left: false,
            right: false,
            ..params(true)
        };
        let pts = grommet_centers(size, &top, 0.0);
        assert_eq!(pts.len(), 3, "{pts:?}");
        assert!(pts.iter().all(|p| (p.y - 190.0).abs() < 1e-9));
        // In a 40 mm border: centred 20 mm from the outer edge.
        let in_border = GrommetParams {
            use_border: true,
            ..params(true)
        };
        let pts = grommet_centers(size, &in_border, 40.0);
        assert!(pts.contains(&Point::new(20.0, 20.0)), "{pts:?}");
        // Without a border the margin applies.
        let pts = grommet_centers(size, &in_border, 0.0);
        assert!(pts.contains(&Point::new(10.0, 10.0)), "{pts:?}");
    }

    #[test]
    fn edge_images_have_the_strip_sizes() {
        let mut img = image::RgbaImage::from_pixel(10, 6, image::Rgba([255, 255, 255, 255]));
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        for mirror in [true, false] {
            let e = edge_images(&img, 2, mirror);
            assert_eq!((e[0].width(), e[0].height()), (10, 2));
            assert_eq!((e[2].width(), e[2].height()), (2, 6));
            assert_eq!((e[4].width(), e[4].height()), (2, 2));
            // The red corner pixel ends up at the outer corner of the
            // top-left square (mirrored) or fills it (stretched).
            assert_eq!(e[4].get_pixel(1, 1), &image::Rgba([255, 0, 0, 255]));
        }
        // A border wider than the image is clamped, never panics.
        let e = edge_images(&img, 50, true);
        assert_eq!((e[0].width(), e[0].height()), (10, 6));
    }

    #[test]
    fn the_selection_becomes_the_page_inside_its_border() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(30.0, 40.0, 90.0, 70.0),
                radius: 0.0,
                corners: None,
            })
            .expect("a rectangle");
        app.select(vec![id]);
        let area = source_area(&app, Source::Selection).expect("selection box");
        let st = BorderGrommetState {
            source: Source::Selection,
            add_border: true,
            border: BorderKind::Solid,
            width_mm: 10.0,
            grommets: true,
            grommet: GrommetParams {
                use_border: true,
                by_count: true,
                count_per_edge: 2,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(st.does_something());
        let n = apply(&mut app, &st);
        let size = app.page_size();
        assert!(
            (size.width - (area.width() + 20.0)).abs() < 1e-6,
            "{size:?}"
        );
        assert!(
            (size.height - (area.height() + 20.0)).abs() < 1e-6,
            "{size:?}"
        );
        // The box's corner sits at the border's inner corner.
        let moved = app.doc().find_shape(id).expect("kept").clone();
        let b = app.visual_bounds(&moved);
        assert!(
            (b.x0 - 10.0).abs() < 1e-6 && (b.y0 - 10.0).abs() < 1e-6,
            "{b:?}"
        );
        // Two per edge in the border: the four corners.
        assert_eq!(n, 4);
        // Nothing to do: neither a border nor grommets.
        assert!(!BorderGrommetState::default().does_something());
    }

    /// #1283: a selection reaching past the page gets its mirrored edges
    /// from itself, not from what the page shows of it (the first run on
    /// a big imported photo used to take the strips from inside it).
    #[test]
    fn a_selection_past_the_page_mirrors_its_own_edges() {
        let mut app = App::headless();
        let page = app.page_size();
        let solid = |app: &mut App, r: Rect, c: Color| {
            let id = app
                .new_shape(ShapeKind::Rect {
                    rect: r,
                    radius: 0.0,
                    corners: None,
                })
                .expect("a rectangle");
            app.select(vec![id]);
            app.apply_fill(Fill::Solid(c));
            id
        };
        // Green, wider than the page; red over the part on the page.
        let wide = Rect::new(-60.0, 50.0, page.width + 60.0, 150.0);
        let green = solid(&mut app, wide, Color::rgb8(0, 200, 0));
        solid(
            &mut app,
            Rect::new(-20.0, 40.0, page.width + 20.0, 160.0),
            Color::rgb8(220, 0, 0),
        );
        // Only the green one selected; the red one lies over its middle.
        app.select(vec![green]);
        let st = BorderGrommetState {
            source: Source::Selection,
            add_border: true,
            border: BorderKind::Mirror,
            width_mm: 10.0,
            ..Default::default()
        };
        apply(&mut app, &st);
        let strips: Vec<image::RgbaImage> = app
            .doc()
            .page(app.page)
            .ok()
            .and_then(|p| p.layers.last())
            .map(|l| {
                l.shapes
                    .iter()
                    .filter_map(|s| match &s.kind {
                        ShapeKind::Bitmap { png, .. } => {
                            image::load_from_memory(png).ok().map(|i| i.to_rgba8())
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(strips.len(), 8);
        // The left and right strips (in the order of `border_rects`).
        for strip in &strips[2..4] {
            let p = strip.get_pixel(strip.width() / 2, strip.height() / 2);
            assert!(p[1] > 150 && p[0] < 60, "green edge, got {p:?}");
        }
    }
}
