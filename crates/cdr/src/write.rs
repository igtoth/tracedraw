//! `.cdr` writer: the version 12 RIFF layout (form type `CDRC`, 32-bit
//! fields, coordinates in 1/254000 inch with the origin at the page
//! centre). The layout is the one `parse.rs` reads and `docs/cdr-format.md`
//! records; files written here open in this crate and in other readers of
//! the public layout.
//!
//! What is written: pages, layers with names, rectangles, ellipses (with
//! arcs), curves for every other outline (polygons, paths, tables,
//! symbols, expanded effects), groups, artistic text with one font, size
//! and style per run, bitmaps (24-bit), solid and fountain fills,
//! outlines with width, caps, joins, dashes, nib and the "scale with
//! object" and "behind fill" flags, and uniform transparency. ClipFrames
//! are written as their contents followed by the frame outline.

use crate::parse::{
    ARG_COORDS, ARG_FILL, ARG_NAME, ARG_OPACITY, ARG_OUTLINE, ARG_STYLE, OBJ_ARTISTIC_TEXT,
    OBJ_BITMAP, OBJ_CURVE, OBJ_ELLIPSE, OBJ_RECT,
};
use std::collections::HashMap;
use tracedraw_core::{
    document::{Shape, ShapeKind, TextSpan},
    geometry::{Affine, BezPath, PathEl, Point},
    style::{FountainKind, LineCap, LineJoin},
    Color, Document, Fill, Stroke,
};

/// Coordinate units per millimetre.
const UNITS_PER_MM: f64 = 254000.0 / 25.4;
const VERSION: u16 = 12;

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

fn units(mm: f64) -> i32 {
    (mm * UNITS_PER_MM)
        .round()
        .clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

fn utf16z(s: &str) -> Vec<u8> {
    let mut v: Vec<u8> = s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    v.extend_from_slice(&[0, 0]);
    v
}

/// 12-byte colour record: model, palette, 4 unknown bytes, 4 value bytes.
/// CMYK keeps its percentages (model 2); everything else is written as
/// sRGB bytes (model 5, stored B G R).
fn color_record(c: Color) -> Vec<u8> {
    let mut v = Vec::with_capacity(12);
    match c {
        Color::Cmyk { c, m, y, k } => {
            v.extend_from_slice(&u16s(&[2, 0]));
            v.extend_from_slice(&[0; 4]);
            let pct = |x: f32| (x.clamp(0.0, 1.0) * 100.0).round() as u8;
            v.extend_from_slice(&[pct(c), pct(m), pct(y), pct(k)]);
        }
        other => {
            let [r, g, b] = other.to_rgb8();
            v.extend_from_slice(&u16s(&[5, 0]));
            v.extend_from_slice(&[0; 4]);
            v.extend_from_slice(&[b, g, r, 0]);
        }
    }
    v
}

/// `loda`: header, argument bodies (4-byte aligned), offsets table with
/// the end marker, types table in reverse order.
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
    out
}

/// `trfd` with one affine record (type 8): a c e b d f, translation in
/// coordinate units.
fn trfd(m: Affine) -> Vec<u8> {
    let c = m.as_coeffs();
    let mut rec = Vec::new();
    rec.extend_from_slice(&8u16.to_le_bytes());
    rec.extend_from_slice(&[0; 6]);
    rec.extend_from_slice(&f64s(&[
        c[0],
        c[2],
        c[4] * UNITS_PER_MM,
        c[1],
        c[3],
        c[5] * UNITS_PER_MM,
    ]));
    let mut out = u32s(&[0, 1, 12, 16]);
    out.extend_from_slice(&rec);
    out
}

/// `mcfg` for versions 9 to 12: 4 unknown bytes, width, height, then
/// padding for the fields readers may expect after them.
fn mcfg(w_mm: f64, h_mm: f64) -> Vec<u8> {
    let mut out = vec![0u8; 4];
    out.extend_from_slice(&i32s(&[units(w_mm), units(h_mm)]));
    out.resize(0x30, 0);
    out
}

/// Pre-X3 `fild`: id, type, then the body.
fn fild_solid(id: u32, c: Color) -> Vec<u8> {
    let mut f = u32s(&[id]);
    f.extend_from_slice(&u16s(&[1, 0]));
    f.extend_from_slice(&color_record(c));
    f
}

fn fild_none(id: u32) -> Vec<u8> {
    let mut f = u32s(&[id]);
    f.extend_from_slice(&u16s(&[0]));
    f
}

/// Pre-X3 fountain body (version 6 to 12 layout).
fn fild_fountain(id: u32, f: &tracedraw_core::Fountain) -> Vec<u8> {
    let mut b = u32s(&[id]);
    b.extend_from_slice(&u16s(&[2]));
    b.extend_from_slice(&[0; 2]);
    b.push(match f.kind {
        FountainKind::Linear => 1,
        FountainKind::Radial => 2,
        FountainKind::Conical => 3,
        FountainKind::Square => 4,
    });
    b.extend_from_slice(&[0; 19]);
    b.extend_from_slice(&i32s(&[(f.edge_pad * 100.0).round() as i32])); // edge
    b.extend_from_slice(&i32s(&[(f.angle * 1_000_000.0).round() as i32]));
    b.extend_from_slice(&i32s(&[
        (f.offset.x * 100.0).round() as i32,
        (f.offset.y * 100.0).round() as i32,
    ]));
    b.extend_from_slice(&[0; 2]);
    b.extend_from_slice(&u32s(&[0])); // mode
    b.push(50); // mid point
    b.push(0);
    let stops: Vec<_> = f.stops.iter().take(64).collect();
    b.extend_from_slice(&u32s(&[stops.len() as u32]));
    for s in stops {
        b.extend_from_slice(&color_record(s.color));
        b.extend_from_slice(&u32s(&[(s.pos.clamp(0.0, 1.0) * 100.0).round() as u32]));
    }
    b
}

/// Version 9 to 12 `outl` layout.
fn outl(id: u32, s: Option<&Stroke>) -> Vec<u8> {
    let mut o = u32s(&[id]);
    let Some(s) = s else {
        // Line type bit 0: no outline; the rest is a plain hairline record.
        o.extend_from_slice(&u16s(&[1, 0, 0]));
        o.extend_from_slice(&[0; 2]);
        o.extend_from_slice(&i32s(&[0]));
        o.extend_from_slice(&u16s(&[100]));
        o.extend_from_slice(&[0; 2]);
        o.extend_from_slice(&i32s(&[0]));
        o.extend_from_slice(&[0; 52]);
        o.extend_from_slice(&color_record(Color::BLACK));
        o.extend_from_slice(&[0; 16]);
        o.extend_from_slice(&u16s(&[0]));
        o.extend_from_slice(&[0; 22]);
        o.extend_from_slice(&u32s(&[0, 0]));
        return o;
    };
    let mut line_type = 0u16;
    if s.scale_with_object {
        line_type |= 0x20;
    }
    if s.behind_fill {
        line_type |= 0x10;
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
    o.extend_from_slice(&u16s(&[line_type, caps, join]));
    o.extend_from_slice(&[0; 2]);
    let width = if s.width <= Stroke::HAIRLINE + 1e-9 {
        0
    } else {
        units(s.width)
    };
    o.extend_from_slice(&i32s(&[width]));
    o.extend_from_slice(&u16s(
        &[(s.stretch.clamp(0.01, 1.0) * 100.0).round() as u16],
    ));
    o.extend_from_slice(&[0; 2]);
    o.extend_from_slice(&i32s(&[(s.nib_angle * 1_000_000.0).round() as i32]));
    o.extend_from_slice(&[0; 52]);
    o.extend_from_slice(&color_record(s.color));
    o.extend_from_slice(&[0; 16]);
    let dashes: Vec<u16> = s
        .dash
        .iter()
        .take(10)
        .map(|d| d.round().clamp(0.0, 65535.0) as u16)
        .collect();
    o.extend_from_slice(&u16s(&[dashes.len() as u16]));
    let dashes_at = o.len();
    o.extend_from_slice(&u16s(&dashes));
    o.resize(dashes_at + 22, 0);
    // Marker ids: preset arrowheads are not written.
    o.extend_from_slice(&u32s(&[0, 0]));
    o
}

/// A minimal `stlt` (version 7 to 12 layout): one font entry, one
/// alignment, one interval and one full style record (id 0) that text
/// objects reference by default. Readers that resolve text through the
/// style table find a complete style here.
fn stlt(font_id: u16) -> Vec<u8> {
    let mut t = u32s(&[1]); // records
    t.extend_from_slice(&u32s(&[0])); // fills
    t.extend_from_slice(&u32s(&[0])); // outlines
                                      // Fonts: id, 20 unknown, font id, encoding, 8, size, 8, flags, 8.
    t.extend_from_slice(&u32s(&[1, 1]));
    t.extend_from_slice(&[0; 20]);
    t.extend_from_slice(&u16s(&[font_id, 0]));
    t.extend_from_slice(&[0; 8]);
    t.extend_from_slice(&i32s(&[(12.0 * 254000.0 / 72.0) as i32]));
    t.extend_from_slice(&[0; 8]);
    t.extend_from_slice(&u32s(&[0]));
    t.extend_from_slice(&[0; 8]);
    // Alignments: id, 4 unknown, value (1 = left).
    t.extend_from_slice(&u32s(&[1, 1, 0, 1]));
    // Intervals: id, 8, character spacing, 8, line spacing, 24.
    t.extend_from_slice(&u32s(&[1, 1]));
    t.extend_from_slice(&[0; 8]);
    t.extend_from_slice(&u32s(&[0]));
    t.extend_from_slice(&[0; 8]);
    t.extend_from_slice(&u32s(&[1_000_000]));
    t.extend_from_slice(&[0; 24]);
    // set5, tabs, bullets, indents, hyphens, drop caps, set11: empty.
    for _ in 0..7 {
        t.extend_from_slice(&u32s(&[0]));
    }
    // The record: 3 sections, style id 0, no parent, 8 unknown, name.
    let name: Vec<u16> = "Default Artistic Text".encode_utf16().collect();
    t.extend_from_slice(&u32s(&[3, 0, 0]));
    t.extend_from_slice(&[0; 8]);
    t.extend_from_slice(&u32s(&[name.len() as u32]));
    for u in &name {
        t.extend_from_slice(&u.to_le_bytes());
    }
    t.extend_from_slice(&u32s(&[0, 0])); // fill, outline refs
    t.extend_from_slice(&u32s(&[1, 1, 1, 0])); // font, align, interval, set5
    t.extend_from_slice(&u32s(&[0])); // set11
    t.extend_from_slice(&u32s(&[0, 0, 0, 0, 0])); // tab, bullet, indent, hyphen, drop cap
    t
}

fn font_chunk(id: u16, name: &str) -> Vec<u8> {
    let mut f = u16s(&[id, 0]);
    f.extend_from_slice(&u32s(&[0]));
    f.extend_from_slice(&[0; 10]);
    f.extend_from_slice(&utf16z(name));
    f
}

/// Transparency argument: 10 unknown bytes, then thousandths.
fn opacity_arg(opacity: f64) -> Vec<u8> {
    let mut v = vec![0u8; 10];
    let t = ((1.0 - opacity.clamp(0.0, 1.0)) * 1000.0).round() as u16;
    v.extend_from_slice(&u16s(&[t]));
    v
}

/// Points and type bytes of a path in local units.
fn curve_coords(path: &BezPath) -> Option<Vec<u8>> {
    let mut pts: Vec<(i32, i32)> = Vec::new();
    let mut types: Vec<u8> = Vec::new();
    let mut last_start = 0usize;
    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => {
                pts.push((units(p.x), units(p.y)));
                types.push(0x00);
                last_start = pts.len() - 1;
            }
            PathEl::LineTo(p) => {
                pts.push((units(p.x), units(p.y)));
                types.push(0x40);
            }
            PathEl::QuadTo(c, p) => {
                // Elevate to a cubic.
                let s = pts.last().copied().unwrap_or((0, 0));
                let sx = s.0 as f64 / UNITS_PER_MM;
                let sy = s.1 as f64 / UNITS_PER_MM;
                let c1 = Point::new(sx + (c.x - sx) * 2.0 / 3.0, sy + (c.y - sy) * 2.0 / 3.0);
                let c2 = Point::new(p.x + (c.x - p.x) * 2.0 / 3.0, p.y + (c.y - p.y) * 2.0 / 3.0);
                pts.push((units(c1.x), units(c1.y)));
                types.push(0xc0);
                pts.push((units(c2.x), units(c2.y)));
                types.push(0xc0);
                pts.push((units(p.x), units(p.y)));
                types.push(0x80);
            }
            PathEl::CurveTo(a, b, p) => {
                pts.push((units(a.x), units(a.y)));
                types.push(0xc0);
                pts.push((units(b.x), units(b.y)));
                types.push(0xc0);
                pts.push((units(p.x), units(p.y)));
                types.push(0x80);
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
    let mut out = u32s(&[pts.len() as u32]);
    for (x, y) in &pts {
        out.extend_from_slice(&i32s(&[*x, *y]));
    }
    out.extend_from_slice(&types);
    Some(out)
}

struct Writer<'a> {
    fills: Vec<Vec<u8>>,
    fill_ids: HashMap<String, u32>,
    outlines: Vec<Vec<u8>>,
    outline_ids: HashMap<String, u32>,
    fonts: Vec<Vec<u8>>,
    font_ids: HashMap<String, u16>,
    bitmaps: Vec<Vec<u8>>,
    symbols: &'a [tracedraw_core::document::Symbol],
    next_fill: u32,
    next_outline: u32,
    next_font: u16,
    next_bitmap: u32,
}

impl Writer<'_> {
    fn fill_id(&mut self, fill: &Fill) -> u32 {
        let key = format!("{fill:?}");
        if let Some(id) = self.fill_ids.get(&key) {
            return *id;
        }
        let id = self.next_fill;
        self.next_fill += 1;
        let bytes = match fill {
            Fill::None => fild_none(id),
            Fill::Solid(c) => fild_solid(id, *c),
            Fill::Fountain(f) => fild_fountain(id, f),
            other => fild_solid(id, other.preview_color().unwrap_or(Color::Gray { v: 0.5 })),
        };
        self.fills.push(bytes);
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
        self.outlines.push(outl(id, stroke));
        self.outline_ids.insert(key, id);
        id
    }

    fn font_id(&mut self, family: &str) -> u16 {
        if let Some(id) = self.font_ids.get(family) {
            return *id;
        }
        let id = self.next_font;
        self.next_font += 1;
        self.fonts.push(font_chunk(id, family));
        self.font_ids.insert(family.to_string(), id);
        id
    }

    /// A `bmp ` chunk (24-bit BGR, bottom-up rows) for PNG bytes; the image id.
    fn bitmap_id(&mut self, png: &[u8]) -> Option<u32> {
        let pm = tiny_skia::Pixmap::decode_png(png).ok()?;
        let (w, h) = (pm.width(), pm.height());
        if w == 0 || h == 0 {
            return None;
        }
        let id = self.next_bitmap;
        self.next_bitmap += 1;
        let stride = (w as usize * 24).div_ceil(32) * 4;
        let mut pixels = vec![0u8; stride * h as usize];
        for row in 0..h as usize {
            let dst = &mut pixels[(h as usize - 1 - row) * stride..][..stride];
            for x in 0..w as usize {
                let p = pm.pixels()[row * w as usize + x].demultiply();
                // Transparent pixels over white.
                let a = p.alpha() as u32;
                let blend = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
                dst[x * 3] = blend(p.blue());
                dst[x * 3 + 1] = blend(p.green());
                dst[x * 3 + 2] = blend(p.red());
            }
        }
        let mut b = u32s(&[id]);
        b.extend_from_slice(&[0; 50]);
        // Colour model 1 is 24-bit BGR (confirmed on a 2019 file; model 5
        // is greyscale in other readers).
        b.extend_from_slice(&u32s(&[1]));
        b.extend_from_slice(&[0; 4]);
        b.extend_from_slice(&u32s(&[w, h]));
        b.extend_from_slice(&[0; 4]);
        b.extend_from_slice(&u32s(&[24]));
        b.extend_from_slice(&[0; 4]);
        b.extend_from_slice(&u32s(&[pixels.len() as u32]));
        b.extend_from_slice(&[0; 32]);
        b.extend_from_slice(&pixels);
        self.bitmaps.push(b);
        Some(id)
    }

    /// Objects of a layer, front to back as the file stores them.
    fn objects(&mut self, shapes: &[Shape], center: Point, depth: usize) -> Vec<u8> {
        let mut out = Vec::new();
        if depth > 60 {
            return out;
        }
        for s in shapes.iter().rev() {
            out.extend(self.object(s, center, depth));
        }
        out
    }

    /// A bitmap object: image rectangle, image id and crop path (the whole
    /// rectangle, or `crop` in the bitmap's local space).
    fn bitmap_object(&mut self, s: &Shape, center: Point, crop: Option<&BezPath>) -> Vec<u8> {
        let ShapeKind::Bitmap { rect, png, .. } = &s.kind else {
            return Vec::new();
        };
        let Some(image) = self.bitmap_id(png) else {
            return Vec::new();
        };
        let transform = Affine::translate(-center.to_vec2()) * s.transform;
        let mut c = i32s(&[
            units(rect.x0),
            units(rect.y0),
            units(rect.x1),
            units(rect.y1),
        ]);
        c.extend_from_slice(&[0; 32]);
        c.extend_from_slice(&u32s(&[image]));
        c.extend_from_slice(&[0; 20]);
        let path_bytes = match crop {
            Some(p) => curve_coords(p),
            None => {
                let mut full = BezPath::new();
                full.move_to((rect.x0, rect.y0));
                full.line_to((rect.x1, rect.y0));
                full.line_to((rect.x1, rect.y1));
                full.line_to((rect.x0, rect.y1));
                full.close_path();
                curve_coords(&full)
            }
        };
        c.extend_from_slice(&path_bytes.unwrap_or_else(|| u32s(&[0])));
        let fill = self.fill_id(&Fill::None);
        let outline = self.outline_id(s.stroke.as_ref());
        let mut args: Vec<(u32, Vec<u8>)> = vec![(ARG_COORDS, c)];
        args.push((ARG_FILL, u32s(&[fill])));
        args.push((ARG_OUTLINE, u32s(&[outline])));
        if let Some(n) = &s.name {
            args.push((ARG_NAME, utf16z(n)));
        }
        if s.opacity < 0.999 {
            args.push((ARG_OPACITY, opacity_arg(s.opacity)));
        }
        let mut lgob = chunk(b"loda", &loda(OBJ_BITMAP, &args));
        lgob.extend(list(b"trfl", &chunk(b"trfd", &trfd(transform))));
        let mut body = chunk(b"flgs", &[0, 0, 0, 0]);
        body.extend(list(b"lgob", &lgob));
        list(b"obj ", &body)
    }

    fn object(&mut self, s: &Shape, center: Point, depth: usize) -> Vec<u8> {
        if !s.visible {
            return Vec::new();
        }
        if !s.effects.is_empty() {
            let ev = tracedraw_core::live::evaluate(s);
            let mut all: Vec<Shape> = ev.below.clone();
            all.push(ev.main.clone());
            all.extend(ev.above.iter().cloned());
            for x in all.iter_mut() {
                x.effects.clear();
            }
            return self.objects(&all, center, depth + 1);
        }
        // The page origin is the page centre in the file.
        let recenter = Affine::translate(-center.to_vec2());
        let transform = recenter * s.transform;
        match &s.kind {
            ShapeKind::Group { children } => {
                let mut body = Vec::new();
                for c in children.iter().rev() {
                    let mut c = c.clone();
                    c.absorb(s.transform);
                    body.extend(self.object(&c, center, depth + 1));
                }
                return list(b"grp ", &body);
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
                return self.objects(&kids, center, depth + 1);
            }
            ShapeKind::ClipFrame { frame, contents }
                if contents.len() == 1
                    && matches!(contents[0].kind, ShapeKind::Bitmap { .. })
                    && contents[0].effects.is_empty() =>
            {
                // A clipped bitmap is a bitmap object with a crop path, the
                // way the target design stores it.
                let bm = &contents[0];
                let clip = (bm.transform.inverse()) * frame.page_path();
                let mut b = bm.clone();
                b.transform = s.transform * bm.transform;
                return self.bitmap_object(&b, center, Some(&clip));
            }
            ShapeKind::ClipFrame { frame, contents } => {
                let mut body = Vec::new();
                let mut f = (**frame).clone();
                f.absorb(s.transform);
                f.fill = Fill::None;
                body.extend(self.object(&f, center, depth + 1));
                for c in contents.iter().rev() {
                    let mut c = c.clone();
                    c.absorb(s.transform);
                    body.extend(self.object(&c, center, depth + 1));
                }
                return list(b"grp ", &body);
            }
            _ => {}
        }
        let fill = self.fill_id(&s.fill);
        let outline = self.outline_id(s.stroke.as_ref());
        let mut args: Vec<(u32, Vec<u8>)> = Vec::new();
        let mut txsm: Option<Vec<u8>> = None;
        let transform_out: Affine;
        // Rectangles one round radius describes stay rectangles; other
        // corners (styles, sizes per corner, fixed sizes on a stretched
        // rectangle) go out as the curve they draw.
        let plain = s.plain_rect();
        let kind = match &s.kind {
            ShapeKind::Rect { .. } if plain.is_some() => {
                let (rect, radius) = plain.unwrap_or_default();
                // Width and height from the rect's own corner: fold the
                // corner into the transform.
                let t = transform * Affine::translate((rect.x0, rect.y0));
                let r = units(radius);
                args.push((
                    ARG_COORDS,
                    i32s(&[units(rect.width()), units(rect.height()), r, r, r, r]),
                ));
                transform_out = t;
                OBJ_RECT
            }
            ShapeKind::Ellipse { rect, arc } => {
                let t = transform * Affine::translate((rect.x0, rect.y0));
                let (a1, a2, pie) = match arc {
                    Some(a) => (a.start_deg, a.end_deg, a.pie as u32),
                    None => (0.0, 0.0, 0),
                };
                let mut c = i32s(&[units(rect.width()), units(rect.height())]);
                c.extend_from_slice(&i32s(&[
                    (a1 * 1_000_000.0).round() as i32,
                    (a2 * 1_000_000.0).round() as i32,
                ]));
                c.extend_from_slice(&u32s(&[pie]));
                args.push((ARG_COORDS, c));
                transform_out = t;
                OBJ_ELLIPSE
            }
            ShapeKind::Text {
                spans,
                origin,
                align,
                ..
            } => {
                let t = transform * Affine::translate(origin.to_vec2());
                args.push((ARG_COORDS, i32s(&[0, 0])));
                args.push((ARG_STYLE, u32s(&[0])));
                transform_out = t;
                txsm = Some(chunk(b"txsm", &self.txsm(spans, *align, &s.fill)));
                OBJ_ARTISTIC_TEXT
            }
            ShapeKind::Bitmap { .. } => return self.bitmap_object(s, center, None),
            _ => {
                let Some(coords) = curve_coords(&s.local_path()) else {
                    return Vec::new();
                };
                args.push((ARG_COORDS, coords));
                transform_out = transform;
                OBJ_CURVE
            }
        };
        args.push((ARG_FILL, u32s(&[fill])));
        args.push((ARG_OUTLINE, u32s(&[outline])));
        if let Some(n) = &s.name {
            args.push((ARG_NAME, utf16z(n)));
        }
        if s.opacity < 0.999 {
            args.push((ARG_OPACITY, opacity_arg(s.opacity)));
        }
        // The object layout of real files: flags, then a `lgob` list with
        // the attributes and a `trfl` list holding the transform, then the
        // text chunk.
        let mut lgob = chunk(b"loda", &loda(kind, &args));
        lgob.extend(list(b"trfl", &chunk(b"trfd", &trfd(transform_out))));
        let mut body = chunk(b"flgs", &[0, 0, 0, 0]);
        body.extend(list(b"lgob", &lgob));
        if let Some(t) = txsm {
            body.extend(t);
        }
        list(b"obj ", &body)
    }

    /// `txsm` in the version 7 to X5 layout: one frame, one paragraph,
    /// one style per span, the text as UTF-16.
    fn txsm(
        &mut self,
        spans: &[TextSpan],
        align: tracedraw_core::TextAlign,
        fill: &Fill,
    ) -> Vec<u8> {
        let mut t = u32s(&[0]); // artistic (no frame)
        t.extend_from_slice(&[0; 32]);
        t.extend_from_slice(&u32s(&[1, 1])); // frames, frame id
        t.extend_from_slice(&[0; 48]);
        t.extend_from_slice(&u32s(&[0])); // not on a path
        t.extend_from_slice(&[0; 34]);
        t.extend_from_slice(&u32s(&[1, 0])); // paragraphs, style id
                                             // Alignment lives in the style tables, which are not written.
        let _ = align;
        t.push(0);
        let runs: Vec<&TextSpan> = spans.iter().filter(|s| !s.text.is_empty()).collect();
        t.extend_from_slice(&u32s(&[runs.len() as u32]));
        for sp in &runs {
            let font = self.font_id(&sp.font_family);
            let chars = sp.text.chars().count().min(65535) as u16;
            t.extend_from_slice(&u16s(&[chars]));
            let mut flags = 0x01 | 0x02 | 0x04;
            let span_fill = sp.fill.as_ref().filter(|f| f != &fill);
            if span_fill.is_some() {
                flags |= 0x40;
            }
            t.push(flags);
            t.push(0);
            t.extend_from_slice(&u16s(&[font, 0]));
            let mut style_flags = 0u32;
            if sp.bold {
                style_flags |= 0x1000;
            }
            if sp.italic {
                style_flags |= 0x2000;
            }
            if sp.underline {
                style_flags |= 0x0004;
            }
            t.extend_from_slice(&u32s(&[style_flags]));
            // Size in coordinate units of one point.
            t.extend_from_slice(&i32s(&[(sp.size_pt * 254000.0 / 72.0).round() as i32]));
            if let Some(f) = span_fill {
                let id = self.fill_id(f);
                t.extend_from_slice(&u32s(&[id]));
            }
        }
        let text: String = runs.iter().map(|s| s.text.as_str()).collect();
        let n = text.chars().count() as u32;
        t.extend_from_slice(&u32s(&[n]));
        t.extend_from_slice(&vec![0u8; n as usize * 8]);
        let bytes: Vec<u8> = text.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        t.extend_from_slice(&u32s(&[bytes.len() as u32]));
        t.extend_from_slice(&bytes);
        t.push(0);
        t
    }
}

/// Write the whole document as a version 12 `.cdr` stream.
pub fn document_to_cdr(doc: &Document) -> Vec<u8> {
    let mut w = Writer {
        fills: Vec::new(),
        fill_ids: HashMap::new(),
        outlines: Vec::new(),
        outline_ids: HashMap::new(),
        fonts: Vec::new(),
        font_ids: HashMap::new(),
        bitmaps: Vec::new(),
        symbols: &doc.symbols,
        next_fill: 1,
        next_outline: 1,
        next_font: 1,
        next_bitmap: 1,
    };
    let first = doc
        .pages
        .first()
        .map(|p| p.size)
        .unwrap_or(tracedraw_core::geometry::Size::new(210.0, 297.0));
    let mut pages = Vec::new();
    for page in &doc.pages {
        let center = Point::new(page.size.width / 2.0, page.size.height / 2.0);
        let mut layers = Vec::new();
        for layer in &page.layers {
            let mut body = Vec::new();
            // Layer flags: visible, drawable.
            body.extend(chunk(b"flgs", &[0x00, 0x00, 0x00, 0x98]));
            body.extend(chunk(b"loda", &loda(0, &[(ARG_NAME, utf16z(&layer.name))])));
            if layer.visible {
                body.extend(w.objects(&layer.shapes, center, 0));
            }
            layers.extend(list(b"layr", &body));
        }
        let mut pbody = chunk(b"flgs", &[0x00, 0x00, 0x00, 0x90]);
        pbody.extend(list(b"gobj", &layers));
        pages.extend(list(b"page", &pbody));
    }
    let mut body = chunk(b"vrsn", &u16s(&[VERSION * 100]));
    let mut docl = chunk(b"mcfg", &mcfg(first.width, first.height));
    if !w.fonts.is_empty() {
        let mut fnt = Vec::new();
        for f in &w.fonts {
            fnt.extend(chunk(b"font", f));
        }
        docl.extend(list(b"fntt", &fnt));
    }
    docl.extend(list(b"stlt", &stlt(if w.fonts.is_empty() { 0 } else { 1 })));
    let mut fills = Vec::new();
    for f in &w.fills {
        fills.extend(chunk(b"fild", f));
    }
    docl.extend(list(b"filt", &fills));
    let mut outlines = Vec::new();
    for o in &w.outlines {
        outlines.extend(chunk(b"outl", o));
    }
    docl.extend(list(b"otlt", &outlines));
    if !w.bitmaps.is_empty() {
        let mut bmps = Vec::new();
        for b in &w.bitmaps {
            bmps.extend(chunk(b"bmp ", b));
        }
        docl.extend(list(b"bmpt", &bmps));
    }
    docl.extend(pages);
    body.extend(list(b"doc ", &docl));
    let mut payload = b"CDRC".to_vec();
    payload.extend_from_slice(&body);
    chunk(b"RIFF", &payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{
        document::{Layer, Page, ParagraphStyle},
        geometry::{Rect, Shape as _, Size},
        TextAlign,
    };

    fn doc_with(shapes: Vec<Shape>) -> Document {
        let mut doc = Document::new("w", Size::new(200.0, 100.0));
        doc.pages[0].layers[0].name = "Ink".into();
        doc.pages[0].layers[0].shapes = shapes;
        doc
    }

    #[test]
    fn rectangle_ellipse_curve_and_text_round_trip() {
        let mut doc = Document::new("w", Size::new(200.0, 100.0));
        let mut ids = doc.ids().clone();
        let mut r = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(10.0, 20.0, 60.0, 50.0),
                radius: 2.0,
                corners: None,
            },
        );
        r.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        let mut st = Stroke::new(Color::rgb8(0, 0, 255), 1.5);
        st.cap = LineCap::Round;
        st.join = LineJoin::Bevel;
        st.dash = vec![3.0, 1.0];
        st.scale_with_object = true;
        r.stroke = Some(st.clone());
        r.name = Some("box".into());
        r.opacity = 0.5;
        let mut e = Shape::new(
            ids.shape(),
            ShapeKind::Ellipse {
                rect: Rect::new(100.0, 10.0, 140.0, 40.0),
                arc: None,
            },
        );
        e.fill = Fill::Fountain(tracedraw_core::Fountain::two(
            FountainKind::Radial,
            Color::BLACK,
            Color::WHITE,
            0.0,
        ));
        e.stroke = None;
        let mut p = BezPath::new();
        p.move_to((10.0, 60.0));
        p.curve_to((20.0, 90.0), (40.0, 90.0), (50.0, 60.0));
        p.line_to((30.0, 55.0));
        p.close_path();
        let mut c = Shape::new(
            ids.shape(),
            ShapeKind::Path {
                path: p,
                closed: true,
            },
        );
        c.fill = Fill::Solid(Color::cmyk_pct(0.0, 100.0, 100.0, 0.0));
        c.stroke = Some(Stroke::hairline(Color::BLACK));
        c.transform = Affine::translate((100.0, 0.0));
        let mut t = Shape::new(
            ids.shape(),
            ShapeKind::Text {
                spans: vec![{
                    let mut s = TextSpan::new("Hello", "Verdana", 24.0);
                    s.bold = true;
                    s
                }],
                origin: Point::new(20.0, 80.0),
                frame: None,
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            },
        );
        t.fill = Fill::Solid(Color::rgb8(0, 128, 0));
        t.stroke = None;
        doc.pages[0].layers[0].name = "Ink".into();
        doc.pages[0].layers[0].shapes = vec![r, e, c, t];
        doc.set_ids(ids);
        let bytes = document_to_cdr(&doc);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        assert_eq!(rep.version.map(|v| v.0), Some(12));
        assert_eq!(rep.skipped_objects, 0, "{:?}", rep.warnings);
        assert_eq!(back.pages.len(), 1);
        let page = &back.pages[0];
        assert!((page.size.width - 200.0).abs() < 0.01 && (page.size.height - 100.0).abs() < 0.01);
        assert_eq!(page.layers.len(), 1);
        assert_eq!(page.layers[0].name, "Ink");
        let shapes = &page.layers[0].shapes;
        assert_eq!(shapes.len(), 4, "{:?}", rep.warnings);
        // Order and geometry.
        let rb = shapes[0].bounds();
        assert!(
            (rb.x0 - 10.0).abs() < 0.01 && (rb.y1 - 50.0).abs() < 0.01,
            "{rb:?}"
        );
        assert!(matches!(shapes[0].kind, ShapeKind::Rect { .. }));
        if let ShapeKind::Rect { radius, .. } = shapes[0].kind {
            assert!((radius - 2.0).abs() < 0.01);
        }
        assert_eq!(shapes[0].fill, Fill::Solid(Color::rgb8(255, 0, 0)));
        let s0 = shapes[0].stroke.as_ref().expect("stroke");
        assert_eq!(s0.color, Color::rgb8(0, 0, 255));
        assert!((s0.width - 1.5).abs() < 0.01);
        assert_eq!(s0.cap, LineCap::Round);
        assert_eq!(s0.join, LineJoin::Bevel);
        assert_eq!(s0.dash, vec![3.0, 1.0]);
        assert!(s0.scale_with_object);
        assert_eq!(shapes[0].name.as_deref(), Some("box"));
        assert!((shapes[0].opacity - 0.5).abs() < 0.01);
        let eb = shapes[1].bounds();
        assert!(
            (eb.x0 - 100.0).abs() < 0.01 && (eb.y0 - 10.0).abs() < 0.01,
            "{eb:?}"
        );
        assert!(matches!(shapes[1].fill, Fill::Fountain(_)));
        if let Fill::Fountain(f) = &shapes[1].fill {
            assert_eq!(f.kind, FountainKind::Radial);
            assert_eq!(f.stops.len(), 2);
        }
        assert!(shapes[1].stroke.is_none());
        let cb = shapes[2].bounds();
        assert!(
            (cb.x0 - 110.0).abs() < 0.05 && (cb.x1 - 150.0).abs() < 0.05,
            "{cb:?}"
        );
        assert!(matches!(
            shapes[2].kind,
            ShapeKind::Path { closed: true, .. }
        ));
        assert_eq!(
            shapes[2].fill,
            Fill::Solid(Color::cmyk_pct(0.0, 100.0, 100.0, 0.0))
        );
        assert!(
            (shapes[2].stroke.as_ref().expect("hairline").width - Stroke::HAIRLINE).abs() < 1e-9
        );
        match &shapes[3].kind {
            ShapeKind::Text { spans, .. } => {
                assert_eq!(spans.len(), 1);
                assert_eq!(spans[0].text, "Hello");
                assert_eq!(spans[0].font_family, "Verdana");
                assert!((spans[0].size_pt - 24.0).abs() < 0.01);
                assert!(spans[0].bold);
            }
            k => panic!("{k:?}"),
        }
        let o = shapes[3].transform * Point::ZERO;
        let origin = match &shapes[3].kind {
            ShapeKind::Text { origin, .. } => *origin,
            _ => unreachable!(),
        };
        let at = shapes[3].transform * origin;
        assert!(
            (at.x - 20.0).abs() < 0.05 && (at.y - 80.0).abs() < 0.05,
            "{o:?} {at:?}"
        );
        assert_eq!(shapes[3].fill, Fill::Solid(Color::rgb8(0, 128, 0)));
    }

    #[test]
    fn groups_bitmaps_and_several_pages_round_trip() {
        let mut doc = doc_with(Vec::new());
        let mut ids = doc.ids().clone();
        let mut a = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        a.fill = Fill::Solid(Color::BLACK);
        let mut b = a.clone();
        b.id = ids.shape();
        b.transform = Affine::translate((20.0, 0.0));
        let mut g = Shape::new(
            ids.shape(),
            ShapeKind::Group {
                children: vec![a, b],
            },
        );
        g.transform = Affine::translate((5.0, 5.0));
        let png = {
            let mut pm = tiny_skia::Pixmap::new(2, 2).expect("pixmap");
            let px = pm.pixels_mut();
            px[0] = tiny_skia::ColorU8::from_rgba(255, 0, 0, 255).premultiply();
            px[1] = tiny_skia::ColorU8::from_rgba(0, 255, 0, 255).premultiply();
            px[2] = tiny_skia::ColorU8::from_rgba(0, 0, 255, 255).premultiply();
            px[3] = tiny_skia::ColorU8::from_rgba(255, 255, 255, 255).premultiply();
            pm.encode_png().expect("png")
        };
        let mut bm = Shape::new(
            ids.shape(),
            ShapeKind::Bitmap {
                rect: Rect::new(100.0, 50.0, 140.0, 90.0),
                width_px: 2,
                height_px: 2,
                png,
                fx: None,
            },
        );
        bm.fill = Fill::None;
        bm.stroke = None;
        doc.pages[0].layers[0].shapes = vec![g, bm];
        let pid = ids.page();
        let lid = ids.layer();
        let mut second = Page {
            id: pid,
            name: "Back".into(),
            size: Size::new(200.0, 100.0),
            layers: vec![Layer::new(lid, "Second")],
            guides: Vec::new(),
            background: None,
        };
        let mut c = Shape::new(
            ids.shape(),
            ShapeKind::Ellipse {
                rect: Rect::new(10.0, 10.0, 30.0, 30.0),
                arc: None,
            },
        );
        c.fill = Fill::Solid(Color::rgb8(1, 2, 3));
        second.layers[0].shapes.push(c);
        doc.pages.push(second);
        doc.set_ids(ids);
        let bytes = document_to_cdr(&doc);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        assert_eq!(back.pages.len(), 2, "{:?}", rep.warnings);
        assert_eq!(rep.skipped_objects, 0, "{:?}", rep.warnings);
        let l = &back.pages[0].layers[0];
        assert_eq!(l.shapes.len(), 2);
        match &l.shapes[0].kind {
            ShapeKind::Group { children } => {
                assert_eq!(children.len(), 2);
                let gb = l.shapes[0].bounds();
                assert!(
                    (gb.x0 - 5.0).abs() < 0.01 && (gb.x1 - 35.0).abs() < 0.01,
                    "{gb:?}"
                );
            }
            k => panic!("{k:?}"),
        }
        match &l.shapes[1].kind {
            ShapeKind::Bitmap {
                width_px,
                height_px,
                png,
                ..
            } => {
                assert_eq!((*width_px, *height_px), (2, 2));
                let pm = tiny_skia::Pixmap::decode_png(png).expect("png");
                let p = pm.pixels();
                assert_eq!((p[0].red(), p[0].green(), p[0].blue()), (255, 0, 0));
                assert_eq!((p[3].red(), p[3].green(), p[3].blue()), (255, 255, 255));
            }
            k => panic!("{k:?}"),
        }
        let bb = l.shapes[1].bounds();
        assert!(
            (bb.x0 - 100.0).abs() < 0.01 && (bb.y1 - 90.0).abs() < 0.01,
            "{bb:?}"
        );
        assert_eq!(back.pages[1].layers[0].name, "Second");
        assert_eq!(back.pages[1].layers[0].shapes.len(), 1);
    }

    #[test]
    fn a_clipped_bitmap_round_trips_as_a_crop_path() {
        let mut doc = doc_with(Vec::new());
        let mut ids = doc.ids().clone();
        let png = {
            let mut pm = tiny_skia::Pixmap::new(4, 4).expect("pixmap");
            pm.fill(tiny_skia::Color::from_rgba8(10, 20, 30, 255));
            pm.encode_png().expect("png")
        };
        let mut bm = Shape::new(
            ids.shape(),
            ShapeKind::Bitmap {
                rect: Rect::new(0.0, 0.0, 100.0, 50.0),
                width_px: 4,
                height_px: 4,
                png,
                fx: None,
            },
        );
        bm.fill = Fill::None;
        bm.stroke = None;
        bm.transform = Affine::translate((20.0, 20.0));
        let mut frame = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(40.0, 30.0, 80.0, 60.0),
                radius: 0.0,
                corners: None,
            },
        );
        frame.fill = Fill::None;
        frame.stroke = None;
        let mut pc = Shape::new(
            ids.shape(),
            ShapeKind::ClipFrame {
                frame: Box::new(frame),
                contents: vec![bm],
            },
        );
        pc.fill = Fill::None;
        pc.stroke = None;
        doc.pages[0].layers[0].shapes = vec![pc];
        doc.set_ids(ids);
        let bytes = document_to_cdr(&doc);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        let l = &back.pages[0].layers[0];
        assert_eq!(l.shapes.len(), 1, "{:?}", rep.warnings);
        match &l.shapes[0].kind {
            ShapeKind::ClipFrame { frame, contents } => {
                // Frame and contents live in the ClipFrame's own space.
                let outer = l.shapes[0].transform;
                let fb = (outer * frame.page_path()).bounding_box();
                assert!(
                    (fb.x0 - 40.0).abs() < 0.05 && (fb.x1 - 80.0).abs() < 0.05,
                    "{fb:?}"
                );
                assert!(
                    (fb.y0 - 30.0).abs() < 0.05 && (fb.y1 - 60.0).abs() < 0.05,
                    "{fb:?}"
                );
                assert_eq!(contents.len(), 1);
                let cb = (outer * contents[0].page_path()).bounding_box();
                assert!(
                    (cb.x0 - 20.0).abs() < 0.05 && (cb.x1 - 120.0).abs() < 0.05,
                    "{cb:?}"
                );
            }
            k => panic!("{k:?}"),
        }
    }

    #[test]
    fn written_files_survive_mutation() {
        let mut doc = doc_with(Vec::new());
        let mut ids = doc.ids().clone();
        let mut r = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        r.fill = Fill::Solid(Color::BLACK);
        doc.pages[0].layers[0].shapes = vec![r];
        doc.set_ids(ids);
        let bytes = document_to_cdr(&doc);
        for cut in (0..bytes.len()).step_by(13) {
            let _ = crate::open_bytes(&bytes[..cut], "cut");
        }
        for i in (0..bytes.len()).step_by(7) {
            let mut m = bytes.clone();
            m[i] ^= 0x5a;
            let _ = crate::open_bytes(&m, "flip");
        }
    }

    #[test]
    fn corners_one_radius_cannot_hold_are_written_as_curves() {
        use tracedraw_core::{CornerKind, Corners};
        let mut doc = doc_with(Vec::new());
        let mut ids = doc.ids().clone();
        let rect = Rect::new(0.0, 0.0, 40.0, 20.0);
        let mut per_corner = Corners::uniform(2.0, CornerKind::Round);
        per_corner.radii[Corners::TOP_LEFT] = 8.0;
        let mut fixed = Corners::uniform(3.0, CornerKind::Round);
        fixed.fixed = true;
        let mut shapes = Vec::new();
        for (corners, scale) in [
            (Corners::uniform(4.0, CornerKind::Scallop), (1.0, 1.0)),
            (per_corner, (1.0, 1.0)),
            (fixed, (2.0, 1.0)),
            (fixed, (2.0, 2.0)),
        ] {
            let mut s = Shape::new(ids.shape(), ShapeKind::rect_with_corners(rect, corners));
            s.transform =
                Affine::translate((20.0, 20.0)) * Affine::scale_non_uniform(scale.0, scale.1);
            s.fill = Fill::Solid(Color::BLACK);
            shapes.push(s);
        }
        doc.pages[0].layers[0].shapes = shapes.clone();
        doc.set_ids(ids);
        let (back, rep) = crate::open_bytes(&document_to_cdr(&doc), "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 4, "{:?}", rep.warnings);
        for (i, (a, b)) in shapes.iter().zip(read).enumerate() {
            let (aa, ab) = (a.page_path().area().abs(), b.page_path().area().abs());
            assert!((aa - ab).abs() < 0.5, "{i}: {aa} vs {ab}");
            let (ba, bb) = (a.bounds(), b.bounds());
            assert!(
                (ba.x0 - bb.x0).abs() < 0.01 && (ba.y1 - bb.y1).abs() < 0.01,
                "{i}"
            );
        }
        assert!(matches!(read[0].kind, ShapeKind::Path { closed: true, .. }));
        assert!(matches!(read[1].kind, ShapeKind::Path { closed: true, .. }));
        assert!(matches!(read[2].kind, ShapeKind::Path { closed: true, .. }));
        // A fixed radius on an evenly scaled rectangle is still a rectangle.
        assert!(
            matches!(read[3].kind, ShapeKind::Rect { radius, .. } if (radius - 1.5).abs() < 0.01)
        );
    }

    #[test]
    fn complex_stars_are_written_as_curves() {
        let mut doc = doc_with(Vec::new());
        let mut ids = doc.ids().clone();
        let mut star = Shape::new(
            ids.shape(),
            ShapeKind::Polygon {
                rect: Rect::new(10.0, 10.0, 50.0, 50.0),
                points: 6,
                sharpness: 0.0,
                complex: Some(1),
            },
        );
        star.fill = Fill::Solid(Color::BLACK);
        doc.pages[0].layers[0].shapes = vec![star.clone()];
        doc.set_ids(ids);
        let (back, rep) = crate::open_bytes(&document_to_cdr(&doc), "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 1, "{:?}", rep.warnings);
        let ShapeKind::Path { path, .. } = &read[0].kind else {
            panic!("{:?}", read[0].kind);
        };
        // Two triangles, as drawn.
        let moves = path
            .elements()
            .iter()
            .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
            .count();
        assert_eq!(moves, 2);
        let (a, b) = (
            star.page_path().area().abs(),
            read[0].page_path().area().abs(),
        );
        assert!((a - b).abs() < 0.5, "{a} vs {b}");
    }
}
