//! What a reader leaves uninterpreted, kept as bytes: the records a
//! file holds that no reader understands yet (`docs/cdr-format.md`,
//! "Unknown records").
//!
//! A [`RawTable`] is built by [`collect`] from a chunk tree and the set of
//! chunk tags and `loda` argument types a reader interprets. It keeps
//! every other leaf chunk and every other argument as a reference into
//! the buffers the tree was read from (the RIFF stream, the external
//! data streams, the inflated compressed lists), which it shares with the
//! tree: nothing is copied, so a huge file costs the table only its
//! index. Records are keyed by the object they belong to (the id in the
//! object's `usdn` chunk) or by the document when they sit outside every
//! numbered object, and kept in file order. A writer can put them back
//! where they were: each record knows its list path, its position among
//! its list's children and, for an argument, its position in the table.

use crate::loda;
use crate::riff::{Chunk, Tree};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

/// A buffer shared between a chunk tree and the tables built from it.
pub type SharedBytes = Arc<Vec<u8>>;

/// Who a record belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Owner {
    /// Outside every numbered object: the document, a page or a layer.
    Document,
    /// The list (an `obj `, a `grp `, a page or a layer) whose `usdn`
    /// chunk holds this id.
    Object(u32),
}

/// What a record is: a whole chunk, or one argument of a `loda` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    Chunk([u8; 4]),
    Arg(u32),
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Chunk(id) => write!(f, "chunk {}", String::from_utf8_lossy(id)),
            Key::Arg(ty) => write!(f, "argument {ty}"),
        }
    }
}

/// One record kept as bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawRecord {
    pub key: Key,
    /// The lists from the root down to the one holding the chunk, as an
    /// index into [`RawTable::paths`].
    pub path: u32,
    /// The chunk's position among its list's children.
    pub index: u32,
    /// For an argument: its position in the `loda` table (0 for a chunk).
    pub slot: u32,
    /// The bytes: a buffer of [`RawTable::buffers`] and a range in it.
    pub buffer: u32,
    pub start: u32,
    pub end: u32,
}

/// How many records a key has and how many bytes they hold.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Count {
    pub records: usize,
    pub bytes: u64,
}

/// The tags and argument types left uninterpreted, with their counts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnknownTags {
    pub entries: BTreeMap<Key, Count>,
}

impl UnknownTags {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Records over every key.
    pub fn records(&self) -> usize {
        self.entries.values().map(|c| c.records).sum()
    }

    /// One line a key, the most frequent first: "chunk obbx: 120 record(s),
    /// 3840 bytes".
    pub fn lines(&self) -> Vec<String> {
        let mut all: Vec<(&Key, &Count)> = self.entries.iter().collect();
        all.sort_by(|a, b| b.1.records.cmp(&a.1.records).then(a.0.cmp(b.0)));
        all.into_iter()
            .map(|(k, c)| format!("{k}: {} record(s), {} bytes", c.records, c.bytes))
            .collect()
    }
}

/// The uninterpreted records of one file.
#[derive(Debug, Clone, Default)]
pub struct RawTable {
    /// The buffers the records point into: the main stream first, then
    /// the tree's out-of-line streams in their order.
    pub buffers: Vec<SharedBytes>,
    /// The list paths records refer to, each the list types from the
    /// root down (`["CDRU", "page", "layr", "obj "]`).
    pub paths: Vec<Vec<[u8; 4]>>,
    /// The records of each owner, in file order.
    pub records: BTreeMap<Owner, Vec<RawRecord>>,
}

impl RawTable {
    /// The bytes of a record (empty when the record points past its
    /// buffer, which a table built here never does).
    pub fn bytes(&self, r: &RawRecord) -> &[u8] {
        let Some(buf) = self.buffers.get(r.buffer as usize) else {
            return &[];
        };
        let end = (r.end as usize).min(buf.len());
        let start = (r.start as usize).min(end);
        &buf[start..end]
    }

    /// The list path of a record, as text: "CDRU/page/layr/obj ".
    pub fn path_of(&self, r: &RawRecord) -> String {
        self.paths
            .get(r.path as usize)
            .map(|p| {
                p.iter()
                    .map(|t| String::from_utf8_lossy(t).into_owned())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .unwrap_or_default()
    }

    /// Records over every owner.
    pub fn len(&self) -> usize {
        self.records.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Every record with its owner, in file order.
    pub fn iter(&self) -> impl Iterator<Item = (Owner, &RawRecord)> {
        self.records
            .iter()
            .flat_map(|(o, rs)| rs.iter().map(move |r| (*o, r)))
    }

    /// The tags and argument types kept, with their counts.
    pub fn summary(&self) -> UnknownTags {
        let mut tags = UnknownTags::default();
        for (_, r) in self.iter() {
            let c = tags.entries.entry(r.key).or_default();
            c.records += 1;
            c.bytes += u64::from(r.end.saturating_sub(r.start));
        }
        tags
    }
}

/// What a reader interprets: the chunk tags it reads anywhere in the
/// tree and the `loda` argument types it reads. Everything else is kept.
#[derive(Debug, Clone, Copy)]
pub struct Known<'a> {
    pub chunks: &'a [[u8; 4]],
    pub args: &'a [u32],
}

/// Most records kept from one file; the rest is counted in the summary
/// but not kept (a file of that many unknown records is damaged).
const MAX_RECORDS: usize = 4_000_000;

/// The records of `tree` (read from `main`) that `known` leaves out,
/// for a file of `version`.
pub fn collect(tree: &Tree, main: SharedBytes, version: u16, known: &Known) -> RawTable {
    let mut buffers = Vec::with_capacity(tree.streams.len() + 1);
    buffers.push(main);
    buffers.extend(tree.streams.iter().cloned());
    let mut table = RawTable {
        buffers,
        paths: Vec::new(),
        records: BTreeMap::new(),
    };
    let mut c = Collector {
        table: &mut table,
        version,
        known,
        total: 0,
    };
    let root = [tree.root.list_type.unwrap_or(*b"RIFF")];
    let root_path = c.path(&root);
    c.list(&tree.root, Owner::Document, root_path, &root);
    table
}

struct Collector<'a> {
    table: &'a mut RawTable,
    version: u16,
    known: &'a Known<'a>,
    total: usize,
}

impl Collector<'_> {
    /// The index of `path` among the table's paths, added when new.
    fn path(&mut self, path: &[[u8; 4]]) -> u32 {
        if let Some(i) = self.table.paths.iter().position(|p| p == path) {
            return i as u32;
        }
        self.table.paths.push(path.to_vec());
        (self.table.paths.len() - 1) as u32
    }

    fn push(&mut self, owner: Owner, r: RawRecord) {
        if self.total >= MAX_RECORDS {
            return;
        }
        self.total += 1;
        self.table.records.entry(owner).or_default().push(r);
    }

    /// The id a list's `usdn` chunk holds: a u32, or a u16 in short
    /// records.
    fn usdn(&self, list: &Chunk) -> Option<u32> {
        let c = list.children.iter().find(|c| c.is(b"usdn"))?;
        let d = self.data(c);
        if let Some(s) = d.get(..4) {
            Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        } else {
            d.get(..2)
                .map(|s| u32::from(u16::from_le_bytes([s[0], s[1]])))
        }
    }

    fn data(&self, c: &Chunk) -> &[u8] {
        let Some(buf) = self.table.buffers.get(c.stream) else {
            return &[];
        };
        let end = c.end.min(buf.len());
        let start = c.start.min(end);
        &buf[start..end]
    }

    fn list(&mut self, list: &Chunk, owner: Owner, path: u32, types: &[[u8; 4]]) {
        for (index, c) in list.children.iter().enumerate() {
            let index = index as u32;
            if let Some(t) = c.list_type {
                let inner = self.usdn(c).map(Owner::Object).unwrap_or(owner);
                let mut below = types.to_vec();
                below.push(t);
                let p = self.path(&below);
                self.list(c, inner, p, &below);
            } else if c.is(b"loda") || c.is(b"lobj") {
                self.arguments(c, owner, path, index);
            } else if !self.known.chunks.contains(&c.id) {
                let r = record(Key::Chunk(c.id), path, index, 0, c, c.start, c.end);
                self.push(owner, r);
            }
        }
    }

    /// The arguments of a `loda` the reader does not interpret; the whole
    /// chunk when its table cannot be read.
    fn arguments(&mut self, c: &Chunk, owner: Owner, path: u32, index: u32) {
        let d = self.data(c);
        let Some(t) = loda::table(d, self.version) else {
            let r = record(Key::Chunk(c.id), path, index, 0, c, c.start, c.end);
            self.push(owner, r);
            return;
        };
        for (slot, a) in t.args.iter().enumerate() {
            if self.known.args.contains(&a.ty) {
                continue;
            }
            let r = record(
                Key::Arg(a.ty),
                path,
                index,
                slot as u32,
                c,
                c.start.saturating_add(a.off),
                c.start.saturating_add(a.end),
            );
            self.push(owner, r);
        }
    }
}

/// A record of `c`'s buffer from `start` to `end` (positions in that
/// buffer).
fn record(
    key: Key,
    path: u32,
    index: u32,
    slot: u32,
    c: &Chunk,
    start: usize,
    end: usize,
) -> RawRecord {
    let clamp = |v: usize| u32::try_from(v).unwrap_or(u32::MAX);
    RawRecord {
        key,
        path,
        index,
        slot,
        buffer: clamp(c.stream),
        start: clamp(start),
        end: clamp(end.max(start)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riff;

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

    /// A `loda` of object type 3 with arguments 20 (a fill id), 30 (six
    /// bytes) and 12010 (two bytes), as a version 13 file lays them out.
    fn loda_bytes() -> Vec<u8> {
        let args: [(u32, &[u8]); 3] = [
            (20, &[7, 0, 0, 0]),
            (30, &[1, 2, 3, 4, 5, 6]),
            (12010, &[8, 9]),
        ];
        let n = args.len() as u32;
        let offsets_at = 20;
        let types_at = offsets_at + (n + 1) * 4;
        let bodies_at = types_at + n * 4;
        let mut bodies = Vec::new();
        let mut offsets = Vec::new();
        for (_, b) in &args {
            offsets.push(bodies_at + bodies.len() as u32);
            bodies.extend_from_slice(b);
        }
        offsets.push(bodies_at + bodies.len() as u32);
        let mut v = Vec::new();
        for x in [bodies_at + bodies.len() as u32, n, offsets_at, types_at, 3] {
            v.extend_from_slice(&x.to_le_bytes());
        }
        for o in offsets {
            v.extend_from_slice(&o.to_le_bytes());
        }
        for (ty, _) in args.iter().rev() {
            v.extend_from_slice(&ty.to_le_bytes());
        }
        v.extend(bodies);
        v
    }

    fn file() -> Vec<u8> {
        file_with(&loda_bytes())
    }

    /// The file around `loda`'s payload.
    fn file_with(loda: &[u8]) -> Vec<u8> {
        let obj = list(
            b"obj ",
            &[
                chunk(b"usdn", &42u32.to_le_bytes()),
                chunk(b"loda", loda),
                chunk(b"trfd", &[0; 8]),
                chunk(b"obbx", &[1; 16]),
            ]
            .concat(),
        );
        let layer = list(b"layr", &[chunk(b"flgs", &[0, 0, 0, 0x98]), obj].concat());
        let page = list(b"page", &layer);
        let mut body = b"CDRD".to_vec();
        body.extend_from_slice(&chunk(b"vrsn", &[0x14, 0x05]));
        body.extend_from_slice(&chunk(b"sumi", &[3; 10]));
        body.extend_from_slice(&page);
        chunk(b"RIFF", &body)
    }

    const KNOWN: Known<'static> = Known {
        chunks: &[*b"vrsn", *b"usdn", *b"trfd", *b"flgs"],
        args: &[20, 30],
    };

    #[test]
    fn keeps_what_the_reader_leaves_by_owner_in_file_order() {
        let bytes = Arc::new(file());
        let tree = riff::parse(&bytes).expect("tree");
        let table = collect(&tree, bytes.clone(), 13, &KNOWN);
        assert_eq!(table.len(), 3);
        let doc = &table.records[&Owner::Document];
        assert_eq!(doc.len(), 1);
        assert_eq!(doc[0].key, Key::Chunk(*b"sumi"));
        assert_eq!(table.bytes(&doc[0]), &[3; 10]);
        assert_eq!(table.path_of(&doc[0]), "CDRD");
        let obj = &table.records[&Owner::Object(42)];
        let keys: Vec<Key> = obj.iter().map(|r| r.key).collect();
        assert_eq!(keys, [Key::Arg(12010), Key::Chunk(*b"obbx")]);
        assert_eq!(table.bytes(&obj[0]), &[8, 9]);
        assert_eq!((obj[0].index, obj[0].slot), (1, 2));
        assert_eq!(table.bytes(&obj[1]), &[1; 16]);
        assert_eq!(obj[1].index, 3);
        assert_eq!(table.path_of(&obj[1]), "CDRD/page/layr/obj ");
        // The buffers are shared, not copied.
        assert!(Arc::ptr_eq(&table.buffers[0], &bytes));
        let summary = table.summary();
        assert_eq!(summary.records(), 3);
        assert_eq!(
            summary.lines(),
            [
                "chunk obbx: 1 record(s), 16 bytes",
                "chunk sumi: 1 record(s), 10 bytes",
                "argument 12010: 1 record(s), 2 bytes"
            ]
        );
    }

    #[test]
    fn a_loda_that_does_not_read_is_kept_whole_and_damage_never_panics() {
        // A header cut short: no table, the chunk kept whole.
        let short = file_with(&loda_bytes()[..12]);
        let tree = riff::parse(&short).expect("tree");
        let table = collect(&tree, Arc::new(short.clone()), 13, &KNOWN);
        let obj = &table.records[&Owner::Object(42)];
        assert_eq!(obj.len(), 2);
        assert_eq!(obj[0].key, Key::Chunk(*b"loda"));
        assert_eq!(table.bytes(&obj[0]), &loda_bytes()[..12]);
        let bytes = file();
        for cut in 0..bytes.len() {
            if let Ok(tree) = riff::parse(&bytes[..cut]) {
                let _ = collect(&tree, Arc::new(bytes[..cut].to_vec()), 13, &KNOWN);
            }
        }
    }
}
