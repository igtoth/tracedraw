//! Print layout: how pages go onto the printed sheets (imposition) and
//! the printers' marks around them. The Print dialog chooses; `print`
//! builds the sheets with [`impose`] and writes them as one PDF.
//!
//! Layouts: as in the document (one page per sheet), n-up (several pages
//! per sheet in reading order), booklet (two pages per sheet side, bound
//! by saddle stitch or perfect binding, see [`Booklet`]) and step and
//! repeat (one page repeated over the sheet). On a fixed sheet the pages are scaled down to fit their cells;
//! on a fitted sheet they keep their size and the sheet grows. Marks sit
//! in the margin around the pages: crop marks at every cut, registration
//! targets, colour bars, a densitometer scale and a line of page
//! information.

use tracedraw_core::{
    document::{Layer, Page, ShapeKind, TextSpan},
    geometry::{Affine, BezPath, Point, Rect, Size},
    Color, Document, Fill, Shape, ShapeId, Stroke,
};

/// How pages go onto sheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// One page per sheet, as in the document.
    #[default]
    AsIs,
    /// Several pages per sheet, left to right and top to bottom.
    NUp { cols: u32, rows: u32 },
    /// Two pages per sheet side in the order a folded booklet needs (the
    /// page count is rounded up to a multiple of four); how the sheets
    /// are bound is in [`Options::booklet`].
    Booklet,
    /// Each page repeated over the sheet (cards, labels).
    Repeat { cols: u32, rows: u32 },
}

impl Layout {
    /// The grid of cells on a sheet (columns, rows).
    pub fn grid(self) -> (u32, u32) {
        match self {
            Layout::AsIs => (1, 1),
            Layout::NUp { cols, rows } | Layout::Repeat { cols, rows } => {
                (cols.clamp(1, 20), rows.clamp(1, 20))
            }
            Layout::Booklet => (2, 1),
        }
    }
}

/// How the folded sheets of a booklet are bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Binding {
    /// Saddle stitch: every sheet folded inside the one before and
    /// stapled through the fold (8 pages: 8 1, 2 7 on the outer sheet).
    #[default]
    SaddleStitch,
    /// Perfect bound: the sheets go in signatures of
    /// [`Booklet::signature_sheets`] sheets, each signature folded on its
    /// own, the signatures stacked and glued at the spine (one sheet per
    /// signature: 4 1, 2 3 on the first sheet, 8 5, 6 7 on the second).
    PerfectBound,
}

/// The booklet's binding and the room it needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Booklet {
    pub binding: Binding,
    /// Sheets in each signature of a perfect-bound booklet (1 to 32).
    pub signature_sheets: u32,
    /// Creep compensation of a saddle-stitched booklet, mm: how far the
    /// pages of the innermost sheet move toward the spine. The outer
    /// sheet stays, the sheets between move in proportion, so the fore
    /// edges line up once the sheets are nested and trimmed.
    pub creep: f64,
    /// The space between the two pages of a spread, mm, centred on the
    /// fold.
    pub gap: f64,
}

impl Default for Booklet {
    fn default() -> Self {
        Booklet {
            binding: Binding::SaddleStitch,
            signature_sheets: 4,
            creep: 0.0,
            gap: 0.0,
        }
    }
}

impl Booklet {
    /// Sheets per signature, clamped to 1..=32.
    pub fn sheets_per_signature(&self) -> usize {
        self.signature_sheets.clamp(1, 32) as usize
    }

    fn safe(v: f64, max: f64) -> f64 {
        if v.is_finite() {
            v.clamp(0.0, max)
        } else {
            0.0
        }
    }

    /// The gap, clamped to 0..=100 mm.
    pub fn gap_mm(&self) -> f64 {
        Self::safe(self.gap, 100.0)
    }

    /// The creep at the innermost sheet, clamped to 0..=20 mm.
    pub fn creep_mm(&self) -> f64 {
        Self::safe(self.creep, 20.0)
    }
}

/// The page count of a booklet and what it takes to print it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BookletInfo {
    /// The pages printed.
    pub pages: usize,
    /// Blank pages added at the end to reach a multiple of four.
    pub blanks: usize,
    /// Sheets of paper, each printed on both sides.
    pub sheets: usize,
    /// Signatures (1 for saddle stitch).
    pub signatures: usize,
}

/// What a booklet of `pages` pages bound as `b` says takes.
pub fn booklet_info(pages: usize, b: &Booklet) -> BookletInfo {
    let padded = pages.div_ceil(4).max(1) * 4;
    let sheets = padded / 4;
    let signatures = match b.binding {
        Binding::SaddleStitch => 1,
        Binding::PerfectBound => sheets.div_ceil(b.sheets_per_signature()),
    };
    BookletInfo {
        pages,
        blanks: padded - pages,
        sheets,
        signatures,
    }
}

/// One side of a booklet sheet: the left and right page (`None` blank),
/// and how deep its sheet sits in its fold (0 outermost) of how many.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BookletSide {
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub depth: usize,
    pub nested: usize,
}

/// The sides of the sheets folded together for the pages `first ..
/// first + n` (`n` a multiple of four), outer sheet first, front then
/// back. Pages from `pages` on are blank.
fn nested_sides(first: usize, n: usize, pages: usize) -> Vec<BookletSide> {
    let page = |i: usize| (first + i < pages).then_some(first + i);
    (0..n / 2)
        .map(|side| {
            let (a, b) = (n - 1 - side, side);
            let (left, right) = if side % 2 == 0 {
                (page(a), page(b))
            } else {
                (page(b), page(a))
            };
            BookletSide {
                left,
                right,
                depth: side / 2,
                nested: n / 4,
            }
        })
        .collect()
}

/// The sides of every sheet of a booklet of `pages` pages bound as `b`,
/// in printing order.
pub fn booklet_sides(pages: usize, b: &Booklet) -> Vec<BookletSide> {
    let padded = pages.div_ceil(4).max(1) * 4;
    match b.binding {
        Binding::SaddleStitch => nested_sides(0, padded, pages),
        Binding::PerfectBound => {
            let per = 4 * b.sheets_per_signature();
            (0..padded)
                .step_by(per)
                .flat_map(|first| nested_sides(first, per.min(padded - first), pages))
                .collect()
        }
    }
}

/// The printers' marks to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Marks {
    pub crop: bool,
    pub registration: bool,
    pub color_bars: bool,
    pub densitometer: bool,
    pub page_info: bool,
    /// Each page's number under it.
    pub page_numbers: bool,
}

impl Marks {
    pub fn any(self) -> bool {
        self.crop
            || self.registration
            || self.color_bars
            || self.densitometer
            || self.page_info
            || self.page_numbers
    }
}

/// Everything the layout needs besides the document.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub layout: Layout,
    /// The sheet size; `None` fits the sheet to the pages.
    pub sheet: Option<Size>,
    /// Space between the cells, in mm.
    pub gutter: f64,
    /// Bleed kept around every page, in mm (0 for none).
    pub bleed: f64,
    /// Scale of the pages on a fitted sheet (1 = 100 %).
    pub scale: f64,
    /// The back of a double-sided sheet: the columns run right to left,
    /// so each page lands behind its front on the other side; the pages
    /// themselves are not mirrored (#387).
    pub back_side: bool,
    /// On a fixed sheet the pages keep their size, the marks taking the
    /// room there is (#388), and the grid keeps its columns and rows: what
    /// does not fit runs off the sheet instead of the pages shrinking
    /// (#496, see [`overflows`]).
    pub keep_size: bool,
    /// Where the page information sits, moved from its place (mm).
    pub info_offset: (f64, f64),
    /// Margins of the paper kept clear (mm: left, right, top, bottom):
    /// the pages and marks go inside them (#580).
    pub paper_margins: [f64; 4],
    /// Every page moved on the sheet (mm, x right and y up) and stretched
    /// about its middle (x and y factors, 1 for none), with its clip and
    /// marks: Print Preview's artwork position and Transform (#536, #595).
    pub page_offset: (f64, f64),
    pub page_stretch: (f64, f64),
    /// On a fixed sheet the pages stretch to fill the room inside the
    /// margins, width and height each on its own (#591).
    pub stretch_fit: bool,
    /// The binding, creep and gap of the Booklet layout.
    pub booklet: Booklet,
    pub marks: Marks,
    /// The drawing's name and the moment, for the page information.
    pub title: String,
    pub when: String,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            layout: Layout::AsIs,
            sheet: None,
            gutter: 0.0,
            bleed: 0.0,
            scale: 1.0,
            back_side: false,
            keep_size: false,
            info_offset: (0.0, 0.0),
            paper_margins: [0.0; 4],
            page_offset: (0.0, 0.0),
            page_stretch: (1.0, 1.0),
            stretch_fit: false,
            booklet: Booklet::default(),
            marks: Marks::default(),
            title: String::new(),
            when: String::new(),
        }
    }
}

/// Room around the pages for the marks, in mm (and the bleed beyond it).
pub const MARK_MARGIN: f64 = 12.0;

/// One page placed on a sheet: the page (`None` for a blank cell of a
/// booklet) and the map from page coordinates to the sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub page: Option<usize>,
    pub transform: Affine,
    /// The page's trimmed area on the sheet.
    pub trim: Rect,
}

/// A sheet: its size and the pages on it.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetPlan {
    pub size: Size,
    pub cells: Vec<Cell>,
}

/// The order of the pages of a saddle-stitched booklet, two per sheet
/// side (left, right): for eight pages 8 1, 2 7, 6 3, 4 5. Pages past the
/// end are blank.
#[cfg(test)]
pub fn booklet_order(pages: usize) -> Vec<(Option<usize>, Option<usize>)> {
    booklet_sides(pages, &Booklet::default())
        .into_iter()
        .map(|s| (s.left, s.right))
        .collect()
}

/// Plan the sheets for pages of `sizes`.
pub fn plan(sizes: &[Size], o: &Options) -> Vec<SheetPlan> {
    if sizes.is_empty() {
        return Vec::new();
    }
    let (cols, rows) = o.layout.grid();
    // A booklet's sides, and how far creep moves each side's pages
    // toward the spine.
    let sides = match o.layout {
        Layout::Booklet => booklet_sides(sizes.len(), &o.booklet),
        _ => Vec::new(),
    };
    let creep: Vec<f64> = sides
        .iter()
        .map(|s| {
            if o.booklet.binding != Binding::SaddleStitch || s.nested < 2 {
                0.0
            } else {
                o.booklet.creep_mm() * s.depth as f64 / (s.nested - 1) as f64
            }
        })
        .collect();
    // Which page goes in each cell, sheet by sheet.
    let sheets: Vec<Vec<Option<usize>>> = match o.layout {
        Layout::AsIs => (0..sizes.len()).map(|i| vec![Some(i)]).collect(),
        Layout::NUp { .. } => {
            let per = (cols * rows) as usize;
            (0..sizes.len())
                .collect::<Vec<_>>()
                .chunks(per)
                .map(|c| {
                    let mut v: Vec<Option<usize>> = c.iter().map(|&i| Some(i)).collect();
                    v.resize(per, None);
                    v
                })
                .collect()
        }
        Layout::Booklet => sides.iter().map(|s| vec![s.left, s.right]).collect(),
        Layout::Repeat { .. } => (0..sizes.len())
            .map(|i| vec![Some(i); (cols * rows) as usize])
            .collect(),
    };
    let margin = if o.marks.any() { MARK_MARGIN } else { 0.0 } + o.bleed.max(0.0);
    let pm = o
        .paper_margins
        .map(|m| if m.is_finite() { m.max(0.0) } else { 0.0 });
    let [ml, mr, mt, mb] = pm;
    // Booklet pages share the spine: no bleed between them, only the
    // booklet's gap.
    let (gutter, inner_bleed) = match o.layout {
        Layout::Booklet => (o.booklet.gap_mm(), 0.0),
        _ => (o.gutter.max(0.0), o.bleed.max(0.0)),
    };
    sheets
        .iter()
        .enumerate()
        .map(|(side, cells)| {
            let creep = creep.get(side).copied().unwrap_or(0.0);
            // The cell size: the largest page on this sheet.
            let cell = cells
                .iter()
                .flatten()
                .filter_map(|&i| sizes.get(i))
                .fold(Size::new(0.0, 0.0), |a, s| {
                    Size::new(a.width.max(s.width), a.height.max(s.height))
                });
            let cell = if cell.width > 0.0 && cell.height > 0.0 {
                cell
            } else {
                sizes[0]
            };
            // Pages with their bleed, and the gaps between them.
            let step_x = cell.width + 2.0 * inner_bleed + gutter;
            let step_y = cell.height + 2.0 * inner_bleed + gutter;
            let grid_w = cols as f64 * step_x - gutter;
            let grid_h = rows as f64 * step_y - gutter;
            let (size, (sx, sy)) = match o.sheet {
                Some(sheet) => {
                    // Kept size: the pages as they are; stretched: they
                    // fill the room each way; else the grid shrinks to fit
                    // inside the marks.
                    let avail_w = (sheet.width - ml - mr - 2.0 * margin).max(1.0);
                    let avail_h = (sheet.height - mt - mb - 2.0 * margin).max(1.0);
                    let s = if o.keep_size {
                        (1.0, 1.0)
                    } else if o.stretch_fit {
                        (avail_w / grid_w, avail_h / grid_h)
                    } else {
                        let s = (avail_w / grid_w).min(avail_h / grid_h).min(1.0);
                        (s, s)
                    };
                    (sheet, s)
                }
                None => {
                    let s = if o.scale.is_finite() && o.scale > 0.0 {
                        o.scale
                    } else {
                        1.0
                    };
                    (
                        Size::new(
                            grid_w * s + 2.0 * margin + ml + mr,
                            grid_h * s + 2.0 * margin + mt + mb,
                        ),
                        (s, s),
                    )
                }
            };
            // The grid centred in the paper's margins (the sheet when
            // there are none).
            let x0 = ml + (size.width - ml - mr - grid_w * sx) / 2.0;
            let y_top = size.height - mt - (size.height - mt - mb - grid_h * sy) / 2.0;
            // Moved and stretched about its middle as Print Preview asks.
            let (ox, oy) = o.page_offset;
            let (kx, ky) = o.page_stretch;
            let ok = |v: f64, d: f64| if v.is_finite() { v } else { d };
            let (ox, oy) = (ok(ox, 0.0), ok(oy, 0.0));
            let (kx, ky) = (ok(kx, 1.0).max(0.01), ok(ky, 1.0).max(0.01));
            let cells = cells
                .iter()
                .enumerate()
                .map(|(n, page)| {
                    let col = n as u32 % cols;
                    let col = if o.back_side { cols - 1 - col } else { col } as f64;
                    let row = (n as u32 / cols) as f64;
                    let ps = page.and_then(|i| sizes.get(i)).copied().unwrap_or(cell);
                    // Creep: the left page moves right, the right page left.
                    let toward_spine = if col == 0.0 { creep } else { -creep };
                    // The page centred in its cell (y up: row 0 at the top).
                    let cx = x0
                        + (col * step_x + inner_bleed + cell.width / 2.0 + toward_spine) * sx
                        + ox;
                    let cy = y_top - (row * step_y + inner_bleed + cell.height / 2.0) * sy + oy;
                    let (w, h) = (ps.width * sx * kx, ps.height * sy * ky);
                    let trim = Rect::new(cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0);
                    let transform = Affine::translate((trim.x0, trim.y0))
                        * Affine::scale_non_uniform(sx * kx, sy * ky);
                    Cell {
                        page: *page,
                        transform,
                        trim,
                    }
                })
                .collect();
            SheetPlan { size, cells }
        })
        .collect()
}

/// Whether a page of the plan runs off its sheet (pages kept at their
/// size in a grid larger than the sheet, #496).
pub fn overflows(plan: &[SheetPlan]) -> bool {
    const EPS: f64 = 1e-6;
    plan.iter().any(|sheet| {
        sheet.cells.iter().filter(|c| c.page.is_some()).any(|c| {
            c.trim.x0 < -EPS
                || c.trim.y0 < -EPS
                || c.trim.x1 > sheet.size.width + EPS
                || c.trim.y1 > sheet.size.height + EPS
        })
    })
}

/// A mark object on the sheet.
fn mark_shape(kind: ShapeKind, fill: Fill, stroke: Option<Stroke>) -> Shape {
    let mut s = Shape::new(ShapeId(0), kind);
    s.fill = fill;
    s.stroke = stroke;
    s
}

fn rect_shape(r: Rect, color: Color) -> Shape {
    mark_shape(
        ShapeKind::Rect {
            rect: r,
            radius: 0.0,
            corners: None,
        },
        Fill::Solid(color),
        None,
    )
}

/// Crop marks: at every cut line, short lines in the margin outside the
/// pages (so they never fall on a neighbouring page).
pub fn crop_marks(cells: &[Cell], bleed: f64) -> BezPath {
    let mut p = BezPath::new();
    let trims: Vec<Rect> = cells
        .iter()
        .filter(|c| c.page.is_some())
        .map(|c| c.trim)
        .collect();
    let Some(all) = trims.iter().copied().reduce(|a, b| a.union(b)) else {
        return p;
    };
    let gap = bleed.max(0.0) + 1.5;
    let len = 5.0;
    let mut xs: Vec<f64> = trims.iter().flat_map(|r| [r.x0, r.x1]).collect();
    let mut ys: Vec<f64> = trims.iter().flat_map(|r| [r.y0, r.y1]).collect();
    for v in [&mut xs, &mut ys] {
        v.sort_by(|a, b| a.total_cmp(b));
        v.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    }
    for x in xs {
        p.move_to((x, all.y1 + gap));
        p.line_to((x, all.y1 + gap + len));
        p.move_to((x, all.y0 - gap));
        p.line_to((x, all.y0 - gap - len));
    }
    for y in ys {
        p.move_to((all.x0 - gap, y));
        p.line_to((all.x0 - gap - len, y));
        p.move_to((all.x1 + gap, y));
        p.line_to((all.x1 + gap + len, y));
    }
    p
}

/// A registration target: a circle with a cross through it.
fn target(p: &mut BezPath, c: Point, r: f64) {
    p.extend(tracedraw_core::geometry::ellipse_path(Rect::new(
        c.x - r,
        c.y - r,
        c.x + r,
        c.y + r,
    )));
    p.move_to((c.x - r * 1.6, c.y));
    p.line_to((c.x + r * 1.6, c.y));
    p.move_to((c.x, c.y - r * 1.6));
    p.line_to((c.x, c.y + r * 1.6));
}

/// The marks of one sheet, as objects in sheet coordinates.
pub fn marks(
    sheet: &SheetPlan,
    o: &Options,
    sheet_no: usize,
    sheets: usize,
    pages: &[String],
) -> Vec<Shape> {
    let mut out = Vec::new();
    let m = o.marks;
    if !m.any() {
        return out;
    }
    let hair = |w: f64| Some(Stroke::new(Color::Registration, w));
    let content = sheet
        .cells
        .iter()
        .map(|c| c.trim)
        .reduce(|a, b| a.union(b))
        .unwrap_or(Rect::new(0.0, 0.0, sheet.size.width, sheet.size.height));
    let bleed = o.bleed.max(0.0);
    let band_top = content.y1 + bleed + MARK_MARGIN / 2.0;
    let band_bottom = content.y0 - bleed - MARK_MARGIN / 2.0;
    if m.crop {
        out.push(mark_shape(
            ShapeKind::Path {
                path: crop_marks(&sheet.cells, bleed),
                closed: false,
            },
            Fill::None,
            hair(0.1),
        ));
        // A fold mark at a booklet's spine.
        if o.layout == Layout::Booklet {
            if let (Some(a), Some(b)) = (sheet.cells.first(), sheet.cells.get(1)) {
                // The fold: half way between the pages (creep moves them
                // toward it by the same amount).
                let x = (a.trim.x1 + b.trim.x0) / 2.0;
                let mut p = BezPath::new();
                for (y0, y1) in [
                    (content.y1 + bleed + 1.5, content.y1 + bleed + 6.5),
                    (content.y0 - bleed - 1.5, content.y0 - bleed - 6.5),
                ] {
                    p.move_to((x, y0));
                    p.line_to((x, y1));
                }
                let mut fold = mark_shape(
                    ShapeKind::Path {
                        path: p,
                        closed: false,
                    },
                    Fill::None,
                    hair(0.1),
                );
                if let Some(s) = &mut fold.stroke {
                    s.dash = vec![1.0, 1.0];
                }
                out.push(fold);
            }
        }
    }
    if m.registration {
        let mut p = BezPath::new();
        let mid_x = (content.x0 + content.x1) / 2.0;
        let mid_y = (content.y0 + content.y1) / 2.0;
        let side = bleed + MARK_MARGIN / 2.0;
        for c in [
            Point::new(mid_x, content.y1 + side),
            Point::new(mid_x, content.y0 - side),
            Point::new(content.x0 - side, mid_y),
            Point::new(content.x1 + side, mid_y),
        ] {
            target(&mut p, c, 2.0);
        }
        out.push(mark_shape(
            ShapeKind::Path {
                path: p,
                closed: false,
            },
            Fill::None,
            hair(0.15),
        ));
    }
    let patch = 4.0;
    if m.color_bars {
        // Solids, then the two-ink overprints, to the left of the top
        // registration target.
        let colors = [
            Color::cmyk_pct(100.0, 0.0, 0.0, 0.0),
            Color::cmyk_pct(0.0, 100.0, 0.0, 0.0),
            Color::cmyk_pct(0.0, 0.0, 100.0, 0.0),
            Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
            Color::cmyk_pct(0.0, 100.0, 100.0, 0.0),
            Color::cmyk_pct(100.0, 0.0, 100.0, 0.0),
            Color::cmyk_pct(100.0, 100.0, 0.0, 0.0),
            Color::cmyk_pct(50.0, 0.0, 0.0, 0.0),
            Color::cmyk_pct(0.0, 50.0, 0.0, 0.0),
            Color::cmyk_pct(0.0, 0.0, 50.0, 0.0),
            Color::cmyk_pct(0.0, 0.0, 0.0, 50.0),
        ];
        let right = (content.x0 + content.x1) / 2.0 - 8.0;
        for (i, c) in colors.iter().enumerate() {
            let x = right - (colors.len() - i) as f64 * patch;
            out.push(rect_shape(
                Rect::new(x, band_top - patch / 2.0, x + patch, band_top + patch / 2.0),
                *c,
            ));
        }
    }
    if m.densitometer {
        // Black from 0 to 100 % in steps of ten, right of the top target.
        let left = (content.x0 + content.x1) / 2.0 + 8.0;
        for i in 0..=10 {
            let x = left + i as f64 * patch;
            let mut s = rect_shape(
                Rect::new(x, band_top - patch / 2.0, x + patch, band_top + patch / 2.0),
                Color::cmyk_pct(0.0, 0.0, 0.0, i as f32 * 10.0),
            );
            s.stroke = Some(Stroke::new(Color::cmyk_pct(0.0, 0.0, 0.0, 40.0), 0.05));
            out.push(s);
        }
    }
    if m.page_numbers {
        for c in &sheet.cells {
            let Some(i) = c.page else { continue };
            out.push(mark_shape(
                ShapeKind::Text {
                    spans: vec![TextSpan::new((i + 1).to_string(), "DejaVu Sans", 6.0)],
                    origin: Point::new(c.trim.center().x, c.trim.y0 - bleed - 3.5),
                    frame: None,
                    align: tracedraw_core::document::TextAlign::Center,
                    para: Default::default(),
                    on_path: None,
                },
                Fill::Solid(Color::Registration),
                None,
            ));
        }
    }
    if m.page_info {
        let on_sheet: Vec<String> = sheet
            .cells
            .iter()
            .filter_map(|c| c.page)
            .filter_map(|i| pages.get(i).cloned())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let text = crate::i18n::trf(
            "print.page_info",
            &[
                ("title", &o.title),
                ("pages", &on_sheet.join(", ")),
                ("sheet", &(sheet_no + 1).to_string()),
                ("sheets", &sheets.to_string()),
                ("when", &o.when),
            ],
        );
        out.push(mark_shape(
            ShapeKind::Text {
                spans: vec![TextSpan::new(text, "DejaVu Sans", 6.0)],
                origin: Point::new(
                    content.x0 + o.info_offset.0,
                    band_bottom - 1.0 + o.info_offset.1,
                ),
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
            Fill::Solid(Color::Registration),
            None,
        ));
    }
    out
}

/// The marks of every page printed on its own (the layout as in the
/// document, the sheet fitted to the page), in each page's own
/// coordinates: they lie outside the page, in its bleed and the
/// [`MARK_MARGIN`] around it. A PDF export draws them there (#789).
/// `names` are the pages' names for the page information.
pub fn page_marks(sizes: &[Size], names: &[String], o: &Options) -> Vec<Vec<Shape>> {
    let o = Options {
        layout: Layout::AsIs,
        sheet: None,
        scale: 1.0,
        page_offset: (0.0, 0.0),
        page_stretch: (1.0, 1.0),
        paper_margins: [0.0; 4],
        ..o.clone()
    };
    let plans = plan(sizes, &o);
    let total = plans.len();
    plans
        .iter()
        .enumerate()
        .map(|(n, sheet)| {
            let Some(cell) = sheet.cells.first() else {
                return Vec::new();
            };
            let to_page = Affine::translate((-cell.trim.x0, -cell.trim.y0));
            let mut shapes = marks(sheet, &o, n, total, names);
            for s in &mut shapes {
                s.transform = to_page * s.transform;
            }
            shapes
        })
        .collect()
}

/// Build the printed sheets of `doc` as a new document: each sheet one
/// page, the document's pages placed and clipped to their bleed, the marks
/// on top. Hidden and non-printing layers are left out; master layers go
/// with every page they show on.
pub fn impose(doc: &Document, o: &Options) -> Document {
    let sizes: Vec<Size> = doc.pages.iter().map(|p| p.size).collect();
    let names: Vec<String> = (1..=doc.pages.len()).map(|n| n.to_string()).collect();
    let plans = plan(&sizes, o);
    let mut out = Document::new(
        doc.title.clone(),
        plans
            .first()
            .map(|p| p.size)
            .unwrap_or(Size::new(210.0, 297.0)),
    );
    out.metadata = doc.metadata.clone();
    out.metadata.bleed = 0.0;
    out.symbols = doc.symbols.clone();
    // The spot inks: separations make a plate for each, and the print
    // preview a box (#779).
    out.inks = doc.inks.clone();
    out.pages.clear();
    out.master.clear();
    let bleed = o.bleed.max(0.0);
    let total = plans.len();
    for (n, sheet) in plans.iter().enumerate() {
        let mut layer = Layer::new(out.ids_mut().layer(), "Sheet");
        for cell in &sheet.cells {
            let Some(pi) = cell.page else {
                continue;
            };
            let Some(page) = doc.pages.get(pi) else {
                continue;
            };
            let mut contents = Vec::new();
            if let Some(bg) = &page.background {
                contents.push(mark_shape(
                    ShapeKind::Rect {
                        rect: Rect::new(
                            -bleed,
                            -bleed,
                            page.size.width + bleed,
                            page.size.height + bleed,
                        ),
                        radius: 0.0,
                        corners: None,
                    },
                    bg.clone(),
                    None,
                ));
            }
            for l in doc.layers_drawn_for_page(page.id).unwrap_or_default() {
                if l.visible && l.printable {
                    contents.extend(l.shapes.iter().filter(|s| s.visible).cloned());
                }
            }
            // The page and its bleed, nothing beyond.
            let frame = mark_shape(
                ShapeKind::Rect {
                    rect: Rect::new(
                        -bleed,
                        -bleed,
                        page.size.width + bleed,
                        page.size.height + bleed,
                    ),
                    radius: 0.0,
                    corners: None,
                },
                Fill::None,
                None,
            );
            let mut clip = mark_shape(
                ShapeKind::ClipFrame {
                    frame: Box::new(frame),
                    contents,
                },
                Fill::None,
                None,
            );
            clip.transform = cell.transform;
            layer.shapes.push(clip);
        }
        layer.shapes.extend(marks(sheet, o, n, total, &names));
        let page = Page {
            id: out.ids_mut().page(),
            name: format!("{}", n + 1),
            size: sheet.size,
            layers: vec![layer],
            guides: Vec::new(),
            background: None,
        };
        out.pages.push(page);
    }
    // Every object needs its own id in the new document.
    let mut ids = out.ids().clone();
    for page in out.pages.iter_mut() {
        for layer in page.layers.iter_mut() {
            for s in layer.shapes.iter_mut() {
                renumber(s, &mut ids);
            }
        }
    }
    out.set_ids(ids);
    out
}

fn renumber(s: &mut Shape, ids: &mut tracedraw_core::id::IdSource) {
    s.id = ids.shape();
    match &mut s.kind {
        ShapeKind::Group { children } => children.iter_mut().for_each(|c| renumber(c, ids)),
        ShapeKind::ClipFrame { frame, contents } => {
            renumber(frame, ids);
            contents.iter_mut().for_each(|c| renumber(c, ids));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The paper's margins are kept clear: the page sits in the middle of
    /// what is left, and shrinks to fit inside it when it would not; a
    /// fitted sheet grows by them (#580).
    #[test]
    fn pages_sit_inside_the_paper_margins() {
        let a4 = Size::new(210.0, 297.0);
        let o = Options {
            sheet: Some(a4),
            paper_margins: [20.0, 0.0, 10.0, 30.0],
            ..Default::default()
        };
        let one = plan(&[Size::new(100.0, 100.0)], &o);
        let t = one[0].cells[0].trim;
        assert!(
            (t.center().x - (20.0 + (210.0 - 20.0) / 2.0)).abs() < 1e-9,
            "{t:?}"
        );
        // y up: 30 at the bottom, 10 at the top.
        assert!(
            (t.center().y - (30.0 + (297.0 - 10.0 - 30.0) / 2.0)).abs() < 1e-9,
            "{t:?}"
        );
        // A page as large as the paper shrinks to the room inside.
        let full = plan(&[a4], &o);
        let t = full[0].cells[0].trim;
        assert!(
            t.x0 >= 20.0 - 1e-9 && t.y0 >= 30.0 - 1e-9 && t.y1 <= 297.0 - 10.0 + 1e-9,
            "{t:?}"
        );
        let fitted = plan(
            &[Size::new(100.0, 100.0)],
            &Options {
                paper_margins: [5.0, 5.0, 5.0, 5.0],
                ..Default::default()
            },
        );
        assert_eq!(fitted[0].size, Size::new(110.0, 110.0));
    }

    /// Stretched to fill, the page takes the whole room inside the
    /// margins, each way on its own (#591); moved and stretched in Print
    /// Preview, the trim moves by the offset and grows about its middle
    /// (#595), and the page's transform follows it.
    #[test]
    fn pages_stretch_to_fill_and_follow_the_preview_transform() {
        let a4 = Size::new(210.0, 297.0);
        let page = Size::new(100.0, 50.0);
        let o = Options {
            sheet: Some(a4),
            paper_margins: [10.0, 10.0, 20.0, 20.0],
            stretch_fit: true,
            ..Default::default()
        };
        let t = plan(&[page], &o)[0].cells[0].trim;
        let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(near(t.x0, 10.0) && near(t.x1, 200.0), "{t:?}");
        assert!(near(t.y0, 20.0) && near(t.y1, 277.0), "{t:?}");
        let base = Options {
            sheet: Some(a4),
            ..Default::default()
        };
        let before = plan(&[page], &base)[0].cells[0].clone();
        let moved = plan(
            &[page],
            &Options {
                page_offset: (5.0, -3.0),
                page_stretch: (2.0, 1.0),
                ..base.clone()
            },
        )[0]
        .cells[0]
            .clone();
        let (b, m) = (before.trim, moved.trim);
        assert!(near(m.center().x, b.center().x + 5.0), "{m:?}");
        assert!(near(m.center().y, b.center().y - 3.0), "{m:?}");
        assert!(near(m.width(), 2.0 * b.width()) && near(m.height(), b.height()));
        // The page's corners land on the trim's.
        let far = moved.transform * Point::new(page.width, page.height);
        assert!(near(far.x, m.x1) && near(far.y, m.y1), "{far:?} {m:?}");
    }

    const A5: Size = Size {
        width: 148.0,
        height: 210.0,
    };

    /// Perfect binding folds each signature on its own: with one sheet
    /// per signature the first sheet holds pages 1 to 4 and the second 5
    /// to 8; with two sheets per signature the sixteen pages go 1 to 8,
    /// then 9 to 16 (#788, #947).
    #[test]
    fn perfect_binding_folds_each_signature_on_its_own() {
        let pairs = |n, sheets| {
            let b = Booklet {
                binding: Binding::PerfectBound,
                signature_sheets: sheets,
                ..Default::default()
            };
            booklet_sides(n, &b)
                .into_iter()
                .map(|s| (s.left.map(|i| i + 1), s.right.map(|i| i + 1)))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            pairs(8, 1),
            vec![
                (Some(4), Some(1)),
                (Some(2), Some(3)),
                (Some(8), Some(5)),
                (Some(6), Some(7))
            ]
        );
        let two = pairs(16, 2);
        assert_eq!(two.len(), 8);
        assert_eq!(two[0], (Some(8), Some(1)));
        assert_eq!(two[3], (Some(4), Some(5)));
        assert_eq!(two[4], (Some(16), Some(9)));
        // Ten pages in signatures of two sheets: twelve pages, the last
        // signature of one sheet, two blanks at the back.
        let ten = pairs(10, 2);
        assert_eq!(ten.len(), 6);
        assert_eq!(ten[4], (None, Some(9)));
        assert_eq!(ten[5], (Some(10), None));
        let info = booklet_info(
            10,
            &Booklet {
                binding: Binding::PerfectBound,
                signature_sheets: 2,
                ..Default::default()
            },
        );
        assert_eq!(
            info,
            BookletInfo {
                pages: 10,
                blanks: 2,
                sheets: 3,
                signatures: 2
            }
        );
        let saddle = booklet_info(6, &Booklet::default());
        assert_eq!((saddle.blanks, saddle.sheets, saddle.signatures), (2, 2, 1));
    }

    /// The gap opens between the two pages of a spread, centred on the
    /// fold; creep moves the pages of the inner sheets toward the spine,
    /// the innermost by the creep, the outer one not at all (#788).
    #[test]
    fn booklet_gap_and_creep_move_the_pages() {
        let page = Size::new(100.0, 150.0);
        let o = Options {
            layout: Layout::Booklet,
            booklet: Booklet {
                gap: 10.0,
                creep: 2.0,
                ..Default::default()
            },
            ..Default::default()
        };
        // 12 pages: three sheets, six sides.
        let p = plan(&[page; 12], &o);
        assert_eq!(p.len(), 6);
        assert!((p[0].size.width - 210.0).abs() < 1e-9, "{:?}", p[0].size);
        let spine = p[0].size.width / 2.0;
        let gap_of = |s: &SheetPlan| s.cells[1].trim.x0 - s.cells[0].trim.x1;
        assert!((gap_of(&p[0]) - 10.0).abs() < 1e-9, "outer sheet: no creep");
        // The middle sheet moves by 1 mm a page, the inner by 2 mm.
        assert!((gap_of(&p[2]) - 8.0).abs() < 1e-9, "{}", gap_of(&p[2]));
        assert!((gap_of(&p[5]) - 6.0).abs() < 1e-9, "{}", gap_of(&p[5]));
        for s in &p {
            let mid = (s.cells[0].trim.x1 + s.cells[1].trim.x0) / 2.0;
            assert!((mid - spine).abs() < 1e-9, "pages centred on the fold");
            // The page drawn where its trim is.
            let c = &s.cells[0];
            let at = c.transform * Point::new(0.0, 0.0);
            assert!((at.x - c.trim.x0).abs() < 1e-9);
        }
        // Perfect binding takes no creep.
        let o = Options {
            booklet: Booklet {
                binding: Binding::PerfectBound,
                ..o.booklet
            },
            ..o
        };
        let p = plan(&[page; 12], &o);
        assert!(p.iter().all(|s| (gap_of(s) - 10.0).abs() < 1e-9));
        // Nonsense values stay harmless.
        let o = Options {
            booklet: Booklet {
                gap: f64::NAN,
                creep: -3.0,
                signature_sheets: 0,
                binding: Binding::PerfectBound,
            },
            ..o
        };
        let p = plan(&[page; 5], &o);
        assert_eq!(p.len(), 4);
        assert!(p.iter().all(|s| gap_of(s).abs() < 1e-9));
    }

    #[test]
    fn booklets_pair_the_pages_for_folding() {
        let pairs = |n| {
            booklet_order(n)
                .into_iter()
                .map(|(a, b)| (a.map(|i| i + 1), b.map(|i| i + 1)))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            pairs(8),
            vec![
                (Some(8), Some(1)),
                (Some(2), Some(7)),
                (Some(6), Some(3)),
                (Some(4), Some(5))
            ]
        );
        // Six pages fill eight with two blanks at the back.
        assert_eq!(
            pairs(6),
            vec![
                (None, Some(1)),
                (Some(2), None),
                (Some(6), Some(3)),
                (Some(4), Some(5))
            ]
        );
        assert_eq!(pairs(1), vec![(None, Some(1)), (None, None)]);
    }

    #[test]
    fn n_up_fills_sheets_in_reading_order_and_scales_to_fit() {
        let o = Options {
            layout: Layout::NUp { cols: 2, rows: 2 },
            sheet: Some(Size::new(297.0, 420.0)),
            gutter: 0.0,
            ..Default::default()
        };
        let sheets = plan(&[A5; 5], &o);
        assert_eq!(sheets.len(), 2);
        let pages: Vec<Option<usize>> = sheets[0].cells.iter().map(|c| c.page).collect();
        assert_eq!(pages, vec![Some(0), Some(1), Some(2), Some(3)]);
        assert_eq!(
            sheets[1].cells.iter().filter(|c| c.page.is_some()).count(),
            1
        );
        // Four A5 pages fit an A3 sheet at full size (296 x 420): page 1
        // top left, page 4 bottom right.
        let c0 = &sheets[0].cells[0];
        let c3 = &sheets[0].cells[3];
        assert!((c0.trim.width() - 148.0).abs() < 1e-6);
        assert!(c0.trim.x0 < c3.trim.x0 && c0.trim.y0 > c3.trim.y0);
        // On A4 they shrink by the square root of a half (A-sizes halve in
        // area), to fit the sheet's height.
        let o = Options {
            sheet: Some(Size::new(210.0, 297.0)),
            ..o
        };
        let s = plan(&[A5; 4], &o);
        let k = 297.0 / 420.0;
        let t = s[0].cells[0].trim;
        assert!((t.width() - 148.0 * k).abs() < 1e-6, "{t:?}");
        assert!((t.y1 - 297.0).abs() < 1e-6, "{t:?}");
    }

    #[test]
    fn fitted_sheets_grow_with_marks_bleed_and_gutters() {
        let o = Options {
            layout: Layout::Repeat { cols: 3, rows: 2 },
            gutter: 2.0,
            bleed: 3.0,
            marks: Marks {
                crop: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let card = Size::new(85.0, 55.0);
        let sheets = plan(&[card], &o);
        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].cells.len(), 6);
        assert!(sheets[0].cells.iter().all(|c| c.page == Some(0)));
        // 3 cards of 85 + 2*3 bleed, 2 gutters, 2 margins of 12 + 3.
        let w = 3.0 * 91.0 + 2.0 * 2.0 + 2.0 * 15.0;
        assert!(
            (sheets[0].size.width - w).abs() < 1e-6,
            "{:?}",
            sheets[0].size
        );
        // Crop marks at every cut: 6 vertical and 4 horizontal cut lines,
        // each with two marks.
        let marks = crop_marks(&sheets[0].cells, 3.0);
        let segments = marks
            .elements()
            .iter()
            .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
            .count();
        assert_eq!(segments, (6 + 4) * 2);
    }

    #[test]
    fn imposing_places_and_clips_every_page() {
        let mut doc = Document::new("t", A5);
        let id = doc.ids_mut().shape();
        let s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 20.0, 20.0),
                radius: 0.0,
                corners: None,
            },
        );
        doc.pages[0].layers[0].shapes.push(s);
        let o = Options {
            layout: Layout::NUp { cols: 2, rows: 1 },
            marks: Marks {
                crop: true,
                registration: true,
                color_bars: true,
                densitometer: true,
                page_info: true,
                page_numbers: true,
            },
            title: "t".into(),
            when: "now".into(),
            ..Default::default()
        };
        let out = impose(&doc, &o);
        assert_eq!(out.pages.len(), 1);
        let shapes = &out.pages[0].layers[0].shapes;
        // The page in a clip frame, then the marks: crop marks,
        // registration, 11 colour patches, 11 density steps, the page
        // number, the text.
        assert!(matches!(shapes[0].kind, ShapeKind::ClipFrame { .. }));
        assert_eq!(shapes.len(), 1 + 1 + 1 + 11 + 11 + 1 + 1);
        // Ids are all different.
        let mut seen = std::collections::HashSet::new();
        fn walk(s: &Shape, seen: &mut std::collections::HashSet<ShapeId>) -> bool {
            let fresh = seen.insert(s.id);
            match &s.kind {
                ShapeKind::ClipFrame { frame, contents } => {
                    fresh && walk(frame, seen) && contents.iter().all(|c| walk(c, seen))
                }
                _ => fresh,
            }
        }
        assert!(shapes.iter().all(|s| walk(s, &mut seen)));
        // A sheet the PDF writer accepts.
        let pdf = tracedraw_io::pdf::document_to_pdf(&out);
        assert!(pdf.starts_with(b"%PDF"));
    }

    /// The marks of each page on its own, in the page's coordinates: crop
    /// marks start beyond the bleed and stay inside the marks margin, the
    /// page information names the page (#789).
    #[test]
    fn page_marks_sit_around_each_page_in_its_own_coordinates() {
        let sizes = [Size::new(100.0, 50.0), Size::new(60.0, 60.0)];
        let names = vec!["Front".to_string(), "Back".to_string()];
        let o = Options {
            bleed: 3.0,
            // Ignored: each page is on its own sheet at its own size.
            layout: Layout::NUp { cols: 2, rows: 2 },
            page_offset: (40.0, 40.0),
            marks: Marks {
                crop: true,
                registration: true,
                color_bars: true,
                page_info: true,
                ..Default::default()
            },
            title: "Card".into(),
            when: "today".into(),
            ..Default::default()
        };
        let all = page_marks(&sizes, &names, &o);
        assert_eq!(all.len(), 2);
        for (marks, size) in all.iter().zip(sizes) {
            // Crop marks, registration, 11 colour patches, the text.
            assert_eq!(marks.len(), 1 + 1 + 11 + 1);
            let page = Rect::from_origin_size((0.0, 0.0), size);
            let room = page.inflate(3.0 + MARK_MARGIN, 3.0 + MARK_MARGIN);
            let bleed = page.inflate(3.0, 3.0);
            let crop = marks[0]
                .transform
                .transform_rect_bbox(match &marks[0].kind {
                    ShapeKind::Path { path, .. } => kurbo::Shape::bounding_box(path),
                    _ => panic!("crop marks are a path"),
                });
            assert!(
                crop.x0 >= room.x0 - 1e-9 && crop.x1 <= room.x1 + 1e-9,
                "{crop:?}"
            );
            assert!(
                crop.y0 >= room.y0 - 1e-9 && crop.y1 <= room.y1 + 1e-9,
                "{crop:?}"
            );
            assert!(crop.x0 < bleed.x0 && crop.y1 > bleed.y1);
        }
        let ShapeKind::Text { spans, origin, .. } = &all[1][13].kind else {
            panic!("the page information is text");
        };
        assert!(spans[0].text.contains("Card") && spans[0].text.contains("Back"));
        let at = all[1][13].transform * *origin;
        assert!(at.y < -3.0 && at.y > -3.0 - MARK_MARGIN, "{at:?}");
        // No marks asked for: none.
        let none = page_marks(&sizes, &names, &Options::default());
        assert!(none.iter().all(|m| m.is_empty()));
    }

    /// The back of a double-sided sheet: the columns run right to left so
    /// each card prints behind its front, and nothing is mirrored (#387).
    #[test]
    fn the_back_side_runs_the_columns_right_to_left() {
        let card = Size::new(85.0, 55.0);
        let sizes = vec![card; 3];
        let o = Options {
            layout: Layout::NUp { cols: 3, rows: 1 },
            sheet: Some(Size::new(297.0, 210.0)),
            gutter: 5.0,
            ..Default::default()
        };
        let front = plan(&sizes, &o);
        let back = plan(
            &sizes,
            &Options {
                back_side: true,
                ..o.clone()
            },
        );
        let x_of = |p: &SheetPlan, page: usize| {
            p.cells
                .iter()
                .find(|c| c.page == Some(page))
                .map(|c| c.trim.center().x)
                .expect("placed")
        };
        let w = front[0].size.width;
        for page in 0..3 {
            // Mirror places on the sheet: what is left on the front is
            // right on the back.
            assert!((x_of(&front[0], page) + x_of(&back[0], page) - w).abs() < 1e-9);
        }
        assert!(x_of(&back[0], 0) > x_of(&back[0], 2));
        // The pages themselves are not mirrored.
        assert!(back[0]
            .cells
            .iter()
            .all(|c| c.transform.as_coeffs()[0] > 0.0));
    }

    /// Keep size: on a fixed sheet the pages are not shrunk for the marks'
    /// room; the page information moves by its offset (#388).
    #[test]
    fn kept_size_does_not_shrink_pages_for_marks() {
        let page = Size::new(200.0, 280.0);
        let base = Options {
            sheet: Some(Size::new(210.0, 297.0)),
            marks: Marks {
                crop: true,
                page_info: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let shrunk = plan(&[page], &base);
        let kept = plan(
            &[page],
            &Options {
                keep_size: true,
                ..base.clone()
            },
        );
        let w = |p: &[SheetPlan]| p[0].cells[0].trim.width();
        assert!(w(&shrunk) < 199.0, "{}", w(&shrunk));
        assert!((w(&kept) - 200.0).abs() < 1e-9);
        assert!(!overflows(&kept));
        // The page information where the drag put it.
        let origin = |o: &Options| {
            let sheets = plan(&[page], o);
            marks(&sheets[0], o, 0, 1, &["1".to_string()])
                .into_iter()
                .find_map(|s| match s.kind {
                    ShapeKind::Text { origin, .. } => Some(origin),
                    _ => None,
                })
                .expect("page info")
        };
        let a = origin(&base);
        let b = origin(&Options {
            info_offset: (15.0, 40.0),
            ..base.clone()
        });
        assert!(((b.x - a.x) - 15.0).abs() < 1e-9 && ((b.y - a.y) - 40.0).abs() < 1e-9);
    }

    /// Keep size with more columns and rows than fit: the grid stays as
    /// set and the pages keep their size, running off the sheet, which
    /// `overflows` tells; without it the pages shrink to fit (#496).
    #[test]
    fn kept_size_keeps_the_grid_and_the_page_size() {
        let page = Size::new(90.0, 50.0);
        let base = Options {
            layout: Layout::NUp { cols: 3, rows: 7 },
            sheet: Some(Size::new(210.0, 297.0)),
            ..Default::default()
        };
        let pages = vec![page; 21];
        let shrunk = plan(&pages, &base);
        assert_eq!(shrunk.len(), 1);
        assert_eq!(shrunk[0].cells.len(), 21);
        assert!(shrunk[0].cells[0].trim.width() < 89.0);
        assert!(!overflows(&shrunk));
        let kept = plan(
            &pages,
            &Options {
                keep_size: true,
                ..base.clone()
            },
        );
        assert_eq!(kept.len(), 1, "the grid as set: one sheet");
        assert_eq!(kept[0].cells.len(), 21);
        assert!(
            kept[0]
                .cells
                .iter()
                .all(|c| (c.trim.width() - 90.0).abs() < 1e-9
                    && (c.trim.height() - 50.0).abs() < 1e-9)
        );
        assert!(overflows(&kept));
        // A grid that fits keeps its pages on the sheet.
        let fits = plan(
            &vec![page; 4],
            &Options {
                layout: Layout::NUp { cols: 2, rows: 2 },
                keep_size: true,
                ..base
            },
        );
        assert!(!overflows(&fits));
    }
}
