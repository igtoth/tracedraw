//! Container detection: bare RIFF, or ZIP wrapping a RIFF stream.

use crate::{Error, Result};
use std::io::Read;
use tracedraw_core::Color;

/// CDR major version, as encoded in the RIFF form type (`CDR9`,
/// `CDRA` = 10, ... `CDRE` = X4 (14), `CDRH` = X7 (17), `CDRJ` = X8 (18),
/// `CDRK` = 2017 (19), and so on; the letter `I` is not used).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u16);

impl Version {
    pub fn from_form_type(form: &[u8; 4]) -> Option<Version> {
        if &form[..3] != b"CDR" && &form[..3] != b"cdr" {
            return None;
        }
        let c = form[3];
        // Digits are versions 1 to 9; letters continue from `A` = 10, but
        // `I` is skipped: `H` = X7 (17), `J` = X8 (18), `K` = 2017 (19).
        match c {
            b'1'..=b'9' => Some(Version((c - b'0') as u16)),
            b'A'..=b'H' => Some(Version(10 + (c - b'A') as u16)),
            b'J'..=b'Z' => Some(Version(9 + (c - b'A') as u16)),
            _ => None,
        }
    }

    /// Human name: 9 → "CDR 9", 13 → "X3", 17 → "X7", 19 → "2017".
    pub fn name(self) -> String {
        match self.0 {
            v @ 1..=12 => format!("CDR {v}"),
            v @ 13..=18 => format!("CDR X{}", v - 10),
            v @ 19.. => format!("CDR {}", 1998 + v as u32),
            _ => "CDR (unknown)".into(),
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
    /// ZIP archive; the RIFF stream is the named member. X6 and later
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
        // X6+: the order of external data streams.
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
            let mut f = archive.by_name(riff_member)?;
            let mut out = Vec::with_capacity(f.size() as usize);
            f.read_to_end(&mut out)?;
            Ok(out)
        }
    }
}

/// The external data streams of an X6+ file, in `dataFileList.dat` order.
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
        if let Ok(mut f) = archive.by_name(&c) {
            let mut out = Vec::new();
            if f.read_to_end(&mut out).is_ok() {
                return Some(out);
            }
        }
    }
    None
}

/// The document palette of an X4+ ZIP file (`color/docPalette.xml`):
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
        if let Ok(mut f) = archive.by_name(name) {
            let mut raw = Vec::new();
            if f.read_to_end(&mut raw).is_ok() {
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
        assert_eq!(Version::from_form_type(b"CDRD"), Some(Version(13)));
        assert_eq!(Version::from_form_type(b"CDRH"), Some(Version(17)));
        // `I` is not used; `J` is X8 and `K` is 2017.
        assert_eq!(Version::from_form_type(b"CDRI"), None);
        assert_eq!(Version::from_form_type(b"CDRJ"), Some(Version(18)));
        assert_eq!(
            Version::from_form_type(b"CDRK").map(|v| v.name()),
            Some("CDR 2017".to_string())
        );
        assert_eq!(Version::from_form_type(b"cdr8"), Some(Version(8)));
        assert_eq!(Version(13).name(), "CDR X3");
        assert_eq!(Version::from_form_type(b"WAVE"), None);
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
}
