//! Embedded OpenType (EOT) font wrappers. `.cdr` files keep the fonts
//! they embed in `embed/embedding<n>` members of the ZIP container (seen
//! in a version 21 file): a header of their own, then an EOT structure
//! that runs to the end of the member. The EOT layout follows the public
//! W3C Member Submission "Embedded OpenType (EOT) File Format" (2008): a
//! little-endian header with sizes, flags and names, the font file last.
//! See docs/cdr-format.md.

/// Flag: the font data is compressed (MicroType Express); not read.
const FLAG_COMPRESSED: u32 = 0x0000_0004;
/// Flag: every byte of the font data is XOR-ed with 0x50.
const FLAG_XOR: u32 = 0x1000_0000;
const XOR_KEY: u8 = 0x50;
/// `MagicNumber`, at the same offset in every version.
const MAGIC: u16 = 0x504C;
const MAGIC_AT: usize = 34;
/// The versions of the submission: 1.0, 2.1 and 2.2.
const VERSIONS: [u32; 3] = [0x0001_0000, 0x0002_0001, 0x0002_0002];
/// Offset of `FamilyNameSize`, the same in every version; the shortest
/// header ends 8 bytes later (four empty names with their padding).
const FAMILY_AT: usize = 82;
const MIN_HEADER: usize = FAMILY_AT + 2 + 3 * 4;
/// `RootStringCheckSum` of an empty root string, as files embed fonts.
const ROOT_CHECKSUM_EMPTY: u32 = 0x5047_5342;
/// `EUDCCodePage` as files embed fonts.
const EUDC_CODE_PAGE: u32 = 1252;

/// A font read from an EOT structure.
#[derive(Debug, Clone, PartialEq)]
pub struct EotFont {
    /// `FamilyName` of the header (may be empty).
    pub family: String,
    /// The TrueType or OpenType file.
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EotError {
    #[error("no embedded OpenType structure")]
    NotEot,
    #[error("the font data is compressed (MicroType Express), which is not read")]
    Compressed,
    #[error("the font data is not a TrueType or OpenType file")]
    NotAFont,
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// The EOT header at `at` in `bytes` is well formed: a known version, the
/// magic number and sizes that fit. Returns the structure's size.
fn header_at(bytes: &[u8], at: usize) -> Option<usize> {
    let size = u32_at(bytes, at)? as usize;
    let font_size = u32_at(bytes, at + 4)? as usize;
    let version = u32_at(bytes, at + 8)?;
    let fits = size >= MIN_HEADER
        && at.checked_add(size)? <= bytes.len()
        && font_size <= size - MIN_HEADER;
    (fits && VERSIONS.contains(&version) && u16_at(bytes, at + MAGIC_AT)? == MAGIC).then_some(size)
}

/// Where an EOT structure starts in `member`: the one that runs to the
/// member's end, else the first well-formed one.
pub fn find(member: &[u8]) -> Option<usize> {
    let last = member.len().checked_sub(MIN_HEADER)?;
    (0..=last)
        .find(|&i| header_at(member, i) == Some(member.len() - i))
        .or_else(|| (0..=last).find(|&i| header_at(member, i).is_some()))
}

/// Read the EOT structure at the start of `bytes`.
pub fn read(bytes: &[u8]) -> Result<EotFont, EotError> {
    let size = header_at(bytes, 0).ok_or(EotError::NotEot)?;
    let eot = bytes.get(..size).ok_or(EotError::NotEot)?;
    let font_size = u32_at(eot, 4).ok_or(EotError::NotEot)? as usize;
    let flags = u32_at(eot, 12).ok_or(EotError::NotEot)?;
    let family = u16_at(eot, FAMILY_AT)
        .and_then(|n| eot.get(FAMILY_AT + 2..FAMILY_AT + 2 + n as usize))
        .map(utf16le)
        .unwrap_or_default();
    if flags & FLAG_COMPRESSED != 0 {
        return Err(EotError::Compressed);
    }
    let mut data = eot
        .get(size - font_size..)
        .ok_or(EotError::NotEot)?
        .to_vec();
    if flags & FLAG_XOR != 0 {
        data.iter_mut().for_each(|b| *b ^= XOR_KEY);
    }
    let sfnt = matches!(
        data.get(..4),
        Some([0, 1, 0, 0] | b"OTTO" | b"true" | b"ttcf")
    );
    if !sfnt {
        return Err(EotError::NotAFont);
    }
    Ok(EotFont { family, data })
}

/// A name of the header: UTF-16LE, without trailing NULs or spaces.
fn utf16le(b: &[u8]) -> String {
    let units: Vec<u16> = b
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&units)
        .trim_end_matches(['\0', ' '])
        .to_string()
}

/// The fields of an EOT header taken from the font itself.
#[derive(Debug, Clone, Default)]
pub struct EotNames {
    pub panose: [u8; 10],
    pub italic: bool,
    pub weight: u32,
    pub fs_type: u16,
    pub unicode_ranges: [u32; 4],
    pub code_page_ranges: [u32; 2],
    pub checksum_adjustment: u32,
    pub family: String,
    pub style: String,
    pub version: String,
    pub full: String,
}

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
}

/// An EOT structure (version 2.2) around `font` as files embed fonts:
/// the data XOR-ed (flag 0x10000000), not compressed, the header filled
/// from `names`.
pub fn encode(font: &[u8], names: &EotNames) -> Vec<u8> {
    let data: Vec<u8> = font.iter().map(|b| b ^ XOR_KEY).collect();
    let mut h = Vec::new();
    h.extend_from_slice(&0u32.to_le_bytes()); // EOTSize, set below
    h.extend_from_slice(&(data.len() as u32).to_le_bytes());
    h.extend_from_slice(&0x0002_0002u32.to_le_bytes());
    h.extend_from_slice(&FLAG_XOR.to_le_bytes());
    h.extend_from_slice(&names.panose);
    h.push(0); // charset: 0, as the files seen have it for every face
    h.push(u8::from(names.italic));
    h.extend_from_slice(&names.weight.to_le_bytes());
    h.extend_from_slice(&names.fs_type.to_le_bytes());
    h.extend_from_slice(&MAGIC.to_le_bytes());
    for r in names.unicode_ranges {
        h.extend_from_slice(&r.to_le_bytes());
    }
    for r in names.code_page_ranges {
        h.extend_from_slice(&r.to_le_bytes());
    }
    h.extend_from_slice(&names.checksum_adjustment.to_le_bytes());
    h.extend_from_slice(&[0; 16]); // reserved
    h.extend_from_slice(&[0; 2]); // padding
    debug_assert_eq!(h.len(), FAMILY_AT);
    // The family and style names end with a NUL, the version and full
    // names do not, as in the files seen.
    for (i, (name, nul)) in [
        (&names.family, true),
        (&names.style, true),
        (&names.version, false),
        (&names.full, false),
    ]
    .into_iter()
    .enumerate()
    {
        if i > 0 {
            h.extend_from_slice(&[0; 2]); // padding
        }
        let mut b = utf16(name);
        if nul {
            b.extend_from_slice(&[0; 2]);
        }
        let len = b.len().min(usize::from(u16::MAX) & !1);
        h.extend_from_slice(&(len as u16).to_le_bytes());
        h.extend_from_slice(&b[..len]);
    }
    // Padding, an empty root string, its checksum (that of no bytes),
    // the EUDC code page (1252), padding, an empty signature, EUDC flags
    // and an empty EUDC font.
    h.extend_from_slice(&[0; 2 + 2]);
    h.extend_from_slice(&ROOT_CHECKSUM_EMPTY.to_le_bytes());
    h.extend_from_slice(&EUDC_CODE_PAGE.to_le_bytes());
    h.extend_from_slice(&[0; 2 + 2 + 4 + 4]);
    h.extend_from_slice(&data);
    let size = h.len() as u32;
    h[0..4].copy_from_slice(&size.to_le_bytes());
    h
}

/// An EOT structure (version 2.2) around `font`, for tests: `flags` as
/// given, the data XOR-ed when the flags say so.
#[cfg(test)]
pub(crate) fn wrap(font: &[u8], family: &str, flags: u32) -> Vec<u8> {
    let mut data = font.to_vec();
    if flags & FLAG_XOR != 0 {
        data.iter_mut().for_each(|b| *b ^= XOR_KEY);
    }
    let name: Vec<u8> = family
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    let mut head = Vec::new();
    head.extend_from_slice(&0u32.to_le_bytes()); // EOTSize, set below
    head.extend_from_slice(&(data.len() as u32).to_le_bytes());
    head.extend_from_slice(&0x0002_0002u32.to_le_bytes());
    head.extend_from_slice(&flags.to_le_bytes());
    head.extend_from_slice(&[0; 10]); // PANOSE
    head.extend_from_slice(&[1, 0]); // charset, italic
    head.extend_from_slice(&400u32.to_le_bytes());
    head.extend_from_slice(&8u16.to_le_bytes()); // fsType
    head.extend_from_slice(&MAGIC.to_le_bytes());
    head.extend_from_slice(&[0; 4 * 4 + 2 * 4 + 4 + 4 * 4 + 2]);
    assert_eq!(head.len(), FAMILY_AT);
    head.extend_from_slice(&(name.len() as u16).to_le_bytes());
    head.extend_from_slice(&name);
    // Style, version and full names, empty.
    for _ in 0..3 {
        head.extend_from_slice(&[0; 4]);
    }
    // Version 2.1: the root string; 2.2: its checksum, the EUDC code
    // page, the signature and the EUDC font, all empty.
    head.extend_from_slice(&[0; 4]);
    head.extend_from_slice(&[0; 4 + 4 + 2 + 2 + 4 + 4]);
    head.extend_from_slice(&data);
    let size = head.len() as u32;
    head[0..4].copy_from_slice(&size.to_le_bytes());
    head
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bundled interface font (SIL Open Font License).
    const FONT: &[u8] = include_bytes!("../../../apps/tracedraw/assets/fonts/Selawik-Regular.ttf");

    #[test]
    fn plain_and_xored_fonts_come_out_as_they_went_in() {
        for flags in [0, FLAG_XOR, FLAG_XOR | 0x1] {
            let eot = wrap(FONT, "Selawik ", flags);
            let font = read(&eot).expect("font");
            assert_eq!(font.data, FONT);
            assert_eq!(font.family, "Selawik");
        }
    }

    #[test]
    fn the_structure_is_found_after_a_member_header() {
        let mut member = b"a header of another kind, any length".to_vec();
        let at = member.len();
        // A false start inside the header: a magic number with no valid
        // version around it.
        member.extend_from_slice(&[0; 34]);
        member.extend_from_slice(&MAGIC.to_le_bytes());
        let at = at + 36;
        member.extend_from_slice(&wrap(FONT, "Selawik", FLAG_XOR));
        assert_eq!(find(&member), Some(at));
        assert_eq!(read(&member[at..]).expect("font").data, FONT);
    }

    #[test]
    fn damaged_or_unsupported_structures_are_refused() {
        let eot = wrap(FONT, "Selawik", 0);
        assert_eq!(read(&[]), Err(EotError::NotEot));
        assert_eq!(read(&eot[..60]), Err(EotError::NotEot));
        // Cut short: the size no longer fits.
        assert_eq!(read(&eot[..eot.len() - 1]), Err(EotError::NotEot));
        assert_eq!(find(&eot[..eot.len() - 1]), None);
        // Unknown version, wrong magic number.
        let mut v = eot.clone();
        v[8] = 7;
        assert_eq!(read(&v), Err(EotError::NotEot));
        let mut m = eot.clone();
        m[MAGIC_AT] = 0;
        assert_eq!(read(&m), Err(EotError::NotEot));
        // A font size larger than the structure.
        let mut s = eot.clone();
        s[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(read(&s), Err(EotError::NotEot));
        // Compressed data is reported, not read.
        let c = wrap(FONT, "Selawik", FLAG_COMPRESSED);
        assert_eq!(read(&c), Err(EotError::Compressed));
        // The XOR flag missing: the bytes are no font.
        let mut x = wrap(FONT, "Selawik", FLAG_XOR);
        x[12..16].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(read(&x), Err(EotError::NotAFont));
        // Odd or oversized name lengths do not matter.
        let mut n = eot.clone();
        n[FAMILY_AT..FAMILY_AT + 2].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(read(&n).expect("font").family, "");
        // Any bytes: no panic.
        for len in 0..200 {
            let junk: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            let _ = find(&junk);
            let _ = read(&junk);
        }
    }
}
