//! What a font list filters families by (#867): the kind of letters a
//! family has (serif, sans serif, script, decorative), whether it is
//! monospaced, a symbol font, a variable font or has OpenType layout
//! features. Read from tables every face carries, as the OpenType
//! specification defines them: the OS/2 table's `sFamilyClass` (class ID
//! in the high byte) and PANOSE digits, the `post` table's `isFixedPitch`,
//! `fvar`, and the feature lists of GSUB and GPOS. See
//! docs/behavior/text.md, "Font list".

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

/// One thing a font list can ask of a family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontTrait {
    Serif,
    SansSerif,
    Script,
    Decorative,
    Monospace,
    Symbol,
    Variable,
    Features,
}

impl FontTrait {
    pub const ALL: [FontTrait; 8] = [
        FontTrait::Serif,
        FontTrait::SansSerif,
        FontTrait::Script,
        FontTrait::Decorative,
        FontTrait::Monospace,
        FontTrait::Symbol,
        FontTrait::Variable,
        FontTrait::Features,
    ];

    fn bit(self) -> u16 {
        1 << self as u16
    }

    /// A kind of letters: of several checked, a family needs one.
    pub fn is_class(self) -> bool {
        matches!(
            self,
            FontTrait::Serif | FontTrait::SansSerif | FontTrait::Script | FontTrait::Decorative
        )
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
            FontTrait::Variable => info.kind == FontKind::Variable,
            FontTrait::Features => info.features,
        }
    }
}

/// The traits checked in a font list's filter. A family shows when it has
/// one of the checked kinds of letters (if any is checked) and every other
/// checked trait. Nothing checked shows every family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FontFilter(u16);

impl FontFilter {
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn has(self, t: FontTrait) -> bool {
        self.0 & t.bit() != 0
    }

    pub fn set(&mut self, t: FontTrait, on: bool) {
        if on {
            self.0 |= t.bit();
        } else {
            self.0 &= !t.bit();
        }
    }

    /// The number of traits checked.
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// A family with `info` shows (`None`: not installed nor offered,
    /// which shows only when nothing is checked; an offered family is not
    /// read until fetched, so it has no trait).
    pub fn matches(self, info: Option<&FamilyInfo>) -> bool {
        if self.is_empty() {
            return true;
        }
        let Some(info) = info else {
            return false;
        };
        let mut any_class = false;
        let mut class_held = false;
        for t in FontTrait::ALL.into_iter().filter(|t| self.has(*t)) {
            if t.is_class() {
                any_class = true;
                class_held |= t.holds(info);
            } else if !t.holds(info) {
                return false;
            }
        }
        !any_class || class_held
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
            faces: Vec::new(),
            embedded: false,
            symbol: false,
            class,
            monospace: false,
            features: false,
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
    fn kinds_of_letters_widen_other_traits_narrow() {
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
        f.set(FontTrait::Variable, true);
        assert!(!f.matches(Some(&serif)));
        assert!(f.matches(Some(&sans)));
        assert!(!f.matches(Some(&mono_sans)));
        // Monospaced alone.
        let mut m = FontFilter::default();
        m.set(FontTrait::Monospace, true);
        assert!(m.matches(Some(&mono_sans)) && !m.matches(Some(&sans)));
        m.set(FontTrait::Monospace, false);
        assert!(m.is_empty());
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
