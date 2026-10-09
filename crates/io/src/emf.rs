//! EMF and WMF import: a GDI record player.
//!
//! Both formats are streams of drawing records against a device context
//! with selectable pens, brushes and fonts. The player keeps that state,
//! maps logical coordinates through the map mode, window/viewport and
//! (EMF) world transform to device pixels, and from there to millimetres
//! using the header frame. Paths, polygons, rectangles, ellipses, arcs,
//! text and DIB bitmaps become TraceDraw objects; clip paths and rects
//! become ClipFrames around what is drawn inside them. Layouts follow the
//! public Windows metafile specifications. Nothing in the input can panic
//! the reader: every read is bounds-checked and record counts are capped.

use std::collections::HashMap;
use tracedraw_core::{
    document::{ParagraphStyle, Shape, ShapeKind, TextSpan},
    geometry::{Affine, BezPath, PathEl, Point, Rect, Shape as _, Size, Vec2},
    id::IdSource,
    style::{LineCap, LineJoin},
    Color, Fill, ShapeId, Stroke, TextAlign,
};

const PT_MM: f64 = 25.4 / 72.0;
const MAX_RECORDS: usize = 2_000_000;
const MAX_POINTS: usize = 2_000_000;
const MAX_SHAPES: usize = 500_000;

pub struct Imported {
    pub shapes: Vec<Shape>,
    pub size: Size,
    pub warnings: Vec<String>,
}

/// Enhanced metafile: record type 1 and the " EMF" signature at offset 40.
pub fn is_emf(bytes: &[u8]) -> bool {
    bytes.len() >= 48 && bytes[0..4] == [1, 0, 0, 0] && &bytes[40..44] == b" EMF"
}

/// Windows metafile: a placeable header, or a standard header of 9 words
/// with type 1 or 2.
pub fn is_wmf(bytes: &[u8]) -> bool {
    if bytes.len() < 18 {
        return false;
    }
    if u32_at(bytes, 0) == 0x9AC6_CDD7 {
        return true;
    }
    let ty = u16_at(bytes, 0);
    let hs = u16_at(bytes, 2);
    (ty == 1 || ty == 2) && hs == 9
}

pub fn parse(bytes: &[u8], ids: &mut IdSource) -> Result<Imported, String> {
    if is_emf(bytes) {
        parse_emf(bytes, ids)
    } else if is_wmf(bytes) {
        parse_wmf(bytes, ids)
    } else {
        Err("not an EMF or WMF file".into())
    }
}

/// Build a document with every imported object on one layer.
pub fn to_document(imp: Imported, title: &str) -> tracedraw_core::Document {
    let mut doc = tracedraw_core::Document::new(title, imp.size);
    if let Some(layer) = doc.pages[0].layers.first_mut() {
        layer.shapes = imp.shapes;
    }
    doc
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    match b.get(o..o + 2) {
        Some(s) => u16::from_le_bytes([s[0], s[1]]),
        None => 0,
    }
}
fn i16_at(b: &[u8], o: usize) -> i16 {
    u16_at(b, o) as i16
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    match b.get(o..o + 4) {
        Some(s) => u32::from_le_bytes([s[0], s[1], s[2], s[3]]),
        None => 0,
    }
}
fn i32_at(b: &[u8], o: usize) -> i32 {
    u32_at(b, o) as i32
}
fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_bits(u32_at(b, o))
}
fn u8_at(b: &[u8], o: usize) -> u8 {
    b.get(o).copied().unwrap_or(0)
}

fn colorref(v: u32) -> Color {
    Color::rgb8(
        (v & 0xff) as u8,
        ((v >> 8) & 0xff) as u8,
        ((v >> 16) & 0xff) as u8,
    )
}

#[derive(Clone, Debug)]
struct Pen {
    null: bool,
    style: u32,
    /// Logical units; 0 means one device pixel.
    width: f64,
    color: Color,
    /// Geometric pens (EXTCREATEPEN) carry caps and joins.
    cap: LineCap,
    join: LineJoin,
}

impl Default for Pen {
    fn default() -> Self {
        Pen {
            null: false,
            style: 0,
            width: 0.0,
            color: Color::BLACK,
            cap: LineCap::Round,
            join: LineJoin::Round,
        }
    }
}

#[derive(Clone, Debug)]
struct Brush {
    fill: Fill,
}

#[derive(Clone, Debug)]
struct Font {
    /// Logical units, negative = em height, positive = cell height.
    height: f64,
    face: String,
    bold: bool,
    italic: bool,
    underline: bool,
    strikeout: bool,
    /// Tenths of a degree, counter-clockwise.
    escapement: f64,
}

impl Default for Font {
    fn default() -> Self {
        Font {
            height: -12.0,
            face: "Arial".into(),
            bold: false,
            italic: false,
            underline: false,
            strikeout: false,
            escapement: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
enum Obj {
    Pen(Pen),
    Brush(Brush),
    Font(Font),
    Other,
}

/// Device-context state, saved and restored as a whole.
#[derive(Clone, Debug)]
struct Dc {
    map_mode: u32,
    window_org: (f64, f64),
    window_ext: (f64, f64),
    viewport_org: (f64, f64),
    viewport_ext: (f64, f64),
    world: Affine,
    pen: Pen,
    brush: Brush,
    font: Font,
    text_color: Color,
    text_align: u32,
    bk_mode: u32,
    bk_color: Color,
    polyfill_even_odd: bool,
    /// Current position, logical.
    cur: (f64, f64),
    /// Clip path in page millimetres.
    clip: Option<BezPath>,
}

impl Default for Dc {
    fn default() -> Self {
        Dc {
            map_mode: 1,
            window_org: (0.0, 0.0),
            window_ext: (1.0, 1.0),
            viewport_org: (0.0, 0.0),
            viewport_ext: (1.0, 1.0),
            world: Affine::IDENTITY,
            pen: Pen::default(),
            brush: Brush {
                fill: Fill::Solid(Color::WHITE),
            },
            font: Font::default(),
            text_color: Color::BLACK,
            text_align: 0,
            bk_mode: 2,
            bk_color: Color::WHITE,
            polyfill_even_odd: true,
            cur: (0.0, 0.0),
            clip: None,
        }
    }
}

/// Device pixels to page millimetres (y up).
#[derive(Clone, Copy, Debug)]
struct Device {
    left: f64,
    top: f64,
    sx: f64,
    sy: f64,
    page_h: f64,
    /// Logical units per device pixel for MM_TEXT files without a frame.
    dpi: f64,
}

struct Player<'a> {
    ids: &'a mut IdSource,
    dev: Device,
    dc: Dc,
    saved: Vec<Dc>,
    objects: HashMap<u32, Obj>,
    /// Path being recorded between BEGINPATH and ENDPATH, in page mm.
    path: Option<BezPath>,
    /// Last finished path, for FILLPATH / STROKEPATH / SELECTCLIPPATH.
    last_path: BezPath,
    shapes: Vec<Shape>,
    warnings: Vec<String>,
    points: usize,
    /// Between BEGINPATH and ENDPATH.
    recording: bool,
}

impl<'a> Player<'a> {
    fn new(ids: &'a mut IdSource, dev: Device) -> Self {
        Player {
            ids,
            dev,
            dc: Dc::default(),
            saved: Vec::new(),
            objects: HashMap::new(),
            path: None,
            last_path: BezPath::new(),
            shapes: Vec::new(),
            warnings: Vec::new(),
            points: 0,
            recording: false,
        }
    }

    fn warn(&mut self, msg: impl Into<String>) {
        if self.warnings.len() < 50 {
            self.warnings.push(msg.into());
        }
    }

    fn new_shape(&mut self, kind: ShapeKind) -> Shape {
        Shape::new(ShapeId(self.ids.shape().0), kind)
    }

    /// Logical to device pixels through the world transform and map mode.
    fn to_device(&self, x: f64, y: f64) -> (f64, f64) {
        let p = self.dc.world * Point::new(x, y);
        let (x, y) = (p.x, p.y);
        let dc = &self.dc;
        match dc.map_mode {
            // MM_TEXT: one logical unit is one pixel.
            1 => (
                x - dc.window_org.0 + dc.viewport_org.0,
                y - dc.window_org.1 + dc.viewport_org.1,
            ),
            // Fixed metric modes: units per inch, y up.
            2..=6 => {
                let upi = match dc.map_mode {
                    2 => 254.0,
                    3 => 2540.0,
                    4 => 100.0,
                    5 => 1000.0,
                    _ => 1440.0,
                };
                let k = self.dev.dpi / upi;
                (
                    (x - dc.window_org.0) * k + dc.viewport_org.0,
                    -(y - dc.window_org.1) * k + dc.viewport_org.1,
                )
            }
            // MM_ISOTROPIC / MM_ANISOTROPIC: window to viewport.
            _ => {
                let (wx, wy) = (
                    if dc.window_ext.0 == 0.0 {
                        1.0
                    } else {
                        dc.window_ext.0
                    },
                    if dc.window_ext.1 == 0.0 {
                        1.0
                    } else {
                        dc.window_ext.1
                    },
                );
                (
                    (x - dc.window_org.0) * dc.viewport_ext.0 / wx + dc.viewport_org.0,
                    (y - dc.window_org.1) * dc.viewport_ext.1 / wy + dc.viewport_org.1,
                )
            }
        }
    }

    /// Logical to page millimetres.
    fn to_page(&self, x: f64, y: f64) -> Point {
        let (dx, dy) = self.to_device(x, y);
        Point::new(
            (dx - self.dev.left) * self.dev.sx,
            self.dev.page_h - (dy - self.dev.top) * self.dev.sy,
        )
    }

    /// Millimetres per logical unit (geometric mean of the axes).
    fn scale(&self) -> f64 {
        let o = self.to_page(0.0, 0.0);
        let x = self.to_page(1000.0, 0.0);
        let y = self.to_page(0.0, 1000.0);
        let sx = (x - o).hypot() / 1000.0;
        let sy = (y - o).hypot() / 1000.0;
        (sx * sy).sqrt().max(1e-9)
    }

    fn stroke(&self) -> Option<Stroke> {
        let pen = &self.dc.pen;
        if pen.null || pen.style & 0xf == 5 {
            return None;
        }
        let mut s = Stroke::new(pen.color, 0.0);
        let w = pen.width * self.scale();
        s.width = if w <= 0.0 {
            Stroke::HAIRLINE
        } else {
            w.max(Stroke::HAIRLINE)
        };
        s.cap = pen.cap;
        s.join = pen.join;
        s.dash = match pen.style & 0xf {
            1 => vec![4.0, 2.0],
            2 => vec![1.0, 1.0],
            3 => vec![4.0, 2.0, 1.0, 2.0],
            4 => vec![4.0, 2.0, 1.0, 2.0, 1.0, 2.0],
            _ => Vec::new(),
        };
        // Dashes on a hairline make no sense in device terms; keep solid.
        if w <= 0.0 && !s.dash.is_empty() {
            s.dash = Vec::new();
        }
        Some(s)
    }

    fn emit_path(&mut self, path: BezPath, fill: bool, stroke: bool) {
        if path.elements().is_empty() || self.shapes.len() >= MAX_SHAPES {
            return;
        }
        let fill_v = if fill {
            self.dc.brush.fill.clone()
        } else {
            Fill::None
        };
        let stroke_v = if stroke { self.stroke() } else { None };
        if matches!(fill_v, Fill::None) && stroke_v.is_none() {
            return;
        }
        let closed = path
            .elements()
            .iter()
            .any(|e| matches!(e, PathEl::ClosePath));
        let mut s = self.new_shape(ShapeKind::Path {
            path,
            closed: closed || fill,
        });
        s.fill = fill_v;
        s.stroke = stroke_v;
        if fill && self.dc.polyfill_even_odd {
            s.data.push(("fill.rule".into(), "evenodd".into()));
        }
        self.emit(s);
    }

    /// Place a shape, inside a ClipFrame when a clip is set and does not
    /// contain it.
    fn emit(&mut self, mut shape: Shape) {
        if let Some(clip) = self.dc.clip.clone() {
            let cb = clip.bounding_box();
            let sb = shape.bounds();
            if cb.width() <= 0.0 || cb.height() <= 0.0 || sb.intersect(cb).area() <= 0.0 {
                return;
            }
            let inside = cb.x0 <= sb.x0 + 1e-6
                && cb.y0 <= sb.y0 + 1e-6
                && cb.x1 >= sb.x1 - 1e-6
                && cb.y1 >= sb.y1 - 1e-6;
            let is_text = matches!(shape.kind, ShapeKind::Text { .. });
            if !inside && !(is_text && sb.intersect(cb).area() >= 0.6 * sb.area()) {
                let mut frame = self.new_shape(ShapeKind::Path {
                    path: clip,
                    closed: true,
                });
                frame.fill = Fill::None;
                frame.stroke = None;
                let mut pc = self.new_shape(ShapeKind::ClipFrame {
                    frame: Box::new(frame),
                    contents: vec![shape],
                });
                pc.fill = Fill::None;
                pc.stroke = None;
                shape = pc;
            }
        }
        self.shapes.push(shape);
    }

    // Drawing primitives in logical coordinates.

    fn move_to(&mut self, x: f64, y: f64) {
        self.dc.cur = (x, y);
        let pt = self.to_page(x, y);
        if let Some(p) = &mut self.path {
            p.move_to(pt);
        }
    }

    fn line_to(&mut self, x: f64, y: f64) {
        let a = self.to_page(self.dc.cur.0, self.dc.cur.1);
        let b = self.to_page(x, y);
        match &mut self.path {
            Some(p) => {
                if p.elements().is_empty() {
                    p.move_to(a);
                }
                p.line_to(b);
            }
            None => {
                let mut l = BezPath::new();
                l.move_to(a);
                l.line_to(b);
                self.emit_path(l, false, true);
            }
        }
        self.dc.cur = (x, y);
    }

    fn polyline(&mut self, pts: &[(f64, f64)], to: bool) {
        if pts.is_empty() {
            return;
        }
        let page: Vec<Point> = pts.iter().map(|(x, y)| self.to_page(*x, *y)).collect();
        let cur = self.to_page(self.dc.cur.0, self.dc.cur.1);
        if let Some(p) = &mut self.path {
            if to {
                if p.elements().is_empty() {
                    p.move_to(cur);
                }
                for q in &page {
                    p.line_to(*q);
                }
            } else {
                p.move_to(page[0]);
                for q in &page[1..] {
                    p.line_to(*q);
                }
            }
        } else {
            let mut l = BezPath::new();
            if to {
                l.move_to(self.to_page(self.dc.cur.0, self.dc.cur.1));
                for q in &page {
                    l.line_to(*q);
                }
            } else {
                l.move_to(page[0]);
                for q in &page[1..] {
                    l.line_to(*q);
                }
            }
            self.emit_path(l, false, true);
        }
        if let Some(last) = pts.last() {
            self.dc.cur = *last;
        }
    }

    fn polybezier(&mut self, pts: &[(f64, f64)], to: bool) {
        let page: Vec<Point> = pts.iter().map(|(x, y)| self.to_page(*x, *y)).collect();
        let (start, rest) = if to {
            (self.to_page(self.dc.cur.0, self.dc.cur.1), &page[..])
        } else {
            match page.split_first() {
                Some((f, r)) => (*f, r),
                None => return,
            }
        };
        let mut l = self.path.take().unwrap_or_default();
        if !self.recording || l.elements().is_empty() || !to {
            l.move_to(start);
        }
        for c in rest.chunks(3) {
            if c.len() == 3 {
                l.curve_to(c[0], c[1], c[2]);
            }
        }
        if let Some(last) = pts.last() {
            self.dc.cur = *last;
        }
        if self.recording {
            self.path = Some(l);
        } else {
            self.emit_path(l, false, true);
        }
    }

    fn polygon(&mut self, pts: &[(f64, f64)]) {
        if pts.len() < 2 {
            return;
        }
        let page: Vec<Point> = pts.iter().map(|(x, y)| self.to_page(*x, *y)).collect();
        if let Some(p) = &mut self.path {
            p.move_to(page[0]);
            for q in &page[1..] {
                p.line_to(*q);
            }
            p.close_path();
            return;
        }
        let mut l = BezPath::new();
        l.move_to(page[0]);
        for q in &page[1..] {
            l.line_to(*q);
        }
        l.close_path();
        self.emit_path(l, true, true);
    }

    fn polypolygon(&mut self, polys: &[Vec<(f64, f64)>]) {
        let mut l = BezPath::new();
        for poly in polys {
            if poly.len() < 2 {
                continue;
            }
            let page: Vec<Point> = poly.iter().map(|(x, y)| self.to_page(*x, *y)).collect();
            l.move_to(page[0]);
            for q in &page[1..] {
                l.line_to(*q);
            }
            l.close_path();
        }
        if let Some(p) = &mut self.path {
            p.extend(l.elements().iter().copied());
            return;
        }
        self.emit_path(l, true, true);
    }

    fn rect_path(&self, l: f64, t: f64, r: f64, b: f64, radius: f64) -> BezPath {
        let a = self.to_page(l, t);
        let c = self.to_page(r, b);
        let rect = Rect::from_points(a, c);
        tracedraw_core::geometry::rect_path(rect, radius)
    }

    fn rectangle(&mut self, l: f64, t: f64, r: f64, b: f64, radius: f64) {
        let path = self.rect_path(l, t, r, b, radius);
        if let Some(p) = &mut self.path {
            p.extend(path.elements().iter().copied());
            return;
        }
        let a = self.to_page(l, t);
        let c = self.to_page(r, b);
        let rect = Rect::from_points(a, c);
        let mut s = self.new_shape(ShapeKind::Rect { rect, radius });
        s.fill = self.dc.brush.fill.clone();
        s.stroke = self.stroke();
        if matches!(s.fill, Fill::None) && s.stroke.is_none() {
            return;
        }
        self.emit(s);
    }

    fn ellipse(&mut self, l: f64, t: f64, r: f64, b: f64) {
        let a = self.to_page(l, t);
        let c = self.to_page(r, b);
        let rect = Rect::from_points(a, c);
        if let Some(p) = &mut self.path {
            p.extend(
                tracedraw_core::geometry::Ellipse::from_rect(rect)
                    .to_path(0.01)
                    .elements()
                    .iter()
                    .copied(),
            );
            return;
        }
        let mut s = self.new_shape(ShapeKind::Ellipse { rect, arc: None });
        s.fill = self.dc.brush.fill.clone();
        s.stroke = self.stroke();
        if matches!(s.fill, Fill::None) && s.stroke.is_none() {
            return;
        }
        self.emit(s);
    }

    #[allow(clippy::too_many_arguments)]
    /// Arc, chord or pie of the ellipse in the box, between the radials
    /// through the two points (counter-clockwise in logical y-down space,
    /// which is clockwise on the page).
    fn arc(&mut self, l: f64, t: f64, r: f64, b: f64, p1: (f64, f64), p2: (f64, f64), kind: u8) {
        let a = self.to_page(l, t);
        let c = self.to_page(r, b);
        let rect = Rect::from_points(a, c);
        let cen = rect.center();
        let (rx, ry) = (rect.width() / 2.0, rect.height() / 2.0);
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        let q1 = self.to_page(p1.0, p1.1);
        let q2 = self.to_page(p2.0, p2.1);
        let ang = |q: Point| ((q.y - cen.y) / ry).atan2((q.x - cen.x) / rx);
        let a1 = ang(q1);
        let mut a2 = ang(q2);
        if a2 <= a1 {
            a2 += std::f64::consts::TAU;
        }
        let n = ((a2 - a1) / (std::f64::consts::PI / 2.0)).ceil().max(1.0) as usize;
        let step = (a2 - a1) / n as f64;
        let mut path = BezPath::new();
        let pt = |ang: f64| Point::new(cen.x + rx * ang.cos(), cen.y + ry * ang.sin());
        let start = pt(a1);
        if kind == 2 {
            path.move_to(cen);
            path.line_to(start);
        } else {
            path.move_to(start);
        }
        for i in 0..n {
            let t0 = a1 + step * i as f64;
            let t1 = t0 + step;
            let k = 4.0 / 3.0 * ((t1 - t0) / 4.0).tan();
            let p0 = pt(t0);
            let p3 = pt(t1);
            let c1 = Point::new(p0.x - k * rx * t0.sin(), p0.y + k * ry * t0.cos());
            let c2 = Point::new(p3.x + k * rx * t1.sin(), p3.y - k * ry * t1.cos());
            path.curve_to(c1, c2, p3);
        }
        if kind != 0 {
            path.close_path();
        }
        if let Some(p) = &mut self.path {
            p.extend(path.elements().iter().copied());
            return;
        }
        self.emit_path(path, kind != 0, true);
    }

    fn text(&mut self, x: f64, y: f64, text: String) {
        if text.trim().is_empty() || self.shapes.len() >= MAX_SHAPES {
            return;
        }
        let f = self.dc.font.clone();
        let scale = self.scale();
        let em_mm = if f.height < 0.0 {
            -f.height * scale
        } else if f.height > 0.0 {
            f.height * scale * 0.85
        } else {
            12.0 * scale
        };
        let size_pt = (em_mm / PT_MM).max(0.1);
        let origin = self.to_page(x, y);
        let align = match self.dc.text_align & 6 {
            2 => TextAlign::Right,
            6 => TextAlign::Center,
            _ => TextAlign::Left,
        };
        // Vertical reference: top (default), bottom (8) or baseline (24).
        let dy = match self.dc.text_align & 24 {
            24 => 0.0,
            8 => 0.2 * em_mm,
            _ => -0.8 * em_mm,
        };
        let mut span = TextSpan::new(text, f.face.clone(), size_pt);
        span.bold = f.bold;
        span.italic = f.italic;
        span.underline = f.underline;
        span.strikethrough = f.strikeout;
        let mut s = self.new_shape(ShapeKind::Text {
            spans: vec![span],
            origin: Point::ZERO,
            frame: None,
            align,
            para: ParagraphStyle::default(),
            on_path: None,
        });
        let rot = (f.escapement / 10.0).to_radians();
        s.transform = Affine::translate(origin.to_vec2())
            * Affine::rotate(rot)
            * Affine::translate(Vec2::new(0.0, dy));
        s.fill = Fill::Solid(self.dc.text_color);
        s.stroke = None;
        self.emit(s);
    }

    /// A DIB (BITMAPINFOHEADER + pixels) placed in the logical rectangle.
    fn dib(&mut self, bmi: &[u8], bits: &[u8], x: f64, y: f64, w: f64, h: f64) {
        let Some((pw, ph, rgba)) = decode_dib(bmi, bits) else {
            self.warn("bitmap with an unsupported DIB format skipped");
            return;
        };
        let a = self.to_page(x, y);
        let b = self.to_page(x + w, y + h);
        let rect = Rect::from_points(a, b);
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }
        let png = crate::svg::encode_png(pw, ph, &rgba);
        if png.is_empty() {
            return;
        }
        // The image's top-left corner is at `a`; when the mapping puts it
        // below or right of the opposite corner the image is mirrored.
        let flip_y = a.y < b.y;
        let flip_x = a.x > b.x;
        let mut s = self.new_shape(ShapeKind::Bitmap {
            rect,
            width_px: pw,
            height_px: ph,
            png,
        });
        let c = rect.center();
        s.transform = Affine::translate(c.to_vec2())
            * Affine::scale_non_uniform(
                if flip_x { -1.0 } else { 1.0 },
                if flip_y { -1.0 } else { 1.0 },
            )
            * Affine::translate(-c.to_vec2());
        s.fill = Fill::None;
        s.stroke = None;
        self.emit(s);
    }

    fn select_object(&mut self, h: u32) {
        if h & 0x8000_0000 != 0 {
            // Stock objects.
            match h & 0x7fff_ffff {
                0 => self.dc.brush.fill = Fill::Solid(Color::WHITE),
                1 => self.dc.brush.fill = Fill::Solid(Color::rgb8(192, 192, 192)),
                2 => self.dc.brush.fill = Fill::Solid(Color::rgb8(128, 128, 128)),
                3 => self.dc.brush.fill = Fill::Solid(Color::rgb8(64, 64, 64)),
                4 => self.dc.brush.fill = Fill::Solid(Color::BLACK),
                5 => self.dc.brush.fill = Fill::None,
                6 => {
                    self.dc.pen = Pen {
                        color: Color::WHITE,
                        ..Pen::default()
                    }
                }
                7 => self.dc.pen = Pen::default(),
                8 => {
                    self.dc.pen = Pen {
                        null: true,
                        ..Pen::default()
                    }
                }
                _ => {}
            }
            return;
        }
        match self.objects.get(&h) {
            Some(Obj::Pen(p)) => self.dc.pen = p.clone(),
            Some(Obj::Brush(b)) => self.dc.brush = b.clone(),
            Some(Obj::Font(f)) => self.dc.font = f.clone(),
            _ => {}
        }
    }

    fn set_clip_rect(&mut self, l: f64, t: f64, r: f64, b: f64) {
        let path = self.rect_path(l, t, r, b, 0.0);
        self.intersect_clip(path);
    }

    fn intersect_clip(&mut self, path: BezPath) {
        self.dc.clip = Some(match self.dc.clip.take() {
            Some(c) => {
                let r = tracedraw_core::shaping::overlay(
                    &c,
                    &path,
                    tracedraw_core::shaping::Op::Intersect,
                );
                if r.elements().is_empty() {
                    path
                } else {
                    r
                }
            }
            None => path,
        });
    }
}

/// Decode a DIB into RGBA8: 1, 4, 8 (palette), 16, 24 and 32 bit
/// uncompressed, BI_BITFIELDS 32 bit, and JPEG or PNG payloads.
fn decode_dib(bmi: &[u8], bits: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if bmi.len() < 40 {
        return None;
    }
    let hdr = u32_at(bmi, 0) as usize;
    let w = i32_at(bmi, 4);
    let h_raw = i32_at(bmi, 8);
    let bpp = u16_at(bmi, 14) as usize;
    let comp = u32_at(bmi, 16);
    let clr_used = u32_at(bmi, 32) as usize;
    if w <= 0 || h_raw == 0 || w > 20000 || h_raw.unsigned_abs() > 20000 {
        return None;
    }
    let (w, h) = (w as u32, h_raw.unsigned_abs());
    let top_down = h_raw < 0;
    if comp == 4 || comp == 5 {
        // BI_JPEG / BI_PNG
        let fmt = if comp == 4 {
            image::ImageFormat::Jpeg
        } else {
            image::ImageFormat::Png
        };
        let img = image::load_from_memory_with_format(bits, fmt)
            .ok()?
            .to_rgba8();
        let (iw, ih) = img.dimensions();
        return Some((iw, ih, img.into_raw()));
    }
    let row_bytes = (w as usize * bpp).div_ceil(32) * 4;
    // Uncompressed data must be there in full; this also bounds the work
    // on damaged headers claiming huge sizes.
    if comp != 0 && comp != 3 {
        return None;
    }
    if row_bytes.saturating_mul(h as usize) > bits.len() || (w as u64) * (h as u64) > 50_000_000 {
        return None;
    }
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let palette_at = hdr;
    let palette_len = if bpp <= 8 {
        let n = if clr_used > 0 { clr_used } else { 1 << bpp };
        n.min(256).min(bmi.len().saturating_sub(hdr) / 4)
    } else {
        0
    };
    let palette: Vec<[u8; 3]> = (0..palette_len)
        .map(|i| {
            let o = palette_at + i * 4;
            [u8_at(bmi, o + 2), u8_at(bmi, o + 1), u8_at(bmi, o)]
        })
        .collect();
    // Bit masks for 16/32-bit BITFIELDS.
    let masks = if comp == 3 {
        let o = hdr;
        let m = if bmi.len() >= o + 12 {
            [u32_at(bmi, o), u32_at(bmi, o + 4), u32_at(bmi, o + 8)]
        } else {
            [u32_at(bmi, 40), u32_at(bmi, 44), u32_at(bmi, 48)]
        };
        Some(m)
    } else {
        None
    };
    let channel = |v: u32, mask: u32| -> u8 {
        if mask == 0 {
            return 0;
        }
        let shift = mask.trailing_zeros();
        let bits = (mask >> shift).count_ones();
        let raw = (v & mask) >> shift;
        if bits >= 8 {
            (raw >> (bits - 8)) as u8
        } else {
            ((raw as u64 * 255) / ((1u64 << bits) - 1)) as u8
        }
    };
    let mut any_alpha = false;
    for row in 0..h as usize {
        let src_row = if top_down { row } else { h as usize - 1 - row };
        let base = src_row * row_bytes;
        for x in 0..w as usize {
            let o = (row * w as usize + x) * 4;
            let px: [u8; 4] = match bpp {
                1 => {
                    let byte = u8_at(bits, base + x / 8);
                    let idx = ((byte >> (7 - (x % 8))) & 1) as usize;
                    let c = palette.get(idx).copied().unwrap_or([0, 0, 0]);
                    [c[0], c[1], c[2], 255]
                }
                4 => {
                    let byte = u8_at(bits, base + x / 2);
                    let idx = if x % 2 == 0 { byte >> 4 } else { byte & 0xf } as usize;
                    let c = palette.get(idx).copied().unwrap_or([0, 0, 0]);
                    [c[0], c[1], c[2], 255]
                }
                8 => {
                    let idx = u8_at(bits, base + x) as usize;
                    let c = palette.get(idx).copied().unwrap_or([0, 0, 0]);
                    [c[0], c[1], c[2], 255]
                }
                16 => {
                    let v = u16_at(bits, base + x * 2) as u32;
                    match masks {
                        Some(m) => [channel(v, m[0]), channel(v, m[1]), channel(v, m[2]), 255],
                        None => [
                            (((v >> 10) & 31) * 255 / 31) as u8,
                            (((v >> 5) & 31) * 255 / 31) as u8,
                            ((v & 31) * 255 / 31) as u8,
                            255,
                        ],
                    }
                }
                24 => {
                    let o = base + x * 3;
                    [u8_at(bits, o + 2), u8_at(bits, o + 1), u8_at(bits, o), 255]
                }
                32 => {
                    let v = u32_at(bits, base + x * 4);
                    match masks {
                        Some(m) => {
                            let a = !(m[0] | m[1] | m[2]);
                            let al = if a != 0 { channel(v, a) } else { 255 };
                            if al != 255 {
                                any_alpha = true;
                            }
                            [channel(v, m[0]), channel(v, m[1]), channel(v, m[2]), al]
                        }
                        None => {
                            let a = (v >> 24) as u8;
                            if a != 0 {
                                any_alpha = true;
                            }
                            [
                                ((v >> 16) & 0xff) as u8,
                                ((v >> 8) & 0xff) as u8,
                                (v & 0xff) as u8,
                                a,
                            ]
                        }
                    }
                }
                _ => return None,
            };
            rgba[o..o + 4].copy_from_slice(&px);
        }
    }
    // 32-bit DIBs without any alpha written are opaque.
    if bpp == 32 && !any_alpha {
        for px in rgba.chunks_mut(4) {
            px[3] = 255;
        }
    }
    Some((w, h, rgba))
}

// ---------------------------------------------------------------- EMF

fn parse_emf(b: &[u8], ids: &mut IdSource) -> Result<Imported, String> {
    let hsize = u32_at(b, 4) as usize;
    if hsize < 88 || hsize > b.len() {
        return Err("bad EMF header".into());
    }
    let bounds = [i32_at(b, 8), i32_at(b, 12), i32_at(b, 16), i32_at(b, 20)];
    let frame = [i32_at(b, 24), i32_at(b, 28), i32_at(b, 32), i32_at(b, 36)];
    let dev_px = (i32_at(b, 72), i32_at(b, 76));
    let dev_mm = (i32_at(b, 80), i32_at(b, 84));
    let dpi = if dev_mm.0 > 0 {
        dev_px.0 as f64 / (dev_mm.0 as f64 / 25.4)
    } else {
        96.0
    };
    let bw = (bounds[2] - bounds[0] + 1).max(1) as f64;
    let bh = (bounds[3] - bounds[1] + 1).max(1) as f64;
    let fw = (frame[2] - frame[0]).max(1) as f64 / 100.0;
    let fh = (frame[3] - frame[1]).max(1) as f64 / 100.0;
    let (page_w, page_h) = if fw > 0.0 && fh > 0.0 {
        (fw, fh)
    } else {
        (bw / dpi * 25.4, bh / dpi * 25.4)
    };
    let dev = Device {
        left: bounds[0] as f64,
        top: bounds[1] as f64,
        sx: page_w / bw,
        sy: page_h / bh,
        page_h,
        dpi,
    };
    let mut pl = Player::new(ids, dev);
    let mut off = hsize;
    let mut n = 0;
    while off + 8 <= b.len() && n < MAX_RECORDS {
        n += 1;
        let ty = u32_at(b, off);
        let size = u32_at(b, off + 4) as usize;
        if size < 8 || off + size > b.len() {
            pl.warn("truncated record; stopped");
            break;
        }
        let r = &b[off..off + size];
        if ty == 14 {
            break;
        }
        pl.emf_record(ty, r);
        off += size;
    }
    if pl.points > MAX_POINTS {
        pl.warn("point limit reached; drawing truncated");
    }
    Ok(Imported {
        shapes: pl.shapes,
        size: Size::new(page_w, page_h),
        warnings: pl.warnings,
    })
}

fn points32(r: &[u8], off: usize, n: usize) -> Vec<(f64, f64)> {
    (0..n)
        .filter(|i| off + i * 8 + 8 <= r.len())
        .map(|i| {
            (
                i32_at(r, off + i * 8) as f64,
                i32_at(r, off + i * 8 + 4) as f64,
            )
        })
        .collect()
}

fn points16(r: &[u8], off: usize, n: usize) -> Vec<(f64, f64)> {
    (0..n)
        .filter(|i| off + i * 4 + 4 <= r.len())
        .map(|i| {
            (
                i16_at(r, off + i * 4) as f64,
                i16_at(r, off + i * 4 + 2) as f64,
            )
        })
        .collect()
}

fn utf16_at(r: &[u8], off: usize, chars: usize) -> String {
    let units: Vec<u16> = (0..chars)
        .filter(|i| off + i * 2 + 2 <= r.len())
        .map(|i| u16_at(r, off + i * 2))
        .take_while(|u| *u != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

impl Player<'_> {
    fn emf_record(&mut self, ty: u32, r: &[u8]) {
        let count = |o: usize| -> usize { (u32_at(r, o) as usize).min(MAX_POINTS) };
        match ty {
            // Polygons and lines, 32-bit points: bounds(16) count(4) points.
            2..=6 => {
                let n = count(24);
                let pts = points32(r, 28, n);
                self.points += pts.len();
                match ty {
                    2 => self.polybezier(&pts, false),
                    3 => self.polygon(&pts),
                    4 => self.polyline(&pts, false),
                    5 => self.polybezier(&pts, true),
                    _ => self.polyline(&pts, true),
                }
            }
            // 16-bit variants.
            85..=89 => {
                let n = count(24);
                let pts = points16(r, 28, n);
                self.points += pts.len();
                match ty {
                    85 => self.polybezier(&pts, false),
                    86 => self.polygon(&pts),
                    87 => self.polyline(&pts, false),
                    88 => self.polybezier(&pts, true),
                    _ => self.polyline(&pts, true),
                }
            }
            // POLYPOLYLINE / POLYPOLYGON: bounds, nPolys, cpts, counts, points.
            7 | 8 | 90 | 91 => {
                let npolys = count(24).min(100_000);
                let total = count(28);
                let counts: Vec<usize> = (0..npolys).map(|i| count(32 + i * 4)).collect();
                let pts_off = 32 + npolys * 4;
                let pts = if ty < 50 {
                    points32(r, pts_off, total)
                } else {
                    points16(r, pts_off, total)
                };
                self.points += pts.len();
                let mut polys = Vec::new();
                let mut at = 0;
                for c in counts {
                    let end = (at + c).min(pts.len());
                    if at < end {
                        polys.push(pts[at..end].to_vec());
                    }
                    at = end;
                }
                if ty == 8 || ty == 91 {
                    self.polypolygon(&polys);
                } else {
                    for p in polys {
                        self.polyline(&p, false);
                    }
                }
            }
            9 => self.dc.window_ext = (i32_at(r, 8) as f64, i32_at(r, 12) as f64),
            10 => self.dc.window_org = (i32_at(r, 8) as f64, i32_at(r, 12) as f64),
            11 => self.dc.viewport_ext = (i32_at(r, 8) as f64, i32_at(r, 12) as f64),
            12 => self.dc.viewport_org = (i32_at(r, 8) as f64, i32_at(r, 12) as f64),
            17 => self.dc.map_mode = u32_at(r, 8),
            18 => self.dc.bk_mode = u32_at(r, 8),
            19 => self.dc.polyfill_even_odd = u32_at(r, 8) != 2,
            24 => self.dc.text_color = colorref(u32_at(r, 8)),
            25 => self.dc.bk_color = colorref(u32_at(r, 8)),
            22 => self.dc.text_align = u32_at(r, 8),
            27 => {
                let (x, y) = (i32_at(r, 8) as f64, i32_at(r, 12) as f64);
                self.move_to(x, y);
            }
            30 => {
                let (l, t, rr, bb) = (
                    i32_at(r, 8) as f64,
                    i32_at(r, 12) as f64,
                    i32_at(r, 16) as f64,
                    i32_at(r, 20) as f64,
                );
                self.set_clip_rect(l, t, rr, bb);
            }
            33 => {
                if self.saved.len() < 256 {
                    self.saved.push(self.dc.clone());
                }
            }
            34 => {
                let n = i32_at(r, 8);
                let back = if n < 0 { (-n) as usize } else { 1 };
                for _ in 0..back {
                    if let Some(dc) = self.saved.pop() {
                        self.dc = dc;
                    }
                }
            }
            35 | 36 => {
                let m = Affine::new([
                    f32_at(r, 8) as f64,
                    f32_at(r, 12) as f64,
                    f32_at(r, 16) as f64,
                    f32_at(r, 20) as f64,
                    f32_at(r, 24) as f64,
                    f32_at(r, 28) as f64,
                ]);
                if m.as_coeffs().iter().all(|v| v.is_finite()) {
                    if ty == 35 {
                        self.dc.world = m;
                    } else {
                        match u32_at(r, 32) {
                            1 => self.dc.world = Affine::IDENTITY,
                            2 => self.dc.world *= m,
                            3 => self.dc.world = m * self.dc.world,
                            _ => {}
                        }
                    }
                }
            }
            37 => self.select_object(u32_at(r, 8)),
            38 => {
                let h = u32_at(r, 8);
                let pen = Pen {
                    null: u32_at(r, 12) & 0xf == 5,
                    style: u32_at(r, 12),
                    width: i32_at(r, 16) as f64,
                    color: colorref(u32_at(r, 24)),
                    ..Pen::default()
                };
                self.objects.insert(h, Obj::Pen(pen));
            }
            95 => {
                let h = u32_at(r, 8);
                let style = u32_at(r, 28);
                let width = u32_at(r, 32) as f64;
                let brush_style = u32_at(r, 36);
                let color = colorref(u32_at(r, 40));
                let cap = match style & 0xf00 {
                    0x100 => LineCap::Square,
                    0x200 => LineCap::Butt,
                    _ => LineCap::Round,
                };
                let join = match style & 0xf000 {
                    0x1000 => LineJoin::Bevel,
                    0x2000 => LineJoin::Miter,
                    _ => LineJoin::Round,
                };
                let pen = Pen {
                    null: style & 0xf == 5 || brush_style == 1,
                    style,
                    width,
                    color,
                    cap,
                    join,
                };
                self.objects.insert(h, Obj::Pen(pen));
            }
            39 => {
                let h = u32_at(r, 8);
                let style = u32_at(r, 12);
                let color = colorref(u32_at(r, 16));
                let fill = match style {
                    1 => Fill::None,
                    _ => Fill::Solid(color),
                };
                self.objects.insert(h, Obj::Brush(Brush { fill }));
            }
            40 => {
                self.objects.remove(&u32_at(r, 8));
            }
            42 | 43 => {
                let (l, t, rr, bb) = (
                    i32_at(r, 8) as f64,
                    i32_at(r, 12) as f64,
                    i32_at(r, 16) as f64,
                    i32_at(r, 20) as f64,
                );
                if ty == 42 {
                    self.ellipse(l, t, rr, bb);
                } else {
                    self.rectangle(l, t, rr, bb, 0.0);
                }
            }
            44 => {
                let (l, t, rr, bb) = (
                    i32_at(r, 8) as f64,
                    i32_at(r, 12) as f64,
                    i32_at(r, 16) as f64,
                    i32_at(r, 20) as f64,
                );
                let radius = (i32_at(r, 24).min(i32_at(r, 28)) as f64 / 2.0).abs() * self.scale();
                self.rectangle(l, t, rr, bb, radius);
            }
            45 | 46 | 47 | 55 => {
                let (l, t, rr, bb) = (
                    i32_at(r, 8) as f64,
                    i32_at(r, 12) as f64,
                    i32_at(r, 16) as f64,
                    i32_at(r, 20) as f64,
                );
                let p1 = (i32_at(r, 24) as f64, i32_at(r, 28) as f64);
                let p2 = (i32_at(r, 32) as f64, i32_at(r, 36) as f64);
                let kind = match ty {
                    46 => 1,
                    47 => 2,
                    _ => 0,
                };
                self.arc(l, t, rr, bb, p1, p2, kind);
            }
            54 => {
                let (x, y) = (i32_at(r, 8) as f64, i32_at(r, 12) as f64);
                self.line_to(x, y);
            }
            59 => {
                self.path = Some(BezPath::new());
                self.recording = true;
            }
            60 => {
                self.recording = false;
                self.last_path = self.path.take().unwrap_or_default();
            }
            61 => {
                if let Some(p) = &mut self.path {
                    p.close_path();
                }
            }
            62 => {
                let p = self.last_path.clone();
                self.emit_path(p, true, false);
            }
            63 => {
                let p = self.last_path.clone();
                self.emit_path(p, true, true);
            }
            64 => {
                let p = self.last_path.clone();
                self.emit_path(p, false, true);
            }
            67 => {
                let p = self.last_path.clone();
                if !p.elements().is_empty() {
                    match u32_at(r, 8) {
                        // RGN_COPY
                        5 => self.dc.clip = Some(p),
                        _ => self.intersect_clip(p),
                    }
                }
            }
            68 => {
                self.path = None;
                self.recording = false;
            }
            75 => {
                // EXTSELECTCLIPRGN: region data of rectangles in device units.
                let mode = u32_at(r, 12);
                if mode == 5 && u32_at(r, 8) == 0 {
                    self.dc.clip = None;
                } else if let Some(path) = self.region_path(r, 16) {
                    match mode {
                        5 => self.dc.clip = Some(path),
                        _ => self.intersect_clip(path),
                    }
                }
            }
            76 | 77 => {
                // BITBLT / STRETCHBLT: dest rect, rop, src, xform, bk, usage, bmi, bits.
                let (x, y, w, h) = (
                    i32_at(r, 24) as f64,
                    i32_at(r, 28) as f64,
                    i32_at(r, 32) as f64,
                    i32_at(r, 36) as f64,
                );
                let (off_bmi, cb_bmi, off_bits, cb_bits) = (
                    u32_at(r, 84) as usize,
                    u32_at(r, 88) as usize,
                    u32_at(r, 92) as usize,
                    u32_at(r, 96) as usize,
                );
                if cb_bmi > 0 {
                    if let (Some(bmi), Some(bits)) = (
                        r.get(off_bmi..off_bmi.saturating_add(cb_bmi)),
                        r.get(off_bits..off_bits.saturating_add(cb_bits)),
                    ) {
                        self.dib(bmi, bits, x, y, w, h);
                    }
                }
            }
            81 => {
                // STRETCHDIBITS.
                let (x, y) = (i32_at(r, 24) as f64, i32_at(r, 28) as f64);
                let (off_bmi, cb_bmi, off_bits, cb_bits) = (
                    u32_at(r, 48) as usize,
                    u32_at(r, 52) as usize,
                    u32_at(r, 56) as usize,
                    u32_at(r, 60) as usize,
                );
                let (w, h) = (i32_at(r, 72) as f64, i32_at(r, 76) as f64);
                if let (Some(bmi), Some(bits)) = (
                    r.get(off_bmi..off_bmi.saturating_add(cb_bmi)),
                    r.get(off_bits..off_bits.saturating_add(cb_bits)),
                ) {
                    self.dib(bmi, bits, x, y, w, h);
                }
            }
            114 => {
                // ALPHABLEND: like STRETCHBLT with a blend function.
                let (x, y, w, h) = (
                    i32_at(r, 24) as f64,
                    i32_at(r, 28) as f64,
                    i32_at(r, 32) as f64,
                    i32_at(r, 36) as f64,
                );
                let (off_bmi, cb_bmi, off_bits, cb_bits) = (
                    u32_at(r, 84) as usize,
                    u32_at(r, 88) as usize,
                    u32_at(r, 92) as usize,
                    u32_at(r, 96) as usize,
                );
                if let (Some(bmi), Some(bits)) = (
                    r.get(off_bmi..off_bmi.saturating_add(cb_bmi)),
                    r.get(off_bits..off_bits.saturating_add(cb_bits)),
                ) {
                    self.dib(bmi, bits, x, y, w, h);
                }
            }
            82 => {
                // EXTCREATEFONTINDIRECTW: ihFont, LOGFONTW.
                let h = u32_at(r, 8);
                let font = Font {
                    height: i32_at(r, 12) as f64,
                    escapement: i32_at(r, 20) as f64,
                    bold: i32_at(r, 28) >= 600,
                    italic: u8_at(r, 32) != 0,
                    underline: u8_at(r, 33) != 0,
                    strikeout: u8_at(r, 34) != 0,
                    face: {
                        let f = utf16_at(r, 40, 32);
                        if f.is_empty() {
                            "Arial".into()
                        } else {
                            f
                        }
                    },
                };
                self.objects.insert(h, Obj::Font(font));
            }
            83 | 84 => {
                // EXTTEXTOUTA/W: bounds, mode, scales, EMRTEXT.
                let (x, y) = (i32_at(r, 36) as f64, i32_at(r, 40) as f64);
                let n = (u32_at(r, 44) as usize).min(100_000);
                let off_s = u32_at(r, 48) as usize;
                let text = if ty == 84 {
                    utf16_at(r, off_s, n)
                } else {
                    r.get(off_s..off_s.saturating_add(n))
                        .map(|s| s.iter().map(|b| *b as char).collect())
                        .unwrap_or_default()
                };
                self.text(x, y, text);
            }
            // Records with no effect on the geometry.
            1 | 13 | 15 | 16 | 20 | 21 | 23 | 26 | 28 | 29 | 31 | 32 | 41 | 48 | 49 | 50 | 51
            | 52 | 53 | 57 | 58 | 65 | 66 | 69 | 70 | 71 | 72 | 73 | 74 | 78 | 79 | 80 | 93
            | 94 | 96 | 97 | 98 | 99 | 100 | 101 | 102 | 103 | 104 | 105 | 106 | 107 | 108
            | 109 | 110 | 111 | 112 | 113 | 115 | 116 | 117 | 119 | 120 | 121 | 122 => {}
            118 => self.gradient_fill(r),
            56 | 92 => self.warn("PolyDraw record not supported"),
            other => self.warn(format!("EMF record {other} skipped")),
        }
    }

    /// A region (RGNDATA) as the union of its rectangles, device units.
    fn region_path(&self, r: &[u8], off: usize) -> Option<BezPath> {
        // RGNDATAHEADER: size(4) type(4) count(4) rgnSize(4) bounds(16).
        let count = (u32_at(r, off + 8) as usize).min(r.len() / 16);
        if count == 0 {
            return None;
        }
        let mut path = BezPath::new();
        for i in 0..count {
            let o = off + 32 + i * 16;
            if o + 16 > r.len() {
                break;
            }
            let (l, t, rr, bb) = (
                i32_at(r, o) as f64,
                i32_at(r, o + 4) as f64,
                i32_at(r, o + 8) as f64,
                i32_at(r, o + 12) as f64,
            );
            // Region rectangles are device pixels: bypass the map mode.
            let a = Point::new(
                (l - self.dev.left) * self.dev.sx,
                self.dev.page_h - (t - self.dev.top) * self.dev.sy,
            );
            let c = Point::new(
                (rr - self.dev.left) * self.dev.sx,
                self.dev.page_h - (bb - self.dev.top) * self.dev.sy,
            );
            path.extend(
                Rect::from_points(a, c)
                    .to_path(0.01)
                    .elements()
                    .iter()
                    .copied(),
            );
        }
        (!path.elements().is_empty()).then_some(path)
    }

    /// GRADIENTFILL: triangles or rectangles with per-vertex colours; each
    /// is drawn as a flat polygon in the average colour.
    fn gradient_fill(&mut self, r: &[u8]) {
        let nver = (u32_at(r, 24) as usize).min(r.len() / 16);
        let ntri = (u32_at(r, 28) as usize).min(r.len() / 8);
        let mode = u32_at(r, 32);
        let vert_off = 36;
        let vert = |i: usize| -> Option<((f64, f64), Color)> {
            let o = vert_off + i * 16;
            if o + 16 > r.len() || i >= nver {
                return None;
            }
            let c = Color::rgb8(
                (u16_at(r, o + 8) >> 8) as u8,
                (u16_at(r, o + 10) >> 8) as u8,
                (u16_at(r, o + 12) >> 8) as u8,
            );
            Some(((i32_at(r, o) as f64, i32_at(r, o + 4) as f64), c))
        };
        let idx_off = vert_off + nver * 16;
        let per = if mode == 2 { 3 } else { 2 };
        for t in 0..ntri {
            let o = idx_off + t * per * 4;
            let ids: Vec<usize> = (0..per).map(|k| u32_at(r, o + k * 4) as usize).collect();
            let vs: Vec<((f64, f64), Color)> = ids.iter().filter_map(|i| vert(*i)).collect();
            if vs.len() != per {
                continue;
            }
            let avg = {
                let n = vs.len() as f32;
                let mut acc = [0.0f32; 3];
                for (_, c) in &vs {
                    let [cr, cg, cb] = c.to_rgb8();
                    acc[0] += cr as f32;
                    acc[1] += cg as f32;
                    acc[2] += cb as f32;
                }
                Color::rgb8((acc[0] / n) as u8, (acc[1] / n) as u8, (acc[2] / n) as u8)
            };
            let saved = self.dc.brush.clone();
            let saved_pen = self.dc.pen.clone();
            self.dc.brush = Brush {
                fill: Fill::Solid(avg),
            };
            self.dc.pen = Pen {
                null: true,
                ..Pen::default()
            };
            if per == 3 {
                let pts: Vec<(f64, f64)> = vs.iter().map(|(p, _)| *p).collect();
                self.polygon(&pts);
            } else {
                let (a, b) = (vs[0].0, vs[1].0);
                self.rectangle(a.0, a.1, b.0, b.1, 0.0);
            }
            self.dc.brush = saved;
            self.dc.pen = saved_pen;
        }
    }
}

// ---------------------------------------------------------------- WMF

fn parse_wmf(b: &[u8], ids: &mut IdSource) -> Result<Imported, String> {
    let mut off = 0;
    // Placeable header: key, handle, bbox (4 x i16), inch, reserved, checksum.
    let placeable = if u32_at(b, 0) == 0x9AC6_CDD7 {
        let bbox = [i16_at(b, 6), i16_at(b, 8), i16_at(b, 10), i16_at(b, 12)];
        let inch = u16_at(b, 14).max(1) as f64;
        off = 22;
        Some((bbox, inch))
    } else {
        None
    };
    if off + 18 > b.len() {
        return Err("bad WMF header".into());
    }
    off += 18;
    // Without a placeable header the window extent (first records) gives the
    // size; scan ahead for it.
    let (page_w, page_h, dev) = match placeable {
        Some((bbox, inch)) => {
            let w_units = (bbox[2] as f64 - bbox[0] as f64).abs().max(1.0);
            let h_units = (bbox[3] as f64 - bbox[1] as f64).abs().max(1.0);
            let k = 25.4 / inch;
            let (pw, ph) = (w_units * k, h_units * k);
            (
                pw,
                ph,
                Device {
                    left: bbox[0] as f64,
                    top: bbox[1] as f64,
                    sx: k,
                    sy: k,
                    page_h: ph,
                    dpi: inch,
                },
            )
        }
        None => {
            let (mut ext, mut org) = ((0i16, 0i16), (0i16, 0i16));
            let mut o = off;
            let mut n = 0;
            while o + 6 <= b.len() && n < 64 {
                n += 1;
                let size = u32_at(b, o) as usize * 2;
                let fnc = u16_at(b, o + 4);
                if size < 6 {
                    break;
                }
                if fnc == 0x020C {
                    ext = (i16_at(b, o + 8), i16_at(b, o + 6));
                }
                if fnc == 0x020B {
                    org = (i16_at(b, o + 8), i16_at(b, o + 6));
                }
                o += size;
            }
            let (w, h) = ((ext.0 as f64).abs().max(1.0), (ext.1 as f64).abs().max(1.0));
            // Assume 96 dpi logical pixels.
            let k = 25.4 / 96.0;
            (
                w * k,
                h * k,
                Device {
                    left: org.0 as f64,
                    top: org.1 as f64,
                    sx: k,
                    sy: k,
                    page_h: h * k,
                    dpi: 96.0,
                },
            )
        }
    };
    let mut pl = Player::new(ids, dev);
    // WMF logical units map straight to the placeable units: the window
    // transform is applied by SETWINDOWORG/EXT when present.
    let mut n = 0;
    let mut wmf_objects: Vec<Option<Obj>> = Vec::new();
    while off + 6 <= b.len() && n < MAX_RECORDS {
        n += 1;
        let size = u32_at(b, off) as usize * 2;
        let fnc = u16_at(b, off + 4);
        if size < 6 || off + size > b.len() {
            break;
        }
        let r = &b[off..off + size];
        if fnc == 0 {
            break;
        }
        pl.wmf_record(fnc, r, &mut wmf_objects);
        off += size;
    }
    Ok(Imported {
        shapes: pl.shapes,
        size: Size::new(page_w, page_h),
        warnings: pl.warnings,
    })
}

impl Player<'_> {
    fn wmf_add_object(&mut self, objs: &mut Vec<Option<Obj>>, o: Obj) {
        // The lowest free slot is reused, as GDI does.
        let idx = match objs.iter().position(|x| x.is_none()) {
            Some(i) => {
                objs[i] = Some(o.clone());
                i
            }
            None => {
                objs.push(Some(o.clone()));
                objs.len() - 1
            }
        };
        self.objects.insert(idx as u32, o);
    }

    fn wmf_record(&mut self, fnc: u16, r: &[u8], objs: &mut Vec<Option<Obj>>) {
        let p = 6; // parameters start after size(4) and function(2)
        let w = |i: usize| i16_at(r, p + i * 2) as f64;
        match fnc {
            0x0103 => self.dc.map_mode = u16_at(r, p) as u32,
            0x020B => self.dc.window_org = (w(1), w(0)),
            0x020C => self.dc.window_ext = (w(1), w(0)),
            0x020D => self.dc.viewport_org = (w(1), w(0)),
            0x020E => self.dc.viewport_ext = (w(1), w(0)),
            0x0102 => self.dc.bk_mode = u16_at(r, p) as u32,
            0x0106 => self.dc.polyfill_even_odd = u16_at(r, p) != 2,
            0x0209 => self.dc.text_color = colorref(u32_at(r, p)),
            0x0201 => self.dc.bk_color = colorref(u32_at(r, p)),
            0x012E => self.dc.text_align = u16_at(r, p) as u32,
            0x001E => {
                if self.saved.len() < 256 {
                    self.saved.push(self.dc.clone());
                }
            }
            0x0127 => {
                if let Some(dc) = self.saved.pop() {
                    self.dc = dc;
                }
            }
            0x012D => self.select_object(u16_at(r, p) as u32),
            0x01F0 => {
                let i = u16_at(r, p) as usize;
                if i < objs.len() {
                    objs[i] = None;
                }
                self.objects.remove(&(i as u32));
            }
            0x02FA => {
                // CREATEPENINDIRECT: style, width (x, y), colour.
                let style = u16_at(r, p) as u32;
                let pen = Pen {
                    null: style & 0xf == 5,
                    style,
                    width: w(1),
                    color: colorref(u32_at(r, p + 6)),
                    ..Pen::default()
                };
                self.wmf_add_object(objs, Obj::Pen(pen));
            }
            0x02FC => {
                // CREATEBRUSHINDIRECT: style, colour, hatch.
                let style = u16_at(r, p);
                let color = colorref(u32_at(r, p + 2));
                let fill = if style == 1 {
                    Fill::None
                } else {
                    Fill::Solid(color)
                };
                self.wmf_add_object(objs, Obj::Brush(Brush { fill }));
            }
            0x02FB => {
                // CREATEFONTINDIRECT: LOGFONT with 8-bit face name.
                let face: String = r
                    .get(p + 18..)
                    .map(|s| {
                        s.iter()
                            .take(32)
                            .take_while(|b| **b != 0)
                            .map(|b| *b as char)
                            .collect()
                    })
                    .unwrap_or_default();
                let font = Font {
                    height: w(0),
                    escapement: w(2),
                    bold: w(4) >= 600.0,
                    italic: u8_at(r, p + 10) != 0,
                    underline: u8_at(r, p + 11) != 0,
                    strikeout: u8_at(r, p + 12) != 0,
                    face: if face.is_empty() {
                        "Arial".into()
                    } else {
                        face
                    },
                };
                self.wmf_add_object(objs, Obj::Font(font));
            }
            0x06FF | 0x0142 | 0x01F9 | 0x00F7 | 0x0037 | 0x0234 | 0x0139 => {
                // Region, DIB brush, palette and other objects: keep the
                // handle numbering in step.
                self.wmf_add_object(objs, Obj::Other);
            }
            0x0214 => self.move_to(w(1), w(0)),
            0x0213 => self.line_to(w(1), w(0)),
            0x0324 | 0x0325 => {
                let n = (u16_at(r, p) as usize).min(MAX_POINTS);
                let pts: Vec<(f64, f64)> = (0..n)
                    .filter(|i| p + 2 + i * 4 + 4 <= r.len())
                    .map(|i| {
                        (
                            i16_at(r, p + 2 + i * 4) as f64,
                            i16_at(r, p + 4 + i * 4) as f64,
                        )
                    })
                    .collect();
                self.points += pts.len();
                if fnc == 0x0324 {
                    self.polygon(&pts);
                } else {
                    self.polyline(&pts, false);
                }
            }
            0x0538 => {
                let npolys = (u16_at(r, p) as usize).min(100_000);
                let counts: Vec<usize> = (0..npolys)
                    .map(|i| u16_at(r, p + 2 + i * 2) as usize)
                    .collect();
                let mut o = p + 2 + npolys * 2;
                let mut polys = Vec::new();
                for c in counts {
                    let pts: Vec<(f64, f64)> = (0..c)
                        .filter(|i| o + i * 4 + 4 <= r.len())
                        .map(|i| (i16_at(r, o + i * 4) as f64, i16_at(r, o + 2 + i * 4) as f64))
                        .collect();
                    o += c * 4;
                    self.points += pts.len();
                    polys.push(pts);
                }
                self.polypolygon(&polys);
            }
            0x041B => self.rectangle(w(3), w(2), w(1), w(0), 0.0),
            0x061C => {
                let radius = (w(0).min(w(1)) / 2.0).abs() * self.scale();
                self.rectangle(w(5), w(4), w(3), w(2), radius);
            }
            0x0418 => self.ellipse(w(3), w(2), w(1), w(0)),
            0x0817 | 0x0830 | 0x081A => {
                // ARC / CHORD / PIE: yEndArc xEndArc yStartArc xStartArc bottom right top left.
                let kind = match fnc {
                    0x0830 => 1,
                    0x081A => 2,
                    _ => 0,
                };
                self.arc(w(7), w(6), w(5), w(4), (w(3), w(2)), (w(1), w(0)), kind);
            }
            0x0521 => {
                // TEXTOUT: count, string, y, x.
                let n = u16_at(r, p) as usize;
                let text: String = r
                    .get(p + 2..p + 2 + n)
                    .map(|s| s.iter().map(|b| *b as char).collect())
                    .unwrap_or_default();
                let o = p + 2 + n.div_ceil(2) * 2;
                let (y, x) = (i16_at(r, o) as f64, i16_at(r, o + 2) as f64);
                self.text(x, y, text);
            }
            0x0A32 => {
                // EXTTEXTOUT: y, x, count, options, [rect], string.
                let (y, x) = (w(0), w(1));
                let n = u16_at(r, p + 4) as usize;
                let opts = u16_at(r, p + 6);
                let mut o = p + 8;
                if opts & 0x6 != 0 {
                    o += 8;
                }
                let text: String = r
                    .get(o..o + n)
                    .map(|s| s.iter().map(|b| *b as char).collect())
                    .unwrap_or_default();
                self.text(x, y, text);
            }
            0x0F43 | 0x0B41 | 0x0940 => {
                // STRETCHDIB: rop(4) usage(2) srcH srcW srcY srcX dstH dstW dstY dstX, DIB.
                // DIBSTRETCHBLT: rop(4) srcH srcW srcY srcX dstH dstW dstY dstX, DIB.
                // DIBBITBLT: rop(4) srcY srcX h w dstY dstX, DIB.
                let (o, nparams) = match fnc {
                    0x0F43 => (p + 6, 8),
                    0x0B41 => (p + 4, 8),
                    _ => (p + 4, 6),
                };
                let v = |i: usize| i16_at(r, o + i * 2) as f64;
                let (dst_h, dst_w, dst_y, dst_x) = if nparams == 8 {
                    (v(4), v(5), v(6), v(7))
                } else {
                    (v(2), v(3), v(4), v(5))
                };
                let dib_off = o + nparams * 2;
                if let Some(dib) = r.get(dib_off..) {
                    let hdr = u32_at(dib, 0) as usize;
                    if hdr >= 40 && hdr < dib.len() {
                        let bpp = u16_at(dib, 14) as usize;
                        let clr_used = u32_at(dib, 32) as usize;
                        let pal = if bpp <= 8 {
                            (if clr_used > 0 { clr_used } else { 1 << bpp }) * 4
                        } else if u32_at(dib, 16) == 3 {
                            12
                        } else {
                            0
                        };
                        let bits_off = (hdr + pal).min(dib.len());
                        let (bmi, bits) = dib.split_at(bits_off);
                        self.dib(bmi, bits, dst_x, dst_y, dst_w, dst_h);
                    }
                }
            }
            0x0626 | 0x0104 | 0x0107 | 0x0416 | 0x0105 | 0x0108 | 0x0109 | 0x0149 | 0x0415
            | 0x0410 | 0x0419 | 0x0412 | 0x0033 | 0x0034 | 0x0035 | 0x0036 | 0x0052 | 0x0055
            | 0x0053 | 0x0054 | 0x0100 | 0x0101 | 0x0220 | 0x0231 | 0x02FF | 0x0B23 | 0x0D33
            | 0x0922 | 0x012A | 0x012B | 0x012C | 0x0126 | 0x0129 | 0x01E0 | 0x01B0 | 0x0211
            | 0x0210 | 0x020A | 0x0203 | 0x0204 | 0x0206 | 0x0207 | 0x0208 => {}
            other => self.warn(format!("WMF record 0x{other:04x} skipped")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le32(v: &mut Vec<u8>, x: i32) {
        v.extend_from_slice(&x.to_le_bytes());
    }
    fn le16(v: &mut Vec<u8>, x: i16) {
        v.extend_from_slice(&x.to_le_bytes());
    }

    fn rec(ty: u32, body: &[i32]) -> Vec<u8> {
        let mut r = Vec::new();
        le32(&mut r, ty as i32);
        le32(&mut r, (8 + body.len() * 4) as i32);
        for b in body {
            le32(&mut r, *b);
        }
        r
    }

    /// An EMF of 100 x 50 mm whose device space is 1000 x 500 px.
    fn emf_file(records: &[Vec<u8>]) -> Vec<u8> {
        let mut h = Vec::new();
        le32(&mut h, 1);
        le32(&mut h, 108);
        for v in [0, 0, 999, 499] {
            le32(&mut h, v);
        }
        for v in [0, 0, 10_000, 5_000] {
            le32(&mut h, v);
        }
        h.extend_from_slice(b" EMF");
        le32(&mut h, 0x10000);
        le32(&mut h, 0); // bytes, patched below
        le32(&mut h, records.len() as i32 + 2);
        le16(&mut h, 16);
        le16(&mut h, 0);
        le32(&mut h, 0);
        le32(&mut h, 0);
        le32(&mut h, 0);
        le32(&mut h, 1000);
        le32(&mut h, 500);
        le32(&mut h, 100);
        le32(&mut h, 50);
        while h.len() < 108 {
            h.push(0);
        }
        let mut f = h;
        for r in records {
            f.extend_from_slice(r);
        }
        f.extend_from_slice(&rec(14, &[0, 0, 0]));
        let n = f.len() as i32;
        f[48..52].copy_from_slice(&n.to_le_bytes());
        f
    }

    #[test]
    fn emf_rectangle_polygon_text_and_bitmap() {
        let mut ids = IdSource::default();
        // Pen 1: solid, 20 px wide, red. Brush 2: solid blue.
        let pen = rec(38, &[1, 0, 20, 0, 0x0000ff]);
        let brush = rec(39, &[2, 0, 0xff0000, 0]);
        let sel_pen = rec(37, &[1]);
        let sel_brush = rec(37, &[2]);
        // Rectangle from (100, 100) to (300, 200) px.
        let rect = rec(43, &[100, 100, 300, 200]);
        // 16-bit triangle.
        let mut tri = Vec::new();
        le32(&mut tri, 86);
        le32(&mut tri, 8 + 16 + 4 + 12);
        for v in [0, 0, 0, 0] {
            le32(&mut tri, v);
        }
        le32(&mut tri, 3);
        for (x, y) in [(500i16, 400i16), (600, 400), (550, 300)] {
            le16(&mut tri, x);
            le16(&mut tri, y);
        }
        // Font 3: 40 px, bold; select; text colour; text at (50, 450).
        let mut font = Vec::new();
        le32(&mut font, 82);
        le32(&mut font, 8 + 4 + 92);
        le32(&mut font, 3);
        le32(&mut font, -40);
        le32(&mut font, 0);
        le32(&mut font, 0);
        le32(&mut font, 0);
        le32(&mut font, 700);
        font.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
        for ch in "Arial".encode_utf16() {
            le16(&mut font, ch as i16);
        }
        while font.len() < 8 + 4 + 92 {
            font.push(0);
        }
        let sel_font = rec(37, &[3]);
        let text_color = rec(24, &[0x00ff00]);
        let mut text = Vec::new();
        le32(&mut text, 84);
        let s: Vec<u16> = "Hi".encode_utf16().collect();
        let size = 8 + 16 + 4 + 8 + 8 + 4 + 4 + 4 + 16 + 4 + s.len() * 2;
        le32(&mut text, size as i32);
        for v in [0, 0, 0, 0] {
            le32(&mut text, v);
        }
        le32(&mut text, 1);
        le32(&mut text, 0);
        le32(&mut text, 0);
        le32(&mut text, 50);
        le32(&mut text, 450);
        le32(&mut text, s.len() as i32);
        le32(&mut text, 76);
        le32(&mut text, 0);
        for v in [0, 0, 0, 0] {
            le32(&mut text, v);
        }
        le32(&mut text, 0);
        for ch in &s {
            le16(&mut text, *ch as i16);
        }
        // 2 x 2 24-bit DIB at (800, 50), 100 x 100 px.
        let mut dib = Vec::new();
        le32(&mut dib, 81);
        let bmi_len = 40;
        let bits_len = 16; // two rows of 8 bytes (6 used + 2 pad)
        le32(&mut dib, (80 + bmi_len + bits_len) as i32);
        for v in [0, 0, 0, 0] {
            le32(&mut dib, v);
        }
        for v in [800, 50, 0, 0, 2, 2] {
            le32(&mut dib, v);
        }
        le32(&mut dib, 80);
        le32(&mut dib, bmi_len as i32);
        le32(&mut dib, (80 + bmi_len) as i32);
        le32(&mut dib, bits_len as i32);
        le32(&mut dib, 0);
        le32(&mut dib, 0x00CC0020);
        le32(&mut dib, 100);
        le32(&mut dib, 100);
        assert_eq!(dib.len(), 80);
        le32(&mut dib, 40);
        le32(&mut dib, 2);
        le32(&mut dib, 2);
        le16(&mut dib, 1);
        le16(&mut dib, 24);
        for v in [0, 16, 0, 0, 0, 0] {
            le32(&mut dib, v);
        }
        // Bottom row first: blue, green; top row: red, white (BGR order).
        dib.extend_from_slice(&[255, 0, 0, 0, 255, 0, 0, 0]);
        dib.extend_from_slice(&[0, 0, 255, 255, 255, 255, 0, 0]);
        let file = emf_file(&[
            pen, brush, sel_pen, sel_brush, rect, tri, font, sel_font, text_color, text, dib,
        ]);
        assert!(is_emf(&file));
        let imp = parse(&file, &mut ids).expect("parse");
        assert!((imp.size.width - 100.0).abs() < 1e-6 && (imp.size.height - 50.0).abs() < 1e-6);
        assert_eq!(imp.shapes.len(), 4, "{:?}", imp.warnings);
        // Rectangle: 10 x 10 mm from x 10, top at 40 mm (y up).
        let r = &imp.shapes[0];
        let b = r.bounds();
        assert!(
            (b.x0 - 10.0).abs() < 1e-6 && (b.x1 - 30.0).abs() < 1e-6,
            "{b:?}"
        );
        assert!(
            (b.y0 - 30.0).abs() < 1e-6 && (b.y1 - 40.0).abs() < 1e-6,
            "{b:?}"
        );
        assert_eq!(r.fill, Fill::Solid(Color::rgb8(0, 0, 255)));
        let st = r.stroke.as_ref().expect("stroke");
        assert_eq!(st.color, Color::rgb8(255, 0, 0));
        assert!((st.width - 2.0).abs() < 1e-6, "{}", st.width);
        // Triangle.
        let t = &imp.shapes[1];
        assert!(matches!(t.kind, ShapeKind::Path { closed: true, .. }));
        let tb = t.bounds();
        assert!(
            (tb.x0 - 50.0).abs() < 1e-6 && (tb.y1 - 20.0).abs() < 1e-6,
            "{tb:?}"
        );
        // Text: 40 px = 4 mm em, bold, green, baseline 0.8 em below the top.
        let x = &imp.shapes[2];
        match &x.kind {
            ShapeKind::Text { spans, .. } => {
                assert_eq!(spans[0].text, "Hi");
                assert!(spans[0].bold);
                assert!((spans[0].size_pt - 4.0 / PT_MM).abs() < 1e-6);
            }
            k => panic!("{k:?}"),
        }
        assert_eq!(x.fill, Fill::Solid(Color::rgb8(0, 255, 0)));
        let o = x.transform * Point::ZERO;
        assert!(
            (o.x - 5.0).abs() < 1e-6 && (o.y - (50.0 - 45.0 - 3.2)).abs() < 1e-6,
            "{o:?}"
        );
        // Bitmap: 10 x 10 mm at x 80, top 45 mm; pixels decoded in order.
        let bm = &imp.shapes[3];
        match &bm.kind {
            ShapeKind::Bitmap {
                width_px,
                height_px,
                png,
                ..
            } => {
                assert_eq!((*width_px, *height_px), (2, 2));
                let pm = tiny_skia::Pixmap::decode_png(png).expect("png");
                let px = pm.pixels();
                assert_eq!((px[0].red(), px[0].green(), px[0].blue()), (255, 0, 0));
                assert_eq!((px[1].red(), px[1].green(), px[1].blue()), (255, 255, 255));
                assert_eq!((px[2].red(), px[2].green(), px[2].blue()), (0, 0, 255));
                assert_eq!((px[3].red(), px[3].green(), px[3].blue()), (0, 255, 0));
            }
            k => panic!("{k:?}"),
        }
        let bb = bm.bounds();
        assert!(
            (bb.x0 - 80.0).abs() < 1e-6 && (bb.y1 - 45.0).abs() < 1e-6,
            "{bb:?}"
        );
    }

    #[test]
    fn emf_path_with_clip_becomes_a_clip_frame() {
        let mut ids = IdSource::default();
        let brush = rec(39, &[1, 0, 0x0000ff, 0]);
        let sel = rec(37, &[1]);
        let null_pen = rec(37, &[0x8000_0008u32 as i32]);
        // Clip path: rectangle (0,0)-(500,500) via BEGINPATH .. SELECTCLIPPATH.
        let begin = rec(59, &[]);
        let r1 = rec(43, &[0, 0, 500, 500]);
        let end = rec(60, &[]);
        let clip = rec(67, &[5]);
        // A rectangle crossing the clip edge.
        let r2 = rec(43, &[250, 250, 750, 400]);
        let file = emf_file(&[brush, sel, null_pen, begin, r1, end, clip, r2]);
        let imp = parse(&file, &mut ids).expect("parse");
        assert_eq!(imp.shapes.len(), 1);
        match &imp.shapes[0].kind {
            ShapeKind::ClipFrame { frame, contents } => {
                let fb = frame.bounds();
                assert!((fb.width() - 50.0).abs() < 1e-6 && (fb.height() - 50.0).abs() < 1e-6);
                assert_eq!(contents.len(), 1);
                assert_eq!(contents[0].fill, Fill::Solid(Color::rgb8(255, 0, 0)));
            }
            k => panic!("{k:?}"),
        }
    }

    fn wmf_rec(func: u16, params: &[i16]) -> Vec<u8> {
        let mut r = Vec::new();
        le32(&mut r, 3 + params.len() as i32);
        le16(&mut r, func as i16);
        for p in params {
            le16(&mut r, *p);
        }
        r
    }

    #[test]
    fn placeable_wmf_rectangle_and_text() {
        let mut ids = IdSource::default();
        let mut f = Vec::new();
        // Placeable header: 0..1440 x 0..720 at 1440 units per inch (1 x 0.5 inch).
        le32(&mut f, 0x9AC6_CDD7u32 as i32);
        le16(&mut f, 0);
        for v in [0i16, 0, 1440, 720] {
            le16(&mut f, v);
        }
        le16(&mut f, 1440);
        le32(&mut f, 0);
        le16(&mut f, 0);
        // Standard header: type 1, size 9 words, version 0x300, size, objects, max record, members.
        le16(&mut f, 1);
        le16(&mut f, 9);
        le16(&mut f, 0x300);
        le32(&mut f, 0);
        le16(&mut f, 2);
        le32(&mut f, 0);
        le16(&mut f, 0);
        // Brush (object 0): solid green. Pen (object 1): null.
        f.extend_from_slice(&wmf_rec(0x02FC, &[0, 0x8000u16 as i16 | 0x00, 0, 0]));
        f.extend_from_slice(&wmf_rec(0x02FA, &[5, 0, 0, 0, 0]));
        f.extend_from_slice(&wmf_rec(0x012D, &[0]));
        f.extend_from_slice(&wmf_rec(0x012D, &[1]));
        // Rectangle: bottom, right, top, left = 360, 720, 0, 0 (half inch square).
        f.extend_from_slice(&wmf_rec(0x041B, &[360, 720, 0, 0]));
        // Text "ok" at y 500, x 100.
        let mut t = wmf_rec(0x0521, &[2]);
        t.extend_from_slice(b"ok");
        le16(&mut t, 500);
        le16(&mut t, 100);
        let words = (t.len() / 2) as i32;
        t[0..4].copy_from_slice(&words.to_le_bytes());
        f.extend_from_slice(&t);
        f.extend_from_slice(&wmf_rec(0, &[]));
        assert!(is_wmf(&f));
        let imp = parse(&f, &mut ids).expect("parse");
        assert!((imp.size.width - 25.4).abs() < 1e-6 && (imp.size.height - 12.7).abs() < 1e-6);
        assert_eq!(imp.shapes.len(), 2, "{:?}", imp.warnings);
        let b = imp.shapes[0].bounds();
        assert!(
            (b.width() - 12.7).abs() < 1e-6 && (b.height() - 6.35).abs() < 1e-6,
            "{b:?}"
        );
        assert!((b.y1 - 12.7).abs() < 1e-6, "{b:?}");
        assert_eq!(imp.shapes[0].fill, Fill::Solid(Color::rgb8(0, 128, 0)));
        assert!(imp.shapes[0].stroke.is_none());
        match &imp.shapes[1].kind {
            ShapeKind::Text { spans, .. } => assert_eq!(spans[0].text, "ok"),
            k => panic!("{k:?}"),
        }
    }

    #[test]
    fn garbage_is_rejected_and_truncation_does_not_panic() {
        let mut ids = IdSource::default();
        assert!(parse(b"not a metafile at all", &mut ids).is_err());
        let file = emf_file(&[rec(43, &[0, 0, 10, 10])]);
        for cut in (0..file.len()).step_by(7) {
            let _ = parse(&file[..cut], &mut ids);
        }
    }
}

// ---------------------------------------------------------------- EMF export

/// Write one page as an enhanced metafile. One logical unit is 0.01 mm
/// (MM_TEXT on a 2540 dpi device), paths are written as GDI paths with
/// geometric pens and solid brushes, fountain fills as clipped bands,
/// text as EXTTEXTOUTW with a LOGFONT, bitmaps as 32-bit DIBs, and
/// ClipFrames as clip paths. Live effects are expanded first.
pub fn page_to_emf(doc: &tracedraw_core::Document, page_index: usize) -> Vec<u8> {
    let Some(page) = doc.pages.get(page_index) else {
        return Vec::new();
    };
    let (w, h) = (page.size.width.max(0.01), page.size.height.max(0.01));
    let mut wr = EmfWriter {
        out: Vec::new(),
        records: 0,
        next_handle: 1,
        max_handle: 1,
        page_h: h,
        symbols: &doc.symbols,
    };
    // Header, patched at the end.
    wr.out.resize(108, 0);
    wr.records = 1;
    wr.record(18, &[1]); // SETBKMODE TRANSPARENT
    wr.record(16, &[2]); // SETGRAPHICSMODE GM_ADVANCED
    for layer in &page.layers {
        if !layer.visible || !layer.printable {
            continue;
        }
        for s in &layer.shapes {
            wr.shape(s, Affine::IDENTITY);
        }
    }
    // EOF: nPalEntries, offPalEntries, nSizeLast.
    wr.record(14, &[0, 16, 20]);
    let n = wr.out.len() as i32;
    let wl = (w * 100.0).round() as i32;
    let hl = (h * 100.0).round() as i32;
    let mut hdr = Vec::new();
    push_i32(&mut hdr, 1);
    push_i32(&mut hdr, 108);
    for v in [0, 0, wl - 1, hl - 1] {
        push_i32(&mut hdr, v);
    }
    for v in [0, 0, wl, hl] {
        push_i32(&mut hdr, v);
    }
    hdr.extend_from_slice(b" EMF");
    push_i32(&mut hdr, 0x10000);
    push_i32(&mut hdr, n);
    push_i32(&mut hdr, wr.records as i32);
    hdr.extend_from_slice(&(wr.max_handle as u16 + 1).to_le_bytes());
    hdr.extend_from_slice(&0u16.to_le_bytes());
    push_i32(&mut hdr, 0); // nDescription
    push_i32(&mut hdr, 0); // offDescription
    push_i32(&mut hdr, 0); // nPalEntries
    push_i32(&mut hdr, wl); // device px
    push_i32(&mut hdr, hl);
    push_i32(&mut hdr, w.round().max(1.0) as i32); // device mm
    push_i32(&mut hdr, h.round().max(1.0) as i32);
    push_i32(&mut hdr, 0); // cbPixelFormat
    push_i32(&mut hdr, 0); // offPixelFormat
    push_i32(&mut hdr, 0); // bOpenGL
    push_i32(&mut hdr, (w * 1000.0).round() as i32); // micrometers
    push_i32(&mut hdr, (h * 1000.0).round() as i32);
    hdr.resize(108, 0);
    wr.out[..108].copy_from_slice(&hdr);
    wr.out
}

fn push_i32(v: &mut Vec<u8>, x: i32) {
    v.extend_from_slice(&x.to_le_bytes());
}

struct EmfWriter<'a> {
    out: Vec<u8>,
    records: usize,
    next_handle: u32,
    max_handle: u32,
    page_h: f64,
    symbols: &'a [tracedraw_core::document::Symbol],
}

impl EmfWriter<'_> {
    fn record(&mut self, ty: u32, body: &[i32]) {
        push_i32(&mut self.out, ty as i32);
        push_i32(&mut self.out, (8 + body.len() * 4) as i32);
        for b in body {
            push_i32(&mut self.out, *b);
        }
        self.records += 1;
    }

    fn record_bytes(&mut self, ty: u32, body: &[u8]) {
        let pad = (4 - body.len() % 4) % 4;
        push_i32(&mut self.out, ty as i32);
        push_i32(&mut self.out, (8 + body.len() + pad) as i32);
        self.out.extend_from_slice(body);
        self.out.extend(std::iter::repeat_n(0u8, pad));
        self.records += 1;
    }

    fn lg(&self, p: Point) -> (i32, i32) {
        (
            (p.x * 100.0).round().clamp(-1e9, 1e9) as i32,
            ((self.page_h - p.y) * 100.0).round().clamp(-1e9, 1e9) as i32,
        )
    }

    fn handle(&mut self) -> u32 {
        let h = self.next_handle;
        self.next_handle += 1;
        self.max_handle = self.max_handle.max(h);
        h
    }

    fn colorref(c: Color) -> i32 {
        let [r, g, b] = c.to_rgb8();
        (r as i32) | ((g as i32) << 8) | ((b as i32) << 16)
    }

    /// A geometric pen for the stroke; `None` for no outline.
    fn pen(&mut self, stroke: Option<&Stroke>, scale: f64) -> Option<u32> {
        let s = stroke?;
        let h = self.handle();
        let w = if s.scale_with_object {
            s.width * scale
        } else {
            s.width
        };
        if w <= Stroke::HAIRLINE + 1e-9 {
            // Cosmetic one-pixel pen.
            self.record(38, &[h as i32, 0, 0, 0, Self::colorref(s.color)]);
            return Some(h);
        }
        let cap = match s.cap {
            LineCap::Round => 0x000,
            LineCap::Square => 0x100,
            LineCap::Butt => 0x200,
        };
        let join = match s.join {
            LineJoin::Round => 0x0000,
            LineJoin::Bevel => 0x1000,
            LineJoin::Miter => 0x2000,
        };
        let dashed = !s.dash.is_empty();
        let style = 0x10000 | cap | join | if dashed { 7 } else { 0 };
        let wl = (w * 100.0).round().max(1.0) as i32;
        let mut body = vec![
            h as i32,
            0,
            0,
            0,
            0,
            style,
            wl,
            0,
            Self::colorref(s.color),
            0,
        ];
        if dashed {
            let entries: Vec<i32> = s
                .dash
                .iter()
                .map(|d| (d * w * 100.0).round().max(1.0) as i32)
                .collect();
            body.push(entries.len() as i32);
            body.extend(entries);
        } else {
            body.push(0);
        }
        self.record(95, &body);
        Some(h)
    }

    fn brush(&mut self, color: Color) -> u32 {
        let h = self.handle();
        self.record(39, &[h as i32, 0, Self::colorref(color), 0]);
        h
    }

    fn select(&mut self, h: u32) {
        self.record(37, &[h as i32]);
    }

    fn delete(&mut self, h: u32) {
        self.record(40, &[h as i32]);
    }

    /// BEGINPATH .. ENDPATH for a page-space path.
    fn path(&mut self, path: &BezPath) {
        self.record(59, &[]);
        let mut cur = None;
        for el in path.elements() {
            match el {
                PathEl::MoveTo(p) => {
                    let (x, y) = self.lg(*p);
                    self.record(27, &[x, y]);
                    cur = Some(*p);
                }
                PathEl::LineTo(p) => {
                    let (x, y) = self.lg(*p);
                    self.record(54, &[x, y]);
                    cur = Some(*p);
                }
                PathEl::QuadTo(c, p) => {
                    let s = cur.unwrap_or(*c);
                    let c1 = s + (*c - s) * (2.0 / 3.0);
                    let c2 = *p + (*c - *p) * (2.0 / 3.0);
                    self.bezier_to(&[c1, c2, *p]);
                    cur = Some(*p);
                }
                PathEl::CurveTo(a, b, p) => {
                    self.bezier_to(&[*a, *b, *p]);
                    cur = Some(*p);
                }
                PathEl::ClosePath => self.record(61, &[]),
            }
        }
        self.record(60, &[]);
    }

    fn bezier_to(&mut self, pts: &[Point; 3]) {
        let mut body = vec![0, 0, -1, -1, 3];
        for p in pts {
            let (x, y) = self.lg(*p);
            body.push(x);
            body.push(y);
        }
        self.record(5, &body);
    }

    fn fill_and_stroke(
        &mut self,
        path: &BezPath,
        fill: &Fill,
        stroke: Option<&Stroke>,
        scale: f64,
        even_odd: bool,
    ) {
        if path.elements().is_empty() {
            return;
        }
        self.record(19, &[if even_odd { 1 } else { 2 }]);
        let pen = self.pen(stroke, scale);
        match fill {
            Fill::None => {}
            Fill::Solid(c) => {
                let b = self.brush(*c);
                self.select(b);
                self.path(path);
                self.record(62, &[]);
                self.delete(b);
            }
            Fill::Fountain(f) => self.fountain(path, f),
            other => {
                let c = other.preview_color().unwrap_or(Color::Gray { v: 0.5 });
                let b = self.brush(c);
                self.select(b);
                self.path(path);
                self.record(62, &[]);
                self.delete(b);
            }
        }
        if let Some(p) = pen {
            self.select(p);
            self.path(path);
            self.record(64, &[]);
            self.select(0x8000_0008u32);
            self.delete(p);
        }
    }

    /// Linear and radial fountains as 64 clipped bands; the others as
    /// their average colour.
    fn fountain(&mut self, path: &BezPath, f: &tracedraw_core::Fountain) {
        let b = path.bounding_box();
        if b.width() <= 0.0 || b.height() <= 0.0 {
            return;
        }
        self.record(33, &[]); // SAVEDC
        self.path(path);
        self.record(67, &[5]); // SELECTCLIPPATH RGN_COPY
        let n = 64;
        match f.kind {
            tracedraw_core::style::FountainKind::Linear => {
                let a = f.angle.to_radians();
                let (ca, sa) = (a.cos(), a.sin());
                let c = b.center();
                let half = (b.width() * ca.abs() + b.height() * sa.abs()) / 2.0;
                let along = Vec2::new(ca, sa);
                let across = Vec2::new(-sa, ca);
                let diag = b.width().hypot(b.height());
                for i in 0..n {
                    let t0 = i as f64 / n as f64;
                    let t1 = (i + 1) as f64 / n as f64;
                    let color = f.color_at((t0 + t1) / 2.0);
                    let s0 = -half + 2.0 * half * t0;
                    let s1 = -half + 2.0 * half * t1;
                    let (s0, s1) = if i == 0 {
                        (s0 - diag, s1)
                    } else if i == n - 1 {
                        (s0, s1 + diag)
                    } else {
                        (s0, s1)
                    };
                    let quad: Vec<Point> = [
                        c + along * s0 - across * diag,
                        c + along * s1 - across * diag,
                        c + along * s1 + across * diag,
                        c + along * s0 + across * diag,
                    ]
                    .to_vec();
                    let br = self.brush(color);
                    self.select(br);
                    let mut body = vec![0, 0, -1, -1, 4];
                    for p in &quad {
                        let (x, y) = self.lg(*p);
                        body.push(x);
                        body.push(y);
                    }
                    self.select(0x8000_0008u32);
                    self.record(3, &body);
                    self.delete(br);
                }
            }
            _ => {
                // Radial, conical, square: concentric ellipses from the outside in.
                let c = Point::new(
                    b.center().x + f.offset.x * b.width() / 2.0,
                    b.center().y + f.offset.y * b.height() / 2.0,
                );
                let r = (b.width().max(b.height()) / 2.0) * std::f64::consts::SQRT_2;
                self.select(0x8000_0008u32);
                for i in (0..n).rev() {
                    let t = (i as f64 + 0.5) / n as f64;
                    let rr = r * (i + 1) as f64 / n as f64;
                    let color = f.color_at(t);
                    let br = self.brush(color);
                    self.select(br);
                    let (x0, y0) = self.lg(Point::new(c.x - rr, c.y + rr));
                    let (x1, y1) = self.lg(Point::new(c.x + rr, c.y - rr));
                    self.record(42, &[x0, y0, x1, y1]);
                    self.delete(br);
                }
            }
        }
        self.record(34, &[-1]); // RESTOREDC
    }

    fn text(
        &mut self,
        shape: &Shape,
        transform: Affine,
        spans: &[TextSpan],
        origin: Point,
        align: TextAlign,
    ) {
        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
        let Some(first) = spans.first() else {
            return;
        };
        let c = transform.as_coeffs();
        let scale = (c[0] * c[3] - c[1] * c[2]).abs().sqrt();
        let rot_deg = c[1].atan2(c[0]).to_degrees();
        let em_mm = first.size_pt * PT_MM * scale;
        let color = match &shape.fill {
            Fill::Solid(col) => *col,
            Fill::Fountain(f) => f.first_color(),
            _ => Color::BLACK,
        };
        // Font.
        let h = self.handle();
        let mut body = Vec::new();
        push_i32(&mut body, h as i32);
        push_i32(&mut body, -((em_mm * 100.0).round() as i32));
        push_i32(&mut body, 0);
        push_i32(&mut body, (rot_deg * 10.0).round() as i32);
        push_i32(&mut body, (rot_deg * 10.0).round() as i32);
        push_i32(&mut body, if first.bold { 700 } else { 400 });
        body.push(first.italic as u8);
        body.push(first.underline as u8);
        body.push(first.strikethrough as u8);
        body.extend_from_slice(&[1, 0, 0, 0, 0]);
        let mut face: Vec<u16> = first.font_family.encode_utf16().take(31).collect();
        face.resize(32, 0);
        for u in face {
            body.extend_from_slice(&u.to_le_bytes());
        }
        // Full LOGFONTW is 92 bytes; the extended fields may be omitted
        // when the record is sized accordingly.
        body.resize(4 + 92, 0);
        self.record_bytes(82, &body);
        self.select(h);
        self.record(24, &[Self::colorref(color)]);
        let ta = 24
            | match align {
                TextAlign::Center => 6,
                TextAlign::Right => 2,
                _ => 0,
            };
        self.record(22, &[ta]);
        let line_h = em_mm * 1.2;
        for (i, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let local = Point::new(origin.x, origin.y - i as f64 * line_h / scale.max(1e-9));
            let p = transform * local;
            let (x, y) = self.lg(p);
            let units: Vec<u16> = line.encode_utf16().collect();
            let mut body = Vec::new();
            for v in [0, 0, -1, -1] {
                push_i32(&mut body, v);
            }
            push_i32(&mut body, 2); // GM_ADVANCED
            body.extend_from_slice(&1.0f32.to_le_bytes());
            body.extend_from_slice(&1.0f32.to_le_bytes());
            push_i32(&mut body, x);
            push_i32(&mut body, y);
            push_i32(&mut body, units.len() as i32);
            push_i32(&mut body, 8 + 16 + 4 + 8 + 40); // offString from record start
            push_i32(&mut body, 0); // options
            for v in [0, 0, -1, -1] {
                push_i32(&mut body, v);
            }
            push_i32(&mut body, 0); // offDx
            for u in &units {
                body.extend_from_slice(&u.to_le_bytes());
            }
            self.record_bytes(84, &body);
        }
        self.select(0x8000_000Du32); // SYSTEM_FONT
        self.delete(h);
    }

    fn bitmap(&mut self, transform: Affine, rect: Rect, png: &[u8]) {
        let Ok(pm) = tiny_skia::Pixmap::decode_png(png) else {
            return;
        };
        let (w, h) = (pm.width(), pm.height());
        if w == 0 || h == 0 {
            return;
        }
        // World transform: local mm to logical units.
        let dev = Affine::new([100.0, 0.0, 0.0, -100.0, 0.0, self.page_h * 100.0]);
        let m = dev * transform;
        let c = m.as_coeffs();
        self.record(33, &[]);
        let mut body = Vec::new();
        for v in c {
            body.extend_from_slice(&(v as f32).to_le_bytes());
        }
        self.record_bytes(35, &body);
        // 32-bit bottom-up DIB, BGRA, straight alpha.
        let mut bmi = Vec::new();
        push_i32(&mut bmi, 40);
        push_i32(&mut bmi, w as i32);
        push_i32(&mut bmi, h as i32);
        bmi.extend_from_slice(&1u16.to_le_bytes());
        bmi.extend_from_slice(&32u16.to_le_bytes());
        push_i32(&mut bmi, 0);
        push_i32(&mut bmi, (w * h * 4) as i32);
        for _ in 0..4 {
            push_i32(&mut bmi, 0);
        }
        let mut bits = Vec::with_capacity((w * h * 4) as usize);
        for row in (0..h).rev() {
            for x in 0..w {
                let p = pm.pixels()[(row * w + x) as usize].demultiply();
                bits.extend_from_slice(&[p.blue(), p.green(), p.red(), p.alpha()]);
            }
        }
        // Destination in local (world) units: the rect, y flipped by the
        // world transform itself, so pass the rect in mm with y up as is.
        let mut body = Vec::new();
        for v in [0, 0, -1, -1] {
            push_i32(&mut body, v);
        }
        push_i32(&mut body, rect.x0.round() as i32);
        push_i32(&mut body, rect.y1.round() as i32);
        push_i32(&mut body, 0);
        push_i32(&mut body, 0);
        push_i32(&mut body, w as i32);
        push_i32(&mut body, h as i32);
        // The fixed part is 80 bytes including the record header.
        let off_bmi = 80;
        push_i32(&mut body, off_bmi);
        push_i32(&mut body, bmi.len() as i32);
        push_i32(&mut body, off_bmi + bmi.len() as i32);
        push_i32(&mut body, bits.len() as i32);
        push_i32(&mut body, 0);
        push_i32(&mut body, 0x00CC0020);
        push_i32(&mut body, rect.width().round().max(1.0) as i32);
        push_i32(&mut body, -(rect.height().round().max(1.0) as i32));
        body.extend_from_slice(&bmi);
        body.extend_from_slice(&bits);
        self.record_bytes(81, &body);
        self.record(34, &[-1]);
    }

    fn shape(&mut self, shape: &Shape, parent: Affine) {
        if !shape.visible {
            return;
        }
        if !shape.effects.is_empty() {
            let ev = tracedraw_core::live::evaluate(shape);
            for s in ev
                .below
                .iter()
                .chain(std::iter::once(&ev.main))
                .chain(ev.above.iter())
            {
                let mut s = s.clone();
                s.effects.clear();
                self.shape(&s, parent);
            }
            return;
        }
        let transform = parent * shape.transform;
        let c = transform.as_coeffs();
        let scale = (c[0] * c[3] - c[1] * c[2]).abs().sqrt();
        match &shape.kind {
            ShapeKind::Group { children } => {
                for ch in children {
                    self.shape(ch, transform);
                }
            }
            ShapeKind::Table(_) | ShapeKind::SymbolInstance { .. } => {
                for ch in shape.expand(self.symbols) {
                    self.shape(&ch, transform);
                }
            }
            ShapeKind::ClipFrame { frame, contents } => {
                let clip = transform * frame.local_path();
                self.record(33, &[]);
                self.path(&clip);
                self.record(67, &[5]);
                for ch in contents {
                    self.shape(ch, transform);
                }
                self.record(34, &[-1]);
                // The frame's own fill and outline, over the contents.
                let mut f = (**frame).clone();
                f.fill = Fill::None;
                self.shape(&f, transform);
            }
            ShapeKind::Text {
                spans,
                origin,
                align,
                ..
            } => self.text(shape, transform, spans, *origin, *align),
            ShapeKind::Bitmap { rect, png, .. } => {
                self.bitmap(transform, *rect, png);
                if let Some(st) = &shape.stroke {
                    let path = transform * shape.local_path();
                    self.fill_and_stroke(&path, &Fill::None, Some(st), scale, true);
                }
            }
            _ => {
                let path = transform * shape.local_path();
                let even_odd = shape
                    .data
                    .iter()
                    .any(|(k, v)| k == "fill.rule" && v == "evenodd")
                    || !matches!(shape.kind, ShapeKind::Text { .. });
                self.fill_and_stroke(&path, &shape.fill, shape.stroke.as_ref(), scale, even_odd);
                if let Some(st) = &shape.stroke {
                    for head in tracedraw_core::style::arrowhead_paths(&shape.page_path(), st) {
                        self.fill_and_stroke(
                            &(parent * head),
                            &Fill::Solid(st.color),
                            None,
                            1.0,
                            false,
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod export_tests {
    use super::*;
    use tracedraw_core::{geometry::Rect, Document};

    #[test]
    fn bitmaps_round_trip_with_their_placement() {
        let mut doc = Document::new("t", Size::new(100.0, 50.0));
        let mut ids = doc.ids().clone();
        // 2 x 2 image: red, blue on top; green, white below.
        let png = crate::svg::encode_png(
            2,
            2,
            &[
                255, 0, 0, 255, 0, 0, 255, 255, 0, 255, 0, 255, 255, 255, 255, 255,
            ],
        );
        let mut b = Shape::new(
            ids.shape(),
            ShapeKind::Bitmap {
                rect: Rect::new(10.0, 20.0, 50.0, 40.0),
                width_px: 2,
                height_px: 2,
                png,
            },
        );
        b.fill = Fill::None;
        b.stroke = None;
        doc.pages[0].layers[0].shapes.push(b);
        doc.set_ids(ids);
        let bytes = page_to_emf(&doc, 0);
        let mut ids2 = IdSource::default();
        let imp = parse(&bytes, &mut ids2).expect("re-read");
        assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
        assert_eq!(imp.shapes.len(), 1);
        let s = &imp.shapes[0];
        let bb = s.bounds();
        assert!(
            (bb.x0 - 10.0).abs() < 0.02 && (bb.x1 - 50.0).abs() < 0.02,
            "{bb:?}"
        );
        assert!(
            (bb.y0 - 20.0).abs() < 0.02 && (bb.y1 - 40.0).abs() < 0.02,
            "{bb:?}"
        );
        match &s.kind {
            ShapeKind::Bitmap { png, .. } => {
                let pm = tiny_skia::Pixmap::decode_png(png).expect("png");
                let px = pm.pixels();
                assert_eq!((px[0].red(), px[0].blue()), (255, 0));
                assert_eq!((px[1].red(), px[1].blue()), (0, 255));
                assert_eq!((px[2].green(), px[2].blue()), (255, 0));
            }
            k => panic!("{k:?}"),
        }
        // Not mirrored: the local top-left of the rect stays top-left on
        // the page after the shape transform.
        let tl = s.transform * Point::new(10.0, 40.0);
        let br = s.transform * Point::new(50.0, 20.0);
        assert!(tl.x < br.x && tl.y > br.y, "{tl:?} {br:?}");
    }

    #[test]
    fn export_round_trips_through_the_reader() {
        let mut doc = Document::new("t", Size::new(100.0, 50.0));
        let mut ids = doc.ids().clone();
        let mut r = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 30.0, 20.0),
                radius: 0.0,
            },
        );
        r.fill = Fill::Solid(Color::rgb8(0, 0, 255));
        r.stroke = Some(Stroke::new(Color::rgb8(255, 0, 0), 2.0));
        let mut t = Shape::new(
            ids.shape(),
            ShapeKind::Text {
                spans: vec![TextSpan::new("Hi", "Arial", 36.0)],
                origin: Point::new(50.0, 40.0),
                frame: None,
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            },
        );
        t.fill = Fill::Solid(Color::rgb8(0, 128, 0));
        let mut e = Shape::new(
            ids.shape(),
            ShapeKind::Ellipse {
                rect: Rect::new(60.0, 5.0, 90.0, 25.0),
                arc: None,
            },
        );
        e.fill = Fill::Fountain(tracedraw_core::Fountain::two(
            tracedraw_core::style::FountainKind::Linear,
            Color::BLACK,
            Color::WHITE,
            0.0,
        ));
        e.stroke = None;
        doc.pages[0].layers[0].shapes.extend([r, t, e]);
        doc.set_ids(ids);
        let bytes = page_to_emf(&doc, 0);
        assert!(is_emf(&bytes));
        assert_eq!(u32_at(&bytes, 48) as usize, bytes.len());
        let mut ids2 = IdSource::default();
        let imp = parse(&bytes, &mut ids2).expect("re-read");
        assert!((imp.size.width - 100.0).abs() < 1e-6 && (imp.size.height - 50.0).abs() < 1e-6);
        assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
        // Rectangle: fill and stroke come back as one filled and one stroked path.
        let b = imp.shapes[0].bounds();
        assert!(
            (b.x0 - 10.0).abs() < 0.02 && (b.y1 - 20.0).abs() < 0.02,
            "{b:?}"
        );
        assert_eq!(imp.shapes[0].fill, Fill::Solid(Color::rgb8(0, 0, 255)));
        let st = imp.shapes[1].stroke.as_ref().expect("stroke");
        assert_eq!(st.color, Color::rgb8(255, 0, 0));
        assert!((st.width - 2.0).abs() < 0.02);
        // Text keeps its content, size and colour and sits on its baseline.
        let text = imp
            .shapes
            .iter()
            .find(|s| matches!(s.kind, ShapeKind::Text { .. }))
            .expect("text");
        match &text.kind {
            ShapeKind::Text { spans, .. } => {
                assert_eq!(spans[0].text, "Hi");
                assert!(
                    (spans[0].size_pt - 36.0).abs() < 0.1,
                    "{}",
                    spans[0].size_pt
                );
            }
            _ => unreachable!(),
        }
        let o = text.transform * Point::ZERO;
        assert!(
            (o.x - 50.0).abs() < 0.02 && (o.y - 40.0).abs() < 0.02,
            "{o:?}"
        );
        assert_eq!(text.fill, Fill::Solid(Color::rgb8(0, 128, 0)));
        // The fountain became clipped bands inside a ClipFrame-free run:
        // dozens of fills with colours from black to white.
        let bands: Vec<&Shape> = imp
            .shapes
            .iter()
            .filter(|s| s.bounds().x0 >= 59.0 && s.bounds().x1 <= 91.0)
            .collect();
        assert!(bands.len() >= 32, "{}", bands.len());
    }
}
