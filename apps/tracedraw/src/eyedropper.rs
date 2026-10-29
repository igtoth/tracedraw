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

/// A decoded picture, shared with the readout's cache.
type Picture = std::sync::Arc<image::RgbaImage>;

/// What the colour eyedroppers show beside the pointer while it hovers
/// (#990): the colour under it, the palette or ink name it has, and its
/// values in the drawing's colour model. Worked out again only when the
/// pointer moves, the drawing changes or Shift is pressed or released;
/// the last picture read stays decoded, so hovering a bitmap does not
/// decode it each frame.
#[derive(Default)]
pub struct HoverReadout {
    /// The pointer (page position in 1/1000 mm), the drawing's revision,
    /// Shift, and whether a picker asked: the input the reading is for.
    key: Option<ReadoutKey>,
    /// The picture decoded last, by its bytes.
    picture: Option<(tracedraw_core::SharedBytes, Picture)>,
    /// What the readout shows, none when nothing is under the pointer.
    pub reading: Option<Reading>,
    /// How many times the colour was worked out, and pictures decoded
    /// (for the tests).
    pub computed: u64,
    pub decoded: u64,
}

impl std::fmt::Debug for HoverReadout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HoverReadout")
            .field("reading", &self.reading)
            .finish()
    }
}

/// The pointer (page position in 1/1000 mm), the zoom (for the hit
/// tolerance), the page, the drawing's revision, Shift, and whether a
/// picker asked.
type ReadoutKey = (i64, i64, i64, tracedraw_core::PageId, u64, bool, bool);

/// The readout's content.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    pub color: Color,
    /// The palette colour's or spot ink's name; empty for none.
    pub name: String,
    /// The values in the drawing's model, one per line, then the hex.
    pub lines: Vec<String>,
}

/// The readout for `c` in a drawing of `mode`: its values in that model
/// (CMYK or RGB, a spot ink as its tint) and its hex value.
pub fn reading_of(
    c: Color,
    name: String,
    mode: tracedraw_core::document::PrimaryColorMode,
) -> Reading {
    use tracedraw_core::document::PrimaryColorMode;
    let shown = match (c, mode) {
        (Color::Spot { .. }, _) | (Color::Registration, _) => c,
        (_, PrimaryColorMode::Cmyk) => c.convert_to("CMYK"),
        (_, PrimaryColorMode::Rgb) => c.convert_to("RGB"),
    };
    let mut lines: Vec<String> = crate::ui::palette::color_lines(shown)
        .lines()
        .map(str::to_string)
        .collect();
    lines.push(c.to_hex().to_uppercase());
    Reading {
        color: c,
        name,
        lines,
    }
}

impl App {
    /// The colour eyedropper's readout, while a colour eyedropper waits
    /// to sample: the Colour Eyedropper in Select mode, or a colour
    /// picker's eyedropper (`picker`). `p` is the pointer on the page.
    /// Cheap when nothing changed since the last frame.
    pub fn hover_reading(&mut self, p: Point, shift: bool, picker: bool) -> Option<&Reading> {
        let q = |v: f64| (v * 1000.0).round() as i64;
        let key = (
            q(p.x),
            q(p.y),
            q(self.view.zoom as f64),
            self.page,
            self.engine.revision(),
            shift && !picker,
            picker,
        );
        if self.eyedropper_hover.key != Some(key) {
            let mut cache = std::mem::take(&mut self.eyedropper_hover.picture);
            let mut decoded = 0;
            let mut decode = |png: &tracedraw_core::SharedBytes| -> Option<Picture> {
                if let Some((b, img)) = &cache {
                    if tracedraw_core::SharedBytes::ptr_eq(b, png) {
                        return Some(img.clone());
                    }
                }
                decoded += 1;
                let img = std::sync::Arc::new(image::load_from_memory(png).ok()?.to_rgba8());
                cache = Some((png.clone(), img.clone()));
                Some(img)
            };
            let c = if picker {
                // A picker's click takes the fill, else the outline, else
                // the paper's white.
                Some(
                    self.sample_color_with(p, false, &mut decode)
                        .or_else(|| self.sample_color_with(p, true, &mut decode))
                        .unwrap_or(Color::WHITE),
                )
            } else {
                self.sample_color_with(p, shift, &mut decode)
            };
            let reading = c.map(|c| {
                let name = self.color_name(c);
                reading_of(c, name, self.doc().metadata.primary_color_mode)
            });
            self.eyedropper_hover.picture = cache;
            self.eyedropper_hover.reading = reading;
            self.eyedropper_hover.key = Some(key);
            self.eyedropper_hover.computed += 1;
            self.eyedropper_hover.decoded += decoded;
        }
        self.eyedropper_hover.reading.as_ref()
    }

    /// The name `c` has: a spot ink's, or a palette colour's.
    pub fn color_name(&self, c: Color) -> String {
        if let Color::Spot { ink, .. } = c {
            if let Some(i) = self.doc().ink(ink) {
                return i.name.clone();
            }
        }
        crate::ui::palette::name_of(self, c)
    }

    /// Does the Colour Eyedropper wait to sample (Select mode)?
    pub fn eyedropper_sampling(&self) -> bool {
        self.tool == crate::tools::Tool::ColorEyedropper
            && !(self.eyedropper_apply && self.eyedropper_color.is_some())
    }
}

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
        for layer in self.page_layers().into_iter().rev() {
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

    /// The colour under `p`, locked objects included: a bitmap pixel
    /// (averaged over the sample box), else the solid fill (first stop of
    /// a fountain), else the outline.
    /// With `outline` the outline colour is read first.
    /// The object read is the innermost one under `p`: a group's child, a
    /// clip frame's content (#403: a picture in a frame gave nothing).
    pub fn sample_color_at(&self, p: Point, outline: bool) -> Option<Color> {
        self.sample_color_with(p, outline, &mut |png| {
            image::load_from_memory(png)
                .ok()
                .map(|i| std::sync::Arc::new(i.to_rgba8()))
        })
    }

    /// [`App::sample_color_at`] with the pictures decoded by `decode`
    /// (the hover readout keeps the last one decoded).
    fn sample_color_with(
        &self,
        p: Point,
        outline: bool,
        decode: &mut dyn FnMut(&tracedraw_core::SharedBytes) -> Option<Picture>,
    ) -> Option<Color> {
        let s = self.leaf_to_read(p)?;
        let s = &s;
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
            fx: _,
        } = &s.kind
        {
            let lp = s.transform.inverse() * p;
            if rect.contains(lp) && rect.width() > 0.0 && rect.height() > 0.0 {
                // Pictures keep the bytes they came with: PNG, JPEG and others.
                if let Some(img) = decode(png) {
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

    /// Read every attribute group from the object under `p` (locked
    /// objects included).
    pub fn sample_attrs_at(&self, p: Point) -> Option<SampledAttrs> {
        let id = self.hit_test_to_read(p)?;
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

    /// Both eyedroppers read from a locked object (and from an object on
    /// a locked layer); applying to a locked object stays blocked (#365).
    #[test]
    fn eyedroppers_read_from_locked_objects_but_never_write_to_them() {
        let mut app = App::headless();
        let a = rect(&mut app, Rect::new(0.0, 0.0, 10.0, 10.0));
        let b = rect(&mut app, Rect::new(20.0, 0.0, 30.0, 10.0));
        app.run(Command::SetFill {
            shapes: vec![a],
            fill: Fill::Solid(Color::rgb8(10, 120, 200)),
        });
        app.run(Command::SetLocked {
            shapes: vec![a, b],
            locked: true,
        });
        app.select(Vec::new());
        // The colour eyedropper.
        app.eyedropper_click(Point::new(5.0, 5.0), false, false);
        assert_eq!(app.eyedropper_color, Some(Color::rgb8(10, 120, 200)));
        // The document palette's eyedropper reads the same.
        assert_eq!(
            app.sample_color_at(Point::new(5.0, 5.0), false),
            Some(Color::rgb8(10, 120, 200))
        );
        // The attributes eyedropper.
        let attrs = app
            .sample_attrs_at(Point::new(5.0, 5.0))
            .expect("attributes");
        assert_eq!(attrs.fill, Fill::Solid(Color::rgb8(10, 120, 200)));
        // Applying to the locked object does nothing.
        let before = app.doc().find_shape(b).expect("b").fill.clone();
        app.eyedropper_click(Point::new(25.0, 5.0), false, false);
        assert_eq!(app.doc().find_shape(b).expect("b").fill, before);
        // An object on a locked layer is read as well.
        app.run(Command::SetLocked {
            shapes: vec![a, b],
            locked: false,
        });
        let layer = app.doc().pages[0].layers[0].id;
        app.run(Command::SetLayerLocked {
            layer,
            locked: true,
        });
        assert_eq!(
            app.sample_color_at(Point::new(5.0, 5.0), false),
            Some(Color::rgb8(10, 120, 200))
        );
        // Selection still ignores locked objects.
        assert_eq!(app.hit_test(Point::new(5.0, 5.0)), None);
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

    /// Pictures give their pixels: placed alone, in a group, inside a clip
    /// frame, and kept as JPEG bytes (#403: pictures gave no colour).
    #[test]
    fn pictures_give_their_pixels_wherever_they_are() {
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        // Left half red, right half blue.
        let img = image::RgbaImage::from_fn(20, 20, |x, _| {
            if x < 10 {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0, 0, 255, 255])
            }
        });
        let id = app.place_image_at(img, None, None).expect("picture");
        let b = app.doc().find_shape(id).unwrap().bounds();
        let left = Point::new(b.x0 + b.width() * 0.25, b.center().y);
        let right = Point::new(b.x0 + b.width() * 0.75, b.center().y);
        let red = Color::rgb8(255, 0, 0);
        let blue = Color::rgb8(0, 0, 255);
        assert_eq!(app.sample_color_at(left, false), Some(red));
        assert_eq!(app.sample_color_at(right, false), Some(blue));
        // In a group with a rectangle beside it.
        let r = rect(&mut app, Rect::new(b.x1 + 10.0, b.y0, b.x1 + 20.0, b.y1));
        app.select(vec![id, r]);
        app.group_selection();
        assert_eq!(app.sample_color_at(left, false), Some(red));
        assert_eq!(app.sample_color_at(right, false), Some(blue));
        app.undo();
        // Inside a clip frame covering it.
        let frame = rect(&mut app, b.inflate(2.0, 2.0));
        app.run(Command::PlaceInside {
            contents: vec![id],
            frame,
        });
        assert!(
            matches!(
                app.doc().find_shape(frame).map(|s| &s.kind),
                Some(ShapeKind::ClipFrame { .. })
            ),
            "placed inside"
        );
        assert_eq!(app.sample_color_at(left, false), Some(red));
        assert_eq!(app.sample_color_at(right, false), Some(blue));
        // The frame out again, and the rectangle that was to be it.
        app.undo();
        app.undo();
        // A picture kept as JPEG bytes (as imported files carry them).
        let mut jpeg = Vec::new();
        let green = image::RgbImage::from_pixel(16, 16, image::Rgb([0, 200, 0]));
        image::DynamicImage::ImageRgb8(green)
            .write_to(
                &mut std::io::Cursor::new(&mut jpeg),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        if let Some(ShapeKind::Bitmap {
            rect,
            width_px,
            height_px,
            fx,
            ..
        }) = app.doc().find_shape(id).map(|s| s.kind.clone())
        {
            app.run(Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Bitmap {
                    rect,
                    width_px,
                    height_px,
                    png: jpeg.into(),
                    fx,
                },
            });
        }
        let c = app
            .sample_color_at(left, false)
            .expect("a colour")
            .to_rgb8();
        assert!(c[0] < 20 && c[1] > 180 && c[2] < 20, "{c:?}");
    }

    /// The readout names a palette colour and gives its values in the
    /// drawing's model and its hex; it is worked out again only when the
    /// pointer moves or the drawing changes, and a picture hovered is
    /// decoded once (#990).
    #[test]
    fn the_hover_readout_names_the_colour_and_keeps_its_work() {
        use tracedraw_core::document::PrimaryColorMode;
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        let (name, c) = app
            .palette
            .iter()
            .find(|(n, c)| !n.is_empty() && matches!(c, Color::Cmyk { .. }))
            .cloned()
            .expect("a named CMYK colour");
        let id = rect(&mut app, Rect::new(10.0, 10.0, 50.0, 40.0));
        app.select(vec![id]);
        app.apply_fill(Fill::Solid(c));
        let inside = Point::new(30.0, 25.0);
        let r = app
            .hover_reading(inside, false, false)
            .cloned()
            .expect("read");
        assert_eq!(r.color, c);
        assert_eq!(r.name, name);
        assert!(r.lines[0].starts_with("C:"), "{:?}", r.lines);
        assert_eq!(r.lines.len(), 5, "C, M, Y, K, hex: {:?}", r.lines);
        assert_eq!(r.lines[4], c.to_hex().to_uppercase());
        // In an RGB drawing the values are RGB.
        let rgb = reading_of(c, String::new(), PrimaryColorMode::Rgb);
        assert!(rgb.lines[0].starts_with("R:"), "{:?}", rgb.lines);
        // A colour of no palette has no name.
        let odd = reading_of(Color::rgb8(1, 2, 3), String::new(), PrimaryColorMode::Cmyk);
        assert!(odd.lines[0].starts_with("C:"));
        assert_eq!(odd.lines.last().map(String::as_str), Some("#010203"));
        // Nothing worked out again while nothing changes.
        let n = app.eyedropper_hover.computed;
        for _ in 0..5 {
            app.hover_reading(inside, false, false);
        }
        assert_eq!(app.eyedropper_hover.computed, n);
        app.hover_reading(Point::new(31.0, 25.0), false, false);
        assert_eq!(app.eyedropper_hover.computed, n + 1, "the pointer moved");
        app.apply_fill(Fill::Solid(Color::rgb8(0, 255, 0)));
        let r = app.hover_reading(Point::new(31.0, 25.0), false, false);
        assert_eq!(r.map(|r| r.color), Some(Color::rgb8(0, 255, 0)), "edited");
        // Off every object: nothing to sample, but a picker gets white.
        assert!(app
            .hover_reading(Point::new(150.0, 200.0), false, false)
            .is_none());
        let r = app.hover_reading(Point::new(150.0, 200.0), false, true);
        assert_eq!(r.map(|r| r.color), Some(Color::WHITE));
        // A picture: decoded once however the pointer moves over it.
        let img = image::RgbaImage::from_pixel(20, 20, image::Rgba([0, 0, 255, 255]));
        let pic = app.place_image_at(img, None, None).expect("picture");
        let b = app.doc().find_shape(pic).unwrap().bounds();
        let before = app.eyedropper_hover.decoded;
        for i in 0..10 {
            let p = Point::new(b.x0 + b.width() * (0.1 + i as f64 * 0.08), b.center().y);
            let r = app.hover_reading(p, false, false);
            assert_eq!(r.map(|r| r.color), Some(Color::rgb8(0, 0, 255)));
        }
        assert_eq!(app.eyedropper_hover.decoded, before + 1);
    }

    #[test]
    fn rotation_of_reads_the_transform_angle() {
        let t = Affine::rotate(0.3);
        assert!((rotation_of(t) - 0.3).abs() < 1e-9);
    }
}
