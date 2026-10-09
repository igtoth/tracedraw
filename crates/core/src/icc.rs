//! ICC profile reader and colour transform in pure Rust, no dependencies.
//!
//! Supported: v2 and v4 profiles; matrix/TRC RGB profiles (`rXYZ`, `gXYZ`,
//! `bXYZ`, `rTRC`, `gTRC`, `bTRC` with `curv` or `para` curves); gray
//! profiles (`kTRC`); LUT profiles with `mft1` (lut8), `mft2` (lut16),
//! `mAB ` and `mBA ` tags for the A2Bn and B2An transforms, with
//! multilinear CLUT interpolation. The PCS may be XYZ or Lab, including the
//! legacy 16-bit Lab encoding used by `mft2`.
//!
//! Layouts follow the public ICC.1 specification (ICC.1:2001-04 for v2 and
//! ICC.1:2010 for v4). Every reader returns `None` or an error on malformed
//! input; nothing here panics on file data.
//!
//! Internal values are `f64`; device values are normalised to 0..1 and the
//! PCS is carried as actual XYZ (D50) or Lab numbers between pipelines.

use std::fmt;
use std::sync::Arc;

/// The ICC PCS illuminant, D50, as XYZ.
pub const D50: [f64; 3] = [0.9642, 1.0, 0.8249];

/// Default round-trip tolerance (CIE76 delta E) for the gamut check.
pub const GAMUT_TOLERANCE: f32 = 3.0;

const MAX_CHANNELS: usize = 15;
const MAX_CLUT_INPUTS: usize = 8;
const HEADER_LEN: usize = 128;

const fn sig(s: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*s)
}

const SIG_ACSP: u32 = sig(b"acsp");
const SIG_A2B: [u32; 3] = [sig(b"A2B0"), sig(b"A2B1"), sig(b"A2B2")];
const SIG_B2A: [u32; 3] = [sig(b"B2A0"), sig(b"B2A1"), sig(b"B2A2")];
const SIG_RXYZ: u32 = sig(b"rXYZ");
const SIG_GXYZ: u32 = sig(b"gXYZ");
const SIG_BXYZ: u32 = sig(b"bXYZ");
const SIG_RTRC: u32 = sig(b"rTRC");
const SIG_GTRC: u32 = sig(b"gTRC");
const SIG_BTRC: u32 = sig(b"bTRC");
const SIG_KTRC: u32 = sig(b"kTRC");
const SIG_WTPT: u32 = sig(b"wtpt");
const SIG_CHAD: u32 = sig(b"chad");
const SIG_DESC: u32 = sig(b"desc");

const TYPE_XYZ: u32 = sig(b"XYZ ");
const TYPE_CURV: u32 = sig(b"curv");
const TYPE_PARA: u32 = sig(b"para");
const TYPE_SF32: u32 = sig(b"sf32");
const TYPE_LUT8: u32 = sig(b"mft1");
const TYPE_LUT16: u32 = sig(b"mft2");
const TYPE_MAB: u32 = sig(b"mAB ");
const TYPE_MBA: u32 = sig(b"mBA ");
const TYPE_DESC: u32 = sig(b"desc");
const TYPE_MLUC: u32 = sig(b"mluc");

/// Why a profile could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IccError {
    /// Fewer bytes than a header and tag count.
    TooShort,
    /// The `acsp` signature is missing.
    BadSignature,
    /// Major version is not 2, 3 or 4.
    BadVersion(u8),
    /// The data colour space signature is unknown.
    UnsupportedColorSpace(String),
    /// The PCS is neither XYZ nor Lab.
    BadPcs(String),
    /// The tag table runs past the end of the data.
    TagTableOutOfRange,
    /// A tag points outside the data.
    TagOutOfRange(String),
}

impl fmt::Display for IccError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IccError::TooShort => write!(f, "profile too short"),
            IccError::BadSignature => write!(f, "missing acsp signature"),
            IccError::BadVersion(v) => write!(f, "unsupported profile version {v}"),
            IccError::UnsupportedColorSpace(s) => write!(f, "unsupported colour space {s}"),
            IccError::BadPcs(s) => write!(f, "unsupported PCS {s}"),
            IccError::TagTableOutOfRange => write!(f, "tag table out of range"),
            IccError::TagOutOfRange(s) => write!(f, "tag {s} out of range"),
        }
    }
}

impl std::error::Error for IccError {}

fn sig_string(v: u32) -> String {
    let b = v.to_be_bytes();
    if b.iter().all(|c| c.is_ascii_graphic() || *c == b' ') {
        String::from_utf8_lossy(&b).trim_end().to_string()
    } else {
        format!("{v:08x}")
    }
}

/// Profile/device class from the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileClass {
    Input,
    Display,
    Output,
    DeviceLink,
    ColorSpace,
    Abstract,
    NamedColor,
    Unknown,
}

impl ProfileClass {
    fn from_sig(v: u32) -> ProfileClass {
        match &v.to_be_bytes() {
            b"scnr" => ProfileClass::Input,
            b"mntr" => ProfileClass::Display,
            b"prtr" => ProfileClass::Output,
            b"link" => ProfileClass::DeviceLink,
            b"spac" => ProfileClass::ColorSpace,
            b"abst" => ProfileClass::Abstract,
            b"nmcl" => ProfileClass::NamedColor,
            _ => ProfileClass::Unknown,
        }
    }
}

/// Data colour space of the device side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Xyz,
    Lab,
    Luv,
    YCbCr,
    Yxy,
    Rgb,
    Gray,
    Hsv,
    Hls,
    Cmyk,
    Cmy,
    /// `2CLR` .. `FCLR` multi-ink spaces.
    NColor(u8),
}

impl ColorSpace {
    fn from_sig(v: u32) -> Option<ColorSpace> {
        let b = v.to_be_bytes();
        Some(match &b {
            b"XYZ " => ColorSpace::Xyz,
            b"Lab " => ColorSpace::Lab,
            b"Luv " => ColorSpace::Luv,
            b"YCbr" => ColorSpace::YCbCr,
            b"Yxy " => ColorSpace::Yxy,
            b"RGB " => ColorSpace::Rgb,
            b"GRAY" => ColorSpace::Gray,
            b"HSV " => ColorSpace::Hsv,
            b"HLS " => ColorSpace::Hls,
            b"CMYK" => ColorSpace::Cmyk,
            b"CMY " => ColorSpace::Cmy,
            [n, b'C', b'L', b'R'] => {
                let count = match n {
                    b'2'..=b'9' => n - b'0',
                    b'A'..=b'F' => n - b'A' + 10,
                    _ => return None,
                };
                ColorSpace::NColor(count)
            }
            _ => return None,
        })
    }

    /// Number of device channels.
    pub fn channels(self) -> usize {
        match self {
            ColorSpace::Gray => 1,
            ColorSpace::Cmyk => 4,
            ColorSpace::NColor(n) => n as usize,
            _ => 3,
        }
    }
}

/// Profile connection space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pcs {
    Xyz,
    Lab,
}

/// Rendering intent. Absolute colorimetric uses the colorimetric tables
/// with media white scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Intent {
    Perceptual,
    #[default]
    RelativeColorimetric,
    Saturation,
    AbsoluteColorimetric,
}

impl Intent {
    /// Parse the names the colour management dialog shows (case
    /// insensitive, "colorimetric" or "colourimetric").
    pub fn from_name(name: &str) -> Option<Intent> {
        let n = name.trim().to_ascii_lowercase().replace("colour", "color");
        match n.as_str() {
            "perceptual" => Some(Intent::Perceptual),
            "relative colorimetric" | "relative" => Some(Intent::RelativeColorimetric),
            "saturation" => Some(Intent::Saturation),
            "absolute colorimetric" | "absolute" => Some(Intent::AbsoluteColorimetric),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Intent::Perceptual => "Perceptual",
            Intent::RelativeColorimetric => "Relative colorimetric",
            Intent::Saturation => "Saturation",
            Intent::AbsoluteColorimetric => "Absolute colorimetric",
        }
    }

    /// Index of the A2Bn/B2An table this intent reads.
    fn table(self) -> usize {
        match self {
            Intent::Perceptual => 0,
            Intent::RelativeColorimetric | Intent::AbsoluteColorimetric => 1,
            Intent::Saturation => 2,
        }
    }
}

// ---------------------------------------------------------------------------
// Byte reading
// ---------------------------------------------------------------------------

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn slice(&self, off: usize, len: usize) -> Option<&'a [u8]> {
        self.0.get(off..off.checked_add(len)?)
    }

    fn u8(&self, off: usize) -> Option<u8> {
        self.0.get(off).copied()
    }

    fn u16(&self, off: usize) -> Option<u16> {
        let b: [u8; 2] = self.slice(off, 2)?.try_into().ok()?;
        Some(u16::from_be_bytes(b))
    }

    fn u32(&self, off: usize) -> Option<u32> {
        let b: [u8; 4] = self.slice(off, 4)?.try_into().ok()?;
        Some(u32::from_be_bytes(b))
    }

    fn s15f16(&self, off: usize) -> Option<f64> {
        Some(self.u32(off)? as i32 as f64 / 65536.0)
    }

    fn xyz(&self, off: usize) -> Option<[f64; 3]> {
        Some([
            self.s15f16(off)?,
            self.s15f16(off + 4)?,
            self.s15f16(off + 8)?,
        ])
    }
}

fn clamp01(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(0.0, 1.0)
    }
}

fn round4(n: usize) -> usize {
    (n + 3) & !3
}

// ---------------------------------------------------------------------------
// Curves
// ---------------------------------------------------------------------------

/// A one-dimensional transfer curve mapping 0..1 to 0..1.
#[derive(Debug, Clone, PartialEq)]
pub enum Curve {
    Identity,
    /// `curv` with one entry: a plain gamma.
    Gamma(f64),
    /// `curv` sample table, uniformly spaced over 0..1.
    Table(Vec<f64>),
    /// `para` parametric curve, function types 0..4.
    Parametric {
        kind: u16,
        p: [f64; 7],
    },
}

fn spow(x: f64, g: f64) -> f64 {
    if x <= 0.0 {
        0.0
    } else {
        x.powf(g)
    }
}

impl Curve {
    /// Parse a `curv` or `para` element at `off`; returns the curve and its
    /// unpadded byte length.
    fn parse(data: &[u8], off: usize) -> Option<(Curve, usize)> {
        let r = Reader(data);
        match r.u32(off)? {
            TYPE_CURV => {
                let n = r.u32(off + 8)? as usize;
                match n {
                    0 => Some((Curve::Identity, 12)),
                    1 => Some((Curve::Gamma(r.u16(off + 12)? as f64 / 256.0), 14)),
                    _ => {
                        let bytes = r.slice(off + 12, n.checked_mul(2)?)?;
                        let table = bytes
                            .chunks_exact(2)
                            .map(|c| u16::from_be_bytes([c[0], c[1]]) as f64 / 65535.0)
                            .collect();
                        Some((Curve::Table(table), 12 + 2 * n))
                    }
                }
            }
            TYPE_PARA => {
                let kind = r.u16(off + 8)?;
                let count = match kind {
                    0 => 1,
                    1 => 3,
                    2 => 4,
                    3 => 5,
                    4 => 7,
                    _ => return None,
                };
                let mut p = [0.0; 7];
                for (i, slot) in p.iter_mut().take(count).enumerate() {
                    *slot = r.s15f16(off + 12 + 4 * i)?;
                }
                Some((Curve::Parametric { kind, p }, 12 + 4 * count))
            }
            _ => None,
        }
    }

    /// Evaluate the curve at `x` (clamped to 0..1); the result is clamped
    /// to 0..1 as the 16-bit pipelines do.
    pub fn eval(&self, x: f64) -> f64 {
        let x = clamp01(x);
        let y = match self {
            Curve::Identity => x,
            Curve::Gamma(g) => spow(x, *g),
            Curve::Table(t) => {
                let n = t.len();
                if n == 0 {
                    x
                } else if n == 1 {
                    t.first().copied().unwrap_or(x)
                } else {
                    let pos = x * (n - 1) as f64;
                    let i0 = (pos.floor() as usize).min(n - 1);
                    let f = pos - i0 as f64;
                    let a = t.get(i0).copied().unwrap_or(0.0);
                    let b = t.get(i0 + 1).copied().unwrap_or(a);
                    a + (b - a) * f
                }
            }
            Curve::Parametric { kind, p } => {
                let [g, a, b, c, d, e, f] = *p;
                let knee = if a.abs() < 1e-12 { 0.0 } else { -b / a };
                match kind {
                    0 => spow(x, g),
                    1 => {
                        if x >= knee {
                            spow(a * x + b, g)
                        } else {
                            0.0
                        }
                    }
                    2 => {
                        if x >= knee {
                            spow(a * x + b, g) + c
                        } else {
                            c
                        }
                    }
                    3 => {
                        if x >= d {
                            spow(a * x + b, g)
                        } else {
                            c * x
                        }
                    }
                    _ => {
                        if x >= d {
                            spow(a * x + b, g) + e
                        } else {
                            c * x + f
                        }
                    }
                }
            }
        };
        clamp01(y)
    }

    /// Inverse of `eval`, analytic for gamma curves and by bisection for
    /// the others (curves are assumed monotonic, as the spec requires).
    pub fn eval_inverse(&self, y: f64) -> f64 {
        let y = clamp01(y);
        match self {
            Curve::Identity => y,
            Curve::Gamma(g) => {
                if g.abs() < 1e-9 {
                    y
                } else {
                    spow(y, 1.0 / g)
                }
            }
            _ => {
                let increasing = self.eval(1.0) >= self.eval(0.0);
                let (mut lo, mut hi) = (0.0, 1.0);
                for _ in 0..48 {
                    let mid = 0.5 * (lo + hi);
                    if (self.eval(mid) < y) == increasing {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                0.5 * (lo + hi)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pipeline stages
// ---------------------------------------------------------------------------

/// A small fixed-capacity vector of channel values.
#[derive(Debug, Clone, Copy)]
struct Vals {
    v: [f64; MAX_CHANNELS],
    n: usize,
}

impl Vals {
    fn zeros(n: usize) -> Vals {
        Vals {
            v: [0.0; MAX_CHANNELS],
            n: n.min(MAX_CHANNELS),
        }
    }

    fn from_slice(s: &[f64]) -> Vals {
        let mut out = Vals::zeros(s.len());
        for (o, i) in out.v.iter_mut().zip(s) {
            *o = *i;
        }
        out
    }

    fn as_slice(&self) -> &[f64] {
        self.v.get(..self.n.min(MAX_CHANNELS)).unwrap_or(&[])
    }

    fn first3(&self) -> [f64; 3] {
        let g = |i: usize| self.v.get(i).copied().unwrap_or(0.0);
        [g(0), g(1), g(2)]
    }
}

/// How the 0..1 pipeline values encode the PCS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PcsEncoding {
    /// u1Fixed15 XYZ: 0x8000 = 1.0.
    Xyz16,
    /// Legacy 16-bit Lab (`mft2`): L 0xFF00 = 100, a/b 0x8000 = 0.
    LabV2,
    /// v4 16-bit and 8-bit Lab: L 0xFFFF = 100, a/b 0x8080 = 0.
    LabV4,
}

impl PcsEncoding {
    fn decode(self, v: [f64; 3]) -> [f64; 3] {
        match self {
            PcsEncoding::Xyz16 => v.map(|x| x * (65535.0 / 32768.0)),
            PcsEncoding::LabV2 => [
                v[0] * (65535.0 / 652.8),
                v[1] * (65535.0 / 256.0) - 128.0,
                v[2] * (65535.0 / 256.0) - 128.0,
            ],
            PcsEncoding::LabV4 => [v[0] * 100.0, v[1] * 255.0 - 128.0, v[2] * 255.0 - 128.0],
        }
    }

    fn encode(self, v: [f64; 3]) -> [f64; 3] {
        let e = match self {
            PcsEncoding::Xyz16 => v.map(|x| x * (32768.0 / 65535.0)),
            PcsEncoding::LabV2 => [
                v[0] * (652.8 / 65535.0),
                (v[1] + 128.0) * (256.0 / 65535.0),
                (v[2] + 128.0) * (256.0 / 65535.0),
            ],
            PcsEncoding::LabV4 => [v[0] / 100.0, (v[1] + 128.0) / 255.0, (v[2] + 128.0) / 255.0],
        };
        e.map(clamp01)
    }
}

/// Multi-dimensional lookup table with multilinear interpolation.
#[derive(Clone)]
struct Clut {
    grid: Vec<usize>,
    out_ch: usize,
    data: Vec<f32>,
}

impl fmt::Debug for Clut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Clut(grid {:?}, out {})", self.grid, self.out_ch)
    }
}

impl Clut {
    fn entries(grid: &[usize], out_ch: usize) -> Option<usize> {
        if grid.is_empty() || grid.len() > MAX_CLUT_INPUTS || out_ch == 0 {
            return None;
        }
        if grid.iter().any(|g| *g < 2) {
            return None;
        }
        grid.iter().try_fold(out_ch, |acc, g| acc.checked_mul(*g))
    }

    fn new(grid: Vec<usize>, out_ch: usize, data: Vec<f32>) -> Option<Clut> {
        if Clut::entries(&grid, out_ch)? != data.len() {
            return None;
        }
        Some(Clut { grid, out_ch, data })
    }

    fn eval(&self, input: &[f64]) -> Vals {
        let mut out = Vals::zeros(self.out_ch);
        let total = self.data.len();
        let mut cum = 1usize;
        let mut base = 0usize;
        let mut dims = [(0usize, 0.0f64); MAX_CLUT_INPUTS];
        let mut n = 0usize;
        for (slot, (&g, &x)) in dims.iter_mut().zip(self.grid.iter().zip(input.iter())) {
            cum = match cum.checked_mul(g) {
                Some(c) if c > 0 => c,
                _ => return out,
            };
            let stride = total / cum;
            let pos = clamp01(x) * (g - 1) as f64;
            let i0 = (pos.floor() as usize).min(g - 1);
            let i1 = (i0 + 1).min(g - 1);
            *slot = (stride * (i1 - i0), pos - i0 as f64);
            base += i0 * stride;
            n += 1;
        }
        for corner in 0..(1usize << n) {
            let mut weight = 1.0;
            let mut offset = base;
            for (bit, (step, frac)) in dims.iter().take(n).enumerate() {
                if (corner >> bit) & 1 == 1 {
                    weight *= frac;
                    offset += step;
                } else {
                    weight *= 1.0 - frac;
                }
            }
            if weight == 0.0 {
                continue;
            }
            for (o, out_v) in out.v.iter_mut().take(self.out_ch).enumerate() {
                *out_v += weight * self.data.get(offset + o).copied().unwrap_or(0.0) as f64;
            }
        }
        out
    }
}

#[derive(Debug, Clone)]
enum Stage {
    Curves(Vec<Curve>),
    InverseCurves(Vec<Curve>),
    /// 3x3 matrix plus offset on the first three channels.
    Matrix {
        m: [[f64; 3]; 3],
        off: [f64; 3],
        clamp: bool,
    },
    Clut(Clut),
    DecodePcs(PcsEncoding),
    EncodePcs(PcsEncoding),
    /// Gray Y to XYZ along the D50 axis.
    GrayToXyz,
    XyzToGray,
    XyzToLab,
    LabToXyz,
}

fn mat_mul(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn mat_inverse(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let [[a, b, c], [d, e, f], [g, h, i]] = *m;
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-12 || !det.is_finite() {
        return None;
    }
    let inv = 1.0 / det;
    Some([
        [
            (e * i - f * h) * inv,
            (c * h - b * i) * inv,
            (b * f - c * e) * inv,
        ],
        [
            (f * g - d * i) * inv,
            (a * i - c * g) * inv,
            (c * d - a * f) * inv,
        ],
        [
            (d * h - e * g) * inv,
            (b * g - a * h) * inv,
            (a * e - b * d) * inv,
        ],
    ])
}

impl Stage {
    fn eval(&self, input: &Vals) -> Vals {
        match self {
            Stage::Curves(cs) => {
                let mut out = Vals::zeros(cs.len());
                for ((o, c), x) in out.v.iter_mut().zip(cs).zip(input.as_slice()) {
                    *o = c.eval(*x);
                }
                out
            }
            Stage::InverseCurves(cs) => {
                let mut out = Vals::zeros(cs.len());
                for ((o, c), x) in out.v.iter_mut().zip(cs).zip(input.as_slice()) {
                    *o = c.eval_inverse(*x);
                }
                out
            }
            Stage::Matrix { m, off, clamp } => {
                let v = mat_mul(m, input.first3());
                let mut r = [v[0] + off[0], v[1] + off[1], v[2] + off[2]];
                if *clamp {
                    r = r.map(clamp01);
                }
                Vals::from_slice(&r)
            }
            Stage::Clut(c) => c.eval(input.as_slice()),
            Stage::DecodePcs(e) => Vals::from_slice(&e.decode(input.first3())),
            Stage::EncodePcs(e) => Vals::from_slice(&e.encode(input.first3())),
            Stage::GrayToXyz => {
                let y = clamp01(input.v.first().copied().unwrap_or(0.0));
                Vals::from_slice(&D50.map(|w| w * y))
            }
            Stage::XyzToGray => Vals::from_slice(&[clamp01(input.first3()[1] / D50[1])]),
            Stage::XyzToLab => Vals::from_slice(&xyz_to_lab(input.first3())),
            Stage::LabToXyz => Vals::from_slice(&lab_to_xyz(input.first3())),
        }
    }
}

/// A chain of stages from device values to actual PCS values (A2B) or
/// from actual PCS values to device values (B2A).
#[derive(Debug, Clone)]
struct Pipeline {
    in_ch: usize,
    out_ch: usize,
    stages: Vec<Stage>,
}

impl Pipeline {
    fn eval(&self, input: &[f64]) -> Vals {
        let mut v = Vals::from_slice(input);
        v.n = self.in_ch.min(MAX_CHANNELS);
        for s in &self.stages {
            v = s.eval(&v);
        }
        v.n = self.out_ch.min(MAX_CHANNELS);
        v
    }
}

// ---------------------------------------------------------------------------
// Lab helpers (D50)
// ---------------------------------------------------------------------------

/// CIE XYZ (D50) to CIE L*a*b*.
pub fn xyz_to_lab(xyz: [f64; 3]) -> [f64; 3] {
    let f = |t: f64| {
        if t > 216.0 / 24389.0 {
            t.cbrt()
        } else {
            (841.0 / 108.0) * t + 4.0 / 29.0
        }
    };
    let fx = f(xyz[0] / D50[0]);
    let fy = f(xyz[1] / D50[1]);
    let fz = f(xyz[2] / D50[2]);
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// CIE L*a*b* to CIE XYZ (D50).
pub fn lab_to_xyz(lab: [f64; 3]) -> [f64; 3] {
    let finv = |t: f64| {
        if t > 6.0 / 29.0 {
            t * t * t
        } else {
            (108.0 / 841.0) * (t - 4.0 / 29.0)
        }
    };
    let fy = (lab[0] + 16.0) / 116.0;
    let fx = fy + lab[1] / 500.0;
    let fz = fy - lab[2] / 200.0;
    [D50[0] * finv(fx), D50[1] * finv(fy), D50[2] * finv(fz)]
}

/// CIE76 colour difference between two Lab values.
pub fn delta_e76(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn pcs_to_xyz(pcs: Pcs, v: [f64; 3]) -> [f64; 3] {
    match pcs {
        Pcs::Xyz => v,
        Pcs::Lab => lab_to_xyz(v),
    }
}

fn xyz_to_pcs(pcs: Pcs, v: [f64; 3]) -> [f64; 3] {
    match pcs {
        Pcs::Xyz => v,
        Pcs::Lab => xyz_to_lab(v),
    }
}

// ---------------------------------------------------------------------------
// Profile
// ---------------------------------------------------------------------------

/// One entry of the tag table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagEntry {
    pub signature: u32,
    pub offset: u32,
    pub size: u32,
}

/// A parsed ICC profile with its device-to-PCS and PCS-to-device
/// pipelines. Pipelines are shared, so cloning a profile is cheap.
#[derive(Debug, Clone)]
pub struct Profile {
    size: u32,
    version: (u8, u8),
    class: ProfileClass,
    color_space: ColorSpace,
    pcs: Pcs,
    description: Option<String>,
    white_point: [f64; 3],
    chad: Option<[[f64; 3]; 3]>,
    tags: Vec<TagEntry>,
    a2b: [Option<Arc<Pipeline>>; 3],
    b2a: [Option<Arc<Pipeline>>; 3],
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    A2B,
    B2A,
}

impl Profile {
    /// Parse a profile from its bytes.
    pub fn parse(data: &[u8]) -> Result<Profile, IccError> {
        let r = Reader(data);
        if data.len() < HEADER_LEN + 4 {
            return Err(IccError::TooShort);
        }
        if r.u32(36) != Some(SIG_ACSP) {
            return Err(IccError::BadSignature);
        }
        let size = r.u32(0).unwrap_or(0);
        let major = r.u8(8).unwrap_or(0);
        if !(2..=4).contains(&major) {
            return Err(IccError::BadVersion(major));
        }
        let minor = r.u8(9).unwrap_or(0) >> 4;
        let class = ProfileClass::from_sig(r.u32(12).unwrap_or(0));
        let space_sig = r.u32(16).unwrap_or(0);
        let color_space = ColorSpace::from_sig(space_sig)
            .ok_or_else(|| IccError::UnsupportedColorSpace(sig_string(space_sig)))?;
        let pcs_sig = r.u32(20).unwrap_or(0);
        let pcs = match &pcs_sig.to_be_bytes() {
            b"XYZ " => Pcs::Xyz,
            b"Lab " => Pcs::Lab,
            _ => return Err(IccError::BadPcs(sig_string(pcs_sig))),
        };

        let count = r.u32(HEADER_LEN).unwrap_or(0) as usize;
        if count > (data.len() - HEADER_LEN - 4) / 12 {
            return Err(IccError::TagTableOutOfRange);
        }
        let mut tags = Vec::with_capacity(count);
        for i in 0..count {
            let base = HEADER_LEN + 4 + 12 * i;
            let (Some(signature), Some(offset), Some(sz)) =
                (r.u32(base), r.u32(base + 4), r.u32(base + 8))
            else {
                return Err(IccError::TagTableOutOfRange);
            };
            let end = (offset as usize).checked_add(sz as usize);
            if !matches!(end, Some(e) if e <= data.len()) {
                return Err(IccError::TagOutOfRange(sig_string(signature)));
            }
            tags.push(TagEntry {
                signature,
                offset,
                size: sz,
            });
        }

        let mut profile = Profile {
            size,
            version: (major, minor),
            class,
            color_space,
            pcs,
            description: None,
            white_point: D50,
            chad: None,
            tags,
            a2b: [None, None, None],
            b2a: [None, None, None],
        };
        profile.description = profile.tag_bytes(data, SIG_DESC).and_then(parse_desc);
        if let Some(w) = profile.tag_bytes(data, SIG_WTPT).and_then(parse_xyz_tag) {
            if w.iter().all(|c| c.is_finite() && *c > 0.0) {
                profile.white_point = w;
            }
        }
        profile.chad = profile
            .tag_bytes(data, SIG_CHAD)
            .and_then(parse_sf32_matrix);

        for i in 0..3 {
            profile.a2b[i] = profile
                .tag_bytes(data, SIG_A2B[i])
                .and_then(|t| profile.parse_lut_tag(t, Direction::A2B))
                .map(Arc::new);
            profile.b2a[i] = profile
                .tag_bytes(data, SIG_B2A[i])
                .and_then(|t| profile.parse_lut_tag(t, Direction::B2A))
                .map(Arc::new);
        }

        let fallback = profile
            .matrix_trc_pipelines(data)
            .or_else(|| profile.gray_pipelines(data));
        if let Some((a2b, b2a)) = fallback {
            let (a2b, b2a) = (Arc::new(a2b), Arc::new(b2a));
            for i in 0..3 {
                if profile.a2b[i].is_none() {
                    profile.a2b[i] = Some(a2b.clone());
                }
                if profile.b2a[i].is_none() {
                    profile.b2a[i] = Some(b2a.clone());
                }
            }
        }
        Ok(profile)
    }

    /// The built-in sRGB display profile (D50-adapted primaries, the
    /// standard piecewise transfer curve). Used as the screen target.
    pub fn srgb() -> Profile {
        let trc = Curve::Parametric {
            kind: 3,
            p: [
                2.4,
                1.0 / 1.055,
                0.055 / 1.055,
                1.0 / 12.92,
                0.04045,
                0.0,
                0.0,
            ],
        };
        let m = [
            [0.436066, 0.385147, 0.143066],
            [0.222488, 0.716873, 0.060608],
            [0.013916, 0.097076, 0.714096],
        ];
        let (a2b, b2a) = matrix_trc(m, [trc.clone(), trc.clone(), trc]).unwrap_or((
            Pipeline {
                in_ch: 3,
                out_ch: 3,
                stages: Vec::new(),
            },
            Pipeline {
                in_ch: 3,
                out_ch: 3,
                stages: Vec::new(),
            },
        ));
        let (a2b, b2a) = (Arc::new(a2b), Arc::new(b2a));
        Profile {
            size: 0,
            version: (4, 3),
            class: ProfileClass::Display,
            color_space: ColorSpace::Rgb,
            pcs: Pcs::Xyz,
            description: Some("sRGB (built-in)".to_string()),
            white_point: D50,
            chad: None,
            tags: Vec::new(),
            a2b: [Some(a2b.clone()), Some(a2b.clone()), Some(a2b)],
            b2a: [Some(b2a.clone()), Some(b2a.clone()), Some(b2a)],
        }
    }

    /// Profile size from the header.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// (major, minor) version.
    pub fn version(&self) -> (u8, u8) {
        self.version
    }

    pub fn class(&self) -> ProfileClass {
        self.class
    }

    pub fn color_space(&self) -> ColorSpace {
        self.color_space
    }

    pub fn pcs(&self) -> Pcs {
        self.pcs
    }

    /// Number of device channels.
    pub fn channels(&self) -> usize {
        self.color_space.channels()
    }

    /// Profile description (`desc` tag), when present.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Media white point (`wtpt`), D50 when absent.
    pub fn white_point(&self) -> [f64; 3] {
        self.white_point
    }

    /// Chromatic adaptation matrix (`chad`), when present.
    pub fn chad(&self) -> Option<[[f64; 3]; 3]> {
        self.chad
    }

    /// The tag table.
    pub fn tags(&self) -> &[TagEntry] {
        &self.tags
    }

    pub fn has_tag(&self, signature: &[u8; 4]) -> bool {
        let s = sig(signature);
        self.tags.iter().any(|t| t.signature == s)
    }

    /// Whether the profile can convert device values to the PCS.
    pub fn has_device_to_pcs(&self) -> bool {
        self.a2b.iter().any(Option::is_some)
    }

    /// Whether the profile can convert PCS values to device values.
    pub fn has_pcs_to_device(&self) -> bool {
        self.b2a.iter().any(Option::is_some)
    }

    fn tag_bytes<'a>(&self, data: &'a [u8], signature: u32) -> Option<&'a [u8]> {
        let t = self.tags.iter().find(|t| t.signature == signature)?;
        Reader(data).slice(t.offset as usize, t.size as usize)
    }

    fn pick(table: &[Option<Arc<Pipeline>>; 3], intent: Intent) -> Option<Arc<Pipeline>> {
        let want = intent.table();
        table
            .get(want)
            .and_then(|p| p.clone())
            .or_else(|| table[0].clone())
            .or_else(|| table[1].clone())
            .or_else(|| table[2].clone())
    }

    fn a2b_for(&self, intent: Intent) -> Option<Arc<Pipeline>> {
        Profile::pick(&self.a2b, intent)
    }

    fn b2a_for(&self, intent: Intent) -> Option<Arc<Pipeline>> {
        Profile::pick(&self.b2a, intent)
    }

    fn device_to_xyz_f64(&self, device: &[f64], intent: Intent) -> Option<[f64; 3]> {
        let p = self.a2b_for(intent)?;
        let out = p.eval(device);
        Some(pcs_to_xyz(self.pcs, out.first3()))
    }

    fn xyz_to_device_f64(&self, xyz: [f64; 3], intent: Intent) -> Option<Vals> {
        let p = self.b2a_for(intent)?;
        let mut out = p.eval(&xyz_to_pcs(self.pcs, xyz));
        for v in out.v.iter_mut() {
            *v = clamp01(*v);
        }
        Some(out)
    }

    /// Device values (0..1 per channel) to XYZ relative to D50.
    pub fn to_xyz(&self, device: &[f32], intent: Intent) -> Option<[f32; 3]> {
        let dev: Vec<f64> = device.iter().map(|v| *v as f64).collect();
        self.device_to_xyz_f64(&dev, intent)
            .map(|x| x.map(|v| v as f32))
    }

    /// Device values to Lab (D50).
    pub fn to_lab(&self, device: &[f32], intent: Intent) -> Option<[f32; 3]> {
        let dev: Vec<f64> = device.iter().map(|v| *v as f64).collect();
        self.device_to_xyz_f64(&dev, intent)
            .map(|x| xyz_to_lab(x).map(|v| v as f32))
    }

    /// XYZ (D50) to device values, clamped to 0..1.
    pub fn from_xyz(&self, xyz: [f32; 3], intent: Intent) -> Option<Vec<f32>> {
        let out = self.xyz_to_device_f64(xyz.map(|v| v as f64), intent)?;
        Some(out.as_slice().iter().map(|v| *v as f32).collect())
    }

    /// Lab (D50) to device values, clamped to 0..1.
    pub fn from_lab(&self, lab: [f32; 3], intent: Intent) -> Option<Vec<f32>> {
        let xyz = lab_to_xyz(lab.map(|v| v as f64));
        let out = self.xyz_to_device_f64(xyz, intent)?;
        Some(out.as_slice().iter().map(|v| *v as f32).collect())
    }

    /// The darkest colour the profile reproduces, as neutral XYZ, for black
    /// point compensation. Uses the round trip of Lab black through B2A and
    /// A2B when both exist, else the device black.
    pub fn black_point(&self, intent: Intent) -> Option<[f64; 3]> {
        let a2b = self.a2b_for(intent)?;
        let device: Vec<f64> = match self.xyz_to_device_f64([0.0; 3], intent) {
            Some(v) => v.as_slice().to_vec(),
            None => {
                let dark = match self.color_space {
                    ColorSpace::Cmyk | ColorSpace::Cmy | ColorSpace::NColor(_) => 1.0,
                    _ => 0.0,
                };
                vec![dark; self.channels()]
            }
        };
        let xyz = pcs_to_xyz(self.pcs, a2b.eval(&device).first3());
        let lab = xyz_to_lab(xyz);
        let l = if lab[0].is_finite() {
            lab[0].clamp(0.0, 50.0)
        } else {
            0.0
        };
        Some(lab_to_xyz([l, 0.0, 0.0]))
    }

    fn parse_lut_tag(&self, tag: &[u8], dir: Direction) -> Option<Pipeline> {
        let r = Reader(tag);
        let dev_ch = self.channels();
        let (want_in, want_out) = match dir {
            Direction::A2B => (dev_ch, 3),
            Direction::B2A => (3, dev_ch),
        };
        let pipeline = match r.u32(0)? {
            TYPE_LUT8 => parse_lut8(tag, dir, self.pcs, self.color_space)?,
            TYPE_LUT16 => parse_lut16(tag, dir, self.pcs, self.color_space)?,
            TYPE_MAB if dir == Direction::A2B => parse_mab(tag, self.pcs)?,
            TYPE_MBA if dir == Direction::B2A => parse_mba(tag, self.pcs)?,
            _ => return None,
        };
        if pipeline.in_ch != want_in || pipeline.out_ch != want_out {
            return None;
        }
        Some(pipeline)
    }

    fn matrix_trc_pipelines(&self, data: &[u8]) -> Option<(Pipeline, Pipeline)> {
        if self.color_space != ColorSpace::Rgb || self.pcs != Pcs::Xyz {
            return None;
        }
        let xyz = |s| self.tag_bytes(data, s).and_then(parse_xyz_tag);
        let trc = |s| self.tag_bytes(data, s).and_then(parse_curve_tag);
        let (r, g, b) = (xyz(SIG_RXYZ)?, xyz(SIG_GXYZ)?, xyz(SIG_BXYZ)?);
        let curves = [trc(SIG_RTRC)?, trc(SIG_GTRC)?, trc(SIG_BTRC)?];
        let m = [[r[0], g[0], b[0]], [r[1], g[1], b[1]], [r[2], g[2], b[2]]];
        matrix_trc(m, curves)
    }

    fn gray_pipelines(&self, data: &[u8]) -> Option<(Pipeline, Pipeline)> {
        if self.color_space != ColorSpace::Gray {
            return None;
        }
        let k = self.tag_bytes(data, SIG_KTRC).and_then(parse_curve_tag)?;
        let mut a2b = vec![Stage::Curves(vec![k.clone()]), Stage::GrayToXyz];
        let mut b2a = Vec::new();
        if self.pcs == Pcs::Lab {
            a2b.push(Stage::XyzToLab);
            b2a.push(Stage::LabToXyz);
        }
        b2a.push(Stage::XyzToGray);
        b2a.push(Stage::InverseCurves(vec![k]));
        Some((
            Pipeline {
                in_ch: 1,
                out_ch: 3,
                stages: a2b,
            },
            Pipeline {
                in_ch: 3,
                out_ch: 1,
                stages: b2a,
            },
        ))
    }
}

fn matrix_trc(m: [[f64; 3]; 3], curves: [Curve; 3]) -> Option<(Pipeline, Pipeline)> {
    if m.iter().flatten().any(|v| !v.is_finite()) {
        return None;
    }
    let inv = mat_inverse(&m)?;
    let a2b = Pipeline {
        in_ch: 3,
        out_ch: 3,
        stages: vec![
            Stage::Curves(curves.to_vec()),
            Stage::Matrix {
                m,
                off: [0.0; 3],
                clamp: false,
            },
        ],
    };
    let b2a = Pipeline {
        in_ch: 3,
        out_ch: 3,
        stages: vec![
            Stage::Matrix {
                m: inv,
                off: [0.0; 3],
                clamp: true,
            },
            Stage::InverseCurves(curves.to_vec()),
        ],
    };
    Some((a2b, b2a))
}

fn parse_xyz_tag(tag: &[u8]) -> Option<[f64; 3]> {
    let r = Reader(tag);
    if r.u32(0)? != TYPE_XYZ {
        return None;
    }
    r.xyz(8)
}

fn parse_curve_tag(tag: &[u8]) -> Option<Curve> {
    Curve::parse(tag, 0).map(|(c, _)| c)
}

fn parse_sf32_matrix(tag: &[u8]) -> Option<[[f64; 3]; 3]> {
    let r = Reader(tag);
    if r.u32(0)? != TYPE_SF32 {
        return None;
    }
    let mut m = [[0.0; 3]; 3];
    for (i, cell) in m.iter_mut().flatten().enumerate() {
        *cell = r.s15f16(8 + 4 * i)?;
    }
    Some(m)
}

fn parse_desc(tag: &[u8]) -> Option<String> {
    let r = Reader(tag);
    match r.u32(0)? {
        TYPE_DESC => {
            let n = r.u32(8)? as usize;
            let bytes = r.slice(12, n)?;
            let s = String::from_utf8_lossy(bytes)
                .trim_end_matches('\0')
                .trim()
                .to_string();
            (!s.is_empty()).then_some(s)
        }
        TYPE_MLUC => {
            let count = r.u32(8)?;
            let rec_size = r.u32(12)? as usize;
            if count == 0 || rec_size < 12 {
                return None;
            }
            // Prefer an English record, else the first one.
            let mut chosen = None;
            for i in 0..(count as usize).min(64) {
                let base = 16 + i * rec_size;
                let lang = r.u16(base)?;
                let len = r.u32(base + 4)? as usize;
                let off = r.u32(base + 8)? as usize;
                if chosen.is_none() || lang == u16::from_be_bytes(*b"en") {
                    chosen = Some((len, off));
                    if lang == u16::from_be_bytes(*b"en") {
                        break;
                    }
                }
            }
            let (len, off) = chosen?;
            let bytes = r.slice(off, len)?;
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            let s = String::from_utf16_lossy(&units)
                .trim_end_matches('\0')
                .trim()
                .to_string();
            (!s.is_empty()).then_some(s)
        }
        _ => None,
    }
}

fn lut_pcs_encoding(pcs: Pcs, legacy_lab: bool) -> PcsEncoding {
    match (pcs, legacy_lab) {
        (Pcs::Xyz, _) => PcsEncoding::Xyz16,
        (Pcs::Lab, true) => PcsEncoding::LabV2,
        (Pcs::Lab, false) => PcsEncoding::LabV4,
    }
}

fn read_matrix9(r: &Reader<'_>, off: usize) -> Option<[[f64; 3]; 3]> {
    let mut m = [[0.0; 3]; 3];
    for (i, cell) in m.iter_mut().flatten().enumerate() {
        *cell = r.s15f16(off + 4 * i)?;
    }
    Some(m)
}

fn is_identity(m: &[[f64; 3]; 3]) -> bool {
    m.iter().enumerate().all(|(i, row)| {
        row.iter()
            .enumerate()
            .all(|(j, v)| (*v - if i == j { 1.0 } else { 0.0 }).abs() < 1e-6)
    })
}

/// Shared assembly for lut8 and lut16: matrix (XYZ input only), input
/// curves, CLUT, output curves, plus PCS encode/decode at the right end.
#[allow(clippy::too_many_arguments)]
fn assemble_lut(
    dir: Direction,
    pcs: Pcs,
    device: ColorSpace,
    legacy_lab: bool,
    in_ch: usize,
    out_ch: usize,
    matrix: [[f64; 3]; 3],
    input: Vec<Curve>,
    clut: Clut,
    output: Vec<Curve>,
) -> Pipeline {
    let enc = lut_pcs_encoding(pcs, legacy_lab);
    let input_is_xyz = match dir {
        Direction::A2B => device == ColorSpace::Xyz,
        Direction::B2A => pcs == Pcs::Xyz,
    };
    let mut stages = Vec::new();
    if dir == Direction::B2A {
        stages.push(Stage::EncodePcs(enc));
    }
    if input_is_xyz && in_ch == 3 && !is_identity(&matrix) {
        stages.push(Stage::Matrix {
            m: matrix,
            off: [0.0; 3],
            clamp: true,
        });
    }
    stages.push(Stage::Curves(input));
    stages.push(Stage::Clut(clut));
    stages.push(Stage::Curves(output));
    if dir == Direction::A2B {
        stages.push(Stage::DecodePcs(enc));
    }
    Pipeline {
        in_ch,
        out_ch,
        stages,
    }
}

fn parse_lut8(tag: &[u8], dir: Direction, pcs: Pcs, device: ColorSpace) -> Option<Pipeline> {
    let r = Reader(tag);
    let in_ch = r.u8(8)? as usize;
    let out_ch = r.u8(9)? as usize;
    let grid = r.u8(10)? as usize;
    if in_ch == 0 || in_ch > MAX_CLUT_INPUTS || out_ch == 0 || out_ch > MAX_CHANNELS {
        return None;
    }
    let matrix = read_matrix9(&r, 12)?;
    let mut off = 48;
    let mut input = Vec::with_capacity(in_ch);
    for _ in 0..in_ch {
        let bytes = r.slice(off, 256)?;
        input.push(Curve::Table(
            bytes.iter().map(|b| *b as f64 / 255.0).collect(),
        ));
        off += 256;
    }
    let dims = vec![grid; in_ch];
    let entries = Clut::entries(&dims, out_ch)?;
    let data: Vec<f32> = r
        .slice(off, entries)?
        .iter()
        .map(|b| *b as f32 / 255.0)
        .collect();
    off += entries;
    let clut = Clut::new(dims, out_ch, data)?;
    let mut output = Vec::with_capacity(out_ch);
    for _ in 0..out_ch {
        let bytes = r.slice(off, 256)?;
        output.push(Curve::Table(
            bytes.iter().map(|b| *b as f64 / 255.0).collect(),
        ));
        off += 256;
    }
    Some(assemble_lut(
        dir, pcs, device, false, in_ch, out_ch, matrix, input, clut, output,
    ))
}

fn read_u16_table(r: &Reader<'_>, off: usize, n: usize) -> Option<Vec<f64>> {
    let bytes = r.slice(off, n.checked_mul(2)?)?;
    Some(
        bytes
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]) as f64 / 65535.0)
            .collect(),
    )
}

fn parse_lut16(tag: &[u8], dir: Direction, pcs: Pcs, device: ColorSpace) -> Option<Pipeline> {
    let r = Reader(tag);
    let in_ch = r.u8(8)? as usize;
    let out_ch = r.u8(9)? as usize;
    let grid = r.u8(10)? as usize;
    if in_ch == 0 || in_ch > MAX_CLUT_INPUTS || out_ch == 0 || out_ch > MAX_CHANNELS {
        return None;
    }
    let matrix = read_matrix9(&r, 12)?;
    let in_entries = r.u16(48)? as usize;
    let out_entries = r.u16(50)? as usize;
    if !(2..=4096).contains(&in_entries) || !(2..=4096).contains(&out_entries) {
        return None;
    }
    let mut off = 52;
    let mut input = Vec::with_capacity(in_ch);
    for _ in 0..in_ch {
        input.push(Curve::Table(read_u16_table(&r, off, in_entries)?));
        off += 2 * in_entries;
    }
    let dims = vec![grid; in_ch];
    let entries = Clut::entries(&dims, out_ch)?;
    let data: Vec<f32> = r
        .slice(off, entries.checked_mul(2)?)?
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]) as f32 / 65535.0)
        .collect();
    off += entries * 2;
    let clut = Clut::new(dims, out_ch, data)?;
    let mut output = Vec::with_capacity(out_ch);
    for _ in 0..out_ch {
        output.push(Curve::Table(read_u16_table(&r, off, out_entries)?));
        off += 2 * out_entries;
    }
    Some(assemble_lut(
        dir, pcs, device, true, in_ch, out_ch, matrix, input, clut, output,
    ))
}

/// Read `n` consecutive curves (each padded to 4 bytes) starting at `off`.
fn read_curve_set(tag: &[u8], off: usize, n: usize) -> Option<Vec<Curve>> {
    let mut curves = Vec::with_capacity(n);
    let mut pos = off;
    for _ in 0..n {
        let (c, len) = Curve::parse(tag, pos)?;
        curves.push(c);
        pos = pos.checked_add(round4(len))?;
    }
    Some(curves)
}

fn read_mab_matrix(tag: &[u8], off: usize) -> Option<Stage> {
    let r = Reader(tag);
    let m = read_matrix9(&r, off)?;
    let o = [
        r.s15f16(off + 36)?,
        r.s15f16(off + 40)?,
        r.s15f16(off + 44)?,
    ];
    Some(Stage::Matrix {
        m,
        off: o,
        clamp: true,
    })
}

fn read_mab_clut(tag: &[u8], off: usize, in_ch: usize, out_ch: usize) -> Option<Clut> {
    let r = Reader(tag);
    let grid_bytes = r.slice(off, 16)?;
    let dims: Vec<usize> = grid_bytes.iter().take(in_ch).map(|g| *g as usize).collect();
    let precision = r.u8(off + 16)? as usize;
    let entries = Clut::entries(&dims, out_ch)?;
    let data: Vec<f32> = match precision {
        1 => r
            .slice(off + 20, entries)?
            .iter()
            .map(|b| *b as f32 / 255.0)
            .collect(),
        2 => r
            .slice(off + 20, entries.checked_mul(2)?)?
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]) as f32 / 65535.0)
            .collect(),
        _ => return None,
    };
    Clut::new(dims, out_ch, data)
}

struct MabHeader {
    in_ch: usize,
    out_ch: usize,
    b: usize,
    matrix: usize,
    m: usize,
    clut: usize,
    a: usize,
}

fn read_mab_header(tag: &[u8]) -> Option<MabHeader> {
    let r = Reader(tag);
    let in_ch = r.u8(8)? as usize;
    let out_ch = r.u8(9)? as usize;
    if in_ch == 0 || in_ch > MAX_CHANNELS || out_ch == 0 || out_ch > MAX_CHANNELS {
        return None;
    }
    Some(MabHeader {
        in_ch,
        out_ch,
        b: r.u32(12)? as usize,
        matrix: r.u32(16)? as usize,
        m: r.u32(20)? as usize,
        clut: r.u32(24)? as usize,
        a: r.u32(28)? as usize,
    })
}

/// lutAtoBType: A curves, CLUT, M curves, matrix, B curves.
fn parse_mab(tag: &[u8], pcs: Pcs) -> Option<Pipeline> {
    let h = read_mab_header(tag)?;
    let mut stages = Vec::new();
    if h.clut != 0 {
        if h.a == 0 {
            return None;
        }
        stages.push(Stage::Curves(read_curve_set(tag, h.a, h.in_ch)?));
        stages.push(Stage::Clut(read_mab_clut(tag, h.clut, h.in_ch, h.out_ch)?));
    } else if h.in_ch != h.out_ch {
        return None;
    }
    if h.matrix != 0 {
        if h.out_ch != 3 {
            return None;
        }
        if h.m != 0 {
            stages.push(Stage::Curves(read_curve_set(tag, h.m, h.out_ch)?));
        }
        stages.push(read_mab_matrix(tag, h.matrix)?);
    }
    if h.b == 0 {
        return None;
    }
    stages.push(Stage::Curves(read_curve_set(tag, h.b, h.out_ch)?));
    stages.push(Stage::DecodePcs(lut_pcs_encoding(pcs, false)));
    Some(Pipeline {
        in_ch: h.in_ch,
        out_ch: h.out_ch,
        stages,
    })
}

/// lutBtoAType: B curves, matrix, M curves, CLUT, A curves.
fn parse_mba(tag: &[u8], pcs: Pcs) -> Option<Pipeline> {
    let h = read_mab_header(tag)?;
    let mut stages = vec![Stage::EncodePcs(lut_pcs_encoding(pcs, false))];
    if h.b == 0 {
        return None;
    }
    stages.push(Stage::Curves(read_curve_set(tag, h.b, h.in_ch)?));
    if h.matrix != 0 {
        if h.in_ch != 3 {
            return None;
        }
        stages.push(read_mab_matrix(tag, h.matrix)?);
        if h.m != 0 {
            stages.push(Stage::Curves(read_curve_set(tag, h.m, h.in_ch)?));
        }
    }
    if h.clut != 0 {
        stages.push(Stage::Clut(read_mab_clut(tag, h.clut, h.in_ch, h.out_ch)?));
        if h.a == 0 {
            return None;
        }
        stages.push(Stage::Curves(read_curve_set(tag, h.a, h.out_ch)?));
    } else if h.in_ch != h.out_ch {
        return None;
    }
    Some(Pipeline {
        in_ch: h.in_ch,
        out_ch: h.out_ch,
        stages,
    })
}

// ---------------------------------------------------------------------------
// Transform
// ---------------------------------------------------------------------------

/// A colour transform from one profile's device space to another's.
#[derive(Debug, Clone)]
pub struct Transform {
    src_space: ColorSpace,
    dst_space: ColorSpace,
    src_pcs: Pcs,
    dst_pcs: Pcs,
    intent: Intent,
    a2b: Arc<Pipeline>,
    b2a: Arc<Pipeline>,
    /// Relative colorimetric pipelines for the gamut check.
    check_a2b: Arc<Pipeline>,
    check_b2a: Arc<Pipeline>,
    check_back: Arc<Pipeline>,
    /// Black point compensation as `xyz * scale + offset`.
    bpc: Option<([f64; 3], [f64; 3])>,
    src_white: [f64; 3],
    dst_white: [f64; 3],
}

/// Per-component linear map that sends `bp_src` to `bp_dst` and keeps D50.
fn bpc_map(bp_src: [f64; 3], bp_dst: [f64; 3]) -> Option<([f64; 3], [f64; 3])> {
    let mut scale = [1.0; 3];
    let mut offset = [0.0; 3];
    for i in 0..3 {
        let den = D50[i] - bp_src[i];
        if den.abs() < 1e-9 {
            return None;
        }
        scale[i] = (D50[i] - bp_dst[i]) / den;
        offset[i] = bp_dst[i] - scale[i] * bp_src[i];
        if !scale[i].is_finite() || !offset[i].is_finite() {
            return None;
        }
    }
    Some((scale, offset))
}

impl Transform {
    /// Build a transform; `None` when either profile lacks the needed table
    /// (a device link or named colour profile, or an input-only profile as
    /// destination).
    pub fn new(src: &Profile, dst: &Profile, intent: Intent, bpc: bool) -> Option<Transform> {
        if matches!(
            src.class(),
            ProfileClass::DeviceLink | ProfileClass::NamedColor
        ) || matches!(
            dst.class(),
            ProfileClass::DeviceLink | ProfileClass::NamedColor
        ) {
            return None;
        }
        let a2b = src.a2b_for(intent)?;
        let b2a = dst.b2a_for(intent)?;
        let rel = Intent::RelativeColorimetric;
        let check_a2b = src.a2b_for(rel)?;
        let check_b2a = dst.b2a_for(rel)?;
        let check_back = dst.a2b_for(rel)?;
        let bpc = if bpc && intent != Intent::AbsoluteColorimetric {
            match (src.black_point(intent), dst.black_point(intent)) {
                (Some(s), Some(d)) if delta_e76(xyz_to_lab(s), xyz_to_lab(d)) > 0.01 => {
                    bpc_map(s, d)
                }
                _ => None,
            }
        } else {
            None
        };
        Some(Transform {
            src_space: src.color_space(),
            dst_space: dst.color_space(),
            src_pcs: src.pcs(),
            dst_pcs: dst.pcs(),
            intent,
            a2b,
            b2a,
            check_a2b,
            check_b2a,
            check_back,
            bpc,
            src_white: src.white_point(),
            dst_white: dst.white_point(),
        })
    }

    pub fn intent(&self) -> Intent {
        self.intent
    }

    pub fn input_channels(&self) -> usize {
        self.src_space.channels()
    }

    pub fn output_channels(&self) -> usize {
        self.dst_space.channels()
    }

    pub fn input_space(&self) -> ColorSpace {
        self.src_space
    }

    pub fn output_space(&self) -> ColorSpace {
        self.dst_space
    }

    /// Whether black point compensation is active (both black points were
    /// found and differ).
    pub fn has_bpc(&self) -> bool {
        self.bpc.is_some()
    }

    fn to_xyz(&self, input: &[f64]) -> [f64; 3] {
        let mut xyz = pcs_to_xyz(self.src_pcs, self.a2b.eval(input).first3());
        if self.intent == Intent::AbsoluteColorimetric {
            for i in 0..3 {
                if self.src_white[i].abs() > 1e-9 {
                    xyz[i] *= self.src_white[i] / D50[i];
                }
            }
        }
        if let Some((scale, offset)) = self.bpc {
            for i in 0..3 {
                xyz[i] = xyz[i] * scale[i] + offset[i];
            }
        }
        if self.intent == Intent::AbsoluteColorimetric {
            for i in 0..3 {
                if self.dst_white[i].abs() > 1e-9 {
                    xyz[i] *= D50[i] / self.dst_white[i];
                }
            }
        }
        xyz.map(|v| if v.is_finite() { v } else { 0.0 })
    }

    fn convert(&self, input: &[f64]) -> Vals {
        let xyz = self.to_xyz(input);
        let mut out = self.b2a.eval(&xyz_to_pcs(self.dst_pcs, xyz));
        for v in out.v.iter_mut() {
            *v = clamp01(*v);
        }
        out
    }

    /// Convert one colour; missing input channels read as 0, extra ones are
    /// ignored. Output has `output_channels()` values in 0..1.
    pub fn transform(&self, input: &[f32]) -> Vec<f32> {
        let dev: Vec<f64> = input
            .iter()
            .take(self.input_channels())
            .map(|v| clamp01(*v as f64))
            .collect();
        self.convert(&dev)
            .as_slice()
            .iter()
            .map(|v| *v as f32)
            .collect()
    }

    /// Source colour as XYZ (D50) after intent and BPC adjustments.
    pub fn to_pcs_xyz(&self, input: &[f32]) -> [f32; 3] {
        let dev: Vec<f64> = input.iter().map(|v| clamp01(*v as f64)).collect();
        self.to_xyz(&dev).map(|v| v as f32)
    }

    /// RGB (0..1) to CMYK (0..1); `None` unless the transform is RGB to CMYK.
    pub fn rgb_to_cmyk(&self, rgb: [f32; 3]) -> Option<[f32; 4]> {
        if self.src_space != ColorSpace::Rgb || self.dst_space != ColorSpace::Cmyk {
            return None;
        }
        let out = self.transform(&rgb);
        Some([
            out.first().copied().unwrap_or(0.0),
            out.get(1).copied().unwrap_or(0.0),
            out.get(2).copied().unwrap_or(0.0),
            out.get(3).copied().unwrap_or(0.0),
        ])
    }

    /// CMYK (0..1) to RGB (0..1); `None` unless the transform is CMYK to RGB.
    pub fn cmyk_to_rgb(&self, cmyk: [f32; 4]) -> Option<[f32; 3]> {
        if self.src_space != ColorSpace::Cmyk || self.dst_space != ColorSpace::Rgb {
            return None;
        }
        let out = self.transform(&cmyk);
        Some([
            out.first().copied().unwrap_or(0.0),
            out.get(1).copied().unwrap_or(0.0),
            out.get(2).copied().unwrap_or(0.0),
        ])
    }

    /// Round-trip error (CIE76 delta E) of a source colour through the
    /// destination profile, with relative colorimetric tables and no BPC.
    pub fn gamut_error(&self, input: &[f32]) -> f32 {
        let dev: Vec<f64> = input
            .iter()
            .take(self.input_channels())
            .map(|v| clamp01(*v as f64))
            .collect();
        let xyz = pcs_to_xyz(self.src_pcs, self.check_a2b.eval(&dev).first3());
        let reference = xyz_to_lab(xyz);
        let mut device = self.check_b2a.eval(&xyz_to_pcs(self.dst_pcs, xyz));
        for v in device.v.iter_mut() {
            *v = clamp01(*v);
        }
        let back = pcs_to_xyz(
            self.dst_pcs,
            self.check_back.eval(device.as_slice()).first3(),
        );
        let d = delta_e76(reference, xyz_to_lab(back));
        if d.is_finite() {
            d as f32
        } else {
            f32::MAX
        }
    }

    /// Whether the source colour is reproducible by the destination profile
    /// within `tolerance` delta E.
    pub fn is_in_gamut_with(&self, input: &[f32], tolerance: f32) -> bool {
        self.gamut_error(input) <= tolerance
    }

    /// Gamut check with the default tolerance [`GAMUT_TOLERANCE`].
    pub fn is_in_gamut(&self, input: &[f32]) -> bool {
        self.is_in_gamut_with(input, GAMUT_TOLERANCE)
    }
}

// ---------------------------------------------------------------------------
// Test profile builders (shared with color.rs tests)
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod test_profiles {
    //! Hand-assembled profiles: header, tag table and tag data.

    pub struct Builder {
        tags: Vec<([u8; 4], Vec<u8>)>,
        class: [u8; 4],
        space: [u8; 4],
        pcs: [u8; 4],
        version: u8,
    }

    impl Builder {
        pub fn new(class: &[u8; 4], space: &[u8; 4], pcs: &[u8; 4], version: u8) -> Builder {
            Builder {
                tags: Vec::new(),
                class: *class,
                space: *space,
                pcs: *pcs,
                version,
            }
        }

        pub fn tag(mut self, sig: &[u8; 4], data: Vec<u8>) -> Builder {
            self.tags.push((*sig, data));
            self
        }

        pub fn build(self) -> Vec<u8> {
            let mut body: Vec<u8> = Vec::new();
            let table_len = 4 + 12 * self.tags.len();
            let mut offsets = Vec::new();
            for (_, data) in &self.tags {
                while !body.len().is_multiple_of(4) {
                    body.push(0);
                }
                offsets.push((128 + table_len + body.len()) as u32);
                body.extend_from_slice(data);
            }
            let total = 128 + table_len + body.len();
            let mut out = vec![0u8; 128];
            out[0..4].copy_from_slice(&(total as u32).to_be_bytes());
            out[8] = self.version;
            out[12..16].copy_from_slice(&self.class);
            out[16..20].copy_from_slice(&self.space);
            out[20..24].copy_from_slice(&self.pcs);
            out[36..40].copy_from_slice(b"acsp");
            out[68..80].copy_from_slice(&xyz_bytes([0.9642, 1.0, 0.8249]));
            out.extend_from_slice(&(self.tags.len() as u32).to_be_bytes());
            for ((sig, data), off) in self.tags.iter().zip(&offsets) {
                out.extend_from_slice(sig);
                out.extend_from_slice(&off.to_be_bytes());
                out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            }
            out.extend_from_slice(&body);
            out
        }
    }

    pub fn s15f16(v: f64) -> [u8; 4] {
        ((v * 65536.0).round() as i32).to_be_bytes()
    }

    pub fn xyz_bytes(v: [f64; 3]) -> Vec<u8> {
        let mut out = Vec::new();
        for c in v {
            out.extend_from_slice(&s15f16(c));
        }
        out
    }

    pub fn xyz_tag(v: [f64; 3]) -> Vec<u8> {
        let mut out = b"XYZ \0\0\0\0".to_vec();
        out.extend(xyz_bytes(v));
        out
    }

    pub fn curv_gamma(g: f64) -> Vec<u8> {
        let mut out = b"curv\0\0\0\0".to_vec();
        out.extend_from_slice(&1u32.to_be_bytes());
        out.extend_from_slice(&((g * 256.0).round() as u16).to_be_bytes());
        out
    }

    pub fn curv_table(values: &[u16]) -> Vec<u8> {
        let mut out = b"curv\0\0\0\0".to_vec();
        out.extend_from_slice(&(values.len() as u32).to_be_bytes());
        for v in values {
            out.extend_from_slice(&v.to_be_bytes());
        }
        out
    }

    pub fn para(kind: u16, params: &[f64]) -> Vec<u8> {
        let mut out = b"para\0\0\0\0".to_vec();
        out.extend_from_slice(&kind.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes());
        for p in params {
            out.extend_from_slice(&s15f16(*p));
        }
        out
    }

    pub fn para_srgb() -> Vec<u8> {
        para(3, &[2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045])
    }

    pub const SRGB_R: [f64; 3] = [0.436066, 0.222488, 0.013916];
    pub const SRGB_G: [f64; 3] = [0.385147, 0.716873, 0.097076];
    pub const SRGB_B: [f64; 3] = [0.143066, 0.060608, 0.714096];

    /// A matrix/TRC RGB display profile with sRGB primaries and the given
    /// three TRC tags.
    pub fn matrix_trc_profile(r: Vec<u8>, g: Vec<u8>, b: Vec<u8>) -> Vec<u8> {
        Builder::new(b"mntr", b"RGB ", b"XYZ ", 2)
            .tag(b"desc", desc("Synthetic RGB"))
            .tag(b"wtpt", xyz_tag([0.9642, 1.0, 0.8249]))
            .tag(b"rXYZ", xyz_tag(SRGB_R))
            .tag(b"gXYZ", xyz_tag(SRGB_G))
            .tag(b"bXYZ", xyz_tag(SRGB_B))
            .tag(b"rTRC", r)
            .tag(b"gTRC", g)
            .tag(b"bTRC", b)
            .build()
    }

    pub fn desc(s: &str) -> Vec<u8> {
        let mut out = b"desc\0\0\0\0".to_vec();
        out.extend_from_slice(&((s.len() + 1) as u32).to_be_bytes());
        out.extend_from_slice(s.as_bytes());
        out.push(0);
        out
    }

    /// Legacy-encoded Lab raw triple for the lut16 CLUT test.
    pub fn lab_v2_raw(l: f64, a: f64, b: f64) -> [u16; 3] {
        [
            (l * 652.8).round() as u16,
            ((a + 128.0) * 256.0).round().min(65535.0) as u16,
            ((b + 128.0) * 256.0).round().min(65535.0) as u16,
        ]
    }

    /// A lut16 A2B0 tag, CMYK to Lab, 2 grid points per axis. The CLUT is a
    /// linear function of the inks so interpolation is exact:
    /// L = 100 (1 - (c + m + y + k) / 4), a = 40 (m - c), b = 40 (y - k).
    pub fn lut16_cmyk_lab_tag() -> Vec<u8> {
        let mut out = b"mft2\0\0\0\0".to_vec();
        out.extend_from_slice(&[4, 3, 2, 0]);
        for i in 0..3 {
            for j in 0..3 {
                out.extend_from_slice(&s15f16(if i == j { 1.0 } else { 0.0 }));
            }
        }
        out.extend_from_slice(&2u16.to_be_bytes());
        out.extend_from_slice(&2u16.to_be_bytes());
        for _ in 0..4 {
            out.extend_from_slice(&0u16.to_be_bytes());
            out.extend_from_slice(&65535u16.to_be_bytes());
        }
        for idx in 0..16u32 {
            let c = ((idx >> 3) & 1) as f64;
            let m = ((idx >> 2) & 1) as f64;
            let y = ((idx >> 1) & 1) as f64;
            let k = (idx & 1) as f64;
            let l = 100.0 * (1.0 - (c + m + y + k) / 4.0);
            for v in lab_v2_raw(l, 40.0 * (m - c), 40.0 * (y - k)) {
                out.extend_from_slice(&v.to_be_bytes());
            }
        }
        for _ in 0..3 {
            out.extend_from_slice(&0u16.to_be_bytes());
            out.extend_from_slice(&65535u16.to_be_bytes());
        }
        out
    }

    pub fn lut16_cmyk_profile() -> Vec<u8> {
        Builder::new(b"prtr", b"CMYK", b"Lab ", 2)
            .tag(b"desc", desc("Synthetic CMYK"))
            .tag(b"wtpt", xyz_tag([0.9642, 1.0, 0.8249]))
            .tag(b"A2B0", lut16_cmyk_lab_tag())
            .build()
    }

    /// A lutBtoAType tag, Lab to CMYK with identity B curves, a 2x2x2 8-bit
    /// CLUT and identity A curves: k = 1 - L, c = a_enc, m = b_enc, y = 0.
    pub fn mba_lab_cmyk_tag() -> Vec<u8> {
        let mut out = b"mBA \0\0\0\0".to_vec();
        out.extend_from_slice(&[3, 4, 0, 0]);
        let b_off = 32u32;
        let identity = para(0, &[1.0]);
        let b_len = 3 * identity.len() as u32;
        let clut_off = b_off + b_len;
        let clut_len = 20 + 8 * 4;
        let a_off = clut_off + clut_len;
        out.extend_from_slice(&b_off.to_be_bytes());
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(&clut_off.to_be_bytes());
        out.extend_from_slice(&a_off.to_be_bytes());
        for _ in 0..3 {
            out.extend(identity.clone());
        }
        let mut grid = [0u8; 16];
        grid[0] = 2;
        grid[1] = 2;
        grid[2] = 2;
        out.extend_from_slice(&grid);
        out.extend_from_slice(&[1, 0, 0, 0]);
        for idx in 0..8u8 {
            let l = (idx >> 2) & 1;
            let a = (idx >> 1) & 1;
            let b = idx & 1;
            out.extend_from_slice(&[a * 255, b * 255, 0, 255 - l * 255]);
        }
        for _ in 0..4 {
            out.extend(identity.clone());
        }
        out
    }

    pub fn mba_cmyk_profile() -> Vec<u8> {
        Builder::new(b"prtr", b"CMYK", b"Lab ", 4)
            .tag(b"wtpt", xyz_tag([0.9642, 1.0, 0.8249]))
            .tag(b"A2B0", lut16_cmyk_lab_tag())
            .tag(b"B2A0", mba_lab_cmyk_tag())
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::test_profiles::*;
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn assert_close3(a: [f32; 3], b: [f64; 3], tol: f64) {
        for (x, y) in a.iter().zip(b) {
            assert!(close(*x as f64, y, tol), "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn parses_header_and_tags() {
        let bytes = matrix_trc_profile(para_srgb(), para_srgb(), para_srgb());
        let p = Profile::parse(&bytes).unwrap();
        assert_eq!(p.version().0, 2);
        assert_eq!(p.class(), ProfileClass::Display);
        assert_eq!(p.color_space(), ColorSpace::Rgb);
        assert_eq!(p.pcs(), Pcs::Xyz);
        assert_eq!(p.channels(), 3);
        assert_eq!(p.description(), Some("Synthetic RGB"));
        assert_eq!(p.tags().len(), 8);
        assert!(p.has_tag(b"rXYZ"));
        assert!(!p.has_tag(b"A2B0"));
        assert!(p.has_device_to_pcs() && p.has_pcs_to_device());
        assert_eq!(p.size() as usize, bytes.len());
    }

    #[test]
    fn matrix_trc_white_is_d50_and_round_trips() {
        let bytes =
            matrix_trc_profile(para_srgb(), curv_gamma(2.2), curv_table(&[0, 16384, 65535]));
        let p = Profile::parse(&bytes).unwrap();
        let rel = Intent::RelativeColorimetric;
        let white = p.to_xyz(&[1.0, 1.0, 1.0], rel).unwrap();
        assert_close3(white, D50, 2e-3);
        let black = p.to_xyz(&[0.0, 0.0, 0.0], rel).unwrap();
        assert_close3(black, [0.0; 3], 1e-6);
        for rgb in [
            [0.2f32, 0.5, 0.8],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.5, 0.5, 0.5],
            [0.9, 0.1, 0.3],
        ] {
            let xyz = p.to_xyz(&rgb, rel).unwrap();
            let back = p.from_xyz(xyz, rel).unwrap();
            for (a, b) in back.iter().zip(rgb) {
                assert!(close(*a as f64, b as f64, 1e-4), "{rgb:?} -> {back:?}");
            }
        }
    }

    #[test]
    fn synthetic_srgb_matches_builtin() {
        let bytes = matrix_trc_profile(para_srgb(), para_srgb(), para_srgb());
        let p = Profile::parse(&bytes).unwrap();
        let t = Transform::new(&p, &Profile::srgb(), Intent::RelativeColorimetric, true).unwrap();
        assert!(!t.has_bpc());
        for rgb in [
            [0.1f32, 0.2, 0.3],
            [1.0, 1.0, 1.0],
            [0.0, 0.0, 0.0],
            [0.7, 0.3, 0.9],
        ] {
            let out = t.transform(&rgb);
            assert_eq!(out.len(), 3);
            for (a, b) in out.iter().zip(rgb) {
                assert!(close(*a as f64, b as f64, 2e-3), "{rgb:?} -> {out:?}");
            }
            assert!(t.is_in_gamut(&rgb));
        }
    }

    #[test]
    fn curves_evaluate_and_invert() {
        let srgb = Curve::Parametric {
            kind: 3,
            p: [
                2.4,
                1.0 / 1.055,
                0.055 / 1.055,
                1.0 / 12.92,
                0.04045,
                0.0,
                0.0,
            ],
        };
        assert!(close(srgb.eval(0.5), 0.214041, 1e-5));
        assert!(close(srgb.eval(0.02), 0.02 / 12.92, 1e-9));
        assert!(close(srgb.eval_inverse(srgb.eval(0.37)), 0.37, 1e-9));
        let g = Curve::Gamma(2.2);
        assert!(close(g.eval_inverse(g.eval(0.6)), 0.6, 1e-12));
        let t = Curve::Table(vec![0.0, 0.25, 1.0]);
        assert!(close(t.eval(0.25), 0.125, 1e-12));
        assert!(close(t.eval_inverse(0.125), 0.25, 1e-9));
        assert_eq!(Curve::Identity.eval(2.0), 1.0);
        assert_eq!(Curve::Identity.eval(f64::NAN), 0.0);
        for kind in 0..5u16 {
            let c = Curve::Parametric {
                kind,
                p: [2.0, 1.0, 0.0, 0.1, 0.2, 0.0, 0.0],
            };
            let v = c.eval(0.5);
            assert!((0.0..=1.0).contains(&v));
        }
    }

    #[test]
    fn lut16_cmyk_corners_and_midpoints() {
        let p = Profile::parse(&lut16_cmyk_profile()).unwrap();
        assert_eq!(p.class(), ProfileClass::Output);
        assert_eq!(p.color_space(), ColorSpace::Cmyk);
        assert_eq!(p.pcs(), Pcs::Lab);
        assert!(p.has_device_to_pcs());
        assert!(!p.has_pcs_to_device());
        let i = Intent::Perceptual;
        let lab = |cmyk: [f32; 4]| p.to_lab(&cmyk, i).unwrap();
        assert_close3(lab([0.0, 0.0, 0.0, 0.0]), [100.0, 0.0, 0.0], 0.01);
        assert_close3(lab([1.0, 1.0, 1.0, 1.0]), [0.0, 0.0, 0.0], 0.01);
        assert_close3(lab([1.0, 0.0, 0.0, 0.0]), [75.0, -40.0, 0.0], 0.01);
        assert_close3(lab([0.0, 1.0, 0.0, 0.0]), [75.0, 40.0, 0.0], 0.01);
        assert_close3(lab([0.0, 0.0, 1.0, 0.0]), [75.0, 0.0, 40.0], 0.01);
        assert_close3(lab([0.0, 0.0, 0.0, 1.0]), [75.0, 0.0, -40.0], 0.01);
        // Midpoints: the CLUT is linear so interpolation reproduces it.
        assert_close3(lab([0.5, 0.5, 0.5, 0.5]), [50.0, 0.0, 0.0], 0.01);
        assert_close3(lab([0.5, 0.0, 0.0, 0.0]), [87.5, -20.0, 0.0], 0.01);
        assert_close3(lab([0.25, 0.75, 0.0, 0.5]), [62.5, 20.0, -20.0], 0.01);
        // Relative colorimetric falls back to A2B0.
        assert_close3(
            p.to_lab(&[0.0; 4], Intent::RelativeColorimetric).unwrap(),
            [100.0, 0.0, 0.0],
            0.01,
        );
        // No B2A: cannot be a destination.
        assert!(Transform::new(&Profile::srgb(), &p, i, true).is_none());
        // Short input reads missing channels as zero.
        assert_close3(lab_short(&p), [75.0, -40.0, 0.0], 0.01);
    }

    fn lab_short(p: &Profile) -> [f32; 3] {
        p.to_lab(&[1.0], Intent::Perceptual).unwrap()
    }

    #[test]
    fn lut16_cmyk_to_srgb_transform() {
        let cmyk = Profile::parse(&lut16_cmyk_profile()).unwrap();
        let srgb = Profile::srgb();
        let t = Transform::new(&cmyk, &srgb, Intent::RelativeColorimetric, true).unwrap();
        assert_eq!(t.input_channels(), 4);
        assert_eq!(t.output_channels(), 3);
        let white = t.cmyk_to_rgb([0.0, 0.0, 0.0, 0.0]).unwrap();
        assert_close3(white, [1.0, 1.0, 1.0], 2e-3);
        let black = t.cmyk_to_rgb([1.0, 1.0, 1.0, 1.0]).unwrap();
        assert_close3(black, [0.0, 0.0, 0.0], 2e-3);
        let cyanish = t.cmyk_to_rgb([1.0, 0.0, 0.0, 0.0]).unwrap();
        assert!(
            cyanish[0] < cyanish[1] && cyanish[0] < cyanish[2],
            "{cyanish:?}"
        );
        assert!(t.rgb_to_cmyk([1.0, 0.0, 0.0]).is_none());
        // Transforming with fewer channels than needed does not panic.
        assert_eq!(t.transform(&[1.0]).len(), 3);
        assert_eq!(t.transform(&[]).len(), 3);
    }

    #[test]
    fn mba_lab_to_cmyk_and_gamut() {
        let cmyk = Profile::parse(&mba_cmyk_profile()).unwrap();
        assert!(cmyk.has_pcs_to_device());
        let i = Intent::RelativeColorimetric;
        let white = cmyk.from_lab([100.0, 0.0, 0.0], i).unwrap();
        assert_eq!(white.len(), 4);
        assert!(close(white[0] as f64, 128.0 / 255.0, 1e-3), "{white:?}");
        assert!(close(white[1] as f64, 128.0 / 255.0, 1e-3));
        assert!(close(white[2] as f64, 0.0, 1e-6));
        assert!(close(white[3] as f64, 0.0, 1e-3));
        let black = cmyk.from_lab([0.0, 0.0, 0.0], i).unwrap();
        assert!(close(black[3] as f64, 1.0, 1e-3), "{black:?}");
        let mid = cmyk.from_lab([50.0, -128.0, 127.0], i).unwrap();
        assert!(close(mid[0] as f64, 0.0, 1e-3), "{mid:?}");
        assert!(close(mid[1] as f64, 1.0, 1e-3));
        assert!(close(mid[3] as f64, 0.5, 1e-3));

        let srgb = Profile::srgb();
        let t = Transform::new(&srgb, &cmyk, i, false).unwrap();
        let k = t.rgb_to_cmyk([1.0, 1.0, 1.0]).unwrap();
        assert!(close(k[3] as f64, 0.0, 2e-3), "{k:?}");
        let k = t.rgb_to_cmyk([0.0, 0.0, 0.0]).unwrap();
        assert!(close(k[3] as f64, 1.0, 2e-3), "{k:?}");
        assert!(t.cmyk_to_rgb([0.0; 4]).is_none());
        // The synthetic CMYK model is not the inverse of the A2B table, so
        // saturated colours come back far away: out of gamut.
        assert!(t.gamut_error(&[1.0, 0.0, 0.0]) > GAMUT_TOLERANCE);
        assert!(!t.is_in_gamut(&[1.0, 0.0, 0.0]));
        assert!(t.is_in_gamut_with(&[1.0, 0.0, 0.0], 1000.0));
    }

    #[test]
    fn black_point_compensation_maps_blacks() {
        let (scale, offset) = bpc_map([0.0; 3], lab_to_xyz([10.0, 0.0, 0.0])).unwrap();
        let bp = lab_to_xyz([10.0, 0.0, 0.0]);
        for i in 0..3 {
            assert!(close(0.0 * scale[i] + offset[i], bp[i], 1e-9));
            assert!(close(D50[i] * scale[i] + offset[i], D50[i], 1e-9));
        }
        assert!(bpc_map(D50, [0.0; 3]).is_none());
        // Matrix/TRC profiles have black at zero; no compensation needed.
        let p = Profile::srgb();
        assert_close3(
            p.black_point(Intent::RelativeColorimetric)
                .unwrap()
                .map(|v| v as f32),
            [0.0; 3],
            1e-6,
        );
        // The synthetic CMYK profile's B2A is not the inverse of its A2B:
        // Lab black maps to (0.502, 0.502, 0, 1), which the A2B table reads
        // back as L = 49.9. That is its measured black point, so sRGB black
        // lands on it with BPC (k about 0.5) and on full ink without.
        let cmyk = Profile::parse(&mba_cmyk_profile()).unwrap();
        let bp = cmyk.black_point(Intent::Perceptual).unwrap();
        assert!(close(xyz_to_lab(bp)[0], 49.902, 0.01), "{bp:?}");
        let with = Transform::new(&Profile::srgb(), &cmyk, Intent::Perceptual, true).unwrap();
        assert!(with.has_bpc());
        let k = with.rgb_to_cmyk([0.0, 0.0, 0.0]).unwrap();
        assert!(close(k[3] as f64, 0.501, 2e-3), "{k:?}");
        let white = with.rgb_to_cmyk([1.0, 1.0, 1.0]).unwrap();
        assert!(close(white[3] as f64, 0.0, 2e-3), "{white:?}");
        let without = Transform::new(&Profile::srgb(), &cmyk, Intent::Perceptual, false).unwrap();
        assert!(!without.has_bpc());
        let k = without.rgb_to_cmyk([0.0, 0.0, 0.0]).unwrap();
        assert!(close(k[3] as f64, 1.0, 2e-3), "{k:?}");
    }

    #[test]
    fn absolute_intent_scales_by_media_white() {
        let bytes = Builder::new(b"mntr", b"RGB ", b"XYZ ", 2)
            .tag(b"wtpt", xyz_tag([0.9505, 1.0, 1.089]))
            .tag(b"rXYZ", xyz_tag(SRGB_R))
            .tag(b"gXYZ", xyz_tag(SRGB_G))
            .tag(b"bXYZ", xyz_tag(SRGB_B))
            .tag(b"rTRC", para_srgb())
            .tag(b"gTRC", para_srgb())
            .tag(b"bTRC", para_srgb())
            .build();
        let p = Profile::parse(&bytes).unwrap();
        let t = Transform::new(&p, &Profile::srgb(), Intent::AbsoluteColorimetric, true).unwrap();
        let xyz = t.to_pcs_xyz(&[1.0, 1.0, 1.0]);
        assert!(close(xyz[2] as f64, 1.089, 5e-3), "{xyz:?}");
        assert!(xyz[2] > xyz[0]);
    }

    #[test]
    fn intent_names() {
        assert_eq!(
            Intent::from_name("Relative colorimetric"),
            Some(Intent::RelativeColorimetric)
        );
        assert_eq!(
            Intent::from_name("relative colourimetric"),
            Some(Intent::RelativeColorimetric)
        );
        assert_eq!(Intent::from_name("Perceptual"), Some(Intent::Perceptual));
        assert_eq!(
            Intent::from_name("Absolute colorimetric"),
            Some(Intent::AbsoluteColorimetric)
        );
        assert_eq!(Intent::from_name("Saturation"), Some(Intent::Saturation));
        assert_eq!(Intent::from_name("vivid"), None);
        assert_eq!(Intent::default().name(), "Relative colorimetric");
    }

    #[test]
    fn lab_helpers() {
        let lab = xyz_to_lab(D50);
        assert_close3(lab.map(|v| v as f32), [100.0, 0.0, 0.0], 1e-6);
        let back = lab_to_xyz([50.0, 20.0, -30.0]);
        let again = xyz_to_lab(back);
        assert_close3(again.map(|v| v as f32), [50.0, 20.0, -30.0], 1e-6);
        assert!(close(delta_e76([0.0; 3], [3.0, 4.0, 0.0]), 5.0, 1e-12));
    }

    #[test]
    fn malformed_inputs_do_not_panic() {
        let good = matrix_trc_profile(para_srgb(), para_srgb(), para_srgb());
        assert_eq!(Profile::parse(&[]).err(), Some(IccError::TooShort));
        assert_eq!(Profile::parse(&good[..100]).err(), Some(IccError::TooShort));
        // Truncated after the tag table: tags point past the end.
        assert!(matches!(
            Profile::parse(&good[..140]),
            Err(IccError::TagTableOutOfRange) | Err(IccError::TagOutOfRange(_))
        ));
        assert!(matches!(
            Profile::parse(&good[..good.len() - 1]),
            Err(IccError::TagOutOfRange(_))
        ));
        let mut bad_sig = good.clone();
        bad_sig[36..40].copy_from_slice(b"nope");
        assert_eq!(Profile::parse(&bad_sig).err(), Some(IccError::BadSignature));
        let mut bad_ver = good.clone();
        bad_ver[8] = 5;
        assert_eq!(
            Profile::parse(&bad_ver).err(),
            Some(IccError::BadVersion(5))
        );
        let mut bad_space = good.clone();
        bad_space[16..20].copy_from_slice(b"ZZZZ");
        assert!(matches!(
            Profile::parse(&bad_space),
            Err(IccError::UnsupportedColorSpace(_))
        ));
        let mut bad_pcs = good.clone();
        bad_pcs[20..24].copy_from_slice(b"RGB ");
        assert!(matches!(Profile::parse(&bad_pcs), Err(IccError::BadPcs(_))));
        let mut huge_count = good.clone();
        huge_count[128..132].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            Profile::parse(&huge_count).err(),
            Some(IccError::TagTableOutOfRange)
        );
        let mut far_tag = good.clone();
        far_tag[136..140].copy_from_slice(&0xFFFF_FFF0u32.to_be_bytes());
        assert!(matches!(
            Profile::parse(&far_tag),
            Err(IccError::TagOutOfRange(_))
        ));
        let mut huge_size_tag = good.clone();
        huge_size_tag[140..144].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(matches!(
            Profile::parse(&huge_size_tag),
            Err(IccError::TagOutOfRange(_))
        ));

        // A tag whose body is garbage: the profile parses but has no
        // transforms (the broken TRC is skipped).
        let broken = matrix_trc_profile(vec![1, 2, 3], para_srgb(), para_srgb());
        let p = Profile::parse(&broken).unwrap();
        assert!(!p.has_device_to_pcs());
        assert!(Transform::new(&p, &Profile::srgb(), Intent::Perceptual, false).is_none());

        // lut16 whose CLUT claims more entries than the data holds.
        let mut lut = lut16_cmyk_lab_tag();
        lut[10] = 200;
        let bytes = Builder::new(b"prtr", b"CMYK", b"Lab ", 2)
            .tag(b"A2B0", lut)
            .build();
        let p = Profile::parse(&bytes).unwrap();
        assert!(!p.has_device_to_pcs());
        // lut16 with absurd channel counts.
        let mut lut = lut16_cmyk_lab_tag();
        lut[8] = 255;
        lut[9] = 255;
        let bytes = Builder::new(b"prtr", b"CMYK", b"Lab ", 2)
            .tag(b"A2B0", lut)
            .build();
        assert!(!Profile::parse(&bytes).unwrap().has_device_to_pcs());
        // lut16 whose channel count does not match the colour space.
        let bytes = Builder::new(b"prtr", b"RGB ", b"Lab ", 2)
            .tag(b"A2B0", lut16_cmyk_lab_tag())
            .build();
        assert!(!Profile::parse(&bytes).unwrap().has_device_to_pcs());
        // mBA with offsets past the end of the tag.
        let mut mba = mba_lab_cmyk_tag();
        mba[24..28].copy_from_slice(&0xFFFFu32.to_be_bytes());
        let bytes = Builder::new(b"prtr", b"CMYK", b"Lab ", 4)
            .tag(b"B2A0", mba)
            .build();
        assert!(!Profile::parse(&bytes).unwrap().has_pcs_to_device());
        // Random bytes with a valid header shape.
        let mut noise = good.clone();
        for (i, b) in noise.iter_mut().enumerate().skip(132) {
            *b = (i * 7919 % 251) as u8;
        }
        let _ = Profile::parse(&noise);
    }

    #[test]
    fn gray_profile_round_trips() {
        let bytes = Builder::new(b"mntr", b"GRAY", b"XYZ ", 2)
            .tag(b"kTRC", curv_gamma(1.8))
            .build();
        let p = Profile::parse(&bytes).unwrap();
        assert_eq!(p.channels(), 1);
        let i = Intent::RelativeColorimetric;
        assert_close3(p.to_xyz(&[1.0], i).unwrap(), D50, 1e-6);
        let xyz = p.to_xyz(&[0.5], i).unwrap();
        let back = p.from_xyz(xyz, i).unwrap();
        assert!(close(back[0] as f64, 0.5, 1e-6));
        let t = Transform::new(&p, &Profile::srgb(), i, true).unwrap();
        let rgb = t.transform(&[0.5]);
        assert!(close(rgb[0] as f64, rgb[1] as f64, 1e-3));
        assert!(close(rgb[1] as f64, rgb[2] as f64, 1e-3));
    }

    #[test]
    fn mluc_description_is_read() {
        let mut mluc = b"mluc\0\0\0\0".to_vec();
        mluc.extend_from_slice(&1u32.to_be_bytes());
        mluc.extend_from_slice(&12u32.to_be_bytes());
        mluc.extend_from_slice(b"enUS");
        let text: Vec<u8> = "Wide"
            .encode_utf16()
            .flat_map(|u| u.to_be_bytes())
            .collect();
        mluc.extend_from_slice(&(text.len() as u32).to_be_bytes());
        mluc.extend_from_slice(&28u32.to_be_bytes());
        mluc.extend_from_slice(&text);
        let bytes = Builder::new(b"mntr", b"RGB ", b"XYZ ", 4)
            .tag(b"desc", mluc)
            .build();
        let p = Profile::parse(&bytes).unwrap();
        assert_eq!(p.description(), Some("Wide"));
    }
}
