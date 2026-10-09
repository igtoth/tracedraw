//! Fill and outline (stroke) properties of a shape.

use crate::color::Color;
use crate::geometry::{Affine, Point, Rect, Size};
use serde::{Deserialize, Serialize};

/// Fountain (gradient) fill geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FountainKind {
    #[default]
    Linear,
    Radial,
    Conical,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stop {
    /// 0.0 at the start, 1.0 at the end.
    pub pos: f64,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fountain {
    pub kind: FountainKind,
    /// At least two stops, sorted by position.
    pub stops: Vec<Stop>,
    /// Degrees, the target design's convention (0 = left to right, CCW positive).
    pub angle: f64,
    /// Centre offset in bounds-relative units (-1..1), for radial/conical/square.
    pub offset: Point,
    /// Edge padding in 0..0.49: fraction of the span held at the end colours.
    pub edge_pad: f64,
}

impl Fountain {
    pub fn two(kind: FountainKind, from: Color, to: Color, angle: f64) -> Self {
        Fountain {
            kind,
            stops: vec![
                Stop {
                    pos: 0.0,
                    color: from,
                },
                Stop {
                    pos: 1.0,
                    color: to,
                },
            ],
            angle,
            offset: Point::ZERO,
            edge_pad: 0.0,
        }
    }

    pub fn first_color(&self) -> Color {
        self.stops.first().map(|s| s.color).unwrap_or(Color::BLACK)
    }

    pub fn last_color(&self) -> Color {
        self.stops.last().map(|s| s.color).unwrap_or(Color::WHITE)
    }

    /// Colour at `t` in 0..1 (sRGB interpolation).
    pub fn color_at(&self, t: f64) -> Color {
        let t = if self.edge_pad > 0.0 {
            ((t - self.edge_pad) / (1.0 - 2.0 * self.edge_pad)).clamp(0.0, 1.0)
        } else {
            t.clamp(0.0, 1.0)
        };
        let mut stops: Vec<&Stop> = self.stops.iter().collect();
        stops.sort_by(|a, b| {
            a.pos
                .partial_cmp(&b.pos)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if stops.is_empty() {
            return Color::BLACK;
        }
        if t <= stops[0].pos {
            return stops[0].color;
        }
        for w in stops.windows(2) {
            if t <= w[1].pos {
                let span = (w[1].pos - w[0].pos).max(1e-9);
                let k = ((t - w[0].pos) / span) as f32;
                return lerp_color(w[0].color, w[1].color, k);
            }
        }
        stops[stops.len() - 1].color
    }
}

pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let [r1, g1, b1] = a.to_rgb8();
    let [r2, g2, b2] = b.to_rgb8();
    let l = |x: u8, y: u8| {
        (x as f32 + (y as f32 - x as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::rgb8(l(r1, r2), l(g1, g2), l(b1, b2))
}

/// Built-in two-colour pattern tiles (the target design's two-colour pattern fill).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PatternTile {
    #[default]
    Checker,
    Dots,
    Stripes,
    DiagonalStripes,
    Grid,
    Bricks,
    Crosshatch,
    Waves,
}

impl PatternTile {
    pub const ALL: [PatternTile; 8] = [
        PatternTile::Checker,
        PatternTile::Dots,
        PatternTile::Stripes,
        PatternTile::DiagonalStripes,
        PatternTile::Grid,
        PatternTile::Bricks,
        PatternTile::Crosshatch,
        PatternTile::Waves,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PatternTile::Checker => "Checker",
            PatternTile::Dots => "Dots",
            PatternTile::Stripes => "Stripes",
            PatternTile::DiagonalStripes => "Diagonal stripes",
            PatternTile::Grid => "Grid",
            PatternTile::Bricks => "Bricks",
            PatternTile::Crosshatch => "Crosshatch",
            PatternTile::Waves => "Waves",
        }
    }

    /// Is the tile pixel at (u, v) in 0..1 "front" (true) or "back" colour?
    pub fn front(self, u: f64, v: f64) -> bool {
        match self {
            PatternTile::Checker => (u < 0.5) == (v < 0.5),
            PatternTile::Dots => ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() < 0.3,
            PatternTile::Stripes => v < 0.5,
            PatternTile::DiagonalStripes => (u + v).rem_euclid(1.0) < 0.5,
            PatternTile::Grid => u < 0.12 || v < 0.12,
            PatternTile::Bricks => {
                let row = v < 0.5;
                let uu = if row { u } else { (u + 0.5).rem_euclid(1.0) };
                v.rem_euclid(0.5) < 0.06 || uu < 0.06
            }
            PatternTile::Crosshatch => {
                (u + v).rem_euclid(0.5) < 0.07 || (u - v).rem_euclid(0.5) < 0.07
            }
            PatternTile::Waves => ((v - 0.5) - 0.2 * (u * std::f64::consts::TAU).sin()).abs() < 0.1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "pattern", rename_all = "lowercase")]
pub enum Pattern {
    TwoColor {
        tile: PatternTile,
        front: Color,
        back: Color,
        size_mm: f64,
    },
    /// Tiled PNG, `size_mm` is the tile width; height follows the aspect ratio.
    Bitmap {
        #[serde(with = "crate::document::png_bytes")]
        png: Vec<u8>,
        width_px: u32,
        height_px: u32,
        size_mm: f64,
    },
    /// Full-colour vector tile (the target design's vector pattern fill):
    /// `shapes` are in tile-local coordinates, origin at the bottom-left
    /// corner of the tile, Y up, and the tile repeats every `tile.width` by
    /// `tile.height` millimetres. The shapes carry their own transforms,
    /// fills and outlines; there is no extra pattern transform.
    Vector {
        #[serde(default)]
        shapes: Vec<crate::document::Shape>,
        tile: Size,
    },
}

impl Pattern {
    /// Smallest tile edge, in mm, so a degenerate selection still tiles.
    pub const MIN_TILE_MM: f64 = 0.01;

    /// Build a vector pattern from page-space shapes: the shapes are moved
    /// so their joint bounds start at the origin and the tile takes the
    /// size of those bounds.
    pub fn vector_from_shapes(shapes: &[crate::document::Shape]) -> Pattern {
        let mut bounds: Option<Rect> = None;
        for s in shapes {
            let b = s.bounds();
            if !(b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite()) {
                continue;
            }
            bounds = Some(match bounds {
                Some(acc) => acc.union(b),
                None => b,
            });
        }
        let bounds = bounds.unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        let shift = Affine::translate((-bounds.x0, -bounds.y0));
        let shapes = shapes
            .iter()
            .map(|s| {
                let mut s = s.clone();
                s.transform = shift * s.transform;
                s
            })
            .collect();
        Pattern::Vector {
            shapes,
            tile: Size::new(
                bounds.width().max(Self::MIN_TILE_MM),
                bounds.height().max(Self::MIN_TILE_MM),
            ),
        }
    }
}

/// Procedural textures (the target design's texture fill, a small subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TextureKind {
    #[default]
    Clouds,
    Marble,
    Noise,
    Wood,
}

impl TextureKind {
    pub const ALL: [TextureKind; 4] = [
        TextureKind::Clouds,
        TextureKind::Marble,
        TextureKind::Noise,
        TextureKind::Wood,
    ];
    pub fn name(self) -> &'static str {
        match self {
            TextureKind::Clouds => "Clouds",
            TextureKind::Marble => "Marble",
            TextureKind::Noise => "Noise",
            TextureKind::Wood => "Wood",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Texture {
    pub kind: TextureKind,
    pub color_a: Color,
    pub color_b: Color,
    /// Feature size in mm.
    pub scale: f64,
    pub seed: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "fill", rename_all = "lowercase")]
pub enum Fill {
    #[default]
    None,
    Solid(Color),
    Fountain(Fountain),
    Pattern(Pattern),
    Texture(Texture),
    Mesh(Mesh),
}

/// Mesh fill: a grid of (rows+1) x (cols+1) nodes with a colour each;
/// cells are bilinear patches (Gouraud) clipped by the object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mesh {
    pub rows: u32,
    pub cols: u32,
    /// Row-major, (rows+1)*(cols+1) nodes in local space.
    pub nodes: Vec<MeshNode>,
    #[serde(default)]
    pub smooth: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeshNode {
    pub pos: Point,
    pub color: Color,
    /// 1.0 = opaque.
    #[serde(default = "one_f32")]
    pub alpha: f32,
}

fn one_f32() -> f32 {
    1.0
}

impl Mesh {
    pub fn new(bounds: crate::geometry::Rect, rows: u32, cols: u32, base: Color) -> Self {
        let mut nodes = Vec::new();
        for r in 0..=rows {
            for c in 0..=cols {
                nodes.push(MeshNode {
                    pos: Point::new(
                        bounds.x0 + bounds.width() * c as f64 / cols as f64,
                        bounds.y0 + bounds.height() * r as f64 / rows as f64,
                    ),
                    color: base,
                    alpha: 1.0,
                });
            }
        }
        Mesh {
            rows,
            cols,
            nodes,
            smooth: false,
        }
    }

    pub fn node(&self, r: u32, c: u32) -> Option<&MeshNode> {
        self.nodes.get((r * (self.cols + 1) + c) as usize)
    }

    /// Index of the node nearest to `p` (local space).
    pub fn nearest(&self, p: Point) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .min_by(|a, b| {
                (a.1.pos - p)
                    .hypot()
                    .partial_cmp(&(b.1.pos - p).hypot())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    }

    /// Add a grid line through the node nearest to `p` (double-click).
    pub fn add_line(&mut self, p: Point) {
        let Some(i) = self.nearest(p) else { return };
        let cols = self.cols as usize + 1;
        let (r, c) = (i / cols, i % cols);
        // Insert a column after c (interpolating positions and colours),
        // or a row after r, whichever edge is nearer.
        let pos = self.nodes[i].pos;
        let add_col = (p.x - pos.x).abs() >= (p.y - pos.y).abs() && c + 1 < cols;
        if add_col {
            let mut new_nodes = Vec::new();
            for rr in 0..=self.rows as usize {
                for cc in 0..cols {
                    let n = self.nodes[rr * cols + cc];
                    new_nodes.push(n);
                    if cc == c {
                        let next = self.nodes[rr * cols + cc + 1];
                        new_nodes.push(MeshNode {
                            pos: n.pos.midpoint(next.pos),
                            color: lerp_color(n.color, next.color, 0.5),
                            alpha: (n.alpha + next.alpha) / 2.0,
                        });
                    }
                }
            }
            self.cols += 1;
            self.nodes = new_nodes;
        } else if r < self.rows as usize {
            let mut new_nodes = Vec::new();
            for rr in 0..=self.rows as usize {
                for cc in 0..cols {
                    new_nodes.push(self.nodes[rr * cols + cc]);
                }
                if rr == r && rr < self.rows as usize {
                    for cc in 0..cols {
                        let n = self.nodes[rr * cols + cc];
                        let next = self.nodes[(rr + 1) * cols + cc];
                        new_nodes.push(MeshNode {
                            pos: n.pos.midpoint(next.pos),
                            color: lerp_color(n.color, next.color, 0.5),
                            alpha: (n.alpha + next.alpha) / 2.0,
                        });
                    }
                }
            }
            self.rows += 1;
            self.nodes = new_nodes;
        }
    }
}

impl Fill {
    pub fn linear(from: Color, to: Color, angle: f64) -> Self {
        Fill::Fountain(Fountain::two(FountainKind::Linear, from, to, angle))
    }

    pub fn radial(from: Color, to: Color) -> Self {
        Fill::Fountain(Fountain::two(FountainKind::Radial, from, to, 0.0))
    }

    /// A representative colour (first stop, front colour) for swatches.
    pub fn preview_color(&self) -> Option<Color> {
        match self {
            Fill::None => None,
            Fill::Solid(c) => Some(*c),
            Fill::Fountain(f) => Some(f.first_color()),
            Fill::Pattern(Pattern::TwoColor { front, .. }) => Some(*front),
            Fill::Pattern(Pattern::Bitmap { .. }) => Some(Color::Gray { v: 0.5 }),
            Fill::Pattern(Pattern::Vector { shapes, .. }) => Some(
                shapes
                    .iter()
                    .find_map(|s| s.fill.preview_color())
                    .unwrap_or(Color::Gray { v: 0.5 }),
            ),
            Fill::Texture(t) => Some(t.color_a),
            Fill::Mesh(m) => m.nodes.first().map(|n| n.color),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// Arrowheads at the ends of open curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Arrowhead {
    #[default]
    None,
    Arrow,
    OpenArrow,
    Circle,
    Square,
    Bar,
    Diamond,
}

impl Arrowhead {
    pub const ALL: [Arrowhead; 7] = [
        Arrowhead::None,
        Arrowhead::Arrow,
        Arrowhead::OpenArrow,
        Arrowhead::Circle,
        Arrowhead::Square,
        Arrowhead::Bar,
        Arrowhead::Diamond,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Arrowhead::None => "None",
            Arrowhead::Arrow => "Arrow",
            Arrowhead::OpenArrow => "Open arrow",
            Arrowhead::Circle => "Circle",
            Arrowhead::Square => "Square",
            Arrowhead::Bar => "Bar",
            Arrowhead::Diamond => "Diamond",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub color: Color,
    /// Width in millimetres. the target design's "hairline" is 0.0762 mm (0.216 pt).
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    /// Dash pattern as alternating on/off lengths, in multiples of the width
    /// (the target design's convention). Empty = solid.
    pub dash: Vec<f64>,
    /// When true, the outline scales with the object (the target design's
    /// "Scale with object"). When false, width stays fixed under transforms.
    pub scale_with_object: bool,
    /// Draw the outline behind the fill.
    pub behind_fill: bool,
    #[serde(default)]
    pub start_arrow: Arrowhead,
    #[serde(default)]
    pub end_arrow: Arrowhead,
    /// Calligraphic nib stretch 0..1 and angle (1.0 = round nib).
    #[serde(default = "one")]
    pub stretch: f64,
    #[serde(default)]
    pub nib_angle: f64,
}

fn one() -> f64 {
    1.0
}

impl Stroke {
    pub const HAIRLINE: f64 = 0.0762;

    pub fn hairline(color: Color) -> Self {
        Stroke {
            color,
            width: Self::HAIRLINE,
            ..Default::default()
        }
    }

    pub fn new(color: Color, width: f64) -> Self {
        Stroke {
            color,
            width,
            ..Default::default()
        }
    }
}

impl Default for Stroke {
    fn default() -> Self {
        Stroke {
            color: Color::BLACK,
            width: Self::HAIRLINE,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash: Vec::new(),
            scale_with_object: false,
            behind_fill: false,
            start_arrow: Arrowhead::None,
            end_arrow: Arrowhead::None,
            stretch: 1.0,
            nib_angle: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fountain_interpolates_stops() {
        let f = Fountain {
            kind: FountainKind::Linear,
            stops: vec![
                Stop {
                    pos: 0.0,
                    color: Color::rgb8(0, 0, 0),
                },
                Stop {
                    pos: 0.5,
                    color: Color::rgb8(255, 0, 0),
                },
                Stop {
                    pos: 1.0,
                    color: Color::rgb8(255, 255, 255),
                },
            ],
            angle: 0.0,
            offset: Point::ZERO,
            edge_pad: 0.0,
        };
        assert_eq!(f.color_at(0.25).to_rgb8(), [128, 0, 0]);
        assert_eq!(f.color_at(0.5).to_rgb8(), [255, 0, 0]);
        assert_eq!(f.color_at(1.5).to_rgb8(), [255, 255, 255]);
    }

    #[test]
    fn pattern_tiles_have_both_colours() {
        for t in PatternTile::ALL {
            let mut front = 0;
            for i in 0..20 {
                for j in 0..20 {
                    if t.front(i as f64 / 20.0, j as f64 / 20.0) {
                        front += 1;
                    }
                }
            }
            assert!(front > 0 && front < 400, "{t:?}");
        }
    }

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> crate::document::Shape {
        let mut s = crate::document::Shape::new(
            crate::ShapeId(1),
            crate::document::ShapeKind::Rect {
                rect: Rect::new(x0, y0, x1, y1),
                radius: 0.0,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        s.stroke = None;
        s
    }

    #[test]
    fn vector_pattern_from_shapes_moves_bounds_to_origin() {
        let shapes = vec![
            square(30.0, 40.0, 35.0, 45.0),
            square(35.0, 45.0, 40.0, 50.0),
        ];
        let Pattern::Vector { shapes, tile } = Pattern::vector_from_shapes(&shapes) else {
            panic!("expected a vector pattern");
        };
        assert_eq!(tile, Size::new(10.0, 10.0));
        assert_eq!(shapes.len(), 2);
        let b = shapes[0].bounds();
        assert!((b.x0).abs() < 1e-9 && (b.y0).abs() < 1e-9, "{b:?}");
        assert!(
            (b.x1 - 5.0).abs() < 1e-9 && (b.y1 - 5.0).abs() < 1e-9,
            "{b:?}"
        );
        let b = shapes[1].bounds();
        assert!(
            (b.x1 - 10.0).abs() < 1e-9 && (b.y1 - 10.0).abs() < 1e-9,
            "{b:?}"
        );
    }

    #[test]
    fn vector_pattern_from_nothing_has_a_minimal_tile() {
        let Pattern::Vector { shapes, tile } = Pattern::vector_from_shapes(&[]) else {
            panic!("expected a vector pattern");
        };
        assert!(shapes.is_empty());
        assert!(tile.width > 0.0 && tile.height > 0.0);
    }

    #[test]
    fn vector_pattern_round_trips_through_json() {
        let fill = Fill::Pattern(Pattern::vector_from_shapes(&[square(0.0, 0.0, 5.0, 5.0)]));
        let json = serde_json::to_string(&fill).unwrap();
        assert!(json.contains("\"pattern\":\"vector\""));
        let back: Fill = serde_json::from_str(&json).unwrap();
        assert_eq!(back, fill);
    }

    #[test]
    fn older_pattern_fills_still_load() {
        let json = r#"{"fill":"pattern","pattern":"twocolor","tile":"dots","front":{"model":"gray","v":0.0},"back":{"model":"gray","v":1.0},"size_mm":5.0}"#;
        let back: Fill = serde_json::from_str(json).unwrap();
        assert!(matches!(
            back,
            Fill::Pattern(Pattern::TwoColor {
                tile: PatternTile::Dots,
                ..
            })
        ));
        // A vector tile without shapes is still a valid (empty) tile.
        let json = r#"{"fill":"pattern","pattern":"vector","tile":{"width":4.0,"height":2.0}}"#;
        let back: Fill = serde_json::from_str(json).unwrap();
        assert!(
            matches!(back, Fill::Pattern(Pattern::Vector { ref shapes, .. }) if shapes.is_empty())
        );
    }
}

/// Arrowhead outlines (page space) for an open path's ends, sized by the
/// outline width (hairlines use 0.25 mm). Filled with the outline colour.
pub fn arrowhead_paths(
    path: &crate::geometry::BezPath,
    stroke: &Stroke,
) -> Vec<crate::geometry::BezPath> {
    use crate::geometry::{BezPath, PathEl, Vec2};
    let w = if stroke.width <= Stroke::HAIRLINE + 1e-9 {
        0.25
    } else {
        stroke.width
    };
    let size = (w * 4.0).max(1.0);
    let mut pts: Vec<Point> = Vec::new();
    let mut closed = false;
    kurbo::flatten(path.elements().iter().copied(), 0.05, &mut |el| match el {
        PathEl::MoveTo(p) => {
            if pts.is_empty() {
                pts.push(p)
            }
        }
        PathEl::LineTo(p) => pts.push(p),
        PathEl::ClosePath => closed = true,
        _ => {}
    });
    if closed || pts.len() < 2 {
        return Vec::new();
    }
    let head = |tip: Point, dir: Vec2, kind: Arrowhead| -> Option<BezPath> {
        let dir = dir.normalize();
        if !dir.x.is_finite() {
            return None;
        }
        let n = Vec2::new(-dir.y, dir.x);
        let mut p = BezPath::new();
        match kind {
            Arrowhead::None => return None,
            Arrowhead::Arrow => {
                p.move_to(tip);
                p.line_to(tip - dir * size + n * (size * 0.4));
                p.line_to(tip - dir * size * 0.7);
                p.line_to(tip - dir * size - n * (size * 0.4));
                p.close_path();
            }
            Arrowhead::OpenArrow => {
                let t = w.max(0.2);
                p.move_to(tip);
                p.line_to(tip - dir * size + n * (size * 0.45));
                p.line_to(tip - dir * size + n * (size * 0.45) - dir * 0.0 + n * (-t) - dir * t);
                p.line_to(tip - dir * (t * 1.4));
                p.line_to(tip - dir * size - n * (size * 0.45) + n * t - dir * t);
                p.line_to(tip - dir * size - n * (size * 0.45));
                p.close_path();
            }
            Arrowhead::Circle => {
                let c = tip - dir * (size * 0.35);
                p = crate::geometry::Circle::new(c, size * 0.35).to_path(0.01);
            }
            Arrowhead::Square => {
                let c = tip - dir * (size * 0.35);
                let h = size * 0.35;
                p.move_to(c + dir * h + n * h);
                p.line_to(c - dir * h + n * h);
                p.line_to(c - dir * h - n * h);
                p.line_to(c + dir * h - n * h);
                p.close_path();
            }
            Arrowhead::Bar => {
                let t = w.max(0.2);
                p.move_to(tip + n * (size * 0.5));
                p.line_to(tip - dir * t + n * (size * 0.5));
                p.line_to(tip - dir * t - n * (size * 0.5));
                p.line_to(tip - n * (size * 0.5));
                p.close_path();
            }
            Arrowhead::Diamond => {
                let h = size * 0.5;
                p.move_to(tip);
                p.line_to(tip - dir * h + n * (h * 0.6));
                p.line_to(tip - dir * h * 2.0);
                p.line_to(tip - dir * h - n * (h * 0.6));
                p.close_path();
            }
        }
        Some(p)
    };
    use crate::geometry::Shape as _;
    let mut out = Vec::new();
    let n = pts.len();
    if let Some(p) = head(pts[0], pts[0] - pts[1.min(n - 1)], stroke.start_arrow) {
        out.push(p);
    }
    if let Some(p) = head(pts[n - 1], pts[n - 1] - pts[n - 2], stroke.end_arrow) {
        out.push(p);
    }
    out
}
