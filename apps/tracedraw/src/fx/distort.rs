//! Effects > Distort: pixels moved without adding depth.

use super::util::*;
use super::*;
use image::{Rgba, RgbaImage};
use std::f32::consts::{PI, TAU};

pub const EFFECTS: &[EffectSpec] = &[
    EffectSpec {
        id: "blocks",
        params: &[
            range("width", 1.0, 100.0, 10.0, 1.0),
            range("height", 1.0, 100.0, 10.0, 1.0),
            range("max_offset", 1.0, 100.0, 20.0, 1.0),
            choice(
                "undefined_area",
                &["original_image", "inverse_image", "black", "white", "other"],
                0,
            ),
            color("other_color", 0x808080),
        ],
        apply: blocks,
        reach: no_reach,
    },
    EffectSpec {
        id: "displace",
        params: &[
            choice(
                "displace_map",
                &[
                    "map_waves",
                    "map_rings",
                    "map_bricks",
                    "map_noise",
                    "map_zigzag",
                ],
                0,
            ),
            range("horizontal", 0.0, 100.0, 10.0, 1.0),
            range("vertical", 0.0, 100.0, 10.0, 1.0),
            choice("edges", &["wrap_around", "repeat_edges"], 1),
        ],
        apply: displace,
        reach: no_reach,
    },
    EffectSpec {
        id: "mesh_warp",
        params: &[
            range("gridlines", 2.0, 10.0, 4.0, 1.0),
            choice(
                "warp_style",
                &["warp_random", "warp_bulge", "warp_pinch", "warp_wave"],
                0,
            ),
            range("strength", 0.0, 100.0, 30.0, 1.0),
        ],
        apply: mesh_warp,
        reach: no_reach,
    },
    EffectSpec {
        id: "offset",
        params: &[
            range("horizontal", -100.0, 100.0, 50.0, 1.0),
            range("vertical", -100.0, 100.0, 50.0, 1.0),
            choice(
                "undefined_area",
                &["wrap_around", "repeat_edges", "color"],
                0,
            ),
            color("other_color", 0xFFFFFF),
        ],
        apply: offset,
        reach: no_reach,
    },
    EffectSpec {
        id: "pixelate",
        params: &[
            choice("pixelate_mode", &["square", "rectangular", "radial"], 0),
            range("width", 1.0, 100.0, 10.0, 1.0),
            range("height", 1.0, 100.0, 10.0, 1.0),
            range("opacity", 0.0, 100.0, 100.0, 1.0),
        ],
        apply: pixelate,
        reach: no_reach,
    },
    EffectSpec {
        id: "ripple",
        params: &[
            range("period", 1.0, 100.0, 20.0, 1.0),
            range("amplitude", 1.0, 100.0, 10.0, 1.0),
            angle("direction", 0.0),
            check("perpendicular_wave", false),
            check("distort_ripple", false),
        ],
        apply: ripple,
        reach: no_reach,
    },
    EffectSpec {
        id: "swirl",
        params: &[
            choice("swirl_direction", &["clockwise", "counterclockwise"], 0),
            range("rotations", 0.0, 10.0, 0.0, 1.0),
            range("additional_degrees", 0.0, 359.0, 90.0, 1.0),
        ],
        apply: swirl,
        reach: no_reach,
    },
    EffectSpec {
        id: "tile",
        params: &[
            range("horizontal", 1.0, 100.0, 2.0, 1.0),
            range("vertical", 1.0, 100.0, 2.0, 1.0),
            range("overlap", 0.0, 100.0, 0.0, 1.0),
        ],
        apply: tile,
        reach: no_reach,
    },
    EffectSpec {
        id: "wet_paint",
        params: &[
            range("percentage", 0.0, 100.0, 30.0, 1.0),
            range("wetness", -100.0, 100.0, 50.0, 1.0),
        ],
        apply: wet_paint,
        reach: no_reach,
    },
    EffectSpec {
        id: "whirlpool",
        params: &[
            range("spacing", 1.0, 100.0, 20.0, 1.0),
            range("smear_length", 1.0, 100.0, 10.0, 1.0),
            range("twist", 1.0, 100.0, 50.0, 1.0),
            range("streak_detail", 1.0, 100.0, 50.0, 1.0),
        ],
        apply: whirlpool,
        reach: no_reach,
    },
    EffectSpec {
        id: "wind",
        params: &[
            range("strength", 1.0, 100.0, 30.0, 1.0),
            range("opacity", 0.0, 100.0, 100.0, 1.0),
            angle("direction", 0.0),
        ],
        apply: wind,
        reach: wind_reach,
    },
];

fn wind_reach(p: &P, _: u32, _: u32) -> u32 {
    p.u("strength") / 2
}

pub fn blocks(img: &RgbaImage, p: &P) -> RgbaImage {
    let (bw, bh) = (p.u("width").max(1), p.u("height").max(1));
    let max = p.f32("max_offset") / 100.0;
    let area = p.i("undefined_area");
    let other = p.rgb("other_color");
    let (w, h) = img.dimensions();
    let mut out = map_px(img, |_, _, q| match area {
        0 => *q,
        1 => Rgba([255 - q[0], 255 - q[1], 255 - q[2], q[3]]),
        2 => Rgba([0, 0, 0, q[3]]),
        3 => Rgba([255, 255, 255, q[3]]),
        _ => Rgba([other[0], other[1], other[2], q[3]]),
    });
    let mut by = 0;
    while by < h {
        let mut bx = 0;
        while bx < w {
            let dx = ((hash01(bx, by, 401) - 0.5) * 2.0 * max * bw as f32) as i64;
            let dy = ((hash01(bx, by, 402) - 0.5) * 2.0 * max * bh as f32) as i64;
            for y in by..(by + bh).min(h) {
                for x in bx..(bx + bw).min(w) {
                    let (tx, ty) = (x as i64 + dx, y as i64 + dy);
                    if tx >= 0 && ty >= 0 && (tx as u32) < w && (ty as u32) < h {
                        out.put_pixel(tx as u32, ty as u32, *img.get_pixel(x, y));
                    }
                }
            }
            bx += bw;
        }
        by += bh;
    }
    out
}

/// A displacement map value -1..1 at (x, y).
fn map_value(kind: usize, x: f32, y: f32, axis: u32) -> f32 {
    match kind {
        0 => {
            if axis == 0 {
                (y * TAU / 40.0).sin()
            } else {
                (x * TAU / 40.0).sin()
            }
        }
        1 => {
            let r = (x * x + y * y).sqrt();
            (r * TAU / 30.0).sin()
        }
        2 => {
            let row = (y / 20.0).floor();
            let bx = (x + if row as i64 % 2 == 0 { 0.0 } else { 20.0 }) % 40.0 / 40.0;
            let by = (y % 20.0) / 20.0;
            if axis == 0 {
                bx * 2.0 - 1.0
            } else {
                by * 2.0 - 1.0
            }
        }
        3 => fractal(x, y, 24.0, 410 + axis) * 2.0 - 1.0,
        _ => {
            let t = ((x + y) / 20.0).rem_euclid(2.0);
            if t < 1.0 {
                t * 2.0 - 1.0
            } else {
                3.0 - t * 2.0
            }
        }
    }
}

pub fn displace(img: &RgbaImage, p: &P) -> RgbaImage {
    let kind = p.i("displace_map");
    let (sx, sy) = (
        p.f32("horizontal") / 100.0 * 30.0,
        p.f32("vertical") / 100.0 * 30.0,
    );
    let wrap = p.i("edges") == 0;
    let (w, h) = (img.width() as f32, img.height() as f32);
    remap(img, false, |x, y| {
        let mut nx = x + map_value(kind, x, y, 0) * sx;
        let mut ny = y + map_value(kind, x, y, 1) * sy;
        if wrap {
            nx = nx.rem_euclid(w);
            ny = ny.rem_euclid(h);
        }
        (nx, ny)
    })
}

pub fn mesh_warp(img: &RgbaImage, p: &P) -> RgbaImage {
    let n = p.u("gridlines").max(2) as f32;
    let style = p.i("warp_style");
    let k = p.f32("strength") / 100.0;
    let (w, h) = (img.width() as f32, img.height() as f32);
    remap(img, false, |x, y| {
        let (u, v) = (x / w, y / h);
        let (du, dv) = match style {
            // Each grid node nudged; in between, bilinear.
            0 => {
                let (gx, gy) = (u * (n - 1.0), v * (n - 1.0));
                let (i, j) = (gx.floor(), gy.floor());
                let (tx, ty) = (gx - i, gy - j);
                let node = |a: f32, b: f32, s: u32| -> f32 {
                    let edge = a <= 0.0 || b <= 0.0 || a >= n - 1.0 || b >= n - 1.0;
                    if edge {
                        0.0
                    } else {
                        (hash01(a as u32, b as u32, s) - 0.5) / (n - 1.0)
                    }
                };
                let lerp2 = |s: u32| {
                    (node(i, j, s) * (1.0 - tx) + node(i + 1.0, j, s) * tx) * (1.0 - ty)
                        + (node(i, j + 1.0, s) * (1.0 - tx) + node(i + 1.0, j + 1.0, s) * tx) * ty
                };
                (lerp2(421), lerp2(422))
            }
            1 | 2 => {
                let (dx, dy) = (u - 0.5, v - 0.5);
                let r = (dx * dx + dy * dy).sqrt() * 2.0;
                let f = (1.0 - r).max(0.0) * if style == 1 { -0.25 } else { 0.25 };
                (dx * f, dy * f)
            }
            _ => (
                (v * TAU * (n - 1.0) / 2.0).sin() * 0.03,
                (u * TAU * (n - 1.0) / 2.0).sin() * 0.03,
            ),
        };
        ((u + du * k * 2.0) * w, (v + dv * k * 2.0) * h)
    })
}

pub fn offset(img: &RgbaImage, p: &P) -> RgbaImage {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let (ox, oy) = (
        p.f32("horizontal") / 100.0 * w,
        p.f32("vertical") / 100.0 * h,
    );
    let area = p.i("undefined_area");
    let c = p.rgb("other_color");
    map_px(img, |x, y, q| {
        let (sx, sy) = (x as f32 - ox, y as f32 + oy);
        match area {
            0 => *img.get_pixel(
                sx.rem_euclid(w) as u32 % img.width(),
                sy.rem_euclid(h) as u32 % img.height(),
            ),
            1 => sample(img, sx, sy),
            _ => {
                if sx < 0.0 || sy < 0.0 || sx >= w || sy >= h {
                    Rgba([c[0], c[1], c[2], q[3]])
                } else {
                    sample(img, sx, sy)
                }
            }
        }
    })
}

pub fn pixelate(img: &RgbaImage, p: &P) -> RgbaImage {
    let mode = p.i("pixelate_mode");
    let (cw, ch) = if mode == 0 {
        (p.f32("width").max(1.0), p.f32("width").max(1.0))
    } else {
        (p.f32("width").max(1.0), p.f32("height").max(1.0))
    };
    let opacity = p.f32("opacity") / 100.0;
    let (w, h) = (img.width() as f32, img.height() as f32);
    let cells = map_px(img, |x, y, q| {
        let (sx, sy) = if mode == 2 {
            // Rings and sectors about the centre.
            let (dx, dy) = (x as f32 - w / 2.0, y as f32 - h / 2.0);
            let r = ((dx * dx + dy * dy).sqrt() / ch).floor() * ch + ch / 2.0;
            let steps = (TAU * r.max(1.0) / cw).max(1.0);
            let a = ((dy.atan2(dx) + PI) / TAU * steps).floor() / steps * TAU - PI + PI / steps;
            (w / 2.0 + r * a.cos(), h / 2.0 + r * a.sin())
        } else {
            (
                (x as f32 / cw).floor() * cw + cw / 2.0,
                (y as f32 / ch).floor() * ch + ch / 2.0,
            )
        };
        let s = sample(img, sx, sy);
        Rgba([s[0], s[1], s[2], q[3]])
    });
    mix(img, &cells, opacity)
}

pub fn ripple(img: &RgbaImage, p: &P) -> RgbaImage {
    let period = p.f32("period").max(1.0) * 2.0;
    let amp = p.f32("amplitude") / 4.0;
    let a = p.f32("direction").to_radians();
    let perp = p.b("perpendicular_wave");
    let distort = p.b("distort_ripple");
    let (s, c) = a.sin_cos();
    remap(img, false, |x, y| {
        let u = x * c - y * s;
        let v = x * s + y * c;
        let mut ph = u / period * TAU;
        if distort {
            ph += (v / period * 2.0).sin();
        }
        let d1 = ph.sin() * amp;
        let d2 = if perp {
            (v / period * TAU).sin() * amp * 0.5
        } else {
            0.0
        };
        // Displace across the wave, and along it for the second wave.
        (x + d1 * s + d2 * c, y + d1 * c - d2 * s)
    })
}

pub fn swirl(img: &RgbaImage, p: &P) -> RgbaImage {
    let turns = p.f32("rotations") + p.f32("additional_degrees") / 360.0;
    let dir = if p.i("swirl_direction") == 0 {
        1.0
    } else {
        -1.0
    };
    let (w, h) = (img.width() as f32, img.height() as f32);
    let r_max = w.min(h) / 2.0;
    remap(img, false, |x, y| {
        let (dx, dy) = (x - w / 2.0, y - h / 2.0);
        let r = (dx * dx + dy * dy).sqrt();
        if r >= r_max {
            return (x, y);
        }
        let t = 1.0 - r / r_max;
        let a = dy.atan2(dx) + dir * turns * TAU * t * t;
        (w / 2.0 + r * a.cos(), h / 2.0 + r * a.sin())
    })
}

pub fn tile(img: &RgbaImage, p: &P) -> RgbaImage {
    let (nx, ny) = (p.f32("horizontal").max(1.0), p.f32("vertical").max(1.0));
    let overlap = p.f32("overlap") / 100.0;
    let (w, h) = (img.width() as f32, img.height() as f32);
    remap(img, false, |x, y| {
        let (tw, th) = (w / nx, h / ny);
        let (u, v) = ((x % tw) / tw, (y % th) / th);
        let s = 1.0 + overlap;
        ((u * s).min(1.0) * w * 0.999, (v * s).min(1.0) * h * 0.999)
    })
}

pub fn wet_paint(img: &RgbaImage, p: &P) -> RgbaImage {
    let t = p.f32("percentage") / 100.0;
    let wet = p.f32("wetness") / 100.0;
    let (w, h) = img.dimensions();
    map_px(img, |x, y, q| {
        let l = luma(q) / 255.0;
        // Dark colours run when wetness is positive, light ones when negative.
        let runs = if wet >= 0.0 { 1.0 - l } else { l };
        let len = (runs * wet.abs() * 40.0 * t * (0.5 + hash01(x, 0, 431))) as i64;
        let mut acc = [0f32; 3];
        let mut n = 0.0;
        for k in 0..=len {
            let sy = (y as i64 - k).clamp(0, h as i64 - 1) as u32;
            let s = img.get_pixel(x.min(w - 1), sy);
            for c in 0..3 {
                acc[c] += s[c] as f32;
            }
            n += 1.0;
        }
        Rgba([
            clamp8(acc[0] / n),
            clamp8(acc[1] / n),
            clamp8(acc[2] / n),
            q[3],
        ])
    })
}

pub fn whirlpool(img: &RgbaImage, p: &P) -> RgbaImage {
    let spacing = p.f32("spacing").max(1.0) * 2.0;
    let smear = p.f32("smear_length").max(1.0);
    let twist = p.f32("twist") / 100.0;
    let detail = p.f32("streak_detail") / 100.0;
    let flow = |x: f32, y: f32| -> f32 { fractal(x, y, spacing, 441) * TAU * 2.0 * twist };
    let n = (smear / 2.0).ceil() as i32;
    map_px(img, |x, y, q| {
        let (mut px, mut py) = (x as f32, y as f32);
        let mut acc = [0f32; 3];
        for _ in 0..=n {
            let s = bilinear(img, px, py, false);
            for c in 0..3 {
                acc[c] += s[c] as f32;
            }
            let a = flow(px, py);
            px += a.cos() * 2.0;
            py += a.sin() * 2.0;
        }
        let m = (n + 1) as f32;
        let streak = 1.0 + (hash01(x, y, 442) - 0.5) * 0.2 * detail;
        Rgba([
            clamp8(acc[0] / m * streak),
            clamp8(acc[1] / m * streak),
            clamp8(acc[2] / m * streak),
            q[3],
        ])
    })
}

pub fn wind(img: &RgbaImage, p: &P) -> RgbaImage {
    let len = p.f32("strength") / 2.0;
    let opacity = p.f32("opacity") / 100.0;
    let a = p.f32("direction").to_radians();
    let (dx, dy) = (-a.cos(), a.sin());
    let blown = map_px(img, |x, y, q| {
        // Streaks of random length behind each pixel, brightest wins.
        let l = len * (0.3 + 0.7 * hash01(0, y, 451));
        let n = l.ceil().max(1.0) as i32;
        let mut best = *q;
        for k in 1..=n {
            let s = bilinear(
                img,
                x as f32 + dx * k as f32,
                y as f32 + dy * k as f32,
                true,
            );
            let fade = 1.0 - k as f32 / (n + 1) as f32;
            let sl = luma(&s) * fade;
            if s[3] > 0 && sl > luma(&best) {
                best = Rgba([
                    clamp8(s[0] as f32 * fade + best[0] as f32 * (1.0 - fade)),
                    clamp8(s[1] as f32 * fade + best[1] as f32 * (1.0 - fade)),
                    clamp8(s[2] as f32 * fade + best[2] as f32 * (1.0 - fade)),
                    s[3].max(q[3]),
                ]);
            }
        }
        best
    });
    mix(img, &blown, opacity)
}
