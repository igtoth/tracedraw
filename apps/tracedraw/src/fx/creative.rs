//! Effects > Creative: crystals, fabric, frames, glass, mosaics,
//! vignettes and vortices.

use super::util::*;
use super::*;
use image::{Rgba, RgbaImage};

pub const EFFECTS: &[EffectSpec] = &[
    EffectSpec {
        id: "crystallize",
        params: &[range("size", 3.0, 100.0, 10.0, 1.0)],
        apply: crystallize,
        reach: no_reach,
    },
    EffectSpec {
        id: "fabric",
        params: &[
            choice(
                "fabric_style",
                &[
                    "needlepoint",
                    "rug_hooking",
                    "quilt",
                    "strings",
                    "ribbons",
                    "tissue_collage",
                ],
                0,
            ),
            range("size", 1.0, 100.0, 20.0, 1.0),
            range("completion", 1.0, 100.0, 100.0, 1.0),
            range("brightness", 0.0, 100.0, 50.0, 1.0),
            angle("rotation", 0.0),
        ],
        apply: fabric,
        reach: no_reach,
    },
    EffectSpec {
        id: "frame",
        params: &[
            choice(
                "frame_style",
                &[
                    "frame_plain",
                    "frame_rounded",
                    "frame_torn",
                    "frame_soft",
                    "frame_brush",
                ],
                0,
            ),
            color("frame_color", 0xFFFFFF),
            range("opacity", 0.0, 100.0, 100.0, 1.0),
            range("feather", 0.0, 100.0, 0.0, 1.0),
            range("horizontal", 1.0, 200.0, 100.0, 1.0),
            range("vertical", 1.0, 200.0, 100.0, 1.0),
            angle("rotation", 0.0),
            check("flip_horizontal", false),
            check("flip_vertical", false),
        ],
        apply: frame,
        reach: no_reach,
    },
    EffectSpec {
        id: "glass_tiles",
        params: &[
            range("width", 1.0, 100.0, 10.0, 1.0),
            range("height", 1.0, 100.0, 10.0, 1.0),
        ],
        apply: glass_tiles,
        reach: no_reach,
    },
    EffectSpec {
        id: "mosaic",
        params: &[
            range("size", 2.0, 100.0, 10.0, 1.0),
            color("background_color", 0xFFFFFF),
            check("vignette", false),
        ],
        apply: mosaic,
        reach: no_reach,
    },
    EffectSpec {
        id: "scatter",
        params: &[
            range("horizontal", 0.0, 100.0, 5.0, 1.0),
            range("vertical", 0.0, 100.0, 5.0, 1.0),
        ],
        apply: scatter,
        reach: no_reach,
    },
    EffectSpec {
        id: "tinted_glass",
        params: &[
            color("tint", 0x3C3C3C),
            range("percentage", 0.0, 100.0, 50.0, 1.0),
            range("blur", 0.0, 100.0, 2.0, 1.0),
        ],
        apply: tinted_glass,
        reach: no_reach,
    },
    EffectSpec {
        id: "stained_glass",
        params: &[
            range("size", 3.0, 100.0, 20.0, 1.0),
            range("light_intensity", 0.0, 10.0, 3.0, 1.0),
            range("solder_width", 1.0, 10.0, 3.0, 1.0),
            color("solder_color", 0x000000),
            check("lighting_3d", false),
        ],
        apply: stained_glass,
        reach: no_reach,
    },
    EffectSpec {
        id: "vignette",
        params: &[
            choice(
                "vignette_shape",
                &["ellipse", "circle", "rectangle", "square"],
                0,
            ),
            choice("vignette_color", &["black", "white", "other"], 0),
            color("other_color", 0x808080),
            range("offset", 0.0, 200.0, 100.0, 1.0),
            range("fade", 0.0, 100.0, 50.0, 1.0),
        ],
        apply: vignette,
        reach: no_reach,
    },
    EffectSpec {
        id: "vortex",
        params: &[
            choice("vortex_style", &["average", "large", "fine", "layered"], 0),
            range("size", 2.0, 100.0, 20.0, 1.0),
            angle("inner_direction", 30.0),
            angle("outer_direction", 330.0),
        ],
        apply: vortex,
        reach: no_reach,
    },
];

/// Nearest of a jittered grid of seeds: (seed x, seed y, second distance
/// minus first) for Voronoi cells of about `size` pixels.
fn voronoi(x: f32, y: f32, size: f32, seed: u32) -> (f32, f32, f32) {
    let s = size.max(1.0);
    let (gx, gy) = ((x / s).floor() as i64, (y / s).floor() as i64);
    let mut best = (f32::MAX, 0.0, 0.0);
    let mut second = f32::MAX;
    for j in gy - 1..=gy + 1 {
        for i in gx - 1..=gx + 1 {
            let px = (i as f32 + hash01(i as u32, j as u32, seed)) * s;
            let py = (j as f32 + hash01(i as u32, j as u32, seed + 1)) * s;
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if d < best.0 {
                second = best.0;
                best = (d, px, py);
            } else if d < second {
                second = d;
            }
        }
    }
    (best.1, best.2, second - best.0)
}

pub fn crystallize(img: &RgbaImage, p: &P) -> RgbaImage {
    let size = p.f32("size");
    map_px(img, |x, y, q| {
        let (cx, cy, _) = voronoi(x as f32, y as f32, size, 301);
        let c = sample(img, cx, cy);
        Rgba([c[0], c[1], c[2], q[3]])
    })
}

pub fn fabric(img: &RgbaImage, p: &P) -> RgbaImage {
    let style = p.i("fabric_style");
    let size = p.f32("size").max(1.0) / 2.0;
    let completion = p.f32("completion") / 100.0;
    let bright = (p.f32("brightness") - 50.0) * 1.5;
    let rot = p.f32("rotation").to_radians();
    let (s, c) = rot.sin_cos();
    map_px(img, |x, y, q| {
        let (fx, fy) = (x as f32, y as f32);
        let u = fx * c + fy * s;
        let v = -fx * s + fy * c;
        let (cu, cv) = ((u / size).floor(), (v / size).floor());
        let (tu, tv) = (u / size - cu, v / size - cv);
        let centre = ((cu + 0.5) * size, (cv + 0.5) * size);
        let src = sample(
            img,
            centre.0 * c - centre.1 * s,
            centre.0 * s + centre.1 * c,
        );
        let shade = match style {
            // Stitches: diagonal threads in each cell.
            0 => 0.75 + 0.25 * (((tu + tv) * std::f32::consts::PI * 2.0).sin() * 0.5 + 0.5),
            // Loops of yarn.
            1 => {
                let d = ((tu - 0.5).powi(2) + (tv - 0.5).powi(2)).sqrt();
                0.7 + 0.3 * (1.0 - (d * 2.0).min(1.0))
            }
            // Quilted squares with seams.
            2 => {
                if tu < 0.06 || tv < 0.06 {
                    0.6
                } else {
                    0.9 + 0.1 * hash01(cu as i64 as u32, cv as i64 as u32, 311)
                }
            }
            // Strings: thin lines one way.
            3 => 0.65 + 0.35 * ((tv * std::f32::consts::PI * 3.0).sin().abs()),
            // Ribbons: bands with a sheen.
            4 => 0.7 + 0.3 * (tu * std::f32::consts::PI).sin(),
            // Torn tissue pieces.
            _ => 0.8 + 0.2 * fractal(fx, fy, size * 1.5, 312),
        };
        let done = hash01(cu as i64 as u32, cv as i64 as u32, 313) <= completion;
        let base = if done { src } else { *q };
        let f = |v: u8| clamp8(v as f32 * if done { shade } else { 1.0 } + bright);
        Rgba([f(base[0]), f(base[1]), f(base[2]), q[3]])
    })
}

pub fn frame(img: &RgbaImage, p: &P) -> RgbaImage {
    let style = p.i("frame_style");
    let col = p.rgb("frame_color");
    let opacity = p.f32("opacity") / 100.0;
    let feather = p.f32("feather") / 100.0;
    let (sx, sy) = (p.f32("horizontal") / 100.0, p.f32("vertical") / 100.0);
    let rot = p.f32("rotation").to_radians();
    let (fh, fv) = (p.b("flip_horizontal"), p.b("flip_vertical"));
    let (w, h) = (img.width() as f32, img.height() as f32);
    let border = 0.08 * w.min(h);
    map_px(img, |x, y, q| {
        // Frame space: centred, rotated, scaled, flipped.
        let (mut u, mut v) = (x as f32 - w / 2.0, y as f32 - h / 2.0);
        let (s, c) = (-rot).sin_cos();
        (u, v) = (u * c - v * s, u * s + v * c);
        u /= sx.max(0.01);
        v /= sy.max(0.01);
        if fh {
            u = -u;
        }
        if fv {
            v = -v;
        }
        // Distance inside the frame's inner edge (negative: on the frame).
        let (hw, hh) = (w / 2.0 - border, h / 2.0 - border);
        let mut d = match style {
            1 => {
                // Inside a rounded rectangle: minus its signed distance.
                let r = border * 2.0;
                let (qx, qy) = (u.abs() - (hw - r), v.abs() - (hh - r));
                let outside =
                    (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0);
                r - outside
            }
            _ => (hw - u.abs()).min(hh - v.abs()),
        };
        if style == 2 {
            d += (fractal(x as f32, y as f32, border * 0.5, 321) - 0.5) * border * 1.2;
        }
        if style == 4 {
            d += (value_noise(x as f32 * 0.3, y as f32 * 3.0, 4.0, 322) - 0.5) * border;
        }
        let soft = if style == 3 {
            border
        } else {
            1.0 + feather * border
        };
        let t = ((-d) / soft + 0.5).clamp(0.0, 1.0) * opacity;
        Rgba([
            clamp8(q[0] as f32 + (col[0] as f32 - q[0] as f32) * t),
            clamp8(q[1] as f32 + (col[1] as f32 - q[1] as f32) * t),
            clamp8(q[2] as f32 + (col[2] as f32 - q[2] as f32) * t),
            clamp8(q[3] as f32 + (255.0 - q[3] as f32) * t),
        ])
    })
}

pub fn glass_tiles(img: &RgbaImage, p: &P) -> RgbaImage {
    let (bw, bh) = (p.f32("width").max(1.0), p.f32("height").max(1.0));
    remap(img, false, |x, y| {
        let (cx, cy) = (
            (x / bw).floor() * bw + bw / 2.0,
            (y / bh).floor() * bh + bh / 2.0,
        );
        // Each block magnifies its middle a little.
        (cx + (x - cx) * 0.6, cy + (y - cy) * 0.6)
    })
}

pub fn mosaic(img: &RgbaImage, p: &P) -> RgbaImage {
    let size = p.f32("size");
    let bg = p.rgb("background_color");
    let vig = p.b("vignette");
    let (w, h) = (img.width() as f32, img.height() as f32);
    map_px(img, |x, y, q| {
        let (cx, cy, edge) = voronoi(x as f32, y as f32, size, 331);
        // Grout between the tiles.
        if edge < 1.2 {
            return Rgba([bg[0], bg[1], bg[2], q[3]]);
        }
        let c = sample(img, cx, cy);
        let mut k = 1.0;
        if vig {
            let (dx, dy) = (x as f32 / w - 0.5, y as f32 / h - 0.5);
            k = 1.0 - ((dx * dx + dy * dy) * 2.0).min(1.0) * 0.7;
        }
        Rgba([
            clamp8(c[0] as f32 * k + bg[0] as f32 * (1.0 - k)),
            clamp8(c[1] as f32 * k + bg[1] as f32 * (1.0 - k)),
            clamp8(c[2] as f32 * k + bg[2] as f32 * (1.0 - k)),
            q[3],
        ])
    })
}

pub fn scatter(img: &RgbaImage, p: &P) -> RgbaImage {
    let (sx, sy) = (p.f32("horizontal"), p.f32("vertical"));
    map_px(img, |x, y, _| {
        let dx = (hash01(x, y, 341) - 0.5) * 2.0 * sx;
        let dy = (hash01(x, y, 342) - 0.5) * 2.0 * sy;
        sample(img, x as f32 + dx, y as f32 + dy)
    })
}

pub fn tinted_glass(img: &RgbaImage, p: &P) -> RgbaImage {
    let tint = p.rgb("tint");
    let t = p.f32("percentage") / 100.0;
    let b = gaussian(img, p.f32("blur") / 4.0);
    map_px(&b, |_, _, q| {
        Rgba([
            clamp8(q[0] as f32 + (tint[0] as f32 - q[0] as f32) * t),
            clamp8(q[1] as f32 + (tint[1] as f32 - q[1] as f32) * t),
            clamp8(q[2] as f32 + (tint[2] as f32 - q[2] as f32) * t),
            q[3],
        ])
    })
}

pub fn stained_glass(img: &RgbaImage, p: &P) -> RgbaImage {
    let size = p.f32("size");
    let light = p.f32("light_intensity") / 10.0;
    let solder = p.f32("solder_width");
    let sc = p.rgb("solder_color");
    let lit = p.b("lighting_3d");
    map_px(img, |x, y, q| {
        let (cx, cy, edge) = voronoi(x as f32, y as f32, size, 351);
        if edge < solder {
            return Rgba([sc[0], sc[1], sc[2], q[3]]);
        }
        let c = sample(img, cx, cy);
        let mut k = 1.0 + light * 0.3;
        if lit {
            k *= 0.8 + 0.4 * ((edge - solder) / size).min(1.0);
        }
        Rgba([
            clamp8(c[0] as f32 * k),
            clamp8(c[1] as f32 * k),
            clamp8(c[2] as f32 * k),
            q[3],
        ])
    })
}

pub fn vignette(img: &RgbaImage, p: &P) -> RgbaImage {
    let col = match p.i("vignette_color") {
        0 => [0u8; 3],
        1 => [255u8; 3],
        _ => p.rgb("other_color"),
    };
    let shape = p.i("vignette_shape");
    let offset = p.f32("offset") / 100.0;
    let fade = (p.f32("fade") / 100.0).max(0.01);
    let (w, h) = (img.width() as f32, img.height() as f32);
    map_px(img, |x, y, q| {
        let (mut dx, mut dy) = (
            (x as f32 + 0.5) / w * 2.0 - 1.0,
            (y as f32 + 0.5) / h * 2.0 - 1.0,
        );
        if shape == 1 || shape == 3 {
            let m = w.min(h);
            dx *= w / m;
            dy *= h / m;
        }
        let d = if shape >= 2 {
            dx.abs().max(dy.abs())
        } else {
            (dx * dx + dy * dy).sqrt()
        };
        // The clear area's edge sits at `offset`; the frame fades in over `fade`.
        let inner = offset * 0.8;
        let t = ((d - inner) / fade).clamp(0.0, 1.0);
        Rgba([
            clamp8(q[0] as f32 + (col[0] as f32 - q[0] as f32) * t),
            clamp8(q[1] as f32 + (col[1] as f32 - q[1] as f32) * t),
            clamp8(q[2] as f32 + (col[2] as f32 - q[2] as f32) * t),
            q[3],
        ])
    })
}

pub fn vortex(img: &RgbaImage, p: &P) -> RgbaImage {
    let style = p.i("vortex_style");
    let size = p.f32("size").max(2.0)
        * match style {
            1 => 2.0,
            2 => 0.5,
            _ => 1.0,
        };
    let inner = p.f32("inner_direction").to_radians();
    let outer = p.f32("outer_direction").to_radians();
    let (w, h) = (img.width() as f32, img.height() as f32);
    let r_max = (w * w + h * h).sqrt() / 2.0;
    let base = remap(img, false, |x, y| {
        let (dx, dy) = (x - w / 2.0, y - h / 2.0);
        let r = (dx * dx + dy * dy).sqrt();
        let t = (r / r_max).min(1.0);
        let turn = (inner * (1.0 - t) + outer * t) * 0.15 * (1.0 + (r / size).sin() * 0.3);
        let a = dy.atan2(dx) + turn;
        (w / 2.0 + r * a.cos(), h / 2.0 + r * a.sin())
    });
    if style == 3 {
        return mix(&base, img, 0.35);
    }
    base
}
