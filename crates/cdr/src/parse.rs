//! From RIFF chunks to a document.
//!
//! The layouts below follow the public, declarative description of the
//! format (a Kaitai Struct specification) and public reverse-engineering
//! notes. Offsets vary by version; where the spec is silent we read
//! defensively and record a warning in the [`ParseReport`] rather than guess
//! silently. `docs/cdr-format.md` lists every layout fact with its status.
//!
//! Units: from version 6 on, coordinates are signed 32-bit integers in
//! 1/254000 inch (exactly 0.0001 mm). Versions before 6 store signed 16-bit
//! values in 1/1000 inch. Angles are signed 32-bit millionths of a degree
//! (16-bit tenths of a degree before version 6). Page origin is the centre
//! of the page; we shift so the page's bottom-left is (0, 0).

use crate::container::Version;
use crate::riff::{Chunk, Tree};
use crate::text::{self, RunStyle, StyleRec};
use std::collections::HashMap;
use tracedraw_core::{
    document::{EllipseArc, Layer, Page, ParagraphStyle, Shape, ShapeKind, TextAlign, TextSpan},
    geometry::{Affine, BezPath, Point, Rect, Size},
    style::{Arrowhead, Fountain, FountainKind, Pattern, PatternTile, Stop, Texture, TextureKind},
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

/// CDR coordinate unit (version 6 and later) to millimetres.
const UNIT_MM: f64 = 25.4 / 254000.0;
/// Coordinate unit of versions before 6 (1/1000 inch) to millimetres.
const UNIT16_MM: f64 = 25.4 / 1000.0;

/// Argument types of the `loda` argument table.
const ARG_OUTLINE: u32 = 0x0a;
const ARG_FILL: u32 = 0x14;
const ARG_COORDS: u32 = 0x1e;
const ARG_STYLE: u32 = 200;
const ARG_NAME: u32 = 1000;
const ARG_OPACITY: u32 = 8000;
const ARG_POLYGON: u32 = 11000;
const ARG_PAGE_SIZE: u32 = 19130;

/// Family and size used when neither the run nor its style names one.
const DEFAULT_FAMILY: &str = "Arial";
const DEFAULT_SIZE_PT: f64 = 24.0;

/// Object types stored in the `loda` header.
const OBJ_RECT: u32 = 0x01;
const OBJ_ELLIPSE: u32 = 0x02;
const OBJ_CURVE: u32 = 0x03;
const OBJ_ARTISTIC_TEXT: u32 = 0x04;
const OBJ_BITMAP: u32 = 0x05;
const OBJ_PARAGRAPH_TEXT: u32 = 0x06;
const OBJ_POLYGON: u32 = 0x14;
const OBJ_PATH: u32 = 0x25;

/// Little-endian cursor over a chunk payload. `v16` selects the 16-bit
/// layouts of versions before 6 for integers, coordinates and angles.
pub(crate) struct Reader<'a> {
    pub(crate) b: &'a [u8],
    pub(crate) pos: usize,
    v16: bool,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(b: &'a [u8], v16: bool) -> Self {
        Reader { b, pos: 0, v16 }
    }
    fn at(b: &'a [u8], pos: usize, v16: bool) -> Self {
        Reader { b, pos, v16 }
    }
    pub(crate) fn remaining(&self) -> usize {
        self.b.len().saturating_sub(self.pos)
    }
    pub(crate) fn skip(&mut self, n: usize) {
        self.pos = self.pos.saturating_add(n);
    }
    pub(crate) fn u8(&mut self) -> Option<u8> {
        let v = *self.b.get(self.pos)?;
        self.pos += 1;
        Some(v)
    }
    pub(crate) fn u16(&mut self) -> Option<u16> {
        let s = self.b.get(self.pos..self.pos.checked_add(2)?)?;
        self.pos += 2;
        Some(u16::from_le_bytes([s[0], s[1]]))
    }
    fn i16(&mut self) -> Option<i16> {
        self.u16().map(|v| v as i16)
    }
    pub(crate) fn u32(&mut self) -> Option<u32> {
        let s = self.b.get(self.pos..self.pos.checked_add(4)?)?;
        self.pos += 4;
        Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    pub(crate) fn i32(&mut self) -> Option<i32> {
        self.u32().map(|v| v as i32)
    }
    fn f64(&mut self) -> Option<f64> {
        let s = self.b.get(self.pos..self.pos.checked_add(8)?)?;
        self.pos += 8;
        let mut a = [0u8; 8];
        a.copy_from_slice(s);
        Some(f64::from_le_bytes(a))
    }
    /// Unsigned integer of the version's natural width (u16 before 6).
    fn uint(&mut self) -> Option<u32> {
        if self.v16 {
            self.u16().map(u32::from)
        } else {
            self.u32()
        }
    }
    /// Signed integer of the version's natural width (i16 before 6).
    pub(crate) fn sint(&mut self) -> Option<i32> {
        if self.v16 {
            self.i16().map(i32::from)
        } else {
            self.i32()
        }
    }
    /// Coordinate in mm.
    fn coord(&mut self) -> Option<f64> {
        if self.v16 {
            self.i16().map(|v| v as f64 * UNIT16_MM)
        } else {
            self.i32().map(|v| v as f64 * UNIT_MM)
        }
    }
    /// Angle in degrees.
    fn angle(&mut self) -> Option<f64> {
        if self.v16 {
            self.i16().map(|v| v as f64 / 10.0)
        } else {
            self.i32().map(|v| v as f64 / 1_000_000.0)
        }
    }
    /// Translation stored as a double in coordinate units, in mm.
    fn f64_coord(&mut self) -> Option<f64> {
        let unit = if self.v16 { UNIT16_MM } else { UNIT_MM };
        self.f64().map(|v| v * unit)
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

#[derive(Debug, Clone)]
struct BitmapDef {
    width_px: u32,
    height_px: u32,
    png: Vec<u8>,
}

/// One entry of a `loda` argument table: type, payload start and end.
#[derive(Debug, Clone, Copy)]
struct Arg {
    ty: u32,
    off: usize,
    end: usize,
}

/// Decoded `loda` header and argument table.
struct Loda {
    kind: u32,
    args: Vec<Arg>,
}

/// Polygon parameters from the `loda` argument of type 11000.
#[derive(Debug, Clone, Copy)]
struct PolygonInfo {
    num_angles: u32,
    rx: f64,
    ry: f64,
    cx: f64,
    cy: f64,
}

struct Ctx<'a> {
    main: &'a [u8],
    tree: &'a Tree,
    /// Effective major version (7 = CDR 7, 13 = X3, 16 = X6 ...).
    v: u16,
    fills: HashMap<u32, FillDef>,
    outlines: HashMap<u32, OutlineDef>,
    bitmaps: HashMap<u32, BitmapDef>,
    /// Two-colour pattern tiles (`bmpf`), 1 bit per pixel: true = front.
    pattern_tiles: HashMap<u32, PatternBits>,
    /// `font` chunks: font id to family name.
    fonts: HashMap<u16, String>,
    /// `stlt` style records by style id.
    styles: HashMap<u32, StyleRec>,
    /// `arrw` arrowhead definitions classified into our presets.
    arrows: HashMap<u32, Arrowhead>,
    report: ParseReport,
}

/// A two-colour pattern tile decoded from `bmpf`.
#[derive(Debug, Clone)]
struct PatternBits {
    width: u32,
    height: u32,
    /// Row-major, top-down; true where the front colour is painted.
    front: Vec<bool>,
}

impl<'a> Ctx<'a> {
    fn new(main: &'a [u8], tree: &'a Tree, v: u16) -> Self {
        Ctx {
            main,
            tree,
            v,
            fills: HashMap::new(),
            outlines: HashMap::new(),
            bitmaps: HashMap::new(),
            pattern_tiles: HashMap::new(),
            fonts: HashMap::new(),
            styles: HashMap::new(),
            arrows: HashMap::new(),
            report: ParseReport::default(),
        }
    }
}

pub fn parse_document(tree: &Tree, main: &[u8], version: Version) -> (Document, ParseReport) {
    let mut ctx = Ctx::new(main, tree, version.0);
    ctx.report.version = Some(version);

    // The `vrsn` chunk carries the version as a number (1300 = X3, 801 =
    // CDR 8 bidi); prefer it over the form-type letter when it is sane.
    let mut vrsn: Option<u16> = None;
    tree.root.walk(&mut |c, _| {
        if vrsn.is_none() && c.is(b"vrsn") {
            let d = tree.data(main, c);
            if let Some(n) = Reader::new(d, false).u16() {
                if (100..=3000).contains(&n) {
                    vrsn = Some(n / 100);
                }
            }
        }
    });
    if let Some(nv) = vrsn {
        if nv != ctx.v {
            ctx.report.warn(format!(
                "vrsn chunk says version {nv}, form type says {}; using vrsn",
                ctx.v
            ));
            ctx.v = nv;
        }
    }

    // Pass 1: style tables and bitmaps, wherever they are in the tree.
    // Before version 7 the fill table chunk is called `fill`. Bitmaps,
    // pattern tiles and arrowheads go first because fills and outlines
    // refer to them.
    tree.root.walk(&mut |c, _| {
        if c.is(b"bmp ") {
            ctx.read_bitmap(c);
        } else if c.is(b"bmpf") {
            ctx.read_pattern_tile(c);
        } else if c.is(b"arrw") {
            ctx.read_arrowhead(c);
        } else if c.is(b"font") {
            ctx.read_font(c);
        }
    });
    tree.root.walk(&mut |c, _| {
        if c.is(b"fild") || c.is(b"fill") {
            ctx.read_fill(c);
        } else if c.is(b"outl") {
            ctx.read_outline(c);
        } else if c.is(b"stlt") {
            ctx.read_styles(c);
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

    // Pass 2: pages. Files store a master page (desktop, guides and grid
    // layers) followed by the real pages. We keep every page that has
    // visible content, and always keep at least one.
    let mut pages: Vec<&Chunk> = Vec::new();
    tree.root.walk(&mut |c, _| {
        if c.is(b"page") {
            pages.push(c);
        }
    });
    if pages.is_empty() {
        ctx.report.warn("no page chunks found");
    }

    let mut master_flags: Vec<bool> = Vec::new();
    for (i, pc) in pages.iter().enumerate() {
        let page_id = doc.ids_mut().page();
        let is_master = ctx.page_is_master(pc);
        master_flags.push(is_master);
        // A page may carry its own size in its `loda` (argument 19130).
        let size = ctx.page_own_size(pc).unwrap_or(page_size);
        let mut page = Page {
            id: page_id,
            name: format!("Page {}", i + 1),
            size,
            layers: Vec::new(),
            guides: Vec::new(),
            background: None,
        };
        let mut layer_chunks: Vec<&Chunk> = Vec::new();
        pc.walk(&mut |c, _| {
            if c.is(b"layr") {
                layer_chunks.push(c);
            }
        });
        for (k, lc) in layer_chunks.iter().enumerate() {
            let default_name = format!("Layer {}", k + 1);
            let name = ctx.layer_name(lc).unwrap_or(default_name);
            let mut layer = Layer::new(doc.ids_mut().layer(), name);
            // Guides and grid layers hold no drawable content.
            match ctx.layer_type(lc) {
                Some(0x0a) | Some(0x1a) => {
                    layer.visible = false;
                    layer.printable = false;
                }
                _ => layer.visible = true,
            }
            let mut objects: Vec<&Chunk> = Vec::new();
            lc.walk(&mut |c, _| {
                if c.is(b"obj ") {
                    objects.push(c);
                }
            });
            for oc in objects {
                let id = doc.ids_mut().shape();
                match ctx.read_object(oc, id, size) {
                    Some(shape) => layer.shapes.push(shape),
                    None => ctx.report.skipped_objects += 1,
                }
            }
            page.layers.push(layer);
        }
        if page.layers.is_empty() {
            page.layers
                .push(Layer::new(doc.ids_mut().layer(), "Layer 1"));
        }
        doc.pages.push(page);
    }

    // Drop a leading master page that carries nothing, keeping at least one page.
    let first_is_master = master_flags.first().copied().unwrap_or(false);
    if doc.pages.len() > 1
        && (first_is_master || doc.pages[0].layers.iter().all(|l| l.shapes.is_empty()))
        && doc.pages[0].layers.iter().all(|l| l.shapes.is_empty())
    {
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
            background: None,
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

    fn v16(&self) -> bool {
        self.v < 6
    }

    fn reader(&self, d: &'a [u8]) -> Reader<'a> {
        Reader::new(d, self.v16())
    }

    fn reader_at(&self, d: &'a [u8], pos: usize) -> Reader<'a> {
        Reader::at(d, pos, self.v16())
    }

    /// `flgs` of a `page` or `layr` list: 4 bytes, byte 3 is the record
    /// type (0x90 page, 0x98 layer), byte 2 marks the master page, byte 0
    /// is the layer type (0 normal, 8 desktop, 0x0a guides, 0x1a grid).
    fn flags(&self, list: &Chunk) -> Option<[u8; 4]> {
        let f = list.find(b"flgs")?;
        let d = self.data(f);
        let s = d.get(0..4)?;
        Some([s[0], s[1], s[2], s[3]])
    }

    fn page_is_master(&self, pc: &Chunk) -> bool {
        matches!(self.flags(pc), Some(f) if f[3] == 0x90 && f[2] != 0)
    }

    fn layer_type(&self, lc: &Chunk) -> Option<u8> {
        self.flags(lc).filter(|f| f[3] == 0x98).map(|f| f[0])
    }

    /// The `loda`/`lobj` chunk that is a direct child of a list.
    fn own_loda<'c>(&self, list: &'c Chunk) -> Option<&'c Chunk> {
        list.children
            .iter()
            .find(|c| c.is(b"loda") || c.is(b"lobj"))
    }

    fn layer_name(&mut self, lc: &Chunk) -> Option<String> {
        let loda = self.own_loda(lc)?;
        let d = self.data(loda);
        let l = self.read_loda(d)?;
        let arg = l.args.iter().find(|a| a.ty == ARG_NAME)?;
        let name = read_name(d.get(arg.off..arg.end)?, self.v);
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    }

    fn page_own_size(&mut self, pc: &Chunk) -> Option<Size> {
        let loda = self.own_loda(pc)?;
        let d = self.data(loda);
        let l = self.read_loda(d)?;
        let arg = l.args.iter().find(|a| a.ty == ARG_PAGE_SIZE)?;
        let mut r = self.reader_at(d, arg.off);
        let w = r.coord()?;
        let h = r.coord()?;
        if w > 1.0 && h > 1.0 && w < 10_000.0 && h < 10_000.0 {
            Some(Size::new(w, h))
        } else {
            None
        }
    }

    /// `mcfg`: page width and height. Leading unknown bytes depend on the
    /// version: 12 (X3+), 4 (9 to 12), 28 (6), none before.
    fn read_page_size(&mut self, c: &Chunk) -> Option<Size> {
        let d = self.data(c);
        let v = self.v;
        let off = if v >= 13 {
            12
        } else if v >= 9 {
            4
        } else if v == 6 {
            0x1c
        } else {
            0
        };
        let mut r = self.reader_at(d, off);
        let (w, h) = if v < 4 {
            // Old layout: 2 unknown bytes, then two corners.
            r.skip(2);
            let x0 = r.coord()?;
            let y0 = r.coord()?;
            let x1 = r.coord()?;
            let y1 = r.coord()?;
            (x1 - x0, y1 - y0)
        } else {
            (r.coord()?, r.coord()?)
        };
        if w > 1.0 && h > 1.0 && w < 10_000.0 && h < 10_000.0 {
            Some(Size::new(w, h))
        } else {
            self.report.warn(format!(
                "mcfg page size {w:.2}x{h:.2} mm looks wrong; ignored"
            ));
            None
        }
    }

    /// Colour record. From version 5: model u16, palette u16, 4 unknown
    /// bytes, 4 value bytes (12 bytes). Version 4: model u16, C M Y K as
    /// u16 each, 2 unknown bytes. Before 4: model u8, 4 value bytes.
    fn read_color(&mut self, r: &mut Reader) -> Option<Color> {
        let (model, b) = if self.v >= 5 {
            let model = r.u16()?;
            let _palette = r.u16()?;
            r.skip(4);
            let raw = r.u32()?;
            (model, raw.to_le_bytes())
        } else if self.v >= 4 {
            let model = r.u16()?;
            let c = r.u16()?;
            let m = r.u16()?;
            let y = r.u16()?;
            let k = r.u16()?;
            r.skip(2);
            (model, [c as u8, m as u8, y as u8, k as u8])
        } else {
            let model = r.u8()? as u16;
            let raw = r.u32()?;
            (model, raw.to_le_bytes())
        };
        Some(match model {
            // CMYK in percent
            0x02 => Color::cmyk_pct(b[0] as f32, b[1] as f32, b[2] as f32, b[3] as f32),
            // CMYK in 0..255 (two encodings)
            0x03 | 0x11 => Color::cmyk8(b[0], b[1], b[2], b[3]),
            // CMY in 0..255
            0x04 => Color::cmyk8(b[0], b[1], b[2], 0),
            // BGR, and BGR with a tint byte (tint already applied)
            0x05 | 0x15 => Color::rgb8(b[2], b[1], b[0]),
            // Grayscale 0..255
            0x09 => Color::Gray {
                v: b[0] as f32 / 255.0,
            },
            // Registration colour prints on every plate; black on screen.
            0x14 => Color::BLACK,
            // Spot, Lab, HSB, HLS, YIQ and others: fall back to black and note it.
            other => {
                self.report.warn(format!(
                    "colour model 0x{other:02x} not modelled; using black"
                ));
                Color::BLACK
            }
        })
    }

    /// `fild` (`fill` before version 7): fill definition. We model solid
    /// fills, fountain fills with all their stops, two-colour patterns,
    /// colour bitmap fills and textures (from their stored bitmap); vector
    /// pattern and PostScript fills become "no fill" with a warning.
    fn read_fill(&mut self, c: &Chunk) {
        let d = self.data(c);
        let mut r = self.reader(d);
        let Some(id) = r.u32() else { return };
        let v = self.v;
        let mut body_end = d.len();
        if v >= 13 {
            // X3+: a "since version" tag (1300) and the body length.
            let _since = r.u32();
            if let Some(len) = r.u32() {
                body_end = body_end.min(r.pos.saturating_add(len as usize));
            }
        }
        let Some(ftype) = r.u16() else { return };
        let Some(body) = d.get(r.pos..body_end.max(r.pos)) else {
            return;
        };
        let fill = match ftype {
            0 => Fill::None,
            1 => self.read_solid_fill(body).unwrap_or_else(|| {
                self.report
                    .warn(format!("solid fill {id} has no readable colour"));
                Fill::None
            }),
            2 => self.read_fountain_fill(body).unwrap_or_else(|| {
                self.report
                    .warn(format!("fountain fill {id} has no readable stops"));
                Fill::None
            }),
            7 | 8 => self.read_pattern_fill(body, id).unwrap_or(Fill::None),
            9 | 11 => self.read_image_fill(body, id, ftype).unwrap_or_else(|| {
                self.report
                    .warn(format!("image fill {id} (type {ftype}) is unreadable"));
                Fill::None
            }),
            10 => {
                self.report.warn(format!(
                    "vector pattern fill {id} not modelled yet; no fill"
                ));
                Fill::None
            }
            other => {
                self.report
                    .warn(format!("fill type {other} not modelled yet (fill id {id})"));
                Fill::None
            }
        };
        self.fills.insert(id, FillDef { fill });
    }

    /// Solid fill body. Before X3: 2 unknown bytes then a colour. From X3:
    /// a tag (1300), a length, then a property list of (type u8, len u32,
    /// body) records ending with type 0; type 1 is the colour.
    fn read_solid_fill(&mut self, body: &'a [u8]) -> Option<Fill> {
        let mut r = self.reader(body);
        if self.v < 13 {
            r.skip(2);
            return self.read_color(&mut r).map(Fill::Solid);
        }
        let _since = r.u32()?;
        let _len = r.u32()?;
        let mut color = None;
        for _ in 0..64 {
            let ptype = r.u8()?;
            if ptype == 0 {
                break;
            }
            let plen = r.u32()? as usize;
            let start = r.pos;
            if ptype == 0x01 && plen == 12 {
                let mut cr = self.reader_at(body, start);
                color = self.read_color(&mut cr);
            }
            r.pos = start.checked_add(plen)?;
            if color.is_some() {
                break;
            }
        }
        color.map(Fill::Solid)
    }

    /// Fountain fill body, see `docs/cdr-format.md` for the layout.
    fn read_fountain_fill(&mut self, body: &'a [u8]) -> Option<Fill> {
        let v = self.v;
        let mut r = self.reader(body);
        r.skip(if v >= 13 { 8 } else { 2 });
        let ftype = r.u8()?;
        r.skip(if v >= 13 {
            17
        } else if v >= 6 {
            19
        } else {
            11
        });
        let edge = if (6..13).contains(&v) {
            r.i32()?
        } else {
            r.i16()? as i32
        };
        let angle = r.angle()?;
        let cx = r.sint()?;
        let cy = r.sint()?;
        if v >= 6 {
            r.skip(2);
        }
        let _mode = r.uint()?;
        let _mid_point = r.u8()?;
        r.skip(1);
        let num_stops = (r.uint()? & 0xffff) as usize;
        if v >= 13 {
            r.skip(3);
        }
        let mut stops = Vec::with_capacity(num_stops.min(64));
        for _ in 0..num_stops.min(64) {
            let color = self.read_color(&mut r)?;
            r.skip(if v >= 15 {
                26
            } else if v >= 13 {
                5
            } else {
                0
            });
            let pos = (r.uint()? & 0xffff) as f64 / 100.0;
            if v >= 13 {
                r.skip(3);
            }
            stops.push(Stop {
                pos: pos.clamp(0.0, 1.0),
                color,
            });
        }
        if stops.is_empty() {
            return None;
        }
        if stops.len() == 1 {
            let only = stops[0];
            stops.push(Stop {
                pos: 1.0,
                color: only.color,
            });
            stops[0].pos = 0.0;
        }
        stops.sort_by(|a, b| a.pos.total_cmp(&b.pos));
        let kind = match ftype {
            2 => FountainKind::Radial,
            3 => FountainKind::Conical,
            4 => FountainKind::Square,
            _ => FountainKind::Linear,
        };
        let mut offset = Point::new(
            (cx as f64 / 100.0).clamp(-1.0, 1.0),
            (cy as f64 / 100.0).clamp(-1.0, 1.0),
        );
        let mut edge_pad = (edge as f64 / 100.0).clamp(0.0, 0.49);
        // X6+: four doubles relative to the object (centre x, centre y,
        // width, height) follow when the body is long enough.
        if v >= 16 {
            r.skip(3);
            if r.remaining() >= 32 {
                let (Some(ox), Some(oy), Some(w), Some(h)) = (r.f64(), r.f64(), r.f64(), r.f64())
                else {
                    return None;
                };
                if [ox, oy, w, h].iter().all(|x| x.is_finite()) {
                    if cx == 0 && cy == 0 && (ox != 0.0 || oy != 0.0) {
                        offset = Point::new(ox.clamp(-1.0, 1.0), oy.clamp(-1.0, 1.0));
                    }
                    // A fill narrower than its object leaves a padded edge.
                    let extent = w.min(h);
                    if edge == 0 && extent > 0.0 && extent < 1.0 {
                        edge_pad = ((1.0 - extent) / 2.0).clamp(0.0, 0.49);
                    } else if extent > 1.0 + 1e-9 {
                        self.report
                            .warn("fountain fill larger than its object; drawn at object size");
                    }
                }
            }
        }
        Some(Fill::Fountain(Fountain {
            kind,
            stops,
            angle,
            offset,
            edge_pad,
        }))
    }

    /// Colour bitmap (9) and texture (11) fill body. X3+ bodies start with
    /// optional records (a 0x640 word followed by 0x640 bytes, or a bare
    /// 0x514 word); then the
    /// pattern id, tile width and height, tile offsets or 4 unknown bytes,
    /// rcp offset, flags, 21 (17 from X3) unknown bytes and the pattern id
    /// again (version 6+). The pattern id names a `bmp ` image.
    fn read_image_fill(&mut self, body: &'a [u8], id: u32, ftype: u16) -> Option<Fill> {
        let v = self.v;
        let mut r = self.reader(body);
        if v >= 13 {
            for _ in 0..64 {
                let Some(tag) = self.reader_at(body, r.pos).u32() else {
                    break;
                };
                // The 0x640 word is a block length (1600 bytes follow);
                // 0x514 is a bare version tag.
                if tag == 0x640 {
                    r.skip(4 + 0x640);
                } else if tag == 0x514 {
                    r.skip(4);
                } else {
                    break;
                }
            }
        } else {
            r.skip(2);
        }
        let pattern_id = r.uint()?;
        let w = r.uint()?;
        let _h = r.uint()?;
        r.skip(4); // tile offsets (before 9) or unknown
        let _rcp = r.u16()?;
        let flags = r.u8()?;
        r.skip(if v >= 13 { 17 } else { 21 });
        let pattern_id = if v >= 6 {
            r.u32().unwrap_or(pattern_id)
        } else {
            pattern_id
        };
        let relative = flags & 0x04 != 0 && v < 9;
        let unit = if v < 6 { UNIT16_MM } else { UNIT_MM };
        let mut size_mm = if relative { 10.0 } else { w as f64 * unit };
        if !(0.01..10_000.0).contains(&size_mm) {
            size_mm = 10.0;
        }
        let Some(bm) = self.bitmaps.get(&pattern_id) else {
            if ftype == 11 {
                self.report.warn(format!(
                    "texture fill {id} has no bitmap {pattern_id}; a built-in texture is used"
                ));
                return Some(Fill::Texture(Texture {
                    kind: TextureKind::Clouds,
                    color_a: Color::rgb8(200, 200, 200),
                    color_b: Color::rgb8(90, 90, 90),
                    scale: size_mm,
                    seed: pattern_id,
                }));
            }
            self.report.warn(format!(
                "bitmap fill {id} refers to unknown image {pattern_id}; no fill"
            ));
            return None;
        };
        if ftype == 11 {
            self.report.warn(format!(
                "texture fill {id} drawn from its stored bitmap, not regenerated"
            ));
        }
        Some(Fill::Pattern(Pattern::Bitmap {
            png: bm.png.clone(),
            width_px: bm.width_px,
            height_px: bm.height_px,
            size_mm,
        }))
    }

    /// Two-colour pattern fill body. The 1-bit tile comes from a `bmpf`
    /// chunk with the same pattern id; without it a placeholder tile is used.
    fn read_pattern_fill(&mut self, body: &'a [u8], id: u32) -> Option<Fill> {
        let v = self.v;
        let mut r = self.reader(body);
        r.skip(if v >= 13 { 8 } else { 2 });
        let pattern_id = r.u32()?;
        let w = r.sint()?;
        let _h = r.sint()?;
        // Tile offsets (before 9) or 4 unknown bytes.
        r.skip(4);
        let _rcp = r.u16()?;
        let flags = r.u8()?;
        r.skip(if v >= 13 { 6 } else { 1 });
        let front = self.read_color(&mut r)?;
        r.skip(if v >= 16 {
            31
        } else if v >= 13 {
            10
        } else {
            0
        });
        let back = self.read_color(&mut r)?;
        let relative = flags & 0x04 != 0 && v < 9;
        let size_mm = if relative {
            10.0
        } else {
            let unit = if v < 6 { UNIT16_MM } else { UNIT_MM };
            (w as f64 * unit).abs()
        };
        let size_mm = if size_mm > 0.01 { size_mm } else { 10.0 };
        // With the tile bitmap at hand, paint it in the two colours.
        if let Some(tile) = self.pattern_tiles.get(&pattern_id) {
            let f = front.to_rgb8();
            let b = back.to_rgb8();
            let mut rgba = Vec::with_capacity(tile.front.len() * 4);
            for &on in &tile.front {
                let c = if on { f } else { b };
                rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
            return Some(Fill::Pattern(Pattern::Bitmap {
                png: encode_png(tile.width, tile.height, &rgba),
                width_px: tile.width,
                height_px: tile.height,
                size_mm,
            }));
        }
        self.report.warn(format!(
            "pattern {pattern_id} of fill {id} drawn with a placeholder tile"
        ));
        Some(Fill::Pattern(Pattern::TwoColor {
            tile: PatternTile::Checker,
            front,
            back,
            size_mm,
        }))
    }

    /// `bmpf`: the 1-bit tile of a two-colour pattern. Layout assumed (not
    /// in the public spec): pattern id u32, then a device-independent
    /// bitmap: a 40-byte info header (size 40, width, height, planes 1,
    /// bits 1), two palette entries and 1-bit rows padded to 4 bytes,
    /// bottom-up. The header is searched for in the first 64 bytes and
    /// validated; anything else is skipped with a warning.
    fn read_pattern_tile(&mut self, c: &Chunk) {
        let d = self.data(c);
        let mut r = self.reader(d);
        let Some(pattern_id) = r.u32() else { return };
        let mut header = None;
        for off in (4..d.len().min(68)).step_by(4) {
            let mut h = Reader::new(d, false);
            h.pos = off;
            if h.u32() == Some(40) {
                let (Some(w), Some(hgt), Some(planes), Some(bpp)) =
                    (h.i32(), h.i32(), h.u16(), h.u16())
                else {
                    break;
                };
                if planes == 1
                    && bpp == 1
                    && (1..=4096).contains(&w)
                    && (1..=4096).contains(&hgt.abs())
                {
                    header = Some((off, w as u32, hgt));
                    break;
                }
            }
        }
        let Some((off, width, height_raw)) = header else {
            self.report.warn(format!(
                "pattern tile {pattern_id}: no 1-bit bitmap header; placeholder used"
            ));
            return;
        };
        let top_down = height_raw < 0;
        let height = height_raw.unsigned_abs();
        // Palette: two BGRx entries right after the header. Black (or the
        // darker entry) is the front colour.
        let pal = off + 40;
        let lum = |i: usize| -> u32 {
            let p = d.get(pal + i * 4..pal + i * 4 + 3).unwrap_or(&[0, 0, 0]);
            p[0] as u32 + p[1] as u32 + p[2] as u32
        };
        let zero_is_front = lum(0) <= lum(1);
        let stride = (width as usize).div_ceil(32) * 4;
        let pixels_start = pal + 8;
        let Some(pixels) = d.get(pixels_start..) else {
            return;
        };
        if pixels.len() < stride * height as usize {
            self.report.warn(format!(
                "pattern tile {pattern_id}: {} pixel bytes, {} expected; placeholder used",
                pixels.len(),
                stride * height as usize
            ));
            return;
        }
        let mut front = Vec::with_capacity((width * height) as usize);
        for row in 0..height as usize {
            let src_row = if top_down {
                row
            } else {
                height as usize - 1 - row
            };
            let src = &pixels[src_row * stride..][..stride];
            for x in 0..width as usize {
                let bit = (src[x / 8] >> (7 - (x % 8))) & 1;
                front.push((bit == 0) == zero_is_front);
            }
        }
        self.pattern_tiles.insert(
            pattern_id,
            PatternBits {
                width,
                height,
                front,
            },
        );
    }

    /// `arrw`: an arrowhead definition. Layout assumed (not in the public
    /// spec): id u32, point count u32, then a point list as in geometry.
    /// The outline is classified into one of our presets by its shape.
    fn read_arrowhead(&mut self, c: &Chunk) {
        let d = self.data(c);
        let mut r = self.reader(d);
        let Some(id) = r.u32() else { return };
        let Some(n) = r.u32() else { return };
        let point_size = if self.v16() { 4 } else { 8 };
        if n == 0 || n as usize > r.remaining() / (point_size + 1) {
            self.report.warn(format!(
                "arrowhead {id}: unreadable outline; drawn without arrowhead"
            ));
            return;
        }
        let Some((pts, types)) = self.read_points(&mut r, n as usize) else {
            return;
        };
        let (path, _) = build_path(&pts, &types);
        let preset = classify_arrowhead(&path, &pts, &types);
        if preset == Arrowhead::None {
            self.report.warn(format!(
                "arrowhead {id}: shape not recognised; drawn without arrowhead"
            ));
        }
        self.arrows.insert(id, preset);
    }

    /// `font`: id and family name.
    fn read_font(&mut self, c: &Chunk) {
        let d = self.data(c);
        if let Some(f) = text::read_font(d, self.v) {
            self.fonts.insert(f.id, f.name);
        }
    }

    /// `stlt`: style records (default font, size, fill, alignment).
    fn read_styles(&mut self, c: &Chunk) {
        if self.v < 7 {
            // Before 7 the chunk is a list of sub-chunks the spec does not type.
            return;
        }
        let d = self.data(c);
        let mut warnings = Vec::new();
        let recs = text::read_stlt(d, self.v, &mut |w| warnings.push(w));
        for w in warnings {
            self.report.warn(w);
        }
        for (id, rec) in recs {
            self.styles.insert(id, rec);
        }
    }

    /// Properties of a style and its ancestors, nearest first.
    fn style_defaults(&self, style_id: u32) -> RunStyle {
        let mut out = RunStyle::default();
        let mut id = style_id;
        for _ in 0..16 {
            let Some(rec) = self.styles.get(&id) else {
                break;
            };
            out = out.over(&rec.style);
            if rec.parent == id || rec.parent == 0 {
                break;
            }
            id = rec.parent;
        }
        out
    }

    /// Resolve a run's properties into a span.
    fn make_span(&mut self, text: String, style: &RunStyle, object_fill: &Fill) -> TextSpan {
        let family = style
            .family
            .clone()
            .or_else(|| style.font_id.and_then(|id| self.fonts.get(&id).cloned()))
            .unwrap_or_else(|| {
                if let Some(id) = style.font_id {
                    self.report.warn(format!(
                        "font id {id} not in the font table; default family used"
                    ));
                }
                DEFAULT_FAMILY.to_string()
            });
        let size_pt = style
            .size_pt
            .filter(|s| s.is_finite() && *s > 0.0 && *s < 10_000.0)
            .unwrap_or(DEFAULT_SIZE_PT);
        let mut span = TextSpan::new(text, family, size_pt);
        if let Some(f) = style.flags {
            span.bold = f.bold();
            span.italic = f.italic();
            span.underline = f.underline();
            span.strikethrough = f.strikethrough();
        }
        if let Some(b) = style.bold {
            span.bold = b;
        }
        if let Some(i) = style.italic {
            span.italic = i;
        }
        if let Some(fid) = style.fill_id {
            match self.fills.get(&fid) {
                Some(f) if f.fill != *object_fill => span.fill = Some(f.fill.clone()),
                Some(_) => {}
                None => self
                    .report
                    .warn(format!("text run refers to unknown fill {fid}")),
            }
        }
        span
    }

    /// Text object (artistic 0x04 or paragraph 0x06): geometry from the
    /// coordinates argument, content from the `txsm` chunk inside the
    /// object's list (assumed placement), defaults from the `stlt` style.
    fn build_text(
        &mut self,
        kind_code: u32,
        d: &'a [u8],
        coords: Option<Arg>,
        oc: &Chunk,
        style_id: Option<u32>,
        object_fill: &Fill,
    ) -> Option<ShapeKind> {
        let mut origin = Point::ZERO;
        let mut frame: Option<Size> = None;
        if let Some(a) = coords {
            let data = d.get(..a.end)?;
            let mut r = self.reader_at(data, a.off);
            if kind_code == OBJ_PARAGRAPH_TEXT {
                r.skip(4);
                let w = r.coord()?;
                let h = r.coord()?;
                if w.is_finite() && h.is_finite() && w.abs() > 1e-6 && h.abs() > 1e-6 {
                    frame = Some(Size::new(w.abs(), h.abs()));
                }
            } else {
                origin = Point::new(r.coord()?, r.coord()?);
            }
        }
        let mut txsm = None;
        oc.walk(&mut |c, _| {
            if txsm.is_none() && c.is(b"txsm") {
                txsm = Some(c);
            }
        });
        let Some(tc) = txsm else {
            self.report
                .warn("text object without a txsm chunk; skipped");
            return None;
        };
        let td = self.data(tc);
        let mut warnings = Vec::new();
        let parsed = text::read_txsm(td, self.v, &mut |w| warnings.push(w));
        for w in warnings {
            self.report.warn(w);
        }
        let parsed = parsed?;
        if frame.is_none() && (parsed.frame || kind_code == OBJ_PARAGRAPH_TEXT) {
            // A frame without readable size: lay the text out unframed.
            self.report
                .warn("paragraph text without a readable frame; treated as artistic text");
        }
        let object_style = style_id
            .map(|id| self.style_defaults(id))
            .unwrap_or_default();
        let mut spans: Vec<TextSpan> = Vec::new();
        let mut align: Option<u32> = object_style.align;
        let n = parsed.paragraphs.len();
        for (i, para) in parsed.paragraphs.iter().enumerate() {
            let para_style = if para.style_id != 0 {
                self.style_defaults(para.style_id).over(&object_style)
            } else {
                object_style.clone()
            };
            let base = para.base.over(&para_style);
            if align.is_none() {
                align = base.align;
            }
            for (k, run) in para.runs.iter().enumerate() {
                let style = run.style.over(&base);
                let mut text = run.text.clone();
                if i + 1 < n && k + 1 == para.runs.len() {
                    text.push('\n');
                }
                let span = self.make_span(text, &style, object_fill);
                match spans.last_mut() {
                    Some(last)
                        if last.font_family == span.font_family
                            && last.size_pt == span.size_pt
                            && last.bold == span.bold
                            && last.italic == span.italic
                            && last.underline == span.underline
                            && last.strikethrough == span.strikethrough
                            && last.fill == span.fill =>
                    {
                        last.text.push_str(&span.text)
                    }
                    _ => spans.push(span),
                }
            }
        }
        if spans.is_empty() {
            self.report.warn("text object with no content; skipped");
            return None;
        }
        let align = match align.unwrap_or(0) {
            2 => TextAlign::Right,
            3 => TextAlign::Center,
            4 | 5 => TextAlign::Justify,
            _ => TextAlign::Left,
        };
        Some(ShapeKind::Text {
            spans,
            origin,
            frame,
            align,
            para: ParagraphStyle::default(),
            on_path: None,
        })
    }

    /// `outl`: outline definition, see `docs/cdr-format.md` for the layout.
    fn read_outline(&mut self, c: &Chunk) {
        let d = self.data(c);
        let mut r = self.reader(d);
        let Some(id) = r.u32() else { return };
        let v = self.v;
        if v >= 13 {
            // X3+: tagged records (id u32, len u32, body) until id 1,
            // whose body is the outline proper.
            let mut found = false;
            for _ in 0..64 {
                let (Some(tag), Some(len)) = (r.u32(), r.u32()) else {
                    break;
                };
                if tag == 1 {
                    found = true;
                    break;
                }
                r.skip(len as usize);
            }
            if !found {
                self.report
                    .warn(format!("outline {id}: no record of type 1; skipped"));
                return;
            }
        }
        let Some(line_type) = r.u16() else { return };
        let caps = r.u16().unwrap_or(0);
        let join = r.u16().unwrap_or(0);
        if (6..13).contains(&v) {
            r.skip(2);
        }
        let width = r.coord().unwrap_or(0.0);
        let stretch = r.u16().unwrap_or(100);
        if v >= 6 {
            r.skip(2);
        }
        let nib_angle = r.angle().unwrap_or(0.0);
        r.skip(if v >= 13 {
            46
        } else if v >= 6 {
            52
        } else {
            0
        });
        let color = self.read_color(&mut r);
        r.skip(if v < 6 { 10 } else { 16 });
        let mut dash = Vec::new();
        let mut markers = (0u32, 0u32);
        if let Some(n) = r.u16() {
            // The dash values occupy the start of a fixed 20/22-byte area
            // that the two marker ids follow.
            let dashes_at = r.pos;
            let n = (n as usize).min(r.remaining() / 2).min(64);
            for _ in 0..n {
                if let Some(x) = r.u16() {
                    dash.push(x as f64);
                }
            }
            let mut m = self.reader_at(d, dashes_at + if v < 6 { 20 } else { 22 });
            if let (Some(s), Some(e)) = (m.u32(), m.u32()) {
                markers = (s, e);
            }
        }
        // A dash pattern needs an even number of on/off entries.
        if dash.len() % 2 == 1 {
            dash.push(dash[dash.len() - 1]);
        }
        if dash.iter().all(|x| *x <= 0.0) {
            dash.clear();
        }
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
            s.dash = dash;
            s.stretch = (stretch as f64 / 100.0).clamp(0.01, 1.0);
            s.nib_angle = nib_angle;
            s.scale_with_object = line_type & 0x20 != 0;
            s.behind_fill = line_type & 0x10 != 0;
            s.start_arrow = self.marker_preset(markers.0, id);
            s.end_arrow = self.marker_preset(markers.1, id);
            Some(s)
        };
        self.outlines.insert(id, OutlineDef { stroke });
    }

    /// Arrowhead preset for a marker id; 0 means none.
    fn marker_preset(&mut self, marker: u32, outline: u32) -> Arrowhead {
        if marker == 0 {
            return Arrowhead::None;
        }
        match self.arrows.get(&marker) {
            Some(a) => *a,
            None => {
                self.report.warn(format!(
                    "outline {outline} uses arrowhead {marker} that has no definition; none drawn"
                ));
                Arrowhead::None
            }
        }
    }

    /// `trfd`: object transform (2x3 matrix). Returns identity when unreadable.
    fn read_transform(&mut self, c: &Chunk) -> Affine {
        let d = self.data(c);
        let v = self.v;
        let mut r = self.reader(d);
        // Header: length, argument count, start of the offsets table.
        let _len = r.uint();
        let num_args = r.uint().unwrap_or(0) as usize;
        let start_args = r.uint().unwrap_or(0) as usize;
        if num_args == 0 {
            return Affine::IDENTITY;
        }
        let step = if self.v16() { 2 } else { 4 };
        let mut result: Option<Affine> = None;
        for i in 0..num_args.min(16) {
            let Some(off) = self.reader_at(d, start_args + i * step).uint() else {
                break;
            };
            let mut tr = self.reader_at(d, off as usize);
            if v >= 13 {
                tr.skip(8);
            }
            // Record type 0x08 is an affine matrix; others are ignored.
            if tr.u16() != Some(0x08) {
                continue;
            }
            if v >= 6 {
                tr.skip(6);
            }
            let (Some(a), Some(c_), Some(e), Some(b), Some(d_), Some(f)) = (
                tr.f64(),
                tr.f64(),
                tr.f64_coord(),
                tr.f64(),
                tr.f64(),
                tr.f64_coord(),
            ) else {
                self.report.warn("trfd too short; identity used");
                return Affine::IDENTITY;
            };
            // Rows are (a c e; b d f): x' = a x + c y + e, y' = b x + d y + f.
            if ![a, b, c_, d_, e, f].iter().all(|x| x.is_finite()) {
                return Affine::IDENTITY;
            }
            let m = Affine::new([a, b, c_, d_, e, f]);
            if result.is_some() {
                self.report
                    .warn("trfd with several matrices; only the first is used");
                break;
            }
            result = Some(m);
        }
        result.unwrap_or(Affine::IDENTITY)
    }

    /// `loda` header and argument table. Header fields are u16 before
    /// version 6, u32 after: length, argument count, offset of the offsets
    /// table, offset of the types table, object type. The offsets table has
    /// one extra entry marking the end of the last argument, and the types
    /// table is stored in reverse order relative to the offsets.
    fn read_loda(&mut self, d: &'a [u8]) -> Option<Loda> {
        let mut r = self.reader(d);
        let _len = r.uint()?;
        let num_args = r.uint()? as usize;
        let start_args = r.uint()? as usize;
        let start_types = r.uint()? as usize;
        let kind = r.uint()?;
        let step = if self.v16() { 2 } else { 4 };
        let n = num_args.min(64);
        let mut offsets = Vec::with_capacity(n + 1);
        for i in 0..=n {
            match self.reader_at(d, start_args + i * step).uint() {
                Some(o) => offsets.push(o as usize),
                None if i == n => offsets.push(d.len()),
                None => return None,
            }
        }
        let mut args = Vec::with_capacity(n);
        for i in 0..n {
            let ty = self.reader_at(d, start_types + (n - 1 - i) * step).uint()?;
            let off = offsets[i].min(d.len());
            let next = offsets[i + 1];
            let end = if next > off && next <= d.len() {
                next
            } else {
                d.len()
            };
            args.push(Arg { ty, off, end });
        }
        Some(Loda { kind, args })
    }

    fn read_polygon_info(&mut self, d: &'a [u8], arg: Arg) -> Option<PolygonInfo> {
        let mut r = self.reader_at(d, arg.off);
        if self.v < 13 {
            r.skip(4);
        }
        let num_angles = r.u32()?;
        // "next point" fields; the second u32 is a second next point when
        // the first is 0 or 1, unknown data otherwise. Either way 4 bytes.
        let _next_point1 = r.u32()?;
        let _next_point2 = r.u32()?;
        if self.v >= 13 {
            r.skip(4);
        }
        let rx = r.f64()?;
        let ry = r.f64()?;
        let cx = r.coord()?;
        let cy = r.coord()?;
        if !rx.is_finite() || !ry.is_finite() {
            return None;
        }
        Some(PolygonInfo {
            num_angles,
            rx: rx * UNIT_MM,
            ry: ry * UNIT_MM,
            cx,
            cy,
        })
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
            if (c.is(b"loda") || c.is(b"lobj")) && loda.is_none() {
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
        let l = self.read_loda(d)?;
        let kind_code = l.kind;

        let mut fill = Fill::None;
        let mut stroke: Option<Stroke> = None;
        let mut name: Option<String> = None;
        let mut coords: Option<Arg> = None;
        let mut polygon: Option<PolygonInfo> = None;
        let mut style_id: Option<u32> = None;
        let mut opacity = 1.0;

        for arg in &l.args {
            match arg.ty {
                ARG_STYLE => style_id = self.reader_at(d, arg.off).uint(),
                ARG_OPACITY => {
                    if let Some(o) = self.read_opacity(d, *arg) {
                        opacity = o;
                    }
                }
                // Before version 4 the fill and outline are stored inline.
                ARG_FILL if self.v >= 4 => {
                    if let Some(fid) = self.reader_at(d, arg.off).u32() {
                        if let Some(f) = self.fills.get(&fid) {
                            fill = f.fill.clone();
                        }
                    }
                }
                ARG_OUTLINE if self.v >= 4 => {
                    if let Some(oid) = self.reader_at(d, arg.off).u32() {
                        if let Some(o) = self.outlines.get(&oid) {
                            stroke = o.stroke.clone();
                        }
                    }
                }
                ARG_COORDS => coords = Some(*arg),
                ARG_NAME => {
                    if let Some(s) = d.get(arg.off..arg.end) {
                        let s = read_name(s, self.v);
                        if !s.is_empty() {
                            name = Some(s);
                        }
                    }
                }
                ARG_POLYGON => polygon = self.read_polygon_info(d, *arg),
                _ => {}
            }
        }

        let kind = match kind_code {
            OBJ_ARTISTIC_TEXT | OBJ_PARAGRAPH_TEXT => {
                self.build_text(kind_code, d, coords, oc, style_id, &fill)
            }
            _ => coords.and_then(|a| self.read_geometry(kind_code, d, a, polygon)),
        };
        let kind = match kind {
            Some(k) => k,
            None => {
                match kind_code {
                    OBJ_ARTISTIC_TEXT | OBJ_PARAGRAPH_TEXT => {}
                    other => self
                        .report
                        .warn(format!("object type 0x{other:02x} not modelled yet")),
                }
                return None;
            }
        };

        // The page origin is the page centre; ours is the bottom-left.
        let recenter = Affine::translate((page_size.width / 2.0, page_size.height / 2.0));
        let mut shape = Shape::new(id, kind);
        shape.transform = recenter * transform;
        shape.fill = fill;
        shape.stroke = stroke;
        shape.name = name;
        shape.opacity = opacity;
        Some(shape)
    }

    /// Opacity argument (8000): 10 unknown bytes (14 from X3), then a u16
    /// in thousandths. The spec does not say whether the value is opacity
    /// or transparency; the target design's slider is a transparency
    /// (0 = opaque), so we read it that way. Values above 1.0 are taken as
    /// percent in thousandths.
    fn read_opacity(&mut self, d: &'a [u8], arg: Arg) -> Option<f64> {
        let data = d.get(..arg.end)?;
        let mut r = self.reader_at(data, arg.off);
        r.skip(if self.v >= 13 { 14 } else { 10 });
        let raw = r.u16()? as f64;
        let mut t = raw / 1000.0;
        if t > 1.0 {
            t /= 100.0;
        }
        if !(0.0..=1.0).contains(&t) {
            self.report.warn(format!(
                "opacity value {raw} out of range; object drawn opaque"
            ));
            return None;
        }
        Some(1.0 - t)
    }

    /// Point list: `n` points of (x, y) followed by one type byte each.
    /// `n` is clamped to what the payload can hold.
    fn read_points(&mut self, r: &mut Reader, n: usize) -> Option<(Vec<Point>, Vec<u8>)> {
        let point_size = if self.v16() { 4 } else { 8 };
        let n = n.min(r.remaining() / (point_size + 1)).min(1_000_000);
        if n == 0 {
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
        Some((pts, types))
    }

    fn read_geometry(
        &mut self,
        kind_code: u32,
        d: &'a [u8],
        arg: Arg,
        polygon: Option<PolygonInfo>,
    ) -> Option<ShapeKind> {
        let v = self.v;
        let data = d.get(..arg.end)?;
        let mut r = self.reader_at(data, arg.off);
        match kind_code {
            // Rectangle: width, height from (0,0), then corner radii.
            OBJ_RECT => {
                let (w, h, radii) = if v < 15 {
                    let w = r.coord()?;
                    let h = r.coord()?;
                    let mut radii = vec![r.coord().unwrap_or(0.0)];
                    if v >= 9 {
                        for _ in 0..3 {
                            if let Some(x) = r.coord() {
                                radii.push(x);
                            }
                        }
                    }
                    (w, h, radii)
                } else {
                    // X5+: doubles in coordinate units, scale factors, and
                    // four radii each followed by corner data we skip.
                    let w = r.f64_coord()?;
                    let h = r.f64_coord()?;
                    let sx = r.f64().unwrap_or(1.0);
                    let sy = r.f64().unwrap_or(1.0);
                    let scale_with = r.u8().unwrap_or(0);
                    r.skip(7);
                    let unit = if scale_with == 0 { UNIT_MM } else { 25.4 };
                    // Each radius is followed by 16 bytes of corner data
                    // (corner type and unknown fields) except the last.
                    let mut radii = Vec::new();
                    for gap in [16usize, 16, 16, 0] {
                        let Some(raw) = r.f64() else { break };
                        if raw.is_finite() {
                            radii.push(raw * unit);
                        }
                        r.skip(gap);
                    }
                    let (sx, sy) = if sx.is_finite() && sy.is_finite() && sx != 0.0 && sy != 0.0 {
                        (sx, sy)
                    } else {
                        (1.0, 1.0)
                    };
                    (w * sx, h * sy, radii)
                };
                let radius = radii.iter().cloned().fold(0.0_f64, f64::max);
                if radii.iter().any(|r| (r - radius).abs() > 1e-6) {
                    self.report
                        .warn("rectangle with per-corner radii; largest used");
                }
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
            // Ellipse: width, height, start/end angle, pie flag.
            OBJ_ELLIPSE => {
                let w = r.coord()?;
                let h = r.coord()?;
                let a1 = r.angle().unwrap_or(0.0);
                let a2 = r.angle().unwrap_or(0.0);
                let pie = r.uint().unwrap_or(0) != 0;
                let rect = Rect::new(
                    0.0_f64.min(w),
                    0.0_f64.min(h),
                    0.0_f64.max(w),
                    0.0_f64.max(h),
                );
                let arc = if (a1 - a2).abs() > 1e-9 {
                    Some(EllipseArc {
                        start_deg: a1,
                        end_deg: a2,
                        pie,
                    })
                } else {
                    None
                };
                Some(ShapeKind::Ellipse { rect, arc })
            }
            // Curve: u32 count, points, one type byte per point.
            OBJ_CURVE => {
                let n = r.u32()? as usize;
                let (pts, types) = self.read_points(&mut r, n)?;
                let (path, closed) = build_path(&pts, &types);
                if path.elements().is_empty() {
                    return None;
                }
                Some(ShapeKind::Path { path, closed })
            }
            // Path (X6+): 4 unknown, two u16 counts, 16 unknown, points.
            OBJ_PATH => {
                r.skip(4);
                let n1 = r.u16()? as usize;
                let n2 = r.u16()? as usize;
                r.skip(16);
                let (pts, types) = self.read_points(&mut r, n1 + n2)?;
                let (path, closed) = build_path(&pts, &types);
                if path.elements().is_empty() {
                    return None;
                }
                Some(ShapeKind::Path { path, closed })
            }
            // Polygon: the coordinates hold the base curve; the polygon
            // argument gives the number of angles, radii and centre.
            OBJ_POLYGON => {
                let n = r.u32()? as usize;
                let (pts, _types) = self.read_points(&mut r, n)?;
                let (rect, points) = match polygon {
                    Some(p) if p.rx.abs() > 1e-9 && p.ry.abs() > 1e-9 => (
                        Rect::new(
                            p.cx - p.rx.abs(),
                            p.cy - p.ry.abs(),
                            p.cx + p.rx.abs(),
                            p.cy + p.ry.abs(),
                        ),
                        p.num_angles.clamp(3, 500),
                    ),
                    Some(p) => (bounds_of(&pts)?, p.num_angles.clamp(3, 500)),
                    None => (bounds_of(&pts)?, 5),
                };
                Some(ShapeKind::Polygon {
                    rect,
                    points,
                    sharpness: 0.0,
                })
            }
            // Bitmap: two corners, 32 unknown bytes, image id, then a
            // clipping outline we ignore.
            OBJ_BITMAP => {
                let x1 = r.coord()?;
                let y1 = r.coord()?;
                let x2 = r.coord()?;
                let y2 = r.coord()?;
                r.skip(32);
                let image_id = r.u32()?;
                let Some(bm) = self.bitmaps.get(&image_id) else {
                    self.report
                        .warn(format!("bitmap object refers to unknown image {image_id}"));
                    return None;
                };
                Some(ShapeKind::Bitmap {
                    rect: Rect::new(x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2)),
                    width_px: bm.width_px,
                    height_px: bm.height_px,
                    png: bm.png.clone(),
                })
            }
            _ => None,
        }
    }

    /// `bmp `: embedded bitmap. Header fields are at fixed positions after
    /// a version dependent gap; pixels are a device-independent bitmap body
    /// (rows padded to 4 bytes, bottom-up). We re-encode as PNG.
    fn read_bitmap(&mut self, c: &Chunk) {
        let d = self.data(c);
        let v = self.v;
        let mut r = self.reader(d);
        let Some(image_id) = r.uint() else { return };
        r.skip(if v < 6 {
            14
        } else if v < 7 {
            46
        } else {
            50
        });
        let (Some(color_model), _, Some(width), Some(height), _, Some(bpp), _, Some(size)) = (
            r.u32(),
            r.skip(4),
            r.u32(),
            r.u32(),
            r.skip(4),
            r.u32(),
            r.skip(4),
            r.u32(),
        ) else {
            return;
        };
        r.skip(32);
        if width == 0 || height == 0 || width > 32_768 || height > 32_768 {
            self.report
                .warn(format!("bitmap {image_id}: size {width}x{height} rejected"));
            return;
        }
        let mut palette: Vec<[u8; 3]> = Vec::new();
        if bpp < 24 && color_model != 5 && color_model != 6 {
            r.skip(2);
            let n = r.u16().unwrap_or(0) as usize;
            let n = n.min(r.remaining() / 3).min(256);
            for _ in 0..n {
                let (Some(b), Some(g), Some(rr)) = (r.u8(), r.u8(), r.u8()) else {
                    break;
                };
                palette.push([rr, g, b]);
            }
        }
        let stride = ((width as usize * bpp as usize + 31) / 32) * 4;
        let needed = stride.saturating_mul(height as usize);
        let Some(pixels) = d.get(r.pos..r.pos.saturating_add(size as usize)) else {
            return;
        };
        if pixels.len() < needed {
            self.report.warn(format!(
                "bitmap {image_id}: {} pixel bytes, {needed} expected; skipped",
                pixels.len()
            ));
            return;
        }
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for row in 0..height as usize {
            // Bottom-up rows.
            let src = &pixels[(height as usize - 1 - row) * stride..][..stride];
            for x in 0..width as usize {
                let px: [u8; 4] = match bpp {
                    32 => [src[x * 4 + 2], src[x * 4 + 1], src[x * 4], 255],
                    24 => [src[x * 3 + 2], src[x * 3 + 1], src[x * 3], 255],
                    8 => {
                        let i = src[x] as usize;
                        match palette.get(i) {
                            Some(c) => [c[0], c[1], c[2], 255],
                            None => [src[x], src[x], src[x], 255],
                        }
                    }
                    1 => {
                        let bit = (src[x / 8] >> (7 - (x % 8))) & 1;
                        match palette.get(bit as usize) {
                            Some(c) => [c[0], c[1], c[2], 255],
                            None => {
                                let g = if bit == 0 { 0 } else { 255 };
                                [g, g, g, 255]
                            }
                        }
                    }
                    other => {
                        self.report.warn(format!(
                            "bitmap {image_id}: {other} bits per pixel not supported"
                        ));
                        return;
                    }
                };
                rgba.extend_from_slice(&px);
            }
        }
        let png = encode_png(width, height, &rgba);
        self.bitmaps.insert(
            image_id,
            BitmapDef {
                width_px: width,
                height_px: height,
                png,
            },
        );
    }
}

fn bounds_of(pts: &[Point]) -> Option<Rect> {
    let first = pts.first()?;
    let mut r = Rect::new(first.x, first.y, first.x, first.y);
    for p in pts {
        r.x0 = r.x0.min(p.x);
        r.y0 = r.y0.min(p.y);
        r.x1 = r.x1.max(p.x);
        r.y1 = r.y1.max(p.y);
    }
    Some(r)
}

/// Object and layer names: NUL-terminated single-byte text before version
/// 12 (the system code page; we read it as Windows-1252), UTF-16LE after.
fn read_name(b: &[u8], v: u16) -> String {
    text::decode_text(b, v)
}

/// Sort an arrowhead outline into one of our presets by its shape: a
/// closed curve made only of Bezier segments is a circle; three corners
/// make an arrow (filled) or an open arrow (not closed); four corners make
/// a square (axis-aligned, near 1:1), a bar (thin) or a diamond (rotated).
fn classify_arrowhead(path: &BezPath, pts: &[Point], types: &[u8]) -> Arrowhead {
    use tracedraw_core::geometry::Shape as _;
    if path.elements().is_empty() {
        return Arrowhead::None;
    }
    let closed = types.iter().any(|t| t & 0x08 != 0 && t >> 6 != 0);
    let has_curve = types.iter().any(|t| t >> 6 == 0b10);
    // Corners: end points of lines and moves, ignoring repeats.
    let mut corners: Vec<Point> = Vec::new();
    for (p, t) in pts.iter().zip(types) {
        if t >> 6 == 0b11 {
            continue;
        }
        if corners.iter().all(|c| (*c - *p).hypot() > 1e-6) {
            corners.push(*p);
        }
    }
    let bounds = path.bounding_box();
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return Arrowhead::None;
    }
    if has_curve {
        let all_curves = !types.iter().any(|t| *t >> 6 == 0b01);
        if all_curves && closed {
            return Arrowhead::Circle;
        }
        // Curved arrowheads with straight parts: treat as a filled arrow.
        return Arrowhead::Arrow;
    }
    match corners.len() {
        3 => {
            if closed {
                Arrowhead::Arrow
            } else {
                Arrowhead::OpenArrow
            }
        }
        4 => {
            // Axis-aligned when every corner shares x or y with two others.
            let aligned = corners.iter().all(|c| {
                corners
                    .iter()
                    .filter(|o| (o.x - c.x).abs() < 1e-6 || (o.y - c.y).abs() < 1e-6)
                    .count()
                    >= 3
            });
            let aspect = bounds.width() / bounds.height();
            if aligned {
                if !(0.35..=1.0 / 0.35).contains(&aspect) {
                    Arrowhead::Bar
                } else {
                    Arrowhead::Square
                }
            } else {
                Arrowhead::Diamond
            }
        }
        _ => Arrowhead::None,
    }
}

/// Point type byte: bits 7..6 are the operation (`00` move to, `01` line
/// to, `10` cubic Bezier end point preceded by two control points, `11`
/// control point); bit 3 marks the segment that closes the subpath (it is
/// also set on the move-to of a closed subpath, which we ignore).
fn build_path(pts: &[Point], types: &[u8]) -> (BezPath, bool) {
    let mut path = BezPath::new();
    let mut closed = false;
    let mut ctrl: Vec<Point> = Vec::with_capacity(2);
    let mut started = false;
    let mut start = Point::ZERO;
    for (p, t) in pts.iter().zip(types) {
        let closing = t & 0x08 != 0;
        match t >> 6 {
            0b00 => {
                path.move_to(*p);
                start = *p;
                started = true;
                ctrl.clear();
            }
            0b11 => ctrl.push(*p),
            op => {
                if !started {
                    path.move_to(*p);
                    start = *p;
                    started = true;
                } else if op == 0b10 && ctrl.len() >= 2 {
                    path.curve_to(ctrl[0], ctrl[1], *p);
                } else if op == 0b10 && ctrl.len() == 1 {
                    path.quad_to(ctrl[0], *p);
                } else if !(closing && (*p - start).hypot() < 1e-9) {
                    path.line_to(*p);
                }
                ctrl.clear();
                if closing {
                    path.close_path();
                    closed = true;
                }
            }
        }
    }
    (path, closed)
}

/// Minimal PNG encoder: 8-bit RGBA, no filtering, one IDAT chunk.
fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    use std::io::Write;
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xffff_ffffu32;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            }
        }
        !crc
    }
    fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], body: &[u8]) {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        let mut c = tag.to_vec();
        c.extend_from_slice(body);
        out.extend_from_slice(&c);
        out.extend_from_slice(&crc32(&c).to_be_bytes());
    }
    let mut raw = Vec::with_capacity((width as usize * 4 + 1) * height as usize);
    let row = width as usize * 4;
    for y in 0..height as usize {
        raw.push(0);
        let line = rgba.get(y * row..(y + 1) * row).unwrap_or(&[]);
        raw.extend_from_slice(line);
        raw.resize((row + 1) * (y + 1), 0);
    }
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    let idat = match enc.write_all(&raw).and_then(|_| enc.finish()) {
        Ok(v) => v,
        Err(_) => Vec::new(),
    };
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &idat);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Shape as _;

    // Byte-building helpers: RIFF chunks and typical payloads.

    fn chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(payload);
        if payload.len() % 2 == 1 {
            v.push(0);
        }
        v
    }

    fn list(lt: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut payload = lt.to_vec();
        payload.extend_from_slice(body);
        chunk(b"LIST", &payload)
    }

    fn riff(form: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut payload = form.to_vec();
        payload.extend_from_slice(body);
        chunk(b"RIFF", &payload)
    }

    fn u16s(v: &[u16]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }
    fn u32s(v: &[u32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }
    fn i32s(v: &[i32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }
    fn f64s(v: &[f64]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    /// 12-byte colour record (version 5+): BGR model 5.
    fn color_bgr(r: u8, g: u8, b: u8) -> Vec<u8> {
        let mut v = u16s(&[5, 0]);
        v.extend_from_slice(&[0xaa; 4]); // unknown bytes must be skipped
        v.extend_from_slice(&[b, g, r, 0]);
        v
    }

    fn color_cmyk_pct(c: u8, m: u8, y: u8, k: u8) -> Vec<u8> {
        let mut v = u16s(&[2, 0]);
        v.extend_from_slice(&[0; 4]);
        v.extend_from_slice(&[c, m, y, k]);
        v
    }

    /// A `loda` payload (32-bit header) with the given object type and
    /// arguments (type, body). Types are written reversed, as in files.
    fn loda(kind: u32, args: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let header = 20usize;
        let mut bodies = Vec::new();
        let mut offsets = Vec::new();
        for (_, b) in args {
            offsets.push((header + bodies.len()) as u32);
            bodies.extend_from_slice(b);
            while bodies.len() % 4 != 0 {
                bodies.push(0);
            }
        }
        let start_args = header + bodies.len();
        offsets.push(start_args as u32);
        let start_types = start_args + offsets.len() * 4;
        let len = start_types + args.len() * 4;
        let mut out = u32s(&[
            len as u32,
            args.len() as u32,
            start_args as u32,
            start_types as u32,
            kind,
        ]);
        out.extend_from_slice(&bodies);
        out.extend_from_slice(&u32s(&offsets));
        let types: Vec<u32> = args.iter().rev().map(|(t, _)| *t).collect();
        out.extend_from_slice(&u32s(&types));
        assert_eq!(out.len(), len);
        out
    }

    /// Same, with the 16-bit header of versions before 6.
    fn loda16(kind: u16, args: &[(u16, Vec<u8>)]) -> Vec<u8> {
        let header = 10usize;
        let mut bodies = Vec::new();
        let mut offsets = Vec::new();
        for (_, b) in args {
            offsets.push((header + bodies.len()) as u16);
            bodies.extend_from_slice(b);
            while bodies.len() % 2 != 0 {
                bodies.push(0);
            }
        }
        let start_args = header + bodies.len();
        offsets.push(start_args as u16);
        let start_types = start_args + offsets.len() * 2;
        let len = start_types + args.len() * 2;
        let mut out = u16s(&[
            len as u16,
            args.len() as u16,
            start_args as u16,
            start_types as u16,
            kind,
        ]);
        out.extend_from_slice(&bodies);
        out.extend_from_slice(&u16s(&offsets));
        let types: Vec<u16> = args.iter().rev().map(|(t, _)| *t).collect();
        out.extend_from_slice(&u16s(&types));
        out
    }

    /// A `trfd` payload with one matrix record, for the given version.
    fn trfd(v: u16, m: [f64; 6]) -> Vec<u8> {
        let mut rec = Vec::new();
        if v >= 13 {
            rec.extend_from_slice(&[0; 8]);
        }
        rec.extend_from_slice(&8u16.to_le_bytes());
        if v >= 6 {
            rec.extend_from_slice(&[0; 6]);
        }
        rec.extend_from_slice(&f64s(&m));
        // header (12) + offsets table (4) + record
        let mut out = u32s(&[0, 1, 12, 16]);
        out.extend_from_slice(&rec);
        out
    }

    fn mcfg(v: u16, w: i32, h: i32) -> Vec<u8> {
        let pad = if v >= 13 {
            12
        } else if v >= 9 {
            4
        } else if v == 6 {
            28
        } else {
            0
        };
        let mut out = vec![0u8; pad];
        out.extend_from_slice(&i32s(&[w, h]));
        out
    }

    fn parse(file: &[u8], v: u16) -> (Document, ParseReport) {
        let tree = crate::riff::parse(file).expect("riff");
        parse_document(&tree, file, Version(v))
    }

    fn first_shape(doc: &Document) -> &Shape {
        doc.pages
            .iter()
            .flat_map(|p| &p.layers)
            .flat_map(|l| &l.shapes)
            .next()
            .expect("one shape")
    }

    #[test]
    fn path_builder_handles_moves_lines_and_curves() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(12.0, 5.0),
            Point::new(8.0, 10.0),
            Point::new(0.0, 10.0),
            Point::new(0.0, 0.0),
        ];
        // move (closed subpath flag), line, control, control, curve end,
        // closing line back to the start.
        let types = [0x08, 0x40, 0xC0, 0xC0, 0x80, 0x48];
        let (p, closed) = build_path(&pts, &types);
        assert!(closed);
        assert_eq!(p.elements().len(), 4); // move, line, curve, close
        let b = p.bounding_box();
        assert!(b.x1 >= 10.0 && b.y1 >= 10.0);
    }

    #[test]
    fn unit_conversion() {
        assert!((254000.0 * UNIT_MM - 25.4).abs() < 1e-9);
        assert!((1000.0 * UNIT16_MM - 25.4).abs() < 1e-9);
    }

    #[test]
    fn color_record_skips_four_unknown_bytes() {
        let tree = crate::riff::parse(&riff(b"CDR9", &[])).unwrap();
        let main = riff(b"CDR9", &[]);
        let mut ctx = Ctx::new(&main, &tree, 9);
        let bytes = color_bgr(255, 128, 0);
        let mut r = Reader::new(&bytes, false);
        assert_eq!(ctx.read_color(&mut r), Some(Color::rgb8(255, 128, 0)));
        assert_eq!(r.pos, 12);
        // Model 0x11 is CMYK in 0..255, model 0x15 is BGR with a tint byte.
        let mut c17 = u16s(&[0x11, 0]);
        c17.extend_from_slice(&[0; 4]);
        c17.extend_from_slice(&[255, 0, 0, 0]);
        assert_eq!(
            ctx.read_color(&mut Reader::new(&c17, false)),
            Some(Color::cmyk8(255, 0, 0, 0))
        );
        let mut c21 = u16s(&[0x15, 0]);
        c21.extend_from_slice(&[0; 4]);
        c21.extend_from_slice(&[0, 0, 200, 50]);
        assert_eq!(
            ctx.read_color(&mut Reader::new(&c21, false)),
            Some(Color::rgb8(200, 0, 0))
        );
    }

    #[test]
    fn solid_fill_before_x3_has_two_unknown_bytes_before_the_colour() {
        let mut fild = u32s(&[7]);
        fild.extend_from_slice(&u16s(&[1, 0x1234]));
        fild.extend_from_slice(&color_cmyk_pct(0, 100, 100, 0));
        let obj = list(
            b"obj ",
            &[chunk(
                b"loda",
                &loda(
                    OBJ_RECT,
                    &[
                        (ARG_COORDS, i32s(&[254000, 127000, 0])),
                        (ARG_FILL, u32s(&[7])),
                    ],
                ),
            )]
            .concat(),
        );
        let body = [
            chunk(b"fild", &fild),
            chunk(b"mcfg", &mcfg(9, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDR9", &body), 9);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        let s = first_shape(&doc);
        assert_eq!(s.fill, Fill::Solid(Color::cmyk_pct(0.0, 100.0, 100.0, 0.0)));
        match &s.kind {
            ShapeKind::Rect { rect, radius } => {
                assert!((rect.width() - 25.4).abs() < 1e-9);
                assert!((rect.height() - 12.7).abs() < 1e-9);
                assert_eq!(*radius, 0.0);
            }
            other => panic!("{other:?}"),
        }
        // Page is 254 mm square; the rectangle sits at the centre.
        let p = s.transform * Point::ZERO;
        assert!((p.x - 127.0).abs() < 1e-9 && (p.y - 127.0).abs() < 1e-9);
    }

    #[test]
    fn solid_fill_x3_uses_a_property_list() {
        // id, since (1300), body length, type 1, since, properties length,
        // property 7 (palette guid, 16 bytes), property 1 (colour), end.
        let mut props = vec![0x07u8];
        props.extend_from_slice(&16u32.to_le_bytes());
        props.extend_from_slice(&[0xee; 16]);
        props.push(0x01);
        props.extend_from_slice(&12u32.to_le_bytes());
        props.extend_from_slice(&color_bgr(10, 20, 30));
        props.push(0x00);
        let mut solid = u32s(&[1300, props.len() as u32]);
        solid.extend_from_slice(&props);
        let mut fild = u32s(&[3, 1300, (solid.len() + 2) as u32]);
        fild.extend_from_slice(&u16s(&[1]));
        fild.extend_from_slice(&solid);
        fild.extend_from_slice(&[0; 8]); // trailing zeros seen in files
        let tree = crate::riff::parse(&riff(b"CDRD", &chunk(b"fild", &fild))).unwrap();
        let main = riff(b"CDRD", &chunk(b"fild", &fild));
        let (_, rep) = {
            let (d, r) = parse_document(&tree, &main, Version(13));
            (d, r)
        };
        assert!(
            rep.warnings.iter().all(|w| !w.contains("fill")),
            "{:?}",
            rep.warnings
        );
        // Re-run to inspect the fill table directly.
        let mut ctx = Ctx::new(&main, &tree, 13);
        let c = tree.root.find(b"fild").unwrap();
        ctx.read_fill(c);
        assert_eq!(ctx.fills[&3].fill, Fill::Solid(Color::rgb8(10, 20, 30)));
    }

    #[test]
    fn fountain_fill_version_12_reads_angle_and_all_stops() {
        // Version 12 (pre X3) layout.
        let mut g = vec![0u8; 2]; // unknown1
        g.push(2); // type: radial
        g.extend_from_slice(&[0; 19]); // unknown2
        g.extend_from_slice(&i32s(&[5])); // edge offset 5 %
        g.extend_from_slice(&i32s(&[45_000_000])); // angle 45 deg
        g.extend_from_slice(&i32s(&[10, -20])); // centre offsets
        g.extend_from_slice(&[0; 2]); // unknown3
        g.extend_from_slice(&u32s(&[0])); // mode
        g.push(50); // mid point
        g.push(0);
        g.extend_from_slice(&u32s(&[3])); // stops
        for (c, pos) in [((255, 0, 0), 0u32), ((0, 255, 0), 50), ((0, 0, 255), 100)] {
            g.extend_from_slice(&color_bgr(c.0, c.1, c.2));
            g.extend_from_slice(&u32s(&[pos]));
        }
        let mut fild = u32s(&[9]);
        fild.extend_from_slice(&u16s(&[2]));
        fild.extend_from_slice(&g);
        let main = riff(b"CDRC", &chunk(b"fild", &fild));
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 12);
        ctx.read_fill(tree.root.find(b"fild").unwrap());
        match &ctx.fills[&9].fill {
            Fill::Fountain(f) => {
                assert_eq!(f.kind, FountainKind::Radial);
                assert!((f.angle - 45.0).abs() < 1e-9);
                assert_eq!(f.stops.len(), 3);
                assert_eq!(f.stops[1].color, Color::rgb8(0, 255, 0));
                assert!((f.stops[1].pos - 0.5).abs() < 1e-9);
                assert!((f.offset.x - 0.1).abs() < 1e-9 && (f.offset.y + 0.2).abs() < 1e-9);
                assert!((f.edge_pad - 0.05).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn fountain_fill_x5_stop_size_is_45_bytes() {
        let v = 15u16;
        let mut g = vec![0u8; 8];
        g.push(1);
        g.extend_from_slice(&[0; 17]);
        g.extend_from_slice(&i16s(&[0])); // edge offset is s2 again from X3
        g.extend_from_slice(&i32s(&[90_000_000]));
        g.extend_from_slice(&i32s(&[0, 0]));
        g.extend_from_slice(&[0; 2]);
        g.extend_from_slice(&u32s(&[0]));
        g.extend_from_slice(&[50, 0]);
        g.extend_from_slice(&u32s(&[2]));
        g.extend_from_slice(&[0; 3]);
        let stops_start = g.len();
        for (c, pos) in [((0, 0, 0), 0u32), ((255, 255, 255), 100)] {
            g.extend_from_slice(&color_bgr(c.0, c.1, c.2));
            g.extend_from_slice(&[0; 26]);
            g.extend_from_slice(&u32s(&[pos]));
            g.extend_from_slice(&[0; 3]);
        }
        assert_eq!(g.len() - stops_start, 90);
        let mut fild = u32s(&[1, 1300, (g.len() + 2) as u32]);
        fild.extend_from_slice(&u16s(&[2]));
        fild.extend_from_slice(&g);
        let main = riff(b"CDRF", &chunk(b"fild", &fild));
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, v);
        ctx.read_fill(tree.root.find(b"fild").unwrap());
        match &ctx.fills[&1].fill {
            Fill::Fountain(f) => {
                assert_eq!(f.kind, FountainKind::Linear);
                assert!((f.angle - 90.0).abs() < 1e-9);
                assert_eq!(f.stops.len(), 2);
                assert_eq!(f.stops[1].color, Color::rgb8(255, 255, 255));
            }
            other => panic!("{other:?}"),
        }
    }

    fn i16s(v: &[i16]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    #[test]
    fn outline_x3_skips_tagged_records_and_reads_dashes() {
        let v = 13u16;
        let mut o = u32s(&[4]); // id
        o.extend_from_slice(&u32s(&[0x0a, 4, 0xdead_beef])); // record 10, skipped
        o.extend_from_slice(&u32s(&[1, 0])); // record 1: the outline
        o.extend_from_slice(&u16s(&[0x20, 1, 2])); // line type (scale), round cap, bevel join
        o.extend_from_slice(&i32s(&[2540])); // width 0.254 mm
        o.extend_from_slice(&u16s(&[50])); // stretch 50 %
        o.extend_from_slice(&[0; 2]);
        o.extend_from_slice(&i32s(&[30_000_000])); // nib angle 30 deg
        o.extend_from_slice(&[0; 46]);
        o.extend_from_slice(&color_bgr(0, 0, 255));
        o.extend_from_slice(&[0; 16]);
        o.extend_from_slice(&u16s(&[4, 3, 1, 1, 1])); // 4 dash entries
        o.extend_from_slice(&[0; 22]);
        o.extend_from_slice(&u32s(&[0, 0]));
        let main = riff(b"CDRD", &chunk(b"outl", &o));
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, v);
        ctx.read_outline(tree.root.find(b"outl").unwrap());
        let s = ctx.outlines[&4].stroke.clone().expect("stroke");
        assert_eq!(s.color, Color::rgb8(0, 0, 255));
        assert!((s.width - 0.254).abs() < 1e-9);
        assert_eq!(s.cap, LineCap::Round);
        assert_eq!(s.join, LineJoin::Bevel);
        assert_eq!(s.dash, vec![3.0, 1.0, 1.0, 1.0]);
        assert!((s.stretch - 0.5).abs() < 1e-9);
        assert!((s.nib_angle - 30.0).abs() < 1e-9);
        assert!(s.scale_with_object);
    }

    #[test]
    fn outline_version_9_layout() {
        let mut o = u32s(&[2]);
        o.extend_from_slice(&u16s(&[0, 0, 0])); // line type, caps, join
        o.extend_from_slice(&[0; 2]); // unknown1 (6 <= v < 13)
        o.extend_from_slice(&i32s(&[5080])); // 0.508 mm
        o.extend_from_slice(&u16s(&[100]));
        o.extend_from_slice(&[0; 2]);
        o.extend_from_slice(&i32s(&[0]));
        o.extend_from_slice(&[0; 52]);
        o.extend_from_slice(&color_bgr(9, 8, 7));
        o.extend_from_slice(&[0; 16]);
        o.extend_from_slice(&u16s(&[0]));
        o.extend_from_slice(&[0; 22]);
        o.extend_from_slice(&u32s(&[0, 0]));
        let main = riff(b"CDR9", &chunk(b"outl", &o));
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 9);
        ctx.read_outline(tree.root.find(b"outl").unwrap());
        let s = ctx.outlines[&2].stroke.clone().expect("stroke");
        assert_eq!(s.color, Color::rgb8(9, 8, 7));
        assert!((s.width - 0.508).abs() < 1e-9);
        assert!(s.dash.is_empty());
    }

    #[test]
    fn transform_layouts_for_versions_9_and_13() {
        for v in [9u16, 13] {
            let form = if v == 9 { *b"CDR9" } else { *b"CDRD" };
            let t = trfd(v, [2.0, 0.0, 254000.0, 0.0, 3.0, 508000.0]);
            let main = riff(&form, &chunk(b"trfd", &t));
            let tree = crate::riff::parse(&main).unwrap();
            let mut ctx = Ctx::new(&main, &tree, v);
            let a = ctx.read_transform(tree.root.find(b"trfd").unwrap());
            let c = a.as_coeffs();
            assert_eq!(c[0], 2.0, "v{v}");
            assert_eq!(c[3], 3.0, "v{v}");
            assert!((c[4] - 25.4).abs() < 1e-9, "v{v}");
            assert!((c[5] - 50.8).abs() < 1e-9, "v{v}");
            assert!(ctx.report.warnings.is_empty(), "{:?}", ctx.report.warnings);
        }
    }

    #[test]
    fn transform_record_of_another_type_is_ignored() {
        let mut t = u32s(&[0, 1, 12, 16]);
        t.extend_from_slice(&[0; 8]);
        t.extend_from_slice(&u16s(&[0x10]));
        t.extend_from_slice(&f64s(&[9.0; 6]));
        let main = riff(b"CDRD", &chunk(b"trfd", &t));
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 13);
        let a = ctx.read_transform(tree.root.find(b"trfd").unwrap());
        assert_eq!(a, Affine::IDENTITY);
    }

    #[test]
    fn curve_with_u32_count_and_reversed_arg_types() {
        // Points: move, line, line, closing line.
        let mut coords = u32s(&[4]);
        coords.extend_from_slice(&i32s(&[0, 0, 254000, 0, 254000, 254000, 0, 0]));
        coords.extend_from_slice(&[0x08, 0x40, 0x40, 0x48]);
        let args = [
            (ARG_COORDS, coords),
            (ARG_FILL, u32s(&[1])),
            (ARG_OUTLINE, u32s(&[2])),
        ];
        let obj = list(
            b"obj ",
            &[
                chunk(b"loda", &loda(OBJ_CURVE, &args)),
                chunk(b"trfd", &trfd(13, [1.0, 0.0, 0.0, 0.0, 1.0, 0.0])),
            ]
            .concat(),
        );
        let body = [
            chunk(b"mcfg", &mcfg(13, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRD", &body), 13);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Path { path, closed } => {
                assert!(*closed);
                let b = path.bounding_box();
                assert!((b.width() - 25.4).abs() < 1e-9 && (b.height() - 25.4).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn version_5_uses_16_bit_fields_and_thousandths_of_an_inch() {
        // One rectangle 1 x 0.5 inch, no radius; 16-bit header and tables.
        let args = [(ARG_COORDS as u16, i16s(&[1000, 500, 0]))];
        let obj = list(b"obj ", &chunk(b"loda", &loda16(OBJ_RECT as u16, &args)));
        let mut cfg = Vec::new();
        cfg.extend_from_slice(&i16s(&[8500, 11000]));
        let body = [chunk(b"mcfg", &cfg), list(b"page", &list(b"layr", &obj))].concat();
        let (doc, rep) = parse(&riff(b"CDR5", &body), 5);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        assert!((doc.pages[0].size.width - 215.9).abs() < 1e-9);
        match &first_shape(&doc).kind {
            ShapeKind::Rect { rect, .. } => {
                assert!((rect.width() - 25.4).abs() < 1e-9);
                assert!((rect.height() - 12.7).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn rectangle_x5_doubles_and_scale() {
        let mut c = f64s(&[254000.0, 127000.0]); // 1 x 0.5 inch
        c.extend_from_slice(&f64s(&[2.0, 2.0])); // scale
        c.push(0); // scale_with = 0: radii in coordinate units
        c.extend_from_slice(&[0; 7]);
        for (r, gap) in [
            (25400.0f64, 16usize),
            (25400.0, 16),
            (25400.0, 16),
            (25400.0, 0),
        ] {
            c.extend_from_slice(&f64s(&[r]));
            c.extend_from_slice(&vec![0u8; gap]);
        }
        let obj = list(
            b"obj ",
            &chunk(b"loda", &loda(OBJ_RECT, &[(ARG_COORDS, c)])),
        );
        let body = [
            chunk(b"mcfg", &mcfg(15, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRF", &body), 15);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Rect { rect, radius } => {
                assert!((rect.width() - 50.8).abs() < 1e-9);
                assert!((rect.height() - 25.4).abs() < 1e-9);
                assert!((radius - 2.54).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ellipse_with_pie_angles() {
        let mut c = i32s(&[508000, 254000]);
        c.extend_from_slice(&i32s(&[0, 90_000_000])); // 0 to 90 degrees
        c.extend_from_slice(&u32s(&[1])); // pie
        let obj = list(
            b"obj ",
            &chunk(b"loda", &loda(OBJ_ELLIPSE, &[(ARG_COORDS, c)])),
        );
        let body = [
            chunk(b"mcfg", &mcfg(9, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDR9", &body), 9);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Ellipse { rect, arc } => {
                assert!((rect.width() - 50.8).abs() < 1e-9);
                let arc = arc.expect("arc");
                assert!((arc.end_deg - 90.0).abs() < 1e-9);
                assert!(arc.pie);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn polygon_uses_the_polygon_argument() {
        let mut coords = u32s(&[2]);
        coords.extend_from_slice(&i32s(&[0, 0, 10000, 10000]));
        coords.extend_from_slice(&[0x00, 0x40]);
        // X3 layout: num angles, next point 1 (> 1 so 4 unknown bytes
        // follow), unknown3, rx, ry, cx, cy.
        let mut poly = u32s(&[6, 2, 0, 0]);
        poly.extend_from_slice(&f64s(&[254000.0, 127000.0]));
        poly.extend_from_slice(&i32s(&[0, 0]));
        let obj = list(
            b"obj ",
            &chunk(
                b"loda",
                &loda(OBJ_POLYGON, &[(ARG_COORDS, coords), (ARG_POLYGON, poly)]),
            ),
        );
        let body = [
            chunk(b"mcfg", &mcfg(13, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRD", &body), 13);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Polygon { rect, points, .. } => {
                assert_eq!(*points, 6);
                assert!((rect.width() - 50.8).abs() < 1e-9);
                assert!((rect.height() - 25.4).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn path_object_type_0x25() {
        let mut c = vec![0u8; 4];
        c.extend_from_slice(&u16s(&[2, 1]));
        c.extend_from_slice(&[0; 16]);
        c.extend_from_slice(&i32s(&[0, 0, 254000, 0, 254000, 254000]));
        c.extend_from_slice(&[0x00, 0x40, 0x40]);
        let obj = list(
            b"obj ",
            &chunk(b"loda", &loda(OBJ_PATH, &[(ARG_COORDS, c)])),
        );
        let body = [
            chunk(b"mcfg", &mcfg(16, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRG", &body), 16);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Path { path, closed } => {
                assert!(!*closed);
                assert_eq!(path.elements().len(), 3);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn mcfg_version_6_offset_and_vrsn_override() {
        let body = [
            chunk(b"vrsn", &u16s(&[600])),
            chunk(b"mcfg", &mcfg(6, 2_159_000, 2_794_000)),
        ]
        .concat();
        // Form type says 7, vrsn says 6: the 28-byte offset must be used.
        let (doc, rep) = parse(&riff(b"CDR7", &body), 7);
        assert!(
            (doc.pages[0].size.width - 215.9).abs() < 1e-9,
            "{:?}",
            rep.warnings
        );
        assert!(rep.warnings.iter().any(|w| w.contains("vrsn")));
    }

    #[test]
    fn layer_names_master_page_and_hidden_guides() {
        let name_arg = {
            let mut s: Vec<u8> = "Ink".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
            s.extend_from_slice(&[0, 0]);
            s
        };
        let layer_loda = loda(0, &[(ARG_NAME, name_arg)]);
        let mut flgs_layer = [0u8; 4];
        flgs_layer[3] = 0x98;
        let mut flgs_guides = [0x0au8, 0, 0, 0x98];
        flgs_guides[3] = 0x98;
        let mut flgs_master = [0u8; 4];
        flgs_master[3] = 0x90;
        flgs_master[2] = 1;
        let rect_obj = list(
            b"obj ",
            &chunk(
                b"loda",
                &loda(OBJ_RECT, &[(ARG_COORDS, i32s(&[254000, 254000, 0]))]),
            ),
        );
        let master = list(
            b"page",
            &[
                chunk(b"flgs", &flgs_master),
                list(b"layr", &[chunk(b"flgs", &flgs_guides)].concat()),
            ]
            .concat(),
        );
        let page = list(
            b"page",
            &list(
                b"layr",
                &[
                    chunk(b"flgs", &flgs_layer),
                    chunk(b"loda", &layer_loda),
                    rect_obj,
                ]
                .concat(),
            ),
        );
        let body = [
            chunk(b"mcfg", &mcfg(13, 2_540_000, 2_540_000)),
            master,
            page,
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRD", &body), 13);
        assert_eq!(doc.pages.len(), 1, "{:?}", rep.warnings);
        assert_eq!(doc.pages[0].layers[0].name, "Ink");
        assert_eq!(doc.pages[0].layers[0].shapes.len(), 1);
    }

    #[test]
    fn bitmap_chunk_becomes_a_png_shape() {
        // 2 x 2, 24 bpp: rows are 8 bytes (6 + 2 padding), bottom-up.
        let mut bmp = u32s(&[42]);
        bmp.extend_from_slice(&[0; 50]);
        bmp.extend_from_slice(&u32s(&[5, 0, 2, 2, 0, 24, 0, 16]));
        bmp.extend_from_slice(&[0; 32]);
        // bottom row: red, green; top row: blue, white
        bmp.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        bmp.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0]);
        let mut c = i32s(&[0, 0, 254000, 254000]);
        c.extend_from_slice(&[0; 32]);
        c.extend_from_slice(&u32s(&[42]));
        c.extend_from_slice(&[0; 20]);
        c.extend_from_slice(&u32s(&[0]));
        let obj = list(
            b"obj ",
            &chunk(b"loda", &loda(OBJ_BITMAP, &[(ARG_COORDS, c)])),
        );
        let body = [
            chunk(b"mcfg", &mcfg(13, 2_540_000, 2_540_000)),
            chunk(b"bmp ", &bmp),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRD", &body), 13);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Bitmap {
                width_px,
                height_px,
                png,
                rect,
            } => {
                assert_eq!((*width_px, *height_px), (2, 2));
                assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
                assert_eq!(&png[12..16], b"IHDR");
                assert!((rect.width() - 25.4).abs() < 1e-9);
                // Decode the IDAT and check the top-left pixel is blue.
                let idat_len = u32::from_be_bytes([png[33], png[34], png[35], png[36]]) as usize;
                let idat = &png[41..41 + idat_len];
                let mut raw = Vec::new();
                std::io::Read::read_to_end(&mut flate2::read::ZlibDecoder::new(idat), &mut raw)
                    .unwrap();
                assert_eq!(&raw[1..5], &[0, 0, 255, 255]);
                assert_eq!(&raw[10..14], &[255, 0, 0, 255]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn pattern_fill_x3_reads_both_colours() {
        let mut p = vec![0u8; 8];
        p.extend_from_slice(&u32s(&[77])); // pattern id
        p.extend_from_slice(&i32s(&[254000, 254000]));
        p.extend_from_slice(&[0; 4]);
        p.extend_from_slice(&u16s(&[0]));
        p.push(0); // flags
        p.extend_from_slice(&[0; 6]);
        p.extend_from_slice(&color_bgr(1, 2, 3));
        p.extend_from_slice(&[0; 10]);
        p.extend_from_slice(&color_bgr(4, 5, 6));
        let mut fild = u32s(&[5, 1300, (p.len() + 2) as u32]);
        fild.extend_from_slice(&u16s(&[7]));
        fild.extend_from_slice(&p);
        let main = riff(b"CDRD", &chunk(b"fild", &fild));
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 13);
        ctx.read_fill(tree.root.find(b"fild").unwrap());
        match &ctx.fills[&5].fill {
            Fill::Pattern(Pattern::TwoColor {
                front,
                back,
                size_mm,
                ..
            }) => {
                assert_eq!(*front, Color::rgb8(1, 2, 3));
                assert_eq!(*back, Color::rgb8(4, 5, 6));
                assert!((size_mm - 25.4).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn malformed_payloads_do_not_panic() {
        let junk: Vec<Vec<u8>> = vec![
            vec![],
            vec![1],
            u32s(&[0xffff_ffff; 5]),
            u32s(&[20, 3, 0xffff_fff0, 0xffff_fff0, 3]),
            u32s(&[1, 0xffff_ffff, 0xffff_ffff, 0x4000_0000, 0x4000_0000, 1]),
            vec![0x47; 97],
        ];
        for j in &junk {
            let text_obj = list(
                b"obj ",
                &[
                    chunk(
                        b"loda",
                        &loda(
                            OBJ_ARTISTIC_TEXT,
                            &[(ARG_COORDS, j.clone()), (ARG_STYLE, j.clone())],
                        ),
                    ),
                    chunk(b"txsm", j),
                ]
                .concat(),
            );
            let body = [
                chunk(b"fild", j),
                chunk(b"outl", j),
                chunk(b"bmp ", j),
                chunk(b"bmpf", j),
                chunk(b"arrw", j),
                chunk(b"font", j),
                list(b"stlt", j),
                chunk(b"mcfg", j),
                list(
                    b"page",
                    &list(
                        b"layr",
                        &[
                            list(b"obj ", &[chunk(b"loda", j), chunk(b"trfd", j)].concat()),
                            text_obj,
                        ]
                        .concat(),
                    ),
                ),
            ]
            .concat();
            for v in [3u16, 5, 6, 7, 9, 12, 13, 15, 16, 17] {
                let (doc, _) = parse(&riff(b"CDR9", &body), v);
                assert_eq!(doc.pages.len(), 1);
            }
        }
    }

    // Text: font table, txsm layouts, style table.

    fn utf16(s: &str) -> Vec<u8> {
        s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
    }

    /// `font` chunk payload (version 12+: UTF-16LE name).
    fn font_chunk(id: u16, name: &str) -> Vec<u8> {
        let mut f = u16s(&[id, 0]);
        f.extend_from_slice(&u32s(&[0]));
        f.extend_from_slice(&[0; 10]);
        f.extend_from_slice(&utf16(name));
        f.extend_from_slice(&[0, 0]);
        f
    }

    /// Pre-X3 solid fill (`fild`: id, type 1, 2 unknown, colour).
    fn solid_fild(id: u32, rgb: (u8, u8, u8)) -> Vec<u8> {
        let mut fild = u32s(&[id]);
        fild.extend_from_slice(&u16s(&[1, 0]));
        fild.extend_from_slice(&color_bgr(rgb.0, rgb.1, rgb.2));
        fild
    }

    /// X3+ solid fill with a property list holding one colour.
    fn solid_fild_x3(id: u32, rgb: (u8, u8, u8)) -> Vec<u8> {
        let mut props = vec![0x01u8];
        props.extend_from_slice(&12u32.to_le_bytes());
        props.extend_from_slice(&color_bgr(rgb.0, rgb.1, rgb.2));
        props.push(0x00);
        let mut solid = u32s(&[1300, props.len() as u32]);
        solid.extend_from_slice(&props);
        let mut fild = u32s(&[id, 1300, (solid.len() + 2) as u32]);
        fild.extend_from_slice(&u16s(&[1]));
        fild.extend_from_slice(&solid);
        fild
    }

    /// One style record of the 7 to X5 `txsm` layout.
    struct Style7 {
        chars: u16,
        font: Option<u16>,
        flags: Option<u32>,
        size_units: Option<i32>,
        fill: Option<u32>,
    }

    fn txsm7(v: u16, frame: bool, styles: &[Style7], text: &str) -> Vec<u8> {
        let mut t = u32s(&[frame as u32]);
        t.extend_from_slice(&[0; 32]);
        if v >= 15 {
            t.push(0);
        }
        t.extend_from_slice(&u32s(&[1])); // frames
        t.extend_from_slice(&u32s(&[1])); // frame id
        t.extend_from_slice(&[0; 48]);
        t.extend_from_slice(&u32s(&[0])); // not on a path
        if v >= 15 {
            t.extend_from_slice(&[0; 8]);
        }
        if !frame {
            t.extend_from_slice(&vec![
                0u8;
                if v >= 15 {
                    40
                } else if v >= 14 {
                    36
                } else {
                    34
                }
            ]);
        } else if v >= 15 {
            t.extend_from_slice(&[0; 4]);
        }
        t.extend_from_slice(&u32s(&[1])); // paragraphs
        t.extend_from_slice(&u32s(&[0])); // style id
        t.push(0);
        if v >= 13 && frame {
            t.push(0);
        }
        t.extend_from_slice(&u32s(&[styles.len() as u32]));
        for s in styles {
            t.extend_from_slice(&u16s(&[s.chars]));
            let mut flags = 0u8;
            if s.font.is_some() {
                flags |= 0x01;
            }
            if s.flags.is_some() {
                flags |= 0x02;
            }
            if s.size_units.is_some() {
                flags |= 0x04;
            }
            if s.fill.is_some() {
                flags |= 0x40;
            }
            t.push(flags);
            t.push(0); // fl3
            if let Some(f) = s.font {
                t.extend_from_slice(&u16s(&[f, 0]));
            }
            if let Some(f) = s.flags {
                t.extend_from_slice(&u32s(&[f]));
            }
            if let Some(sz) = s.size_units {
                t.extend_from_slice(&i32s(&[sz]));
            }
            if let Some(f) = s.fill {
                t.extend_from_slice(&u32s(&[f]));
                if v >= 13 {
                    t.extend_from_slice(&[0; 48]);
                }
            }
        }
        let n = text.chars().count() as u32;
        t.extend_from_slice(&u32s(&[n]));
        t.extend_from_slice(&vec![0u8; n as usize * if v >= 12 { 8 } else { 4 }]);
        if v >= 12 {
            let bytes = utf16(text);
            t.extend_from_slice(&u32s(&[bytes.len() as u32]));
            t.extend_from_slice(&bytes);
        } else {
            t.extend_from_slice(text.as_bytes());
        }
        t.push(0); // no path
        t
    }

    fn text_doc(
        v: u16,
        form: &[u8; 4],
        tables: Vec<u8>,
        kind: u32,
        args: &[(u32, Vec<u8>)],
        txsm: Vec<u8>,
    ) -> (Document, ParseReport) {
        let obj = list(
            b"obj ",
            &[chunk(b"loda", &loda(kind, args)), chunk(b"txsm", &txsm)].concat(),
        );
        let body = [
            chunk(b"mcfg", &mcfg(v, 2_540_000, 2_540_000)),
            tables,
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        parse(&riff(form, &body), v)
    }

    #[test]
    fn artistic_text_version_12_runs_family_size_bold_and_fill() {
        let styles = [
            Style7 {
                chars: 5,
                font: Some(3),
                flags: Some(0x1000),
                size_units: Some(127_000),
                fill: Some(9),
            },
            Style7 {
                chars: 6,
                font: None,
                flags: None,
                size_units: Some(63_500),
                fill: None,
            },
        ];
        let tables = [
            chunk(b"font", &font_chunk(3, "Verdana")),
            chunk(b"fild", &solid_fild(7, (255, 0, 0))),
            chunk(b"fild", &solid_fild(9, (0, 0, 255))),
        ]
        .concat();
        let (doc, rep) = text_doc(
            12,
            b"CDRC",
            tables,
            OBJ_ARTISTIC_TEXT,
            &[
                (ARG_COORDS, i32s(&[254_000, 127_000])),
                (ARG_FILL, u32s(&[7])),
            ],
            txsm7(12, false, &styles, "Hello World"),
        );
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        let s = first_shape(&doc);
        assert_eq!(s.fill, Fill::Solid(Color::rgb8(255, 0, 0)));
        match &s.kind {
            ShapeKind::Text {
                spans,
                origin,
                frame,
                align,
                ..
            } => {
                assert!((origin.x - 25.4).abs() < 1e-9 && (origin.y - 12.7).abs() < 1e-9);
                assert!(frame.is_none());
                assert_eq!(*align, TextAlign::Left);
                assert_eq!(spans.len(), 2, "{spans:?}");
                assert_eq!(spans[0].text, "Hello");
                assert_eq!(spans[0].font_family, "Verdana");
                assert!((spans[0].size_pt - 36.0).abs() < 1e-9);
                assert!(spans[0].bold && !spans[0].italic);
                assert_eq!(spans[0].fill, Some(Fill::Solid(Color::rgb8(0, 0, 255))));
                assert_eq!(spans[1].text, " World");
                assert_eq!(spans[1].font_family, DEFAULT_FAMILY);
                assert!((spans[1].size_pt - 18.0).abs() < 1e-9);
                assert!(!spans[1].bold);
                assert_eq!(spans[1].fill, None);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn text_version_9_uses_single_byte_code_page() {
        let styles = [Style7 {
            chars: 4,
            font: None,
            flags: None,
            size_units: None,
            fill: None,
        }];
        let mut t = txsm7(9, false, &styles, "");
        // Replace the empty text: count 4, descriptions, bytes "caf" + 0xe9.
        let cut = t.len() - 1 - 4; // has_path byte and the u32 count of 0
        t.truncate(cut);
        t.extend_from_slice(&u32s(&[4]));
        t.extend_from_slice(&[0; 16]);
        t.extend_from_slice(&[b'c', b'a', b'f', 0xe9]);
        t.push(0);
        let (doc, rep) = text_doc(
            9,
            b"CDR9",
            Vec::new(),
            OBJ_ARTISTIC_TEXT,
            &[(ARG_COORDS, i32s(&[0, 0]))],
            t,
        );
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Text { spans, .. } => {
                assert_eq!(spans[0].text, "caf\u{e9}");
                assert!((spans[0].size_pt - DEFAULT_SIZE_PT).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
    }

    /// X6+ style string: u32 length in UTF-16 units, then the text.
    fn style_string(v: u16, s: &str) -> Vec<u8> {
        if v < 17 {
            let mut out = u32s(&[s.encode_utf16().count() as u32]);
            out.extend_from_slice(&utf16(s));
            out
        } else {
            let mut out = u32s(&[s.len() as u32]);
            out.extend_from_slice(s.as_bytes());
            out
        }
    }

    #[test]
    fn paragraph_text_x6_layout_with_style_strings() {
        let v = 16u16;
        let mut t = u32s(&[1]); // frame
        t.extend_from_slice(&[0; 32]);
        t.extend_from_slice(&u16s(&[1600]));
        t.extend_from_slice(&[0; 3]);
        t.extend_from_slice(&u32s(&[1, 1])); // one frame, id 1
        t.extend_from_slice(&[0; 48]);
        t.extend_from_slice(&u32s(&[0])); // not on a path
        t.extend_from_slice(&[0; 8]);
        t.extend_from_slice(&u32s(&[1])); // one paragraph
        t.extend_from_slice(&u32s(&[0]));
        t.push(0);
        t.push(0); // frame flag byte: not 1
        t.extend_from_slice(&style_string(
            v,
            r#"{"character":{"font":5,"size":10},"paragraph":{"justify":3}}"#,
        ));
        t.extend_from_slice(&u32s(&[1])); // one style record
        t.extend_from_slice(&u16s(&[0, 1, 0]));
        t.extend_from_slice(&style_string(v, r#"{"character":{"fill":9}}"#));
        t.extend_from_slice(&u32s(&[4]));
        for idx in [0u8, 0, 0xfe, 0xfe] {
            t.extend_from_slice(&u16s(&[0]));
            t.push(idx);
            t.extend_from_slice(&[0; 5]);
        }
        let bytes = utf16("Abcd");
        t.extend_from_slice(&u32s(&[bytes.len() as u32]));
        t.extend_from_slice(&bytes);
        t.push(0);
        let tables = [
            chunk(b"font", &font_chunk(5, "Georgia")),
            chunk(b"fild", &solid_fild_x3(9, (0, 128, 0))),
        ]
        .concat();
        let mut coords = vec![0u8; 4];
        coords.extend_from_slice(&i32s(&[508_000, 254_000]));
        let (doc, rep) = text_doc(
            v,
            b"CDRG",
            tables,
            OBJ_PARAGRAPH_TEXT,
            &[(ARG_COORDS, coords)],
            t,
        );
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Text {
                spans,
                frame,
                align,
                ..
            } => {
                let f = frame.expect("frame");
                assert!((f.width - 50.8).abs() < 1e-9 && (f.height - 25.4).abs() < 1e-9);
                assert_eq!(*align, TextAlign::Center);
                assert_eq!(spans.len(), 2, "{spans:?}");
                assert_eq!(spans[0].text, "Ab");
                assert_eq!(spans[0].font_family, "Georgia");
                assert!((spans[0].size_pt - 10.0).abs() < 1e-9);
                assert_eq!(spans[0].fill, Some(Fill::Solid(Color::rgb8(0, 128, 0))));
                assert_eq!(spans[1].text, "cd");
                assert_eq!(spans[1].fill, None);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn style_table_supplies_font_size_face_and_alignment() {
        let v = 13u16;
        let mut st = u32s(&[1]); // one record
        st.extend_from_slice(&u32s(&[0])); // fills
        st.extend_from_slice(&u32s(&[0])); // outlines
        st.extend_from_slice(&u32s(&[1, 100])); // one font entry, id 100
        st.extend_from_slice(&[0; 20]);
        st.extend_from_slice(&u16s(&[3, 0])); // font id 3, encoding
        st.extend_from_slice(&[0; 8]);
        st.extend_from_slice(&i32s(&[127_000])); // 36 pt
        st.extend_from_slice(&[0; 8]);
        st.extend_from_slice(&u32s(&[0x80])); // italic
        st.extend_from_slice(&[0; 8]);
        st.extend_from_slice(&u32s(&[1, 200, 0, 2])); // one align entry: right
        for _ in 0..3 {
            st.extend_from_slice(&u32s(&[0])); // intervals, set5, tabs
        }
        st.extend_from_slice(&u32s(&[0])); // bullets
        for _ in 0..4 {
            st.extend_from_slice(&u32s(&[0])); // indents, hyphens, drop caps, set11
        }
        // Record: num 2, style 77, parent 0, 8 unknown, name "Std".
        st.extend_from_slice(&u32s(&[2, 77, 0, 0, 0, 3]));
        st.extend_from_slice(&utf16("Std"));
        st.extend_from_slice(&u32s(&[0, 0])); // fill, outline refs
        st.extend_from_slice(&u32s(&[100, 200, 0, 0, 0])); // font, align, interval, set5, set11
        let styles = [Style7 {
            chars: 2,
            font: None,
            flags: None,
            size_units: None,
            fill: None,
        }];
        let tables = [
            chunk(b"font", &font_chunk(3, "Verdana")),
            list(b"stlt", &st),
        ]
        .concat();
        let (doc, rep) = text_doc(
            v,
            b"CDRD",
            tables,
            OBJ_ARTISTIC_TEXT,
            &[(ARG_COORDS, i32s(&[0, 0])), (ARG_STYLE, u32s(&[77]))],
            txsm7(v, false, &styles, "Hi"),
        );
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Text { spans, align, .. } => {
                assert_eq!(*align, TextAlign::Right);
                assert_eq!(spans[0].font_family, "Verdana");
                assert!((spans[0].size_pt - 36.0).abs() < 1e-9);
                assert!(spans[0].italic && !spans[0].bold);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn text_object_without_txsm_is_skipped_with_a_warning() {
        let obj = list(
            b"obj ",
            &chunk(
                b"loda",
                &loda(OBJ_ARTISTIC_TEXT, &[(ARG_COORDS, i32s(&[0, 0]))]),
            ),
        );
        let body = [
            chunk(b"mcfg", &mcfg(13, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (_, rep) = parse(&riff(b"CDRD", &body), 13);
        assert_eq!(rep.shapes, 0);
        assert_eq!(rep.skipped_objects, 1);
        assert!(rep.warnings.iter().any(|w| w.contains("txsm")));
    }

    // Fills: bitmaps, textures, pattern tiles, X6 fountain transform.

    /// 2 x 2 24-bit `bmp ` payload with the given image id.
    fn bmp_2x2(image_id: u32) -> Vec<u8> {
        let mut bmp = u32s(&[image_id]);
        bmp.extend_from_slice(&[0; 50]);
        bmp.extend_from_slice(&u32s(&[5, 0, 2, 2, 0, 24, 0, 16]));
        bmp.extend_from_slice(&[0; 32]);
        bmp.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        bmp.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0]);
        bmp
    }

    /// X3+ image fill body (types 9 and 11): one 0x514 record, then the
    /// pattern data.
    fn image_fill_x3(ftype: u16, id: u32, pattern: u32, w: u32) -> Vec<u8> {
        let mut p = u32s(&[0x514]);
        p.extend_from_slice(&u32s(&[pattern, w, w]));
        p.extend_from_slice(&[0; 4]);
        p.extend_from_slice(&u16s(&[0]));
        p.push(0);
        p.extend_from_slice(&[0; 17]);
        p.extend_from_slice(&u32s(&[pattern]));
        let mut fild = u32s(&[id, 1300, (p.len() + 2) as u32]);
        fild.extend_from_slice(&u16s(&[ftype]));
        fild.extend_from_slice(&p);
        fild
    }

    #[test]
    fn colour_bitmap_fill_uses_the_bmp_table() {
        let main = riff(
            b"CDRD",
            &[
                chunk(b"bmp ", &bmp_2x2(42)),
                chunk(b"fild", &image_fill_x3(9, 4, 42, 254_000)),
            ]
            .concat(),
        );
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 13);
        ctx.read_bitmap(tree.root.find(b"bmp ").unwrap());
        ctx.read_fill(tree.root.find(b"fild").unwrap());
        match &ctx.fills[&4].fill {
            Fill::Pattern(Pattern::Bitmap {
                width_px,
                height_px,
                size_mm,
                png,
            }) => {
                assert_eq!((*width_px, *height_px), (2, 2));
                assert!((size_mm - 25.4).abs() < 1e-9);
                assert_eq!(&png[1..4], b"PNG");
            }
            other => panic!("{other:?}"),
        }
        assert!(ctx.report.warnings.is_empty(), "{:?}", ctx.report.warnings);
    }

    #[test]
    fn texture_fill_falls_back_to_a_built_in_texture() {
        let main = riff(b"CDRD", &chunk(b"fild", &image_fill_x3(11, 6, 99, 508_000)));
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 13);
        ctx.read_fill(tree.root.find(b"fild").unwrap());
        match &ctx.fills[&6].fill {
            Fill::Texture(t) => {
                assert_eq!(t.kind, TextureKind::Clouds);
                assert!((t.scale - 50.8).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
        assert!(ctx.report.warnings.iter().any(|w| w.contains("texture")));
    }

    #[test]
    fn pattern_tile_bitmap_colours_the_two_colour_pattern() {
        // 8 x 2 one-bit tile: top row 11110000, bottom row 10101010.
        let mut bmpf = u32s(&[77]);
        bmpf.extend_from_slice(&u32s(&[40]));
        bmpf.extend_from_slice(&i32s(&[8, 2]));
        bmpf.extend_from_slice(&u16s(&[1, 1]));
        bmpf.extend_from_slice(&u32s(&[0, 8]));
        bmpf.extend_from_slice(&[0; 16]);
        bmpf.extend_from_slice(&[0, 0, 0, 0, 255, 255, 255, 0]); // palette: black, white
        bmpf.extend_from_slice(&[0b1010_1010, 0, 0, 0]); // bottom row
        bmpf.extend_from_slice(&[0b1111_0000, 0, 0, 0]); // top row
        let mut p = vec![0u8; 8];
        p.extend_from_slice(&u32s(&[77]));
        p.extend_from_slice(&i32s(&[254000, 254000]));
        p.extend_from_slice(&[0; 4]);
        p.extend_from_slice(&u16s(&[0]));
        p.push(0);
        p.extend_from_slice(&[0; 6]);
        p.extend_from_slice(&color_bgr(1, 2, 3));
        p.extend_from_slice(&[0; 10]);
        p.extend_from_slice(&color_bgr(4, 5, 6));
        let mut fild = u32s(&[5, 1300, (p.len() + 2) as u32]);
        fild.extend_from_slice(&u16s(&[7]));
        fild.extend_from_slice(&p);
        let main = riff(
            b"CDRD",
            &[chunk(b"bmpf", &bmpf), chunk(b"fild", &fild)].concat(),
        );
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 13);
        ctx.read_pattern_tile(tree.root.find(b"bmpf").unwrap());
        ctx.read_fill(tree.root.find(b"fild").unwrap());
        match &ctx.fills[&5].fill {
            Fill::Pattern(Pattern::Bitmap {
                width_px,
                height_px,
                png,
                ..
            }) => {
                assert_eq!((*width_px, *height_px), (8, 2));
                let idat_len = u32::from_be_bytes([png[33], png[34], png[35], png[36]]) as usize;
                let mut raw = Vec::new();
                std::io::Read::read_to_end(
                    &mut flate2::read::ZlibDecoder::new(&png[41..41 + idat_len]),
                    &mut raw,
                )
                .unwrap();
                // Top row: bit 1 (white) is the back colour, bit 0 the front.
                assert_eq!(&raw[1..4], &[4, 5, 6]);
                assert_eq!(&raw[1 + 4 * 4..1 + 4 * 4 + 3], &[1, 2, 3]);
                // Bottom row starts with a set bit: back colour.
                assert_eq!(&raw[33 + 1..33 + 4], &[4, 5, 6]);
                assert_eq!(&raw[33 + 1 + 4..33 + 4 + 4], &[1, 2, 3]);
            }
            other => panic!("{other:?}"),
        }
        assert!(ctx.report.warnings.is_empty(), "{:?}", ctx.report.warnings);
    }

    #[test]
    fn fountain_x6_transformation_sets_offset_and_edge_pad() {
        let v = 16u16;
        let mut g = vec![0u8; 8];
        g.push(1);
        g.extend_from_slice(&[0; 17]);
        g.extend_from_slice(&i16s(&[0]));
        g.extend_from_slice(&i32s(&[0]));
        g.extend_from_slice(&i32s(&[0, 0]));
        g.extend_from_slice(&[0; 2]);
        g.extend_from_slice(&u32s(&[0]));
        g.extend_from_slice(&[50, 0]);
        g.extend_from_slice(&u32s(&[2]));
        g.extend_from_slice(&[0; 3]);
        for (c, pos) in [((0, 0, 0), 0u32), ((255, 255, 255), 100)] {
            g.extend_from_slice(&color_bgr(c.0, c.1, c.2));
            g.extend_from_slice(&[0; 26]);
            g.extend_from_slice(&u32s(&[pos]));
            g.extend_from_slice(&[0; 3]);
        }
        g.extend_from_slice(&[0; 3]);
        g.extend_from_slice(&f64s(&[0.25, -0.1, 0.5, 0.8]));
        let mut fild = u32s(&[1, 1300, (g.len() + 2) as u32]);
        fild.extend_from_slice(&u16s(&[2]));
        fild.extend_from_slice(&g);
        let main = riff(b"CDRG", &chunk(b"fild", &fild));
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, v);
        ctx.read_fill(tree.root.find(b"fild").unwrap());
        match &ctx.fills[&1].fill {
            Fill::Fountain(f) => {
                assert!((f.offset.x - 0.25).abs() < 1e-9 && (f.offset.y + 0.1).abs() < 1e-9);
                assert!((f.edge_pad - 0.25).abs() < 1e-9);
                assert_eq!(f.stops.len(), 2);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn opacity_argument_is_a_transparency() {
        let mut op = vec![0u8; 14];
        op.extend_from_slice(&u16s(&[400]));
        let obj = list(
            b"obj ",
            &chunk(
                b"loda",
                &loda(
                    OBJ_RECT,
                    &[(ARG_COORDS, i32s(&[254000, 254000, 0])), (ARG_OPACITY, op)],
                ),
            ),
        );
        let body = [
            chunk(b"mcfg", &mcfg(13, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRD", &body), 13);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        assert!((first_shape(&doc).opacity - 0.6).abs() < 1e-9);
    }

    // Arrowheads.

    fn arrw(id: u32, pts: &[(i32, i32)], types: &[u8]) -> Vec<u8> {
        let mut a = u32s(&[id, pts.len() as u32]);
        for (x, y) in pts {
            a.extend_from_slice(&i32s(&[*x, *y]));
        }
        a.extend_from_slice(types);
        a
    }

    /// X3 outline with the given marker ids.
    fn outl_x3(id: u32, start: u32, end: u32) -> Vec<u8> {
        let mut o = u32s(&[id]);
        o.extend_from_slice(&u32s(&[1, 0]));
        o.extend_from_slice(&u16s(&[0, 0, 0]));
        o.extend_from_slice(&i32s(&[2540]));
        o.extend_from_slice(&u16s(&[100]));
        o.extend_from_slice(&[0; 2]);
        o.extend_from_slice(&i32s(&[0]));
        o.extend_from_slice(&[0; 46]);
        o.extend_from_slice(&color_bgr(0, 0, 0));
        o.extend_from_slice(&[0; 16]);
        o.extend_from_slice(&u16s(&[0]));
        o.extend_from_slice(&[0; 22]);
        o.extend_from_slice(&u32s(&[start, end]));
        o
    }

    #[test]
    fn arrowheads_are_classified_from_their_outline() {
        let triangle = arrw(
            5,
            &[(0, 0), (100, 50), (0, 100), (0, 0)],
            &[0x08, 0x40, 0x40, 0x48],
        );
        let diamond = arrw(
            6,
            &[(50, 0), (100, 50), (50, 100), (0, 50), (50, 0)],
            &[0x08, 0x40, 0x40, 0x40, 0x48],
        );
        let square = arrw(
            7,
            &[(0, 0), (100, 0), (100, 100), (0, 100), (0, 0)],
            &[0x08, 0x40, 0x40, 0x40, 0x48],
        );
        let bar = arrw(
            8,
            &[(0, 0), (10, 0), (10, 100), (0, 100), (0, 0)],
            &[0x08, 0x40, 0x40, 0x40, 0x48],
        );
        // Circle: four Bezier segments.
        let circle = arrw(
            9,
            &[
                (100, 50),
                (100, 78),
                (78, 100),
                (50, 100),
                (22, 100),
                (0, 78),
                (0, 50),
                (0, 22),
                (22, 0),
                (50, 0),
                (78, 0),
                (100, 22),
                (100, 50),
            ],
            &[
                0x08, 0xC0, 0xC0, 0x80, 0xC0, 0xC0, 0x80, 0xC0, 0xC0, 0x80, 0xC0, 0xC0, 0x88,
            ],
        );
        let open = arrw(10, &[(0, 0), (100, 50), (0, 100)], &[0x00, 0x40, 0x40]);
        let main = riff(
            b"CDRD",
            &[
                chunk(b"arrw", &triangle),
                chunk(b"arrw", &diamond),
                chunk(b"arrw", &square),
                chunk(b"arrw", &bar),
                chunk(b"arrw", &circle),
                chunk(b"arrw", &open),
                chunk(b"outl", &outl_x3(1, 5, 6)),
                chunk(b"outl", &outl_x3(2, 7, 8)),
                chunk(b"outl", &outl_x3(3, 9, 10)),
                chunk(b"outl", &outl_x3(4, 99, 0)),
            ]
            .concat(),
        );
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 13);
        for c in tree.root.find_all(b"arrw") {
            ctx.read_arrowhead(c);
        }
        for c in tree.root.find_all(b"outl") {
            ctx.read_outline(c);
        }
        let arrows = |id: u32| {
            let s = ctx.outlines[&id].stroke.clone().expect("stroke");
            (s.start_arrow, s.end_arrow)
        };
        assert_eq!(arrows(1), (Arrowhead::Arrow, Arrowhead::Diamond));
        assert_eq!(arrows(2), (Arrowhead::Square, Arrowhead::Bar));
        assert_eq!(arrows(3), (Arrowhead::Circle, Arrowhead::OpenArrow));
        assert_eq!(arrows(4), (Arrowhead::None, Arrowhead::None));
        assert!(ctx
            .report
            .warnings
            .iter()
            .any(|w| w.contains("arrowhead 99")));
    }
}
