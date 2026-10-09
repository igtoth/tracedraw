//! The document model: pages, layers and shapes. Pure data, serde-friendly,
//! no behaviour beyond lookups and bounds. Mutation goes through commands.

use crate::geometry::{self, Affine, BezPath, Rect, Shape as _};
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ShapeKind {
    /// Rectangle in local space; `radius` rounds the corners (mm).
    Rect { rect: Rect, radius: f64 },
    /// Ellipse inscribed in `rect`, in local space; optionally a pie or arc.
    Ellipse {
        rect: Rect,
        #[serde(default)]
        arc: Option<EllipseArc>,
    },
    /// Polygon or star inscribed in `rect`.
    Polygon {
        rect: Rect,
        points: u32,
        sharpness: f64,
    },
    /// Free path (lines and cubic Beziers) in local space.
    Path { path: BezPath, closed: bool },
    /// Artistic text (the target design's single-line text object).
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
    /// A bitmap, PNG-encoded, placed in `rect` (local space).
    Bitmap {
        rect: Rect,
        width_px: u32,
        height_px: u32,
        #[serde(with = "png_bytes")]
        png: Vec<u8>,
    },
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

/// Drop shadow attached to an object (the target design's Drop Shadow tool).
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
    /// Baseline shift in points, positive = up.
    #[serde(default)]
    pub baseline_shift_pt: f64,
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

/// Paragraph formatting (Text > Tabs, Columns, Bullets, Drop Cap and the
/// Text docker's paragraph section).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParagraphStyle {
    /// Line spacing as a percentage of the font's line height.
    pub leading_pct: f64,
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
    /// 1.0 = opaque, 0.0 = invisible (the target design's uniform transparency).
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

    /// Outline of the shape in local coordinates. Text and groups return an
    /// approximation (bounds box) until the type engine exists.
    pub fn local_path(&self) -> BezPath {
        match &self.kind {
            ShapeKind::Rect { rect, radius } => geometry::rect_path(*rect, *radius),
            ShapeKind::Ellipse { rect, arc } => match arc {
                None => geometry::ellipse_path(*rect),
                Some(a) => geometry::ellipse_arc_path(*rect, a.start_deg, a.end_deg, a.pie),
            },
            ShapeKind::Polygon {
                rect,
                points,
                sharpness,
            } => geometry::polygon_path(*rect, *points, *sharpness),
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

/// A guideline on a page, in mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "dir", rename_all = "lowercase")]
pub enum Guide {
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

impl Guide {
    /// Signed distance from `p` to the guide line.
    pub fn distance(&self, p: crate::geometry::Point) -> f64 {
        match self {
            Guide::Horizontal { y } => p.y - y,
            Guide::Vertical { x } => p.x - x,
            Guide::Angled { x, y, angle } => {
                let a = angle.to_radians();
                // Normal of the line direction (cos a, sin a).
                -(p.x - x) * a.sin() + (p.y - y) * a.cos()
            }
        }
    }

    /// The guide moved so that it passes through `p` (keeps orientation).
    pub fn through(&self, p: crate::geometry::Point) -> Guide {
        match *self {
            Guide::Horizontal { .. } => Guide::Horizontal { y: p.y },
            Guide::Vertical { .. } => Guide::Vertical { x: p.x },
            Guide::Angled { angle, .. } => Guide::Angled {
                x: p.x,
                y: p.y,
                angle,
            },
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

    /// Find any shape, including children of groups and ClipFrames.
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
}
