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
    /// Bevel inside the outline: an inset band lit from `light_angle`.
    Bevel {
        distance: f64,
        light_angle: f64,
        intensity: f64,
        /// Soft edge (0) or emboss (1).
        style: u8,
        shadow_color: Color,
        light_color: Color,
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
                    .map(|c| {
                        let mut c = c.clone();
                        c.transform = self.transform * c.transform;
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
pub struct Evaluated {
    pub below: Vec<Shape>,
    pub main: Shape,
    pub above: Vec<Shape>,
}

/// Evaluate every effect of `shape` in order. Geometry effects replace the
/// main shape's kind with a path; object effects add shapes.
pub fn evaluate(shape: &Shape) -> Evaluated {
    let mut main = shape.clone();
    let mut below: Vec<Shape> = Vec::new();
    let mut above: Vec<Shape> = Vec::new();
    for effect in &shape.effects {
        match effect {
            Effect::Distort(how) => {
                let p = crate::effects::distort(&main.local_path(), *how);
                set_path(&mut main, p);
            }
            Effect::Envelope {
                nodes,
                keep_lines,
                handles,
                corners,
                mapping,
            } => {
                if let Some(frame) = crate::envelope::Frame::from_parts(nodes, handles, corners) {
                    let p =
                        crate::envelope::apply(&main.local_path(), &frame, *keep_lines, *mapping);
                    set_path(&mut main, p);
                }
            }
            Effect::Perspective { corners } => {
                let p = perspective(&main.local_path(), corners);
                set_path(&mut main, p);
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
            Effect::Bevel {
                distance,
                light_angle,
                intensity,
                style: _,
                shadow_color,
                light_color,
            } => {
                above.extend(bevel(
                    &main,
                    *distance,
                    *light_angle,
                    *intensity,
                    *shadow_color,
                    *light_color,
                ));
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

fn set_path(shape: &mut Shape, path: BezPath) {
    let closed = path
        .elements()
        .iter()
        .any(|e| matches!(e, PathEl::ClosePath));
    shape.kind = ShapeKind::Path { path, closed };
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

/// `t` (0 to 1) bent by an acceleration `a` (-1 to 1): `t ^ 4^a`, so a
/// positive acceleration packs the steps toward the start and a negative
/// one toward the end.
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

/// The contour steps of `shape`: step `i` of `n` is its outline offset by
/// `n * offset * accelerate(i / n, accel_objects)` (to the centre: by
/// `i * offset` until nothing is left), coloured `accelerate(i / n,
/// accel_colors)` of the way to the end colours.
fn contour(shape: &Shape, p: &ContourParams) -> Vec<Shape> {
    let base = shape.local_path();
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
            paths.push(q);
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
            paths.push(q);
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
        _ => steps.max(1) as usize,
    };
    let paths = crate::effects::blend(&a, &b, n, 120);
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
        let to = t.powf((1.0 + accel_objects).max(0.05));
        let tc = t.powf((1.0 + accel_colors).max(0.05));
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
        s.kind = ShapeKind::Path {
            path: geo,
            closed: true,
        };
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

fn bevel(
    shape: &Shape,
    distance: f64,
    light_angle: f64,
    intensity: f64,
    shadow: Color,
    light: Color,
) -> Vec<Shape> {
    let outer = shape.local_path();
    let inner = crate::shaping::offset(&outer, -distance.abs());
    if inner.elements().is_empty() {
        return Vec::new();
    }
    // The band between outer and inner, split into lit and shaded halves
    // by the light direction: approximate with two overlapping bands.
    let band = crate::shaping::overlay(&outer, &inner, crate::shaping::Op::Trim);
    let base = shape.fill.preview_color().unwrap_or(Color::WHITE);
    let k = (intensity / 100.0).clamp(0.0, 1.0);
    let ldir = Vec2::new(
        light_angle.to_radians().cos(),
        light_angle.to_radians().sin(),
    );
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
    let lit = crate::shaping::overlay(&band, &half, crate::shaping::Op::Intersect);
    let dark = crate::shaping::overlay(&band, &half, crate::shaping::Op::Trim);
    let mut out = Vec::new();
    for (p, col) in [
        (lit, lerp_color(base, light, k as f32)),
        (dark, lerp_color(base, shadow, k as f32)),
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
    out
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
/// straight edges bend with the warp.
pub(crate) fn warp(path: &BezPath, b: Rect, map: impl Fn(Point) -> Point) -> BezPath {
    let step = (b.width().max(b.height()) / 64.0).max(0.05);
    let mut out = BezPath::new();
    let mut last: Option<Point> = None;
    let mut start: Option<Point> = None;
    kurbo::flatten(path.elements().iter().copied(), 0.05, |el| match el {
        PathEl::MoveTo(p) => {
            out.move_to(map(p));
            last = Some(p);
            start = Some(p);
        }
        PathEl::LineTo(p) => {
            if let Some(l) = last {
                let n = ((p - l).hypot() / step).ceil().max(1.0) as usize;
                for i in 1..=n {
                    let t = i as f64 / n as f64;
                    out.line_to(map(l + (p - l) * t));
                }
            } else {
                out.line_to(map(p));
            }
            last = Some(p);
        }
        PathEl::ClosePath => {
            if let (Some(l), Some(s)) = (last, start) {
                let n = ((s - l).hypot() / step).ceil().max(1.0) as usize;
                for i in 1..n {
                    let t = i as f64 / n as f64;
                    out.line_to(map(l + (s - l) * t));
                }
            }
            out.close_path();
            last = start;
        }
        _ => {}
    });
    out
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
    let b = path.bounding_box();
    if b.width() < 1e-9 || b.height() < 1e-9 {
        return path.clone();
    }
    let src = [
        Point::new(b.x0, b.y0),
        Point::new(b.x1, b.y0),
        Point::new(b.x1, b.y1),
        Point::new(b.x0, b.y1),
    ];
    let Some(h) = homography(&src, corners) else {
        return path.clone();
    };
    let map = |p: Point| -> Point {
        let w = h[6] * p.x + h[7] * p.y + 1.0;
        let w = if w.abs() < 1e-12 { 1e-12 } else { w };
        Point::new(
            (h[0] * p.x + h[1] * p.y + h[2]) / w,
            (h[3] * p.x + h[4] * p.y + h[5]) / w,
        )
    };
    warp(path, b, map)
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
        assert!((p0.y - t0.y).abs() > 3.0, "{p0:?} {t0:?}");
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
