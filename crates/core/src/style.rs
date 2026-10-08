//! Fill and outline (stroke) properties of a shape.

use crate::color::Color;
use crate::geometry::Point;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Fill {
    None,
    Solid(Color),
    /// Two-stop linear gradient; `angle` in degrees, the editor convention
    /// (0 = left to right, counter-clockwise positive).
    Linear {
        from: Color,
        to: Color,
        angle: f64,
    },
    /// Two-stop radial gradient centred on the shape bounds; `offset` moves
    /// the centre in bounds-relative units (-1..1).
    Radial {
        from: Color,
        to: Color,
        offset: Point,
    },
}

impl Default for Fill {
    fn default() -> Self {
        Fill::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub color: Color,
    /// Width in millimetres. the "hairline" is 0.0762 mm (0.216 pt).
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    /// Dash pattern as alternating on/off lengths, in multiples of the width
    /// (the editor convention). Empty = solid.
    pub dash: Vec<f64>,
    /// When true, the outline scales with the object (the editor's
    /// "Scale with object"). When false, width stays fixed under transforms.
    pub scale_with_object: bool,
    /// Draw the outline behind the fill.
    pub behind_fill: bool,
}

impl Stroke {
    pub const HAIRLINE: f64 = 0.0762;

    pub fn hairline(color: Color) -> Self {
        Stroke {
            color,
            width: Self::HAIRLINE,
            ..Default::default()
        }
    }

    pub fn new(color: Color, width: f64) -> Self {
        Stroke {
            color,
            width,
            ..Default::default()
        }
    }
}

impl Default for Stroke {
    fn default() -> Self {
        Stroke {
            color: Color::BLACK,
            width: Self::HAIRLINE,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash: Vec::new(),
            scale_with_object: false,
            behind_fill: false,
        }
    }
}
