//! The argument table of a `loda` (or `lobj`) chunk: the record that
//! describes an object, a layer or a page by a list of typed arguments
//! (see `docs/cdr-format.md`, "`loda` argument table").
//!
//! Layout: the length, the argument count, the offset of the offsets
//! table, the offset of the types table and the object type, each a u32
//! (a u16 before version 6). The offsets table has one entry more than
//! the count (the extra one ends the last argument); the types table is
//! stored in reverse order, so the first offset pairs with the last type.

/// One entry of the table: the argument's type and where its bytes lie
/// in the chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arg {
    pub ty: u32,
    pub off: usize,
    pub end: usize,
}

/// A decoded table: the object type and the arguments in table order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub kind: u32,
    pub args: Vec<Arg>,
}

/// Most arguments read from one table.
const MAX_ARGS: usize = 64;

/// An unsigned integer of the version's natural width at `at`: a u16
/// before version 6, a u32 from then on.
fn uint_at(d: &[u8], at: usize, version: u16) -> Option<u32> {
    if version < 6 {
        let s = d.get(at..at.checked_add(2)?)?;
        Some(u32::from(u16::from_le_bytes([s[0], s[1]])))
    } else {
        let s = d.get(at..at.checked_add(4)?)?;
        Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
}

/// Decode the table of a `loda` chunk's payload `d` written by `version`.
/// `None` when the header or an offset runs past the payload. An
/// argument whose end offset is out of order runs to the end of the
/// payload.
pub fn table(d: &[u8], version: u16) -> Option<Table> {
    let step = if version < 6 { 2 } else { 4 };
    let _len = uint_at(d, 0, version)?;
    let num_args = uint_at(d, step, version)? as usize;
    let start_args = uint_at(d, 2 * step, version)? as usize;
    let start_types = uint_at(d, 3 * step, version)? as usize;
    let kind = uint_at(d, 4 * step, version)?;
    let n = num_args.min(MAX_ARGS);
    let mut offsets = Vec::with_capacity(n + 1);
    for i in 0..=n {
        match uint_at(d, start_args.checked_add(i * step)?, version) {
            Some(o) => offsets.push(o as usize),
            None if i == n => offsets.push(d.len()),
            None => return None,
        }
    }
    let mut args = Vec::with_capacity(n);
    for i in 0..n {
        let ty = uint_at(d, start_types.checked_add((n - 1 - i) * step)?, version)?;
        let off = offsets[i].min(d.len());
        let next = offsets[i + 1];
        let end = if next > off && next <= d.len() {
            next
        } else {
            d.len()
        };
        args.push(Arg { ty, off, end });
    }
    Some(Table { kind, args })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table of `args` (type, bytes) for object type `kind`, as the
    /// reader's tests build them: header, offsets, reversed types, bodies.
    fn build(kind: u32, args: &[(u32, Vec<u8>)], version: u16) -> Vec<u8> {
        let step = if version < 6 { 2 } else { 4 };
        let put = |v: &mut Vec<u8>, x: u32| {
            if step == 2 {
                v.extend_from_slice(&(x as u16).to_le_bytes());
            } else {
                v.extend_from_slice(&x.to_le_bytes());
            }
        };
        let n = args.len() as u32;
        let header = 5 * step as u32;
        let offsets_at = header;
        let types_at = offsets_at + (n + 1) * step as u32;
        let bodies_at = types_at + n * step as u32;
        let mut bodies = Vec::new();
        let mut offsets = Vec::new();
        for (_, b) in args {
            offsets.push(bodies_at + bodies.len() as u32);
            bodies.extend_from_slice(b);
        }
        offsets.push(bodies_at + bodies.len() as u32);
        let mut v = Vec::new();
        put(&mut v, bodies_at + bodies.len() as u32);
        put(&mut v, n);
        put(&mut v, offsets_at);
        put(&mut v, types_at);
        put(&mut v, kind);
        for o in offsets {
            put(&mut v, o);
        }
        for (ty, _) in args.iter().rev() {
            put(&mut v, *ty);
        }
        v.extend(bodies);
        v
    }

    #[test]
    fn reads_the_arguments_in_table_order() {
        for version in [3u16, 7, 21] {
            let d = build(
                3,
                &[
                    (20, vec![1, 0, 0, 0]),
                    (30, vec![9; 10]),
                    (1000, vec![65, 0]),
                ],
                version,
            );
            let t = table(&d, version).expect("table");
            assert_eq!(t.kind, 3);
            let tys: Vec<u32> = t.args.iter().map(|a| a.ty).collect();
            assert_eq!(tys, [20, 30, 1000]);
            assert_eq!(&d[t.args[1].off..t.args[1].end], &[9; 10]);
            assert_eq!(&d[t.args[2].off..t.args[2].end], &[65, 0]);
        }
    }

    #[test]
    fn a_damaged_table_gives_what_it_can_or_nothing() {
        let d = build(1, &[(20, vec![1, 0, 0, 0]), (30, vec![2; 8])], 13);
        for cut in 0..d.len() {
            let _ = table(&d[..cut], 13);
        }
        assert!(table(&[], 13).is_none());
        // A last offset past the payload: the argument runs to the end.
        let mut d = d;
        let last = 20 + 3 * 4 - 4;
        d[last..last + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let t = table(&d, 13).expect("table");
        assert_eq!(t.args[1].end, d.len());
    }
}
