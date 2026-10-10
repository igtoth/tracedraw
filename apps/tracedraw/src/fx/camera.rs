//! Effects > Camera: photographic filters and old photo looks.

use super::util::*;
use super::*;
use image::{Rgba, RgbaImage};

pub const EFFECTS: &[EffectSpec] = &[
    EffectSpec {
        id: "colorize",
        params: &[
            range("hue", 0.0, 360.0, 30.0, 1.0),
            range("saturation", 0.0, 100.0, 50.0, 1.0),
        ],
        apply: colorize,
        reach: no_reach,
    },
    EffectSpec {
        id: "diffuse",
        params: &[range("level", 1.0, 100.0, 20.0, 1.0)],
        apply: diffuse,
        reach: no_reach,
    },
    EffectSpec {
        id: "photo_filter",
        params: &[
            color("filter_color", 0xEC8A00),
            range("density", 1.0, 100.0, 25.0, 1.0),
            check("preserve_luminosity", true),
        ],
        apply: photo_filter,
        reach: no_reach,
    },
    EffectSpec {
        id: "sepia_toning",
        params: &[range("level", 0.0, 100.0, 50.0, 1.0)],
        apply: sepia,
        reach: no_reach,
    },
    EffectSpec {
        id: "vintage_photo",
        params: &[
            choice(
                "era",
                &[
                    "era_1839", "era_1876", "era_1925", "era_1945", "era_1955", "era_1960",
                    "era_1965",
                ],
                1,
            ),
            range("intensity", 0.0, 100.0, 70.0, 1.0),
        ],
        apply: vintage_photo,
        reach: no_reach,
    },
];

pub fn colorize(img: &RgbaImage, p: &P) -> RgbaImage {
    let h = p.f32("hue");
    let s = p.f32("saturation") / 100.0;
    map_px(img, |_, _, q| {
        let l = luma(q) / 255.0;
        let (r, g, b) = hsl_to_rgb(h, s, l);
        Rgba([
            clamp8(r * 255.0),
            clamp8(g * 255.0),
            clamp8(b * 255.0),
            q[3],
        ])
    })
}

/// Pixels swapped with near neighbours, then softened: a diffusion filter.
pub fn diffuse(img: &RgbaImage, p: &P) -> RgbaImage {
    let r = 1.0 + p.f32("level") / 100.0 * 4.0;
    let scattered = map_px(img, |x, y, _| {
        let dx = (hash01(x, y, 201) - 0.5) * 2.0 * r;
        let dy = (hash01(x, y, 202) - 0.5) * 2.0 * r;
        sample(img, x as f32 + dx, y as f32 + dy)
    });
    mix(&scattered, &gaussian(&scattered, r * 0.5), 0.5)
}

pub fn photo_filter(img: &RgbaImage, p: &P) -> RgbaImage {
    let f = p.rgb("filter_color").map(|v| v as f32 / 255.0);
    let d = p.f32("density") / 100.0;
    let keep = p.b("preserve_luminosity");
    map_px(img, |_, _, q| {
        let c = [
            q[0] as f32 / 255.0,
            q[1] as f32 / 255.0,
            q[2] as f32 / 255.0,
        ];
        let l0 = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
        let mut o: [f32; 3] = std::array::from_fn(|k| c[k] * (1.0 - d) + c[k] * f[k] * d * 2.0);
        if keep {
            let l1 = 0.299 * o[0] + 0.587 * o[1] + 0.114 * o[2];
            if l1 > 1e-6 {
                for v in o.iter_mut() {
                    *v *= l0 / l1;
                }
            }
        }
        Rgba([
            clamp8(o[0] * 255.0),
            clamp8(o[1] * 255.0),
            clamp8(o[2] * 255.0),
            q[3],
        ])
    })
}

pub fn sepia(img: &RgbaImage, p: &P) -> RgbaImage {
    let t = p.f32("level") / 100.0;
    map_px(img, |_, _, q| {
        let l = luma(q);
        let s = [l * 1.07 + 20.0 * t, l * 0.87 + 8.0 * t, l * 0.6];
        let g = [l, l, l];
        Rgba([
            clamp8(g[0] + (s[0] - g[0]) * t),
            clamp8(g[1] + (s[1] - g[1]) * t),
            clamp8(g[2] + (s[2] - g[2]) * t),
            q[3],
        ])
    })
}

/// Old photographic processes: tone, contrast, fading, grain and a
/// vignette per era.
pub fn vintage_photo(img: &RgbaImage, p: &P) -> RgbaImage {
    let era = p.i("era");
    let t = p.f32("intensity") / 100.0;
    // (tint, contrast, fade, grain, vignette)
    let (tint, contrast, fade, grain, vig): ([f32; 3], f32, f32, f32, f32) = match era {
        0 => ([1.0, 0.92, 0.75], 1.3, 0.1, 0.25, 0.6),
        1 => ([1.05, 0.9, 0.7], 1.15, 0.15, 0.12, 0.5),
        2 => ([0.95, 0.95, 0.92], 1.25, 0.05, 0.1, 0.35),
        3 => ([0.9, 0.9, 0.9], 1.4, 0.0, 0.15, 0.3),
        4 => ([1.0, 1.0, 1.05], 1.1, 0.1, 0.06, 0.25),
        5 => ([1.08, 0.98, 0.85], 0.9, 0.2, 0.05, 0.2),
        _ => ([1.12, 0.95, 0.8], 0.95, 0.25, 0.04, 0.15),
    };
    let (w, h) = (img.width() as f32, img.height() as f32);
    map_px(img, |x, y, q| {
        let l = luma(q) / 255.0;
        let mut v = ((l - 0.5) * contrast + 0.5) * (1.0 - fade) + fade * 0.85;
        v += (hash01(x, y, 211) - 0.5) * grain;
        let (dx, dy) = ((x as f32 / w - 0.5) * 2.0, (y as f32 / h - 0.5) * 2.0);
        v *= 1.0 - vig * ((dx * dx + dy * dy) / 2.0).powf(1.5);
        let o: [f32; 3] = std::array::from_fn(|k| (v * tint[k]).clamp(0.0, 1.0) * 255.0);
        Rgba([
            clamp8(q[0] as f32 + (o[0] - q[0] as f32) * t),
            clamp8(q[1] as f32 + (o[1] - q[1] as f32) * t),
            clamp8(q[2] as f32 + (o[2] - q[2] as f32) * t),
            q[3],
        ])
    })
}
