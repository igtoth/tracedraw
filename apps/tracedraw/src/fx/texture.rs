//! Effects > 3D Effects, Custom (bump map) and Texture.

use super::util::*;
use super::*;
use image::{Rgba, RgbaImage};
use std::f32::consts::{FRAC_PI_2, PI};

const CORNERS: [&str; 4] = ["top_left", "top_right", "bottom_left", "bottom_right"];

pub const THREE_D: &[EffectSpec] = &[
    EffectSpec {
        id: "rotate_3d",
        params: &[
            range("vertical", -75.0, 75.0, 15.0, 1.0),
            range("horizontal", -75.0, 75.0, 15.0, 1.0),
            check("best_fit", true),
        ],
        apply: rotate_3d,
        reach: no_reach,
    },
    EffectSpec {
        id: "cylinder",
        params: &[
            choice("cylinder_mode", &["horizontal", "vertical"], 0),
            range("percentage", -100.0, 100.0, 50.0, 1.0),
        ],
        apply: cylinder,
        reach: no_reach,
    },
    EffectSpec {
        id: "emboss",
        params: &[
            range("depth", 1.0, 20.0, 2.0, 1.0),
            range("level", 1.0, 500.0, 100.0, 1.0),
            angle("direction", 45.0),
            choice(
                "emboss_color",
                &["original_color", "gray", "black", "other"],
                1,
            ),
            color("other_color", 0xC0C0C0),
        ],
        apply: emboss,
        reach: no_reach,
    },
    EffectSpec {
        id: "page_curl",
        params: &[
            choice("corner", &CORNERS, 3),
            choice("curl_direction", &["vertical", "horizontal"], 0),
            choice("curl_type", &["opaque", "transparent"], 0),
            color("curl_color", 0xF0F0F0),
            color("background_color", 0xFFFFFF),
            range("width", 1.0, 100.0, 50.0, 1.0),
            range("height", 1.0, 100.0, 50.0, 1.0),
        ],
        apply: page_curl,
        reach: no_reach,
    },
    EffectSpec {
        id: "pinch_punch",
        params: &[
            range("amount", -100.0, 100.0, 50.0, 1.0),
            range("center_x", 0.0, 100.0, 50.0, 1.0),
            range("center_y", 0.0, 100.0, 50.0, 1.0),
        ],
        apply: pinch_punch,
        reach: no_reach,
    },
    EffectSpec {
        id: "sphere",
        params: &[
            range("amount", -100.0, 100.0, 50.0, 1.0),
            range("center_x", 0.0, 100.0, 50.0, 1.0),
            range("center_y", 0.0, 100.0, 50.0, 1.0),
        ],
        apply: sphere,
        reach: no_reach,
    },
];

pub const CUSTOM: &[EffectSpec] = &[EffectSpec {
    id: "bump_map",
    params: &[
        choice(
            "bump_source",
            &[
                "map_image",
                "map_waves",
                "map_rings",
                "map_bricks",
                "map_noise",
                "map_zigzag",
            ],
            3,
        ),
        range("depth", 1.0, 100.0, 20.0, 1.0),
        range("smoothness", 0.0, 100.0, 30.0, 1.0),
        angle("light_direction", 135.0),
        range("brightness", 0.0, 100.0, 50.0, 1.0),
    ],
    apply: bump_map,
    reach: no_reach,
}];

pub const TEXTURE: &[EffectSpec] = &[
    EffectSpec {
        id: "cobblestone",
        params: &[
            range("size", 1.0, 100.0, 20.0, 1.0),
            range("coarseness", 1.0, 100.0, 50.0, 1.0),
            range("spacing", 1.0, 100.0, 10.0, 1.0),
            angle("light_direction", 135.0),
        ],
        apply: cobblestone,
        reach: no_reach,
    },
    EffectSpec {
        id: "wrinkles",
        params: &[
            range("age", 1.0, 100.0, 50.0, 1.0),
            color("skin_color", 0x8C7C6C),
        ],
        apply: wrinkles,
        reach: no_reach,
    },
    EffectSpec {
        id: "etching",
        params: &[
            range("detail", 1.0, 100.0, 50.0, 1.0),
            range("depth", 1.0, 100.0, 50.0, 1.0),
            range("brightness", 0.0, 100.0, 50.0, 1.0),
            angle("light_direction", 135.0),
            color("metal_color", 0xB4A48C),
        ],
        apply: etching,
        reach: no_reach,
    },
    EffectSpec {
        id: "plastic",
        params: &[
            range("highlight", 1.0, 100.0, 50.0, 1.0),
            range("depth", 1.0, 100.0, 20.0, 1.0),
            range("smoothness", 1.0, 100.0, 50.0, 1.0),
            angle("light_direction", 135.0),
            color("light_color", 0xFFFFFF),
        ],
        apply: plastic,
        reach: no_reach,
    },
    EffectSpec {
        id: "relief_sculpture",
        params: &[
            range("detail", 1.0, 100.0, 50.0, 1.0),
            range("depth", 1.0, 100.0, 50.0, 1.0),
            range("smoothness", 1.0, 100.0, 50.0, 1.0),
            angle("light_direction", 135.0),
            color("surface_color", 0xC8C0B0),
        ],
        apply: relief_sculpture,
        reach: no_reach,
    },
    EffectSpec {
        id: "stone",
        params: &[
            range("detail", 1.0, 100.0, 50.0, 1.0),
            range("density", 1.0, 100.0, 50.0, 1.0),
            angle("light_direction", 135.0),
        ],
        apply: stone,
        reach: no_reach,
    },
];

pub fn rotate_3d(img: &RgbaImage, p: &P) -> RgbaImage {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let (av, ah) = (
        p.f32("vertical").to_radians(),
        p.f32("horizontal").to_radians(),
    );
    let f = 2.0 * w.max(h);
    // Rotation: about x by the vertical angle, then about y.
    let (sv, cv) = av.sin_cos();
    let (sh, ch) = ah.sin_cos();
    let rot = |x: f32, y: f32, z: f32| -> (f32, f32, f32) {
        let (y1, z1) = (y * cv - z * sv, y * sv + z * cv);
        (x * ch + z1 * sh, y1, -x * sh + z1 * ch)
    };
    let inv = |x: f32, y: f32, z: f32| -> (f32, f32, f32) {
        let (x1, z1) = (x * ch - z * sh, x * sh + z * ch);
        (x1, y * cv + z1 * sv, -y * sv + z1 * cv)
    };
    let project = |x: f32, y: f32| {
        let (rx, ry, rz) = rot(x, y, 0.0);
        let k = f / (rz + f).max(1e-3);
        (rx * k, ry * k)
    };
    let mut scale = 1.0;
    if p.b("best_fit") {
        let mut m = (0f32, 0f32);
        for (x, y) in [
            (-w / 2.0, -h / 2.0),
            (w / 2.0, -h / 2.0),
            (-w / 2.0, h / 2.0),
            (w / 2.0, h / 2.0),
        ] {
            let (px, py) = project(x, y);
            m = (m.0.max(px.abs()), m.1.max(py.abs()));
        }
        scale = (w / 2.0 / m.0.max(1e-3)).min(h / 2.0 / m.1.max(1e-3));
    }
    let n = rot(0.0, 0.0, 1.0);
    remap(img, true, |x, y| {
        let (sx, sy) = ((x - w / 2.0) / scale, (y - h / 2.0) / scale);
        let den = n.0 * sx + n.1 * sy + n.2 * f;
        if den.abs() < 1e-6 {
            return (-1e6, -1e6);
        }
        let t = n.2 * f / den;
        let (px, py, pz) = (t * sx, t * sy, -f + t * f);
        let (u, v, _) = inv(px, py, pz);
        (u + w / 2.0, v + h / 2.0)
    })
}

pub fn cylinder(img: &RgbaImage, p: &P) -> RgbaImage {
    let k = p.f32("percentage") / 100.0;
    let vertical = p.i("cylinder_mode") == 1;
    let (w, h) = (img.width() as f32, img.height() as f32);
    let warp = |t: f32| -> f32 {
        let t = t.clamp(-1.0, 1.0);
        if k >= 0.0 {
            t + k * (t.asin() / FRAC_PI_2 - t)
        } else {
            t + (-k) * ((t * FRAC_PI_2).sin() - t)
        }
    };
    remap(img, false, |x, y| {
        if vertical {
            let t = y / h * 2.0 - 1.0;
            (x, (warp(t) + 1.0) / 2.0 * h)
        } else {
            let t = x / w * 2.0 - 1.0;
            ((warp(t) + 1.0) / 2.0 * w, y)
        }
    })
}

pub fn emboss(img: &RgbaImage, p: &P) -> RgbaImage {
    let (w, h) = img.dimensions();
    let hts = heights(img);
    let depth = p.f32("depth");
    let level = p.f32("level") / 100.0;
    let a = p.f32("direction").to_radians();
    let (dx, dy) = (a.cos() * depth, -a.sin() * depth);
    let at = |x: f32, y: f32| -> f32 {
        let xi = (x.round() as i64).clamp(0, w as i64 - 1) as u32;
        let yi = (y.round() as i64).clamp(0, h as i64 - 1) as u32;
        hts[(yi * w + xi) as usize]
    };
    let mode = p.i("emboss_color");
    let other = p.rgb("other_color");
    map_px(img, |x, y, q| {
        let d = (at(x as f32 + dx, y as f32 + dy) - at(x as f32 - dx, y as f32 - dy)) * level;
        let base: [f32; 3] = match mode {
            0 => [q[0] as f32, q[1] as f32, q[2] as f32],
            1 => [128.0; 3],
            2 => [40.0; 3],
            _ => other.map(|v| v as f32),
        };
        let k = d * 255.0;
        Rgba([
            clamp8(base[0] + k),
            clamp8(base[1] + k),
            clamp8(base[2] + k),
            q[3],
        ])
    })
}

pub fn page_curl(img: &RgbaImage, p: &P) -> RgbaImage {
    let corner = p.i("corner");
    let horizontal = p.i("curl_direction") == 1;
    let transparent = p.i("curl_type") == 1;
    let curl = p.rgb("curl_color");
    let bg = p.rgb("background_color");
    let (w, h) = (img.width() as f32, img.height() as f32);
    let (cw, chh) = (p.f32("width") / 100.0 * w, p.f32("height") / 100.0 * h);
    map_px(img, |x, y, q| {
        // Coordinates measured from the curled corner.
        let (mut u, mut v) = (x as f32, y as f32);
        if corner == 1 || corner == 3 {
            u = w - 1.0 - u;
        }
        if corner == 2 || corner == 3 {
            v = h - 1.0 - v;
        }
        // Distance across the fold (the diagonal of the curl box).
        let s = u / cw.max(1.0) + v / chh.max(1.0);
        if s >= 1.0 {
            return *q;
        }
        let fold = 1.0 - s;
        if s < 1.0 - 2.0 * fold.min(0.5) && s < 0.5 {
            // Uncovered: the background.
            return Rgba([bg[0], bg[1], bg[2], 255]);
        }
        // On the curl: shaded by its roundness.
        let t = ((1.0 - s) * 2.0).min(1.0);
        let shade = 0.55 + 0.45 * (t * PI).sin();
        let grain = if horizontal {
            (y as f32 * 0.8).sin()
        } else {
            (x as f32 * 0.8).sin()
        } * 0.03;
        let base = if transparent {
            [q[0] as f32, q[1] as f32, q[2] as f32]
        } else {
            curl.map(|v| v as f32)
        };
        Rgba([
            clamp8(base[0] * (shade + grain)),
            clamp8(base[1] * (shade + grain)),
            clamp8(base[2] * (shade + grain)),
            255,
        ])
    })
}

fn radial(img: &RgbaImage, p: &P, f: impl Fn(f32, f32) -> f32) -> RgbaImage {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let (cx, cy) = (p.f32("center_x") / 100.0 * w, p.f32("center_y") / 100.0 * h);
    let r_max = w.min(h) / 2.0;
    let k = p.f32("amount") / 100.0;
    remap(img, false, |x, y| {
        let (dx, dy) = (x - cx, y - cy);
        let r = (dx * dx + dy * dy).sqrt() / r_max;
        if r >= 1.0 || r <= 1e-6 {
            return (x, y);
        }
        let s = f(r, k) / r;
        (cx + dx * s, cy + dy * s)
    })
}

/// Positive amounts pinch toward the centre, negative ones punch out.
pub fn pinch_punch(img: &RgbaImage, p: &P) -> RgbaImage {
    radial(img, p, |r, k| r.powf(1.0 - k * 0.6))
}

/// Wraps the middle over a sphere: positive amounts bulge (the middle
/// grows), negative ones dent it.
pub fn sphere(img: &RgbaImage, p: &P) -> RgbaImage {
    radial(img, p, |r, k| {
        if k >= 0.0 {
            r + k * (r * r - r)
        } else {
            r + (-k) * (r.sqrt() - r)
        }
    })
}

/// A height map 0..1 for a bump source (`map_image` uses the image).
fn bump_height(kind: usize, img: &RgbaImage, x: u32, y: u32) -> f32 {
    let (fx, fy) = (x as f32, y as f32);
    match kind {
        0 => luma(img.get_pixel(x, y)) / 255.0,
        1 => (fy * 0.3).sin() * 0.5 + 0.5,
        2 => ((fx * fx + fy * fy).sqrt() * 0.3).sin() * 0.5 + 0.5,
        3 => {
            let row = (fy / 12.0).floor() as i64;
            let bx = (fx + if row % 2 == 0 { 0.0 } else { 12.0 }) % 24.0;
            let by = fy % 12.0;
            if bx < 1.5 || by < 1.5 {
                0.0
            } else {
                0.8
            }
        }
        4 => fractal(fx, fy, 10.0, 601),
        _ => ((fx + fy) * 0.25).sin().abs(),
    }
}

pub fn bump_map(img: &RgbaImage, p: &P) -> RgbaImage {
    let (w, h) = img.dimensions();
    let kind = p.i("bump_source");
    let mut hm: Vec<f32> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| bump_height(kind, img, x, y))
        .collect();
    let smooth = p.f32("smoothness") / 100.0;
    if smooth > 0.0 {
        let hi: RgbaImage = image::ImageBuffer::from_fn(w, h, |x, y| {
            let v = clamp8(hm[(y * w + x) as usize] * 255.0);
            Rgba([v, v, v, 255])
        });
        hm = heights(&gaussian(&hi, smooth * 3.0));
    }
    let lit = shade(&hm, w, h, p.f32("light_direction"), p.f32("depth") / 10.0);
    let bright = (p.f32("brightness") - 50.0) * 1.5;
    map_px(img, |x, y, q| {
        let k = lit[(y * w + x) as usize] * 2.0;
        Rgba([
            clamp8(q[0] as f32 * k + bright),
            clamp8(q[1] as f32 * k + bright),
            clamp8(q[2] as f32 * k + bright),
            q[3],
        ])
    })
}

pub fn cobblestone(img: &RgbaImage, p: &P) -> RgbaImage {
    let (w, h) = img.dimensions();
    let size = p.f32("size").max(1.0);
    let coarse = p.f32("coarseness") / 100.0;
    let spacing = p.f32("spacing") / 100.0 * size * 0.5;
    let hm: Vec<f32> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            let (fx, fy) = (x as f32, y as f32);
            let (gx, gy) = ((fx / size).floor(), (fy / size).floor());
            let mut best = (f32::MAX, f32::MAX);
            for j in -1..=1 {
                for i in -1..=1 {
                    let (cx, cy) = (gx + i as f32, gy + j as f32);
                    let px = (cx + hash01(cx as i64 as u32, cy as i64 as u32, 611)) * size;
                    let py = (cy + hash01(cx as i64 as u32, cy as i64 as u32, 612)) * size;
                    let d = ((px - fx).powi(2) + (py - fy).powi(2)).sqrt();
                    if d < best.0 {
                        best = (d, best.0);
                    } else if d < best.1 {
                        best.1 = d;
                    }
                }
            }
            let edge = best.1 - best.0;
            let stone = (edge / spacing.max(0.5)).min(1.0);
            stone * (0.8 + coarse * 0.2 * hash01(x, y, 613))
        })
        .collect();
    let lit = shade(&hm, w, h, p.f32("light_direction"), 3.0);
    map_px(img, |x, y, q| {
        let k = lit[(y * w + x) as usize] * 1.6 * (0.4 + 0.6 * hm[(y * w + x) as usize]);
        Rgba([
            clamp8(q[0] as f32 * k),
            clamp8(q[1] as f32 * k),
            clamp8(q[2] as f32 * k),
            q[3],
        ])
    })
}

pub fn wrinkles(img: &RgbaImage, p: &P) -> RgbaImage {
    let (w, h) = img.dimensions();
    let age = p.f32("age") / 100.0;
    let skin = p.rgb("skin_color").map(|v| v as f32);
    let hm: Vec<f32> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            let (fx, fy) = (x as f32, y as f32);
            let wave = (fy * 0.5 + fractal(fx, fy, 12.0, 621) * 12.0).sin();
            (wave.abs() * (0.3 + 0.7 * age)).min(1.0)
        })
        .collect();
    let lit = shade(&hm, w, h, 135.0, 2.0);
    map_px(img, |x, y, q| {
        let k = lit[(y * w + x) as usize] * 1.6;
        let l = luma(q) / 255.0;
        Rgba([
            clamp8(skin[0] * k * (0.5 + l * 0.6)),
            clamp8(skin[1] * k * (0.5 + l * 0.6)),
            clamp8(skin[2] * k * (0.5 + l * 0.6)),
            q[3],
        ])
    })
}

fn metal_relief(
    img: &RgbaImage,
    detail: f32,
    depth: f32,
    smooth: f32,
    angle: f32,
    metal: [u8; 3],
    bright: f32,
) -> RgbaImage {
    let (w, h) = img.dimensions();
    let base = gaussian(img, smooth * 2.0 + (1.0 - detail) * 2.0);
    let hm = heights(&base);
    let lit = shade(&hm, w, h, angle, depth * 6.0);
    map_px(img, |x, y, q| {
        let k = lit[(y * w + x) as usize] * 2.0;
        Rgba([
            clamp8(metal[0] as f32 * k + bright),
            clamp8(metal[1] as f32 * k + bright),
            clamp8(metal[2] as f32 * k + bright),
            q[3],
        ])
    })
}

pub fn etching(img: &RgbaImage, p: &P) -> RgbaImage {
    metal_relief(
        img,
        p.f32("detail") / 100.0,
        p.f32("depth") / 100.0,
        0.0,
        p.f32("light_direction"),
        p.rgb("metal_color"),
        (p.f32("brightness") - 50.0) * 1.5,
    )
}

pub fn relief_sculpture(img: &RgbaImage, p: &P) -> RgbaImage {
    metal_relief(
        img,
        p.f32("detail") / 100.0,
        p.f32("depth") / 100.0,
        p.f32("smoothness") / 100.0,
        p.f32("light_direction"),
        p.rgb("surface_color"),
        0.0,
    )
}

pub fn plastic(img: &RgbaImage, p: &P) -> RgbaImage {
    let (w, h) = img.dimensions();
    let smooth = p.f32("smoothness") / 100.0;
    let base = gaussian(img, 0.5 + smooth * 3.0);
    let hm = heights(&base);
    let lit = shade(
        &hm,
        w,
        h,
        p.f32("light_direction"),
        p.f32("depth") / 100.0 * 10.0,
    );
    let hl = p.f32("highlight") / 100.0;
    let lc = p.rgb("light_color").map(|v| v as f32);
    map_px(&base, |x, y, q| {
        let l = lit[(y * w + x) as usize];
        // A specular sheen on the lit slopes.
        let spec = ((l - 0.5).max(0.0) * 2.0).powf(3.0) * hl;
        let k = 0.6 + l * 0.8;
        Rgba([
            clamp8(q[0] as f32 * k + lc[0] * spec),
            clamp8(q[1] as f32 * k + lc[1] * spec),
            clamp8(q[2] as f32 * k + lc[2] * spec),
            img.get_pixel(x, y)[3],
        ])
    })
}

pub fn stone(img: &RgbaImage, p: &P) -> RgbaImage {
    let (w, h) = img.dimensions();
    let detail = p.f32("detail") / 100.0;
    let density = p.f32("density") / 100.0;
    let hm: Vec<f32> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            fractal(x as f32, y as f32, 4.0 + (1.0 - density) * 20.0, 631) * (0.5 + detail * 0.5)
        })
        .collect();
    let lit = shade(&hm, w, h, p.f32("light_direction"), 4.0);
    map_px(img, |x, y, q| {
        let k = lit[(y * w + x) as usize] * 1.7;
        Rgba([
            clamp8(q[0] as f32 * k),
            clamp8(q[1] as f32 * k),
            clamp8(q[2] as f32 * k),
            q[3],
        ])
    })
}
