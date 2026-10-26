//! Text engine: finds system fonts, shapes runs with rustybuzz (OpenType
//! features, kerning) and lays paragraphs out with wrapping, alignment,
//! justification, leading, indents, tabs, bullets, drop caps, columns,
//! hyphenation and text on a path. The result is a path in millimetres,
//! so text is plain vectors for the renderer and for "convert to curves".
//!
//! Coordinates: baseline of the first line at y = 0 for artistic text; the
//! top of the frame at y = 0 for paragraph text. Y is up, as everywhere.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock, RwLockReadGuard};
use tracedraw_core::{
    document::{
        text_outline::TextRequest, DocumentFont, ParagraphStyle, PathTextOrientation,
        PathTextVertical, TextAlign, TextOnPath, TextSpan, VerticalAlign,
    },
    geometry::{Affine, BezPath, Point, Rect, Shape as _, Size},
    text_flow::{free_segments, WrapRegion},
};

pub mod catalog;
pub mod hyphen;
#[cfg(any(windows, all(test, not(target_arch = "wasm32"))))]
mod installed;

/// Points to millimetres.
pub const PT_MM: f64 = 25.4 / 72.0;

use tracedraw_core::document::text_outline::{GlyphLayout, PlacedGlyph};

pub struct FontSystem {
    /// Installed fonts, then the fonts documents brought (see
    /// [`FontSystem::add_document_fonts`]).
    db: RwLock<fontdb::Database>,
    faces: Mutex<HashMap<fontdb::ID, Arc<Vec<u8>>>>,
    families: RwLock<Vec<String>>,
    /// Families that came with documents, sorted.
    document_families: RwLock<Vec<String>>,
    /// The fonts documents brought that were loaded, with the families
    /// each added.
    carried: RwLock<Vec<(Vec<String>, DocumentFont)>>,
    cache: Mutex<HashMap<String, CachedOutline>>,
    /// Families the application can fetch when text asks for them (the
    /// browser build's font folder), sorted; see
    /// [`FontSystem::offer_families`].
    offered: RwLock<Vec<String>>,
    /// Offered families text asked for and the application has not taken
    /// yet, then every family ever asked for (each is asked for once).
    requests: Mutex<(Vec<String>, Vec<String>)>,
    /// The face drawing a letter the span's own face lacks, by letter and
    /// style (`None`: no face has it).
    fallbacks: Mutex<HashMap<(char, bool, bool), Option<fontdb::ID>>>,
    /// Counts the times fonts were added, so views drawn with stand-ins
    /// know to draw again.
    generation: AtomicU64,
    /// Family names fonts give as their legacy family (name ID 1, "Arial
    /// Narrow", "Arial Black", "Segoe UI Semibold") where fontdb lists the
    /// typographic family ("Arial"), with their faces: drawings name fonts
    /// that way.
    legacy: RwLock<HashMap<String, Vec<fontdb::ID>>>,
}

/// Register this engine as the core's text outliner (call once at startup).
pub fn install() {
    tracedraw_core::document::text_outline::set(|req| fonts().outline_cached(req).as_ref().clone());
    tracedraw_core::document::text_outline::set_bounds(|req| fonts().bounds_cached(req));
    tracedraw_core::document::text_outline::set_glyphs(|req| fonts().glyph_layout(req));
    tracedraw_core::document::text_outline::set_lines(|req| {
        let layout = fonts().layout(req);
        // Lines past what fits the frame flow on (to a linked frame);
        // only those inside it are reported.
        layout
            .lines
            .iter()
            .filter(|l| l.start_char < layout.fitted_chars || layout.fitted_chars == 0)
            .map(|l| tracedraw_core::document::text_outline::Line {
                start_char: l.start_char,
                end_char: l.end_char,
                // The left end, the end of right-to-left text too.
                x: l.x_range().0,
                baseline: l.baseline,
                ascent: l.ascent,
                descent: l.descent,
                edges: l.edges.clone(),
                starts: l.starts.clone(),
                ends: l.ends.clone(),
            })
            .collect()
    });
}

/// A cached text outline and its box (`None` for an empty outline).
type CachedOutline = (Arc<BezPath>, Option<Rect>);

static SYSTEM: OnceLock<FontSystem> = OnceLock::new();

/// Font files registered before the font system starts (the browser build
/// has no system fonts and registers bundled ones here).
static EXTRA_FONTS: Mutex<Vec<Arc<Vec<u8>>>> = Mutex::new(Vec::new());

/// Register a font file (TTF, OTF or a collection). Takes effect when the
/// font system starts, so call it before the first `fonts()`; returns
/// false when the system has already started and the data was ignored.
pub fn add_font_data(data: Vec<u8>) -> bool {
    if SYSTEM.get().is_some() {
        return false;
    }
    match EXTRA_FONTS.lock() {
        Ok(mut v) => {
            v.push(Arc::new(data));
            true
        }
        Err(_) => false,
    }
}

/// The process-wide font system; loads system fonts on first use.
pub fn fonts() -> &'static FontSystem {
    SYSTEM.get_or_init(FontSystem::load)
}

/// Families that stand in for `family` when it is not installed, best
/// first: fonts drawn to the same widths (so text keeps its line breaks:
/// Liberation, Arimo, Tinos and Cousine for the common office faces,
/// Carlito, Caladea, Gelasio, Selawik, the TeX Gyre and URW families for
/// the classic PostScript faces), then fonts of the same style (serif,
/// monospaced, casual or sans-serif, judged by the name).
pub fn substitutes(family: &str) -> Vec<&'static str> {
    const SANS: &[&str] = &[
        "Liberation Sans",
        "Arimo",
        "Nimbus Sans",
        "Nimbus Sans L",
        "TeX Gyre Heros",
        "FreeSans",
    ];
    const SERIF: &[&str] = &[
        "Liberation Serif",
        "Tinos",
        "Nimbus Roman",
        "Nimbus Roman No9 L",
        "TeX Gyre Termes",
        "FreeSerif",
    ];
    const MONO: &[&str] = &[
        "Liberation Mono",
        "Cousine",
        "Nimbus Mono PS",
        "Nimbus Mono L",
        "TeX Gyre Cursor",
        "FreeMono",
    ];
    // Lower case letters and digits only, without the style and the
    // endings foundries add to names ("Arial MT", "TimesNewRomanPS",
    // "Arial-BoldMT").
    let mut key: String = family
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    const ENDINGS: [&str; 13] = [
        "regular",
        "bolditalic",
        "boldoblique",
        "italic",
        "oblique",
        "bold",
        "psmt",
        "mt",
        "ps",
        "ms",
        "std",
        "pro",
        "lt",
    ];
    loop {
        let before = key.len();
        for end in ENDINGS {
            if key.len() > end.len() + 3 {
                if let Some(k) = key.strip_suffix(end) {
                    key = k.to_string();
                }
            }
        }
        if key.len() == before {
            break;
        }
    }
    let same: &[&'static str] = match key.as_str() {
        "arial" | "helvetica" | "helveticaneue" | "arialunicode" | "arimo" | "nimbussans"
        | "liberationsans" => SANS,
        "arialnarrow" | "helveticanarrow" | "helveticacondensed" | "liberationsansnarrow" => &[
            "Liberation Sans Narrow",
            "Nimbus Sans Narrow",
            "Arial Narrow",
        ],
        "timesnewroman" | "times" | "timesroman" | "tmsrmn" | "tinos" | "nimbusroman"
        | "liberationserif" => SERIF,
        "couriernew" | "courier" | "cousine" | "liberationmono" => MONO,
        "calibri" => &["Carlito"],
        "cambria" => &["Caladea"],
        "georgia" => &["Gelasio"],
        "segoeui" => &["Selawik"],
        "verdana" | "tahoma" => &["DejaVu Sans", "Bitstream Vera Sans"],
        "palatino" | "palatinolinotype" | "bookantiqua" => {
            &["TeX Gyre Pagella", "P052", "URW Palladio L"]
        }
        "bookman" | "bookmanoldstyle" | "itcbookman" => {
            &["TeX Gyre Bonum", "URW Bookman", "URW Bookman L"]
        }
        "centuryschoolbook" | "newcenturyschoolbook" => {
            &["TeX Gyre Schola", "C059", "Century Schoolbook L"]
        }
        "centurygothic" | "avantgarde" | "avantgardegothic" | "itcavantgardegothic" => {
            &["TeX Gyre Adventor", "URW Gothic", "URW Gothic L"]
        }
        "zapfchancery" | "itczapfchancery" | "monotypecorsiva" => {
            &["TeX Gyre Chorus", "Z003", "URW Chancery L"]
        }
        "comicsans" => &["Comic Neue", "Comic Relief"],
        "garamond" | "ebgaramond" | "garamondpremier" | "itcgaramond" => {
            &["EB Garamond", "Cormorant Garamond"]
        }
        "consolas" | "lucidaconsole" | "menlo" | "monaco" => &["DejaVu Sans Mono"],
        _ => &[],
    };
    let has = |words: &[&str]| words.iter().any(|w| key.contains(w));
    let style: &[&'static str] = if has(&[
        "mono",
        "courier",
        "consol",
        "code",
        "typewriter",
        "menlo",
        "monaco",
    ]) {
        MONO
    } else if !key.contains("sans")
        && has(&[
            "serif",
            "times",
            "roman",
            "garamond",
            "georgia",
            "bookman",
            "bodoni",
            "baskerville",
            "caslon",
            "minion",
            "cambria",
            "antiqua",
            "schoolbook",
            "palatino",
            "goudy",
            "didot",
            "plantin",
            "century",
        ])
        && !key.contains("gothic")
    {
        SERIF
    } else if has(&["comic", "marker", "handwrit", "casual"]) {
        &["Comic Neue", "Comic Relief", "Liberation Sans", "Arimo"]
    } else {
        SANS
    };
    let mut out: Vec<&'static str> = same.to_vec();
    for f in style {
        if !out.contains(f) {
            out.push(f);
        }
    }
    out
}

/// Weight names a family name can end in, with their weights: drawings
/// name a face of a family that way ("Titillium Web SemiBold").
const WEIGHT_NAMES: [(&str, u16); 12] = [
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

/// A family name ending in a weight name, split into the family and the
/// weight: "Titillium Web SemiBold" is ("Titillium Web", 600), and so is
/// "Titillium Web Semi Bold". `None` for other names.
pub fn split_weight(family: &str) -> Option<(&str, u16)> {
    // Compared letter by letter, without building a string: the font
    // lists ask this for every family a frame shows.
    let weight_of = |words: &[&str]| {
        WEIGHT_NAMES
            .iter()
            .find(|(n, _)| {
                let mut key = words
                    .iter()
                    .flat_map(|w| w.chars())
                    .filter(|c| c.is_ascii_alphanumeric())
                    .map(|c| c.to_ascii_lowercase());
                n.chars().all(|c| key.next() == Some(c)) && key.next().is_none()
            })
            .map(|(_, w)| *w)
    };
    // The weight is the last word, or the last two run together.
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

/// The weight text in `family` is drawn at: the weight the family's
/// name ends in ("Lato Black" 900), else 400; bold text at least 700.
pub fn target_weight(family: &str, bold: bool) -> u16 {
    let named = split_weight(family).map_or(400, |(_, w)| w);
    if bold {
        named.max(700)
    } else {
        named
    }
}

/// The legacy family names (name ID 1) a face's name table gives that
/// differ from the families fontdb lists for it (the typographic family,
/// name ID 16): "Arial Narrow" for a face fontdb files under "Arial".
fn legacy_families(db: &fontdb::Database, id: fontdb::ID) -> Vec<String> {
    let Some(face) = db.face(id) else {
        return Vec::new();
    };
    let known: Vec<&str> = face.families.iter().map(|(n, _)| n.as_str()).collect();
    db.with_face_data(id, |data, index| {
        let Ok(f) = ttf_parser::Face::parse(data, index) else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        for name in f.names() {
            if name.name_id != ttf_parser::name_id::FAMILY || !name.is_unicode() {
                continue;
            }
            let Some(text) = name.to_string() else {
                continue;
            };
            let text = text.trim().to_string();
            if !text.is_empty() && !known.contains(&text.as_str()) && !out.contains(&text) {
                out.push(text);
            }
        }
        out
    })
    .unwrap_or_default()
}

/// How a family's fonts are built, for the icons of font lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FontKind {
    /// TrueType outlines.
    TrueType,
    /// OpenType with PostScript (CFF) outlines.
    OpenType,
    /// A variable font: weights or widths along axes.
    Variable,
    /// Not installed: the application fetches it when asked (the browser
    /// build's font folder and catalog).
    Offered,
}

/// One face of a family, for a font list's styles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceStyle {
    /// The style name the font gives ("Bold Italic", "Light"), else one
    /// made from the weight and slant.
    pub name: String,
    pub weight: u16,
    pub italic: bool,
}

/// What a font list shows about a family (see
/// [`FontSystem::family_info`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyInfo {
    pub kind: FontKind,
    /// Its faces, upright first, by weight; empty for an offered family.
    pub faces: Vec<FaceStyle>,
    /// The family came with a document: it is not installed.
    pub embedded: bool,
    /// A symbol font (its character map is the symbol encoding): its
    /// letters are pictures, so its name reads in another font.
    pub symbol: bool,
}

/// A face's style name: the typographic subfamily (name ID 17), else the
/// subfamily (name ID 2), in English where the font has it.
fn face_style_name(f: &ttf_parser::Face) -> Option<String> {
    let mut best: Option<(u8, String)> = None;
    for name in f.names() {
        let rank = match name.name_id {
            ttf_parser::name_id::TYPOGRAPHIC_SUBFAMILY => 0,
            ttf_parser::name_id::SUBFAMILY => 2,
            _ => continue,
        };
        let english =
            name.language_id == 0x409 || name.platform_id == ttf_parser::PlatformId::Macintosh;
        let rank = rank + u8::from(!english);
        let Some(text) = name.to_string() else {
            continue;
        };
        let text = text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        if best.as_ref().is_none_or(|(r, _)| rank < *r) {
            best = Some((rank, text));
        }
    }
    best.map(|(_, t)| t)
}

/// `data` is a font file (or collection) whose every face reads and
/// allows use in documents: its licence (the OS/2 embedding bits) is not
/// "restricted" and allows outlines, not just bitmaps.
pub fn usable_in_documents(data: &[u8]) -> bool {
    let n = ttf_parser::fonts_in_collection(data).unwrap_or(1).min(64);
    let faces: Vec<ttf_parser::Face> = (0..n)
        .filter_map(|i| ttf_parser::Face::parse(data, i).ok())
        .collect();
    !faces.is_empty() && faces.len() == n as usize && faces.iter().all(embedding_allowed)
}

/// The OS/2 embedding bits (`fsType`) read as the specification has them:
/// restricted only when the restricted bit (0x0002) is set and no more
/// permissive one (preview and print 0x0004, editable 0x0008) is, so a
/// font that sets several takes the most permissive; the bitmap-only bit
/// (0x0200) counts from table version 2, where it was introduced; the
/// reserved bits are ignored (old fonts set bit 0, which means nothing).
/// A font without the table may be used (#313).
fn embedding_allowed(face: &ttf_parser::Face) -> bool {
    let Some(os2) = face.raw_face().table(ttf_parser::Tag::from_bytes(b"OS/2")) else {
        return true;
    };
    let u16_at = |i: usize| match os2.get(i..i + 2) {
        Some(&[hi, lo]) => u16::from_be_bytes([hi, lo]),
        _ => 0,
    };
    let (version, fs_type) = (u16_at(0), u16_at(8));
    let restricted = fs_type & 0x0002 != 0 && fs_type & 0x000c == 0;
    let bitmap_only = version >= 2 && fs_type & 0x0200 != 0;
    !restricted && !bitmap_only
}

/// Choose the instance of a variable font: the text's weight (see
/// [`target_weight`]: 700 for bold text, 400 for regular, the weight a
/// family name ends in; within the font's range), and its italic or slant axis
/// for italic text when the face is upright. Returns the weight and
/// whether the letters slant, which decide the synthetic styles, and the
/// axis values set (empty for a static font).
fn set_axes(
    face: &mut ttf_parser::Face,
    hb: &mut rustybuzz::Face,
    target: u16,
    italic: bool,
) -> (u16, bool, Vec<([u8; 4], f32)>) {
    let mut weight = face.weight().to_number();
    let mut slanted = face.is_italic() || face.is_oblique();
    if !face.is_variable() {
        return (weight, slanted, Vec::new());
    }
    let mut set: Vec<rustybuzz::Variation> = Vec::new();
    for axis in face.variation_axes() {
        let lo = axis.min_value.min(axis.max_value);
        let hi = axis.max_value.max(axis.min_value);
        match &axis.tag.to_bytes() {
            b"wght" => {
                let v = f32::from(target).clamp(lo, hi);
                set.push(rustybuzz::Variation {
                    tag: axis.tag,
                    value: v,
                });
                weight = v.round() as u16;
            }
            b"ital" if italic && !slanted && hi >= 1.0 => {
                set.push(rustybuzz::Variation {
                    tag: axis.tag,
                    value: 1.0,
                });
                slanted = true;
            }
            b"slnt" if italic && !slanted && lo < 0.0 => {
                set.push(rustybuzz::Variation {
                    tag: axis.tag,
                    value: lo.max(-12.0),
                });
                slanted = true;
            }
            _ => {}
        }
    }
    for v in &set {
        face.set_variation(v.tag, v.value);
    }
    hb.set_variations(&set);
    let coords = set.iter().map(|v| (v.tag.to_bytes(), v.value)).collect();
    (weight, slanted, coords)
}

/// Letters 0x20 to 0xFF of the symbol encoding of the Symbol family as
/// Unicode, from the public table that maps that encoding to Unicode
/// (0 where it has no character). Glyphs that are parts of larger signs
/// (brace and bracket pieces) take their Unicode piece characters.
const SYMBOL_ENCODING: [u16; 224] = [
    // 0x20
    0x0020, 0x0021, 0x2200, 0x0023, 0x2203, 0x0025, 0x0026, 0x220B, 0x0028, 0x0029, 0x2217, 0x002B,
    0x002C, 0x2212, 0x002E, 0x002F, // 0x30
    0x0030, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037, 0x0038, 0x0039, 0x003A, 0x003B,
    0x003C, 0x003D, 0x003E, 0x003F, // 0x40
    0x2245, 0x0391, 0x0392, 0x03A7, 0x0394, 0x0395, 0x03A6, 0x0393, 0x0397, 0x0399, 0x03D1, 0x039A,
    0x039B, 0x039C, 0x039D, 0x039F, // 0x50
    0x03A0, 0x0398, 0x03A1, 0x03A3, 0x03A4, 0x03A5, 0x03C2, 0x03A9, 0x039E, 0x03A8, 0x0396, 0x005B,
    0x2234, 0x005D, 0x22A5, 0x005F, // 0x60
    0x203E, 0x03B1, 0x03B2, 0x03C7, 0x03B4, 0x03B5, 0x03C6, 0x03B3, 0x03B7, 0x03B9, 0x03D5, 0x03BA,
    0x03BB, 0x03BC, 0x03BD, 0x03BF, // 0x70
    0x03C0, 0x03B8, 0x03C1, 0x03C3, 0x03C4, 0x03C5, 0x03D6, 0x03C9, 0x03BE, 0x03C8, 0x03B6, 0x007B,
    0x007C, 0x007D, 0x223C, 0, // 0x80 to 0x9F: none
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    // 0xA0
    0x20AC, 0x03D2, 0x2032, 0x2264, 0x2044, 0x221E, 0x0192, 0x2663, 0x2666, 0x2665, 0x2660, 0x2194,
    0x2190, 0x2191, 0x2192, 0x2193, // 0xB0
    0x00B0, 0x00B1, 0x2033, 0x2265, 0x00D7, 0x221D, 0x2202, 0x2022, 0x00F7, 0x2260, 0x2261, 0x2248,
    0x2026, 0x23D0, 0x23AF, 0x21B5, // 0xC0
    0x2135, 0x2111, 0x211C, 0x2118, 0x2297, 0x2295, 0x2205, 0x2229, 0x222A, 0x2283, 0x2287, 0x2284,
    0x2282, 0x2286, 0x2208, 0x2209, // 0xD0
    0x2220, 0x2207, 0x00AE, 0x00A9, 0x2122, 0x220F, 0x221A, 0x22C5, 0x00AC, 0x2227, 0x2228, 0x21D4,
    0x21D0, 0x21D1, 0x21D2, 0x21D3, // 0xE0
    0x25CA, 0x27E8, 0x00AE, 0x00A9, 0x2122, 0x2211, 0x239B, 0x239C, 0x239D, 0x23A1, 0x23A2, 0x23A3,
    0x23A7, 0x23A8, 0x23A9, 0x23AA, // 0xF0
    0, 0x27E9, 0x222B, 0x2320, 0x23AE, 0x2321, 0x239E, 0x239F, 0x23A0, 0x23A4, 0x23A5, 0x23A6,
    0x23AB, 0x23AC, 0x23AD, 0,
];

/// The family sets its letters in the symbol encoding (its text holds the
/// codes 0x20 to 0xFF, or the same in the private area at 0xF020, rather
/// than the characters they draw).
fn symbol_encoded(family: &str) -> bool {
    let key: String = family
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    matches!(key.as_str(), "symbol" | "symbolmt")
}

/// The glyph `face` draws for `c`: through its Unicode character map, or
/// for a font made for the symbol encoding (a Windows symbol map only)
/// the glyph of the code 0x20 to 0xFF typed, found at U+F020 to U+F0FF as
/// the system finds it (#469).
pub(crate) fn face_glyph(face: &ttf_parser::Face, c: char) -> Option<ttf_parser::GlyphId> {
    face.glyph_index(c).or_else(|| {
        let cp = c as u32;
        if !(0x20..=0xFF).contains(&cp) {
            return None;
        }
        face.tables()
            .cmap?
            .subtables
            .into_iter()
            .filter(|s| s.platform_id == ttf_parser::PlatformId::Windows && s.encoding_id == 0)
            .find_map(|s| s.glyph_index(0xF000 + cp).or_else(|| s.glyph_index(cp)))
            .filter(|g| g.0 != 0)
    })
}

/// A letter of a symbol-encoded family as the character it draws; others
/// unchanged.
pub fn symbol_to_unicode(c: char) -> char {
    let code = match c as u32 {
        n @ 0x20..=0xFF => n,
        n @ 0xF020..=0xF0FF => n - 0xF000,
        _ => return c,
    };
    match SYMBOL_ENCODING.get(code as usize - 0x20) {
        Some(&u) if u != 0 => char::from_u32(u as u32).unwrap_or(c),
        _ => c,
    }
}

/// The script a letter is written in, for cutting text into runs one
/// shaper takes: `None` for what every script shares (spaces, digits,
/// punctuation, marks), which stays with the letters around it. Only the
/// scripts whose shaping depends on it are told apart.
fn script_of(c: char) -> Option<u8> {
    Some(match c as u32 {
        0x0041..=0x005A | 0x0061..=0x007A | 0x00C0..=0x024F | 0x1E00..=0x1EFF => 1,
        0x0370..=0x03FF | 0x1F00..=0x1FFF => 2,
        0x0400..=0x052F => 3,
        0x0900..=0x0963 | 0x0966..=0x097F | 0xA8E0..=0xA8FF => 10,
        0x0980..=0x09FF => 11,
        0x0A00..=0x0A7F => 12,
        0x0A80..=0x0AFF => 13,
        0x0B00..=0x0B7F => 14,
        0x0B80..=0x0BFF => 15,
        0x0C00..=0x0C7F => 16,
        0x0C80..=0x0CFF => 17,
        0x0D00..=0x0D7F => 18,
        0x0D80..=0x0DFF => 19,
        0x0E00..=0x0E7F => 20,
        0x0E80..=0x0EFF => 21,
        0x0F00..=0x0FFF => 22,
        0x1000..=0x109F => 23,
        0x1780..=0x17FF => 24,
        _ => return None,
    })
}

/// A letter of a right-to-left script (Hebrew, Arabic and their forms,
/// Syriac, Thaana, N'Ko, and the historic and newer ones in the planes
/// above: Adlam, Hanifi Rohingya and others).
fn right_to_left(c: char) -> bool {
    matches!(
        c as u32,
        0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF
    )
}

/// `text` cut into runs of one direction, in the order they are seen,
/// each with whether it runs right to left: the Unicode bidirectional
/// algorithm, the text's first strong letter setting its direction. Text
/// without right-to-left letters is one left-to-right run.
fn bidi_runs(text: &str) -> Option<Vec<(Range<usize>, u8)>> {
    if !text.chars().any(right_to_left) {
        return None;
    }
    let info = unicode_bidi::BidiInfo::new(text, None);
    let mut out: Vec<(Range<usize>, u8)> = Vec::new();
    for (i, c) in text.char_indices() {
        let level = info.levels.get(i).map_or(0, |l| l.number());
        let end = i + c.len_utf8();
        match out.last_mut() {
            Some((r, l)) if *l == level => r.end = end,
            _ => out.push((i..end, level)),
        }
    }
    Some(out)
}

/// The order glyphs on a line are seen in, from their bidirectional
/// levels (in text order): from the highest level down to the lowest odd
/// one, every stretch at that level or above turns round. Spaces at the
/// line's end take the line's lowest level, as the paragraph's direction
/// does there. `None` when the line holds no right-to-left glyph (the
/// order is the text's).
fn display_order(glyphs: &[Glyph]) -> Option<Vec<usize>> {
    if glyphs.iter().all(|g| g.level == 0) {
        return None;
    }
    let base = glyphs.iter().map(|g| g.level).min().unwrap_or(0);
    let mut levels: Vec<u8> = glyphs.iter().map(|g| g.level).collect();
    for (l, g) in levels.iter_mut().zip(glyphs).rev() {
        if !g.ch.is_whitespace() {
            break;
        }
        *l = base;
    }
    let mut order: Vec<usize> = (0..glyphs.len()).collect();
    let top = levels.iter().copied().max().unwrap_or(0);
    let lowest_odd = levels
        .iter()
        .copied()
        .filter(|l| l % 2 == 1)
        .min()
        .unwrap_or(top + 1);
    let mut level = top;
    while level >= lowest_odd && level > 0 {
        let mut i = 0;
        while i < order.len() {
            if levels[order[i]] >= level {
                let start = i;
                while i < order.len() && levels[order[i]] >= level {
                    i += 1;
                }
                order[start..i].reverse();
            } else {
                i += 1;
            }
        }
        level -= 1;
    }
    Some(order)
}

/// `text` cut where its script changes (see [`script_of`]); what the
/// scripts share goes with the run before it, or the first one at the
/// start. Text with right-to-left letters stays whole: it is shaped as
/// one run as before.
fn script_runs(text: &str) -> Vec<Range<usize>> {
    let whole = || std::iter::once(0..text.len()).collect::<Vec<_>>();
    if text.is_ascii() || text.chars().any(right_to_left) {
        return whole();
    }
    let mut runs: Vec<(Range<usize>, Option<u8>)> = Vec::new();
    for (i, c) in text.char_indices() {
        let end = i + c.len_utf8();
        let sc = script_of(c);
        match runs.last_mut() {
            Some(last) if sc.is_none() || last.1.is_none() || last.1 == sc => {
                last.0.end = end;
                if last.1.is_none() {
                    last.1 = sc;
                }
            }
            _ => runs.push((i..end, sc)),
        }
    }
    if runs.len() < 2 {
        return whole();
    }
    runs.into_iter().map(|r| r.0).collect()
}

/// A conjunct linker: the virama of the scripts whose consonants it
/// joins into conjuncts and half forms (Devanagari, Bengali, Gujarati,
/// Oriya, Telugu, Malayalam: the Unicode property Indic_Conjunct_Break
/// = Linker).
fn linker(c: char) -> bool {
    matches!(
        c,
        '\u{094D}' | '\u{09CD}' | '\u{0ACD}' | '\u{0B4D}' | '\u{0C4D}' | '\u{0D4D}'
    )
}

/// A letter of the Indian scripts' blocks (U+0900 to U+0DFF).
fn indic_letter(c: char) -> bool {
    matches!(c as u32, 0x0900..=0x0DFF) && c.is_alphabetic()
}

/// `c` after `prev` belongs to the syllable `prev` is in: a combining
/// mark (vowel sign, virama, nukta, anusvara, accent), a joiner, a
/// consonant of the same script after a linker, or a letter of the
/// Indian scripts after a zero width joiner. Text is never cut for
/// shaping, and lines never break, before such a character: a syllable
/// shaped in pieces loses its half forms and conjuncts and shows its
/// virama (#599). Plain ASCII text has none (see [`complex_text`]).
fn continues_syllable(prev: char, c: char) -> bool {
    use unicode_general_category::{get_general_category, GeneralCategory as G};
    if c.is_ascii() {
        return false;
    }
    matches!(
        get_general_category(c),
        G::NonspacingMark | G::SpacingMark | G::EnclosingMark
    ) || matches!(c, '\u{200C}' | '\u{200D}')
        || (linker(prev) && indic_letter(c) && (prev as u32 >> 7) == (c as u32 >> 7))
        || (prev == '\u{200D}' && indic_letter(c))
}

/// Text in which a character may continue the syllable before it: any
/// text but plain ASCII, the only text the syllable checks are skipped
/// for (every frame of editing lays text out, #599).
fn complex_text(text: &str) -> bool {
    !text.is_ascii()
}

/// Characters drawn with the face of the letter before them: spaces,
/// controls, joiners, variation selectors and combining marks.
fn inherits_face(c: char) -> bool {
    c.is_whitespace()
        || c.is_control()
        || matches!(c,
            '\u{0300}'..='\u{036F}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{200B}'..='\u{200F}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FE20}'..='\u{FE2F}')
}

/// Families made for the script of `c`, best first, tried before any
/// other face when a span's font lacks the letter. Missing ones are
/// skipped (or asked for, when the application offers them).
fn script_fonts(c: char) -> &'static [&'static str] {
    match c as u32 {
        // Kana, then the CJK ideographs, punctuation and full-width forms.
        0x3040..=0x30FF | 0x31F0..=0x31FF => &[
            "Noto Sans CJK JP",
            "Noto Sans JP",
            "Source Han Sans JP",
            "IPAGothic",
            "IPAPGothic",
            "Yu Gothic",
            "Meiryo",
            "Hiragino Sans",
            "Noto Sans CJK SC",
            "Noto Sans SC",
        ],
        0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF => &[
            "Noto Sans CJK KR",
            "Noto Sans KR",
            "Source Han Sans KR",
            "Malgun Gothic",
            "NanumGothic",
            "Gulim",
            "Apple SD Gothic Neo",
            "UnDotum",
        ],
        0x2E80..=0x2FFF
        | 0x3000..=0x303F
        | 0x3100..=0x312F
        | 0x3190..=0x31EF
        | 0x3200..=0x9FFF
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE4F
        | 0xFF00..=0xFFEF
        | 0x20000..=0x3FFFF => &[
            "Noto Sans CJK SC",
            "Noto Sans SC",
            "Source Han Sans SC",
            "WenQuanYi Zen Hei",
            "WenQuanYi Micro Hei",
            "Droid Sans Fallback",
            "SimSun",
            "SimHei",
            "PingFang SC",
            "Hiragino Sans GB",
            "Noto Sans CJK TC",
            "Noto Sans TC",
            "Noto Sans CJK JP",
            "Noto Sans JP",
            "IPAGothic",
        ],
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => {
            &[
                "Noto Sans Arabic",
                "Noto Naskh Arabic",
                "DejaVu Sans",
                "Arial",
                "Tahoma",
                "Geeza Pro",
            ]
        }
        0x0590..=0x05FF | 0xFB1D..=0xFB4F => &[
            "Noto Sans Hebrew",
            "DejaVu Sans",
            "Arial",
            "Arial Hebrew",
            "Tahoma",
        ],
        0x0E00..=0x0E7F => &[
            "Noto Sans Thai",
            "Noto Sans Thai UI",
            "Tahoma",
            "Leelawadee UI",
            "Thonburi",
            "Garuda",
            "Loma",
        ],
        0x0900..=0x097F | 0xA8E0..=0xA8FF => &[
            "Noto Sans Devanagari",
            "Nirmala UI",
            "Mangal",
            "Lohit Devanagari",
        ],
        0x0980..=0x09FF => &["Noto Sans Bengali", "Nirmala UI", "Vrinda", "Lohit Bengali"],
        _ => &[
            "DejaVu Sans",
            "Noto Sans",
            "Liberation Sans",
            "FreeSans",
            "Noto Sans Symbols",
            "Noto Sans Symbols 2",
            "DejaVu Sans Mono",
        ],
    }
}

/// One shaped glyph, positioned relative to the pen of its line.
#[derive(Clone)]
struct Glyph {
    /// Outline at the pen origin (x = 0, baseline y = 0), mm.
    path: Arc<BezPath>,
    advance: f64,
    x_offset: f64,
    y_offset: f64,
    /// The character this glyph (cluster) starts; used for spaces and tabs.
    ch: char,
    /// Index of that character in the concatenated span text.
    char_idx: usize,
    /// One past the last character of its cluster.
    char_end: usize,
    /// Shaped right to left: its cluster's first character is at its
    /// right end.
    rtl: bool,
    /// Its bidirectional level (0 for left-to-right text, odd for right
    /// to left): lines hold glyphs in text order and are put in the order
    /// they are seen when placed (#628).
    level: u8,
    span: usize,
    /// No line breaks before this glyph: it is a further glyph of the
    /// cluster before it or continues its syllable (see
    /// [`continues_syllable`]).
    joins: bool,
    /// Width of a trailing hyphen if the line breaks after this glyph.
    hyphen_after: Option<Arc<HyphenGlyph>>,
    /// The font glyph behind the outline, for writers that keep text as
    /// text; `None` for markers without an outline.
    src: Option<Arc<GlyphSource>>,
}

/// Where a shaped glyph's outline comes from.
struct GlyphSource {
    font: tracedraw_core::document::text_outline::GlyphFont,
    gid: u16,
    /// From the glyph's space in ems to the outline at the pen origin
    /// (mm): size, baseline shift, slant and character angle.
    local: Affine,
    /// The characters of its cluster (empty for its further glyphs).
    text: String,
    /// The outline is the font's own glyph (not made bold by doubling).
    exact: bool,
}

#[derive(Clone)]
struct HyphenGlyph {
    path: BezPath,
    advance: f64,
}

/// Vertical metrics of a span's face at its size, mm.
#[derive(Clone, Copy, Default)]
struct Metrics {
    /// The font size, mm.
    size: f64,
    ascent: f64,
    descent: f64,
    line_height: f64,
    underline_pos: f64,
    underline_thick: f64,
    strike_pos: f64,
}

struct Line {
    glyphs: Vec<Glyph>,
    /// Trailing hyphen to draw when the line was broken inside a word.
    hyphen: Option<Arc<HyphenGlyph>>,
    metrics: Metrics,
    /// Last line of its paragraph (not justified).
    last_in_para: bool,
    para: usize,
    /// Index in the concatenated span text of the line's first character
    /// (the paragraph start for an empty line).
    start_char: usize,
    /// Where the line goes: its baseline, the pen x before alignment and
    /// the width it is aligned in (layout space, mm).
    y: f64,
    x0: f64,
    avail: f64,
    /// The left edge of its column (tab stops count from it).
    col_x: f64,
    /// Where the paragraph's drop cap and bullet go, on its first line.
    cap_x: Option<f64>,
    bullet_x: Option<f64>,
}

pub struct TextLayout {
    pub path: BezPath,
    /// Logical bounds, mm.
    pub bounds: Rect,
    pub glyphs: usize,
    /// True when paragraph text did not fit its frame.
    pub overflow: bool,
    /// Number of characters of the concatenated span text (in order, line
    /// breaks and tabs included) placed inside the frame before it
    /// overflowed. Equals the total character count for artistic text and
    /// for frames that hold all their text, so
    /// `tracedraw_core::split_spans_at(spans, fitted_chars)` gives the
    /// spans that stay in this frame and the spans that flow to the next.
    pub fitted_chars: usize,
    /// One entry per laid-out line: (x of the line start, baseline y), mm,
    /// in layout space (frame top or first baseline at y = 0, Y up).
    pub baselines: Vec<(f64, f64)>,
    /// One entry per laid-out line, in order: the character range it holds
    /// and the pen position before each of its characters (for carets).
    pub lines: Vec<LineBox>,
    /// The glyphs as fonts draw them and the other marks, filled in by
    /// [`FontSystem::glyph_layout`] only.
    glyph_layout: GlyphLayout,
    /// Vertical text: `path`, `bounds` and the glyphs are laid out in
    /// columns (down from the top right, columns going left), while
    /// `lines` and `baselines` describe the written lines as if
    /// horizontal; [`TextLayout::line_frame`] maps those into the columns.
    pub vertical: bool,
}

impl TextLayout {
    /// From the space of `lines` and `baselines` to the space of `path`:
    /// the identity, or the quarter turn of vertical text.
    pub fn line_frame(&self) -> Affine {
        if self.vertical {
            tracedraw_core::document::vertical_line_frame()
        } else {
            Affine::IDENTITY
        }
    }
}

/// Caret geometry of one laid-out line, mm, in layout space (Y up).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LineBox {
    /// Index of the line's first character in the concatenated span text.
    pub start_char: usize,
    /// One past the line's last character (the newline that ends a
    /// paragraph is not part of the line).
    pub end_char: usize,
    pub baseline: f64,
    pub ascent: f64,
    pub descent: f64,
    /// Pen x before each character from `start_char` to `end_char`, then
    /// after the last one: `end_char - start_char + 1` entries. The caret
    /// sits there; before a right-to-left character it is the right end.
    pub edges: Vec<f64>,
    /// The left x of each character's own glyph (a share of it for a
    /// cluster), `end_char - start_char` entries. Unlike `edges` it holds
    /// in right-to-left text too, where the glyphs run the other way and
    /// one character's end is not the next one's start (#474).
    pub starts: Vec<f64>,
    /// The right x of each character's own glyph, `end_char - start_char`
    /// entries: with `starts`, the stretch a selection highlights.
    pub ends: Vec<f64>,
}

impl LineBox {
    /// The left and right ends of the line's carets: right-to-left text
    /// runs its edges the other way.
    pub fn x_range(&self) -> (f64, f64) {
        let a = self.edges.iter().copied().reduce(f64::min).unwrap_or(0.0);
        let b = self.edges.iter().copied().reduce(f64::max).unwrap_or(a);
        (a, b)
    }
}

impl TextLayout {
    /// Caret position for the character index `idx`: (x, baseline, ascent,
    /// descent) in layout space. Past the end of the text the caret sits
    /// after the last character.
    pub fn caret(&self, idx: usize) -> Option<(f64, f64, f64, f64)> {
        let line = self
            .lines
            .iter()
            .find(|l| idx >= l.start_char && idx < l.end_char)
            .or_else(|| self.lines.iter().rev().find(|l| idx >= l.start_char))
            .or_else(|| self.lines.first())?;
        let i = idx
            .saturating_sub(line.start_char)
            .min(line.edges.len().saturating_sub(1));
        let x = line.edges.get(i).copied()?;
        Some((x, line.baseline, line.ascent, line.descent))
    }

    /// Character index nearest to the layout-space point `p`: the line
    /// whose vertical band holds `p.y` (the nearest one outside all bands;
    /// of the parts of a line split around a wrap object, the one nearest
    /// `p.x`) and the nearest character edge on that line.
    pub fn hit_char(&self, p: Point) -> Option<usize> {
        let x_gap = |l: &LineBox| {
            let (a, b) = l.x_range();
            if p.x < a {
                a - p.x
            } else if p.x > b {
                p.x - b
            } else {
                0.0
            }
        };
        let in_band = self
            .lines
            .iter()
            .filter(|l| p.y <= l.baseline + l.ascent && p.y >= l.baseline - l.descent)
            .min_by(|a, b| x_gap(a).total_cmp(&x_gap(b)));
        let line = in_band.or_else(|| {
            self.lines.iter().min_by(|a, b| {
                let da = (a.baseline - p.y).abs();
                let db = (b.baseline - p.y).abs();
                da.total_cmp(&db)
                    .then_with(|| x_gap(a).total_cmp(&x_gap(b)))
            })
        })?;
        let (i, _) = line.edges.iter().enumerate().min_by(|(_, a), (_, b)| {
            let da = (*a - p.x).abs();
            let db = (*b - p.x).abs();
            da.total_cmp(&db)
        })?;
        Some(line.start_char + i)
    }

    /// Index of the line holding the character index `idx`.
    pub fn line_of(&self, idx: usize) -> Option<usize> {
        self.lines
            .iter()
            .position(|l| idx >= l.start_char && idx < l.end_char)
            .or_else(|| self.lines.iter().rposition(|l| idx >= l.start_char))
    }
}

impl FontSystem {
    fn load() -> Self {
        let mut db = fontdb::Database::new();
        #[cfg(not(target_arch = "wasm32"))]
        db.load_system_fonts();
        #[cfg(windows)]
        installed::drop_unregistered(&mut db);
        if let Ok(extra) = EXTRA_FONTS.lock() {
            for data in extra.iter() {
                let source: Arc<dyn AsRef<[u8]> + Sync + Send> = data.clone();
                db.load_font_source(fontdb::Source::Binary(source));
            }
        }
        Self::from_db(db)
    }

    /// A font system with no fonts, for tests that install their own.
    pub fn empty() -> Self {
        Self::from_db(fontdb::Database::new())
    }

    /// A font system over `db` (tests start from an empty one).
    fn from_db(db: fontdb::Database) -> Self {
        let mut families: Vec<String> = db
            .faces()
            .flat_map(|f| f.families.iter().map(|(n, _)| n.clone()))
            .collect();
        let mut legacy: HashMap<String, Vec<fontdb::ID>> = HashMap::new();
        let ids: Vec<fontdb::ID> = db.faces().map(|f| f.id).collect();
        for id in ids {
            for name in legacy_families(&db, id) {
                legacy.entry(name).or_default().push(id);
            }
        }
        families.extend(legacy.keys().cloned());
        families.sort();
        families.dedup();
        log::info!(
            "loaded {} font faces, {} families",
            db.len(),
            families.len()
        );
        FontSystem {
            db: RwLock::new(db),
            faces: Mutex::new(HashMap::new()),
            families: RwLock::new(families),
            document_families: RwLock::new(Vec::new()),
            carried: RwLock::new(Vec::new()),
            cache: Mutex::new(HashMap::new()),
            offered: RwLock::new(Vec::new()),
            requests: Mutex::new((Vec::new(), Vec::new())),
            fallbacks: Mutex::new(HashMap::new()),
            generation: AtomicU64::new(0),
            legacy: RwLock::new(legacy),
        }
    }

    /// Note the legacy family names of faces just loaded (see
    /// [`FontSystem::legacy`]); returns them.
    fn register_legacy(&self, ids: &[fontdb::ID]) -> Vec<String> {
        let mut names = Vec::new();
        {
            let db = self.db();
            let mut legacy = self.legacy.write().unwrap_or_else(|e| e.into_inner());
            for id in ids {
                for name in legacy_families(&db, *id) {
                    let faces = legacy.entry(name.clone()).or_default();
                    if !faces.contains(id) {
                        faces.push(*id);
                    }
                    names.push(name);
                }
            }
        }
        names.sort();
        names.dedup();
        if !names.is_empty() {
            let mut families = self.families.write().unwrap_or_else(|e| e.into_inner());
            families.extend(names.iter().cloned());
            families.sort();
            families.dedup();
        }
        names
    }

    /// The face of a legacy family name nearest the weight and style.
    fn legacy_face(
        &self,
        db: &fontdb::Database,
        family: &str,
        weight: fontdb::Weight,
        style: fontdb::Style,
    ) -> Option<fontdb::ID> {
        let legacy = self.legacy.read().unwrap_or_else(|e| e.into_inner());
        // Names differ in case between drawings and fonts ("SemiBold" and
        // "Semibold").
        let faces = legacy.get(family).or_else(|| {
            legacy
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(family))
                .map(|(_, v)| v)
        })?;
        faces
            .iter()
            .filter_map(|id| db.face(*id).map(|f| (*id, f)))
            .min_by_key(|(_, f)| {
                let off = (i32::from(f.weight.0) - i32::from(weight.0)).abs();
                off + if f.style == style { 0 } else { 1000 }
            })
            .map(|(id, _)| id)
    }

    /// Fonts were added: drop what was laid out or chosen without them.
    fn fonts_changed(&self) {
        if let Ok(mut c) = self.cache.lock() {
            c.clear();
        }
        if let Ok(mut f) = self.fallbacks.lock() {
            f.clear();
        }
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    /// Changes each time fonts are added or offered: a view drawn before
    /// may now draw its text with other faces.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }

    /// Families the application can fetch on request (the browser build's
    /// font folder); installed ones are left out. Text set in one of them,
    /// in a family one of them stands in for, or holding letters only one
    /// of them has, asks for it while it draws with what is installed:
    /// [`FontSystem::take_font_requests`] hands the requests over and
    /// [`FontSystem::add_fonts`] brings the files in.
    pub fn offer_families(&self, families: &[String]) {
        let installed = self.families();
        let mut offered: Vec<String> = families
            .iter()
            .filter(|f| !f.is_empty() && !installed.contains(f))
            .cloned()
            .collect();
        offered.sort();
        offered.dedup();
        *self.offered.write().unwrap_or_else(|e| e.into_inner()) = offered;
        self.fonts_changed();
    }

    /// The families installed and the families offered, sorted: what a
    /// font list shows.
    pub fn available_families(&self) -> Vec<String> {
        let mut all = self.families();
        all.extend(
            self.offered
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .cloned(),
        );
        all.sort();
        all.dedup();
        all
    }

    /// The offered families text asked for since the last call, each once
    /// in the session.
    pub fn take_font_requests(&self) -> Vec<String> {
        let mut r = self.requests.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut r.0)
    }

    /// Ask for `family` when it is offered (once); true when it is offered.
    fn request_offered(&self, family: &str) -> bool {
        let offered = self
            .offered
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .binary_search_by(|f| f.as_str().cmp(family))
            .is_ok();
        if offered {
            let mut r = self.requests.lock().unwrap_or_else(|e| e.into_inner());
            if !r.1.iter().any(|f| f == family) {
                r.1.push(family.to_string());
                r.0.push(family.to_string());
            }
        }
        offered
    }

    /// Text set in `family` asks for the family, or for the first of its
    /// stand-ins, when that is offered and nothing before it is installed.
    fn ask_for(&self, family: &str) {
        if self
            .offered
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
        {
            return;
        }
        // A family named with its weight ("Titillium Web SemiBold") is
        // fetched as its family, and stands in as that family would.
        // The name itself first: "Archivo Black" is a family of its own
        // that "Archivo" does not stand for.
        if self.has_exact_family(family) || self.request_offered(family) {
            return;
        }
        let base = split_weight(family).map(|(b, _)| b);
        for name in base.into_iter().chain(substitutes(base.unwrap_or(family))) {
            if self.has_exact_family(name) || self.request_offered(name) {
                return;
            }
        }
    }

    /// Whether a family of exactly this name (or legacy name) is loaded.
    fn has_exact_family(&self, family: &str) -> bool {
        self.families
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|f| f == family)
    }

    /// Load font files the application supplies (fetched after
    /// [`FontSystem::take_font_requests`]) for the rest of the session;
    /// files that do not read as fonts are skipped. Returns the families
    /// they added.
    pub fn add_fonts(&self, files: Vec<Vec<u8>>) -> Vec<String> {
        let mut added = Vec::new();
        let mut loaded = Vec::new();
        let mut db = self.db.write().unwrap_or_else(|e| e.into_inner());
        for data in files {
            let source = fontdb::Source::Binary(Arc::new(data));
            let ids = db.load_font_source(source);
            for id in ids {
                if let Some(face) = db.face(id) {
                    added.extend(face.families.iter().map(|(n, _)| n.clone()));
                    loaded.push(id);
                }
            }
        }
        drop(db);
        added.extend(self.register_legacy(&loaded));
        added.sort();
        added.dedup();
        if added.is_empty() {
            return added;
        }
        let mut families = self.families.write().unwrap_or_else(|e| e.into_inner());
        families.extend(added.iter().cloned());
        families.sort();
        families.dedup();
        drop(families);
        self.offered
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|f| !added.contains(f));
        self.fonts_changed();
        log::info!("fonts added: {}", added.join(", "));
        added
    }

    /// The font database to read (a lock poisoned by a panic elsewhere
    /// still holds usable data).
    fn db(&self) -> RwLockReadGuard<'_, fontdb::Database> {
        self.db.read().unwrap_or_else(|e| e.into_inner())
    }

    /// Load the fonts a document carries (fonts embedded in the file it
    /// came from) for the rest of the session, so its text keeps its
    /// letters. A font is used when its licence allows use in documents
    /// (not "restricted", outlines allowed) and it brings a family that
    /// is not installed: an installed font wins over an embedded copy,
    /// which may hold only the letters the document needed. Returns the
    /// families added.
    pub fn add_document_fonts(&self, fonts: &[DocumentFont]) -> Vec<String> {
        let mut added = Vec::new();
        for f in fonts {
            added.extend(self.add_document_font(f));
        }
        if !added.is_empty() {
            // Outlines laid out with a substitute for these families.
            self.fonts_changed();
            log::info!("document fonts loaded: {}", added.join(", "));
        }
        added
    }

    fn add_document_font(&self, font: &DocumentFont) -> Vec<String> {
        if !usable_in_documents(&font.data) {
            log::warn!(
                "font {:?} carried by the document is not usable (damaged, or its licence does not allow it)",
                font.family
            );
            return Vec::new();
        }
        let source = fontdb::Source::Binary(font.data.clone());
        let mut probe = fontdb::Database::new();
        probe.load_font_source(source.clone());
        // A face is new when no loaded face has its family, weight and
        // style: a drawing brings the bold face of a family it also brings
        // (or that is installed) in regular, and both serve its text.
        let faces: Vec<(Vec<String>, fontdb::Weight, fontdb::Style)> = probe
            .faces()
            .map(|f| {
                let names = f.families.iter().map(|(n, _)| n.clone()).collect();
                (names, f.weight, f.style)
            })
            .collect();
        // Checked and loaded under one lock, so two documents bringing the
        // same font load it once.
        let mut db = self.db.write().unwrap_or_else(|e| e.into_inner());
        let mut new: Vec<String> = faces
            .iter()
            .filter(|(names, weight, style)| {
                !db.faces().any(|g| {
                    g.weight == *weight
                        && g.style == *style
                        && g.families.iter().any(|(m, _)| names.contains(m))
                })
            })
            .flat_map(|(names, _, _)| names.iter().cloned())
            .collect();
        new.sort();
        new.dedup();
        if new.is_empty() {
            return Vec::new();
        }
        let loaded = db.load_font_source(source);
        if loaded.is_empty() {
            return Vec::new();
        }
        drop(db);
        let before = self.families();
        let legacy_names = self.register_legacy(&loaded);
        new.extend(legacy_names);
        new.sort();
        new.dedup();
        let mut families = self.families.write().unwrap_or_else(|e| e.into_inner());
        // Families nothing installed or brought before: the drawing's own.
        let own: Vec<String> = new
            .iter()
            .filter(|n| !before.contains(n))
            .cloned()
            .collect();
        families.extend(new.iter().cloned());
        families.sort();
        families.dedup();
        drop(families);
        let mut doc = self
            .document_families
            .write()
            .unwrap_or_else(|e| e.into_inner());
        doc.extend(own);
        doc.sort();
        doc.dedup();
        drop(doc);
        self.carried
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push((new.clone(), font.clone()));
        new
    }

    /// `doc` carrying, besides its own fonts, the fonts other documents
    /// brought for the families its text uses (text imported or pasted
    /// from them), so the saved file keeps its letters where those fonts
    /// are not installed.
    pub fn with_carried_fonts<'d>(
        &self,
        doc: &'d tracedraw_core::Document,
    ) -> std::borrow::Cow<'d, tracedraw_core::Document> {
        let carried = self.carried.read().unwrap_or_else(|e| e.into_inner());
        if carried.is_empty() {
            return std::borrow::Cow::Borrowed(doc);
        }
        let mut add: Vec<DocumentFont> = Vec::new();
        for family in doc.font_families_used() {
            let Some((_, font)) = carried.iter().find(|(f, _)| f.contains(&family)) else {
                continue;
            };
            let has = |f: &DocumentFont| Arc::ptr_eq(&f.data, &font.data) || f.data == font.data;
            if !doc.fonts.iter().any(has) && !add.iter().any(has) {
                add.push(font.clone());
            }
        }
        if add.is_empty() {
            return std::borrow::Cow::Borrowed(doc);
        }
        let mut out = doc.clone();
        out.fonts.extend(add);
        std::borrow::Cow::Owned(out)
    }

    /// The installed fonts the drawing's text uses, as fonts for it to
    /// carry (Save As's Embed fonts): one file per face the text uses
    /// (the family's regular face standing in for a bold or italic one it
    /// lacks), only where the face's licence allows use in documents
    /// (`usable_in_documents`), leaving out faces in collection files,
    /// families that came with a document (`with_carried_fonts` keeps
    /// those) and files the drawing already carries.
    pub fn fonts_to_embed(&self, doc: &tracedraw_core::Document) -> Vec<DocumentFont> {
        let mut out: Vec<DocumentFont> = Vec::new();
        for (family, bold, italic) in doc.font_faces_used() {
            if self.is_document_family(&family) {
                continue;
            }
            let Some(data) = self.face_file(&family, bold, italic).or_else(|| {
                (bold || italic)
                    .then(|| self.face_file(&family, false, false))
                    .flatten()
            }) else {
                continue;
            };
            let same = |f: &DocumentFont| Arc::ptr_eq(&f.data, &data) || f.data == data;
            if doc.fonts.iter().any(same) || out.iter().any(same) {
                continue;
            }
            if !usable_in_documents(&data) {
                continue;
            }
            out.push(DocumentFont {
                family: family.clone(),
                data,
            });
        }
        out
    }

    /// `doc` carrying, besides its own fonts, the installed fonts its
    /// text uses that may be embedded (`fonts_to_embed`), so the saved
    /// file keeps its letters on machines without them.
    pub fn with_embedded_fonts<'d>(
        &self,
        doc: &'d tracedraw_core::Document,
    ) -> std::borrow::Cow<'d, tracedraw_core::Document> {
        let add = self.fonts_to_embed(doc);
        if add.is_empty() {
            return std::borrow::Cow::Borrowed(doc);
        }
        let mut out = doc.clone();
        out.fonts.extend(add);
        std::borrow::Cow::Owned(out)
    }

    /// The family came with a document (it is not installed).
    pub fn is_document_family(&self, family: &str) -> bool {
        self.document_families
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|f| f == family)
    }

    /// The embedding bits (OS/2 `fsType`) of the face that draws `family`
    /// in this style (a stand-in when the family is not installed);
    /// `None` without such a face or without the bits.
    pub fn embedding_bits(&self, family: &str, bold: bool, italic: bool) -> Option<u16> {
        let id = self.face_for(family, bold, italic)?;
        let (data, index) = self.face_data(id)?;
        let face = ttf_parser::RawFace::parse(&data, index).ok()?;
        match face
            .table(ttf_parser::Tag::from_bytes(b"OS/2"))?
            .get(8..10)?
        {
            &[hi, lo] => Some(u16::from_be_bytes([hi, lo])),
            _ => None,
        }
    }

    /// The font file of `family`'s face in that weight and slant, for
    /// embedding in a saved drawing: `None` when the family lacks that
    /// face (bold or italic drawn from another) or the face sits in a
    /// collection file.
    pub fn face_file(&self, family: &str, bold: bool, italic: bool) -> Option<Arc<Vec<u8>>> {
        let db = self.db();
        let id = db.query(&fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            weight: if bold {
                fontdb::Weight::BOLD
            } else {
                fontdb::Weight::NORMAL
            },
            stretch: fontdb::Stretch::Normal,
            style: if italic {
                fontdb::Style::Italic
            } else {
                fontdb::Style::Normal
            },
        })?;
        let face = db.face(id)?;
        let is_bold = face.weight.0 >= 600;
        let is_italic = face.style != fontdb::Style::Normal;
        if is_bold != bold || is_italic != italic {
            return None;
        }
        db.with_face_data(id, |d, i| {
            (i == 0 && ttf_parser::fonts_in_collection(d).is_none()).then(|| Arc::new(d.to_vec()))
        })
        .flatten()
    }

    /// Raw file data and face index of the first installed family in
    /// `candidates`, for registering UI fallback fonts (scripts the bundled
    /// UI font lacks: CJK, Arabic, Devanagari, Bengali...).
    pub fn first_face_data(&self, candidates: &[&str]) -> Option<(String, Arc<Vec<u8>>, u32)> {
        let db = self.db();
        for name in candidates {
            let Some(id) = db.query(&fontdb::Query {
                families: &[fontdb::Family::Name(name)],
                weight: fontdb::Weight::NORMAL,
                stretch: fontdb::Stretch::Normal,
                style: fontdb::Style::Normal,
            }) else {
                continue;
            };
            let got = db.with_face_data(id, |d, i| (Arc::new(d.to_vec()), i));
            if let Some((data, index)) = got {
                return Some((name.to_string(), data, index));
            }
        }
        None
    }

    /// The characters `family`'s regular face draws, in code point order
    /// (empty when the family is unknown). For the Glyphs docker.
    pub fn char_coverage(&self, family: &str) -> Vec<char> {
        let Some(id) = self.face_for(family, false, false) else {
            return Vec::new();
        };
        let Some((data, index)) = self.face_data(id) else {
            return Vec::new();
        };
        let Ok(face) = ttf_parser::Face::parse(&data, index) else {
            return Vec::new();
        };
        let mut out: Vec<char> = Vec::new();
        if let Some(cmap) = face.tables().cmap {
            for sub in cmap.subtables {
                // A symbol map lists its codes as the letters typed for
                // them (U+F041 is "A", Alt+0065).
                let symbol =
                    sub.platform_id == ttf_parser::PlatformId::Windows && sub.encoding_id == 0;
                if !sub.is_unicode() && !symbol {
                    continue;
                }
                sub.codepoints(|cp| {
                    let typed = match cp {
                        _ if !symbol => cp,
                        0xF020..=0xF0FF => cp - 0xF000,
                        0x20..=0xFF => cp,
                        _ => return,
                    };
                    if let Some(c) = char::from_u32(typed) {
                        if !c.is_control() && sub.glyph_index(cp).is_some_and(|g| g.0 != 0) {
                            out.push(c);
                        }
                    }
                });
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The OpenType features among `candidates` that draw `ch` with
    /// another glyph in `family` (its alternates, for the Glyphs docker).
    pub fn glyph_variants(&self, family: &str, ch: char, candidates: &[&str]) -> Vec<String> {
        let Some(id) = self.face_for(family, false, false) else {
            return Vec::new();
        };
        let Some((data, index)) = self.face_data(id) else {
            return Vec::new();
        };
        let Some(hb) = rustybuzz::Face::from_slice(&data, index) else {
            return Vec::new();
        };
        let shape = |features: &[rustybuzz::Feature]| -> Vec<u32> {
            let mut buffer = rustybuzz::UnicodeBuffer::new();
            buffer.push_str(&ch.to_string());
            buffer.guess_segment_properties();
            rustybuzz::shape(&hb, features, buffer)
                .glyph_infos()
                .iter()
                .map(|g| g.glyph_id)
                .collect()
        };
        let plain = shape(&[]);
        candidates
            .iter()
            .filter(|tag| {
                let t = rustybuzz::ttf_parser::Tag::from_bytes_lossy(tag.as_bytes());
                shape(&[rustybuzz::Feature::new(t, 1, ..)]) != plain
            })
            .map(|t| t.to_string())
            .collect()
    }

    /// Sorted list of the family names available: those installed on
    /// this machine and those documents brought.
    pub fn families(&self) -> Vec<String> {
        self.families
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// What a font list shows about `family`: how its fonts are built,
    /// its faces with their style names, whether it came with a document
    /// and whether it is a symbol font. `None` when it is neither installed
    /// nor offered.
    pub fn family_info(&self, family: &str) -> Option<FamilyInfo> {
        let ids: Vec<fontdb::ID> = {
            let db = self.db();
            let mut ids: Vec<fontdb::ID> = db
                .faces()
                .filter(|f| f.families.iter().any(|(n, _)| n == family))
                .map(|f| f.id)
                .collect();
            if ids.is_empty() {
                let legacy = self.legacy.read().unwrap_or_else(|e| e.into_inner());
                ids = legacy.get(family).cloned().unwrap_or_default();
            }
            ids
        };
        if ids.is_empty() {
            let offered = self
                .offered
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .binary_search_by(|f| f.as_str().cmp(family))
                .is_ok();
            return offered.then(|| FamilyInfo {
                kind: FontKind::Offered,
                faces: Vec::new(),
                embedded: false,
                symbol: false,
            });
        }
        let db = self.db();
        let mut kind = FontKind::TrueType;
        let mut symbol = false;
        let mut faces: Vec<FaceStyle> = Vec::new();
        for id in ids {
            let Some(info) = db.face(id) else {
                continue;
            };
            let read = db.with_face_data(id, |data, index| {
                let f = ttf_parser::Face::parse(data, index).ok()?;
                let tables = f.tables();
                let face_kind = if tables.fvar.is_some() {
                    FontKind::Variable
                } else if tables.cff.is_some() || tables.cff2.is_some() {
                    FontKind::OpenType
                } else {
                    FontKind::TrueType
                };
                let symbol = tables.cmap.is_some_and(|c| {
                    c.subtables.into_iter().any(|s| {
                        s.platform_id == ttf_parser::PlatformId::Windows && s.encoding_id == 0
                    })
                });
                let style = face_style_name(&f);
                Some((face_kind, symbol, style))
            });
            let Some(Some((face_kind, face_symbol, style))) = read else {
                continue;
            };
            // A family is as its most capable face shows it.
            kind = match (kind, face_kind) {
                (FontKind::Variable, _) | (_, FontKind::Variable) => FontKind::Variable,
                (FontKind::OpenType, _) | (_, FontKind::OpenType) => FontKind::OpenType,
                _ => FontKind::TrueType,
            };
            symbol |= face_symbol;
            let italic = info.style != fontdb::Style::Normal;
            let name = style.unwrap_or_else(|| {
                match (info.weight.0 >= 600, italic) {
                    (false, false) => "Regular",
                    (false, true) => "Italic",
                    (true, false) => "Bold",
                    (true, true) => "Bold Italic",
                }
                .to_string()
            });
            if !faces.iter().any(|f| f.name == name) {
                faces.push(FaceStyle {
                    name,
                    weight: info.weight.0,
                    italic,
                });
            }
        }
        faces.sort_by_key(|f| (f.italic, f.weight));
        Some(FamilyInfo {
            kind,
            faces,
            embedded: self.is_document_family(family),
            symbol,
        })
    }

    /// Ask for an offered family now (a font list showing it), as text set
    /// in it would; true when it is offered. The application fetches it
    /// after [`FontSystem::take_font_requests`].
    pub fn request_family(&self, family: &str) -> bool {
        self.request_offered(family)
    }

    /// Whether a family exists (missing fonts are substituted and reported).
    pub fn has_family(&self, family: &str) -> bool {
        let families = self.families.read().unwrap_or_else(|e| e.into_inner());
        families.iter().any(|f| f == family)
            // A face named by its family and weight ("Titillium Web
            // SemiBold") is drawn by the family.
            || split_weight(family).is_some_and(|(base, _)| families.iter().any(|f| f == base))
    }

    /// Pick a face for a span: the family itself, else a stand-in (see
    /// [`substitutes`]), else a sans-serif, else anything.
    fn face_for(&self, family: &str, bold: bool, italic: bool) -> Option<fontdb::ID> {
        self.face_and_weight(family, bold, italic).map(|(id, _)| id)
    }

    /// [`FontSystem::face_for`] with the weight the text is to be drawn
    /// at: 400 or 700 when a face lists `family` itself ("Archivo Black"
    /// is a regular face of its own family), else the weight the name ends
    /// in (see [`target_weight`]), for the family without it or a
    /// stand-in.
    fn face_and_weight(&self, family: &str, bold: bool, italic: bool) -> Option<(fontdb::ID, u16)> {
        self.ask_for(family);
        let plain = if bold { 700 } else { 400 };
        let weight = fontdb::Weight(plain);
        let style = if italic {
            fontdb::Style::Italic
        } else {
            fontdb::Style::Normal
        };
        let db = self.db();
        let exact = fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            weight,
            stretch: fontdb::Stretch::Normal,
            style,
        };
        if let Some(id) = db
            .query(&exact)
            .or_else(|| self.legacy_face(&db, family, weight, style))
        {
            return Some((id, plain));
        }
        let target = target_weight(family, bold);
        let weight = fontdb::Weight(target);
        // A family named with its weight ("Titillium Web SemiBold") whose
        // faces are not listed under that name: the face of the family
        // nearest that weight (bold text at least bold).
        let base = split_weight(family).map(|(b, _)| b);
        if let Some(base) = base {
            let q = fontdb::Query {
                families: &[fontdb::Family::Name(base)],
                weight,
                stretch: fontdb::Stretch::Normal,
                style,
            };
            if let Some(id) = db
                .query(&q)
                .or_else(|| self.legacy_face(&db, base, weight, style))
            {
                return Some((id, target));
            }
        }
        let mut fams: Vec<fontdb::Family> = substitutes(base.unwrap_or(family))
            .into_iter()
            .map(fontdb::Family::Name)
            .collect();
        fams.extend([
            fontdb::Family::Name("DejaVu Sans"),
            fontdb::Family::Name("Arial"),
            fontdb::Family::SansSerif,
        ]);
        let q = fontdb::Query {
            families: &fams,
            weight,
            stretch: fontdb::Stretch::Normal,
            style,
        };
        db.query(&q)
            .or_else(|| db.faces().next().map(|f| f.id))
            .map(|id| (id, target))
    }

    /// The installed family that draws text set in `family` when that is
    /// not installed: the first of [`substitutes`] that is, else the
    /// general fallback. `None` when `family` is installed (or no font is).
    pub fn substitute(&self, family: &str) -> Option<String> {
        if self.has_family(family) {
            return None;
        }
        let id = self.face_for(family, false, false)?;
        let db = self.db();
        let face = db.face(id)?;
        face.families.first().map(|(name, _)| name.clone())
    }

    fn face_data(&self, id: fontdb::ID) -> Option<(Arc<Vec<u8>>, u32)> {
        let mut cache = self.faces.lock().ok()?;
        let db = self.db();
        if let Some(d) = cache.get(&id) {
            let idx = db.face(id)?.index;
            return Some((d.clone(), idx));
        }
        let (data, idx) = db.with_face_data(id, |d, i| (Arc::new(d.to_vec()), i))?;
        cache.insert(id, data.clone());
        Some((data, idx))
    }

    /// The key of a text object's outline in the cache: everything its
    /// layout depends on.
    fn cache_key(req: &TextRequest) -> String {
        format!(
            "{:?}|{:?}|{:?}|{}|{}",
            req.frame,
            req.align,
            req.para,
            req.on_path
                .map(|p| {
                    // The whole path, not just its size: two ellipses have
                    // the same number of elements.
                    use std::hash::{Hash, Hasher};
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    for el in p.path.elements() {
                        for pt in [el.end_point()].into_iter().flatten().chain(match el {
                            tracedraw_core::geometry::PathEl::QuadTo(a, _) => vec![*a],
                            tracedraw_core::geometry::PathEl::CurveTo(a, b, _) => vec![*a, *b],
                            _ => Vec::new(),
                        }) {
                            pt.x.to_bits().hash(&mut h);
                            pt.y.to_bits().hash(&mut h);
                        }
                        std::mem::discriminant(el).hash(&mut h);
                    }
                    format!(
                        "{:x}|{}|{}|{}|{}|{}|{:?}|{:?}",
                        h.finish(),
                        p.offset,
                        p.distance,
                        p.mirror,
                        p.mirror_h,
                        p.mirror_v,
                        p.orientation,
                        p.vertical
                    )
                })
                .unwrap_or_default(),
            req.spans
                .iter()
                .map(|s| format!(
                    "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}|{}",
                    s.layout_only,
                    s.font_family,
                    s.size_pt,
                    s.bold,
                    s.italic,
                    s.tracking_pct,
                    s.baseline_shift_pt,
                    s.offset_x_pct,
                    s.angle_deg,
                    s.underline,
                    s.strikethrough,
                    s.features,
                    s.paragraph,
                    s.text
                ))
                .collect::<Vec<_>>()
                .join("\u{1}")
        )
    }

    /// Cached outline and its box; shaping is expensive and the same text
    /// is asked for every frame.
    fn cached(&self, req: &TextRequest) -> CachedOutline {
        let key = Self::cache_key(req);
        if let Ok(c) = self.cache.lock() {
            if let Some(hit) = c.get(&key) {
                return hit.clone();
            }
        }
        let p = Arc::new(self.layout(req).path);
        let b = (!p.elements().is_empty()).then(|| p.bounding_box());
        if let Ok(mut c) = self.cache.lock() {
            if c.len() > 2000 {
                c.clear();
            }
            c.insert(key, (p.clone(), b));
        }
        (p, b)
    }

    /// Cached outline; shaping is expensive and the same text is asked for
    /// every frame.
    pub fn outline_cached(&self, req: &TextRequest) -> Arc<BezPath> {
        self.cached(req).0
    }

    /// The box of the cached outline, without copying the outline (#802);
    /// `None` for an empty one.
    pub fn bounds_cached(&self, req: &TextRequest) -> Option<Rect> {
        self.cached(req).1
    }

    /// Simple entry point: artistic text, left aligned.
    pub fn outline(&self, spans: &[TextSpan]) -> TextLayout {
        let para = ParagraphStyle::default();
        self.layout(&TextRequest {
            spans,
            frame: None,
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        })
    }

    /// Measure the advance width of a string in a span's style, mm.
    pub fn measure(&self, style: &TextSpan, text: &str) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        let span = TextSpan {
            text: text.to_string(),
            ..style.clone()
        };
        self.outline(&[span]).bounds.width()
    }

    // ----- shaping -----------------------------------------------------------

    fn metrics(&self, span: &TextSpan) -> Metrics {
        let Some(id) = self.face_for(&span.font_family, span.bold, span.italic) else {
            let s = span.size_pt * PT_MM;
            return Metrics {
                size: s,
                ascent: s * 0.8,
                descent: s * 0.2,
                line_height: s * 1.2,
                underline_pos: -s * 0.1,
                underline_thick: s * 0.05,
                strike_pos: s * 0.3,
            };
        };
        let Some((data, index)) = self.face_data(id) else {
            return Metrics::default();
        };
        let Ok(face) = ttf_parser::Face::parse(&data, index) else {
            return Metrics::default();
        };
        let scale = span.size_pt * PT_MM / face.units_per_em() as f64;
        let asc = face.ascender() as f64 * scale;
        let desc = -(face.descender() as f64) * scale;
        let gap = face.line_gap() as f64 * scale;
        let ul = face.underline_metrics();
        let st = face.strikeout_metrics();
        Metrics {
            size: span.size_pt * PT_MM,
            ascent: asc,
            descent: desc,
            line_height: asc + desc + gap,
            underline_pos: ul.map(|u| u.position as f64 * scale).unwrap_or(-desc * 0.5),
            underline_thick: ul
                .map(|u| u.thickness as f64 * scale)
                .unwrap_or(span.size_pt * PT_MM * 0.05),
            strike_pos: st.map(|s| s.position as f64 * scale).unwrap_or(asc * 0.35),
        }
    }

    /// Shape one run of text in one span's style into glyphs, with the
    /// paragraph's character and word spacing. Letters the span's face
    /// lacks are drawn with a face that has them.
    fn shape_run(
        &self,
        span: &TextSpan,
        span_idx: usize,
        text: &str,
        size_scale: f64,
        char_base: usize,
        para: &ParagraphStyle,
    ) -> Vec<Glyph> {
        let mut out = Vec::new();
        if text.is_empty() {
            return out;
        }
        let Some(id) = self.face_for(&span.font_family, span.bold, span.italic) else {
            return out;
        };
        // Text in the symbol encoding drawn without that font: the
        // characters its codes stand for, one for one.
        let mapped: String;
        let text = if symbol_encoded(&span.font_family) && !self.has_symbol_cmap(id) {
            mapped = text.chars().map(symbol_to_unicode).collect();
            mapped.as_str()
        } else {
            text
        };
        // The runs of each direction, in text order, each with its level
        // (#628); text with no right-to-left letter is one run whose
        // direction harfbuzz takes from its script.
        let runs = bidi_runs(text);
        let runs: Vec<(Range<usize>, Option<u8>)> = match runs {
            Some(r) => r.into_iter().map(|(r, l)| (r, Some(l))).collect(),
            None => vec![(0..text.len(), None)],
        };
        for (run, level) in runs {
            let rtl = level.is_some_and(|l| l % 2 == 1);
            for (range, face) in self.coverage_runs(id, &text[run.clone()], span.bold, span.italic)
            {
                let range = run.start + range.start..run.start + range.end;
                // Each script on its own: one buffer takes the script of
                // its first letter, so Hindi after a Latin word lost its
                // half forms and conjuncts (#599).
                let parts: Vec<Range<usize>> = if rtl {
                    std::iter::once(0..range.len()).collect()
                } else {
                    script_runs(&text[range.clone()])
                };
                for part in parts {
                    let part = range.start + part.start..range.start + part.end;
                    let before = text[..part.start].chars().count();
                    out.extend(self.shape_with(
                        face,
                        span,
                        span_idx,
                        &text[part],
                        size_scale,
                        char_base + before,
                        para,
                        level,
                    ));
                }
            }
        }
        out
    }

    /// The face maps codes through a symbol character map (Windows
    /// platform, encoding 0), as fonts made for the symbol encoding do.
    fn has_symbol_cmap(&self, id: fontdb::ID) -> bool {
        let Some((data, index)) = self.face_data(id) else {
            return false;
        };
        ttf_parser::Face::parse(&data, index).is_ok_and(|f| {
            f.tables().cmap.is_some_and(|c| {
                c.subtables
                    .into_iter()
                    .any(|s| s.platform_id == ttf_parser::PlatformId::Windows && s.encoding_id == 0)
            })
        })
    }

    /// `text` cut into runs that one face draws: `primary` where it has
    /// the letters, else the face [`FontSystem::fallback_face`] finds.
    /// Spaces, controls and combining marks stay with the run before them.
    fn coverage_runs(
        &self,
        primary: fontdb::ID,
        text: &str,
        bold: bool,
        italic: bool,
    ) -> Vec<(Range<usize>, fontdb::ID)> {
        let whole = vec![(0..text.len(), primary)];
        let Some((data, index)) = self.face_data(primary) else {
            return whole;
        };
        let Ok(face) = ttf_parser::Face::parse(&data, index) else {
            return whole;
        };
        if text
            .chars()
            .all(|c| inherits_face(c) || face_glyph(&face, c).is_some())
        {
            return whole;
        }
        // One face for each syllable: the span's when it has all of its
        // letters, else the face that has the first one it lacks (#599).
        let mut runs: Vec<(Range<usize>, fontdb::ID)> = Vec::new();
        let mut chars = text.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            let mut end = i + c.len_utf8();
            let mut lacks = (!inherits_face(c) && face_glyph(&face, c).is_none()).then_some(c);
            let mut last = c;
            while let Some(&(j, n)) = chars.peek() {
                if !continues_syllable(last, n) {
                    break;
                }
                if lacks.is_none() && !inherits_face(n) && face_glyph(&face, n).is_none() {
                    lacks = Some(n);
                }
                end = j + n.len_utf8();
                last = n;
                chars.next();
            }
            let id = match lacks {
                Some(l) => self.fallback_face(l, bold, italic).unwrap_or(primary),
                None if inherits_face(c) => runs.last().map(|r| r.1).unwrap_or(primary),
                None => primary,
            };
            match runs.last_mut() {
                Some(last) if last.1 == id => last.0.end = end,
                _ => runs.push((i..end, id)),
            }
        }
        runs
    }

    /// The face that draws `c` when the span's face lacks it: the fonts
    /// made for its script (see [`script_fonts`]) first, then any face
    /// that has it, upright faces of the span's weight first. Offered
    /// families made for the script are asked for. Remembered per letter
    /// until fonts are added.
    fn fallback_face(&self, c: char, bold: bool, italic: bool) -> Option<fontdb::ID> {
        let key = (c, bold, italic);
        if let Some(hit) = self
            .fallbacks
            .lock()
            .ok()
            .and_then(|m| m.get(&key).copied())
        {
            return hit;
        }
        let found = self.find_fallback(c, bold, italic);
        if let Ok(mut m) = self.fallbacks.lock() {
            if m.len() > 20000 {
                m.clear();
            }
            m.insert(key, found);
        }
        found
    }

    fn find_fallback(&self, c: char, bold: bool, italic: bool) -> Option<fontdb::ID> {
        let weight = if bold {
            fontdb::Weight::BOLD
        } else {
            fontdb::Weight::NORMAL
        };
        let style = if italic {
            fontdb::Style::Italic
        } else {
            fontdb::Style::Normal
        };
        let covers = |db: &fontdb::Database, id: fontdb::ID| {
            db.with_face_data(id, |d, i| {
                ttf_parser::Face::parse(d, i)
                    .ok()
                    .and_then(|f| f.glyph_index(c))
                    .is_some()
            })
            .unwrap_or(false)
        };
        for name in script_fonts(c) {
            if !self.has_family(name) {
                self.request_offered(name);
                continue;
            }
            let db = self.db();
            let q = fontdb::Query {
                families: &[fontdb::Family::Name(name)],
                weight,
                stretch: fontdb::Stretch::Normal,
                style,
            };
            if let Some(id) = db.query(&q) {
                if covers(&db, id) {
                    return Some(id);
                }
            }
        }
        let db = self.db();
        let mut any = None;
        for f in db.faces() {
            if !covers(&db, f.id) {
                continue;
            }
            if f.style == fontdb::Style::Normal && (f.weight.0 >= 600) == bold {
                return Some(f.id);
            }
            any.get_or_insert(f.id);
        }
        any
    }

    /// Shape `text` with the face `id` (see [`FontSystem::shape_run`]).
    #[allow(clippy::too_many_arguments)]
    fn shape_with(
        &self,
        id: fontdb::ID,
        span: &TextSpan,
        span_idx: usize,
        text: &str,
        size_scale: f64,
        char_base: usize,
        para: &ParagraphStyle,
        level: Option<u8>,
    ) -> Vec<Glyph> {
        let mut out = Vec::new();
        let Some((data, index)) = self.face_data(id) else {
            return out;
        };
        let Ok(mut face) = ttf_parser::Face::parse(&data, index) else {
            return out;
        };
        let Some(mut hb) = rustybuzz::Face::from_slice(&data, index) else {
            return out;
        };
        let target = self
            .face_and_weight(&span.font_family, span.bold, span.italic)
            .map_or(if span.bold { 700 } else { 400 }, |(_, w)| w);
        let (weight, slanted, coords) = set_axes(&mut face, &mut hb, target, span.italic);
        let font = tracedraw_core::document::text_outline::GlyphFont {
            data: data.clone(),
            index,
            coords,
        };
        let size_pt = span.size_pt * size_scale;
        let upem = face.units_per_em() as f64;
        let scale = size_pt * PT_MM / upem;
        // A heavy weight the face lacks is drawn thicker.
        let synth_bold = target >= 600 && weight < 600;
        let synth_italic = span.italic && !slanted;
        let tracking = span.tracking_pct / 100.0 * size_pt * PT_MM;
        let baseline = span.baseline_shift_pt * PT_MM;
        // Character spacing counts in widths of a space; word spacing
        // scales the spaces themselves.
        let space_adv = face
            .glyph_index(' ')
            .and_then(|g| face.glyph_hor_advance(g))
            .map(|a| a as f64 * scale)
            .unwrap_or(size_pt * PT_MM * 0.25);
        let char_spacing = para.char_spacing_pct.clamp(-100.0, 2000.0) / 100.0 * space_adv;
        let word_scale = para.word_spacing_pct.clamp(0.0, 2000.0) / 100.0;
        // Shifted and rotated characters: about the glyph's origin on its
        // shifted baseline; the pen does not move with them.
        let shift_x = span.offset_x_pct / 100.0 * size_pt * PT_MM;
        let place = if span.angle_deg.abs() > 1e-9 || shift_x.abs() > 1e-12 {
            Some(
                Affine::translate((shift_x, 0.0))
                    * Affine::rotate_about(span.angle_deg.to_radians(), Point::new(0.0, baseline)),
            )
        } else {
            None
        };

        let mut features: Vec<rustybuzz::Feature> = span
            .features
            .iter()
            .map(|f| {
                let tag = rustybuzz::ttf_parser::Tag::from_bytes_lossy(f.as_bytes());
                rustybuzz::Feature::new(tag, 1, ..)
            })
            .collect();
        // Vertical text takes the font's vertical forms (brackets and
        // punctuation turned or moved for columns) for its upright
        // letters.
        if para.vertical && text.chars().any(upright) {
            features.push(rustybuzz::Feature::new(
                rustybuzz::ttf_parser::Tag::from_bytes(b"vert"),
                1,
                ..,
            ));
        }
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        // The direction the bidirectional runs gave, whatever the script
        // (digits and spaces inside right-to-left text included).
        if let Some(l) = level {
            buffer.set_direction(if l % 2 == 1 {
                rustybuzz::Direction::RightToLeft
            } else {
                rustybuzz::Direction::LeftToRight
            });
        }
        let rtl = buffer.direction() == rustybuzz::Direction::RightToLeft;
        let level = level.unwrap_or(u8::from(rtl));
        let glyphs = rustybuzz::shape(&hb, &features, buffer);
        let infos = glyphs.glyph_infos();
        let positions = glyphs.glyph_positions();
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let complex = complex_text(text);
        // The characters each glyph stands for: from its cluster to the
        // next cluster (in text order); the first glyph of a cluster
        // carries them.
        let mut starts: Vec<usize> = infos.iter().map(|i| i.cluster as usize).collect();
        starts.sort_unstable();
        starts.dedup();
        let mut claimed: Vec<usize> = Vec::new();
        let size_mm = size_pt * PT_MM;
        let hyphen_glyph = {
            let gid = face.glyph_index('-');
            gid.map(|g| {
                let mut sink = OutlineSink {
                    path: BezPath::new(),
                    scale,
                    x: 0.0,
                    y: baseline,
                };
                face.outline_glyph(g, &mut sink);
                Arc::new(HyphenGlyph {
                    path: sink.path,
                    advance: face.glyph_hor_advance(g).unwrap_or(0) as f64 * scale,
                })
            })
        };
        for (info, pos) in infos.iter().zip(positions) {
            let gid = ttf_parser::GlyphId(info.glyph_id as u16);
            let mut sink = OutlineSink {
                path: BezPath::new(),
                scale,
                x: 0.0,
                y: baseline,
            };
            face.outline_glyph(gid, &mut sink);
            let mut gp = sink.path;
            if synth_italic {
                gp = Affine::new([1.0, 0.0, 0.2, 1.0, 0.0, 0.0]) * gp;
            }
            if synth_bold {
                let off = size_pt * PT_MM * 0.02;
                let g2 = Affine::translate((off, 0.0)) * gp.clone();
                gp.extend(g2.elements().iter().copied());
            }
            if let Some(a) = place {
                gp = a * gp;
            }
            let start = info.cluster as usize;
            let cluster_end = starts
                .iter()
                .copied()
                .find(|c| *c > start)
                .unwrap_or(text.len());
            let further = claimed.contains(&start);
            let joins = further
                || complex
                    && text
                        .get(..start)
                        .and_then(|t| t.chars().next_back())
                        .is_some_and(|p| {
                            text.get(start..)
                                .and_then(|t| t.chars().next())
                                .is_some_and(|c| continues_syllable(p, c))
                        });
            let cluster_text = if further {
                String::new()
            } else {
                claimed.push(start);
                text.get(start..cluster_end).unwrap_or("").to_string()
            };
            let mut local = Affine::translate((0.0, baseline)) * Affine::scale(size_mm);
            if synth_italic {
                local = Affine::new([1.0, 0.0, 0.2, 1.0, 0.0, 0.0]) * local;
            }
            if let Some(a) = place {
                local = a * local;
            }
            let src = Arc::new(GlyphSource {
                font: font.clone(),
                gid: gid.0,
                local,
                text: cluster_text,
                exact: !synth_bold,
            });
            let cluster = chars.iter().position(|(i, _)| *i == info.cluster as usize);
            let ch = cluster
                .and_then(|k| chars.get(k))
                .map(|(_, c)| *c)
                .unwrap_or(' ');
            let mut adv = pos.x_advance as f64 * scale;
            if ch == ' ' {
                adv *= word_scale;
            }
            out.push(Glyph {
                path: Arc::new(gp),
                advance: adv + tracking + char_spacing,
                x_offset: pos.x_offset as f64 * scale,
                y_offset: pos.y_offset as f64 * scale,
                ch,
                char_idx: char_base + cluster.unwrap_or(0),
                char_end: char_base + chars.partition_point(|(i, _)| *i < cluster_end),
                rtl,
                level,
                span: span_idx,
                joins,
                hyphen_after: hyphen_glyph.clone(),
                src: Some(src),
            });
        }
        // Right to left the glyphs come as they are seen: back to text
        // order, for the line breaker (#628).
        if rtl {
            out.reverse();
        }
        out
    }

    // ----- layout ------------------------------------------------------------

    /// Full layout of a text object.
    pub fn layout(&self, req: &TextRequest) -> TextLayout {
        self.layout_with(req, false)
    }

    /// The glyphs of a text object as its fonts draw them, placed as
    /// [`FontSystem::layout`] places its outline, and its other marks
    /// (underlines, strike lines, added hyphens) as curves.
    pub fn glyph_layout(&self, req: &TextRequest) -> GlyphLayout {
        self.layout_with(req, true).glyph_layout
    }

    fn layout_with(&self, req: &TextRequest, collect: bool) -> TextLayout {
        if req.para.fit_to_frame && req.frame.is_some() {
            // Scale the font size so the text fills the frame height.
            let (mut lo, mut hi) = (0.1, 20.0);
            let mut best = 1.0;
            for _ in 0..14 {
                let mid = (lo + hi) / 2.0;
                let l = self.layout_scaled(req, mid, false);
                if l.overflow {
                    hi = mid;
                } else {
                    best = mid;
                    lo = mid;
                }
            }
            return self.layout_scaled(req, best, collect);
        }
        self.layout_scaled(req, 1.0, collect)
    }

    fn layout_scaled(&self, req: &TextRequest, size_scale: f64, collect: bool) -> TextLayout {
        let spans = req.spans;
        if spans.is_empty() {
            return TextLayout {
                path: BezPath::new(),
                bounds: Rect::ZERO,
                glyphs: 0,
                overflow: false,
                fitted_chars: 0,
                baselines: Vec::new(),
                lines: Vec::new(),
                glyph_layout: GlyphLayout::default(),
                vertical: false,
            };
        }
        let para = req.para;
        // Vertical text is laid out as written lines in a frame turned on
        // its side (the columns run down the frame's height), and turned
        // into columns as the glyphs are placed; see `orient` below.
        let vertical = para.vertical && req.on_path.is_none();
        let frame = req.frame.map(|f| {
            if vertical {
                Size::new(f.height, f.width)
            } else {
                f
            }
        });
        let turn = tracedraw_core::document::vertical_line_frame();
        // Each paragraph's alignment and style: its own, or the object's.
        let styles = tracedraw_core::paragraphs::paragraph_styles(spans, req.align, para);
        let fallback = (req.align, para.clone());
        let style_of =
            |pi: usize| -> &(TextAlign, ParagraphStyle) { styles.get(pi).unwrap_or(&fallback) };
        let metrics: Vec<Metrics> = spans
            .iter()
            .map(|s| {
                let scaled = TextSpan {
                    size_pt: s.size_pt * size_scale,
                    ..s.clone()
                };
                self.metrics(&scaled)
            })
            .collect();

        // 1. Split the spans into paragraphs of glyph runs. `char_idx`
        // counts characters of the concatenated span text, newlines and
        // tabs included, so lines can report where they start.
        let mut paragraphs: Vec<Vec<Glyph>> = vec![Vec::new()];
        let mut para_starts: Vec<usize> = vec![0];
        let mut char_idx = 0usize;
        // A syllable a span boundary cuts (a vowel sign or the second half
        // of a conjunct in another colour) is shaped whole, in the style
        // of the span it starts in (#599): `taken[j]` bytes at the start
        // of span `j` went with the spans before it. Spans of plain ASCII
        // text take nothing.
        let mut taken: Vec<usize> = Vec::new();
        for (si, span) in spans.iter().enumerate() {
            let mut first = true;
            let skip = taken.get(si).copied().unwrap_or(0);
            let text = span.text.get(skip..).unwrap_or(&span.text);
            let mut pieces = text.split('\n').peekable();
            while let Some(piece) = pieces.next() {
                let last_piece = pieces.peek().is_none();
                if !first {
                    // The newline itself.
                    char_idx += 1;
                    paragraphs.push(Vec::new());
                    para_starts.push(char_idx);
                }
                first = false;
                // Tabs are kept as glyph-less markers so the line breaker can
                // expand them to the next tab stop.
                let mut parts = piece.split('\t').enumerate().peekable();
                while let Some((ti, part)) = parts.next() {
                    if ti > 0 {
                        if let Some(p) = paragraphs.last_mut() {
                            p.push(Glyph {
                                path: Arc::new(BezPath::new()),
                                advance: 0.0,
                                x_offset: 0.0,
                                y_offset: 0.0,
                                ch: '\t',
                                char_idx,
                                char_end: char_idx + 1,
                                rtl: false,
                                level: 0,
                                span: si,
                                joins: false,
                                hyphen_after: None,
                                src: None,
                            })
                        }
                        char_idx += 1;
                    }
                    let mut shaped = std::borrow::Cow::Borrowed(part);
                    if let (true, Some(mut prev)) = (
                        last_piece && parts.peek().is_none(),
                        part.chars().next_back(),
                    ) {
                        for (sj, next) in spans.iter().enumerate().skip(si + 1) {
                            let mut n = 0;
                            for c in next.text.chars() {
                                if !continues_syllable(prev, c) {
                                    break;
                                }
                                prev = c;
                                n += c.len_utf8();
                            }
                            if let Some(more) = next.text.get(..n).filter(|m| !m.is_empty()) {
                                shaped.to_mut().push_str(more);
                                if taken.len() <= sj {
                                    taken.resize(sj + 1, 0);
                                }
                                if let Some(t) = taken.get_mut(sj) {
                                    *t = n;
                                }
                            }
                            if n < next.text.len() {
                                break;
                            }
                        }
                    }
                    let pi = paragraphs.len().saturating_sub(1);
                    let run =
                        self.shape_run(span, si, &shaped, size_scale, char_idx, &style_of(pi).1);
                    char_idx += shaped.chars().count();
                    if let Some(p) = paragraphs.last_mut() {
                        p.extend(run);
                    }
                }
            }
        }
        let total_chars = char_idx;

        // 2. Flow the paragraphs into lines: break them to the room each
        // line has (the column, less indents, drop caps and the objects the
        // text wraps around) and give every line its place.
        let columns = para.columns.max(1) as usize;
        let col_width = frame
            .map(|f| ((f.width - para.gutter * (columns as f64 - 1.0)) / columns as f64).max(1.0));
        let frame_h = frame.map(|f| f.height);
        let artistic = frame.is_none();
        // Vertical text does not wrap around objects.
        let wrap: &[WrapRegion] = if artistic || vertical {
            &[]
        } else {
            &para.flow.wrap
        };
        // Baseline grid: baselines snap to the grid line at or below their
        // natural position: the page's grid, or one measured from the frame
        // top (y is negative below the frame top).
        let grid = if !artistic && para.baseline_grid_mm > 1e-6 {
            Some(para.flow.grid.unwrap_or((0.0, para.baseline_grid_mm)))
        } else {
            None
        };
        let snap = |y: f64| -> f64 {
            match grid {
                Some((o, g)) if g > 1e-6 && g.is_finite() && o.is_finite() => {
                    o + ((y - o) / g + 1e-9).floor() * g
                }
                _ => y,
            }
        };
        // The baseline after one at `y` in column `col` for a line with
        // metrics `m` that moves down by `advance`: (column, baseline). A
        // line that would leave the frame goes to the top of the next
        // column (except the frame's first line).
        let row_at = |y: f64, col: usize, advance: f64, m: &Metrics| -> (usize, f64) {
            let next = snap(y - advance);
            match frame_h {
                Some(h) if next - m.descent < -h - 1e-6 && (y != 0.0 || col > 0) => {
                    (col + 1, snap(-m.ascent))
                }
                _ => (col, next),
            }
        };
        let col_x_of =
            |col: usize| col.min(columns - 1) as f64 * (col_width.unwrap_or(0.0) + para.gutter);
        let mut lines: Vec<Line> = Vec::new();
        let mut overflow = false;
        // The first line that does not fit the frame.
        let mut overflow_line: Option<usize> = None;
        let mut fitted_chars = total_chars;
        let mut col = 0usize;
        let mut y = 0.0f64; // top of the frame or baseline of artistic text
        let mut prev_para: Option<usize> = None;
        for (pi, glyphs) in paragraphs.iter().enumerate() {
            let (_, ps) = style_of(pi);
            let m_para = glyphs
                .first()
                .map(|g| metrics[g.span])
                .unwrap_or_else(|| metrics[0]);
            let bullet = if ps.bullets { ps.bullet_indent } else { 0.0 };
            let indent_first = ps.left_indent + ps.first_line_indent + bullet;
            let indent_rest = ps.left_indent + bullet;
            let para_start = para_starts.get(pi).copied().unwrap_or(0);
            let Some(w) = col_width else {
                // Artistic text: one line a paragraph.
                let m = line_metrics(glyphs, &metrics, m_para);
                let advance = if pi == 0 {
                    0.0
                } else {
                    ps.line_step(m.line_height, m.size)
                };
                y -= advance;
                prev_para = Some(pi);
                lines.push(Line {
                    glyphs: glyphs.clone(),
                    hyphen: None,
                    metrics: m,
                    last_in_para: true,
                    para: pi,
                    start_char: para_start,
                    y,
                    x0: indent_first,
                    avail: f64::INFINITY,
                    col_x: 0.0,
                    cap_x: None,
                    bullet_x: None,
                });
                continue;
            };
            // A drop cap takes the first glyph out of the first line and
            // narrows the lines beside it.
            let cap = glyphs.first().filter(|_| ps.drop_cap_lines > 1).map(|g| {
                let step = ps.line_step(m_para.line_height, m_para.size);
                let n = ps.drop_cap_lines as f64;
                let (scale, _) = drop_cap_placement(&g.path, n, step, 0.0, m_para.ascent);
                DropCap {
                    width: g.advance * scale + 1.0,
                    lines: ps.drop_cap_lines as usize,
                    glyph_advance: g.advance,
                }
            });
            let mut br = Breaker::new(glyphs, ps.hyphenate, &metrics, pi, para_start);
            let mut row = 0usize;
            // How far a line moves down from the one before it.
            let advance_of = |m: &Metrics, first: bool, prev: Option<usize>| -> f64 {
                let Some(pp) = prev else {
                    return m.ascent;
                };
                let mut a = ps.line_step(m.line_height, m.size);
                if first {
                    a += ps.space_before;
                    if pp != pi {
                        a += style_of(pp).1.space_after;
                    }
                }
                a
            };
            loop {
                let first = br.first;
                let indent = if first { indent_first } else { indent_rest };
                let cap_w = match cap {
                    Some(c) if row < c.lines => c.width,
                    _ => 0.0,
                };
                // The first line gives the letter it drops back.
                let back = match cap {
                    Some(c) if first => c.glyph_advance,
                    _ => 0.0,
                };
                let base = (w - indent - ps.right_indent).max(1.0);
                let room = |seg: f64| (seg + back).max(1.0);
                let mut row_lines: Vec<Line> = Vec::new();
                let (row_col, row_y);
                let placed = !(wrap.is_empty() || overflow);
                if !placed {
                    let avail = if cap_w > 0.0 {
                        (base - cap_w + back).max(1.0)
                    } else {
                        base
                    };
                    let line = br.next(avail);
                    let m = line.metrics;
                    let (c, ny) = row_at(y, col, advance_of(&m, first, prev_para), &m);
                    row_col = c;
                    row_y = ny;
                    row_lines.push(Line {
                        avail: w - indent - ps.right_indent - cap_w,
                        ..line
                    });
                } else {
                    // Find the first place down the frame with room beside
                    // the objects the text wraps around.
                    let m_est = br.peek().map(|g| metrics[g.span]).unwrap_or(m_para);
                    let advance = advance_of(&m_est, first, prev_para);
                    let (c0, y0) = row_at(y, col, advance, &m_est);
                    let (mut c, mut ty) = (c0, y0);
                    let nudge = (ps.line_step(m_est.line_height, m_est.size) / 4.0).max(0.25);
                    let min_w = m_est.size.max(1.0);
                    let mut guard = 0usize;
                    // The parts of the line, the drop cap taken from the
                    // first one (the letter goes just before the text).
                    let segs = loop {
                        let col_x = col_x_of(c);
                        let lo = col_x + indent;
                        let hi = col_x + w - ps.right_indent;
                        if c >= columns || guard > 20_000 {
                            break vec![(lo + cap_w, hi.max(lo + cap_w + 1.0))];
                        }
                        let mut segs = free_segments(
                            lo,
                            hi,
                            ty - m_est.descent,
                            ty + m_est.ascent,
                            wrap,
                            min_w,
                        );
                        if cap_w > 0.0 {
                            segs.retain(|(s, e)| e - s >= cap_w + min_w);
                            if let Some(first) = segs.first_mut() {
                                first.0 += cap_w;
                            }
                        }
                        if !segs.is_empty() {
                            break segs;
                        }
                        guard += 1;
                        let next = snap(ty - nudge);
                        ty = if next < ty - 1e-9 { next } else { ty - nudge };
                        if let Some(h) = frame_h {
                            if ty - m_est.descent < -h - 1e-6 {
                                c += 1;
                                ty = snap(-m_est.ascent);
                            }
                        }
                    };
                    for (k, (s, e)) in segs.iter().enumerate() {
                        if k > 0 && br.done() {
                            break;
                        }
                        let avail = if k == 0 {
                            room(e - s)
                        } else {
                            (e - s).max(1.0)
                        };
                        let line = br.next(avail);
                        row_lines.push(Line {
                            x0: *s,
                            avail: e - s,
                            ..line
                        });
                        if br.done() {
                            break;
                        }
                    }
                    // The baseline from the line's own metrics where it
                    // stayed in place; moved down by a taller line where it
                    // had to find room.
                    let m = row_lines.first().map_or(m_est, |l| l.metrics);
                    let (c1, y1) = row_at(y, col, advance_of(&m, first, prev_para), &m);
                    if (c, ty) == (c0, y0) && c1 == c {
                        row_y = y1;
                    } else {
                        row_y = snap(ty - (m.ascent - m_est.ascent).max(0.0));
                    }
                    row_col = c;
                }
                if row_col >= columns && !overflow {
                    overflow = true;
                    overflow_line = Some(lines.len());
                    fitted_chars = row_lines
                        .first()
                        .map_or(total_chars, |l| l.start_char.min(total_chars));
                }
                col = row_col;
                y = row_y;
                prev_para = Some(pi);
                let col_x = col_x_of(col);
                for (k, mut line) in row_lines.into_iter().enumerate() {
                    line.y = y;
                    line.col_x = col_x;
                    if !placed {
                        line.x0 = col_x + indent + cap_w;
                    }
                    if k == 0 && first {
                        if cap.is_some() && !line.glyphs.is_empty() {
                            line.cap_x = Some(line.x0 - cap_w);
                        }
                        if ps.bullets {
                            line.bullet_x = Some(line.x0 - cap_w - indent + ps.left_indent);
                        }
                    }
                    lines.push(line);
                }
                row += 1;
                if br.done() {
                    break;
                }
            }
        }

        // 3. Draw the lines: drop caps, bullets, alignment, glyphs,
        // underlines. Characters of a span that is only laid out take their
        // room but are not drawn (a text drawn in parts, one per character
        // colour).
        let span_drawn = |span: usize| spans.get(span).is_none_or(|s| !s.layout_only);
        let mut path = BezPath::new();
        let mut glyphs_out = GlyphLayout::default();
        let mut glyph_count = 0usize;
        let mut min_y = 0.0f64;
        let mut max_x = 0.0f64;
        let mut line_baselines: Vec<(f64, f64)> = Vec::new();
        let mut line_boxes: Vec<LineBox> = Vec::new();
        // Where each line's characters end: the next line's start, or the
        // paragraph end (before its newline) for the last line of one.
        let line_ends: Vec<usize> = lines
            .iter()
            .enumerate()
            .map(|(li, line)| {
                if line.last_in_para {
                    para_starts
                        .get(line.para + 1)
                        .map(|s| s.saturating_sub(1))
                        .unwrap_or(total_chars)
                } else {
                    lines
                        .get(li + 1)
                        .map(|n| n.start_char)
                        .unwrap_or(total_chars)
                }
            })
            .collect();

        // Vertical text: a glyph's placing affine turned into the column.
        // Letters that lie on their side take the quarter turn whole; an
        // upright one (ideographs, kana, hangul) keeps its orientation,
        // centred on the column, the top of its em box at the pen (the
        // letter's ascent above its baseline), as the column advances
        // by the written advance.
        let orient = |a: Affine, g: &Glyph, ascent: f64| -> Affine {
            if !vertical {
                return a;
            }
            if upright(g.ch) {
                let c = a.as_coeffs();
                let pen = turn * Point::new(c[4], c[5]);
                let linear = Affine::new([c[0], c[1], c[2], c[3], 0.0, 0.0]);
                Affine::translate((pen.x - g.advance / 2.0, pen.y - ascent)) * linear
            } else {
                turn * a
            }
        };
        let orient_rest = |p: BezPath| -> BezPath {
            if vertical {
                turn * p
            } else {
                p
            }
        };
        for (li, line) in lines.iter().enumerate() {
            let m = line.metrics;
            let (align, ps) = style_of(line.para);
            let align = *align;
            let step = ps.line_step(m.line_height, m.size);
            let y = line.y;
            // Drawn only while the text fits the frame: the rest overflows.
            let line_drawn = overflow_line.is_none_or(|o| li < o);

            // The line's outlines; lines past the frame (overflow) are not
            // drawn.
            let mut line_path = BezPath::new();
            let mut line_glyphs = GlyphLayout::default();
            // Drop cap: the first glyph of a paragraph, scaled so its top
            // meets the top of the first line's letters and it sits on the
            // baseline of the Nth line.
            let mut glyphs: &[Glyph] = &line.glyphs;
            let x0 = line.x0;
            if let (Some(cap_x), Some(g), false) = (line.cap_x, glyphs.first(), artistic) {
                let n = ps.drop_cap_lines as f64;
                let (scale, baseline) = drop_cap_placement(&g.path, n, step, y, m.ascent);
                let a = Affine::translate((cap_x, baseline)) * Affine::scale(scale);
                let a = orient(a, g, m.ascent * scale);
                if span_drawn(g.span) {
                    place_glyph(collect, a, g, &mut line_glyphs);
                    let gp = a * g.path.as_ref().clone();
                    line_path.extend(gp.elements().iter().copied());
                }
                glyphs = &glyphs[1..];
                glyph_count += 1;
            }
            // Bullet.
            if let (Some(bx), false) = (line.bullet_x, artistic) {
                let first_span = glyphs.first().map(|g| g.span).unwrap_or(0);
                if let Some(span) = spans.get(first_span).filter(|_| span_drawn(first_span)) {
                    let b = self.shape_run(span, 0, &ps.bullet_char, size_scale, 0, ps);
                    for g in &b {
                        let a = orient(Affine::translate((bx, y)), g, m.ascent);
                        place_glyph(collect, a, g, &mut line_glyphs);
                        let gp = a * g.path.as_ref().clone();
                        line_path.extend(gp.elements().iter().copied());
                    }
                }
            }

            let avail = line.avail;
            let col_start = line.col_x;
            let (content_w, n_spaces) = line_width(glyphs, ps, x0 - col_start);
            let hyphen_w = line.hyphen.as_ref().map(|h| h.advance).unwrap_or(0.0);
            let total_w = content_w + hyphen_w;
            let (dx, space_extra) = match align {
                TextAlign::Left | TextAlign::None => (0.0, 0.0),
                TextAlign::Center => (((avail.min(1e9)) - total_w).max(0.0) / 2.0, 0.0),
                TextAlign::Right => ((avail.min(1e9) - total_w).max(0.0), 0.0),
                TextAlign::Justify | TextAlign::ForceJustify => {
                    // Forced justification spreads the last line too.
                    let last = line.last_in_para && align == TextAlign::Justify;
                    if last || n_spaces == 0 || !avail.is_finite() {
                        (0.0, 0.0)
                    } else {
                        (0.0, (avail - total_w).max(0.0) / n_spaces as f64)
                    }
                }
            };
            let dx = if artistic {
                match align {
                    TextAlign::Center => -total_w / 2.0,
                    TextAlign::Right => -total_w,
                    _ => 0.0,
                }
            } else {
                dx
            };

            // Emit glyphs.
            let mut pen = x0 + dx;
            let line_start = pen;
            let mut underline_runs: Vec<(usize, f64, f64)> = Vec::new();
            let end_char = line_ends.get(li).copied().unwrap_or(total_chars);
            let n_chars = end_char.saturating_sub(line.start_char);
            // Pen x before each character; clusters of several characters
            // share their glyph's advance evenly.
            let mut edges: Vec<f64> = vec![line_start; n_chars + 1];
            let mut starts: Vec<Option<f64>> = vec![None; n_chars];
            let mut ends: Vec<Option<f64>> = vec![None; n_chars];
            // Right-to-left characters run from x1 to x0: the pen before
            // one is at its right end (#629).
            let mut record = |from: usize, to: usize, x0: f64, x1: f64, rtl: bool| {
                let from = from.max(line.start_char);
                let to = to.min(end_char);
                if to <= from {
                    return;
                }
                let n = (to - from) as f64;
                let (a, b) = if rtl { (x1, x0) } else { (x0, x1) };
                for (k, c) in (from..to).enumerate() {
                    let x = a + (b - a) * k as f64 / n;
                    let next = a + (b - a) * (k + 1) as f64 / n;
                    let i = c - line.start_char;
                    if let Some(e) = edges.get_mut(i) {
                        *e = x;
                    }
                    if let Some(s) = starts.get_mut(i) {
                        if rtl {
                            *s = Some(next);
                        } else {
                            s.get_or_insert(x);
                        }
                    }
                    if let Some(e) = ends.get_mut(i) {
                        *e = Some(x.max(next));
                    }
                }
                // After the last character, unless the next one has a
                // glyph of its own placed elsewhere (where the direction
                // changes).
                let after = to - line.start_char;
                if !starts.get(after).is_some_and(|s| s.is_some()) {
                    if let Some(e) = edges.get_mut(after) {
                        *e = b;
                    }
                }
            };
            // The first pen x of the right-to-left cluster being placed,
            // so the glyphs of one cluster (a letter and its marks) count
            // as one.
            let mut rtl_cluster: Option<(usize, f64)> = None;
            // The line's glyphs are in text order; they are placed in the
            // order they are seen (#628).
            let order = display_order(glyphs);
            for k in 0..glyphs.len() {
                let gi = order.as_ref().map_or(k, |o| o[k]);
                let Some(g) = glyphs.get(gi) else {
                    continue;
                };
                // Up to the next glyph's character in text order, or this
                // cluster's end.
                let next_idx = match glyphs.get(gi + 1) {
                    Some(n) if n.char_idx > g.char_idx => n.char_idx,
                    Some(_) => g.char_end,
                    None => end_char,
                }
                .max(g.char_idx + 1);
                if g.ch == '\t' {
                    let rel = pen - col_start;
                    let next = next_tab(rel, &ps.tabs);
                    let before = pen;
                    pen = col_start + next;
                    record(g.char_idx, next_idx, before, pen, false);
                    continue;
                }
                if span_drawn(g.span) {
                    let a = Affine::translate((pen + g.x_offset, y + g.y_offset));
                    let a = orient(a, g, metrics[g.span].ascent);
                    place_glyph(collect, a, g, &mut line_glyphs);
                    let gp = a * g.path.as_ref().clone();
                    line_path.extend(gp.elements().iter().copied());
                }
                let adv = g.advance + if g.ch == ' ' { space_extra } else { 0.0 };
                let span = &spans[g.span];
                if (span.underline || span.strikethrough) && !span.layout_only {
                    match underline_runs.last_mut() {
                        Some((s, _, end)) if *s == g.span && (*end - pen).abs() < 1e-6 => {
                            *end = pen + adv
                        }
                        _ => underline_runs.push((g.span, pen, pen + adv)),
                    }
                }
                if g.rtl {
                    let x0 = match rtl_cluster {
                        Some((c, x0)) if c == g.char_idx => x0,
                        _ => pen,
                    };
                    rtl_cluster = Some((g.char_idx, x0));
                    record(g.char_idx, next_idx, x0, pen + adv, true);
                } else {
                    rtl_cluster = None;
                    record(g.char_idx, next_idx, pen, pen + adv, false);
                }
                pen += adv;
                glyph_count += 1;
            }
            let starts: Vec<f64> = starts
                .iter()
                .zip(&edges)
                .map(|(s, e)| s.unwrap_or(*e))
                .collect();
            let ends: Vec<f64> = ends
                .iter()
                .zip(&starts)
                .map(|(e, s)| e.unwrap_or(*s))
                .collect();
            line_boxes.push(LineBox {
                start_char: line.start_char,
                end_char,
                baseline: y,
                ascent: m.ascent,
                descent: m.descent,
                edges,
                starts,
                ends,
            });
            if let Some(h) = &line.hyphen {
                if line.glyphs.last().is_none_or(|g| span_drawn(g.span)) {
                    let gp = orient_rest(Affine::translate((pen, y)) * h.path.clone());
                    line_path.extend(gp.elements().iter().copied());
                    if collect {
                        line_glyphs.rest.extend(gp.elements().iter().copied());
                    }
                }
                pen += h.advance;
            }
            for (si, a, b) in underline_runs {
                let span = &spans[si];
                let sm = metrics[si];
                if span.underline {
                    let r = Rect::new(
                        a,
                        y + sm.underline_pos - sm.underline_thick,
                        b,
                        y + sm.underline_pos,
                    );
                    let rp = orient_rest(r.to_path(0.01));
                    line_path.extend(rp.elements().iter().copied());
                    if collect {
                        line_glyphs.rest.extend(rp.elements().iter().copied());
                    }
                }
                if span.strikethrough {
                    let r = Rect::new(
                        a,
                        y + sm.strike_pos - sm.underline_thick / 2.0,
                        b,
                        y + sm.strike_pos + sm.underline_thick / 2.0,
                    );
                    let rp = orient_rest(r.to_path(0.01));
                    line_path.extend(rp.elements().iter().copied());
                    if collect {
                        line_glyphs.rest.extend(rp.elements().iter().copied());
                    }
                }
            }
            if line_drawn {
                path.extend(line_path.elements().iter().copied());
                glyphs_out.glyphs.append(&mut line_glyphs.glyphs);
                glyphs_out
                    .rest
                    .extend(line_glyphs.rest.elements().iter().copied());
            }
            line_baselines.push((line_start, y));
            max_x = max_x.max(pen);
            min_y = min_y.min(y - m.descent);
        }

        // Vertical alignment in the frame: the lines move down together
        // by the room left under them (half of it for the middle). Text
        // that overflows stays at the top.
        if let (Some(h), false) = (frame_h, overflow) {
            let share = match para.vertical_align {
                VerticalAlign::Top => 0.0,
                VerticalAlign::Center => 0.5,
                VerticalAlign::Bottom => 1.0,
            };
            let dy = -(h + min_y).max(0.0) * share;
            if dy.abs() > 1e-9 {
                // In the columns of vertical text the move is sideways.
                let shift = if vertical {
                    Affine::translate((dy, 0.0))
                } else {
                    Affine::translate((0.0, dy))
                };
                path.apply_affine(shift);
                for g in &mut glyphs_out.glyphs {
                    g.transform = shift * g.transform;
                    g.path.apply_affine(shift);
                }
                glyphs_out.rest.apply_affine(shift);
                for b in &mut line_baselines {
                    b.1 += dy;
                }
                for lb in &mut line_boxes {
                    lb.baseline += dy;
                }
                min_y += dy;
            }
        }

        let top = if artistic {
            lines.first().map(|l| l.metrics.ascent).unwrap_or(0.0)
        } else {
            0.0
        };
        let bounds = Rect::new(
            0.0,
            min_y,
            max_x.max(frame.map(|f| f.width).unwrap_or(0.0)),
            top,
        );
        let bounds = if vertical {
            (turn * bounds.to_path(0.01)).bounding_box()
        } else {
            bounds
        };

        // 4. Text on a path: re-place the glyphs of artistic text along the curve.
        if let (Some(tp), true) = (req.on_path, artistic) {
            let mut on_path = GlyphLayout::default();
            let placed = place_on_path(&lines, tp, style_of(0).0, spans, collect, &mut on_path);
            return TextLayout {
                path: placed,
                bounds,
                glyphs: glyph_count,
                overflow: false,
                fitted_chars: total_chars,
                baselines: line_baselines,
                lines: line_boxes,
                glyph_layout: on_path,
                vertical: false,
            };
        }

        TextLayout {
            path,
            bounds,
            glyphs: glyph_count,
            overflow,
            fitted_chars,
            baselines: line_baselines,
            lines: line_boxes,
            glyph_layout: glyphs_out,
            vertical,
        }
    }
}

/// Whether a character stands upright in vertical text (ideographs,
/// kana, hangul, their punctuation and full-width forms, enclosed and
/// pictorial symbols) rather than lying on its side like Latin letters
/// and digits: the Unicode vertical orientation (UAX #50), by its main
/// ranges.
pub fn upright(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x11FF       // Hangul Jamo
            | 0x2460..=0x24FF // enclosed alphanumerics
            | 0x25A0..=0x27BF // geometric shapes, miscellaneous symbols, dingbats
            | 0x2E80..=0x2FDF // CJK radicals, Kangxi radicals
            | 0x2FF0..=0x303F // ideographic description, CJK symbols and punctuation
            | 0x3040..=0x30FF // hiragana, katakana
            | 0x3100..=0x31FF // bopomofo, hangul compatibility jamo, kanbun, small kana
            | 0x3200..=0x4DBF // enclosed CJK, CJK compatibility, extension A
            | 0x4E00..=0x9FFF // CJK unified ideographs
            | 0xA000..=0xA4CF // Yi
            | 0xA960..=0xA97F // Hangul Jamo extended A
            | 0xAC00..=0xD7FF // Hangul syllables, Jamo extended B
            | 0xF900..=0xFAFF // CJK compatibility ideographs
            | 0xFE10..=0xFE1F // vertical forms
            | 0xFE30..=0xFE4F // CJK compatibility forms
            | 0xFF01..=0xFF60 // full-width forms
            | 0xFFE0..=0xFFE6 // full-width signs
            | 0x1F000..=0x1FAFF // mahjong, domino, playing cards, emoji, symbols
            | 0x20000..=0x3FFFF // CJK extensions B and on
    )
}

/// The tallest metrics among a line's glyphs; `fallback` for a line with
/// none (a larger first span elsewhere in the paragraph, such as a big
/// first letter, does not open up the other lines).
fn line_metrics(glyphs: &[Glyph], metrics: &[Metrics], fallback: Metrics) -> Metrics {
    let Some(first) = glyphs.first() else {
        return fallback;
    };
    let mut m = metrics[first.span];
    for g in glyphs {
        let gm = metrics[g.span];
        m.ascent = m.ascent.max(gm.ascent);
        m.descent = m.descent.max(gm.descent);
        m.line_height = m.line_height.max(gm.line_height);
        m.size = m.size.max(gm.size);
    }
    m
}

/// Scale and baseline of a drop cap spanning `lines` lines `step` apart,
/// whose first line has its baseline at `baseline` (y up): the letter sits
/// on the last line's baseline and its top meets the top of the first
/// line's letters (the glyph's own height there, or 70 % of `ascent` for
/// a glyph without outline).
fn drop_cap_placement(
    glyph: &BezPath,
    lines: f64,
    step: f64,
    baseline: f64,
    ascent: f64,
) -> (f64, f64) {
    let top = glyph.bounding_box().y1;
    let letter = if top.is_finite() && top > 1e-6 {
        top
    } else {
        ascent * 0.7
    };
    let last = baseline - (lines - 1.0) * step;
    let height = baseline + letter - last;
    let scale = if letter > 1e-6 {
        height / letter
    } else {
        lines
    };
    (scale.max(1.0), last)
}

/// Width of a line's glyphs with tab expansion; returns (width, spaces).
fn line_width(glyphs: &[Glyph], para: &ParagraphStyle, start: f64) -> (f64, usize) {
    let mut x = start;
    let mut spaces = 0;
    for g in glyphs {
        if g.ch == '\t' {
            x = next_tab(x, &para.tabs);
        } else {
            if g.ch == ' ' {
                spaces += 1;
            }
            x += g.advance;
        }
    }
    (x - start, spaces)
}

fn next_tab(x: f64, tabs: &[f64]) -> f64 {
    if tabs.is_empty() {
        let step = 12.7;
        return (x / step).floor() * step + step;
    }
    tabs.iter()
        .cloned()
        .find(|t| *t > x + 1e-6)
        .unwrap_or(x + 12.7)
}

/// Characters that never start a line in Chinese and Japanese text:
/// closing brackets and quotes, sentence and clause punctuation, the
/// iteration marks, the prolonged sound mark and small kana.
fn no_line_start(c: char) -> bool {
    matches!(
        c,
        '!' | ')' | ',' | '.' | ':' | ';' | '?' | ']' | '}' | '%'
            | '\u{00B0}' | '\u{00B7}' | '\u{00BB}'
            | '\u{2019}' | '\u{201D}' | '\u{2025}' | '\u{2026}' | '\u{2030}'
            | '\u{2032}' | '\u{2033}' | '\u{203A}' | '\u{203C}' | '\u{2047}'..='\u{2049}'
            | '\u{2103}'
            | '\u{3001}' | '\u{3002}' | '\u{3005}' | '\u{3009}' | '\u{300B}' | '\u{300D}'
            | '\u{300F}' | '\u{3011}' | '\u{3015}' | '\u{3017}' | '\u{3019}' | '\u{301B}'
            | '\u{301C}' | '\u{301E}' | '\u{301F}' | '\u{303B}'
            | '\u{3041}' | '\u{3043}' | '\u{3045}' | '\u{3047}' | '\u{3049}' | '\u{3063}'
            | '\u{3083}' | '\u{3085}' | '\u{3087}' | '\u{308E}' | '\u{3095}' | '\u{3096}'
            | '\u{309B}'..='\u{309E}'
            | '\u{30A0}' | '\u{30A1}' | '\u{30A3}' | '\u{30A5}' | '\u{30A7}' | '\u{30A9}'
            | '\u{30C3}' | '\u{30E3}' | '\u{30E5}' | '\u{30E7}' | '\u{30EE}' | '\u{30F5}'
            | '\u{30F6}' | '\u{30FB}'..='\u{30FE}' | '\u{31F0}'..='\u{31FF}'
            | '\u{FE10}'..='\u{FE19}' | '\u{FE30}' | '\u{FE31}' | '\u{FE36}' | '\u{FE38}'
            | '\u{FE3A}' | '\u{FE3C}' | '\u{FE3E}' | '\u{FE40}' | '\u{FE42}' | '\u{FE44}'
            | '\u{FE50}'..='\u{FE57}' | '\u{FE5A}' | '\u{FE5C}' | '\u{FE5E}'
            | '\u{FF01}' | '\u{FF05}' | '\u{FF09}' | '\u{FF0C}' | '\u{FF0E}' | '\u{FF1A}'
            | '\u{FF1B}' | '\u{FF1F}' | '\u{FF3D}' | '\u{FF5D}' | '\u{FF60}' | '\u{FF61}'
            | '\u{FF63}' | '\u{FF64}' | '\u{FF65}' | '\u{FF67}'..='\u{FF70}' | '\u{FF9E}'
            | '\u{FF9F}' | '\u{FFE0}'
    )
}

/// Characters that never end a line in Chinese and Japanese text:
/// opening brackets and quotes and the currency signs put before a sum.
fn no_line_end(c: char) -> bool {
    matches!(
        c,
        '(' | '['
            | '{'
            | '$'
            | '\u{00A3}'
            | '\u{00A5}'
            | '\u{00AB}'
            | '\u{2018}'
            | '\u{201C}'
            | '\u{2039}'
            | '\u{3008}'
            | '\u{300A}'
            | '\u{300C}'
            | '\u{300E}'
            | '\u{3010}'
            | '\u{3014}'
            | '\u{3016}'
            | '\u{3018}'
            | '\u{301A}'
            | '\u{301D}'
            | '\u{FE35}'
            | '\u{FE37}'
            | '\u{FE39}'
            | '\u{FE3B}'
            | '\u{FE3D}'
            | '\u{FE3F}'
            | '\u{FE41}'
            | '\u{FE43}'
            | '\u{FE59}'
            | '\u{FE5B}'
            | '\u{FE5D}'
            | '\u{FF04}'
            | '\u{FF08}'
            | '\u{FF3B}'
            | '\u{FF5B}'
            | '\u{FF5F}'
            | '\u{FF62}'
            | '\u{FFE1}'
            | '\u{FFE5}'
            | '\u{FFE6}'
    )
}

/// Chinese and Japanese characters and their punctuation: text written
/// without spaces, which lines may break between.
fn cjk_char(c: char) -> bool {
    matches!(c as u32,
        0x2E80..=0x2FDF // radicals
            | 0x2FF0..=0x30FF // CJK symbols and punctuation, kana
            | 0x3100..=0x312F // bopomofo
            | 0x3190..=0x31FF // kanbun, CJK strokes, kana extensions
            | 0x3200..=0x4DBF // enclosed CJK, compatibility, extension A
            | 0x4E00..=0x9FFF // unified ideographs
            | 0xF900..=0xFAFF // compatibility ideographs
            | 0xFE10..=0xFE1F // vertical forms
            | 0xFE30..=0xFE4F // compatibility forms
            | 0xFE50..=0xFE6F // small form variants
            | 0xFF00..=0xFFEF // half-width and full-width forms
            | 0x20000..=0x3FFFF // extensions B and on
    )
}

/// A line may break between `a` and `b` (neighbours with no space between
/// them) because one of them is Chinese or Japanese, unless `b` may not
/// start a line or `a` may not end one (#847). Other text breaks only at
/// spaces and hyphenation points.
fn cjk_break(a: char, b: char) -> bool {
    (cjk_char(a) || cjk_char(b)) && !no_line_start(b) && !no_line_end(a)
}

/// Greedy line breaking at spaces, with optional hyphenation and a
/// fallback to breaking inside words that do not fit alone, one line at a
/// time for the room each line has.
struct Breaker<'a> {
    /// Runs separated by spaces; the space belongs to the preceding word.
    words: Vec<Vec<Glyph>>,
    i: usize,
    /// What is left of `words[i]` after a break inside it.
    tail: Option<Vec<Glyph>>,
    /// The next line is the paragraph's first.
    first: bool,
    /// An empty paragraph still owes its one empty line.
    empty_pending: bool,
    hyphenate: bool,
    metrics: &'a [Metrics],
    pi: usize,
    para_start: usize,
}

impl<'a> Breaker<'a> {
    fn new(
        glyphs: &[Glyph],
        hyphenate: bool,
        metrics: &'a [Metrics],
        pi: usize,
        para_start: usize,
    ) -> Self {
        let mut words: Vec<Vec<Glyph>> = Vec::new();
        let mut cur: Vec<Glyph> = Vec::new();
        for (k, g) in glyphs.iter().enumerate() {
            let is_sep = g.ch == ' ' || g.ch == '\t';
            cur.push(g.clone());
            // Text without spaces (Chinese, Japanese) breaks between its
            // characters, where the line-start and line-end rules allow.
            let cjk_break = glyphs
                .get(k + 1)
                .is_some_and(|n| !n.joins && n.ch != ' ' && n.ch != '\t' && cjk_break(g.ch, n.ch));
            if is_sep || cjk_break {
                words.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            words.push(cur);
        }
        Breaker {
            empty_pending: words.is_empty(),
            words,
            i: 0,
            tail: None,
            first: true,
            hyphenate,
            metrics,
            pi,
            para_start,
        }
    }

    fn done(&self) -> bool {
        !self.empty_pending && self.i >= self.words.len()
    }

    /// The next glyph to place.
    fn peek(&self) -> Option<&Glyph> {
        match &self.tail {
            Some(t) => t.first(),
            None => self.words.get(self.i).and_then(|w| w.first()),
        }
    }

    fn line(&mut self, mut glyphs: Vec<Glyph>, hyphen: Option<Arc<HyphenGlyph>>) -> Line {
        self.first = false;
        let start_char = glyphs
            .first()
            .map(|g| g.char_idx)
            .unwrap_or(self.para_start);
        // Trailing spaces do not count toward alignment: strip them.
        while glyphs.last().map(|g| g.ch == ' ').unwrap_or(false) {
            glyphs.pop();
        }
        let fallback = self.metrics.first().copied().unwrap_or_default();
        Line {
            metrics: line_metrics(&glyphs, self.metrics, fallback),
            glyphs,
            hyphen,
            last_in_para: self.done(),
            para: self.pi,
            start_char,
            y: 0.0,
            x0: 0.0,
            avail: 0.0,
            col_x: 0.0,
            cap_x: None,
            bullet_x: None,
        }
    }

    /// The next line, `avail` wide: at least one glyph while any is left.
    fn next(&mut self, avail: f64) -> Line {
        if self.empty_pending {
            self.empty_pending = false;
            return self.line(Vec::new(), None);
        }
        let width_of = |gs: &[Glyph]| -> f64 {
            gs.iter()
                .map(|g| if g.ch == '\t' { 12.7 } else { g.advance })
                .sum()
        };
        let trimmed = |gs: &[Glyph]| -> f64 {
            let mut w = width_of(gs);
            if let Some(l) = gs.last() {
                if l.ch == ' ' {
                    w -= l.advance;
                }
            }
            w
        };
        let mut line: Vec<Glyph> = Vec::new();
        loop {
            let word = match &self.tail {
                Some(t) => t.clone(),
                None => match self.words.get(self.i) {
                    Some(w) => w.clone(),
                    None => break,
                },
            };
            if trimmed(&[line.as_slice(), word.as_slice()].concat()) <= avail {
                line.extend(word);
                self.tail = None;
                self.i += 1;
                continue;
            }
            // Try hyphenating the word.
            if self.hyphenate || line.is_empty() {
                let text: String = word.iter().map(|g| g.ch).collect();
                let points = if self.hyphenate {
                    hyphen::break_points(text.trim_end())
                } else {
                    // Forced break inside an over-long word: any position.
                    (1..word.len()).collect()
                };
                let mut best: Option<usize> = None;
                for p in points {
                    if p == 0 || word.get(p).is_none_or(|g| g.joins) {
                        continue;
                    }
                    let head = &word[..p];
                    let hy = head
                        .last()
                        .and_then(|g| g.hyphen_after.as_ref())
                        .map(|h| h.advance)
                        .unwrap_or(0.0);
                    let w = trimmed(&[line.as_slice(), head].concat())
                        + if self.hyphenate { hy } else { 0.0 };
                    if w <= avail {
                        best = Some(p);
                    } else {
                        break;
                    }
                }
                if let Some(p) = best {
                    let head = word[..p].to_vec();
                    let hyphen = if self.hyphenate {
                        head.last().and_then(|g| g.hyphen_after.clone())
                    } else {
                        None
                    };
                    line.extend(head);
                    self.tail = Some(word[p..].to_vec());
                    return self.line(line, hyphen);
                }
            }
            if line.is_empty() {
                // Nothing fits at all; put the word anyway to make progress.
                line.extend(word);
                self.tail = None;
                self.i += 1;
            }
            break;
        }
        self.line(line, None)
    }
}

/// The room a drop cap takes from the lines of its paragraph.
#[derive(Debug, Clone, Copy)]
struct DropCap {
    /// The indent of the lines beside it, mm.
    width: f64,
    /// How many lines it spans.
    lines: usize,
    /// The advance of the letter at text size (it leaves the first line).
    glyph_advance: f64,
}

/// Place artistic text along a path: each glyph is centred on its arc
/// length position along the baseline (see [`TextOnPath::baseline`]) and
/// turned to its direction there. Positions wrap around closed paths.
fn place_on_path(
    lines: &[Line],
    tp: &TextOnPath,
    align: TextAlign,
    spans: &[TextSpan],
    collect: bool,
    glyphs: &mut GlyphLayout,
) -> BezPath {
    let mut out = BezPath::new();
    let Some(line) = lines.first() else {
        return out;
    };
    let baseline = tp.baseline();
    let total = baseline.length();
    if total <= 0.0 {
        return out;
    }
    let text_w: f64 = line.glyphs.iter().map(|g| g.advance).sum();
    let start = tp.offset
        + match align {
            TextAlign::Center => (total - text_w) / 2.0,
            TextAlign::Right => total - text_w,
            _ => 0.0,
        };
    // The line of the letters that sits on the path, as a shift of the
    // letters (y up) before they are placed.
    let m = &line.metrics;
    let lift = match tp.vertical {
        PathTextVertical::Baseline => 0.0,
        PathTextVertical::Ascender => -m.ascent,
        PathTextVertical::Descender => m.descent,
        PathTextVertical::Center => (m.descent - m.ascent) / 2.0,
    };
    // One mirror on: the letters are flipped across the baseline.
    let flip = if tp.flipped() {
        Affine::scale_non_uniform(1.0, -1.0)
    } else {
        Affine::IDENTITY
    };
    let mut pen = start;
    let order = display_order(&line.glyphs);
    for k in 0..line.glyphs.len() {
        let Some(g) = line.glyphs.get(order.as_ref().map_or(k, |o| o[k])) else {
            continue;
        };
        let mid = pen + g.advance / 2.0;
        pen += g.advance;
        let Some((p, dir)) = baseline.at(mid) else {
            continue;
        };
        let along = letter_frame(tp.orientation, dir);
        let place = Affine::translate(p.to_vec2())
            * along
            * flip
            * Affine::translate((-g.advance / 2.0 + g.x_offset, g.y_offset + lift));
        if spans.get(g.span).is_some_and(|s| s.layout_only) {
            continue;
        }
        place_glyph(collect, place, g, glyphs);
        let gp = place * g.path.as_ref().clone();
        out.extend(gp.elements().iter().copied());
    }
    out
}

/// Add glyph `g`, placed by `a`, to the glyph layout when glyphs are
/// collected: as a font glyph, or as curves when it has no font glyph.
fn place_glyph(collect: bool, a: Affine, g: &Glyph, out: &mut GlyphLayout) {
    if !collect {
        return;
    }
    let path = a * g.path.as_ref().clone();
    match &g.src {
        Some(src) => out.glyphs.push(PlacedGlyph {
            font: src.font.clone(),
            glyph_id: src.gid,
            transform: a * src.local,
            text: src.text.clone(),
            span: g.span,
            path,
            exact: src.exact,
        }),
        None => out.rest.extend(path.elements().iter().copied()),
    }
}

/// How a letter centred at a point of the path with unit direction `dir`
/// is turned or sheared there (before the letter's own flip).
fn letter_frame(o: PathTextOrientation, dir: tracedraw_core::geometry::Vec2) -> Affine {
    // The direction's horizontal part, kept away from zero so a nearly
    // vertical stretch of path does not shear a letter to a line.
    let cx = if dir.x.abs() < 0.2 {
        0.2_f64.copysign(if dir.x == 0.0 { 1.0 } else { dir.x })
    } else {
        dir.x
    };
    match o {
        PathTextOrientation::Rotate => Affine::rotate(dir.y.atan2(dir.x)),
        PathTextOrientation::Upright => Affine::IDENTITY,
        // x follows the path's slope, y stays vertical.
        PathTextOrientation::VerticalSkew => Affine::new([1.0, dir.y / cx, 0.0, 1.0, 0.0, 0.0]),
        // x stays horizontal, y leans along the path's normal.
        PathTextOrientation::HorizontalSkew => Affine::new([1.0, 0.0, -dir.y / cx, 1.0, 0.0, 0.0]),
    }
}

struct OutlineSink {
    path: BezPath,
    scale: f64,
    x: f64,
    y: f64,
}

impl OutlineSink {
    fn p(&self, x: f32, y: f32) -> Point {
        Point::new(
            self.x + x as f64 * self.scale,
            self.y + y as f64 * self.scale,
        )
    }
}

impl ttf_parser::OutlineBuilder for OutlineSink {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to(self.p(x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(self.p(x, y));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path.quad_to(self.p(x1, y1), self.p(x, y));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path
            .curve_to(self.p(x1, y1), self.p(x2, y2), self.p(x, y));
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}

#[cfg(test)]
mod devanagari_tests;

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines of `text` broken to `chars` full-width cells (every glyph is
    /// 1 mm wide), as strings.
    fn cjk_lines(text: &str, chars: usize) -> Vec<String> {
        let glyphs: Vec<Glyph> = text
            .chars()
            .enumerate()
            .map(|(i, ch)| Glyph {
                path: Arc::new(BezPath::new()),
                advance: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
                ch,
                char_idx: i,
                char_end: i + 1,
                rtl: false,
                level: 0,
                span: 0,
                joins: false,
                hyphen_after: None,
                src: None,
            })
            .collect();
        let metrics = [Metrics::default()];
        let mut br = Breaker::new(&glyphs, false, &metrics, 0, 0);
        let mut out = Vec::new();
        while !br.done() {
            out.push(
                br.next(chars as f64 + 0.01)
                    .glyphs
                    .iter()
                    .map(|g| g.ch)
                    .collect(),
            );
        }
        out
    }

    /// Chinese text breaks between its characters, but a comma, a full
    /// stop or a closing quote never starts a line and an opening quote
    /// never ends one: the character before goes down with it (#847).
    #[test]
    fn cjk_punctuation_never_starts_a_line() {
        // Free breaking: five per line.
        assert_eq!(
            cjk_lines("相师得此奇遇潜心修习", 5),
            vec!["相师得此奇", "遇潜心修习"]
        );
        // The comma would start the second line: 成 goes down with it.
        assert_eq!(
            cjk_lines("小有所成，乃出。几番", 4),
            vec!["小有所", "成，乃", "出。几番"]
        );
        for text in [
            "我半生说学尽在相术尤精于风水之相。这青云山乃是人间罕有灵地",
            "名曰“青云”。因无名古卷所载近于道家，他便做道人打扮，自号“青云子”",
            "日本語の文章です。「かぎ括弧」も、ちゃんと処理されます。ー",
        ] {
            for width in 3..9 {
                let lines = cjk_lines(text, width);
                assert_eq!(lines.concat(), text);
                for l in &lines {
                    assert!(l.chars().count() <= width, "{l} wider than {width}");
                }
                for pair in lines.windows(2) {
                    let first = pair[1].chars().next().unwrap_or(' ');
                    let last = pair[0].chars().next_back().unwrap_or(' ');
                    assert!(
                        !no_line_start(first),
                        "{:?} starts a line at {width}",
                        pair[1]
                    );
                    assert!(!no_line_end(last), "{:?} ends a line at {width}", pair[0]);
                }
            }
        }
        // Latin text keeps breaking at spaces only.
        assert_eq!(cjk_lines("ab cd", 3), vec!["ab", "cd"]);
    }

    /// The box of a text object comes from the box kept with its cached
    /// outline when the object is not turned or skewed, and is the same
    /// as the outline's own; a turned one still measures its outline
    /// (#802).
    #[test]
    fn text_bounds_come_from_the_cached_box_and_match_the_outline() {
        install();
        let Some(family) = fonts().families().into_iter().next() else {
            return;
        };
        let mut s = tracedraw_core::Shape::new(
            tracedraw_core::ShapeId(1),
            tracedraw_core::ShapeKind::Text {
                spans: vec![TextSpan::new("Long text, many letters", &family, 40.0)],
                origin: Point::new(12.0, 30.0),
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
        );
        let near = |a: Rect, b: Rect| {
            (a.x0 - b.x0).abs() < 1e-6
                && (a.y0 - b.y0).abs() < 1e-6
                && (a.x1 - b.x1).abs() < 1e-6
                && (a.y1 - b.y1).abs() < 1e-6
        };
        for t in [
            Affine::IDENTITY,
            Affine::translate((5.0, -7.0)) * Affine::scale_non_uniform(2.0, 0.5),
            Affine::scale_non_uniform(-1.0, 1.0),
            Affine::rotate(0.3),
        ] {
            s.transform = t;
            let outline = s.page_path().bounding_box();
            assert!(
                near(s.bounds(), outline),
                "{t:?}: {:?} {outline:?}",
                s.bounds()
            );
        }
        // The box is kept with the outline.
        s.transform = Affine::IDENTITY;
        let tracedraw_core::ShapeKind::Text { spans, para, .. } = &s.kind else {
            unreachable!()
        };
        let req = TextRequest {
            spans,
            frame: None,
            align: Default::default(),
            para,
            on_path: None,
        };
        let kept = fonts().bounds_cached(&req).expect("a box");
        assert!(near(kept, fonts().outline_cached(&req).bounding_box()));
    }
    use tracedraw_core::geometry::Size;

    /// Hindi after a Latin word is shaped as Hindi: the run is cut where
    /// the script changes, the space and digits going with the run before
    /// them, so its half forms and conjuncts are made (#599). One script,
    /// plain ASCII and right-to-left text stay whole.
    #[test]
    fn text_is_shaped_one_script_at_a_time() {
        let t = "PDF में स्थिति 12.";
        let parts: Vec<&str> = script_runs(t).into_iter().map(|r| &t[r]).collect();
        assert_eq!(parts, ["PDF ", "में स्थिति 12."]);
        let t = "1. स्थिति and more";
        let parts: Vec<&str> = script_runs(t).into_iter().map(|r| &t[r]).collect();
        assert_eq!(parts, ["1. स्थिति ", "and more"]);
        for whole in ["स्थिति क्षमता", "plain text", "مرحبا PDF", "Ünïcödé"]
        {
            assert_eq!(script_runs(whole), vec![0..whole.len()], "{whole}");
        }
    }

    /// In right-to-left text the caret before each letter is at the
    /// letter's right end, the end of the text at its left, and each letter
    /// covers its own glyph (#629).
    #[test]
    fn right_to_left_carets_run_from_the_right() {
        if !has_fonts() {
            return;
        }
        let l = fonts().layout(&TextRequest {
            spans: &[span("\u{645}\u{631}\u{62D}\u{628}\u{627}")],
            frame: None,
            align: TextAlign::Left,
            para: &ParagraphStyle::default(),
            on_path: None,
        });
        if l.glyphs == 0 {
            return;
        }
        let line = &l.lines[0];
        assert_eq!(line.edges.len(), 6);
        assert!(
            line.edges.windows(2).all(|w| w[1] < w[0]),
            "{:?}",
            line.edges
        );
        let right = l.path.bounding_box().x1;
        assert!(line.edges[0] >= right - 0.5, "{:?} {right}", line.edges);
        assert!(line.edges[5].abs() < 1e-6, "{:?}", line.edges);
        for i in 0..5 {
            assert!((line.starts[i] - line.edges[i + 1]).abs() < 1e-9);
            assert!((line.ends[i] - line.edges[i]).abs() < 1e-9);
        }
        // Left-to-right text as before.
        let l = fonts().layout(&TextRequest {
            spans: &[span("abc")],
            frame: None,
            align: TextAlign::Left,
            para: &ParagraphStyle::default(),
            on_path: None,
        });
        let line = &l.lines[0];
        assert!(
            line.edges.windows(2).all(|w| w[1] > w[0]),
            "{:?}",
            line.edges
        );
        assert_eq!(line.starts[..], line.edges[..3]);
        assert_eq!(line.ends[..], line.edges[1..]);
    }

    /// Text mixing directions is cut into runs of one level, in text
    /// order; the first strong letter sets the text's own direction, and
    /// text with no right-to-left letter is not analysed (#628).
    #[test]
    fn mixed_text_is_cut_into_runs_of_one_direction() {
        let runs = |t: &str| -> Vec<(String, u8)> {
            bidi_runs(t)
                .unwrap_or_default()
                .into_iter()
                .map(|(r, l)| (t[r].to_string(), l))
                .collect()
        };
        assert!(bidi_runs("plain text").is_none());
        let ar = "\u{645}\u{631}\u{62D}\u{628}\u{627}";
        assert_eq!(runs(ar), [(ar.into(), 1)]);
        assert_eq!(
            runs(&format!("Asasa {ar}")),
            [("Asasa ".into(), 0), (ar.into(), 1)]
        );
        // Right to left first: the Latin word one level up.
        assert_eq!(
            runs(&format!("{ar} Asasa")),
            [(format!("{ar} "), 1), ("Asasa".into(), 2)]
        );
        // Digits inside right-to-left text read left to right.
        assert_eq!(
            runs(&format!("{ar} 123")),
            [(format!("{ar} "), 1), ("123".into(), 2)]
        );
        // Scripts of the planes above too (Adlam).
        assert_eq!(
            runs("\u{1E900}\u{1E901}"),
            [("\u{1E900}\u{1E901}".into(), 1)]
        );
    }

    /// Paragraph text mixing directions breaks in text order: "Hi مرحبا
    /// بك" in a frame too narrow for it puts the first Arabic word on the
    /// first line, right of "Hi", and the second on the next; a frame
    /// holding one line keeps text up to the word that flows on (#628).
    #[test]
    fn mixed_paragraph_text_breaks_in_text_order() {
        if !has_fonts() {
            return;
        }
        let t = "Hi \u{645}\u{631}\u{62D}\u{628}\u{627} \u{628}\u{643}";
        let lay = |frame: Option<Size>| {
            fonts().layout(&TextRequest {
                spans: &[span(t)],
                frame,
                align: TextAlign::Left,
                para: &ParagraphStyle::default(),
                on_path: None,
            })
        };
        let whole = lay(None);
        if whole.glyphs == 0 {
            return;
        }
        let l = &whole.lines[0];
        let (w0, w1) = l.x_range();
        // Room for "Hi مرحبا " and a little more, not for "بك".
        let first: f64 = (0..9).map(|i| l.ends[i] - l.starts[i]).sum();
        let width = first + (w1 - w0 - first) * 0.3;
        let wrapped = lay(Some(Size::new(width, 100.0)));
        assert_eq!(wrapped.lines.len(), 2, "{width}");
        let (a, b) = (&wrapped.lines[0], &wrapped.lines[1]);
        assert_eq!((a.start_char, b.start_char, b.end_char), (0, 9, 11));
        // "Hi " left to right, the first word right to left right of it.
        assert!(a.starts[3..8].windows(2).all(|w| w[1] < w[0]), "{a:?}");
        assert!(
            a.starts[3..8].iter().all(|x| *x >= a.ends[2] - 1e-6),
            "{a:?}"
        );
        // The caret after "Hi " and the end of each line where it reads.
        assert!(b.starts[1] < b.starts[0], "{b:?}");
        // One line high: the frame keeps the text up to "بك".
        let lh = a.ascent + a.descent;
        let one = lay(Some(Size::new(width, lh * 1.2)));
        assert!(one.overflow);
        assert_eq!(one.fitted_chars, 9);
    }

    /// Glyphs in text order are seen with every right-to-left stretch
    /// turned round, numbers inside one kept left to right, and spaces at
    /// a line's end at the line's own level.
    #[test]
    fn lines_are_seen_in_bidi_order() {
        let g = |level: u8, ch: char| Glyph {
            path: Arc::new(BezPath::new()),
            advance: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
            ch,
            char_idx: 0,
            char_end: 1,
            rtl: level % 2 == 1,
            level,
            span: 0,
            hyphen_after: None,
            src: None,
            joins: false,
        };
        let order = |v: &[(u8, char)]| {
            let glyphs: Vec<Glyph> = v.iter().map(|(l, c)| g(*l, *c)).collect();
            display_order(&glyphs)
        };
        assert_eq!(order(&[(0, 'a'), (0, 'b')]), None);
        assert_eq!(
            order(&[(0, 'a'), (0, ' '), (1, 'x'), (1, 'y')]),
            Some(vec![0, 1, 3, 2])
        );
        // Right to left with a number: the number reads left to right.
        assert_eq!(
            order(&[(1, 'x'), (1, ' '), (2, '1'), (2, '2')]),
            Some(vec![2, 3, 1, 0])
        );
        // A space ending the line goes to the line's level.
        assert_eq!(
            order(&[(0, 'a'), (1, 'x'), (1, 'y'), (1, ' ')]),
            Some(vec![0, 2, 1, 3])
        );
    }

    /// A right-to-left word typed after a left-to-right one is shaped and
    /// laid out right to left, to the right of it: its first letter at its
    /// right end, as on its own; and the other way round (#628).
    #[test]
    fn mixed_directions_lay_each_run_its_own_way() {
        if !has_fonts() {
            return;
        }
        let lay = |t: &str| {
            fonts().layout(&TextRequest {
                spans: &[span(t)],
                frame: None,
                align: TextAlign::Left,
                para: &ParagraphStyle::default(),
                on_path: None,
            })
        };
        let ar = "\u{645}\u{631}\u{62D}\u{628}\u{627}";
        let alone = lay(ar);
        if alone.glyphs == 0 {
            return;
        }
        let word = alone.lines[0].x_range();
        let mixed = lay(&format!("Asasa {ar}"));
        let line = &mixed.lines[0];
        // "Asasa " left to right from 0, then the Arabic word.
        assert!(
            line.edges[..7].windows(2).take(5).all(|w| w[1] > w[0]),
            "{:?}",
            line.edges
        );
        let start = line.ends[5];
        assert!(
            (line.edges[6] - (start + word.1 - word.0)).abs() < 1e-6,
            "{:?}",
            line.edges
        );
        assert!(
            line.edges[6..].windows(2).all(|w| w[1] < w[0]),
            "{:?}",
            line.edges
        );
        assert!((line.edges[11] - start).abs() < 1e-6, "{:?}", line.edges);
        // Each letter where it is on its own, moved by the Latin word.
        for i in 0..5 {
            let shifted = alone.lines[0].starts[i] + start;
            assert!((line.starts[6 + i] - shifted).abs() < 1e-6, "{i}");
        }
        // Right to left first: the Latin word at the left, the Arabic to
        // its right, the caret before the Latin word at its left.
        let other = lay(&format!("{ar} Asasa"));
        let line = &other.lines[0];
        assert!(line.edges[0] > line.edges[5], "{:?}", line.edges);
        assert!(line.edges[6].abs() < 1e-6, "{:?}", line.edges);
        assert!(line.starts[5] >= line.ends[10] - 1e-6, "{:?}", line);
    }

    fn span(text: &str) -> TextSpan {
        TextSpan {
            text: text.into(),
            font_family: "DejaVu Sans".into(),
            size_pt: 12.0,
            bold: false,
            italic: false,
            tracking_pct: 0.0,
            baseline_shift_pt: 0.0,
            offset_x_pct: 0.0,
            angle_deg: 0.0,
            underline: false,
            strikethrough: false,
            fill: None,
            background: None,
            outline: None,
            layout_only: false,
            features: Vec::new(),
            paragraph: None,
        }
    }

    fn has_fonts() -> bool {
        !fonts().families().is_empty()
    }

    #[test]
    fn shapes_some_text_if_any_font_exists() {
        let f = fonts();
        let layout = f.outline(&[span("Ab")]);
        if !has_fonts() {
            assert_eq!(layout.glyphs, 0);
        } else {
            assert_eq!(layout.glyphs, 2);
            assert!(layout.bounds.width() > 1.0);
            assert!(!layout.path.elements().is_empty());
        }
    }

    #[test]
    fn shifted_and_rotated_characters_leave_the_others_in_place() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let plain = f.outline(&[span("AB")]);
        let b_box = |l: &TextLayout| {
            // The second glyph's outline: the path after the first glyph's
            // subpaths, found by its x position.
            l.path.bounding_box()
        };
        let mut a = span("A");
        a.offset_x_pct = 50.0;
        let shifted = f.outline(&[a.clone(), span("B")]);
        // A moved right by half the size; B did not move, so the whole
        // outline grows on the left side only by what A covers.
        let size = 12.0 * PT_MM;
        assert!((shifted.path.bounding_box().x0 - (b_box(&plain).x0 + 0.5 * size)).abs() < 1e-6);
        assert!((shifted.path.bounding_box().x1 - b_box(&plain).x1).abs() < 1e-6);
        assert_eq!(shifted.lines[0].edges, plain.lines[0].edges);
        // Turning A a quarter turn about its origin puts it left of x = 0.
        let mut a = span("A");
        a.angle_deg = 90.0;
        let turned = f.outline(&[a, span("B")]);
        assert!(turned.path.bounding_box().x0 < -1.0);
    }

    #[test]
    fn character_and_word_spacing_widen_the_text() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let measure = |para: &ParagraphStyle| {
            let l = f.layout(&TextRequest {
                spans: &[span("a b c")],
                frame: None,
                align: TextAlign::Left,
                para,
                on_path: None,
            });
            *l.lines[0].edges.last().expect("edges")
        };
        let base = measure(&ParagraphStyle::default());
        let wide = measure(&ParagraphStyle {
            char_spacing_pct: 100.0,
            ..Default::default()
        });
        let words = measure(&ParagraphStyle {
            word_spacing_pct: 300.0,
            ..Default::default()
        });
        // Five characters, each one space wider; two spaces three times as
        // wide.
        let space = f.measure(&span(" "), " ");
        assert!(
            (wide - base - 5.0 * space).abs() < 1e-6,
            "{base} {wide} {space}"
        );
        assert!(
            (words - base - 4.0 * space).abs() < 1e-6,
            "{base} {words} {space}"
        );
    }

    #[test]
    fn drop_cap_spans_its_lines_without_rising_above_the_text() {
        // The placement alone: a 2 mm tall letter over 3 lines 5 mm apart
        // sits on the third baseline and reaches the first line's top.
        let mut glyph = BezPath::new();
        glyph.move_to((0.0, 0.0));
        glyph.line_to((1.0, 2.0));
        glyph.line_to((2.0, 0.0));
        glyph.close_path();
        let (scale, base) = drop_cap_placement(&glyph, 3.0, 5.0, -4.0, 3.0);
        assert!((base - (-14.0)).abs() < 1e-9, "{base}");
        assert!((base + 2.0 * scale - (-4.0 + 2.0)).abs() < 1e-9, "{scale}");
        if !has_fonts() {
            return;
        }
        // In a laid-out paragraph the drop cap neither rises above the
        // first line nor reaches below the text without one.
        let f = fonts();
        let words = "Words in a paragraph long enough to wrap over several lines, \
                     so the drop cap has room to span three of them.";
        let lay = |drop_cap_lines: u32| {
            f.layout(&TextRequest {
                spans: &[span(words)],
                frame: Some(Size::new(40.0, 100.0)),
                align: TextAlign::Left,
                para: &ParagraphStyle {
                    drop_cap_lines,
                    ..Default::default()
                },
                on_path: None,
            })
        };
        let plain = lay(0).path.bounding_box();
        let capped_layout = lay(3);
        let capped = capped_layout.path.bounding_box();
        assert!(capped.y1 <= plain.y1 + 0.3, "{capped:?} {plain:?}");
        assert!(capped.y0 >= plain.y0 - 6.0, "{capped:?} {plain:?}");
        // The lines beside the drop cap are narrowed, not pushed out of
        // the frame.
        for l in &capped_layout.lines {
            let right = l.edges.last().copied().unwrap_or(0.0);
            assert!(right <= 40.0 + 0.01, "line ends at {right}");
        }
    }

    #[test]
    fn paragraph_wraps_to_frame_width() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("one two three four five six seven eight nine ten");
        let para = ParagraphStyle::default();
        let one_line = f.outline(std::slice::from_ref(&s));
        let wrapped = f.layout(&TextRequest {
            spans: &[s],
            frame: Some(Size::new(one_line.bounds.width() / 2.5, 100.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        assert!(wrapped.bounds.height() > one_line.bounds.height() * 2.0);
        assert!(!wrapped.overflow);
    }

    #[test]
    fn tracking_widens_text() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let a = f.outline(&[span("Hello")]).bounds.width();
        let mut t = span("Hello");
        t.tracking_pct = 50.0;
        let b = f.outline(&[t]).bounds.width();
        assert!(b > a * 1.5);
    }

    #[test]
    fn caret_geometry_follows_the_characters_and_lines() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let l = f.outline(&[span("ab\ncd")]);
        assert_eq!(l.lines.len(), 2);
        assert_eq!((l.lines[0].start_char, l.lines[0].end_char), (0, 2));
        assert_eq!((l.lines[1].start_char, l.lines[1].end_char), (3, 5));
        let c0 = l.caret(0).unwrap();
        let c1 = l.caret(1).unwrap();
        let c2 = l.caret(2).unwrap();
        let c3 = l.caret(3).unwrap();
        let c5 = l.caret(5).unwrap();
        assert!(c1.0 > c0.0 && c2.0 > c1.0, "carets advance along the line");
        assert!(c3.1 < c0.1, "the second line is below the first");
        assert!((c3.0 - c0.0).abs() < 1e-6, "line starts share x");
        assert!(c5.0 > c3.0);
        assert_eq!(l.line_of(2), Some(0));
        assert_eq!(l.line_of(3), Some(1));
        // Clicking before the first character of the second line, and
        // after the last of the first.
        assert_eq!(l.hit_char(Point::new(c3.0 - 1.0, c3.1)), Some(3));
        assert_eq!(l.hit_char(Point::new(c2.0 + 50.0, c0.1)), Some(2));
        // A caret past the end sits after the last character.
        assert_eq!(l.caret(99), Some(c5));
    }

    #[test]
    fn overflow_is_reported_and_fit_to_frame_fixes_it() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("a lot of text that will not fit in a tiny frame at all");
        let para = ParagraphStyle::default();
        let fit = ParagraphStyle {
            fit_to_frame: true,
            ..ParagraphStyle::default()
        };
        fn req<'a>(s: &'a TextSpan, p: &'a ParagraphStyle) -> TextRequest<'a> {
            TextRequest {
                spans: std::slice::from_ref(s),
                frame: Some(Size::new(20.0, 5.0)),
                align: TextAlign::Left,
                para: p,
                on_path: None,
            }
        }
        let over = f.layout(&req(&s, &para));
        assert!(over.overflow);
        assert!(!f.layout(&req(&s, &fit)).overflow);
        // The lines that do not fit are not drawn: nothing reaches more
        // than a line below the frame (5 mm tall, top at y = 0).
        use kurbo::Shape as _;
        let b = over.path.bounding_box();
        assert!(b.y0 > -5.0 - 3.0, "{b:?}");
        // Laid out in a tall frame, the same text draws more.
        let tall = f.layout(&TextRequest {
            spans: std::slice::from_ref(&s),
            frame: Some(Size::new(20.0, 100.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        assert!(tall.path.elements().len() > over.path.elements().len());
    }

    #[test]
    fn paragraph_text_sits_at_the_top_middle_or_bottom_of_its_frame() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("one\ntwo");
        let lay = |v: VerticalAlign, h: f64| {
            let para = ParagraphStyle {
                vertical_align: v,
                ..ParagraphStyle::default()
            };
            f.layout(&TextRequest {
                spans: std::slice::from_ref(&s),
                frame: Some(Size::new(100.0, h)),
                align: TextAlign::Left,
                para: &para,
                on_path: None,
            })
        };
        let top = lay(VerticalAlign::Top, 100.0);
        let mid = lay(VerticalAlign::Center, 100.0);
        let bottom = lay(VerticalAlign::Bottom, 100.0);
        // The bottom of the text: at the frame's bottom for Bottom, the
        // room shared evenly above and below for Center.
        let room = 100.0 + top.bounds.y0;
        assert!(room > 10.0, "{:?}", top.bounds);
        assert!(
            (bottom.bounds.y0 + 100.0).abs() < 1e-6,
            "{:?}",
            bottom.bounds
        );
        assert!((mid.bounds.y0 - (top.bounds.y0 - room / 2.0)).abs() < 1e-6);
        // Every baseline and the caret lines move with the text.
        for (a, b) in top.baselines.iter().zip(&bottom.baselines) {
            assert!((b.1 - (a.1 - room)).abs() < 1e-6);
        }
        assert!((bottom.lines[0].baseline - (top.lines[0].baseline - room)).abs() < 1e-6);
        use kurbo::Shape as _;
        let (tb, bb) = (top.path.bounding_box(), bottom.path.bounding_box());
        assert!((bb.y0 - (tb.y0 - room)).abs() < 1e-6);
        // Text that overflows starts at the top.
        let small = lay(VerticalAlign::Bottom, 3.0);
        let small_top = lay(VerticalAlign::Top, 3.0);
        assert!(small.overflow);
        assert_eq!(small.baselines, small_top.baselines);
    }

    #[test]
    fn justify_fills_the_width() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("aa bb cc dd ee ff gg hh ii jj kk ll mm nn oo pp");
        let para = ParagraphStyle::default();
        let l = f.layout(&TextRequest {
            spans: &[s],
            frame: Some(Size::new(30.0, 100.0)),
            align: TextAlign::Justify,
            para: &para,
            on_path: None,
        });
        assert!((l.bounds.width() - 30.0).abs() < 0.01);
    }

    #[test]
    fn forced_justification_spreads_the_last_line_too() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("aa bb cc dd ee ff gg hh ii jj kk ll mm nn oo pp qq");
        let para = ParagraphStyle::default();
        let lay = |align| {
            f.layout(&TextRequest {
                spans: std::slice::from_ref(&s),
                frame: Some(Size::new(30.0, 100.0)),
                align,
                para: &para,
                on_path: None,
            })
        };
        let last_width = |l: &TextLayout| {
            let line = l.lines.last().expect("lines");
            line.edges.last().copied().unwrap_or(0.0) - line.edges.first().copied().unwrap_or(0.0)
        };
        let full = lay(TextAlign::Justify);
        let forced = lay(TextAlign::ForceJustify);
        assert!(full.lines.len() > 1);
        assert!(last_width(&forced) > last_width(&full) + 1.0);
        // No alignment lays out as left.
        assert_eq!(
            lay(TextAlign::None).baselines,
            lay(TextAlign::Left).baselines
        );
    }

    #[test]
    fn line_spacing_in_points_and_in_percent_of_the_size() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("one\ntwo");
        let gap = |para: ParagraphStyle| {
            let l = f.layout(&TextRequest {
                spans: std::slice::from_ref(&s),
                frame: Some(Size::new(100.0, 100.0)),
                align: TextAlign::Left,
                para: &para,
                on_path: None,
            });
            l.lines[0].baseline - l.lines[1].baseline
        };
        let pts = gap(ParagraphStyle {
            leading_unit: tracedraw_core::LeadingUnit::Points,
            leading_pct: 36.0,
            ..ParagraphStyle::default()
        });
        assert!((pts - 36.0 * PT_MM).abs() < 1e-6, "{pts}");
        let size = s.size_pt * PT_MM;
        let pct = gap(ParagraphStyle {
            leading_unit: tracedraw_core::LeadingUnit::PercentPointSize,
            leading_pct: 150.0,
            ..ParagraphStyle::default()
        });
        assert!((pct - 1.5 * size).abs() < 1e-6, "{pct}");
    }

    #[test]
    fn spans_only_laid_out_keep_their_room_but_are_not_drawn() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let whole = f.outline(&[span("ab"), span("cd")]);
        let mut hidden = span("ab");
        hidden.layout_only = true;
        let mut ul = span("cd");
        ul.underline = true;
        ul.layout_only = true;
        let part = f.outline(&[hidden, span("cd")]);
        let none = f.outline(&[span("ab"), ul]);
        // The visible letters stay where they are in the whole text.
        let wb = whole.path.bounding_box();
        let pb = part.path.bounding_box();
        assert!(pb.x0 > wb.x0 + 0.5, "{pb:?} {wb:?}");
        assert!((pb.x1 - wb.x1).abs() < 1e-6, "{pb:?} {wb:?}");
        assert_eq!(part.lines, whole.lines);
        // A hidden span draws neither its letters nor its underline.
        let nb = none.path.bounding_box();
        assert!(nb.x1 < wb.x1 - 0.5, "{nb:?} {wb:?}");
        assert_eq!(
            part.path.elements().len() + none.path.elements().len(),
            whole.path.elements().len()
        );
    }

    #[test]
    fn character_backgrounds_cover_their_characters() {
        if !has_fonts() {
            return;
        }
        install();
        let mut a = span("ab");
        a.background = Some(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
            255, 255, 0,
        )));
        let b = span("cd");
        let kind = tracedraw_core::ShapeKind::Text {
            spans: vec![a, b],
            origin: Point::new(10.0, 20.0),
            frame: None,
            align: TextAlign::Left,
            para: ParagraphStyle::default(),
            on_path: None,
        };
        let rects = tracedraw_core::document::text_backgrounds(&kind);
        assert_eq!(rects.len(), 1);
        let r = rects[0].0;
        assert!((r.x0 - 10.0).abs() < 1e-6, "{r:?}");
        assert!(r.y0 < 20.0 && r.y1 > 20.0, "{r:?}");
        // The background stops where "cd" starts.
        let whole = tracedraw_core::document::text_outline::lines(
            &tracedraw_core::document::text_outline::TextRequest {
                spans: match &kind {
                    tracedraw_core::ShapeKind::Text { spans, .. } => spans,
                    _ => unreachable!(),
                },
                frame: None,
                align: TextAlign::Left,
                para: &ParagraphStyle::default(),
                on_path: None,
            },
        )
        .expect("lines");
        let end = whole[0].edges.last().copied().unwrap_or(0.0) + 10.0;
        assert!(r.x1 < end - 0.5, "{r:?} {end}");
    }

    #[test]
    fn custom_hyphens_come_before_the_patterns() {
        hyphen::set_custom(vec![("zebracorn".into(), vec![3])]);
        assert_eq!(hyphen::break_points("Zebracorn"), vec![3]);
        hyphen::set_custom(Vec::new());
        assert_ne!(hyphen::break_points("zebracorn"), vec![3]);
    }

    fn frame_layout(
        f: &FontSystem,
        spans: &[TextSpan],
        para: &ParagraphStyle,
        w: f64,
    ) -> TextLayout {
        f.layout(&TextRequest {
            spans,
            frame: Some(Size::new(w, 200.0)),
            align: TextAlign::Left,
            para,
            on_path: None,
        })
    }

    #[test]
    fn paragraphs_with_their_own_alignment_and_spacing_lay_out_apart() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let mut kind = tracedraw_core::ShapeKind::Text {
            spans: vec![span("left\nright")],
            origin: Point::ZERO,
            frame: Some(Size::new(100.0, 100.0)),
            align: TextAlign::Left,
            para: ParagraphStyle::default(),
            on_path: None,
        };
        let plain = match &kind {
            tracedraw_core::ShapeKind::Text { spans, para, .. } => {
                frame_layout(f, spans, para, 100.0)
            }
            _ => unreachable!(),
        };
        // The second paragraph right aligned, with 4 mm before it.
        tracedraw_core::paragraphs::edit_paragraph_formats(&mut kind, Some((7, 7)), |p| {
            p.align = TextAlign::Right;
            p.space_before = 4.0;
        });
        let tracedraw_core::ShapeKind::Text { spans, para, .. } = &kind else {
            unreachable!()
        };
        let l = frame_layout(f, spans, para, 100.0);
        assert_eq!(l.baselines.len(), 2);
        // The first paragraph is where it was, at the left.
        assert!((l.baselines[0].0 - plain.baselines[0].0).abs() < 1e-9);
        assert!((l.baselines[0].1 - plain.baselines[0].1).abs() < 1e-9);
        // The second one ends at the right edge and is 4 mm lower.
        let right_end = l.lines[1].edges.last().copied().unwrap_or(0.0);
        assert!((right_end - 100.0).abs() < 0.05, "{right_end}");
        assert!(l.baselines[1].0 > 50.0);
        assert!((plain.baselines[1].1 - l.baselines[1].1 - 4.0).abs() < 1e-6);
    }

    fn wrap_square(
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        side: tracedraw_core::WrapSide,
    ) -> WrapRegion {
        WrapRegion::from_path(
            &Rect::new(x0, y0, x1, y1).to_path(0.01),
            tracedraw_core::TextWrap {
                mode: tracedraw_core::WrapMode::Square,
                side,
                offset: 1.0,
            },
        )
        .unwrap()
    }

    fn words(n: usize) -> TextSpan {
        span(&vec!["word"; n].join(" "))
    }

    #[test]
    fn paragraph_text_flows_around_a_wrap_object() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = words(120);
        let plain = ParagraphStyle::default();
        let free = frame_layout(f, std::slice::from_ref(&s), &plain, 100.0);
        // A square in the middle of the frame, 20 to 40 mm down: lines
        // beside it come in two parts, none crosses it.
        let mut para = ParagraphStyle::default();
        para.flow = tracedraw_core::text_flow::FrameFlow::new(
            vec![wrap_square(
                40.0,
                -40.0,
                60.0,
                -20.0,
                tracedraw_core::WrapSide::Straddle,
            )],
            None,
        );
        let l = frame_layout(f, std::slice::from_ref(&s), &para, 100.0);
        assert!(l.lines.len() > free.lines.len());
        let mut split_rows = 0;
        for line in &l.lines {
            let (x0, x1) = (line.edges[0], line.edges.last().copied().unwrap_or(0.0));
            let band = (line.baseline - line.descent, line.baseline + line.ascent);
            if band.0 < -19.0 && band.1 > -41.0 {
                // Beside the square: wholly left of 39 or right of 61.
                assert!(
                    x1 <= 39.0 + 0.05 || x0 >= 61.0 - 0.05,
                    "{x0}..{x1} at {band:?}"
                );
                if x0 >= 61.0 - 0.05 {
                    split_rows += 1;
                }
            }
        }
        assert!(split_rows >= 2, "{split_rows}");
        // The text keeps its order: carets run on through both halves.
        let starts: Vec<usize> = l.lines.iter().map(|l| l.start_char).collect();
        assert!(starts.windows(2).all(|w| w[0] < w[1]));
        // Hit testing finds the right half of a split row.
        let right = l.lines.iter().find(|l| l.edges[0] >= 61.0 - 0.05).unwrap();
        let hit = l
            .hit_char(Point::new(right.edges[1], right.baseline))
            .unwrap();
        assert!(
            hit >= right.start_char && hit <= right.end_char,
            "{hit} {right:?}"
        );
    }

    #[test]
    fn wrap_sides_keep_the_text_left_right_or_above_and_below() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = words(120);
        let band = |side| {
            let mut para = ParagraphStyle::default();
            para.flow = tracedraw_core::text_flow::FrameFlow::new(
                vec![wrap_square(40.0, -40.0, 60.0, -20.0, side)],
                None,
            );
            let l = frame_layout(f, std::slice::from_ref(&s), &para, 100.0);
            l.lines
                .iter()
                .filter(|line| {
                    line.baseline - line.descent < -19.0 && line.baseline + line.ascent > -41.0
                })
                .map(|line| (line.edges[0], line.edges.last().copied().unwrap_or(0.0)))
                .collect::<Vec<_>>()
        };
        let left = band(tracedraw_core::WrapSide::Left);
        assert!(
            !left.is_empty() && left.iter().all(|(_, x1)| *x1 <= 39.05),
            "{left:?}"
        );
        let right = band(tracedraw_core::WrapSide::Right);
        assert!(
            !right.is_empty() && right.iter().all(|(x0, _)| *x0 >= 60.95),
            "{right:?}"
        );
        // Above and below: no line at the square's height at all.
        assert!(band(tracedraw_core::WrapSide::AboveBelow).is_empty());
    }

    #[test]
    fn the_page_baseline_grid_lines_up_frames_at_different_heights() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("one\ntwo\nthree");
        // Grid lines at y = 1.3 + 5k in the frame's layout space (the page
        // grid seen from this frame).
        let mut para = ParagraphStyle {
            baseline_grid_mm: 5.0,
            ..ParagraphStyle::default()
        };
        para.flow = tracedraw_core::text_flow::FrameFlow::new(Vec::new(), Some((1.3, 5.0)));
        let l = frame_layout(f, std::slice::from_ref(&s), &para, 100.0);
        assert_eq!(l.baselines.len(), 3);
        for (_, y) in &l.baselines {
            let k = (y - 1.3) / 5.0;
            assert!((k - k.round()).abs() < 1e-6, "{y}");
        }
    }

    #[test]
    fn baseline_grid_snaps_every_baseline_to_the_pitch() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("one\ntwo\nthree");
        let para = ParagraphStyle {
            baseline_grid_mm: 5.0,
            ..ParagraphStyle::default()
        };
        let l = f.layout(&TextRequest {
            spans: std::slice::from_ref(&s),
            frame: Some(Size::new(100.0, 100.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        assert_eq!(l.baselines.len(), 3);
        let mut prev = 0.0;
        for (_, y) in &l.baselines {
            assert!(*y < prev, "baselines go down: {:?}", l.baselines);
            let k = -y / 5.0;
            assert!(
                (k - k.round()).abs() < 1e-6 && k.round() >= 1.0,
                "baseline {y} is not on the 5 mm grid: {:?}",
                l.baselines
            );
            prev = *y;
        }
        assert!(!l.overflow);
        assert_eq!(l.fitted_chars, 13);
        // Without the grid the same text has baselines off the grid
        // (12 pt lines are about 5.9 mm apart, the first one one ascent down).
        let free = ParagraphStyle::default();
        let l2 = f.layout(&TextRequest {
            spans: std::slice::from_ref(&s),
            frame: Some(Size::new(100.0, 100.0)),
            align: TextAlign::Left,
            para: &free,
            on_path: None,
        });
        assert_eq!(l2.baselines.len(), 3);
        assert!(l2.baselines.iter().any(|(_, y)| {
            let k = -y / 5.0;
            (k - k.round()).abs() > 1e-3
        }));
        // Snapping never moves a baseline up.
        for (a, b) in l.baselines.iter().zip(&l2.baselines) {
            assert!(a.1 <= b.1 + 1e-9, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn fitted_chars_counts_what_the_frame_holds() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("one\ntwo\nthree");
        let para = ParagraphStyle::default();
        // Artistic text: everything fits by definition.
        assert_eq!(f.outline(std::slice::from_ref(&s)).fitted_chars, 13);
        // A frame tall enough for one 12 pt line only.
        let l = f.layout(&TextRequest {
            spans: std::slice::from_ref(&s),
            frame: Some(Size::new(60.0, 6.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        assert!(l.overflow);
        assert_eq!(
            l.fitted_chars, 4,
            "\"one\\n\" fits, \"two\" starts the overflow"
        );
        let (head, tail) = tracedraw_core::split_spans_at(std::slice::from_ref(&s), l.fitted_chars);
        assert_eq!(head[0].text, "one\n");
        assert_eq!(tail[0].text, "two\nthree");
        // A frame that holds everything reports the full count.
        let l = f.layout(&TextRequest {
            spans: std::slice::from_ref(&s),
            frame: Some(Size::new(60.0, 100.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        assert!(!l.overflow);
        assert_eq!(l.fitted_chars, 13);
    }

    #[test]
    fn fitted_chars_splits_inside_a_wrapped_paragraph_and_across_spans() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let a = span("aaaa bbbb ");
        let mut b = span("cccc dddd eeee");
        b.underline = true;
        let spans = vec![a, b];
        let para = ParagraphStyle::default();
        let one_line = f.outline(&[span("aaaa bbbb")]);
        // Width for one word pair per line, height for two lines.
        let l = f.layout(&TextRequest {
            spans: &spans,
            frame: Some(Size::new(one_line.bounds.width() + 1.0, 11.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        assert!(l.overflow);
        assert_eq!(
            l.fitted_chars, 20,
            "two lines of two words: 'aaaa bbbb ' + 'cccc dddd ' = 20 chars"
        );
        let (head, tail) = tracedraw_core::split_spans_at(&spans, l.fitted_chars);
        assert_eq!(head.len(), 2);
        assert_eq!(head[1].text, "cccc dddd ");
        assert!(head[1].underline);
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].text, "eeee");
        assert!(tail[0].underline);
    }

    #[test]
    fn text_on_path_follows_the_curve() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("curve");
        let para = ParagraphStyle::default();
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((0.0, 50.0));
        let tp = TextOnPath {
            path: p,
            offset: 0.0,
            distance: 0.0,
            mirror: false,
            ..Default::default()
        };
        let l = f.layout(&TextRequest {
            spans: &[s],
            frame: None,
            align: TextAlign::Left,
            para: &para,
            on_path: Some(&tp),
        });
        let b = l.path.bounding_box();
        assert!(
            b.height() > b.width(),
            "vertical path should give tall text: {b:?}"
        );
    }

    fn on_path(f: &FontSystem, text: &str, tp: &TextOnPath, align: TextAlign) -> BezPath {
        let s = span(text);
        f.layout(&TextRequest {
            spans: &[s],
            frame: None,
            align,
            para: &ParagraphStyle::default(),
            on_path: Some(tp),
        })
        .path
    }

    #[test]
    fn mirrored_text_on_a_path_reads_along_its_other_side() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((100.0, 0.0));
        let mut tp = TextOnPath {
            path: line,
            offset: 0.0,
            distance: 2.0,
            mirror: false,
            ..Default::default()
        };
        let above = on_path(f, "Fit", &tp, TextAlign::Center);
        tp.mirror = true;
        let below = on_path(f, "Fit", &tp, TextAlign::Center);
        // The same letters turned half round about the line's middle: in
        // order when the page is turned, 2 mm below the line.
        let turned = Affine::translate((100.0, 0.0)) * Affine::scale(-1.0) * above.clone();
        assert_eq!(below.elements().len(), turned.elements().len());
        let ends = |p: &BezPath| -> Vec<Point> {
            p.elements().iter().filter_map(|e| e.end_point()).collect()
        };
        for (a, b) in ends(&below).iter().zip(ends(&turned).iter()) {
            assert!((*a - *b).hypot() < 1e-6, "{a:?} {b:?}");
        }
        assert!(above.bounding_box().y0 > 1.5);
        assert!(below.bounding_box().y1 < -1.5);
    }

    fn flat_line() -> BezPath {
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((100.0, 0.0));
        line
    }

    #[test]
    fn the_letters_line_on_the_path_follows_the_vertical_placement() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let mut tp = TextOnPath {
            path: flat_line(),
            ..Default::default()
        };
        let base = on_path(f, "Hxg", &tp, TextAlign::Left).bounding_box();
        // Baseline: "g" hangs below, "H" stands above.
        assert!(base.y0 < -0.3 && base.y1 > 2.0, "{base:?}");
        tp.vertical = PathTextVertical::Ascender;
        let asc = on_path(f, "Hxg", &tp, TextAlign::Left).bounding_box();
        assert!(asc.y1 <= 0.05 && asc.y1 > -1.5, "{asc:?}");
        tp.vertical = PathTextVertical::Descender;
        let desc = on_path(f, "Hxg", &tp, TextAlign::Left).bounding_box();
        assert!(desc.y0 >= -0.05 && desc.y0 < 1.0, "{desc:?}");
        tp.vertical = PathTextVertical::Center;
        let mid = on_path(f, "Hxg", &tp, TextAlign::Left).bounding_box();
        assert!(mid.y0 < 0.0 && mid.y1 > 0.0, "{mid:?}");
        assert!((mid.y0 + mid.y1).abs() < base.height() * 0.5, "{mid:?}");
    }

    #[test]
    fn single_mirrors_flip_the_letters_and_both_turn_them_round() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let mut tp = TextOnPath {
            path: flat_line(),
            distance: 2.0,
            ..Default::default()
        };
        let plain = on_path(f, "L", &tp, TextAlign::Left);
        // Vertical mirror: the same letter upside down below the line.
        tp.set_mirrors(false, true);
        let v = on_path(f, "L", &tp, TextAlign::Left);
        let flipped = Affine::scale_non_uniform(1.0, -1.0) * plain.clone();
        let ends = |p: &BezPath| -> Vec<Point> {
            p.elements().iter().filter_map(|e| e.end_point()).collect()
        };
        for (a, b) in ends(&v).iter().zip(ends(&flipped).iter()) {
            assert!((*a - *b).hypot() < 1e-6, "{a:?} {b:?}");
        }
        // Horizontal mirror: above the line, reading from its end.
        tp.set_mirrors(true, false);
        let h = on_path(f, "L", &tp, TextAlign::Left).bounding_box();
        assert!(h.y0 > 1.5 && h.x1 > 95.0, "{h:?}");
        // Both: the old mirror, turned half round below the line.
        tp.set_mirrors(true, true);
        let both = on_path(f, "L", &tp, TextAlign::Left).bounding_box();
        assert!(both.y1 < -1.5 && both.x1 > 95.0, "{both:?}");
        assert_eq!(tp.mirrors(), (true, true));
    }

    #[test]
    fn upright_and_skewed_letters_keep_their_strokes_straight() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        // A 45 degree line.
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((100.0, 100.0));
        let mut tp = TextOnPath {
            path: line,
            ..Default::default()
        };
        let straight = f.outline(&[span("I")]).path.bounding_box();
        let rotated = on_path(f, "I", &tp, TextAlign::Left).bounding_box();
        tp.orientation = PathTextOrientation::Upright;
        let upright = on_path(f, "I", &tp, TextAlign::Left).bounding_box();
        // Upright: the letter keeps its own size; turned, its box grows.
        assert!(
            (upright.width() - straight.width()).abs() < 1e-6,
            "{upright:?}"
        );
        assert!(
            (upright.height() - straight.height()).abs() < 1e-6,
            "{upright:?}"
        );
        assert!(rotated.width() > straight.width() + 1.0, "{rotated:?}");
        // Vertical skew keeps the height; horizontal skew keeps the width
        // of each horizontal slice and leans the letter.
        tp.orientation = PathTextOrientation::VerticalSkew;
        let vs = on_path(f, "I", &tp, TextAlign::Left).bounding_box();
        assert!((vs.width() - straight.width()).abs() < 1e-6, "{vs:?}");
        assert!(vs.height() > straight.height(), "{vs:?}");
        tp.orientation = PathTextOrientation::HorizontalSkew;
        let hs = on_path(f, "I", &tp, TextAlign::Left).bounding_box();
        assert!((hs.height() - straight.height()).abs() < 1e-6, "{hs:?}");
        assert!(hs.width() > straight.width() + 1.0, "{hs:?}");
    }

    #[test]
    fn a_text_refitted_to_another_path_of_the_same_kind_is_laid_out_again() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let e = |cx: f64| {
            tracedraw_core::geometry::Ellipse::new((cx, 0.0), (40.0, 20.0), 0.0).to_path(0.01)
        };
        let s = span("same");
        let para = ParagraphStyle::default();
        let first = TextOnPath {
            path: e(0.0),
            ..Default::default()
        };
        let second = TextOnPath {
            path: e(200.0),
            ..Default::default()
        };
        assert_eq!(first.path.elements().len(), second.path.elements().len());
        let out = |tp: &TextOnPath| {
            f.outline_cached(&TextRequest {
                spans: std::slice::from_ref(&s),
                frame: None,
                align: TextAlign::Left,
                para: &para,
                on_path: Some(tp),
            })
            .bounding_box()
        };
        let a = out(&first);
        let b = out(&second);
        assert!(b.x0 > a.x1, "{a:?} {b:?}");
    }

    #[test]
    fn text_on_a_closed_path_continues_past_its_start() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        // A 100 mm square from the origin, counter-clockwise; the text
        // starts 5 mm before the end of the last side.
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((100.0, 0.0));
        path.line_to((100.0, 100.0));
        path.line_to((0.0, 100.0));
        path.close_path();
        let tp = TextOnPath {
            path,
            offset: 395.0,
            distance: 0.0,
            mirror: false,
            ..Default::default()
        };
        let b = on_path(f, "MMMMMM", &tp, TextAlign::Left).bounding_box();
        // On along the first side, not heaped at the corner.
        assert!(b.x1 > 10.0, "{b:?}");
        assert!(b.y1 < 10.0, "{b:?}");
    }

    /// The bundled interface font (SIL Open Font License).
    const SELAWIK: &[u8] =
        include_bytes!("../../../apps/tracedraw/assets/fonts/Selawik-Regular.ttf");
    /// A variable font (weight 300 to 700, 300 by default) with letters
    /// Selawik lacks ("Ĉ") and without one it has (U+0336).
    const QUICKSAND: &[u8] = include_bytes!("../testdata/Quicksand-VF.ttf");

    fn system_with(files: &[&[u8]]) -> FontSystem {
        let mut db = fontdb::Database::new();
        for f in files {
            db.load_font_data(f.to_vec());
        }
        FontSystem::from_db(db)
    }

    #[test]
    fn families_tell_how_they_are_built_and_their_styles() {
        const SELAWIK_BOLD: &[u8] =
            include_bytes!("../../../apps/tracedraw/assets/fonts/Selawik-Bold.ttf");
        let f = system_with(&[SELAWIK, SELAWIK_BOLD, QUICKSAND]);
        let selawik = f.family_info("Selawik").expect("installed");
        assert_eq!(selawik.kind, FontKind::TrueType);
        let names: Vec<&str> = selawik.faces.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Regular", "Bold"]);
        assert!(selawik.faces[1].weight >= 600 && !selawik.faces[1].italic);
        assert!(!selawik.embedded && !selawik.symbol);
        let quicksand = f.family_info("Quicksand").expect("installed");
        assert_eq!(quicksand.kind, FontKind::Variable);
        assert_eq!(quicksand.faces.len(), 1);
        // Found by its legacy name too.
        assert_eq!(
            f.family_info("Quicksand Light").map(|i| i.kind),
            Some(FontKind::Variable)
        );
        // An offered family: fetched when asked for, here by a font list.
        assert_eq!(f.family_info("Far Away"), None);
        f.offer_families(&["Far Away".to_string()]);
        let offered = f.family_info("Far Away").expect("offered");
        assert_eq!(offered.kind, FontKind::Offered);
        assert!(offered.faces.is_empty());
        assert!(f.request_family("Far Away"));
        assert!(!f.request_family("Nowhere"));
        assert_eq!(f.take_font_requests(), ["Far Away"]);
        // A family that came with a document says so.
        let g = system_with(&[]);
        g.add_document_fonts(&[DocumentFont::new("Selawik", SELAWIK.to_vec())]);
        assert!(g.family_info("Selawik").is_some_and(|i| i.embedded));
    }

    #[test]
    fn fonts_are_found_by_their_legacy_family_names() {
        // Quicksand lists "Quicksand" as its family and "Quicksand Light"
        // as its legacy one (name ID 1): text set in either finds it.
        let f = system_with(&[QUICKSAND]);
        assert!(f.has_family("Quicksand Light"));
        assert_eq!(f.substitute("Quicksand Light"), None);
        let a = f.outline(&[TextSpan::new("Hello", "Quicksand Light", 24.0)]);
        let b = f.outline(&[TextSpan::new("Hello", "Quicksand", 24.0)]);
        assert_eq!(a.path.elements().len(), b.path.elements().len());
        // A face listed under the name draws as that face does, at 400.
        assert!((a.bounds.width() - b.bounds.width()).abs() < 1e-9);
    }

    /// A family named with its weight keeps that weight where no face of
    /// that name is installed (#624): on a variable font's weight axis,
    /// as a thickened regular face, in a stand-in, and when fetching.
    #[test]
    fn a_weight_in_the_family_name_reaches_every_way_of_drawing() {
        let advance = |l: &TextLayout| l.lines[0].edges.last().copied().unwrap_or(0.0);
        let em = 100.0 * PT_MM;
        // The variable font at 600: between its 400 and 700 widths
        // (2339 and 2463 units of 1000 for "Hello").
        let f = system_with(&[QUICKSAND]);
        let semi = f.outline(&[TextSpan::new("Hello", "Quicksand SemiBold", 100.0)]);
        let w = advance(&semi) / em;
        assert!(w > 2.36 && w < 2.45, "{w}");
        // Only a regular face: a black weight is thickened as bold is.
        let g = system_with(&[SELAWIK]);
        let mut bold = TextSpan::new("Hello", "Selawik", 100.0);
        bold.bold = true;
        let black = g.outline(&[TextSpan::new("Hello", "Selawik Black", 100.0)]);
        let thick = g.outline(&[bold]);
        assert_eq!(black.path.elements().len(), thick.path.elements().len());
        let plain = g.outline(&[TextSpan::new("Hello", "Selawik", 100.0)]);
        assert_ne!(black.path.elements().len(), plain.path.elements().len());
        // A missing family's stand-in at the weight: the same as bold
        // text in the plain family name gets.
        const SELAWIK_BOLD: &[u8] =
            include_bytes!("../../../apps/tracedraw/assets/fonts/Selawik-Bold.ttf");
        let h = system_with(&[SELAWIK, SELAWIK_BOLD]);
        assert_eq!(
            h.face_for("Segoe UI SemiBold", false, false),
            h.face_for("Segoe UI", true, false),
            "Selawik's bold stands in"
        );
        assert_ne!(
            h.face_for("Segoe UI SemiBold", false, false),
            h.face_for("Segoe UI", false, false)
        );
        // Fetching: the family is asked for, not the name with the weight.
        let k = system_with(&[]);
        k.offer_families(&["Quicksand".into()]);
        let _ = k.outline(&[TextSpan::new("Hello", "Quicksand SemiBold", 24.0)]);
        assert_eq!(k.take_font_requests(), ["Quicksand"]);
        // A family of its own whose name ends in a weight word is fetched
        // as itself.
        let m = system_with(&[]);
        m.offer_families(&["Archivo".into(), "Archivo Black".into()]);
        let _ = m.outline(&[TextSpan::new("Hello", "Archivo Black", 24.0)]);
        assert_eq!(m.take_font_requests(), ["Archivo Black"]);
        // A face listed under a name ending in a weight word (as "Archivo
        // Black" is a regular face of its own family) draws at 400, not
        // at the word's weight: "Quicksand Light" is listed by the font.
        assert_eq!(
            f.face_and_weight("Quicksand Light", false, false)
                .map(|(_, w)| w),
            Some(400)
        );
        assert_eq!(
            f.face_and_weight("Quicksand SemiBold", false, false)
                .map(|(_, w)| w),
            Some(600)
        );
        // Names match their fonts' legacy names whatever the case.
        assert!(f.face_for("quicksand light", false, false).is_some());
        assert_eq!(target_weight("Lato Black", false), 900);
        assert_eq!(target_weight("Lato Light", true), 700);
        assert_eq!(target_weight("Lato", false), 400);
    }

    #[test]
    fn family_names_ending_in_a_weight_split() {
        assert_eq!(
            split_weight("Titillium Web SemiBold"),
            Some(("Titillium Web", 600))
        );
        assert_eq!(
            split_weight("Titillium Web Semi Bold"),
            Some(("Titillium Web", 600))
        );
        assert_eq!(split_weight("Segoe UI Semibold"), Some(("Segoe UI", 600)));
        assert_eq!(split_weight("Lato Light"), Some(("Lato", 300)));
        assert_eq!(split_weight("Arial Black"), Some(("Arial", 900)));
        assert_eq!(split_weight("Arial"), None);
        assert_eq!(split_weight("Arial Bold"), None);
        assert_eq!(split_weight("Light"), None);
        assert_eq!(split_weight("Semi Bold"), None);
    }

    #[test]
    fn a_family_named_with_its_weight_draws_with_the_nearest_face() {
        // "Selawik SemiBold" is not a family: drawn with Selawik's face
        // nearest 600 (its bold), not reported missing (#624).
        const SELAWIK_BOLD: &[u8] =
            include_bytes!("../../../apps/tracedraw/assets/fonts/Selawik-Bold.ttf");
        let f = system_with(&[SELAWIK, SELAWIK_BOLD]);
        assert!(f.has_family("Selawik SemiBold"));
        assert_eq!(f.substitute("Selawik SemiBold"), None);
        let bold = f.face_for("Selawik", true, false);
        assert_eq!(f.face_for("Selawik SemiBold", false, false), bold);
        assert_eq!(
            f.face_for("Selawik Light", false, false),
            f.face_for("Selawik", false, false)
        );
        assert!(!f.has_family("Missing SemiBold"));
    }

    #[test]
    fn a_drawing_brings_every_face_of_a_family() {
        const SELAWIK_BOLD: &[u8] =
            include_bytes!("../../../apps/tracedraw/assets/fonts/Selawik-Bold.ttf");
        let f = system_with(&[]);
        let regular = DocumentFont::new("Selawik", SELAWIK.to_vec());
        let bold = DocumentFont::new("Selawik", SELAWIK_BOLD.to_vec());
        // The bold face of the same family loads too.
        assert_eq!(
            f.add_document_fonts(std::slice::from_ref(&regular)),
            ["Selawik"]
        );
        assert_eq!(
            f.add_document_fonts(std::slice::from_ref(&bold)),
            ["Selawik"]
        );
        assert_eq!(f.db().len(), 2);
        // The same faces again: nothing new.
        assert!(f.add_document_fonts(&[regular, bold]).is_empty());
        assert_eq!(f.db().len(), 2);
        // Bold text is drawn with the bold face, not a thickened regular one.
        let mut span = TextSpan::new("Hello", "Selawik", 100.0);
        let plain = f.outline(std::slice::from_ref(&span));
        span.bold = true;
        let heavy = f.outline(&[span]);
        assert!(heavy.path.elements().len() < plain.path.elements().len() * 2);
        let advance = |l: &TextLayout| l.lines[0].edges.last().copied().unwrap_or(0.0);
        assert!((advance(&heavy) - advance(&plain)).abs() > 0.1);
    }

    #[test]
    fn fonts_the_application_offers_are_asked_for_once_and_then_draw() {
        let f = system_with(&[]);
        let _ = f.outline(&[TextSpan::new("Hello", "Quicksand", 24.0)]);
        assert!(f.take_font_requests().is_empty(), "nothing offered");
        let g = f.generation();
        f.offer_families(&["Quicksand".into(), "Unused Sans".into()]);
        assert!(f.generation() > g);
        assert_eq!(f.available_families(), ["Quicksand", "Unused Sans"]);
        assert!(!f.has_family("Quicksand"));
        // Text in the family asks for it, once in the session.
        let _ = f.outline(&[TextSpan::new("Hello", "Quicksand", 24.0)]);
        let _ = f.outline(&[TextSpan::new("Hello!", "Quicksand", 24.0)]);
        assert_eq!(f.take_font_requests(), ["Quicksand"]);
        let _ = f.outline(&[TextSpan::new("Hello?", "Quicksand", 24.0)]);
        assert!(f.take_font_requests().is_empty());
        // The files arrive: the family is installed, no longer offered,
        // and draws; a file that is not a font is skipped.
        let g = f.generation();
        let added = f.add_fonts(vec![QUICKSAND.to_vec(), b"not a font".to_vec()]);
        // The family, and the legacy name of its default instance.
        assert_eq!(added, ["Quicksand", "Quicksand Light"]);
        assert!(f.generation() > g);
        assert!(f.has_family("Quicksand"));
        assert_eq!(
            f.available_families(),
            ["Quicksand", "Quicksand Light", "Unused Sans"]
        );
        let l = f.outline(&[TextSpan::new("Hello", "Quicksand", 24.0)]);
        assert!(l.path.elements().len() > 20);
        assert!(f.add_fonts(vec![vec![1, 2, 3]]).is_empty());

        // A missing family asks for its stand-in, letters of another
        // script for the fonts made for it.
        let f = system_with(&[SELAWIK]);
        f.offer_families(&["Carlito".into(), "Noto Sans JP".into()]);
        let _ = f.outline(&[TextSpan::new("Hello", "Calibri", 24.0)]);
        assert_eq!(f.take_font_requests(), ["Carlito"]);
        let _ = f.outline(&[TextSpan::new("Hello \u{3042}", "Selawik", 24.0)]);
        assert_eq!(f.take_font_requests(), ["Noto Sans JP"]);
    }

    #[test]
    fn letters_a_font_lacks_are_drawn_with_a_font_that_has_them() {
        let f = system_with(&[SELAWIK, QUICKSAND]);
        let one = |family: &str, text: &str| f.outline(&[TextSpan::new(text, family, 100.0)]);
        // Selawik has no "Ĉ": Quicksand draws it, as it does on its own.
        let mixed = one("Selawik", "\u{108}");
        let own = one("Quicksand", "\u{108}");
        assert!(!own.path.elements().is_empty());
        assert_eq!(mixed.path.elements().len(), own.path.elements().len());
        let (a, b) = (mixed.path.bounding_box(), own.path.bounding_box());
        assert!(
            (a.x0 - b.x0).abs() < 1e-6 && (a.x1 - b.x1).abs() < 1e-6,
            "{a:?} {b:?}"
        );
        // Within a word: each letter from the face that has it, one run of
        // glyphs, characters in order.
        let word = one("Selawik", "A\u{108}A");
        assert_eq!(word.glyphs, 3);
        let edges = &word.lines[0].edges;
        assert_eq!(edges.len(), 4);
        assert!(edges.windows(2).all(|w| w[1] > w[0]), "{edges:?}");
        // The other way round, and a letter no face has: drawn with the
        // span's face, no panic.
        assert!(!one("Quicksand", "A\u{336}").path.elements().is_empty());
        let _ = one("Selawik", "\u{10FFFD}x");
    }

    /// A font made for the symbol encoding (only a Windows symbol
    /// character map, its glyphs at U+F020 to U+F0FF) draws the letters
    /// typed as their codes 0x20 to 0xFF, as legacy script fonts do, and
    /// lists them as those letters: they went to another font (#447,
    /// #469).
    #[test]
    fn a_symbol_font_draws_the_codes_typed() {
        // Selawik's "A", "B", "C" and space at U+F041.. only.
        const TEST_SYMBOL: &[u8] = include_bytes!("../testdata/TestSymbol.ttf");
        let f = system_with(&[TEST_SYMBOL, QUICKSAND]);
        assert_eq!(f.char_coverage("TestSymbol"), [' ', 'A', 'B', 'C']);
        let got = f.outline(&[TextSpan::new("AB", "TestSymbol", 100.0)]);
        let want = system_with(&[SELAWIK]).outline(&[TextSpan::new("AB", "Selawik", 100.0)]);
        assert_eq!(got.glyphs, 2);
        assert_eq!(got.path.elements().len(), want.path.elements().len());
        let (a, b) = (got.path.bounding_box(), want.path.bounding_box());
        assert!(
            (a.x0 - b.x0).abs() < 1e-6 && (a.x1 - b.x1).abs() < 1e-6,
            "{a:?} {b:?}"
        );
    }

    #[test]
    fn variable_fonts_draw_bold_with_their_weight_axis() {
        let f = system_with(&[QUICKSAND]);
        let set = |bold: bool, italic: bool| {
            let mut s = TextSpan::new("Hello", "Quicksand", 100.0);
            s.bold = bold;
            s.italic = italic;
            f.outline(&[s])
        };
        let em = 100.0 * PT_MM;
        let advance = |l: &TextLayout| l.lines[0].edges.last().copied().unwrap_or(0.0);
        // Regular at weight 400, not the font's default 300: "Hello" is
        // 2339 units of 1000 wide there with its "lo" kerning of -10
        // (2295 at 300, 2463 at 700).
        let regular = set(false, false);
        assert!(
            (advance(&regular) - 2.339 * em).abs() < 0.002 * em,
            "{}",
            advance(&regular) / em
        );
        let bold = set(true, false);
        assert!(
            (advance(&bold) - 2.463 * em).abs() < 0.002 * em,
            "{}",
            advance(&bold) / em
        );
        // The weight comes from the axis, not from drawing the letters twice.
        assert_eq!(bold.path.elements().len(), regular.path.elements().len());
        // No slant axis: italic is slanted synthetically.
        let italic = set(false, true);
        assert_eq!(italic.path.elements().len(), regular.path.elements().len());
        assert!(italic.path.bounding_box().x1 > regular.path.bounding_box().x1);
    }

    /// `data` with the OS/2 embedding bits set to `fs_type`.
    fn with_fs_type(data: &[u8], fs_type: u16) -> Vec<u8> {
        let mut out = data.to_vec();
        let n = u16::from_be_bytes([data[4], data[5]]) as usize;
        for i in 0..n {
            let rec = 12 + 16 * i;
            if &data[rec..rec + 4] == b"OS/2" {
                let off = u32::from_be_bytes([
                    data[rec + 8],
                    data[rec + 9],
                    data[rec + 10],
                    data[rec + 11],
                ]) as usize;
                out[off + 8..off + 10].copy_from_slice(&fs_type.to_be_bytes());
            }
        }
        out
    }

    #[test]
    fn missing_fonts_get_stand_ins_with_the_same_widths_or_style() {
        assert_eq!(substitutes("Arial")[0], "Liberation Sans");
        assert_eq!(substitutes("ArialMT")[0], "Liberation Sans");
        assert_eq!(substitutes("Helvetica")[0], "Liberation Sans");
        assert_eq!(substitutes("Times New Roman")[0], "Liberation Serif");
        assert_eq!(substitutes("TimesNewRomanPSMT")[0], "Liberation Serif");
        assert_eq!(substitutes("Courier New")[0], "Liberation Mono");
        assert_eq!(substitutes("Calibri")[0], "Carlito");
        assert_eq!(substitutes("Cambria")[0], "Caladea");
        assert_eq!(substitutes("Arial Narrow")[0], "Liberation Sans Narrow");
        assert_eq!(substitutes("Palatino Linotype")[0], "TeX Gyre Pagella");
        // Style words and foundry endings do not hide the family.
        assert_eq!(substitutes("Arial-BoldMT")[0], "Liberation Sans");
        assert_eq!(
            substitutes("Times New Roman Bold Italic")[0],
            "Liberation Serif"
        );
        assert_eq!(substitutes("Comic Sans MS")[0], "Comic Neue");
        assert_eq!(substitutes("Helvetica Neue LT Std")[0], "Liberation Sans");
        assert_eq!(substitutes("Menlo")[1], "Liberation Mono");
        assert_eq!(substitutes("Avenir Book")[0], "Liberation Sans");
        assert_eq!(substitutes("Minion Pro")[0], "Liberation Serif");
        // Calibri falls back to a sans-serif, Cambria to a serif.
        assert!(substitutes("Calibri").contains(&"Liberation Sans"));
        assert!(substitutes("Cambria").contains(&"Liberation Serif"));
        // By the name's style when nothing is drawn to its widths.
        assert_eq!(substitutes("Fancy Serif Display")[0], "Liberation Serif");
        assert_eq!(substitutes("Bodoni 72")[0], "Liberation Serif");
        assert_eq!(substitutes("Fira Code")[0], "Liberation Mono");
        assert_eq!(substitutes("Some Sans Serif")[0], "Liberation Sans");
        assert_eq!(substitutes("Century Gothic")[0], "TeX Gyre Adventor");
        assert_eq!(substitutes("Unknown Face")[0], "Liberation Sans");
        assert_eq!(substitutes("")[0], "Liberation Sans");
        // With Liberation installed, a missing Arial is drawn with it.
        let f = fonts();
        if f.has_family("Liberation Sans") && !f.has_family("Arial") {
            assert_eq!(f.substitute("Arial").as_deref(), Some("Liberation Sans"));
            let id = f.face_for("Arial", false, false).expect("face");
            let db = f.db();
            let face = db.face(id).expect("info");
            assert!(face.families.iter().any(|(n, _)| n == "Liberation Sans"));
        }
        if let Some(installed) = f.families().first() {
            assert_eq!(f.substitute(installed), None);
        }
    }

    #[test]
    fn a_font_licence_decides_whether_a_document_may_use_it() {
        // Editable (the bundled font), installable, preview and print.
        assert!(usable_in_documents(SELAWIK));
        assert!(usable_in_documents(&with_fs_type(SELAWIK, 0)));
        assert!(usable_in_documents(&with_fs_type(SELAWIK, 4)));
        // Restricted, or bitmaps only: not used.
        assert!(!usable_in_documents(&with_fs_type(SELAWIK, 2)));
        assert!(!usable_in_documents(&with_fs_type(SELAWIK, 0x0208)));
        assert!(
            !usable_in_documents(&with_fs_type(SELAWIK, 3)),
            "restricted with a reserved bit"
        );
        // A reserved bit alone (old fonts set bit 0): not a restriction
        // (#313). Several bits: the most permissive wins.
        assert!(usable_in_documents(&with_fs_type(SELAWIK, 1)));
        assert!(usable_in_documents(&with_fs_type(SELAWIK, 0x000a)));
        assert!(usable_in_documents(&with_fs_type(SELAWIK, 0x0006)));
        // Not a font, or a cut one.
        assert!(!usable_in_documents(b"not a font at all"));
        assert!(!usable_in_documents(&SELAWIK[..40]));
        assert!(!usable_in_documents(&[]));
    }

    #[test]
    fn fonts_a_document_brings_are_loaded_once_unless_installed() {
        let f = fonts();
        let installed = f.has_family("Selawik");
        let font = DocumentFont::new("Selawik", SELAWIK.to_vec());
        let added = f.add_document_fonts(std::slice::from_ref(&font));
        // Added unless the machine has the font; usable either way.
        assert_eq!(added.is_empty(), installed, "{added:?}");
        assert!(f.has_family("Selawik"));
        assert!(f.families().iter().any(|n| n == "Selawik"));
        assert_eq!(f.is_document_family("Selawik"), !installed);
        // A second document with the same font adds nothing.
        assert!(f.add_document_fonts(std::slice::from_ref(&font)).is_empty());
        // Text in that family is laid out with it.
        let id = f.face_for("Selawik", false, false).expect("a face");
        let db = f.db();
        let face = db.face(id).expect("face info");
        assert!(face.families.iter().any(|(n, _)| n == "Selawik"));
        drop(db);
        let mut s = span("Hello");
        s.font_family = "Selawik".into();
        assert!(f.outline(&[s]).glyphs > 0);
        // A drawing using the family without carrying the font gets it
        // when saved (unless the machine has it installed).
        let mut doc = tracedraw_core::Document::default();
        doc.pages[0].layers[0]
            .shapes
            .push(tracedraw_core::Shape::new(
                tracedraw_core::ShapeId(1),
                tracedraw_core::ShapeKind::Text {
                    spans: vec![TextSpan::new("Hi", "Selawik", 12.0)],
                    origin: Point::ZERO,
                    frame: None,
                    align: TextAlign::Left,
                    para: ParagraphStyle::default(),
                    on_path: None,
                },
            ));
        let saved = f.with_carried_fonts(&doc);
        assert_eq!(saved.fonts.len(), usize::from(!installed));
        if !installed {
            assert!(Arc::ptr_eq(&saved.fonts[0].data, &font.data));
            // Once carried, nothing more is added.
            let again = f.with_carried_fonts(&saved);
            assert!(matches!(again, std::borrow::Cow::Borrowed(_)));
        }
        // A restricted or damaged font is refused.
        let restricted = DocumentFont::new("Selawik", with_fs_type(SELAWIK, 2));
        let junk = DocumentFont::new("Junk", vec![1, 2, 3]);
        assert!(f.add_document_fonts(&[restricted, junk]).is_empty());
        assert!(!f.has_family("Junk"));
    }

    /// Save As's Embed fonts: the installed faces the text uses go with
    /// the drawing when their licence allows it, each file once, the
    /// regular face standing in for a style the family lacks; a family
    /// that came with a document or a file already carried is not added
    /// again (#275).
    #[test]
    fn installed_fonts_the_text_uses_are_embedded_when_their_licence_allows() {
        const SELAWIK_BOLD: &[u8] =
            include_bytes!("../../../apps/tracedraw/assets/fonts/Selawik-Bold.ttf");
        let restricted = with_fs_type(QUICKSAND, 2);
        let f = system_with(&[SELAWIK, SELAWIK_BOLD, &restricted]);
        let text = |family: &str, bold: bool, italic: bool| {
            let mut sp = TextSpan::new("Hi", family, 12.0);
            sp.bold = bold;
            sp.italic = italic;
            tracedraw_core::Shape::new(
                tracedraw_core::ShapeId(1),
                tracedraw_core::ShapeKind::Text {
                    spans: vec![sp],
                    origin: Point::ZERO,
                    frame: None,
                    align: TextAlign::Left,
                    para: ParagraphStyle::default(),
                    on_path: None,
                },
            )
        };
        let mut doc = tracedraw_core::Document::default();
        let shapes = &mut doc.pages[0].layers[0].shapes;
        shapes.push(text("Selawik", false, false));
        shapes.push(text("Selawik", true, false));
        // Italic: the family has none, so its regular face goes (once).
        shapes.push(text("Selawik", false, true));
        // A restricted licence: left out. A family nobody has: nothing.
        shapes.push(text("Quicksand", false, false));
        shapes.push(text("Nowhere Sans", false, false));
        let add = f.fonts_to_embed(&doc);
        let mut families: Vec<&str> = add.iter().map(|d| d.family.as_str()).collect();
        families.sort();
        assert_eq!(families, ["Selawik", "Selawik"]);
        let mut sizes: Vec<usize> = add.iter().map(|d| d.data.len()).collect();
        sizes.sort();
        let mut want = vec![SELAWIK.len(), SELAWIK_BOLD.len()];
        want.sort();
        assert_eq!(sizes, want);
        let saved = f.with_embedded_fonts(&doc);
        assert_eq!(saved.fonts.len(), 2);
        // Saved again, the carried files are not added twice.
        assert!(matches!(
            f.with_embedded_fonts(&saved),
            std::borrow::Cow::Borrowed(_)
        ));
        // A family that came with a document is the carried fonts' business.
        let g = system_with(&[]);
        g.add_document_fonts(&[DocumentFont::new("Selawik", SELAWIK.to_vec())]);
        assert!(g.fonts_to_embed(&doc).is_empty());
        // Nothing to embed: the drawing is left as it is.
        assert!(matches!(
            g.with_embedded_fonts(&doc),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    /// Vertical text (#257): the written line becomes a column going
    /// down from the origin, Latin letters turned a quarter clockwise
    /// (their tops to the right of the column's axis), an upright letter
    /// centred on the axis with its em top at the pen; the reported
    /// lines stay as written, and `line_frame` turns them.
    #[test]
    fn vertical_text_sets_columns_with_turned_and_upright_letters() {
        let mut para = ParagraphStyle::default();
        para.vertical = true;
        fn req<'a>(spans: &'a [TextSpan], para: &'a ParagraphStyle) -> TextRequest<'a> {
            TextRequest {
                spans,
                frame: None,
                align: TextAlign::Left,
                para,
                on_path: None,
            }
        }
        let f = system_with(&[SELAWIK]);
        // Four letters: a line longer than it is high.
        let mut ab = TextSpan::new("abcd", "Selawik", 72.0);
        ab.size_pt = 72.0;
        let flat = f.layout(&req(std::slice::from_ref(&ab), &ParagraphStyle::default()));
        let tall = f.layout(&req(std::slice::from_ref(&ab), &para));
        assert!(tall.vertical && !flat.vertical);
        assert_eq!(tall.glyphs, 4);
        // The same written line: edges from the origin to the right.
        assert_eq!(tall.lines.len(), 1);
        assert_eq!(tall.lines[0].edges, flat.lines[0].edges);
        assert_eq!(
            tall.line_frame(),
            tracedraw_core::document::vertical_line_frame()
        );
        // Turned: the glyphs run down from the top, as deep as the line
        // was long, and sit to the right of the axis (x = 0) as high as
        // the letters stood.
        let (fb, tb) = (flat.bounds, tall.bounds);
        assert!(tb.height() > tb.width(), "{tb:?}");
        assert!(
            (tb.y0 - -fb.x1).abs() < 1e-6 && tb.y1.abs() < 1e-6,
            "{tb:?} vs {fb:?}"
        );
        assert!(
            (tb.x1 - fb.y1).abs() < 1e-6 && (tb.x0 - fb.y0).abs() < 1e-6,
            "{tb:?} vs {fb:?}"
        );
        let pb = tall.path.bounding_box();
        assert!(pb.y1 < 0.5 && pb.y0 > -fb.x1 - 0.5, "{pb:?}");
        assert!(pb.x0 > -0.5 && pb.x1 < fb.y1 + 0.5, "{pb:?}");
        // The turn itself: a point of the written line maps as a quarter
        // turn clockwise.
        let turn = tall.line_frame();
        let p = turn * Point::new(10.0, 2.0);
        assert!(
            (p.x - 2.0).abs() < 1e-9 && (p.y - -10.0).abs() < 1e-9,
            "{p:?}"
        );

        // An upright letter (a circle the system fonts have): centred on
        // the axis, within the em below the pen.
        let f = fonts();
        let mut circle = TextSpan::new("\u{25CB}", "Selawik", 72.0);
        circle.size_pt = 72.0;
        let l = f.layout(&req(std::slice::from_ref(&circle), &para));
        if l.glyphs == 1 && !l.path.elements().is_empty() {
            // (The circle may come from another font, with its own
            // ascent: it hangs from the pen by that ascent.)
            let b = l.path.bounding_box();
            let em = 72.0 * PT_MM;
            assert!(b.center().x.abs() < em * 0.2, "{b:?}");
            assert!(b.y1 <= 0.5 && b.y0 >= -1.4 * em, "{b:?}");
        }
        // Text on a path is never vertical.
        let on_path = TextOnPath {
            path: {
                let mut p = BezPath::new();
                p.move_to((0.0, 0.0));
                p.line_to((100.0, 0.0));
                p
            },
            ..Default::default()
        };
        let l = f.layout(&TextRequest {
            spans: std::slice::from_ref(&ab),
            frame: None,
            align: TextAlign::Left,
            para: &para,
            on_path: Some(&on_path),
        });
        assert!(!l.vertical);
    }

    #[test]
    fn letters_that_stand_upright_in_columns() {
        for c in ['日', 'あ', 'ア', '한', '。', '「', 'ａ', '○', '①', '😀'] {
            assert!(upright(c), "{c}");
        }
        for c in ['A', 'z', '1', '.', ' ', 'é', 'Ω', 'я', 'ب'] {
            assert!(!upright(c), "{c}");
        }
    }

    #[test]
    fn paragraph_text_splits_into_one_artistic_text_per_line() {
        if !has_fonts() {
            return;
        }
        install();
        let kind = tracedraw_core::ShapeKind::Text {
            spans: vec![span("one two three four five six seven eight nine ten")],
            origin: Point::ZERO,
            frame: Some(Size::new(20.0, 200.0)),
            align: TextAlign::Center,
            para: ParagraphStyle::default(),
            on_path: None,
        };
        let lines = tracedraw_core::document::paragraph_lines(&kind).expect("lines");
        assert!(lines.len() > 1);
        let mut joined = String::new();
        let mut last_y = f64::INFINITY;
        for line in &lines {
            let tracedraw_core::ShapeKind::Text {
                spans,
                origin,
                frame,
                align,
                ..
            } = line
            else {
                panic!("not text");
            };
            assert!(frame.is_none());
            assert!(matches!(align, TextAlign::Left));
            // Lines run downwards from the top of the frame.
            assert!(origin.y < last_y && origin.y <= 200.0);
            last_y = origin.y;
            for sp in spans {
                joined.push_str(&sp.text);
            }
        }
        let words: Vec<&str> = joined.split_whitespace().collect();
        assert_eq!(
            words.join(" "),
            "one two three four five six seven eight nine ten"
        );
        // Artistic text has no lines to split.
        let artistic = tracedraw_core::ShapeKind::Text {
            spans: vec![span("x")],
            origin: Point::ZERO,
            frame: None,
            align: TextAlign::Left,
            para: ParagraphStyle::default(),
            on_path: None,
        };
        assert!(tracedraw_core::document::paragraph_lines(&artistic).is_none());
    }

    #[test]
    fn symbol_encoded_text_reads_as_the_characters_it_draws() {
        assert!(symbol_encoded("Symbol") && symbol_encoded("SymbolMT"));
        assert!(!symbol_encoded("Times New Roman"));
        assert_eq!(symbol_to_unicode('\u{c4}'), '\u{2297}');
        assert_eq!(symbol_to_unicode('\u{f0ac}'), '\u{2190}');
        assert_eq!(symbol_to_unicode('a'), '\u{3b1}');
        assert_eq!(symbol_to_unicode('1'), '1');
        // No character there, or outside the encoding: unchanged.
        assert_eq!(symbol_to_unicode('\u{80}'), '\u{80}');
        assert_eq!(symbol_to_unicode('\u{2295}'), '\u{2295}');
        // Without a symbol font installed the codes draw as those signs.
        let f = fonts();
        if f.has_family("Symbol") || !has_fonts() {
            return;
        }
        let mut code = span("\u{c4}");
        code.font_family = "Symbol".into();
        let mut sign = span("\u{2297}");
        sign.font_family = "Symbol".into();
        let a = f.outline(&[code]);
        let b = f.outline(&[sign]);
        assert_eq!(a.path.elements().len(), b.path.elements().len());
    }

    #[test]
    fn a_big_first_letter_opens_up_only_its_own_line() {
        if !has_fonts() {
            return;
        }
        let mut cap = span("L");
        cap.size_pt = 48.0;
        let body = span(&"word ".repeat(60));
        let para = ParagraphStyle::default();
        let layout = fonts().layout(&TextRequest {
            spans: &[cap, body.clone()],
            frame: Some(Size::new(40.0, 400.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        let plain = fonts().layout(&TextRequest {
            spans: &[body],
            frame: Some(Size::new(40.0, 400.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        let gap = |l: &TextLayout, i: usize| l.lines[i].baseline - l.lines[i + 1].baseline;
        assert!(layout.lines.len() > 3);
        // The first line drops by the big letter's height; the others keep
        // the spacing of the body text.
        assert!(layout.lines[0].baseline < plain.lines[0].baseline * 2.0);
        assert!((gap(&layout, 1) - gap(&plain, 1)).abs() < 1e-6);
        assert!((gap(&layout, 2) - gap(&plain, 2)).abs() < 1e-6);
    }
}
