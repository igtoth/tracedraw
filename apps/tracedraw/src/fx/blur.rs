//! Effects > Blur.

use super::util::*;
use super::*;
use image::{Rgba, RgbaImage};

pub const EFFECTS: &[EffectSpec] = &[
    EffectSpec {
        id: "directional_smooth",
        params: &[range("percentage", 1.0, 100.0, 50.0, 1.0)],
        apply: directional_smooth,
        reach: no_reach,
    },
    EffectSpec {
        id: "gaussian_blur",
        params: &[range("radius", 0.1, 250.0, 1.0, 0.1)],
        apply: gaussian_blur,
        reach: gaussian_reach,
    },
    EffectSpec {
        id: "jagged_edges",
        params: &[
            range("width", 1.0, 10.0, 1.0, 1.0),
            range("height", 1.0, 10.0, 1.0, 1.0),
        ],
        apply: jagged_edges,
        reach: no_reach,
    },
    EffectSpec {
        id: "low_pass",
        params: &[
            range("percentage", 1.0, 100.0, 50.0, 1.0),
            range("radius", 1.0, 100.0, 3.0, 1.0),
        ],
        apply: low_pass,
        reach: low_pass_reach,
    },
    EffectSpec {
        id: "motion_blur",
        params: &[
            range("distance", 1.0, 200.0, 10.0, 1.0),
            angle("direction", 0.0),
        ],
        apply: motion_blur,
        reach: motion_reach,
    },
    EffectSpec {
        id: "radial_blur",
        params: &[
            range("amount", 1.0, 100.0, 10.0, 1.0),
            range("center_x", 0.0, 100.0, 50.0, 1.0),
            range("center_y", 0.0, 100.0, 50.0, 1.0),
        ],
        apply: radial_blur,
        reach: no_reach,
    },
    EffectSpec {
        id: "smooth",
        params: &[range("percentage", 1.0, 100.0, 50.0, 1.0)],
        apply: smooth,
        reach: no_reach,
    },
    EffectSpec {
        id: "soften",
        params: &[range("percentage", 1.0, 100.0, 50.0, 1.0)],
        apply: soften,
        reach: no_reach,
    },
    EffectSpec {
        id: "zoom",
        params: &[
            range("amount", 1.0, 100.0, 20.0, 1.0),
            range("center_x", 0.0, 100.0, 50.0, 1.0),
            range("center_y", 0.0, 100.0, 50.0, 1.0),
        ],
        apply: zoom,
        reach: no_reach,
    },
];

fn gaussian_reach(p: &P, _: u32, _: u32) -> u32 {
    (p.f32("radius") * 2.0).ceil() as u32
}

fn low_pass_reach(p: &P, _: u32, _: u32) -> u32 {
    // The blur's sigma is the radius: three sigma hold nearly all of it.
    p.u("radius") * 3
}

fn motion_reach(p: &P, _: u32, _: u32) -> u32 {
    p.u("distance")
}

/// Smooths along edges: each pixel moves toward the average of the
/// neighbours most like it.
pub fn directional_smooth(img: &RgbaImage, p: &P) -> RgbaImage {
    let t = p.f32("percentage") / 100.0;
    map_px(img, |x, y, q| {
        let dirs: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];
        let mut best = (f32::MAX, [0f32; 3]);
        for (dx, dy) in dirs {
            let a = sample(img, x as f32 + dx as f32, y as f32 + dy as f32);
            let b = sample(img, x as f32 - dx as f32, y as f32 - dy as f32);
            let d = (luma(&a) - luma(&b)).abs();
            if d < best.0 {
                best = (
                    d,
                    std::array::from_fn(|c| (a[c] as f32 + b[c] as f32 + q[c] as f32) / 3.0),
                );
            }
        }
        Rgba([
            clamp8(q[0] as f32 + (best.1[0] - q[0] as f32) * t),
            clamp8(q[1] as f32 + (best.1[1] - q[1] as f32) * t),
            clamp8(q[2] as f32 + (best.1[2] - q[2] as f32) * t),
            q[3],
        ])
    })
}

pub fn gaussian_blur(img: &RgbaImage, p: &P) -> RgbaImage {
    // The radius is the target design's: about two sigma.
    gaussian(img, p.f32("radius") / 2.0)
}

/// Smooths stair steps: a small blur weighted toward the pixels that
/// differ most.
pub fn jagged_edges(img: &RgbaImage, p: &P) -> RgbaImage {
    let avg = box_average(img, p.u("width").max(1), p.u("height").max(1));
    map_px(img, |x, y, q| {
        let a = avg.get_pixel(x, y);
        let diff = (luma(q) - luma(a)).abs() / 255.0;
        let t = (diff * 3.0).min(1.0) * 0.8 + 0.2;
        lerp_premultiplied(q, a, t)
    })
}

pub fn low_pass(img: &RgbaImage, p: &P) -> RgbaImage {
    let b = gaussian(img, p.f32("radius"));
    mix(img, &b, p.f32("percentage") / 100.0)
}

pub fn motion_blur(img: &RgbaImage, p: &P) -> RgbaImage {
    let d = p.f32("distance");
    let a = p.f32("direction").to_radians();
    let (dx, dy) = (a.cos(), -a.sin());
    // One sample per pixel of the streak, at most 96 (longer streaks
    // space them out a little).
    let n = d.ceil().clamp(1.0, 96.0) as i32;
    let step = d.max(1.0) / n as f32;
    map_px(img, |x, y, _| {
        let mut acc = [0f32; 4];
        for k in 0..=n {
            let t = (k as f32 - n as f32 / 2.0) * step;
            let s = bilinear(img, x as f32 + dx * t, y as f32 + dy * t, true);
            add_premultiplied(&mut acc, &s);
        }
        unpremultiply(acc, (n + 1) as f32)
    })
}

pub fn radial_blur(img: &RgbaImage, p: &P) -> RgbaImage {
    let amount = p.f32("amount") / 100.0 * 0.5;
    let (w, h) = (img.width() as f32, img.height() as f32);
    let (cx, cy) = (p.f32("center_x") / 100.0 * w, p.f32("center_y") / 100.0 * h);
    map_px(img, |x, y, _| {
        let (dx, dy) = (x as f32 - cx, y as f32 - cy);
        let r = (dx * dx + dy * dy).sqrt();
        let a0 = dy.atan2(dx);
        let spread = amount;
        let n = ((r * spread).ceil() as i32).clamp(1, 32);
        let mut acc = [0f32; 4];
        for k in -n..=n {
            let a = a0 + spread * k as f32 / n as f32;
            let s = bilinear(img, cx + r * a.cos(), cy + r * a.sin(), false);
            add_premultiplied(&mut acc, &s);
        }
        unpremultiply(acc, (2 * n + 1) as f32)
    })
}

pub fn smooth(img: &RgbaImage, p: &P) -> RgbaImage {
    mix(img, &box_blur(img, 1), p.f32("percentage") / 100.0)
}

pub fn soften(img: &RgbaImage, p: &P) -> RgbaImage {
    mix(img, &gaussian(img, 0.7), p.f32("percentage") / 100.0)
}

pub fn zoom(img: &RgbaImage, p: &P) -> RgbaImage {
    let amount = p.f32("amount") / 100.0 * 0.3;
    let (w, h) = (img.width() as f32, img.height() as f32);
    let (cx, cy) = (p.f32("center_x") / 100.0 * w, p.f32("center_y") / 100.0 * h);
    map_px(img, |x, y, _| {
        let (dx, dy) = (x as f32 - cx, y as f32 - cy);
        let n = (((dx * dx + dy * dy).sqrt() * amount).ceil() as i32).clamp(1, 32);
        let mut acc = [0f32; 4];
        for k in 0..=n {
            let t = 1.0 - amount * k as f32 / n as f32;
            let s = bilinear(img, cx + dx * t, cy + dy * t, false);
            add_premultiplied(&mut acc, &s);
        }
        unpremultiply(acc, (n + 1) as f32)
    })
}
