//! Bitmaps > Straighten Image: lens distortion
//! correction (barrel or pincushion), a rotation of -15 to 15 degrees,
//! vertical and horizontal perspective, quarter turns, and cropping to
//! the largest rectangle of the original proportions inside the result
//! (optionally resampled back to the original size).

use crate::fx::util::bilinear;
use image::{ImageBuffer, RgbaImage};

/// The Straighten Image settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Straighten {
    /// Correct lens distortion, -100 (barrel) to 100 (pincushion).
    pub lens: f32,
    /// Rotate image, degrees counter-clockwise, -15 to 15.
    pub angle: f32,
    /// Vertical perspective, -100 to 100: negative widens the top (a
    /// building leaning back).
    pub vertical: f32,
    /// Horizontal perspective, -100 to 100: negative enlarges the left
    /// side (taken from the right of the object).
    pub horizontal: f32,
    /// Quarter turns clockwise, applied first.
    pub turns: u8,
    /// Crop to the largest rectangle of the original proportions.
    pub crop: bool,
    /// When cropping, resample to the original pixel size.
    pub resample: bool,
}

impl Default for Straighten {
    fn default() -> Self {
        Straighten {
            lens: 0.0,
            angle: 0.0,
            vertical: 0.0,
            horizontal: 0.0,
            turns: 0,
            crop: true,
            resample: false,
        }
    }
}

impl Straighten {
    /// The settings kept in their ranges (files, scripts, typing).
    pub fn clamped(self) -> Self {
        let c = |v: f32, m: f32| if v.is_finite() { v.clamp(-m, m) } else { 0.0 };
        Straighten {
            lens: c(self.lens, 100.0),
            angle: c(self.angle, 15.0),
            vertical: c(self.vertical, 100.0),
            horizontal: c(self.horizontal, 100.0),
            turns: self.turns % 4,
            ..self
        }
    }

    /// Where a point of the corrected image (pixels from its centre)
    /// comes from in the turned original (pixels from its centre), for an
    /// original of `w` by `h` pixels: the rotation undone, then the
    /// perspective, then the lens.
    pub fn source(&self, p: (f32, f32), w: f32, h: f32) -> (f32, f32) {
        // Positive angles turn the picture counter-clockwise on screen
        // (y down).
        let (s, c) = self.angle.to_radians().sin_cos();
        let (x0, y0) = (p.0 * c - p.1 * s, p.0 * s + p.1 * c);
        let kv = self.vertical / 100.0 * 0.5;
        let kh = self.horizontal / 100.0 * 0.5;
        let (x, y) = (
            x0 * (1.0 - kv * y0 / h.max(1.0)),
            y0 * (1.0 - kh * x0 / w.max(1.0)),
        );
        let d2 = (w * w + h * h).max(1.0) / 4.0;
        let k = 1.0 + self.lens / 100.0 * 0.3 * (x * x + y * y) / d2;
        (x * k, y * k)
    }

    /// The largest scale (at most 1) of a centred rectangle of the
    /// original proportions whose every point comes from inside the
    /// original.
    pub fn crop_scale(&self, w: f32, h: f32) -> f32 {
        let inside = |s: f32| -> bool {
            let (hw, hh) = (w * s / 2.0, h * s / 2.0);
            let n = 48;
            (0..=n).all(|i| {
                let t = i as f32 / n as f32 * 2.0 - 1.0;
                [(t * hw, -hh), (t * hw, hh), (-hw, t * hh), (hw, t * hh)]
                    .iter()
                    .all(|&p| {
                        let (x, y) = self.source(p, w, h);
                        x.abs() <= w / 2.0 + 1e-3 && y.abs() <= h / 2.0 + 1e-3
                    })
            })
        };
        if inside(1.0) {
            return 1.0;
        }
        let (mut lo, mut hi) = (0.0f32, 1.0f32);
        for _ in 0..24 {
            let mid = (lo + hi) / 2.0;
            if inside(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo.max(0.01)
    }
}

/// The image turned by quarter turns clockwise.
pub fn turned(img: &RgbaImage, turns: u8) -> RgbaImage {
    match turns % 4 {
        1 => image::imageops::rotate90(img),
        2 => image::imageops::rotate180(img),
        3 => image::imageops::rotate270(img),
        _ => img.clone(),
    }
}

/// The size of a `w` by `h` image rotated by `degrees`.
fn rotated_box(w: f32, h: f32, degrees: f32) -> (f32, f32) {
    let (s, c) = degrees.to_radians().sin_cos();
    (
        (w * c.abs() + h * s.abs()).ceil().max(1.0),
        (w * s.abs() + h * c.abs()).ceil().max(1.0),
    )
}

/// The straightened image. Without cropping the canvas grows to hold the
/// rotated image and the corners are transparent.
pub fn straighten(img: &RgbaImage, st: &Straighten) -> RgbaImage {
    let st = st.clamped();
    let img = turned(img, st.turns);
    let (w, h) = (img.width() as f32, img.height() as f32);
    if w < 1.0 || h < 1.0 {
        return img;
    }
    let (ow, oh, scale) = if st.crop {
        let k = st.crop_scale(w, h);
        if st.resample {
            (w, h, k)
        } else {
            ((w * k).round().max(1.0), (h * k).round().max(1.0), 1.0)
        }
    } else {
        let (bw, bh) = rotated_box(w, h, st.angle);
        (bw, bh, 1.0)
    };
    // Nothing to do: the pixels as they are.
    if st.lens == 0.0
        && st.angle == 0.0
        && st.vertical == 0.0
        && st.horizontal == 0.0
        && ow == w
        && oh == h
    {
        return img;
    }
    ImageBuffer::from_fn(ow as u32, oh as u32, |i, j| {
        let p = (
            (i as f32 + 0.5 - ow / 2.0) * scale,
            (j as f32 + 0.5 - oh / 2.0) * scale,
        );
        let (x, y) = st.source(p, w, h);
        bilinear(&img, x + w / 2.0 - 0.5, y + h / 2.0 - 0.5, true)
    })
}

impl crate::app::App {
    /// Straighten every selected bitmap, in one undo step. Each keeps its
    /// centre and resolution, so a cropped result is smaller on the page.
    pub fn straighten_image(&mut self, st: &Straighten) {
        use tracedraw_core::document::ShapeKind;
        let mut cmds = Vec::new();
        for (id, rect, w, h, png) in self.bitmap_shapes() {
            let Some(img) = crate::bitmap_fx::decode(&png) else {
                continue;
            };
            let out = straighten(&img, st);
            let Some(png) = crate::bitmap_fx::encode(&out) else {
                continue;
            };
            // Millimetres per pixel, swapped by an odd number of turns.
            let (mut mx, mut my) = (
                rect.width() / w.max(1) as f64,
                rect.height() / h.max(1) as f64,
            );
            if st.turns % 2 == 1 {
                std::mem::swap(&mut mx, &mut my);
            }
            let c = rect.center();
            let (nw, nh) = (out.width() as f64 * mx, out.height() as f64 * my);
            cmds.push(tracedraw_core::Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Bitmap {
                    rect: tracedraw_core::Rect::new(
                        c.x - nw / 2.0,
                        c.y - nh / 2.0,
                        c.x + nw / 2.0,
                        c.y + nh / 2.0,
                    ),
                    width_px: out.width(),
                    height_px: out.height(),
                    png,
                    fx: None,
                },
            });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Straighten Image", &cmds) {
            self.status = e.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn checker(w: u32, h: u32) -> RgbaImage {
        ImageBuffer::from_fn(w, h, |x, y| {
            if (x / 4 + y / 4) % 2 == 0 {
                Rgba([250, 30, 30, 255])
            } else {
                Rgba([30, 30, 250, 255])
            }
        })
    }

    #[test]
    fn neutral_settings_and_turns_keep_pixels() {
        let img = checker(40, 24);
        assert_eq!(straighten(&img, &Straighten::default()), img);
        let quarter = straighten(
            &img,
            &Straighten {
                turns: 1,
                ..Default::default()
            },
        );
        assert_eq!(quarter.dimensions(), (24, 40));
        // Clockwise: the top left comes from the bottom left.
        assert_eq!(quarter.get_pixel(0, 0), img.get_pixel(0, 23));
        let back = straighten(
            &quarter,
            &Straighten {
                turns: 3,
                ..Default::default()
            },
        );
        assert_eq!(back, img);
    }

    #[test]
    fn rotation_crops_to_the_original_proportions_or_grows() {
        // Counter-clockwise: the source of a point right of the middle
        // lies below it.
        let st = Straighten {
            angle: 10.0,
            ..Default::default()
        };
        assert!(st.source((20.0, 0.0), 60.0, 40.0).1 > 0.0);
        let img = checker(60, 40);
        let st = Straighten {
            angle: 10.0,
            ..Default::default()
        };
        let cropped = straighten(&img, &st);
        let (cw, ch) = cropped.dimensions();
        assert!(cw < 60 && ch < 40, "{cw} x {ch}");
        assert!((cw as f32 / ch as f32 - 1.5).abs() < 0.05);
        // Every pixel of the crop comes from inside: nothing transparent.
        assert!(cropped.pixels().all(|p| p[3] == 255));
        let resampled = straighten(
            &img,
            &Straighten {
                resample: true,
                ..st
            },
        );
        assert_eq!(resampled.dimensions(), (60, 40));
        assert!(resampled.pixels().all(|p| p[3] == 255));
        let grown = straighten(&img, &Straighten { crop: false, ..st });
        assert!(grown.width() > 60 && grown.height() > 40);
        assert_eq!(grown.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn lens_and_perspective_move_pixels_the_right_way() {
        let (w, h) = (100.0, 80.0);
        // Barrel correction samples nearer the middle at the corners.
        let barrel = Straighten {
            lens: -50.0,
            ..Default::default()
        };
        let (x, y) = barrel.source((40.0, 30.0), w, h);
        assert!(x < 40.0 && y < 30.0);
        assert_eq!(barrel.source((0.0, 0.0), w, h), (0.0, 0.0));
        // Leaning back (negative): the top samples a narrower span, so it
        // widens; the bottom the other way.
        let lean = Straighten {
            vertical: -60.0,
            ..Default::default()
        };
        assert!(lean.source((30.0, -40.0), w, h).0 < 30.0);
        assert!(lean.source((30.0, 40.0), w, h).0 > 30.0);
        let side = Straighten {
            horizontal: -60.0,
            ..Default::default()
        };
        assert!(side.source((-50.0, 20.0), w, h).1 < 20.0);
        // Cropping removes the corners that would come from outside: a
        // pincushion correction reaches out, a barrel one stays inside.
        let pincushion = Straighten {
            lens: 50.0,
            ..Default::default()
        };
        assert!(pincushion.crop_scale(w, h) < 1.0 && lean.crop_scale(w, h) < 1.0);
        assert_eq!(barrel.crop_scale(w, h), 1.0);
        assert_eq!(Straighten::default().crop_scale(w, h), 1.0);
        // Out-of-range settings are clamped.
        let wild = Straighten {
            angle: 400.0,
            lens: f32::NAN,
            turns: 7,
            ..Default::default()
        }
        .clamped();
        assert_eq!((wild.angle, wild.lens, wild.turns), (15.0, 0.0, 3));
    }

    #[test]
    fn straightening_keeps_centre_and_resolution_in_one_step() {
        use tracedraw_core::document::ShapeKind;
        let mut app = crate::app::App::headless();
        let png = crate::bitmap_fx::encode(&checker(60, 40)).expect("png");
        let id = app
            .new_shape(ShapeKind::Bitmap {
                rect: tracedraw_core::Rect::new(10.0, 10.0, 70.0, 50.0),
                width_px: 60,
                height_px: 40,
                png,
                fx: None,
            })
            .expect("bitmap");
        app.select(vec![id]);
        let depth = app.engine.history_labels().0.len();
        app.straighten_image(&Straighten {
            angle: 8.0,
            turns: 1,
            ..Default::default()
        });
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let ShapeKind::Bitmap {
            rect,
            width_px,
            height_px,
            ..
        } = &app.doc().find_shape(id).expect("bitmap").kind
        else {
            panic!("not a bitmap");
        };
        // Turned: taller than wide, still one millimetre per pixel.
        assert!(height_px > width_px);
        assert!((rect.width() - *width_px as f64).abs() < 1e-6);
        assert!((rect.center().x - 40.0).abs() < 1e-6 && (rect.center().y - 30.0).abs() < 1e-6);
    }
}
