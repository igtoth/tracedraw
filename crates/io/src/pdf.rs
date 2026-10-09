//! Minimal PDF writer: one page per document page, vector paths with
//! uniform fills (RGB or CMYK), axial/radial shadings for fountain fills,
//! outlines with width, caps, joins and dashes, bitmaps as images, and
//! uniform transparency via ExtGState. Written by hand; no dependency.

use std::fmt::Write as _;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, PathEl},
    Color, Document, Fill, FountainKind, LineCap, LineJoin,
};

const MM_PT: f64 = 72.0 / 25.4;

struct Pdf {
    objects: Vec<Vec<u8>>,
}

impl Pdf {
    fn add(&mut self, body: Vec<u8>) -> usize {
        self.objects.push(body);
        self.objects.len()
    }
    fn add_str(&mut self, s: String) -> usize {
        self.add(s.into_bytes())
    }
    fn add_image(&mut self, w: u32, h: u32, rgb: &[u8]) -> usize {
        let data = flate(rgb);
        self.stream(
            &format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode"),
            &data,
        )
    }

    fn stream(&mut self, dict: &str, data: &[u8]) -> usize {
        let mut v = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
        v.extend_from_slice(data);
        v.extend_from_slice(b"\nendstream");
        self.add(v)
    }
    fn finish(self) -> Vec<u8> {
        use std::io::Write;
        let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets = Vec::with_capacity(self.objects.len());
        for (i, body) in self.objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            self.objects.len() + 1
        );
        for o in offsets {
            let _ = writeln!(out, "{o:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            self.objects.len() + 1
        );
        out
    }
}

fn f(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn color_op(c: Color, fill: bool) -> String {
    match c {
        Color::Cmyk { c, m, y, k } => format!(
            "{} {} {} {} {}",
            f(c as f64),
            f(m as f64),
            f(y as f64),
            f(k as f64),
            if fill { "k" } else { "K" }
        ),
        other => {
            let [r, g, b] = other.to_rgb8();
            format!(
                "{} {} {} {}",
                f(r as f64 / 255.0),
                f(g as f64 / 255.0),
                f(b as f64 / 255.0),
                if fill { "rg" } else { "RG" }
            )
        }
    }
}

fn rgb_array(c: Color) -> String {
    let [r, g, b] = c.to_rgb8();
    format!(
        "[{} {} {}]",
        f(r as f64 / 255.0),
        f(g as f64 / 255.0),
        f(b as f64 / 255.0)
    )
}

struct PageWriter<'a> {
    content: String,
    pdf: &'a mut Pdf,
    shadings: Vec<(String, usize)>,
    gstates: Vec<(String, usize)>,
    images: Vec<(String, usize)>,
    symbols: &'a [tracedraw_core::Symbol],
}

impl PageWriter<'_> {
    fn path_ops(&mut self, path: &tracedraw_core::BezPath) {
        for el in path.elements() {
            match el {
                PathEl::MoveTo(p) => {
                    let _ = writeln!(self.content, "{} {} m", f(p.x * MM_PT), f(p.y * MM_PT));
                }
                PathEl::LineTo(p) => {
                    let _ = writeln!(self.content, "{} {} l", f(p.x * MM_PT), f(p.y * MM_PT));
                }
                PathEl::QuadTo(c, p) => {
                    // PDF has no quadratics; elevate to a cubic.
                    let _ = writeln!(
                        self.content,
                        "{} {} {} {} {} {} c",
                        f(c.x * MM_PT),
                        f(c.y * MM_PT),
                        f(c.x * MM_PT),
                        f(c.y * MM_PT),
                        f(p.x * MM_PT),
                        f(p.y * MM_PT)
                    );
                }
                PathEl::CurveTo(c1, c2, p) => {
                    let _ = writeln!(
                        self.content,
                        "{} {} {} {} {} {} c",
                        f(c1.x * MM_PT),
                        f(c1.y * MM_PT),
                        f(c2.x * MM_PT),
                        f(c2.y * MM_PT),
                        f(p.x * MM_PT),
                        f(p.y * MM_PT)
                    );
                }
                PathEl::ClosePath => self.content.push_str("h\n"),
            }
        }
    }

    fn shape(&mut self, shape: &Shape, parent: Affine) {
        if !shape.visible {
            return;
        }
        if !shape.effects.is_empty() {
            let ev = tracedraw_core::live::evaluate(shape);
            for b in &ev.below {
                let mut b = b.clone();
                b.effects.clear();
                self.shape(&b, parent);
            }
            let mut main = ev.main.clone();
            main.effects.clear();
            for e in &shape.effects {
                match e {
                    tracedraw_core::live::Effect::Transparency { mask, .. } => {
                        main.opacity *= crate::svg::average_luminance(mask) as f64;
                    }
                    tracedraw_core::live::Effect::Lens(
                        tracedraw_core::live::Lens::Transparency { rate, color },
                    ) => {
                        main.fill = Fill::Solid(*color);
                        main.opacity *= rate / 100.0;
                    }
                    tracedraw_core::live::Effect::Lens(_) => main.opacity *= 0.5,
                    _ => {}
                }
            }
            self.shape(&main, parent);
            for a in &ev.above {
                let mut a = a.clone();
                a.effects.clear();
                self.shape(&a, parent);
            }
            return;
        }
        let transform = parent * shape.transform;
        if let ShapeKind::Group { children } = &shape.kind {
            for c in children {
                self.shape(c, transform);
            }
            return;
        }
        if matches!(
            shape.kind,
            ShapeKind::Table(_) | ShapeKind::SymbolInstance { .. }
        ) {
            for c in shape.expand(self.symbols) {
                self.shape(&c, transform);
            }
            return;
        }
        if let ShapeKind::ClipFrame { frame, contents } = &shape.kind {
            let mut fill_only = (**frame).clone();
            fill_only.stroke = None;
            self.shape(&fill_only, transform);
            self.content.push_str("q\n");
            if shape.opacity < 1.0 {
                let name = format!("GS{}", self.gstates.len());
                let id = self.pdf.add_str(format!(
                    "<< /Type /ExtGState /CA {} /ca {} >>",
                    f(shape.opacity),
                    f(shape.opacity)
                ));
                self.gstates.push((name.clone(), id));
                let _ = writeln!(self.content, "/{name} gs");
            }
            self.path_ops(&(transform * frame.page_path()));
            self.content.push_str("W n\n");
            for c in contents {
                self.shape(c, transform);
            }
            self.content.push_str("Q\n");
            let mut outline = (**frame).clone();
            outline.fill = Fill::None;
            if outline.stroke.is_some() {
                self.shape(&outline, transform);
            }
            return;
        }
        let path = transform * shape.local_path();
        if path.elements().is_empty() {
            return;
        }
        use tracedraw_core::geometry::Shape as _;
        let bounds = path.bounding_box();
        self.content.push_str("q\n");
        if shape.opacity < 1.0 {
            let name = format!("GS{}", self.gstates.len());
            let id = self.pdf.add_str(format!(
                "<< /Type /ExtGState /CA {} /ca {} >>",
                f(shape.opacity),
                f(shape.opacity)
            ));
            self.gstates.push((name.clone(), id));
            let _ = writeln!(self.content, "/{name} gs");
        }

        if let ShapeKind::Bitmap {
            rect,
            width_px,
            height_px,
            png,
        } = &shape.kind
        {
            if let Some((rgb, alpha)) = decode_png_rgb_alpha(png) {
                let name = format!("Im{}", self.images.len());
                let data = flate(&rgb);
                // Transparent pixels go to a soft mask.
                let smask = alpha.map(|a| {
                    let adata = flate(&a);
                    self.pdf.stream(
                        &format!("/Type /XObject /Subtype /Image /Width {width_px} /Height {height_px} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode"),
                        &adata,
                    )
                });
                let smask_ref = smask
                    .map(|id| format!(" /SMask {id} 0 R"))
                    .unwrap_or_default();
                let id = self.pdf.stream(
                    &format!("/Type /XObject /Subtype /Image /Width {width_px} /Height {height_px} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode{smask_ref}"),
                    &data,
                );
                self.images.push((name.clone(), id));
                // Image space (unit square, y up) to local rect to page.
                let m = (transform
                    * Affine::new([rect.width(), 0.0, 0.0, rect.height(), rect.x0, rect.y0]))
                .as_coeffs();
                let _ = writeln!(
                    self.content,
                    "{} {} {} {} {} {} cm /{name} Do",
                    f(m[0] * MM_PT),
                    f(m[1] * MM_PT),
                    f(m[2] * MM_PT),
                    f(m[3] * MM_PT),
                    f(m[4] * MM_PT),
                    f(m[5] * MM_PT)
                );
            }
            self.content.push_str("Q\n");
            return;
        }

        // Fill.
        match &shape.fill {
            Fill::None => {}
            Fill::Solid(c) => {
                let _ = writeln!(self.content, "{}", color_op(*c, true));
                self.path_ops(&path);
                self.content
                    .push_str(if matches!(shape.kind, ShapeKind::Text { .. }) {
                        "f\n"
                    } else {
                        "f*\n"
                    });
            }
            Fill::Fountain(ft) => {
                let func = stitching_function(ft);
                let shading = match ft.kind {
                    FountainKind::Radial => {
                        let c = bounds.center();
                        let cx = (c.x + ft.offset.x * bounds.width() / 2.0) * MM_PT;
                        let cy = (c.y + ft.offset.y * bounds.height() / 2.0) * MM_PT;
                        let r = bounds.width().max(bounds.height()) / 2.0
                            * std::f64::consts::SQRT_2
                            * MM_PT;
                        format!(
                            "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [{} {} 0 {} {} {}] /Function {} /Extend [true true] >>",
                            f(cx), f(cy), f(cx), f(cy), f(r), func
                        )
                    }
                    // Conical and square fountains have no PDF shading type;
                    // they are approximated by a linear axis at the same angle.
                    _ => {
                        let a = ft.angle.to_radians();
                        let c = bounds.center();
                        let half = (bounds.width() * a.cos().abs()
                            + bounds.height() * a.sin().abs())
                            / 2.0;
                        let (x0, y0) = (
                            (c.x - half * a.cos()) * MM_PT,
                            (c.y - half * a.sin()) * MM_PT,
                        );
                        let (x1, y1) = (
                            (c.x + half * a.cos()) * MM_PT,
                            (c.y + half * a.sin()) * MM_PT,
                        );
                        format!(
                            "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [{} {} {} {}] /Function {} /Extend [true true] >>",
                            f(x0), f(y0), f(x1), f(y1), func
                        )
                    }
                };
                let name = format!("Sh{}", self.shadings.len());
                let id = self.pdf.add_str(shading);
                self.shadings.push((name.clone(), id));
                self.content.push_str("q\n");
                self.path_ops(&path);
                let _ = writeln!(self.content, "W* n /{name} sh\nQ");
            }
            // Patterns and textures are rasterised into an image clipped by
            // the path; the renderer produces the same pixels as the screen.
            Fill::Pattern(_) | Fill::Texture(_) | Fill::Mesh(_) => {
                if let Some((png, w, h)) = rasterise_fill(shape, bounds) {
                    if let Some(rgb) = decode_png_rgb(&png) {
                        let name = format!("Im{}", self.images.len());
                        let id = self.pdf.add_image(w, h, &rgb);
                        self.images.push((name.clone(), id));
                        self.content.push_str("q\n");
                        self.path_ops(&path);
                        let _ = writeln!(
                            self.content,
                            "W* n {} 0 0 {} {} {} cm /{name} Do\nQ",
                            f(bounds.width() * MM_PT),
                            f(bounds.height() * MM_PT),
                            f(bounds.x0 * MM_PT),
                            f(bounds.y0 * MM_PT)
                        );
                    }
                } else {
                    let _ = writeln!(
                        self.content,
                        "{}",
                        color_op(shape.fill.preview_color().unwrap_or(Color::BLACK), true)
                    );
                    self.path_ops(&path);
                    self.content.push_str("f*\n");
                }
            }
        }

        // Outline.
        if let Some(s) = &shape.stroke {
            let width = if s.width <= tracedraw_core::Stroke::HAIRLINE + 1e-9 {
                0.0
            } else {
                s.width * MM_PT
            };
            let _ = writeln!(
                self.content,
                "{} {} w {} J {} j",
                color_op(s.color, false),
                f(width),
                match s.cap {
                    LineCap::Butt => 0,
                    LineCap::Round => 1,
                    LineCap::Square => 2,
                },
                match s.join {
                    LineJoin::Miter => 0,
                    LineJoin::Round => 1,
                    LineJoin::Bevel => 2,
                }
            );
            if !s.dash.is_empty() {
                let w = if width > 0.0 { width } else { 1.0 };
                let arr: Vec<String> = s.dash.iter().map(|d| f(d * w)).collect();
                let _ = writeln!(self.content, "[{}] 0 d", arr.join(" "));
            }
            self.path_ops(&path);
            self.content.push_str("S\n");
            // Arrowheads (presets and custom) are filled with the outline
            // colour. They are built on the page-space path, which already
            // carries the shape's own transform, so only the parent applies.
            for head in tracedraw_core::arrowhead_paths(&shape.page_path(), s) {
                let _ = writeln!(self.content, "{}", color_op(s.color, true));
                self.path_ops(&(parent * head));
                self.content.push_str("f\n");
            }
        }
        self.content.push_str("Q\n");
    }
}

/// PDF stitching function over the fountain's stops (type 3 wrapping type 2
/// segments), or a single type 2 function for two stops.
fn stitching_function(ft: &tracedraw_core::Fountain) -> String {
    let mut stops: Vec<(f64, Color)> = ft
        .stops
        .iter()
        .map(|s| (s.pos.clamp(0.0, 1.0), s.color))
        .collect();
    stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    if stops.is_empty() {
        stops.push((0.0, Color::BLACK));
    }
    if stops.len() == 1 {
        stops.push((1.0, stops[0].1));
    }
    if stops.len() == 2 {
        return format!(
            "<< /FunctionType 2 /Domain [0 1] /C0 {} /C1 {} /N 1 >>",
            rgb_array(stops[0].1),
            rgb_array(stops[1].1)
        );
    }
    // Normalise to [0,1]: pad the first and last stop to the ends.
    let first = stops[0].0;
    let last = stops[stops.len() - 1].0;
    let span = (last - first).max(1e-6);
    let mut funcs = String::new();
    let mut bounds = String::new();
    let mut encode = String::new();
    for w in stops.windows(2) {
        let _ = write!(
            funcs,
            "<< /FunctionType 2 /Domain [0 1] /C0 {} /C1 {} /N 1 >> ",
            rgb_array(w[0].1),
            rgb_array(w[1].1)
        );
        encode.push_str("0 1 ");
    }
    for s in &stops[1..stops.len() - 1] {
        let _ = write!(bounds, "{} ", f((s.0 - first) / span));
    }
    format!(
        "<< /FunctionType 3 /Domain [0 1] /Functions [{}] /Bounds [{}] /Encode [{}] >>",
        funcs.trim_end(),
        bounds.trim_end(),
        encode.trim_end()
    )
}

/// Rasterise a pattern or texture fill over the shape bounds at 150 dpi.
fn rasterise_fill(
    shape: &Shape,
    bounds: tracedraw_core::geometry::Rect,
) -> Option<(Vec<u8>, u32, u32)> {
    let pm = tracedraw_render::render_fill_image(&shape.fill, bounds, 150.0)?;
    let (w, h) = (pm.width(), pm.height());
    Some((pm.encode_png().ok()?, w, h))
}

fn decode_png_rgb(png: &[u8]) -> Option<Vec<u8>> {
    decode_png_rgb_alpha(png).map(|(rgb, _)| rgb)
}

/// RGB samples and, when any pixel is not opaque, the alpha channel.
fn decode_png_rgb_alpha(png: &[u8]) -> Option<(Vec<u8>, Option<Vec<u8>>)> {
    let pm = tiny_skia::Pixmap::decode_png(png).ok()?;
    let mut rgb = Vec::with_capacity((pm.width() * pm.height() * 3) as usize);
    let mut alpha = Vec::with_capacity((pm.width() * pm.height()) as usize);
    let mut any = false;
    for p in pm.pixels() {
        let c = p.demultiply();
        rgb.extend_from_slice(&[c.red(), c.green(), c.blue()]);
        alpha.push(c.alpha());
        any |= c.alpha() != 255;
    }
    Some((rgb, any.then_some(alpha)))
}

fn flate(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    let _ = e.write_all(data);
    e.finish().unwrap_or_default()
}

pub fn document_to_pdf(doc: &Document) -> Vec<u8> {
    let mut pdf = Pdf {
        objects: Vec::new(),
    };
    pdf.add_str(String::new()); // 1: catalog, filled at the end
    pdf.add_str(String::new()); // 2: pages
    let mut page_ids = Vec::new();
    for page in &doc.pages {
        let mut w = PageWriter {
            content: String::new(),
            pdf: &mut pdf,
            shadings: Vec::new(),
            gstates: Vec::new(),
            images: Vec::new(),
            symbols: &doc.symbols,
        };
        if let Some(bg) = &page.background {
            let mut bg_shape = Shape::new(
                tracedraw_core::ShapeId(0),
                ShapeKind::Rect {
                    rect: page.rect(),
                    radius: 0.0,
                },
            );
            bg_shape.fill = bg.clone();
            bg_shape.stroke = None;
            w.shape(&bg_shape, Affine::IDENTITY);
        }
        let layers = doc.layers_for_page(page.id).unwrap_or_default();
        for layer in layers.iter().filter(|l| l.visible && l.printable) {
            for s in &layer.shapes {
                w.shape(s, Affine::IDENTITY);
            }
        }
        let PageWriter {
            content,
            shadings,
            gstates,
            images,
            ..
        } = w;
        let data = flate(content.as_bytes());
        let content_id = pdf.stream("/Filter /FlateDecode", &data);
        let mut res = String::from("<< ");
        if !shadings.is_empty() {
            res.push_str("/Shading << ");
            for (n, id) in &shadings {
                let _ = write!(res, "/{n} {id} 0 R ");
            }
            res.push_str(">> ");
        }
        if !gstates.is_empty() {
            res.push_str("/ExtGState << ");
            for (n, id) in &gstates {
                let _ = write!(res, "/{n} {id} 0 R ");
            }
            res.push_str(">> ");
        }
        if !images.is_empty() {
            res.push_str("/XObject << ");
            for (n, id) in &images {
                let _ = write!(res, "/{n} {id} 0 R ");
            }
            res.push_str(">> ");
        }
        res.push_str(">>");
        let page_id = pdf.add_str(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents {} 0 R /Resources {} >>",
            f(page.size.width * MM_PT),
            f(page.size.height * MM_PT),
            content_id,
            res
        ));
        page_ids.push(page_id);
    }
    let kids: Vec<String> = page_ids.iter().map(|id| format!("{id} 0 R")).collect();
    pdf.objects[1] = format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids.join(" "),
        page_ids.len()
    )
    .into_bytes();
    pdf.objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    pdf.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;

    #[test]
    fn multi_stop_fountain_uses_stitching_function() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
            },
        );
        s.fill = Fill::Fountain(tracedraw_core::Fountain {
            kind: FountainKind::Linear,
            stops: vec![
                tracedraw_core::Stop {
                    pos: 0.0,
                    color: Color::rgb8(255, 0, 0),
                },
                tracedraw_core::Stop {
                    pos: 0.3,
                    color: Color::rgb8(0, 255, 0),
                },
                tracedraw_core::Stop {
                    pos: 1.0,
                    color: Color::rgb8(0, 0, 255),
                },
            ],
            angle: 90.0,
            offset: tracedraw_core::geometry::Point::ZERO,
            edge_pad: 0.0,
        });
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let text = String::from_utf8_lossy(&document_to_pdf(&doc)).to_string();
        assert!(text.contains("/FunctionType 3"));
        assert!(text.contains("/Bounds [0.3]"));
    }

    #[test]
    fn pattern_fill_becomes_image() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Ellipse {
                rect: Rect::new(10.0, 10.0, 60.0, 40.0),
                arc: None,
            },
        );
        s.fill = Fill::Pattern(tracedraw_core::Pattern::TwoColor {
            tile: tracedraw_core::PatternTile::Dots,
            front: Color::BLACK,
            back: Color::WHITE,
            size_mm: 5.0,
        });
        s.stroke = Some(tracedraw_core::Stroke {
            end_arrow: tracedraw_core::Arrowhead::Arrow,
            ..tracedraw_core::Stroke::new(Color::BLACK, 1.0)
        });
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let text = String::from_utf8_lossy(&document_to_pdf(&doc)).to_string();
        assert!(text.contains("/Subtype /Image"));
        assert!(text.contains("/Im0 "));
    }

    #[test]
    fn vector_pattern_fill_becomes_image() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let mut tile_square = Shape::new(
            tracedraw_core::ShapeId(7),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 5.0, 5.0),
                radius: 0.0,
            },
        );
        tile_square.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        tile_square.stroke = None;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 30.0, 30.0),
                radius: 0.0,
            },
        );
        s.fill = Fill::Pattern(tracedraw_core::Pattern::Vector {
            shapes: vec![tile_square],
            tile: tracedraw_core::geometry::Size::new(10.0, 10.0),
        });
        s.stroke = None;
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let text = String::from_utf8_lossy(&document_to_pdf(&doc)).to_string();
        // The content stream is compressed; the image resource is not.
        assert!(text.contains("/Subtype /Image"));
        assert!(text.contains("/XObject << /Im0 "));
    }

    #[test]
    fn custom_arrowhead_lands_at_the_line_end_in_the_content_stream() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let mut tri = tracedraw_core::BezPath::new();
        tri.move_to((0.0, 0.0));
        tri.line_to((20.0, 10.0));
        tri.line_to((0.0, 20.0));
        tri.close_path();
        let mut line = Shape::new(
            tracedraw_core::ShapeId(1),
            ShapeKind::Path {
                path: {
                    let mut p = tracedraw_core::BezPath::new();
                    p.move_to((10.0, 50.0));
                    p.line_to((60.0, 50.0));
                    p
                },
                closed: false,
            },
        );
        line.fill = Fill::None;
        line.stroke = Some(tracedraw_core::Stroke {
            end_arrow: tracedraw_core::Arrowhead::from_shape_path(&tri, "Tri"),
            ..tracedraw_core::Stroke::new(Color::BLACK, 2.0)
        });
        line.transform = Affine::translate((0.0, 100.0));
        doc.layer_mut(layer).unwrap().shapes.push(line);
        let bytes = document_to_pdf(&doc);
        // Inflate every zlib stream and look for the head's base corner and tip.
        let mut content = String::new();
        let mut rest: &[u8] = &bytes;
        while let Some(i) = rest.windows(6).position(|w| w == b"stream") {
            let after = &rest[i + 6..];
            let start = after
                .iter()
                .position(|b| *b == b'\n')
                .map(|n| n + 1)
                .unwrap_or(0);
            let data = &after[start..];
            use std::io::Read;
            let mut out = Vec::new();
            if flate2::read::ZlibDecoder::new(data)
                .read_to_end(&mut out)
                .is_ok()
            {
                content.push_str(&String::from_utf8_lossy(&out));
            }
            rest = &after[start..];
        }
        let base = format!("{} {} m", f(52.0 * MM_PT), f(146.0 * MM_PT));
        assert!(content.contains(&base), "head base corner: {content}");
        let tip = format!("{} {} l", f(60.0 * MM_PT), f(150.0 * MM_PT));
        assert!(content.contains(&tip), "head tip: {content}");
        assert!(!content.contains(&format!("{} {} l", f(60.0 * MM_PT), f(250.0 * MM_PT))));
    }

    #[test]
    fn writes_a_parseable_pdf() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
            },
        );
        s.fill = Fill::linear(Color::cmyk_pct(100.0, 0.0, 0.0, 0.0), Color::WHITE, 0.0);
        s.opacity = 0.5;
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let bytes = document_to_pdf(&doc);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("/ShadingType 2"));
        assert!(text.contains("/ExtGState"));
        assert!(text.contains("MediaBox [0 0 595.2756 841.8898]"));
        assert!(text.trim_end().ends_with("%%EOF"));
    }
}
