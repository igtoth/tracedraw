//! SVG export. One page per SVG, in millimetres, with the Y axis flipped
//! from our bottom-up page space to SVG's top-down space.

use std::fmt::Write;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, PathEl, Shape as _},
    Color, Document, Fill, LineCap, LineJoin,
};

pub fn page_to_svg(doc: &Document, page_index: usize) -> String {
    let doc = &*crate::resolve_open_fills(doc);
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
    let layers = doc.layers_for_page(page.id).unwrap_or_default();
    for layer in layers {
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
                &doc.symbols,
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

/// A linked object inside a link (`<a>`) to its address; links of the
/// objects inside it are dropped, since links cannot nest. Page links
/// (`#page=n`) have no target in a one-page SVG and are left out.
fn write_shape(
    shape: &Shape,
    symbols: &[tracedraw_core::Symbol],
    parent: Affine,
    out: &mut String,
    defs: &mut String,
    next_grad: &mut usize,
    indent: usize,
) {
    // Bitmap effects on a vector object: its picture in its place.
    if let Some(pic) = shape.drawn_as() {
        return write_shape(&pic, symbols, parent, out, defs, next_grad, indent);
    }
    let link = shape
        .link
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("#page="));
    let Some(link) = link.filter(|_| shape.visible) else {
        write_unlinked(shape, symbols, parent, out, defs, next_grad, indent);
        return;
    };
    let pad = "  ".repeat(indent);
    let _ = writeln!(out, "{pad}<a href=\"{}\">", escape(link));
    let mut inner = shape.clone();
    drop_links(&mut inner);
    write_unlinked(&inner, symbols, parent, out, defs, next_grad, indent + 1);
    let _ = writeln!(out, "{pad}</a>");
}

/// `shape` and everything inside it without links.
fn drop_links(shape: &mut Shape) {
    shape.link = None;
    match &mut shape.kind {
        ShapeKind::Group { children } => children.iter_mut().for_each(drop_links),
        ShapeKind::ClipFrame { frame, contents } => {
            drop_links(frame);
            contents.iter_mut().for_each(drop_links);
        }
        _ => {}
    }
}

fn write_unlinked(
    shape: &Shape,
    symbols: &[tracedraw_core::Symbol],
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
    if !shape.effects.is_empty() {
        let ev = tracedraw_core::live::evaluate(shape);
        let _ = writeln!(
            out,
            "{pad}<g id=\"{}\" data-effects=\"{}\">",
            shape.id.raw(),
            shape.effects.len()
        );
        for b in &ev.below {
            let mut b = b.clone();
            b.effects.clear();
            write_shape(&b, symbols, parent, out, defs, next_grad, indent + 1);
        }
        let mut main = ev.main.clone();
        main.effects.clear();
        // Lens and non-uniform transparency have no SVG equivalent here;
        // approximate with the average opacity of the mask.
        for e in &shape.effects {
            match e {
                tracedraw_core::live::Effect::Transparency { mask, .. } => {
                    main.opacity *= average_luminance(mask) as f64;
                }
                tracedraw_core::live::Effect::Lens(tracedraw_core::live::Lens::Transparency {
                    rate,
                    color,
                }) => {
                    main.fill = Fill::Solid(*color);
                    main.opacity *= rate / 100.0;
                }
                tracedraw_core::live::Effect::Lens(_) => {
                    main.opacity *= 0.5;
                }
                _ => {}
            }
        }
        write_shape(&main, symbols, parent, out, defs, next_grad, indent + 1);
        for a in &ev.above {
            let mut a = a.clone();
            a.effects.clear();
            write_shape(&a, symbols, parent, out, defs, next_grad, indent + 1);
        }
        let _ = writeln!(out, "{pad}</g>");
        return;
    }
    // Characters with their own fill or outline: one text a look.
    if let Some(parts) = shape.text_parts() {
        let _ = writeln!(out, "{pad}<g id=\"{}\">", shape.id.raw());
        for p in &parts {
            write_unlinked(p, symbols, parent, out, defs, next_grad, indent + 1);
        }
        let _ = writeln!(out, "{pad}</g>");
        return;
    }
    let transform = parent * shape.transform;
    if let ShapeKind::Group { children } = &shape.kind {
        let _ = writeln!(out, "{pad}<g id=\"{}\">", shape.id.raw());
        for c in children {
            write_shape(c, symbols, transform, out, defs, next_grad, indent + 1);
        }
        let _ = writeln!(out, "{pad}</g>");
        return;
    }
    if matches!(
        shape.kind,
        ShapeKind::Table(_) | ShapeKind::SymbolInstance { .. }
    ) {
        let _ = writeln!(out, "{pad}<g id=\"{}\">", shape.id.raw());
        for c in shape.expand(symbols) {
            write_shape(&c, symbols, transform, out, defs, next_grad, indent + 1);
        }
        let _ = writeln!(out, "{pad}</g>");
        return;
    }
    if let (ShapeKind::ClipFrame { frame, contents }, Some(om)) = (&shape.kind, shape.opacity_mask)
    {
        // Opacity mask: the frame's fill is a luminance mask over the
        // contents (inverted through a colour matrix); the frame is not
        // drawn.
        let id = shape.id.raw();
        let mut fill_only = (**frame).clone();
        fill_only.stroke = None;
        let mut m = String::new();
        write_shape(&fill_only, symbols, transform, &mut m, defs, next_grad, 3);
        let mut mask = format!(
            "    <mask id=\"mask{id}\" maskUnits=\"userSpaceOnUse\" x=\"-100000\" y=\"-100000\" width=\"200000\" height=\"200000\">\n"
        );
        if om.invert {
            let _ = writeln!(
                defs,
                "    <filter id=\"maskinv{id}\"><feColorMatrix type=\"matrix\" values=\"-1 0 0 0 1 0 -1 0 0 1 0 0 -1 0 1 0 0 0 1 0\"/></filter>"
            );
            let _ = writeln!(mask, "      <g filter=\"url(#maskinv{id})\">");
            mask.push_str(&m);
            mask.push_str("      </g>\n");
        } else {
            mask.push_str(&m);
        }
        mask.push_str("    </mask>\n");
        defs.push_str(&mask);
        let opacity = if shape.opacity < 1.0 {
            format!(" opacity=\"{}\"", fmt(shape.opacity))
        } else {
            String::new()
        };
        let _ = writeln!(out, "{pad}<g id=\"{id}\" mask=\"url(#mask{id})\"{opacity}>");
        for c in contents {
            write_shape(c, symbols, transform, out, defs, next_grad, indent + 1);
        }
        let _ = writeln!(out, "{pad}</g>");
        return;
    }
    if let ShapeKind::ClipFrame { frame, contents } = &shape.kind {
        // Frame fill, then the contents clipped to the frame, then its outline.
        let mut fill_only = (**frame).clone();
        fill_only.stroke = None;
        write_shape(&fill_only, symbols, transform, out, defs, next_grad, indent);
        let clip_id = format!("clip{}", shape.id.raw());
        let _ = writeln!(
            defs,
            "    <clipPath id=\"{clip_id}\"><path d=\"{}\"/></clipPath>",
            path_data(&(transform * frame.page_path()))
        );
        let _ = writeln!(
            out,
            "{pad}<g id=\"{}\" clip-path=\"url(#{clip_id})\">",
            shape.id.raw()
        );
        for c in contents {
            write_shape(c, symbols, transform, out, defs, next_grad, indent + 1);
        }
        let _ = writeln!(out, "{pad}</g>");
        let mut outline = (**frame).clone();
        outline.fill = Fill::None;
        if outline.stroke.is_some() {
            write_shape(&outline, symbols, transform, out, defs, next_grad, indent);
        }
        return;
    }
    // Further fills and outlines: the parts in a group, bottom first.
    if let Some(parts) = shape.layers() {
        let _ = writeln!(out, "{pad}<g id=\"{}\">", shape.id.raw());
        for p in &parts {
            write_shape(p, symbols, parent, out, defs, next_grad, indent + 1);
        }
        let _ = writeln!(out, "{pad}</g>");
        return;
    }
    if let ShapeKind::Bitmap {
        rect,
        width_px,
        height_px,
        png,
        fx: _,
    } = &shape.kind
    {
        // Image space (pixels, y down) to the local rect to the page.
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(png);
        let m = transform
            * Affine::new([
                rect.width() / (*width_px).max(1) as f64,
                0.0,
                0.0,
                -(rect.height() / (*height_px).max(1) as f64),
                rect.x0,
                rect.y1,
            ]);
        let c = m.as_coeffs();
        let opacity = if shape.opacity < 1.0 {
            format!(" opacity=\"{}\"", fmt(shape.opacity))
        } else {
            String::new()
        };
        let _ = writeln!(
            out,
            "{pad}<image id=\"{}\" width=\"{width_px}\" height=\"{height_px}\" transform=\"matrix({} {} {} {} {} {})\" preserveAspectRatio=\"none\"{opacity} href=\"data:image/png;base64,{b64}\"/>",
            shape.id.raw(),
            fmt(c[0]),
            fmt(c[1]),
            fmt(c[2]),
            fmt(c[3]),
            fmt(c[4]),
            fmt(c[5])
        );
        return;
    }
    let path = transform * shape.local_path();
    let d = path_data(&path);
    let bounds = path.bounding_box();

    let fill_attr = format!(
        "fill=\"{}\"",
        paint_value(&shape.fill, bounds, symbols, defs, next_grad)
    );

    // A calligraphic, variable, inside or outside outline, or one with
    // dashes fitted to the corners, has no SVG equivalent: its area is
    // written as a filled path after the object.
    let calligraphic = shape.stroke.as_ref().and_then(|s| {
        tracedraw_core::shaping::outline_band(&shape.page_path(), s, 1.0).map(|b| {
            let band = parent * b;
            let paint = outline_paint_value(s, band.bounding_box(), symbols, defs, next_grad);
            (band, paint)
        })
    });
    let stroke_attr = match &shape.stroke {
        None => "stroke=\"none\"".to_string(),
        Some(s) if calligraphic.is_some() || s.painterly.is_some() => "stroke=\"none\"".to_string(),
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
            // The outline's own paint spreads over the outline's bounds,
            // as on screen.
            let half = s.width / 2.0;
            let paint =
                outline_paint_value(s, bounds.inflate(half, half), symbols, defs, next_grad);
            let mut a = format!(
                "stroke=\"{paint}\" stroke-width=\"{}\" stroke-linecap=\"{cap}\" stroke-linejoin=\"{join}\"",
                s.width
            );
            if s.join == LineJoin::Miter {
                let _ = write!(a, " stroke-miterlimit=\"{}\"", fmt(s.miter_ratio()));
            }
            if !s.dash.is_empty() {
                let dashes: Vec<String> = s.dash.iter().map(|d| fmt(d * s.width)).collect();
                let _ = write!(a, " stroke-dasharray=\"{}\"", dashes.join(" "));
            }
            // An outline behind the fill: painted first, the fill over it.
            if s.behind_fill {
                a.push_str(" paint-order=\"stroke\"");
            }
            a
        }
    };

    let name = shape
        .name
        .as_deref()
        .map(|n| format!(" data-name=\"{}\"", escape(n)))
        .unwrap_or_default();
    // Shapes fill even-odd, as the renderer draws them (holes in combined
    // curves, the middle of a complex star); text keeps the default.
    let rule = if matches!(shape.kind, ShapeKind::Text { .. }) {
        ""
    } else {
        " fill-rule=\"evenodd\""
    };
    let _ = writeln!(
        out,
        "{pad}<path id=\"{}\"{name} d=\"{d}\" {fill_attr}{rule} {stroke_attr}/>",
        shape.id.raw()
    );
    if let Some((band, paint)) = calligraphic {
        let _ = writeln!(
            out,
            "{pad}<path d=\"{}\" fill=\"{paint}\" stroke=\"none\"/>",
            path_data(&band)
        );
    }
    // A painterly outline: its marks, one path per opacity step.
    if let Some((s, brush)) = shape
        .stroke
        .as_ref()
        .and_then(|s| s.painterly.as_ref().map(|b| (s, b)))
    {
        for (alpha, marks) in
            tracedraw_core::painterly::marks_by_opacity(&shape.page_path(), s.width, brush, 12)
        {
            let _ = writeln!(
                out,
                "{pad}<path d=\"{}\" fill=\"{}\" fill-opacity=\"{}\" stroke=\"none\"/>",
                path_data(&(parent * marks)),
                s.color.to_hex(),
                fmt(alpha)
            );
        }
    }
    // Arrowheads (presets and custom) are filled outlines in the stroke
    // colour. They are built in page space, where the curve direction is
    // unmirrored, and then taken through the parent transform and the flip.
    if let Some(s) = &shape.stroke {
        for head in tracedraw_core::arrowhead_paths(&shape.page_path(), s) {
            let _ = writeln!(
                out,
                "{pad}<path d=\"{}\" fill=\"{}\" stroke=\"none\"/>",
                path_data(&(parent * head)),
                s.color.to_hex()
            );
        }
    }
}

/// The SVG paint of an outline: its own paint (a gradient, pattern,
/// texture or mesh) over `bounds`, or its colour.
fn outline_paint_value(
    s: &tracedraw_core::Stroke,
    bounds: tracedraw_core::Rect,
    symbols: &[tracedraw_core::Symbol],
    defs: &mut String,
    next_grad: &mut usize,
) -> String {
    match s.fill.as_deref() {
        Some(fill) if !matches!(fill, Fill::None | Fill::Solid(_)) => {
            paint_value(fill, bounds, symbols, defs, next_grad)
        }
        _ => s.color.to_hex(),
    }
}

/// The SVG paint of `fill` over `bounds` (output space): `none`, a
/// colour, or a reference to a gradient or pattern written into `defs`.
/// Fills SVG cannot draw (textures, meshes) give a representative colour.
fn paint_value(
    fill: &Fill,
    bounds: tracedraw_core::Rect,
    symbols: &[tracedraw_core::Symbol],
    defs: &mut String,
    next_grad: &mut usize,
) -> String {
    match fill {
        Fill::None => "none".to_string(),
        Fill::Solid(c) => c.to_hex(),
        Fill::Fountain(f) => {
            *next_grad += 1;
            let id = format!("grad{}", *next_grad);
            // The edge pad, repeats, mirror and inversion as one run of
            // stops; a shaped fountain (0 on the edge) runs from the
            // centre out as a radial one.
            let mut stops = f.resolved_stops();
            if f.kind == tracedraw_core::FountainKind::Shaped {
                stops.reverse();
                for s in &mut stops {
                    s.pos = 1.0 - s.pos;
                }
            }
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
                tracedraw_core::FountainKind::Shaped => {
                    // SVG has no gradient that follows an outline: a radial
                    // one reaching the nearer sides stands in for it.
                    let c = bounds.center();
                    let r = bounds.width().min(bounds.height()).max(1e-6) / 2.0;
                    let _ = writeln!(defs, "    <radialGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" cx=\"{}\" cy=\"{}\" r=\"{}\">\n{stop_xml}    </radialGradient>", c.x, c.y, r);
                }
            }
            format!("url(#{id})")
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
            format!("url(#{id})")
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
            format!("url(#{id})")
        }
        Fill::Pattern(tracedraw_core::Pattern::Vector {
            shapes: tile_shapes,
            tile,
        }) => {
            *next_grad += 1;
            let id = format!("pat{}", *next_grad);
            // The tile's content stays vector: its shapes are written into
            // the pattern with the tile's own Y flip (tile space is Y up,
            // origin bottom-left). Gradients they use land in `defs` too.
            let tile_flip = Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, tile.height]);
            let mut inner = String::new();
            for s in tile_shapes {
                write_shape(s, symbols, tile_flip, &mut inner, defs, next_grad, 3);
            }
            let _ = writeln!(
            defs,
            "    <pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" width=\"{}\" height=\"{}\">\n{inner}    </pattern>",
            fmt(tile.width),
            fmt(tile.height)
        );
            format!("url(#{id})")
        }
        Fill::Texture(t) => {
            // Approximate with the average of the two colours.
            tracedraw_core::style::lerp_color(t.color_a, t.color_b, 0.5).to_hex()
        }
        Fill::Mesh(m) => {
            // Average of the node colours; the renderer image path is exact.
            let n = m.nodes.len().max(1) as f32;
            let mut acc = [0.0f32; 3];
            for node in &m.nodes {
                let [r, g, b] = node.color.to_rgb_f32();
                acc[0] += r;
                acc[1] += g;
                acc[2] += b;
            }
            Color::Rgb {
                r: acc[0] / n,
                g: acc[1] / n,
                b: acc[2] / n,
            }
            .to_hex()
        }
    }
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

/// Mean luminance (0..1) of a mask fill, for exporters without masks.
pub fn average_luminance(fill: &Fill) -> f32 {
    match fill {
        Fill::None => 1.0,
        Fill::Solid(c) => c.luminance(),
        Fill::Fountain(f) => {
            let n = f.stops.len().max(1) as f32;
            f.stops.iter().map(|s| s.color.luminance()).sum::<f32>() / n
        }
        Fill::Pattern(tracedraw_core::Pattern::TwoColor { front, back, .. }) => {
            (front.luminance() + back.luminance()) / 2.0
        }
        Fill::Pattern(_) => 0.5,
        Fill::Texture(t) => (t.color_a.luminance() + t.color_b.luminance()) / 2.0,
        Fill::Mesh(m) => {
            let n = m.nodes.len().max(1) as f32;
            m.nodes.iter().map(|x| x.color.luminance()).sum::<f32>() / n
        }
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
    fn opacity_masks_are_svg_masks() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let contents = Shape::new(
            doc.ids_mut().shape(),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 40.0, 40.0),
                radius: 0.0,
                corners: None,
            },
        );
        let mut frame = Shape::new(
            doc.ids_mut().shape(),
            ShapeKind::Rect {
                rect: Rect::new(5.0, 5.0, 35.0, 35.0),
                radius: 0.0,
                corners: None,
            },
        );
        frame.fill = Fill::Solid(Color::WHITE);
        let mut clip = Shape::new(
            doc.ids_mut().shape(),
            ShapeKind::ClipFrame {
                frame: Box::new(frame),
                contents: vec![contents],
            },
        );
        let id = clip.id.raw();
        clip.opacity_mask = Some(tracedraw_core::document::OpacityMask { invert: true });
        doc.layer_mut(layer).unwrap().shapes.push(clip);
        let svg = page_to_svg(&doc, 0);
        assert!(svg.contains(&format!("<mask id=\"mask{id}\"")), "{svg}");
        assert!(svg.contains(&format!("mask=\"url(#mask{id})\"")));
        assert!(svg.contains("feColorMatrix"), "inverted");
        assert!(!svg.contains("clip-path"));
    }

    #[test]
    fn linked_objects_sit_inside_links() {
        let mut doc = Document::default();
        let layer = &mut doc.pages[0].layers[0];
        let rect = |id: u64, link: Option<&str>| {
            let mut s = Shape::new(
                tracedraw_core::ShapeId(id),
                ShapeKind::Rect {
                    rect: Rect::new(10.0, 10.0, 20.0, 20.0),
                    radius: 0.0,
                    corners: None,
                },
            );
            s.link = link.map(String::from);
            s
        };
        let mut group = Shape::new(
            tracedraw_core::ShapeId(10),
            ShapeKind::Group {
                children: vec![rect(11, Some("https://inner.example")), rect(12, None)],
            },
        );
        group.link = Some("https://example.com/?a=1&b=2".into());
        layer.shapes = vec![
            group,
            rect(20, Some("#page=2")),
            rect(21, Some("mailto:a@b.c")),
        ];
        let svg = page_to_svg(&doc, 0);
        assert!(
            svg.contains("<a href=\"https://example.com/?a=1&amp;b=2\">"),
            "{svg}"
        );
        // Links do not nest; page links have no target in one page.
        assert!(!svg.contains("inner.example") && !svg.contains("#page"));
        assert!(svg.contains("<a href=\"mailto:a@b.c\">"));
        assert_eq!(svg.matches("<a ").count(), 2);
        assert_eq!(svg.matches("</a>").count(), 2);
    }

    /// Characters with their own colour: the text goes out as one part
    /// a look, in a group, each part with its colour.
    #[test]
    fn characters_with_their_own_colour_are_written_in_parts() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut red = tracedraw_core::TextSpan::new("b", "Any", 12.0);
        red.fill = Some(Fill::Solid(Color::rgb8(255, 0, 0)));
        let mut s = Shape::new(
            id,
            ShapeKind::Text {
                spans: vec![tracedraw_core::TextSpan::new("a", "Any", 12.0), red],
                origin: tracedraw_core::Point::ZERO,
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(0, 0, 255));
        s.stroke = None;
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let svg = page_to_svg(&doc, 0);
        assert!(svg.contains(&format!("<g id=\"{}\">", id.raw())), "{svg}");
        assert!(svg.contains("fill=\"#0000ff\""), "{svg}");
        assert!(svg.contains("fill=\"#ff0000\""), "{svg}");
    }

    #[test]
    fn an_outline_behind_the_fill_paints_first() {
        let svg_of = |behind: bool| {
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
            s.fill = Fill::Solid(Color::rgb8(0, 0, 0));
            s.stroke = Some(Stroke {
                behind_fill: behind,
                ..Stroke::new(Color::rgb8(255, 255, 255), 2.0)
            });
            doc.layer_mut(layer).unwrap().shapes.push(s);
            page_to_svg(&doc, 0)
        };
        assert!(svg_of(true).contains("paint-order=\"stroke\""));
        assert!(!svg_of(false).contains("paint-order"));
    }

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
                corners: None,
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

    #[test]
    fn shapes_fill_even_odd_like_the_renderer() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Polygon {
                rect: Rect::new(10.0, 10.0, 60.0, 60.0),
                points: 5,
                sharpness: 0.0,
                complex: Some(1),
                twist: 0.0,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let svg = page_to_svg(&doc, 0);
        let line = svg
            .lines()
            .find(|l| l.contains("fill=\"#ff0000\""))
            .expect("the star");
        assert!(line.contains("fill-rule=\"evenodd\""), "{line}");
    }

    #[test]
    fn calligraphic_outline_becomes_a_filled_band() {
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
        s.fill = Fill::None;
        let mut st = Stroke::new(Color::rgb8(0, 0, 255), 3.0);
        st.stretch = 0.2;
        st.nib_angle = 45.0;
        s.stroke = Some(st);
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let svg = page_to_svg(&doc, 0);
        // The object itself carries no stroke; the band is a blue fill.
        assert!(!svg.contains("stroke-width=\"3\""));
        assert!(svg.contains("fill=\"#0000ff\" stroke=\"none\""));
        let band_start = svg.find("fill=\"#0000ff\"").unwrap();
        let line = svg[..band_start].rsplit('\n').next().unwrap();
        // A real outline of the rectangle: dozens of vertices.
        assert!(line.matches('L').count() > 20, "{line}");
    }

    #[test]
    fn variable_outlines_become_a_filled_band_and_survive_the_native_format() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut path = tracedraw_core::geometry::BezPath::new();
        path.move_to(tracedraw_core::Point::new(10.0, 50.0));
        path.line_to(tracedraw_core::Point::new(110.0, 50.0));
        let mut s = Shape::new(
            id,
            ShapeKind::Path {
                path,
                closed: false,
            },
        );
        let mut st = Stroke::new(Color::rgb8(255, 0, 0), 1.0);
        st.widths
            .push(tracedraw_core::WidthNode::symmetric(0.5, 8.0));
        s.stroke = Some(st.clone());
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let svg = page_to_svg(&doc, 0);
        assert!(svg.contains("fill=\"#ff0000\" stroke=\"none\""), "{svg}");
        assert!(!svg.contains("stroke-width=\"1\""));
        // The native format keeps the nodes.
        let file = std::env::temp_dir().join(format!("vo-{}.tdraw", std::process::id()));
        crate::save_native(&doc, &file).unwrap();
        let back = crate::load_native(&file).unwrap();
        let _ = std::fs::remove_file(&file);
        let widths = back.pages[0].layers[0].shapes[0]
            .stroke
            .as_ref()
            .map(|s| s.widths.clone());
        assert_eq!(widths, Some(st.widths));
    }

    #[test]
    fn painterly_outlines_are_written_as_marks_by_opacity() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut path = tracedraw_core::geometry::BezPath::new();
        path.move_to(tracedraw_core::Point::new(10.0, 50.0));
        path.line_to(tracedraw_core::Point::new(110.0, 50.0));
        let mut s = Shape::new(
            id,
            ShapeKind::Path {
                path,
                closed: false,
            },
        );
        let mut st = Stroke::new(Color::rgb8(0, 128, 0), 6.0);
        st.painterly = Some(tracedraw_core::painterly::Painterly::new(
            tracedraw_core::painterly::PainterlyBrush::Watercolor,
        ));
        s.stroke = Some(st);
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let svg = page_to_svg(&doc, 0);
        assert!(svg.contains("fill=\"#008000\" fill-opacity=\""), "{svg}");
        assert!(!svg.contains("stroke-width=\"6\""));
        let pdf = crate::pdf::document_to_pdf(&doc);
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("/ca "));
    }

    /// A line from (10, 50) to (60, 50), 2 mm wide, with a custom
    /// triangular end head, moved 100 mm up by its transform.
    fn arrow_line() -> Shape {
        let mut tri = tracedraw_core::geometry::BezPath::new();
        tri.move_to((0.0, 0.0));
        tri.line_to((20.0, 10.0));
        tri.line_to((0.0, 20.0));
        tri.close_path();
        let mut line = Shape::new(
            tracedraw_core::ShapeId(1),
            ShapeKind::Path {
                path: {
                    let mut p = tracedraw_core::geometry::BezPath::new();
                    p.move_to((10.0, 50.0));
                    p.line_to((60.0, 50.0));
                    p
                },
                closed: false,
            },
        );
        line.fill = Fill::None;
        line.stroke = Some(Stroke {
            end_arrow: tracedraw_core::Arrowhead::from_shape_path(&tri, "Tri"),
            ..Stroke::new(Color::rgb8(0, 0, 255), 2.0)
        });
        line.transform = Affine::translate((0.0, 100.0));
        line
    }

    #[test]
    fn custom_arrowhead_is_written_as_a_filled_path_at_the_line_end() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        doc.layer_mut(layer).unwrap().shapes.push(arrow_line());
        let svg = page_to_svg(&doc, 0);
        // The head is 8 mm (four widths) long and tall: base corners at
        // (52, 154) and (52, 146) in page space, tip at (60, 150); SVG
        // flips y against the 297 mm page.
        let head = svg
            .lines()
            .find(|l| l.contains("fill=\"#0000ff\" stroke=\"none\""))
            .expect("head path");
        assert!(head.contains("52 143"), "{head}");
        assert!(head.contains("60 147"), "{head}");
        assert!(head.contains("52 151"), "{head}");
        assert!(
            head.ends_with("Z\" fill=\"#0000ff\" stroke=\"none\"/>"),
            "{head}"
        );
        // No head: no extra path.
        if let Some(s) = doc.pages[0].layers[0].shapes.first_mut() {
            if let Some(st) = s.stroke.as_mut() {
                st.end_arrow = tracedraw_core::Arrowhead::None;
            }
        }
        let svg = page_to_svg(&doc, 0);
        assert!(!svg.contains("fill=\"#0000ff\" stroke=\"none\""));
    }

    #[test]
    fn vector_pattern_exports_as_svg_pattern_with_vector_content() {
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
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 30.0, 30.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Pattern(tracedraw_core::Pattern::Vector {
            shapes: vec![tile_square],
            tile: tracedraw_core::geometry::Size::new(10.0, 10.0),
        });
        s.stroke = None;
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let svg = page_to_svg(&doc, 0);
        let start = svg.find("<pattern id=\"pat1\"").expect("pattern element");
        let end = svg[start..].find("</pattern>").expect("pattern end") + start;
        let pattern = &svg[start..end];
        assert!(pattern.contains("patternUnits=\"userSpaceOnUse\" width=\"10\" height=\"10\""));
        // The tile content is a vector path, not an image, flipped into the
        // tile's top-down space (y 0..5 becomes 5..10).
        assert!(pattern.contains("<path id=\"7\""), "{pattern}");
        assert!(pattern.contains("fill=\"#ff0000\""), "{pattern}");
        assert!(pattern.contains("M0 10L5 10L5 5L0 5Z"), "{pattern}");
        assert!(!pattern.contains("<image"));
        assert!(svg.contains("fill=\"url(#pat1)\""));
    }

    fn one_shape_svg(fill: Fill, stroke: Option<Stroke>) -> String {
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
        s.fill = fill;
        s.stroke = stroke;
        doc.layer_mut(layer).unwrap().shapes.push(s);
        page_to_svg(&doc, 0)
    }

    #[test]
    fn positioned_outlines_are_written_as_filled_areas() {
        let mut st = Stroke::new(Color::rgb8(255, 0, 0), 2.0);
        // Centred: a stroke with its mitre limit.
        let svg = one_shape_svg(Fill::None, Some(st.clone()));
        assert!(svg.contains("stroke-miterlimit=\"22.9"), "{svg}");
        // Inside: no stroke on the object, a filled path in the colour.
        st.position = tracedraw_core::OutlinePosition::Inside;
        let svg = one_shape_svg(Fill::None, Some(st.clone()));
        assert!(!svg.contains("stroke-width"), "{svg}");
        assert!(svg.contains("fill=\"#ff0000\""), "{svg}");
        // Aligned dashes too.
        st.position = tracedraw_core::OutlinePosition::Center;
        st.dash = vec![3.0, 2.0];
        st.align_dashes = true;
        let svg = one_shape_svg(Fill::None, Some(st));
        assert!(!svg.contains("stroke-dasharray"), "{svg}");
        assert!(svg.contains("fill=\"#ff0000\""), "{svg}");
    }

    #[test]
    fn outline_paints_are_written_as_the_stroke_paint() {
        use tracedraw_core::{Fountain, FountainKind};
        let red = Color::rgb8(255, 0, 0);
        let blue = Color::rgb8(0, 0, 255);
        // A gradient outline strokes with the gradient.
        let mut st = Stroke::new(Color::rgb8(0, 255, 0), 2.0);
        st.fill = Some(Box::new(Fill::linear(red, blue, 0.0)));
        let svg = one_shape_svg(Fill::None, Some(st.clone()));
        assert!(svg.contains("stroke=\"url(#grad1)\""), "{svg}");
        assert!(svg.contains("<linearGradient id=\"grad1\""), "{svg}");
        assert!(!svg.contains("#00ff00"), "{svg}");
        // A pattern outline strokes with the pattern.
        st.fill = Some(Box::new(Fill::Pattern(tracedraw_core::Pattern::TwoColor {
            tile: tracedraw_core::PatternTile::Checker,
            front: red,
            back: blue,
            size_mm: 5.0,
        })));
        let svg = one_shape_svg(Fill::None, Some(st.clone()));
        assert!(svg.contains("stroke=\"url(#pat1)\""), "{svg}");
        // A texture outline: a colour between its two, still an outline.
        st.fill = Some(Box::new(Fill::Texture(tracedraw_core::Texture {
            kind: tracedraw_core::TextureKind::Clouds,
            color_a: red,
            color_b: blue,
            scale: 5.0,
            seed: 1,
        })));
        let svg = one_shape_svg(Fill::None, Some(st.clone()));
        assert!(
            svg.contains("stroke=\"#800080\" stroke-width=\"2\""),
            "{svg}"
        );
        // A calligraphic band takes the paint as its fill.
        st.stretch = 0.3;
        st.fill = Some(Box::new(Fill::radial(red, blue)));
        let svg = one_shape_svg(Fill::None, Some(st));
        assert!(svg.contains("stroke=\"none\"/>\n"), "{svg}");
        assert!(
            svg.contains("fill=\"url(#grad1)\" stroke=\"none\""),
            "{svg}"
        );
        // Gradient options are worked into the stops; a shaped gradient
        // is written as a radial one, its inside colour at the centre.
        let mut f = Fountain::two(FountainKind::Shaped, red, blue, 0.0);
        f.invert = true;
        let svg = one_shape_svg(Fill::Fountain(f), None);
        assert!(svg.contains("<radialGradient id=\"grad1\""), "{svg}");
        let first = svg.find("<stop offset=\"0\" stop-color=\"").expect("stop");
        assert!(
            svg[first..].starts_with("<stop offset=\"0\" stop-color=\"#ff0000\""),
            "{svg}"
        );
        let mut saw = Fountain::two(FountainKind::Linear, red, blue, 0.0);
        saw.repeat = tracedraw_core::FountainRepeat::Repeat;
        saw.repeats = 3;
        let svg = one_shape_svg(Fill::Fountain(saw), None);
        assert_eq!(svg.matches("<stop ").count(), 6, "{svg}");
    }
}
