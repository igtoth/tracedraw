//! Lens docker state.

use tracedraw_core::{live::Lens, Color};

#[derive(Debug, Clone, PartialEq)]
pub struct LensSettings {
    pub kind: usize,
    pub rate: f64,
    pub color: Color,
    pub color_to: Color,
    pub amount: f64,
    pub frozen: bool,
    pub remove_face: bool,
}

impl Default for LensSettings {
    fn default() -> Self {
        LensSettings {
            kind: 0,
            rate: 50.0,
            color: Color::WHITE,
            color_to: Color::BLACK,
            amount: 2.0,
            frozen: false,
            remove_face: false,
        }
    }
}

pub const LENS_KEYS: [&str; 12] = [
    "lens.none",
    "lens.brighten",
    "lens.color_add",
    "lens.color_limit",
    "lens.custom_color_map",
    "lens.fish_eye",
    "lens.heat_map",
    "lens.invert",
    "lens.magnify",
    "lens.tinted_grayscale",
    "lens.transparency",
    "lens.wireframe",
];

impl LensSettings {
    pub fn to_lens(&self) -> Option<Lens> {
        Some(match self.kind {
            1 => Lens::Brighten { rate: self.rate },
            2 => Lens::ColorAdd {
                color: self.color,
                rate: self.rate,
            },
            3 => Lens::ColorLimit {
                color: self.color,
                rate: self.rate,
            },
            4 => Lens::CustomColorMap {
                from: self.color_to,
                to: self.color,
                direction: 0,
            },
            5 => Lens::FishEye {
                rate: self.rate * 10.0,
            },
            6 => Lens::HeatMap {
                rotation: self.rate,
            },
            7 => Lens::Invert,
            8 => Lens::Magnify {
                amount: self.amount,
            },
            9 => Lens::TintedGrayscale { color: self.color },
            10 => Lens::Transparency {
                rate: self.rate,
                color: self.color,
            },
            11 => Lens::Wireframe {
                outline: Color::BLACK,
                fill: self.color,
            },
            _ => return None,
        })
    }

    pub fn from_lens(l: &Lens) -> Self {
        let mut s = LensSettings::default();
        match l {
            Lens::Brighten { rate } => {
                s.kind = 1;
                s.rate = *rate;
            }
            Lens::ColorAdd { color, rate } => {
                s.kind = 2;
                s.color = *color;
                s.rate = *rate;
            }
            Lens::ColorLimit { color, rate } => {
                s.kind = 3;
                s.color = *color;
                s.rate = *rate;
            }
            Lens::CustomColorMap { from, to, .. } => {
                s.kind = 4;
                s.color_to = *from;
                s.color = *to;
            }
            Lens::FishEye { rate } => {
                s.kind = 5;
                s.rate = rate / 10.0;
            }
            Lens::HeatMap { rotation } => {
                s.kind = 6;
                s.rate = *rotation;
            }
            Lens::Invert => s.kind = 7,
            Lens::Magnify { amount } => {
                s.kind = 8;
                s.amount = *amount;
            }
            Lens::TintedGrayscale { color } => {
                s.kind = 9;
                s.color = *color;
            }
            Lens::Transparency { rate, color } => {
                s.kind = 10;
                s.rate = *rate;
                s.color = *color;
            }
            Lens::Wireframe { fill, .. } => {
                s.kind = 11;
                s.color = *fill;
            }
        }
        s
    }
}
