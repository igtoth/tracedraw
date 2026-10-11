//! Effects > Picture Mosaic: a mosaic of
//! pictures from an image library that recreates the selected objects. A
//! grid of cells takes the library picture closest in colour to each
//! cell; the reference can be blended over the tiles; the result is one
//! bitmap, a bitmap with the blend on top, or an array of tile bitmaps.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

use image::{imageops, Rgba, RgbaImage};
use tracedraw_core::document::{Shape, ShapeKind};
use tracedraw_core::geometry::Rect;
use tracedraw_core::{Command, Fill};

/// The most pixels along either side of a mosaic.
pub const MAX_SIDE: u32 = 15_000;
/// The most pictures a library indexes.
pub const MAX_LIBRARY: usize = 1000;
/// The most rows of tiles (five times as tall as wide at 300 columns).
pub const MAX_ROWS: u32 = 1500;
/// The most pixels in a mosaic (256 MB of colour).
pub const MAX_PIXELS: u64 = 64_000_000;

/// One library picture: where it is, its average colour, and (once
/// loaded) its pixels.
#[derive(Clone)]
pub struct LibImage {
    pub path: PathBuf,
    pub avg: [f32; 3],
    pub image: Option<Arc<RgbaImage>>,
}

impl std::fmt::Debug for LibImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LibImage({:?}, {:?})", self.path, self.avg)
    }
}

impl PartialEq for LibImage {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.avg == other.avg
    }
}

/// How the mosaic comes out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Composition {
    /// One bitmap with the blend in it.
    #[default]
    Single,
    /// The mosaic bitmap with the blend as a second bitmap on top.
    Stack,
    /// A group of tile bitmaps with the blend on top.
    Array,
}

impl Composition {
    pub const ALL: [Composition; 3] = [Composition::Single, Composition::Stack, Composition::Array];

    pub fn key(self) -> &'static str {
        match self {
            Composition::Single => "docker.picture_mosaic_single",
            Composition::Stack => "docker.picture_mosaic_stack",
            Composition::Array => "docker.picture_mosaic_array",
        }
    }
}

/// What happens to tiles the reference only partly covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Edges {
    /// Tiles stretched so the mosaic matches the reference exactly.
    #[default]
    Stretch,
    /// Square tiles; the incomplete row along the bottom is left out.
    Remove,
}

impl Edges {
    pub const ALL: [Edges; 2] = [Edges::Stretch, Edges::Remove];

    pub fn key(self) -> &'static str {
        match self {
            Edges::Stretch => "docker.picture_mosaic_stretch",
            Edges::Remove => "docker.picture_mosaic_remove",
        }
    }
}

/// How the output's pixel size is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Priority {
    #[default]
    DocumentDpi,
    CustomDpi,
    TileSize,
    OutputSize,
}

impl Priority {
    pub const ALL: [Priority; 4] = [
        Priority::DocumentDpi,
        Priority::CustomDpi,
        Priority::TileSize,
        Priority::OutputSize,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Priority::DocumentDpi => "docker.picture_mosaic_document_dpi",
            Priority::CustomDpi => "docker.picture_mosaic_custom_dpi",
            Priority::TileSize => "docker.picture_mosaic_tile_size",
            Priority::OutputSize => "docker.picture_mosaic_output_size",
        }
    }
}

/// The Picture Mosaic docker's settings and library.
#[derive(Debug, Clone, PartialEq)]
pub struct PictureMosaicSettings {
    pub library: Option<PathBuf>,
    pub images: Vec<LibImage>,
    pub keep_original: bool,
    /// Columns of tiles, 2 to 300; rows follow the reference's shape.
    pub columns: u32,
    /// How much of the reference shows over the tiles, 0 to 100.
    pub blending: f64,
    pub duplicates: bool,
    /// The fewest cells between two uses of one picture.
    pub spacing: u32,
    pub composition: Composition,
    pub edges: Edges,
    pub priority: Priority,
    pub dpi: f64,
    /// Tile width in pixels (Custom tile dimensions).
    pub tile_px: u32,
    /// Mosaic width in pixels (Custom output dimensions).
    pub output_px: u32,
}

impl Default for PictureMosaicSettings {
    fn default() -> Self {
        PictureMosaicSettings {
            library: None,
            images: Vec::new(),
            keep_original: true,
            columns: 30,
            blending: 20.0,
            duplicates: true,
            spacing: 2,
            composition: Composition::Single,
            edges: Edges::Stretch,
            priority: Priority::DocumentDpi,
            dpi: 300.0,
            tile_px: 64,
            output_px: 3000,
        }
    }
}

/// An image's average colour (premultiplied, flattened on white).
pub fn average(img: &RgbaImage) -> [f32; 3] {
    let mut acc = [0f64; 3];
    let mut n = 0f64;
    for p in img.pixels() {
        let a = p[3] as f64 / 255.0;
        for c in 0..3 {
            acc[c] += p[c] as f64 * a + 255.0 * (1.0 - a);
        }
        n += 1.0;
    }
    if n == 0.0 {
        return [255.0; 3];
    }
    acc.map(|v| (v / n) as f32)
}

/// The largest centred square of an image.
fn square(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let s = w.min(h).max(1);
    imageops::crop_imm(
        img,
        (w - s.min(w)) / 2,
        (h - s.min(h)) / 2,
        s.min(w),
        s.min(h),
    )
    .to_image()
}

/// The pictures of a library folder (not its subfolders) in name order,
/// at most [`MAX_LIBRARY`] of them.
pub fn library_paths(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = crate::files::list_dir(dir)
        .into_iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| {
                    matches!(
                        e.to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "tif" | "tiff"
                    )
                })
                .unwrap_or(false)
        })
        .collect();
    paths.sort();
    paths.truncate(MAX_LIBRARY);
    paths
}

/// One library picture's entry: the average colour of its centred
/// square. `None` when it cannot be read.
pub fn index_one(path: &Path) -> Option<LibImage> {
    let img = crate::files::open_image(path).ok()?.to_rgba8();
    let small = imageops::resize(&square(&img), 32, 32, imageops::FilterType::Triangle);
    Some(LibImage {
        path: path.to_path_buf(),
        avg: average(&small),
        image: None,
    })
}

/// A library being indexed in the background
/// shows a progress bar while it reads the pictures.
pub struct IndexJob {
    pub dir: PathBuf,
    pub total: usize,
    done: Arc<AtomicUsize>,
    rx: Receiver<Vec<LibImage>>,
}

impl IndexJob {
    /// Pictures read so far.
    pub fn done(&self) -> usize {
        self.done.load(Ordering::Relaxed).min(self.total)
    }

    /// The index once it is complete.
    pub fn finished(&self) -> Option<Vec<LibImage>> {
        self.rx.try_recv().ok()
    }
}

/// Start indexing a folder: on worker threads on the desktop, at once in
/// a browser (which has no folders to read).
pub fn start_index(dir: PathBuf) -> IndexJob {
    let paths = library_paths(&dir);
    let done = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = channel();
    let job = IndexJob {
        dir,
        total: paths.len(),
        done: done.clone(),
        rx,
    };
    if paths.is_empty() || crate::files::WEB {
        // Nothing to read (always so in a browser, which lists no folders).
        let _ = tx.send(Vec::new());
        return job;
    }
    std::thread::spawn(move || {
        let workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2)
            .clamp(1, 8);
        let chunk = paths.len().div_ceil(workers).max(1);
        let parts: Vec<Vec<LibImage>> = std::thread::scope(|scope| {
            let handles: Vec<_> = paths
                .chunks(chunk)
                .map(|part| {
                    let done = &done;
                    scope.spawn(move || {
                        part.iter()
                            .filter_map(|p| {
                                let entry = index_one(p);
                                done.fetch_add(1, Ordering::Relaxed);
                                entry
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap_or_default())
                .collect()
        });
        let _ = tx.send(parts.into_iter().flatten().collect());
    });
    job
}

/// The grid for a reference of `w` by `h` (any unit): columns, rows, and
/// the part of the reference the mosaic covers (as a fraction of its
/// height).
pub fn grid(columns: u32, w: f64, h: f64, edges: Edges) -> (u32, u32, f64) {
    let cols = columns.clamp(2, 300);
    let side = w / cols as f64;
    if side.is_nan() || side <= 0.0 || h.is_nan() || h <= 0.0 {
        return (cols, 1, 1.0);
    }
    match edges {
        Edges::Stretch => (cols, ((h / side).round() as u32).max(1), 1.0),
        Edges::Remove => {
            let rows = ((h / side).floor() as u32).max(1);
            (cols, rows, (rows as f64 * side / h).min(1.0))
        }
    }
}

/// The tile size in pixels for a reference `w` by `h` mm split into
/// `cols` by `rows` cells (from [`grid`]), following the output priority
/// and kept within [`MAX_SIDE`] and [`MAX_PIXELS`].
pub fn tile_pixels(
    s: &PictureMosaicSettings,
    w: f64,
    h: f64,
    cols: u32,
    rows: u32,
    document_dpi: f64,
) -> (u32, u32) {
    let (cols, rows) = (cols.max(1), rows.max(1));
    let mm_w = if w.is_finite() { w.max(0.1) } else { 0.1 };
    let mm_h = if h.is_finite() { h.max(0.1) } else { 0.1 };
    let width_px = match s.priority {
        Priority::DocumentDpi => mm_w / 25.4 * document_dpi.clamp(10.0, 2400.0),
        Priority::CustomDpi => mm_w / 25.4 * s.dpi.clamp(10.0, 2400.0),
        Priority::TileSize => s.tile_px.clamp(4, 2000) as f64 * cols as f64,
        Priority::OutputSize => s.output_px.clamp(16, MAX_SIDE) as f64,
    };
    let mut tw = (width_px / cols as f64)
        .round()
        .clamp(2.0, (MAX_SIDE / cols).max(2) as f64);
    let aspect = match s.edges {
        Edges::Remove => 1.0,
        // Cells as tall, relative to their width, as the reference's.
        Edges::Stretch => (mm_h / rows as f64) / (mm_w / cols as f64),
    };
    let mut th = (tw * aspect)
        .round()
        .clamp(2.0, (MAX_SIDE / rows).max(2) as f64);
    let pixels = tw * cols as f64 * th * rows as f64;
    if pixels > MAX_PIXELS as f64 {
        let k = (MAX_PIXELS as f64 / pixels).sqrt();
        tw = (tw * k).floor().max(2.0);
        th = (th * k).floor().max(2.0);
    }
    (tw as u32, th as u32)
}

/// The library picture for each cell (row by row): the nearest in colour
/// that the duplicate rule allows. Without duplicates a picture is used
/// once while unused ones remain.
pub fn assign(
    cells: &[[f32; 3]],
    cols: u32,
    lib: &[LibImage],
    duplicates: bool,
    spacing: u32,
) -> Vec<usize> {
    if lib.is_empty() {
        return Vec::new();
    }
    let cols = cols.max(1) as usize;
    let mut used: Vec<Vec<usize>> = vec![Vec::new(); lib.len()];
    let mut out = Vec::with_capacity(cells.len());
    let dist = |a: &[f32; 3], b: &[f32; 3]| -> f32 {
        // Weighted for the eye: green counts most.
        let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        2.0 * d[0] * d[0] + 4.0 * d[1] * d[1] + 3.0 * d[2] * d[2]
    };
    for (i, c) in cells.iter().enumerate() {
        let (r, col) = (i / cols, i % cols);
        let allowed = |k: usize| -> bool {
            if !duplicates {
                return used[k].is_empty();
            }
            // Uses come in cell order, so only the last few rows matter.
            for &j in used[k].iter().rev() {
                let (jr, jc) = (j / cols, j % cols);
                if r - jr > spacing as usize {
                    break;
                }
                if jc.abs_diff(col) <= spacing as usize {
                    return false;
                }
            }
            true
        };
        let best = (0..lib.len())
            .filter(|&k| allowed(k))
            .min_by(|&a, &b| dist(c, &lib[a].avg).total_cmp(&dist(c, &lib[b].avg)))
            // Nothing allowed (a small library): the nearest anyway.
            .or_else(|| {
                (0..lib.len())
                    .min_by(|&a, &b| dist(c, &lib[a].avg).total_cmp(&dist(c, &lib[b].avg)))
            })
            .unwrap_or(0);
        used[best].push(i);
        out.push(best);
    }
    out
}

/// The average colour of each cell of `img` split into `cols` by `rows`
/// over its top `cover` fraction.
pub fn cell_colors(img: &RgbaImage, cols: u32, rows: u32, cover: f64) -> Vec<[f32; 3]> {
    let (w, h) = (
        img.width() as f64,
        img.height() as f64 * cover.clamp(0.0, 1.0),
    );
    let mut out = Vec::with_capacity((cols * rows) as usize);
    for r in 0..rows {
        for c in 0..cols {
            let x0 = (c as f64 * w / cols as f64).floor() as u32;
            let x1 = (((c + 1) as f64 * w / cols as f64).ceil() as u32)
                .max(x0 + 1)
                .min(img.width());
            let y0 = (r as f64 * h / rows as f64).floor() as u32;
            let y1 = (((r + 1) as f64 * h / rows as f64).ceil() as u32)
                .max(y0 + 1)
                .min(img.height());
            if x0 >= x1 || y0 >= y1 {
                out.push([255.0; 3]);
                continue;
            }
            out.push(average(
                &imageops::crop_imm(img, x0, y0, x1 - x0, y1 - y0).to_image(),
            ));
        }
    }
    out
}

/// Each picked picture once: its centred square resized to `tw` by `th`
/// (a swatch of its average colour when it cannot be read any more).
pub fn tiles_for(lib: &[LibImage], picks: &[usize], tw: u32, th: u32) -> HashMap<usize, RgbaImage> {
    let (tw, th) = (tw.max(1), th.max(1));
    let mut out = HashMap::new();
    for &k in picks {
        if out.contains_key(&k) {
            continue;
        }
        let src = lib.get(k).and_then(|l| {
            l.image.clone().or_else(|| {
                crate::files::open_image(&l.path)
                    .ok()
                    .map(|d| Arc::new(d.to_rgba8()))
            })
        });
        let tile = match src {
            Some(img) => imageops::resize(&square(&img), tw, th, imageops::FilterType::Triangle),
            None => {
                let a = lib.get(k).map(|l| l.avg).unwrap_or([255.0; 3]);
                RgbaImage::from_pixel(tw, th, Rgba([a[0] as u8, a[1] as u8, a[2] as u8, 255]))
            }
        };
        out.insert(k, tile);
    }
    out
}

/// The mosaic picture: each cell of a `cols` by `rows` grid of `tw` by
/// `th` pixel tiles filled with its library picture, on white, and the
/// reference blended over it by `blend` (0 to 1) when given.
pub fn compose(
    lib: &[LibImage],
    picks: &[usize],
    cols: u32,
    rows: u32,
    tw: u32,
    th: u32,
    reference: Option<(&RgbaImage, f32)>,
) -> RgbaImage {
    let tiles = tiles_for(lib, picks, tw, th);
    let (w, h) = (cols * tw, rows * th);
    let mut out = RgbaImage::from_pixel(w.max(1), h.max(1), Rgba([255, 255, 255, 255]));
    for (i, k) in picks.iter().enumerate().take((cols * rows) as usize) {
        if let Some(tile) = tiles.get(k) {
            let (c, r) = (i as u32 % cols.max(1), i as u32 / cols.max(1));
            imageops::overlay(&mut out, tile, (c * tw) as i64, (r * th) as i64);
        }
    }
    if let Some((refimg, t)) = reference {
        if t > 0.0 {
            let ov = imageops::resize(refimg, w.max(1), h.max(1), imageops::FilterType::Triangle);
            for (p, q) in out.pixels_mut().zip(ov.pixels()) {
                let a = q[3] as f32 / 255.0 * t;
                for ch in 0..3 {
                    p[ch] = (p[ch] as f32 * (1.0 - a) + q[ch] as f32 * a).round() as u8;
                }
            }
        }
    }
    out
}

impl crate::app::App {
    /// Start indexing a library folder for the docker; the pictures arrive
    /// on a later frame (see [`App::poll_picture_mosaic_index`]).
    pub fn picture_mosaic_load_library(&mut self, dir: PathBuf) {
        self.picture_mosaic_job = Some(start_index(dir));
    }

    /// Collect a finished library index, once per frame. True while one is
    /// still being read (the caller keeps repainting).
    pub fn poll_picture_mosaic_index(&mut self) -> bool {
        let Some(job) = &self.picture_mosaic_job else {
            return false;
        };
        let Some(images) = job.finished() else {
            return true;
        };
        let dir = job.dir.clone();
        self.status = crate::i18n::trf(
            "status.picture_mosaic_indexed",
            &[("n", &images.len().to_string())],
        );
        self.picture_mosaic.images = images;
        self.picture_mosaic.library = Some(dir);
        self.picture_mosaic_job = None;
        false
    }

    /// Apply Picture Mosaic to the selection, in one undo step. False with
    /// a status message when nothing came out.
    pub fn apply_picture_mosaic(&mut self, s: &PictureMosaicSettings) -> bool {
        if s.images.is_empty() {
            self.status = crate::i18n::tr("status.picture_mosaic_no_library");
            return false;
        }
        let Some(bounds) = self.selection_bounds() else {
            return false;
        };
        let Some(layer) = self.active_layer() else {
            return false;
        };
        let (cols, rows, cover) = grid(s.columns, bounds.width(), bounds.height(), s.edges);
        if rows > MAX_ROWS {
            self.status = crate::i18n::tr("status.picture_mosaic_too_tall");
            return false;
        }
        let (tw, th) = tile_pixels(
            s,
            bounds.width(),
            bounds.height(),
            cols,
            rows,
            self.document_dpi(),
        );
        // The reference, rendered about 8 pixels a cell, for the cell
        // colours and the blend.
        let mm_w = bounds.width().max(0.1);
        let dpi = ((cols * 8) as f64 / (mm_w / 25.4)).clamp(36.0, 600.0);
        let Some((png, ..)) = self.render_selection_png_with(dpi, true, true) else {
            return false;
        };
        let Some(reference) = crate::bitmap_fx::decode(&png) else {
            return false;
        };
        let cells = cell_colors(&reference, cols, rows, cover);
        let picks = assign(&cells, cols, &s.images, s.duplicates, s.spacing);
        let blend = (s.blending.clamp(0.0, 100.0) / 100.0) as f32;
        let covered = Rect::new(
            bounds.x0,
            bounds.y1 - bounds.height() * cover,
            bounds.x1,
            bounds.y1,
        );
        // The part of the reference the mosaic covers, for the blend.
        let ref_cut = {
            let h =
                ((reference.height() as f64 * cover).round() as u32).clamp(1, reference.height());
            imageops::crop_imm(&reference, 0, 0, reference.width(), h).to_image()
        };
        let bitmap = |id, rect: Rect, w: u32, h: u32, png: Vec<u8>| -> Shape {
            let mut sh = Shape::new(
                id,
                ShapeKind::Bitmap {
                    rect,
                    width_px: w,
                    height_px: h,
                    png,
                    fx: None,
                },
            );
            sh.fill = Fill::None;
            sh.stroke = None;
            sh
        };
        let mut shapes: Vec<Shape> = Vec::new();
        match s.composition {
            Composition::Single => {
                let img = compose(
                    &s.images,
                    &picks,
                    cols,
                    rows,
                    tw,
                    th,
                    Some((&ref_cut, blend)),
                );
                if let Some(png) = crate::bitmap_fx::encode(&img) {
                    let id = self.engine.new_shape_id();
                    shapes.push(bitmap(id, covered, img.width(), img.height(), png));
                }
            }
            Composition::Stack => {
                let img = compose(&s.images, &picks, cols, rows, tw, th, None);
                if let Some(png) = crate::bitmap_fx::encode(&img) {
                    let id = self.engine.new_shape_id();
                    shapes.push(bitmap(id, covered, img.width(), img.height(), png));
                }
            }
            Composition::Array => {
                // One bitmap per cell; a picture used again shares its PNG.
                let tiles = tiles_for(&s.images, &picks, tw, th);
                let mut pngs: HashMap<usize, Vec<u8>> = HashMap::new();
                let (cw, ch) = (
                    covered.width() / cols as f64,
                    covered.height() / rows as f64,
                );
                for (i, &k) in picks.iter().enumerate() {
                    let Some(tile) = tiles.get(&k) else {
                        continue;
                    };
                    let png = match pngs.entry(k) {
                        std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
                        std::collections::hash_map::Entry::Vacant(e) => {
                            match crate::bitmap_fx::encode(tile) {
                                Some(png) => e.insert(png),
                                None => continue,
                            }
                        }
                    };
                    let (c, r) = (i as u32 % cols, i as u32 / cols);
                    let rect = Rect::new(
                        covered.x0 + c as f64 * cw,
                        covered.y1 - (r + 1) as f64 * ch,
                        covered.x0 + (c + 1) as f64 * cw,
                        covered.y1 - r as f64 * ch,
                    );
                    let id = self.engine.new_shape_id();
                    shapes.push(bitmap(id, rect, tile.width(), tile.height(), png.clone()));
                }
            }
        }
        // Stack and Array: the blend as a see-through copy of the reference.
        if s.composition != Composition::Single && blend > 0.0 && !shapes.is_empty() {
            if let Some(png) = crate::bitmap_fx::encode(&ref_cut) {
                let id = self.engine.new_shape_id();
                let mut over = bitmap(id, covered, ref_cut.width(), ref_cut.height(), png);
                over.opacity = blend as f64;
                shapes.push(over);
            }
        }
        if shapes.is_empty() {
            return false;
        }
        let new_id = if shapes.len() == 1 {
            shapes[0].id
        } else {
            let id = self.engine.new_shape_id();
            let group = Shape::new(id, ShapeKind::Group { children: shapes });
            shapes = vec![group];
            id
        };
        let mut cmds = Vec::new();
        if !s.keep_original {
            cmds.push(Command::DeleteShapes {
                shapes: self.selection.clone(),
            });
        }
        for shape in shapes {
            cmds.push(Command::AddShape { layer, shape });
        }
        if let Err(e) = self.engine.run_batch("Picture Mosaic", &cmds) {
            self.status = e.to_string();
            return false;
        }
        self.selection = vec![new_id];
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib() -> Vec<LibImage> {
        [
            [250u8, 20, 20],
            [20, 20, 250],
            [20, 200, 20],
            [240, 240, 240],
        ]
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let img = RgbaImage::from_pixel(10, 8, Rgba([c[0], c[1], c[2], 255]));
            LibImage {
                path: PathBuf::from(format!("/lib/{i}.png")),
                avg: average(&img),
                image: Some(Arc::new(img)),
            }
        })
        .collect()
    }

    #[test]
    fn grids_follow_the_reference_shape() {
        assert_eq!(grid(10, 100.0, 50.0, Edges::Stretch), (10, 5, 1.0));
        let (c, r, cover) = grid(10, 100.0, 55.0, Edges::Remove);
        assert_eq!((c, r), (10, 5));
        assert!((cover - 50.0 / 55.0).abs() < 1e-9);
        assert_eq!(grid(1, 100.0, 50.0, Edges::Stretch).0, 2);
        assert_eq!(grid(10, 0.0, 50.0, Edges::Stretch), (10, 1, 1.0));
    }

    #[test]
    fn cells_take_the_nearest_picture_and_respect_duplicates() {
        let l = lib();
        // A red and blue reference: left red, right blue.
        let reference = RgbaImage::from_fn(40, 20, |x, _| {
            if x < 20 {
                Rgba([240, 30, 30, 255])
            } else {
                Rgba([30, 30, 240, 255])
            }
        });
        let cells = cell_colors(&reference, 4, 2, 1.0);
        let picks = assign(&cells, 4, &l, true, 0);
        assert_eq!(picks, vec![0, 0, 1, 1, 0, 0, 1, 1]);
        // Spacing 1: the same picture is never a neighbour.
        let spaced = assign(&cells, 4, &l, true, 1);
        for (i, a) in spaced.iter().enumerate() {
            if i % 4 < 3 {
                assert_ne!(*a, spaced[i + 1], "{spaced:?}");
            }
        }
        // No duplicates: each picture once while others remain.
        let once = assign(&cells[..4], 4, &l, false, 0);
        let mut sorted = once.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 4);
        assert!(assign(&cells, 4, &[], true, 0).is_empty());
    }

    #[test]
    fn mosaics_tile_and_blend() {
        let l = lib();
        let img = compose(&l, &[0, 1, 2, 3], 2, 2, 5, 4, None);
        assert_eq!(img.dimensions(), (10, 8));
        assert_eq!(img.get_pixel(1, 1).0, [250, 20, 20, 255]);
        assert_eq!(img.get_pixel(7, 1).0, [20, 20, 250, 255]);
        assert_eq!(img.get_pixel(1, 6).0, [20, 200, 20, 255]);
        // Half blend of black: halfway to black.
        let black = RgbaImage::from_pixel(10, 8, Rgba([0, 0, 0, 255]));
        let blended = compose(&l, &[3, 3, 3, 3], 2, 2, 5, 4, Some((&black, 0.5)));
        assert_eq!(blended.get_pixel(3, 3).0, [120, 120, 120, 255]);
    }

    #[test]
    fn libraries_index_in_name_order_in_the_background() {
        let dir =
            std::env::temp_dir().join(format!("tracedraw-picture_mosaic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let save = |name: &str, c: [u8; 3]| {
            let img = RgbaImage::from_pixel(12, 6, Rgba([c[0], c[1], c[2], 255]));
            img.save(dir.join(name)).expect("save");
        };
        save("b.png", [0, 0, 255]);
        save("a.PNG", [255, 0, 0]);
        std::fs::write(dir.join("notes.txt"), "not a picture").expect("txt");
        std::fs::write(dir.join("broken.jpg"), [1u8, 2, 3]).expect("jpg");
        assert_eq!(library_paths(&dir).len(), 3);
        let job = start_index(dir.clone());
        assert_eq!(job.total, 3);
        let mut images = None;
        for _ in 0..500 {
            if let Some(found) = job.finished() {
                images = Some(found);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let images = images.expect("indexed in time");
        assert_eq!(job.done(), 3);
        // The unreadable file is left out; names in order.
        let names: Vec<String> = images
            .iter()
            .filter_map(|l| l.path.file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();
        assert_eq!(names, vec!["a.PNG", "b.png"]);
        assert_eq!(images[0].avg, [255.0, 0.0, 0.0]);
        assert_eq!(images[1].avg, [0.0, 0.0, 255.0]);
        // A missing folder indexes nothing.
        let none = start_index(dir.join("missing"));
        assert_eq!(none.total, 0);
        assert_eq!(none.finished(), Some(Vec::new()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn output_sizes_follow_the_priority_and_stay_bounded() {
        let mut s = PictureMosaicSettings::default();
        // 254 mm wide at 300 dpi: 3000 pixels over 30 columns.
        let (c, r, _) = grid(30, 254.0, 127.0, Edges::Stretch);
        assert_eq!((c, r), (30, 15));
        assert_eq!(tile_pixels(&s, 254.0, 127.0, c, r, 300.0), (100, 100));
        s.priority = Priority::CustomDpi;
        s.dpi = 150.0;
        assert_eq!(tile_pixels(&s, 254.0, 127.0, c, r, 300.0), (50, 50));
        s.priority = Priority::TileSize;
        s.tile_px = 40;
        assert_eq!(tile_pixels(&s, 254.0, 127.0, c, r, 300.0), (40, 40));
        s.priority = Priority::OutputSize;
        s.output_px = 600;
        assert_eq!(tile_pixels(&s, 254.0, 127.0, c, r, 300.0), (20, 20));
        // Stretched cells keep the reference's shape.
        s.output_px = 300;
        let (c, r, _) = grid(10, 100.0, 55.0, Edges::Stretch);
        let (tw, th) = tile_pixels(&s, 100.0, 55.0, c, r, 300.0);
        assert_eq!((c, r, tw), (10, 6, 30));
        assert_eq!(th, (30.0f64 * (55.0 / 6.0) / 10.0).round() as u32);
        // Huge requests stay within the limits; odd inputs do not panic.
        s.priority = Priority::CustomDpi;
        s.dpi = 2400.0;
        let (tw, th) = tile_pixels(&s, 5000.0, 5000.0, 300, 300, 300.0);
        assert!(tw * 300 <= MAX_SIDE && th * 300 <= MAX_SIDE);
        assert!((tw * 300) as u64 * (th * 300) as u64 <= MAX_PIXELS);
        let (tw, th) = tile_pixels(&s, 1.0, 1000.0, 300, 300_000, 300.0);
        assert!(tw >= 2 && th >= 2);
        let _ = tile_pixels(&s, f64::NAN, f64::INFINITY, 0, 0, f64::NAN);
    }

    #[test]
    fn applying_makes_one_step_in_each_composition() {
        let mut app = crate::app::App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 40.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(tracedraw_core::Color::rgb8(240, 30, 30)),
        });
        app.select(vec![id]);
        let mut s = PictureMosaicSettings {
            images: lib(),
            columns: 8,
            priority: Priority::TileSize,
            tile_px: 6,
            ..Default::default()
        };
        assert!(app.apply_picture_mosaic(&s));
        let made = app.selected_shapes()[0].clone();
        assert!(
            matches!(made.kind, ShapeKind::Bitmap { width_px: 48, .. }),
            "{:?}",
            made.kind
        );
        app.undo();
        app.select(vec![id]);
        // Stack: the mosaic and the blend at 20 % on top.
        s.composition = Composition::Stack;
        assert!(app.apply_picture_mosaic(&s));
        match &app.selected_shapes()[0].kind {
            ShapeKind::Group { children } => {
                assert_eq!(children.len(), 2);
                assert!((children[1].opacity - 0.2).abs() < 1e-6);
            }
            k => panic!("{k:?}"),
        }
        app.undo();
        app.select(vec![id]);
        s.composition = Composition::Array;
        s.keep_original = false;
        assert!(app.apply_picture_mosaic(&s));
        assert!(app.doc().find_shape(id).is_none());
        match &app.selected_shapes()[0].kind {
            // 8 x 4 tiles and the blend on top.
            ShapeKind::Group { children } => assert_eq!(children.len(), 33),
            k => panic!("{k:?}"),
        }
        // No library: nothing happens.
        app.undo();
        app.select(vec![id]);
        assert!(!app.apply_picture_mosaic(&PictureMosaicSettings::default()));
        // A reference far taller than wide is refused.
        let tall = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 1.0, 1000.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![tall]);
        s.columns = 300;
        let before = app.doc().clone();
        assert!(!app.apply_picture_mosaic(&s));
        assert_eq!(app.doc(), &before);
    }
}
