//! The fonts of a version 16+ package: `font/fontTable.dat`, listing every
//! face the text uses, and `embed/embedding<n>`, the faces embedded. See
//! docs/cdr-format.md, Embedded fonts.

use super::Buf;
use crate::eot::{self, EotNames};
use std::sync::Arc;

/// A face the text uses, with its font file when one was found.
#[derive(Debug, Clone)]
pub struct FontFace {
    pub family: String,
    pub bold: bool,
    pub italic: bool,
    /// The TrueType or OpenType file (a single font, not a collection).
    pub data: Option<Arc<Vec<u8>>>,
}

/// What the font table and an embedding header say about a face.
struct FaceInfo {
    flags: u32,
    panose: [u8; 10],
    kind: u32,
    postscript: String,
    version: String,
    style: String,
    /// The face may be embedded (OS/2 `fsType` allows it).
    embeddable: bool,
    eot: Option<EotNames>,
}

/// The style flags of a face in font records: 0x40 regular, 0x1000 bold,
/// 0x80 italic, 0x2000 bold italic.
fn face_flags(bold: bool, italic: bool) -> u32 {
    match (bold, italic) {
        (false, false) => 0x40,
        (true, false) => 0x1000,
        (false, true) => 0x80,
        (true, true) => 0x2000,
    }
}

fn be16(b: &[u8], at: usize) -> u16 {
    b.get(at..at + 2)
        .map_or(0, |s| u16::from_be_bytes([s[0], s[1]]))
}

fn be32(b: &[u8], at: usize) -> u32 {
    b.get(at..at + 4)
        .map_or(0, |s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

/// The English (United States) name `id` of the Windows records first, as
/// files have them (a bold face that also names itself in other languages
/// keeps "Bold"), then any name that reads; empty when none does.
fn english_name(f: &ttf_parser::Face, id: u16) -> String {
    let of_id = || f.names().into_iter().filter(move |n| n.name_id == id);
    of_id()
        .filter(|n| n.platform_id == ttf_parser::PlatformId::Windows && n.language_id == 0x0409)
        .find_map(|n| n.to_string())
        .or_else(|| of_id().find_map(|n| n.to_string()))
        .unwrap_or_default()
}

/// The family a font names itself under: its typographic family (name
/// ID 16) when it has one, else its family (name ID 1). A static face of
/// a large family ("Khand SemiBold", ID 1) gives the family ("Khand").
fn typographic_family(f: &ttf_parser::Face) -> String {
    let t = english_name(f, 16);
    if t.trim().is_empty() {
        english_name(f, 1)
    } else {
        t
    }
}

/// The family and weight a face of `family` is written under when the
/// family is named with its weight ("Titillium Web SemiBold", as text
/// set in a semibold face reads): the family without the weight's name
/// and that weight ("Titillium Web" at 600), the way files name such a
/// face (#892). `None` keeps the name: bold text, a name with no weight
/// in it, or a font whose own family (its typographic family, else its
/// name ID 1) is the whole name. A font that names the whole name only
/// in ID 1 and the family in ID 16 ("Khand SemiBold" and "Khand") is
/// split: a file naming the whole name at the regular weight asked for
/// a font that is missing where the face is only embedded (#1055).
pub(crate) fn split_family(family: &str, bold: bool, faces: &[FontFace]) -> Option<(String, u16)> {
    if bold {
        return None;
    }
    let (head, weight) = crate::parse::family_weight(family)?;
    let named_so = faces
        .iter()
        .filter(|f| f.family == family)
        .filter_map(|f| f.data.as_deref())
        .filter_map(|d| ttf_parser::Face::parse(d, 0).ok())
        .any(|f| {
            typographic_family(&f)
                .trim()
                .eq_ignore_ascii_case(family.trim())
        });
    (!named_so).then(|| (head.to_string(), weight))
}

/// The style flags of a face of `weight` (one bit per 100 from 1 for
/// 100: 64 = 400, 1024 = 600, 4096 = 700), its italic the bit above.
fn weight_flags(weight: u16, italic: bool) -> u32 {
    let step = u32::from(weight.clamp(100, 900) / 100 - 1);
    1 << (2 * step + u32::from(italic))
}

fn info(face: &FontFace) -> FaceInfo {
    let style_name = match (face.bold, face.italic) {
        (false, false) => "Regular",
        (true, false) => "Bold",
        (false, true) => "Italic",
        (true, true) => "Bold Italic",
    };
    let mut out = FaceInfo {
        flags: face_flags(face.bold, face.italic),
        panose: [0; 10],
        kind: 4,
        postscript: face.family.replace(' ', ""),
        version: String::new(),
        style: style_name.to_string(),
        embeddable: false,
        eot: None,
    };
    let Some(data) = &face.data else {
        return out;
    };
    let Ok(f) = ttf_parser::Face::parse(data, 0) else {
        return out;
    };
    let name = |id: u16| english_name(&f, id);
    let raw = f.raw_face();
    let os2 = raw
        .table(ttf_parser::Tag::from_bytes(b"OS/2"))
        .unwrap_or(&[]);
    let head = raw
        .table(ttf_parser::Tag::from_bytes(b"head"))
        .unwrap_or(&[]);
    if let Some(p) = os2.get(32..42) {
        out.panose.copy_from_slice(p);
    }
    let fs_type = be16(os2, 8);
    // Restricted licence (2) or bitmaps only (0x200): not embedded.
    out.embeddable = !os2.is_empty() && fs_type & 0x000f != 0x0002 && fs_type & 0x0200 == 0;
    // 5 for PostScript outlines, 4 for TrueType ones, as files have them.
    if raw.table(ttf_parser::Tag::from_bytes(b"CFF ")).is_some() {
        out.kind = 5;
    }
    let ps = name(6);
    if !ps.is_empty() {
        out.postscript = ps;
    }
    out.version = name(5);
    // The typographic subfamily ("SemiBold") when the font has one, as
    // the font table names a face; else the subfamily (name ID 2).
    let style = Some(name(17))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| name(2));
    if !style.is_empty() {
        out.style = style;
    }
    let mut family = name(1);
    if family.is_empty() {
        family = face.family.clone();
    }
    out.eot = Some(EotNames {
        panose: out.panose,
        italic: face.italic,
        weight: u32::from(be16(os2, 4)).max(if face.bold { 700 } else { 400 }),
        fs_type,
        // Left zero for an OS/2 table of version 0, as files have it.
        unicode_ranges: if be16(os2, 0) >= 1 {
            [be32(os2, 42), be32(os2, 46), be32(os2, 50), be32(os2, 54)]
        } else {
            [0; 4]
        },
        code_page_ranges: if be16(os2, 0) >= 1 {
            [be32(os2, 78), be32(os2, 82)]
        } else {
            [0; 2]
        },
        checksum_adjustment: be32(head, 8),
        family,
        // The EOT names are the font's own family and subfamily (ID 1
        // and 2), as the format defines them.
        style: Some(name(2))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| out.style.clone()),
        version: out.version.clone(),
        full: name(4),
    });
    out
}

/// The bytes the faces that may be embedded add to a file (their font
/// files, each once), 0 when none may be.
pub fn embedded_size(faces: &[FontFace]) -> u64 {
    let mut seen: Vec<&Arc<Vec<u8>>> = Vec::new();
    for d in faces
        .iter()
        .filter_map(|f| f.data.as_ref().filter(|_| info(f).embeddable))
    {
        if !seen
            .iter()
            .any(|s| Arc::ptr_eq(s, d) || s.as_slice() == d.as_slice())
        {
            seen.push(d);
        }
    }
    seen.iter().map(|d| d.len() as u64).sum()
}

/// Why embedding leaves a face the text uses out of the file (#1219).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftOut {
    /// The font's licence (its OS/2 `fsType`) forbids embedding, or the
    /// font holds bitmaps only.
    Licence,
    /// No font file to embed: the face is drawn by a stand-in, or its
    /// file is a collection (`.ttc`).
    NoFile,
}

/// The families of `faces` that embedding leaves out, each once with the
/// reason; a family with one face that may be embedded is not listed.
pub fn left_out(faces: &[FontFace]) -> Vec<(String, LeftOut)> {
    let mut out: Vec<(String, LeftOut)> = Vec::new();
    for f in faces {
        let why = match &f.data {
            Some(_) if info(f).embeddable => {
                out.retain(|(family, _)| family != &f.family);
                continue;
            }
            Some(_) => LeftOut::Licence,
            None => LeftOut::NoFile,
        };
        let embedded = faces
            .iter()
            .any(|g| g.family == f.family && g.data.as_ref().is_some_and(|_| info(g).embeddable));
        if embedded || out.iter().any(|(family, _)| family == &f.family) {
            continue;
        }
        out.push((f.family.clone(), why));
    }
    out
}

/// A string as the font table stores it: its length in UTF-16 units with
/// the closing zero, then the units.
fn string(b: &mut Buf, s: &str) {
    let units: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    b.u32(units.len() as u32);
    for u in units {
        b.u16(u);
    }
}

/// The font number of each face: the id of its family's `font` records
/// in the drawing (`ids`, as the text refers to it), shared by the faces
/// of a family; a family the drawing does not name gets a number after
/// those.
fn font_numbers(families: &[String], ids: &[(u16, String)]) -> Vec<u16> {
    let mut extra: Vec<(u16, &str)> = Vec::new();
    let mut next = ids.iter().map(|(id, _)| *id).max().unwrap_or(0);
    families
        .iter()
        .map(|family| {
            let known = ids
                .iter()
                .find(|(_, f)| f == family)
                .or_else(|| ids.iter().find(|(_, f)| f.eq_ignore_ascii_case(family)));
            if let Some((id, _)) = known {
                return *id;
            }
            if let Some((id, _)) = extra.iter().find(|(_, f)| f == family) {
                return *id;
            }
            next = next.saturating_add(1);
            extra.push((next, family));
            next
        })
        .collect()
}

/// What the drawing's `font` records (version 16 and later) say about a
/// face, as the font table does: its font number and style flags, the
/// PANOSE bytes and PostScript name of its font, the kind, and whether
/// the package embeds it. Files mark an embedded face in its `font`
/// record too; a face marked there as not embedded asked for the font to
/// be installed even with its embedding in the package (#1009).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceRecord {
    pub number: u16,
    pub flags: u32,
    pub panose: [u8; 10],
    pub kind: u32,
    pub postscript: String,
    pub embedded: bool,
}

/// A face in the order the package lists it: its index in `faces`, what
/// the table says about it, the family it is listed under, its font
/// number and whether its file is embedded (once per file).
struct Planned {
    face: usize,
    info: FaceInfo,
    family: String,
    number: u16,
    embedded: bool,
}

/// The faces as the package lists them, in ascending font numbers
/// (faces of one family in the order given), with the file of each face
/// that may be embedded marked once: a face drawn with another face's
/// file (italic text in a family without an italic face) is listed, and
/// its file embedded once.
fn plan(faces: &[FontFace], embed: bool, ids: &[(u16, String)]) -> Vec<Planned> {
    // A face of a family named with its weight is listed under the
    // family, with the flags of its weight, as the text names it.
    let splits: Vec<Option<(String, u16)>> = faces
        .iter()
        .map(|f| split_family(&f.family, f.bold, faces))
        .collect();
    let families: Vec<String> = faces
        .iter()
        .zip(&splits)
        .map(|(f, s)| {
            s.as_ref()
                .map_or_else(|| f.family.clone(), |(h, _)| h.clone())
        })
        .collect();
    let numbers = font_numbers(&families, ids);
    let mut order: Vec<usize> = (0..faces.len()).collect();
    order.sort_by_key(|&i| numbers[i]);
    let mut embedded_files: Vec<&Arc<Vec<u8>>> = Vec::new();
    let mut out = Vec::with_capacity(faces.len());
    for (i, family) in order.into_iter().map(|i| (i, families[i].clone())) {
        let face = &faces[i];
        let mut inf = info(face);
        if let Some((_, w)) = &splits[i] {
            inf.flags = weight_flags(*w, face.italic);
        }
        let new_file = face.data.as_ref().filter(|d| {
            !embedded_files
                .iter()
                .any(|e| Arc::ptr_eq(e, d) || e.as_slice() == d.as_slice())
        });
        let embedded = embed && inf.embeddable && new_file.is_some();
        if let (true, Some(d)) = (embedded, new_file) {
            embedded_files.push(d);
        }
        out.push(Planned {
            face: i,
            info: inf,
            family,
            number: numbers[i],
            embedded,
        });
    }
    out
}

/// What the `font` records of the drawing say about each face (see
/// [`FaceRecord`]), with `embed` and `ids` as [`members`] takes them.
pub fn records(faces: &[FontFace], embed: bool, ids: &[(u16, String)]) -> Vec<FaceRecord> {
    plan(faces, embed, ids)
        .into_iter()
        .map(|p| FaceRecord {
            number: p.number,
            flags: p.info.flags,
            panose: p.info.panose,
            kind: p.info.kind,
            postscript: p.info.postscript,
            embedded: p.embedded,
        })
        .collect()
}

/// The fonts members of a package: the font table, and an embedding per
/// face that may be embedded when `embed` is set. `ids` are the drawing's
/// font ids by family. Entries and embeddings follow the font numbers
/// (faces of one family in the order given), as files order their
/// embeddings.
pub fn members(faces: &[FontFace], embed: bool, ids: &[(u16, String)]) -> Vec<(String, Vec<u8>)> {
    if faces.is_empty() {
        return Vec::new();
    }
    let mut table = Buf::default();
    table.u32(0).u32(0).u32(faces.len() as u32);
    let mut embeddings = Vec::new();
    for p in plan(faces, embed, ids) {
        let (face, inf) = (&faces[p.face], &p.info);
        let mut e = Buf::default();
        // The face's font number (its family's font id), then the
        // character set (Western).
        e.u16(p.number).u16(0).u32(inf.flags);
        for b in inf.panose {
            e.u8(b);
        }
        string(&mut e, &p.family);
        e.u8(0);
        string(&mut e, "Western");
        e.u32(u32::from(p.embedded)).u32(inf.kind);
        string(&mut e, &inf.postscript);
        string(&mut e, &inf.version);
        string(&mut e, &inf.style);
        let e = e.done();
        table.u32(0).u32(e.len() as u32);
        table.bytes(&e);
        if p.embedded {
            if let (Some(data), Some(names)) = (&face.data, &inf.eot) {
                let block = FaceBlock {
                    family: &p.family,
                    flags: inf.flags,
                    panose: inf.panose,
                    fs_type: names.fs_type,
                };
                embeddings.push(embedding(inf.kind, &inf.version, &block, data, names));
            }
        }
    }
    let mut table = table.done();
    let size = (table.len() - 8) as u32;
    table[4..8].copy_from_slice(&size.to_le_bytes());
    let mut out = vec![("font/fontTable.dat".to_string(), table)];
    for (n, m) in embeddings.into_iter().enumerate() {
        out.push((format!("embed/embedding{n}"), m));
    }
    out
}

/// An `embed/` member: u32 1, u32 0x803, the face's kind byte, its
/// version string in UTF-16 with a closing zero, then at 2063 the face
/// block (below), which runs to 2705 bytes, then the EOT structure.
const EMBEDDING_HEADER: usize = 2705;
/// Where the face block starts in the header: its u32 size (638, the
/// rest of the header), then the block.
const FACE_BLOCK_AT: usize = 2063;
/// The version string field ends before the face block: 1026 UTF-16
/// units at most (2052 bytes from offset 9, then zeros).
const VERSION_UNITS: usize = 1026;

/// The face block of an embedding header, which names the face the
/// member holds: the family in UTF-16 (32 units) and then in single
/// bytes (32), the style flags of the font table, a zero byte, the
/// PANOSE bytes, the embedding level of the font's licence, five zero
/// bytes and u32 1. Files hold it in every member seen (the 14 members
/// of the version 19 file of #624 and the version 27 file of #313);
/// members without it (written before 2026-10-28) asked for the
/// font to be installed (#1009).
struct FaceBlock<'a> {
    family: &'a str,
    flags: u32,
    panose: [u8; 10],
    fs_type: u16,
}

/// The embedding level byte of the face block from the font's OS/2
/// `fsType`: 3 for installable fonts (0, and the reserved 1 old fonts
/// carry), 2 for editable embedding (8), as the files seen pair them;
/// 1 for preview and print (4) is taken to continue the ladder.
fn embedding_level(fs_type: u16) -> u8 {
    match fs_type & 0x000f {
        0 | 1 => 3,
        8 => 2,
        _ => 1,
    }
}

fn embedding(kind: u32, version: &str, face: &FaceBlock, data: &[u8], names: &EotNames) -> Vec<u8> {
    let mut m = Vec::with_capacity(EMBEDDING_HEADER + data.len() + 256);
    m.extend_from_slice(&1u32.to_le_bytes());
    m.extend_from_slice(&0x803u32.to_le_bytes());
    m.push(kind as u8);
    for u in version
        .encode_utf16()
        .take(VERSION_UNITS - 1)
        .chain(std::iter::once(0))
    {
        m.extend_from_slice(&u.to_le_bytes());
    }
    m.resize(FACE_BLOCK_AT, 0);
    m.extend_from_slice(&((EMBEDDING_HEADER - FACE_BLOCK_AT - 4) as u32).to_le_bytes());
    let units: Vec<u16> = face.family.encode_utf16().take(31).collect();
    for u in &units {
        m.extend_from_slice(&u.to_le_bytes());
    }
    m.resize(FACE_BLOCK_AT + 4 + 64, 0);
    // The single-byte name: the characters of the family that fit a
    // byte, as files name Western faces.
    let bytes: Vec<u8> = face
        .family
        .chars()
        .filter_map(|c| u8::try_from(u32::from(c)).ok())
        .take(31)
        .collect();
    m.extend_from_slice(&bytes);
    m.resize(FACE_BLOCK_AT + 4 + 96, 0);
    m.extend_from_slice(&face.flags.to_le_bytes());
    m.push(0);
    m.extend_from_slice(&face.panose);
    m.push(embedding_level(face.fs_type));
    m.extend_from_slice(&[0; 5]);
    m.extend_from_slice(&1u32.to_le_bytes());
    m.resize(EMBEDDING_HEADER, 0);
    m.extend(eot::encode(data, names));
    m
}
