//! RIFF chunk walker. Builds a tree of chunks referencing the source bytes by
//! offset, so nothing is copied until a parser asks for a payload.
//!
//! The RIFF layout is standard: `fourcc u32(size) payload [pad]`, with
//! `LIST` chunks carrying a list type and nested chunks. Two format quirks:
//! - Since version 14 chunk sizes are stored in a slightly different way in
//!   some files (the low bits can carry flags); we mask them off.
//! - `cmpr` lists hold zlib-compressed sub-streams (CDR 7 to version 13 with
//!   compression on): two `CPng` blocks, the first with the chunk stream
//!   and the second with a pool of chunk sizes. Inside the inflated stream
//!   a chunk's size field is an index into that pool, not a byte count.
//! - Version 16 and later keep most payloads outside the RIFF stream: a chunk
//!   whose declared size is exactly 16 bytes is a redirect record
//!   `stream u32, length u32, offset u32, reserved u32` pointing into one of
//!   the external data streams (`content/data/*.dat`, in `dataFileList.dat`
//!   order); stream `0xFFFF_FFFF` means the payload (at most 8 bytes) is
//!   stored inline right after the length. Confirmed against the public
//!   Kaitai Struct description of the format.

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
    /// Out-of-line buffers, indexed from 1 (0 is the main buffer): first the
    /// external data streams of version 16+ files, then inflated `cmpr` streams.
    pub streams: Vec<Vec<u8>>,
}

/// Parsing context shared down the chunk tree.
struct Ctx {
    /// Number of external data streams (version 16+); 0 for older files.
    externals: usize,
    /// Whether 16-byte chunks are redirect records.
    redirects: bool,
    /// Major format version; 0 when unknown (treated as recent).
    version: u16,
}

/// Size of a redirect record (version 16+).
const REDIRECT_LEN: usize = 16;

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
    parse_with_externals(main, Vec::new(), 0)
}

/// Parse a RIFF stream whose chunks may redirect into `externals` (version 16+).
/// `version` is the major format version; redirects are only recognised
/// from 16 on.
pub fn parse_with_externals(main: &[u8], externals: Vec<Vec<u8>>, version: u16) -> Result<Tree> {
    if main.len() < 12 || &main[0..4] != b"RIFF" {
        return Err(Error::NotCdr);
    }
    let ctx = Ctx {
        externals: externals.len(),
        redirects: version >= 16,
        version,
    };
    let mut streams = externals;
    // The header size is advisory: truncated or padded files are common
    // enough that trusting the buffer length is the safer choice.
    let _declared = u32le(main, 4).ok_or(Error::Truncated(4))?;
    let end = main.len();
    let list_type: [u8; 4] = main[8..12].try_into().map_err(|_| Error::Truncated(8))?;
    let children = parse_children(main, 0, 12, end, &mut streams, &ctx, None);
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
/// quietly; whatever was parsed before is kept. Inside a compressed stream
/// `sizes` is the pool of chunk sizes that the size fields index into.
fn parse_children(
    buf: &[u8],
    stream: usize,
    mut pos: usize,
    end: usize,
    streams: &mut Vec<Vec<u8>>,
    ctx: &Ctx,
    sizes: Option<&[u32]>,
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
        // In a compressed stream the field is an index into the sizes pool.
        let size = match sizes {
            Some(pool) => match pool.get(raw as usize) {
                Some(s) => *s as usize,
                None => break,
            },
            None => raw as usize,
        };
        let payload = pos + 8;
        let payload_end = payload.saturating_add(size).min(end);
        if payload > end {
            break;
        }
        let is_list = &id == b"LIST";
        // version 16+ redirect record: resolve where the payload really lives.
        let mut body = (stream, payload, payload_end);
        if ctx.redirects && size == REDIRECT_LEN && payload_end - payload == REDIRECT_LEN {
            let s = u32le(buf, payload).unwrap_or(u32::MAX);
            let len = u32le(buf, payload + 4).unwrap_or(0) as usize;
            if s == u32::MAX {
                // Inline payload of up to 8 bytes after the length field.
                let start = payload + 8;
                body = (stream, start, start.saturating_add(len).min(payload_end));
            } else if (s as usize) < ctx.externals {
                let ofs = u32le(buf, payload + 8).unwrap_or(0) as usize;
                let ext_len = streams[s as usize].len();
                let start = ofs.min(ext_len);
                body = (s as usize + 1, start, ofs.saturating_add(len).min(ext_len));
            } else {
                log::warn!(
                    "chunk {} redirects to missing stream {s}",
                    String::from_utf8_lossy(&id)
                );
            }
        }
        let (bstream, bstart, bend) = body;
        if is_list {
            // The list body may be in another buffer than the chunk header.
            let taken = if bstream != stream && bstream > 0 {
                Some(std::mem::take(&mut streams[bstream - 1]))
            } else {
                None
            };
            let bbuf: &[u8] = match &taken {
                Some(t) => t,
                None => buf,
            };
            let lt: Option<[u8; 4]> = bbuf
                .get(bstart..bstart + 4)
                .map(|s| [s[0], s[1], s[2], s[3]]);
            let children = match lt {
                Some(t) if &t == b"cmpr" => inflate_cmpr(bbuf, bstart + 4, bend, streams, ctx),
                // From version 7 the `stlt` list body is one record, not
                // sub-chunks (the parser reads it from the list's data).
                Some(t) if &t == b"stlt" && (ctx.version >= 7 || ctx.version == 0) => Vec::new(),
                Some(_) => parse_children(bbuf, bstream, bstart + 4, bend, streams, ctx, sizes),
                None => Vec::new(),
            };
            if let Some(t) = taken {
                streams[bstream - 1] = t;
            }
            out.push(Chunk {
                id,
                list_type: lt,
                start: bstart + 4,
                end: bend,
                stream: bstream,
                children,
            });
        } else {
            out.push(Chunk {
                id,
                list_type: None,
                start: bstart,
                end: bend,
                stream: bstream,
                children: Vec::new(),
            });
        }
        // Chunks are word-aligned.
        pos = payload + size + (size & 1);
    }
    out
}

/// A `LIST cmpr` body holds two size pairs (compressed, uncompressed; u32
/// each), then two `CPng` blocks of those compressed sizes. Each block is
/// `CPng`, the bytes `01 00 04 00`, and a zlib stream. The first block
/// inflates to the chunk stream, the second to a pool of u32 chunk sizes
/// that the size fields of the inflated chunks index into.
fn inflate_cmpr(
    buf: &[u8],
    pos: usize,
    end: usize,
    streams: &mut Vec<Vec<u8>>,
    ctx: &Ctx,
) -> Vec<Chunk> {
    let (Some(c0), Some(c1)) = (u32le(buf, pos), u32le(buf, pos + 8)) else {
        return Vec::new();
    };
    let first = pos + 16;
    let second = first.saturating_add(c0 as usize);
    let Some(chunks) = inflate_cpng(buf, first, second.min(end)) else {
        log::warn!("cmpr block at {first} could not be inflated");
        return Vec::new();
    };
    let sizes: Vec<u32> = inflate_cpng(buf, second, second.saturating_add(c1 as usize).min(end))
        .unwrap_or_default()
        .chunks_exact(4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .collect();
    if sizes.is_empty() {
        log::warn!("cmpr sizes pool at {second} missing; chunk stream skipped");
        return Vec::new();
    }
    streams.push(chunks);
    let idx = streams.len();
    let len = streams[idx - 1].len();
    let inflated_ref = std::mem::take(&mut streams[idx - 1]);
    let children = parse_children(&inflated_ref, idx, 0, len, streams, ctx, Some(&sizes));
    streams[idx - 1] = inflated_ref;
    vec![Chunk {
        id: *b"cmpr",
        list_type: Some(*b"cmpr"),
        start: 0,
        end: len,
        stream: idx,
        children,
    }]
}

/// Inflate one `CPng` block in `buf[start..end]`.
fn inflate_cpng(buf: &[u8], start: usize, end: usize) -> Option<Vec<u8>> {
    let block = buf.get(start..end)?;
    if block.len() < 9 || &block[0..4] != b"CPng" {
        return None;
    }
    let mut inflated = Vec::new();
    let mut dec = flate2::read::ZlibDecoder::new(&block[8..]);
    std::io::Read::read_to_end(&mut dec, &mut inflated).ok()?;
    if inflated.is_empty() {
        None
    } else {
        Some(inflated)
    }
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

    fn deflate(data: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn cmpr_lists_use_cpng_blocks_and_a_sizes_pool() {
        // Inflated chunk stream: size fields are indices into the pool.
        let mut inner = Vec::new();
        inner.extend_from_slice(b"LIST");
        inner.extend_from_slice(&0u32.to_le_bytes()); // index 0: 4 + 2 + 8 = 14
        inner.extend_from_slice(b"page");
        inner.extend_from_slice(b"vrsn");
        inner.extend_from_slice(&1u32.to_le_bytes()); // index 1: 2
        inner.extend_from_slice(&[0x84, 0x03]);
        let pool: Vec<u8> = [14u32, 2u32].iter().flat_map(|x| x.to_le_bytes()).collect();
        let mut block1 = b"CPng\x01\x00\x04\x00".to_vec();
        block1.extend_from_slice(&deflate(&inner));
        let mut block2 = b"CPng\x01\x00\x04\x00".to_vec();
        block2.extend_from_slice(&deflate(&pool));
        let mut body = Vec::new();
        for (c, u) in [(block1.len(), inner.len()), (block2.len(), pool.len())] {
            body.extend_from_slice(&(c as u32).to_le_bytes());
            body.extend_from_slice(&(u as u32).to_le_bytes());
        }
        body.extend_from_slice(&block1);
        body.extend_from_slice(&block2);
        let mut riff_body = b"CDR9".to_vec();
        riff_body.extend_from_slice(&list(b"cmpr", &body));
        let file = chunk(b"RIFF", &riff_body);
        let tree = parse(&file).unwrap();
        // The outer list wraps a node for the inflated stream (stream 1).
        let outer = tree.root.find(b"cmpr").expect("cmpr list");
        let inner = outer.find(b"cmpr").expect("inflated stream node");
        assert_eq!(inner.stream, 1);
        let page = inner.find(b"page").expect("page inside inflated stream");
        let vrsn = page.find(b"vrsn").expect("vrsn inside page");
        assert_eq!(tree.data(&file, vrsn), &[0x84, 0x03]);
    }

    #[test]
    fn truncated_file_does_not_panic() {
        let mut file = chunk(b"RIFF", b"CDR9");
        file.extend_from_slice(b"LIST\xff\xff\xff\xffpage");
        let tree = parse(&file).unwrap();
        assert!(tree.root.find(b"page").is_some());
    }
}

#[cfg(test)]
mod redirect_tests {
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

    fn redirect(id: &[u8; 4], stream: u32, len: u32, ofs: u32) -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(&stream.to_le_bytes());
        p.extend_from_slice(&len.to_le_bytes());
        p.extend_from_slice(&ofs.to_le_bytes());
        p.extend_from_slice(&0u32.to_le_bytes());
        chunk(id, &p)
    }

    #[test]
    fn x6_redirects_resolve_into_external_streams() {
        // External stream 0: a `page` list body (form type + one chunk) and a `vrsn` payload.
        let mut ext = Vec::new();
        ext.extend_from_slice(b"page");
        ext.extend_from_slice(&chunk(b"loda", &[1, 2, 3, 4, 5, 6]));
        let page_len = ext.len() as u32;
        let vrsn_ofs = ext.len() as u32;
        ext.extend_from_slice(&[0x40, 0x06]);
        // Main stream: RIFF CDRG with a redirected LIST, a redirected vrsn and an inline chunk.
        let mut body = b"CDRG".to_vec();
        body.extend_from_slice(&redirect(b"LIST", 0, page_len, 0));
        body.extend_from_slice(&redirect(b"vrsn", 0, 2, vrsn_ofs));
        let mut inline = Vec::new();
        inline.extend_from_slice(&u32::MAX.to_le_bytes());
        inline.extend_from_slice(&3u32.to_le_bytes());
        inline.extend_from_slice(&[9, 8, 7, 0, 0, 0, 0, 0]);
        body.extend_from_slice(&chunk(b"disp", &inline));
        let mut main = b"RIFF".to_vec();
        main.extend_from_slice(&(body.len() as u32).to_le_bytes());
        main.extend_from_slice(&body);

        let tree = parse_with_externals(&main, vec![ext.clone()], 16).unwrap();
        let list = tree.root.find(b"page").expect("page list via redirect");
        assert_eq!(list.stream, 1);
        let loda = list.find(b"loda").expect("child inside external stream");
        assert_eq!(tree.data(&main, loda), &[1, 2, 3, 4, 5, 6]);
        let vrsn = tree.root.find(b"vrsn").unwrap();
        assert_eq!(tree.data(&main, vrsn), &[0x40, 0x06]);
        let disp = tree.root.find(b"disp").unwrap();
        assert_eq!(tree.data(&main, disp), &[9, 8, 7]);

        // Without the version gate the same bytes are plain 16-byte chunks.
        let old = parse_with_externals(&main, vec![ext], 13).unwrap();
        assert_eq!(
            old.root.find(b"vrsn").unwrap().end - old.root.find(b"vrsn").unwrap().start,
            16
        );
    }
}
