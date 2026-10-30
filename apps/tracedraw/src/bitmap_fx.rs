//! Bitmap helpers: PNG codecs, resampling, inflation, colour masks and
//! the simple colour modes. The effects of the Effects menu are in `fx`,
//! Straighten Image in `straighten`.

use image::{ImageBuffer, Rgba, RgbaImage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Grayscale,
    Rgb,
    Cmyk,
}

/// Convert the colour mode (Bitmaps > Mode).
pub fn convert_mode(img: &RgbaImage, mode: ColorMode) -> RgbaImage {
    match mode {
        ColorMode::Grayscale => {
            let mut out = img.clone();
            for p in out.pixels_mut() {
                let l = (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                *p = Rgba([l, l, l, p[3]]);
            }
            out
        }
        // RGB and CMYK keep the pixels; the mode is a document attribute.
        ColorMode::Rgb | ColorMode::Cmyk => img.clone(),
    }
}

/// Grid points per channel of the table [`through_cmyk`] samples.
const CMYK_GRID: usize = 17;

/// `img` as it prints: every pixel taken to CMYK and back to the screen
/// through the colour engine (the drawing's profiles, or the press model
/// without them), so a picture converted to CMYK shows the colours the
/// inks can make (#1254). Alpha is kept. The engine is sampled on a grid
/// of `CMYK_GRID` points per channel and the pixels read from it by
/// trilinear interpolation, so a large photo takes a few milliseconds.
pub fn through_cmyk(img: &RgbaImage) -> RgbaImage {
    let n = CMYK_GRID;
    let step = 255.0 / (n - 1) as f32;
    let mut grid: Vec<[f32; 3]> = Vec::with_capacity(n * n * n);
    for r in 0..n {
        for g in 0..n {
            for b in 0..n {
                let c = tracedraw_core::Color::rgb8(
                    (r as f32 * step).round() as u8,
                    (g as f32 * step).round() as u8,
                    (b as f32 * step).round() as u8,
                );
                let back = c.convert_to("CMYK").convert_to("RGB").to_rgb8();
                grid.push([back[0] as f32, back[1] as f32, back[2] as f32]);
            }
        }
    }
    let at = |r: usize, g: usize, b: usize| grid[(r * n + g) * n + b];
    let mut out = img.clone();
    for p in out.pixels_mut() {
        let pos = |v: u8| {
            let x = v as f32 / step;
            let i = (x.floor() as usize).min(n - 2);
            (i, x - i as f32)
        };
        let (ri, rf) = pos(p[0]);
        let (gi, gf) = pos(p[1]);
        let (bi, bf) = pos(p[2]);
        let mut acc = [0.0f32; 3];
        for (dr, wr) in [(0, 1.0 - rf), (1, rf)] {
            for (dg, wg) in [(0, 1.0 - gf), (1, gf)] {
                for (db, wb) in [(0, 1.0 - bf), (1, bf)] {
                    let w = wr * wg * wb;
                    if w > 0.0 {
                        let v = at(ri + dr, gi + dg, bi + db);
                        acc[0] += w * v[0];
                        acc[1] += w * v[1];
                        acc[2] += w * v[2];
                    }
                }
            }
        }
        *p = Rgba([
            acc[0].round().clamp(0.0, 255.0) as u8,
            acc[1].round().clamp(0.0, 255.0) as u8,
            acc[2].round().clamp(0.0, 255.0) as u8,
            p[3],
        ]);
    }
    out
}

/// Decode PNG bytes into an RGBA image.
pub fn decode(png: &[u8]) -> Option<RgbaImage> {
    image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .ok()
        .map(|i| i.to_rgba8())
}

/// Encode an RGBA image as PNG bytes.
pub fn encode(img: &RgbaImage) -> Option<Vec<u8>> {
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).ok()?;
    Some(buf.into_inner())
}

/// Resample to a new pixel size (Bitmaps > Resample).
pub fn resample(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    image::imageops::resize(
        img,
        w.max(1),
        h.max(1),
        image::imageops::FilterType::Lanczos3,
    )
}

/// Grow the canvas to `w` by `h` pixels (at least the image's size), the
/// image in the middle and transparent around it.
pub fn inflate_to(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    let (w, h) = (w.max(img.width()), h.max(img.height()));
    let mut out = ImageBuffer::from_pixel(w, h, Rgba([0, 0, 0, 0]));
    let (left, top) = ((w - img.width()) / 2, (h - img.height()) / 2);
    image::imageops::overlay(&mut out, img, left as i64, top as i64);
    out
}

/// Grow the canvas by `px` on every side (Inflate Bitmap).
pub fn inflate(img: &RgbaImage, px: u32) -> RgbaImage {
    let mut out = ImageBuffer::from_pixel(
        img.width() + 2 * px,
        img.height() + 2 * px,
        Rgba([0, 0, 0, 0]),
    );
    image::imageops::overlay(&mut out, img, px as i64, px as i64);
    out
}

/// The slots of the Bitmap Mask docker.
pub const MASK_SLOTS: usize = 10;

/// One colour of a bitmap mask: in use, the colour, and how far (in % of
/// the channel range) a pixel may be from it and still match.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaskSlot {
    pub on: bool,
    pub color: tracedraw_core::Color,
    pub tolerance: u8,
}

impl Default for MaskSlot {
    fn default() -> Self {
        MaskSlot {
            on: false,
            color: tracedraw_core::Color::BLACK,
            tolerance: 0,
        }
    }
}

/// The Bitmap Mask docker's state.
#[derive(Debug, Clone, PartialEq)]
pub struct MaskSettings {
    pub slots: [MaskSlot; MASK_SLOTS],
    pub picked: usize,
    /// Hide the selected colours (else show only them).
    pub hide: bool,
    /// The eyedropper fills the picked slot with the colour it samples.
    pub picking: bool,
    /// The sampled colour already taken, so one sample fills one slot.
    pub seen: Option<tracedraw_core::Color>,
}

impl Default for MaskSettings {
    fn default() -> Self {
        MaskSettings {
            slots: [MaskSlot::default(); MASK_SLOTS],
            picked: 0,
            hide: false,
            picking: false,
            seen: None,
        }
    }
}

impl MaskSettings {
    /// The slots in use.
    pub fn active(&self) -> Vec<MaskSlot> {
        self.slots.iter().copied().filter(|s| s.on).collect()
    }

    /// The mask as a text file: one line per slot, "on #rrggbb tolerance".
    pub fn to_text(&self) -> String {
        self.slots
            .iter()
            .map(|s| format!("{} {} {}\n", u8::from(s.on), s.color.to_hex(), s.tolerance))
            .collect()
    }

    /// Read a mask written by [`MaskSettings::to_text`]; lines that do not
    /// read are skipped.
    pub fn read_text(&mut self, text: &str) -> bool {
        let mut slots = [MaskSlot::default(); MASK_SLOTS];
        let mut n = 0;
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let (Some(on), Some(hex), Some(tol)) = (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let (Some(color), Ok(tol)) = (tracedraw_core::Color::from_hex(hex), tol.parse::<u8>())
            else {
                continue;
            };
            if n < MASK_SLOTS {
                slots[n] = MaskSlot {
                    on: on == "1",
                    color,
                    tolerance: tol.min(100),
                };
                n += 1;
            }
        }
        if n > 0 {
            self.slots = slots;
        }
        n > 0
    }
}

/// Bitmap mask: with `hide`, pixels matching any slot become transparent;
/// without, only the matching pixels stay. A slot matches a pixel whose
/// channels are each within the slot's tolerance (% of 255).
pub fn mask_bitmap(img: &RgbaImage, slots: &[MaskSlot], hide: bool) -> RgbaImage {
    let targets: Vec<([i32; 3], i32)> = slots
        .iter()
        .map(|s| {
            let [r, g, b] = s.color.to_rgb8();
            (
                [r as i32, g as i32, b as i32],
                (s.tolerance.min(100) as i32 * 255 + 50) / 100,
            )
        })
        .collect();
    crate::fx::util::map_px(img, |_, _, p| {
        let hit = targets.iter().any(|(c, t)| {
            (p[0] as i32 - c[0]).abs() <= *t
                && (p[1] as i32 - c[1]).abs() <= *t
                && (p[2] as i32 - c[2]).abs() <= *t
        });
        if hit == hide {
            Rgba([p[0], p[1], p[2], 0])
        } else {
            *p
        }
    })
}

/// The `n` colours a bitmap uses most (opaque pixels, near colours
/// merged), most used first.
pub fn main_colors(img: &RgbaImage, n: usize) -> Vec<[u8; 3]> {
    let mut counts: std::collections::HashMap<[u8; 3], usize> = Default::default();
    for p in img.pixels() {
        if p[3] < 128 {
            continue;
        }
        // Merged by their top five bits, so noise does not split a colour.
        let key = [p[0] & 0xF8, p[1] & 0xF8, p[2] & 0xF8];
        *counts.entry(key).or_default() += 1;
    }
    let mut v: Vec<([u8; 3], usize)> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.into_iter()
        .take(n)
        .map(|(c, _)| [c[0] | 4, c[1] | 4, c[2] | 4])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient() -> RgbaImage {
        ImageBuffer::from_fn(32, 32, |x, _| Rgba([(x * 8) as u8, 128, 64, 255]))
    }

    #[test]
    fn grayscale_mode_uses_luminance() {
        let g = convert_mode(&gradient(), ColorMode::Grayscale);
        let p = g.get_pixel(10, 3);
        assert!(p[0] == p[1] && p[1] == p[2]);
        assert_eq!(convert_mode(&gradient(), ColorMode::Rgb), gradient());
    }

    #[test]
    fn inflate_pads_with_transparency() {
        let g = gradient();
        let i = inflate(&g, 5);
        assert_eq!(i.dimensions(), (42, 42));
        assert_eq!(i.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn png_round_trip() {
        let g = gradient();
        let png = encode(&g).unwrap();
        let back = decode(&png).unwrap();
        assert_eq!(back.get_pixel(10, 10), g.get_pixel(10, 10));
    }

    #[test]
    fn a_bitmap_mask_hides_or_keeps_its_colours() {
        let img: RgbaImage = ImageBuffer::from_fn(4, 1, |x, _| match x {
            0 => Rgba([250, 10, 10, 255]),
            1 => Rgba([240, 20, 20, 255]),
            2 => Rgba([10, 10, 250, 255]),
            _ => Rgba([255, 255, 255, 255]),
        });
        let red = MaskSlot {
            on: true,
            color: tracedraw_core::Color::rgb8(255, 0, 0),
            tolerance: 10,
        };
        // 10 % is 26 levels: both reds match, blue and white do not.
        let hidden = mask_bitmap(&img, &[red], true);
        let alpha: Vec<u8> = hidden.pixels().map(|p| p[3]).collect();
        assert_eq!(alpha, vec![0, 0, 255, 255]);
        let kept = mask_bitmap(&img, &[red], false);
        let alpha: Vec<u8> = kept.pixels().map(|p| p[3]).collect();
        assert_eq!(alpha, vec![255, 255, 0, 0]);
        // The main colours, most used first.
        let many: RgbaImage = ImageBuffer::from_fn(10, 1, |x, _| {
            if x < 7 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let main = main_colors(&many, 10);
        assert_eq!(main.len(), 2);
        assert!(main[0][0] < 10 && main[1][0] > 245);
        // The mask file reads back.
        let mut m = MaskSettings::default();
        m.slots[0] = red;
        m.slots[3].tolerance = 40;
        let text = m.to_text();
        let mut back = MaskSettings::default();
        assert!(back.read_text(&text));
        assert_eq!(back.slots, m.slots);
        assert!(!back.read_text("nothing here"));
    }
}
