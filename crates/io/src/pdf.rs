//! Minimal PDF writer: one page per document page, vector paths with
//! uniform fills (RGB or CMYK), axial/radial shadings for fountain fills,
//! outlines with width, caps, joins and dashes, bitmaps as images, and
//! uniform transparency via ExtGState. Written by hand; no dependency.

use std::fmt::Write as _;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, PathEl, Rect},
    Color, Document, Fill, FountainKind, LineCap, LineJoin,
};

const MM_PT: f64 = 72.0 / 25.4;

/// PDF/X conformance level of the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PdfStandard {
    /// Plain PDF 1.4.
    #[default]
    None,
    /// PDF/X-1a:2003: CMYK and gray only, no transparency, output intent.
    X1a,
    /// PDF/X-3:2003: like X-1a but RGB allowed under the output intent.
    X3,
    /// PDF/X-4: transparency allowed (PDF 1.6).
    X4,
}

impl PdfStandard {
    pub fn name(self) -> &'static str {
        match self {
            PdfStandard::None => "PDF",
            PdfStandard::X1a => "PDF/X-1a:2003",
            PdfStandard::X3 => "PDF/X-3:2003",
            PdfStandard::X4 => "PDF/X-4",
        }
    }
    fn version(self) -> &'static str {
        match self {
            PdfStandard::X4 => "1.6",
            _ => "1.4",
        }
    }
    /// Every colour must be CMYK or gray.
    fn cmyk_only(self) -> bool {
        self == PdfStandard::X1a
    }
    /// Transparency has to be flattened.
    fn flatten(self) -> bool {
        matches!(self, PdfStandard::X1a | PdfStandard::X3)
    }
}

/// Options for `document_to_pdf_with`.
#[derive(Debug, Clone, Default)]
pub struct PdfOptions {
    pub standard: PdfStandard,
    /// ICC profile embedded as the output intent's destination profile
    /// (a CMYK press profile for X-1a). Without one, a registered
    /// characterisation name is used instead.
    pub output_profile: Option<Vec<u8>>,
    /// Output condition identifier, e.g. "FOGRA39" or "CGATS TR 001".
    pub output_condition: String,
    /// Bleed added around every page (TrimBox = page, MediaBox = page + bleed).
    pub bleed_mm: f64,
    pub title: Option<String>,
    /// Flattening resolution for X-1a/X-3 transparency.
    pub flatten_dpi: f64,
}

struct Pdf {
    objects: Vec<Vec<u8>>,
    cmyk_only: bool,
    allow_alpha: bool,
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
        if self.cmyk_only {
            let cmyk = rgb_to_cmyk_bytes(rgb);
            let data = flate(&cmyk);
            return self.stream(
                &format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Filter /FlateDecode"),
                &data,
            );
        }
        let data = flate(rgb);
        self.stream(
            &format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode"),
            &data,
        )
    }
    fn color_space(&self) -> &'static str {
        if self.cmyk_only {
            "/DeviceCMYK"
        } else {
            "/DeviceRGB"
        }
    }

    fn stream(&mut self, dict: &str, data: &[u8]) -> usize {
        let mut v = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
        v.extend_from_slice(data);
        v.extend_from_slice(b"\nendstream");
        self.add(v)
    }
    fn finish_with(self, version: &str, info_id: Option<usize>) -> Vec<u8> {
        use std::io::Write;
        let mut out = format!("%PDF-{version}\n").into_bytes();
        out.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");
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
        let id = {
            // Two FNV-1a hashes of the body as the file identifier.
            let h1 = fnv1a(&out, 0xcbf29ce484222325);
            let h2 = fnv1a(&out, 0x84222325cbf29ce4);
            format!("<{h1:016x}{h2:016x}> <{h1:016x}{h2:016x}>")
        };
        let info = info_id
            .map(|i| format!(" /Info {i} 0 R"))
            .unwrap_or_default();
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R{info} /ID [{id}] >>\nstartxref\n{xref}\n%%EOF\n",
            self.objects.len() + 1
        );
        out
    }
}

fn fnv1a(data: &[u8], seed: u64) -> u64 {
    let mut h = seed;
    for b in data {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// RGB8 samples to CMYK8 with the built-in conversion.
fn rgb_to_cmyk_bytes(rgb: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgb.len() / 3 * 4);
    for px in rgb.chunks_exact(3) {
        let (r, g, b) = (
            px[0] as f32 / 255.0,
            px[1] as f32 / 255.0,
            px[2] as f32 / 255.0,
        );
        let k = 1.0 - r.max(g).max(b);
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        if k >= 1.0 - 1e-6 {
            out.extend_from_slice(&[0, 0, 0, 255]);
        } else {
            out.extend_from_slice(&[
                q((1.0 - r - k) / (1.0 - k)),
                q((1.0 - g - k) / (1.0 - k)),
                q((1.0 - b - k) / (1.0 - k)),
                q(k),
            ]);
        }
    }
    out
}

/// PDF date string for now (UTC).
fn pdf_date() -> String {
    // web_time is std's clock natively and the JavaScript clock in a browser.
    let secs = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, mi, sec) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("D:{y:04}{m:02}{d:02}{h:02}{mi:02}{sec:02}Z")
}

fn pdf_text(s: &str) -> String {
    let esc: String = s
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            '(' => "\\(".to_string(),
            ')' => "\\)".to_string(),
            '\\' => "\\\\".to_string(),
            c if (c as u32) < 128 => c.to_string(),
            c => format!("\\{:03o}", (c as u32).min(255)),
        })
        .collect();
    format!("({esc})")
}

fn f(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn color_op_in(c: Color, fill: bool, cmyk_only: bool) -> String {
    let c = if cmyk_only && !matches!(c, Color::Cmyk { .. } | Color::Gray { .. }) {
        c.convert_to("CMYK")
    } else {
        c
    };
    match c {
        Color::Gray { v } if cmyk_only => {
            format!("{} {}", f(v as f64), if fill { "g" } else { "G" })
        }
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

fn color_array(c: Color, cmyk_only: bool) -> String {
    if cmyk_only {
        if let Color::Cmyk { c, m, y, k } = c.convert_to("CMYK") {
            return format!(
                "[{} {} {} {}]",
                f(c as f64),
                f(m as f64),
                f(y as f64),
                f(k as f64)
            );
        }
    }
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
            if shape.opacity < 1.0 && self.pdf.allow_alpha {
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
        if shape.opacity < 1.0 && self.pdf.allow_alpha {
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
            fx: _,
        } = &shape.kind
        {
            if let Some((mut rgb, alpha)) = decode_png_rgb_alpha(png) {
                let name = format!("Im{}", self.images.len());
                let alpha = if self.pdf.allow_alpha {
                    alpha
                } else {
                    // No soft masks: composite over white.
                    if let Some(a) = &alpha {
                        for (px, a) in rgb.chunks_exact_mut(3).zip(a.iter()) {
                            let a = *a as u32;
                            for c in px.iter_mut() {
                                *c = ((*c as u32 * a + 255 * (255 - a)) / 255) as u8;
                            }
                        }
                    }
                    None
                };
                let cs = self.pdf.color_space();
                let data = if self.pdf.cmyk_only {
                    flate(&rgb_to_cmyk_bytes(&rgb))
                } else {
                    flate(&rgb)
                };
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
                    &format!("/Type /XObject /Subtype /Image /Width {width_px} /Height {height_px} /ColorSpace {cs} /BitsPerComponent 8 /Filter /FlateDecode{smask_ref}"),
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
                let _ = writeln!(
                    self.content,
                    "{}",
                    color_op_in(*c, true, self.pdf.cmyk_only)
                );
                self.path_ops(&path);
                self.content
                    .push_str(if matches!(shape.kind, ShapeKind::Text { .. }) {
                        "f\n"
                    } else {
                        "f*\n"
                    });
            }
            Fill::Fountain(ft) => {
                let func = stitching_function_in(ft, self.pdf.cmyk_only);
                let shading = match ft.kind {
                    FountainKind::Radial => {
                        let c = bounds.center();
                        let cx = (c.x + ft.offset.x * bounds.width() / 2.0) * MM_PT;
                        let cy = (c.y + ft.offset.y * bounds.height() / 2.0) * MM_PT;
                        let r = bounds.width().max(bounds.height()) / 2.0
                            * std::f64::consts::SQRT_2
                            * MM_PT;
                        format!(
                            "<< /ShadingType 3 /ColorSpace {} /Coords [{} {} 0 {} {} {}] /Function {} /Extend [true true] >>",
                            self.pdf.color_space(), f(cx), f(cy), f(cx), f(cy), f(r), func
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
                            "<< /ShadingType 2 /ColorSpace {} /Coords [{} {} {} {}] /Function {} /Extend [true true] >>",
                            self.pdf.color_space(), f(x0), f(y0), f(x1), f(y1), func
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
                        color_op_in(
                            shape.fill.preview_color().unwrap_or(Color::BLACK),
                            true,
                            self.pdf.cmyk_only
                        )
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
                color_op_in(s.color, false, self.pdf.cmyk_only),
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
            if s.stretch < 0.999 && width > 0.0 {
                // Calligraphic nib: the swept band, filled.
                let band = tracedraw_core::shaping::calligraphic_band(
                    &shape.page_path(),
                    s.width,
                    s.stretch,
                    s.nib_angle,
                );
                let _ = writeln!(
                    self.content,
                    "{}",
                    color_op_in(s.color, true, self.pdf.cmyk_only)
                );
                self.path_ops(&(parent * band));
                self.content.push_str("f\n");
            } else {
                self.path_ops(&path);
                self.content.push_str("S\n");
            }
            // Arrowheads (presets and custom) are filled with the outline
            // colour. They are built on the page-space path, which already
            // carries the shape's own transform, so only the parent applies.
            for head in tracedraw_core::arrowhead_paths(&shape.page_path(), s) {
                let _ = writeln!(
                    self.content,
                    "{}",
                    color_op_in(s.color, true, self.pdf.cmyk_only)
                );
                self.path_ops(&(parent * head));
                self.content.push_str("f\n");
            }
        }
        self.content.push_str("Q\n");
    }
}

/// PDF stitching function over the fountain's stops (type 3 wrapping type 2
/// segments), or a single type 2 function for two stops.
fn stitching_function_in(ft: &tracedraw_core::Fountain, cmyk_only: bool) -> String {
    let rgb_array = |c: Color| color_array(c, cmyk_only);
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
    document_to_pdf_with(doc, &PdfOptions::default())
}

/// Objects that need flattening for X-1a/X-3: non-opaque ones and those
/// with transparency or lens effects.
fn needs_flattening(s: &Shape) -> bool {
    if s.opacity < 1.0 {
        return true;
    }
    if s.effects.iter().any(|e| {
        matches!(
            e,
            tracedraw_core::live::Effect::Transparency { .. }
                | tracedraw_core::live::Effect::Lens(_)
        )
    }) {
        return true;
    }
    match &s.kind {
        ShapeKind::Group { children } => children.iter().any(needs_flattening),
        ShapeKind::ClipFrame { frame, contents } => {
            needs_flattening(frame) || contents.iter().any(needs_flattening)
        }
        ShapeKind::Bitmap { png, .. } => {
            // Any transparent pixel.
            tiny_skia::Pixmap::decode_png(png)
                .map(|pm| pm.pixels().iter().any(|p| p.alpha() != 255))
                .unwrap_or(false)
        }
        _ => false,
    }
}

/// Replace every object that needs flattening by a raster of everything
/// drawn up to and including it, cropped to its bounds, so the composite
/// is kept without transparency in the file.
fn flatten_page(doc: &Document, page: &tracedraw_core::document::Page, dpi: f64) -> Vec<Shape> {
    let layers = doc.layers_for_page(page.id).unwrap_or_default();
    let ordered: Vec<Shape> = layers
        .iter()
        .filter(|l| l.visible && l.printable)
        .flat_map(|l| l.shapes.iter().cloned())
        .collect();
    let mut out: Vec<Shape> = Vec::with_capacity(ordered.len());
    let page_rect = page.rect();
    for (i, s) in ordered.iter().enumerate() {
        if !needs_flattening(s) {
            out.push(s.clone());
            continue;
        }
        let b = s.bounds().intersect(page_rect);
        if b.width() <= 0.0 || b.height() <= 0.0 {
            continue;
        }
        // A document with this page's objects up to here.
        let mut tmp = Document::new("flatten", page.size);
        tmp.symbols = doc.symbols.clone();
        tmp.pages[0].background = page.background.clone();
        tmp.pages[0].layers[0].shapes = ordered[..=i].to_vec();
        let pid = tmp.pages[0].id;
        let Some(pm) = tracedraw_render::render_page_image(&tmp, pid, dpi) else {
            out.push(s.clone());
            continue;
        };
        let zoom = dpi / 25.4;
        let x0 = (b.x0 * zoom).floor().max(0.0) as i32;
        let y0 = ((page.size.height - b.y1) * zoom).floor().max(0.0) as i32;
        let x1 = ((b.x1 * zoom).ceil() as i32).min(pm.width() as i32);
        let y1 = (((page.size.height - b.y0) * zoom).ceil() as i32).min(pm.height() as i32);
        let Some(rect) =
            tiny_skia::IntRect::from_xywh(x0, y0, (x1 - x0).max(1) as u32, (y1 - y0).max(1) as u32)
        else {
            out.push(s.clone());
            continue;
        };
        let Some(crop) = pm.clone_rect(rect) else {
            out.push(s.clone());
            continue;
        };
        let Ok(png) = crop.encode_png() else {
            out.push(s.clone());
            continue;
        };
        let mut raster = Shape::new(
            s.id,
            ShapeKind::Bitmap {
                rect: Rect::new(
                    x0 as f64 / zoom,
                    page.size.height - y1 as f64 / zoom,
                    x1 as f64 / zoom,
                    page.size.height - y0 as f64 / zoom,
                ),
                width_px: crop.width(),
                height_px: crop.height(),
                png,
                fx: None,
            },
        );
        raster.fill = Fill::None;
        raster.stroke = None;
        raster.name = s.name.clone();
        out.push(raster);
    }
    out
}

/// Write every page with the given options (PDF/X, bleed, metadata).
pub fn document_to_pdf_with(doc: &Document, opts: &PdfOptions) -> Vec<u8> {
    let doc = &*crate::resolve_open_fills(doc);
    let std_ = opts.standard;
    let mut pdf = Pdf {
        objects: Vec::new(),
        cmyk_only: std_.cmyk_only(),
        allow_alpha: !std_.flatten(),
    };
    pdf.add_str(String::new()); // 1: catalog, filled at the end
    pdf.add_str(String::new()); // 2: pages
    let bleed = opts.bleed_mm.max(0.0);
    let flatten_dpi = if opts.flatten_dpi > 0.0 {
        opts.flatten_dpi
    } else {
        300.0
    };
    let mut page_ids = Vec::new();
    for page in &doc.pages {
        let flat = if std_.flatten() {
            Some(flatten_page(doc, page, flatten_dpi))
        } else {
            None
        };
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
                    corners: None,
                },
            );
            bg_shape.fill = bg.clone();
            bg_shape.stroke = None;
            w.shape(&bg_shape, Affine::IDENTITY);
        }
        let offset = Affine::translate((bleed, bleed));
        match &flat {
            Some(shapes) => {
                for s in shapes {
                    w.shape(s, offset);
                }
            }
            None => {
                let layers = doc.layers_for_page(page.id).unwrap_or_default();
                for layer in layers.iter().filter(|l| l.visible && l.printable) {
                    for s in &layer.shapes {
                        w.shape(s, offset);
                    }
                }
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
        let (mw, mh) = (
            (page.size.width + 2.0 * bleed) * MM_PT,
            (page.size.height + 2.0 * bleed) * MM_PT,
        );
        let boxes = if std_ != PdfStandard::None || bleed > 0.0 {
            format!(
                " /TrimBox [{} {} {} {}] /BleedBox [0 0 {} {}]",
                f(bleed * MM_PT),
                f(bleed * MM_PT),
                f((bleed + page.size.width) * MM_PT),
                f((bleed + page.size.height) * MM_PT),
                f(mw),
                f(mh)
            )
        } else {
            String::new()
        };
        let page_id = pdf.add_str(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}]{boxes} /Contents {} 0 R /Resources {} >>",
            f(mw),
            f(mh),
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
    // Info dictionary and metadata.
    let title = opts
        .title
        .clone()
        .unwrap_or_else(|| doc.title.clone())
        .trim()
        .to_string();
    let title = if title.is_empty() {
        "Untitled".to_string()
    } else {
        title
    };
    let date = pdf_date();
    let mut info = format!(
        "<< /Title {} /Creator (TraceDraw) /Producer (TraceDraw) /CreationDate {} /ModDate {}",
        pdf_text(&title),
        pdf_text(&date),
        pdf_text(&date)
    );
    if std_ != PdfStandard::None {
        let _ = write!(
            info,
            " /Trapped /False /GTS_PDFXVersion {}",
            pdf_text(std_.name())
        );
        if std_ == PdfStandard::X1a {
            let _ = write!(info, " /GTS_PDFXConformance {}", pdf_text(std_.name()));
        }
    }
    info.push_str(" >>");
    let info_id = pdf.add_str(info);
    let mut catalog = String::from("<< /Type /Catalog /Pages 2 0 R");
    if std_ != PdfStandard::None {
        let condition = if opts.output_condition.trim().is_empty() {
            "CGATS TR 001".to_string()
        } else {
            opts.output_condition.trim().to_string()
        };
        let profile_ref = opts.output_profile.as_ref().map(|icc| {
            let n = icc_channels(icc);
            let id = pdf.stream(&format!("/N {n}"), icc);
            format!(" /DestOutputProfile {id} 0 R")
        });
        let intent = pdf.add_str(format!(
            "<< /Type /OutputIntent /S /GTS_PDFX /OutputConditionIdentifier {} /OutputCondition {} /RegistryName (http://www.color.org) /Info {}{} >>",
            pdf_text(&condition),
            pdf_text(&condition),
            pdf_text(&condition),
            profile_ref.unwrap_or_default()
        ));
        let _ = write!(catalog, " /OutputIntents [{intent} 0 R]");
        // XMP metadata (required by X-4, harmless elsewhere).
        let xmp = xmp_packet(&title, std_, &date);
        let meta = pdf.stream("/Type /Metadata /Subtype /XML", xmp.as_bytes());
        let _ = write!(catalog, " /Metadata {meta} 0 R");
    }
    catalog.push_str(" >>");
    pdf.objects[0] = catalog.into_bytes();
    pdf.finish_with(std_.version(), Some(info_id))
}

/// Channel count of an ICC profile from its colour space signature.
fn icc_channels(icc: &[u8]) -> u32 {
    match icc.get(16..20) {
        Some(b"GRAY") => 1,
        Some(b"CMYK") => 4,
        _ => 3,
    }
}

fn xmp_packet(title: &str, std_: PdfStandard, date: &str) -> String {
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    // D:YYYYMMDDHHmmSSZ to ISO 8601.
    let iso = if date.len() >= 16 {
        format!(
            "{}-{}-{}T{}:{}:{}Z",
            &date[2..6],
            &date[6..8],
            &date[8..10],
            &date[10..12],
            &date[12..14],
            &date[14..16]
        )
    } else {
        String::new()
    };
    let (version, conformance) = match std_ {
        PdfStandard::X1a => ("PDF/X-1a:2003", "PDF/X-1a:2003"),
        PdfStandard::X3 => ("PDF/X-3:2003", ""),
        PdfStandard::X4 => ("PDF/X-4", ""),
        PdfStandard::None => ("", ""),
    };
    let mut x = String::new();
    x.push_str("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
    x.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n");
    let _ = write!(
        x,
        "<rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\" xmlns:pdfxid=\"http://www.npes.org/pdfx/ns/id/\" xmlns:pdfx=\"http://ns.adobe.com/pdfx/1.3/\">\n<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:title>\n<xmp:CreatorTool>TraceDraw</xmp:CreatorTool>\n<xmp:CreateDate>{iso}</xmp:CreateDate>\n<xmp:ModifyDate>{iso}</xmp:ModifyDate>\n<pdf:Producer>TraceDraw</pdf:Producer>\n<pdf:Trapped>False</pdf:Trapped>\n",
        esc(title)
    );
    if !version.is_empty() {
        let _ = write!(x, "<pdfxid:GTS_PDFXVersion>{version}</pdfxid:GTS_PDFXVersion>\n<pdfx:GTS_PDFXVersion>{version}</pdfx:GTS_PDFXVersion>\n");
    }
    if !conformance.is_empty() {
        let _ = writeln!(
            x,
            "<pdfx:GTS_PDFXConformance>{conformance}</pdfx:GTS_PDFXConformance>"
        );
    }
    x.push_str("</rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>");
    x
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
                corners: None,
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
                corners: None,
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

#[cfg(test)]
mod pdfx_tests {
    use super::*;
    use tracedraw_core::geometry::Rect;

    fn doc_with_transparent_rect() -> Document {
        let mut doc = Document::new("Flyer", tracedraw_core::Size::new(100.0, 50.0));
        let mut ids = doc.ids().clone();
        let mut base = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 100.0, 50.0),
                radius: 0.0,
                corners: None,
            },
        );
        base.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        base.stroke = None;
        let mut top = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(20.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            },
        );
        top.fill = Fill::Solid(Color::rgb8(0, 0, 255));
        top.stroke = None;
        top.opacity = 0.5;
        doc.pages[0].layers[0].shapes = vec![base, top];
        doc.set_ids(ids);
        doc
    }

    #[test]
    fn x1a_has_cmyk_only_output_intent_and_flattened_transparency() {
        let doc = doc_with_transparent_rect();
        let opts = PdfOptions {
            standard: PdfStandard::X1a,
            output_condition: "FOGRA39".into(),
            bleed_mm: 3.0,
            flatten_dpi: 72.0,
            ..Default::default()
        };
        let bytes = document_to_pdf_with(&doc, &opts);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("/GTS_PDFXVersion (PDF/X-1a:2003)"));
        assert!(text.contains("/GTS_PDFXConformance (PDF/X-1a:2003)"));
        assert!(text.contains("/OutputIntents ["));
        assert!(text.contains("/OutputConditionIdentifier (FOGRA39)"));
        assert!(text.contains("/Trapped /False"));
        assert!(text.contains("/ID [<"));
        assert!(text.contains("/Metadata "));
        // No RGB anywhere, no ExtGState alpha.
        assert!(!text.contains("/DeviceRGB"));
        assert!(!text.contains("/ca "));
        assert!(!text.contains("/SMask"));
        // Bleed: MediaBox is the page plus 3 mm per side; TrimBox the page.
        assert!(text.contains(&format!(
            "/MediaBox [0 0 {} {}]",
            f(106.0 * MM_PT),
            f(56.0 * MM_PT)
        )));
        assert!(text.contains(&format!(
            "/TrimBox [{} {} {} {}]",
            f(3.0 * MM_PT),
            f(3.0 * MM_PT),
            f(103.0 * MM_PT),
            f(53.0 * MM_PT)
        )));
        // The half-transparent rectangle became a CMYK image.
        assert!(text.contains("/Subtype /Image") && text.contains("/ColorSpace /DeviceCMYK"));
    }

    #[test]
    fn x4_keeps_transparency_and_rgb() {
        let doc = doc_with_transparent_rect();
        let opts = PdfOptions {
            standard: PdfStandard::X4,
            ..Default::default()
        };
        let bytes = document_to_pdf_with(&doc, &opts);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.6"));
        assert!(text.contains("/GTS_PDFXVersion (PDF/X-4)"));
        assert!(text.contains("/ca 0.5"));
        assert!(!text.contains("/Subtype /Image"));
        // The embedded profile's channel count follows its signature.
        let mut icc = vec![0u8; 128];
        icc[16..20].copy_from_slice(b"CMYK");
        assert_eq!(icc_channels(&icc), 4);
        let opts = PdfOptions {
            standard: PdfStandard::X3,
            output_profile: Some(icc),
            ..Default::default()
        };
        let text = String::from_utf8_lossy(&document_to_pdf_with(&doc, &opts)).into_owned();
        assert!(text.contains("/DestOutputProfile"));
        assert!(text.contains("/N 4"));
    }

    #[test]
    fn plain_pdf_is_unchanged_apart_from_info() {
        let doc = doc_with_transparent_rect();
        let text = String::from_utf8_lossy(&document_to_pdf(&doc)).into_owned();
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("/Title (Flyer)"));
        assert!(!text.contains("/OutputIntents"));
        assert!(text.contains("/ca 0.5"));
        assert!(pdf_date().starts_with("D:20"));
    }
}
