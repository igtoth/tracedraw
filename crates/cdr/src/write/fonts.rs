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
    // The English (United States) name of the Windows records first, as
    // files have them (a bold face that also names itself in other
    // languages keeps "Bold"), then any name that reads.
    let name = |id: u16| {
        let of_id = || f.names().into_iter().filter(move |n| n.name_id == id);
        of_id()
            .filter(|n| n.platform_id == ttf_parser::PlatformId::Windows && n.language_id == 0x0409)
            .find_map(|n| n.to_string())
            .or_else(|| of_id().find_map(|n| n.to_string()))
            .unwrap_or_default()
    };
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
    let style = name(2);
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
        style: out.style.clone(),
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
fn font_numbers(faces: &[FontFace], ids: &[(u16, String)]) -> Vec<u16> {
    let mut extra: Vec<(u16, &str)> = Vec::new();
    let mut next = ids.iter().map(|(id, _)| *id).max().unwrap_or(0);
    faces
        .iter()
        .map(|face| {
            let known = ids.iter().find(|(_, f)| *f == face.family).or_else(|| {
                ids.iter()
                    .find(|(_, f)| f.eq_ignore_ascii_case(&face.family))
            });
            if let Some((id, _)) = known {
                return *id;
            }
            if let Some((id, _)) = extra.iter().find(|(_, f)| *f == face.family) {
                return *id;
            }
            next = next.saturating_add(1);
            extra.push((next, &face.family));
            next
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
    let numbers = font_numbers(faces, ids);
    let mut order: Vec<usize> = (0..faces.len()).collect();
    order.sort_by_key(|&i| numbers[i]);
    let infos: Vec<FaceInfo> = faces.iter().map(info).collect();
    let mut table = Buf::default();
    table.u32(0).u32(0).u32(faces.len() as u32);
    let mut embeddings = Vec::new();
    // The font files embedded so far: a face drawn with another face's
    // file (italic text in a family without an italic face) is listed,
    // and its file embedded once.
    let mut embedded_files: Vec<&Arc<Vec<u8>>> = Vec::new();
    for i in order {
        let (face, inf) = (&faces[i], &infos[i]);
        let new_file = face.data.as_ref().filter(|d| {
            !embedded_files
                .iter()
                .any(|e| Arc::ptr_eq(e, d) || e.as_slice() == d.as_slice())
        });
        let embedded = embed && inf.embeddable && new_file.is_some();
        if let (true, Some(d)) = (embedded, new_file) {
            embedded_files.push(d);
        }
        let mut e = Buf::default();
        // The face's font number (its family's font id), then the
        // character set (Western).
        e.u16(numbers[i]).u16(0).u32(inf.flags);
        for p in inf.panose {
            e.u8(p);
        }
        string(&mut e, &face.family);
        e.u8(0);
        string(&mut e, "Western");
        e.u32(u32::from(embedded)).u32(inf.kind);
        string(&mut e, &inf.postscript);
        string(&mut e, &inf.version);
        string(&mut e, &inf.style);
        let e = e.done();
        table.u32(0).u32(e.len() as u32);
        table.bytes(&e);
        if embedded {
            if let (Some(data), Some(names)) = (&face.data, &inf.eot) {
                embeddings.push(embedding(inf.kind, &inf.version, data, names));
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
/// version string in UTF-16 with a closing zero, zeros up to 2705 bytes,
/// then the EOT structure.
const EMBEDDING_HEADER: usize = 2705;

fn embedding(kind: u32, version: &str, data: &[u8], names: &EotNames) -> Vec<u8> {
    let mut m = Vec::with_capacity(EMBEDDING_HEADER + data.len() + 256);
    m.extend_from_slice(&1u32.to_le_bytes());
    m.extend_from_slice(&0x803u32.to_le_bytes());
    m.push(kind as u8);
    for u in version.encode_utf16().chain(std::iter::once(0)) {
        m.extend_from_slice(&u.to_le_bytes());
    }
    m.truncate(EMBEDDING_HEADER);
    m.resize(EMBEDDING_HEADER, 0);
    m.extend(eot::encode(data, names));
    m
}
