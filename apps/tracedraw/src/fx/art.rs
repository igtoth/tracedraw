//! Effects > Art Strokes: hand-made looks.

use super::util::*;
use super::*;
use image::{ImageBuffer, Rgba, RgbaImage};

pub const EFFECTS: &[EffectSpec] = &[
    EffectSpec {
        id: "charcoal",
        params: &[
            range("size", 1.0, 10.0, 3.0, 1.0),
            range("edge", 1.0, 10.0, 5.0, 1.0),
        ],
        apply: charcoal,
        reach: no_reach,
    },
    EffectSpec {
        id: "conte_crayon",
        params: &[
            range("pressure", 1.0, 100.0, 50.0, 1.0),
            range("texture", 1.0, 100.0, 50.0, 1.0),
            choice("crayon_color", &["black", "sepia", "sanguine", "gray"], 0),
            color("paper_color", 0xFFFFFF),
        ],
        apply: conte_crayon,
        reach: no_reach,
    },
    EffectSpec {
        id: "crayon",
        params: &[
            range("size", 1.0, 20.0, 5.0, 1.0),
            range("outline", 0.0, 100.0, 30.0, 1.0),
        ],
        apply: crayon,
        reach: no_reach,
    },
    EffectSpec {
        id: "cubist",
        params: &[
            range("size", 2.0, 20.0, 10.0, 1.0),
            range("brightness", 0.0, 100.0, 50.0, 1.0),
            color("paper_color", 0xFFFFFF),
        ],
        apply: cubist,
        reach: no_reach,
    },
    EffectSpec {
        id: "impressionist",
        params: &[
            choice("style", &["strokes", "dabs"], 0),
            range("size", 1.0, 20.0, 5.0, 1.0),
            range("coloration", 0.0, 100.0, 20.0, 1.0),
            range("brightness", 0.0, 100.0, 50.0, 1.0),
        ],
        apply: impressionist,
        reach: no_reach,
    },
    EffectSpec {
        id: "palette_knife",
        params: &[
            range("blade_size", 1.0, 20.0, 6.0, 1.0),
            range("soft_edge", 0.0, 100.0, 50.0, 1.0),
            angle("angle", 45.0),
        ],
        apply: palette_knife,
        reach: no_reach,
    },
    EffectSpec {
        id: "pastels",
        params: &[
            choice("pastel_type", &["soft_pastel", "oil_pastel"], 0),
            range("stroke_size", 1.0, 20.0, 5.0, 1.0),
            range("hue_variation", 0.0, 100.0, 20.0, 1.0),
        ],
        apply: pastels,
        reach: no_reach,
    },
    EffectSpec {
        id: "pen_ink",
        params: &[
            choice("pen_style", &["crosshatch", "stippling"], 0),
            range("density", 1.0, 100.0, 50.0, 1.0),
            range("ink_pools", 1.0, 100.0, 50.0, 1.0),
        ],
        apply: pen_ink,
        reach: no_reach,
    },
    EffectSpec {
        id: "pointillist",
        params: &[
            range("size", 1.0, 30.0, 5.0, 1.0),
            range("brightness", 0.0, 100.0, 50.0, 1.0),
        ],
        apply: pointillist,
        reach: no_reach,
    },
    EffectSpec {
        id: "scraperboard",
        params: &[
            choice("paint", &["color", "black_white"], 0),
            range("density", 1.0, 100.0, 50.0, 1.0),
            range("size", 1.0, 20.0, 5.0, 1.0),
        ],
        apply: scraperboard,
        reach: no_reach,
    },
    EffectSpec {
        id: "sketch_pad",
        params: &[
            choice("pencil_type", &["graphite", "colored_pencil"], 0),
            range("style", 1.0, 100.0, 50.0, 1.0),
            range("pressure", 1.0, 100.0, 50.0, 1.0),
            range("outline", 0.0, 100.0, 50.0, 1.0),
        ],
        apply: sketch_pad,
        reach: no_reach,
    },
    EffectSpec {
        id: "watercolor",
        params: &[
            range("brush_size", 1.0, 10.0, 3.0, 1.0),
            range("granulation", 1.0, 100.0, 50.0, 1.0),
            range("water", 1.0, 100.0, 50.0, 1.0),
            range("bleed", 1.0, 100.0, 50.0, 1.0),
            range("brightness", 0.0, 100.0, 50.0, 1.0),
        ],
        apply: watercolor,
        reach: no_reach,
    },
    EffectSpec {
        id: "felt_marker",
        params: &[
            choice("variation", &["default", "order", "random"], 0),
            range("size", 1.0, 20.0, 5.0, 1.0),
            range("color_variation", 0.0, 100.0, 20.0, 1.0),
        ],
        apply: felt_marker,
        reach: no_reach,
    },
    EffectSpec {
        id: "paper_grain",
        params: &[
            choice("paint", &["color", "black_white"], 0),
            range("brush_pressure", 1.0, 100.0, 50.0, 1.0),
        ],
        apply: paper_grain,
        reach: no_reach,
    },
];

/// Sobel edge strength 0..1 of the brightness.
fn edges(img: &RgbaImage) -> Vec<f32> {
    let (w, h) = img.dimensions();
    let l = heights(img);
    let at = |x: i64, y: i64| -> f32 {
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
            out[(y as u32 * w + x as u32) as usize] = (gx * gx + gy * gy).sqrt().min(1.0);
        }
    }
    out
}

fn gray(v: f32, a: u8) -> Rgba<u8> {
    let g = clamp8(v);
    Rgba([g, g, g, a])
}

/// A stroke texture along `angle` degrees: streaks `len` pixels long.
fn streaks(x: u32, y: u32, angle: f32, len: f32, seed: u32) -> f32 {
    let (s, c) = angle.to_radians().sin_cos();
    let u = x as f32 * c + y as f32 * s;
    let v = -(x as f32) * s + y as f32 * c;
    value_noise(u / len.max(1.0) * 0.6, v * 1.3, 1.0, seed)
}

pub fn charcoal(img: &RgbaImage, p: &P) -> RgbaImage {
    let size = p.f32("size");
    let blurred = gaussian(img, size * 0.6);
    let e = edges(&blurred);
    let k = p.f32("edge") / 5.0;
    let w = img.width();
    map_px(&blurred, |x, y, q| {
        let dark = (1.0 - luma(q) / 255.0) * 0.85 + e[(y * w + x) as usize] * k;
        let grain = streaks(x, y, 50.0, size * 3.0, 7);
        let v = 255.0 * (1.0 - (dark * (0.75 + 0.5 * grain)).clamp(0.0, 1.0));
        gray(v, img.get_pixel(x, y)[3])
    })
}

pub fn conte_crayon(img: &RgbaImage, p: &P) -> RgbaImage {
    let crayon: [f32; 3] = match p.i("crayon_color") {
        1 => [112.0, 66.0, 20.0],
        2 => [150.0, 40.0, 30.0],
        3 => [90.0, 90.0, 90.0],
        _ => [20.0, 20.0, 20.0],
    };
    let paper = p.rgb("paper_color").map(|v| v as f32);
    let pressure = p.f32("pressure") / 100.0;
    let texture = p.f32("texture") / 100.0;
    map_px(img, |x, y, q| {
        let dark = 1.0 - luma(q) / 255.0;
        let grain = fractal(x as f32, y as f32, 6.0, 11);
        let t = (dark * (0.5 + pressure) - texture * 0.35 * (grain - 0.5)).clamp(0.0, 1.0);
        let c: [f32; 3] = std::array::from_fn(|k| paper[k] + (crayon[k] - paper[k]) * t);
        Rgba([clamp8(c[0]), clamp8(c[1]), clamp8(c[2]), q[3]])
    })
}

pub fn crayon(img: &RgbaImage, p: &P) -> RgbaImage {
    let size = p.f32("size");
    let base = box_blur(img, (size / 3.0) as u32);
    let e = edges(&base);
    let outline = p.f32("outline") / 100.0;
    let w = img.width();
    map_px(&base, |x, y, q| {
        let grain = hash01(x / 2, y / 2, 3) * 0.35 + streaks(x, y, 30.0, size * 2.0, 5) * 0.3;
        let paper = 1.0 - (grain - 0.3).max(0.0);
        let edge = 1.0 - e[(y * w + x) as usize] * outline * 2.0;
        let k = (paper * edge).clamp(0.0, 1.0);
        let f = |v: u8| clamp8(255.0 - (255.0 - v as f32) * k * 1.1);
        Rgba([f(q[0]), f(q[1]), f(q[2]), img.get_pixel(x, y)[3]])
    })
}

pub fn cubist(img: &RgbaImage, p: &P) -> RgbaImage {
    let size = p.u("size").max(2);
    let bright = (p.f32("brightness") - 50.0) * 1.5;
    let paper = p.rgb("paper_color");
    let (w, h) = img.dimensions();
    let mut out: RgbaImage = ImageBuffer::from_fn(w, h, |x, y| {
        Rgba([paper[0], paper[1], paper[2], img.get_pixel(x, y)[3]])
    });
    // Squares at jittered places, each the colour under its centre.
    let step = (size / 2).max(1);
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let jx = (hash01(x, y, 21) * size as f32) as i64 - size as i64 / 2;
            let jy = (hash01(x, y, 22) * size as f32) as i64 - size as i64 / 2;
            let cx = (x as i64 + jx).clamp(0, w as i64 - 1) as u32;
            let cy = (y as i64 + jy).clamp(0, h as i64 - 1) as u32;
            let c = *img.get_pixel(cx, cy);
            let half = (size as f32 * (0.6 + 0.4 * hash01(x, y, 23))) as i64 / 2;
            for dy in -half..=half {
                for dx in -half..=half {
                    let (px, py) = (cx as i64 + dx, cy as i64 + dy);
                    if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                        let a = out.get_pixel(px as u32, py as u32)[3];
                        out.put_pixel(
                            px as u32,
                            py as u32,
                            Rgba([
                                clamp8(c[0] as f32 + bright),
                                clamp8(c[1] as f32 + bright),
                                clamp8(c[2] as f32 + bright),
                                a,
                            ]),
                        );
                    }
                }
            }
            x += step + step / 2;
        }
        y += step + step / 2;
    }
    out
}

pub fn impressionist(img: &RgbaImage, p: &P) -> RgbaImage {
    let size = p.u("size").max(1);
    let strokes = p.i("style") == 0;
    let coloration = p.f32("coloration") / 100.0;
    let bright = (p.f32("brightness") - 50.0) * 1.2;
    let (w, h) = img.dimensions();
    let mut out = img.clone();
    let step = (size / 2).max(1);
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let c = *img.get_pixel(x, y);
            let jit = |s: u32| (hash01(x, y, s) - 0.5) * 80.0 * coloration;
            let col = [
                clamp8(c[0] as f32 + bright + jit(1)),
                clamp8(c[1] as f32 + bright + jit(2)),
                clamp8(c[2] as f32 + bright + jit(3)),
            ];
            let ang = hash01(x, y, 4) * std::f32::consts::PI;
            let (len, wid) = if strokes {
                (size as f32 * 1.5, size as f32 * 0.35)
            } else {
                (size as f32 * 0.6, size as f32 * 0.6)
            };
            let r = len.ceil() as i64;
            for dy in -r..=r {
                for dx in -r..=r {
                    let (u, v) = (
                        dx as f32 * ang.cos() + dy as f32 * ang.sin(),
                        -(dx as f32) * ang.sin() + dy as f32 * ang.cos(),
                    );
                    if (u / len).powi(2) + (v / wid.max(0.5)).powi(2) <= 1.0 {
                        let (px, py) = (x as i64 + dx, y as i64 + dy);
                        if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                            let a = img.get_pixel(px as u32, py as u32)[3];
                            out.put_pixel(px as u32, py as u32, Rgba([col[0], col[1], col[2], a]));
                        }
                    }
                }
            }
            x += step;
        }
        y += step;
    }
    out
}

pub fn palette_knife(img: &RgbaImage, p: &P) -> RgbaImage {
    let len = p.f32("blade_size") * 2.0;
    let soft = p.f32("soft_edge") / 100.0;
    let (s, c) = (
        p.f32("angle").to_radians().sin(),
        p.f32("angle").to_radians().cos(),
    );
    let steps = len.ceil().max(1.0) as i32;
    let smeared = map_px(img, |x, y, q| {
        // Paint dragged along the blade: the strongest colour over the
        // stroke, softened toward its average.
        let mut acc = [0f32; 3];
        let mut darkest = *q;
        for k in -steps..=steps {
            let t = k as f32 / 2.0;
            let sp = sample(img, x as f32 + c * t, y as f32 - s * t);
            for i in 0..3 {
                acc[i] += sp[i] as f32;
            }
            if luma(&sp) < luma(&darkest) {
                darkest = sp;
            }
        }
        let n = (2 * steps + 1) as f32;
        Rgba([
            clamp8(darkest[0] as f32 * (1.0 - soft) + acc[0] / n * soft),
            clamp8(darkest[1] as f32 * (1.0 - soft) + acc[1] / n * soft),
            clamp8(darkest[2] as f32 * (1.0 - soft) + acc[2] / n * soft),
            q[3],
        ])
    });
    // Blocks of paint: hold each colour over short stretches.
    let cell = (len / 2.0).max(1.0) as u32;
    map_px(&smeared, |x, y, q| {
        let bx = x / cell * cell;
        let by = y / cell * cell;
        let seg = sample(
            &smeared,
            bx as f32 + cell as f32 / 2.0,
            by as f32 + cell as f32 / 2.0,
        );
        let t = 0.35 * (1.0 - soft);
        Rgba([
            clamp8(q[0] as f32 * (1.0 - t) + seg[0] as f32 * t),
            clamp8(q[1] as f32 * (1.0 - t) + seg[1] as f32 * t),
            clamp8(q[2] as f32 * (1.0 - t) + seg[2] as f32 * t),
            q[3],
        ])
    })
}

pub fn pastels(img: &RgbaImage, p: &P) -> RgbaImage {
    let size = p.f32("stroke_size");
    let oil = p.i("pastel_type") == 1;
    let hv = p.f32("hue_variation") / 100.0;
    let base = gaussian(img, size * 0.4);
    map_px(&base, |x, y, q| {
        let streak = streaks(x, y, 60.0, size * 3.0, 31);
        let paper = if oil {
            0.9 + 0.1 * streak
        } else {
            0.7 + 0.3 * streak
        };
        let (h, s, l) = rgb_to_hsl(
            q[0] as f32 / 255.0,
            q[1] as f32 / 255.0,
            q[2] as f32 / 255.0,
        );
        let h2 = h + (hash01(x / 3, y / 3, 32) - 0.5) * 60.0 * hv;
        let l2 = (l * paper + (1.0 - paper) * 0.95).clamp(0.0, 1.0);
        let (r, g, b) = hsl_to_rgb(h2, (s * if oil { 1.15 } else { 0.85 }).min(1.0), l2);
        Rgba([
            clamp8(r * 255.0),
            clamp8(g * 255.0),
            clamp8(b * 255.0),
            img.get_pixel(x, y)[3],
        ])
    })
}

pub fn pen_ink(img: &RgbaImage, p: &P) -> RgbaImage {
    let density = p.f32("density") / 100.0;
    let pools = p.f32("ink_pools") / 100.0;
    let stipple = p.i("pen_style") == 1;
    map_px(img, |x, y, q| {
        let dark = 1.0 - luma(q) / 255.0;
        let ink = if stipple {
            hash01(x, y, 41) < dark * (0.4 + density * 0.8)
        } else {
            let spacing = (8.0 - density * 5.0).max(2.0);
            let l1 = ((x + y) as f32 % spacing) < 1.0 && dark > 0.2;
            let l2 =
                (((x as i64 - y as i64).rem_euclid(spacing as i64)) as f32) < 1.0 && dark > 0.45;
            let l3 = (x as f32 % spacing) < 1.0 && dark > 0.7;
            l1 || l2 || l3 || dark > 0.97 - pools * 0.2
        };
        gray(if ink { 0.0 } else { 255.0 }, q[3])
    })
}

pub fn pointillist(img: &RgbaImage, p: &P) -> RgbaImage {
    let size = p.u("size").max(1);
    let bright = (p.f32("brightness") - 50.0) * 1.2;
    let (w, h) = img.dimensions();
    let mut out: RgbaImage = ImageBuffer::from_fn(w, h, |x, y| {
        let a = img.get_pixel(x, y)[3];
        Rgba([255, 255, 255, a])
    });
    let r = (size as f32 * 0.6).max(0.7);
    let step = size.max(1);
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let cx = x as f32 + (hash01(x, y, 51) - 0.5) * size as f32;
            let cy = y as f32 + (hash01(x, y, 52) - 0.5) * size as f32;
            let c = sample(img, cx, cy);
            let col = [
                clamp8(c[0] as f32 + bright),
                clamp8(c[1] as f32 + bright),
                clamp8(c[2] as f32 + bright),
            ];
            let ri = r.ceil() as i64;
            for dy in -ri..=ri {
                for dx in -ri..=ri {
                    if (dx * dx + dy * dy) as f32 <= r * r {
                        let (px, py) = (cx as i64 + dx, cy as i64 + dy);
                        if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                            let a = out.get_pixel(px as u32, py as u32)[3];
                            out.put_pixel(px as u32, py as u32, Rgba([col[0], col[1], col[2], a]));
                        }
                    }
                }
            }
            x += step;
        }
        y += step;
    }
    out
}

pub fn scraperboard(img: &RgbaImage, p: &P) -> RgbaImage {
    let density = p.f32("density") / 100.0;
    let size = p.f32("size");
    let color = p.i("paint") == 0;
    map_px(img, |x, y, q| {
        let light = luma(q) / 255.0;
        let s = streaks(
            x,
            y,
            120.0 + 30.0 * value_noise(x as f32, y as f32, 40.0, 61),
            size * 2.0,
            62,
        );
        let scraped = s < light * (0.6 + density * 0.6) - 0.1;
        if !scraped {
            return Rgba([0, 0, 0, q[3]]);
        }
        if color {
            *q
        } else {
            Rgba([255, 255, 255, q[3]])
        }
    })
}

pub fn sketch_pad(img: &RgbaImage, p: &P) -> RgbaImage {
    let style = p.f32("style") / 100.0;
    let pressure = p.f32("pressure") / 100.0;
    let outline = p.f32("outline") / 100.0;
    let colored = p.i("pencil_type") == 1;
    let e = edges(&gaussian(img, 0.8));
    let w = img.width();
    map_px(img, |x, y, q| {
        let dark = 1.0 - luma(q) / 255.0;
        let hatch = streaks(x, y, 45.0 + style * 30.0, 6.0, 71);
        let shade = (dark * (0.3 + pressure * 0.7) - (hatch - 0.5) * 0.4).clamp(0.0, 1.0);
        let line = e[(y * w + x) as usize] * outline * 2.5;
        let v = (1.0 - (shade * 0.8 + line).clamp(0.0, 1.0)).clamp(0.0, 1.0);
        if colored {
            Rgba([
                clamp8(255.0 - (255.0 - q[0] as f32) * (1.0 - v)),
                clamp8(255.0 - (255.0 - q[1] as f32) * (1.0 - v)),
                clamp8(255.0 - (255.0 - q[2] as f32) * (1.0 - v)),
                q[3],
            ])
        } else {
            gray(v * 255.0, q[3])
        }
    })
}

pub fn watercolor(img: &RgbaImage, p: &P) -> RgbaImage {
    let brush = p.u("brush_size").max(1);
    let gran = p.f32("granulation") / 100.0;
    let water = p.f32("water") / 100.0;
    let bleed = p.f32("bleed") / 100.0;
    let bright = (p.f32("brightness") - 50.0) * 1.0;
    let washed = rank(&gaussian(img, bleed * 2.0 + 0.3), brush, 0.5);
    let e = edges(&washed);
    let w = img.width();
    map_px(&washed, |x, y, q| {
        let g = (fractal(x as f32, y as f32, 4.0, 81) - 0.5) * gran * 40.0;
        let rim = e[(y * w + x) as usize] * 60.0 * water;
        let f = |v: u8| {
            clamp8(v as f32 * (1.0 - 0.15 * water) + 255.0 * 0.15 * water + g - rim + bright)
        };
        Rgba([f(q[0]), f(q[1]), f(q[2]), img.get_pixel(x, y)[3]])
    })
}

pub fn felt_marker(img: &RgbaImage, p: &P) -> RgbaImage {
    let size = p.u("size").max(1);
    let cv = p.f32("color_variation") / 100.0;
    let var = p.i("variation");
    let (w, h) = img.dimensions();
    let mut out = map_px(img, |_, _, q| Rgba([255, 255, 255, q[3]]));
    let step = size.max(1);
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let c = *img.get_pixel(x, y);
            let ang = match var {
                1 => 0.0,
                2 => hash01(x, y, 91) * std::f32::consts::PI,
                _ => 0.35,
            };
            let jit = |s: u32| (hash01(x, y, s) - 0.5) * 70.0 * cv;
            let col = [
                clamp8(c[0] as f32 + jit(92)),
                clamp8(c[1] as f32 + jit(93)),
                clamp8(c[2] as f32 + jit(94)),
            ];
            let len = size as f32 * 1.6;
            let wid = size as f32 * 0.7;
            let r = len.ceil() as i64;
            for dy in -r..=r {
                for dx in -r..=r {
                    let (u, v) = (
                        dx as f32 * ang.cos() + dy as f32 * ang.sin(),
                        -(dx as f32) * ang.sin() + dy as f32 * ang.cos(),
                    );
                    if u.abs() <= len && v.abs() <= wid {
                        let (px, py) = (x as i64 + dx, y as i64 + dy);
                        if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                            let o = out.get_pixel(px as u32, py as u32);
                            // Markers darken what is under them.
                            let m = |a: u8, b: u8| ((a as u16 * b as u16) / 255) as u8;
                            let a = o[3];
                            let mixed =
                                Rgba([m(o[0], col[0]), m(o[1], col[1]), m(o[2], col[2]), a]);
                            let keep = if o[0] == 255 && o[1] == 255 && o[2] == 255 {
                                Rgba([col[0], col[1], col[2], a])
                            } else {
                                mixed
                            };
                            out.put_pixel(px as u32, py as u32, keep);
                        }
                    }
                }
            }
            x += step * 2;
        }
        y += step;
    }
    out
}

pub fn paper_grain(img: &RgbaImage, p: &P) -> RgbaImage {
    let pressure = p.f32("brush_pressure") / 100.0;
    let bw = p.i("paint") == 1;
    map_px(img, |x, y, q| {
        let wave = ((x as f32 * 0.5 + (y as f32 * 0.25).sin() * 3.0).sin() * 0.5 + 0.5) * 0.6
            + hash01(x, y, 101) * 0.4;
        let k = 1.0 - (wave - 0.5) * (0.3 + pressure * 0.5);
        if bw {
            gray(luma(q) * k, q[3])
        } else {
            Rgba([
                clamp8(q[0] as f32 * k),
                clamp8(q[1] as f32 * k),
                clamp8(q[2] as f32 * k),
                q[3],
            ])
        }
    })
}
