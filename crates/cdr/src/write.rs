//! `.cdr` writer for every version from 7 to 27.
//!
//! The chunk tree follows the layout of real files of each version, record
//! by record (`docs/cdr-format.md`, Writing, lists each record and its
//! sources): versions 7 to 13 are a RIFF file; versions 14 and 15
//! keep that RIFF stream in a ZIP archive as `content/riffData.cdr`; from
//! version 16 the RIFF stream (`content/root.dat`) holds the chunk tree and
//! each payload lives in a data file of the archive (`content/data/*.dat`),
//! reached through a 16-byte redirect record. Coordinates are 1/254000 inch
//! with the origin at the page centre.
//!
//! What is written: pages (after a master page with the guide, desktop and
//! grid layers files carry), layers with names, rectangles, ellipses (with
//! arcs), curves for every other outline (polygons, paths, tables, symbols,
//! expanded effects), groups, artistic text with one font, size and style
//! per run, bitmaps (24-bit), solid and fountain fills, outlines (width,
//! caps, joins, dashes, nib, "scale with object", "behind fill") and
//! uniform transparency. A clip frame holding one bitmap becomes that
//! bitmap with the frame as its crop path; other clip frames a group of
//! the frame outline and the contents. Rectangles of versions 15 and later
//! are written as the curve they draw (their corner records are not
//! understood well enough to write).

use crate::parse::{
    ARG_COORDS, ARG_FILL, ARG_NAME, ARG_OPACITY, ARG_OUTLINE, ARG_STYLE, ARG_STYLE_STRING,
    OBJ_ARTISTIC_TEXT, OBJ_BITMAP, OBJ_CURVE, OBJ_ELLIPSE, OBJ_RECT,
};
use std::collections::HashMap;
use std::io::Write as _;
use tracedraw_core::{
    document::{Shape, ShapeKind, Symbol, TextSpan},
    geometry::{Affine, BezPath, PathEl, Point, Rect, Shape as _},
    style::{FountainKind, LineCap, LineJoin},
    Color, Document, Fill, Pattern, SharedBytes, Stroke, TextAlign,
};

/// The oldest version whose files written here are known to open in the
/// program that wrote the samples (by a user's tests: text from 16 in
/// files of versions 7 to 22, and every object and text look of files
/// of versions 14 and 15); older ones are offered as experimental.
pub const CHECKED_SINCE: u16 = 14;
/// The newest version known to open there (versions 23 to 27, written
/// with the records of version 22, opened with their images in that
/// program's 2026 release, a user's test of 2026-10-18).
pub const CHECKED_UNTIL: u16 = 27;
/// The oldest version the writer produces.
pub const OLDEST_VERSION: u16 = 7;
/// The newest version the writer produces.
pub const NEWEST_VERSION: u16 = 27;
/// The version [`document_to_cdr`] writes: the newest.
pub const DEFAULT_VERSION: u16 = NEWEST_VERSION;

/// The versions the app and the command line save in: those checked to
/// open in other programs. The older ones the writer can produce stay
/// for testing (`TRACEDRAW_CDR_EXPERIMENTAL=1` on the command line).
pub fn checked_versions() -> std::ops::RangeInclusive<u16> {
    CHECKED_SINCE..=CHECKED_UNTIL
}

/// The versions [`document_to_cdr_version`] can write, oldest first.
pub fn versions() -> std::ops::RangeInclusive<u16> {
    OLDEST_VERSION..=NEWEST_VERSION
}

/// Write the whole document as a `.cdr` file of [`DEFAULT_VERSION`].
pub fn document_to_cdr(doc: &Document) -> Vec<u8> {
    document_to_cdr_version(doc, DEFAULT_VERSION)
}

/// Write the whole document as a `.cdr` file of `version` (7 to 27; other
/// values are brought into that range).
pub fn document_to_cdr_version(doc: &Document, version: u16) -> Vec<u8> {
    document_to_cdr_version_with(doc, version, &Previews::default())
}

/// Pictures of the drawing stored with it from version 16 (PNG files):
/// the thumbnail and one per page, in page order. Readers show them in
/// file dialogs; the drawing opens without them.
#[derive(Debug, Clone, Default)]
pub struct Previews {
    pub thumbnail: Option<Png>,
    pub pages: Vec<Png>,
    /// The faces the text uses, with their font files where found: listed
    /// in the font table (version 16 and later).
    pub fonts: Vec<FontFace>,
    /// Embed the faces whose licence allows it.
    pub embed_fonts: bool,
}

/// A PNG file and its size in pixels.
#[derive(Debug, Clone)]
pub struct Png {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// [`document_to_cdr_version`] with the pictures stored beside the drawing
/// (version 16 and later).
pub fn document_to_cdr_version_with(doc: &Document, version: u16, previews: &Previews) -> Vec<u8> {
    let v = Ver(version.clamp(OLDEST_VERSION, NEWEST_VERSION));
    let mut w = Writer::new(v, &doc.symbols, &doc.title);
    let mut tree = w.document(doc);
    let font_ids = w.fonts.clone();
    if !v.zip() {
        // RIFF files carry their picture and an empty keywords and
        // comment list right after `vrsn`.
        let at = tree
            .iter()
            .position(|n| matches!(n, Node::Chunk(id, _) if id == b"vrsn"))
            .map_or(0, |i| i + 1);
        let mut extra = Vec::new();
        if let Some(disp) = previews.pages.first().and_then(|p| disp(v, &p.data)) {
            extra.push(chunk(b"DISP", disp));
        }
        let empty = vec![0u8; if v.unicode() { 2 } else { 128 }];
        extra.push(list(
            b"INFO",
            vec![chunk(b"IKEY", empty.clone()), chunk(b"ICMT", empty)],
        ));
        tree.splice(at..at, extra);
    }
    if !v.redirects() {
        tree = compress_riff(tree);
    }
    if !v.zip() {
        tree.push(chunk(b"sumi", sumi(v, doc)));
    }
    if v.redirects() {
        let (root, files) = pack_redirected(v, &tree);
        zip_v16(v, doc, &root, &files, previews, &font_ids)
    } else {
        let riff = riff_file(v.form(), &tree);
        if v.zip() {
            zip_v14(v, doc, &riff, previews)
        } else {
            riff
        }
    }
}

/// Coordinate units per millimetre.
const UNITS_PER_MM: f64 = 254000.0 / 25.4;
/// The "since version" tag records of version 13 and later start with.
const SINCE_13: u32 = 1300;
/// The `trfd` record tag of version 18 and later.
const TRFD_SINCE_18: u32 = 1740;
/// Halftone screen of fills and outlines: 45 degrees, 60 lines per inch.
const SCREEN: [u8; 8] = [0x40, 0xa5, 0xae, 0x02, 0x3c, 0x00, 0x00, 0x00];
/// A full ellipse is stored from 270 to 270 degrees (millionths).
const FULL_ELLIPSE_ANGLE: i32 = 270_000_000;
/// Argument types this writer adds to those `parse.rs` names.
const ARG_ID: u32 = 100;
const ARG_LAYER_COLOR: u32 = 2000;
const ARG_PROPS: u32 = 12000;
const ARG_GUID: u32 = 40050;
/// On text objects of version 16 and later: 1.
const ARG_TEXT_LINK: u32 = 16015;
/// Version 21: 1 hides the object (it is kept but not drawn).
const ARG_HIDDEN: u32 = 16016;
/// On guides layers of version 17 and later: 2.
const ARG_GUIDES_MODE: u32 = 16001;
/// Layer kinds stored as the `loda` object type of a layer.
const LAYER_PLAIN: u32 = 0;
const LAYER_GRID: u32 = 0x0b;
const LAYER_GUIDES: u32 = 0x0c;
const LAYER_DESKTOP: u32 = 0x11;
/// The first text style; text paragraphs refer to the one of their
/// alignment (`style_for`).
const STYLE_ID: u32 = 1;
/// The style graphics refer to (before version 16, argument 200).
const GRAPHIC_STYLE_ID: u32 = STYLE_ID + STYLE_ALIGNS.len() as u32;

/// Alignments of the style table, in style id order from `STYLE_ID`:
/// left, centre, right, justify, as alignment codes (0 none, 1 left,
/// 2 centre, 3 right, 4 full justify, 5 forced; the style strings'
/// numbering too).
const STYLE_ALIGNS: [(TextAlign, u32); 4] = [
    (TextAlign::Left, 1),
    (TextAlign::Center, 2),
    (TextAlign::Right, 3),
    (TextAlign::Justify, 4),
];

/// The style of text with this alignment.
fn style_for(align: TextAlign) -> u32 {
    // The style table holds the four common alignments; forced
    // justification takes the justified style, no alignment the left one.
    let align = match align {
        TextAlign::ForceJustify => TextAlign::Justify,
        TextAlign::None => TextAlign::Left,
        a => a,
    };
    STYLE_ALIGNS
        .iter()
        .position(|(a, _)| *a == align)
        .map_or(STYLE_ID, |i| STYLE_ID + i as u32)
}

/// The version being written and what it implies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ver(u16);

impl Ver {
    /// `CDR` and the version letter: digits up to 9, then `A` (10) to `H`
    /// (17), `J` (18) to `O` (23); `I` is not used. Then `P` (24), `R`
    /// (25), `T` (26) and `U` (27), as files saved in each have them (the
    /// letters between belong to updates).
    fn form(self) -> [u8; 4] {
        let c = match self.0 {
            v @ 0..=9 => b'0' + v as u8,
            v @ 10..=17 => b'A' + (v - 10) as u8,
            v @ 18..=23 => b'J' + (v - 18) as u8,
            24 => b'P',
            25 => b'R',
            26 => b'T',
            _ => b'U',
        };
        [b'C', b'D', b'R', c]
    }
    /// Tagged fill, outline, transform and configuration records.
    fn tagged(self) -> bool {
        self.0 >= 13
    }
    /// The zero 12000 on layers, groups and objects, flag 0x40 on pages
    /// and a master page with an object box, as files from version 13 on
    /// have them (a version 13 file without them closed the program that
    /// wrote the samples).
    fn props(self) -> bool {
        self.0 >= 13
    }
    /// A ZIP archive around the RIFF stream.
    fn zip(self) -> bool {
        self.0 >= 14
    }
    /// Payloads in data files, reached through redirect records.
    fn redirects(self) -> bool {
        self.0 >= 16
    }
    /// Names and text in UTF-16 (8-bit before version 12).
    fn unicode(self) -> bool {
        self.0 >= 12
    }
    /// Objects carry their style as a string (argument 201).
    fn style_strings(self) -> bool {
        self.0 >= 16
    }
    /// Style strings and text one byte per character (UTF-16 before).
    fn byte_strings(self) -> bool {
        self.0 >= 17
    }
    /// Rectangles keep their own record (from 15 on their layout changes).
    fn native_rects(self) -> bool {
        self.0 < 15
    }
}

// ---- bytes -------------------------------------------------------------

#[derive(Default)]
struct Buf(Vec<u8>);

impl Buf {
    fn u8(&mut self, x: u8) -> &mut Self {
        self.0.push(x);
        self
    }
    fn u16(&mut self, x: u16) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }
    fn u32(&mut self, x: u32) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }
    fn i32(&mut self, x: i32) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }
    fn f64(&mut self, x: f64) -> &mut Self {
        self.0.extend_from_slice(&x.to_le_bytes());
        self
    }
    fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.0.extend_from_slice(b);
        self
    }
    fn zeros(&mut self, n: usize) -> &mut Self {
        self.0.resize(self.0.len() + n, 0);
        self
    }
    /// Zeros up to `len` bytes in all (nothing when already longer).
    fn pad_to(&mut self, len: usize) -> &mut Self {
        if self.0.len() < len {
            self.0.resize(len, 0);
        }
        self
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn done(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.0)
    }
}

fn units(mm: f64) -> i32 {
    if !mm.is_finite() {
        return 0;
    }
    (mm * UNITS_PER_MM)
        .round()
        .clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
}

fn utf16z(s: &str) -> Vec<u8> {
    let mut v = utf16(s);
    v.extend_from_slice(&[0, 0]);
    v
}

/// Windows-1252 bytes of `s`, or `None` when a character has none.
fn cp1252(s: &str) -> Option<Vec<u8>> {
    const HIGH: [u16; 32] = [
        0x20ac, 0, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
        0x0152, 0, 0x017d, 0, 0, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x02dc,
        0x2122, 0x0161, 0x203a, 0x0153, 0, 0x017e, 0x0178,
    ];
    s.chars()
        .map(|c| {
            let u = c as u32;
            if u < 0x80 || (0xa0..0x100).contains(&u) {
                Some(u as u8)
            } else {
                HIGH.iter()
                    .position(|h| *h != 0 && u32::from(*h) == u)
                    .map(|i| 0x80 + i as u8)
            }
        })
        .collect()
}

/// A layer or object name: UTF-16 from version 12, 8-bit before.
fn name_bytes(v: Ver, s: &str) -> Vec<u8> {
    if v.unicode() {
        utf16z(s)
    } else {
        let mut b = cp1252(s).unwrap_or_else(|| s.bytes().filter(|c| c.is_ascii()).collect());
        b.push(0);
        b
    }
}

/// A style string argument or record: its length and text (UTF-16 before
/// version 17, counted in units; bytes after).
/// The locale of a text record: a count, then that many UTF-16 units.
fn locale_bytes() -> Vec<u8> {
    let units = utf16(TEXT_LOCALE);
    let mut b = Buf::default();
    b.u32((units.len() / 2) as u32).bytes(&units);
    b.done()
}

/// The locale text records are written with.
const TEXT_LOCALE: &str = "ENU";

fn style_string(v: Ver, s: &str) -> Vec<u8> {
    let mut b = Buf::default();
    if v.byte_strings() {
        b.u32(s.len() as u32).bytes(s.as_bytes());
    } else {
        let units: Vec<u8> = utf16(s);
        b.u32((units.len() / 2) as u32).bytes(&units);
    }
    b.done()
}

/// A random-looking GUID (version 4, as files carry them, in the byte
/// order files store), the same for the same seed and counter.
fn guid(seed: u64, n: u64) -> [u8; 16] {
    let mut x = seed ^ n.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut out = [0u8; 16];
    for chunk in out.chunks_mut(8) {
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
        x ^= x >> 33;
        x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        x ^= x >> 33;
        chunk.copy_from_slice(&x.to_le_bytes());
    }
    // Version 4 in the high bits of the third field, variant 10 in the
    // fourth.
    out[7] = (out[7] & 0x0f) | 0x40;
    out[8] = (out[8] & 0x3f) | 0x80;
    out
}

// ---- chunk tree --------------------------------------------------------

#[derive(Debug, Clone)]
enum Node {
    Chunk([u8; 4], Vec<u8>),
    List([u8; 4], Vec<Node>),
    /// A list whose body after its type is a record, not chunks (the
    /// style table before version 16).
    RawList([u8; 4], Vec<u8>),
}

fn chunk(id: &[u8; 4], data: Vec<u8>) -> Node {
    Node::Chunk(*id, data)
}

fn list(kind: &[u8; 4], children: Vec<Node>) -> Node {
    Node::List(*kind, children)
}

fn put_chunk(out: &mut Vec<u8>, id: &[u8], payload: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
}

fn serialize(nodes: &[Node], out: &mut Vec<u8>) {
    for n in nodes {
        match n {
            Node::Chunk(id, data) => put_chunk(out, id, data),
            Node::List(kind, children) => {
                let mut body = kind.to_vec();
                serialize(children, &mut body);
                put_chunk(out, b"LIST", &body);
            }
            Node::RawList(kind, data) => {
                let mut body = kind.to_vec();
                body.extend_from_slice(data);
                put_chunk(out, b"LIST", &body);
            }
        }
    }
}

/// `nodes` as the inflated stream of a `cmpr` list: each chunk's size field
/// holds the index of its size in a pool (every chunk and list in order,
/// lists before their children), padding as in the plain stream.
fn serialize_indexed(nodes: &[Node], out: &mut Vec<u8>, pool: &mut Vec<u32>) {
    for n in nodes {
        let index = pool.len();
        pool.push(0);
        let (id, start): (&[u8], usize) = match n {
            Node::Chunk(id, _) => (id, out.len() + 8),
            Node::List(..) | Node::RawList(..) => (b"LIST", out.len() + 8),
        };
        out.extend_from_slice(id);
        out.extend_from_slice(&(index as u32).to_le_bytes());
        match n {
            Node::Chunk(_, data) => out.extend_from_slice(data),
            Node::List(kind, children) => {
                out.extend_from_slice(kind);
                serialize_indexed(children, out, pool);
            }
            Node::RawList(kind, data) => {
                out.extend_from_slice(kind);
                out.extend_from_slice(data);
            }
        }
        let size = out.len() - start;
        pool[index] = size as u32;
        if size % 2 == 1 {
            out.push(0);
        }
    }
}

/// A `CPng` block: the tag, `01 00 04 00` and the zlib stream of `data`.
fn cpng(data: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    let mut b = b"CPng\x01\x00\x04\x00".to_vec();
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    // Writing to memory does not fail.
    let _ = enc.write_all(data);
    b.extend_from_slice(&enc.finish().unwrap_or_default());
    b
}

/// A `LIST cmpr` holding `nodes` compressed: the two (compressed,
/// inflated) size pairs, then the chunk stream's `CPng` block and the size
/// pool's, as RIFF files (and version 14 and 15 RIFF streams) hold them.
/// With no nodes both streams are empty.
fn cmpr(nodes: &[Node]) -> Node {
    let mut stream = Vec::new();
    let mut pool = Vec::new();
    serialize_indexed(nodes, &mut stream, &mut pool);
    let pool: Vec<u8> = pool.iter().flat_map(|s| s.to_le_bytes()).collect();
    let (a, b) = (cpng(&stream), cpng(&pool));
    let mut body = Vec::with_capacity(16 + a.len() + b.len());
    for (c, u) in [(a.len(), stream.len()), (b.len(), pool.len())] {
        body.extend_from_slice(&(c as u32).to_le_bytes());
        body.extend_from_slice(&(u as u32).to_le_bytes());
    }
    body.extend_from_slice(&a);
    body.extend_from_slice(&b);
    Node::RawList(*b"cmpr", body)
}

/// `sumi`, the last chunk of RIFF files (before version 14): the box of
/// the first page's drawing (left, top, right, bottom in coordinate units
/// from the page centre), two u16 1, a flag (0), the time saved (0 here),
/// two zero words, the version x 100, a build number (0), from version
/// 10 a zero word, the locale (1033) and a zero word, and from 12 the
/// version again: 44, 56 or 60 bytes as files of versions 8, 10 and 12 to
/// 13 hold.
fn sumi(v: Ver, doc: &Document) -> Vec<u8> {
    let (bbox, centre) = doc
        .pages
        .first()
        .map(|p| {
            let b = doc
                .content_bounds(p.id)
                .ok()
                .flatten()
                .unwrap_or(Rect::ZERO);
            (b, Point::new(p.size.width / 2.0, p.size.height / 2.0))
        })
        .unwrap_or((Rect::ZERO, Point::ZERO));
    let u = |mm: f64| (mm * UNITS_PER_MM).round() as i32;
    let mut b = Buf::default();
    b.i32(u(bbox.x0 - centre.x))
        .i32(u(bbox.y1 - centre.y))
        .i32(u(bbox.x1 - centre.x))
        .i32(u(bbox.y0 - centre.y));
    b.u16(1).u16(1).u32(0).u32(0).u32(0).u32(0);
    b.u32(u32::from(v.0) * 100).u32(0);
    if v.0 >= 10 {
        b.u32(0).u32(1033).u32(0);
    }
    if v.0 >= 12 {
        b.u32(u32::from(v.0) * 100);
    }
    b.done()
}

/// RIFF streams before version 16 keep their contents in two compressed
/// lists after the picture and info: the first with the bitmaps (or
/// nothing), the second with the document tables and the pages.
fn compress_riff(tree: Vec<Node>) -> Vec<Node> {
    const INFO: [u8; 4] = *b"INFO";
    const BMPT: [u8; 4] = *b"bmpt";
    let mut head = Vec::new();
    let mut bitmaps = Vec::new();
    let mut body = Vec::new();
    for n in tree {
        match &n {
            Node::Chunk(id, _) if [b"vrsn", b"DISP"].contains(&id) => head.push(n),
            Node::List(INFO, _) => head.push(n),
            Node::List(BMPT, _) => bitmaps.push(n),
            _ => body.push(n),
        }
    }
    head.push(cmpr(&bitmaps));
    head.push(cmpr(&body));
    head
}

/// A RIFF file of the form type and chunks.
fn riff_file(form: [u8; 4], nodes: &[Node]) -> Vec<u8> {
    let mut body = form.to_vec();
    serialize(nodes, &mut body);
    let mut out = Vec::with_capacity(body.len() + 8);
    put_chunk(&mut out, b"RIFF", &body);
    out
}

/// The data files of a version 16+ archive, in the order they are listed.
#[derive(Default)]
struct DataFiles {
    names: Vec<String>,
    data: Vec<Vec<u8>>,
}

impl DataFiles {
    fn index(&mut self, name: &str) -> usize {
        match self.names.iter().position(|n| n == name) {
            Some(i) => i,
            None => {
                self.names.push(name.to_string());
                self.data.push(Vec::new());
                self.names.len() - 1
            }
        }
    }
}

/// Version 16+: the RIFF stream with every chunk a 16-byte record. Up to
/// 8 bytes stay in the record (stream 0xFFFFFFFF); larger payloads go to
/// the data file of their top-level list (the bitmaps, the document
/// tables, the master page, each page).
fn pack_redirected(v: Ver, tree: &[Node]) -> (Vec<u8>, DataFiles) {
    let mut files = DataFiles::default();
    let mut body = v.form().to_vec();
    let mut page = 0usize;
    for node in tree {
        let file = match node {
            Node::List(kind, _) => match kind {
                b"bmpt" => Some("Bitmaps.dat".to_string()),
                b"doc " => Some("data1.dat".to_string()),
                b"page" => {
                    page += 1;
                    Some(if page == 1 {
                        "masterPage.dat".to_string()
                    } else {
                        format!("page{}.dat", page - 1)
                    })
                }
                _ => None,
            },
            _ => None,
        };
        let index = file.map(|f| files.index(&f));
        redirect(std::slice::from_ref(node), index, &mut files, &mut body);
    }
    let mut out = Vec::with_capacity(body.len() + 8);
    put_chunk(&mut out, b"RIFF", &body);
    (out, files)
}

fn redirect(nodes: &[Node], file: Option<usize>, files: &mut DataFiles, out: &mut Vec<u8>) {
    for n in nodes {
        match n {
            Node::List(kind, children) => {
                let mut body = kind.to_vec();
                redirect(children, file, files, &mut body);
                put_chunk(out, b"LIST", &body);
            }
            Node::RawList(kind, data) => {
                let mut body = kind.to_vec();
                body.extend_from_slice(data);
                put_chunk(out, b"LIST", &body);
            }
            Node::Chunk(id, data) => {
                let mut rec = Buf::default();
                match file.and_then(|f| files.data.get_mut(f).map(|d| (f, d))) {
                    Some((f, d)) if data.len() > 8 => {
                        let ofs = d.len();
                        d.extend_from_slice(data);
                        rec.u32(f as u32)
                            .u32(data.len() as u32)
                            .u32(ofs as u32)
                            .u32(0);
                    }
                    _ => {
                        let mut inline = data.clone();
                        inline.resize(8, 0);
                        rec.u32(u32::MAX)
                            .u32(data.len().min(8) as u32)
                            .bytes(&inline[..8]);
                    }
                }
                put_chunk(out, id, &rec.done());
            }
        }
    }
}

/// `DISP`: the picture of a RIFF file (before version 14): a u32 8, then a
/// device-independent bitmap of the drawing fitted into a square on white,
/// 96 pixels and 8 bits with a 256-colour palette before version 13, 128
/// pixels and 24 bits from 13, as files of those versions hold.
fn disp(v: Ver, png: &[u8]) -> Option<Vec<u8>> {
    let pm = tiny_skia::Pixmap::decode_png(png).ok()?;
    let (sw, sh) = (pm.width() as f64, pm.height() as f64);
    let true_colour = v.0 >= 13;
    let side: usize = if true_colour { 128 } else { 96 };
    let scale = (side as f64 / sw).min(side as f64 / sh);
    let (dw, dh) = (sw * scale, sh * scale);
    let (ox, oy) = ((side as f64 - dw) / 2.0, (side as f64 - dh) / 2.0);
    let px = pm.pixels();
    // The picture's colour at square pixel (x, y), top-down, on white.
    let sample = |x: usize, y: usize| -> [u8; 3] {
        let fx = (x as f64 + 0.5 - ox) / scale;
        let fy = (y as f64 + 0.5 - oy) / scale;
        if fx < 0.0 || fy < 0.0 || fx >= sw || fy >= sh {
            return [255, 255, 255];
        }
        let i = fy as usize * pm.width() as usize + fx as usize;
        match px.get(i) {
            Some(p) => {
                let c = p.demultiply();
                let a = u32::from(c.alpha());
                let w = |v: u8| ((u32::from(v) * a + 255 * (255 - a)) / 255) as u8;
                [w(c.red()), w(c.green()), w(c.blue())]
            }
            None => [255, 255, 255],
        }
    };
    let bpp: u16 = if true_colour { 24 } else { 8 };
    let row = (side * usize::from(bpp) / 8).div_ceil(4) * 4;
    let colours: u32 = if true_colour { 0 } else { 256 };
    let mut b = Vec::new();
    b.extend_from_slice(&8u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(side as i32).to_le_bytes());
    b.extend_from_slice(&(side as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&bpp.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&((row * side) as u32).to_le_bytes());
    b.extend_from_slice(&3780i32.to_le_bytes());
    b.extend_from_slice(&3780i32.to_le_bytes());
    b.extend_from_slice(&colours.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    // The 8-bit palette: a 6 x 6 x 6 colour cube, then 40 greys.
    let level = |i: usize| (i * 51) as u8;
    if !true_colour {
        for i in 0..216 {
            let (r, g, bl) = (level(i / 36), level(i / 6 % 6), level(i % 6));
            b.extend_from_slice(&[bl, g, r, 0]);
        }
        for i in 0..40u32 {
            let g = (i * 255 / 39) as u8;
            b.extend_from_slice(&[g, g, g, 0]);
        }
    }
    let nearest = |c: [u8; 3]| -> u8 {
        let cube = |v: u8| (u32::from(v) + 25) / 51;
        let idx = cube(c[0]) * 36 + cube(c[1]) * 6 + cube(c[2]);
        let grey = (u32::from(c[0]) + u32::from(c[1]) + u32::from(c[2])) / 3;
        let spread = c
            .iter()
            .map(|v| (i32::from(*v) - grey as i32).abs())
            .max()
            .unwrap_or(0);
        if spread < 8 {
            (216 + (grey * 39 + 127) / 255) as u8
        } else {
            idx as u8
        }
    };
    for y in (0..side).rev() {
        let start = b.len();
        for x in 0..side {
            let c = sample(x, y);
            if true_colour {
                b.extend_from_slice(&[c[2], c[1], c[0]]);
            } else {
                b.push(nearest(c));
            }
        }
        b.resize(start + row, 0);
    }
    Some(b)
}

/// A PNG picture as a 24-bit bottom-up BMP file on white (the thumbnails
/// of version 14 and 15 packages), or `None` when it cannot be decoded.
fn png_to_bmp(png: &[u8]) -> Option<Vec<u8>> {
    let pm = tiny_skia::Pixmap::decode_png(png).ok()?;
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let row = (w * 3).div_ceil(4) * 4;
    let size = 54 + row * h;
    let mut b = Vec::with_capacity(size);
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&(size as u32).to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&54u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(w as i32).to_le_bytes());
    b.extend_from_slice(&(h as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&24u16.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&((row * h) as u32).to_le_bytes());
    // 72 pixels per inch, no palette.
    b.extend_from_slice(&2835i32.to_le_bytes());
    b.extend_from_slice(&2835i32.to_le_bytes());
    b.extend_from_slice(&[0; 8]);
    let px = pm.pixels();
    for y in (0..h).rev() {
        let start = b.len();
        for x in 0..w {
            let c = px.get(y * w + x)?.demultiply();
            let a = u32::from(c.alpha());
            let on_white = |v: u8| ((u32::from(v) * a + 255 * (255 - a)) / 255) as u8;
            b.extend_from_slice(&[on_white(c.blue()), on_white(c.green()), on_white(c.red())]);
        }
        b.resize(start + row, 0);
    }
    Some(b)
}

/// `metadata/metadata.xml` of version 14 and 15 packages: a summary of
/// the drawing (fonts used, title, the version that wrote it, pages,
/// layers and the first page's size in coordinate units).
fn summary_xml(v: Ver, doc: &Document) -> String {
    let mut fonts: Vec<String> = Vec::new();
    fn walk(shapes: &[tracedraw_core::Shape], fonts: &mut Vec<String>) {
        for s in shapes {
            match &s.kind {
                tracedraw_core::ShapeKind::Text { spans, .. } => {
                    for sp in spans {
                        if !fonts.contains(&sp.font_family) {
                            fonts.push(sp.font_family.clone());
                        }
                    }
                }
                tracedraw_core::ShapeKind::Group { children } => walk(children, fonts),
                _ => {}
            }
        }
    }
    let mut layers = 0;
    for p in &doc.pages {
        for l in &p.layers {
            layers += 1;
            walk(&l.shapes, &mut fonts);
        }
    }
    fonts.sort();
    let bag = if fonts.is_empty() {
        "                <rdf:Bag />\n".to_string()
    } else {
        let mut b = String::from("                <rdf:Bag>\n");
        for f in &fonts {
            b.push_str(&format!(
                "                    <rdf:li>{}</rdf:li>\n",
                xml_escape(f)
            ));
        }
        b.push_str("                </rdf:Bag>\n");
        b
    };
    let (w, h) = doc
        .pages
        .first()
        .map(|p| (p.size.width, p.size.height))
        .unwrap_or((0.0, 0.0));
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n    <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n        <rdf:Description rdf:about=\"\" xmlns=\"{TEXTINFO_NS}\">\n            <FontsUsed>\n{bag}            </FontsUsed>\n            <Title>{}</Title>\n            <AppVersion>{ver}</AppVersion>\n            <CoreVersion>{ver}</CoreVersion>\n            <NumPages>{}</NumPages>\n            <NumLayers>{layers}</NumLayers>\n            <PageWidth>{}</PageWidth>\n            <PageHeight>{}</PageHeight>\n        </rdf:Description>\n    </rdf:RDF>\n</x:xmpmeta>\n",
        xml_escape(&doc.title),
        doc.pages.len(),
        (w * UNITS_PER_MM).round() as i64,
        (h * UNITS_PER_MM).round() as i64,
        ver = u32::from(v.0) * 100,
    )
}

/// Versions 14 and 15: the RIFF stream stored, its pictures as BMP files,
/// a summary, the names and an empty link list, as files of these versions
/// hold them.
fn zip_v14(v: Ver, doc: &Document, riff: &[u8], previews: &Previews) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let deflated = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut ok =
        zip.start_file("content/riffData.cdr", stored).is_ok() && zip.write_all(riff).is_ok();
    let thumbnail = previews
        .thumbnail
        .as_ref()
        .and_then(|p| png_to_bmp(&p.data));
    if let Some(bmp) = thumbnail {
        ok &= zip
            .start_file("metadata/thumbnails/thumbnail.bmp", deflated)
            .is_ok()
            && zip.write_all(&bmp).is_ok();
    }
    for (i, p) in previews.pages.iter().enumerate() {
        if let Some(bmp) = png_to_bmp(&p.data) {
            ok &= zip
                .start_file(format!("metadata/thumbnails/page{}.bmp", i + 1), deflated)
                .is_ok()
                && zip.write_all(&bmp).is_ok();
        }
    }
    let textinfo = textinfo_xml(doc);
    let members: [(&str, String); 3] = [
        ("metadata/metadata.xml", summary_xml(v, doc)),
        ("metadata/textinfo.xml", textinfo),
        (
            "links.xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<linkInfo />\n".to_string(),
        ),
    ];
    for (name, text) in members {
        ok &= zip.start_file(name, deflated).is_ok() && zip.write_all(text.as_bytes()).is_ok();
    }
    match zip.finish() {
        Ok(c) if ok => c.into_inner(),
        _ => Vec::new(),
    }
}

// ---- version 16+ package ---------------------------------------------
//
// Version 16 and later files are a package: a `mimetype` member first and
// stored (not compressed), the drawing under `content/`, pictures under
// `previews/`, colour settings under `color/` and a `META-INF/` folder
// whose `container.xml` names the drawing's root file (the layout of an
// open container format). The strings below are the ones files of these
// versions hold; programs reading such a package look for them.

/// The package's media type, the whole of its `mimetype` member.
const PACKAGE_MEDIA_TYPE: &str = "application/x-vnd.corel.zcf.draw.document+zip";
/// The media type `container.xml` gives the root file.
const ROOT_MEDIA_TYPE: &str = "application/x-vnd.corel.cdr.rootdata";
/// The namespace of the package's own attributes in `container.xml`.
const CONTAINER_NS: &str = "urn:corel:namespaces:zcf:xmlns:container";
/// The namespaces of `metadata.xml` and `textinfo.xml`.
const METADATA_NS: &str = "http://namespace.corel.com/cdr/metadata/1.0/";
const TEXTINFO_NS: &str = "http://namespace.corel.com/Cdr/metadata";

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

/// `META-INF/container.xml`: the root file, then the thumbnail and the
/// page pictures stored.
fn container_xml(doc: &Document, previews: &Previews) -> String {
    let mut x = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n");
    x.push_str(&format!(
        "<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\" xmlns:crl=\"{CONTAINER_NS}\">\n    <rootfiles>\n"
    ));
    x.push_str(&format!(
        "        <rootfile crl:file-kind=\"main\" full-path=\"content/root.dat\" media-type=\"{ROOT_MEDIA_TYPE}\" />\n"
    ));
    if let Some(t) = &previews.thumbnail {
        x.push_str(&format!(
            "        <rootfile crl:file-kind=\"thumbnail\" crl:image-height=\"{}\" crl:image-width=\"{}\" full-path=\"previews/thumbnail.png\" media-type=\"image/png\" />\n",
            t.height, t.width
        ));
    }
    for (i, p) in previews.pages.iter().enumerate() {
        let name = doc
            .pages
            .get(i)
            .map(|pg| pg.name.clone())
            .unwrap_or_else(|| format!("Page {}", i + 1));
        let name = xml_escape(&name);
        x.push_str(&format!(
            "        <rootfile crl:caption=\":[{name}];\" crl:file-kind=\"page\" crl:image-height=\"{}\" crl:image-width=\"{}\" crl:index=\"{}\" full-path=\"previews/page{}.png\" media-type=\"image/png\" />\n",
            p.height,
            p.width,
            i + 1,
            i + 1
        ));
    }
    x.push_str("    </rootfiles>\n</container>\n");
    x
}

/// `META-INF/metadata.xml`: the format and the version that wrote the
/// file (`CoreVersion`, in hundreds) and the title.
fn metadata_xml(v: Ver, doc: &Document) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n    <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n        <rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:cdr=\"{METADATA_NS}\">\n            <dc:format>{PACKAGE_MEDIA_TYPE}</dc:format>\n            <cdr:CoreVersion>{}</cdr:CoreVersion>\n            <dc:title>\n                <rdf:Alt>\n                    <rdf:li xml:lang=\"x-default\">{}</rdf:li>\n                </rdf:Alt>\n            </dc:title>\n        </rdf:Description>\n    </rdf:RDF>\n</x:xmpmeta>\n",
        u32::from(v.0) * 100,
        xml_escape(&doc.title)
    )
}

/// `META-INF/textinfo.xml`: the names of objects and layers (pages keep
/// theirs in the drawing; their default names are not listed).
fn textinfo_xml(doc: &Document) -> String {
    let mut objects: Vec<String> = Vec::new();
    let mut layers: Vec<String> = Vec::new();
    fn names(shapes: &[tracedraw_core::Shape], out: &mut Vec<String>) {
        for s in shapes {
            if let Some(n) = s.name.as_deref().filter(|n| !n.is_empty()) {
                out.push(n.to_string());
            }
            if let tracedraw_core::ShapeKind::Group { children } = &s.kind {
                names(children, out);
            }
        }
    }
    for p in &doc.pages {
        for l in &p.layers {
            if !layers.contains(&l.name) {
                layers.push(l.name.clone());
            }
            names(&l.shapes, &mut objects);
        }
    }
    objects.sort();
    objects.dedup();
    let bag = |items: &[String]| -> String {
        if items.is_empty() {
            "                <rdf:Bag />\n".to_string()
        } else {
            let mut b = String::from("                <rdf:Bag>\n");
            for i in items {
                b.push_str(&format!(
                    "                    <rdf:li>{}</rdf:li>\n",
                    xml_escape(i)
                ));
            }
            b.push_str("                </rdf:Bag>\n");
            b
        }
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n    <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n        <rdf:Description rdf:about=\"\" xmlns=\"{TEXTINFO_NS}\">\n            <ObjectNames>\n{}            </ObjectNames>\n            <LayerNames>\n{}            </LayerNames>\n            <PageNames>\n                <rdf:Bag />\n            </PageNames>\n        </rdf:Description>\n    </rdf:RDF>\n</x:xmpmeta>\n",
        bag(&objects),
        bag(&layers)
    )
}

/// `styles/document.cdss`: the document's default styles (graphic,
/// artistic and paragraph text, artistic media, callouts, dimensions, bar
/// and QR codes) and its colour styles, as JSON. Every file of versions 17
/// to 26 holds these styles with the same fields; the values are the ones
/// most files hold (black fill and hairline outline, 12 pt text, ...),
/// and `version` is the file's (in hundreds).
const DOCUMENT_STYLES: &str = include_str!("document_styles.json");

fn document_styles(v: Ver) -> String {
    DOCUMENT_STYLES.replacen(
        "\"version\": 0,",
        &format!("\"version\": {},", u32::from(v.0) * 100),
        1,
    )
}

/// `color/docPalette.xml`: the document palette (empty: the colours a
/// drawing uses live in its fills).
const DOC_PALETTE_XML: &str = "<?xml version=\"1.0\"?>\r\n<palette guid=\"3f8e2a71-6c4d-4b19-a0e5-9d2c7b61f483\" name=\"Document Palette\"/>\r\n";

/// `color/color.xml` naming the document's CMYK profile, when it carries
/// one (stored under `color/profiles/cmyk/`), before the colour model, as
/// files with profiles have it.
fn color_xml(doc: &Document) -> String {
    match &doc.cmyk_profile {
        Some(p) => COLOR_XML.replacen(
            "<ColorContext>",
            &format!(
                "<ColorContext><ColorProfile id=\"Cmyk\">{}</ColorProfile>",
                xml_escape(&p.name)
            ),
            1,
        ),
        None => COLOR_XML.to_string(),
    }
}

/// The member a CMYK profile is stored as: its name in lower-case letters
/// and digits (`uswebcoatedswopv2.icc`).
fn cmyk_profile_member(name: &str) -> String {
    let file: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let file = if file.is_empty() {
        "profile".to_string()
    } else {
        file
    };
    format!("color/profiles/cmyk/{file}.icc")
}

/// `color/color.xml`: the colour model of the document and which kinds of
/// colour its objects use.
const COLOR_XML: &str = "<Color><ColorContext><ColorModel>Cmyk</ColorModel><RenderingIntent>RelativeColorimetric</RenderingIntent></ColorContext><HasRgbObjects>true</HasRgbObjects><HasCmykObjects>true</HasCmykObjects><HasGrayscaleObjects>false</HasGrayscaleObjects></Color>\r\n";

fn zip_v16(
    v: Ver,
    doc: &Document,
    root: &[u8],
    files: &DataFiles,
    previews: &Previews,
    font_ids: &[(u16, String)],
) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let deflated = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut ok = zip.start_file("mimetype", stored).is_ok()
        && zip.write_all(PACKAGE_MEDIA_TYPE.as_bytes()).is_ok();
    for (name, data) in files.names.iter().zip(&files.data) {
        ok &= zip
            .start_file(format!("content/data/{name}"), deflated)
            .is_ok()
            && zip.write_all(data).is_ok();
    }
    for (name, data) in fonts::members(&previews.fonts, previews.embed_fonts, font_ids) {
        ok &= zip.start_file(name, deflated).is_ok() && zip.write_all(&data).is_ok();
    }
    ok &= zip.start_file("content/dataFileList.dat", deflated).is_ok()
        && zip.write_all(files.names.join("\n").as_bytes()).is_ok();
    ok &= zip.start_file("content/root.dat", deflated).is_ok() && zip.write_all(root).is_ok();
    if let Some(t) = &previews.thumbnail {
        ok &= zip.start_file("previews/thumbnail.png", deflated).is_ok()
            && zip.write_all(&t.data).is_ok();
    }
    for (i, p) in previews.pages.iter().enumerate() {
        ok &= zip
            .start_file(format!("previews/page{}.png", i + 1), deflated)
            .is_ok()
            && zip.write_all(&p.data).is_ok();
    }
    ok &= zip.start_file("color/color.xml", deflated).is_ok()
        && zip.write_all(color_xml(doc).as_bytes()).is_ok();
    if let Some(p) = &doc.cmyk_profile {
        ok &= zip
            .start_file(cmyk_profile_member(&p.name), deflated)
            .is_ok()
            && zip.write_all(&p.data).is_ok();
    }
    let members: [(&str, String); 6] = [
        ("META-INF/textinfo.xml", textinfo_xml(doc)),
        ("color/docPalette.xml", DOC_PALETTE_XML.to_string()),
        ("styles/document.cdss", document_styles(v)),
        ("META-INF/metadata.xml", metadata_xml(v, doc)),
        (
            "META-INF/links.xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<linkInfo />\n".to_string(),
        ),
        ("META-INF/container.xml", container_xml(doc, previews)),
    ];
    for (name, text) in members {
        ok &= zip.start_file(name, deflated).is_ok() && zip.write_all(text.as_bytes()).is_ok();
    }
    match zip.finish() {
        Ok(c) if ok => c.into_inner(),
        _ => Vec::new(),
    }
}

// ---- records -----------------------------------------------------------

/// 12-byte colour record: model, palette, 4 unknown bytes, 4 values. CMYK
/// keeps its percentages (model 2), grey its level (model 9); everything
/// else is written as sRGB (model 5, stored B G R). A spot colour is
/// written as its process stand-in.
fn colour(c: Color) -> [u8; 12] {
    let (model, values) = match c.process() {
        Color::Cmyk { c, m, y, k } => {
            let pct = |x: f32| (x.clamp(0.0, 1.0) * 100.0).round() as u8;
            (2u16, [pct(c), pct(m), pct(y), pct(k)])
        }
        Color::Gray { v } => (9, [(v.clamp(0.0, 1.0) * 255.0).round() as u8, 0, 0, 0]),
        Color::Registration => (2, [100, 100, 100, 100]),
        other => {
            let [r, g, b] = other.to_rgb8();
            (5, [b, g, r, 0])
        }
    };
    let mut out = [0u8; 12];
    out[0..2].copy_from_slice(&model.to_le_bytes());
    out[2..4].copy_from_slice(&5u16.to_le_bytes());
    out[8..12].copy_from_slice(&values);
    out
}

/// Version 13+: a colour as a property list (type 1, then from version 15
/// an empty palette entry id, type 7), closed by an empty record.
fn colour_props(v: Ver, c: Color) -> Vec<u8> {
    let mut b = Buf::default();
    b.u8(1).u32(12).bytes(&colour(c));
    if v.0 >= 15 {
        b.u8(7).u32(16).zeros(16);
    }
    b.u8(0).u32(0);
    b.done()
}

/// The master page's `loda` arguments for a solid page background, as
/// version 21 files hold them: 17000, a record tagged 1300 of 651 bytes (1
/// at its start and at byte 12, the colour's property list at byte 568);
/// 12000, zero; and an empty style string.
fn page_background_args(v: Ver, c: Color) -> Vec<(u32, Vec<u8>)> {
    let mut body = vec![0u8; 651];
    body[0] = 1;
    body[12] = 1;
    let props = colour_props(v, c);
    body[568..568 + props.len()].copy_from_slice(&props);
    let mut rec = Buf::default();
    rec.u32(0x514).u32(body.len() as u32).bytes(&body);
    let mut style = Buf::default();
    style.u32(1).u32(2).bytes(b"{}");
    vec![
        (17000, rec.done()),
        (12000, vec![0; 4]),
        (201, style.done()),
    ]
}

/// A colour as style strings write it.
fn colour_spec(c: Color) -> String {
    const NO_ID: &str = "00000000-0000-0000-0000-000000000000";
    match c {
        Color::Cmyk { c, m, y, k } => {
            let pct = |x: f32| (x.clamp(0.0, 1.0) * 100.0).round() as u8;
            format!(
                "CMYK,USER,{},{},{},{},100,{NO_ID}",
                pct(c),
                pct(m),
                pct(y),
                pct(k)
            )
        }
        other => {
            let [r, g, b] = other.to_rgb8();
            format!("RGB255,USER,{r},{g},{b},100,{NO_ID}")
        }
    }
}

/// The colour a fill shows as one colour (gradients: their first stop).
fn fill_colour(fill: &Fill) -> Option<Color> {
    match fill {
        Fill::None => None,
        Fill::Solid(c) => Some(*c),
        Fill::Fountain(f) => f.stops.first().map(|s| s.color),
        other => stand_in_colour(other),
    }
}

/// The one colour written for a fill these files cannot hold yet: the
/// mean colour of a bitmap pattern's tile, the middle of a texture's or a
/// two-colour pattern's colours, else its preview colour.
fn stand_in_colour(fill: &Fill) -> Option<Color> {
    use tracedraw_core::style::{lerp_color, Pattern};
    match fill {
        Fill::Pattern(Pattern::Bitmap { png, .. }) => {
            let pm = tiny_skia::Pixmap::decode_png(png).ok()?;
            let (mut sum, mut weight) = ([0.0f64; 3], 0.0f64);
            for p in pm.pixels() {
                let p = p.demultiply();
                let a = p.alpha() as f64 / 255.0;
                sum[0] += p.red() as f64 * a;
                sum[1] += p.green() as f64 * a;
                sum[2] += p.blue() as f64 * a;
                weight += a;
            }
            if weight <= 0.0 {
                return fill.preview_color();
            }
            let c = |v: f64| (v / weight).round().clamp(0.0, 255.0) as u8;
            Some(Color::rgb8(c(sum[0]), c(sum[1]), c(sum[2])))
        }
        Fill::Pattern(Pattern::TwoColor { front, back, .. }) => {
            Some(lerp_color(*front, *back, 0.5))
        }
        Fill::Texture(t) => Some(lerp_color(t.color_a, t.color_b, 0.5)),
        other => other.preview_color(),
    }
}

/// `pref` (version 16+, after `mcfg`): the document's preferences as
/// pairs of NUL-terminated UTF-16 strings, a key and its value. Files hold
/// up to a hundred and fifty; the three every file of versions 17 to 26
/// holds are written, with the values they hold.
const PREFERENCES: [(&str, &str); 3] = [
    ("Options - General/EmbedFontsOnSave", "1"),
    ("Options - View/SimulateOverprints", "0"),
    ("Options - View/ViewQuality", "4"),
];

fn pref() -> Vec<u8> {
    let mut b = Vec::new();
    for (k, val) in PREFERENCES {
        for s in [k, val] {
            for u in s.encode_utf16() {
                b.extend_from_slice(&u.to_le_bytes());
            }
            b.extend_from_slice(&[0, 0]);
        }
    }
    b
}

fn fountain_type(kind: FountainKind) -> u8 {
    match kind {
        FountainKind::Linear => 1,
        FountainKind::Radial => 2,
        FountainKind::Conical => 3,
        // A shaped gradient is written as a square one (`Fountain::to_plain`).
        FountainKind::Square | FountainKind::Shaped => 4,
    }
}

/// `fild`: a fill record. See `docs/cdr-format.md` for both layouts.
fn fild(v: Ver, id: u32, fill: &Fill) -> Vec<u8> {
    let fill = match fill {
        Fill::Fountain(f) => {
            // Repeats, mirror, inversion and the shaped kind have no field
            // here: the closest plain gradient carries them in its stops.
            let mut f = f.to_plain(64);
            crate::mirror_centred_stops(&mut f);
            Fill::Fountain(f)
        }
        Fill::None | Fill::Solid(_) => fill.clone(),
        other => stand_in_colour(other)
            .map(Fill::Solid)
            .unwrap_or(Fill::None),
    };
    let mut b = Buf::default();
    if !v.tagged() {
        b.u32(id);
        match &fill {
            Fill::Solid(c) => {
                b.u16(1).zeros(2).bytes(&colour(*c)).zeros(8).bytes(&SCREEN);
            }
            Fill::Fountain(f) => {
                b.u16(2).zeros(2).u8(fountain_type(f.kind));
                b.zeros(11).bytes(&SCREEN);
                b.i32((f.edge_pad * 100.0).round() as i32)
                    .i32((f.angle * 1_000_000.0).round() as i32)
                    .i32((f.offset.x * 100.0).round() as i32)
                    .i32((f.offset.y * 100.0).round() as i32)
                    .zeros(2)
                    .bytes(&[3, 0, 1, 0])
                    .u8(50)
                    .u8(0);
                let stops: Vec<_> = f.stops.iter().take(64).collect();
                b.u32(stops.len() as u32);
                for s in stops {
                    b.bytes(&colour(s.color))
                        .u32((s.pos.clamp(0.0, 1.0) * 100.0).round() as u32);
                }
            }
            _ => {
                b.u16(0).zeros(2);
            }
        }
        return b.done();
    }
    // Version 13+: id, tag, length, then the body.
    let mut body = Buf::default();
    let late = v.0 >= 16;
    match &fill {
        Fill::Solid(c) => {
            let mut inner = Buf::default();
            inner.bytes(&colour_props(v, *c)).zeros(8).bytes(&SCREEN);
            if !late {
                inner.zeros(8);
            }
            let inner = inner.done();
            body.u16(1)
                .u32(SINCE_13)
                .u32(inner.len() as u32)
                .bytes(&inner)
                .zeros(8);
        }
        Fill::Fountain(f) => {
            let mut inner = Buf::default();
            inner
                .u8(fountain_type(f.kind))
                .bytes(&[0, 0, 0, 1, 0x51, 0, 0, 0, 0])
                .bytes(&SCREEN)
                .u16((f.edge_pad * 100.0).round().clamp(0.0, 49.0) as u16)
                .i32((f.angle * 1_000_000.0).round() as i32)
                .i32((f.offset.x * 100.0).round() as i32)
                .i32((f.offset.y * 100.0).round() as i32)
                .zeros(2)
                .bytes(&[3, 0, 1, 0])
                .u8(50)
                .u8(0);
            let stops: Vec<_> = f.stops.iter().take(64).collect();
            inner.u16(stops.len() as u16);
            for s in stops {
                inner.u8(1).u32(12).bytes(&colour(s.color));
                if v.0 >= 15 {
                    inner.u8(7).u32(16).zeros(16);
                }
                inner
                    .zeros(5)
                    .u16((s.pos.clamp(0.0, 1.0) * 100.0).round() as u16);
            }
            inner.zeros(8);
            if late {
                // The fill's place and size relative to the object.
                inner.f64(0.0).f64(0.0).f64(1.0).f64(1.0);
            }
            let inner = inner.done();
            body.u16(2)
                .u32(SINCE_13)
                .u32(inner.len() as u32)
                .bytes(&inner)
                .zeros(8);
        }
        _ => {
            body.u16(0);
            if !late {
                body.zeros(8);
            }
        }
    }
    let body = body.done();
    b.u32(id).u32(SINCE_13).u32(body.len() as u32).bytes(&body);
    if late {
        b.zeros(8);
    }
    b.done()
}

/// Bytes of the identity matrix as six doubles.
fn identity_matrix() -> Vec<u8> {
    let mut b = Buf::default();
    for x in [1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        b.f64(x);
    }
    b.done()
}

/// `outl`: an outline record. Before version 13 the outline proper; from
/// 13 tagged records (3: a value of 5000000, 2: the colour, 1: the
/// outline proper) and 8 closing bytes.
fn outl(v: Ver, id: u32, s: Option<&Stroke>) -> Vec<u8> {
    let mut o = Buf::default();
    let (line_type, caps, join, width, stretch, angle, color, dash) = match s {
        None => (
            1u16,
            0u16,
            0u16,
            0i32,
            100u16,
            0i32,
            Color::BLACK,
            Vec::new(),
        ),
        Some(s) => {
            let mut t = 0x02u16;
            if s.scale_with_object {
                t |= 0x20;
            }
            if s.behind_fill {
                t |= 0x10;
            }
            let caps = match s.cap {
                LineCap::Butt => 0,
                LineCap::Round => 1,
                LineCap::Square => 2,
            };
            let join = match s.join {
                LineJoin::Miter => 0,
                LineJoin::Round => 1,
                LineJoin::Bevel => 2,
            };
            let width = if s.width <= Stroke::HAIRLINE + 1e-9 {
                0
            } else {
                units(s.width)
            };
            let dash: Vec<u16> = s
                .dash
                .iter()
                .take(10)
                .map(|d| d.round().clamp(0.0, 65535.0) as u16)
                .collect();
            (
                t,
                caps,
                join,
                width,
                (s.stretch.clamp(0.01, 1.0) * 100.0).round() as u16,
                (s.nib_angle * 1_000_000.0).round() as i32,
                s.color,
                dash,
            )
        }
    };
    o.u16(line_type).u16(caps).u16(join);
    if !v.tagged() {
        o.zeros(2);
    }
    o.i32(width).u16(stretch);
    if !v.tagged() {
        o.zeros(2);
    }
    o.i32(angle);
    if !v.tagged() {
        o.zeros(4);
    }
    o.bytes(&identity_matrix())
        .bytes(&colour(color))
        .zeros(8)
        .bytes(&SCREEN)
        .u16(dash.len() as u16);
    let dashes_at = o.len();
    for d in &dash {
        o.u16(*d);
    }
    // The count and ten dash slots fill 22 bytes; the two marker ids
    // follow (none), then 2 bytes we leave zero.
    o.pad_to(dashes_at + 20);
    o.u32(0).u32(0).zeros(2);
    let proper = o.done();
    let mut b = Buf::default();
    b.u32(id);
    if !v.tagged() {
        b.bytes(&proper).zeros(4);
        return b.done();
    }
    let mut proper = proper;
    proper.resize(130, 0);
    let col = colour_props(v, color);
    if v.0 == 15 {
        b.u32(5).u32(48).zeros(48);
    }
    b.u32(3)
        .u32(4)
        .u32(5_000_000)
        .u32(2)
        .u32(col.len() as u32)
        .bytes(&col)
        .u32(1)
        .u32(proper.len() as u32)
        .bytes(&proper)
        .zeros(8);
    b.done()
}

/// `trfd`: one transform record (type 8): the matrix rows (a c e; b d f),
/// translation in coordinate units.
///
/// The record opens with a tag from version 13: [1300, 0] up to version
/// 17, [1740, 56] from 18 (56 = the matrix and the 8 bytes after it).
/// Files of every version accept either tag when they are read.
fn trfd(v: Ver, m: Affine) -> Vec<u8> {
    let c = m.as_coeffs();
    let mut rec = Buf::default();
    if v.0 >= 18 {
        rec.u32(TRFD_SINCE_18).u32(56);
    } else if v.tagged() {
        rec.u32(SINCE_13).u32(0);
    }
    rec.u16(8).zeros(6);
    for x in [
        c[0],
        c[2],
        c[4] * UNITS_PER_MM,
        c[1],
        c[3],
        c[5] * UNITS_PER_MM,
    ] {
        rec.f64(if x.is_finite() { x } else { 0.0 });
    }
    if v.tagged() {
        rec.zeros(8);
    }
    let rec = rec.done();
    let start = 24u32;
    let end = start + rec.len() as u32;
    let mut b = Buf::default();
    b.u32(end)
        .u32(1)
        .u32(16)
        .u32(0xffff)
        .u32(start)
        .u32(end)
        .bytes(&rec);
    b.done()
}

/// `txtj`: the line breaking tables that follow every `txsm` from version
/// 11. Files store 528 bytes: a word of 14, then three tables, each a
/// count and that many UTF-16 characters (81 at offset 4, 20 at 312, 8 at
/// 420). Files of versions 11 to 21 carry the same tables in every text;
/// the bytes between them differ from file to file and are written as
/// zeros.
fn txtj() -> Vec<u8> {
    // Offset, length, and the characters before the `?` that fill the
    // rest of the table.
    const TABLES: [(usize, usize, &str); 3] = [
        (
            4,
            81,
            "!%),.?]}??????????????????,.\u{b7}:;?!????????\u{92}\u{94})?]}>\u{bb}???\u{b0}'\"??%\u{89}",
        ),
        (312, 20, "$([\\{?\u{91}\u{93}(?[{<\u{ab}????$"),
        (420, 8, ",.????,."),
    ];
    let mut b = Buf::default();
    b.u32(14);
    for (at, n, chars) in TABLES {
        b.pad_to(at);
        b.u32(n as u32);
        for u in chars
            .encode_utf16()
            .chain(std::iter::repeat(u16::from(b'?')))
            .take(n)
        {
            b.u16(u);
        }
    }
    b.pad_to(528);
    b.done()
}

/// `ftil`: the fill's transform, the identity.
fn ftil() -> Vec<u8> {
    identity_matrix()
}

/// `mcfg`: the document configuration. Versions 7, 8 and 12 to 27 get
/// the record of files of their version, with the fields that are
/// understood (see `docs/cdr-format.md`, Writing); versions 9 to 11 get
/// the page size and zeros up to 4088 bytes.
fn mcfg(v: Ver, w_mm: f64, h_mm: f64) -> Vec<u8> {
    if v.0 >= 21 {
        return mcfg_21(w_mm, h_mm);
    }
    if v.tagged() {
        return mcfg_13(w_mm, h_mm);
    }
    if v.0 == 12 {
        return mcfg_12(w_mm, h_mm);
    }
    if v.0 <= 8 {
        return mcfg_7_8(v, w_mm, h_mm);
    }
    let mut b = Buf::default();
    let size: usize = if v.0 >= 15 {
        8323
    } else if v.tagged() {
        8197
    } else {
        4088
    };
    if v.tagged() {
        b.u32(SINCE_13).u32(size as u32 - 8).zeros(4);
    } else if v.0 >= 9 {
        b.zeros(4);
    }
    b.i32(units(w_mm))
        .i32(units(h_mm))
        .u32(0x0f)
        .u32(1)
        .pad_to(size);
    b.done()
}

/// The configuration record of versions 13 to 20 (8323 bytes, tag 1300):
/// files of those versions store the version 21 record with two more
/// blocks, 8 bytes at 869 (a scale of 1000 twice for a metric drawing,
/// 2540 for one in inches) and 371 bytes at 890 (three default text
/// settings and a few flags, the same in every file seen; their name
/// fields are left empty). A record that held only the page size made
/// the program that opens the file refuse it.
fn mcfg_13(w_mm: f64, h_mm: f64) -> Vec<u8> {
    let r21 = mcfg_21(w_mm, h_mm);
    let mut block = [0u8; 371];
    for (at, x) in [
        (3, 1u8),
        (19, 1),
        (20, 12),
        (24, 5),
        (26, 5),
        (32, 196),
        (36, 7),
        (37, 16),
        (62, 1),
        (63, 12),
        (67, 5),
        (69, 5),
        (75, 32),
        (77, 196),
        (79, 7),
        (80, 16),
        (105, 1),
        (106, 12),
        (110, 5),
        (112, 5),
        (118, 196),
        (120, 196),
        (122, 7),
        (123, 16),
        (302, 3),
        (306, 1),
        (310, 1),
        (314, 1),
        (322, 1),
        (346, 2),
        (354, 4),
        (358, 100),
        (370, 1),
    ] {
        block[at] = x;
    }
    let mut r = Vec::with_capacity(8323);
    r.extend_from_slice(r21.get(..869).unwrap_or_default());
    r.extend_from_slice(&1000u32.to_le_bytes());
    r.extend_from_slice(&1000u32.to_le_bytes());
    r.extend_from_slice(r21.get(869..882).unwrap_or_default());
    r.extend_from_slice(&block);
    r.extend_from_slice(r21.get(882..).unwrap_or_default());
    r.resize(8323, 0);
    put(&mut r, 0, &SINCE_13.to_le_bytes());
    put(&mut r, 4, &(8323u32 - 8).to_le_bytes());
    r
}

/// The configuration record of version 12 (8152 bytes, no tag): files
/// saved for version 12 hold the version 13 record without the fields
/// that version added (its tag and size, smaller text settings, a few
/// flags) and with a few more zero fields. The steps go through the
/// version 13 record: keep, skip or add zeros.
fn mcfg_12(w_mm: f64, h_mm: f64) -> Vec<u8> {
    enum Step {
        Keep(usize),
        Skip(usize),
        Zeros(usize),
    }
    use Step::{Keep, Skip, Zeros};
    const STEPS: [Step; 42] = [
        Skip(8),
        Keep(17),
        Zeros(2),
        Keep(677),
        Zeros(6),
        Keep(58),
        Skip(5),
        Keep(3),
        Skip(26),
        Keep(67),
        Zeros(2),
        Keep(6),
        Skip(8),
        Keep(34),
        Skip(5),
        Keep(9),
        Skip(31),
        Keep(15),
        Skip(31),
        Keep(11),
        Skip(26),
        Keep(155),
        Zeros(2),
        Keep(72),
        Skip(2),
        Zeros(1),
        Keep(15),
        Skip(31),
        Keep(10),
        Skip(26),
        Keep(56),
        Zeros(4),
        Keep(30),
        Zeros(4),
        Keep(9),
        Zeros(4),
        Keep(20),
        Zeros(3),
        Keep(6296),
        Zeros(4),
        Keep(560),
        Skip(4),
    ];
    let r13 = mcfg_13(w_mm, h_mm);
    let mut out = Vec::with_capacity(8152);
    let mut at = 0usize;
    for step in STEPS {
        match step {
            Keep(n) => {
                out.extend_from_slice(r13.get(at..at + n).unwrap_or_default());
                at += n;
            }
            Skip(n) => at += n,
            Zeros(n) => out.resize(out.len() + n, 0),
        }
    }
    out.resize(8152, 0);
    out
}

/// The configuration record of version 8 (656 bytes, no tag) and of
/// version 7 (536 bytes), with the values files saved for those versions
/// hold: the page size, the paper (A4 code as before), the ruler origin
/// at the page's bottom-left corner, a grid of 0.2 across and down, the
/// units, a scale of 1, the 46.5 inch square twice, the nudge (1000) and
/// the flags and default colour records after them. Version 7 holds the
/// same fields with the page colour (white) inserted after the paper and
/// 4 more bytes after the ruler origin, and stops 120 bytes earlier. A
/// record holding only the page size, followed by zeros, was what these
/// versions got before (every version 7 file written so crashed the
/// program that wrote the samples, issue #98).
fn mcfg_7_8(v: Ver, w_mm: f64, h_mm: f64) -> Vec<u8> {
    const SIZE_8: usize = 656;
    let (w, h) = (units(w_mm), units(h_mm));
    let mut r = vec![0u8; SIZE_8];
    put(&mut r, 0, &w.to_le_bytes());
    put(&mut r, 4, &h.to_le_bytes());
    put(&mut r, 8, &0x0fu32.to_le_bytes());
    put(&mut r, 12, &1u32.to_le_bytes());
    // Ruler origin: the page's bottom-left corner, from the page centre.
    put(&mut r, 24, &(-(w / 2)).to_le_bytes());
    put(&mut r, 28, &(-(h / 2)).to_le_bytes());
    put(&mut r, 32, &0.2f64.to_le_bytes());
    put(&mut r, 40, &0.2f64.to_le_bytes());
    put(&mut r, 48, &2u16.to_le_bytes());
    put(&mut r, 50, &2u16.to_le_bytes());
    put(&mut r, 56, &1.0f64.to_le_bytes());
    for (at, x) in [
        (64, 2i32),
        (68, 11_811_024),
        (72, 11_811_024),
        (76, 3),
        (88, 1_000),
        (92, 10_000),
        (100, 2),
        (444, 1),
        (448, 1),
        (452, 1),
        (456, 1),
        (460, 2),
        (468, 1),
        (476, 1),
        (500, 1),
        (504, 1),
        (512, 1),
        (524, 1),
        (528, 1),
        (532, 1),
        (536, 1),
        (592, 3),
        (600, 1),
        (604, 1),
        (608, 1),
        (616, 1),
        (636, 2),
        (644, 4),
        (648, 100),
        (652, 1),
    ] {
        put(&mut r, at, &x.to_le_bytes());
    }
    // Colour records (model 5, palette 5, black) and the values between.
    for at in [484, 552, 564, 576] {
        put(&mut r, at, &[5, 0, 5, 0]);
    }
    for (at, x) in [(560, 196u16), (572, 32), (574, 196), (584, 196), (586, 196)] {
        put(&mut r, at, &x.to_le_bytes());
    }
    if v.0 >= 8 {
        return r;
    }
    // Version 7: the page colour (model 5, palette 5, white) after the
    // paper, 4 bytes more after the ruler origin, a flag the version 8
    // record leaves zero (its byte 464) and 536 bytes in all.
    let mut r7 = Vec::with_capacity(536);
    r7.extend_from_slice(r.get(..12).unwrap_or_default());
    r7.extend_from_slice(&[5, 0, 5, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0]);
    r7.extend_from_slice(r.get(12..32).unwrap_or_default());
    r7.extend_from_slice(&[0; 4]);
    r7.extend_from_slice(r.get(32..520).unwrap_or_default());
    r7.resize(536, 0);
    put(&mut r7, 464 + 16, &1u32.to_le_bytes());
    r7
}

/// Write `bytes` into `rec` at `at` (records are built to their size
/// first, so the offsets are always inside).
fn put(rec: &mut [u8], at: usize, bytes: &[u8]) {
    if let Some(dst) = rec.get_mut(at..at + bytes.len()) {
        dst.copy_from_slice(bytes);
    }
}

/// The version 21 and 22 configuration record (7944 bytes): page size and
/// orientation, ruler origin, grid, units, duplicate offset and nudge,
/// drawing scales and the isometric drawing profile, with the values the
/// files of those versions have for a metric drawing. Text fields of the
/// record (a folder for web publishing) are left empty.
fn mcfg_21(w_mm: f64, h_mm: f64) -> Vec<u8> {
    const SIZE: usize = 7944;
    /// Units: 1 inch, 2 millimetre.
    const MM: u16 = 2;
    let mut r = vec![0u8; SIZE];
    let (w, h) = (units(w_mm), units(h_mm));
    put(&mut r, 0, &2100u32.to_le_bytes());
    put(&mut r, 4, &(SIZE as u32 - 8).to_le_bytes());
    put(&mut r, 12, &w.to_le_bytes());
    put(&mut r, 16, &h.to_le_bytes());
    // Paper preset (0: custom size) and orientation (1: landscape).
    put(&mut r, 20, &0u16.to_le_bytes());
    put(&mut r, 22, &u16::from(w > h).to_le_bytes());
    put(&mut r, 24, &1u32.to_le_bytes());
    put(&mut r, 34, &1u32.to_le_bytes());
    put(&mut r, 38, &1u32.to_le_bytes());
    // 1/8 inch, in every version 19 to 21 file seen.
    put(&mut r, 42, &31750u32.to_le_bytes());
    // Ruler origin: the page's bottom-left corner, from the page centre.
    put(&mut r, 670, &(-(w / 2)).to_le_bytes());
    put(&mut r, 674, &(-(h / 2)).to_le_bytes());
    // Grid: 4 a unit across and down, in millimetres.
    put(&mut r, 678, &4.0f64.to_le_bytes());
    put(&mut r, 686, &4.0f64.to_le_bytes());
    put(&mut r, 694, &MM.to_le_bytes());
    put(&mut r, 696, &1.0f64.to_le_bytes());
    // Horizontal and vertical units.
    put(&mut r, 704, &MM.to_le_bytes());
    put(&mut r, 706, &MM.to_le_bytes());
    put(&mut r, 708, &1u16.to_le_bytes());
    // A 46.5 inch square twice, with three flags between.
    for (at, x) in [
        (716, 11_811_024i32),
        (720, 11_811_024),
        (724, 1),
        (728, 1),
        (732, 1),
        (736, 11_811_024),
        (740, 11_811_024),
        (744, 1),
    ] {
        put(&mut r, at, &x.to_le_bytes());
    }
    put(&mut r, 760, &[0x01, 0x0c]);
    put(&mut r, 767, &[0x05]);
    put(&mut r, 777, &[0x07, 0x10]);
    put(&mut r, 811, &1u32.to_le_bytes());
    put(&mut r, 815, &1u32.to_le_bytes());
    put(&mut r, 819, &[1]);
    // Duplicate offset (5 mm across and down), nudge (0.1 mm), super
    // nudge times 2, micro nudge divided by 2.
    for (at, x) in [
        (827, 1i32),
        (831, 50_000),
        (835, 50_000),
        (839, 1_000),
        (843, 2),
        (847, 2),
        (851, 1),
    ] {
        put(&mut r, at, &x.to_le_bytes());
    }
    put(&mut r, 859, &MM.to_le_bytes());
    put(&mut r, 861, &12_700u16.to_le_bytes());
    for at in [869, 873, 877] {
        put(&mut r, at, &[1]);
    }
    put(&mut r, 885, &[0x01, 0x0c, 0, 0, 0, 0x05, 0, 0x05]);
    put(&mut r, 898, &[0xff, 0, 0, 0, 0x07, 0x10]);
    put(&mut r, 928, &[0x01, 0x0c, 0, 0, 0, 0x05, 0, 0x05]);
    put(&mut r, 942, &[0x64, 0, 0, 0x07, 0x10]);
    put(&mut r, 971, &[1]);
    put(&mut r, 1007, &[1]);
    // Four scales of 1, then 2, 0.2, 1 and a 4 by 4 grid.
    for k in 0..4 {
        put(&mut r, 1019 + 8 * k, &1.0f64.to_le_bytes());
    }
    put(&mut r, 1051, &2u32.to_le_bytes());
    put(&mut r, 1055, &0.2f64.to_le_bytes());
    put(&mut r, 1063, &1u32.to_le_bytes());
    put(&mut r, 1067, &4.0f64.to_le_bytes());
    put(&mut r, 1075, &4.0f64.to_le_bytes());
    put(&mut r, 1083, &[4, 1]);
    put(&mut r, 1100, &[1]);
    // The drawing profile: 12, three flags, three scales of 0.8165, the
    // axes at 330, 90 and 210 degrees (millionths) and its name.
    put(&mut r, 7360, &12.0f64.to_le_bytes());
    for at in [7368, 7372, 7376] {
        put(&mut r, at, &1u32.to_le_bytes());
    }
    for k in 0..3 {
        put(&mut r, 7380 + 8 * k, &0.8165f64.to_le_bytes());
    }
    for (k, a) in [330_000_000i32, 90_000_000, 210_000_000]
        .into_iter()
        .enumerate()
    {
        put(&mut r, 7404 + 4 * k, &a.to_le_bytes());
    }
    let name: Vec<u8> = "Isometric"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    put(&mut r, 7416, &name);
    r
}

/// `font`: id, character set, style flags, 10 bytes of classification,
/// the family name: 8-bit in a 74-byte field before version 12, UTF-16 in
/// a 296-byte record after (with the technology value 2 at 160).
/// A font table record: id, character set, the face's style flags (0x40
/// regular, 0x1000 bold, 0x80 italic, 0x2000 bold italic), 10 bytes,
/// then the name. Files list a record for each face their styles use
/// (two for Arial when text is set in Arial and Arial bold); a style
/// naming a face without its record was drawn in a stand-in font.
fn font_record(v: Ver, id: u16, family: &str, flags: u32) -> Vec<u8> {
    let mut b = Buf::default();
    b.u16(id).u16(0).u32(flags).zeros(10);
    if v.unicode() {
        let name: Vec<u16> = family.encode_utf16().take(31).collect();
        for u in &name {
            b.u16(*u);
        }
        b.pad_to(160).u32(2).pad_to(296);
    } else {
        let mut name =
            cp1252(family).unwrap_or_else(|| family.bytes().filter(|c| c.is_ascii()).collect());
        name.truncate(73);
        b.bytes(&name).pad_to(92);
    }
    b.done()
}

/// What the style table's styles use: the fill records of text (black) and
/// of graphics (none), the outline record (none) and the font.
#[derive(Debug, Clone)]
struct StyleUses {
    text_fill: u32,
    graphic_fill: u32,
    outline: u32,
    font: u16,
    /// Text formats of versions 7 to 15, one style each after the
    /// graphic style (ids from `GRAPHIC_STYLE_ID + 1`).
    formats: Vec<TextFormat>,
}

/// The look a paragraph of text takes from its style before version 16,
/// as files of versions 13 to 15 keep it: the font, size and style flags
/// in a font entry, the fill and the alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TextFormat {
    font: u16,
    size: i32,
    flags: u32,
    fill: u32,
    align: usize,
}

/// Decoration lines of the character settings (`set5`), as files of every
/// version carry them: (position, thickness) in millionths of the font
/// size, for underline (single thin, single thick, double), overline and
/// strikethrough; a double line has two entries.
const DECORATION_LINES: [(i32, i32); 12] = [
    (-90_909, 45_455),
    (-90_909, 90_909),
    (-90_909, 45_455),
    (-181_819, 45_455),
    (845_455, 45_455),
    (868_182, 90_909),
    (800_000, 45_455),
    (890_910, 45_455),
    (333_333, 45_455),
    (356_060, 90_909),
    (378_788, 45_455),
    (287_878, 45_455),
];

/// `stlt`: the style table. Its tables (fills, outlines, fonts,
/// alignments, spacings, character settings, then empty tab, bullet,
/// indent, hyphenation and drop cap tables, and from version 9 one set11
/// entry) number their entries in one sequence from 1, as files do, and
/// every style names an entry of each table it uses: a style per
/// alignment for artistic text (ids from `STYLE_ID`, 12 pt, 100 % line
/// spacing) and one for graphics (`GRAPHIC_STYLE_ID`). The version's
/// layout is the one `text::read_stlt` reads.
fn stlt(v: Ver, uses: StyleUses) -> Vec<u8> {
    let n = v.0;
    let mut next = 0u32;
    let mut id = || {
        next += 1;
        next
    };
    // Entries are numbered in the order the tables list them: fills (with
    // one for each text format), the outline, fonts (one for each text
    // format), alignments, spacing, decoration lines and set11.
    let (text_fill, graphic_fill) = (id(), id());
    let format_fills: Vec<u32> = uses.formats.iter().map(|_| id()).collect();
    let outline = id();
    let font = id();
    let format_fonts: Vec<u32> = uses.formats.iter().map(|_| id()).collect();
    let aligns: Vec<u32> = STYLE_ALIGNS.iter().map(|_| id()).collect();
    let (interval, set5) = (id(), id());
    let set11 = id();
    let format_entries: Vec<(u32, u32)> = format_fills.into_iter().zip(format_fonts).collect();
    let mut t = Buf::default();
    t.u32(STYLE_ALIGNS.len() as u32 + 1 + uses.formats.len() as u32); // records
                                                                      // Fills: id, 4 unknown, the fill record, from 13 the fill's matrix.
    t.u32(2 + uses.formats.len() as u32);
    let fills = [
        (text_fill, uses.text_fill),
        (graphic_fill, uses.graphic_fill),
    ]
    .into_iter()
    .chain(
        uses.formats
            .iter()
            .zip(&format_entries)
            .map(|(f, (fill, _))| (*fill, f.fill)),
    );
    for (entry, record) in fills {
        t.u32(entry).u32(0).u32(record);
        if v.tagged() {
            t.bytes(&identity_matrix());
        }
    }
    // Outlines: id, 4 unknown, the outline record.
    t.u32(1).u32(outline).u32(0).u32(uses.outline);
    // Fonts: id, a mask (12 bytes before 10, 20 after), then the font,
    // size and style of the three scripts (Latin, Asian, other), all in
    // the one font: font id u16 and character set u16, two more font
    // references, sizes, style flags (one before 10).
    let mask = if v.redirects() { 0xff } else { 0xffff };
    let font_entry = |t: &mut Buf, entry: u32, face: u16, size: i32, flags: u32| {
        let script_font = u32::from(face);
        t.u32(entry).u32(0).u32(mask).u32(mask);
        if n >= 10 {
            t.u32(mask).u32(if v.redirects() { u32::MAX } else { 1 });
        }
        t.u16(face).u16(0).u32(script_font).u32(script_font);
        t.i32(size).i32(size).i32(size).u32(flags);
        if n >= 10 {
            t.u32(flags).u32(flags);
        }
    };
    t.u32(1 + uses.formats.len() as u32);
    font_entry(
        &mut t,
        font,
        uses.font,
        (12.0 * 254000.0 / 72.0) as i32,
        0x40,
    );
    for (f, (_, entry)) in uses.formats.iter().zip(&format_entries) {
        font_entry(&mut t, *entry, f.font, f.size, f.flags);
    }
    // Alignments: id, 4 unknown, value.
    t.u32(STYLE_ALIGNS.len() as u32);
    for (entry, (_, code)) in aligns.iter().zip(STYLE_ALIGNS.iter()) {
        t.u32(*entry).u32(0).u32(*code);
    }
    // Spacings: id, 0, a mask (0xffff, from 16 0x3ff), character spacing
    // 0, then 0, 100 %, the line spacing (100 %), 100 %, 0, 200 %, 60 %,
    // 300 % and 0 in millionths: the values every sample file has, of
    // which only the character and line spacings are understood.
    t.u32(1).u32(interval).u32(0);
    t.u32(if v.redirects() { 0x3ff } else { 0xffff });
    t.u32(0).u32(0).u32(1_000_000).u32(1_000_000).u32(1_000_000);
    t.u32(0).u32(2_000_000).u32(600_000).u32(3_000_000).u32(0);
    // Character settings: id, 0, then each decoration line as 0,
    // position, thickness.
    t.u32(1).u32(set5).u32(0);
    for (position, thickness) in DECORATION_LINES {
        t.u32(0).i32(position).i32(thickness);
    }
    // Tabs, bullets, indents, hyphenation, drop caps: none; set11 from 9.
    for _ in 0..5 {
        t.u32(0);
    }
    if n >= 9 {
        t.u32(1).u32(set11).u32(0).u32(0);
    }
    // The records: sections (1: fill and outline; 2: also the character
    // and paragraph settings), id, no parent, 8 unknown, the name (none
    // from 16, as files have it), then the entries they use.
    let name = |t: &mut Buf, s: &str| {
        let s = if v.redirects() { "" } else { s };
        t.u32(s.chars().count() as u32);
        if v.unicode() {
            t.bytes(&utf16(s));
        } else {
            t.bytes(s.as_bytes());
        }
    };
    let labels = ["Default", "Centred", "Right", "Justified"];
    for (i, (label, align)) in labels.iter().zip(&aligns).enumerate() {
        t.u32(2).u32(STYLE_ID + i as u32).u32(0).zeros(8);
        name(&mut t, label);
        t.u32(text_fill).u32(outline);
        t.u32(font).u32(*align).u32(interval).u32(set5);
        if n >= 9 {
            t.u32(set11);
        }
    }
    t.u32(1).u32(GRAPHIC_STYLE_ID).u32(0).zeros(8);
    name(&mut t, "Default Graphic");
    t.u32(graphic_fill).u32(outline);
    for (i, (f, (fill, font))) in uses.formats.iter().zip(&format_entries).enumerate() {
        let label = format!("Text {}", i + 1);
        t.u32(2)
            .u32(GRAPHIC_STYLE_ID + 1 + i as u32)
            .u32(0)
            .zeros(8);
        name(&mut t, &label);
        t.u32(*fill).u32(outline);
        let align = aligns.get(f.align).copied().unwrap_or(aligns[0]);
        t.u32(*font).u32(align).u32(interval).u32(set5);
        if n >= 9 {
            t.u32(set11);
        }
    }
    t.done()
}

/// `bmp `: a 24-bit image: id, a file-like header ("UI" block size, the
/// "RI" block with its size and pixel offset), colour model 1, size,
/// bits, row stride, pixel bytes, resolution, then rows bottom-up in
/// B G R order padded to 4 bytes. An image with see-through pixels adds
/// the transparency block files carry (byte 24 gives its place from byte
/// 8): "RI", its length, 0, a 78-byte header (model 99, 1, size, 1 plane,
/// 8 bits, stride, bytes, 64 at 62) and one byte per pixel, 255 opaque,
/// rows bottom-up padded to 4 bytes.
fn bmp_record(id: u32, png: &[u8], size_mm: (f64, f64)) -> Option<Vec<u8>> {
    let pm = tiny_skia::Pixmap::decode_png(png).ok()?;
    let (w, h) = (pm.width(), pm.height());
    if w == 0 || h == 0 {
        return None;
    }
    let stride = (w as usize * 24).div_ceil(32) * 4;
    let mut pixels = vec![0u8; stride * h as usize];
    let src = pm.pixels();
    let mask_stride = (w as usize).div_ceil(4) * 4;
    let see_through = src.iter().any(|p| p.alpha() < 255);
    let mut mask = vec![
        0u8;
        if see_through {
            mask_stride * h as usize
        } else {
            0
        }
    ];
    for row in 0..h as usize {
        let at = (h as usize - 1 - row) * stride;
        let mask_at = (h as usize - 1 - row) * mask_stride;
        for x in 0..w as usize {
            let Some(p) = src.get(row * w as usize + x).map(|p| p.demultiply()) else {
                continue;
            };
            if let Some(m) = mask.get_mut(mask_at + x) {
                *m = p.alpha();
            }
            if let Some(px) = pixels.get_mut(at + x * 3..at + x * 3 + 3) {
                px.copy_from_slice(&[p.blue(), p.green(), p.red()]);
            }
        }
    }
    let per_km = |px: u32, mm: f64| {
        if mm > 1e-6 {
            (px as f64 / mm * 1_000_000.0)
                .round()
                .clamp(1.0, u32::MAX as f64) as u32
        } else {
            2_834_646
        }
    };
    let image_len = 4 + 50 + 32 + 32 + pixels.len();
    let mask_len = if see_through { 78 + mask.len() } else { 0 };
    let len = image_len + mask_len;
    let mask_place = if see_through { image_len as u32 - 8 } else { 0 };
    let mut b = Buf::default();
    b.u32(id)
        .u32(1)
        .u32(0x4955)
        .u32((len - 8) as u32)
        .u32(0)
        .u32(32)
        .u32(mask_place)
        .zeros(4)
        .u32(2)
        .u32(255)
        .u16(0x4952)
        .u32((image_len - 40) as u32)
        .u32(0)
        .u32((image_len - pixels.len() - 40) as u32);
    b.u32(1)
        .u32(1)
        .u32(w)
        .u32(h)
        .u32(1)
        .u32(24)
        .u32(stride as u32)
        .u32(pixels.len() as u32);
    b.u32(0)
        .u32(per_km(w, size_mm.0))
        .u32(per_km(h, size_mm.1))
        .u32(0)
        .u32(64)
        .zeros(12);
    b.bytes(&pixels);
    if see_through {
        b.u16(0x4952)
            .u32(mask_len as u32)
            .u32(0)
            .u32(78)
            .u32(99)
            .u32(1)
            .u32(w)
            .u32(h)
            .u32(1)
            .u32(8)
            .u32(mask_stride as u32)
            .u32(mask.len() as u32)
            .zeros(16)
            .u32(64)
            .zeros(12);
        b.bytes(&mask);
    }
    Some(b.done())
}

/// `loda`: header (length, argument count, offsets table, types table,
/// object type), the offsets table with the end marker, the types table
/// in reverse order, then the argument bodies back to back. Arguments go
/// highest type first, as files order them.
fn loda(kind: u32, mut args: Vec<(u32, Vec<u8>)>) -> Vec<u8> {
    args.sort_by_key(|a| std::cmp::Reverse(a.0));
    let n = args.len();
    let start_args = 20usize;
    let start_types = start_args + (n + 1) * 4;
    let mut pos = start_types + n * 4;
    let mut offsets = Vec::with_capacity(n + 1);
    for (_, b) in &args {
        offsets.push(pos as u32);
        pos += b.len();
    }
    offsets.push(pos as u32);
    let mut out = Buf::default();
    out.u32(pos as u32)
        .u32(n as u32)
        .u32(start_args as u32)
        .u32(start_types as u32)
        .u32(kind);
    for o in &offsets {
        out.u32(*o);
    }
    for (t, _) in args.iter().rev() {
        out.u32(*t);
    }
    for (_, b) in &args {
        out.bytes(b);
    }
    out.done()
}

/// The mean opacity of a transparency effect's mask: the lightness of
/// its colours, averaged over a fountain's stops.
fn mask_mean_opacity(e: &tracedraw_core::live::Effect) -> Option<f64> {
    let tracedraw_core::live::Effect::Transparency { mask, .. } = e else {
        return None;
    };
    let level = |c: &tracedraw_core::Color| c.luminance().clamp(0.0, 1.0) as f64;
    match mask {
        tracedraw_core::Fill::Solid(c) => Some(level(c)),
        tracedraw_core::Fill::Fountain(f) if !f.stops.is_empty() => {
            Some(f.stops.iter().map(|st| level(&st.color)).sum::<f64>() / f.stops.len() as f64)
        }
        _ => None,
    }
}

/// Opacity argument: 10 unknown bytes (14 from version 13), then the
/// transparency in thousandths.
fn opacity_arg(v: Ver, opacity: f64) -> Vec<u8> {
    let mut b = Buf::default();
    b.zeros(if v.tagged() { 14 } else { 10 })
        .u16(((1.0 - opacity.clamp(0.0, 1.0)) * 1000.0).round() as u16);
    b.done()
}

/// Points and node bytes of a path in local units: bits 7-6 the node kind
/// (00 move, 01 line, 10 curve end, 11 control point), bits 5-4 the
/// node's type (0 cusp, 1 smooth, 2 symmetrical), bit 3 closes the subpath.
fn curve_coords(path: &BezPath) -> Option<Vec<u8>> {
    curve_coords_typed(path, &Default::default())
}

/// [`curve_coords`] for a curve with node types chosen for it.
fn curve_coords_typed(
    path: &BezPath,
    chosen: &std::collections::BTreeMap<usize, tracedraw_core::nodes::NodeType>,
) -> Option<Vec<u8>> {
    use tracedraw_core::nodes::{self, NodeType};
    let continuity: std::collections::BTreeMap<usize, u8> = nodes::nodes(path)
        .into_iter()
        .map(|n| {
            let bits = match nodes::node_type_of_with(&n, chosen) {
                NodeType::Cusp => 0x00,
                NodeType::Smooth => 0x10,
                NodeType::Symmetrical => 0x20,
            };
            (n.index, bits)
        })
        .collect();
    let node_bits = |i: usize| continuity.get(&i).copied().unwrap_or(0);
    let mut pts: Vec<(i32, i32)> = Vec::new();
    let mut types: Vec<u8> = Vec::new();
    let mut last_start = 0usize;
    let mut last = Point::ZERO;
    for (i, el) in path.elements().iter().enumerate() {
        match el {
            PathEl::MoveTo(p) => {
                pts.push((units(p.x), units(p.y)));
                types.push(node_bits(i));
                last_start = pts.len() - 1;
                last = *p;
            }
            PathEl::LineTo(p) => {
                pts.push((units(p.x), units(p.y)));
                types.push(0x40 | node_bits(i));
                last = *p;
            }
            PathEl::QuadTo(c, p) => {
                // Raised to a cubic.
                let c1 = last + (*c - last) * (2.0 / 3.0);
                let c2 = *p + (*c - *p) * (2.0 / 3.0);
                pts.push((units(c1.x), units(c1.y)));
                types.push(0xc0);
                pts.push((units(c2.x), units(c2.y)));
                types.push(0xc0);
                pts.push((units(p.x), units(p.y)));
                types.push(0x80 | node_bits(i));
                last = *p;
            }
            PathEl::CurveTo(a, b, p) => {
                pts.push((units(a.x), units(a.y)));
                types.push(0xc0);
                pts.push((units(b.x), units(b.y)));
                types.push(0xc0);
                pts.push((units(p.x), units(p.y)));
                types.push(0x80 | node_bits(i));
                last = *p;
            }
            PathEl::ClosePath => {
                // A closing line back to the subpath start, flagged.
                let s = pts.get(last_start).copied().unwrap_or((0, 0));
                pts.push(s);
                types.push(0x48);
            }
        }
    }
    if pts.is_empty() {
        return None;
    }
    let mut out = Buf::default();
    out.u32(pts.len() as u32);
    for (x, y) in &pts {
        out.i32(*x).i32(*y);
    }
    out.bytes(&types);
    Some(out.done())
}

/// The node marks files of every version (7 to 27 seen) carry in a
/// curve's node bytes: bit 2 on every node but the control points and the node that
/// closes a subpath, and bit 3 also on the start of a closed subpath.
/// Curves without them are drawn without their fill and can make the
/// program that opens the file hang.
fn mark_nodes(coords: &mut [u8]) {
    let n = coords
        .get(0..4)
        .and_then(|b| b.try_into().ok())
        .map_or(0, u32::from_le_bytes) as usize;
    let Some(types) = coords.get_mut(4 + 8 * n..4 + 9 * n) else {
        return;
    };
    let mut start = 0;
    for k in 0..=types.len() {
        let ends = k == types.len() || (k > start && types[k] & 0xc0 == 0);
        if !ends {
            continue;
        }
        let closed = types.get(k.wrapping_sub(1)).is_some_and(|t| t & 0x08 != 0);
        for (i, t) in types.iter_mut().enumerate().take(k).skip(start) {
            if *t & 0xc0 == 0xc0 || (closed && i + 1 == k) {
                continue;
            }
            *t |= 0x04;
        }
        if closed {
            if let Some(t) = types.get_mut(start) {
                *t |= 0x08;
            }
        }
        start = k;
    }
}

/// A box as `bbox` stores it: top left, then bottom right (file units,
/// y up).
fn box_bytes(r: Rect) -> Vec<u8> {
    let mut b = Buf::default();
    b.i32(units(r.x0))
        .i32(units(r.y1))
        .i32(units(r.x1))
        .i32(units(r.y0));
    b.done()
}

/// `obbx`: the box, then the box with the outline.
fn obbx_bytes(r: Rect, outline: f64) -> Vec<u8> {
    let mut b = box_bytes(r);
    b.extend(box_bytes(r.inflate(outline, outline)));
    b
}

/// A graded transparency as the `transparency` section of a version 16+
/// style string: a fountain `fill` (`type` 2) of two greys, 255 opaque,
/// with its kind, angle, centre offsets (percent) and scale, and the part
/// it applies to (the layout read in `text::object_transparency_mask`).
/// `None` when the mask's ends are not plain colours.
fn graded_transparency_json(g: (&tracedraw_core::Fountain, u8)) -> Option<String> {
    use tracedraw_core::FountainKind;
    let (f, target) = g;
    let mut f = f.clone();
    crate::mirror_centred_stops(&mut f);
    let (first, last) = (f.stops.first()?, f.stops.last()?);
    let grey = |c: &tracedraw_core::Color| (c.luminance().clamp(0.0, 1.0) * 255.0).round() as u32;
    let kind = match f.kind {
        FountainKind::Radial => 2,
        FountainKind::Conical => 3,
        FountainKind::Square => 4,
        _ => 1,
    };
    let scale = 1.0 - 2.0 * f.edge_pad.clamp(0.0, 0.49);
    let nil = "00000000-0000-0000-0000-000000000000";
    Some(format!(
        r#"{{"mode":"0","fill":{{"type":"2","fountainType":"{kind}","angle":"{}","xCenterOffset":"{}","yCenterOffset":"{}","xScale":"{scale}","yScale":"{scale}","primaryColor":"GRAY255,USER,{},100,{nil}","secondaryColor":"GRAY255,USER,{},100,{nil}","primaryAlpha":"255","secondaryAlpha":"255","spreadMethod":"0","numSteps":"0","mergeMode":"0"}},"appliesTo":"{}","startTransparency":"0","endTransparency":"1"}}"#,
        f.angle,
        f.offset.x * 100.0,
        f.offset.y * 100.0,
        grey(&first.color),
        grey(&last.color),
        target.min(2),
    ))
}

/// The style string of an object (versions 16+): its fill, outline and
/// transparency.
fn object_style_json(
    fill: &Fill,
    stroke: Option<&Stroke>,
    opacity: f64,
    graded: Option<(&tracedraw_core::Fountain, u8)>,
) -> String {
    let fill_part = match fill_colour(fill) {
        Some(c) if !matches!(fill, Fill::None) => {
            format!(r#"{{"type":"1","primaryColor":"{}"}}"#, colour_spec(c))
        }
        _ => r#"{"type":"0"}"#.to_string(),
    };
    let outline_part = match stroke {
        // The mitre limit in degrees, where the reader takes it (#384).
        Some(s) => format!(
            r#"{{"width":"{}","color":"{}","miterLimit":"{}"}}"#,
            if s.width <= Stroke::HAIRLINE + 1e-9 {
                0
            } else {
                units(s.width)
            },
            colour_spec(s.color),
            Stroke::miter_angle_from_ratio(s.miter_ratio())
        ),
        None => r#"{"type":"1","width":"0"}"#.to_string(),
    };
    let transparency = if let Some(g) = graded.and_then(graded_transparency_json) {
        g
    } else if opacity < 0.999 {
        format!(
            r#"{{"type":"1","uniformTransparency":"{}"}}"#,
            ((1.0 - opacity.clamp(0.0, 1.0)) * 1000.0).round() / 1000.0
        )
    } else {
        "{}".to_string()
    };
    format!(r#"{{"fill":{fill_part},"outline":{outline_part},"transparency":{transparency}}}"#)
}

/// The style codes of the font tables: one bit per weight and slant
/// (0x40 regular, 0x80 italic, 0x1000 bold, 0x2000 bold italic), single
/// underline in bits 18 to 20, single strike-through in bits 24 to 26.
/// A face of another weight (a variable font's weight axis set on the
/// text, #1013) takes the code of its weight: one bit per 100 of weight
/// (1 = 100 ... 64 = 400, 1024 = 600, 4096 = 700 ... 65536 = 900), the
/// next bit up its italic.
fn style_flags(sp: &TextSpan) -> u32 {
    let mut f = weight_code(sp);
    if sp.underline {
        f |= 1 << 18;
    }
    if sp.strikethrough {
        f |= 1 << 24;
    }
    f
}

/// The weight `sp` is drawn at, in the steps of 100 the style codes
/// count: its variable font weight axis when that is set (#1013), else
/// 700 bold and 400 regular.
fn span_weight(sp: &TextSpan) -> u32 {
    match sp.variation("wght").filter(|w| w.is_finite()) {
        Some(w) => (w / 100.0).round().clamp(1.0, 9.0) as u32 * 100,
        None if sp.bold => 700,
        None => 400,
    }
}

/// The style code of `sp`'s face: a power of two counting weights from
/// 1 for 100 (64 = 400, 1024 = 600, 4096 = 700), the italic of a weight
/// the bit above it (128 = 400 italic, 8192 = 700 italic).
fn weight_code(sp: &TextSpan) -> u32 {
    let step = span_weight(sp) / 100 - 1;
    1 << (2 * step + u32::from(sp.italic))
}

/// The weight member of a version 16+ font style: the weight's code,
/// without the italic, which has a member of its own.
fn weight_member(sp: &TextSpan) -> u32 {
    1 << (2 * (span_weight(sp) / 100 - 1))
}

/// The style string of a run (versions 16+): its font, size, weight,
/// slant, lines and colour.
fn run_style_json(sp: &TextSpan, fill: Option<&Fill>) -> String {
    format!("{{{}}}", character_json(sp, fill))
}

/// The `character` member of a style string.
fn character_json(sp: &TextSpan, fill: Option<&Fill>) -> String {
    let weight = weight_member(sp);
    let size = (sp.size_pt * 254000.0 / 72.0).round() as i64;
    let family = sp.font_family.replace('\\', "\\\\").replace('"', "\\\"");
    let font = format!(
        r#"{{"font":"{family}","size":"{size}","weight":"{weight}","italic":"{}","underline":"{}","strikeout":"{}"}}"#,
        u8::from(sp.italic),
        u8::from(sp.underline),
        u8::from(sp.strikethrough)
    );
    let fill = match fill.and_then(fill_colour) {
        Some(c) => format!(
            r#""fill":{{"type":"1","primaryColor":"{}"}},"#,
            colour_spec(c)
        ),
        None => String::new(),
    };
    format!(r#""character":{{{fill}"latin":{font},"RTL":{font},"farEast":{font}}}"#)
}

/// A text paragraph's whole default style, as files of versions 16 and
/// later write it: the first run's font, size, weight and colour, the
/// alignment, and every other member with the values files hold when it
/// is not set.
/// A paragraph's line spacing as a factor of the character height (the
/// unit of a style string's `interLineSpacing`), the character height of
/// `size_pt` text taken as 1.15 times the size as the reader takes it.
fn line_spacing_factor(f: &tracedraw_core::ParagraphFormat, size_pt: f64) -> f64 {
    let size = if size_pt > 0.0 { size_pt } else { 12.0 };
    let v = match f.leading_unit {
        tracedraw_core::LeadingUnit::PercentCharHeight => f.leading_pct / 100.0,
        tracedraw_core::LeadingUnit::Points => f.leading_pct / (size * 1.15),
        tracedraw_core::LeadingUnit::PercentPointSize => f.leading_pct / 100.0 / 1.15,
    };
    if v.is_finite() {
        v.clamp(0.1, 10.0)
    } else {
        1.0
    }
}

fn full_text_style(
    sp: Option<&TextSpan>,
    fill: Option<&Fill>,
    align: TextAlign,
    leading: f64,
) -> String {
    let black = "CMYK,USER,0,0,0,100,100,00000000-0000-0000-0000-000000000000";
    let colour = fill
        .and_then(fill_colour)
        .map(colour_spec)
        .unwrap_or_else(|| black.to_string());
    let (family, size, weight, italic, underline, strike) = match sp {
        Some(sp) => (
            sp.font_family.replace('\\', "\\\\").replace('"', "\\\""),
            (sp.size_pt * 254000.0 / 72.0).round() as i64,
            weight_member(sp),
            u8::from(sp.italic),
            u8::from(sp.underline),
            u8::from(sp.strikethrough),
        ),
        None => ("Arial".to_string(), 254000 * 24 / 72, 64, 0, 0, 0),
    };
    let font = |charset: u32| {
        format!(
            r#"{{"overline":"0","effect":"0","version":"","italic":"{italic}","weight":"{weight}","underline":"{underline}","charset":"{charset}","technology":"4","size":"{size}","font":"{family}","script":"0","strikeout":"{strike}"}}"#
        )
    };
    let screen = "0,0,45000000,60,0";
    format!(
        concat!(
            r#"{{"character":{{"fill":{{"type":"1","overprint":"0","secondaryColor":"CMYK,USER,0,0,0,0,100,00000000-0000-0000-0000-000000000000","primaryColor":"{colour}","screenSpec":"{screen}"}},"#,
            r#""angle":"0","outline":{{"overprint":"0","angle":"0","aspect":"100","dashDotSpec":"0","behindFill":"0","miterLimit":"5","justification":"0","dashAdjust":"0","rightArrow":"|0","screenSpec":"{screen}","overlapArrow":"0","shareArrow":"0","dotLength":"0","scaleWithObject":"0","matrix":"1,0,0,0,1,0","color":"{black}","leftArrow":"|0","projectMatrix":"1,0,0,0,1,0","leftArrowAttributes":"0|0|0|0|0|0|0","endCaps":"0","rightArrowAttributes":"0|0|0|0|0|0|0","joinType":"0","width":"0"}},"#,
            r#""RTL":{rtl},"latin":{latin},"h_shift":"0","v_shift":"0","farEast":{far}}},"frame":{{}},"#,
            r#""paragraph":{{"interCharSpacing":"0","thickOverlineAbsolute":"0","thickAbsolute":"0","dblStrikeout2Thickness":"45455","dblStrikeout2Distance":"287878","dblOverline1Absolute":"0","maxInterCharSpacing":"2000000","dblUnderline1Distance":"-90909","dblOverline2Absolute":"0","dblStrikeout1Distance":"378788","dblStrikeout1Absolute":"0","thickThickness":"90909","dblUnderline2Distance":"-181819","dblUnderline2Thickness":"45455","interLineSpacing":"{leading}","dblOverline1Thickness":"45455","afterParaSpacing":"0","thickOverlineThickness":"90909","spacingMethod":"0","dblStrikeout2Absolute":"0","thickStrikeoutAbsolute":"0","thinThickness":"45455","interLangSpacing":"0","interWordSpacing":"1000000","maxInterWordSpacing":"3000000","thinAbsolute":"0","dblUnderline1Absolute":"0","dblUnderline2Absolute":"0","dblUnderline1Thickness":"45455","thinOverlineAbsolute":"0","thinOverlineThickness":"45455","thinDistance":"-90909","beforeParaSpacing":"1000000","dblOverline2Distance":"890910","thinOverlineDistance":"845455","dblOverline2Thickness":"45455","thinStrikeoutAbsolute":"0","dblOverline1Distance":"800000","thinStrikeoutDistance":"333333","minInterWordSpacing":"600000","thinStrikeoutThickness":"45455","thickDistance":"-90909","thickOverlineDistance":"868182","thickStrikeoutDistance":"356060","thickStrikeoutThickness":"90909","dblStrikeout1Thickness":"45455","justify":"{justify}"}}}}"#
        ),
        colour = colour,
        screen = screen,
        black = black,
        rtl = font(178),
        latin = font(0),
        far = font(128),
        justify = justify_code(align),
        leading = (leading * 1_000_000.0).round() as i64,
    )
}

/// The alignment of each paragraph of a text (its own, or `align`).
fn paragraph_aligns(spans: &[TextSpan], align: TextAlign) -> Vec<TextAlign> {
    tracedraw_core::paragraphs::paragraph_formats_of(
        spans,
        align,
        &tracedraw_core::ParagraphStyle::default(),
    )
    .into_iter()
    .map(|f| f.align)
    .collect()
}

/// The `justify` value of a style string: 1 left, 2 centre, 3 right, 4
/// full justify (5 forced).
fn justify_code(align: TextAlign) -> u32 {
    match align {
        TextAlign::None => 0,
        TextAlign::Left => 1,
        TextAlign::Center => 2,
        TextAlign::Right => 3,
        TextAlign::Justify => 4,
        TextAlign::ForceJustify => 5,
    }
}

// ---- the writer ----------------------------------------------------------

struct Writer<'a> {
    v: Ver,
    symbols: &'a [Symbol],
    fills: Vec<Vec<u8>>,
    fill_ids: HashMap<String, u32>,
    outlines: Vec<Vec<u8>>,
    outline_ids: HashMap<String, u32>,
    fonts: Vec<(u16, String)>,
    bitmaps: Vec<Vec<u8>>,
    /// Bitmap pattern fills already written, matched by their picture's
    /// content (the pointer first, then the bytes) and size.
    bitmap_fills: Vec<(SharedBytes, u32, u32, u64, u32)>,
    /// Static ids (`usdn`), object ids (argument 100), next fill, outline
    /// and bitmap ids.
    next_usdn: u32,
    next_object: u32,
    /// Graded transparencies (mask, target) of the objects being written,
    /// by shape: version 16+ style strings carry them.
    graded: std::collections::HashMap<tracedraw_core::ShapeId, (tracedraw_core::Fountain, u8)>,
    next_fill: u32,
    next_outline: u32,
    next_bitmap: u32,
    /// The next text object's `spnd` number.
    next_spnd: u32,
    /// Text formats met so far (versions 7 to 15), see [`TextFormat`].
    text_formats: Vec<TextFormat>,
    guid_seed: u64,
    guids: u64,
    /// The GUIDs of the master page's guides, desktop and grid layers,
    /// which each page's entries for them repeat.
    master_guids: [[u8; 16]; 3],
    has_text: bool,
}

/// The master page's layers, in the order of `Writer::master_guids`.
#[derive(Debug, Clone, Copy)]
enum MasterLayer {
    Guides,
    Desktop,
    Grid,
}

impl MasterLayer {
    fn flags(self) -> [u8; 4] {
        match self {
            MasterLayer::Guides => [0x0a, 0, 1, 0x98],
            MasterLayer::Desktop => [0x08, 0, 1, 0x98],
            MasterLayer::Grid => [0x1a, 1, 1, 0x98],
        }
    }

    fn kind(self) -> u32 {
        match self {
            MasterLayer::Guides => LAYER_GUIDES,
            MasterLayer::Desktop => LAYER_DESKTOP,
            MasterLayer::Grid => LAYER_GRID,
        }
    }

    fn name(self) -> &'static str {
        match self {
            MasterLayer::Guides => "Guides",
            MasterLayer::Desktop => "Desktop",
            MasterLayer::Grid => "Document Grid",
        }
    }
}

/// Where a written object sits: its box and the box with its outline.
struct Placed {
    nodes: Vec<Node>,
    bounds: Option<Rect>,
}

impl<'a> Writer<'a> {
    fn new(v: Ver, symbols: &'a [Symbol], title: &str) -> Self {
        let seed = title.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
        });
        let master_guids = [guid(seed, 1), guid(seed, 2), guid(seed, 3)];
        Writer {
            v,
            symbols,
            fills: Vec::new(),
            fill_ids: HashMap::new(),
            outlines: Vec::new(),
            outline_ids: HashMap::new(),
            fonts: Vec::new(),
            bitmaps: Vec::new(),
            next_usdn: 1,
            next_object: 1,
            graded: std::collections::HashMap::new(),
            next_fill: 1,
            next_outline: 1,
            next_bitmap: 1,
            bitmap_fills: Vec::new(),
            next_spnd: 1,
            text_formats: Vec::new(),
            guid_seed: seed,
            guids: master_guids.len() as u64,
            master_guids,
            has_text: false,
        }
    }

    fn guid(&mut self) -> [u8; 16] {
        self.guids += 1;
        guid(self.guid_seed, self.guids)
    }

    fn fill_id(&mut self, fill: &Fill) -> u32 {
        // A bitmap pattern goes by its picture's content, not by its
        // printed bytes (about five characters per byte of the PNG, kept
        // for the whole write).
        if let Fill::Pattern(Pattern::Bitmap {
            png,
            width_px,
            height_px,
            size_mm,
        }) = fill
        {
            let key = (*width_px, *height_px, size_mm.to_bits());
            if let Some(f) = self
                .bitmap_fills
                .iter()
                .find(|f| (f.1, f.2, f.3) == key && f.0 == *png)
            {
                return f.4;
            }
            let id = self.next_fill;
            self.next_fill += 1;
            self.fills.push(fild(self.v, id, fill));
            self.bitmap_fills
                .push((png.clone(), key.0, key.1, key.2, id));
            return id;
        }
        let key = format!("{fill:?}");
        if let Some(id) = self.fill_ids.get(&key) {
            return *id;
        }
        let id = self.next_fill;
        self.next_fill += 1;
        self.fills.push(fild(self.v, id, fill));
        self.fill_ids.insert(key, id);
        id
    }

    fn outline_id(&mut self, stroke: Option<&Stroke>) -> u32 {
        let key = format!("{stroke:?}");
        if let Some(id) = self.outline_ids.get(&key) {
            return *id;
        }
        let id = self.next_outline;
        self.next_outline += 1;
        self.outlines.push(outl(self.v, id, stroke));
        self.outline_ids.insert(key, id);
        id
    }

    fn font_id(&mut self, family: &str) -> u16 {
        if let Some((id, _)) = self.fonts.iter().find(|(_, f)| f == family) {
            return *id;
        }
        let id = self.fonts.len() as u16 + 1;
        self.fonts.push((id, family.to_string()));
        id
    }

    fn bitmap_id(&mut self, png: &[u8], size_mm: (f64, f64)) -> Option<u32> {
        let id = self.next_bitmap;
        let rec = bmp_record(id, png, size_mm)?;
        self.next_bitmap += 1;
        self.bitmaps.push(rec);
        Some(id)
    }

    /// The whole file: version chunks, bitmaps, the document tables, the
    /// master page and every page.
    fn document(&mut self, doc: &Document) -> Vec<Node> {
        let first = doc
            .pages
            .first()
            .map(|p| p.size)
            .unwrap_or(tracedraw_core::geometry::Size::new(210.0, 297.0));
        // Pages first: they fill the tables.
        let mut pages = Vec::new();
        let mut content: Option<Rect> = None;
        for page in &doc.pages {
            let (node, b) = self.page(page);
            if content.is_none() {
                content = b;
            }
            pages.push(node);
        }
        let v = self.v;
        // The faces the text styles use, for the font table: the face bits
        // of their style flags (lines left out).
        let format_faces: Vec<(u16, u32)> = self
            .text_formats
            .iter()
            .map(|f| (f.font, f.flags & 0x3_ffff))
            .collect();
        let mut out = Vec::new();
        if v.redirects() {
            let mut fver = Buf::default();
            fver.u16(v.0 * 100).u16(1).u16(0).u16(v.0);
            out.push(chunk(b"fver", fver.done()));
        }
        out.push(chunk(b"vrsn", (v.0 * 100).to_le_bytes().to_vec()));
        if !self.bitmaps.is_empty() {
            let bmps = std::mem::take(&mut self.bitmaps)
                .into_iter()
                .map(|b| chunk(b"bmp ", b))
                .collect();
            out.push(list(b"bmpt", bmps));
        }
        let mut docl = Vec::new();
        if !v.redirects() {
            docl.push(chunk(b"pfrd", vec![0; if v.tagged() { 2 } else { 1 }]));
        }
        docl.push(chunk(b"mcfg", mcfg(v, first.width, first.height)));
        if v.redirects() {
            docl.push(chunk(b"pref", pref()));
        }
        let content = content.unwrap_or(Rect::ZERO);
        let centre = Point::new(first.width / 2.0, first.height / 2.0);
        docl.push(chunk(b"ptrt", box_bytes(content - centre.to_vec2())));
        // The styles name a font, so the font table is never empty.
        if self.fonts.is_empty() {
            self.font_id("Arial");
        }
        if !self.fonts.is_empty() {
            let mut fonts = Vec::new();
            for (id, f) in &self.fonts {
                fonts.push(chunk(b"font", font_record(v, *id, f, 0x40)));
                let mut faces: Vec<u32> = format_faces
                    .iter()
                    .filter(|(fid, flags)| fid == id && *flags != 0x40)
                    .map(|(_, flags)| *flags)
                    .collect();
                faces.sort_unstable();
                faces.dedup();
                for flags in faces {
                    fonts.push(chunk(b"font", font_record(v, *id, f, flags)));
                }
            }
            docl.push(list(b"fntt", fonts));
        }
        // Every object refers to a fill and an outline, and the styles to
        // the fills of text (black) and graphics (none) and to no outline.
        let uses = StyleUses {
            text_fill: self.fill_id(&Fill::Solid(Color::BLACK)),
            graphic_fill: self.fill_id(&Fill::None),
            outline: self.outline_id(None),
            font: self.fonts.first().map(|(id, _)| *id).unwrap_or(1),
            formats: std::mem::take(&mut self.text_formats),
        };

        let fills = std::mem::take(&mut self.fills)
            .into_iter()
            .map(|f| list(b"filc", vec![chunk(b"fild", f)]))
            .collect();
        docl.push(list(b"filt", fills));
        let outlines = std::mem::take(&mut self.outlines)
            .into_iter()
            .map(|o| chunk(b"outl", o))
            .collect();
        docl.push(list(b"otlt", outlines));
        let styles = stlt(v, uses);
        if v.redirects() {
            docl.push(chunk(b"stlt", styles));
        } else {
            // Before version 16 the style table is the body of a list.
            docl.push(Node::RawList(*b"stlt", styles));
        }
        if !v.zip() {
            // RIFF files (before version 14) end the document tables
            // with an empty `uil` list.
            docl.push(list(b"uil ", Vec::new()));
        }
        out.push(list(b"doc ", docl));
        let background = match doc.pages.first().and_then(|p| p.background.as_ref()) {
            Some(Fill::Solid(c)) => Some(*c),
            _ => None,
        };
        out.push(self.master_page(background));
        out.extend(pages);
        out
    }

    /// A layer list: its flags, its properties (kind, a GUID from version
    /// 14, the colour and, unless it is a page's entry for a master layer,
    /// the name) and its objects.
    fn layer_entry(
        &mut self,
        flags: [u8; 4],
        kind: u32,
        name: Option<&str>,
        guid: [u8; 16],
        objects: Vec<Node>,
    ) -> Node {
        let v = self.v;
        let colour_bytes = match kind {
            LAYER_GUIDES => Color::rgb8(0, 0, 255),
            LAYER_GRID => Color::rgb8(194, 194, 193),
            _ => Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
        };
        let mut args = Vec::new();
        if let Some(name) = name {
            args.push((ARG_NAME, name_bytes(v, name)));
        }
        if v.tagged() {
            args.push((ARG_LAYER_COLOR, colour_props(v, colour_bytes)));
        } else {
            args.push((ARG_LAYER_COLOR, colour(colour_bytes).to_vec()));
        }
        // A zero 12000 from version 13, as layers of files of versions 13
        // to 27 have it (it was written from 17 only).
        if v.props() {
            args.push((ARG_PROPS, vec![0; 4]));
        }
        if v.0 >= 17 && kind == LAYER_GUIDES {
            args.push((ARG_GUIDES_MODE, 2u32.to_le_bytes().to_vec()));
        }
        if v.zip() {
            args.push((ARG_GUID, guid.to_vec()));
        }
        let mut children = vec![
            chunk(b"flgs", flags.to_vec()),
            list(b"lgob", vec![chunk(b"loda", loda(kind, args))]),
        ];
        children.extend(objects);
        list(b"layr", children)
    }

    /// A layer of its own, with a new GUID.
    fn layer(&mut self, flags: [u8; 4], kind: u32, name: &str, objects: Vec<Node>) -> Node {
        let guid = self.guid();
        self.layer_entry(flags, kind, Some(name), guid, objects)
    }

    /// A layer of the master page (`page` false), or a page's entry for it:
    /// the same flags and GUID, and from version 14 no name.
    fn master_layer(&mut self, which: MasterLayer, page: bool) -> Node {
        let guid = self.master_guids[which as usize];
        let name = (!page || !self.v.zip()).then(|| which.name());
        self.layer_entry(which.flags(), which.kind(), name, guid, Vec::new())
    }

    /// The master page: guides, desktop and grid layers, no objects; from
    /// version 21 the page background colour in its `loda`.
    fn master_page(&mut self, background: Option<Color>) -> Node {
        let v = self.v;
        let mut children = vec![chunk(
            b"flgs",
            vec![if v.props() { 0x40 } else { 0 }, 0, 1, 0x90],
        )];
        let unit = 1.0 / UNITS_PER_MM;
        let tiny = Rect::new(-2.0 * unit, -2.0 * unit, 2.0 * unit, 2.0 * unit);
        if v.props() {
            children.push(chunk(b"bbox", box_bytes(tiny)));
            children.push(chunk(b"obbx", obbx_bytes(tiny, 0.0)));
        } else {
            children.push(chunk(b"bbox", vec![0; 16]));
        }
        let layers = vec![
            self.master_layer(MasterLayer::Guides, false),
            self.master_layer(MasterLayer::Desktop, false),
            self.master_layer(MasterLayer::Grid, false),
        ];
        children.push(list(b"gobj", layers));
        match background {
            Some(c) if v.0 >= 21 => {
                children.push(list(
                    b"lgob",
                    vec![chunk(b"loda", loda(0, page_background_args(v, c)))],
                ));
            }
            _ if v.redirects() => {
                children.push(list(b"lgob", vec![chunk(b"loda", loda(0, Vec::new()))]));
            }
            _ => {}
        }
        list(b"page", children)
    }

    /// A page: its flags, the box of its objects, its entries for the
    /// master's guides and desktop layers, its own guides layer (from
    /// version 14), its layers and its entry for the master's grid layer.
    fn page(&mut self, page: &tracedraw_core::document::Page) -> (Node, Option<Rect>) {
        let v = self.v;
        let centre = Point::new(page.size.width / 2.0, page.size.height / 2.0);
        let mut layers = vec![
            self.master_layer(MasterLayer::Guides, true),
            self.master_layer(MasterLayer::Desktop, true),
        ];
        if v.zip() {
            layers.push(self.layer([0x0a, 0, 0, 0x98], LAYER_GUIDES, "Guides", Vec::new()));
        }
        let mut content: Option<Rect> = None;
        // Front to back, as files list them.
        for layer in page.layers.iter().rev() {
            let objects = if layer.visible {
                let placed = self.objects(&layer.shapes, centre, 0);
                if let Some(b) = placed.bounds {
                    content = Some(content.map_or(b, |c| c.union(b)));
                }
                placed.nodes
            } else {
                Vec::new()
            };
            layers.push(self.layer([0, 0, 0, 0x98], LAYER_PLAIN, &layer.name, objects));
        }
        layers.push(self.master_layer(MasterLayer::Grid, true));
        let page_box = content.map(|c| c - centre.to_vec2()).unwrap_or(Rect::ZERO);
        let mut children = vec![chunk(
            b"flgs",
            vec![if v.props() { 0x40 } else { 0 }, 0, 0, 0x90],
        )];
        children.push(chunk(b"bbox", box_bytes(page_box)));
        children.push(chunk(b"obbx", obbx_bytes(page_box, 0.0)));
        children.push(list(b"gobj", layers));
        (list(b"page", children), content)
    }

    /// Objects of a layer or group, front to back as files store them.
    fn objects(&mut self, shapes: &[Shape], centre: Point, depth: usize) -> Placed {
        let mut nodes = Vec::new();
        let mut bounds: Option<Rect> = None;
        if depth > 60 {
            return Placed { nodes, bounds };
        }
        for s in shapes.iter().rev() {
            let p = self.object(s, centre, depth);
            if let Some(b) = p.bounds {
                bounds = Some(bounds.map_or(b, |c| c.union(b)));
            }
            nodes.extend(p.nodes);
        }
        Placed { nodes, bounds }
    }

    /// The common head of an object or group list: flags, boxes and the
    /// static id.
    fn head(&mut self, flags: [u8; 4], page_box: Rect, outline: f64, centre: Point) -> Vec<Node> {
        let b = page_box - centre.to_vec2();
        let usdn = self.next_usdn;
        self.next_usdn += 1;
        vec![
            chunk(b"flgs", flags.to_vec()),
            chunk(b"bbox", box_bytes(b)),
            chunk(b"obbx", obbx_bytes(b, outline)),
            chunk(b"usdn", usdn.to_le_bytes().to_vec()),
        ]
    }

    fn group(&mut self, children: &[Shape], centre: Point, depth: usize) -> Placed {
        let inner = self.objects(children, centre, depth + 1);
        let Some(b) = inner.bounds else {
            return Placed {
                nodes: Vec::new(),
                bounds: None,
            };
        };
        let id = self.next_object;
        self.next_object += 1;
        let mut body = self.head([0, 0, 0, 0x10], b, 0.0, centre);
        body.push(list(
            b"lgob",
            vec![
                chunk(b"loda", {
                    // Groups carry a zero 12000 from version 13 as well.
                    let mut args = vec![(ARG_ID, id.to_le_bytes().to_vec())];
                    if self.v.props() {
                        args.push((ARG_PROPS, vec![0; 4]));
                    }
                    loda(0, args)
                }),
                list(
                    b"trfl",
                    vec![chunk(b"trfd", trfd(self.v, Affine::IDENTITY))],
                ),
            ],
        ));
        body.extend(inner.nodes);
        Placed {
            nodes: vec![list(b"grp ", body)],
            bounds: Some(b),
        }
    }

    /// A bitmap object: image rectangle, image id and crop path (the whole
    /// rectangle, or `crop` in the bitmap's local space).
    fn bitmap_object(&mut self, s: &Shape, centre: Point, crop: Option<&BezPath>) -> Placed {
        let ShapeKind::Bitmap { rect, png, .. } = &s.kind else {
            return Placed {
                nodes: Vec::new(),
                bounds: None,
            };
        };
        let (sx, sy) = {
            let c = s.transform.as_coeffs();
            (c[0].hypot(c[1]), c[2].hypot(c[3]))
        };
        let Some(image) = self.bitmap_id(png, (rect.width() * sx, rect.height() * sy)) else {
            return Placed {
                nodes: Vec::new(),
                bounds: None,
            };
        };
        let v = self.v;
        let (width_px, height_px) = match &s.kind {
            ShapeKind::Bitmap {
                width_px,
                height_px,
                ..
            } => (*width_px, *height_px),
            _ => (0, 0),
        };
        // As files of versions 7 to 27 hold it: the top-left and the
        // bottom-right corner, the image's own box the same way, 2 and
        // the bits per pixel (u16 each), the size in pixels, 4 bytes the
        // program fills with anything, the image id, then 0, 0, -1, 0, 0.
        // The program that wrote the samples shows no image when the
        // pixel size is left at zero.
        let corners = |c: &mut Buf| {
            c.i32(units(rect.x0))
                .i32(units(rect.y1))
                .i32(units(rect.x1))
                .i32(units(rect.y0));
        };
        let mut c = Buf::default();
        corners(&mut c);
        corners(&mut c);
        c.u16(2)
            .u16(24)
            .u32(width_px)
            .u32(height_px)
            .u32(0)
            .u32(image)
            .u32(0)
            .u32(0)
            .u32(u32::MAX);
        if v.0 != 8 {
            c.zeros(8);
        }
        let outline = {
            let mut p = BezPath::new();
            p.move_to((rect.x0, rect.y1));
            p.line_to((rect.x1, rect.y1));
            p.line_to((rect.x1, rect.y0));
            p.line_to((rect.x0, rect.y0));
            p.close_path();
            p
        };
        let path_bytes = match crop {
            Some(p) => curve_coords(p),
            None => curve_coords(&outline),
        };
        c.bytes(&path_bytes.unwrap_or_else(|| 0u32.to_le_bytes().to_vec()));
        let page_box = match crop {
            Some(p) => (s.transform * p.clone()).bounding_box(),
            None => (s.transform * rect.to_path(0.01)).bounding_box(),
        };
        self.place(
            s,
            OBJ_BITMAP,
            c.done(),
            &Fill::None,
            s.transform,
            page_box,
            None,
            centre,
        )
    }

    /// An object list: head, attributes with the transform and, for text,
    /// the text record (and, from version 16, the character settings for
    /// the object's style string).
    #[allow(clippy::too_many_arguments)]
    fn place(
        &mut self,
        s: &Shape,
        kind: u32,
        coords: Vec<u8>,
        fill: &Fill,
        transform: Affine,
        page_box: Rect,
        text: Option<(Vec<u8>, String, u32)>,
        centre: Point,
    ) -> Placed {
        let v = self.v;
        let fill_id = self.fill_id(fill);
        let outline_id = self.outline_id(s.stroke.as_ref());
        let id = self.next_object;
        self.next_object += 1;
        let mut args: Vec<(u32, Vec<u8>)> = vec![
            (ARG_COORDS, coords),
            (ARG_FILL, fill_id.to_le_bytes().to_vec()),
            (ARG_OUTLINE, outline_id.to_le_bytes().to_vec()),
            (ARG_ID, id.to_le_bytes().to_vec()),
        ];
        if !s.visible && v.0 >= 21 {
            args.push((ARG_HIDDEN, 1u32.to_le_bytes().to_vec()));
        }
        if v.style_strings() {
            let graded = self.graded.get(&s.id).map(|(f, t)| (f, *t));
            let mut json = object_style_json(fill, s.stroke.as_ref(), s.opacity, graded);
            if let Some((_, character, _)) = &text {
                // Text objects keep their character settings there too.
                json.pop();
                json.push(',');
                json.push_str(character);
                json.push('}');
                args.push((ARG_TEXT_LINK, 1u32.to_le_bytes().to_vec()));
            }
            let mut a = Buf::default();
            a.u32(1).bytes(&style_string(v, &json));
            args.push((ARG_STYLE_STRING, a.done()));
        } else {
            // Version 15 text carries 1 in argument 16015 as from 16 (every
            // text object of the version 15 files seen).
            if text.is_some() && v.0 == 15 {
                args.push((ARG_TEXT_LINK, 1u32.to_le_bytes().to_vec()));
            }
            // Text refers to the style of its alignment, as its paragraphs;
            // graphics to the graphic style.
            let style = text
                .as_ref()
                .map_or(GRAPHIC_STYLE_ID, |(_, _, style)| *style);
            args.push((ARG_STYLE, style.to_le_bytes().to_vec()));
        }
        if v.props() {
            args.push((ARG_PROPS, vec![0; 4]));
        }
        if let Some(n) = &s.name {
            args.push((ARG_NAME, name_bytes(v, n)));
        }
        // From version 13 the argument holds a tagged record (lenses and
        // transparencies) whose transparency layout is not known yet; the
        // older bare value made a version 15 file "contain an invalid
        // object" in the program that wrote the samples. Style strings
        // carry the transparency from 16.
        if s.opacity < 0.999 && !v.tagged() {
            args.push((ARG_OPACITY, opacity_arg(v, s.opacity)));
        }
        let outline = s.stroke.as_ref().map(|st| st.width / 2.0).unwrap_or(0.0);
        // Text objects start with a number of their own (`spnd`: 1, 2, ...
        // in the order written, as files number them) and before version
        // 18 are flagged 9 instead of 8.
        let mut body = Vec::new();
        let mut flags = [0, 2, 0, 8];
        if text.is_some() {
            body.push(chunk(b"spnd", self.next_spnd.to_le_bytes().to_vec()));
            self.next_spnd += 1;
            if v.0 < 18 {
                flags[3] = 9;
            }
        }
        body.extend(self.head(flags, page_box, outline, centre));
        let recenter = Affine::translate(-centre.to_vec2());
        body.push(list(
            b"lgob",
            vec![
                chunk(b"loda", loda(kind, args)),
                chunk(b"ftil", ftil()),
                list(b"trfl", vec![chunk(b"trfd", trfd(v, recenter * transform))]),
            ],
        ));
        if let Some((t, _, _)) = text {
            body.push(chunk(b"txsm", t));
            if v.0 >= 11 {
                body.push(chunk(b"txtj", txtj()));
            }
        }
        Placed {
            nodes: vec![list(b"obj ", body)],
            bounds: Some(page_box),
        }
    }

    fn object(&mut self, s: &Shape, centre: Point, depth: usize) -> Placed {
        let none = || Placed {
            nodes: Vec::new(),
            bounds: None,
        };
        // Hidden objects are kept from version 21 (a flag); before, and
        // for containers, they are left out.
        let keeps_hidden = self.v.0 >= 21
            && !matches!(
                s.kind,
                ShapeKind::Group { .. }
                    | ShapeKind::ClipFrame { .. }
                    | ShapeKind::Table(_)
                    | ShapeKind::SymbolInstance { .. }
            );
        if !s.visible && !keeps_hidden {
            return none();
        }
        // Paragraph text goes out as a group of artistic texts, one a line
        // placed where its frame put it (frames are not written yet), so it
        // keeps its look.
        if let Some(kinds) = tracedraw_core::document::paragraph_lines(&s.kind) {
            let children: Vec<Shape> = kinds
                .into_iter()
                .map(|kind| Shape { kind, ..s.clone() })
                .collect();
            let group = match children.len() {
                0 => return none(),
                1 => children.into_iter().next().unwrap_or_else(|| s.clone()),
                _ => Shape {
                    kind: ShapeKind::Group { children },
                    transform: Affine::IDENTITY,
                    effects: Vec::new(),
                    ..s.clone()
                },
            };
            return self.object(&group, centre, depth);
        }
        if !s.effects.is_empty() {
            let ev = tracedraw_core::live::evaluate(s);
            let mut all: Vec<Shape> = ev.below.clone();
            let mut main = ev.main.clone();
            // A graded transparency is written as a uniform one: the mean
            // opacity of its mask (as files read before masks were).
            // Version 16+ style strings keep the gradient itself.
            let graded = s.effects.iter().find_map(|e| match e {
                tracedraw_core::live::Effect::Transparency {
                    mask: Fill::Fountain(f),
                    target,
                    ..
                } => Some((f.clone(), *target)),
                _ => None,
            });
            match graded {
                Some(g) if self.v.style_strings() => {
                    self.graded.insert(main.id, g);
                }
                _ => {
                    if let Some(o) = s.effects.iter().find_map(mask_mean_opacity) {
                        main.opacity *= o;
                    }
                }
            }
            all.push(main);
            all.extend(ev.above.iter().cloned());
            for x in all.iter_mut() {
                x.effects.clear();
            }
            // A blend goes out as one plain group of what it draws: its
            // start object and its steps as curves (and its end when the
            // effect holds it), ordinary objects held together as the
            // blend is one object here. They were loose objects of the
            // layer; a blend of two texts saved so closed the program
            // that wrote the samples when the text was clicked (#946).
            let blends = s
                .effects
                .iter()
                .any(|e| matches!(e, tracedraw_core::live::Effect::Blend { .. }));
            if blends && all.len() > 1 {
                return self.group(&all, centre, depth + 1);
            }
            return self.objects(&all, centre, depth + 1);
        }
        let v = self.v;
        match &s.kind {
            ShapeKind::Group { children } => {
                let kids: Vec<Shape> = children
                    .iter()
                    .map(|c| {
                        let mut c = c.clone();
                        c.absorb(s.transform);
                        c
                    })
                    .collect();
                return self.group(&kids, centre, depth);
            }
            ShapeKind::Table(_) | ShapeKind::SymbolInstance { .. } => {
                let kids: Vec<Shape> = s
                    .expand(self.symbols)
                    .into_iter()
                    .map(|mut c| {
                        c.absorb(s.transform);
                        c
                    })
                    .collect();
                return self.group(&kids, centre, depth);
            }
            ShapeKind::ClipFrame { frame, contents }
                if contents.len() == 1
                    && matches!(contents[0].kind, ShapeKind::Bitmap { .. })
                    && contents[0].effects.is_empty() =>
            {
                // A clipped bitmap is a bitmap object with a crop path.
                let bm = &contents[0];
                let clip = bm.transform.inverse() * frame.page_path();
                let mut b = bm.clone();
                b.transform = s.transform * bm.transform;
                return self.bitmap_object(&b, centre, Some(&clip));
            }
            ShapeKind::ClipFrame { frame, contents } => {
                // Not yet written as a clip container: the frame's fill
                // under the contents and its outline over them, unclipped.
                let mut f = (**frame).clone();
                f.absorb(s.transform);
                let mut kids = Vec::new();
                if !matches!(f.fill, Fill::None) {
                    let mut under = f.clone();
                    under.stroke = None;
                    kids.push(under);
                }
                for c in contents {
                    let mut c = c.clone();
                    c.absorb(s.transform);
                    kids.push(c);
                }
                if f.stroke.is_some() {
                    f.fill = Fill::None;
                    kids.push(f);
                }
                return self.group(&kids, centre, depth);
            }
            ShapeKind::Bitmap { .. } => return self.bitmap_object(s, centre, None),
            _ => {}
        }
        let page_box = s.bounds();
        // Rectangles one round radius describes stay rectangles (before
        // version 15); other corners go out as the curve they draw.
        let plain = s.plain_rect().filter(|_| v.native_rects());
        match &s.kind {
            ShapeKind::Rect { .. } if plain.is_some() => {
                // Files draw rectangles from their top left corner down
                // (a negative height), which sets the way corners turn.
                let (rect, radius, place) = plain.unwrap_or((Rect::ZERO, 0.0, s.transform));
                let t = place * Affine::translate((rect.x0, rect.y1));
                let r = units(radius);
                let mut c = Buf::default();
                c.i32(units(rect.width())).i32(-units(rect.height())).i32(r);
                if v.0 >= 9 {
                    c.i32(r).i32(r).i32(r);
                }
                self.place(s, OBJ_RECT, c.done(), &s.fill, t, page_box, None, centre)
            }
            ShapeKind::Ellipse { rect, arc } => {
                let t = s.transform * Affine::translate((rect.x0, rect.y0));
                let (a1, a2, pie) = match arc {
                    Some(a) => (
                        (a.start_deg * 1_000_000.0).round() as i32,
                        (a.end_deg * 1_000_000.0).round() as i32,
                        u32::from(a.pie),
                    ),
                    None => (FULL_ELLIPSE_ANGLE, FULL_ELLIPSE_ANGLE, 0),
                };
                let mut c = Buf::default();
                c.i32(units(rect.width()))
                    .i32(units(rect.height()))
                    .i32(a1)
                    .i32(a2)
                    .u32(pie);
                self.place(s, OBJ_ELLIPSE, c.done(), &s.fill, t, page_box, None, centre)
            }
            // Text fitted to a path goes out as the outline of its letters
            // (below); other text as text.
            ShapeKind::Text {
                spans,
                origin,
                align,
                para,
                on_path: None,
                ..
            } => {
                // Each paragraph goes out with its own alignment (and line
                // spacing from version 16); the object takes the first
                // paragraph's.
                let own_align = *align;
                let first_align = paragraph_aligns(spans, own_align)
                    .first()
                    .copied()
                    .unwrap_or(own_align);
                let align = &first_align;
                let t = s.transform * Affine::translate(origin.to_vec2());
                self.has_text = true;
                let record = if v.0 >= 16 {
                    self.txsm16(
                        spans,
                        own_align,
                        para,
                        &s.fill,
                        Affine::translate(-centre.to_vec2()) * t,
                    )
                } else {
                    self.txsm7(spans, own_align, &s.fill)
                };
                // The character settings of the first run, the width of
                // the lines as the frame's one column (files give artistic
                // text its line width there) and the alignment.
                let width = s.local_path().bounding_box().width();
                let character = format!(
                    r#"{},"frame":{{"equalWidthColumns":"1","rtlColumns":"0","columns":"{},0"}},"paragraph":{{"justify":"{}"}}"#,
                    spans
                        .first()
                        .map(|sp| character_json(sp, Some(&s.fill)))
                        .unwrap_or_else(|| r#""character":{}"#.to_string()),
                    units(if width.is_finite() { width } else { 0.0 }).max(0),
                    justify_code(*align)
                );
                let mut c = Buf::default();
                c.i32(0).i32(0);
                self.place(
                    s,
                    OBJ_ARTISTIC_TEXT,
                    c.done(),
                    &s.fill,
                    t,
                    page_box,
                    Some((record, character, style_for(*align))),
                    centre,
                )
            }
            _ => {
                let coords = match &s.kind {
                    ShapeKind::Path { path, .. } => curve_coords_typed(path, &s.node_types),
                    _ => curve_coords(&s.local_path()),
                };
                let Some(mut coords) = coords else {
                    return none();
                };
                // The node marks of every version's files. (Version 13 and
                // 14 files written with them once closed the program that
                // read them, but those curves also lacked the 8 bytes below;
                // without the marks a version 15 file lost its filled
                // curves.)
                mark_nodes(&mut coords);
                if v.0 < 16 {
                    // Before 16 the record ends with 8 more bytes after the
                    // node bytes (zeros, or leftovers, in files of versions
                    // 11, 12 and 15). Without them the program that wrote
                    // the samples took the fill and outline ids after the
                    // record for those bytes and dropped the curve.
                    coords.extend_from_slice(&[0; 8]);
                }
                self.place(
                    s,
                    OBJ_CURVE,
                    coords,
                    &s.fill,
                    s.transform,
                    page_box,
                    None,
                    centre,
                )
            }
        }
    }

    /// Paragraphs of the text: the runs between line breaks, each run with
    /// its own style.
    fn paragraphs(spans: &[TextSpan]) -> Vec<Vec<TextSpan>> {
        let mut out: Vec<Vec<TextSpan>> = vec![Vec::new()];
        for sp in spans {
            let mut parts = sp.text.split('\n').peekable();
            while let Some(part) = parts.next() {
                if !part.is_empty() {
                    let mut run = sp.clone();
                    run.text = part.replace('\r', "");
                    if let Some(last) = out.last_mut() {
                        last.push(run);
                    }
                }
                if parts.peek().is_some() {
                    out.push(Vec::new());
                }
            }
        }
        out
    }

    /// `txsm` of versions 7 to 15: one frame, a paragraph per line, one
    /// style per run (font, encoding, style flags, size and, when it
    /// differs from the object's, the fill), the text in UTF-16 from
    /// version 12 and in Windows-1252 before.
    /// The style id of a text format (versions 7 to 15), adding it to
    /// the style table when new.
    fn text_style(&mut self, f: TextFormat) -> u32 {
        let i = match self.text_formats.iter().position(|x| *x == f) {
            Some(i) => i,
            None => {
                self.text_formats.push(f);
                self.text_formats.len() - 1
            }
        };
        GRAPHIC_STYLE_ID + 1 + i as u32
    }

    fn txsm7(&mut self, spans: &[TextSpan], align: TextAlign, fill: &Fill) -> Vec<u8> {
        let v = self.v;
        let n = v.0;
        // As files of versions 7 to 15 hold artistic text: -1, -1, 0, -1,
        // 8, a number growing with the version (2 in 7, 3 in 8, 5 in 10
        // and 11, 6 in 12, 8 in 13, 9 in 14, 11 in 15), 0, 3, 0; one frame
        // with an identity matrix (six f64). A header of zeros and a zero
        // matrix made the program that wrote the samples drop the text of
        // a version 15 file as an invalid object.
        let layout = match n {
            0..=7 => 2,
            8 => 3,
            9 => 4,
            10 | 11 => 5,
            12 => 6,
            13 => 8,
            14 => 9,
            _ => 11,
        };
        let mut t = Buf::default();
        t.u32(u32::MAX)
            .u32(u32::MAX)
            .u32(0)
            .u32(u32::MAX)
            .u32(8)
            .u32(layout)
            .u32(0)
            .u32(3)
            .u32(0);
        if n >= 15 {
            t.zeros(1);
        }
        if n < 8 {
            t.u32(0); // not on a path
        }
        // One frame, numbered as the text object (its `spnd`), as files of
        // versions 12 to 15 number it (it was 1 for every text).
        t.u32(1).u32(self.next_spnd);
        for m in [1.0f64, 0.0, 0.0, 0.0, 1.0, 0.0] {
            t.bytes(&m.to_le_bytes());
        }
        if n >= 8 {
            t.u32(0); // not on a path
            if n >= 15 {
                t.zeros(8 + 4);
            }
        }
        let paragraphs = Self::paragraphs(spans);
        t.u32(paragraphs.len() as u32);
        let aligns = paragraph_aligns(spans, align);
        for (pi, runs) in paragraphs.iter().enumerate() {
            let a = aligns.get(pi).copied().unwrap_or(align);
            let align_index = STYLE_ALIGNS.iter().position(|(x, _)| *x == a).unwrap_or(0);
            // The paragraph's look lives in a style of its own (font, size,
            // weight, colour, alignment), as files of versions 13 to 15
            // keep it; the run records then only override what differs.
            // Overrides of the font in those records were not shown by the
            // program that wrote the samples.
            let format_of = |w: &mut Self, sp: &TextSpan| TextFormat {
                font: w.font_id(&sp.font_family),
                size: (sp.size_pt * 254000.0 / 72.0).round() as i32,
                flags: style_flags(sp),
                fill: w.fill_id(sp.fill.as_ref().unwrap_or(fill)),
                align: align_index,
            };
            let base = match runs.first() {
                Some(sp) => format_of(self, sp),
                None => format_of(self, &TextSpan::new("", "Arial", 12.0)),
            };
            let style_id = self.text_style(base);
            t.u32(style_id).zeros(1);
            if n >= 13 {
                t.zeros(1);
            }
            t.u32(runs.len() as u32);
            let mut text = String::new();
            let last_run = runs.len().saturating_sub(1);
            for (ri, sp) in runs.iter().enumerate() {
                let look = format_of(self, sp);
                // The last run counts the paragraph's end too, as files
                // count it.
                let chars =
                    (sp.text.chars().count() + usize::from(ri == last_run)).min(65535) as u16;
                let mut flags = 0u8;
                if look.font != base.font && n < 13 {
                    flags |= 0x01;
                }
                if look.flags != base.flags && n < 13 {
                    flags |= 0x02;
                }
                if look.size != base.size {
                    flags |= 0x04;
                }
                if look.fill != base.fill {
                    flags |= 0x40;
                }
                t.u16(chars).u8(flags);
                if n >= 8 {
                    t.u8(0);
                }
                if flags & 0x01 != 0 {
                    t.u16(look.font).u16(0);
                }
                if flags & 0x02 != 0 {
                    t.u32(look.flags);
                }
                if flags & 0x04 != 0 {
                    t.i32(look.size);
                }
                if flags & 0x40 != 0 {
                    t.u32(look.fill);
                    if v.tagged() {
                        t.zeros(48);
                    }
                }
                text.push_str(&sp.text);
            }
            let chars = text.chars().count();
            if v.unicode() {
                // From version 12 a letter is one byte in Windows-1252
                // unless bit 0 of the first byte of its 8-byte record marks
                // it as a UTF-16 unit in two bytes; the third byte numbers
                // its run, times 2. Text all in UTF-16 with no letter
                // marked made the program that wrote the samples crash.
                let mut records = Buf::default();
                let mut bytes = Vec::new();
                let mut run_of = Vec::with_capacity(chars);
                for (ri, sp) in runs.iter().enumerate() {
                    run_of.extend(std::iter::repeat_n(ri, sp.text.chars().count()));
                }
                for (i, c) in text.chars().enumerate() {
                    let run = run_of.get(i).copied().unwrap_or(0).min(127) as u8;
                    match cp1252(&c.to_string()) {
                        Some(b) => {
                            records.u8(0).u8(0).u8(run << 1).zeros(5);
                            bytes.extend_from_slice(&b);
                        }
                        None => {
                            let unit = if (c as u32) <= 0xffff {
                                c as u16
                            } else {
                                b'?' as u16
                            };
                            records.u8(1).u8(0).u8(run << 1).zeros(5);
                            bytes.extend_from_slice(&unit.to_le_bytes());
                        }
                    }
                }
                t.u32(chars as u32).bytes(&records.done());
                t.u32(bytes.len() as u32).bytes(&bytes);
            } else {
                t.u32(chars as u32).zeros(chars * 4);
                let bytes = cp1252(&text).unwrap_or_else(|| {
                    text.chars()
                        .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
                        .collect()
                });
                t.bytes(&bytes);
            }
            t.u8(0); // no per-letter path data
        }
        t.done()
    }

    /// `txsm` of version 16 and later, as files of versions 17 to 21 store
    /// artistic text: the frame flag set, layout 1700 (also in version
    /// 16), one frame with the text's matrix, then a paragraph per line
    /// with the default style string (the first run's style and the
    /// alignment) and one record per run whose style string the letters
    /// select; text in UTF-16 (16) or one byte per letter (17 on:
    /// Windows-1252, or UTF-8 for letters it lacks).
    fn txsm16(
        &mut self,
        spans: &[TextSpan],
        align: TextAlign,
        para: &tracedraw_core::ParagraphStyle,
        fill: &Fill,
        m: Affine,
    ) -> Vec<u8> {
        let v = self.v;
        let mut t = Buf::default();
        t.u32(u32::MAX)
            .u32(u32::MAX)
            .u32(0)
            .u32(u32::MAX)
            .u32(8)
            // 12 in version 16, 13 from 17; the layout is 1700 in both,
            // as files saved for those versions have them (a version 16
            // file with 13 and 1600 was refused as holding an invalid
            // object).
            .u32(if v.0 >= 17 { 13 } else { 12 })
            .u32(0)
            .u32(3)
            .u32(0)
            .u16(1700)
            .zeros(3);
        // One frame, numbered as the text object (its `spnd`), the text's
        // matrix, not on a path, 8 unknown. A frame number other than the
        // object's own makes the program that opens the file hang.
        t.u32(1).u32(self.next_spnd);
        let c = m.as_coeffs();
        for x in [
            c[0],
            c[2],
            c[4] * UNITS_PER_MM,
            c[1],
            c[3],
            c[5] * UNITS_PER_MM,
        ] {
            t.f64(if x.is_finite() { x } else { 0.0 });
        }
        t.u32(0).zeros(8);
        let paragraphs = Self::paragraphs(spans);
        t.u32(paragraphs.len() as u32);
        let formats = tracedraw_core::paragraphs::paragraph_formats_of(spans, align, para);
        for (pi, runs) in paragraphs.iter().enumerate() {
            let format = formats.get(pi);
            let align = format.map_or(align, |f| f.align);
            for sp in runs {
                self.font_id(&sp.font_family);
            }
            let first = runs.first().or(spans.first());
            let leading = format.map_or(1.0, |f| {
                line_spacing_factor(f, first.map_or(12.0, |sp| sp.size_pt))
            });
            // The paragraph's default style is complete, as files hold it:
            // a style that leaves out the spacings makes the program that
            // opens the file hang.
            let default = full_text_style(first, Some(fill), align, leading);
            t.u32(style_for(align))
                .zeros(1)
                .u8(0)
                .bytes(&style_string(v, &default));
            // One record per run; a letter's override index selects it.
            // One record per run: its letter count (the last one counts the
            // paragraph end too), no URL, a locale, then its style: empty
            // when the run has the paragraph's own. Records of no letters
            // make the program that opens the file hang.
            t.u32(runs.len().max(1) as u32);
            let single = runs.len() <= 1;
            for (k, sp) in runs.iter().enumerate() {
                let mut n = sp.text.chars().count();
                if k + 1 == runs.len() {
                    n += 1;
                }
                let json = if single {
                    r#"{"character":{}}"#.to_string()
                } else {
                    run_style_json(sp, sp.fill.as_ref().or(Some(fill)))
                };
                t.u16(n.min(usize::from(u16::MAX)) as u16)
                    .u16(0)
                    .u16(4)
                    .bytes(&locale_bytes())
                    .bytes(&style_string(v, &json));
            }
            if runs.is_empty() {
                t.u16(1)
                    .u16(0)
                    .u16(4)
                    .bytes(&locale_bytes())
                    .bytes(&style_string(v, r#"{"character":{}}"#));
            }
            let mut text = String::new();
            let mut index = Vec::new();
            for (k, sp) in runs.iter().enumerate() {
                text.push_str(&sp.text);
                index.extend(std::iter::repeat_n(k as u8, sp.text.chars().count()));
            }
            t.u32(index.len() as u32);
            // The letters are one byte each from version 16 (files saved
            // for 16 hold them so, while their style strings are UTF-16;
            // UTF-16 letters in a version 16 file are read as half the
            // text by the program that opens it), Windows-1252, except a
            // letter Windows-1252 lacks (Cyrillic, Greek, most of Central
            // European): bit 0 of its record's flags marks it as a UTF-16
            // unit of two bytes, as from version 12 (#368). A letter beyond
            // the 16-bit range goes out as "?".
            let mut bytes = Vec::new();
            for (c, k) in text.chars().zip(&index) {
                let one = if v.0 >= 16 {
                    cp1252(&c.to_string())
                } else {
                    None
                };
                let wide = one.is_none();
                t.u16(u16::from(wide && v.0 >= 16))
                    .u8(k.saturating_mul(2))
                    .zeros(5);
                match one {
                    Some(b) => bytes.extend_from_slice(&b),
                    None => {
                        let unit = if (c as u32) <= 0xffff {
                            c as u16
                        } else {
                            u16::from(b'?')
                        };
                        bytes.extend_from_slice(&unit.to_le_bytes());
                    }
                }
            }
            t.u32(bytes.len() as u32).bytes(&bytes).u8(0);
        }
        t.done()
    }
}

mod fonts;
pub use fonts::{embedded_size, FontFace};

#[cfg(test)]
mod tests;
