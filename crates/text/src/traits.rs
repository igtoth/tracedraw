//! What a font list filters families by (#867): the kind of letters a
//! family has (serif, sans serif, script, decorative, monospaced, symbol),
//! how its outlines are stored, its weights and widths, its embedding
//! rights, the scripts it covers, its OpenType layout features and the
//! folders its files are in. Read from tables every face carries, as the
//! OpenType specification defines them: the OS/2 table's `sFamilyClass`
//! (class ID in the high byte), PANOSE digits, `usWeightClass`,
//! `usWidthClass`, `fsType` and Unicode ranges, the `post` table's
//! `isFixedPitch`, `fvar` and its axes, `cmap`, and the feature lists of
//! GSUB and GPOS. See docs/behavior/text.md, "Font list".

use crate::catalog::{Embedding, Script};
use crate::{FamilyInfo, FontKind};

/// The kind of letters a family has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontClass {
    /// The font does not say (and its name does not either).
    #[default]
    Unknown,
    Serif,
    SansSerif,
    /// Handwritten and script letters.
    Script,
    /// Ornamental and display letters.
    Decorative,
    /// Pictures and signs rather than letters.
    Symbol,
}

/// What one face says about itself, read from its tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FaceTraits {
    pub class: FontClass,
    pub monospace: bool,
    /// Has OpenType layout features beyond kerning (ligatures, small
    /// capitals, figure styles, stylistic sets ...).
    pub features: bool,
}

/// The class an OS/2 table gives: its `sFamilyClass` class ID when it
/// names one, else the PANOSE family kind and, for text faces, the serif
/// style digit. `os2` is the whole table (short tables read as unknown).
pub fn os2_class(os2: &[u8]) -> FontClass {
    match os2.get(30) {
        Some(1..=5 | 7) => return FontClass::Serif,
        Some(8) => return FontClass::SansSerif,
        Some(9) => return FontClass::Decorative,
        Some(10) => return FontClass::Script,
        Some(12) => return FontClass::Symbol,
        _ => {}
    }
    // PANOSE starts at byte 32: family kind, then the serif style.
    match (os2.get(32), os2.get(33)) {
        (Some(2), Some(2..=10)) => FontClass::Serif,
        // Normal, obtuse and perpendicular sans, flared and rounded.
        (Some(2), Some(11..=15)) => FontClass::SansSerif,
        (Some(3), _) => FontClass::Script,
        (Some(4), _) => FontClass::Decorative,
        (Some(5), _) => FontClass::Symbol,
        _ => FontClass::Unknown,
    }
}

/// The PANOSE digits say the letters all have one width: proportion 9
/// for a text face, spacing 3 for a handwritten one.
pub fn os2_monospace(os2: &[u8]) -> bool {
    matches!(
        (os2.get(32), os2.get(35)),
        (Some(2), Some(9)) | (Some(3), Some(3))
    )
}

/// The class a family's name gives when its tables say nothing: a word
/// "Sans", "Serif", "Script" or "Symbol" ("Noto Sans", "Liberation
/// Serif"); "Sans" wins over "Serif" ("PT Sans Serif" reads as sans).
pub fn name_class(family: &str) -> FontClass {
    let words: Vec<String> = family
        .split([' ', '-', '_'])
        .map(str::to_ascii_lowercase)
        .collect();
    let has = |w: &str| words.iter().any(|x| x == w);
    if has("sans") {
        FontClass::SansSerif
    } else if has("serif") {
        FontClass::Serif
    } else if has("script") || has("handwriting") {
        FontClass::Script
    } else if has("symbol") || has("symbols") || has("dingbats") {
        FontClass::Symbol
    } else {
        FontClass::Unknown
    }
}

/// A family name that says its letters are all one width ("DejaVu Sans
/// Mono", "Source Code Pro").
pub fn name_monospace(family: &str) -> bool {
    family.split([' ', '-', '_']).any(|w| {
        let w = w.to_ascii_lowercase();
        w == "mono" || w == "monospace" || w == "code"
    })
}

/// The OpenType layout features that count as "has features": what the
/// Text docker's OpenType section offers, not kerning or the features
/// every script needs.
const LAYOUT_FEATURES: [&[u8; 4]; 16] = [
    b"liga", b"dlig", b"hlig", b"smcp", b"c2sc", b"onum", b"lnum", b"pnum", b"tnum", b"frac",
    b"swsh", b"salt", b"calt", b"ss01", b"ordn", b"zero",
];

/// What face `f` says about itself.
pub fn face_traits(f: &ttf_parser::Face) -> FaceTraits {
    let raw = f.raw_face();
    let os2 = raw
        .table(ttf_parser::Tag::from_bytes(b"OS/2"))
        .unwrap_or(&[]);
    let mut features = false;
    for t in [b"GSUB", b"GPOS"] {
        if let Some(table) = raw.table(ttf_parser::Tag::from_bytes(t)) {
            features |= crate::catalog::feature_tags(table)
                .iter()
                .any(|tag| LAYOUT_FEATURES.contains(&tag));
        }
    }
    FaceTraits {
        class: os2_class(os2),
        monospace: f.is_monospaced() || os2_monospace(os2),
        features,
    }
}

/// A kind of letters a font list's filter asks for (its Style section):
/// of several checked, a family needs one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontTrait {
    Serif,
    SansSerif,
    Script,
    Decorative,
    Monospace,
    Symbol,
}

impl FontTrait {
    pub const ALL: [FontTrait; 6] = [
        FontTrait::Serif,
        FontTrait::SansSerif,
        FontTrait::Script,
        FontTrait::Decorative,
        FontTrait::Monospace,
        FontTrait::Symbol,
    ];

    fn bit(self) -> u16 {
        1 << self as u16
    }

    /// `info` has this trait.
    pub fn holds(self, info: &FamilyInfo) -> bool {
        match self {
            FontTrait::Serif => info.class == FontClass::Serif,
            FontTrait::SansSerif => info.class == FontClass::SansSerif,
            FontTrait::Script => info.class == FontClass::Script,
            FontTrait::Decorative => info.class == FontClass::Decorative,
            FontTrait::Monospace => info.monospace,
            FontTrait::Symbol => info.symbol || info.class == FontClass::Symbol,
        }
    }
}

/// How a face's outlines are stored: TrueType (`glyf`), OpenType with
/// PostScript outlines (`CFF `, `CFF2`), or a variable font (`fvar`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Technology {
    TrueType,
    OpenType,
    Variable,
}

impl Technology {
    pub const ALL: [Technology; 3] = [
        Technology::TrueType,
        Technology::OpenType,
        Technology::Variable,
    ];

    pub fn bit(self) -> u8 {
        1 << self as u8
    }

    /// The technology of a face of `kind`.
    pub fn of(kind: FontKind) -> Option<Technology> {
        match kind {
            FontKind::TrueType => Some(Technology::TrueType),
            FontKind::OpenType => Some(Technology::OpenType),
            FontKind::Variable => Some(Technology::Variable),
            FontKind::Offered => None,
        }
    }
}

/// Weights as a font list groups them, from the OS/2 `usWeightClass`
/// (or a variable font's `wght` axis): light below 350, bold from 600.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeightClass {
    Light,
    Normal,
    Bold,
}

impl WeightClass {
    pub const ALL: [WeightClass; 3] = [WeightClass::Light, WeightClass::Normal, WeightClass::Bold];

    pub fn bit(self) -> u8 {
        1 << self as u8
    }

    /// The groups the weights `min` to `max` reach (one weight: one group).
    pub fn bits_of(min: f32, max: f32) -> u8 {
        let (lo, hi) = (min.min(max), min.max(max));
        let mut bits = 0;
        if lo < 350.0 {
            bits |= WeightClass::Light.bit();
        }
        if lo < 600.0 && hi >= 350.0 {
            bits |= WeightClass::Normal.bit();
        }
        if hi >= 600.0 {
            bits |= WeightClass::Bold.bit();
        }
        bits
    }
}

/// Widths as a font list groups them, from the OS/2 `usWidthClass` (1 to
/// 4 condensed, 5 normal, 6 to 9 expanded) or a variable font's `wdth`
/// axis (percent of normal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidthClass {
    Condensed,
    Normal,
    Expanded,
}

impl WidthClass {
    pub const ALL: [WidthClass; 3] = [
        WidthClass::Condensed,
        WidthClass::Normal,
        WidthClass::Expanded,
    ];

    pub fn bit(self) -> u8 {
        1 << self as u8
    }

    /// The group of an OS/2 width class (out of range reads as normal).
    pub fn of_class(class: u16) -> WidthClass {
        match class {
            1..=4 => WidthClass::Condensed,
            6..=9 => WidthClass::Expanded,
            _ => WidthClass::Normal,
        }
    }

    /// The groups a `wdth` axis from `min` to `max` percent reaches.
    pub fn bits_of_axis(min: f32, max: f32) -> u8 {
        let (lo, hi) = (min.min(max), min.max(max));
        let mut bits = 0;
        if lo < 100.0 {
            bits |= WidthClass::Condensed.bit();
        }
        if lo <= 100.0 && hi >= 100.0 {
            bits |= WidthClass::Normal.bit();
        }
        if hi > 100.0 {
            bits |= WidthClass::Expanded.bit();
        }
        bits
    }
}

/// The bit of embedding rights `e` in [`FontFilter::embedding`].
pub fn embedding_bit(e: Embedding) -> u8 {
    1 << Embedding::ALL.iter().position(|x| *x == e).unwrap_or(0)
}

/// The bit of script `s` in [`FamilyInfo::scripts`] and
/// [`FontFilter::scripts`].
pub fn script_bit(s: Script) -> u16 {
    1 << Script::ALL.iter().position(|x| *x == s).unwrap_or(0)
}

/// The OpenType features a font list's filter offers, in its order.
/// `ss01` stands for every stylistic set (`ss01` to `ss20`).
pub const FILTER_FEATURES: [&[u8; 4]; 18] = [
    b"liga", b"dlig", b"kern", b"smcp", b"c2sc", b"onum", b"lnum", b"pnum", b"tnum", b"frac",
    b"sups", b"subs", b"ordn", b"zero", b"swsh", b"salt", b"calt", b"ss01",
];

/// A family whose faces have the feature tags `tags` has feature `want`
/// (`ss01`: any stylistic set).
pub fn has_feature(tags: &[[u8; 4]], want: &[u8; 4]) -> bool {
    if want == b"ss01" {
        return tags.iter().any(|t| {
            t[0] == b's' && t[1] == b's' && t[2].is_ascii_digit() && t[3].is_ascii_digit()
        });
    }
    tags.iter().any(|t| t == want)
}

/// What a font list's filter asks of a family (#867). Each section is a
/// set of choices: of several checked in one section a family needs one
/// (Style, Technology, Weight, Width, Embedding rights, Scripts,
/// Folders), except OpenType features, of which it needs every one
/// checked; it needs what every section with a choice asks. Nothing
/// checked shows every family.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FontFilter {
    /// [`FontTrait`] bits (see [`FontFilter::has`]).
    pub styles: u16,
    /// [`Technology::bit`]s.
    pub tech: u8,
    /// [`WeightClass::bit`]s.
    pub weights: u8,
    /// [`WidthClass::bit`]s.
    pub widths: u8,
    /// [`embedding_bit`]s: the family's most restrictive face counts.
    pub embedding: u8,
    /// [`script_bit`]s.
    pub scripts: u16,
    /// Feature tags the family must have, each.
    pub features: Vec<[u8; 4]>,
    /// Folders a face of the family must be in, one of them.
    pub folders: Vec<std::path::PathBuf>,
}

impl FontFilter {
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn has(&self, t: FontTrait) -> bool {
        self.styles & t.bit() != 0
    }

    pub fn set(&mut self, t: FontTrait, on: bool) {
        if on {
            self.styles |= t.bit();
        } else {
            self.styles &= !t.bit();
        }
    }

    /// Check or uncheck feature `tag`.
    pub fn set_feature(&mut self, tag: [u8; 4], on: bool) {
        self.features.retain(|t| *t != tag);
        if on {
            self.features.push(tag);
        }
    }

    /// Check or uncheck folder `dir`.
    pub fn set_folder(&mut self, dir: &std::path::Path, on: bool) {
        self.folders.retain(|d| d != dir);
        if on {
            self.folders.push(dir.to_path_buf());
        }
    }

    /// The number of choices checked.
    pub fn count(&self) -> u32 {
        self.styles.count_ones()
            + self.tech.count_ones()
            + self.weights.count_ones()
            + self.widths.count_ones()
            + self.embedding.count_ones()
            + self.scripts.count_ones()
            + self.features.len() as u32
            + self.folders.len() as u32
    }

    /// A family with `info` shows (`None`: not installed nor offered,
    /// which shows only when nothing is checked; an offered family is not
    /// read until fetched, so it has nothing to show by).
    pub fn matches(&self, info: Option<&FamilyInfo>) -> bool {
        if self.is_empty() {
            return true;
        }
        let Some(info) = info else {
            return false;
        };
        if info.kind == FontKind::Offered {
            return false;
        }
        let any = |checked: u32, held: u32| checked == 0 || checked & held != 0;
        let styles_held = FontTrait::ALL
            .iter()
            .filter(|t| t.holds(info))
            .fold(0u16, |b, t| b | t.bit());
        let embedding_held = info.embedding.map_or(0, embedding_bit);
        any(self.styles.into(), styles_held.into())
            && any(self.tech.into(), info.kinds.into())
            && any(self.weights.into(), info.weights.into())
            && any(self.widths.into(), info.widths.into())
            && any(self.embedding.into(), embedding_held.into())
            && any(self.scripts.into(), info.scripts.into())
            && self
                .features
                .iter()
                .all(|f| has_feature(&info.feature_tags, f))
            && (self.folders.is_empty() || info.folders.iter().any(|d| self.folders.contains(d)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os2(class: u8, panose: [u8; 4]) -> Vec<u8> {
        let mut t = vec![0u8; 96];
        t[30] = class;
        t[32..36].copy_from_slice(&panose);
        t
    }

    fn info(class: FontClass, kind: FontKind) -> FamilyInfo {
        FamilyInfo {
            kind,
            kinds: Technology::of(kind).map_or(0, Technology::bit),
            class,
            weights: WeightClass::Normal.bit(),
            widths: WidthClass::Normal.bit(),
            ..Default::default()
        }
    }

    #[test]
    fn the_family_class_comes_first_then_panose() {
        assert_eq!(os2_class(&os2(1, [0; 4])), FontClass::Serif);
        assert_eq!(os2_class(&os2(5, [2, 11, 0, 0])), FontClass::Serif);
        assert_eq!(os2_class(&os2(8, [0; 4])), FontClass::SansSerif);
        assert_eq!(os2_class(&os2(9, [0; 4])), FontClass::Decorative);
        assert_eq!(os2_class(&os2(10, [0; 4])), FontClass::Script);
        assert_eq!(os2_class(&os2(12, [0; 4])), FontClass::Symbol);
        // Reserved and "no classification" class IDs fall to PANOSE.
        assert_eq!(os2_class(&os2(0, [2, 11, 5, 2])), FontClass::SansSerif);
        assert_eq!(os2_class(&os2(6, [2, 2, 6, 3])), FontClass::Serif);
        assert_eq!(os2_class(&os2(0, [3, 0, 0, 0])), FontClass::Script);
        assert_eq!(os2_class(&os2(0, [4, 0, 0, 0])), FontClass::Decorative);
        assert_eq!(os2_class(&os2(0, [5, 0, 0, 0])), FontClass::Symbol);
        assert_eq!(os2_class(&os2(0, [0; 4])), FontClass::Unknown);
        // Short or missing tables never panic.
        assert_eq!(os2_class(&[]), FontClass::Unknown);
        assert_eq!(os2_class(&[0; 31]), FontClass::Unknown);
        assert!(!os2_monospace(&[0; 33]));
    }

    #[test]
    fn panose_says_monospaced() {
        assert!(os2_monospace(&os2(0, [2, 11, 6, 9])));
        assert!(os2_monospace(&os2(0, [3, 1, 1, 3])));
        assert!(!os2_monospace(&os2(0, [2, 11, 6, 3])));
    }

    #[test]
    fn names_classify_unclassified_families() {
        assert_eq!(name_class("Noto Sans"), FontClass::SansSerif);
        assert_eq!(name_class("PT Sans Serif"), FontClass::SansSerif);
        assert_eq!(name_class("Liberation Serif"), FontClass::Serif);
        assert_eq!(name_class("Dancing-Script"), FontClass::Script);
        assert_eq!(name_class("Noto Sans Symbols"), FontClass::SansSerif);
        assert_eq!(name_class("Wingdings"), FontClass::Unknown);
        assert_eq!(name_class("Sansation"), FontClass::Unknown);
        assert!(name_monospace("DejaVu Sans Mono"));
        assert!(name_monospace("Source Code Pro"));
        assert!(!name_monospace("Monoton"));
    }

    #[test]
    fn nothing_checked_shows_every_family() {
        let f = FontFilter::default();
        assert!(f.matches(None));
        assert!(f.matches(Some(&info(FontClass::Unknown, FontKind::TrueType))));
    }

    #[test]
    fn choices_in_a_section_widen_sections_narrow() {
        let serif = info(FontClass::Serif, FontKind::TrueType);
        let sans = info(FontClass::SansSerif, FontKind::Variable);
        let mut mono_sans = info(FontClass::SansSerif, FontKind::OpenType);
        mono_sans.monospace = true;
        let unknown = info(FontClass::Unknown, FontKind::TrueType);

        let mut f = FontFilter::default();
        f.set(FontTrait::Serif, true);
        assert!(f.matches(Some(&serif)));
        assert!(!f.matches(Some(&sans)));
        assert!(!f.matches(Some(&unknown)));
        assert!(!f.matches(None));
        // Serif or sans serif.
        f.set(FontTrait::SansSerif, true);
        assert_eq!(f.count(), 2);
        assert!(f.matches(Some(&serif)) && f.matches(Some(&sans)));
        // ... and variable.
        f.tech = Technology::Variable.bit();
        assert!(!f.matches(Some(&serif)));
        assert!(f.matches(Some(&sans)));
        assert!(!f.matches(Some(&mono_sans)));
        // Variable or OpenType.
        f.tech |= Technology::OpenType.bit();
        assert!(f.matches(Some(&mono_sans)));
        assert_eq!(f.count(), 4);
        // Monospaced alone.
        let mut m = FontFilter::default();
        m.set(FontTrait::Monospace, true);
        assert!(m.matches(Some(&mono_sans)) && !m.matches(Some(&sans)));
        m.set(FontTrait::Monospace, false);
        assert!(m.is_empty());
    }

    #[test]
    fn weights_widths_rights_scripts_features_and_folders() {
        let mut fam = info(FontClass::SansSerif, FontKind::Variable);
        fam.weights = WeightClass::bits_of(100.0, 900.0);
        fam.widths = WidthClass::bits_of_axis(75.0, 100.0);
        fam.embedding = Some(Embedding::PreviewPrint);
        fam.scripts = script_bit(Script::Latin) | script_bit(Script::Cyrillic);
        fam.feature_tags = vec![*b"liga", *b"smcp", *b"ss03"];
        fam.folders = vec!["/fonts/a".into()];
        let plain = info(FontClass::SansSerif, FontKind::TrueType);
        let mut f = FontFilter::default();
        f.weights = WeightClass::Bold.bit();
        assert!(f.matches(Some(&fam)) && !f.matches(Some(&plain)));
        f = FontFilter {
            widths: WidthClass::Condensed.bit(),
            ..Default::default()
        };
        assert!(f.matches(Some(&fam)) && !f.matches(Some(&plain)));
        assert_eq!(
            WidthClass::bits_of_axis(100.0, 100.0),
            WidthClass::Normal.bit()
        );
        assert_eq!(WidthClass::of_class(3), WidthClass::Condensed);
        assert_eq!(WidthClass::of_class(0), WidthClass::Normal);
        assert_eq!(
            WeightClass::bits_of(400.0, 400.0),
            WeightClass::Normal.bit()
        );
        assert_eq!(WeightClass::bits_of(300.0, 300.0), WeightClass::Light.bit());
        f = FontFilter {
            embedding: embedding_bit(Embedding::PreviewPrint) | embedding_bit(Embedding::Editable),
            ..Default::default()
        };
        assert!(f.matches(Some(&fam)) && !f.matches(Some(&plain)));
        f = FontFilter {
            scripts: script_bit(Script::Cyrillic),
            ..Default::default()
        };
        assert!(f.matches(Some(&fam)) && !f.matches(Some(&plain)));
        // Features: every one checked.
        let mut f = FontFilter::default();
        f.set_feature(*b"smcp", true);
        f.set_feature(*b"ss01", true);
        assert!(f.matches(Some(&fam)), "ss03 is a stylistic set");
        f.set_feature(*b"onum", true);
        assert!(!f.matches(Some(&fam)));
        f.set_feature(*b"onum", false);
        assert_eq!(f.count(), 2);
        // Folders: one of them.
        let mut f = FontFilter::default();
        f.set_folder(std::path::Path::new("/fonts/b"), true);
        assert!(!f.matches(Some(&fam)));
        f.set_folder(std::path::Path::new("/fonts/a"), true);
        assert!(f.matches(Some(&fam)) && !f.matches(Some(&plain)));
        assert!(has_feature(&[*b"ss12"], b"ss01") && !has_feature(&[*b"salt"], b"ss01"));
    }

    #[test]
    fn symbol_fonts_by_their_character_map_or_class() {
        let mut f = FontFilter::default();
        f.set(FontTrait::Symbol, true);
        let mut by_cmap = info(FontClass::Unknown, FontKind::TrueType);
        by_cmap.symbol = true;
        assert!(f.matches(Some(&by_cmap)));
        assert!(f.matches(Some(&info(FontClass::Symbol, FontKind::TrueType))));
        assert!(!f.matches(Some(&info(FontClass::Serif, FontKind::TrueType))));
        // Offered families are not read yet: no trait.
        assert!(!f.matches(Some(&info(FontClass::Unknown, FontKind::Offered))));
    }
}
