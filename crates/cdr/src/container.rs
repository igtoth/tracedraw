//! Container detection: bare RIFF, or ZIP wrapping a RIFF stream.

use crate::{Error, Result};
use std::io::Read;
use tracedraw_core::Color;

/// CDR major version, as encoded in the RIFF form type (`CDR9`,
/// `CDRA` = 10, ... `CDRE` = 14, `CDRH` = 17, `CDRJ` = 18,
/// `CDRK` = 19, and so on; the letter `I` is not used).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u16);

impl Version {
    pub fn from_form_type(form: &[u8; 4]) -> Option<Version> {
        let c = form[3];
        // Templates carry `CDT` and their own letters: `CDTQ` holds a
        // version 21 template (its `vrsn` 2100). The `vrsn` chunk, read
        // later, has the last word.
        if &form[..3] == b"CDT" {
            return match c {
                b'A'..=b'Z' => Some(Version(5 + (c - b'A') as u16)),
                _ => None,
            };
        }
        if &form[..3] != b"CDR" && &form[..3] != b"cdr" {
            return None;
        }
        // Digits are versions 1 to 9; letters continue from `A` = 10 with
        // `I` skipped: `H` = 17, `J` = 18, `N` = 22, `O` = 23. From there
        // some updates took a letter of their own: `P` = 24, `Q` = 24 (its
        // later update), `R` = 25, `S` = 25 (update), `T` = 26, `U` = 27.
        // Version 3 files carry a space. The `vrsn` chunk has the last word.
        match c {
            b' ' => Some(Version(3)),
            b'1'..=b'9' => Some(Version((c - b'0') as u16)),
            b'A'..=b'H' => Some(Version(10 + (c - b'A') as u16)),
            b'J'..=b'O' => Some(Version(9 + (c - b'A') as u16)),
            b'P' | b'Q' => Some(Version(24)),
            b'R' | b'S' => Some(Version(25)),
            b'T'..=b'Z' => Some(Version(26 + (c - b'T') as u16)),
            _ => None,
        }
    }

    /// Human name: 9 → "CDR version 9", 21 → "CDR version 21".
    pub fn name(self) -> String {
        match self.0 {
            0 => "CDR (unknown version)".into(),
            v => format!("CDR version {v}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Container {
    pub kind: ContainerKind,
    pub version: Version,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerKind {
    Riff,
    /// ZIP archive; the RIFF stream is the named member. version 16 and later
    /// store most chunk payloads outside the RIFF stream, in the members
    /// listed by `content/dataFileList.dat` (`data_files`, in that order).
    Zip {
        riff_member: String,
        data_files: Vec<String>,
    },
}

const RIFF_MEMBERS: &[&str] = &[
    "content/riffData.cdr",
    "content/riffdata.cdr",
    "riffData.cdr",
    "content/root.dat",
];

const DATA_FILE_LIST: &str = "content/dataFileList.dat";

pub fn detect(bytes: &[u8]) -> Result<Container> {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" {
        let form: [u8; 4] = bytes[8..12].try_into().map_err(|_| Error::NotCdr)?;
        let version = Version::from_form_type(&form).ok_or(Error::NotCdr)?;
        return Ok(Container {
            kind: ContainerKind::Riff,
            version,
        });
    }
    if bytes.len() >= 4 && &bytes[0..4] == b"PK\x03\x04" {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)?;
        let member = RIFF_MEMBERS
            .iter()
            .find(|m| archive.by_name(m).is_ok())
            .map(|m| m.to_string())
            .or_else(|| {
                archive
                    .file_names()
                    .find(|n| n.to_ascii_lowercase().ends_with("riffdata.cdr"))
                    .map(String::from)
            })
            .ok_or_else(|| {
                Error::UnsupportedContainer("ZIP without a riffData.cdr member".into())
            })?;
        let mut head = [0u8; 12];
        {
            let mut f = archive.by_name(&member)?;
            f.read_exact(&mut head).map_err(|_| Error::Truncated(0))?;
        }
        if &head[0..4] != b"RIFF" {
            return Err(Error::UnsupportedContainer(format!(
                "{member} is not a RIFF stream"
            )));
        }
        let form: [u8; 4] = head[8..12].try_into().map_err(|_| Error::NotCdr)?;
        let version = Version::from_form_type(&form).ok_or(Error::NotCdr)?;
        // version 16+: the order of external data streams.
        let mut data_files = Vec::new();
        if let Ok(mut list) = archive.by_name(DATA_FILE_LIST) {
            let mut text = String::new();
            if list.read_to_string(&mut text).is_ok() {
                data_files = text
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
            }
        }
        return Ok(Container {
            kind: ContainerKind::Zip {
                riff_member: member,
                data_files,
            },
            version,
        });
    }
    Err(Error::NotCdr)
}

/// The RIFF stream as a contiguous byte vector.
pub fn riff_stream(bytes: &[u8], container: &Container) -> Result<Vec<u8>> {
    match &container.kind {
        ContainerKind::Riff => Ok(bytes.to_vec()),
        ContainerKind::Zip { riff_member, .. } => {
            let cursor = std::io::Cursor::new(bytes);
            let mut archive = zip::ZipArchive::new(cursor)?;
            let f = archive.by_name(riff_member)?;
            // The size the archive claims is not trusted for the
            // allocation, and what inflates past the limit is cut off.
            let mut out = Vec::with_capacity(f.size().min(MAX_STREAM) as usize);
            f.take(MAX_STREAM).read_to_end(&mut out)?;
            if out.len() as u64 >= MAX_STREAM {
                log::warn!("{riff_member} inflates past {MAX_STREAM} bytes; cut off there");
            }
            Ok(out)
        }
    }
}

/// The external data streams of an version 16+ file, in `dataFileList.dat` order.
/// A missing member yields an empty stream so indices stay aligned.
pub fn external_streams(bytes: &[u8], container: &Container) -> Vec<Vec<u8>> {
    let ContainerKind::Zip { data_files, .. } = &container.kind else {
        return Vec::new();
    };
    data_files
        .iter()
        .map(|name| {
            external_member(bytes, container, name).unwrap_or_else(|| {
                log::warn!("data stream {name} listed but missing from the archive");
                Vec::new()
            })
        })
        .collect()
}

/// Read an external data member (`content/data/<name>`) from a ZIP file.
/// Returns None for a RIFF container or a missing member.
pub fn external_member(bytes: &[u8], container: &Container, name: &str) -> Option<Vec<u8>> {
    if container.kind == ContainerKind::Riff {
        return None;
    }
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).ok()?;
    let candidates = [
        format!("content/data/{name}"),
        format!("content/{name}"),
        name.to_string(),
    ];
    for c in candidates {
        if let Ok(f) = archive.by_name(&c) {
            // The member's own size, as the archive declares it, within
            // the ceiling: a drawing of large bitmaps declares its
            // `Bitmaps.dat` honestly (850 MB in a version 27 file of two
            // pictures on a 1.5 m page, #323), a bomb claims less than it
            // inflates to and stops there.
            let declared = f.size();
            let limit = member_limit(declared);
            let mut out = Vec::new();
            if f.take(limit).read_to_end(&mut out).is_ok() {
                if cut_short(out.len() as u64, limit, declared) {
                    log::warn!("{c} inflates past {limit} bytes; cut off there");
                }
                return Some(out);
            }
        }
    }
    None
}

/// A member of TraceDraw's own in a ZIP package (see
/// [`crate::kept_blends`]), read up to `limit` bytes; `None` for RIFF
/// files, packages without it and members past the limit.
pub(crate) fn own_member(
    bytes: &[u8],
    container: &Container,
    name: &str,
    limit: u64,
) -> Option<Vec<u8>> {
    if container.kind == ContainerKind::Riff {
        return None;
    }
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    let f = archive.by_name(name).ok()?;
    if f.size() > limit {
        log::warn!("{name} declares {} bytes; left out", f.size());
        return None;
    }
    let mut out = Vec::new();
    f.take(limit + 1).read_to_end(&mut out).ok()?;
    (out.len() as u64 <= limit).then_some(out)
}

/// Embedded members larger than this are skipped (fonts are far smaller;
/// this keeps a damaged size from taking all memory).
const MAX_EMBEDDED: u64 = 64 << 20;

/// The most a member of the package (the RIFF stream, a data stream) or
/// one compressed block may inflate to: a file that claims more is a
/// decompression bomb or damaged, and is read only up to here.
pub(crate) const MAX_STREAM: u64 = 512 << 20;

/// The most an external data member may inflate to when the archive
/// declares it that large (the bitmaps of a big drawing).
pub(crate) const MAX_DATA_STREAM: u64 = 3 << 30;

/// How far a data member declared `declared` bytes long may inflate:
/// [`MAX_STREAM`] at least, its declared size up to [`MAX_DATA_STREAM`].
pub(crate) fn member_limit(declared: u64) -> u64 {
    declared.clamp(MAX_STREAM, MAX_DATA_STREAM)
}

/// Whether a member read to `len` bytes under `limit` was cut off: it
/// filled the limit while claiming another size (a bomb inflating past
/// what it declares, or a member larger than the ceiling).
pub(crate) fn cut_short(len: u64, limit: u64, declared: u64) -> bool {
    len >= limit && declared != limit
}

/// The CMYK colour profile a version 14+ ZIP file carries (a member
/// under `color/profiles/cmyk/`, seen in version 17 to 21 files next to
/// `color/color.xml`, which names it), when it reads as an ICC profile.
/// `None` for RIFF files and files without one.
/// The picture of the first page a ZIP container keeps (`previews/` from
/// version 17, `metadata/thumbnails/` before), when there is one.
pub fn page_preview(bytes: &[u8], container: &Container) -> Option<crate::spot::Preview> {
    if container.kind == ContainerKind::Riff {
        return None;
    }
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    for name in ["previews/page1.png", "metadata/thumbnails/page1.bmp"] {
        let Ok(f) = archive.by_name(name) else {
            continue;
        };
        if f.size() > MAX_PROFILE {
            continue;
        }
        let mut data = Vec::new();
        if std::io::Read::read_to_end(&mut f.take(MAX_PROFILE), &mut data).is_err() {
            continue;
        }
        if let Some(p) = crate::spot::Preview::decode(&data) {
            return Some(p);
        }
    }
    None
}

pub fn cmyk_profile(
    bytes: &[u8],
    container: &Container,
) -> Option<tracedraw_core::DocumentProfile> {
    if container.kind == ContainerKind::Riff {
        return None;
    }
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    let mut names: Vec<String> = archive
        .file_names()
        .filter(|n| {
            let lower = n.to_ascii_lowercase();
            lower.starts_with("color/profiles/cmyk/")
                && (lower.ends_with(".icc") || lower.ends_with(".icm"))
        })
        .map(String::from)
        .collect();
    names.sort();
    for name in names {
        let Ok(f) = archive.by_name(&name) else {
            continue;
        };
        if f.size() > MAX_PROFILE {
            continue;
        }
        let mut data = Vec::new();
        if f.take(MAX_PROFILE).read_to_end(&mut data).is_err() {
            continue;
        }
        let Ok(profile) = tracedraw_core::icc::Profile::parse(&data) else {
            continue;
        };
        if profile.color_space() != tracedraw_core::icc::ColorSpace::Cmyk {
            continue;
        }
        let file = name.rsplit('/').next().unwrap_or(&name);
        let label = profile
            .description()
            .map(String::from)
            .unwrap_or_else(|| file.to_string());
        return Some(tracedraw_core::DocumentProfile::new(label, data));
    }
    None
}

/// The largest colour profile read from a file.
const MAX_PROFILE: u64 = 16 << 20;

/// The fonts a ZIP file embeds: each `embed/embedding<n>` member holds a
/// header of its own and then an EOT structure (see [`crate::eot`]).
/// Returns the fonts, without repeats, and a note for each member that
/// could not be read.
pub fn embedded_fonts(
    bytes: &[u8],
    container: &Container,
) -> (Vec<tracedraw_core::DocumentFont>, Vec<String>) {
    let mut fonts: Vec<tracedraw_core::DocumentFont> = Vec::new();
    let mut notes = Vec::new();
    if container.kind == ContainerKind::Riff {
        return (fonts, notes);
    }
    let cursor = std::io::Cursor::new(bytes);
    let Ok(mut archive) = zip::ZipArchive::new(cursor) else {
        return (fonts, notes);
    };
    // Next to each font, newer files keep pictures of it and a sample
    // ("embedding0.png", "embedding0-Extended.png",
    // "embedding0.sample.curve"): not fonts, left alone.
    const NOT_FONTS: [&str; 7] = [".png", ".curve", ".jpg", ".jpeg", ".bmp", ".xml", ".json"];
    let mut names: Vec<String> = archive
        .file_names()
        .filter(|n| {
            let lower = n.to_ascii_lowercase();
            lower.starts_with("embed/")
                && !n.ends_with('/')
                && !NOT_FONTS.iter().any(|e| lower.ends_with(e))
        })
        .map(String::from)
        .collect();
    names.sort();
    for name in names {
        let Ok(f) = archive.by_name(&name) else {
            continue;
        };
        if f.size() > MAX_EMBEDDED {
            notes.push(format!("embedded member {name} is too large, skipped"));
            continue;
        }
        let mut member = Vec::new();
        if f.take(MAX_EMBEDDED).read_to_end(&mut member).is_err() {
            notes.push(format!("embedded member {name} could not be unpacked"));
            continue;
        }
        let Some(at) = crate::eot::find(&member) else {
            notes.push(format!(
                "embedded member {name} holds no font that can be read"
            ));
            continue;
        };
        match member.get(at..).map(crate::eot::read) {
            Some(Ok(font)) => {
                if !fonts.iter().any(|f| f.data.as_slice() == font.data) {
                    fonts.push(tracedraw_core::DocumentFont::new(font.family, font.data));
                }
            }
            Some(Err(e)) => notes.push(format!("embedded font in {name}: {e}")),
            None => {}
        }
    }
    (fonts, notes)
}

/// The document palette of an version 14+ ZIP file (`color/docPalette.xml`):
/// each `<color cs=".." name=".." tints=".."/>` with its name. CMYK, RGB
/// and grey colours are read; other colour spaces are skipped.
pub fn document_palette(bytes: &[u8], container: &Container) -> Vec<(String, Color)> {
    if container.kind == ContainerKind::Riff {
        return Vec::new();
    }
    let cursor = std::io::Cursor::new(bytes);
    let Ok(mut archive) = zip::ZipArchive::new(cursor) else {
        return Vec::new();
    };
    let mut xml = String::new();
    for name in ["color/docPalette.xml", "color/DocumentPalette.xml"] {
        if let Ok(f) = archive.by_name(name) {
            let mut raw = Vec::new();
            if f.take(MAX_PROFILE).read_to_end(&mut raw).is_ok() {
                xml = String::from_utf8_lossy(&raw).into_owned();
                break;
            }
        }
    }
    parse_palette_xml(&xml)
}

/// The colours of a palette XML document, in order.
pub fn parse_palette_xml(xml: &str) -> Vec<(String, Color)> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find("<color ") {
        rest = &rest[at + 7..];
        let end = rest.find('>').unwrap_or(rest.len());
        let tag = &rest[..end];
        rest = &rest[end..];
        let cs = attribute(tag, "cs")
            .unwrap_or_default()
            .to_ascii_uppercase();
        let name = attribute(tag, "name").unwrap_or_default();
        let tints: Vec<f32> = attribute(tag, "tints")
            .unwrap_or_default()
            .split(',')
            .filter_map(|v| v.trim().parse::<f32>().ok())
            .filter(|v| v.is_finite())
            .map(|v| v.clamp(0.0, 1.0))
            .collect();
        let color = match (cs.as_str(), tints.as_slice()) {
            ("CMYK", [c, m, y, k]) => Color::Cmyk {
                c: *c,
                m: *m,
                y: *y,
                k: *k,
            },
            ("RGB", [r, g, b]) => Color::Rgb {
                r: *r,
                g: *g,
                b: *b,
            },
            ("GRAY" | "GRAYSCALE", [v]) => Color::Gray { v: *v },
            // The other models, each component a fraction of 255 (the
            // names give the values: "C60 M169 Y255", "H119 S80 B190",
            // "H295 L165 S69", "L76 a-14 b66", "Y100 I69 Q89"; seen in a
            // version 17 file). CMY is the complement of RGB.
            ("CMY", [c, m, y]) => Color::Rgb {
                r: 1.0 - c,
                g: 1.0 - m,
                b: 1.0 - y,
            },
            ("HSB" | "HSV", [h, s, b]) => Color::Hsb {
                h: h * 360.0,
                s: *s,
                b: *b,
            },
            ("HLS", [h, l, s]) => Color::Hsl {
                h: h * 360.0,
                s: *s,
                l: *l,
            },
            ("LAB", [l, a, b]) => Color::Lab {
                l: l * 100.0,
                a: a * 255.0 - 128.0,
                b: b * 255.0 - 128.0,
            },
            ("YIQ", [y, i, q]) => Color::Yiq {
                y: *y,
                i: *i,
                q: *q,
            },
            ("REGISTRATION", _) => Color::Registration,
            _ => {
                log::warn!("document palette colour {name:?} in {cs:?} skipped");
                continue;
            }
        };
        out.push((name, color));
    }
    out
}

/// The value of `key="..."` in a tag, with the five XML entities decoded.
fn attribute(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let mut search = tag;
    loop {
        let at = search.find(&pat)?;
        // A whole attribute name, not the end of a longer one.
        let ok = at == 0 || search[..at].ends_with(char::is_whitespace);
        let after = &search[at + pat.len()..];
        if ok {
            let end = after.find('"')?;
            let v = &after[..end];
            return Some(
                v.replace("&quot;", "\"")
                    .replace("&apos;", "'")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&amp;", "&"),
            );
        }
        search = after;
    }
}

#[cfg(test)]
mod tests {
    /// A data member inflates to the size the archive declares for it,
    /// within the ceiling (a big drawing's bitmaps, #323), and never
    /// below the plain limit; it is cut short when it fills the limit
    /// while claiming another size.
    #[test]
    fn data_members_inflate_to_their_declared_size_within_the_ceiling() {
        assert_eq!(super::member_limit(1 << 20), super::MAX_STREAM);
        assert_eq!(super::member_limit(851_688_712), 851_688_712);
        assert_eq!(super::member_limit(5 << 30), super::MAX_DATA_STREAM);
        // Read whole: not cut.
        assert!(!super::cut_short(851_688_712, 851_688_712, 851_688_712));
        assert!(!super::cut_short(100, super::MAX_STREAM, 100));
        // A member claiming 100 bytes that fills the plain limit: a bomb.
        assert!(super::cut_short(super::MAX_STREAM, super::MAX_STREAM, 100));
        // Past the ceiling: cut there.
        assert!(super::cut_short(
            super::MAX_DATA_STREAM,
            super::MAX_DATA_STREAM,
            5 << 30
        ));
    }

    use super::*;

    #[test]
    fn document_palette_xml_reads_names_and_tints() {
        let xml = r#"<?xml version="1.0"?><palette guid="x" name="Document Palette"><colors><page><color cs="CMYK" name="Green" tints="1,0,1,0"/><color cs="CMYK" name="Mint Green" tints="0.4,0,0.4,0"/><color cs="RGB" name="R &amp; B" tints="1,0,0.5"/><color cs="SPOT" name="Ink" tints="1"/><color cs="CMYK" name="Short" tints="1,0"/><color cs="CMYK" name="Black" tints="0,0,0,1"/></page></colors></palette>"#;
        let p = parse_palette_xml(xml);
        let names: Vec<&str> = p.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["Green", "Mint Green", "R & B", "Black"]);
        assert_eq!(
            p[1].1,
            Color::Cmyk {
                c: 0.4,
                m: 0.0,
                y: 0.4,
                k: 0.0
            }
        );
        assert_eq!(
            p[2].1,
            Color::Rgb {
                r: 1.0,
                g: 0.0,
                b: 0.5
            }
        );
    }

    #[test]
    fn document_palette_reads_the_other_colour_models() {
        let xml = r#"<colors><color cs="CMY" name="C60 M169 Y255" tints="0.235294,0.662745,1"/><color cs="HSB" name="H119 S80 B190" tints="0.330556,0.313725,0.745098"/><color cs="HLS" name="H295 L165 S69" tints="0.819444,0.647059,0.270588"/><color cs="LAB" name="L76 a-14 b66" tints="0.764706,0.447059,0.760784"/><color cs="YIQ" name="Y100 I69 Q89" tints="0.392157,0.270588,0.34902"/><color cs="Gray" name="Grey" tints="0.639216"/><color cs="Registration" name="Reg" tints="1,1,1,1"/><color cs="PANTONE 448 C" name="Spot" tints="1"/></colors>"#;
        let p = parse_palette_xml(xml);
        assert_eq!(p.len(), 7, "{p:?}");
        let close = |a: f32, b: f32| (a - b).abs() < 0.01;
        match p[0].1 {
            Color::Rgb { r, g, b } => assert!(close(r, 0.765) && close(g, 0.337) && close(b, 0.0)),
            other => panic!("{other:?}"),
        }
        match p[1].1 {
            Color::Hsb { h, s, b } => {
                assert!(close(h, 119.0) && close(s, 0.3137) && close(b, 0.745))
            }
            other => panic!("{other:?}"),
        }
        match p[2].1 {
            Color::Hsl { h, s, l } => {
                assert!(close(h, 295.0) && close(s, 0.2706) && close(l, 0.647))
            }
            other => panic!("{other:?}"),
        }
        match p[3].1 {
            Color::Lab { l, a, b } => assert!(close(l, 76.47) && close(a, -14.0) && close(b, 66.0)),
            other => panic!("{other:?}"),
        }
        assert!(matches!(p[4].1, Color::Yiq { .. }));
        assert!(matches!(p[5].1, Color::Gray { .. }));
        assert_eq!(p[6].1, Color::Registration);
    }

    #[test]
    fn broken_palette_xml_yields_what_it_can() {
        assert!(parse_palette_xml("").is_empty());
        assert!(parse_palette_xml("<color cs=\"CMYK\" tints=\"a,b,c,d\"").is_empty());
        let p = parse_palette_xml("<color tints=\"9,-1,0.5,0\" cs=\"cmyk\">");
        assert_eq!(
            p[0].1,
            Color::Cmyk {
                c: 1.0,
                m: 0.0,
                y: 0.5,
                k: 0.0
            }
        );
    }

    #[test]
    fn version_letters() {
        assert_eq!(Version::from_form_type(b"CDR9"), Some(Version(9)));
        assert_eq!(Version::from_form_type(b"CDR "), Some(Version(3)));
        assert_eq!(Version::from_form_type(b"CDRD"), Some(Version(13)));
        assert_eq!(Version::from_form_type(b"CDRH"), Some(Version(17)));
        // `I` is not used; `J` is version 18 and `K` is 19.
        assert_eq!(Version::from_form_type(b"CDRI"), None);
        assert_eq!(Version::from_form_type(b"CDRJ"), Some(Version(18)));
        assert_eq!(Version::from_form_type(b"CDRN"), Some(Version(22)));
        // Files saved in each version from 23 to 27 (and two updates).
        assert_eq!(Version::from_form_type(b"CDRO"), Some(Version(23)));
        assert_eq!(Version::from_form_type(b"CDRP"), Some(Version(24)));
        assert_eq!(Version::from_form_type(b"CDRQ"), Some(Version(24)));
        assert_eq!(Version::from_form_type(b"CDRR"), Some(Version(25)));
        assert_eq!(Version::from_form_type(b"CDRS"), Some(Version(25)));
        assert_eq!(Version::from_form_type(b"CDRT"), Some(Version(26)));
        assert_eq!(Version::from_form_type(b"CDRU"), Some(Version(27)));
        assert_eq!(
            Version::from_form_type(b"CDRK").map(|v| v.name()),
            Some("CDR version 19".to_string())
        );
        assert_eq!(Version::from_form_type(b"cdr8"), Some(Version(8)));
        assert_eq!(Version(13).name(), "CDR version 13");
        assert_eq!(Version::from_form_type(b"WAVE"), None);
        // Templates.
        assert_eq!(Version::from_form_type(b"CDTQ"), Some(Version(21)));
        assert_eq!(Version::from_form_type(b"CDT1"), None);
    }

    #[test]
    fn detects_riff() {
        let mut b = b"RIFF".to_vec();
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(b"CDRC");
        let c = detect(&b).unwrap();
        assert_eq!(c.kind, ContainerKind::Riff);
        assert_eq!(c.version, Version(12));
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(detect(b"hello world!"), Err(Error::NotCdr)));
    }

    /// The bundled interface font (SIL Open Font License).
    const FONT: &[u8] = include_bytes!("../../../apps/tracedraw/assets/fonts/Selawik-Regular.ttf");

    /// A ZIP file around a written RIFF stream, with extra members.
    fn zip_file(members: &[(&str, Vec<u8>)]) -> Vec<u8> {
        use std::io::Write;
        // A bare RIFF stream, as versions before 14 write it.
        let riff = crate::write::document_to_cdr_version(&tracedraw_core::Document::default(), 12);
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            w.start_file("content/riffData.cdr", opts).expect("member");
            w.write_all(&riff).expect("write");
            for (name, data) in members {
                w.start_file(*name, opts).expect("member");
                w.write_all(data).expect("write");
            }
            w.finish().expect("zip");
        }
        buf.into_inner()
    }

    /// An `embed/` member: a header of its own, then the EOT structure.
    fn embedding(font: &[u8], family: &str) -> Vec<u8> {
        let mut m = vec![1, 0, 0, 0, 3, 8, 0, 0];
        m.extend("Version 1.0".encode_utf16().flat_map(|u| u.to_le_bytes()));
        m.extend_from_slice(&[0x5f; 300]);
        m.extend(crate::eot::wrap(font, family, 0x1000_0000));
        m
    }

    #[test]
    fn the_cmyk_profile_comes_with_the_document() {
        use tracedraw_core::icc::test_profiles as tp;
        let cmyk = tp::lut16_cmyk_profile();
        let rgb = tp::matrix_trc_profile(tp::para_srgb(), tp::para_srgb(), tp::para_srgb());
        // An RGB profile in the CMYK folder and a damaged one are passed
        // over; the CMYK profile is kept, named by its description.
        let bytes = zip_file(&[
            ("color/color.xml", b"<Color/>".to_vec()),
            (
                "color/profiles/cmyk/a damaged.icc",
                b"not a profile".to_vec(),
            ),
            ("color/profiles/cmyk/b rgb.icc", rgb),
            ("color/profiles/cmyk/c press.icc", cmyk.clone()),
        ]);
        let (doc, _) = crate::open_bytes(&bytes, "colour").expect("opens");
        let p = doc.cmyk_profile.expect("profile");
        assert_eq!(p.name, "Synthetic CMYK");
        assert_eq!(p.data.as_slice(), cmyk.as_slice());
        // None in a file without one, nor in a RIFF file.
        let (doc, _) = crate::open_bytes(&zip_file(&[]), "plain").expect("opens");
        assert!(doc.cmyk_profile.is_none());
        // A bare RIFF stream, as versions before 14 write it.
        let riff = crate::write::document_to_cdr_version(&tracedraw_core::Document::default(), 12);
        assert!(crate::open_bytes(&riff, "riff")
            .expect("opens")
            .0
            .cmyk_profile
            .is_none());
    }

    #[test]
    fn embedded_fonts_come_with_the_document() {
        let bytes = zip_file(&[
            ("embed/embedding0", embedding(FONT, "Selawik")),
            // The same font again, and a member that holds none.
            ("embed/embedding1", embedding(FONT, "Selawik")),
            ("embed/embedding2", b"nothing to see".to_vec()),
            // Pictures and a sample of a font, kept next to it.
            ("embed/embedding0.png", b"\x89PNG".to_vec()),
            ("embed/embedding0-Extended.png", b"\x89PNG".to_vec()),
            ("embed/embedding0.sample.curve", b"curve".to_vec()),
        ]);
        let (doc, report) = crate::open_bytes(&bytes, "fonts").expect("opens");
        assert_eq!(doc.fonts.len(), 1);
        assert_eq!(doc.fonts[0].family, "Selawik");
        assert_eq!(doc.fonts[0].data.as_slice(), FONT);
        assert!(
            report.warnings.iter().any(|w| w.contains("embedding2")),
            "{:?}",
            report.warnings
        );
        assert!(
            !report
                .warnings
                .iter()
                .any(|w| w.contains(".png") || w.contains(".curve")),
            "{:?}",
            report.warnings
        );
        // Compressed fonts are named in the report and left out.
        let mut c = embedding(FONT, "Selawik");
        let at = crate::eot::find(&c).expect("eot");
        c[at + 12] |= 4;
        let (doc, report) =
            crate::open_bytes(&zip_file(&[("embed/embedding0", c)]), "c").expect("opens");
        assert!(doc.fonts.is_empty());
        assert!(report.warnings.iter().any(|w| w.contains("compressed")));
        // A RIFF file has no embedded members; the document still opens.
        // A bare RIFF stream, as versions before 14 write it.
        let riff = crate::write::document_to_cdr_version(&tracedraw_core::Document::default(), 12);
        let (doc, _) = crate::open_bytes(&riff, "riff").expect("opens");
        assert!(doc.fonts.is_empty());
    }

    #[test]
    fn cut_or_damaged_font_members_never_stop_the_file_opening() {
        let member = embedding(FONT, "Selawik");
        for cut in [0, 1, 10, 100, 400, member.len() / 2, member.len() - 1] {
            let bytes = zip_file(&[("embed/embedding0", member[..cut].to_vec())]);
            let (doc, _) = crate::open_bytes(&bytes, "cut").expect("opens");
            assert!(doc.fonts.is_empty(), "cut at {cut}");
        }
        let mut flipped = member.clone();
        for i in (0..flipped.len()).step_by(97) {
            flipped[i] ^= 0xa5;
        }
        let bytes = zip_file(&[("embed/embedding0", flipped)]);
        assert!(crate::open_bytes(&bytes, "flip").is_ok());
    }
}
