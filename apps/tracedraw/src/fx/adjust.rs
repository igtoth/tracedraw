//! Effects > Adjust, Transform and Correction: colour and tone
//! corrections.

use super::util::*;
use super::*;
use image::{Rgba, RgbaImage};

pub const EFFECTS: &[EffectSpec] = &[
    EffectSpec {
        id: "auto_adjust",
        params: &[],
        apply: auto_adjust,
        reach: no_reach,
    },
    EffectSpec {
        id: "image_adjustments",
        params: &[
            range("black_point", 0.0, 254.0, 0.0, 1.0),
            range("white_point", 1.0, 255.0, 255.0, 1.0),
            range("temperature", -100.0, 100.0, 0.0, 1.0),
            range("tint", -100.0, 100.0, 0.0, 1.0),
            range("saturation", -100.0, 100.0, 0.0, 1.0),
            range("brightness", -100.0, 100.0, 0.0, 1.0),
            range("contrast", -100.0, 100.0, 0.0, 1.0),
            range("highlights", -100.0, 100.0, 0.0, 1.0),
            range("shadows", -100.0, 100.0, 0.0, 1.0),
            range("midtones", -100.0, 100.0, 0.0, 1.0),
        ],
        apply: image_adjustments,
        reach: no_reach,
    },
    EffectSpec {
        id: "contrast_enhancement",
        params: &[
            choice("channel", &RGB_CHANNELS, 0),
            range("input_low", 0.0, 254.0, 0.0, 1.0),
            range("input_high", 1.0, 255.0, 255.0, 1.0),
            range("output_low", 0.0, 255.0, 0.0, 1.0),
            range("output_high", 0.0, 255.0, 255.0, 1.0),
            range("gamma", 0.1, 10.0, 1.0, 0.01),
        ],
        apply: contrast_enhancement,
        reach: no_reach,
    },
    EffectSpec {
        id: "local_equalization",
        params: &[
            range("width", 2.0, 255.0, 20.0, 1.0),
            range("height", 2.0, 255.0, 20.0, 1.0),
            check("lock", true),
        ],
        apply: local_equalization,
        reach: no_reach,
    },
    EffectSpec {
        id: "target_balance",
        params: &[
            choice("channel", &RGB_CHANNELS, 0),
            check("all_channels", false),
            color("low_sample", 0x000000),
            color("low_target", 0x000000),
            color("mid_sample", 0x808080),
            color("mid_target", 0x808080),
            color("high_sample", 0xFFFFFF),
            color("high_target", 0xFFFFFF),
        ],
        apply: target_balance,
        reach: no_reach,
    },
    EffectSpec {
        id: "tone_curve",
        params: &[],
        apply: tone_curve,
        reach: no_reach,
    },
    EffectSpec {
        id: "brightness_contrast_intensity",
        params: &[
            range("brightness", -100.0, 100.0, 0.0, 1.0),
            range("contrast", -100.0, 100.0, 0.0, 1.0),
            range("intensity", -100.0, 100.0, 0.0, 1.0),
        ],
        apply: brightness_contrast_intensity,
        reach: no_reach,
    },
    EffectSpec {
        id: "color_balance",
        params: &[
            check("shadows_on", true),
            check("midtones_on", true),
            check("highlights_on", true),
            check("preserve_luminance", true),
            range("cyan_red", -100.0, 100.0, 0.0, 1.0),
            range("magenta_green", -100.0, 100.0, 0.0, 1.0),
            range("yellow_blue", -100.0, 100.0, 0.0, 1.0),
        ],
        apply: color_balance,
        reach: no_reach,
    },
    EffectSpec {
        id: "gamma",
        params: &[range("gamma", 0.1, 10.0, 1.0, 0.01)],
        apply: gamma,
        reach: no_reach,
    },
    EffectSpec {
        id: "hue_saturation_lightness",
        params: &[
            choice("channel", &CHANNELS, 0),
            range("hue", -180.0, 180.0, 0.0, 1.0),
            range("saturation", -100.0, 100.0, 0.0, 1.0),
            range("lightness", -100.0, 100.0, 0.0, 1.0),
        ],
        apply: hsl,
        reach: no_reach,
    },
    EffectSpec {
        id: "selective_color",
        params: &[
            choice("spectrum", &SPECTRUM, 0),
            choice("method", &["relative", "absolute"], 0),
            range("cyan", -100.0, 100.0, 0.0, 1.0),
            range("magenta", -100.0, 100.0, 0.0, 1.0),
            range("yellow", -100.0, 100.0, 0.0, 1.0),
            range("black", -100.0, 100.0, 0.0, 1.0),
        ],
        apply: selective_color,
        reach: no_reach,
    },
    EffectSpec {
        id: "replace_colors",
        params: &[
            color("old_color", 0xFF0000),
            color("new_color", 0x0000FF),
            check("ignore_grayscale", true),
            check("single_destination", false),
            range("range", 1.0, 100.0, 30.0, 1.0),
            range("hue", -180.0, 180.0, 0.0, 1.0),
            range("saturation", -100.0, 100.0, 0.0, 1.0),
            range("lightness", -100.0, 100.0, 0.0, 1.0),
        ],
        apply: replace_colors,
        reach: no_reach,
    },
    EffectSpec {
        id: "desaturate",
        params: &[],
        apply: desaturate,
        reach: no_reach,
    },
    EffectSpec {
        id: "channel_mixer",
        params: &[
            check("monochrome", false),
            range("red_red", -200.0, 200.0, 100.0, 1.0),
            range("red_green", -200.0, 200.0, 0.0, 1.0),
            range("red_blue", -200.0, 200.0, 0.0, 1.0),
            range("green_red", -200.0, 200.0, 0.0, 1.0),
            range("green_green", -200.0, 200.0, 100.0, 1.0),
            range("green_blue", -200.0, 200.0, 0.0, 1.0),
            range("blue_red", -200.0, 200.0, 0.0, 1.0),
            range("blue_green", -200.0, 200.0, 0.0, 1.0),
            range("blue_blue", -200.0, 200.0, 100.0, 1.0),
        ],
        apply: channel_mixer,
        reach: no_reach,
    },
];

pub const TRANSFORM: &[EffectSpec] = &[
    EffectSpec {
        id: "deinterlace",
        params: &[
            choice("scan_lines", &["even_lines", "odd_lines"], 0),
            choice("replacement", &["duplication", "interpolation"], 1),
        ],
        apply: deinterlace,
        reach: no_reach,
    },
    EffectSpec {
        id: "invert_colors",
        params: &[],
        apply: invert,
        reach: no_reach,
    },
    EffectSpec {
        id: "posterize",
        params: &[range("level", 2.0, 32.0, 4.0, 1.0)],
        apply: posterize,
        reach: no_reach,
    },
];

pub const CORRECTION: &[EffectSpec] = &[EffectSpec {
    id: "dust_and_scratch",
    params: &[
        range("radius", 1.0, 20.0, 2.0, 1.0),
        range("threshold", 0.0, 255.0, 32.0, 1.0),
    ],
    apply: dust_and_scratch,
    reach: no_reach,
}];

pub const CHANNELS: [&str; 8] = [
    "master",
    "red",
    "yellow",
    "green",
    "cyan",
    "blue",
    "magenta",
    "grayscale",
];
/// The composite channel and the red, green and blue ones.
pub const RGB_CHANNELS: [&str; 4] = ["master", "red", "green", "blue"];
pub const SPECTRUM: [&str; 7] = [
    "reds", "yellows", "greens", "cyans", "blues", "magentas", "grays",
];

fn rgb01(p: &Rgba<u8>) -> [f32; 3] {
    [
        p[0] as f32 / 255.0,
        p[1] as f32 / 255.0,
        p[2] as f32 / 255.0,
    ]
}

fn px(c: [f32; 3], a: u8) -> Rgba<u8> {
    Rgba([
        clamp8(c[0] * 255.0),
        clamp8(c[1] * 255.0),
        clamp8(c[2] * 255.0),
        a,
    ])
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Each channel stretched between the levels that cut 0.5 % of the
/// pixels at either end.
pub fn auto_adjust(img: &RgbaImage, _: &P) -> RgbaImage {
    let mut hist = [[0u32; 256]; 3];
    let mut n = 0u32;
    for p in img.pixels().filter(|p| p[3] > 0) {
        n += 1;
        for c in 0..3 {
            hist[c][p[c] as usize] += 1;
        }
    }
    if n == 0 {
        return img.clone();
    }
    let cut = (n as f32 * 0.005) as u32;
    let mut lo = [0usize; 3];
    let mut hi = [255usize; 3];
    for c in 0..3 {
        let mut acc = 0;
        for (i, v) in hist[c].iter().enumerate() {
            acc += v;
            if acc > cut {
                lo[c] = i;
                break;
            }
        }
        let mut acc = 0;
        for i in (0..256).rev() {
            acc += hist[c][i];
            if acc > cut {
                hi[c] = i;
                break;
            }
        }
    }
    map_px(img, |_, _, p| {
        let mut o = *p;
        for c in 0..3 {
            let range = (hi[c] as f32 - lo[c] as f32).max(1.0);
            o[c] = clamp8((p[c] as f32 - lo[c] as f32) / range * 255.0);
        }
        o
    })
}

/// The Image Adjustments's corrections, in its order: black and white
/// points, temperature and tint, saturation, brightness and contrast,
/// then highlights, shadows and midtones.
pub fn image_adjustments(img: &RgbaImage, p: &P) -> RgbaImage {
    let (bp, wp) = (p.f32("black_point"), p.f32("white_point"));
    let (bp, wp) = (bp.min(wp - 1.0), wp.max(bp + 1.0));
    let temp = p.f32("temperature") / 100.0;
    let tint = p.f32("tint") / 100.0;
    let sat = p.f32("saturation") / 100.0;
    let bright = p.f32("brightness") / 100.0;
    let contrast = p.f32("contrast") / 100.0;
    let (hl, sh, mt) = (
        p.f32("highlights") / 100.0,
        p.f32("shadows") / 100.0,
        p.f32("midtones") / 100.0,
    );
    let g = 2f32.powf(-bright);
    map_px(img, |_, _, q| {
        let mut c = [0f32; 3];
        for k in 0..3 {
            c[k] = ((q[k] as f32 - bp) / (wp - bp)).clamp(0.0, 1.0);
        }
        // Higher temperature cools (more blue); tint adds green.
        c[0] *= 1.0 - 0.25 * temp;
        c[2] *= 1.0 + 0.25 * temp;
        c[1] *= 1.0 + 0.2 * tint;
        c[0] *= 1.0 - 0.1 * tint;
        c[2] *= 1.0 - 0.1 * tint;
        let l = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
        for v in c.iter_mut() {
            *v = l + (*v - l) * (1.0 + sat);
        }
        for v in c.iter_mut() {
            let x = v.clamp(0.0, 1.0).powf(g);
            *v = (x - 0.5) * (1.0 + contrast) + 0.5;
        }
        let l = (0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]).clamp(0.0, 1.0);
        let d = 0.35
            * (hl * smoothstep(0.5, 1.0, l)
                + sh * (1.0 - smoothstep(0.0, 0.5, l))
                + mt * (1.0 - (2.0 * l - 1.0).abs()));
        for v in c.iter_mut() {
            *v += d;
        }
        px(c, q[3])
    })
}

/// Levels: input range to output range through a gamma, on every
/// channel or the one chosen.
pub fn contrast_enhancement(img: &RgbaImage, p: &P) -> RgbaImage {
    let (il, ih) = (
        p.f32("input_low"),
        p.f32("input_high").max(p.f32("input_low") + 1.0),
    );
    let (ol, oh) = (p.f32("output_low"), p.f32("output_high"));
    let g = p.f32("gamma").max(0.01);
    let mut lut = [0u8; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        let t = ((i as f32 - il) / (ih - il)).clamp(0.0, 1.0).powf(1.0 / g);
        *v = clamp8(ol + (oh - ol) * t);
    }
    luts_on(img, &channel_luts(&lut, p.i("channel"), false))
}

/// The tables for the red, green and blue channels when `lut` applies to
/// the channel chosen in [`RGB_CHANNELS`] (all of them for the composite
/// or with `all`).
fn channel_luts(lut: &[u8; 256], channel: usize, all: bool) -> [[u8; 256]; 3] {
    let id: [u8; 256] = std::array::from_fn(|i| i as u8);
    std::array::from_fn(|c| {
        if channel == 0 || all || channel == c + 1 {
            *lut
        } else {
            id
        }
    })
}

fn luts_on(img: &RgbaImage, luts: &[[u8; 256]; 3]) -> RgbaImage {
    map_px(img, |_, _, q| {
        Rgba([
            luts[0][q[0] as usize],
            luts[1][q[1] as usize],
            luts[2][q[2] as usize],
            q[3],
        ])
    })
}

/// Contrast-limited equalization over tiles of the given size (square
/// when locked), blended between neighbouring tiles; colours keep their
/// hue.
pub fn local_equalization(img: &RgbaImage, p: &P) -> RgbaImage {
    let (w, h) = img.dimensions();
    let tw = p.u("width").max(2);
    // Locked, the region stays square: the width sets both.
    let th = if p.b("lock") {
        tw
    } else {
        p.u("height").max(2)
    };
    let nx = w.div_ceil(tw).max(1);
    let ny = h.div_ceil(th).max(1);
    let mut luts = vec![[0u8; 256]; (nx * ny) as usize];
    for ty in 0..ny {
        for tx in 0..nx {
            let mut hist = [0u32; 256];
            let mut n = 0u32;
            for y in ty * th..((ty + 1) * th).min(h) {
                for x in tx * tw..((tx + 1) * tw).min(w) {
                    hist[clamp8(luma(img.get_pixel(x, y))) as usize] += 1;
                    n += 1;
                }
            }
            // Clip the histogram so flat areas do not turn into noise.
            let limit = (n / 64).max(2);
            let mut excess = 0;
            for v in hist.iter_mut() {
                if *v > limit {
                    excess += *v - limit;
                    *v = limit;
                }
            }
            let add = excess / 256;
            let mut acc = 0u32;
            let lut = &mut luts[(ty * nx + tx) as usize];
            for i in 0..256 {
                acc += hist[i] + add;
                lut[i] = clamp8(acc as f32 / n.max(1) as f32 * 255.0);
            }
        }
    }
    map_px(img, |x, y, q| {
        let l = clamp8(luma(q)) as usize;
        let fx = (x as f32 + 0.5) / tw as f32 - 0.5;
        let fy = (y as f32 + 0.5) / th as f32 - 0.5;
        let (x0, y0) = (fx.floor(), fy.floor());
        let (ax, ay) = (fx - x0, fy - y0);
        let at = |i: f32, j: f32| -> f32 {
            let i = (i.max(0.0) as u32).min(nx - 1);
            let j = (j.max(0.0) as u32).min(ny - 1);
            luts[(j * nx + i) as usize][l] as f32
        };
        let v = (at(x0, y0) * (1.0 - ax) + at(x0 + 1.0, y0) * ax) * (1.0 - ay)
            + (at(x0, y0 + 1.0) * (1.0 - ax) + at(x0 + 1.0, y0 + 1.0) * ax) * ay;
        let k = v / (l as f32).max(1.0);
        if l == 0 {
            let g = clamp8(v);
            return Rgba([g, g, g, q[3]]);
        }
        Rgba([
            clamp8(q[0] as f32 * k),
            clamp8(q[1] as f32 * k),
            clamp8(q[2] as f32 * k),
            q[3],
        ])
    })
}

/// Each channel through the line from black through the low, mid and
/// high samples (to their targets) to white; with a single channel
/// chosen, only that one unless all channels are adjusted.
pub fn target_balance(img: &RgbaImage, p: &P) -> RgbaImage {
    let pairs = [
        (p.rgb("low_sample"), p.rgb("low_target")),
        (p.rgb("mid_sample"), p.rgb("mid_target")),
        (p.rgb("high_sample"), p.rgb("high_target")),
    ];
    let mut luts = [[0u8; 256]; 3];
    for (c, lut) in luts.iter_mut().enumerate() {
        let mut pts: Vec<(f32, f32)> = vec![(0.0, 0.0), (255.0, 255.0)];
        for (s, t) in &pairs {
            pts.push((s[c] as f32, t[c] as f32));
        }
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        pts.dedup_by(|a, b| (a.0 - b.0).abs() < 0.5);
        let curve = crate::bitmap_modes::ToneCurve {
            points: pts,
            smooth: false,
        };
        *lut = curve.lut();
    }
    // A single channel chosen (and not all of them): the others stay.
    let channel = p.i("channel");
    if channel > 0 && !p.b("all_channels") {
        for (c, lut) in luts.iter_mut().enumerate() {
            if c + 1 != channel {
                *lut = std::array::from_fn(|i| i as u8);
            }
        }
    }
    luts_on(img, &luts)
}

/// The Tone Curve filter's curves: `<channel>n` points, then
/// `<channel><k>x` and `<channel><k>y`, for the channels `rgb`, `r`, `g`
/// and `b`; `<channel>style` (see [`CURVE_STYLES`]), `<channel>gamma`
/// for the Gamma style, and `<channel>linear` 1 for straight segments
/// (what older settings have instead of a style).
pub fn curve_from(values: &BTreeMap<String, f64>, ch: &str) -> crate::bitmap_modes::ToneCurve {
    let n = values
        .get(&format!("{ch}n"))
        .copied()
        .unwrap_or(0.0)
        .clamp(0.0, 32.0) as usize;
    let mut points = Vec::new();
    for k in 0..n {
        let x = values
            .get(&format!("{ch}{k}x"))
            .copied()
            .unwrap_or(f64::NAN);
        let y = values
            .get(&format!("{ch}{k}y"))
            .copied()
            .unwrap_or(f64::NAN);
        if x.is_finite() && y.is_finite() {
            points.push((x as f32, y as f32));
        }
    }
    let style = curve_style(values, ch);
    if style == 3 {
        return gamma_curve(curve_gamma(values, ch));
    }
    if points.len() < 2 {
        return crate::bitmap_modes::ToneCurve::identity();
    }
    crate::bitmap_modes::ToneCurve {
        points,
        smooth: style == 0,
    }
}

/// Store a curve under `ch` (see [`curve_from`]).
pub fn curve_into(
    values: &mut BTreeMap<String, f64>,
    ch: &str,
    c: &crate::bitmap_modes::ToneCurve,
) {
    values.retain(|k, _| {
        !(k.starts_with(ch)
            && k[ch.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit() || c == 'n' || c == 'l'))
    });
    let pts = c.normalized();
    values.insert(format!("{ch}n"), pts.len() as f64);
    for (k, (x, y)) in pts.iter().enumerate() {
        values.insert(format!("{ch}{k}x"), *x as f64);
        values.insert(format!("{ch}{k}y"), *y as f64);
    }
    values.insert(format!("{ch}linear"), if c.smooth { 0.0 } else { 1.0 });
}

/// The Tone Curve's styles, as its Style list: smooth curve, straight
/// segments, freehand (drawn, kept as straight segments every 4 levels)
/// and gamma (one value).
pub const CURVE_STYLES: [&str; 4] = ["curve", "straight", "freehand", "gamma"];

/// A channel's curve style (an index into [`CURVE_STYLES`]); without
/// `<channel>style`, `<channel>linear` picks curve or straight.
pub fn curve_style(values: &BTreeMap<String, f64>, ch: &str) -> usize {
    match values
        .get(&format!("{ch}style"))
        .copied()
        .filter(|v| v.is_finite())
    {
        Some(v) => (v.round().max(0.0) as usize).min(CURVE_STYLES.len() - 1),
        None => usize::from(values.get(&format!("{ch}linear")).copied().unwrap_or(0.0) != 0.0),
    }
}

/// Set a channel's style, keeping `<channel>linear` in step.
pub fn set_curve_style(values: &mut BTreeMap<String, f64>, ch: &str, style: usize) {
    let style = style.min(CURVE_STYLES.len() - 1);
    values.insert(format!("{ch}style"), style as f64);
    values.insert(
        format!("{ch}linear"),
        if style == 1 || style == 2 { 1.0 } else { 0.0 },
    );
}

/// A channel's gamma (Gamma style), 0.1 to 10.
pub fn curve_gamma(values: &BTreeMap<String, f64>, ch: &str) -> f64 {
    values
        .get(&format!("{ch}gamma"))
        .copied()
        .filter(|v| v.is_finite())
        .unwrap_or(1.0)
        .clamp(0.1, 10.0)
}

/// The curve `255 (x / 255)^(1 / gamma)` through 33 points.
pub fn gamma_curve(gamma: f64) -> crate::bitmap_modes::ToneCurve {
    let g = gamma.clamp(0.1, 10.0) as f32;
    crate::bitmap_modes::ToneCurve {
        points: (0..=32)
            .map(|i| {
                let x = i as f32 * 255.0 / 32.0;
                (x, 255.0 * (x / 255.0).powf(1.0 / g))
            })
            .collect(),
        smooth: true,
    }
}

/// A drawn curve (one value per level) as straight segments every 4
/// levels, the last at 255.
pub fn freehand_points(lut: &[f32; 256]) -> Vec<(f32, f32)> {
    let mut pts: Vec<(f32, f32)> = (0..256)
        .step_by(4)
        .map(|x| (x as f32, lut[x].clamp(0.0, 255.0)))
        .collect();
    pts.push((255.0, lut[255].clamp(0.0, 255.0)));
    pts
}

/// A curve's values smoothed by a 9-level moving average, ends kept.
pub fn smoothed(c: &crate::bitmap_modes::ToneCurve) -> [f32; 256] {
    let v: Vec<f32> = (0..256).map(|x| c.eval(x as f32)).collect();
    let mut out = [0f32; 256];
    for (x, o) in out.iter_mut().enumerate() {
        let lo = x.saturating_sub(4);
        let hi = (x + 4).min(255);
        *o = v[lo..=hi].iter().sum::<f32>() / (hi - lo + 1) as f32;
    }
    out[0] = v[0];
    out[255] = v[255];
    out
}

/// Auto Balance Tone: per channel, the levels that cut `clip` of the
/// opaque pixels at either end, stretched to 0 and 255.
pub fn balance_curves(img: &RgbaImage, clip: f32) -> [crate::bitmap_modes::ToneCurve; 3] {
    let mut hist = [[0u32; 256]; 3];
    let mut n = 0u32;
    for p in img.pixels().filter(|p| p[3] > 0) {
        n += 1;
        for c in 0..3 {
            hist[c][p[c] as usize] += 1;
        }
    }
    let cut = (n as f32 * clip.clamp(0.0, 0.49)) as u32;
    std::array::from_fn(|c| {
        if n == 0 {
            return crate::bitmap_modes::ToneCurve::identity();
        }
        let mut acc = 0;
        let lo = (0..256)
            .find(|&i| {
                acc += hist[c][i];
                acc > cut
            })
            .unwrap_or(0);
        acc = 0;
        let hi = (0..256)
            .rev()
            .find(|&i| {
                acc += hist[c][i];
                acc > cut
            })
            .unwrap_or(255);
        if hi <= lo {
            return crate::bitmap_modes::ToneCurve::identity();
        }
        let mut points = vec![(lo as f32, 0.0), (hi as f32, 255.0)];
        if lo > 0 {
            points.insert(0, (0.0, 0.0));
        }
        if hi < 255 {
            points.push((255.0, 255.0));
        }
        crate::bitmap_modes::ToneCurve {
            points,
            smooth: false,
        }
    })
}

pub fn tone_curve(img: &RgbaImage, p: &P) -> RgbaImage {
    let all = curve_from(p.values, "rgb").lut();
    let ch = ["r", "g", "b"].map(|c| curve_from(p.values, c).lut());
    map_px(img, |_, _, q| {
        Rgba([
            ch[0][all[q[0] as usize] as usize],
            ch[1][all[q[1] as usize] as usize],
            ch[2][all[q[2] as usize] as usize],
            q[3],
        ])
    })
}

pub fn brightness_contrast_intensity(img: &RgbaImage, p: &P) -> RgbaImage {
    let b = p.f32("brightness") / 100.0;
    let c = p.f32("contrast") / 100.0;
    let i = p.f32("intensity") / 100.0;
    let mut lut = [0u8; 256];
    for (k, v) in lut.iter_mut().enumerate() {
        let mut x = k as f32 / 255.0 + b * 0.5;
        x = (x - 0.5) * (1.0 + c) + 0.5;
        x *= 1.0 + i * 0.5;
        *v = clamp8(x * 255.0);
    }
    map_lut(img, &lut)
}

pub fn color_balance(img: &RgbaImage, p: &P) -> RgbaImage {
    let d = [
        p.f32("cyan_red") / 100.0,
        p.f32("magenta_green") / 100.0,
        p.f32("yellow_blue") / 100.0,
    ];
    let (s_on, m_on, h_on) = (p.b("shadows_on"), p.b("midtones_on"), p.b("highlights_on"));
    let keep = p.b("preserve_luminance");
    map_px(img, |_, _, q| {
        let mut c = rgb01(q);
        let l0 = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
        let mut w = 0.0;
        if s_on {
            w += 1.0 - smoothstep(0.0, 0.5, l0);
        }
        if m_on {
            w += 1.0 - (2.0 * l0 - 1.0).abs();
        }
        if h_on {
            w += smoothstep(0.5, 1.0, l0);
        }
        for k in 0..3 {
            c[k] += d[k] * 0.3 * w;
        }
        if keep {
            let l1 = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
            for v in c.iter_mut() {
                *v += l0 - l1;
            }
        }
        px(c, q[3])
    })
}

pub fn gamma(img: &RgbaImage, p: &P) -> RgbaImage {
    let g = p.f32("gamma").max(0.01);
    let mut lut = [0u8; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        *v = clamp8((i as f32 / 255.0).powf(1.0 / g) * 255.0);
    }
    map_lut(img, &lut)
}

/// How much a colour belongs to a hue channel (red 0, yellow 60, ... ).
fn hue_weight(h: f32, s: f32, centre: f32) -> f32 {
    let d = ((h - centre + 540.0).rem_euclid(360.0) - 180.0).abs();
    let w = if d <= 30.0 {
        1.0
    } else if d >= 60.0 {
        0.0
    } else {
        (60.0 - d) / 30.0
    };
    w * (s * 4.0).min(1.0)
}

pub fn hsl(img: &RgbaImage, p: &P) -> RgbaImage {
    let ch = p.i("channel");
    let (dh, ds, dl) = (
        p.f32("hue"),
        p.f32("saturation") / 100.0,
        p.f32("lightness") / 100.0,
    );
    map_px(img, |_, _, q| {
        let c = rgb01(q);
        let (h, s, l) = rgb_to_hsl(c[0], c[1], c[2]);
        let w = match ch {
            0 => 1.0,
            7 => (1.0 - s * 4.0).max(0.0),
            k => hue_weight(h, s, (k as f32 - 1.0) * 60.0),
        };
        if w <= 0.0 {
            return *q;
        }
        let h2 = h + dh * w;
        let s2 = (s * (1.0 + ds * w)).clamp(0.0, 1.0);
        let l2 = if dl >= 0.0 {
            l + (1.0 - l) * dl * w
        } else {
            l * (1.0 + dl * w)
        };
        let (r, g, b) = hsl_to_rgb(h2, s2, l2.clamp(0.0, 1.0));
        px([r, g, b], q[3])
    })
}

pub fn selective_color(img: &RgbaImage, p: &P) -> RgbaImage {
    let sp = p.i("spectrum");
    let absolute = p.i("method") == 1;
    let d = [
        p.f32("cyan") / 100.0,
        p.f32("magenta") / 100.0,
        p.f32("yellow") / 100.0,
        p.f32("black") / 100.0,
    ];
    map_px(img, |_, _, q| {
        let c = rgb01(q);
        let (h, s, _) = rgb_to_hsl(c[0], c[1], c[2]);
        let w = if sp == 6 {
            (1.0 - s * 4.0).max(0.0)
        } else {
            hue_weight(h, s, sp as f32 * 60.0)
        };
        if w <= 0.0 {
            return *q;
        }
        let k = 1.0 - c[0].max(c[1]).max(c[2]);
        let den = (1.0 - k).max(1e-6);
        let mut cmyk = [
            (1.0 - c[0] - k) / den,
            (1.0 - c[1] - k) / den,
            (1.0 - c[2] - k) / den,
            k,
        ];
        for i in 0..4 {
            let delta = if absolute { d[i] } else { d[i] * cmyk[i] };
            cmyk[i] = (cmyk[i] + delta * w).clamp(0.0, 1.0);
        }
        let k = cmyk[3];
        px(
            [
                (1.0 - cmyk[0]) * (1.0 - k),
                (1.0 - cmyk[1]) * (1.0 - k),
                (1.0 - cmyk[2]) * (1.0 - k),
            ],
            q[3],
        )
    })
}

pub fn replace_colors(img: &RgbaImage, p: &P) -> RgbaImage {
    let old = p.rgb("old_color").map(|v| v as f32 / 255.0);
    let new = p.rgb("new_color").map(|v| v as f32 / 255.0);
    let (oh, os, ol) = rgb_to_hsl(old[0], old[1], old[2]);
    let (nh, ns, nl) = rgb_to_hsl(new[0], new[1], new[2]);
    let nh = nh + p.f32("hue");
    let ns = (ns + p.f32("saturation") / 100.0).clamp(0.0, 1.0);
    let nl = (nl + p.f32("lightness") / 100.0).clamp(0.0, 1.0);
    let range = p.f32("range") / 100.0;
    let ignore_gray = p.b("ignore_grayscale");
    let single = p.b("single_destination");
    map_px(img, |_, _, q| {
        let c = rgb01(q);
        let (h, s, l) = rgb_to_hsl(c[0], c[1], c[2]);
        if ignore_gray && s < 0.05 {
            return *q;
        }
        let dh = ((h - oh + 540.0).rem_euclid(360.0) - 180.0).abs() / 180.0;
        let dist = if ignore_gray {
            dh
        } else {
            (dh * dh + (s - os) * (s - os) + (l - ol) * (l - ol)).sqrt()
        };
        let w = (1.0 - dist / range.max(0.01)).clamp(0.0, 1.0);
        if w <= 0.0 {
            return *q;
        }
        let (h2, s2, l2) = if single {
            (nh, ns, nl)
        } else {
            (
                h + ((nh - oh + 540.0).rem_euclid(360.0) - 180.0),
                (s + ns - os).clamp(0.0, 1.0),
                (l + nl - ol).clamp(0.0, 1.0),
            )
        };
        let (r, g, b) = hsl_to_rgb(h2, s2, l2);
        px(
            [
                c[0] + (r - c[0]) * w,
                c[1] + (g - c[1]) * w,
                c[2] + (b - c[2]) * w,
            ],
            q[3],
        )
    })
}

pub fn desaturate(img: &RgbaImage, _: &P) -> RgbaImage {
    map_px(img, |_, _, q| {
        let l = clamp8(luma(q));
        Rgba([l, l, l, q[3]])
    })
}

pub fn channel_mixer(img: &RgbaImage, p: &P) -> RgbaImage {
    let row = |o: &str| {
        [
            p.f32(&format!("{o}_red")) / 100.0,
            p.f32(&format!("{o}_green")) / 100.0,
            p.f32(&format!("{o}_blue")) / 100.0,
        ]
    };
    let m = [row("red"), row("green"), row("blue")];
    let mono = p.b("monochrome");
    map_px(img, |_, _, q| {
        let c = rgb01(q);
        let mut o = [0f32; 3];
        for (k, r) in m.iter().enumerate() {
            o[k] = r[0] * c[0] + r[1] * c[1] + r[2] * c[2];
        }
        if mono {
            o = [o[0]; 3];
        }
        px(o, q[3])
    })
}

pub fn deinterlace(img: &RgbaImage, p: &P) -> RgbaImage {
    let even = p.i("scan_lines") == 0;
    let interpolate = p.i("replacement") == 1;
    let h = img.height();
    map_px(img, |x, y, q| {
        let replace = (y % 2 == 0) == even;
        if !replace {
            return *q;
        }
        let above = if y > 0 {
            Some(img.get_pixel(x, y - 1))
        } else {
            None
        };
        let below = if y + 1 < h {
            Some(img.get_pixel(x, y + 1))
        } else {
            None
        };
        match (above, below) {
            (Some(a), Some(b)) if interpolate => Rgba([
                ((a[0] as u16 + b[0] as u16) / 2) as u8,
                ((a[1] as u16 + b[1] as u16) / 2) as u8,
                ((a[2] as u16 + b[2] as u16) / 2) as u8,
                q[3],
            ]),
            (Some(a), _) => Rgba([a[0], a[1], a[2], q[3]]),
            (None, Some(b)) => Rgba([b[0], b[1], b[2], q[3]]),
            _ => *q,
        }
    })
}

pub fn invert(img: &RgbaImage, _: &P) -> RgbaImage {
    map_px(img, |_, _, q| {
        Rgba([255 - q[0], 255 - q[1], 255 - q[2], q[3]])
    })
}

pub fn posterize(img: &RgbaImage, p: &P) -> RgbaImage {
    let n = p.f32("level").max(2.0);
    let mut lut = [0u8; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        let step = (i as f32 / 255.0 * (n - 1.0)).round() / (n - 1.0);
        *v = clamp8(step * 255.0);
    }
    map_lut(img, &lut)
}

/// The median where a pixel stands out from it by more than the
/// threshold; elsewhere the pixel stays.
pub fn dust_and_scratch(img: &RgbaImage, p: &P) -> RgbaImage {
    let med = rank(img, p.u("radius").max(1), 0.5);
    let t = p.f32("threshold");
    map_px(img, |x, y, q| {
        let m = med.get_pixel(x, y);
        let diff = (0..3)
            .map(|c| (q[c] as f32 - m[c] as f32).abs())
            .fold(0.0, f32::max);
        if diff > t {
            Rgba([m[0], m[1], m[2], q[3]])
        } else {
            *q
        }
    })
}
