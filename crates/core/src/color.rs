//! Colour values. TraceDraw keeps the colour model the user chose, since
//! print work is CMYK-first; spot colours keep their name and a CMYK
//! fallback. CMYK shows through the built-in press model (`crate::press`,
//! fitted to the target design's default colour settings) and the other
//! models through the usual formulas, unless a colour engine is registered:
//! either a converter function through `engine::set`, or ICC profiles
//! through `engine::install` (see `crate::icc`).

use crate::icc::{Intent, Profile, Transform};
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
    /// CIE L*a*b*, L 0..100, a and b about -128..127. The built-in
    /// formulas use D65; the ICC engine treats it as ICC Lab (D50).
    Lab { l: f32, a: f32, b: f32 },
    /// NTSC YIQ, each component 0..1 (I and Q centred on 0.5).
    Yiq { y: f32, i: f32, q: f32 },
    /// Registration colour: prints on every separation.
    Registration,
}

/// Hooks for a colour engine. See `docs/decisions.md` D5.
///
/// Two levels: a converter function (`set`), kept for callers that bring
/// their own engine, and the built-in ICC engine (`install`) that holds the
/// working RGB and CMYK profiles. `Color::to_rgb8`, `convert_to` and
/// `in_cmyk_gamut` consult the converter first, then the ICC engine, then
/// fall back to the built-in conversions.
pub mod engine {
    use super::{Color, IccEngine};
    use std::sync::{Arc, OnceLock, RwLock};

    pub type Converter = fn(Color) -> [u8; 3];
    static ENGINE: OnceLock<Converter> = OnceLock::new();
    static ICC: RwLock<Option<Arc<IccEngine>>> = RwLock::new(None);

    /// Register a converter for CMYK and Lab screen colours (once).
    pub fn set(f: Converter) {
        let _ = ENGINE.set(f);
    }

    pub fn get() -> Option<Converter> {
        ENGINE.get().copied()
    }

    /// Install (or replace) the ICC engine used for screen and model
    /// conversions.
    pub fn install(engine: IccEngine) {
        let mut slot = ICC.write().unwrap_or_else(|e| e.into_inner());
        *slot = Some(Arc::new(engine));
    }

    /// Remove the ICC engine; conversions go back to the built-in ones.
    pub fn clear() {
        let mut slot = ICC.write().unwrap_or_else(|e| e.into_inner());
        *slot = None;
    }

    /// The installed ICC engine, if any.
    pub fn icc() -> Option<Arc<IccEngine>> {
        ICC.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// Profile-based colour engine. `Color::Rgb` values are taken to be in the
/// working RGB profile (sRGB when none is loaded), `Color::Cmyk` in the
/// CMYK profile and `Color::Lab` as ICC Lab (D50). The screen is the
/// built-in sRGB profile.
#[derive(Debug)]
pub struct IccEngine {
    display: Profile,
    rgb: Option<Profile>,
    cmyk: Option<Profile>,
    intent: Intent,
    bpc: bool,
    rgb_to_display: Option<Transform>,
    cmyk_to_display: Option<Transform>,
    display_to_cmyk: Option<Transform>,
    rgb_to_cmyk: Option<Transform>,
    cmyk_to_rgb: Option<Transform>,
}

impl IccEngine {
    /// Build an engine from the loaded working profiles. `rgb` is `None`
    /// for sRGB. Transforms that a profile cannot provide (for example a
    /// CMYK profile without a B2A table) are simply absent and the built-in
    /// conversion is used for that direction.
    pub fn new(
        rgb: Option<Profile>,
        cmyk: Option<Profile>,
        intent: Intent,
        bpc: bool,
    ) -> IccEngine {
        let display = Profile::srgb();
        let working = rgb.as_ref().unwrap_or(&display);
        let rgb_to_display = rgb
            .as_ref()
            .and_then(|p| Transform::new(p, &display, intent, bpc));
        let cmyk_to_display = cmyk
            .as_ref()
            .and_then(|p| Transform::new(p, &display, intent, bpc));
        let display_to_cmyk = cmyk
            .as_ref()
            .and_then(|p| Transform::new(&display, p, intent, bpc));
        let rgb_to_cmyk = cmyk
            .as_ref()
            .and_then(|p| Transform::new(working, p, intent, bpc));
        let cmyk_to_rgb = cmyk
            .as_ref()
            .and_then(|p| Transform::new(p, working, intent, bpc));
        IccEngine {
            display,
            rgb,
            cmyk,
            intent,
            bpc,
            rgb_to_display,
            cmyk_to_display,
            display_to_cmyk,
            rgb_to_cmyk,
            cmyk_to_rgb,
        }
    }

    pub fn intent(&self) -> Intent {
        self.intent
    }

    pub fn black_point_compensation(&self) -> bool {
        self.bpc
    }

    /// The working RGB profile, `None` for sRGB.
    pub fn rgb_profile(&self) -> Option<&Profile> {
        self.rgb.as_ref()
    }

    pub fn cmyk_profile(&self) -> Option<&Profile> {
        self.cmyk.as_ref()
    }

    /// The built-in sRGB screen profile.
    pub fn display_profile(&self) -> &Profile {
        &self.display
    }

    /// Screen (sRGB) colour, or `None` when no loaded profile applies to
    /// this colour model and the caller should fall back.
    pub fn to_rgb_f32(&self, color: Color) -> Option<[f32; 3]> {
        match color {
            Color::Rgb { r, g, b } => {
                let t = self.rgb_to_display.as_ref()?;
                first3(&t.transform(&[r, g, b]))
            }
            Color::Cmyk { c, m, y, k } => self.cmyk_to_display.as_ref()?.cmyk_to_rgb([c, m, y, k]),
            Color::Lab { l, a, b } => {
                let out = self.display.from_lab([l, a, b], self.intent)?;
                first3(&out)
            }
            _ => None,
        }
    }

    /// Working RGB to CMYK through the loaded profiles.
    pub fn rgb_to_cmyk(&self, rgb: [f32; 3]) -> Option<[f32; 4]> {
        self.rgb_to_cmyk.as_ref()?.rgb_to_cmyk(rgb)
    }

    /// Screen sRGB to CMYK through the loaded profiles.
    pub fn display_rgb_to_cmyk(&self, rgb: [f32; 3]) -> Option<[f32; 4]> {
        self.display_to_cmyk.as_ref()?.rgb_to_cmyk(rgb)
    }

    /// CMYK to working RGB through the loaded profiles.
    pub fn cmyk_to_rgb(&self, cmyk: [f32; 4]) -> Option<[f32; 3]> {
        self.cmyk_to_rgb.as_ref()?.cmyk_to_rgb(cmyk)
    }

    /// Whether a working RGB colour is reproducible by the CMYK profile.
    pub fn rgb_in_cmyk_gamut(&self, rgb: [f32; 3], tolerance_delta_e: f32) -> Option<bool> {
        Some(
            self.rgb_to_cmyk
                .as_ref()?
                .is_in_gamut_with(&rgb, tolerance_delta_e),
        )
    }
}

fn first3(v: &[f32]) -> Option<[f32; 3]> {
    Some([*v.first()?, *v.get(1)?, *v.get(2)?])
}

fn quantise(rgb: [f32; 3]) -> [u8; 3] {
    let f = |v: f32| {
        if v.is_nan() {
            0
        } else {
            (v.clamp(0.0, 1.0) * 255.0).round() as u8
        }
    };
    [f(rgb[0]), f(rgb[1]), f(rgb[2])]
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

    /// Screen colour as 8-bit sRGB. Uses the registered converter or ICC
    /// engine when there is one, else the built-in conversions.
    pub fn to_rgb8(self) -> [u8; 3] {
        if let Some(conv) = engine::get() {
            if matches!(self, Color::Cmyk { .. } | Color::Lab { .. }) {
                return conv(self);
            }
        }
        quantise(self.to_rgb_f32())
    }

    /// Screen colour as floats 0..1. RGB, CMYK and Lab go through the ICC
    /// engine when one is installed and has a profile for the model; every
    /// other case uses [`Color::to_rgb_f32_builtin`].
    pub fn to_rgb_f32(self) -> [f32; 3] {
        if matches!(
            self,
            Color::Rgb { .. } | Color::Cmyk { .. } | Color::Lab { .. }
        ) {
            if let Some(rgb) = engine::icc().and_then(|e| e.to_rgb_f32(self)) {
                return rgb;
            }
        }
        self.to_rgb_f32_builtin()
    }

    /// Screen colour as floats 0..1 with the built-in conversions (the
    /// press model for CMYK), ignoring any colour engine.
    pub fn to_rgb_f32_builtin(self) -> [f32; 3] {
        match self {
            Color::Rgb { r, g, b } => [r, g, b],
            Color::Cmyk { c, m, y, k } => crate::press::cmyk_to_srgb([c, m, y, k]),
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

    /// Convert to another model by way of sRGB. RGB to CMYK and back use
    /// the ICC engine when one is installed with a CMYK profile, else the
    /// press model; the other models use the built-in formulas. A colour
    /// already in the asked model comes back unchanged.
    pub fn convert_to(self, model: &str) -> Color {
        if self.model_name() == model {
            return self;
        }
        if let Some(icc) = engine::icc() {
            let managed = match (self, model) {
                (Color::Rgb { r, g, b }, "CMYK") => icc
                    .rgb_to_cmyk([r, g, b])
                    .map(|[c, m, y, k]| Color::Cmyk { c, m, y, k }),
                (Color::Cmyk { .. }, "CMYK") => Some(self),
                (_, "CMYK") => icc
                    .display_rgb_to_cmyk(self.to_rgb_f32())
                    .map(|[c, m, y, k]| Color::Cmyk { c, m, y, k }),
                (Color::Cmyk { c, m, y, k }, "RGB") => icc
                    .cmyk_to_rgb([c, m, y, k])
                    .map(|[r, g, b]| Color::Rgb { r, g, b }),
                _ => None,
            };
            if let Some(c) = managed {
                return c;
            }
        }
        let [r, g, b] = self.to_rgb_f32();
        let (r, g, b) = (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
        match model {
            "RGB" => Color::Rgb { r, g, b },
            "CMYK" => {
                let [c, m, y, k] = crate::press::srgb_to_cmyk([r, g, b]);
                Color::Cmyk { c, m, y, k }
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

    /// Whether the colour is printable in CMYK: round trip error under the
    /// given tolerance. Uses the CMYK profile of the ICC engine when one is
    /// installed, else the press model. Used by the gamut alarm.
    pub fn in_cmyk_gamut(self, tolerance_delta_e: f32) -> bool {
        if let Some(icc) = engine::icc() {
            let rgb = match self {
                Color::Rgb { r, g, b } => [r, g, b],
                _ => self.to_rgb_f32(),
            };
            if let Some(ok) = icc.rgb_in_cmyk_gamut(rgb, tolerance_delta_e) {
                return ok;
            }
        }
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
    fn cmyk_shows_like_the_reference_editor() {
        // Process black is the warm dark grey of coated stock, not 0 0 0.
        let black = Color::cmyk_pct(0.0, 0.0, 0.0, 100.0).to_rgb8();
        for (a, b) in black.iter().zip([34u8, 31, 32]) {
            assert!(a.abs_diff(b) <= 2, "{black:?}");
        }
        assert_eq!(
            Color::cmyk_pct(0.0, 0.0, 0.0, 0.0).to_rgb8(),
            [255, 255, 255]
        );
        let cyan = Color::cmyk_pct(100.0, 0.0, 0.0, 0.0).to_rgb8();
        assert!(cyan[0] == 0 && (171..=176).contains(&cyan[1]), "{cyan:?}");
        assert_eq!(Color::rgb8(255, 128, 0).to_hex(), "#ff8000");
    }

    #[test]
    fn round_trips_through_models() {
        let c = Color::rgb8(200, 60, 30);
        for model in ["HSB", "HSL", "Lab", "YIQ"] {
            let back = c.convert_to(model).to_rgb8();
            for (a, b) in back.iter().zip(c.to_rgb8()) {
                assert!((*a as i32 - b as i32).abs() <= 2, "{model}: {back:?}");
            }
        }
        // CMYK keeps colours the press can print.
        for c in [
            Color::rgb8(190, 70, 50),
            Color::rgb8(140, 190, 160),
            Color::rgb8(100, 50, 20),
        ] {
            let back = c.convert_to("CMYK").to_rgb8();
            for (a, b) in back.iter().zip(c.to_rgb8()) {
                assert!((*a as i32 - b as i32).abs() <= 1, "{c:?}: {back:?}");
            }
        }
    }

    #[test]
    fn converting_to_the_same_model_keeps_the_colour() {
        let k = Color::cmyk_pct(0.0, 0.0, 0.0, 100.0);
        assert_eq!(k.convert_to("CMYK"), k);
        let rich = Color::cmyk_pct(60.0, 40.0, 40.0, 100.0);
        assert_eq!(rich.convert_to("CMYK"), rich);
        let rgb = Color::rgb8(1, 2, 3);
        assert_eq!(rgb.convert_to("RGB"), rgb);
    }

    #[test]
    fn black_and_greys_convert_to_black_ink() {
        assert_eq!(
            Color::rgb8(0, 0, 0).convert_to("CMYK"),
            Color::cmyk_pct(0.0, 0.0, 0.0, 100.0)
        );
        let Color::Cmyk { c, m, y, k } = Color::Gray { v: 0.5 }.convert_to("CMYK") else {
            panic!("not CMYK");
        };
        assert_eq!((c, m, y), (0.0, 0.0, 0.0));
        assert!(k > 0.5 && k < 0.7, "{k}");
    }

    #[test]
    fn hex_parse() {
        assert_eq!(Color::from_hex("#ff8000").unwrap().to_rgb8(), [255, 128, 0]);
        assert_eq!(Color::from_hex("f80").unwrap().to_rgb8(), [255, 136, 0]);
        assert!(Color::from_hex("zz").is_none());
    }

    #[test]
    fn screen_red_is_outside_the_press_gamut() {
        assert!(!Color::rgb8(255, 0, 0).in_cmyk_gamut(2.0));
        assert!(!Color::rgb8(0, 0, 255).in_cmyk_gamut(2.0));
        // The red of solid magenta and yellow prints.
        let press_red = Color::cmyk_pct(0.0, 100.0, 100.0, 0.0).to_rgb8();
        let [r, g, b] = press_red;
        assert!(Color::rgb8(r, g, b).in_cmyk_gamut(2.0));
    }

    // The ICC engine is exercised as a value here, never installed in the
    // process-wide slot, so the built-in expectations above stay valid when
    // tests run in parallel.

    #[test]
    fn icc_engine_without_profiles_declines() {
        let e = IccEngine::new(None, None, Intent::RelativeColorimetric, true);
        assert!(e.to_rgb_f32(Color::rgb8(10, 20, 30)).is_none());
        assert!(e
            .to_rgb_f32(Color::cmyk_pct(0.0, 0.0, 0.0, 100.0))
            .is_none());
        assert!(e.rgb_to_cmyk([1.0, 0.0, 0.0]).is_none());
        assert!(e.rgb_in_cmyk_gamut([1.0, 0.0, 0.0], 2.0).is_none());
        // Lab always goes through the built-in display profile.
        let white = e
            .to_rgb_f32(Color::Lab {
                l: 100.0,
                a: 0.0,
                b: 0.0,
            })
            .unwrap();
        assert_eq!(quantise(white), [255, 255, 255]);
        let black = e
            .to_rgb_f32(Color::Lab {
                l: 0.0,
                a: 0.0,
                b: 0.0,
            })
            .unwrap();
        assert_eq!(quantise(black), [0, 0, 0]);
        assert!(e.rgb_profile().is_none());
        assert!(e.cmyk_profile().is_none());
        assert_eq!(e.display_profile().description(), Some("sRGB (built-in)"));
    }

    #[test]
    fn icc_engine_converts_cmyk_through_profile() {
        use crate::icc::test_profiles::{lut16_cmyk_profile, mba_cmyk_profile};
        // A2B only: screen colours work, RGB to CMYK does not.
        let cmyk = Profile::parse(&lut16_cmyk_profile()).unwrap();
        let e = IccEngine::new(None, Some(cmyk), Intent::RelativeColorimetric, true);
        let paper = e.to_rgb_f32(Color::cmyk_pct(0.0, 0.0, 0.0, 0.0)).unwrap();
        assert_eq!(quantise(paper), [255, 255, 255]);
        let ink = e
            .to_rgb_f32(Color::cmyk_pct(100.0, 100.0, 100.0, 100.0))
            .unwrap();
        assert_eq!(quantise(ink), [0, 0, 0]);
        let cyan = quantise(e.to_rgb_f32(Color::cmyk_pct(100.0, 0.0, 0.0, 0.0)).unwrap());
        assert!(cyan[0] < cyan[2], "{cyan:?}");
        assert!(e.rgb_to_cmyk([1.0, 1.0, 1.0]).is_none());
        assert!(e.cmyk_to_rgb([0.0; 4]).is_some());

        // A2B and B2A: both directions and the gamut check.
        let cmyk = Profile::parse(&mba_cmyk_profile()).unwrap();
        let e = IccEngine::new(None, Some(cmyk), Intent::RelativeColorimetric, false);
        let k = e.rgb_to_cmyk([0.0, 0.0, 0.0]).unwrap();
        assert!((k[3] - 1.0).abs() < 2e-3, "{k:?}");
        let k = e.display_rgb_to_cmyk([1.0, 1.0, 1.0]).unwrap();
        assert!(k[3].abs() < 2e-3, "{k:?}");
        assert_eq!(e.rgb_in_cmyk_gamut([1.0, 0.0, 0.0], 2.0), Some(false));
        assert_eq!(e.rgb_in_cmyk_gamut([1.0, 0.0, 0.0], 1000.0), Some(true));
        assert!(!e.black_point_compensation());
        assert_eq!(e.intent(), Intent::RelativeColorimetric);
    }

    #[test]
    fn icc_engine_working_rgb_maps_to_screen() {
        use crate::icc::test_profiles::{matrix_trc_profile, para_srgb};
        let srgb_like =
            Profile::parse(&matrix_trc_profile(para_srgb(), para_srgb(), para_srgb())).unwrap();
        let e = IccEngine::new(Some(srgb_like), None, Intent::Perceptual, true);
        let c = Color::rgb8(200, 60, 30);
        let screen = quantise(e.to_rgb_f32(c).unwrap());
        for (a, b) in screen.iter().zip([200u8, 60, 30]) {
            assert!((*a as i32 - b as i32).abs() <= 1, "{screen:?}");
        }
        assert!(e.rgb_profile().is_some());
    }
}
