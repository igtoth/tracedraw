//! Bitmap colour mode conversions (Bitmaps > Mode): black and white with seven conversion methods,
//! duotone (one to four inks, each with a tone curve), paletted (uniform,
//! standard VGA, adaptive, optimized, grayscale, system or custom
//! palettes, with ordered or error-diffusion dithering), and Lab. Pixels
//! stay RGBA8; alpha is kept.

use image::{ImageBuffer, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

fn luma(p: &Rgba<u8>) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

fn clamp8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

// ----- tone curves -----------------------------------------------------------

/// Convert to Bitmap's colour modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConvertMode {
    BlackWhite,
    Grayscale,
    Paletted,
    #[default]
    Rgb,
    Cmyk,
}

impl ConvertMode {
    pub const ALL: [ConvertMode; 5] = [
        ConvertMode::BlackWhite,
        ConvertMode::Grayscale,
        ConvertMode::Paletted,
        ConvertMode::Rgb,
        ConvertMode::Cmyk,
    ];

    /// The mode's name in the list.
    pub fn key(self) -> &'static str {
        match self {
            ConvertMode::BlackWhite => "dialog.convert_mode_bw",
            ConvertMode::Grayscale => "dialog.convert_mode_gray",
            ConvertMode::Paletted => "dialog.convert_mode_paletted",
            ConvertMode::Rgb => "dialog.convert_mode_rgb",
            ConvertMode::Cmyk => "dialog.convert_mode_cmyk",
        }
    }

    /// Bits per pixel, for the uncompressed size.
    pub fn bits(self) -> u64 {
        match self {
            ConvertMode::BlackWhite => 1,
            ConvertMode::Grayscale | ConvertMode::Paletted => 8,
            ConvertMode::Rgb => 24,
            ConvertMode::Cmyk => 32,
        }
    }

    /// Dithering applies to modes of 256 colours or fewer.
    pub fn can_dither(self) -> bool {
        matches!(self, ConvertMode::BlackWhite | ConvertMode::Paletted)
    }
}

/// Bitmaps > Convert to Bitmap's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConvertOptions {
    pub dpi: f64,
    pub mode: ConvertMode,
    pub dithered: bool,
    /// Always overprint black: the bitmap is marked to overprint, so its
    /// black prints over what lies beneath.
    pub overprint_black: bool,
    pub anti_alias: bool,
    pub transparent: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        ConvertOptions {
            dpi: 300.0,
            mode: ConvertMode::Rgb,
            dithered: false,
            overprint_black: false,
            anti_alias: true,
            transparent: true,
        }
    }
}

impl ConvertOptions {
    /// The uncompressed size of a `w` by `h` result, bytes.
    pub fn bytes(&self, w: u64, h: u64) -> u64 {
        (w * self.mode.bits()).div_ceil(8) * h
    }
}

/// The rendered pixels in the chosen colour mode: black and white by a
/// 50 % threshold or Floyd-Steinberg when dithered; grayscale by
/// luminance; paletted to an optimized 256-colour palette, dithered or
/// not; RGB and CMYK as they are. Alpha is kept.
pub fn convert_pixels(img: &RgbaImage, o: &ConvertOptions) -> RgbaImage {
    match o.mode {
        ConvertMode::BlackWhite => black_and_white(
            img,
            &BwSettings {
                method: if o.dithered {
                    BwMethod::FloydSteinberg
                } else {
                    BwMethod::LineArt
                },
                threshold: 128,
                intensity: 50.0,
                ..Default::default()
            },
            o.dpi as f32,
        ),
        ConvertMode::Grayscale => {
            crate::bitmap_fx::convert_mode(img, crate::bitmap_fx::ColorMode::Grayscale)
        }
        ConvertMode::Paletted => paletted(
            img,
            &PalettedSettings {
                palette: PaletteType::Optimized,
                colors: 256,
                dither: if o.dithered {
                    Dither::FloydSteinberg
                } else {
                    Dither::None
                },
                ..Default::default()
            },
        ),
        ConvertMode::Rgb | ConvertMode::Cmyk => img.clone(),
    }
}

/// A tone curve through points (x, y), both 0 to 255, sorted by x; the
/// first and last points are the ends. Smooth curves pass through the
/// points with a monotone cubic; others join them with straight lines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToneCurve {
    pub points: Vec<(f32, f32)>,
    pub smooth: bool,
}

impl Default for ToneCurve {
    fn default() -> Self {
        ToneCurve::identity()
    }
}

impl ToneCurve {
    pub fn identity() -> Self {
        ToneCurve {
            points: vec![(0.0, 0.0), (255.0, 255.0)],
            smooth: true,
        }
    }

    /// A straight curve from (0, `a`) to (255, `b`).
    pub fn line(a: f32, b: f32) -> Self {
        ToneCurve {
            points: vec![(0.0, a), (255.0, b)],
            smooth: true,
        }
    }

    /// Points sorted, inside the box, with distinct x, ends present.
    pub fn normalized(&self) -> Vec<(f32, f32)> {
        let mut pts: Vec<(f32, f32)> = self
            .points
            .iter()
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .map(|(x, y)| (x.clamp(0.0, 255.0), y.clamp(0.0, 255.0)))
            .collect();
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-3);
        if pts.is_empty() {
            return vec![(0.0, 0.0), (255.0, 255.0)];
        }
        if pts.len() == 1 {
            let y = pts[0].1;
            return vec![(0.0, y), (255.0, y)];
        }
        pts
    }

    /// The curve's value at `x` (0 to 255).
    pub fn eval(&self, x: f32) -> f32 {
        let pts = self.normalized();
        let n = pts.len();
        if x <= pts[0].0 {
            return pts[0].1;
        }
        if x >= pts[n - 1].0 {
            return pts[n - 1].1;
        }
        let k = pts
            .partition_point(|p| p.0 <= x)
            .saturating_sub(1)
            .min(n - 2);
        let (x0, y0) = pts[k];
        let (x1, y1) = pts[k + 1];
        let h = x1 - x0;
        let t = (x - x0) / h;
        if !self.smooth || n == 2 {
            return y0 + (y1 - y0) * t;
        }
        // Fritsch-Carlson tangents keep the curve monotone where the
        // points are.
        let slope = |i: usize| (pts[i + 1].1 - pts[i].1) / (pts[i + 1].0 - pts[i].0);
        let tangent = |i: usize| -> f32 {
            if i == 0 {
                slope(0)
            } else if i == n - 1 {
                slope(n - 2)
            } else {
                let (a, b) = (slope(i - 1), slope(i));
                if a * b <= 0.0 {
                    0.0
                } else {
                    let w1 = 2.0 * (pts[i + 1].0 - pts[i].0) + (pts[i].0 - pts[i - 1].0);
                    let w2 = (pts[i + 1].0 - pts[i].0) + 2.0 * (pts[i].0 - pts[i - 1].0);
                    (w1 + w2) / (w1 / a + w2 / b)
                }
            }
        };
        let (m0, m1) = (tangent(k), tangent(k + 1));
        let t2 = t * t;
        let t3 = t2 * t;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        (h00 * y0 + h10 * h * m0 + h01 * y1 + h11 * h * m1).clamp(0.0, 255.0)
    }

    /// A lookup table of the curve.
    pub fn lut(&self) -> [u8; 256] {
        let mut out = [0u8; 256];
        for (i, v) in out.iter_mut().enumerate() {
            *v = clamp8(self.eval(i as f32));
        }
        out
    }
}

// ----- black and white -------------------------------------------------------

/// Bitmaps > Mode > Black and White (1-bit): the conversion method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BwMethod {
    LineArt,
    Ordered,
    #[default]
    Halftone,
    CardinalityDistribution,
    Jarvis,
    Stucki,
    FloydSteinberg,
}

impl BwMethod {
    pub const ALL: [BwMethod; 7] = [
        BwMethod::LineArt,
        BwMethod::Ordered,
        BwMethod::Halftone,
        BwMethod::CardinalityDistribution,
        BwMethod::Jarvis,
        BwMethod::Stucki,
        BwMethod::FloydSteinberg,
    ];

    pub fn key(self) -> &'static str {
        match self {
            BwMethod::LineArt => "bmode.line_art",
            BwMethod::Ordered => "bmode.ordered",
            BwMethod::Halftone => "bmode.halftone",
            BwMethod::CardinalityDistribution => "bmode.cardinality",
            BwMethod::Jarvis => "bmode.jarvis",
            BwMethod::Stucki => "bmode.stucki",
            BwMethod::FloydSteinberg => "bmode.floyd_steinberg",
        }
    }
}

/// The halftone screen's dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Screen {
    #[default]
    Round,
    Line,
    Square,
}

/// Black and white settings: `threshold` (0 to 255) for line art,
/// `intensity` (0 to 100, 50 neutral; higher is lighter) for the others,
/// and the halftone screen (angle in degrees, `lpi` lines per inch at the
/// bitmap's `dpi`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BwSettings {
    pub method: BwMethod,
    pub threshold: u8,
    pub intensity: f32,
    pub screen: Screen,
    pub angle: f32,
    pub lpi: f32,
}

impl Default for BwSettings {
    fn default() -> Self {
        BwSettings {
            method: BwMethod::Halftone,
            threshold: 128,
            intensity: 50.0,
            screen: Screen::Round,
            angle: 45.0,
            lpi: 60.0,
        }
    }
}

const BAYER8: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// Error-diffusion kernels: (dx, dy, weight) and the divisor.
fn kernel(method: BwMethod) -> (&'static [(i32, i32, f32)], f32) {
    const FS: [(i32, i32, f32); 4] = [(1, 0, 7.0), (-1, 1, 3.0), (0, 1, 5.0), (1, 1, 1.0)];
    const JARVIS: [(i32, i32, f32); 12] = [
        (1, 0, 7.0),
        (2, 0, 5.0),
        (-2, 1, 3.0),
        (-1, 1, 5.0),
        (0, 1, 7.0),
        (1, 1, 5.0),
        (2, 1, 3.0),
        (-2, 2, 1.0),
        (-1, 2, 3.0),
        (0, 2, 5.0),
        (1, 2, 3.0),
        (2, 2, 1.0),
    ];
    const STUCKI: [(i32, i32, f32); 12] = [
        (1, 0, 8.0),
        (2, 0, 4.0),
        (-2, 1, 2.0),
        (-1, 1, 4.0),
        (0, 1, 8.0),
        (1, 1, 4.0),
        (2, 1, 2.0),
        (-2, 2, 1.0),
        (-1, 2, 2.0),
        (0, 2, 4.0),
        (1, 2, 2.0),
        (2, 2, 1.0),
    ];
    match method {
        BwMethod::Jarvis => (&JARVIS, 48.0),
        BwMethod::Stucki => (&STUCKI, 42.0),
        _ => (&FS, 16.0),
    }
}

/// A stable pseudo-random value 0..1 per pixel.
fn hash01(x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Convert to black and white; `dpi` is the bitmap's resolution (for the
/// halftone screen).
pub fn black_and_white(img: &RgbaImage, s: &BwSettings, dpi: f32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let bias = (s.intensity.clamp(0.0, 100.0) - 50.0) * 2.55;
    let gray: Vec<f32> = img
        .pixels()
        .map(|p| {
            (luma(p)
                + if s.method == BwMethod::LineArt {
                    0.0
                } else {
                    bias
                })
            .clamp(0.0, 255.0)
        })
        .collect();
    let at = |x: u32, y: u32| (y * w + x) as usize;
    let mut bits = vec![false; (w * h) as usize];
    match s.method {
        BwMethod::LineArt => {
            for (i, g) in gray.iter().enumerate() {
                bits[i] = *g >= s.threshold as f32;
            }
        }
        BwMethod::Ordered => {
            for y in 0..h {
                for x in 0..w {
                    let t =
                        (BAYER8[(y % 8) as usize][(x % 8) as usize] as f32 + 0.5) / 64.0 * 255.0;
                    bits[at(x, y)] = gray[at(x, y)] > t;
                }
            }
        }
        BwMethod::CardinalityDistribution => {
            for y in 0..h {
                for x in 0..w {
                    let t = hash01(x, y) * 255.0;
                    bits[at(x, y)] = gray[at(x, y)] > t;
                }
            }
        }
        BwMethod::Halftone => {
            let period = (dpi.max(1.0) / s.lpi.clamp(1.0, 2000.0)).max(2.0);
            let (sa, ca) = s.angle.to_radians().sin_cos();
            for y in 0..h {
                for x in 0..w {
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let u = (fx * ca + fy * sa) / period * std::f32::consts::TAU;
                    let v = (-fx * sa + fy * ca) / period * std::f32::consts::TAU;
                    // The screen's threshold field: dots grow as the tone
                    // darkens.
                    let t = match s.screen {
                        Screen::Round => (u.cos() + v.cos()) / 4.0 + 0.5,
                        Screen::Line => (v.cos() + 1.0) / 2.0,
                        Screen::Square => {
                            let a = (u / std::f32::consts::TAU).rem_euclid(1.0);
                            let b = (v / std::f32::consts::TAU).rem_euclid(1.0);
                            1.0 - ((a - 0.5).abs().max((b - 0.5).abs()) * 2.0)
                        }
                    };
                    bits[at(x, y)] = gray[at(x, y)] / 255.0 > 1.0 - t;
                }
            }
        }
        BwMethod::Jarvis | BwMethod::Stucki | BwMethod::FloydSteinberg => {
            let (k, div) = kernel(s.method);
            let mut g = gray.clone();
            for y in 0..h {
                for x in 0..w {
                    let i = at(x, y);
                    let old = g[i];
                    let on = old >= 128.0;
                    bits[i] = on;
                    let err = old - if on { 255.0 } else { 0.0 };
                    for (dx, dy, wt) in k {
                        let (nx, ny) = (x as i64 + *dx as i64, y as i64 + *dy as i64);
                        if nx >= 0 && ny >= 0 && (nx as u32) < w && (ny as u32) < h {
                            g[at(nx as u32, ny as u32)] += err * wt / div;
                        }
                    }
                }
            }
        }
    }
    ImageBuffer::from_fn(w, h, |x, y| {
        let v = if bits[at(x, y)] { 255 } else { 0 };
        Rgba([v, v, v, img.get_pixel(x, y)[3]])
    })
}

// ----- duotone ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DuotoneType {
    Monotone,
    #[default]
    Duotone,
    Tritone,
    Quadtone,
}

impl DuotoneType {
    pub const ALL: [DuotoneType; 4] = [
        DuotoneType::Monotone,
        DuotoneType::Duotone,
        DuotoneType::Tritone,
        DuotoneType::Quadtone,
    ];

    pub fn inks(self) -> usize {
        match self {
            DuotoneType::Monotone => 1,
            DuotoneType::Duotone => 2,
            DuotoneType::Tritone => 3,
            DuotoneType::Quadtone => 4,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            DuotoneType::Monotone => "bmode.monotone",
            DuotoneType::Duotone => "bmode.duotone",
            DuotoneType::Tritone => "bmode.tritone",
            DuotoneType::Quadtone => "bmode.quadtone",
        }
    }
}

/// One duotone ink: its colour and how much of it (curve y, 0 to 255 for
/// 0 to 100 %) goes on each gray level (curve x, 0 black to 255 white).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ink {
    pub rgb: [u8; 3],
    pub curve: ToneCurve,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuotoneSettings {
    pub kind: DuotoneType,
    /// Four inks; the type uses the first ones.
    pub inks: Vec<Ink>,
}

impl Default for DuotoneSettings {
    fn default() -> Self {
        DuotoneSettings {
            kind: DuotoneType::Duotone,
            inks: vec![
                Ink {
                    rgb: [0, 0, 0],
                    curve: ToneCurve::line(255.0, 0.0),
                },
                Ink {
                    rgb: [247, 148, 29],
                    curve: ToneCurve {
                        points: vec![(0.0, 191.0), (128.0, 128.0), (255.0, 0.0)],
                        smooth: true,
                    },
                },
                Ink {
                    rgb: [0, 158, 219],
                    curve: ToneCurve::line(140.0, 0.0),
                },
                Ink {
                    rgb: [227, 0, 122],
                    curve: ToneCurve::line(102.0, 0.0),
                },
            ],
        }
    }
}

/// The colour a duotone gives a gray level: the inks multiply as they
/// would on paper.
pub fn duotone_color(s: &DuotoneSettings, luts: &[[u8; 256]], gray: u8) -> [u8; 3] {
    let mut c = [1.0f32; 3];
    for (ink, lut) in s.inks.iter().zip(luts).take(s.kind.inks()) {
        let cover = lut[gray as usize] as f32 / 255.0;
        for (v, ink_v) in c.iter_mut().zip(ink.rgb) {
            *v *= 1.0 - cover * (1.0 - ink_v as f32 / 255.0);
        }
    }
    [
        clamp8(c[0] * 255.0),
        clamp8(c[1] * 255.0),
        clamp8(c[2] * 255.0),
    ]
}

pub fn duotone(img: &RgbaImage, s: &DuotoneSettings) -> RgbaImage {
    let luts: Vec<[u8; 256]> = s.inks.iter().map(|i| i.curve.lut()).collect();
    let mut table = [[0u8; 3]; 256];
    for (g, t) in table.iter_mut().enumerate() {
        *t = duotone_color(s, &luts, g as u8);
    }
    let mut out = img.clone();
    for p in out.pixels_mut() {
        let g = clamp8(luma(p));
        let [r, gg, b] = table[g as usize];
        *p = Rgba([r, gg, b, p[3]]);
    }
    out
}

// ----- paletted --------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PaletteType {
    Uniform,
    StandardVga,
    Adaptive,
    #[default]
    Optimized,
    Grayscale,
    System,
    /// The document palette.
    Custom,
}

impl PaletteType {
    pub const ALL: [PaletteType; 7] = [
        PaletteType::Uniform,
        PaletteType::StandardVga,
        PaletteType::Adaptive,
        PaletteType::Optimized,
        PaletteType::Grayscale,
        PaletteType::System,
        PaletteType::Custom,
    ];

    pub fn key(self) -> &'static str {
        match self {
            PaletteType::Uniform => "bmode.palette_uniform",
            PaletteType::StandardVga => "bmode.palette_vga",
            PaletteType::Adaptive => "bmode.palette_adaptive",
            PaletteType::Optimized => "bmode.palette_optimized",
            PaletteType::Grayscale => "bmode.palette_grayscale",
            PaletteType::System => "bmode.palette_system",
            PaletteType::Custom => "bmode.palette_custom",
        }
    }

    /// The type takes a colour count.
    pub fn counts(self) -> bool {
        matches!(self, PaletteType::Adaptive | PaletteType::Optimized)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Dither {
    None,
    Ordered,
    Jarvis,
    Stucki,
    #[default]
    FloydSteinberg,
}

impl Dither {
    pub const ALL: [Dither; 5] = [
        Dither::None,
        Dither::Ordered,
        Dither::Jarvis,
        Dither::Stucki,
        Dither::FloydSteinberg,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Dither::None => "bmode.dither_none",
            Dither::Ordered => "bmode.ordered",
            Dither::Jarvis => "bmode.jarvis",
            Dither::Stucki => "bmode.stucki",
            Dither::FloydSteinberg => "bmode.floyd_steinberg",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PalettedSettings {
    pub palette: PaletteType,
    /// Colours of an adaptive or optimized palette (2 to 256).
    pub colors: u32,
    pub dither: Dither,
    /// 0 to 100.
    pub intensity: f32,
    /// The custom palette's colours.
    pub custom: Vec<[u8; 3]>,
}

impl Default for PalettedSettings {
    fn default() -> Self {
        PalettedSettings {
            palette: PaletteType::Optimized,
            colors: 256,
            dither: Dither::FloydSteinberg,
            intensity: 100.0,
            custom: Vec::new(),
        }
    }
}

/// The 16 colours of the standard VGA palette.
pub const VGA16: [[u8; 3]; 16] = [
    [0, 0, 0],
    [128, 0, 0],
    [0, 128, 0],
    [128, 128, 0],
    [0, 0, 128],
    [128, 0, 128],
    [0, 128, 128],
    [192, 192, 192],
    [128, 128, 128],
    [255, 0, 0],
    [0, 255, 0],
    [255, 255, 0],
    [0, 0, 255],
    [255, 0, 255],
    [0, 255, 255],
    [255, 255, 255],
];

/// Median cut: split the box of colours with the widest channel at its
/// median, until there are `n` boxes; each box gives its mean colour.
/// `weighted` splits the most populous boxes first (Optimized) instead
/// of the largest (Adaptive).
fn median_cut(colors: &[[u8; 3]], n: usize, weighted: bool) -> Vec<[u8; 3]> {
    struct Box3 {
        colors: Vec<[u8; 3]>,
        channel: usize,
        range: u8,
    }
    let make = |colors: Vec<[u8; 3]>| -> Box3 {
        let mut lo = [255u8; 3];
        let mut hi = [0u8; 3];
        for c in &colors {
            for k in 0..3 {
                lo[k] = lo[k].min(c[k]);
                hi[k] = hi[k].max(c[k]);
            }
        }
        let (mut channel, mut range) = (0usize, 0u8);
        for k in 0..3 {
            let r = hi[k].saturating_sub(lo[k]);
            if r > range {
                channel = k;
                range = r;
            }
        }
        Box3 {
            colors,
            channel,
            range,
        }
    };
    if colors.is_empty() {
        return vec![[0, 0, 0]];
    }
    let mut boxes: Vec<Box3> = vec![make(colors.to_vec())];
    while boxes.len() < n {
        let pick = boxes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.colors.len() > 1 && b.range > 0)
            .max_by_key(|(_, b)| {
                let r = b.range as usize;
                if weighted {
                    b.colors.len() * (r + 1)
                } else {
                    r * 1_000_000 + b.colors.len()
                }
            })
            .map(|(i, _)| i);
        let Some(i) = pick else {
            break;
        };
        let mut b = boxes.swap_remove(i);
        let k = b.channel;
        b.colors.sort_unstable_by_key(|c| c[k]);
        let half = b.colors.split_off(b.colors.len() / 2);
        boxes.push(make(b.colors));
        boxes.push(make(half));
    }
    boxes
        .iter()
        .filter(|b| !b.colors.is_empty())
        .map(|b| {
            let mut s = [0u64; 3];
            for c in &b.colors {
                for k in 0..3 {
                    s[k] += c[k] as u64;
                }
            }
            let n = b.colors.len() as u64;
            [(s[0] / n) as u8, (s[1] / n) as u8, (s[2] / n) as u8]
        })
        .collect()
}

/// The palette a conversion uses for `img`.
pub fn build_palette(img: &RgbaImage, s: &PalettedSettings) -> Vec<[u8; 3]> {
    match s.palette {
        PaletteType::Uniform => {
            // 6 x 6 x 6 colours and 40 grays between them.
            let mut p = Vec::with_capacity(256);
            for r in 0..6u32 {
                for g in 0..6u32 {
                    for b in 0..6u32 {
                        p.push([(r * 51) as u8, (g * 51) as u8, (b * 51) as u8]);
                    }
                }
            }
            for i in 1..=40u32 {
                let v = (i * 255 / 41) as u8;
                p.push([v, v, v]);
            }
            p
        }
        PaletteType::StandardVga => VGA16.to_vec(),
        PaletteType::Grayscale => (0..=255u8).map(|v| [v, v, v]).collect(),
        PaletteType::System => {
            let mut p: Vec<[u8; 3]> = VGA16.to_vec();
            for r in 0..6u32 {
                for g in 0..6u32 {
                    for b in 0..6u32 {
                        let c = [(r * 51) as u8, (g * 51) as u8, (b * 51) as u8];
                        if !p.contains(&c) {
                            p.push(c);
                        }
                    }
                }
            }
            p.truncate(256);
            p
        }
        PaletteType::Custom => {
            if s.custom.is_empty() {
                VGA16.to_vec()
            } else {
                s.custom.iter().take(256).copied().collect()
            }
        }
        PaletteType::Adaptive | PaletteType::Optimized => {
            let n = s.colors.clamp(2, 256) as usize;
            // Sample the image (at most about 64k pixels).
            let total = (img.width() as usize * img.height() as usize).max(1);
            let step = (total / 16_384).max(1);
            let colors: Vec<[u8; 3]> = img
                .pixels()
                .step_by(step)
                .filter(|p| p[3] > 0)
                .map(|p| [p[0], p[1], p[2]])
                .collect();
            let mut unique = colors.clone();
            unique.sort_unstable();
            unique.dedup();
            if unique.len() <= n {
                return if unique.is_empty() {
                    vec![[0, 0, 0]]
                } else {
                    unique
                };
            }
            median_cut(&colors, n, s.palette == PaletteType::Optimized)
        }
    }
}

/// Nearest palette entry, through a 32 x 32 x 32 cache.
struct Nearest<'a> {
    palette: &'a [[u8; 3]],
    cache: Vec<u16>,
}

impl<'a> Nearest<'a> {
    fn new(palette: &'a [[u8; 3]]) -> Self {
        Nearest {
            palette,
            cache: vec![u16::MAX; 32 * 32 * 32],
        }
    }

    fn find(&mut self, c: [f32; 3]) -> usize {
        let q = |v: f32| (v.clamp(0.0, 255.0) as usize) >> 3;
        let key = (q(c[0]) << 10) | (q(c[1]) << 5) | q(c[2]);
        if self.cache[key] != u16::MAX {
            return self.cache[key] as usize;
        }
        let mut best = (0usize, f32::MAX);
        for (i, p) in self.palette.iter().enumerate() {
            let d = (0..3)
                .map(|k| {
                    let e = c[k] - p[k] as f32;
                    e * e
                })
                .sum::<f32>();
            if d < best.1 {
                best = (i, d);
            }
        }
        self.cache[key] = best.0 as u16;
        best.0
    }
}

/// Convert to the paletted mode: every pixel takes a palette colour.
pub fn paletted(img: &RgbaImage, s: &PalettedSettings) -> RgbaImage {
    let palette = build_palette(img, s);
    let (w, h) = img.dimensions();
    let mut nearest = Nearest::new(&palette);
    let amount = s.intensity.clamp(0.0, 100.0) / 100.0;
    let mut out = img.clone();
    match s.dither {
        Dither::None | Dither::Ordered => {
            for (x, y, p) in out.enumerate_pixels_mut() {
                let mut c = [p[0] as f32, p[1] as f32, p[2] as f32];
                if s.dither == Dither::Ordered {
                    let t = (BAYER8[(y % 8) as usize][(x % 8) as usize] as f32 + 0.5) / 64.0 - 0.5;
                    for v in c.iter_mut() {
                        *v += t * 64.0 * amount;
                    }
                }
                let [r, g, b] = palette[nearest.find(c)];
                *p = Rgba([r, g, b, p[3]]);
            }
        }
        Dither::Jarvis | Dither::Stucki | Dither::FloydSteinberg => {
            let method = match s.dither {
                Dither::Jarvis => BwMethod::Jarvis,
                Dither::Stucki => BwMethod::Stucki,
                _ => BwMethod::FloydSteinberg,
            };
            let (k, div) = kernel(method);
            let mut buf: Vec<[f32; 3]> = img
                .pixels()
                .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
                .collect();
            for y in 0..h {
                for x in 0..w {
                    let i = (y * w + x) as usize;
                    let c = buf[i];
                    let pc = palette[nearest.find(c)];
                    let a = img.get_pixel(x, y)[3];
                    out.put_pixel(x, y, Rgba([pc[0], pc[1], pc[2], a]));
                    let err = [
                        (c[0] - pc[0] as f32) * amount,
                        (c[1] - pc[1] as f32) * amount,
                        (c[2] - pc[2] as f32) * amount,
                    ];
                    for (dx, dy, wt) in k {
                        let (nx, ny) = (x as i64 + *dx as i64, y as i64 + *dy as i64);
                        if nx >= 0 && ny >= 0 && (nx as u32) < w && (ny as u32) < h {
                            let j = (ny as u32 * w + nx as u32) as usize;
                            for ch in 0..3 {
                                buf[j][ch] += err[ch] * wt / div;
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

// ----- Lab -------------------------------------------------------------------

fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB (0 to 255) to CIE Lab (D65).
pub fn rgb_to_lab(c: [u8; 3]) -> [f32; 3] {
    let [r, g, b] = c.map(|v| srgb_to_linear(v as f32 / 255.0));
    let x = (0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b) / 0.950_47;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b;
    let z = (0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b) / 1.088_83;
    let f = |t: f32| {
        if t > 216.0 / 24389.0 {
            t.cbrt()
        } else {
            (24389.0 / 27.0 * t + 16.0) / 116.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// CIE Lab (D65) to sRGB.
pub fn lab_to_rgb(lab: [f32; 3]) -> [u8; 3] {
    let fy = (lab[0] + 16.0) / 116.0;
    let fx = fy + lab[1] / 500.0;
    let fz = fy - lab[2] / 200.0;
    let inv = |t: f32| {
        let t3 = t * t * t;
        if t3 > 216.0 / 24389.0 {
            t3
        } else {
            (116.0 * t - 16.0) * 27.0 / 24389.0
        }
    };
    let (x, y, z) = (inv(fx) * 0.950_47, inv(fy), inv(fz) * 1.088_83);
    let r = 3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z;
    let g = -0.969_266 * x + 1.876_010_8 * y + 0.041_556 * z;
    let b = 0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z;
    [r, g, b].map(|v| clamp8(linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0))
}

/// Lab Color (24-bit): the pixels go through Lab and back (8 bits a
/// channel), which they survive unchanged within rounding.
pub fn lab_mode(img: &RgbaImage) -> RgbaImage {
    let mut out = img.clone();
    for p in out.pixels_mut() {
        let [r, g, b] = lab_to_rgb(rgb_to_lab([p[0], p[1], p[2]]));
        *p = Rgba([r, g, b, p[3]]);
    }
    out
}

/// A monochrome bitmap's two colours, the darker (foreground) first;
/// `None` when its opaque pixels have more than two colours or none. A
/// single colour pairs with white (or black when it is light).
pub fn mono_colors(img: &RgbaImage) -> Option<([u8; 3], [u8; 3])> {
    let mut a: Option<[u8; 3]> = None;
    let mut b: Option<[u8; 3]> = None;
    for p in img.pixels().filter(|p| p[3] > 0) {
        let c = [p[0], p[1], p[2]];
        if a.is_none() || a == Some(c) {
            a = Some(c);
        } else if b.is_none() || b == Some(c) {
            b = Some(c);
        } else {
            return None;
        }
    }
    let a = a?;
    let l = |c: [u8; 3]| luma(&Rgba([c[0], c[1], c[2], 255]));
    let b = b.unwrap_or(if l(a) < 128.0 { [255; 3] } else { [0; 3] });
    Some(if l(a) <= l(b) { (a, b) } else { (b, a) })
}

/// A monochrome bitmap with its foreground (or background) pixels in
/// `to`; `None` when it is not monochrome.
pub fn recolor_mono(img: &RgbaImage, foreground: bool, to: [u8; 3]) -> Option<RgbaImage> {
    let (fg, bg) = mono_colors(img)?;
    let from = if foreground { fg } else { bg };
    let mut out = img.clone();
    for p in out.pixels_mut() {
        if p[3] > 0 && [p[0], p[1], p[2]] == from {
            *p = Rgba([to[0], to[1], to[2], p[3]]);
        }
    }
    Some(out)
}

impl crate::app::App {
    /// Colour the selected monochrome bitmaps from the palette: the background (white) pixels with a click, the
    /// foreground (black) ones with a right-click. False (nothing done)
    /// unless every selected object is a monochrome bitmap without
    /// effects.
    pub fn recolor_mono_bitmaps(&mut self, foreground: bool, c: tracedraw_core::Color) -> bool {
        use tracedraw_core::document::ShapeKind;
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            return false;
        }
        let to = c.to_rgb8();
        let mut cmds = Vec::new();
        for s in &shapes {
            let ShapeKind::Bitmap {
                rect,
                width_px,
                height_px,
                png,
                fx: None,
            } = &s.kind
            else {
                return false;
            };
            let Some(out) =
                crate::bitmap_fx::decode(png).and_then(|img| recolor_mono(&img, foreground, to))
            else {
                return false;
            };
            let Some(png) = crate::bitmap_fx::encode(&out) else {
                return false;
            };
            cmds.push(tracedraw_core::Command::SetShapeKind {
                shape: s.id,
                kind: ShapeKind::Bitmap {
                    rect: *rect,
                    width_px: *width_px,
                    height_px: *height_px,
                    png,
                    fx: None,
                },
            });
        }
        if let Err(e) = self.engine.run_batch("Bitmap Color", &cmds) {
            self.status = e.to_string();
        }
        true
    }

    /// Run `f` (the image and its pixels per inch) on every selected
    /// bitmap, in one undo step.
    pub fn apply_to_bitmaps(
        &mut self,
        label: &'static str,
        f: impl Fn(&RgbaImage, f32) -> RgbaImage,
    ) {
        use tracedraw_core::{document::ShapeKind, Command};
        let mut cmds = Vec::new();
        for s in self.selected_shapes() {
            let ShapeKind::Bitmap {
                rect,
                width_px,
                png,
                ..
            } = &s.kind
            else {
                continue;
            };
            let Some(img) = crate::bitmap_fx::decode(png) else {
                continue;
            };
            let dpi = (*width_px as f64 / (rect.width().abs().max(1e-6) / 25.4)) as f32;
            let out = f(&img, dpi);
            let Some(png) = crate::bitmap_fx::encode(&out) else {
                continue;
            };
            cmds.push(Command::SetShapeKind {
                shape: s.id,
                kind: ShapeKind::Bitmap {
                    rect: *rect,
                    width_px: out.width(),
                    height_px: out.height(),
                    png,
                    fx: None,
                },
            });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch(label, &cmds) {
            self.status = e.to_string();
        }
    }

    /// The document palette's colours as RGB (the Custom palette).
    pub fn document_palette_rgb(&self) -> Vec<[u8; 3]> {
        self.doc().palette.iter().map(|c| c.to_rgb8()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monochrome_bitmaps_take_palette_colours() {
        use tracedraw_core::document::ShapeKind;
        let img: RgbaImage = ImageBuffer::from_fn(8, 4, |x, _| {
            if x < 4 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        assert_eq!(mono_colors(&img), Some(([0; 3], [255; 3])));
        let gray: RgbaImage = ImageBuffer::from_fn(3, 1, |x, _| Rgba([x as u8 * 100, 0, 0, 255]));
        assert_eq!(mono_colors(&gray), None);
        let mut app = crate::app::App::headless();
        let png = crate::bitmap_fx::encode(&img).expect("png");
        let id = app
            .new_shape(ShapeKind::Bitmap {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 8.0, 4.0),
                width_px: 8,
                height_px: 4,
                png,
                fx: None,
            })
            .expect("bitmap");
        app.select(vec![id]);
        let shown = |app: &crate::app::App| match &app.doc().find_shape(id).expect("bitmap").kind {
            ShapeKind::Bitmap { png, .. } => crate::bitmap_fx::decode(png).expect("png"),
            _ => panic!("not a bitmap"),
        };
        let stroke = app.doc().find_shape(id).expect("bitmap").stroke.clone();
        // A click colours the background, a right-click the foreground.
        app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
            255, 0, 0,
        )));
        assert_eq!(shown(&app).get_pixel(6, 0).0, [255, 0, 0, 255]);
        app.apply_outline_color(Some(tracedraw_core::Color::rgb8(0, 0, 255)));
        assert_eq!(shown(&app).get_pixel(1, 0).0, [0, 0, 255, 255]);
        assert_eq!(shown(&app).get_pixel(6, 0).0, [255, 0, 0, 255]);
        // The object's own outline is left alone.
        assert_eq!(app.doc().find_shape(id).expect("bitmap").stroke, stroke);
        app.undo();
        assert_eq!(shown(&app).get_pixel(1, 0).0, [0, 0, 0, 255]);
    }

    fn ramp() -> RgbaImage {
        ImageBuffer::from_fn(64, 16, |x, y| {
            Rgba([(x * 4) as u8, (y * 16) as u8, (255 - x * 4) as u8, 255])
        })
    }

    fn gray_ramp() -> RgbaImage {
        ImageBuffer::from_fn(256, 8, |x, _| Rgba([x as u8, x as u8, x as u8, 200]))
    }

    #[test]
    fn tone_curves_pass_through_their_points() {
        let c = ToneCurve {
            points: vec![(0.0, 0.0), (64.0, 128.0), (255.0, 255.0)],
            smooth: true,
        };
        assert!((c.eval(64.0) - 128.0).abs() < 1e-3);
        let lut = c.lut();
        assert_eq!((lut[0], lut[64], lut[255]), (0, 128, 255));
        // Monotone between increasing points.
        assert!(lut.windows(2).all(|w| w[1] >= w[0]));
        let lin = ToneCurve {
            smooth: false,
            ..c.clone()
        };
        assert!((lin.eval(32.0) - 64.0).abs() < 1e-3);
        assert_eq!(ToneCurve::identity().lut()[77], 77);
        // Bad points are cleaned up.
        let odd = ToneCurve {
            points: vec![(300.0, f32::NAN), (10.0, 20.0)],
            smooth: true,
        };
        assert_eq!(odd.normalized(), vec![(0.0, 20.0), (255.0, 20.0)]);
    }

    #[test]
    fn black_and_white_methods_give_two_levels_and_follow_tone() {
        let img = gray_ramp();
        for m in BwMethod::ALL {
            let s = BwSettings {
                method: m,
                ..Default::default()
            };
            let out = black_and_white(&img, &s, 300.0);
            assert!(out.pixels().all(|p| p[0] == 0 || p[0] == 255), "{m:?}");
            assert!(out.pixels().all(|p| p[3] == 200));
            // Darker on the left than on the right.
            let white = |x0: u32, x1: u32| {
                (x0..x1)
                    .flat_map(|x| (0..8).map(move |y| (x, y)))
                    .filter(|(x, y)| out.get_pixel(*x, *y)[0] == 255)
                    .count()
            };
            assert!(white(0, 64) < white(192, 256), "{m:?}");
        }
        // Line art is a plain threshold.
        let s = BwSettings {
            method: BwMethod::LineArt,
            threshold: 100,
            ..Default::default()
        };
        let out = black_and_white(&img, &s, 300.0);
        assert_eq!(out.get_pixel(99, 0)[0], 0);
        assert_eq!(out.get_pixel(100, 0)[0], 255);
    }

    #[test]
    fn duotone_inks_multiply() {
        let s = DuotoneSettings::default();
        let luts: Vec<[u8; 256]> = s.inks.iter().map(|i| i.curve.lut()).collect();
        // Black gets full black ink: black.
        assert_eq!(duotone_color(&s, &luts, 0), [0, 0, 0]);
        // White gets no ink.
        assert_eq!(duotone_color(&s, &luts, 255), [255, 255, 255]);
        // A monotone in one colour tints the grays with it.
        let mono = DuotoneSettings {
            kind: DuotoneType::Monotone,
            inks: vec![Ink {
                rgb: [255, 0, 0],
                curve: ToneCurve::line(255.0, 0.0),
            }],
        };
        let luts: Vec<[u8; 256]> = mono.inks.iter().map(|i| i.curve.lut()).collect();
        let mid = duotone_color(&mono, &luts, 128);
        assert_eq!(mid[0], 255);
        assert!(mid[1] > 100 && mid[1] < 160 && mid[1] == mid[2]);
        let out = duotone(&ramp(), &s);
        assert_eq!(out.dimensions(), (64, 16));
    }

    #[test]
    fn paletted_uses_only_palette_colours() {
        let img = ramp();
        for p in PaletteType::ALL {
            for d in Dither::ALL {
                let s = PalettedSettings {
                    palette: p,
                    colors: 8,
                    dither: d,
                    ..Default::default()
                };
                let pal = build_palette(&img, &s);
                assert!(!pal.is_empty() && pal.len() <= 256, "{p:?}");
                let out = paletted(&img, &s);
                assert!(
                    out.pixels().all(|q| pal.contains(&[q[0], q[1], q[2]])),
                    "{p:?} {d:?}"
                );
            }
        }
        let s = PalettedSettings {
            palette: PaletteType::Adaptive,
            colors: 8,
            ..Default::default()
        };
        assert!(build_palette(&img, &s).len() <= 8);
        // Few colours keep them all.
        let two = ImageBuffer::from_fn(4, 4, |x, _| {
            if x < 2 {
                Rgba([10, 20, 30, 255])
            } else {
                Rgba([200, 100, 0, 255])
            }
        });
        assert_eq!(build_palette(&two, &s).len(), 2);
        assert_eq!(paletted(&two, &s), two);
    }

    #[test]
    fn conversions_apply_to_every_selected_bitmap_in_one_step() {
        use tracedraw_core::{document::ShapeKind, geometry::Rect};
        let mut app = crate::app::App::headless();
        let img = gray_ramp();
        let png = crate::bitmap_fx::encode(&img).expect("png");
        let mk = |app: &mut crate::app::App| {
            app.new_shape(ShapeKind::Bitmap {
                rect: Rect::new(0.0, 0.0, 256.0 / 300.0 * 25.4, 8.0 / 300.0 * 25.4),
                width_px: 256,
                height_px: 8,
                png: png.clone(),
                fx: None,
            })
            .expect("bitmap")
        };
        let (a, b) = (mk(&mut app), mk(&mut app));
        app.select(vec![a, b]);
        let depth = app.engine.history_labels().0.len();
        let seen = std::cell::Cell::new(0.0f32);
        app.apply_to_bitmaps("Black and White", |img, dpi| {
            seen.set(dpi);
            black_and_white(img, &BwSettings::default(), dpi)
        });
        assert!((seen.get() - 300.0).abs() < 0.01, "{}", seen.get());
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        for id in [a, b] {
            let s = app.doc().find_shape(id).cloned().expect("bitmap");
            let ShapeKind::Bitmap { png, .. } = &s.kind else {
                panic!();
            };
            let out = crate::bitmap_fx::decode(png).expect("decodes");
            assert!(out.pixels().all(|p| p[0] == 0 || p[0] == 255));
        }
    }

    #[test]
    fn lab_round_trips() {
        for c in [
            [0, 0, 0],
            [255, 255, 255],
            [255, 0, 0],
            [12, 200, 77],
            [128, 128, 128],
        ] {
            let back = lab_to_rgb(rgb_to_lab(c));
            for k in 0..3 {
                assert!((back[k] as i32 - c[k] as i32).abs() <= 1, "{c:?} {back:?}");
            }
        }
        let l = rgb_to_lab([255, 255, 255]);
        assert!((l[0] - 100.0).abs() < 0.05 && l[1].abs() < 0.05 && l[2].abs() < 0.05);
        assert_eq!(lab_mode(&ramp()).dimensions(), (64, 16));
    }
}
