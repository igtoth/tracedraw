//! Effects > Noise and Sharpen.

use super::util::*;
use super::*;
use image::{Rgba, RgbaImage};

pub const NOISE: &[EffectSpec] = &[
    EffectSpec {
        id: "add_noise",
        params: &[
            choice("noise_type", &["gaussian", "spike", "uniform"], 0),
            range("level", 0.0, 100.0, 50.0, 1.0),
            range("density", 0.0, 100.0, 50.0, 1.0),
            choice("color_mode", &["intensity", "random", "single"], 0),
            color("noise_color", 0x000000),
        ],
        apply: add_noise,
        reach: no_reach,
    },
    EffectSpec {
        id: "maximum",
        params: &[
            range("percentage", 1.0, 100.0, 100.0, 1.0),
            range("radius", 1.0, 20.0, 1.0, 1.0),
        ],
        apply: maximum,
        reach: no_reach,
    },
    EffectSpec {
        id: "median",
        params: &[range("radius", 1.0, 20.0, 1.0, 1.0)],
        apply: median,
        reach: no_reach,
    },
    EffectSpec {
        id: "minimum",
        params: &[
            range("percentage", 1.0, 100.0, 100.0, 1.0),
            range("radius", 1.0, 20.0, 1.0, 1.0),
        ],
        apply: minimum,
        reach: no_reach,
    },
    EffectSpec {
        id: "remove_moire",
        params: &[
            range("amount", 1.0, 100.0, 50.0, 1.0),
            choice("optimize", &["speed", "quality"], 1),
        ],
        apply: remove_moire,
        reach: no_reach,
    },
    EffectSpec {
        id: "remove_noise",
        params: &[
            check("auto", true),
            range("threshold", 0.0, 255.0, 20.0, 1.0),
        ],
        apply: remove_noise,
        reach: no_reach,
    },
];

pub const SHARPEN: &[EffectSpec] = &[
    EffectSpec {
        id: "adaptive_unsharp",
        params: &[range("percentage", 1.0, 100.0, 50.0, 1.0)],
        apply: adaptive_unsharp,
        reach: no_reach,
    },
    EffectSpec {
        id: "directional_sharpen",
        params: &[range("percentage", 1.0, 100.0, 50.0, 1.0)],
        apply: directional_sharpen,
        reach: no_reach,
    },
    EffectSpec {
        id: "high_pass",
        params: &[
            range("percentage", 1.0, 100.0, 50.0, 1.0),
            range("radius", 1.0, 20.0, 3.0, 1.0),
        ],
        apply: high_pass,
        reach: no_reach,
    },
    EffectSpec {
        id: "sharpen",
        params: &[
            range("edge_level", 1.0, 100.0, 50.0, 1.0),
            range("threshold", 0.0, 255.0, 0.0, 1.0),
            check("preserve_colors", false),
        ],
        apply: sharpen,
        reach: no_reach,
    },
    EffectSpec {
        id: "unsharp_mask",
        params: &[
            range("percentage", 1.0, 500.0, 100.0, 1.0),
            range("radius", 0.1, 100.0, 1.0, 0.1),
            range("threshold", 0.0, 255.0, 0.0, 1.0),
        ],
        apply: unsharp_mask,
        reach: no_reach,
    },
];

pub fn add_noise(img: &RgbaImage, p: &P) -> RgbaImage {
    let kind = p.i("noise_type");
    let level = p.f32("level") / 100.0 * 255.0;
    let density = p.f32("density") / 100.0;
    let mode = p.i("color_mode");
    let col = p.rgb("noise_color");
    map_px(img, |x, y, q| {
        if hash01(x, y, 501) > density {
            return *q;
        }
        let n = |s: u32| -> f32 {
            let u = hash01(x, y, s);
            match kind {
                // Sum of uniforms approximates a normal distribution.
                0 => {
                    let v = (u + hash01(x, y, s + 7) + hash01(x, y, s + 13) - 1.5) / 1.5;
                    v * level
                }
                1 => {
                    if u > 0.92 {
                        level
                    } else if u < 0.08 {
                        -level
                    } else {
                        0.0
                    }
                }
                _ => (u - 0.5) * 2.0 * level,
            }
        };
        match mode {
            0 => {
                let d = n(502);
                Rgba([
                    clamp8(q[0] as f32 + d),
                    clamp8(q[1] as f32 + d),
                    clamp8(q[2] as f32 + d),
                    q[3],
                ])
            }
            1 => Rgba([
                clamp8(q[0] as f32 + n(503)),
                clamp8(q[1] as f32 + n(504)),
                clamp8(q[2] as f32 + n(505)),
                q[3],
            ]),
            _ => {
                let t = (n(506).abs() / 255.0).min(1.0);
                Rgba([
                    clamp8(q[0] as f32 + (col[0] as f32 - q[0] as f32) * t),
                    clamp8(q[1] as f32 + (col[1] as f32 - q[1] as f32) * t),
                    clamp8(q[2] as f32 + (col[2] as f32 - q[2] as f32) * t),
                    q[3],
                ])
            }
        }
    })
}

pub fn maximum(img: &RgbaImage, p: &P) -> RgbaImage {
    mix(
        img,
        &rank(img, p.u("radius").max(1), 1.0),
        p.f32("percentage") / 100.0,
    )
}

pub fn median(img: &RgbaImage, p: &P) -> RgbaImage {
    rank(img, p.u("radius").max(1), 0.5)
}

pub fn minimum(img: &RgbaImage, p: &P) -> RgbaImage {
    mix(
        img,
        &rank(img, p.u("radius").max(1), 0.0),
        p.f32("percentage") / 100.0,
    )
}

/// Moiré is a beat of fine patterns: smooth them away, keeping edges.
pub fn remove_moire(img: &RgbaImage, p: &P) -> RgbaImage {
    let a = p.f32("amount") / 100.0;
    let quality = p.i("optimize") == 1;
    let sm = if quality {
        gaussian(&rank(img, 1, 0.5), 0.6 + a * 1.5)
    } else {
        box_blur(img, 1 + (a * 2.0) as u32)
    };
    mix(img, &sm, 0.4 + a * 0.6)
}

pub fn remove_noise(img: &RgbaImage, p: &P) -> RgbaImage {
    let med = rank(img, 1, 0.5);
    let t = if p.b("auto") {
        24.0
    } else {
        p.f32("threshold")
    };
    map_px(img, |x, y, q| {
        let m = med.get_pixel(x, y);
        let d = (0..3)
            .map(|c| (q[c] as f32 - m[c] as f32).abs())
            .fold(0.0, f32::max);
        if d > t {
            Rgba([m[0], m[1], m[2], q[3]])
        } else {
            *q
        }
    })
}

/// Sharpen by `k` times the difference from a blur of `sigma`, where the
/// difference is above `threshold` levels.
fn unsharp(img: &RgbaImage, sigma: f32, k: f32, threshold: f32, preserve: bool) -> RgbaImage {
    let b = gaussian(img, sigma);
    map_px(img, |x, y, q| {
        let s = b.get_pixel(x, y);
        let d: [f32; 3] = std::array::from_fn(|c| q[c] as f32 - s[c] as f32);
        if d.iter().fold(0f32, |m, v| m.max(v.abs())) < threshold {
            return *q;
        }
        if preserve {
            // The same change on every channel: hues stay.
            let dl = 0.299 * d[0] + 0.587 * d[1] + 0.114 * d[2];
            return Rgba([
                clamp8(q[0] as f32 + dl * k),
                clamp8(q[1] as f32 + dl * k),
                clamp8(q[2] as f32 + dl * k),
                q[3],
            ]);
        }
        Rgba([
            clamp8(q[0] as f32 + d[0] * k),
            clamp8(q[1] as f32 + d[1] * k),
            clamp8(q[2] as f32 + d[2] * k),
            q[3],
        ])
    })
}

/// Stronger where the neighbourhood varies more.
pub fn adaptive_unsharp(img: &RgbaImage, p: &P) -> RgbaImage {
    let k = p.f32("percentage") / 100.0 * 1.5;
    let b = gaussian(img, 1.0);
    map_px(img, |x, y, q| {
        let s = b.get_pixel(x, y);
        let var = (luma(q) - luma(s)).abs() / 255.0;
        let kk = k * (0.3 + var * 3.0).min(1.5);
        Rgba([
            clamp8(q[0] as f32 + (q[0] as f32 - s[0] as f32) * kk),
            clamp8(q[1] as f32 + (q[1] as f32 - s[1] as f32) * kk),
            clamp8(q[2] as f32 + (q[2] as f32 - s[2] as f32) * kk),
            q[3],
        ])
    })
}

/// Sharpens across the strongest edge direction only, so flat areas keep
/// no grain.
pub fn directional_sharpen(img: &RgbaImage, p: &P) -> RgbaImage {
    let k = p.f32("percentage") / 100.0 * 1.2;
    map_px(img, |x, y, q| {
        let dirs: [(f32, f32); 4] = [(1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, -1.0)];
        let mut best = (0f32, [0f32; 3]);
        for (dx, dy) in dirs {
            let a = sample(img, x as f32 + dx, y as f32 + dy);
            let b = sample(img, x as f32 - dx, y as f32 - dy);
            let d = (luma(&a) - luma(&b)).abs();
            if d > best.0 {
                best = (
                    d,
                    std::array::from_fn(|c| q[c] as f32 - (a[c] as f32 + b[c] as f32) / 2.0),
                );
            }
        }
        Rgba([
            clamp8(q[0] as f32 + best.1[0] * k),
            clamp8(q[1] as f32 + best.1[1] * k),
            clamp8(q[2] as f32 + best.1[2] * k),
            q[3],
        ])
    })
}

/// Detail on mid gray, mixed in by the percentage.
pub fn high_pass(img: &RgbaImage, p: &P) -> RgbaImage {
    let b = gaussian(img, p.f32("radius"));
    let hp = map_px(img, |x, y, q| {
        let s = b.get_pixel(x, y);
        Rgba([
            clamp8(128.0 + q[0] as f32 - s[0] as f32),
            clamp8(128.0 + q[1] as f32 - s[1] as f32),
            clamp8(128.0 + q[2] as f32 - s[2] as f32),
            q[3],
        ])
    });
    mix(img, &hp, p.f32("percentage") / 100.0)
}

pub fn sharpen(img: &RgbaImage, p: &P) -> RgbaImage {
    unsharp(
        img,
        0.8,
        p.f32("edge_level") / 100.0 * 2.0,
        p.f32("threshold"),
        p.b("preserve_colors"),
    )
}

pub fn unsharp_mask(img: &RgbaImage, p: &P) -> RgbaImage {
    unsharp(
        img,
        p.f32("radius").max(0.1),
        p.f32("percentage") / 100.0,
        p.f32("threshold"),
        false,
    )
}
