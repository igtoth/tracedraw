//! Colour and Attributes eyedroppers.
//!
//! Both tools have two phases. In the sample phase a click reads from the
//! object under the pointer; in the apply phase a click writes what was
//! sampled to the object under the pointer. Shift inverts the phase for one
//! click, and Esc clears the sample. The Colour eyedropper reads a solid
//! fill (or the pixel of a bitmap, averaged over the sample size) and
//! applies it to the fill, or to the outline with Shift. The Attributes
//! eyedropper copies the ticked groups: object properties (outline, fill,
//! text), transformations (size, rotation, position) and effects
//! (opacity, drop shadow, live effects).

use crate::app::App;
use tracedraw_core::{
    document::{Shadow, ShapeKind, TextSpan},
    geometry::{Affine, Point},
    id::ShapeId,
    live::Effect,
    Color, Command, Fill, Stroke,
};

/// Which groups the Attributes eyedropper copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttrGroups {
    pub outline: bool,
    pub fill: bool,
    pub text: bool,
    pub size: bool,
    pub rotation: bool,
    pub position: bool,
    pub effects: bool,
}

impl Default for AttrGroups {
    fn default() -> Self {
        AttrGroups {
            outline: true,
            fill: true,
            text: true,
            size: false,
            rotation: false,
            position: false,
            effects: false,
        }
    }
}

/// What the Attributes eyedropper picked up.
#[derive(Clone, Debug, PartialEq)]
pub struct SampledAttrs {
    pub fill: Fill,
    pub stroke: Option<Stroke>,
    pub text: Option<TextSpan>,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub center: Point,
    pub opacity: f64,
    pub shadow: Option<Shadow>,
    pub effects: Vec<Effect>,
}

/// Rotation of a shape's transform in radians (angle of its x axis).
pub fn rotation_of(t: Affine) -> f64 {
    let c = t.as_coeffs();
    c[1].atan2(c[0])
}

impl App {
    /// The usual hit test, else the topmost unfilled closed object whose
    /// interior contains `p`: what the paint bucket fills, and what a
    /// connector or blend drag starts from and ends on.
    pub fn hit_test_inside(&self, p: Point) -> Option<ShapeId> {
        if let Some(id) = self.hit_test(p) {
            return Some(id);
        }
        let page = self.doc().page(self.page).ok()?;
        for layer in page.layers.iter().rev() {
            if !layer.visible || layer.locked {
                continue;
            }
            for s in layer.shapes.iter().rev() {
                if s.locked || !s.visible || !s.bounds().contains(p) {
                    continue;
                }
                if crate::tools2::point_in_path(&s.page_path(), p) {
                    return Some(s.id);
                }
            }
        }
        None
    }

    /// The colour under `p`: a bitmap pixel (averaged over the sample box),
    /// else the solid fill (first stop of a fountain), else the outline.
    /// With `outline` the outline colour is read first.
    pub fn sample_color_at(&self, p: Point, outline: bool) -> Option<Color> {
        let id = self.hit_test(p)?;
        let (_, s) = self.doc().shape(id).ok()?;
        if outline {
            if let Some(st) = &s.stroke {
                return Some(st.color);
            }
        }
        if let ShapeKind::Bitmap {
            rect,
            width_px,
            height_px,
            png,
        } = &s.kind
        {
            let lp = s.transform.inverse() * p;
            if rect.contains(lp) && rect.width() > 0.0 && rect.height() > 0.0 {
                if let Some(img) = crate::bitmap_fx::decode(png) {
                    let fx = (lp.x - rect.x0) / rect.width() * *width_px as f64;
                    let fy = (rect.y1 - lp.y) / rect.height() * *height_px as f64;
                    let n = self.eyedropper_sample.max(1) as i64;
                    let half = (n - 1) / 2;
                    let (mut r, mut g, mut b, mut cnt) = (0u64, 0u64, 0u64, 0u64);
                    for dy in -half..(n - half) {
                        for dx in -half..(n - half) {
                            let x = fx.floor() as i64 + dx;
                            let y = fy.floor() as i64 + dy;
                            if x < 0 || y < 0 || x >= img.width() as i64 || y >= img.height() as i64
                            {
                                continue;
                            }
                            let px = img.get_pixel(x as u32, y as u32);
                            r += px[0] as u64;
                            g += px[1] as u64;
                            b += px[2] as u64;
                            cnt += 1;
                        }
                    }
                    if let (Some(r), Some(g), Some(b)) =
                        (r.checked_div(cnt), g.checked_div(cnt), b.checked_div(cnt))
                    {
                        return Some(Color::rgb8(r as u8, g as u8, b as u8));
                    }
                }
            }
        }
        match &s.fill {
            Fill::Solid(c) => Some(*c),
            Fill::Fountain(f) => f.stops.first().map(|st| st.color),
            _ => s.stroke.as_ref().map(|st| st.color),
        }
    }

    /// Apply a sampled colour to the object under `p`, to its outline when
    /// `outline` (a hairline is created when the object has none).
    pub fn apply_color_at(&mut self, p: Point, c: Color, outline: bool) -> bool {
        let Some(id) = self.hit_test_inside(p) else {
            return false;
        };
        if outline {
            let Ok((_, s)) = self.doc().shape(id) else {
                return false;
            };
            let mut st = s.stroke.clone().unwrap_or_default();
            st.color = c;
            self.run(Command::SetStroke {
                shapes: vec![id],
                stroke: Some(st),
            });
        } else {
            self.run(Command::SetFill {
                shapes: vec![id],
                fill: Fill::Solid(c),
            });
        }
        true
    }

    /// Read every attribute group from the object under `p`.
    pub fn sample_attrs_at(&self, p: Point) -> Option<SampledAttrs> {
        let id = self.hit_test(p)?;
        let (_, s) = self.doc().shape(id).ok()?;
        let b = s.bounds();
        let text = match &s.kind {
            ShapeKind::Text { spans, .. } => spans.first().cloned(),
            _ => None,
        };
        Some(SampledAttrs {
            fill: s.fill.clone(),
            stroke: s.stroke.clone(),
            text,
            width: b.width(),
            height: b.height(),
            rotation: rotation_of(s.transform),
            center: b.center(),
            opacity: s.opacity,
            shadow: s.shadow,
            effects: s.effects.clone(),
        })
    }

    /// Write the ticked groups of the sample to the object under `p`.
    pub fn apply_attrs_at(&mut self, p: Point, a: &SampledAttrs, g: AttrGroups) -> bool {
        let Some(id) = self.hit_test_inside(p) else {
            return false;
        };
        let Ok((_, s)) = self.doc().shape(id) else {
            return false;
        };
        let s = s.clone();
        let mut cmds = Vec::new();
        let shapes = vec![id];
        if g.fill {
            cmds.push(Command::SetFill {
                shapes: shapes.clone(),
                fill: a.fill.clone(),
            });
        }
        if g.outline {
            cmds.push(Command::SetStroke {
                shapes: shapes.clone(),
                stroke: a.stroke.clone(),
            });
        }
        if g.text {
            if let (Some(t), ShapeKind::Text { spans, .. }) = (&a.text, &s.kind) {
                let mut kind = s.kind.clone();
                if let ShapeKind::Text { spans: ns, .. } = &mut kind {
                    *ns = spans
                        .iter()
                        .map(|sp| TextSpan {
                            text: sp.text.clone(),
                            font_family: t.font_family.clone(),
                            size_pt: t.size_pt,
                            bold: t.bold,
                            italic: t.italic,
                            ..sp.clone()
                        })
                        .collect();
                }
                cmds.push(Command::SetShapeKind { shape: id, kind });
            }
        }
        if g.effects {
            cmds.push(Command::SetOpacity {
                shapes: shapes.clone(),
                opacity: a.opacity,
            });
            cmds.push(Command::SetShadow {
                shapes: shapes.clone(),
                shadow: a.shadow,
            });
            cmds.push(Command::SetEffects {
                shape: id,
                effects: a.effects.clone(),
            });
        }
        // Transformations, about the object's centre: rotate to the sampled
        // angle, scale to the sampled size, then move to the sampled position.
        let b = s.bounds();
        let c = b.center();
        let mut t = Affine::IDENTITY;
        if g.rotation {
            t = Affine::rotate_about(a.rotation - rotation_of(s.transform), c) * t;
        }
        if g.size && b.width() > 1e-9 && b.height() > 1e-9 {
            let sx = a.width / b.width();
            let sy = a.height / b.height();
            t = Affine::translate(c.to_vec2())
                * Affine::scale_non_uniform(sx, sy)
                * Affine::translate(-c.to_vec2())
                * t;
        }
        if g.position {
            t = Affine::translate(a.center - c) * t;
        }
        if t != Affine::IDENTITY {
            cmds.push(Command::TransformShapes {
                shapes: shapes.clone(),
                transform: t,
            });
        }
        if cmds.is_empty() {
            return false;
        }
        if let Err(e) = self.engine.run_batch("Copy Attributes", &cmds) {
            self.status = e.to_string();
            return false;
        }
        true
    }

    /// Clicks of both eyedropper tools. In Select mode a click samples and
    /// switches to Apply mode; in Apply mode a click applies. For the
    /// colour eyedropper Shift reads or writes the outline colour; for the
    /// attributes eyedropper Shift samples again for this click.
    pub fn eyedropper_click(&mut self, p: Point, attributes: bool, shift: bool) {
        if attributes {
            let apply = self.eyedropper_apply && self.eyedropper_attrs.is_some() && !shift;
            if apply {
                if let Some(a) = self.eyedropper_attrs.clone() {
                    let g = self.eyedropper_groups;
                    self.apply_attrs_at(p, &a, g);
                }
            } else if let Some(a) = self.sample_attrs_at(p) {
                self.eyedropper_attrs = Some(a);
                self.eyedropper_apply = true;
                self.status = crate::i18n::tr("status.sampled_attrs");
            }
        } else {
            let apply = self.eyedropper_apply && self.eyedropper_color.is_some();
            if apply {
                if let Some(c) = self.eyedropper_color {
                    self.apply_color_at(p, c, shift);
                }
            } else if let Some(c) = self.sample_color_at(p, shift) {
                self.eyedropper_color = Some(c);
                self.eyedropper_apply = true;
                self.status = crate::i18n::trf(
                    "status.sampled_color",
                    &[("c", &crate::app::color_description(c))],
                );
            }
        }
    }

    /// Esc or the Select Color button: back to sampling.
    pub fn eyedropper_reset(&mut self) {
        self.eyedropper_color = None;
        self.eyedropper_attrs = None;
        self.eyedropper_apply = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;

    fn rect(app: &mut App, r: Rect) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: r,
            radius: 0.0,
            corners: None,
        })
        .expect("shape")
    }

    #[test]
    fn colour_eyedropper_samples_fill_then_applies_to_fill_or_outline() {
        let mut app = App::headless();
        let a = rect(&mut app, Rect::new(0.0, 0.0, 10.0, 10.0));
        let b = rect(&mut app, Rect::new(20.0, 0.0, 30.0, 10.0));
        app.run(Command::SetFill {
            shapes: vec![a],
            fill: Fill::Solid(Color::rgb8(200, 10, 10)),
        });
        app.select(Vec::new());
        app.eyedropper_click(Point::new(5.0, 5.0), false, false);
        assert_eq!(app.eyedropper_color, Some(Color::rgb8(200, 10, 10)));
        // An unfilled object is hit inside too when applying.
        app.eyedropper_click(Point::new(25.0, 5.0), false, false);
        let sb = app.doc().find_shape(b).expect("b");
        assert_eq!(sb.fill, Fill::Solid(Color::rgb8(200, 10, 10)));
        // Shift+click in the apply phase colours the outline.
        app.eyedropper_click(Point::new(25.0, 5.0), false, true);
        let sb = app.doc().find_shape(b).expect("b");
        assert_eq!(
            sb.stroke.as_ref().map(|s| s.color),
            Some(Color::rgb8(200, 10, 10))
        );
    }

    #[test]
    fn attributes_eyedropper_copies_ticked_groups_only() {
        let mut app = App::headless();
        let a = rect(&mut app, Rect::new(0.0, 0.0, 10.0, 20.0));
        let b = rect(&mut app, Rect::new(40.0, 0.0, 50.0, 10.0));
        app.run(Command::SetFill {
            shapes: vec![a],
            fill: Fill::Solid(Color::rgb8(0, 0, 250)),
        });
        app.run(Command::SetOpacity {
            shapes: vec![a],
            opacity: 0.5,
        });
        app.select(Vec::new());
        app.eyedropper_click(Point::new(5.0, 5.0), true, false);
        assert!(app.eyedropper_attrs.is_some());
        // Defaults: fill and outline, no transformations, no effects.
        app.eyedropper_click(Point::new(45.0, 5.0), true, false);
        let sb = app.doc().find_shape(b).expect("b");
        assert_eq!(sb.fill, Fill::Solid(Color::rgb8(0, 0, 250)));
        assert!((sb.opacity - 1.0).abs() < 1e-9);
        assert!((sb.bounds().width() - 10.0).abs() < 1e-9);
        // Tick size and effects: b takes a's size about its own centre.
        app.eyedropper_groups.size = true;
        app.eyedropper_groups.effects = true;
        app.eyedropper_click(Point::new(45.0, 5.0), true, false);
        let sb = app.doc().find_shape(b).expect("b");
        let bb = sb.bounds();
        assert!((bb.height() - 20.0).abs() < 1e-6, "{bb:?}");
        assert!((bb.center().x - 45.0).abs() < 1e-6, "{bb:?}");
        assert!((sb.opacity - 0.5).abs() < 1e-9);
        // Shift+click samples again instead of applying.
        app.eyedropper_click(Point::new(45.0, 5.0), true, true);
        let s = app.eyedropper_attrs.as_ref().expect("sample");
        assert!((s.height - 20.0).abs() < 1e-6);
    }

    #[test]
    fn rotation_of_reads_the_transform_angle() {
        let t = Affine::rotate(0.3);
        assert!((rotation_of(t) - 0.3).abs() < 1e-9);
    }
}
