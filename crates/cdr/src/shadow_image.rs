//! The picture a drop shadow is written as when the caller passed none
//! ([`crate::write::Previews::shadows`]; a drawing written with
//! [`crate::write::document_to_cdr_version`], or a shadow the renderer
//! gave no picture for): the object's outline, moved or sheared as the shadow's
//! settings say, filled with the shadow colour, blurred by its feathering
//! and shaded with its opacity and fade, as `docs/behavior/drop-shadow.md`
//! describes the renderer's drawing. The silhouette is the outline of the
//! object (of a group, the outlines of its objects), not what its fills
//! show; the application and the command line pass the renderer's
//! pictures, which draw the exact silhouette.

use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};
use tracedraw_core::geometry::{Affine, BezPath, PathEl, Rect, Shape as _};
use tracedraw_core::{Shadow, Shape};

/// The most pixels a shadow picture may have (the renderer's limit too).
const MAX_PIXELS: f64 = 64.0e6;

/// The drop shadow of `shape` alone as an image at `dpi`, with the page
/// rectangle it covers; `None` when the shape has no shadow or it covers
/// nothing. The same contract as the renderer's shadow image.
pub fn shadow_image(shape: &Shape, dpi: f64) -> Option<(Rect, Pixmap)> {
    let sh = shape.shadow.as_ref()?;
    let path = shape.page_path();
    if path.elements().is_empty() || !(dpi.is_finite() && dpi > 0.0) {
        return None;
    }
    let b = path.bounding_box();
    if !(b.width().is_finite() && b.height().is_finite()) {
        return None;
    }
    let area = sh.bounds(b);
    let zoom = dpi / 25.4;
    let w = (area.width() * zoom).ceil();
    let h = (area.height() * zoom).ceil();
    if !(w >= 1.0 && h >= 1.0 && w * h <= MAX_PIXELS) {
        return None;
    }
    let mut pm = Pixmap::new(w as u32, h as u32)?;
    // Page (y up) to the image (y down), the area's top left at 0, 0.
    let screen = Affine::new([zoom, 0.0, 0.0, -zoom, -area.x0 * zoom, area.y1 * zoom]);
    let shifted = screen * (sh.silhouette_transform(b) * path);
    let sk = to_sk_path(&shifted)?;
    let paint = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    pm.fill_path(&sk, &paint, FillRule::Winding, Transform::identity(), None);
    shade(&mut pm, sh, b, screen, zoom);
    let exact = Rect::new(area.x0, area.y1 - h / zoom, area.x0 + w / zoom, area.y1);
    Some((exact, pm))
}

fn to_sk_path(path: &BezPath) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let f = |v: f64| v as f32;
    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => pb.move_to(f(p.x), f(p.y)),
            PathEl::LineTo(p) => pb.line_to(f(p.x), f(p.y)),
            PathEl::QuadTo(c, p) => pb.quad_to(f(c.x), f(c.y), f(p.x), f(p.y)),
            PathEl::CurveTo(c1, c2, p) => {
                pb.cubic_to(f(c1.x), f(c1.y), f(c2.x), f(c2.y), f(p.x), f(p.y))
            }
            PathEl::ClosePath => pb.close(),
        }
    }
    pb.finish()
}

/// Blur the coverage and turn it into the shadow's colour at the opacity
/// its feathering, fade and opacity give each pixel.
fn shade(layer: &mut Pixmap, sh: &Shadow, b: Rect, screen: Affine, zoom: f64) {
    use tracedraw_core::shadow::FeatherDirection;
    let blur = if sh.blur.is_finite() { sh.blur } else { 0.0 };
    let radius = (blur * zoom).round() as i32;
    let feathered = radius > 0;
    if feathered {
        match sh.feather_direction {
            FeatherDirection::Average => {
                box_blur(layer, ((radius as f64) * 1.7).round().min(110.0) as i32, 1)
            }
            _ => box_blur(layer, radius.min(64), 3),
        }
    }
    let fade = sh.fade_axis(b).map(|(a, e, f)| {
        let (a, e) = (screen * a, screen * e);
        let d = e - a;
        (a, d, d.hypot2().max(1e-9), f)
    });
    let opacity = sh.opacity.clamp(0.0, 1.0);
    let [cr, cg, cb] = sh.color.to_rgb8();
    let width = layer.width() as usize;
    for (i, px) in layer.pixels_mut().iter_mut().enumerate() {
        let cov = px.alpha();
        if cov == 0 {
            continue;
        }
        let mut a = cov as f64 / 255.0;
        if feathered {
            a = sh.feather_value(a);
        }
        if let Some((pa, d, len2, f)) = fade {
            let x = (i % width) as f64 + 0.5;
            let y = (i / width) as f64 + 0.5;
            let t = ((x - pa.x) * d.x + (y - pa.y) * d.y) / len2;
            a *= 1.0 - f * t.clamp(0.0, 1.0);
        }
        let a = (a * opacity).clamp(0.0, 1.0);
        *px = tiny_skia::ColorU8::from_rgba(cr, cg, cb, (a * 255.0).round() as u8).premultiply();
    }
}

/// Box blur of `passes` passes each way on premultiplied RGBA, the window
/// clamped at the edges.
fn box_blur(pm: &mut Pixmap, radius: i32, passes: usize) {
    let w = pm.width() as usize;
    let h = pm.height() as usize;
    if w == 0 || h == 0 {
        return;
    }
    let r = radius.max(1) as usize;
    let data = pm.data_mut();
    let mut tmp = vec![0u8; data.len()];
    for _ in 0..passes {
        blur_pass(data, &mut tmp, w, h, r, true);
        blur_pass(&tmp, data, w, h, r, false);
    }
}

fn blur_pass(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize, horizontal: bool) {
    let (outer, inner) = if horizontal { (h, w) } else { (w, h) };
    let idx = |o: usize, i: usize| -> usize {
        if horizontal {
            (o * w + i) * 4
        } else {
            (i * w + o) * 4
        }
    };
    let span = (2 * r + 1) as u32;
    let at = |buf: &[u8], p: usize, c: usize| buf.get(p + c).copied().unwrap_or(0) as u32;
    for o in 0..outer {
        let mut acc = [0u32; 4];
        for i in 0..=r {
            let p = idx(o, i.min(inner - 1));
            for (c, a) in acc.iter_mut().enumerate() {
                *a += at(src, p, c);
            }
        }
        for _ in 0..r {
            let p = idx(o, 0);
            for (c, a) in acc.iter_mut().enumerate() {
                *a += at(src, p, c);
            }
        }
        for i in 0..inner {
            let d = idx(o, i);
            for (c, a) in acc.iter().enumerate() {
                if let Some(v) = dst.get_mut(d + c) {
                    *v = (a / span) as u8;
                }
            }
            let add = idx(o, (i + r + 1).min(inner - 1));
            let sub = idx(o, i.saturating_sub(r));
            for (c, a) in acc.iter_mut().enumerate() {
                *a = (*a + at(src, add, c)).saturating_sub(at(src, sub, c));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Vec2;
    use tracedraw_core::{Color, Fill, ShapeKind};

    fn square() -> Shape {
        let mut s = Shape::new(
            tracedraw_core::id::IdSource::default().shape(),
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 30.0, 30.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(200, 0, 0));
        s
    }

    #[test]
    fn a_flat_shadow_lies_where_the_offset_puts_it() {
        let mut s = square();
        s.shadow = Some(Shadow {
            offset: Vec2::new(5.0, -5.0),
            blur: 0.0,
            opacity: 1.0,
            ..Shadow::default()
        });
        let (r, pm) = shadow_image(&s, 25.4).expect("image");
        // The square moved right and down: 15..35 by 5..25.
        assert!(
            (r.x0 - 15.0).abs() < 1.0 && (r.x1 - 35.0).abs() < 1.1,
            "{r:?}"
        );
        assert!(
            (r.y0 - 5.0).abs() < 1.0 && (r.y1 - 25.0).abs() < 1.1,
            "{r:?}"
        );
        let mid = pm.pixel(pm.width() / 2, pm.height() / 2).expect("pixel");
        assert_eq!(mid.alpha(), 255);
        assert_eq!((mid.red(), mid.green(), mid.blue()), (0, 0, 0));
    }

    #[test]
    fn feathering_widens_the_picture_and_softens_its_edge() {
        let mut s = square();
        s.shadow = Some(Shadow {
            offset: Vec2::ZERO,
            blur: 3.0,
            opacity: 0.5,
            ..Shadow::default()
        });
        let (r, pm) = shadow_image(&s, 25.4).expect("image");
        // Grown by three times the feathering on each side.
        assert!(
            (r.x0 - 1.0).abs() < 1.0 && (r.x1 - 39.0).abs() < 1.1,
            "{r:?}"
        );
        let mid = pm.pixel(pm.width() / 2, pm.height() / 2).expect("pixel");
        assert!((120..=135).contains(&mid.alpha()), "{}", mid.alpha());
        let edge = pm.pixel(1, pm.height() / 2).expect("pixel");
        assert!(edge.alpha() < 40, "{}", edge.alpha());
    }

    #[test]
    fn no_shadow_or_nothing_to_draw_gives_no_picture() {
        assert!(shadow_image(&square(), 100.0).is_none());
        let mut s = square();
        s.shadow = Some(Shadow::default());
        assert!(shadow_image(&s, f64::NAN).is_none());
    }
}
