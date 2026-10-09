//! Colour values. TraceDraw keeps the colour model the user chose, since
//! print work is CMYK-first; spot colours keep their name and a CMYK
//! fallback. Screen conversion is the naive formula unless a colour engine
//! (ICC) is registered through `engine::set`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "model", rename_all = "lowercase")]
pub enum Color {
    /// sRGB, components in 0.0..=1.0.
    Rgb { r: f32, g: f32, b: f32 },
    /// CMYK, components in 0.0..=1.0 (0 = no ink).
    Cmyk { c: f32, m: f32, y: f32, k: f32 },
    /// Grayscale, 0.0 = black, 1.0 = white.
    Gray { v: f32 },
    /// Hue 0..360, saturation and brightness 0..1.
    Hsb { h: f32, s: f32, b: f32 },
    /// Hue 0..360, saturation and lightness 0..1.
    Hsl { h: f32, s: f32, l: f32 },
    /// CIE L*a*b* (D65), L 0..100, a and b about -128..127.
    Lab { l: f32, a: f32, b: f32 },
    /// NTSC YIQ, each component 0..1 (I and Q centred on 0.5).
    Yiq { y: f32, i: f32, q: f32 },
    /// Registration colour: prints on every separation.
    Registration,
}

/// Hook for an ICC colour engine; when set, `to_rgb8` uses it for CMYK
/// and Lab. See `docs/decisions.md` D5.
pub mod engine {
    use super::Color;
    use std::sync::OnceLock;
    pub type Converter = fn(Color) -> [u8; 3];
    static ENGINE: OnceLock<Converter> = OnceLock::new();
    pub fn set(f: Converter) {
        let _ = ENGINE.set(f);
    }
    pub fn get() -> Option<Converter> {
        ENGINE.get().copied()
    }
}

impl Color {
    pub const BLACK: Color = Color::Rgb {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
    pub const WHITE: Color = Color::Rgb {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };

    pub fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Color::Rgb {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
        }
    }

    pub fn cmyk8(c: u8, m: u8, y: u8, k: u8) -> Self {
        Color::Cmyk {
            c: c as f32 / 255.0,
            m: m as f32 / 255.0,
            y: y as f32 / 255.0,
            k: k as f32 / 255.0,
        }
    }

    /// CMYK given in percent, the way the target design shows it.
    pub fn cmyk_pct(c: f32, m: f32, y: f32, k: f32) -> Self {
        Color::Cmyk {
            c: c / 100.0,
            m: m / 100.0,
            y: y / 100.0,
            k: k / 100.0,
        }
    }

    pub fn model_name(self) -> &'static str {
        match self {
            Color::Rgb { .. } => "RGB",
            Color::Cmyk { .. } => "CMYK",
            Color::Gray { .. } => "Grayscale",
            Color::Hsb { .. } => "HSB",
            Color::Hsl { .. } => "HSL",
            Color::Lab { .. } => "Lab",
            Color::Yiq { .. } => "YIQ",
            Color::Registration => "Registration",
        }
    }

    /// Screen colour as 8-bit sRGB. Uses the registered engine when there
    /// is one, else the naive formulas.
    pub fn to_rgb8(self) -> [u8; 3] {
        if let Some(conv) = engine::get() {
            if matches!(self, Color::Cmyk { .. } | Color::Lab { .. }) {
                return conv(self);
            }
        }
        let [r, g, b] = self.to_rgb_f32();
        let f = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        [f(r), f(g), f(b)]
    }

    /// Screen colour as floats 0..1 (naive conversions).
    pub fn to_rgb_f32(self) -> [f32; 3] {
        match self {
            Color::Rgb { r, g, b } => [r, g, b],
            Color::Cmyk { c, m, y, k } => [
                (1.0 - c) * (1.0 - k),
                (1.0 - m) * (1.0 - k),
                (1.0 - y) * (1.0 - k),
            ],
            Color::Gray { v } => [v, v, v],
            Color::Registration => [0.0, 0.0, 0.0],
            Color::Hsb { h, s, b } => hsb_to_rgb(h, s, b),
            Color::Hsl { h, s, l } => {
                // HSL -> HSB then to RGB.
                let b = l + s * l.min(1.0 - l);
                let s2 = if b <= 0.0 { 0.0 } else { 2.0 * (1.0 - l / b) };
                hsb_to_rgb(h, s2, b)
            }
            Color::Lab { l, a, b } => lab_to_rgb(l, a, b),
            Color::Yiq { y, i, q } => {
                let i = (i - 0.5) * 1.1914;
                let q = (q - 0.5) * 1.0452;
                [
                    y + 0.956 * i + 0.619 * q,
                    y - 0.272 * i - 0.647 * q,
                    y - 1.106 * i + 1.703 * q,
                ]
            }
        }
    }

    /// Convert to another model by way of sRGB (naive, no ICC).
    pub fn convert_to(self, model: &str) -> Color {
        let [r, g, b] = self.to_rgb_f32();
        let (r, g, b) = (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
        match model {
            "RGB" => Color::Rgb { r, g, b },
            "CMYK" => {
                let k = 1.0 - r.max(g).max(b);
                if k >= 1.0 - 1e-6 {
                    return Color::Cmyk {
                        c: 0.0,
                        m: 0.0,
                        y: 0.0,
                        k: 1.0,
                    };
                }
                Color::Cmyk {
                    c: (1.0 - r - k) / (1.0 - k),
                    m: (1.0 - g - k) / (1.0 - k),
                    y: (1.0 - b - k) / (1.0 - k),
                    k,
                }
            }
            "Grayscale" => Color::Gray {
                v: luminance(r, g, b),
            },
            "HSB" => {
                let (h, s, v) = rgb_to_hsb(r, g, b);
                Color::Hsb { h, s, b: v }
            }
            "HSL" => {
                let (h, s, v) = rgb_to_hsb(r, g, b);
                let l = v * (1.0 - s / 2.0);
                let sl = if l <= 0.0 || l >= 1.0 {
                    0.0
                } else {
                    (v - l) / l.min(1.0 - l)
                };
                Color::Hsl { h, s: sl, l }
            }
            "Lab" => {
                let (l, a, bb) = rgb_to_lab(r, g, b);
                Color::Lab { l, a, b: bb }
            }
            "YIQ" => {
                let y = 0.299 * r + 0.587 * g + 0.114 * b;
                let i = 0.596 * r - 0.274 * g - 0.322 * b;
                let q = 0.211 * r - 0.523 * g + 0.312 * b;
                Color::Yiq {
                    y,
                    i: i / 1.1914 + 0.5,
                    q: q / 1.0452 + 0.5,
                }
            }
            "Registration" => Color::Registration,
            _ => self,
        }
    }

    /// Hue, saturation, brightness (0..360, 0..1, 0..1).
    pub fn to_hsb(self) -> (f64, f64, f64) {
        let [r, g, b] = self.to_rgb_f32();
        let (h, s, v) = rgb_to_hsb(r, g, b);
        (h as f64, s as f64, v as f64)
    }

    pub fn from_hsb(h: f64, s: f64, b: f64) -> Color {
        let [r, g, bb] = hsb_to_rgb(h as f32, s.clamp(0.0, 1.0) as f32, b.clamp(0.0, 1.0) as f32);
        Color::Rgb { r, g, b: bb }
    }

    /// Relative luminance 0..1 of the screen colour.
    pub fn luminance(self) -> f32 {
        let [r, g, b] = self.to_rgb_f32();
        luminance(r, g, b)
    }

    /// `#rrggbb` for SVG and CSS.
    pub fn to_hex(self) -> String {
        let [r, g, b] = self.to_rgb8();
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    /// Parse `#rgb`, `#rrggbb` or `rrggbb`.
    pub fn from_hex(s: &str) -> Option<Color> {
        let s = s.trim().trim_start_matches('#');
        let v = u32::from_str_radix(s, 16).ok()?;
        match s.len() {
            6 => Some(Color::rgb8((v >> 16) as u8, (v >> 8) as u8, v as u8)),
            3 => {
                let r = ((v >> 8) & 0xf) as u8;
                let g = ((v >> 4) & 0xf) as u8;
                let b = (v & 0xf) as u8;
                Some(Color::rgb8(r * 17, g * 17, b * 17))
            }
            _ => None,
        }
    }

    /// Delta between two colours in Lab (CIE76), for gamut and harmony tools.
    pub fn delta_e(self, other: Color) -> f32 {
        let [r1, g1, b1] = self.to_rgb_f32();
        let [r2, g2, b2] = other.to_rgb_f32();
        let (l1, a1, c1) = rgb_to_lab(r1, g1, b1);
        let (l2, a2, c2) = rgb_to_lab(r2, g2, b2);
        ((l1 - l2).powi(2) + (a1 - a2).powi(2) + (c1 - c2).powi(2)).sqrt()
    }

    /// Whether the colour is (naively) printable in CMYK: round trip error
    /// under the given tolerance. Used by the gamut alarm.
    pub fn in_cmyk_gamut(self, tolerance_delta_e: f32) -> bool {
        let back = self.convert_to("CMYK").convert_to("RGB");
        self.delta_e(back) <= tolerance_delta_e
    }
}

fn luminance(r: f32, g: f32, b: f32) -> f32 {
    let lin = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let y = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
    // Back to gamma so grey 50% looks like 50%.
    if y <= 0.0031308 {
        12.92 * y
    } else {
        1.055 * y.powf(1.0 / 2.4) - 0.055
    }
}

pub fn hsb_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i.rem_euclid(6) {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

pub fn rgb_to_hsb(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d <= 1e-6 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max <= 0.0 { 0.0 } else { d / max };
    (h.rem_euclid(360.0), s, max)
}

fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

pub fn rgb_to_lab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let (r, g, b) = (srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b));
    let x = (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047;
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let z = (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883;
    let f = |t: f32| {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
}

pub fn lab_to_rgb(l: f32, a: f32, b: f32) -> [f32; 3] {
    let fy = (l + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    let finv = |t: f32| {
        if t > 0.206893 {
            t * t * t
        } else {
            (t - 16.0 / 116.0) / 7.787
        }
    };
    let x = finv(fx) * 0.95047;
    let y = finv(fy);
    let z = finv(fz) * 1.08883;
    let r = 3.2406 * x - 1.5372 * y - 0.4986 * z;
    let g = -0.9689 * x + 1.8758 * y + 0.0415 * z;
    let bb = 0.0557 * x - 0.2040 * y + 1.0570 * z;
    [
        linear_to_srgb(r).clamp(0.0, 1.0),
        linear_to_srgb(g).clamp(0.0, 1.0),
        linear_to_srgb(bb).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmyk_to_rgb() {
        assert_eq!(Color::cmyk_pct(0.0, 0.0, 0.0, 100.0).to_rgb8(), [0, 0, 0]);
        assert_eq!(
            Color::cmyk_pct(100.0, 0.0, 0.0, 0.0).to_rgb8(),
            [0, 255, 255]
        );
        assert_eq!(Color::rgb8(255, 128, 0).to_hex(), "#ff8000");
    }

    #[test]
    fn round_trips_through_models() {
        let c = Color::rgb8(200, 60, 30);
        for model in ["CMYK", "HSB", "HSL", "Lab", "YIQ"] {
            let back = c.convert_to(model).to_rgb8();
            for (a, b) in back.iter().zip(c.to_rgb8()) {
                assert!((*a as i32 - b as i32).abs() <= 2, "{model}: {back:?}");
            }
        }
    }

    #[test]
    fn hex_parse() {
        assert_eq!(Color::from_hex("#ff8000").unwrap().to_rgb8(), [255, 128, 0]);
        assert_eq!(Color::from_hex("f80").unwrap().to_rgb8(), [255, 136, 0]);
        assert!(Color::from_hex("zz").is_none());
    }

    #[test]
    fn pure_red_is_in_cmyk_gamut_naively() {
        assert!(Color::rgb8(255, 0, 0).in_cmyk_gamut(2.0));
    }
}
