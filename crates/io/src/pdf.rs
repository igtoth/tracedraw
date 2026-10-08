//! Minimal PDF writer: one page per document page, vector paths with
//! uniform fills (RGB or CMYK), axial/radial shadings for fountain fills,
//! outlines with width, caps, joins and dashes, bitmaps as images, and
//! uniform transparency via ExtGState. Written by hand; no dependency.

use std::fmt::Write as _;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, PathEl},
    Color, Document, Fill, LineCap, LineJoin,
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
            let _ = write!(out, "{o:010} 00000 n \n");
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
        let transform = parent * shape.transform;
        if let ShapeKind::Group { children } = &shape.kind {
            for c in children {
                self.shape(c, transform);
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
            if let Some(rgb) = decode_png_rgb(png) {
                let name = format!("Im{}", self.images.len());
                let data = flate(&rgb);
                let id = self.pdf.stream(
                    &format!("/Type /XObject /Subtype /Image /Width {width_px} /Height {height_px} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode"),
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
            Fill::Linear { from, to, angle } => {
                let a = angle.to_radians();
                let c = bounds.center();
                let half = (bounds.width() * a.cos().abs() + bounds.height() * a.sin().abs()) / 2.0;
                let (x0, y0) = (
                    (c.x - half * a.cos()) * MM_PT,
                    (c.y - half * a.sin()) * MM_PT,
                );
                let (x1, y1) = (
                    (c.x + half * a.cos()) * MM_PT,
                    (c.y + half * a.sin()) * MM_PT,
                );
                let name = format!("Sh{}", self.shadings.len());
                let id = self.pdf.add_str(format!(
                    "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [{} {} {} {}] /Function << /FunctionType 2 /Domain [0 1] /C0 {} /C1 {} /N 1 >> /Extend [true true] >>",
                    f(x0), f(y0), f(x1), f(y1), rgb_array(*from), rgb_array(*to)
                ));
                self.shadings.push((name.clone(), id));
                self.content.push_str("q\n");
                self.path_ops(&path);
                let _ = writeln!(self.content, "W* n /{name} sh\nQ");
            }
            Fill::Radial { from, to, offset } => {
                let c = bounds.center();
                let cx = (c.x + offset.x * bounds.width() / 2.0) * MM_PT;
                let cy = (c.y + offset.y * bounds.height() / 2.0) * MM_PT;
                let r =
                    bounds.width().max(bounds.height()) / 2.0 * std::f64::consts::SQRT_2 * MM_PT;
                let name = format!("Sh{}", self.shadings.len());
                let id = self.pdf.add_str(format!(
                    "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [{} {} 0 {} {} {}] /Function << /FunctionType 2 /Domain [0 1] /C0 {} /C1 {} /N 1 >> /Extend [true true] >>",
                    f(cx), f(cy), f(cx), f(cy), f(r), rgb_array(*from), rgb_array(*to)
                ));
                self.shadings.push((name.clone(), id));
                self.content.push_str("q\n");
                self.path_ops(&path);
                let _ = writeln!(self.content, "W* n /{name} sh\nQ");
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
        }
        self.content.push_str("Q\n");
    }
}

fn decode_png_rgb(png: &[u8]) -> Option<Vec<u8>> {
    let pm = tiny_skia::Pixmap::decode_png(png).ok()?;
    let mut rgb = Vec::with_capacity((pm.width() * pm.height() * 3) as usize);
    for p in pm.pixels() {
        let c = p.demultiply();
        rgb.extend_from_slice(&[c.red(), c.green(), c.blue()]);
    }
    Some(rgb)
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
        };
        for layer in page.layers.iter().filter(|l| l.visible && l.printable) {
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
        s.fill = Fill::Linear {
            from: Color::cmyk_pct(100.0, 0.0, 0.0, 0.0),
            to: Color::WHITE,
            angle: 0.0,
        };
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
