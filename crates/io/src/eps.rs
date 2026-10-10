//! Encapsulated PostScript writer (Level 3): paths with solid and fountain
//! fills (`shfill`), outlines with width, caps, joins and dashes, bitmaps as
//! `colorimage`, text as outlines. One page per file.

use std::fmt::Write as _;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, PathEl, Shape as _},
    Color, Document, Fill, FountainKind, LineCap, LineJoin,
};

const MM_PT: f64 = 72.0 / 25.4;

fn f(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn color_op(c: Color) -> String {
    match c {
        Color::Cmyk { c, m, y, k } => format!(
            "{} {} {} {} setcmykcolor",
            f(c as f64),
            f(m as f64),
            f(y as f64),
            f(k as f64)
        ),
        Color::Gray { v } => format!("{} setgray", f(v as f64)),
        other => {
            let [r, g, b] = other.to_rgb8();
            format!(
                "{} {} {} setrgbcolor",
                f(r as f64 / 255.0),
                f(g as f64 / 255.0),
                f(b as f64 / 255.0)
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

pub fn page_to_eps(doc: &Document, page_index: usize) -> String {
    let Some(page) = doc.pages.get(page_index) else {
        return String::new();
    };
    let w = page.size.width * MM_PT;
    let h = page.size.height * MM_PT;
    let mut out = String::new();
    let _ = writeln!(out, "%!PS-Adobe-3.0 EPSF-3.0");
    let _ = writeln!(out, "%%Creator: TraceDraw");
    let _ = writeln!(out, "%%Title: {}", doc.title.replace(['\n', '\r'], " "));
    let _ = writeln!(
        out,
        "%%BoundingBox: 0 0 {} {}",
        w.ceil() as i64,
        h.ceil() as i64
    );
    let _ = writeln!(out, "%%HiResBoundingBox: 0 0 {} {}", f(w), f(h));
    let _ = writeln!(out, "%%LanguageLevel: 3");
    let _ = writeln!(out, "%%EndComments");
    let _ = writeln!(
        out,
        "/m {{moveto}} def /l {{lineto}} def /c {{curveto}} def /h {{closepath}} def"
    );
    let _ = writeln!(out, "gsave");
    let mut w_ = Writer {
        out: &mut out,
        symbols: &doc.symbols,
    };
    if let Some(bg) = &page.background {
        let mut s = Shape::new(
            tracedraw_core::ShapeId(0),
            ShapeKind::Rect {
                rect: page.rect(),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = bg.clone();
        s.stroke = None;
        w_.shape(&s, Affine::IDENTITY);
    }
    let layers = doc.layers_for_page(page.id).unwrap_or_default();
    for layer in layers.iter().filter(|l| l.visible && l.printable) {
        for s in &layer.shapes {
            w_.shape(s, Affine::IDENTITY);
        }
    }
    let _ = writeln!(out, "grestore");
    let _ = writeln!(out, "%%EOF");
    out
}

struct Writer<'a> {
    out: &'a mut String,
    symbols: &'a [tracedraw_core::Symbol],
}

impl Writer<'_> {
    fn path_ops(&mut self, path: &tracedraw_core::BezPath) {
        for el in path.elements() {
            match el {
                PathEl::MoveTo(p) => {
                    let _ = writeln!(self.out, "{} {} m", f(p.x * MM_PT), f(p.y * MM_PT));
                }
                PathEl::LineTo(p) => {
                    let _ = writeln!(self.out, "{} {} l", f(p.x * MM_PT), f(p.y * MM_PT));
                }
                PathEl::QuadTo(c, p) => {
                    let _ = writeln!(
                        self.out,
                        "{} {} {} {} {} {} c",
                        f(c.x * MM_PT),
                        f(c.y * MM_PT),
                        f(c.x * MM_PT),
                        f(c.y * MM_PT),
                        f(p.x * MM_PT),
                        f(p.y * MM_PT)
                    );
                }
                PathEl::CurveTo(a, b, p) => {
                    let _ = writeln!(
                        self.out,
                        "{} {} {} {} {} {} c",
                        f(a.x * MM_PT),
                        f(a.y * MM_PT),
                        f(b.x * MM_PT),
                        f(b.y * MM_PT),
                        f(p.x * MM_PT),
                        f(p.y * MM_PT)
                    );
                }
                PathEl::ClosePath => {
                    let _ = writeln!(self.out, "h");
                }
            }
        }
    }

    /// Emit an RGB `colorimage` of `pm` mapped from the unit square (row 0
    /// at the top) through `m` (mm) to the page. Alpha is composited over
    /// white, since EPS has none. The caller wraps this in gsave/grestore.
    fn color_image(&mut self, pm: &tiny_skia::Pixmap, m: Affine) {
        let (w, h) = (pm.width(), pm.height());
        let m = m.as_coeffs();
        let _ = writeln!(
            self.out,
            "[{} {} {} {} {} {}] concat",
            f(m[0] * MM_PT),
            f(m[1] * MM_PT),
            f(m[2] * MM_PT),
            f(m[3] * MM_PT),
            f(m[4] * MM_PT),
            f(m[5] * MM_PT)
        );
        let _ = writeln!(self.out, "{w} {h} 8 [{w} 0 0 -{h} 0 {h}]");
        let _ = writeln!(
            self.out,
            "{{currentfile {} string readhexstring pop}} false 3 colorimage",
            w * 3
        );
        let mut line = String::new();
        for p in pm.pixels() {
            let d = p.demultiply();
            let a = d.alpha() as u32;
            let r = (d.red() as u32 * a + 255 * (255 - a)) / 255;
            let g = (d.green() as u32 * a + 255 * (255 - a)) / 255;
            let b = (d.blue() as u32 * a + 255 * (255 - a)) / 255;
            let _ = write!(line, "{r:02x}{g:02x}{b:02x}");
            if line.len() >= 76 {
                let _ = writeln!(self.out, "{line}");
                line.clear();
            }
        }
        if !line.is_empty() {
            let _ = writeln!(self.out, "{line}");
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
            self.shape(&main, parent);
            for a in &ev.above {
                let mut a = a.clone();
                a.effects.clear();
                self.shape(&a, parent);
            }
            return;
        }
        let transform = parent * shape.transform;
        match &shape.kind {
            ShapeKind::Group { children } => {
                for c in children {
                    self.shape(c, transform);
                }
                return;
            }
            ShapeKind::Table(_) | ShapeKind::SymbolInstance { .. } => {
                for c in shape.expand(self.symbols) {
                    self.shape(&c, transform);
                }
                return;
            }
            ShapeKind::ClipFrame { frame, contents } => {
                let mut fill_only = (**frame).clone();
                fill_only.stroke = None;
                self.shape(&fill_only, transform);
                let _ = writeln!(self.out, "gsave");
                self.path_ops(&(transform * frame.page_path()));
                let _ = writeln!(self.out, "eoclip newpath");
                for c in contents {
                    self.shape(c, transform);
                }
                let _ = writeln!(self.out, "grestore");
                let mut outline = (**frame).clone();
                outline.fill = Fill::None;
                self.shape(&outline, transform);
                return;
            }
            ShapeKind::Bitmap { rect, png, .. } => {
                if let Ok(pm) = tiny_skia::Pixmap::decode_png(png) {
                    let m = transform
                        * Affine::new([rect.width(), 0.0, 0.0, rect.height(), rect.x0, rect.y0]);
                    let _ = writeln!(self.out, "gsave");
                    self.color_image(&pm, m);
                    let _ = writeln!(self.out, "grestore");
                }
                return;
            }
            _ => {}
        }
        let path = transform * shape.local_path();
        if path.elements().is_empty() {
            return;
        }
        let bounds = path.bounding_box();
        let even_odd = !matches!(shape.kind, ShapeKind::Text { .. });
        match &shape.fill {
            Fill::None => {}
            Fill::Solid(c) => {
                let _ = writeln!(self.out, "{}", color_op(*c));
                self.path_ops(&path);
                let _ = writeln!(self.out, "{}", if even_odd { "eofill" } else { "fill" });
            }
            Fill::Fountain(ft) => {
                let stops = {
                    let mut s = ft.stops.clone();
                    s.sort_by(|a, b| {
                        a.pos
                            .partial_cmp(&b.pos)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    if s.is_empty() {
                        s.push(tracedraw_core::Stop {
                            pos: 0.0,
                            color: Color::BLACK,
                        });
                    }
                    if s.len() == 1 {
                        let c = s[0].color;
                        s.push(tracedraw_core::Stop { pos: 1.0, color: c });
                    }
                    s
                };
                let func = if stops.len() == 2 {
                    format!(
                        "<< /FunctionType 2 /Domain [0 1] /C0 {} /C1 {} /N 1 >>",
                        rgb_array(stops[0].color),
                        rgb_array(stops[1].color)
                    )
                } else {
                    let mut funcs = String::new();
                    let mut bounds_s = String::new();
                    let mut enc = String::new();
                    let first = stops[0].pos;
                    let span = (stops[stops.len() - 1].pos - first).max(1e-6);
                    for w in stops.windows(2) {
                        let _ = write!(
                            funcs,
                            "<< /FunctionType 2 /Domain [0 1] /C0 {} /C1 {} /N 1 >> ",
                            rgb_array(w[0].color),
                            rgb_array(w[1].color)
                        );
                        enc.push_str("0 1 ");
                    }
                    for s in &stops[1..stops.len() - 1] {
                        let _ = write!(bounds_s, "{} ", f((s.pos - first) / span));
                    }
                    format!("<< /FunctionType 3 /Domain [0 1] /Functions [{}] /Bounds [{}] /Encode [{}] >>", funcs.trim_end(), bounds_s.trim_end(), enc.trim_end())
                };
                let c = bounds.center();
                let shading = match ft.kind {
                    FountainKind::Radial => {
                        let cx = (c.x + ft.offset.x * bounds.width() / 2.0) * MM_PT;
                        let cy = (c.y + ft.offset.y * bounds.height() / 2.0) * MM_PT;
                        let r = bounds.width().max(bounds.height()) / 2.0
                            * std::f64::consts::SQRT_2
                            * MM_PT;
                        format!("<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [{} {} 0 {} {} {}] /Function {} /Extend [true true] >>", f(cx), f(cy), f(cx), f(cy), f(r), func)
                    }
                    _ => {
                        let a = ft.angle.to_radians();
                        let half = (bounds.width() * a.cos().abs()
                            + bounds.height() * a.sin().abs())
                            / 2.0;
                        format!(
                            "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [{} {} {} {}] /Function {} /Extend [true true] >>",
                            f((c.x - half * a.cos()) * MM_PT),
                            f((c.y - half * a.sin()) * MM_PT),
                            f((c.x + half * a.cos()) * MM_PT),
                            f((c.y + half * a.sin()) * MM_PT),
                            func
                        )
                    }
                };
                let _ = writeln!(self.out, "gsave");
                self.path_ops(&path);
                let _ = writeln!(
                    self.out,
                    "{} {} shfill grestore",
                    if even_odd {
                        "eoclip newpath"
                    } else {
                        "clip newpath"
                    },
                    shading
                );
            }
            other @ (Fill::Pattern(_) | Fill::Texture(_) | Fill::Mesh(_)) => {
                // Patterns, textures and meshes: rasterised by the renderer
                // over the bounds and clipped by the path, so the file shows
                // the same pixels as the screen. Average colour if that fails.
                match tracedraw_render::render_fill_image(other, bounds, 150.0) {
                    Some(pm) => {
                        let _ = writeln!(self.out, "gsave");
                        self.path_ops(&path);
                        let _ = writeln!(
                            self.out,
                            "{}",
                            if even_odd {
                                "eoclip newpath"
                            } else {
                                "clip newpath"
                            }
                        );
                        let m = Affine::new([
                            bounds.width(),
                            0.0,
                            0.0,
                            bounds.height(),
                            bounds.x0,
                            bounds.y0,
                        ]);
                        self.color_image(&pm, m);
                        let _ = writeln!(self.out, "grestore");
                    }
                    None => {
                        let c = other.preview_color().unwrap_or(Color::Gray { v: 0.5 });
                        let _ = writeln!(self.out, "{}", color_op(c));
                        self.path_ops(&path);
                        let _ = writeln!(self.out, "{}", if even_odd { "eofill" } else { "fill" });
                    }
                }
            }
        }
        if let Some(s) = &shape.stroke {
            let width = if s.width <= tracedraw_core::Stroke::HAIRLINE + 1e-9 {
                0.0
            } else {
                s.width * MM_PT
            };
            let _ = writeln!(
                self.out,
                "{} {} setlinewidth {} setlinecap {} setlinejoin",
                color_op(s.color),
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
                let _ = writeln!(self.out, "[{}] 0 setdash", arr.join(" "));
            } else {
                let _ = writeln!(self.out, "[] 0 setdash");
            }
            self.path_ops(&path);
            let _ = writeln!(self.out, "stroke");
            // Arrowheads (presets and custom): the page-space path already
            // carries the shape's own transform, so only the parent applies.
            for head in tracedraw_core::arrowhead_paths(&shape.page_path(), s) {
                let _ = writeln!(self.out, "{}", color_op(s.color));
                self.path_ops(&(parent * head));
                let _ = writeln!(self.out, "fill");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;

    #[test]
    fn writes_eps_with_bounding_box_and_fill() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Solid(Color::cmyk_pct(100.0, 0.0, 0.0, 0.0));
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let eps = page_to_eps(&doc, 0);
        assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0"));
        assert!(eps.contains("%%BoundingBox: 0 0 596 842"));
        assert!(eps.contains("setcmykcolor"));
        assert!(eps.contains("eofill"));
    }

    #[test]
    fn arrowhead_follows_the_shape_transform_once() {
        // A line from (10, 50) to (60, 50) moved 100 mm up: the preset
        // arrow's tip must land at (60, 150), not at (60, 250).
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
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
            end_arrow: tracedraw_core::Arrowhead::Arrow,
            ..tracedraw_core::Stroke::new(Color::BLACK, 2.0)
        });
        line.transform = tracedraw_core::Affine::translate((0.0, 100.0));
        doc.layer_mut(layer).unwrap().shapes.push(line);
        let eps = page_to_eps(&doc, 0);
        let tip = format!("{} {} m\n", f(60.0 * MM_PT), f(150.0 * MM_PT));
        assert!(eps.contains(&tip), "tip at (60, 150) pt-scaled: {eps}");
        let wrong = format!("{} {} m\n", f(60.0 * MM_PT), f(250.0 * MM_PT));
        assert!(!eps.contains(&wrong));
        // A custom head goes through the same code path.
        let mut tri = tracedraw_core::BezPath::new();
        tri.move_to((0.0, 0.0));
        tri.line_to((20.0, 10.0));
        tri.line_to((0.0, 20.0));
        tri.close_path();
        if let Some(s) = doc.pages[0].layers[0].shapes.first_mut() {
            if let Some(st) = s.stroke.as_mut() {
                st.end_arrow = tracedraw_core::Arrowhead::from_shape_path(&tri, "Tri");
            }
        }
        let eps = page_to_eps(&doc, 0);
        let base = format!("{} {} m\n", f(52.0 * MM_PT), f(146.0 * MM_PT));
        assert!(eps.contains(&base), "custom head base corner: {eps}");
        assert!(eps.contains(&format!("{} {} l\n", f(60.0 * MM_PT), f(150.0 * MM_PT))));
    }

    #[test]
    fn vector_pattern_fill_is_a_clipped_image() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let mut tile_square = Shape::new(
            tracedraw_core::ShapeId(7),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 5.0, 5.0),
                radius: 0.0,
                corners: None,
            },
        );
        tile_square.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        tile_square.stroke = None;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Ellipse {
                rect: Rect::new(10.0, 10.0, 30.0, 30.0),
                arc: None,
            },
        );
        s.fill = Fill::Pattern(tracedraw_core::Pattern::Vector {
            shapes: vec![tile_square],
            tile: tracedraw_core::geometry::Size::new(10.0, 10.0),
        });
        s.stroke = None;
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let eps = page_to_eps(&doc, 0);
        assert!(eps.contains("eoclip newpath"));
        assert!(eps.contains("false 3 colorimage"));
        // 20 mm at 150 dpi is 119 px (ceil of 118.1).
        assert!(eps.contains("119 119 8 [119 0 0 -119 0 119]"), "{eps}");
        // Red pixels from the tile appear in the hex data.
        assert!(eps.contains("ff0000ff0000"));
    }
}
