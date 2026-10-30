//! A checker for written `.cdr` files (`tracedraw-cli check file.cdr`):
//! walks the chunk tree of a file of version 7 or later and reports what
//! a reader would stumble on. It is run on every file the writer's tests
//! produce (`document_to_cdr_version_with` under `cfg(test)`), so the
//! writer cannot regress on:
//!
//! - curves: a subpath that ends back at its start closes on its last
//!   node (bit 3 of its type byte), not with a closing line of no length;
//!   no segment of no length; no subpath of a move alone;
//! - boxes: the object's `bbox` holds its geometry (its nodes inside,
//!   its control points around it) taken through its transform, and the
//!   `obbx` corners lie within the box grown by half the outline width;
//! - ids: `usdn` numbers unique among objects and groups, `spnd` numbers
//!   unique among the objects that carry one, fill and outline ids unique
//!   in their tables;
//! - text: every text object has a `txsm` record with at least one
//!   paragraph, and every paragraph and run holds at least one letter;
//! - references: every fill id, outline id and style id an object or a
//!   text paragraph names exists in the tables, and a clip container's
//!   argument 8005 names a group of the document's `LIST clpt` (whose
//!   objects carry no `usdn`, as files have them);
//! - blends: a blend's linked group (`LIST lnkg` of object type 7 with a
//!   step argument) stands between two objects of its list, its ends,
//!   has a `usdn` of its own and a step count of 1 to 999 (#1192).
//!
//! Files of other programs may show findings on purpose (the `obbx` of a
//! turned object, a text the reader cannot decode); the checker is tuned
//! to what this writer produces, and reads files of versions 7 to 27.

use crate::riff::{Chunk, Tree};
use crate::{container, riff, text, Error, Result};
use std::collections::{HashMap, HashSet};

/// Millimetres per coordinate unit (1/254000 inch).
const MM_PER_UNIT: f64 = 25.4 / 254_000.0;
/// How far a box may miss the geometry (the writer rounds to units).
const BOX_TOLERANCE_MM: f64 = 0.05;

/// One problem of a file: where it was found and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The object (`obj 3`, `grp 2`) or table the finding is about.
    pub place: String,
    pub what: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.place, self.what)
    }
}

/// Check a `.cdr` file's bytes; the findings, empty when the file passes.
pub fn check_bytes(bytes: &[u8]) -> Result<Vec<Finding>> {
    let container = container::detect(bytes)?;
    let v = container.version.0;
    if v < 7 {
        return Err(Error::UnsupportedContainer(format!(
            "version {v}: the checker reads versions 7 and later"
        )));
    }
    let main = container::riff_stream(bytes, &container)?;
    let externals = container::external_streams(bytes, &container);
    let tree = riff::parse_with_externals(&main, externals, v)?;
    let mut c = Checker {
        v,
        tree: &tree,
        main: &main,
        findings: Vec::new(),
        fills: HashSet::new(),
        outlines: HashSet::new(),
        styles: HashSet::new(),
        images: HashSet::new(),
        usdn: HashMap::new(),
        spnd: HashMap::new(),
        objects: 0,
        clip_groups: HashSet::new(),
        in_clip: HashSet::new(),
    };
    c.tables();
    c.objects();
    c.linked_groups();
    Ok(c.findings)
}

struct Checker<'a> {
    v: u16,
    tree: &'a Tree,
    main: &'a [u8],
    findings: Vec<Finding>,
    fills: HashSet<u32>,
    outlines: HashSet<u32>,
    styles: HashSet<u32>,
    /// The ids of the `bmp ` records.
    images: HashSet<u32>,
    /// `usdn` number to the place that holds it.
    usdn: HashMap<u32, String>,
    spnd: HashMap<u32, String>,
    objects: usize,
    /// The `spnd` numbers of the groups of `LIST clpt` (clip contents).
    clip_groups: HashSet<u32>,
    /// The lists inside `LIST clpt`, whose objects carry no `usdn`.
    in_clip: HashSet<*const Chunk>,
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
}

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    u32_at(d, at).map(|x| x as i32)
}

fn f64_at(d: &[u8], at: usize) -> Option<f64> {
    d.get(at..at + 8)
        .and_then(|b| b.try_into().ok())
        .map(f64::from_le_bytes)
}

/// An axis-aligned box in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Box {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Box {
    fn of(points: impl IntoIterator<Item = (f64, f64)>) -> Option<Box> {
        let mut b: Option<Box> = None;
        for (x, y) in points {
            b = Some(match b {
                None => Box {
                    x0: x,
                    y0: y,
                    x1: x,
                    y1: y,
                },
                Some(b) => Box {
                    x0: b.x0.min(x),
                    y0: b.y0.min(y),
                    x1: b.x1.max(x),
                    y1: b.y1.max(y),
                },
            });
        }
        b
    }

    fn contains(&self, other: &Box, tol: f64) -> bool {
        other.x0 >= self.x0 - tol
            && other.y0 >= self.y0 - tol
            && other.x1 <= self.x1 + tol
            && other.y1 <= self.y1 + tol
    }

    fn holds_point(&self, x: f64, y: f64, tol: f64) -> bool {
        x >= self.x0 - tol && x <= self.x1 + tol && y >= self.y0 - tol && y <= self.y1 + tol
    }
}

impl std::fmt::Display for Box {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:.3}, {:.3})-({:.3}, {:.3})",
            self.x0, self.y0, self.x1, self.y1
        )
    }
}

/// A `loda` record's arguments: (type, body).
pub(crate) type Args<'a> = Vec<(u32, &'a [u8])>;

/// A `loda` record's object kind and arguments.
pub(crate) fn loda_args(d: &[u8]) -> Option<(u32, Args<'_>)> {
    let count = u32_at(d, 4)? as usize;
    let offsets = u32_at(d, 8)? as usize;
    let types = u32_at(d, 12)? as usize;
    let kind = u32_at(d, 16)?;
    if count > 256 {
        return None;
    }
    let mut args = Vec::with_capacity(count);
    for i in 0..count {
        let ty = u32_at(d, types + 4 * (count - 1 - i))?;
        let start = u32_at(d, offsets + 4 * i)? as usize;
        let end = u32_at(d, offsets + 4 * i + 4)? as usize;
        args.push((ty, d.get(start..end)?));
    }
    Some((kind, args))
}

/// The matrix of a `trfd` record: (a, b, c, d, e, f) with
/// x' = a x + c y + e and y' = b x + d y + f, e and f in millimetres.
fn trfd_matrix(d: &[u8], v: u16) -> Option<[f64; 6]> {
    let offsets = u32_at(d, 8)? as usize;
    let mut at = u32_at(d, offsets)? as usize;
    if v >= 13 {
        at += 8;
    }
    if d.get(at..at + 2)? != [8, 0] {
        return None;
    }
    at += 8;
    let a = f64_at(d, at)?;
    let c = f64_at(d, at + 8)?;
    let e = f64_at(d, at + 16)? * MM_PER_UNIT;
    let b = f64_at(d, at + 24)?;
    let dd = f64_at(d, at + 32)?;
    let f = f64_at(d, at + 40)? * MM_PER_UNIT;
    [a, b, c, dd, e, f]
        .iter()
        .all(|x| x.is_finite())
        .then_some([a, b, c, dd, e, f])
}

fn apply(m: &[f64; 6], x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

/// The box a `bbox` or `obbx` chunk holds: pairs of coordinates.
fn boxes(d: &[u8]) -> Vec<Box> {
    d.chunks_exact(16)
        .filter_map(|q| {
            let x0 = i32_at(q, 0)? as f64 * MM_PER_UNIT;
            let y0 = i32_at(q, 4)? as f64 * MM_PER_UNIT;
            let x1 = i32_at(q, 8)? as f64 * MM_PER_UNIT;
            let y1 = i32_at(q, 12)? as f64 * MM_PER_UNIT;
            Box::of([(x0, y0), (x1, y1)])
        })
        .collect()
}

impl<'a> Checker<'a> {
    fn note(&mut self, place: &str, what: impl Into<String>) {
        self.findings.push(Finding {
            place: place.to_string(),
            what: what.into(),
        });
    }

    fn data(&self, c: &Chunk) -> &'a [u8] {
        self.tree.data(self.main, c)
    }

    /// The fill, outline, style and bitmap tables.
    fn tables(&mut self) {
        let mut fills = Vec::new();
        let mut outlines = Vec::new();
        let mut images = Vec::new();
        let mut stlt = None;
        self.tree.root.walk(&mut |c, _| {
            if c.is(b"fild") {
                fills.push(self.data(c));
            } else if c.is(b"outl") {
                outlines.push(self.data(c));
            } else if c.is(b"bmp ") {
                images.push(self.data(c));
            } else if c.is(b"stlt") && stlt.is_none() {
                stlt = Some(self.data(c));
            }
        });
        for d in images {
            match u32_at(d, 0) {
                Some(id) if !self.images.insert(id) => {
                    self.note("bmp ", format!("image id {id} used by two records"));
                }
                Some(_) => {}
                None => self.note("bmp ", "record shorter than its id"),
            }
        }
        for (name, records) in [("fild", &fills), ("outl", &outlines)] {
            let mut seen = HashSet::new();
            for d in records {
                let Some(id) = u32_at(d, 0) else {
                    self.note(name, "record shorter than its id");
                    continue;
                };
                if !seen.insert(id) {
                    self.note(name, format!("id {id} used by two records"));
                }
            }
            if name == "fild" {
                self.fills = seen;
            } else {
                self.outlines = seen;
            }
        }
        if let Some(d) = stlt {
            let mut warnings = Vec::new();
            let recs = text::read_stlt(d, self.v, &mut |w| warnings.push(w));
            for w in warnings {
                self.note("stlt", w);
            }
            let mut seen = HashSet::new();
            for (id, _) in recs {
                if !seen.insert(id) {
                    self.note("stlt", format!("style {id} defined twice"));
                }
            }
            self.styles = seen;
        }
    }

    /// Every object and group list of every page.
    fn objects(&mut self) {
        let mut lists = Vec::new();
        let mut clips = Vec::new();
        self.tree.root.walk(&mut |c, _| {
            if c.is(b"obj ") || c.is(b"grp ") {
                lists.push(c);
            } else if c.is(b"clpt") {
                clips.push(c);
            }
        });
        for clip in clips {
            for g in clip.children.iter().filter(|g| g.is(b"grp ")) {
                if let Some(n) = g.find(b"spnd").and_then(|s| u32_at(self.data(s), 0)) {
                    self.clip_groups.insert(n);
                }
            }
            clip.walk(&mut |c, _| {
                if c.is(b"obj ") || c.is(b"grp ") {
                    self.in_clip.insert(c as *const Chunk);
                }
            });
        }
        for c in lists {
            self.objects += 1;
            let n = self.objects;
            let place = if c.is(b"grp ") {
                format!("grp {n}")
            } else {
                format!("obj {n}")
            };
            self.object(c, &place);
        }
    }

    /// Every blend's linked group of every page (#1192): an empty `LIST
    /// lnkg` whose `loda` is of object type 7 with a step argument. A
    /// reader takes the objects just before and after it in the list as
    /// its ends, so both must be there; the step count is 1 to 999 (what
    /// the program saves; another value reads as another layout); its
    /// `usdn` counts among the objects'.
    fn linked_groups(&mut self) {
        fn collect<'a>(c: &'a Chunk, out: &mut Vec<(&'a Chunk, usize)>) {
            for (i, k) in c.children.iter().enumerate() {
                if k.is(b"lnkg") {
                    out.push((c, i));
                }
                collect(k, out);
            }
        }
        let mut found = Vec::new();
        collect(&self.tree.root, &mut found);
        let mut n = 0;
        for (parent, i) in found {
            let c = &parent.children[i];
            let Some(loda) = c.find(b"lgob").and_then(|l| l.find(b"loda")) else {
                continue;
            };
            let Some((kind, args)) = loda_args(self.data(loda)) else {
                continue;
            };
            if kind != 7 {
                continue;
            }
            let Some((_, body)) = args
                .iter()
                .find(|(ty, _)| *ty == crate::parse::ARG_BLEND_STEPS)
            else {
                continue;
            };
            n += 1;
            let place = format!("lnkg {n}");
            match c.find(b"usdn").and_then(|u| u32_at(self.data(u), 0)) {
                Some(u) => {
                    if let Some(other) = self.usdn.insert(u, place.clone()) {
                        self.note(&place, format!("usdn {u} is also {other}'s"));
                    }
                }
                None => self.note(&place, "no usdn chunk"),
            }
            let is_object = |k: &Chunk| k.is(b"obj ") || k.is(b"grp ");
            let before = i
                .checked_sub(1)
                .and_then(|j| parent.children.get(j))
                .is_some_and(is_object);
            let after = parent.children.get(i + 1).is_some_and(is_object);
            if !(before && after) {
                self.note(
                    &place,
                    "a blend's linked group without an object before and after it (its ends)",
                );
            }
            match body.get(0..2).map(|b| u16::from_le_bytes([b[0], b[1]])) {
                Some(1..=999) => {}
                Some(x) => self.note(&place, format!("blend step count {x}, not 1 to 999")),
                None => self.note(&place, "blend argument shorter than a step count"),
            }
        }
    }

    fn object(&mut self, c: &'a Chunk, place: &str) {
        let is_group = c.is(b"grp ");
        // Ids.
        if let Some(u) = c.find(b"usdn").and_then(|u| u32_at(self.data(u), 0)) {
            if let Some(other) = self.usdn.insert(u, place.to_string()) {
                self.note(place, format!("usdn {u} is also {other}'s"));
            }
        } else if !self.in_clip.contains(&(c as *const Chunk)) {
            self.note(place, "no usdn chunk");
        }
        if let Some(s) = c.find(b"spnd").and_then(|u| u32_at(self.data(u), 0)) {
            if s != 0 {
                if let Some(other) = self.spnd.insert(s, place.to_string()) {
                    self.note(place, format!("spnd {s} is also {other}'s"));
                }
            }
        }
        let Some(loda) = c.find(b"lgob").and_then(|l| l.find(b"loda")) else {
            self.note(place, "no loda record");
            return;
        };
        let loda = self.data(loda);
        let Some((kind, args)) = loda_args(loda) else {
            self.note(place, "loda record not readable");
            return;
        };
        let matrix = c
            .find(b"lgob")
            .and_then(|l| l.find(b"trfl"))
            .and_then(|l| l.find(b"trfd"))
            .and_then(|t| trfd_matrix(self.data(t), self.v))
            .unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let mut coords: Option<&[u8]> = None;
        let mut outline_mm = 0.0;
        for (ty, body) in &args {
            match *ty {
                crate::parse::ARG_FILL => match u32_at(body, 0) {
                    Some(id) if !self.fills.contains(&id) => {
                        self.note(place, format!("fill {id} is not in the fill table"));
                    }
                    Some(_) => {}
                    None => self.note(place, "fill argument shorter than an id"),
                },
                crate::parse::ARG_OUTLINE => match u32_at(body, 0) {
                    Some(id) if !self.outlines.contains(&id) => {
                        self.note(place, format!("outline {id} is not in the outline table"));
                    }
                    Some(_) => {
                        outline_mm = self.outline_width(u32_at(body, 0).unwrap_or(0));
                    }
                    None => self.note(place, "outline argument shorter than an id"),
                },
                crate::parse::ARG_STYLE => match u32_at(body, 0) {
                    Some(id) if id != 0 && !self.styles.contains(&id) => {
                        self.note(place, format!("style {id} is not in the style table"));
                    }
                    _ => {}
                },
                crate::parse::ARG_COORDS => coords = Some(body),
                crate::parse::ARG_CLIP_CONTENTS => match u32_at(body, 0) {
                    Some(n) if !self.clip_groups.contains(&n) => {
                        self.note(
                            place,
                            format!("clip container names group {n}, not in LIST clpt"),
                        );
                    }
                    Some(_) => {}
                    None => self.note(place, "clip contents argument shorter than a number"),
                },
                _ => {}
            }
        }
        if is_group {
            return;
        }
        // Geometry.
        // The box the object's nodes span (inside its bbox) and the box
        // all its points span (around its bbox); an arc has no node box.
        // A bitmap's crop outline is taken through the matrix point by
        // point (a turned outline's box is not its box's turn).
        let identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mut through = &matrix;
        let geometry: Option<(Option<Box>, Box)> = match kind {
            crate::parse::OBJ_CURVE => coords
                .and_then(|d| self.curve(d, place))
                .map(|(inner, outer)| (Some(inner), outer)),
            crate::parse::OBJ_RECT if self.v < 15 => coords.and_then(|d| {
                let w = i32_at(d, 0)? as f64 * MM_PER_UNIT;
                let h = i32_at(d, 4)? as f64 * MM_PER_UNIT;
                let b = Box::of([(0.0, 0.0), (w, h)])?;
                Some((Some(b), b))
            }),
            crate::parse::OBJ_ELLIPSE => coords.and_then(|d| {
                let w = i32_at(d, 0)? as f64 * MM_PER_UNIT;
                let h = i32_at(d, 4)? as f64 * MM_PER_UNIT;
                let b = Box::of([(0.0, 0.0), (w, h)])?;
                let whole = i32_at(d, 8)? == i32_at(d, 12)?;
                // The corners of a whole ellipse's box stand for its
                // nodes only while the box is not turned or slanted: a
                // turned ellipse (the frame of a clip container, #1222)
                // reaches less far than its box's corners.
                let turned = matrix[1].abs() > 1e-9 || matrix[2].abs() > 1e-9;
                Some(((whole && !turned).then_some(b), b))
            }),
            crate::parse::OBJ_BITMAP => {
                through = &identity;
                coords.and_then(|d| self.bitmap(d, place, &matrix))
            }
            _ => None,
        };
        if let Some((inner, outer)) = geometry {
            self.boxes(c, place, through, inner, outer, outline_mm);
        }
        // Text.
        if kind == crate::parse::OBJ_ARTISTIC_TEXT || kind == crate::parse::OBJ_PARAGRAPH_TEXT {
            self.text(c, place);
        }
    }

    /// A bitmap object's coordinates argument, as files of versions 14,
    /// 18, 19 and 27 hold it (the origin and the number checked only from 14): the
    /// image's box from the origin (top left, bottom
    /// right) twice, 2 and the bits per pixel, the pixel size, the
    /// image's number (never 0), the id of a `bmp ` record, 0, 0, -1, 0,
    /// 0, then the crop outline (count, i32 points, a node byte each) and
    /// from version 14 four zero bytes. Returns the outline's boxes (the
    /// nodes', the points') taken through `m`, which the object's `bbox`
    /// holds, when the record reads.
    fn bitmap(&mut self, d: &[u8], place: &str, m: &[f64; 6]) -> Option<(Option<Box>, Box)> {
        let Some(corners) = (0..4)
            .map(|k| i32_at(d, k * 4))
            .collect::<Option<Vec<i32>>>()
        else {
            self.note(place, "bitmap record shorter than its corners");
            return None;
        };
        // Only files of versions 14, 18, 19 and 27 were seen to hold the box at
        // the origin and a non-zero number; older ones are not held to it.
        let seen = self.v >= 14;
        if seen && (corners[0] != 0 || corners[3] != 0) {
            self.note(
                place,
                format!(
                    "bitmap box starts at ({}, {}), not at the origin",
                    corners[0], corners[3]
                ),
            );
        }
        if corners[2] <= corners[0] || corners[1] <= corners[3] {
            self.note(place, "bitmap box has no size");
        }
        if (0..4).any(|k| i32_at(d, 16 + k * 4) != Some(corners[k])) {
            self.note(place, "bitmap image box differs from its corners");
        }
        if u32_at(d, 36).is_none_or(|w| w == 0) || u32_at(d, 40).is_none_or(|h| h == 0) {
            self.note(place, "bitmap pixel size is zero");
        }
        if seen && u32_at(d, 44) == Some(0) {
            self.note(place, "bitmap image number is 0");
        }
        match u32_at(d, 48) {
            Some(id) if !self.images.contains(&id) => {
                self.note(
                    place,
                    format!("bitmap names image {id}, which has no record"),
                );
            }
            Some(_) => {}
            None => self.note(place, "bitmap record shorter than its image id"),
        }
        let tail_at = if self.v == 8 { 64 } else { 72 };
        let Some(n) = u32_at(d, tail_at) else {
            self.note(place, "bitmap record has no crop outline");
            return None;
        };
        let n = n as usize;
        if !(3..=100_000).contains(&n) {
            self.note(place, format!("bitmap crop outline of {n} points"));
            return None;
        }
        let points_at = tail_at + 4;
        let types_at = points_at + n * 8;
        let end = types_at + n + if self.v >= 14 { 4 } else { 0 };
        if d.len() != end {
            self.note(
                place,
                format!(
                    "bitmap record is {} bytes, {end} expected for {n} points",
                    d.len()
                ),
            );
        } else if self.v >= 14 && u32_at(d, types_at + n) != Some(0) {
            self.note(place, "bitmap record does not end with 4 zero bytes");
        }
        // The outline's first node carries the marks of a curve's (bit 2,
        // and bit 3 when the outline closes), as every bitmap of the
        // program's files has them; a bitmap without them closed that
        // program when it was clicked (#1226).
        if let (Some(&first), Some(&last)) = (d.get(types_at), d.get(types_at + n - 1)) {
            let want = if last & 0x08 != 0 { 0x0c } else { 0x04 };
            if first & want != want {
                self.note(
                    place,
                    format!(
                        "bitmap crop outline's first node byte {first:#04x} lacks the node marks"
                    ),
                );
            }
        }
        // Nodes inside the box, control points (type bits 7-6 set) around.
        let point = |k: usize| {
            Some(apply(
                m,
                i32_at(d, points_at + k * 8)? as f64 * MM_PER_UNIT,
                i32_at(d, points_at + k * 8 + 4)? as f64 * MM_PER_UNIT,
            ))
        };
        let control = |k: usize| d.get(types_at + k).is_some_and(|t| t >> 6 == 3);
        let outer = Box::of((0..n).filter_map(point))?;
        let inner = Box::of((0..n).filter(|k| !control(*k)).filter_map(point));
        Some((inner, outer))
    }

    /// The width of an outline of the table, in millimetres (0 when it is
    /// not readable).
    fn outline_width(&self, id: u32) -> f64 {
        let mut width = 0.0;
        self.tree.root.walk(&mut |c, _| {
            if c.is(b"outl") {
                let d = self.data(c);
                if u32_at(d, 0) != Some(id) {
                    return;
                }
                let mut at = 4;
                if self.v >= 13 {
                    // Tagged records until id 1.
                    for _ in 0..64 {
                        let (Some(tag), Some(len)) = (u32_at(d, at), u32_at(d, at + 4)) else {
                            return;
                        };
                        at += 8;
                        if tag == 1 {
                            break;
                        }
                        at += len as usize;
                    }
                }
                // line type, caps, join (u16 each), 2 bytes in 6 to 12, width.
                let line_type = d.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
                at += 6;
                if (6..13).contains(&self.v) {
                    at += 2;
                }
                if line_type.is_some_and(|t| t & 1 == 0) {
                    width = i32_at(d, at).unwrap_or(0).max(0) as f64 * MM_PER_UNIT;
                }
            }
        });
        width
    }

    /// A curve's points: the boxes of its nodes and of all its points
    /// (control points included), after checking its subpaths.
    fn curve(&mut self, d: &[u8], place: &str) -> Option<(Box, Box)> {
        let n = u32_at(d, 0)? as usize;
        if n == 0 {
            self.note(place, "curve with no points");
            return None;
        }
        let points_end = 4usize.checked_add(n.checked_mul(8)?)?;
        let types = d.get(points_end..points_end + n)?;
        let mut pts = Vec::with_capacity(n);
        for i in 0..n {
            let x = i32_at(d, 4 + 8 * i)? as f64 * MM_PER_UNIT;
            let y = i32_at(d, 8 + 8 * i)? as f64 * MM_PER_UNIT;
            pts.push((x, y));
        }
        let mut nodes = Vec::new();
        let mut start = 0usize;
        let mut last_node: Option<(f64, f64)> = None;
        let mut i = 0usize;
        let same =
            |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9;
        while i < n {
            let t = types[i];
            match t & 0xc0 {
                0x00 => {
                    // A move: ends the subpath before it.
                    if i > 0 {
                        self.subpath_end(place, &pts, types, start, i - 1);
                    }
                    start = i;
                    nodes.push(pts[i]);
                    last_node = Some(pts[i]);
                    i += 1;
                }
                0x40 => {
                    if let Some(l) = last_node {
                        if same(l, pts[i]) {
                            self.note(place, format!("line of no length at point {i}"));
                        }
                    }
                    nodes.push(pts[i]);
                    last_node = Some(pts[i]);
                    i += 1;
                }
                0xc0 => {
                    // Two control points then the curve's end.
                    let (Some(c2), Some(end)) = (pts.get(i + 1), pts.get(i + 2)) else {
                        self.note(place, format!("curve segment cut short at point {i}"));
                        return None;
                    };
                    if types.get(i + 1).map(|t| t & 0xc0) != Some(0xc0)
                        || types.get(i + 2).map(|t| t & 0xc0) != Some(0x80)
                    {
                        self.note(place, format!("curve segment malformed at point {i}"));
                        return None;
                    }
                    if let Some(l) = last_node {
                        if same(l, pts[i]) && same(l, *c2) && same(l, *end) {
                            self.note(place, format!("curve of no length at point {i}"));
                        }
                    }
                    nodes.push(*end);
                    last_node = Some(*end);
                    i += 3;
                }
                _ => {
                    self.note(
                        place,
                        format!("curve end without its control points at point {i}"),
                    );
                    return None;
                }
            }
        }
        self.subpath_end(place, &pts, types, start, n - 1);
        let inner = Box::of(nodes.iter().copied())?;
        let outer = Box::of(pts.iter().copied())?;
        Some((inner, outer))
    }

    /// A subpath from `start` to `last`: a move alone is nothing to draw;
    /// one that ends back at its start must close there (bit 3 on the
    /// last node) instead of running a closing line of no length.
    fn subpath_end(
        &mut self,
        place: &str,
        pts: &[(f64, f64)],
        types: &[u8],
        start: usize,
        last: usize,
    ) {
        if last == start {
            self.note(place, format!("subpath of a move alone at point {start}"));
            return;
        }
        let closed = types[last] & 0x08 != 0;
        let back_at_start =
            (pts[last].0 - pts[start].0).abs() < 1e-9 && (pts[last].1 - pts[start].1).abs() < 1e-9;
        if back_at_start && !closed {
            self.note(
                place,
                format!("subpath ends back at its start without closing on its last node (point {last})"),
            );
        }
        if closed && types[start] & 0x08 == 0 {
            self.note(
                place,
                format!("closed subpath whose move (point {start}) lacks the close bit"),
            );
        }
    }

    /// `bbox` holds the geometry's nodes and lies within its control
    /// points, both through the object's transform; `obbx` corners lie
    /// within the box grown by half the outline.
    fn boxes(
        &mut self,
        c: &Chunk,
        place: &str,
        m: &[f64; 6],
        inner: Option<Box>,
        outer: Box,
        outline_mm: f64,
    ) {
        let corners = |b: &Box| [(b.x0, b.y0), (b.x1, b.y0), (b.x1, b.y1), (b.x0, b.y1)];
        let through = |b: &Box| Box::of(corners(b).iter().map(|&(x, y)| apply(m, x, y)));
        let Some(outer) = through(&outer) else {
            return;
        };
        let inner = inner.and_then(|b| through(&b));
        let bbox = c.find(b"bbox").map(|b| boxes(self.data(b)));
        let Some(bbox) = bbox.as_ref().and_then(|b| b.first().copied()) else {
            self.note(place, "no bbox chunk");
            return;
        };
        let tol = BOX_TOLERANCE_MM;
        if let Some(inner) = inner {
            if !bbox.contains(&inner, tol) {
                self.note(
                    place,
                    format!("bbox {bbox} does not hold the nodes {inner}"),
                );
            }
        }
        if !outer.contains(&bbox, tol) {
            self.note(
                place,
                format!("bbox {bbox} reaches beyond the points {outer}"),
            );
        }
        let Some(obbx) = c.find(b"obbx").map(|b| self.data(b)) else {
            self.note(place, "no obbx chunk");
            return;
        };
        let pad = outline_mm / 2.0 + tol;
        for (i, q) in obbx.chunks_exact(8).enumerate() {
            let (Some(x), Some(y)) = (i32_at(q, 0), i32_at(q, 4)) else {
                continue;
            };
            let (x, y) = (x as f64 * MM_PER_UNIT, y as f64 * MM_PER_UNIT);
            if !bbox.holds_point(x, y, pad) {
                self.note(
                    place,
                    format!("obbx corner {i} ({x:.3}, {y:.3}) lies outside bbox {bbox} grown by {pad:.3}"),
                );
            }
        }
    }

    /// A text object's record: at least a paragraph, each with letters.
    fn text(&mut self, c: &Chunk, place: &str) {
        let mut txsm = None;
        c.walk(&mut |x, _| {
            if txsm.is_none() && x.is(b"txsm") {
                txsm = Some(x);
            }
        });
        let Some(t) = txsm else {
            self.note(place, "text object without a txsm record");
            return;
        };
        let d = self.data(t);
        let mut warnings = Vec::new();
        let parsed = text::read_txsm(d, self.v, 0, &mut |w| warnings.push(w));
        for w in warnings {
            self.note(place, format!("txsm: {w}"));
        }
        let Some(parsed) = parsed else {
            self.note(place, "txsm record not readable");
            return;
        };
        if parsed.paragraphs.is_empty() {
            self.note(place, "text record without paragraphs");
        }
        for (i, p) in parsed.paragraphs.iter().enumerate() {
            if p.style_id != 0 && !self.styles.contains(&p.style_id) {
                self.note(
                    place,
                    format!(
                        "paragraph {i} names style {} which is not in the style table",
                        p.style_id
                    ),
                );
            }
            if p.runs.is_empty() {
                self.note(place, format!("paragraph {i} has no runs"));
            }
            let letters: usize = p.runs.iter().map(|r| r.text.chars().count()).sum();
            if letters == 0 {
                self.note(place, format!("paragraph {i} has no letters"));
            }
            for (k, r) in p.runs.iter().enumerate() {
                if r.text.is_empty() {
                    self.note(place, format!("paragraph {i}, run {k} has no letters"));
                }
            }
        }
    }
}

/// Panics when `bytes` has findings: the writer's tests run it on every
/// file they produce.
#[cfg(test)]
pub(crate) fn assert_clean(bytes: &[u8], version: u16) {
    match check_bytes(bytes) {
        Ok(findings) if findings.is_empty() => {}
        Ok(findings) => {
            let lines: Vec<String> = findings.iter().map(ToString::to_string).collect();
            panic!(
                "version {version}: the written file has findings:\n{}",
                lines.join("\n")
            );
        }
        Err(e) => panic!("version {version}: the written file cannot be checked: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::tests::{
        chunk, i32s, list, loda, mcfg, riff, solid_fild_v13, txsm7, u16s, u32s, Style7,
    };
    use crate::parse::{
        ARG_COORDS, ARG_FILL, ARG_OUTLINE, ARG_STYLE, OBJ_ARTISTIC_TEXT, OBJ_CURVE, OBJ_ELLIPSE,
    };

    const V: u16 = 13;
    const UNITS: f64 = 254_000.0 / 25.4;

    fn units(mm: f64) -> i32 {
        (mm * UNITS).round() as i32
    }

    /// A version 13 `trfd` with a translation.
    fn trfd(tx: f64, ty: f64) -> Vec<u8> {
        let mut rec = u32s(&[1300, 0]);
        rec.extend_from_slice(&u16s(&[8]));
        rec.extend_from_slice(&[0; 6]);
        for x in [1.0, 0.0, tx * UNITS, 0.0, 1.0, ty * UNITS] {
            rec.extend_from_slice(&f64::to_le_bytes(x));
        }
        rec.extend_from_slice(&[0; 8]);
        let end = 24 + rec.len() as u32;
        let mut b = u32s(&[end, 1, 16, 0xffff, 24, end]);
        b.extend_from_slice(&rec);
        b
    }

    fn box_bytes(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<u8> {
        i32s(&[units(x0), units(y1), units(x1), units(y0)])
    }

    /// A curve's coordinates: points in millimetres with their type bytes.
    fn curve(points: &[(f64, f64, u8)]) -> Vec<u8> {
        let mut b = u32s(&[points.len() as u32]);
        for (x, y, _) in points {
            b.extend_from_slice(&i32s(&[units(*x), units(*y)]));
        }
        for (_, _, t) in points {
            b.push(*t);
        }
        b
    }

    /// A version 13 outline record: id, then its tagged body.
    fn outl(id: u32, width_mm: f64) -> Vec<u8> {
        let mut body = u16s(&[2, 0, 0]);
        body.extend_from_slice(&i32s(&[units(width_mm)]));
        body.extend_from_slice(&[0; 120]);
        let mut b = u32s(&[id, 1, body.len() as u32]);
        b.extend_from_slice(&body);
        b
    }

    /// A style table with one style, 77, of fill and outline only.
    fn stlt() -> Vec<u8> {
        let mut st = u32s(&[1]);
        for _ in 0..12 {
            st.extend_from_slice(&u32s(&[0]));
        }
        st.extend_from_slice(&u32s(&[1, 77, 0, 0, 0, 0, 0, 0]));
        list(b"stlt", &st)
    }

    struct Obj {
        kind: u32,
        args: Vec<(u32, Vec<u8>)>,
        bbox: Vec<u8>,
        obbx: Vec<u8>,
        usdn: u32,
        at: (f64, f64),
        txsm: Option<Vec<u8>>,
    }

    fn obj(o: &Obj) -> Vec<u8> {
        let mut body = chunk(b"flgs", &[0, 2, 0, 8]);
        body.extend(chunk(b"bbox", &o.bbox));
        body.extend(chunk(b"obbx", &o.obbx));
        body.extend(chunk(b"usdn", &u32s(&[o.usdn])));
        body.extend(list(
            b"lgob",
            &[
                chunk(b"loda", &loda(o.kind, &o.args)),
                list(b"trfl", &chunk(b"trfd", &trfd(o.at.0, o.at.1))),
            ]
            .concat(),
        ));
        if let Some(t) = &o.txsm {
            body.extend(chunk(b"txsm", t));
        }
        list(b"obj ", &body)
    }

    fn file(objects: &[Obj]) -> Vec<u8> {
        let mut page = Vec::new();
        for o in objects {
            page.extend(obj(o));
        }
        file_of_page(page)
    }

    /// A file whose only layer holds `page` (object lists and linked
    /// groups).
    fn file_of_page(page: Vec<u8>) -> Vec<u8> {
        let body = [
            chunk(b"vrsn", &u16s(&[1300])),
            chunk(b"mcfg", &mcfg(V, 2_540_000, 2_540_000)),
            chunk(b"fild", &solid_fild_v13(1, (255, 0, 0))),
            chunk(b"outl", &outl(7, 1.0)),
            stlt(),
            list(b"page", &list(b"layr", &page)),
        ]
        .concat();
        riff(b"CDRD", &body)
    }

    /// A closed square of 10 mm at (5, 5), closing on its last node.
    fn square() -> Obj {
        let pts = [
            (0.0, 0.0, 0x08),
            (10.0, 0.0, 0x40),
            (10.0, 10.0, 0x40),
            (0.0, 10.0, 0x40),
            (0.0, 0.0, 0x48),
        ];
        let b = box_bytes(5.0, 5.0, 15.0, 15.0);
        let mut obbx = b.clone();
        obbx.extend(box_bytes(4.5, 4.5, 15.5, 15.5));
        Obj {
            kind: OBJ_CURVE,
            args: vec![
                (ARG_COORDS, curve(&pts)),
                (ARG_FILL, u32s(&[1])),
                (ARG_OUTLINE, u32s(&[7])),
                (ARG_STYLE, u32s(&[77])),
            ],
            bbox: b,
            obbx,
            usdn: 1,
            at: (5.0, 5.0),
            txsm: None,
        }
    }

    fn text() -> Obj {
        let styles = [Style7 {
            chars: 3,
            font: None,
            flags: None,
            size_units: None,
            fill: None,
        }];
        Obj {
            kind: OBJ_ARTISTIC_TEXT,
            args: vec![
                (ARG_COORDS, i32s(&[0, 0])),
                (ARG_FILL, u32s(&[1])),
                (ARG_STYLE, u32s(&[77])),
            ],
            bbox: box_bytes(0.0, 0.0, 5.0, 2.0),
            obbx: [box_bytes(0.0, 0.0, 5.0, 2.0), box_bytes(0.0, 0.0, 5.0, 2.0)].concat(),
            usdn: 2,
            at: (0.0, 0.0),
            txsm: Some(txsm7(V, false, &styles, "Hi")),
        }
    }

    fn findings(objects: &[Obj]) -> Vec<String> {
        check_bytes(&file(objects))
            .expect("checked")
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn a_sound_file_has_no_findings() {
        assert_eq!(findings(&[square(), text()]), Vec::<String>::new());
    }

    /// A blend's linked group as the writer lays it out from version 9
    /// (#1192): `usdn` 9, a `loda` of type 7 with the link argument and
    /// the step argument holding `count`.
    fn blend_group(count: u16) -> Vec<u8> {
        let mut arg = u16s(&[count, 0]);
        arg.extend_from_slice(&i32s(&[254_000]));
        arg.resize(88, 0);
        let mut body = chunk(b"spnd", &u32s(&[1]));
        body.extend(chunk(b"flgs", &[1, 0, 0, 0x18]));
        body.extend(chunk(b"bbox", &box_bytes(5.0, 5.0, 40.0, 40.0)));
        body.extend(chunk(b"obbx", &box_bytes(5.0, 5.0, 40.0, 40.0)));
        body.extend(chunk(b"usdn", &u32s(&[9])));
        body.extend(list(
            b"lgob",
            &chunk(b"loda", &loda(7, &[(12020, u32s(&[2, 0, 0])), (3000, arg)])),
        ));
        list(b"lnkg", &body)
    }

    /// A blend's linked group stands between its two ends, with a `usdn`
    /// of its own and a step count the program can save (#1192).
    #[test]
    fn a_blend_group_stands_between_its_ends() {
        let mut second = square();
        second.usdn = 2;
        second.at = (30.0, 30.0);
        second.bbox = box_bytes(30.0, 30.0, 40.0, 40.0);
        second.obbx = [second.bbox.clone(), box_bytes(29.5, 29.5, 40.5, 40.5)].concat();
        let strings = |bytes: Vec<u8>| -> Vec<String> {
            check_bytes(&bytes)
                .expect("checked")
                .iter()
                .map(ToString::to_string)
                .collect()
        };
        let sound = [obj(&square()), blend_group(20), obj(&second)].concat();
        assert_eq!(strings(file_of_page(sound)), Vec::<String>::new());
        let last = [obj(&square()), obj(&second), blend_group(20)].concat();
        let f = strings(file_of_page(last));
        assert!(
            f.iter()
                .any(|f| f
                    .contains("lnkg 1: a blend's linked group without an object before and after")),
            "{f:?}"
        );
        let none = [obj(&square()), blend_group(0), obj(&second)].concat();
        let f = strings(file_of_page(none));
        assert!(
            f.iter()
                .any(|f| f.contains("blend step count 0, not 1 to 999")),
            "{f:?}"
        );
        let mut taken = square();
        taken.usdn = 9;
        let shared = [obj(&taken), blend_group(1), obj(&second)].concat();
        let f = strings(file_of_page(shared));
        assert!(
            f.iter().any(|f| f.contains("usdn 9 is also obj 1's")),
            "{f:?}"
        );
    }

    #[test]
    fn curves_must_close_on_their_last_node_and_have_no_empty_segments() {
        let mut s = square();
        // A closing line of no length after a node back at the start.
        let pts = [
            (0.0, 0.0, 0x08),
            (10.0, 0.0, 0x40),
            (10.0, 10.0, 0x40),
            (0.0, 10.0, 0x40),
            (0.0, 0.0, 0x40),
            (0.0, 0.0, 0x48),
        ];
        s.args[0] = (ARG_COORDS, curve(&pts));
        let f = findings(&[s]);
        assert!(
            f.iter().any(|f| f.contains("line of no length at point 5")),
            "{f:?}"
        );
        // Back at the start without the close bit.
        let mut s = square();
        let pts = [
            (0.0, 0.0, 0x00),
            (10.0, 0.0, 0x40),
            (10.0, 10.0, 0x40),
            (0.0, 0.0, 0x40),
        ];
        s.args[0] = (ARG_COORDS, curve(&pts));
        let f = findings(&[s]);
        assert!(
            f.iter()
                .any(|f| f.contains("without closing on its last node")),
            "{f:?}"
        );
        // A move alone.
        let mut s = square();
        let pts = [(0.0, 0.0, 0x00), (3.0, 3.0, 0x00), (10.0, 10.0, 0x40)];
        s.args[0] = (ARG_COORDS, curve(&pts));
        let f = findings(&[s]);
        assert!(
            f.iter().any(|f| f.contains("move alone at point 0")),
            "{f:?}"
        );
        // A curve segment of no length.
        let mut s = square();
        let pts = [
            (0.0, 0.0, 0x00),
            (10.0, 0.0, 0x40),
            (10.0, 0.0, 0xc0),
            (10.0, 0.0, 0xc0),
            (10.0, 0.0, 0x80),
            (10.0, 10.0, 0x40),
        ];
        s.args[0] = (ARG_COORDS, curve(&pts));
        let f = findings(&[s]);
        assert!(
            f.iter()
                .any(|f| f.contains("curve of no length at point 2")),
            "{f:?}"
        );
    }

    #[test]
    fn boxes_must_match_the_geometry() {
        let mut s = square();
        s.bbox = box_bytes(5.0, 5.0, 12.0, 15.0);
        let f = findings(&[s]);
        assert!(
            f.iter().any(|f| f.contains("does not hold the nodes")),
            "{f:?}"
        );
        let mut s = square();
        s.bbox = box_bytes(0.0, 5.0, 15.0, 15.0);
        let f = findings(&[s]);
        assert!(
            f.iter().any(|f| f.contains("reaches beyond the points")),
            "{f:?}"
        );
        let mut s = square();
        s.obbx = [
            box_bytes(5.0, 5.0, 15.0, 15.0),
            box_bytes(3.0, 5.0, 15.0, 15.0),
        ]
        .concat();
        let f = findings(&[s]);
        assert!(f.iter().any(|f| f.contains("obbx corner")), "{f:?}");
        // An ellipse fills its box; a transform moves it.
        let mut e = square();
        e.kind = OBJ_ELLIPSE;
        e.args[0] = (ARG_COORDS, i32s(&[units(10.0), units(10.0), 0, 0, 0]));
        assert_eq!(findings(&[e]), Vec::<String>::new());
    }

    #[test]
    fn ids_must_be_unique_and_references_resolve() {
        let mut a = square();
        let mut b = square();
        b.usdn = 1;
        b.at = (30.0, 30.0);
        b.bbox = box_bytes(30.0, 30.0, 40.0, 40.0);
        b.obbx = [b.bbox.clone(), box_bytes(29.5, 29.5, 40.5, 40.5)].concat();
        let f = findings(&[square(), b]);
        assert!(
            f.iter().any(|f| f.contains("usdn 1 is also obj 1's")),
            "{f:?}"
        );
        a.args[1] = (ARG_FILL, u32s(&[9]));
        a.args[2] = (ARG_OUTLINE, u32s(&[8]));
        a.args[3] = (ARG_STYLE, u32s(&[6]));
        let f = findings(&[a]);
        assert!(
            f.iter()
                .any(|f| f.contains("fill 9 is not in the fill table")),
            "{f:?}"
        );
        assert!(
            f.iter()
                .any(|f| f.contains("outline 8 is not in the outline table")),
            "{f:?}"
        );
        assert!(
            f.iter()
                .any(|f| f.contains("style 6 is not in the style table")),
            "{f:?}"
        );
    }

    /// A bitmap object whose crop outline's node bytes are `types`, as
    /// version 13 files hold it: the box from the origin twice, 2 and 24
    /// bits, the pixel size, image number 1, image id 1, 0, 0, -1, eight
    /// zero bytes, then the outline.
    fn bitmap(types: [u8; 5]) -> Obj {
        let mut c = box_bytes(0.0, 0.0, 10.0, 10.0);
        c.extend(box_bytes(0.0, 0.0, 10.0, 10.0));
        c.extend(u16s(&[2, 24]));
        c.extend(u32s(&[4, 4, 1, 1, 0, 0, u32::MAX, 0, 0]));
        let corners = [
            (0.0, 10.0),
            (10.0, 10.0),
            (10.0, 0.0),
            (0.0, 0.0),
            (0.0, 10.0),
        ];
        let pts: Vec<(f64, f64, u8)> = corners
            .iter()
            .zip(types)
            .map(|(&(x, y), t)| (x, y, t))
            .collect();
        c.extend(curve(&pts));
        let b = box_bytes(5.0, 5.0, 15.0, 15.0);
        let mut obbx = b.clone();
        obbx.extend(box_bytes(5.0, 5.0, 15.0, 15.0));
        Obj {
            kind: crate::parse::OBJ_BITMAP,
            args: vec![
                (ARG_COORDS, c),
                (ARG_FILL, u32s(&[0])),
                (ARG_OUTLINE, u32s(&[0])),
                (ARG_STYLE, u32s(&[77])),
            ],
            bbox: b,
            obbx,
            usdn: 1,
            at: (5.0, 5.0),
            txsm: None,
        }
    }

    /// A bitmap outline whose nodes lack the marks curves carry (as
    /// TraceDraw wrote them before, 0x00, 0x40, 0x40, 0x40, 0x48) is
    /// reported; the marked one the writer now writes is not (#1226).
    #[test]
    fn bitmap_outlines_need_node_marks() {
        let lacks = |types: [u8; 5]| {
            check_bytes(&file(&[bitmap(types)]))
                .expect("checks")
                .iter()
                .any(|f| f.what.contains("lacks the node marks"))
        };
        assert!(lacks([0x00, 0x40, 0x40, 0x40, 0x48]));
        assert!(!lacks([0x0c, 0x44, 0x44, 0x44, 0x48]));
    }

    #[test]
    fn text_records_must_hold_letters() {
        let mut t = text();
        let styles = [Style7 {
            chars: 0,
            font: None,
            flags: None,
            size_units: None,
            fill: None,
        }];
        t.txsm = Some(txsm7(V, false, &styles, ""));
        let f = findings(&[t]);
        assert!(
            f.iter()
                .any(|f| f.contains("no letters") || f.contains("no runs")),
            "{f:?}"
        );
        let mut t = text();
        t.txsm = None;
        let f = findings(&[t]);
        assert!(
            f.iter().any(|f| f.contains("without a txsm record")),
            "{f:?}"
        );
    }

    #[test]
    fn versions_before_7_are_not_checked() {
        let body = chunk(b"vrsn", &u16s(&[500]));
        assert!(check_bytes(&riff(b"CDR5", &body)).is_err());
    }
}
