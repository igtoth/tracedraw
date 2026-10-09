//! PDF (and PDF-compatible AI) import.
//!
//! The content stream of every page is interpreted into TraceDraw objects:
//! paths with fills and outlines, images, text runs, form XObjects, axial
//! and radial shadings as fountain fills, constant alpha from ExtGStates,
//! and clipping paths as ClipFrames. The object parsing (xref, streams,
//! filters) is `lopdf`'s; everything after the operator list is here.
//!
//! Units: a PDF user unit is a point; the page's media box bottom-left
//! corner becomes our origin. Every result is best effort: anything the
//! importer does not understand is skipped with a warning, never an error.

use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document as PdfDoc, Object, Stream};
use std::collections::HashMap;
use tracedraw_core::{
    document::{ParagraphStyle, Shape, ShapeKind, TextSpan},
    geometry::{Affine, BezPath, Point, Rect, Shape as _, Size, Vec2},
    id::IdSource,
    style::{Fountain, FountainKind, LineCap, LineJoin, Stop},
    Color, Fill, ShapeId, Stroke, TextAlign,
};

const PT_MM: f64 = 25.4 / 72.0;
const MAX_FORM_DEPTH: usize = 12;
const MAX_OPS: usize = 2_000_000;

/// One imported page.
#[derive(Debug, Clone)]
pub struct ImportedPage {
    pub size: Size,
    pub shapes: Vec<Shape>,
}

#[derive(Debug, Clone, Default)]
pub struct ImportedPdf {
    pub pages: Vec<ImportedPage>,
    pub warnings: Vec<String>,
    pub title: Option<String>,
}

/// Does the buffer hold a PDF, possibly after an AI/PostScript preamble?
pub fn is_pdf(bytes: &[u8]) -> bool {
    pdf_start(bytes).is_some()
}

fn pdf_start(bytes: &[u8]) -> Option<usize> {
    let limit = bytes.len().min(1 << 20);
    bytes[..limit].windows(5).position(|w| w == b"%PDF-")
}

/// Parse a PDF (or an AI file with embedded PDF) into pages of shapes.
pub fn parse(bytes: &[u8], ids: &mut IdSource) -> Result<ImportedPdf, String> {
    let start = pdf_start(bytes).ok_or_else(|| "not a PDF file".to_string())?;
    let mut doc = PdfDoc::load_mem(&bytes[start..]).map_err(|e| format!("PDF: {e}"))?;
    if doc.is_encrypted() {
        if let Err(e) = doc.decrypt("") {
            return Err(format!("PDF is encrypted: {e}"));
        }
    }
    let mut out = ImportedPdf::default();
    out.title = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|o| doc.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok())
        .and_then(|d| d.get(b"Title").ok())
        .and_then(|t| t.as_str().ok())
        .map(pdf_string)
        .filter(|s| !s.trim().is_empty());
    let pages: Vec<lopdf::ObjectId> = doc.page_iter().collect();
    if pages.is_empty() {
        return Err("PDF has no pages".into());
    }
    for (index, page_id) in pages.iter().enumerate() {
        let mut imp = Importer {
            doc: &doc,
            ids,
            warnings: Vec::new(),
            shapes: Vec::new(),
            fonts: HashMap::new(),
            ops: 0,
        };
        let page = imp.import_page(*page_id, index);
        out.warnings.extend(imp.warnings);
        out.pages.push(page);
    }
    out.warnings.dedup();
    Ok(out)
}

/// Convert an imported PDF into a document (one page per PDF page).
pub fn to_document(imp: ImportedPdf, title: &str) -> tracedraw_core::Document {
    use tracedraw_core::document::{Layer, Page};
    let first = imp
        .pages
        .first()
        .map(|p| p.size)
        .unwrap_or(tracedraw_core::document::paper::A4);
    let mut doc = tracedraw_core::Document::new(imp.title.as_deref().unwrap_or(title), first);
    doc.pages.clear();
    for (i, p) in imp.pages.into_iter().enumerate() {
        let pid = doc.ids_mut().page();
        let lid = doc.ids_mut().layer();
        let mut layer = Layer::new(lid, "Layer 1");
        layer.shapes = p.shapes;
        doc.pages.push(Page {
            id: pid,
            name: format!("Page {}", i + 1),
            size: p.size,
            layers: vec![layer],
            guides: Vec::new(),
            background: None,
        });
    }
    doc
}

/// Text string (PDFDocEncoding or UTF-16BE with BOM).
fn pdf_string(b: &[u8]) -> String {
    if b.len() >= 2 && b[0] == 0xfe && b[1] == 0xff {
        let units: Vec<u16> = b[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        b.iter().map(|&c| c as char).collect()
    }
}

#[derive(Clone)]
struct GState {
    ctm: Affine,
    fill: Option<Color>,
    stroke: Option<Color>,
    fill_pattern: Option<Fill>,
    fill_cs: ColorSpace,
    stroke_cs: ColorSpace,
    line_width: f64,
    cap: LineCap,
    join: LineJoin,
    dash: Vec<f64>,
    fill_alpha: f64,
    stroke_alpha: f64,
    clip: Option<BezPath>,
    // Text state.
    font: Option<std::rc::Rc<FontInfo>>,
    font_size: f64,
    char_spacing: f64,
    word_spacing: f64,
    hscale: f64,
    leading: f64,
    rise: f64,
    render_mode: i64,
}

impl Default for GState {
    fn default() -> Self {
        GState {
            ctm: Affine::IDENTITY,
            fill: Some(Color::BLACK),
            stroke: Some(Color::BLACK),
            fill_pattern: None,
            fill_cs: ColorSpace::Gray,
            stroke_cs: ColorSpace::Gray,
            line_width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash: Vec::new(),
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            clip: None,
            font: None,
            font_size: 0.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            hscale: 1.0,
            leading: 0.0,
            rise: 0.0,
            render_mode: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ColorSpace {
    Gray,
    Rgb,
    Cmyk,
    /// Component count only (ICCBased, Lab, Separation...).
    N(usize),
    Pattern,
}

impl ColorSpace {
    fn components(&self) -> usize {
        match self {
            ColorSpace::Gray => 1,
            ColorSpace::Rgb => 3,
            ColorSpace::Cmyk => 4,
            ColorSpace::N(n) => *n,
            ColorSpace::Pattern => 0,
        }
    }
}

struct FontInfo {
    family: String,
    bold: bool,
    italic: bool,
    /// Two-byte codes (Type0).
    two_byte: bool,
    to_unicode: Option<HashMap<u32, String>>,
    one_byte: Option<[Option<char>; 256]>,
    widths: HashMap<u32, f64>,
    default_width: f64,
    type3_matrix: Option<Affine>,
}

impl FontInfo {
    fn decode(&self, bytes: &[u8]) -> Vec<(u32, String, bool)> {
        // (code, text, is_single_byte_space)
        let mut out = Vec::new();
        if self.two_byte {
            for c in bytes.chunks(2) {
                let code = if c.len() == 2 {
                    u16::from_be_bytes([c[0], c[1]]) as u32
                } else {
                    c[0] as u32
                };
                let s = self
                    .to_unicode
                    .as_ref()
                    .and_then(|m| m.get(&code).cloned())
                    .unwrap_or_default();
                out.push((code, s, false));
            }
        } else {
            for &b in bytes {
                let code = b as u32;
                let s = self
                    .to_unicode
                    .as_ref()
                    .and_then(|m| m.get(&code).cloned())
                    .or_else(|| {
                        self.one_byte
                            .as_ref()
                            .and_then(|t| t[b as usize])
                            .map(|c| c.to_string())
                    })
                    .unwrap_or_else(|| (b as char).to_string());
                out.push((code, s, b == 32));
            }
        }
        out
    }

    /// Glyph advance in text space units (1/1000 em).
    fn width(&self, code: u32) -> f64 {
        self.widths
            .get(&code)
            .copied()
            .unwrap_or(self.default_width)
    }
}

struct Importer<'a> {
    doc: &'a PdfDoc,
    ids: &'a mut IdSource,
    warnings: Vec<String>,
    shapes: Vec<Shape>,
    fonts: HashMap<(u32, u16), std::rc::Rc<FontInfo>>,
    ops: usize,
}

/// A run of text being assembled while inside BT ... ET.
struct TextRun {
    text: String,
    font: std::rc::Rc<FontInfo>,
    size_pt: f64,
    /// Device-space matrix of the run's origin (without the font size).
    matrix: Affine,
    /// Advance along the baseline in text space, for appending.
    end_x: f64,
    fill: Option<Color>,
    alpha: f64,
    clip: Option<BezPath>,
}

impl<'a> Importer<'a> {
    fn warn(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        if self.warnings.len() < 100 && !self.warnings.contains(&msg) {
            self.warnings.push(msg);
        }
    }

    fn deref<'b>(&self, o: &'b Object) -> &'b Object
    where
        'a: 'b,
    {
        match o {
            Object::Reference(id) => self.doc.get_object(*id).unwrap_or(&Object::Null),
            other => other,
        }
    }

    fn dict_of<'b>(&self, o: &'b Object) -> Option<&'b Dictionary>
    where
        'a: 'b,
    {
        match self.deref(o) {
            Object::Dictionary(d) => Some(d),
            Object::Stream(s) => Some(&s.dict),
            _ => None,
        }
    }

    fn get<'b>(&self, d: &'b Dictionary, key: &[u8]) -> Option<&'b Object>
    where
        'a: 'b,
    {
        d.get(key).ok().map(|o| self.deref(o))
    }

    fn num(o: &Object) -> Option<f64> {
        match o {
            Object::Integer(i) => Some(*i as f64),
            Object::Real(r) => Some(*r as f64),
            _ => None,
        }
    }

    fn nums(&self, o: &Object) -> Vec<f64> {
        match self.deref(o) {
            Object::Array(a) => a.iter().filter_map(|x| Self::num(self.deref(x))).collect(),
            other => Self::num(other).into_iter().collect(),
        }
    }

    fn rect_of(&self, o: &Object) -> Option<Rect> {
        let v = self.nums(o);
        if v.len() != 4 || v.iter().any(|x| !x.is_finite()) {
            return None;
        }
        Some(Rect::new(
            v[0].min(v[2]),
            v[1].min(v[3]),
            v[0].max(v[2]),
            v[1].max(v[3]),
        ))
    }

    fn matrix_of(&self, o: &Object) -> Option<Affine> {
        let v = self.nums(o);
        if v.len() != 6 || v.iter().any(|x| !x.is_finite()) {
            return None;
        }
        Some(Affine::new([v[0], v[1], v[2], v[3], v[4], v[5]]))
    }

    /// Inherited page attribute (MediaBox, Resources, Rotate).
    fn page_attr<'b>(&self, page: &'b Dictionary, key: &[u8]) -> Option<&'b Object>
    where
        'a: 'b,
    {
        let mut d = page;
        for _ in 0..32 {
            if let Some(o) = self.get(d, key) {
                return Some(o);
            }
            let parent = self.get(d, b"Parent")?;
            d = self.dict_of(parent)?;
        }
        None
    }

    fn import_page(&mut self, page_id: lopdf::ObjectId, index: usize) -> ImportedPage {
        let Ok(page) = self.doc.get_dictionary(page_id) else {
            self.warn(format!("page {} is not a dictionary", index + 1));
            return ImportedPage {
                size: tracedraw_core::document::paper::A4,
                shapes: Vec::new(),
            };
        };
        let media = self
            .page_attr(page, b"MediaBox")
            .and_then(|o| self.rect_of(o))
            .filter(|r| r.width() > 1.0 && r.height() > 1.0 && r.width() < 200_000.0)
            .unwrap_or(Rect::new(0.0, 0.0, 612.0, 792.0));
        let crop = self
            .page_attr(page, b"CropBox")
            .and_then(|o| self.rect_of(o))
            .map(|c| c.intersect(media))
            .filter(|r| r.width() > 1.0 && r.height() > 1.0)
            .unwrap_or(media);
        let rotate = self
            .page_attr(page, b"Rotate")
            .and_then(Self::num)
            .map(|r| (r.round() as i64).rem_euclid(360))
            .unwrap_or(0);
        let (w_pt, h_pt) = if rotate == 90 || rotate == 270 {
            (crop.height(), crop.width())
        } else {
            (crop.width(), crop.height())
        };
        // Base transform: points to mm, crop box origin to (0,0), rotation.
        let to_origin = Affine::translate((-crop.x0, -crop.y0));
        let rot = match rotate {
            90 => {
                Affine::translate((0.0, crop.width()))
                    * Affine::rotate(-std::f64::consts::FRAC_PI_2)
            }
            180 => {
                Affine::translate((crop.width(), crop.height()))
                    * Affine::rotate(std::f64::consts::PI)
            }
            270 => {
                Affine::translate((crop.height(), 0.0))
                    * Affine::rotate(std::f64::consts::FRAC_PI_2)
            }
            _ => Affine::IDENTITY,
        };
        let base = Affine::scale(PT_MM) * rot * to_origin;
        let resources = self
            .page_attr(page, b"Resources")
            .and_then(|o| self.dict_of(o))
            .cloned()
            .unwrap_or_default();
        let content = self.doc.get_page_content(page_id).unwrap_or_default();
        let mut gs = GState {
            ctm: base,
            ..GState::default()
        };
        self.shapes.clear();
        self.run_content(&content, &resources, &mut gs, 0);
        // Annotations' appearance streams (stamps, form fields) are drawn too.
        self.import_annotations(page, base);
        ImportedPage {
            size: Size::new(w_pt * PT_MM, h_pt * PT_MM),
            shapes: std::mem::take(&mut self.shapes),
        }
    }

    fn import_annotations(&mut self, page: &Dictionary, base: Affine) {
        let Some(Object::Array(annots)) = self.get(page, b"Annots").cloned() else {
            return;
        };
        for a in annots.iter().take(500) {
            let Some(ad) = self.dict_of(a).cloned() else {
                continue;
            };
            let subtype = self
                .get(&ad, b"Subtype")
                .and_then(|o| o.as_name().ok())
                .unwrap_or(b"");
            if subtype == b"Link" || subtype == b"Popup" {
                continue;
            }
            let flags = self.get(&ad, b"F").and_then(Self::num).unwrap_or(0.0) as i64;
            if flags & 2 != 0 {
                continue; // hidden
            }
            let Some(rect) = self.get(&ad, b"Rect").and_then(|o| self.rect_of(o)) else {
                continue;
            };
            let Some(ap) = self.get(&ad, b"AP").and_then(|o| self.dict_of(o)) else {
                continue;
            };
            let Some(n) = self.get(ap, b"N") else {
                continue;
            };
            let n = match self.deref(n) {
                Object::Stream(s) => s.clone(),
                Object::Dictionary(states) => {
                    // Appearance sub-dictionary: pick /AS or the first state.
                    let as_name = self
                        .get(&ad, b"AS")
                        .and_then(|o| o.as_name().ok())
                        .map(|v| v.to_vec());
                    let pick = as_name
                        .and_then(|k| states.get(&k).ok())
                        .or_else(|| states.iter().next().map(|(_, v)| v));
                    match pick.map(|o| self.deref(o)) {
                        Some(Object::Stream(s)) => s.clone(),
                        _ => continue,
                    }
                }
                _ => continue,
            };
            // Form bbox mapped into the annotation rectangle (PDF 12.5.5).
            let bbox = self.get(&n.dict, b"BBox").and_then(|o| self.rect_of(o));
            let matrix = self
                .get(&n.dict, b"Matrix")
                .and_then(|o| self.matrix_of(o))
                .unwrap_or(Affine::IDENTITY);
            let a_mat = match bbox {
                Some(b) => {
                    let tb = matrix.transform_rect_bbox(b);
                    let sx = if tb.width() > 1e-9 {
                        rect.width() / tb.width()
                    } else {
                        1.0
                    };
                    let sy = if tb.height() > 1e-9 {
                        rect.height() / tb.height()
                    } else {
                        1.0
                    };
                    Affine::translate((rect.x0, rect.y0))
                        * Affine::scale_non_uniform(sx, sy)
                        * Affine::translate((-tb.x0, -tb.y0))
                }
                None => Affine::IDENTITY,
            };
            let mut gs = GState {
                ctm: base * a_mat,
                ..GState::default()
            };
            self.draw_form(&n, &mut gs, 1);
        }
    }

    // ----- content interpretation -------------------------------------------

    fn run_content(&mut self, data: &[u8], resources: &Dictionary, gs: &mut GState, depth: usize) {
        let content = match Content::decode(data) {
            Ok(c) => c,
            Err(e) => {
                self.warn(format!("content stream not parsed: {e}"));
                return;
            }
        };
        let mut stack: Vec<GState> = Vec::new();
        let mut path = BezPath::new();
        let mut start = Point::ZERO;
        let mut current = Point::ZERO;
        let mut pending_clip: Option<bool> = None; // Some(even_odd)
        let mut text_matrix = Affine::IDENTITY;
        let mut line_matrix = Affine::IDENTITY;
        let mut run: Option<TextRun> = None;
        let mut compat = 0usize;

        for op in content.operations.iter() {
            self.ops += 1;
            if self.ops > MAX_OPS {
                self.warn("content too long; the rest was skipped");
                break;
            }
            let Operation { operator, operands } = op;
            let n = |i: usize| -> f64 { operands.get(i).and_then(Self::num).unwrap_or(0.0) };
            match operator.as_str() {
                "q" => {
                    stack.push(gs.clone());
                    if stack.len() > 256 {
                        stack.remove(0);
                    }
                }
                "Q" => {
                    if let Some(g) = stack.pop() {
                        *gs = g;
                    }
                }
                "cm" => {
                    if operands.len() >= 6 {
                        let m = Affine::new([n(0), n(1), n(2), n(3), n(4), n(5)]);
                        gs.ctm *= m;
                    }
                }
                "w" => gs.line_width = n(0).max(0.0),
                "J" => {
                    gs.cap = match n(0) as i64 {
                        1 => LineCap::Round,
                        2 => LineCap::Square,
                        _ => LineCap::Butt,
                    }
                }
                "j" => {
                    gs.join = match n(0) as i64 {
                        1 => LineJoin::Round,
                        2 => LineJoin::Bevel,
                        _ => LineJoin::Miter,
                    }
                }
                "d" => {
                    gs.dash = operands
                        .first()
                        .map(|a| self.nums(a))
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|v| v.is_finite() && *v >= 0.0)
                        .collect();
                    if gs.dash.iter().all(|v| *v == 0.0) {
                        gs.dash.clear();
                    }
                }
                "gs" => {
                    if let Some(name) = operands.first().and_then(|o| o.as_name().ok()) {
                        self.apply_ext_gstate(resources, name, gs);
                    }
                }
                // Path construction (points are mapped through the CTM now).
                "m" => {
                    if operands.len() >= 2 {
                        current = Point::new(n(0), n(1));
                        start = current;
                        path.move_to(gs.ctm * current);
                    }
                }
                "l" => {
                    if operands.len() >= 2 {
                        current = Point::new(n(0), n(1));
                        path.line_to(gs.ctm * current);
                    }
                }
                "c" => {
                    if operands.len() >= 6 {
                        let p1 = Point::new(n(0), n(1));
                        let p2 = Point::new(n(2), n(3));
                        current = Point::new(n(4), n(5));
                        path.curve_to(gs.ctm * p1, gs.ctm * p2, gs.ctm * current);
                    }
                }
                "v" => {
                    if operands.len() >= 4 {
                        let p1 = current;
                        let p2 = Point::new(n(0), n(1));
                        current = Point::new(n(2), n(3));
                        path.curve_to(gs.ctm * p1, gs.ctm * p2, gs.ctm * current);
                    }
                }
                "y" => {
                    if operands.len() >= 4 {
                        let p1 = Point::new(n(0), n(1));
                        current = Point::new(n(2), n(3));
                        path.curve_to(gs.ctm * p1, gs.ctm * current, gs.ctm * current);
                    }
                }
                "h" => {
                    if !path.elements().is_empty() {
                        path.close_path();
                        current = start;
                    }
                }
                "re" => {
                    if operands.len() >= 4 {
                        let (x, y, w, h) = (n(0), n(1), n(2), n(3));
                        let pts = [
                            Point::new(x, y),
                            Point::new(x + w, y),
                            Point::new(x + w, y + h),
                            Point::new(x, y + h),
                        ];
                        path.move_to(gs.ctm * pts[0]);
                        for p in &pts[1..] {
                            path.line_to(gs.ctm * *p);
                        }
                        path.close_path();
                        current = pts[0];
                        start = pts[0];
                    }
                }
                // Path painting.
                "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" => {
                    let o = operator.as_str();
                    if matches!(o, "s" | "b" | "b*") && !path.elements().is_empty() {
                        path.close_path();
                    }
                    let fill = matches!(o, "f" | "F" | "f*" | "B" | "B*" | "b" | "b*");
                    let stroke = matches!(o, "S" | "s" | "B" | "B*" | "b" | "b*");
                    let even_odd = o.ends_with('*');
                    if fill || stroke {
                        self.paint_path(&path, fill, stroke, even_odd, gs);
                    }
                    if let Some(eo) = pending_clip.take() {
                        let _ = eo;
                        gs.clip = Some(match &gs.clip {
                            Some(old) => intersect_clip(old, &path),
                            None => path.clone(),
                        });
                    }
                    path = BezPath::new();
                }
                "W" => pending_clip = Some(false),
                "W*" => pending_clip = Some(true),
                // Colour.
                "g" | "G" => {
                    let c = Color::Gray {
                        v: n(0).clamp(0.0, 1.0) as f32,
                    };
                    self.set_color(gs, operator == "g", Some(c), ColorSpace::Gray);
                }
                "rg" | "RG" => {
                    let c = Color::Rgb {
                        r: n(0).clamp(0.0, 1.0) as f32,
                        g: n(1).clamp(0.0, 1.0) as f32,
                        b: n(2).clamp(0.0, 1.0) as f32,
                    };
                    self.set_color(gs, operator == "rg", Some(c), ColorSpace::Rgb);
                }
                "k" | "K" => {
                    let c = Color::Cmyk {
                        c: n(0).clamp(0.0, 1.0) as f32,
                        m: n(1).clamp(0.0, 1.0) as f32,
                        y: n(2).clamp(0.0, 1.0) as f32,
                        k: n(3).clamp(0.0, 1.0) as f32,
                    };
                    self.set_color(gs, operator == "k", Some(c), ColorSpace::Cmyk);
                }
                "cs" | "CS" => {
                    let cs = operands
                        .first()
                        .and_then(|o| o.as_name().ok())
                        .map(|name| self.color_space(resources, name))
                        .unwrap_or(ColorSpace::Gray);
                    let initial = match cs {
                        ColorSpace::Cmyk => Color::Cmyk {
                            c: 0.0,
                            m: 0.0,
                            y: 0.0,
                            k: 1.0,
                        },
                        ColorSpace::Pattern => Color::BLACK,
                        _ => Color::BLACK,
                    };
                    if operator == "cs" {
                        gs.fill_cs = cs;
                        gs.fill = Some(initial);
                        gs.fill_pattern = None;
                    } else {
                        gs.stroke_cs = cs;
                        gs.stroke = Some(initial);
                    }
                }
                "sc" | "scn" | "SC" | "SCN" => {
                    let is_fill = operator.starts_with("sc");
                    let cs = if is_fill { &gs.fill_cs } else { &gs.stroke_cs };
                    let vals: Vec<f64> = operands.iter().filter_map(Self::num).collect();
                    if *cs == ColorSpace::Pattern {
                        if let Some(name) = operands.last().and_then(|o| o.as_name().ok()) {
                            let pat = self.pattern_fill(resources, name, gs);
                            if is_fill {
                                gs.fill_pattern = pat.clone();
                                gs.fill = match pat {
                                    Some(Fill::Solid(c)) => Some(c),
                                    Some(_) => Some(Color::Gray { v: 0.5 }),
                                    None => None,
                                };
                            } else {
                                gs.stroke = match pat {
                                    Some(Fill::Solid(c)) => Some(c),
                                    Some(Fill::Fountain(f)) => f.stops.first().map(|s| s.color),
                                    _ => Some(Color::Gray { v: 0.5 }),
                                };
                            }
                        }
                    } else {
                        let c = color_from_components(&vals, cs);
                        if let Some(c) = c {
                            if is_fill {
                                gs.fill = Some(c);
                                gs.fill_pattern = None;
                            } else {
                                gs.stroke = Some(c);
                            }
                        }
                    }
                }
                // XObjects and shadings.
                "Do" => {
                    if let Some(name) = operands.first().and_then(|o| o.as_name().ok()) {
                        self.do_xobject(resources, name, gs, depth);
                    }
                }
                "sh" => {
                    if let Some(name) = operands.first().and_then(|o| o.as_name().ok()) {
                        self.paint_shading(resources, name, gs);
                    }
                }
                "BI" => {
                    // Inline images carry their data in the operands.
                    self.inline_image(operands, gs);
                }
                // Text.
                "BT" => {
                    text_matrix = Affine::IDENTITY;
                    line_matrix = Affine::IDENTITY;
                    run = None;
                }
                "ET" => {
                    if let Some(r) = run.take() {
                        self.flush_run(r);
                    }
                }
                "Tf" => {
                    let name = operands
                        .first()
                        .and_then(|o| o.as_name().ok())
                        .unwrap_or(b"");
                    gs.font_size = n(1);
                    gs.font = self.font(resources, name);
                }
                "Td" => {
                    line_matrix *= Affine::translate((n(0), n(1)));
                    text_matrix = line_matrix;
                }
                "TD" => {
                    gs.leading = -n(1);
                    line_matrix *= Affine::translate((n(0), n(1)));
                    text_matrix = line_matrix;
                }
                "Tm" => {
                    if operands.len() >= 6 {
                        line_matrix = Affine::new([n(0), n(1), n(2), n(3), n(4), n(5)]);
                        text_matrix = line_matrix;
                    }
                }
                "T*" => {
                    line_matrix *= Affine::translate((0.0, -gs.leading));
                    text_matrix = line_matrix;
                }
                "TL" => gs.leading = n(0),
                "Tc" => gs.char_spacing = n(0),
                "Tw" => gs.word_spacing = n(0),
                "Tz" => gs.hscale = n(0) / 100.0,
                "Ts" => gs.rise = n(0),
                "Tr" => gs.render_mode = n(0) as i64,
                "Tj" | "'" | "\"" => {
                    if operator != "Tj" {
                        if operator == "\"" {
                            gs.word_spacing = n(0);
                            gs.char_spacing = n(1);
                        }
                        line_matrix *= Affine::translate((0.0, -gs.leading));
                        text_matrix = line_matrix;
                    }
                    if let Some(Object::String(s, _)) = operands.last() {
                        self.show_text(s, gs, &mut text_matrix, &mut run);
                    }
                }
                "TJ" => {
                    if let Some(Object::Array(items)) = operands.first() {
                        for it in items {
                            match it {
                                Object::String(s, _) => {
                                    self.show_text(s, gs, &mut text_matrix, &mut run)
                                }
                                other => {
                                    if let Some(adj) = Self::num(other) {
                                        let tx = -adj / 1000.0 * gs.font_size * gs.hscale;
                                        text_matrix *= Affine::translate((tx, 0.0));
                                        if let Some(r) = run.as_mut() {
                                            r.end_x += tx;
                                            // A big gap is a space the producer left out.
                                            if adj < -180.0 && !r.text.ends_with(' ') {
                                                r.text.push(' ');
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                "BX" => compat += 1,
                "EX" => compat = compat.saturating_sub(1),
                "d0" | "d1" | "BMC" | "BDC" | "EMC" | "MP" | "DP" | "ri" | "i" | "M" => {}
                other => {
                    if compat == 0 {
                        self.warn(format!("operator {other} not supported"));
                    }
                }
            }
        }
        if let Some(r) = run.take() {
            self.flush_run(r);
        }
    }

    fn set_color(&mut self, gs: &mut GState, fill: bool, c: Option<Color>, cs: ColorSpace) {
        if fill {
            gs.fill = c;
            gs.fill_cs = cs;
            gs.fill_pattern = None;
        } else {
            gs.stroke = c;
            gs.stroke_cs = cs;
        }
    }

    fn color_space(&mut self, resources: &Dictionary, name: &[u8]) -> ColorSpace {
        match name {
            b"DeviceGray" | b"G" | b"CalGray" => return ColorSpace::Gray,
            b"DeviceRGB" | b"RGB" | b"CalRGB" => return ColorSpace::Rgb,
            b"DeviceCMYK" | b"CMYK" => return ColorSpace::Cmyk,
            b"Pattern" => return ColorSpace::Pattern,
            _ => {}
        }
        let cs_dict = self
            .get(resources, b"ColorSpace")
            .and_then(|o| self.dict_of(o));
        let entry = cs_dict.and_then(|d| self.get(d, name)).cloned();
        match entry {
            Some(o) => self.color_space_object(&o, 0),
            None => {
                self.warn(format!(
                    "colour space {} unknown; treated as gray",
                    String::from_utf8_lossy(name)
                ));
                ColorSpace::Gray
            }
        }
    }

    fn color_space_object(&mut self, o: &Object, depth: usize) -> ColorSpace {
        if depth > 4 {
            return ColorSpace::Gray;
        }
        match self.deref(o) {
            Object::Name(n) => match n.as_slice() {
                b"DeviceGray" | b"G" | b"CalGray" => ColorSpace::Gray,
                b"DeviceRGB" | b"RGB" | b"CalRGB" | b"Lab" => ColorSpace::Rgb,
                b"DeviceCMYK" | b"CMYK" => ColorSpace::Cmyk,
                b"Pattern" => ColorSpace::Pattern,
                _ => ColorSpace::Gray,
            },
            Object::Array(a) => {
                let family = a
                    .first()
                    .and_then(|f| self.deref(f).as_name().ok())
                    .unwrap_or(b"");
                match family {
                    b"ICCBased" => {
                        let n = a
                            .get(1)
                            .and_then(|s| self.dict_of(s))
                            .and_then(|d| self.get(d, b"N"))
                            .and_then(Self::num)
                            .unwrap_or(3.0) as usize;
                        match n {
                            1 => ColorSpace::Gray,
                            4 => ColorSpace::Cmyk,
                            _ => ColorSpace::Rgb,
                        }
                    }
                    b"CalRGB" | b"Lab" => ColorSpace::Rgb,
                    b"CalGray" => ColorSpace::Gray,
                    b"Indexed" | b"I" => ColorSpace::N(1),
                    b"Separation" => ColorSpace::N(1),
                    b"DeviceN" => {
                        let n = a
                            .get(1)
                            .and_then(|names| self.deref(names).as_array().ok())
                            .map(|v| v.len())
                            .unwrap_or(1);
                        ColorSpace::N(n)
                    }
                    b"Pattern" => ColorSpace::Pattern,
                    b"DeviceGray" => ColorSpace::Gray,
                    b"DeviceRGB" => ColorSpace::Rgb,
                    b"DeviceCMYK" => ColorSpace::Cmyk,
                    _ => {
                        self.warn(format!(
                            "colour space family {} approximated",
                            String::from_utf8_lossy(family)
                        ));
                        ColorSpace::N(1)
                    }
                }
            }
            _ => ColorSpace::Gray,
        }
    }

    fn apply_ext_gstate(&mut self, resources: &Dictionary, name: &[u8], gs: &mut GState) {
        let Some(eg) = self
            .get(resources, b"ExtGState")
            .and_then(|o| self.dict_of(o))
            .and_then(|d| self.get(d, name))
            .and_then(|o| self.dict_of(o))
            .cloned()
        else {
            return;
        };
        if let Some(v) = self.get(&eg, b"ca").and_then(Self::num) {
            gs.fill_alpha = v.clamp(0.0, 1.0);
        }
        if let Some(v) = self.get(&eg, b"CA").and_then(Self::num) {
            gs.stroke_alpha = v.clamp(0.0, 1.0);
        }
        if let Some(v) = self.get(&eg, b"LW").and_then(Self::num) {
            gs.line_width = v.max(0.0);
        }
        if let Some(Object::Array(font)) = self.get(&eg, b"Font") {
            if let (Some(fd), Some(size)) = (
                font.first().and_then(|f| self.dict_of(f)).cloned(),
                font.get(1).and_then(Self::num),
            ) {
                let key = (0u32, 0u16);
                let _ = key;
                gs.font = Some(std::rc::Rc::new(self.font_info(&fd)));
                gs.font_size = size;
            }
        }
        if let Some(Object::Dictionary(sm)) = self.get(&eg, b"SMask") {
            // Soft masks on groups are not modelled; keep the content opaque
            // and say so.
            if sm.has(b"G") {
                self.warn("soft mask (SMask) in graphics state ignored");
            }
        }
    }

    fn scale_of(m: Affine) -> f64 {
        let c = m.as_coeffs();
        (c[0] * c[3] - c[1] * c[2]).abs().sqrt()
    }

    fn new_shape(&mut self, kind: ShapeKind) -> Shape {
        let id = ShapeId(self.ids.shape().0);
        Shape::new(id, kind)
    }

    fn push(&mut self, mut shape: Shape, clip: &Option<BezPath>) {
        if let Some(clip) = clip {
            let cb = clip.bounding_box();
            let sb = shape.bounds();
            if cb.width() <= 0.0 || cb.height() <= 0.0 {
                return; // everything clipped away
            }
            let inside = cb.x0 <= sb.x0 + 1e-6
                && cb.y0 <= sb.y0 + 1e-6
                && cb.x1 >= sb.x1 - 1e-6
                && cb.y1 >= sb.y1 - 1e-6;
            if !inside {
                let visible = sb.intersect(cb).area();
                if visible <= 0.0 {
                    return;
                }
                // Text is laid out with substitute fonts, so its box may
                // poke out of a clip that the original glyphs fitted; keep
                // it unclipped while most of it is inside.
                if matches!(shape.kind, ShapeKind::Text { .. }) && visible >= 0.6 * sb.area() {
                    self.shapes.push(shape);
                    return;
                }
                let mut frame = self.new_shape(ShapeKind::Path {
                    path: clip.clone(),
                    closed: true,
                });
                frame.fill = Fill::None;
                frame.stroke = None;
                let mut pc = self.new_shape(ShapeKind::ClipFrame {
                    frame: Box::new(frame),
                    contents: vec![shape],
                });
                pc.fill = Fill::None;
                pc.stroke = None;
                shape = pc;
            }
        }
        self.shapes.push(shape);
    }

    fn paint_path(
        &mut self,
        path: &BezPath,
        fill: bool,
        stroke: bool,
        even_odd: bool,
        gs: &GState,
    ) {
        if path.elements().is_empty() {
            return;
        }
        let b = path.bounding_box();
        if !(b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite()) {
            return;
        }
        let closed = path
            .elements()
            .iter()
            .any(|e| matches!(e, tracedraw_core::geometry::PathEl::ClosePath));
        let mut shape = self.new_shape(ShapeKind::Path {
            path: path.clone(),
            closed: closed || fill,
        });
        shape.fill = if fill {
            match (&gs.fill_pattern, gs.fill) {
                (Some(p), _) => p.clone(),
                (None, Some(c)) => Fill::Solid(c),
                (None, None) => Fill::None,
            }
        } else {
            Fill::None
        };
        if even_odd && fill {
            shape.data.push(("fill.rule".into(), "evenodd".into()));
        }
        shape.stroke = if stroke {
            gs.stroke.map(|c| {
                let scale = Self::scale_of(gs.ctm);
                let mut s = Stroke::new(c, (gs.line_width * scale).max(0.0));
                if s.width < Stroke::HAIRLINE {
                    s.width = Stroke::HAIRLINE;
                }
                s.cap = gs.cap;
                s.join = gs.join;
                if !gs.dash.is_empty() && s.width > 0.0 {
                    s.dash = gs.dash.iter().map(|d| d * scale / s.width).collect();
                }
                s
            })
        } else {
            None
        };
        shape.opacity = if fill { gs.fill_alpha } else { gs.stroke_alpha };
        self.push(shape, &gs.clip);
    }

    // ----- XObjects ---------------------------------------------------------

    fn do_xobject(&mut self, resources: &Dictionary, name: &[u8], gs: &mut GState, depth: usize) {
        let Some(x) = self
            .get(resources, b"XObject")
            .and_then(|o| self.dict_of(o))
            .and_then(|d| d.get(name).ok())
            .cloned()
        else {
            self.warn(format!("XObject {} missing", String::from_utf8_lossy(name)));
            return;
        };
        let Object::Stream(stream) = self.deref(&x).clone() else {
            return;
        };
        let subtype = self
            .get(&stream.dict, b"Subtype")
            .and_then(|o| o.as_name().ok())
            .unwrap_or(b"");
        match subtype {
            b"Form" => {
                if depth >= MAX_FORM_DEPTH {
                    self.warn("form XObjects nested too deeply; skipped");
                    return;
                }
                let mut inner = gs.clone();
                self.draw_form(&stream, &mut inner, depth + 1);
            }
            b"Image" => self.draw_image(&stream, gs),
            other => self.warn(format!(
                "XObject subtype {} not supported",
                String::from_utf8_lossy(other)
            )),
        }
    }

    fn draw_form(&mut self, stream: &Stream, gs: &mut GState, depth: usize) {
        if let Some(m) = self
            .get(&stream.dict, b"Matrix")
            .and_then(|o| self.matrix_of(o))
        {
            gs.ctm *= m;
        }
        if let Some(b) = self
            .get(&stream.dict, b"BBox")
            .and_then(|o| self.rect_of(o))
        {
            let clip = gs.ctm * b.to_path(0.01);
            gs.clip = Some(match &gs.clip {
                Some(old) => intersect_clip(old, &clip),
                None => clip,
            });
        }
        let resources = self
            .get(&stream.dict, b"Resources")
            .and_then(|o| self.dict_of(o))
            .cloned()
            .unwrap_or_default();
        let data = stream
            .decompressed_content()
            .unwrap_or_else(|_| stream.content.clone());
        self.run_content(&data, &resources, gs, depth);
    }

    fn draw_image(&mut self, stream: &Stream, gs: &GState) {
        let Some((png, w, h)) = self.decode_image(stream) else {
            return;
        };
        // The image fills the unit square of the current CTM.
        let unit = Rect::new(0.0, 0.0, 1.0, 1.0);
        let mut shape = self.new_shape(ShapeKind::Bitmap {
            rect: unit,
            width_px: w,
            height_px: h,
            png,
        });
        shape.transform = gs.ctm;
        shape.stroke = None;
        shape.fill = Fill::None;
        shape.opacity = gs.fill_alpha;
        self.push(shape, &gs.clip);
    }

    fn inline_image(&mut self, operands: &[Object], gs: &GState) {
        // lopdf gives the inline image as a stream operand.
        for o in operands {
            if let Object::Stream(s) = o {
                let mut s = s.clone();
                expand_inline_keys(&mut s.dict);
                self.draw_image(&s, gs);
                return;
            }
        }
    }

    /// Decode an image XObject to PNG (RGBA).
    fn decode_image(&mut self, stream: &Stream) -> Option<(Vec<u8>, u32, u32)> {
        let d = &stream.dict;
        let w = self.get(d, b"Width").and_then(Self::num)? as u32;
        let h = self.get(d, b"Height").and_then(Self::num)? as u32;
        if w == 0 || h == 0 || w > 20_000 || h > 20_000 || (w as u64) * (h as u64) > 80_000_000 {
            self.warn("image too large; skipped");
            return None;
        }
        let bpc = self
            .get(d, b"BitsPerComponent")
            .and_then(Self::num)
            .unwrap_or(8.0) as u32;
        let is_mask = matches!(self.get(d, b"ImageMask"), Some(Object::Boolean(true)));
        let filters: Vec<Vec<u8>> = match self.get(d, b"Filter") {
            Some(Object::Name(n)) => vec![n.clone()],
            Some(Object::Array(a)) => a
                .iter()
                .filter_map(|x| self.deref(x).as_name().ok().map(|n| n.to_vec()))
                .collect(),
            _ => Vec::new(),
        };
        let last = filters.last().map(|f| f.as_slice()).unwrap_or(b"");
        let mut rgba: Vec<u8>;
        let cs_obj = d.get(b"ColorSpace").ok().cloned();
        let cs = cs_obj
            .as_ref()
            .map(|o| self.color_space_object(o, 0))
            .unwrap_or(ColorSpace::Gray);
        let decode_arr = self
            .get(d, b"Decode")
            .map(|o| self.nums(o))
            .unwrap_or_default();
        let inverted = decode_arr.first().map(|v| *v == 1.0).unwrap_or(false);
        match last {
            b"DCTDecode" | b"DCT" | b"JPXDecode" => {
                // Strip the other filters first (Flate over JPEG is rare but legal).
                let data = if filters.len() > 1 {
                    let mut tmp = stream.clone();
                    tmp.dict.set(
                        "Filter",
                        Object::Array(
                            filters[..filters.len() - 1]
                                .iter()
                                .map(|f| Object::Name(f.clone()))
                                .collect(),
                        ),
                    );
                    tmp.decompressed_content()
                        .unwrap_or_else(|_| stream.content.clone())
                } else {
                    stream.content.clone()
                };
                if last == b"JPXDecode" {
                    self.warn("JPEG 2000 image replaced by a grey box");
                    rgba = vec![160; (w * h * 4) as usize];
                    for px in rgba.chunks_exact_mut(4) {
                        px[3] = 255;
                    }
                } else {
                    let img = match image::load_from_memory_with_format(
                        &data,
                        image::ImageFormat::Jpeg,
                    ) {
                        Ok(i) => i,
                        Err(e) => {
                            self.warn(format!("JPEG image not decoded: {e}"));
                            return None;
                        }
                    };
                    let is_cmyk_jpeg =
                        img.color() == image::ColorType::Rgba8 && cs == ColorSpace::Cmyk;
                    let _ = is_cmyk_jpeg;
                    let rgb = img.to_rgba8();
                    rgba = rgb.into_raw();
                    if cs == ColorSpace::Cmyk && inverted {
                        for px in rgba.chunks_exact_mut(4) {
                            px[0] = 255 - px[0];
                            px[1] = 255 - px[1];
                            px[2] = 255 - px[2];
                        }
                    }
                    // Decoded JPEG size wins over the dictionary.
                    let (jw, jh) = (img.width(), img.height());
                    if jw != w || jh != h {
                        return Some((tracedraw_render_png(jw, jh, &rgba), jw, jh));
                    }
                }
            }
            _ => {
                let data = if filters.is_empty() {
                    stream.content.clone()
                } else {
                    match stream.decompressed_content() {
                        Ok(d) => d,
                        Err(e) => {
                            self.warn(format!("image data not decoded: {e}"));
                            return None;
                        }
                    }
                };
                let ncomp = if is_mask { 1 } else { cs.components().max(1) };
                let row_bytes = ((w as usize) * ncomp * (bpc as usize)).div_ceil(8);
                if data.len() < row_bytes * (h as usize) {
                    self.warn("image data shorter than its size; skipped");
                    return None;
                }
                // Indexed palettes.
                let palette = cs_obj.as_ref().and_then(|o| self.indexed_palette(o));
                rgba = Vec::with_capacity((w * h * 4) as usize);
                let max = ((1u32 << bpc) - 1) as f32;
                for y in 0..h as usize {
                    let row = &data[y * row_bytes..(y + 1) * row_bytes];
                    let mut bits = BitReader::new(row);
                    for _ in 0..w {
                        let mut comps = [0f32; 4];
                        let mut raw0 = 0u32;
                        for (ci, c) in comps.iter_mut().enumerate().take(ncomp.min(4)) {
                            let v = bits.read(bpc);
                            if ci == 0 {
                                raw0 = v;
                            }
                            *c = v as f32 / max;
                        }
                        for _ in 4..ncomp {
                            bits.read(bpc);
                        }
                        if is_mask {
                            // 1 = masked out (transparent) unless Decode [1 0].
                            let on = (raw0 == 0) != inverted;
                            let c = gs_fill_rgb(None);
                            rgba.extend_from_slice(&[c[0], c[1], c[2], if on { 255 } else { 0 }]);
                            continue;
                        }
                        let (r, g, b) = if let Some(pal) = &palette {
                            let i = (raw0 as usize) * 3;
                            (
                                *pal.get(i).unwrap_or(&0),
                                *pal.get(i + 1).unwrap_or(&0),
                                *pal.get(i + 2).unwrap_or(&0),
                            )
                        } else {
                            let to8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                            match cs {
                                ColorSpace::Rgb => (to8(comps[0]), to8(comps[1]), to8(comps[2])),
                                ColorSpace::Cmyk => {
                                    let (c, m, yy, k) = if inverted {
                                        (
                                            1.0 - comps[0],
                                            1.0 - comps[1],
                                            1.0 - comps[2],
                                            1.0 - comps[3],
                                        )
                                    } else {
                                        (comps[0], comps[1], comps[2], comps[3])
                                    };
                                    (
                                        to8((1.0 - c) * (1.0 - k)),
                                        to8((1.0 - m) * (1.0 - k)),
                                        to8((1.0 - yy) * (1.0 - k)),
                                    )
                                }
                                _ => {
                                    let v = if inverted { 1.0 - comps[0] } else { comps[0] };
                                    (to8(v), to8(v), to8(v))
                                }
                            }
                        };
                        rgba.extend_from_slice(&[r, g, b, 255]);
                    }
                }
            }
        }
        if rgba.len() != (w * h * 4) as usize {
            return None;
        }
        // Soft mask: alpha from a gray image of the same size.
        if let Some(Object::Stream(sm)) = self.get(d, b"SMask").cloned() {
            if let Some(alpha) = self.decode_gray(&sm, w, h) {
                for (px, a) in rgba.chunks_exact_mut(4).zip(alpha) {
                    px[3] = a;
                }
            }
        }
        Some((tracedraw_render_png(w, h, &rgba), w, h))
    }

    /// An SMask as 8-bit alpha, resampled to `w` x `h` (nearest).
    fn decode_gray(&mut self, sm: &Stream, w: u32, h: u32) -> Option<Vec<u8>> {
        let (png, sw, sh) = self.decode_image(sm)?;
        let pm = tiny_skia::Pixmap::decode_png(&png).ok()?;
        let px = pm.pixels();
        let mut out = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            let sy = (y as u64 * sh as u64 / h as u64) as usize;
            for x in 0..w {
                let sx = (x as u64 * sw as u64 / w as u64) as usize;
                let p = px
                    .get(sy * sw as usize + sx)
                    .map(|p| p.demultiply().red())
                    .unwrap_or(255);
                out.push(p);
            }
        }
        Some(out)
    }

    fn indexed_palette(&mut self, cs: &Object) -> Option<Vec<u8>> {
        let Object::Array(a) = self.deref(cs) else {
            return None;
        };
        let family = a.first().and_then(|f| self.deref(f).as_name().ok())?;
        if family != b"Indexed" && family != b"I" {
            return None;
        }
        let base = self.color_space_object(a.get(1)?, 1);
        let lookup: Vec<u8> = match self.deref(a.get(3)?) {
            Object::String(s, _) => s.clone(),
            Object::Stream(s) => s
                .decompressed_content()
                .unwrap_or_else(|_| s.content.clone()),
            _ => return None,
        };
        let n = base.components().max(1);
        let mut pal = Vec::with_capacity(lookup.len() / n * 3);
        for e in lookup.chunks_exact(n) {
            let (r, g, b) = match base {
                ColorSpace::Rgb => (e[0], e[1], e[2]),
                ColorSpace::Cmyk => {
                    let f = |v: u8| v as f32 / 255.0;
                    let k = f(e[3]);
                    let to8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                    (
                        to8((1.0 - f(e[0])) * (1.0 - k)),
                        to8((1.0 - f(e[1])) * (1.0 - k)),
                        to8((1.0 - f(e[2])) * (1.0 - k)),
                    )
                }
                _ => (e[0], e[0], e[0]),
            };
            pal.extend_from_slice(&[r, g, b]);
        }
        Some(pal)
    }

    // ----- shadings and patterns --------------------------------------------

    fn pattern_fill(&mut self, resources: &Dictionary, name: &[u8], gs: &GState) -> Option<Fill> {
        let pat = self
            .get(resources, b"Pattern")
            .and_then(|o| self.dict_of(o))
            .and_then(|d| d.get(name).ok())
            .cloned()?;
        let pd = self.dict_of(&pat)?.clone();
        let ptype = self
            .get(&pd, b"PatternType")
            .and_then(Self::num)
            .unwrap_or(2.0) as i64;
        let matrix = self
            .get(&pd, b"Matrix")
            .and_then(|o| self.matrix_of(o))
            .unwrap_or(Affine::IDENTITY);
        if ptype == 2 {
            let sh = self
                .get(&pd, b"Shading")
                .and_then(|o| self.dict_of(o))
                .cloned()?;
            return self.shading_fill(&sh, gs.ctm * matrix).map(Fill::Fountain);
        }
        // Tiling pattern: the tile content is rendered into a vector
        // pattern fill when it is small enough, else its average colour.
        if let Object::Stream(tile) = self.deref(&pat).clone() {
            let bbox = self
                .get(&tile.dict, b"BBox")
                .and_then(|o| self.rect_of(o))?;
            let xstep = self
                .get(&tile.dict, b"XStep")
                .and_then(Self::num)
                .unwrap_or(bbox.width());
            let ystep = self
                .get(&tile.dict, b"YStep")
                .and_then(Self::num)
                .unwrap_or(bbox.height());
            let saved = std::mem::take(&mut self.shapes);
            let m = gs.ctm * matrix;
            let scale = Self::scale_of(m);
            let mut inner = GState {
                ctm: Affine::scale(scale) * Affine::translate((-bbox.x0, -bbox.y0)),
                ..GState::default()
            };
            let before = self.ops;
            self.draw_form(&tile, &mut inner, MAX_FORM_DEPTH - 2);
            self.ops = before + 1;
            let shapes = std::mem::replace(&mut self.shapes, saved);
            if shapes.is_empty() {
                return None;
            }
            let tile_size = Size::new(
                (xstep.abs() * scale).max(tracedraw_core::Pattern::MIN_TILE_MM),
                (ystep.abs() * scale).max(tracedraw_core::Pattern::MIN_TILE_MM),
            );
            return Some(Fill::Pattern(tracedraw_core::Pattern::Vector {
                shapes,
                tile: tile_size,
            }));
        }
        None
    }

    fn paint_shading(&mut self, resources: &Dictionary, name: &[u8], gs: &GState) {
        let Some(sh) = self
            .get(resources, b"Shading")
            .and_then(|o| self.dict_of(o))
            .and_then(|d| self.get(d, name))
            .and_then(|o| self.dict_of(o))
            .cloned()
        else {
            return;
        };
        // `sh` paints the whole clip region.
        let area = match &gs.clip {
            Some(c) => c.clone(),
            None => {
                self.warn("shading without a clip painted over the page");
                gs.ctm * Rect::new(-10_000.0, -10_000.0, 20_000.0, 20_000.0).to_path(0.01)
            }
        };
        let Some(f) = self.shading_fill(&sh, gs.ctm) else {
            return;
        };
        let mut shape = self.new_shape(ShapeKind::Path {
            path: area,
            closed: true,
        });
        shape.fill = Fill::Fountain(f);
        shape.stroke = None;
        shape.opacity = gs.fill_alpha;
        self.shapes.push(shape);
    }

    /// Axial and radial shadings as fountain fills; other types take the
    /// shading's background or first colour.
    fn shading_fill(&mut self, sh: &Dictionary, m: Affine) -> Option<Fountain> {
        let stype = self
            .get(sh, b"ShadingType")
            .and_then(Self::num)
            .unwrap_or(2.0) as i64;
        let cs = sh.get(b"ColorSpace").ok().cloned();
        let cs = cs
            .as_ref()
            .map(|o| self.color_space_object(o, 0))
            .unwrap_or(ColorSpace::Rgb);
        let func = sh.get(b"Function").ok().cloned();
        let stops = match &func {
            Some(f) => self.sample_function(f, &cs),
            None => Vec::new(),
        };
        let stops = if stops.len() >= 2 {
            stops
        } else {
            let c = stops
                .first()
                .map(|s| s.color)
                .unwrap_or(Color::Gray { v: 0.5 });
            vec![Stop { pos: 0.0, color: c }, Stop { pos: 1.0, color: c }]
        };
        let coords = self
            .get(sh, b"Coords")
            .map(|o| self.nums(o))
            .unwrap_or_default();
        let (kind, angle) = match (stype, coords.len()) {
            (2, 4) => {
                let a = m * Point::new(coords[0], coords[1]);
                let b = m * Point::new(coords[2], coords[3]);
                let d = b - a;
                (FountainKind::Linear, d.y.atan2(d.x).to_degrees())
            }
            (3, 6) => (FountainKind::Radial, 0.0),
            (4..=7, _) => {
                self.warn("mesh shading approximated by a linear fountain");
                (FountainKind::Linear, 0.0)
            }
            _ => (FountainKind::Linear, 0.0),
        };
        Some(Fountain {
            kind,
            stops,
            angle,
            offset: Point::ZERO,
            edge_pad: 0.0,
        })
    }

    /// Sample a shading function into colour stops.
    fn sample_function(&mut self, f: &Object, cs: &ColorSpace) -> Vec<Stop> {
        let mut stops = Vec::new();
        let obj = self.deref(f).clone();
        // An array of functions, one per component.
        if let Object::Array(parts) = &obj {
            let fs: Vec<Object> = parts.iter().map(|p| self.deref(p).clone()).collect();
            for i in 0..=8 {
                let t = i as f64 / 8.0;
                let comps: Vec<f64> = fs
                    .iter()
                    .map(|p| self.eval_function(p, t, 0).first().copied().unwrap_or(0.0))
                    .collect();
                if let Some(c) = color_from_components(&comps, cs) {
                    stops.push(Stop { pos: t, color: c });
                }
            }
            return dedup_stops(stops);
        }
        let Some(fd) = self.dict_of(&obj) else {
            return stops;
        };
        let ftype = self
            .get(fd, b"FunctionType")
            .and_then(Self::num)
            .unwrap_or(2.0) as i64;
        if ftype == 2 {
            let c0 = self
                .get(fd, b"C0")
                .map(|o| self.nums(o))
                .unwrap_or_else(|| vec![0.0]);
            let c1 = self
                .get(fd, b"C1")
                .map(|o| self.nums(o))
                .unwrap_or_else(|| vec![1.0]);
            let n = self.get(fd, b"N").and_then(Self::num).unwrap_or(1.0);
            if (n - 1.0).abs() < 1e-9 {
                if let (Some(a), Some(b)) = (
                    color_from_components(&c0, cs),
                    color_from_components(&c1, cs),
                ) {
                    return vec![Stop { pos: 0.0, color: a }, Stop { pos: 1.0, color: b }];
                }
            }
        }
        if ftype == 3 {
            // Stitching: sub-functions over Bounds; sample each boundary.
            let funcs: Vec<Object> = self
                .get(fd, b"Functions")
                .and_then(|o| o.as_array().ok())
                .map(|a| a.iter().map(|x| self.deref(x).clone()).collect())
                .unwrap_or_default();
            let bounds = self
                .get(fd, b"Bounds")
                .map(|o| self.nums(o))
                .unwrap_or_default();
            let domain = self
                .get(fd, b"Domain")
                .map(|o| self.nums(o))
                .unwrap_or_else(|| vec![0.0, 1.0]);
            let (d0, d1) = (
                domain.first().copied().unwrap_or(0.0),
                domain.get(1).copied().unwrap_or(1.0),
            );
            let mut edges = vec![d0];
            edges.extend(bounds.iter().copied());
            edges.push(d1);
            for (i, f) in funcs.iter().enumerate() {
                let (lo, hi) = (
                    edges.get(i).copied().unwrap_or(0.0),
                    edges.get(i + 1).copied().unwrap_or(1.0),
                );
                let span = (d1 - d0).abs().max(1e-9);
                for (k, t) in [(0usize, 0.0f64), (1, 1.0)] {
                    let _ = k;
                    let comps = self.eval_function(f, t, 1);
                    if let Some(c) = color_from_components(&comps, cs) {
                        let pos = ((lo + (hi - lo) * t) - d0) / span;
                        stops.push(Stop {
                            pos: pos.clamp(0.0, 1.0),
                            color: c,
                        });
                    }
                }
            }
            return dedup_stops(stops);
        }
        // Sampled (type 0), PostScript (type 4) and non-linear type 2:
        // evaluate at a few points.
        for i in 0..=8 {
            let t = i as f64 / 8.0;
            let comps = self.eval_function(&obj, t, 0);
            if let Some(c) = color_from_components(&comps, cs) {
                stops.push(Stop { pos: t, color: c });
            }
        }
        dedup_stops(stops)
    }

    /// Evaluate a 1-in function at `t` in 0..1 (mapped onto its domain).
    fn eval_function(&mut self, f: &Object, t: f64, depth: usize) -> Vec<f64> {
        if depth > 4 {
            return Vec::new();
        }
        let obj = self.deref(f).clone();
        let Some(fd) = self.dict_of(&obj) else {
            return Vec::new();
        };
        let domain = self
            .get(fd, b"Domain")
            .map(|o| self.nums(o))
            .unwrap_or_else(|| vec![0.0, 1.0]);
        let (d0, d1) = (
            domain.first().copied().unwrap_or(0.0),
            domain.get(1).copied().unwrap_or(1.0),
        );
        let x = d0 + (d1 - d0) * t;
        let ftype = self
            .get(fd, b"FunctionType")
            .and_then(Self::num)
            .unwrap_or(2.0) as i64;
        match ftype {
            2 => {
                let c0 = self
                    .get(fd, b"C0")
                    .map(|o| self.nums(o))
                    .unwrap_or_else(|| vec![0.0]);
                let c1 = self
                    .get(fd, b"C1")
                    .map(|o| self.nums(o))
                    .unwrap_or_else(|| vec![1.0]);
                let n = self.get(fd, b"N").and_then(Self::num).unwrap_or(1.0);
                let tt = if n == 1.0 { t } else { t.powf(n) };
                c0.iter()
                    .zip(c1.iter())
                    .map(|(a, b)| a + (b - a) * tt)
                    .collect()
            }
            3 => {
                let funcs: Vec<Object> = self
                    .get(fd, b"Functions")
                    .and_then(|o| o.as_array().ok())
                    .map(|a| a.iter().map(|x| self.deref(x).clone()).collect())
                    .unwrap_or_default();
                let bounds = self
                    .get(fd, b"Bounds")
                    .map(|o| self.nums(o))
                    .unwrap_or_default();
                let mut idx = 0;
                while idx < bounds.len() && x >= bounds[idx] {
                    idx += 1;
                }
                let lo = if idx == 0 { d0 } else { bounds[idx - 1] };
                let hi = if idx >= bounds.len() { d1 } else { bounds[idx] };
                let local = if (hi - lo).abs() < 1e-12 {
                    0.0
                } else {
                    (x - lo) / (hi - lo)
                };
                let encode = self
                    .get(fd, b"Encode")
                    .map(|o| self.nums(o))
                    .unwrap_or_default();
                let (e0, e1) = (
                    encode.get(idx * 2).copied().unwrap_or(0.0),
                    encode.get(idx * 2 + 1).copied().unwrap_or(1.0),
                );
                let sub_t = e0 + (e1 - e0) * local;
                match funcs.get(idx) {
                    Some(sf) => self.eval_function(sf, sub_t.clamp(0.0, 1.0), depth + 1),
                    None => Vec::new(),
                }
            }
            0 => {
                // Sampled function: nearest sample along the first input.
                let Object::Stream(s) = &obj else {
                    return Vec::new();
                };
                let data = s
                    .decompressed_content()
                    .unwrap_or_else(|_| s.content.clone());
                let size = self
                    .get(fd, b"Size")
                    .map(|o| self.nums(o))
                    .unwrap_or_default();
                let bps = self
                    .get(fd, b"BitsPerSample")
                    .and_then(Self::num)
                    .unwrap_or(8.0) as u32;
                let range = self
                    .get(fd, b"Range")
                    .map(|o| self.nums(o))
                    .unwrap_or_default();
                let nout = (range.len() / 2).max(1);
                let n0 = size.first().copied().unwrap_or(2.0).max(1.0) as usize;
                let i = ((t * (n0 as f64 - 1.0)).round() as usize).min(n0 - 1);
                let max = ((1u64 << bps.min(32)) - 1) as f64;
                let mut br = BitReader::new(&data);
                br.skip(i * nout * bps as usize);
                (0..nout)
                    .map(|k| {
                        let v = br.read(bps) as f64 / max;
                        let (r0, r1) = (
                            range.get(k * 2).copied().unwrap_or(0.0),
                            range.get(k * 2 + 1).copied().unwrap_or(1.0),
                        );
                        r0 + (r1 - r0) * v
                    })
                    .collect()
            }
            _ => {
                self.warn("PostScript calculator shading function approximated");
                vec![t]
            }
        }
    }

    // ----- text ---------------------------------------------------------------

    fn font(&mut self, resources: &Dictionary, name: &[u8]) -> Option<std::rc::Rc<FontInfo>> {
        let fonts = self.get(resources, b"Font").and_then(|o| self.dict_of(o))?;
        let entry = fonts.get(name).ok()?;
        let key = match entry {
            Object::Reference(id) => Some(*id),
            _ => None,
        };
        if let Some(k) = key {
            if let Some(f) = self.fonts.get(&k) {
                return Some(f.clone());
            }
        }
        let fd = self.dict_of(entry)?.clone();
        let info = std::rc::Rc::new(self.font_info(&fd));
        if let Some(k) = key {
            self.fonts.insert(k, info.clone());
        }
        Some(info)
    }

    fn font_info(&mut self, fd: &Dictionary) -> FontInfo {
        let subtype = self
            .get(fd, b"Subtype")
            .and_then(|o| o.as_name().ok())
            .unwrap_or(b"")
            .to_vec();
        let base = self
            .get(fd, b"BaseFont")
            .and_then(|o| o.as_name().ok())
            .map(|b| String::from_utf8_lossy(b).to_string())
            .unwrap_or_else(|| "Helvetica".into());
        let (family, mut bold, mut italic) = split_font_name(&base);
        let two_byte = subtype == b"Type0";
        // Descriptor (own or the descendant's) for flags and missing width.
        let desc_font: Dictionary = if two_byte {
            self.get(fd, b"DescendantFonts")
                .and_then(|o| o.as_array().ok())
                .and_then(|a| a.first())
                .and_then(|d| self.dict_of(d))
                .cloned()
                .unwrap_or_default()
        } else {
            fd.clone()
        };
        let descriptor = self
            .get(&desc_font, b"FontDescriptor")
            .and_then(|o| self.dict_of(o))
            .cloned();
        let mut default_width = 500.0;
        if let Some(desc) = &descriptor {
            let flags = self.get(desc, b"Flags").and_then(Self::num).unwrap_or(0.0) as i64;
            if flags & (1 << 18) != 0 {
                bold = true;
            }
            if flags & (1 << 6) != 0 {
                italic = true;
            }
            if let Some(w) = self.get(desc, b"StemV").and_then(Self::num) {
                if w >= 120.0 {
                    bold = true;
                }
            }
            if let Some(mw) = self.get(desc, b"MissingWidth").and_then(Self::num) {
                if mw > 0.0 {
                    default_width = mw;
                }
            }
        }
        // ToUnicode CMap.
        let to_unicode = self
            .get(fd, b"ToUnicode")
            .and_then(|o| match o {
                Object::Stream(s) => Some(s.clone()),
                _ => None,
            })
            .and_then(|s| s.decompressed_content().ok().or(Some(s.content.clone())))
            .map(|data| parse_tounicode(&data));
        // Simple-font encoding table.
        let one_byte = if two_byte {
            None
        } else {
            Some(self.simple_encoding(fd, &base, &subtype))
        };
        // Widths.
        let mut widths = HashMap::new();
        if two_byte {
            if let Some(dw) = self.get(&desc_font, b"DW").and_then(Self::num) {
                default_width = dw;
            } else {
                default_width = 1000.0;
            }
            if let Some(Object::Array(w)) = self.get(&desc_font, b"W").cloned() {
                let items: Vec<Object> = w.iter().map(|x| self.deref(x).clone()).collect();
                let mut i = 0;
                while i < items.len() {
                    let Some(first) = Self::num(&items[i]) else {
                        break;
                    };
                    match items.get(i + 1) {
                        Some(Object::Array(ws)) => {
                            for (k, wv) in ws.iter().enumerate() {
                                if let Some(wv) = Self::num(self.deref(wv)) {
                                    widths.insert(first as u32 + k as u32, wv);
                                }
                            }
                            i += 2;
                        }
                        Some(last) => {
                            let last = Self::num(last).unwrap_or(first);
                            let wv = items
                                .get(i + 2)
                                .and_then(Self::num)
                                .unwrap_or(default_width);
                            let (a, b) = (first as u32, (last as u32).min(first as u32 + 65535));
                            if b >= a && b - a < 65536 {
                                for c in a..=b {
                                    widths.insert(c, wv);
                                }
                            }
                            i += 3;
                        }
                        None => break,
                    }
                }
            }
        } else {
            let first = self
                .get(fd, b"FirstChar")
                .and_then(Self::num)
                .unwrap_or(0.0) as u32;
            if let Some(Object::Array(w)) = self.get(fd, b"Widths").cloned() {
                for (k, wv) in w.iter().enumerate() {
                    if let Some(wv) = Self::num(self.deref(wv)) {
                        widths.insert(first + k as u32, wv);
                    }
                }
            } else {
                // Standard 14 fonts without widths: rough metrics.
                let lower = base.to_ascii_lowercase();
                default_width = if lower.contains("courier") || lower.contains("mono") {
                    600.0
                } else {
                    520.0
                };
                for (c, w) in STANDARD_WIDTHS {
                    widths.insert(*c as u32, *w);
                }
            }
        }
        let type3_matrix = if subtype == b"Type3" {
            self.get(fd, b"FontMatrix").and_then(|o| self.matrix_of(o))
        } else {
            None
        };
        if subtype == b"Type3" {
            // Glyph procedures are not run; the widths are in glyph space.
            if let Some(m) = type3_matrix {
                let s = m.as_coeffs()[0] * 1000.0;
                for w in widths.values_mut() {
                    *w *= s;
                }
                default_width *= s;
            }
        }
        FontInfo {
            family,
            bold,
            italic,
            two_byte,
            to_unicode,
            one_byte,
            widths,
            default_width,
            type3_matrix,
        }
    }

    fn simple_encoding(
        &mut self,
        fd: &Dictionary,
        base: &str,
        subtype: &[u8],
    ) -> [Option<char>; 256] {
        let symbolic = base.contains("Symbol") || base.contains("Dingbat");
        let mut table: [Option<char>; 256] = if symbolic && subtype != b"TrueType" {
            [None; 256]
        } else {
            standard_table(STANDARD_ENCODING)
        };
        match self.get(fd, b"Encoding").cloned() {
            Some(Object::Name(n)) => {
                table = named_encoding(&n).unwrap_or(table);
            }
            Some(Object::Dictionary(ed)) => {
                if let Some(Object::Name(n)) = self.get(&ed, b"BaseEncoding") {
                    if let Some(t) = named_encoding(n) {
                        table = t;
                    }
                } else if !symbolic {
                    table = standard_table(STANDARD_ENCODING);
                }
                if let Some(Object::Array(diff)) = self.get(&ed, b"Differences").cloned() {
                    let mut code = 0usize;
                    for item in diff {
                        match self.deref(&item) {
                            Object::Integer(i) => code = (*i).max(0) as usize,
                            Object::Real(r) => code = (*r).max(0.0) as usize,
                            Object::Name(g) => {
                                if code < 256 {
                                    if let Some(c) = glyph_name_to_char(g) {
                                        table[code] = Some(c);
                                    }
                                }
                                code += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {
                if !symbolic {
                    table = standard_table(WIN_ANSI_ENCODING);
                }
            }
        }
        table
    }

    fn show_text(
        &mut self,
        bytes: &[u8],
        gs: &mut GState,
        text_matrix: &mut Affine,
        run: &mut Option<TextRun>,
    ) {
        let Some(font) = gs.font.clone() else {
            // Text without a font: advance only.
            return;
        };
        if gs.font_size == 0.0 {
            return;
        }
        let items = font.decode(bytes);
        let invisible = gs.render_mode == 3 || gs.render_mode == 7;
        // Device matrix of the glyph origin: ctm * Tm * [1 0 0 1 0 rise].
        let trm = gs.ctm * *text_matrix * Affine::translate((0.0, gs.rise));
        let scale = Self::scale_of(trm);
        let size_pt = (gs.font_size * scale / PT_MM).abs();
        let size_pt = if size_pt.is_finite() { size_pt } else { 0.0 };
        let same_run = match run {
            Some(r) => {
                std::rc::Rc::ptr_eq(&r.font, &font)
                    && (r.size_pt - size_pt).abs() < 1e-6
                    && r.fill == gs.fill
                    && same_line(&r.matrix, &trm, r.end_x)
            }
            None => false,
        };
        if !same_run {
            if let Some(r) = run.take() {
                self.flush_run(r);
            }
            if !invisible && size_pt > 0.0 {
                *run = Some(TextRun {
                    text: String::new(),
                    font: font.clone(),
                    size_pt,
                    matrix: trm,
                    end_x: 0.0,
                    fill: gs.fill,
                    alpha: gs.fill_alpha,
                    clip: gs.clip.clone(),
                });
            }
        } else if let Some(r) = run.as_mut() {
            // Continuing on the same line: measure where this string starts
            // along the run's baseline; a gap means a space.
            let a = r.matrix.as_coeffs();
            let here = trm.as_coeffs();
            let d = Vec2::new(here[4] - a[4], here[5] - a[5]);
            let dir = Vec2::new(a[0], a[1]);
            let len2 = dir.hypot2().max(1e-12);
            let new_end = (d.x * dir.x + d.y * dir.y) / len2;
            let gap = (new_end - r.end_x) * len2.sqrt();
            let em = r.size_pt * PT_MM;
            if gap > em * 0.18 && !r.text.ends_with(' ') {
                r.text.push(' ');
            }
            r.end_x = new_end;
        }
        let mut advance = 0.0; // text space units (before Tm)
        for (code, s, is_space) in items {
            let w0 = font.width(code) / 1000.0;
            let mut tx = (w0 * gs.font_size + gs.char_spacing) * gs.hscale;
            if is_space {
                tx += gs.word_spacing * gs.hscale;
            }
            if let Some(r) = run.as_mut() {
                r.text.push_str(&s);
            }
            advance += tx;
        }
        if let Some(r) = run.as_mut() {
            r.end_x += advance;
        }
        *text_matrix *= Affine::translate((advance, 0.0));
    }

    fn flush_run(&mut self, r: TextRun) {
        if r.text.trim().is_empty() {
            return;
        }
        let scale = Self::scale_of(r.matrix);
        if scale <= 0.0 || !scale.is_finite() {
            return;
        }
        // Take the size out of the matrix: the span carries it in points.
        let transform = r.matrix * Affine::scale(1.0 / scale);
        let span = TextSpan {
            bold: r.font.bold,
            italic: r.font.italic,
            ..TextSpan::new(r.text, r.font.family.clone(), r.size_pt)
        };
        let mut shape = self.new_shape(ShapeKind::Text {
            spans: vec![span],
            origin: Point::ZERO,
            frame: None,
            align: TextAlign::Left,
            para: ParagraphStyle::default(),
            on_path: None,
        });
        shape.transform = transform;
        shape.fill = match r.fill {
            Some(c) => Fill::Solid(c),
            None => Fill::Solid(Color::BLACK),
        };
        shape.stroke = None;
        shape.opacity = r.alpha;
        let _ = r.font.type3_matrix;
        self.push(shape, &r.clip);
    }
}

/// Same baseline and direction, continuing after the previous advance.
fn same_line(prev: &Affine, next: &Affine, end_x: f64) -> bool {
    let a = prev.as_coeffs();
    let b = next.as_coeffs();
    let dir_same = (a[0] - b[0]).abs() < 1e-6
        && (a[1] - b[1]).abs() < 1e-6
        && (a[2] - b[2]).abs() < 1e-6
        && (a[3] - b[3]).abs() < 1e-6;
    if !dir_same {
        return false;
    }
    let expected = *prev * Affine::translate((end_x, 0.0));
    let e = expected.as_coeffs();
    let d = Vec2::new(b[4] - e[4], b[5] - e[5]);
    // Along the baseline: anything short of a line break; across it: tiny.
    let dir = Vec2::new(a[0], a[1]);
    let len = dir.hypot().max(1e-9);
    let along = (d.x * dir.x + d.y * dir.y) / len;
    let across = (d.x * -dir.y + d.y * dir.x).abs() / len;
    let em = (a[0] * a[3] - a[1] * a[2]).abs().sqrt().max(1e-9);
    across < em * 0.05 && along > -em * 0.2 && along < em * 3.0
}

fn dedup_stops(mut stops: Vec<Stop>) -> Vec<Stop> {
    stops.sort_by(|a, b| a.pos.total_cmp(&b.pos));
    stops.dedup_by(|a, b| (a.pos - b.pos).abs() < 1e-9 && a.color == b.color);
    stops
}

fn color_from_components(v: &[f64], cs: &ColorSpace) -> Option<Color> {
    let c = |i: usize| v.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0) as f32;
    match (cs, v.len()) {
        (ColorSpace::Gray, 1) | (ColorSpace::N(1), 1) => Some(Color::Gray { v: c(0) }),
        (ColorSpace::Rgb, 3) | (ColorSpace::N(3), 3) => Some(Color::Rgb {
            r: c(0),
            g: c(1),
            b: c(2),
        }),
        (ColorSpace::Cmyk, 4) | (ColorSpace::N(4), 4) => Some(Color::Cmyk {
            c: c(0),
            m: c(1),
            y: c(2),
            k: c(3),
        }),
        (_, 1) => Some(Color::Gray { v: 1.0 - c(0) }), // Separation tint: 1 = full ink
        (_, 3) => Some(Color::Rgb {
            r: c(0),
            g: c(1),
            b: c(2),
        }),
        (_, 4) => Some(Color::Cmyk {
            c: c(0),
            m: c(1),
            y: c(2),
            k: c(3),
        }),
        _ => None,
    }
}

fn gs_fill_rgb(c: Option<Color>) -> [u8; 3] {
    let c = c.unwrap_or(Color::BLACK);
    let [r, g, b] = c.to_rgb8();
    [r, g, b]
}

/// Intersection of two clip paths, approximated by the second when it lies
/// inside the first's bounds and by the first otherwise.
fn intersect_clip(old: &BezPath, new: &BezPath) -> BezPath {
    let ob = old.bounding_box();
    let nb = new.bounding_box();
    if nb.x0 >= ob.x0 - 1e-6
        && nb.y0 >= ob.y0 - 1e-6
        && nb.x1 <= ob.x1 + 1e-6
        && nb.y1 <= ob.y1 + 1e-6
    {
        new.clone()
    } else if ob.x0 >= nb.x0 - 1e-6
        && ob.y0 >= nb.y0 - 1e-6
        && ob.x1 <= nb.x1 + 1e-6
        && ob.y1 <= nb.y1 + 1e-6
    {
        old.clone()
    } else {
        // Neither contains the other: the rectangle both share.
        ob.intersect(nb).to_path(0.01)
    }
}

/// Inline image abbreviations to their full keys.
fn expand_inline_keys(d: &mut Dictionary) {
    let pairs: [(&[u8], &str); 8] = [
        (b"W", "Width"),
        (b"H", "Height"),
        (b"BPC", "BitsPerComponent"),
        (b"CS", "ColorSpace"),
        (b"F", "Filter"),
        (b"IM", "ImageMask"),
        (b"D", "Decode"),
        (b"DP", "DecodeParms"),
    ];
    for (short, long) in pairs {
        if let Ok(v) = d.get(short).cloned() {
            if !d.has(long.as_bytes()) {
                d.set(long, v);
            }
        }
    }
    // Abbreviated filter names.
    if let Ok(Object::Name(n)) = d.get(b"Filter").cloned() {
        let full = match n.as_slice() {
            b"Fl" => Some("FlateDecode"),
            b"AHx" => Some("ASCIIHexDecode"),
            b"A85" => Some("ASCII85Decode"),
            b"LZW" => Some("LZWDecode"),
            b"RL" => Some("RunLengthDecode"),
            b"CCF" => Some("CCITTFaxDecode"),
            b"DCT" => Some("DCTDecode"),
            _ => None,
        };
        if let Some(f) = full {
            d.set("Filter", Object::Name(f.as_bytes().to_vec()));
        }
    }
}

fn tracedraw_render_png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    crate::svg::encode_png(w, h, rgba)
}

/// MSB-first bit reader for packed image samples.
struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, pos: 0 }
    }
    fn skip(&mut self, bits: usize) {
        self.pos += bits;
    }
    fn read(&mut self, bits: u32) -> u32 {
        let mut v = 0u32;
        for _ in 0..bits.min(32) {
            let byte = self.data.get(self.pos / 8).copied().unwrap_or(0);
            let bit = (byte >> (7 - (self.pos % 8))) & 1;
            v = (v << 1) | bit as u32;
            self.pos += 1;
        }
        v
    }
}

/// "ABCDEF+Arial-BoldItalicMT" -> ("Arial", bold, italic).
fn split_font_name(base: &str) -> (String, bool, bool) {
    let name = match base.find('+') {
        Some(6) => &base[7..],
        _ => base,
    };
    let lower = name.to_ascii_lowercase();
    let bold = lower.contains("bold")
        || lower.contains("black")
        || lower.contains("heavy")
        || lower.contains("semibold");
    let italic = lower.contains("italic") || lower.contains("oblique");
    let mut family = name.split([',', '-']).next().unwrap_or(name).to_string();
    for suffix in ["MT", "PSMT", "PS"] {
        if family.len() > suffix.len() + 2 && family.ends_with(suffix) {
            family.truncate(family.len() - suffix.len());
        }
    }
    // Camel-case base names like "TimesNewRoman" get their spaces back.
    let family = match family.as_str() {
        "TimesNewRoman" | "Times" | "TimesNewRomanPS" => "Times New Roman".to_string(),
        "ArialNarrow" => "Arial Narrow".to_string(),
        "CourierNew" | "Courier" => "Courier New".to_string(),
        "Helvetica" | "ArialMT" => "Arial".to_string(),
        "Symbol" => "Symbol".to_string(),
        "ZapfDingbats" => "Zapf Dingbats".to_string(),
        other => other.to_string(),
    };
    (family, bold, italic)
}

/// Parse a ToUnicode CMap (bfchar and bfrange sections) into code -> text.
fn parse_tounicode(data: &[u8]) -> HashMap<u32, String> {
    let text = String::from_utf8_lossy(data);
    let mut map = HashMap::new();
    let tokens: Vec<&str> = tokenize(&text);
    let hex = |t: &str| -> Option<Vec<u8>> {
        let t = t.strip_prefix('<')?.strip_suffix('>')?;
        let t: String = t.chars().filter(|c| !c.is_whitespace()).collect();
        (0..t.len().saturating_sub(1))
            .step_by(2)
            .map(|i| u8::from_str_radix(&t[i..i + 2], 16).ok())
            .collect()
    };
    let code_of = |b: &[u8]| -> u32 { b.iter().fold(0u32, |a, x| (a << 8) | *x as u32) };
    let utf16 = |b: &[u8]| -> String {
        let units: Vec<u16> = b
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    };
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "beginbfchar" => {
                i += 1;
                while i + 1 < tokens.len() && tokens[i] != "endbfchar" {
                    if let (Some(src), Some(dst)) = (hex(tokens[i]), hex(tokens[i + 1])) {
                        map.insert(code_of(&src), utf16(&dst));
                    }
                    i += 2;
                }
            }
            "beginbfrange" => {
                i += 1;
                while i + 2 < tokens.len() && tokens[i] != "endbfrange" {
                    let (lo, hi) = (hex(tokens[i]), hex(tokens[i + 1]));
                    if tokens[i + 2] == "[" {
                        // Explicit list of destinations.
                        let mut k = i + 3;
                        let mut code = lo.as_ref().map(|b| code_of(b)).unwrap_or(0);
                        while k < tokens.len() && tokens[k] != "]" {
                            if let Some(d) = hex(tokens[k]) {
                                map.insert(code, utf16(&d));
                            }
                            code += 1;
                            k += 1;
                        }
                        i = k + 1;
                        continue;
                    }
                    if let (Some(lo), Some(hi), Some(dst)) = (lo, hi, hex(tokens[i + 2])) {
                        let (a, b) = (code_of(&lo), code_of(&hi));
                        if b >= a && b - a < 65536 {
                            let base = utf16(&dst);
                            let mut chars: Vec<char> = base.chars().collect();
                            for c in a..=b {
                                map.insert(c, chars.iter().collect());
                                if let Some(last) = chars.last_mut() {
                                    *last = char::from_u32(*last as u32 + 1).unwrap_or(*last);
                                }
                            }
                        }
                    }
                    i += 3;
                }
            }
            _ => i += 1,
        }
    }
    map
}

fn tokenize(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == b'<' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'>' {
                i += 1;
            }
            i = (i + 1).min(bytes.len());
            out.push(&text[start..i]);
        } else if c == b'[' || c == b']' {
            out.push(&text[i..i + 1]);
            i += 1;
        } else if c == b'%' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else {
            let start = i;
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && !matches!(bytes[i], b'<' | b'[' | b']')
            {
                i += 1;
            }
            out.push(&text[start..i]);
        }
    }
    out
}

/// Encoding tables: index = code, value = Unicode scalar (0 = undefined).
type EncTable = &'static [(u8, u32)];

fn standard_table(t: EncTable) -> [Option<char>; 256] {
    let mut out = [None; 256];
    for (code, u) in t {
        out[*code as usize] = char::from_u32(*u);
    }
    // ASCII printable range is shared by every Latin encoding.
    for c in 0x20u8..0x7f {
        if out[c as usize].is_none() {
            out[c as usize] = Some(c as char);
        }
    }
    out
}

fn named_encoding(n: &[u8]) -> Option<[Option<char>; 256]> {
    match n {
        b"WinAnsiEncoding" => Some(standard_table(WIN_ANSI_ENCODING)),
        b"MacRomanEncoding" => Some(standard_table(MAC_ROMAN_ENCODING)),
        b"StandardEncoding" | b"PDFDocEncoding" => Some(standard_table(STANDARD_ENCODING)),
        b"MacExpertEncoding" => Some(standard_table(STANDARD_ENCODING)),
        _ => None,
    }
}

/// Differences from ASCII in the Standard encoding (high half only).
const STANDARD_ENCODING: EncTable = &[
    (0x27, 0x2019),
    (0x60, 0x2018),
    (0xa1, 0xa1),
    (0xa2, 0xa2),
    (0xa3, 0xa3),
    (0xa4, 0x2044),
    (0xa5, 0xa5),
    (0xa6, 0x192),
    (0xa7, 0xa7),
    (0xa8, 0xa4),
    (0xa9, 0x27),
    (0xaa, 0x201c),
    (0xab, 0xab),
    (0xac, 0x2039),
    (0xad, 0x203a),
    (0xae, 0xfb01),
    (0xaf, 0xfb02),
    (0xb1, 0x2013),
    (0xb2, 0x2020),
    (0xb3, 0x2021),
    (0xb4, 0xb7),
    (0xb6, 0xb6),
    (0xb7, 0x2022),
    (0xb8, 0x201a),
    (0xb9, 0x201e),
    (0xba, 0x201d),
    (0xbb, 0xbb),
    (0xbc, 0x2026),
    (0xbd, 0x2030),
    (0xbf, 0xbf),
    (0xc1, 0x60),
    (0xc2, 0xb4),
    (0xc3, 0x2c6),
    (0xc4, 0x2dc),
    (0xc5, 0xaf),
    (0xc6, 0x2d8),
    (0xc7, 0x2d9),
    (0xc8, 0xa8),
    (0xca, 0x2da),
    (0xcb, 0xb8),
    (0xcd, 0x2dd),
    (0xce, 0x2db),
    (0xcf, 0x2c7),
    (0xd0, 0x2014),
    (0xe1, 0xc6),
    (0xe3, 0xaa),
    (0xe8, 0x141),
    (0xe9, 0xd8),
    (0xea, 0x152),
    (0xeb, 0xba),
    (0xf1, 0xe6),
    (0xf5, 0x131),
    (0xf8, 0x142),
    (0xf9, 0xf8),
    (0xfa, 0x153),
    (0xfb, 0xdf),
];

/// WinAnsi: Latin-1 above 0xa0 plus the 0x80..0x9f specials.
const WIN_ANSI_ENCODING: EncTable = &[
    (0x80, 0x20ac),
    (0x82, 0x201a),
    (0x83, 0x192),
    (0x84, 0x201e),
    (0x85, 0x2026),
    (0x86, 0x2020),
    (0x87, 0x2021),
    (0x88, 0x2c6),
    (0x89, 0x2030),
    (0x8a, 0x160),
    (0x8b, 0x2039),
    (0x8c, 0x152),
    (0x8e, 0x17d),
    (0x91, 0x2018),
    (0x92, 0x2019),
    (0x93, 0x201c),
    (0x94, 0x201d),
    (0x95, 0x2022),
    (0x96, 0x2013),
    (0x97, 0x2014),
    (0x98, 0x2dc),
    (0x99, 0x2122),
    (0x9a, 0x161),
    (0x9b, 0x203a),
    (0x9c, 0x153),
    (0x9e, 0x17e),
    (0x9f, 0x178),
    (0xa0, 0xa0),
    (0xa1, 0xa1),
    (0xa2, 0xa2),
    (0xa3, 0xa3),
    (0xa4, 0xa4),
    (0xa5, 0xa5),
    (0xa6, 0xa6),
    (0xa7, 0xa7),
    (0xa8, 0xa8),
    (0xa9, 0xa9),
    (0xaa, 0xaa),
    (0xab, 0xab),
    (0xac, 0xac),
    (0xad, 0xad),
    (0xae, 0xae),
    (0xaf, 0xaf),
    (0xb0, 0xb0),
    (0xb1, 0xb1),
    (0xb2, 0xb2),
    (0xb3, 0xb3),
    (0xb4, 0xb4),
    (0xb5, 0xb5),
    (0xb6, 0xb6),
    (0xb7, 0xb7),
    (0xb8, 0xb8),
    (0xb9, 0xb9),
    (0xba, 0xba),
    (0xbb, 0xbb),
    (0xbc, 0xbc),
    (0xbd, 0xbd),
    (0xbe, 0xbe),
    (0xbf, 0xbf),
    (0xc0, 0xc0),
    (0xc1, 0xc1),
    (0xc2, 0xc2),
    (0xc3, 0xc3),
    (0xc4, 0xc4),
    (0xc5, 0xc5),
    (0xc6, 0xc6),
    (0xc7, 0xc7),
    (0xc8, 0xc8),
    (0xc9, 0xc9),
    (0xca, 0xca),
    (0xcb, 0xcb),
    (0xcc, 0xcc),
    (0xcd, 0xcd),
    (0xce, 0xce),
    (0xcf, 0xcf),
    (0xd0, 0xd0),
    (0xd1, 0xd1),
    (0xd2, 0xd2),
    (0xd3, 0xd3),
    (0xd4, 0xd4),
    (0xd5, 0xd5),
    (0xd6, 0xd6),
    (0xd7, 0xd7),
    (0xd8, 0xd8),
    (0xd9, 0xd9),
    (0xda, 0xda),
    (0xdb, 0xdb),
    (0xdc, 0xdc),
    (0xdd, 0xdd),
    (0xde, 0xde),
    (0xdf, 0xdf),
    (0xe0, 0xe0),
    (0xe1, 0xe1),
    (0xe2, 0xe2),
    (0xe3, 0xe3),
    (0xe4, 0xe4),
    (0xe5, 0xe5),
    (0xe6, 0xe6),
    (0xe7, 0xe7),
    (0xe8, 0xe8),
    (0xe9, 0xe9),
    (0xea, 0xea),
    (0xeb, 0xeb),
    (0xec, 0xec),
    (0xed, 0xed),
    (0xee, 0xee),
    (0xef, 0xef),
    (0xf0, 0xf0),
    (0xf1, 0xf1),
    (0xf2, 0xf2),
    (0xf3, 0xf3),
    (0xf4, 0xf4),
    (0xf5, 0xf5),
    (0xf6, 0xf6),
    (0xf7, 0xf7),
    (0xf8, 0xf8),
    (0xf9, 0xf9),
    (0xfa, 0xfa),
    (0xfb, 0xfb),
    (0xfc, 0xfc),
    (0xfd, 0xfd),
    (0xfe, 0xfe),
    (0xff, 0xff),
];

const MAC_ROMAN_ENCODING: EncTable = &[
    (0x80, 0xc4),
    (0x81, 0xc5),
    (0x82, 0xc7),
    (0x83, 0xc9),
    (0x84, 0xd1),
    (0x85, 0xd6),
    (0x86, 0xdc),
    (0x87, 0xe1),
    (0x88, 0xe0),
    (0x89, 0xe2),
    (0x8a, 0xe4),
    (0x8b, 0xe3),
    (0x8c, 0xe5),
    (0x8d, 0xe7),
    (0x8e, 0xe9),
    (0x8f, 0xe8),
    (0x90, 0xea),
    (0x91, 0xeb),
    (0x92, 0xed),
    (0x93, 0xec),
    (0x94, 0xee),
    (0x95, 0xef),
    (0x96, 0xf1),
    (0x97, 0xf3),
    (0x98, 0xf2),
    (0x99, 0xf4),
    (0x9a, 0xf6),
    (0x9b, 0xf5),
    (0x9c, 0xfa),
    (0x9d, 0xf9),
    (0x9e, 0xfb),
    (0x9f, 0xfc),
    (0xa0, 0x2020),
    (0xa1, 0xb0),
    (0xa2, 0xa2),
    (0xa3, 0xa3),
    (0xa4, 0xa7),
    (0xa5, 0x2022),
    (0xa6, 0xb6),
    (0xa7, 0xdf),
    (0xa8, 0xae),
    (0xa9, 0xa9),
    (0xaa, 0x2122),
    (0xab, 0xb4),
    (0xac, 0xa8),
    (0xae, 0xc6),
    (0xaf, 0xd8),
    (0xb1, 0xb1),
    (0xb4, 0xa5),
    (0xb5, 0xb5),
    (0xbb, 0xaa),
    (0xbc, 0xba),
    (0xbe, 0xe6),
    (0xbf, 0xf8),
    (0xc0, 0xbf),
    (0xc1, 0xa1),
    (0xc2, 0xac),
    (0xc4, 0x192),
    (0xc7, 0xab),
    (0xc8, 0xbb),
    (0xc9, 0x2026),
    (0xca, 0xa0),
    (0xcb, 0xc0),
    (0xcc, 0xc3),
    (0xcd, 0xd5),
    (0xce, 0x152),
    (0xcf, 0x153),
    (0xd0, 0x2013),
    (0xd1, 0x2014),
    (0xd2, 0x201c),
    (0xd3, 0x201d),
    (0xd4, 0x2018),
    (0xd5, 0x2019),
    (0xd6, 0xf7),
    (0xd8, 0xff),
    (0xd9, 0x178),
    (0xda, 0x2044),
    (0xdb, 0x20ac),
    (0xdc, 0x2039),
    (0xdd, 0x203a),
    (0xde, 0xfb01),
    (0xdf, 0xfb02),
    (0xe0, 0x2021),
    (0xe1, 0xb7),
    (0xe2, 0x201a),
    (0xe3, 0x201e),
    (0xe4, 0x2030),
    (0xe5, 0xc2),
    (0xe6, 0xca),
    (0xe7, 0xc1),
    (0xe8, 0xcb),
    (0xe9, 0xc8),
    (0xea, 0xcd),
    (0xeb, 0xce),
    (0xec, 0xcf),
    (0xed, 0xcc),
    (0xee, 0xd3),
    (0xef, 0xd4),
    (0xf1, 0xd2),
    (0xf2, 0xda),
    (0xf3, 0xdb),
    (0xf4, 0xd9),
    (0xf5, 0x131),
    (0xf6, 0x2c6),
    (0xf7, 0x2dc),
    (0xf8, 0xaf),
    (0xf9, 0x2d8),
    (0xfa, 0x2d9),
    (0xfb, 0x2da),
    (0xfc, 0xb8),
    (0xfd, 0x2dd),
    (0xfe, 0x2db),
    (0xff, 0x2c7),
];

/// Helvetica widths for the standard 14 fonts without a Widths array.
const STANDARD_WIDTHS: &[(u8, f64)] = &[
    (b' ', 278.0),
    (b'!', 278.0),
    (b'"', 355.0),
    (b'#', 556.0),
    (b'$', 556.0),
    (b'%', 889.0),
    (b'&', 667.0),
    (b'\'', 191.0),
    (b'(', 333.0),
    (b')', 333.0),
    (b'*', 389.0),
    (b'+', 584.0),
    (b',', 278.0),
    (b'-', 333.0),
    (b'.', 278.0),
    (b'/', 278.0),
    (b'0', 556.0),
    (b'1', 556.0),
    (b'2', 556.0),
    (b'3', 556.0),
    (b'4', 556.0),
    (b'5', 556.0),
    (b'6', 556.0),
    (b'7', 556.0),
    (b'8', 556.0),
    (b'9', 556.0),
    (b':', 278.0),
    (b';', 278.0),
    (b'<', 584.0),
    (b'=', 584.0),
    (b'>', 584.0),
    (b'?', 556.0),
    (b'@', 1015.0),
    (b'A', 667.0),
    (b'B', 667.0),
    (b'C', 722.0),
    (b'D', 722.0),
    (b'E', 667.0),
    (b'F', 611.0),
    (b'G', 778.0),
    (b'H', 722.0),
    (b'I', 278.0),
    (b'J', 500.0),
    (b'K', 667.0),
    (b'L', 556.0),
    (b'M', 833.0),
    (b'N', 722.0),
    (b'O', 778.0),
    (b'P', 667.0),
    (b'Q', 778.0),
    (b'R', 722.0),
    (b'S', 667.0),
    (b'T', 611.0),
    (b'U', 722.0),
    (b'V', 667.0),
    (b'W', 944.0),
    (b'X', 667.0),
    (b'Y', 667.0),
    (b'Z', 611.0),
    (b'[', 278.0),
    (b'\\', 278.0),
    (b']', 278.0),
    (b'^', 469.0),
    (b'_', 556.0),
    (b'`', 333.0),
    (b'a', 556.0),
    (b'b', 556.0),
    (b'c', 500.0),
    (b'd', 556.0),
    (b'e', 556.0),
    (b'f', 278.0),
    (b'g', 556.0),
    (b'h', 556.0),
    (b'i', 222.0),
    (b'j', 222.0),
    (b'k', 500.0),
    (b'l', 222.0),
    (b'm', 833.0),
    (b'n', 556.0),
    (b'o', 556.0),
    (b'p', 556.0),
    (b'q', 556.0),
    (b'r', 333.0),
    (b's', 500.0),
    (b't', 278.0),
    (b'u', 556.0),
    (b'v', 500.0),
    (b'w', 722.0),
    (b'x', 500.0),
    (b'y', 500.0),
    (b'z', 500.0),
    (b'{', 334.0),
    (b'|', 260.0),
    (b'}', 334.0),
    (b'~', 584.0),
];

/// Glyph names of the Adobe Glyph List that matter for Latin text, plus
/// the `uniXXXX` and `uXXXX[X[X]]` forms.
fn glyph_name_to_char(name: &[u8]) -> Option<char> {
    let n = std::str::from_utf8(name).ok()?;
    if let Some(hex) = n.strip_prefix("uni") {
        if hex.len() >= 4 {
            return u32::from_str_radix(&hex[..4], 16)
                .ok()
                .and_then(char::from_u32);
        }
    }
    if let Some(hex) = n.strip_prefix('u') {
        if (4..=6).contains(&hex.len()) {
            if let Ok(v) = u32::from_str_radix(hex, 16) {
                return char::from_u32(v);
            }
        }
    }
    if n.len() == 1 {
        return n.chars().next();
    }
    // gXX / cidXX / GXX are glyph indices with no Unicode meaning.
    let table: &[(&str, char)] = &[
        ("space", ' '),
        ("exclam", '!'),
        ("quotedbl", '"'),
        ("numbersign", '#'),
        ("dollar", '$'),
        ("percent", '%'),
        ("ampersand", '&'),
        ("quotesingle", '\''),
        ("parenleft", '('),
        ("parenright", ')'),
        ("asterisk", '*'),
        ("plus", '+'),
        ("comma", ','),
        ("hyphen", '-'),
        ("period", '.'),
        ("slash", '/'),
        ("zero", '0'),
        ("one", '1'),
        ("two", '2'),
        ("three", '3'),
        ("four", '4'),
        ("five", '5'),
        ("six", '6'),
        ("seven", '7'),
        ("eight", '8'),
        ("nine", '9'),
        ("colon", ':'),
        ("semicolon", ';'),
        ("less", '<'),
        ("equal", '='),
        ("greater", '>'),
        ("question", '?'),
        ("at", '@'),
        ("bracketleft", '['),
        ("backslash", '\\'),
        ("bracketright", ']'),
        ("asciicircum", '^'),
        ("underscore", '_'),
        ("grave", '`'),
        ("braceleft", '{'),
        ("bar", '|'),
        ("braceright", '}'),
        ("asciitilde", '~'),
        ("exclamdown", '\u{a1}'),
        ("cent", '\u{a2}'),
        ("sterling", '\u{a3}'),
        ("currency", '\u{a4}'),
        ("yen", '\u{a5}'),
        ("brokenbar", '\u{a6}'),
        ("section", '\u{a7}'),
        ("dieresis", '\u{a8}'),
        ("copyright", '\u{a9}'),
        ("ordfeminine", '\u{aa}'),
        ("guillemotleft", '\u{ab}'),
        ("logicalnot", '\u{ac}'),
        ("registered", '\u{ae}'),
        ("macron", '\u{af}'),
        ("degree", '\u{b0}'),
        ("plusminus", '\u{b1}'),
        ("twosuperior", '\u{b2}'),
        ("threesuperior", '\u{b3}'),
        ("acute", '\u{b4}'),
        ("mu", '\u{b5}'),
        ("paragraph", '\u{b6}'),
        ("periodcentered", '\u{b7}'),
        ("cedilla", '\u{b8}'),
        ("onesuperior", '\u{b9}'),
        ("ordmasculine", '\u{ba}'),
        ("guillemotright", '\u{bb}'),
        ("onequarter", '\u{bc}'),
        ("onehalf", '\u{bd}'),
        ("threequarters", '\u{be}'),
        ("questiondown", '\u{bf}'),
        ("Agrave", '\u{c0}'),
        ("Aacute", '\u{c1}'),
        ("Acircumflex", '\u{c2}'),
        ("Atilde", '\u{c3}'),
        ("Adieresis", '\u{c4}'),
        ("Aring", '\u{c5}'),
        ("AE", '\u{c6}'),
        ("Ccedilla", '\u{c7}'),
        ("Egrave", '\u{c8}'),
        ("Eacute", '\u{c9}'),
        ("Ecircumflex", '\u{ca}'),
        ("Edieresis", '\u{cb}'),
        ("Igrave", '\u{cc}'),
        ("Iacute", '\u{cd}'),
        ("Icircumflex", '\u{ce}'),
        ("Idieresis", '\u{cf}'),
        ("Eth", '\u{d0}'),
        ("Ntilde", '\u{d1}'),
        ("Ograve", '\u{d2}'),
        ("Oacute", '\u{d3}'),
        ("Ocircumflex", '\u{d4}'),
        ("Otilde", '\u{d5}'),
        ("Odieresis", '\u{d6}'),
        ("multiply", '\u{d7}'),
        ("Oslash", '\u{d8}'),
        ("Ugrave", '\u{d9}'),
        ("Uacute", '\u{da}'),
        ("Ucircumflex", '\u{db}'),
        ("Udieresis", '\u{dc}'),
        ("Yacute", '\u{dd}'),
        ("Thorn", '\u{de}'),
        ("germandbls", '\u{df}'),
        ("agrave", '\u{e0}'),
        ("aacute", '\u{e1}'),
        ("acircumflex", '\u{e2}'),
        ("atilde", '\u{e3}'),
        ("adieresis", '\u{e4}'),
        ("aring", '\u{e5}'),
        ("ae", '\u{e6}'),
        ("ccedilla", '\u{e7}'),
        ("egrave", '\u{e8}'),
        ("eacute", '\u{e9}'),
        ("ecircumflex", '\u{ea}'),
        ("edieresis", '\u{eb}'),
        ("igrave", '\u{ec}'),
        ("iacute", '\u{ed}'),
        ("icircumflex", '\u{ee}'),
        ("idieresis", '\u{ef}'),
        ("eth", '\u{f0}'),
        ("ntilde", '\u{f1}'),
        ("ograve", '\u{f2}'),
        ("oacute", '\u{f3}'),
        ("ocircumflex", '\u{f4}'),
        ("otilde", '\u{f5}'),
        ("odieresis", '\u{f6}'),
        ("divide", '\u{f7}'),
        ("oslash", '\u{f8}'),
        ("ugrave", '\u{f9}'),
        ("uacute", '\u{fa}'),
        ("ucircumflex", '\u{fb}'),
        ("udieresis", '\u{fc}'),
        ("yacute", '\u{fd}'),
        ("thorn", '\u{fe}'),
        ("ydieresis", '\u{ff}'),
        ("quoteleft", '\u{2018}'),
        ("quoteright", '\u{2019}'),
        ("quotedblleft", '\u{201c}'),
        ("quotedblright", '\u{201d}'),
        ("quotesinglbase", '\u{201a}'),
        ("quotedblbase", '\u{201e}'),
        ("endash", '\u{2013}'),
        ("emdash", '\u{2014}'),
        ("bullet", '\u{2022}'),
        ("ellipsis", '\u{2026}'),
        ("dagger", '\u{2020}'),
        ("daggerdbl", '\u{2021}'),
        ("perthousand", '\u{2030}'),
        ("guilsinglleft", '\u{2039}'),
        ("guilsinglright", '\u{203a}'),
        ("trademark", '\u{2122}'),
        ("Euro", '\u{20ac}'),
        ("fi", '\u{fb01}'),
        ("fl", '\u{fb02}'),
        ("OE", '\u{152}'),
        ("oe", '\u{153}'),
        ("Scaron", '\u{160}'),
        ("scaron", '\u{161}'),
        ("Zcaron", '\u{17d}'),
        ("zcaron", '\u{17e}'),
        ("Ydieresis", '\u{178}'),
        ("florin", '\u{192}'),
        ("circumflex", '\u{2c6}'),
        ("tilde", '\u{2dc}'),
        ("dotlessi", '\u{131}'),
        ("Lslash", '\u{141}'),
        ("lslash", '\u{142}'),
        ("minus", '\u{2212}'),
        ("fraction", '\u{2044}'),
        ("nbspace", '\u{a0}'),
        ("sfthyphen", '\u{ad}'),
        ("middot", '\u{b7}'),
        ("Ccaron", '\u{10c}'),
        ("ccaron", '\u{10d}'),
        ("Rcaron", '\u{158}'),
        ("rcaron", '\u{159}'),
        ("Ecaron", '\u{11a}'),
        ("ecaron", '\u{11b}'),
        ("Dcaron", '\u{10e}'),
        ("dcaron", '\u{10f}'),
        ("Ncaron", '\u{147}'),
        ("ncaron", '\u{148}'),
        ("Tcaron", '\u{164}'),
        ("tcaron", '\u{165}'),
        ("Uring", '\u{16e}'),
        ("uring", '\u{16f}'),
        ("Aogonek", '\u{104}'),
        ("aogonek", '\u{105}'),
        ("Eogonek", '\u{118}'),
        ("eogonek", '\u{119}'),
        ("Sacute", '\u{15a}'),
        ("sacute", '\u{15b}'),
        ("Zacute", '\u{179}'),
        ("zacute", '\u{17a}'),
        ("Zdotaccent", '\u{17b}'),
        ("zdotaccent", '\u{17c}'),
        ("Cacute", '\u{106}'),
        ("cacute", '\u{107}'),
        ("Nacute", '\u{143}'),
        ("nacute", '\u{144}'),
        ("Gbreve", '\u{11e}'),
        ("gbreve", '\u{11f}'),
        ("Idotaccent", '\u{130}'),
        ("Scedilla", '\u{15e}'),
        ("scedilla", '\u{15f}'),
    ];
    table.iter().find(|(k, _)| *k == n).map(|(_, c)| *c)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny PDF writer for the tests: objects in order, xref built here.
    fn pdf(objects: &[String]) -> Vec<u8> {
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, o) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, o).as_bytes());
        }
        let xref = out.len();
        out.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
        );
        for o in offsets {
            out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                objects.len() + 1,
                xref
            )
            .as_bytes(),
        );
        out
    }

    fn stream(dict: &str, content: &str) -> String {
        format!(
            "<< {dict} /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        )
    }

    fn one_page(content: &str, resources: &str, extra: &[String]) -> Vec<u8> {
        let mut objs = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources {resources} >>"
            ),
            stream("", content),
        ];
        objs.extend(extra.iter().cloned());
        pdf(&objs)
    }

    #[test]
    fn paths_fills_strokes_and_page_size() {
        let bytes = one_page(
            "1 0 0 RG 0 0 1 rg 4 w 10 10 m 60 10 l 60 40 l h B 0.5 g 100 50 50 30 re f",
            "<< >>",
            &[],
        );
        let mut ids = IdSource::default();
        let imp = parse(&bytes, &mut ids).expect("parse");
        assert_eq!(imp.pages.len(), 1);
        let p = &imp.pages[0];
        assert!((p.size.width - 200.0 * PT_MM).abs() < 1e-9);
        assert!((p.size.height - 100.0 * PT_MM).abs() < 1e-9);
        assert_eq!(p.shapes.len(), 2, "{:?}", imp.warnings);
        let tri = &p.shapes[0];
        assert_eq!(
            tri.fill,
            Fill::Solid(Color::Rgb {
                r: 0.0,
                g: 0.0,
                b: 1.0
            })
        );
        let s = tri.stroke.as_ref().expect("stroke");
        assert_eq!(
            s.color,
            Color::Rgb {
                r: 1.0,
                g: 0.0,
                b: 0.0
            }
        );
        assert!((s.width - 4.0 * PT_MM).abs() < 1e-9);
        let b = tri.bounds();
        assert!((b.x0 - 10.0 * PT_MM).abs() < 1e-6 && (b.x1 - 60.0 * PT_MM).abs() < 1e-6);
        let rect = &p.shapes[1];
        assert_eq!(rect.fill, Fill::Solid(Color::Gray { v: 0.5 }));
        assert!(rect.stroke.is_none());
        let rb = rect.bounds();
        assert!((rb.y1 - 80.0 * PT_MM).abs() < 1e-6);
    }

    #[test]
    fn text_runs_merge_and_carry_font_and_size() {
        let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>";
        let bytes = one_page(
            "BT /F1 12 Tf 20 30 Td (Hello) Tj ( World) Tj 0 -20 Td [(Se) -250 (cond)] TJ ET",
            "<< /Font << /F1 5 0 R >> >>",
            &[font.to_string()],
        );
        let mut ids = IdSource::default();
        let imp = parse(&bytes, &mut ids).expect("parse");
        let p = &imp.pages[0];
        let texts: Vec<(String, f64, bool)> = p
            .shapes
            .iter()
            .filter_map(|s| match &s.kind {
                ShapeKind::Text { spans, .. } => {
                    Some((spans[0].text.clone(), spans[0].size_pt, spans[0].bold))
                }
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 2, "{:?} {:?}", texts, imp.warnings);
        assert_eq!(texts[0].0, "Hello World");
        assert!((texts[0].1 - 12.0).abs() < 1e-6);
        assert!(texts[0].2);
        assert_eq!(texts[1].0, "Se cond");
        // Origin of the first run at (20, 30) pt.
        let t = p.shapes[0].transform.as_coeffs();
        assert!((t[4] - 20.0 * PT_MM).abs() < 1e-6 && (t[5] - 30.0 * PT_MM).abs() < 1e-6);
    }

    #[test]
    fn form_xobject_clip_and_alpha() {
        let form = stream(
            "/Type /XObject /Subtype /Form /BBox [0 0 50 50] /Matrix [1 0 0 1 100 0]",
            "0 0 1 rg 0 0 50 50 re f",
        );
        let bytes = one_page(
            "/GS1 gs /Fx Do q 0 0 20 20 re W n 1 0 0 rg 0 0 100 100 re f Q",
            "<< /XObject << /Fx 5 0 R >> /ExtGState << /GS1 << /ca 0.5 >> >> >>",
            &[form],
        );
        let mut ids = IdSource::default();
        let imp = parse(&bytes, &mut ids).expect("parse");
        let p = &imp.pages[0];
        assert_eq!(p.shapes.len(), 2, "{:?}", imp.warnings);
        // The form's square moved by its matrix, with the page alpha.
        let b = p.shapes[0].bounds();
        assert!((b.x0 - 100.0 * PT_MM).abs() < 1e-6, "{b:?}");
        assert!((p.shapes[0].opacity - 0.5).abs() < 1e-9);
        // The clipped rectangle became a ClipFrame with the clip as frame.
        match &p.shapes[1].kind {
            ShapeKind::ClipFrame { frame, contents } => {
                let fb = frame.bounds();
                assert!((fb.x1 - 20.0 * PT_MM).abs() < 1e-6);
                assert_eq!(contents.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn axial_shading_pattern_becomes_a_fountain() {
        let bytes = one_page(
            "/Pattern cs /P1 scn 0 0 100 100 re f",
            "<< /Pattern << /P1 << /PatternType 2 /Shading << /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 100 0] /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >> >> >> >> >>",
            &[],
        );
        let mut ids = IdSource::default();
        let imp = parse(&bytes, &mut ids).expect("parse");
        let s = &imp.pages[0].shapes[0];
        match &s.fill {
            Fill::Fountain(f) => {
                assert_eq!(f.kind, FountainKind::Linear);
                assert_eq!(f.stops.len(), 2);
                assert_eq!(
                    f.stops[0].color,
                    Color::Rgb {
                        r: 1.0,
                        g: 0.0,
                        b: 0.0
                    }
                );
                assert!(f.angle.abs() < 1e-6);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn raw_rgb_image_and_ai_preamble() {
        // 2x1 RGB image: red, green.
        let img = "<< /Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 6 >>\nstream\n\u{ff}\u{0}\u{0}\u{0}\u{ff}\u{0}\nendstream";
        let _ = img;
        let mut objs = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>".to_string(),
            stream("", "q 50 0 0 25 10 10 cm /Im1 Do Q"),
        ];
        // Binary image data cannot go through format!; build the object by hand.
        let mut doc = pdf(&objs);
        let _ = &mut doc;
        objs.push(String::new());
        let mut bytes = b"%!PS-Adobe-3.0\n%%Creator: test\n".to_vec();
        let mut body = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, o) in objs.iter().enumerate() {
            offsets.push(body.len());
            if i == 4 {
                body.extend_from_slice(b"5 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 6 >>\nstream\n");
                body.extend_from_slice(&[255, 0, 0, 0, 255, 0]);
                body.extend_from_slice(b"\nendstream\nendobj\n");
            } else {
                body.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, o).as_bytes());
            }
        }
        let xref = body.len();
        body.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes(),
        );
        for o in offsets {
            body.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        body.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objs.len() + 1
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(&body);
        assert!(is_pdf(&bytes));
        let mut ids = IdSource::default();
        let imp = parse(&bytes, &mut ids).expect("parse");
        let s = &imp.pages[0].shapes[0];
        match &s.kind {
            ShapeKind::Bitmap {
                width_px,
                height_px,
                png,
                ..
            } => {
                assert_eq!((*width_px, *height_px), (2, 1));
                let pm = tiny_skia::Pixmap::decode_png(png).unwrap();
                let px = pm.pixels();
                assert_eq!((px[0].red(), px[0].green()), (255, 0));
                assert_eq!((px[1].red(), px[1].green()), (0, 255));
            }
            other => panic!("{other:?}"),
        }
        let b = s.bounds();
        assert!((b.x0 - 10.0 * PT_MM).abs() < 1e-6 && (b.width() - 50.0 * PT_MM).abs() < 1e-6);
    }

    #[test]
    fn tounicode_and_differences_decode_text() {
        let cmap = "/CIDInit /ProcSet findresource begin begincmap 1 begincodespacerange <00> <FF> endcodespacerange 2 beginbfchar <41> <00E9> <42> <0041 0042> endbfchar 1 beginbfrange <61> <63> <0078> endbfrange endcmap";
        let map = parse_tounicode(cmap.as_bytes());
        assert_eq!(map.get(&0x41).map(String::as_str), Some("é"));
        assert_eq!(map.get(&0x42).map(String::as_str), Some("AB"));
        assert_eq!(map.get(&0x62).map(String::as_str), Some("y"));
        assert_eq!(glyph_name_to_char(b"eacute"), Some('é'));
        assert_eq!(glyph_name_to_char(b"uni20AC"), Some('€'));
        assert_eq!(
            split_font_name("ABCDEF+TimesNewRomanPS-BoldItalicMT"),
            ("Times New Roman".into(), true, true)
        );
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        let mut ids = IdSource::default();
        assert!(parse(b"hello", &mut ids).is_err());
        assert!(parse(b"%PDF-1.4 garbage", &mut ids).is_err());
    }
}
