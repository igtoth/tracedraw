//! Objects embedded from other programs (`ole2` chunks): a compound file
//! (the public MS-CFB layout) whose storages are named like the objects
//! that show them ("Embedding 3"). A storage's `\x01Ole10Native` stream
//! holds the object's own data; for pictures from a paint program it is a
//! size (u32) and a whole `.bmp` file, which we decode. Seen in version 12
//! files.

use std::collections::HashMap;

const SIGNATURE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const END_OF_CHAIN: u32 = 0xFFFF_FFFE;
const FREE: u32 = 0xFFFF_FFFF;
const NO_STREAM: u32 = 0xFFFF_FFFF;
/// Longest sector chain followed, against loops in damaged files.
const MAX_CHAIN: usize = 1 << 20;

/// One directory entry.
#[derive(Debug, Clone)]
struct Entry {
    name: String,
    /// 1 storage, 2 stream, 5 root.
    kind: u8,
    left: u32,
    right: u32,
    child: u32,
    start: u32,
    size: u64,
}

/// A compound file read from memory.
pub struct Compound<'a> {
    buf: &'a [u8],
    sector: usize,
    mini_sector: usize,
    mini_cutoff: u64,
    fat: Vec<u32>,
    mini_fat: Vec<u32>,
    entries: Vec<Entry>,
    mini_stream: Vec<u8>,
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

impl<'a> Compound<'a> {
    pub fn parse(buf: &'a [u8]) -> Option<Compound<'a>> {
        if buf.get(..8)? != SIGNATURE {
            return None;
        }
        let sector_shift = u16_at(buf, 0x1E)?;
        let mini_shift = u16_at(buf, 0x20)?;
        if !(7..=16).contains(&sector_shift) || mini_shift > sector_shift {
            return None;
        }
        let mut c = Compound {
            buf,
            sector: 1 << sector_shift,
            mini_sector: 1 << mini_shift,
            mini_cutoff: u32_at(buf, 0x38)? as u64,
            fat: Vec::new(),
            mini_fat: Vec::new(),
            entries: Vec::new(),
            mini_stream: Vec::new(),
        };
        // The sectors of the FAT: 109 in the header, the rest in DIFAT
        // sectors (each ends with the next one's number).
        let mut fat_sectors: Vec<u32> = (0..109)
            .filter_map(|i| u32_at(buf, 0x4C + i * 4))
            .filter(|s| *s != FREE)
            .collect();
        let mut difat = u32_at(buf, 0x44)?;
        let per = c.sector / 4;
        let mut guard = 0;
        while difat != END_OF_CHAIN && difat != FREE && guard < MAX_CHAIN {
            let s = c.sector_bytes(difat)?;
            for i in 0..per - 1 {
                match u32_at(s, i * 4) {
                    Some(v) if v != FREE => fat_sectors.push(v),
                    _ => {}
                }
            }
            difat = u32_at(s, (per - 1) * 4)?;
            guard += 1;
        }
        for s in fat_sectors {
            let bytes = c.sector_bytes(s)?;
            c.fat.extend(
                bytes
                    .chunks_exact(4)
                    .map(|v| u32::from_le_bytes([v[0], v[1], v[2], v[3]])),
            );
        }
        let dir = c.chain(u32_at(buf, 0x30)?, None)?;
        for e in dir.chunks_exact(128) {
            let len = (u16_at(e, 64)? as usize).min(64);
            let units: Vec<u16> = e[..len.saturating_sub(2)]
                .chunks_exact(2)
                .map(|u| u16::from_le_bytes([u[0], u[1]]))
                .collect();
            c.entries.push(Entry {
                name: String::from_utf16_lossy(&units),
                kind: e[66],
                left: u32_at(e, 68)?,
                right: u32_at(e, 72)?,
                child: u32_at(e, 76)?,
                start: u32_at(e, 116)?,
                size: u64_at(e, 120)?,
            });
        }
        let mini = c.chain(u32_at(buf, 0x3C)?, None).unwrap_or_default();
        c.mini_fat = mini
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
            .collect();
        if let Some(root) = c.entries.first().cloned() {
            c.mini_stream = c.chain(root.start, Some(root.size)).unwrap_or_default();
        }
        Some(c)
    }

    fn sector_bytes(&self, n: u32) -> Option<&'a [u8]> {
        let at = (n as usize + 1).checked_mul(self.sector)?;
        self.buf.get(at..at.checked_add(self.sector)?)
    }

    /// The bytes of the sector chain from `start`, cut to `size`.
    fn chain(&self, start: u32, size: Option<u64>) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        let mut s = start;
        let mut n = 0;
        while s != END_OF_CHAIN && n < MAX_CHAIN {
            out.extend_from_slice(self.sector_bytes(s)?);
            s = *self.fat.get(s as usize)?;
            n += 1;
            if size.is_some_and(|z| out.len() as u64 >= z) {
                break;
            }
        }
        if let Some(z) = size {
            out.truncate(z.min(out.len() as u64) as usize);
        }
        Some(out)
    }

    fn mini_chain(&self, start: u32, size: u64) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        let mut s = start;
        let mut n = 0;
        while s != END_OF_CHAIN && n < MAX_CHAIN && (out.len() as u64) < size {
            let at = (s as usize).checked_mul(self.mini_sector)?;
            out.extend_from_slice(self.mini_stream.get(at..at + self.mini_sector)?);
            s = *self.mini_fat.get(s as usize)?;
            n += 1;
        }
        out.truncate(size.min(out.len() as u64) as usize);
        Some(out)
    }

    fn stream(&self, e: &Entry) -> Option<Vec<u8>> {
        if e.size < self.mini_cutoff {
            self.mini_chain(e.start, e.size)
        } else {
            self.chain(e.start, Some(e.size))
        }
    }

    /// The entries directly inside the storage at `index`.
    fn children(&self, index: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut todo = vec![self
            .entries
            .get(index)
            .map(|e| e.child)
            .unwrap_or(NO_STREAM)];
        while let Some(i) = todo.pop() {
            if i == NO_STREAM || out.len() > self.entries.len() {
                continue;
            }
            let Some(e) = self.entries.get(i as usize) else {
                continue;
            };
            out.push(i as usize);
            todo.push(e.left);
            todo.push(e.right);
        }
        out
    }

    /// Each storage's name with the bytes of its stream called `stream`.
    pub fn streams_named(&self, stream: &str) -> HashMap<String, Vec<u8>> {
        let mut out = HashMap::new();
        for (i, e) in self.entries.iter().enumerate() {
            if e.kind != 1 && e.kind != 5 {
                continue;
            }
            for k in self.children(i) {
                let s = &self.entries[k];
                if s.kind == 2 && s.name == stream {
                    if let Some(bytes) = self.stream(s) {
                        out.insert(e.name.clone(), bytes);
                    }
                }
            }
        }
        out
    }
}

/// A picture: width and height in pixels and its PNG bytes.
pub type Picture = (u32, u32, Vec<u8>);

/// The pictures of the embedded objects in the compound file `buf`, by
/// storage name.
pub fn embedded_pictures(buf: &[u8]) -> HashMap<String, Picture> {
    let Some(c) = Compound::parse(buf) else {
        return HashMap::new();
    };
    c.streams_named("\u{1}Ole10Native")
        .into_iter()
        .filter_map(|(name, data)| {
            // A u32 size, then the object's data.
            let body = data.get(4..)?;
            decode_bmp_file(body).map(|p| (name, p))
        })
        .collect()
}

/// The cached pictures (`\x02OlePres000` streams) of the embedded objects
/// in the compound file `buf` that hold a Windows metafile, by storage
/// name. Objects whose own data we cannot draw (a barcode, #1131) keep
/// the picture their program last drew there.
pub fn embedded_metafiles(buf: &[u8]) -> HashMap<String, Vec<u8>> {
    let Some(c) = Compound::parse(buf) else {
        return HashMap::new();
    };
    c.streams_named("\u{2}OlePres000")
        .into_iter()
        .filter_map(|(name, data)| presentation_metafile(&data).map(|m| (name, m)))
        .collect()
}

/// Clipboard formats of a presentation stream: a Windows metafile, an
/// enhanced one.
const CF_METAFILEPICT: u32 = 3;
const CF_ENHMETAFILE: u32 = 14;

/// The metafile of a presentation stream (the public MS-OLEDS layout):
/// the clipboard format (a u32 marker of -1 or -2, then the format), the
/// target device (a u32 size counting itself, then the rest), aspect,
/// index, advise flags, a reserved u32, width and height (0.01 mm), the
/// data size and the data. `None` for other formats.
pub fn presentation_metafile(b: &[u8]) -> Option<Vec<u8>> {
    let marker = u32_at(b, 0)?;
    if marker != 0xFFFF_FFFF && marker != 0xFFFF_FFFE {
        return None;
    }
    let format = u32_at(b, 4)?;
    if format != CF_METAFILEPICT && format != CF_ENHMETAFILE {
        return None;
    }
    let device = u32_at(b, 8)? as usize;
    if device < 4 {
        return None;
    }
    let at = 8usize.checked_add(device)?;
    let size = u32_at(b, at.checked_add(24)?)? as usize;
    let start = at.checked_add(28)?;
    let data = b.get(start..start.checked_add(size)?)?;
    Some(data.to_vec())
}

/// A `.bmp` file (a 40-byte or larger info header; 24 and 32 bits, or 1,
/// 4 and 8 bits with a palette; no compression) as a PNG.
pub fn decode_bmp_file(b: &[u8]) -> Option<Picture> {
    if b.get(..2)? != b"BM" {
        return None;
    }
    let pixels_at = u32_at(b, 10)? as usize;
    let info = 14;
    let header = u32_at(b, info)? as usize;
    if header < 40 {
        return None;
    }
    let width = i32::from_le_bytes(b.get(info + 4..info + 8)?.try_into().ok()?);
    let height = i32::from_le_bytes(b.get(info + 8..info + 12)?.try_into().ok()?);
    let bpp = u16_at(b, info + 14)?;
    let compression = u32_at(b, info + 16)?;
    if compression != 0 && !(compression == 3 && bpp == 32) {
        return None;
    }
    let (w, h) = (width.unsigned_abs(), height.unsigned_abs());
    if w == 0 || h == 0 || w > 32_768 || h > 32_768 {
        return None;
    }
    let bottom_up = height > 0;
    let palette: Vec<[u8; 3]> = if bpp <= 8 {
        let n = match u32_at(b, info + 32)? {
            0 => 1usize << bpp,
            n => (n as usize).min(256),
        };
        (0..n)
            .filter_map(|i| {
                let at = info + header + i * 4;
                let p = b.get(at..at + 3)?;
                Some([p[2], p[1], p[0]])
            })
            .collect()
    } else {
        Vec::new()
    };
    let stride = (w as usize * bpp as usize).div_ceil(32) * 4;
    let mut pm = tiny_skia::Pixmap::new(w, h)?;
    let data = pm.data_mut();
    for y in 0..h as usize {
        let src_row = if bottom_up { h as usize - 1 - y } else { y };
        let row = b.get(pixels_at + src_row * stride..pixels_at + (src_row + 1) * stride)?;
        for x in 0..w as usize {
            let [r, g, bl] = match bpp {
                32 | 24 => {
                    let n = bpp as usize / 8;
                    let p = row.get(x * n..x * n + 3)?;
                    [p[2], p[1], p[0]]
                }
                8 | 4 | 1 => {
                    let bit = x * bpp as usize;
                    let byte = *row.get(bit / 8)?;
                    let shift = 8 - bpp as usize - bit % 8;
                    let i = (byte >> shift) & ((1u16 << bpp) - 1) as u8;
                    *palette.get(i as usize).unwrap_or(&[0, 0, 0])
                }
                _ => return None,
            };
            let at = (y * w as usize + x) * 4;
            data[at..at + 4].copy_from_slice(&[r, g, bl, 255]);
        }
    }
    let png = pm.encode_png().ok()?;
    Some((w, h, png))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A 2 by 2 bottom-up 24-bit `.bmp`: red, green / blue, white.
    fn bmp() -> Vec<u8> {
        let mut b = Vec::new();
        let stride = 8; // 6 bytes padded to 8
        let size = 54 + stride * 2;
        b.extend_from_slice(b"BM");
        b.extend_from_slice(&(size as u32).to_le_bytes());
        b.extend_from_slice(&[0; 4]);
        b.extend_from_slice(&54u32.to_le_bytes());
        b.extend_from_slice(&40u32.to_le_bytes());
        b.extend_from_slice(&2i32.to_le_bytes());
        b.extend_from_slice(&2i32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&24u16.to_le_bytes());
        b.extend_from_slice(&[0; 24]);
        // Bottom row first: blue, white.
        b.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0]);
        b.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        b
    }

    #[test]
    fn bmp_files_decode_top_row_first() {
        let (w, h, png) = decode_bmp_file(&bmp()).expect("bmp");
        assert_eq!((w, h), (2, 2));
        let pm = tiny_skia::Pixmap::decode_png(&png).expect("png");
        let px = |x: u32, y: u32| {
            let p = pm.pixel(x, y).expect("pixel");
            [p.red(), p.green(), p.blue()]
        };
        assert_eq!(px(0, 0), [255, 0, 0]);
        assert_eq!(px(1, 0), [0, 255, 0]);
        assert_eq!(px(0, 1), [0, 0, 255]);
        assert_eq!(px(1, 1), [255, 255, 255]);
        // Damaged files give nothing, without a panic.
        let b = bmp();
        for cut in 0..b.len() {
            let _ = decode_bmp_file(&b[..cut]);
        }
        assert!(decode_bmp_file(b"BM").is_none());
    }

    /// A compound file of 512-byte sectors: header, FAT, directory, and
    /// one big stream of 4096 bytes inside a storage.
    fn compound(stream: &[u8]) -> Vec<u8> {
        compound_named("\u{1}Ole10Native", stream)
    }

    /// [`compound`] with the stream under another name.
    pub(crate) fn compound_named(name: &str, stream: &[u8]) -> Vec<u8> {
        assert_eq!(stream.len(), 4096);
        let mut h = vec![0u8; 512];
        h[..8].copy_from_slice(&SIGNATURE);
        h[0x1A..0x1C].copy_from_slice(&0x3Eu16.to_le_bytes());
        h[0x1C..0x1E].copy_from_slice(&3u16.to_le_bytes());
        h[0x1E..0x20].copy_from_slice(&9u16.to_le_bytes());
        h[0x20..0x22].copy_from_slice(&6u16.to_le_bytes());
        h[0x2C..0x30].copy_from_slice(&1u32.to_le_bytes());
        h[0x30..0x34].copy_from_slice(&1u32.to_le_bytes()); // directory at sector 1
        h[0x38..0x3C].copy_from_slice(&4096u32.to_le_bytes());
        h[0x3C..0x40].copy_from_slice(&END_OF_CHAIN.to_le_bytes());
        h[0x44..0x48].copy_from_slice(&END_OF_CHAIN.to_le_bytes());
        for i in 0..109 {
            h[0x4C + i * 4..0x50 + i * 4].copy_from_slice(&FREE.to_le_bytes());
        }
        h[0x4C..0x50].copy_from_slice(&0u32.to_le_bytes()); // FAT at sector 0
                                                            // FAT: 0 = FAT itself, 1 = directory, 2..10 = the stream.
        let mut fat = vec![FREE; 128];
        fat[0] = 0xFFFF_FFFD;
        fat[1] = END_OF_CHAIN;
        for s in 2..9 {
            fat[s] = s as u32 + 1;
        }
        fat[9] = END_OF_CHAIN;
        let fat: Vec<u8> = fat.iter().flat_map(|v| v.to_le_bytes()).collect();
        let entry = |name: &str, kind: u8, child: u32, start: u32, size: u64| {
            let mut e = vec![0u8; 128];
            let units: Vec<u16> = name.encode_utf16().chain([0]).collect();
            for (i, u) in units.iter().enumerate() {
                e[i * 2..i * 2 + 2].copy_from_slice(&u.to_le_bytes());
            }
            e[64..66].copy_from_slice(&((units.len() * 2) as u16).to_le_bytes());
            e[66] = kind;
            e[68..72].copy_from_slice(&NO_STREAM.to_le_bytes());
            e[72..76].copy_from_slice(&NO_STREAM.to_le_bytes());
            e[76..80].copy_from_slice(&child.to_le_bytes());
            e[116..120].copy_from_slice(&start.to_le_bytes());
            e[120..128].copy_from_slice(&size.to_le_bytes());
            e
        };
        let mut dir = Vec::new();
        dir.extend(entry("Root Entry", 5, 1, END_OF_CHAIN, 0));
        dir.extend(entry("Embedding 3", 1, 2, 0, 0));
        dir.extend(entry(name, 2, NO_STREAM, 2, 4096));
        dir.extend(vec![0u8; 128]);
        let mut out = h;
        out.extend(fat);
        out.extend(dir);
        out.extend_from_slice(stream);
        out
    }

    /// A compound file whose "Embedding 3" holds the 2 by 2 picture.
    pub(crate) fn compound_with_picture() -> Vec<u8> {
        let pic = bmp();
        let mut stream = (pic.len() as u32).to_le_bytes().to_vec();
        stream.extend_from_slice(&pic);
        stream.resize(4096, 0);
        compound(&stream)
    }

    #[test]
    fn pictures_embedded_from_a_paint_program_are_found_by_storage() {
        let file = compound_with_picture();
        let found = embedded_pictures(&file);
        let (w, h, _) = found.get("Embedding 3").expect("picture");
        assert_eq!((*w, *h), (2, 2));
        // Cut anywhere: no panic, at most nothing found.
        for cut in (0..file.len()).step_by(37) {
            let _ = embedded_pictures(&file[..cut]);
        }
        assert!(embedded_pictures(b"not a compound file").is_empty());
    }

    /// A presentation stream holding `metafile` as a Windows metafile
    /// picture, as the barcodes of #1131 keep theirs.
    pub(crate) fn presentation(metafile: &[u8]) -> Vec<u8> {
        let mut p = Vec::new();
        for v in [
            0xFFFF_FFFFu32,
            CF_METAFILEPICT,
            4,
            1,
            0xFFFF_FFFF,
            2,
            0,
            3810,
            2773,
        ] {
            p.extend_from_slice(&v.to_le_bytes());
        }
        p.extend_from_slice(&(metafile.len() as u32).to_le_bytes());
        p.extend_from_slice(metafile);
        p
    }

    #[test]
    fn metafile_pictures_of_embedded_objects_are_found_by_storage() {
        let wmf = [1u8, 0, 9, 0, 0, 3, 7, 7];
        let mut stream = presentation(&wmf);
        stream.resize(4096, 0);
        let file = compound_named("\u{2}OlePres000", &stream);
        let found = embedded_metafiles(&file);
        assert_eq!(found.get("Embedding 3").map(Vec::as_slice), Some(&wmf[..]));
        // Other formats (a bitmap, 8) and short or damaged streams give none.
        let mut dib = presentation(&wmf);
        dib[4] = 8;
        assert_eq!(presentation_metafile(&dib), None);
        let p = presentation(&wmf);
        for cut in 0..p.len() {
            assert_eq!(presentation_metafile(&p[..cut]), None, "{cut}");
        }
        let mut huge = presentation(&wmf);
        huge[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(presentation_metafile(&huge), None);
        for cut in (0..file.len()).step_by(37) {
            let _ = embedded_metafiles(&file[..cut]);
        }
    }
}
