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

    type Bounder = fn(&TextRequest) -> Option<crate::geometry::Rect>;
    static BOUNDER: OnceLock<Bounder> = OnceLock::new();

    /// Register the text engine's box of an outline, kept with the
    /// outline it caches, so the box of a text object costs no copy of
    /// its outline (#802).
    pub fn set_bounds(f: Bounder) {
        let _ = BOUNDER.set(f);
    }

    /// The box of [`outline`]'s result, `None` without a text engine or
    /// for an empty outline.
    pub fn bounds(req: &TextRequest) -> Option<crate::geometry::Rect> {
        BOUNDER.get().and_then(|f| f(req))
    }

    /// One laid-out line: the characters it holds (indexes in the
    /// concatenated span text, the newline ending a paragraph left out)
    /// and where it starts (x and baseline, mm, frame top at y = 0, y up).
    #[derive(Debug, Clone, PartialEq)]
    pub struct Line {
        pub start_char: usize,
        pub end_char: usize,
        pub x: f64,
        pub baseline: f64,
        /// Height above and depth below the baseline, mm.
        pub ascent: f64,
        pub descent: f64,
        /// Pen x before each character from `start_char` to `end_char`,
        /// then after the last one.
        pub edges: Vec<f64>,
        /// The left and right x of each character's own glyph,
        /// `end_char - start_char` entries each (right-to-left text runs
        /// its edges the other way, #629).
        pub starts: Vec<f64>,
        pub ends: Vec<f64>,
    }

    impl Line {
        /// The stretches the characters `a..b` (text indexes) of this line
        /// cover, from left to right, those touching merged: one for text
        /// in one direction, more where directions mix. `out` is cleared
        /// first.
        pub fn stretches(&self, a: usize, b: usize, out: &mut Vec<(f64, f64)>) {
            out.clear();
            let from = a.max(self.start_char) - self.start_char;
            let to = b.min(self.end_char).saturating_sub(self.start_char);
            for i in from..to {
                if let (Some(&x0), Some(&x1)) = (self.starts.get(i), self.ends.get(i)) {
                    merge_stretch(out, x0.min(x1), x0.max(x1));
                }
            }
        }
    }

    /// Add the stretch `x0..x1` to `out`, merged with those it touches,
    /// `out` kept in order of x.
    pub fn merge_stretch(out: &mut Vec<(f64, f64)>, x0: f64, x1: f64) {
        let eps = 1e-6;
        let (mut x0, mut x1) = (x0, x1);
        let mut i = 0;
        while i < out.len() {
            let (a, b) = out[i];
            if a <= x1 + eps && x0 <= b + eps {
                x0 = x0.min(a);
                x1 = x1.max(b);
                out.remove(i);
            } else {
                i += 1;
            }
        }
        let at = out.partition_point(|s| s.0 < x0);
        out.insert(at, (x0, x1));
    }

    type Liner = fn(&TextRequest) -> Vec<Line>;
    static LINER: OnceLock<Liner> = OnceLock::new();

    /// Register the text engine's line breaking.
    pub fn set_lines(f: Liner) {
        let _ = LINER.set(f);
    }

    /// The lines of a text object, when a text engine is registered.
    pub fn lines(req: &TextRequest) -> Option<Vec<Line>> {
        LINER.get().map(|f| f(req))
    }

    /// The font file a glyph comes from, shared with the text engine.
    #[derive(Clone)]
    pub struct GlyphFont {
        pub data: std::sync::Arc<Vec<u8>>,
        /// Face index inside a collection (0 for a single font).
        pub index: u32,
        /// Variation axis values the glyph is drawn at, as (tag, value);
        /// empty for a static font.
        pub coords: Vec<([u8; 4], f32)>,
    }

    impl std::fmt::Debug for GlyphFont {
        // The font data stays out of logs.
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("GlyphFont")
                .field("bytes", &self.data.len())
                .field("index", &self.index)
                .field("coords", &self.coords)
                .finish()
        }
    }

    /// One glyph of laid-out text, as a font file draws it.
    #[derive(Clone, Debug)]
    pub struct PlacedGlyph {
        pub font: GlyphFont,
        pub glyph_id: u16,
        /// From the glyph's own space in ems (pen at the origin, y up) to
        /// layout space (mm): size, position, shifts, angles, slants and
        /// the turn a path gives it.
        pub transform: crate::geometry::Affine,
        /// The characters the glyph stands for (a ligature holds several);
        /// empty for the further glyphs of one character.
        pub text: String,
        /// The span it belongs to.
        pub span: usize,
        /// Its outline as drawn, layout space (mm).
        pub path: BezPath,
        /// False when the drawn outline is not the font's own glyph
        /// (synthetic bold); such glyphs are drawn as curves.
        pub exact: bool,
    }

    /// Laid-out text as glyphs, for writers that keep text as text: the
    /// glyphs, and every other mark of the text (underlines, strike lines,
    /// hyphens added at line breaks, letters no font has) as curves.
    #[derive(Clone, Debug, Default)]
    pub struct GlyphLayout {
        pub glyphs: Vec<PlacedGlyph>,
        pub rest: BezPath,
    }

    type Glypher = fn(&TextRequest) -> GlyphLayout;
    static GLYPHER: OnceLock<Glypher> = OnceLock::new();

    /// Register the text engine's glyph layout.
    pub fn set_glyphs(f: Glypher) {
        let _ = GLYPHER.set(f);
    }

    /// The glyphs of a text object, when a text engine is registered.
    pub fn glyphs(req: &TextRequest) -> Option<GlyphLayout> {
        GLYPHER.get().map(|f| f(req))
    }
}

/// The glyphs of a text object in its local space (apply its transform),
/// placed as [`Shape::local_path`] places its outline. `None` for other
/// shapes and without a text engine.
pub fn text_glyph_layout(kind: &ShapeKind) -> Option<text_outline::GlyphLayout> {
    let ShapeKind::Text {
        spans,
        origin,
        frame,
        align,
        para,
        on_path,
    } = kind
    else {
        return None;
    };
    let mut layout = text_outline::glyphs(&text_outline::TextRequest {
        spans,
        frame: *frame,
        align: *align,
        para,
        on_path: on_path.as_ref(),
    })?;
    let shift = text_layout_shift(*origin, *frame, on_path.is_some(), para.vertical);
    for g in &mut layout.glyphs {
        g.transform = shift * g.transform;
        g.path.apply_affine(shift);
    }
    layout.rest.apply_affine(shift);
    Some(layout)
}

/// From the text engine's layout space to a text's local space: paragraph
/// text hangs from the top left of its frame (vertical text from the top
/// right), artistic text sits on its first baseline at its origin
/// (vertical text starts at the top of its first column there); text on
/// a path is placed at the origin.
pub fn text_layout_shift(
    origin: Point,
    frame: Option<geometry::Size>,
    on_path: bool,
    vertical: bool,
) -> Affine {
    match frame {
        Some(f) if !on_path => {
            let dx = if vertical { f.width } else { 0.0 };
            Affine::translate((origin.x + dx, origin.y + f.height))
        }
        _ => Affine::translate(origin.to_vec2()),
    }
}

/// How the written lines of vertical text (as the text engine reports
/// them, laid out left to right and down) sit in its layout space: a
/// quarter turn clockwise, so a line's run to the right goes down a
/// column and its lines going down become columns going left.
pub fn vertical_line_frame() -> Affine {
    Affine::new([0.0, -1.0, 1.0, 0.0, 0.0, 0.0])
}

/// The rectangles behind a text, in its local space (apply its
/// transform): the paragraph frame's background first, then one per piece
/// of a span with a background on each line, from descent to ascent.
/// Empty for other shapes, text on a path and without a text engine.
pub fn text_backgrounds(kind: &ShapeKind) -> Vec<(Rect, Fill)> {
    let ShapeKind::Text {
        spans,
        origin,
        frame,
        align,
        para,
        on_path: None,
    } = kind
    else {
        return Vec::new();
    };
    let frame_bg = match (frame, &para.frame_fill) {
        (Some(f), Some(fill)) => Some((
            Rect::new(origin.x, origin.y, origin.x + f.width, origin.y + f.height),
            fill.clone(),
        )),
        _ => None,
    };
    if spans.iter().all(|s| s.background.is_none()) {
        return frame_bg.into_iter().collect();
    }
    let Some(lines) = text_outline::lines(&text_outline::TextRequest {
        spans,
        frame: *frame,
        align: *align,
        para,
        on_path: None,
    }) else {
        return Vec::new();
    };
    // The written lines to local space, as the outline is placed (the
    // columns of vertical text are the lines turned).
    let to_local = text_layout_shift(*origin, *frame, false, para.vertical)
        * if para.vertical {
            vertical_line_frame()
        } else {
            Affine::IDENTITY
        };
    let mut out: Vec<(Rect, Fill)> = frame_bg.into_iter().collect();
    let mut stretches = Vec::new();
    let mut span_start = 0usize;
    for sp in spans {
        let n = sp.char_count();
        let (from, to) = (span_start, span_start + n);
        span_start = to;
        let Some(fill) = &sp.background else {
            continue;
        };
        for line in &lines {
            let a = from.max(line.start_char);
            let b = to.min(line.end_char);
            if b <= a {
                continue;
            }
            // The stretches its characters cover: right-to-left text and
            // mixed directions too (#629).
            line.stretches(a, b, &mut stretches);
            for &(x0, x1) in &stretches {
                let r = Rect::new(
                    x0,
                    line.baseline - line.descent,
                    x1,
                    line.baseline + line.ascent,
                );
                out.push(((to_local * r.to_path(0.01)).bounding_box(), fill.clone()));
            }
        }
    }
    out
}

/// Paragraph text as one artistic text a laid-out line, each placed where
/// its frame put it (alignment and justification spacing aside), the lines
/// that overflow the frame left out. `None` for other shapes or without a
/// text engine. Formats that hold no frames use it to keep the look.
pub fn paragraph_lines(kind: &ShapeKind) -> Option<Vec<ShapeKind>> {
    let ShapeKind::Text {
        spans,
        origin,
        frame: Some(frame),
        align,
        para,
        on_path: None,
    } = kind
    else {
        return None;
    };
    let lines = text_outline::lines(&text_outline::TextRequest {
        spans,
        frame: Some(*frame),
        align: *align,
        para,
        on_path: None,
    })?;
    // The written lines to local space (vertical text: the columns run
    // down the frame's height, so the lines go across its width).
    let to_local = text_layout_shift(*origin, Some(*frame), false, para.vertical)
        * if para.vertical {
            vertical_line_frame()
        } else {
            Affine::IDENTITY
        };
    let depth = if para.vertical {
        frame.width
    } else {
        frame.height
    };
    let mut out = Vec::new();
    for line in &lines {
        // Below the frame: overflow, shown in the next linked frame or not
        // at all.
        if line.baseline < -depth - 1e-6 {
            break;
        }
        let (_, tail) = split_spans_at(spans, line.start_char);
        let (mut piece, _) = split_spans_at(&tail, line.end_char.saturating_sub(line.start_char));
        for sp in piece.iter_mut() {
            sp.text = sp.text.replace('\n', "");
            // Each line is placed already: one artistic line of its own.
            sp.paragraph = None;
        }
        piece.retain(|sp| !sp.text.is_empty());
        if piece.iter().all(|sp| sp.text.trim().is_empty()) {
            continue;
        }
        out.push(ShapeKind::Text {
            spans: piece,
            origin: to_local * Point::new(line.x, line.baseline),
            frame: None,
            align: TextAlign::Left,
            para: ParagraphStyle {
                first_line_indent: 0.0,
                left_indent: 0.0,
                right_indent: 0.0,
                ..para.clone()
            },
            on_path: None,
        });
    }
    Some(out)
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

    /// The corners a new rectangle starts with: square, round once given a
    /// size, and keeping that size when the rectangle is resized (Relative
    /// corner scaling off). Drawings read from files keep what they store:
    /// a missing `fixed` there still means scaling corners.
    pub fn tool_default() -> Corners {
        Corners {
            fixed: true,
            ..Corners::default()
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
    /// that they keep their size on the page. The two corners on a side
    /// together are at most as long as the side: when they would be
    /// longer, every corner shrinks by the same factor, so equal corners
    /// are at most half the shorter side and a single corner can take a
    /// whole side.
    fn layout(&self, rect: Rect, scale: (f64, f64)) -> [CornerAt; 4] {
        let rect = rect.abs();
        let (sx, sy) = if self.fixed {
            (scale.0.abs().max(1e-9), scale.1.abs().max(1e-9))
        } else {
            (1.0, 1.0)
        };
        // Sizes in the units the radii are in, then each corner as an
        // ellipse in local space.
        let (w, h) = (rect.width() * sx, rect.height() * sy);
        let radius = |i: usize| {
            let r = self.radii[i];
            if r.is_finite() {
                r.max(0.0)
            } else {
                0.0
            }
        };
        let fit = |side: f64, a: usize, b: usize| {
            let sum = radius(a) + radius(b);
            if sum > side.max(0.0) && sum > 0.0 {
                side.max(0.0) / sum
            } else {
                1.0
            }
        };
        let shrink = fit(w, Self::TOP_LEFT, Self::TOP_RIGHT)
            .min(fit(w, Self::BOTTOM_LEFT, Self::BOTTOM_RIGHT))
            .min(fit(h, Self::TOP_LEFT, Self::BOTTOM_LEFT))
            .min(fit(h, Self::TOP_RIGHT, Self::BOTTOM_RIGHT));
        let (x0, y0, x1, y1) = (rect.x0, rect.y0, rect.x1, rect.y1);
        let at = |index: usize, p: Point, d_in: Vec2, d_out: Vec2| {
            let r = radius(index) * shrink;
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
    /// sides cross (`sharpness` and `twist` are then unused). `twist`
    /// turns the points between the vertices about the centre (radians,
    /// counter-clockwise): a twisted star.
    Polygon {
        rect: Rect,
        points: u32,
        sharpness: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        complex: Option<u32>,
        #[serde(default, skip_serializing_if = "is_zero")]
        twist: f64,
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
        png: crate::SharedBytes,
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
    pub png: crate::SharedBytes,
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
    /// How the shadow combines with what lies beneath. Shadows saved
    /// before the field existed were drawn normally and keep that; new
    /// shadows multiply.
    #[serde(default)]
    pub merge: crate::live::MergeMode,
    /// Which side of the edge the feathering spreads to.
    #[serde(default, skip_serializing_if = "is_default")]
    pub feather_direction: crate::shadow::FeatherDirection,
    /// The shape of the feathered ramp.
    #[serde(default, skip_serializing_if = "is_default")]
    pub feather_edges: crate::shadow::FeatherEdges,
    /// A perspective shadow laid from one of the object's edges; `None`:
    /// a flat shadow moved by `offset`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perspective: Option<crate::shadow::Perspective>,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

impl Default for Shadow {
    fn default() -> Self {
        Shadow {
            offset: crate::geometry::Vec2::new(2.0, -2.0),
            opacity: 0.5,
            blur: 1.5,
            color: crate::color::Color::BLACK,
            merge: crate::live::MergeMode::Multiply,
            feather_direction: crate::shadow::FeatherDirection::Middle,
            feather_edges: crate::shadow::FeatherEdges::Linear,
            perspective: None,
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
    /// Full justification: every line but a paragraph's last fills the
    /// width.
    Justify,
    /// No alignment: lines start at the left edge, as Left does, and
    /// characters keep any offsets they were given.
    None,
    /// Forced justification: the last line of each paragraph fills the
    /// width too.
    #[serde(rename = "force_justify")]
    ForceJustify,
}

impl TextAlign {
    /// Every alignment in the order the alignment buttons show them.
    pub const ALL: [TextAlign; 6] = [
        TextAlign::None,
        TextAlign::Left,
        TextAlign::Center,
        TextAlign::Right,
        TextAlign::Justify,
        TextAlign::ForceJustify,
    ];

    /// Lines are spread over the width (full or forced justification).
    pub fn is_justified(self) -> bool {
        matches!(self, TextAlign::Justify | TextAlign::ForceJustify)
    }
}

/// A value set for one axis of a variable font (#870): the axis's
/// OpenType tag ("wght", "wdth", "slnt", "opsz" or a font's own) and a
/// value in the axis's units, drawn clamped to the font's range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontVariation {
    pub tag: String,
    pub value: f64,
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
    /// The characters' background (a highlight behind each character's
    /// box, from descent to ascent); `None` = no background.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Fill>,
    /// Per-span outline override (character outline); `None` = the
    /// object's outline. An outline of width 0 or less takes the
    /// object's outline off these characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline: Option<Stroke>,
    /// Laid out but not drawn: set only on the parts a text with its own
    /// character colours is drawn in (see [`Shape::text_parts`]), never
    /// saved.
    #[serde(skip)]
    pub layout_only: bool,
    /// OpenType features to enable, e.g. "liga", "smcp", "frac".
    #[serde(default)]
    pub features: Vec<String>,
    /// Axis values of a variable font set for these characters (#870);
    /// an axis not listed takes the value the text's weight and slant
    /// give, else the font's default.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variations: Vec<FontVariation>,
    /// The formatting of the paragraph these characters are in, when it
    /// has its own (see [`crate::paragraphs`]); `None` = the text
    /// object's alignment and paragraph style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paragraph: Option<Box<crate::paragraphs::ParagraphFormat>>,
}

/// Serde: leave a zero out of the file.
fn is_zero(v: &f64) -> bool {
    *v == 0.0
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
            background: None,
            outline: None,
            layout_only: false,
            features: Vec::new(),
            variations: Vec::new(),
            paragraph: None,
        }
    }

    /// The value set for variable font axis `tag`, if any.
    pub fn variation(&self, tag: &str) -> Option<f64> {
        self.variations
            .iter()
            .find(|v| v.tag == tag)
            .map(|v| v.value)
    }

    /// Set axis `tag` to `value`, or take it off (`None`); a value that
    /// is not finite takes it off too.
    pub fn set_variation(&mut self, tag: &str, value: Option<f64>) {
        match value.filter(|v| v.is_finite()) {
            Some(value) => match self.variations.iter_mut().find(|v| v.tag == tag) {
                Some(v) => v.value = value,
                None => self.variations.push(FontVariation {
                    tag: tag.to_string(),
                    value,
                }),
            },
            None => self.variations.retain(|v| v.tag != tag),
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
    let (mut i, mut cut) = match target {
        Some(t) => t,
        None => match spans.len() {
            0 => return,
            n => (n - 1, spans[n - 1].char_count()),
        },
    };
    // At the start of a paragraph with formatting other than the one
    // before it, the text joins the paragraph's first character, so the
    // paragraph keeps its format.
    if at > 0 && at < total {
        let before_nl = spans
            .get(i)
            .zip(cut.checked_sub(1))
            .and_then(|(s, c)| s.text.chars().nth(c))
            == Some('\n');
        if before_nl {
            let mut seen = 0usize;
            for (k, span) in spans.iter().enumerate() {
                let n = span.char_count();
                if at < seen + n {
                    if spans.get(i).map(|s| &s.paragraph) != Some(&span.paragraph) {
                        i = k;
                        cut = at - seen;
                    }
                    break;
                }
                seen += n;
            }
        }
    }
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
    /// Line spacing, in `leading_unit`: a percentage of the font's line
    /// height (the default), points, or a percentage of the font size.
    pub leading_pct: f64,
    /// What `leading_pct` measures.
    pub leading_unit: LeadingUnit,
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
    /// Align baselines to a grid of this pitch in mm (Text > Align to
    /// Baseline Grid); 0 = off. Only paragraph text snaps. The grid is the
    /// page's (from its top and the grid's start offset) for a frame that
    /// is not turned, else it is measured down from the frame top.
    pub baseline_grid_mm: f64,
    /// Where paragraph text sits in its frame from top to bottom.
    pub vertical_align: VerticalAlign,
    /// Vertical text, as CJK typesetting sets it: the lines become
    /// columns read top to bottom and placed right to left, ideographs,
    /// kana and hangul stand upright, Latin letters and digits lie on
    /// their side (turned a quarter clockwise). A frame's columns run
    /// down its height; the text starts at the top right. Text fitted to
    /// a path ignores it. See docs/behavior/text.md, Vertical text.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub vertical: bool,
    /// The paragraph frame's background; `None` = transparent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_fill: Option<Fill>,
    /// What the frame's text wraps around and where the page's baseline
    /// grid falls, worked out from the drawing (see
    /// [`crate::text_flow`]); not saved.
    #[serde(skip)]
    pub flow: crate::text_flow::FrameFlow,
}

/// The unit of a paragraph's line spacing (the Text docker's list next to
/// the line spacing field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeadingUnit {
    /// Percent of the character height (the font's line height).
    #[default]
    PercentCharHeight,
    /// Points from baseline to baseline.
    Points,
    /// Percent of the font size.
    PercentPointSize,
}

impl LeadingUnit {
    pub const ALL: [LeadingUnit; 3] = [
        LeadingUnit::PercentCharHeight,
        LeadingUnit::Points,
        LeadingUnit::PercentPointSize,
    ];
}

impl ParagraphStyle {
    /// The distance from one baseline to the next, mm, for a line whose
    /// font has `line_height_mm` from ascent to descent plus gap at a size
    /// of `size_mm`.
    pub fn line_step(&self, line_height_mm: f64, size_mm: f64) -> f64 {
        match self.leading_unit {
            LeadingUnit::PercentCharHeight => line_height_mm * self.leading_pct / 100.0,
            LeadingUnit::Points => self.leading_pct * 25.4 / 72.0,
            LeadingUnit::PercentPointSize => size_mm * self.leading_pct / 100.0,
        }
    }
}

/// Where paragraph text sits in its frame (the Text docker's Frame
/// section): at the top, in the middle or at the bottom of the room its
/// lines leave. Text that overflows the frame starts at the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlign {
    #[default]
    Top,
    Center,
    Bottom,
}

impl Default for ParagraphStyle {
    fn default() -> Self {
        ParagraphStyle {
            leading_pct: 100.0,
            leading_unit: LeadingUnit::PercentCharHeight,
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
            vertical_align: VerticalAlign::Top,
            vertical: false,
            frame_fill: None,
            flow: crate::text_flow::FrameFlow::default(),
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
    /// Margin between a cell's edges and its text on every side, mm
    /// (used when `margins` is not set).
    pub padding: f64,
    /// Margins per side; `None` = `padding` on every side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margins: Option<crate::table::CellMargins>,
    /// Separated borders: horizontal and vertical space between the cells'
    /// boxes, mm; `None` = cells share their borders.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing: Option<[f64; 2]>,
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
    /// Border of each side; the table's border where unset.
    #[serde(default, skip_serializing_if = "crate::table::CellBorders::is_default")]
    pub borders: crate::table::CellBorders,
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
                cells.push(crate::table::blank_cell(r, c));
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
            margins: None,
            spacing: None,
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

/// How the letters of a text on a path follow it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PathTextOrientation {
    /// Each letter turned to the path's direction.
    #[default]
    Rotate,
    /// Upright strokes stay vertical; the letter's width leans with the path.
    VerticalSkew,
    /// Horizontal strokes stay horizontal; the letter's height leans with
    /// the path's normal.
    HorizontalSkew,
    /// Letters stay upright, only moved along the path.
    Upright,
}

/// Which line of the letters sits on the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PathTextVertical {
    #[default]
    Baseline,
    Ascender,
    Descender,
    Center,
}

/// Text fitted to a path: glyphs follow `path` (local space). The letters
/// stand on the path's [`baseline`](TextOnPath::baseline).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TextOnPath {
    pub path: BezPath,
    /// Start offset along the baseline, mm (with centred or right-aligned
    /// text, added to the alignment's own start).
    #[serde(default)]
    pub offset: f64,
    /// Distance from the path to the baseline, mm, towards the letters'
    /// tops (to the left of the path's direction).
    #[serde(default)]
    pub distance: f64,
    /// Place the text on the other side of the path, reading the other
    /// way (the path is followed from its end back to its start).
    #[serde(default)]
    pub mirror: bool,
    /// Mirror horizontally: the text reads backwards along the path, its
    /// letters mirrored (combined with `mirror`, which is both mirrors).
    #[serde(default)]
    pub mirror_h: bool,
    /// Mirror vertically: the letters flipped to the other side of the
    /// path, upside down (combined with `mirror`).
    #[serde(default)]
    pub mirror_v: bool,
    #[serde(default)]
    pub orientation: PathTextOrientation,
    #[serde(default)]
    pub vertical: PathTextVertical,
}

impl TextOnPath {
    /// The horizontal and vertical mirrors in effect (`mirror` is both).
    pub fn mirrors(&self) -> (bool, bool) {
        (self.mirror ^ self.mirror_h, self.mirror ^ self.mirror_v)
    }

    /// The same text with its mirrors set to `h` and `v` (stored in the
    /// separate fields, `mirror` cleared).
    pub fn set_mirrors(&mut self, h: bool, v: bool) {
        self.mirror = false;
        self.mirror_h = h;
        self.mirror_v = v;
    }

    /// Move the text `distance` from the path, keeping it where it is along
    /// the path: the offset is measured along the baseline, which grows
    /// or shrinks with the distance (round a circle, with its radius), so
    /// it is scaled with the baseline's length. Text centred on the bottom
    /// of a circle stays centred there as it moves out or in (#193).
    pub fn set_distance(&mut self, distance: f64) {
        let before = self.baseline().length();
        self.distance = distance;
        let after = self.baseline().length();
        if before > 1e-9 && after.is_finite() {
            self.offset *= after / before;
        }
    }

    /// The letters are drawn flipped across the baseline (one mirror on).
    pub fn flipped(&self) -> bool {
        let (h, v) = self.mirrors();
        h ^ v
    }

    /// The line the letters stand on: the first subpath of `path`,
    /// followed backwards when mirrored horizontally, moved `distance` to
    /// the side the letters' tops point to. Offsets and advances are
    /// measured along it.
    pub fn baseline(&self) -> Baseline {
        let (h, _) = self.mirrors();
        let d = if self.flipped() {
            -self.distance
        } else {
            self.distance
        };
        Baseline::new(&self.path, d, h)
    }

    /// The path itself as the text follows it (no distance), for finding
    /// where along and how far from it a point is.
    pub fn guide(&self) -> Baseline {
        Baseline::new(&self.path, 0.0, self.mirrors().0)
    }
}

/// The baseline of a text on a path: a polyline with the arc length at
/// each point.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Baseline {
    points: Vec<Point>,
    lengths: Vec<f64>,
    closed: bool,
}

impl Baseline {
    /// Flattening tolerance, mm.
    const TOLERANCE: f64 = 0.05;
    /// Longest corner extension when moving a polyline sideways, as a
    /// multiple of the distance.
    const MITER_LIMIT: f64 = 4.0;

    /// The first subpath of `path` as a polyline, reversed when asked,
    /// then moved `distance` to its left (corners mitred).
    pub fn new(path: &BezPath, distance: f64, reverse: bool) -> Baseline {
        let mut pts: Vec<Point> = Vec::new();
        let mut closed = false;
        let mut done = false;
        geometry::flatten(path, Self::TOLERANCE, &mut |el| {
            if done {
                return;
            }
            match el {
                geometry::PathEl::MoveTo(p) if pts.is_empty() => pts.push(p),
                geometry::PathEl::MoveTo(_) => done = true,
                geometry::PathEl::LineTo(p) => pts.push(p),
                geometry::PathEl::ClosePath => {
                    closed = true;
                    done = true;
                }
                _ => {}
            }
        });
        pts.retain(|p| p.x.is_finite() && p.y.is_finite());
        // Repeated points have no direction.
        pts.dedup_by(|a, b| (*a - *b).hypot() < 1e-9);
        if let (Some(&first), Some(&last)) = (pts.first(), pts.last()) {
            if pts.len() > 2 && (first - last).hypot() < 1e-6 {
                closed = true;
                pts.pop();
            }
            if closed && pts.len() > 2 {
                pts.push(first);
            }
        }
        closed &= pts.len() > 3;
        if reverse {
            pts.reverse();
        }
        if distance.is_finite() && distance != 0.0 && pts.len() >= 2 {
            pts = Self::moved_left(&pts, distance, closed);
        }
        let mut lengths = Vec::with_capacity(pts.len());
        let mut total = 0.0;
        for (i, p) in pts.iter().enumerate() {
            if let Some(prev) = i.checked_sub(1).and_then(|j| pts.get(j)) {
                total += (*p - *prev).hypot();
            }
            lengths.push(total);
        }
        Baseline {
            points: pts,
            lengths,
            closed,
        }
    }

    /// Each point moved `d` to the left of the polyline, along the mean
    /// of the neighbouring segments' normals, far enough to keep the
    /// segments `d` away.
    fn moved_left(pts: &[Point], d: f64, closed: bool) -> Vec<Point> {
        let normal = |a: Point, b: Point| -> Vec2 {
            let v = b - a;
            let len = v.hypot();
            if len > 0.0 {
                Vec2::new(-v.y / len, v.x / len)
            } else {
                Vec2::ZERO
            }
        };
        let n = pts.len();
        // Normal of segment i (from point i to point i + 1).
        let seg: Vec<Vec2> = pts.windows(2).map(|w| normal(w[0], w[1])).collect();
        let mut out = Vec::with_capacity(n);
        for (i, p) in pts.iter().enumerate() {
            let before = if i > 0 {
                seg.get(i - 1)
            } else if closed {
                seg.last()
            } else {
                None
            };
            let after = if i + 1 < n {
                seg.get(i)
            } else if closed {
                seg.first()
            } else {
                None
            };
            let (m, reach) = match (before, after) {
                (Some(a), Some(b)) => {
                    let sum = *a + *b;
                    let len = sum.hypot();
                    if len < 1e-9 {
                        (*a, 1.0)
                    } else {
                        let m = sum / len;
                        let cos = m.dot(*a).max(1.0 / Self::MITER_LIMIT);
                        (m, 1.0 / cos)
                    }
                }
                (Some(a), None) | (None, Some(a)) => (*a, 1.0),
                (None, None) => (Vec2::ZERO, 1.0),
            };
            out.push(*p + m * (d * reach));
        }
        out
    }

    /// Length from start to end.
    pub fn length(&self) -> f64 {
        self.lengths.last().copied().unwrap_or(0.0)
    }

    /// The arc length of the point nearest `p` and how far `p` is from
    /// it, positive to the left of the direction (where letters stand).
    pub fn nearest(&self, p: Point) -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64, f64)> = None;
        for (i, w) in self.points.windows(2).enumerate() {
            let v = w[1] - w[0];
            let len2 = v.hypot2();
            if len2 <= 0.0 {
                continue;
            }
            let t = ((p - w[0]).dot(v) / len2).clamp(0.0, 1.0);
            let q = w[0] + v * t;
            let d = (p - q).hypot();
            if !matches!(best, Some((_, _, bd)) if bd <= d) {
                let side = v.cross(p - q).signum();
                best = Some((self.lengths[i] + t * len2.sqrt(), d * side, d));
            }
        }
        best.map(|(s, d, _)| (s, d))
    }

    /// The path is closed: positions past the end continue from the start.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// The point at arc length `s` and the unit direction there. Positions
    /// wrap around a closed baseline and stop at the ends of an open one.
    pub fn at(&self, s: f64) -> Option<(Point, Vec2)> {
        let total = self.length();
        if self.points.len() < 2 || total <= 0.0 || !s.is_finite() {
            return None;
        }
        let s = if self.closed {
            s.rem_euclid(total)
        } else {
            s.clamp(0.0, total)
        };
        let i = self
            .lengths
            .partition_point(|c| *c <= s)
            .saturating_sub(1)
            .min(self.points.len() - 2);
        let (a, b) = (*self.points.get(i)?, *self.points.get(i + 1)?);
        let start = *self.lengths.get(i)?;
        let v = b - a;
        let len = v.hypot();
        if len <= 0.0 {
            return None;
        }
        let t = ((s - start) / len).clamp(0.0, 1.0);
        Some((a + v * t, v / len))
    }

    /// The shortest distance from `p` to the baseline.
    pub fn distance_to(&self, p: Point) -> Option<f64> {
        self.points
            .windows(2)
            .map(|w| {
                let v = w[1] - w[0];
                let len2 = v.hypot2();
                let t = if len2 > 0.0 {
                    ((p - w[0]).dot(v) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (p - (w[0] + v * t)).hypot()
            })
            .min_by(|a, b| a.total_cmp(b))
    }
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
    /// A text that shows the number of the page it is drawn on (Layout >
    /// Insert Page Number on a master layer, #807).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_number: Option<PageNumberField>,
    /// Live effects, evaluated at render time (see `live.rs`).
    #[serde(default)]
    pub effects: Vec<crate::live::Effect>,
    /// Paragraph text flows around this object (see
    /// [`crate::text_flow`]).
    #[serde(default)]
    pub wrap_text: bool,
    /// How paragraph text flows around it when `wrap_text` is on.
    #[serde(
        default,
        skip_serializing_if = "crate::text_flow::TextWrap::is_default"
    )]
    pub wrap_style: crate::text_flow::TextWrap,
    /// The object style it is linked to ([`ObjectStyle::id`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<u32>,
    /// A curve's node types as chosen (node element index to type), kept
    /// where its handles alone would read otherwise: a cusp node with
    /// aligned handles, a smooth one with handles of equal length.
    /// Cleared when the curve gains or loses nodes.
    #[serde(
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty",
        deserialize_with = "index_keys::deserialize"
    )]
    pub node_types: std::collections::BTreeMap<usize, crate::nodes::NodeType>,
    /// The control points (local space) of the B-spline the curve is,
    /// for a curve drawn with the B-spline tool: the Shape tool shows and
    /// moves these instead of the curve's nodes, and the curve is their
    /// spline ([`crate::geometry::bspline_path`], [`Command::SetSpline`]).
    /// Any other change to the curve's outline, and Convert To Curves,
    /// drops them and leaves an ordinary curve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spline: Option<Vec<Point>>,
    /// The object's identity across saves, undo and moves between
    /// drawings (see [`Uid`]); the engine gives one to every object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uid: Option<Uid>,
    /// A clip frame drawn as an opacity mask: its frame's brightness (its
    /// fill, gradients included) sets how much of the contents shows,
    /// white fully and black not at all, instead of a hard clip; the
    /// frame itself is not drawn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity_mask: Option<OpacityMask>,
    /// Effects switched off in the effect stack, each with the place it
    /// holds in the stack; they are kept but not drawn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parked_effects: Vec<ParkedEffect>,
    /// Further fills, painted over `fill` in order (see [`Shape::layers`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_fills: Vec<Fill>,
    /// Further outlines, drawn over `stroke` in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_strokes: Vec<Stroke>,
    /// Bitmap effects on an object that is not a bitmap: the object stays
    /// as it is and is drawn as the picture they make of it (see
    /// [`Shape::drawn_as`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raster_fx: Option<Box<RasterFx>>,
    /// A group that is a QR code: what it encodes and how it looks. The
    /// group's children (the background and the dark pixels) are made
    /// again from it whenever it changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qr: Option<Box<QrCode>>,
}

/// What a QR code holds: the kind of content, which decides the fields
/// and how they are written into the code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum QrKind {
    #[default]
    Url,
    Email,
    Phone,
    Sms,
    Contact,
    Text,
    Location,
}

impl QrKind {
    pub const ALL: [QrKind; 7] = [
        QrKind::Url,
        QrKind::Email,
        QrKind::Phone,
        QrKind::Sms,
        QrKind::Contact,
        QrKind::Text,
        QrKind::Location,
    ];

    /// The names of the kind's fields (keys of the interface's strings),
    /// in the order [`QrCode::fields`] holds them.
    pub fn fields(self) -> &'static [&'static str] {
        match self {
            QrKind::Url => &["qr.field_url"],
            QrKind::Email => &["qr.field_email", "qr.field_subject", "qr.field_message"],
            QrKind::Phone => &["qr.field_phone"],
            QrKind::Sms => &["qr.field_phone", "qr.field_message"],
            QrKind::Contact => &[
                "qr.field_first_name",
                "qr.field_last_name",
                "qr.field_organization",
                "qr.field_phone",
                "qr.field_email",
                "qr.field_url",
                "qr.field_address",
            ],
            QrKind::Text => &["qr.field_text"],
            QrKind::Location => &["qr.field_latitude", "qr.field_longitude"],
        }
    }

    /// The fields of one kind carried over to another: what both share by
    /// name keeps its value, the rest starts empty.
    pub fn carry(self, from: QrKind, values: &[String]) -> Vec<String> {
        let old = from.fields();
        self.fields()
            .iter()
            .map(|f| {
                old.iter()
                    .position(|o| o == f)
                    .and_then(|i| values.get(i).cloned())
                    .unwrap_or_default()
            })
            .collect()
    }
}

/// A QR code object's settings (see [`Shape::qr`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QrCode {
    pub kind: QrKind,
    /// The kind's fields, in the order [`QrKind::fields`] names them.
    pub fields: Vec<String>,
    /// The side of one pixel (module), in millimetres.
    pub pixel_mm: f64,
    /// The fill of the dark pixels.
    pub dark: Fill,
    /// The fill behind them, over the code and its quiet zone.
    pub light: Fill,
    /// The outline of the dark pixels.
    pub outline: Option<Stroke>,
}

impl Default for QrCode {
    fn default() -> Self {
        QrCode {
            kind: QrKind::Url,
            fields: vec!["https://github.com/igtoth/tracedraw".into()],
            pixel_mm: 2.5,
            dark: Fill::Solid(crate::Color::BLACK),
            light: Fill::Solid(crate::Color::WHITE),
            outline: None,
        }
    }
}

impl QrCode {
    fn field(&self, i: usize) -> &str {
        self.fields.get(i).map(|s| s.trim()).unwrap_or("")
    }

    /// The text the code encodes: the fields in the form phones and
    /// scanners read for the kind (a `mailto:`-like message, `tel:`,
    /// `SMSTO:`, a vCard 3.0, `geo:`). Empty when nothing is filled in.
    pub fn payload(&self) -> String {
        let f = |i| self.field(i);
        match self.kind {
            QrKind::Url | QrKind::Text => self.fields.first().cloned().unwrap_or_default(),
            QrKind::Email if f(0).is_empty() => String::new(),
            QrKind::Email => format!("MATMSG:TO:{};SUB:{};BODY:{};;", f(0), f(1), f(2)),
            QrKind::Phone if f(0).is_empty() => String::new(),
            QrKind::Phone => format!("tel:{}", f(0)),
            QrKind::Sms if f(0).is_empty() => String::new(),
            QrKind::Sms => format!("SMSTO:{}:{}", f(0), f(1)),
            QrKind::Contact => {
                if (0..7).all(|i| f(i).is_empty()) {
                    return String::new();
                }
                let mut v = String::from("BEGIN:VCARD\nVERSION:3.0\n");
                v.push_str(&format!("N:{};{}\n", f(1), f(0)));
                v.push_str(&format!("FN:{}\n", format!("{} {}", f(0), f(1)).trim()));
                for (key, i) in [("ORG", 2), ("TEL", 3), ("EMAIL", 4), ("URL", 5), ("ADR", 6)] {
                    if !f(i).is_empty() {
                        if key == "ADR" {
                            v.push_str(&format!("ADR:;;{}\n", f(i)));
                        } else {
                            v.push_str(&format!("{key}:{}\n", f(i)));
                        }
                    }
                }
                v.push_str("END:VCARD");
                v
            }
            QrKind::Location if f(0).is_empty() || f(1).is_empty() => String::new(),
            QrKind::Location => format!("geo:{},{}", f(0), f(1)),
        }
    }
}

/// Bitmap effects on a vector object, and the picture they make of it.
/// The picture is made by the app (which holds the effects) and kept with
/// the drawing, so every output draws it without the effects' code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RasterFx {
    pub effects: Vec<BitmapEffect>,
    /// Where the picture goes, in the object's own space (under its
    /// transform); larger than the object where effects spread.
    pub rect: Rect,
    pub width_px: u32,
    pub height_px: u32,
    /// The object with its visible effects; empty until made.
    pub png: crate::SharedBytes,
    /// A fingerprint of what the picture was made from (see
    /// [`Shape::raster_source`]); another one means it is out of date.
    #[serde(default)]
    pub source: u64,
}

/// The lengths the transform gives the object's x and y unit vectors.
pub fn transform_scale(t: Affine) -> (f64, f64) {
    let c = t.as_coeffs();
    (
        (c[0] * c[0] + c[1] * c[1]).sqrt(),
        (c[2] * c[2] + c[3] * c[3]).sqrt(),
    )
}

/// An effect switched off, and its place in the stack (see
/// [`Shape::effect_stack`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParkedEffect {
    pub index: usize,
    pub effect: crate::live::Effect,
}

/// How a clip frame masks its contents (see `Shape::opacity_mask`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OpacityMask {
    /// Black shows and white hides.
    #[serde(default)]
    pub invert: bool,
}

/// An object's stable identity: 128 bits, written as 32 hex digits. Unlike
/// [`ShapeId`] (a number counted per drawing), it stays the same when the
/// drawing is saved and opened again, when a step is undone and when the
/// object is cut and pasted into another drawing; a copy that would share
/// it with its original gets a new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Uid(pub u128);

/// Hashing of identities, which are random already: their bits folded.
#[derive(Default, Clone, Copy)]
pub struct UidHasher(u64);

impl std::hash::Hasher for UidHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut b = [0u8; 8];
            b[..chunk.len()].copy_from_slice(chunk);
            self.0 = (self.0.rotate_left(5) ^ u64::from_le_bytes(b))
                .wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        }
    }
    fn write_u128(&mut self, v: u128) {
        self.0 =
            ((v as u64) ^ ((v >> 64) as u64).rotate_left(29)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

/// The hasher maps of identities use.
pub type UidHash = std::hash::BuildHasherDefault<UidHasher>;

impl std::fmt::Display for Uid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

impl Serialize for Uid {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Uid {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        u128::from_str_radix(s.trim(), 16)
            .map(Uid)
            .map_err(serde::de::Error::custom)
    }
}

/// The next value of a SplitMix64 sequence.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

impl Uid {
    /// A new identity from the generator `state`.
    pub fn next(state: &mut u64) -> Uid {
        let hi = u128::from(splitmix(state));
        let lo = u128::from(splitmix(state));
        Uid((hi << 64) | lo)
    }
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
            page_number: None,
            effects: Vec::new(),
            wrap_text: false,
            wrap_style: crate::text_flow::TextWrap::default(),
            style: None,
            node_types: std::collections::BTreeMap::new(),
            spline: None,
            uid: None,
            opacity_mask: None,
            parked_effects: Vec::new(),
            extra_fills: Vec::new(),
            extra_strokes: Vec::new(),
            raster_fx: None,
            qr: None,
        }
    }

    /// How the object is drawn: itself, or, with visible bitmap effects
    /// whose picture is made, a bitmap of that picture in its place (its
    /// fill, outlines, shadow, effects and opacity are in the picture).
    pub fn drawn_as(&self) -> Option<Shape> {
        let fx = self.raster_fx.as_ref()?;
        if fx.png.is_empty() || !fx.effects.iter().any(|e| e.visible) {
            return None;
        }
        let mut s = self.clone();
        s.kind = ShapeKind::Bitmap {
            rect: fx.rect,
            width_px: fx.width_px,
            height_px: fx.height_px,
            png: fx.png.clone(),
            fx: None,
        };
        s.raster_fx = None;
        s.fill = Fill::None;
        s.stroke = None;
        s.extra_fills.clear();
        s.extra_strokes.clear();
        s.shadow = None;
        s.effects.clear();
        s.parked_effects.clear();
        s.opacity = 1.0;
        Some(s)
    }

    /// A text whose characters have their own fill or outline, as the
    /// texts it is drawn in: one per look (fill and outline), in the order
    /// the looks first appear, each laid out as the whole text with the
    /// other characters left undrawn (`TextSpan::layout_only`). The
    /// character backgrounds and the frame background are left out of the
    /// parts (they are drawn once under the text) and only the first part
    /// keeps the shadow. `None` for other shapes and for text whose
    /// characters all take the object's fill and outline.
    pub fn text_parts(&self) -> Option<Vec<Shape>> {
        let ShapeKind::Text { spans, .. } = &self.kind else {
            return None;
        };
        if spans.iter().any(|s| s.layout_only)
            || spans
                .iter()
                .all(|s| s.fill.is_none() && s.outline.is_none())
        {
            return None;
        }
        let look = |s: &TextSpan| -> (Fill, Option<Stroke>) {
            let fill = s.fill.clone().unwrap_or_else(|| self.fill.clone());
            let stroke = match &s.outline {
                Some(o) if o.width > 0.0 => Some(o.clone()),
                Some(_) => None,
                None => self.stroke.clone(),
            };
            (fill, stroke)
        };
        let mut looks: Vec<(Fill, Option<Stroke>)> = Vec::new();
        for s in spans.iter().filter(|s| s.char_count() > 0) {
            let l = look(s);
            if !looks.contains(&l) {
                looks.push(l);
            }
        }
        let parts = looks
            .into_iter()
            .enumerate()
            .map(|(i, (fill, stroke))| {
                let mut part = self.clone();
                if let ShapeKind::Text { spans, para, .. } = &mut part.kind {
                    for s in spans.iter_mut() {
                        s.layout_only = look(s) != (fill.clone(), stroke.clone());
                        s.background = None;
                        s.fill = None;
                        s.outline = None;
                    }
                    para.frame_fill = None;
                }
                part.fill = fill;
                part.stroke = stroke;
                if i > 0 {
                    part.shadow = None;
                }
                part
            })
            .collect();
        Some(parts)
    }

    /// A fingerprint of what a bitmap-effect picture is made from: the
    /// object without its picture and place, and the scale of its
    /// transform (which sets the picture's resolution).
    pub fn raster_source(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut c = self.clone();
        let (sx, sy) = transform_scale(c.transform);
        c.transform = Affine::IDENTITY;
        if let Some(fx) = c.raster_fx.as_mut() {
            fx.png = Default::default();
            fx.source = 0;
            fx.rect = Rect::ZERO;
            fx.width_px = 0;
            fx.height_px = 0;
        }
        let mut h = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(&c).unwrap_or_default().hash(&mut h);
        ((sx * 1000.0).round() as i64).hash(&mut h);
        ((sy * 1000.0).round() as i64).hash(&mut h);
        h.finish()
    }

    /// The effect stack in order, each effect with whether it is on: the
    /// effects that are drawn, with the switched-off ones at their places.
    pub fn effect_stack(&self) -> Vec<(crate::live::Effect, bool)> {
        let total = self.effects.len() + self.parked_effects.len();
        let mut parked: Vec<&ParkedEffect> = self.parked_effects.iter().collect();
        parked.sort_by_key(|p| p.index);
        let mut parked = parked.into_iter().peekable();
        let mut on = self.effects.iter();
        let mut out = Vec::with_capacity(total);
        for i in 0..total {
            match parked.peek() {
                Some(p) if p.index <= i => {
                    out.push((p.effect.clone(), false));
                    parked.next();
                }
                _ => match on.next() {
                    Some(e) => out.push((e.clone(), true)),
                    None => {
                        if let Some(p) = parked.next() {
                            out.push((p.effect.clone(), false));
                        }
                    }
                },
            }
        }
        out
    }

    /// Split a stack (effects with whether each is on) into the effects
    /// drawn and the switched-off ones with their places.
    pub fn split_stack(
        stack: Vec<(crate::live::Effect, bool)>,
    ) -> (Vec<crate::live::Effect>, Vec<ParkedEffect>) {
        let mut effects = Vec::new();
        let mut parked = Vec::new();
        for (index, (effect, on)) in stack.into_iter().enumerate() {
            if on {
                effects.push(effect);
            } else {
                parked.push(ParkedEffect { index, effect });
            }
        }
        (effects, parked)
    }

    /// An object with further fills or outlines as the plain objects that
    /// draw it, bottom first: the outlines drawn behind the fill, its
    /// fill, each further fill, then the other outlines, its own first
    /// (each part with no other paint; the object's shadow only under the
    /// first part). `None` when it has no further fill or outline.
    pub fn layers(&self) -> Option<Vec<Shape>> {
        if self.extra_fills.is_empty() && self.extra_strokes.is_empty() {
            return None;
        }
        let mut plain = self.clone();
        plain.extra_fills.clear();
        plain.extra_strokes.clear();
        plain.shadow = None;
        let strokes: Vec<&Stroke> = self.stroke.iter().chain(&self.extra_strokes).collect();
        let stroke_part = |st: &Stroke| {
            let mut s = plain.clone();
            s.fill = Fill::None;
            s.stroke = Some(st.clone());
            s
        };
        let mut out = Vec::new();
        out.extend(
            strokes
                .iter()
                .filter(|s| s.behind_fill)
                .map(|s| stroke_part(s)),
        );
        let mut fill = plain.clone();
        fill.stroke = None;
        out.push(fill);
        for f in &self.extra_fills {
            let mut s = plain.clone();
            s.fill = f.clone();
            s.stroke = None;
            out.push(s);
        }
        out.extend(
            strokes
                .iter()
                .filter(|s| !s.behind_fill)
                .map(|s| stroke_part(s)),
        );
        // The shadow goes under the first part.
        if let Some(first) = out.first_mut() {
            first.shadow = self.shadow;
        }
        Some(out)
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

    /// Make a rectangle's rounded corners keep their size when it is
    /// resized (Relative corner scaling off), at the size they measure on
    /// the page now; a rectangle whose axes scale differently gets round
    /// corners of the smaller scale, and when its axes stay square the
    /// stretch goes into its size, so its transform scales evenly and
    /// later stretches keep it a rectangle (not when its outline scales
    /// with it, and a skewed rectangle is left as it is). Corners keep their size even
    /// past half the shorter side (one corner may take a whole side; the
    /// drawing fits corners too big for it). Other shapes, square corners
    /// and corners that already keep their size are left as they are.
    pub fn keep_corner_size(&mut self) {
        let Some(page) = self.page_corners() else {
            return;
        };
        if page.fixed || page.max() <= 0.0 {
            return;
        }
        let Some((mut rect, _)) = self.kind.rect_corners() else {
            return;
        };
        let [a, b, c, d, _, _] = self.transform.as_coeffs();
        let (sx, sy) = axis_scale(self.transform);
        // A skewed rectangle keeps corners that follow it: fixed ones
        // could not be written to the files that keep rectangles plain.
        let square = (a * c + b * d).abs() <= 1e-9 * sx * sy;
        if !square {
            return;
        }
        // An outline that scales with the object would thin out if the
        // stretch left the transform; fixed corners draw round under the
        // stretch anyway.
        let widens_outline = self.stroke.as_ref().is_some_and(|s| s.scale_with_object);
        if sx > 1e-12 && sy > 1e-12 && !widens_outline && (sx - sy).abs() > 1e-9 * sx.max(sy) {
            rect = Rect::new(rect.x0 * sx, rect.y0 * sy, rect.x1 * sx, rect.y1 * sy);
            self.transform *= Affine::scale_non_uniform(1.0 / sx, 1.0 / sy);
        }
        self.kind = ShapeKind::rect_with_corners(
            rect,
            Corners {
                fixed: true,
                ..page
            },
        );
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
    /// measure against the rectangle's own transform: a `parent` that
    /// scales evenly grows them by its factor and they stay fixed; one that
    /// stretches turns them into scaling corners of the same size, or a
    /// curve when the rectangle's axes already scale differently.
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
            // A parent that scales evenly (turning or mirroring as it
            // likes) keeps fixed corners fixed, grown by its factor.
            let [a, b, pc, pd, _, _] = parent.as_coeffs();
            let (pa, pb) = (a.hypot(b), pc.hypot(pd));
            let even = pa > 1e-9 && same(pa, pb) && (a * pc + b * pd).abs() <= 1e-9 * pa * pb;
            if c.fixed && !(same(sx, nx) && same(sy, ny)) && even {
                let mut grown = *c;
                grown.radii = c.radii.map(|r| r * pa);
                self.kind = ShapeKind::rect_with_corners(*rect, grown);
            } else if c.fixed && !(same(sx, nx) && same(sy, ny)) {
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
        // A symmetry's mirror lines go with the object.
        self.carry_effects(parent);
    }

    /// A rectangle that one round radius describes (what formats with a
    /// single radius store): the rectangle, the radius in its space and the
    /// transform that places it, usually the shape's own. A stretched
    /// rectangle whose corners keep their size has the stretch moved into
    /// the rectangle's size, so that its corners stay circles of their page
    /// radius; not when the stretch also widens a scaling outline or the
    /// axes are skewed. `None` for other kinds and for corners that need a
    /// path.
    pub fn plain_rect(&self) -> Option<(Rect, f64, Affine)> {
        let ShapeKind::Rect {
            rect,
            radius,
            corners,
        } = &self.kind
        else {
            return None;
        };
        let t = self.transform;
        let Some(c) = corners else {
            return Some((*rect, *radius, t));
        };
        if c.is_square() {
            return Some((*rect, 0.0, t));
        }
        if c.kind != CornerKind::Round || !c.is_uniform() {
            return None;
        }
        if !c.fixed {
            return Some((*rect, c.radii[0], t));
        }
        let (sx, sy) = axis_scale(t);
        if sx < 1e-9 || sy < 1e-9 {
            return None;
        }
        let r = c.radii[0];
        let r = if r.is_finite() { r.max(0.0) } else { 0.0 };
        let limit = (rect.width().abs() * sx).min(rect.height().abs() * sy) / 2.0;
        if (sx - sy).abs() <= 1e-9 * sx.max(sy) {
            return Some((*rect, r.min(limit) / sx, t));
        }
        let [a, b, cc, d, _, _] = t.as_coeffs();
        let skewed = (a * cc + b * d).abs() > 1e-9 * sx * sy;
        let widens_outline = self.stroke.as_ref().is_some_and(|s| s.scale_with_object);
        if skewed || widens_outline {
            return None;
        }
        let page = Rect::new(rect.x0 * sx, rect.y0 * sy, rect.x1 * sx, rect.y1 * sy);
        Some((
            page,
            r.min(limit),
            t * Affine::scale_non_uniform(1.0 / sx, 1.0 / sy),
        ))
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
                twist,
            } => match complex {
                Some(k) => geometry::complex_star_path(*rect, *points, *k),
                None => geometry::twisted_polygon_path(*rect, *points, *sharpness, *twist),
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
                    // Paragraph text hangs from the top of its frame;
                    // artistic text sits on its baseline.
                    return text_layout_shift(*origin, *frame, false, para.vertical) * p;
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
                    let r = t.cell_box(cell);
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
                        let inner = t.cell_inner(cell);
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
                for (border, path) in t.border_paths() {
                    let mut b = Shape::new(
                        placeholder,
                        ShapeKind::Path {
                            path,
                            closed: false,
                        },
                    );
                    b.fill = Fill::None;
                    b.stroke = Some(border);
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

    /// Whether Convert to Curves changes this object: curves and groups
    /// already are curves, and a bitmap has no outline to become one.
    pub fn converts_to_curves(&self) -> bool {
        match &self.kind {
            // A B-spline becomes an ordinary curve: its nodes instead of
            // its control points.
            ShapeKind::Path { .. } => self.spline.is_some(),
            ShapeKind::Group { .. } | ShapeKind::Bitmap { .. } => false,
            // A clip frame changes only when its frame or something it
            // holds (inside groups too) is not a curve yet, or something it
            // holds has a warp to bake. The frame's own warp stays: the
            // contents are clipped to its outline without it.
            ShapeKind::ClipFrame { frame, contents } => {
                let warps = !self.outline_stays_live();
                frame.converts_to_curves_deep(0, false)
                    || contents.iter().any(|c| c.converts_to_curves_deep(0, warps))
            }
            _ => true,
        }
    }

    /// [`Shape::converts_to_curves`], looking inside groups; `warps` when
    /// an outline warp would be baked too.
    fn converts_to_curves_deep(&self, depth: usize, warps: bool) -> bool {
        let warp = warps && self.has_outline_warp();
        match &self.kind {
            ShapeKind::Group { children } if depth < 16 => {
                let inner = warps && !self.outline_stays_live();
                warp || children
                    .iter()
                    .any(|c| c.converts_to_curves_deep(depth + 1, inner))
            }
            _ => warp || self.converts_to_curves(),
        }
    }

    /// The object converted to curves (Convert to Curves): its kind as
    /// [`Shape::curves_kind`] makes it, then its envelope, perspective and
    /// distortion made part of the curves, the other effects kept (#647).
    /// `None` when it stays as it is.
    pub fn converted_to_curves(
        &self,
        symbols: &[Symbol],
        next_id: &mut dyn FnMut() -> ShapeId,
    ) -> Option<Shape> {
        let mut out = self.clone();
        let mut changed = false;
        // A group's or clip frame's warps go first, then those of what it
        // holds, as they are drawn; an object's own after its conversion.
        let container = matches!(
            self.kind,
            ShapeKind::Group { .. } | ShapeKind::ClipFrame { .. }
        );
        if container {
            if let Some(baked) = crate::live::bake_outline_warps(&out) {
                out = baked;
                changed = true;
            }
        }
        if let Some(kind) = out.curves_kind_depth(symbols, next_id, 0, true) {
            out.kind = kind;
            out.node_types.clear();
            out.spline = None;
            changed = true;
        }
        if !container {
            if let Some(baked) = crate::live::bake_outline_warps(&out) {
                out = baked;
                changed = true;
            }
        }
        changed.then_some(out)
    }

    /// The kind this object takes when converted to curves, or `None` when
    /// [`Shape::converts_to_curves`] says it stays as it is. Tables and
    /// symbol instances become a group of curves made from what they draw
    /// (cell fills, borders and text; the symbol's objects), each piece
    /// with its own fill and outline; `next_id` names the new pieces.
    pub fn curves_kind(
        &self,
        symbols: &[Symbol],
        next_id: &mut dyn FnMut() -> ShapeId,
    ) -> Option<ShapeKind> {
        self.curves_kind_depth(symbols, next_id, 0, false)
    }

    /// [`Shape::curves_kind`]; `bake` makes the outline warps of the
    /// objects inside part of their curves too (see
    /// [`Shape::converted_to_curves`]).
    fn curves_kind_depth(
        &self,
        symbols: &[Symbol],
        next_id: &mut dyn FnMut() -> ShapeId,
        depth: usize,
        bake: bool,
    ) -> Option<ShapeKind> {
        // A symbol that holds itself would never end.
        const MAX_DEPTH: usize = 16;
        if !self.converts_to_curves() || depth > MAX_DEPTH {
            return None;
        }
        match &self.kind {
            ShapeKind::Table(_) | ShapeKind::SymbolInstance { .. } => {
                let mut children = self.expand(symbols);
                if children.is_empty() {
                    return None;
                }
                for c in &mut children {
                    c.curves_deep(symbols, next_id, depth + 1, bake);
                }
                Some(ShapeKind::Group { children })
            }
            // A clip frame stays a clip frame: its frame and what it holds
            // become curves (keeping their ids) and stay clipped. The
            // frame keeps its own warps: the contents are clipped to its
            // outline without them.
            ShapeKind::ClipFrame { frame, contents } => {
                let mut frame = frame.clone();
                let mut contents = contents.clone();
                let inner = bake && !self.outline_stays_live();
                frame.curves_in_place(symbols, next_id, depth + 1, false);
                for c in &mut contents {
                    c.curves_in_place(symbols, next_id, depth + 1, inner);
                }
                let kind = ShapeKind::ClipFrame { frame, contents };
                (kind != self.kind).then_some(kind)
            }
            // A B-spline keeps its outline; the control points go.
            ShapeKind::Path { .. } => Some(self.kind.clone()),
            // Text fills with the nonzero rule, curves with even-odd: the
            // overlapping strokes of a glyph would leave holes (#800).
            ShapeKind::Text { .. } => Some(ShapeKind::Path {
                path: crate::shaping::nonzero_as_even_odd(&self.local_path()),
                closed: true,
            }),
            _ => Some(ShapeKind::Path {
                path: self.local_path(),
                closed: true,
            }),
        }
    }

    /// Turn this object, and everything inside a group or clip frame, into
    /// curves, keeping the ids of the objects that were there (new pieces
    /// of tables and symbols get new ones); `bake` makes their outline
    /// warps part of the curves too.
    fn curves_in_place(
        &mut self,
        symbols: &[Symbol],
        next_id: &mut dyn FnMut() -> ShapeId,
        depth: usize,
        bake: bool,
    ) {
        if depth > 16 {
            return;
        }
        let container = self.bake_container_warps(bake);
        let inner = bake && !self.outline_stays_live();
        match &mut self.kind {
            ShapeKind::Group { children } => {
                for c in children {
                    c.curves_in_place(symbols, next_id, depth + 1, inner);
                }
            }
            _ => {
                if let Some(kind) = self.curves_kind_depth(symbols, next_id, depth, bake) {
                    self.kind = kind;
                    self.node_types.clear();
                    self.spline = None;
                }
                if bake && !container {
                    if let Some(b) = crate::live::bake_outline_warps(self) {
                        *self = b;
                    }
                }
            }
        }
    }

    /// With `bake`, a group's or clip frame's outline warps made part of
    /// what it holds, before the warps of those objects, as they are
    /// drawn. Whether this is such a container.
    fn bake_container_warps(&mut self, bake: bool) -> bool {
        let container = matches!(
            self.kind,
            ShapeKind::Group { .. } | ShapeKind::ClipFrame { .. }
        );
        if bake && container {
            if let Some(b) = crate::live::bake_outline_warps(self) {
                *self = b;
            }
        }
        container
    }

    /// Give this object and everything inside it new ids, and turn every
    /// convertible piece into curves.
    fn curves_deep(
        &mut self,
        symbols: &[Symbol],
        next_id: &mut dyn FnMut() -> ShapeId,
        depth: usize,
        bake: bool,
    ) {
        self.id = next_id();
        let container = self.bake_container_warps(bake);
        let inner = bake && !self.outline_stays_live();
        match &mut self.kind {
            ShapeKind::Group { children } => {
                for c in children {
                    c.curves_deep(symbols, next_id, depth + 1, inner);
                }
            }
            // A frame keeps what it holds; both become curves, the frame
            // keeping its own warps (the contents are clipped without).
            ShapeKind::ClipFrame { frame, contents } => {
                frame.curves_deep(symbols, next_id, depth + 1, false);
                for c in contents {
                    c.curves_deep(symbols, next_id, depth + 1, inner);
                }
            }
            _ => {
                // The pieces of a converted table or symbol get their ids there.
                if let Some(kind) = self.curves_kind_depth(symbols, next_id, depth, bake) {
                    self.kind = kind;
                    self.node_types.clear();
                    self.spline = None;
                }
                if bake && !container {
                    if let Some(b) = crate::live::bake_outline_warps(self) {
                        *self = b;
                    }
                }
            }
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

    /// Apply `t` to the object (in the space its transform maps to): its
    /// transform, and the places its live effects hold (those switched
    /// off too), such as a symmetry's mirror lines, which go with it (see
    /// [`crate::live::carry_effects`]).
    pub fn transform_by(&mut self, t: Affine) {
        self.transform = t * self.transform;
        self.carry_effects(t);
    }

    /// The fountain fills and gradient transparencies of the object and of
    /// the objects inside it carried along with `t`, a page-space
    /// transformation about to be applied to it; `parent` maps its space
    /// to the page. The fills are placed on the page box of each object,
    /// so without this a turned or mirrored object kept its gradient
    /// running the old way (#775).
    pub fn carry_fills(&mut self, t: Affine, parent: Affine) {
        let to_page = parent * self.transform;
        match &mut self.kind {
            ShapeKind::Group { children } => {
                for c in children {
                    c.carry_fills(t, to_page);
                }
            }
            ShapeKind::ClipFrame { frame, contents } => {
                frame.carry_fills(t, to_page);
                for c in contents {
                    c.carry_fills(t, to_page);
                }
            }
            _ => {}
        }
        let gradient = |f: &Fill| matches!(f, Fill::Fountain(_));
        let masks = |e: &crate::live::Effect| matches!(e, crate::live::Effect::Transparency { mask, .. } if gradient(mask));
        if !gradient(&self.fill) && !self.effects.iter().any(masks) {
            return;
        }
        let path = to_page * self.local_path();
        let before = path.bounding_box();
        let after = (t * path).bounding_box();
        self.fill.carry(t, before, after);
        for e in &mut self.effects {
            if let crate::live::Effect::Transparency { mask, .. } = e {
                mask.carry(t, before, after);
            }
        }
    }

    /// The places the object's live effects hold, those switched off too,
    /// moved by `t` (see [`crate::live::carry_effects`]).
    pub fn carry_effects(&mut self, t: Affine) {
        crate::live::carry_effects(&mut self.effects, t);
        crate::live::carry_effects(self.parked_effects.iter_mut().map(|p| &mut p.effect), t);
    }

    /// Outline in page coordinates (transform applied).
    pub fn page_path(&self) -> BezPath {
        self.transform * self.local_path()
    }

    /// The outline in page coordinates the shape would have once `t` (a
    /// page-space transformation) is applied to it, as a move, scale or
    /// turn being dragged shows it: corners that keep their size stay
    /// round instead of stretching with `t`.
    pub fn page_path_after(&self, t: Affine) -> BezPath {
        match &self.kind {
            ShapeKind::Rect {
                rect,
                corners: Some(c),
                ..
            } => {
                let moved = t * self.transform;
                moved * c.path(*rect, axis_scale(moved))
            }
            _ => t * self.page_path(),
        }
    }

    /// Axis-aligned bounds in page coordinates, ignoring outline width.
    /// Paragraph text counts its whole frame (empty or not), and a group
    /// the bounds of its objects.
    /// The page box of a text object from the box the text engine keeps
    /// with its outline, when the object is neither turned nor skewed
    /// (the box then maps onto the page box exactly); `None` otherwise or
    /// for other objects. Asking for the outline itself copies all of its
    /// points, which in a drawing of long, large text took most of every
    /// frame: the box is asked for many times a frame (#802).
    fn text_bounds(&self) -> Option<Rect> {
        let ShapeKind::Text {
            spans,
            origin,
            frame,
            align,
            para,
            on_path,
        } = &self.kind
        else {
            return None;
        };
        let req = text_outline::TextRequest {
            spans,
            frame: *frame,
            align: *align,
            para,
            on_path: on_path.as_ref(),
        };
        let local = if on_path.is_some() {
            Affine::translate(origin.to_vec2())
        } else {
            text_layout_shift(*origin, *frame, false, para.vertical)
        };
        let m = self.transform * local;
        let [a, b, c, d, _, _] = m.as_coeffs();
        let scale = a.abs().max(d.abs()).max(1e-12);
        if b.abs() > 1e-12 * scale || c.abs() > 1e-12 * scale {
            return None;
        }
        let r = text_outline::bounds(&req)?;
        let (p, q) = (m * Point::new(r.x0, r.y0), m * Point::new(r.x1, r.y1));
        let out = Rect::from_points(p, q);
        (out.x0.is_finite() && out.y0.is_finite() && out.x1.is_finite() && out.y1.is_finite())
            .then_some(out)
    }

    pub fn bounds(&self) -> Rect {
        match &self.kind {
            ShapeKind::Text {
                origin,
                frame: Some(f),
                on_path: None,
                ..
            } => {
                let frame = Rect::new(
                    origin.x,
                    origin.y,
                    origin.x + f.width.max(0.0),
                    origin.y + f.height.max(0.0),
                );
                let fb = (self.transform * frame.to_path(0.01)).bounding_box();
                if let Some(tb) = self.text_bounds() {
                    return tb.union(fb);
                }
                let path = self.page_path();
                if path.elements().is_empty() {
                    fb
                } else {
                    path.bounding_box().union(fb)
                }
            }
            ShapeKind::Text { .. } => self
                .text_bounds()
                .unwrap_or_else(|| self.page_path().bounding_box()),
            ShapeKind::Group { children } => children
                .iter()
                .map(|c| {
                    let mut c = c.clone();
                    c.transform = self.transform * c.transform;
                    c.bounds()
                })
                .reduce(|a, b| a.union(b))
                .unwrap_or_else(|| self.page_path().bounding_box()),
            _ => self.page_path().bounding_box(),
        }
    }

    /// The frame of paragraph text in page space (its four corners, the
    /// local frame's lower left first, counter-clockwise), `None` for other
    /// objects.
    pub fn text_frame_corners(&self) -> Option<[Point; 4]> {
        match &self.kind {
            ShapeKind::Text {
                origin,
                frame: Some(f),
                on_path: None,
                ..
            } => {
                let t = self.transform;
                Some([
                    t * *origin,
                    t * Point::new(origin.x + f.width, origin.y),
                    t * Point::new(origin.x + f.width, origin.y + f.height),
                    t * Point::new(origin.x, origin.y + f.height),
                ])
            }
            _ => None,
        }
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

/// A named set of object properties (Object Styles panel). Objects it is
/// applied to keep a link to it ([`Shape::style`]), so changing the style
/// changes them; the parts it leaves out stay as each object has them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectStyle {
    pub name: String,
    pub fill: Fill,
    pub stroke: Option<Stroke>,
    /// What links to the style refer to; 0 until the styles are set
    /// (older files), when it gets a free number.
    #[serde(default)]
    pub id: u32,
    /// The properties the style sets.
    #[serde(default)]
    pub parts: StyleParts,
    /// The transparency part: opacity, 1 being opaque.
    #[serde(default = "one")]
    pub opacity: f64,
}

/// Which properties an object style sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleParts {
    pub fill: bool,
    pub outline: bool,
    pub transparency: bool,
}

impl Default for StyleParts {
    /// Fill and outline: what styles held before they had parts.
    fn default() -> Self {
        StyleParts {
            fill: true,
            outline: true,
            transparency: false,
        }
    }
}

impl ObjectStyle {
    /// A style named `name` with the fill and outline of `shape`, and its
    /// transparency when it has some.
    pub fn from_shape(name: impl Into<String>, shape: &Shape) -> ObjectStyle {
        ObjectStyle {
            name: name.into(),
            fill: shape.fill.clone(),
            stroke: shape.stroke.clone(),
            id: 0,
            parts: StyleParts {
                fill: true,
                outline: true,
                transparency: shape.opacity < 1.0,
            },
            opacity: shape.opacity,
        }
    }

    /// Give `shape` the parts this style sets.
    pub fn apply_to(&self, shape: &mut Shape) {
        if self.parts.fill {
            shape.fill = self.fill.clone();
        }
        if self.parts.outline {
            shape.stroke = self.stroke.clone();
        }
        if self.parts.transparency {
            shape.opacity = self.opacity.clamp(0.0, 1.0);
        }
    }
}

/// The fill and outline new objects get, by kind (Object Styles panel,
/// Default Properties); saved with the drawing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ObjectDefaults {
    pub graphic: DefaultProperties,
    pub artistic_text: DefaultProperties,
    pub paragraph_text: DefaultProperties,
    /// Tabs, columns, bullets and drop caps new paragraph text gets
    /// (Text > Tabs, Columns, Bullets, Drop Cap with no paragraph text
    /// selected).
    pub paragraph: ParagraphStyle,
}

/// One kind's defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultProperties {
    pub fill: Fill,
    pub stroke: Option<Stroke>,
}

/// The kinds of new objects with their own defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefaultKind {
    Graphic,
    ArtisticText,
    ParagraphText,
}

impl DefaultKind {
    pub const ALL: [DefaultKind; 3] = [
        DefaultKind::Graphic,
        DefaultKind::ArtisticText,
        DefaultKind::ParagraphText,
    ];

    /// The kind of `kind`: text in a frame is paragraph text, other text
    /// artistic text, everything else a graphic.
    pub fn of(kind: &ShapeKind) -> DefaultKind {
        match kind {
            ShapeKind::Text { frame: Some(_), .. } => DefaultKind::ParagraphText,
            ShapeKind::Text { .. } => DefaultKind::ArtisticText,
            _ => DefaultKind::Graphic,
        }
    }
}

impl Default for ObjectDefaults {
    /// The defaults of a CMYK drawing (the default primary colour mode).
    fn default() -> Self {
        ObjectDefaults::for_mode(PrimaryColorMode::Cmyk)
    }
}

impl ObjectDefaults {
    /// Graphics: no fill and a 0.2 mm CMYK black outline; text: a black
    /// fill in the drawing's primary colour mode (CMYK or RGB black) and
    /// no outline.
    pub fn for_mode(mode: PrimaryColorMode) -> Self {
        let text = DefaultProperties {
            fill: Fill::Solid(mode.black()),
            stroke: None,
        };
        ObjectDefaults {
            graphic: DefaultProperties {
                fill: Fill::None,
                stroke: Some(Stroke {
                    color: crate::Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
                    width: 0.2,
                    ..Stroke::default()
                }),
            },
            artistic_text: text.clone(),
            paragraph_text: text,
            paragraph: ParagraphStyle::default(),
        }
    }
}

impl ObjectDefaults {
    pub fn get(&self, kind: DefaultKind) -> &DefaultProperties {
        match kind {
            DefaultKind::Graphic => &self.graphic,
            DefaultKind::ArtisticText => &self.artistic_text,
            DefaultKind::ParagraphText => &self.paragraph_text,
        }
    }

    pub fn get_mut(&mut self, kind: DefaultKind) -> &mut DefaultProperties {
        match kind {
            DefaultKind::Graphic => &mut self.graphic,
            DefaultKind::ArtisticText => &mut self.artistic_text,
            DefaultKind::ParagraphText => &mut self.paragraph_text,
        }
    }

    /// Give a new `shape` the fill and outline of its kind.
    pub fn apply_to(&self, shape: &mut Shape) {
        let d = self.get(DefaultKind::of(&shape.kind));
        shape.fill = d.fill.clone();
        shape.stroke = d.stroke.clone();
        if let ShapeKind::Text {
            frame: Some(_),
            para,
            ..
        } = &mut shape.kind
        {
            self.paragraph.apply_layout_to(para);
        }
    }
}

impl ParagraphStyle {
    /// Copy the tabs, columns, bullets and drop cap of `self` to `to`,
    /// leaving its spacing and indents.
    pub fn apply_layout_to(&self, to: &mut ParagraphStyle) {
        to.tabs = self.tabs.clone();
        to.columns = self.columns;
        to.gutter = self.gutter;
        to.bullets = self.bullets;
        to.bullet_char = self.bullet_char.clone();
        to.bullet_indent = self.bullet_indent;
        to.drop_cap_lines = self.drop_cap_lines;
    }
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
    /// The drawing's language (a language tag such as "en-US"; empty: not
    /// set).
    pub language: String,
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
    /// The drawing's grayscale profile name (empty: the default one).
    pub gray_profile: String,
    /// The drawing's rendering intent (empty: the default one).
    pub rendering_intent: String,
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

impl PrimaryColorMode {
    /// Black in this mode: 100 % K in CMYK, RGB black in RGB.
    pub fn black(self) -> crate::Color {
        match self {
            PrimaryColorMode::Cmyk => crate::Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
            PrimaryColorMode::Rgb => crate::Color::BLACK,
        }
    }
}

/// How a page number field shows the number (Layout > Insert Page
/// Number): the number of the first page, the style and the text around
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageNumberField {
    pub start_at: i64,
    /// 0 arabic, 1 upper roman, 2 lower roman, 3 upper alpha, 4 lower alpha.
    pub style: u8,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
}

impl PageNumberField {
    /// The text for the page at `index` (0-based).
    pub fn label(&self, index: usize) -> String {
        let n = self.start_at + index as i64;
        let body = match self.style {
            1 => roman(n),
            2 => roman(n).to_lowercase(),
            3 => alpha(n),
            4 => alpha(n).to_lowercase(),
            _ => n.to_string(),
        };
        format!("{}{}{}", self.prefix, body, self.suffix)
    }
}

fn roman(mut n: i64) -> String {
    if n <= 0 {
        return n.to_string();
    }
    const T: [(i64, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (v, s) in T {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

fn alpha(n: i64) -> String {
    if n <= 0 {
        return n.to_string();
    }
    // A..Z, then AA, AB... like spreadsheet columns.
    let mut n = n - 1;
    let mut out = Vec::new();
    loop {
        out.push((b'A' + (n % 26) as u8) as char);
        n /= 26;
        if n == 0 {
            break;
        }
        n -= 1;
    }
    out.iter().rev().collect()
}

impl Shape {
    /// This object, or one inside it, is a page number field.
    pub fn holds_page_number(&self) -> bool {
        self.page_number.is_some()
            || match &self.kind {
                ShapeKind::Group { children } => children.iter().any(Shape::holds_page_number),
                _ => false,
            }
    }

    /// Page number fields in this object show the number of the page at
    /// `index` (0-based).
    pub fn show_page_number(&mut self, index: usize) {
        if let (Some(f), ShapeKind::Text { spans, .. }) = (&self.page_number, &mut self.kind) {
            let label = f.label(index);
            spans.truncate(1);
            if let Some(first) = spans.first_mut() {
                first.text = label;
            }
        }
        if let ShapeKind::Group { children } = &mut self.kind {
            for c in children {
                c.show_page_number(index);
            }
        }
    }
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RulerSettings {
    /// The ruler origin, in mm from the page's bottom-left corner.
    pub origin_x: f64,
    pub origin_y: f64,
    /// Number of tick marks between two numbered marks.
    pub tick_divisions: u32,
    /// Drawing scale: `scale_page` mm on the page stand for `scale_world`
    /// mm in the world (1:1 by default). Rulers, positions, sizes and
    /// dimension labels show world distances; outline widths, text sizes
    /// and the page size stay as printed.
    pub scale_page: f64,
    pub scale_world: f64,
    /// The units the two distances were given in (unit ids such as "cm"
    /// or "m"), kept for the dialog; empty for millimetres.
    pub scale_page_units: String,
    pub scale_world_units: String,
}

impl Default for RulerSettings {
    fn default() -> Self {
        RulerSettings {
            origin_x: 0.0,
            origin_y: 0.0,
            tick_divisions: 10,
            scale_page: 1.0,
            scale_world: 1.0,
            scale_page_units: String::new(),
            scale_world_units: String::new(),
        }
    }
}

impl RulerSettings {
    /// World millimetres per page millimetre: 1 for a 1:1 drawing, 100 for
    /// 1:100. Never zero, negative or infinite (1 then).
    pub fn world_per_page(&self) -> f64 {
        let r = self.scale_world / self.scale_page;
        if r.is_finite() && r > 0.0 && self.scale_page > 0.0 {
            r
        } else {
            1.0
        }
    }

    /// True when the drawing is not at 1:1.
    pub fn is_scaled(&self) -> bool {
        (self.world_per_page() - 1.0).abs() > 1e-12
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
    /// The fill and outline new objects get.
    #[serde(default)]
    pub defaults: ObjectDefaults,
    #[serde(default)]
    pub color_styles: Vec<ColorStyle>,
    #[serde(default)]
    pub symbols: Vec<Symbol>,
    #[serde(default)]
    pub metadata: Metadata,
    /// Document-level palette (the bottom "document palette").
    #[serde(default)]
    pub palette: Vec<crate::Color>,
    /// Time Tracker records. Like the id counter they stay as they are
    /// when the drawing's changes are undone.
    #[serde(default, skip_serializing_if = "TimeTracker::is_empty")]
    pub time_tracker: TimeTracker,
    /// Selection groups 0 to 9: objects stored under a number to be
    /// selected again with it, saved with the drawing.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub selection_groups: std::collections::BTreeMap<u8, Vec<ShapeId>>,
    /// Font files the drawing carries (fonts embedded in the file it was
    /// opened from), saved with it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fonts: Vec<DocumentFont>,
    /// The CMYK colour profile the drawing's CMYK colours were chosen in
    /// (the profile a `.cdr` file carries), saved with it; screen colours
    /// of CMYK values come from it while the drawing is open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmyk_profile: Option<DocumentProfile>,
    /// Spot (named) inks the drawing's colours use (`Color::Spot`), each
    /// printed on a plate of its own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inks: Vec<Ink>,
    /// Review comments pinned to the pages (never printed or exported).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Comment>,
}

/// What a comment points at on its page (page millimetres).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommentMark {
    Pin {
        at: crate::geometry::Point,
    },
    Rect {
        rect: crate::geometry::Rect,
    },
    Arrow {
        from: crate::geometry::Point,
        to: crate::geometry::Point,
    },
}

impl CommentMark {
    /// Where the comment's number is shown.
    pub fn anchor(&self) -> crate::geometry::Point {
        match self {
            CommentMark::Pin { at } => *at,
            CommentMark::Rect { rect } => crate::geometry::Point::new(rect.x1, rect.y1),
            CommentMark::Arrow { from, .. } => *from,
        }
    }
}

/// An answer in a comment's thread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommentReply {
    pub author: String,
    /// Seconds since 1970 (UTC).
    pub time: i64,
    pub text: String,
}

/// A review comment: who wrote what, when, where on which page, its
/// replies, and whether it has been dealt with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: u32,
    pub page: PageId,
    pub mark: CommentMark,
    pub author: String,
    /// Seconds since 1970 (UTC).
    pub time: i64,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replies: Vec<CommentReply>,
    #[serde(default)]
    pub resolved: bool,
}

/// A spot ink: its number (what `Color::Spot` refers to), its name and
/// its process stand-in at full strength (C, M, Y, K, 0 to 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ink {
    pub id: u32,
    pub name: String,
    pub cmyk: [f32; 4],
}

impl Ink {
    /// The ink at `tint` (0 to 1) as a colour.
    pub fn color(&self, tint: f32) -> crate::Color {
        let [c, m, y, k] = self.cmyk;
        crate::Color::Spot {
            ink: self.id,
            tint: tint.clamp(0.0, 1.0),
            c,
            m,
            y,
            k,
        }
    }
}

/// An ICC profile a drawing carries.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentProfile {
    /// The profile's name, for display.
    pub name: String,
    /// The ICC file, shared between undo steps.
    #[serde(with = "shared_bytes")]
    pub data: std::sync::Arc<Vec<u8>>,
}

impl DocumentProfile {
    pub fn new(name: impl Into<String>, data: Vec<u8>) -> Self {
        DocumentProfile {
            name: name.into(),
            data: std::sync::Arc::new(data),
        }
    }
}

impl std::fmt::Debug for DocumentProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentProfile")
            .field("name", &self.name)
            .field("bytes", &self.data.len())
            .finish()
    }
}

/// A font file a drawing carries, so its text keeps its letters where the
/// font is not installed. The text engine loads it while the drawing is
/// open when no installed font has its family and its licence allows use
/// in documents. See docs/behavior/text.md.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentFont {
    /// The family name the file it came from gives (for display; the text
    /// engine reads the names in the font itself).
    pub family: String,
    /// The TrueType or OpenType file, shared between undo steps.
    #[serde(with = "shared_bytes")]
    pub data: std::sync::Arc<Vec<u8>>,
}

impl DocumentFont {
    pub fn new(family: impl Into<String>, data: Vec<u8>) -> Self {
        DocumentFont {
            family: family.into(),
            data: std::sync::Arc::new(data),
        }
    }
}

impl std::fmt::Debug for DocumentFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentFont")
            .field("family", &self.family)
            .field("bytes", &self.data.len())
            .finish()
    }
}

/// Shared binary data as base64 text, like bitmaps.
mod shared_bytes {
    use serde::{Deserializer, Serializer};
    use std::sync::Arc;
    pub fn serialize<S: Serializer>(v: &Arc<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        super::png_bytes::serialize(v.as_slice(), s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Arc<Vec<u8>>, D::Error> {
        super::png_bytes::deserialize(d).map(Arc::new)
    }
}

/// The Time Tracker's records, saved with the drawing: its tasks and the
/// one being tracked (Window > Toolbars > Time Tracker).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TimeTracker {
    #[serde(default)]
    pub tasks: Vec<TrackedTask>,
    /// The active task's index in `tasks`.
    #[serde(default)]
    pub active: Option<usize>,
}

/// One task of the Time Tracker.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TrackedTask {
    pub name: String,
    /// Recorded time, in seconds.
    #[serde(default)]
    pub seconds: f64,
    /// When work on the task started and last finished, in seconds since
    /// 1970-01-01 UTC. Recording time moves them; they can also be set by
    /// hand, independently of the recorded time.
    #[serde(default)]
    pub start: Option<i64>,
    #[serde(default)]
    pub finish: Option<i64>,
}

impl TimeTracker {
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// The active task, when the index points at one.
    pub fn active_task(&self) -> Option<&TrackedTask> {
        self.active.and_then(|i| self.tasks.get(i))
    }

    pub fn active_task_mut(&mut self) -> Option<&mut TrackedTask> {
        self.active.and_then(|i| self.tasks.get_mut(i))
    }

    /// Every task's recorded time together, in seconds.
    pub fn total_seconds(&self) -> f64 {
        self.tasks.iter().map(|t| t.seconds.max(0.0)).sum()
    }

    /// Add `seconds` of work to the active task at `now` (seconds since
    /// 1970): the first time sets its start, every time its finish.
    pub fn record(&mut self, seconds: f64, now: i64) {
        if !(seconds.is_finite() && seconds > 0.0) {
            return;
        }
        if let Some(t) = self.active_task_mut() {
            t.seconds += seconds;
            if t.start.is_none() {
                t.start = Some(now - seconds.round() as i64);
            }
            t.finish = Some(now);
        }
    }
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
            defaults: ObjectDefaults::default(),
            color_styles: Vec::new(),
            symbols: Vec::new(),
            metadata: Metadata {
                resolution_dpi: 300.0,
                ..Default::default()
            },
            palette: Vec::new(),
            time_tracker: TimeTracker::default(),
            selection_groups: std::collections::BTreeMap::new(),
            fonts: Vec::new(),
            cmyk_profile: None,
            inks: Vec::new(),
            comments: Vec::new(),
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

    /// The font families the drawing's text uses, sorted: text on layers,
    /// inside groups and clip frames, in table cells and in symbols.
    pub fn font_families_used(&self) -> Vec<String> {
        let mut out: Vec<String> = self.font_faces_used().into_iter().map(|f| f.0).collect();
        out.dedup();
        out
    }

    /// The faces the drawing's text uses: family, bold, italic, sorted,
    /// each once (empty family names left out).
    pub fn font_faces_used(&self) -> Vec<(String, bool, bool)> {
        type Faces = std::collections::BTreeSet<(String, bool, bool)>;
        fn add(spans: &[TextSpan], out: &mut Faces) {
            out.extend(
                spans
                    .iter()
                    .filter(|sp| !sp.font_family.is_empty())
                    .map(|sp| (sp.font_family.clone(), sp.bold, sp.italic)),
            );
        }
        fn walk(s: &Shape, out: &mut Faces) {
            match &s.kind {
                ShapeKind::Text { spans, .. } => add(spans, out),
                ShapeKind::Table(t) => t.cells.iter().for_each(|c| add(&c.text, out)),
                ShapeKind::Group { children } => children.iter().for_each(|c| walk(c, out)),
                ShapeKind::ClipFrame { frame, contents } => {
                    walk(frame, out);
                    contents.iter().for_each(|c| walk(c, out));
                }
                _ => {}
            }
        }
        let mut out = Faces::new();
        for s in self.all_layers().flat_map(|l| &l.shapes) {
            walk(s, &mut out);
        }
        for sym in &self.symbols {
            walk(&sym.shape, &mut out);
        }
        out.into_iter().collect()
    }

    /// Layers drawn for a page, bottom to top: applicable master layers,
    /// then the page's own layers, as they are stored (what clicks and
    /// selections take objects from).
    pub fn layers_for_page(&self, page: PageId) -> Result<Vec<&Layer>> {
        let idx = self.page_index(page)?;
        let mut out: Vec<&Layer> = self
            .master
            .iter()
            .filter(|l| l.scope.applies(idx))
            .collect();
        out.extend(self.pages[idx].layers.iter());
        Ok(out)
    }

    /// [`Document::layers_for_page`] as drawn, printed and exported: a
    /// master layer holding a page number field comes as a copy showing
    /// this page's number (#807).
    pub fn layers_drawn_for_page(&self, page: PageId) -> Result<Vec<std::borrow::Cow<'_, Layer>>> {
        use std::borrow::Cow;
        let idx = self.page_index(page)?;
        Ok(self
            .layers_for_page(page)?
            .into_iter()
            .map(|l| {
                let master = self.master.iter().any(|m| m.id == l.id);
                if master && l.shapes.iter().any(Shape::holds_page_number) {
                    let mut l = l.clone();
                    for s in &mut l.shapes {
                        s.show_page_number(idx);
                    }
                    Cow::Owned(l)
                } else {
                    Cow::Borrowed(l)
                }
            })
            .collect())
    }

    fn page_index(&self, page: PageId) -> Result<usize> {
        self.pages
            .iter()
            .position(|p| p.id == page)
            .ok_or(Error::PageNotFound(page))
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

    /// Find any shape to change it, including children of groups and clip
    /// frames.
    pub fn find_shape_mut(&mut self, id: ShapeId) -> Option<&mut Shape> {
        fn walk(s: &mut Shape, id: ShapeId) -> Option<&mut Shape> {
            if s.id == id {
                return Some(s);
            }
            match &mut s.kind {
                ShapeKind::Group { children } => children.iter_mut().find_map(|c| walk(c, id)),
                ShapeKind::ClipFrame { frame, contents } => {
                    walk(frame, id).or_else(|| contents.iter_mut().find_map(|c| walk(c, id)))
                }
                _ => None,
            }
        }
        self.all_layers_mut()
            .flat_map(|l| &mut l.shapes)
            .find_map(|s| walk(s, id))
    }

    /// The transform taking the space of `id` to the page: the product of
    /// the transforms of the groups and clip frames it is in (the identity
    /// for an object lying on a layer). `None` when there is no such
    /// object.
    pub fn parent_transform(&self, id: ShapeId) -> Option<Affine> {
        fn walk(s: &Shape, id: ShapeId, above: Affine) -> Option<Affine> {
            if s.id == id {
                return Some(above);
            }
            let inner = above * s.transform;
            match &s.kind {
                ShapeKind::Group { children } => children.iter().find_map(|c| walk(c, id, inner)),
                ShapeKind::ClipFrame { frame, contents } => walk(frame, id, inner)
                    .or_else(|| contents.iter().find_map(|c| walk(c, id, inner))),
                _ => None,
            }
        }
        self.all_layers()
            .flat_map(|l| &l.shapes)
            .find_map(|s| walk(s, id, Affine::IDENTITY))
    }

    /// A copy of any object, nested ones too, whose transform takes it
    /// straight to the page, so its paths and bounds are in page space.
    pub fn shape_on_page(&self, id: ShapeId) -> Option<Shape> {
        if let Ok((_, s)) = self.shape(id) {
            return Some(s.clone());
        }
        let parent = self.parent_transform(id)?;
        let mut s = self.find_shape(id)?.clone();
        // The effects stay as stored (in the parent's space): what reads
        // them here writes them back to the object.
        s.transform = parent * s.transform;
        Some(s)
    }

    /// The groups and clip frames holding `id`, outermost first (empty for
    /// an object on a layer or one that is not there).
    pub fn ancestors(&self, id: ShapeId) -> Vec<ShapeId> {
        fn walk(s: &Shape, id: ShapeId, path: &mut Vec<ShapeId>) -> bool {
            if s.id == id {
                return true;
            }
            path.push(s.id);
            let found = match &s.kind {
                ShapeKind::Group { children } => children.iter().any(|c| walk(c, id, path)),
                ShapeKind::ClipFrame { frame, contents } => {
                    walk(frame, id, path) || contents.iter().any(|c| walk(c, id, path))
                }
                _ => false,
            };
            if !found {
                path.pop();
            }
            found
        }
        let mut path = Vec::new();
        for s in self.all_layers().flat_map(|l| &l.shapes) {
            if walk(s, id, &mut path) {
                return path;
            }
        }
        Vec::new()
    }

    /// The object lies inside a group or a clip frame (not on a layer).
    pub fn is_nested(&self, id: ShapeId) -> bool {
        self.shape(id).is_err() && self.find_shape(id).is_some()
    }

    /// The object on a layer that holds `id` (itself when it lies on a
    /// layer).
    pub fn top_level_of(&self, id: ShapeId) -> Option<ShapeId> {
        fn holds(s: &Shape, id: ShapeId) -> bool {
            s.id == id
                || match &s.kind {
                    ShapeKind::Group { children } => children.iter().any(|c| holds(c, id)),
                    ShapeKind::ClipFrame { frame, contents } => {
                        holds(frame, id) || contents.iter().any(|c| holds(c, id))
                    }
                    _ => false,
                }
        }
        self.all_layers()
            .flat_map(|l| &l.shapes)
            .find(|s| holds(s, id))
            .map(|s| s.id)
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

    /// The spot ink numbered `id`.
    pub fn ink(&self, id: u32) -> Option<&Ink> {
        self.inks.iter().find(|i| i.id == id)
    }

    /// A number no ink of the drawing has.
    pub fn next_ink_id(&self) -> u32 {
        self.inks
            .iter()
            .map(|i| i.id)
            .max()
            .map_or(1, |m| m.saturating_add(1))
    }

    /// Give every object an identity of its own: objects without one, and
    /// every object but the oldest (lowest id) of those sharing one, get a
    /// new one from `state`. Objects in groups, master layers and symbols
    /// count too. Returns how many were given one.
    pub fn assign_uids(&mut self, state: &mut u64) -> usize {
        fn visit(shapes: &[Shape], f: &mut dyn FnMut(&Shape)) {
            for s in shapes {
                f(s);
                if let ShapeKind::Group { children } = &s.kind {
                    visit(children, f);
                }
            }
        }
        fn visit_mut(shapes: &mut [Shape], f: &mut dyn FnMut(&mut Shape)) {
            for s in shapes {
                f(s);
                if let ShapeKind::Group { children } = &mut s.kind {
                    visit_mut(children, f);
                }
            }
        }
        // Who keeps each identity: the lowest id carrying it. Identities
        // are random already, so a plain fold of their bits hashes them
        // (std's keyed hash made this walk a fifth of every edit, #345).
        let mut owner: std::collections::HashMap<Uid, ShapeId, UidHash> =
            std::collections::HashMap::with_capacity_and_hasher(1024, UidHash::default());
        let mut count = 0usize;
        let mut carried = 0usize;
        {
            let mut note = |s: &Shape| {
                count += 1;
                if let Some(u) = s.uid {
                    carried += 1;
                    let e = owner.entry(u).or_insert(s.id);
                    if s.id < *e {
                        *e = s.id;
                    }
                }
            };
            for l in self
                .pages
                .iter()
                .flat_map(|p| &p.layers)
                .chain(&self.master)
            {
                visit(&l.shapes, &mut note);
            }
            for sym in &self.symbols {
                visit(std::slice::from_ref(&sym.shape), &mut note);
            }
        }
        // Every object carries an identity no other one does.
        if carried == count && owner.len() == count {
            return 0;
        }
        let mut seen: std::collections::HashSet<Uid, UidHash> =
            std::collections::HashSet::with_capacity_and_hasher(owner.len(), UidHash::default());
        let mut given = 0;
        let mut fix = |s: &mut Shape| {
            let keep = s
                .uid
                .is_some_and(|u| owner.get(&u) == Some(&s.id) && seen.insert(u));
            if !keep {
                let mut u = Uid::next(state);
                while owner.contains_key(&u) || !seen.insert(u) {
                    u = Uid::next(state);
                }
                s.uid = Some(u);
                given += 1;
            }
        };
        for l in self.pages.iter_mut().flat_map(|p| &mut p.layers) {
            visit_mut(&mut l.shapes, &mut fix);
        }
        for l in &mut self.master {
            visit_mut(&mut l.shapes, &mut fix);
        }
        for sym in &mut self.symbols {
            visit_mut(std::slice::from_mut(&mut sym.shape), &mut fix);
        }
        given
    }

    /// Serialize to the native `.tdraw` JSON format: the document's
    /// fields and `format_version` ([`TDRAW_FORMAT_VERSION`]).
    pub fn to_json(&self) -> Result<String> {
        self.to_json_version(TDRAW_FORMAT_VERSION)
    }

    /// Serialize as a `.tdraw` file of format `version` (the current one
    /// for a newer number), for an older TraceDraw: the current layout is
    /// taken down one step at a time ([`DOWN_STEPS`]).
    pub fn to_json_version(&self, version: u32) -> Result<String> {
        let mut out = Vec::new();
        self.write_json_version(&mut out, version)?;
        // serde_json writes UTF-8 only.
        Ok(String::from_utf8(out)
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))
    }

    /// Write the `.tdraw` JSON of format `version` to `w` as it is made.
    /// The current format goes straight to the writer; only an older one
    /// builds the whole tree in memory first (many times the drawing's
    /// size), to take it down.
    pub fn write_json_version<W: std::io::Write>(&self, w: W, version: u32) -> Result<()> {
        if (version as usize) >= DOWN_STEPS.len() {
            #[derive(Serialize)]
            struct Versioned<'a> {
                #[serde(flatten)]
                doc: &'a Document,
                format_version: u32,
            }
            serde_json::to_writer_pretty(
                w,
                &Versioned {
                    doc: self,
                    format_version: TDRAW_FORMAT_VERSION,
                },
            )?;
            return Ok(());
        }
        let mut v = serde_json::to_value(self)?;
        if let serde_json::Value::Object(m) = &mut v {
            m.insert(
                FORMAT_VERSION_KEY.into(),
                serde_json::Value::from(TDRAW_FORMAT_VERSION),
            );
        }
        for step in DOWN_STEPS.iter().skip(version as usize).rev() {
            step(&mut v);
        }
        serde_json::to_writer_pretty(w, &v)?;
        Ok(())
    }

    /// Read a `.tdraw` file of any version: older ones are brought up to
    /// date by [`migrate`]; newer ones are read as far as this version
    /// knows them (fields it does not know are left out).
    pub fn from_json(s: &str) -> Result<Self> {
        let mut v: serde_json::Value = serde_json::from_str(s)?;
        let from = tdraw_version(&v);
        migrate(&mut v, from);
        if let serde_json::Value::Object(m) = &mut v {
            m.remove(FORMAT_VERSION_KEY);
        }
        let mut doc: Document = serde_json::from_value(v)?;
        // What text wraps around is not saved: work it out again.
        doc.refresh_text_flow();
        Ok(doc)
    }
}

/// The `.tdraw` format's version, written into every file. A change that
/// an older TraceDraw would read wrongly (a field renamed, moved or given
/// another meaning) raises it and adds a step to [`migrate`]; a new field
/// with a default does not. The next version is 2: 1 was written by
/// TraceDraw 0.5 to 0.9 for the layout of 0 and is read as 0. See
/// `docs/tdraw-format.md`.
pub const TDRAW_FORMAT_VERSION: u32 = 0;

/// The number TraceDraw 0.5 to 0.9 wrote for the layout of version 0.
const SAME_AS_ZERO: u64 = 1;

const FORMAT_VERSION_KEY: &str = "format_version";

/// The format version a `.tdraw` file's JSON says it has: 0 for files
/// written before versions were recorded, and for the 1 of TraceDraw 0.5
/// to 0.9 (the same layout).
pub fn tdraw_version(v: &serde_json::Value) -> u32 {
    v.get(FORMAT_VERSION_KEY)
        .and_then(serde_json::Value::as_u64)
        .map(|n| if n == SAME_AS_ZERO { 0 } else { n })
        .map(|n| n.min(u64::from(u32::MAX)) as u32)
        .unwrap_or(0)
}

/// The version a `.tdraw` text says it has (0 for unreadable text).
pub fn tdraw_text_version(s: &str) -> u32 {
    serde_json::from_str::<serde_json::Value>(s)
        .map(|v| tdraw_version(&v))
        .unwrap_or(0)
}

/// One step of [`migrate`]: the JSON of version `i` to version `i + 1`.
type Step = fn(&mut serde_json::Value);

/// `STEPS[i]` takes a file from version `i` to `i + 1` (none yet).
const STEPS: [Step; TDRAW_FORMAT_VERSION as usize] = [];

/// `DOWN_STEPS[i]` takes a file from version `i + 1` back to `i`, for
/// saving in an older version (none yet).
const DOWN_STEPS: [Step; TDRAW_FORMAT_VERSION as usize] = [];

/// The TraceDraw release that first wrote each `.tdraw` format version
/// (`TDRAW_RELEASES[v]`), for Save As.
pub const TDRAW_RELEASES: [&str; TDRAW_FORMAT_VERSION as usize + 1] = ["0.4"];

/// Bring a `.tdraw` file's JSON from version `from` up to
/// [`TDRAW_FORMAT_VERSION`], one step at a time.
pub fn migrate(v: &mut serde_json::Value, from: u32) {
    for step in STEPS.iter().skip(from as usize) {
        step(v);
    }
}

/// Maps keyed by an index, read whether the keys come as numbers or as
/// the strings JSON object keys are (inside a shape nested in a tagged
/// kind, such as a group's child, they arrive as strings and a plain
/// number key refused them, so such drawings did not open again).
mod index_keys {
    use serde::de::{Deserializer, MapAccess, Visitor};
    use std::collections::BTreeMap;
    use std::fmt;
    use std::marker::PhantomData;

    struct Key(usize);

    impl<'de> serde::Deserialize<'de> for Key {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct K;
            impl<'de> Visitor<'de> for K {
                type Value = Key;
                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str("an index as a number or a string")
                }
                fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Key, E> {
                    usize::try_from(v).map(Key).map_err(E::custom)
                }
                fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Key, E> {
                    usize::try_from(v).map(Key).map_err(E::custom)
                }
                fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Key, E> {
                    v.trim().parse().map(Key).map_err(E::custom)
                }
            }
            d.deserialize_any(K)
        }
    }

    pub fn deserialize<'de, D, V>(d: D) -> Result<BTreeMap<usize, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: serde::Deserialize<'de>,
    {
        struct M<V>(PhantomData<V>);
        impl<'de, V: serde::Deserialize<'de>> Visitor<'de> for M<V> {
            type Value = BTreeMap<usize, V>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a map keyed by index")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> Result<Self::Value, A::Error> {
                let mut out = BTreeMap::new();
                while let Some((Key(k), v)) = m.next_entry::<Key, V>()? {
                    out.insert(k, v);
                }
                Ok(out)
            }
        }
        d.deserialize_map(M(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn node_types_of_curves_inside_groups_survive_the_native_format() {
        use crate::geometry::BezPath;
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.curve_to((1.0, 2.0), (3.0, 2.0), (4.0, 0.0));
        path.curve_to((5.0, -2.0), (7.0, -2.0), (8.0, 0.0));
        let mut curve = Shape::new(
            ShapeId(2),
            ShapeKind::Path {
                path,
                closed: false,
            },
        );
        curve.node_types.insert(3, crate::nodes::NodeType::Smooth);
        let group = Shape::new(
            ShapeId(1),
            ShapeKind::Group {
                children: vec![curve.clone()],
            },
        );
        let mut doc = Document::default();
        doc.pages[0].layers[0].shapes = vec![curve, group];
        let json = doc.to_json().expect("json");
        let back = Document::from_json(&json).expect("opens again");
        let shapes = &back.pages[0].layers[0].shapes;
        assert_eq!(
            shapes[0].node_types.get(&3),
            Some(&crate::nodes::NodeType::Smooth)
        );
        let ShapeKind::Group { children } = &shapes[1].kind else {
            panic!("a group");
        };
        assert_eq!(
            children[0].node_types.get(&3),
            Some(&crate::nodes::NodeType::Smooth)
        );
    }

    use super::*;

    /// Variable font axis values (#870) are set, replaced and taken off
    /// per tag, saved with the text in the native format and left out of
    /// it when none is set.
    #[test]
    fn variable_font_axes_are_kept_with_the_text() {
        let mut sp = TextSpan::new("Hello", "Quicksand", 24.0);
        sp.set_variation("wght", Some(550.0));
        sp.set_variation("wdth", Some(80.0));
        sp.set_variation("wght", Some(620.0));
        sp.set_variation("XOPQ", Some(f64::NAN));
        assert_eq!(sp.variation("wght"), Some(620.0));
        assert_eq!(sp.variations.len(), 2);
        sp.set_variation("wdth", None);
        assert_eq!(sp.variation("wdth"), None);

        let text = |sp: TextSpan| {
            Shape::new(
                ShapeId(1),
                ShapeKind::Text {
                    spans: vec![sp],
                    origin: crate::geometry::Point::new(0.0, 0.0),
                    frame: None,
                    align: TextAlign::Left,
                    para: ParagraphStyle::default(),
                    on_path: None,
                },
            )
        };
        let mut doc = Document::default();
        doc.pages[0].layers[0].shapes = vec![text(sp.clone())];
        let json = doc.to_json().expect("json");
        assert!(json.contains("\"wght\""));
        let back = Document::from_json(&json).expect("opens again");
        let ShapeKind::Text { spans, .. } = &back.pages[0].layers[0].shapes[0].kind else {
            panic!("a text");
        };
        assert_eq!(spans[0].variations, sp.variations);

        doc.pages[0].layers[0].shapes = vec![text(TextSpan::new("Hello", "Quicksand", 24.0))];
        let json = doc.to_json().expect("json");
        assert!(!json.contains("variations"));
        // Files written before keep opening.
        let back = Document::from_json(&json).expect("opens again");
        let ShapeKind::Text { spans, .. } = &back.pages[0].layers[0].shapes[0].kind else {
            panic!("a text");
        };
        assert!(spans[0].variations.is_empty());
    }

    /// Imported rounded corners keep their size (#619): a corner larger
    /// than the shorter side stays as it was, and a stretched rectangle
    /// takes the stretch into its size, its corners round at the smaller
    /// scale.
    #[test]
    fn keep_corner_size_leaves_the_outline_and_takes_the_stretch() {
        let mut ids = IdSource::default();
        let corners = Corners {
            radii: [30.0, 30.0, 0.0, 0.0],
            kind: CornerKind::Round,
            fixed: false,
        };
        let mut s = Shape::new(
            ids.shape(),
            ShapeKind::rect_with_corners(Rect::new(0.0, 0.0, 100.0, 40.0), corners),
        );
        s.transform = Affine::translate((5.0, 5.0));
        let before = s.page_path().area();
        s.keep_corner_size();
        let page = s.page_corners().expect("corners");
        assert!(page.fixed);
        assert_eq!(page.radii, [30.0, 30.0, 0.0, 0.0]);
        assert!((s.page_path().area() - before).abs() < 1e-6);
        // Stretched 2 x 1: the size takes it, the corners 2 mm round.
        let mut t = Shape::new(
            ids.shape(),
            ShapeKind::rect_with_corners(
                Rect::new(0.0, 0.0, 20.0, 10.0),
                Corners::uniform(2.0, CornerKind::Round),
            ),
        );
        t.transform = Affine::translate((5.0, 5.0)) * Affine::scale_non_uniform(2.0, 1.0);
        let bounds = t.bounds();
        t.keep_corner_size();
        let (sx, sy) = axis_scale(t.transform);
        assert!((sx - 1.0).abs() < 1e-9 && (sy - 1.0).abs() < 1e-9);
        assert!(
            matches!(t.kind, ShapeKind::Rect { rect, .. } if (rect.width() - 40.0).abs() < 1e-9)
        );
        let b = t.bounds();
        assert!((b.x0 - bounds.x0).abs() < 1e-9 && (b.x1 - bounds.x1).abs() < 1e-9);
        assert_eq!(t.page_corners().expect("corners").radii, [2.0; 4]);
        // An outline that scales with the object keeps its width: the
        // stretch stays in the transform.
        let mut u = Shape::new(
            ids.shape(),
            ShapeKind::rect_with_corners(
                Rect::new(0.0, 0.0, 20.0, 10.0),
                Corners::uniform(2.0, CornerKind::Round),
            ),
        );
        u.transform = Affine::scale_non_uniform(2.0, 1.0);
        let mut stroke = Stroke::default();
        stroke.scale_with_object = true;
        u.stroke = Some(stroke);
        let before = u.transform;
        u.keep_corner_size();
        assert_eq!(u.transform, before);
        assert!(u.page_corners().expect("corners").fixed);
        assert_eq!(u.page_corners().expect("corners").radii, [2.0; 4]);
        // A skewed one is left as it was.
        let mut k = Shape::new(
            ids.shape(),
            ShapeKind::rect_with_corners(
                Rect::new(0.0, 0.0, 20.0, 10.0),
                Corners::uniform(2.0, CornerKind::Round),
            ),
        );
        k.transform = Affine::new([1.0, 0.0, 0.5, 1.0, 0.0, 0.0]);
        let kind = k.kind.clone();
        k.keep_corner_size();
        assert_eq!(k.kind, kind);
    }

    /// A clip frame's warp is baked before the warps of what it holds, as
    /// they are drawn, and its frame keeps its own envelope: the contents
    /// are clipped to the frame's outline without it (#647).
    #[test]
    fn a_clip_frame_bakes_its_warp_before_the_warps_inside() {
        use crate::live::{evaluate, Effect};
        let b = Rect::new(0.0, 0.0, 40.0, 20.0);
        let arch = |b: Rect| {
            let mut nodes = crate::envelope::default_nodes(b);
            nodes[5].y += 10.0;
            let f = crate::envelope::Frame::from_parts(&nodes, &[], &[]).expect("frame");
            Effect::from_envelope(&f, false, crate::envelope::Mapping::Putty)
        };
        let mut inner = Shape::new(
            ShapeId(2),
            ShapeKind::Rect {
                rect: b,
                radius: 0.0,
                corners: None,
            },
        );
        inner.effects = vec![arch(b)];
        let mut content = Shape::new(
            ShapeId(3),
            ShapeKind::Rect {
                rect: Rect::new(5.0, 2.0, 35.0, 18.0),
                radius: 0.0,
                corners: None,
            },
        );
        content.effects = vec![Effect::Perspective {
            corners: [
                Point::new(5.0, 2.0),
                Point::new(35.0, 2.0),
                Point::new(28.0, 18.0),
                Point::new(12.0, 18.0),
            ],
        }];
        let mut clip = Shape::new(
            ShapeId(1),
            ShapeKind::ClipFrame {
                frame: Box::new(inner),
                contents: vec![content],
            },
        );
        clip.effects = vec![arch(b)];
        let drawn = evaluate(&clip).main;
        let ShapeKind::ClipFrame {
            frame: drawn_frame,
            contents: drawn_contents,
        } = &drawn.kind
        else {
            panic!("a clip frame");
        };
        let want = evaluate(&drawn_contents[0]).main.local_path();
        let mut n = 10;
        let got = clip
            .converted_to_curves(&[], &mut || {
                n += 1;
                ShapeId(n)
            })
            .expect("converted");
        assert!(got.effects.is_empty());
        let ShapeKind::ClipFrame { frame, contents } = &got.kind else {
            panic!("still a clip frame: {:?}", got.kind);
        };
        assert!(contents[0].effects.is_empty());
        assert!(matches!(contents[0].kind, ShapeKind::Path { .. }));
        assert!(crate::live::same_outline(
            &contents[0].local_path(),
            &want,
            0.02
        ));
        assert_eq!(frame.effects.len(), 1, "the frame keeps its envelope");
        assert_eq!(
            frame.local_path().elements(),
            drawn_frame.local_path().elements()
        );
        // A clip frame whose only warp is its frame's has nothing to convert.
        let mut only_frame = got.clone();
        if let ShapeKind::ClipFrame { contents, .. } = &mut only_frame.kind {
            contents[0].effects.clear();
        }
        assert!(!only_frame.converts_to_curves());
        assert!(!only_frame.has_outline_warp());
    }

    /// The objects in a group whose contour stays live keep their warps:
    /// baking them would change the contour drawn around them.
    #[test]
    fn warps_under_a_live_contour_stay() {
        use crate::live::Effect;
        let mut child = Shape::new(
            ShapeId(2),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                radius: 0.0,
                corners: None,
            },
        );
        child.effects = vec![Effect::Distort(crate::effects::Distort::Twister {
            angle_deg: 40.0,
            center: None,
        })];
        let mut g = Shape::new(
            ShapeId(1),
            ShapeKind::Group {
                children: vec![child],
            },
        );
        g.effects = vec![Effect::BlockShadow {
            offset: crate::geometry::Vec2::new(2.0, -2.0),
            color: crate::Color::BLACK,
            gap: 0.0,
        }];
        let clip = Shape::new(
            ShapeId(5),
            ShapeKind::ClipFrame {
                frame: Box::new(Shape::new(
                    ShapeId(6),
                    ShapeKind::Rect {
                        rect: Rect::new(0.0, 0.0, 30.0, 30.0),
                        radius: 0.0,
                        corners: None,
                    },
                )),
                contents: vec![g],
            },
        );
        let mut n = 10;
        let got = clip
            .converted_to_curves(&[], &mut || {
                n += 1;
                ShapeId(n)
            })
            .expect("the frame becomes a curve");
        let ShapeKind::ClipFrame { contents, .. } = &got.kind else {
            panic!("a clip frame");
        };
        let ShapeKind::Group { children } = &contents[0].kind else {
            panic!("a group");
        };
        assert_eq!(children[0].effects.len(), 1, "the twister stays live");
        assert!(matches!(children[0].kind, ShapeKind::Path { .. }));
    }

    #[test]
    fn further_fills_and_outlines_draw_in_layers() {
        use crate::{Color, Stroke};
        let mut s = Shape::new(
            ShapeId(1),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        assert!(s.layers().is_none());
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        s.stroke = Some(Stroke::new(Color::BLACK, 1.0));
        s.extra_fills = vec![Fill::Solid(Color::rgb8(0, 0, 255))];
        let mut behind = Stroke::new(Color::WHITE, 3.0);
        behind.behind_fill = true;
        s.extra_strokes = vec![behind, Stroke::new(Color::rgb8(0, 255, 0), 0.5)];
        let parts = s.layers().expect("parts");
        // Behind outline, fill, further fill, outline, further outline.
        assert_eq!(parts.len(), 5);
        assert!(parts[0].stroke.as_ref().is_some_and(|st| st.behind_fill));
        assert_eq!(parts[1].fill, Fill::Solid(Color::rgb8(255, 0, 0)));
        assert_eq!(parts[2].fill, Fill::Solid(Color::rgb8(0, 0, 255)));
        assert!(parts[3]
            .stroke
            .as_ref()
            .is_some_and(|st| st.color == Color::BLACK));
        assert!(parts.iter().all(|p| p.layers().is_none()));
        assert!(parts[1..].iter().all(|p| p.shadow.is_none()));
        let back: Shape = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.extra_strokes.len(), 2);
    }
    use crate::geometry::Point;

    #[test]
    fn tdraw_files_carry_their_format_version_and_older_ones_open() {
        let doc = Document::new("v", crate::geometry::Size::new(100.0, 50.0));
        let json = doc.to_json().expect("json");
        assert_eq!(tdraw_text_version(&json), TDRAW_FORMAT_VERSION);
        assert_eq!(Document::from_json(&json).expect("load").title, "v");
        // Before versions were recorded: version 0, same layout.
        let mut old: serde_json::Value = serde_json::from_str(&json).expect("value");
        old.as_object_mut()
            .expect("object")
            .remove("format_version");
        assert_eq!(tdraw_version(&old), 0);
        assert_eq!(
            Document::from_json(&old.to_string()).expect("old").title,
            "v"
        );
        // The 1 that TraceDraw 0.5 to 0.9 wrote is the layout of 0.
        let mut one = old.clone();
        one.as_object_mut()
            .expect("object")
            .insert("format_version".into(), 1.into());
        assert_eq!(tdraw_version(&one), 0);
        assert_eq!(
            Document::from_json(&one.to_string()).expect("one").title,
            "v"
        );
        // A newer file: unknown fields are left out, the rest reads.
        let mut newer = old.clone();
        let m = newer.as_object_mut().expect("object");
        m.insert("format_version".into(), 99.into());
        m.insert("something_new".into(), serde_json::json!({"a": 1}));
        assert_eq!(
            Document::from_json(&newer.to_string())
                .expect("newer")
                .title,
            "v"
        );
        assert_eq!(tdraw_text_version("not json"), 0);
    }

    /// A circle of radius `r` about the origin, from its top clockwise.
    fn clockwise_circle(r: f64) -> BezPath {
        let mut p = BezPath::new();
        let n = 64;
        for i in 0..=n {
            let a = std::f64::consts::FRAC_PI_2 - i as f64 / n as f64 * std::f64::consts::TAU;
            let q = Point::new(r * a.cos(), r * a.sin());
            if i == 0 {
                p.move_to(q);
            } else {
                p.line_to(q);
            }
        }
        p.close_path();
        p
    }

    /// The file is written as it is made, and reads as the same JSON the
    /// tree built in memory gave.
    #[test]
    fn tdraw_json_streams_to_the_writer() {
        let mut doc = Document::new("Streamed", crate::geometry::Size::new(100.0, 50.0));
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 5.0),
                radius: 1.0,
                corners: None,
            },
        );
        s.fill = crate::Fill::Solid(crate::Color::rgb8(10, 20, 30));
        doc.layer_mut(layer).expect("layer").shapes.push(s);
        // A writer that sees the text in pieces, never all at once.
        struct Pieces {
            text: Vec<u8>,
            writes: usize,
        }
        impl std::io::Write for Pieces {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.writes += 1;
                self.text.extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut w = Pieces {
            text: Vec::new(),
            writes: 0,
        };
        doc.write_json_version(&mut w, TDRAW_FORMAT_VERSION)
            .expect("written");
        assert!(w.writes > 10, "{} writes", w.writes);
        // The tree printed f32 values through f64 (0.6000000238418579
        // for 0.6); both read back as the same drawing.
        let mut tree = serde_json::to_value(&doc).expect("value");
        tree.as_object_mut().expect("object").insert(
            FORMAT_VERSION_KEY.into(),
            serde_json::Value::from(TDRAW_FORMAT_VERSION),
        );
        let streamed = std::str::from_utf8(&w.text).expect("utf-8");
        let a = Document::from_json(streamed).expect("streamed");
        let b = Document::from_json(&tree.to_string()).expect("tree");
        assert_eq!(
            serde_json::to_value(&a).expect("a"),
            serde_json::to_value(&b).expect("b")
        );
        assert_eq!(doc.to_json().expect("json").as_bytes(), &w.text[..]);
        let back = a;
        assert_eq!(back.pages, doc.pages);
    }

    #[test]
    fn tdraw_files_can_be_written_in_an_older_format_version() {
        let doc = Document {
            title: "Older".into(),
            ..Default::default()
        };
        for version in 0..=TDRAW_FORMAT_VERSION {
            let json = doc.to_json_version(version).expect("json");
            assert_eq!(tdraw_text_version(&json), version);
            assert_eq!(Document::from_json(&json).expect("reads").title, "Older");
        }
        let newer = doc.to_json_version(TDRAW_FORMAT_VERSION + 5).expect("json");
        assert_eq!(tdraw_text_version(&newer), TDRAW_FORMAT_VERSION);
        assert_eq!(TDRAW_RELEASES.len(), TDRAW_FORMAT_VERSION as usize + 1);
    }

    #[test]
    fn text_baselines_wrap_round_closed_paths_and_move_by_the_distance() {
        let circle = clockwise_circle(10.0);
        let plain = Baseline::new(&circle, 0.0, false);
        assert!(plain.is_closed());
        let total = plain.length();
        // 64 chords of a radius 10 circle.
        let chords = 64.0 * 2.0 * 10.0 * (std::f64::consts::PI / 64.0).sin();
        assert!((total - chords).abs() < 1e-9, "{total}");
        // Past the end: round again from the start.
        let (a, _) = plain.at(1.0).expect("point");
        let (b, _) = plain.at(total + 1.0).expect("point");
        let (c, _) = plain.at(1.0 - total).expect("point");
        assert!((a - b).hypot() < 1e-9 && (a - c).hypot() < 1e-9);
        // Clockwise, the left is outwards: 2 mm out, radius 12.
        let out = Baseline::new(&circle, 2.0, false);
        assert!((out.distance_to(Point::new(0.0, 12.0)).expect("d")).abs() < 0.05);
        assert!((out.length() / total - 1.2).abs() < 1e-3);
        // (The corners go a little further, keeping the sides 2 away.)
        let (top, dir) = out.at(0.0).expect("point");
        assert!((top - Point::new(0.0, 12.0)).hypot() < 0.01, "{top:?}");
        assert!(dir.x > 0.99, "{dir:?}");
        // Followed backwards the left is inwards: radius 8, still from
        // the top, now running to the left.
        let back = Baseline::new(&circle, 2.0, true);
        let (top, dir) = back.at(0.0).expect("point");
        assert!((top - Point::new(0.0, 8.0)).hypot() < 0.01, "{top:?}");
        assert!(dir.x < -0.99, "{dir:?}");
    }

    #[test]
    fn open_text_baselines_stop_at_their_ends_and_corners_keep_the_distance() {
        let mut l = BezPath::new();
        l.move_to((0.0, 0.0));
        l.line_to((10.0, 0.0));
        l.line_to((10.0, 10.0));
        // A second subpath is not followed.
        l.move_to((50.0, 50.0));
        l.line_to((60.0, 50.0));
        let b = Baseline::new(&l, 0.0, false);
        assert!(!b.is_closed());
        assert!((b.length() - 20.0).abs() < 1e-9);
        assert_eq!(b.at(-5.0).map(|(p, _)| p), Some(Point::new(0.0, 0.0)));
        assert_eq!(b.at(99.0).map(|(p, _)| p), Some(Point::new(10.0, 10.0)));
        // One to the left of the bend (the inside): its corner at (9, 1).
        let moved = Baseline::new(&l, 1.0, false);
        let (corner, _) = moved.at(9.0).expect("point");
        assert!((corner - Point::new(9.0, 1.0)).hypot() < 1e-9, "{corner:?}");
        // Nothing to follow: no length, no points, no panic.
        let empty = Baseline::new(&BezPath::new(), 3.0, true);
        assert_eq!(empty.length(), 0.0);
        assert_eq!(empty.at(1.0), None);
        let mut bad = BezPath::new();
        bad.move_to((f64::NAN, 0.0));
        bad.line_to((1.0, f64::INFINITY));
        assert_eq!(Baseline::new(&bad, 1.0, false).at(0.0), None);
    }

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
    fn time_tracker_round_trips_and_is_left_out_when_empty() {
        let mut doc = Document::default();
        let plain = doc.to_json().unwrap();
        assert!(!plain.contains("time_tracker"));
        doc.time_tracker.tasks.push(TrackedTask {
            name: "Sketch, \"first\" pass".into(),
            seconds: 4000.5,
            start: Some(1_760_000_000),
            finish: Some(1_760_004_000),
        });
        doc.time_tracker.tasks.push(TrackedTask {
            name: "Colour".into(),
            ..Default::default()
        });
        doc.time_tracker.active = Some(1);
        let back = Document::from_json(&doc.to_json().unwrap()).unwrap();
        assert_eq!(back, doc);
        assert_eq!(
            back.time_tracker.active_task().map(|t| t.name.as_str()),
            Some("Colour")
        );
        // Files from before the timer existed still open.
        let old = Document::from_json(&plain).unwrap();
        assert!(old.time_tracker.is_empty());
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
            png: vec![1, 2, 3, 4].into(),
            fx,
        };
        let mut params = std::collections::BTreeMap::new();
        params.insert("radius".to_string(), 3.5);
        let stack = BitmapFxStack {
            rect: Rect::new(1.0, 1.0, 19.0, 9.0),
            width_px: 3,
            height_px: 1,
            png: vec![9, 8, 7].into(),
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
    fn a_single_corner_can_take_a_whole_side() {
        // One corner of 70 mm on a 56 x 37 mm rectangle (as a file had
        // it): it shrinks to the shorter side, 37 mm along both edges.
        let mut c = Corners::default();
        c.radii[Corners::TOP_LEFT] = 70.0;
        let rect = Rect::new(0.0, 0.0, 56.0, 37.0);
        let ends = c.ends(rect, (1.0, 1.0));
        assert_eq!(
            ends[Corners::TOP_LEFT],
            (
                Point::new(0.0, 37.0),
                Point::new(0.0, 0.0),
                Point::new(37.0, 37.0)
            )
        );
        // With a second corner every corner shrinks by the factor of the
        // tightest side (the left one: 37 / 70).
        c.radii[Corners::TOP_RIGHT] = 30.0;
        let ends = c.ends(rect, (1.0, 1.0));
        let k = 37.0 / 70.0;
        assert!((ends[Corners::TOP_LEFT].2.x - 70.0 * k).abs() < 1e-9);
        assert!((ends[Corners::TOP_RIGHT].2.x - (56.0 - 30.0 * k)).abs() < 1e-9);
        assert!((ends[Corners::TOP_RIGHT].1.y - (37.0 - 30.0 * k)).abs() < 1e-9);
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
        // Formats with one radius get the stretch moved into the size, so
        // the corners stay circles of 4 mm.
        let (rect, radius, place) = s.plain_rect().expect("one radius");
        assert_eq!(rect, Rect::new(0.0, 0.0, 60.0, 20.0));
        assert_eq!(radius, r);
        let same_place = place * Point::new(60.0, 20.0) - s.transform * Point::new(20.0, 10.0);
        assert!(same_place.hypot() < 1e-9, "{place:?}");
        assert_eq!(axis_scale(place), (1.0, 1.0));
        // Not when that would thin a scaling outline, nor on skewed axes.
        let mut outlined = s.clone();
        outlined.stroke = Some(crate::style::Stroke {
            scale_with_object: true,
            ..Default::default()
        });
        assert_eq!(outlined.plain_rect(), None);
        let mut skewed = s.clone();
        skewed.transform = Affine::new([3.0, 0.0, 1.0, 2.0, 0.0, 0.0]);
        assert_eq!(skewed.plain_rect(), None);
        // Scaling corners stretch with the object.
        let mut s = rect_shape(20.0, 10.0, Corners::uniform(r, CornerKind::Round));
        s.transform = Affine::scale_non_uniform(3.0, 2.0);
        let a = area(&s);
        assert!((a - (1200.0 - 6.0 * (4.0 - pi) * r * r)).abs() < 0.1, "{a}");
        assert_eq!(s.page_corners().unwrap().radii, [2.0 * r; 4]);
        assert_eq!(
            s.plain_rect(),
            Some((Rect::new(0.0, 0.0, 20.0, 10.0), r, s.transform))
        );
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
        assert_eq!(
            s.plain_rect(),
            Some((Rect::new(0.0, 0.0, 20.0, 10.0), 3.0, s.transform))
        );
        // Fixed corners are stored as page sizes, limited by the page box.
        let mut c = Corners::uniform(50.0, CornerKind::Scallop);
        c.fixed = true;
        let kind = s.with_page_corners(c).unwrap();
        assert_eq!(kind.rect_corners().unwrap().1.radii, [10.0; 4]);
        // A fixed round radius on a uniformly scaled rectangle is still one radius.
        c.kind = CornerKind::Round;
        c.radii = [4.0; 4];
        s.kind = s.with_page_corners(c).unwrap();
        assert_eq!(
            s.plain_rect(),
            Some((Rect::new(0.0, 0.0, 20.0, 10.0), 2.0, s.transform))
        );
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
            twist: 0.0,
        };
        let back: ShapeKind = serde_json::from_str(&serde_json::to_string(&k).unwrap()).unwrap();
        assert_eq!(back, k);
        // A twist is written only when there is one, and read back.
        assert!(!serde_json::to_string(&old).unwrap().contains("twist"));
        let twisted = ShapeKind::Polygon {
            rect: Rect::new(0.0, 0.0, 2.0, 2.0),
            points: 5,
            sharpness: 0.4,
            complex: None,
            twist: 0.25,
        };
        let json = serde_json::to_string(&twisted).unwrap();
        assert!(json.contains("twist"));
        assert_eq!(serde_json::from_str::<ShapeKind>(&json).unwrap(), twisted);
    }

    #[test]
    fn the_outline_after_a_transform_is_what_applying_it_gives() {
        for fixed in [true, false] {
            let mut c = Corners::uniform(4.0, CornerKind::Round);
            c.fixed = fixed;
            let s = rect_shape(20.0, 10.0, c);
            for t in [
                Affine::scale_non_uniform(3.0, 1.0),
                Affine::rotate(0.4) * Affine::scale_non_uniform(1.0, 2.5),
                Affine::translate((5.0, -2.0)),
            ] {
                let mut applied = s.clone();
                applied.transform = t * applied.transform;
                let want = applied.page_path();
                let got = s.page_path_after(t);
                assert_eq!(got, want, "fixed {fixed} {t:?}");
            }
            // Fixed corners stay round in the preview: the plain stretched
            // outline (what the preview used to draw) differs.
            if fixed {
                let t = Affine::scale_non_uniform(3.0, 1.0);
                assert_ne!(s.page_path_after(t), t * s.page_path());
            }
        }
        // Other kinds: the outline with the transform applied.
        let mut plain = rect_shape(20.0, 10.0, Corners::uniform(0.0, CornerKind::Round));
        plain.kind = ShapeKind::Ellipse {
            rect: Rect::new(0.0, 0.0, 20.0, 10.0),
            arc: None,
        };
        let t = Affine::scale(2.0);
        assert_eq!(plain.page_path_after(t), t * plain.page_path());
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
        // Stretched by the parent: the same corners, now scaling with it.
        let s = check(Affine::IDENTITY, Affine::scale_non_uniform(2.0, 1.0));
        assert!(
            matches!(&s.kind, ShapeKind::Rect { corners: Some(k), .. } if !k.fixed && k.kind == CornerKind::Chamfer && k.radii == [3.0; 4])
        );
        // Scaled evenly by the parent: still fixed, grown by its factor,
        // also when the rectangle itself is stretched (no curve).
        for own in [Affine::IDENTITY, Affine::scale_non_uniform(2.0, 1.0)] {
            for parent in [
                Affine::scale(2.0),
                Affine::rotate(0.7) * Affine::scale(2.0),
                Affine::FLIP_X * Affine::scale(2.0),
            ] {
                let s = check(own, parent);
                assert!(
                    matches!(&s.kind, ShapeKind::Rect { corners: Some(k), .. } if k.fixed && k.kind == CornerKind::Chamfer && k.radii.iter().all(|r| (r - 6.0).abs() < 1e-9)),
                    "{:?}",
                    s.kind
                );
            }
        }
        // A stretched rectangle stretched again the other way: its outline
        // as a curve.
        let s = check(
            Affine::scale_non_uniform(2.0, 1.0),
            Affine::scale_non_uniform(1.0, 3.0),
        );
        assert!(matches!(s.kind, ShapeKind::Path { closed: true, .. }));
        // Moving and turning change nothing.
        let s = check(
            Affine::scale_non_uniform(2.0, 1.0),
            Affine::rotate(0.5) * Affine::translate((3.0, 4.0)),
        );
        assert!(matches!(&s.kind, ShapeKind::Rect { corners: Some(k), .. } if k.fixed));
    }

    /// Vertical text (#257): the flag loads from older files as off, is
    /// saved only when on, and places a frame's text from its top right.
    #[test]
    fn vertical_text_is_off_by_default_and_placed_from_the_top_right() {
        let old: ParagraphStyle = serde_json::from_str("{}").expect("parses");
        assert!(!old.vertical);
        let on: ParagraphStyle = serde_json::from_str(r#"{"vertical":true}"#).expect("parses");
        assert!(on.vertical);
        let json = serde_json::to_string(&ParagraphStyle::default()).expect("writes");
        assert!(!json.contains("vertical\""), "{json}");
        let json = serde_json::to_string(&on).expect("writes");
        assert!(json.contains("\"vertical\":true"), "{json}");
        let o = Point::new(10.0, 20.0);
        let f = Some(geometry::Size::new(50.0, 80.0));
        let flat = text_layout_shift(o, f, false, false) * Point::ZERO;
        let tall = text_layout_shift(o, f, false, true) * Point::ZERO;
        assert_eq!(flat, Point::new(10.0, 100.0));
        assert_eq!(tall, Point::new(60.0, 100.0));
        assert_eq!(text_layout_shift(o, None, false, true) * Point::ZERO, o);
        // The written lines' frame: a quarter turn clockwise.
        let p = vertical_line_frame() * Point::new(5.0, 1.0);
        assert!(
            (p.x - 1.0).abs() < 1e-9 && (p.y - -5.0).abs() < 1e-9,
            "{p:?}"
        );
    }

    #[test]
    fn carried_fonts_are_saved_and_shared_between_copies() {
        let mut doc = Document::default();
        // No fonts: nothing written, and older files load without them.
        let json = doc.to_json().expect("json");
        assert!(!json.contains("\"fonts\""));
        assert!(Document::from_json(&json).expect("load").fonts.is_empty());
        doc.fonts
            .push(DocumentFont::new("Test Sans", vec![0, 1, 0, 0, 7, 8, 9]));
        let back = Document::from_json(&doc.to_json().expect("json")).expect("load");
        assert_eq!(back.fonts, doc.fonts);
        // Copies (undo steps) share the bytes.
        let copy = doc.clone();
        assert!(std::sync::Arc::ptr_eq(
            &copy.fonts[0].data,
            &doc.fonts[0].data
        ));
        // Debug output names the font without dumping it.
        let shown = format!("{:?}", doc.fonts[0]);
        assert!(
            shown.contains("Test Sans") && shown.contains("bytes: 7"),
            "{shown}"
        );
        // Damaged base64 is an error, not a panic.
        let bad = r#"{"family":"X","data":"@@not base64@@"}"#;
        assert!(serde_json::from_str::<DocumentFont>(bad).is_err());
    }

    #[test]
    fn a_carried_colour_profile_is_saved_with_the_drawing() {
        let mut doc = Document::default();
        let json = doc.to_json().expect("json");
        // Not written when there is none (the metadata's profile name is
        // another field).
        let v: serde_json::Value = serde_json::from_str(&json).expect("value");
        assert!(v.get("cmyk_profile").is_none());
        assert!(Document::from_json(&json)
            .expect("load")
            .cmyk_profile
            .is_none());
        doc.cmyk_profile = Some(DocumentProfile::new("Press", vec![1, 2, 3, 4]));
        let back = Document::from_json(&doc.to_json().expect("json")).expect("load");
        assert_eq!(back.cmyk_profile, doc.cmyk_profile);
        let copy = doc.clone();
        assert!(std::sync::Arc::ptr_eq(
            &copy.cmyk_profile.as_ref().expect("profile").data,
            &doc.cmyk_profile.as_ref().expect("profile").data
        ));
        let shown = format!("{:?}", doc.cmyk_profile);
        assert!(
            shown.contains("Press") && shown.contains("bytes: 4"),
            "{shown}"
        );
    }

    #[test]
    fn the_families_used_include_nested_text_tables_and_symbols() {
        let text = |id: u64, family: &str| {
            Shape::new(
                ShapeId(id),
                ShapeKind::Text {
                    spans: vec![TextSpan::new("a", family, 12.0)],
                    origin: Point::ZERO,
                    frame: None,
                    align: Default::default(),
                    para: Default::default(),
                    on_path: None,
                },
            )
        };
        let mut doc = Document::default();
        let group = Shape::new(
            ShapeId(10),
            ShapeKind::Group {
                children: vec![text(11, "Inner"), text(12, "Top")],
            },
        );
        doc.pages[0].layers[0].shapes = vec![text(1, "Top"), group];
        doc.symbols.push(Symbol {
            name: "s".into(),
            shape: text(20, "In Symbol"),
        });
        assert_eq!(doc.font_families_used(), vec!["In Symbol", "Inner", "Top"]);
    }

    #[test]
    fn characters_with_their_own_colours_are_drawn_in_one_part_per_look() {
        use crate::color::Color;
        let red = Fill::Solid(Color::rgb8(255, 0, 0));
        let mut a = TextSpan::new("ab", "Arial", 12.0);
        a.background = Some(Fill::Solid(Color::rgb8(0, 255, 0)));
        let mut b = TextSpan::new("cd", "Arial", 12.0);
        b.fill = Some(red.clone());
        let mut c = TextSpan::new("ef", "Arial", 12.0);
        c.outline = Some(Stroke {
            width: 0.0,
            ..Stroke::default()
        });
        let d = TextSpan::new("gh", "Arial", 12.0);
        let mut shape = Shape::new(
            ShapeId(1),
            ShapeKind::Text {
                spans: vec![a, b, c, d],
                origin: Point::ZERO,
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
        );
        shape.fill = Fill::Solid(Color::BLACK);
        shape.shadow = Some(Shadow::default());
        let parts = shape.text_parts().expect("parts");
        // Black with the outline (ab, gh), red (cd), black without it (ef).
        assert_eq!(parts.len(), 3);
        let drawn = |p: &Shape| -> String {
            match &p.kind {
                ShapeKind::Text { spans, .. } => spans
                    .iter()
                    .filter(|s| !s.layout_only)
                    .map(|s| s.text.as_str())
                    .collect(),
                _ => String::new(),
            }
        };
        assert_eq!(drawn(&parts[0]), "abgh");
        assert_eq!(parts[0].fill, Fill::Solid(Color::BLACK));
        assert!(parts[0].stroke.is_some() && parts[0].shadow.is_some());
        assert_eq!(drawn(&parts[1]), "cd");
        assert_eq!(parts[1].fill, red);
        assert!(parts[1].shadow.is_none());
        assert_eq!(drawn(&parts[2]), "ef");
        assert!(parts[2].stroke.is_none());
        // Every part lays out the whole text, with no overrides or
        // backgrounds left, so it is not split again.
        for p in &parts {
            let ShapeKind::Text { spans, .. } = &p.kind else {
                panic!("text");
            };
            assert_eq!(spans_text(spans), "abcdefgh");
            assert!(spans
                .iter()
                .all(|s| s.fill.is_none() && s.background.is_none()));
            assert!(p.text_parts().is_none());
        }
        // A plain text and other shapes have no parts.
        let mut plain = shape.clone();
        if let ShapeKind::Text { spans, .. } = &mut plain.kind {
            spans.iter_mut().for_each(|s| {
                s.fill = None;
                s.outline = None;
            });
        }
        assert!(plain.text_parts().is_none());
        let rect = Shape::new(
            ShapeId(2),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                radius: 0.0,
                corners: None,
            },
        );
        assert!(rect.text_parts().is_none());
    }

    #[test]
    fn character_outlines_are_saved_and_layout_only_is_not() {
        let mut s = TextSpan::new("a", "Arial", 12.0);
        s.outline = Some(Stroke::default());
        s.layout_only = true;
        let json = serde_json::to_string(&s).expect("json");
        assert!(!json.contains("layout_only"), "{json}");
        let back: TextSpan = serde_json::from_str(&json).expect("back");
        assert_eq!(back.outline, Some(Stroke::default()));
        assert!(!back.layout_only);
        // Older files without the field read as no override.
        let old: TextSpan = serde_json::from_str(
            r#"{"text":"a","font_family":"Arial","size_pt":12.0,"bold":false,"italic":false}"#,
        )
        .expect("old");
        assert_eq!(old.outline, None);
    }
}
