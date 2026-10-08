//! SVG export. One page per SVG, in millimetres, with the Y axis flipped
//! from our bottom-up page space to SVG's top-down space.

use std::fmt::Write;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, PathEl, Shape as _},
    Document, Fill, LineCap, LineJoin,
};

pub fn page_to_svg(doc: &Document, page_index: usize) -> String {
    let Some(page) = doc.pages.get(page_index) else {
        return String::from("<svg xmlns=\"http://www.w3.org/2000/svg\"/>");
    };
    let w = page.size.width;
    let h = page.size.height;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\">"
    );
    let _ = writeln!(out, "  <title>{}</title>", escape(&doc.title));
    // Flip Y: page space has origin bottom-left.
    let flip = Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, h]);
    let mut gradient_defs = String::new();
    let mut body = String::new();
    let mut next_grad = 0usize;
    for layer in &page.layers {
        if !layer.visible {
            continue;
        }
        let _ = writeln!(
            body,
            "  <g id=\"{}\" data-name=\"{}\">",
            layer.id.raw(),
            escape(&layer.name)
        );
        for shape in &layer.shapes {
            write_shape(
                shape,
                flip,
                &mut body,
                &mut gradient_defs,
                &mut next_grad,
                2,
            );
        }
        let _ = writeln!(body, "  </g>");
    }
    if !gradient_defs.is_empty() {
        let _ = writeln!(out, "  <defs>\n{gradient_defs}  </defs>");
    }
    out.push_str(&body);
    out.push_str("</svg>\n");
    out
}

fn write_shape(
    shape: &Shape,
    parent: Affine,
    out: &mut String,
    defs: &mut String,
    next_grad: &mut usize,
    indent: usize,
) {
    if !shape.visible {
        return;
    }
    let pad = "  ".repeat(indent);
    let transform = parent * shape.transform;
    if let ShapeKind::Group { children } = &shape.kind {
        let _ = writeln!(out, "{pad}<g id=\"{}\">", shape.id.raw());
        for c in children {
            write_shape(c, transform, out, defs, next_grad, indent + 1);
        }
        let _ = writeln!(out, "{pad}</g>");
        return;
    }
    let path = transform * shape.local_path();
    let d = path_data(&path);
    let bounds = path.bounding_box();

    let fill_attr = match &shape.fill {
        Fill::None => "fill=\"none\"".to_string(),
        Fill::Solid(c) => format!("fill=\"{}\"", c.to_hex()),
        Fill::Fountain(f) => {
            *next_grad += 1;
            let id = format!("grad{}", *next_grad);
            let mut stops = f.stops.clone();
            stops.sort_by(|a, b| {
                a.pos
                    .partial_cmp(&b.pos)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let stop_xml: String = stops
                .iter()
                .map(|st| {
                    format!(
                        "      <stop offset=\"{}\" stop-color=\"{}\"/>\n",
                        fmt(st.pos),
                        st.color.to_hex()
                    )
                })
                .collect();
            match f.kind {
                tracedraw_core::FountainKind::Linear
                | tracedraw_core::FountainKind::Conical
                | tracedraw_core::FountainKind::Square => {
                    // SVG has no conical/square gradients; they become linear.
                    let a = f.angle.to_radians();
                    let (x1, y1, x2, y2) = (
                        bounds.x0 + bounds.width() * (0.5 - a.cos() / 2.0),
                        bounds.y0 + bounds.height() * (0.5 + a.sin() / 2.0),
                        bounds.x0 + bounds.width() * (0.5 + a.cos() / 2.0),
                        bounds.y0 + bounds.height() * (0.5 - a.sin() / 2.0),
                    );
                    let _ = writeln!(defs, "    <linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\">\n{stop_xml}    </linearGradient>");
                }
                tracedraw_core::FountainKind::Radial => {
                    let cx = bounds.center().x + f.offset.x * bounds.width() / 2.0;
                    let cy = bounds.center().y - f.offset.y * bounds.height() / 2.0;
                    let r = bounds.width().max(bounds.height()) / 2.0 * std::f64::consts::SQRT_2;
                    let _ = writeln!(defs, "    <radialGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\">\n{stop_xml}    </radialGradient>");
                }
            }
            format!("fill=\"url(#{id})\"")
        }
        Fill::Pattern(tracedraw_core::Pattern::TwoColor {
            tile,
            front,
            back,
            size_mm,
        }) => {
            *next_grad += 1;
            let id = format!("pat{}", *next_grad);
            // Rasterize the tile at 32 px into a data URI.
            let n = 32u32;
            let mut rgba = Vec::with_capacity((n * n * 4) as usize);
            for y in 0..n {
                for x in 0..n {
                    let u = (x as f64 + 0.5) / n as f64;
                    let v = 1.0 - (y as f64 + 0.5) / n as f64;
                    let c = if tile.front(u, v) { *front } else { *back };
                    let [r, g, b] = c.to_rgb8();
                    rgba.extend_from_slice(&[r, g, b, 255]);
                }
            }
            let png = encode_png(n, n, &rgba);
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(png);
            let _ = writeln!(defs, "    <pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" width=\"{s}\" height=\"{s}\"><image width=\"{s}\" height=\"{s}\" href=\"data:image/png;base64,{b64}\"/></pattern>", s = fmt(*size_mm));
            format!("fill=\"url(#{id})\"")
        }
        Fill::Pattern(tracedraw_core::Pattern::Bitmap {
            png,
            width_px,
            height_px,
            size_mm,
        }) => {
            *next_grad += 1;
            let id = format!("pat{}", *next_grad);
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(png);
            let h = size_mm * *height_px as f64 / (*width_px).max(1) as f64;
            let _ = writeln!(defs, "    <pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" width=\"{}\" height=\"{}\"><image width=\"{}\" height=\"{}\" href=\"data:image/png;base64,{b64}\"/></pattern>", fmt(*size_mm), fmt(h), fmt(*size_mm), fmt(h));
            format!("fill=\"url(#{id})\"")
        }
        Fill::Texture(t) => {
            // Approximate with the average of the two colours.
            format!(
                "fill=\"{}\"",
                tracedraw_core::style::lerp_color(t.color_a, t.color_b, 0.5).to_hex()
            )
        }
    };

    let stroke_attr = match &shape.stroke {
        None => "stroke=\"none\"".to_string(),
        Some(s) => {
            let cap = match s.cap {
                LineCap::Butt => "butt",
                LineCap::Round => "round",
                LineCap::Square => "square",
            };
            let join = match s.join {
                LineJoin::Miter => "miter",
                LineJoin::Round => "round",
                LineJoin::Bevel => "bevel",
            };
            let mut a = format!(
                "stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"{cap}\" stroke-linejoin=\"{join}\"",
                s.color.to_hex(),
                s.width
            );
            if !s.dash.is_empty() {
                let dashes: Vec<String> = s.dash.iter().map(|d| fmt(d * s.width)).collect();
                let _ = write!(a, " stroke-dasharray=\"{}\"", dashes.join(" "));
            }
            a
        }
    };

    let name = shape
        .name
        .as_deref()
        .map(|n| format!(" data-name=\"{}\"", escape(n)))
        .unwrap_or_default();
    let _ = writeln!(
        out,
        "{pad}<path id=\"{}\"{name} d=\"{d}\" {fill_attr} {stroke_attr}/>",
        shape.id.raw()
    );
}

fn fmt(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".into()
    } else {
        s.to_string()
    }
}

pub fn path_data(path: &tracedraw_core::BezPath) -> String {
    let mut d = String::new();
    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => {
                let _ = write!(d, "M{} {}", fmt(p.x), fmt(p.y));
            }
            PathEl::LineTo(p) => {
                let _ = write!(d, "L{} {}", fmt(p.x), fmt(p.y));
            }
            PathEl::QuadTo(c, p) => {
                let _ = write!(d, "Q{} {} {} {}", fmt(c.x), fmt(c.y), fmt(p.x), fmt(p.y));
            }
            PathEl::CurveTo(c1, c2, p) => {
                let _ = write!(
                    d,
                    "C{} {} {} {} {} {}",
                    fmt(c1.x),
                    fmt(c1.y),
                    fmt(c2.x),
                    fmt(c2.y),
                    fmt(p.x),
                    fmt(p.y)
                );
            }
            PathEl::ClosePath => d.push('Z'),
        }
    }
    d
}

/// Minimal PNG encoder (RGBA8) via tiny-skia.
pub fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut pm = match tiny_skia::Pixmap::new(w, h) {
        Some(p) => p,
        None => return Vec::new(),
    };
    for (i, px) in pm.pixels_mut().iter_mut().enumerate() {
        let o = i * 4;
        if o + 3 < rgba.len() {
            *px = tiny_skia::ColorU8::from_rgba(rgba[o], rgba[o + 1], rgba[o + 2], rgba[o + 3])
                .premultiply();
        }
    }
    pm.encode_png().unwrap_or_default()
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{geometry::Rect, Color, Document, Stroke};

    #[test]
    fn exports_a_filled_rect() {
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
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        s.stroke = Some(Stroke::new(Color::BLACK, 0.5));
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let svg = page_to_svg(&doc, 0);
        assert!(svg.contains("viewBox=\"0 0 210 297\""));
        assert!(svg.contains("fill=\"#ff0000\""));
        // Bottom-left origin: y=10 in page space is y=287 in SVG space.
        assert!(svg.contains("M10 287"));
        assert!(svg.contains("stroke-width=\"0.5\""));
    }
}
