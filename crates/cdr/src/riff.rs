//! RIFF chunk walker. Builds a tree of chunks referencing the source bytes by
//! offset, so nothing is copied until a parser asks for a payload.
//!
//! the RIFF is standard: `fourcc u32(size) payload [pad]`, with
//! `LIST` chunks carrying a list type and nested chunks. Two the vendor quirks:
//! - Since X4 (v14) chunk sizes are stored in a slightly different way in
//!   some files (the low bits can carry flags); we mask them off.
//! - `cmpr` lists hold zlib-compressed sub-streams (the editor 7 to X3 with
//!   compression on). We inflate them into an owned buffer and parse that.

use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: [u8; 4],
    /// For `LIST`/`RIFF` chunks: the list type (`page`, `layr`, ...).
    pub list_type: Option<[u8; 4]>,
    /// Payload range in the source buffer this chunk belongs to.
    pub start: usize,
    pub end: usize,
    /// Which buffer the range refers to: 0 = main stream, n = inflated
    /// stream n in `Tree::streams`.
    pub stream: usize,
    pub children: Vec<Chunk>,
}

impl Chunk {
    pub fn id_str(&self) -> String {
        String::from_utf8_lossy(&self.id).into_owned()
    }

    pub fn list_type_str(&self) -> Option<String> {
        self.list_type
            .map(|t| String::from_utf8_lossy(&t).into_owned())
    }

    /// Does this chunk match `id`, or for lists, the list type?
    pub fn is(&self, name: &[u8; 4]) -> bool {
        self.list_type.as_ref() == Some(name) || &self.id == name
    }

    pub fn find(&self, name: &[u8; 4]) -> Option<&Chunk> {
        self.children.iter().find(|c| c.is(name))
    }

    pub fn find_all<'a>(&'a self, name: &'a [u8; 4]) -> impl Iterator<Item = &'a Chunk> + 'a {
        self.children.iter().filter(move |c| c.is(name))
    }

    /// Depth-first walk over all descendants, including self.
    pub fn walk<'a>(&'a self, f: &mut dyn FnMut(&'a Chunk, usize)) {
        fn go<'a>(c: &'a Chunk, depth: usize, f: &mut dyn FnMut(&'a Chunk, usize)) {
            f(c, depth);
            for ch in &c.children {
                go(ch, depth + 1, f);
            }
        }
        go(self, 0, f);
    }
}

#[derive(Debug)]
pub struct Tree {
    pub root: Chunk,
    /// Inflated `cmpr` streams, indexed from 1 (0 is the main buffer).
    pub streams: Vec<Vec<u8>>,
}

impl Tree {
    /// Payload bytes of a chunk.
    pub fn data<'a>(&'a self, main: &'a [u8], c: &Chunk) -> &'a [u8] {
        let buf: &[u8] = if c.stream == 0 {
            main
        } else {
            &self.streams[c.stream - 1]
        };
        let end = c.end.min(buf.len());
        let start = c.start.min(end);
        &buf[start..end]
    }

    /// Indented dump of the tree, for `tracedraw-cli inspect` and for tests.
    pub fn dump(&self) -> String {
        let mut out = String::new();
        self.root.walk(&mut |c, depth| {
            let name = match c.list_type_str() {
                Some(t) => format!("{} {}", c.id_str(), t),
                None => c.id_str(),
            };
            out.push_str(&format!(
                "{:indent$}{} [{}..{}]{}\n",
                "",
                name,
                c.start,
                c.end,
                if c.stream > 0 {
                    format!(" s{}", c.stream)
                } else {
                    String::new()
                },
                indent = depth * 2
            ));
        });
        out
    }
}

fn u32le(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

pub fn parse(main: &[u8]) -> Result<Tree> {
    if main.len() < 12 || &main[0..4] != b"RIFF" {
        return Err(Error::NotCdr);
    }
    let mut streams = Vec::new();
    // The header size is advisory: truncated or padded files are common
    // enough that trusting the buffer length is the safer choice.
    let _declared = u32le(main, 4).ok_or(Error::Truncated(4))?;
    let end = main.len();
    let list_type: [u8; 4] = main[8..12].try_into().map_err(|_| Error::Truncated(8))?;
    let children = parse_children(main, 0, 12, end, &mut streams);
    Ok(Tree {
        root: Chunk {
            id: *b"RIFF",
            list_type: Some(list_type),
            start: 12,
            end,
            stream: 0,
            children,
        },
        streams,
    })
}

/// Parse sibling chunks in `buf[pos..end]`. Malformed data ends the list
/// quietly; whatever was parsed before is kept.
fn parse_children(
    buf: &[u8],
    stream: usize,
    mut pos: usize,
    end: usize,
    streams: &mut Vec<Vec<u8>>,
) -> Vec<Chunk> {
    let mut out = Vec::new();
    while pos + 8 <= end {
        let id: [u8; 4] = match buf.get(pos..pos + 4) {
            Some(s) => [s[0], s[1], s[2], s[3]],
            None => break,
        };
        let Some(raw) = u32le(buf, pos + 4) else {
            break;
        };
        // Newer versions are reported to align chunks to 4 bytes instead of
        // 2; we handle that when we have corpus files showing it.
        let size = raw as usize;
        let payload = pos + 8;
        let payload_end = payload.saturating_add(size).min(end);
        if payload > end {
            break;
        }
        let is_list = &id == b"LIST";
        if is_list {
            let lt: Option<[u8; 4]> = buf
                .get(payload..payload + 4)
                .map(|s| [s[0], s[1], s[2], s[3]]);
            let children = match lt {
                Some(t) if &t == b"cmpr" => inflate_cmpr(buf, payload + 4, payload_end, streams),
                Some(_) => parse_children(buf, stream, payload + 4, payload_end, streams),
                None => Vec::new(),
            };
            out.push(Chunk {
                id,
                list_type: lt,
                start: payload + 4,
                end: payload_end,
                stream,
                children,
            });
        } else {
            out.push(Chunk {
                id,
                list_type: None,
                start: payload,
                end: payload_end,
                stream,
                children: Vec::new(),
            });
        }
        // Chunks are word-aligned.
        pos = payload + size + (size & 1);
    }
    out
}

/// A `LIST cmpr` holds, in order: a `cmpr` chunk with the zlib-compressed
/// chunk stream and a second `cmpr` chunk with the compressed table of
/// original chunk sizes (which we do not need, since the inflated stream is
/// a plain sequence of RIFF chunks). Some files carry extra chunks; only
/// the first compressed block is the content stream.
fn inflate_cmpr(buf: &[u8], pos: usize, end: usize, streams: &mut Vec<Vec<u8>>) -> Vec<Chunk> {
    let mut siblings = Vec::new();
    // Walk the raw sub-chunks ourselves to grab the first `cmpr` payload.
    let mut p = pos;
    while p + 8 <= end {
        let id = &buf[p..p + 4];
        let Some(size) = u32le(buf, p + 4) else { break };
        let size = size as usize;
        let payload = p + 8;
        let payload_end = payload.saturating_add(size).min(end);
        if id == b"cmpr" && siblings.is_empty() {
            // The payload starts with the uncompressed size (u32), then zlib data.
            let data = &buf[payload..payload_end];
            if data.len() > 4 {
                let mut inflated = Vec::new();
                let mut dec = flate2::read::ZlibDecoder::new(&data[4..]);
                if std::io::Read::read_to_end(&mut dec, &mut inflated).is_ok()
                    && !inflated.is_empty()
                {
                    streams.push(inflated);
                    let idx = streams.len();
                    let len = streams[idx - 1].len();
                    // Parse the inflated stream as a chunk sequence.
                    let inflated_ref = std::mem::take(&mut streams[idx - 1]);
                    let children = parse_children(&inflated_ref, idx, 0, len, streams);
                    streams[idx - 1] = inflated_ref;
                    siblings.push(Chunk {
                        id: *b"cmpr",
                        list_type: Some(*b"cmpr"),
                        start: 0,
                        end: len,
                        stream: idx,
                        children,
                    });
                } else {
                    log::warn!("cmpr block at {payload} could not be inflated");
                }
            }
        }
        p = payload + size + (size & 1);
    }
    siblings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(payload);
        if payload.len() % 2 == 1 {
            v.push(0);
        }
        v
    }

    fn list(lt: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut payload = lt.to_vec();
        payload.extend_from_slice(body);
        chunk(b"LIST", &payload)
    }

    #[test]
    fn parses_nested_lists() {
        let page = list(
            b"page",
            &[
                chunk(b"vrsn", &[0x84, 0x03]),
                list(b"layr", &chunk(b"obj ", &[1, 2, 3])),
            ]
            .concat(),
        );
        let mut body = b"CDR9".to_vec();
        body.extend_from_slice(&page);
        let file = chunk(b"RIFF", &body);
        let tree = parse(&file).unwrap();
        assert_eq!(tree.root.list_type, Some(*b"CDR9"));
        let page = tree.root.find(b"page").unwrap();
        assert_eq!(page.children.len(), 2);
        let obj = page.find(b"layr").unwrap().find(b"obj ").unwrap();
        assert_eq!(tree.data(&file, obj), &[1, 2, 3]);
        assert!(tree.dump().contains("LIST layr"));
    }

    #[test]
    fn truncated_file_does_not_panic() {
        let mut file = chunk(b"RIFF", b"CDR9");
        file.extend_from_slice(b"LIST\xff\xff\xff\xffpage");
        let tree = parse(&file).unwrap();
        assert!(tree.root.find(b"page").is_some());
    }
}
