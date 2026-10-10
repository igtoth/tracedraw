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
    },
    /// Geometry distortions (push/pull, zipper, twister).
    Distort(crate::effects::Distort),
    /// Envelope: 8 control points (4 corners + 4 edge midpoints) in local
    /// space, mapping the shape's bounding box through a Coons patch.
    Envelope { nodes: Vec<Point>, keep_lines: bool },
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

/// The eleven lens types of the target design.
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
            Lens::FishEye { .. } | Lens::Magnify { .. } | Lens::Wireframe { .. } => c,
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
            Effect::Envelope { nodes, keep_lines } => {
                let p = envelope(&main.local_path(), nodes, *keep_lines);
                set_path(&mut main, p);
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
            } => {
                let shapes = contour(
                    &main,
                    *steps,
                    *offset,
                    *outside,
                    *to_center,
                    *fill_to,
                    *outline_to,
                    *color_blend,
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
            } => {
                let shapes = blend(
                    &main,
                    end,
                    *steps,
                    *accel_objects,
                    *accel_colors,
                    *rotation,
                    path.as_ref(),
                    *rotate_on_path,
                );
                above.extend(shapes);
                above.push((**end).clone());
            }
            Effect::Extrude {
                depth,
                vanishing,
                amount,
                shade_from,
                shade_to,
                light_angle,
                light_intensity,
                bevel: _,
            } => {
                below.extend(extrude(
                    &main,
                    *depth,
                    *vanishing,
                    *amount,
                    *shade_from,
                    *shade_to,
                    *light_angle,
                    *light_intensity,
                ));
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

#[allow(clippy::too_many_arguments)]
fn contour(
    shape: &Shape,
    steps: u32,
    offset: f64,
    outside: bool,
    to_center: bool,
    fill_to: Color,
    outline_to: Option<Color>,
    color_blend: u8,
) -> Vec<Shape> {
    let base = shape.local_path();
    let from_fill = shape.fill.preview_color().unwrap_or(Color::WHITE);
    let from_outline = shape.stroke.as_ref().map(|s| s.color);
    let n = if to_center { 999 } else { steps.max(1) };
    let sign = if outside { 1.0 } else { -1.0 };
    let mut out = Vec::new();
    for i in 1..=n {
        let p = crate::shaping::offset(&base, sign * offset * i as f64);
        if p.elements().is_empty() {
            break;
        }
        let t = if to_center {
            (i as f64 / 20.0).min(1.0)
        } else {
            i as f64 / n as f64
        };
        let mut s = shape.clone();
        s.effects.clear();
        s.shadow = None;
        s.kind = ShapeKind::Path {
            path: p,
            closed: true,
        };
        if !matches!(shape.fill, Fill::None) {
            s.fill = Fill::Solid(mix(from_fill, fill_to, t, color_blend));
        }
        if let (Some(st), Some(from)) = (&mut s.stroke, from_outline) {
            st.color = mix(from, outline_to.unwrap_or(fill_to), t, color_blend);
        }
        out.push(s);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn blend(
    start: &Shape,
    end: &Shape,
    steps: u32,
    accel_objects: f64,
    accel_colors: f64,
    rotation: f64,
    path: Option<&BezPath>,
    rotate_on_path: bool,
) -> Vec<Shape> {
    let a = start.page_path();
    let b = end.page_path();
    let n = steps.max(1) as usize;
    let paths = crate::effects::blend(&a, &b, n, 120);
    let fa = start.fill.preview_color();
    let fb = end.fill.preview_color();
    let ca = start.stroke.as_ref().map(|s| s.color);
    let cb = end.stroke.as_ref().map(|s| s.color);
    let wa = start.stroke.as_ref().map(|s| s.width).unwrap_or(0.0);
    let wb = end.stroke.as_ref().map(|s| s.width).unwrap_or(0.0);
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
    let ca_center = a.bounding_box().center();
    let cb_center = b.bounding_box().center();
    let mut out = Vec::new();
    for (i, p) in paths.into_iter().enumerate() {
        let t = (i + 1) as f64 / (n + 1) as f64;
        let to = t.powf((1.0 + accel_objects).max(0.05));
        let tc = t.powf((1.0 + accel_colors).max(0.05));
        let mut s = start.clone();
        s.effects.clear();
        s.shadow = None;
        s.transform = Affine::IDENTITY;
        let mut geo = p;
        let center = geo.bounding_box().center();
        let mut extra = Affine::IDENTITY;
        if rotation != 0.0 {
            extra = Affine::rotate_about(rotation.to_radians() * to, center);
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
                let straight = ca_center + (cb_center - ca_center) * to;
                extra = Affine::translate(pos - straight) * extra;
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
            s.fill = Fill::Solid(lerp_color(x, y, tc as f32));
        }
        if let (Some(st), Some(x), Some(y)) = (&mut s.stroke, ca, cb) {
            st.color = lerp_color(x, y, tc as f32);
            st.width = wa + (wb - wa) * to;
        }
        out.push(s);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn extrude(
    shape: &Shape,
    depth: Vec2,
    vanishing: Option<Point>,
    amount: f64,
    shade_from: Option<Color>,
    shade_to: Option<Color>,
    light_angle: f64,
    light_intensity: f64,
) -> Vec<Shape> {
    let front = shape.page_path();
    // Exact vertices keep the corners sharp; very dense outlines are
    // resampled so the weld stays cheap.
    let mut pts = crate::effects::flat_points(&front);
    if pts.len() > 400 {
        pts = crate::effects::resample(&front, 400);
    }
    if pts.len() < 3 {
        return Vec::new();
    }
    let back_pts: Vec<Point> = match vanishing {
        None => pts.iter().map(|p| *p + depth).collect(),
        Some(v) => pts
            .iter()
            .map(|p| *p + (v - *p) * amount.clamp(0.0, 0.95))
            .collect(),
    };
    let base = shape.fill.preview_color().unwrap_or(Color::WHITE);
    let from = shade_from.unwrap_or(base);
    let to = shade_to.unwrap_or_else(|| lerp_color(base, Color::BLACK, 0.5));
    let light = Vec2::new(
        light_angle.to_radians().cos(),
        light_angle.to_radians().sin(),
    );
    let mut faces: Vec<(f64, Shape)> = Vec::new();
    let dir = match vanishing {
        None => depth,
        Some(v) => v - front.bounding_box().center(),
    };
    for i in 0..pts.len() - 1 {
        let (a, b) = (pts[i], pts[i + 1]);
        let (a2, b2) = (back_pts[i], back_pts[i + 1]);
        let quad = crate::effects::polygon(&[a, b, b2, a2]);
        let edge = b - a;
        let mut normal = Vec2::new(edge.y, -edge.x);
        if normal.hypot() > 1e-9 {
            normal = normal.normalize();
        }
        // Faces pointing away from the extrusion direction are hidden.
        let facing = normal.dot(dir.normalize());
        let shade = 0.55
            + 0.45 * ((normal.dot(light) + 1.0) / 2.0) * (light_intensity / 100.0).clamp(0.0, 1.0)
            + 0.45 * (1.0 - (light_intensity / 100.0).clamp(0.0, 1.0));
        let mid =
            (a + b.to_vec2() + a2.to_vec2() + b2.to_vec2().to_point().to_vec2()).to_vec2() / 4.0;
        let depth_key = mid.dot(dir.normalize());
        let mut s = shape.clone();
        s.effects.clear();
        s.shadow = None;
        s.transform = Affine::IDENTITY;
        s.kind = ShapeKind::Path {
            path: quad,
            closed: true,
        };
        let c = lerp_color(from, to, 0.5);
        s.fill = Fill::Solid(scale_color(c, shade.clamp(0.2, 1.0)));
        if let Some(st) = &mut s.stroke {
            st.width = st.width.min(0.1);
        }
        let _ = facing;
        faces.push((depth_key, s));
    }
    // Painter's order: farthest first.
    faces.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = Vec::new();
    let mut back = shape.clone();
    back.effects.clear();
    back.shadow = None;
    back.transform = Affine::IDENTITY;
    back.kind = ShapeKind::Path {
        path: crate::effects::polygon(&back_pts),
        closed: true,
    };
    back.fill = Fill::Solid(to);
    out.push(back);
    out.extend(faces.into_iter().map(|(_, s)| s));
    out
}

fn scale_color(c: Color, k: f64) -> Color {
    let [r, g, b] = c.to_rgb8();
    Color::rgb8(
        (r as f64 * k).round().clamp(0.0, 255.0) as u8,
        (g as f64 * k).round().clamp(0.0, 255.0) as u8,
        (b as f64 * k).round().clamp(0.0, 255.0) as u8,
    )
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

/// Default envelope nodes for a bounding box: corners and edge midpoints,
/// ordered clockwise from the bottom-left: BL, BM, BR, RM, TR, TM, TL, LM.
pub fn envelope_default(b: Rect) -> Vec<Point> {
    vec![
        Point::new(b.x0, b.y0),
        Point::new(b.center().x, b.y0),
        Point::new(b.x1, b.y0),
        Point::new(b.x1, b.center().y),
        Point::new(b.x1, b.y1),
        Point::new(b.center().x, b.y1),
        Point::new(b.x0, b.y1),
        Point::new(b.x0, b.center().y),
    ]
}

/// Map a path through the envelope: each edge is the quadratic through its
/// corner, midpoint and corner; the interior is a Coons patch.
pub fn envelope(path: &BezPath, nodes: &[Point], keep_lines: bool) -> BezPath {
    if nodes.len() < 8 {
        return path.clone();
    }
    let b = path.bounding_box();
    if b.width() < 1e-9 || b.height() < 1e-9 {
        return path.clone();
    }
    let q = |a: Point, m: Point, c: Point, t: f64| -> Point {
        // Quadratic through a, m (at t=0.5), c.
        let ctrl = Point::new(2.0 * m.x - 0.5 * (a.x + c.x), 2.0 * m.y - 0.5 * (a.y + c.y));
        let u = 1.0 - t;
        Point::new(
            u * u * a.x + 2.0 * u * t * ctrl.x + t * t * c.x,
            u * u * a.y + 2.0 * u * t * ctrl.y + t * t * c.y,
        )
    };
    let bottom = |u: f64| q(nodes[0], nodes[1], nodes[2], u);
    let right = |v: f64| q(nodes[2], nodes[3], nodes[4], v);
    let top = |u: f64| q(nodes[6], nodes[5], nodes[4], u);
    let left = |v: f64| q(nodes[0], nodes[7], nodes[6], v);
    let map = |p: Point| -> Point {
        let u = ((p.x - b.x0) / b.width()).clamp(0.0, 1.0);
        let v = ((p.y - b.y0) / b.height()).clamp(0.0, 1.0);
        let bu = bottom(u);
        let tu = top(u);
        let lv = left(v);
        let rv = right(v);
        let p00 = nodes[0];
        let p10 = nodes[2];
        let p01 = nodes[6];
        let p11 = nodes[4];
        let x = (1.0 - v) * bu.x + v * tu.x + (1.0 - u) * lv.x + u * rv.x
            - ((1.0 - u) * (1.0 - v) * p00.x
                + u * (1.0 - v) * p10.x
                + (1.0 - u) * v * p01.x
                + u * v * p11.x);
        let y = (1.0 - v) * bu.y + v * tu.y + (1.0 - u) * lv.y + u * rv.y
            - ((1.0 - u) * (1.0 - v) * p00.y
                + u * (1.0 - v) * p10.y
                + (1.0 - u) * v * p01.y
                + u * v * p11.y);
        Point::new(x, y)
    };
    if keep_lines {
        let mut out = BezPath::new();
        for el in path.elements() {
            out.push(match *el {
                PathEl::MoveTo(p) => PathEl::MoveTo(map(p)),
                PathEl::LineTo(p) => PathEl::LineTo(map(p)),
                PathEl::QuadTo(a, p) => PathEl::QuadTo(map(a), map(p)),
                PathEl::CurveTo(a, c, p) => PathEl::CurveTo(map(a), map(c), map(p)),
                PathEl::ClosePath => PathEl::ClosePath,
            });
        }
        return out;
    }
    warp(path, b, map)
}

/// Flatten, subdivide long segments and map every point through `map`, so
/// straight edges bend with the warp.
fn warp(path: &BezPath, b: Rect, map: impl Fn(Point) -> Point) -> BezPath {
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
fn homography(src: &[Point; 4], dst: &[Point; 4]) -> Option<[f64; 8]> {
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
        });
        let ev = evaluate(&s);
        assert_eq!(ev.below.len(), 3);
        let outer = ev.below[0].bounds();
        assert!((outer.width() - 32.0).abs() < 0.5, "{outer:?}");
        assert_eq!(ev.below[0].fill, Fill::Solid(Color::WHITE));
    }

    #[test]
    fn identity_envelope_keeps_geometry() {
        let s = rect_shape();
        let b = s.bounds();
        let p = envelope(&s.local_path(), &envelope_default(b), false);
        let pb = p.bounding_box();
        assert!((pb.x1 - 20.0).abs() < 1e-6 && (pb.y1 - 20.0).abs() < 1e-6);
    }

    #[test]
    fn envelope_moves_top_centre() {
        let s = rect_shape();
        let mut nodes = envelope_default(s.bounds());
        nodes[5].y += 10.0;
        let p = envelope(&s.local_path(), &nodes, false);
        assert!((p.bounding_box().y1 - 30.0).abs() < 0.5);
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
            end: Box::new(b),
            steps: 9,
            accel_objects: 0.0,
            accel_colors: 0.0,
            rotation: 0.0,
            path: None,
            rotate_on_path: false,
        });
        let ev = evaluate(&s);
        assert_eq!(ev.above.len(), 10);
        let mid = ev.above[4].bounds().center();
        assert!((mid.x - 60.0).abs() < 1.0, "{mid:?}");
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
        });
        let ev = evaluate(&s);
        assert!(ev.below.len() > 4);
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
