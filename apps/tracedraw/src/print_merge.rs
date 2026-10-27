//! Print Merge: the records, their fields and the merge itself (see
//! `docs/behavior/print-merge.md`). The dialog edits a
//! [`PrintMergeState`]; [`perform`] turns the current page into one page
//! per record.

use crate::app::App;
use crate::i18n::trf;
use crate::ui::dialogs::PrintMergeState;
use tracedraw_core::geometry::{Rect, Shape as _};
use tracedraw_core::{Color, Command, Fill, LayerId, Shape, ShapeKind, TextSpan};

/// What a field holds.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FieldKind {
    /// Text put in place of `<Field>` in every text object.
    #[default]
    Text,
    /// A counter: `start`, then `step` more each record, written in
    /// `format` (numbers padded with zeros to `digits` digits); its cells
    /// are not typed in.
    Number {
        start: i64,
        step: i64,
        digits: u32,
        format: NumberFormat,
    },
    /// The path of a picture (relative paths from the data file's folder);
    /// a text object holding only `<Field>` becomes the picture, fitted in
    /// its box.
    Image,
    /// A QR code of the cell's text in place of a text object holding only
    /// `<Field>`, as large as the box allows.
    Qr,
}

/// How a number field writes its counter: the Format of Add Column and
/// Configure Columns (#930).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum NumberFormat {
    /// 1, 2, 3, padded with zeros to the field's digits.
    #[default]
    Digits,
    /// A, B, C ... Z, AA, AB: 1 is A, 26 is Z, 27 is AA.
    Upper,
    /// a, b, c ... z, aa, ab, the same in small letters.
    Lower,
    /// The number padded as for `Digits`, between a text of the user's
    /// own before it and one after it (tickets "T-0001", serial numbers).
    Custom { prefix: String, suffix: String },
}

impl NumberFormat {
    /// The index in the Format list: 0 digits, 1 capitals, 2 small
    /// letters, 3 custom.
    pub fn index(&self) -> usize {
        match self {
            NumberFormat::Digits => 0,
            NumberFormat::Upper => 1,
            NumberFormat::Lower => 2,
            NumberFormat::Custom { .. } => 3,
        }
    }

    /// The format at `index` of the Format list; a custom one starts with
    /// no text around the number.
    pub fn from_index(index: usize) -> NumberFormat {
        match index {
            1 => NumberFormat::Upper,
            2 => NumberFormat::Lower,
            3 => NumberFormat::Custom {
                prefix: String::new(),
                suffix: String::new(),
            },
            _ => NumberFormat::Digits,
        }
    }

    /// The i18n keys of the Format list, in index order.
    pub const KEYS: [&'static str; 4] = [
        "merge.format_digits",
        "merge.format_upper",
        "merge.format_lower",
        "merge.format_custom",
    ];

    /// `n` written in this format, numbers padded to `digits` digits.
    /// Letters count from 1 (A); 0 and below have no letters and are
    /// written as numbers.
    pub fn write(&self, n: i64, digits: u32) -> String {
        let number = || {
            let width = digits.min(30) as usize;
            if n < 0 {
                format!("-{:0width$}", n.unsigned_abs())
            } else {
                format!("{n:0width$}")
            }
        };
        match self {
            NumberFormat::Digits => number(),
            NumberFormat::Upper if n > 0 => letters(n.unsigned_abs(), b'A'),
            NumberFormat::Lower if n > 0 => letters(n.unsigned_abs(), b'a'),
            NumberFormat::Upper | NumberFormat::Lower => number(),
            NumberFormat::Custom { prefix, suffix } => format!("{prefix}{}{suffix}", number()),
        }
    }
}

/// `n` (1 or more) in letters from `first`: A to Z, then AA to ZZ, AAA...
fn letters(mut n: u64, first: u8) -> String {
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push(first + (n % 26) as u8);
        n /= 26;
    }
    out.reverse();
    String::from_utf8_lossy(&out).into_owned()
}

/// How a picture sits in the placeholder of a picture field: the Print
/// Merge docker's Align and scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scale {
    /// At its own size (its resolution, else 96 dpi), placed by the anchor.
    Actual,
    /// As large as fits inside, keeping its proportions.
    #[default]
    Fit,
    /// As small as covers the placeholder, keeping its proportions; the
    /// placeholder cuts what is outside.
    Fill,
    /// Stretched to the placeholder.
    Stretch,
}

impl Scale {
    pub const ALL: [Scale; 4] = [Scale::Actual, Scale::Fit, Scale::Fill, Scale::Stretch];

    /// How it is kept in the placeholder's object data.
    pub fn id(self) -> &'static str {
        match self {
            Scale::Actual => "actual",
            Scale::Fit => "fit",
            Scale::Fill => "fill",
            Scale::Stretch => "stretch",
        }
    }

    pub fn from_id(id: &str) -> Option<Scale> {
        Scale::ALL.into_iter().find(|s| s.id() == id)
    }

    /// i18n key of its name.
    pub fn key(self) -> &'static str {
        match self {
            Scale::Actual => "merge.scale_actual",
            Scale::Fit => "merge.scale_fit",
            Scale::Fill => "merge.scale_fill",
            Scale::Stretch => "merge.scale_stretch",
        }
    }
}

/// The object data names a placeholder keeps its Align and scale under.
pub const SCALE_DATA: &str = "PrintMerge.Scale";
pub const ANCHOR_DATA: &str = "PrintMerge.Anchor";

/// The object data a copy of a field made by On every page keeps: the id
/// of the field object on the page it was copied from, and the box the
/// copy was last put in ("x0 y0 x1 y1", mm). While the copy is still in
/// that box (not moved or resized by hand) it follows its original (#956).
pub const COPY_OF_DATA: &str = "PrintMerge.CopyOf";
pub const COPY_AT_DATA: &str = "PrintMerge.CopyAt";

/// `data` with the link of a copy of `original` put in `at`, replacing
/// any link it had.
pub fn copy_data(
    data: &[(String, String)],
    original: tracedraw_core::ShapeId,
    at: Rect,
) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = data
        .iter()
        .filter(|(k, _)| k != COPY_OF_DATA && k != COPY_AT_DATA)
        .cloned()
        .collect();
    out.push((COPY_OF_DATA.to_string(), original.raw().to_string()));
    out.push((
        COPY_AT_DATA.to_string(),
        format!("{} {} {} {}", at.x0, at.y0, at.x1, at.y1),
    ));
    out
}

/// The original a copy made by On every page follows and the box it was
/// put in; `None` for any other object or unreadable data.
pub fn copy_link(s: &Shape) -> Option<(tracedraw_core::ShapeId, Rect)> {
    let get = |k: &str| s.data.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    let id = get(COPY_OF_DATA)?.trim().parse::<u64>().ok()?;
    let v: Vec<f64> = get(COPY_AT_DATA)?
        .split_whitespace()
        .map(|t| t.parse::<f64>())
        .collect::<Result<_, _>>()
        .ok()?;
    match v.as_slice() {
        [x0, y0, x1, y1] if v.iter().all(|n| n.is_finite()) => {
            Some((tracedraw_core::ShapeId(id), Rect::new(*x0, *y0, *x1, *y1)))
        }
        _ => None,
    }
}

/// Two boxes the same within 0.01 mm.
pub fn same_box(a: Rect, b: Rect) -> bool {
    (a.x0 - b.x0).abs() < 0.01
        && (a.y0 - b.y0).abs() < 0.01
        && (a.x1 - b.x1).abs() < 0.01
        && (a.y1 - b.y1).abs() < 0.01
}

/// The move (and scale) that takes box `from` onto box `to`.
pub fn box_to_box(from: Rect, to: Rect) -> tracedraw_core::geometry::Affine {
    use tracedraw_core::geometry::Affine;
    let k = |a: f64, b: f64| {
        if a.abs() > 1e-9 && b.abs() > 1e-9 {
            b / a
        } else {
            1.0
        }
    };
    Affine::translate((to.x0, to.y0))
        * Affine::scale_non_uniform(k(from.width(), to.width()), k(from.height(), to.height()))
        * Affine::translate((-from.x0, -from.y0))
}

/// The commands that put `copy` (a copy made by On every page) in the box
/// of its original `original`, when it is still where it was put: the
/// field moved or resized on its page after it was copied moves the same
/// on the other pages; a copy moved by hand stays where it is (#956).
pub fn follow(copy: &Shape, original: &Shape) -> Vec<Command> {
    let Some((of, at)) = copy_link(copy) else {
        return Vec::new();
    };
    let (from, to) = (copy.bounds(), original.bounds());
    if of != original.id
        || copy.id == original.id
        || !same_box(from, at)
        || same_box(from, to)
        || ![to.x0, to.y0, to.x1, to.y1].iter().all(|v| v.is_finite())
    {
        return Vec::new();
    }
    vec![
        Command::TransformShapes {
            shapes: vec![copy.id],
            transform: box_to_box(from, to),
        },
        Command::SetObjectData {
            shape: copy.id,
            data: copy_data(&copy.data, of, to),
        },
    ]
}

/// [`follow`] for every copy on the drawing's pages whose original is
/// still there.
pub fn follow_originals(doc: &tracedraw_core::Document) -> Vec<Command> {
    let mut cmds = Vec::new();
    for s in doc
        .pages
        .iter()
        .flat_map(|p| p.layers.iter().flat_map(|l| l.shapes.iter()))
    {
        let Some((of, _)) = copy_link(s) else {
            continue;
        };
        if let Some(o) = doc.find_shape(of) {
            cmds.extend(follow(s, o));
        }
    }
    cmds
}

/// A placeholder's Align and scale: its object data, else (placeholders
/// made before the docker) filling it from the centre.
pub fn placement_of(s: &Shape) -> (Scale, u8) {
    let get = |k: &str| s.data.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    let scale = get(SCALE_DATA)
        .and_then(Scale::from_id)
        .unwrap_or(Scale::Fill);
    let anchor = get(ANCHOR_DATA)
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|a| *a < 9)
        .unwrap_or(4);
    (scale, anchor)
}

/// Where a picture `w` x `h` mm goes in `area` for `scale`; `anchor` is
/// the point of the area it is placed by, 0 top left to 8 bottom right
/// row by row (page space: y grows upward).
pub fn place(area: Rect, w: f64, h: f64, scale: Scale, anchor: u8) -> Rect {
    let (w, h) = (w.max(1e-9), h.max(1e-9));
    let (w, h) = match scale {
        Scale::Actual => (w, h),
        Scale::Fit => {
            let k = (area.width() / w).min(area.height() / h);
            (w * k, h * k)
        }
        Scale::Fill => {
            let k = (area.width() / w).max(area.height() / h);
            (w * k, h * k)
        }
        Scale::Stretch => return area,
    };
    let (col, row) = ((anchor % 3) as f64, (anchor / 3).min(2) as f64);
    let x0 = area.x0 + (area.width() - w) * col / 2.0;
    let y1 = area.y1 - (area.height() - h) * row / 2.0;
    Rect::new(x0, y1 - h, x0 + w, y1)
}

/// A picture field's picture: the PNG, its size in pixels and in mm.
#[derive(Clone)]
pub struct Picture {
    pub png: tracedraw_core::SharedBytes,
    pub px: (u32, u32),
    pub mm: (f64, f64),
}

/// The picture a cell names (relative paths from the data file's
/// folder), or `None` when it does not load.
pub fn picture(value: &str, base: Option<&std::path::Path>) -> Option<Picture> {
    let dirs: Vec<std::path::PathBuf> = base
        .and_then(|b| b.parent())
        .map(|d| d.to_path_buf())
        .into_iter()
        .collect();
    picture_in(value, &dirs)
}

/// The folders a picture cell's relative path is looked for in: the
/// pictures folder chosen in the Print Merge docker, then the data
/// file's folder (#460).
pub fn picture_dirs(st: &PrintMergeState) -> Vec<std::path::PathBuf> {
    let mut dirs: Vec<std::path::PathBuf> = st.pictures.iter().cloned().collect();
    if let Some(d) = st.path.as_deref().and_then(|p| p.parent()) {
        dirs.push(d.to_path_buf());
    }
    dirs
}

/// The extensions tried for a cell that names a picture without one (an
/// ID card's "1024" finds "1024.jpg").
const PICTURE_EXTENSIONS: [&str; 8] = ["jpg", "jpeg", "png", "webp", "bmp", "gif", "tif", "tiff"];

/// The bytes of the picture a cell names: an absolute path as it is, a
/// relative one in each of `dirs` in turn; a name without an extension
/// with each picture extension.
fn picture_bytes(value: &str, dirs: &[std::path::PathBuf]) -> Option<Vec<u8>> {
    let name = std::path::PathBuf::from(value.trim());
    let mut tries: Vec<std::path::PathBuf> = Vec::new();
    if name.is_absolute() {
        tries.push(name.clone());
    } else {
        tries.extend(dirs.iter().map(|d| d.join(&name)));
        tries.push(name.clone());
    }
    for t in tries {
        if let Ok(b) = crate::files::read(&t) {
            return Some(b);
        }
        if t.extension().is_none() {
            for ext in PICTURE_EXTENSIONS {
                for e in [ext.to_string(), ext.to_uppercase()] {
                    if let Ok(b) = crate::files::read(t.with_extension(&e)) {
                        return Some(b);
                    }
                }
            }
        }
    }
    None
}

/// The pictures directly inside `dir` (by their extension), in name
/// order ignoring case, numbers by their value ("2" before "10").
pub fn folder_pictures(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out: Vec<std::path::PathBuf> = crate::files::list_dir(dir)
        .into_iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| PICTURE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    out.sort_by_cached_key(|p| natural_key(&p.file_name().unwrap_or_default().to_string_lossy()));
    out
}

/// A sort key reading runs of digits as numbers and letters without case.
fn natural_key(name: &str) -> Vec<(u8, u128, String)> {
    let mut key = Vec::new();
    let mut chars = name.chars().peekable();
    while let Some(&c) = chars.peek() {
        let digit = c.is_ascii_digit();
        let mut run = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() != digit {
                break;
            }
            run.push(c);
            chars.next();
        }
        if digit {
            key.push((0, run.parse().unwrap_or(u128::MAX), String::new()));
        } else {
            key.push((1, 0, run.to_lowercase()));
        }
    }
    key
}

/// [`picture`] looked for in `dirs`.
pub fn picture_in(value: &str, dirs: &[std::path::PathBuf]) -> Option<Picture> {
    if value.trim().is_empty() {
        return None;
    }
    let bytes = picture_bytes(value, dirs)?;
    let (dx, dy) = crate::image_dpi::resolution(&bytes)
        .unwrap_or((crate::image_dpi::DEFAULT_DPI, crate::image_dpi::DEFAULT_DPI));
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let png = crate::bitmap_fx::encode(&img)?;
    let (w, h) = img.dimensions();
    Some(Picture {
        png: png.into(),
        px: (w, h),
        mm: (w as f64 * 25.4 / dx, h as f64 * 25.4 / dy),
    })
}

/// The records of a data file: a CSV (comma, semicolon or tab, in any
/// encoding [`crate::export::decode_text`] reads) or an Excel workbook's
/// first sheet (`.xlsx`); the first row names the fields.
pub fn read_data(path: &std::path::Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let excel = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("xlsx"));
    if excel {
        let bytes = crate::files::read(path).map_err(|e| e.to_string())?;
        let mut rows = tracedraw_io::xlsx::read_rows(&bytes)?;
        if rows.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let headers = rows.remove(0);
        return Ok((headers, rows));
    }
    let bytes = crate::files::read(path).map_err(|e| e.to_string())?;
    Ok(crate::export::parse_csv(&crate::export::decode_text(
        &bytes,
    )))
}

/// Field types guessed from a data file: a field whose name says picture
/// or photo, or whose cells name picture files, is a picture; one whose
/// name says QR is a QR code; the rest are text. They can be changed in
/// Configure Columns.
pub fn guess_kinds(headers: &[String], rows: &[Vec<String>]) -> Vec<FieldKind> {
    const PICTURE_WORDS: [&str; 7] = ["photo", "picture", "image", "foto", "imagem", "img", "logo"];
    const PICTURE_ENDS: [&str; 7] = [".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".tif"];
    headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let name = h.to_lowercase();
            let cells: Vec<String> = rows
                .iter()
                .filter_map(|r| r.get(i))
                .map(|c| c.trim().to_lowercase())
                .filter(|c| !c.is_empty())
                .collect();
            let files = !cells.is_empty()
                && cells
                    .iter()
                    .all(|c| PICTURE_ENDS.iter().any(|e| c.ends_with(e)));
            if name.contains("qr") {
                FieldKind::Qr
            } else if files || PICTURE_WORDS.iter().any(|w| name.contains(w)) {
                FieldKind::Image
            } else {
                FieldKind::Text
            }
        })
        .collect()
}

/// The kind of field `i`, text when the state has none for it.
pub fn kind_of(st: &PrintMergeState, i: usize) -> FieldKind {
    st.kinds.get(i).cloned().unwrap_or_default()
}

/// The value of every cell as the merge uses it: counters worked out,
/// short records padded with empty cells.
pub fn values(st: &PrintMergeState) -> Vec<Vec<String>> {
    st.rows
        .iter()
        .enumerate()
        .map(|(r, row)| {
            (0..st.headers.len())
                .map(|c| match kind_of(st, c) {
                    FieldKind::Number {
                        start,
                        step,
                        digits,
                        format,
                    } => {
                        let n = start.saturating_add(step.saturating_mul(r as i64));
                        format.write(n, digits)
                    }
                    _ => row.get(c).cloned().unwrap_or_default(),
                })
                .collect()
        })
        .collect()
}

/// Add a field named `name` (text) to every record.
pub fn add_field(st: &mut PrintMergeState, name: &str) {
    st.headers.push(name.to_string());
    st.kinds.resize(st.headers.len() - 1, FieldKind::Text);
    st.kinds.push(FieldKind::Text);
    for row in st.rows.iter_mut() {
        row.resize(st.headers.len(), String::new());
    }
}

/// Add a number field counting from `start` by `step`, padded to
/// `digits` digits and written in `format` (#930); with `end`, records
/// are added until the counter reaches it (from start to end, both
/// included), as many as that takes, at most [`MAX_RANGE_RECORDS`] in
/// all; records already there are kept (#791).
pub fn add_number_field(
    st: &mut PrintMergeState,
    name: &str,
    start: i64,
    step: i64,
    digits: u32,
    format: NumberFormat,
    end: Option<i64>,
) {
    add_field(st, name);
    if let Some(k) = st.kinds.last_mut() {
        *k = FieldKind::Number {
            start,
            step,
            digits: digits.clamp(1, 12),
            format,
        };
    }
    let Some(end) = end else {
        return;
    };
    if step == 0 || (end - start).signum() != step.signum() && end != start {
        return;
    }
    let count = ((end - start) / step).unsigned_abs() as usize + 1;
    let want = count.min(MAX_RANGE_RECORDS);
    while st.rows.len() < want {
        add_record(st);
    }
}

/// The most records an end value adds.
pub const MAX_RANGE_RECORDS: usize = 100_000;

/// Remove field `i` and its cells.
pub fn remove_field(st: &mut PrintMergeState, i: usize) {
    if i >= st.headers.len() {
        return;
    }
    st.headers.remove(i);
    if i < st.kinds.len() {
        st.kinds.remove(i);
    }
    for row in st.rows.iter_mut() {
        if i < row.len() {
            row.remove(i);
        }
    }
}

/// Add an empty record at the end.
pub fn add_record(st: &mut PrintMergeState) {
    st.rows.push(vec![String::new(); st.headers.len()]);
}

/// What a drag in the data grid selects: a block of cells, whole records
/// (from the record numbers) or whole fields (from the field names).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridDrag {
    Cells,
    Records,
    Fields,
}

/// A block of cells of the data grid: the cell a drag started on and the
/// one it reached, as (record, field), both 0-based (#501).
pub type CellRange = ((usize, usize), (usize, usize));

/// The records and fields a block spans, in order.
pub fn range_bounds(
    sel: CellRange,
) -> (
    std::ops::RangeInclusive<usize>,
    std::ops::RangeInclusive<usize>,
) {
    let ((r0, c0), (r1, c1)) = sel;
    (r0.min(r1)..=r0.max(r1), c0.min(c1)..=c0.max(c1))
}

/// Insert `n` empty records before record `at` (at the end when past it).
pub fn insert_records(st: &mut PrintMergeState, at: usize, n: usize) {
    let at = at.min(st.rows.len());
    let width = st.headers.len();
    for _ in 0..n {
        st.rows.insert(at, vec![String::new(); width]);
    }
}

/// Delete the records `rows` spans (those that exist).
pub fn delete_records(st: &mut PrintMergeState, rows: std::ops::RangeInclusive<usize>) {
    let n = st.rows.len();
    let (a, b) = (*rows.start(), (*rows.end()).min(n.saturating_sub(1)));
    if a < n && a <= b {
        st.rows.drain(a..=b);
    }
    st.current = st.current.min(st.rows.len().saturating_sub(1));
}

/// Empty the cells of a block; counter fields, worked out rather than
/// typed, keep theirs.
pub fn clear_cells(st: &mut PrintMergeState, sel: CellRange) {
    let (rows, cols) = range_bounds(sel);
    for r in rows {
        for c in cols.clone() {
            if matches!(kind_of(st, c), FieldKind::Number { .. }) {
                continue;
            }
            if let Some(cell) = st.rows.get_mut(r).and_then(|row| row.get_mut(c)) {
                cell.clear();
            }
        }
    }
}

/// The records as CSV text, counters written out, with a header line.
pub fn to_csv(st: &PrintMergeState) -> String {
    let mut w = csv::WriterBuilder::new().from_writer(Vec::new());
    let _ = w.write_record(&st.headers);
    for row in values(st) {
        let _ = w.write_record(&row);
    }
    w.into_inner()
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_default()
}

/// The text of a text object, all its runs together.
fn text_of(s: &Shape) -> Option<String> {
    match &s.kind {
        ShapeKind::Text { spans, .. } => Some(spans.iter().map(|sp| sp.text.as_str()).collect()),
        _ => None,
    }
}

/// The box a field object stands for, in its own coordinates: the frame of
/// paragraph text, or what artistic text covers.
fn local_box(s: &Shape) -> Rect {
    let page = s.bounds();
    if s.transform.determinant().abs() < 1e-12 {
        return page;
    }
    (s.transform.inverse() * page.to_path(0.01)).bounding_box()
}

/// The largest rectangle of aspect `w : h` centred in `r`.
fn fit(r: Rect, w: f64, h: f64) -> Rect {
    let k = (r.width() / w.max(1e-9)).min(r.height() / h.max(1e-9));
    centred(r, w * k, h * k)
}

/// The smallest rectangle of aspect `w : h` centred on `r` that covers it
/// (a picture filling a frame, cut by it).
fn cover(r: Rect, w: f64, h: f64) -> Rect {
    let k = (r.width() / w.max(1e-9)).max(r.height() / h.max(1e-9));
    centred(r, w * k, h * k)
}

fn centred(r: Rect, w: f64, h: f64) -> Rect {
    let c = r.center();
    Rect::new(c.x - w / 2.0, c.y - h / 2.0, c.x + w / 2.0, c.y + h / 2.0)
}

/// What takes the place of a picture or QR field object, or `None` when
/// the value gives nothing (an empty cell, a file that does not load). A
/// picture fits in `area`, or with `fill` covers it (inside a frame that
/// cuts it).
fn replacement(
    kind: &FieldKind,
    value: &str,
    area: Rect,
    dirs: &[std::path::PathBuf],
    fill: bool,
) -> Option<(ShapeKind, Fill)> {
    if value.trim().is_empty() {
        return None;
    }
    match kind {
        FieldKind::Image => {
            let pic = picture_in(value, dirs)?;
            let (w, h) = (pic.px.0 as f64, pic.px.1 as f64);
            Some((
                ShapeKind::Bitmap {
                    rect: if fill {
                        cover(area, w, h)
                    } else {
                        fit(area, w, h)
                    },
                    width_px: pic.px.0,
                    height_px: pic.px.1,
                    png: pic.png,
                    fx: None,
                },
                Fill::None,
            ))
        }
        FieldKind::Qr => {
            let side = area.width().min(area.height());
            let path = crate::ui::dialogs::qr_path(value, side).ok()?;
            let r = fit(area, 1.0, 1.0);
            let path = tracedraw_core::geometry::Affine::translate((r.x0, r.y0)) * path;
            Some((
                ShapeKind::Path { path, closed: true },
                Fill::Solid(Color::Cmyk {
                    c: 0.0,
                    m: 0.0,
                    y: 0.0,
                    k: 1.0,
                }),
            ))
        }
        _ => None,
    }
}

/// The picture or QR field whose placeholder `<Field>` is `text`.
fn whole_field(st: &PrintMergeState, text: &str) -> Option<usize> {
    st.headers
        .iter()
        .enumerate()
        .find(|(i, h)| {
            matches!(kind_of(st, *i), FieldKind::Image | FieldKind::Qr)
                && text.trim() == format!("<{h}>")
        })
        .map(|(i, _)| i)
}

/// The commands that fill one page's field objects with one record:
/// objects anywhere in groups and in clip frames too. `new_id` gives the
/// ids of pictures put inside frames. With `keep_when_missing` a picture
/// frame whose picture does not load keeps what it holds (a merge onto
/// the drawing's own pages, #618); without it the frame is emptied, so a
/// copy never shows the template's stand-in for a record without one.
fn fill_page(
    st: &PrintMergeState,
    page: &tracedraw_core::document::Page,
    row: &[String],
    keep_when_missing: bool,
    new_id: &mut dyn FnMut() -> tracedraw_core::ShapeId,
) -> Vec<Command> {
    let mut cmds = Vec::new();
    for l in &page.layers {
        for s in &l.shapes {
            fill_shape(
                st,
                l.id,
                s,
                row,
                None,
                true,
                keep_when_missing,
                new_id,
                &mut cmds,
            );
        }
    }
    cmds
}

/// [`fill_page`] for one object. `frame` is the box of the clip frame the
/// object is in (in the object's parent space): a picture there fills it.
#[allow(clippy::too_many_arguments)]
fn fill_shape(
    st: &PrintMergeState,
    layer: LayerId,
    s: &Shape,
    row: &[String],
    frame: Option<Rect>,
    top: bool,
    keep: bool,
    new_id: &mut dyn FnMut() -> tracedraw_core::ShapeId,
    cmds: &mut Vec<Command>,
) {
    let value_of = |i: usize| row.get(i).map(String::as_str).unwrap_or("");
    // An object named `<Field>` of a picture or QR field: a placeholder
    // the Print Merge docker made, or a shape of the user's made the
    // field's frame (#378, #437), on the page or inside a group (#460).
    if text_of(s).is_none() && (top || !matches!(s.kind, ShapeKind::Group { .. })) {
        if let Some(i) = s.name.as_deref().and_then(|n| whole_field(st, n)) {
            if top {
                named_placeholder(st, layer, s, i, value_of(i), keep, new_id, cmds);
            } else if kind_of(st, i) == FieldKind::Image {
                nested_picture_frame(st, s, value_of(i), keep, new_id, cmds);
            } else {
                named_placeholder(st, layer, s, i, value_of(i), keep, new_id, cmds);
            }
            return;
        }
    }
    match &s.kind {
        ShapeKind::Group { children } => {
            for c in children {
                fill_shape(st, layer, c, row, None, false, keep, new_id, cmds);
            }
            return;
        }
        ShapeKind::ClipFrame { frame: f, contents } => {
            let area = f.bounds();
            for c in contents {
                fill_shape(st, layer, c, row, Some(area), false, keep, new_id, cmds);
            }
            return;
        }
        _ => {}
    }
    let Some(text) = text_of(s) else {
        return;
    };
    // A picture or QR field: the object holds only its placeholder.
    if let Some(i) = whole_field(st, &text) {
        // In a clip frame a picture fills the frame, in the object's own
        // space; elsewhere it fits the object's box.
        let (area, fill) = match frame.filter(|_| kind_of(st, i) == FieldKind::Image) {
            Some(r) if s.transform.determinant().abs() > 1e-12 => (
                (s.transform.inverse() * r.to_path(0.01)).bounding_box(),
                true,
            ),
            _ => (local_box(s), false),
        };
        match replacement(&kind_of(st, i), value_of(i), area, &picture_dirs(st), fill) {
            Some((kind, fill)) => {
                cmds.push(Command::SetShapeKind { shape: s.id, kind });
                cmds.push(Command::SetFill {
                    shapes: vec![s.id],
                    fill,
                });
                cmds.push(Command::SetStroke {
                    shapes: vec![s.id],
                    stroke: None,
                });
            }
            None => cmds.push(Command::DeleteShapes { shapes: vec![s.id] }),
        }
        return;
    }
    let ShapeKind::Text {
        spans,
        origin,
        frame: text_frame,
        align,
        para,
        on_path,
    } = &s.kind
    else {
        return;
    };
    let mut changed = false;
    let new_spans: Vec<TextSpan> = spans
        .iter()
        .map(|sp| {
            let mut t = sp.text.clone();
            for (i, (h, v)) in st.headers.iter().zip(row).enumerate() {
                if matches!(kind_of(st, i), FieldKind::Image | FieldKind::Qr) {
                    continue;
                }
                let ph = format!("<{h}>");
                if t.contains(&ph) {
                    t = t.replace(&ph, v);
                    changed = true;
                }
            }
            TextSpan {
                text: t,
                ..sp.clone()
            }
        })
        .collect();
    if changed {
        cmds.push(Command::SetShapeKind {
            shape: s.id,
            kind: ShapeKind::Text {
                spans: new_spans,
                origin: *origin,
                frame: *text_frame,
                align: *align,
                para: para.clone(),
                on_path: on_path.clone(),
            },
        });
    }
}

/// A placeholder object named `<Field>` filled with the record's value:
/// for a picture field the picture goes inside it (a clip frame's old
/// contents, such as the placeholder picture, give way), placed by its
/// Align and scale; for a QR field the object becomes the code, as large
/// as its box allows. For a picture that does not load the frame is
/// emptied, or with `keep` keeps what it holds.
#[allow(clippy::too_many_arguments)]
fn named_placeholder(
    st: &PrintMergeState,
    layer: LayerId,
    s: &Shape,
    i: usize,
    value: &str,
    keep: bool,
    new_id: &mut dyn FnMut() -> tracedraw_core::ShapeId,
    cmds: &mut Vec<Command>,
) {
    match kind_of(st, i) {
        FieldKind::Image => {
            // On the drawing's own pages an empty cell or a picture that
            // does not load keeps the frame's current picture (#618).
            let pic = picture_in(value, &picture_dirs(st));
            if pic.is_none() && keep {
                return;
            }
            if let ShapeKind::ClipFrame { contents, .. } = &s.kind {
                if !contents.is_empty() {
                    cmds.push(Command::ExtractContents { clip: s.id });
                    cmds.push(Command::DeleteShapes {
                        shapes: contents.iter().map(|c| c.id).collect(),
                    });
                }
            }
            let Some(pic) = pic else {
                return;
            };
            let (scale, anchor) = placement_of(s);
            let rect = place(s.bounds(), pic.mm.0, pic.mm.1, scale, anchor);
            let id = new_id();
            let mut shape = Shape::new(
                id,
                ShapeKind::Bitmap {
                    rect,
                    width_px: pic.px.0,
                    height_px: pic.px.1,
                    png: pic.png,
                    fx: None,
                },
            );
            shape.stroke = None;
            shape.fill = Fill::None;
            cmds.push(Command::AddShape { layer, shape });
            cmds.push(Command::PlaceInside {
                contents: vec![id],
                frame: s.id,
            });
        }
        FieldKind::Qr => match replacement(&FieldKind::Qr, value, local_box(s), &[], false) {
            Some((kind, fill)) => {
                cmds.push(Command::SetShapeKind { shape: s.id, kind });
                cmds.push(Command::SetFill {
                    shapes: vec![s.id],
                    fill,
                });
                cmds.push(Command::SetStroke {
                    shapes: vec![s.id],
                    stroke: None,
                });
            }
            None => cmds.push(Command::DeleteShapes { shapes: vec![s.id] }),
        },
        _ => {}
    }
}

/// [`named_placeholder`] for a picture field's frame inside a group, where
/// objects cannot be moved into it: the object becomes, in place, a clip
/// frame of its own outline holding the picture (a clip frame's contents
/// give way). Its transform stays on the clip frame, so the frame and the
/// picture are laid out in the object's own space.
fn nested_picture_frame(
    st: &PrintMergeState,
    s: &Shape,
    value: &str,
    keep: bool,
    new_id: &mut dyn FnMut() -> tracedraw_core::ShapeId,
    cmds: &mut Vec<Command>,
) {
    let frame = match &s.kind {
        ShapeKind::ClipFrame { frame, .. } => (**frame).clone(),
        _ => {
            let mut f = s.clone();
            f.id = new_id();
            f.name = None;
            f.data.clear();
            f.transform = tracedraw_core::geometry::Affine::IDENTITY;
            f
        }
    };
    let mut contents = Vec::new();
    if let Some(pic) = picture_in(value, &picture_dirs(st)) {
        let (scale, anchor) = placement_of(s);
        let rect = place(frame.bounds(), pic.mm.0, pic.mm.1, scale, anchor);
        let mut shape = Shape::new(
            new_id(),
            ShapeKind::Bitmap {
                rect,
                width_px: pic.px.0,
                height_px: pic.px.1,
                png: pic.png,
                fx: None,
            },
        );
        shape.stroke = None;
        shape.fill = Fill::None;
        contents.push(shape);
    } else if keep || !matches!(s.kind, ShapeKind::ClipFrame { .. }) {
        // No picture: a frame on the drawing's own pages keeps its
        // contents (#618); a plain object stays as it is.
        return;
    }
    cmds.push(Command::SetShapeKind {
        shape: s.id,
        kind: ShapeKind::ClipFrame {
            frame: Box::new(frame),
            contents,
        },
    });
}

/// Duplicate `page` once per record right after it and fill the copies'
/// fields, as one undo step per stage; the new pages, in the records'
/// order.
fn merge_with(
    engine: &mut tracedraw_core::Engine,
    page: tracedraw_core::PageId,
    st: &PrintMergeState,
) -> Result<Vec<tracedraw_core::PageId>, String> {
    let rows = values(st);
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let template = engine
        .document()
        .page(page)
        .map_err(|e| e.to_string())?
        .clone();
    let cmds: Vec<Command> = rows.iter().map(|_| Command::duplicate_page(page)).collect();
    engine
        .run_batch("Print Merge", &cmds)
        .map_err(|e| e.to_string())?;
    // The copies follow the template; record k goes on the k-th of them,
    // so the pages read in the records' order.
    let idx = engine
        .document()
        .pages
        .iter()
        .position(|p| p.id == page)
        .unwrap_or(0);
    let new_pages: Vec<tracedraw_core::PageId> = engine
        .document()
        .pages
        .iter()
        .skip(idx + 1)
        .take(rows.len())
        .map(|p| p.id)
        .collect();
    let mut cmds = Vec::new();
    for (k, pid) in new_pages.iter().enumerate() {
        let (Some(row), Ok(p)) = (rows.get(k), engine.document().page(*pid)) else {
            continue;
        };
        let p = p.clone();
        let mut new_id = || engine.new_shape_id();
        cmds.extend(fill_page(st, &p, row, false, &mut new_id));
        cmds.push(Command::RenamePage {
            page: *pid,
            name: format!("{} {}", template.name, k + 1),
        });
    }
    engine
        .run_batch("Print Merge", &cmds)
        .map_err(|e| e.to_string())?;
    Ok(new_pages)
}

/// Duplicate the current page once per record and fill the copies'
/// fields, as one undo step per stage.
pub fn perform(app: &mut App, st: &PrintMergeState) {
    let page = app.page;
    match merge_with(&mut app.engine, page, st) {
        Ok(pages) if !pages.is_empty() => {
            app.status = trf("status.merge_done", &[("n", &pages.len().to_string())]);
        }
        Ok(_) => {}
        Err(e) => app.status = e,
    }
}

/// What [`merge_onto_pages`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InPlaceMerge {
    /// Pages that had fields to fill; 0 when everything was already filled.
    pub filled: usize,
    /// Records past the drawing's last page, left out.
    pub left_out: usize,
    /// The pages merged hold merge fields at all.
    pub fields: bool,
}

/// Whether `s`, or an object in it, is a merge field: a text holding a
/// field's `<Name>`, or an object named after a picture or QR field.
fn holds_field(st: &PrintMergeState, s: &Shape) -> bool {
    if s.name
        .as_deref()
        .is_some_and(|n| whole_field(st, n).is_some())
    {
        return true;
    }
    if let Some(t) = text_of(s) {
        if st.headers.iter().any(|h| t.contains(&format!("<{h}>"))) {
            return true;
        }
    }
    match &s.kind {
        ShapeKind::Group { children } => children.iter().any(|c| holds_field(st, c)),
        ShapeKind::ClipFrame { contents, .. } => contents.iter().any(|c| holds_field(st, c)),
        _ => false,
    }
}

/// Fill the drawing's own pages instead of copies (the docker's merge
/// with On every page, #618): record k goes on the k-th page from the
/// first, as one undo step; pages past the last record, and records past
/// the last page, are left out. A picture field whose picture does not
/// load keeps what it shows, so a drawing already filled is left as it is.
pub fn merge_onto_pages(
    engine: &mut tracedraw_core::Engine,
    st: &PrintMergeState,
) -> Result<InPlaceMerge, String> {
    let rows = values(st);
    // Copies of a field put on every page go where the field is now on
    // its own page, unless they were moved by hand (#956).
    let follow = if rows.is_empty() {
        Vec::new()
    } else {
        follow_originals(engine.document())
    };
    if !follow.is_empty() {
        engine
            .run_batch("Print Merge", &follow)
            .map_err(|e| e.to_string())?;
    }
    let pages: Vec<tracedraw_core::document::Page> = engine
        .document()
        .pages
        .iter()
        .take(rows.len())
        .cloned()
        .collect();
    if pages.is_empty() {
        return Err(crate::i18n::tr("merge.no_records"));
    }
    let mut cmds = Vec::new();
    let mut filled = 0;
    for (p, row) in pages.iter().zip(&rows) {
        let mut new_id = || engine.new_shape_id();
        let page_cmds = fill_page(st, p, row, true, &mut new_id);
        filled += usize::from(!page_cmds.is_empty());
        cmds.extend(page_cmds);
    }
    if !cmds.is_empty() {
        // One undo step with the copies' moves.
        let done = if follow.is_empty() {
            engine.run_batch("Print Merge", &cmds)
        } else {
            engine.amend_batch("Print Merge", &cmds)
        };
        done.map_err(|e| e.to_string())?;
    }
    let fields = pages
        .iter()
        .flat_map(|p| p.layers.iter().flat_map(|l| l.shapes.iter()))
        .any(|s| holds_field(st, s));
    Ok(InPlaceMerge {
        filled,
        left_out: rows.len() - pages.len(),
        fields,
    })
}

/// The merged pages as a drawing of their own (the docker's Create
/// Merged Document): the current page filled once per record, the
/// drawing it came from left as it is.
pub fn merged_document(
    app: &App,
    st: &PrintMergeState,
) -> Result<tracedraw_core::Document, String> {
    let mut engine = tracedraw_core::Engine::new(app.doc().clone());
    let pages = merge_with(&mut engine, app.page, st)?;
    if pages.is_empty() {
        return Err(crate::i18n::tr("merge.no_records"));
    }
    let others: Vec<tracedraw_core::PageId> = engine
        .document()
        .pages
        .iter()
        .map(|p| p.id)
        .filter(|id| !pages.contains(id))
        .collect();
    for page in others {
        engine
            .run(&Command::DeletePage { page })
            .map_err(|e| e.to_string())?;
    }
    Ok(engine.document().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> PrintMergeState {
        PrintMergeState {
            headers: vec!["Name".into(), "No".into(), "Link".into()],
            rows: vec![
                vec!["Ana".into(), String::new(), "https://a.example".into()],
                vec!["Bob".into()],
            ],
            kinds: vec![
                FieldKind::Text,
                FieldKind::Number {
                    start: 7,
                    step: 3,
                    digits: 3,
                    format: NumberFormat::Digits,
                },
                FieldKind::Qr,
            ],
            ..Default::default()
        }
    }

    /// Add Column with a number from 15 to 25 in three digits, as in the
    /// report: the records are made for the whole range, 015 to 025, and
    /// records already there are kept; no end value adds none, and an end
    /// the counter never reaches adds none either (#791).
    #[test]
    fn a_number_column_with_an_end_value_makes_its_records() {
        let mut st = PrintMergeState::default();
        add_number_field(
            &mut st,
            "Check Number",
            15,
            1,
            3,
            NumberFormat::Digits,
            Some(25),
        );
        assert_eq!(st.rows.len(), 11);
        let v = values(&st);
        assert_eq!(v.first().map(|r| r[0].as_str()), Some("015"));
        assert_eq!(v.last().map(|r| r[0].as_str()), Some("025"));
        // Counting down by two.
        let mut st = PrintMergeState::default();
        add_number_field(&mut st, "Down", 10, -2, 2, NumberFormat::Digits, Some(0));
        let v: Vec<String> = values(&st).into_iter().map(|r| r[0].clone()).collect();
        assert_eq!(v, ["10", "08", "06", "04", "02", "00"]);
        // A second column keeps the records there and adds what it needs.
        add_number_field(&mut st, "More", 1, 1, 1, NumberFormat::Digits, Some(8));
        assert_eq!(st.rows.len(), 8);
        add_number_field(&mut st, "Fewer", 1, 1, 1, NumberFormat::Digits, Some(3));
        assert_eq!(st.rows.len(), 8, "records are never taken away");
        // No end, a fixed value, or an end the counter never reaches.
        let mut st = PrintMergeState::default();
        add_number_field(&mut st, "N", 1, 1, 1, NumberFormat::Digits, None);
        add_number_field(&mut st, "Same", 5, 0, 1, NumberFormat::Digits, Some(9));
        add_number_field(&mut st, "Away", 5, 1, 1, NumberFormat::Digits, Some(1));
        assert!(st.rows.is_empty());
        // A huge range stops at the limit.
        let mut st = PrintMergeState::default();
        add_number_field(
            &mut st,
            "Huge",
            0,
            1,
            1,
            NumberFormat::Digits,
            Some(i64::MAX),
        );
        assert_eq!(st.rows.len(), MAX_RANGE_RECORDS);
    }

    #[test]
    fn counters_and_short_records_are_filled_in() {
        let v = values(&state());
        assert_eq!(v[0], vec!["Ana", "007", "https://a.example"]);
        assert_eq!(v[1], vec!["Bob", "010", ""]);
        let mut st = state();
        st.kinds[1] = FieldKind::Number {
            start: -2,
            step: 1,
            digits: 2,
            format: NumberFormat::Digits,
        };
        assert_eq!(values(&st)[0][1], "-02");
    }

    /// A number field's Format: letters (A to Z, then AA), small letters,
    /// and a custom text before and after the padded number, for tickets
    /// and serial numbers (#930).
    #[test]
    fn number_fields_write_letters_and_custom_text() {
        let column = |format: NumberFormat, start: i64, end: i64| {
            let mut st = PrintMergeState::default();
            add_number_field(&mut st, "No", start, 1, 4, format, Some(end));
            values(&st)
                .into_iter()
                .map(|r| r[0].clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(column(NumberFormat::Upper, 1, 3), ["A", "B", "C"]);
        assert_eq!(column(NumberFormat::Upper, 25, 28), ["Y", "Z", "AA", "AB"]);
        assert_eq!(column(NumberFormat::Lower, 52, 53), ["az", "ba"]);
        assert_eq!(NumberFormat::Upper.write(702, 1), "ZZ");
        assert_eq!(NumberFormat::Upper.write(703, 1), "AAA");
        assert_eq!(NumberFormat::Upper.write(i64::MAX, 1), "CRPXNLSKVLJFHG");
        // No letters for 0 and below: written as numbers.
        assert_eq!(NumberFormat::Lower.write(0, 2), "00");
        assert_eq!(NumberFormat::Upper.write(-3, 1), "-3");
        let ticket = NumberFormat::Custom {
            prefix: "Ticket T-".into(),
            suffix: "/2026".into(),
        };
        assert_eq!(
            column(ticket, 98, 100),
            [
                "Ticket T-0098/2026",
                "Ticket T-0099/2026",
                "Ticket T-0100/2026"
            ]
        );
        // The Format list round-trips its indices.
        for i in 0..NumberFormat::KEYS.len() {
            assert_eq!(NumberFormat::from_index(i).index(), i);
        }
        assert_eq!(NumberFormat::from_index(99), NumberFormat::Digits);
    }

    #[test]
    fn fields_and_records_are_added_and_removed() {
        let mut st = state();
        add_field(&mut st, "City");
        assert_eq!(st.headers.len(), 4);
        assert!(st.rows.iter().all(|r| r.len() == 4));
        add_record(&mut st);
        assert_eq!(st.rows.len(), 3);
        remove_field(&mut st, 1);
        assert_eq!(st.headers, vec!["Name", "Link", "City"]);
        assert_eq!(st.kinds[1], FieldKind::Qr);
        assert!(st.rows.iter().all(|r| r.len() == 3));
    }

    /// Clearing a block of the grid empties its typed cells; a counter
    /// field, worked out, keeps its numbers (#501).
    #[test]
    fn clearing_cells_leaves_counters() {
        let mut st = state();
        for row in st.rows.iter_mut() {
            row.resize(3, String::new());
        }
        st.rows[0][1] = "kept".into();
        clear_cells(&mut st, ((1, 2), (0, 0)));
        assert_eq!(st.rows[0], vec!["", "kept", ""]);
        assert_eq!(values(&st)[1][1], "010");
        assert_eq!(range_bounds(((3, 1), (1, 4))), (1..=3, 1..=4));
    }

    #[test]
    fn csv_round_trips_with_counters_written_out() {
        let text = to_csv(&state());
        let (h, r) = crate::export::parse_csv(&text);
        assert_eq!(h, vec!["Name", "No", "Link"]);
        assert_eq!(r[1], vec!["Bob", "010", ""]);
    }

    #[test]
    fn merge_fills_text_qr_and_counters() {
        let mut app = App::headless();
        let mut add = |text: &str, frame: bool| {
            app.new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new(text, "Arial", 12.0)],
                origin: tracedraw_core::geometry::Point::new(10.0, 10.0),
                frame: frame.then(|| tracedraw_core::geometry::Size::new(30.0, 30.0)),
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            })
            .expect("added");
        };
        add("Hello <Name> no <No>", false);
        add("<Link>", true);
        let pages_before = app.doc().pages.len();
        perform(&mut app, &state());
        let doc = app.doc();
        assert_eq!(doc.pages.len(), pages_before + 2);
        let first = &doc.pages[1];
        let shapes: Vec<&Shape> = first.layers.iter().flat_map(|l| &l.shapes).collect();
        let texts: Vec<String> = shapes.iter().filter_map(|s| text_of(s)).collect();
        assert!(texts.contains(&"Hello Ana no 007".to_string()), "{texts:?}");
        // The QR field became a path inside the 30 mm frame.
        let qr = shapes
            .iter()
            .find(|s| matches!(s.kind, ShapeKind::Path { .. }))
            .expect("qr code");
        let b = qr.bounds();
        assert!(b.width() > 25.0 && b.width() <= 30.0 + 1e-6, "{b:?}");
        // The second record has no link: its field object is gone.
        let second = &doc.pages[2];
        let n: usize = second.layers.iter().map(|l| l.shapes.len()).sum();
        assert_eq!(n, 1);
    }

    /// Bulk cards (#377, #378): fields inside a group are filled, a
    /// picture placeholder inside a clip frame fills the frame, and a
    /// rectangle named `<Logo>` takes the picture inside it.
    #[test]
    fn fields_in_groups_and_frames_are_filled() {
        let dir = std::env::temp_dir().join(format!("td_merge_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // A tall picture (20 x 40 px) and a wide one.
        image::RgbaImage::from_pixel(20, 40, image::Rgba([200, 0, 0, 255]))
            .save(dir.join("ana.png"))
            .unwrap();
        image::RgbaImage::from_pixel(60, 20, image::Rgba([0, 0, 200, 255]))
            .save(dir.join("logo.png"))
            .unwrap();
        let mut app = App::headless();
        let text = |app: &mut App, t: &str, x: f64| {
            app.new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new(t, "Arial", 12.0)],
                origin: tracedraw_core::geometry::Point::new(x, 10.0),
                frame: Some(tracedraw_core::geometry::Size::new(20.0, 20.0)),
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            })
            .expect("text")
        };
        let rect = |app: &mut App, x: f64| {
            app.new_shape(ShapeKind::Rect {
                rect: Rect::new(x, 100.0, x + 30.0, 130.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect")
        };
        // The name in a group with a rectangle.
        let name = text(&mut app, "Card of <Name>", 10.0);
        let back = rect(&mut app, 0.0);
        app.select(vec![name, back]);
        app.group_selection();
        // The photo placeholder inside a 30 x 30 frame.
        let frame = rect(&mut app, 50.0);
        let ph = text(&mut app, "<Photo>", 55.0);
        app.place_inside(vec![ph], frame);
        // A rectangle named <Logo>.
        let logo = rect(&mut app, 100.0);
        app.run(Command::SetShapeName {
            shape: logo,
            name: Some("<Logo>".into()),
        });
        let st = PrintMergeState {
            headers: vec!["Name".into(), "Photo".into(), "Logo".into()],
            rows: vec![vec!["Ana".into(), "ana.png".into(), "logo.png".into()]],
            kinds: vec![FieldKind::Text, FieldKind::Image, FieldKind::Image],
            path: Some(dir.join("cards.csv")),
            ..Default::default()
        };
        perform(&mut app, &st);
        let doc = app.doc();
        let page = &doc.pages[1];
        fn walk<'a>(s: &'a Shape, out: &mut Vec<&'a Shape>) {
            out.push(s);
            match &s.kind {
                ShapeKind::Group { children } => children.iter().for_each(|c| walk(c, out)),
                ShapeKind::ClipFrame { frame, contents } => {
                    walk(frame, out);
                    contents.iter().for_each(|c| walk(c, out));
                }
                _ => {}
            }
        }
        let mut all = Vec::new();
        for s in page.layers.iter().flat_map(|l| &l.shapes) {
            walk(s, &mut all);
        }
        let texts: Vec<String> = all.iter().filter_map(|s| text_of(s)).collect();
        assert!(texts.contains(&"Card of Ana".to_string()), "{texts:?}");
        // The photo covers the 30 x 30 frame: 30 wide, 60 tall.
        let frames: Vec<&&Shape> = all
            .iter()
            .filter(|s| matches!(s.kind, ShapeKind::ClipFrame { .. }))
            .collect();
        assert_eq!(frames.len(), 2, "the photo frame and the logo frame");
        let bitmaps: Vec<Rect> = all
            .iter()
            .filter_map(|s| match &s.kind {
                ShapeKind::Bitmap { .. } => Some(s.bounds()),
                _ => None,
            })
            .collect();
        assert_eq!(bitmaps.len(), 2, "{bitmaps:?} {texts:?}");
        assert!(
            bitmaps
                .iter()
                .any(|b| (b.width() - 30.0).abs() < 1e-6 && (b.height() - 60.0).abs() < 1e-6),
            "{bitmaps:?}"
        );
        // The logo (3 : 1) covers its 30 x 30 rectangle: 90 x 30.
        assert!(
            bitmaps
                .iter()
                .any(|b| (b.width() - 90.0).abs() < 1e-6 && (b.height() - 30.0).abs() < 1e-6),
            "{bitmaps:?}"
        );
        // The template page keeps its placeholders.
        let mut tpl = Vec::new();
        for s in doc.pages[0].layers.iter().flat_map(|l| &l.shapes) {
            walk(s, &mut tpl);
        }
        let tpl_texts: Vec<String> = tpl.iter().filter_map(|s| text_of(s)).collect();
        assert!(tpl_texts.contains(&"<Photo>".to_string()), "{tpl_texts:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Align and scale: the four sizes, and the anchor placing what does
    /// not fill the area (y grows upward: row 0 is the top).
    #[test]
    fn pictures_are_placed_by_scale_and_anchor() {
        let area = Rect::new(0.0, 0.0, 40.0, 20.0);
        let near = |a: Rect, b: Rect| {
            (a.x0 - b.x0).abs() < 1e-9
                && (a.y0 - b.y0).abs() < 1e-9
                && (a.x1 - b.x1).abs() < 1e-9
                && (a.y1 - b.y1).abs() < 1e-9
        };
        // A 10 x 10 picture.
        let r = place(area, 10.0, 10.0, Scale::Actual, 0);
        assert!(near(r, Rect::new(0.0, 10.0, 10.0, 20.0)), "{r:?}");
        let r = place(area, 10.0, 10.0, Scale::Actual, 8);
        assert!(near(r, Rect::new(30.0, 0.0, 40.0, 10.0)), "{r:?}");
        let r = place(area, 10.0, 10.0, Scale::Fit, 4);
        assert!(near(r, Rect::new(10.0, 0.0, 30.0, 20.0)), "{r:?}");
        let r = place(area, 10.0, 10.0, Scale::Fit, 3);
        assert!(near(r, Rect::new(0.0, 0.0, 20.0, 20.0)), "{r:?}");
        let r = place(area, 10.0, 10.0, Scale::Fill, 4);
        assert!(near(r, Rect::new(0.0, -10.0, 40.0, 30.0)), "{r:?}");
        let r = place(area, 10.0, 10.0, Scale::Stretch, 0);
        assert!(near(r, area));
    }

    /// ID cards from a folder of photos (#460): the cells name the photos
    /// (with or without an extension), the folder is chosen in the docker,
    /// and each card gets its own.
    #[test]
    fn each_record_gets_its_photo_from_the_pictures_folder() {
        let dir = std::env::temp_dir().join(format!("td_merge_photos_{}", std::process::id()));
        let photos = dir.join("photos");
        std::fs::create_dir_all(&photos).unwrap();
        image::RgbImage::from_pixel(10, 20, image::Rgb([200, 0, 0]))
            .save(photos.join("1001.jpg"))
            .unwrap();
        image::RgbaImage::from_pixel(30, 10, image::Rgba([0, 0, 200, 255]))
            .save(photos.join("1002.png"))
            .unwrap();
        let mut app = App::headless();
        let frame = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 40.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("frame");
        app.run(Command::SetShapeName {
            shape: frame,
            name: Some("<Photo>".into()),
        });
        let st = PrintMergeState {
            headers: vec!["Photo".into()],
            rows: vec![vec!["1001".into()], vec!["1002.png".into()]],
            kinds: vec![FieldKind::Image],
            // The data file is somewhere else.
            path: Some(dir.join("data").join("cards.xlsx")),
            pictures: Some(photos.clone()),
            ..Default::default()
        };
        perform(&mut app, &st);
        let doc = app.doc();
        let size_on = |page: usize| -> Option<(u32, u32)> {
            fn find(s: &Shape) -> Option<(u32, u32)> {
                match &s.kind {
                    ShapeKind::Bitmap {
                        width_px,
                        height_px,
                        ..
                    } => Some((*width_px, *height_px)),
                    ShapeKind::ClipFrame { contents, .. } => contents.iter().find_map(find),
                    _ => None,
                }
            }
            doc.pages[page]
                .layers
                .iter()
                .flat_map(|l| &l.shapes)
                .find_map(find)
        };
        assert_eq!(size_on(1), Some((10, 20)), "the first card, its photo");
        assert_eq!(size_on(2), Some((30, 10)), "the second card, its own");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn field_types_are_guessed_from_the_data() {
        let h: Vec<String> = ["Name", "Foto", "Code QR", "Badge"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let rows = vec![vec![
            "Ana".to_string(),
            "a.jpg".into(),
            "x".into(),
            "pics/b.PNG".into(),
        ]];
        assert_eq!(
            guess_kinds(&h, &rows),
            vec![
                FieldKind::Text,
                FieldKind::Image,
                FieldKind::Qr,
                FieldKind::Image
            ]
        );
    }
}
