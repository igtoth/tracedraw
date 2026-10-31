//! Bitmap to vector tracing: outline mode (colour regions become filled
//! curves) and centreline mode (strokes become open curves). See
//! docs/behavior/bitmap-tracing.md for the algorithm and defaults.

use image::{Rgba, RgbaImage};
use tracedraw_core::{
    geometry::{BezPath, Point, Rect},
    Color,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    /// Outline, with the detail chosen for each picture: the one that
    /// draws it most faithfully in the fewest nodes (see [`optimize`]).
    Optimized,
}

impl Preset {
    /// Every preset, centreline ones first.
    pub const ALL: [Preset; 9] = [
        Preset::Technical,
        Preset::LineDrawing,
        Preset::LineArt,
        Preset::Logo,
        Preset::DetailedLogo,
        Preset::Clipart,
        Preset::LowQualityImage,
        Preset::HighQualityImage,
        Preset::Optimized,
    ];

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
            Preset::Optimized => "menu.bitmaps.trace_optimized",
        }
    }
    /// The presets of one trace type, in the order the dialog lists them.
    pub fn of_type(centerline: bool) -> &'static [Preset] {
        if centerline {
            &Preset::ALL[..2]
        } else {
            &Preset::ALL[2..]
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
            // The most colours the picture may keep: shades of one
            // colour join, so a flat picture keeps only its own.
            Preset::Optimized => (70.0, 25.0, 32, false),
        };
        Settings {
            detail,
            smoothing,
            corner_smoothness: 0.0,
            colors,
            mode: if bw {
                ColorMode::BlackWhite
            } else {
                ColorMode::Rgb
            },
            remove: Remove::Background,
            background: None,
            merge_adjacent: true,
            remove_overlap: false,
            group_by_color: false,
            centerline: self.is_centerline(),
            optimize: self == Preset::Optimized,
            delete_original: false,
            merged: Vec::new(),
            recolored: Vec::new(),
        }
    }
}

/// The colours a trace works in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorMode {
    #[default]
    Rgb,
    /// The picture's greys, as many as Colors asks for.
    Grayscale,
    /// Two colours, by brightness.
    BlackWhite,
}

impl ColorMode {
    pub const ALL: [ColorMode; 3] = [ColorMode::Rgb, ColorMode::Grayscale, ColorMode::BlackWhite];
    pub fn key(self) -> &'static str {
        match self {
            ColorMode::Rgb => "dialog.trace_mode_rgb",
            ColorMode::Grayscale => "dialog.trace_mode_grayscale",
            ColorMode::BlackWhite => "dialog.black_and_white",
        }
    }
}

/// What a trace leaves out (the Remove list of the dialog).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Remove {
    /// The background: the regions of the background colour that touch
    /// the picture's edge. Patches of the same colour inside the picture
    /// stay.
    #[default]
    Background,
    /// Every region of the background colour, inside the picture too.
    EntireColor,
    /// Nothing: the background is traced like any colour.
    None,
}

impl Remove {
    pub const ALL: [Remove; 3] = [Remove::Background, Remove::EntireColor, Remove::None];
    pub fn key(self) -> &'static str {
        match self {
            Remove::Background => "dialog.trace_remove_background_color",
            Remove::EntireColor => "dialog.trace_remove_entire_color",
            Remove::None => "dialog.trace_remove_none",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub detail: f64,
    pub smoothing: f64,
    pub corner_smoothness: f64,
    pub colors: u32,
    pub mode: ColorMode,
    pub remove: Remove,
    /// The background colour `remove` takes out; `None` finds it (the
    /// commonest colour on the picture's edge).
    pub background: Option<[u8; 3]>,
    /// One curve per colour (every region of a colour combined); off,
    /// each separate patch of a colour is a curve of its own.
    pub merge_adjacent: bool,
    /// Each region exactly its own pixels; off, a region reaches a little
    /// under the regions drawn over it, so no page shows between them.
    pub remove_overlap: bool,
    /// The result holds a group per colour.
    pub group_by_color: bool,
    pub centerline: bool,
    /// The detail is chosen for each picture traced (the Optimized
    /// preset, [`optimize`]); `detail` is where the search starts.
    pub optimize: bool,
    pub delete_original: bool,
    /// Colours of the trace joined into one, the first of each group
    /// (the Colors tab's Merge).
    pub merged: Vec<Vec<[u8; 3]>>,
    /// Colours of the trace drawn as another colour (the Colors tab's
    /// colour buttons): (traced, shown).
    pub recolored: Vec<([u8; 3], [u8; 3])>,
}

impl Settings {
    /// Whether the background is removed at all.
    pub fn removes_background(&self) -> bool {
        self.remove != Remove::None
    }
    /// The colour `c` is shown as (the Colors tab's edits).
    pub fn shown_color(&self, c: [u8; 3]) -> [u8; 3] {
        self.recolored
            .iter()
            .find(|(from, _)| *from == c)
            .map(|(_, to)| *to)
            .unwrap_or(c)
    }

    /// Only what changes the traced curves and the colours they are
    /// traced as: the Colors tab's recolouring (see [`recolor`]),
    /// grouping and deleting the original are left at rest, and so are
    /// the values the trace type or mode does not use, so settings that
    /// trace alike compare (and hash) alike.
    pub fn traced_part(&self) -> Settings {
        let mut t = self.clone();
        t.recolored.clear();
        t.group_by_color = false;
        t.delete_original = false;
        if t.remove == Remove::None {
            t.background = None;
        }
        if t.mode == ColorMode::BlackWhite {
            t.colors = 2;
        }
        if t.centerline {
            // A centreline trace reads the detail and the smoothing only.
            t.corner_smoothness = 0.0;
            t.colors = 2;
            t.mode = ColorMode::BlackWhite;
            t.remove = Remove::None;
            t.background = None;
            t.merge_adjacent = false;
            t.remove_overlap = false;
            t.merged.clear();
            t.optimize = false;
        }
        t
    }
}

/// `traced` (made with [`Settings::traced_part`]) in the colours the
/// Colors tab gave them: the same as tracing with `s` itself.
pub fn recolor(traced: &mut [Traced], s: &Settings) {
    if s.recolored.is_empty() {
        return;
    }
    for t in traced {
        let c = t.color.to_rgb8();
        let to = s.shown_color(c);
        if to != c {
            t.color = Color::rgb8(to[0], to[1], to[2]);
        }
    }
}

/// One traced region or stroke.
#[derive(Debug, Clone, PartialEq)]
pub struct Traced {
    pub path: BezPath,
    pub color: Color,
    pub closed: bool,
}

/// What the dialog's status line says of a trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    pub curves: usize,
    pub nodes: usize,
    pub colors: usize,
}

/// The counts of `traced`: curves (one per object), their nodes and the
/// distinct colours.
pub fn stats(traced: &[Traced]) -> Stats {
    let mut colors: Vec<[u8; 3]> = Vec::new();
    let mut nodes = 0;
    for t in traced {
        nodes += tracedraw_core::nodes::nodes(&t.path).len();
        let c = t.color.to_rgb8();
        if !colors.contains(&c) {
            colors.push(c);
        }
    }
    Stats {
        curves: traced.len(),
        nodes,
        colors: colors.len(),
    }
}

/// The distinct colours of `traced`, each with its number of curves, in
/// the order the regions stack (darkest first).
pub fn color_counts(traced: &[Traced]) -> Vec<([u8; 3], usize)> {
    let mut out: Vec<([u8; 3], usize)> = Vec::new();
    for t in traced {
        let c = t.color.to_rgb8();
        match out.iter_mut().find(|(k, _)| *k == c) {
            Some((_, n)) => *n += 1,
            None => out.push((c, 1)),
        }
    }
    out
}

/// A flag a trace running on another thread looks at between its steps:
/// set, the trace stops and gives nothing back.
pub type Cancel = std::sync::atomic::AtomicBool;

/// The flood fills look at the flag once per this many pixels.
const CANCEL_EVERY: usize = 4096;

fn cancelled(c: Option<&Cancel>) -> bool {
    c.is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed))
}

/// Trace `img` into shapes whose coordinates are in pixel space (y down).
/// The caller maps them into the bitmap's rectangle.
pub fn trace(img: &RgbaImage, s: &Settings) -> Vec<Traced> {
    trace_unless(img, s, None).unwrap_or_default()
}

/// [`trace`], giving up (`None`) once `cancel` is set.
pub fn trace_unless(img: &RgbaImage, s: &Settings, cancel: Option<&Cancel>) -> Option<Vec<Traced>> {
    if s.centerline {
        return centerline(img, s, cancel);
    }
    if s.optimize {
        let best = optimize(img, s, cancel)?;
        return outline(img, &best, cancel);
    }
    outline(img, s, cancel)
}

// ----- the best settings for a picture -----------------------------------------

/// The details [`optimize`] tries, coarse to fine. Above 85 the cleaning
/// of edges and specks stops and the curves multiply.
#[cfg(not(target_arch = "wasm32"))]
const OPTIMIZE_DETAILS: [f64; 5] = [50.0, 60.0, 70.0, 80.0, 85.0];
/// In the browser the details are traced one after the other, in the
/// frame: the three in the middle, where the search lands on nearly every
/// picture.
#[cfg(target_arch = "wasm32")]
const OPTIMIZE_DETAILS: [f64; 3] = [60.0, 70.0, 80.0];

/// The side of the samples [`optimize`] traces, pixels, and how many.
const SAMPLE_SIDE: u32 = 200;
const SAMPLES: usize = 4;

/// What a node costs against a pixel drawn wrong, in [`optimize`]'s
/// score: the share of pixels drawn wrong (percent) plus this much per
/// node for each pixel of the samples' sides.
const NODE_COST: f64 = 0.3;

/// The settings `s` with the detail that draws `img` best: each
/// detail of [`OPTIMIZE_DETAILS`] traces samples of the picture at its
/// own size (the [`SAMPLES`] squares of its art with the most edges, or
/// the whole art when small, side by side), the trace is drawn back and
/// compared with them ([`wrong_share`]), and the detail with the lowest
/// score wins: the share drawn wrong plus [`NODE_COST`] per node per
/// pixel of the samples' sides, so a finer detail is chosen while it
/// draws the picture better for the nodes it adds, and no further. The
/// result traces as it is (`optimize` off). `None` once `cancel` is set.
pub fn optimize(img: &RgbaImage, s: &Settings, cancel: Option<&Cancel>) -> Option<Settings> {
    let mut base = s.clone();
    base.optimize = false;
    if s.centerline {
        return Some(base);
    }
    let samples = samples_of(img);
    if samples.is_empty() {
        return Some(base);
    }
    let sides: f64 = samples
        .iter()
        .map(|p| (p.width() + p.height()) as f64)
        .sum();
    // The samples side by side, a transparent pixel apart, traced at once.
    let sheet = sheet_of(&samples);
    // The samples hold the art only: its background, transparent around
    // them, is no colour of theirs to take out (the colour along a
    // sample's edge is the art's).
    let mut sample_settings = base.clone();
    sample_settings.remove = Remove::None;
    sample_settings.background = None;
    let score = |detail: f64| -> Option<f64> {
        let mut t = sample_settings.clone();
        t.detail = detail;
        let (w, h) = sheet.dimensions();
        let traced = outline(&sheet, &t, cancel)?;
        let drawn = render_traced(&traced, w, h, 1.0);
        let wrong = wrong_share(&sheet, &drawn, cancel)?;
        Some(wrong + NODE_COST * stats(&traced).nodes as f64 / sides.max(1.0))
    };
    // Each detail on a thread of its own where there are threads.
    #[cfg(not(target_arch = "wasm32"))]
    let scores: Vec<Option<f64>> = std::thread::scope(|scope| {
        let runs: Vec<_> = OPTIMIZE_DETAILS
            .iter()
            .map(|d| scope.spawn(move || score(*d)))
            .collect();
        runs.into_iter().map(|r| r.join().ok().flatten()).collect()
    });
    #[cfg(target_arch = "wasm32")]
    let scores: Vec<Option<f64>> = OPTIMIZE_DETAILS.iter().map(|d| score(*d)).collect();
    if cancelled(cancel) {
        return None;
    }
    let best = OPTIMIZE_DETAILS
        .iter()
        .zip(&scores)
        .filter_map(|(d, s)| s.map(|s| (*d, s)))
        .min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((detail, _)) = best {
        base.detail = detail;
    }
    Some(base)
}

/// `samples` in a row, a transparent pixel between each two.
fn sheet_of(samples: &[RgbaImage]) -> RgbaImage {
    let w: u32 = samples.iter().map(|p| p.width() + 1).sum();
    let h = samples.iter().map(|p| p.height()).max().unwrap_or(0);
    let mut sheet = RgbaImage::new(w.max(1), h.max(1));
    let mut x = 0;
    for p in samples {
        image::imageops::replace(&mut sheet, p, x as i64, 0);
        x += p.width() + 1;
    }
    sheet
}

/// The parts of `img` [`optimize`] traces: its opaque pixels' box when it
/// is no more than two samples across, else the [`SAMPLES`] squares of
/// [`SAMPLE_SIDE`] in that box with the most edges (neighbouring pixels
/// more than 60 apart), the art's finest parts, where the detail shows.
fn samples_of(img: &RgbaImage) -> Vec<RgbaImage> {
    let (w, h) = img.dimensions();
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] >= 128 {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
    }
    if x0 > x1 || x1 - x0 < 8 || y1 - y0 < 8 {
        return Vec::new();
    }
    let (bw, bh) = (x1 - x0 + 1, y1 - y0 + 1);
    if bw <= 2 * SAMPLE_SIDE && bh <= 2 * SAMPLE_SIDE {
        return vec![image::imageops::crop_imm(img, x0, y0, bw, bh).to_image()];
    }
    let raw = img.as_raw();
    let px = |x: u32, y: u32| {
        let i = (y * w + x) as usize * 4;
        [
            raw[i] as i32,
            raw[i + 1] as i32,
            raw[i + 2] as i32,
            raw[i + 3] as i32,
        ]
    };
    let differs = |a: [i32; 4], b: [i32; 4]| {
        (a[3] >= 128) != (b[3] >= 128) || (0..3).map(|k| (a[k] - b[k]).pow(2)).sum::<i32>() > 3600
    };
    // Squares on a grid over the box, scored by their edges.
    let side = SAMPLE_SIDE.min(bw).min(bh);
    let mut scored: Vec<(u64, u32, u32)> = Vec::new();
    let mut ty = y0;
    while ty + side <= y1 + 1 {
        let mut tx = x0;
        while tx + side <= x1 + 1 {
            let mut edges = 0u64;
            for y in (ty..ty + side - 1).step_by(2) {
                for x in tx..tx + side - 1 {
                    let p = px(x, y);
                    if differs(p, px(x + 1, y)) || differs(p, px(x, y + 1)) {
                        edges += 1;
                    }
                }
            }
            scored.push((edges, tx, ty));
            tx += side;
        }
        ty += side;
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    scored
        .iter()
        .take(SAMPLES)
        .map(|(_, x, y)| image::imageops::crop_imm(img, *x, *y, side, side).to_image())
        .collect()
}

/// How far (RGB distance, both over white) a pixel's colour must be from
/// every pixel of the other picture within a pixel of it to be drawn
/// wrong.
const WRONG_DISTANCE: f64 = 60.0;

/// The share (percent) of the pixels of `a` and `b` (the same size; the
/// pixels opaque in either) drawn wrong: whose colour in one picture is
/// further than [`WRONG_DISTANCE`] from every pixel of the other within a
/// pixel. An edge drawn a pixel off is not wrong; a lost dot or line, a
/// region in another colour, a gap or a blob are.
pub fn wrong_share(a: &RgbaImage, b: &RgbaImage, cancel: Option<&Cancel>) -> Option<f64> {
    let (w, h) = a.dimensions();
    if b.dimensions() != (w, h) || w == 0 || h == 0 {
        return Some(100.0);
    }
    let over = |p: &Rgba<u8>| -> [f32; 3] {
        let k = p[3] as f32 / 255.0;
        std::array::from_fn(|c| p[c] as f32 * k + 255.0 * (1.0 - k))
    };
    let pa: Vec<[f32; 3]> = a.pixels().map(over).collect();
    let pb: Vec<[f32; 3]> = b.pixels().map(over).collect();
    let (w, h) = (w as usize, h as usize);
    let limit = (WRONG_DISTANCE * WRONG_DISTANCE) as f32;
    let far = |img: &[[f32; 3]], x: usize, y: usize, c: [f32; 3]| {
        (y.saturating_sub(1)..=(y + 1).min(h - 1)).all(|v| {
            (x.saturating_sub(1)..=(x + 1).min(w - 1)).all(|u| {
                let q = img[v * w + u];
                (0..3).map(|k| (q[k] - c[k]).powi(2)).sum::<f32>() > limit
            })
        })
    };
    let (ra, rb) = (a.as_raw(), b.as_raw());
    let (mut seen, mut wrong) = (0usize, 0usize);
    for y in 0..h {
        if y % 32 == 0 && cancelled(cancel) {
            return None;
        }
        for x in 0..w {
            let i = y * w + x;
            if ra[i * 4 + 3] < 8 && rb[i * 4 + 3] < 8 {
                continue;
            }
            seen += 1;
            if far(&pb, x, y, pa[i]) || far(&pa, x, y, pb[i]) {
                wrong += 1;
            }
        }
    }
    Some(if seen == 0 {
        0.0
    } else {
        wrong as f64 * 100.0 / seen as f64
    })
}

/// The palette index nearest `c` (none for an empty palette).
fn palette_index(palette: &[[u8; 3]], c: [u8; 3]) -> Option<usize> {
    (!palette.is_empty()).then(|| nearest(palette, c))
}

/// `labels` with the colours of each group of `merged` relabelled to the
/// group's first colour; a colour matches the palette entry nearest it
/// when within `reach` (RGB distance, squared).
fn merge_colors(labels: &mut [usize], palette: &[[u8; 3]], merged: &[Vec<[u8; 3]>], reach: u32) {
    let mut alias: Vec<usize> = (0..palette.len()).collect();
    for group in merged {
        let Some(first) = group.first() else {
            continue;
        };
        let Some(to) = palette_index(palette, *first) else {
            continue;
        };
        if distance2(palette[to], *first) > reach {
            continue;
        }
        for c in &group[1..] {
            if let Some(from) = palette_index(palette, *c) {
                if distance2(palette[from], *c) <= reach && from != to {
                    alias[from] = to;
                }
            }
        }
    }
    if alias.iter().enumerate().all(|(i, a)| i == *a) {
        return;
    }
    for l in labels.iter_mut() {
        if let Some(a) = alias.get(*l) {
            *l = *a;
        }
    }
}

/// Set the pixels of label `bg` that reach the picture's edge (through
/// pixels of that label, 4-connected) to transparent, so only the patches
/// of that colour inside the picture are left to trace.
fn clear_border_region(labels: &mut [usize], w: usize, h: usize, bg: usize) {
    if labels.len() != w * h {
        return;
    }
    let mut stack: Vec<usize> = Vec::new();
    let push = |i: usize, labels: &mut [usize], stack: &mut Vec<usize>| {
        if labels[i] == bg {
            labels[i] = usize::MAX;
            stack.push(i);
        }
    };
    for x in 0..w {
        push(x, labels, &mut stack);
        push((h - 1) * w + x, labels, &mut stack);
    }
    for y in 0..h {
        push(y * w, labels, &mut stack);
        push(y * w + w - 1, labels, &mut stack);
    }
    while let Some(i) = stack.pop() {
        let (x, y) = (i % w, i / w);
        if x > 0 {
            push(i - 1, labels, &mut stack);
        }
        if x + 1 < w {
            push(i + 1, labels, &mut stack);
        }
        if y > 0 {
            push(i - w, labels, &mut stack);
        }
        if y + 1 < h {
            push(i + w, labels, &mut stack);
        }
    }
}

/// The 8-connected patches of `mask` (`w` x `h`), each as the box of its
/// pixels (x0, y0, x1, y1, inclusive) and its own mask over that box.
fn patches(mask: &[bool], w: usize, h: usize) -> Vec<((usize, usize, usize, usize), Vec<bool>)> {
    let mut out = Vec::new();
    if mask.len() != w * h {
        return out;
    }
    let mut seen = vec![false; w * h];
    let mut stack: Vec<usize> = Vec::new();
    let mut pixels: Vec<usize> = Vec::new();
    for start in 0..w * h {
        if seen[start] || !mask[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        pixels.clear();
        let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
        while let Some(i) = stack.pop() {
            pixels.push(i);
            let (x, y) = (i % w, i / w);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if mask[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
        }
        let (pw, ph) = (x1 - x0 + 1, y1 - y0 + 1);
        let mut own = vec![false; pw * ph];
        for &i in &pixels {
            let (x, y) = (i % w, i / w);
            own[(y - y0) * pw + (x - x0)] = true;
        }
        out.push(((x0, y0, x1, y1), own));
    }
    out
}

// ----- outline mode -----------------------------------------------------------

fn outline(img: &RgbaImage, s: &Settings, cancel: Option<&Cancel>) -> Option<Vec<Traced>> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w == 0 || h == 0 {
        return Some(Vec::new());
    }
    // 1. Take out the noise of photos and compressed images below the
    // highest detail, so it makes no specks, holes and seams of its own
    // inside flat colours (#611); smooth a little more when detail is
    // low, then quantise.
    let mut quiet = img.clone();
    for _ in 0..denoise_passes(s.detail) {
        quiet = denoise(&quiet, cancel)?;
    }
    let mut src = if s.detail < 50.0 {
        image::imageops::blur(&quiet, (50.0 - s.detail as f32) / 25.0)
    } else {
        quiet
    };
    if cancelled(cancel) {
        return None;
    }
    if s.mode == ColorMode::Grayscale {
        for p in src.pixels_mut() {
            let l = luma(*p).round().clamp(0.0, 255.0) as u8;
            *p = Rgba([l, l, l, p[3]]);
        }
    }
    let black_white = s.mode == ColorMode::BlackWhite;
    let palette: Vec<[u8; 3]> = if black_white {
        vec![[0, 0, 0], [255, 255, 255]]
    } else {
        // At the highest detail every colour counts, the edges' too.
        let n = s.colors.clamp(2, 256) as usize;
        if speck_area(s.detail) <= 1 {
            median_cut(&src, n, cancel)?
        } else {
            art_palette(&src, n, cancel)?
        }
    };
    let mut labels = vec![0usize; w * h];
    for (x, y, p) in src.enumerate_pixels() {
        // Up to 256 colours to compare each pixel with: look at the flag
        // every few rows.
        if x == 0 && y % 32 == 0 && cancelled(cancel) {
            return None;
        }
        let idx = if p[3] < 128 {
            usize::MAX
        } else if black_white {
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
    // Thin bands of an in-between colour along the edge of two others
    // (the anti-aliasing of a black and yellow edge, the fringes of a
    // blurred or compressed one) go to the colours on either side, each
    // pixel to the one its own colour is nearer, so they make no slivers
    // of their own and the edge runs where the art's did (#611, #1250).
    // Before the specks, which would hand them out by their neighbours
    // and leave steps and bumps along the edge. Like the specks, kept at
    // the highest detail.
    let colors: Vec<[u8; 3]> = src.pixels().map(|p| [p[0], p[1], p[2]]).collect();
    // One flat colour split in two by its noise, when the art has fewer
    // colours than asked for, is one colour again (#611).
    let palette = merge_split_colors(&mut labels, palette, &colors);
    // Shades no eye tells apart (the three blacks and seven whites the
    // colour reduction makes of a black and white drawing's noise) are
    // one colour, in any picture: each would be a curve of its own.
    // Rounds until nothing joins: shades joined change which others are
    // side by side.
    // Black and white stays #000000 and #FFFFFF: its two colours are
    // the mode's, not the picture's.
    let mut palette = palette;
    if !black_white {
        for _ in 0..4 {
            let before = labels.clone();
            palette = merge_near_colors(&mut labels, w, h, palette, &colors, near_reach(s.detail));
            if labels == before || cancelled(cancel) {
                break;
            }
        }
        snap_to_commonest(&labels, &colors, &mut palette);
    }
    // Colours no pixel has any more (joined into others) leave the
    // palette, so the background colour specified, picked with the
    // eyedropper or merged on the Colors tab is matched among the colours
    // of the trace only: a joined shade left behind stood nearer the
    // colour picked than the colour its pixels took, and nothing was
    // removed.
    let palette = compact_palette(&mut labels, palette);
    if speck_area(s.detail) > 1 {
        settle_edges(&mut labels, w, h, &palette, &colors, cancel);
        dissolve_fringes(&mut labels, w, h, &palette, &colors, cancel)?;
    }
    // Specks: patches of one colour smaller than the detail allows take
    // the colour around them, so stray dots and the fringes of soft edges
    // do not become curves of their own (#611).
    despeckle(
        &mut labels,
        w,
        h,
        speck_area(s.detail),
        dot_area(s.detail),
        &palette,
        cancel,
    );
    if cancelled(cancel) {
        return None;
    }
    // The one-pixel bumps and notches left along the edges go, so the
    // curves fitted to them are smooth (#1250).
    if speck_area(s.detail) > 1 {
        smooth_labels(&mut labels, w, h, cancel)?;
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
    if cancelled(cancel) {
        return None;
    }
    // Colours the Colors tab joined are one colour from here on; the
    // match must be close, so a palette that changed with the settings
    // does not join colours never meant to be.
    if !s.merged.is_empty() {
        merge_colors(&mut labels, &palette, &s.merged, MERGE_REACH * MERGE_REACH);
        if cancelled(cancel) {
            return None;
        }
    }
    let mut background = match s.remove {
        Remove::None => None,
        Remove::Background | Remove::EntireColor => match s.background {
            // The colour chosen: the palette colour nearest it.
            Some(c) => palette_index(&palette, c),
            // None when no colour touches the edge (a picture on a
            // transparent background has its background removed already):
            // the commonest of none was the last colour of the palette,
            // and Remove entire color took it out of the art (#1250).
            None => border_counts
                .iter()
                .enumerate()
                .filter(|(_, c)| **c > 0)
                .max_by_key(|(_, c)| **c)
                .map(|(i, _)| i),
        },
    };
    // Removing the background (not the entire colour) takes out the
    // patches of that colour reaching the picture's edge; patches of it
    // inside the picture (the white of an eye) are traced like any colour.
    if let (Remove::Background, Some(bg)) = (s.remove, background) {
        clear_border_region(&mut labels, w, h, bg);
        background = None;
    }
    // 3. The box of each colour's pixels, in one pass.
    let mut boxes = vec![(w, h, 0usize, 0usize); palette.len()];
    for (i, l) in labels.iter().enumerate() {
        if let Some(b) = boxes.get_mut(*l) {
            let (x, y) = (i % w, i / w);
            *b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
        }
    }
    let present: Vec<bool> = boxes.iter().map(|b| b.0 <= b.2).collect();
    // The background taken out is no region.
    let regions: std::borrow::Cow<[usize]> = match background {
        Some(bg) => labels
            .iter()
            .map(|l| if *l == bg { usize::MAX } else { *l })
            .collect::<Vec<usize>>()
            .into(),
        None => labels.as_slice().into(),
    };
    // The order the colours stack in: a colour inside another's holes
    // (the eyes in a head) above it, so the one around is drawn whole
    // under it, as one ring; else darkest first (#1250).
    let luma: Vec<u32> = palette
        .iter()
        .map(|c| (c[0] as u32 * 299 + c[1] as u32 * 587 + c[2] as u32 * 114) / 1000)
        .collect();
    // The holes of each colour with no page showing in them: their
    // pixels (in the colour's box) and the colours in them.
    let mut holes: Vec<Vec<(Vec<u32>, Vec<usize>)>> = vec![Vec::new(); palette.len()];
    let order = if s.remove_overlap {
        let mut order: Vec<usize> = (0..palette.len()).collect();
        order.sort_by_key(|i| luma[*i]);
        order
    } else {
        let mut inside: Vec<Vec<bool>> = vec![vec![false; palette.len()]; palette.len()];
        let mut seen = vec![false; palette.len()];
        for ci in 0..palette.len() {
            if !present[ci] || Some(ci) == background {
                continue;
            }
            if cancelled(cancel) {
                return None;
            }
            let (x0, y0, x1, y1) = boxes[ci];
            let (cw, ch) = (x1 - x0 + 1, y1 - y0 + 1);
            let label_at = |i: usize| regions[(y0 + i / cw) * w + x0 + i % cw];
            let own: Vec<bool> = (0..cw * ch).map(|i| label_at(i) == ci).collect();
            for_each_hole(&own, cw, ch, |hole| {
                seen.fill(false);
                for &i in hole {
                    match seen.get_mut(label_at(i)) {
                        Some(v) => *v = true,
                        // The page shows in this hole: it stays one.
                        None => return,
                    }
                }
                let colours: Vec<usize> = (0..seen.len()).filter(|l| seen[*l]).collect();
                for &l in &colours {
                    inside[ci][l] = true;
                }
                holes[ci].push((hole.iter().map(|i| *i as u32).collect(), colours));
            });
        }
        stacking_order(&inside, &luma)
    };
    let tolerance = 0.3 + (100.0 - s.detail) / 100.0 * 2.5;
    let mut rank = vec![0usize; palette.len()];
    for (k, ci) in order.iter().enumerate() {
        rank[*ci] = k;
    }
    // The borders between the regions, each fitted once: every region's
    // rings are put together from them, so where two regions meet their
    // outlines are the same curve, with no gap between them and no second
    // curve beside the first (#611, #1250).
    let shared = SharedBorders::new(&regions, w, h, tolerance, s, cancel)?;
    let mut out = Vec::new();
    for ci in order {
        if Some(ci) == background || !present[ci] {
            continue;
        }
        if cancelled(cancel) {
            return None;
        }
        let (x0, y0, x1, y1) = boxes[ci];
        let (cw, ch) = (x1 - x0 + 1, y1 - y0 + 1);
        let label_at = |i: usize| regions[(y0 + i / cw) * w + x0 + i % cw];
        let own: Vec<bool> = (0..cw * ch).map(|i| label_at(i) == ci).collect();
        // Stacked: the holes of the region holding only regions drawn
        // above it are part of it, so its curve is the outline alone and
        // the edges inside are drawn once, by the regions on top, which
        // leave no page showing between them and it. Remove overlap keeps
        // every region to its own pixels (no holes were looked for).
        let mut mask = own;
        for (pixels, colours) in &holes[ci] {
            if colours.iter().all(|l| rank[*l] > rank[ci]) {
                for &i in pixels {
                    mask[i as usize] = true;
                }
            }
        }
        // One curve for the colour (every region combined), or one per
        // separate patch of it.
        let pieces: Vec<((usize, usize), usize, usize, Vec<bool>)> = if s.merge_adjacent {
            vec![((x0, y0), cw, ch, mask)]
        } else {
            patches(&mask, cw, ch)
                .into_iter()
                .map(|((px0, py0, px1, py1), own)| {
                    ((x0 + px0, y0 + py0), px1 - px0 + 1, py1 - py0 + 1, own)
                })
                .collect()
        };
        let c = s.shown_color(palette[ci]);
        for ((ox, oy), pw, ph, piece) in pieces {
            if cancelled(cancel) {
                return None;
            }
            let path = shared.rings(&piece, (ox, oy), pw, ph, cancel)?;
            if path.elements().is_empty() {
                continue;
            }
            out.push(Traced {
                path,
                color: Color::rgb8(c[0], c[1], c[2]),
                closed: true,
            });
        }
    }
    Some(out)
}

/// How near (RGB distance) a colour of the Colors tab must come to a
/// palette colour to stand for it.
const MERGE_REACH: u32 = 12;

/// The boundary rings of `mask` (`w` x `h`, at `offset` in the picture)
/// as one path: each ring fitted, or simplified to straight lines without
/// smoothing.
#[cfg(test)]
fn rings_path(
    mask: &[bool],
    w: usize,
    h: usize,
    offset: tracedraw_core::geometry::Vec2,
    tolerance: f64,
    s: &Settings,
    cancel: Option<&Cancel>,
) -> Option<BezPath> {
    let labels: Vec<usize> = mask
        .iter()
        .map(|m| if *m { 0 } else { usize::MAX })
        .collect();
    let fitted = SharedBorders::new(&labels, w, h, tolerance, s, cancel)?;
    let path = fitted.rings(mask, (0, 0), w, h, cancel)?;
    Some(tracedraw_core::geometry::Affine::translate(offset) * path)
}

// ----- shared borders -------------------------------------------------------------

/// The unit steps along the pixel edges, from a pixel corner: east,
/// south, west, north (picture space, y down).
const STEPS: [(i64, i64); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

/// The pixels on either side of the step `d` from the corner `(x, y)`:
/// the `a` side first (the side a region's rings keep it on, as in
/// clockwise rings on screen), then the `b` side.
fn step_sides(x: i64, y: i64, d: u8) -> ((i64, i64), (i64, i64)) {
    match d {
        0 => ((x, y), (x, y - 1)),
        1 => ((x - 1, y), (x, y)),
        2 => ((x - 1, y - 1), (x - 1, y)),
        _ => ((x, y - 1), (x - 1, y - 1)),
    }
}

/// A stretch of the border between two regions of a labelling, along the
/// pixel edges: from one junction to the next (a pixel corner where
/// three regions meet, or two meet diagonally), or round a loop with no
/// junction (an island inside one other region). The region with the
/// smaller label is on its `a` side.
struct Border {
    /// The pixel corner it starts at and the one it ends at.
    start: (i64, i64),
    end: (i64, i64),
    /// The direction of each unit step (an index into [`STEPS`]).
    steps: Vec<u8>,
    /// Round a loop: it ends where it starts, with no junction on it.
    closed: bool,
    /// The pixel on each side of its first step (`None` outside the
    /// picture).
    a: Option<usize>,
    b: Option<usize>,
}

impl Border {
    /// The pixel corners it passes, from its start to its end.
    fn corners(&self) -> Vec<Point> {
        let mut out = Vec::with_capacity(self.steps.len() + 1);
        let (mut x, mut y) = self.start;
        out.push(Point::new(x as f64, y as f64));
        for d in &self.steps {
            let (dx, dy) = STEPS[*d as usize & 3];
            x += dx;
            y += dy;
            out.push(Point::new(x as f64, y as f64));
        }
        out
    }
}

/// The borders between the regions of `labels` (`w` x `h`; `usize::MAX`
/// is no region: transparent, or the background taken out). Every edge
/// between two pixels of different labels, and between a region and the
/// picture's edge, is on exactly one border. `None` once `cancel` is set.
fn borders(labels: &[usize], w: usize, h: usize, cancel: Option<&Cancel>) -> Option<Vec<Border>> {
    let mut out = Vec::new();
    if labels.len() != w * h || w == 0 || h == 0 {
        return Some(out);
    }
    // The labels with a frame of no region round them, so the four pixels
    // round every pixel corner can be read without looking at the edges.
    let pw = w + 2;
    let mut framed = vec![usize::MAX; pw * (h + 2)];
    for y in 0..h {
        framed[(y + 1) * pw + 1..(y + 1) * pw + 1 + w].copy_from_slice(&labels[y * w..(y + 1) * w]);
    }
    // For each pixel corner, the steps from it along an edge between two
    // labels, walked one way only, with the smaller label on the step's
    // `a` side (bits 0 to 3, see [`step_sides`]), and whether it is a
    // junction (bit 4): three labels or more round it, or two diagonally.
    const JUNCTION: u8 = 1 << 4;
    let cw = w + 1;
    let mut info = vec![0u8; cw * (h + 1)];
    for y in 0..=h {
        if y % 64 == 0 && cancelled(cancel) {
            return None;
        }
        let (r0, r1) = (y * pw, (y + 1) * pw);
        for x in 0..=w {
            let (tl, tr, bl, br) = (
                framed[r0 + x],
                framed[r0 + x + 1],
                framed[r1 + x],
                framed[r1 + x + 1],
            );
            if tl == tr && tl == bl && tl == br {
                continue;
            }
            let outs = (br < tr) as u8
                | ((bl < br) as u8) << 1
                | ((tl < bl) as u8) << 2
                | ((tr < tl) as u8) << 3;
            let distinct = 1
                + (tr != tl) as usize
                + (bl != tl && bl != tr) as usize
                + (br != tl && br != tr && br != bl) as usize;
            let junction = distinct >= 3 || (distinct == 2 && tl == br && tr == bl);
            info[y * cw + x] = outs | if junction { JUNCTION } else { 0 };
        }
    }
    let pixel = |(x, y): (i64, i64)| -> Option<usize> {
        (x >= 0 && y >= 0 && x < w as i64 && y < h as i64).then(|| y as usize * w + x as usize)
    };
    let corner = |x: i64, y: i64| y as usize * cw + x as usize;
    // The steps already walked, per corner.
    let mut used = vec![0u8; cw * (h + 1)];
    let limit = 4 * cw * (h + 1);
    let walk = |x0: i64, y0: i64, d0: u8, used: &mut [u8]| -> Border {
        let (a, b) = step_sides(x0, y0, d0);
        let mut steps = vec![d0];
        used[corner(x0, y0)] |= 1 << d0;
        let (mut x, mut y) = (x0 + STEPS[d0 as usize].0, y0 + STEPS[d0 as usize].1);
        let mut closed = false;
        while steps.len() <= limit {
            let here = info[corner(x, y)];
            if (x, y) == (x0, y0) {
                closed = here & JUNCTION == 0;
                break;
            }
            if here & JUNCTION != 0 {
                break;
            }
            let free = here & 0xF & !used[corner(x, y)];
            if free == 0 {
                break;
            }
            let d = free.trailing_zeros() as u8;
            used[corner(x, y)] |= 1 << d;
            steps.push(d);
            x += STEPS[d as usize].0;
            y += STEPS[d as usize].1;
        }
        Border {
            start: (x0, y0),
            end: (x, y),
            steps,
            closed,
            a: pixel(a),
            b: pixel(b),
        }
    };
    // The borders from junction to junction, then the loops.
    for pass in 0..2 {
        for y in 0..=h {
            if y % 64 == 0 && cancelled(cancel) {
                return None;
            }
            for x in 0..=w {
                let i = y * cw + x;
                if info[i] == 0 || (pass == 0 && info[i] & JUNCTION == 0) {
                    continue;
                }
                loop {
                    let free = info[i] & 0xF & !used[i];
                    if free == 0 {
                        break;
                    }
                    let d = free.trailing_zeros() as u8;
                    out.push(walk(x as i64, y as i64, d, &mut used));
                }
            }
        }
    }
    Some(out)
}

/// The side of the squares [`SharedBorders`] files its borders in, pixels.
const BORDER_TILE: i64 = 64;

/// The borders of a labelling, each fitted once ([`fit_border`]), and the
/// rings of any region made of whole regions of it put together from
/// them: where two regions meet, both use the same curve, one forwards,
/// the other backwards, so their outlines coincide exactly, with no gap
/// and no second curve beside the first (#611, #1250).
struct SharedBorders {
    w: usize,
    borders: Vec<Border>,
    curves: Vec<BezPath>,
    /// The borders starting in each square of [`BORDER_TILE`].
    tiles: Vec<Vec<u32>>,
    tiles_across: usize,
}

impl SharedBorders {
    fn new(
        labels: &[usize],
        w: usize,
        h: usize,
        tolerance: f64,
        s: &Settings,
        cancel: Option<&Cancel>,
    ) -> Option<SharedBorders> {
        let borders = borders(labels, w, h, cancel)?;
        let mut curves = Vec::with_capacity(borders.len());
        for (k, b) in borders.iter().enumerate() {
            if k % 256 == 0 && cancelled(cancel) {
                return None;
            }
            curves.push(fit_border(b, tolerance, s));
        }
        let tiles_across = w / BORDER_TILE as usize + 1;
        let mut tiles = vec![Vec::new(); tiles_across * (h / BORDER_TILE as usize + 1)];
        for (k, b) in borders.iter().enumerate() {
            let t = (b.start.1 / BORDER_TILE) as usize * tiles_across
                + (b.start.0 / BORDER_TILE) as usize;
            if let Some(v) = tiles.get_mut(t) {
                v.push(k as u32);
            }
        }
        Some(SharedBorders {
            w,
            borders,
            curves,
            tiles,
            tiles_across,
        })
    }

    /// The rings of `mask` (`mw` x `mh` at `(ox, oy)` in the picture),
    /// which must be made of whole regions of the labelling, as one path
    /// of the fitted borders. `None` once `cancel` is set.
    fn rings(
        &self,
        mask: &[bool],
        (ox, oy): (usize, usize),
        mw: usize,
        mh: usize,
        cancel: Option<&Cancel>,
    ) -> Option<BezPath> {
        let w = self.w;
        let inside = |p: Option<usize>| {
            p.is_some_and(|p| {
                let (x, y) = (p % w, p / w);
                x >= ox
                    && y >= oy
                    && x < ox + mw
                    && y < oy + mh
                    && mask.get((y - oy) * mw + (x - ox)).copied().unwrap_or(false)
            })
        };
        // The borders between the mask and the rest, each turned so the
        // mask is on its `a` side: (border, forwards).
        let mut open: Vec<(usize, bool)> = Vec::new();
        let mut path = BezPath::new();
        let (x0, y0) = (ox as i64, oy as i64);
        let (x1, y1) = ((ox + mw) as i64, (oy + mh) as i64);
        for ty in (y0 / BORDER_TILE)..=(y1 / BORDER_TILE) {
            for tx in (x0 / BORDER_TILE)..=(x1 / BORDER_TILE) {
                let Some(list) = self
                    .tiles
                    .get(ty as usize * self.tiles_across + tx as usize)
                else {
                    continue;
                };
                for &k in list {
                    let b = &self.borders[k as usize];
                    let (sx, sy) = b.start;
                    if sx < x0 || sy < y0 || sx > x1 || sy > y1 {
                        continue;
                    }
                    let (ia, ib) = (inside(b.a), inside(b.b));
                    if ia == ib {
                        continue;
                    }
                    if b.closed {
                        append_curve(&mut path, &self.curves[k as usize], ia, true);
                    } else {
                        open.push((k as usize, ia));
                    }
                }
            }
        }
        if cancelled(cancel) {
            return None;
        }
        // Chained into rings at the junctions, turning right where the
        // mask touches itself at a corner, so touching rings stay apart.
        let ends = |(k, fwd): (usize, bool)| -> ((i64, i64), (i64, i64), u8, u8) {
            let b = &self.borders[k];
            let (first, last) = (
                b.steps.first().copied().unwrap_or(0),
                b.steps.last().copied().unwrap_or(0),
            );
            if fwd {
                (b.start, b.end, first, last)
            } else {
                (b.end, b.start, (last + 2) % 4, (first + 2) % 4)
            }
        };
        let mut from: std::collections::HashMap<(i64, i64), Vec<usize>> =
            std::collections::HashMap::with_capacity(open.len());
        for (i, o) in open.iter().enumerate() {
            from.entry(ends(*o).0).or_default().push(i);
        }
        let mut taken = vec![false; open.len()];
        for first in 0..open.len() {
            if taken[first] {
                continue;
            }
            taken[first] = true;
            let (ring_start, mut at, _, mut dir) = ends(open[first]);
            path.move_to(Point::new(ring_start.0 as f64, ring_start.1 as f64));
            append_curve(&mut path, &self.curves[open[first].0], open[first].1, false);
            let mut guard = open.len();
            while at != ring_start && guard > 0 {
                guard -= 1;
                let Some(next) = from.get(&at).and_then(|c| {
                    let free = || c.iter().copied().filter(|i| !taken[*i]);
                    let turning = |t: u8| free().find(|i| ends(open[*i]).2 == t);
                    turning((dir + 1) % 4)
                        .or_else(|| turning(dir))
                        .or_else(|| free().next())
                }) else {
                    break;
                };
                taken[next] = true;
                append_curve(&mut path, &self.curves[open[next].0], open[next].1, false);
                let (_, e, _, l) = ends(open[next]);
                at = e;
                dir = l;
            }
            path.close_path();
        }
        Some(path)
    }
}

/// Append `curve` (a border's fitted curve: a move then lines and
/// curves, closed for a loop) to `path`, forwards or backwards. A loop is
/// appended whole (`closed`); an open border continues the ring `path`
/// is drawing, from where it ends.
fn append_curve(path: &mut BezPath, curve: &BezPath, forwards: bool, closed: bool) {
    use tracedraw_core::geometry::PathEl;
    let els = curve.elements();
    let Some(PathEl::MoveTo(p0)) = els.first().copied() else {
        return;
    };
    if forwards {
        let body = if closed { els } else { &els[1..] };
        path.extend(body.iter().copied());
        return;
    }
    // Each piece with the point it starts from, then the pieces the other
    // way round.
    let mut pieces: Vec<(Point, PathEl)> = Vec::with_capacity(els.len());
    let mut at = p0;
    for el in &els[1..] {
        match *el {
            PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => {
                pieces.push((at, *el));
                at = p;
            }
            PathEl::ClosePath => {
                if at != p0 {
                    pieces.push((at, PathEl::LineTo(p0)));
                    at = p0;
                }
            }
            PathEl::MoveTo(_) => {}
        }
    }
    if closed {
        path.move_to(at);
    }
    for (start, el) in pieces.into_iter().rev() {
        match el {
            PathEl::LineTo(_) => path.line_to(start),
            PathEl::QuadTo(c, _) => path.quad_to(c, start),
            PathEl::CurveTo(c1, c2, _) => path.curve_to(c2, c1, start),
            _ => {}
        }
    }
    if closed {
        path.close_path();
    }
}

/// A border's curve, from its start to its end (closed round a loop):
/// fitted as a ring is ([`fit_ring`], [`fit_open`]), or simplified to
/// straight lines along the pixel edges without smoothing.
fn fit_border(b: &Border, tolerance: f64, s: &Settings) -> BezPath {
    let mut pts = b.corners();
    if b.closed {
        pts.pop();
    }
    if s.smoothing <= 0.0 {
        let simplified = tracedraw_core::geometry::simplify(&pts, tolerance);
        if b.closed {
            // A loop that simplifies to a line is left out, on both sides.
            if simplified.len() < 3 {
                return BezPath::new();
            }
            return polyline_closed(&simplified);
        }
        let mut out = BezPath::new();
        if let Some(first) = simplified.first() {
            out.move_to(*first);
            for p in &simplified[1..] {
                out.line_to(*p);
            }
        }
        return out;
    }
    if b.closed {
        fit_ring(&pts, s)
    } else {
        fit_open(&pts, s)
    }
}

/// Call `f` with the pixels of each hole of `mask` (`w` x `h`): each
/// 4-connected patch of unset pixels not reaching the box's edge.
fn for_each_hole(mask: &[bool], w: usize, h: usize, mut f: impl FnMut(&[usize])) {
    if mask.len() != w * h || w < 3 || h < 3 {
        return;
    }
    let mut seen = vec![false; w * h];
    let mut stack: Vec<usize> = Vec::new();
    let mut patch: Vec<usize> = Vec::new();
    for start in 0..w * h {
        if seen[start] || mask[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        patch.clear();
        let mut edge = false;
        while let Some(i) = stack.pop() {
            patch.push(i);
            let (x, y) = (i % w, i / w);
            edge |= x == 0 || y == 0 || x + 1 == w || y + 1 == h;
            let near = [
                (x > 0).then(|| i - 1),
                (x + 1 < w).then(|| i + 1),
                (y > 0).then(|| i - w),
                (y + 1 < h).then(|| i + w),
            ];
            for j in near.into_iter().flatten() {
                if !mask[j] && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        if !edge {
            f(&patch);
        }
    }
}

/// The order colours stack in, bottom first: a colour found in another's
/// holes (`inside[a][b]`: `b` in a hole of `a` with no page showing in
/// it) goes above that one, so the one around is drawn whole under it;
/// among the colours free to go next the darkest (`luma`) goes first,
/// and where colours sit in each other's holes (black eyes in a yellow
/// head inside a black outline) the darkest of them.
fn stacking_order(inside: &[Vec<bool>], luma: &[u32]) -> Vec<usize> {
    let n = luma.len();
    let mut below = vec![0usize; n];
    for (a, row) in inside.iter().enumerate() {
        for (b, v) in row.iter().enumerate() {
            if *v && a != b && b < n {
                below[b] += 1;
            }
        }
    }
    let mut placed = vec![false; n];
    let mut order = Vec::with_capacity(n);
    while order.len() < n {
        let free = (0..n)
            .filter(|i| !placed[*i] && below[*i] == 0)
            .min_by_key(|i| (luma[*i], *i));
        let Some(next) = free.or_else(|| {
            (0..n)
                .filter(|i| !placed[*i])
                .min_by_key(|i| (luma[*i], *i))
        }) else {
            break;
        };
        placed[next] = true;
        order.push(next);
        if let Some(row) = inside.get(next) {
            for (b, v) in row.iter().enumerate() {
                if *v && b != next && b < n && !placed[b] {
                    below[b] = below[b].saturating_sub(1);
                }
            }
        }
    }
    order
}

/// The 4-connected patches of a labelling: each pixel's patch number
/// (`u32::MAX` for transparent pixels), and each patch's label, size,
/// edge count (pixel edges against another label or a transparent pixel)
/// and the labels it touches, most first, each with its edge count and
/// how many of those are against `solid` pixels.
struct Patches {
    of: Vec<u32>,
    label: Vec<usize>,
    area: Vec<u32>,
    edges: Vec<u32>,
    touches: Vec<Vec<(usize, u32, u32)>>,
}

fn label_patches(
    labels: &[usize],
    solid: &[bool],
    w: usize,
    h: usize,
    cancel: Option<&Cancel>,
) -> Option<Patches> {
    let mut p = Patches {
        of: vec![u32::MAX; w * h],
        label: Vec::new(),
        area: Vec::new(),
        edges: Vec::new(),
        touches: Vec::new(),
    };
    let mut stack: Vec<usize> = Vec::new();
    for start in 0..w * h {
        if start % CANCEL_EVERY == 0 && cancelled(cancel) {
            return None;
        }
        if p.of[start] != u32::MAX || labels[start] == usize::MAX {
            continue;
        }
        let l = labels[start];
        let id = p.label.len() as u32;
        let (mut area, mut edges) = (0u32, 0u32);
        let mut touches: Vec<(usize, u32, u32)> = Vec::new();
        p.of[start] = id;
        stack.push(start);
        while let Some(i) = stack.pop() {
            area += 1;
            let (x, y) = (i % w, i / w);
            let near = [
                (x > 0).then(|| i - 1),
                (x + 1 < w).then(|| i + 1),
                (y > 0).then(|| i - w),
                (y + 1 < h).then(|| i + w),
            ];
            for j in near.into_iter().flatten() {
                let m = labels[j];
                if m == l {
                    if p.of[j] == u32::MAX {
                        p.of[j] = id;
                        stack.push(j);
                    }
                    continue;
                }
                edges += 1;
                if m != usize::MAX {
                    let f = solid[j] as u32;
                    match touches.iter_mut().find(|(k, _, _)| *k == m) {
                        Some((_, n, nf)) => {
                            *n += 1;
                            *nf += f;
                        }
                        None => touches.push((m, 1, f)),
                    }
                }
            }
        }
        touches.sort_by_key(|(k, n, _)| (u32::MAX - *n, *k));
        p.label.push(l);
        p.area.push(area);
        p.edges.push(edges);
        p.touches.push(touches);
    }
    Some(p)
}

/// How thick (twice the area over the edge count, pixels) a band along
/// the edge of two colours can be and still be their soft edge.
const FRINGE_WIDTH: f64 = 5.0;

/// Give the patches that are the soft edge of two colours to those two
/// colours, each pixel to the one its own colour is nearer: a band no
/// more than `FRINGE_WIDTH` thick, of a colour between the two it
/// touches most (at least `EDGE_CONTRAST` apart, the second on a tenth of
/// its edges or more), both of them solid areas where they touch it (at
/// least half their pixels there flat or beside a flat pixel of their
/// colour). A blurred or compressed edge makes such bands thicker than
/// [`settle_edges`] reaches, and they were slivers and bumps along the
/// edge (#611, #1250). The pieces of a thin letter in two shades, one
/// touching the other and the fill, stay: the other shade is no solid
/// area.
fn dissolve_fringes(
    labels: &mut [usize],
    w: usize,
    h: usize,
    palette: &[[u8; 3]],
    colors: &[[u8; 3]],
    cancel: Option<&Cancel>,
) -> Option<()> {
    if labels.len() != w * h || colors.len() != w * h || w < 3 || h < 3 {
        return Some(());
    }
    let flat = flat_pixels(labels, w, h);
    let solid: Vec<bool> = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let l = labels[i];
            flat[i]
                || (l != usize::MAX
                    && (y.saturating_sub(1)..=(y + 1).min(h - 1)).any(|v| {
                        (x.saturating_sub(1)..=(x + 1).min(w - 1))
                            .any(|u| flat[v * w + u] && labels[v * w + u] == l)
                    }))
        })
        .collect();
    let p = label_patches(labels, &solid, w, h, cancel)?;
    let mut to: Vec<Option<(usize, usize)>> = vec![None; p.label.len()];
    for (k, t) in to.iter_mut().enumerate() {
        let (area, edges) = (p.area[k] as f64, p.edges[k] as f64);
        let touch = &p.touches[k];
        if edges <= 0.0 || touch.len() < 2 || 2.0 * area / edges > FRINGE_WIDTH {
            continue;
        }
        let ((a, na, sa), (b, nb, sb)) = (touch[0], touch[1]);
        let (Some(&ca), Some(&cb), Some(&cl)) =
            (palette.get(a), palette.get(b), palette.get(p.label[k]))
        else {
            continue;
        };
        if (nb as f64) < 0.1 * edges
            || 2 * sa < na
            || 2 * sb < nb
            || distance2(ca, cb) < EDGE_CONTRAST * EDGE_CONTRAST
            || !between(cl, ca, cb)
        {
            continue;
        }
        *t = Some((a, b));
    }
    if cancelled(cancel) {
        return None;
    }
    for i in 0..w * h {
        let Some((a, b)) = to.get(p.of[i] as usize).copied().flatten() else {
            continue;
        };
        labels[i] = if nearer_first(colors[i], palette[a], palette[b]) {
            a
        } else {
            b
        };
    }
    Some(())
}

/// How many times [`smooth_labels`] goes over the picture.
const SMOOTH_PASSES: usize = 2;

/// How many of its 8 neighbours must have one other colour for a pixel
/// to take it.
const SMOOTH_MAJORITY: u32 = 5;

/// Each pixel at least `SMOOTH_MAJORITY` of whose 8 neighbours have one
/// other colour takes it, `SMOOTH_PASSES` times over, so the one-pixel
/// bumps and notches noise leaves along an edge go and the edge is
/// smooth before it is fitted (#611, #1250). Only a pixel whose own
/// neighbours are the three on one side of it changes (a bump on a
/// straight edge, a notch in one): corners, the pixels of a line (their
/// own on two sides) and its end stay, so squares keep their corners and
/// lines one pixel wide their pixels.
fn smooth_labels(labels: &mut [usize], w: usize, h: usize, cancel: Option<&Cancel>) -> Option<()> {
    if labels.len() != w * h || w < 3 || h < 3 {
        return Some(());
    }
    // The eight neighbours in order round the pixel.
    let ring: [isize; 8] = {
        let w = w as isize;
        [-w - 1, -w, -w + 1, 1, w + 1, w, w - 1, -1]
    };
    for _ in 0..SMOOTH_PASSES {
        let prev = labels.to_vec();
        let mut changed = false;
        for y in 1..h - 1 {
            if (y - 1) % 64 == 0 && cancelled(cancel) {
                return None;
            }
            for x in 1..w - 1 {
                let i = y * w + x;
                let own = prev[i];
                if own == usize::MAX {
                    continue;
                }
                let mut around = [usize::MAX; 8];
                let mut mine = 0u8;
                for (k, d) in ring.iter().enumerate() {
                    let m = prev[(i as isize + d) as usize];
                    around[k] = m;
                    if m == own {
                        mine |= 1 << k;
                    }
                }
                // Its own neighbours: three in a straight row on one side
                // (ring places 0-2, 2-4, 4-6 or 6-0), so the pixel is a
                // bump on a straight edge or a notch in it, not a corner
                // (its own round a corner of the ring), part of a line
                // (its own on two sides) or a line's end.
                if ![0b0000_0111u8, 0b0001_1100, 0b0111_0000, 0b1100_0001].contains(&mine) {
                    continue;
                }
                let mut best = (0u32, usize::MAX);
                for m in around {
                    if m == own || m == usize::MAX {
                        continue;
                    }
                    let c = around.iter().filter(|o| **o == m).count() as u32;
                    if c > best.0 || (c == best.0 && m < best.1) {
                        best = (c, m);
                    }
                }
                if best.0 >= SMOOTH_MAJORITY {
                    labels[i] = best.1;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    Some(())
}

/// How far apart (RGB distance) two colours must be for a thin band
/// between them to count as their edge's anti-aliasing.
const EDGE_CONTRAST: u32 = 64;

fn distance2(a: [u8; 3], b: [u8; 3]) -> u32 {
    (0..3)
        .map(|k| (a[k] as i32 - b[k] as i32).pow(2) as u32)
        .sum()
}

/// Whether `c` lies between `a` and `b`: near the segment joining them in
/// RGB, as the anti-aliased pixels of their edge do.
fn between(c: [u8; 3], a: [u8; 3], b: [u8; 3]) -> bool {
    let f = |v: [u8; 3]| [v[0] as f64, v[1] as f64, v[2] as f64];
    let (c, a, b) = (f(c), f(a), f(b));
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let len2 = ab.iter().map(|v| v * v).sum::<f64>();
    if len2 < 1.0 {
        return false;
    }
    let t = (ab.iter().zip(&ac).map(|(p, q)| p * q).sum::<f64>() / len2).clamp(0.0, 1.0);
    let d2 = (0..3).map(|k| (ac[k] - t * ab[k]).powi(2)).sum::<f64>();
    let slack = (0.08 * len2.sqrt()).max(12.0);
    d2 <= slack * slack
}

/// The farthest apart (RGB distance) two colours split from one are.
const SPLIT_REACH: f64 = 40.0;

/// An image is flat art when its `FLAT_ART_COLORS` commonest colours,
/// once joined, cover `FLAT_ART_SHARE` of it.
const FLAT_ART_COLORS: usize = 8;
const FLAT_ART_SHARE: f64 = 0.85;

/// Join the palette colours that are one colour split by its noise: two
/// colours nearer each other than their pixels lie, on average, from
/// them (and than `SPLIT_REACH`). Their pixels take the colour of the one
/// with more, weighted by count. The steps of a gradient, and distinct
/// colours however close, lie further apart than their own spread and
/// stay, and photos are left as they are. Returns the palette with each
/// joined colour at their mean.
fn merge_split_colors(
    labels: &mut [usize],
    mut palette: Vec<[u8; 3]>,
    colors: &[[u8; 3]],
) -> Vec<[u8; 3]> {
    let n = palette.len();
    if labels.len() != colors.len() || n < 2 {
        return palette;
    }
    let mut count = vec![0f64; n];
    let mut sum = vec![[0f64; 3]; n];
    let mut spread = vec![0f64; n];
    for (l, c) in labels.iter().zip(colors) {
        if let (Some(k), Some(p)) = (count.get_mut(*l), palette.get(*l)) {
            *k += 1.0;
            for d in 0..3 {
                sum[*l][d] += c[d] as f64;
            }
            spread[*l] += distance2(*c, *p) as f64;
        }
    }
    for l in 0..n {
        if count[l] > 0.0 {
            spread[l] = (spread[l] / count[l]).sqrt();
        }
    }
    // Union-find over the colours, joining the nearest pairs first.
    let mut root: Vec<usize> = (0..n).collect();
    fn find(root: &mut [usize], mut i: usize) -> usize {
        while root[i] != i {
            root[i] = root[root[i]];
            i = root[i];
        }
        i
    }
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for a in 0..n {
        for b in a + 1..n {
            if count[a] == 0.0 || count[b] == 0.0 {
                continue;
            }
            let d = (distance2(palette[a], palette[b]) as f64).sqrt();
            if d < SPLIT_REACH && d < spread[a] + spread[b] {
                pairs.push((d, a, b));
            }
        }
    }
    if pairs.is_empty() {
        return palette;
    }
    pairs.sort_by(|p, q| p.0.total_cmp(&q.0));
    // Each group's pixel count and colour sum; two groups join only when
    // their mean colours are near too, so in-between colours along an
    // edge do not chain two distinct colours into one.
    let mut total: Vec<(f64, [f64; 3])> = (0..n).map(|l| (count[l], sum[l])).collect();
    let mean = |t: &(f64, [f64; 3])| -> [f64; 3] { std::array::from_fn(|d| t.1[d] / t.0) };
    for (_, a, b) in pairs {
        let (ra, rb) = (find(&mut root, a), find(&mut root, b));
        if ra == rb {
            continue;
        }
        let (ma, mb) = (mean(&total[ra]), mean(&total[rb]));
        let d2: f64 = (0..3).map(|d| (ma[d] - mb[d]).powi(2)).sum();
        if d2 >= SPLIT_REACH * SPLIT_REACH {
            continue;
        }
        root[rb] = ra;
        let t = total[rb];
        total[ra].0 += t.0;
        for d in 0..3 {
            total[ra].1[d] += t.1[d];
        }
    }
    // Flat art only: once joined, a few colours cover nearly all of it.
    // In a photo neighbouring colours of a gradient lie as near as their
    // spread too, and are left apart.
    let mut groups: Vec<f64> = (0..n)
        .filter(|&r| root[r] == r)
        .map(|r| total[r].0)
        .collect();
    groups.sort_by(|a, b| b.total_cmp(a));
    let all: f64 = groups.iter().sum();
    let few: f64 = groups.iter().take(FLAT_ART_COLORS).sum();
    if all <= 0.0 || few < FLAT_ART_SHARE * all {
        return palette;
    }
    for (r, (k, s)) in total.iter().enumerate() {
        if *k > 0.0 && root[r] == r {
            palette[r] = std::array::from_fn(|d| (s[d] / k).round().clamp(0.0, 255.0) as u8);
        }
    }
    for l in labels.iter_mut() {
        if *l < n {
            *l = find(&mut root, *l);
        }
    }
    palette
}

/// How near (RGB distance) two colours of the palette must be to be one
/// colour: none from detail 95 up, 30 at the Clipart detail of 60, 44 at
/// 35.
fn near_reach(detail: f64) -> f64 {
    if detail >= 95.0 {
        0.0
    } else {
        10.0 + (100.0 - detail.clamp(0.0, 100.0)) * 0.5
    }
}

/// `palette` without the colours no label uses, the labels renumbered to
/// match (transparent pixels stay `usize::MAX`).
fn compact_palette(labels: &mut [usize], palette: Vec<[u8; 3]>) -> Vec<[u8; 3]> {
    let mut used = vec![false; palette.len()];
    for l in labels.iter() {
        if let Some(u) = used.get_mut(*l) {
            *u = true;
        }
    }
    if used.iter().all(|u| *u) {
        return palette;
    }
    let mut to = vec![usize::MAX; palette.len()];
    let mut kept = Vec::new();
    for (i, c) in palette.into_iter().enumerate() {
        if used[i] {
            to[i] = kept.len();
            kept.push(c);
        }
    }
    for l in labels.iter_mut() {
        if let Some(t) = to.get(*l) {
            *l = *t;
        }
    }
    kept
}

/// How many pixel edges two shades must share, per pixel of the smaller,
/// to be one colour's noise.
const INTERLEAVED: f64 = 0.6;

/// Whether each of the `n` labels is a solid colour: at least half its
/// pixels flat (their eight neighbours of the label too), as the
/// colours of the art are and the shades of its edges and noise are not.
fn solid_colors(labels: &[usize], w: usize, h: usize, n: usize) -> Vec<bool> {
    let mut count = vec![(0u32, 0u32); n];
    if labels.len() == w * h {
        for (l, f) in labels.iter().zip(flat_pixels(labels, w, h)) {
            if let Some(c) = count.get_mut(*l) {
                c.0 += 1;
                c.1 += f as u32;
            }
        }
    }
    count
        .iter()
        .map(|(all, f)| *all > 0 && 2 * f >= *all)
        .collect()
}

/// Join the palette colours nearer each other than `reach` (RGB
/// distance) of which one at least is no solid colour, nearest pairs
/// first; two groups join only while the means
/// of their pixels stay within `reach`, so a run of steps (a gradient,
/// the in-between colours of an edge) does not chain far apart colours
/// into one. The labels of a joined colour take the group's, which keeps
/// the colour of its solid member.
fn merge_near_colors(
    labels: &mut [usize],
    w: usize,
    h: usize,
    palette: Vec<[u8; 3]>,
    colors: &[[u8; 3]],
    reach: f64,
) -> Vec<[u8; 3]> {
    let n = palette.len();
    if labels.len() != colors.len() || n < 2 || reach <= 0.0 {
        return palette;
    }
    let mut total = vec![(0f64, [0f64; 3]); n];
    for (l, c) in labels.iter().zip(colors) {
        if let Some(t) = total.get_mut(*l) {
            t.0 += 1.0;
            for (sum, v) in t.1.iter_mut().zip(c) {
                *sum += *v as f64;
            }
        }
    }
    let solid = solid_colors(labels, w, h, n);
    // How many pixel edges each two labels share.
    let mut shared = vec![0u32; n * n];
    for y in 0..h {
        for x in 0..w {
            let a = labels[y * w + x];
            for b in [
                (x + 1 < w).then(|| labels[y * w + x + 1]),
                (y + 1 < h).then(|| labels[(y + 1) * w + x]),
            ]
            .into_iter()
            .flatten()
            {
                if a != b && a < n && b < n {
                    shared[a * n + b] += 1;
                    shared[b * n + a] += 1;
                }
            }
        }
    }
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for a in 0..n {
        for b in a + 1..n {
            let d = (distance2(palette[a], palette[b]) as f64).sqrt();
            // A shade that lines the edges of a solid colour joins it,
            // and so do shades scattered through each other (a flat
            // colour's noise split in several, their pixels side by side
            // all over: more shared edges than the smaller has pixels).
            // Two solid colours, however close, are both meant (a 246
            // grey logo on white), and the steps of a gradient, bands
            // side by side, stay apart.
            let fewer = total[a].0.min(total[b].0);
            let mixed = fewer > 0.0 && shared[a * n + b] as f64 >= INTERLEAVED * fewer;
            if d < reach && (solid[a] != solid[b] || mixed) && fewer > 0.0 {
                pairs.push((d, a, b));
            }
        }
    }
    if pairs.is_empty() {
        return palette;
    }
    pairs.sort_by(|p, q| p.0.total_cmp(&q.0));
    let mut root: Vec<usize> = (0..n).collect();
    fn find(root: &mut [usize], mut i: usize) -> usize {
        while root[i] != i {
            root[i] = root[root[i]];
            i = root[i];
        }
        i
    }
    let mean = |t: &(f64, [f64; 3])| -> [f64; 3] { std::array::from_fn(|d| t.1[d] / t.0) };
    let mut has_solid = solid;
    for (_, a, b) in pairs {
        let (ra, rb) = (find(&mut root, a), find(&mut root, b));
        if ra == rb {
            continue;
        }
        let (ma, mb) = (mean(&total[ra]), mean(&total[rb]));
        let d2: f64 = (0..3).map(|d| (ma[d] - mb[d]).powi(2)).sum();
        // A group holds one solid colour at most, and stays its colour.
        if d2 >= reach * reach || (has_solid[ra] && has_solid[rb]) {
            continue;
        }
        let (keep, gone) = if has_solid[ra] || (!has_solid[rb] && total[ra].0 >= total[rb].0) {
            (ra, rb)
        } else {
            (rb, ra)
        };
        has_solid[keep] |= has_solid[gone];
        root[gone] = keep;
        let t = total[gone];
        total[keep].0 += t.0;
        for d in 0..3 {
            total[keep].1[d] += t.1[d];
        }
    }
    // Each group keeps the colour of its solid member (the root): a mean
    // drew a black joined with its dark brown fringe brown.
    for l in labels.iter_mut() {
        if *l < n {
            *l = find(&mut root, *l);
        }
    }
    palette
}

/// How near (RGB distance) an edge pixel's colour must be to the one
/// flat colour beside it to join it.
const EDGE_JOIN: u32 = 48;

/// How far, in pixels, an edge pixel looks for flat colours: across a
/// band of three in-between pixels (a blurred or compressed edge) to the
/// flat pixel past the next one.
const EDGE_REACH: usize = 4;

/// Give the pixels along the edges between flat colours (anti-aliasing,
/// the fringes of compressed images) to those colours, so the edge runs
/// where the art's did instead of along a ragged mix of in-between
/// colours. A pixel is flat when its eight neighbours share its colour;
/// a colour is a colour of its own when at least half its pixels are
/// flat, else it is only the fringe of edges (#1250). A pixel that is not
/// flat, or is of a fringe colour, with no flat pixel of its own colour
/// of its own within `EDGE_REACH`, looks at the flat pixels of colours of
/// their own there: between two of them it takes the one its colour is
/// nearer; beside one alone, it takes that one when its colour is near
/// it. Lines of their own colour (text, hairlines) keep it.
fn settle_edges(
    labels: &mut [usize],
    w: usize,
    h: usize,
    palette: &[[u8; 3]],
    colors: &[[u8; 3]],
    cancel: Option<&Cancel>,
) {
    if labels.len() != w * h || colors.len() != w * h || w < 3 || h < 3 {
        return;
    }
    let flat = flat_pixels(labels, w, h);
    // Colours of their own: half their pixels flat or more. The others
    // only line edges (the fringes of a blurred or compressed edge) and
    // take the colours of their own beside them, flat pixels and all.
    let mut count = vec![(0u32, 0u32); palette.len()];
    for (l, f) in labels.iter().zip(&flat) {
        if let Some(c) = count.get_mut(*l) {
            c.0 += 1;
            c.1 += *f as u32;
        }
    }
    let real: Vec<bool> = count.iter().map(|(n, f)| 2 * f >= *n).collect();
    let mut changes: Vec<(usize, usize)> = Vec::new();
    let mut near: Vec<usize> = Vec::new();
    for i in 0..w * h {
        if i % CANCEL_EVERY == 0 && cancelled(cancel) {
            return;
        }
        let own = labels[i];
        if own == usize::MAX || (flat[i] && real.get(own).copied().unwrap_or(true)) {
            continue;
        }
        let (x, y) = (i % w, i / w);
        near.clear();
        for v in y.saturating_sub(EDGE_REACH)..=(y + EDGE_REACH).min(h - 1) {
            for u in x.saturating_sub(EDGE_REACH)..=(x + EDGE_REACH).min(w - 1) {
                let j = v * w + u;
                if flat[j] && real[labels[j]] && !near.contains(&labels[j]) {
                    near.push(labels[j]);
                }
            }
        }
        if near.is_empty() || near.contains(&own) {
            continue;
        }
        let c = colors[i];
        let mut by_distance: Vec<(u32, usize)> = near
            .iter()
            .filter_map(|l| palette.get(*l).map(|p| (distance2(c, *p), *l)))
            .collect();
        by_distance.sort_unstable();
        let to = match by_distance[..] {
            [(da, a), (_, b), ..] => {
                let (pa, pb) = (palette[a], palette[b]);
                if between(c, pa, pb) {
                    Some(if nearer_first(c, pa, pb) { a } else { b })
                } else {
                    (da < EDGE_JOIN * EDGE_JOIN).then_some(a)
                }
            }
            [(da, a)] => (da < EDGE_JOIN * EDGE_JOIN).then_some(a),
            [] => None,
        };
        if let Some(to) = to {
            changes.push((i, to));
        }
    }
    for (i, to) in changes {
        labels[i] = to;
    }
}

/// Whether each pixel is flat: its eight neighbours share its colour.
fn flat_pixels(labels: &[usize], w: usize, h: usize) -> Vec<bool> {
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let l = labels[i];
            l != usize::MAX
                && x > 0
                && y > 0
                && x + 1 < w
                && y + 1 < h
                && (y - 1..=y + 1).all(|v| (x - 1..=x + 1).all(|u| labels[v * w + u] == l))
        })
        .collect()
}

/// Whether `c` lies nearer `a` than `b` along the way from one to the
/// other (its projection on the segment before the halfway mark).
fn nearer_first(c: [u8; 3], a: [u8; 3], b: [u8; 3]) -> bool {
    let ab: [i32; 3] = std::array::from_fn(|k| b[k] as i32 - a[k] as i32);
    let ac: [i32; 3] = std::array::from_fn(|k| c[k] as i32 - a[k] as i32);
    let dot: i32 = (0..3).map(|k| ab[k] * ac[k]).sum();
    let len2: i32 = (0..3).map(|k| ab[k] * ab[k]).sum();
    2 * dot < len2
}

/// `img` with the noise of photos and compressed images taken out (#611):
/// each colour channel of each pixel becomes the middle value of the
/// pixel and its four edge neighbours (a median over a plus-shaped
/// window). Isolated odd pixels take the value around them, while
/// straight edges, square corners and lines one pixel wide along the rows
/// or columns keep their place (a line's end pixels go), unlike a blur
/// that softens every edge.
/// Alpha is left as it is, and transparent pixels take no part.
/// `None` once `cancel` is set (looked at every few rows).
fn denoise(img: &RgbaImage, cancel: Option<&Cancel>) -> Option<RgbaImage> {
    let (w, h) = (img.width(), img.height());
    let mut out = img.clone();
    if w < 3 || h < 3 {
        return Some(out);
    }
    for y in 0..h {
        if y % 32 == 0 && cancelled(cancel) {
            return None;
        }
        for x in 0..w {
            let at = |dx: i64, dy: i64| {
                let px = (x as i64 + dx).clamp(0, w as i64 - 1) as u32;
                let py = (y as i64 + dy).clamp(0, h as i64 - 1) as u32;
                *img.get_pixel(px, py)
            };
            let mut p = *img.get_pixel(x, y);
            if p[3] < 128 {
                continue;
            }
            // Opaque pixels only: the colour under a transparent one
            // means nothing.
            let mut window = [p; 5];
            let mut n = 1;
            for q in [at(-1, 0), at(1, 0), at(0, -1), at(0, 1)] {
                if q[3] >= 128 {
                    window[n] = q;
                    n += 1;
                }
            }
            for k in 0..3 {
                let mut v = [0u8; 5];
                for (slot, q) in v.iter_mut().zip(&window[..n]) {
                    *slot = q[k];
                }
                let v = &mut v[..n];
                v.sort_unstable();
                p[k] = v[n / 2];
            }
            out.put_pixel(x, y, p);
        }
    }
    Some(out)
}

/// How many times noise is taken out: none at the highest detail (where
/// specks stay too), once at high detail, twice below.
fn denoise_passes(detail: f64) -> usize {
    if speck_area(detail) <= 1 {
        0
    } else if detail >= 75.0 {
        1
    } else {
        2
    }
}

/// The smallest patch of one colour that is kept, in pixels: none
/// below detail 100, a few pixels at 80, about 40 at 35.
fn speck_area(detail: f64) -> usize {
    let k = (100.0 - detail.clamp(0.0, 100.0)) / 100.0 * 10.0;
    (k * k).round().max(1.0) as usize
}

/// The smallest dot of the art kept however small the detail asks
/// patches to be: a quarter of [`speck_area`], at least 2 pixels (4 at
/// the Clipart detail of 60), so the dots of a halftone and the ends of
/// fine hatching stay.
fn dot_area(detail: f64) -> usize {
    (speck_area(detail) / 4).max(2)
}

/// How far (RGB distance) a small patch's colour must be from every
/// colour around it to be a dot of the art rather than noise.
const DOT_CONTRAST: u32 = 64;

/// Give every 4-connected patch of one label smaller than `min_area`
/// pixels the label of the neighbours outside it whose colour is
/// nearest its own, when one is within `SPLIT_REACH`, else the label most
/// of them have (transparent pixels are left alone and never spread). A
/// piece of a dark letter split into several dark shades joins the other
/// shades and stays a letter, where the colour around it would erase it
/// (#611). Patches joined into others are looked at again, until none
/// is left.
fn despeckle(
    labels: &mut [usize],
    w: usize,
    h: usize,
    min_area: usize,
    keep_area: usize,
    palette: &[[u8; 3]],
    cancel: Option<&Cancel>,
) {
    if min_area <= 1 || labels.len() != w * h {
        return;
    }
    for _ in 0..4 {
        if !despeckle_pass(labels, w, h, min_area, keep_area, palette, cancel) || cancelled(cancel)
        {
            break;
        }
    }
}

/// One pass of `despeckle`; whether it changed anything.
fn despeckle_pass(
    labels: &mut [usize],
    w: usize,
    h: usize,
    min_area: usize,
    keep_area: usize,
    palette: &[[u8; 3]],
    cancel: Option<&Cancel>,
) -> bool {
    let mut changed = false;
    let mut seen = vec![false; w * h];
    let mut patch: Vec<usize> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut counts: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for start in 0..w * h {
        if start % CANCEL_EVERY == 0 && cancelled(cancel) {
            return false;
        }
        if seen[start] || labels[start] == usize::MAX {
            continue;
        }
        let l = labels[start];
        patch.clear();
        counts.clear();
        stack.push(start);
        seen[start] = true;
        while let Some(i) = stack.pop() {
            patch.push(i);
            let (x, y) = (i % w, i / w);
            let mut near = [usize::MAX; 4];
            if x > 0 {
                near[0] = i - 1;
            }
            if x + 1 < w {
                near[1] = i + 1;
            }
            if y > 0 {
                near[2] = i - w;
            }
            if y + 1 < h {
                near[3] = i + w;
            }
            for j in near.into_iter().filter(|j| *j != usize::MAX) {
                if labels[j] == l {
                    if !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                } else if labels[j] != usize::MAX && patch.len() < min_area {
                    *counts.entry(labels[j]).or_default() += 1;
                }
            }
        }
        if patch.len() < min_area {
            let own = palette.get(l).copied();
            let far = |k: usize| match (own, palette.get(k)) {
                (Some(a), Some(b)) => distance2(a, *b),
                _ => u32::MAX,
            };
            let reach = (SPLIT_REACH * SPLIT_REACH) as u32;
            let near = counts
                .iter()
                .filter(|(k, _)| far(**k) < reach)
                .min_by_key(|(k, c)| (far(**k), usize::MAX - **c, **k));
            let most = || counts.iter().max_by_key(|(k, c)| (**c, usize::MAX - **k));
            // Between two neighbours (the anti-aliased part of a thin
            // dark line on a light fill): the one it is nearer.
            let side = || {
                let own = own?;
                let mut by: Vec<(&usize, &usize)> = counts.iter().collect();
                by.sort_by_key(|(k, c)| (usize::MAX - **c, **k));
                by.iter()
                    .enumerate()
                    .flat_map(|(n, a)| by[n + 1..].iter().map(move |b| (*a, *b)))
                    .find_map(|((a, _), (b, _))| {
                        let (pa, pb) = (*palette.get(*a)?, *palette.get(*b)?);
                        (distance2(pa, pb) >= EDGE_CONTRAST * EDGE_CONTRAST && between(own, pa, pb))
                            .then_some(if nearer_first(own, pa, pb) { a } else { b })
                    })
                    .map(|k| (k, k))
            };
            // A dot of the art (a halftone dot, the end of a hatch line):
            // no neighbour near its colour, standing out from every one of
            // them, big enough to be drawn and two pixels across both
            // ways, keeps its colour. Noise
            // differs little from what is around it and went to the
            // nearest colour above; the soft edge of two colours lies
            // between them and went to one of them.
            let stands_out = || {
                let own = own?;
                // Two pixels across at least, both ways: a column of one
                // (a colour bled sideways by compression) is no dot.
                let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
                for &i in &patch {
                    let (x, y) = (i % w, i / w);
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
                }
                (patch.len() >= keep_area
                    && x1 > x0
                    && y1 > y0
                    && counts.keys().all(|k| {
                        palette
                            .get(*k)
                            .is_some_and(|p| distance2(own, *p) >= DOT_CONTRAST * DOT_CONTRAST)
                    }))
                .then_some(())
            };
            if near.is_none() && side().is_none() && stands_out().is_some() {
                continue;
            }
            if let Some((&to, _)) = near.or_else(side).or_else(most) {
                for &i in &patch {
                    labels[i] = to;
                }
                changed = true;
            }
        }
    }
    changed
}

/// A pixel-edge ring as smooth curves (#611): the ring through the
/// middles of the pixel edges, so stair steps become slopes; its corners
/// found where it turns sharply (fewer with more corner smoothness);
/// the stretches between them smoothed (more with more smoothing) and
/// fitted with as few cubic Beziers as stay within the detail's
/// tolerance of them. Corners stay sharp, a ring without corners closes
/// smoothly.
fn fit_ring(ring: &[Point], s: &Settings) -> BezPath {
    // One point per pixel edge: the middle of each unit step, with the
    // pixel corner the step starts at.
    let mut pts: Vec<Point> = Vec::new();
    let mut starts: Vec<Point> = Vec::new();
    let n = ring.len();
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let steps = (b - a).hypot().round().max(1.0) as usize;
        for k in 0..steps {
            starts.push(a.lerp(b, k as f64 / steps as f64));
            pts.push(a.lerp(b, (k as f64 + 0.5) / steps as f64));
        }
    }
    let n = pts.len();
    if n < 4 {
        return polyline_closed(&pts);
    }
    // Corners: where the ring turns by more than the threshold over a
    // few pixels, the sharpest of each run, and still turns that much
    // seen from twice as far: a notch of a pixel or two along a smooth
    // edge (a pixel that fell to the other colour) turns sharply only up
    // close, and became a kink in the curve (#611).
    let reach = 3usize.min(n / 4).max(1);
    let wide = (2 * reach).min(n / 4).max(reach);
    let threshold = (60.0 + s.corner_smoothness.clamp(0.0, 100.0) * 0.8).to_radians();
    let at = |i: isize| pts[i.rem_euclid(n as isize) as usize];
    let turn_over = |i: isize, r: usize| {
        let (a, b, c) = (at(i - r as isize), at(i), at(i + r as isize));
        let (u, v) = (b - a, c - b);
        u.cross(v).atan2(u.dot(v)).abs()
    };
    let turn: Vec<f64> = (0..n as isize).map(|i| turn_over(i, reach)).collect();
    let mut corners: Vec<usize> = (0..n)
        .filter(|&i| {
            turn[i] > threshold
                && turn_over(i as isize, wide) > threshold
                && (1..=reach as isize).all(|d| {
                    let before = turn[(i as isize - d).rem_euclid(n as isize) as usize];
                    let after = turn[(i as isize + d).rem_euclid(n as isize) as usize];
                    turn[i] >= before && turn[i] > after
                })
        })
        .collect();
    // A corner sits on the pixel corner at one end of its edge: the one
    // standing further out from its neighbours, so a square stays square.
    let mut is_corner = vec![false; n];
    for &c in &corners {
        is_corner[c] = true;
        let mid = pts[(c + n - 1) % n].midpoint(pts[(c + 1) % n]);
        let (a, b) = (starts[c], starts[(c + 1) % n]);
        pts[c] = if (a - mid).hypot() > (b - mid).hypot() {
            a
        } else {
            b
        };
    }
    // Smooth the stretches between corners.
    let passes = (1.0 + s.smoothing.clamp(0.0, 100.0) / 100.0 * 12.0).round() as usize;
    for _ in 0..passes {
        let prev = pts.clone();
        for i in 0..n {
            if is_corner[i] {
                continue;
            }
            let a = prev[(i + n - 1) % n];
            let c = prev[(i + 1) % n];
            pts[i] = Point::new(
                (a.x + 2.0 * prev[i].x + c.x) / 4.0,
                (a.y + 2.0 * prev[i].y + c.y) / 4.0,
            );
        }
    }
    // Within a pixel at the default detail: the smoothed points lie on a
    // smooth curve already, and a tighter fit only adds nodes (#611).
    let tolerance = 0.2 + (100.0 - s.detail.clamp(0.0, 100.0)) / 100.0 * 2.0;
    let mut out = BezPath::new();
    if corners.is_empty() {
        // No corner: quarters meeting smoothly (a cubic bends through a
        // quarter turn well, a half turn only roughly).
        let tangent = |i: usize| unit(pts[(i + reach) % n] - pts[(i + n - reach) % n]);
        let cuts: Vec<usize> = (0..4).map(|k| k * n / 4).collect();
        out.move_to(pts[0]);
        for k in 0..4 {
            let (a, b) = (cuts[k], cuts[(k + 1) % 4]);
            let mut piece: Vec<Point> = if b > a {
                pts[a..=b].to_vec()
            } else {
                pts[a..].to_vec()
            };
            if b <= a {
                piece.push(pts[0]);
            }
            fit_cubics(&piece, tangent(a), -tangent(b), tolerance, &mut out);
        }
        out.close_path();
        return out;
    }
    corners.sort_unstable();
    out.move_to(pts[corners[0]]);
    for k in 0..corners.len() {
        let a = corners[k];
        let b = corners[(k + 1) % corners.len()];
        let mut piece: Vec<Point> = Vec::new();
        let mut i = a;
        loop {
            piece.push(pts[i]);
            if i == b && piece.len() > 1 {
                break;
            }
            i = (i + 1) % n;
            if piece.len() > n + 1 {
                break;
            }
        }
        if piece.len() < 2 {
            continue;
        }
        // The directions the sides leave and reach the corners in, over a
        // few pixels so a stair step does not tilt them.
        let m = piece.len();
        let far = TANGENT_REACH.min(m - 1);
        let t1 = unit(piece[far] - piece[0]);
        let t2 = unit(piece[m - 1 - far] - piece[m - 1]);
        fit_cubics(&piece, t1, t2, tolerance, &mut out);
    }
    out.close_path();
    out
}

/// A border from one junction to another (`corners`, the pixel corners
/// it passes) as smooth curves, fitted as [`fit_ring`] fits a ring: the
/// middles of its pixel edges, its corners, the stretches between them
/// smoothed and fitted. Its two ends stay on the junctions, where the
/// borders of the other regions meeting there start and end too.
fn fit_open(corners: &[Point], s: &Settings) -> BezPath {
    let mut out = BezPath::new();
    let Some(&first) = corners.first() else {
        return out;
    };
    let last = corners[corners.len() - 1];
    // The ends, and one point in the middle of each pixel edge between.
    let mut pts: Vec<Point> = Vec::with_capacity(corners.len() + 1);
    pts.push(first);
    for e in corners.windows(2) {
        pts.push(e[0].midpoint(e[1]));
    }
    pts.push(last);
    let n = pts.len();
    out.move_to(first);
    if n < 5 {
        for p in &pts[1..] {
            out.line_to(*p);
        }
        return out;
    }
    let reach = 3usize.min(n / 4).max(1);
    let wide = (2 * reach).min(n / 4).max(reach);
    let threshold = (60.0 + s.corner_smoothness.clamp(0.0, 100.0) * 0.8).to_radians();
    let at = |i: isize| pts[i.clamp(0, n as isize - 1) as usize];
    let turn_over = |i: isize, r: usize| {
        let (a, b, c) = (at(i - r as isize), at(i), at(i + r as isize));
        let (u, v) = (b - a, c - b);
        if u.hypot2() < 1e-12 || v.hypot2() < 1e-12 {
            return 0.0;
        }
        u.cross(v).atan2(u.dot(v)).abs()
    };
    let turn: Vec<f64> = (0..n as isize).map(|i| turn_over(i, reach)).collect();
    let corners_at: Vec<usize> = (1..n - 1)
        .filter(|&i| {
            turn[i] > threshold
                && turn_over(i as isize, wide) > threshold
                && (1..=reach as isize).all(|d| {
                    let before = turn[(i as isize - d).clamp(0, n as isize - 1) as usize];
                    let after = turn[(i as isize + d).clamp(0, n as isize - 1) as usize];
                    turn[i] >= before && turn[i] > after
                })
        })
        .collect();
    // A corner sits on the pixel corner at one end of its edge (point i
    // is the middle of the edge from corner i - 1 to corner i), the one
    // standing further out from its neighbours.
    let mut fixed = vec![false; n];
    fixed[0] = true;
    fixed[n - 1] = true;
    for &c in &corners_at {
        fixed[c] = true;
        let mid = pts[c - 1].midpoint(pts[c + 1]);
        let (a, b) = (corners[c - 1], corners[c]);
        pts[c] = if (a - mid).hypot() > (b - mid).hypot() {
            a
        } else {
            b
        };
    }
    let passes = (1.0 + s.smoothing.clamp(0.0, 100.0) / 100.0 * 12.0).round() as usize;
    for _ in 0..passes {
        let prev = pts.clone();
        for i in 1..n - 1 {
            if fixed[i] {
                continue;
            }
            let (a, c) = (prev[i - 1], prev[i + 1]);
            pts[i] = Point::new(
                (a.x + 2.0 * prev[i].x + c.x) / 4.0,
                (a.y + 2.0 * prev[i].y + c.y) / 4.0,
            );
        }
    }
    let tolerance = 0.2 + (100.0 - s.detail.clamp(0.0, 100.0)) / 100.0 * 2.0;
    let mut anchors = vec![0];
    anchors.extend(corners_at);
    anchors.push(n - 1);
    for k in anchors.windows(2) {
        let piece = &pts[k[0]..=k[1]];
        let m = piece.len();
        let far = TANGENT_REACH.min(m - 1);
        let t1 = unit(piece[far] - piece[0]);
        let t2 = unit(piece[m - 1 - far] - piece[m - 1]);
        fit_cubics(piece, t1, t2, tolerance, &mut out);
    }
    out
}

/// How many points away a fitted curve's end direction is taken from.
const TANGENT_REACH: usize = 4;

fn unit(v: tracedraw_core::geometry::Vec2) -> tracedraw_core::geometry::Vec2 {
    let l = v.hypot();
    if l > 1e-12 && l.is_finite() {
        v / l
    } else {
        tracedraw_core::geometry::Vec2::ZERO
    }
}

/// Fit cubic Beziers to `pts` (from the first point to the last), the
/// first leaving along `t1` and the last arriving back along `t2` (unit
/// vectors pointing into the curve from its ends), no point further than
/// `tolerance` from them, and append them to `out`: least squares on the
/// chord-length parameters, improved by Newton steps, split at the worst
/// point when it still does not fit (P. J. Schneider, "An Algorithm for
/// Automatically Fitting Digitized Curves", Graphics Gems, 1990).
fn fit_cubics(
    pts: &[Point],
    t1: tracedraw_core::geometry::Vec2,
    t2: tracedraw_core::geometry::Vec2,
    tolerance: f64,
    out: &mut BezPath,
) {
    fit_cubics_depth(pts, t1, t2, tolerance, out, 0);
}

fn fit_cubics_depth(
    pts: &[Point],
    t1: tracedraw_core::geometry::Vec2,
    t2: tracedraw_core::geometry::Vec2,
    tolerance: f64,
    out: &mut BezPath,
    depth: usize,
) {
    use tracedraw_core::geometry::{CubicBez, ParamCurve, ParamCurveDeriv};
    let n = pts.len();
    if n < 2 {
        return;
    }
    let (p0, p3) = (pts[0], pts[n - 1]);
    let dist = (p3 - p0).hypot();
    if n == 2 || depth > 24 {
        let h = dist / 3.0;
        out.curve_to(p0 + t1 * h, p3 + t2 * h, p3);
        return;
    }
    // Chord-length parameters.
    let mut u = vec![0.0; n];
    for i in 1..n {
        u[i] = u[i - 1] + (pts[i] - pts[i - 1]).hypot();
    }
    let total = u[n - 1];
    if total < 1e-12 {
        return;
    }
    for v in u.iter_mut() {
        *v /= total;
    }
    let generate = |u: &[f64]| -> CubicBez {
        // The handle lengths minimising the squared distances.
        let (mut c00, mut c01, mut c11, mut x0, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for (p, &t) in pts.iter().zip(u) {
            let mt = 1.0 - t;
            let b0 = mt * mt * mt;
            let b1 = 3.0 * t * mt * mt;
            let b2 = 3.0 * t * t * mt;
            let b3 = t * t * t;
            let a1 = t1 * b1;
            let a2 = t2 * b2;
            c00 += a1.dot(a1);
            c01 += a1.dot(a2);
            c11 += a2.dot(a2);
            let tmp = p.to_vec2() - (p0.to_vec2() * (b0 + b1) + p3.to_vec2() * (b2 + b3));
            x0 += a1.dot(tmp);
            x1 += a2.dot(tmp);
        }
        let det = c00 * c11 - c01 * c01;
        let (mut l1, mut l2) = if det.abs() > 1e-12 {
            ((x0 * c11 - x1 * c01) / det, (c00 * x1 - c01 * x0) / det)
        } else {
            (0.0, 0.0)
        };
        let eps = 1e-6 * dist;
        if !(l1.is_finite() && l2.is_finite()) || l1 < eps || l2 < eps {
            l1 = dist / 3.0;
            l2 = dist / 3.0;
        }
        CubicBez::new(p0, p0 + t1 * l1, p3 + t2 * l2, p3)
    };
    let worst = |c: &CubicBez, u: &[f64]| -> (f64, usize) {
        let mut best = (0.0, n / 2);
        for i in 1..n - 1 {
            let d = (c.eval(u[i]) - pts[i]).hypot2();
            if d > best.0 {
                best = (d, i);
            }
        }
        best
    };
    let mut curve = generate(&u);
    let (mut err, mut split) = worst(&curve, &u);
    let tol2 = tolerance * tolerance;
    if err <= tol2 {
        out.curve_to(curve.p1, curve.p2, curve.p3);
        return;
    }
    if err < tol2 * 4.0 {
        for _ in 0..4 {
            // Newton steps towards each point's nearest parameter.
            let d1 = curve.deriv();
            let d2 = d1.deriv();
            for i in 0..n {
                let t = u[i];
                let diff = curve.eval(t) - pts[i];
                let q1 = d1.eval(t).to_vec2();
                let q2 = d2.eval(t).to_vec2();
                let den = q1.dot(q1) + diff.dot(q2);
                if den.abs() > 1e-12 {
                    u[i] = (t - diff.dot(q1) / den).clamp(0.0, 1.0);
                }
            }
            curve = generate(&u);
            (err, split) = worst(&curve, &u);
            if err <= tol2 {
                out.curve_to(curve.p1, curve.p2, curve.p3);
                return;
            }
        }
    }
    let split = split.clamp(1, n - 2);
    let reach = TANGENT_REACH.min(split).min(n - 1 - split).max(1);
    let centre = unit(pts[split - reach] - pts[split + reach]);
    let centre = if centre == tracedraw_core::geometry::Vec2::ZERO {
        unit(pts[split - 1] - pts[split])
    } else {
        centre
    };
    fit_cubics_depth(&pts[..=split], t1, centre, tolerance, out, depth + 1);
    fit_cubics_depth(&pts[split..], -centre, t2, tolerance, out, depth + 1);
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

/// Median cut colour quantisation to `n` colours, giving up (`None`)
/// once `cancel` is set.
pub fn median_cut(img: &RgbaImage, n: usize, cancel: Option<&Cancel>) -> Option<Vec<[u8; 3]>> {
    let pixels: Vec<[u8; 3]> = img
        .pixels()
        .filter(|p| p[3] >= 128)
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    median_cut_of(pixels, n, cancel)
}

/// How near (RGB distance) a pixel's eight neighbours must be to it for
/// it to lie inside a flat colour of the art.
const FLAT_NOISE: u32 = 24;

/// The share of the opaque pixels that must lie inside flat colours for
/// the palette to be taken from them alone.
const FLAT_SHARE: f64 = 0.3;

/// The palette of `img` in `n` colours. In art drawn in flat colours (at
/// least `FLAT_SHARE` of its pixels inside one), from the pixels inside
/// the flat colours only: the in-between colours of anti-aliased and
/// compressed edges, a pixel or two wide but many, took palette places
/// of their own and left too few for the art's real colours, which came
/// out split or mixed. Photos take every pixel.
pub fn art_palette(img: &RgbaImage, n: usize, cancel: Option<&Cancel>) -> Option<Vec<[u8; 3]>> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let raw = img.as_raw();
    let at = |i: usize| [raw[i * 4], raw[i * 4 + 1], raw[i * 4 + 2]];
    let opaque = |i: usize| raw[i * 4 + 3] >= 128;
    let mut flat: Vec<[u8; 3]> = Vec::new();
    let mut all = 0usize;
    for y in 0..h {
        if y % 64 == 0 && cancelled(cancel) {
            return None;
        }
        for x in 0..w {
            let i = y * w + x;
            if !opaque(i) {
                continue;
            }
            all += 1;
            if x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                continue;
            }
            let c = at(i);
            let inside = (y - 1..=y + 1).all(|v| {
                (x - 1..=x + 1).all(|u| {
                    let j = v * w + u;
                    opaque(j) && distance2(at(j), c) <= FLAT_NOISE * FLAT_NOISE
                })
            });
            if inside {
                flat.push(c);
            }
        }
    }
    let every = median_cut(img, n, cancel)?;
    if all == 0 || (flat.len() as f64) < FLAT_SHARE * all as f64 {
        return Some(every);
    }
    // Colours of the art too thin to be flat anywhere (a hairline of a
    // colour of its own) come from the palette of every pixel: those far
    // from every flat colour and not the blend of the two nearest them,
    // as an anti-aliased edge's pixels are. They take up to a quarter of
    // the places, the flat colours the rest: never more colours than
    // asked for.
    let base = median_cut_of(flat.clone(), n, cancel)?;
    let extra: Vec<[u8; 3]> = every
        .into_iter()
        .filter(|c| unexplained(*c, &base))
        .take(n / 4)
        .collect();
    if extra.is_empty() {
        return Some(base);
    }
    let mut palette = median_cut_of(flat, n - extra.len(), cancel)?;
    palette.extend(extra);
    Some(palette)
}

/// How far (RGB distance) a colour must be from every colour of a
/// palette for the palette not to stand for it.
const UNEXPLAINED: u32 = 56;

/// Whether `c` is far from every colour of `palette` and not between
/// the two nearest it.
fn unexplained(c: [u8; 3], palette: &[[u8; 3]]) -> bool {
    let mut best = [(u32::MAX, 0usize); 2];
    for (i, p) in palette.iter().enumerate() {
        let d = distance2(c, *p);
        if d < best[0].0 {
            best[1] = best[0];
            best[0] = (d, i);
        } else if d < best[1].0 {
            best[1] = (d, i);
        }
    }
    if best[0].0 <= UNEXPLAINED * UNEXPLAINED {
        return false;
    }
    match (palette.get(best[0].1), palette.get(best[1].1)) {
        (Some(a), Some(b)) if best[1].0 != u32::MAX => !between(c, *a, *b),
        _ => true,
    }
}

/// [`median_cut`] of a list of colours.
fn median_cut_of(
    mut pixels: Vec<[u8; 3]>,
    n: usize,
    cancel: Option<&Cancel>,
) -> Option<Vec<[u8; 3]>> {
    if pixels.is_empty() {
        return Some(vec![[0, 0, 0]]);
    }
    // Subsample large images.
    if pixels.len() > 200_000 {
        let step = pixels.len() / 200_000 + 1;
        pixels = pixels.into_iter().step_by(step).collect();
    }
    let mut buckets: Vec<Vec<[u8; 3]>> = vec![pixels.clone()];
    while buckets.len() < n {
        if cancelled(cancel) {
            return None;
        }
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
        // Cut at the median, but never through a run of equal values: a
        // picture of mostly black and white would otherwise put both in
        // one bucket and average them to grey.
        let v = b[b.len() / 2][ch];
        let mut mid = b.partition_point(|p| p[ch] < v);
        if mid == 0 {
            mid = b.partition_point(|p| p[ch] <= v);
        }
        let hi = b.split_off(mid);
        buckets.push(b);
        buckets.push(hi);
    }
    let start: Vec<[u8; 3]> = buckets
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
        .collect();
    refine(&pixels, start, cancel)
}

/// How near (RGB distance) the commonest exact colour of a palette
/// colour's pixels must be to take its place.
const SNAP_REACH: u32 = 16;

/// Each colour of the palette moved onto the commonest exact colour of
/// its pixels, when that is near: the mean of a flat colour's noisy
/// pixels, and of colours joined into one, was a shade off the colour the
/// art was drawn in (a black traced as #0B0703), and a trace should keep
/// the art's own colours. Every other pixel is counted, which
/// finds the commonest colour of any region worth a curve.
fn snap_to_commonest(labels: &[usize], colors: &[[u8; 3]], palette: &mut [[u8; 3]]) {
    use std::collections::HashMap;
    let mut counts: Vec<HashMap<[u8; 3], u32>> = vec![HashMap::new(); palette.len()];
    for (l, c) in labels.iter().zip(colors).step_by(2) {
        if let Some(m) = counts.get_mut(*l) {
            *m.entry(*c).or_default() += 1;
        }
    }
    for (p, m) in palette.iter_mut().zip(&counts) {
        if let Some((best, _)) = m.iter().max_by_key(|(k, n)| (**n, **k)) {
            if distance2(*best, *p) <= SNAP_REACH * SNAP_REACH {
                *p = *best;
            }
        }
    }
}

/// A few rounds of k-means from the median cut's colours: each colour
/// moves to the mean of the pixels nearest it, so the palette lands on
/// the colours the picture really has.
fn refine(
    pixels: &[[u8; 3]],
    mut palette: Vec<[u8; 3]>,
    cancel: Option<&Cancel>,
) -> Option<Vec<[u8; 3]>> {
    for _ in 0..6 {
        if cancelled(cancel) {
            return None;
        }
        let mut sums = vec![[0u64; 4]; palette.len()];
        for p in pixels {
            let i = nearest(&palette, *p);
            for k in 0..3 {
                sums[i][k] += p[k] as u64;
            }
            sums[i][3] += 1;
        }
        let mut moved = false;
        for (c, s) in palette.iter_mut().zip(&sums) {
            if s[3] == 0 {
                continue;
            }
            let m = [
                (s[0] / s[3]) as u8,
                (s[1] / s[3]) as u8,
                (s[2] / s[3]) as u8,
            ];
            moved |= m != *c;
            *c = m;
        }
        if !moved {
            break;
        }
    }
    palette.dedup();
    Some(palette)
}

// ----- centreline mode --------------------------------------------------------

fn centerline(img: &RgbaImage, s: &Settings, cancel: Option<&Cancel>) -> Option<Vec<Traced>> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w == 0 || h == 0 {
        return Some(Vec::new());
    }
    let mut mask: Vec<bool> = img
        .pixels()
        .map(|p| p[3] >= 128 && luma(*p) < 128.0)
        .collect();
    // Mean stroke thickness before thinning: area / skeleton length later.
    let area = mask.iter().filter(|m| **m).count() as f64;
    if !thin_unless(&mut mask, w, h, cancel) {
        return None;
    }
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
    if cancelled(cancel) {
        return None;
    }
    let tolerance = 0.5 + (100.0 - s.detail) / 100.0 * 2.0;
    let mut out = Vec::new();
    let c = s.shown_color([0, 0, 0]);
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
            color: Color::rgb8(c[0], c[1], c[2]),
            closed: false,
        });
    }
    // Encode the stroke width in the first result's colour alpha? No: the
    // caller reads `stroke_width` through the returned tuple below.
    STROKE_WIDTH.with(|c| c.set(width));
    Some(out)
}

thread_local! {
    static STROKE_WIDTH: std::cell::Cell<f64> = const { std::cell::Cell::new(1.0) };
}

/// Mean stroke width (pixels) measured by the last centreline trace.
pub fn last_stroke_width_px() -> f64 {
    STROKE_WIDTH.with(|c| c.get())
}

/// Zhang-Suen thinning, stopping between passes once `cancel` is set;
/// whether it finished.
fn thin_unless(mask: &mut [bool], w: usize, h: usize, cancel: Option<&Cancel>) -> bool {
    let at = |m: &[bool], x: i64, y: i64| -> bool {
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && m[y as usize * w + x as usize]
    };
    loop {
        if cancelled(cancel) {
            return false;
        }
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
            return true;
        }
    }
}

/// Draw traced shapes (pixel space, y down) into a transparent image of
/// `w` x `h`: closed shapes filled with their colour, centrelines stroked
/// `stroke_px` wide ([`optimize`] and the tests draw traces back this
/// way; the dialog's preview is [`render_traced_scaled`]).
pub fn render_traced(traced: &[Traced], w: u32, h: u32, stroke_px: f64) -> RgbaImage {
    paint_traced(traced, w, h, 1.0, Paint::Filled(stroke_px))
}

/// [`render_traced`] into an image `scale` times the trace's pixel space
/// (`w` x `h` is the trace's size): the dialog's preview of a trace made
/// on the whole picture.
pub fn render_traced_scaled(
    traced: &[Traced],
    w: u32,
    h: u32,
    scale: f32,
    stroke_px: f64,
) -> RgbaImage {
    paint_traced(traced, w, h, scale, Paint::Filled(stroke_px))
}

/// The curves of `traced` as thin lines of one colour on a transparent
/// image `scale` times the trace's `w` x `h`: the dialog's wireframe,
/// laid over the picture.
pub fn render_wireframe(traced: &[Traced], w: u32, h: u32, scale: f32) -> RgbaImage {
    paint_traced(traced, w, h, scale, Paint::Wire)
}

enum Paint {
    /// Fills and centrelines (this wide, pixels) in their colours.
    Filled(f64),
    /// Every curve as a one point line in the wireframe colour.
    Wire,
}

/// The colour of the wireframe's lines.
const WIRE: [u8; 4] = [0x1E, 0x5A, 0xC8, 0xFF];

fn paint_traced(traced: &[Traced], w: u32, h: u32, scale: f32, how: Paint) -> RgbaImage {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let (pw, ph) = (
        ((w as f32 * scale).round() as u32).max(1),
        ((h as f32 * scale).round() as u32).max(1),
    );
    let Some(mut pm) = tiny_skia::Pixmap::new(pw, ph) else {
        return RgbaImage::new(pw, ph);
    };
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    for t in traced {
        let mut pb = tiny_skia::PathBuilder::new();
        for el in t.path.elements() {
            match *el {
                tracedraw_core::geometry::PathEl::MoveTo(p) => pb.move_to(p.x as f32, p.y as f32),
                tracedraw_core::geometry::PathEl::LineTo(p) => pb.line_to(p.x as f32, p.y as f32),
                tracedraw_core::geometry::PathEl::QuadTo(a, p) => {
                    pb.quad_to(a.x as f32, a.y as f32, p.x as f32, p.y as f32)
                }
                tracedraw_core::geometry::PathEl::CurveTo(a, b, p) => pb.cubic_to(
                    a.x as f32, a.y as f32, b.x as f32, b.y as f32, p.x as f32, p.y as f32,
                ),
                tracedraw_core::geometry::PathEl::ClosePath => pb.close(),
            }
        }
        let Some(path) = pb.finish() else {
            continue;
        };
        let [r, g, b] = t.color.to_rgb8();
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(r, g, b, 255);
        paint.anti_alias = true;
        match how {
            Paint::Wire => {
                paint.set_color_rgba8(WIRE[0], WIRE[1], WIRE[2], WIRE[3]);
                // About a pixel of the image, whatever the scale (the
                // curves run along pixel edges, so a line of exactly one
                // pixel would half cover two).
                let stroke = tiny_skia::Stroke {
                    width: 1.25 / scale,
                    ..Default::default()
                };
                pm.stroke_path(&path, &paint, &stroke, transform, None);
            }
            Paint::Filled(_) if t.closed => {
                pm.fill_path(&path, &paint, tiny_skia::FillRule::EvenOdd, transform, None);
            }
            Paint::Filled(stroke_px) => {
                let stroke = tiny_skia::Stroke {
                    width: stroke_px.max(0.5) as f32,
                    line_cap: tiny_skia::LineCap::Round,
                    line_join: tiny_skia::LineJoin::Round,
                    ..Default::default()
                };
                pm.stroke_path(&path, &paint, &stroke, transform, None);
            }
        }
    }
    let mut out = RgbaImage::new(pm.width(), pm.height());
    for (px, p) in out.pixels_mut().zip(pm.pixels()) {
        let c = p.demultiply();
        *px = image::Rgba([c.red(), c.green(), c.blue(), c.alpha()]);
    }
    out
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

    /// Flat art saved with noise, as compressed images are (#611): the
    /// yellow disc crossed by a black band, each pixel off by up to 10 in
    /// every channel. Traced with the Clipart preset, the noise makes
    /// neither holes showing the page inside the art nor a crowd of
    /// specks: before noise was taken out, about 3800 pixels inside the
    /// disc showed the page and the trace held over 1400 subpaths.
    #[test]
    fn noisy_flat_art_traces_without_holes_or_specks() {
        use tracedraw_core::geometry::PathEl;
        const N: u32 = 160;
        let mut seed = 12345u32;
        let mut noise = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            ((seed >> 16) % 21) as f32 - 10.0
        };
        let mut img = RgbaImage::new(N, N);
        for y in 0..N {
            for x in 0..N {
                let (mut yl, mut bl) = (0.0f32, 0.0f32);
                for sy in 0..4 {
                    for sx in 0..4 {
                        let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                        let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                        let disc = (px - 80.0).powi(2) + (py - 80.0).powi(2) < 60.0f32.powi(2);
                        let band = (py - 0.3 * px - 50.0).abs() < 14.0;
                        if disc && band {
                            bl += 1.0 / 16.0;
                        } else if disc {
                            yl += 1.0 / 16.0;
                        }
                    }
                }
                let wh = 1.0 - yl - bl;
                let n = noise();
                let mix =
                    |a: f32, b: f32, c: f32| (a * yl + b * bl + c * wh + n).clamp(0.0, 255.0) as u8;
                img.put_pixel(
                    x,
                    y,
                    Rgba([
                        mix(250.0, 10.0, 255.0),
                        mix(205.0, 10.0, 255.0),
                        mix(40.0, 10.0, 255.0),
                        255,
                    ]),
                );
            }
        }
        let t = trace(&img, &Preset::Clipart.settings());
        let subpaths: usize = t
            .iter()
            .map(|r| {
                r.path
                    .elements()
                    .iter()
                    .filter(|e| matches!(e, PathEl::MoveTo(_)))
                    .count()
            })
            .sum();
        assert!(subpaths < 1000, "{subpaths} subpaths");
        let out = render_traced(&t, N, N, 1.0);
        let (mut inside, mut holes) = (0, 0);
        for y in 0..N {
            for x in 0..N {
                if (x as f32 - 80.0).hypot(y as f32 - 80.0) >= 56.0 {
                    continue;
                }
                inside += 1;
                let p = out.get_pixel(x, y);
                // Where two colours meet along one curve, each covers part
                // of the pixels it crosses, and the page shows through at
                // most a quarter of one (both half covered): a gap lets
                // more through.
                if p[3] < 180 || (p[0] > 230 && p[1] > 230 && p[2] > 230) {
                    holes += 1;
                }
            }
        }
        assert!(
            holes * 200 < inside,
            "{holes} of {inside} pixels show the page"
        );
    }

    /// Taking noise out replaces a lone odd pixel by its surroundings and
    /// keeps square corners, straight edges and lines one pixel wide along
    /// a row (all but its end pixels); transparent pixels stay as they are.
    #[test]
    fn denoise_removes_lone_pixels_and_keeps_edges() {
        let white = Rgba([255, 255, 255, 255]);
        let black = Rgba([0, 0, 0, 255]);
        let img: RgbaImage = ImageBuffer::from_fn(20, 20, |x, y| {
            let square = (5..12).contains(&x) && (5..12).contains(&y);
            let line = y == 15 && (2..18).contains(&x);
            let lone = x == 16 && y == 4;
            if (x, y) == (0, 0) {
                Rgba([9, 9, 9, 0])
            } else if square || line || lone {
                black
            } else {
                white
            }
        });
        let out = denoise(&img, None).expect("not cancelled");
        assert_eq!(*out.get_pixel(16, 4), white, "the lone pixel goes");
        for (x, y) in [
            (5, 5),
            (11, 5),
            (5, 11),
            (11, 11),
            (8, 8),
            (3, 15),
            (10, 15),
            (16, 15),
        ] {
            assert_eq!(*out.get_pixel(x, y), black, "kept at {x},{y}");
        }
        for (x, y) in [(4, 5), (12, 11), (8, 14), (8, 16)] {
            assert_eq!(*out.get_pixel(x, y), white, "kept at {x},{y}");
        }
        assert_eq!(*out.get_pixel(0, 0), Rgba([9, 9, 9, 0]));
        // Nothing to do on a tiny image.
        let tiny = RgbaImage::new(2, 1);
        assert_eq!(denoise(&tiny, None), Some(tiny));
    }

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
    fn black_and_white_logos_keep_their_colours() {
        // A white disc on black with a black square inside and a soft
        // edge, mostly black pixels: the disc stays white, not grey.
        let img: RgbaImage = ImageBuffer::from_fn(120, 120, |x, y| {
            let d = ((x as f32 - 60.0).powi(2) + (y as f32 - 60.0).powi(2)).sqrt();
            let inner = (45..75).contains(&x) && (45..75).contains(&y);
            let v = if inner {
                0.0
            } else {
                (255.0 * (40.0 - d).clamp(0.0, 1.0)).round()
            };
            let v = v as u8;
            Rgba([v, v, v, 255])
        });
        for colors in [2, 8] {
            let mut s = Preset::Logo.settings();
            s.colors = colors;
            let t = trace(&img, &s);
            let biggest = t
                .iter()
                .max_by(|a, b| a.path.area().abs().total_cmp(&b.path.area().abs()))
                .expect("a region");
            let [r, g, b] = biggest.color.to_rgb8();
            assert!(
                r > 240 && g > 240 && b > 240,
                "{colors} colours: {r},{g},{b}"
            );
        }
        let pal = median_cut(&img, 2, None).expect("not cancelled");
        assert!(
            pal.contains(&[0, 0, 0]) && pal.iter().any(|c| c[0] > 245),
            "{pal:?}"
        );
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

    /// Smooth outlines from pixels (#611): a circle becomes a few curves,
    /// no stair steps, at the right size; a square keeps four sharp
    /// corners on its pixel corners; stray specks make no curves.
    #[test]
    fn traces_are_smooth_curves_with_sharp_corners_and_no_specks() {
        use tracedraw_core::geometry::PathEl;
        let s = Preset::LineArt.settings();
        let t = trace(&circle_image(), &s);
        assert_eq!(t.len(), 1);
        let els = t[0].path.elements();
        let curves = els
            .iter()
            .filter(|e| matches!(e, PathEl::CurveTo(..)))
            .count();
        assert!(
            els.iter().all(|e| !matches!(e, PathEl::LineTo(..))),
            "no steps"
        );
        assert!((2..=12).contains(&curves), "{curves} curves");
        let area = t[0].path.area().abs();
        let expect = std::f64::consts::PI * 30.0 * 30.0;
        assert!((area - expect).abs() / expect < 0.02, "{area} vs {expect}");
        // No point of the curves strays from the circle by more than the
        // fitting tolerance (1.2 px at detail 40).
        for (p, _) in t[0].path.segments().flat_map(|seg| {
            (0..=8).map(move |k| {
                (
                    tracedraw_core::geometry::ParamCurve::eval(&seg, k as f64 / 8.0),
                    (),
                )
            })
        }) {
            let r = (p - Point::new(50.5, 50.5)).hypot();
            assert!((r - 30.0).abs() < 1.25, "{r}");
        }

        // A square with specks around it.
        let img: RgbaImage = ImageBuffer::from_fn(100, 100, |x, y| {
            let square = (20..80).contains(&x) && (20..80).contains(&y);
            let speck = (x % 13 == 3 && y % 17 == 5) && !square;
            if square || speck {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let t = trace(&img, &s);
        assert_eq!(t.len(), 1, "the specks are gone");
        let path = &t[0].path;
        let subpaths = path
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count();
        assert_eq!(subpaths, 1, "the square alone");
        let b = path.bounding_box();
        assert!(
            (b.x0 - 20.0).abs() < 0.01 && (b.x1 - 80.0).abs() < 0.01,
            "sharp corners on the pixel corners: {b:?}"
        );
        assert!((path.area().abs() - 3600.0).abs() < 1.0, "{}", path.area());
        // With all the detail, a speck of one pixel stays.
        let mut keep = s.clone();
        keep.detail = 100.0;
        assert!(
            trace(&img, &keep)[0]
                .path
                .elements()
                .iter()
                .filter(|e| matches!(e, PathEl::MoveTo(_)))
                .count()
                > 1
        );
    }

    /// Flat art with anti-aliased edges (#611): a yellow disc crossed by a
    /// black band on white, traced with the Clipart preset. Neighbouring
    /// regions meet with no page showing between them (every point well
    /// inside the art is inside a curve, of the colour of the art there),
    /// the in-between shades of the edges make no regions of their own,
    /// and the trace holds about as many regions as the art has colours.
    #[test]
    fn neighbouring_regions_leave_no_gaps_and_edges_make_no_slivers() {
        const N: u32 = 120;
        // Coverage of a pixel by the art, 4 x 4 samples: yellow, black.
        let cover = |x: u32, y: u32| {
            let (mut yellow, mut black) = (0.0f32, 0.0f32);
            for sy in 0..4 {
                for sx in 0..4 {
                    let (px, py) = (
                        x as f32 + (sx as f32 + 0.5) / 4.0,
                        y as f32 + (sy as f32 + 0.5) / 4.0,
                    );
                    let disc = (px - 60.0).powi(2) + (py - 60.0).powi(2) < 42.0f32.powi(2);
                    // A slanted band, so its edges are anti-aliased too.
                    let band = (py - 0.3 * px - 34.0).abs() < 8.0;
                    if disc && band {
                        black += 1.0 / 16.0;
                    } else if disc {
                        yellow += 1.0 / 16.0;
                    }
                }
            }
            (yellow, black)
        };
        let img: RgbaImage = ImageBuffer::from_fn(N, N, |x, y| {
            let (yl, bl) = cover(x, y);
            let wh = 1.0 - yl - bl;
            let mix = |y: f32, b: f32, w: f32| (y * yl + b * bl + w * wh).round() as u8;
            Rgba([
                mix(250.0, 10.0, 255.0),
                mix(205.0, 10.0, 255.0),
                mix(40.0, 10.0, 255.0),
                255,
            ])
        });
        let s = Preset::Clipart.settings();
        let t = trace(&img, &s);
        assert!((2..=3).contains(&t.len()), "{} regions", t.len());
        for y in 0..N {
            for x in 0..N {
                // Pixels well inside the art (it covers every pixel within
                // two of them, so the outline's own fitting at the art's
                // edge is left out): no page, along the edges between the
                // colours too.
                let inside = x > 1
                    && y > 1
                    && x + 2 < N
                    && y + 2 < N
                    && (y - 2..=y + 2).all(|v| {
                        (x - 2..=x + 2).all(|u| {
                            let (a, b) = cover(u, v);
                            a + b >= 1.0
                        })
                    });
                if !inside {
                    continue;
                }
                for k in 0..4 {
                    let p = Point::new(
                        x as f64 + 0.125 + 0.25 * k as f64,
                        y as f64 + 0.375 + 0.125 * k as f64,
                    );
                    let top = top_colour_at(&t, p);
                    assert!(top.is_some(), "page shows at {p:?}");
                    assert!(
                        top.is_some_and(|c| c.iter().any(|v| *v < 230)),
                        "white at {p:?}: {top:?}"
                    );
                }
            }
        }
        let (coincident, apart) = border_distances(&t);
        assert!(
            coincident > 200,
            "the band's edges are shared: {coincident}"
        );
        assert!(
            apart * 20 < coincident,
            "{apart} apart, {coincident} together"
        );
    }

    /// A pale logo close to its white ground keeps its colour, at the
    /// Logo preset and with two colours; a grey gradient keeps the colour
    /// count it was asked for (#611).
    #[test]
    fn close_colours_and_colour_counts_are_kept() {
        let img: RgbaImage = ImageBuffer::from_fn(80, 80, |x, y| {
            if (20..60).contains(&x) && (20..60).contains(&y) {
                Rgba([246, 246, 246, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let mut s = Preset::Logo.settings();
        assert_eq!(trace(&img, &s).len(), 1, "the logo");
        s.colors = 2;
        assert_eq!(trace(&img, &s).len(), 1, "with two colours");
        let ramp: RgbaImage =
            ImageBuffer::from_fn(256, 8, |x, _| Rgba([x as u8, x as u8, x as u8, 255]));
        let mut s = Preset::HighQualityImage.settings();
        s.colors = 96;
        s.remove = Remove::None;
        let n = trace(&ramp, &s).len();
        assert!(n > 24, "{n} colours of 96");
    }

    /// A hairline two pixels wide in a colour of its own, crossing a
    /// white page beside a black block, keeps its colour at the Clipart
    /// preset: no pixel of it is flat, but it is no edge of two colours.
    #[test]
    fn hairlines_of_their_own_colour_keep_it() {
        let img: RgbaImage = ImageBuffer::from_fn(120, 80, |x, y| {
            if (10..50).contains(&x) && (10..70).contains(&y) {
                Rgba([0, 0, 0, 255])
            } else if (60..110).contains(&x) && (39..41).contains(&y) {
                Rgba([220, 30, 40, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let t = trace(&img, &Preset::Clipart.settings());
        let out = render_traced(&t, 120, 80, 1.0);
        for x in 66..104 {
            let p = out.get_pixel(x, 40);
            assert!(
                p[3] == 255 && p[0] > 150 && p[1] < 100,
                "the red line at {x}: {p:?} of {:?}",
                color_counts(&t)
            );
        }
    }

    /// At the highest detail an orange line of one pixel between red and
    /// yellow stays, as the specks do.
    #[test]
    fn the_highest_detail_keeps_thin_lines() {
        let img: RgbaImage = ImageBuffer::from_fn(40, 40, |_, y| match y {
            0..=19 => Rgba([230, 20, 20, 255]),
            20 => Rgba([240, 120, 20, 255]),
            _ => Rgba([250, 220, 20, 255]),
        });
        let mut s = Preset::Logo.settings();
        s.detail = 100.0;
        s.remove = Remove::None;
        let colours: Vec<[u8; 3]> = trace(&img, &s).iter().map(|t| t.color.to_rgb8()).collect();
        assert!(
            colours.iter().any(|c| c[1] > 90 && c[1] < 160),
            "the orange line: {colours:?}"
        );
    }

    /// Flat art saved as a compressed image (#611: a bee with a black
    /// band, a short dark bar like a letter's stroke, anti-aliased, with
    /// noise and its colours bled a pixel sideways as compression does),
    /// traced with Quick Trace's Clipart preset of 32 colours. Each colour
    /// stays one region with smooth edges: no notches, specks or slivers
    /// along the edges (before, a shade split by noise made regions of
    /// its own and the edges hundreds of bits), and the bar stays.
    #[test]
    fn compressed_flat_art_traces_as_few_clean_regions() {
        use tracedraw_core::geometry::PathEl;
        const N: u32 = 200;
        let mut seed = 777u32;
        let mut noise = move || {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            ((seed >> 16) % 13) as f32 - 6.0
        };
        let white = [253.0f32, 253.0, 253.0];
        let yellow = [246.0f32, 205.0, 50.0];
        let black = [0.0f32, 0.0, 0.0];
        let at = |px: f32, py: f32| {
            let disc = (px - 100.0).powi(2) + (py - 100.0).powi(2) < 80.0f32.powi(2);
            let band = (py - 0.2 * px - 110.0).abs() < 14.0;
            let bar = (60.0..100.0).contains(&px) && (py - 60.0).abs() < 1.6;
            if !disc {
                white
            } else if band || bar {
                black
            } else {
                yellow
            }
        };
        let clean: Vec<[f32; 3]> = (0..N * N)
            .map(|i| {
                let (x, y) = ((i % N) as f32, (i / N) as f32);
                let mut c = [0.0f32; 3];
                for sy in 0..4 {
                    for sx in 0..4 {
                        let s = at(x + (sx as f32 + 0.5) / 4.0, y + (sy as f32 + 0.5) / 4.0);
                        for k in 0..3 {
                            c[k] += s[k] / 16.0;
                        }
                    }
                }
                c
            })
            .collect();
        let img: RgbaImage = ImageBuffer::from_fn(N, N, |x, y| {
            let i = (y * N + x) as usize;
            let j = (y * N + x.saturating_sub(1)) as usize;
            // Blue bled from the pixel to the left, plus noise.
            let c = clean[i];
            let b = (c[2] + clean[j][2]) / 2.0;
            let v = |f: f32| f.round().clamp(0.0, 255.0) as u8;
            Rgba([v(c[0] + noise()), v(c[1] + noise()), v(b + noise()), 255])
        });
        let t = trace(&img, &Preset::Clipart.settings());
        let subpaths: usize = t
            .iter()
            .map(|t| {
                t.path
                    .elements()
                    .iter()
                    .filter(|e| matches!(e, PathEl::MoveTo(_)))
                    .count()
            })
            .sum();
        assert!(t.len() <= 3, "{} regions: {:?}", t.len(), color_counts(&t));
        assert!(subpaths <= 8, "{subpaths} subpaths");
        let out = render_traced(&t, N, N, 1.0);
        // The bar is still there, dark.
        for x in 64..96 {
            let p = out.get_pixel(x, 60);
            assert!(p[3] == 255 && p[0] < 90, "bar at {x}: {p:?}");
        }
    }

    /// The in-between pixels along the edge of two flat colours go to the
    /// one their colour is nearer; a hairline on one colour, or of a
    /// colour that is not between them, stays.
    #[test]
    fn edge_pixels_go_to_the_flat_colours_beside_them() {
        assert!(between([130, 108, 25], [0, 0, 0], [250, 205, 40]));
        assert!(!between([250, 205, 40], [0, 0, 0], [255, 255, 255]));
        // A pale blue line between black and white is a colour of its own.
        assert!(!between([170, 200, 230], [0, 0, 0], [255, 255, 255]));
        assert!(between([128, 128, 128], [0, 0, 0], [255, 255, 255]));
        assert!(nearer_first([60, 50, 10], [0, 0, 0], [250, 205, 40]));
        assert!(!nearer_first([190, 160, 30], [0, 0, 0], [250, 205, 40]));
        let palette = [[0, 0, 0], [250, 205, 40], [125, 102, 20]];
        let (w, h) = (12, 16);
        // Black above, yellow below, a band of 2 px between: darker
        // olive on the black side, lighter on the yellow side.
        let mut labels: Vec<usize> = (0..w * h)
            .map(|i| match i / w {
                0..=6 => 0,
                7 | 8 => 2,
                _ => 1,
            })
            .collect();
        let colors: Vec<[u8; 3]> = (0..w * h)
            .map(|i| match i / w {
                7 => [70, 57, 11],
                8 => [180, 148, 29],
                r => palette[labels[r * w]],
            })
            .collect();
        settle_edges(&mut labels, w, h, &palette, &colors, None);
        assert!(labels[7 * w..8 * w].iter().all(|l| *l == 0), "{labels:?}");
        assert!(labels[8 * w..9 * w].iter().all(|l| *l == 1), "{labels:?}");
        // An olive hairline on yellow alone stays.
        let mut labels: Vec<usize> = (0..w * h).map(|i| if i / w == 8 { 2 } else { 1 }).collect();
        let colors: Vec<[u8; 3]> = labels.iter().map(|l| palette[*l]).collect();
        settle_edges(&mut labels, w, h, &palette, &colors, None);
        assert_eq!(labels.iter().filter(|l| **l == 2).count(), w);
    }

    /// Patches smaller than the detail allows take the colour around them.
    #[test]
    fn despeckle_fills_small_patches_with_their_surroundings() {
        let (w, h) = (6, 5);
        let mut labels = vec![0usize; w * h];
        labels[2 * w + 2] = 1; // a one-pixel speck
        for x in 0..w {
            labels[4 * w + x] = 2; // a row of six
        }
        labels[0] = usize::MAX; // transparent stays
        despeckle(
            &mut labels,
            w,
            h,
            3,
            2,
            &[[0, 0, 0], [90, 90, 90], [255, 255, 255]],
            None,
        );
        assert_eq!(labels[2 * w + 2], 0);
        assert!(labels[4 * w..].iter().all(|l| *l == 2));
        assert_eq!(labels[0], usize::MAX);
        assert_eq!(speck_area(100.0), 1);
        assert!(speck_area(35.0) > speck_area(80.0));
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
        let s = Preset::Logo.settings();
        let zero = tracedraw_core::geometry::Vec2::new(0.0, 0.0);
        let Some(path) = rings_path(&mask, w, 10, zero, 0.5, &s, None) else {
            panic!("not cancelled");
        };
        let rings = path
            .elements()
            .iter()
            .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
            .count();
        assert_eq!(rings, 2);
    }

    /// A disc on white with white patches inside it (the whites of eyes):
    /// Remove Background takes out the white reaching the edge and traces
    /// the patches inside as white curves, Entire color takes every
    /// white out, None traces the white too; a colour given is removed
    /// instead of the one found; a colour given that is not the picture's
    /// takes the palette colour nearest it.
    #[test]
    fn removing_the_background_or_the_entire_colour_or_nothing() {
        let img: RgbaImage = ImageBuffer::from_fn(120, 120, |x, y| {
            let d = ((x as f32 - 60.0).powi(2) + (y as f32 - 60.0).powi(2)).sqrt();
            let eye = ((x as f32 - 45.0).powi(2) + (y as f32 - 50.0).powi(2)).sqrt() < 6.0
                || ((x as f32 - 75.0).powi(2) + (y as f32 - 50.0).powi(2)).sqrt() < 6.0;
            if d < 45.0 && !eye {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let mut s = Preset::Logo.settings();
        s.remove = Remove::Background;
        let t = trace(&img, &s);
        let colours: Vec<[u8; 3]> = t.iter().map(|t| t.color.to_rgb8()).collect();
        assert_eq!(colours, vec![[0, 0, 0], [255, 255, 255]], "the eyes stay");
        let white_rings = t[1]
            .path
            .elements()
            .iter()
            .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
            .count();
        assert_eq!(white_rings, 2, "two eyes, no outer white");
        s.remove = Remove::EntireColor;
        let t = trace(&img, &s);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].color.to_rgb8(), [0, 0, 0]);
        s.remove = Remove::None;
        let t = trace(&img, &s);
        assert_eq!(t.len(), 2);
        // The black given as the background: the disc goes, the white stays
        // whole (it reaches the edge, and is not the background now).
        s.remove = Remove::EntireColor;
        s.background = Some([0, 0, 0]);
        let t = trace(&img, &s);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].color.to_rgb8(), [255, 255, 255]);
        // A colour picked a little off (the anti-aliasing) still means
        // the nearest palette colour.
        s.background = Some([20, 20, 20]);
        assert_eq!(trace(&img, &s)[0].color.to_rgb8(), [255, 255, 255]);
        // Removing the background with a colour whose patches do not reach
        // the edge takes nothing out.
        s.remove = Remove::Background;
        s.background = Some([0, 0, 0]);
        assert_eq!(trace(&img, &s).len(), 2);
    }

    /// Merge adjacent off: each separate patch of a colour is a curve of
    /// its own, with its holes; on, one curve per colour.
    #[test]
    fn merge_adjacent_off_makes_a_curve_per_patch() {
        let img: RgbaImage = ImageBuffer::from_fn(100, 100, |x, y| {
            let a = (10..40).contains(&x) && (10..40).contains(&y);
            let hole = (20..30).contains(&x) && (20..30).contains(&y);
            let b = (60..90).contains(&x) && (60..90).contains(&y);
            if (a && !hole) || b {
                Rgba([200, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let mut s = Preset::Logo.settings();
        // The hole's white does not reach the edge: Entire color, so the
        // red is all there is.
        s.remove = Remove::EntireColor;
        assert_eq!(trace(&img, &s).len(), 1);
        s.merge_adjacent = false;
        let t = trace(&img, &s);
        assert_eq!(t.len(), 2);
        let rings = |p: &BezPath| {
            p.elements()
                .iter()
                .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
                .count()
        };
        let mut counts: Vec<usize> = t.iter().map(|t| rings(&t.path)).collect();
        counts.sort();
        assert_eq!(counts, vec![1, 2], "the square with the hole has two rings");
        // The patches of a mask, each in its own box.
        let mask: Vec<bool> = (0..6 * 4)
            .map(|i| matches!(i % 6, 0 | 1) && i / 6 < 2 || i == 6 * 3 + 5)
            .collect();
        let p = patches(&mask, 6, 4);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].0, (0, 0, 1, 1));
        assert_eq!(p[0].1, vec![true; 4]);
        assert_eq!(p[1].0, (5, 3, 5, 3));
    }

    /// Remove overlap keeps each region to its own pixels: the black ring
    /// no longer reaches under the yellow inside it, as one disc, but
    /// keeps its hole.
    #[test]
    fn remove_overlap_keeps_regions_to_their_pixels() {
        let img: RgbaImage = ImageBuffer::from_fn(80, 80, |x, y| {
            let d = ((x as f32 - 40.0).powi(2) + (y as f32 - 40.0).powi(2)).sqrt();
            if d < 18.0 {
                Rgba([250, 200, 40, 255])
            } else if d < 30.0 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let mut s = Preset::Logo.settings();
        s.smoothing = 0.0;
        let area = |t: &[Traced]| -> f64 {
            t.iter()
                .find(|t| t.color.to_rgb8()[0] < 50)
                .map(|t| t.path.area().abs())
                .unwrap_or(0.0)
        };
        let under = area(&trace(&img, &s));
        s.remove_overlap = true;
        let own = area(&trace(&img, &s));
        assert!(under > own + 20.0, "under {under}, own {own}");
    }

    /// The Colors tab: colours merged into one trace as one; colours
    /// edited are drawn in the new colour; grey mode traces greys.
    #[test]
    fn merged_and_edited_colours_and_grey_mode() {
        let img: RgbaImage = ImageBuffer::from_fn(90, 30, |x, _| match x / 30 {
            0 => Rgba([200, 0, 0, 255]),
            1 => Rgba([0, 0, 200, 255]),
            _ => Rgba([255, 255, 255, 255]),
        });
        let mut s = Preset::Logo.settings();
        s.remove = Remove::None;
        let t = trace(&img, &s);
        assert_eq!(t.len(), 3);
        s.merged = vec![vec![[200, 0, 0], [0, 0, 200]]];
        let t = trace(&img, &s);
        assert_eq!(t.len(), 2, "red and blue are one");
        assert!(t.iter().any(|t| t.color.to_rgb8() == [200, 0, 0]));
        // A colour that is not in the palette merges nothing.
        s.merged = vec![vec![[0, 200, 0], [0, 0, 200]]];
        assert_eq!(trace(&img, &s).len(), 3);
        s.merged.clear();
        s.recolored = vec![([0, 0, 200], [0, 120, 0])];
        let t = trace(&img, &s);
        assert!(t.iter().any(|t| t.color.to_rgb8() == [0, 120, 0]));
        assert!(!t.iter().any(|t| t.color.to_rgb8() == [0, 0, 200]));
        s.recolored.clear();
        s.mode = ColorMode::Grayscale;
        let t = trace(&img, &s);
        assert!(t.iter().all(|t| {
            let c = t.color.to_rgb8();
            c[0] == c[1] && c[1] == c[2]
        }));
        assert_eq!(stats(&t).colors, 3);
        assert_eq!(color_counts(&t).len(), 3);
    }

    /// A trace stops once cancelled, in both modes.
    #[test]
    fn a_cancelled_trace_gives_nothing() {
        let img: RgbaImage = ImageBuffer::from_fn(60, 60, |x, y| {
            if (x / 10 + y / 10) % 2 == 0 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let cancel = Cancel::new(true);
        assert!(trace_unless(&img, &Preset::Logo.settings(), Some(&cancel)).is_none());
        assert!(trace_unless(&img, &Preset::Technical.settings(), Some(&cancel)).is_none());
        let go = Cancel::new(false);
        assert!(trace_unless(&img, &Preset::Logo.settings(), Some(&go)).is_some());
    }

    /// Every long stage of the outline trace looks at the flag: noise
    /// removal, colour reduction, despeckling and the fitting of each
    /// patch's rings.
    #[test]
    fn every_stage_stops_at_the_flag() {
        let raised = Cancel::new(true);
        let img: RgbaImage = ImageBuffer::from_fn(40, 40, |x, y| {
            Rgba([(x * 6) as u8, (y * 6) as u8, ((x + y) * 3) as u8, 255])
        });
        assert!(denoise(&img, Some(&raised)).is_none());
        assert!(denoise(&img, None).is_some());
        assert!(median_cut(&img, 8, Some(&raised)).is_none());
        assert!(median_cut(&img, 8, None).is_some_and(|p| !p.is_empty()));
        let (w, h) = (6, 5);
        let mut labels = vec![0usize; w * h];
        labels[2 * w + 2] = 1;
        despeckle(&mut labels, w, h, 3, 2, &[[0; 3], [9; 3]], Some(&raised));
        assert_eq!(labels[2 * w + 2], 1, "stopped before any change");
        let mask: Vec<bool> = (0..w * h).map(|i| i % w > 1).collect();
        let s = Preset::Logo.settings();
        let zero = tracedraw_core::geometry::Vec2::new(0.0, 0.0);
        assert!(rings_path(&mask, w, h, zero, 0.5, &s, Some(&raised)).is_none());
        assert!(rings_path(&mask, w, h, zero, 0.5, &s, None).is_some());
        let mut labels = vec![0usize; w * h];
        labels[2 * w + 2] = 1;
        labels[2 * w + 3] = 1;
        assert!(smooth_labels(&mut labels, w, h, Some(&raised)).is_none());
        assert!(dissolve_fringes(
            &mut labels,
            w,
            h,
            &[[0; 3], [9; 3]],
            &[[0; 3]; 30],
            Some(&raised)
        )
        .is_none());
    }

    /// A trace of a large noisy picture whose flag is raised while it
    /// runs ends soon after, wherever it was.
    #[test]
    fn a_trace_cancelled_while_it_runs_ends_soon() {
        let img: RgbaImage = ImageBuffer::from_fn(1200, 900, |x, y| {
            let n = (x.wrapping_mul(7919) ^ y.wrapping_mul(104_729)) % 23;
            let base = if (x / 60 + y / 60) % 2 == 0 { 40 } else { 200 };
            let v = (base + n) as u8;
            Rgba([v, v / 2, 255 - v, 255])
        });
        let mut s = Preset::Logo.settings();
        s.detail = 40.0;
        s.colors = 64;
        for wait_ms in [5u64, 60, 250] {
            let cancel = std::sync::Arc::new(Cancel::new(false));
            let flag = cancel.clone();
            let raise = std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(wait_ms));
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
                std::time::Instant::now()
            });
            let out = trace_unless(&img, &s, Some(&cancel));
            let done = std::time::Instant::now();
            let raised_at = raise.join().expect("the thread");
            if out.is_some() {
                // Done before the flag went up (a fast build).
                assert!(
                    wait_ms > 5 && done <= raised_at,
                    "cancelled after {wait_ms} ms"
                );
                continue;
            }
            let late = done.saturating_duration_since(raised_at);
            assert!(
                late < std::time::Duration::from_millis(1500),
                "ended {late:?} after the flag (raised after {wait_ms} ms)"
            );
        }
    }

    /// The preview pictures: a trace drawn at another scale fills the
    /// scaled picture, the wireframe draws lines of its colour only
    /// along the curves; the counts of a trace.
    #[test]
    fn scaled_renders_and_wireframes_and_stats() {
        let img: RgbaImage = ImageBuffer::from_fn(40, 40, |x, y| {
            if (10..30).contains(&x) && (10..30).contains(&y) {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let t = trace(&img, &Preset::Logo.settings());
        let st = stats(&t);
        assert_eq!((st.curves, st.colors), (1, 1));
        assert!(st.nodes >= 4, "{st:?}");
        let big = render_traced_scaled(&t, 40, 40, 3.0, 1.0);
        assert_eq!(big.dimensions(), (120, 120));
        assert_eq!(big.get_pixel(60, 60).0, [0, 0, 0, 255]);
        assert_eq!(big.get_pixel(5, 5)[3], 0, "outside is clear");
        let wire = render_wireframe(&t, 40, 40, 2.0);
        assert_eq!(wire.dimensions(), (80, 80));
        assert_eq!(wire.get_pixel(40, 40)[3], 0, "the inside is clear");
        let on_edge = (0..80).any(|x| wire.get_pixel(x, 20)[3] > 0);
        assert!(on_edge, "the line along the top edge");
        // The wireframe colour (the edges' half covered pixels within
        // rounding).
        let coloured = wire
            .pixels()
            .filter(|p| p[3] >= 128)
            .all(|p| (0..3).all(|k| (p[k] as i32 - WIRE[k] as i32).abs() <= 2));
        assert!(coloured);
        assert!(wire.pixels().any(|p| p[3] >= 128));
    }
    /// #1250: art on a transparent background (a clip art file) has no
    /// colour on the picture's edge. The commonest of none was the last
    /// colour of the palette, which Remove entire color took out of the
    /// art: the yellow of the bee's head and stripes, leaving their
    /// fringes as outlines. Nothing is removed now.
    #[test]
    fn art_on_a_transparent_background_keeps_every_colour() {
        let img: RgbaImage = ImageBuffer::from_fn(100, 100, |x, y| {
            let d = ((x as f32 - 50.0).powi(2) + (y as f32 - 50.0).powi(2)).sqrt();
            if d >= 40.0 {
                Rgba([0, 0, 0, 0])
            } else if (y as f32 - 50.0).abs() < 8.0 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([246, 205, 50, 255])
            }
        });
        for remove in Remove::ALL {
            let mut s = Preset::Clipart.settings();
            s.remove = remove;
            let t = trace(&img, &s);
            let colours: Vec<[u8; 3]> = t.iter().map(|t| t.color.to_rgb8()).collect();
            assert_eq!(colours.len(), 2, "{remove:?}: {colours:?}");
            let out = render_traced(&t, 100, 100, 1.0);
            let p = out.get_pixel(50, 25);
            assert!(
                p[3] == 255 && p[0] > 200 && p[1] > 160,
                "{remove:?}: head {p:?}"
            );
        }
    }

    /// #1250: flat art from a video frame or a scaled screenshot, where
    /// the edge of black and yellow is a soft band four pixels wide of
    /// in-between colours, with noise. The band goes to the two colours:
    /// one ring each and few nodes, no slivers or bumps of in-between
    /// colours along the edge (before: a dozen regions along it).
    #[test]
    fn blurred_edges_go_to_the_colours_either_side() {
        use tracedraw_core::geometry::PathEl;
        const N: u32 = 160;
        let mut seed = 99u32;
        let mut noise = move || {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            ((seed >> 16) % 11) as f32 - 5.0
        };
        let black = [2.0f32, 1.0, 2.0];
        let yellow = [247.0f32, 205.0, 52.0];
        let img: RgbaImage = ImageBuffer::from_fn(N, N, |x, y| {
            let edge = 80.0 + 12.0 * (y as f32 / 25.0).sin();
            let t = ((x as f32 - edge) / 4.0 + 0.5).clamp(0.0, 1.0);
            let mut v = |k: usize| {
                (black[k] + (yellow[k] - black[k]) * t + noise())
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            Rgba([v(0), v(1), v(2), 255])
        });
        let mut s = Preset::Clipart.settings();
        s.remove = Remove::None;
        let t = trace(&img, &s);
        let rings = |t: &Traced| {
            t.path
                .elements()
                .iter()
                .filter(|e| matches!(e, PathEl::MoveTo(_)))
                .count()
        };
        assert_eq!(
            t.len(),
            2,
            "{:?}",
            t.iter().map(|t| t.color.to_rgb8()).collect::<Vec<_>>()
        );
        assert!(t.iter().all(|t| rings(t) == 1));
        let st = stats(&t);
        assert!(st.nodes <= 40, "{} nodes", st.nodes);
    }

    /// One-pixel bumps and notches along an edge go; a line one pixel
    /// wide and its ends stay.
    #[test]
    fn smoothing_takes_bumps_and_notches_off_edges_and_keeps_lines() {
        let (w, h) = (12, 10);
        let mut labels: Vec<usize> = (0..w * h).map(|i| if i / w < 5 { 0 } else { 1 }).collect();
        labels[5 * w + 4] = 0; // a bump of the top colour
        labels[4 * w + 8] = 1; // a notch in it
        for x in 1..w - 1 {
            labels[7 * w + x] = 2; // a line
        }
        smooth_labels(&mut labels, w, h, None).unwrap();
        assert_eq!(labels[5 * w + 4], 1, "bump");
        assert_eq!(labels[4 * w + 8], 0, "notch");
        assert!((1..w - 1).all(|x| labels[7 * w + x] == 2), "line");
        assert!(labels[..5 * w].iter().all(|l| *l == 0));
    }

    /// A band of an in-between colour along the edge of black and yellow
    /// goes to them, each pixel to the nearer; a thin stroke of that colour
    /// on yellow alone stays.
    #[test]
    fn fringes_go_to_the_two_sides_and_strokes_stay() {
        let palette = [[0, 0, 0], [250, 205, 40], [125, 102, 20]];
        let (w, h) = (20, 14);
        // Black above, yellow below, a band 3 px thick between.
        let mut labels: Vec<usize> = (0..w * h)
            .map(|i| match i / w {
                0..=5 => 0,
                6..=8 => 2,
                _ => 1,
            })
            .collect();
        let colors: Vec<[u8; 3]> = (0..w * h)
            .map(|i| match i / w {
                6 => [60, 50, 10],
                7 => [110, 90, 18],
                8 => [200, 165, 32],
                r => palette[labels[r * w]],
            })
            .collect();
        dissolve_fringes(&mut labels, w, h, &palette, &colors, None).unwrap();
        assert!(labels[6 * w..8 * w].iter().all(|l| *l == 0), "{labels:?}");
        assert!(labels[8 * w..9 * w].iter().all(|l| *l == 1), "{labels:?}");
        // A stroke two pixels wide of the in-between colour on yellow.
        let mut labels: Vec<usize> = (0..w * h)
            .map(|i| if (6..8).contains(&(i / w)) { 2 } else { 1 })
            .collect();
        let colors: Vec<[u8; 3]> = labels.iter().map(|l| palette[*l]).collect();
        dissolve_fringes(&mut labels, w, h, &palette, &colors, None).unwrap();
        assert_eq!(labels.iter().filter(|l| **l == 2).count(), 2 * w);
    }

    /// A deterministic noise of up to `amp` either way, per call.
    fn noise_source(amp: i32) -> impl FnMut() -> i32 {
        let mut seed = 12345u32;
        move || {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            ((seed >> 16) % (2 * amp as u32 + 1)) as i32 - amp
        }
    }

    /// A sunflower's heart as halftone art: a black ring round a teal disc
    /// printed with small yellow-green dots, on a transparent background,
    /// with a little noise.
    fn halftone_image() -> (RgbaImage, Vec<(u32, u32)>) {
        let mut noise = noise_source(4);
        let (n, c) = (220u32, 110.0f32);
        let mut dots = Vec::new();
        let img = ImageBuffer::from_fn(n, n, |x, y| {
            let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
            let base: [i32; 3] = if d >= 100.0 {
                return Rgba([0, 0, 0, 0]);
            } else if d >= 90.0 {
                [0, 0, 0]
            } else if x % 6 < 3 && y % 6 < 3 && d < 84.0 {
                [150, 170, 90]
            } else {
                [35, 60, 62]
            };
            let mut v = |k: usize| (base[k] + noise()).clamp(0, 255) as u8;
            Rgba([v(0), v(1), v(2), 255])
        });
        for y in (1..n).step_by(6) {
            for x in (1..n).step_by(6) {
                let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
                if d < 80.0 {
                    dots.push((x, y));
                }
            }
        }
        (img, dots)
    }

    /// The dots of a halftone are art, not noise: at the Clipart preset
    /// they stay (every dot of three pixels standing out from its teal);
    /// before, every patch under 16 pixels went to the colour around it
    /// and the heart came out plain teal. The black ring, under all the
    /// rest, is one outline with its hole filled by the regions drawn
    /// over it, and the whole is drawn back nearly exactly.
    #[test]
    fn halftone_dots_stay_and_the_regions_below_are_whole() {
        use tracedraw_core::geometry::PathEl;
        let (img, dots) = halftone_image();
        let t = trace(&img, &Preset::Clipart.settings());
        let out = render_traced(&t, img.width(), img.height(), 1.0);
        let kept = dots
            .iter()
            .filter(|(x, y)| {
                let p = out.get_pixel(*x, *y);
                distance2([p[0], p[1], p[2]], [150, 170, 90])
                    < distance2([p[0], p[1], p[2]], [35, 60, 62])
            })
            .count();
        assert!(kept * 10 >= dots.len() * 9, "{kept} of {} dots", dots.len());
        assert!(t.len() <= 4, "{:?}", color_counts(&t));
        let black = t
            .iter()
            .find(|t| t.color.to_rgb8().iter().all(|c| *c < 16))
            .expect("the black ring");
        let rings = black
            .path
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::MoveTo(_)))
            .count();
        assert_eq!(rings, 1, "the ring's hole is filled");
        let wrong = wrong_share(&img, &out, None).unwrap();
        assert!(wrong < 3.0, "{wrong}% drawn wrong");
    }

    /// Shades the colour reduction makes of one colour's noise and edges
    /// join that colour, which keeps its exact value: a black square with
    /// a dark brown fringe and grey-flecked white traces as #000000 and
    /// white, two colours, where each shade was a curve of its own and the
    /// black came out a brown mean.
    #[test]
    fn shades_join_their_colour_which_keeps_its_value() {
        let mut noise = noise_source(6);
        let img: RgbaImage = ImageBuffer::from_fn(120, 120, |x, y| {
            let inside = (30..90).contains(&x) && (30..90).contains(&y);
            let fringe = (28..92).contains(&x) && (28..92).contains(&y) && !inside;
            if inside {
                Rgba([0, 0, 0, 255])
            } else if fringe {
                Rgba([24, 16, 6, 255])
            } else {
                let v = (250 + noise().abs().min(5)) as u8;
                Rgba([v, v, v, 255])
            }
        });
        let mut s = Preset::Clipart.settings();
        s.remove = Remove::None;
        let t = trace(&img, &s);
        let colors: Vec<[u8; 3]> = color_counts(&t).into_iter().map(|(c, _)| c).collect();
        assert!(colors.contains(&[0, 0, 0]), "{colors:?}");
        assert!(colors.len() <= 2, "{colors:?}");
    }

    /// A small logo, anti-aliased: a red disc with a white bar and a blue
    /// triangle on white.
    fn logo_image() -> RgbaImage {
        let at = |x: f32, y: f32| -> [f32; 3] {
            let d = ((x - 150.0).powi(2) + (y - 150.0).powi(2)).sqrt();
            let tri = y > 40.0 && y < 125.0 && (x - 150.0).abs() < (y - 40.0) * 0.6;
            if tri {
                [20.0, 40.0, 120.0]
            } else if (70.0..230.0).contains(&x) && (130.0..190.0).contains(&y) {
                [255.0, 255.0, 255.0]
            } else if d < 120.0 {
                [220.0, 30.0, 40.0]
            } else {
                [255.0, 255.0, 255.0]
            }
        };
        ImageBuffer::from_fn(300, 300, |x, y| {
            let mut c = [0.0f32; 3];
            for sy in 0..4 {
                for sx in 0..4 {
                    let s = at(
                        x as f32 + (sx as f32 + 0.5) / 4.0,
                        y as f32 + (sy as f32 + 0.5) / 4.0,
                    );
                    for k in 0..3 {
                        c[k] += s[k] / 16.0;
                    }
                }
            }
            Rgba([c[0] as u8, c[1] as u8, c[2] as u8, 255])
        })
    }

    /// The Optimized preset picks a detail of its own for the picture and
    /// traces it as it is in few curves and nodes; the settings it finds
    /// trace as they are.
    #[test]
    fn the_optimized_preset_draws_the_picture_as_it_is_in_few_nodes() {
        let img = logo_image();
        let s = Preset::Optimized.settings();
        assert!(s.optimize && !s.centerline);
        let found = optimize(&img, &s, None).expect("settings");
        assert!(!found.optimize);
        assert!(OPTIMIZE_DETAILS.contains(&found.detail), "{}", found.detail);
        let t = trace(&img, &s);
        let st = stats(&t);
        let out = render_traced(&t, 300, 300, 1.0);
        let wrong = wrong_share(&img, &out, None).unwrap();
        assert!(wrong < 1.5, "{wrong}% drawn wrong");
        assert!(st.curves <= 4, "{:?}", color_counts(&t));
        assert!(st.nodes < 200, "{} nodes", st.nodes);
        // A cancelled search gives nothing.
        let raised = Cancel::new(true);
        assert!(optimize(&img, &s, Some(&raised)).is_none());
    }

    /// A picture drawn a pixel off along its edges is not drawn wrong; a
    /// lost dot is.
    #[test]
    fn wrong_share_forgives_edges_a_pixel_off_and_counts_lost_dots() {
        let disc = circle_image();
        assert_eq!(wrong_share(&disc, &disc, None), Some(0.0));
        let moved = ImageBuffer::from_fn(100, 100, |x, y| *disc.get_pixel(x.saturating_sub(1), y));
        assert_eq!(wrong_share(&disc, &moved, None), Some(0.0));
        let mut dotted = disc.clone();
        for y in 80..84 {
            for x in 80..84 {
                dotted.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        assert!(wrong_share(&disc, &dotted, None).unwrap() > 0.0);
    }

    /// Black and white traces are #000000 and #FFFFFF, whatever the ink
    /// and paper of the scan: the mode's two colours are not moved onto
    /// the picture's commonest shades.
    #[test]
    fn black_and_white_stays_pure() {
        let img: RgbaImage = ImageBuffer::from_fn(100, 100, |x, y| {
            if (30..70).contains(&x) && (30..70).contains(&y) {
                Rgba([6, 6, 8, 255])
            } else {
                Rgba([250, 250, 250, 255])
            }
        });
        let mut s = Preset::LineArt.settings();
        s.remove = Remove::None;
        let colors: Vec<[u8; 3]> = color_counts(&trace(&img, &s))
            .into_iter()
            .map(|(c, _)| c)
            .collect();
        assert!(
            colors
                .iter()
                .all(|c| *c == [0, 0, 0] || *c == [255, 255, 255]),
            "{colors:?}"
        );
    }

    /// A background colour specified (or picked with the eyedropper) is
    /// matched among the colours of the trace: on noisy paper whose shades
    /// joined into one, white specified still takes the paper out. Before,
    /// a joined shade left in the palette stood nearer white than the
    /// paper's colour, matched no pixel, and the page was traced.
    #[test]
    fn a_specified_background_is_removed_after_shades_join() {
        let mut noise = noise_source(4);
        let img: RgbaImage = ImageBuffer::from_fn(120, 120, |x, y| {
            if (40..80).contains(&x) && (40..80).contains(&y) {
                Rgba([0, 0, 0, 255])
            } else {
                let v = (250 + noise()).clamp(0, 255) as u8;
                Rgba([v, v, v, 255])
            }
        });
        let mut s = Preset::Clipart.settings();
        s.remove = Remove::Background;
        s.background = Some([255, 255, 255]);
        let t = trace(&img, &s);
        let colors: Vec<[u8; 3]> = color_counts(&t).into_iter().map(|(c, _)| c).collect();
        assert_eq!(colors, vec![[0, 0, 0]], "the paper is gone");
    }

    /// Never more colours than asked for: flat art with hairlines of
    /// colours of their own, traced in 8 colours, keeps 8 at most.
    #[test]
    fn the_palette_keeps_to_the_number_of_colours() {
        let lines: [[u8; 3]; 6] = [
            [220, 30, 40],
            [30, 160, 60],
            [40, 60, 220],
            [230, 200, 20],
            [160, 40, 200],
            [20, 200, 210],
        ];
        let img: RgbaImage = ImageBuffer::from_fn(200, 200, |x, y| {
            let band = (y / 30) as usize;
            if y % 30 < 2 && band < lines.len() {
                let c = lines[band];
                Rgba([c[0], c[1], c[2], 255])
            } else if x < 100 {
                Rgba([0, 0, 0, 255])
            } else if x < 150 {
                Rgba([128, 128, 128, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let mut s = Preset::Clipart.settings();
        s.colors = 8;
        s.remove = Remove::None;
        let n = color_counts(&trace(&img, &s)).len();
        assert!(n <= 8, "{n} colours");
        let palette = art_palette(&img, 8, None).unwrap();
        assert!(palette.len() <= 8, "{palette:?}");
    }

    /// The holes of a mask: the patch of unset pixels it closes round,
    /// not what reaches the box's edge (the trace fills a region's holes
    /// that hold only regions drawn above it).
    #[test]
    fn holes_of_regions_drawn_above_are_filled() {
        let (w, h) = (7, 7);
        let ring: Vec<bool> = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                (1..6).contains(&x)
                    && (1..6).contains(&y)
                    && !((2..5).contains(&x) && (2..5).contains(&y))
            })
            .collect();
        let mut found: Vec<Vec<usize>> = Vec::new();
        for_each_hole(&ring, w, h, |hole| found.push(hole.to_vec()));
        assert_eq!(found.len(), 1, "the hole, not the outside");
        found[0].sort_unstable();
        let want: Vec<usize> = (2..5)
            .flat_map(|y| (2..5).map(move |x| y * w + x))
            .collect();
        assert_eq!(found[0], want);
    }

    /// The colour of the topmost curve whose fill holds `p` (even-odd).
    fn top_colour_at(t: &[Traced], p: Point) -> Option<[u8; 3]> {
        t.iter()
            .rev()
            .find(|t| t.closed && t.path.winding(p) % 2 != 0)
            .map(|t| t.color.to_rgb8())
    }

    /// Where the curves of a trace run along each other: the points along
    /// each curve (every quarter pixel) that lie on another curve (within
    /// 0.05 px), and those that run beside one, 0.05 to 2.5 px off it: a
    /// border drawn twice, once by each region beside it (#1250). Points
    /// within 3 px of a junction (a pixel corner that is a node of two
    /// curves or more, where regions meet) are left out: the curves
    /// leaving it part there.
    fn border_distances(t: &[Traced]) -> (usize, usize) {
        use tracedraw_core::geometry::PathEl;
        let mut nodes: std::collections::HashMap<(i64, i64), Vec<usize>> = Default::default();
        for (k, c) in t.iter().enumerate() {
            for el in c.path.elements() {
                if let PathEl::MoveTo(p) | PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) = el {
                    if p.x.fract() == 0.0 && p.y.fract() == 0.0 {
                        let v = nodes.entry((p.x as i64, p.y as i64)).or_default();
                        if !v.contains(&k) {
                            v.push(k);
                        }
                    }
                }
            }
        }
        let junctions: Vec<Point> = nodes
            .iter()
            .filter(|(_, v)| v.len() >= 2)
            .map(|((x, y), _)| Point::new(*x as f64, *y as f64))
            .collect();
        let near_junction = |p: Point| junctions.iter().any(|j| (p - *j).hypot2() < 9.0);
        let mut segments: Vec<(usize, Point, Point)> = Vec::new();
        for (k, c) in t.iter().enumerate() {
            let mut at = Point::ZERO;
            let mut start = Point::ZERO;
            tracedraw_core::geometry::flatten(&c.path, 0.005, &mut |el| match el {
                PathEl::MoveTo(p) => {
                    at = p;
                    start = p;
                }
                PathEl::LineTo(p) => {
                    segments.push((k, at, p));
                    at = p;
                }
                PathEl::ClosePath => {
                    segments.push((k, at, start));
                    at = start;
                }
                _ => {}
            });
        }
        const CELL: f64 = 3.0;
        let mut grid: std::collections::HashMap<(i64, i64), Vec<usize>> = Default::default();
        for (i, (_, a, b)) in segments.iter().enumerate() {
            let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
            let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
            for gy in (y0 / CELL).floor() as i64..=(y1 / CELL).floor() as i64 {
                for gx in (x0 / CELL).floor() as i64..=(x1 / CELL).floor() as i64 {
                    grid.entry((gx, gy)).or_default().push(i);
                }
            }
        }
        let to_segment = |p: Point, a: Point, b: Point| {
            let ab = b - a;
            let l = ab.hypot2();
            let t = if l < 1e-12 {
                0.0
            } else {
                ((p - a).dot(ab) / l).clamp(0.0, 1.0)
            };
            (p - (a + ab * t)).hypot()
        };
        let (mut together, mut apart) = (0, 0);
        for (k, a, b) in &segments {
            let n = ((*b - *a).hypot() / 0.25).ceil().max(1.0) as usize;
            for s in 0..n {
                let p = a.lerp(*b, s as f64 / n as f64);
                if near_junction(p) {
                    continue;
                }
                let (gx, gy) = ((p.x / CELL).floor() as i64, (p.y / CELL).floor() as i64);
                let mut best = f64::INFINITY;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        for &i in grid.get(&(gx + dx, gy + dy)).into_iter().flatten() {
                            let (j, c, d) = segments[i];
                            if j != *k {
                                best = best.min(to_segment(p, c, d));
                            }
                        }
                    }
                }
                if best < 0.05 {
                    together += 1;
                } else if best < 2.5 {
                    apart += 1;
                }
            }
        }
        (together, apart)
    }

    /// A bee's head with its eyes, cheeks, wings beside it and stripes
    /// below (#1250): every border between two colours is one curve that
    /// both regions use, so their outlines coincide (no second curve a
    /// pixel or two beside the first, as when each region was fitted on
    /// its own and reached under the next), no page shows between them,
    /// and the head, drawn under its eyes and cheeks, is one ring.
    #[test]
    fn neighbouring_regions_share_one_curve_on_each_border() {
        const N: u32 = 160;
        let yellow = Rgba([255, 204, 53, 255]);
        let black = Rgba([20, 18, 18, 255]);
        let blue = Rgba([148, 213, 217, 255]);
        let orange = Rgba([246, 153, 99, 255]);
        let white = Rgba([255, 255, 255, 255]);
        let colour = |x: f64, y: f64| -> Rgba<u8> {
            let d = |cx: f64, cy: f64| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            let ellipse = |cx: f64, cy: f64, rx: f64, ry: f64| {
                ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) < 1.0
            };
            if d(80.0, 62.0) < 40.0 {
                if d(64.0, 58.0) < 7.0 || d(96.0, 58.0) < 7.0 {
                    black
                } else if ellipse(60.0, 76.0, 7.0, 4.5) || ellipse(100.0, 76.0, 7.0, 4.5) {
                    orange
                } else {
                    yellow
                }
            } else if ellipse(80.0, 118.0, 32.0, 30.0) {
                // Stripes across the body, side by side.
                if ((y - 0.15 * x) as i64).rem_euclid(18) < 9 {
                    black
                } else {
                    yellow
                }
            } else if ellipse(38.0, 70.0, 30.0, 20.0) || ellipse(122.0, 70.0, 30.0, 20.0) {
                blue
            } else {
                white
            }
        };
        // Anti-aliased, 4 x 4 samples a pixel.
        let img: RgbaImage = ImageBuffer::from_fn(N, N, |x, y| {
            let mut sum = [0u32; 3];
            for sy in 0..4 {
                for sx in 0..4 {
                    let c = colour(
                        x as f64 + (sx as f64 + 0.5) / 4.0,
                        y as f64 + (sy as f64 + 0.5) / 4.0,
                    );
                    for k in 0..3 {
                        sum[k] += c[k] as u32;
                    }
                }
            }
            Rgba([
                (sum[0] / 16) as u8,
                (sum[1] / 16) as u8,
                (sum[2] / 16) as u8,
                255,
            ])
        });
        let mut s = Preset::Clipart.settings();
        let t = trace(&img, &s);
        let (together, apart) = border_distances(&t);
        assert!(together > 1000, "borders shared: {together}");
        assert!(
            apart * 20 < together,
            "{apart} beside another curve, {together} on one"
        );
        // No page between the regions: every point well inside the art
        // (three pixels from its outer edge) is inside a curve.
        let art = |x: f64, y: f64| colour(x, y) != white;
        for y in 0..N {
            for x in 0..N {
                let p = Point::new(x as f64 + 0.3, y as f64 + 0.6);
                let deep =
                    (-3..=3).all(|dy| (-3..=3).all(|dx| art(p.x + dx as f64, p.y + dy as f64)));
                if deep {
                    assert!(top_colour_at(&t, p).is_some(), "page shows at {p:?}");
                }
            }
        }
        // The head is one ring: its eyes and cheeks are drawn over it.
        let is_yellow = |c: &&Traced| {
            let [r, g, b] = c.color.to_rgb8();
            r > 200 && g > 180 && b < 120
        };
        let yellow_rings: Vec<usize> = t
            .iter()
            .filter(is_yellow)
            .map(|c| {
                c.path
                    .elements()
                    .iter()
                    .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
                    .count()
            })
            .collect();
        assert_eq!(yellow_rings.len(), 1, "one yellow curve");
        let head_eye = Point::new(64.0, 58.0);
        let top = top_colour_at(&t, head_eye).unwrap_or([255; 3]);
        assert!(top[0] < 80, "the eye shows over the head: {top:?}");
        let yellow_curve = t.iter().find(is_yellow);
        assert!(
            yellow_curve.is_some_and(|c| c.path.winding(head_eye) % 2 != 0),
            "the head reaches under its eyes"
        );
        // Each curve one per patch too, and without smoothing.
        s.merge_adjacent = false;
        let (together, apart) = border_distances(&trace(&img, &s));
        assert!(
            apart * 20 < together,
            "patches: {apart} beside, {together} on one"
        );
        s.smoothing = 0.0;
        let (together, apart) = border_distances(&trace(&img, &s));
        assert!(
            apart * 20 < together,
            "unsmoothed: {apart} beside, {together} on one"
        );
    }

    /// The borders of a labelling: each edge between two regions on one
    /// border, borders ending at the corners where three regions meet,
    /// an island a loop; a ring put together from them runs forwards on
    /// one side and backwards on the other.
    #[test]
    fn borders_meet_at_junctions_and_islands_are_loops() {
        // 0 0 1 1
        // 0 2 2 1
        // 0 0 1 1, and a picture edge all round.
        let labels = vec![0, 0, 1, 1, 0, 2, 2, 1, 0, 0, 1, 1];
        let Some(b) = borders(&labels, 4, 3, None) else {
            panic!("not cancelled");
        };
        let edges: usize = b.iter().map(|b| b.steps.len()).sum();
        // Inner edges between different labels, plus the outline.
        let inner = 2 + 2 + 1 + 1 + 1 + 1;
        assert_eq!(edges, inner + 2 * (4 + 3));
        assert!(b.iter().all(|b| !b.closed));
        // A pixel of 1 inside 0: one loop.
        let mut island = vec![0usize; 25];
        island[12] = 1;
        let Some(b) = borders(&island, 5, 5, None) else {
            panic!("not cancelled");
        };
        assert_eq!(
            b.iter().filter(|b| b.closed).count(),
            2,
            "the island and the outline"
        );
        let raised = Cancel::new(true);
        assert!(borders(&island, 5, 5, Some(&raised)).is_none());
        // Reversed curves end where they start and keep their pieces.
        let mut c = BezPath::new();
        c.move_to(Point::new(0.0, 0.0));
        c.curve_to(
            Point::new(1.0, 0.0),
            Point::new(2.0, 1.0),
            Point::new(2.0, 2.0),
        );
        c.line_to(Point::new(0.0, 2.0));
        let mut r = BezPath::new();
        r.move_to(Point::new(0.0, 2.0));
        append_curve(&mut r, &c, false, false);
        use tracedraw_core::geometry::PathEl;
        assert_eq!(
            r.elements()[1..],
            [
                PathEl::LineTo(Point::new(2.0, 2.0)),
                PathEl::CurveTo(
                    Point::new(2.0, 1.0),
                    Point::new(1.0, 0.0),
                    Point::new(0.0, 0.0)
                ),
            ]
        );
    }

    /// Colours inside another's holes stack above it, the darkest first
    /// otherwise, and colours in each other's holes darkest first.
    #[test]
    fn colours_inside_holes_stack_above_the_one_around() {
        // 0 holds 2 in a hole, 1 is beside both.
        let mut inside = vec![vec![false; 3]; 3];
        inside[0][2] = true;
        assert_eq!(stacking_order(&inside, &[200, 10, 50]), vec![1, 0, 2]);
        // 0 and 2 in each other's holes: the darker first.
        inside[2][0] = true;
        assert_eq!(stacking_order(&inside, &[200, 10, 50]), vec![1, 2, 0]);
        assert_eq!(stacking_order(&[], &[]), Vec::<usize>::new());
    }
}
