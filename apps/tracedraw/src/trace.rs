//! Bitmap to vector tracing: outline mode (colour regions become filled
//! curves) and centreline mode (strokes become open curves). See
//! docs/behavior/bitmap-tracing.md for the algorithm and defaults.

use image::{Rgba, RgbaImage};
use tracedraw_core::{
    geometry::{BezPath, Point, Rect},
    Color,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    // Centreline
    Technical,
    LineDrawing,
    // Outline
    LineArt,
    Logo,
    DetailedLogo,
    Clipart,
    LowQualityImage,
    HighQualityImage,
}

impl Preset {
    pub fn is_centerline(self) -> bool {
        matches!(self, Preset::Technical | Preset::LineDrawing)
    }
    pub fn key(self) -> &'static str {
        match self {
            Preset::Technical => "menu.bitmaps.trace_technical",
            Preset::LineDrawing => "menu.bitmaps.trace_line_drawing",
            Preset::LineArt => "menu.bitmaps.trace_line_art",
            Preset::Logo => "menu.bitmaps.trace_logo",
            Preset::DetailedLogo => "menu.bitmaps.trace_detailed_logo",
            Preset::Clipart => "menu.bitmaps.trace_clipart",
            Preset::LowQualityImage => "menu.bitmaps.trace_low_quality",
            Preset::HighQualityImage => "menu.bitmaps.trace_high_quality",
        }
    }
    pub fn settings(self) -> Settings {
        let (detail, smoothing, colors, bw) = match self {
            Preset::Technical => (60.0, 30.0, 2, true),
            Preset::LineDrawing => (50.0, 50.0, 2, true),
            Preset::LineArt => (40.0, 25.0, 2, true),
            Preset::Logo => (50.0, 25.0, 8, false),
            Preset::DetailedLogo => (75.0, 20.0, 24, false),
            Preset::Clipart => (60.0, 25.0, 32, false),
            Preset::LowQualityImage => (35.0, 40.0, 48, false),
            Preset::HighQualityImage => (80.0, 15.0, 96, false),
        };
        Settings {
            detail,
            smoothing,
            corner_smoothness: 0.0,
            colors,
            black_white: bw,
            remove_background: true,
            merge_adjacent: false,
            centerline: self.is_centerline(),
            delete_original: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub detail: f64,
    pub smoothing: f64,
    pub corner_smoothness: f64,
    pub colors: u32,
    pub black_white: bool,
    pub remove_background: bool,
    pub merge_adjacent: bool,
    pub centerline: bool,
    pub delete_original: bool,
}

/// One traced region or stroke.
pub struct Traced {
    pub path: BezPath,
    pub color: Color,
    pub closed: bool,
}

/// Trace `img` into shapes whose coordinates are in pixel space (y down).
/// The caller maps them into the bitmap's rectangle.
pub fn trace(img: &RgbaImage, s: &Settings) -> Vec<Traced> {
    if s.centerline {
        return centerline(img, s);
    }
    outline(img, s)
}

// ----- outline mode -----------------------------------------------------------

fn outline(img: &RgbaImage, s: &Settings) -> Vec<Traced> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w == 0 || h == 0 {
        return Vec::new();
    }
    // 1. Smooth a little when detail is low, then quantise.
    let src = if s.detail < 50.0 {
        image::imageops::blur(img, (50.0 - s.detail as f32) / 25.0)
    } else {
        img.clone()
    };
    let palette: Vec<[u8; 3]> = if s.black_white {
        vec![[0, 0, 0], [255, 255, 255]]
    } else {
        median_cut(&src, s.colors.clamp(2, 256) as usize)
    };
    let mut labels = vec![0usize; w * h];
    for (x, y, p) in src.enumerate_pixels() {
        let idx = if p[3] < 128 {
            usize::MAX
        } else if s.black_white {
            if luma(*p) < 128.0 {
                0
            } else {
                1
            }
        } else {
            nearest(&palette, [p[0], p[1], p[2]])
        };
        labels[y as usize * w + x as usize] = idx;
    }
    // 2. Background: the most common colour touching the border.
    let mut border_counts = vec![0usize; palette.len()];
    for x in 0..w {
        for y in [0, h - 1] {
            let l = labels[y * w + x];
            if l != usize::MAX {
                border_counts[l] += 1;
            }
        }
    }
    for y in 0..h {
        for x in [0, w - 1] {
            let l = labels[y * w + x];
            if l != usize::MAX {
                border_counts[l] += 1;
            }
        }
    }
    let background = if s.remove_background {
        border_counts
            .iter()
            .enumerate()
            .max_by_key(|(_, c)| **c)
            .map(|(i, _)| i)
    } else {
        None
    };
    // 3. Per colour, darkest first (so lighter regions stack on top),
    // extract the boundary contours of the mask.
    let mut order: Vec<usize> = (0..palette.len()).collect();
    order.sort_by_key(|i| {
        let c = palette[*i];
        (c[0] as u32 * 299 + c[1] as u32 * 587 + c[2] as u32 * 114) / 1000
    });
    let tolerance = 0.3 + (100.0 - s.detail) / 100.0 * 2.5;
    let mut out = Vec::new();
    for ci in order {
        if Some(ci) == background {
            continue;
        }
        let mask: Vec<bool> = labels.iter().map(|l| *l == ci).collect();
        let contours = contours(&mask, w, h);
        let mut path = BezPath::new();
        for ring in contours {
            if ring.len() < 3 {
                continue;
            }
            let simplified = tracedraw_core::geometry::simplify(&ring, tolerance);
            if simplified.len() < 3 {
                continue;
            }
            let sub = if s.smoothing > 0.0 {
                tracedraw_core::geometry::smooth_path(&simplified, true)
            } else {
                polyline_closed(&simplified)
            };
            path.extend(sub);
        }
        if path.elements().is_empty() {
            continue;
        }
        let c = palette[ci];
        out.push(Traced {
            path,
            color: Color::rgb8(c[0], c[1], c[2]),
            closed: true,
        });
    }
    out
}

fn polyline_closed(pts: &[Point]) -> BezPath {
    let mut p = BezPath::new();
    if let Some(f) = pts.first() {
        p.move_to(*f);
        for q in &pts[1..] {
            p.line_to(*q);
        }
        p.close_path();
    }
    p
}

fn luma(p: Rgba<u8>) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

fn nearest(palette: &[[u8; 3]], c: [u8; 3]) -> usize {
    let mut best = (u32::MAX, 0);
    for (i, p) in palette.iter().enumerate() {
        let d = (0..3)
            .map(|k| (p[k] as i32 - c[k] as i32).pow(2) as u32)
            .sum::<u32>();
        if d < best.0 {
            best = (d, i);
        }
    }
    best.1
}

/// Median cut colour quantisation to `n` colours.
pub fn median_cut(img: &RgbaImage, n: usize) -> Vec<[u8; 3]> {
    let mut pixels: Vec<[u8; 3]> = img
        .pixels()
        .filter(|p| p[3] >= 128)
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    if pixels.is_empty() {
        return vec![[0, 0, 0]];
    }
    // Subsample large images.
    if pixels.len() > 200_000 {
        let step = pixels.len() / 200_000 + 1;
        pixels = pixels.into_iter().step_by(step).collect();
    }
    let mut buckets: Vec<Vec<[u8; 3]>> = vec![pixels];
    while buckets.len() < n {
        // Split the bucket with the largest range.
        let mut best: Option<(usize, usize, i32)> = None; // (bucket, channel, range)
        for (bi, b) in buckets.iter().enumerate() {
            if b.len() < 2 {
                continue;
            }
            for ch in 0..3 {
                let (mn, mx) = b.iter().fold((255i32, 0i32), |(mn, mx), p| {
                    (mn.min(p[ch] as i32), mx.max(p[ch] as i32))
                });
                let r = mx - mn;
                if best.map(|b| r > b.2).unwrap_or(true) {
                    best = Some((bi, ch, r));
                }
            }
        }
        let Some((bi, ch, r)) = best else { break };
        if r == 0 {
            break;
        }
        let mut b = buckets.remove(bi);
        b.sort_by_key(|p| p[ch]);
        let mid = b.len() / 2;
        let hi = b.split_off(mid);
        buckets.push(b);
        buckets.push(hi);
    }
    buckets
        .iter()
        .filter(|b| !b.is_empty())
        .map(|b| {
            let n = b.len() as u32;
            let s = b.iter().fold([0u32; 3], |acc, p| {
                [
                    acc[0] + p[0] as u32,
                    acc[1] + p[1] as u32,
                    acc[2] + p[2] as u32,
                ]
            });
            [(s[0] / n) as u8, (s[1] / n) as u8, (s[2] / n) as u8]
        })
        .collect()
}

/// Boundary rings of a binary mask (pixel-edge following). Outer rings are
/// counter-clockwise and holes clockwise, so even-odd filling is right.
pub fn contours(mask: &[bool], w: usize, h: usize) -> Vec<Vec<Point>> {
    // Build the set of boundary edges between mask and non-mask cells, then
    // chain them into rings. Each edge is a directed unit segment keeping
    // the filled cell on its left.
    use std::collections::HashMap;
    let at = |x: i64, y: i64| -> bool {
        if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 {
            false
        } else {
            mask[y as usize * w + x as usize]
        }
    };
    // Directed edges keyed by start vertex.
    let mut edges: HashMap<(i64, i64), Vec<(i64, i64)>> = HashMap::new();
    let mut push = |a: (i64, i64), b: (i64, i64)| edges.entry(a).or_default().push(b);
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            if !at(x, y) {
                continue;
            }
            // Image y grows down; walk so the filled cell is on the left in
            // image space (clockwise on screen), which flips to CCW in page space.
            if !at(x, y - 1) {
                push((x, y), (x + 1, y)); // top edge, left to right
            }
            if !at(x + 1, y) {
                push((x + 1, y), (x + 1, y + 1)); // right edge, down
            }
            if !at(x, y + 1) {
                push((x + 1, y + 1), (x, y + 1)); // bottom edge, right to left
            }
            if !at(x - 1, y) {
                push((x, y + 1), (x, y)); // left edge, up
            }
        }
    }
    let mut rings = Vec::new();
    let mut keys: Vec<(i64, i64)> = edges.keys().copied().collect();
    keys.sort_unstable();
    for start in keys {
        while let Some(first) = edges.get_mut(&start).and_then(|v| v.pop()) {
            let mut ring = vec![Point::new(start.0 as f64, start.1 as f64)];
            let mut cur = first;
            let mut prev = start;
            let mut guard = 0;
            while cur != start && guard < w * h * 4 {
                ring.push(Point::new(cur.0 as f64, cur.1 as f64));
                let Some(nexts) = edges.get_mut(&cur) else {
                    break;
                };
                if nexts.is_empty() {
                    break;
                }
                // Prefer turning right (keeps separate touching rings apart).
                let dir = (cur.0 - prev.0, cur.1 - prev.1);
                let right = (-dir.1, dir.0);
                let pick = nexts
                    .iter()
                    .position(|n| (n.0 - cur.0, n.1 - cur.1) == right)
                    .or_else(|| nexts.iter().position(|n| (n.0 - cur.0, n.1 - cur.1) == dir))
                    .unwrap_or(0);
                let next = nexts.remove(pick);
                prev = cur;
                cur = next;
                guard += 1;
            }
            // Drop collinear points.
            let mut compact: Vec<Point> = Vec::new();
            for p in ring {
                let n = compact.len();
                if n >= 2 {
                    let a = compact[n - 2];
                    let b = compact[n - 1];
                    if (b.x - a.x) * (p.y - b.y) - (b.y - a.y) * (p.x - b.x) == 0.0 {
                        compact.pop();
                    }
                }
                compact.push(p);
            }
            if compact.len() >= 3 {
                rings.push(compact);
            }
        }
    }
    rings
}

// ----- centreline mode --------------------------------------------------------

fn centerline(img: &RgbaImage, s: &Settings) -> Vec<Traced> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let mut mask: Vec<bool> = img
        .pixels()
        .map(|p| p[3] >= 128 && luma(*p) < 128.0)
        .collect();
    // Mean stroke thickness before thinning: area / skeleton length later.
    let area = mask.iter().filter(|m| **m).count() as f64;
    thin(&mut mask, w, h);
    let skeleton = mask.iter().filter(|m| **m).count().max(1) as f64;
    let width = (area / skeleton).clamp(1.0, 50.0);
    // Trace skeleton pixels into polylines by walking neighbours.
    let mut visited = vec![false; w * h];
    let idx = |x: usize, y: usize| y * w + x;
    let neighbours = |x: usize, y: usize, mask: &[bool]| -> Vec<(usize, usize)> {
        let mut v = Vec::new();
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx >= 0
                    && ny >= 0
                    && (nx as usize) < w
                    && (ny as usize) < h
                    && mask[ny as usize * w + nx as usize]
                {
                    v.push((nx as usize, ny as usize));
                }
            }
        }
        v
    };
    let mut lines: Vec<Vec<Point>> = Vec::new();
    // Start from endpoints first (one neighbour), then anything left.
    let mut starts: Vec<(usize, usize)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if mask[idx(x, y)] && neighbours(x, y, &mask).len() == 1 {
                starts.push((x, y));
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            if mask[idx(x, y)] {
                starts.push((x, y));
            }
        }
    }
    for (sx, sy) in starts {
        if visited[idx(sx, sy)] {
            continue;
        }
        let mut line = vec![Point::new(sx as f64 + 0.5, sy as f64 + 0.5)];
        visited[idx(sx, sy)] = true;
        let (mut x, mut y) = (sx, sy);
        loop {
            let next = neighbours(x, y, &mask)
                .into_iter()
                .find(|(nx, ny)| !visited[idx(*nx, *ny)]);
            let Some((nx, ny)) = next else { break };
            visited[idx(nx, ny)] = true;
            line.push(Point::new(nx as f64 + 0.5, ny as f64 + 0.5));
            x = nx;
            y = ny;
        }
        if line.len() >= 2 {
            lines.push(line);
        }
    }
    let tolerance = 0.5 + (100.0 - s.detail) / 100.0 * 2.0;
    let mut out = Vec::new();
    for l in lines {
        let simplified = tracedraw_core::geometry::simplify(&l, tolerance);
        if simplified.len() < 2 {
            continue;
        }
        let path = if s.smoothing > 0.0 && simplified.len() > 2 {
            tracedraw_core::geometry::smooth_path(&simplified, false)
        } else {
            tracedraw_core::geometry::polyline_path(&simplified, false)
        };
        out.push(Traced {
            path,
            color: Color::BLACK,
            closed: false,
        });
    }
    // Encode the stroke width in the first result's colour alpha? No: the
    // caller reads `stroke_width` through the returned tuple below.
    STROKE_WIDTH.with(|c| c.set(width));
    out
}

thread_local! {
    static STROKE_WIDTH: std::cell::Cell<f64> = const { std::cell::Cell::new(1.0) };
}

/// Mean stroke width (pixels) measured by the last centreline trace.
pub fn last_stroke_width_px() -> f64 {
    STROKE_WIDTH.with(|c| c.get())
}

/// Zhang-Suen thinning.
pub fn thin(mask: &mut [bool], w: usize, h: usize) {
    let at = |m: &[bool], x: i64, y: i64| -> bool {
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && m[y as usize * w + x as usize]
    };
    loop {
        let mut changed = false;
        for pass in 0..2 {
            let mut remove = Vec::new();
            for y in 0..h as i64 {
                for x in 0..w as i64 {
                    if !at(mask, x, y) {
                        continue;
                    }
                    let p2 = at(mask, x, y - 1);
                    let p3 = at(mask, x + 1, y - 1);
                    let p4 = at(mask, x + 1, y);
                    let p5 = at(mask, x + 1, y + 1);
                    let p6 = at(mask, x, y + 1);
                    let p7 = at(mask, x - 1, y + 1);
                    let p8 = at(mask, x - 1, y);
                    let p9 = at(mask, x - 1, y - 1);
                    let ring = [p2, p3, p4, p5, p6, p7, p8, p9];
                    let b = ring.iter().filter(|v| **v).count();
                    if !(2..=6).contains(&b) {
                        continue;
                    }
                    let a = (0..8).filter(|i| !ring[*i] && ring[(i + 1) % 8]).count();
                    if a != 1 {
                        continue;
                    }
                    let (c1, c2) = if pass == 0 {
                        (p2 && p4 && p6, p4 && p6 && p8)
                    } else {
                        (p2 && p4 && p8, p2 && p6 && p8)
                    };
                    if c1 || c2 {
                        continue;
                    }
                    remove.push((x as usize, y as usize));
                }
            }
            if !remove.is_empty() {
                changed = true;
            }
            for (x, y) in remove {
                mask[y * w + x] = false;
            }
        }
        if !changed {
            break;
        }
    }
}

/// Map traced pixel-space paths into a rectangle (page mm, Y up).
pub fn to_rect(path: &BezPath, img_w: u32, img_h: u32, rect: Rect) -> BezPath {
    let sx = rect.width() / img_w.max(1) as f64;
    let sy = rect.height() / img_h.max(1) as f64;
    tracedraw_core::geometry::Affine::new([sx, 0.0, 0.0, -sy, rect.x0, rect.y1]) * path.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageBuffer;
    use tracedraw_core::geometry::Shape as _;

    fn circle_image() -> RgbaImage {
        ImageBuffer::from_fn(100, 100, |x, y| {
            let d = ((x as f32 - 50.0).powi(2) + (y as f32 - 50.0).powi(2)).sqrt();
            if d < 30.0 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        })
    }

    #[test]
    fn outline_traces_a_circle_with_right_area() {
        let img = circle_image();
        let t = trace(&img, &Preset::LineArt.settings());
        assert_eq!(t.len(), 1, "background removed, one region");
        let area = t[0].path.area().abs();
        let expect = std::f64::consts::PI * 30.0 * 30.0;
        assert!(
            (area - expect).abs() / expect < 0.05,
            "area {area} vs {expect}"
        );
        assert_eq!(t[0].color.to_rgb8(), [0, 0, 0]);
    }

    #[test]
    fn centerline_traces_a_line() {
        let img = ImageBuffer::from_fn(100, 40, |x, y| {
            if (18..=22).contains(&y) && x > 5 && x < 95 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let t = trace(&img, &Preset::Technical.settings());
        assert!(!t.is_empty());
        let b = t[0].path.bounding_box();
        assert!(b.width() > 80.0 && b.height() < 3.0, "{b:?}");
        assert!((last_stroke_width_px() - 5.0).abs() < 1.5);
    }

    #[test]
    fn contours_of_a_square_with_hole() {
        let w = 10;
        let mut mask = vec![false; 100];
        for y in 2..8 {
            for x in 2..8 {
                mask[y * w + x] = true;
            }
        }
        for y in 4..6 {
            for x in 4..6 {
                mask[y * w + x] = false;
            }
        }
        let rings = contours(&mask, w, 10);
        assert_eq!(rings.len(), 2);
    }
}
