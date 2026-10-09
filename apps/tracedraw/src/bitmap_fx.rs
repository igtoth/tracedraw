//! Bitmap effects (Effects menu groups: 3D, Adjust, Art Strokes, Blur,
//! Camera, Color Transform, Contour, Correction, Creative, Custom,
//! Distort, Noise, Sharpen, Texture, Transform) and bitmap colour modes.
//! Everything works on RGBA8 buffers; results go back as PNG.

use image::{ImageBuffer, Rgba, RgbaImage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fx {
    // 3D
    Emboss,
    PageCurl,
    Perspective3d,
    // Adjust
    BrightnessContrast,
    HueSaturation,
    AutoLevels,
    Gamma,
    Desaturate,
    // Art strokes
    Impressionist,
    Crayon,
    Sketch,
    Watercolor,
    // Blur
    Gaussian,
    Motion,
    Radial,
    Smart,
    Soften,
    // Camera
    DiffuseGlow,
    LensFlare,
    Vignette,
    // Color transform
    Invert,
    Posterize,
    Psychedelic,
    Halftone,
    // Contour
    EdgeDetect,
    FindEdges,
    TraceContour,
    // Correction
    DustAndScratch,
    // Creative
    Mosaic,
    Pixelate,
    Crystallize,
    Vortex,
    // Custom
    Bump,
    // Distort
    Ripple,
    Swirl,
    Wind,
    Tile,
    // Noise
    AddNoise,
    RemoveNoise,
    Median,
    // Sharpen
    Sharpen,
    UnsharpMask,
    HighPass,
    // Texture
    Canvas,
    Plastic,
    Stone,
    // Transform
    Deinterlace,
    Threshold,
}

pub struct FxEntry {
    pub key: &'static str,
    pub fx: Fx,
}

const fn e(key: &'static str, fx: Fx) -> FxEntry {
    FxEntry { key, fx }
}

/// Menu groups in the target design's order.
pub const GROUPS: &[(&str, &[FxEntry])] = &[
    (
        "menu.effects.3d",
        &[
            e("fx.emboss", Fx::Emboss),
            e("fx.page_curl", Fx::PageCurl),
            e("fx.perspective", Fx::Perspective3d),
        ],
    ),
    (
        "menu.effects.adjust",
        &[
            e("fx.brightness_contrast", Fx::BrightnessContrast),
            e("fx.hue_saturation", Fx::HueSaturation),
            e("fx.auto_levels", Fx::AutoLevels),
            e("fx.gamma", Fx::Gamma),
            e("fx.desaturate", Fx::Desaturate),
        ],
    ),
    (
        "menu.effects.art_strokes",
        &[
            e("fx.impressionist", Fx::Impressionist),
            e("fx.crayon", Fx::Crayon),
            e("fx.sketch", Fx::Sketch),
            e("fx.watercolor", Fx::Watercolor),
        ],
    ),
    (
        "menu.effects.blur",
        &[
            e("fx.gaussian_blur", Fx::Gaussian),
            e("fx.motion_blur", Fx::Motion),
            e("fx.radial_blur", Fx::Radial),
            e("fx.smart_blur", Fx::Smart),
            e("fx.soften", Fx::Soften),
        ],
    ),
    (
        "menu.effects.camera",
        &[
            e("fx.diffuse_glow", Fx::DiffuseGlow),
            e("fx.lens_flare", Fx::LensFlare),
            e("fx.vignette", Fx::Vignette),
        ],
    ),
    (
        "menu.effects.color_transform",
        &[
            e("fx.invert", Fx::Invert),
            e("fx.posterize", Fx::Posterize),
            e("fx.psychedelic", Fx::Psychedelic),
            e("fx.halftone", Fx::Halftone),
        ],
    ),
    (
        "menu.effects.contour_group",
        &[
            e("fx.edge_detect", Fx::EdgeDetect),
            e("fx.find_edges", Fx::FindEdges),
            e("fx.trace_contour", Fx::TraceContour),
        ],
    ),
    (
        "menu.effects.correction",
        &[e("fx.dust_and_scratch", Fx::DustAndScratch)],
    ),
    (
        "menu.effects.creative",
        &[
            e("fx.mosaic", Fx::Mosaic),
            e("fx.pixelate", Fx::Pixelate),
            e("fx.crystallize", Fx::Crystallize),
            e("fx.vortex", Fx::Vortex),
        ],
    ),
    ("menu.effects.custom", &[e("fx.bump_map", Fx::Bump)]),
    (
        "menu.effects.distort",
        &[
            e("fx.ripple", Fx::Ripple),
            e("fx.swirl", Fx::Swirl),
            e("fx.wind", Fx::Wind),
            e("fx.tile", Fx::Tile),
        ],
    ),
    (
        "menu.effects.noise",
        &[
            e("fx.add_noise", Fx::AddNoise),
            e("fx.remove_noise", Fx::RemoveNoise),
            e("fx.median", Fx::Median),
        ],
    ),
    (
        "menu.effects.sharpen",
        &[
            e("fx.sharpen", Fx::Sharpen),
            e("fx.unsharp_mask", Fx::UnsharpMask),
            e("fx.high_pass", Fx::HighPass),
        ],
    ),
    (
        "menu.effects.texture",
        &[
            e("fx.canvas", Fx::Canvas),
            e("fx.plastic", Fx::Plastic),
            e("fx.stone", Fx::Stone),
        ],
    ),
    (
        "menu.effects.transform",
        &[
            e("fx.deinterlace", Fx::Deinterlace),
            e("fx.threshold", Fx::Threshold),
        ],
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    BlackWhite,
    Grayscale,
    Rgb,
    Cmyk,
}

/// Apply an effect with `amount` 0..100 (the single slider of the dialog).
pub fn apply(img: &RgbaImage, fx: Fx, amount: f32) -> RgbaImage {
    let a = amount.clamp(0.0, 100.0) / 100.0;
    match fx {
        Fx::Emboss => emboss(img, a),
        Fx::PageCurl => page_curl(img, a),
        Fx::Perspective3d => perspective(img, a),
        Fx::BrightnessContrast => brightness_contrast(img, (a - 0.5) * 0.6, 1.0 + (a - 0.5)),
        Fx::HueSaturation => hue_saturation(img, (a - 0.5) * 180.0, 1.0 + (a - 0.5)),
        Fx::AutoLevels => auto_levels(img),
        Fx::Gamma => gamma(img, 0.4 + a * 1.6),
        Fx::Desaturate => hue_saturation(img, 0.0, 1.0 - a),
        Fx::Impressionist => impressionist(img, 2 + (a * 10.0) as u32),
        Fx::Crayon => crayon(img, a),
        Fx::Sketch => sketch(img),
        Fx::Watercolor => watercolor(img, 1 + (a * 6.0) as u32),
        Fx::Gaussian => gaussian(img, 0.5 + a * 10.0),
        Fx::Motion => motion_blur(img, 1 + (a * 30.0) as u32),
        Fx::Radial => radial_blur(img, a * 0.2),
        Fx::Smart => smart_blur(img, 1 + (a * 4.0) as u32, 20.0 + a * 60.0),
        Fx::Soften => gaussian(img, 0.5 + a * 2.0),
        Fx::DiffuseGlow => diffuse_glow(img, a),
        Fx::LensFlare => lens_flare(img, a),
        Fx::Vignette => vignette(img, a),
        Fx::Invert => invert(img),
        Fx::Posterize => posterize(img, 2 + ((1.0 - a) * 6.0) as u8),
        Fx::Psychedelic => psychedelic(img, a),
        Fx::Halftone => halftone(img, 2 + (a * 10.0) as u32),
        Fx::EdgeDetect => edge_detect(img, false),
        Fx::FindEdges => edge_detect(img, true),
        Fx::TraceContour => trace_contour(img, 64 + (a * 128.0) as i32),
        Fx::DustAndScratch => median(img, 1 + (a * 2.0) as u32),
        Fx::Mosaic => mosaic(img, 2 + (a * 30.0) as u32, true),
        Fx::Pixelate => mosaic(img, 2 + (a * 30.0) as u32, false),
        Fx::Crystallize => crystallize(img, 4 + (a * 40.0) as u32),
        Fx::Vortex => swirl(img, a * 6.0),
        Fx::Bump => bump(img, a),
        Fx::Ripple => ripple(img, 2.0 + a * 20.0, 10.0 + a * 60.0),
        Fx::Swirl => swirl(img, (a - 0.5) * 4.0),
        Fx::Wind => wind(img, 1 + (a * 40.0) as u32),
        Fx::Tile => tile(img, 2 + (a * 6.0) as u32),
        Fx::AddNoise => add_noise(img, a * 0.5),
        Fx::RemoveNoise => median(img, 1),
        Fx::Median => median(img, 1 + (a * 3.0) as u32),
        Fx::Sharpen => sharpen(img, 0.2 + a * 1.5),
        Fx::UnsharpMask => unsharp(img, 1.0 + a * 4.0, 0.3 + a * 1.5),
        Fx::HighPass => high_pass(img, 1.0 + a * 10.0),
        Fx::Canvas => texture(img, 3, a * 0.6),
        Fx::Plastic => plastic(img, a),
        Fx::Stone => texture(img, 7, a * 0.8),
        Fx::Deinterlace => deinterlace(img),
        Fx::Threshold => threshold(img, (a * 255.0) as u8),
    }
}

/// Convert the colour mode (Bitmaps > Mode).
pub fn convert_mode(img: &RgbaImage, mode: ColorMode) -> RgbaImage {
    match mode {
        ColorMode::BlackWhite => threshold(img, 128),
        ColorMode::Grayscale => hue_saturation(img, 0.0, 0.0),
        // RGB and CMYK keep the pixels; the mode is a document attribute.
        ColorMode::Rgb | ColorMode::Cmyk => img.clone(),
    }
}

// ----- helpers ---------------------------------------------------------------

fn map_px(img: &RgbaImage, f: impl Fn(u32, u32, Rgba<u8>) -> Rgba<u8>) -> RgbaImage {
    let mut out = ImageBuffer::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        out.put_pixel(x, y, f(x, y, *p));
    }
    out
}

fn clamp8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

fn luma(p: Rgba<u8>) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

fn sample(img: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let xi = (x.round() as i64).clamp(0, w - 1);
    let yi = (y.round() as i64).clamp(0, h - 1);
    *img.get_pixel(xi as u32, yi as u32)
}

fn bilinear(img: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let x = x.clamp(0.0, w - 1.0);
    let y = y.clamp(0.0, h - 1.0);
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = x - x0;
    let fy = y - y0;
    let p00 = sample(img, x0, y0);
    let p10 = sample(img, x0 + 1.0, y0);
    let p01 = sample(img, x0, y0 + 1.0);
    let p11 = sample(img, x0 + 1.0, y0 + 1.0);
    let mut out = [0u8; 4];
    for c in 0..4 {
        let a = p00[c] as f32 * (1.0 - fx) + p10[c] as f32 * fx;
        let b = p01[c] as f32 * (1.0 - fx) + p11[c] as f32 * fx;
        out[c] = clamp8(a * (1.0 - fy) + b * fy);
    }
    Rgba(out)
}

fn remap(img: &RgbaImage, f: impl Fn(f32, f32) -> (f32, f32)) -> RgbaImage {
    let mut out = ImageBuffer::new(img.width(), img.height());
    for y in 0..img.height() {
        for x in 0..img.width() {
            let (sx, sy) = f(x as f32, y as f32);
            out.put_pixel(x, y, bilinear(img, sx, sy));
        }
    }
    out
}

fn convolve(img: &RgbaImage, k: &[f32], size: usize, bias: f32) -> RgbaImage {
    let r = (size / 2) as i64;
    let (w, h) = (img.width() as i64, img.height() as i64);
    let mut out = ImageBuffer::new(img.width(), img.height());
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f32; 3];
            for ky in 0..size as i64 {
                for kx in 0..size as i64 {
                    let sx = (x + kx - r).clamp(0, w - 1);
                    let sy = (y + ky - r).clamp(0, h - 1);
                    let p = img.get_pixel(sx as u32, sy as u32);
                    let kv = k[(ky as usize) * size + kx as usize];
                    for c in 0..3 {
                        acc[c] += p[c] as f32 * kv;
                    }
                }
            }
            let a = img.get_pixel(x as u32, y as u32)[3];
            out.put_pixel(
                x as u32,
                y as u32,
                Rgba([
                    clamp8(acc[0] + bias),
                    clamp8(acc[1] + bias),
                    clamp8(acc[2] + bias),
                    a,
                ]),
            );
        }
    }
    out
}

fn gaussian(img: &RgbaImage, sigma: f32) -> RgbaImage {
    image::imageops::blur(img, sigma)
}

// ----- effects ---------------------------------------------------------------

fn emboss(img: &RgbaImage, a: f32) -> RgbaImage {
    let s = 1.0 + a * 2.0;
    let k = [-s, -1.0, 0.0, -1.0, 1.0, 1.0, 0.0, 1.0, s];
    let e = convolve(img, &k, 3, 128.0);
    map_px(&e, |_, _, p| {
        let l = clamp8(luma(p));
        Rgba([l, l, l, p[3]])
    })
}

fn page_curl(img: &RgbaImage, a: f32) -> RgbaImage {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let radius = (0.15 + a * 0.4) * w.min(h);
    map_px(img, |x, y, p| {
        // Distance beyond the diagonal fold line at the bottom-right corner.
        let d = (x as f32 + y as f32) - (w + h - radius * 2.0);
        if d <= 0.0 {
            p
        } else if d < radius {
            let t = d / radius;
            let shade = 1.0 - t * 0.6;
            Rgba([
                clamp8(p[0] as f32 * shade),
                clamp8(p[1] as f32 * shade),
                clamp8(p[2] as f32 * shade),
                p[3],
            ])
        } else {
            let t = ((d - radius) / radius).min(1.0);
            let g = clamp8(230.0 - t * 60.0);
            Rgba([g, g, g, p[3]])
        }
    })
}

fn perspective(img: &RgbaImage, a: f32) -> RgbaImage {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let k = a * 0.5;
    remap(img, |x, y| {
        // Top edge narrower: map to source by inverse scaling per row.
        let t = y / h;
        let scale = 1.0 - k * (1.0 - t);
        let cx = w / 2.0;
        let sx = cx + (x - cx) / scale.max(0.05);
        (sx, y)
    })
}

fn brightness_contrast(img: &RgbaImage, brightness: f32, contrast: f32) -> RgbaImage {
    map_px(img, |_, _, p| {
        let f = |v: u8| clamp8(((v as f32 / 255.0 - 0.5) * contrast + 0.5 + brightness) * 255.0);
        Rgba([f(p[0]), f(p[1]), f(p[2]), p[3]])
    })
}

fn hue_saturation(img: &RgbaImage, hue_shift: f32, sat: f32) -> RgbaImage {
    map_px(img, |_, _, p| {
        let (h, s, v) = tracedraw_core::color::rgb_to_hsb(
            p[0] as f32 / 255.0,
            p[1] as f32 / 255.0,
            p[2] as f32 / 255.0,
        );
        let [r, g, b] =
            tracedraw_core::color::hsb_to_rgb(h + hue_shift, (s * sat).clamp(0.0, 1.0), v);
        Rgba([
            clamp8(r * 255.0),
            clamp8(g * 255.0),
            clamp8(b * 255.0),
            p[3],
        ])
    })
}

fn auto_levels(img: &RgbaImage) -> RgbaImage {
    let mut lo = [255u8; 3];
    let mut hi = [0u8; 3];
    for p in img.pixels() {
        for c in 0..3 {
            lo[c] = lo[c].min(p[c]);
            hi[c] = hi[c].max(p[c]);
        }
    }
    map_px(img, |_, _, p| {
        let mut o = [0u8; 4];
        for c in 0..3 {
            let range = (hi[c] as f32 - lo[c] as f32).max(1.0);
            o[c] = clamp8((p[c] as f32 - lo[c] as f32) / range * 255.0);
        }
        o[3] = p[3];
        Rgba(o)
    })
}

fn gamma(img: &RgbaImage, g: f32) -> RgbaImage {
    let lut: Vec<u8> = (0..256)
        .map(|i| clamp8((i as f32 / 255.0).powf(1.0 / g) * 255.0))
        .collect();
    map_px(img, |_, _, p| {
        Rgba([
            lut[p[0] as usize],
            lut[p[1] as usize],
            lut[p[2] as usize],
            p[3],
        ])
    })
}

fn impressionist(img: &RgbaImage, dab: u32) -> RgbaImage {
    let mut out = img.clone();
    let mut seed = 12345u32;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    let n = (img.width() * img.height() / (dab * dab).max(1)) * 2;
    for _ in 0..n {
        let x = rnd() % img.width();
        let y = rnd() % img.height();
        let p = *img.get_pixel(x, y);
        let r = dab as i64 / 2;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                let (px, py) = (x as i64 + dx, y as i64 + dy);
                if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                    out.put_pixel(px as u32, py as u32, p);
                }
            }
        }
    }
    out
}

fn crayon(img: &RgbaImage, a: f32) -> RgbaImage {
    let edges = edge_detect(img, true);
    let post = posterize(img, 4);
    let mut out = ImageBuffer::new(img.width(), img.height());
    for (x, y, p) in post.enumerate_pixels() {
        let e = edges.get_pixel(x, y);
        let l = luma(*e) / 255.0;
        let k = 1.0 - l * a;
        out.put_pixel(
            x,
            y,
            Rgba([
                clamp8(p[0] as f32 * k),
                clamp8(p[1] as f32 * k),
                clamp8(p[2] as f32 * k),
                p[3],
            ]),
        );
    }
    out
}

fn sketch(img: &RgbaImage) -> RgbaImage {
    let e = edge_detect(img, false);
    map_px(&e, |_, _, p| {
        let l = clamp8(255.0 - luma(p));
        Rgba([l, l, l, p[3]])
    })
}

fn watercolor(img: &RgbaImage, r: u32) -> RgbaImage {
    posterize(&median(img, r), 6)
}

fn motion_blur(img: &RgbaImage, len: u32) -> RgbaImage {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let mut out = ImageBuffer::new(img.width(), img.height());
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0u32; 4];
            let mut n = 0;
            for k in 0..len as i64 {
                let sx = (x + k - len as i64 / 2).clamp(0, w - 1);
                let p = img.get_pixel(sx as u32, y as u32);
                for c in 0..4 {
                    acc[c] += p[c] as u32;
                }
                n += 1;
            }
            out.put_pixel(
                x as u32,
                y as u32,
                Rgba([
                    (acc[0] / n) as u8,
                    (acc[1] / n) as u8,
                    (acc[2] / n) as u8,
                    (acc[3] / n) as u8,
                ]),
            );
        }
    }
    out
}

fn radial_blur(img: &RgbaImage, amount: f32) -> RgbaImage {
    let (cx, cy) = (img.width() as f32 / 2.0, img.height() as f32 / 2.0);
    let steps = 8;
    let mut out = ImageBuffer::new(img.width(), img.height());
    for y in 0..img.height() {
        for x in 0..img.width() {
            let mut acc = [0.0f32; 4];
            for k in 0..steps {
                let t = 1.0 - amount * k as f32 / steps as f32;
                let p = bilinear(img, cx + (x as f32 - cx) * t, cy + (y as f32 - cy) * t);
                for c in 0..4 {
                    acc[c] += p[c] as f32;
                }
            }
            out.put_pixel(
                x,
                y,
                Rgba([
                    clamp8(acc[0] / steps as f32),
                    clamp8(acc[1] / steps as f32),
                    clamp8(acc[2] / steps as f32),
                    clamp8(acc[3] / steps as f32),
                ]),
            );
        }
    }
    out
}

fn smart_blur(img: &RgbaImage, r: u32, threshold: f32) -> RgbaImage {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let r = r as i64;
    let mut out = ImageBuffer::new(img.width(), img.height());
    for y in 0..h {
        for x in 0..w {
            let c0 = img.get_pixel(x as u32, y as u32);
            let mut acc = [0u32; 3];
            let mut n = 0;
            for dy in -r..=r {
                for dx in -r..=r {
                    let sx = (x + dx).clamp(0, w - 1);
                    let sy = (y + dy).clamp(0, h - 1);
                    let p = img.get_pixel(sx as u32, sy as u32);
                    if (luma(*p) - luma(*c0)).abs() < threshold {
                        for c in 0..3 {
                            acc[c] += p[c] as u32;
                        }
                        n += 1;
                    }
                }
            }
            let n = n.max(1);
            out.put_pixel(
                x as u32,
                y as u32,
                Rgba([
                    (acc[0] / n) as u8,
                    (acc[1] / n) as u8,
                    (acc[2] / n) as u8,
                    c0[3],
                ]),
            );
        }
    }
    out
}

fn diffuse_glow(img: &RgbaImage, a: f32) -> RgbaImage {
    let blurred = gaussian(img, 4.0 + a * 8.0);
    let mut out = ImageBuffer::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        let b = blurred.get_pixel(x, y);
        let mut o = [0u8; 4];
        for c in 0..3 {
            // Screen blend of the blurred highlights.
            let s = 1.0 - (1.0 - p[c] as f32 / 255.0) * (1.0 - (b[c] as f32 / 255.0) * a);
            o[c] = clamp8(s * 255.0);
        }
        o[3] = p[3];
        out.put_pixel(x, y, Rgba(o));
    }
    out
}

fn lens_flare(img: &RgbaImage, a: f32) -> RgbaImage {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let (fx, fy) = (w * 0.3, h * 0.3);
    let r = w.min(h) * (0.1 + a * 0.3);
    map_px(img, |x, y, p| {
        let d = ((x as f32 - fx).powi(2) + (y as f32 - fy).powi(2)).sqrt();
        let glow = (1.0 - d / r).max(0.0).powi(2);
        let ring = (1.0 - ((d - r * 2.2).abs() / (r * 0.15))).max(0.0) * 0.5;
        let k = glow + ring;
        Rgba([
            clamp8(p[0] as f32 + 255.0 * k),
            clamp8(p[1] as f32 + 240.0 * k),
            clamp8(p[2] as f32 + 200.0 * k),
            p[3],
        ])
    })
}

fn vignette(img: &RgbaImage, a: f32) -> RgbaImage {
    let (cx, cy) = (img.width() as f32 / 2.0, img.height() as f32 / 2.0);
    let rmax = (cx * cx + cy * cy).sqrt();
    map_px(img, |x, y, p| {
        let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / rmax;
        let k = 1.0 - (d.powi(2) * a * 1.5).min(1.0);
        Rgba([
            clamp8(p[0] as f32 * k),
            clamp8(p[1] as f32 * k),
            clamp8(p[2] as f32 * k),
            p[3],
        ])
    })
}

fn invert(img: &RgbaImage) -> RgbaImage {
    map_px(img, |_, _, p| {
        Rgba([255 - p[0], 255 - p[1], 255 - p[2], p[3]])
    })
}

fn posterize(img: &RgbaImage, levels: u8) -> RgbaImage {
    let l = levels.max(2) as f32;
    map_px(img, |_, _, p| {
        let f = |v: u8| clamp8(((v as f32 / 255.0 * (l - 1.0)).round() / (l - 1.0)) * 255.0);
        Rgba([f(p[0]), f(p[1]), f(p[2]), p[3]])
    })
}

fn psychedelic(img: &RgbaImage, a: f32) -> RgbaImage {
    map_px(img, |_, _, p| {
        let l = luma(p) / 255.0;
        let [r, g, b] =
            tracedraw_core::color::hsb_to_rgb((l * 720.0 * (0.5 + a)) % 360.0, 1.0, 1.0);
        Rgba([
            clamp8(r * 255.0),
            clamp8(g * 255.0),
            clamp8(b * 255.0),
            p[3],
        ])
    })
}

fn halftone(img: &RgbaImage, cell: u32) -> RgbaImage {
    let cell = cell.max(2);
    let mut out = ImageBuffer::from_pixel(img.width(), img.height(), Rgba([255, 255, 255, 255]));
    for cy in (0..img.height()).step_by(cell as usize) {
        for cx in (0..img.width()).step_by(cell as usize) {
            let mut sum = 0.0f32;
            let mut n = 0.0f32;
            for y in cy..(cy + cell).min(img.height()) {
                for x in cx..(cx + cell).min(img.width()) {
                    sum += luma(*img.get_pixel(x, y));
                    n += 1.0;
                }
            }
            let dark = 1.0 - sum / n.max(1.0) / 255.0;
            let r = dark.sqrt() * cell as f32 / 2.0;
            let (mx, my) = (cx as f32 + cell as f32 / 2.0, cy as f32 + cell as f32 / 2.0);
            for y in cy..(cy + cell).min(img.height()) {
                for x in cx..(cx + cell).min(img.width()) {
                    let d = ((x as f32 + 0.5 - mx).powi(2) + (y as f32 + 0.5 - my).powi(2)).sqrt();
                    if d <= r {
                        let a = img.get_pixel(x, y)[3];
                        out.put_pixel(x, y, Rgba([0, 0, 0, a]));
                    }
                }
            }
        }
    }
    out
}

fn edge_detect(img: &RgbaImage, color: bool) -> RgbaImage {
    let gx = [-1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0];
    let gy = [-1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0];
    let ex = convolve(img, &gx, 3, 0.0);
    let ey = convolve(img, &gy, 3, 0.0);
    let ex2 = convolve(img, &gx.map(|v| -v), 3, 0.0);
    let ey2 = convolve(img, &gy.map(|v| -v), 3, 0.0);
    let mut out = ImageBuffer::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        let mut o = [0u8; 4];
        let mut mag = 0.0f32;
        #[allow(clippy::needless_range_loop)]
        for c in 0..3 {
            let dx = ex.get_pixel(x, y)[c] as f32 + ex2.get_pixel(x, y)[c] as f32;
            let dy = ey.get_pixel(x, y)[c] as f32 + ey2.get_pixel(x, y)[c] as f32;
            let m = (dx * dx + dy * dy).sqrt();
            o[c] = clamp8(m);
            mag = mag.max(m);
        }
        if !color {
            let m = clamp8(mag);
            o = [m, m, m, 0];
        }
        o[3] = p[3];
        out.put_pixel(x, y, Rgba(o));
    }
    out
}

fn trace_contour(img: &RgbaImage, level: i32) -> RgbaImage {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let mut out = ImageBuffer::from_pixel(img.width(), img.height(), Rgba([255, 255, 255, 255]));
    for y in 0..h {
        for x in 0..w {
            let l = luma(*img.get_pixel(x as u32, y as u32)) as i32;
            let mut edge = false;
            for (dx, dy) in [(1i64, 0i64), (0, 1)] {
                let (nx, ny) = ((x + dx).min(w - 1), (y + dy).min(h - 1));
                let n = luma(*img.get_pixel(nx as u32, ny as u32)) as i32;
                if (l >= level) != (n >= level) {
                    edge = true;
                }
            }
            if edge {
                let a = img.get_pixel(x as u32, y as u32)[3];
                out.put_pixel(x as u32, y as u32, Rgba([0, 0, 0, a]));
            }
        }
    }
    out
}

fn median(img: &RgbaImage, r: u32) -> RgbaImage {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let r = r as i64;
    let mut out = ImageBuffer::new(img.width(), img.height());
    let mut buf: [Vec<u8>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for y in 0..h {
        for x in 0..w {
            for b in buf.iter_mut() {
                b.clear();
            }
            for dy in -r..=r {
                for dx in -r..=r {
                    let sx = (x + dx).clamp(0, w - 1);
                    let sy = (y + dy).clamp(0, h - 1);
                    let p = img.get_pixel(sx as u32, sy as u32);
                    for c in 0..3 {
                        buf[c].push(p[c]);
                    }
                }
            }
            let mut o = [0u8; 4];
            for c in 0..3 {
                buf[c].sort_unstable();
                o[c] = buf[c][buf[c].len() / 2];
            }
            o[3] = img.get_pixel(x as u32, y as u32)[3];
            out.put_pixel(x as u32, y as u32, Rgba(o));
        }
    }
    out
}

fn mosaic(img: &RgbaImage, cell: u32, grout: bool) -> RgbaImage {
    let cell = cell.max(2);
    let mut out = ImageBuffer::new(img.width(), img.height());
    for cy in (0..img.height()).step_by(cell as usize) {
        for cx in (0..img.width()).step_by(cell as usize) {
            let mut acc = [0u32; 4];
            let mut n = 0;
            for y in cy..(cy + cell).min(img.height()) {
                for x in cx..(cx + cell).min(img.width()) {
                    let p = img.get_pixel(x, y);
                    for c in 0..4 {
                        acc[c] += p[c] as u32;
                    }
                    n += 1;
                }
            }
            let n = n.max(1);
            let avg = Rgba([
                (acc[0] / n) as u8,
                (acc[1] / n) as u8,
                (acc[2] / n) as u8,
                (acc[3] / n) as u8,
            ]);
            for y in cy..(cy + cell).min(img.height()) {
                for x in cx..(cx + cell).min(img.width()) {
                    let edge = grout && (x == cx || y == cy);
                    out.put_pixel(
                        x,
                        y,
                        if edge {
                            Rgba([avg[0] / 2, avg[1] / 2, avg[2] / 2, avg[3]])
                        } else {
                            avg
                        },
                    );
                }
            }
        }
    }
    out
}

fn crystallize(img: &RgbaImage, size: u32) -> RgbaImage {
    // Voronoi cells from a jittered grid of seeds.
    let size = size.max(3);
    let mut seeds: Vec<(f32, f32, Rgba<u8>)> = Vec::new();
    let mut s = 7u32;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s % 1000) as f32 / 1000.0
    };
    for gy in (0..img.height() + size).step_by(size as usize) {
        for gx in (0..img.width() + size).step_by(size as usize) {
            let x = (gx as f32 + rnd() * size as f32).min(img.width() as f32 - 1.0);
            let y = (gy as f32 + rnd() * size as f32).min(img.height() as f32 - 1.0);
            seeds.push((x, y, sample(img, x, y)));
        }
    }
    let cols = (img.width() + size) / size + 1;
    map_px(img, |x, y, p| {
        let gx = x / size;
        let gy = y / size;
        let mut best = (f32::MAX, p);
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let (sx, sy) = (gx as i64 + dx, gy as i64 + dy);
                if sx < 0 || sy < 0 {
                    continue;
                }
                let idx = (sy as u32 * cols + sx as u32) as usize;
                if let Some((px, py, c)) = seeds.get(idx) {
                    let d = (x as f32 - px).powi(2) + (y as f32 - py).powi(2);
                    if d < best.0 {
                        best = (d, *c);
                    }
                }
            }
        }
        Rgba([best.1[0], best.1[1], best.1[2], p[3]])
    })
}

fn swirl(img: &RgbaImage, turns: f32) -> RgbaImage {
    let (cx, cy) = (img.width() as f32 / 2.0, img.height() as f32 / 2.0);
    let rmax = cx.min(cy);
    remap(img, |x, y| {
        let dx = x - cx;
        let dy = y - cy;
        let r = (dx * dx + dy * dy).sqrt();
        let a = dy.atan2(dx) + turns * (1.0 - r / rmax).max(0.0);
        (cx + r * a.cos(), cy + r * a.sin())
    })
}

fn bump(img: &RgbaImage, a: f32) -> RgbaImage {
    let (w, h) = (img.width() as i64, img.height() as i64);
    map_px(img, |x, y, p| {
        let l0 = luma(*img.get_pixel(
            ((x as i64) - 1).clamp(0, w - 1) as u32,
            ((y as i64) - 1).clamp(0, h - 1) as u32,
        ));
        let l1 = luma(*img.get_pixel(
            ((x as i64) + 1).clamp(0, w - 1) as u32,
            ((y as i64) + 1).clamp(0, h - 1) as u32,
        ));
        let k = 1.0 + (l0 - l1) / 255.0 * a * 2.0;
        Rgba([
            clamp8(p[0] as f32 * k),
            clamp8(p[1] as f32 * k),
            clamp8(p[2] as f32 * k),
            p[3],
        ])
    })
}

fn ripple(img: &RgbaImage, amplitude: f32, period: f32) -> RgbaImage {
    remap(img, |x, y| {
        let sx = x + amplitude * (y / period * std::f32::consts::TAU).sin();
        let sy = y + amplitude * (x / period * std::f32::consts::TAU).sin();
        (sx, sy)
    })
}

fn wind(img: &RgbaImage, len: u32) -> RgbaImage {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let mut out = img.clone();
    let mut seed = 99u32;
    for y in 0..h {
        for x in 0..w {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let l = (seed % len.max(1)) as i64;
            let p = *img.get_pixel(x as u32, y as u32);
            let bright = luma(p) > 128.0;
            if bright {
                for k in 1..=l {
                    let tx = x + k;
                    if tx < w {
                        let q = out.get_pixel_mut(tx as u32, y as u32);
                        let t = k as f32 / (l + 1) as f32;
                        for c in 0..3 {
                            q[c] = clamp8(q[c] as f32 * t + p[c] as f32 * (1.0 - t));
                        }
                    }
                }
            }
        }
    }
    out
}

fn tile(img: &RgbaImage, n: u32) -> RgbaImage {
    let n = n.max(1) as f32;
    let (w, h) = (img.width() as f32, img.height() as f32);
    remap(img, |x, y| ((x * n) % w, (y * n) % h))
}

fn add_noise(img: &RgbaImage, amount: f32) -> RgbaImage {
    let seed = std::cell::Cell::new(2024u32);
    map_px(img, |_, _, p| {
        let mut s = seed.get();
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        seed.set(s);
        let n = ((s % 512) as f32 - 256.0) * amount;
        Rgba([
            clamp8(p[0] as f32 + n),
            clamp8(p[1] as f32 + n),
            clamp8(p[2] as f32 + n),
            p[3],
        ])
    })
}

fn sharpen(img: &RgbaImage, k: f32) -> RgbaImage {
    let kernel = [0.0, -k, 0.0, -k, 1.0 + 4.0 * k, -k, 0.0, -k, 0.0];
    convolve(img, &kernel, 3, 0.0)
}

fn unsharp(img: &RgbaImage, sigma: f32, amount: f32) -> RgbaImage {
    let blurred = gaussian(img, sigma);
    let mut out = ImageBuffer::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        let b = blurred.get_pixel(x, y);
        let mut o = [0u8; 4];
        for c in 0..3 {
            o[c] = clamp8(p[c] as f32 + (p[c] as f32 - b[c] as f32) * amount);
        }
        o[3] = p[3];
        out.put_pixel(x, y, Rgba(o));
    }
    out
}

fn high_pass(img: &RgbaImage, sigma: f32) -> RgbaImage {
    let blurred = gaussian(img, sigma);
    let mut out = ImageBuffer::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        let b = blurred.get_pixel(x, y);
        let mut o = [0u8; 4];
        for c in 0..3 {
            o[c] = clamp8(p[c] as f32 - b[c] as f32 + 128.0);
        }
        o[3] = p[3];
        out.put_pixel(x, y, Rgba(o));
    }
    out
}

fn texture(img: &RgbaImage, period: u32, strength: f32) -> RgbaImage {
    map_px(img, |x, y, p| {
        let n = tracedraw_render::texture_value(
            tracedraw_core::TextureKind::Noise,
            x as f64 / period as f64,
            y as f64 / period as f64,
            3,
        );
        let k = 1.0 + (n as f32 - 0.5) * strength;
        Rgba([
            clamp8(p[0] as f32 * k),
            clamp8(p[1] as f32 * k),
            clamp8(p[2] as f32 * k),
            p[3],
        ])
    })
}

fn plastic(img: &RgbaImage, a: f32) -> RgbaImage {
    let e = emboss(img, a);
    let mut out = ImageBuffer::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        let l = e.get_pixel(x, y)[0] as f32 / 128.0;
        out.put_pixel(
            x,
            y,
            Rgba([
                clamp8(p[0] as f32 * l),
                clamp8(p[1] as f32 * l),
                clamp8(p[2] as f32 * l),
                p[3],
            ]),
        );
    }
    out
}

fn deinterlace(img: &RgbaImage) -> RgbaImage {
    let mut out = img.clone();
    for y in (1..img.height()).step_by(2) {
        for x in 0..img.width() {
            let above = img.get_pixel(x, y - 1);
            let below = img.get_pixel(x, (y + 1).min(img.height() - 1));
            let mut o = [0u8; 4];
            for c in 0..4 {
                o[c] = ((above[c] as u16 + below[c] as u16) / 2) as u8;
            }
            out.put_pixel(x, y, Rgba(o));
        }
    }
    out
}

fn threshold(img: &RgbaImage, t: u8) -> RgbaImage {
    map_px(img, |_, _, p| {
        let v = if luma(p) >= t as f32 { 255 } else { 0 };
        Rgba([v, v, v, p[3]])
    })
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
    map_px(img, |_, _, p| {
        let hit = colors.iter().any(|c| {
            (p[0] as i32 - c[0] as i32).abs() <= tolerance as i32
                && (p[1] as i32 - c[1] as i32).abs() <= tolerance as i32
                && (p[2] as i32 - c[2] as i32).abs() <= tolerance as i32
        });
        if hit {
            Rgba([p[0], p[1], p[2], 0])
        } else {
            p
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
    fn invert_and_threshold() {
        let g = gradient();
        let inv = apply(&g, Fx::Invert, 50.0);
        assert_eq!(inv.get_pixel(0, 0)[0], 255);
        let t = apply(&g, Fx::Threshold, 50.0);
        assert_eq!(t.get_pixel(0, 0)[0], 0);
        assert_eq!(t.get_pixel(31, 0)[0], 255);
    }

    #[test]
    fn every_effect_keeps_size() {
        let g = gradient();
        for (_, group) in GROUPS {
            for fx in group.iter() {
                let out = apply(&g, fx.fx, 60.0);
                assert_eq!(out.dimensions(), g.dimensions(), "{:?}", fx.fx);
            }
        }
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
