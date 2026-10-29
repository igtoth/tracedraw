//! Decoded pictures, kept between draws: decoding a large PNG takes far
//! longer than drawing it, and a page draws its pictures again on every
//! change of the view. Pictures are found by a fingerprint of their PNG
//! bytes; past a memory budget the least recently drawn go first.
//!
//! One cache serves every thread: the canvas draws in bands on threads
//! of their own, and a cache per thread made every band decode every
//! picture it crosses again, on every render. A picture being decoded
//! is decoded once; other threads that need it wait for that result.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};
use tiny_skia::Pixmap;

/// Bytes of decoded pixels kept for the whole program.
#[cfg(target_arch = "wasm32")]
const BUDGET: usize = 256 << 20;
#[cfg(not(target_arch = "wasm32"))]
const BUDGET: usize = 512 << 20;

/// A picture, or why there is none, once decoded.
type Slot = Arc<OnceLock<Option<Arc<Pixmap>>>>;

struct Entry {
    slot: Slot,
    /// Decoded bytes counted in the budget; 0 while being decoded.
    size: usize,
    last_use: u64,
}

#[derive(Default)]
struct Cache {
    map: HashMap<u64, Entry>,
    bytes: usize,
    tick: u64,
}

impl Cache {
    /// The slot of `key`, a new empty one when it has none.
    fn slot(&mut self, key: u64) -> Slot {
        self.tick += 1;
        let tick = self.tick;
        let e = self.map.entry(key).or_insert_with(|| Entry {
            slot: Slot::default(),
            size: 0,
            last_use: tick,
        });
        e.last_use = tick;
        e.slot.clone()
    }

    /// Count the decoded picture of `slot` under `key`, dropping the least
    /// recently used pictures until the cache fits `budget`; a picture
    /// larger than half the budget is not kept.
    fn decoded(&mut self, key: u64, slot: &Slot, budget: usize) {
        let Some(e) = self.map.get_mut(&key) else {
            return;
        };
        if !Arc::ptr_eq(&e.slot, slot) || e.size != 0 {
            return;
        }
        let size = match slot.get() {
            Some(Some(img)) => img.data().len(),
            _ => return,
        };
        if size > budget / 2 {
            self.map.remove(&key);
            return;
        }
        e.size = size;
        self.bytes += size;
        while self.bytes > budget {
            let oldest = self
                .map
                .iter()
                .filter(|(_, e)| e.size > 0)
                .min_by_key(|(_, e)| e.last_use)
                .map(|(&k, _)| k);
            let Some(gone) = oldest.and_then(|k| self.map.remove(&k)) else {
                break;
            };
            self.bytes -= gone.size;
        }
    }
}

fn cache() -> std::sync::MutexGuard<'static, Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE
        .get_or_init(|| Mutex::new(Cache::default()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// A fingerprint of `png`: its length and bytes taken from its start, its
/// end (the last chunks and their checksums) and evenly in between.
pub(crate) fn fingerprint(png: &[u8]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let n = png.len();
    n.hash(&mut h);
    let mut take = |at: usize, len: usize| {
        png.get(at.min(n)..at.saturating_add(len).min(n))
            .hash(&mut h);
    };
    take(0, 64);
    take(n.saturating_sub(64), 64);
    for i in 1..32 {
        take(n / 32 * i, 32);
    }
    h.finish()
}

/// The picture of `png`, decoded once and then kept.
pub(crate) fn decode(png: &[u8]) -> Option<Arc<Pixmap>> {
    decode_in(png, BUDGET)
}

fn decode_in(png: &[u8], budget: usize) -> Option<Arc<Pixmap>> {
    let key = fingerprint(png);
    made_once(key, budget, || {
        #[cfg(test)]
        count_decode(key);
        Pixmap::decode_png(png).ok().map(Arc::new)
    })
}

/// The picture under `key`, made by `make` once and then kept (other
/// threads asking meanwhile wait for that result).
fn made_once(
    key: u64,
    budget: usize,
    make: impl FnOnce() -> Option<Arc<Pixmap>>,
) -> Option<Arc<Pixmap>> {
    let slot = cache().slot(key);
    if let Some(done) = slot.get() {
        return done.clone();
    }
    // Made outside the lock; a thread that asks meanwhile waits here.
    let img = slot.get_or_init(make).clone();
    cache().decoded(key, &slot, budget);
    img
}

/// Deepest reduced level kept: 1/4096 of the picture's sides.
pub(crate) const MAX_LEVEL: u32 = 12;

/// The picture of `png` reduced `level` times to half its width and
/// height (each pixel the average of the four beneath it; level 0 is the
/// picture itself), made once and kept in the same budget as the
/// decoded pictures. A picture drawn much smaller than its pixels is
/// drawn from the level nearest its size: sampling the full picture
/// there picks a few scattered pixels, so it looked noisy and broken
/// up, and it reads far more memory per drawn pixel (#853). Levels a
/// view no longer uses go first when memory runs short, the full
/// picture too.
pub(crate) fn decode_level(png: &[u8], level: u32) -> Option<Arc<Pixmap>> {
    let level = level.min(MAX_LEVEL);
    if level == 0 {
        return decode(png);
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (fingerprint(png), level).hash(&mut h);
    made_once(h.finish(), BUDGET, || {
        let above = decode_level(png, level - 1)?;
        if above.width() <= 1 && above.height() <= 1 {
            return Some(above);
        }
        halve(&above).map(Arc::new)
    })
}

/// `src` at half its width and height (rounded up), each pixel the
/// average of the (up to) four beneath it. Premultiplied colours average
/// to premultiplied colours.
pub(crate) fn halve(src: &Pixmap) -> Option<Pixmap> {
    let (sw, sh) = (src.width() as usize, src.height() as usize);
    let (w, h) = (sw.div_ceil(2).max(1), sh.div_ceil(2).max(1));
    let mut out = Pixmap::new(w as u32, h as u32)?;
    let from = src.data();
    let to = out.data_mut();
    for y in 0..h {
        let ys = [2 * y, (2 * y + 1).min(sh - 1)];
        for x in 0..w {
            let xs = [2 * x, (2 * x + 1).min(sw - 1)];
            let mut sum = [0u32; 4];
            for yy in ys {
                for xx in xs {
                    let i = (yy * sw + xx) * 4;
                    if let Some(px) = from.get(i..i + 4) {
                        for (s, v) in sum.iter_mut().zip(px) {
                            *s += *v as u32;
                        }
                    }
                }
            }
            let o = (y * w + x) * 4;
            if let Some(px) = to.get_mut(o..o + 4) {
                for (d, s) in px.iter_mut().zip(sum) {
                    *d = ((s + 2) / 4) as u8;
                }
            }
        }
    }
    Some(out)
}

#[cfg(test)]
static DECODES: Mutex<Option<HashMap<u64, usize>>> = Mutex::new(None);

#[cfg(test)]
fn count_decode(key: u64) {
    if let Ok(mut d) = DECODES.lock() {
        *d.get_or_insert_with(HashMap::new).entry(key).or_default() += 1;
    }
}

/// How often the picture of `png` was decoded, for tests.
#[cfg(test)]
pub(crate) fn decodes_of(png: &[u8]) -> usize {
    DECODES
        .lock()
        .ok()
        .and_then(|d| d.as_ref()?.get(&fingerprint(png)).copied())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32, shade: u8) -> Vec<u8> {
        let mut p = Pixmap::new(w, h).expect("pixmap");
        p.fill(tiny_skia::Color::from_rgba8(shade, 0, 0, 255));
        p.encode_png().expect("png")
    }

    #[test]
    fn a_picture_is_decoded_once() {
        let a = png(4, 4, 10);
        let b = png(4, 4, 200);
        let first = decode(&a).expect("decoded");
        let again = decode(&a).expect("decoded");
        assert!(Arc::ptr_eq(&first, &again));
        let other = decode(&b).expect("decoded");
        assert!(!Arc::ptr_eq(&first, &other));
        assert_eq!(other.pixel(0, 0).map(|p| p.red()), Some(200));
        assert!(decode(b"not a png").is_none());
        assert!(decode(b"not a png").is_none());
        assert_ne!(fingerprint(&a), fingerprint(&b));
        assert_ne!(fingerprint(&[]), fingerprint(&[0]));
    }

    /// Threads asking for one picture at once share a single decode.
    #[test]
    fn threads_share_one_decode() {
        let a = png(64, 64, 77);
        let all: Vec<Arc<Pixmap>> = std::thread::scope(|sc| {
            let hs: Vec<_> = (0..8).map(|_| sc.spawn(|| decode(&a))).collect();
            hs.into_iter()
                .filter_map(|h| h.join().ok().flatten())
                .collect()
        });
        assert_eq!(all.len(), 8);
        assert_eq!(decodes_of(&a), 1);
        assert!(all.iter().all(|p| Arc::ptr_eq(p, &all[0])));
    }

    /// Each level halves the sides (rounded up) and averages the pixels:
    /// a one-pixel checkerboard turns an even grey.
    #[test]
    fn reduced_levels_average_the_pixels() {
        let mut p = Pixmap::new(9, 4).expect("pixmap");
        for (i, px) in p.data_mut().chunks_mut(4).enumerate() {
            let (x, y) = (i % 9, i / 9);
            let v = if (x + y) % 2 == 0 { 255 } else { 0 };
            px.copy_from_slice(&[v, v, v, 255]);
        }
        let png = p.encode_png().expect("png");
        let one = decode_level(&png, 1).expect("level 1");
        assert_eq!((one.width(), one.height()), (5, 2));
        let c = one.pixel(1, 1).expect("pixel");
        assert!((126..=129).contains(&c.red()), "{c:?}");
        assert_eq!(c.alpha(), 255);
        let deep = decode_level(&png, 40).expect("deepest");
        assert_eq!((deep.width(), deep.height()), (1, 1));
        assert!(Arc::ptr_eq(&one, &decode_level(&png, 1).expect("kept")));
        assert!(Arc::ptr_eq(
            &decode(&png).expect("decoded"),
            &decode_level(&png, 0).expect("level 0")
        ));
    }

    #[test]
    fn the_oldest_pictures_go_past_the_budget() {
        let mut c = Cache::default();
        let put = |c: &mut Cache, key, shade| {
            let slot = c.slot(key);
            let img = Pixmap::decode_png(&png(8, 8, shade)).expect("png");
            let _ = slot.set(Some(Arc::new(img)));
            c.decoded(key, &slot, 600);
        };
        let has = |c: &mut Cache, key| c.map.get(&key).is_some_and(|e| e.size > 0);
        // 256 bytes each, a budget of 600: two fit.
        put(&mut c, 1, 1);
        put(&mut c, 2, 2);
        c.slot(1); // 1 is now newer than 2
        put(&mut c, 3, 3);
        assert!(!has(&mut c, 2));
        assert!(has(&mut c, 1) && has(&mut c, 3));
        assert_eq!(c.bytes, 512);
        // Counted once however often it is reported.
        let slot = c.slot(3);
        c.decoded(3, &slot, 600);
        assert_eq!(c.bytes, 512);
        // Too large for the budget: not kept.
        let slot = c.slot(4);
        let _ = slot.set(Some(Arc::new(
            Pixmap::decode_png(&png(16, 16, 4)).expect("png"),
        )));
        c.decoded(4, &slot, 600);
        assert!(!c.map.contains_key(&4));
        assert_eq!(c.bytes, 512);
    }
}
