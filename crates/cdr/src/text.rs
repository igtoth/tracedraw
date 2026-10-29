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

/// Decode version 17+ text whose characters are one byte each (Windows-1252)
/// except those flagged `wide`, which are a UTF-16LE unit of two bytes:
/// letters beyond Latin-1 such as "č" (0D 01) or "ř" (59 01). `None` when
/// the widths do not add up to the bytes stored.
pub(crate) fn decode_mixed_width(b: &[u8], wide: &[bool]) -> Option<String> {
    let needed: usize = wide.iter().map(|w| if *w { 2 } else { 1 }).sum();
    if needed != b.len() {
        return None;
    }
    let mut units: Vec<u16> = Vec::with_capacity(wide.len());
    let mut pos = 0;
    for w in wide {
        if *w {
            units.push(u16::from_le_bytes([*b.get(pos)?, *b.get(pos + 1)?]));
            pos += 2;
        } else {
            let c = *b.get(pos)?;
            let u = if (0x80..0xa0).contains(&c) {
                match CP1252_HIGH[(c - 0x80) as usize] {
                    0 => 0xfffd,
                    u => u,
                }
            } else {
                c as u16
            };
            units.push(u);
            pos += 1;
        }
    }
    Some(String::from_utf16_lossy(&units))
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

/// The code page a font record's character set names (the Windows
/// character set values): Latin (0, and the default, OEM and unknown
/// sets) reads as Windows-1252.
fn charset_encoding(charset: u8) -> Option<&'static encoding_rs::Encoding> {
    use encoding_rs::*;
    Some(match charset {
        128 => SHIFT_JIS,
        129 => EUC_KR,
        134 => GBK,
        136 => BIG5,
        161 => WINDOWS_1253,
        162 => WINDOWS_1254,
        163 => WINDOWS_1258,
        177 => WINDOWS_1255,
        178 => WINDOWS_1256,
        186 => WINDOWS_1257,
        204 => WINDOWS_1251,
        222 => WINDOWS_874,
        238 => WINDOWS_1250,
        _ => return None,
    })
}

/// Single-byte (or double-byte) text in the code page of the character set
/// `charset`, stopping at the first NUL. Symbol fonts (2) keep the byte
/// values, which symbol fonts map to their glyphs.
pub(crate) fn decode_charset(b: &[u8], charset: u8) -> String {
    let b = &b[..b.iter().position(|c| *c == 0).unwrap_or(b.len())];
    if charset == 2 {
        return b.iter().map(|&c| c as char).collect();
    }
    match charset_encoding(charset) {
        Some(enc) => enc.decode_without_bom_handling(b).0.into_owned(),
        None => decode_cp1252(b),
    }
}

/// The character set is one whose text differs from Windows-1252.
pub(crate) fn is_non_latin_charset(charset: u8) -> bool {
    charset == 2 || charset_encoding(charset).is_some()
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
    /// The weight of the face the flags name (100 thin .. 900 heavy);
    /// `None` when no face bit is set.
    pub fn weight(self) -> Option<u16> {
        let face = self.0 & 0x3ffff;
        (face != 0).then(|| (face.trailing_zeros() as u16 / 2 + 1) * 100)
    }
    pub fn underline(self) -> bool {
        (self.0 >> 18) & 0x7 != 0
    }
    pub fn strikethrough(self) -> bool {
        (self.0 >> 24) & 0x7 != 0
    }
}

/// `font` chunk: id, family name and character set.
pub(crate) struct FontRec {
    pub id: u16,
    pub name: String,
    /// The Windows character set of the font's text (the low byte of the
    /// encoding field): 0 Latin, 204 Cyrillic, 238 Central European ...
    pub charset: u8,
}

/// `font`: id u16, encoding u16, style flags u32, 10 unknown bytes, name
/// to the end of the chunk (UTF-16LE from version 12, before that in the
/// code page of the encoding's character set).
pub(crate) fn read_font(d: &[u8], v: u16) -> Option<FontRec> {
    let mut r = Reader::new(d, false);
    let id = r.u16()?;
    let charset = (r.u16()? & 0xff) as u8;
    let _flags = r.u32()?;
    r.skip(10);
    let raw = d.get(r.pos..)?;
    let name = if v >= 12 {
        decode_utf16le(raw)
    } else {
        decode_charset(raw, charset)
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(FontRec { id, name, charset })
}

/// Character and paragraph properties a run or a style may set. `None`
/// means "inherit".
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RunStyle {
    pub font_id: Option<u16>,
    /// The character set of the text (before version 12; see
    /// [`decode_charset`]).
    pub charset: Option<u8>,
    pub family: Option<String>,
    pub size_pt: Option<f64>,
    pub flags: Option<StyleFlags>,
    pub bold: Option<bool>,
    /// The face's weight (400 regular, 600 semibold, 700 bold), set with
    /// `bold` when the file gives it.
    pub weight: Option<u16>,
    pub italic: Option<bool>,
    pub fill_id: Option<u32>,
    pub outl_id: Option<u32>,
    /// Alignment code (0 none, 1 left, 2 centre, 3 right, 4 and 5 justify).
    pub align: Option<u32>,
    /// Line spacing as a factor of the character height (1.0 = 100 %).
    pub line_spacing: Option<f64>,
    /// Line spacing as a distance between baselines, mm (set instead of
    /// `line_spacing`).
    pub line_spacing_mm: Option<f64>,
    /// Extra character spacing as a fraction of the em (0 = none).
    pub char_spacing: Option<f64>,
    /// Paragraph indents in points: (left, first line, right).
    pub indent_pt: Option<(f64, f64, f64)>,
    /// Version 16+ style strings: the space before and after a paragraph
    /// as factors of the character height (`beforeParaSpacing` 1.0 is no
    /// extra space, `afterParaSpacing` 0.0 none), and the unit they and
    /// the line spacing are given in (`spacingMethod`; only 0, the
    /// character height, is read).
    pub para_before: Option<f64>,
    pub para_after: Option<f64>,
    pub spacing_method: Option<u32>,
    /// An inline uniform fill colour (version 16 and later style strings
    /// describe the run's fill as an object with its colour);
    /// `Some(None)` is no fill.
    pub fill_color: Option<Option<tracedraw_core::Color>>,
    /// Version 16+ style strings: underline and strikeout on or off.
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    /// Versions 7 to 15: a character's own placement, as (angle in
    /// degrees counter-clockwise, horizontal shift and vertical shift in
    /// percent of the font size, up positive).
    pub char_shift: Option<CharShift>,
    /// The font the style gives East Asian letters (the second script
    /// slot of a style table font entry, the `farEast` section of a
    /// version 16+ style string), when it is not the Latin one.
    pub asian: Option<ScriptFont>,
}

/// The font settings of one script in a style: the style table's font
/// entries hold three (Latin, East Asian, other), the version 16+ style
/// strings `latin`, `farEast` and `RTL` sections.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ScriptFont {
    pub font_id: Option<u16>,
    pub charset: Option<u8>,
    pub family: Option<String>,
    pub size_pt: Option<f64>,
    pub flags: Option<StyleFlags>,
    pub bold: Option<bool>,
    pub weight: Option<u16>,
    pub italic: Option<bool>,
}

impl ScriptFont {
    /// The font settings of `s` as a script's font.
    pub fn of(s: &RunStyle) -> ScriptFont {
        ScriptFont {
            font_id: s.font_id,
            charset: s.charset,
            family: s.family.clone(),
            size_pt: s.size_pt,
            flags: s.flags,
            bold: s.bold,
            weight: s.weight,
            italic: s.italic,
        }
    }

    /// Whether the font differs from the Latin settings of `s` in a way
    /// that changes how letters are drawn.
    pub fn differs_from(&self, s: &RunStyle) -> bool {
        let font_set = self.font_id.is_some() || self.family.is_some();
        (font_set && (self.font_id != s.font_id || self.family != s.family))
            || (self.size_pt.is_some() && self.size_pt != s.size_pt)
            || (self.flags.is_some() && self.flags != s.flags)
            || (self.bold.is_some() && self.bold != s.bold)
            || (self.italic.is_some() && self.italic != s.italic)
    }
}

/// Whether a character is set in the East Asian font of a style: the
/// ideographs, kana, hangul, bopomofo, the CJK symbols and punctuation,
/// the enclosed and compatibility forms and the full-width forms.
pub(crate) fn is_east_asian(c: char) -> bool {
    matches!(
        c as u32,
        0x1100..=0x11FF
            | 0x2E80..=0x2FDF
            | 0x3000..=0x303F
            | 0x3040..=0x30FF
            | 0x3100..=0x312F
            | 0x3130..=0x318F
            | 0x3190..=0x31FF
            | 0x3200..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA960..=0xA97F
            | 0xAC00..=0xD7FF
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFFEF
            | 0x20000..=0x3FFFF
    )
}

/// `text` cut into runs of East Asian letters and of the rest, each with
/// whether it is East Asian. Characters of neither kind (spaces, line
/// breaks, common punctuation) go with the run before them, or with the
/// run after them at the start.
pub(crate) fn split_scripts(text: &str) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut pending = String::new();
    for c in text.chars() {
        let neutral = c.is_whitespace() || c.is_ascii_punctuation() || c.is_control();
        if neutral {
            match out.last_mut() {
                Some(last) => last.0.push(c),
                None => pending.push(c),
            }
            continue;
        }
        let asian = is_east_asian(c);
        match out.last_mut() {
            Some(last) if last.1 == asian => last.0.push(c),
            _ => {
                let mut s = std::mem::take(&mut pending);
                s.push(c);
                out.push((s, asian));
            }
        }
    }
    if !pending.is_empty() {
        out.push((pending, false));
    }
    out
}

/// A character moved and turned on its own (style flags 0x08, 0x10 and
/// 0x20 of a version 7 to 15 text run).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CharShift {
    /// Degrees, counter-clockwise, about the character's origin.
    pub angle_deg: f64,
    /// Percent of the font size, positive to the right.
    pub x_pct: f64,
    /// Percent of the font size, positive up.
    pub y_pct: f64,
}

impl RunStyle {
    /// `self` on top of `base`: set fields win, unset ones inherit.
    pub fn over(&self, base: &RunStyle) -> RunStyle {
        RunStyle {
            font_id: self.font_id.or(base.font_id),
            charset: self.charset.or(base.charset),
            family: self.family.clone().or_else(|| base.family.clone()),
            size_pt: self.size_pt.or(base.size_pt),
            flags: self.flags.or(base.flags),
            bold: self.bold.or(base.bold),
            // The weight belongs to the bold setting it came with.
            weight: if self.bold.is_some() {
                self.weight
            } else {
                base.weight
            },
            italic: self.italic.or(base.italic),
            fill_id: self.fill_id.or(base.fill_id),
            outl_id: self.outl_id.or(base.outl_id),
            align: self.align.or(base.align),
            // The two ways of giving the spacing replace each other.
            line_spacing: if self.line_spacing.is_some() || self.line_spacing_mm.is_some() {
                self.line_spacing
            } else {
                base.line_spacing
            },
            line_spacing_mm: if self.line_spacing.is_some() || self.line_spacing_mm.is_some() {
                self.line_spacing_mm
            } else {
                base.line_spacing_mm
            },
            char_spacing: self.char_spacing.or(base.char_spacing),
            indent_pt: self.indent_pt.or(base.indent_pt),
            para_before: self.para_before.or(base.para_before),
            para_after: self.para_after.or(base.para_after),
            spacing_method: self.spacing_method.or(base.spacing_method),
            fill_color: self.fill_color.or(base.fill_color),
            underline: self.underline.or(base.underline),
            strikethrough: self.strikethrough.or(base.strikethrough),
            char_shift: self.char_shift.or(base.char_shift),
            // A style naming its own font sets it for every script; one
            // that only changes the size or the face keeps the base's
            // East Asian font at that size and face.
            asian: if self.font_id.is_some() || self.family.is_some() {
                self.asian.clone()
            } else {
                self.asian.clone().or_else(|| {
                    base.asian.clone().map(|mut a| {
                        if self.size_pt.is_some() {
                            a.size_pt = self.size_pt;
                        }
                        if self.flags.is_some() {
                            a.flags = self.flags;
                        }
                        if self.bold.is_some() {
                            a.bold = self.bold;
                            a.weight = self.weight;
                        }
                        if self.italic.is_some() {
                            a.italic = self.italic;
                        }
                        a
                    })
                })
            },
        }
    }

    /// The style with its East Asian font in place of the Latin one, for
    /// the letters set in it.
    pub fn for_east_asian(&self) -> RunStyle {
        let Some(a) = &self.asian else {
            return self.clone();
        };
        let mut s = self.clone();
        if a.font_id.is_some() || a.family.is_some() {
            s.font_id = a.font_id;
            s.family = a.family.clone();
            s.charset = a.charset.or(s.charset);
        }
        s.size_pt = a.size_pt.or(s.size_pt);
        s.flags = a.flags.or(s.flags);
        if a.bold.is_some() {
            s.bold = a.bold;
            s.weight = a.weight;
        }
        s.italic = a.italic.or(s.italic);
        s.asian = None;
        s
    }
}

/// One record of the `stlt` table after its indirections are resolved.
#[derive(Debug, Clone, Default)]
pub(crate) struct StyleRec {
    pub parent: u32,
    pub style: RunStyle,
}

/// A style table font entry's Latin and East Asian fonts: font id,
/// character set, size in points and style flags.
struct FontEntry {
    latin: (u16, u8, f64, StyleFlags),
    asian: (u16, u8, f64, StyleFlags),
}

/// A style's line spacing as the interval table gives it.
#[derive(Debug, Clone, Copy)]
enum LineSpacing {
    Factor(f64),
    Mm(f64),
}

/// `stlt`: count, mapping tables (fills, outlines, fonts, alignments,
/// intervals, set5, tabs, bullets, indents, hyphens, drop caps, set11),
/// then the style records. Returns (style id, record).
pub(crate) fn read_stlt(d: &[u8], v: u16, warn: &mut dyn FnMut(String)) -> Vec<(u32, StyleRec)> {
    if v != 8 {
        return read_stlt_set11(d, v, v >= 9, warn);
    }
    // Version 8 has two layouts (800 and the bidirectional 801, which has
    // the set11 table) that the major version cannot tell apart: without
    // it first, and with it when that reads with complaints.
    let mut notes = Vec::new();
    let without = read_stlt_set11(d, v, false, &mut |m| notes.push(m));
    if notes.is_empty() && !without.is_empty() {
        return without;
    }
    let mut notes_with = Vec::new();
    let with = read_stlt_set11(d, v, true, &mut |m| notes_with.push(m));
    let use_with = notes_with.is_empty() && !with.is_empty();
    for m in if use_with { notes_with } else { notes } {
        warn(m);
    }
    if use_with {
        with
    } else {
        without
    }
}

/// [`read_stlt`] with the set11 table present or not.
fn read_stlt_set11(
    d: &[u8],
    v: u16,
    has_set11: bool,
    warn: &mut dyn FnMut(String),
) -> Vec<(u32, StyleRec)> {
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
    // Fonts: id, unknown 12 (before 10) / 20, then three scripts (Latin,
    // East Asian, other): font id u16 and encoding u16 each, a size
    // coord each, and style flags u32 each (one for all before 10). A
    // version 14 file (#955) sets Chinese titles in the East Asian slot
    // (a script font at 54 pt) with Arial at 12 pt in the Latin one.
    let font_gap = if v < 10 { 12 } else { 20 };
    let font_size = 4 + font_gap + 3 * 4 + 3 * coord_size + if v >= 10 { 12 } else { 4 };
    let Some(nf) = r.u32() else {
        return Vec::new();
    };
    let nf = (nf as usize).min(r.remaining() / font_size);
    let mut fonts: std::collections::HashMap<u32, FontEntry> =
        std::collections::HashMap::with_capacity(nf);
    let pt_per_unit = if v16 { PT_PER_UNIT16 } else { PT_PER_UNIT };
    for _ in 0..nf {
        let Some(id) = r.u32() else {
            return Vec::new();
        };
        r.skip(font_gap);
        let mut ids = [(0u16, 0u16); 3];
        for slot in &mut ids {
            let (Some(font_id), Some(enc)) = (r.u16(), r.u16()) else {
                return Vec::new();
            };
            *slot = (font_id, enc);
        }
        let mut sizes = [0.0f64; 3];
        for s in &mut sizes {
            let Some(size) = r.sint() else {
                return Vec::new();
            };
            *s = size as f64 * pt_per_unit;
        }
        let mut flags = [0u32; 3];
        if v >= 10 {
            for f in &mut flags {
                let Some(x) = r.u32() else {
                    return Vec::new();
                };
                *f = x;
            }
        } else {
            let Some(x) = r.u32() else {
                return Vec::new();
            };
            flags = [x; 3];
        }
        let latin = (
            ids[0].0,
            (ids[0].1 & 0xff) as u8,
            sizes[0],
            StyleFlags(flags[0]),
        );
        let asian = (
            ids[1].0,
            (ids[1].1 & 0xff) as u8,
            sizes[1],
            StyleFlags(flags[1]),
        );
        fonts.insert(id, FontEntry { latin, asian });
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
    // Intervals: id, 8 unknown, character spacing i32 (millionths), 8
    // unknown, line spacing, 20 unknown, the line spacing's unit u32: 0
    // (and 2) millionths of the character height, 1 a distance in
    // coordinate units (7 pt stored as 24 694 in a version 12 file whose
    // multi-line text otherwise collapses onto one line).
    let coord_mm = 25.4 / 72.0 * if v16 { PT_PER_UNIT16 } else { PT_PER_UNIT };
    let Some(ni) = r.u32() else {
        return Vec::new();
    };
    let ni = (ni as usize).min(r.remaining() / 52);
    let mut intervals: std::collections::HashMap<u32, (f64, LineSpacing)> =
        std::collections::HashMap::with_capacity(ni);
    for _ in 0..ni {
        let (Some(id), _, Some(ch), _, Some(ln), _, Some(unit)) = (
            r.u32(),
            r.skip(8),
            r.u32(),
            r.skip(8),
            r.u32(),
            r.skip(20),
            r.u32(),
        ) else {
            return Vec::new();
        };
        let line = if unit == 1 {
            LineSpacing::Mm(ln as i32 as f64 * coord_mm)
        } else {
            LineSpacing::Factor(ln as i32 as f64 / 1_000_000.0)
        };
        intervals.insert(id, (ch as i32 as f64 / 1_000_000.0, line));
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
    // The set11 table exists from the CDR 8 variant (801) on (the caller
    // says whether to read it).
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
                match *ln {
                    LineSpacing::Factor(f) if f > 0.0 && f.is_finite() => {
                        style.line_spacing = Some(f);
                    }
                    LineSpacing::Mm(mm) if mm > 0.0 && mm.is_finite() => {
                        style.line_spacing_mm = Some(mm);
                    }
                    _ => {}
                }
                if ch.is_finite() && ch.abs() <= 20.0 {
                    style.char_spacing = Some(*ch);
                }
            }
            if has_set11 {
                r.skip(4);
            }
            if let Some(entry) = fonts.get(&font_ref) {
                let (font_id, charset, pt, flags) = entry.latin;
                style.font_id = Some(font_id);
                style.charset = Some(charset);
                if pt > 0.0 && pt.is_finite() {
                    style.size_pt = Some(pt);
                }
                style.flags = Some(flags);
                // The East Asian slot, when it names another font, size
                // or face than the Latin one.
                let (a_id, a_charset, a_pt, a_flags) = entry.asian;
                if a_id != 0 && (a_id != font_id || a_pt != pt || a_flags != flags) {
                    style.asian = Some(ScriptFont {
                        font_id: Some(a_id),
                        charset: Some(a_charset),
                        size_pt: (a_pt > 0.0 && a_pt.is_finite()).then_some(a_pt),
                        flags: Some(a_flags),
                        ..ScriptFont::default()
                    });
                }
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
    /// Before version 12: the run's bytes, decoded in its font's code page
    /// once its style is resolved (`text` holds them read as
    /// Windows-1252).
    pub raw: Option<Vec<u8>>,
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
    /// At least one frame is fitted to a path (the path itself is a
    /// sibling object in the same group).
    pub on_path: bool,
    /// How the first frame fitted to a path sits on it (version 16+).
    pub path_settings: Option<PathSettings>,
    pub paragraphs: Vec<TextParagraph>,
}

/// How a text fitted to a path sits on it: the 40 bytes after the
/// on-path flag of a version 16+ `txsm` frame (assumed from a version 21
/// file: two texts on one circle, at its top and bottom).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PathSettings {
    /// 4: the middle of the text sits at `offset` (the only value seen).
    pub placement: u32,
    /// Along the baseline, from the path's start, in coordinate units.
    pub offset: i32,
    /// From the path to the baseline, in coordinate units; negative
    /// towards the inside of a closed path.
    pub distance: i32,
    /// Placed on the other side of the path (both flags seen together).
    pub mirror_h: bool,
    pub mirror_v: bool,
}

impl PathSettings {
    fn read(b: &[u8]) -> Option<PathSettings> {
        let word = |i: usize| -> Option<i32> {
            let w = b.get(i * 4..i * 4 + 4)?;
            Some(i32::from_le_bytes([w[0], w[1], w[2], w[3]]))
        };
        Some(PathSettings {
            placement: word(1)? as u32,
            offset: word(4)?,
            distance: word(6)?,
            mirror_h: word(8)? != 0,
            mirror_v: word(9)? != 0,
        })
    }
}

/// `txsm`: two layouts, versions 7 to version 15 and version 16+. Versions before 7 are
/// not described and yield `None`.
pub(crate) fn read_txsm(d: &[u8], v: u16, warn: &mut dyn FnMut(String)) -> Option<Txsm> {
    if v == 6 {
        return read_txsm_6(d);
    }
    if v < 6 {
        return read_txsm_5(d, v, warn);
    }
    if v >= 16 {
        read_txsm_16(d, v, warn)
    } else if v == 8 {
        // Version 8 has two layouts (800 and the bidirectional 801) that
        // the major version cannot tell apart: frames of 32 or 34 bytes.
        // The shorter one first; when its counts are implausible, the
        // longer one.
        let mut notes = Vec::new();
        let short = read_txsm_7_frames(d, v, 32, &mut |m| notes.push(m));
        if short.is_some() && notes.is_empty() {
            return short;
        }
        let mut notes_long = Vec::new();
        let long = read_txsm_7_frames(d, v, 34, &mut |m| notes_long.push(m));
        let use_long = long.is_some() && notes_long.is_empty();
        for m in if use_long { notes_long } else { notes } {
            warn(m);
        }
        if use_long {
            long
        } else {
            short
        }
    } else {
        read_txsm_7(d, v, warn)
    }
}

/// Paragraphs of single-byte characters split at carriage returns (a line
/// feed after one is dropped), all at `size_pt`.
fn early_paragraphs(chars: &[u8], size_pt: Option<f64>) -> Txsm {
    let style = RunStyle {
        size_pt: size_pt.filter(|p| *p > 0.0 && p.is_finite()),
        ..RunStyle::default()
    };
    // Lines end with CR, or CR LF (version 5): the LF is not a character.
    let paragraphs = chars
        .split(|c| *c == b'\r')
        .map(|line| line.strip_prefix(b"\n").unwrap_or(line))
        .map(|line| TextParagraph {
            style_id: 0,
            base: RunStyle::default(),
            runs: split_byte_runs(line, &[(line.len(), style.clone())]),
        })
        .collect();
    Txsm {
        on_path: false,
        path_settings: None,
        paragraphs,
    }
}

/// Version 5 text (one file): the font size at byte 40 (u16, thousandths
/// of an inch), the character count at 50 (u16), then a record of 8 bytes
/// a character: advance, an unknown word, the character (u16, its code
/// page) and the style. Version 4 is read the same way (assumed).
fn read_txsm_5(d: &[u8], v: u16, warn: &mut dyn FnMut(String)) -> Option<Txsm> {
    let word =
        |at: usize| -> Option<u16> { Some(u16::from_le_bytes([*d.get(at)?, *d.get(at + 1)?])) };
    if v < 4 {
        warn(format!(
            "txsm layout of version {v} is not described; text skipped"
        ));
        return None;
    }
    let size = word(40)? as f64 * PT_PER_UNIT16;
    let n = word(50)? as usize;
    if 52 + n * 8 > d.len() {
        warn("version 5 text: character count past the record; text skipped".into());
        return None;
    }
    let chars: Vec<u8> = (0..n)
        .filter_map(|k| word(52 + k * 8 + 4).map(|c| c as u8))
        .collect();
    Some(early_paragraphs(&chars, Some(size)))
}

/// Version 6 text (one file, four texts): the number of 60-byte style
/// blocks at byte 96 (u32), the blocks from 100 (the first holding the
/// font size at 36, u32 in coordinate units), then the character count
/// (u32) and a record of 12 bytes a character: advance, the character
/// (u32, its code page) and flags.
fn read_txsm_6(d: &[u8]) -> Option<Txsm> {
    let dword = |at: usize| -> Option<u32> {
        Some(u32::from_le_bytes([
            *d.get(at)?,
            *d.get(at + 1)?,
            *d.get(at + 2)?,
            *d.get(at + 3)?,
        ]))
    };
    let blocks = dword(96)? as usize;
    if blocks == 0 || blocks > 1024 {
        return None;
    }
    let size = coord_pt(dword(100 + 36)? as i32);
    let at = 100 + 60 * blocks;
    let n = dword(at)? as usize;
    if n > MAX_CHARS || at + 4 + n * 12 > d.len() {
        return None;
    }
    let chars: Vec<u8> = (0..n)
        .filter_map(|k| dword(at + 4 + k * 12 + 4).map(|c| c as u8))
        .collect();
    Some(early_paragraphs(&chars, Some(size)))
}

fn coord_pt(raw: i32) -> f64 {
    raw as f64 * PT_PER_UNIT
}

fn read_txsm_7(d: &[u8], v: u16, warn: &mut dyn FnMut(String)) -> Option<Txsm> {
    read_txsm_7_frames(d, v, 32, warn)
}

/// [`read_txsm_7`] with the size of a version 8 text frame given.
fn read_txsm_7_frames(
    d: &[u8],
    v: u16,
    frame_v8: usize,
    warn: &mut dyn FnMut(String),
) -> Option<Txsm> {
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
            // Version 8: 32 bytes (800) or 34 (the bidirectional 801),
            // chosen by the caller.
            r.skip(if v >= 15 {
                40
            } else if v >= 14 {
                36
            } else if v > 8 {
                34
            } else if v == 8 {
                frame_v8
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
        on_path: any_on_path,
        path_settings: None,
        paragraphs: Vec::new(),
    };
    for _ in 0..np {
        let style_id = r.u32()?;
        r.skip(1);
        // Version 13+ with the frame flag: one more byte; when it is 1,
        // 64 bytes follow (a version 14 file: the frame's id, -1, the
        // frame's matrix and 8 zero bytes, before text whose letters are
        // each moved and turned; #955), as in version 16 on.
        if v >= 13 && frame && r.u8()? == 1 {
            r.skip(64);
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
                s.charset = Some((r.u16()? & 0xff) as u8);
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
            // The character's angle (millionths of a degree) and its
            // horizontal and vertical shifts (hundredths of the font
            // size): a version 14 seal's ring of letters, each turned and
            // moved onto the ring (#955).
            let angle = if flags & 0x08 != 0 {
                Some(r.i32()?)
            } else {
                None
            };
            let dx = if flags & 0x10 != 0 {
                Some(r.i32()?)
            } else {
                None
            };
            let dy = if flags & 0x20 != 0 {
                Some(r.i32()?)
            } else {
                None
            };
            if angle.is_some() || dx.is_some() || dy.is_some() {
                s.char_shift = char_shift(angle, dx, dy);
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
                // A flag byte. Version 15 reads it and, when it is set,
                // a 52-byte block follows it (a version 15 file: the
                // character count comes 1 and 53 bytes after it). Before,
                // the byte is peeked and starts a 4-byte block when set.
                let flag = *d.get(r.pos)?;
                if v >= 15 {
                    r.skip(if flag != 0 { 53 } else { 1 });
                } else if flag != 0 {
                    r.skip(4);
                }
            }
            styles.push((n, s));
        }
        let nc = r.u32()? as usize;
        if nc > MAX_CHARS {
            return finish(out, warn, "character count");
        }
        // One record a character; from version 12 they are 8 bytes and
        // bit 0 of the first marks a character stored in two bytes.
        let mut wide = Vec::new();
        let mut style_of = Vec::new();
        if v >= 12 {
            if nc > r.remaining() / 8 {
                return finish(out, warn, "character count");
            }
            wide.reserve(nc);
            style_of.reserve(nc);
            for _ in 0..nc {
                let first = r.u8()?;
                r.skip(1);
                let style = r.u8()?;
                wide.push(first & 1 != 0);
                style_of.push((style >> 1) as usize);
                r.skip(5);
            }
        } else {
            // Four bytes a character; the third numbers its style as from
            // version 12 (a version 8 file: "x\u{c4}y" in one font with
            // the middle letter, style 1, in a symbol font).
            if nc > r.remaining() / 4 {
                return finish(out, warn, "character count");
            }
            style_of.reserve(nc);
            for _ in 0..nc {
                r.skip(2);
                style_of.push((r.u8()? >> 1) as usize);
                r.skip(1);
            }
        }
        let nbytes = if v >= 12 { r.u32()? as usize } else { nc };
        let text = d.get(r.pos..r.pos.checked_add(nbytes)?)?;
        r.skip(nbytes);
        let has_path = r.u8()?;
        if has_path != 0 {
            r.skip(nc.saturating_mul(24));
        }
        // From version 12 a character is one byte (in its font's code
        // page) unless its record flags it as a UTF-16LE unit of two bytes
        // ("ć" in a Croatian word stored as 07 01 among single-byte
        // Portuguese letters). Text with every character in two bytes and
        // no flags is read as UTF-16.
        // Bits 1 to 7 of the third byte of a character's record (in
        // versions 8 to 11 too) number its style in the list above (a version 15 file: "MASS IN RUNNING ORDER 941 kg",
        // the figures 2, the rest 0 and 1). When they do, they decide the
        // runs; the lengths in the style list do not add up to the text.
        if style_of.iter().any(|i| *i != 0) && style_of.iter().all(|i| *i < styles.len()) {
            styles = runs_by_index(&style_of, &styles);
        }
        let any_wide = wide.iter().any(|w| *w);
        let runs = if v < 12 || (nbytes == nc && !any_wide) {
            split_byte_runs(text, &styles)
        } else if let Some(s) = decode_mixed_width(text, &wide) {
            split_runs(&s, &styles)
        } else if nbytes == nc.saturating_mul(2) {
            split_runs(&decode_utf16le(text), &styles)
        } else {
            warn(format!(
                "text of {nc} characters in {nbytes} bytes; read one byte per character"
            ));
            split_byte_runs(text, &styles)
        };
        out.paragraphs.push(TextParagraph {
            style_id,
            base: RunStyle::default(),
            runs,
        });
    }
    Some(out)
}

/// A character's placement from its raw fields (a shift of 100 is the
/// font size); values out of reason (shifts beyond 100 times the font
/// size) are dropped.
fn char_shift(angle: Option<i32>, dx: Option<i32>, dy: Option<i32>) -> Option<CharShift> {
    const MAX_SHIFT: i32 = 10_000;
    let dx = dx.unwrap_or(0);
    let dy = dy.unwrap_or(0);
    if dx.unsigned_abs() > MAX_SHIFT as u32 || dy.unsigned_abs() > MAX_SHIFT as u32 {
        return None;
    }
    let shift = CharShift {
        // Turned the short way: 347 degrees is -13.
        angle_deg: {
            let a = (f64::from(angle.unwrap_or(0)) / 1_000_000.0).rem_euclid(360.0);
            if a > 180.0 {
                a - 360.0
            } else {
                a
            }
        },
        x_pct: f64::from(dx),
        y_pct: f64::from(dy),
    };
    (shift != CharShift::default()).then_some(shift)
}

/// Runs of consecutive characters sharing a style index, each with the
/// style it names.
fn runs_by_index(style_of: &[usize], styles: &[(usize, RunStyle)]) -> Vec<(usize, RunStyle)> {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for &i in style_of {
        match runs.last_mut() {
            Some((n, last)) if *last == i => *n += 1,
            _ => runs.push((1, i)),
        }
    }
    runs.into_iter()
        .filter_map(|(n, i)| styles.get(i).map(|(_, s)| (n, s.clone())))
        .collect()
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

/// Cut `units` into runs of `n` units per style; a shortfall extends the
/// last run, and a surplus becomes a run with no properties. Never empty.
fn split_by_lengths<'t, T>(
    units: &'t [T],
    styles: &[(usize, RunStyle)],
) -> Vec<(&'t [T], RunStyle)> {
    let mut runs = Vec::new();
    let mut pos = 0usize;
    for (i, (n, s)) in styles.iter().enumerate() {
        let last = i + 1 == styles.len();
        let end = if last {
            units.len()
        } else {
            pos.saturating_add(*n).min(units.len())
        };
        if end > pos || last {
            runs.push((&units[pos..end], s.clone()));
        }
        pos = end;
    }
    if pos < units.len() {
        runs.push((&units[pos..], RunStyle::default()));
    }
    if runs.is_empty() {
        runs.push((units, RunStyle::default()));
    }
    runs
}

/// Cut `text` into runs of `n` characters per style (see
/// [`split_by_lengths`]).
fn split_runs(text: &str, styles: &[(usize, RunStyle)]) -> Vec<TextRun> {
    let chars: Vec<char> = text.chars().collect();
    split_by_lengths(&chars, styles)
        .into_iter()
        .map(|(c, style)| TextRun {
            text: c.iter().collect(),
            style,
            raw: None,
        })
        .collect()
}

/// Cut single-byte `text` (before version 12; it ends at the first NUL)
/// into runs of `n` bytes per style, each keeping its bytes for decoding
/// in its font's code page.
fn split_byte_runs(text: &[u8], styles: &[(usize, RunStyle)]) -> Vec<TextRun> {
    let text = &text[..text.iter().position(|c| *c == 0).unwrap_or(text.len())];
    split_by_lengths(text, styles)
        .into_iter()
        .map(|(b, style)| TextRun {
            text: decode_cp1252(b),
            style,
            raw: Some(b.to_vec()),
        })
        .collect()
}

fn read_style_string(r: &mut Reader, d: &[u8], v: u16) -> Option<String> {
    let len = r.u32()? as usize;
    let bytes = if v < 17 { len.saturating_mul(2) } else { len };
    let s = d.get(r.pos..r.pos.checked_add(bytes)?)?;
    r.skip(bytes);
    Some(if v < 17 {
        decode_utf16le(s)
    } else {
        decode_style_bytes(s)
    })
}

/// A style string of version 17 on, held as bytes: UTF-8, as the object
/// style strings and the strings TraceDraw writes are, so a font named
/// in Chinese ("方正黑体_GBK") reads as named (#898); bytes that are not
/// UTF-8 read as Windows-1252, stopping at the first NUL either way.
pub(crate) fn decode_style_bytes(b: &[u8]) -> String {
    let b = &b[..b.iter().position(|c| *c == 0).unwrap_or(b.len())];
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => decode_cp1252(b),
    }
}

fn read_txsm_16(d: &[u8], v: u16, warn: &mut dyn FnMut(String)) -> Option<Txsm> {
    let mut r = Reader::new(d, false);
    // Not zero: the frame layout of the records below (artistic text has
    // it too in version 21, with all bits set).
    let frame = r.u32()? != 0;
    r.skip(32);
    let layout = r.u16()?;
    r.skip(3);
    let nf = r.u32()? as usize;
    if nf > MAX_FRAMES {
        return None;
    }
    let mut any_on_path = false;
    let mut path_settings = None;
    for _ in 0..nf {
        let _frame_id = r.u32()?;
        r.skip(48);
        let on_path = r.u32()?;
        if on_path != 0 {
            any_on_path = true;
            if path_settings.is_none() {
                path_settings = d.get(r.pos..r.pos + 40).and_then(PathSettings::read);
            }
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
        on_path: any_on_path,
        path_settings,
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
        let mut wide = Vec::with_capacity(nc);
        for _ in 0..nc {
            let flags = r.u16()?;
            let idx = r.u8()? >> 1;
            r.skip(5);
            overrides.push(idx as usize);
            wide.push(flags & 1 != 0);
        }
        let nbytes = r.u32()? as usize;
        let text = d.get(r.pos..r.pos.checked_add(nbytes)?)?;
        r.skip(nbytes);
        let has_path = r.u8()?;
        if has_path != 0 {
            r.skip(nc.saturating_mul(24));
        }
        // From version 17 a character is one byte (Windows-1252) unless
        // its record's flag bit 0 is set: then it is a UTF-16LE unit of
        // two bytes (seen in a version 22 file: "č" stored as 0D 01, "ř" as
        // 59 01, "í" as ED). Version 16 files hold either UTF-16LE (two
        // bytes a character) or the same one-byte text.
        let text = if v >= 17 || nbytes != nc.saturating_mul(2) {
            match decode_mixed_width(text, &wide) {
                Some(s) => s,
                // Files saved by TraceDraw before 0.11.1 held the letters
                // Windows-1252 lacks as UTF-8, with no letter marked
                // (#368): such bytes, UTF-8 with exactly as many letters as
                // records, are read so (before the UTF-16 guess: Cyrillic
                // in UTF-8 is two bytes a letter too).
                None if !wide.iter().any(|w| *w)
                    && !text.is_ascii()
                    && std::str::from_utf8(text).is_ok_and(|t| t.chars().count() == nc) =>
                {
                    String::from_utf8_lossy(text).into_owned()
                }
                None if nbytes == nc * 2 => decode_utf16le(text),
                None => {
                    warn(format!(
                        "text of {nc} characters in {nbytes} bytes; read one byte per character"
                    ));
                    decode_cp1252(text)
                }
            }
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
                    raw: None,
                }),
            }
        }
        if runs.is_empty() {
            runs.push(TextRun {
                text: String::new(),
                style: default.clone(),
                raw: None,
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

/// Text with its line breaks as `\n`: files end lines with a carriage
/// return (`\r`, version 21 artistic text), a CR LF pair, a line or
/// paragraph separator or a vertical tab.
pub(crate) fn line_breaks(text: &str) -> String {
    if !text.contains(['\r', '\u{2028}', '\u{2029}', '\u{0B}']) {
        return text.to_string();
    }
    text.replace("\r\n", "\n")
        .replace(['\r', '\u{2028}', '\u{2029}', '\u{0B}'], "\n")
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
    // Version 16+ style strings write numbers as strings ("1024").
    let number = match value {
        Json::Str(t) => t.trim().parse::<f64>().ok().map(Json::Num),
        _ => None,
    };
    let value = number.as_ref().unwrap_or(value);
    if k == "underline" || k == "strikeout" || k == "strikethrough" {
        if let Some(n) = value.num() {
            if k == "underline" {
                s.underline = Some(n != 0.0);
            } else {
                s.strikethrough = Some(n != 0.0);
            }
        }
        return;
    }
    if k.contains("overline") {
        return;
    }
    if k.contains("weight") {
        // The style code of the style tables: a power of two whose
        // exponent counts weights in steps (64 = 400, 1024 = 600,
        // 4096 = 700; an odd exponent is the italic of the weight below,
        // as the embedded faces of a version 19 file confirm for 64,
        // 1024 and 4096). Other numbers are CSS weights.
        if let Some(n) = value.num() {
            let code = n as u64;
            if n >= 1.0 && n.fract() == 0.0 && code.is_power_of_two() && code < 1 << 18 {
                let f = StyleFlags(code as u32);
                s.bold = Some(f.bold());
                s.weight = f.weight();
                if f.italic() {
                    s.italic = Some(true);
                }
            } else if (100.0..=1000.0).contains(&n) {
                s.bold = Some(n >= 600.0);
                s.weight = Some((n / 100.0).round() as u16 * 100);
            }
            return;
        }
    }
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
                    s.weight = (100.0..=1000.0)
                        .contains(&n)
                        .then(|| (n / 100.0).round() as u16 * 100);
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
    } else if k.contains("outl") || k == "line" {
        if let Some(n) = value.num() {
            if n >= 0.0 && n <= u32::MAX as f64 {
                s.outl_id = Some(n as u32);
            }
        }
    } else if k.contains("justif") || k.contains("align") {
        match value {
            // A word ("center"); numbers written as strings ("2") are codes.
            Json::Str(a) if a.trim().parse::<f64>().is_err() => {
                let a = a.to_ascii_lowercase();
                s.align = Some(if a.contains("cent") {
                    2
                } else if a.contains("right") {
                    3
                } else if a.contains("just") || a.contains("full") {
                    4
                } else {
                    1
                });
            }
            other => {
                // The style table's numbering: 2 centres the lines on one
                // another (seen in version 19 and 21 files); 3 right and
                // 4, 5 justify are assumed; 0 and 1 read as left.
                if let Some(n) = other.num() {
                    if (0.0..=5.0).contains(&n) {
                        s.align = Some(match n as u32 {
                            0 => 1,
                            k => k,
                        });
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
            match (k.as_str(), v) {
                // An inline fill: the run's colour. Its own keys (type,
                // overprint, screens) are not run settings.
                ("fill", Json::Obj(_)) => {
                    if let Some(c) = inline_fill_color(v) {
                        s.fill_color = Some(c);
                    }
                }
                // An inline outline, the frame and the settings for
                // right-to-left scripts are not read; the Latin font
                // settings apply to the text, the East Asian ones to its
                // East Asian letters.
                ("outline" | "frame" | "RTL", Json::Obj(_)) => {}
                ("farEast", Json::Obj(_)) => {
                    let mut f = RunStyle::default();
                    apply_style_object(&mut f, v, depth + 1);
                    if f.font_id.is_some() || f.family.is_some() || f.size_pt.is_some() {
                        s.asian = Some(ScriptFont::of(&f));
                    }
                }
                ("paragraph", Json::Obj(p)) => apply_paragraph_object(s, p),
                (_, Json::Obj(_)) => apply_style_object(s, v, depth + 1),
                _ => apply_style_key(s, k, v),
            }
        }
    }
}

/// The `paragraph` section of a version 16+ style string: line spacing as
/// a millionth of the line height (1000000 = 100 %, assumed from a version
/// 21 file holding the default), the space before and after the paragraph
/// in the same unit and the unit itself (`spacingMethod`), alignment; the
/// underline, overline and
/// strikeout geometry keys are left out (their names would read as
/// outline references).
fn apply_paragraph_object(s: &mut RunStyle, items: &[(String, Json)]) {
    for (k, v) in items {
        let lk = k.to_ascii_lowercase();
        if matches!(v, Json::Obj(_) | Json::Arr(_)) {
            continue;
        }
        if lk == "beforeparaspacing" || lk == "afterparaspacing" {
            if let Some(n) = v.num() {
                let f = n / 1_000_000.0;
                if f.is_finite() && (0.0..100.0).contains(&f) {
                    if lk.starts_with("before") {
                        s.para_before = Some(f);
                    } else {
                        s.para_after = Some(f);
                    }
                }
            }
            continue;
        }
        if lk == "spacingmethod" {
            if let Some(n) = v.num() {
                if (0.0..=16.0).contains(&n) {
                    s.spacing_method = Some(n as u32);
                }
            }
            continue;
        }
        if lk == "interlinespacing" {
            if let Some(n) = v.num() {
                let f = n / 1_000_000.0;
                if f.is_finite() && f > 0.0 && f < 100.0 {
                    s.line_spacing = Some(f);
                }
            }
            continue;
        }
        if lk.contains("underline")
            || lk.contains("overline")
            || lk.contains("strikeout")
            || lk.starts_with("thin")
            || lk.starts_with("thick")
            || lk.starts_with("dbl")
            || lk.starts_with("inter")
            || lk.contains("spacing")
        {
            continue;
        }
        apply_style_key(s, k, v);
    }
}

/// The opacity an object's version 16+ style string gives it: its
/// `transparency` section's `uniformTransparency` is the share of the
/// object that lets the background through (0.3 draws it at 70 %: the
/// colours of a version 19 file's preview match that). Without it, the
/// mean of `startTransparency` and `endTransparency` (a graded
/// transparency drawn uniform, assumed). `None` for an empty section.
/// The mitre limit (degrees) in an object's version 16+ style string:
/// `outline.miterLimit` (a file of version 27 holds "28.955024..." for an
/// outline its program shows with a limit of 29 degrees, #384).
pub(crate) fn object_miter_limit(style: &str) -> Option<f64> {
    let t = style.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    let Json::Obj(items) = parse_json(t)? else {
        return None;
    };
    let Json::Obj(outline) = &items.iter().find(|(k, _)| k == "outline")?.1 else {
        return None;
    };
    outline
        .iter()
        .find(|(k, _)| k == "miterLimit")
        .and_then(|(_, v)| v.num())
        .filter(|a| a.is_finite() && *a > 0.0)
        .map(|a| {
            a.clamp(
                *tracedraw_core::Stroke::MITER_LIMIT_RANGE.start(),
                *tracedraw_core::Stroke::MITER_LIMIT_RANGE.end(),
            )
        })
}

pub(crate) fn object_opacity(style: &str) -> Option<f64> {
    let t = style.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    let Json::Obj(items) = parse_json(t)? else {
        return None;
    };
    let Json::Obj(tr) = &items.iter().find(|(k, _)| k == "transparency")?.1 else {
        return None;
    };
    let get = |key: &str| {
        tr.iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.num())
            .filter(|n| n.is_finite())
    };
    let amount = match get("uniformTransparency") {
        Some(u) => u,
        None => (get("startTransparency")? + get("endTransparency")?) / 2.0,
    };
    Some((1.0 - amount).clamp(0.0, 1.0))
}

/// A graded transparency from an object's version 16+ style string: its
/// `transparency` section with a fountain `fill` (`type` 2) becomes a
/// mask whose lightness is the opacity, with the target it applies to
/// (`appliesTo`: 0 fill, 1 outline, 2 both). The mask's colours are the
/// fill's `primaryColor` (first) and `secondaryColor` (last) greys:
/// `GRAY255,USER,<0..255>,...`, 255 opaque (version 18 and 20 tutorial
/// files: the start and end amounts read 0 and 1 whichever grey comes
/// first, so the greys tell the direction). `fountainType` 1 to 4 is
/// linear, radial, conical and square as in fill records; `angle` is in
/// degrees, the centre offsets in percent of the object and a `xScale` or
/// `yScale` under 1 a padded edge, as the doubles of version 16+ fill
/// records. `None` for other transparencies.
pub(crate) fn object_transparency_mask(style: &str) -> Option<(tracedraw_core::Fill, u8)> {
    use tracedraw_core::{Color, Fill, Fountain, FountainKind, Stop};
    let t = style.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    let Json::Obj(items) = parse_json(t)? else {
        return None;
    };
    let field = |obj: &[(String, Json)], key: &str| -> Option<Json> {
        obj.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    };
    let Json::Obj(tr) = field(&items, "transparency")? else {
        return None;
    };
    let Json::Obj(fill) = field(&tr, "fill")? else {
        return None;
    };
    let num = |obj: &[(String, Json)], key: &str| {
        field(obj, key)
            .and_then(|v| v.num())
            .filter(|n| n.is_finite())
    };
    if num(&fill, "type")? as i64 != 2 {
        return None;
    }
    let grey = |key: &str| -> Option<f32> {
        let Json::Str(s) = field(&fill, key)? else {
            return None;
        };
        let mut parts = s.split(',');
        if !parts.next()?.starts_with("GRAY") {
            return None;
        }
        let v: f32 = parts.nth(1)?.trim().parse().ok()?;
        Some((v / 255.0).clamp(0.0, 1.0))
    };
    let (first, last) = (grey("primaryColor")?, grey("secondaryColor")?);
    let kind = match num(&fill, "fountainType").unwrap_or(1.0) as i64 {
        2 => FountainKind::Radial,
        3 => FountainKind::Conical,
        4 => FountainKind::Square,
        _ => FountainKind::Linear,
    };
    let pct = |key: &str| (num(&fill, key).unwrap_or(0.0) / 100.0).clamp(-1.0, 1.0);
    let extent = num(&fill, "xScale")
        .unwrap_or(1.0)
        .min(num(&fill, "yScale").unwrap_or(1.0));
    let edge_pad = if extent > 0.0 && extent < 1.0 {
        ((1.0 - extent) / 2.0).clamp(0.0, 0.49)
    } else {
        0.0
    };
    let mut fountain = Fountain {
        kind,
        stops: vec![
            Stop::new(0.0, Color::Gray { v: first }),
            Stop::new(1.0, Color::Gray { v: last }),
        ],
        angle: num(&fill, "angle").unwrap_or(0.0).rem_euclid(360.0),
        offset: tracedraw_core::geometry::Point::new(pct("xCenterOffset"), pct("yCenterOffset")),
        edge_pad,
        ..Default::default()
    };
    crate::mirror_centred_stops(&mut fountain);
    let target = num(&tr, "appliesTo").unwrap_or(2.0).clamp(0.0, 2.0) as u8;
    Some((Fill::Fountain(fountain), target))
}

/// The width of a paragraph frame from an object's version 16+ style
/// string, in file units: its `frame` section's `columns` lists each
/// column's width and the gutter after it (`"623721,0"`: one column); the
/// frame is the columns with the gutters between them (seen in a version
/// 21 file with one column; several columns assumed alike).
pub(crate) fn frame_width_units(style: &str) -> Option<f64> {
    let t = style.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    let Json::Obj(items) = parse_json(t)? else {
        return None;
    };
    let Json::Obj(frame) = &items.iter().find(|(k, _)| k == "frame")?.1 else {
        return None;
    };
    let Json::Str(cols) = &frame.iter().find(|(k, _)| k == "columns")?.1 else {
        return None;
    };
    let v: Vec<f64> = cols
        .split(',')
        .map(|p| p.trim().parse::<f64>().ok())
        .collect::<Option<Vec<f64>>>()?;
    if v.is_empty() {
        return None;
    }
    // Widths at even places, gutters at odd ones; the last gutter is
    // outside the frame.
    let widths: f64 = v.iter().step_by(2).sum();
    let gutters: f64 = v
        .iter()
        .skip(1)
        .step_by(2)
        .take(v.len().div_ceil(2).saturating_sub(1))
        .sum();
    let w = widths + gutters;
    (w.is_finite() && w > 0.0).then_some(w)
}

/// The colour of an inline fill object of a version 16+ style string:
/// `type` 1 is a uniform fill whose `primaryColor` reads
/// `MODEL,PALETTE,v1,v2,...` (`CMYK255` on 0..255 and `CMYK` on 0..100,
/// both seen in a version 21 file; `RGB255` and `GRAY255` assumed alike);
/// `type` 0 is no fill (`Some(None)`). Other fill types give `None` (the
/// object's fill applies).
fn inline_fill_color(fill: &Json) -> Option<Option<tracedraw_core::Color>> {
    let Json::Obj(items) = fill else {
        return None;
    };
    let get = |key: &str| items.iter().find(|(k, _)| k == key).map(|(_, v)| v);
    match get("type").and_then(Json::num) {
        Some(0.0) => return Some(None),
        Some(1.0) => {}
        _ => return None,
    }
    let Json::Str(spec) = get("primaryColor")? else {
        return None;
    };
    parse_color_spec(spec).map(Some)
}

/// A colour written `MODEL,PALETTE,v1,v2,...` in version 16+ style strings.
pub(crate) fn parse_color_spec(spec: &str) -> Option<tracedraw_core::Color> {
    use tracedraw_core::Color;
    let parts: Vec<&str> = spec.split(',').map(str::trim).collect();
    let model = parts.first()?.to_ascii_uppercase();
    let nums: Vec<f64> = parts
        .iter()
        .skip(2)
        .map_while(|p| p.parse::<f64>().ok())
        .collect();
    let unit = |v: f64, max: f64| (v / max).clamp(0.0, 1.0) as f32;
    let c = match model.as_str() {
        "CMYK255" if nums.len() >= 4 => Color::cmyk_pct(
            unit(nums[0], 255.0) * 100.0,
            unit(nums[1], 255.0) * 100.0,
            unit(nums[2], 255.0) * 100.0,
            unit(nums[3], 255.0) * 100.0,
        ),
        "CMYK" if nums.len() >= 4 => Color::cmyk_pct(
            unit(nums[0], 100.0) * 100.0,
            unit(nums[1], 100.0) * 100.0,
            unit(nums[2], 100.0) * 100.0,
            unit(nums[3], 100.0) * 100.0,
        ),
        "RGB255" | "RGB" if nums.len() >= 3 => Color::rgb8(
            nums[0].clamp(0.0, 255.0) as u8,
            nums[1].clamp(0.0, 255.0) as u8,
            nums[2].clamp(0.0, 255.0) as u8,
        ),
        "GRAY255" | "GRAYSCALE" | "GRAY" if !nums.is_empty() => {
            let g = nums[0].clamp(0.0, 255.0) as u8;
            Color::rgb8(g, g, g)
        }
        _ => return None,
    };
    Some(c)
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
    /// The object style string of a version 27 file whose outline its
    /// program shows with a mitre limit of 29 degrees (#384).
    #[test]
    fn the_mitre_limit_comes_from_the_outline_of_the_style_string() {
        let st = r#"{"StackedBitmapEffects":{},"fill":{"type":"0"},"outline":{"angle":"0","joinType":"0","miterLimit":"28.95502400000000164936864166520535945892333984375","width":"2238"},"transparency":{}}"#;
        let a = object_miter_limit(st).expect("a limit");
        assert!((a - 28.955024).abs() < 1e-6, "{a}");
        assert_eq!(object_miter_limit(r#"{"outline":{"width":"1"}}"#), None);
        assert_eq!(
            object_miter_limit(r#"{"outline":{"miterLimit":"x"}}"#),
            None
        );
        assert_eq!(
            object_miter_limit(r#"{"outline":{"miterLimit":"500"}}"#),
            Some(179.0)
        );
    }

    use super::*;

    /// Style strings of version 17 on are UTF-8: a font named in Chinese
    /// keeps its name; bytes that are not UTF-8 still read as
    /// Windows-1252 (#898).
    #[test]
    fn byte_style_strings_read_as_utf8() {
        let json = r#"{"character":{"latin":{"font":"方正黑体_GBK","size":"128819"}}}"#;
        let s = parse_style_string(&decode_style_bytes(json.as_bytes()));
        assert_eq!(s.family.as_deref(), Some("方正黑体_GBK"));
        assert_eq!(decode_style_bytes(b"Caf\xe9\0x"), "Caf\u{e9}");
        assert_eq!(decode_style_bytes(b"ab\0cd"), "ab");
        // The record as read from a text chunk: length, then the bytes.
        let mut d = (json.len() as u32).to_le_bytes().to_vec();
        d.extend_from_slice(json.as_bytes());
        let mut r = Reader::new(&d, false);
        let read = read_style_string(&mut r, &d, 19).expect("string");
        assert!(read.contains("方正黑体_GBK"), "{read}");
    }

    #[test]
    fn early_text_lines_end_at_cr_or_cr_lf() {
        let t = early_paragraphs(b"ab\r\ncd\ref", Some(10.0));
        let lines: Vec<String> = t
            .paragraphs
            .iter()
            .map(|p| p.runs.iter().map(|r| r.text.clone()).collect())
            .collect();
        assert_eq!(lines, ["ab", "cd", "ef"]);
    }

    #[test]
    fn version_5_and_6_texts_are_read() {
        let texts = |t: &Txsm| -> Vec<String> {
            t.paragraphs
                .iter()
                .map(|p| p.runs.iter().map(|r| r.text.as_str()).collect())
                .collect()
        };
        // Version 5: size 500 thousandths of an inch (36 pt), "Hi".
        let mut v5 = vec![0u8; 52];
        v5[40..42].copy_from_slice(&500u16.to_le_bytes());
        v5[50..52].copy_from_slice(&2u16.to_le_bytes());
        for c in [b'H', b'i'] {
            v5.extend_from_slice(&[0x10, 0x01, 0, 0, c, 0, 9, 0]);
        }
        let t = read_txsm(&v5, 5, &mut |_| {}).expect("text");
        assert_eq!(texts(&t), ["Hi"]);
        let size = t.paragraphs[0].runs[0].style.size_pt.unwrap();
        assert!((size - 36.0).abs() < 1e-9);
        // Version 6: one style block with a 10 mm size, "A\rB" as two
        // paragraphs.
        let mut v6 = vec![0u8; 160];
        v6[96..100].copy_from_slice(&1u32.to_le_bytes());
        v6[136..140].copy_from_slice(&100_000u32.to_le_bytes());
        v6.extend_from_slice(&3u32.to_le_bytes());
        for c in [b'A', b'\r', b'B'] {
            v6.extend_from_slice(&50_000u32.to_le_bytes());
            v6.extend_from_slice(&(c as u32).to_le_bytes());
            v6.extend_from_slice(&0x1000_0000u32.to_le_bytes());
        }
        let t = read_txsm(&v6, 6, &mut |_| {}).expect("text");
        assert_eq!(texts(&t), ["A", "B"]);
        let size = t.paragraphs[0].runs[0].style.size_pt.unwrap();
        assert!((size - 100_000.0 * 72.0 / 254000.0).abs() < 1e-9);
        // Cut anywhere: nothing panics.
        for cut in 0..v6.len() {
            let _ = read_txsm(&v6[..cut], 6, &mut |_| {});
        }
        for cut in 0..v5.len() {
            let _ = read_txsm(&v5[..cut], 5, &mut |_| {});
        }
    }

    #[test]
    fn cp1252_maps_the_high_half() {
        assert_eq!(
            decode_cp1252(&[0x41, 0x80, 0x93, 0xe9, 0x00, 0x42]),
            "A\u{20ac}\u{201c}é"
        );
    }

    #[test]
    fn old_text_reads_in_the_code_page_of_its_character_set() {
        // Cyrillic, Central European, Greek, Turkish, Baltic, Japanese.
        assert_eq!(
            decode_charset(&[0xcf, 0xf0, 0xe8, 0xe2, 0xe5, 0xf2], 204),
            "\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}"
        );
        assert_eq!(
            decode_charset(&[0xa3, 0xf3, 0x64, 0x9f], 238),
            "\u{141}\u{f3}d\u{17a}"
        );
        assert_eq!(
            decode_charset(&[0xc3, 0xe5, 0xe9, 0xe1], 161),
            "\u{393}\u{3b5}\u{3b9}\u{3b1}"
        );
        assert_eq!(
            decode_charset(&[0xf0, 0xfd, 0xfe], 162),
            "\u{11f}\u{131}\u{15f}"
        );
        assert_eq!(decode_charset(&[0xe0, 0xf0], 186), "\u{105}\u{161}");
        assert_eq!(
            decode_charset(&[0x93, 0xfa, 0x96, 0x7b], 128),
            "\u{65e5}\u{672c}"
        );
        // Latin and unknown sets read as Windows-1252; symbol fonts keep
        // the byte values; text ends at a NUL.
        assert_eq!(decode_charset(&[0x80, 0xe9], 0), "\u{20ac}\u{e9}");
        assert_eq!(decode_charset(&[0x80, 0xe9], 77), "\u{20ac}\u{e9}");
        assert_eq!(decode_charset(&[0x41, 0x80, 0xf0], 2), "A\u{80}\u{f0}");
        assert_eq!(decode_charset(&[0xcf, 0, 0xf0], 204), "\u{41f}");
        assert!(is_non_latin_charset(204) && is_non_latin_charset(2));
        assert!(!is_non_latin_charset(0) && !is_non_latin_charset(1));
        // A cut double-byte character is replaced, not a panic.
        assert_eq!(decode_charset(&[0x41, 0x93], 128), "A\u{fffd}");
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

    /// The version 21 style string: the run's colour from its inline
    /// fill, the Latin font, the size; the outline's and the paragraph's
    /// own keys (behindFill, overline geometry) are not run settings.
    #[test]
    fn version_21_style_strings_give_the_inline_colour() {
        let s = parse_style_string(
            r#"{"character":{"fill":{"type":"1","primaryColor":"CMYK255,USER,0,255,255,0,100,u"},
            "outline":{"behindFill":"0","width":"762"},
            "RTL":{"font":"Other","size":"9999999"},
            "latin":{"font":"Example Script","weight":"64","italic":"0","size":"128819"}},
            "paragraph":{"interLineSpacing":"1200000","thinOverlineThickness":"45455","dblUnderline1Distance":"-90909"}}"#,
        );
        assert_eq!(s.family.as_deref(), Some("Example Script"));
        assert!((s.size_pt.expect("size") - 36.5156).abs() < 1e-3);
        assert_eq!(s.bold, Some(false));
        assert_eq!(s.fill_id, None);
        assert_eq!(s.outl_id, None);
        assert_eq!(
            s.fill_color,
            Some(Some(tracedraw_core::Color::cmyk_pct(
                0.0, 100.0, 100.0, 0.0
            )))
        );
        assert!((s.line_spacing.expect("spacing") - 1.2).abs() < 1e-9);
        // No fill, and colour specs in the other spellings.
        let s = parse_style_string(r#"{"character":{"fill":{"type":"0"}}}"#);
        assert_eq!(s.fill_color, Some(None));
        assert_eq!(
            parse_color_spec("CMYK,USER,0,0,0,100,100,x"),
            Some(tracedraw_core::Color::cmyk_pct(0.0, 0.0, 0.0, 100.0))
        );
        assert_eq!(
            parse_color_spec("RGB255,USER,255,128,0,100,x"),
            Some(tracedraw_core::Color::rgb8(255, 128, 0))
        );
        assert_eq!(parse_color_spec("PANTONE,X,1"), None);
        assert_eq!(parse_color_spec(""), None);
        // The frame's width: the columns and the gutters between them.
        assert_eq!(
            frame_width_units(r#"{"frame":{"columns":"623721,0","equalWidthColumns":"1"}}"#),
            Some(623721.0)
        );
        assert_eq!(
            frame_width_units(r#"{"frame":{"columns":"100,10,100,10"}}"#),
            Some(210.0)
        );
        assert_eq!(frame_width_units(r#"{"frame":{}}"#), None);
        assert_eq!(frame_width_units(r#"{"frame":{"columns":"x"}}"#), None);
        assert_eq!(frame_width_units("not json"), None);
    }

    #[test]
    fn style_strings_give_the_object_transparency() {
        let o = object_opacity(
            r#"{"transparency":{"startTransparency":"0.3","endTransparency":"1","uniformTransparency":"0.3","mode":"0"},"fill":{}}"#,
        );
        assert!((o.unwrap() - 0.7).abs() < 1e-9);
        let o =
            object_opacity(r#"{"transparency":{"startTransparency":"0","endTransparency":"1"}}"#);
        assert!((o.unwrap() - 0.5).abs() < 1e-9);
        assert_eq!(object_opacity(r#"{"transparency":{},"fill":{}}"#), None);
        assert_eq!(object_opacity(r#"{"fill":{}}"#), None);
    }

    #[test]
    fn graded_transparency_becomes_a_mask() {
        use tracedraw_core::{Color, Fill, FountainKind};
        let style = r#"{"fill":{},"transparency":{"mode":"0","fill":{"type":"2","fountainType":"1","angle":"-90","xCenterOffset":"10","yCenterOffset":"-5","xScale":"0.8","yScale":"1","primaryColor":"GRAY255,USER,255,100,0","secondaryColor":"GRAY255,USER,0,100,0"},"appliesTo":"0","startTransparency":"0","endTransparency":"1"}}"#;
        let (mask, target) = object_transparency_mask(style).expect("mask");
        assert_eq!(target, 0);
        let Fill::Fountain(f) = mask else {
            panic!("{mask:?}")
        };
        assert_eq!(f.kind, FountainKind::Linear);
        assert!((f.angle - 270.0).abs() < 1e-9);
        assert!((f.offset.x - 0.1).abs() < 1e-9 && (f.offset.y + 0.05).abs() < 1e-9);
        assert!((f.edge_pad - 0.1).abs() < 1e-9);
        assert_eq!(f.stops[0].color, Color::Gray { v: 1.0 });
        assert_eq!(f.stops[1].color, Color::Gray { v: 0.0 });
        // A uniform transparency, a pattern one or none: no mask.
        for other in [
            r#"{"transparency":{"fill":{"type":"0"},"uniformTransparency":"0.4"}}"#,
            r#"{"transparency":{"fill":{"type":"10"},"startTransparency":"0","endTransparency":"1"}}"#,
            r#"{"transparency":{}}"#,
            r#"{"fill":{}}"#,
            r#"{"transparency":{"fill":{"type":"2","primaryColor":"CMYK,USER,0,0,0,0"}}}"#,
        ] {
            assert!(object_transparency_mask(other).is_none(), "{other}");
        }
        assert_eq!(object_opacity("not json"), None);
        let o = object_opacity(r#"{"transparency":{"uniformTransparency":"7"}}"#);
        assert_eq!(o, Some(0.0));
    }

    #[test]
    fn style_string_weights_are_style_codes() {
        let latin = |inner: &str| parse_style_string(&format!(r#"{{"latin":{{{inner}}}}}"#));
        let s = latin(r#""weight":"64","italic":"0""#);
        assert_eq!((s.bold, s.italic), (Some(false), Some(false)));
        // 1024 is a 600 weight (a semibold face), 4096 a 700 one.
        assert_eq!(latin(r#""weight":"1024""#).bold, Some(true));
        assert_eq!(latin(r#""weight":"4096""#).bold, Some(true));
        // The weights the codes stand for.
        assert_eq!(latin(r#""weight":"64""#).weight, Some(400));
        assert_eq!(latin(r#""weight":"1024""#).weight, Some(600));
        assert_eq!(latin(r#""weight":"4096""#).weight, Some(700));
        assert_eq!(latin(r#""weight":"8192""#).weight, Some(700));
        assert_eq!(StyleFlags(0x10).weight(), Some(300));
        assert_eq!(StyleFlags(0x2_0000).weight(), Some(900));
        assert_eq!(StyleFlags(1 << 18).weight(), None);
        // An odd exponent is the italic of a weight.
        let s = latin(r#""weight":"8192""#);
        assert_eq!((s.bold, s.italic), (Some(true), Some(true)));
        assert_eq!(latin(r#""weight":"128""#).italic, Some(true));
        // CSS weights and words still read.
        assert_eq!(latin(r#""weight":700"#).bold, Some(true));
        assert_eq!(latin(r#""weight":"400""#).bold, Some(false));
        assert_eq!(latin(r#""fontWeight":"bold""#).bold, Some(true));
        // Italic, underline and strikeout written as "1" or "0".
        let s = latin(r#""italic":"1","underline":"1","strikeout":"0","overline":"1""#);
        assert_eq!(s.italic, Some(true));
        assert_eq!(s.underline, Some(true));
        assert_eq!(s.strikethrough, Some(false));
        assert_eq!(s.outl_id, None);
    }

    #[test]
    fn line_breaks_become_newlines() {
        assert_eq!(line_breaks("Fazendo\rum bolo"), "Fazendo\num bolo");
        assert_eq!(
            line_breaks("a\r\nb\u{2028}c\u{2029}d\u{0B}e"),
            "a\nb\nc\nd\ne"
        );
        assert_eq!(line_breaks("plain"), "plain");
    }

    #[test]
    fn style_string_json_and_key_value_forms() {
        let s = parse_style_string(
            r#"{"character":{"font":7,"size":304800,"fill":12},"paragraph":{"justify":"2"}}"#,
        );
        assert_eq!(s.font_id, Some(7));
        assert!((s.size_pt.unwrap() - 86.4).abs() < 1e-6);
        assert_eq!(s.fill_id, Some(12));
        // "justify" 2 centres, as in the style table.
        assert_eq!(s.align, Some(2));
        let align =
            |j: &str| parse_style_string(&format!(r#"{{"paragraph":{{"justify":"{j}"}}}}"#)).align;
        assert_eq!(align("0"), Some(1));
        assert_eq!(align("1"), Some(1));
        assert_eq!(align("2"), Some(2));
        assert_eq!(align("3"), Some(3));
        assert_eq!(align("4"), Some(4));
        assert_eq!(align("9"), None);
        let s = parse_style_string("font-family:Times New Roman;font-size:12;font-weight:bold");
        assert_eq!(s.family.as_deref(), Some("Times New Roman"));
        assert_eq!(s.size_pt, Some(12.0));
        assert_eq!(s.bold, Some(true));
        assert_eq!(parse_style_string("").size_pt, None);
        assert_eq!(parse_style_string("{broken").size_pt, None);
    }

    #[test]
    fn split_scripts_cuts_east_asian_runs() {
        assert!(is_east_asian('荣') && is_east_asian('〇') && is_east_asian('ア'));
        assert!(!is_east_asian('A') && !is_east_asian('é') && !is_east_asian('5'));
        assert_eq!(
            split_scripts("Ab 荣誉, 250g铜版纸\n"),
            vec![
                ("Ab ".to_string(), false),
                ("荣誉, ".to_string(), true),
                ("250g".to_string(), false),
                ("铜版纸\n".to_string(), true),
            ]
        );
        // Neutral characters at the start go with the first run.
        assert_eq!(split_scripts(" 在"), vec![(" 在".to_string(), true)]);
        assert_eq!(split_scripts(" \n"), vec![(" \n".to_string(), false)]);
        assert!(split_scripts("").is_empty());
    }

    #[test]
    fn style_string_far_east_section_is_the_east_asian_font() {
        let s = parse_style_string(
            r#"{"character":{"latin":{"font":"Arial","size":"42333"},"farEast":{"font":"Script Han","size":"190500","weight":"4096"}}}"#,
        );
        assert_eq!(s.family.as_deref(), Some("Arial"));
        let a = s.asian.as_ref().expect("far east font");
        assert_eq!(a.family.as_deref(), Some("Script Han"));
        assert!(a.differs_from(&s));
        let ea = s.for_east_asian();
        assert_eq!(ea.family.as_deref(), Some("Script Han"));
        assert!((ea.size_pt.unwrap() - 54.0).abs() < 1e-3);
        assert_eq!(ea.bold, Some(true));
        assert!(ea.asian.is_none());
        // A run naming its own font sets it for every script; one that
        // only changes the size keeps the East Asian font at that size.
        let own = RunStyle {
            family: Some("Verdana".into()),
            ..RunStyle::default()
        };
        assert!(own.over(&s).asian.is_none());
        let sized = RunStyle {
            size_pt: Some(20.0),
            ..RunStyle::default()
        };
        let o = sized.over(&s);
        assert_eq!(o.asian.as_ref().and_then(|a| a.size_pt), Some(20.0));
        assert_eq!(
            o.asian.as_ref().and_then(|a| a.family.clone()).as_deref(),
            Some("Script Han")
        );
        // Alike settings in both sections make no second run.
        let same = parse_style_string(
            r#"{"character":{"latin":{"font":"Arial","size":"42333"},"farEast":{"font":"Arial","size":"42333"}}}"#,
        );
        assert!(!same.asian.as_ref().unwrap().differs_from(&same));
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

    /// A version 15 artistic text record (frame layout, one frame, one
    /// paragraph) with three styles: a locale only, 8 pt, and 8 pt with
    /// the flag byte of fl3 0x20 (`block` set or not); `index` numbers
    /// each letter's style in its record.
    fn txsm15(text: &str, index: &[u8], block: bool) -> Vec<u8> {
        let mut d = vec![0xff; 4];
        d.extend_from_slice(&[0; 33]);
        d.extend_from_slice(&1u32.to_le_bytes()); // frames
        d.extend_from_slice(&7u32.to_le_bytes()); // frame id
        d.extend_from_slice(&[0; 48]); // matrix
        d.extend_from_slice(&0u32.to_le_bytes()); // not on a path
        d.extend_from_slice(&[0; 8 + 4]);
        d.extend_from_slice(&1u32.to_le_bytes()); // paragraphs
        d.extend_from_slice(&9u32.to_le_bytes()); // style id
        d.extend_from_slice(&[0, 0]);
        d.extend_from_slice(&3u32.to_le_bytes()); // styles
                                                  // Lengths that do not add up to the text, as in the file.
        d.extend_from_slice(&[0x21, 0, 0, 0x08]);
        d.extend_from_slice(&3u32.to_le_bytes());
        d.extend_from_slice(&[b'C', 0, b'S', 0, b'Y', 0]);
        let size = 28222i32.to_le_bytes(); // 8 pt
        d.extend_from_slice(&[0x21, 0, 0x04, 0]);
        d.extend_from_slice(&size);
        d.extend_from_slice(&[3, 0, 0x04, 0x20]);
        d.extend_from_slice(&size);
        if block {
            d.push(1);
            d.extend_from_slice(&[0xab; 52]);
        } else {
            d.push(0);
        }
        d.extend_from_slice(&(text.len() as u32).to_le_bytes());
        for i in index {
            d.extend_from_slice(&[0, 0, i << 1, 0, 0, 0, 0, 0]);
        }
        d.extend_from_slice(&(text.len() as u32).to_le_bytes());
        d.extend_from_slice(text.as_bytes());
        d.push(0); // no path
        d
    }

    #[test]
    fn version_15_letters_take_the_style_their_records_number() {
        let index = [0, 0, 0, 0, 2, 2, 2, 1, 1, 1];
        for block in [false, true] {
            let mut warnings = Vec::new();
            let t = read_txsm(&txsm15("MASS 941kg", &index, block), 15, &mut |w| {
                warnings.push(w)
            })
            .expect("txsm");
            assert!(warnings.is_empty(), "{warnings:?}");
            let runs: Vec<(&str, Option<f64>)> = t.paragraphs[0]
                .runs
                .iter()
                .map(|r| {
                    (
                        r.text.as_str(),
                        r.style.size_pt.map(|p| (p * 10.0).round() / 10.0),
                    )
                })
                .collect();
            assert_eq!(
                runs,
                [("MASS", None), (" 94", Some(8.0)), ("1kg", Some(8.0))],
                "block {block}"
            );
        }
        // With every record naming style 0 the lengths still cut the runs.
        let t = read_txsm(&txsm15("MASS 941kg", &[0; 10], false), 15, &mut |_| {}).expect("txsm");
        assert_eq!(t.paragraphs[0].runs[0].text.len(), 10);
    }

    /// A version 8 text record (frame layout, one frame, one paragraph)
    /// with a base style over every letter and a second style naming font
    /// 7 in character set 2; `index` numbers each letter's style.
    fn txsm8(text: &[u8], index: &[u8]) -> Vec<u8> {
        let mut d = vec![0xff; 4];
        d.extend_from_slice(&[0; 32]);
        d.extend_from_slice(&1u32.to_le_bytes()); // frames
        d.extend_from_slice(&7u32.to_le_bytes()); // frame id
        d.extend_from_slice(&[0; 48]); // matrix
        d.extend_from_slice(&0u32.to_le_bytes()); // not on a path
        d.extend_from_slice(&1u32.to_le_bytes()); // paragraphs
        d.extend_from_slice(&9u32.to_le_bytes()); // style id
        d.push(0);
        d.extend_from_slice(&2u32.to_le_bytes()); // styles
        d.extend_from_slice(&[text.len() as u8, 0, 0, 0]);
        d.extend_from_slice(&[1, 0, 0x05, 0, 7, 0, 2, 0]);
        d.extend_from_slice(&28222i32.to_le_bytes()); // 8 pt
        d.extend_from_slice(&(text.len() as u32).to_le_bytes());
        for i in index {
            d.extend_from_slice(&[0, 0, i << 1, 0]);
        }
        d.extend_from_slice(text);
        d.push(0); // no path
        d
    }

    /// A version 8 paragraph text whose frames take `frame_bytes`.
    fn txsm8_frames(text: &[u8], frame_bytes: usize) -> Vec<u8> {
        let mut d = vec![0; 4]; // not a frame
        d.extend_from_slice(&[0; 32]);
        d.extend_from_slice(&1u32.to_le_bytes()); // frames
        d.extend_from_slice(&7u32.to_le_bytes()); // frame id
        d.extend_from_slice(&[0; 48]); // matrix
        d.extend_from_slice(&0u32.to_le_bytes()); // not on a path
        d.extend_from_slice(&vec![0; frame_bytes]);
        d.extend_from_slice(&1u32.to_le_bytes()); // paragraphs
        d.extend_from_slice(&9u32.to_le_bytes()); // style id
        d.push(0);
        d.extend_from_slice(&1u32.to_le_bytes()); // styles
        d.extend_from_slice(&[text.len() as u8, 0, 0, 0]);
        d.extend_from_slice(&(text.len() as u32).to_le_bytes());
        for _ in text {
            d.extend_from_slice(&[0, 0, 0, 0]);
        }
        d.extend_from_slice(text);
        d.push(0); // no path
        d
    }

    /// Version 8 text frames of 32 bytes (800) and of 34 (the
    /// bidirectional 801) are both read.
    #[test]
    fn version_8_frames_of_either_layout() {
        for bytes in [32, 34] {
            let mut notes = Vec::new();
            let t =
                read_txsm(&txsm8_frames(b"Lorem", bytes), 8, &mut |w| notes.push(w)).expect("txsm");
            assert!(notes.is_empty(), "{bytes}: {notes:?}");
            assert_eq!(t.paragraphs[0].runs[0].text, "Lorem", "{bytes}");
        }
    }

    #[test]
    fn version_8_letters_take_the_style_their_records_number() {
        let t = read_txsm(&txsm8(b"x\xc4y", &[0, 1, 0]), 8, &mut |w| panic!("{w}")).expect("txsm");
        let runs: Vec<(&str, Option<u16>)> = t.paragraphs[0]
            .runs
            .iter()
            .map(|r| (r.text.as_str(), r.style.font_id))
            .collect();
        assert_eq!(runs, [("x", None), ("\u{c4}", Some(7)), ("y", None)]);
        // Records that all name style 0 leave the lengths to cut the runs.
        let t = read_txsm(&txsm8(b"x\xc4y", &[0, 0, 0]), 8, &mut |_| {}).expect("txsm");
        assert_eq!(t.paragraphs[0].runs[0].text, "x\u{c4}y");
    }
}
