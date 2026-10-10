//! Pixel helpers the bitmap effects share. Images are RGBA8, straight
//! alpha; effects keep alpha unless they say otherwise.

use image::{ImageBuffer, Rgba, RgbaImage};

pub fn clamp8(v: f32) -> u8 {
    if v.is_nan() {
        return 0;
    }
    v.round().clamp(0.0, 255.0) as u8
}

pub fn luma(p: &Rgba<u8>) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

pub fn rgb(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

/// A new image from `f(x, y, pixel)`.
pub fn map_px(img: &RgbaImage, f: impl Fn(u32, u32, &Rgba<u8>) -> Rgba<u8>) -> RgbaImage {
    let mut out = ImageBuffer::new(img.width(), img.height());
    for (x, y, p) in img.enumerate_pixels() {
        out.put_pixel(x, y, f(x, y, p));
    }
    out
}

/// Each colour channel through a lookup table.
pub fn map_lut(img: &RgbaImage, lut: &[u8; 256]) -> RgbaImage {
    map_px(img, |_, _, p| {
        Rgba([
            lut[p[0] as usize],
            lut[p[1] as usize],
            lut[p[2] as usize],
            p[3],
        ])
    })
}

/// The pixel nearest (x, y), clamped to the image.
pub fn sample(img: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    let (w, h) = (img.width() as i64, img.height() as i64);
    if w == 0 || h == 0 {
        return Rgba([0, 0, 0, 0]);
    }
    let xi = (x.round() as i64).clamp(0, w - 1);
    let yi = (y.round() as i64).clamp(0, h - 1);
    *img.get_pixel(xi as u32, yi as u32)
}

/// Bilinear sample; outside the image gives transparent when `clear`.
pub fn bilinear(img: &RgbaImage, x: f32, y: f32, clear: bool) -> Rgba<u8> {
    let (w, h) = (img.width() as f32, img.height() as f32);
    if w < 1.0 || h < 1.0 {
        return Rgba([0, 0, 0, 0]);
    }
    if clear && (x < -0.5 || y < -0.5 || x > w - 0.5 || y > h - 0.5) {
        return Rgba([0, 0, 0, 0]);
    }
    let x = x.clamp(0.0, w - 1.0);
    let y = y.clamp(0.0, h - 1.0);
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = x - x0;
    let fy = y - y0;
    // Premultiplied, so a transparent neighbour adds no (black) colour.
    let mut acc = [0f32; 4];
    for (p, k) in [
        (sample(img, x0, y0), (1.0 - fx) * (1.0 - fy)),
        (sample(img, x0 + 1.0, y0), fx * (1.0 - fy)),
        (sample(img, x0, y0 + 1.0), (1.0 - fx) * fy),
        (sample(img, x0 + 1.0, y0 + 1.0), fx * fy),
    ] {
        let a = p[3] as f32 * k;
        for c in 0..3 {
            acc[c] += p[c] as f32 * a;
        }
        acc[3] += a;
    }
    unpremultiply(acc, 1.0)
}

/// Every output pixel from the source point `f(x, y)` gives.
pub fn remap(img: &RgbaImage, clear: bool, f: impl Fn(f32, f32) -> (f32, f32)) -> RgbaImage {
    let mut out = ImageBuffer::new(img.width(), img.height());
    for y in 0..img.height() {
        for x in 0..img.width() {
            let (sx, sy) = f(x as f32, y as f32);
            out.put_pixel(x, y, bilinear(img, sx, sy, clear));
        }
    }
    out
}

/// Premultiplied pixels as floats: colour times alpha (0..1), alpha 0..255.
fn premultiplied(img: &RgbaImage) -> Vec<[f32; 4]> {
    img.pixels()
        .map(|p| {
            let a = p[3] as f32 / 255.0;
            [
                p[0] as f32 * a,
                p[1] as f32 * a,
                p[2] as f32 * a,
                p[3] as f32,
            ]
        })
        .collect()
}

fn straight(buf: &[[f32; 4]], w: u32, h: u32) -> RgbaImage {
    ImageBuffer::from_fn(w, h, |x, y| {
        let p = buf[(y * w + x) as usize];
        if p[3] <= 0.5 {
            return Rgba([0, 0, 0, 0]);
        }
        let k = 255.0 / p[3];
        Rgba([
            clamp8(p[0] * k),
            clamp8(p[1] * k),
            clamp8(p[2] * k),
            clamp8(p[3]),
        ])
    })
}

/// One pass of a kernel along rows (`horizontal`) or columns; edges
/// repeat the last pixel.
fn convolve(buf: &[[f32; 4]], w: u32, h: u32, k: &[f32], horizontal: bool) -> Vec<[f32; 4]> {
    let r = (k.len() / 2) as i64;
    let (len, lines) = if horizontal {
        (w as i64, h as i64)
    } else {
        (h as i64, w as i64)
    };
    let at = |line: i64, i: i64| -> usize {
        let i = i.clamp(0, len - 1);
        if horizontal {
            (line * w as i64 + i) as usize
        } else {
            (i * w as i64 + line) as usize
        }
    };
    let mut out = vec![[0f32; 4]; buf.len()];
    for line in 0..lines {
        for i in 0..len {
            let mut acc = [0f32; 4];
            for (j, kv) in k.iter().enumerate() {
                let p = buf[at(line, i + j as i64 - r)];
                for c in 0..4 {
                    acc[c] += p[c] * kv;
                }
            }
            out[at(line, i)] = acc;
        }
    }
    out
}

/// A box average of radius `r` along rows or columns with a running sum.
fn box_pass(buf: &[[f32; 4]], w: u32, h: u32, r: i64, horizontal: bool) -> Vec<[f32; 4]> {
    let (len, lines) = if horizontal {
        (w as i64, h as i64)
    } else {
        (h as i64, w as i64)
    };
    let at = |line: i64, i: i64| -> usize {
        let i = i.clamp(0, len - 1);
        if horizontal {
            (line * w as i64 + i) as usize
        } else {
            (i * w as i64 + line) as usize
        }
    };
    let n = (2 * r + 1) as f32;
    let mut out = vec![[0f32; 4]; buf.len()];
    for line in 0..lines {
        let mut acc = [0f32; 4];
        for i in -r..=r {
            let p = buf[at(line, i)];
            for c in 0..4 {
                acc[c] += p[c];
            }
        }
        for i in 0..len {
            out[at(line, i)] = [acc[0] / n, acc[1] / n, acc[2] / n, acc[3] / n];
            let (add, sub) = (buf[at(line, i + r + 1)], buf[at(line, i - r)]);
            for c in 0..4 {
                acc[c] += add[c] - sub[c];
            }
        }
    }
    out
}

/// Radii of three box blurs whose result is close to a Gaussian of
/// `sigma` (their variances add up to sigma squared).
fn box_radii(sigma: f32) -> [i64; 3] {
    let n = 3.0;
    let ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut wl = ideal.floor() as i64;
    if wl % 2 == 0 {
        wl -= 1;
    }
    let wu = wl + 2;
    let wlf = wl as f32;
    let m = ((12.0 * sigma * sigma - n * wlf * wlf - 4.0 * n * wlf - 3.0 * n) / (-4.0 * wlf - 4.0))
        .round() as i64;
    std::array::from_fn(|i| {
        if (i as i64) < m {
            (wl - 1) / 2
        } else {
            (wu - 1) / 2
        }
    })
}

/// Gaussian blur of every channel with premultiplied alpha, so colours
/// do not darken toward transparent edges. Small blurs use the exact
/// kernel, larger ones three box blurs (the same spread, in time that
/// does not grow with the radius).
pub fn gaussian(img: &RgbaImage, sigma: f32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if sigma <= 0.05 || !sigma.is_finite() || w == 0 || h == 0 {
        return img.clone();
    }
    let mut buf = premultiplied(img);
    if sigma < 2.0 {
        let r = (sigma * 3.0).ceil() as i64;
        let mut k: Vec<f32> = (-r..=r)
            .map(|i| (-((i * i) as f32) / (2.0 * sigma * sigma)).exp())
            .collect();
        let sum: f32 = k.iter().sum();
        for v in k.iter_mut() {
            *v /= sum;
        }
        buf = convolve(&buf, w, h, &k, true);
        buf = convolve(&buf, w, h, &k, false);
    } else {
        for r in box_radii(sigma.min(4096.0)) {
            if r > 0 {
                buf = box_pass(&buf, w, h, r, true);
                buf = box_pass(&buf, w, h, r, false);
            }
        }
    }
    straight(&buf, w, h)
}

/// Add a pixel to a premultiplied sum (colour times alpha, and alpha).
pub fn add_premultiplied(acc: &mut [f32; 4], p: &Rgba<u8>) {
    let a = p[3] as f32;
    for c in 0..3 {
        acc[c] += p[c] as f32 * a;
    }
    acc[3] += a;
}

/// The average of `n` pixels summed by [`add_premultiplied`].
pub fn unpremultiply(acc: [f32; 4], n: f32) -> Rgba<u8> {
    if acc[3] <= 0.0 {
        return Rgba([0, 0, 0, 0]);
    }
    Rgba([
        clamp8(acc[0] / acc[3]),
        clamp8(acc[1] / acc[3]),
        clamp8(acc[2] / acc[3]),
        clamp8(acc[3] / n.max(1.0)),
    ])
}

/// `p` moved toward `q` by `t`, colour and alpha, with premultiplied
/// alpha so a transparent pixel's colour does not show.
pub fn lerp_premultiplied(p: &Rgba<u8>, q: &Rgba<u8>, t: f32) -> Rgba<u8> {
    let mut acc = [0f32; 4];
    let (a0, a1) = (p[3] as f32, q[3] as f32);
    for c in 0..3 {
        acc[c] = p[c] as f32 * a0 * (1.0 - t) + q[c] as f32 * a1 * t;
    }
    acc[3] = a0 * (1.0 - t) + a1 * t;
    unpremultiply(acc, 1.0)
}

/// The average over `2 rx + 1` by `2 ry + 1` pixels, alpha included
/// (premultiplied, so transparent pixels add no colour).
pub fn box_average(img: &RgbaImage, rx: u32, ry: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return img.clone();
    }
    let mut buf = premultiplied(img);
    if rx > 0 {
        buf = box_pass(&buf, w, h, rx as i64, true);
    }
    if ry > 0 {
        buf = box_pass(&buf, w, h, ry as i64, false);
    }
    straight(&buf, w, h)
}

/// Box blur of radius `r` (colour channels).
pub fn box_blur(img: &RgbaImage, r: u32) -> RgbaImage {
    if r == 0 {
        return img.clone();
    }
    let (w, h) = img.dimensions();
    let pass = |src: &RgbaImage, horizontal: bool| -> RgbaImage {
        let mut out = src.clone();
        let (len, lines) = if horizontal { (w, h) } else { (h, w) };
        for line in 0..lines {
            let px = |i: i64| -> &Rgba<u8> {
                let i = i.clamp(0, len as i64 - 1) as u32;
                if horizontal {
                    src.get_pixel(i, line)
                } else {
                    src.get_pixel(line, i)
                }
            };
            let mut acc = [0f32; 3];
            for i in -(r as i64)..=(r as i64) {
                let p = px(i);
                for c in 0..3 {
                    acc[c] += p[c] as f32;
                }
            }
            let n = (2 * r + 1) as f32;
            for i in 0..len as i64 {
                let a = px(i)[3];
                let v = Rgba([
                    clamp8(acc[0] / n),
                    clamp8(acc[1] / n),
                    clamp8(acc[2] / n),
                    a,
                ]);
                if horizontal {
                    out.put_pixel(i as u32, line, v);
                } else {
                    out.put_pixel(line, i as u32, v);
                }
                let (add, sub) = (px(i + r as i64 + 1), px(i - r as i64));
                for c in 0..3 {
                    acc[c] += add[c] as f32 - sub[c] as f32;
                }
            }
        }
        out
    };
    let a = pass(img, true);
    pass(&a, false)
}

/// Per-channel rank filter over a square of radius `r`: the median
/// (`rank` 0.5), minimum (0) or maximum (1). Small squares sort their
/// values; larger ones keep a running histogram along each row.
pub fn rank(img: &RgbaImage, r: u32, rank: f32) -> RgbaImage {
    if r == 0 || img.width() == 0 || img.height() == 0 {
        return img.clone();
    }
    let (w, h) = (img.width() as i64, img.height() as i64);
    let r = r as i64;
    let rank = rank.clamp(0.0, 1.0);
    let at = |x: i64, y: i64| img.get_pixel(x.clamp(0, w - 1) as u32, y.clamp(0, h - 1) as u32);
    let mut out = img.clone();
    if r <= 2 {
        let mut vals: [Vec<u8>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for y in 0..h {
            for x in 0..w {
                for v in vals.iter_mut() {
                    v.clear();
                }
                for dy in -r..=r {
                    for dx in -r..=r {
                        let p = at(x + dx, y + dy);
                        for c in 0..3 {
                            vals[c].push(p[c]);
                        }
                    }
                }
                let mut o = *img.get_pixel(x as u32, y as u32);
                for c in 0..3 {
                    vals[c].sort_unstable();
                    let i = ((vals[c].len() - 1) as f32 * rank).round() as usize;
                    o[c] = vals[c][i.min(vals[c].len() - 1)];
                }
                out.put_pixel(x as u32, y as u32, o);
            }
        }
        return out;
    }
    let count = ((2 * r + 1) * (2 * r + 1)) as u32;
    let target = ((count - 1) as f32 * rank).round() as u32;
    for y in 0..h {
        let mut hist = [[0u32; 256]; 3];
        for dy in -r..=r {
            for dx in -r..=r {
                let p = at(dx, y + dy);
                for c in 0..3 {
                    hist[c][p[c] as usize] += 1;
                }
            }
        }
        for x in 0..w {
            let mut o = *img.get_pixel(x as u32, y as u32);
            for c in 0..3 {
                let mut acc = 0;
                for (v, n) in hist[c].iter().enumerate() {
                    acc += n;
                    if acc > target {
                        o[c] = v as u8;
                        break;
                    }
                }
            }
            out.put_pixel(x as u32, y as u32, o);
            // Slide the window one pixel right.
            for dy in -r..=r {
                let (old, new) = (at(x - r, y + dy), at(x + r + 1, y + dy));
                for c in 0..3 {
                    hist[c][old[c] as usize] -= 1;
                    hist[c][new[c] as usize] += 1;
                }
            }
        }
    }
    out
}

/// Mix `b` over `a` by `t` (0 keeps `a`), alpha too: a blur mixed in
/// spreads into the transparent margin like the blur itself.
pub fn mix(a: &RgbaImage, b: &RgbaImage, t: f32) -> RgbaImage {
    let t = t.clamp(0.0, 1.0);
    map_px(a, |x, y, p| {
        let q = b.get_pixel(
            x.min(b.width().saturating_sub(1)),
            y.min(b.height().saturating_sub(1)),
        );
        if p[3] == q[3] {
            // The common case: straight blending is exact and cheaper.
            return Rgba([
                clamp8(p[0] as f32 * (1.0 - t) + q[0] as f32 * t),
                clamp8(p[1] as f32 * (1.0 - t) + q[1] as f32 * t),
                clamp8(p[2] as f32 * (1.0 - t) + q[2] as f32 * t),
                p[3],
            ]);
        }
        lerp_premultiplied(p, q, t)
    })
}

/// A stable pseudo-random value 0..1 for a pixel and a seed.
pub fn hash01(x: u32, y: u32, seed: u32) -> f32 {
    let mut h =
        x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77) ^ seed.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0xFF_FFFF) as f32 / 16_777_215.0
}

/// Smooth value noise 0..1 at (x, y) with cells of `scale` pixels.
pub fn value_noise(x: f32, y: f32, scale: f32, seed: u32) -> f32 {
    let s = scale.max(1.0);
    let (fx, fy) = (x / s, y / s);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let h = |i: f32, j: f32| hash01(i as i64 as u32, j as i64 as u32, seed);
    let a = h(x0, y0) * (1.0 - sx) + h(x0 + 1.0, y0) * sx;
    let b = h(x0, y0 + 1.0) * (1.0 - sx) + h(x0 + 1.0, y0 + 1.0) * sx;
    a * (1.0 - sy) + b * sy
}

/// Fractal noise: four octaves of value noise.
pub fn fractal(x: f32, y: f32, scale: f32, seed: u32) -> f32 {
    let mut v = 0.0;
    let mut amp = 0.5;
    let mut s = scale;
    for o in 0..4 {
        v += value_noise(x, y, s, seed.wrapping_add(o)) * amp;
        amp *= 0.5;
        s *= 0.5;
    }
    v / 0.9375
}

/// A height field (0..1) from the image's brightness.
pub fn heights(img: &RgbaImage) -> Vec<f32> {
    img.pixels().map(|p| luma(p) / 255.0).collect()
}

/// Shade a height field lit from `angle` degrees (0 = from the right,
/// counter-clockwise) at `depth`: 0.5 flat, more lit, less shadowed.
pub fn shade(h: &[f32], w: u32, hgt: u32, angle: f32, depth: f32) -> Vec<f32> {
    let (lx, ly) = (angle.to_radians().cos(), -angle.to_radians().sin());
    let at = |x: i64, y: i64| -> f32 {
        let x = x.clamp(0, w as i64 - 1) as u32;
        let y = y.clamp(0, hgt as i64 - 1) as u32;
        h[(y * w + x) as usize]
    };
    let mut out = vec![0.5; h.len()];
    for y in 0..hgt as i64 {
        for x in 0..w as i64 {
            let dx = at(x + 1, y) - at(x - 1, y);
            let dy = at(x, y + 1) - at(x, y - 1);
            let v = 0.5 + (dx * lx + dy * ly) * depth;
            out[(y as u32 * w + x as u32) as usize] = v.clamp(0.0, 1.0);
        }
    }
    out
}

pub fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s <= 0.0 {
        return (l, l, l);
    }
    let h = h.rem_euclid(360.0) / 360.0;
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let f = |t: f32| {
        let t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    (f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noisy(w: u32, h: u32) -> RgbaImage {
        ImageBuffer::from_fn(w, h, |x, y| {
            let v = |s| (hash01(x, y, s) * 255.0) as u8;
            Rgba([v(1), v(2), v(3), 255])
        })
    }

    #[test]
    fn gaussian_matches_the_exact_blur_and_keeps_edges_clean() {
        let img = noisy(64, 48);
        for sigma in [0.8f32, 3.0, 9.0] {
            let fast = gaussian(&img, sigma);
            let exact = image::imageops::blur(&img, sigma);
            let mean = fast
                .pixels()
                .zip(exact.pixels())
                .map(|(a, b)| (0..3).map(|c| a[c].abs_diff(b[c]) as u32).sum::<u32>())
                .sum::<u32>() as f32
                / (64.0 * 48.0 * 3.0);
            assert!(mean < 3.0, "sigma {sigma}: {mean}");
        }
        // White on transparent stays white where it spreads: no dark rim.
        let spot: RgbaImage = ImageBuffer::from_fn(40, 40, |x, y| {
            if (15..25).contains(&x) && (15..25).contains(&y) {
                Rgba([255, 255, 255, 255])
            } else {
                Rgba([0, 0, 0, 0])
            }
        });
        let b = gaussian(&spot, 4.0);
        for p in b.pixels().filter(|p| p[3] > 8) {
            assert!(p[0] > 245, "{p:?}");
        }
        assert!(b.get_pixel(12, 20)[3] > 0 && b.get_pixel(20, 20)[3] < 255);
    }

    #[test]
    fn rank_filters_agree_for_every_radius() {
        let img = noisy(23, 17);
        let naive = |r: i64, rank: f32| -> RgbaImage {
            let (w, h) = (img.width() as i64, img.height() as i64);
            ImageBuffer::from_fn(img.width(), img.height(), |x, y| {
                let mut o = *img.get_pixel(x, y);
                for c in 0..3 {
                    let mut v: Vec<u8> = (-r..=r)
                        .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
                        .map(|(dx, dy)| {
                            img.get_pixel(
                                (x as i64 + dx).clamp(0, w - 1) as u32,
                                (y as i64 + dy).clamp(0, h - 1) as u32,
                            )[c]
                        })
                        .collect();
                    v.sort_unstable();
                    o[c] = v[((v.len() - 1) as f32 * rank).round() as usize];
                }
                o
            })
        };
        for r in [1u32, 3, 6] {
            for k in [0.0f32, 0.5, 1.0] {
                assert_eq!(rank(&img, r, k), naive(r as i64, k), "r {r} rank {k}");
            }
        }
    }
}
