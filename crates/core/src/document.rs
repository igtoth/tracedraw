//! The document model: pages, layers and shapes. Pure data, serde-friendly,
//! no behaviour beyond lookups and bounds. Mutation goes through commands.

use crate::geometry::{self, Affine, BezPath, Point, Rect, Shape as _, Vec2};
use crate::id::{IdSource, LayerId, PageId, ShapeId};
use crate::style::{Fill, Stroke};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// Hook for the text engine. The core cannot depend on fonts, so the app
/// registers a function that turns spans into glyph outlines (baseline at
/// the origin, mm). Without it text is drawn as a box.
pub mod text_outline {
    use super::{ParagraphStyle, TextAlign, TextOnPath, TextSpan};
    use crate::geometry::BezPath;
    use std::sync::OnceLock;

    /// Everything the text engine needs to lay out one text object.
    pub struct TextRequest<'a> {
        pub spans: &'a [TextSpan],
        /// Frame size in mm for paragraph text; `None` for artistic text.
        pub frame: Option<crate::geometry::Size>,
        pub align: TextAlign,
        pub para: &'a ParagraphStyle,
        pub on_path: Option<&'a TextOnPath>,
    }

    type Outliner = fn(&TextRequest) -> BezPath;
    static OUTLINER: OnceLock<Outliner> = OnceLock::new();

    pub fn set(f: Outliner) {
        let _ = OUTLINER.set(f);
    }

    pub fn outline(req: &TextRequest) -> Option<BezPath> {
        OUTLINER.get().map(|f| f(req))
    }
}

/// Paper sizes in millimetres.
pub mod paper {
    use crate::geometry::Size;
    pub const A4: Size = Size::new(210.0, 297.0);
    pub const A3: Size = Size::new(297.0, 420.0);
    pub const LETTER: Size = Size::new(215.9, 279.4);
}

/// The style of a rectangle's corners: Round, Scallop or Chamfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CornerKind {
    #[default]
    Round,
    /// Cut by a quarter circle centred on the corner.
    Scallop,
    /// Cut by a straight line.
    Chamfer,
}

impl CornerKind {
    pub const ALL: [CornerKind; 3] = [CornerKind::Round, CornerKind::Scallop, CornerKind::Chamfer];
}

/// Each corner's size (mm) and the corner style of a rectangle. The radii
/// run in the property bar's order: top left, top right, bottom left,
/// bottom right (top is +y in the local space).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Corners {
    pub radii: [f64; 4],
    #[serde(default)]
    pub kind: CornerKind,
    /// The corners keep their size when the rectangle is scaled (Relative
    /// corner scaling turned off): the radii
    /// are page millimetres. Otherwise they are in the rectangle's own
    /// space and scale, or stretch, with it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fixed: bool,
}

impl Corners {
    pub const TOP_LEFT: usize = 0;
    pub const TOP_RIGHT: usize = 1;
    pub const BOTTOM_LEFT: usize = 2;
    pub const BOTTOM_RIGHT: usize = 3;

    pub fn uniform(radius: f64, kind: CornerKind) -> Corners {
        Corners {
            radii: [radius; 4],
            kind,
            fixed: false,
        }
    }

    /// Every corner the same size.
    pub fn is_uniform(&self) -> bool {
        self.radii.iter().all(|r| (r - self.radii[0]).abs() < 1e-9)
    }

    /// Same size on every corner, round and scaling with the object: a
    /// plain radius says as much.
    pub fn is_plain(&self) -> bool {
        self.kind == CornerKind::Round && !self.fixed && self.is_uniform()
    }

    /// The largest corner.
    pub fn max(&self) -> f64 {
        self.radii.iter().cloned().fold(0.0, f64::max)
    }

    /// No corner is cut.
    pub fn is_square(&self) -> bool {
        self.max() <= 1e-9
    }

    /// Each corner in drawing order (counter-clockwise from the bottom
    /// right): its index, the corner point, the edge directions into and
    /// out of it, and how far the cut runs along each edge, all in the
    /// rectangle's own space. `scale` is the page length of one unit along
    /// the rectangle's x and y axes; fixed corners are divided by it so
    /// that they keep their size on the page. A corner is at most half the
    /// shorter side.
    fn layout(&self, rect: Rect, scale: (f64, f64)) -> [CornerAt; 4] {
        let rect = rect.abs();
        let (sx, sy) = if self.fixed {
            (scale.0.abs().max(1e-9), scale.1.abs().max(1e-9))
        } else {
            (1.0, 1.0)
        };
        // Page-space limit, then each corner as an ellipse in local space.
        let limit = (rect.width() * sx).min(rect.height() * sy) / 2.0;
        let (x0, y0, x1, y1) = (rect.x0, rect.y0, rect.x1, rect.y1);
        let at = |index: usize, p: Point, d_in: Vec2, d_out: Vec2| {
            let r = self.radii[index];
            let r = if r.is_finite() {
                r.clamp(0.0, limit.max(0.0))
            } else {
                0.0
            };
            // The part of the corner's size along a direction.
            let along = |d: Vec2| if d.x != 0.0 { r / sx } else { r / sy };
            CornerAt {
                index,
                p,
                d_in,
                d_out,
                a_in: along(d_in),
                a_out: along(d_out),
            }
        };
        [
            at(
                Self::BOTTOM_RIGHT,
                Point::new(x1, y0),
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
            ),
            at(
                Self::TOP_RIGHT,
                Point::new(x1, y1),
                Vec2::new(0.0, 1.0),
                Vec2::new(-1.0, 0.0),
            ),
            at(
                Self::TOP_LEFT,
                Point::new(x0, y1),
                Vec2::new(-1.0, 0.0),
                Vec2::new(0.0, -1.0),
            ),
            at(
                Self::BOTTOM_LEFT,
                Point::new(x0, y0),
                Vec2::new(0.0, -1.0),
                Vec2::new(1.0, 0.0),
            ),
        ]
    }

    /// Where each corner's cut meets the edges, in the rectangle's own
    /// space, by corner index (top left, top right, bottom left, bottom
    /// right): the corner point, the end on the vertical edge, the end on
    /// the horizontal edge. A square corner has both ends on its point.
    pub fn ends(&self, rect: Rect, scale: (f64, f64)) -> [(Point, Point, Point); 4] {
        let mut out = [(Point::ZERO, Point::ZERO, Point::ZERO); 4];
        for c in self.layout(rect, scale) {
            let a = c.p - c.d_in * c.a_in;
            let b = c.p + c.d_out * c.a_out;
            out[c.index] = if c.d_in.x == 0.0 {
                (c.p, a, b)
            } else {
                (c.p, b, a)
            };
        }
        out
    }

    /// Outline of `rect` with these corners; see `layout` for `scale`.
    pub fn path(&self, rect: Rect, scale: (f64, f64)) -> BezPath {
        // Quarter-ellipse handle length.
        const K: f64 = 0.552_284_749_830_793_4;
        let order = self.layout(rect, scale);
        let mut path = BezPath::new();
        let last = &order[3];
        let start = last.p + last.d_out * last.a_out;
        path.move_to(start);
        let mut at = start;
        for c in order {
            let (a_in, a_out) = (c.a_in, c.a_out);
            let (d_in, d_out) = (c.d_in, c.d_out);
            let a = c.p - d_in * a_in;
            let b = c.p + d_out * a_out;
            if (a - at).hypot() > 1e-12 {
                path.line_to(a);
            }
            if a_in > 1e-12 && a_out > 1e-12 {
                match self.kind {
                    CornerKind::Round => {
                        path.curve_to(a + d_in * (K * a_in), b - d_out * (K * a_out), b)
                    }
                    CornerKind::Scallop => {
                        path.curve_to(a + d_out * (K * a_out), b - d_in * (K * a_in), b)
                    }
                    CornerKind::Chamfer => path.line_to(b),
                }
            } else if (b - a).hypot() > 1e-12 {
                path.line_to(b);
            }
            at = b;
        }
        path.close_path();
        path
    }
}

/// One corner of a rectangle as `Corners::layout` places it.
struct CornerAt {
    index: usize,
    p: Point,
    d_in: Vec2,
    d_out: Vec2,
    a_in: f64,
    a_out: f64,
}

/// Page length of one unit along the x and y axes of `t`.
pub fn axis_scale(t: Affine) -> (f64, f64) {
    let [a, b, c, d, _, _] = t.as_coeffs();
    (a.hypot(b), c.hypot(d))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ShapeKind {
    /// Rectangle in local space; `radius` rounds the corners (mm), or
    /// `corners` gives each corner its own size and the corner style.
    Rect {
        rect: Rect,
        radius: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        corners: Option<Corners>,
    },
    /// Ellipse inscribed in `rect`, in local space; optionally a pie or arc.
    Ellipse {
        rect: Rect,
        #[serde(default)]
        arc: Option<EllipseArc>,
    },
    /// Polygon or star inscribed in `rect`. `complex` makes a complex
    /// star: each vertex joined to the one `complex + 1` further on, so the
    /// sides cross (`sharpness` is then unused).
    Polygon {
        rect: Rect,
        points: u32,
        sharpness: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        complex: Option<u32>,
    },
    /// Free path (lines and cubic Beziers) in local space.
    Path { path: BezPath, closed: bool },
    /// Artistic text (a single-line text object).
    Text {
        spans: Vec<TextSpan>,
        origin: crate::geometry::Point,
        /// Paragraph text has a frame (width, height); artistic text has none.
        #[serde(default)]
        frame: Option<crate::geometry::Size>,
        #[serde(default)]
        align: TextAlign,
        #[serde(default)]
        para: ParagraphStyle,
        /// Text fitted to a path (Text > Fit Text to Path).
        #[serde(default)]
        on_path: Option<TextOnPath>,
    },
    /// A group of child shapes.
    Group { children: Vec<Shape> },
    /// ClipFrame: `contents` drawn clipped to the `frame` object's outline.
    ClipFrame {
        frame: Box<Shape>,
        contents: Vec<Shape>,
    },
    /// A table: a grid of cells with text; rendered through `Shape::expand`.
    Table(Table),
    /// An instance of `Document::symbols[index]`, drawn with this shape's transform.
    SymbolInstance { index: usize },
    /// A bitmap, PNG-encoded, placed in `rect` (local space). With bitmap
    /// effects `png` is their result and `fx` keeps the original and the
    /// effects, so they can be edited, hidden or removed.
    Bitmap {
        rect: Rect,
        width_px: u32,
        height_px: u32,
        #[serde(with = "png_bytes")]
        png: Vec<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fx: Option<Box<BitmapFxStack>>,
    },
}

/// One bitmap effect, as the Properties docker's FX tab lists it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BitmapEffect {
    /// The effect's stable name ("gaussian_blur").
    pub id: String,
    /// Its settings by name: colours as 0xRRGGBB, choices as their index,
    /// check boxes as 0 or 1. Missing ones take the effect's defaults.
    #[serde(default)]
    pub params: std::collections::BTreeMap<String, f64>,
    #[serde(default = "visible_default")]
    pub visible: bool,
}

fn visible_default() -> bool {
    true
}

/// What a bitmap's effects were applied to: the original pixels and
/// place, and the effects in order (the first applies first).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BitmapFxStack {
    pub rect: Rect,
    pub width_px: u32,
    pub height_px: u32,
    #[serde(with = "png_bytes")]
    pub png: Vec<u8>,
    pub effects: Vec<BitmapEffect>,
}

impl ShapeKind {
    /// A rectangle with these corners. Plain ones (one round radius that
    /// scales with the object) keep the short form; otherwise `radius`
    /// holds the largest corner for readers that know only one.
    pub fn rect_with_corners(rect: Rect, corners: Corners) -> ShapeKind {
        if corners.is_plain() {
            let r = corners.radii[0];
            ShapeKind::Rect {
                rect,
                radius: if r.is_finite() { r.max(0.0) } else { 0.0 },
                corners: None,
            }
        } else {
            ShapeKind::Rect {
                rect,
                radius: corners.max(),
                corners: Some(corners),
            }
        }
    }

    /// The corners of a rectangle as stored (a plain radius as four equal
    /// round corners); `None` for other kinds.
    pub fn rect_corners(&self) -> Option<(Rect, Corners)> {
        match self {
            ShapeKind::Rect {
                rect,
                radius,
                corners,
            } => Some((
                *rect,
                corners.unwrap_or_else(|| Corners::uniform(*radius, CornerKind::Round)),
            )),
            _ => None,
        }
    }
}

/// PNG bytes as base64 in the JSON format.
pub mod png_bytes {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(v))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .map_err(serde::de::Error::custom)
    }
}

/// Pie or arc section of an ellipse, angles in degrees counter-clockwise from +x.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EllipseArc {
    pub start_deg: f64,
    pub end_deg: f64,
    /// Pie (wedge closed through the centre) or open arc.
    pub pie: bool,
}

/// Drop shadow attached to an object (the Drop Shadow tool).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub offset: crate::geometry::Vec2,
    /// 0..1
    pub opacity: f64,
    /// Feathering radius in mm.
    pub blur: f64,
    pub color: crate::color::Color,
}

impl Default for Shadow {
    fn default() -> Self {
        Shadow {
            offset: crate::geometry::Vec2::new(2.0, -2.0),
            opacity: 0.5,
            blur: 1.5,
            color: crate::color::Color::BLACK,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextSpan {
    pub text: String,
    pub font_family: String,
    /// Font size in points.
    pub size_pt: f64,
    pub bold: bool,
    pub italic: bool,
    /// Range kerning (tracking) as a percentage of the em; 0 = none.
    #[serde(default)]
    pub tracking_pct: f64,
    /// Baseline shift in points, positive = up (the vertical character
    /// offset).
    #[serde(default)]
    pub baseline_shift_pt: f64,
    /// Horizontal character offset as a percentage of the font size,
    /// positive = right; the characters after it keep their place.
    #[serde(default)]
    pub offset_x_pct: f64,
    /// Character angle in degrees, counter-clockwise, about the
    /// character's origin on its (shifted) baseline.
    #[serde(default)]
    pub angle_deg: f64,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub strikethrough: bool,
    /// Per-span fill override (character colour); `None` = object fill.
    #[serde(default)]
    pub fill: Option<Fill>,
    /// OpenType features to enable, e.g. "liga", "smcp", "frac".
    #[serde(default)]
    pub features: Vec<String>,
}

impl TextSpan {
    pub fn new(text: impl Into<String>, font_family: impl Into<String>, size_pt: f64) -> Self {
        TextSpan {
            text: text.into(),
            font_family: font_family.into(),
            size_pt,
            bold: false,
            italic: false,
            tracking_pct: 0.0,
            baseline_shift_pt: 0.0,
            offset_x_pct: 0.0,
            angle_deg: 0.0,
            underline: false,
            strikethrough: false,
            fill: None,
            features: Vec::new(),
        }
    }

    /// Number of characters (Unicode scalar values) in the span.
    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }
}

/// Total character count of a span list (the index space of
/// [`split_spans_at`] and of `TextLayout::fitted_chars`).
pub fn spans_char_count(spans: &[TextSpan]) -> usize {
    spans.iter().map(TextSpan::char_count).sum()
}

/// Split a span list at a character index of the concatenated text,
/// keeping every span's style on both sides. A span cut in the middle
/// becomes two spans with the same style; spans left empty by the cut are
/// dropped. `at_chars` beyond the end puts everything in the first list.
pub fn split_spans_at(spans: &[TextSpan], at_chars: usize) -> (Vec<TextSpan>, Vec<TextSpan>) {
    let mut head = Vec::new();
    let mut tail = Vec::new();
    let mut seen = 0usize;
    for span in spans {
        let n = span.char_count();
        if seen + n <= at_chars {
            if n > 0 {
                head.push(span.clone());
            }
        } else if seen >= at_chars {
            if n > 0 {
                tail.push(span.clone());
            }
        } else {
            let cut = at_chars - seen;
            let byte = span
                .text
                .char_indices()
                .nth(cut)
                .map(|(b, _)| b)
                .unwrap_or(span.text.len());
            let (a, b) = span.text.split_at(byte);
            if !a.is_empty() {
                head.push(TextSpan {
                    text: a.to_string(),
                    ..span.clone()
                });
            }
            if !b.is_empty() {
                tail.push(TextSpan {
                    text: b.to_string(),
                    ..span.clone()
                });
            }
        }
        seen += n;
    }
    (head, tail)
}

/// The concatenated text of a span list.
pub fn spans_text(spans: &[TextSpan]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

/// Insert `text` at character index `at`, with the style of the character
/// before the insertion point (the first span's style at the start). An
/// empty list gets nothing inserted; callers keep at least one span.
pub fn spans_insert(spans: &mut [TextSpan], at: usize, text: &str) {
    if text.is_empty() {
        return;
    }
    let total = spans_char_count(spans);
    let at = at.min(total);
    // The span holding the character before `at`.
    let mut seen = 0usize;
    let mut target: Option<(usize, usize)> = None;
    for (i, span) in spans.iter().enumerate() {
        let n = span.char_count();
        if at <= seen + n && (n > 0 || spans.len() == 1) && (at > seen || i == 0) {
            target = Some((i, at - seen));
            break;
        }
        seen += n;
    }
    let (i, cut) = match target {
        Some(t) => t,
        None => match spans.len() {
            0 => return,
            n => (n - 1, spans[n - 1].char_count()),
        },
    };
    if let Some(span) = spans.get_mut(i) {
        let byte = span
            .text
            .char_indices()
            .nth(cut)
            .map(|(b, _)| b)
            .unwrap_or(span.text.len());
        span.text.insert_str(byte, text);
    }
}

/// Delete the characters `from..to`. The list keeps at least one span so
/// the style survives when all the text is removed.
pub fn spans_delete(spans: &mut Vec<TextSpan>, from: usize, to: usize) {
    let total = spans_char_count(spans);
    let (from, to) = (from.min(to).min(total), to.max(from).min(total));
    if from == to {
        return;
    }
    let keep = spans.first().cloned();
    let (head, rest) = split_spans_at(spans, from);
    let (_, tail) = split_spans_at(&rest, to - from);
    let mut out = head;
    out.extend(tail);
    if out.is_empty() {
        if let Some(mut k) = keep {
            k.text.clear();
            out.push(k);
        }
    }
    *spans = merge_equal_spans(out);
}

/// Apply `f` to the style of the characters `from..to`, splitting spans at
/// the range ends and merging neighbours that end up with the same style.
pub fn spans_apply(spans: &mut Vec<TextSpan>, from: usize, to: usize, f: impl Fn(&mut TextSpan)) {
    let total = spans_char_count(spans);
    let (from, to) = (from.min(to).min(total), to.max(from).min(total));
    if from == to {
        return;
    }
    let (head, rest) = split_spans_at(spans, from);
    let (mut mid, tail) = split_spans_at(&rest, to - from);
    for span in &mut mid {
        f(span);
    }
    let mut out = head;
    out.extend(mid);
    out.extend(tail);
    *spans = merge_equal_spans(out);
}

/// Merge neighbouring spans whose style is identical.
pub fn merge_equal_spans(spans: Vec<TextSpan>) -> Vec<TextSpan> {
    let mut out: Vec<TextSpan> = Vec::new();
    for span in spans {
        match out.last_mut() {
            Some(last) if same_style(last, &span) => last.text.push_str(&span.text),
            _ => out.push(span),
        }
    }
    out
}

fn same_style(a: &TextSpan, b: &TextSpan) -> bool {
    let strip = |s: &TextSpan| TextSpan {
        text: String::new(),
        ..s.clone()
    };
    strip(a) == strip(b)
}

/// Paragraph formatting (Text > Tabs, Columns, Bullets, Drop Cap and the
/// Text docker's paragraph section).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParagraphStyle {
    /// Line spacing as a percentage of the font's line height.
    pub leading_pct: f64,
    /// Space added after every character, as a percentage of the width
    /// of a space (-100 to 2000).
    pub char_spacing_pct: f64,
    /// The width of spaces, as a percentage of their own (0 to 2000).
    pub word_spacing_pct: f64,
    /// Space before and after each paragraph, mm.
    pub space_before: f64,
    pub space_after: f64,
    pub first_line_indent: f64,
    pub left_indent: f64,
    pub right_indent: f64,
    pub columns: u32,
    pub gutter: f64,
    pub bullets: bool,
    pub bullet_char: String,
    pub bullet_indent: f64,
    /// Drop cap height in lines; 0 = none.
    pub drop_cap_lines: u32,
    /// Tab stops in mm from the left edge; empty = default every 12.7 mm.
    pub tabs: Vec<f64>,
    pub hyphenate: bool,
    /// Fit text to frame: scale the font so the text fills the frame height.
    pub fit_to_frame: bool,
    /// Align baselines to a grid of this pitch in mm, measured down from the
    /// top of the frame (Text > Align to Baseline Grid); 0 = off. Only
    /// paragraph text snaps.
    pub baseline_grid_mm: f64,
}

impl Default for ParagraphStyle {
    fn default() -> Self {
        ParagraphStyle {
            leading_pct: 100.0,
            char_spacing_pct: 0.0,
            word_spacing_pct: 100.0,
            space_before: 0.0,
            space_after: 0.0,
            first_line_indent: 0.0,
            left_indent: 0.0,
            right_indent: 0.0,
            columns: 1,
            gutter: 5.0,
            bullets: false,
            bullet_char: "\u{2022}".into(),
            bullet_indent: 5.0,
            drop_cap_lines: 0,
            tabs: Vec::new(),
            hyphenate: false,
            fit_to_frame: false,
            baseline_grid_mm: 0.0,
        }
    }
}

/// Table object (Table tool). Cell text is laid out as paragraph text
/// inside the cell rectangle minus padding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub rect: Rect,
    pub col_widths: Vec<f64>,
    pub row_heights: Vec<f64>,
    pub cells: Vec<TableCell>,
    /// Border drawn on every cell edge; `None` = no borders.
    pub border: Option<Stroke>,
    /// Fill of cells without their own fill.
    pub cell_fill: Fill,
    pub padding: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    pub row: u32,
    pub col: u32,
    #[serde(default = "one_u32")]
    pub row_span: u32,
    #[serde(default = "one_u32")]
    pub col_span: u32,
    #[serde(default)]
    pub text: Vec<TextSpan>,
    #[serde(default)]
    pub fill: Option<Fill>,
    #[serde(default)]
    pub align: TextAlign,
}

fn one_u32() -> u32 {
    1
}

impl Table {
    /// A `rows` x `cols` table filling `rect`, equal cells.
    pub fn new(rect: Rect, rows: u32, cols: u32, border: Option<Stroke>) -> Self {
        let rows = rows.max(1);
        let cols = cols.max(1);
        let mut cells = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                cells.push(TableCell {
                    row: r,
                    col: c,
                    row_span: 1,
                    col_span: 1,
                    text: Vec::new(),
                    fill: None,
                    align: TextAlign::Left,
                });
            }
        }
        Table {
            rect,
            col_widths: vec![rect.width() / cols as f64; cols as usize],
            row_heights: vec![rect.height() / rows as f64; rows as usize],
            cells,
            border,
            cell_fill: Fill::None,
            padding: 1.0,
        }
    }

    pub fn rows(&self) -> u32 {
        self.row_heights.len() as u32
    }

    pub fn cols(&self) -> u32 {
        self.col_widths.len() as u32
    }

    /// Left x of column `c` (local space).
    pub fn col_x(&self, c: u32) -> f64 {
        self.rect.x0 + self.col_widths.iter().take(c as usize).sum::<f64>()
    }

    /// Top y of row `r` (rows go down from the top edge; Y is up).
    pub fn row_top(&self, r: u32) -> f64 {
        self.rect.y1 - self.row_heights.iter().take(r as usize).sum::<f64>()
    }

    /// Rectangle of a cell including its spans.
    pub fn cell_rect(&self, cell: &TableCell) -> Rect {
        let x0 = self.col_x(cell.col);
        let x1 = self.col_x(cell.col + cell.col_span);
        let y1 = self.row_top(cell.row);
        let y0 = self.row_top(cell.row + cell.row_span);
        Rect::new(x0, y0, x1, y1)
    }

    /// Cell at a local point, if any.
    pub fn cell_at(&self, p: crate::geometry::Point) -> Option<usize> {
        self.cells
            .iter()
            .position(|c| self.cell_rect(c).contains(p))
    }

    /// Whether a (row, col) is hidden under another cell's span.
    pub fn covered(&self, row: u32, col: u32) -> bool {
        self.cells.iter().any(|c| {
            !(c.row == row && c.col == col)
                && row >= c.row
                && row < c.row + c.row_span
                && col >= c.col
                && col < c.col + c.col_span
        })
    }

    /// Keep `rect` consistent with the column and row sizes.
    pub fn refit(&mut self) {
        let w: f64 = self.col_widths.iter().sum();
        let h: f64 = self.row_heights.iter().sum();
        self.rect = Rect::new(
            self.rect.x0,
            self.rect.y1 - h,
            self.rect.x0 + w,
            self.rect.y1,
        );
    }
}

/// Text fitted to a path: glyphs follow `path` (local space).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextOnPath {
    pub path: BezPath,
    /// Start offset along the path, mm.
    #[serde(default)]
    pub offset: f64,
    /// Distance from the path along its normal, mm.
    #[serde(default)]
    pub distance: f64,
    /// Place text on the other side of the path.
    #[serde(default)]
    pub mirror: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    pub id: ShapeId,
    pub name: Option<String>,
    pub kind: ShapeKind,
    /// Local-to-page transform.
    pub transform: Affine,
    pub fill: Fill,
    pub stroke: Option<Stroke>,
    pub visible: bool,
    pub locked: bool,
    /// 1.0 = opaque, 0.0 = invisible (uniform transparency).
    #[serde(default = "one")]
    pub opacity: f64,
    #[serde(default)]
    pub shadow: Option<Shadow>,
    /// Prepress: print the fill over the colours beneath instead of
    /// knocking them out.
    #[serde(default)]
    pub overprint_fill: bool,
    #[serde(default)]
    pub overprint_outline: bool,
    /// Hyperlink attached to the object (exported to PDF and SVG).
    #[serde(default)]
    pub link: Option<String>,
    /// Object data fields (name/value), like the Object Data docker.
    #[serde(default)]
    pub data: Vec<(String, String)>,
    /// Live effects, evaluated at render time (see `live.rs`).
    #[serde(default)]
    pub effects: Vec<crate::live::Effect>,
    /// Paragraph text flows around this object.
    #[serde(default)]
    pub wrap_text: bool,
}

fn one() -> f64 {
    1.0
}

impl Shape {
    pub fn new(id: ShapeId, kind: ShapeKind) -> Self {
        Shape {
            id,
            name: None,
            kind,
            transform: Affine::IDENTITY,
            fill: Fill::None,
            stroke: Some(Stroke::default()),
            visible: true,
            locked: false,
            opacity: 1.0,
            shadow: None,
            overprint_fill: false,
            overprint_outline: false,
            link: None,
            data: Vec::new(),
            effects: Vec::new(),
            wrap_text: false,
        }
    }

    /// A rectangle's corners as they measure on the page: fixed corners as
    /// stored, the others times the smaller scale of the rectangle's axes.
    /// `None` for other kinds.
    pub fn page_corners(&self) -> Option<Corners> {
        let (_, c) = self.kind.rect_corners()?;
        if c.fixed {
            return Some(c);
        }
        let (sx, sy) = axis_scale(self.transform);
        let k = sx.min(sy);
        Some(Corners {
            radii: c.radii.map(|r| r * k),
            ..c
        })
    }

    /// This rectangle with corners measured on the page (as
    /// `page_corners` gives them), each at most half the shorter side.
    /// `None` for other kinds.
    pub fn with_page_corners(&self, page: Corners) -> Option<ShapeKind> {
        let (rect, _) = self.kind.rect_corners()?;
        let rect = rect.abs();
        let (sx, sy) = axis_scale(self.transform);
        let limit = (rect.width() * sx).min(rect.height() * sy) / 2.0;
        let mut c = page;
        let fit = |r: f64, limit: f64| {
            if r.is_finite() {
                r.clamp(0.0, limit.max(0.0))
            } else {
                0.0
            }
        };
        c.radii = c.radii.map(|r| fit(r, limit));
        if !c.fixed {
            let k = sx.min(sy).max(1e-9);
            let local_limit = rect.width().min(rect.height()) / 2.0;
            c.radii = c.radii.map(|r| fit(r / k, local_limit));
        }
        Some(ShapeKind::rect_with_corners(rect, c))
    }

    /// Put `parent` in front of the shape's own transform (ungrouping,
    /// flattening a group for a file) and keep its look. Fixed corners
    /// measure against the rectangle's own transform, so when `parent`
    /// scales them they first become scaling corners of the same size, or a
    /// curve when the rectangle's axes scale differently.
    pub fn absorb(&mut self, parent: Affine) {
        let next = parent * self.transform;
        if let ShapeKind::Rect {
            rect,
            corners: Some(c),
            ..
        } = &self.kind
        {
            let (sx, sy) = axis_scale(self.transform);
            let (nx, ny) = axis_scale(next);
            let same = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0);
            if c.fixed && !(same(sx, nx) && same(sy, ny)) {
                self.kind = if same(sx, sy) && sx > 1e-9 {
                    let rect = rect.abs();
                    let limit = (rect.width() * sx).min(rect.height() * sy) / 2.0;
                    let mut rel = *c;
                    rel.fixed = false;
                    rel.radii = c.radii.map(|r| {
                        if r.is_finite() {
                            r.clamp(0.0, limit.max(0.0)) / sx
                        } else {
                            0.0
                        }
                    });
                    ShapeKind::rect_with_corners(rect, rel)
                } else {
                    ShapeKind::Path {
                        path: self.local_path(),
                        closed: true,
                    }
                };
            }
        }
        self.transform = next;
    }

    /// A rectangle that one round radius in its own space describes (what
    /// formats with a single radius store): the rectangle and the radius.
    /// `None` for other kinds and for corners that need a path.
    pub fn plain_rect(&self) -> Option<(Rect, f64)> {
        let ShapeKind::Rect {
            rect,
            radius,
            corners,
        } = &self.kind
        else {
            return None;
        };
        let Some(c) = corners else {
            return Some((*rect, *radius));
        };
        if c.is_square() {
            return Some((*rect, 0.0));
        }
        if c.kind != CornerKind::Round || !c.is_uniform() {
            return None;
        }
        if !c.fixed {
            return Some((*rect, c.radii[0]));
        }
        let (sx, sy) = axis_scale(self.transform);
        if sx < 1e-9 || (sx - sy).abs() > 1e-9 * sx.max(sy) {
            return None;
        }
        let limit = (rect.width().abs() * sx).min(rect.height().abs() * sy) / 2.0;
        Some((*rect, c.radii[0].min(limit) / sx))
    }

    /// Outline of the shape in local coordinates. Text and groups return an
    /// approximation (bounds box) until the type engine exists.
    pub fn local_path(&self) -> BezPath {
        match &self.kind {
            ShapeKind::Rect {
                rect,
                radius,
                corners,
            } => match corners {
                Some(c) => c.path(*rect, axis_scale(self.transform)),
                None => geometry::rect_path(*rect, *radius),
            },
            ShapeKind::Ellipse { rect, arc } => match arc {
                None => geometry::ellipse_path(*rect),
                Some(a) => geometry::ellipse_arc_path(*rect, a.start_deg, a.end_deg, a.pie),
            },
            ShapeKind::Polygon {
                rect,
                points,
                sharpness,
                complex,
            } => match complex {
                Some(k) => geometry::complex_star_path(*rect, *points, *k),
                None => geometry::polygon_path(*rect, *points, *sharpness),
            },
            ShapeKind::Path { path, .. } => path.clone(),
            ShapeKind::Text {
                spans,
                origin,
                frame,
                align,
                para,
                on_path,
            } => {
                let req = text_outline::TextRequest {
                    spans,
                    frame: *frame,
                    align: *align,
                    para,
                    on_path: on_path.as_ref(),
                };
                if let Some(p) = text_outline::outline(&req) {
                    if on_path.is_some() {
                        return Affine::translate(origin.to_vec2()) * p;
                    }
                    // Paragraph text hangs from the top of its frame; artistic text sits on its baseline.
                    let shift = match frame {
                        Some(f) => Affine::translate((origin.x, origin.y + f.height)),
                        None => Affine::translate(origin.to_vec2()),
                    };
                    return shift * p;
                }
                // No text engine registered: rough box, 0.5 em per character.
                let size_mm: f64 =
                    spans.iter().map(|s| s.size_pt).fold(0.0, f64::max) * 25.4 / 72.0;
                let chars: usize = spans.iter().map(|s| s.text.chars().count()).sum();
                let w = chars as f64 * size_mm * 0.5;
                Rect::new(origin.x, origin.y, origin.x + w, origin.y + size_mm).to_path(0.01)
            }
            ShapeKind::Group { children } => {
                let mut path = BezPath::new();
                for c in children {
                    path.extend(c.page_path());
                }
                path
            }
            ShapeKind::Bitmap { rect, .. } => rect.to_path(0.01),
            ShapeKind::ClipFrame { frame, .. } => frame.page_path(),
            ShapeKind::Table(t) => t.rect.to_path(0.01),
            ShapeKind::SymbolInstance { .. } => {
                // Bounds come from the symbol through `expand_with`; here a unit box.
                Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.01)
            }
        }
    }

    /// Child shapes a composite object renders as (tables, symbol
    /// instances). Other kinds return an empty list. Children are in
    /// local space; apply this shape's transform.
    pub fn expand(&self, symbols: &[Symbol]) -> Vec<Shape> {
        match &self.kind {
            ShapeKind::Table(t) => {
                let mut out = Vec::new();
                let placeholder = ShapeId(0);
                for cell in &t.cells {
                    if t.covered(cell.row, cell.col) {
                        continue;
                    }
                    let r = t.cell_rect(cell);
                    let fill = cell.fill.clone().unwrap_or_else(|| t.cell_fill.clone());
                    if !matches!(fill, Fill::None) {
                        let mut bg = Shape::new(
                            placeholder,
                            ShapeKind::Rect {
                                rect: r,
                                radius: 0.0,
                                corners: None,
                            },
                        );
                        bg.fill = fill;
                        bg.stroke = None;
                        out.push(bg);
                    }
                    if !cell.text.is_empty() {
                        let inner = r.inset(-t.padding);
                        let mut tx = Shape::new(
                            placeholder,
                            ShapeKind::Text {
                                spans: cell.text.clone(),
                                origin: crate::geometry::Point::new(inner.x0, inner.y0),
                                frame: Some(crate::geometry::Size::new(
                                    inner.width().max(0.1),
                                    inner.height().max(0.1),
                                )),
                                align: cell.align,
                                para: ParagraphStyle::default(),
                                on_path: None,
                            },
                        );
                        tx.fill = cell
                            .text
                            .first()
                            .and_then(|s| s.fill.clone())
                            .unwrap_or(Fill::Solid(crate::Color::BLACK));
                        tx.stroke = None;
                        out.push(tx);
                    }
                }
                if let Some(border) = &t.border {
                    let mut path = BezPath::new();
                    for cell in &t.cells {
                        if t.covered(cell.row, cell.col) {
                            continue;
                        }
                        path.extend(t.cell_rect(cell).to_path(0.01));
                    }
                    let mut b = Shape::new(placeholder, ShapeKind::Path { path, closed: true });
                    b.fill = Fill::None;
                    b.stroke = Some(border.clone());
                    out.push(b);
                }
                out
            }
            ShapeKind::SymbolInstance { index } => symbols
                .get(*index)
                .map(|s| vec![s.shape.clone()])
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Outline in local space, using the symbol table for instances.
    pub fn local_path_with(&self, symbols: &[Symbol]) -> BezPath {
        match &self.kind {
            ShapeKind::SymbolInstance { .. } | ShapeKind::Table(_) => {
                let mut p = BezPath::new();
                for c in self.expand(symbols) {
                    p.extend(c.page_path());
                }
                if p.elements().is_empty() {
                    self.local_path()
                } else {
                    p
                }
            }
            _ => self.local_path(),
        }
    }

    /// Outline in page coordinates (transform applied).
    pub fn page_path(&self) -> BezPath {
        self.transform * self.local_path()
    }

    /// Axis-aligned bounds in page coordinates, ignoring outline width.
    pub fn bounds(&self) -> Rect {
        self.page_path().bounding_box()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub printable: bool,
    pub locked: bool,
    /// Bottom to top drawing order.
    pub shapes: Vec<Shape>,
    /// For master layers: which pages show this layer.
    #[serde(default)]
    pub scope: MasterScope,
}

/// Which pages a master layer appears on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MasterScope {
    #[default]
    All,
    Odd,
    Even,
}

impl MasterScope {
    /// `index` is 0-based; page 1 is odd.
    pub fn applies(self, index: usize) -> bool {
        match self {
            MasterScope::All => true,
            MasterScope::Odd => index.is_multiple_of(2),
            MasterScope::Even => !index.is_multiple_of(2),
        }
    }
}

/// A saved object style (Object Styles docker).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectStyle {
    pub name: String,
    pub fill: Fill,
    pub stroke: Option<Stroke>,
}

/// A named colour (Color Styles docker); `harmony` groups styles that
/// rotate together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorStyle {
    pub name: String,
    pub color: crate::Color,
    #[serde(default)]
    pub harmony: Option<String>,
}

/// A symbol definition; instances reference it by index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub shape: Shape,
}

/// Document properties (File > Document Properties).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Metadata {
    pub author: String,
    pub subject: String,
    pub keywords: String,
    pub copyright: String,
    pub notes: String,
    pub rating: u8,
    /// Rendering resolution for effects, dpi.
    pub resolution_dpi: f64,
    /// Bleed around the page, mm.
    pub bleed: f64,
    pub rgb_profile: String,
    pub cmyk_profile: String,
    /// Colour mode effects (blends, transparencies) and exports default to.
    pub primary_color_mode: PrimaryColorMode,
    pub grid: GridSettings,
    pub rulers: RulerSettings,
    pub guides: GuideSettings,
    /// Fill the open subpaths of curves too (Document Options > General,
    /// off by default).
    pub fill_open_curves: bool,
    /// Document Options > General: Auto inflate bitmaps for effects is on
    /// unless this is set.
    pub no_auto_inflate: bool,
}

impl Metadata {
    /// Bitmap effects that spread past the edges grow the bitmap first.
    pub fn auto_inflate_bitmaps(&self) -> bool {
        !self.no_auto_inflate
    }
}

/// A document's primary colour mode (Create a New Document dialog).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PrimaryColorMode {
    #[default]
    Cmyk,
    Rgb,
}

impl Layer {
    pub fn master(id: LayerId, name: impl Into<String>, scope: MasterScope) -> Self {
        let mut l = Layer::new(id, name);
        l.scope = scope;
        l
    }

    pub fn new(id: LayerId, name: impl Into<String>) -> Self {
        Layer {
            id,
            name: name.into(),
            visible: true,
            printable: true,
            locked: false,
            shapes: Vec::new(),
            scope: MasterScope::All,
        }
    }
}

/// The line of a guideline, in mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "dir", rename_all = "lowercase")]
pub enum GuideLine {
    Horizontal {
        y: f64,
    },
    Vertical {
        x: f64,
    },
    /// A line through `(x, y)` at `angle` degrees (0 = horizontal, CCW positive).
    Angled {
        x: f64,
        y: f64,
        angle: f64,
    },
}

impl GuideLine {
    /// Signed distance from `p` to the line.
    pub fn distance(&self, p: crate::geometry::Point) -> f64 {
        match self {
            GuideLine::Horizontal { y } => p.y - y,
            GuideLine::Vertical { x } => p.x - x,
            GuideLine::Angled { x, y, angle } => {
                let a = angle.to_radians();
                // Normal of the line direction (cos a, sin a).
                -(p.x - x) * a.sin() + (p.y - y) * a.cos()
            }
        }
    }

    /// The line moved so that it passes through `p` (keeps orientation).
    pub fn through(&self, p: crate::geometry::Point) -> GuideLine {
        match *self {
            GuideLine::Horizontal { .. } => GuideLine::Horizontal { y: p.y },
            GuideLine::Vertical { .. } => GuideLine::Vertical { x: p.x },
            GuideLine::Angled { angle, .. } => GuideLine::Angled {
                x: p.x,
                y: p.y,
                angle,
            },
        }
    }

    /// A point on the line and its direction (unit vector).
    pub fn point_and_direction(&self) -> (crate::geometry::Point, crate::geometry::Vec2) {
        use crate::geometry::{Point, Vec2};
        match *self {
            GuideLine::Horizontal { y } => (Point::new(0.0, y), Vec2::new(1.0, 0.0)),
            GuideLine::Vertical { x } => (Point::new(x, 0.0), Vec2::new(0.0, 1.0)),
            GuideLine::Angled { x, y, angle } => {
                let a = angle.to_radians();
                (Point::new(x, y), Vec2::new(a.cos(), a.sin()))
            }
        }
    }

    /// The line turned to `angle` degrees about `pivot`: 0 and 180 give a
    /// horizontal guideline, 90 and 270 a vertical one.
    pub fn rotated_to(&self, angle: f64, pivot: crate::geometry::Point) -> GuideLine {
        let a = angle.rem_euclid(180.0);
        if a.abs() < 1e-9 {
            GuideLine::Horizontal { y: pivot.y }
        } else if (a - 90.0).abs() < 1e-9 {
            GuideLine::Vertical { x: pivot.x }
        } else {
            GuideLine::Angled {
                x: pivot.x,
                y: pivot.y,
                angle: a,
            }
        }
    }

    /// Angle in degrees (0 horizontal, 90 vertical).
    pub fn angle(&self) -> f64 {
        match *self {
            GuideLine::Horizontal { .. } => 0.0,
            GuideLine::Vertical { .. } => 90.0,
            GuideLine::Angled { angle, .. } => angle,
        }
    }
}

/// How a guideline is drawn (the guideline style picker).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GuideStyle {
    Solid,
    #[default]
    Dashed,
    Dotted,
    DashDot,
}

impl GuideStyle {
    pub const ALL: [GuideStyle; 4] = [
        GuideStyle::Solid,
        GuideStyle::Dashed,
        GuideStyle::Dotted,
        GuideStyle::DashDot,
    ];

    /// On and off lengths in screen pixels, repeated along the line; empty
    /// for a solid line.
    pub fn pattern(self) -> &'static [f32] {
        match self {
            GuideStyle::Solid => &[],
            GuideStyle::Dashed => &[4.0, 3.0],
            GuideStyle::Dotted => &[1.0, 2.0],
            GuideStyle::DashDot => &[6.0, 2.0, 1.0, 2.0],
        }
    }
}

/// A guideline on a page: its line, and how it looks. A guideline
/// without its own colour uses the document's default guideline colour.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    #[serde(flatten)]
    pub line: GuideLine,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<crate::Color>,
    #[serde(default)]
    pub style: GuideStyle,
    /// Locked guidelines can be selected but not moved or deleted.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
}

impl From<GuideLine> for Guide {
    fn from(line: GuideLine) -> Self {
        Guide {
            line,
            color: None,
            style: GuideStyle::default(),
            locked: false,
        }
    }
}

impl Guide {
    pub fn horizontal(y: f64) -> Self {
        GuideLine::Horizontal { y }.into()
    }

    pub fn vertical(x: f64) -> Self {
        GuideLine::Vertical { x }.into()
    }

    pub fn angled(x: f64, y: f64, angle: f64) -> Self {
        GuideLine::Angled { x, y, angle }.into()
    }

    /// Signed distance from `p` to the guideline.
    pub fn distance(&self, p: crate::geometry::Point) -> f64 {
        self.line.distance(p)
    }

    /// The guideline moved so that it passes through `p` (keeps
    /// orientation, colour, style and lock).
    pub fn through(&self, p: crate::geometry::Point) -> Guide {
        Guide {
            line: self.line.through(p),
            ..*self
        }
    }

    /// The same guideline with another line.
    pub fn with_line(&self, line: GuideLine) -> Guide {
        Guide { line, ..*self }
    }
}

/// How the document grid is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GridDisplay {
    #[default]
    Lines,
    Dots,
}

/// Document grid, baseline grid and pixel grid (Document Options, Grid).
/// Distances in mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GridSettings {
    /// Distance between the vertical lines of the document grid.
    pub spacing_x: f64,
    /// Distance between the horizontal lines of the document grid.
    pub spacing_y: f64,
    pub display: GridDisplay,
    /// Line spacing of the baseline grid (14 pt by default).
    pub baseline_spacing: f64,
    /// Distance from the page top to the first baseline.
    pub baseline_start: f64,
    pub baseline_color: crate::Color,
    pub pixel_color: crate::Color,
    /// Opacity of the pixel grid, 0 to 1.
    pub pixel_opacity: f64,
}

impl Default for GridSettings {
    fn default() -> Self {
        GridSettings {
            spacing_x: 10.0,
            spacing_y: 10.0,
            display: GridDisplay::Lines,
            baseline_spacing: 14.0 * 25.4 / 72.0,
            baseline_start: 0.5 * 25.4,
            baseline_color: crate::Color::rgb8(0x8F, 0xC7, 0xF0),
            pixel_color: crate::Color::rgb8(0xD9, 0xD9, 0xD9),
            pixel_opacity: 1.0,
        }
    }
}

/// Ruler settings (Document Options, Rulers).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RulerSettings {
    /// The ruler origin, in mm from the page's bottom-left corner.
    pub origin_x: f64,
    pub origin_y: f64,
    /// Number of tick marks between two numbered marks.
    pub tick_divisions: u32,
}

impl Default for RulerSettings {
    fn default() -> Self {
        RulerSettings {
            origin_x: 0.0,
            origin_y: 0.0,
            tick_divisions: 10,
        }
    }
}

/// Guideline defaults (Document Options, Guidelines).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GuideSettings {
    /// Colour of guidelines without their own colour.
    pub color: crate::Color,
    /// Colour given to guidelines added from presets.
    pub preset_color: crate::Color,
}

impl Default for GuideSettings {
    fn default() -> Self {
        GuideSettings {
            color: crate::Color::rgb8(0x00, 0x00, 0xFF),
            preset_color: crate::Color::rgb8(0x00, 0x99, 0xFF),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: PageId,
    pub name: String,
    /// Page size in millimetres.
    pub size: crate::geometry::Size,
    /// Bottom to top drawing order.
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub guides: Vec<Guide>,
    /// Page background: none, a solid colour or a bitmap (PNG, stretched).
    #[serde(default)]
    pub background: Option<Fill>,
}

impl Page {
    pub fn rect(&self) -> Rect {
        Rect::from_origin_size((0.0, 0.0), self.size)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub title: String,
    pub pages: Vec<Page>,
    ids: IdSource,
    /// Master layers, drawn beneath every page's own layers.
    #[serde(default)]
    pub master: Vec<Layer>,
    #[serde(default)]
    pub object_styles: Vec<ObjectStyle>,
    #[serde(default)]
    pub color_styles: Vec<ColorStyle>,
    #[serde(default)]
    pub symbols: Vec<Symbol>,
    #[serde(default)]
    pub metadata: Metadata,
    /// Document-level palette (the bottom "document palette").
    #[serde(default)]
    pub palette: Vec<crate::Color>,
}

impl Default for Document {
    fn default() -> Self {
        Document::new("Untitled", paper::A4)
    }
}

impl Document {
    /// A document with one page and one layer.
    pub fn new(title: impl Into<String>, size: crate::geometry::Size) -> Self {
        let mut ids = IdSource::default();
        let page = Page {
            id: ids.page(),
            name: "Page 1".into(),
            size,
            layers: vec![Layer::new(ids.layer(), "Layer 1")],
            guides: Vec::new(),
            background: None,
        };
        Document {
            title: title.into(),
            pages: vec![page],
            ids,
            master: Vec::new(),
            object_styles: Vec::new(),
            color_styles: Vec::new(),
            symbols: Vec::new(),
            metadata: Metadata {
                resolution_dpi: 300.0,
                ..Default::default()
            },
            palette: Vec::new(),
        }
    }

    pub fn ids_mut(&mut self) -> &mut IdSource {
        &mut self.ids
    }

    pub fn ids(&self) -> &IdSource {
        &self.ids
    }

    /// Replace the id source. Used by the engine so undo never rewinds ids,
    /// which would let a new object reuse the id of an undone one.
    pub fn set_ids(&mut self, ids: IdSource) {
        self.ids = ids;
    }

    pub fn page(&self, id: PageId) -> Result<&Page> {
        self.pages
            .iter()
            .find(|p| p.id == id)
            .ok_or(Error::PageNotFound(id))
    }

    pub fn page_mut(&mut self, id: PageId) -> Result<&mut Page> {
        self.pages
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(Error::PageNotFound(id))
    }

    /// All layers: master layers first, then every page's layers.
    pub fn all_layers(&self) -> impl Iterator<Item = &Layer> {
        self.master
            .iter()
            .chain(self.pages.iter().flat_map(|p| &p.layers))
    }

    pub fn all_layers_mut(&mut self) -> impl Iterator<Item = &mut Layer> {
        self.master
            .iter_mut()
            .chain(self.pages.iter_mut().flat_map(|p| &mut p.layers))
    }

    /// Layers drawn for a page, bottom to top: applicable master layers,
    /// then the page's own layers.
    pub fn layers_for_page(&self, page: PageId) -> Result<Vec<&Layer>> {
        let idx = self
            .pages
            .iter()
            .position(|p| p.id == page)
            .ok_or(Error::PageNotFound(page))?;
        let mut out: Vec<&Layer> = self
            .master
            .iter()
            .filter(|l| l.scope.applies(idx))
            .collect();
        out.extend(self.pages[idx].layers.iter());
        Ok(out)
    }

    pub fn layer(&self, id: LayerId) -> Result<&Layer> {
        self.all_layers()
            .find(|l| l.id == id)
            .ok_or(Error::LayerNotFound(id))
    }

    pub fn layer_mut(&mut self, id: LayerId) -> Result<&mut Layer> {
        self.all_layers_mut()
            .find(|l| l.id == id)
            .ok_or(Error::LayerNotFound(id))
    }

    /// Find a top-level shape and the layer holding it.
    pub fn shape(&self, id: ShapeId) -> Result<(&Layer, &Shape)> {
        for layer in self.all_layers() {
            if let Some(s) = layer.shapes.iter().find(|s| s.id == id) {
                return Ok((layer, s));
            }
        }
        Err(Error::ShapeNotFound(id))
    }

    pub fn shape_mut(&mut self, id: ShapeId) -> Result<&mut Shape> {
        for layer in self.all_layers_mut() {
            if let Some(s) = layer.shapes.iter_mut().find(|s| s.id == id) {
                return Ok(s);
            }
        }
        Err(Error::ShapeNotFound(id))
    }

    /// Position of a shape: (layer id, index inside the layer).
    pub fn locate(&self, id: ShapeId) -> Result<(LayerId, usize)> {
        for layer in self.all_layers() {
            if let Some(i) = layer.shapes.iter().position(|s| s.id == id) {
                return Ok((layer.id, i));
            }
        }
        Err(Error::ShapeNotFound(id))
    }

    /// Find any shape, including children of groups and clip frames.
    pub fn find_shape(&self, id: ShapeId) -> Option<&Shape> {
        fn walk(s: &Shape, id: ShapeId) -> Option<&Shape> {
            if s.id == id {
                return Some(s);
            }
            match &s.kind {
                ShapeKind::Group { children } => children.iter().find_map(|c| walk(c, id)),
                ShapeKind::ClipFrame { frame, contents } => {
                    walk(frame, id).or_else(|| contents.iter().find_map(|c| walk(c, id)))
                }
                _ => None,
            }
        }
        self.all_layers()
            .flat_map(|l| &l.shapes)
            .find_map(|s| walk(s, id))
    }

    /// Union of all shape bounds on a page, or None when the page is empty.
    pub fn content_bounds(&self, page: PageId) -> Result<Option<Rect>> {
        let p = self.page(page)?;
        Ok(p.layers
            .iter()
            .flat_map(|l| &l.shapes)
            .map(Shape::bounds)
            .reduce(|a, b| a.union(b)))
    }

    /// Serialize to the native `.tdraw` JSON format.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(s: &str) -> Result<Self> {
        Ok(serde_json::from_str(s)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn guides_written_before_styles_still_load() {
        let old =
            r#"[{"dir":"horizontal","y":12.5},{"dir":"angled","x":1.0,"y":2.0,"angle":30.0}]"#;
        let guides: Vec<Guide> = serde_json::from_str(old).unwrap();
        assert_eq!(guides[0], Guide::horizontal(12.5));
        assert_eq!(
            guides[1].line,
            GuideLine::Angled {
                x: 1.0,
                y: 2.0,
                angle: 30.0
            }
        );
        assert_eq!(guides[1].style, GuideStyle::Dashed);
        assert!(!guides[1].locked && guides[1].color.is_none());
    }

    #[test]
    fn guide_appearance_round_trips() {
        let g = Guide {
            line: GuideLine::Vertical { x: 4.0 },
            color: Some(crate::Color::rgb8(255, 0, 0)),
            style: GuideStyle::Dotted,
            locked: true,
        };
        let json = serde_json::to_string(&g).unwrap();
        assert!(json.contains("\"dir\":\"vertical\""));
        let back: Guide = serde_json::from_str(&json).unwrap();
        assert_eq!(back, g);
        // Moving keeps the appearance.
        let moved = g.through(Point::new(9.0, 0.0));
        assert_eq!(moved.line, GuideLine::Vertical { x: 9.0 });
        assert!(moved.locked && moved.style == GuideStyle::Dotted);
    }

    #[test]
    fn rotating_a_guide_snaps_to_horizontal_and_vertical() {
        let g = GuideLine::Horizontal { y: 5.0 };
        let p = Point::new(3.0, 5.0);
        assert_eq!(g.rotated_to(90.0, p), GuideLine::Vertical { x: 3.0 });
        assert_eq!(g.rotated_to(180.0, p), GuideLine::Horizontal { y: 5.0 });
        assert_eq!(
            g.rotated_to(-45.0, p),
            GuideLine::Angled {
                x: 3.0,
                y: 5.0,
                angle: 135.0
            }
        );
        let (o, d) = GuideLine::Angled {
            x: 0.0,
            y: 0.0,
            angle: 90.0,
        }
        .point_and_direction();
        assert!(o == Point::ZERO && d.x.abs() < 1e-12 && (d.y - 1.0).abs() < 1e-12);
    }

    #[test]
    fn documents_without_grid_settings_get_the_defaults() {
        let m: Metadata = serde_json::from_str(r#"{"author":"a"}"#).unwrap();
        assert_eq!(m.grid, GridSettings::default());
        assert_eq!(m.rulers.tick_divisions, 10);
        assert!((m.grid.baseline_spacing - 4.938_888).abs() < 1e-5);
        assert_eq!(m.guides, GuideSettings::default());
    }

    #[test]
    fn json_round_trip() {
        let mut doc = Document::default();
        let id = doc.ids_mut().shape();
        let page = doc.pages[0].id;
        let layer = doc.pages[0].layers[0].id;
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 50.0, 30.0),
                radius: 2.0,
                corners: None,
            },
        );
        s.transform = Affine::translate((5.0, 5.0));
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let json = doc.to_json().unwrap();
        let back = Document::from_json(&json).unwrap();
        assert_eq!(doc, back);
        let b = back.content_bounds(page).unwrap().unwrap();
        assert!((b.x0 - 15.0).abs() < 1e-6 && (b.y1 - 35.0).abs() < 1e-6);
        assert_eq!(back.locate(id).unwrap(), (layer, 0));
        let _ = Point::ZERO;
    }

    #[test]
    fn bitmap_effects_round_trip_and_plain_bitmaps_stay_short() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let plain = doc.ids_mut().shape();
        let with_fx = doc.ids_mut().shape();
        let bitmap = |fx| ShapeKind::Bitmap {
            rect: Rect::new(0.0, 0.0, 20.0, 10.0),
            width_px: 4,
            height_px: 2,
            png: vec![1, 2, 3, 4],
            fx,
        };
        let mut params = std::collections::BTreeMap::new();
        params.insert("radius".to_string(), 3.5);
        let stack = BitmapFxStack {
            rect: Rect::new(1.0, 1.0, 19.0, 9.0),
            width_px: 3,
            height_px: 1,
            png: vec![9, 8, 7],
            effects: vec![
                BitmapEffect {
                    id: "gaussian_blur".into(),
                    params,
                    visible: true,
                },
                BitmapEffect {
                    id: "invert_colors".into(),
                    params: Default::default(),
                    visible: false,
                },
            ],
        };
        let l = doc.layer_mut(layer).unwrap();
        l.shapes.push(Shape::new(plain, bitmap(None)));
        l.shapes
            .push(Shape::new(with_fx, bitmap(Some(Box::new(stack)))));
        doc.metadata.no_auto_inflate = true;
        let json = doc.to_json().unwrap();
        let back = Document::from_json(&json).unwrap();
        assert_eq!(back, doc);
        assert!(!back.metadata.auto_inflate_bitmaps());
        // Only the bitmap with effects writes the field.
        assert_eq!(json.matches("\"fx\"").count(), 1, "{json}");
        // Files from before effects were kept apart still load, and an
        // effect written without settings or visibility takes defaults.
        let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let removed = v["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("no_auto_inflate");
        assert_eq!(removed, Some(serde_json::Value::Bool(true)));
        let back = Document::from_json(&v.to_string()).unwrap();
        assert!(back.metadata.auto_inflate_bitmaps());
        let e: BitmapEffect = serde_json::from_str(r#"{"id":"emboss"}"#).unwrap();
        assert!(e.visible && e.params.is_empty());
    }

    #[test]
    fn split_spans_in_the_middle_of_a_span_keeps_styles() {
        let mut bold = TextSpan::new("Hello ", "Sans", 12.0);
        bold.bold = true;
        let mut italic = TextSpan::new("wörld!", "Serif", 10.0);
        italic.italic = true;
        italic.fill = Some(Fill::Solid(crate::Color::rgb8(255, 0, 0)));
        let spans = vec![bold.clone(), italic.clone()];
        assert_eq!(spans_char_count(&spans), 12);

        // Cut inside the second span, after the multi-byte "ö".
        let (head, tail) = split_spans_at(&spans, 8);
        assert_eq!(head.len(), 2);
        assert_eq!(head[0], bold);
        assert_eq!(head[1].text, "wö");
        assert!(head[1].italic && head[1].fill == italic.fill);
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].text, "rld!");
        assert!(tail[0].italic && tail[0].size_pt == 10.0);
        assert_eq!(spans_char_count(&head) + spans_char_count(&tail), 12);
    }

    #[test]
    fn span_editing_inserts_deletes_and_restyles_ranges() {
        let mut bold = TextSpan::new("ab", "Sans", 12.0);
        bold.bold = true;
        let plain = TextSpan::new("cd", "Sans", 12.0);
        let mut spans = vec![bold.clone(), plain.clone()];

        // Insertion takes the style of the character before the caret.
        spans_insert(&mut spans, 2, "X");
        assert_eq!(spans_text(&spans), "abXcd");
        assert_eq!(spans[0].text, "abX");
        spans_insert(&mut spans, 0, "Y");
        assert_eq!(spans[0].text, "YabX");
        spans_insert(&mut spans, 99, "Z");
        assert_eq!(spans_text(&spans), "YabXcdZ");
        assert_eq!(spans[1].text, "cdZ");

        // Deleting across the span boundary keeps both styles.
        spans_delete(&mut spans, 3, 5);
        assert_eq!(spans_text(&spans), "YabdZ");
        assert_eq!(spans.len(), 2);
        assert!(spans[0].bold && !spans[1].bold);

        // Restyling a range splits and merges.
        spans_apply(&mut spans, 1, 4, |s| s.bold = false);
        assert_eq!(spans_text(&spans), "YabdZ");
        assert_eq!(spans.len(), 2, "{spans:?}");
        assert_eq!(spans[0].text, "Y");
        assert_eq!(spans[1].text, "abdZ");
        spans_apply(&mut spans, 0, 5, |s| s.bold = false);
        assert_eq!(spans.len(), 1);

        // Deleting everything keeps one empty span with the style.
        spans_delete(&mut spans, 0, 5);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "");
        assert_eq!(spans[0].font_family, "Sans");
        spans_insert(&mut spans, 0, "new");
        assert_eq!(spans_text(&spans), "new");
    }

    #[test]
    fn split_spans_at_boundaries() {
        let spans = vec![
            TextSpan::new("ab", "Sans", 12.0),
            TextSpan::new("cd", "Sans", 12.0),
        ];
        // Exactly on a span boundary: no span is cut.
        let (head, tail) = split_spans_at(&spans, 2);
        assert_eq!(head, vec![spans[0].clone()]);
        assert_eq!(tail, vec![spans[1].clone()]);
        // At zero: everything goes to the tail.
        let (head, tail) = split_spans_at(&spans, 0);
        assert!(head.is_empty());
        assert_eq!(tail, spans);
        // Past the end: everything stays in the head.
        let (head, tail) = split_spans_at(&spans, 99);
        assert_eq!(head, spans);
        assert!(tail.is_empty());
        // Empty input.
        let (head, tail) = split_spans_at(&[], 3);
        assert!(head.is_empty() && tail.is_empty());
    }

    #[test]
    fn baseline_grid_defaults_to_off_and_loads_from_old_files() {
        assert_eq!(ParagraphStyle::default().baseline_grid_mm, 0.0);
        let p: ParagraphStyle = serde_json::from_str(r#"{"leading_pct":120.0}"#).unwrap();
        assert_eq!(p.baseline_grid_mm, 0.0);
        assert_eq!(p.leading_pct, 120.0);
    }

    fn rect_shape(w: f64, h: f64, corners: Corners) -> Shape {
        Shape::new(
            ShapeId(1),
            ShapeKind::rect_with_corners(Rect::new(0.0, 0.0, w, h), corners),
        )
    }

    fn area(s: &Shape) -> f64 {
        s.page_path().area().abs()
    }

    #[test]
    fn corner_styles_cut_the_expected_area() {
        let (w, h, r) = (40.0, 20.0, 5.0);
        let pi = std::f64::consts::PI;
        for (kind, cut) in [
            (CornerKind::Round, (4.0 - pi) * r * r),
            (CornerKind::Scallop, pi * r * r),
            (CornerKind::Chamfer, 2.0 * r * r),
        ] {
            let s = rect_shape(w, h, Corners::uniform(r, kind));
            let a = area(&s);
            assert!((a - (w * h - cut)).abs() < 0.05, "{kind:?}: {a}");
            // The outline still spans the whole box.
            let b = s.bounds();
            assert!((b.width() - w).abs() < 1e-9 && (b.height() - h).abs() < 1e-9);
        }
        // Equal round corners are the plain radius.
        let plain = rect_shape(w, h, Corners::uniform(r, CornerKind::Round));
        assert!(matches!(plain.kind, ShapeKind::Rect { corners: None, radius, .. } if radius == r));
    }

    #[test]
    fn each_corner_has_its_own_size() {
        let mut c = Corners::uniform(0.0, CornerKind::Chamfer);
        c.radii[Corners::TOP_LEFT] = 6.0;
        let s = rect_shape(30.0, 20.0, c);
        let path = s.local_path();
        // Only the top left corner (x0, y1) is cut.
        assert!(!path.contains(Point::new(0.5, 19.5)));
        for p in [(29.5, 19.5), (0.5, 0.5), (29.5, 0.5)] {
            assert!(path.contains(Point::new(p.0, p.1)), "{p:?}");
        }
        assert!((area(&s) - (600.0 - 18.0)).abs() < 1e-9);
    }

    #[test]
    fn corner_ends_sit_on_the_edges() {
        let mut c = Corners::uniform(5.0, CornerKind::Round);
        c.radii[Corners::BOTTOM_RIGHT] = 0.0;
        let ends = c.ends(Rect::new(0.0, 0.0, 30.0, 20.0), (1.0, 1.0));
        assert_eq!(
            ends[Corners::TOP_LEFT],
            (
                Point::new(0.0, 20.0),
                Point::new(0.0, 15.0),
                Point::new(5.0, 20.0)
            )
        );
        assert_eq!(
            ends[Corners::TOP_RIGHT],
            (
                Point::new(30.0, 20.0),
                Point::new(30.0, 15.0),
                Point::new(25.0, 20.0)
            )
        );
        assert_eq!(
            ends[Corners::BOTTOM_LEFT],
            (
                Point::new(0.0, 0.0),
                Point::new(0.0, 5.0),
                Point::new(5.0, 0.0)
            )
        );
        let p = Point::new(30.0, 0.0);
        assert_eq!(ends[Corners::BOTTOM_RIGHT], (p, p, p));
        // Fixed corners on a rectangle shown twice as wide.
        c.fixed = true;
        let ends = c.ends(Rect::new(0.0, 0.0, 30.0, 20.0), (2.0, 1.0));
        assert_eq!(ends[Corners::TOP_LEFT].2, Point::new(2.5, 20.0));
    }

    #[test]
    fn corners_are_at_most_half_the_shorter_side() {
        let s = rect_shape(40.0, 10.0, Corners::uniform(50.0, CornerKind::Chamfer));
        // Chamfers of 5 mm: four triangles of 12.5 mm2.
        assert!((area(&s) - (400.0 - 50.0)).abs() < 1e-9);
        // Negative and non-finite sizes count as square corners.
        let mut c = Corners::uniform(-3.0, CornerKind::Scallop);
        c.radii[1] = f64::NAN;
        c.radii[2] = f64::INFINITY;
        let s = rect_shape(40.0, 10.0, c);
        assert!(area(&s) <= 400.0 + 1e-9);
        assert!(s.local_path().elements().len() >= 5);
    }

    #[test]
    fn fixed_corners_keep_their_size_when_scaled() {
        let r = 4.0;
        let mut c = Corners::uniform(r, CornerKind::Round);
        c.fixed = true;
        let mut s = rect_shape(20.0, 10.0, c);
        assert!(matches!(
            s.kind,
            ShapeKind::Rect {
                corners: Some(_),
                ..
            }
        ));
        s.transform = Affine::scale_non_uniform(3.0, 2.0);
        let pi = std::f64::consts::PI;
        let a = area(&s);
        assert!((a - (60.0 * 20.0 - (4.0 - pi) * r * r)).abs() < 0.05, "{a}");
        assert_eq!(s.page_corners().unwrap().radii, [r; 4]);
        assert_eq!(s.plain_rect(), None);
        // Scaling corners stretch with the object.
        let mut s = rect_shape(20.0, 10.0, Corners::uniform(r, CornerKind::Round));
        s.transform = Affine::scale_non_uniform(3.0, 2.0);
        let a = area(&s);
        assert!((a - (1200.0 - 6.0 * (4.0 - pi) * r * r)).abs() < 0.1, "{a}");
        assert_eq!(s.page_corners().unwrap().radii, [2.0 * r; 4]);
        assert_eq!(s.plain_rect(), Some((Rect::new(0.0, 0.0, 20.0, 10.0), r)));
    }

    #[test]
    fn corners_set_on_the_page_land_in_the_right_space() {
        let mut s = rect_shape(20.0, 10.0, Corners::default());
        s.transform = Affine::scale(2.0);
        // Scaling corners: 6 mm on the page is 3 in the rectangle's space.
        let kind = s
            .with_page_corners(Corners::uniform(6.0, CornerKind::Round))
            .unwrap();
        assert_eq!(kind.rect_corners().unwrap().1.radii, [3.0; 4]);
        s.kind = kind;
        assert_eq!(s.page_corners().unwrap().radii, [6.0; 4]);
        assert_eq!(s.plain_rect(), Some((Rect::new(0.0, 0.0, 20.0, 10.0), 3.0)));
        // Fixed corners are stored as page sizes, limited by the page box.
        let mut c = Corners::uniform(50.0, CornerKind::Scallop);
        c.fixed = true;
        let kind = s.with_page_corners(c).unwrap();
        assert_eq!(kind.rect_corners().unwrap().1.radii, [10.0; 4]);
        // A fixed round radius on a uniformly scaled rectangle is still one radius.
        c.kind = CornerKind::Round;
        c.radii = [4.0; 4];
        s.kind = s.with_page_corners(c).unwrap();
        assert_eq!(s.plain_rect(), Some((Rect::new(0.0, 0.0, 20.0, 10.0), 2.0)));
        // Other kinds have no corners.
        let e = Shape::new(
            ShapeId(2),
            ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                arc: None,
            },
        );
        assert!(e.page_corners().is_none() && e.with_page_corners(c).is_none());
        assert!(e.plain_rect().is_none());
    }

    #[test]
    fn rectangles_save_their_corners_only_when_they_have_some() {
        let plain = ShapeKind::rect_with_corners(Rect::new(0.0, 0.0, 2.0, 1.0), Corners::default());
        let json = serde_json::to_string(&plain).unwrap();
        assert!(!json.contains("corners"), "{json}");
        let old: ShapeKind = serde_json::from_str(
            r#"{"type":"rect","rect":{"x0":0,"y0":0,"x1":2,"y1":1},"radius":0.25}"#,
        )
        .unwrap();
        assert_eq!(
            old.rect_corners().unwrap().1,
            Corners::uniform(0.25, CornerKind::Round)
        );
        let mut c = Corners::uniform(0.5, CornerKind::Chamfer);
        c.radii[3] = 0.1;
        let k = ShapeKind::rect_with_corners(Rect::new(0.0, 0.0, 2.0, 1.0), c);
        let json = serde_json::to_string(&k).unwrap();
        assert!(
            json.contains("\"chamfer\"") && !json.contains("fixed"),
            "{json}"
        );
        let back: ShapeKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, k);
        c.fixed = true;
        let k = ShapeKind::rect_with_corners(Rect::new(0.0, 0.0, 2.0, 1.0), c);
        let back: ShapeKind = serde_json::from_str(&serde_json::to_string(&k).unwrap()).unwrap();
        assert_eq!(back, k);
    }

    #[test]
    fn complex_stars_save_their_step_and_old_polygons_read() {
        let old: ShapeKind = serde_json::from_str(
            r#"{"type":"polygon","rect":{"x0":0,"y0":0,"x1":2,"y1":2},"points":5,"sharpness":0.5}"#,
        )
        .unwrap();
        assert!(matches!(old, ShapeKind::Polygon { complex: None, .. }));
        assert!(!serde_json::to_string(&old).unwrap().contains("complex"));
        let k = ShapeKind::Polygon {
            rect: Rect::new(0.0, 0.0, 2.0, 2.0),
            points: 9,
            sharpness: 0.0,
            complex: Some(3),
        };
        let back: ShapeKind = serde_json::from_str(&serde_json::to_string(&k).unwrap()).unwrap();
        assert_eq!(back, k);
    }

    #[test]
    fn absorbing_a_parent_keeps_fixed_corners_as_they_look() {
        let mut c = Corners::uniform(3.0, CornerKind::Chamfer);
        c.fixed = true;
        let check = |own: Affine, parent: Affine| -> Shape {
            let mut s = rect_shape(20.0, 10.0, c);
            s.transform = own;
            let before = parent * s.page_path();
            s.absorb(parent);
            let after = s.page_path();
            assert!((before.area() - after.area()).abs() < 1e-6, "{parent:?}");
            let (b, a) = (before.bounding_box(), after.bounding_box());
            assert!(
                (b.x0 - a.x0).abs() < 1e-9 && (b.y1 - a.y1).abs() < 1e-9,
                "{b:?} {a:?}"
            );
            s
        };
        // Evenly scaled rectangle: the same corners, now scaling with it.
        for parent in [Affine::scale(2.0), Affine::scale_non_uniform(2.0, 1.0)] {
            let s = check(Affine::IDENTITY, parent);
            assert!(
                matches!(&s.kind, ShapeKind::Rect { corners: Some(k), .. } if !k.fixed && k.kind == CornerKind::Chamfer && k.radii == [3.0; 4])
            );
        }
        // A stretched rectangle: its outline as a curve.
        let s = check(Affine::scale_non_uniform(2.0, 1.0), Affine::scale(2.0));
        assert!(matches!(s.kind, ShapeKind::Path { closed: true, .. }));
        // Moving and turning change nothing.
        let s = check(
            Affine::scale_non_uniform(2.0, 1.0),
            Affine::rotate(0.5) * Affine::translate((3.0, 4.0)),
        );
        assert!(matches!(&s.kind, ShapeKind::Rect { corners: Some(k), .. } if k.fixed));
    }
}
