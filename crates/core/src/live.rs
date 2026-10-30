//! Live (non-destructive) effects. A shape carries a stack of `Effect`s
//! that are evaluated whenever it is drawn or exported; the source
//! geometry is never changed. `evaluate` returns the shapes to draw
//! beneath the object, the object itself (possibly with its geometry
//! warped), and the shapes to draw above it. "Break Effect Apart" turns
//! the result into ordinary objects.

use crate::document::{Shape, ShapeKind};
use crate::geometry::{Affine, BezPath, PathEl, Point, Rect, Shape as _, Vec2};
use crate::style::{lerp_color, Fill};
use crate::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "effect", rename_all = "lowercase")]
pub enum Effect {
    /// Concentric copies inside or outside the outline.
    Contour {
        steps: u32,
        offset: f64,
        /// true = outside, false = inside; `to_center` ignores `steps`.
        outside: bool,
        to_center: bool,
        fill_to: Color,
        outline_to: Option<Color>,
        /// 0 = direct RGB, 1 = clockwise HSB, 2 = counter-clockwise HSB.
        color_blend: u8,
        /// How the steps turn their corners; older files have none (round).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        corners: Option<crate::shaping::Join>,
        /// Acceleration of the steps' spacing and of their colours, -1 to
        /// 1 (0: even).
        #[serde(default, skip_serializing_if = "is_zero")]
        accel_objects: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        accel_colors: f64,
    },
    /// Intermediate objects between this shape and `end`.
    Blend {
        end: Box<Shape>,
        steps: u32,
        /// Acceleration -1..1 for object spacing and colours.
        accel_objects: f64,
        accel_colors: f64,
        /// Extra rotation of the intermediates, degrees.
        rotation: f64,
        /// Optional path the blend follows (page space).
        path: Option<BezPath>,
        rotate_on_path: bool,
        /// The end object, when it is an object of the drawing: `end` is
        /// then a copy of it kept up to date (in page space), and the
        /// object draws itself. Older files have none (the end lives only
        /// in the effect and is drawn with the steps).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        end_id: Option<crate::ShapeId>,
        /// Spacing, loop, full path, sizing acceleration and the way the
        /// colours turn; older files have the defaults.
        #[serde(default, skip_serializing_if = "BlendOptions::is_default")]
        options: BlendOptions,
    },
    /// Geometry distortions (push/pull, zipper, twister).
    Distort(crate::effects::Distort),
    /// Envelope: a frame of nodes in local space (see
    /// [`crate::envelope`]) the shape's bounding box is mapped into.
    Envelope {
        nodes: Vec<Point>,
        keep_lines: bool,
        /// The handles of the segment from each node to the next; older
        /// files have none (each side is then the arc through its nodes).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        handles: Vec<[Point; 2]>,
        /// The corner nodes (bottom-left, bottom-right, top-right,
        /// top-left); older files have none (nodes 0, 2, 4 and 6 of 8).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        corners: Vec<usize>,
        #[serde(default)]
        mapping: crate::envelope::Mapping,
    },
    /// Perspective: the four corners of the bounding box moved.
    Perspective { corners: [Point; 4] },
    /// Extrusion with optional vanishing point and shading.
    Extrude {
        depth: Vec2,
        /// Vanishing point in page space; None = parallel extrusion.
        vanishing: Option<Point>,
        /// 0..1 fraction toward the vanishing point for the back face.
        amount: f64,
        shade_from: Option<Color>,
        shade_to: Option<Color>,
        light_angle: f64,
        light_intensity: f64,
        bevel: f64,
        /// How the body runs; older files have none (Back parallel, or
        /// Small back with a vanishing point). See [`crate::extrude`].
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<crate::extrude::ExtrudeKind>,
        /// The vanishing point is in the object's own space (it moves with
        /// the object) instead of the page's.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        on_object: bool,
        /// One colour for the whole body.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        solid: Option<Color>,
        /// Turn in space, the first light's elevation and further lights
        /// (see [`crate::extrude3d`]); `bevel` is the width of the near
        /// cap's bevel in mm.
        #[serde(default, skip_serializing_if = "crate::extrude3d::Solid::is_plain")]
        space: crate::extrude3d::Solid,
    },
    /// Bevel: a band of `distance` lit from `light_angle`, inside the
    /// outline (soft edge) or around it (emboss).
    Bevel {
        distance: f64,
        light_angle: f64,
        intensity: f64,
        /// Soft edge (0, the band inside the outline) or emboss (1, the
        /// band around it, so the object stands out of the page).
        style: u8,
        shadow_color: Color,
        light_color: Color,
        /// How many steps the band is cut into: 1 is two flat tones, more
        /// fade the light and the shade across the band, strongest at its
        /// outer edge. Older files have 1.
        #[serde(default = "one_level")]
        level: u8,
        /// The colour the band is toned from, in place of the object's
        /// own fill; older files use the fill.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        emboss_color: Option<Color>,
    },
    /// Block shadow: a solid extrusion behind the object.
    BlockShadow {
        offset: Vec2,
        color: Color,
        /// Keep a gap between the object and the shadow (overprint trick).
        gap: f64,
    },
    /// Symmetry: mirrored copies of the object across `lines` mirror lines
    /// through `center` (page space), the first at `angle` degrees and the
    /// others spread evenly; the object itself stays where it is.
    Symmetry {
        center: Point,
        angle: f64,
        lines: u8,
    },
    /// Lens over everything beneath (evaluated by the renderer).
    Lens(Lens),
    /// Non-uniform transparency: the fill's luminance is the opacity.
    Transparency {
        mask: Fill,
        merge: MergeMode,
        /// Apply to fill, outline or both (0, 1, 2).
        target: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MergeMode {
    #[default]
    Normal,
    Add,
    Subtract,
    Difference,
    Multiply,
    Divide,
    IfLighter,
    IfDarker,
    Hue,
    Saturation,
    Color,
    Invert,
    And,
    Or,
    Xor,
    Red,
    Green,
    Blue,
}

impl MergeMode {
    pub const ALL: [MergeMode; 18] = [
        MergeMode::Normal,
        MergeMode::Add,
        MergeMode::Subtract,
        MergeMode::Difference,
        MergeMode::Multiply,
        MergeMode::Divide,
        MergeMode::IfLighter,
        MergeMode::IfDarker,
        MergeMode::Hue,
        MergeMode::Saturation,
        MergeMode::Color,
        MergeMode::Invert,
        MergeMode::And,
        MergeMode::Or,
        MergeMode::Xor,
        MergeMode::Red,
        MergeMode::Green,
        MergeMode::Blue,
    ];
    pub fn name(self) -> &'static str {
        match self {
            MergeMode::Normal => "Normal",
            MergeMode::Add => "Add",
            MergeMode::Subtract => "Subtract",
            MergeMode::Difference => "Difference",
            MergeMode::Multiply => "Multiply",
            MergeMode::Divide => "Divide",
            MergeMode::IfLighter => "If Lighter",
            MergeMode::IfDarker => "If Darker",
            MergeMode::Hue => "Hue",
            MergeMode::Saturation => "Saturation",
            MergeMode::Color => "Color",
            MergeMode::Invert => "Invert",
            MergeMode::And => "And",
            MergeMode::Or => "Or",
            MergeMode::Xor => "Xor",
            MergeMode::Red => "Red",
            MergeMode::Green => "Green",
            MergeMode::Blue => "Blue",
        }
    }
}

/// The eleven lens types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "lens", rename_all = "lowercase")]
pub enum Lens {
    Brighten {
        rate: f64,
    },
    ColorAdd {
        color: Color,
        rate: f64,
    },
    ColorLimit {
        color: Color,
        rate: f64,
    },
    CustomColorMap {
        from: Color,
        to: Color,
        direction: u8,
    },
    FishEye {
        rate: f64,
    },
    HeatMap {
        rotation: f64,
    },
    Invert,
    Magnify {
        amount: f64,
    },
    TintedGrayscale {
        color: Color,
    },
    Transparency {
        rate: f64,
        color: Color,
    },
    Wireframe {
        outline: Color,
        fill: Color,
    },
    /// Bitmap effects applied to whatever lies beneath, in order (the
    /// application supplies them to the renderer; see
    /// `tracedraw_render::set_pixel_effects`).
    Effects {
        effects: Vec<crate::BitmapEffect>,
    },
}

impl Lens {
    pub fn name(&self) -> &'static str {
        match self {
            Lens::Brighten { .. } => "Brighten",
            Lens::ColorAdd { .. } => "Color Add",
            Lens::ColorLimit { .. } => "Color Limit",
            Lens::CustomColorMap { .. } => "Custom Color Map",
            Lens::FishEye { .. } => "Fish Eye",
            Lens::HeatMap { .. } => "Heat Map",
            Lens::Invert => "Invert",
            Lens::Magnify { .. } => "Magnify",
            Lens::TintedGrayscale { .. } => "Tinted Grayscale",
            Lens::Transparency { .. } => "Transparency",
            Lens::Wireframe { .. } => "Wireframe",
            Lens::Effects { .. } => "Bitmap Effects",
        }
    }

    /// Per-pixel colour function for the lenses that are pure colour maps
    /// (everything but fish eye, magnify and wireframe). Input and output
    /// are 0..1 RGB.
    pub fn map_color(&self, c: [f32; 3]) -> [f32; 3] {
        fn k(c: Color) -> [f32; 3] {
            let [r, g, b] = c.to_rgb8();
            [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
        }
        match self {
            Lens::Brighten { rate } => {
                let r = (*rate as f32 / 100.0).clamp(-1.0, 1.0);
                let f = |v: f32| {
                    if r >= 0.0 {
                        v + r * (1.0 - v)
                    } else {
                        v * (1.0 + r)
                    }
                };
                [f(c[0]), f(c[1]), f(c[2])]
            }
            Lens::ColorAdd { color, rate } => {
                let k = k(*color);
                let r = *rate as f32 / 100.0;
                [
                    (c[0] + r * k[0]).min(1.0),
                    (c[1] + r * k[1]).min(1.0),
                    (c[2] + r * k[2]).min(1.0),
                ]
            }
            Lens::ColorLimit { color, rate } => {
                let k = k(*color);
                let r = *rate as f32 / 100.0;
                [
                    c[0] * (1.0 - r) + r * c[0] * k[0],
                    c[1] * (1.0 - r) + r * c[1] * k[1],
                    c[2] * (1.0 - r) + r * c[2] * k[2],
                ]
            }
            Lens::CustomColorMap { from, to, .. } => {
                let y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
                let a = k(*from);
                let b = k(*to);
                [
                    a[0] + (b[0] - a[0]) * y,
                    a[1] + (b[1] - a[1]) * y,
                    a[2] + (b[2] - a[2]) * y,
                ]
            }
            Lens::HeatMap { rotation } => {
                let y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
                let t = (y + *rotation as f32 / 100.0).fract();
                // Blue, cyan, green, yellow, red, white.
                let stops: [[f32; 3]; 6] = [
                    [0.0, 0.0, 0.6],
                    [0.0, 0.8, 1.0],
                    [0.0, 0.8, 0.0],
                    [1.0, 1.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [1.0, 1.0, 1.0],
                ];
                let f = t * 5.0;
                let i = (f.floor() as usize).min(4);
                let u = f - i as f32;
                let a = stops[i];
                let b = stops[i + 1];
                [
                    a[0] + (b[0] - a[0]) * u,
                    a[1] + (b[1] - a[1]) * u,
                    a[2] + (b[2] - a[2]) * u,
                ]
            }
            Lens::Invert => [1.0 - c[0], 1.0 - c[1], 1.0 - c[2]],
            Lens::TintedGrayscale { color } => {
                let y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
                let k = k(*color);
                // Tint: black stays black, white becomes the tint colour's complement-free mix.
                [
                    y * (1.0 - (1.0 - k[0]) * 0.0) * (0.5 + 0.5 * k[0])
                        + y * 0.5 * (1.0 - k[0]) * 0.0,
                    y * (0.5 + 0.5 * k[1]),
                    y * (0.5 + 0.5 * k[2]),
                ]
            }
            Lens::Transparency { rate, color } => {
                let k = k(*color);
                let r = *rate as f32 / 100.0;
                [
                    c[0] * (1.0 - r) + k[0] * r,
                    c[1] * (1.0 - r) + k[1] * r,
                    c[2] * (1.0 - r) + k[2] * r,
                ]
            }
            Lens::FishEye { .. }
            | Lens::Magnify { .. }
            | Lens::Wireframe { .. }
            | Lens::Effects { .. } => c,
        }
    }
}

impl Effect {
    /// Only reshapes the outline (Distort, Envelope, Perspective): Convert
    /// to Curves and Break Apart make it part of the curves (#647).
    pub fn warps_outline(&self) -> bool {
        matches!(
            self,
            Effect::Envelope { .. } | Effect::Perspective { .. } | Effect::Distort(_)
        )
    }

    /// Draws objects made from the outline as it is at its place in the
    /// stack (contours, blends, extrusions, shadows, symmetry...): a warp
    /// after it changes nothing of what it drew, so it stays live.
    fn draws_from_outline(&self) -> bool {
        !self.warps_outline() && !matches!(self, Effect::Lens(_) | Effect::Transparency { .. })
    }
}

/// Which entries of an effect stack a bake takes: the warps switched on
/// with nothing before them drawing from the outline.
fn baked_entries(stack: &[(Effect, bool)]) -> Vec<bool> {
    let mut drawn_from = false;
    stack
        .iter()
        .map(|(e, on)| {
            if !*on {
                return false;
            }
            if e.draws_from_outline() {
                drawn_from = true;
            }
            !drawn_from && e.warps_outline()
        })
        .collect()
}

/// A picture anywhere in `shape` (it cannot become a curve).
fn holds_bitmap(shape: &Shape, depth: usize) -> bool {
    if depth > 64 {
        return true;
    }
    match &shape.kind {
        ShapeKind::Bitmap { .. } => true,
        ShapeKind::Group { children } => children.iter().any(|c| holds_bitmap(c, depth + 1)),
        ShapeKind::ClipFrame { frame, contents } => {
            holds_bitmap(frame, depth + 1) || contents.iter().any(|c| holds_bitmap(c, depth + 1))
        }
        _ => false,
    }
}

/// `shape` with its outline warps (envelope, perspective, distortion; those
/// no contour, blend or other drawn effect comes before) made part of its
/// outline, as Convert to Curves does (#647): a curve, or a group whose
/// objects are curves, drawn as before; the other effects stay, those
/// switched off too. `None` when there is nothing to bake, or a picture
/// inside would be lost.
pub fn bake_outline_warps(shape: &Shape) -> Option<Shape> {
    if !shape.has_outline_warp() || holds_bitmap(shape, 0) {
        return None;
    }
    let stack = shape.effect_stack();
    let mask = baked_entries(&stack);
    if !mask.contains(&true) {
        return None;
    }
    let mut probe = shape.clone();
    probe.effects = stack
        .iter()
        .zip(&mask)
        .filter(|(_, m)| **m)
        .map(|((e, _), _)| e.clone())
        .collect();
    probe.parked_effects.clear();
    let main = evaluate(&probe).main;
    let rest: Vec<(Effect, bool)> = stack
        .into_iter()
        .zip(mask)
        .filter(|(_, m)| !m)
        .map(|(e, _)| e)
        .collect();
    let (effects, parked) = Shape::split_stack(rest);
    let mut out = shape.clone();
    let tolerance = refit_tolerance(&main.page_path());
    out.kind = refit_kind(main.kind, tolerance);
    out.effects = effects;
    out.parked_effects = parked;
    out.node_types.clear();
    out.spline = None;
    Some(out)
}

/// The outline warps a bake would take, each with the box it maps (for
/// the letters of a text broken apart). Not for groups and clip frames.
pub struct OutlineWarp(Vec<(Effect, Rect)>);

impl OutlineWarp {
    /// `path` (the object's own space) warped with the boxes of the whole
    /// outline. A distortion also measures its reach on the path it is
    /// given, so parts go through [`OutlineWarp::apply_parts`].
    pub fn apply(&self, path: &BezPath) -> BezPath {
        self.0.iter().fold(path.clone(), |p, (e, b)| {
            warp_outline(e, &p, *b).unwrap_or(p)
        })
    }

    /// `parts`, which together make the whole outline, warped as the whole
    /// is drawn: warped together, then split back by their subpaths (a
    /// twister turns each part by its distance across the whole, not
    /// across the part alone). Each part on its own if the warp changed
    /// the number of subpaths.
    pub fn apply_parts(&self, parts: &[BezPath]) -> Vec<BezPath> {
        warp_parts(parts, &|p| self.apply(p))
    }
}

/// `parts`, which together make one outline, warped by `warp` as the
/// whole is: warped together, then split back by their subpaths. Each
/// part on its own if the warp changed the number of subpaths.
fn warp_parts(parts: &[BezPath], warp: &dyn Fn(&BezPath) -> BezPath) -> Vec<BezPath> {
    let starts = |p: &BezPath| {
        p.elements()
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count()
    };
    let mut whole = BezPath::new();
    for p in parts {
        whole.extend(p.elements().iter().copied());
    }
    let warped = warp(&whole);
    let counts: Vec<usize> = parts.iter().map(starts).collect();
    if counts.iter().sum::<usize>() != starts(&warped) {
        return parts.iter().map(warp).collect();
    }
    let mut out = Vec::with_capacity(parts.len());
    let mut els = warped.elements().iter().copied().peekable();
    for n in counts {
        let mut part = BezPath::new();
        let mut seen = 0;
        while let Some(e) = els.peek() {
            if matches!(e, PathEl::MoveTo(_)) {
                if seen == n {
                    break;
                }
                seen += 1;
            }
            part.push(*e);
            els.next();
        }
        out.push(part);
    }
    out
}

/// The outline warps of `shape` a bake would take (see [`OutlineWarp`]).
pub fn outline_warp(shape: &Shape) -> Option<OutlineWarp> {
    if matches!(
        shape.kind,
        ShapeKind::Group { .. } | ShapeKind::ClipFrame { .. } | ShapeKind::Bitmap { .. }
    ) {
        return None;
    }
    let stack = shape.effect_stack();
    let mask = baked_entries(&stack);
    let mut p = shape.local_path();
    let mut steps = Vec::new();
    for ((e, _), m) in stack.iter().zip(mask) {
        if !m {
            continue;
        }
        let b = p.bounding_box();
        steps.push((e.clone(), b));
        p = warp_outline(e, &p, b).unwrap_or(p);
    }
    (!steps.is_empty()).then_some(OutlineWarp(steps))
}

/// One outline warp of `p` with the box `b` (as [`evaluate`] does).
fn warp_outline(e: &Effect, p: &BezPath, b: Rect) -> Option<BezPath> {
    match e {
        Effect::Distort(how) => Some(crate::effects::distort_in(p, b, *how)),
        Effect::Envelope {
            nodes,
            keep_lines,
            handles,
            corners,
            mapping,
        } => {
            let frame = crate::envelope::Frame::from_parts(nodes, handles, corners)?;
            Some(crate::envelope::apply_in(
                p,
                b,
                &frame,
                *keep_lines,
                *mapping,
            ))
        }
        Effect::Perspective { corners } => Some(perspective_in(p, b, corners)),
        _ => None,
    }
}

/// The letters of a warped text, each its own curve warped as the whole
/// text is (Break Apart, #647): one per character (a mark stays with its
/// letter), the underlines and other marks as one more. `None` for other
/// shapes, without a text engine or without a warp to bake.
pub fn warped_letters(shape: &Shape) -> Option<Vec<BezPath>> {
    if !matches!(shape.kind, ShapeKind::Text { .. }) {
        return None;
    }
    let warp = outline_warp(shape)?;
    let layout = crate::document::text_glyph_layout(&shape.kind)?;
    let mut letters: Vec<BezPath> = Vec::new();
    for g in &layout.glyphs {
        match letters.last_mut() {
            Some(last) if g.text.is_empty() => last.extend(g.path.elements().iter().copied()),
            _ => letters.push(g.path.clone()),
        }
    }
    if !layout.rest.elements().is_empty() {
        letters.push(layout.rest.clone());
    }
    // Curves fill with the even-odd rule, text with nonzero (#800).
    let letters: Vec<BezPath> = letters
        .iter()
        .filter(|p| !p.elements().is_empty())
        .map(crate::shaping::nonzero_as_even_odd)
        .collect();
    let warped = warp.apply_parts(&letters);
    let mut all = BezPath::new();
    warped
        .iter()
        .for_each(|p| all.extend(p.elements().iter().copied()));
    let tolerance = refit_tolerance(&all);
    Some(warped.iter().map(|p| refit_path(p, tolerance)).collect())
}

/// How closely a baked warp's outline is fitted with curves: a ten
/// thousandth of the outline's size, between 0.005 and 0.05 mm. The warp
/// is drawn as short lines; fitting them gives smooth curves with about
/// as many nodes as the outline had before (#647).
fn refit_tolerance(path: &BezPath) -> f64 {
    let b = path.bounding_box();
    let size = b.width().hypot(b.height());
    if size.is_finite() {
        (size * 1e-4).clamp(0.005, 0.05)
    } else {
        0.01
    }
}

/// A baked warp's outline (short lines, and curves flattened first) as
/// curves within `tolerance`, its corners kept sharp (#647, #840).
fn refit_path(path: &BezPath, tolerance: f64) -> BezPath {
    let mut lines = BezPath::new();
    crate::geometry::flatten(path, tolerance * 0.25, &mut |el| lines.push(el));
    crate::shaping::fit_polygon_curves(&lines, tolerance)
}

/// Whether outlines `a` and `b` run within `tol` of each other: every
/// point of either flattened lies that near the other (for tests of a
/// baked warp, which is fitted with curves).
#[cfg(test)]
pub(crate) fn same_outline(a: &BezPath, b: &BezPath, tol: f64) -> bool {
    fn segments(p: &BezPath) -> Vec<(Point, Point)> {
        let mut out = Vec::new();
        let (mut at, mut start) = (Point::ZERO, Point::ZERO);
        crate::geometry::flatten(p, 0.001, &mut |el| match el {
            PathEl::MoveTo(q) => {
                at = q;
                start = q;
            }
            PathEl::LineTo(q) => {
                out.push((at, q));
                at = q;
            }
            PathEl::ClosePath => {
                out.push((at, start));
                at = start;
            }
            _ => {}
        });
        out
    }
    fn dist(p: Point, (a, b): (Point, Point)) -> f64 {
        let d = b - a;
        let len = d.hypot2();
        let t = if len > 0.0 {
            ((p - a).dot(d) / len).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (p - (a + d * t)).hypot()
    }
    let (sa, sb) = (segments(a), segments(b));
    let near = |from: &[(Point, Point)], to: &[(Point, Point)]| {
        from.iter().all(|s| {
            [s.0, s.1]
                .iter()
                .all(|p| to.iter().map(|t| dist(*p, *t)).fold(f64::MAX, f64::min) <= tol)
        })
    };
    !sa.is_empty() && near(&sa, &sb) && near(&sb, &sa)
}

/// `kind` with its curves (a group's too) fitted within `tolerance`.
fn refit_kind(kind: ShapeKind, tolerance: f64) -> ShapeKind {
    match kind {
        ShapeKind::Path { path, closed } => ShapeKind::Path {
            path: refit_path(&path, tolerance),
            closed,
        },
        ShapeKind::Group { children } => ShapeKind::Group {
            children: children
                .into_iter()
                .map(|mut c| {
                    c.kind = refit_kind(c.kind, tolerance);
                    c
                })
                .collect(),
        },
        other => other,
    }
}

impl Shape {
    /// An outline warp a bake would take (no evaluation: menus ask every
    /// frame). Pictures keep their warps.
    pub fn has_outline_warp(&self) -> bool {
        self.effects
            .iter()
            .take_while(|e| !e.draws_from_outline())
            .any(Effect::warps_outline)
            && !holds_bitmap(self, 0)
    }

    /// Effects of this object that stay live and read its outline after a
    /// bake (a contour, shadow, blend..., or a warp kept for a picture
    /// inside): the warps of the objects inside it then stay live too,
    /// or what those effects draw would change.
    pub fn outline_stays_live(&self) -> bool {
        self.effects.iter().any(Effect::draws_from_outline)
            || (self.effects.iter().any(Effect::warps_outline) && holds_bitmap(self, 0))
    }
}

impl Shape {
    /// The object, or an object in its groups, has live effects.
    pub fn has_live_effects(&self) -> bool {
        !self.effects.is_empty()
            || matches!(&self.kind, ShapeKind::Group { children } if children.iter().any(Shape::has_live_effects))
    }

    /// The bounds of what the object draws: the outline its live effects
    /// warp it to and the objects they add (contours, blends, extrusions)
    /// included. The same as [`Shape::bounds`] without live effects.
    pub fn visual_bounds(&self) -> Rect {
        if !self.has_live_effects() {
            return self.bounds();
        }
        if self.effects.is_empty() {
            if let ShapeKind::Group { children } = &self.kind {
                return children
                    .iter()
                    .filter(|c| !c.is_empty_path())
                    .map(|c| {
                        let mut c = c.clone();
                        c.transform_by(self.transform);
                        c.visual_bounds()
                    })
                    .reduce(|a, b| a.union(b))
                    .unwrap_or_else(|| self.bounds());
            }
        }
        let ev = evaluate(self);
        std::iter::once(&ev.main)
            .chain(ev.below.iter())
            .chain(ev.above.iter())
            .map(Shape::bounds)
            .filter(|b| {
                b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite()
            })
            .reduce(|a, b| a.union(b))
            .unwrap_or_else(|| self.bounds())
    }
}

impl Effect {
    /// A fresh envelope around `b` (Putty mapping): the object stays as it
    /// is until the frame changes.
    pub fn envelope_around(b: Rect, keep_lines: bool) -> Effect {
        Effect::from_envelope(
            &crate::envelope::Frame::around(b),
            keep_lines,
            crate::envelope::Mapping::default(),
        )
    }

    /// An envelope effect holding `frame`.
    pub fn from_envelope(
        frame: &crate::envelope::Frame,
        keep_lines: bool,
        mapping: crate::envelope::Mapping,
    ) -> Effect {
        Effect::Envelope {
            nodes: frame.nodes.clone(),
            keep_lines,
            handles: frame.handles.clone(),
            corners: frame.corners.to_vec(),
            mapping,
        }
    }

    /// The frame of an envelope effect (None for other effects and for an
    /// envelope with too few nodes).
    pub fn envelope_frame(&self) -> Option<crate::envelope::Frame> {
        match self {
            Effect::Envelope {
                nodes,
                handles,
                corners,
                ..
            } => crate::envelope::Frame::from_parts(nodes, handles, corners),
            _ => None,
        }
    }
}

/// Result of evaluating a shape's effect stack.
#[derive(Clone)]
pub struct Evaluated {
    pub below: Vec<Shape>,
    pub main: Shape,
    pub above: Vec<Shape>,
}

/// How many evaluations [`evaluate`] remembers.
const EVALUATION_MEMORY: usize = 8;

/// The latest evaluations of shapes that draw objects from their outline,
/// newest last, each with the shape it was made from. Shared by all
/// threads: the rasteriser works on its own.
static EVALUATED: std::sync::Mutex<Vec<(Shape, Evaluated)>> = std::sync::Mutex::new(Vec::new());

/// Evaluate every effect of `shape` in order. Geometry effects replace the
/// main shape's kind with a path; object effects add shapes.
///
/// A contour, blend or extrusion costs milliseconds, and a frame asks for
/// it to draw, to measure and to pick: the evaluations of the latest
/// shapes are kept, so the same shape is worked out once (#897).
pub fn evaluate(shape: &Shape) -> Evaluated {
    if !shape.effects.iter().any(Effect::draws_from_outline) {
        return evaluate_now(shape);
    }
    if let Ok(mut kept) = EVALUATED.lock() {
        if let Some(i) = kept.iter().position(|(from, _)| from == shape) {
            let hit = kept.remove(i);
            let ev = hit.1.clone();
            kept.push(hit);
            return ev;
        }
    }
    let ev = evaluate_now(shape);
    if let Ok(mut kept) = EVALUATED.lock() {
        if kept.len() >= EVALUATION_MEMORY {
            kept.remove(0);
        }
        kept.push((shape.clone(), ev.clone()));
    }
    ev
}

fn evaluate_now(shape: &Shape) -> Evaluated {
    let mut main = shape.clone();
    let mut below: Vec<Shape> = Vec::new();
    let mut above: Vec<Shape> = Vec::new();
    for effect in &shape.effects {
        match effect {
            Effect::Distort(how) => {
                reshape(&mut main, &|p, b| crate::effects::distort_in(p, b, *how));
            }
            Effect::Envelope {
                nodes,
                keep_lines,
                handles,
                corners,
                mapping,
            } => {
                if let Some(frame) = crate::envelope::Frame::from_parts(nodes, handles, corners) {
                    reshape(&mut main, &|p, b| {
                        crate::envelope::apply_in(p, b, &frame, *keep_lines, *mapping)
                    });
                }
            }
            Effect::Perspective { corners } => {
                reshape(&mut main, &|p, b| perspective_in(p, b, corners));
            }
            Effect::Contour {
                steps,
                offset,
                outside,
                to_center,
                fill_to,
                outline_to,
                color_blend,
                corners,
                accel_objects,
                accel_colors,
            } => {
                let shapes = contour(
                    &main,
                    &ContourParams {
                        steps: *steps,
                        offset: *offset,
                        outside: *outside,
                        to_center: *to_center,
                        fill_to: *fill_to,
                        outline_to: *outline_to,
                        color_blend: *color_blend,
                        corners: corners.unwrap_or(crate::shaping::Join::Round),
                        accel_objects: *accel_objects,
                        accel_colors: *accel_colors,
                    },
                );
                if *outside {
                    // Outermost first so the object stays on top.
                    below.extend(shapes.into_iter().rev());
                } else {
                    above.extend(shapes);
                }
            }
            Effect::Blend {
                end,
                steps,
                accel_objects,
                accel_colors,
                rotation,
                path,
                rotate_on_path,
                end_id,
                options,
            } => {
                let shapes = blend(
                    &main,
                    end,
                    *steps,
                    (*accel_objects, *accel_colors),
                    *rotation,
                    path.as_ref(),
                    *rotate_on_path,
                    options,
                );
                if options.steps_below {
                    // Bottom to top: the end (when the effect draws it),
                    // then the steps from the end's side to the start.
                    if end_id.is_none() {
                        below.push((**end).clone());
                    }
                    below.extend(shapes.into_iter().rev());
                } else {
                    above.extend(shapes);
                    // An end object of the drawing draws itself.
                    if end_id.is_none() {
                        above.push((**end).clone());
                    }
                }
            }
            Effect::Extrude {
                depth,
                vanishing,
                amount,
                shade_from,
                shade_to,
                light_angle,
                light_intensity,
                kind,
                on_object,
                solid,
                bevel,
                space,
            } => {
                if let Some(mut body) = extrude_body(
                    &main,
                    *depth,
                    *vanishing,
                    *amount,
                    (*shade_from, *shade_to, *solid),
                    (*light_angle, *light_intensity),
                    *kind,
                    *on_object,
                ) {
                    body.bevel = *bevel;
                    body.solid = space.clone();
                    let faces = body.faces(&main);
                    below.extend(faces.below);
                    above.extend(faces.above);
                    if let Some(m) = faces.main {
                        main = m;
                    }
                }
            }
            e @ Effect::Bevel { .. } => {
                above.extend(bevel(&main, e));
            }
            Effect::BlockShadow { offset, color, gap } => {
                below.extend(block_shadow(&main, *offset, *color, *gap));
            }
            Effect::Symmetry {
                center,
                angle,
                lines,
            } => {
                above.extend(symmetry(&main, *center, *angle, *lines));
            }
            Effect::Lens(_) | Effect::Transparency { .. } => {}
        }
    }
    Evaluated { below, main, above }
}

/// Reshape `shape` by `f`, which works in the shape's own space. A group
/// (or a clip frame) is reshaped object by object, each keeping its own
/// fill and outline, as the reference program does with an envelope on a
/// group; the whole would otherwise become one path with the group's
/// empty fill and vanish.
/// `f` takes a path and the box mapped by the effect: the shape's own
/// bounding box, for a group the box of all its objects.
fn reshape(shape: &mut Shape, f: &dyn Fn(&BezPath, Rect) -> BezPath) {
    let b = shape.local_path().bounding_box();
    // Text with characters of their own colours: a group of one curve a
    // look (as the text is drawn in parts), warped together in the box of
    // the whole text, so the colours stay (#912).
    if let Some(parts) = shape.text_parts() {
        let paths: Vec<BezPath> = parts.iter().map(Shape::local_path).collect();
        let warped = warp_parts(&paths, &|p| f(p, b));
        let children = parts
            .into_iter()
            .zip(warped)
            .map(|(mut part, path)| {
                part.transform = Affine::IDENTITY;
                part.effects.clear();
                part.parked_effects.clear();
                part.shadow = None;
                set_path(&mut part, path);
                part
            })
            .collect();
        shape.kind = ShapeKind::Group { children };
        shape.spline = None;
        shape.node_types.clear();
        return;
    }
    match &mut shape.kind {
        ShapeKind::Group { children } => {
            for c in children.iter_mut() {
                reshape_inside(c, Affine::IDENTITY, b, f);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            reshape_inside(frame, Affine::IDENTITY, b, f);
            for c in contents.iter_mut() {
                reshape_inside(c, Affine::IDENTITY, b, f);
            }
        }
        _ => {
            let p = f(&shape.local_path(), b);
            set_path(shape, p);
        }
    }
}

/// [`reshape`] for an object inside a group whose space is `parent` times
/// the object's transform away from the object's own.
fn reshape_inside(
    shape: &mut Shape,
    parent: Affine,
    b: Rect,
    f: &dyn Fn(&BezPath, Rect) -> BezPath,
) {
    let t = parent * shape.transform;
    match &mut shape.kind {
        ShapeKind::Group { children } => {
            for c in children.iter_mut() {
                reshape_inside(c, t, b, f);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            reshape_inside(frame, t, b, f);
            for c in contents.iter_mut() {
                reshape_inside(c, t, b, f);
            }
        }
        _ => {
            if t.determinant().abs() < 1e-12 {
                return;
            }
            let warped = f(&(t * shape.local_path()), b);
            set_path(shape, t.inverse() * warped);
        }
    }
}

fn set_path(shape: &mut Shape, path: BezPath) {
    let closed = path
        .elements()
        .iter()
        .any(|e| matches!(e, PathEl::ClosePath));
    shape.kind = ShapeKind::Path { path, closed };
    // A warped outline is no longer the B-spline of its control points,
    // and its nodes are new (#647).
    shape.spline = None;
    shape.node_types.clear();
}

/// Break the effect stack into ordinary shapes (below, main, above), with
/// the given ids assigned in order.
pub fn break_apart(shape: &Shape, mut next_id: impl FnMut() -> crate::ShapeId) -> Vec<Shape> {
    let ev = evaluate(shape);
    let mut out = Vec::new();
    for mut s in ev.below {
        s.id = next_id();
        s.effects.clear();
        out.push(s);
    }
    let mut m = ev.main;
    m.effects.clear();
    out.push(m);
    for mut s in ev.above {
        s.id = next_id();
        s.effects.clear();
        out.push(s);
    }
    out
}

fn mix(a: Color, b: Color, t: f64, mode: u8) -> Color {
    if mode == 0 {
        return lerp_color(a, b, t as f32);
    }
    let (h1, s1, v1) = a.to_hsb();
    let (h2, s2, v2) = b.to_hsb();
    let mut dh = h2 - h1;
    if mode == 1 && dh < 0.0 {
        dh += 360.0;
    }
    if mode == 2 && dh > 0.0 {
        dh -= 360.0;
    }
    Color::from_hsb(
        (h1 + dh * t).rem_euclid(360.0),
        s1 + (s2 - s1) * t,
        v1 + (v2 - v1) * t,
    )
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// What a contour is drawn from.
pub struct ContourParams {
    pub steps: u32,
    pub offset: f64,
    pub outside: bool,
    pub to_center: bool,
    pub fill_to: Color,
    pub outline_to: Option<Color>,
    pub color_blend: u8,
    pub corners: crate::shaping::Join,
    pub accel_objects: f64,
    pub accel_colors: f64,
}

/// Where the middle step of a blend with acceleration `a` (-1 to 1)
/// lands, as a share of the way from the start: `0.5 - 0.45 a`, so at
/// +1 it is 5 % from the start and at -1 5 % from the end, the same
/// distance either way (#1261: the exponent `1 + a` put it 25 % from
/// the start at +1 and 3 % from the end at -1, so the slider went all
/// the way on one side only).
pub fn blend_middle(a: f64) -> f64 {
    let a = if a.is_finite() {
        a.clamp(-1.0, 1.0)
    } else {
        0.0
    };
    0.5 - 0.45 * a
}

/// The exponent a blend's steps take for acceleration `a`: step `t` is
/// placed at `t ^ blend_exponent(a)`, the middle step at
/// [`blend_middle`]`(a)`.
pub fn blend_exponent(a: f64) -> f64 {
    blend_middle(a).ln() / 0.5f64.ln()
}

/// `t` (0 to 1) bent by an acceleration `a` (-1 to 1): `t ^ 4^a`, so a
/// positive acceleration packs the steps toward the start and a negative
/// one toward the end (contours).
pub fn accelerate(t: f64, a: f64) -> f64 {
    let a = if a.is_finite() {
        a.clamp(-1.0, 1.0)
    } else {
        0.0
    };
    t.clamp(0.0, 1.0).powf(4f64.powf(a))
}

/// The most steps a contour draws.
const MAX_CONTOUR_STEPS: u32 = 999;

/// The most steps a blend draws (the range the Blend tool offers). A
/// blend asking for more, from a file or a script, draws this many, so
/// one value can never fill the memory with steps (#980).
pub const MAX_BLEND_STEPS: u32 = 999;

/// The contour steps of `shape`: step `i` of `n` is its outline offset by
/// `n * offset * accelerate(i / n, accel_objects)` (to the centre: by
/// `i * offset` until nothing is left), coloured `accelerate(i / n,
/// accel_colors)` of the way to the end colours.
///
/// The offset is measured on the page: the outline is offset with the
/// object's transformation applied and brought back to its local
/// coordinates, so a scaled or stretched object keeps the distance the
/// user typed (#754).
fn contour(shape: &Shape, p: &ContourParams) -> Vec<Shape> {
    let to_page = shape.transform;
    let to_local = if to_page.determinant().abs() > 1e-12 {
        to_page.inverse()
    } else {
        return Vec::new();
    };
    let base = to_page * shape.local_path();
    let from_fill = shape.fill.preview_color().unwrap_or(Color::WHITE);
    let from_outline = shape.stroke.as_ref().map(|s| s.color);
    let sign = if p.outside && !p.to_center { 1.0 } else { -1.0 };
    let offset = if p.offset.is_finite() {
        p.offset.abs().max(1e-3)
    } else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    if p.to_center {
        for i in 1..=MAX_CONTOUR_STEPS {
            let q = crate::shaping::offset_with(&base, sign * offset * i as f64, p.corners);
            if q.elements().is_empty() {
                break;
            }
            paths.push(to_local * q);
        }
    } else {
        let n = p.steps.clamp(1, MAX_CONTOUR_STEPS);
        let total = offset * n as f64;
        for i in 1..=n {
            let d = total * accelerate(i as f64 / n as f64, p.accel_objects);
            let q = crate::shaping::offset_with(&base, sign * d, p.corners);
            if q.elements().is_empty() {
                break;
            }
            paths.push(to_local * q);
        }
    }
    let n = paths.len().max(1) as f64;
    paths
        .into_iter()
        .enumerate()
        .map(|(i, path)| {
            let t = accelerate((i + 1) as f64 / n, p.accel_colors);
            let mut s = shape.clone();
            s.effects.clear();
            s.shadow = None;
            s.kind = ShapeKind::Path { path, closed: true };
            if !matches!(shape.fill, Fill::None) {
                s.fill = Fill::Solid(mix(from_fill, p.fill_to, t, p.color_blend));
            }
            if let (Some(st), Some(from)) = (&mut s.stroke, from_outline) {
                st.color = mix(from, p.outline_to.unwrap_or(p.fill_to), t, p.color_blend);
            }
            s
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn blend(
    start: &Shape,
    end: &Shape,
    steps: u32,
    (accel_objects, accel_colors): (f64, f64),
    rotation: f64,
    path: Option<&BezPath>,
    rotate_on_path: bool,
    options: &BlendOptions,
) -> Vec<Shape> {
    let a = start.page_path();
    let b = end.page_path();
    let path_samples = path.map(|p| {
        let mut pts = Vec::new();
        kurbo::flatten(p.elements().iter().copied(), 0.05, |el| match el {
            PathEl::MoveTo(q) | PathEl::LineTo(q) => pts.push(q),
            _ => {}
        });
        let mut cum = vec![0.0];
        for w in pts.windows(2) {
            cum.push(cum.last().copied().unwrap_or(0.0) + (w[1] - w[0]).hypot());
        }
        (pts, cum)
    });
    let path_len = path_samples
        .as_ref()
        .and_then(|(_, cum)| cum.last().copied())
        .unwrap_or(0.0);
    // Fixed spacing along a path sets the number of steps.
    let n = match options.spacing {
        Some(d) if d > 1e-6 && path_len > 0.0 => ((path_len / d).floor() as usize)
            .saturating_sub(1)
            .clamp(1, 999),
        _ => steps.clamp(1, MAX_BLEND_STEPS) as usize,
    };
    // Paired from the given nodes (in page space now) or by shape.
    let nodes = options.map_nodes.map(|m| {
        let (ts, te) = (start.transform, end.transform);
        [
            ts * m.start[0],
            ts * m.start[1],
            te * m.end[0],
            te * m.end[1],
        ]
    });
    let paths = crate::effects::blend_from_nodes(&a, &b, n, 120, nodes);
    let fa = start.fill.preview_color();
    let fb = end.fill.preview_color();
    let ca = start.stroke.as_ref().map(|s| s.color);
    let cb = end.stroke.as_ref().map(|s| s.color);
    let wa = start.stroke.as_ref().map(|s| s.width).unwrap_or(0.0);
    let wb = end.stroke.as_ref().map(|s| s.width).unwrap_or(0.0);
    let ba = a.bounding_box();
    let bb = b.bounding_box();
    let ca_center = ba.center();
    let cb_center = bb.center();
    let middle = ca_center.midpoint(cb_center);
    let size = |r: Rect| (r.width() + r.height()) / 2.0;
    let mut out = Vec::new();
    for (i, p) in paths.into_iter().enumerate() {
        // Along the whole path the steps reach its ends; otherwise they
        // sit between the two objects.
        let t = if options.full_path && path_samples.is_some() && n > 1 {
            i as f64 / (n - 1) as f64
        } else {
            (i + 1) as f64 / (n + 1) as f64
        };
        let to = t.powf(blend_exponent(accel_objects));
        let tc = t.powf(blend_exponent(accel_colors));
        let mut s = start.clone();
        s.effects.clear();
        s.shadow = None;
        s.transform = Affine::IDENTITY;
        let mut geo = p;
        let center = geo.bounding_box().center();
        let mut extra = Affine::IDENTITY;
        if options.accel_sizing {
            // The size the acceleration asks for over the size the even
            // blend gave.
            let (sa, sb) = (size(ba), size(bb));
            let even = sa + (sb - sa) * t;
            let wanted = sa + (sb - sa) * to;
            if even > 1e-9 {
                extra = Affine::scale_about(wanted / even, center);
            }
        }
        if rotation != 0.0 {
            let pivot = if options.loop_rotation {
                middle
            } else {
                center
            };
            extra = Affine::rotate_about(rotation.to_radians() * to, pivot) * extra;
        }
        if path_samples.is_none() && to != t {
            // Off a path the object acceleration moves the steps toward
            // one end: each goes where its accelerated place lies between
            // the two objects' centres (the steps stay evenly sized unless
            // Apply to sizing is on). It moved only sizes and turns, so
            // the slider seemed to do nothing (#910).
            extra = Affine::translate((cb_center - ca_center) * (to - t)) * extra;
        }
        if let Some((pts, cum)) = &path_samples {
            if pts.len() >= 2 {
                let total = *cum.last().unwrap_or(&0.0);
                let s_at = to * total;
                let k = cum
                    .partition_point(|c| *c <= s_at)
                    .saturating_sub(1)
                    .min(pts.len() - 2);
                let seg = pts[k + 1] - pts[k];
                let len = seg.hypot().max(1e-9);
                let u = ((s_at - cum[k]) / len).clamp(0.0, 1.0);
                let pos = pts[k] + seg * u;
                // The step's own centre goes onto the path.
                extra = Affine::translate(pos - center) * extra;
                if rotate_on_path {
                    let ang = seg.y.atan2(seg.x);
                    extra = Affine::translate(pos.to_vec2())
                        * Affine::rotate(ang)
                        * Affine::translate(-pos.to_vec2())
                        * extra;
                }
            }
        }
        geo = extra * geo;
        // The steps of two open lines are open lines (#1024).
        let closed = geo
            .elements()
            .iter()
            .any(|e| matches!(e, PathEl::ClosePath));
        s.kind = ShapeKind::Path { path: geo, closed };
        if let (Some(x), Some(y)) = (fa, fb) {
            s.fill = Fill::Solid(blend_color(x, y, tc as f32, options.color_blend));
        }
        if let (Some(st), Some(x), Some(y)) = (&mut s.stroke, ca, cb) {
            st.color = blend_color(x, y, tc as f32, options.color_blend);
            st.width = wa + (wb - wa) * to;
        }
        out.push(s);
    }
    out
}

/// How a blend's colours go from the start colour to the end colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ColorBlend {
    /// Straight through the colour space.
    #[default]
    Direct,
    /// Around the colour wheel, hue increasing.
    Clockwise,
    /// Around the colour wheel, hue decreasing.
    CounterClockwise,
}

/// A blend's further options.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BlendOptions {
    /// Along a path: the distance between steps, mm (the step count then
    /// follows from the path's length). `None`: the step count.
    pub spacing: Option<f64>,
    /// The rotation turns the steps about the middle between the two
    /// objects instead of about each step's own centre.
    pub loop_rotation: bool,
    /// Along a path: the steps spread over the whole path, its ends
    /// included.
    pub full_path: bool,
    /// The object acceleration changes the steps' sizes too.
    pub accel_sizing: bool,
    pub color_blend: ColorBlend,
    /// The end object lies below the start object: the steps are drawn
    /// below the start, from the end's side up, so they sit between the
    /// two (kept up to date with the stacking order by the editor).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub steps_below: bool,
    /// The nodes the morph starts from: on each object, in its own
    /// coordinates, the node the steps pair with the other object's, then
    /// a point the outline passes next (the way it runs). The outlines are
    /// then paired from these nodes as they are, twists included, as a
    /// `.cdr` blend draws (#597). `None`: they are matched by shape, so
    /// the steps do not twist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_nodes: Option<BlendNodes>,
}

/// See [`BlendOptions::map_nodes`]: each pair is a first node and a point
/// after it along the outline, in that object's own coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlendNodes {
    pub start: [Point; 2],
    pub end: [Point; 2],
}

impl BlendOptions {
    pub fn is_default(&self) -> bool {
        *self == BlendOptions::default()
    }
}

/// A colour between `a` and `b` at `t`, straight or around the wheel.
pub fn blend_color(a: Color, b: Color, t: f32, how: ColorBlend) -> Color {
    let turn = match how {
        ColorBlend::Direct => return lerp_color(a, b, t),
        ColorBlend::Clockwise => 1.0,
        ColorBlend::CounterClockwise => -1.0,
    };
    let (ha, sa, va) = a.to_hsb();
    let (hb, sb, vb) = b.to_hsb();
    // The hue's way round in the chosen direction, in degrees.
    let mut d = (hb - ha).rem_euclid(360.0);
    if turn < 0.0 {
        d -= 360.0;
        if d <= -360.0 {
            d += 360.0;
        }
    }
    let t = t as f64;
    let h = (ha + d * t).rem_euclid(360.0);
    Color::from_hsb(h, sa + (sb - sa) * t, va + (vb - va) * t)
}

/// The body an extrude effect on `main` describes (older files: Back
/// parallel, or Small back with a vanishing point). `None` for a body
/// that needs a vanishing point and has none.
#[allow(clippy::too_many_arguments)]
pub fn extrude_body(
    main: &Shape,
    depth: Vec2,
    vanishing: Option<Point>,
    amount: f64,
    (shade_from, shade_to, solid): (Option<Color>, Option<Color>, Option<Color>),
    (light_angle, light_intensity): (f64, f64),
    kind: Option<crate::extrude::ExtrudeKind>,
    on_object: bool,
) -> Option<crate::extrude::Body> {
    use crate::extrude::{Body, Coloring, ExtrudeKind};
    let kind = kind.unwrap_or(if vanishing.is_some() {
        ExtrudeKind::SmallBack
    } else {
        ExtrudeKind::BackParallel
    });
    let vp = match (kind.parallel(), vanishing) {
        (true, _) => Point::ZERO,
        (false, Some(v)) if on_object => main.transform * v,
        (false, Some(v)) => v,
        (false, None) => return None,
    };
    let base = main.fill.preview_color().unwrap_or(Color::WHITE);
    let coloring = match (solid, shade_from) {
        (Some(c), _) => Coloring::Solid(c),
        (None, Some(f)) => Coloring::Shade(
            f,
            shade_to.unwrap_or_else(|| lerp_color(base, Color::BLACK, 0.5)),
        ),
        (None, None) => Coloring::Object,
    };
    Some(Body {
        kind,
        vector: depth,
        vanishing: vp,
        depth: if amount.is_finite() {
            amount.clamp(0.0, 0.99)
        } else {
            0.0
        },
        coloring,
        light_angle,
        light_intensity,
        bevel: 0.0,
        solid: Default::default(),
    })
}

/// The most steps a bevel band is cut into.
pub const MAX_BEVEL_LEVEL: u8 = 8;

fn one_level() -> u8 {
    1
}

/// The shapes a [`Effect::Bevel`] draws over `shape`: the band cut into
/// `level` steps, each split into a lit and a shaded half by the light
/// direction. Soft edge puts the band inside the outline, emboss around it.
fn bevel(shape: &Shape, effect: &Effect) -> Vec<Shape> {
    let Effect::Bevel {
        distance,
        light_angle,
        intensity,
        style,
        shadow_color,
        light_color,
        level,
        emboss_color,
    } = effect
    else {
        return Vec::new();
    };
    let path = shape.local_path();
    let width = if distance.is_finite() {
        distance.abs()
    } else {
        return Vec::new();
    };
    let steps = (*level).clamp(1, MAX_BEVEL_LEVEL) as usize;
    // Offsets of the step boundaries, from the band's outer edge in.
    let reach = |j: usize| {
        let t = j as f64 / steps as f64;
        if *style == 1 {
            width * (1.0 - t)
        } else {
            -width * t
        }
    };
    let edge = |j: usize| {
        let d = reach(j);
        if d == 0.0 {
            path.clone()
        } else {
            crate::shaping::offset(&path, d)
        }
    };
    let base = emboss_color
        .or_else(|| shape.fill.preview_color())
        .unwrap_or(Color::WHITE);
    let k = (intensity / 100.0).clamp(0.0, 1.0);
    let ldir = Vec2::new(
        light_angle.to_radians().cos(),
        light_angle.to_radians().sin(),
    );
    let mut outer = edge(0);
    let b = outer.bounding_box();
    let c = b.center();
    // Lit half: the part of the band on the light side of the centre line.
    let big = b.inflate(b.width(), b.height());
    let half = crate::effects::polygon(&[
        c + Vec2::new(-ldir.y, ldir.x) * big.width() * 2.0,
        c + Vec2::new(ldir.y, -ldir.x) * big.width() * 2.0,
        c + Vec2::new(ldir.y, -ldir.x) * big.width() * 2.0 + ldir * big.width() * 2.0,
        c + Vec2::new(-ldir.y, ldir.x) * big.width() * 2.0 + ldir * big.width() * 2.0,
    ]);
    let mut out = Vec::new();
    for i in 0..steps {
        let inner = edge(i + 1);
        if outer.elements().is_empty() || inner.elements().is_empty() {
            break;
        }
        let band = crate::shaping::overlay(&outer, &inner, crate::shaping::Op::Trim);
        // The light fades from the band's outer edge inwards.
        let strength = (k * (steps - i) as f64 / steps as f64) as f32;
        let lit = crate::shaping::overlay(&band, &half, crate::shaping::Op::Intersect);
        let dark = crate::shaping::overlay(&band, &half, crate::shaping::Op::Trim);
        for (p, col) in [
            (lit, lerp_color(base, *light_color, strength)),
            (dark, lerp_color(base, *shadow_color, strength)),
        ] {
            if p.elements().is_empty() {
                continue;
            }
            let mut s = shape.clone();
            s.effects.clear();
            s.shadow = None;
            s.stroke = None;
            s.kind = ShapeKind::Path {
                path: p,
                closed: true,
            };
            s.fill = Fill::Solid(col);
            out.push(s);
        }
        outer = inner;
    }
    out
}

/// Carry the places an object's live effects hold with the object when
/// `t` is applied to it (in the space its transform maps to): a
/// symmetry's centre and the angle of its first mirror line, so its
/// copies go with it. Left behind, the copies mirrored a move about the
/// old centre and a scale or a turn rearranged them, as if the symmetry
/// were still being edited. A move, a turn, a uniform scale and a mirror
/// carry it exactly. A stretch or a slant cannot keep the copies mirror
/// images of the changed whole; the centre goes with it and the lines
/// keep their angle (mirrored when `t` mirrors), so the copies stay mirror
/// images of the changed object.
pub fn carry_effects<'a>(effects: impl IntoIterator<Item = &'a mut Effect>, t: Affine) {
    let [a, b, c, d, _, _] = t.as_coeffs();
    let scale = a.hypot(b).max(c.hypot(d)).max(1e-12);
    let near = |x: f64, y: f64| (x - y).abs() <= 1e-9 * scale;
    // A turn and an even scale, or that and a mirror.
    let similar = (near(a, d) && near(b, -c)) || (near(a, -d) && near(b, c));
    let mirrors = a * d - b * c < 0.0;
    for e in effects {
        if let Effect::Symmetry { center, angle, .. } = e {
            let p = t * *center;
            if !(p.x.is_finite() && p.y.is_finite()) {
                continue;
            }
            if similar {
                let r = angle.to_radians();
                let dir = t * (*center + Vec2::new(r.cos(), r.sin())) - p;
                if dir.hypot() > 1e-12 && dir.hypot().is_finite() {
                    *angle = dir.y.atan2(dir.x).to_degrees();
                }
            } else if mirrors {
                *angle = -*angle;
            }
            // A line is the same turned half a turn.
            *angle = angle.rem_euclid(180.0);
            *center = p;
        }
    }
}

/// Mirror copies for the Symmetry effect. With `n` lines through `center`
/// the result is the dihedral orbit of the shape: reflections across each
/// line and the rotations they generate (2n - 1 copies).
fn symmetry(shape: &Shape, center: Point, angle: f64, lines: u8) -> Vec<Shape> {
    let n = lines.clamp(1, 12) as usize;
    let mut out = Vec::with_capacity(2 * n);
    let base = shape.clone();
    let to_center = Affine::translate(center.to_vec2());
    let from_center = Affine::translate(-center.to_vec2());
    let reflect = |theta: f64| -> Affine {
        // Reflection across a line through the origin at angle theta.
        let (s, c) = (2.0 * theta).sin_cos();
        Affine::new([c, s, s, -c, 0.0, 0.0])
    };
    let a0 = angle.to_radians();
    for k in 0..n {
        let theta = a0 + std::f64::consts::PI * k as f64 / n as f64;
        let m = to_center * reflect(theta) * from_center;
        let mut s = base.clone();
        s.effects.clear();
        s.transform = m * s.transform;
        out.push(s);
    }
    // Rotations (products of two reflections), skipping the identity.
    for k in 1..n {
        let rot = Affine::rotate(2.0 * std::f64::consts::PI * k as f64 / n as f64);
        let m = to_center * rot * from_center;
        let mut s = base.clone();
        s.effects.clear();
        s.transform = m * s.transform;
        out.push(s);
    }
    out
}

fn block_shadow(shape: &Shape, offset: Vec2, color: Color, gap: f64) -> Vec<Shape> {
    let front = shape.page_path();
    let pts = crate::effects::resample(&front, 96);
    if pts.len() < 3 {
        return Vec::new();
    }
    // Union of the swept quads and the back face, as one solid.
    let mut solid = crate::effects::polygon(&pts.iter().map(|p| *p + offset).collect::<Vec<_>>());
    for i in 0..pts.len() - 1 {
        let (a, b) = (pts[i], pts[i + 1]);
        let quad = crate::effects::polygon(&[a, b, b + offset, a + offset]);
        solid = crate::shaping::overlay(&solid, &quad, crate::shaping::Op::Weld);
    }
    if gap > 0.0 {
        let grown = crate::shaping::offset(&front, gap);
        solid = crate::shaping::overlay(&solid, &grown, crate::shaping::Op::Trim);
    }
    let mut s = shape.clone();
    s.effects.clear();
    s.shadow = None;
    s.transform = Affine::IDENTITY;
    s.stroke = None;
    s.kind = ShapeKind::Path {
        path: solid,
        closed: true,
    };
    s.fill = Fill::Solid(color);
    vec![s]
}

/// Flatten, subdivide long segments and map every point through `map`, so
/// straight edges bend with the warp. Where the warp bends hard a piece
/// of 1/64 of the object's size is halved again, so a tight bend is not
/// drawn in visible facets (#881); elsewhere the points are as many as
/// before and so are the calls to `map`.
pub(crate) fn warp(path: &BezPath, b: Rect, map: impl Fn(Point) -> Point) -> BezPath {
    let size = b.width().max(b.height());
    let step = (size / 64.0).max(size * 1e-4).max(1e-9);
    // Curves are flattened finely for the object's own size: a fixed
    // tolerance made letters, whose own space is small and scaled up by
    // their transform, come out in facets (#455).
    let tolerance = (size * 2e-5).clamp(1e-7, 0.05);
    // How far where a piece's middle maps to may lie off its mapped
    // chord.
    let deviation = (size * 1e-5).clamp(1e-8, 0.01);
    let mut out = BezPath::new();
    // The last point, where it maps to, and the subpath's start.
    let mut last: Option<(Point, Point)> = None;
    let mut start: Option<(Point, Point)> = None;
    let mut line = WarpLine::default();
    kurbo::flatten(path.elements().iter().copied(), tolerance, |el| match el {
        PathEl::MoveTo(p) => {
            let m = map(p);
            out.move_to(m);
            last = Some((p, m));
            start = last;
        }
        PathEl::LineTo(p) => match last {
            Some(from) => {
                line.run(&map, from, p, step, deviation);
                line.out.iter().for_each(|q| out.line_to(*q));
                last = Some((p, line.end));
            }
            None => {
                let m = map(p);
                out.line_to(m);
                last = Some((p, m));
            }
        },
        PathEl::ClosePath => {
            if let (Some(from), Some(s)) = (last, start) {
                if (s.0 - from.0).hypot() > 0.0 {
                    line.run(&map, from, s.0, step, deviation);
                    // The last point is the start, which the close reaches.
                    line.out.pop();
                    line.out.iter().for_each(|q| out.line_to(*q));
                }
            }
            out.close_path();
            last = start;
        }
        _ => {}
    });
    out
}

/// How many times a piece of a warped line may be halved: up to 64
/// pieces for each 1/64 of the object's size.
const WARP_HALVINGS: u32 = 6;

/// One line of a warped outline, divided and mapped; its buffers are
/// kept from line to line.
#[derive(Default)]
struct WarpLine {
    src: Vec<Point>,
    mapped: Vec<Point>,
    /// How far each mapped point lies off its neighbours' chord (0 at
    /// the ends).
    off: Vec<f64>,
    /// The mapped points after the line's start, its end included.
    out: Vec<Point>,
    /// Where the line's end maps to.
    end: Point,
}

impl WarpLine {
    /// Map the line from `from` (a point and where it maps to) to `to`
    /// in pieces of at most `step`. A piece is halved where the warp
    /// bends it more than `deviation` off its chord: judged without more
    /// calls to `map` from how far each point lies off the chord of its
    /// neighbours (on a straight line only the warp bends them; a piece
    /// strays from its chord about a quarter of that), then measured at
    /// its middle. A line of one piece (a curve's flattening) is measured
    /// only when longer than a quarter of `step`: a shorter one strays
    /// sixteen times less than the pieces of `step` drawn before.
    fn run(
        &mut self,
        map: &impl Fn(Point) -> Point,
        from: (Point, Point),
        to: Point,
        step: f64,
        deviation: f64,
    ) {
        let (l, ml) = from;
        let len = (to - l).hypot();
        let n = (len / step).ceil().clamp(1.0, 1e6) as usize;
        self.src.clear();
        self.mapped.clear();
        self.out.clear();
        self.src.push(l);
        self.mapped.push(ml);
        for i in 1..=n {
            let at = l + (to - l) * (i as f64 / n as f64);
            self.src.push(at);
            self.mapped.push(map(at));
        }
        self.end = self.mapped.last().copied().unwrap_or(ml);
        // How far each inner point lies off its neighbours' chord.
        self.off.clear();
        if n > 1 {
            self.off.push(0.0);
            for w in self.mapped.windows(3) {
                self.off.push(off_chord(w[1], w[0], w[2]));
            }
        }
        let off = &self.off;
        for i in 0..n {
            let bent = if n == 1 {
                len > step * 0.25
            } else {
                // The bend at either end of the piece, or at its inner
                // neighbour's for the first and the last piece.
                let near = [i, i + 1, i.max(1), (i + 1).min(n - 1)];
                near.iter()
                    .any(|k| off.get(*k).copied().unwrap_or(0.0) * 0.25 > deviation)
            };
            let a = (self.src[i], self.mapped[i]);
            let c = (self.src[i + 1], self.mapped[i + 1]);
            if bent {
                refine(map, a, c, deviation, WARP_HALVINGS, &mut self.out);
            } else {
                self.out.push(c.1);
            }
        }
    }
}

/// How far `p` lies off the line through `a` and `b` (off `a` when they
/// meet).
fn off_chord(p: Point, a: Point, b: Point) -> f64 {
    let chord = b - a;
    let len = chord.hypot();
    if len > 1e-12 {
        (p - a).cross(chord).abs() / len
    } else {
        (p - a).hypot()
    }
}

/// The mapped points after `from` up to `to` (each a point and where it
/// maps to), the piece halved while where its middle maps to lies more
/// than `deviation` off the mapped chord.
fn refine(
    map: &impl Fn(Point) -> Point,
    from: (Point, Point),
    to: (Point, Point),
    deviation: f64,
    halvings: u32,
    out: &mut Vec<Point>,
) {
    if halvings > 0 {
        let mid = from.0.midpoint(to.0);
        let m = map(mid);
        // Off the mapped chord, not along it: a warp that only stretches
        // a straight piece needs no more points there.
        let off = off_chord(m, from.1, to.1);
        if off.is_finite() && off > deviation {
            refine(map, from, (mid, m), deviation, halvings - 1, out);
            refine(map, (mid, m), to, deviation, halvings - 1, out);
            return;
        }
    }
    out.push(to.1);
}

/// Projective map of the bounding box onto `corners` (BL, BR, TR, TL).
/// A point through a homography from [`homography`].
pub fn map_homography(h: &[f64; 8], p: Point) -> Point {
    let w = h[6] * p.x + h[7] * p.y + 1.0;
    let w = if w.abs() < 1e-12 { 1e-12 } else { w };
    Point::new(
        (h[0] * p.x + h[1] * p.y + h[2]) / w,
        (h[3] * p.x + h[4] * p.y + h[5]) / w,
    )
}

pub fn perspective(path: &BezPath, corners: &[Point; 4]) -> BezPath {
    perspective_in(path, path.bounding_box(), corners)
}

/// As [`perspective`], with the box `b` mapped onto the corners instead
/// of the path's own bounding box.
///
/// The nodes and the control points of the outline are mapped, so a
/// segment stays one segment: a straight side stays straight (a projective
/// map keeps lines), and a curve bends as its handles do, so a side of the
/// box pulled to a point brings the handles along it to that point too
/// and the outline comes to a tip there (an ellipse in a triangle is a
/// drop, #879). Only where some of the outline lies past the horizon (a
/// frame folded over itself) is the outline flattened and mapped point by
/// point instead.
pub fn perspective_in(path: &BezPath, b: Rect, corners: &[Point; 4]) -> BezPath {
    if b.width() < 1e-9 || b.height() < 1e-9 {
        return path.clone();
    }
    let src = [
        Point::new(b.x0, b.y0),
        Point::new(b.x1, b.y0),
        Point::new(b.x1, b.y1),
        Point::new(b.x0, b.y1),
    ];
    let corners = corners_apart(corners);
    let Some(h) = homography(&src, &corners) else {
        return path.clone();
    };
    let w = |p: Point| h[6] * p.x + h[7] * p.y + 1.0;
    // The side of the horizon the box's middle is on; a point closer to
    // the horizon than a billionth of that counts as past it.
    let wc = w(b.center());
    let near = wc.abs() * 1e-9;
    let map = |p: Point| map_homography(&h, p);
    let before_horizon = path.elements().iter().all(|e| {
        let pts: &[Point] = match e {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => std::slice::from_ref(p),
            PathEl::QuadTo(a, p) => &[*a, *p][..],
            PathEl::CurveTo(a, c, p) => &[*a, *c, *p][..],
            PathEl::ClosePath => &[],
        };
        pts.iter().all(|p| w(*p) * wc.signum() > near)
    });
    if !before_horizon {
        return warp(path, b, map);
    }
    let mut out = BezPath::new();
    for e in path.elements() {
        match *e {
            PathEl::MoveTo(p) => out.move_to(map(p)),
            PathEl::LineTo(p) => out.line_to(map(p)),
            PathEl::QuadTo(a, p) => out.quad_to(map(a), map(p)),
            PathEl::CurveTo(a, c, p) => out.curve_to(map(a), map(c), map(p)),
            PathEl::ClosePath => out.close_path(),
        }
    }
    out
}

/// Perspective corners (BL, BR, TR, TL) with two neighbours that meet (a
/// side pulled to a point), or that were dragged a little past each other
/// on the way there, set a hair apart along the side across from them, in
/// its direction: the frame stays a quadrilateral a projective map can take
/// the box to, and the outline comes to a tip there (#879). Corners that
/// crossed by more than a fiftieth of the frame (a frame folded on
/// purpose) and all others stay as given.
fn corners_apart(corners: &[Point; 4]) -> [Point; 4] {
    let mut c = *corners;
    let span = c
        .iter()
        .flat_map(|p| c.iter().map(move |q| (*p - *q).hypot()))
        .fold(0.0f64, f64::max);
    if !(span.is_finite() && span > 0.0) {
        return c;
    }
    let eps = span * 1e-4;
    for i in 0..4 {
        let j = (i + 1) % 4;
        let gap = c[j] - c[i];
        if gap.hypot() > span * 0.02 {
            continue;
        }
        // The side across, in the same direction (TL to TR across from
        // BL to BR).
        let along = c[(i + 2) % 4] - c[(i + 3) % 4];
        let len = along.hypot();
        let d = if len > 1e-12 {
            along / len
        } else {
            Vec2::new(1.0, 0.0)
        };
        let crossed = gap.dot(d) < 0.0;
        if gap.hypot() > eps * 0.5 && !crossed {
            continue;
        }
        let mid = c[i].midpoint(c[j]);
        c[i] = mid - d * eps;
        c[j] = mid + d * eps;
    }
    c
}

/// Give a group's `member` the group's perspectives as it is ungrouped
/// (#674): the group (outline `group_path`, effects `group`) warps the
/// member's outline before the member's own effects run, so the member
/// takes one Perspective at the front of its stack, the group's
/// projections seen in its own space (a homography restricted to the
/// member is still one). A Perspective the member had first already is
/// folded into it, so it keeps one set of corners. Drawn as it was in the
/// group; other group effects are not carried.
pub fn take_group_perspectives(member: &mut Shape, group_path: &BezPath, group: &[Effect]) {
    let t = member.transform;
    if t.determinant().abs() < 1e-12 {
        return;
    }
    let inv = t.inverse();
    let raw = member.local_path();
    let mb = raw.bounding_box();
    if mb.width() < 1e-9 || mb.height() < 1e-9 {
        return;
    }
    let rect = |b: Rect| {
        [
            Point::new(b.x0, b.y0),
            Point::new(b.x1, b.y0),
            Point::new(b.x1, b.y1),
            Point::new(b.x0, b.y1),
        ]
    };
    // The member's box corners through each group perspective in turn,
    // each on the box of the group's outline as the one before left it.
    let mut corners = rect(mb);
    let mut path = group_path.clone();
    let mut any = false;
    for e in group {
        let Effect::Perspective { corners: to } = e else {
            continue;
        };
        let b = path.bounding_box();
        if b.width() < 1e-9 || b.height() < 1e-9 {
            continue;
        }
        let Some(h) = homography(&rect(b), to) else {
            continue;
        };
        corners = corners.map(|q| inv * map_homography(&h, t * q));
        path = perspective_in(&path, b, to);
        any = true;
    }
    if !any || !corners.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
        return;
    }
    let mut stack = member.effect_stack();
    // The member's own first perspective ran on the warped outline's box.
    let folded = match stack.first() {
        Some((Effect::Perspective { corners: own }, true)) => {
            let b = perspective_in(&raw, mb, &corners).bounding_box();
            homography(&rect(b), own).map(|h| corners.map(|g| map_homography(&h, g)))
        }
        _ => None,
    };
    match folded {
        Some(c) if c.iter().all(|p| p.x.is_finite() && p.y.is_finite()) => {
            stack[0].0 = Effect::Perspective { corners: c };
        }
        _ => stack.insert(0, (Effect::Perspective { corners }, true)),
    }
    let (effects, parked) = Shape::split_stack(stack);
    member.effects = effects;
    member.parked_effects = parked;
}

/// Solve the 8 parameters of a homography from 4 point pairs.
pub fn homography(src: &[Point; 4], dst: &[Point; 4]) -> Option<[f64; 8]> {
    let mut m = [[0.0f64; 9]; 8];
    for i in 0..4 {
        let (x, y) = (src[i].x, src[i].y);
        let (u, v) = (dst[i].x, dst[i].y);
        m[2 * i] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y, u];
        m[2 * i + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y, v];
    }
    // Gaussian elimination.
    for col in 0..8 {
        let mut pivot = col;
        for r in col + 1..8 {
            if m[r][col].abs() > m[pivot][col].abs() {
                pivot = r;
            }
        }
        if m[pivot][col].abs() < 1e-12 {
            return None;
        }
        m.swap(col, pivot);
        let p = m[col][col];
        for v in m[col].iter_mut() {
            *v /= p;
        }
        let pivot_row = m[col];
        for (r, row) in m.iter_mut().enumerate() {
            if r != col {
                let f = row[col];
                for (v, pv) in row.iter_mut().zip(pivot_row.iter()) {
                    *v -= f * pv;
                }
            }
        }
    }
    let mut h = [0.0; 8];
    for i in 0..8 {
        h[i] = m[i][8];
    }
    Some(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A symmetry goes with its object: the centre and the first line are
    /// carried by a move, a turn and a mirror, so the copies the object
    /// makes are those of the transformed whole.
    #[test]
    fn a_symmetry_is_carried_with_its_object() {
        let sym = |center: Point, angle: f64| Effect::Symmetry {
            center,
            angle,
            lines: 2,
        };
        let angle_of = |e: &Effect| match e {
            Effect::Symmetry { center, angle, .. } => (*center, *angle),
            _ => unreachable!(),
        };
        let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
        // A move: the centre moves, the line keeps its angle.
        let mut fx = vec![sym(Point::new(10.0, 5.0), 30.0)];
        carry_effects(&mut fx, Affine::translate((4.0, -2.0)));
        let (c, a) = angle_of(&fx[0]);
        assert!(
            near(c.x, 14.0) && near(c.y, 3.0) && near(a, 30.0),
            "{c:?} {a}"
        );
        // A quarter turn about the origin: the line turns with it.
        carry_effects(&mut fx, Affine::rotate(std::f64::consts::FRAC_PI_2));
        let (c, a) = angle_of(&fx[0]);
        assert!(
            near(c.x, -3.0) && near(c.y, 14.0) && near(a, 120.0),
            "{c:?} {a}"
        );
        // A mirror across the upright axis: 120 degrees becomes 60.
        carry_effects(&mut fx, Affine::scale_non_uniform(-1.0, 1.0));
        let (c, a) = angle_of(&fx[0]);
        assert!(
            near(c.x, 3.0) && near(c.y, 14.0) && near(a, 60.0),
            "{c:?} {a}"
        );
        // The copies of the carried symmetry are the transformed copies.
        let mut s = Shape::new(
            crate::ShapeId(1),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 4.0, 2.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.effects = vec![Effect::Symmetry {
            center: Point::new(4.0, 1.0),
            angle: 90.0,
            lines: 1,
        }];
        let t = Affine::translate((10.0, 3.0)) * Affine::rotate(0.7);
        let before: Vec<Rect> = evaluate(&s)
            .above
            .iter()
            .map(|c| (t * c.page_path()).bounding_box())
            .collect();
        let mut moved = s.clone();
        moved.transform_by(t);
        let after: Vec<Rect> = evaluate(&moved)
            .above
            .iter()
            .map(|c| c.page_path().bounding_box())
            .collect();
        assert_eq!(before.len(), after.len());
        for (b, a) in before.iter().zip(&after) {
            assert!(
                near(b.x0, a.x0) && near(b.y0, a.y0) && near(b.x1, a.x1) && near(b.y1, a.y1),
                "{b:?} {a:?}"
            );
        }
        // Switched off, a symmetry still goes with the object, and an
        // object taken out of a group takes the group's transform with
        // its symmetry.
        let mut off = s.clone();
        off.parked_effects = vec![crate::document::ParkedEffect {
            index: 0,
            effect: off.effects.remove(0),
        }];
        off.transform_by(Affine::translate((7.0, 0.0)));
        let Effect::Symmetry { center, .. } = off.parked_effects[0].effect else {
            unreachable!()
        };
        assert!(near(center.x, 11.0), "{center:?}");
        let mut out = s.clone();
        out.absorb(Affine::translate((0.0, 5.0)));
        let Effect::Symmetry { center, .. } = out.effects[0] else {
            unreachable!()
        };
        assert!(near(center.y, 6.0), "{center:?}");
    }
    use crate::geometry::Rect;

    fn rect_shape() -> Shape {
        let mut s = Shape::new(
            crate::ShapeId(1),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        s
    }

    fn bevelled(style: u8) -> Evaluated {
        let mut s = rect_shape();
        s.effects.push(Effect::Bevel {
            distance: 2.0,
            light_angle: 135.0,
            intensity: 50.0,
            style,
            shadow_color: Color::BLACK,
            light_color: Color::WHITE,
            level: 1,
            emboss_color: None,
        });
        evaluate(&s)
    }

    #[test]
    fn soft_edge_bevel_stays_inside_the_outline() {
        let ev = bevelled(0);
        assert!(!ev.above.is_empty());
        for part in &ev.above {
            let b = part.bounds();
            assert!(
                b.x0 >= -1e-6 && b.x1 <= 20.0 + 1e-6 && b.y0 >= -1e-6 && b.y1 <= 20.0 + 1e-6,
                "{b:?}"
            );
        }
    }

    #[test]
    fn emboss_bevel_surrounds_the_outline() {
        let soft = bevelled(0);
        let emboss = bevelled(1);
        assert!(!emboss.above.is_empty());
        let mut reach = Rect::new(0.0, 0.0, 20.0, 20.0);
        for part in &emboss.above {
            reach = reach.union(part.bounds());
        }
        // The band stands out of the object by the distance.
        assert!(
            (reach.x0 + 2.0).abs() < 0.1 && (reach.x1 - 22.0).abs() < 0.1,
            "{reach:?}"
        );
        // And it differs from the soft edge.
        assert_ne!(
            emboss.above.iter().map(|p| p.bounds()).collect::<Vec<_>>(),
            soft.above.iter().map(|p| p.bounds()).collect::<Vec<_>>()
        );
    }

    fn bevel_with(level: u8, emboss_color: Option<Color>) -> Evaluated {
        let mut s = rect_shape();
        s.effects.push(Effect::Bevel {
            distance: 3.0,
            light_angle: 135.0,
            intensity: 80.0,
            style: 0,
            shadow_color: Color::BLACK,
            light_color: Color::WHITE,
            level,
            emboss_color,
        });
        evaluate(&s)
    }

    fn colours(ev: &Evaluated) -> Vec<Color> {
        ev.above
            .iter()
            .filter_map(|p| p.fill.preview_color())
            .collect()
    }

    #[test]
    fn bevel_levels_fade_the_light_across_the_band() {
        let flat = colours(&bevel_with(1, None));
        let stepped = colours(&bevel_with(4, None));
        assert_eq!(flat.len(), 2, "one lit and one shaded tone");
        assert!(stepped.len() > flat.len());
        let mut distinct = stepped.clone();
        distinct.dedup();
        assert!(distinct.len() > 2, "{stepped:?}");
        // The steps stay inside the first level's band.
        for part in &bevel_with(4, None).above {
            let b = part.bounds();
            assert!(b.x0 >= -1e-6 && b.x1 <= 20.0 + 1e-6, "{b:?}");
        }
    }

    #[test]
    fn bevel_level_is_clamped() {
        let many = bevel_with(200, None);
        assert!(many.above.len() <= 2 * usize::from(MAX_BEVEL_LEVEL));
        assert_eq!(
            bevel_with(0, None).above.len(),
            bevel_with(1, None).above.len()
        );
    }

    #[test]
    fn the_emboss_colour_replaces_the_fill_as_the_base_of_the_band() {
        let own = colours(&bevel_with(1, None));
        let blue = Color::rgb8(0, 0, 255);
        let set = colours(&bevel_with(1, Some(blue)));
        assert_eq!(own.len(), set.len());
        assert_ne!(own, set);
    }

    #[test]
    fn older_bevels_load_as_one_level_on_the_fill() {
        let json = serde_json::to_string(&Effect::Bevel {
            distance: 2.0,
            light_angle: 135.0,
            intensity: 50.0,
            style: 0,
            shadow_color: Color::BLACK,
            light_color: Color::WHITE,
            level: 1,
            emboss_color: None,
        })
        .unwrap_or_default();
        // The new fields are left out while they hold their defaults ...
        assert!(!json.contains("emboss_color"), "{json}");
        // ... and a bevel saved without them reads back with them.
        match serde_json::from_str::<Effect>(&json.replace(",\"level\":1", "")) {
            Ok(Effect::Bevel {
                level,
                emboss_color,
                ..
            }) => assert_eq!((level, emboss_color), (1, None)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn block_shadow_keeps_corners_sharp() {
        let mut s = rect_shape();
        s.effects.push(Effect::BlockShadow {
            offset: Vec2::new(5.0, -5.0),
            color: Color::BLACK,
            gap: 0.0,
        });
        let ev = evaluate(&s);
        assert_eq!(ev.below.len(), 1);
        let b = ev.below[0].bounds();
        assert!(
            (b.x0 - 0.0).abs() < 1e-6 && (b.x1 - 25.0).abs() < 1e-6,
            "{b:?}"
        );
        assert!(
            (b.y0 + 5.0).abs() < 1e-6 && (b.y1 - 20.0).abs() < 1e-6,
            "{b:?}"
        );
        // The shadow's far corner is a true corner: a point 0.2 mm inside
        // it is covered (a rounded corner would miss it).
        let path = ev.below[0].page_path();
        let corner = Point::new(25.0 - 0.2, -5.0 + 0.2);
        let inside = crate::shaping::overlay(
            &path,
            &crate::effects::polygon(&[
                corner,
                corner + Vec2::new(0.1, 0.0),
                corner + Vec2::new(0.1, 0.1),
                corner + Vec2::new(0.0, 0.1),
            ]),
            crate::shaping::Op::Intersect,
        );
        assert!(!inside.elements().is_empty());
    }

    /// The contour offset is the distance on the page, whatever the
    /// object's scale (#754): an 80 mm inside step of a polygon scaled
    /// down lay 47 mm from its edge.
    #[test]
    fn contour_offset_is_measured_on_the_page() {
        let mut s = Shape::new(
            crate::ShapeId(1),
            ShapeKind::Polygon {
                rect: Rect::new(0.0, 0.0, 100.0, 100.0),
                points: 8,
                sharpness: 0.0,
                complex: None,
                twist: 0.0,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        s.transform = Affine::translate((30.0, 40.0))
            * Affine::scale_non_uniform(1.7, 1.7)
            * Affine::rotate(22.5f64.to_radians());
        s.effects.push(Effect::Contour {
            steps: 1,
            offset: 20.0,
            outside: false,
            to_center: false,
            fill_to: Color::WHITE,
            outline_to: None,
            color_blend: 0,
            corners: Some(crate::shaping::Join::Miter),
            accel_objects: 0.0,
            accel_colors: 0.0,
        });
        let page = |s: &Shape| (s.transform * s.local_path()).bounding_box();
        let outer = page(&s);
        let ev = evaluate(&s);
        let inner = ev
            .above
            .iter()
            .chain(&ev.below)
            .map(page)
            .next()
            .expect("one step");
        for gap in [
            inner.x0 - outer.x0,
            outer.x1 - inner.x1,
            inner.y0 - outer.y0,
            outer.y1 - inner.y1,
        ] {
            assert!((gap - 20.0).abs() < 0.2, "{gap} {outer:?} {inner:?}");
        }
        // Stretched: the same distance across and down.
        let effects = s.effects.clone();
        s = rect_shape();
        s.effects = effects;
        s.transform = Affine::scale_non_uniform(3.0, 1.0);
        if let Some(Effect::Contour { offset, .. }) = s.effects.last_mut() {
            *offset = 4.0;
        }
        let outer = page(&s);
        let ev = evaluate(&s);
        let inner = ev
            .above
            .iter()
            .chain(&ev.below)
            .map(page)
            .next()
            .expect("step");
        assert!(
            (inner.x0 - outer.x0 - 4.0).abs() < 0.05,
            "{outer:?} {inner:?}"
        );
        assert!(
            (inner.y0 - outer.y0 - 4.0).abs() < 0.05,
            "{outer:?} {inner:?}"
        );
    }

    /// An evaluation kept for the same shape is the one made then, and a
    /// change of the shape or of its effect is never answered with it
    /// (#897).
    #[test]
    fn a_kept_evaluation_follows_the_shape() {
        let contour = |offset: f64| Effect::Contour {
            steps: 2,
            offset,
            outside: true,
            to_center: false,
            fill_to: Color::WHITE,
            outline_to: None,
            color_blend: 0,
            corners: Some(crate::shaping::Join::Miter),
            accel_objects: 0.0,
            accel_colors: 0.0,
        };
        let mut s = rect_shape();
        s.effects.push(contour(2.0));
        let first = evaluate(&s);
        let again = evaluate(&s);
        assert_eq!(first.below.len(), again.below.len());
        assert_eq!(first.below[0].bounds(), again.below[0].bounds());
        // Another offset, the same object.
        s.effects = vec![contour(3.0)];
        let wider = evaluate(&s).below[0].bounds().width();
        assert!(wider > first.below[0].bounds().width() + 1.0, "{wider}");
        // Another fill, the same effect.
        let mut red = s.clone();
        red.fill = Fill::Solid(Color::rgb8(200, 0, 0));
        assert_ne!(evaluate(&red).main.fill, evaluate(&s).main.fill);
        // Moved: the steps move with it.
        let mut moved = s.clone();
        moved.transform = Affine::translate((10.0, 0.0)) * moved.transform;
        let dx = evaluate(&moved).below[0].bounds().x0 - evaluate(&s).below[0].bounds().x0;
        assert!((dx - 10.0).abs() < 1e-6, "{dx}");
    }

    #[test]
    fn contour_outside_three_steps() {
        let mut s = rect_shape();
        s.effects.push(Effect::Contour {
            steps: 3,
            offset: 2.0,
            outside: true,
            to_center: false,
            fill_to: Color::WHITE,
            outline_to: None,
            color_blend: 0,
            corners: Some(crate::shaping::Join::Miter),
            accel_objects: 0.0,
            accel_colors: 0.0,
        });
        let ev = evaluate(&s);
        assert_eq!(ev.below.len(), 3);
        let outer = ev.below[0].bounds();
        assert!((outer.width() - 32.0).abs() < 0.5, "{outer:?}");
        assert_eq!(ev.below[0].fill, Fill::Solid(Color::WHITE));
        // Mitred: the outermost step is a full square.
        let area = ev.below[0].local_path().area().abs();
        assert!((area - 32.0 * 32.0).abs() < 1.0, "{area}");
        // Accelerated: the first step lies nearer the object.
        if let Some(Effect::Contour { accel_objects, .. }) = s.effects.last_mut() {
            *accel_objects = 1.0;
        }
        let ev = evaluate(&s);
        let first = ev.below[2].bounds().width();
        assert!(first < 22.0 && first > 20.0, "{first}");
        // To the centre: steps until nothing is left, the last in the
        // end colour.
        let mut c = rect_shape();
        c.fill = Fill::Solid(Color::BLACK);
        c.effects.push(Effect::Contour {
            steps: 1,
            offset: 2.0,
            outside: false,
            to_center: true,
            fill_to: Color::WHITE,
            outline_to: None,
            color_blend: 0,
            corners: None,
            accel_objects: 0.0,
            accel_colors: 0.0,
        });
        let ev = evaluate(&c);
        assert!(ev.above.len() >= 4, "{}", ev.above.len());
        assert_eq!(
            ev.above.last().map(|s| s.fill.clone()),
            Some(Fill::Solid(Color::WHITE))
        );
        assert!((accelerate(0.5, 0.0) - 0.5).abs() < 1e-12);
        assert!(accelerate(0.5, 1.0) < 0.1 && accelerate(0.5, -1.0) > 0.8);
    }

    /// A blend's acceleration moves its middle step as far toward the
    /// start at +1 as toward the end at -1 (#1261).
    #[test]
    fn blend_acceleration_goes_as_far_either_way() {
        assert!((blend_exponent(0.0) - 1.0).abs() < 1e-12);
        let at = |a: f64| 0.5f64.powf(blend_exponent(a));
        assert!((at(1.0) - 0.05).abs() < 1e-9);
        assert!((at(-1.0) - 0.95).abs() < 1e-9);
        assert!((at(0.5) + at(-0.5) - 1.0).abs() < 1e-9);
        assert!((blend_exponent(f64::NAN) - 1.0).abs() < 1e-12);
        assert!((at(7.0) - 0.05).abs() < 1e-9, "held to the range");
        // Two squares 100 mm apart blended with one step: the step's
        // centre lands where the middle step should, either way.
        let square = |id: u64, x: f64| {
            let mut s = Shape::new(
                crate::ShapeId(id),
                ShapeKind::Rect {
                    rect: Rect::new(x, 0.0, x + 10.0, 10.0),
                    radius: 0.0,
                    corners: None,
                },
            );
            s.fill = Fill::Solid(Color::BLACK);
            s
        };
        let steps_at = |accel: f64| -> f64 {
            let start = square(1, 0.0);
            let end = square(2, 100.0);
            let steps = blend(
                &start,
                &end,
                1,
                (accel, 0.0),
                0.0,
                None,
                false,
                &BlendOptions::default(),
            );
            assert_eq!(steps.len(), 1);
            steps[0].bounds().center().x
        };
        assert!((steps_at(0.0) - 55.0).abs() < 1e-6, "{}", steps_at(0.0));
        assert!((steps_at(1.0) - 10.0).abs() < 1e-6, "{}", steps_at(1.0));
        assert!((steps_at(-1.0) - 100.0).abs() < 1e-6, "{}", steps_at(-1.0));
    }

    #[test]
    fn visual_bounds_follow_the_live_effects() {
        let mut s = rect_shape();
        let plain = s.bounds();
        assert_eq!(s.visual_bounds(), plain);
        let mut nodes = crate::envelope::default_nodes(plain);
        nodes[5].y += 10.0;
        let frame = crate::envelope::Frame::from_parts(&nodes, &[], &[]).expect("frame");
        s.effects.push(Effect::from_envelope(
            &frame,
            false,
            crate::envelope::Mapping::Putty,
        ));
        assert!(s.has_live_effects());
        // The arch rises above the rectangle; plain bounds do not see it.
        assert!((s.visual_bounds().y1 - (plain.y1 + 10.0)).abs() < 0.5);
        assert_eq!(s.bounds(), plain);
        // In a group too.
        let g = Shape::new(
            crate::ShapeId(99),
            ShapeKind::Group {
                children: vec![s.clone()],
            },
        );
        assert!(g.has_live_effects());
        assert!((g.visual_bounds().y1 - (plain.y1 + 10.0)).abs() < 0.5);
    }

    #[test]
    fn identity_envelope_keeps_geometry() {
        let mut s = rect_shape();
        s.effects.push(Effect::envelope_around(s.bounds(), false));
        let pb = evaluate(&s).main.bounds();
        assert!((pb.x1 - 20.0).abs() < 1e-6 && (pb.y1 - 20.0).abs() < 1e-6);
    }

    #[test]
    fn envelope_moves_top_centre() {
        let mut s = rect_shape();
        let mut nodes = crate::envelope::default_nodes(s.bounds());
        nodes[5].y += 10.0;
        // As older files store it: the nodes alone.
        let json = serde_json::json!({
            "effect": "envelope",
            "nodes": nodes,
            "keep_lines": false,
        });
        let e: Effect = serde_json::from_value(json).expect("older envelope");
        s.effects.push(e.clone());
        assert!((evaluate(&s).main.bounds().y1 - 30.0).abs() < 0.5);
        // Written back as it was read.
        let back = serde_json::to_value(&e).expect("json");
        assert!(back.get("handles").is_none() && back.get("corners").is_none());
        assert_eq!(back.get("mapping").and_then(|m| m.as_str()), Some("putty"));
    }

    /// An envelope on a group reshapes each object from the box of them
    /// all, each keeping its fill; the group used to become one path with
    /// the group's empty fill and vanish.
    #[test]
    fn an_envelope_on_a_group_keeps_each_object() {
        let mut left = rect_shape();
        left.id = crate::ShapeId(2);
        let mut right = rect_shape();
        right.id = crate::ShapeId(3);
        right.transform = Affine::translate((30.0, 0.0));
        right.fill = Fill::Solid(Color::rgb8(0, 0, 255));
        let mut g = Shape::new(
            crate::ShapeId(4),
            ShapeKind::Group {
                children: vec![left, right],
            },
        );
        let b = Rect::new(0.0, 0.0, 50.0, 20.0);
        let mut frame = crate::envelope::Frame::around(b);
        // The top middle raised: an arch over both.
        frame.move_node(5, Vec2::new(0.0, 10.0), crate::envelope::Mode::SingleArc);
        g.effects
            .push(Effect::from_envelope(&frame, false, Default::default()));
        let ev = evaluate(&g);
        let ShapeKind::Group { children } = &ev.main.kind else {
            panic!("still a group: {:?}", ev.main.kind);
        };
        assert_eq!(children.len(), 2);
        assert_eq!(children[1].fill, Fill::Solid(Color::rgb8(0, 0, 255)));
        let page = |s: &Shape| (s.transform * s.local_path()).bounding_box();
        let (l, r) = (page(&children[0]), page(&children[1]));
        // Each stays on its side of the box, and the arch lifts the inner
        // edges more than the outer ones.
        assert!(l.x1 <= 25.0 && r.x0 >= 25.0, "{l:?} {r:?}");
        assert!(l.y1 > 20.0 && r.y1 > 20.0, "{l:?} {r:?}");
        assert!(l.y1 < 30.0 + 1e-6, "{l:?}");
        // Perspective on a group too (#458): each object stays, drawn.
        g.effects = vec![Effect::Perspective {
            corners: [
                Point::new(0.0, 0.0),
                Point::new(50.0, 0.0),
                Point::new(40.0, 20.0),
                Point::new(10.0, 20.0),
            ],
        }];
        let ev = evaluate(&g);
        let ShapeKind::Group { children } = &ev.main.kind else {
            panic!("still a group: {:?}", ev.main.kind);
        };
        assert_eq!(children.len(), 2);
        assert!(children
            .iter()
            .all(|c| !c.local_path().elements().is_empty()));
        assert_eq!(children[1].fill, Fill::Solid(Color::rgb8(0, 0, 255)));
    }

    /// The arch on [`rect_shape`]: its top middle raised 10 mm.
    fn arch() -> Effect {
        let mut nodes = crate::envelope::default_nodes(rect_shape().bounds());
        nodes[5].y += 10.0;
        let frame = crate::envelope::Frame::from_parts(&nodes, &[], &[]).expect("frame");
        Effect::from_envelope(&frame, false, crate::envelope::Mapping::Putty)
    }

    fn see_through() -> Effect {
        Effect::Transparency {
            mask: Fill::Solid(Color::WHITE),
            merge: MergeMode::Normal,
            target: 2,
        }
    }

    fn shadow() -> Effect {
        Effect::BlockShadow {
            offset: Vec2::new(2.0, -2.0),
            color: Color::BLACK,
            gap: 0.0,
        }
    }

    fn contour() -> Effect {
        Effect::Contour {
            steps: 1,
            offset: 2.0,
            outside: true,
            to_center: false,
            fill_to: Color::WHITE,
            outline_to: None,
            color_blend: 0,
            corners: None,
            accel_objects: 0.0,
            accel_colors: 0.0,
        }
    }

    /// Convert to Curves makes an envelope part of the curve (#647): a
    /// path drawn as the enveloped object was, the other effects kept in
    /// their order, a switched-off one switched off still.
    #[test]
    fn bake_outline_warps_makes_the_envelope_the_outline() {
        let mut s = rect_shape();
        s.effects = vec![arch(), see_through()];
        s.parked_effects = vec![crate::document::ParkedEffect {
            index: 2,
            effect: shadow(),
        }];
        assert!(s.has_outline_warp());
        let baked = bake_outline_warps(&s).expect("baked");
        assert!(matches!(baked.kind, ShapeKind::Path { closed: true, .. }));
        assert_eq!(baked.effects, vec![see_through()]);
        assert_eq!(baked.parked_effects.len(), 1);
        assert_eq!(baked.parked_effects[0].index, 1);
        assert_eq!(baked.parked_effects[0].effect, shadow());
        assert_eq!(baked.fill, s.fill);
        assert!(!baked.has_outline_warp());
        assert!(bake_outline_warps(&baked).is_none());
        // Drawn as before, fitted with curves.
        assert!(same_outline(
            &baked.local_path(),
            &evaluate(&s).main.local_path(),
            0.02
        ));
        assert!(
            (baked.bounds().y1 - 30.0).abs() < 0.5,
            "{:?}",
            baked.bounds()
        );
        // Nothing to bake on a plain object.
        assert!(bake_outline_warps(&rect_shape()).is_none());
    }

    /// A warp after a contour, blend or other object drawn from the outline
    /// stays live: baking it would change what that effect draws.
    #[test]
    fn only_warps_before_a_contour_are_baked() {
        let mut s = rect_shape();
        s.effects = vec![contour(), arch()];
        assert!(!s.has_outline_warp());
        assert!(bake_outline_warps(&s).is_none());
        let twist = Effect::Distort(crate::effects::Distort::Twister {
            angle_deg: 30.0,
            center: None,
        });
        s.effects = vec![arch(), contour(), twist.clone()];
        let baked = bake_outline_warps(&s).expect("baked");
        assert_eq!(baked.effects, vec![contour(), twist]);
        // The contour is drawn around the same arch.
        assert!(same_outline(
            &evaluate(&baked).below[0].local_path(),
            &evaluate(&s).below[0].local_path(),
            0.05
        ));
        // A transparency does not draw from the outline.
        s.effects = vec![see_through(), arch()];
        let baked = bake_outline_warps(&s).expect("baked");
        assert_eq!(baked.effects, vec![see_through()]);
    }

    /// Perspective and distortion bake too, one after the other.
    #[test]
    fn perspective_and_distortion_bake_too() {
        let mut s = rect_shape();
        s.effects = vec![
            Effect::Perspective {
                corners: [
                    Point::new(0.0, 0.0),
                    Point::new(20.0, 0.0),
                    Point::new(15.0, 20.0),
                    Point::new(5.0, 20.0),
                ],
            },
            Effect::Distort(crate::effects::Distort::PushPull {
                amount: 30.0,
                center: None,
            }),
        ];
        let baked = bake_outline_warps(&s).expect("baked");
        assert!(baked.effects.is_empty() && baked.parked_effects.is_empty());
        assert!(same_outline(
            &baked.local_path(),
            &evaluate(&s).main.local_path(),
            0.02
        ));
    }

    /// An envelope on a group bakes into each object, which keeps its own
    /// look: the group stays a group of the same objects.
    #[test]
    fn an_envelope_on_a_group_bakes_into_each_object() {
        let mut left = rect_shape();
        left.id = crate::ShapeId(2);
        let mut right = rect_shape();
        right.id = crate::ShapeId(3);
        right.transform = Affine::translate((30.0, 0.0));
        right.fill = Fill::Solid(Color::rgb8(0, 0, 255));
        let mut g = Shape::new(
            crate::ShapeId(4),
            ShapeKind::Group {
                children: vec![left, right],
            },
        );
        let mut frame = crate::envelope::Frame::around(Rect::new(0.0, 0.0, 50.0, 20.0));
        frame.move_node(5, Vec2::new(0.0, 10.0), crate::envelope::Mode::SingleArc);
        g.effects
            .push(Effect::from_envelope(&frame, false, Default::default()));
        assert!(g.has_outline_warp());
        let baked = bake_outline_warps(&g).expect("baked");
        assert!(baked.effects.is_empty());
        let ShapeKind::Group { children } = &baked.kind else {
            panic!("still a group: {:?}", baked.kind);
        };
        let ids: Vec<_> = children.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![crate::ShapeId(2), crate::ShapeId(3)]);
        assert!(children
            .iter()
            .all(|c| matches!(c.kind, ShapeKind::Path { .. })));
        assert_eq!(children[1].fill, Fill::Solid(Color::rgb8(0, 0, 255)));
        assert!(
            baked.visual_bounds().y1 > 25.0,
            "{:?}",
            baked.visual_bounds()
        );
    }

    /// A picture cannot become a curve: it keeps its envelope, on its own
    /// or in a group.
    #[test]
    fn a_bitmap_keeps_its_envelope() {
        let mut b = Shape::new(
            crate::ShapeId(7),
            ShapeKind::Bitmap {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                width_px: 1,
                height_px: 1,
                png: Default::default(),
                fx: None,
            },
        );
        b.effects = vec![arch()];
        assert!(!b.has_outline_warp());
        assert!(bake_outline_warps(&b).is_none());
        let mut g = Shape::new(
            crate::ShapeId(8),
            ShapeKind::Group {
                children: vec![rect_shape(), {
                    b.effects.clear();
                    b
                }],
            },
        );
        g.effects = vec![arch()];
        assert!(!g.has_outline_warp());
        assert!(bake_outline_warps(&g).is_none());
    }

    /// The warp of the whole applied to each part gives the parts of the
    /// warped whole (the letters of a text broken apart).
    #[test]
    fn outline_warp_warps_parts_as_the_whole() {
        let square = |x: f64| {
            let mut p = BezPath::new();
            p.move_to((x, 0.0));
            p.line_to((x + 8.0, 0.0));
            p.line_to((x + 8.0, 20.0));
            p.line_to((x, 20.0));
            p.close_path();
            p
        };
        let (a, b) = (square(0.0), square(12.0));
        let mut whole = a.clone();
        whole.extend(b.elements().iter().copied());
        let mut s = Shape::new(
            crate::ShapeId(9),
            ShapeKind::Path {
                path: whole,
                closed: true,
            },
        );
        s.effects = vec![arch()];
        let warp = outline_warp(&s).expect("warp");
        let mut parts = warp.apply(&a);
        parts.extend(warp.apply(&b).elements().iter().copied());
        assert_eq!(parts.elements(), evaluate(&s).main.local_path().elements());
        // Nothing for a group, or with no warp to bake.
        s.effects = vec![contour(), arch()];
        assert!(outline_warp(&s).is_none());
        // A twister and a local zipper measure their reach on the whole:
        // three parts warped together give the parts of the warped whole
        // (each part on its own turned far more).
        let parts = [square(0.0), square(12.0), square(24.0)];
        let mut whole = BezPath::new();
        for p in &parts {
            whole.extend(p.elements().iter().copied());
        }
        s.kind = ShapeKind::Path {
            path: whole,
            closed: true,
        };
        for how in [
            crate::effects::Distort::Twister {
                angle_deg: 90.0,
                center: None,
            },
            crate::effects::Distort::Zipper {
                amplitude: 2.0,
                frequency: 4,
                center: None,
                random: false,
                smooth: false,
                local: true,
            },
        ] {
            s.effects = vec![Effect::Distort(how)];
            let warp = outline_warp(&s).expect("warp");
            let got = warp.apply_parts(&parts);
            assert_eq!(got.len(), 3);
            let mut joined = BezPath::new();
            for p in &got {
                joined.extend(p.elements().iter().copied());
            }
            assert_eq!(
                joined.elements(),
                evaluate(&s).main.local_path().elements(),
                "{how:?}"
            );
        }
    }

    /// A warp baked into a group's objects drops their B-spline control
    /// points and node types: the Shape tool would rebuild the old outline
    /// from them.
    #[test]
    fn a_baked_group_object_loses_its_spline() {
        let mut child = rect_shape();
        child.id = crate::ShapeId(2);
        child.kind = ShapeKind::Path {
            path: child.local_path(),
            closed: true,
        };
        child.spline = Some(vec![Point::new(0.0, 0.0), Point::new(20.0, 20.0)]);
        child.node_types.insert(1, crate::nodes::NodeType::Smooth);
        let mut g = Shape::new(
            crate::ShapeId(4),
            ShapeKind::Group {
                children: vec![child],
            },
        );
        g.effects = vec![arch()];
        let baked = bake_outline_warps(&g).expect("baked");
        let ShapeKind::Group { children } = &baked.kind else {
            panic!("a group");
        };
        assert!(children[0].spline.is_none());
        assert!(children[0].node_types.is_empty());
    }

    /// A curve whose own space is small and scaled up by its transform
    /// (letters are) stays smooth under an envelope: it was flattened
    /// with a fixed tolerance and came out in facets (#455).
    #[test]
    fn an_envelope_keeps_small_curves_smooth() {
        let mut s = Shape::new(
            crate::ShapeId(5),
            ShapeKind::Ellipse {
                rect: Rect::new(-0.5, -0.5, 0.5, 0.5),
                arc: None,
            },
        );
        s.transform = Affine::scale(80.0);
        let b = s.local_path().bounding_box();
        s.effects.push(Effect::from_envelope(
            &crate::envelope::Frame::around(b),
            false,
            Default::default(),
        ));
        let ev = evaluate(&s);
        let page = s.transform * ev.main.local_path();
        // Every point of the warped outline lies on the 40 mm circle, and
        // the chords between them sag less than a few hundredths of a mm
        // (the Bézier circle itself is off by about one).
        let mut worst: f64 = 0.0;
        let mut prev: Option<Point> = None;
        for el in page.elements() {
            let p = match el {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => *p,
                _ => continue,
            };
            if let Some(q) = prev {
                let mid = q.midpoint(p);
                worst = worst.max((40.0 - mid.to_vec2().hypot()).abs());
            }
            worst = worst.max((40.0 - p.to_vec2().hypot()).abs());
            prev = Some(p);
        }
        assert!(worst < 0.03, "{worst}");
    }

    #[test]
    fn perspective_maps_corners() {
        let s = rect_shape();
        let corners = [
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
            Point::new(15.0, 20.0),
            Point::new(5.0, 20.0),
        ];
        let p = perspective(&s.local_path(), &corners);
        let b = p.bounding_box();
        assert!((b.x0 - 0.0).abs() < 1e-6 && (b.x1 - 20.0).abs() < 1e-6);
        assert!((b.y1 - 20.0).abs() < 1e-6);
    }

    /// A perspective whose top corners meet (the frame a triangle) still
    /// warps the outline, and an ellipse comes to a tip there, a drop:
    /// its top node and the handles beside it all go to the point where
    /// the corners meet (#879). It was left as it was, since no projective
    /// map takes a box to a triangle; corners a hair apart, or a hair past
    /// each other, give the same.
    #[test]
    fn a_perspective_with_two_corners_together_makes_a_tip() {
        let ellipse = Shape::new(
            crate::ShapeId(3),
            ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 20.0, 40.0),
                arc: None,
            },
        )
        .local_path();
        let apex = Point::new(10.0, 60.0);
        let corners = [Point::new(-20.0, 0.0), Point::new(40.0, 0.0), apex, apex];
        let p = perspective(&ellipse, &corners);
        assert_ne!(p, ellipse);
        assert_eq!(p.elements().len(), ellipse.elements().len());
        let b = p.bounding_box();
        assert!((b.y1 - 60.0).abs() < 0.05, "{b:?}");
        assert!(b.y0.abs() < 0.05, "{b:?}");
        // The top node and both handles beside it at the tip.
        let els = p.elements();
        let k = els
            .iter()
            .position(|e| matches!(e, PathEl::CurveTo(_, _, q) if (*q - apex).hypot() < 0.05))
            .expect("a node at the tip");
        let PathEl::CurveTo(_, c2, _) = els[k] else {
            unreachable!()
        };
        let next = els[k + 1..]
            .iter()
            .chain(els.iter())
            .find_map(|e| match e {
                PathEl::CurveTo(c1, _, _) => Some(*c1),
                _ => None,
            })
            .expect("the next curve");
        assert!((c2 - apex).hypot() < 0.05, "{c2:?}");
        assert!((next - apex).hypot() < 0.05, "{next:?}");
        // Corners nearly together give nearly the same drop.
        let close = [
            corners[0],
            corners[1],
            apex + Vec2::new(0.001, 0.0),
            apex - Vec2::new(0.001, 0.0),
        ];
        let q = perspective(&ellipse, &close).bounding_box();
        for (a, b) in [(q.x0, b.x0), (q.y0, b.y0), (q.x1, b.x1), (q.y1, b.y1)] {
            assert!((a - b).abs() < 0.05, "{q:?} {b:?}");
        }
        // Dragged together by hand, the two corners end a little past
        // each other (as in the drawing of the report): the same drop, not
        // a sliver running off past the frame.
        let crossed = [
            corners[0],
            corners[1],
            apex - Vec2::new(0.05, 0.0),
            apex + Vec2::new(0.05, 0.0),
        ];
        let q = perspective(&ellipse, &crossed).bounding_box();
        for (a, b) in [(q.x0, b.x0), (q.y0, b.y0), (q.x1, b.x1), (q.y1, b.y1)] {
            assert!((a - b).abs() < 0.05, "{q:?} {b:?}");
        }
    }

    /// A frame folded over itself puts part of the outline past the
    /// horizon: it is mapped point by point there and stays finite.
    #[test]
    fn a_folded_perspective_stays_finite() {
        let ellipse = Shape::new(
            crate::ShapeId(3),
            ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                arc: None,
            },
        )
        .local_path();
        let corners = [
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
            Point::new(-30.0, 5.0),
            Point::new(0.0, 20.0),
        ];
        let p = perspective(&ellipse, &corners);
        assert!(p.elements().iter().all(|e| e
            .end_point()
            .is_none_or(|q| q.x.is_finite() && q.y.is_finite())));
    }

    /// Convert to Curves of a square in perspective gives its four
    /// corners joined by straight sides, and of a circle a few curves, not
    /// a row of points (#840, #647).
    #[test]
    fn a_baked_perspective_is_a_few_nodes() {
        let corners = [
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
            Point::new(15.0, 20.0),
            Point::new(5.0, 20.0),
        ];
        let pieces = |p: &BezPath| {
            p.elements()
                .iter()
                .filter(|e| !matches!(e, PathEl::MoveTo(_) | PathEl::ClosePath))
                .count()
        };
        let mut square = rect_shape();
        square.effects = vec![Effect::Perspective { corners }];
        let baked = bake_outline_warps(&square).expect("baked");
        let p = baked.local_path();
        assert!(pieces(&p) <= 4, "{:?}", p.elements());
        assert!(p
            .elements()
            .iter()
            .all(|e| !matches!(e, PathEl::CurveTo(..))));
        assert!(same_outline(&p, &evaluate(&square).main.local_path(), 0.02));
        let mut circle = Shape::new(
            crate::ShapeId(2),
            ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                arc: None,
            },
        );
        circle.effects = vec![Effect::Perspective { corners }];
        let baked = bake_outline_warps(&circle).expect("baked");
        let p = baked.local_path();
        assert!(pieces(&p) <= 12, "{} pieces", pieces(&p));
        assert!(same_outline(&p, &evaluate(&circle).main.local_path(), 0.05));
    }

    /// A blend asking for more steps than the Blend tool offers draws
    /// that most, so a misread or edited value cannot fill the memory
    /// (#980).
    #[test]
    fn a_blend_draws_at_most_its_maximum_steps() {
        let a = rect_shape();
        let mut b = rect_shape();
        b.transform = Affine::translate((100.0, 0.0));
        let mut s = a.clone();
        s.effects.push(Effect::Blend {
            end: Box::new(b.clone()),
            steps: u32::MAX,
            accel_objects: 0.0,
            accel_colors: 0.0,
            rotation: 0.0,
            path: None,
            rotate_on_path: false,
            end_id: Some(b.id),
            options: BlendOptions::default(),
        });
        assert_eq!(evaluate(&s).above.len(), MAX_BLEND_STEPS as usize);
    }

    #[test]
    fn blend_midpoint_is_halfway() {
        let a = rect_shape();
        let mut b = rect_shape();
        b.transform = Affine::translate((100.0, 0.0));
        b.fill = Fill::Solid(Color::rgb8(0, 0, 255));
        let mut s = a.clone();
        s.effects.push(Effect::Blend {
            end: Box::new(b.clone()),
            steps: 9,
            accel_objects: 0.0,
            accel_colors: 0.0,
            rotation: 0.0,
            path: None,
            rotate_on_path: false,
            end_id: None,
            options: BlendOptions::default(),
        });
        let ev = evaluate(&s);
        assert_eq!(ev.above.len(), 10);
        // With an end object of the drawing, the steps only.
        let mut linked = s.clone();
        if let Some(Effect::Blend { end_id, .. }) = linked.effects.last_mut() {
            *end_id = Some(b.id);
        }
        assert_eq!(evaluate(&linked).above.len(), 9);
        let mid = ev.above[4].bounds().center();
        assert!((mid.x - 60.0).abs() < 1.0, "{mid:?}");
    }

    /// The object acceleration moves the steps toward one end of a blend
    /// off a path; it only changed their sizes and turns, so the slider
    /// seemed to do nothing (#910).
    #[test]
    fn object_acceleration_moves_the_steps() {
        let a = rect_shape();
        let mut b = rect_shape();
        b.transform = Affine::translate((100.0, 0.0));
        let centres = |accel: f64| -> Vec<f64> {
            let mut s = a.clone();
            s.effects.push(Effect::Blend {
                end: Box::new(b.clone()),
                steps: 3,
                accel_objects: accel,
                accel_colors: 0.0,
                rotation: 0.0,
                path: None,
                rotate_on_path: false,
                end_id: Some(b.id),
                options: BlendOptions::default(),
            });
            evaluate(&s)
                .above
                .iter()
                .map(|x| x.bounds().center().x)
                .collect()
        };
        let even = centres(0.0);
        let x0 = a.bounds().center().x;
        for (k, x) in even.iter().enumerate() {
            let want = x0 + 100.0 * (k + 1) as f64 / 4.0;
            assert!((x - want).abs() < 1.0, "even step {k}: {x} not {want}");
        }
        // Toward the start object, then toward the end one.
        let to_start = centres(0.8);
        let to_end = centres(-0.8);
        for k in 0..3 {
            assert!(
                to_start[k] < even[k] - 2.0,
                "step {k}: {to_start:?} {even:?}"
            );
            assert!(to_end[k] > even[k] + 2.0, "step {k}: {to_end:?} {even:?}");
        }
        // Still in order between the two objects.
        assert!(to_start.windows(2).all(|w| w[0] < w[1]) && to_start[0] > x0);
        assert!(to_end.windows(2).all(|w| w[0] < w[1]) && to_end[2] < x0 + 100.0);
    }

    #[test]
    fn blend_options_change_the_steps() {
        let a = rect_shape();
        let mut b = rect_shape();
        b.transform = Affine::translate((100.0, 0.0));
        b.fill = Fill::Solid(Color::rgb8(0, 0, 255));
        let blend = |options: BlendOptions, path: Option<BezPath>, rotation: f64| {
            let mut s = a.clone();
            s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
            s.effects.push(Effect::Blend {
                end: Box::new(b.clone()),
                steps: 3,
                accel_objects: 0.5,
                accel_colors: 0.0,
                rotation,
                path,
                rotate_on_path: false,
                end_id: Some(b.id),
                options,
            });
            evaluate(&s).above
        };
        // A 100 mm path at 10 mm spacing: nine steps whatever the count.
        let mut line = BezPath::new();
        line.move_to((0.0, 50.0));
        line.line_to((100.0, 50.0));
        let spaced = BlendOptions {
            spacing: Some(10.0),
            ..Default::default()
        };
        assert_eq!(blend(spaced, Some(line.clone()), 0.0).len(), 9);
        // Without a path the spacing is ignored.
        assert_eq!(blend(spaced, None, 0.0).len(), 3);
        // Along the full path the first and last steps sit on its ends.
        let full = BlendOptions {
            full_path: true,
            ..Default::default()
        };
        let steps = blend(full, Some(line), 0.0);
        let first = steps[0].bounds().center();
        assert!((first.x - 0.0).abs() < 1.0, "{first:?}");
        // Loop: the steps turn about the middle between the objects, so
        // they leave the straight line between them.
        let looped = BlendOptions {
            loop_rotation: true,
            ..Default::default()
        };
        let plain = blend(BlendOptions::default(), None, 90.0);
        let turned = blend(looped, None, 90.0);
        let (p0, t0) = (plain[0].bounds().center(), turned[0].bounds().center());
        // (At acceleration 0.5 the first step turns by a sixth of the
        // way, so its centre moves about 3 mm.)
        assert!((p0.y - t0.y).abs() > 2.0, "{p0:?} {t0:?}");
        // Sizing: toward a bigger end object, acceleration 0.5 keeps the
        // steps smaller than the even blend makes them.
        let mut big = b.clone();
        big.transform = Affine::translate((100.0, 0.0)) * Affine::scale(3.0);
        let sized_width = |accel_sizing: bool| {
            let mut s = a.clone();
            s.effects.push(Effect::Blend {
                end: Box::new(big.clone()),
                steps: 3,
                accel_objects: 0.5,
                accel_colors: 0.0,
                rotation: 0.0,
                path: None,
                rotate_on_path: false,
                end_id: Some(big.id),
                options: BlendOptions {
                    accel_sizing,
                    ..Default::default()
                },
            });
            evaluate(&s).above[1].bounds().width()
        };
        assert!(sized_width(true) < sized_width(false) - 1e-6);
        // Around the wheel the middle colour is neither the straight mix
        // nor the same each way.
        let mid = |how: ColorBlend| {
            let o = BlendOptions {
                color_blend: how,
                ..Default::default()
            };
            blend(o, None, 0.0)[1].fill.preview_color().expect("colour")
        };
        let (d, cw, ccw) = (
            mid(ColorBlend::Direct),
            mid(ColorBlend::Clockwise),
            mid(ColorBlend::CounterClockwise),
        );
        assert_ne!(d.to_rgb8(), cw.to_rgb8());
        assert_ne!(cw.to_rgb8(), ccw.to_rgb8());
        // Red to blue clockwise (hue up) passes green; the other way magenta.
        let [r, g, _] = cw.to_rgb8();
        assert!(g > r, "{:?}", cw.to_rgb8());
        let [r, g, bl] = ccw.to_rgb8();
        assert!(r > g && bl > g, "{:?}", ccw.to_rgb8());
        // Old files read with the defaults.
        let old: Effect = serde_json::from_str(
            &serde_json::to_string(&Effect::Blend {
                end: Box::new(b.clone()),
                steps: 3,
                accel_objects: 0.0,
                accel_colors: 0.0,
                rotation: 0.0,
                path: None,
                rotate_on_path: false,
                end_id: None,
                options: BlendOptions::default(),
            })
            .expect("json"),
        )
        .expect("reads");
        assert!(matches!(old, Effect::Blend { options, .. } if options.is_default()));
        // Nodes given in each object's own coordinates (the end's moved by
        // its transform): paired from the next corner, the steps turn and
        // differ from the plain blend; the nodes are saved (#597).
        let local = a.local_path().bounding_box();
        let (p0, p1) = (
            Point::new(local.x0, local.y0),
            Point::new(local.x1, local.y0),
        );
        let p2 = Point::new(local.x1, local.y1);
        let mapped = BlendOptions {
            map_nodes: Some(BlendNodes {
                start: [p0, p1],
                end: [p1, p2],
            }),
            ..Default::default()
        };
        let plain = blend(BlendOptions::default(), None, 0.0);
        let turned = blend(mapped, None, 0.0);
        assert_eq!(plain.len(), turned.len());
        assert_ne!(plain[1].page_path(), turned[1].page_path());
        let json = serde_json::to_string(&mapped).expect("json");
        let back: BlendOptions = serde_json::from_str(&json).expect("reads");
        assert_eq!(back, mapped);
    }

    #[test]
    fn extrude_parallel_has_back_and_sides() {
        let mut s = rect_shape();
        s.effects.push(Effect::Extrude {
            depth: Vec2::new(10.0, -10.0),
            vanishing: None,
            amount: 0.0,
            shade_from: None,
            shade_to: None,
            light_angle: 135.0,
            light_intensity: 50.0,
            bevel: 0.0,
            kind: None,
            on_object: false,
            solid: None,
            space: Default::default(),
        });
        let ev = evaluate(&s);
        // The far face and the two faces that show (toward the vector),
        // each outlined.
        assert_eq!(ev.below.len(), 5);
        let back = ev.below[0].bounds();
        assert!((back.x0 - 10.0).abs() < 0.5);
    }
}

#[cfg(test)]
mod symmetry_tests {
    use super::*;
    use crate::geometry::Rect;

    #[test]
    fn one_line_mirrors_the_object_across_it() {
        let mut s = Shape::new(
            crate::ShapeId(1),
            ShapeKind::Rect {
                rect: Rect::new(10.0, 0.0, 20.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.effects.push(Effect::Symmetry {
            center: Point::new(0.0, 0.0),
            angle: 90.0,
            lines: 1,
        });
        let ev = evaluate(&s);
        assert_eq!(ev.above.len(), 1);
        let b = ev.above[0].bounds();
        assert!(
            (b.x0 + 20.0).abs() < 1e-9 && (b.x1 + 10.0).abs() < 1e-9,
            "{b:?}"
        );
        assert!((b.y0).abs() < 1e-9 && (b.y1 - 10.0).abs() < 1e-9);

        // Two lines: three copies in the other quadrants.
        s.effects[0] = Effect::Symmetry {
            center: Point::new(0.0, 0.0),
            angle: 0.0,
            lines: 2,
        };
        let ev = evaluate(&s);
        assert_eq!(ev.above.len(), 3);
    }
}
