//! Container detection: bare RIFF, or ZIP wrapping a RIFF stream.

use crate::{Error, Result};
use std::io::Read;

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

/// A member of the ZIP package by its exact name, read up to `limit`
/// bytes; `None` for RIFF files, packages without it and members past
/// the limit.
pub fn own_member(bytes: &[u8], container: &Container, name: &str, limit: u64) -> Option<Vec<u8>> {
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

/// The most a member of the package (the RIFF stream, a data stream) or
/// one compressed block may inflate to: a file that claims more is a
/// decompression bomb or damaged, and is read only up to here.
pub const MAX_STREAM: u64 = 512 << 20;

/// The most an external data member may inflate to when the archive
/// declares it that large (the bitmaps of a big drawing).
pub const MAX_DATA_STREAM: u64 = 3 << 30;

/// How far a data member declared `declared` bytes long may inflate:
/// [`MAX_STREAM`] at least, its declared size up to [`MAX_DATA_STREAM`].
pub fn member_limit(declared: u64) -> u64 {
    declared.clamp(MAX_STREAM, MAX_DATA_STREAM)
}

/// Whether a member read to `len` bytes under `limit` was cut off: it
/// filled the limit while claiming another size (a bomb inflating past
/// what it declares, or a member larger than the ceiling).
pub fn cut_short(len: u64, limit: u64, declared: u64) -> bool {
    len >= limit && declared != limit
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
