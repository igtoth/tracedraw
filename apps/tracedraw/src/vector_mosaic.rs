//! Effects > VectorMosaic: a vector mosaic of
//! tiles (circles, squares or a custom curve) on a screen of rows turned by
//! the screen angle, sampled from the selected objects. The tiles can be
//! uniform (flattened on white), sized by opacity or by darkness, limited
//! to a number of colours, merged into bigger tiles where neighbours match,
//! and welded into one curve per colour.

use tracedraw_core::document::{Shape, ShapeKind};
use tracedraw_core::geometry::{Affine, BezPath, Point, Rect, Shape as _};
use tracedraw_core::{Color, Command, Fill};

/// How tiles follow the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrackMethod {
    /// Uniform (white matte): tiles of one size, transparency flattened
    /// against white.
    #[default]
    Uniform,
    /// Size modulation 1 (opacity): more opaque, bigger.
    Opacity,
    /// Size modulation 2 (luminosity): darker, bigger.
    Luminosity,
}

impl TrackMethod {
    pub const ALL: [TrackMethod; 3] = [
        TrackMethod::Uniform,
        TrackMethod::Opacity,
        TrackMethod::Luminosity,
    ];

    pub fn key(self) -> &'static str {
        match self {
            TrackMethod::Uniform => "docker.vector_mosaic_uniform",
            TrackMethod::Opacity => "docker.vector_mosaic_opacity",
            TrackMethod::Luminosity => "docker.vector_mosaic_luminosity",
        }
    }
}

/// The tile shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileShape {
    #[default]
    Circle,
    Square,
    /// A closed curve picked from the drawing.
    Custom,
}

impl TileShape {
    pub const ALL: [TileShape; 3] = [TileShape::Circle, TileShape::Square, TileShape::Custom];

    pub fn key(self) -> &'static str {
        match self {
            TileShape::Circle => "docker.vector_mosaic_circle",
            TileShape::Square => "docker.vector_mosaic_square",
            TileShape::Custom => "docker.vector_mosaic_custom",
        }
    }
}

/// The VectorMosaic docker's settings.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorMosaicSettings {
    /// Tiles per inch along the rows and columns, 1 to 100.
    pub density: f64,
    /// Tile size factor, 0.1 to 5.
    pub scale: f64,
    /// Screen angle, degrees counter-clockwise, -90 to 90.
    pub angle: f64,
    pub keep_original: bool,
    pub limit_colors: bool,
    /// Most colours when limited, 2 to 256.
    pub colors: u32,
    pub method: TrackMethod,
    /// Merge adjacent: the most tiles of one colour combined into one.
    pub merge: u32,
    /// Weld adjacent overlap: one curve per colour.
    pub weld: bool,
    pub shape: TileShape,
    /// The custom tile, fitted to the unit square around the origin.
    pub custom: Option<BezPath>,
}

impl Default for VectorMosaicSettings {
    fn default() -> Self {
        VectorMosaicSettings {
            density: 10.0,
            scale: 1.0,
            angle: 0.0,
            keep_original: true,
            limit_colors: false,
            colors: 8,
            method: TrackMethod::Uniform,
            merge: 1,
            weld: false,
            shape: TileShape::Circle,
            custom: None,
        }
    }
}

/// One tile: its centre (mm, page space), size (mm) and colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tile {
    pub centre: Point,
    pub size: f64,
    pub color: [u8; 3],
    /// Row and column on the screen.
    pub cell: (i64, i64),
}

/// The most tiles one mosaic makes.
pub const MAX_TILES: usize = 60_000;

/// A shape's outline fitted to the unit square centred on the origin (a
/// custom tile).
pub fn unit_tile(path: &BezPath) -> Option<BezPath> {
    let b = path.bounding_box();
    let side = b.width().max(b.height());
    if side.is_nan() || side <= 1e-9 || path.elements().is_empty() {
        return None;
    }
    let c = b.center();
    Some(Affine::scale(1.0 / side) * Affine::translate((-c.x, -c.y)) * path.clone())
}

/// The tiles for an image of the selection: `img` covers `bounds`
/// (page mm, y up) with `px_per_mm` pixels per millimetre.
pub fn tiles(
    img: &image::RgbaImage,
    bounds: Rect,
    px_per_mm: f64,
    s: &VectorMosaicSettings,
) -> Result<Vec<Tile>, usize> {
    let density = if s.density.is_finite() {
        s.density.clamp(1.0, 100.0)
    } else {
        10.0
    };
    let pitch = 25.4 / density;
    let scale = if s.scale.is_finite() {
        s.scale.clamp(0.1, 5.0)
    } else {
        1.0
    };
    let a = (if s.angle.is_finite() {
        s.angle.clamp(-90.0, 90.0)
    } else {
        0.0
    })
    .to_radians();
    let (sin, cos) = a.sin_cos();
    let centre = bounds.center();
    let half = (bounds.width().hypot(bounds.height()) / 2.0 / pitch).ceil() as i64 + 1;
    let estimate = ((bounds.width() / pitch + 2.0) * (bounds.height() / pitch + 2.0)) as usize;
    if estimate > MAX_TILES {
        return Err(estimate);
    }
    let (w, h) = (img.width() as f64, img.height() as f64);
    // Average a 3 x 3 grid of samples over the tile's cell.
    let sample = |p: Point| -> ([f64; 3], f64) {
        let mut acc = [0f64; 4];
        for dj in [-1.0, 0.0, 1.0] {
            for di in [-1.0, 0.0, 1.0] {
                let q = Point::new(p.x + di * pitch / 3.0, p.y + dj * pitch / 3.0);
                let x = ((q.x - bounds.x0) * px_per_mm).floor();
                let y = ((bounds.y1 - q.y) * px_per_mm).floor();
                if x < 0.0 || y < 0.0 || x >= w || y >= h {
                    continue;
                }
                let px = img.get_pixel(x as u32, y as u32);
                let al = px[3] as f64 / 255.0;
                for c in 0..3 {
                    acc[c] += px[c] as f64 * al;
                }
                acc[3] += al;
            }
        }
        let alpha = acc[3] / 9.0;
        if acc[3] <= 1e-9 {
            return ([255.0; 3], 0.0);
        }
        ([acc[0] / acc[3], acc[1] / acc[3], acc[2] / acc[3]], alpha)
    };
    let mut out = Vec::new();
    for j in -half..=half {
        for i in -half..=half {
            let (u, v) = (i as f64 * pitch, j as f64 * pitch);
            let p = Point::new(centre.x + u * cos - v * sin, centre.y + u * sin + v * cos);
            if p.x < bounds.x0 || p.x > bounds.x1 || p.y < bounds.y0 || p.y > bounds.y1 {
                continue;
            }
            let (c, alpha) = sample(p);
            // Flattened against white.
            let flat: [f64; 3] = std::array::from_fn(|k| c[k] * alpha + 255.0 * (1.0 - alpha));
            let factor = match s.method {
                TrackMethod::Uniform => 1.0,
                TrackMethod::Opacity => alpha.sqrt(),
                TrackMethod::Luminosity => {
                    let l = 0.299 * flat[0] + 0.587 * flat[1] + 0.114 * flat[2];
                    (1.0 - l / 255.0).max(0.0).sqrt()
                }
            };
            if factor < 0.05 {
                continue;
            }
            let color = match s.method {
                TrackMethod::Opacity => c,
                _ => flat,
            };
            out.push(Tile {
                centre: p,
                size: pitch * scale * factor,
                color: color.map(|v| v.round().clamp(0.0, 255.0) as u8),
                cell: (i, j),
            });
            if out.len() > MAX_TILES {
                return Err(out.len());
            }
        }
    }
    if s.limit_colors {
        limit_colors(&mut out, s.colors.clamp(2, 256));
    }
    if s.merge >= 4 {
        out = merge(out, s.merge);
    }
    Ok(out)
}

/// Reduce the tiles' colours to at most `n` with a median-cut palette.
fn limit_colors(tiles: &mut [Tile], n: u32) {
    if tiles.is_empty() {
        return;
    }
    let img = image::RgbaImage::from_fn(tiles.len() as u32, 1, |x, _| {
        let c = tiles[x as usize].color;
        image::Rgba([c[0], c[1], c[2], 255])
    });
    let settings = crate::bitmap_modes::PalettedSettings {
        palette: crate::bitmap_modes::PaletteType::Optimized,
        colors: n,
        ..Default::default()
    };
    let palette = crate::bitmap_modes::build_palette(&img, &settings);
    if palette.is_empty() {
        return;
    }
    for t in tiles.iter_mut() {
        let d = |p: &[u8; 3]| -> i32 {
            (0..3)
                .map(|k| (p[k] as i32 - t.color[k] as i32).pow(2))
                .sum()
        };
        if let Some(best) = palette.iter().min_by_key(|p| d(p)) {
            t.color = *best;
        }
    }
}

/// Merge adjacent: blocks of k x k tiles of one colour and size (k up to
/// the square root of `most`) become one tile k times as big.
fn merge(tiles: Vec<Tile>, most: u32) -> Vec<Tile> {
    use std::collections::HashMap;
    let k_max = ((most as f64).sqrt().floor() as i64).clamp(1, 6);
    let index: HashMap<(i64, i64), usize> =
        tiles.iter().enumerate().map(|(i, t)| (t.cell, i)).collect();
    let mut used = vec![false; tiles.len()];
    let mut out = Vec::new();
    for k in (2..=k_max).rev() {
        for i in 0..tiles.len() {
            if used[i] {
                continue;
            }
            let (ci, cj) = tiles[i].cell;
            let mut block = Vec::new();
            'cells: for dj in 0..k {
                for di in 0..k {
                    match index.get(&(ci + di, cj + dj)) {
                        Some(&t)
                            if !used[t]
                                && tiles[t].color == tiles[i].color
                                && (tiles[t].size - tiles[i].size).abs() < 1e-9 =>
                        {
                            block.push(t)
                        }
                        _ => break 'cells,
                    }
                }
            }
            if block.len() == (k * k) as usize {
                let n = block.len() as f64;
                let (sx, sy) = block.iter().fold((0.0, 0.0), |acc, &t| {
                    (acc.0 + tiles[t].centre.x, acc.1 + tiles[t].centre.y)
                });
                for &t in &block {
                    used[t] = true;
                }
                out.push(Tile {
                    centre: Point::new(sx / n, sy / n),
                    size: tiles[i].size * k as f64,
                    color: tiles[i].color,
                    cell: tiles[i].cell,
                });
            }
        }
    }
    out.extend(
        tiles
            .iter()
            .enumerate()
            .filter(|(i, _)| !used[*i])
            .map(|(_, t)| *t),
    );
    out
}

/// A tile's outline in page space.
fn tile_path(t: &Tile, s: &VectorMosaicSettings) -> BezPath {
    let half = t.size / 2.0;
    match (s.shape, &s.custom) {
        (TileShape::Square, _) => {
            let r = Rect::new(
                t.centre.x - half,
                t.centre.y - half,
                t.centre.x + half,
                t.centre.y + half,
            );
            Affine::rotate_about(s.angle.to_radians(), t.centre) * r.to_path(0.01)
        }
        (TileShape::Custom, Some(unit)) => {
            Affine::translate(t.centre.to_vec2())
                * Affine::rotate(s.angle.to_radians())
                * Affine::scale(t.size)
                * unit.clone()
        }
        _ => tracedraw_core::geometry::Circle::new(t.centre, half).to_path(0.01),
    }
}

/// The mosaic's objects: one per tile, or one curve per colour when
/// welding.
pub fn mosaic_shapes(
    tiles: &[Tile],
    s: &VectorMosaicSettings,
    mut next_id: impl FnMut() -> tracedraw_core::ShapeId,
) -> Vec<Shape> {
    let make = |id, kind: ShapeKind, color: [u8; 3]| {
        let mut sh = Shape::new(id, kind);
        sh.fill = Fill::Solid(Color::rgb8(color[0], color[1], color[2]));
        sh.stroke = None;
        sh
    };
    if s.weld {
        let mut by_color: Vec<([u8; 3], BezPath)> = Vec::new();
        for t in tiles {
            let p = tile_path(t, s);
            match by_color.iter_mut().find(|(c, _)| *c == t.color) {
                Some((_, path)) => path.extend(p),
                None => by_color.push((t.color, p)),
            }
        }
        return by_color
            .into_iter()
            .map(|(c, path)| make(next_id(), ShapeKind::Path { path, closed: true }, c))
            .collect();
    }
    tiles
        .iter()
        .map(|t| {
            let half = t.size / 2.0;
            let r = Rect::new(
                t.centre.x - half,
                t.centre.y - half,
                t.centre.x + half,
                t.centre.y + half,
            );
            match s.shape {
                TileShape::Circle => make(
                    next_id(),
                    ShapeKind::Ellipse { rect: r, arc: None },
                    t.color,
                ),
                _ => make(
                    next_id(),
                    ShapeKind::Path {
                        path: tile_path(t, s),
                        closed: true,
                    },
                    t.color,
                ),
            }
        })
        .collect()
}

impl crate::app::App {
    /// Apply the VectorMosaic to the selection: the mosaic, grouped, on
    /// top of it (the selection removed unless Keep original), in one
    /// undo step. False with a status message when nothing came out.
    pub fn apply_vector_mosaic(&mut self, s: &VectorMosaicSettings) -> bool {
        let Some(bounds) = self.selection_bounds() else {
            return false;
        };
        let Some(layer) = self.active_layer() else {
            return false;
        };
        let dpi = (s.density.clamp(1.0, 100.0) * 6.0).clamp(36.0, 600.0);
        let Some((png, _, _, rendered)) = self.render_selection_png_with(dpi, true, true) else {
            return false;
        };
        let Some(img) = crate::bitmap_fx::decode(&png) else {
            return false;
        };
        let px_per_mm = img.width() as f64 / rendered.width().max(1e-9);
        let tiles = match tiles(&img, bounds, px_per_mm, s) {
            Ok(t) => t,
            Err(n) => {
                self.status =
                    crate::i18n::trf("status.vector_mosaic_too_many", &[("n", &n.to_string())]);
                return false;
            }
        };
        if tiles.is_empty() {
            self.status = crate::i18n::tr("status.vector_mosaic_empty");
            return false;
        }
        let engine = &mut self.engine;
        let children = mosaic_shapes(&tiles, s, || engine.new_shape_id());
        let group_id = self.engine.new_shape_id();
        let group = Shape::new(group_id, ShapeKind::Group { children });
        let mut cmds = Vec::new();
        if !s.keep_original {
            cmds.push(Command::DeleteShapes {
                shapes: self.selection.clone(),
            });
        }
        cmds.push(Command::AddShape {
            layer,
            shape: group,
        });
        if let Err(e) = self.engine.run_batch("VectorMosaic", &cmds) {
            self.status = e.to_string();
            return false;
        }
        self.selection = vec![group_id];
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    /// A 40 x 20 mm source at 4 px/mm: red left half, transparent right.
    fn source() -> (image::RgbaImage, Rect) {
        let img = ImageBuffer::from_fn(160, 80, |x, _| {
            if x < 80 {
                Rgba([220, 20, 20, 255])
            } else {
                Rgba([0, 0, 0, 0])
            }
        });
        (img, Rect::new(0.0, 0.0, 40.0, 20.0))
    }

    #[test]
    fn uniform_tiles_cover_the_bounds_flattened_on_white() {
        let (img, b) = source();
        let s = VectorMosaicSettings::default();
        let t = tiles(&img, b, 4.0, &s).expect("tiles");
        // 2.54 mm pitch over 40 x 20 mm: about 16 x 8 tiles.
        assert!((100..=180).contains(&t.len()), "{}", t.len());
        assert!(t.iter().all(|t| (t.size - 2.54).abs() < 1e-9));
        assert!(t.iter().any(|t| t.color == [255, 255, 255]));
        assert!(t.iter().any(|t| t.color[0] > 200 && t.color[1] < 40));
    }

    #[test]
    fn opacity_and_luminosity_size_the_tiles() {
        let (img, b) = source();
        let s = VectorMosaicSettings {
            method: TrackMethod::Opacity,
            ..Default::default()
        };
        let t = tiles(&img, b, 4.0, &s).expect("tiles");
        // Transparent cells give no tile.
        assert!(
            t.iter().all(|t| t.centre.x < 21.5),
            "{:?}",
            t.iter().map(|t| t.centre.x).fold(0.0, f64::max)
        );
        let gray = ImageBuffer::from_fn(80, 80, |x, _| {
            let v = (x * 3) as u8;
            Rgba([v, v, v, 255])
        });
        let s = VectorMosaicSettings {
            method: TrackMethod::Luminosity,
            ..Default::default()
        };
        let t = tiles(&gray, Rect::new(0.0, 0.0, 20.0, 20.0), 4.0, &s).expect("tiles");
        let left = t
            .iter()
            .filter(|t| t.centre.x < 5.0)
            .map(|t| t.size)
            .fold(0.0, f64::max);
        let right = t
            .iter()
            .filter(|t| t.centre.x > 15.0)
            .map(|t| t.size)
            .fold(0.0, f64::max);
        assert!(left > right, "{left} {right}");
    }

    #[test]
    fn limits_merges_screens_and_welds() {
        let (img, b) = source();
        let limited = tiles(
            &img,
            b,
            4.0,
            &VectorMosaicSettings {
                limit_colors: true,
                colors: 2,
                ..Default::default()
            },
        )
        .expect("tiles");
        let mut colors: Vec<[u8; 3]> = limited.iter().map(|t| t.color).collect();
        colors.sort();
        colors.dedup();
        assert!(colors.len() <= 2);
        let plain = tiles(&img, b, 4.0, &VectorMosaicSettings::default()).expect("tiles");
        let merged = tiles(
            &img,
            b,
            4.0,
            &VectorMosaicSettings {
                merge: 4,
                ..Default::default()
            },
        )
        .expect("tiles");
        assert!(merged.len() < plain.len());
        assert!(merged.iter().any(|t| (t.size - 5.08).abs() < 1e-9));
        // A turned screen still stays inside the bounds.
        let turned = tiles(
            &img,
            b,
            4.0,
            &VectorMosaicSettings {
                angle: 30.0,
                ..Default::default()
            },
        )
        .expect("tiles");
        assert!(turned.iter().all(|t| b.contains(t.centre)));
        // Welding makes one curve per colour.
        let mut n = 0;
        let shapes = mosaic_shapes(
            &plain,
            &VectorMosaicSettings {
                weld: true,
                ..Default::default()
            },
            || {
                n += 1;
                tracedraw_core::ShapeId(n)
            },
        );
        let distinct: std::collections::HashSet<[u8; 3]> = plain.iter().map(|t| t.color).collect();
        assert_eq!(shapes.len(), distinct.len());
        // Too dense for the area: refused, not a freeze.
        assert!(tiles(
            &img,
            Rect::new(0.0, 0.0, 2000.0, 2000.0),
            0.1,
            &VectorMosaicSettings {
                density: 100.0,
                ..Default::default()
            }
        )
        .is_err());
        assert!(unit_tile(&BezPath::new()).is_none());
    }

    #[test]
    fn applying_groups_the_mosaic_in_one_step() {
        let mut app = crate::app::App::headless();
        let id = app
            .new_shape(ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                arc: None,
            })
            .expect("ellipse");
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(Color::rgb8(30, 90, 200)),
        });
        app.select(vec![id]);
        let depth = app.engine.history_labels().0.len();
        assert!(app.apply_vector_mosaic(&VectorMosaicSettings {
            keep_original: false,
            method: TrackMethod::Opacity,
            ..Default::default()
        }));
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        assert!(app.doc().find_shape(id).is_none());
        let g = app.selected_shapes().into_iter().next().expect("group");
        match g.kind {
            ShapeKind::Group { children } => assert!(children.len() > 20, "{}", children.len()),
            _ => panic!("not a group"),
        }
        app.undo();
        assert!(app.doc().find_shape(id).is_some());
    }
}
