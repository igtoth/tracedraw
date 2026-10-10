//! Bitmap helpers: PNG codecs, resampling, rotation (Straighten Image),
//! inflation, colour masks and the simple colour modes. The effects of
//! the Effects menu are in `fx`.

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

fn bilinear(img: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    crate::fx::util::bilinear(img, x, y, false)
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

/// Rotate by `degrees` about the centre, growing the canvas (Straighten Image).
pub fn rotate(img: &RgbaImage, degrees: f32) -> RgbaImage {
    let a = degrees.to_radians();
    let (w, h) = (img.width() as f32, img.height() as f32);
    let nw = (w * a.cos().abs() + h * a.sin().abs()).ceil().max(1.0) as u32;
    let nh = (w * a.sin().abs() + h * a.cos().abs()).ceil().max(1.0) as u32;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let (ncx, ncy) = (nw as f32 / 2.0, nh as f32 / 2.0);
    let mut out = ImageBuffer::from_pixel(nw, nh, Rgba([0, 0, 0, 0]));
    for y in 0..nh {
        for x in 0..nw {
            let dx = x as f32 - ncx;
            let dy = y as f32 - ncy;
            let sx = cx + dx * a.cos() + dy * a.sin();
            let sy = cy - dx * a.sin() + dy * a.cos();
            if sx >= 0.0 && sy >= 0.0 && sx < w && sy < h {
                out.put_pixel(x, y, bilinear(img, sx, sy));
            }
        }
    }
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

/// Colour mask: make pixels within `tolerance` of any of `colors` transparent.
pub fn color_mask(img: &RgbaImage, colors: &[[u8; 3]], tolerance: u8) -> RgbaImage {
    crate::fx::util::map_px(img, |_, _, p| {
        let hit = colors.iter().any(|c| {
            (p[0] as i32 - c[0] as i32).abs() <= tolerance as i32
                && (p[1] as i32 - c[1] as i32).abs() <= tolerance as i32
                && (p[2] as i32 - c[2] as i32).abs() <= tolerance as i32
        });
        if hit {
            Rgba([p[0], p[1], p[2], 0])
        } else {
            *p
        }
    })
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
    fn rotate_grows_canvas_and_inflate_pads() {
        let g = gradient();
        let r = rotate(&g, 45.0);
        assert!(r.width() > 40 && r.height() > 40);
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
}
