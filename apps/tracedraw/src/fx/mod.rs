//! The bitmap effects of the Effects menu, grouped
//! groups them, each with its own settings. A bitmap keeps its effects
//! apart from its pixels (the Properties docker's FX tab lists them): the
//! effects can be edited, hidden, reordered or removed, and the bitmap
//! shows their result. Effects that spread past the edges grow the
//! bitmap first when Auto inflate is on.

pub mod adjust;
pub mod art;
pub mod blur;
pub mod camera;
pub mod color;
pub mod creative;
pub mod distort;
pub mod noise;
pub mod texture;
pub mod util;

use image::RgbaImage;
use std::collections::BTreeMap;
use tracedraw_core::{BitmapEffect, BitmapFxStack};

/// What kind of control a setting has.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamKind {
    /// A slider from `min` to `max`; `step` 1 makes it whole numbers.
    Range {
        min: f64,
        max: f64,
        default: f64,
        step: f64,
    },
    /// A direction, degrees counter-clockwise from the right.
    Angle {
        default: f64,
    },
    /// One of several choices (their i18n keys are `fxo.<option>`).
    Choice {
        options: &'static [&'static str],
        default: usize,
    },
    Check {
        default: bool,
    },
    /// 0xRRGGBB.
    Color {
        default: u32,
    },
}

/// One setting: its stored name (and i18n key `fxp.<name>`) and control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamSpec {
    pub name: &'static str,
    pub kind: ParamKind,
}

pub const fn range(name: &'static str, min: f64, max: f64, default: f64, step: f64) -> ParamSpec {
    ParamSpec {
        name,
        kind: ParamKind::Range {
            min,
            max,
            default,
            step,
        },
    }
}

pub const fn angle(name: &'static str, default: f64) -> ParamSpec {
    ParamSpec {
        name,
        kind: ParamKind::Angle { default },
    }
}

pub const fn choice(
    name: &'static str,
    options: &'static [&'static str],
    default: usize,
) -> ParamSpec {
    ParamSpec {
        name,
        kind: ParamKind::Choice { options, default },
    }
}

pub const fn check(name: &'static str, default: bool) -> ParamSpec {
    ParamSpec {
        name,
        kind: ParamKind::Check { default },
    }
}

pub const fn color(name: &'static str, default: u32) -> ParamSpec {
    ParamSpec {
        name,
        kind: ParamKind::Color { default },
    }
}

/// An effect: its stable id (and i18n key `fx.<id>`), its settings, what
/// it does and how far past the edges it reaches (pixels) with them.
pub struct EffectSpec {
    pub id: &'static str,
    pub params: &'static [ParamSpec],
    pub apply: fn(&RgbaImage, &P) -> RgbaImage,
    pub reach: fn(&P, u32, u32) -> u32,
}

/// Effects that stay inside the image.
pub fn no_reach(_: &P, _: u32, _: u32) -> u32 {
    0
}

/// An effect's settings, falling back to its defaults and kept in range.
pub struct P<'a> {
    pub spec: &'a EffectSpec,
    pub values: &'a BTreeMap<String, f64>,
}

impl P<'_> {
    fn param(&self, name: &str) -> Option<&ParamSpec> {
        self.spec.params.iter().find(|p| p.name == name)
    }

    /// A number setting (sliders, angles, check boxes as 0 or 1).
    pub fn f(&self, name: &str) -> f64 {
        let Some(p) = self.param(name) else {
            return 0.0;
        };
        let v = self.values.get(name).copied().filter(|v| v.is_finite());
        match p.kind {
            ParamKind::Range {
                min,
                max,
                default,
                step,
            } => {
                let v = v.unwrap_or(default).clamp(min.min(max), max.max(min));
                if step >= 1.0 {
                    v.round()
                } else {
                    v
                }
            }
            ParamKind::Angle { default } => v.unwrap_or(default),
            ParamKind::Choice { options, default } => {
                (v.map(|v| v.round().max(0.0) as usize).unwrap_or(default))
                    .min(options.len().saturating_sub(1)) as f64
            }
            ParamKind::Check { default } => {
                if v.map(|v| v != 0.0).unwrap_or(default) {
                    1.0
                } else {
                    0.0
                }
            }
            ParamKind::Color { default } => {
                v.unwrap_or(default as f64).clamp(0.0, 16_777_215.0).round()
            }
        }
    }

    pub fn f32(&self, name: &str) -> f32 {
        self.f(name) as f32
    }

    pub fn u(&self, name: &str) -> u32 {
        self.f(name).max(0.0).round() as u32
    }

    /// A choice's index.
    pub fn i(&self, name: &str) -> usize {
        self.f(name) as usize
    }

    pub fn b(&self, name: &str) -> bool {
        self.f(name) != 0.0
    }

    pub fn rgb(&self, name: &str) -> [u8; 3] {
        util::rgb(self.f(name) as u32)
    }
}

/// Every effect, in menu order within its group.
pub fn catalog() -> impl Iterator<Item = &'static EffectSpec> {
    adjust::EFFECTS
        .iter()
        .chain(adjust::TRANSFORM.iter())
        .chain(adjust::CORRECTION.iter())
        .chain(texture::THREE_D.iter())
        .chain(art::EFFECTS.iter())
        .chain(blur::EFFECTS.iter())
        .chain(camera::EFFECTS.iter())
        .chain(color::TRANSFORM.iter())
        .chain(color::CONTOUR.iter())
        .chain(creative::EFFECTS.iter())
        .chain(texture::CUSTOM.iter())
        .chain(distort::EFFECTS.iter())
        .chain(noise::NOISE.iter())
        .chain(noise::SHARPEN.iter())
        .chain(texture::TEXTURE.iter())
}

/// The Effects menu's bitmap groups: (i18n key, effects).
pub fn groups() -> Vec<(&'static str, &'static [EffectSpec])> {
    vec![
        ("menu.effects.adjust", adjust::EFFECTS),
        ("menu.effects.transform", adjust::TRANSFORM),
        ("menu.effects.correction", adjust::CORRECTION),
        ("menu.effects.3d", texture::THREE_D),
        ("menu.effects.art_strokes", art::EFFECTS),
        ("menu.effects.blur", blur::EFFECTS),
        ("menu.effects.camera", camera::EFFECTS),
        ("menu.effects.color_transform", color::TRANSFORM),
        ("menu.effects.contour_group", color::CONTOUR),
        ("menu.effects.creative", creative::EFFECTS),
        ("menu.effects.custom", texture::CUSTOM),
        ("menu.effects.distort", distort::EFFECTS),
        ("menu.effects.noise", noise::NOISE),
        ("menu.effects.sharpen", noise::SHARPEN),
        ("menu.effects.texture", texture::TEXTURE),
    ]
}

pub fn spec(id: &str) -> Option<&'static EffectSpec> {
    catalog().find(|s| s.id == id)
}

/// A new effect with its default settings.
pub fn new_effect(id: &str) -> BitmapEffect {
    BitmapEffect {
        id: id.to_string(),
        params: BTreeMap::new(),
        visible: true,
    }
}

/// Run one effect (unknown ones leave the image as it is).
pub fn apply_effect(img: &RgbaImage, e: &BitmapEffect) -> RgbaImage {
    match spec(&e.id) {
        Some(s) => {
            let p = P {
                spec: s,
                values: &e.params,
            };
            let out = (s.apply)(img, &p);
            if out.dimensions() == img.dimensions() {
                out
            } else {
                img.clone()
            }
        }
        None => img.clone(),
    }
}

/// Settings measured in pixels: a copy of the bitmap at another scale
/// (a preview) runs them scaled so it looks like the result.
const PIXEL_PARAMS: [&str; 16] = [
    "radius",
    "distance",
    "size",
    "width",
    "height",
    "blade_size",
    "stroke_size",
    "brush_size",
    "max_dot_radius",
    "period",
    "amplitude",
    "spacing",
    "smear_length",
    "solder_width",
    "depth",
    "strength",
];

/// The effect as a copy at `scale` runs it: pixel sizes scaled, never
/// below their minimum.
pub fn scaled(e: &BitmapEffect, scale: f32) -> BitmapEffect {
    let Some(spec) = spec(&e.id) else {
        return e.clone();
    };
    let mut out = e.clone();
    for p in spec.params {
        if let ParamKind::Range { min, default, .. } = p.kind {
            if PIXEL_PARAMS.contains(&p.name) {
                let v = e.params.get(p.name).copied().unwrap_or(default);
                out.params
                    .insert(p.name.to_string(), (v * scale as f64).max(min));
            }
        }
    }
    out
}

/// How far an effect reaches past the image's edges, pixels.
pub fn reach(e: &BitmapEffect, w: u32, h: u32) -> u32 {
    match spec(&e.id) {
        Some(s) => {
            let p = P {
                spec: s,
                values: &e.params,
            };
            (s.reach)(&p, w, h).min(w.max(h))
        }
        None => 0,
    }
}

/// The result of a bitmap's effects: the image and its place. With
/// `inflate` the original grows by what the visible effects reach first.
pub fn render_stack(
    stack: &BitmapFxStack,
    inflate: bool,
) -> Option<(RgbaImage, tracedraw_core::Rect)> {
    let mut img = crate::bitmap_fx::decode(&stack.png)?;
    let (w, h) = img.dimensions();
    let pad: u32 = if inflate {
        stack
            .effects
            .iter()
            .filter(|e| e.visible)
            .map(|e| reach(e, w, h))
            .sum::<u32>()
            .min(w.max(h))
    } else {
        0
    };
    let mut rect = stack.rect;
    if pad > 0 {
        img = crate::bitmap_fx::inflate(&img, pad);
        let mx = stack.rect.width() / w.max(1) as f64 * pad as f64;
        let my = stack.rect.height() / h.max(1) as f64 * pad as f64;
        rect = tracedraw_core::Rect::new(rect.x0 - mx, rect.y0 - my, rect.x1 + mx, rect.y1 + my);
    }
    for e in stack.effects.iter().filter(|e| e.visible) {
        img = apply_effect(&img, e);
    }
    Some((img, rect))
}

impl crate::app::App {
    /// Change the effects of the bitmap `id` (its original kept aside);
    /// returns the command, or `None` when it is not a bitmap.
    pub fn bitmap_fx_command(
        &self,
        id: tracedraw_core::ShapeId,
        f: impl FnOnce(&mut Vec<BitmapEffect>),
    ) -> Option<tracedraw_core::Command> {
        use tracedraw_core::document::ShapeKind;
        let s = self.doc().find_shape(id)?;
        let ShapeKind::Bitmap {
            rect,
            width_px,
            height_px,
            png,
            fx,
        } = &s.kind
        else {
            return None;
        };
        let mut stack = match fx {
            Some(st) => (**st).clone(),
            None => BitmapFxStack {
                rect: *rect,
                width_px: *width_px,
                height_px: *height_px,
                png: png.clone(),
                effects: Vec::new(),
            },
        };
        f(&mut stack.effects);
        let kind = if stack.effects.is_empty() {
            ShapeKind::Bitmap {
                rect: stack.rect,
                width_px: stack.width_px,
                height_px: stack.height_px,
                png: stack.png,
                fx: None,
            }
        } else {
            let (img, r) = render_stack(&stack, self.doc().metadata.auto_inflate_bitmaps())?;
            let png = crate::bitmap_fx::encode(&img)?;
            ShapeKind::Bitmap {
                rect: r,
                width_px: img.width(),
                height_px: img.height(),
                png,
                fx: Some(Box::new(stack)),
            }
        };
        Some(tracedraw_core::Command::SetShapeKind { shape: id, kind })
    }

    /// Add an effect to every selected bitmap, in one undo step.
    pub fn add_bitmap_effect(&mut self, e: BitmapEffect) {
        let label = "Add Effect";
        let ids: Vec<_> = self.selection.clone();
        let cmds: Vec<_> = ids
            .into_iter()
            .filter_map(|id| self.bitmap_fx_command(id, |list| list.push(e.clone())))
            .collect();
        if cmds.is_empty() {
            return;
        }
        if let Err(err) = self.engine.run_batch(label, &cmds) {
            self.status = err.to_string();
        }
    }

    /// Change the effect list of one bitmap, in one undo step.
    pub fn edit_bitmap_effects(
        &mut self,
        id: tracedraw_core::ShapeId,
        label: &'static str,
        f: impl FnOnce(&mut Vec<BitmapEffect>),
    ) {
        if let Some(cmd) = self.bitmap_fx_command(id, f) {
            if let Err(err) = self.engine.run_with_label(&cmd, label) {
                self.status = err.to_string();
            }
        }
    }

    /// Bitmaps > Inflate Bitmap > Auto Inflate Bitmap, the same switch as
    /// Document Options > General: effects rendered from now on that
    /// spread past the edges grow the bitmap first.
    pub fn set_auto_inflate_bitmaps(&mut self, on: bool) {
        let mut metadata = self.doc().metadata.clone();
        metadata.no_auto_inflate = !on;
        if metadata != self.doc().metadata {
            self.run(tracedraw_core::Command::SetMetadata { metadata });
        }
    }

    /// The effects of a bitmap.
    pub fn bitmap_effects(&self, id: tracedraw_core::ShapeId) -> Vec<BitmapEffect> {
        self.bitmap_fx_stack(id)
            .map(|s| s.effects)
            .unwrap_or_default()
    }

    /// A bitmap's original pixels and effects, when it has effects.
    pub fn bitmap_fx_stack(&self, id: tracedraw_core::ShapeId) -> Option<BitmapFxStack> {
        use tracedraw_core::document::ShapeKind;
        match self.doc().find_shape(id).map(|s| &s.kind) {
            Some(ShapeKind::Bitmap { fx: Some(st), .. }) => Some((**st).clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn img() -> RgbaImage {
        ImageBuffer::from_fn(24, 20, |x, y| {
            Rgba([(x * 10) as u8, (y * 12) as u8, ((x + y) * 5) as u8, 255])
        })
    }

    #[test]
    fn every_effect_runs_keeps_size_and_has_names() {
        let src = img();
        let mut ids = std::collections::BTreeSet::new();
        for s in catalog() {
            assert!(ids.insert(s.id), "duplicate {}", s.id);
            let e = new_effect(s.id);
            let out = apply_effect(&src, &e);
            assert_eq!(out.dimensions(), src.dimensions(), "{}", s.id);
            // Extreme settings do not break it either.
            let mut hi = e.clone();
            for p in s.params {
                let v = match p.kind {
                    ParamKind::Range { max, .. } => max,
                    ParamKind::Angle { .. } => 359.0,
                    ParamKind::Choice { options, .. } => (options.len() - 1) as f64,
                    ParamKind::Check { .. } => 1.0,
                    ParamKind::Color { .. } => 16_777_215.0,
                };
                hi.params.insert(p.name.to_string(), v);
            }
            assert_eq!(
                apply_effect(&src, &hi).dimensions(),
                src.dimensions(),
                "{}",
                s.id
            );
            let mut lo = e.clone();
            for p in s.params {
                let v = match p.kind {
                    ParamKind::Range { min, .. } => min,
                    _ => 0.0,
                };
                lo.params.insert(p.name.to_string(), v);
            }
            assert_eq!(
                apply_effect(&src, &lo).dimensions(),
                src.dimensions(),
                "{}",
                s.id
            );
            assert!(
                crate::i18n::has_english(&format!("fx.{}", s.id)),
                "fx.{}",
                s.id
            );
            for p in s.params {
                assert!(
                    crate::i18n::has_english(&format!("fxp.{}", p.name)),
                    "fxp.{}",
                    p.name
                );
                if let ParamKind::Choice { options, .. } = p.kind {
                    for o in options {
                        assert!(crate::i18n::has_english(&format!("fxo.{o}")), "fxo.{o}");
                    }
                }
            }
        }
        assert!(ids.len() >= 80, "{}", ids.len());
        // Every effect is in a group.
        let grouped: usize = groups().iter().map(|g| g.1.len()).sum();
        assert_eq!(grouped, ids.len());
    }

    /// Largest channel difference between two images of one size.
    fn diff(a: &RgbaImage, b: &RgbaImage) -> u8 {
        a.pixels()
            .zip(b.pixels())
            .flat_map(|(p, q)| (0..4).map(move |c| p[c].abs_diff(q[c])))
            .max()
            .unwrap_or(0)
    }

    fn variance(i: &RgbaImage) -> f32 {
        let v: Vec<f32> = i.pixels().map(util::luma).collect();
        let m = v.iter().sum::<f32>() / v.len() as f32;
        v.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / v.len() as f32
    }

    fn run(src: &RgbaImage, id: &str, set: &[(&str, f64)]) -> RgbaImage {
        let mut e = new_effect(id);
        for (k, v) in set {
            e.params.insert(k.to_string(), *v);
        }
        apply_effect(src, &e)
    }

    #[test]
    fn neutral_settings_leave_the_image_alone() {
        let src = img();
        for id in [
            "brightness_contrast_intensity",
            "color_balance",
            "gamma",
            "hue_saturation_lightness",
            "selective_color",
            "channel_mixer",
            "image_adjustments",
            "tone_curve",
            "contrast_enhancement",
        ] {
            let out = run(&src, id, &[]);
            assert!(diff(&src, &out) <= 1, "{id}: {}", diff(&src, &out));
        }
    }

    #[test]
    fn effects_do_what_their_names_say() {
        let src = img();
        // Inverting twice gives the image back.
        let inv = run(&src, "invert_colors", &[]);
        assert_eq!(run(&inv, "invert_colors", &[]), src);
        // Desaturate leaves grays; posterize to two levels leaves 0 and 255.
        assert!(run(&src, "desaturate", &[])
            .pixels()
            .all(|p| p[0] == p[1] && p[1] == p[2]));
        let two = run(&src, "posterize", &[("level", 2.0)]);
        assert!(two
            .pixels()
            .all(|p| (0..3).all(|c| p[c] == 0 || p[c] == 255)));
        // More brightness never darkens.
        let bright = run(
            &src,
            "brightness_contrast_intensity",
            &[("brightness", 50.0)],
        );
        assert!(bright
            .pixels()
            .zip(src.pixels())
            .all(|(a, b)| util::luma(a) + 0.5 >= util::luma(b)));
        assert!(diff(&bright, &src) > 20);
        // Blurs lower the contrast of a checkerboard, sharpening raises it.
        let checker: RgbaImage = ImageBuffer::from_fn(16, 16, |x, y| {
            if (x / 2 + y / 2) % 2 == 0 {
                Rgba([200, 200, 200, 255])
            } else {
                Rgba([60, 60, 60, 255])
            }
        });
        let v0 = variance(&checker);
        assert!(variance(&run(&checker, "gaussian_blur", &[("radius", 2.0)])) < v0 * 0.5);
        assert!(variance(&run(&checker, "unsharp_mask", &[("percentage", 200.0)])) > v0);
        // The median removes a lone speck.
        let mut speck: RgbaImage = ImageBuffer::from_pixel(9, 9, Rgba([100, 100, 100, 255]));
        speck.put_pixel(4, 4, Rgba([255, 255, 255, 255]));
        assert_eq!(run(&speck, "median", &[]).get_pixel(4, 4)[0], 100);
        // Colour changes keep transparency.
        let mut half = src.clone();
        for p in half.pixels_mut() {
            p[3] = 128;
        }
        for id in [
            "sepia_toning",
            "colorize",
            "hue_saturation_lightness",
            "tone_curve",
        ] {
            assert!(run(&half, id, &[]).pixels().all(|p| p[3] == 128), "{id}");
        }
        // A tone curve through (0, 255) and (255, 0) inverts.
        let mut e = new_effect("tone_curve");
        adjust::curve_into(
            &mut e.params,
            "rgb",
            &crate::bitmap_modes::ToneCurve::line(255.0, 0.0),
        );
        assert!(diff(&apply_effect(&src, &e), &inv) <= 1);
    }

    #[test]
    fn blurs_spread_colour_not_black_into_transparency() {
        let spot: RgbaImage = ImageBuffer::from_fn(30, 30, |x, y| {
            if (10..20).contains(&x) && (10..20).contains(&y) {
                Rgba([250, 40, 40, 255])
            } else {
                Rgba([0, 0, 0, 0])
            }
        });
        let cases: [(&str, &[(&str, f64)]); 6] = [
            ("gaussian_blur", &[("radius", 6.0)]),
            ("motion_blur", &[("distance", 12.0), ("direction", 30.0)]),
            ("low_pass", &[("radius", 4.0), ("percentage", 100.0)]),
            ("jagged_edges", &[("width", 3.0), ("height", 3.0)]),
            ("radial_blur", &[("amount", 60.0)]),
            ("zoom", &[("amount", 60.0)]),
        ];
        for (id, set) in cases {
            let out = run(&spot, id, set);
            assert!(out.pixels().any(|p| p[3] > 0 && p[3] < 255), "{id} spreads");
            for p in out.pixels().filter(|p| p[3] > 16) {
                assert!(p[0] > 225 && p[1] < 70, "{id}: {p:?}");
            }
        }
    }

    #[test]
    fn tone_curve_styles_gamma_freehand_and_balance() {
        use adjust::*;
        let mut v = BTreeMap::new();
        assert_eq!(curve_style(&v, "rgb"), 0);
        v.insert("rgblinear".to_string(), 1.0);
        assert_eq!(curve_style(&v, "rgb"), 1);
        set_curve_style(&mut v, "rgb", 3);
        assert_eq!((curve_style(&v, "rgb"), v["rgblinear"]), (3, 0.0));
        set_curve_style(&mut v, "r", 2);
        assert_eq!((curve_style(&v, "r"), v["rlinear"]), (2, 1.0));
        // Gamma 2 lifts 64 to about 128.
        v.insert("rgbgamma".to_string(), 2.0);
        let mut e = new_effect("tone_curve");
        e.params = v.clone();
        e.params.remove("rstyle");
        e.params.remove("rlinear");
        let gray: RgbaImage = ImageBuffer::from_pixel(2, 2, Rgba([64, 64, 64, 255]));
        let out = apply_effect(&gray, &e);
        assert!(
            (out.get_pixel(0, 0)[0] as i32 - 128).abs() <= 1,
            "{:?}",
            out.get_pixel(0, 0)
        );
        // Freehand: points every 4 levels and the last at 255.
        let lut: [f32; 256] = std::array::from_fn(|x| 255.0 - x as f32);
        let pts = freehand_points(&lut);
        assert_eq!(
            (pts.len(), pts[1], *pts.last().expect("points")),
            (65, (4.0, 251.0), (255.0, 0.0))
        );
        let c = crate::bitmap_modes::ToneCurve {
            points: pts,
            smooth: false,
        };
        let sm = smoothed(&c);
        assert_eq!((sm[0], sm[255]), (255.0, 0.0));
        assert!((sm[100] - 155.0).abs() < 0.5);
        // Auto Balance Tone stretches 50..200 to 0..255.
        let img: RgbaImage = ImageBuffer::from_fn(151, 4, |x, _| {
            let v = (50 + x) as u8;
            Rgba([v, v, v, 255])
        });
        let curves = balance_curves(&img, 0.005);
        assert!(curves[0].eval(50.0) < 1.0 && curves[0].eval(200.0) > 254.0);
        assert!((curves[1].eval(125.0) - 127.5).abs() < 1.0);
    }

    #[test]
    fn channel_lists_limit_levels_and_target_balance() {
        let gray: RgbaImage = ImageBuffer::from_pixel(2, 2, Rgba([64, 64, 64, 255]));
        let red_only = run(
            &gray,
            "contrast_enhancement",
            &[("channel", 1.0), ("input_high", 128.0)],
        );
        assert_eq!(red_only.get_pixel(0, 0).0, [128, 64, 64, 255]);
        let all = run(&gray, "contrast_enhancement", &[("input_high", 128.0)]);
        assert_eq!(all.get_pixel(0, 0).0, [128, 128, 128, 255]);
        // Sample 64 to target 128: green alone, then every channel.
        let set = [
            ("mid_sample", 0x404040 as f64),
            ("mid_target", 0x808080 as f64),
            ("channel", 2.0),
        ];
        assert_eq!(
            run(&gray, "target_balance", &set).get_pixel(0, 0).0,
            [64, 128, 64, 255]
        );
        let mut every = set.to_vec();
        every.push(("all_channels", 1.0));
        assert_eq!(
            run(&gray, "target_balance", &every)
                .get_pixel(0, 0)
                .0,
            [128, 128, 128, 255]
        );
    }

    #[test]
    fn previews_scale_pixel_sizes_only() {
        let mut e = new_effect("unsharp_mask");
        e.params.insert("radius".into(), 10.0);
        let s = scaled(&e, 0.25);
        assert_eq!(s.params["radius"], 2.5);
        // Percentages stay; defaults scale too, but not below the minimum.
        assert_eq!(s.params.get("percentage"), None);
        let tiny = scaled(&new_effect("gaussian_blur"), 0.01);
        assert_eq!(tiny.params["radius"], 0.1);
        assert_eq!(scaled(&new_effect("no_such"), 0.5), new_effect("no_such"));
    }

    #[test]
    fn settings_fall_back_to_defaults_and_stay_in_range() {
        let s = spec("gaussian_blur").expect("gaussian blur");
        let mut v = BTreeMap::new();
        let p = P {
            spec: s,
            values: &v,
        };
        assert_eq!(p.f("radius"), 1.0);
        v.insert("radius".to_string(), 1e9);
        let p = P {
            spec: s,
            values: &v,
        };
        assert!(p.f("radius") <= 250.0);
        v.insert("radius".to_string(), f64::NAN);
        let p = P {
            spec: s,
            values: &v,
        };
        assert_eq!(p.f("radius"), 1.0);
        assert_eq!(p.f("no_such"), 0.0);
    }

    #[test]
    fn stacks_keep_the_original_and_inflate_for_spreading_effects() {
        let src = img();
        let png = crate::bitmap_fx::encode(&src).expect("png");
        let mut blur = new_effect("gaussian_blur");
        blur.params.insert("radius".into(), 3.0);
        let stack = BitmapFxStack {
            rect: tracedraw_core::Rect::new(0.0, 0.0, 24.0, 20.0),
            width_px: 24,
            height_px: 20,
            png,
            effects: vec![blur.clone()],
        };
        let (out, rect) = render_stack(&stack, true).expect("renders");
        assert!(out.width() > 24 && rect.x0 < 0.0, "{:?}", rect);
        let (flat, r2) = render_stack(&stack, false).expect("renders");
        assert_eq!((flat.dimensions(), r2), ((24, 20), stack.rect));
        // Hidden effects do nothing.
        let mut hidden = stack.clone();
        hidden.effects[0].visible = false;
        let (same, _) = render_stack(&hidden, true).expect("renders");
        assert_eq!(same, src);
    }

    #[test]
    fn effects_are_added_edited_and_removed_in_single_steps() {
        use tracedraw_core::document::ShapeKind;
        let mut app = crate::app::App::headless();
        let src = img();
        let png = crate::bitmap_fx::encode(&src).expect("png");
        let id = app
            .new_shape(ShapeKind::Bitmap {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 24.0, 20.0),
                width_px: 24,
                height_px: 20,
                png: png.clone(),
                fx: None,
            })
            .expect("bitmap");
        app.select(vec![id]);
        let depth = app.engine.history_labels().0.len();
        app.add_bitmap_effect(new_effect("invert_colors"));
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        assert_eq!(app.bitmap_effects(id).len(), 1);
        let shown = |app: &crate::app::App| match &app.doc().find_shape(id).expect("bitmap").kind {
            ShapeKind::Bitmap { png, .. } => crate::bitmap_fx::decode(png).expect("png"),
            _ => panic!(),
        };
        assert_eq!(shown(&app).get_pixel(0, 0)[0], 255);
        // Hide it: the original shows again.
        app.edit_bitmap_effects(id, "Hide Effect", |l| l[0].visible = false);
        assert_eq!(shown(&app), src);
        // Remove it: no stack left.
        app.edit_bitmap_effects(id, "Delete Effect", |l| l.clear());
        assert!(matches!(
            app.doc().find_shape(id).expect("bitmap").kind,
            ShapeKind::Bitmap { fx: None, .. }
        ));
        app.undo();
        app.undo();
        assert_eq!(shown(&app).get_pixel(0, 0)[0], 255);
    }

    #[test]
    fn auto_inflate_is_a_document_switch() {
        use tracedraw_core::document::ShapeKind;
        let mut app = crate::app::App::headless();
        let png = crate::bitmap_fx::encode(&img()).expect("png");
        let id = app
            .new_shape(ShapeKind::Bitmap {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 24.0, 20.0),
                width_px: 24,
                height_px: 20,
                png,
                fx: None,
            })
            .expect("bitmap");
        app.select(vec![id]);
        assert!(app.doc().metadata.auto_inflate_bitmaps());
        app.set_auto_inflate_bitmaps(false);
        assert!(!app.doc().metadata.auto_inflate_bitmaps());
        let size = |app: &crate::app::App| match &app.doc().find_shape(id).expect("bitmap").kind {
            ShapeKind::Bitmap {
                width_px,
                height_px,
                ..
            } => (*width_px, *height_px),
            _ => (0, 0),
        };
        let mut blur = new_effect("gaussian_blur");
        blur.params.insert("radius".into(), 4.0);
        app.add_bitmap_effect(blur.clone());
        assert_eq!(size(&app), (24, 20));
        // Back on: the next change renders the stack grown.
        app.set_auto_inflate_bitmaps(true);
        app.edit_bitmap_effects(id, "Edit Effect", |l| l[0] = blur);
        assert_eq!(size(&app), (24 + 16, 20 + 16));
        app.undo();
        app.undo();
        assert!(!app.doc().metadata.auto_inflate_bitmaps());
    }
}
