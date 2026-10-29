//! Settings held by the effect dockers (Lens, Bevel, Transparency)
//! before they are applied as live effects.

use tracedraw_core::{live::MergeMode, Color, Fill};

#[derive(Debug, Clone, PartialEq)]
pub struct BevelSettings {
    pub distance: f64,
    pub light_angle: f64,
    pub intensity: f64,
    pub style: u8,
    pub shadow_color: Color,
    pub light_color: Color,
}

impl Default for BevelSettings {
    fn default() -> Self {
        BevelSettings {
            distance: 2.0,
            light_angle: 135.0,
            intensity: 50.0,
            style: 0,
            shadow_color: Color::BLACK,
            light_color: Color::WHITE,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransparencySettings {
    /// 0 uniform, 1 fountain, 2 pattern, 3 texture.
    pub kind: u8,
    pub amount: f64,
    pub mask: Fill,
    pub merge: MergeMode,
    pub target: u8,
}

impl TransparencySettings {
    /// Default mask for a transparency kind, at `angle` degrees for fountains.
    pub fn default_mask(kind: u8, angle: f64) -> Fill {
        use tracedraw_core::style::{Pattern, PatternTile, Texture, TextureKind};
        match kind {
            2 => Fill::Pattern(Pattern::TwoColor {
                tile: PatternTile::Checker,
                front: Color::WHITE,
                back: Color::BLACK,
                size_mm: 10.0,
                placement: Default::default(),
            }),
            3 => Fill::Texture(Texture {
                kind: TextureKind::Clouds,
                color_a: Color::WHITE,
                color_b: Color::BLACK,
                scale: 20.0,
                seed: 1,
            }),
            _ => Fill::linear(Color::WHITE, Color::BLACK, angle),
        }
    }

    /// Change the kind, resetting the mask to that kind's default.
    pub fn set_kind(&mut self, kind: u8) {
        if self.kind != kind {
            self.kind = kind;
            self.mask = Self::default_mask(kind, 0.0);
        }
    }
}

impl Default for TransparencySettings {
    fn default() -> Self {
        TransparencySettings {
            kind: 0,
            amount: 50.0,
            mask: Fill::linear(Color::WHITE, Color::BLACK, 0.0),
            merge: MergeMode::Normal,
            target: 2,
        }
    }
}
