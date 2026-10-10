//! Effects > Color Transform and Contour.

use super::util::*;
use super::*;
use image::{Rgba, RgbaImage};

pub const TRANSFORM: &[EffectSpec] = &[
    EffectSpec {
        id: "bit_planes",
        params: &[
            check("apply_to_all", true),
            range("red", 0.0, 7.0, 0.0, 1.0),
            range("green", 0.0, 7.0, 0.0, 1.0),
            range("blue", 0.0, 7.0, 0.0, 1.0),
        ],
        apply: bit_planes,
        reach: no_reach,
    },
    EffectSpec {
        id: "halftone",
        params: &[
            range("max_dot_radius", 2.0, 20.0, 4.0, 1.0),
            angle("cyan_angle", 105.0),
            angle("magenta_angle", 75.0),
            angle("yellow_angle", 90.0),
            angle("black_angle", 45.0),
        ],
        apply: halftone,
        reach: no_reach,
    },
    EffectSpec {
        id: "psychedelic",
        params: &[range("level", 0.0, 255.0, 128.0, 1.0)],
        apply: psychedelic,
        reach: no_reach,
    },
    EffectSpec {
        id: "solarize",
        params: &[range("level", 0.0, 255.0, 128.0, 1.0)],
        apply: solarize,
        reach: no_reach,
    },
];

pub const CONTOUR: &[EffectSpec] = &[
    EffectSpec {
        id: "edge_detect",
        params: &[
            choice("background", &["white", "black", "other"], 0),
            color("other_color", 0x808080),
            range("sensitivity", 1.0, 10.0, 5.0, 1.0),
        ],
        apply: edge_detect,
        reach: no_reach,
    },
    EffectSpec {
        id: "find_edges",
        params: &[
            choice("edge_type", &["soft", "solid"], 0),
            range("level", 0.0, 100.0, 50.0, 1.0),
        ],
        apply: find_edges,
        reach: no_reach,
    },
    EffectSpec {
        id: "trace_contour",
        params: &[
            range("level", 0.0, 255.0, 128.0, 1.0),
            choice("edge_type", &["lower", "upper"], 0),
        ],
        apply: trace_contour,
        reach: no_reach,
    },
];

pub fn bit_planes(img: &RgbaImage, p: &P) -> RgbaImage {
    let all = p.b("apply_to_all");
    let r = p.u("red").min(7);
    let planes = if all {
        [r; 3]
    } else {
        [r, p.u("green").min(7), p.u("blue").min(7)]
    };
    map_px(img, |_, _, q| {
        let f = |v: u8, k: u32| if (v >> (7 - k)) & 1 == 1 { 255 } else { 0 };
        Rgba([
            f(q[0], planes[0]),
            f(q[1], planes[1]),
            f(q[2], planes[2]),
            q[3],
        ])
    })
}

/// Colour halftone: cyan, magenta, yellow and black screens of round
/// dots at their angles, printed over one another.
pub fn halftone(img: &RgbaImage, p: &P) -> RgbaImage {
    let r = p.f32("max_dot_radius").max(1.0);
    let cell = r * 2.0;
    let angles = [
        p.f32("cyan_angle"),
        p.f32("magenta_angle"),
        p.f32("yellow_angle"),
        p.f32("black_angle"),
    ]
    .map(|a| a.to_radians());
    map_px(img, |x, y, q| {
        let mut ink = [0f32; 4];
        for (i, a) in angles.iter().enumerate() {
            let (s, co) = a.sin_cos();
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let u = fx * co + fy * s;
            let v = -fx * s + fy * co;
            // The centre of this pixel's cell, back in image space.
            let (cu, cv) = ((u / cell).floor() * cell + r, (v / cell).floor() * cell + r);
            let (sx, sy) = (cu * co - cv * s, cu * s + cv * co);
            let sp = sample(img, sx, sy);
            let sc = [
                sp[0] as f32 / 255.0,
                sp[1] as f32 / 255.0,
                sp[2] as f32 / 255.0,
            ];
            let sk = 1.0 - sc[0].max(sc[1]).max(sc[2]);
            let sden = (1.0 - sk).max(1e-6);
            let amount = if i == 3 {
                sk
            } else {
                (1.0 - sc[i] - sk) / sden
            };
            let dot = amount.clamp(0.0, 1.0).sqrt() * r * std::f32::consts::FRAC_2_SQRT_PI;
            let d = ((u - cu).powi(2) + (v - cv).powi(2)).sqrt();
            ink[i] = (dot - d + 0.5).clamp(0.0, 1.0);
        }
        let o: [f32; 3] = std::array::from_fn(|ch| (1.0 - ink[ch]) * (1.0 - ink[3]));
        Rgba([
            clamp8(o[0] * 255.0),
            clamp8(o[1] * 255.0),
            clamp8(o[2] * 255.0),
            q[3],
        ])
    })
}

pub fn psychedelic(img: &RgbaImage, p: &P) -> RgbaImage {
    let l = p.f32("level");
    map_px(img, |_, _, q| {
        let f = |v: u8, k: f32| {
            let t = (v as f32 + l * k).rem_euclid(256.0);
            clamp8(if t > 127.0 {
                255.0 - (t - 128.0) * 2.0
            } else {
                t * 2.0
            })
        };
        Rgba([f(q[0], 1.0), f(q[1], 2.0), f(q[2], 3.0), q[3]])
    })
}

pub fn solarize(img: &RgbaImage, p: &P) -> RgbaImage {
    let t = p.f32("level");
    let mut lut = [0u8; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        *v = if i as f32 >= 255.0 - t {
            255 - i as u8
        } else {
            i as u8
        };
    }
    map_lut(img, &lut)
}

fn sobel(img: &RgbaImage) -> Vec<f32> {
    let (w, h) = img.dimensions();
    let l = heights(img);
    let at = |x: i64, y: i64| {
        l[(y.clamp(0, h as i64 - 1) as u32 * w + x.clamp(0, w as i64 - 1) as u32) as usize]
    };
    let mut out = vec![0.0; l.len()];
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            let gx = at(x + 1, y - 1) + 2.0 * at(x + 1, y) + at(x + 1, y + 1)
                - at(x - 1, y - 1)
                - 2.0 * at(x - 1, y)
                - at(x - 1, y + 1);
            let gy = at(x - 1, y + 1) + 2.0 * at(x, y + 1) + at(x + 1, y + 1)
                - at(x - 1, y - 1)
                - 2.0 * at(x, y - 1)
                - at(x + 1, y - 1);
            out[(y as u32 * w + x as u32) as usize] = (gx * gx + gy * gy).sqrt();
        }
    }
    out
}

/// Edges as dark lines on a background colour.
pub fn edge_detect(img: &RgbaImage, p: &P) -> RgbaImage {
    let e = sobel(img);
    let bg = match p.i("background") {
        0 => [255u8; 3],
        1 => [0u8; 3],
        _ => p.rgb("other_color"),
    };
    let dark_bg = p.i("background") == 1;
    let k = p.f32("sensitivity") / 5.0;
    let w = img.width();
    map_px(img, |x, y, q| {
        let t = (e[(y * w + x) as usize] * k).clamp(0.0, 1.0);
        let line = if dark_bg { [255.0; 3] } else { [0.0; 3] };
        let o: [f32; 3] = std::array::from_fn(|c| bg[c] as f32 + (line[c] - bg[c] as f32) * t);
        Rgba([clamp8(o[0]), clamp8(o[1]), clamp8(o[2]), q[3]])
    })
}

/// Edges in their own colours on white, soft or solid.
pub fn find_edges(img: &RgbaImage, p: &P) -> RgbaImage {
    let e = sobel(img);
    let level = p.f32("level") / 100.0;
    let solid = p.i("edge_type") == 1;
    let w = img.width();
    map_px(img, |x, y, q| {
        let mut t = (e[(y * w + x) as usize] * (0.5 + level * 2.0)).clamp(0.0, 1.0);
        if solid {
            t = if t > 0.3 { 1.0 } else { 0.0 };
        }
        let f = |v: u8| clamp8(255.0 - (255.0 - v as f32) * t);
        Rgba([f(q[0]), f(q[1]), f(q[2]), q[3]])
    })
}

/// Pixels on the level's boundary (below or above it) keep their colour;
/// everything else turns white.
pub fn trace_contour(img: &RgbaImage, p: &P) -> RgbaImage {
    let level = p.f32("level");
    let upper = p.i("edge_type") == 1;
    let (w, h) = img.dimensions();
    let side = |x: u32, y: u32| -> bool {
        let v = luma(img.get_pixel(x, y));
        if upper {
            v >= level
        } else {
            v < level
        }
    };
    map_px(img, |x, y, q| {
        let me = side(x, y);
        let border = me
            && [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|(dx, dy)| {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    nx >= 0
                        && ny >= 0
                        && (nx as u32) < w
                        && (ny as u32) < h
                        && !side(nx as u32, ny as u32)
                });
        if border {
            *q
        } else {
            Rgba([255, 255, 255, q[3]])
        }
    })
}
