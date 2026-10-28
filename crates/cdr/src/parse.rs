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
    document::{
        Corners, EllipseArc, Layer, Page, ParagraphStyle, Shape, ShapeKind, TextAlign, TextSpan,
    },
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
    pub(crate) fn warn(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        if self.warnings.len() < 200 && !self.warnings.contains(&msg) {
            self.warnings.push(msg);
        }
    }
}

/// Steps of a blend read from a file whose step count field holds no
/// plausible count: the default of the program that saves them, which
/// both sample files hold (#980).
const BLEND_DEFAULT_STEPS: u32 = 20;

/// The most steps all the blends of one file take together: 5000 blends
/// of the default 20 steps. Each step is drawn from its own outline, so
/// without a bound a file of many blends could take all the memory
/// (#980).
const FILE_BLEND_STEPS: u32 = 100_000;

/// Farther than this from (0, 0), the coordinates of artistic text are
/// not a place (see `build_text`).
const MAX_TEXT_ORIGIN_MM: f64 = 10_000.0;
/// Longest page side read, mm: large format work (banners, wraps) has
/// pages of 20 m and more, and a larger value means a misread record. The
/// CMX reader takes the same bound.
const MAX_PAGE_MM: f64 = 100_000.0;
/// CDR coordinate unit (version 6 and later) to millimetres.
const UNIT_MM: f64 = 25.4 / 254000.0;
/// Coordinate unit of versions before 6 (1/1000 inch) to millimetres.
const UNIT16_MM: f64 = 25.4 / 1000.0;
/// Byte offset of the point count in an `arrx` record.
const ARRX_COUNT_AT: usize = 268;

/// Argument types of the `loda` argument table.
pub(crate) const ARG_OUTLINE: u32 = 0x0a;
pub(crate) const ARG_FILL: u32 = 0x14;
pub(crate) const ARG_COORDS: u32 = 0x1e;
pub(crate) const ARG_STYLE: u32 = 200;
/// The object's matrix inside its `lobj` (version 3).
const ARG_LEGACY_TRANSFORM: u32 = 100;
/// The object's style as a string (version 16 and later): fill, outline,
/// character, frame and paragraph settings (seen in a version 21 file).
pub(crate) const ARG_STYLE_STRING: u32 = 201;
pub(crate) const ARG_NAME: u32 = 1000;
pub(crate) const ARG_OPACITY: u32 = 8000;
const ARG_POLYGON: u32 = 11000;
/// The contents of a clip container (version 12): the `spnd` number of a
/// group in the document's `LIST clpt`, then three u32 (1, 0, 1 seen).
const ARG_CLIP_CONTENTS: u32 = 8005;
const ARG_PAGE_SIZE: u32 = 19130;
/// Master page `loda` argument holding the document's page background.
const ARG_PAGE_BACKGROUND: u32 = 17000;
/// Object argument 16016: 1 hides the object (version 21).
const ARG_HIDDEN: u32 = 16016;

/// Family and size used when neither the run nor its style names one.
const DEFAULT_FAMILY: &str = "Arial";

/// The names a family takes for a face of a weight other than regular
/// and bold, the way installed fonts list such faces under their own
/// family name (name ID 1).
const WEIGHT_NAMES: [(u16, &str); 7] = [
    (100, "Thin"),
    (200, "ExtraLight"),
    (300, "Light"),
    (500, "Medium"),
    (600, "SemiBold"),
    (800, "ExtraBold"),
    (900, "Black"),
];

/// Weight names a family name can end in, run together and in lower
/// case, with their weights (the text engine reads the same names).
const NAMED_WEIGHTS: [(&str, u16); 12] = [
    ("thin", 100),
    ("hairline", 100),
    ("extralight", 200),
    ("ultralight", 200),
    ("light", 300),
    ("medium", 500),
    ("semibold", 600),
    ("demibold", 600),
    ("extrabold", 800),
    ("ultrabold", 800),
    ("black", 900),
    ("heavy", 900),
];

/// A family name ending in a weight name (the last word, or the last two
/// run together), split into the family and the weight: "Titillium Web
/// SemiBold" is ("Titillium Web", 600).
pub(crate) fn family_weight(family: &str) -> Option<(&str, u16)> {
    let weight_of = |words: &[&str]| {
        let key: String = words
            .iter()
            .flat_map(|w| w.chars())
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        NAMED_WEIGHTS
            .iter()
            .find(|(n, _)| *n == key)
            .map(|(_, w)| *w)
    };
    let (head, last) = family.trim_end().rsplit_once(' ')?;
    let head = head.trim_end();
    if let Some(w) = weight_of(&[last]).filter(|_| !head.is_empty()) {
        return Some((head, w));
    }
    let (head2, prev) = head.rsplit_once(' ')?;
    let head2 = head2.trim_end();
    weight_of(&[prev, last])
        .filter(|_| !head2.is_empty())
        .map(|w| (head2, w))
}

/// A span set in a face of a weight between or beyond regular and bold
/// (a semibold, a light): the family takes the weight's name and the
/// span is not bold, so the text draws with that face rather than the
/// bold or regular one ("Titillium Web" at 600 is "Titillium Web
/// SemiBold"). A family already named with a weight keeps its name, and
/// is not bold when the name gives the code's weight.
fn name_weight(span: &mut TextSpan, weight: Option<u16>) {
    let Some(w) = weight else {
        return;
    };
    if let Some((_, named)) = family_weight(&span.font_family) {
        // The family names the face already ("Segoe UI Semibold" at
        // 600): that face, not a bold one.
        if named == w {
            span.bold = false;
        }
        return;
    }
    if span.font_family.to_ascii_lowercase().ends_with("bold") {
        return;
    }
    let Some(&(_, name)) = WEIGHT_NAMES.iter().find(|(n, _)| *n == w) else {
        return;
    };
    span.font_family = format!("{} {name}", span.font_family.trim_end());
    span.bold = false;
}
const DEFAULT_SIZE_PT: f64 = 24.0;

/// Object types stored in the `loda` header.
pub(crate) const OBJ_RECT: u32 = 0x01;
pub(crate) const OBJ_ELLIPSE: u32 = 0x02;
pub(crate) const OBJ_CURVE: u32 = 0x03;
pub(crate) const OBJ_ARTISTIC_TEXT: u32 = 0x04;
pub(crate) const OBJ_BITMAP: u32 = 0x05;
const OBJ_PARAGRAPH_TEXT: u32 = 0x06;
const OBJ_POLYGON: u32 = 0x14;
const OBJ_PATH: u32 = 0x25;
/// A straight connector line whose ends are in its `ppdt` record.
const OBJ_CONNECTOR: u32 = 0x26;
/// An object embedded from another program (seen in version 12 files):
/// its name argument names the storage of an `ole2` chunk, its coordinates
/// hold its size and two opposite corners.
const OBJ_EMBEDDED: u32 = 0x0a;
/// A mesh fill object (seen in a version 11 file, #896): a curve (its
/// coordinates argument) filled with the colour grid of [`ARG_MESH`].
const OBJ_MESH: u32 = 0x20;
/// The mesh of an [`OBJ_MESH`] object: its grid lines and node colours
/// (see [`Ctx::read_mesh`]).
const ARG_MESH: u32 = 19150;
/// The largest mesh read, in nodes.
const MESH_MAX_NODES: usize = 250_000;
/// The tag of records added by version 24 and later at the start of fill
/// records (a 16-byte id), which files saved for older versions carry too.
const NEWER_RECORD_TAG: u32 = 2400;

/// The width of the space character in ems, for character spacing.
const SPACE_EM: f64 = 0.25;
/// A guideline (seen in version 12 files on the guides layer).
const OBJ_GUIDE: u32 = 0x16;
/// Argument of a guideline: its line (see [`Ctx::read_guide`]).
const ARG_GUIDE_LINE: u32 = 16000;

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
    pub(crate) fn i16(&mut self) -> Option<i16> {
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
    png: tracedraw_core::SharedBytes,
}

/// One entry of a `loda` argument table: type, payload start and end.
#[derive(Debug, Clone, Copy)]
struct Arg {
    ty: u32,
    off: usize,
    end: usize,
}

/// What a paragraph text's frame may be worked out from when its
/// coordinates hold no size (version 16 and later).
struct FrameHints<'s> {
    /// The object's style string.
    style: Option<&'s str>,
    /// The object's box on the page (file coordinates).
    bbox: Option<Rect>,
    /// The object's transform (file coordinates).
    transform: Affine,
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
    /// Scale factors of the base shape the object's coordinates hold.
    rx: f64,
    ry: f64,
    cx: f64,
    cy: f64,
}

struct Ctx<'a> {
    main: &'a [u8],
    tree: &'a Tree,
    /// Effective major version (7 = CDR 7, 13 = version 13, 16 = version 16 ...).
    v: u16,
    fills: HashMap<u32, FillDef>,
    outlines: HashMap<u32, OutlineDef>,
    bitmaps: HashMap<u32, BitmapDef>,
    /// Pictures of objects embedded from other programs, by storage name
    /// (`ole2` chunks).
    embedded: HashMap<String, crate::ole::Picture>,
    /// Two-colour pattern tiles (`bmpf`), 1 bit per pixel: true = front.
    pattern_tiles: HashMap<u32, PatternBits>,
    /// Vector pattern drawings (`vpat`): an id, then a CMX file.
    vector_patterns: HashMap<u32, &'a [u8]>,
    /// `font` chunks by font id: family name and character set.
    fonts: HashMap<u16, text::FontRec>,
    /// Character set of names (layers, objects) before version 12: the
    /// non-Latin set most fonts use, else Latin (0).
    name_charset: u8,
    /// `stlt` style records by style id.
    styles: HashMap<u32, StyleRec>,
    /// `arrw` arrowhead definitions classified into our presets.
    arrows: HashMap<u32, Arrowhead>,
    /// Crop outline of the bitmap object being read (local coordinates).
    pending_crop: Option<BezPath>,
    /// Groups of `LIST clpt` by their `spnd` number: the contents of clip
    /// containers.
    clip_groups: HashMap<u32, &'a Chunk>,
    /// The `clpt` group the object being read holds clipped inside it.
    pending_clip: Option<u32>,
    /// Colours of spot colour libraries met: (library, number).
    library_inks: std::collections::BTreeSet<(u16, u16)>,
    /// The font names of a version 3 `ftbl`, in file order.
    v3_fonts: Vec<String>,
    v3_effect_warned: bool,
    stand_in_warned: bool,
    /// Blend steps the file's blends may still take together (#980).
    blend_steps_left: u32,
    /// Guidelines of the page being read (objects of type 0x16).
    guides: Vec<tracedraw_core::document::Guide>,
    /// The last object read was a guideline, not a skipped object.
    last_was_guide: bool,
    /// The transform of the linked groups (`LIST lnkg`) being read, which
    /// their objects are drawn through.
    outer_transform: Affine,
    /// The text object being read is fitted to a path (a sibling supplies
    /// the path).
    pending_on_path: Option<OnPathText>,
    /// Texts fitted to a path, by shape, until their list is read and the
    /// path found among their siblings.
    on_path: HashMap<tracedraw_core::ShapeId, OnPathText>,
    /// The transform of the paragraph text being read when its frame took
    /// the object's scale (see [`unscaled_frame`]).
    pending_text_transform: Option<Affine>,
    /// Facing pages (version 21 `mcfg` byte 26): objects are placed from
    /// the spine of the spread, not the page centre.
    facing_pages: bool,

    /// The node types a curve's node bytes give where its handles read
    /// otherwise, for the shape being read.
    pending_node_types: std::collections::BTreeMap<usize, tracedraw_core::nodes::NodeType>,
    /// The CMYK profile the file carries, for CMYK bitmaps.
    cmyk_profile: Option<tracedraw_core::icc::Profile>,
    /// CMYK to RGB through that profile, built for the first CMYK bitmap.
    cmyk_lut: Option<tracedraw_core::icc::CmykLut>,
    report: ParseReport,
}

/// A text the file fits to a path: how it sits on the path and where the
/// fitted text lies on the page. The path is one of its siblings.
#[derive(Debug, Clone, Copy, Default)]
struct OnPathText {
    settings: Option<text::PathSettings>,
    /// The fitted text's box on the page (the object's `bbox`, our page
    /// coordinates).
    page_box: Option<Rect>,
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
            embedded: HashMap::new(),
            pattern_tiles: HashMap::new(),
            vector_patterns: HashMap::new(),
            fonts: HashMap::new(),
            name_charset: 0,
            styles: HashMap::new(),
            arrows: HashMap::new(),
            pending_crop: None,
            clip_groups: HashMap::new(),
            pending_clip: None,
            guides: Vec::new(),
            last_was_guide: false,
            outer_transform: Affine::IDENTITY,
            library_inks: Default::default(),
            v3_fonts: Vec::new(),
            v3_effect_warned: false,
            stand_in_warned: false,
            blend_steps_left: FILE_BLEND_STEPS,
            pending_on_path: None,
            on_path: HashMap::new(),
            pending_text_transform: None,
            facing_pages: false,

            pending_node_types: Default::default(),
            cmyk_profile: None,
            cmyk_lut: None,
            report: ParseReport::default(),
        }
    }
}

pub fn parse_document(tree: &Tree, main: &[u8], version: Version) -> (Document, ParseReport) {
    parse_document_with(tree, main, version, None)
}

/// [`parse_document`] with the CMYK profile the file carries (an ICC
/// profile's bytes): CMYK bitmaps are converted to RGB through it, else
/// with the plain formula.
pub fn parse_document_with(
    tree: &Tree,
    main: &[u8],
    version: Version,
    cmyk_profile: Option<&[u8]>,
) -> (Document, ParseReport) {
    let mut ctx = Ctx::new(main, tree, version.0);
    ctx.report.version = Some(version);
    ctx.cmyk_profile = cmyk_profile
        .and_then(|b| tracedraw_core::icc::Profile::parse(b).ok())
        .filter(|p| p.color_space() == tracedraw_core::icc::ColorSpace::Cmyk);

    // The `vrsn` chunk carries the version as a number (1300 = version 13, 801 =
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
            ctx.report.version = Some(crate::container::Version(nv));
        }
    }

    // Pass 1: style tables and bitmaps, wherever they are in the tree.
    // Before version 7 the fill table chunk is called `fill`. Bitmaps,
    // pattern tiles and arrowheads go first because fills and outlines
    // refer to them.
    let mut bitmaps: Vec<&Chunk> = Vec::new();
    tree.root.walk(&mut |c, _| {
        if c.is(b"bmp ") {
            bitmaps.push(c);
        } else if c.is(b"bmpf") {
            ctx.read_pattern_tile(c);
        } else if c.is(b"vpat") {
            let d = ctx.data(c);
            if let (Some(id), Some(cmx)) = (ctx.reader(d).u32(), d.get(4..)) {
                ctx.vector_patterns.insert(id, cmx);
            }
        } else if c.is(b"arrw") {
            ctx.read_arrowhead(c);
        } else if c.is(b"arrx") {
            ctx.read_arrowhead_x(c);
        } else if c.is(b"font") {
            ctx.read_font(c);
        } else if c.is(b"ftbl") {
            // Version 3 font table: the font file names, NUL-terminated
            // runs of lowercase letters among other fields (assumed).
            let names = v3_font_names(ctx.data(c));
            ctx.v3_fonts = names;
        } else if c.is(b"ole2") {
            let pictures = crate::ole::embedded_pictures(ctx.data(c));
            ctx.embedded.extend(pictures);
        }
    });
    ctx.read_bitmaps(&bitmaps);
    ctx.index_clip_groups();
    ctx.name_charset = ctx.common_charset();
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
                // Version 21: byte 26 is 1 for facing pages (multi-page
                // files whose objects otherwise land half a page across).
                ctx.facing_pages = ctx.v >= 21 && ctx.data(c).get(26) == Some(&1);
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
    let mut master_background: Option<Fill> = None;
    for (i, pc) in pages.iter().enumerate() {
        let page_id = doc.ids_mut().page();
        let is_master = ctx.page_is_master(pc);
        master_flags.push(is_master);
        if is_master {
            master_background = ctx.page_background(pc);
        }
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
        for lc in &layer_chunks {
            let default_name = format!("Layer {}", page.layers.len() + 1);
            let name = ctx.layer_name(lc).unwrap_or(default_name);
            let mut layer = Layer::new(doc.ids_mut().layer(), name);
            // Guides and grid layers hold no drawable content.
            match ctx.layer_type(lc) {
                Some(t) if t & 0x0f == 0x0a => {
                    layer.visible = false;
                    layer.printable = false;
                }
                _ => layer.visible = true,
            }
            let mut ids = std::mem::take(doc.ids_mut());
            layer.shapes = ctx.read_objects(lc, &mut ids, size, 0);
            doc.set_ids(ids);
            // Guides, grid and desktop layers, and the master page's layers
            // each page lists again, are no drawing of their own when
            // empty.
            if layer.shapes.is_empty() && ctx.is_structural_layer(lc) {
                continue;
            }
            page.layers.push(layer);
        }
        // Facing pages measure objects from the spine of the spread, half
        // a page to the left or right of the page's centre; which way is
        // not worked out yet, so the side holding more of the page's
        // objects wins (version 21 files: the first page's objects lie
        // right of the spine, later pages' mostly left of it).
        if ctx.facing_pages && !is_master {
            shift_onto_page(&mut page.layers, size.width);
        }
        // Files list a page's layers front to back, as they list objects
        // (a version 21 file: the layer of the lettering first, then the
        // layer whose full-page background its preview shows under it).
        page.layers.reverse();
        if page.layers.is_empty() {
            page.layers
                .push(Layer::new(doc.ids_mut().layer(), "Layer 1"));
        }
        page.guides = std::mem::take(&mut ctx.guides);
        // The master page's background shows on every page.
        page.background = master_background.clone();
        doc.pages.push(page);
    }

    // Drop a leading master page that carries nothing, keeping at least one page.
    let first_is_master = master_flags.first().copied().unwrap_or(false);
    if doc.pages.len() > 1
        && (first_is_master || doc.pages[0].layers.iter().all(|l| l.shapes.is_empty()))
        && doc.pages[0].layers.iter().all(|l| l.shapes.is_empty())
    {
        // The master page's guidelines show on every page.
        let master = doc.pages.remove(0);
        for (i, p) in doc.pages.iter_mut().enumerate() {
            p.name = format!("Page {}", i + 1);
            for g in &master.guides {
                if !p.guides.contains(g) {
                    p.guides.push(*g);
                }
            }
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

    if !ctx.library_inks.is_empty() {
        let n = ctx.library_inks.len();
        for (palette, number) in &ctx.library_inks {
            doc.inks.push(crate::spot::ink_entry(*palette, *number));
        }
        ctx.report.warn(format!(
            "{n} spot colour(s) from colour libraries: the file names them by number only"
        ));
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

    /// A guides, grid or desktop layer (type byte not 0), or a page's
    /// entry for a layer of the master page (byte 2 set).
    fn is_structural_layer(&self, lc: &Chunk) -> bool {
        matches!(self.flags(lc), Some(f) if f[3] == 0x98 && (f[0] != 0 || f[2] != 0))
    }

    /// The `loda`/`lobj` chunk of a list: a direct child, or (version 16+,
    /// seen in version 17 to 21 files) the one in its `lgob` list.
    fn own_loda<'c>(&self, list: &'c Chunk) -> Option<&'c Chunk> {
        let direct = |l: &'c Chunk| l.children.iter().find(|c| c.is(b"loda") || c.is(b"lobj"));
        direct(list).or_else(|| list.find(b"lgob").and_then(direct))
    }

    fn layer_name(&mut self, lc: &Chunk) -> Option<String> {
        let loda = self.own_loda(lc)?;
        let d = self.data(loda);
        let l = self.read_loda(d)?;
        let arg = l.args.iter().find(|a| a.ty == ARG_NAME)?;
        let name = self.read_name(d.get(arg.off..arg.end)?);
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
        if w > 1.0 && h > 1.0 && w < MAX_PAGE_MM && h < MAX_PAGE_MM {
            Some(Size::new(w, h))
        } else {
            None
        }
    }

    /// The page background in a master page's `loda` (argument 17000,
    /// version 21): a record tagged 1300 with its length, 1 for a solid
    /// colour, and the colour's property list (id 1, length 12, the
    /// colour record) 576 bytes into the argument. Other kinds are not
    /// read.
    fn page_background(&mut self, pc: &Chunk) -> Option<Fill> {
        let loda = self.own_loda(pc)?;
        let d = self.data(loda);
        let l = self.read_loda(d)?;
        let arg = l.args.iter().find(|a| a.ty == ARG_PAGE_BACKGROUND)?;
        let body = d.get(arg.off..arg.end)?;
        let mut r = self.reader_at(body, 0);
        let (tag, _len, kind) = (r.u32()?, r.u32()?, r.u32()?);
        if tag != 0x514 || kind != 1 {
            return None;
        }
        let mut r = self.reader_at(body, 576);
        if r.u8()? != 1 || r.u32()? != 12 {
            return None;
        }
        self.read_color(&mut r).map(Fill::Solid)
    }

    /// `mcfg`: page width and height. Leading unknown bytes depend on the
    /// version: 12 (version 13+), 4 (9 to 12), 28 (6), none before.
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
            // The corners run top-left to bottom-right (y down) in the
            // version 3 files seen.
            ((x1 - x0).abs(), (y1 - y0).abs())
        } else {
            (r.coord()?, r.coord()?)
        };
        if w > 1.0 && h > 1.0 && w < MAX_PAGE_MM && h < MAX_PAGE_MM {
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
        let mut palette = 0u16;
        let (model, b) = if self.v >= 5 {
            let model = r.u16()?;
            palette = r.u16()?;
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
        // Before version 5, model 1 is CMYK in percent (version 4 as u16s,
        // version 3 as bytes), and model 0 (version 3) a palette entry: a
        // u16 index and a u16 tint, index 0 black at the tint, 1 white (as in
        // the files of versions 1 and 2). Seen in the same drawings saved in
        // versions 3, 4 and 5. A version 4 index above 255 is not kept.
        if self.v < 5 && model == 1 {
            let p = |x: u8| (x as f32).min(100.0);
            return Some(Color::cmyk_pct(p(b[0]), p(b[1]), p(b[2]), p(b[3])));
        }
        if self.v < 5 && model == 0 {
            // Version 4 keeps the index and tint as its first two u16s.
            let (index, tint) = if self.v >= 4 {
                (b[0] as u16, b[1] as u16)
            } else {
                (
                    u16::from_le_bytes([b[0], b[1]]),
                    u16::from_le_bytes([b[2], b[3]]),
                )
            };
            return Some(match index {
                1 => Color::cmyk_pct(0.0, 0.0, 0.0, 0.0),
                0 => Color::cmyk_pct(0.0, 0.0, 0.0, (tint as f32).min(100.0)),
                // Other entries are colours of a spot colour library, by
                // number (the numbers seen match a printing ink guide's;
                // the file does not hold the colours): spot inks, coloured
                // from the drawing's thumbnail when it has one.
                number => {
                    self.library_inks.insert((crate::spot::V3_PALETTE, number));
                    crate::spot::library_color(crate::spot::V3_PALETTE, number, tint)
                }
            });
        }
        // Model 0 with a zero value is the empty colour some records keep
        // in place of one (seen in version 15 and 22 files, where nothing
        // shows it): black, as before, but without a warning.
        if model == 0 && b == [0; 4] {
            return Some(Color::BLACK);
        }
        // A colour of a spot colour library (model 1, and 0x0e for the
        // libraries numbered in hex; seen in versions 12 to 22): its number
        // in the library (u16) and a tint in percent (u16), the palette
        // field naming the library. The colour itself is not in the file:
        // it becomes a spot ink, black until the file's preview tells
        // (`spot::colours_from_preview`).
        if model == 0x01 || model == 0x0e || model == 0x19 {
            let number = u16::from_le_bytes([b[0], b[1]]);
            let tint = u16::from_le_bytes([b[2], b[3]]);
            self.library_inks.insert((palette, number));
            return Some(crate::spot::library_color(palette, number, tint));
        }
        Some(match color_from_model(model, b) {
            Some(c) => c,
            None => {
                self.report.warn(format!(
                    "colour model 0x{model:02x} (value {b:02x?}) not modelled; using black"
                ));
                Color::BLACK
            }
        })
    }

    /// `fild` (`fill` before version 7): fill definition. We model solid
    /// fills, fountain fills with all their stops, two-colour patterns,
    /// colour bitmap fills, textures (from their stored bitmap) and vector
    /// patterns (from their CMX drawing); PostScript fills become "no
    /// fill" with a warning.
    fn read_fill(&mut self, c: &Chunk) {
        let d = self.data(c);
        let mut r = self.reader(d);
        let Some(id) = r.u32() else { return };
        let v = self.v;
        let mut body_end = d.len();
        if v >= 13 {
            // version 13+: a "since version" tag (1300) and the body length.
            // Files written by version 24 and later start with a record of
            // their own (tag 2400, a 16-byte id), in files saved for older
            // versions too; it is skipped.
            for _ in 0..4 {
                let (Some(since), Some(len)) = (r.u32(), r.u32()) else {
                    return;
                };
                if since >= NEWER_RECORD_TAG {
                    r.pos = r.pos.saturating_add(len as usize);
                    continue;
                }
                body_end = body_end.min(r.pos.saturating_add(len as usize));
                break;
            }
        }
        let Some(ftype) = r.u16() else { return };
        let Some(body) = d.get(r.pos..body_end.max(r.pos)) else {
            return;
        };
        if log::log_enabled!(log::Level::Trace) {
            let hex: Vec<String> = body.iter().take(160).map(|b| format!("{b:02x}")).collect();
            log::trace!("fill {id:#x} type {ftype}: {}", hex.join(" "));
        }
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
            6 => self.stand_in_fill(6),
            10 => self.read_vector_pattern_fill(body).unwrap_or_else(|| {
                self.report.warn(format!(
                    "vector pattern fill {id} not readable yet; no fill"
                ));
                Fill::None
            }),
            other => {
                self.report
                    .warn(format!("fill type {other} not modelled yet (fill id {id})"));
                Fill::None
            }
        };
        self.fills.insert(id, FillDef { fill });
    }

    /// Vector pattern fill body (type 10; seen in version 18): tagged
    /// records, 1600 holding the part of the pattern's drawing that makes
    /// one tile (four i32: left, top, right, bottom, from the centre of
    /// its page) and 1300 holding the tile's width and height (u32 each),
    /// 8 bytes, that box again and the id of the `vpat` that holds the
    /// drawing as a CMX file. The tile is that part of the drawing
    /// stretched to the tile's size.
    fn read_vector_pattern_fill(&mut self, body: &'a [u8]) -> Option<Fill> {
        let mut r = self.reader(body);
        let mut part: Option<[i32; 4]> = None;
        let mut tile: Option<(u32, u32, u32)> = None;
        while r.remaining() >= 8 {
            let tag = r.u32()?;
            let len = r.u32()? as usize;
            let start = r.pos;
            match tag {
                1600 if len >= 16 => part = Some([r.i32()?, r.i32()?, r.i32()?, r.i32()?]),
                1300 if len >= 36 => {
                    let w = r.u32()?;
                    let h = r.u32()?;
                    let id = self.reader_at(body, start + 32).u32()?;
                    tile = Some((w, h, id));
                }
                _ => {}
            }
            r.pos = start.checked_add(len)?;
        }
        let ([x0, top, x1, bottom], (w, h, id)) = (part?, tile?);
        let cmx = *self.vector_patterns.get(&id)?;
        let (drawing, _) = crate::cmx::open_bytes(cmx, "pattern").ok()?;
        let page = drawing.pages.first()?;
        let centre = Point::new(page.size.width / 2.0, page.size.height / 2.0);
        let left = centre.x + x0 as f64 * UNIT_MM;
        let right = centre.x + x1 as f64 * UNIT_MM;
        let low = centre.y + bottom as f64 * UNIT_MM;
        let high = centre.y + top as f64 * UNIT_MM;
        let tile = Size::new(w as f64 * UNIT_MM, h as f64 * UNIT_MM);
        if !(right - left > 1e-6 && high - low > 1e-6 && tile.width > 1e-6 && tile.height > 1e-6) {
            return None;
        }
        let to_tile =
            Affine::scale_non_uniform(tile.width / (right - left), tile.height / (high - low))
                * Affine::translate((-left, -low));
        let shapes: Vec<Shape> = page
            .layers
            .iter()
            .flat_map(|l| l.shapes.iter().cloned())
            .map(|mut s| {
                s.transform = to_tile * s.transform;
                s
            })
            .collect();
        if shapes.is_empty() {
            return None;
        }
        Some(Fill::Pattern(tracedraw_core::style::Pattern::Vector {
            shapes,
            tile,
        }))
    }

    /// Solid fill body. Before version 13: 2 unknown bytes then a colour. From version 13:
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
            stops.push(Stop::new(pos.clamp(0.0, 1.0), color));
        }
        if stops.is_empty() {
            return None;
        }
        if stops.len() == 1 {
            let only = stops[0];
            stops.push(Stop::new(1.0, only.color));
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
        // version 16+: four doubles relative to the object (centre x, centre y,
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
        let mut fountain = Fountain {
            kind,
            stops,
            angle,
            offset,
            edge_pad,
            ..Default::default()
        };
        crate::mirror_centred_stops(&mut fountain);
        Some(Fill::Fountain(fountain))
    }

    /// Colour bitmap (9) and texture (11) fill body: the pattern id, tile
    /// width and height, tile offsets or 4 unknown bytes, rcp offset,
    /// flags, 21 (17 from version 13) unknown bytes and the pattern id again
    /// (version 6+), which names a `bmp ` image. From version 13 that
    /// layout is the body of a record tagged 1300 (0x514) with its length,
    /// after other tagged records such as 1600 (0x640): a version 22 file
    /// has a 16-byte 1600 record and a 36-byte 1300 record whose last u32
    /// names the image.
    fn read_image_fill(&mut self, body: &'a [u8], id: u32, ftype: u16) -> Option<Fill> {
        let v = self.v;
        // The layout at `pos`: (image id, tile width, flags). In a record
        // of `len` bytes the image id is its last u32.
        let layout_at = |pos: usize, len: Option<usize>| -> Option<(u32, u32, u8)> {
            let mut r = self.reader_at(body, pos);
            let first = r.uint()?;
            let w = r.uint()?;
            let _h = r.uint()?;
            r.skip(4); // tile offsets (before 9) or unknown
            let _rcp = r.u16()?;
            let flags = r.u8()?;
            let image = match len {
                Some(l) => self.reader_at(body, (pos + l).checked_sub(4)?).u32()?,
                None => {
                    r.skip(if v >= 13 { 17 } else { 21 });
                    if v >= 6 {
                        r.u32().unwrap_or(first)
                    } else {
                        first
                    }
                }
            };
            Some((image, w, flags))
        };
        let reading = if v >= 13 {
            // Tagged records with their lengths, up to the 1300 one.
            let mut pos = 0;
            let mut tagged = None;
            for _ in 0..64 {
                let mut peek = self.reader_at(body, pos);
                let (Some(tag), Some(len)) = (peek.u32(), peek.u32()) else {
                    break;
                };
                let len = len as usize;
                if len > peek.remaining() || !(tag == 0x640 || tag == 0x514) {
                    break;
                }
                if tag == 0x514 {
                    tagged = layout_at(pos + 8, Some(len));
                    break;
                }
                pos += 8 + len;
            }
            // The public notes' reading: a 0x640 word followed by 0x640
            // bytes, or a bare 0x514 word, before the layout.
            let mut pos = 0;
            for _ in 0..64 {
                match self.reader_at(body, pos).u32() {
                    Some(0x640) => pos += 4 + 0x640,
                    Some(0x514) => pos += 4,
                    _ => break,
                }
            }
            let bare = layout_at(pos, None);
            match tagged {
                Some(t) if self.bitmaps.contains_key(&t.0) => Some(t),
                _ => bare.or(tagged),
            }
        } else {
            layout_at(2, None)
        };
        let (pattern_id, w, flags) = reading?;
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
                png: encode_png(tile.width, tile.height, &rgba).into(),
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
        let pixels_start = pal + 8;
        let Some(pixels) = d.get(pixels_start..) else {
            return;
        };
        // Rows pad to 4 bytes; version 3 tiles pad them to 2 (a 16 x 16
        // tile in 32 bytes).
        let stride =
            if self.v < 4 && pixels.len() < (width as usize).div_ceil(32) * 4 * height as usize {
                (width as usize).div_ceil(16) * 2
            } else {
                (width as usize).div_ceil(32) * 4
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

    /// `arrw`: an arrowhead definition, seen in version 11 and 12 files:
    /// id u32, 4 bytes, the point count u16 at byte 8, 4 bytes, the node
    /// types, one byte, then the points (i32 pairs) and one spare byte.
    /// The outline is classified into one of our presets by its shape.
    fn read_arrowhead(&mut self, c: &Chunk) {
        let d = self.data(c);
        if self.v < 5 {
            return self.read_arrowhead_v3(d);
        }
        let mut r = self.reader(d);
        let Some(id) = r.u32() else { return };
        let n = self.reader_at(d, 8).u16().map_or(0, usize::from);
        if n == 0 || 15 + 9 * n > d.len() {
            self.report.warn(format!(
                "arrowhead {id}: unreadable outline; drawn without arrowhead"
            ));
            return;
        }
        let Some(types) = d.get(14..14 + n).map(<[u8]>::to_vec) else {
            return;
        };
        let mut r = self.reader_at(d, 15 + n);
        let mut pts = Vec::with_capacity(n);
        for _ in 0..n {
            let (Some(x), Some(y)) = (r.i32(), r.i32()) else {
                return;
            };
            pts.push(Point::new(f64::from(x), f64::from(y)));
        }
        self.add_arrowhead(id, &pts, &types);
    }

    /// `arrw` of versions 3 and 4: the id (u32; version 4 a u16 and the
    /// u16 10), a u16, the point count (u16 at 6), where the points start
    /// (u16 at 8, counted from byte 6), the node types at 10 (at 12 when
    /// the u16 at 10 is 1) and the points as i16 pairs. Seen in one drawing
    /// saved in both versions; the outlines refer to these ids.
    fn read_arrowhead_v3(&mut self, d: &'a [u8]) {
        let id = if self.v >= 4 {
            self.reader_at(d, 0).u16().map(u32::from)
        } else {
            self.reader_at(d, 0).u32()
        };
        let Some(id) = id else { return };
        let n = self.reader_at(d, 6).u16().map_or(0, usize::from);
        let points_at = 6 + self.reader_at(d, 8).u16().map_or(0, usize::from);
        let types_at = if self.reader_at(d, 10).u16() == Some(1) {
            12
        } else {
            10
        };
        if n == 0 || types_at + n > points_at || points_at + 4 * n > d.len() {
            self.report.warn(format!(
                "arrowhead {id}: unreadable outline; drawn without arrowhead"
            ));
            return;
        }
        let Some(types) = d.get(types_at..types_at + n).map(<[u8]>::to_vec) else {
            return;
        };
        let mut r = self.reader_at(d, points_at);
        let mut pts = Vec::with_capacity(n);
        for _ in 0..n {
            let (Some(x), Some(y)) = (r.i16(), r.i16()) else {
                return;
            };
            pts.push(Point::new(f64::from(x), f64::from(y)));
        }
        self.add_arrowhead(id, &pts, &types);
    }

    /// `arrx` (in the `arrt` list of newer files): an arrowhead definition.
    /// Layout from one sample file (assumed beyond it): id u32, then at
    /// byte 268 a u16 point count, 4 bytes, the node types and the points
    /// (i32 pairs).
    fn read_arrowhead_x(&mut self, c: &Chunk) {
        let d = self.data(c);
        let mut r = self.reader(d);
        let Some(id) = r.u32() else { return };
        let mut r = self.reader_at(d, ARRX_COUNT_AT);
        let n = r.u16().map_or(0, usize::from);
        if n == 0 || ARRX_COUNT_AT + 6 + 9 * n > d.len() {
            self.report.warn(format!(
                "arrowhead {id}: unreadable outline; drawn without arrowhead"
            ));
            return;
        }
        r.skip(4);
        let mut types = Vec::with_capacity(n);
        for _ in 0..n {
            let Some(t) = r.u8() else { return };
            types.push(t);
        }
        let mut pts = Vec::with_capacity(n);
        for _ in 0..n {
            let (Some(x), Some(y)) = (r.i32(), r.i32()) else {
                return;
            };
            pts.push(Point::new(f64::from(x), f64::from(y)));
        }
        self.add_arrowhead(id, &pts, &types);
    }

    /// Classifies an arrowhead outline into a preset, or keeps it as a
    /// custom head.
    fn add_arrowhead(&mut self, id: u32, pts: &[Point], types: &[u8]) {
        let (path, _) = build_path(pts, types);
        let mut head = classify_arrowhead(&path, pts, types);
        if head == Arrowhead::None {
            // Not one of our presets: keep the outline itself as a custom
            // arrowhead (assumed: the file draws the tip towards +x on a
            // line running left to right, like our custom heads).
            let unit = if self.v16() { UNIT16_MM } else { UNIT_MM };
            let mm = Affine::scale(unit) * path;
            head = Arrowhead::from_shape_path(&mm, format!("Arrowhead {id}"));
            if head == Arrowhead::None {
                self.report.warn(format!(
                    "arrowhead {id}: unreadable outline; drawn without arrowhead"
                ));
            }
        }
        self.arrows.insert(id, head);
    }

    /// `font`: id, family name and character set.
    fn read_font(&mut self, c: &Chunk) {
        let d = self.data(c);
        if let Some(f) = text::read_font(d, self.v) {
            self.fonts.insert(f.id, f);
        }
    }

    /// The non-Latin character set most fonts use (Latin, 0, when none
    /// does): the code page names were most likely typed in.
    fn common_charset(&self) -> u8 {
        let mut counts: HashMap<u8, usize> = HashMap::new();
        for f in self.fonts.values() {
            if f.charset != 2 && text::is_non_latin_charset(f.charset) {
                *counts.entry(f.charset).or_default() += 1;
            }
        }
        counts
            .into_iter()
            .max_by_key(|(cs, n)| (*n, std::cmp::Reverse(*cs)))
            .map_or(0, |(cs, _)| cs)
    }

    /// A layer or object name: UTF-16LE from version 12, before that
    /// single-byte (or double-byte) text in the code page of the fonts.
    fn read_name(&self, b: &[u8]) -> String {
        if self.v >= 12 {
            text::decode_utf16le(b)
        } else {
            text::decode_charset(b, self.name_charset)
        }
    }

    /// The text of a run with its style resolved: before version 12 its
    /// bytes in the code page of its character set, the style's or else its
    /// font's (a non-Latin set wins: a Latin one tells nothing).
    fn run_text(&self, run: &text::TextRun, style: &text::RunStyle) -> String {
        let Some(raw) = &run.raw else {
            return run.text.clone();
        };
        let font = style
            .font_id
            .and_then(|id| self.fonts.get(&id))
            .map(|f| f.charset);
        let charset = [style.charset, font]
            .into_iter()
            .flatten()
            .find(|cs| text::is_non_latin_charset(*cs));
        match charset {
            Some(cs) => text::decode_charset(raw, cs),
            None => run.text.clone(),
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
            .or_else(|| {
                style
                    .font_id
                    .and_then(|id| self.fonts.get(&id).map(|f| f.name.clone()))
            })
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
        let mut weight = None;
        if let Some(f) = style.flags {
            span.bold = f.bold();
            span.italic = f.italic();
            span.underline = f.underline();
            span.strikethrough = f.strikethrough();
            weight = f.weight();
        }
        if let Some(b) = style.bold {
            span.bold = b;
            weight = style.weight;
        }
        name_weight(&mut span, weight);
        if let Some(i) = style.italic {
            span.italic = i;
        }
        if let Some(u) = style.underline {
            span.underline = u;
        }
        if let Some(t) = style.strikethrough {
            span.strikethrough = t;
        }
        if let Some(cs) = style.char_spacing {
            if cs != 0.0 {
                // The file counts character spacing in widths of the
                // space character, taken here as a quarter of the em.
                span.tracking_pct = cs * 100.0 * SPACE_EM;
            }
        }
        if let Some(c) = style.char_shift {
            span.angle_deg = c.angle_deg;
            span.offset_x_pct = c.x_pct;
            span.baseline_shift_pt = c.y_pct / 100.0 * size_pt;
        }
        if let Some(inline) = style.fill_color {
            // Version 16+ files describe the run's colour inline.
            let fill = inline.map(Fill::Solid).unwrap_or(Fill::None);
            if fill != *object_fill {
                span.fill = Some(fill);
            }
        } else if let Some(fid) = style.fill_id {
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
    #[allow(clippy::too_many_arguments)]
    fn build_text(
        &mut self,
        kind_code: u32,
        d: &'a [u8],
        coords: Option<Arg>,
        oc: &Chunk,
        style_id: Option<u32>,
        object_fill: &Fill,
        hints: FrameHints,
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
                    // The frame hangs from (0, 0), its top left (seen in a
                    // version 22 file: the transform moves (0, 0) to the
                    // top left of the object's box).
                    frame = Some(Size::new(w.abs(), h.abs()));
                    origin = Point::new(0.0, -h.abs());
                }
            } else if self.v >= 16 {
                let o = Point::new(r.coord()?, r.coord()?);
                if o.x.abs() <= MAX_TEXT_ORIGIN_MM && o.y.abs() <= MAX_TEXT_ORIGIN_MM {
                    origin = o;
                }
            }
            // Before version 16 the argument holds record ids, not a
            // place (files of versions 7 to 12; in 13 to 15 (0, 0) or
            // (0, a record id), read as a point 9 to 40 m away): the text
            // starts at the origin and the transform places it.
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
        self.pending_on_path = parsed.on_path.then_some(OnPathText {
            settings: parsed.path_settings,
            page_box: None,
        });
        // Artistic text (type 0x04) keeps its first baseline at the origin
        // even when its text record has the frame layout (version 21; the
        // object's box is then the box of its letters).
        if frame.is_none() && kind_code == OBJ_PARAGRAPH_TEXT {
            // Version 16+: the frame is the object's box in its own space,
            // its width the columns of the object's style.
            match self.frame_from_hints(&hints) {
                Some((o, size)) => {
                    origin = o;
                    frame = Some(size);
                }
                // A frame without readable size: lay the text out unframed.
                None => self
                    .report
                    .warn("paragraph text without a readable frame; treated as artistic text"),
            }
        }
        // Paragraph text keeps its letters' size when its frame is scaled:
        // the frame takes the transform's scale and the text is laid out
        // in it unscaled (a version 22 file's export draws text at its
        // point size in frames scaled to 39 % and 72 % of their height).
        if kind_code == OBJ_PARAGRAPH_TEXT {
            if let Some((t, o, s)) = frame.and_then(|f| unscaled_frame(hints.transform, origin, f))
            {
                origin = o;
                frame = Some(s);
                self.pending_text_transform = Some(t);
            }
        }
        // The object's own style string (version 16+: its paragraph
        // alignment, spacing and character settings), over the style
        // table's entry.
        let table_style = style_id
            .map(|id| self.style_defaults(id))
            .unwrap_or_default();
        let object_style = match hints.style {
            Some(st) => text::parse_style_string(st).over(&table_style),
            None => table_style,
        };
        let mut spans: Vec<TextSpan> = Vec::new();
        let mut align: Option<u32> = object_style.align;
        let mut para_fmt = ParagraphStyle::default();
        // Each paragraph's own alignment (its style string from version
        // 16, its style table entry before) and spacing, with where it
        // starts in the text.
        let mut own: Vec<(usize, Option<u32>, ParagraphStyle)> = Vec::new();
        let mut chars = 0usize;
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
            let own_align = para.base.align.or(if para.style_id != 0 {
                self.style_defaults(para.style_id).align
            } else {
                None
            });
            let fmt = paragraph_spacing(&base, &para_fmt);
            if i == 0 {
                // The first paragraph sets the object's formatting.
                para_fmt = fmt.clone();
            }
            own.push((chars, own_align, fmt));
            for (k, run) in para.runs.iter().enumerate() {
                let style = run.style.over(&base);
                let mut text = text::line_breaks(&self.run_text(run, &style));
                if i + 1 < n && k + 1 == para.runs.len() {
                    text.push('\n');
                }
                chars += text.chars().count();
                let span = self.make_span(text, &style, object_fill);
                match spans.last_mut() {
                    Some(last)
                        if last.font_family == span.font_family
                            && last.size_pt == span.size_pt
                            && last.bold == span.bold
                            && last.italic == span.italic
                            && last.underline == span.underline
                            && last.strikethrough == span.strikethrough
                            && last.fill == span.fill
                            && last.angle_deg == span.angle_deg
                            && last.offset_x_pct == span.offset_x_pct
                            && last.baseline_shift_pt == span.baseline_shift_pt =>
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
        // 0 none, 1 left, 2 centre, 3 right, 4 full and 5 forced justify
        // (a centred version 12 paragraph stores 2).
        let code = |a: u32| match a {
            2 => TextAlign::Center,
            3 => TextAlign::Right,
            4 => TextAlign::Justify,
            5 => TextAlign::ForceJustify,
            _ => TextAlign::Left,
        };
        let align = code(align.unwrap_or(0));
        let mut kind = ShapeKind::Text {
            spans,
            origin,
            frame,
            align,
            para: para_fmt.clone(),
            on_path: None,
        };
        // Paragraphs formatted apart from one another keep their own
        // alignment and spacing; text whose paragraphs all agree keeps the
        // object's, as before.
        let aligns_differ = own.windows(2).any(|w| w[0].1 != w[1].1);
        let spacing_differs = own.iter().any(|(_, _, f)| *f != para_fmt);
        if aligns_differ || spacing_differs {
            for (start, own_align, fmt) in &own {
                let a = match own_align {
                    Some(a) if aligns_differ => code(*a),
                    _ => align,
                };
                let target = tracedraw_core::ParagraphFormat::of(a, fmt);
                tracedraw_core::paragraphs::edit_paragraph_formats(
                    &mut kind,
                    Some((*start, *start)),
                    |f| *f = target.clone(),
                );
            }
        }
        Some(kind)
    }

    /// `outl`: outline definition, see `docs/cdr-format.md` for the layout.
    fn read_outline(&mut self, c: &Chunk) {
        let d = self.data(c);
        let mut r = self.reader(d);
        let Some(id) = r.u32() else { return };
        let v = self.v;
        if v >= 13 {
            // version 13+: tagged records (id u32, len u32, body) until id 1,
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
        // Versions 6 to 12 put two unknown bytes before the nib angle and
        // four before the outline's matrix (six doubles); from version 13
        // the angle follows the stretch and the matrix the angle (files of
        // versions 14 to 22 have the matrix's 1.0 entries there).
        if (6..13).contains(&v) {
            r.skip(2);
        }
        let nib_angle = r.angle().unwrap_or(0.0);
        r.skip(if v >= 13 {
            48
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
            let dashes_at = r.pos;
            let n = (n as usize).min(r.remaining() / 2).min(64);
            for _ in 0..n {
                if let Some(x) = r.u16() {
                    dash.push(x as f64);
                }
            }
            // The marker ids follow a fixed dash area: 22 bytes after the
            // first dash in the untagged records before version 13 (files
            // of versions 11 and 12), 20 in the tagged ones (version 21).
            let mut m = self.reader_at(d, dashes_at + if (6..13).contains(&v) { 22 } else { 20 });
            if let (Some(s), Some(e)) = (m.u32(), m.u32()) {
                // Version 4 keeps a u16 id and the u16 10, as its `arrw`.
                markers = if v == 4 {
                    (s & 0xffff, e & 0xffff)
                } else {
                    (s, e)
                };
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
            Some(a) => a.clone(),
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
            // Before version 5 the matrix is six i32: the four factors in
            // 16.16 fixed point, the offsets in coordinate units.
            if v < 5 {
                // Several matrices apply one after the other.
                match fixed_matrix(&mut tr) {
                    Some(m) => result = Some(m * result.unwrap_or(Affine::IDENTITY)),
                    None => self.report.warn("trfd too short; identity used"),
                }
                continue;
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
            // Several matrices apply one after the other (seen in a
            // version 5 file whose labels are placed by a second, moving
            // matrix after a first, scaling one).
            let m = Affine::new([a, b, c_, d_, e, f]);
            result = Some(m * result.unwrap_or(Affine::IDENTITY));
        }
        result.unwrap_or(Affine::IDENTITY)
    }

    /// `loda` header and argument table. Header fields are u16 before
    /// version 6, u32 after: length, argument count, offset of the offsets
    /// table, offset of the types table, object type. The offsets table has
    /// one extra entry marking the end of the last argument, and the types
    /// table is stored in reverse order relative to the offsets.
    /// The object style string argument (version 16 and later): a u32,
    /// the length, then the text (UTF-16 before version 17, one byte per
    /// character from 17 on).
    fn read_object_style_string(&self, d: &'a [u8], arg: Arg) -> Option<String> {
        let mut r = self.reader_at(d.get(..arg.end)?, arg.off);
        r.skip(4);
        let len = r.u32()? as usize;
        let bytes = if self.v < 17 {
            len.checked_mul(2)?
        } else {
            len
        };
        let s = d.get(r.pos..r.pos.checked_add(bytes)?.min(arg.end))?;
        Some(if self.v < 17 {
            text::decode_utf16le(s)
        } else {
            String::from_utf8_lossy(s).into_owned()
        })
    }

    /// The object's `bbox` chunk: its box on the page (file coordinates,
    /// origin at the page centre).
    /// Object type 0x26 (version 22 files) keeps its points in the `ppdt`
    /// record of its `LIST npps`, in one of two layouts:
    /// - a connector line: u32 0, u32 1, then the start and the end point
    ///   (coordinates), then what the ends attach to. The file's preview
    ///   draws each as a straight line between the two points;
    /// - a list of points: u16 count, u32 1, the points (coordinates), then
    ///   a u32 for each point (0 or 1), 6 + 12 bytes a point in all. Seen
    ///   as the frames of PowerClips, closed outlines of some hundred
    ///   points whose last point lies a step from the first; the preview
    ///   clips the contents to the outline through the points.
    fn read_connector(&mut self, oc: &Chunk) -> Option<ShapeKind> {
        let mut found = None;
        oc.walk(&mut |c, _| {
            if found.is_none() && c.is(b"ppdt") {
                found = Some(c);
            }
        });
        let d = self.data(found?);
        let n = u16::from_le_bytes([*d.first()?, *d.get(1)?]) as usize;
        if n >= 2 && d.len() == 6 + 12 * n && d.get(2..6) == Some(&[1, 0, 0, 0][..]) {
            let mut r = self.reader_at(d, 6);
            let mut path = BezPath::new();
            for i in 0..n {
                let p = Point::new(r.coord()?, r.coord()?);
                if !(p.x.is_finite() && p.y.is_finite()) {
                    return None;
                }
                if i == 0 {
                    path.move_to(p);
                } else {
                    path.line_to(p);
                }
            }
            path.close_path();
            return Some(ShapeKind::Path { path, closed: true });
        }
        let mut r = self.reader_at(d, 8);
        let a = Point::new(r.coord()?, r.coord()?);
        let b = Point::new(r.coord()?, r.coord()?);
        if ![a.x, a.y, b.x, b.y].iter().all(|v| v.is_finite()) {
            return None;
        }
        let mut path = BezPath::new();
        path.move_to(a);
        path.line_to(b);
        Some(ShapeKind::Path {
            path,
            closed: false,
        })
    }

    /// An embedded object drawn as its picture: coordinates are the size
    /// (two coordinates) and two opposite corners.
    fn read_embedded(
        &mut self,
        d: &[u8],
        coords: Option<Arg>,
        name: Option<&str>,
    ) -> Option<ShapeKind> {
        let Some(name) = name else {
            // Version 21 files keep such objects in `embed/` members that
            // hold no picture (seen: media players' saved state).
            self.report
                .warn("an object embedded from another program is not shown");
            return None;
        };
        let Some((w, h, png)) = self.embedded.get(name).cloned() else {
            self.report.warn(format!(
                "embedded object \"{name}\" has no picture we can show"
            ));
            return None;
        };
        let mut r = self.reader_at(d, coords?.off + 8);
        let (x0, y0, x1, y1) = (r.coord()?, r.coord()?, r.coord()?, r.coord()?);
        let rect = Rect::new(x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1));
        if !(rect.width() > 0.0 && rect.height() > 0.0) {
            return None;
        }
        Some(ShapeKind::Bitmap {
            rect,
            width_px: w,
            height_px: h,
            png: png.into(),
            fx: None,
        })
    }

    /// A guideline's line (argument 16000, versions 6 and later): 12 bytes
    /// not used here, the angle (millionths of a degree, 90 for vertical
    /// lines; the points say the same), then two points the line passes
    /// through (and a third, the second again). Points
    /// are from the page centre like the objects'; vertical and
    /// horizontal lines become plain guidelines. Seen in version 12 files.
    fn read_guide(
        &self,
        d: &[u8],
        arg: Arg,
        page_size: Size,
    ) -> Option<tracedraw_core::document::Guide> {
        use tracedraw_core::document::{Guide, GuideLine};
        if self.v16() || arg.end.saturating_sub(arg.off) < 32 {
            return None;
        }
        let mut r = self.reader_at(d, arg.off + 16);
        let (x1, y1, x2, y2) = (r.coord()?, r.coord()?, r.coord()?, r.coord()?);
        if ![x1, y1, x2, y2].iter().all(|v| v.is_finite()) {
            return None;
        }
        let (cx, cy) = (page_size.width / 2.0, page_size.height / 2.0);
        let eps = 1e-6;
        let line = if (x1 - x2).abs() < eps && (y1 - y2).abs() >= eps {
            GuideLine::Vertical { x: x1 + cx }
        } else if (y1 - y2).abs() < eps && (x1 - x2).abs() >= eps {
            GuideLine::Horizontal { y: y1 + cy }
        } else if (x1 - x2).abs() >= eps || (y1 - y2).abs() >= eps {
            GuideLine::Angled {
                x: (x1 + x2) / 2.0 + cx,
                y: (y1 + y2) / 2.0 + cy,
                angle: (y2 - y1).atan2(x2 - x1).to_degrees(),
            }
        } else {
            return None;
        };
        Some(Guide::from(line))
    }

    fn object_bbox(&self, oc: &Chunk) -> Option<Rect> {
        let c = oc.find(b"bbox")?;
        let d = self.data(c);
        let mut r = self.reader(d);
        let (x0, y0, x1, y1) = (r.coord()?, r.coord()?, r.coord()?, r.coord()?);
        let b = Rect::new(x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1));
        (b.x0.is_finite() && b.y0.is_finite() && b.width() > 0.0 && b.height() > 0.0).then_some(b)
    }

    /// A paragraph frame from the object's box and style, for paragraph
    /// text (type 0x06) whose coordinates hold no frame size: the box
    /// taken back through the object's transform gives the frame's place
    /// and height, the style's columns its width when present. Only for
    /// transforms that neither turn nor slant (a turned box is larger
    /// than the frame). Assumed from the version 21 layout (the style's
    /// `frame` section); not yet seen in a file.
    fn frame_from_hints(&self, h: &FrameHints) -> Option<(Point, Size)> {
        let b = h.bbox?;
        let [a, bb, c, dd, _, _] = h.transform.as_coeffs();
        if bb.abs() > 1e-9 || c.abs() > 1e-9 || a.abs() < 1e-9 || dd.abs() < 1e-9 {
            return None;
        }
        let inv = h.transform.inverse();
        let p0 = inv * Point::new(b.x0, b.y0);
        let p1 = inv * Point::new(b.x1, b.y1);
        let local = Rect::from_points(p0, p1);
        let width = h
            .style
            .and_then(text::frame_width_units)
            .map(|w| w * UNIT_MM)
            .filter(|w| *w > 0.0)
            .unwrap_or(local.width());
        let size = Size::new(width, local.height());
        let ok = local.x0.is_finite()
            && local.y0.is_finite()
            && size.width.is_finite()
            && size.height.is_finite()
            && size.width > 1e-3
            && size.height > 1e-3;
        ok.then(|| (Point::new(local.x0, local.y0), size))
    }

    /// The transformations of a version 3 object (argument 100): a u16, the
    /// count (u16 at 2), then 10 bytes each: the type (u16; 0x8c a matrix)
    /// and where its data starts (u16 at 6, from the argument's start).
    /// Matrices apply in file order; others (perspective, envelope, seen
    /// between two matrices) are not applied yet.
    fn read_transform_list_v3(&mut self, d: &'a [u8], arg: Arg) -> Option<Affine> {
        let t = self.read_transforms_v3(d, arg)?;
        if t.envelope.is_some() {
            self.warn_v3_envelope();
        }
        Some(t.after * t.before)
    }

    fn warn_v3_envelope(&mut self) {
        if !self.v3_effect_warned {
            self.v3_effect_warned = true;
            self.report
                .warn("an envelope of a version 3 text or group is not applied");
        }
    }

    /// The transformation list of a version 3 object: the matrices before
    /// an envelope (type 0x14), the envelope, and the matrices after it.
    fn read_transforms_v3(&mut self, d: &'a [u8], arg: Arg) -> Option<V3Transforms> {
        let g = d.get(arg.off..arg.end)?;
        let u = |k: usize| g.get(k..k + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
        let n = (u(2)? as usize).min(32);
        let mut t = V3Transforms {
            before: Affine::IDENTITY,
            envelope: None,
            after: Affine::IDENTITY,
        };
        let mut any = false;
        for i in 0..n {
            let (Some(kind), Some(at)) = (u(4 + 10 * i), u(4 + 10 * i + 6)) else {
                break;
            };
            match kind {
                0x8c => {
                    let mut r = self.reader_at(g, at as usize);
                    if let Some(m) = fixed_matrix(&mut r) {
                        any = true;
                        if t.envelope.is_some() {
                            t.after = m * t.after;
                        } else {
                            t.before = m * t.before;
                        }
                    }
                }
                0x14 if t.envelope.is_none() => match v3_envelope(g, at as usize) {
                    Some(e) => t.envelope = Some(e),
                    None => self
                        .report
                        .warn("an envelope of a version 3 object could not be read"),
                },
                other => {
                    if !self.v3_effect_warned {
                        self.v3_effect_warned = true;
                        self.report.warn(format!(
                            "a transformation of type 0x{other:02x} of a version 3 object is not applied"
                        ));
                    }
                }
            }
        }
        (any || t.envelope.is_some()).then_some(t)
    }

    /// Version 3 artistic text, in its coordinates argument: the character
    /// count (u16 at 11), the font (u16 at 15), character spacing, size in
    /// 1/1000 inch, word and line spacing (u16 at 17, 19, 21, 23; spacing
    /// in percent), the alignment (25: 1 left, 2 centre, 3 right), tab
    /// stops from 32, and from 132 one 3-byte record a character (a byte,
    /// the character in Windows-1252, a byte). The family comes from the
    /// file's font table; the font number is not matched to it yet.
    fn read_text_v3(&mut self, d: &'a [u8], arg: Arg) -> Option<ShapeKind> {
        let g = d.get(arg.off..arg.end)?;
        let u = |k: usize| g.get(k..k + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
        let n = u(11)? as usize;
        let size_in = u(19).unwrap_or(0) as f64 / 1000.0;
        let pct = |k: usize, lo: f64, hi: f64, def: f64| {
            u(k).map(|v| v as i16 as f64)
                .filter(|v| (lo..=hi).contains(v))
                .unwrap_or(def)
        };
        let raw: Vec<u8> = (0..n.min(32_000))
            .filter_map(|i| g.get(132 + 3 * i + 1).copied())
            .collect();
        let text = crate::text::decode_cp1252(&raw)
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        if text.trim().is_empty() {
            return None;
        }
        let family = self
            .v3_fonts
            .first()
            .map(|f| crate::wl::family_of(f))
            .unwrap_or_else(|| "Arial".into());
        let size_pt = if size_in > 0.0 { size_in * 72.0 } else { 24.0 };
        let para = ParagraphStyle {
            char_spacing_pct: pct(17, -100.0, 2000.0, 0.0),
            word_spacing_pct: pct(21, 0.0, 2000.0, 100.0),
            leading_pct: pct(23, 0.0, 2000.0, 100.0),
            ..Default::default()
        };
        let align = match g.get(25) {
            Some(2) => TextAlign::Center,
            Some(3) => TextAlign::Right,
            _ => TextAlign::Left,
        };
        Some(ShapeKind::Text {
            spans: vec![TextSpan::new(text, family, size_pt)],
            origin: Point::ZERO,
            frame: None,
            align,
            para,
            on_path: None,
        })
    }

    /// Version 3 fill, inline in the object: the type (u8: 0 none,
    /// 1 uniform, 2 linear and 4 radial fountain) and its colour records.
    fn read_inline_fill(&mut self, d: &'a [u8], arg: Arg) -> Fill {
        let mut r = self.reader_at(d, arg.off);
        match r.u8() {
            Some(1) => self
                .read_color(&mut r)
                .map(Fill::Solid)
                .unwrap_or(Fill::None),
            Some(0) | None => Fill::None,
            // Fountain, linear (2) or radial (4): the angle (i16, tenths of
            // a degree), the start colour and the end colour.
            Some(t @ (2 | 4)) => {
                let angle = r.i16().map_or(0.0, |a| a as f64 / 10.0);
                let (Some(from), Some(to)) = (self.read_color(&mut r), self.read_color(&mut r))
                else {
                    return Fill::None;
                };
                Fill::Fountain(Fountain {
                    kind: if t == 4 {
                        FountainKind::Radial
                    } else {
                        FountainKind::Linear
                    },
                    stops: vec![Stop::new(0.0, from), Stop::new(1.0, to)],
                    angle,
                    ..Default::default()
                })
            }
            // Two-colour pattern: the tile (u32, a `bmpf` id), the tile's
            // width and height (u16, 1/1000 inch), 7 bytes, the front and
            // the back colour.
            Some(7) => {
                let (Some(tile_id), Some(w)) = (r.u32(), r.u16()) else {
                    return Fill::None;
                };
                r.skip(2 + 7);
                let (Some(front), Some(back)) = (self.read_color(&mut r), self.read_color(&mut r))
                else {
                    return Fill::None;
                };
                let size_mm = if w > 0 { w as f64 * UNIT16_MM } else { 10.0 };
                self.two_colour_pattern(tile_id, front, back, size_mm)
            }
            // PostScript texture (6) and vector pattern (10) fills: shown
            // with a stand-in pattern.
            Some(t @ (6 | 10)) => self.stand_in_fill(t as u16),
            Some(other) => {
                self.report.warn(format!(
                    "fill type {other} of a version 3 drawing not modelled yet"
                ));
                Fill::None
            }
        }
    }

    /// A two-colour pattern from the `bmpf` tile `tile_id`, a checker when
    /// the file has no such tile.
    fn two_colour_pattern(
        &mut self,
        tile_id: u32,
        front: Color,
        back: Color,
        size_mm: f64,
    ) -> Fill {
        if let Some(tile) = self.pattern_tiles.get(&tile_id) {
            let f = front.to_rgb8();
            let b = back.to_rgb8();
            let mut rgba = Vec::with_capacity(tile.front.len() * 4);
            for &on in &tile.front {
                let c = if on { f } else { b };
                rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
            return Fill::Pattern(Pattern::Bitmap {
                png: encode_png(tile.width, tile.height, &rgba).into(),
                width_px: tile.width,
                height_px: tile.height,
                size_mm,
            });
        }
        self.report
            .warn(format!("pattern {tile_id} drawn with a placeholder tile"));
        Fill::Pattern(Pattern::TwoColor {
            tile: PatternTile::Checker,
            front,
            back,
            size_mm,
        })
    }

    /// The stand-in for fills whose look the file does not hold: PostScript
    /// textures (6, procedures of the program that wrote the file, named
    /// by number) and, in version 3, vector patterns (10). Grey diagonal
    /// stripes show where they are.
    fn stand_in_fill(&mut self, ftype: u16) -> Fill {
        if !self.stand_in_warned {
            self.stand_in_warned = true;
            let what = if ftype == 6 {
                "PostScript texture"
            } else {
                "vector pattern"
            };
            self.report.warn(format!(
                "{what} fills are shown with a stand-in pattern of grey stripes"
            ));
        }
        Fill::Pattern(Pattern::TwoColor {
            tile: PatternTile::DiagonalStripes,
            front: Color::cmyk_pct(0.0, 0.0, 0.0, 40.0),
            back: Color::cmyk_pct(0.0, 0.0, 0.0, 0.0),
            size_mm: 4.0,
        })
    }

    /// Version 3 outline, inline in the object: flags (u8, bit 0 set when
    /// the object has an outline), width (u16, 1/1000 inch), stretch (u16,
    /// percent), nib angle (u16, tenths of a degree), a colour record.
    fn read_inline_outline(&mut self, d: &'a [u8], arg: Arg) -> Option<Stroke> {
        let mut r = self.reader_at(d, arg.off);
        let flags = r.u8()?;
        let width = r.u16()? as f64 * UNIT16_MM;
        let stretch = r.u16()?;
        let angle = r.i16()? as f64 / 10.0;
        let color = self.read_color(&mut r)?;
        if flags & 1 == 0 {
            return None;
        }
        let mut s = if width > 0.0 {
            Stroke::new(color, width)
        } else {
            Stroke::hairline(color)
        };
        s.stretch = (stretch as f64 / 100.0).clamp(0.01, 1.0);
        s.nib_angle = angle;
        // Line caps at 32 and joins at 33 (0 butt or mitre, 1 round,
        // 2 square or bevel), then the start and end arrowhead ids (u32 at
        // 34 and 38, ids of the file's `arrw` chunks).
        let byte = |k: usize| {
            d.get(arg.off + k)
                .copied()
                .filter(|_| arg.off + k < arg.end)
        };
        s.cap = match byte(32) {
            Some(1) => LineCap::Round,
            Some(2) => LineCap::Square,
            _ => LineCap::Butt,
        };
        s.join = match byte(33) {
            Some(1) => LineJoin::Round,
            Some(2) => LineJoin::Bevel,
            _ => LineJoin::Miter,
        };
        if arg.end >= arg.off + 42 {
            let start = self.reader_at(d, arg.off + 34).u32().unwrap_or(0);
            let end = self.reader_at(d, arg.off + 38).u32().unwrap_or(0);
            s.start_arrow = self.marker_preset(start, 0);
            s.end_arrow = self.marker_preset(end, 0);
        }
        Some(s)
    }

    fn read_loda(&mut self, d: &'a [u8]) -> Option<Loda> {
        let mut r = self.reader(d);
        let _len = r.uint()?;
        let num_args = r.uint()? as usize;
        let start_args = r.uint()? as usize;
        let start_types = r.uint()? as usize;
        let mut kind = r.uint()?;
        // Version 3 numbers the object types from 0 (rectangle, ellipse,
        // curve, text, bitmap), one below the later codes.
        if self.v < 4 && kind <= 4 {
            kind += 1;
        }
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

    /// The mesh of a mesh fill object (argument 19150, a version 11 file,
    /// #896): its byte length, 100, the number of column lines `C` and of
    /// row lines `R`; the `C + R` grid lines, each 100, a point count `n`,
    /// the `n` points and their `n` node types as a curve has them, then
    /// for each point the (row, column) of the mesh node it is (-1, -1
    /// for control points and points between nodes), for each point its
    /// place along the line (0 to 1, -1 for control points), the line's
    /// place across the grid (f64) and 1 for a column line, 0 for a row
    /// line; the object's transform (six f64, as in its `trfd`); then
    /// 100, `R`, `C` and the `R x C` node colours, row by row (12-byte
    /// colour records). Nodes take their places from the lines through
    /// them; the cells between are drawn with straight sides (the lines'
    /// curves are not kept). Also the mesh's edge (the first and last row
    /// and column lines), the object's outline.
    fn read_mesh(
        &mut self,
        d: &'a [u8],
        arg: Arg,
    ) -> Option<(tracedraw_core::Mesh, Option<BezPath>)> {
        let data = d.get(..arg.end)?;
        let mut r = self.reader_at(data, arg.off);
        let _len = r.u32()?;
        let _tag = r.u32()?;
        let cols = r.u32()? as usize;
        let rows = r.u32()? as usize;
        if cols < 2 || rows < 2 || cols.saturating_mul(rows) > MESH_MAX_NODES {
            return None;
        }
        let mut pos: Vec<Option<Point>> = vec![None; rows * cols];
        // Each line's curve, and whether it runs from its last node back
        // to its first.
        let mut lines: Vec<(BezPath, bool)> = Vec::with_capacity(rows + cols);
        for _ in 0..rows + cols {
            let _tag = r.u32()?;
            let n = r.u32()? as usize;
            // Each point takes 8 bytes, a type byte, 8 for its node and 8
            // for its place.
            if n == 0 || n > r.remaining() / 25 {
                return None;
            }
            let (pts, types) = self.read_points(&mut r, n)?;
            let mut first: Option<usize> = None;
            let mut last: Option<usize> = None;
            for p in &pts {
                let row = r.i32()?;
                let col = r.i32()?;
                if (0..rows as i32).contains(&row) && (0..cols as i32).contains(&col) {
                    let i = row as usize * cols + col as usize;
                    pos.get_mut(i)?.get_or_insert(*p);
                    first.get_or_insert(i);
                    last = Some(i);
                }
            }
            r.skip(8 * n + 8 + 4);
            lines.push((build_path(&pts, &types).0, first > last));
        }
        // The colour grid closes the record.
        let grid = arg.end.checked_sub(12 + rows * cols * 12)?;
        if grid < r.pos {
            return None;
        }
        let mut g = self.reader_at(data, grid);
        let _tag = g.u32()?;
        if g.u32()? as usize != rows || g.u32()? as usize != cols {
            return None;
        }
        let mut nodes = Vec::with_capacity(rows * cols);
        for p in pos {
            let color = self.read_color(&mut g)?;
            nodes.push(tracedraw_core::MeshNode {
                pos: p?,
                color,
                alpha: 1.0,
            });
        }
        // The edge: the first row, the last column, the last row and the
        // first column, round in that order (rows follow the columns).
        let edge = [
            (lines.get(cols), false),
            (lines.get(cols - 1), false),
            (lines.get(cols + rows - 1), true),
            (lines.first(), true),
        ];
        let mut path = BezPath::new();
        for (line, back) in edge {
            let (curve, reversed) = line?;
            let mut segs: Vec<tracedraw_core::geometry::PathSeg> = curve.segments().collect();
            if back != *reversed {
                segs.reverse();
                for s in segs.iter_mut() {
                    *s = s.reverse();
                }
            }
            for s in segs {
                if path.elements().is_empty() {
                    path.move_to(tracedraw_core::geometry::ParamCurve::start(&s));
                }
                path.push(s.as_path_el());
            }
        }
        path.close_path();
        let edge = (path.elements().len() > 2).then_some(path);
        Some((
            tracedraw_core::Mesh {
                rows: rows as u32 - 1,
                cols: cols as u32 - 1,
                nodes,
                smooth: false,
            },
            edge,
        ))
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
            rx,
            ry,
            cx,
            cy,
        })
    }

    /// The step count of a blend's linked group (`LIST lnkg` whose own
    /// `lgob` holds a `loda` of object type 7): the first u32 of its
    /// argument of type 3000. `None` for other linked groups (a contour's
    /// is of type 15). Read from two files saved by versions 9 and 13 with
    /// the default 20 steps (see CLEANROOM.md).
    ///
    /// A value outside 1..=999 is no step count the program can save: the
    /// field holds something else in this file (another version's layout),
    /// so the blend gets the default 20 steps with a warning naming the
    /// value. It used to be kept to 999, which drew such blends with 999
    /// steps each and could take all the memory (#980).
    fn blend_steps(&mut self, group: &Chunk) -> Option<u32> {
        let lgob = group.children.iter().find(|k| k.is(b"lgob"))?;
        let loda = lgob.children.iter().find(|k| k.is(b"loda"))?;
        let d = self.data(loda);
        let l = self.read_loda(d)?;
        if l.kind != 7 {
            return None;
        }
        let arg = l.args.iter().find(|a| a.ty == 3000)?;
        let raw = self.reader_at(d, arg.off).uint();
        Some(match raw {
            Some(n @ 1..=tracedraw_core::live::MAX_BLEND_STEPS) => n,
            other => {
                let value = other.map_or_else(|| "nothing".to_owned(), |n| n.to_string());
                log::warn!(
                    "blend step count of a version {} file reads {value}; {BLEND_DEFAULT_STEPS} steps used",
                    self.v
                );
                self.report.warn(format!(
                    "a blend's step count reads {value}, not 1 to {}; drawn with {BLEND_DEFAULT_STEPS} steps",
                    tracedraw_core::live::MAX_BLEND_STEPS
                ));
                BLEND_DEFAULT_STEPS
            }
        })
    }

    /// `steps` for the next blend of the file, within what the file's
    /// blends may still take together ([`FILE_BLEND_STEPS`]); past that
    /// each blend gets one step, with a warning, so a file of thousands
    /// of blends opens without filling the memory (#980).
    fn budget_blend_steps(&mut self, steps: u32) -> u32 {
        let granted = steps.min(self.blend_steps_left).max(1);
        if granted < steps {
            log::warn!("blend steps past {FILE_BLEND_STEPS} in one file; a blend drawn with {granted} of its {steps}");
            self.report.warn(format!(
                "the file's blends ask for more than {FILE_BLEND_STEPS} steps together; the later ones are drawn with fewer"
            ));
        }
        self.blend_steps_left = self.blend_steps_left.saturating_sub(granted);
        granted
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
        self.pending_clip = None;
        self.pending_on_path = None;
        self.pending_text_transform = None;
        let mut v3_envelope_of: Option<(Affine, Rect, tracedraw_core::envelope::Frame, Affine)> =
            None;
        let mut transform = self.outer_transform
            * trfd
                .map(|t| self.read_transform(t))
                .unwrap_or(Affine::IDENTITY);

        let d = self.data(loda);
        let l = self.read_loda(d)?;
        let kind_code = l.kind;
        if kind_code == OBJ_GUIDE {
            self.last_was_guide = true;
            let line = l.args.iter().find(|a| a.ty == ARG_GUIDE_LINE);
            match line.and_then(|a| self.read_guide(d, *a, page_size)) {
                Some(g) => self.guides.push(g),
                None => self.report.warn("a guideline could not be read"),
            }
            return None;
        }
        log::debug!(
            "object type 0x{kind_code:02x}, args (type, bytes): {:?}",
            l.args
                .iter()
                .map(|a| (a.ty, a.end.saturating_sub(a.off)))
                .collect::<Vec<_>>()
        );
        if log::log_enabled!(log::Level::Trace) {
            for a in &l.args {
                let bytes = d.get(a.off..a.end.min(a.off + 192)).unwrap_or(&[]);
                let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
                log::trace!("  arg {}: {}", a.ty, hex.join(" "));
            }
        }

        let mut fill = Fill::None;
        let mut stroke: Option<Stroke> = None;
        let mut name: Option<String> = None;
        let mut hidden = false;
        let mut coords: Option<Arg> = None;
        let mut polygon: Option<PolygonInfo> = None;
        let mut style_id: Option<u32> = None;
        let mut style_string: Option<String> = None;
        let mut opacity = 1.0;
        let mut lens: Option<tracedraw_core::live::Lens> = None;
        let mut mesh: Option<Arg> = None;

        for arg in &l.args {
            match arg.ty {
                ARG_STYLE => style_id = self.reader_at(d, arg.off).uint(),
                ARG_STYLE_STRING => style_string = self.read_object_style_string(d, *arg),
                ARG_OPACITY => {
                    if let Some(l) = self.read_lens(d, *arg) {
                        lens = Some(l);
                    } else if let Some(o) = self.read_opacity(d, *arg) {
                        opacity = o;
                    }
                }
                // Before version 4 the fill, the outline and the matrix are
                // stored inline.
                ARG_FILL if self.v < 4 => fill = self.read_inline_fill(d, *arg),
                ARG_OUTLINE if self.v < 4 => stroke = self.read_inline_outline(d, *arg),
                ARG_LEGACY_TRANSFORM if self.v < 4 => {
                    if let Some(t) = self.read_transforms_v3(d, *arg) {
                        transform = self.outer_transform * t.after * t.before;
                        v3_envelope_of = t.envelope.map(|(r, f)| (t.before, r, f, t.after));
                    }
                }
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
                        let s = self.read_name(s);
                        if !s.is_empty() {
                            name = Some(s);
                        }
                    }
                }
                ARG_POLYGON => polygon = self.read_polygon_info(d, *arg),
                ARG_CLIP_CONTENTS => self.pending_clip = self.reader_at(d, arg.off).u32(),
                ARG_HIDDEN => hidden = self.reader_at(d, arg.off).u32().is_some_and(|h| h != 0),
                ARG_MESH if kind_code == OBJ_MESH => mesh = Some(*arg),
                _ => {}
            }
        }

        // A shape without an outline argument takes its style's outline
        // (versions 7 to 15, whose objects name a `stlt` style): seen in
        // version 14 and 15 files whose previews and exports draw such
        // shapes with the default style's hairline.
        let shape_kind = matches!(
            kind_code,
            OBJ_RECT | OBJ_ELLIPSE | OBJ_CURVE | OBJ_POLYGON | OBJ_PATH
        );
        if shape_kind && self.v >= 7 && !l.args.iter().any(|a| a.ty == ARG_OUTLINE) {
            if let Some(o) = style_id
                .and_then(|sid| self.style_defaults(sid).outl_id)
                .and_then(|o| self.outlines.get(&o))
            {
                stroke = o.stroke.clone();
            }
        }
        let mut kind = match kind_code {
            OBJ_ARTISTIC_TEXT if self.v < 4 => coords.and_then(|a| self.read_text_v3(d, a)),
            OBJ_ARTISTIC_TEXT | OBJ_PARAGRAPH_TEXT => {
                let frame = FrameHints {
                    style: style_string.as_deref(),
                    bbox: self.object_bbox(oc),
                    transform,
                };
                self.build_text(kind_code, d, coords, oc, style_id, &fill, frame)
            }
            OBJ_CONNECTOR => self.read_connector(oc),
            OBJ_EMBEDDED => self.read_embedded(d, coords, name.as_deref()),
            // A mesh fill object's outline is a curve.
            OBJ_MESH => coords.and_then(|a| self.read_geometry(OBJ_CURVE, d, a, polygon)),
            _ => coords.and_then(|a| self.read_geometry(kind_code, d, a, polygon)),
        };
        // Its fill is the mesh and its outline the mesh's edge (the
        // coordinates can keep the outline the object had before its
        // nodes were moved); one that cannot be read keeps the curve and
        // the fill record's colour.
        if kind_code == OBJ_MESH && kind.is_some() {
            match mesh.and_then(|a| self.read_mesh(d, a)) {
                Some((m, edge)) => {
                    fill = Fill::Mesh(m);
                    if let Some(path) = edge {
                        self.pending_node_types.clear();
                        kind = Some(ShapeKind::Path { path, closed: true });
                    }
                }
                None => self
                    .report
                    .warn("a mesh fill could not be read; the object keeps a plain fill"),
            }
        }
        let mut kind = match kind {
            Some(k) => k,
            None => {
                // (A bitmap whose picture is missing has said so already.)
                match kind_code {
                    OBJ_ARTISTIC_TEXT | OBJ_PARAGRAPH_TEXT | OBJ_EMBEDDED | OBJ_BITMAP => {}
                    other => self
                        .report
                        .warn(format!("object type 0x{other:02x} not modelled yet")),
                }
                return None;
            }
        };
        // A version 3 envelope bends the shape itself: its outline goes
        // through the matrices before the envelope, is mapped from the
        // envelope's box onto its frame, and keeps the matrices after.
        if let Some((before, rect, frame, after)) = v3_envelope_of {
            let closed = match &kind {
                ShapeKind::Path { closed, .. } => Some(*closed),
                ShapeKind::Rect { .. } | ShapeKind::Ellipse { .. } => Some(true),
                _ => None,
            };
            match closed {
                Some(closed) => {
                    let local = Shape::new(id, kind.clone()).local_path();
                    let path = tracedraw_core::envelope::apply_in(
                        &(before * local),
                        rect,
                        &frame,
                        false,
                        tracedraw_core::envelope::Mapping::Putty,
                    );
                    kind = ShapeKind::Path { path, closed };
                    transform = self.outer_transform * after;
                }
                None => self.warn_v3_envelope(),
            }
        }

        // Version 16+: the outline's mitre limit is in the object's style
        // string (the outline record leaves it out, #384).
        if let (Some(s), Some(a)) = (
            stroke.as_mut(),
            style_string.as_deref().and_then(text::object_miter_limit),
        ) {
            s.miter_limit = a;
        }

        // Version 16+: a graded transparency in the object's style string
        // becomes a mask; otherwise its uniform transparency an opacity.
        let mask = style_string
            .as_deref()
            .and_then(text::object_transparency_mask);
        if opacity >= 1.0 && mask.is_none() {
            if let Some(o) = style_string.as_deref().and_then(text::object_opacity) {
                opacity = o;
            }
        }

        // The page origin is the page centre; ours is the bottom-left.
        let recenter = Affine::translate((page_size.width / 2.0, page_size.height / 2.0));
        let transform = self.pending_text_transform.take().unwrap_or(transform);
        let mut shape = Shape::new(id, kind);
        if matches!(shape.kind, ShapeKind::Path { .. }) {
            shape.node_types = std::mem::take(&mut self.pending_node_types);
        }
        shape.transform = recenter * transform;
        // A hidden object (a version 21 template: the white page-sized
        // rectangle its preview leaves out, over the page's dark
        // background) is kept but not drawn.
        shape.visible = !hidden;
        shape.fill = fill;
        shape.stroke = stroke;
        // Rounded corners keep their size when the rectangle is resized,
        // as new rectangles do, rather than stretch with it (#619); after
        // the outline, which decides whether the stretch may move.
        shape.keep_corner_size();
        shape.name = name;
        shape.opacity = opacity;
        if let Some((mask, target)) = mask {
            shape
                .effects
                .push(tracedraw_core::live::Effect::Transparency {
                    mask,
                    merge: tracedraw_core::live::MergeMode::Normal,
                    target,
                });
        }
        if let Some(l) = lens {
            shape.effects.push(tracedraw_core::live::Effect::Lens(l));
        }
        if let Some(mut fitted) = self.pending_on_path.take() {
            fitted.page_box = self
                .object_bbox(oc)
                .map(|b| recenter.transform_rect_bbox(b));
            log::debug!("text fitted to a path: {fitted:?}");
            self.on_path.insert(id, fitted);
        }
        Some(shape)
    }

    /// Index the groups of `LIST clpt` by the number in their `spnd`.
    fn index_clip_groups(&mut self) {
        let tree = self.tree;
        let mut found = Vec::new();
        tree.root.walk(&mut |c, _| {
            if c.is(b"clpt") {
                found.extend(c.children.iter().filter(|g| g.is(b"grp ")));
            }
        });
        for g in found {
            let Some(sp) = g.children.iter().find(|c| c.is(b"spnd")) else {
                continue;
            };
            if let Some(n) = self.reader(self.data(sp)).u32() {
                self.clip_groups.insert(n, g);
            }
        }
    }

    /// A clip container: `shape` with the objects of the `clpt` group
    /// numbered `clip` drawn inside it (the container's fill under them,
    /// its outline over them). A number naming no group leaves the shape.
    fn with_clip_contents(
        &mut self,
        shape: Shape,
        clip: u32,
        ids: &mut tracedraw_core::id::IdSource,
        page_size: Size,
        depth: usize,
    ) -> Shape {
        // Taken out while read, so a group naming itself ends there.
        let Some(group) = self.clip_groups.remove(&clip) else {
            self.report.warn(format!(
                "clip container names contents {clip:#x} the file lacks"
            ));
            return shape;
        };
        let contents = self.read_objects(group, ids, page_size, depth + 1);
        self.clip_groups.insert(clip, group);
        if contents.is_empty() {
            return shape;
        }
        let mut outer = Shape::new(
            ids.shape(),
            ShapeKind::Group {
                children: Vec::new(),
            },
        );
        outer.name = shape.name.clone();
        outer.fill = Fill::None;
        outer.stroke = None;
        outer.kind = ShapeKind::ClipFrame {
            frame: Box::new(shape),
            contents,
        };
        outer
    }

    /// The objects of a layer or group list, bottom to top. Files store
    /// them front to back (confirmed with a version 21 file: text, then a patch,
    /// then the full-page picture underneath). `grp ` lists become groups.
    /// Texts fitted to a path are placed on their path, one of the other
    /// objects of the list.
    fn read_objects(
        &mut self,
        list: &Chunk,
        ids: &mut tracedraw_core::id::IdSource,
        page_size: Size,
        depth: usize,
    ) -> Vec<Shape> {
        let mut out: Vec<Shape> = Vec::new();
        self.collect_objects(list, ids, page_size, depth, &mut out);
        out.reverse();
        self.fit_texts_to_paths(&mut out, list.is(b"grp "));
        out
    }

    /// The objects of a list front to back, nested lists other than
    /// groups taken in place.
    fn collect_objects(
        &mut self,
        list: &Chunk,
        ids: &mut tracedraw_core::id::IdSource,
        page_size: Size,
        depth: usize,
        out: &mut Vec<Shape>,
    ) {
        if depth > 64 {
            self.report
                .warn("group nesting deeper than 64; inner objects skipped");
            return;
        }
        // Blends found in this list: where their linked group stood among
        // the objects read, and their step count (see `blend_steps`).
        let first = out.len();
        let mut blends: Vec<(usize, u32)> = Vec::new();
        for c in &list.children {
            if c.is(b"obj ") {
                let id = ids.shape();
                match self.read_object(c, id, page_size) {
                    Some(mut shape) => {
                        if let Some(crop) = self.pending_crop.take() {
                            shape = crop_to_clip_frame(shape, crop, ids);
                        }
                        if let Some(clip) = self.pending_clip.take() {
                            shape = self.with_clip_contents(shape, clip, ids, page_size, depth);
                        }
                        out.push(shape);
                    }
                    None if std::mem::take(&mut self.last_was_guide) => {}
                    None => self.report.skipped_objects += 1,
                }
            } else if c.is(b"grp ") {
                // Version 3: the group's own `lobj` holds a matrix that
                // places all its objects; later versions store whole
                // transforms.
                let own = if self.v < 4 {
                    c.children.iter().find(|k| k.is(b"lobj")).and_then(|k| {
                        let d = self.data(k);
                        let l = self.read_loda(d)?;
                        let arg = *l.args.iter().find(|a| a.ty == ARG_LEGACY_TRANSFORM)?;
                        self.read_transform_list_v3(d, arg)
                    })
                } else {
                    None
                };
                let saved = self.outer_transform;
                if let Some(m) = own {
                    self.outer_transform = saved * m;
                }
                let children = self.read_objects(c, ids, page_size, depth + 1);
                self.outer_transform = saved;
                match children.len() {
                    0 => {}
                    1 => out.extend(children),
                    _ => {
                        let mut g = Shape::new(ids.shape(), ShapeKind::Group { children });
                        g.fill = Fill::None;
                        g.stroke = None;
                        out.push(g);
                    }
                }
            } else if c.is(b"lnkg") {
                // A linked group (the strokes an artistic brush makes from
                // a curve, in a version 10 file): its own `lgob` holds the
                // transform its objects are drawn through (theirs is the
                // identity there).
                let own = c
                    .children
                    .iter()
                    .find(|k| k.is(b"lgob"))
                    .and_then(|lgob| {
                        let mut t = None;
                        lgob.walk(&mut |k, _| {
                            if k.is(b"trfd") && t.is_none() {
                                t = Some(k);
                            }
                        });
                        t
                    })
                    .map(|t| self.read_transform(t))
                    .unwrap_or(Affine::IDENTITY);
                let saved = self.outer_transform;
                self.outer_transform = saved * own;
                let before = out.len();
                self.collect_objects(c, ids, page_size, depth + 1, out);
                self.outer_transform = saved;
                // A blend keeps no steps of its own: an empty linked group
                // between its two end objects, its `loda` of object type 7
                // holding the step count (#597).
                if out.len() == before {
                    if let Some(steps) = self.blend_steps(c) {
                        let steps = self.budget_blend_steps(steps);
                        blends.push((before, steps));
                    }
                }
            } else if c.list_type.is_some() {
                // Other lists (`lgob`, `spnd`, ...) are not objects, but
                // older files nest the object lists one level down.
                let before = out.len();
                self.collect_objects(c, ids, page_size, depth + 1, out);
                if out.len() > before + 1 {
                    log::debug!(
                        "{} objects in a nested {:?} list",
                        out.len() - before,
                        c.list_type
                            .map(|t| String::from_utf8_lossy(&t).into_owned())
                            .unwrap_or_default()
                    );
                }
            }
        }
        for (at, steps) in blends {
            // The objects on either side of the group are its ends.
            if at <= first || at >= out.len() {
                self.report
                    .warn("a blend without both end objects; left out");
                continue;
            }
            let end = out[at].clone();
            let end_id = end.id;
            // The steps pair the two outlines from their first nodes, as
            // the file's program draws them (twists and folds included),
            // and lie between the two: the start, read first, is the
            // upper one (#597).
            let map_nodes = match (blend_nodes(&out[at - 1]), blend_nodes(&end)) {
                (Some(start), Some(end)) => Some(tracedraw_core::live::BlendNodes { start, end }),
                _ => None,
            };
            out[at - 1]
                .effects
                .push(tracedraw_core::live::Effect::Blend {
                    end: Box::new(end),
                    steps,
                    accel_objects: 0.0,
                    accel_colors: 0.0,
                    rotation: 0.0,
                    path: None,
                    rotate_on_path: false,
                    end_id: Some(end_id),
                    options: tracedraw_core::live::BlendOptions {
                        steps_below: true,
                        map_nodes,
                        ..Default::default()
                    },
                });
        }
    }

    /// Place each text of `shapes` that the file fits to a path on its
    /// path: in a group of one text and one curve that curve, otherwise
    /// the curve along which the fitted text's box lies (see
    /// [`path_for_text`]). Texts whose path is not found stay plain text.
    fn fit_texts_to_paths(&mut self, shapes: &mut [Shape], group: bool) {
        let texts: Vec<(usize, OnPathText)> = shapes
            .iter()
            .enumerate()
            .filter_map(|(i, s)| self.on_path.remove(&s.id).map(|f| (i, f)))
            .filter(|(i, _)| matches!(shapes[*i].kind, ShapeKind::Text { .. }))
            .collect();
        if texts.is_empty() {
            return;
        }
        let curves: Vec<usize> = shapes
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                matches!(
                    s.kind,
                    ShapeKind::Path { .. }
                        | ShapeKind::Ellipse { .. }
                        | ShapeKind::Rect { .. }
                        | ShapeKind::Polygon { .. }
                )
            })
            .map(|(i, _)| i)
            .collect();
        for (ti, fitted) in texts {
            let found = match curves.as_slice() {
                [only] if group => Some((*only, None)),
                _ => path_for_text(shapes, &curves, &fitted)
                    .map(|ci| (ci, None))
                    .or_else(|| {
                        let cap = shapes.get(ti).map(cap_height_mm).unwrap_or(0.0);
                        anchored_path_for_text(shapes, &curves, &fitted, cap)
                    }),
            };
            let Some((curve, anchor)) =
                found.and_then(|(ci, a)| shapes.get(ci).cloned().map(|c| (c, a)))
            else {
                self.report
                    .warn("text fitted to a path whose path was not found; drawn straight");
                continue;
            };
            let fits = shapes
                .get_mut(ti)
                .map(|t| match anchor {
                    Some(a) => fit_text_at(t, &curve, a),
                    None => fit_text(t, &curve, &fitted),
                })
                .unwrap_or(false);
            if !fits {
                self.report
                    .warn("text fitted to a path that could not be placed on it; drawn straight");
            }
        }
    }

    /// Opacity argument (8000): 10 unknown bytes (14 from version 13), then a u16
    /// in thousandths. The spec does not say whether the value is opacity
    /// or transparency; files observed so far store a transparency
    /// (0 = opaque), so we read it that way. Values above 1.0 are taken as
    /// percent in thousandths.
    /// A lens in the transparency argument of a version 13+ object: a
    /// record tagged 1300 whose body starts with 1, then a u32 id and the
    /// lens type (u8), then two colour slots. Only type 3 is known: an
    /// invert lens (a version 15 file whose preview shows each colour
    /// beneath the object as its complement). Other types are reported and
    /// read as before, as a transparency.
    fn read_lens(&mut self, d: &'a [u8], arg: Arg) -> Option<tracedraw_core::live::Lens> {
        if self.v < 13 {
            return None;
        }
        let data = d.get(..arg.end)?;
        let mut r = self.reader_at(data, arg.off);
        if r.u32()? != 0x514 {
            return None;
        }
        let len = r.u32()? as usize;
        let body = data.get(r.pos..r.pos.checked_add(len)?)?;
        if body.first() != Some(&1) {
            return None;
        }
        match body.get(5).copied()? {
            3 => Some(tracedraw_core::live::Lens::Invert),
            other => {
                self.report.warn(format!(
                    "lens type {other} not read; drawn without its lens"
                ));
                None
            }
        }
    }

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
        self.pending_node_types.clear();
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
                    // version 15+: width and height as doubles in coordinate
                    // units, the scale the transform applies (it is in
                    // `trfd` too, so it is not applied again), a flag, and
                    // four radii each followed by corner data we skip. A
                    // radius is a share of half the shorter side (1.0 =
                    // half of it): a version 22 file's preview shows a
                    // 0.0929 corner on a 369.8 x 499.0 mm frame drawn at
                    // 17.2 mm, and a version 17 file's 2.7604 on a 36.8 mm
                    // side is exactly 2 inches.
                    let w = r.f64_coord()?;
                    let h = r.f64_coord()?;
                    let _sx = r.f64();
                    let _sy = r.f64();
                    let _scale_with = r.u8();
                    r.skip(7);
                    let half = w.abs().min(h.abs()) / 2.0;
                    // Each radius is followed by 16 bytes of corner data
                    // (corner type and unknown fields) except the last.
                    let mut radii = Vec::new();
                    for gap in [16usize, 16, 16, 0] {
                        let Some(raw) = r.f64() else { break };
                        if raw.is_finite() {
                            radii.push(raw * half);
                        }
                        r.skip(gap);
                    }
                    (w, h, radii)
                };
                let rect = Rect::new(
                    0.0_f64.min(w),
                    0.0_f64.min(h),
                    0.0_f64.max(w),
                    0.0_f64.max(h),
                );
                Some(ShapeKind::rect_with_corners(
                    rect,
                    file_corners(&radii, w, h),
                ))
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
                // The angles turn from the ellipse's own axes: with a
                // negative height (drawn downwards, as files store most
                // ellipses) its y axis points down and an arc from 0 to
                // 180 degrees is the lower half (a version 20 file: the
                // smiles of its faces, as its preview shows them); a
                // negative width mirrors it left to right.
                let (mut a1, mut a2) = (a1, a2);
                if h < 0.0 {
                    (a1, a2) = (-a2, -a1);
                }
                if w < 0.0 {
                    (a1, a2) = (180.0 - a2, 180.0 - a1);
                }
                let norm = |a: f64| a.rem_euclid(360.0);
                let (a1, a2) = (norm(a1), norm(a2));
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
                // Before version 6 the count is a u16 followed by a u16 that
                // is 1 in some curves of versions 3 and 4 (glyph outlines),
                // not part of the count.
                let n = if self.v16() {
                    let n = r.u16()? as usize;
                    r.skip(2);
                    n
                } else {
                    r.u32()? as usize
                };
                let (pts, types) = self.read_points(&mut r, n)?;
                let (path, closed) = build_path(&pts, &types);
                if path.elements().is_empty() {
                    return None;
                }
                self.pending_node_types = file_node_types(&path, &pts, &types);
                Some(ShapeKind::Path { path, closed })
            }
            // Path (version 16+): 4 unknown, two u16 counts, 16 unknown, points.
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
                self.pending_node_types = file_node_types(&path, &pts, &types);
                Some(ShapeKind::Path { path, closed })
            }
            // Polygon: the coordinates hold the base curve; the polygon
            // argument gives the number of angles, radii and centre.
            // Polygon: the coordinates hold one segment of a base shape
            // centred on the origin (its first point the top vertex, then
            // the middle of the side or the inner point of a star, then
            // the next vertex clockwise); argument 11000 scales it and
            // places its centre (seen in a version 19 file: a base radius
            // of 1000 mm scaled by 0.0445 to a 44.5 mm pentagon).
            OBJ_POLYGON => {
                let n = r.u32()? as usize;
                let (pts, _types) = self.read_points(&mut r, n)?;
                let radius = pts.first().map(|p| p.to_vec2().hypot()).unwrap_or(0.0);
                let inner = pts
                    .get(1)
                    .map(|p| p.to_vec2().hypot() / radius)
                    .filter(|k| k.is_finite());
                let (rect, points) = match polygon {
                    Some(p)
                        if radius > 1e-9
                            && (p.rx * radius).abs() > 1e-6
                            && (p.ry * radius).abs() > 1e-6 =>
                    {
                        let (hx, hy) = ((p.rx * radius).abs(), (p.ry * radius).abs());
                        (
                            Rect::new(p.cx - hx, p.cy - hy, p.cx + hx, p.cy + hy),
                            p.num_angles.clamp(3, 500),
                        )
                    }
                    Some(p) => (bounds_of(&pts)?, p.num_angles.clamp(3, 500)),
                    None => (bounds_of(&pts)?, 5),
                };
                // An inner point closer than the middle of a side: a star.
                let side_middle = (std::f64::consts::PI / points as f64).cos();
                let sharpness = match inner {
                    Some(k) if k < side_middle - 1e-3 => (1.0 - k).clamp(0.0, 1.0),
                    _ => 0.0,
                };
                Some(ShapeKind::Polygon {
                    rect,
                    points,
                    sharpness,
                    complex: None,
                    twist: 0.0,
                })
            }
            // Bitmap: two corners, 32 unknown bytes, image id, a version
            // dependent gap, then the crop outline as a point list. When the
            // outline is not the full rectangle the object is cropped
            // (confirmed with a version 21 file: a patch cut from a larger image).
            OBJ_BITMAP if v < 6 => {
                // Before version 6 (seen in version 5): the corner
                // opposite the origin, then the image id (u16); the
                // picture spans from (0, 0) to that corner and the
                // object's transform places it.
                let x1 = r.coord()?;
                let y1 = r.coord()?;
                let image_id = r.u16()? as u32;
                let rect = Rect::new(x1.min(0.0), y1.min(0.0), x1.max(0.0), y1.max(0.0));
                self.pending_crop = None;
                let Some(bm) = self.bitmaps.get(&image_id) else {
                    self.report
                        .warn(format!("bitmap object refers to unknown image {image_id}"));
                    return None;
                };
                Some(ShapeKind::Bitmap {
                    rect,
                    width_px: bm.width_px,
                    height_px: bm.height_px,
                    png: bm.png.clone(),
                    fx: None,
                })
            }
            OBJ_BITMAP => {
                let x1 = r.coord()?;
                let y1 = r.coord()?;
                let x2 = r.coord()?;
                let y2 = r.coord()?;
                r.skip(32);
                let image_id = r.u32()?;
                let v = self.v;
                r.skip(if v < 4 {
                    8
                } else if v == 8 {
                    12
                } else {
                    20
                });
                let rect = Rect::new(x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2));
                self.pending_crop = None;
                if let Some(n) = r.u32() {
                    let n = n as usize;
                    if (3..=100_000).contains(&n) {
                        if let Some((pts, types)) = self.read_points(&mut r, n) {
                            let (path, _closed) = build_path(&pts, &types);
                            let b = bounds_of(&pts);
                            let full = b
                                .map(|b| {
                                    (b.x0 - rect.x0).abs() < 0.05
                                        && (b.y0 - rect.y0).abs() < 0.05
                                        && (b.x1 - rect.x1).abs() < 0.05
                                        && (b.y1 - rect.y1).abs() < 0.05
                                        && n <= 5
                                })
                                .unwrap_or(true);
                            if !full && !path.elements().is_empty() {
                                self.pending_crop = Some(path);
                            }
                        }
                    }
                }
                let Some(bm) = self.bitmaps.get(&image_id) else {
                    self.report
                        .warn(format!("bitmap object refers to unknown image {image_id}"));
                    self.pending_crop = None;
                    return None;
                };
                Some(ShapeKind::Bitmap {
                    rect,
                    width_px: bm.width_px,
                    height_px: bm.height_px,
                    png: bm.png.clone(),
                    fx: None,
                })
            }
            _ => None,
        }
    }

    /// `bmp `: embedded bitmap. Header fields are at fixed positions after
    /// a version dependent gap; pixels are a device-independent bitmap body
    /// (rows padded to 4 bytes, bottom-up). We re-encode as PNG.
    /// `bmp ` chunks: embedded bitmaps, decoded side by side where the
    /// platform has threads (opening a file with many large pictures
    /// spends most of its time here).
    fn read_bitmaps(&mut self, chunks: &[&Chunk]) {
        let v = self.v;
        let datas: Vec<&'a [u8]> = chunks.iter().map(|c| self.data(c)).collect();
        let cmyk = datas
            .iter()
            .any(|d| bitmap_header(d, v).is_some_and(|h| h.model == BITMAP_CMYK && h.bpp == 32));
        if cmyk && self.cmyk_lut.is_none() {
            self.cmyk_lut = self.cmyk_profile.as_ref().and_then(|p| {
                let t = tracedraw_core::icc::Transform::new(
                    p,
                    &tracedraw_core::icc::Profile::srgb(),
                    tracedraw_core::icc::Intent::default(),
                    false,
                )?;
                tracedraw_core::icc::CmykLut::new(&t)
            });
        }
        let lut = self.cmyk_lut.as_ref();
        for decoded in parallel_map(&datas, |d| decode_bitmap(d, v, lut)) {
            match decoded {
                Some(Ok((id, def))) => {
                    self.bitmaps.insert(id, def);
                }
                Some(Err(note)) => self.report.warn(note),
                None => {}
            }
        }
    }

    #[cfg(test)]
    fn read_bitmap(&mut self, c: &Chunk) {
        self.read_bitmaps(&[c]);
    }
}

/// The header of a `bmp ` chunk.
struct BitmapHeader {
    id: u32,
    model: u32,
    width: u32,
    height: u32,
    bpp: u32,
    size: u32,
    /// Where the palette (if any) and the pixels start.
    body: usize,
}

/// Image id (u32; u16 before 6), a version dependent gap, colour model,
/// 4, width, height, 4, bits per pixel, 4, size, 32.
fn bitmap_header(d: &[u8], v: u16) -> Option<BitmapHeader> {
    let mut r = Reader::new(d, v < 6);
    let id = r.uint()?;
    r.skip(if v < 6 {
        14
    } else if v < 7 {
        46
    } else {
        50
    });
    let model = r.u32()?;
    r.skip(4);
    let width = r.u32()?;
    let height = r.u32()?;
    r.skip(4);
    let bpp = r.u32()?;
    r.skip(4);
    let size = r.u32()?;
    r.skip(32);
    Some(BitmapHeader {
        id,
        model,
        width,
        height,
        bpp,
        size,
        body: r.pos,
    })
}

/// A `bmp ` chunk's picture as (image id, picture), a note on why it is
/// left out, or `None` for a chunk too short to tell. Rows are padded to 4
/// bytes and stored bottom-up; 32 and 24 bits are BGR(x), colour model 3
/// with 32 bits is CMYK (bytes C, M, Y, K with 0 for no ink, converted
/// through `lut`), 8 and 1 bits index the palette.
fn decode_bitmap(
    d: &[u8],
    v: u16,
    lut: Option<&tracedraw_core::icc::CmykLut>,
) -> Option<Result<(u32, BitmapDef), String>> {
    let h = bitmap_header(d, v)?;
    let (image_id, width, height, bpp) = (h.id, h.width, h.height, h.bpp);
    if width == 0 || height == 0 || width > 32_768 || height > 32_768 {
        return Some(Err(format!(
            "bitmap {image_id}: size {width}x{height} rejected"
        )));
    }
    let mut r = Reader::new(d, v < 6);
    r.pos = h.body;
    let mut palette: Vec<[u8; 3]> = Vec::new();
    if bpp < 24 && h.model != 5 && h.model != 6 {
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
    let stride = (width as usize * bpp as usize).div_ceil(32) * 4;
    let needed = stride.saturating_mul(height as usize);
    let pixels = d.get(r.pos..r.pos.saturating_add(h.size as usize))?;
    if pixels.len() < needed {
        return Some(Err(format!(
            "bitmap {image_id}: {} pixel bytes, {needed} expected; skipped",
            pixels.len()
        )));
    }
    if !matches!(bpp, 32 | 24 | 8 | 1) {
        return Some(Err(format!(
            "bitmap {image_id}: {bpp} bits per pixel not supported"
        )));
    }
    let cmyk = h.model == BITMAP_CMYK && bpp == 32;
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for row in 0..height as usize {
        // Bottom-up rows.
        let src = &pixels[(height as usize - 1 - row) * stride..][..stride];
        for x in 0..width as usize {
            let px: [u8; 4] = match bpp {
                32 if cmyk => {
                    let ink = [src[x * 4], src[x * 4 + 1], src[x * 4 + 2], src[x * 4 + 3]];
                    cmyk_pixel(ink, lut)
                }
                32 => [src[x * 4 + 2], src[x * 4 + 1], src[x * 4], 255],
                24 => [src[x * 3 + 2], src[x * 3 + 1], src[x * 3], 255],
                8 => {
                    let i = src[x] as usize;
                    match palette.get(i) {
                        Some(c) => [c[0], c[1], c[2], 255],
                        None => [src[x], src[x], src[x], 255],
                    }
                }
                _ => {
                    let bit = (src[x / 8] >> (7 - (x % 8))) & 1;
                    match palette.get(bit as usize) {
                        Some(c) => [c[0], c[1], c[2], 255],
                        None => {
                            let g = if bit == 0 { 0 } else { 255 };
                            [g, g, g, 255]
                        }
                    }
                }
            };
            rgba.extend_from_slice(&px);
        }
    }
    if let Some(alpha) = bitmap_mask(d, width, height) {
        for (px, a) in rgba.chunks_exact_mut(4).zip(alpha) {
            px[3] = a;
        }
    }
    let png = encode_png(width, height, &rgba);
    Some(Ok((
        image_id,
        BitmapDef {
            width_px: width,
            height_px: height,
            png: png.into(),
        },
    )))
}

/// `f` over `items` in order, on up to four threads where the platform
/// has them (a browser does not).
fn parallel_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .min(4)
            .min(items.len());
        if threads > 1 {
            let next = std::sync::atomic::AtomicUsize::new(0);
            let mut done: Vec<(usize, R)> = std::thread::scope(|scope| {
                let workers: Vec<_> = (0..threads)
                    .map(|_| {
                        scope.spawn(|| {
                            let mut out = Vec::new();
                            loop {
                                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                let Some(item) = items.get(i) else {
                                    break;
                                };
                                out.push((i, f(item)));
                            }
                            out
                        })
                    })
                    .collect();
                workers
                    .into_iter()
                    .flat_map(|w| w.join().unwrap_or_default())
                    .collect()
            });
            if done.len() == items.len() {
                done.sort_by_key(|(i, _)| *i);
                return done.into_iter().map(|(_, r)| r).collect();
            }
            // A worker failed: do it all again in order.
        }
    }
    items.iter().map(f).collect()
}

/// A paragraph frame (`origin` its lower left, `size`) under `transform`
/// as the same frame on the page with the transform's scale taken into it:
/// the transform without its scale, the frame's new lower left and size.
/// `None` for a transform that slants or has no size.
fn unscaled_frame(transform: Affine, origin: Point, size: Size) -> Option<(Affine, Point, Size)> {
    let [a, b, c, d, e, f] = transform.as_coeffs();
    let sx = a.hypot(b);
    let sy = c.hypot(d);
    if !(sx > 1e-9 && sy > 1e-9 && sx.is_finite() && sy.is_finite()) {
        return None;
    }
    if (a * c + b * d).abs() > 1e-6 * sx * sy {
        return None;
    }
    if (sx - 1.0).abs() < 1e-9 && (sy - 1.0).abs() < 1e-9 {
        return None;
    }
    let t = Affine::new([a / sx, b / sx, c / sy, d / sy, e, f]);
    Some((
        t,
        Point::new(origin.x * sx, origin.y * sy),
        Size::new(size.width * sx, size.height * sy),
    ))
}

/// The transparency of a `bmp ` chunk `d` of `width` x `height` pixels,
/// top row first (255 opaque, 0 clear), when it has one: a u32 at byte
/// 24 gives the place of a second image block counted from byte 8, which
/// is tagged "RI", has its block length, a header of 78 bytes and one byte
/// per pixel in rows padded to 4 bytes, bottom row first (seen in version
/// 21 and 22 files: cut-out photos whose exports show the page through the
/// zero values).
fn bitmap_mask(d: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let u32_at = |at: usize| -> Option<u32> {
        d.get(at..at.checked_add(4)?)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let offset = u32_at(24)? as usize;
    if offset == 0 {
        return None;
    }
    let at = offset.checked_add(8)?;
    let header = u32_at(at.checked_add(10)?)? as usize;
    let ok = d.get(at..at + 2) == Some(b"RI".as_slice())
        && u32_at(at + 22)? == width
        && u32_at(at + 26)? == height
        && u32_at(at + 34)? == 8
        && header >= 42;
    if !ok {
        return None;
    }
    let (w, h) = (width as usize, height as usize);
    let stride = w.div_ceil(4) * 4;
    let pixels = d.get(at.checked_add(header)?..)?;
    if pixels.len() < stride.checked_mul(h)? {
        return None;
    }
    let mut out = Vec::with_capacity(w * h);
    for row in 0..h {
        let src = pixels.get((h - 1 - row) * stride..)?.get(..w)?;
        out.extend_from_slice(src);
    }
    Some(out)
}

/// The bitmap colour model of CMYK images.
const BITMAP_CMYK: u32 = 3;

/// One CMYK pixel (ink bytes, 255 = full ink) as opaque RGBA: through
/// `lut` when given, else one minus the ink times one minus the black.
fn cmyk_pixel(ink: [u8; 4], lut: Option<&tracedraw_core::icc::CmykLut>) -> [u8; 4] {
    let f = |v: u8| v as f32 / 255.0;
    let rgb = match lut {
        Some(l) => l.eval([f(ink[0]), f(ink[1]), f(ink[2]), f(ink[3])]),
        None => {
            let k = 1.0 - f(ink[3]);
            [
                (1.0 - f(ink[0])) * k,
                (1.0 - f(ink[1])) * k,
                (1.0 - f(ink[2])) * k,
            ]
        }
    };
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [b(rgb[0]), b(rgb[1]), b(rgb[2]), 255]
}

/// A bitmap with a crop outline becomes a clip frame whose frame is that
/// outline.
fn crop_to_clip_frame(
    mut shape: Shape,
    crop: BezPath,
    ids: &mut tracedraw_core::id::IdSource,
) -> Shape {
    let mut frame = Shape::new(
        ids.shape(),
        ShapeKind::Path {
            path: crop,
            closed: true,
        },
    );
    frame.fill = Fill::None;
    frame.stroke = None;
    let mut inner = Shape::new(ids.shape(), shape.kind.clone());
    inner.fill = shape.fill.clone();
    inner.stroke = shape.stroke.clone();
    shape.kind = ShapeKind::ClipFrame {
        frame: Box::new(frame),
        contents: vec![inner],
    };
    shape.fill = Fill::None;
    shape.stroke = None;
    shape
}

/// The control point distance of a quarter ellipse drawn as one cubic.
const KAPPA: f64 = 0.552_284_749_830_793_4;

/// An ellipse's outline starting at its top and running clockwise.
fn ellipse_from_top(rect: Rect) -> BezPath {
    let c = rect.center();
    let (rx, ry) = (rect.width() / 2.0, rect.height() / 2.0);
    let (kx, ky) = (KAPPA * rx, KAPPA * ry);
    let mut p = BezPath::new();
    p.move_to((c.x, c.y + ry));
    p.curve_to((c.x + kx, c.y + ry), (c.x + rx, c.y + ky), (c.x + rx, c.y));
    p.curve_to((c.x + rx, c.y - ky), (c.x + kx, c.y - ry), (c.x, c.y - ry));
    p.curve_to((c.x - kx, c.y - ry), (c.x - rx, c.y - ky), (c.x - rx, c.y));
    p.curve_to((c.x - rx, c.y + ky), (c.x - kx, c.y + ry), (c.x, c.y + ry));
    p.close_path();
    p
}

/// The curve a fitted text follows, on the page: the curve's outline; an
/// ellipse's starts at its top and runs clockwise.
fn text_path_of(curve: &Shape) -> BezPath {
    match &curve.kind {
        ShapeKind::Ellipse { rect, arc: None } => curve.transform * ellipse_from_top(*rect),
        _ => curve.page_path(),
    }
}

/// A fitted text's settings on `path`, whose units are `per_mm` to the
/// millimetre: the text on the path, its alignment and the position along
/// the baseline that the settings' offset names (the text's middle when
/// centred).
fn text_on(
    path: BezPath,
    fitted: &OnPathText,
    per_mm: f64,
) -> (tracedraw_core::TextOnPath, TextAlign, f64) {
    let Some(s) = fitted.settings else {
        let tp = tracedraw_core::TextOnPath {
            path,
            offset: 0.0,
            distance: 0.0,
            mirror: false,
            ..Default::default()
        };
        return (tp, TextAlign::Left, 0.0);
    };
    // The file measures the distance to the left of the path and the
    // offset from its start, mirrored or not; a mirrored text of ours
    // follows the path backwards.
    let mirror = s.mirror_h && s.mirror_v;
    let distance = f64::from(s.distance) * UNIT_MM * per_mm;
    let mut tp = tracedraw_core::TextOnPath {
        path,
        offset: 0.0,
        distance: if mirror { -distance } else { distance },
        mirror,
        ..Default::default()
    };
    let total = tp.baseline().length();
    let mut at = f64::from(s.offset) * UNIT_MM * per_mm;
    if mirror {
        at = total - at;
    }
    if s.placement == 4 {
        tp.offset = at - total / 2.0;
        (tp, TextAlign::Center, at)
    } else {
        tp.offset = at;
        (tp, TextAlign::Left, at)
    }
}

/// The curve among `curves` that a fitted text follows: the one whose
/// baseline puts the point the text's offset names inside the fitted
/// text's box on the page (1 mm to spare), the nearest such point to the
/// box's centre when several do.
fn path_for_text(shapes: &[Shape], curves: &[usize], fitted: &OnPathText) -> Option<usize> {
    // The text's start lies on its path, inside the text's box or just
    // outside it (text set below a path, seen in version 12 files: 0.8 mm
    // out on a 340 mm box), so the box grows by 2 % of its size.
    let pb = fitted.page_box?;
    let grow = 1.0 + 0.02 * pb.width().max(pb.height());
    let b = pb.inflate(grow, grow);
    let mut best: Option<(usize, f64)> = None;
    for &ci in curves {
        let Some(curve) = shapes.get(ci) else {
            continue;
        };
        let (tp, _, at) = text_on(text_path_of(curve), fitted, 1.0);
        let Some((p, _)) = tp.baseline().at(at) else {
            continue;
        };
        log::debug!(
            "text on path: curve {ci} ({:?}) puts the text at {p:?} (box {b:?})",
            curve.bounds()
        );
        if !b.contains(p) {
            continue;
        }
        let d = (p - b.center()).hypot();
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((ci, d));
        }
    }
    best.map(|(ci, _)| ci)
}

/// Where a fitted text lies along its path when the file's settings do
/// not say (before version 16): the point of the text's box farthest out
/// from the path's middle, near which the baseline runs, and the path's
/// middle, which the letters face away from.
#[derive(Debug, Clone, Copy)]
struct Anchor {
    target: Point,
    middle: Point,
}

/// The curve a fitted text follows judged by its box alone: the box sits
/// on one side of a closed curve (a line of text along the bottom of a
/// disc label), so the box's edge facing away from the curve's middle lies
/// on the curve. Returns the curve nearest that edge, within 3 % of the
/// box's size, and where the text goes on it. The letters hang outwards,
/// so the baseline runs a capital's height (`cap`, mm) inside that edge.
/// A box around the whole curve (text all the way round) names no side
/// and is left to [`path_for_text`].
fn anchored_path_for_text(
    shapes: &[Shape],
    curves: &[usize],
    fitted: &OnPathText,
    cap: f64,
) -> Option<(usize, Option<Anchor>)> {
    let pb = fitted.page_box?;
    let centre = pb.center();
    let size = pb.width().max(pb.height());
    let tolerance = 1.0 + 0.03 * size;
    let mut best: Option<(usize, f64, Anchor)> = None;
    for &ci in curves {
        let Some(curve) = shapes.get(ci) else {
            continue;
        };
        let path = text_path_of(curve);
        let cb = {
            use tracedraw_core::geometry::Shape as _;
            path.bounding_box()
        };
        let middle = cb.center();
        let (ax, ay) = (centre.x - middle.x, centre.y - middle.y);
        let len = ax.hypot(ay);
        if !len.is_finite() || len < 0.1 * cb.width().max(cb.height()) {
            continue;
        }
        let dir = tracedraw_core::geometry::Vec2::new(ax / len, ay / len);
        // The box's support point in that direction.
        let target = Point::new(
            if dir.x >= 0.0 { pb.x1 } else { pb.x0 },
            if dir.y >= 0.0 { pb.y1 } else { pb.y0 },
        );
        let target = Point::new(
            if dir.x.abs() < 0.3 {
                centre.x
            } else {
                target.x - dir.x.signum() * cap
            },
            if dir.y.abs() < 0.3 {
                centre.y
            } else {
                target.y - dir.y.signum() * cap
            },
        );
        let Some(d) = distance_to_path(&path, target) else {
            continue;
        };
        if d > tolerance {
            continue;
        }
        let anchor = Anchor { target, middle };
        if best.map(|(_, bd, _)| d < bd).unwrap_or(true) {
            best = Some((ci, d, anchor));
        }
    }
    best.map(|(ci, _, a)| (ci, Some(a)))
}

/// The height of a capital of the text's largest letters on the page,
/// mm (seven tenths of the size).
fn cap_height_mm(t: &Shape) -> f64 {
    let ShapeKind::Text { spans, .. } = &t.kind else {
        return 0.0;
    };
    let pt = spans.iter().map(|s| s.size_pt).fold(0.0, f64::max);
    let scale = t.transform.determinant().abs().sqrt();
    let cap = pt * 25.4 / 72.0 * 0.7 * scale;
    if cap.is_finite() {
        cap.min(50.0)
    } else {
        0.0
    }
}

/// The distance from `p` to the first subpath of `path` (sampled every
/// half millimetre).
fn distance_to_path(path: &BezPath, p: Point) -> Option<f64> {
    let tp = tracedraw_core::TextOnPath {
        path: path.clone(),
        offset: 0.0,
        distance: 0.0,
        mirror: false,
        ..Default::default()
    };
    nearest_on(&tp.baseline(), p).map(|(_, d)| d)
}

/// The arc length along `b` of the point nearest `p`, and its distance.
fn nearest_on(b: &tracedraw_core::document::Baseline, p: Point) -> Option<(f64, f64)> {
    let total = b.length();
    if !total.is_finite() || total <= 0.0 {
        return None;
    }
    let steps = ((total / 0.5).ceil() as usize).clamp(8, 20_000);
    let mut best: Option<(f64, f64)> = None;
    for i in 0..=steps {
        let s = total * i as f64 / steps as f64;
        let Some((q, _)) = b.at(s) else {
            continue;
        };
        let d = (q - p).hypot();
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((s, d));
        }
    }
    best
}

/// Place text `t` on `curve` centred at the anchor's target, its letters
/// standing away from the curve's middle: a version 12 disc label's line
/// along the bottom of its circle has its box reach a capital's height
/// below the circle, so the letters hang outwards there.
fn fit_text_at(t: &mut Shape, curve: &Shape, anchor: Anchor) -> bool {
    let ShapeKind::Text {
        spans,
        origin,
        para,
        ..
    } = t.kind.clone()
    else {
        return false;
    };
    let det = t.transform.determinant();
    if !det.is_finite() || det.abs() < 1e-12 {
        return false;
    }
    let to_local = Affine::translate(-origin.to_vec2()) * t.transform.inverse();
    let path = to_local * text_path_of(curve);
    let target = to_local * anchor.target;
    let middle = to_local * anchor.middle;
    let mut tp = tracedraw_core::TextOnPath {
        path,
        offset: 0.0,
        distance: 0.0,
        mirror: false,
        ..Default::default()
    };
    let Some((at, _)) = nearest_on(&tp.baseline(), target) else {
        return false;
    };
    // The letters' tops point to the left of the way the baseline runs;
    // follow the path backwards when that is towards the middle.
    let Some((q, dir)) = tp.baseline().at(at) else {
        return false;
    };
    let left = tracedraw_core::geometry::Vec2::new(-dir.y, dir.x);
    let (ox, oy) = (q.x - middle.x, q.y - middle.y);
    if left.x * ox + left.y * oy < 0.0 {
        tp.mirror = true;
    }
    let b = tp.baseline();
    let Some((at, _)) = nearest_on(&b, target) else {
        return false;
    };
    tp.offset = at - b.length() / 2.0;
    t.kind = ShapeKind::Text {
        spans,
        origin,
        frame: None,
        align: TextAlign::Center,
        para,
        on_path: Some(tp),
    };
    true
}

/// Place text `t` on `curve` as `fitted` says. The path is taken into the
/// text's own space (the text keeps its transform, the curve stays a
/// separate, often invisible, object).
fn fit_text(t: &mut Shape, curve: &Shape, fitted: &OnPathText) -> bool {
    let ShapeKind::Text {
        spans,
        origin,
        align,
        para,
        ..
    } = t.kind.clone()
    else {
        return false;
    };
    let det = t.transform.determinant();
    if !det.is_finite() || det.abs() < 1e-12 {
        return false;
    }
    let to_local = Affine::translate(-origin.to_vec2()) * t.transform.inverse();
    let path = to_local * text_path_of(curve);
    let (tp, fitted_align, _) = text_on(path, fitted, 1.0 / det.abs().sqrt());
    t.kind = ShapeKind::Text {
        spans,
        origin,
        frame: None,
        align: if fitted.settings.is_some() {
            fitted_align
        } else {
            align
        },
        para,
        on_path: Some(tp),
    };
    true
}

/// Where a blend's steps start on an end object, in its own coordinates:
/// its first node and a point after it along the outline. A rectangle
/// runs from its corner at the origin (where the file's transform places
/// it) along its height first: the steps of the two blends attached to
/// issue #597 turn and fold as they do only from there. Other objects
/// run from their own first node, as their outline is built (a polygon
/// from its top vertex, clockwise, as the file stores it). `None` when
/// the outline has fewer than two distinct points.
fn blend_nodes(s: &Shape) -> Option<[Point; 2]> {
    if let ShapeKind::Rect { rect, .. } = &s.kind {
        let near = |a: f64, b: f64| if a.abs() <= b.abs() { a } else { b };
        let far = |a: f64, b: f64| if a.abs() <= b.abs() { b } else { a };
        let first = Point::new(near(rect.x0, rect.x1), near(rect.y0, rect.y1));
        let next = Point::new(first.x, far(rect.y0, rect.y1));
        return ((next - first).hypot() > 1e-9).then_some([first, next]);
    }
    use tracedraw_core::geometry::PathEl;
    let path = s.local_path();
    let mut nodes = path.elements().iter().filter_map(|el| match el {
        PathEl::MoveTo(p) | PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => {
            Some(*p)
        }
        PathEl::ClosePath => None,
    });
    let first = nodes.next()?;
    let next = nodes.find(|p| (*p - first).hypot() > 1e-6)?;
    Some([first, next])
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

/// Sort an arrowhead outline into one of our presets by its shape: a
/// closed curve made only of Bezier segments is a circle; three corners
/// make an arrow (filled) or an open arrow (not closed); four corners make
/// a square (axis-aligned, near 1:1), a bar (thin) or a diamond (rotated).
pub(crate) fn classify_arrowhead(path: &BezPath, pts: &[Point], types: &[u8]) -> Arrowhead {
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
        // Curved arrowheads with straight parts keep their own outline.
        return Arrowhead::None;
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
/// A colour from its model code and four value bytes (the encoding shared
/// by `.cdr` from version 5 and CMX): `None` for models we do not model.
pub(crate) fn color_from_model(model: u16, b: [u8; 4]) -> Option<Color> {
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
        // Spot, Lab, HSB, HLS, YIQ and others.
        _ => return None,
    })
}

/// A version 3 object's transformations, split at its envelope.
struct V3Transforms {
    before: Affine,
    envelope: Option<(Rect, tracedraw_core::envelope::Frame)>,
    after: Affine,
}

/// A version 3 envelope (transformation type 0x14) at `at` in `g`: two
/// u16, the box it maps from (i16 left, top, right, bottom in 1/1000 inch),
/// the indexes of the four corner points and the point count (five u16),
/// then the points (i16 pairs). The points run clockwise from the
/// bottom-left corner up the left side, three a cubic segment (node and
/// two handles), the last one closing on the first.
fn v3_envelope(g: &[u8], at: usize) -> Option<(Rect, tracedraw_core::envelope::Frame)> {
    let i = |k: usize| {
        g.get(at + k..at + k + 2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]) as f64)
    };
    let mm = |v: f64| v * UNIT16_MM;
    let (x0, y1, x1, y0) = (i(4)?, i(6)?, i(8)?, i(10)?);
    let rect = Rect::new(
        mm(x0.min(x1)),
        mm(y0.min(y1)),
        mm(x0.max(x1)),
        mm(y0.max(y1)),
    );
    let corners_at: Vec<usize> = (0..4)
        .map(|k| i(12 + 2 * k).map(|v| v as usize))
        .collect::<Option<_>>()?;
    let total = i(20)? as usize;
    if total < 13
        || !(total - 1).is_multiple_of(3)
        || total > 3 * tracedraw_core::envelope::MAX_NODES + 1
    {
        return None;
    }
    let pts: Vec<Point> = (0..total)
        .map(|k| Some(Point::new(mm(i(22 + 4 * k)?), mm(i(24 + 4 * k)?))))
        .collect::<Option<_>>()?;
    let k = (total - 1) / 3;
    // Turn the clockwise list round: node j is file node (k - j) % k, and
    // the segment from it runs back along file segment k - 1 - j.
    let nodes: Vec<Point> = (0..k).map(|j| pts[3 * ((k - j) % k)]).collect();
    let handles: Vec<[Point; 2]> = (0..k)
        .map(|j| {
            let m = k - 1 - j;
            [pts[3 * m + 2], pts[3 * m + 1]]
        })
        .collect();
    let mut corners: Vec<usize> = corners_at
        .iter()
        .filter(|c| **c % 3 == 0)
        .map(|c| (k - (c / 3) % k) % k)
        .collect();
    corners.sort_unstable();
    corners.dedup();
    let frame = tracedraw_core::envelope::Frame::from_parts(&nodes, &handles, &corners)?;
    (rect.width() > 0.0 && rect.height() > 0.0).then_some((rect, frame))
}

/// The font names a version 3 `ftbl` holds: NUL-terminated runs of at
/// least three lowercase letters, first appearance first.
fn v3_font_names(d: &[u8]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in d.split(|b| *b == 0) {
        let tail: Vec<u8> = part
            .iter()
            .rev()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            .copied()
            .collect();
        if tail.len() >= 3 {
            let name: String = tail.iter().rev().map(|c| *c as char).collect();
            if !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}

/// A matrix of versions 3 and 4: a, c, e, b, d, f as i32, the factors in
/// 16.16 fixed point and the offsets in 1/1000 inch, so that
/// x' = a x + c y + e and y' = b x + d y + f.
fn fixed_matrix(r: &mut Reader) -> Option<Affine> {
    let k = |v: i32| v as f64 / 65536.0;
    let a = k(r.i32()?);
    let c = k(r.i32()?);
    let e = r.i32()? as f64 * UNIT16_MM;
    let b = k(r.i32()?);
    let d = k(r.i32()?);
    let f = r.i32()? as f64 * UNIT16_MM;
    Some(Affine::new([a, b, c, d, e, f]))
}

/// A path from points and their node bytes: bits 7-6 are the node kind
/// (00 move, 01 line, 10 curve end, 11 control point) and bit 3 closes the
/// subpath after the node.
/// The node types a curve's node bytes give (bits 5..4 of a node's byte:
/// 0 cusp, 1 smooth, 2 symmetrical), for the nodes of `path` (built from
/// `pts` and `types`) whose handles read otherwise.
pub(crate) fn file_node_types(
    path: &BezPath,
    pts: &[Point],
    types: &[u8],
) -> std::collections::BTreeMap<usize, tracedraw_core::nodes::NodeType> {
    use tracedraw_core::nodes::{self, NodeType};
    let ends: Vec<(Point, u8)> = pts
        .iter()
        .zip(types)
        .filter(|(_, t)| *t >> 6 != 0b11)
        .map(|(p, t)| (*p, (t >> 4) & 3))
        .collect();
    let mut used = vec![false; ends.len()];
    // Nodes come in the order the points were written: look from where
    // the last match was first, so a curve of thousands of nodes is read
    // in one pass (whole-list searches made a large file take minutes).
    let mut cursor = 0;
    let mut out = std::collections::BTreeMap::new();
    for n in nodes::nodes(path) {
        let matches = |k: &usize| !used[*k] && (ends[*k].0 - n.pos).hypot() < 1e-9;
        let Some(k) = (cursor..ends.len()).chain(0..cursor).find(|k| matches(k)) else {
            continue;
        };
        used[k] = true;
        cursor = k + 1;
        let ty = match ends[k].1 {
            0 => NodeType::Cusp,
            1 => NodeType::Smooth,
            2 => NodeType::Symmetrical,
            _ => continue,
        };
        if nodes::node_type_of(&n) != ty {
            out.insert(n.index, ty);
        }
    }
    out
}

pub(crate) fn build_path(pts: &[Point], types: &[u8]) -> (BezPath, bool) {
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

/// The pixels of `png` as the encoder's rows were before it filtered
/// them: a zero byte, then the row's RGBA, top row first. For tests. The
/// decoder premultiplies, so colours are exact only where a pixel is
/// opaque (alpha is always exact).
#[cfg(test)]
pub(crate) fn raw_rows(png: &[u8]) -> Vec<u8> {
    let pm = tiny_skia::Pixmap::decode_png(png).expect("a png");
    let mut out = Vec::new();
    for y in 0..pm.height() {
        out.push(0);
        for x in 0..pm.width() {
            let c = pm.pixel(x, y).expect("pixel").demultiply();
            out.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
        }
    }
    out
}

/// Minimal PNG encoder: 8-bit RGB when every pixel is opaque, else RGBA,
/// each row with the Sub filter, one IDAT chunk. The rows go into a fast
/// zlib stream: the PNG only carries the pixels inside the document, and
/// the default level made opening files with large pictures several
/// times slower. Unfiltered RGBA came out nearly as large as the raw
/// pixels for photos (40 MB for a 4000 x 3000 one, 22 MB like this), and
/// every undo step and copy of the drawing used to hold it.
pub(crate) fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    use std::io::Write;
    fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], body: &[u8]) {
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        let mut crc = flate2::Crc::new();
        crc.update(tag);
        crc.update(body);
        out.extend_from_slice(tag);
        out.extend_from_slice(body);
        out.extend_from_slice(&crc.sum().to_be_bytes());
    }
    let row = width as usize * 4;
    let pixels = rgba.get(..row * height as usize).unwrap_or(rgba);
    // Rows the data does not reach are blank (clear).
    let opaque =
        pixels.len() == row * height as usize && pixels.chunks_exact(4).all(|p| p[3] == 255);
    let bpp = if opaque { 3 } else { 4 };
    let mut enc = flate2::write::ZlibEncoder::new(
        Vec::with_capacity(rgba.len() / 4),
        flate2::Compression::fast(),
    );
    let blank = vec![0u8; row];
    let mut line = Vec::with_capacity(1 + width as usize * bpp);
    let mut ok = true;
    for y in 0..height as usize {
        let src = rgba.get(y * row..(y + 1) * row).unwrap_or(&blank[..]);
        line.clear();
        line.push(1);
        if opaque {
            src.chunks_exact(4)
                .for_each(|p| line.extend_from_slice(&p[..3]));
        } else {
            line.extend_from_slice(src);
        }
        // Filter type 1 (Sub): each byte less the same byte of the pixel
        // to its left (from the end, so the left ones are still raw).
        let px = &mut line[1..];
        for i in (bpp..px.len()).rev() {
            px[i] = px[i].wrapping_sub(px[i - bpp]);
        }
        ok &= enc.write_all(&line).is_ok();
    }
    let idat = if ok {
        enc.finish().unwrap_or_default()
    } else {
        Vec::new()
    };
    let mut out = Vec::with_capacity(idat.len() + 64);
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, if opaque { 2 } else { 6 }, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &idat);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// A rectangle's corners from the radii in file order: one radius for
/// every corner, or four running counter-clockwise from the corner at
/// (width, 0) (assumed, see `docs/cdr-format.md`). `w` and `h` are the
/// signed size from the object's origin, so a negative one mirrors the
/// corners onto the other side.
fn file_corners(radii: &[f64], w: f64, h: f64) -> Corners {
    let r = |i: usize| -> f64 {
        let v = radii.get(i).or(radii.first()).copied().unwrap_or(0.0);
        if v.is_finite() {
            v.max(0.0)
        } else {
            0.0
        }
    };
    // Corner positions in file order, as (right, top) flags for w, h > 0.
    let order = [(true, false), (true, true), (false, true), (false, false)];
    let mut c = Corners::default();
    for (i, (right, top)) in order.into_iter().enumerate() {
        let right = right == (w >= 0.0);
        let top = top == (h >= 0.0);
        let at = match (right, top) {
            (false, true) => Corners::TOP_LEFT,
            (true, true) => Corners::TOP_RIGHT,
            (false, false) => Corners::BOTTOM_LEFT,
            (true, false) => Corners::BOTTOM_RIGHT,
        };
        c.radii[at] = r(i);
    }
    c
}

/// Move a facing page's objects half a page left or right, whichever puts
/// more of their centres on the page (`width` across).
fn shift_onto_page(layers: &mut [Layer], width: f64) {
    let centres: Vec<f64> = layers
        .iter()
        .flat_map(|l| l.shapes.iter())
        .map(|s| s.bounds())
        .filter(|b| b.x0.is_finite() && b.x1.is_finite())
        .map(|b| (b.x0 + b.x1) / 2.0)
        .collect();
    let on_page = |dx: f64| {
        centres
            .iter()
            .filter(|c| (0.0..=width).contains(&(**c + dx)))
            .count()
    };
    let half = width / 2.0;
    let dx = if on_page(half) > on_page(-half) {
        half
    } else {
        -half
    };
    for s in layers.iter_mut().flat_map(|l| l.shapes.iter_mut()) {
        s.transform = Affine::translate((dx, 0.0)) * s.transform;
    }
}

/// A paragraph's line spacing, space before and after and indents from
/// its resolved style, the values of `base` where it says nothing.
fn paragraph_spacing(style: &text::RunStyle, base: &ParagraphStyle) -> ParagraphStyle {
    let mut p = base.clone();
    if let Some(ls) = style.line_spacing {
        p.leading_pct = (ls * 100.0).clamp(10.0, 1000.0);
    } else if let Some(mm) = style.line_spacing_mm {
        // A distance between baselines, as a share of the font's line
        // height, which our leading counts in (taken as 1.15 times the
        // size, as common text faces have it).
        let line_mm = style.size_pt.unwrap_or(12.0) * 25.4 / 72.0 * 1.15;
        if line_mm > 0.0 {
            p.leading_pct = (mm / line_mm * 100.0).clamp(10.0, 1000.0);
        }
    }
    // Space before and after (version 16+ style strings), in character
    // heights when the spacing unit is the character height: 1 before is
    // the usual line spacing, so only what goes past it is extra room.
    if matches!(style.spacing_method, None | Some(0)) {
        let line_mm = style.size_pt.unwrap_or(12.0) * 25.4 / 72.0 * 1.15;
        if let Some(b) = style.para_before {
            p.space_before = ((b - 1.0).max(0.0) * line_mm).min(1000.0);
        }
        if let Some(a) = style.para_after {
            p.space_after = (a.max(0.0) * line_mm).min(1000.0);
        }
    }
    if let Some((left, first, right)) = style.indent_pt {
        const PT_MM: f64 = 25.4 / 72.0;
        p.left_indent = left * PT_MM;
        p.first_line_indent = first * PT_MM;
        p.right_indent = right * PT_MM;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_spacing_before_and_after_come_from_the_style_string() {
        let st = text::parse_style_string(
            r#"{"character":{"latin":{"size":"304800"}},"paragraph":{"beforeParaSpacing":"2000000","afterParaSpacing":"500000","spacingMethod":"0","interLineSpacing":"1200000"}}"#,
        );
        assert_eq!(st.para_before, Some(2.0));
        assert_eq!(st.para_after, Some(0.5));
        let p = paragraph_spacing(&st, &ParagraphStyle::default());
        let line_mm = st.size_pt.unwrap_or(12.0) * 25.4 / 72.0 * 1.15;
        assert!((p.space_before - line_mm).abs() < 1e-9, "{p:?}");
        assert!((p.space_after - line_mm / 2.0).abs() < 1e-9, "{p:?}");
        assert!((p.leading_pct - 120.0).abs() < 1e-9);
        // The defaults files hold: no extra room.
        let d = text::parse_style_string(
            r#"{"paragraph":{"beforeParaSpacing":"1000000","afterParaSpacing":"0"}}"#,
        );
        let p = paragraph_spacing(&d, &ParagraphStyle::default());
        assert_eq!((p.space_before, p.space_after), (0.0, 0.0));
        // Another unit is not read.
        let other = text::parse_style_string(
            r#"{"paragraph":{"beforeParaSpacing":"2000000","spacingMethod":"1"}}"#,
        );
        assert_eq!(
            paragraph_spacing(&other, &ParagraphStyle::default()).space_before,
            0.0
        );
    }
    use tracedraw_core::geometry::Shape as _;

    #[test]
    fn continuity_bits_give_the_node_types_the_handles_do_not() {
        use tracedraw_core::nodes::NodeType;
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(20.0, 10.0),
            Point::new(30.0, 0.0),
            Point::new(40.0, -10.0),
            Point::new(50.0, -10.0),
            Point::new(60.0, 0.0),
            Point::new(70.0, 0.0),
        ];
        // move, control, control, curve end (cusp bits on aligned handles),
        // control, control, curve end (cusp bits before a line, matching the
        // handles), line (bits 3: unknown, ignored).
        let types = [0x00, 0xC0, 0xC0, 0x80, 0xC0, 0xC0, 0x80, 0x70];
        let (path, _) = build_path(&pts, &types);
        let got = file_node_types(&path, &pts, &types);
        assert_eq!(got, [(1, NodeType::Cusp)].into());
        // Truncated or mismatched lists never panic.
        let _ = file_node_types(&path, &pts[..3], &types[..2]);
        let _ = file_node_types(&BezPath::new(), &pts, &types);
    }

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
        // Model 0x19 is a library colour like models 1 and 0x0e: its
        // number and tint, the ink taking its look from the preview.
        let mut c19 = u16s(&[0x19, 7]);
        c19.extend_from_slice(&[0; 4]);
        c19.extend_from_slice(&[0x1b, 0, 100, 0]);
        assert_eq!(
            ctx.read_color(&mut Reader::new(&c19, false)),
            Some(crate::spot::library_color(7, 27, 100))
        );
        assert!(ctx.library_inks.contains(&(7, 27)));
        assert!(ctx.report.warnings.is_empty(), "{:?}", ctx.report.warnings);
    }

    #[test]
    fn solid_fill_before_version_13_has_two_unknown_bytes_before_the_colour() {
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
            ShapeKind::Rect { rect, radius, .. } => {
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

    /// A mesh fill object (type 0x20, a version 11 file, #896): a curve
    /// whose fill is the grid of argument 19150, its outline the mesh's
    /// edge, not the coordinates' stale curve.
    #[test]
    fn mesh_fill_objects_are_read_with_their_grid() {
        let unit = 100_000; // 10 mm
                            // A line through nodes (row, col) at (col, row) * 10 mm.
        let line = |nodes: &[(i32, i32)], column: bool| -> Vec<u8> {
            let mut v = u32s(&[100, nodes.len() as u32]);
            for (r, c) in nodes {
                v.extend_from_slice(&i32s(&[c * unit, r * unit]));
            }
            v.push(0x04);
            v.extend(std::iter::repeat_n(0x44, nodes.len() - 1));
            for (r, c) in nodes {
                v.extend_from_slice(&i32s(&[*r, *c]));
            }
            let n = (nodes.len() - 1).max(1) as f64;
            v.extend_from_slice(&f64s(
                &(0..nodes.len()).map(|i| i as f64 / n).collect::<Vec<_>>(),
            ));
            v.extend_from_slice(&f64s(&[0.0]));
            v.extend_from_slice(&u32s(&[u32::from(column)]));
            v
        };
        let mut body = u32s(&[100, 2, 2]);
        body.extend(line(&[(0, 0), (1, 0)], true));
        body.extend(line(&[(1, 1), (0, 1)], true));
        body.extend(line(&[(0, 0), (0, 1)], false));
        body.extend(line(&[(1, 0), (1, 1)], false));
        body.extend(f64s(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0]));
        body.extend(u32s(&[100, 2, 2]));
        for (r, g, b) in [(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 255)] {
            body.extend(color_bgr(r, g, b));
        }
        let mut mesh = u32s(&[body.len() as u32 + 4]);
        mesh.extend(body);
        // The coordinates' curve: a triangle far from the mesh.
        let mut coords = u32s(&[3]);
        coords.extend(i32s(&[
            5 * unit,
            5 * unit,
            6 * unit,
            5 * unit,
            5 * unit,
            6 * unit,
        ]));
        coords.extend([0x04, 0x44, 0x4c]);
        let obj = |mesh: Vec<u8>| {
            list(
                b"obj ",
                &chunk(
                    b"loda",
                    &loda(OBJ_MESH, &[(ARG_MESH, mesh), (ARG_COORDS, coords.clone())]),
                ),
            )
        };
        let doc_of = |obj: Vec<u8>| {
            let body = [
                chunk(b"mcfg", &mcfg(11, 2_540_000, 2_540_000)),
                list(b"page", &list(b"layr", &obj)),
            ]
            .concat();
            parse(&riff(b"CDRB", &body), 11)
        };
        let (doc, rep) = doc_of(obj(mesh.clone()));
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        assert_eq!(rep.skipped_objects, 0);
        let s = first_shape(&doc);
        let Fill::Mesh(m) = &s.fill else {
            panic!("{:?}", s.fill)
        };
        assert_eq!((m.rows, m.cols), (1, 1));
        let at = |r, c| m.node(r, c).map(|n| (n.pos, n.color));
        assert_eq!(
            at(0, 0),
            Some((Point::new(0.0, 0.0), Color::rgb8(255, 0, 0)))
        );
        assert_eq!(
            at(0, 1),
            Some((Point::new(10.0, 0.0), Color::rgb8(0, 255, 0)))
        );
        assert_eq!(
            at(1, 0),
            Some((Point::new(0.0, 10.0), Color::rgb8(0, 0, 255)))
        );
        assert_eq!(
            at(1, 1),
            Some((Point::new(10.0, 10.0), Color::rgb8(255, 255, 255)))
        );
        // The outline is the mesh's edge, round the square.
        let ShapeKind::Path { path, closed } = &s.kind else {
            panic!("{:?}", s.kind)
        };
        assert!(closed);
        let b = path.bounding_box();
        assert!(b.x0.abs() < 1e-9 && (b.x1 - 10.0).abs() < 1e-9, "{b:?}");
        assert!(b.y0.abs() < 1e-9 && (b.y1 - 10.0).abs() < 1e-9, "{b:?}");
        // A mesh cut short keeps the curve, with a warning.
        let (doc, rep) = doc_of(obj(mesh[..mesh.len() - 20].to_vec()));
        assert_eq!(rep.shapes, 1);
        assert!(rep.warnings.iter().any(|w| w.contains("mesh")));
        assert!(!matches!(first_shape(&doc).fill, Fill::Mesh(_)));
        // Every shorter cut opens without a panic.
        for cut in 0..mesh.len() {
            let _ = doc_of(obj(mesh[..cut].to_vec()));
        }
    }

    #[test]
    fn solid_fill_version_13_uses_a_property_list() {
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
    fn vector_pattern_fills_tile_a_part_of_their_cmx_drawing() {
        use crate::cmx::tests as cmx;
        // A 100 mm square CMX page with a 20 x 10 mm rectangle at its
        // centre; the tile is the 40 mm square around the centre, drawn
        // 20 mm wide and 10 mm high.
        let drawing = cmx::file(&[cmx::page(&[
            cmx::begin_page(1_000_000, 1_000_000),
            cmx::begin_layer("L", None),
            cmx::rect(&cmx::attrs(1, 0), 0, 0, 200_000, 100_000, 0, 0),
            cmx::instr(14, &[]),
            cmx::instr(12, &[]),
            cmx::instr(10, &[]),
        ])]);
        let mut fild = u32s(&[5, 1300, 2 + 8 + 16 + 8 + 36]);
        fild.extend_from_slice(&u16s(&[10]));
        fild.extend_from_slice(&u32s(&[1600, 16]));
        fild.extend_from_slice(&i32s(&[-200_000, 200_000, 200_000, -200_000]));
        fild.extend_from_slice(&u32s(&[1300, 36, 200_000, 100_000, 0, 0]));
        fild.extend_from_slice(&i32s(&[-200_000, 200_000, 200_000, -200_000]));
        fild.extend_from_slice(&u32s(&[12]));
        let main = riff(b"CDRK", &chunk(b"fild", &fild));
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 18);
        ctx.vector_patterns.insert(12, &drawing);
        ctx.read_fill(tree.root.find(b"fild").unwrap());
        let Fill::Pattern(tracedraw_core::style::Pattern::Vector { shapes, tile }) =
            &ctx.fills[&5].fill
        else {
            panic!("{:?}", ctx.fills[&5].fill)
        };
        assert!((tile.width - 20.0).abs() < 1e-6 && (tile.height - 10.0).abs() < 1e-6);
        // The rectangle, half the tile's width and a quarter of its height
        // after the stretch, sits at the tile's centre.
        let b = shapes[0].bounds();
        assert!(
            (b.width() - 10.0).abs() < 1e-6 && (b.height() - 2.5).abs() < 1e-6,
            "{b:?}"
        );
        assert!(
            (b.center().x - 10.0).abs() < 1e-6 && (b.center().y - 5.0).abs() < 1e-6,
            "{b:?}"
        );
        // No drawing for the id: no fill, with a warning.
        let mut ctx = Ctx::new(&main, &tree, 18);
        ctx.read_fill(tree.root.find(b"fild").unwrap());
        assert_eq!(ctx.fills[&5].fill, Fill::None);
        assert!(ctx
            .report
            .warnings
            .iter()
            .any(|w| w.contains("vector pattern")));
    }

    #[test]
    fn fills_skip_the_leading_record_newer_writers_add() {
        // Newer writers put a record of their own (tag 2400, a 16-byte
        // id) before the usual 1300 record, also in files saved for older
        // versions.
        let mut props = vec![0x01u8];
        props.extend_from_slice(&12u32.to_le_bytes());
        props.extend_from_slice(&color_bgr(10, 20, 30));
        props.push(0x00);
        let mut solid = u32s(&[1300, props.len() as u32]);
        solid.extend_from_slice(&props);
        let mut fild = u32s(&[3, 2400, 16]);
        fild.extend_from_slice(&[0xab; 16]);
        fild.extend_from_slice(&u32s(&[1300, (solid.len() + 2) as u32]));
        fild.extend_from_slice(&u16s(&[1]));
        fild.extend_from_slice(&solid);
        let main = riff(b"CDRF", &chunk(b"fild", &fild));
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 15);
        ctx.read_fill(tree.root.find(b"fild").unwrap());
        assert_eq!(ctx.fills[&3].fill, Fill::Solid(Color::rgb8(10, 20, 30)));
        // Cut anywhere: no panic.
        for cut in 0..fild.len() {
            let main = riff(b"CDRF", &chunk(b"fild", &fild[..cut]));
            if let Ok(tree) = crate::riff::parse(&main) {
                let mut ctx = Ctx::new(&main, &tree, 15);
                if let Some(c) = tree.root.find(b"fild") {
                    ctx.read_fill(c);
                }
            }
        }
    }

    #[test]
    fn fountain_fill_version_12_reads_angle_and_all_stops() {
        // Version 12 (before version 13) layout.
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
                // The file lists a radial gradient from its edge: red is
                // the outside, blue the centre (ours run from the centre).
                assert_eq!(f.stops[0].color, Color::rgb8(0, 0, 255));
                assert_eq!(f.stops[2].color, Color::rgb8(255, 0, 0));
                assert!(f.stops[0].pos.abs() < 1e-9 && (f.stops[2].pos - 1.0).abs() < 1e-9);
                assert!((f.offset.x - 0.1).abs() < 1e-9 && (f.offset.y + 0.2).abs() < 1e-9);
                assert!((f.edge_pad - 0.05).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn fountain_fill_version_15_stop_size_is_45_bytes() {
        let v = 15u16;
        let mut g = vec![0u8; 8];
        g.push(1);
        g.extend_from_slice(&[0; 17]);
        g.extend_from_slice(&i16s(&[0])); // edge offset is s2 again from version 13
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
    fn outline_version_13_skips_tagged_records_and_reads_dashes() {
        let v = 13u16;
        let mut o = u32s(&[4]); // id
        o.extend_from_slice(&u32s(&[0x0a, 4, 0xdead_beef])); // record 10, skipped
        o.extend_from_slice(&u32s(&[1, 0])); // record 1: the outline
        o.extend_from_slice(&u16s(&[0x20, 1, 2])); // line type (scale), round cap, bevel join
        o.extend_from_slice(&i32s(&[2540])); // width 0.254 mm
        o.extend_from_slice(&u16s(&[50])); // stretch 50 %
        o.extend_from_slice(&i32s(&[30_000_000])); // nib angle 30 deg
        o.extend_from_slice(&[0; 48]); // the matrix
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

    /// A version 3 transformation list (argument 100) of one matrix:
    /// factors in 16.16, offsets in thousandths of an inch.
    fn v3_transform(m: [i32; 6]) -> Vec<u8> {
        let mut t = u16s(&[3, 1, 0x8c, 0, 0, 34, 0]);
        t.resize(34, 0);
        for v in m {
            t.extend_from_slice(&v.to_le_bytes());
        }
        t
    }

    /// A version 3 inline outline: flags, width, stretch, angle, colour
    /// (model 1, CMYK bytes), caps at 32, join at 33, arrowheads at 34, 38.
    fn v3_outline(on: bool, width: u16, cap: u8, start_arrow: u32) -> Vec<u8> {
        let mut o = vec![u8::from(on)];
        o.extend_from_slice(&u16s(&[width, 100, 0]));
        o.extend_from_slice(&[1, 0, 0, 0, 100]);
        o.resize(32, 0);
        o.extend_from_slice(&[cap, cap]);
        o.extend_from_slice(&start_arrow.to_le_bytes());
        o.extend_from_slice(&0u32.to_le_bytes());
        o
    }

    fn v3_file(objects: &[u8], extra: &[u8]) -> Vec<u8> {
        let mut cfg = u16s(&[0]);
        cfg.extend_from_slice(&i16s(&[-4250, 5500, 4250, -5500]));
        let body = [
            chunk(b"mcfg", &cfg),
            extra.to_vec(),
            list(b"page", &list(b"layr", objects)),
        ]
        .concat();
        riff(b"CDR ", &body)
    }

    #[test]
    fn version_3_objects_carry_their_matrix_fill_and_outline_inline() {
        // A curve (type 2 in version 3) from (0,0) to (1000,0), moved one
        // inch right and scaled twice, filled 100 % cyan, outlined 0.1 inch
        // with round caps; and a rectangle (type 0) in palette black at
        // 60 % (model 0: index 0, tint 60).
        let mut coords = u16s(&[2, 0]);
        coords.extend_from_slice(&i16s(&[0, 0, 1000, 0]));
        coords.extend_from_slice(&[0x00, 0x48]);
        let curve = loda16(
            2,
            &[
                (100, v3_transform([2 << 16, 0, 1000, 0, 2 << 16, 0])),
                (ARG_COORDS as u16, coords),
                (ARG_FILL as u16, vec![1, 1, 100, 0, 0, 0]),
                (ARG_OUTLINE as u16, v3_outline(true, 100, 1, 0)),
            ],
        );
        let rect = loda16(
            0,
            &[
                (100, v3_transform([1 << 16, 0, 0, 0, 1 << 16, 0])),
                (ARG_COORDS as u16, i16s(&[500, 500, 0])),
                (ARG_FILL as u16, vec![1, 0, 0, 0, 60, 0]),
                (ARG_OUTLINE as u16, v3_outline(false, 14, 0, 0)),
            ],
        );
        let objects = [
            list(b"obj ", &chunk(b"lobj", &curve)),
            list(b"obj ", &chunk(b"lobj", &rect)),
        ]
        .concat();
        let (doc, rep) = parse(&v3_file(&objects, &[]), 3);
        assert_eq!(rep.shapes, 2, "{:?}", rep.warnings);
        assert!((doc.pages[0].size.width - 215.9).abs() < 1e-9);
        assert!((doc.pages[0].size.height - 279.4).abs() < 1e-9);
        let shapes: Vec<&Shape> = doc.pages[0].layers.iter().flat_map(|l| &l.shapes).collect();
        let curve = shapes
            .iter()
            .find(|s| matches!(s.kind, ShapeKind::Path { .. }))
            .expect("curve");
        let b = curve.page_path().bounding_box();
        assert!((b.width() - 50.8).abs() < 1e-6, "{b:?}");
        // Coordinates run from the page's centre.
        assert!((b.x0 - (215.9 / 2.0 + 25.4)).abs() < 1e-6, "{b:?}");
        assert_eq!(
            curve.fill,
            Fill::Solid(Color::cmyk_pct(100.0, 0.0, 0.0, 0.0))
        );
        let st = curve.stroke.as_ref().expect("outline");
        assert!((st.width - 2.54).abs() < 1e-9);
        assert_eq!(st.cap, LineCap::Round);
        let rect = shapes
            .iter()
            .find(|s| matches!(s.kind, ShapeKind::Rect { .. }))
            .expect("rect");
        assert_eq!(rect.fill, Fill::Solid(Color::cmyk_pct(0.0, 0.0, 0.0, 60.0)));
        assert!(rect.stroke.is_none());
    }

    #[test]
    fn version_3_envelopes_bend_shapes_into_their_frame() {
        // A 1000 x 1000 square mapped from its box onto a frame whose top
        // side is raised to 1500: matrix, envelope, matrix.
        let mut env = u16s(&[1, 0]);
        env.extend_from_slice(&i16s(&[0, 1000, 1000, 0]));
        env.extend_from_slice(&u16s(&[0, 6, 12, 18, 25]));
        // Clockwise from the bottom-left: up the left side, along the top,
        // down the right side, back along the bottom; straight sides of two
        // segments each (node, handle, handle).
        let corner = [(0, 0), (0, 1000), (1000, 1500), (1000, 0)];
        let mut pts: Vec<i16> = Vec::new();
        for s in 0..4 {
            let (a, b) = (corner[s], corner[(s + 1) % 4]);
            for k in 0..6 {
                let t = k as f64 / 6.0;
                pts.push((a.0 as f64 + (b.0 - a.0) as f64 * t) as i16);
                pts.push((a.1 as f64 + (b.1 - a.1) as f64 * t) as i16);
            }
        }
        pts.extend_from_slice(&[0, 0]);
        env.extend_from_slice(&i16s(&pts));
        let identity = |t: &mut Vec<u8>| {
            for v in [1i32 << 16, 0, 0, 0, 1 << 16, 0] {
                t.extend_from_slice(&v.to_le_bytes());
            }
        };
        let mut steps = u16s(&[3, 3]);
        steps.extend_from_slice(&u16s(&[0x8c, 0, 0, 34, 0]));
        steps.extend_from_slice(&u16s(&[0x14, 0, 0, 58, 0]));
        steps.extend_from_slice(&u16s(&[0x8c, 0, 0, (58 + env.len()) as u16, 0]));
        steps.resize(34, 0);
        identity(&mut steps);
        steps.extend_from_slice(&env);
        identity(&mut steps);
        let rect = loda16(
            0,
            &[
                (100, steps),
                (ARG_COORDS as u16, i16s(&[1000, 1000, 0])),
                (ARG_FILL as u16, vec![1, 1, 100, 0, 0, 0]),
            ],
        );
        let (doc, rep) = parse(&v3_file(&list(b"obj ", &chunk(b"lobj", &rect)), &[]), 3);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        let s = first_shape(&doc);
        assert!(
            matches!(s.kind, ShapeKind::Path { closed: true, .. }),
            "{:?}",
            s.kind
        );
        let b = s.page_path().bounding_box();
        let cx = 215.9 / 2.0;
        assert!((b.width() - 25.4).abs() < 0.2, "{b:?}");
        assert!((b.height() - 38.1).abs() < 0.2, "{b:?}");
        assert!((b.x0 - cx).abs() < 0.2, "{b:?}");
    }

    #[test]
    fn version_3_pattern_and_stand_in_fills() {
        // A 16 x 16 1-bit tile with word-aligned rows (32 bytes), id 7; a
        // rectangle filled with it in red on white, one with a PostScript
        // texture.
        let mut tile = u32s(&[7, 40, 16, 16]);
        tile.extend_from_slice(&u16s(&[1, 1]));
        tile.extend_from_slice(&u32s(&[0, 32, 0, 0, 0, 0]));
        tile.extend_from_slice(&[0, 0, 0, 0, 255, 255, 255, 0]);
        tile.extend_from_slice(&[0xF0; 32]);
        let rect = |fill: Vec<u8>| {
            list(
                b"obj ",
                &chunk(
                    b"lobj",
                    &loda16(
                        0,
                        &[
                            (100, v3_transform([1 << 16, 0, 0, 0, 1 << 16, 0])),
                            (ARG_COORDS as u16, i16s(&[500, 500, 0])),
                            (ARG_FILL as u16, fill),
                        ],
                    ),
                ),
            )
        };
        let mut pattern = vec![7];
        pattern.extend_from_slice(&u32s(&[7]));
        pattern.extend_from_slice(&u16s(&[250, 250]));
        pattern.extend_from_slice(&[0; 7]);
        pattern.extend_from_slice(&[1, 0, 100, 100, 0, 1, 0, 0, 0, 0]);
        let mut ps = vec![6];
        ps.extend_from_slice(&[0xbf, 0xec, 4, 0, 10, 0, 100, 0, 100, 0, 10, 0]);
        let objects = [rect(pattern), rect(ps)].concat();
        let extra = list(b"bpft", &chunk(b"bmpf", &tile));
        let (doc, rep) = parse(&v3_file(&objects, &extra), 3);
        assert_eq!(rep.shapes, 2, "{:?}", rep.warnings);
        let fills: Vec<&Fill> = doc.pages[0]
            .layers
            .iter()
            .flat_map(|l| &l.shapes)
            .map(|s| &s.fill)
            .collect();
        assert!(fills.iter().any(|f| matches!(f, Fill::Pattern(Pattern::Bitmap { width_px: 16, size_mm, .. }) if (size_mm - 6.35).abs() < 1e-9)), "{fills:?}");
        assert!(fills.iter().any(|f| matches!(
            f,
            Fill::Pattern(Pattern::TwoColor {
                tile: PatternTile::DiagonalStripes,
                ..
            })
        )));
        assert!(
            !rep.warnings.iter().any(|w| w.contains("placeholder")),
            "{:?}",
            rep.warnings
        );
    }

    #[test]
    fn version_3_groups_place_their_objects_and_text_is_read() {
        // A group scaling its text twice; the text "Hi" in 0.5 inch (36 pt)
        // centred, in the file's only font.
        let mut t = vec![0u8; 132];
        t[11..13].copy_from_slice(&2u16.to_le_bytes());
        t[19..21].copy_from_slice(&500u16.to_le_bytes());
        t[21..23].copy_from_slice(&100u16.to_le_bytes());
        t[23..25].copy_from_slice(&120u16.to_le_bytes());
        t[25] = 2;
        t.extend_from_slice(&[0, b'H', 0, 0, b'i', 0]);
        let text = loda16(
            3,
            &[
                (100, v3_transform([1 << 16, 0, 0, 0, 1 << 16, 0])),
                (ARG_COORDS as u16, t),
            ],
        );
        let group = loda16(0, &[(100, v3_transform([2 << 16, 0, 0, 0, 2 << 16, 0]))]);
        let objects = list(
            b"grp ",
            &[
                chunk(b"lobj", &group),
                list(b"obj ", &chunk(b"lobj", &text)),
            ]
            .concat(),
        );
        let mut ftbl = u16s(&[1]);
        ftbl.extend_from_slice(&[b' '; 32]);
        ftbl.extend_from_slice(&u16s(&[1, 2]));
        ftbl.extend_from_slice(b"toronto\0");
        let (doc, rep) = parse(&v3_file(&objects, &chunk(b"ftbl", &ftbl)), 3);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        let s = first_shape(&doc);
        let ShapeKind::Text {
            spans, align, para, ..
        } = &s.kind
        else {
            panic!("{:?}", s.kind);
        };
        assert_eq!(spans[0].text, "Hi");
        assert_eq!(spans[0].font_family, "Times New Roman");
        assert!((spans[0].size_pt - 36.0).abs() < 1e-9);
        assert_eq!(*align, TextAlign::Center);
        assert!((para.leading_pct - 120.0).abs() < 1e-9);
        assert!((s.transform.as_coeffs()[0] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn version_3_fountains_and_palette_colours() {
        // A linear fountain at 90 degrees from white to black, and a
        // numbered palette colour that becomes a spot ink.
        let rect = |fill: Vec<u8>| {
            list(
                b"obj ",
                &chunk(
                    b"lobj",
                    &loda16(
                        0,
                        &[
                            (100, v3_transform([1 << 16, 0, 0, 0, 1 << 16, 0])),
                            (ARG_COORDS as u16, i16s(&[500, 500, 0])),
                            (ARG_FILL as u16, fill),
                        ],
                    ),
                ),
            )
        };
        let mut fountain = vec![2];
        fountain.extend_from_slice(&900i16.to_le_bytes());
        fountain.extend_from_slice(&[0, 1, 0, 0, 0]);
        fountain.extend_from_slice(&[0, 0, 0, 100, 0]);
        let objects = [rect(fountain), rect(vec![1, 0, 7, 0, 100, 0])].concat();
        let (doc, rep) = parse(&v3_file(&objects, &[]), 3);
        assert_eq!(rep.shapes, 2, "{:?}", rep.warnings);
        let fills: Vec<&Fill> = doc.pages[0]
            .layers
            .iter()
            .flat_map(|l| &l.shapes)
            .map(|s| &s.fill)
            .collect();
        assert!(fills.iter().any(|f| matches!(f, Fill::Fountain(g) if (g.angle - 90.0).abs() < 1e-9 && g.stops.len() == 2)));
        assert!(fills
            .iter()
            .any(|f| matches!(f, Fill::Solid(Color::Spot { .. }))));
        assert_eq!(doc.inks.len(), 1);
        assert_eq!(doc.inks[0].name, "Spot 7");
    }

    #[test]
    fn version_4_matrices_colours_and_glyph_curves() {
        // trfd with a 16.16 matrix (scale 2, offset one inch); a fill in
        // model 1 (CMYK as u16 percents); a curve whose count is followed by
        // the u16 1.
        let mut tr = u16s(&[0x2a, 1, 10, 0xffff, 0, 16, 0x2a, 0, 8]);
        for v in [2i32 << 16, 0, 1000, 0, 2 << 16, 0] {
            tr.extend_from_slice(&v.to_le_bytes());
        }
        let mut coords = u16s(&[2, 1]);
        coords.extend_from_slice(&i16s(&[0, 0, 1000, 0]));
        coords.extend_from_slice(&[0x00, 0x48]);
        let mut fill = u32s(&[1]);
        fill.extend_from_slice(&u16s(&[1, 0, 1, 0, 100, 0, 0, 0]));
        let obj = list(
            b"obj ",
            &[
                list(
                    b"lgob",
                    &chunk(
                        b"loda",
                        &loda16(
                            OBJ_CURVE as u16,
                            &[(ARG_COORDS as u16, coords), (ARG_FILL as u16, u32s(&[1]))],
                        ),
                    ),
                ),
                list(b"trfl", &chunk(b"trfd", &tr)),
            ]
            .concat(),
        );
        let body = [
            chunk(b"mcfg", &i16s(&[8500, 11000])),
            list(b"filt", &chunk(b"fill", &fill)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDR4", &body), 4);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        let s = first_shape(&doc);
        let b = s.page_path().bounding_box();
        assert!((b.width() - 50.8).abs() < 1e-6, "{b:?}");
        // Coordinates run from the page's centre.
        assert!((b.x0 - (215.9 / 2.0 + 25.4)).abs() < 1e-6, "{b:?}");
        assert_eq!(s.fill, Fill::Solid(Color::cmyk_pct(0.0, 100.0, 0.0, 0.0)));
    }

    #[test]
    fn several_trfd_matrices_apply_one_after_the_other() {
        // Version 5: a scale by 2, then a move by 1000 (thousandths of an
        // inch) to the right.
        let mut tr = u16s(&[116, 2, 10, 0xffff, 0, 16, 66, 116]);
        for m in [
            [2.0, 0.0, 0.0, 0.0, 2.0, 0.0],
            [1.0, 0.0, 1000.0, 0.0, 1.0, 0.0],
        ] {
            tr.extend_from_slice(&u16s(&[8]));
            for v in m {
                tr.extend_from_slice(&f64::to_le_bytes(v));
            }
        }
        let args = [(ARG_COORDS as u16, i16s(&[1000, 500, 0]))];
        let obj = list(
            b"obj ",
            &[
                chunk(b"loda", &loda16(OBJ_RECT as u16, &args)),
                chunk(b"trfd", &tr),
            ]
            .concat(),
        );
        let body = [
            chunk(b"mcfg", &i16s(&[8500, 11000])),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDR5", &body), 5);
        assert!(
            !rep.warnings.iter().any(|w| w.contains("several")),
            "{:?}",
            rep.warnings
        );
        let b = first_shape(&doc).page_path().bounding_box();
        assert!((b.width() - 50.8).abs() < 1e-6, "{b:?}");
        assert!((b.x0 - (215.9 / 2.0 + 25.4)).abs() < 1e-6, "{b:?}");
    }

    #[test]
    fn version_5_bitmaps_span_from_the_origin_to_their_corner() {
        // A 2 x 1 pixel 24-bit picture, id 2, and an object whose corner
        // is (1000, -500) thousandths of an inch.
        let mut bmp = u16s(&[2]);
        bmp.extend_from_slice(&[0; 14]);
        bmp.extend_from_slice(&u32s(&[5, 1, 2, 1, 1, 24, 8, 8]));
        bmp.extend_from_slice(&[0; 32]);
        bmp.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        let mut c = i16s(&[1000, -500]);
        c.extend_from_slice(&u16s(&[2]));
        c.extend_from_slice(&[0; 40]);
        let args = [(ARG_COORDS as u16, c)];
        let obj = list(b"obj ", &chunk(b"loda", &loda16(OBJ_BITMAP as u16, &args)));
        let body = [
            chunk(b"mcfg", &i16s(&[8500, 11000])),
            list(b"bmpt", &chunk(b"bmp ", &bmp)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDR5", &body), 5);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Bitmap { rect, width_px, .. } => {
                assert_eq!(*width_px, 2);
                assert!((rect.width() - 25.4).abs() < 1e-9 && (rect.height() - 12.7).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn rectangle_version_15_doubles_and_scale() {
        // 1 x 0.5 inch with corners of 0.4 (radii are shares of half the
        // shorter side, here 6.35 mm, as version 17 and 22 files show: so
        // 2.54 mm); the scale is the transform's, so the rectangle keeps
        // its own size.
        let mut c = f64s(&[254000.0, 127000.0]);
        c.extend_from_slice(&f64s(&[2.0, 2.0])); // scale
        c.push(0); // scale_with
        c.extend_from_slice(&[0; 7]);
        for (r, gap) in [(0.4f64, 16usize), (0.4, 16), (0.4, 16), (0.4, 0)] {
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
            ShapeKind::Rect { rect, radius, .. } => {
                assert!((rect.width() - 25.4).abs() < 1e-9);
                assert!((rect.height() - 12.7).abs() < 1e-9);
                assert!((radius - 2.54).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn rectangle_radii_per_corner() {
        // Width, height, then four radii from the corner at (width, 0)
        // counter-clockwise: 1, 2, 3 and 4 mm.
        let read = |w: i32| {
            let c = i32s(&[w, 127000, 10000, 20000, 30000, 40000]);
            let obj = list(
                b"obj ",
                &chunk(b"loda", &loda(OBJ_RECT, &[(ARG_COORDS, c)])),
            );
            let body = [
                chunk(b"mcfg", &mcfg(9, 2_540_000, 2_540_000)),
                list(b"page", &list(b"layr", &obj)),
            ]
            .concat();
            let (doc, rep) = parse(&riff(b"CDR9", &body), 9);
            assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
            assert!(rep.warnings.is_empty(), "{:?}", rep.warnings);
            first_shape(&doc)
                .kind
                .rect_corners()
                .expect("a rectangle")
                .1
        };
        let near = |a: [f64; 4], b: [f64; 4]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
        // Top left, top right, bottom left, bottom right.
        let c = read(254000);
        assert!(near(c.radii, [3.0, 2.0, 4.0, 1.0]), "{c:?}");
        // Drawn leftwards: the first corner is on the left.
        let c = read(-254000);
        assert!(near(c.radii, [2.0, 3.0, 1.0, 4.0]), "{c:?}");
        // One radius for every corner: the same on every corner, keeping
        // its size when the rectangle is resized (#619).
        let c = i32s(&[254000, 127000, 5000, 5000, 5000, 5000]);
        let obj = list(
            b"obj ",
            &chunk(b"loda", &loda(OBJ_RECT, &[(ARG_COORDS, c)])),
        );
        let body = [
            chunk(b"mcfg", &mcfg(9, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, _) = parse(&riff(b"CDR9", &body), 9);
        assert!(matches!(
            first_shape(&doc).kind,
            ShapeKind::Rect { corners: Some(c), radius, .. }
                if (radius - 0.5).abs() < 1e-9 && c.fixed && c.is_uniform()
        ));
    }

    /// Rounded rectangles open with corners that keep their size when
    /// resized, at the size they measure on the page; a rectangle the
    /// file stretches gets round corners, not oval ones (#619).
    #[test]
    fn rounded_rectangles_keep_their_corner_size() {
        // 25.4 x 12.7 mm with 1 mm corners, scaled 2 x 1 by its transform.
        let c = i32s(&[254000, 127000, 10000, 10000, 10000, 10000]);
        let obj = list(
            b"obj ",
            &[
                chunk(b"loda", &loda(OBJ_RECT, &[(ARG_COORDS, c)])),
                chunk(b"trfd", &trfd(9, [2.0, 0.0, 0.0, 0.0, 1.0, 0.0])),
            ]
            .concat(),
        );
        let body = [
            chunk(b"mcfg", &mcfg(9, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, _) = parse(&riff(b"CDR9", &body), 9);
        let mut s = first_shape(&doc).clone();
        let page = s.page_corners().expect("a rectangle");
        assert!(page.fixed, "{page:?}");
        assert!(
            page.radii.iter().all(|r| (r - 1.0).abs() < 1e-9),
            "{page:?}"
        );
        // Resized, the corners stay 1 mm.
        s.transform = s.transform * Affine::scale_non_uniform(1.0, 3.0);
        let page = s.page_corners().expect("a rectangle");
        assert!(
            page.radii.iter().all(|r| (r - 1.0).abs() < 1e-9),
            "{page:?}"
        );
        // Square corners stay square and plain.
        let c = i32s(&[254000, 127000, 0, 0, 0, 0]);
        let obj = list(
            b"obj ",
            &chunk(b"loda", &loda(OBJ_RECT, &[(ARG_COORDS, c)])),
        );
        let body = [
            chunk(b"mcfg", &mcfg(9, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, _) = parse(&riff(b"CDR9", &body), 9);
        assert!(matches!(
            first_shape(&doc).kind,
            ShapeKind::Rect { corners: None, radius, .. } if radius == 0.0
        ));
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
    fn facing_page_objects_move_to_the_side_holding_most_of_them() {
        let rect = |x: f64| {
            let mut s = Shape::new(
                tracedraw_core::ShapeId(1),
                ShapeKind::Rect {
                    rect: Rect::new(x, 10.0, x + 20.0, 30.0),
                    radius: 0.0,
                    corners: None,
                },
            );
            s.transform = Affine::IDENTITY;
            s
        };
        let centre_x = |l: &[Layer]| {
            let b = l[0].shapes[0].bounds();
            (b.x0 + b.x1) / 2.0
        };
        // Measured from the spine with the page to its right: half a page
        // too far right when placed from the page centre.
        let mut layers = vec![Layer::new(tracedraw_core::LayerId(1), "L")];
        layers[0].shapes = vec![rect(150.0), rect(250.0), rect(280.0)];
        shift_onto_page(&mut layers, 210.0);
        assert!((centre_x(&layers) - 55.0).abs() < 1e-9);
        // To its left: half a page too far left.
        layers[0].shapes = vec![rect(-100.0), rect(-60.0), rect(150.0)];
        shift_onto_page(&mut layers, 210.0);
        assert!((centre_x(&layers) - 15.0).abs() < 1e-9);
    }

    #[test]
    fn arcs_of_ellipses_drawn_downwards_turn_the_other_way() {
        // An open arc from 0 to 180 degrees on a 20 mm circle stored with a
        // negative height, then with a negative width too.
        let read = |w: i32, h: i32| {
            let mut c = i32s(&[w, h]);
            c.extend_from_slice(&i32s(&[0, 180_000_000]));
            c.extend_from_slice(&u32s(&[0]));
            let obj = list(
                b"obj ",
                &chunk(b"loda", &loda(OBJ_ELLIPSE, &[(ARG_COORDS, c)])),
            );
            let body = [
                chunk(b"mcfg", &mcfg(20, 2_540_000, 2_540_000)),
                list(b"page", &list(b"layr", &obj)),
            ]
            .concat();
            let (doc, rep) = parse(&riff(b"CDRL", &body), 20);
            assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
            let s = first_shape(&doc);
            let ShapeKind::Ellipse { rect, .. } = &s.kind else {
                panic!("{:?}", s.kind);
            };
            // Where the arc lies against the ellipse's centre (y up).
            let local = s.kind.clone();
            let centre = rect.center();
            let b = Shape::new(s.id, local).local_path().bounding_box();
            (b.center().y - centre.y, b.height())
        };
        // Upright: the upper half.
        let (dy, h) = read(200_000, 200_000);
        assert!(dy > 4.0 && (h - 10.0).abs() < 0.1, "{dy} {h}");
        // Drawn downwards: the lower half (a smile).
        let (dy, h) = read(200_000, -200_000);
        assert!(dy < -4.0 && (h - 10.0).abs() < 0.1, "{dy} {h}");
        let (dy, _) = read(-200_000, -200_000);
        assert!(dy < -4.0, "{dy}");
    }

    #[test]
    fn polygon_uses_the_polygon_argument() {
        // One segment of a base hexagon of radius 1000 mm: the top vertex,
        // the middle of a side (or a star's inner point), the next vertex.
        let read = |inner_mm: f64| {
            let a = 60f64.to_radians();
            let mut coords = u32s(&[3]);
            coords.extend_from_slice(&i32s(&[
                0,
                10_000_000,
                (inner_mm * 10_000.0 * a.cos()).round() as i32,
                (inner_mm * 10_000.0 * a.sin()).round() as i32,
                8_660_254,
                5_000_000,
            ]));
            coords.extend_from_slice(&[0x00, 0x01, 0x01]);
            // version 13 layout: num angles, next point, unknown, 4 unknown,
            // the scale of the base shape, the centre.
            let mut poly = u32s(&[6, 2, 0, 0]);
            poly.extend_from_slice(&f64s(&[0.0254, 0.0127]));
            poly.extend_from_slice(&i32s(&[100_000, 0]));
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
            first_shape(&doc).kind.clone()
        };
        // A plain hexagon: 25.4 by 12.7 mm radii around (10 mm, 0).
        match read(1000.0 * 30f64.to_radians().cos()) {
            ShapeKind::Polygon {
                rect,
                points,
                sharpness,
                ..
            } => {
                assert_eq!(points, 6);
                assert!((rect.width() - 50.8).abs() < 1e-9, "{rect:?}");
                assert!((rect.height() - 25.4).abs() < 1e-9);
                assert!((rect.center().x - 10.0).abs() < 1e-9);
                assert_eq!(sharpness, 0.0);
            }
            other => panic!("{other:?}"),
        }
        // An inner point at half the radius: a star.
        match read(500.0) {
            ShapeKind::Polygon { sharpness, .. } => {
                assert!((sharpness - 0.5).abs() < 1e-6, "{sharpness}")
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
    fn cropped_bitmap_object_becomes_a_clip_frame_and_layer_order_is_reversed() {
        // Same bitmap as the test below, placed in a 25.4 mm square.
        let mut bmp = u32s(&[42]);
        bmp.extend_from_slice(&[0; 50]);
        bmp.extend_from_slice(&u32s(&[5, 0, 2, 2, 0, 24, 0, 16]));
        bmp.extend_from_slice(&[0; 32]);
        bmp.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        bmp.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0]);
        let mut c = i32s(&[0, 0, 254_000, 254_000]);
        c.extend_from_slice(&[0; 32]);
        c.extend_from_slice(&u32s(&[42]));
        c.extend_from_slice(&[0; 20]);
        // Crop outline: the lower-left quarter (4 points, move + 3 lines, closed).
        c.extend_from_slice(&u32s(&[4]));
        c.extend_from_slice(&i32s(&[0, 0, 127_000, 0, 127_000, 127_000, 0, 127_000]));
        c.extend_from_slice(&[0x00, 0x40, 0x40, 0x48]);
        let cropped = list(
            b"obj ",
            &chunk(b"loda", &loda(OBJ_BITMAP, &[(ARG_COORDS, c)])),
        );
        // A plain rectangle stored after the bitmap in the file.
        let rc = i32s(&[2_540_000, 2_540_000]);
        let rect = list(
            b"obj ",
            &chunk(b"loda", &loda(OBJ_RECT, &[(ARG_COORDS, rc)])),
        );
        let body = [
            chunk(b"mcfg", &mcfg(13, 2_540_000, 2_540_000)),
            chunk(b"bmp ", &bmp),
            list(b"page", &list(b"layr", &[cropped, rect].concat())),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRD", &body), 13);
        assert_eq!(rep.shapes, 2, "{:?}", rep.warnings);
        let shapes = &doc.pages[0].layers[0].shapes;
        // File order is front to back: the rectangle (last in the file) is drawn first.
        assert!(
            matches!(shapes[0].kind, ShapeKind::Rect { .. }),
            "{:?}",
            shapes[0].kind
        );
        match &shapes[1].kind {
            ShapeKind::ClipFrame { frame, contents } => {
                let fb = frame.bounds();
                assert!((fb.width() - 12.7).abs() < 1e-6 && (fb.height() - 12.7).abs() < 1e-6);
                assert_eq!(contents.len(), 1);
                assert!(matches!(contents[0].kind, ShapeKind::Bitmap { .. }));
                let cb = contents[0].bounds();
                assert!((cb.width() - 25.4).abs() < 1e-6);
            }
            other => panic!("expected a ClipFrame, got {other:?}"),
        }
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
                fx: _,
            } => {
                assert_eq!((*width_px, *height_px), (2, 2));
                assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
                assert_eq!(&png[12..16], b"IHDR");
                assert!((rect.width() - 25.4).abs() < 1e-9);
                // Decode the IDAT and check the top-left pixel is blue.
                let raw = raw_rows(png);
                assert_eq!(&raw[1..5], &[0, 0, 255, 255]);
                assert_eq!(&raw[10..14], &[255, 0, 0, 255]);
            }
            other => panic!("{other:?}"),
        }
    }

    /// Objects naming one bitmap record share one copy of its picture.
    #[test]
    fn objects_naming_one_bitmap_share_its_picture() {
        let mut bmp = u32s(&[42]);
        bmp.extend_from_slice(&[0; 50]);
        bmp.extend_from_slice(&u32s(&[5, 0, 2, 2, 0, 24, 0, 16]));
        bmp.extend_from_slice(&[0; 32]);
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
            list(
                b"page",
                &list(b"layr", &[obj.clone(), obj.clone(), obj].concat()),
            ),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRD", &body), 13);
        assert_eq!(rep.shapes, 3, "{:?}", rep.warnings);
        let pictures: Vec<&tracedraw_core::SharedBytes> = doc.pages[0]
            .layers
            .iter()
            .flat_map(|l| &l.shapes)
            .filter_map(|s| match &s.kind {
                ShapeKind::Bitmap { png, .. } => Some(png),
                _ => None,
            })
            .collect();
        assert_eq!(pictures.len(), 3);
        assert!(pictures
            .iter()
            .all(|p| tracedraw_core::SharedBytes::ptr_eq(p, pictures[0])));
    }

    #[test]
    fn bitmap_masks_make_pixels_clear() {
        // 2 x 2, 24 bpp, then a mask block: "RI", its length, 4 zero bytes,
        // a 78-byte header (model 99, 1, width, height, 1 plane, 8 bits,
        // stride, size) and one byte per pixel, bottom row first.
        let mut bmp = u32s(&[42]);
        bmp.extend_from_slice(&[0; 50]);
        bmp.extend_from_slice(&u32s(&[1, 0, 2, 2, 0, 24, 0, 16]));
        bmp.extend_from_slice(&[0; 32]);
        bmp.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        bmp.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0]);
        let mask_at = bmp.len();
        bmp[24..28].copy_from_slice(&((mask_at - 8) as u32).to_le_bytes());
        let mut mask = b"RI".to_vec();
        mask.extend_from_slice(&u32s(&[78 + 8, 0, 78, 99, 1, 2, 2, 1, 8, 4, 8]));
        mask.resize(78, 0);
        mask.extend_from_slice(&[255, 0, 0, 0]); // bottom row: opaque, clear
        mask.extend_from_slice(&[0, 128, 0, 0]); // top row: clear, half
        bmp.extend_from_slice(&mask);
        let alpha = |bmp: &[u8]| {
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
                chunk(b"bmp ", bmp),
                list(b"page", &list(b"layr", &obj)),
            ]
            .concat();
            let (doc, rep) = parse(&riff(b"CDRD", &body), 13);
            assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
            let ShapeKind::Bitmap { png, .. } = &first_shape(&doc).kind else {
                panic!("a bitmap");
            };
            let raw = raw_rows(png);
            // Rows of a filter byte and two RGBA pixels, top row first.
            [raw[4], raw[8], raw[13], raw[17]]
        };
        assert_eq!(alpha(&bmp), [0, 128, 255, 0]);
        // A mask that does not match the image is ignored.
        let mut odd = bmp.clone();
        odd[mask_at + 22] = 3; // width 3
        assert_eq!(alpha(&odd), [255; 4]);
        let mut odd = bmp.clone();
        odd[mask_at] = b'X';
        assert_eq!(alpha(&odd), [255; 4]);
        let mut short = bmp.clone();
        short.truncate(short.len() - 3);
        assert_eq!(alpha(&short), [255; 4]);
    }

    #[test]
    fn cmyk_bitmaps_are_read_as_ink() {
        // 2 x 1, colour model 3, 32 bpp: C, M, Y, K bytes, 0 for no ink.
        let mut bmp = u32s(&[42]);
        bmp.extend_from_slice(&[0; 50]);
        bmp.extend_from_slice(&u32s(&[BITMAP_CMYK, 0, 2, 1, 0, 32, 0, 8]));
        bmp.extend_from_slice(&[0; 32]);
        bmp.extend_from_slice(&[255, 0, 0, 0, 0, 0, 0, 128]); // cyan, half black
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
        let ShapeKind::Bitmap { png, .. } = &first_shape(&doc).kind else {
            panic!("a bitmap");
        };
        let raw = raw_rows(png);
        // Without a profile: one minus the ink, times one minus the black.
        assert_eq!(&raw[1..5], &[0, 255, 255, 255]);
        assert_eq!(&raw[5..9], &[127, 127, 127, 255]);
        // Through a table, the table decides.
        let lut =
            tracedraw_core::icc::CmykLut::with_points(3, |[c, _, _, k]| Some([1.0 - c, 0.5, k]))
                .unwrap();
        assert_eq!(cmyk_pixel([255, 0, 0, 255], Some(&lut)), [0, 128, 255, 255]);
        // A profile that is not CMYK is not used.
        let tree = crate::riff::parse(&riff(b"CDRD", &body)).unwrap();
        let (doc, _) =
            parse_document_with(&tree, &riff(b"CDRD", &body), Version(13), Some(b"junk"));
        assert!(matches!(first_shape(&doc).kind, ShapeKind::Bitmap { .. }));
    }

    #[test]
    fn pattern_fill_version_13_reads_both_colours() {
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

    /// Before version 13: solid fill (`fild`: id, type 1, 2 unknown, colour).
    fn solid_fild(id: u32, rgb: (u8, u8, u8)) -> Vec<u8> {
        let mut fild = u32s(&[id]);
        fild.extend_from_slice(&u16s(&[1, 0]));
        fild.extend_from_slice(&color_bgr(rgb.0, rgb.1, rgb.2));
        fild
    }

    /// version 13+ solid fill with a property list holding one colour.
    fn solid_fild_v13(id: u32, rgb: (u8, u8, u8)) -> Vec<u8> {
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

    /// One style record of the 7 to version 15 `txsm` layout.
    struct Style7 {
        chars: u16,
        font: Option<u16>,
        flags: Option<u32>,
        size_units: Option<i32>,
        fill: Option<u32>,
    }

    fn txsm7(v: u16, frame: bool, styles: &[Style7], text: &str) -> Vec<u8> {
        txsm7_opt(v, frame, styles, text, false)
    }

    fn txsm7_opt(v: u16, frame: bool, styles: &[Style7], text: &str, on_path: bool) -> Vec<u8> {
        let mut t = u32s(&[frame as u32]);
        t.extend_from_slice(&[0; 32]);
        if v >= 15 {
            t.push(0);
        }
        t.extend_from_slice(&u32s(&[1])); // frames
        t.extend_from_slice(&u32s(&[1])); // frame id
        t.extend_from_slice(&[0; 48]);
        if on_path {
            t.extend_from_slice(&u32s(&[1]));
            let mut n = 4 + 28;
            if v >= 13 {
                n += 8;
            }
            if v >= 15 {
                n += 8;
            }
            t.extend_from_slice(&vec![0u8; n]);
        } else {
            t.extend_from_slice(&u32s(&[0])); // not on a path
            if v >= 15 {
                t.extend_from_slice(&[0; 8]);
            }
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
                // Before version 13 the coordinates argument holds record
                // ids; the text starts at the origin.
                assert!(origin.x.abs() < 1e-9 && origin.y.abs() < 1e-9, "{origin:?}");
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
    fn group_lists_become_groups_and_text_on_path_follows_the_curve() {
        let styles = [Style7 {
            chars: 5,
            font: None,
            flags: None,
            size_units: Some(127_000),
            fill: None,
        }];
        // A curve from (0,0) to (50.8,0) mm and a text flagged "on path",
        // together in a group; a plain rectangle outside the group.
        let mut coords = u32s(&[2]);
        coords.extend_from_slice(&i32s(&[0, 0, 508_000, 0]));
        coords.extend_from_slice(&[0x08, 0x40]);
        let curve = list(
            b"obj ",
            &chunk(b"loda", &loda(OBJ_CURVE, &[(ARG_COORDS, coords)])),
        );
        let text = list(
            b"obj ",
            &[
                chunk(
                    b"loda",
                    &loda(OBJ_ARTISTIC_TEXT, &[(ARG_COORDS, i32s(&[0, 0]))]),
                ),
                chunk(b"txsm", &txsm7_opt(12, false, &styles, "Hello", true)),
            ]
            .concat(),
        );
        // Front to back in the file: text, curve.
        let grp = list(b"grp ", &[text, curve].concat());
        let rect = list(
            b"obj ",
            &chunk(
                b"loda",
                &loda(OBJ_RECT, &[(ARG_COORDS, i32s(&[254_000, 254_000, 0]))]),
            ),
        );
        let body = [
            chunk(b"mcfg", &mcfg(12, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &[grp, rect].concat())),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRC", &body), 12);
        let layer = &doc.pages[0].layers[0];
        // File order front to back, ours bottom to top: rect, then group.
        assert_eq!(layer.shapes.len(), 2, "{:?}", rep.warnings);
        assert!(matches!(layer.shapes[0].kind, ShapeKind::Rect { .. }));
        let ShapeKind::Group { children } = &layer.shapes[1].kind else {
            panic!("{:?}", layer.shapes[1].kind);
        };
        assert_eq!(children.len(), 2);
        assert!(matches!(children[0].kind, ShapeKind::Path { .. }));
        match &children[1].kind {
            ShapeKind::Text { on_path, spans, .. } => {
                assert_eq!(spans[0].text, "Hello");
                let p = on_path.as_ref().expect("text on path");
                let b = p.path.bounding_box();
                assert!((b.width() - 50.8).abs() < 1e-6, "{b:?}");
                assert!(children[1].data.is_empty());
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(rep.shapes, 2);
    }

    /// A version 21 artistic text record fitted to a path: the frame's
    /// path settings (placement 4, `offset` and `distance` in file units,
    /// both mirror flags or none) and 24 bytes of path data per letter.
    fn txsm21_on_path(text: &str, offset: i32, distance: i32, mirror: bool) -> Vec<u8> {
        let mut t = u32s(&[0]); // no frame flag (artistic text)
        t.extend_from_slice(&[0; 32]);
        t.extend_from_slice(&u16s(&[2100]));
        t.extend_from_slice(&[0; 3]);
        t.extend_from_slice(&u32s(&[1, 1])); // one frame, id 1
        t.extend_from_slice(&[0; 48]);
        t.extend_from_slice(&u32s(&[1])); // on a path
        let m = i32::from(mirror);
        t.extend_from_slice(&i32s(&[0, 4, 0, 16082, offset, 0, distance, 1, m, m]));
        t.extend_from_slice(&[0; 8]);
        t.extend_from_slice(&[0; 16]);
        t.extend_from_slice(&u32s(&[0]));
        t.extend_from_slice(&u32s(&[1])); // one paragraph
        t.extend_from_slice(&u32s(&[0]));
        t.push(0);
        t.extend_from_slice(&style_string(21, r#"{"character":{"font":5,"size":24}}"#));
        t.extend_from_slice(&u32s(&[0])); // no style records
        let n = text.len() as u32;
        t.extend_from_slice(&u32s(&[n]));
        for _ in 0..n {
            t.extend_from_slice(&u16s(&[0]));
            t.push(0xfe);
            t.extend_from_slice(&[0; 5]);
        }
        t.extend_from_slice(&u32s(&[n]));
        t.extend_from_slice(text.as_bytes());
        t.push(1); // path data per letter
        t.extend_from_slice(&vec![0; 24 * text.len()]);
        t
    }

    /// Blends between ellipses in a version `v` file, one linked group of
    /// object type 7 with the given first u32 of its 3000 argument each:
    /// the step count of each live blend read, and the report.
    fn read_blends(v: u16, counts: &[u32]) -> (Vec<u32>, ParseReport) {
        let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let ellipse = |x: i32| {
            let mut c = i32s(&[x, -100_000, x + 200_000, 100_000]);
            c.extend_from_slice(&u32s(&[0]));
            list(
                b"obj ",
                &[
                    chunk(b"loda", &loda(OBJ_ELLIPSE, &[(ARG_COORDS, c)])),
                    chunk(b"trfd", &trfd(v, identity)),
                ]
                .concat(),
            )
        };
        let group = |steps: u32| {
            let mut a = u32s(&[steps, 254_000]);
            a.extend_from_slice(&[0; 80]);
            list(
                b"lnkg",
                &list(
                    b"lgob",
                    &chunk(b"loda", &loda(7, &[(12020, u32s(&[2, 0, 0])), (3000, a)])),
                ),
            )
        };
        let mut objects = Vec::new();
        for &n in counts {
            objects.extend(ellipse(0));
            objects.extend(group(n));
            objects.extend(ellipse(600_000));
        }
        let body = [
            chunk(b"mcfg", &mcfg(v, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &objects)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRD", &body), v);
        // The layer lists its objects from the top, the file from the
        // bottom: in file order.
        let steps = doc.pages[0].layers[0]
            .shapes
            .iter()
            .rev()
            .flat_map(|s| &s.effects)
            .filter_map(|e| match e {
                tracedraw_core::live::Effect::Blend { steps, .. } => Some(*steps),
                _ => None,
            })
            .collect();
        (steps, rep)
    }

    /// A step count field holding no count a blend can have (0, or the
    /// 254000 that follows it in the sample files, as another layout
    /// would put there) gives the default 20 steps and a warning naming
    /// the value; it was kept to 999, so such files drew every blend with
    /// 999 steps (#980). Counts of 1 to 999 are kept as read.
    #[test]
    fn a_blend_step_count_out_of_range_gives_the_default() {
        for v in [9u16, 13, 15] {
            let (steps, rep) = read_blends(v, &[254_000, 0, u32::MAX, 1, 999, 35]);
            assert_eq!(steps, [20, 20, 20, 1, 999, 35], "version {v}");
            assert!(
                rep.warnings.iter().any(|w| w.contains("254000")),
                "{:?}",
                rep.warnings
            );
            let (_, rep) = read_blends(v, &[20, 400]);
            assert!(
                !rep.warnings.iter().any(|w| w.contains("blend")),
                "{:?}",
                rep.warnings
            );
        }
    }

    /// The blends of one file take at most `FILE_BLEND_STEPS` steps
    /// together; the blend that crosses it gets what is left, later ones
    /// one step, with a warning (#980).
    #[test]
    fn the_blends_of_a_file_share_a_bound_on_steps() {
        let per = 999u32;
        let full = (FILE_BLEND_STEPS / per) as usize;
        let mut counts = vec![per; full + 2];
        counts.push(5);
        let (steps, rep) = read_blends(13, &counts);
        assert_eq!(steps.len(), counts.len());
        assert!(steps[..full].iter().all(|n| *n == per));
        assert_eq!(steps[full], FILE_BLEND_STEPS - full as u32 * per);
        assert_eq!(&steps[full + 1..], [1, 1]);
        assert!(
            rep.warnings.iter().any(|w| w.contains("fewer")),
            "{:?}",
            rep.warnings
        );
        let (steps, rep) = read_blends(13, &[20; 50]);
        assert_eq!(steps, [20; 50]);
        assert!(rep.warnings.is_empty(), "{:?}", rep.warnings);
    }

    /// A blend as versions 9 and 13 store it: the two end objects with an
    /// empty linked group between them, whose `loda` of object type 7
    /// holds the step count in its argument of type 3000. The first end
    /// gets a live blend to the second, which stays an object of its own;
    /// a linked group of another type (a contour's, 15) makes none (#597).
    #[test]
    fn an_empty_blend_group_links_its_two_end_objects() {
        let v = 13u16;
        let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let ellipse = |x: i32| {
            let mut c = i32s(&[x, -100_000, x + 200_000, 100_000]);
            c.extend_from_slice(&u32s(&[0]));
            list(
                b"obj ",
                &[
                    chunk(b"loda", &loda(OBJ_ELLIPSE, &[(ARG_COORDS, c)])),
                    chunk(b"trfd", &trfd(v, identity)),
                ]
                .concat(),
            )
        };
        let group = |kind: u32, steps: u32| {
            let mut a = u32s(&[steps, 254_000]);
            a.extend_from_slice(&[0; 80]);
            list(
                b"lnkg",
                &list(
                    b"lgob",
                    &chunk(
                        b"loda",
                        &loda(kind, &[(12020, u32s(&[2, 0, 0])), (3000, a)]),
                    ),
                ),
            )
        };
        let read = |kind: u32| {
            let body = [
                chunk(b"mcfg", &mcfg(v, 2_540_000, 2_540_000)),
                list(
                    b"page",
                    &list(
                        b"layr",
                        &[ellipse(0), group(kind, 12), ellipse(600_000)].concat(),
                    ),
                ),
            ]
            .concat();
            parse(&riff(b"CDRD", &body), v)
        };
        let (doc, rep) = read(7);
        assert_eq!(rep.shapes, 2, "{:?}", rep.warnings);
        let shapes = &doc.pages[0].layers[0].shapes;
        let blends: Vec<(u32, Option<tracedraw_core::ShapeId>)> = shapes
            .iter()
            .flat_map(|s| &s.effects)
            .filter_map(|e| match e {
                tracedraw_core::live::Effect::Blend { steps, end_id, .. } => {
                    Some((*steps, *end_id))
                }
                _ => None,
            })
            .collect();
        assert_eq!(blends.len(), 1, "{shapes:?}");
        assert_eq!(blends[0].0, 12);
        let other = shapes
            .iter()
            .find(|s| s.effects.is_empty())
            .expect("the other end");
        assert_eq!(blends[0].1, Some(other.id));
        // The steps lie below the start (the upper object) and pair the
        // two outlines from their first nodes (#597).
        let options = shapes
            .iter()
            .flat_map(|s| &s.effects)
            .find_map(|e| match e {
                tracedraw_core::live::Effect::Blend { options, .. } => Some(*options),
                _ => None,
            })
            .expect("blend");
        assert!(options.steps_below);
        let nodes = options.map_nodes.expect("nodes");
        assert!((nodes.start[0] - nodes.start[1]).hypot() > 1e-6);
        assert!((nodes.end[0] - nodes.end[1]).hypot() > 1e-6);
        // A blend group with no object before it: a warning, no blend.
        let body = [
            chunk(b"mcfg", &mcfg(v, 2_540_000, 2_540_000)),
            list(
                b"page",
                &list(b"layr", &[group(7, 12), ellipse(600_000)].concat()),
            ),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRD", &body), v);
        assert!(doc.pages[0].layers[0].shapes[0].effects.is_empty());
        assert!(
            rep.warnings.iter().any(|w| w.contains("blend")),
            "{:?}",
            rep.warnings
        );
        // A contour's linked group (type 15): no blend.
        let (doc, _) = read(15);
        assert!(doc.pages[0].layers[0]
            .shapes
            .iter()
            .all(|s| s.effects.is_empty()));
    }

    /// A blend starts a rectangle at its corner at the origin, along its
    /// height first, whichever way the file drew it; other objects at
    /// their first node, toward the next (#597).
    #[test]
    fn blend_nodes_start_at_the_file_s_first_node() {
        let rect = |x0, y0, x1, y1| {
            Shape::new(
                tracedraw_core::ShapeId(1),
                ShapeKind::Rect {
                    rect: Rect::new(x0, y0, x1, y1),
                    radius: 0.0,
                    corners: None,
                },
            )
        };
        // Width and height negative: the origin is the upper right corner.
        let n = blend_nodes(&rect(-73.0, -67.0, 0.0, 0.0)).expect("nodes");
        assert_eq!(n, [Point::new(0.0, 0.0), Point::new(0.0, -67.0)]);
        // Height positive.
        let n = blend_nodes(&rect(-81.0, 0.0, 0.0, 75.0)).expect("nodes");
        assert_eq!(n, [Point::new(0.0, 0.0), Point::new(0.0, 75.0)]);
        // A polygon: its top vertex, then the next one clockwise.
        let poly = Shape::new(
            tracedraw_core::ShapeId(2),
            ShapeKind::Polygon {
                rect: Rect::new(-10.0, -10.0, 10.0, 10.0),
                points: 4,
                sharpness: 0.0,
                complex: None,
                twist: 0.0,
            },
        );
        let [a, b] = blend_nodes(&poly).expect("nodes");
        assert!((a - Point::new(0.0, 10.0)).hypot() < 1e-9, "{a:?}");
        assert!((b - Point::new(10.0, 0.0)).hypot() < 1e-9, "{b:?}");
        // A flat rectangle has no way to run.
        assert!(blend_nodes(&rect(-5.0, 0.0, 0.0, 0.0)).is_none());
    }

    /// Version 21 texts fitted to a circle they are not grouped with (as
    /// in a version 21 file: two texts on one circle, the upper one
    /// inside it, the lower one mirrored). The circle is found from the
    /// texts' boxes; a larger circle around it is not taken.
    #[test]
    fn a_linked_group_draws_its_objects_through_its_own_transform() {
        // As a version 10 file stores an artistic brush's strokes: a
        // `LIST lnkg` whose `lgob` holds the transform and whose object's
        // own is the identity. Placed at the top level with the group's
        // transform, the same object lands in the same place.
        let v = 13u16;
        let ellipse = |m: [f64; 6]| {
            let mut c = i32s(&[200_000, -100_000, 0, 0]);
            c.extend_from_slice(&u32s(&[0]));
            list(
                b"obj ",
                &[
                    chunk(b"loda", &loda(OBJ_ELLIPSE, &[(ARG_COORDS, c)])),
                    chunk(b"trfd", &trfd(v, m)),
                ]
                .concat(),
            )
        };
        let group_m = [0.5, 0.0, 150_000.0, 0.0, 0.5, -20_000.0];
        let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let linked = list(
            b"lnkg",
            &[
                list(
                    b"lgob",
                    &[
                        chunk(b"loda", &loda(OBJ_ELLIPSE, &[])),
                        list(b"trfl", &chunk(b"trfd", &trfd(v, group_m))),
                    ]
                    .concat(),
                ),
                ellipse(identity),
            ]
            .concat(),
        );
        let body = [
            chunk(b"mcfg", &mcfg(v, 2_540_000, 2_540_000)),
            list(
                b"page",
                &list(b"layr", &[linked, ellipse(group_m)].concat()),
            ),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRD", &body), v);
        assert_eq!(rep.shapes, 2, "{:?}", rep.warnings);
        let shapes = &doc.pages[0].layers[0].shapes;
        let (a, b) = (
            shapes[0].transform.as_coeffs(),
            shapes[1].transform.as_coeffs(),
        );
        for (x, y) in a.iter().zip(b) {
            assert!((x - y).abs() < 1e-9, "{a:?} {b:?}");
        }
        assert!(
            (a[0] - 0.5).abs() < 1e-12,
            "the group's scale applies: {a:?}"
        );
    }

    #[test]
    fn texts_fitted_to_a_circle_sit_where_the_file_sets_them() {
        let v = 21u16;
        let text_obj = |t: Vec<u8>, bbox: [i32; 4]| {
            list(
                b"obj ",
                &[
                    chunk(b"bbox", &i32s(&bbox)),
                    chunk(
                        b"loda",
                        &loda(OBJ_ARTISTIC_TEXT, &[(ARG_COORDS, i32s(&[0, 0]))]),
                    ),
                    // Unfitted, the text would sit off the page.
                    chunk(b"trfd", &trfd(v, [1.0, 0.0, -3_000_000.0, 0.0, 1.0, 0.0])),
                    chunk(b"txsm", &t),
                ]
                .concat(),
            )
        };
        // A circle of radius `r` units about the page centre (the file's
        // origin), drawn from (0, 0) downwards as files store them.
        let circle = |r: i32| {
            let mut c = i32s(&[2 * r, -2 * r, 0, 0]);
            c.extend_from_slice(&u32s(&[0]));
            list(
                b"obj ",
                &[
                    chunk(b"loda", &loda(OBJ_ELLIPSE, &[(ARG_COORDS, c)])),
                    chunk(
                        b"trfd",
                        &trfd(v, [1.0, 0.0, -f64::from(r), 0.0, 1.0, f64::from(r)]),
                    ),
                ]
                .concat(),
            )
        };
        // Radius 50 mm, baselines 10 mm inside: 40 mm, 251.3 mm round.
        let quarter = (std::f64::consts::PI * 40.0 / 2.0 * 10_000.0).round() as i32;
        // Its middle a quarter turn clockwise from the top: the right side.
        let right = text_obj(
            txsm21_on_path("Right", quarter, -100_000, false),
            [330_000, -170_000, 480_000, 180_000],
        );
        // Mirrored, half a turn from the top: the bottom, reading upright.
        let bottom = text_obj(
            txsm21_on_path("Bottom", 2 * quarter, -100_000, true),
            [-170_000, -470_000, 180_000, -270_000],
        );
        // Its box is nowhere near either circle's baseline.
        let lost = text_obj(
            txsm21_on_path("Lost", 0, 0, false),
            [-1_200_000, -1_200_000, -1_100_000, -1_100_000],
        );
        let body = [
            chunk(b"mcfg", &mcfg(v, 2_540_000, 2_540_000)),
            chunk(b"font", &font_chunk(5, "Georgia")),
            list(
                b"page",
                &list(
                    b"layr",
                    &[right, bottom, lost, circle(500_000), circle(800_000)].concat(),
                ),
            ),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRL", &body), v);
        assert_eq!(rep.shapes, 5, "{:?}", rep.warnings);
        let text = |s: &str| {
            doc.pages[0].layers[0]
                .shapes
                .iter()
                .find(|sh| match &sh.kind {
                    ShapeKind::Text { spans, .. } => spans[0].text == s,
                    _ => false,
                })
                .cloned()
                .expect("text")
        };
        // The point the file's offset names, on the page.
        let anchor = |sh: &Shape| {
            let ShapeKind::Text {
                on_path: Some(tp),
                origin,
                align,
                ..
            } = &sh.kind
            else {
                panic!("not on a path: {:?}", sh.kind);
            };
            assert_eq!(*align, TextAlign::Center);
            let base = tp.baseline();
            assert!((base.length() - 2.0 * std::f64::consts::PI * 40.0).abs() < 0.5);
            let (p, dir) = base.at(tp.offset + base.length() / 2.0).expect("point");
            let page = sh.transform * Affine::translate(origin.to_vec2());
            (page * p, page * (p + dir) - page * p, tp.clone())
        };
        // Page centre (127, 127).
        let (p, dir, tp) = anchor(&text("Right"));
        assert!((p - Point::new(167.0, 127.0)).hypot() < 0.5, "{p:?}");
        assert!(
            dir.y < -0.99,
            "runs clockwise, down the right side: {dir:?}"
        );
        assert!(!tp.mirror);
        assert!((tp.distance + 10.0).abs() < 1e-9);
        let (p, dir, tp) = anchor(&text("Bottom"));
        assert!((p - Point::new(127.0, 87.0)).hypot() < 0.5, "{p:?}");
        assert!(dir.x > 0.99, "reads left to right: {dir:?}");
        assert!(tp.mirror);
        // Mirrored, the letters' tops point into the circle: the
        // baseline is 10 mm to the left of the path followed backwards.
        assert!((tp.distance - 10.0).abs() < 1e-9);
        let lost = text("Lost");
        assert!(matches!(lost.kind, ShapeKind::Text { on_path: None, .. }));
        assert!(
            rep.warnings
                .iter()
                .any(|w| w.contains("path was not found")),
            "{:?}",
            rep.warnings
        );
    }

    /// Lists nested in a layer keep the file's front-to-back order.
    #[test]
    fn objects_of_nested_lists_keep_their_stacking_order() {
        let rect = |w: i32| {
            list(
                b"obj ",
                &chunk(b"loda", &loda(OBJ_RECT, &[(ARG_COORDS, i32s(&[w, w]))])),
            )
        };
        // Front to back: 10 mm, then (nested) 20 and 30 mm, then 40 mm.
        let nested = list(b"spnd", &[rect(200_000), rect(300_000)].concat());
        let body = [
            chunk(b"mcfg", &mcfg(13, 2_540_000, 2_540_000)),
            list(
                b"page",
                &list(b"layr", &[rect(100_000), nested, rect(400_000)].concat()),
            ),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRD", &body), 13);
        let widths: Vec<i64> = doc.pages[0].layers[0]
            .shapes
            .iter()
            .map(|s| s.bounds().width().round() as i64)
            .collect();
        assert_eq!(widths, [40, 30, 20, 10], "{:?}", rep.warnings);
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

    /// `font` chunk payload before version 12: the character set in the
    /// encoding field, the name as single bytes.
    fn font_chunk_8bit(id: u16, charset: u16, name: &[u8]) -> Vec<u8> {
        let mut f = u16s(&[id, charset]);
        f.extend_from_slice(&u32s(&[0]));
        f.extend_from_slice(&[0; 10]);
        f.extend_from_slice(name);
        f.push(0);
        f
    }

    #[test]
    fn old_text_and_names_take_the_code_page_of_their_fonts() {
        // Six Cyrillic letters in font 3 (Cyrillic set), then " caf" and
        // e acute in font 4 (Latin set).
        let styles = [
            Style7 {
                chars: 6,
                font: Some(3),
                flags: None,
                size_units: None,
                fill: None,
            },
            Style7 {
                chars: 5,
                font: Some(4),
                flags: None,
                size_units: None,
                fill: None,
            },
        ];
        let mut t = txsm7(9, false, &styles, "");
        let cut = t.len() - 1 - 4;
        t.truncate(cut);
        let bytes = [
            0xcf, 0xf0, 0xe8, 0xe2, 0xe5, 0xf2, b' ', b'c', b'a', b'f', 0xe9,
        ];
        t.extend_from_slice(&u32s(&[bytes.len() as u32]));
        t.extend_from_slice(&vec![0; bytes.len() * 4]);
        t.extend_from_slice(&bytes);
        t.push(0);
        let tables = [
            // A font name in the Cyrillic code page too.
            chunk(
                b"font",
                &font_chunk_8bit(3, 204, &[0xc0, 0xf0, 0xe8, 0xe0, 0xeb]),
            ),
            chunk(b"font", &font_chunk_8bit(4, 0, b"Times")),
        ]
        .concat();
        let obj = list(
            b"obj ",
            &[
                chunk(
                    b"loda",
                    &loda(OBJ_ARTISTIC_TEXT, &[(ARG_COORDS, i32s(&[0, 0]))]),
                ),
                chunk(b"txsm", &t),
            ]
            .concat(),
        );
        // The layer's name, single-byte: "Layer" in Russian.
        let mut name = vec![0xd1, 0xeb, 0xee, 0xe9];
        name.push(0);
        let layer = list(
            b"layr",
            &[chunk(b"loda", &loda(0, &[(ARG_NAME, name)])), obj].concat(),
        );
        let body = [
            chunk(b"mcfg", &mcfg(9, 2_540_000, 2_540_000)),
            tables,
            list(b"page", &layer),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDR9", &body), 9);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        assert_eq!(doc.pages[0].layers[0].name, "\u{421}\u{43b}\u{43e}\u{439}");
        match &first_shape(&doc).kind {
            ShapeKind::Text { spans, .. } => {
                assert_eq!(spans.len(), 2, "{spans:?}");
                assert_eq!(spans[0].text, "\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}");
                assert_eq!(spans[0].font_family, "\u{410}\u{440}\u{438}\u{430}\u{43b}");
                assert_eq!(spans[1].text, " caf\u{e9}");
                assert_eq!(spans[1].font_family, "Times");
            }
            other => panic!("{other:?}"),
        }
    }

    /// version 16+ style string: u32 length in UTF-16 units, then the text.
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
    fn paragraph_text_version_16_layout_with_style_strings() {
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
            r#"{"character":{"font":5,"size":10},"paragraph":{"justify":"2"}}"#,
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
            chunk(b"fild", &solid_fild_v13(9, (0, 128, 0))),
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

    /// A run set in a semibold face (style code 1024, weight 600) reads
    /// as that face, named by the family and its weight, not as bold;
    /// regular (64) and bold (4096) runs keep the family's name (#624).
    #[test]
    fn semibold_runs_keep_their_face() {
        let v = 21u16;
        let read = |family: &str, weight: &str| {
            let mut t = u32s(&[0xffff_ffff]);
            t.extend_from_slice(&[0; 32]);
            t.extend_from_slice(&u16s(&[1700]));
            t.extend_from_slice(&[0; 3]);
            t.extend_from_slice(&u32s(&[1, 1]));
            t.extend_from_slice(&[0; 48]);
            t.extend_from_slice(&u32s(&[0]));
            t.extend_from_slice(&[0; 8]);
            t.extend_from_slice(&u32s(&[1]));
            t.extend_from_slice(&u32s(&[0]));
            t.push(0);
            t.push(0);
            t.extend_from_slice(&style_string(
                v,
                &format!(
                    r#"{{"character":{{"latin":{{"font":"{family}","size":"128819","weight":"{weight}","italic":"0"}}}}}}"#
                ),
            ));
            t.extend_from_slice(&u32s(&[0]));
            t.extend_from_slice(&u32s(&[2]));
            for _ in 0..2 {
                t.extend_from_slice(&u16s(&[0]));
                t.push(0xfe);
                t.extend_from_slice(&[0; 5]);
            }
            t.extend_from_slice(&u32s(&[2]));
            t.extend_from_slice(b"Hi");
            t.push(0);
            let obj = list(
                b"obj ",
                &[
                    chunk(b"bbox", &i32s(&[0, 100_000, 100_000, 0])),
                    chunk(
                        b"loda",
                        &loda(OBJ_ARTISTIC_TEXT, &[(ARG_COORDS, i32s(&[0, 8]))]),
                    ),
                    chunk(b"trfd", &trfd(v, [1.0, 0.0, 0.0, 0.0, 1.0, 0.0])),
                    chunk(b"txsm", &t),
                ]
                .concat(),
            );
            let body = [
                chunk(b"mcfg", &mcfg(v, 2_540_000, 2_540_000)),
                list(b"page", &list(b"layr", &obj)),
            ]
            .concat();
            let (doc, _) = parse(&riff(b"CDRL", &body), v);
            match &first_shape(&doc).kind {
                ShapeKind::Text { spans, .. } => (spans[0].font_family.clone(), spans[0].bold),
                other => panic!("{other:?}"),
            }
        };
        assert_eq!(
            read("Titillium Web", "1024"),
            ("Titillium Web SemiBold".to_string(), false)
        );
        assert_eq!(
            read("Titillium Web", "4096"),
            ("Titillium Web".to_string(), true)
        );
        assert_eq!(
            read("Titillium Web", "64"),
            ("Titillium Web".to_string(), false)
        );
        // Light (16) and black (65536) faces too.
        assert_eq!(read("Lato", "16").0, "Lato Light");
        assert_eq!(read("Lato", "65536"), ("Lato Black".to_string(), false));
        // A family already named with its weight keeps its name.
        assert_eq!(
            read("Segoe UI Semibold", "1024"),
            ("Segoe UI Semibold".to_string(), false),
            "the name gives the weight: that face, not bold"
        );
        assert_eq!(
            read("Arial Black", "64"),
            ("Arial Black".to_string(), false)
        );
        assert_eq!(read("Lato Hairline", "1").0, "Lato Hairline");
    }

    /// Version 21 paragraph text: the loda has no frame size; the frame
    /// comes from the object's box taken back through its transform (a
    /// horizontal scale of 0.75 here) and the width of the columns in the
    /// object's style string; the run's colour from its inline fill.
    #[test]
    fn version_21_text_keeps_its_baseline_unless_it_is_paragraph_text() {
        let v = 21u16;
        let mut t = u32s(&[0xffff_ffff]); // frame layout
        t.extend_from_slice(&[0; 32]);
        t.extend_from_slice(&u16s(&[1700]));
        t.extend_from_slice(&[0; 3]);
        t.extend_from_slice(&u32s(&[1, 1])); // one frame, id 1
        t.extend_from_slice(&[0; 48]);
        t.extend_from_slice(&u32s(&[0])); // not on a path
        t.extend_from_slice(&[0; 8]);
        t.extend_from_slice(&u32s(&[1])); // one paragraph
        t.extend_from_slice(&u32s(&[0]));
        t.push(0);
        t.push(0);
        t.extend_from_slice(&style_string(
            v,
            r#"{"character":{"fill":{"type":"1","primaryColor":"CMYK255,USER,0,255,255,0,100,u"},"outline":{"behindFill":"0"},"latin":{"font":"Georgia","size":"128819","weight":"64"}},"frame":{},"paragraph":{"thinOverlineThickness":"45455"}}"#,
        ));
        t.extend_from_slice(&u32s(&[0])); // no style records
        t.extend_from_slice(&u32s(&[2]));
        for _ in 0..2 {
            t.extend_from_slice(&u16s(&[0]));
            t.push(0xfe);
            t.extend_from_slice(&[0; 5]);
        }
        t.extend_from_slice(&u32s(&[2]));
        t.extend_from_slice(b"Hi");
        t.push(0);
        // The object's style string: frame columns 60 mm wide.
        let json = r#"{"fill":{"type":"1"},"frame":{"columns":"600000,0"}}"#;
        let mut style = u32s(&[1, json.len() as u32]);
        style.extend_from_slice(json.as_bytes());
        // On the page: the box at x 10..55 (60 mm times 0.75) and y
        // 20..32, the transform's origin at (11, 22).
        let bbox = i32s(&[100_000, 320_000, 550_000, 200_000]);
        let read = |kind: u32, coords: Vec<u8>| {
            let obj = list(
                b"obj ",
                &[
                    chunk(b"bbox", &bbox),
                    chunk(
                        b"loda",
                        &loda(
                            kind,
                            &[(ARG_STYLE_STRING, style.clone()), (ARG_COORDS, coords)],
                        ),
                    ),
                    chunk(
                        b"trfd",
                        &trfd(v, [0.75, 0.0, 110_000.0, 0.0, 1.0, 220_000.0]),
                    ),
                    chunk(b"txsm", &t),
                ]
                .concat(),
            );
            let body = [
                chunk(b"mcfg", &mcfg(v, 2_540_000, 2_540_000)),
                list(b"page", &list(b"layr", &obj)),
            ]
            .concat();
            let (doc, rep) = parse(&riff(b"CDRL", &body), v);
            assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
            assert!(
                !rep.warnings
                    .iter()
                    .any(|w| w.contains("frame") || w.contains("fill")),
                "{:?}",
                rep.warnings
            );
            first_shape(&doc).clone()
        };
        // Artistic text (type 0x04): the first baseline starts at the
        // origin of the coordinates (as in the file: 0 and 8 units); the
        // box only bounds the letters.
        match read(OBJ_ARTISTIC_TEXT, i32s(&[0, 8])).kind {
            ShapeKind::Text {
                spans,
                frame,
                origin,
                ..
            } => {
                assert_eq!(frame, None);
                assert!(origin.x.abs() < 1e-6 && origin.y.abs() < 1e-3, "{origin:?}");
                assert_eq!(spans[0].text, "Hi");
                assert_eq!(spans[0].font_family, "Georgia");
                assert_eq!(
                    spans[0].fill,
                    Some(Fill::Solid(Color::cmyk_pct(0.0, 100.0, 100.0, 0.0)))
                );
            }
            other => panic!("{other:?}"),
        }
        // Paragraph text (type 0x06) without a size in its coordinates:
        // the frame is the box, as wide as the style's columns (60 mm)
        // times the transform's scale (0.75), and the text is not scaled.
        match read(OBJ_PARAGRAPH_TEXT, vec![0u8; 12]).kind {
            ShapeKind::Text { frame, origin, .. } => {
                let f = frame.expect("frame");
                assert!((f.width - 45.0).abs() < 1e-6, "{f:?}");
                assert!((f.height - 12.0).abs() < 1e-6, "{f:?}");
                // 10 - 11 and 20 - 22 from the transform's origin.
                assert!((origin.x - (-1.0)).abs() < 1e-6, "{origin:?}");
                assert!((origin.y - (-2.0)).abs() < 1e-6, "{origin:?}");
            }
            other => panic!("{other:?}"),
        }
        // A frame size in the coordinates (60 x 10 mm): the frame hangs
        // from (0, 0), the transform's scale goes into the frame and the
        // object keeps only its place.
        let s = read(OBJ_PARAGRAPH_TEXT, i32s(&[0, 600_000, 100_000]));
        match &s.kind {
            ShapeKind::Text { frame, origin, .. } => {
                let f = frame.expect("frame");
                assert!(
                    (f.width - 45.0).abs() < 1e-6 && (f.height - 10.0).abs() < 1e-6,
                    "{f:?}"
                );
                assert!(
                    origin.x.abs() < 1e-6 && (origin.y + 10.0).abs() < 1e-6,
                    "{origin:?}"
                );
            }
            other => panic!("{other:?}"),
        }
        let [a, b, c, d, ..] = s.transform.as_coeffs();
        assert!(
            (a - 1.0).abs() < 1e-9 && b.abs() < 1e-9 && c.abs() < 1e-9 && (d - 1.0).abs() < 1e-9
        );
    }

    #[test]
    fn frames_keep_their_scale_only_when_they_slant() {
        let size = Size::new(10.0, 4.0);
        let o = Point::new(0.0, -4.0);
        let (t, o2, s2) =
            unscaled_frame(Affine::new([2.0, 0.0, 0.0, 0.5, 3.0, 4.0]), o, size).expect("scaled");
        assert_eq!(t, Affine::translate((3.0, 4.0)));
        assert_eq!((o2, s2), (Point::new(0.0, -2.0), Size::new(20.0, 2.0)));
        // A turned and scaled frame keeps its turn.
        let turn = Affine::rotate(0.5) * Affine::scale(2.0);
        let (t, _, s2) = unscaled_frame(turn, o, size).expect("turned");
        assert!((t.determinant() - 1.0).abs() < 1e-9);
        assert!((s2.width - 20.0).abs() < 1e-9);
        // Slanted, unscaled or degenerate: left alone.
        assert!(unscaled_frame(Affine::skew(0.3, 0.0), o, size).is_none());
        assert!(unscaled_frame(Affine::translate((5.0, 5.0)), o, size).is_none());
        assert!(unscaled_frame(Affine::scale(0.0), o, size).is_none());
        assert!(
            unscaled_frame(Affine::new([f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0]), o, size).is_none()
        );
    }

    /// The text of a version 17+ artistic text whose characters are
    /// `chars`: (flags of the character's record, its bytes).
    fn text_of_v17_chars(v: u16, chars: &[(u16, &[u8])]) -> (String, Vec<String>) {
        let mut t = u32s(&[0]); // no frame flag (artistic text)
        t.extend_from_slice(&[0; 32]);
        t.extend_from_slice(&u16s(&[1800]));
        t.extend_from_slice(&[0; 3]);
        t.extend_from_slice(&u32s(&[1, 1])); // one frame, id 1
        t.extend_from_slice(&[0; 48]);
        t.extend_from_slice(&u32s(&[0])); // not on a path
        t.extend_from_slice(&[0; 8]);
        // skip record (frame flag is 0): 16 bytes + length + bytes
        t.extend_from_slice(&[0; 16]);
        t.extend_from_slice(&u32s(&[0]));
        t.extend_from_slice(&u32s(&[1])); // one paragraph
        t.extend_from_slice(&u32s(&[0]));
        t.push(0);
        t.extend_from_slice(&style_string(v, r#"{"character":{"font":5,"size":12}}"#));
        t.extend_from_slice(&u32s(&[0])); // no style records
        t.extend_from_slice(&u32s(&[chars.len() as u32]));
        for (flags, _) in chars {
            t.extend_from_slice(&u16s(&[*flags]));
            t.push(0xfe);
            t.extend_from_slice(&[0; 5]);
        }
        let bytes: Vec<u8> = chars.iter().flat_map(|(_, b)| b.iter().copied()).collect();
        t.extend_from_slice(&u32s(&[bytes.len() as u32]));
        t.extend_from_slice(&bytes);
        t.push(0);
        let tables = chunk(b"font", &font_chunk(5, "Georgia"));
        let coords = i32s(&[0, 0]);
        let (doc, rep) = text_doc(
            v,
            b"CDRM",
            tables,
            OBJ_ARTISTIC_TEXT,
            &[(ARG_COORDS, coords)],
            t,
        );
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Text { spans, .. } => (
                spans.iter().map(|s| s.text.as_str()).collect(),
                rep.warnings.clone(),
            ),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn text_from_version_17_on_is_stored_one_byte_per_character() {
        // Confirmed with a version 21 file: six letters as 6 bytes for 6 characters.
        let chars: Vec<(u16, &[u8])> = b"PANELS"
            .iter()
            .map(|b| (0, std::slice::from_ref(b)))
            .collect();
        assert_eq!(text_of_v17_chars(21, &chars).0, "PANELS");
    }

    #[test]
    fn letters_beyond_latin_1_take_two_bytes() {
        // As in a version 22 file: "Průmyslová" with "ů" stored as the
        // UTF-16 unit 6F 01 (its record's flag bit 0 set), "á" as one
        // byte E1; "č" (0D 01) is a letter, not a line break.
        let mut chars: Vec<(u16, &[u8])> =
            b"Pr".iter().map(|b| (0, std::slice::from_ref(b))).collect();
        chars.push((1, &[0x6f, 0x01]));
        chars.extend(b"myslov".iter().map(|b| (0u16, std::slice::from_ref(b))));
        chars.push((0, &[0xe1]));
        chars.push((0x41, &[0x0d, 0x01])); // other flag bits set as well
        let (text, warnings) = text_of_v17_chars(22, &chars);
        assert_eq!(text, "Průmyslováč");
        assert!(warnings.is_empty(), "{warnings:?}");
        // Widths that do not add up: one byte per character, with a note.
        let (text, warnings) = text_of_v17_chars(22, &[(1, b"A"), (0, b"B")]);
        assert_eq!(text, "AB");
        assert!(warnings
            .iter()
            .any(|w| w.contains("one byte per character")));
    }

    #[test]
    fn style_table_alignment_2_centres_and_spacing_can_be_a_distance() {
        // A version 12 style: alignment code 2 (centred), character
        // spacing -10 % (a negative i32) and line spacing given as a
        // distance (unit 1): 36 pt text with baselines 1.5 line heights
        // apart.
        let v = 12u16;
        let mut st = u32s(&[1]);
        st.extend_from_slice(&u32s(&[0]));
        st.extend_from_slice(&u32s(&[0]));
        st.extend_from_slice(&u32s(&[1, 100]));
        st.extend_from_slice(&[0; 20]);
        st.extend_from_slice(&u16s(&[3, 0]));
        st.extend_from_slice(&[0; 8]);
        st.extend_from_slice(&i32s(&[127_000])); // 36 pt
        st.extend_from_slice(&[0; 8]);
        st.extend_from_slice(&u32s(&[0]));
        st.extend_from_slice(&[0; 8]);
        st.extend_from_slice(&u32s(&[1, 200, 0, 2])); // centred
        let line = (127_000.0 * 1.15 * 1.5_f64).round() as i32;
        st.extend_from_slice(&u32s(&[1, 300, 0, 0]));
        st.extend_from_slice(&i32s(&[-100_000, 0, 0, line]));
        st.extend_from_slice(&[0; 20]);
        st.extend_from_slice(&u32s(&[1])); // unit: a distance
        for _ in 0..2 {
            st.extend_from_slice(&u32s(&[0]));
        }
        st.extend_from_slice(&u32s(&[0]));
        st.extend_from_slice(&u32s(&[0]));
        for _ in 0..3 {
            st.extend_from_slice(&u32s(&[0]));
        }
        st.extend_from_slice(&u32s(&[3, 77, 0, 0, 0, 3]));
        st.extend_from_slice(&utf16("Std"));
        st.extend_from_slice(&u32s(&[0, 0]));
        st.extend_from_slice(&u32s(&[100, 200, 300, 0, 0]));
        st.extend_from_slice(&u32s(&[0, 0, 0, 0, 0]));
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
            b"CDRC",
            tables,
            OBJ_ARTISTIC_TEXT,
            &[(ARG_COORDS, i32s(&[0, 0])), (ARG_STYLE, u32s(&[77]))],
            txsm7(v, false, &styles, "Hi"),
        );
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Text {
                spans, align, para, ..
            } => {
                assert_eq!(*align, TextAlign::Center);
                assert!((spans[0].tracking_pct + 10.0 * SPACE_EM).abs() < 1e-9);
                assert!((para.leading_pct - 150.0).abs() < 1e-6, "{para:?}");
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
        st.extend_from_slice(&u32s(&[1, 200, 0, 3])); // one align entry: right
                                                      // One interval entry, id 300: char spacing 5 %, line spacing 150 %.
        st.extend_from_slice(&u32s(&[1, 300, 0, 0, 50_000, 0, 0, 1_500_000]));
        st.extend_from_slice(&[0; 24]);
        for _ in 0..2 {
            st.extend_from_slice(&u32s(&[0])); // set5, tabs
        }
        st.extend_from_slice(&u32s(&[0])); // bullets
                                           // One indent entry, id 400: right 36 pt, first 18 pt, left 72 pt.
        st.extend_from_slice(&u32s(&[1, 400, 0, 0, 0]));
        st.extend_from_slice(&i32s(&[127_000, 63_500, 254_000]));
        for _ in 0..3 {
            st.extend_from_slice(&u32s(&[0])); // hyphens, drop caps, set11
        }
        // Record: num 3, style 77, parent 0, 8 unknown, name "Std".
        st.extend_from_slice(&u32s(&[3, 77, 0, 0, 0, 3]));
        st.extend_from_slice(&utf16("Std"));
        st.extend_from_slice(&u32s(&[0, 0])); // fill, outline refs
        st.extend_from_slice(&u32s(&[100, 200, 300, 0, 0])); // font, align, interval, set5, set11
        st.extend_from_slice(&u32s(&[0, 0, 400, 0, 0])); // tab, bullet, indent, hyphen, drop cap
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
            ShapeKind::Text {
                spans, align, para, ..
            } => {
                assert_eq!(*align, TextAlign::Right);
                assert_eq!(spans[0].font_family, "Verdana");
                assert!((spans[0].size_pt - 36.0).abs() < 1e-9);
                assert!(spans[0].italic && !spans[0].bold);
                assert!((spans[0].tracking_pct - 5.0 * SPACE_EM).abs() < 1e-9);
                assert!((para.leading_pct - 150.0).abs() < 1e-9);
                assert!((para.left_indent - 25.4).abs() < 1e-9, "{para:?}");
                assert!((para.first_line_indent - 6.35).abs() < 1e-9);
                assert!((para.right_indent - 12.7).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn shapes_without_an_outline_take_their_styles() {
        // A style table whose one graphic style (55) has outline 1.
        let v = 13u16;
        let mut st = u32s(&[1]); // one record
        st.extend_from_slice(&u32s(&[0])); // fills
        st.extend_from_slice(&u32s(&[1, 900, 0, 1])); // outline ref 900 -> outline 1
        for _ in 0..3 {
            st.extend_from_slice(&u32s(&[0])); // fonts, aligns, intervals
        }
        for _ in 0..3 {
            st.extend_from_slice(&u32s(&[0])); // set5, tabs, bullets
        }
        for _ in 0..4 {
            st.extend_from_slice(&u32s(&[0])); // indents, hyphens, drop caps, set11
        }
        st.extend_from_slice(&u32s(&[1, 55, 0, 0, 0, 0])); // num 1, style 55, no name
        st.extend_from_slice(&u32s(&[0, 900])); // fill and outline refs
        let rect = |args: &[(u32, Vec<u8>)]| {
            let mut a = vec![(ARG_COORDS, i32s(&[254000, 127000, 0, 0, 0, 0]))];
            a.extend_from_slice(args);
            list(b"obj ", &chunk(b"loda", &loda(OBJ_RECT, &a)))
        };
        let body = [
            chunk(b"mcfg", &mcfg(v, 2_540_000, 2_540_000)),
            chunk(b"outl", &outl_v13(1, 0, 0)),
            list(b"stlt", &st),
            list(
                b"page",
                &list(
                    b"layr",
                    &[
                        // Front to back: an explicit unknown outline, then
                        // the style's.
                        rect(&[(ARG_STYLE, u32s(&[55])), (ARG_OUTLINE, u32s(&[404]))]),
                        rect(&[(ARG_STYLE, u32s(&[55]))]),
                    ]
                    .concat(),
                ),
            ),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRD", &body), v);
        assert_eq!(rep.shapes, 2, "{:?}", rep.warnings);
        let shapes = &doc.pages[0].layers[0].shapes;
        let s = shapes[0].stroke.as_ref().expect("the style's outline");
        assert!((s.width - 0.254).abs() < 1e-9, "{s:?}");
        assert!(shapes[1].stroke.is_none());
    }

    /// A version 14 framed `txsm` as a seal's ring of letters is stored
    /// (#955): the paragraph's extra byte set to 1 and 64 bytes after it,
    /// then one style per letter turning and shifting it.
    fn txsm14_ring(letters: &[(char, i32, i32, i32)]) -> Vec<u8> {
        let mut t = u32s(&[1]); // frame flag
        t.extend_from_slice(&[0; 32]);
        t.extend_from_slice(&u32s(&[1, 4])); // one frame, id 4
        t.extend_from_slice(&[0; 48]);
        t.extend_from_slice(&u32s(&[0])); // not on a path
        t.extend_from_slice(&u32s(&[1, 0x1a])); // one paragraph, style id
        t.push(0);
        t.push(1); // 64 bytes follow
        let mut block = u32s(&[4, u32::MAX]);
        for m in [0.77f64, 0.0, 1000.0, 0.0, 1.0, 2000.0] {
            block.extend_from_slice(&m.to_le_bytes());
        }
        block.extend_from_slice(&[0; 8]);
        assert_eq!(block.len(), 64);
        t.extend_from_slice(&block);
        t.extend_from_slice(&u32s(&[letters.len() as u32]));
        for &(_, angle, dx, dy) in letters {
            t.extend_from_slice(&u16s(&[1]));
            t.push(0x3c); // size, angle, shifts
            t.push(0);
            t.extend_from_slice(&i32s(&[63_500, angle, dx, dy]));
        }
        t.extend_from_slice(&u32s(&[letters.len() as u32]));
        for i in 0..letters.len() {
            // Two bytes a letter; its style numbered in the third byte.
            t.extend_from_slice(&[1, 0, (i as u8) << 1, 0, 0, 0, 0, 0]);
        }
        let text: String = letters.iter().map(|l| l.0).collect();
        let bytes = utf16(&text);
        t.extend_from_slice(&u32s(&[bytes.len() as u32]));
        t.extend_from_slice(&bytes);
        t.push(0); // no path
        t
    }

    #[test]
    fn version_14_ring_of_turned_and_shifted_letters_opens() {
        // As a version 14 seal stores its ring of letters (#955).
        let letters = [
            ('\u{6d2a}', 129_432_549, 842, 428),
            ('\u{5c71}', 98_894_641, 655, 505),
            ('\u{533a}', 347_035_491, 424, -760),
        ];
        let (doc, rep) = text_doc(
            14,
            b"CDRE",
            Vec::new(),
            OBJ_ARTISTIC_TEXT,
            &[(ARG_COORDS, i32s(&[0, 0]))],
            txsm14_ring(&letters),
        );
        assert_eq!(rep.skipped_objects, 0, "{:?}", rep.warnings);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        let ShapeKind::Text { spans, .. } = &first_shape(&doc).kind else {
            panic!("{:?}", first_shape(&doc).kind);
        };
        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "\u{6d2a}\u{5c71}\u{533a}");
        // One span a letter, each with its own placement.
        assert_eq!(spans.len(), 3, "{spans:?}");
        assert!((spans[0].angle_deg - 129.432549).abs() < 1e-9);
        assert!((spans[0].offset_x_pct - 842.0).abs() < 1e-9);
        assert!((spans[0].size_pt - 18.0).abs() < 1e-9);
        assert!((spans[0].baseline_shift_pt - 4.28 * 18.0).abs() < 1e-9);
        assert!((spans[1].angle_deg - 98.894641).abs() < 1e-9);
        // 347 degrees is 13 clockwise; a negative shift goes down.
        assert!((spans[2].angle_deg + 12.964509).abs() < 1e-9);
        assert!((spans[2].baseline_shift_pt + 7.6 * 18.0).abs() < 1e-9);
    }

    #[test]
    fn version_14_ring_text_cut_short_never_panics() {
        let t = txsm14_ring(&[('a', 0, 10, 10), ('b', 90_000_000, 20, 20)]);
        for cut in 0..t.len() {
            let _ = text::read_txsm(&t[..cut], 14, &mut |_| {});
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

    // Fills: bitmaps, textures, pattern tiles, version 16 fountain transform.

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

    /// version 13+ image fill body (types 9 and 11): one 0x514 record, then the
    /// pattern data.
    fn image_fill_v13(ftype: u16, id: u32, pattern: u32, w: u32) -> Vec<u8> {
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
                chunk(b"fild", &image_fill_v13(9, 4, 42, 254_000)),
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
    fn work_shared_between_threads_comes_back_in_order() {
        let items: Vec<u64> = (0..257).collect();
        let squares = parallel_map(&items, |x| x * x);
        assert_eq!(squares, items.iter().map(|x| x * x).collect::<Vec<_>>());
        assert!(parallel_map(&[] as &[u8], |x| *x).is_empty());
    }

    #[test]
    fn the_empty_colour_reads_quietly() {
        let main = riff(b"CDRV", &[]);
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 22);
        let empty = [0u8; 12];
        let mut r = ctx.reader(&empty);
        assert_eq!(ctx.read_color(&mut r), Some(Color::BLACK));
        assert!(ctx.report.warnings.is_empty());
        // An unknown model with a value still warns.
        let mut odd = [0u8; 12];
        odd[0] = 0x7f;
        odd[8] = 9;
        let mut r = ctx.reader(&odd);
        assert_eq!(ctx.read_color(&mut r), Some(Color::BLACK));
        assert_eq!(ctx.report.warnings.len(), 1);
    }

    #[test]
    fn bitmap_fills_in_tagged_records() {
        // As in a version 22 file: a 1600 record of 16 bytes, then a 1300
        // record of 36 bytes whose last u32 names the image (41).
        let mut p = u32s(&[0x640, 16, 0, 500_000, 500_000, 0]);
        p.extend_from_slice(&u32s(&[
            0x514, 36, 682_203, 500_000, 0, 0, 0, 500_000, 500_000, 0, 41,
        ]));
        p.extend_from_slice(&[0; 12]);
        let mut fild = u32s(&[1, 1300, (p.len() + 2) as u32]);
        fild.extend_from_slice(&u16s(&[9]));
        fild.extend_from_slice(&p);
        // A small first id (5) that a bare-word reading would misread.
        let mut q = u32s(&[0x514, 5, 254_000, 254_000]);
        q.extend_from_slice(&[0; 4]);
        q.extend_from_slice(&u16s(&[0]));
        q.push(0);
        q.extend_from_slice(&[0; 17]);
        q.extend_from_slice(&u32s(&[5]));
        let mut fild2 = u32s(&[2, 1300, (q.len() + 2) as u32]);
        fild2.extend_from_slice(&u16s(&[9]));
        fild2.extend_from_slice(&q);
        let main = riff(
            b"CDRV",
            &[
                chunk(b"bmp ", &bmp_2x2(41)),
                chunk(b"bmp ", &bmp_2x2(5)),
                chunk(b"fild", &fild),
                chunk(b"fild", &fild2),
            ]
            .concat(),
        );
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 22);
        for c in tree.root.find_all(b"bmp ") {
            ctx.read_bitmap(c);
        }
        for c in tree.root.find_all(b"fild") {
            ctx.read_fill(c);
        }
        assert!(ctx.report.warnings.is_empty(), "{:?}", ctx.report.warnings);
        for (fill, size) in [(1, 50.0), (2, 25.4)] {
            match &ctx.fills[&fill].fill {
                Fill::Pattern(Pattern::Bitmap { size_mm, .. }) => {
                    assert!((size_mm - size).abs() < 1e-9, "{fill}: {size_mm}");
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn texture_fill_falls_back_to_a_built_in_texture() {
        let main = riff(
            b"CDRD",
            &chunk(b"fild", &image_fill_v13(11, 6, 99, 508_000)),
        );
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
                let raw = raw_rows(png);
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
    fn fountain_version_16_transformation_sets_offset_and_edge_pad() {
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

    /// A one-rectangle version 14 page whose transparency argument is a
    /// record tagged 1300 with `body`.
    fn rect_with_transparency_record(body: &[u8]) -> (tracedraw_core::Document, ParseReport) {
        let mut op = u32s(&[0x514, body.len() as u32]);
        op.extend_from_slice(body);
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
            chunk(b"mcfg", &mcfg(14, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        parse(&riff(b"CDRE", &body), 14)
    }

    #[test]
    fn lens_records_in_the_transparency_argument() {
        use tracedraw_core::live::{Effect, Lens};
        // As in a version 15 file: kind 1, an id, type 3 (invert), then
        // two colour slots (CMYK white, the empty colour) and padding.
        let mut lens = vec![1u8, 0x48, 0x8b, 0x06, 0x0b, 3, 0, 0];
        lens.extend_from_slice(&[1, 12, 0, 0, 0, 2, 0, 5, 0]);
        lens.resize(126, 0);
        let (doc, rep) = rect_with_transparency_record(&lens);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        let s = first_shape(&doc);
        assert_eq!(s.effects, vec![Effect::Lens(Lens::Invert)]);
        assert_eq!(s.opacity, 1.0);
        assert!(rep.warnings.is_empty(), "{:?}", rep.warnings);

        // Another lens type: reported, the object read as before.
        lens[5] = 7;
        let (doc, rep) = rect_with_transparency_record(&lens);
        assert!(first_shape(&doc).effects.is_empty());
        assert!(rep.warnings.iter().any(|w| w.contains("lens type 7")));

        // A record of another kind keeps its transparency in thousandths.
        let mut t = vec![0u8; 6];
        t.extend_from_slice(&u16s(&[400]));
        t.resize(40, 0);
        let (doc, rep) = rect_with_transparency_record(&t);
        assert!(first_shape(&doc).effects.is_empty());
        assert!((first_shape(&doc).opacity - 0.6).abs() < 1e-9);
        assert!(rep.warnings.is_empty(), "{:?}", rep.warnings);

        // Truncated records read as no lens.
        let (doc, _) = rect_with_transparency_record(&[1, 0, 0]);
        assert!(first_shape(&doc).effects.is_empty());
    }

    // Arrowheads.

    /// `arrw` laid out like the version 11 and 12 samples: id, 4 bytes,
    /// the count, 4 bytes, the node types, a byte, the points, a byte.
    fn arrw(id: u32, pts: &[(i32, i32)], types: &[u8]) -> Vec<u8> {
        let mut a = u32s(&[id, 0]);
        a.extend_from_slice(&u16s(&[pts.len() as u16, 11, 0]));
        a.extend_from_slice(types);
        a.push(0);
        for (x, y) in pts {
            a.extend_from_slice(&i32s(&[*x, *y]));
        }
        a.push(0);
        a
    }

    /// version 13 outline with the given marker ids.
    fn outl_v13(id: u32, start: u32, end: u32) -> Vec<u8> {
        let mut o = u32s(&[id]);
        o.extend_from_slice(&u32s(&[1, 0]));
        o.extend_from_slice(&u16s(&[0, 0, 0]));
        o.extend_from_slice(&i32s(&[2540]));
        o.extend_from_slice(&u16s(&[100]));
        o.extend_from_slice(&i32s(&[0]));
        o.extend_from_slice(&[0; 48]);
        o.extend_from_slice(&color_bgr(0, 0, 0));
        o.extend_from_slice(&[0; 16]);
        o.extend_from_slice(&u16s(&[0]));
        o.extend_from_slice(&[0; 20]);
        o.extend_from_slice(&u32s(&[start, end]));
        o.extend_from_slice(&[0; 2]);
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
        // A five-cornered head is no preset: it is kept as a custom outline.
        let pentagon = arrw(
            11,
            &[(0, 0), (60, 0), (100, 50), (60, 100), (0, 100), (0, 0)],
            &[0x08, 0x40, 0x40, 0x40, 0x40, 0x48],
        );
        let main = riff(
            b"CDRD",
            &[
                chunk(b"arrw", &triangle),
                chunk(b"arrw", &diamond),
                chunk(b"arrw", &square),
                chunk(b"arrw", &bar),
                chunk(b"arrw", &circle),
                chunk(b"arrw", &open),
                chunk(b"arrw", &pentagon),
                chunk(b"outl", &outl_v13(1, 5, 6)),
                chunk(b"outl", &outl_v13(2, 7, 8)),
                chunk(b"outl", &outl_v13(3, 9, 10)),
                chunk(b"outl", &outl_v13(4, 99, 0)),
                chunk(b"outl", &outl_v13(5, 11, 0)),
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
        match arrows(5).0 {
            Arrowhead::Custom { path, name } => {
                use tracedraw_core::geometry::Shape as _;
                assert_eq!(name, "Arrowhead 11");
                let b = path.bounding_box();
                // Normalised: one unit tall, tip at x = 0, body towards -x.
                assert!((b.height() - 1.0).abs() < 1e-9, "{b:?}");
                assert!(b.x1.abs() < 1e-9 && (b.x0 + 1.0).abs() < 1e-9, "{b:?}");
            }
            other => panic!("{other:?}"),
        }
        assert!(ctx
            .report
            .warnings
            .iter()
            .any(|w| w.contains("arrowhead 99")));
    }

    /// `arrx` record laid out like the sample: id, zeros up to the count
    /// at byte 268, 4 bytes, node types, then i32 points, one spare byte.
    fn arrx(id: u32, pts: &[(i32, i32)], types: &[u8]) -> Vec<u8> {
        let mut a = u32s(&[id]);
        a.resize(ARRX_COUNT_AT, 0);
        a.extend_from_slice(&u16s(&[pts.len() as u16, 10]));
        a.extend_from_slice(&[0; 2]);
        a.extend_from_slice(types);
        for (x, y) in pts {
            a.extend_from_slice(&i32s(&[*x, *y]));
        }
        a.push(0);
        a
    }

    #[test]
    fn arrx_arrowheads_are_read_and_bad_ones_skipped() {
        // The sample's head: tip at the origin, body towards -x.
        let head = arrx(
            1,
            &[
                (-1_333_253, -357_244),
                (0, 0),
                (-1_333_253, 357_244),
                (-1_333_253, -357_244),
            ],
            &[0x0c, 0x44, 0x44, 0x48],
        );
        let mut short = arrx(2, &[(0, 0), (1, 1)], &[0x0c, 0x44]);
        short.truncate(ARRX_COUNT_AT + 8);
        let main = riff(
            b"CDRD",
            &[
                chunk(b"arrx", &head),
                chunk(b"arrx", &short),
                chunk(b"arrx", &[1, 0]),
            ]
            .concat(),
        );
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 21);
        for c in tree.root.find_all(b"arrx") {
            ctx.read_arrowhead_x(c);
        }
        assert_eq!(ctx.arrows.get(&1), Some(&Arrowhead::Arrow));
        assert!(!ctx.arrows.contains_key(&2));
        assert!(ctx
            .report
            .warnings
            .iter()
            .any(|w| w.contains("arrowhead 2")));
    }

    #[test]
    fn version_12_text_with_a_byte_a_character_is_8_bit() {
        let styles = [Style7 {
            chars: 4,
            font: None,
            flags: None,
            size_units: None,
            fill: None,
        }];
        // The UTF-16 form: as two bytes a character.
        let wide = txsm7(12, false, &styles, "Next");
        let mut warn = |_: String| {};
        let read = |d: &[u8], warn: &mut dyn FnMut(String)| {
            text::read_txsm(d, 12, warn).map(|t| {
                t.paragraphs[0]
                    .runs
                    .iter()
                    .map(|r| r.text.clone())
                    .collect::<String>()
            })
        };
        assert_eq!(read(&wide, &mut warn).as_deref(), Some("Next"));
        // The same record with the text as one byte a character (as a
        // version 12 sample stores it): its byte count equals its length.
        let tail = 4 + 8 + 1; // count, UTF-16 text, no path
        let mut narrow = wide[..wide.len() - tail].to_vec();
        narrow.extend_from_slice(&u32s(&[4]));
        narrow.extend_from_slice(b"Next");
        narrow.push(0);
        assert_eq!(read(&narrow, &mut warn).as_deref(), Some("Next"));
    }

    #[test]
    fn node_types_of_a_long_curve_are_read_in_one_pass() {
        use tracedraw_core::nodes::NodeType;
        // A zigzag of 20 000 line nodes, every third marked smooth in the
        // file (its straight segments read as cusps): each marked node is
        // kept with its type. Searching the whole curve for every node
        // made a file with such curves take minutes to open.
        let n = 20_000;
        let pts: Vec<Point> = (0..n)
            .map(|i| Point::new(i as f64, if i % 2 == 0 { 0.0 } else { 1.0 }))
            .collect();
        let types: Vec<u8> = (0..n)
            .map(|i| {
                let op = if i == 0 { 0x00 } else { 0x40 };
                op | if i % 3 == 0 { 0x10 } else { 0x00 }
            })
            .collect();
        let (path, _) = build_path(&pts, &types);
        let start = std::time::Instant::now();
        let kept = file_node_types(&path, &pts, &types);
        assert!(start.elapsed().as_secs() < 20, "{:?}", start.elapsed());
        assert_eq!(kept.len(), (0..n).filter(|i| i % 3 == 0).count());
        assert!(kept.values().all(|t| *t == NodeType::Smooth));
    }

    #[test]
    fn a_fitted_text_without_settings_follows_the_curve_its_box_rests_on() {
        use tracedraw_core::document::{ParagraphStyle, TextSpan};
        use tracedraw_core::ShapeId;
        let circle = |id: u64, r: f64| {
            Shape::new(
                ShapeId(id),
                ShapeKind::Ellipse {
                    rect: Rect::new(100.0 - r, 200.0 - r, 100.0 + r, 200.0 + r),
                    arc: None,
                },
            )
        };
        let mut text = Shape::new(
            ShapeId(3),
            ShapeKind::Text {
                spans: vec![TextSpan::new("along the bottom", "Arial", 6.0)],
                origin: Point::ZERO,
                frame: None,
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            },
        );
        text.transform = Affine::translate((60.0, 150.0));
        let shapes = vec![circle(1, 50.0), circle(2, 53.0), text];
        let curves = [0, 1];
        let cap = cap_height_mm(&shapes[2]);
        assert!((cap - 6.0 * 25.4 / 72.0 * 0.7).abs() < 1e-9);
        // A line along the bottom of the outer circle, its letters hanging
        // outwards: the box reaches a capital's height below the circle
        // and stops below the centre.
        let fitted = OnPathText {
            settings: None,
            page_box: Some(Rect::new(52.0, 147.0 - cap, 148.0, 190.0)),
        };
        let (ci, anchor) = anchored_path_for_text(&shapes, &curves, &fitted, cap).expect("a curve");
        assert_eq!(ci, 1, "the outer circle");
        let anchor = anchor.expect("an anchor");
        let mut t = shapes[2].clone();
        assert!(fit_text_at(&mut t, &shapes[1], anchor));
        let ShapeKind::Text {
            on_path: Some(tp),
            align,
            ..
        } = &t.kind
        else {
            panic!("{:?}", t.kind);
        };
        assert_eq!(*align, TextAlign::Center);
        let b = tp.baseline();
        let (mid, dir) = b.at(tp.offset + b.length() / 2.0).expect("middle");
        // In the text's space (moved by its transform): the bottom of the
        // circle, letters pointing down and out.
        assert!(
            (mid.x - 40.0).abs() < 0.6 && (mid.y + 3.0).abs() < 0.6,
            "{mid:?}"
        );
        assert!(dir.x < 0.0, "read right to left along the bottom: {dir:?}");
        // A box around the whole circle names no side.
        let ring = OnPathText {
            settings: None,
            page_box: Some(Rect::new(45.0, 145.0, 155.0, 255.0)),
        };
        assert!(anchored_path_for_text(&shapes, &curves, &ring, cap).is_none());
        // A box nowhere near a curve.
        let far = OnPathText {
            settings: None,
            page_box: Some(Rect::new(400.0, 400.0, 420.0, 410.0)),
        };
        assert!(anchored_path_for_text(&shapes, &curves, &far, cap).is_none());
    }

    #[test]
    fn clip_containers_draw_their_clpt_group_inside() {
        // A version 12 container (a 30 mm square) naming group 0x55 of the
        // document's LIST clpt, which holds a 10 mm square.
        let square = |side: i32| {
            list(
                b"obj ",
                &chunk(
                    b"loda",
                    &loda(
                        OBJ_RECT,
                        &[(ARG_COORDS, i32s(&[side * 10_000, side * 10_000, 0]))],
                    ),
                ),
            )
        };
        let group = list(
            b"grp ",
            &[chunk(b"spnd", &u32s(&[0x55])), square(10)].concat(),
        );
        let container = |id: u32| {
            list(
                b"obj ",
                &chunk(
                    b"loda",
                    &loda(
                        OBJ_RECT,
                        &[
                            (ARG_CLIP_CONTENTS, u32s(&[id, 1, 0, 1])),
                            (ARG_COORDS, i32s(&[300_000, 300_000, 0])),
                        ],
                    ),
                ),
            )
        };
        let read = |id: u32| {
            let body = [
                chunk(b"mcfg", &mcfg(12, 2_540_000, 2_540_000)),
                list(b"clpt", &group),
                list(b"page", &list(b"layr", &container(id))),
            ]
            .concat();
            parse(&riff(b"CDRC", &body), 12)
        };
        let (doc, rep) = read(0x55);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::ClipFrame { frame, contents } => {
                assert!((frame.bounds().width() - 30.0).abs() < 1e-6);
                assert_eq!(contents.len(), 1);
                assert!((contents[0].bounds().width() - 10.0).abs() < 1e-6);
            }
            other => panic!("{other:?}"),
        }
        // The group is not drawn on its own anywhere else.
        assert_eq!(
            doc.pages
                .iter()
                .map(|p| p.layers.iter().map(|l| l.shapes.len()).sum::<usize>())
                .sum::<usize>(),
            1
        );
        // A number naming no group: the container alone, with a warning.
        let (doc, rep) = read(0x99);
        assert!(matches!(
            first_shape(&doc).kind,
            ShapeKind::Rect { .. } | ShapeKind::Path { .. }
        ));
        assert!(
            rep.warnings.iter().any(|w| w.contains("clip container")),
            "{:?}",
            rep.warnings
        );
    }

    #[test]
    fn version_15_artistic_text_coordinates_holding_an_id_are_no_place() {
        let styles = [Style7 {
            chars: 2,
            font: None,
            flags: None,
            size_units: None,
            fill: None,
        }];
        let t = txsm7(15, true, &styles, "Hi");
        let read = |coords: Vec<u8>| {
            let obj = list(
                b"obj ",
                &[
                    chunk(b"loda", &loda(OBJ_ARTISTIC_TEXT, &[(ARG_COORDS, coords)])),
                    chunk(b"txsm", &t),
                ]
                .concat(),
            );
            let body = [
                chunk(b"mcfg", &mcfg(15, 2_540_000, 2_540_000)),
                list(b"page", &list(b"layr", &obj)),
            ]
            .concat();
            let (doc, rep) = parse(&riff(b"CDRF", &body), 15);
            assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
            match &first_shape(&doc).kind {
                ShapeKind::Text { origin, .. } => *origin,
                other => panic!("{other:?}"),
            }
        };
        // As version 15 files hold it: 0 and a record id (some 41 m, or
        // 9.2 m, which once passed for a place and put the text off the
        // page). The transform places the text.
        for id in [0x1887_1a00, 91_956_000] {
            let o = read(u32s(&[0, id]));
            assert!(o.x.abs() < 1e-9 && o.y.abs() < 1e-9, "{o:?}");
        }
    }

    #[test]
    fn version_12_text_mixes_one_and_two_byte_characters() {
        let styles = [Style7 {
            chars: 6,
            font: None,
            flags: None,
            size_units: None,
            fill: None,
        }];
        // As a version 12 file stores "Stećak": the letters in one byte
        // (Windows-1252) and "ć" as a UTF-16 unit, its record flagged.
        let wide = txsm7(12, false, &styles, "Stećak");
        let tail = 6 * 8 + 4 + 12 + 1; // records, count, UTF-16 text, no path
        let mut d = wide[..wide.len() - tail].to_vec();
        for i in 0..6 {
            d.extend_from_slice(&u16s(&[(i == 3) as u16, 0, 0, 0]));
        }
        let bytes = [b'S', b't', b'e', 0x07, 0x01, b'a', b'k'];
        d.extend_from_slice(&u32s(&[bytes.len() as u32]));
        d.extend_from_slice(&bytes);
        d.push(0);
        let mut warnings = Vec::new();
        let t = text::read_txsm(&d, 12, &mut |w| warnings.push(w)).expect("txsm");
        let got: String = t.paragraphs[0]
            .runs
            .iter()
            .map(|r| r.text.clone())
            .collect();
        assert_eq!(got, "Stećak");
        assert!(warnings.is_empty(), "{warnings:?}");
        // Accented Latin-1 letters stay single bytes ("ç" as E7).
        let wide = txsm7(12, false, &styles, "açaí!");
        let tail = 5 * 8 + 4 + 10 + 1;
        let mut d = wide[..wide.len() - tail].to_vec();
        d.extend_from_slice(&[0; 5 * 8]);
        d.extend_from_slice(&u32s(&[5]));
        d.extend_from_slice(&[b'a', 0xe7, b'a', 0xed, b'!']);
        d.push(0);
        let t = text::read_txsm(&d, 12, &mut |_| {}).expect("txsm");
        let got: String = t.paragraphs[0]
            .runs
            .iter()
            .map(|r| r.text.clone())
            .collect();
        assert_eq!(got, "açaí!");
    }

    /// A version 12 outline: 52 bytes where version 13 has the matrix, and
    /// the marker ids 22 bytes after the first dash.
    fn outl_v12(id: u32, start: u32, end: u32) -> Vec<u8> {
        let mut o = u32s(&[id]);
        // Line type 2 (a line), caps, join, 2 bytes, width, stretch, 2
        // bytes, nib angle.
        o.extend_from_slice(&u16s(&[2, 0, 0, 0]));
        o.extend_from_slice(&i32s(&[2540]));
        o.extend_from_slice(&u16s(&[100, 0]));
        o.extend_from_slice(&i32s(&[0]));
        o.extend_from_slice(&[0; 52]);
        o.extend_from_slice(&color_bgr(0, 0, 0));
        o.extend_from_slice(&[0; 16]);
        o.extend_from_slice(&u16s(&[0]));
        o.extend_from_slice(&[0; 22]);
        o.extend_from_slice(&u32s(&[start, end]));
        o
    }

    #[test]
    fn version_12_outlines_find_their_arrowheads() {
        let triangle = arrw(
            0x07da_c718,
            &[(0, 0), (-777_494, 388_620), (-777_494, -388_874), (0, 0)],
            &[0x0c, 0x44, 0x44, 0x48],
        );
        let main = riff(
            b"CDRD",
            &[
                chunk(b"arrw", &triangle),
                chunk(b"outl", &outl_v12(1, 0x07da_c718, 0)),
                chunk(b"outl", &outl_v12(2, 0, 0x07da_c718)),
            ]
            .concat(),
        );
        let tree = crate::riff::parse(&main).unwrap();
        let mut ctx = Ctx::new(&main, &tree, 12);
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
        assert_eq!(arrows(1), (Arrowhead::Arrow, Arrowhead::None));
        assert_eq!(arrows(2), (Arrowhead::None, Arrowhead::Arrow));
        assert!(ctx.report.warnings.is_empty(), "{:?}", ctx.report.warnings);
    }

    #[test]
    fn connectors_are_straight_lines_between_their_ends() {
        // ppdt: 0, 1, start, end (25.4 mm apart), then attachment data.
        let mut ppdt = u32s(&[0, 1]);
        ppdt.extend_from_slice(&i32s(&[0, 0, 254_000, 0]));
        ppdt.extend_from_slice(&[0; 32]);
        let obj = list(
            b"obj ",
            &[
                chunk(b"loda", &loda(OBJ_CONNECTOR, &[])),
                list(
                    b"npps",
                    &[chunk(b"ppid", &[0; 16]), chunk(b"ppdt", &ppdt)].concat(),
                ),
            ]
            .concat(),
        );
        // A connector without its record is skipped with a warning.
        let bare = list(b"obj ", &chunk(b"loda", &loda(OBJ_CONNECTOR, &[])));
        let body = [
            chunk(b"mcfg", &mcfg(9, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &[obj, bare].concat())),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDR9", &body), 9);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Path { path, closed } => {
                use tracedraw_core::geometry::Shape as _;
                assert!(!closed);
                assert_eq!(path.elements().len(), 2);
                let b = path.bounding_box();
                assert!(
                    (b.width() - 25.4).abs() < 1e-9 && b.height().abs() < 1e-9,
                    "{b:?}"
                );
            }
            other => panic!("{other:?}"),
        }
        assert!(rep.warnings.iter().any(|w| w.contains("0x26")));
    }

    #[test]
    fn point_lists_are_closed_outlines_through_their_points() {
        // ppdt: u16 count, u32 1, the points, then a u32 for each point.
        let mut ppdt = 4u16.to_le_bytes().to_vec();
        ppdt.extend_from_slice(&u32s(&[1]));
        ppdt.extend_from_slice(&i32s(&[0, 0, 254_000, 0, 254_000, 127_000, 0, 127_000]));
        ppdt.extend_from_slice(&u32s(&[1, 0, 1, 0]));
        let obj = list(
            b"obj ",
            &[
                chunk(b"loda", &loda(OBJ_CONNECTOR, &[])),
                list(
                    b"npps",
                    &[chunk(b"ppid", &[0; 16]), chunk(b"ppdt", &ppdt)].concat(),
                ),
            ]
            .concat(),
        );
        let body = [
            chunk(b"mcfg", &mcfg(9, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &obj)),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDR9", &body), 9);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Path { path, closed } => {
                use tracedraw_core::geometry::Shape as _;
                assert!(closed);
                // A move, three lines and the close.
                assert_eq!(path.elements().len(), 5);
                let b = path.bounding_box();
                assert!(
                    (b.width() - 25.4).abs() < 1e-9 && (b.height() - 12.7).abs() < 1e-9,
                    "{b:?}"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn guidelines_become_page_guides_not_skipped_objects() {
        // 12 bytes, the angle, then the line's points from the page centre:
        // a vertical line 10 mm right of centre, a horizontal one 20 mm
        // below it.
        let guide = |pts: &[i32], angle: i32| {
            let mut a = vec![0u8; 12];
            a.extend_from_slice(&i32s(&[angle]));
            a.extend_from_slice(&i32s(pts));
            chunk(b"loda", &loda(OBJ_GUIDE, &[(ARG_GUIDE_LINE, a)]))
        };
        let far = 228_600_000;
        let v = list(
            b"obj ",
            &guide(&[100_000, -far, 100_000, far, 100_000, far], 90_000_000),
        );
        let h = list(
            b"obj ",
            &guide(&[-far, -200_000, far, -200_000, 0, -200_000], 0),
        );
        let body = [
            chunk(b"mcfg", &mcfg(9, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &[v, h].concat())),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDR9", &body), 9);
        assert_eq!(rep.skipped_objects, 0, "{:?}", rep.warnings);
        use tracedraw_core::document::GuideLine;
        let page = &doc.pages[0];
        let (cx, cy) = (page.size.width / 2.0, page.size.height / 2.0);
        let lines: Vec<GuideLine> = page.guides.iter().map(|g| g.line).collect();
        assert_eq!(
            lines,
            vec![
                GuideLine::Vertical { x: cx + 10.0 },
                GuideLine::Horizontal { y: cy - 20.0 }
            ]
        );
    }

    #[test]
    fn embedded_pictures_show_in_their_objects_rectangle() {
        // Size, then two corners: 20 by 10 mm around the page centre.
        let obj = |storage: &str| {
            let mut coords = i32s(&[200_000, 100_000]);
            coords.extend_from_slice(&i32s(&[-100_000, 50_000, 100_000, -50_000]));
            let name: Vec<u8> = storage
                .encode_utf16()
                .chain([0])
                .flat_map(|u| u.to_le_bytes())
                .collect();
            list(
                b"obj ",
                &chunk(
                    b"loda",
                    &loda(OBJ_EMBEDDED, &[(ARG_NAME, name), (ARG_COORDS, coords)]),
                ),
            )
        };
        let body = [
            chunk(b"mcfg", &mcfg(12, 2_540_000, 2_540_000)),
            chunk(b"ole2", &crate::ole::tests::compound_with_picture()),
            list(
                b"page",
                &list(b"layr", &[obj("Embedding 3"), obj("Embedding 9")].concat()),
            ),
        ]
        .concat();
        let (doc, rep) = parse(&riff(b"CDRC", &body), 12);
        assert_eq!(rep.shapes, 1, "{:?}", rep.warnings);
        match &first_shape(&doc).kind {
            ShapeKind::Bitmap {
                rect,
                width_px,
                height_px,
                ..
            } => {
                assert_eq!((*width_px, *height_px), (2, 2));
                assert!((rect.width() - 20.0).abs() < 1e-9 && (rect.height() - 10.0).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
        // A storage the file does not hold: skipped, named in a warning.
        assert!(rep.warnings.iter().any(|w| w.contains("Embedding 9")));
        // Without a storage name (version 21 keeps such objects apart):
        // skipped with its own warning, not as an unknown type.
        let bare = list(
            b"obj ",
            &chunk(
                b"loda",
                &loda(OBJ_EMBEDDED, &[(ARG_COORDS, i32s(&[1, 1, 0, 0, 1, 1]))]),
            ),
        );
        let body = [
            chunk(b"mcfg", &mcfg(12, 2_540_000, 2_540_000)),
            list(b"page", &list(b"layr", &bare)),
        ]
        .concat();
        let (_, rep) = parse(&riff(b"CDRC", &body), 12);
        assert!(rep.warnings.iter().any(|w| w.contains("not shown")));
        assert!(!rep.warnings.iter().any(|w| w.contains("0x0a")));
    }
}
