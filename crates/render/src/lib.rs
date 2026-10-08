//! CPU rasterizer. Draws a page region into an RGBA pixmap with tiny-skia:
//! correct non-convex fills, fountain fills, outline widths, caps, joins,
//! dashes, hairlines and anti-aliasing. The GPU renderer, when it exists,
//! must match this one.

use tiny_skia::{
    Color as SkColor, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint,
    Path as SkPath, PathBuilder, Pixmap, RadialGradient, SpreadMode, Stroke as SkStroke,
    StrokeDash, Transform,
};
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, BezPath, PathEl, Point, Rect},
    Color, Document, Fill, LineCap as LCap, LineJoin as LJoin, PageId, ShapeId, Stroke,
};

/// Page (mm, Y up) to pixel mapping for one render.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewTransform {
    /// Pixels per millimetre.
    pub zoom: f64,
    /// Pixel position of the page origin (bottom-left corner) inside the target.
    pub origin_x: f64,
    pub origin_y: f64,
}

impl ViewTransform {
    pub fn affine(&self) -> Affine {
        Affine::new([
            self.zoom,
            0.0,
            0.0,
            -self.zoom,
            self.origin_x,
            self.origin_y,
        ])
    }
}

/// Objects to draw displaced by a transform (live drag preview).
#[derive(Debug, Clone, Default)]
pub struct Preview {
    pub shapes: Vec<ShapeId>,
    pub transform: Affine,
}

pub struct RenderOptions {
    pub width: u32,
    pub height: u32,
    pub view: ViewTransform,
    pub preview: Option<Preview>,
    /// Draw outlines only (the Wireframe view).
    pub wireframe: bool,
}

/// Render the objects of one page onto a transparent pixmap.
pub fn render_page(doc: &Document, page: PageId, opts: &RenderOptions) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(opts.width.max(1), opts.height.max(1))?;
    let page = doc.page(page).ok()?;
    let screen = opts.view.affine();
    let mut r = Renderer {
        pixmap: &mut pixmap,
        screen,
        zoom: opts.view.zoom,
        wireframe: opts.wireframe,
    };
    for layer in &page.layers {
        if !layer.visible {
            continue;
        }
        for shape in &layer.shapes {
            let extra = match &opts.preview {
                Some(p) if p.shapes.contains(&shape.id) => p.transform,
                _ => Affine::IDENTITY,
            };
            r.draw_shape(shape, extra);
        }
    }
    Some(pixmap)
}

struct Renderer<'a> {
    pixmap: &'a mut Pixmap,
    screen: Affine,
    zoom: f64,
    wireframe: bool,
}

impl Renderer<'_> {
    fn draw_shape(&mut self, shape: &Shape, parent: Affine) {
        if !shape.visible {
            return;
        }
        let transform = parent * shape.transform;
        if let ShapeKind::Group { children } = &shape.kind {
            for c in children {
                self.draw_shape(c, transform);
            }
            return;
        }
        if let ShapeKind::ClipFrame { frame, contents } = &shape.kind {
            // Frame's fill first, contents clipped to the frame, then the frame's outline.
            let mut frame_fill_only = (**frame).clone();
            frame_fill_only.stroke = None;
            frame_fill_only.opacity = shape.opacity;
            self.draw_shape(&frame_fill_only, transform);
            let frame_path = self.screen * (transform * frame.page_path());
            if let Some(sk_frame) = to_sk_path(&frame_path) {
                let (w, h) = (self.pixmap.width(), self.pixmap.height());
                if let (Some(mut layer), Some(mut mask)) =
                    (Pixmap::new(w, h), tiny_skia::Mask::new(w, h))
                {
                    mask.fill_path(&sk_frame, FillRule::EvenOdd, true, Transform::identity());
                    {
                        let mut sub = Renderer {
                            pixmap: &mut layer,
                            screen: self.screen,
                            zoom: self.zoom,
                            wireframe: self.wireframe,
                        };
                        for c in contents {
                            sub.draw_shape(c, transform);
                        }
                    }
                    let pp = tiny_skia::PixmapPaint {
                        opacity: shape.opacity.clamp(0.0, 1.0) as f32,
                        ..Default::default()
                    };
                    self.pixmap.draw_pixmap(
                        0,
                        0,
                        layer.as_ref(),
                        &pp,
                        Transform::identity(),
                        Some(&mask),
                    );
                }
            }
            if frame.stroke.is_some() {
                let mut outline_only = (**frame).clone();
                outline_only.fill = Fill::None;
                outline_only.opacity = shape.opacity;
                self.draw_shape(&outline_only, transform);
            }
            return;
        }
        let local = shape.local_path();
        if local.elements().is_empty() {
            return;
        }
        let page_path = transform * local;
        let Some(sk) = to_sk_path(&(self.screen * page_path.clone())) else {
            return;
        };
        let opacity = shape.opacity.clamp(0.0, 1.0) as f32;
        if opacity <= 0.0 {
            return;
        }

        // Drop shadow: blurred silhouette behind the object.
        if let Some(sh) = &shape.shadow {
            if !self.wireframe && sh.opacity > 0.0 {
                let (w, h) = (self.pixmap.width(), self.pixmap.height());
                if let Some(mut layer) = Pixmap::new(w, h) {
                    let shifted = Affine::translate(sh.offset) * page_path.clone();
                    if let Some(sk_sh) = to_sk_path(&(self.screen * shifted)) {
                        let mut paint = Paint::default();
                        paint.anti_alias = true;
                        paint.set_color(sk_color(sh.color));
                        layer.fill_path(
                            &sk_sh,
                            &paint,
                            FillRule::Winding,
                            Transform::identity(),
                            None,
                        );
                        let radius = (sh.blur * self.zoom).round() as i32;
                        if radius > 0 {
                            box_blur(&mut layer, radius.min(64));
                        }
                        let pp = tiny_skia::PixmapPaint {
                            opacity: sh.opacity.clamp(0.0, 1.0) as f32,
                            ..Default::default()
                        };
                        self.pixmap.draw_pixmap(
                            0,
                            0,
                            layer.as_ref(),
                            &pp,
                            Transform::identity(),
                            None,
                        );
                    }
                }
            }
        }

        // Semi-transparent objects render into a scratch layer first so fill
        // and outline do not double up where they overlap.
        if opacity < 1.0 {
            let (w, h) = (self.pixmap.width(), self.pixmap.height());
            if let Some(mut layer) = Pixmap::new(w, h) {
                let mut sub = Renderer {
                    pixmap: &mut layer,
                    screen: self.screen,
                    zoom: self.zoom,
                    wireframe: self.wireframe,
                };
                let mut opaque = shape.clone();
                opaque.opacity = 1.0;
                sub.draw_shape(&opaque, parent);
                let paint = tiny_skia::PixmapPaint {
                    opacity,
                    ..Default::default()
                };
                self.pixmap
                    .draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), None);
            }
            return;
        }

        if let ShapeKind::Bitmap {
            rect,
            width_px,
            height_px,
            png,
        } = &shape.kind
        {
            if !self.wireframe {
                if let Ok(img) = Pixmap::decode_png(png) {
                    // Image pixel space (y down) to local rect (y up) to screen.
                    let sx = rect.width() / (*width_px).max(1) as f64;
                    let sy = rect.height() / (*height_px).max(1) as f64;
                    let img_to_local = Affine::new([sx, 0.0, 0.0, -sy, rect.x0, rect.y1]);
                    let m = (self.screen * transform * img_to_local).as_coeffs();
                    let t = Transform::from_row(
                        m[0] as f32,
                        m[1] as f32,
                        m[2] as f32,
                        m[3] as f32,
                        m[4] as f32,
                        m[5] as f32,
                    );
                    let mut paint = Paint::default();
                    paint.anti_alias = true;
                    paint.shader = tiny_skia::Pattern::new(
                        img.as_ref(),
                        SpreadMode::Pad,
                        tiny_skia::FilterQuality::Bilinear,
                        1.0,
                        t,
                    );
                    self.pixmap.fill_path(
                        &sk,
                        &paint,
                        FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                }
            }
            if let Some(stroke) = &shape.stroke {
                let (paint, sk_stroke) = self.stroke_paint(stroke, transform);
                self.pixmap
                    .stroke_path(&sk, &paint, &sk_stroke, Transform::identity(), None);
            } else if self.wireframe {
                let mut paint = Paint::default();
                paint.set_color_rgba8(0, 0, 0, 255);
                let s = SkStroke {
                    width: 0.0,
                    ..Default::default()
                };
                self.pixmap
                    .stroke_path(&sk, &paint, &s, Transform::identity(), None);
            }
            return;
        }

        if !self.wireframe {
            if let Some(paint) = self.fill_paint(&shape.fill, &page_path) {
                let rule = match &shape.kind {
                    ShapeKind::Text { .. } => FillRule::Winding,
                    _ => FillRule::EvenOdd,
                };
                self.pixmap
                    .fill_path(&sk, &paint, rule, Transform::identity(), None);
            }
        }
        if let Some(stroke) = &shape.stroke {
            let (paint, sk_stroke) = self.stroke_paint(stroke, transform);
            self.pixmap
                .stroke_path(&sk, &paint, &sk_stroke, Transform::identity(), None);
        } else if self.wireframe {
            let mut paint = Paint::default();
            paint.set_color_rgba8(0, 0, 0, 255);
            paint.anti_alias = true;
            let s = SkStroke {
                width: 0.0,
                ..Default::default()
            };
            self.pixmap
                .stroke_path(&sk, &paint, &s, Transform::identity(), None);
        }
    }

    fn fill_paint(&self, fill: &Fill, page_path: &BezPath) -> Option<Paint<'static>> {
        let mut paint = Paint::default();
        paint.anti_alias = true;
        match fill {
            Fill::None => return None,
            Fill::Solid(c) => paint.set_color(sk_color(*c)),
            Fill::Linear { from, to, angle } => {
                let b = bbox(page_path);
                let a = angle.to_radians();
                let (cx, cy) = (b.center().x, b.center().y);
                let half = (b.width() * a.cos().abs() + b.height() * a.sin().abs()) / 2.0;
                let p0 = self.screen * Point::new(cx - half * a.cos(), cy - half * a.sin());
                let p1 = self.screen * Point::new(cx + half * a.cos(), cy + half * a.sin());
                let shader = LinearGradient::new(
                    tiny_skia::Point::from_xy(p0.x as f32, p0.y as f32),
                    tiny_skia::Point::from_xy(p1.x as f32, p1.y as f32),
                    vec![
                        GradientStop::new(0.0, sk_color(*from)),
                        GradientStop::new(1.0, sk_color(*to)),
                    ],
                    SpreadMode::Pad,
                    Transform::identity(),
                )?;
                paint.shader = shader;
            }
            Fill::Radial { from, to, offset } => {
                let b = bbox(page_path);
                let c = Point::new(
                    b.center().x + offset.x * b.width() / 2.0,
                    b.center().y + offset.y * b.height() / 2.0,
                );
                let radius = (b.width().max(b.height()) / 2.0) * std::f64::consts::SQRT_2;
                let sc = self.screen * c;
                let shader = RadialGradient::new(
                    tiny_skia::Point::from_xy(sc.x as f32, sc.y as f32),
                    0.0,
                    tiny_skia::Point::from_xy(sc.x as f32, sc.y as f32),
                    (radius * self.zoom) as f32,
                    vec![
                        GradientStop::new(0.0, sk_color(*from)),
                        GradientStop::new(1.0, sk_color(*to)),
                    ],
                    SpreadMode::Pad,
                    Transform::identity(),
                )?;
                paint.shader = shader;
            }
        }
        Some(paint)
    }

    fn stroke_paint(&self, stroke: &Stroke, transform: Affine) -> (Paint<'static>, SkStroke) {
        let mut paint = Paint::default();
        paint.anti_alias = true;
        paint.set_color(sk_color(stroke.color));
        let hair = stroke.width <= Stroke::HAIRLINE + 1e-9;
        // "Scale with object" uses the object's own scale factor.
        let object_scale = if stroke.scale_with_object {
            let c = transform.as_coeffs();
            ((c[0] * c[3] - c[1] * c[2]).abs()).sqrt().max(1e-6)
        } else {
            1.0
        };
        let width_px = if hair {
            0.0
        } else {
            (stroke.width * object_scale * self.zoom) as f32
        };
        let dash = if stroke.dash.is_empty() || hair {
            None
        } else {
            let arr: Vec<f32> = stroke
                .dash
                .iter()
                .map(|d| (*d as f32 * width_px).max(0.01))
                .collect();
            let arr = if arr.len() % 2 == 1 {
                [arr.clone(), arr].concat()
            } else {
                arr
            };
            StrokeDash::new(arr, 0.0)
        };
        let sk = SkStroke {
            width: width_px,
            miter_limit: 4.0,
            line_cap: match stroke.cap {
                LCap::Butt => LineCap::Butt,
                LCap::Round => LineCap::Round,
                LCap::Square => LineCap::Square,
            },
            line_join: match stroke.join {
                LJoin::Miter => LineJoin::Miter,
                LJoin::Round => LineJoin::Round,
                LJoin::Bevel => LineJoin::Bevel,
            },
            dash,
        };
        (paint, sk)
    }
}

/// Three-pass box blur (approximates a Gaussian) on premultiplied RGBA.
pub fn box_blur(pm: &mut Pixmap, radius: i32) {
    let w = pm.width() as usize;
    let h = pm.height() as usize;
    let r = radius.max(1) as usize;
    let data = pm.data_mut();
    let mut tmp = vec![0u8; data.len()];
    for _ in 0..3 {
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
    for o in 0..outer {
        let mut acc = [0u32; 4];
        // Prime the window with the first pixel repeated, like edge clamping.
        for i in 0..=r {
            let p = idx(o, i.min(inner - 1));
            for c in 0..4 {
                acc[c] += src[p + c] as u32;
            }
        }
        for _ in 0..r {
            let p = idx(o, 0);
            for c in 0..4 {
                acc[c] += src[p + c] as u32;
            }
        }
        for i in 0..inner {
            let d = idx(o, i);
            for c in 0..4 {
                dst[d + c] = (acc[c] / span) as u8;
            }
            let add = idx(o, (i + r + 1).min(inner - 1));
            let sub = idx(o, i.saturating_sub(r));
            for c in 0..4 {
                acc[c] = acc[c] + src[add + c] as u32 - src[sub + c] as u32;
            }
        }
    }
}

fn bbox(p: &BezPath) -> Rect {
    use tracedraw_core::geometry::Shape as _;
    p.bounding_box()
}

pub fn sk_color(c: Color) -> SkColor {
    let [r, g, b] = c.to_rgb8();
    SkColor::from_rgba8(r, g, b, 255)
}

pub fn to_sk_path(path: &BezPath) -> Option<SkPath> {
    let mut pb = PathBuilder::new();
    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => pb.move_to(p.x as f32, p.y as f32),
            PathEl::LineTo(p) => pb.line_to(p.x as f32, p.y as f32),
            PathEl::QuadTo(c, p) => pb.quad_to(c.x as f32, c.y as f32, p.x as f32, p.y as f32),
            PathEl::CurveTo(c1, c2, p) => pb.cubic_to(
                c1.x as f32,
                c1.y as f32,
                c2.x as f32,
                c2.y as f32,
                p.x as f32,
                p.y as f32,
            ),
            PathEl::ClosePath => pb.close(),
        }
    }
    pb.finish()
}

/// Render a whole page to an RGBA image at `dpi`, white background (exports, thumbnails, tests).
pub fn render_page_image(doc: &Document, page: PageId, dpi: f64) -> Option<Pixmap> {
    let p = doc.page(page).ok()?;
    let zoom = dpi / 25.4;
    let w = (p.size.width * zoom).ceil() as u32;
    let h = (p.size.height * zoom).ceil() as u32;
    let view = ViewTransform {
        zoom,
        origin_x: 0.0,
        origin_y: h as f64,
    };
    let mut objects = render_page(
        doc,
        page,
        &RenderOptions {
            width: w,
            height: h,
            view,
            preview: None,
            wireframe: false,
        },
    )?;
    let mut out = Pixmap::new(w, h)?;
    out.fill(SkColor::WHITE);
    out.draw_pixmap(
        0,
        0,
        objects.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    let _ = &mut objects;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::document::{Shape, ShapeKind};

    #[test]
    fn star_fill_covers_center_and_not_corner() {
        let mut doc = Document::new("t", tracedraw_core::geometry::Size::new(100.0, 100.0));
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Polygon {
                rect: Rect::new(10.0, 10.0, 90.0, 90.0),
                points: 5,
                sharpness: 0.6,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        s.stroke = None;
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let page = doc.pages[0].id;
        let img = render_page_image(&doc, page, 25.4).unwrap(); // 1 px per mm
        let px = |x: u32, y: u32| img.pixel(x, y).unwrap();
        // Centre of the star is red; the page corner is white.
        assert_eq!(px(50, 50).red(), 255);
        assert_eq!(px(50, 50).green(), 0);
        assert_eq!(px(2, 2).green(), 255);
    }

    #[test]
    fn half_transparent_red_over_white_is_pink() {
        let mut doc = Document::new("t", tracedraw_core::geometry::Size::new(20.0, 20.0));
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                radius: 0.0,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        s.stroke = None;
        s.opacity = 0.5;
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let page = doc.pages[0].id;
        let img = render_page_image(&doc, page, 25.4).unwrap();
        let p = img.pixel(10, 10).unwrap();
        assert_eq!(p.red(), 255);
        assert!((120..=136).contains(&p.green()), "green was {}", p.green());
    }

    #[test]
    fn shadow_darkens_pixels_beside_the_object() {
        let mut doc = Document::new("t", tracedraw_core::geometry::Size::new(40.0, 40.0));
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 25.0, 25.0),
                radius: 0.0,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(0, 0, 255));
        s.stroke = None;
        s.shadow = Some(tracedraw_core::Shadow {
            offset: tracedraw_core::geometry::Vec2::new(5.0, -5.0),
            opacity: 0.8,
            blur: 0.0,
            color: Color::BLACK,
        });
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let page = doc.pages[0].id;
        let img = render_page_image(&doc, page, 25.4).unwrap();
        // Just right of the rect and below: shadow only (y down in the image).
        let p = img.pixel(28, 32).unwrap();
        assert!(p.red() < 100, "red was {}", p.red());
        let far = img.pixel(2, 2).unwrap();
        assert_eq!(far.red(), 255);
    }

    #[test]
    fn dashed_hairline_does_not_panic() {
        let mut doc = Document::new("t", tracedraw_core::geometry::Size::new(50.0, 50.0));
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(5.0, 5.0, 45.0, 45.0),
                radius: 0.0,
            },
        );
        let mut st = Stroke::new(Color::BLACK, 1.0);
        st.dash = vec![3.0, 1.0, 1.0];
        s.stroke = Some(st);
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let page = doc.pages[0].id;
        assert!(render_page_image(&doc, page, 96.0).is_some());
    }
}
