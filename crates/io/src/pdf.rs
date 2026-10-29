//! Minimal PDF writer: one page per document page, vector paths with
//! uniform fills (RGB or CMYK), axial/radial shadings for fountain fills,
//! outlines with width, caps, joins and dashes, bitmaps as images,
//! uniform transparency via ExtGState, and text as text with embedded
//! subset fonts (see `pdf_fonts.rs`). Written by hand; fonts are subset
//! with the `subsetter` crate.

use std::fmt::Write as _;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, PathEl, Rect},
    Color, Document, Fill, FountainKind, LineCap, LineJoin,
};

const MM_PT: f64 = 72.0 / 25.4;

#[path = "pdf_fonts.rs"]
mod fonts;
pub use fonts::{base14_name, font_licence, licence_of, win_ansi, FontLicence, PdfText};

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

/// How bitmaps and page content go into the file (#550): pictures
/// above a resolution are downsampled to it, colour and gray ones can be
/// JPEG compressed, and the pages' text and line art compressed or not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdfImages {
    /// Downsample colour, gray and black-and-white pictures to these
    /// resolutions (dots per inch, as placed) when they have more;
    /// `None` keeps every pixel.
    pub color_dpi: Option<f64>,
    pub gray_dpi: Option<f64>,
    pub mono_dpi: Option<f64>,
    /// JPEG for colour and gray pictures (lossy), else ZIP (lossless).
    /// Black-and-white pictures, transparency masks and PDF/X-1a files
    /// stay ZIP.
    pub jpeg: bool,
    /// JPEG quality, 1 to 100.
    pub jpeg_quality: u8,
    /// Compress the pages' text and line art.
    pub compress_content: bool,
}

impl Default for PdfImages {
    fn default() -> Self {
        PdfImages {
            color_dpi: None,
            gray_dpi: None,
            mono_dpi: None,
            jpeg: false,
            jpeg_quality: 85,
            compress_content: true,
        }
    }
}

/// What a picture holds, for its resolution and how it is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageKind {
    Color,
    Gray,
    Mono,
}

fn image_kind(rgb: &[u8]) -> ImageKind {
    let mut mono = true;
    for px in rgb.chunks_exact(3) {
        if px[0] != px[1] || px[1] != px[2] {
            return ImageKind::Color;
        }
        if px[0] != 0 && px[0] != 255 {
            mono = false;
        }
    }
    if mono {
        ImageKind::Mono
    } else {
        ImageKind::Gray
    }
}

/// True when `b` and `area` share no point. Bounds that are not finite
/// count as reaching it.
fn misses(b: Rect, area: Rect) -> bool {
    [b.x0, b.y0, b.x1, b.y1].iter().all(|v| v.is_finite())
        && (b.x1 < area.x0 || b.x0 > area.x1 || b.y1 < area.y0 || b.y0 > area.y1)
}

/// `data` (`channels` bytes a pixel, `w` x `h`) resized to `nw` x `nh`.
fn resample(data: &[u8], channels: u8, w: u32, h: u32, nw: u32, nh: u32) -> Option<Vec<u8>> {
    use image::imageops::{resize, FilterType};
    let out = match channels {
        1 => resize(
            &image::GrayImage::from_raw(w, h, data.to_vec())?,
            nw,
            nh,
            FilterType::Triangle,
        )
        .into_raw(),
        3 => resize(
            &image::RgbImage::from_raw(w, h, data.to_vec())?,
            nw,
            nh,
            FilterType::Triangle,
        )
        .into_raw(),
        _ => return None,
    };
    Some(out)
}

/// `data` as a JPEG file (gray with one channel, else RGB).
fn jpeg_bytes(data: &[u8], channels: u8, w: u32, h: u32, quality: u8) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut enc =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100));
    let color = if channels == 1 {
        image::ExtendedColorType::L8
    } else {
        image::ExtendedColorType::Rgb8
    };
    enc.encode(data, w, h, color).ok()?;
    Some(out)
}

/// Options for `document_to_pdf_with`.
#[derive(Debug, Clone, Default)]
pub struct PdfOptions {
    pub standard: PdfStandard,
    /// ICC profile embedded as the output intent's destination profile
    /// (a CMYK press profile for X-1a). Without one, the CMYK profile the
    /// drawing carries is embedded; without either, a registered
    /// characterisation name is used instead.
    pub output_profile: Option<Vec<u8>>,
    /// Output condition identifier, e.g. "FOGRA39" or "CGATS TR 001".
    /// Empty: the embedded profile's description, else "CGATS TR 001".
    pub output_condition: String,
    /// Bleed added around every page (TrimBox = page, BleedBox = page +
    /// bleed, MediaBox = page + bleed + `marks_margin_mm`).
    pub bleed_mm: f64,
    /// Room beyond the bleed for printers' marks, in mm on every side (0
    /// for none). The page and its bleed are clipped to the BleedBox so
    /// nothing of the drawing reaches the marks.
    pub marks_margin_mm: f64,
    /// Printers' marks of each page (index as in the document), drawn on
    /// top in page coordinates: they sit outside the page, in the bleed
    /// and the marks margin.
    pub page_marks: Vec<Vec<Shape>>,
    pub title: Option<String>,
    /// Flattening resolution for X-1a/X-3 transparency.
    pub flatten_dpi: f64,
    /// Document information: author, subject and keywords.
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    /// A bookmark per page (its name) that goes to the top of the page.
    pub bookmarks: bool,
    /// Open the bookmarks panel when the file opens.
    pub show_bookmarks: bool,
    /// Password protection (AES-256). PDF/X files are never protected:
    /// the standards forbid it.
    pub security: Option<crate::pdf_crypt::Security>,
    /// Text as text with fonts, or as curves.
    pub text: PdfText,
    /// Bitmap resolution and compression (#550).
    pub images: PdfImages,
}

/// What went into a file besides the drawing: its fonts.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PdfReport {
    /// Fonts put in the file (their PostScript names).
    pub embedded: Vec<String>,
    /// Fonts only named, for the viewer to supply.
    pub named: Vec<String>,
    /// Font families whose licence forbids embedding: their text went in
    /// as curves.
    pub restricted: Vec<String>,
}

/// The key of the registration colour's colour space among the spot inks'.
const REGISTRATION_SPACE: u32 = u32::MAX;

struct Pdf {
    objects: Vec<Vec<u8>>,
    /// Separation colour space objects, by spot ink number.
    spot_spaces: std::collections::HashMap<u32, usize>,
    cmyk_only: bool,
    allow_alpha: bool,
    /// The /Encrypt dictionary's object, once the file is protected.
    encrypt_id: Option<usize>,
    /// The fonts text uses, written at the end.
    fonts: Vec<fonts::FontSlot>,
    /// Fonts that cannot go in a given way: (data address, index, kind).
    bad_fonts: Vec<(usize, u32, fonts::SlotKind)>,
    report: PdfReport,
    images: PdfImages,
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
        if self.images.jpeg {
            if let Some(data) = jpeg_bytes(rgb, 3, w, h, self.images.jpeg_quality) {
                return self.stream(
                    &format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode"),
                    &data,
                );
            }
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
    /// Protect the file: every object's strings and streams encrypted, the
    /// /Encrypt dictionary added (and named in the trailer).
    fn encrypt(&mut self, security: &crate::pdf_crypt::Security, random: crate::pdf_crypt::Random) {
        let mut enc = crate::pdf_crypt::Encryptor::new(security, random);
        for body in self.objects.iter_mut() {
            *body = enc.encrypt_object(body);
        }
        let dict = std::mem::take(&mut enc.dictionary);
        self.encrypt_id = Some(self.add_str(dict));
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
        let mut info = info_id
            .map(|i| format!(" /Info {i} 0 R"))
            .unwrap_or_default();
        if let Some(e) = self.encrypt_id {
            info.push_str(&format!(" /Encrypt {e} 0 R"));
        }
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

/// A text string: a literal string for plain ASCII, otherwise UTF-16BE
/// with a byte order mark (so any script survives).
fn pdf_text_any(s: &str) -> String {
    if s.chars().all(|c| (c as u32) < 128) {
        return pdf_text(s);
    }
    let mut out = String::from("<FEFF");
    for unit in s
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .encode_utf16()
    {
        let _ = write!(out, "{unit:04X}");
    }
    out.push('>');
    out
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
    /// The drawing's spot inks, and the colour spaces this page uses.
    inks: &'a [tracedraw_core::document::Ink],
    spaces: Vec<(String, usize)>,
    /// The fonts this page uses (resource name, object).
    fonts: Vec<(String, usize)>,
    /// How text is written; `None` writes it as curves.
    text: Option<TextMode>,
    /// The page's media box (the page and its bleed) in millimetres, in
    /// the space the shapes are written in.
    media: Rect,
}

/// How a writer keeps text as text.
#[derive(Debug, Clone, Copy)]
struct TextMode {
    /// Fonts go in the file.
    embed: bool,
    /// The standard fonts go in too.
    embed_base14: bool,
}

/// A PDF name: `#xx` for bytes outside `!` to `~` and for delimiters.
fn pdf_name(s: &str) -> String {
    let mut out = String::from("/");
    for b in s.bytes() {
        if (b'!'..=b'~').contains(&b) && !b"#()<>[]{}/%".contains(&b) {
            out.push(b as char);
        } else {
            let _ = write!(out, "#{b:02X}");
        }
    }
    if out.len() == 1 {
        out.push_str("Spot");
    }
    out
}

impl PageWriter<'_> {
    /// The operator setting colour `c`: a spot ink of the drawing on a
    /// Separation colour space of its own (its name, its process stand-in
    /// as the alternate), the others as before.
    fn color_op(&mut self, c: Color, fill: bool) -> String {
        if c == Color::Registration {
            // Registration colour prints on every plate: the All separation.
            let id = match self.pdf.spot_spaces.get(&REGISTRATION_SPACE) {
                Some(id) => *id,
                None => {
                    let id = self.pdf.add_str(
                        "[/Separation /All /DeviceCMYK << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [1 1 1 1] /N 1 >>]".into(),
                    );
                    self.pdf.spot_spaces.insert(REGISTRATION_SPACE, id);
                    id
                }
            };
            let name = "CSAll".to_string();
            if !self.spaces.iter().any(|(n, _)| *n == name) {
                self.spaces.push((name.clone(), id));
            }
            return if fill {
                format!("/{name} cs 1 scn")
            } else {
                format!("/{name} CS 1 SCN")
            };
        }
        if let Color::Spot { ink, tint, .. } = c {
            if let Some(i) = self.inks.iter().find(|i| i.id == ink) {
                let id = match self.pdf.spot_spaces.get(&ink) {
                    Some(id) => *id,
                    None => {
                        let [cc, m, y, k] = i.cmyk.map(|v| f(v.clamp(0.0, 1.0) as f64));
                        let id = self.pdf.add_str(format!(
                            "[/Separation {} /DeviceCMYK << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [{cc} {m} {y} {k}] /N 1 >>]",
                            pdf_name(&i.name)
                        ));
                        self.pdf.spot_spaces.insert(ink, id);
                        id
                    }
                };
                let name = format!("CS{ink}");
                if !self.spaces.iter().any(|(n, _)| *n == name) {
                    self.spaces.push((name.clone(), id));
                }
                let t = if tint.is_finite() {
                    tint.clamp(0.0, 1.0)
                } else {
                    1.0
                };
                return if fill {
                    format!("/{name} cs {} scn", f(t as f64))
                } else {
                    format!("/{name} CS {} SCN", f(t as f64))
                };
            }
        }
        color_op_in(c.process(), fill, self.pdf.cmyk_only)
    }

    fn path_ops(&mut self, path: &tracedraw_core::BezPath) {
        // The current point and the start of the subpath.
        let mut cur: Option<tracedraw_core::geometry::Point> = None;
        let mut start: Option<tracedraw_core::geometry::Point> = None;
        for el in path.elements() {
            match el {
                PathEl::MoveTo(p) => {
                    cur = Some(*p);
                    start = Some(*p);
                    let _ = writeln!(self.content, "{} {} m", f(p.x * MM_PT), f(p.y * MM_PT));
                }
                PathEl::LineTo(p) => {
                    cur = Some(*p);
                    let _ = writeln!(self.content, "{} {} l", f(p.x * MM_PT), f(p.y * MM_PT));
                }
                PathEl::QuadTo(c, p) => {
                    // Raised to the cubic that is the same curve: the
                    // handles two thirds of the way to the control point.
                    let s = cur.unwrap_or(*c);
                    let c1 = s + (*c - s) * (2.0 / 3.0);
                    let c2 = *p + (*c - *p) * (2.0 / 3.0);
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
                    cur = Some(*p);
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
                    cur = Some(*p);
                }
                PathEl::ClosePath => {
                    self.content.push_str("h\n");
                    cur = start;
                }
            }
        }
    }

    /// Paint `path` (page space, mm) with `fill` spread over `bounds`:
    /// a colour, a shading for a gradient, or the renderer's pixels
    /// clipped by the path for the rest. `nonzero` fills and clips with
    /// the non-zero rule (text, outline areas), otherwise even-odd.
    fn paint_area(
        &mut self,
        fill: &Fill,
        path: &tracedraw_core::BezPath,
        bounds: tracedraw_core::geometry::Rect,
        nonzero: bool,
    ) {
        let (fill_op, clip) = if nonzero { ("f", "W") } else { ("f*", "W*") };
        match fill {
            Fill::None => {}
            Fill::Solid(c) => {
                let op = self.color_op(*c, true);
                let _ = writeln!(self.content, "{op}");
                self.path_ops(path);
                let _ = writeln!(self.content, "{fill_op}");
            }
            Fill::Fountain(ft) if ft.kind != FountainKind::Shaped => {
                let func = stitching_function_in(ft, self.pdf.cmyk_only);
                // Nodes placed freely (Interactive Fill): a unit shading
                // drawn through the fill's own frame.
                if let Some(frame) = ft.frame(bounds) {
                    let m = frame.as_coeffs();
                    let shading = if ft.kind == FountainKind::Radial {
                        format!(
                            "<< /ShadingType 3 /ColorSpace {} /Coords [0 0 0 0 0 1] /Function {} /Extend [true true] >>",
                            self.pdf.color_space(), func
                        )
                    } else {
                        format!(
                            "<< /ShadingType 2 /ColorSpace {} /Coords [0 0 1 0] /Function {} /Extend [true true] >>",
                            self.pdf.color_space(), func
                        )
                    };
                    let name = format!("Sh{}", self.shadings.len());
                    let id = self.pdf.add_str(shading);
                    self.shadings.push((name.clone(), id));
                    self.content.push_str("q\n");
                    self.path_ops(path);
                    let _ = writeln!(
                        self.content,
                        "{clip} n {} {} {} {} {} {} cm /{name} sh\nQ",
                        f(m[0] * MM_PT),
                        f(m[1] * MM_PT),
                        f(m[2] * MM_PT),
                        f(m[3] * MM_PT),
                        f(m[4] * MM_PT),
                        f(m[5] * MM_PT)
                    );
                    return;
                }
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
                self.path_ops(path);
                let _ = writeln!(self.content, "{clip} n /{name} sh\nQ");
            }
            // Patterns, textures, meshes and shaped gradients are
            // rasterised into an image clipped by the path; the renderer
            // produces the same pixels as the screen.
            Fill::Fountain(_) | Fill::Pattern(_) | Fill::Texture(_) | Fill::Mesh(_) => {
                // A shaped gradient covers the bounds of the path it follows.
                let bounds = if matches!(fill, Fill::Fountain(_)) {
                    use tracedraw_core::geometry::Shape as _;
                    path.bounding_box()
                } else {
                    bounds
                };
                if let Some((png, w, h)) = rasterise_fill(fill, path, bounds) {
                    if let Some(rgb) = decode_png_rgb(&png) {
                        let name = format!("Im{}", self.images.len());
                        let id = self.pdf.add_image(w, h, &rgb);
                        self.images.push((name.clone(), id));
                        self.content.push_str("q\n");
                        self.path_ops(path);
                        let _ = writeln!(
                            self.content,
                            "{clip} n {} 0 0 {} {} {} cm /{name} Do\nQ",
                            f(bounds.width() * MM_PT),
                            f(bounds.height() * MM_PT),
                            f(bounds.x0 * MM_PT),
                            f(bounds.y0 * MM_PT)
                        );
                    }
                } else {
                    let op = self.color_op(fill.preview_color().unwrap_or(Color::BLACK), true);
                    let _ = writeln!(self.content, "{op}");
                    self.path_ops(path);
                    let _ = writeln!(self.content, "{fill_op}");
                }
            }
        }
    }

    fn shape(&mut self, shape: &Shape, parent: Affine) {
        // Bitmap effects on a vector object: its picture in its place.
        if let Some(pic) = shape.drawn_as() {
            return self.shape(&pic, parent);
        }
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
        // Characters with their own fill or outline: one text a look.
        if let Some(parts) = shape.text_parts() {
            for p in &parts {
                self.shape(p, parent);
            }
            return;
        }
        // An outline set behind the fill: the outline first, then the
        // fill over it (half the outline's width shows outside), as the
        // drawing window draws it.
        if shape.stroke.as_ref().is_some_and(|st| st.behind_fill)
            && !matches!(shape.fill, Fill::None)
        {
            let mut outline = shape.clone();
            outline.fill = Fill::None;
            if let Some(st) = outline.stroke.as_mut() {
                st.behind_fill = false;
            }
            let mut fill = shape.clone();
            fill.stroke = None;
            // The shadow goes once, beneath the outline drawn first.
            fill.shadow = None;
            self.shape(&outline, parent);
            self.shape(&fill, parent);
            return;
        }
        let transform = parent * shape.transform;
        // A group's or a clip frame's own shadow, beneath all it holds (a
        // clip frame with an opacity mask goes in as a picture, shadow
        // and all).
        if matches!(shape.kind, ShapeKind::Group { .. })
            || matches!(shape.kind, ShapeKind::ClipFrame { .. }) && shape.opacity_mask.is_none()
        {
            self.drop_shadow(shape, parent);
        }
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
        if shape.opacity_mask.is_some() && matches!(shape.kind, ShapeKind::ClipFrame { .. }) {
            // An opacity mask has no plain PDF form here: it goes in as a
            // picture with its transparency.
            if let Some(raster) = raster_of(shape, parent, self.symbols, MASK_DPI) {
                self.shape(&raster, Affine::IDENTITY);
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
        // Further fills and outlines: the parts, bottom first.
        if let Some(parts) = shape.layers() {
            for p in &parts {
                self.shape(p, parent);
            }
            return;
        }
        let path = transform * shape.local_path();
        if path.elements().is_empty() {
            return;
        }
        use tracedraw_core::geometry::Shape as _;
        let bounds = path.bounding_box();
        // A bitmap paints inside its placed rectangle only: wholly outside
        // the media box it cannot show, so its pixels stay out of the file.
        if let ShapeKind::Bitmap { rect, .. } = &shape.kind {
            if misses(transform.transform_rect_bbox(*rect), self.media) {
                return;
            }
        }
        // Drop shadow: the blurred silhouette behind the object.
        self.drop_shadow(shape, parent);
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
                let opts = self.pdf.images;
                let kind = image_kind(&rgb);
                // Gray and black-and-white pictures go in with one channel.
                let (mut data, mut channels): (Vec<u8>, u8) = if kind == ImageKind::Color {
                    (rgb, 3)
                } else {
                    (rgb.chunks_exact(3).map(|p| p[0]).collect(), 1)
                };
                let (mut w, mut h) = (*width_px, *height_px);
                let mut alpha = alpha;
                // Downsampled to the resolution asked for its kind, as
                // placed on the page.
                let wide_mm = {
                    let c = transform.as_coeffs();
                    (c[0] * rect.width()).hypot(c[1] * rect.width())
                };
                let target = match kind {
                    ImageKind::Color => opts.color_dpi,
                    ImageKind::Gray => opts.gray_dpi,
                    ImageKind::Mono => opts.mono_dpi,
                };
                if let Some(t) = target.filter(|t| *t >= 1.0) {
                    let dpi = if wide_mm > 1e-9 {
                        w as f64 / (wide_mm / 25.4)
                    } else {
                        0.0
                    };
                    if dpi > t * 1.05 {
                        let k = t / dpi;
                        let nw = ((w as f64 * k).round() as u32).max(1);
                        let nh = ((h as f64 * k).round() as u32).max(1);
                        if let Some(d) = resample(&data, channels, w, h, nw, nh) {
                            data = d;
                            if kind == ImageKind::Mono {
                                // Back to black and white after the filter.
                                for v in data.iter_mut() {
                                    *v = if *v >= 128 { 255 } else { 0 };
                                }
                            }
                            alpha = alpha.and_then(|a| resample(&a, 1, w, h, nw, nh));
                            (w, h) = (nw, nh);
                        }
                    }
                }
                // Transparent pixels go to a soft mask.
                let smask = alpha.map(|a| {
                    let adata = flate(&a);
                    self.pdf.stream(
                        &format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode"),
                        &adata,
                    )
                });
                let smask_ref = smask
                    .map(|id| format!(" /SMask {id} 0 R"))
                    .unwrap_or_default();
                let cs = if channels == 1 {
                    "/DeviceGray"
                } else {
                    self.pdf.color_space()
                };
                if channels == 3 && self.pdf.cmyk_only {
                    data = rgb_to_cmyk_bytes(&data);
                    channels = 4;
                }
                let jpeg = (opts.jpeg && kind != ImageKind::Mono && channels != 4)
                    .then(|| jpeg_bytes(&data, channels, w, h, opts.jpeg_quality))
                    .flatten();
                let (data, filter) = match jpeg {
                    Some(j) => (j, "/DCTDecode"),
                    None => (flate(&data), "/FlateDecode"),
                };
                let id = self.pdf.stream(
                    &format!("/Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace {cs} /BitsPerComponent 8 /Filter {filter}{smask_ref}"),
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

        // Text as text with its fonts, when it can be.
        if self.text_shape(shape, parent, transform, &path, bounds) {
            self.content.push_str("Q\n");
            return;
        }

        // Fill.
        let text = matches!(shape.kind, ShapeKind::Text { .. });
        self.paint_area(&shape.fill, &path, bounds, text);

        // Outline.
        if let Some(s) = &shape.stroke {
            self.outline(shape, s, parent, transform, &path, bounds);
        }
        self.content.push_str("Q\n");
    }

    /// Set the outline's colour, width, caps, joins and dashes.
    /// The drop shadow of `shape` (placed by `parent`) as the drawing
    /// window draws it: the blurred silhouette in the shadow's colour and
    /// opacity, as a picture with a soft mask at [`SHADOW_DPI`] (PDF has
    /// no blur of its own), merged by the shadow's merge mode. The object
    /// itself is drawn over it by the caller. Without transparency in the
    /// file (PDF/X-1a, X-3) the page flattening has drawn it already.
    fn drop_shadow(&mut self, shape: &Shape, parent: Affine) {
        let Some(sh) = shape.shadow.filter(|sh| sh.opacity > 0.0) else {
            return;
        };
        if !self.pdf.allow_alpha || shape.opacity <= 0.0 {
            return;
        }
        let mut placed = shape.clone();
        placed.transform = parent * shape.transform;
        let area = sh.bounds(placed.bounds());
        if misses(area, self.media) {
            return;
        }
        // Within 40 million pixels, as the other pictures drawn here.
        let area_in2 = (area.width() / 25.4) * (area.height() / 25.4);
        let dpi = SHADOW_DPI
            .min((40.0e6 / area_in2.max(1e-9)).sqrt())
            .max(1.0);
        let Some((rect, pm)) = tracedraw_render::render_shadow_image(&placed, dpi) else {
            return;
        };
        let Ok(png) = pm.encode_png() else {
            return;
        };
        let mut pic = Shape::new(
            shape.id,
            ShapeKind::Bitmap {
                rect,
                width_px: pm.width(),
                height_px: pm.height(),
                png: png.into(),
                fx: None,
            },
        );
        pic.fill = Fill::None;
        pic.stroke = None;
        let blend = pdf_blend_mode(sh.merge);
        if blend != "Normal" {
            let name = format!("GS{}", self.gstates.len());
            let id = self
                .pdf
                .add_str(format!("<< /Type /ExtGState /BM /{blend} >>"));
            self.gstates.push((name.clone(), id));
            let _ = writeln!(self.content, "q\n/{name} gs");
            self.shape(&pic, Affine::IDENTITY);
            self.content.push_str("Q\n");
        } else {
            self.shape(&pic, Affine::IDENTITY);
        }
    }

    fn stroke_state(&mut self, s: &tracedraw_core::Stroke) {
        let width = if s.width <= tracedraw_core::Stroke::HAIRLINE + 1e-9 {
            0.0
        } else {
            s.width * MM_PT
        };
        let op = self.color_op(s.color, false);
        let _ = writeln!(
            self.content,
            "{} {} w {} J {} j",
            op,
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
        let _ = writeln!(self.content, "{} M", f(s.miter_ratio()));
    }

    /// Draw the outline `s` of `shape` along `path` (page space).
    fn outline(
        &mut self,
        shape: &Shape,
        s: &tracedraw_core::Stroke,
        parent: Affine,
        transform: Affine,
        path: &tracedraw_core::BezPath,
        bounds: Rect,
    ) {
        use tracedraw_core::geometry::Shape as _;
        let (drawn, scale) = crate::svg::drawn_stroke(s, transform);
        self.stroke_state(&drawn);
        if let Some(brush) = &s.painterly {
            // Painterly brush: the marks, one filled path per opacity.
            for (alpha, marks) in tracedraw_core::painterly::marks_by_opacity(
                &shape.page_path(),
                drawn.width,
                brush,
                12,
            ) {
                self.content.push_str("q\n");
                if self.pdf.allow_alpha {
                    let name = format!("GS{}", self.gstates.len());
                    let id = self.pdf.add_str(format!(
                        "<< /Type /ExtGState /CA {} /ca {} >>",
                        f(alpha),
                        f(alpha)
                    ));
                    self.gstates.push((name.clone(), id));
                    let _ = writeln!(self.content, "/{name} gs");
                }
                let op = self.color_op(s.color, true);
                let _ = writeln!(self.content, "{op}");
                self.path_ops(&(parent * marks));
                self.content.push_str("f\nQ\n");
            }
        } else if let Some(band) =
            tracedraw_core::shaping::outline_band(&shape.page_path(), s, scale)
        {
            // Calligraphic nib or variable width: the band, filled.
            let band = parent * band;
            if let Some(paint) = outline_paint(s, transform) {
                let b = band.bounding_box();
                self.paint_area(&paint, &band, b, true);
            } else {
                let op = self.color_op(s.color, true);
                let _ = writeln!(self.content, "{op}");
                self.path_ops(&band);
                self.content.push_str("f\n");
            }
        } else if let Some((paint, area)) = outline_paint(s, transform)
            .map(|p| (p, tracedraw_core::shaping::stroke_area(path, &drawn)))
            .filter(|(_, a)| !a.elements().is_empty())
        {
            // The outline's own paint: the area it covers, painted
            // like a fill spread over the outline's bounds.
            let half = drawn.width / 2.0;
            self.paint_area(&paint, &area, bounds.inflate(half, half), true);
        } else {
            self.path_ops(path);
            self.content.push_str("S\n");
        }
        // Arrowheads (presets and custom) are filled with the outline
        // colour. They are built on the page-space path, which already
        // carries the shape's own transform, so only the parent applies.
        for head in tracedraw_core::arrowhead_paths(&shape.page_path(), s) {
            let op = self.color_op(s.color, true);
            let _ = writeln!(self.content, "{op}");
            self.path_ops(&(parent * head));
            self.content.push_str("f\n");
        }
    }

    /// Text as PDF text: each glyph a font can draw is shown in its font
    /// (embedded, or named), the rest (underlines, added hyphens, glyphs
    /// of fonts that may not be embedded, synthetic bold) as curves. A
    /// solid fill and a plain outline are the text's own fill and stroke;
    /// other fills and outlines are painted on the outlines, and the text
    /// goes over them invisibly so it can still be searched and copied.
    /// False when the shape is no text or text goes as curves; `full` is
    /// the whole outline (page space) and `bounds` its bounds.
    fn text_shape(
        &mut self,
        shape: &Shape,
        parent: Affine,
        transform: Affine,
        full: &tracedraw_core::BezPath,
        bounds: Rect,
    ) -> bool {
        let Some(mode) = self.text else {
            return false;
        };
        let ShapeKind::Text { spans, .. } = &shape.kind else {
            return false;
        };
        let Some(layout) = tracedraw_core::document::text_glyph_layout(&shape.kind) else {
            return false;
        };
        if layout.glyphs.is_empty() {
            return false;
        }
        let solid_fill = matches!(shape.fill, Fill::Solid(_));
        let plain_stroke = shape.stroke.as_ref().filter(|s| {
            s.painterly.is_none()
                && s.widths.is_empty()
                && !(s.stretch < 0.999 && s.width > tracedraw_core::Stroke::HAIRLINE + 1e-9)
                && outline_paint(s, transform).is_none()
        });
        if !solid_fill && matches!(shape.fill, Fill::None) && shape.stroke.is_none() {
            // Nothing shows.
            return true;
        }
        // Each glyph: shown visibly, shown invisibly over its curves, or
        // curves only.
        let mut visible: Vec<(Affine, fonts::Shown)> = Vec::new();
        let mut hidden: Vec<(Affine, fonts::Shown)> = Vec::new();
        let mut curves = transform * layout.rest.clone();
        for g in &layout.glyphs {
            let span = spans.get(g.span);
            if self.pdf.licence(&g.font) == FontLicence::Restricted {
                if let Some(sp) = span {
                    if !self.pdf.report.restricted.contains(&sp.font_family) {
                        self.pdf.report.restricted.push(sp.font_family.clone());
                    }
                }
                curves.extend((transform * g.path.clone()).elements().iter().copied());
                continue;
            }
            let shown = self.show_glyph(g, span, mode);
            let m = transform * g.transform;
            match shown {
                Some(sh) if g.exact => visible.push((m, sh)),
                Some(sh) => {
                    curves.extend((transform * g.path.clone()).elements().iter().copied());
                    hidden.push((m, sh));
                }
                None => curves.extend((transform * g.path.clone()).elements().iter().copied()),
            }
        }
        // Fill: a colour on the curves and as the text's own; anything
        // else on every outline.
        if solid_fill {
            if !curves.elements().is_empty() {
                self.paint_area(&shape.fill, &curves, bounds, true);
            }
        } else {
            self.paint_area(&shape.fill, full, bounds, true);
        }
        // The text, then the outline of the curves (or of everything).
        let render = match (solid_fill, plain_stroke.is_some()) {
            (true, true) => 2,
            (true, false) => 0,
            (false, true) => 1,
            (false, false) => 3,
        };
        if let Fill::Solid(c) = shape.fill {
            let op = self.color_op(c, true);
            let _ = writeln!(self.content, "{op}");
        }
        if let Some(s) = plain_stroke {
            self.stroke_state(&crate::svg::drawn_stroke(s, transform).0);
        }
        self.show_text(&visible, render);
        self.show_text(&hidden, 3);
        if let Some(s) = &shape.stroke {
            if plain_stroke.is_some() {
                if !curves.elements().is_empty() {
                    self.path_ops(&curves);
                    self.content.push_str("S\n");
                }
            } else {
                self.outline(shape, s, parent, transform, full, bounds);
            }
        }
        true
    }

    /// The font and code that show glyph `g` of `span`: a standard font
    /// named only (unless those go in), else the embedded font, else the
    /// font named only. `None` when no font can show it here.
    fn show_glyph(
        &mut self,
        g: &tracedraw_core::document::text_outline::PlacedGlyph,
        span: Option<&tracedraw_core::TextSpan>,
        mode: TextMode,
    ) -> Option<fonts::Shown> {
        if !mode.embed_base14 {
            let base14 = span.and_then(|s| base14_name(&s.font_family, s.bold, s.italic));
            if let Some(name) = base14 {
                if let Some(sh) = self
                    .pdf
                    .show_named(&g.font, g.glyph_id, &g.text, Some(name))
                {
                    return Some(sh);
                }
            }
        }
        if mode.embed {
            self.pdf.show_embedded(&g.font, g.glyph_id, &g.text)
        } else {
            self.pdf.show_named(&g.font, g.glyph_id, &g.text, None)
        }
    }

    /// Show `glyphs` (each with its matrix from ems to page millimetres)
    /// with text render mode `render`: glyphs of one font on one line go
    /// in one run, set by one text matrix and kept in place by
    /// adjustments.
    fn show_text(&mut self, glyphs: &[(Affine, fonts::Shown)], render: u8) {
        if glyphs.is_empty() {
            return;
        }
        let _ = writeln!(self.content, "BT\n{render} Tr");
        // Clusters whose glyphs do not each stand for their own
        // characters (a vowel sign drawn before its consonant, a reph
        // after its syllable, a glyph used for other characters
        // elsewhere): their text in typed order as marked content, so
        // readers take the characters from it (#599). (first, last).
        let mut clusters: Vec<(usize, usize)> = Vec::new();
        for (i, (_, sh)) in glyphs.iter().enumerate() {
            match clusters.last_mut() {
                Some(c) if sh.text.is_empty() => c.1 = i,
                _ => clusters.push((i, i)),
            }
        }
        clusters.retain(|&(a, b)| {
            b > a
                || glyphs
                    .get(a)
                    .is_some_and(|(_, sh)| !sh.mapped && !sh.text.is_empty())
        });
        let mut font: Option<usize> = None;
        // The run: its matrix (points) and the pen along it, ems.
        let mut run: Option<(Affine, f64)> = None;
        let mut pending = String::new();
        let flush = |content: &mut String, pending: &mut String| {
            if !pending.is_empty() {
                let _ = writeln!(content, "[{}] TJ", pending.trim_end());
                pending.clear();
            }
        };
        let mut marked = clusters.into_iter().peekable();
        let mut open: Option<usize> = None;
        for (i, (m, sh)) in glyphs.iter().enumerate() {
            if let Some((_, last)) = marked.next_if(|c| c.0 == i) {
                flush(&mut self.content, &mut pending);
                let _ = writeln!(
                    self.content,
                    "/Span << /ActualText {} >> BDC",
                    pdf_text_any(&sh.text)
                );
                open = Some(last);
            }
            self.show_one(m, sh, &mut font, &mut run, &mut pending);
            if open == Some(i) {
                flush(&mut self.content, &mut pending);
                self.content.push_str("EMC\n");
                open = None;
            }
        }
        flush(&mut self.content, &mut pending);
        self.content.push_str("ET\n");
    }

    /// Show one glyph of [`Self::show_text`]: added to the pending
    /// adjustments of the current run, or a new run.
    fn show_one(
        &mut self,
        m: &Affine,
        sh: &fonts::Shown,
        font: &mut Option<usize>,
        run: &mut Option<(Affine, f64)>,
        pending: &mut String,
    ) {
        let flush = |content: &mut String, pending: &mut String| {
            if !pending.is_empty() {
                let _ = writeln!(content, "[{}] TJ", pending.trim_end());
                pending.clear();
            }
        };
        let m = Affine::scale(MM_PT) * *m;
        let c = m.as_coeffs();
        let det = c[0] * c[3] - c[1] * c[2];
        if !det.is_finite() || det.abs() < 1e-12 || c.iter().any(|v| !v.is_finite()) {
            return;
        }
        if *font != Some(sh.slot) {
            flush(&mut self.content, pending);
            let Some(slot) = self.pdf.fonts.get(sh.slot) else {
                return;
            };
            let (name, id) = (slot.name.clone(), slot.id);
            if !self.fonts.iter().any(|(n, _)| *n == name) {
                self.fonts.push((name.clone(), id));
            }
            let _ = writeln!(self.content, "/{name} 1 Tf");
            *font = Some(sh.slot);
            *run = None;
        }
        // Along the run: the glyph's origin in the run's space.
        let along = run.and_then(|(r, pen)| {
            let rc = r.as_coeffs();
            let same = (0..4).all(|i| (rc[i] - c[i]).abs() <= 1e-6 * (1.0 + rc[i].abs()));
            if !same {
                return None;
            }
            let p = r.inverse() * tracedraw_core::geometry::Point::new(c[4], c[5]);
            (p.y.abs() < 1e-3).then_some((p.x, pen))
        });
        let x = match along {
            Some((x, pen)) => {
                let adj = (pen - x) * 1000.0;
                if adj.abs() > 0.01 {
                    let _ = write!(pending, "{} ", f(adj));
                }
                x
            }
            None => {
                flush(&mut self.content, pending);
                let _ = writeln!(
                    self.content,
                    "{} {} {} {} {} {} Tm",
                    f6(c[0]),
                    f6(c[1]),
                    f6(c[2]),
                    f6(c[3]),
                    f(c[4]),
                    f(c[5])
                );
                *run = Some((m, 0.0));
                0.0
            }
        };
        pending.push('<');
        for b in &sh.code {
            let _ = write!(pending, "{b:02X}");
        }
        pending.push_str("> ");
        if let Some((_, pen)) = run.as_mut() {
            *pen = x + sh.width_em;
        }
    }
}

/// A number with six decimals at most (text matrices hold font sizes).
fn f6(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// PDF stitching function over the fountain's stops (type 3 wrapping type 2
/// segments), or a single type 2 function for two stops.
fn stitching_function_in(ft: &tracedraw_core::Fountain, cmyk_only: bool) -> String {
    let rgb_array = |c: Color| color_array(c, cmyk_only);
    // One run of stops with the edge pad, repeats, mirror and inversion
    // worked in.
    let mut stops: Vec<(f64, Color)> = ft
        .resolved_stops()
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
    // Segments of no width (a hard edge between two colours) are left
    // out, so the bounds rise strictly; the next segment starts with the
    // new colour.
    let segments: Vec<_> = stops
        .windows(2)
        .filter(|w| w[1].0 - w[0].0 > 1e-9)
        .collect();
    if segments.is_empty() {
        return format!(
            "<< /FunctionType 2 /Domain [0 1] /C0 {} /C1 {} /N 1 >>",
            rgb_array(stops[0].1),
            rgb_array(stops[stops.len() - 1].1)
        );
    }
    for (i, w) in segments.iter().enumerate() {
        let _ = write!(
            funcs,
            "<< /FunctionType 2 /Domain [0 1] /C0 {} /C1 {} /N 1 >> ",
            rgb_array(w[0].1),
            rgb_array(w[1].1)
        );
        encode.push_str("0 1 ");
        if i + 1 < segments.len() {
            let _ = write!(bounds, "{} ", f((w[1].0 - first) / span));
        }
    }
    format!(
        "<< /FunctionType 3 /Domain [0 1] /Functions [{}] /Bounds [{}] /Encode [{}] >>",
        funcs.trim_end(),
        bounds.trim_end(),
        encode.trim_end()
    )
}

/// An outline's own paint other than a plain colour, ready for page
/// space: a mesh's nodes go through the object's `transform`.
fn outline_paint(s: &tracedraw_core::Stroke, transform: Affine) -> Option<Fill> {
    match s.fill.as_deref()? {
        Fill::None | Fill::Solid(_) => None,
        Fill::Mesh(m) => {
            let mut m = m.clone();
            for n in m.nodes.iter_mut() {
                n.pos = transform * n.pos;
            }
            Some(Fill::Mesh(m))
        }
        other => Some(other.clone()),
    }
}

/// Rasterise a pattern, texture, mesh or shaped gradient fill over
/// `bounds` at 150 dpi (a shaped gradient follows `path`).
fn rasterise_fill(
    fill: &Fill,
    path: &tracedraw_core::BezPath,
    bounds: tracedraw_core::geometry::Rect,
) -> Option<(Vec<u8>, u32, u32)> {
    let pm = match fill {
        Fill::Fountain(_) => tracedraw_render::render_fill_image_in(fill, path, 150.0)?,
        _ => tracedraw_render::render_fill_image(fill, bounds, 150.0)?,
    };
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

/// The link areas of `shapes`: each linked object's bounds (in page
/// millimetres, `parent` applied) with its link. A linked group's link
/// covers its children; hidden objects have none.
fn link_areas(
    shapes: &[Shape],
    symbols: &[tracedraw_core::Symbol],
    parent: Affine,
    out: &mut Vec<(Rect, String)>,
) {
    use tracedraw_core::geometry::Shape as _;
    for s in shapes {
        if !s.visible {
            continue;
        }
        let transform = parent * s.transform;
        if let Some(link) = s.link.as_deref().map(str::trim).filter(|l| !l.is_empty()) {
            let r = (transform * s.local_path_with(symbols)).bounding_box();
            if r.width() > 0.0 || r.height() > 0.0 {
                out.push((r, link.to_string()));
            }
            continue;
        }
        if let ShapeKind::Group { children } = &s.kind {
            link_areas(children, symbols, transform, out);
        }
    }
}

/// What a link annotation does: `#page=n` goes to the top of page n of
/// the file; anything else is opened as an address (made 7-bit, as the
/// standard asks, by percent-encoding the rest).
fn link_target(link: &str, page_ids: &[usize], doc: &Document, bleed: f64) -> Option<String> {
    if let Some(n) = link.strip_prefix("#page=") {
        let n: usize = n.trim().parse().ok()?;
        let index = n.checked_sub(1)?;
        let id = page_ids.get(index)?;
        let top = (doc.pages.get(index)?.size.height + 2.0 * bleed) * MM_PT;
        return Some(format!("/Dest [{id} 0 R /XYZ 0 {} null]", f(top)));
    }
    let mut uri = String::with_capacity(link.len());
    for b in link.bytes() {
        if b.is_ascii_graphic() {
            uri.push(b as char);
        } else {
            let _ = write!(uri, "%{b:02X}");
        }
    }
    Some(format!("/A << /S /URI /URI {} >>", pdf_text(&uri)))
}

pub fn document_to_pdf(doc: &Document) -> Vec<u8> {
    document_to_pdf_with(doc, &PdfOptions::default())
}

/// Objects that need flattening for X-1a/X-3: non-opaque ones and those
/// with transparency or lens effects.
fn needs_flattening(s: &Shape) -> bool {
    if s.opacity < 1.0 || s.opacity_mask.is_some() {
        return true;
    }
    // A drop shadow is a soft, see-through picture.
    if s.shadow.is_some_and(|sh| sh.opacity > 0.0) {
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
    let layers = doc.layers_drawn_for_page(page.id).unwrap_or_default();
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
        // With the drop shadow, which reaches past the object.
        let mut b = s.bounds();
        if let Some(sh) = &s.shadow {
            b = b.union(sh.bounds(b));
        }
        let b = b.intersect(page_rect);
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
                png: png.into(),
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

/// The resolution opacity masks are drawn at in PDF output.
const MASK_DPI: f64 = 300.0;

/// The resolution drop shadows are drawn at in PDF output.
const SHADOW_DPI: f64 = 300.0;

/// The PDF blend mode closest to a shadow's merge mode (the drawing
/// window's blend for it, see `tracedraw_render::merge_blend`). Modes
/// with no PDF blend draw normally.
fn pdf_blend_mode(merge: tracedraw_core::live::MergeMode) -> &'static str {
    use tracedraw_core::live::MergeMode as M;
    match merge {
        M::Multiply => "Multiply",
        M::Subtract | M::Difference => "Difference",
        M::Divide => "Screen",
        M::IfLighter => "Lighten",
        M::IfDarker => "Darken",
        M::Hue => "Hue",
        M::Saturation => "Saturation",
        M::Color => "Color",
        M::Invert => "Exclusion",
        M::Normal | M::Add | M::And | M::Or | M::Xor | M::Red | M::Green | M::Blue => "Normal",
    }
}

/// `shape` (placed by `parent`) drawn on its own as a bitmap object with
/// its transparency, covering its bounds; the resolution drops so the
/// picture stays within 40 million pixels.
fn raster_of(
    shape: &Shape,
    parent: Affine,
    symbols: &[tracedraw_core::Symbol],
    dpi: f64,
) -> Option<Shape> {
    let b = parent.transform_rect_bbox(shape.bounds());
    if !(b.width() > 0.0 && b.height() > 0.0 && b.width().is_finite() && b.height().is_finite()) {
        return None;
    }
    let area_in2 = (b.width() / 25.4) * (b.height() / 25.4);
    let dpi = dpi.min((40.0e6 / area_in2.max(1e-9)).sqrt()).max(1.0);
    let mut tmp = Document::new("mask", tracedraw_core::Size::new(b.width(), b.height()));
    tmp.symbols = symbols.to_vec();
    let mut placed = shape.clone();
    placed.transform = Affine::translate((-b.x0, -b.y0)) * parent * shape.transform;
    tmp.pages[0].background = None;
    tmp.pages[0].layers[0].shapes = vec![placed];
    let pid = tmp.pages[0].id;
    // On a transparent background (not the white page of a page picture).
    let zoom = dpi / 25.4;
    let (w, h) = (
        (b.width() * zoom).ceil().max(1.0) as u32,
        (b.height() * zoom).ceil().max(1.0) as u32,
    );
    let pm = tracedraw_render::render_page(
        &tmp,
        pid,
        &tracedraw_render::RenderOptions {
            width: w,
            height: h,
            view: tracedraw_render::ViewTransform {
                zoom,
                origin_x: 0.0,
                origin_y: h as f64,
            },
            ..Default::default()
        },
    )?;
    let png = pm.encode_png().ok()?;
    let mut raster = Shape::new(
        shape.id,
        ShapeKind::Bitmap {
            // Whole pixels: the picture reaches a little past the bounds.
            rect: Rect::new(b.x0, b.y0, b.x0 + w as f64 / zoom, b.y0 + h as f64 / zoom),
            width_px: pm.width(),
            height_px: pm.height(),
            png: png.into(),
            fx: None,
        },
    );
    raster.fill = Fill::None;
    raster.stroke = None;
    Some(raster)
}

/// Write every page with the given options (PDF/X, bleed, metadata).
pub fn document_to_pdf_with(doc: &Document, opts: &PdfOptions) -> Vec<u8> {
    document_to_pdf_report(doc, opts).0
}

/// [`document_to_pdf_with`], with what went into the file: its fonts and
/// the fonts whose text became curves.
pub fn document_to_pdf_report(doc: &Document, opts: &PdfOptions) -> (Vec<u8>, PdfReport) {
    let doc = &*crate::resolve_open_fills(doc);
    let std_ = opts.standard;
    let mut pdf = Pdf {
        objects: Vec::new(),
        spot_spaces: std::collections::HashMap::new(),
        cmyk_only: std_.cmyk_only(),
        allow_alpha: !std_.flatten(),
        encrypt_id: None,
        fonts: Vec::new(),
        bad_fonts: Vec::new(),
        report: PdfReport::default(),
        images: opts.images,
    };
    // PDF/X wants every font in the file.
    let text_mode = (!opts.text.as_curves).then_some(TextMode {
        embed: opts.text.embed_fonts || std_ != PdfStandard::None,
        embed_base14: opts.text.embed_base14 || std_ != PdfStandard::None,
    });
    pdf.add_str(String::new()); // 1: catalog, filled at the end
    pdf.add_str(String::new()); // 2: pages
    let bleed = if opts.bleed_mm.is_finite() {
        opts.bleed_mm.max(0.0)
    } else {
        0.0
    };
    let slug = if opts.marks_margin_mm.is_finite() {
        opts.marks_margin_mm.clamp(0.0, 100.0)
    } else {
        0.0
    };
    // From the media box's edge to the page: the marks margin and the bleed.
    let edge = slug + bleed;
    let flatten_dpi = if opts.flatten_dpi > 0.0 {
        opts.flatten_dpi
    } else {
        300.0
    };
    let mut page_ids = Vec::new();
    // Each page's link areas, made into annotations once every page has
    // its object (a link can go to a later page).
    let mut page_link_areas: Vec<Vec<(Rect, String)>> = Vec::new();
    for (page_no, page) in doc.pages.iter().enumerate() {
        let mut areas = Vec::new();
        // PDF/X allows no link annotations on the page.
        if std_ == PdfStandard::None {
            let offset = Affine::translate((edge, edge));
            for layer in doc
                .layers_drawn_for_page(page.id)
                .unwrap_or_default()
                .iter()
                .filter(|l| l.visible && l.printable)
            {
                link_areas(&layer.shapes, &doc.symbols, offset, &mut areas);
            }
        }
        page_link_areas.push(areas);
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
            inks: &doc.inks,
            spaces: Vec::new(),
            fonts: Vec::new(),
            text: text_mode,
            media: Rect::new(
                0.0,
                0.0,
                page.size.width + 2.0 * edge,
                page.size.height + 2.0 * edge,
            ),
        };
        let marks = opts.page_marks.get(page_no).filter(|m| !m.is_empty());
        if slug > 0.0 {
            // The drawing stops at the bleed: the marks margin stays clear.
            let _ = writeln!(
                w.content,
                "q {} {} {} {} re W n",
                f(slug * MM_PT),
                f(slug * MM_PT),
                f((page.size.width + 2.0 * bleed) * MM_PT),
                f((page.size.height + 2.0 * bleed) * MM_PT)
            );
        }
        let offset = Affine::translate((edge, edge));
        if let Some(bg) = &page.background {
            // The page's background covers its bleed too.
            let mut bg_shape = Shape::new(
                tracedraw_core::ShapeId(0),
                ShapeKind::Rect {
                    rect: page.rect().inflate(bleed, bleed),
                    radius: 0.0,
                    corners: None,
                },
            );
            bg_shape.fill = bg.clone();
            bg_shape.stroke = None;
            w.shape(&bg_shape, offset);
        }
        match &flat {
            Some(shapes) => {
                for s in shapes {
                    w.shape(s, offset);
                }
            }
            None => {
                let layers = doc.layers_drawn_for_page(page.id).unwrap_or_default();
                for layer in layers.iter().filter(|l| l.visible && l.printable) {
                    for s in &layer.shapes {
                        w.shape(s, offset);
                    }
                }
            }
        }
        if slug > 0.0 {
            w.content.push_str("Q\n");
        }
        for s in marks.into_iter().flatten() {
            w.shape(s, offset);
        }
        let PageWriter {
            content,
            shadings,
            gstates,
            images,
            spaces,
            fonts,
            ..
        } = w;
        let content_id = if pdf.images.compress_content {
            let data = flate(content.as_bytes());
            pdf.stream("/Filter /FlateDecode", &data)
        } else {
            pdf.stream("", content.as_bytes())
        };
        let mut res = String::from("<< ");
        if !shadings.is_empty() {
            res.push_str("/Shading << ");
            for (n, id) in &shadings {
                let _ = write!(res, "/{n} {id} 0 R ");
            }
            res.push_str(">> ");
        }
        if !spaces.is_empty() {
            res.push_str("/ColorSpace << ");
            for (n, id) in &spaces {
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
        if !fonts.is_empty() {
            res.push_str("/Font << ");
            for (n, id) in &fonts {
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
            (page.size.width + 2.0 * edge) * MM_PT,
            (page.size.height + 2.0 * edge) * MM_PT,
        );
        let boxes = if std_ != PdfStandard::None || edge > 0.0 {
            format!(
                " /TrimBox [{} {} {} {}] /BleedBox [{} {} {} {}]",
                f(edge * MM_PT),
                f(edge * MM_PT),
                f((edge + page.size.width) * MM_PT),
                f((edge + page.size.height) * MM_PT),
                f(slug * MM_PT),
                f(slug * MM_PT),
                f((edge + bleed + page.size.width) * MM_PT),
                f((edge + bleed + page.size.height) * MM_PT)
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
    pdf.finish_fonts(&opts.text);
    // Link annotations: a web or mail address, or a page of the file.
    for (areas, &page_id) in page_link_areas.iter().zip(&page_ids) {
        let mut annots = Vec::new();
        for (rect, link) in areas {
            let Some(target) = link_target(link, &page_ids, doc, edge) else {
                continue;
            };
            let id = pdf.add_str(format!(
                "<< /Type /Annot /Subtype /Link /Rect [{} {} {} {}] /Border [0 0 0] {target} >>",
                f(rect.x0 * MM_PT),
                f(rect.y0 * MM_PT),
                f(rect.x1 * MM_PT),
                f(rect.y1 * MM_PT)
            ));
            annots.push(format!("{id} 0 R"));
        }
        if annots.is_empty() {
            continue;
        }
        let body = &mut pdf.objects[page_id - 1];
        if body.ends_with(b" >>") {
            body.truncate(body.len() - 3);
            body.extend_from_slice(format!(" /Annots [{}] >>", annots.join(" ")).as_bytes());
        }
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
        pdf_text_any(&title),
        pdf_text(&date),
        pdf_text(&date)
    );
    // Author, subject and keywords: the options', else the document's
    // properties; an empty value leaves the entry out.
    let pick = |given: &Option<String>, own: &str| -> String {
        given.as_deref().unwrap_or(own).trim().to_string()
    };
    let author = pick(&opts.author, &doc.metadata.author);
    let subject = pick(&opts.subject, &doc.metadata.subject);
    let keywords = pick(&opts.keywords, &doc.metadata.keywords);
    for (key, value) in [
        ("Author", &author),
        ("Subject", &subject),
        ("Keywords", &keywords),
    ] {
        if !value.is_empty() {
            let _ = write!(info, " /{key} {}", pdf_text_any(value));
        }
    }
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
    // Bookmarks: one per page, in page order, each to the page's top.
    if opts.bookmarks && !page_ids.is_empty() {
        let outlines = pdf.add_str(String::new());
        let first = pdf.objects.len() + 1;
        let n = page_ids.len();
        for (i, (page, &page_id)) in doc.pages.iter().zip(&page_ids).enumerate() {
            let id = first + i;
            let name = page.name.trim();
            let title = if name.is_empty() {
                format!("Page {}", i + 1)
            } else {
                name.to_string()
            };
            let top = (page.size.height + 2.0 * edge) * MM_PT;
            let mut item = format!(
                "<< /Title {} /Parent {outlines} 0 R /Dest [{page_id} 0 R /XYZ 0 {} null]",
                pdf_text_any(&title),
                f(top)
            );
            if i > 0 {
                let _ = write!(item, " /Prev {} 0 R", id - 1);
            }
            if i + 1 < n {
                let _ = write!(item, " /Next {} 0 R", id + 1);
            }
            item.push_str(" >>");
            pdf.add_str(item);
        }
        pdf.objects[outlines - 1] = format!(
            "<< /Type /Outlines /First {first} 0 R /Last {} 0 R /Count {n} >>",
            first + n - 1
        )
        .into_bytes();
        let _ = write!(catalog, " /Outlines {outlines} 0 R");
        if opts.show_bookmarks {
            catalog.push_str(" /PageMode /UseOutlines");
        }
    }
    let security = opts.security.as_ref().filter(|_| std_ == PdfStandard::None);
    if security.is_some() {
        // AES-256 (revision 6) came with PDF 1.7, extension level 8.
        catalog.push_str(" /Extensions << /ADBE << /BaseVersion /1.7 /ExtensionLevel 8 >> >>");
    }
    if std_ != PdfStandard::None {
        let profile = output_profile(opts, doc);
        let condition = if !opts.output_condition.trim().is_empty() {
            opts.output_condition.trim().to_string()
        } else if let Some(icc) = profile {
            // The embedded profile is the condition: a registered name
            // would describe another press.
            profile_condition(icc)
        } else {
            "CGATS TR 001".to_string()
        };
        let profile_ref = profile.map(|icc| {
            let n = icc_channels(icc);
            let id = pdf.stream(&format!("/N {n}"), icc);
            format!(" /DestOutputProfile {id} 0 R")
        });
        let intent = pdf.add_str(format!(
            "<< /Type /OutputIntent /S /GTS_PDFX /OutputConditionIdentifier {} /OutputCondition {} /RegistryName (http://www.color.org) /Info {}{} >>",
            pdf_text_any(&condition),
            pdf_text_any(&condition),
            pdf_text_any(&condition),
            profile_ref.unwrap_or_default()
        ));
        let _ = write!(catalog, " /OutputIntents [{intent} 0 R]");
        // XMP metadata (required by X-4, harmless elsewhere).
        let xmp = xmp_packet(
            &XmpText {
                title: &title,
                author: &author,
                subject: &subject,
                keywords: &keywords,
            },
            std_,
            &date,
        );
        let meta = pdf.stream("/Type /Metadata /Subtype /XML", xmp.as_bytes());
        let _ = write!(catalog, " /Metadata {meta} 0 R");
    }
    // The drawing's language (Document Properties), for readers and
    // screen readers.
    let lang = doc.metadata.language.trim();
    if !lang.is_empty() && lang.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        let _ = write!(catalog, " /Lang ({lang})");
    }
    catalog.push_str(" >>");
    pdf.objects[0] = catalog.into_bytes();
    let report = std::mem::take(&mut pdf.report);
    let bytes = match security {
        Some(sec) => {
            let random: crate::pdf_crypt::Random = Box::new(|buf: &mut [u8]| {
                if getrandom::fill(buf).is_err() {
                    // No system randomness: a time-seeded generator.
                    let mut x = web_time::SystemTime::now()
                        .duration_since(web_time::UNIX_EPOCH)
                        .map(|d| d.as_nanos() as u64)
                        .unwrap_or(0x9E37_79B9_7F4A_7C15)
                        | 1;
                    for b in buf.iter_mut() {
                        x ^= x << 13;
                        x ^= x >> 7;
                        x ^= x << 17;
                        *b = (x >> 32) as u8;
                    }
                }
            });
            pdf.encrypt(sec, random);
            pdf.finish_with("1.7", Some(info_id))
        }
        None => pdf.finish_with(std_.version(), Some(info_id)),
    };
    (bytes, report)
}

/// The output intent's profile: the one given, else the CMYK profile the
/// drawing carries, the profile its CMYK values are in.
fn output_profile<'a>(opts: &'a PdfOptions, doc: &'a Document) -> Option<&'a [u8]> {
    opts.output_profile.as_deref().or_else(|| {
        doc.cmyk_profile
            .as_ref()
            .map(|p| p.data.as_slice())
            .filter(|icc| icc_channels(icc) == 4)
    })
}

/// The output condition an embedded profile stands for: its description,
/// else "Custom".
fn profile_condition(icc: &[u8]) -> String {
    tracedraw_core::icc::Profile::parse(icc)
        .ok()
        .and_then(|p| p.description().map(|d| d.trim().to_string()))
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| "Custom".to_string())
}

/// Channel count of an ICC profile from its colour space signature.
fn icc_channels(icc: &[u8]) -> u32 {
    match icc.get(16..20) {
        Some(b"GRAY") => 1,
        Some(b"CMYK") => 4,
        _ => 3,
    }
}

/// The document information the XMP packet repeats.
struct XmpText<'a> {
    title: &'a str,
    author: &'a str,
    subject: &'a str,
    keywords: &'a str,
}

fn xmp_packet(text: &XmpText, std_: PdfStandard, date: &str) -> String {
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
        esc(text.title)
    );
    // The same author, subject and keywords as the document information.
    if !text.author.is_empty() {
        let _ = writeln!(
            x,
            "<dc:creator><rdf:Seq><rdf:li>{}</rdf:li></rdf:Seq></dc:creator>",
            esc(text.author)
        );
    }
    if !text.subject.is_empty() {
        let _ = writeln!(
            x,
            "<dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:description>",
            esc(text.subject)
        );
    }
    if !text.keywords.is_empty() {
        let _ = writeln!(x, "<pdf:Keywords>{}</pdf:Keywords>", esc(text.keywords));
    }
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

    /// A 600 x 600 pixel picture placed 25.4 mm wide (600 dpi): kept as
    /// it is at first; downsampled to 150 dpi (150 x 150) when asked; JPEG
    /// compressed when asked; a gray picture goes in with one channel; the
    /// page content can stay uncompressed (#550).
    #[test]
    fn pictures_are_downsampled_and_compressed_as_asked() {
        let picture = |gray: bool| {
            let img = image::RgbImage::from_fn(600, 600, |x, y| {
                let v = ((x * 7 + y * 3) % 256) as u8;
                if gray {
                    image::Rgb([v, v, v])
                } else {
                    image::Rgb([v, 255 - v, (x % 256) as u8])
                }
            });
            let mut png = std::io::Cursor::new(Vec::new());
            img.write_to(&mut png, image::ImageFormat::Png)
                .expect("png");
            let mut doc = Document::default();
            let layer = doc.pages[0].layers[0].id;
            let id = doc.ids_mut().shape();
            let s = Shape::new(
                id,
                ShapeKind::Bitmap {
                    rect: Rect::new(10.0, 10.0, 35.4, 35.4),
                    width_px: 600,
                    height_px: 600,
                    png: png.into_inner().into(),
                    fx: Default::default(),
                },
            );
            doc.layer_mut(layer).unwrap().shapes.push(s);
            doc
        };
        let text = |doc: &Document, images: PdfImages| {
            let opts = PdfOptions {
                images,
                ..Default::default()
            };
            String::from_utf8_lossy(&document_to_pdf_with(doc, &opts)).to_string()
        };
        let color = picture(false);
        let plain = text(&color, PdfImages::default());
        assert!(plain.contains("/Width 600 /Height 600"));
        assert!(plain.contains("/DeviceRGB"));
        let small = text(
            &color,
            PdfImages {
                color_dpi: Some(150.0),
                ..Default::default()
            },
        );
        assert!(small.contains("/Width 150 /Height 150"), "downsampled");
        assert!(small.len() < plain.len());
        let jpeg = text(
            &color,
            PdfImages {
                jpeg: true,
                jpeg_quality: 60,
                ..Default::default()
            },
        );
        assert!(jpeg.contains("/DCTDecode"));
        // Below the resolution asked: left alone.
        let kept = text(
            &color,
            PdfImages {
                color_dpi: Some(1200.0),
                ..Default::default()
            },
        );
        assert!(kept.contains("/Width 600 /Height 600"));
        // Gray: one channel, its own resolution.
        let gray = picture(true);
        let g = text(
            &gray,
            PdfImages {
                color_dpi: Some(72.0),
                gray_dpi: Some(300.0),
                ..Default::default()
            },
        );
        assert!(
            g.contains("/Width 300 /Height 300 /ColorSpace /DeviceGray"),
            "gray"
        );
        // The page content uncompressed: its operators readable.
        let raw = text(
            &color,
            PdfImages {
                compress_content: false,
                ..Default::default()
            },
        );
        assert!(raw.contains("cm /Im0 Do"));
        assert!(!plain.contains("cm /Im0 Do"));
    }

    /// A picture wholly outside the page and its bleed cannot show: it
    /// stays out of the file. One in the bleed, or partly on the page,
    /// goes in (#642).
    #[test]
    fn pictures_wholly_outside_the_media_box_stay_out_of_the_file() {
        let mut doc = Document::default();
        let page = doc.pages[0].size;
        let layer = doc.pages[0].layers[0].id;
        let place = |doc: &mut Document, rect: Rect| {
            let img = image::RgbImage::from_fn(8, 8, |x, y| {
                image::Rgb([(x * 30) as u8, (y * 30) as u8, 128])
            });
            let mut png = std::io::Cursor::new(Vec::new());
            img.write_to(&mut png, image::ImageFormat::Png)
                .expect("png");
            let id = doc.ids_mut().shape();
            let s = Shape::new(
                id,
                ShapeKind::Bitmap {
                    rect,
                    width_px: 8,
                    height_px: 8,
                    png: png.into_inner().into(),
                    fx: Default::default(),
                },
            );
            doc.layer_mut(layer).unwrap().shapes.push(s);
        };
        let pictures = |doc: &Document, bleed_mm: f64| {
            let opts = PdfOptions {
                bleed_mm,
                ..Default::default()
            };
            String::from_utf8_lossy(&document_to_pdf_with(doc, &opts))
                .matches("/Subtype /Image")
                .count()
        };
        // 2 mm right of the page.
        place(
            &mut doc,
            Rect::new(page.width + 2.0, 10.0, page.width + 30.0, 40.0),
        );
        assert_eq!(pictures(&doc, 0.0), 0, "outside the page");
        assert_eq!(pictures(&doc, 3.0), 1, "in the 3 mm bleed");
        // Half on the page, across its left edge.
        place(&mut doc, Rect::new(-10.0, 10.0, 10.0, 30.0));
        assert_eq!(pictures(&doc, 0.0), 1, "partly on the page");
    }

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
                tracedraw_core::Stop::new(0.0, Color::rgb8(255, 0, 0)),
                tracedraw_core::Stop::new(0.3, Color::rgb8(0, 255, 0)),
                tracedraw_core::Stop::new(1.0, Color::rgb8(0, 0, 255)),
            ],
            angle: 90.0,
            offset: tracedraw_core::geometry::Point::ZERO,
            edge_pad: 0.0,
            ..Default::default()
        });
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let text = String::from_utf8_lossy(&document_to_pdf(&doc)).to_string();
        assert!(text.contains("/FunctionType 3"));
        assert!(text.contains("/Bounds [0.3]"));
    }

    /// A fill whose nodes were placed freely is a unit shading drawn
    /// through its frame (#330).
    #[test]
    fn free_fill_nodes_draw_a_unit_shading_through_their_frame() {
        use tracedraw_core::style::FountainHandles;
        let mut f =
            tracedraw_core::Fountain::two(FountainKind::Linear, Color::BLACK, Color::WHITE, 0.0);
        f.handles = Some(FountainHandles {
            start: tracedraw_core::geometry::Point::new(-1.0, 0.0),
            end: tracedraw_core::geometry::Point::new(0.0, 0.0),
            skew: tracedraw_core::geometry::Point::new(-1.0, 1.0),
        });
        let text = one_rect_pdf(Fill::Fountain(f.clone()), None);
        assert!(text.contains("/Coords [0 0 1 0]"), "{text}");
        assert!(text.contains(" cm /Sh0 sh"), "{text}");
        f.kind = FountainKind::Radial;
        let text = one_rect_pdf(Fill::Fountain(f), None);
        assert!(text.contains("/Coords [0 0 0 0 0 1]"), "{text}");
    }

    fn one_rect_pdf(fill: Fill, stroke: Option<tracedraw_core::Stroke>) -> String {
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
        // The file as text, then every zlib stream inflated.
        let bytes = document_to_pdf(&doc);
        let mut text = String::from_utf8_lossy(&bytes).to_string();
        let mut rest: &[u8] = &bytes;
        while let Some(i) = rest.windows(7).position(|w| w == b"stream\n") {
            let data = &rest[i + 7..];
            use std::io::Read;
            let mut out = Vec::new();
            if flate2::read::ZlibDecoder::new(data)
                .read_to_end(&mut out)
                .is_ok()
            {
                text.push_str(&String::from_utf8_lossy(&out));
            }
            rest = data;
        }
        text
    }

    #[test]
    fn gradient_options_and_outline_paints_reach_the_pdf() {
        use tracedraw_core::{Fountain, FountainRepeat};
        let red = Color::rgb8(255, 0, 0);
        let blue = Color::rgb8(0, 0, 255);
        // A sawtooth: hard edges are left out of the stitching, so the
        // bounds rise strictly (0.5 once, between two runs).
        let mut saw = Fountain::two(FountainKind::Linear, red, blue, 0.0);
        saw.repeat = FountainRepeat::Repeat;
        saw.repeats = 2;
        let text = one_rect_pdf(Fill::Fountain(saw), None);
        assert!(text.contains("/Bounds [0.5]"), "{text}");
        // A shaped gradient is the renderer's picture clipped by the path.
        let shaped = Fountain::two(FountainKind::Shaped, red, blue, 0.0);
        let text = one_rect_pdf(Fill::Fountain(shaped), None);
        assert!(text.contains("/Subtype /Image"), "{text}");
        assert!(!text.contains("/ShadingType"), "{text}");
        // A gradient outline: its area clipped and shaded, not a stroke
        // in the outline colour.
        let mut st = tracedraw_core::Stroke::new(Color::rgb8(0, 255, 0), 2.0);
        st.fill = Some(Box::new(Fill::linear(red, blue, 0.0)));
        let text = one_rect_pdf(Fill::None, Some(st.clone()));
        assert!(text.contains("/ShadingType 2"), "{text}");
        assert!(text.contains("W n /Sh0 sh"), "{text}");
        assert!(!text.contains("\nS\n"), "{text}");
        // A texture outline: the renderer's pixels in the outline's area.
        st.fill = Some(Box::new(Fill::Texture(tracedraw_core::Texture {
            kind: tracedraw_core::TextureKind::Wood,
            color_a: red,
            color_b: blue,
            scale: 4.0,
            seed: 2,
        })));
        let text = one_rect_pdf(Fill::None, Some(st.clone()));
        assert!(text.contains("/Subtype /Image"), "{text}");
        // A hairline keeps its plain stroke.
        st.width = tracedraw_core::Stroke::HAIRLINE;
        let text = one_rect_pdf(Fill::None, Some(st));
        assert!(text.contains("\nS\n"), "{text}");
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

    /// The page's content streams, inflated.
    fn page_content(bytes: &[u8]) -> String {
        let mut content = String::new();
        let mut rest: &[u8] = bytes;
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
        content
    }

    /// An outline behind the fill is painted first and the fill over it,
    /// as the drawing window draws it (#235: a white outline behind black
    /// letters covered them in the PDF).
    #[test]
    fn an_outline_behind_the_fill_is_painted_first() {
        let order = |behind: bool| -> (usize, usize) {
            let mut doc = Document::default();
            let layer = doc.pages[0].layers[0].id;
            let mut sq = Shape::new(
                tracedraw_core::ShapeId(1),
                ShapeKind::Rect {
                    rect: tracedraw_core::Rect::new(10.0, 10.0, 60.0, 40.0),
                    radius: 0.0,
                    corners: None,
                },
            );
            sq.fill = Fill::Solid(Color::rgb8(0, 0, 0));
            sq.stroke = Some(tracedraw_core::Stroke {
                behind_fill: behind,
                ..tracedraw_core::Stroke::new(Color::rgb8(255, 255, 255), 2.0)
            });
            doc.layer_mut(layer).unwrap().shapes.push(sq);
            let content = page_content(&document_to_pdf(&doc));
            let lines: Vec<&str> = content.lines().map(str::trim).collect();
            let stroke = lines.iter().position(|l| *l == "S").expect("a stroke");
            let fill = lines
                .iter()
                .position(|l| *l == "f" || *l == "f*")
                .expect("a fill");
            (stroke, fill)
        };
        let (s, f) = order(true);
        assert!(s < f, "behind: the outline first ({s} {f})");
        let (s, f) = order(false);
        assert!(f < s, "in front: the fill first ({s} {f})");
    }

    /// An outline that scales with its object is as wide in the PDF as in
    /// the drawing window: its width times the object's scale factor.
    #[test]
    fn an_outline_that_scales_with_its_object_takes_the_object_scale() {
        let width = |scale_with_object: bool| -> String {
            let mut doc = Document::default();
            let layer = doc.pages[0].layers[0].id;
            let mut sq = Shape::new(
                tracedraw_core::ShapeId(1),
                ShapeKind::Rect {
                    rect: tracedraw_core::Rect::new(0.0, 0.0, 200.0, 100.0),
                    radius: 0.0,
                    corners: None,
                },
            );
            sq.transform = Affine::translate((20.0, 20.0)) * Affine::scale(0.25);
            sq.stroke = Some(tracedraw_core::Stroke {
                scale_with_object,
                ..tracedraw_core::Stroke::new(Color::rgb8(0, 0, 0), 10.0)
            });
            doc.layer_mut(layer).unwrap().shapes.push(sq);
            let content = page_content(&document_to_pdf(&doc));
            let line = content
                .lines()
                .find(|l| l.contains(" w "))
                .expect("a line width");
            let (before, _) = line.split_once(" w ").expect("the width");
            before.rsplit(' ').next().unwrap_or_default().to_string()
        };
        assert_eq!(width(true), f(2.5 * MM_PT));
        assert_eq!(width(false), f(10.0 * MM_PT));
    }

    #[test]
    fn spot_colors_print_on_separation_color_spaces() {
        let mut doc = Document::default();
        doc.inks.push(tracedraw_core::document::Ink {
            id: 1,
            name: "Gold 871".into(),
            cmyk: [0.0, 0.2, 0.8, 0.1],
        });
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
        s.fill = Fill::Solid(doc.inks[0].color(0.5));
        s.stroke = Some(tracedraw_core::Stroke::new(doc.inks[0].color(1.0), 1.0));
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let bytes = document_to_pdf(&doc);
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("[/Separation /Gold#20871 /DeviceCMYK"),
            "{text}"
        );
        assert!(text.contains("/C1 [0 0.2 0.8 0.1]"));
        assert!(text.contains("/ColorSpace << /CS1 "));
        // One colour space for both uses.
        assert_eq!(text.matches("/Separation").count(), 1);
        // A spot colour whose ink is gone prints as its process stand-in.
        doc.inks.clear();
        let text = String::from_utf8_lossy(&document_to_pdf(&doc)).to_string();
        assert!(!text.contains("/Separation"));
        assert_eq!(pdf_name("A/B c"), "/A#2FB#20c");
        assert_eq!(pdf_name(""), "/Spot");
    }

    #[test]
    fn opacity_masks_go_in_as_transparent_pictures() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let mut contents = Shape::new(
            doc.ids_mut().shape(),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 40.0, 40.0),
                radius: 0.0,
                corners: None,
            },
        );
        contents.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        let mut frame = Shape::new(
            doc.ids_mut().shape(),
            ShapeKind::Ellipse {
                rect: Rect::new(5.0, 5.0, 35.0, 35.0),
                arc: None,
            },
        );
        frame.fill = Fill::linear(Color::WHITE, Color::BLACK, 0.0);
        let mut clip = Shape::new(
            doc.ids_mut().shape(),
            ShapeKind::ClipFrame {
                frame: Box::new(frame),
                contents: vec![contents],
            },
        );
        clip.opacity_mask = Some(tracedraw_core::document::OpacityMask::default());
        doc.layer_mut(layer).unwrap().shapes.push(clip);
        let text = String::from_utf8_lossy(&document_to_pdf(&doc)).to_string();
        assert!(text.contains("/Subtype /Image"), "a picture");
        assert!(text.contains("/SMask"), "with its transparency");
        assert!(
            !text.contains("/ShadingType"),
            "the frame's gradient is not drawn"
        );
    }

    /// The image streams of `bytes` with their dictionaries, inflated.
    fn image_streams(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        let mut rest: &[u8] = bytes;
        while let Some(i) = rest.windows(7).position(|w| w == b"stream\n") {
            let dict_start = rest[..i].windows(2).rposition(|w| w == b"<<").unwrap_or(0);
            let dict = String::from_utf8_lossy(&rest[dict_start..i]).into_owned();
            let data = &rest[i + 7..];
            if dict.contains("/Subtype /Image") {
                use std::io::Read;
                let mut px = Vec::new();
                if flate2::read::ZlibDecoder::new(data)
                    .read_to_end(&mut px)
                    .is_ok()
                {
                    out.push((dict, px));
                }
            }
            rest = data;
        }
        out
    }

    /// A drop shadow reaches the PDF: a blurred picture in the shadow's
    /// colour with its opacity in a soft mask, under the object, over the
    /// shadow's area, multiplied (#1029: the shadow showed only once
    /// broken apart into a bitmap).
    #[test]
    fn a_drop_shadow_goes_in_beneath_its_object() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(20.0, 20.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        s.stroke = None;
        s.shadow = Some(tracedraw_core::Shadow {
            offset: tracedraw_core::geometry::Vec2::new(3.0, -3.0),
            opacity: 0.6,
            blur: 1.0,
            color: Color::rgb8(0, 0, 255),
            ..Default::default()
        });
        doc.layer_mut(layer).unwrap().shapes.push(s.clone());
        let bytes = document_to_pdf(&doc);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        assert!(text.contains("/BM /Multiply"), "merged as it asks");
        let content = page_content(&bytes);
        let draw = content.find(" cm /Im0 Do").expect("the shadow's picture");
        let fill = content.find("1 0 0 rg").expect("the object's fill");
        assert!(draw < fill, "the shadow beneath the object");
        // Placed over the moved box grown by three times the feathering.
        let line = content[..draw].lines().last().unwrap_or_default();
        let m: Vec<f64> = line
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        assert_eq!(m.len(), 6, "{line}");
        let (x0, y0) = (m[4] / MM_PT, m[5] / MM_PT);
        let (x1, y1) = (x0 + m[0] / MM_PT, y0 + m[3] / MM_PT);
        for (got, want) in [(x0, 20.0), (y0, 14.0), (x1, 66.0), (y1, 40.0)] {
            assert!((got - want).abs() < 0.2, "{x0} {y0} {x1} {y1}");
        }
        // Blue, at most the shadow's opacity, solid in the middle.
        let images = image_streams(&bytes);
        let mask = images
            .iter()
            .find(|(d, _)| d.contains("/DeviceGray"))
            .expect("a soft mask");
        let picture = images
            .iter()
            .find(|(d, _)| d.contains("/SMask"))
            .expect("the picture");
        let top = mask.1.iter().copied().max().unwrap_or(0);
        assert!((150..=156).contains(&top), "opacity 0.6: {top}");
        let at = mask.1.iter().position(|a| *a == top).unwrap();
        assert_eq!(&picture.1[at * 3..at * 3 + 3], &[0, 0, 255]);
        assert!(mask.1.contains(&0), "feathered to nothing");

        // No shadow, no picture.
        s.shadow = None;
        doc.layer_mut(layer).unwrap().shapes = vec![s];
        assert!(!page_content(&document_to_pdf(&doc)).contains("/Im0 Do"));
    }

    /// PDF/X-1a keeps no transparency: the shadow is flattened into the
    /// page picture with its object, over the shadow's area too.
    #[test]
    fn a_drop_shadow_is_flattened_for_x1a() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(20.0, 20.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        s.shadow = Some(tracedraw_core::Shadow::default());
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let opts = PdfOptions {
            standard: PdfStandard::X1a,
            flatten_dpi: 72.0,
            ..Default::default()
        };
        let bytes = document_to_pdf_with(&doc, &opts);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        assert!(!text.contains("/SMask") && !text.contains("/BM"));
        let content = page_content(&bytes);
        let draw = content.find(" cm /Im0 Do").expect("the flattened picture");
        let line = content[..draw].lines().last().unwrap_or_default();
        let m: Vec<f64> = line
            .split_whitespace()
            .filter_map(|v| v.parse().ok())
            .collect();
        assert_eq!(m.len(), 6, "{line}");
        // Down to the shadow's lower edge (2 mm down, 4.5 mm feathering).
        assert!(m[5] / MM_PT < 14.0, "{line}");
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

    /// Room for printers' marks: the MediaBox grows by it on every side,
    /// the BleedBox is the page and its bleed, the TrimBox the page; the
    /// drawing is clipped to the bleed and the marks are drawn after, in
    /// registration colour on the All separation.
    #[test]
    fn marks_margin_grows_the_media_box_and_keeps_trim_and_bleed() {
        let doc = doc_with_transparent_rect();
        let mut mark = Shape::new(
            tracedraw_core::ShapeId(0),
            ShapeKind::Path {
                path: {
                    let mut p = tracedraw_core::BezPath::new();
                    p.move_to((-8.0, 0.0));
                    p.line_to((-4.0, 0.0));
                    p
                },
                closed: false,
            },
        );
        mark.fill = Fill::None;
        mark.stroke = Some(tracedraw_core::Stroke::new(Color::Registration, 0.1));
        let opts = PdfOptions {
            bleed_mm: 3.0,
            marks_margin_mm: 12.0,
            page_marks: vec![vec![mark]],
            images: PdfImages {
                compress_content: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let bytes = document_to_pdf_with(&doc, &opts);
        let text = String::from_utf8_lossy(&bytes);
        // The page is 100 x 50 mm: 15 mm of bleed and marks per side.
        assert!(
            text.contains(&format!(
                "/MediaBox [0 0 {} {}]",
                f(130.0 * MM_PT),
                f(80.0 * MM_PT)
            )),
            "{text}"
        );
        assert!(text.contains(&format!(
            "/TrimBox [{} {} {} {}]",
            f(15.0 * MM_PT),
            f(15.0 * MM_PT),
            f(115.0 * MM_PT),
            f(65.0 * MM_PT)
        )));
        assert!(text.contains(&format!(
            "/BleedBox [{} {} {} {}]",
            f(12.0 * MM_PT),
            f(12.0 * MM_PT),
            f(118.0 * MM_PT),
            f(68.0 * MM_PT)
        )));
        // The clip to the bleed, closed before the marks.
        let clip = format!(
            "q {} {} {} {} re W n",
            f(12.0 * MM_PT),
            f(12.0 * MM_PT),
            f(106.0 * MM_PT),
            f(56.0 * MM_PT)
        );
        let at_clip = text.find(&clip).expect("clipped to the bleed");
        let at_mark = text.find("/CSAll CS 1 SCN").expect("registration stroke");
        assert!(at_clip < at_mark);
        assert!(text.contains("/Separation /All /DeviceCMYK"));
        // The mark at x = -8 mm sits 7 mm from the media box's edge.
        assert!(text.contains(&format!("{} ", f(7.0 * MM_PT))));
        // Without marks nothing changes: no clip, the bleed as before.
        let plain = document_to_pdf_with(
            &doc,
            &PdfOptions {
                bleed_mm: 3.0,
                ..Default::default()
            },
        );
        let plain = String::from_utf8_lossy(&plain);
        assert!(plain.contains(&format!(
            "/BleedBox [0 0 {} {}]",
            f(106.0 * MM_PT),
            f(56.0 * MM_PT)
        )));
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

    fn doc_carrying(icc: Vec<u8>) -> Document {
        let mut doc = doc_with_transparent_rect();
        doc.cmyk_profile = Some(tracedraw_core::DocumentProfile::new("carried", icc));
        doc
    }

    fn pdfx(doc: &Document, opts: PdfOptions) -> String {
        String::from_utf8_lossy(&document_to_pdf_with(doc, &opts)).into_owned()
    }

    #[test]
    fn pdfx_output_intent_embeds_the_profile_the_drawing_carries() {
        use tracedraw_core::icc::test_profiles as tp;
        let doc = doc_carrying(tp::lut16_cmyk_profile());
        for standard in [PdfStandard::X1a, PdfStandard::X3, PdfStandard::X4] {
            let text = pdfx(
                &doc,
                PdfOptions {
                    standard,
                    ..Default::default()
                },
            );
            assert!(text.contains("/DestOutputProfile"), "{standard:?}");
            assert!(text.contains("/N 4"), "{standard:?}");
            // The identifier names the embedded profile, not another press.
            assert!(
                text.contains("/OutputConditionIdentifier (Synthetic CMYK)"),
                "{standard:?}"
            );
            assert!(!text.contains("CGATS TR 001"), "{standard:?}");
        }
        // A plain PDF still has no output intent.
        let text = pdfx(&doc, PdfOptions::default());
        assert!(!text.contains("/OutputIntents"));
    }

    #[test]
    fn pdfx_given_profile_and_condition_win_over_the_carried_profile() {
        use tracedraw_core::icc::test_profiles as tp;
        let doc = doc_carrying(tp::lut16_cmyk_profile());
        let given = tp::mba_cmyk_profile();
        let text = pdfx(
            &doc,
            PdfOptions {
                standard: PdfStandard::X1a,
                output_profile: Some(given.clone()),
                ..Default::default()
            },
        );
        // The given profile has no description: the condition is custom.
        assert!(text.contains("/OutputConditionIdentifier (Custom)"));
        assert!(text.contains(&format!("<< /N 4 /Length {} >>", given.len())));
        assert!(!text.contains("(Synthetic CMYK)"));
        let text = pdfx(
            &doc,
            PdfOptions {
                standard: PdfStandard::X4,
                output_condition: "FOGRA39".into(),
                ..Default::default()
            },
        );
        assert!(text.contains("/OutputConditionIdentifier (FOGRA39)"));
        assert!(text.contains("/DestOutputProfile"));
    }

    #[test]
    fn pdfx_skips_a_carried_profile_that_is_not_cmyk_or_not_readable() {
        // An RGB profile in the CMYK slot is not an output intent for CMYK
        // values: the registered name stays.
        let mut rgb = vec![0u8; 128];
        rgb[16..20].copy_from_slice(b"RGB ");
        let text = pdfx(
            &doc_carrying(rgb),
            PdfOptions {
                standard: PdfStandard::X1a,
                ..Default::default()
            },
        );
        assert!(!text.contains("/DestOutputProfile"));
        assert!(text.contains("/OutputConditionIdentifier (CGATS TR 001)"));
        // A CMYK signature on a profile that does not parse: embedded as
        // carried, named "Custom", no panic.
        let mut broken = vec![0xFFu8; 64];
        broken[16..20].copy_from_slice(b"CMYK");
        let text = pdfx(
            &doc_carrying(broken),
            PdfOptions {
                standard: PdfStandard::X4,
                ..Default::default()
            },
        );
        assert!(text.contains("/DestOutputProfile"));
        assert!(text.contains("/OutputConditionIdentifier (Custom)"));
        // Truncated before the signature: not used.
        let text = pdfx(
            &doc_carrying(vec![0u8; 10]),
            PdfOptions {
                standard: PdfStandard::X4,
                ..Default::default()
            },
        );
        assert!(!text.contains("/DestOutputProfile"));
    }

    fn two_pages() -> Document {
        let mut doc = doc_with_transparent_rect();
        doc.pages[0].name = "Cover".into();
        let mut ids = doc.ids().clone();
        let mut second = doc.pages[0].clone();
        second.id = ids.page();
        second.name = "\u{65E5}\u{672C}".into();
        doc.set_ids(ids);
        doc.pages.push(second);
        doc
    }

    #[test]
    fn bookmarks_name_every_page_and_can_open_the_panel() {
        let opts = PdfOptions {
            bookmarks: true,
            show_bookmarks: true,
            author: Some("Ighor".into()),
            keywords: Some("poster, test".into()),
            ..Default::default()
        };
        let bytes = document_to_pdf_with(&two_pages(), &opts);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("/Type /Outlines") && text.contains("/Count 2"));
        assert!(text.contains("/Title (Cover)"));
        // Non-Latin names as UTF-16 with a byte order mark.
        assert!(text.contains("/Title <FEFF65E5672C>"));
        assert!(text.contains("/PageMode /UseOutlines"));
        assert!(text.contains("/Author (Ighor)") && text.contains("/Keywords (poster, test)"));
        // An independent reader finds two bookmarks.
        let parsed = lopdf::Document::load_mem(&bytes).expect("parse");
        let outlines = parsed
            .get_outlines(None, None, &mut Default::default())
            .ok()
            .flatten();
        assert_eq!(outlines.map(|o| o.len()).unwrap_or(0), 2);
        // Without the option: no outline.
        let plain = String::from_utf8_lossy(&document_to_pdf(&two_pages())).into_owned();
        assert!(!plain.contains("/Outlines"));
    }

    #[test]
    fn protected_files_open_with_their_passwords_only() {
        let security = crate::pdf_crypt::Security {
            user_password: "open sesame".into(),
            owner_password: "owner".into(),
            permissions: crate::pdf_crypt::Permissions {
                copy: false,
                ..Default::default()
            },
        };
        let opts = PdfOptions {
            security: Some(security.clone()),
            title: Some("Secret plan".into()),
            ..Default::default()
        };
        let bytes = document_to_pdf_with(&two_pages(), &opts);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        assert!(text.starts_with("%PDF-1.7"));
        assert!(text.contains("/Encrypt ") && text.contains("/R 6"));
        assert!(!text.contains("Secret plan"));
        // Another implementation of the standard accepts either password
        // and refuses a wrong one or a swapped role.
        let doc = lopdf::Document::load_mem(&bytes).expect("parse");
        assert!(doc.is_encrypted());
        assert!(doc.authenticate_raw_user_password(b"open sesame").is_ok());
        assert!(doc.authenticate_raw_owner_password(b"owner").is_ok());
        assert!(doc.authenticate_raw_user_password(b"wrong").is_err());
        assert!(doc.authenticate_raw_owner_password(b"open sesame").is_err());
        assert!(doc.clone().decrypt("wrong").is_err());
        // With the owner password it decrypts everything: the document
        // information and the page content.
        let mut d = doc.clone();
        d.decrypt("owner").expect("decrypt");
        let info = d
            .trailer
            .get(b"Info")
            .and_then(lopdf::Object::as_reference)
            .and_then(|id| d.get_dictionary(id))
            .expect("info");
        let title = info
            .get(b"Title")
            .and_then(lopdf::Object::as_str)
            .expect("title");
        assert_eq!(title, b"Secret plan");
        let page = *d.get_pages().values().next().expect("page");
        let content =
            String::from_utf8_lossy(&d.get_page_content(page).expect("content")).into_owned();
        assert!(content.contains(" rg") && content.contains(" l\n"));
        // The user password unwraps the same file key from /UE, and /Perms
        // decrypts with it to the permissions. (That reader's own check of
        // /Perms is not usable, so it is done here as the standard says.)
        let state = lopdf::encryption::EncryptionState::decode(&doc, b"owner").expect("state");
        let key = state.file_encryption_key().to_vec();
        assert_eq!(key.len(), 32);
        let enc = doc.get_encrypted().expect("encrypt dictionary");
        let string = |name: &[u8]| {
            enc.get(name)
                .and_then(lopdf::Object::as_str)
                .expect("entry")
                .to_vec()
        };
        let (u, ue, perms) = (string(b"U"), string(b"UE"), string(b"Perms"));
        let wrap = crate::pdf_crypt::hash_r6(b"open sesame", &u[40..48], &[]);
        let mut unwrapped = ue.clone();
        {
            use aes::cipher::{BlockDecryptMut, KeyIvInit};
            let mut dec =
                cbc::Decryptor::<aes::Aes256>::new_from_slices(&wrap, &[0u8; 16]).expect("cbc");
            for block in unwrapped.chunks_exact_mut(16) {
                dec.decrypt_block_mut(aes::cipher::generic_array::GenericArray::from_mut_slice(
                    block,
                ));
            }
        }
        assert_eq!(unwrapped, key);
        let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(&perms);
        {
            use aes::cipher::{BlockDecrypt, KeyInit};
            aes::Aes256::new_from_slice(&key)
                .expect("aes")
                .decrypt_block(&mut block);
        }
        let p = security.permissions.p_value();
        assert_eq!(&block[..4], &p.to_le_bytes());
        assert_eq!(&block[8..12], b"Tadb");
        assert!(!state
            .permissions()
            .contains(lopdf::encryption::Permissions::COPYABLE));
        assert!(state
            .permissions()
            .contains(lopdf::encryption::Permissions::PRINTABLE));
        // PDF/X files are never protected.
        let opts = PdfOptions {
            security: Some(security),
            standard: PdfStandard::X4,
            ..Default::default()
        };
        let text = String::from_utf8_lossy(&document_to_pdf_with(&two_pages(), &opts)).into_owned();
        assert!(!text.contains("/Encrypt"));
    }

    #[test]
    fn document_properties_fill_the_information_and_the_xmp() {
        let mut doc = two_pages();
        doc.metadata.author = "Ana".into();
        doc.metadata.subject = "Spring sale".into();
        doc.metadata.keywords = "sale, poster".into();
        doc.metadata.language = "pt-BR".into();
        let text = String::from_utf8_lossy(&document_to_pdf(&doc)).into_owned();
        assert!(text.contains("/Author (Ana)") && text.contains("/Subject (Spring sale)"));
        assert!(text.contains("/Keywords (sale, poster)"));
        assert!(text.contains("/Lang (pt-BR)"));
        // An empty value given in the options leaves the entry out; PDF/X
        // repeats the rest in its metadata.
        let opts = PdfOptions {
            author: Some(String::new()),
            standard: PdfStandard::X4,
            ..Default::default()
        };
        let text = String::from_utf8_lossy(&document_to_pdf_with(&doc, &opts)).into_owned();
        assert!(!text.contains("/Author") && !text.contains("<dc:creator>"));
        assert!(text.contains(
            "<dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">Spring sale</rdf:li>"
        ));
        assert!(text.contains("<pdf:Keywords>sale, poster</pdf:Keywords>"));
    }

    #[test]
    fn linked_objects_become_link_areas() {
        let mut doc = two_pages();
        // The page's red base links to a web address (é percent-encoded),
        // the blue square to the second page.
        doc.pages[0].layers[0].shapes[0].link = Some("https://example.com/caf\u{e9}".into());
        doc.pages[0].layers[0].shapes[1].link = Some("#page=2".into());
        let bytes = document_to_pdf(&doc);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        assert!(
            text.contains("/URI (https://example.com/caf%C3%A9)"),
            "{text}"
        );
        assert!(text.contains("/Rect [0 0 283.4646 141.7323]"));
        // The square: 20 to 60 by 10 to 40 mm.
        assert!(text.contains("/Rect [56.6929 28.3465 170.0787 113.3858]"));
        let parsed = lopdf::Document::load_mem(&bytes).expect("parse");
        let pages: Vec<lopdf::ObjectId> = parsed.get_pages().values().copied().collect();
        let annots = parsed
            .get_dictionary(pages[0])
            .and_then(|p| p.get(b"Annots"))
            .and_then(lopdf::Object::as_array)
            .expect("annotations");
        assert_eq!(annots.len(), 2);
        let dest = annots
            .iter()
            .filter_map(|a| a.as_reference().ok())
            .filter_map(|id| parsed.get_dictionary(id).ok())
            .find_map(|d| {
                d.get(b"Dest")
                    .and_then(lopdf::Object::as_array)
                    .ok()
                    .cloned()
            })
            .expect("page link");
        assert_eq!(dest[0].as_reference().ok(), Some(pages[1]));
        // A page that is not there, an empty link and PDF/X: none.
        doc.pages[0].layers[0].shapes[1].link = Some("#page=9".into());
        doc.pages[0].layers[0].shapes[0].link = Some("  ".into());
        let text = String::from_utf8_lossy(&document_to_pdf(&doc)).into_owned();
        assert!(!text.contains("/Annots"));
        doc.pages[0].layers[0].shapes[0].link = Some("https://example.com".into());
        let opts = PdfOptions {
            standard: PdfStandard::X4,
            ..Default::default()
        };
        let text = String::from_utf8_lossy(&document_to_pdf_with(&doc, &opts)).into_owned();
        assert!(!text.contains("/Annots"));
        // The bleed moves the area with the page.
        let opts = PdfOptions {
            bleed_mm: 3.0,
            ..Default::default()
        };
        let text = String::from_utf8_lossy(&document_to_pdf_with(&doc, &opts)).into_owned();
        assert!(
            text.contains("/Rect [8.5039 8.5039 291.9685 150.2362]"),
            "{text}"
        );
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

/// Text written as PDF text, with a stand-in text engine that lays out
/// one glyph per character of a test font on one line.
#[cfg(test)]
mod text_tests {
    use super::*;
    use std::sync::{Arc, OnceLock};
    use tracedraw_core::document::text_outline::{
        self, GlyphFont, GlyphLayout, PlacedGlyph, TextRequest,
    };
    use tracedraw_core::geometry::{BezPath, Point};
    use tracedraw_core::{Stroke, TextSpan};

    const QUICKSAND: &[u8] = include_bytes!("../../text/testdata/Quicksand-VF.ttf");

    /// The test font, and a copy whose licence forbids embedding.
    fn font(family: &str) -> GlyphFont {
        static PLAIN: OnceLock<Arc<Vec<u8>>> = OnceLock::new();
        static LOCKED: OnceLock<Arc<Vec<u8>>> = OnceLock::new();
        let data = if family == "Locked Sans" {
            LOCKED
                .get_or_init(|| Arc::new(with_fs_type(QUICKSAND, 0x0002)))
                .clone()
        } else {
            PLAIN.get_or_init(|| Arc::new(QUICKSAND.to_vec())).clone()
        };
        GlyphFont {
            data,
            index: 0,
            coords: Vec::new(),
        }
    }

    /// `data` with the OS/2 embedding bits set to `fs_type`.
    fn with_fs_type(data: &[u8], fs_type: u16) -> Vec<u8> {
        let mut out = data.to_vec();
        let n = u16::from_be_bytes([data[4], data[5]]) as usize;
        for i in 0..n {
            let rec = 12 + 16 * i;
            if &data[rec..rec + 4] == b"OS/2" {
                let off = u32::from_be_bytes([
                    data[rec + 8],
                    data[rec + 9],
                    data[rec + 10],
                    data[rec + 11],
                ]) as usize;
                out[off + 8..off + 10].copy_from_slice(&fs_type.to_be_bytes());
            }
        }
        out
    }

    struct Sink(BezPath);
    impl ttf_parser::OutlineBuilder for Sink {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.move_to((x as f64, y as f64));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.line_to((x as f64, y as f64));
        }
        fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
            self.0.quad_to((x1 as f64, y1 as f64), (x as f64, y as f64));
        }
        fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
            self.0.curve_to(
                (x1 as f64, y1 as f64),
                (x2 as f64, y2 as f64),
                (x as f64, y as f64),
            );
        }
        fn close(&mut self) {
            self.0.close_path();
        }
    }

    /// One line, one glyph per character; bold spans count as synthetic
    /// bold (not the font's own glyphs).
    fn glyphs(req: &TextRequest) -> GlyphLayout {
        let mut out = GlyphLayout::default();
        let mut pen = 0.0;
        for (si, span) in req.spans.iter().enumerate() {
            let gf = font(&span.font_family);
            let face = ttf_parser::Face::parse(&gf.data, 0).expect("test font");
            let upem = face.units_per_em() as f64;
            let size = span.size_pt * 25.4 / 72.0;
            for c in span.text.chars() {
                let gid = face.glyph_index(c).map(|g| g.0).unwrap_or(0);
                let t = Affine::translate((pen, 0.0)) * Affine::scale(size);
                let mut sink = Sink(BezPath::new());
                face.outline_glyph(ttf_parser::GlyphId(gid), &mut sink);
                out.glyphs.push(PlacedGlyph {
                    font: gf.clone(),
                    glyph_id: gid,
                    transform: t,
                    text: c.to_string(),
                    span: si,
                    path: t * Affine::scale(1.0 / upem) * sink.0,
                    exact: !span.bold,
                });
                pen += face
                    .glyph_hor_advance(ttf_parser::GlyphId(gid))
                    .unwrap_or(0) as f64
                    / upem
                    * size;
            }
            if span.underline {
                out.rest.move_to((0.0, -1.0));
                out.rest.line_to((pen, -1.0));
                out.rest.line_to((pen, -1.3));
                out.rest.close_path();
            }
        }
        out
    }

    fn doc_with(spans: Vec<TextSpan>, fill: Fill, stroke: Option<Stroke>) -> Document {
        text_outline::set_glyphs(glyphs);
        let mut doc = Document::new("t", tracedraw_core::Size::new(100.0, 50.0));
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Text {
                spans,
                origin: Point::new(10.0, 20.0),
                frame: None,
                align: tracedraw_core::TextAlign::Left,
                para: Default::default(),
                on_path: None,
            },
        );
        s.fill = fill;
        s.stroke = stroke;
        doc.layer_mut(layer).unwrap().shapes.push(s);
        doc
    }

    fn black() -> Fill {
        Fill::Solid(Color::BLACK)
    }

    /// The file as text with every zlib stream inflated after it.
    fn inflated(bytes: &[u8]) -> String {
        let mut text = String::from_utf8_lossy(bytes).to_string();
        let mut rest: &[u8] = bytes;
        while let Some(i) = rest.windows(7).position(|w| w == b"stream\n") {
            let data = &rest[i + 7..];
            use std::io::Read;
            let mut out = Vec::new();
            if flate2::read::ZlibDecoder::new(data)
                .read_to_end(&mut out)
                .is_ok()
            {
                text.push_str(&String::from_utf8_lossy(&out));
            }
            rest = data;
        }
        text
    }

    fn extract(bytes: &[u8]) -> String {
        let doc = lopdf::Document::load_mem(bytes).expect("parses");
        doc.extract_text(&[1]).expect("text")
    }

    /// A PDF's text read back as curves from the font it embeds (#386):
    /// filled outlines where the letters were, no text left; read as text
    /// it stays text.
    #[test]
    fn embedded_text_reads_back_as_curves() {
        use tracedraw_core::geometry::Shape as _;
        let doc = doc_with(
            vec![TextSpan::new("Hello PDF", "Quicksand", 24.0)],
            black(),
            None,
        );
        let (bytes, _) = document_to_pdf_report(&doc, &PdfOptions::default());
        let mut ids = tracedraw_core::id::IdSource::default();
        let as_text = crate::pdf_import::parse(&bytes, &mut ids).expect("reads");
        assert!(as_text.pages[0]
            .shapes
            .iter()
            .any(|s| matches!(s.kind, tracedraw_core::ShapeKind::Text { .. })));
        let curves = crate::pdf_import::parse_with_options(
            &bytes,
            &mut ids,
            "",
            crate::pdf_import::ImportOptions {
                text_as_curves: true,
            },
        )
        .expect("reads");
        let shapes = &curves.pages[0].shapes;
        assert!(!shapes
            .iter()
            .any(|s| matches!(s.kind, tracedraw_core::ShapeKind::Text { .. })));
        let paths: Vec<_> = shapes
            .iter()
            .filter(|s| matches!(s.kind, tracedraw_core::ShapeKind::Path { .. }))
            .collect();
        assert!(!paths.is_empty());
        // The letters stand about as high as 24 pt capitals (8.5 mm em).
        let b = paths
            .iter()
            .map(|p| p.bounds())
            .reduce(|a, c| a.union(c))
            .expect("bounds");
        assert!(b.height() > 4.0 && b.height() < 9.0, "{b:?}");
        assert!(b.width() > 20.0, "{b:?}");
        // Where the text was in the file read as text.
        let t = as_text.pages[0].shapes[0].bounds();
        assert!((b.x0 - t.x0).abs() < 3.0, "{b:?} {t:?}");
    }

    #[test]
    fn text_goes_in_as_text_with_an_embedded_subset_font() {
        let doc = doc_with(
            vec![TextSpan::new("Hello PDF", "Quicksand", 24.0)],
            black(),
            None,
        );
        let (bytes, report) = document_to_pdf_report(&doc, &PdfOptions::default());
        let text = inflated(&bytes);
        assert!(text.contains("/Subtype /Type0"), "a font object");
        assert!(text.contains("/Encoding /Identity-H"));
        assert!(text.contains("/CIDFontType2"));
        assert!(text.contains("/FontFile2"));
        assert!(text.contains("/ToUnicode"));
        assert!(text.contains("beginbfchar"));
        assert!(text.contains("BT\n0 Tr"), "filled text");
        assert!(text.contains("/Font << /F0 "));
        // A subset: tagged name, and much smaller than the font.
        assert!(
            text.contains("+Quicksand"),
            "{}",
            &text[..2000.min(text.len())]
        );
        assert!(bytes.len() < QUICKSAND.len() / 2, "{} bytes", bytes.len());
        assert_eq!(report.embedded.len(), 1);
        assert!(report.restricted.is_empty());
        // The string comes back out of the file.
        let out = extract(&bytes);
        assert!(out.contains("Hello PDF"), "{out:?}");
        // The file opens again in our own reader.
        let mut ids = tracedraw_core::id::IdSource::default();
        assert!(crate::pdf_import::parse(&bytes, &mut ids).is_ok());
    }

    #[test]
    fn text_as_curves_keeps_the_font_curves() {
        // A round letter as curves: the glyph's own quadratic segments,
        // each raised to one cubic, not a run of short lines. (The
        // letter's outline as a curve object: what text as curves
        // writes, without registering an outliner for the other tests.)
        let spans = vec![TextSpan::new("o", "Quicksand", 72.0)];
        let l = glyphs(&TextRequest {
            spans: &spans,
            frame: None,
            align: tracedraw_core::TextAlign::Left,
            para: &Default::default(),
            on_path: None,
        });
        let mut doc = doc_with(spans.clone(), black(), None);
        let s = &mut doc.pages[0].layers[0].shapes[0];
        s.kind = ShapeKind::Path {
            path: l.glyphs[0].path.clone(),
            closed: true,
        };
        let opts = PdfOptions::default();
        let text = inflated(&document_to_pdf_with(&doc, &opts));
        let curves = text.lines().filter(|l| l.ends_with(" c")).count();
        let lines = text.lines().filter(|l| l.ends_with(" l")).count();
        assert!(curves >= 8, "{curves} curves");
        assert!(lines <= curves, "{lines} lines, {curves} curves");
    }

    #[test]
    fn quadratic_segments_become_the_same_cubic() {
        let mut pdf = Pdf {
            objects: Vec::new(),
            spot_spaces: Default::default(),
            cmyk_only: false,
            allow_alpha: true,
            encrypt_id: None,
            fonts: Vec::new(),
            bad_fonts: Vec::new(),
            report: PdfReport::default(),
            images: PdfImages::default(),
        };
        let mut w = PageWriter {
            content: String::new(),
            pdf: &mut pdf,
            shadings: Vec::new(),
            gstates: Vec::new(),
            images: Vec::new(),
            symbols: &[],
            inks: &[],
            spaces: Vec::new(),
            fonts: Vec::new(),
            text: None,
            media: Rect::new(0.0, 0.0, 210.0, 297.0),
        };
        let k = 25.4 / 72.0;
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.quad_to((3.0 * k, 3.0 * k), (6.0 * k, 0.0));
        p.close_path();
        p.move_to((10.0 * k, 0.0));
        p.quad_to((13.0 * k, 3.0 * k), (16.0 * k, 0.0));
        w.path_ops(&p);
        assert!(w.content.contains("2 2 4 2 6 0 c"), "{}", w.content);
        assert!(w.content.contains("12 2 14 2 16 0 c"), "{}", w.content);
    }

    #[test]
    fn text_as_curves_writes_no_fonts() {
        let doc = doc_with(
            vec![TextSpan::new("Curves", "Quicksand", 24.0)],
            black(),
            None,
        );
        let opts = PdfOptions {
            text: PdfText {
                as_curves: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let (bytes, report) = document_to_pdf_report(&doc, &opts);
        let text = inflated(&bytes);
        assert!(!text.contains("/Font"));
        assert!(!text.contains("BT\n"));
        assert!(report.embedded.is_empty());
    }

    #[test]
    fn a_font_that_may_not_be_embedded_goes_as_curves() {
        let doc = doc_with(
            vec![
                TextSpan::new("Open ", "Quicksand", 24.0),
                TextSpan::new("Locked", "Locked Sans", 24.0),
            ],
            black(),
            None,
        );
        let (bytes, report) = document_to_pdf_report(&doc, &PdfOptions::default());
        assert_eq!(report.restricted, vec!["Locked Sans".to_string()]);
        assert_eq!(report.embedded.len(), 1, "only the open font");
        let out = extract(&bytes);
        assert!(out.contains("Open"), "{out:?}");
        assert!(!out.contains("Locked"), "{out:?}");
        // Only restricted text: no font at all, the outlines are filled.
        let doc = doc_with(
            vec![TextSpan::new("Locked", "Locked Sans", 24.0)],
            black(),
            None,
        );
        let (bytes, _) = document_to_pdf_report(&doc, &PdfOptions::default());
        let text = inflated(&bytes);
        assert!(!text.contains("/Font"));
        assert!(text.contains("\nf\n"));
    }

    #[test]
    fn outlines_fills_and_synthetic_bold_pick_the_render_mode() {
        let stroke = || {
            Some(Stroke {
                width: 0.3,
                ..Stroke::default()
            })
        };
        // Fill and outline: render mode 2, the stroke set before the text.
        let doc = doc_with(
            vec![TextSpan::new("Ab", "Quicksand", 24.0)],
            black(),
            stroke(),
        );
        let text = inflated(&document_to_pdf(&doc));
        assert!(text.contains("BT\n2 Tr"), "{text}");
        // Outline only: mode 1.
        let doc = doc_with(
            vec![TextSpan::new("Ab", "Quicksand", 24.0)],
            Fill::None,
            stroke(),
        );
        let text = inflated(&document_to_pdf(&doc));
        assert!(text.contains("BT\n1 Tr"));
        // A gradient fill is painted on the outlines; the text goes over
        // it invisibly and is still found.
        let ft =
            tracedraw_core::Fountain::two(FountainKind::Linear, Color::BLACK, Color::WHITE, 0.0);
        let doc = doc_with(
            vec![TextSpan::new("Ab", "Quicksand", 24.0)],
            Fill::Fountain(ft),
            None,
        );
        let bytes = document_to_pdf(&doc);
        let text = inflated(&bytes);
        assert!(text.contains("/ShadingType 2"));
        assert!(text.contains("BT\n3 Tr"));
        assert!(extract(&bytes).contains("Ab"));
        // Synthetic bold: curves, and invisible text over them.
        let mut bold = TextSpan::new("Bold", "Quicksand", 24.0);
        bold.bold = true;
        let doc = doc_with(vec![bold], black(), None);
        let bytes = document_to_pdf(&doc);
        let text = inflated(&bytes);
        assert!(text.contains("BT\n3 Tr") && text.contains("\nf\n"));
        assert!(extract(&bytes).contains("Bold"));
    }

    #[test]
    fn turned_text_and_cmyk_colours_keep_their_matrix_and_colour() {
        let mut doc = doc_with(
            vec![TextSpan::new("Turn", "Quicksand", 10.0)],
            Fill::Solid(Color::Cmyk {
                c: 0.0,
                m: 1.0,
                y: 0.0,
                k: 0.0,
            }),
            None,
        );
        let s = &mut doc.pages[0].layers[0].shapes[0];
        s.transform = Affine::rotate(std::f64::consts::FRAC_PI_2);
        let text = inflated(&document_to_pdf(&doc));
        assert!(text.contains("0 1 0 0 k"), "{text}");
        // A quarter turn: the matrix maps x to y.
        let size = 10.0;
        let tm = text
            .lines()
            .find(|l| l.ends_with(" Tm"))
            .expect("a text matrix");
        let v: Vec<f64> = tm
            .split_whitespace()
            .take(4)
            .map(|x| x.parse().unwrap())
            .collect();
        assert!(v[0].abs() < 1e-6 && (v[1] - size).abs() < 1e-3, "{tm}");
        assert!((v[2] + size).abs() < 1e-3 && v[3].abs() < 1e-6, "{tm}");
        // One run for the whole word: one matrix, one TJ.
        assert_eq!(text.matches(" Tm\n").count(), 1);
    }

    #[test]
    fn named_fonts_and_standard_fonts_stay_out_of_the_file() {
        // Fonts not embedded: a simple font named only, in WinAnsi.
        let doc = doc_with(
            vec![TextSpan::new("Caf\u{e9}", "Quicksand", 24.0)],
            black(),
            None,
        );
        let opts = PdfOptions {
            text: PdfText {
                embed_fonts: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let (bytes, report) = document_to_pdf_report(&doc, &opts);
        let text = inflated(&bytes);
        assert!(text.contains("/Subtype /TrueType") && text.contains("/WinAnsiEncoding"));
        assert!(!text.contains("/FontFile"));
        assert_eq!(report.named.len(), 1);
        assert!(extract(&bytes).contains("Caf\u{e9}"));
        // PDF/X wants fonts in the file whatever the choice.
        let x = PdfOptions {
            standard: PdfStandard::X4,
            ..opts.clone()
        };
        let text = inflated(&document_to_pdf_with(&doc, &x));
        assert!(text.contains("/FontFile2"));
        // A standard family is named, not embedded, unless asked.
        let doc = doc_with(vec![TextSpan::new("Std", "Helvetica", 24.0)], black(), None);
        let text = inflated(&document_to_pdf(&doc));
        assert!(text.contains("/BaseFont /Helvetica "), "{text}");
        assert!(!text.contains("/FontFile"));
        let opts = PdfOptions {
            text: PdfText {
                embed_base14: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let text = inflated(&document_to_pdf_with(&doc, &opts));
        assert!(text.contains("/FontFile2"));
    }

    #[test]
    fn subsetting_and_the_threshold_decide_how_much_goes_in() {
        let doc = doc_with(vec![TextSpan::new("abc", "Quicksand", 24.0)], black(), None);
        let small = document_to_pdf(&doc);
        let whole = document_to_pdf_with(
            &doc,
            &PdfOptions {
                text: PdfText {
                    subset: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert!(
            whole.len() > small.len() * 3,
            "{} vs {}",
            whole.len(),
            small.len()
        );
        // A threshold of 0 %: always the whole font, without a subset tag.
        let all = document_to_pdf_with(
            &doc,
            &PdfOptions {
                text: PdfText {
                    subset_below_pct: 0.0,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert!(!inflated(&all).contains("+Quicksand"));
        assert!(extract(&all).contains("abc"));
        // Underlines and the like stay curves.
        let mut u = TextSpan::new("u", "Quicksand", 24.0);
        u.underline = true;
        let text = inflated(&document_to_pdf(&doc_with(vec![u], black(), None)));
        assert!(text.contains("BT\n0 Tr") && text.contains("\nf\n"));
    }

    #[test]
    fn protected_files_keep_their_text() {
        let doc = doc_with(
            vec![TextSpan::new("Secret", "Quicksand", 24.0)],
            black(),
            None,
        );
        let opts = PdfOptions {
            security: Some(crate::pdf_crypt::Security {
                user_password: String::new(),
                owner_password: "owner".into(),
                permissions: Default::default(),
            }),
            ..Default::default()
        };
        let bytes = document_to_pdf_with(&doc, &opts);
        let mut ids = tracedraw_core::id::IdSource::default();
        let imp = crate::pdf_import::parse(&bytes, &mut ids).expect("opens");
        assert_eq!(imp.pages.len(), 1);
    }
}
