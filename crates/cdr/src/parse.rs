//! From RIFF chunks to a document.
//!
//! The layout below reflects public reverse-engineering notes on the CDR
//! stream (version 7 through X7). Offsets vary by version; where we are not
//! sure we read defensively and record a warning in the [`ParseReport`]
//! rather than guess silently. Everything here is to be validated against a
//! corpus of real files, see `docs/cdr-format.md`.
//!
//! Units: CDR stores coordinates as signed 32-bit integers in 1/254000 inch,
//! which is exactly 0.0001 mm. Page origin is the centre of the page in old
//! versions; we shift so the page's bottom-left is (0, 0).

use crate::container::Version;
use crate::riff::{Chunk, Tree};
use std::collections::HashMap;
use tracedraw_core::{
    document::{Layer, Page, Shape, ShapeKind},
    geometry::{Affine, BezPath, Point, Rect, Size},
    Color, Document, Fill, LineCap, LineJoin, Stroke,
};

/// What the parser understood and what it skipped.
#[derive(Debug, Default, Clone)]
pub struct ParseReport {
    pub version: Option<Version>,
    pub pages: usize,
    pub layers: usize,
    pub shapes: usize,
    pub skipped_objects: usize,
    pub warnings: Vec<String>,
}

impl ParseReport {
    fn warn(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        if self.warnings.len() < 200 && !self.warnings.contains(&msg) {
            self.warnings.push(msg);
        }
    }
}

/// CDR coordinate unit to millimetres.
const UNIT_MM: f64 = 25.4 / 254000.0;

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Reader { b, pos: 0 }
    }
    fn at(b: &'a [u8], pos: usize) -> Self {
        Reader { b, pos }
    }
    fn remaining(&self) -> usize {
        self.b.len().saturating_sub(self.pos)
    }
    fn skip(&mut self, n: usize) {
        self.pos = self.pos.saturating_add(n);
    }
    fn u8(&mut self) -> Option<u8> {
        let v = *self.b.get(self.pos)?;
        self.pos += 1;
        Some(v)
    }
    fn u16(&mut self) -> Option<u16> {
        let s = self.b.get(self.pos..self.pos + 2)?;
        self.pos += 2;
        Some(u16::from_le_bytes([s[0], s[1]]))
    }
    fn u32(&mut self) -> Option<u32> {
        let s = self.b.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn i32(&mut self) -> Option<i32> {
        self.u32().map(|v| v as i32)
    }
    fn f64(&mut self) -> Option<f64> {
        let s = self.b.get(self.pos..self.pos + 8)?;
        self.pos += 8;
        let mut a = [0u8; 8];
        a.copy_from_slice(s);
        Some(f64::from_le_bytes(a))
    }
    /// Coordinate in mm.
    fn coord(&mut self) -> Option<f64> {
        self.i32().map(|v| v as f64 * UNIT_MM)
    }
}

#[derive(Debug, Clone)]
struct FillDef {
    fill: Fill,
}

#[derive(Debug, Clone)]
struct OutlineDef {
    stroke: Option<Stroke>,
}

struct Ctx<'a> {
    main: &'a [u8],
    tree: &'a Tree,
    version: Version,
    fills: HashMap<u32, FillDef>,
    outlines: HashMap<u32, OutlineDef>,
    report: ParseReport,
}

pub fn parse_document(tree: &Tree, main: &[u8], version: Version) -> (Document, ParseReport) {
    let mut ctx = Ctx {
        main,
        tree,
        version,
        fills: HashMap::new(),
        outlines: HashMap::new(),
        report: ParseReport::default(),
    };
    ctx.report.version = Some(version);

    // Pass 1: style tables, wherever they are in the tree.
    tree.root.walk(&mut |c, _| {
        if c.is(b"fild") {
            ctx.read_fill(c);
        } else if c.is(b"outl") {
            ctx.read_outline(c);
        }
    });

    // Page size from the document configuration chunk, else A4.
    let mut page_size = Size::new(210.0, 297.0);
    let mut found_mcfg = false;
    tree.root.walk(&mut |c, _| {
        if !found_mcfg && c.is(b"mcfg") {
            if let Some(s) = ctx.read_page_size(c) {
                page_size = s;
                found_mcfg = true;
            }
        }
    });
    if !found_mcfg {
        ctx.report.warn("no readable mcfg chunk; assuming A4");
    }

    let mut doc = Document::new("Untitled", page_size);
    doc.pages.clear();

    // Pass 2: pages. the editor stores a master page (page 0, desktop layers)
    // followed by the real pages. We keep every page that has visible
    // content, and always keep at least one.
    let mut pages: Vec<&Chunk> = Vec::new();
    tree.root.walk(&mut |c, _| {
        if c.is(b"page") {
            pages.push(c);
        }
    });
    if pages.is_empty() {
        ctx.report.warn("no page chunks found");
    }

    for (i, pc) in pages.iter().enumerate() {
        let page_id = doc.ids_mut().page();
        let mut page = Page {
            id: page_id,
            name: format!("Page {}", i + 1),
            size: page_size,
            layers: Vec::new(),
            guides: Vec::new(),
        };
        let mut layer_chunks: Vec<&Chunk> = Vec::new();
        pc.walk(&mut |c, _| {
            if c.is(b"layr") {
                layer_chunks.push(c);
            }
        });
        for (k, lc) in layer_chunks.iter().enumerate() {
            let mut layer = Layer::new(doc.ids_mut().layer(), format!("Layer {}", k + 1));
            // Layer name lives in a `lnam`/`layr` info block in newer files; skipped for now.
            let mut objects: Vec<&Chunk> = Vec::new();
            lc.walk(&mut |c, _| {
                if c.is(b"obj ") {
                    objects.push(c);
                }
            });
            for oc in objects {
                let id = doc.ids_mut().shape();
                match ctx.read_object(oc, id, page_size) {
                    Some(shape) => layer.shapes.push(shape),
                    None => ctx.report.skipped_objects += 1,
                }
            }
            layer.visible = true;
            page.layers.push(layer);
        }
        if page.layers.is_empty() {
            page.layers
                .push(Layer::new(doc.ids_mut().layer(), "Layer 1"));
        }
        doc.pages.push(page);
    }

    // Drop a leading master page that carries nothing, keeping at least one page.
    if doc.pages.len() > 1 && doc.pages[0].layers.iter().all(|l| l.shapes.is_empty()) {
        doc.pages.remove(0);
        for (i, p) in doc.pages.iter_mut().enumerate() {
            p.name = format!("Page {}", i + 1);
        }
    }
    if doc.pages.is_empty() {
        let pid = doc.ids_mut().page();
        let lid = doc.ids_mut().layer();
        doc.pages.push(Page {
            id: pid,
            name: "Page 1".into(),
            size: page_size,
            layers: vec![Layer::new(lid, "Layer 1")],
            guides: Vec::new(),
        });
    }

    ctx.report.pages = doc.pages.len();
    ctx.report.layers = doc.pages.iter().map(|p| p.layers.len()).sum();
    ctx.report.shapes = doc
        .pages
        .iter()
        .flat_map(|p| &p.layers)
        .map(|l| l.shapes.len())
        .sum();
    (doc, ctx.report)
}

impl<'a> Ctx<'a> {
    fn data(&self, c: &Chunk) -> &'a [u8] {
        self.tree.data(self.main, c)
    }

    /// `mcfg`: page width and height. Position depends on version.
    fn read_page_size(&mut self, c: &Chunk) -> Option<Size> {
        let d = self.data(c);
        let v = self.version.0;
        let off = if v >= 13 {
            12
        } else if v >= 9 {
            4
        } else {
            0
        };
        let mut r = Reader::at(d, off);
        let w = r.coord()?;
        let h = r.coord()?;
        if w > 1.0 && h > 1.0 && w < 10_000.0 && h < 10_000.0 {
            Some(Size::new(w, h))
        } else {
            self.report.warn(format!(
                "mcfg page size {w:.2}x{h:.2} mm looks wrong; ignored"
            ));
            None
        }
    }

    fn read_color(&mut self, r: &mut Reader) -> Option<Color> {
        let model = r.u16()?;
        let _palette = r.u16()?;
        let raw = r.u32()?;
        let b = raw.to_le_bytes();
        Some(match model {
            // CMYK in percent
            0x02 | 0x11 => Color::cmyk_pct(b[0] as f32, b[1] as f32, b[2] as f32, b[3] as f32),
            // CMYK in 0..255
            0x03 => Color::cmyk8(b[0], b[1], b[2], b[3]),
            // CMY in 0..255
            0x04 => Color::cmyk8(b[0], b[1], b[2], 0),
            // BGR
            0x05 | 0x0a => Color::rgb8(b[2], b[1], b[0]),
            // Grayscale 0..255
            0x09 => Color::Gray {
                v: b[0] as f32 / 255.0,
            },
            // Registration / unknown models: fall back to black and note it.
            other => {
                self.report
                    .warn(format!("unknown colour model 0x{other:02x}; using black"));
                Color::BLACK
            }
        })
    }

    /// `fild`: fill definition. We model solid fills and the two simplest
    /// fountain fills; everything else becomes "no fill" with a warning.
    fn read_fill(&mut self, c: &Chunk) {
        let d = self.data(c);
        let mut r = Reader::new(d);
        let Some(id) = r.u32() else { return };
        let v = self.version.0;
        if v >= 13 {
            // X3+: a 4-byte flags/version field precedes the type.
            r.skip(4);
        }
        let Some(ftype) = r.u16() else { return };
        let fill = match ftype {
            0 => Fill::None,
            1 => {
                if v >= 13 {
                    r.skip(2);
                }
                match self.read_color(&mut r) {
                    Some(c) => Fill::Solid(c),
                    None => Fill::None,
                }
            }
            2 => {
                // Fountain fill: skip to the colour stops. Layout is version
                // dependent; we read the first and last stop only.
                if v >= 13 {
                    r.skip(2);
                }
                let _gradient_type = r.u8();
                r.skip(if v >= 13 { 19 } else { 7 });
                let angle = r.f64().unwrap_or(0.0);
                r.skip(if v >= 13 { 20 } else { 12 });
                let stops = r.u16().unwrap_or(0) as usize;
                let mut colors = Vec::new();
                for _ in 0..stops.min(64) {
                    if let Some(c) = self.read_color(&mut r) {
                        colors.push(c);
                    }
                    r.skip(if v >= 13 { 6 } else { 4 });
                }
                match (colors.first(), colors.last()) {
                    (Some(a), Some(b)) => Fill::Linear {
                        from: *a,
                        to: *b,
                        angle,
                    },
                    _ => Fill::None,
                }
            }
            other => {
                self.report
                    .warn(format!("fill type {other} not modelled yet (fill id {id})"));
                Fill::None
            }
        };
        self.fills.insert(id, FillDef { fill });
    }

    /// `outl`: outline definition.
    fn read_outline(&mut self, c: &Chunk) {
        let d = self.data(c);
        let mut r = Reader::new(d);
        let Some(id) = r.u32() else { return };
        let v = self.version.0;
        if v >= 13 {
            r.skip(4);
        }
        let Some(line_type) = r.u16() else { return };
        let caps = r.u16().unwrap_or(0);
        let join = r.u16().unwrap_or(0);
        if v >= 13 {
            r.skip(2);
        }
        let width = r.coord().unwrap_or(0.0);
        let _stretch = r.u16();
        let _angle = r.u32();
        // Colour follows; dash array after it.
        let color = if v >= 13 {
            r.skip(6);
            self.read_color(&mut r)
        } else {
            self.read_color(&mut r)
        };
        let stroke = if line_type & 0x1 != 0 || width <= 0.0 && color.is_none() {
            // Bit 0: no outline.
            None
        } else {
            let mut s = Stroke::new(
                color.unwrap_or(Color::BLACK),
                if width <= 0.0 {
                    Stroke::HAIRLINE
                } else {
                    width
                },
            );
            s.cap = match caps {
                1 => LineCap::Round,
                2 => LineCap::Square,
                _ => LineCap::Butt,
            };
            s.join = match join {
                1 => LineJoin::Round,
                2 => LineJoin::Bevel,
                _ => LineJoin::Miter,
            };
            s.scale_with_object = line_type & 0x20 != 0;
            s.behind_fill = line_type & 0x10 != 0;
            Some(s)
        };
        self.outlines.insert(id, OutlineDef { stroke });
    }

    /// `trfd`: object transform (2x3 matrix). Returns identity when unreadable.
    fn read_transform(&mut self, c: &Chunk) -> Affine {
        let d = self.data(c);
        let v = self.version.0;
        let mut r = Reader::new(d);
        // Header: length, argument count, start of args, start of arg types.
        let _len = r.u32();
        let num_args = r.u32().unwrap_or(0);
        let start_args = r.u32().unwrap_or(0) as usize;
        let _start_types = r.u32();
        if num_args == 0 {
            return Affine::IDENTITY;
        }
        let mut ar = Reader::at(d, start_args);
        let Some(first) = ar.u32() else {
            return Affine::IDENTITY;
        };
        let mut tr = Reader::at(d, first as usize);
        let _tmp = tr.u32();
        if v >= 13 {
            tr.skip(4);
        }
        let _kind = tr.u16();
        if v >= 13 {
            tr.skip(2);
        }
        let (Some(a), Some(c_), Some(e), Some(b), Some(d_), Some(f)) =
            (tr.f64(), tr.f64(), tr.f64(), tr.f64(), tr.f64(), tr.f64())
        else {
            self.report.warn("trfd too short; identity used");
            return Affine::IDENTITY;
        };
        // Matrix rows are (a c e; b d f) with translation in CDR units.
        if !a.is_finite() || !d_.is_finite() {
            return Affine::IDENTITY;
        }
        Affine::new([a, b, c_, d_, e * UNIT_MM, f * UNIT_MM])
    }

    /// `obj `: one object, built from its `loda` (attributes) and `trfd`.
    fn read_object(
        &mut self,
        oc: &Chunk,
        id: tracedraw_core::ShapeId,
        page_size: Size,
    ) -> Option<Shape> {
        let mut loda = None;
        let mut trfd = None;
        oc.walk(&mut |c, _| {
            if c.is(b"loda") && loda.is_none() {
                loda = Some(c);
            } else if c.is(b"trfd") && trfd.is_none() {
                trfd = Some(c);
            }
        });
        let loda = loda?;
        let transform = trfd
            .map(|t| self.read_transform(t))
            .unwrap_or(Affine::IDENTITY);

        let d = self.data(loda);
        let mut r = Reader::new(d);
        let _len = r.u32()?;
        let num_args = r.u32()? as usize;
        let start_args = r.u32()? as usize;
        let start_types = r.u32()? as usize;
        let kind_code = r.u32()?;

        let mut args: Vec<(u32, usize)> = Vec::with_capacity(num_args.min(64));
        for i in 0..num_args.min(64) {
            let off = Reader::at(d, start_args + i * 4).u32()? as usize;
            let ty = Reader::at(d, start_types + i * 4).u32()?;
            args.push((ty, off));
        }

        let mut fill = Fill::None;
        let mut stroke: Option<Stroke> = None;
        let mut kind: Option<ShapeKind> = None;

        for (ty, off) in &args {
            match ty {
                0x14 => {
                    if let Some(fid) = Reader::at(d, *off).u32() {
                        if let Some(f) = self.fills.get(&fid) {
                            fill = f.fill.clone();
                        }
                    }
                }
                0x0a => {
                    if let Some(oid) = Reader::at(d, *off).u32() {
                        if let Some(o) = self.outlines.get(&oid) {
                            stroke = o.stroke.clone();
                        }
                    }
                }
                0x1e => {
                    kind = self.read_geometry(kind_code, d, *off);
                }
                _ => {}
            }
        }

        let kind = match kind {
            Some(k) => k,
            None => {
                self.report
                    .warn(format!("object type 0x{kind_code:02x} not modelled yet"));
                return None;
            }
        };

        // CDR's page origin is the page centre; ours is the bottom-left.
        let recenter = Affine::translate((page_size.width / 2.0, page_size.height / 2.0));
        let mut shape = Shape::new(id, kind);
        shape.transform = recenter * transform;
        shape.fill = fill;
        shape.stroke = stroke;
        Some(shape)
    }

    fn read_geometry(&mut self, kind_code: u32, d: &[u8], off: usize) -> Option<ShapeKind> {
        let v = self.version.0;
        let mut r = Reader::at(d, off);
        match kind_code {
            // Rectangle: width, height from (0,0), then corner radii.
            0x01 => {
                let w = r.coord()?;
                let h = r.coord()?;
                let radius = if v < 15 {
                    r.coord().unwrap_or(0.0)
                } else {
                    0.0
                };
                let rect = Rect::new(
                    0.0_f64.min(w),
                    0.0_f64.min(h),
                    0.0_f64.max(w),
                    0.0_f64.max(h),
                );
                Some(ShapeKind::Rect {
                    rect,
                    radius: radius.max(0.0),
                })
            }
            // Ellipse: width, height, then start/end angle and pie flag.
            0x02 => {
                let w = r.coord()?;
                let h = r.coord()?;
                let rect = Rect::new(
                    0.0_f64.min(w),
                    0.0_f64.min(h),
                    0.0_f64.max(w),
                    0.0_f64.max(h),
                );
                Some(ShapeKind::Ellipse { rect, arc: None })
            }
            // Curve: point list followed by one type byte per point.
            0x03 => {
                let n = if v >= 16 {
                    r.u32()? as usize
                } else {
                    r.u16()? as usize
                };
                if n == 0 || n > 1_000_000 || r.remaining() < n * 9 {
                    return None;
                }
                let mut pts = Vec::with_capacity(n);
                for _ in 0..n {
                    pts.push(Point::new(r.coord()?, r.coord()?));
                }
                let mut types = Vec::with_capacity(n);
                for _ in 0..n {
                    types.push(r.u8()?);
                }
                let (path, closed) = build_path(&pts, &types);
                if path.elements().is_empty() {
                    return None;
                }
                Some(ShapeKind::Path { path, closed })
            }
            // Polygon: width, height, then the base curve shares the 0x03 layout.
            0x14 => {
                let w = r.coord()?;
                let h = r.coord()?;
                let rect = Rect::new(
                    0.0_f64.min(w),
                    0.0_f64.min(h),
                    0.0_f64.max(w),
                    0.0_f64.max(h),
                );
                Some(ShapeKind::Polygon {
                    rect,
                    points: 5,
                    sharpness: 0.0,
                })
            }
            _ => None,
        }
    }
}

/// Point type byte, as observed:
/// - `0x00` move to, `0x40` line to, `0x80` Bezier control point,
///   `0xC0` Bezier end point (preceded by two control points);
/// - bit `0x08` on an end point closes the subpath.
fn build_path(pts: &[Point], types: &[u8]) -> (BezPath, bool) {
    let mut path = BezPath::new();
    let mut closed = false;
    let mut ctrl: Vec<Point> = Vec::with_capacity(2);
    let mut started = false;
    for (p, t) in pts.iter().zip(types) {
        match t & 0xC0 {
            0x00 => {
                path.move_to(*p);
                started = true;
                ctrl.clear();
            }
            0x40 => {
                if !started {
                    path.move_to(*p);
                    started = true;
                } else {
                    path.line_to(*p);
                }
                ctrl.clear();
            }
            0x80 => ctrl.push(*p),
            _ => {
                if !started {
                    path.move_to(*p);
                    started = true;
                } else if ctrl.len() >= 2 {
                    path.curve_to(ctrl[0], ctrl[1], *p);
                } else if ctrl.len() == 1 {
                    path.quad_to(ctrl[0], *p);
                } else {
                    path.line_to(*p);
                }
                ctrl.clear();
            }
        }
        if t & 0x08 != 0 && started {
            path.close_path();
            closed = true;
        }
    }
    (path, closed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Shape as _;

    #[test]
    fn path_builder_handles_moves_lines_and_curves() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(12.0, 5.0),
            Point::new(8.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        let types = [0x00, 0x40, 0x80, 0x80, 0xC8];
        let (p, closed) = build_path(&pts, &types);
        assert!(closed);
        assert_eq!(p.elements().len(), 4); // move, line, curve, close
        let b = p.bounding_box();
        assert!(b.x1 >= 10.0 && b.y1 >= 10.0);
    }

    #[test]
    fn unit_conversion() {
        assert!((254000.0 * UNIT_MM - 25.4).abs() < 1e-9);
    }
}
