//! PSD import: Photoshop documents as bitmaps.
//!
//! Layers become one bitmap object each (name, opacity, visibility kept),
//! inside a group; a file without usable layers, or with a flattened
//! composite only, gives a single bitmap from the composite image data.
//! Supported: 8 and 16 bits per channel, Bitmap, Grayscale, Indexed, RGB,
//! CMYK, Duotone and Lab modes, raw and RLE (PackBits) compression, layer
//! masks (applied as alpha). Blend modes and adjustment layers are not
//! interpreted; adjustment layers have no pixels and are skipped.

use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Rect, Size},
    id::IdSource,
    Document, Fill, ShapeId,
};

/// Millimetres per pixel at 72 ppi (the default when the file has no
/// resolution resource).
pub const PX_MM: f64 = 25.4 / 72.0;

#[derive(Debug, Clone)]
pub struct Imported {
    pub shapes: Vec<Shape>,
    pub size: Size,
    pub warnings: Vec<String>,
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn u8(&mut self) -> Option<u8> {
        let v = *self.b.get(self.pos)?;
        self.pos += 1;
        Some(v)
    }
    fn u16(&mut self) -> Option<u16> {
        let s = self.b.get(self.pos..self.pos + 2)?;
        self.pos += 2;
        Some(u16::from_be_bytes([s[0], s[1]]))
    }
    fn i16(&mut self) -> Option<i16> {
        self.u16().map(|v| v as i16)
    }
    fn u32(&mut self) -> Option<u32> {
        let s = self.b.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn i32(&mut self) -> Option<i32> {
        self.u32().map(|v| v as i32)
    }
    fn u64(&mut self) -> Option<u64> {
        let s = self.b.get(self.pos..self.pos + 8)?;
        self.pos += 8;
        Some(u64::from_be_bytes([
            s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
        ]))
    }
    fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.pos..self.pos.checked_add(n)?)?;
        self.pos += n;
        Some(s)
    }
    fn skip(&mut self, n: usize) -> Option<()> {
        self.pos = self.pos.checked_add(n)?;
        (self.pos <= self.b.len()).then_some(())
    }
    fn remaining(&self) -> usize {
        self.b.len().saturating_sub(self.pos)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Bitmap,
    Gray,
    Indexed,
    Rgb,
    Cmyk,
    Multichannel,
    Duotone,
    Lab,
}

impl Mode {
    fn from(v: u16) -> Option<Mode> {
        Some(match v {
            0 => Mode::Bitmap,
            1 => Mode::Gray,
            2 => Mode::Indexed,
            3 => Mode::Rgb,
            4 => Mode::Cmyk,
            7 => Mode::Multichannel,
            8 => Mode::Duotone,
            9 => Mode::Lab,
            _ => return None,
        })
    }
    /// Colour channels (without alpha).
    fn channels(self) -> usize {
        match self {
            Mode::Rgb | Mode::Lab => 3,
            Mode::Cmyk => 4,
            _ => 1,
        }
    }
}

struct Header {
    version: u16,
    channels: usize,
    depth: u16,
    mode: Mode,
}

/// Is the buffer a PSD or PSB file?
pub fn is_psd(bytes: &[u8]) -> bool {
    bytes.starts_with(b"8BPS")
}

pub fn parse(bytes: &[u8], ids: &mut IdSource) -> Result<Imported, String> {
    let mut r = Reader { b: bytes, pos: 0 };
    if r.bytes(4) != Some(b"8BPS") {
        return Err("not a PSD file".into());
    }
    let version = r.u16().ok_or("truncated header")?;
    if version != 1 && version != 2 {
        return Err(format!("PSD version {version} not supported"));
    }
    r.skip(6);
    let channels = r.u16().ok_or("truncated header")? as usize;
    let height = r.u32().ok_or("truncated header")?;
    let width = r.u32().ok_or("truncated header")?;
    let depth = r.u16().ok_or("truncated header")?;
    let mode = Mode::from(r.u16().ok_or("truncated header")?).ok_or("unknown colour mode")?;
    if width == 0
        || height == 0
        || width > 300_000
        || height > 300_000
        || (width as u64) * (height as u64) > 200_000_000
    {
        return Err("image size out of range".into());
    }
    if !matches!(depth, 1 | 8 | 16 | 32) {
        return Err(format!("{depth} bits per channel not supported"));
    }
    let header = Header {
        version,
        channels,
        depth,
        mode,
    };
    let mut warnings = Vec::new();
    // Colour mode data (palette for Indexed, duotone specs).
    let cm_len = r.u32().ok_or("truncated colour mode data")? as usize;
    let palette: Option<Vec<u8>> = if mode == Mode::Indexed && cm_len >= 768 {
        let p = r.bytes(cm_len).ok_or("truncated palette")?;
        // Planar: 256 reds, 256 greens, 256 blues.
        let mut pal = Vec::with_capacity(768);
        for i in 0..256 {
            pal.extend_from_slice(&[p[i], p[256 + i], p[512 + i]]);
        }
        Some(pal)
    } else {
        r.skip(cm_len).ok_or("truncated colour mode data")?;
        None
    };
    // Image resources: resolution (0x03ED) for the page size.
    let res_len = r.u32().ok_or("truncated resources")? as usize;
    let res = r.bytes(res_len).ok_or("truncated resources")?;
    let (ppi_x, ppi_y) = resolution(res).unwrap_or((72.0, 72.0));
    let mm_x = 25.4 / ppi_x.max(1.0);
    let mm_y = 25.4 / ppi_y.max(1.0);
    let size = Size::new(width as f64 * mm_x, height as f64 * mm_y);
    // Layer and mask information.
    let lm_len = if version == 2 {
        r.u64().ok_or("truncated layer info")? as usize
    } else {
        r.u32().ok_or("truncated layer info")? as usize
    };
    let lm_start = r.pos;
    let layers_data = r.bytes(lm_len.min(r.remaining()));
    let mut shapes = Vec::new();
    if let Some(ld) = layers_data {
        match parse_layers(ld, &header, palette.as_deref(), &mut warnings) {
            Ok(layers) if !layers.is_empty() => {
                for l in layers {
                    let Some((png, w, h)) = l.png else { continue };
                    let rect = Rect::new(
                        l.left as f64 * mm_x,
                        size.height - l.bottom as f64 * mm_y,
                        l.right as f64 * mm_x,
                        size.height - l.top as f64 * mm_y,
                    );
                    let mut s = Shape::new(
                        ShapeId(ids.shape().0),
                        ShapeKind::Bitmap {
                            rect,
                            width_px: w,
                            height_px: h,
                            png,
                        },
                    );
                    s.fill = Fill::None;
                    s.stroke = None;
                    s.opacity = l.opacity;
                    s.visible = l.visible;
                    s.name = Some(l.name);
                    shapes.push(s);
                }
            }
            Ok(_) => {}
            Err(e) => warnings.push(format!("layers not read ({e}); composite used")),
        }
    }
    r.pos = lm_start.saturating_add(lm_len).min(bytes.len());
    if shapes.is_empty() {
        // Composite image data.
        let compression = r.u16().ok_or("truncated image data")?;
        let data = &bytes[r.pos.min(bytes.len())..];
        match decode_channels(
            data,
            compression,
            &header,
            header.channels,
            width,
            height,
            version,
        ) {
            Some(planes) => {
                let rgba = to_rgba(
                    &planes,
                    &header,
                    palette.as_deref(),
                    width,
                    height,
                    header.channels > mode.channels(),
                );
                let png = crate::svg::encode_png(width, height, &rgba);
                let mut s = Shape::new(
                    ShapeId(ids.shape().0),
                    ShapeKind::Bitmap {
                        rect: Rect::new(0.0, 0.0, size.width, size.height),
                        width_px: width,
                        height_px: height,
                        png,
                    },
                );
                s.fill = Fill::None;
                s.stroke = None;
                shapes.push(s);
            }
            None => return Err("image data could not be decoded".into()),
        }
    }
    Ok(Imported {
        shapes,
        size,
        warnings,
    })
}

/// Resolution info resource 0x03ED: hres (fixed 16.16), unit, width unit, vres, ...
fn resolution(res: &[u8]) -> Option<(f64, f64)> {
    let mut r = Reader { b: res, pos: 0 };
    while r.remaining() >= 12 {
        if r.bytes(4)? != b"8BIM" {
            return None;
        }
        let id = r.u16()?;
        // Pascal name padded to even length.
        let nlen = r.u8()? as usize;
        r.skip(nlen)?;
        if nlen.is_multiple_of(2) {
            r.skip(1)?;
        }
        let len = r.u32()? as usize;
        let start = r.pos;
        if id == 0x03ED && len >= 16 {
            let h = r.u32()? as f64 / 65536.0;
            r.skip(4)?;
            let v = r.u32()? as f64 / 65536.0;
            if h > 0.0 && v > 0.0 {
                return Some((h, v));
            }
            return None;
        }
        r.pos = start + len + (len % 2);
    }
    None
}

struct LayerOut {
    name: String,
    top: i32,
    left: i32,
    bottom: i32,
    right: i32,
    opacity: f64,
    visible: bool,
    png: Option<(Vec<u8>, u32, u32)>,
}

struct ChannelInfo {
    id: i16,
    len: usize,
}

struct LayerRec {
    top: i32,
    left: i32,
    bottom: i32,
    right: i32,
    channels: Vec<ChannelInfo>,
    opacity: u8,
    hidden: bool,
    name: String,
    mask: Option<(i32, i32, i32, i32, u8)>,
}

fn parse_layers(
    data: &[u8],
    h: &Header,
    palette: Option<&[u8]>,
    warnings: &mut Vec<String>,
) -> Result<Vec<LayerOut>, String> {
    let mut r = Reader { b: data, pos: 0 };
    let li_len = if h.version == 2 {
        r.u64().ok_or("truncated")? as usize
    } else {
        r.u32().ok_or("truncated")? as usize
    };
    if li_len == 0 {
        return Ok(Vec::new());
    }
    let count = r.i16().ok_or("truncated")?;
    let count = count.unsigned_abs() as usize;
    if count > 10_000 {
        return Err("too many layers".into());
    }
    let mut recs = Vec::with_capacity(count);
    for _ in 0..count {
        let top = r.i32().ok_or("truncated layer record")?;
        let left = r.i32().ok_or("truncated layer record")?;
        let bottom = r.i32().ok_or("truncated layer record")?;
        let right = r.i32().ok_or("truncated layer record")?;
        let nch = r.u16().ok_or("truncated layer record")? as usize;
        if nch > 64 {
            return Err("channel count out of range".into());
        }
        let mut channels = Vec::with_capacity(nch);
        for _ in 0..nch {
            let id = r.i16().ok_or("truncated channel info")?;
            let len = if h.version == 2 {
                r.u64().ok_or("truncated")? as usize
            } else {
                r.u32().ok_or("truncated")? as usize
            };
            channels.push(ChannelInfo { id, len });
        }
        if r.bytes(4) != Some(b"8BIM") {
            return Err("bad blend mode signature".into());
        }
        let _blend = r.bytes(4).ok_or("truncated")?;
        let opacity = r.u8().ok_or("truncated")?;
        let _clipping = r.u8().ok_or("truncated")?;
        let flags = r.u8().ok_or("truncated")?;
        r.skip(1);
        let extra_len = r.u32().ok_or("truncated")? as usize;
        let extra_start = r.pos;
        // Mask data.
        let mask_len = r.u32().ok_or("truncated")? as usize;
        let mut mask = None;
        if mask_len >= 20 {
            let mt = r.i32().ok_or("truncated")?;
            let ml = r.i32().ok_or("truncated")?;
            let mb = r.i32().ok_or("truncated")?;
            let mr = r.i32().ok_or("truncated")?;
            let default_color = r.u8().ok_or("truncated")?;
            mask = Some((mt, ml, mb, mr, default_color));
            r.pos = extra_start + 4 + mask_len;
        } else {
            r.skip(mask_len);
        }
        // Blending ranges.
        let br_len = r.u32().ok_or("truncated")? as usize;
        r.skip(br_len);
        // Pascal name, padded to 4.
        let nlen = r.u8().ok_or("truncated")? as usize;
        let name_bytes = r.bytes(nlen).ok_or("truncated name")?;
        let mut name = String::from_utf8_lossy(name_bytes).into_owned();
        let pad = (4 - ((nlen + 1) % 4)) % 4;
        r.skip(pad);
        // Additional layer info: look for a Unicode name (luni) and a
        // section divider (lsct) marking group boundaries.
        let mut is_divider = false;
        while r.pos + 12 <= extra_start + extra_len && r.pos + 12 <= data.len() {
            let sig = r.bytes(4).unwrap_or(b"");
            if sig != b"8BIM" && sig != b"8B64" {
                break;
            }
            let key = r.bytes(4).unwrap_or(b"").to_vec();
            let big = h.version == 2
                && matches!(
                    key.as_slice(),
                    b"LMsk"
                        | b"Lr16"
                        | b"Lr32"
                        | b"Layr"
                        | b"Mt16"
                        | b"Mt32"
                        | b"Mtrn"
                        | b"Alph"
                        | b"FMsk"
                        | b"lnk2"
                        | b"FEid"
                        | b"FXid"
                        | b"PxSD"
                );
            let len = if big {
                r.u64().unwrap_or(0) as usize
            } else {
                r.u32().unwrap_or(0) as usize
            };
            let start = r.pos;
            match key.as_slice() {
                b"luni" => {
                    if let Some(n) = r.u32() {
                        let n = (n as usize).min(1024);
                        let mut units = Vec::with_capacity(n);
                        for _ in 0..n {
                            if let Some(u) = r.u16() {
                                units.push(u);
                            }
                        }
                        let s = String::from_utf16_lossy(&units);
                        if !s.is_empty() {
                            name = s;
                        }
                    }
                }
                b"lsct" => is_divider = true,
                _ => {}
            }
            r.pos = start + len + (len % 2);
        }
        r.pos = extra_start + extra_len;
        recs.push((
            LayerRec {
                top,
                left,
                bottom,
                right,
                channels,
                opacity,
                hidden: flags & 2 != 0,
                name,
                mask,
            },
            is_divider,
        ));
    }
    // Channel image data follows, in layer order.
    let mut out = Vec::new();
    for (rec, divider) in recs {
        let w = (rec.right - rec.left).max(0) as u32;
        let hgt = (rec.bottom - rec.top).max(0) as u32;
        let mut planes: Vec<(i16, Vec<u8>)> = Vec::new();
        for ch in &rec.channels {
            let chunk = r.bytes(ch.len).ok_or("truncated channel data")?;
            if divider || w == 0 || hgt == 0 {
                continue;
            }
            let (cw, chh) = if ch.id == -2 {
                match rec.mask {
                    Some((mt, ml, mb, mr, _)) => ((mr - ml).max(0) as u32, (mb - mt).max(0) as u32),
                    None => (0, 0),
                }
            } else {
                (w, hgt)
            };
            if cw == 0 || chh == 0 || ch.len < 2 {
                continue;
            }
            let compression = u16::from_be_bytes([chunk[0], chunk[1]]);
            if let Some(mut p) = decode_channels(&chunk[2..], compression, h, 1, cw, chh, h.version)
            {
                if let Some(plane) = p.pop() {
                    planes.push((ch.id, plane));
                }
            }
        }
        if divider || w == 0 || hgt == 0 {
            continue;
        }
        if (w as u64) * (hgt as u64) > 100_000_000 {
            warnings.push(format!("layer {} too large; skipped", rec.name));
            continue;
        }
        // Order colour planes by id 0..n, then alpha (-1), then mask (-2).
        let n = h.mode.channels();
        let mut color: Vec<Vec<u8>> = Vec::new();
        for id in 0..n as i16 {
            match planes.iter().find(|(i, _)| *i == id) {
                Some((_, p)) => color.push(p.clone()),
                None => color.push(vec![0; (w * hgt) as usize * bytes_per_sample(h.depth)]),
            }
        }
        let alpha = planes
            .iter()
            .find(|(i, _)| *i == -1)
            .map(|(_, p)| p.clone());
        let has_alpha = alpha.is_some();
        if let Some(a) = alpha {
            color.push(a);
        }
        let mut rgba = to_rgba(&color, h, palette, w, hgt, has_alpha);
        // Layer mask: multiply alpha where the mask covers the layer.
        if let (Some((_, mask)), Some((mt, ml, mb, mr, default))) =
            (planes.iter().find(|(i, _)| *i == -2), rec.mask)
        {
            let (mw, mh) = ((mr - ml).max(0) as u32, (mb - mt).max(0) as u32);
            let bps = bytes_per_sample(h.depth);
            for y in 0..hgt {
                for x in 0..w {
                    let gx = rec.left + x as i32 - ml;
                    let gy = rec.top + y as i32 - mt;
                    let m = if gx >= 0 && gy >= 0 && (gx as u32) < mw && (gy as u32) < mh {
                        let i = (gy as u32 * mw + gx as u32) as usize * bps;
                        sample8(mask, i, h.depth)
                    } else {
                        default
                    };
                    let k = ((y * w + x) * 4 + 3) as usize;
                    rgba[k] = ((rgba[k] as u32 * m as u32) / 255) as u8;
                }
            }
        }
        let png = crate::svg::encode_png(w, hgt, &rgba);
        out.push(LayerOut {
            name: rec.name,
            top: rec.top,
            left: rec.left,
            bottom: rec.bottom,
            right: rec.right,
            opacity: rec.opacity as f64 / 255.0,
            visible: !rec.hidden,
            png: Some((png, w, hgt)),
        });
    }
    // Layers are stored bottom to top already.
    Ok(out)
}

fn bytes_per_sample(depth: u16) -> usize {
    match depth {
        16 => 2,
        32 => 4,
        _ => 1,
    }
}

/// An 8-bit sample from a plane at byte offset `i`.
fn sample8(plane: &[u8], i: usize, depth: u16) -> u8 {
    match depth {
        16 => *plane.get(i).unwrap_or(&0),
        32 => {
            let b = plane.get(i..i + 4).unwrap_or(&[0, 0, 0, 0]);
            let f = f32::from_be_bytes([b[0], b[1], b[2], b[3]]);
            (f.clamp(0.0, 1.0) * 255.0).round() as u8
        }
        _ => *plane.get(i).unwrap_or(&0),
    }
}

/// Decode `nch` planes of `w` x `h` samples (raw or RLE) into byte planes.
fn decode_channels(
    data: &[u8],
    compression: u16,
    h: &Header,
    nch: usize,
    w: u32,
    hgt: u32,
    version: u16,
) -> Option<Vec<Vec<u8>>> {
    let bps = bytes_per_sample(h.depth);
    let row_bytes = if h.depth == 1 {
        (w as usize).div_ceil(8)
    } else {
        w as usize * bps
    };
    let plane_len = row_bytes * hgt as usize;
    let mut planes = Vec::with_capacity(nch);
    match compression {
        0 => {
            for c in 0..nch {
                let start = c * plane_len;
                let p = data.get(start..start + plane_len)?;
                planes.push(p.to_vec());
            }
        }
        1 => {
            // Row byte counts for every row of every channel, then packed rows.
            let count_size = if version == 2 { 4 } else { 2 };
            let rows = hgt as usize * nch;
            let mut r = Reader { b: data, pos: 0 };
            let mut counts = Vec::with_capacity(rows);
            for _ in 0..rows {
                counts.push(if version == 2 {
                    r.u32()? as usize
                } else {
                    r.u16()? as usize
                });
            }
            let _ = count_size;
            for c in 0..nch {
                let mut plane = Vec::with_capacity(plane_len);
                for y in 0..hgt as usize {
                    let n = counts[c * hgt as usize + y];
                    let packed = r.bytes(n)?;
                    unpack_bits(packed, &mut plane, row_bytes);
                }
                plane.resize(plane_len, 0);
                planes.push(plane);
            }
        }
        2 | 3 => {
            // ZIP (with or without prediction): one zlib stream per channel set.
            use std::io::Read;
            let mut out = Vec::new();
            let _ = flate2::read::ZlibDecoder::new(data).read_to_end(&mut out);
            if out.len() < plane_len * nch {
                return None;
            }
            for c in 0..nch {
                let mut p = out[c * plane_len..(c + 1) * plane_len].to_vec();
                if compression == 3 {
                    // Delta prediction per row.
                    for row in p.chunks_mut(row_bytes) {
                        if bps == 1 {
                            for i in 1..row.len() {
                                row[i] = row[i].wrapping_add(row[i - 1]);
                            }
                        } else if bps == 2 {
                            for i in 1..row.len() / 2 {
                                let prev = u16::from_be_bytes([row[2 * i - 2], row[2 * i - 1]]);
                                let cur = u16::from_be_bytes([row[2 * i], row[2 * i + 1]]);
                                let v = cur.wrapping_add(prev).to_be_bytes();
                                row[2 * i] = v[0];
                                row[2 * i + 1] = v[1];
                            }
                        }
                    }
                }
                planes.push(p);
            }
        }
        _ => return None,
    }
    Some(planes)
}

fn unpack_bits(packed: &[u8], out: &mut Vec<u8>, limit: usize) {
    let start = out.len();
    let mut i = 0;
    while i < packed.len() && out.len() - start < limit {
        let n = packed[i] as i8;
        i += 1;
        if n >= 0 {
            let count = n as usize + 1;
            let end = (i + count).min(packed.len());
            out.extend_from_slice(&packed[i..end]);
            i = end;
        } else if n != -128 {
            let count = (-(n as i32) + 1) as usize;
            if let Some(&b) = packed.get(i) {
                out.extend(std::iter::repeat_n(b, count));
            }
            i += 1;
        }
    }
    out.truncate(start + limit);
    out.resize(start + limit, 0);
}

/// Planes to RGBA8 per the colour mode.
fn to_rgba(
    planes: &[Vec<u8>],
    h: &Header,
    palette: Option<&[u8]>,
    w: u32,
    hgt: u32,
    has_alpha: bool,
) -> Vec<u8> {
    let n = (w * hgt) as usize;
    let mut out = Vec::with_capacity(n * 4);
    let bps = bytes_per_sample(h.depth);
    let nc = h.mode.channels();
    let alpha_plane = if has_alpha { planes.get(nc) } else { None };
    for i in 0..n {
        let s = |c: usize| -> u8 {
            match planes.get(c) {
                Some(p) => {
                    if h.depth == 1 {
                        let byte = p.get(i / 8).copied().unwrap_or(0);
                        // 1 = black in bitmap mode.
                        if (byte >> (7 - (i % 8))) & 1 == 1 {
                            0
                        } else {
                            255
                        }
                    } else {
                        sample8(p, i * bps, h.depth)
                    }
                }
                None => 0,
            }
        };
        let (r, g, b) = match h.mode {
            Mode::Rgb => (s(0), s(1), s(2)),
            Mode::Cmyk => {
                // Photoshop stores CMYK inverted (0 = full ink).
                let (c, m, y, k) = (
                    s(0) as f32 / 255.0,
                    s(1) as f32 / 255.0,
                    s(2) as f32 / 255.0,
                    s(3) as f32 / 255.0,
                );
                let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                (q(c * k), q(m * k), q(y * k))
            }
            Mode::Indexed => {
                let idx = s(0) as usize * 3;
                match palette {
                    Some(p) if idx + 2 < p.len() => (p[idx], p[idx + 1], p[idx + 2]),
                    _ => (s(0), s(0), s(0)),
                }
            }
            Mode::Lab => {
                let l = s(0) as f32 / 255.0 * 100.0;
                let a = s(1) as f32 - 128.0;
                let bb = s(2) as f32 - 128.0;
                let [r, g, b] = tracedraw_core::color::lab_to_rgb(l, a, bb);
                let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                (q(r), q(g), q(b))
            }
            _ => (s(0), s(0), s(0)),
        };
        let a = match alpha_plane {
            Some(p) => sample8(p, i * bps, h.depth),
            None => 255,
        };
        out.extend_from_slice(&[r, g, b, a]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal PSD: header, no palette, no resources, optional
    /// layers block, composite image data.
    fn psd(
        width: u32,
        height: u32,
        mode: u16,
        channels: u16,
        layers: &[u8],
        image: &[u8],
    ) -> Vec<u8> {
        let mut b = b"8BPS".to_vec();
        b.extend_from_slice(&1u16.to_be_bytes());
        b.extend_from_slice(&[0; 6]);
        b.extend_from_slice(&channels.to_be_bytes());
        b.extend_from_slice(&height.to_be_bytes());
        b.extend_from_slice(&width.to_be_bytes());
        b.extend_from_slice(&8u16.to_be_bytes());
        b.extend_from_slice(&mode.to_be_bytes());
        b.extend_from_slice(&0u32.to_be_bytes()); // colour mode data
        b.extend_from_slice(&0u32.to_be_bytes()); // resources
        b.extend_from_slice(&(layers.len() as u32).to_be_bytes());
        b.extend_from_slice(layers);
        b.extend_from_slice(image);
        b
    }

    #[test]
    fn raw_rgb_composite_becomes_one_bitmap() {
        // 2x1 RGB raw: planes R [255, 0], G [0, 255], B [0, 0].
        let mut img = 0u16.to_be_bytes().to_vec();
        img.extend_from_slice(&[255, 0, 0, 255, 0, 0]);
        let bytes = psd(2, 1, 3, 3, &[], &img);
        let mut ids = IdSource::default();
        let imp = parse(&bytes, &mut ids).expect("parse");
        assert_eq!(imp.shapes.len(), 1);
        match &imp.shapes[0].kind {
            ShapeKind::Bitmap { png, width_px, .. } => {
                assert_eq!(*width_px, 2);
                let pm = tiny_skia::Pixmap::decode_png(png).unwrap();
                assert_eq!((pm.pixels()[0].red(), pm.pixels()[1].green()), (255, 255));
            }
            other => panic!("{other:?}"),
        }
        // 72 ppi: 2 px = 2 pt.
        assert!((imp.size.width - 2.0 * PX_MM).abs() < 1e-9);
    }

    #[test]
    fn rle_layer_with_alpha_and_name() {
        // One 2x2 RGBA layer at (1,1)-(3,3) in a 4x4 document.
        let mut layer_info = Vec::new();
        layer_info.extend_from_slice(&1i16.to_be_bytes()); // count
                                                           // record
        for v in [1i32, 1, 3, 3] {
            layer_info.extend_from_slice(&v.to_be_bytes());
        }
        layer_info.extend_from_slice(&4u16.to_be_bytes()); // channels
                                                           // Each channel: RLE, 2 rows, each row packed as [1, a, b] (literal 2).
        let chan = |a: u8, b: u8| -> Vec<u8> {
            let mut c = 1u16.to_be_bytes().to_vec();
            c.extend_from_slice(&3u16.to_be_bytes());
            c.extend_from_slice(&3u16.to_be_bytes());
            c.extend_from_slice(&[1, a, b, 1, a, b]);
            c
        };
        let chans = [
            (-1i16, chan(255, 128)),
            (0, chan(10, 20)),
            (1, chan(30, 40)),
            (2, chan(50, 60)),
        ];
        for (id, data) in &chans {
            layer_info.extend_from_slice(&id.to_be_bytes());
            layer_info.extend_from_slice(&(data.len() as u32).to_be_bytes());
        }
        layer_info.extend_from_slice(b"8BIMnorm");
        layer_info.push(200); // opacity
        layer_info.push(0); // clipping
        layer_info.push(0); // flags
        layer_info.push(0);
        // extra: mask len 0, ranges len 0, name "Hi" padded.
        let mut extra = Vec::new();
        extra.extend_from_slice(&0u32.to_be_bytes());
        extra.extend_from_slice(&0u32.to_be_bytes());
        extra.push(2);
        extra.extend_from_slice(b"Hi");
        extra.push(0); // pad to 4
        layer_info.extend_from_slice(&(extra.len() as u32).to_be_bytes());
        layer_info.extend_from_slice(&extra);
        for (_, data) in &chans {
            layer_info.extend_from_slice(data);
        }
        let mut layers = Vec::new();
        layers.extend_from_slice(&(layer_info.len() as u32).to_be_bytes());
        layers.extend_from_slice(&layer_info);
        let mut img = 0u16.to_be_bytes().to_vec();
        img.extend_from_slice(&[0; 48]);
        let bytes = psd(4, 4, 3, 3, &layers, &img);
        let mut ids = IdSource::default();
        let imp = parse(&bytes, &mut ids).expect("parse");
        assert_eq!(imp.shapes.len(), 1, "{:?}", imp.warnings);
        let s = &imp.shapes[0];
        assert_eq!(s.name.as_deref(), Some("Hi"));
        assert!((s.opacity - 200.0 / 255.0).abs() < 1e-9);
        match &s.kind {
            ShapeKind::Bitmap {
                png,
                width_px,
                height_px,
                rect,
            } => {
                assert_eq!((*width_px, *height_px), (2, 2));
                let pm = tiny_skia::Pixmap::decode_png(png).unwrap();
                let p0 = pm.pixels()[0].demultiply();
                assert_eq!(
                    (p0.red(), p0.green(), p0.blue(), p0.alpha()),
                    (10, 30, 50, 255)
                );
                assert_eq!(pm.pixels()[1].demultiply().alpha(), 128);
                // Placed at pixel (1,1): x from 1 px, top 1 px below the page top.
                assert!(
                    (rect.x0 - PX_MM).abs() < 1e-9 && (rect.y1 - 3.0 * PX_MM).abs() < 1e-9,
                    "{rect:?}"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn garbage_is_an_error() {
        let mut ids = IdSource::default();
        assert!(parse(b"nope", &mut ids).is_err());
        assert!(parse(b"8BPS\x00\x01", &mut ids).is_err());
    }
}

// ----- writer ------------------------------------------------------------------

/// Largest pixel dimension written (the PSD version 1 limit is 30000).
const MAX_WRITE_PX: f64 = 30000.0;

fn unpremultiply(px: &tiny_skia::Pixmap) -> Vec<[u8; 4]> {
    px.pixels()
        .iter()
        .map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

fn put_pascal(out: &mut Vec<u8>, name: &str, pad: usize) {
    let bytes: Vec<u8> = name.bytes().take(255).collect();
    let start = out.len();
    out.push(bytes.len() as u8);
    out.extend_from_slice(&bytes);
    while !(out.len() - start).is_multiple_of(pad) {
        out.push(0);
    }
}

/// Render one page and write it as a Photoshop file: one raster layer per
/// visible document layer (master layers included) over a white
/// background, plus the flattened composite, RGB 8-bit, uncompressed, at
/// `dpi` (clamped so neither side exceeds 30000 px). `None` when the
/// page does not exist or cannot be rendered.
pub fn page_to_psd(doc: &Document, page_index: usize, dpi: f64) -> Option<Vec<u8>> {
    let page = doc.pages.get(page_index)?;
    let page_id = page.id;
    let max_dpi = (MAX_WRITE_PX / page.size.width.max(page.size.height) * 25.4).max(1.0);
    let dpi = dpi.clamp(1.0, max_dpi);
    let zoom = dpi / 25.4;
    let w = (page.size.width * zoom).ceil().max(1.0) as u32;
    let h = (page.size.height * zoom).ceil().max(1.0) as u32;
    let view = tracedraw_render::ViewTransform {
        zoom,
        origin_x: 0.0,
        origin_y: h as f64,
    };
    let opts = || tracedraw_render::RenderOptions {
        width: w,
        height: h,
        view,
        preview: None,
        wireframe: false,
        simulate_overprints: false,
        complex_effects: true,
    };

    // One render per visible layer, with every other layer hidden.
    let layer_ids: Vec<(tracedraw_core::id::LayerId, String)> = doc
        .layers_for_page(page_id)
        .ok()?
        .iter()
        .filter(|l| l.visible)
        .map(|l| (l.id, l.name.clone()))
        .collect();
    let mut layers: Vec<(String, Vec<[u8; 4]>)> = Vec::new();
    for (id, name) in &layer_ids {
        let mut one = doc.clone();
        for l in one.master.iter_mut() {
            l.visible = l.id == *id;
        }
        for p in one.pages.iter_mut() {
            for l in p.layers.iter_mut() {
                l.visible = l.id == *id;
            }
        }
        let px = tracedraw_render::render_page(&one, page_id, &opts())?;
        layers.push((name.clone(), unpremultiply(&px)));
    }
    let composite = tracedraw_render::render_page_image_with(doc, page_id, dpi, &opts())?;
    let comp = unpremultiply(&composite);
    let n = (w as usize) * (h as usize);

    let mut out = Vec::with_capacity(n * 3 + layers.len() * n * 4 + 256);
    // Header.
    out.extend_from_slice(b"8BPS");
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&[0u8; 6]);
    out.extend_from_slice(&3u16.to_be_bytes()); // channels
    out.extend_from_slice(&h.to_be_bytes());
    out.extend_from_slice(&w.to_be_bytes());
    out.extend_from_slice(&8u16.to_be_bytes()); // depth
    out.extend_from_slice(&3u16.to_be_bytes()); // RGB
                                                // Colour mode data: none.
    out.extend_from_slice(&0u32.to_be_bytes());
    // Image resources: resolution info (0x03ED).
    let mut res = Vec::new();
    res.extend_from_slice(b"8BIM");
    res.extend_from_slice(&0x03EDu16.to_be_bytes());
    res.extend_from_slice(&[0, 0]); // empty pascal name, padded
    res.extend_from_slice(&16u32.to_be_bytes());
    let fixed = ((dpi * 65536.0).round() as u32).to_be_bytes();
    res.extend_from_slice(&fixed);
    res.extend_from_slice(&1u16.to_be_bytes()); // pixels per inch
    res.extend_from_slice(&2u16.to_be_bytes()); // width unit: cm
    res.extend_from_slice(&fixed);
    res.extend_from_slice(&1u16.to_be_bytes());
    res.extend_from_slice(&2u16.to_be_bytes());
    out.extend_from_slice(&(res.len() as u32).to_be_bytes());
    out.extend_from_slice(&res);

    // Layer and mask information.
    let mut layer_info = Vec::new();
    layer_info.extend_from_slice(&(layers.len() as i16).to_be_bytes());
    let channel_len = (2 + n) as u32;
    for (name, _) in &layers {
        layer_info.extend_from_slice(&0u32.to_be_bytes()); // top
        layer_info.extend_from_slice(&0u32.to_be_bytes()); // left
        layer_info.extend_from_slice(&h.to_be_bytes()); // bottom
        layer_info.extend_from_slice(&w.to_be_bytes()); // right
        layer_info.extend_from_slice(&4u16.to_be_bytes());
        for ch in [-1i16, 0, 1, 2] {
            layer_info.extend_from_slice(&ch.to_be_bytes());
            layer_info.extend_from_slice(&channel_len.to_be_bytes());
        }
        layer_info.extend_from_slice(b"8BIMnorm");
        layer_info.push(255); // opacity
        layer_info.push(0); // clipping: base
        layer_info.push(0); // flags: visible
        layer_info.push(0);
        let mut extra = Vec::new();
        extra.extend_from_slice(&0u32.to_be_bytes()); // mask data
        extra.extend_from_slice(&0u32.to_be_bytes()); // blending ranges
        put_pascal(&mut extra, name, 4);
        layer_info.extend_from_slice(&(extra.len() as u32).to_be_bytes());
        layer_info.extend_from_slice(&extra);
    }
    for (_, px) in &layers {
        for ch in [3usize, 0, 1, 2] {
            layer_info.extend_from_slice(&0u16.to_be_bytes()); // raw
            layer_info.extend(px.iter().map(|p| p[ch]));
        }
    }
    if layer_info.len() % 2 == 1 {
        layer_info.push(0);
    }
    let mut lm = Vec::new();
    if layers.is_empty() {
        lm.extend_from_slice(&0u32.to_be_bytes());
    } else {
        lm.extend_from_slice(&(layer_info.len() as u32).to_be_bytes());
        lm.extend_from_slice(&layer_info);
        lm.extend_from_slice(&0u32.to_be_bytes()); // global mask: none
    }
    out.extend_from_slice(&(lm.len() as u32).to_be_bytes());
    out.extend_from_slice(&lm);

    // Composite image data: raw, planar.
    out.extend_from_slice(&0u16.to_be_bytes());
    for ch in 0..3usize {
        out.extend(comp.iter().map(|p| p[ch]));
    }
    Some(out)
}

#[cfg(test)]
mod write_tests {
    use super::*;
    use tracedraw_core::geometry::{Rect, Size};
    use tracedraw_core::{Color, Fill, Shape, ShapeKind};

    #[test]
    fn written_psd_reads_back_with_its_layers_and_pixels() {
        let mut doc = Document::new("t", Size::new(20.0, 10.0));
        let mut ids = doc.ids().clone();
        let mut r = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
            },
        );
        r.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        r.stroke = None;
        doc.pages[0].layers[0].shapes.push(r);
        let second = tracedraw_core::Layer::new(ids.layer(), "Top");
        doc.pages[0].layers.push(second);
        doc.set_ids(ids);
        let bytes = page_to_psd(&doc, 0, 25.4).unwrap();
        assert!(bytes.starts_with(b"8BPS"));
        let mut rid = IdSource::default();
        let back = parse(&bytes, &mut rid).unwrap();
        assert_eq!(back.shapes.len(), 2, "one layer per document layer");
        assert!((back.size.width - 20.0).abs() < 0.5, "{:?}", back.size);
        // The layer pixels: the left half red, the right transparent.
        let ShapeKind::Bitmap { png, width_px, .. } = &back.shapes[0].kind else {
            panic!("expected a bitmap layer");
        };
        assert_eq!(*width_px, 20);
        let img = image::load_from_memory(png).unwrap().to_rgba8();
        let left = img.get_pixel(2, 5);
        let right = img.get_pixel(17, 5);
        assert_eq!(left.0, [255, 0, 0, 255]);
        assert_eq!(right.0[3], 0);
        // An absent page.
        assert!(page_to_psd(&doc, 5, 72.0).is_none());
    }
}
