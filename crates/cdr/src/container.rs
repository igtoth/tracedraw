//! Container detection: bare RIFF, or ZIP wrapping a RIFF stream.

use crate::{Error, Result};
use std::io::Read;

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

#[cfg(test)]
mod tests {
    use super::*;

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
