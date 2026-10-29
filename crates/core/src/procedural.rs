//! The tiles of the procedural fill ([`crate::style::Procedural`]).
//!
//! Each pattern is one repeating tile of filled paths, drawn here from
//! the fill's numbers: the renderer paints the tile once at the current
//! zoom and repeats it, the SVG and PDF writers put the same paths in a
//! pattern, and the Properties panel's preview draws them too, so the
//! fill looks the same everywhere. Tile space is millimetres with the
//! origin at the tile's bottom-left corner and Y up, as the page is. The
//! paths are filled with the non-zero rule in the fill's foreground
//! colour; the background, when the fill has one, covers the whole tile
//! beneath them. Lines are stroked into outlines here (round joins), so a
//! tile is nothing but filled regions.
//!
//! Every drawing is our own: the patterns are described by their geometry
//! below and come from no other program.

use crate::geometry::{BezPath, Circle, Point, Rect, Shape as _, Size, Vec2};
use crate::style::{Procedural, ProceduralParams, ProceduralPattern};
use std::sync::Arc;

/// One tile of a procedural fill: its size in mm and the paths filled
/// with the foreground colour.
#[derive(Debug, Clone, PartialEq)]
pub struct ProceduralTile {
    pub size: Size,
    pub paths: Vec<BezPath>,
}

/// The tolerance the curves are flattened and stroked to, in mm.
const TOL: f64 = 0.005;

/// The last tiles built, newest last.
const CACHE: usize = 16;

thread_local! {
    static TILES: std::cell::RefCell<Vec<(String, Arc<ProceduralTile>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// The tile of `p`, kept from the last time the same numbers were asked
/// for (the colours are not part of the tile).
pub fn procedural_tile(p: &Procedural) -> Arc<ProceduralTile> {
    let key = format!("{:?}|{:?}|{}", p.pattern, p.params, p.seed);
    if let Some(hit) = TILES.with(|c| {
        let mut c = c.borrow_mut();
        let i = c.iter().position(|(k, _)| *k == key)?;
        let hit = c.remove(i);
        let tile = hit.1.clone();
        c.push(hit);
        Some(tile)
    }) {
        return hit;
    }
    let tile = Arc::new(build_tile(p));
    TILES.with(|c| {
        let mut c = c.borrow_mut();
        c.push((key, tile.clone()));
        while c.len() > CACHE {
            c.remove(0);
        }
    });
    tile
}

/// The tile of `p`, built anew.
pub fn build_tile(p: &Procedural) -> ProceduralTile {
    let q = p.params.clamped();
    let (s, w) = (q.size_mm, q.line_width_mm);
    match p.pattern {
        ProceduralPattern::Octagons => octagons(s, w),
        ProceduralPattern::Triangles => triangles(s, w),
        ProceduralPattern::Bricks => bricks(s, w),
        ProceduralPattern::Honeycomb => honeycomb(s, w),
        ProceduralPattern::Crosshatch => crosshatch(s, w),
        ProceduralPattern::Dots => dots(s, q.share),
        ProceduralPattern::Waves => waves(s, w, q.amplitude),
        ProceduralPattern::Spirals => spirals(s, w, q.turns),
        ProceduralPattern::Speckle => speckle(&q, p.seed),
    }
}

/// The outline of `path` stroked `w` wide with round joins; `round_caps`
/// rounds the ends of open paths, else they are cut flat.
fn stroked(path: &BezPath, w: f64, round_caps: bool) -> BezPath {
    let caps = if round_caps {
        kurbo::Cap::Round
    } else {
        kurbo::Cap::Butt
    };
    let style = kurbo::Stroke::new(w)
        .with_join(kurbo::Join::Round)
        .with_caps(caps);
    kurbo::stroke(path.iter(), &style, &kurbo::StrokeOpts::default(), TOL)
}

fn polyline(points: &[Point], closed: bool) -> BezPath {
    let mut path = BezPath::new();
    for (i, p) in points.iter().enumerate() {
        if i == 0 {
            path.move_to(*p);
        } else {
            path.line_to(*p);
        }
    }
    if closed {
        path.close_path();
    }
    path
}

fn segment(a: Point, b: Point) -> BezPath {
    polyline(&[a, b], false)
}

/// A cubic through sample points with the derivatives `d` at each, the
/// parameter stepping `h` between them (Hermite segments).
fn hermite(points: &[(Point, Vec2)], h: f64) -> BezPath {
    let mut path = BezPath::new();
    for (i, (p, d)) in points.iter().enumerate() {
        if i == 0 {
            path.move_to(*p);
            continue;
        }
        let (p0, d0) = points[i - 1];
        path.curve_to(p0 + d0 * (h / 3.0), *p - *d * (h / 3.0), *p);
    }
    path
}

/// Regular octagons on a square grid of pitch `s`, one per tile with its
/// flat sides on the tile's edges; the corners left between four
/// octagons are squares, so the octagon's edges draw the whole tiling.
fn octagons(s: f64, w: f64) -> ProceduralTile {
    let a = s / (1.0 + std::f64::consts::SQRT_2);
    let (h, k) = (s / 2.0, a / 2.0);
    let pts = [
        Point::new(h - k, 0.0),
        Point::new(h + k, 0.0),
        Point::new(s, h - k),
        Point::new(s, h + k),
        Point::new(h + k, s),
        Point::new(h - k, s),
        Point::new(0.0, h + k),
        Point::new(0.0, h - k),
    ];
    ProceduralTile {
        size: Size::new(s, s),
        paths: vec![stroked(&polyline(&pts, true), w, false)],
    }
}

/// Equilateral triangles of side `s`: the tile is one row, `s` wide and
/// `s sqrt(3) / 2` high, with a horizontal line on each edge and the two
/// slanted families each as two segments through the row.
fn triangles(s: f64, w: f64) -> ProceduralTile {
    let h = s * 3f64.sqrt() / 2.0;
    let (o, m, r, t) = (0.0, s / 2.0, s, h);
    let lines = [
        segment(Point::new(o, o), Point::new(r, o)),
        segment(Point::new(o, t), Point::new(r, t)),
        segment(Point::new(o, o), Point::new(m, t)),
        segment(Point::new(m, o), Point::new(r, t)),
        segment(Point::new(r, o), Point::new(m, t)),
        segment(Point::new(m, o), Point::new(o, t)),
    ];
    ProceduralTile {
        size: Size::new(s, h),
        paths: lines.iter().map(|l| stroked(l, w, false)).collect(),
    }
}

/// Two rows of bricks `s` long and `s / 2` high, the upper row half a
/// brick along; the bricks are the filled paths, `w` of mortar between
/// them showing the background.
fn bricks(s: f64, w: f64) -> ProceduralTile {
    let (half, m) = (s / 2.0, w / 2.0);
    let rects = [
        Rect::new(m, m, s - m, half - m),
        Rect::new(0.0, half + m, half - m, s - m),
        Rect::new(half + m, half + m, s, s - m),
    ];
    ProceduralTile {
        size: Size::new(s, s),
        paths: rects
            .iter()
            .filter(|r| r.width() > 0.0 && r.height() > 0.0)
            .map(|r| r.to_path(TOL))
            .collect(),
    }
}

/// Regular hexagons `s` wide (flat side to flat side), point up, in rows
/// `1.5 R` apart (R the circumradius, `s / sqrt 3`), every other row half
/// a hexagon along; the tile holds two rows.
fn honeycomb(s: f64, w: f64) -> ProceduralTile {
    let r = s / 3f64.sqrt();
    let height = 3.0 * r;
    let hexagon = |c: Point| {
        let pts: Vec<Point> = (0..6)
            .map(|i| {
                let angle = (90.0 + 60.0 * i as f64).to_radians();
                c + Vec2::new(r * angle.cos(), r * angle.sin())
            })
            .collect();
        stroked(&polyline(&pts, true), w, false)
    };
    let centres = [
        Point::new(0.0, 0.0),
        Point::new(s, 0.0),
        Point::new(s / 2.0, 1.5 * r),
        Point::new(-s / 2.0, 1.5 * r),
        Point::new(1.5 * s, 1.5 * r),
        Point::new(0.0, height),
        Point::new(s, height),
    ];
    ProceduralTile {
        size: Size::new(s, height),
        paths: centres.iter().map(|c| hexagon(*c)).collect(),
    }
}

/// Lines at 45 and 135 degrees through the tile's corners, `s / sqrt 2`
/// apart.
fn crosshatch(s: f64, w: f64) -> ProceduralTile {
    let lines = [
        segment(Point::new(0.0, 0.0), Point::new(s, s)),
        segment(Point::new(0.0, s), Point::new(s, 0.0)),
    ];
    ProceduralTile {
        size: Size::new(s, s),
        paths: lines.iter().map(|l| stroked(l, w, false)).collect(),
    }
}

/// One dot in the middle of the tile covering `share` of its area:
/// `r = s sqrt(share / pi)`.
fn dots(s: f64, share: f64) -> ProceduralTile {
    let r = (s * (share / std::f64::consts::PI).sqrt()).min(s * 0.49);
    let paths = if r > 0.0 {
        vec![Circle::new(Point::new(s / 2.0, s / 2.0), r).to_path(TOL)]
    } else {
        Vec::new()
    };
    ProceduralTile {
        size: Size::new(s, s),
        paths,
    }
}

/// One period of `y = s / 2 + A sin(2 pi x / s)` across the tile, `A`
/// the amplitude share of `s`, as eight Hermite cubics from the exact
/// tangents, stroked `w` wide.
fn waves(s: f64, w: f64, amplitude: f64) -> ProceduralTile {
    let a = amplitude * s;
    let n = 8;
    let h = s / n as f64;
    let k = std::f64::consts::TAU / s;
    let pts: Vec<(Point, Vec2)> = (0..=n)
        .map(|i| {
            let x = i as f64 * h;
            (
                Point::new(x, s / 2.0 + a * (k * x).sin()),
                Vec2::new(1.0, a * k * (k * x).cos()),
            )
        })
        .collect();
    ProceduralTile {
        size: Size::new(s, s),
        paths: vec![stroked(&hermite(&pts, h), w, false)],
    }
}

/// An evenly wound spiral, `r = k theta`, from the tile's centre out to
/// `s / 2 - w` over `turns` turns, sixteen cubics a turn from the exact
/// tangents, stroked `w` wide with round ends.
fn spirals(s: f64, w: f64, turns: f64) -> ProceduralTile {
    let c = Point::new(s / 2.0, s / 2.0);
    let r_max = (s / 2.0 - w).max(w);
    let theta_max = std::f64::consts::TAU * turns;
    let k = r_max / theta_max;
    let n = ((16.0 * turns).ceil() as usize).max(4);
    let h = theta_max / n as f64;
    let pts: Vec<(Point, Vec2)> = (0..=n)
        .map(|i| {
            let t = i as f64 * h;
            let r = k * t;
            let (sin, cos) = t.sin_cos();
            (
                c + Vec2::new(r * cos, r * sin),
                Vec2::new(k * cos - r * sin, k * sin + r * cos),
            )
        })
        .collect();
    ProceduralTile {
        size: Size::new(s, s),
        paths: vec![stroked(&hermite(&pts, h), w, true)],
    }
}

/// `density` specks a tile, each a disc of radius `w / 4` to `3 w / 4`
/// placed by a pseudo-random sequence from `seed`, wholly inside the
/// tile so the tiling has no seams. The same seed always scatters the
/// same specks.
fn speckle(q: &ProceduralParams, seed: u32) -> ProceduralTile {
    let (s, w) = (q.size_mm, q.line_width_mm);
    let n = q.density.round().max(1.0) as usize;
    let mut rng = Rng::new(seed);
    let paths = (0..n)
        .map(|_| {
            let r = w / 2.0 * (0.5 + rng.next());
            let r = r.min(s / 2.0);
            let x = r + rng.next() * (s - 2.0 * r);
            let y = r + rng.next() * (s - 2.0 * r);
            Circle::new(Point::new(x, y), r).to_path(TOL)
        })
        .collect();
    ProceduralTile {
        size: Size::new(s, s),
        paths,
    }
}

/// A small pseudo-random sequence (a 64-bit mixing generator) that gives
/// the same numbers for the same seed on every platform.
struct Rng(u64);

impl Rng {
    fn new(seed: u32) -> Self {
        Rng((seed as u64) ^ 0x5851_F42D_4C95_7F2D)
    }

    /// The next number in 0..1.
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Color;

    fn fill(pattern: ProceduralPattern) -> Procedural {
        Procedural::new(pattern, Color::BLACK)
    }

    /// Every pattern yields paths inside a tile of the size its numbers
    /// say: `s` wide, `s` high but for the triangles and the honeycomb.
    #[test]
    fn every_pattern_yields_a_tile_of_its_size() {
        for pattern in ProceduralPattern::ALL {
            let mut p = fill(pattern);
            p.params.size_mm = 8.0;
            let tile = build_tile(&p);
            assert!(!tile.paths.is_empty(), "{pattern:?}");
            assert!(tile.paths.iter().all(|path| !path.elements().is_empty()));
            assert!((tile.size.width - 8.0).abs() < 1e-9, "{pattern:?}");
            let expect_h = match pattern {
                ProceduralPattern::Triangles => 8.0 * 3f64.sqrt() / 2.0,
                ProceduralPattern::Honeycomb => 8.0 * 3f64.sqrt(),
                _ => 8.0,
            };
            assert!((tile.size.height - expect_h).abs() < 1e-9, "{pattern:?}");
            // Nothing reaches far outside the tile (the honeycomb's
            // neighbours stick out by a hexagon; the rest by half a line).
            let margin = match pattern {
                ProceduralPattern::Honeycomb => 9.0,
                _ => 1.0,
            };
            for path in &tile.paths {
                let b = path.bounding_box();
                assert!(
                    b.x0 >= -margin
                        && b.y0 >= -margin
                        && b.x1 <= tile.size.width + margin
                        && b.y1 <= tile.size.height + margin,
                    "{pattern:?}: {b:?}"
                );
            }
        }
    }

    /// The speckle is the same for a seed and different for another one;
    /// the sequence does not depend on the colours.
    #[test]
    fn speckle_is_deterministic_for_a_seed() {
        let mut a = fill(ProceduralPattern::Speckle);
        a.seed = 7;
        let mut b = a.clone();
        b.foreground = Color::rgb8(200, 0, 0);
        assert_eq!(build_tile(&a), build_tile(&b));
        let mut c = a.clone();
        c.seed = 8;
        assert_ne!(build_tile(&a), build_tile(&c));
        assert_eq!(build_tile(&a).paths.len(), 40);
        // The cache gives the same tile back.
        assert_eq!(*procedural_tile(&a), build_tile(&a));
        assert_eq!(*procedural_tile(&a), build_tile(&a));
    }

    /// The numbers change the tile: the size, the line width (a wider
    /// line covers more), the dot's share, the wave's amplitude, the
    /// spiral's turns and the speckle's density.
    #[test]
    fn parameters_change_the_tile() {
        let area =
            |tile: &ProceduralTile| -> f64 { tile.paths.iter().map(|p| p.area().abs()).sum() };
        let mut p = fill(ProceduralPattern::Crosshatch);
        let thin = build_tile(&p);
        p.params.line_width_mm = 1.5;
        let thick = build_tile(&p);
        assert!(area(&thick) > area(&thin) * 2.0);
        p.params.size_mm = 20.0;
        let big = build_tile(&p);
        assert_eq!(big.size, Size::new(20.0, 20.0));
        assert!(area(&big) > area(&thick));

        let mut d = fill(ProceduralPattern::Dots);
        d.params.share = 0.5;
        let half = build_tile(&d);
        assert!((area(&half) - 50.0).abs() < 0.5, "{}", area(&half));
        d.params.share = 0.1;
        assert!((area(&build_tile(&d)) - 10.0).abs() < 0.5);

        let mut wv = fill(ProceduralPattern::Waves);
        let flat = build_tile(&wv);
        wv.params.amplitude = 0.4;
        let tall = build_tile(&wv);
        let height = |t: &ProceduralTile| t.paths[0].bounding_box().height();
        assert!(height(&tall) > height(&flat) + 3.0);

        let mut sp = fill(ProceduralPattern::Spirals);
        let three = build_tile(&sp);
        sp.params.turns = 6.0;
        let six = build_tile(&sp);
        assert!(area(&six) > area(&three) * 1.5);

        let mut sk = fill(ProceduralPattern::Speckle);
        sk.params.density = 10.0;
        assert_eq!(build_tile(&sk).paths.len(), 10);
    }

    /// Numbers a file may hold out of range are clamped, never a panic
    /// or an empty tile.
    #[test]
    fn wild_numbers_are_clamped() {
        for pattern in ProceduralPattern::ALL {
            let mut p = fill(pattern);
            p.params = ProceduralParams {
                size_mm: f64::NAN,
                line_width_mm: -4.0,
                share: 9.0,
                amplitude: f64::INFINITY,
                turns: 0.0,
                density: 1e12,
            };
            let tile = build_tile(&p);
            assert!(!tile.paths.is_empty(), "{pattern:?}");
            assert!(tile.size.width.is_finite() && tile.size.height.is_finite());
        }
        let q = ProceduralParams {
            size_mm: 4.0,
            line_width_mm: 10.0,
            ..ProceduralParams::default()
        }
        .clamped();
        assert_eq!(q.line_width_mm, 2.0);
    }

    /// The bricks tile: filled bricks with a mortar gap at half height.
    #[test]
    fn bricks_leave_a_mortar_gap() {
        let p = fill(ProceduralPattern::Bricks);
        let tile = build_tile(&p);
        assert_eq!(tile.paths.len(), 3);
        // No path covers the row at y = 5 (the mortar between the rows).
        for path in &tile.paths {
            let b = path.bounding_box();
            assert!(b.y1 <= 5.0 - 0.2 || b.y0 >= 5.0 + 0.2, "{b:?}");
        }
    }
}
