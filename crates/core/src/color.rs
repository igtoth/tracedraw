//! Colour values. Traco keeps the colour model the user chose (RGB or CMYK),
//! since print work in classic documents is CMYK-first. Conversion to
//! screen RGB is a naive formula for now; a proper ICC path comes later.

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
}

impl Color {
    pub const BLACK: Color = Color::Rgb { r: 0.0, g: 0.0, b: 0.0 };
    pub const WHITE: Color = Color::Rgb { r: 1.0, g: 1.0, b: 1.0 };

    pub fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Color::Rgb { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0 }
    }

    pub fn cmyk8(c: u8, m: u8, y: u8, k: u8) -> Self {
        Color::Cmyk { c: c as f32 / 255.0, m: m as f32 / 255.0, y: y as f32 / 255.0, k: k as f32 / 255.0 }
    }

    /// CMYK given in percent, the way the editor shows it.
    pub fn cmyk_pct(c: f32, m: f32, y: f32, k: f32) -> Self {
        Color::Cmyk { c: c / 100.0, m: m / 100.0, y: y / 100.0, k: k / 100.0 }
    }

    /// Screen colour as 8-bit sRGB. CMYK uses the simple multiplicative
    /// formula, which is what most non-managed viewers do.
    pub fn to_rgb8(self) -> [u8; 3] {
        let f = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        match self {
            Color::Rgb { r, g, b } => [f(r), f(g), f(b)],
            Color::Cmyk { c, m, y, k } => [f((1.0 - c) * (1.0 - k)), f((1.0 - m) * (1.0 - k)), f((1.0 - y) * (1.0 - k))],
            Color::Gray { v } => [f(v), f(v), f(v)],
        }
    }

    /// `#rrggbb` for SVG and CSS.
    pub fn to_hex(self) -> String {
        let [r, g, b] = self.to_rgb8();
        format!("#{r:02x}{g:02x}{b:02x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmyk_to_rgb() {
        assert_eq!(Color::cmyk_pct(0.0, 0.0, 0.0, 100.0).to_rgb8(), [0, 0, 0]);
        assert_eq!(Color::cmyk_pct(100.0, 0.0, 0.0, 0.0).to_rgb8(), [0, 255, 255]);
        assert_eq!(Color::rgb8(255, 128, 0).to_hex(), "#ff8000");
    }
}
