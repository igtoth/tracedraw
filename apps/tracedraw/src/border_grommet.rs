//! Tools > Border and Grommet: finishing for large-format prints. Grows
//! the page by a border on every side (mirrored or stretched page edges,
//! or a solid colour), keeps the design centred, and places grommet
//! marks (hairline circles) along the edges on a new layer.

use crate::app::App;
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Affine, BezPath, Point, Rect, Size},
    Color, Command, Fill, Shape, Stroke,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderKind {
    None,
    Mirror,
    Stretch,
    Solid,
}

impl BorderKind {
    pub const ALL: [BorderKind; 4] = [
        BorderKind::None,
        BorderKind::Mirror,
        BorderKind::Stretch,
        BorderKind::Solid,
    ];

    pub fn key(self) -> &'static str {
        match self {
            BorderKind::None => "border_grommet.border_none",
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
    pub count_per_edge: u32,
    pub corners_only: bool,
}

impl Default for GrommetParams {
    fn default() -> Self {
        GrommetParams {
            diameter_mm: 12.0,
            margin_mm: 10.0,
            by_count: false,
            spacing_mm: 500.0,
            count_per_edge: 4,
            corners_only: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BorderGrommetState {
    pub border: BorderKind,
    pub width_mm: f64,
    pub color: Color,
    pub grommets: bool,
    pub grommet: GrommetParams,
}

impl Default for BorderGrommetState {
    fn default() -> Self {
        BorderGrommetState {
            border: BorderKind::Mirror,
            width_mm: 25.0,
            color: Color::WHITE,
            grommets: true,
            grommet: GrommetParams::default(),
        }
    }
}

impl BorderGrommetState {
    /// Border width actually added (zero when the border is off).
    pub fn border_mm(&self) -> f64 {
        if self.border == BorderKind::None {
            0.0
        } else {
            self.width_mm.max(0.0)
        }
    }

    /// Page size after the border is added.
    pub fn final_size(&self, page: Size) -> Size {
        let b = self.border_mm();
        Size::new(page.width + 2.0 * b, page.height + 2.0 * b)
    }
}

/// Grommet centres for a page of `size`, corners first, then the edges,
/// without duplicates. Empty when the page is too small for the margin.
pub fn grommet_centers(size: Size, g: &GrommetParams) -> Vec<Point> {
    let m = g.margin_mm.max(0.0) + g.diameter_mm.max(0.0) / 2.0;
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
    for p in [
        Point::new(x0, y1),
        Point::new(x1, y1),
        Point::new(x0, y0),
        Point::new(x1, y0),
    ] {
        push(p);
    }
    if g.corners_only {
        return out;
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
    for i in 0..nx {
        let x = x0 + (x1 - x0) * i as f64 / (nx - 1) as f64;
        push(Point::new(x, y1));
        push(Point::new(x, y0));
    }
    for i in 0..ny {
        let y = y0 + (y1 - y0) * i as f64 / (ny - 1) as f64;
        push(Point::new(x0, y));
        push(Point::new(x1, y));
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

/// Rasterise the current page (white background) at a resolution that
/// keeps the longest side at or under about 4000 pixels.
fn render_page(app: &App) -> Option<(image::RgbaImage, f64)> {
    let size = app.page_size();
    let longest = size.width.max(size.height).max(1.0);
    let dpi = (4000.0 * 25.4 / longest).clamp(10.0, 150.0);
    let pm = tracedraw_render::render_page_image(app.doc(), app.page, dpi)?;
    let (w, h) = (pm.width(), pm.height());
    // The background is opaque white, so premultiplied data is plain RGBA.
    let img = image::RgbaImage::from_raw(w, h, pm.data().to_vec())?;
    Some((img, dpi))
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
            png,
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
    let old = app.page_size();
    let b = st.border_mm();
    let new_size = st.final_size(old);

    // Edge images come from the page as it is now, before anything moves.
    let edges = match st.border {
        BorderKind::Mirror | BorderKind::Stretch if b > 0.0 => {
            render_page(app).map(|(img, dpi)| {
                let bpx = (b * dpi / 25.4).round().max(1.0) as u32;
                edge_images(&img, bpx, st.border == BorderKind::Mirror)
            })
        }
        _ => None,
    };

    let mut cmds = Vec::new();
    if b > 0.0 {
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
                transform: Affine::translate((b, b)),
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
        BorderKind::None => {}
        BorderKind::Solid if b > 0.0 => {
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
        for c in grommet_centers(new_size, &st.grommet) {
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
            corners_only: false,
        }
    }

    #[test]
    fn corners_only_gives_four_inset_by_margin_and_radius() {
        let g = GrommetParams {
            corners_only: true,
            ..params(false)
        };
        let pts = grommet_centers(Size::new(300.0, 200.0), &g);
        assert_eq!(pts.len(), 4);
        assert!(pts.contains(&Point::new(10.0, 190.0)));
        assert!(pts.contains(&Point::new(290.0, 10.0)));
    }

    #[test]
    fn spacing_mode_fills_each_edge() {
        // 300 x 200 page, usable span 280 x 180 at 100 mm spacing:
        // 3 along the top and bottom (0, 140, 280), 2 along the sides.
        let pts = grommet_centers(Size::new(300.0, 200.0), &params(false));
        assert_eq!(pts.len(), 6, "{pts:?}");
        assert!(pts.contains(&Point::new(150.0, 190.0)));
        assert!(pts.contains(&Point::new(150.0, 10.0)));
        // Corners are not duplicated.
        let corners = pts.iter().filter(|p| p.x == 10.0 && p.y == 10.0).count();
        assert_eq!(corners, 1);
    }

    #[test]
    fn count_mode_places_n_per_edge() {
        let pts = grommet_centers(Size::new(300.0, 200.0), &params(true));
        // 3 per edge, 4 edges, 4 shared corners: 8.
        assert_eq!(pts.len(), 8, "{pts:?}");
        assert!(pts.contains(&Point::new(10.0, 100.0)));
        assert!(pts.contains(&Point::new(290.0, 100.0)));
    }

    #[test]
    fn too_small_a_page_gives_nothing() {
        assert!(grommet_centers(Size::new(10.0, 10.0), &params(true)).is_empty());
        let zero = GrommetParams {
            spacing_mm: 0.0,
            ..params(false)
        };
        assert_eq!(grommet_centers(Size::new(300.0, 200.0), &zero).len(), 4);
    }

    #[test]
    fn final_size_and_border_rects() {
        let st = BorderGrommetState {
            border: BorderKind::Solid,
            width_mm: 20.0,
            ..Default::default()
        };
        assert_eq!(
            st.final_size(Size::new(100.0, 50.0)),
            Size::new(140.0, 90.0)
        );
        let off = BorderGrommetState {
            border: BorderKind::None,
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
}
