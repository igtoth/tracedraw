//! Container detection: bare RIFF, or ZIP wrapping a RIFF stream.

use crate::{Error, Result};
use std::io::Read;

/// the editor major version, as encoded in the RIFF form type (`CDR9`,
/// `CDRA` = 10, ... `CDRE` = X4 (14), `CDRH` = X7 (17), and so on).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u16);

impl Version {
    pub fn from_form_type(form: &[u8; 4]) -> Option<Version> {
        if &form[..3] != b"CDR" && &form[..3] != b"cdr" {
            return None;
        }
        let c = form[3];
        match c {
            b'1'..=b'9' => Some(Version((c - b'0') as u16)),
            b'A'..=b'Z' => Some(Version(10 + (c - b'A') as u16)),
            b'a'..=b'z' => Some(Version(10 + (c - b'a') as u16)),
            _ => None,
        }
    }

    /// Human name: 9 → "the editor 9", 13 → "X3", 17 → "X7", 19 → "2017".
    pub fn name(self) -> String {
        match self.0 {
            v @ 1..=12 => format!("the editor {v}"),
            v @ 13..=18 => format!("the editor X{}", v - 10),
            v @ 19.. => format!("the editor {}", 1998 + v as u32),
            _ => "the editor (unknown)".into(),
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
    /// ZIP archive; the RIFF stream is the named member.
    Zip {
        riff_member: String,
    },
}

const RIFF_MEMBERS: &[&str] = &[
    "content/riffData.cdr",
    "content/riffdata.cdr",
    "riffData.cdr",
];

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
        let mut f = archive.by_name(&member)?;
        f.read_exact(&mut head).map_err(|_| Error::Truncated(0))?;
        if &head[0..4] != b"RIFF" {
            return Err(Error::UnsupportedContainer(
                "riffData.cdr is not RIFF".into(),
            ));
        }
        let form: [u8; 4] = head[8..12].try_into().map_err(|_| Error::NotCdr)?;
        let version = Version::from_form_type(&form).ok_or(Error::NotCdr)?;
        return Ok(Container {
            kind: ContainerKind::Zip {
                riff_member: member,
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
        ContainerKind::Zip { riff_member } => {
            let cursor = std::io::Cursor::new(bytes);
            let mut archive = zip::ZipArchive::new(cursor)?;
            let mut f = archive.by_name(riff_member)?;
            let mut out = Vec::with_capacity(f.size() as usize);
            f.read_to_end(&mut out)?;
            Ok(out)
        }
    }
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
        assert_eq!(
            Version::from_form_type(b"CDRJ").map(|v| v.name()),
            Some("the editor 2017".to_string())
        );
        assert_eq!(Version(13).name(), "the editor X3");
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
