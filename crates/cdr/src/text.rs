//! Text chunks: `font` (font table), `stlt` (style table) and `txsm` (text
//! content with its runs), plus the string encodings they use.
//!
//! Layouts follow the public declarative description of the format; see
//! `docs/cdr-format.md` for the status of each fact. Everything here is
//! tolerant: a malformed chunk yields `None` or a shorter result, never a
//! panic.

use crate::parse::Reader;

/// Points per coordinate unit (version 6+: 1/254000 inch).
const PT_PER_UNIT: f64 = 72.0 / 254000.0;
/// Points per coordinate unit before version 6 (1/1000 inch).
const PT_PER_UNIT16: f64 = 72.0 / 1000.0;

/// Upper bounds that keep hostile counts from allocating.
const MAX_FRAMES: usize = 4096;
const MAX_PARAGRAPHS: usize = 65536;
const MAX_RUNS: usize = 65536;
const MAX_CHARS: usize = 4_000_000;

/// Windows-1252 code points for bytes 0x80..0x9f (0 = undefined, mapped to
/// U+FFFD). Bytes below 0x80 and from 0xa0 are Latin-1.
const CP1252_HIGH: [u16; 32] = [
    0x20ac, 0, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
    0x0152, 0, 0x017d, 0, 0, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x02dc,
    0x2122, 0x0161, 0x203a, 0x0153, 0, 0x017e, 0x0178,
];

/// Decode single-byte text as Windows-1252, stopping at the first NUL.
pub(crate) fn decode_cp1252(b: &[u8]) -> String {
    b.iter()
        .take_while(|c| **c != 0)
        .map(|&c| {
            if (0x80..0xa0).contains(&c) {
                match CP1252_HIGH[(c - 0x80) as usize] {
                    0 => '\u{fffd}',
                    u => char::from_u32(u as u32).unwrap_or('\u{fffd}'),
                }
            } else {
                c as char
            }
        })
        .collect()
}

/// Decode UTF-16LE text, stopping at the first NUL unit.
pub(crate) fn decode_utf16le(b: &[u8]) -> String {
    let units: Vec<u16> = b
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|u| *u != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

/// Text in the version's string encoding: UTF-16LE from version 12,
/// single-byte (system code page, read as Windows-1252) before.
pub(crate) fn decode_text(b: &[u8], v: u16) -> String {
    if v >= 12 {
        decode_utf16le(b)
    } else {
        decode_cp1252(b)
    }
}

/// Decoded `text_style_flags`: the low 18 bits name the face (one bit per
/// weight/italic combination), bits 18..20 underline, 21..23 overline,
/// 24..26 strike-through, 27..28 script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StyleFlags(pub u32);

impl StyleFlags {
    const BOLD_BITS: u32 = 0x400 | 0x800 | 0x1000 | 0x2000 | 0x4000 | 0x8000 | 0x1_0000 | 0x2_0000;
    const ITALIC_BITS: u32 = 0x2 | 0x8 | 0x20 | 0x80 | 0x200 | 0x800 | 0x2000 | 0x8000 | 0x2_0000;

    pub fn bold(self) -> bool {
        self.0 & 0x3ffff & Self::BOLD_BITS != 0
    }
    pub fn italic(self) -> bool {
        self.0 & 0x3ffff & Self::ITALIC_BITS != 0
    }
    pub fn underline(self) -> bool {
        (self.0 >> 18) & 0x7 != 0
    }
    pub fn strikethrough(self) -> bool {
        (self.0 >> 24) & 0x7 != 0
    }
}

/// `font` chunk: id and family name.
pub(crate) struct FontRec {
    pub id: u16,
    pub name: String,
}

/// `font`: id u16, encoding u16, style flags u32, 10 unknown bytes, name
/// to the end of the chunk (UTF-16LE from version 12).
pub(crate) fn read_font(d: &[u8], v: u16) -> Option<FontRec> {
    let mut r = Reader::new(d, false);
    let id = r.u16()?;
    let _encoding = r.u16()?;
    let _flags = r.u32()?;
    r.skip(10);
    let name = decode_text(d.get(r.pos..)?, v);
    let name = name.trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(FontRec { id, name })
}

/// Character and paragraph properties a run or a style may set. `None`
/// means "inherit".
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RunStyle {
    pub font_id: Option<u16>,
    pub family: Option<String>,
    pub size_pt: Option<f64>,
    pub flags: Option<StyleFlags>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub fill_id: Option<u32>,
    pub outl_id: Option<u32>,
    /// Alignment code (0 none, 1 left, 2 right, 3 centre, 4 and 5 justify).
    pub align: Option<u32>,
    /// Line spacing as a factor of the character height (1.0 = 100 %).
    pub line_spacing: Option<f64>,
    /// Extra character spacing as a fraction of the em (0 = none).
    pub char_spacing: Option<f64>,
    /// Paragraph indents in points: (left, first line, right).
    pub indent_pt: Option<(f64, f64, f64)>,
}

impl RunStyle {
    /// `self` on top of `base`: set fields win, unset ones inherit.
    pub fn over(&self, base: &RunStyle) -> RunStyle {
        RunStyle {
            font_id: self.font_id.or(base.font_id),
            family: self.family.clone().or_else(|| base.family.clone()),
            size_pt: self.size_pt.or(base.size_pt),
            flags: self.flags.or(base.flags),
            bold: self.bold.or(base.bold),
            italic: self.italic.or(base.italic),
            fill_id: self.fill_id.or(base.fill_id),
            outl_id: self.outl_id.or(base.outl_id),
            align: self.align.or(base.align),
            line_spacing: self.line_spacing.or(base.line_spacing),
            char_spacing: self.char_spacing.or(base.char_spacing),
            indent_pt: self.indent_pt.or(base.indent_pt),
        }
    }
}

/// One record of the `stlt` table after its indirections are resolved.
#[derive(Debug, Clone, Default)]
pub(crate) struct StyleRec {
    pub parent: u32,
    pub style: RunStyle,
}

/// `stlt`: count, mapping tables (fills, outlines, fonts, alignments,
/// intervals, set5, tabs, bullets, indents, hyphens, drop caps, set11),
/// then the style records. Returns (style id, record).
pub(crate) fn read_stlt(d: &[u8], v: u16, warn: &mut dyn FnMut(String)) -> Vec<(u32, StyleRec)> {
    let v16 = v < 6;
    let coord_size = if v16 { 2 } else { 4 };
    let mut r = Reader::new(d, v16);
    let Some(num_records) = r.u32() else {
        return Vec::new();
    };
    if num_records == 0 {
        return Vec::new();
    }
    // Entry tables: (id, 4 unknown, value) with an optional tail.
    fn read_entries(r: &mut Reader, extra: usize) -> Option<std::collections::HashMap<u32, u32>> {
        let n = r.u32()? as usize;
        let size = 12 + extra;
        let n = n.min(r.remaining() / size);
        let mut m = std::collections::HashMap::with_capacity(n);
        for _ in 0..n {
            let id = r.u32()?;
            r.skip(4);
            let value = r.u32()?;
            r.skip(extra);
            m.insert(id, value);
        }
        Some(m)
    }
    let Some(fills) = read_entries(&mut r, if v >= 13 { 48 } else { 0 }) else {
        return Vec::new();
    };
    let Some(outls) = read_entries(&mut r, 0) else {
        return Vec::new();
    };
    // Fonts: id, unknown 12 (before 10) / 20, font id u16, encoding u16,
    // 8 unknown, size coord, 8 unknown, style flags u32, 8 unknown (10+).
    let font_gap = if v < 10 { 12 } else { 20 };
    let font_size = 4 + font_gap + 2 + 2 + 8 + coord_size + 8 + 4 + if v >= 10 { 8 } else { 0 };
    let Some(nf) = r.u32() else {
        return Vec::new();
    };
    let nf = (nf as usize).min(r.remaining() / font_size);
    let mut fonts: std::collections::HashMap<u32, (u16, f64, StyleFlags)> =
        std::collections::HashMap::with_capacity(nf);
    for _ in 0..nf {
        let (Some(id), _, Some(font_id), Some(_enc), _, Some(size), _, Some(flags)) = (
            r.u32(),
            r.skip(font_gap),
            r.u16(),
            r.u16(),
            r.skip(8),
            r.sint(),
            r.skip(8),
            r.u32(),
        ) else {
            return Vec::new();
        };
        if v >= 10 {
            r.skip(8);
        }
        let pt = size as f64 * if v16 { PT_PER_UNIT16 } else { PT_PER_UNIT };
        fonts.insert(id, (font_id, pt, StyleFlags(flags)));
    }
    let Some(aligns) = read_entries(&mut r, 0) else {
        return Vec::new();
    };
    // Tables we do not use: skip by their fixed record sizes.
    let skip_table = |r: &mut Reader, size: usize| -> Option<()> {
        let n = r.u32()? as usize;
        let n = n.min(r.remaining() / size.max(1));
        r.skip(n * size);
        Some(())
    };
    // Intervals: id, 8 unknown, character spacing u32, 8 unknown, line
    // spacing u32, 24 unknown; both spacings in millionths.
    let Some(ni) = r.u32() else {
        return Vec::new();
    };
    let ni = (ni as usize).min(r.remaining() / 52);
    let mut intervals: std::collections::HashMap<u32, (f64, f64)> =
        std::collections::HashMap::with_capacity(ni);
    for _ in 0..ni {
        let (Some(id), _, Some(ch), _, Some(ln), _) =
            (r.u32(), r.skip(8), r.u32(), r.skip(8), r.u32(), r.skip(24))
        else {
            return Vec::new();
        };
        intervals.insert(id, (ch as f64 / 1_000_000.0, ln as f64 / 1_000_000.0));
    }
    if skip_table(&mut r, 152).is_none() || skip_table(&mut r, 784).is_none() {
        return Vec::new();
    }
    // Bullets have a variable size.
    let Some(nb) = r.u32() else {
        return Vec::new();
    };
    for _ in 0..(nb as usize).min(4096) {
        if r.remaining() == 0 {
            break;
        }
        r.skip(40);
        if v >= 14 {
            r.skip(4);
        }
        if v >= 13 {
            let Some(ind) = r.u32() else { break };
            r.skip(if ind != 0 { 68 } else { 12 });
        } else {
            r.skip(20);
            if v >= 10 {
                r.skip(8);
            }
            let Some(ind) = r.u32() else { break };
            if ind != 0 {
                r.skip(8);
            }
            r.skip(8);
        }
    }
    // Indents: id, 12 unknown, right, first line, left (coordinates).
    let indent_size = 4 + 12 + 3 * coord_size;
    let Some(nind) = r.u32() else {
        return Vec::new();
    };
    let nind = (nind as usize).min(r.remaining() / indent_size);
    let mut indents: std::collections::HashMap<u32, (f64, f64, f64)> =
        std::collections::HashMap::with_capacity(nind);
    let unit_pt = if v16 { PT_PER_UNIT16 } else { PT_PER_UNIT };
    for _ in 0..nind {
        let (Some(id), _, Some(right), Some(first), Some(left)) =
            (r.u32(), r.skip(12), r.sint(), r.sint(), r.sint())
        else {
            return Vec::new();
        };
        indents.insert(
            id,
            (
                left as f64 * unit_pt,
                first as f64 * unit_pt,
                right as f64 * unit_pt,
            ),
        );
    }
    if skip_table(&mut r, 32 + if v >= 13 { 4 } else { 0 }).is_none()
        || skip_table(&mut r, 28).is_none()
    {
        return Vec::new();
    }
    // The set11 table exists from the CDR 8 variant (801) on; our major
    // version cannot tell 800 from 801, so version 8 reads it as absent.
    let has_set11 = v >= 9;
    if has_set11 && skip_table(&mut r, 12).is_none() {
        return Vec::new();
    }

    let mut out = Vec::new();
    for _ in 0..(num_records as usize).min(65536) {
        let (Some(num), Some(style_id), Some(parent), _, Some(len)) =
            (r.u32(), r.u32(), r.u32(), r.skip(8), r.u32())
        else {
            break;
        };
        let bytes = (len as usize).saturating_mul(if v >= 12 { 2 } else { 1 });
        if bytes > r.remaining() {
            warn(format!(
                "stlt record {style_id}: name longer than the chunk"
            ));
            break;
        }
        r.skip(bytes);
        let (Some(fill_ref), Some(outl_ref)) = (r.u32(), r.u32()) else {
            break;
        };
        let mut style = RunStyle {
            fill_id: fills.get(&fill_ref).copied(),
            outl_id: outls.get(&outl_ref).copied(),
            ..RunStyle::default()
        };
        if num > 1 {
            let (Some(font_ref), Some(align_ref), Some(interval_ref), Some(_set5)) =
                (r.u32(), r.u32(), r.u32(), r.u32())
            else {
                break;
            };
            if let Some((ch, ln)) = intervals.get(&interval_ref) {
                if *ln > 0.0 && ln.is_finite() {
                    style.line_spacing = Some(*ln);
                }
                if ch.is_finite() && ch.abs() <= 20.0 {
                    style.char_spacing = Some(*ch);
                }
            }
            if has_set11 {
                r.skip(4);
            }
            if let Some((font_id, pt, flags)) = fonts.get(&font_ref) {
                style.font_id = Some(*font_id);
                if *pt > 0.0 && pt.is_finite() {
                    style.size_pt = Some(*pt);
                }
                style.flags = Some(*flags);
            }
            style.align = aligns.get(&align_ref).copied();
        }
        if num > 2 {
            // tab, bullet, indent, hyphen and drop cap ids.
            let (Some(_tab), Some(_bullet), Some(indent_ref), Some(_hyphen), Some(_drop)) =
                (r.u32(), r.u32(), r.u32(), r.u32(), r.u32())
            else {
                break;
            };
            if let Some(ind) = indents.get(&indent_ref) {
                if ind.0.is_finite() && ind.1.is_finite() && ind.2.is_finite() {
                    style.indent_pt = Some(*ind);
                }
            }
        }
        out.push((style_id, StyleRec { parent, style }));
    }
    out
}

/// One styled run of text.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextRun {
    pub text: String,
    pub style: RunStyle,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TextParagraph {
    pub style_id: u32,
    /// Paragraph-wide properties (version 16+ style strings); runs sit on top.
    pub base: RunStyle,
    pub runs: Vec<TextRun>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Txsm {
    /// Paragraph text (frame) rather than artistic text.
    pub frame: bool,
    /// At least one frame is fitted to a path (the path itself is a
    /// sibling object in the same group).
    pub on_path: bool,
    pub paragraphs: Vec<TextParagraph>,
}

/// `txsm`: two layouts, versions 7 to version 15 and version 16+. Versions before 7 are
/// not described and yield `None`.
pub(crate) fn read_txsm(d: &[u8], v: u16, warn: &mut dyn FnMut(String)) -> Option<Txsm> {
    if v < 7 {
        warn(format!(
            "txsm layout of version {v} is not described; text skipped"
        ));
        return None;
    }
    if v >= 16 {
        read_txsm_16(d, v, warn)
    } else {
        read_txsm_7(d, v, warn)
    }
}

fn coord_pt(raw: i32) -> f64 {
    raw as f64 * PT_PER_UNIT
}

fn read_txsm_7(d: &[u8], v: u16, warn: &mut dyn FnMut(String)) -> Option<Txsm> {
    let mut r = Reader::new(d, false);
    let frame = r.u32()? != 0;
    r.skip(32);
    if v >= 15 {
        r.skip(1);
    }
    let mut any_on_path = false;
    if v < 8 {
        let on_path = r.u32()?;
        if on_path != 0 {
            any_on_path = true;
            r.skip(32);
        }
    }
    let nf = r.u32()? as usize;
    if nf > MAX_FRAMES {
        return None;
    }
    for _ in 0..nf {
        let _frame_id = r.u32()?;
        r.skip(48);
        if v >= 8 {
            let on_path = r.u32()?;
            if on_path != 0 {
                any_on_path = true;
                r.skip(4);
                if v >= 13 {
                    r.skip(8);
                }
                r.skip(28);
                if v >= 15 {
                    r.skip(8);
                }
            } else if v >= 15 {
                r.skip(8);
            }
        }
        if !frame {
            // The 34-byte size belongs to the CDR 8 variant (801), which our
            // major version cannot tell from 800 (32 bytes).
            r.skip(if v >= 15 {
                40
            } else if v >= 14 {
                36
            } else if v > 8 {
                34
            } else if v == 8 {
                32
            } else {
                36
            });
        } else if v >= 15 {
            r.skip(4);
        }
    }
    let np = r.u32()? as usize;
    if np > MAX_PARAGRAPHS {
        return None;
    }
    let mut out = Txsm {
        frame,
        on_path: any_on_path,
        paragraphs: Vec::new(),
    };
    for _ in 0..np {
        let style_id = r.u32()?;
        r.skip(1);
        if v >= 13 && frame {
            r.skip(1);
        }
        let ns = r.u32()? as usize;
        if ns > MAX_RUNS || ns > r.remaining() / 3 {
            return finish(out, warn, "style count");
        }
        let mut styles: Vec<(usize, RunStyle)> = Vec::with_capacity(ns);
        for _ in 0..ns {
            let n = r.u16()? as usize;
            let flags = r.u8()?;
            let fl3 = if v >= 8 { r.u8()? } else { 0 };
            let mut s = RunStyle::default();
            if flags & 0x01 != 0 {
                s.font_id = Some(r.u16()?);
                let _encoding = r.u16()?;
            }
            if flags & 0x02 != 0 {
                s.flags = Some(StyleFlags(r.u32()?));
            }
            if flags & 0x04 != 0 {
                let pt = coord_pt(r.i32()?);
                if pt > 0.0 {
                    s.size_pt = Some(pt);
                }
            }
            if flags & 0x08 != 0 {
                r.skip(4);
            }
            if flags & 0x10 != 0 {
                r.skip(4);
            }
            if flags & 0x20 != 0 {
                r.skip(4);
            }
            if flags & 0x40 != 0 {
                s.fill_id = Some(r.u32()?);
                if v >= 13 {
                    r.skip(48);
                }
            }
            if flags & 0x80 != 0 {
                s.outl_id = Some(r.u32()?);
            }
            if fl3 & 0x02 != 0 {
                let len = r.u32()? as usize;
                r.skip(len.saturating_mul(2));
            }
            if fl3 & 0x08 != 0 {
                if v < 13 {
                    r.skip(4);
                } else {
                    let len = r.u32()? as usize;
                    r.skip(len.saturating_mul(2));
                }
            }
            if fl3 & 0x20 != 0 {
                // A flag byte is peeked; when set, a block (including the
                // byte) follows.
                let flag = *d.get(r.pos)?;
                if flag != 0 {
                    r.skip(if v >= 15 { 52 } else { 4 });
                }
            }
            styles.push((n, s));
        }
        let nc = r.u32()? as usize;
        if nc > MAX_CHARS {
            return finish(out, warn, "character count");
        }
        r.skip(nc.saturating_mul(if v >= 12 { 8 } else { 4 }));
        let nbytes = if v >= 12 { r.u32()? as usize } else { nc };
        let text = d.get(r.pos..r.pos.checked_add(nbytes)?)?;
        r.skip(nbytes);
        let has_path = r.u8()?;
        if has_path != 0 {
            r.skip(nc.saturating_mul(24));
        }
        let text = decode_text(text, v);
        out.paragraphs.push(TextParagraph {
            style_id,
            base: RunStyle::default(),
            runs: split_runs(&text, &styles),
        });
    }
    Some(out)
}

/// Return what was read so far when a count is implausible.
fn finish(out: Txsm, warn: &mut dyn FnMut(String), what: &str) -> Option<Txsm> {
    warn(format!("txsm: implausible {what}; text truncated"));
    if out.paragraphs.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Cut `text` into runs of `n` characters per style; a shortfall extends
/// the last run, and a surplus becomes a run with no properties.
fn split_runs(text: &str, styles: &[(usize, RunStyle)]) -> Vec<TextRun> {
    let chars: Vec<char> = text.chars().collect();
    let mut runs = Vec::new();
    let mut pos = 0usize;
    for (i, (n, s)) in styles.iter().enumerate() {
        let last = i + 1 == styles.len();
        let end = if last {
            chars.len()
        } else {
            (pos + n).min(chars.len())
        };
        if end > pos || last {
            runs.push(TextRun {
                text: chars[pos..end].iter().collect(),
                style: s.clone(),
            });
        }
        pos = end;
    }
    if pos < chars.len() {
        runs.push(TextRun {
            text: chars[pos..].iter().collect(),
            style: RunStyle::default(),
        });
    }
    if runs.is_empty() {
        runs.push(TextRun {
            text: text.to_string(),
            style: RunStyle::default(),
        });
    }
    runs
}

fn read_style_string(r: &mut Reader, d: &[u8], v: u16) -> Option<String> {
    let len = r.u32()? as usize;
    let bytes = if v < 17 { len.saturating_mul(2) } else { len };
    let s = d.get(r.pos..r.pos.checked_add(bytes)?)?;
    r.skip(bytes);
    Some(if v < 17 {
        decode_utf16le(s)
    } else {
        decode_cp1252(s)
    })
}

fn read_txsm_16(d: &[u8], v: u16, warn: &mut dyn FnMut(String)) -> Option<Txsm> {
    let mut r = Reader::new(d, false);
    let frame = r.u32()? != 0;
    r.skip(32);
    let layout = r.u16()?;
    r.skip(3);
    let nf = r.u32()? as usize;
    if nf > MAX_FRAMES {
        return None;
    }
    let mut any_on_path = false;
    for _ in 0..nf {
        let _frame_id = r.u32()?;
        r.skip(48);
        let on_path = r.u32()?;
        if on_path != 0 {
            any_on_path = true;
            r.skip(40);
        }
        r.skip(8);
        if !frame {
            r.skip(16);
            let len = r.u32()? as usize;
            r.skip(if v >= 17 { len } else { len.saturating_mul(2) });
        }
    }
    let np = r.u32()? as usize;
    if np > MAX_PARAGRAPHS {
        return None;
    }
    let mut out = Txsm {
        frame,
        on_path: any_on_path,
        paragraphs: Vec::new(),
    };
    for _ in 0..np {
        let style_id = r.u32()?;
        r.skip(1);
        let flag = if frame { r.u8()? } else { 0 };
        if flag == 1 {
            r.skip(64);
        }
        let mut base = RunStyle::default();
        if layout < 1700 && !frame {
            let s = read_style_string(&mut r, d, v)?;
            base = parse_style_string(&s);
        }
        let default = parse_style_string(&read_style_string(&mut r, d, v)?).over(&base);
        let nr = r.u32()? as usize;
        if nr > MAX_RUNS || nr > r.remaining() / 6 {
            return finish(out, warn, "style record count");
        }
        let mut records: Vec<RunStyle> = Vec::with_capacity(nr);
        for _ in 0..nr {
            let _st1 = r.u16()?;
            let st2 = r.u16()?;
            let st3 = r.u16()?;
            if st2 == 0x3fff && st3 & 0x11 == 0x11 {
                let len = r.u32()? as usize;
                r.skip(if v >= 17 { len } else { len.saturating_mul(2) });
            }
            if st3 & 0x04 != 0 {
                let len = r.u32()? as usize;
                r.skip(len.saturating_mul(2));
            }
            let style = if st2 != 0 || st3 & 0x04 != 0 {
                parse_style_string(&read_style_string(&mut r, d, v)?)
            } else {
                RunStyle::default()
            };
            records.push(style);
        }
        let nc = r.u32()? as usize;
        if nc > MAX_CHARS || nc > r.remaining() / 8 {
            return finish(out, warn, "character count");
        }
        let mut overrides = Vec::with_capacity(nc);
        for _ in 0..nc {
            let _flags = r.u16()?;
            let idx = r.u8()? >> 1;
            r.skip(5);
            overrides.push(idx as usize);
        }
        let nbytes = r.u32()? as usize;
        let text = d.get(r.pos..r.pos.checked_add(nbytes)?)?;
        r.skip(nbytes);
        let has_path = r.u8()?;
        if has_path != 0 {
            r.skip(nc.saturating_mul(24));
        }
        // From version 17 the text is stored one byte per character
        // (UTF-8 when valid, else Windows-1252); earlier version 16+ files use UTF-16LE.
        // Confirmed with a version 21 file: six letters stored as 6 bytes for 6 chars.
        let text = if v >= 17 && nbytes == nc {
            match std::str::from_utf8(text) {
                Ok(s) => s.to_string(),
                Err(_) => decode_cp1252(text),
            }
        } else if v >= 17 && nbytes != nc * 2 {
            String::from_utf8_lossy(text).into_owned()
        } else {
            decode_utf16le(text)
        };
        // Group consecutive characters sharing a style record.
        let mut runs: Vec<TextRun> = Vec::new();
        for (i, ch) in text.chars().enumerate() {
            let idx = overrides.get(i).copied().unwrap_or(usize::MAX);
            let style = match records.get(idx) {
                Some(s) => s.over(&default),
                None => default.clone(),
            };
            match runs.last_mut() {
                Some(last) if last.style == style => last.text.push(ch),
                _ => runs.push(TextRun {
                    text: ch.to_string(),
                    style,
                }),
            }
        }
        if runs.is_empty() {
            runs.push(TextRun {
                text: String::new(),
                style: default.clone(),
            });
        }
        out.paragraphs.push(TextParagraph {
            style_id,
            base: default,
            runs,
        });
    }
    Some(out)
}

/// Minimal JSON value for the version 16+ style strings.
#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            Json::Str(s) => s.trim().parse().ok(),
            Json::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }
}

/// Lenient recursive-descent JSON parser (depth and size bounded).
fn parse_json(s: &str) -> Option<Json> {
    let chars: Vec<char> = s.chars().collect();
    let mut pos = 0usize;
    let v = json_value(&chars, &mut pos, 0)?;
    Some(v)
}

fn json_ws(c: &[char], pos: &mut usize) {
    while *pos < c.len() && c[*pos].is_whitespace() {
        *pos += 1;
    }
}

fn json_value(c: &[char], pos: &mut usize, depth: usize) -> Option<Json> {
    if depth > 32 {
        return None;
    }
    json_ws(c, pos);
    match *c.get(*pos)? {
        '{' => {
            *pos += 1;
            let mut items = Vec::new();
            loop {
                json_ws(c, pos);
                match c.get(*pos)? {
                    '}' => {
                        *pos += 1;
                        break;
                    }
                    ',' => {
                        *pos += 1;
                    }
                    _ => {
                        let key = match json_value(c, pos, depth + 1)? {
                            Json::Str(k) => k,
                            Json::Num(n) => n.to_string(),
                            _ => return None,
                        };
                        json_ws(c, pos);
                        if *c.get(*pos)? != ':' {
                            return None;
                        }
                        *pos += 1;
                        let v = json_value(c, pos, depth + 1)?;
                        if items.len() < 1024 {
                            items.push((key, v));
                        }
                    }
                }
            }
            Some(Json::Obj(items))
        }
        '[' => {
            *pos += 1;
            let mut items = Vec::new();
            loop {
                json_ws(c, pos);
                match c.get(*pos)? {
                    ']' => {
                        *pos += 1;
                        break;
                    }
                    ',' => {
                        *pos += 1;
                    }
                    _ => {
                        let v = json_value(c, pos, depth + 1)?;
                        if items.len() < 1024 {
                            items.push(v);
                        }
                    }
                }
            }
            Some(Json::Arr(items))
        }
        '"' | '\'' => {
            let quote = c[*pos];
            *pos += 1;
            let mut s = String::new();
            loop {
                let ch = *c.get(*pos)?;
                *pos += 1;
                match ch {
                    '\\' => {
                        let e = *c.get(*pos)?;
                        *pos += 1;
                        match e {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            'r' => s.push('\r'),
                            'u' => {
                                let hex: String = c.get(*pos..*pos + 4)?.iter().collect();
                                *pos += 4;
                                let n = u32::from_str_radix(&hex, 16).ok()?;
                                s.push(char::from_u32(n).unwrap_or('\u{fffd}'));
                            }
                            other => s.push(other),
                        }
                    }
                    q if q == quote => break,
                    other => s.push(other),
                }
            }
            Some(Json::Str(s))
        }
        't' | 'f' | 'n' => {
            let rest: String = c[*pos..].iter().take(5).collect();
            if rest.starts_with("true") {
                *pos += 4;
                Some(Json::Bool(true))
            } else if rest.starts_with("false") {
                *pos += 5;
                Some(Json::Bool(false))
            } else if rest.starts_with("null") {
                *pos += 4;
                Some(Json::Null)
            } else {
                None
            }
        }
        _ => {
            let start = *pos;
            while *pos < c.len()
                && (c[*pos].is_ascii_digit() || matches!(c[*pos], '-' | '+' | '.' | 'e' | 'E'))
            {
                *pos += 1;
            }
            if start == *pos {
                return None;
            }
            let s: String = c[start..*pos].iter().collect();
            s.parse().ok().map(Json::Num)
        }
    }
}

/// Font size as stored in a style string: values above 1000 are taken as
/// coordinate units, smaller ones as points (assumed).
fn style_size_pt(n: f64) -> Option<f64> {
    if !n.is_finite() || n <= 0.0 {
        return None;
    }
    Some(if n > 1000.0 { n * PT_PER_UNIT } else { n })
}

/// Apply one (key, value) pair of a style string to `s`, matching keys
/// loosely so both the JSON and the `key:value;` spellings work.
fn apply_style_key(s: &mut RunStyle, key: &str, value: &Json) {
    let k = key.to_ascii_lowercase();
    if k.contains("size") {
        if let Some(pt) = value.num().and_then(style_size_pt) {
            s.size_pt = Some(pt);
        }
    } else if k.contains("weight") || k.contains("bold") {
        match value {
            Json::Str(w) => {
                let w = w.to_ascii_lowercase();
                s.bold = Some(w.contains("bold") || w.contains("heavy") || w.contains("black"));
            }
            other => {
                if let Some(n) = other.num() {
                    s.bold = Some(n >= 600.0 || (n > 0.0 && n <= 1.0));
                }
            }
        }
    } else if k.contains("italic") || k == "style" || k.contains("font-style") {
        match value {
            Json::Str(w) => {
                let w = w.to_ascii_lowercase();
                if w.contains("italic") || w.contains("oblique") {
                    s.italic = Some(true);
                } else if k.contains("italic") {
                    s.italic = Some(false);
                }
            }
            other => {
                if let Some(n) = other.num() {
                    if k.contains("italic") {
                        s.italic = Some(n != 0.0);
                    } else if n >= 0.0 && n <= u32::MAX as f64 {
                        s.flags = Some(StyleFlags(n as u32));
                    }
                }
            }
        }
    } else if k.contains("fill") {
        if let Some(n) = value.num() {
            if n >= 0.0 && n <= u32::MAX as f64 {
                s.fill_id = Some(n as u32);
            }
        }
    } else if k.contains("outl") || (k.contains("line") && !k.contains("under")) {
        if let Some(n) = value.num() {
            if n >= 0.0 && n <= u32::MAX as f64 {
                s.outl_id = Some(n as u32);
            }
        }
    } else if k.contains("justif") || k.contains("align") {
        match value {
            Json::Str(a) => {
                let a = a.to_ascii_lowercase();
                s.align = Some(if a.contains("cent") {
                    3
                } else if a.contains("right") {
                    2
                } else if a.contains("just") || a.contains("full") {
                    4
                } else {
                    1
                });
            }
            other => {
                if let Some(n) = other.num() {
                    if (0.0..=5.0).contains(&n) {
                        s.align = Some(n as u32);
                    }
                }
            }
        }
    } else if k.contains("font") || k.contains("family") || k.contains("face") {
        match value {
            Json::Num(n) if *n >= 0.0 && *n <= u16::MAX as f64 => s.font_id = Some(*n as u16),
            Json::Str(name) if !name.trim().is_empty() => {
                if let Ok(n) = name.trim().parse::<u16>() {
                    s.font_id = Some(n);
                } else {
                    s.family = Some(name.trim().to_string());
                }
            }
            _ => {}
        }
    }
}

fn apply_style_object(s: &mut RunStyle, obj: &Json, depth: usize) {
    if depth > 8 {
        return;
    }
    if let Json::Obj(items) = obj {
        for (k, v) in items {
            match v {
                Json::Obj(_) => apply_style_object(s, v, depth + 1),
                _ => apply_style_key(s, k, v),
            }
        }
    }
}

/// version 16+ style string: a JSON object with `character` and `paragraph`
/// sections (font, size, fill, outline, justify), or a `key:value;` list.
/// Unknown keys are ignored.
pub(crate) fn parse_style_string(s: &str) -> RunStyle {
    let mut out = RunStyle::default();
    let t = s.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    if t.is_empty() {
        return out;
    }
    if t.starts_with('{') {
        if let Some(json) = parse_json(t) {
            apply_style_object(&mut out, &json, 0);
            return out;
        }
    }
    for item in t.split(';') {
        let mut kv = item.splitn(2, [':', '=']);
        let (Some(k), Some(v)) = (kv.next(), kv.next()) else {
            continue;
        };
        let v = v.trim().trim_matches('"');
        let value = match v.parse::<f64>() {
            Ok(n) => Json::Num(n),
            Err(_) => Json::Str(v.to_string()),
        };
        apply_style_key(&mut out, k.trim(), &value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cp1252_maps_the_high_half() {
        assert_eq!(
            decode_cp1252(&[0x41, 0x80, 0x93, 0xe9, 0x00, 0x42]),
            "A\u{20ac}\u{201c}é"
        );
    }

    #[test]
    fn style_flags_bold_italic_underline() {
        assert!(StyleFlags(0x1000).bold());
        assert!(!StyleFlags(0x1000).italic());
        assert!(StyleFlags(0x2000).bold() && StyleFlags(0x2000).italic());
        assert!(StyleFlags(0x80).italic() && !StyleFlags(0x80).bold());
        assert!(!StyleFlags(0x40).bold());
        assert!(StyleFlags(1 << 18).underline());
        assert!(StyleFlags(1 << 24).strikethrough());
        assert!(!StyleFlags(0x40).underline());
    }

    #[test]
    fn style_string_json_and_key_value_forms() {
        let s = parse_style_string(
            r#"{"character":{"font":7,"size":304800,"fill":12},"paragraph":{"justify":3}}"#,
        );
        assert_eq!(s.font_id, Some(7));
        assert!((s.size_pt.unwrap() - 86.4).abs() < 1e-6);
        assert_eq!(s.fill_id, Some(12));
        assert_eq!(s.align, Some(3));
        let s = parse_style_string("font-family:Times New Roman;font-size:12;font-weight:bold");
        assert_eq!(s.family.as_deref(), Some("Times New Roman"));
        assert_eq!(s.size_pt, Some(12.0));
        assert_eq!(s.bold, Some(true));
        assert_eq!(parse_style_string("").size_pt, None);
        assert_eq!(parse_style_string("{broken").size_pt, None);
    }

    #[test]
    fn split_runs_covers_the_whole_text() {
        let styles = vec![
            (2usize, RunStyle::default()),
            (
                1usize,
                RunStyle {
                    bold: Some(true),
                    ..RunStyle::default()
                },
            ),
        ];
        let runs = split_runs("abcd", &styles);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "ab");
        assert_eq!(runs[1].text, "cd");
        assert_eq!(split_runs("xyz", &[]).len(), 1);
    }
}
