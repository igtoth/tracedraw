//! HPGL plotter files (`.plt`, `.hpgl`, `.hgl`): import of the pen
//! commands and export of a page as pen strokes.
//!
//! Units are plotter units, 1/40 mm (0.025 mm), origin at the bottom
//! left with Y up, which is the document's own orientation. The reader
//! understands the HPGL and HPGL/2 commands that carry geometry: `IN`,
//! `SP`, `PU`, `PD`, `PA`, `PR`, `CI`, `AA`, `AR`, `EA`, `ER`, `RA`,
//! `RR`, `PM`, `EP`, `FP`, `PW`, `LT`, `PC`, `IP`, `SC` (uniform scaling
//! only). Everything else is skipped. The writer emits `PA` moves with
//! `PU`/`PD`, one pen per distinct outline colour (up to 8, more share
//! pen 8), `PW` widths and `PC` colours (HPGL/2).

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{self, Write};

use tracedraw_core::geometry::{Affine, BezPath, PathEl, Point, Rect, Shape as _, Size};
use tracedraw_core::id::IdSource;
use tracedraw_core::{Color, Document, Fill, Shape, ShapeKind, Stroke};

/// Plotter units per millimetre.
pub const PLU_PER_MM: f64 = 40.0;
const MAX_COMMANDS: usize = 2_000_000;
const MAX_POINTS: usize = 4_000_000;

#[derive(Debug, Clone)]
pub struct Imported {
    pub shapes: Vec<Shape>,
    pub size: Size,
    pub warnings: Vec<String>,
}

/// True when the bytes look like an HPGL file (a pen command near the start).
pub fn is_plt(bytes: &[u8]) -> bool {
    let head: String = bytes
        .iter()
        .take(512)
        .map(|&b| b as char)
        .collect::<String>()
        .to_ascii_uppercase();
    head.contains("IN;")
        || head.contains("PU")
        || head.contains("PD")
        || head.contains("SP")
        || head.starts_with("\x1b")
}

/// Default pen colours for pens 1 to 8 (the common plotter carousel).
fn pen_color(n: usize) -> Color {
    match n {
        1 => Color::BLACK,
        2 => Color::rgb8(255, 0, 0),
        3 => Color::rgb8(0, 160, 0),
        4 => Color::rgb8(255, 255, 0),
        5 => Color::rgb8(0, 0, 255),
        6 => Color::rgb8(255, 0, 255),
        7 => Color::rgb8(0, 200, 200),
        _ => Color::rgb8(255, 128, 0),
    }
}

struct Plotter {
    pos: Point,
    down: bool,
    relative: bool,
    pen: usize,
    pen_width_mm: HashMap<usize, f64>,
    pen_colors: HashMap<usize, Color>,
    /// Polygon mode buffer (`PM`), drawn by `EP` (edge) or `FP` (fill).
    polygon: Option<BezPath>,
    /// Current path: open stroke being drawn with the pen down.
    current: BezPath,
    current_points: usize,
    shapes: Vec<Shape>,
    points: usize,
    scale: f64,
    offset: Point,
    bounds: Option<Rect>,
    warnings: Vec<String>,
}

impl Plotter {
    fn new() -> Self {
        Plotter {
            pos: Point::ZERO,
            down: false,
            relative: false,
            pen: 1,
            pen_width_mm: HashMap::new(),
            pen_colors: HashMap::new(),
            polygon: None,
            current: BezPath::new(),
            current_points: 0,
            shapes: Vec::new(),
            points: 0,
            scale: 1.0 / PLU_PER_MM,
            offset: Point::ZERO,
            bounds: None,
            warnings: Vec::new(),
        }
    }

    fn mm(&self, p: Point) -> Point {
        Point::new(
            self.offset.x + p.x * self.scale,
            self.offset.y + p.y * self.scale,
        )
    }

    fn stroke(&self) -> Stroke {
        let mut s = Stroke::new(
            self.pen_colors
                .get(&self.pen)
                .copied()
                .unwrap_or_else(|| pen_color(self.pen)),
            self.pen_width_mm
                .get(&self.pen)
                .copied()
                .unwrap_or(0.35)
                .max(Stroke::HAIRLINE),
        );
        s.scale_with_object = true;
        s
    }

    fn note(&mut self, p: Point) {
        self.bounds = Some(match self.bounds {
            Some(b) => b.union_pt(p),
            None => Rect::from_points(p, p),
        });
    }

    fn flush(&mut self, ids: &mut IdSource) {
        if self.current_points >= 2 {
            let path = std::mem::take(&mut self.current);
            let mut s = Shape::new(
                ids.shape(),
                ShapeKind::Path {
                    path,
                    closed: false,
                },
            );
            s.fill = Fill::None;
            s.stroke = Some(self.stroke());
            self.shapes.push(s);
        } else {
            self.current = BezPath::new();
        }
        self.current_points = 0;
    }

    fn move_to(&mut self, target: Point, ids: &mut IdSource) {
        let p = self.mm(target);
        if self.points >= MAX_POINTS {
            return;
        }
        self.points += 1;
        let prev = self.mm(self.pos);
        let down = self.down;
        if let Some(poly) = &mut self.polygon {
            if down {
                if poly.elements().is_empty() {
                    poly.move_to(prev);
                }
                poly.line_to(p);
            } else {
                poly.move_to(p);
            }
        } else if self.down {
            if self.current_points == 0 {
                self.current.move_to(self.mm(self.pos));
                self.current_points = 1;
                self.note(self.mm(self.pos));
            }
            self.current.line_to(p);
            self.current_points += 1;
            self.note(p);
        } else {
            self.flush(ids);
        }
        self.pos = target;
    }

    fn pen_up(&mut self, ids: &mut IdSource) {
        if self.down {
            self.down = false;
            self.flush(ids);
        }
    }

    fn arc_path(&self, center: Point, radius_plu: f64, start: f64, sweep: f64) -> BezPath {
        let c = self.mm(center);
        let r = radius_plu * self.scale;
        let mut path = BezPath::new();
        let segs = ((sweep.abs() / 90.0).ceil() as usize).max(1);
        let step = sweep / segs as f64;
        let mut a0 = start.to_radians();
        path.move_to(Point::new(c.x + r * a0.cos(), c.y + r * a0.sin()));
        for _ in 0..segs {
            let a1 = a0 + step.to_radians();
            let k = 4.0 / 3.0 * ((a1 - a0) / 4.0).tan();
            let p0 = Point::new(c.x + r * a0.cos(), c.y + r * a0.sin());
            let p3 = Point::new(c.x + r * a1.cos(), c.y + r * a1.sin());
            let p1 = Point::new(p0.x - k * r * a0.sin(), p0.y + k * r * a0.cos());
            let p2 = Point::new(p3.x + k * r * a1.sin(), p3.y - k * r * a1.cos());
            path.curve_to(p1, p2, p3);
            a0 = a1;
        }
        path
    }

    fn add_path(&mut self, path: BezPath, fill: bool, ids: &mut IdSource) {
        self.flush(ids);
        let b = path.bounding_box();
        self.note(Point::new(b.x0, b.y0));
        self.note(Point::new(b.x1, b.y1));
        let mut s = Shape::new(ids.shape(), ShapeKind::Path { path, closed: true });
        let stroke = self.stroke();
        if fill {
            s.fill = Fill::Solid(stroke.color);
            s.stroke = None;
        } else {
            s.fill = Fill::None;
            s.stroke = Some(stroke);
        }
        self.shapes.push(s);
    }
}

/// Split the file into (mnemonic, numeric arguments) pairs.
fn commands(text: &str) -> Vec<(String, Vec<f64>)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() && out.len() < MAX_COMMANDS {
        let b = bytes[i];
        if b == 0x1b {
            // PJL / escape sequences: skip to the end of the line.
            while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b';' {
                i += 1;
            }
            continue;
        }
        if !b.is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        if i + 1 >= bytes.len() || !bytes[i + 1].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let name = String::from_utf8_lossy(&bytes[i..i + 2]).to_ascii_uppercase();
        i += 2;
        // Label text runs to the terminator (ETX) and carries no geometry.
        if name == "LB" {
            while i < bytes.len() && bytes[i] != 0x03 {
                i += 1;
            }
            i += 1;
            out.push((name, Vec::new()));
            continue;
        }
        let mut args = Vec::new();
        let mut num = String::new();
        while i < bytes.len() {
            let c = bytes[i];
            if c.is_ascii_digit() || c == b'.' || c == b'-' || c == b'+' {
                num.push(c as char);
                i += 1;
            } else if c == b',' || c == b' ' || c == b'\t' || c == b'\r' || c == b'\n' {
                if !num.is_empty() {
                    if let Ok(v) = num.parse::<f64>() {
                        args.push(v);
                    }
                    num.clear();
                }
                i += 1;
            } else if c == b';' {
                i += 1;
                break;
            } else {
                break;
            }
        }
        if !num.is_empty() {
            if let Ok(v) = num.parse::<f64>() {
                args.push(v);
            }
        }
        out.push((name, args));
    }
    out
}

/// Parse an HPGL file.
pub fn parse(bytes: &[u8], ids: &mut IdSource) -> Result<Imported, String> {
    if !is_plt(bytes) {
        return Err("not an HPGL file".into());
    }
    let text: String = bytes.iter().map(|&b| b as char).collect();
    let cmds = commands(&text);
    if cmds.is_empty() {
        return Err("no plotter commands".into());
    }
    let mut pl = Plotter::new();
    let mut p1 = Point::ZERO;
    let mut p2: Option<Point> = None;
    for (name, args) in &cmds {
        let a = |k: usize| args.get(k).copied().filter(|v| v.is_finite());
        match name.as_str() {
            "IN" | "DF" => {
                pl.pen_up(ids);
                pl.relative = false;
                pl.pos = Point::ZERO;
            }
            "SP" => {
                pl.pen_up(ids);
                pl.pen = a(0).map(|v| v.max(0.0) as usize).unwrap_or(0);
            }
            "PW" => {
                let w = a(0).unwrap_or(0.35).max(0.0);
                match a(1) {
                    Some(p) => {
                        pl.pen_width_mm.insert(p.max(0.0) as usize, w);
                    }
                    None => {
                        for k in 0..=16 {
                            pl.pen_width_mm.insert(k, w);
                        }
                    }
                }
            }
            "PC" => {
                if let (Some(p), Some(r), Some(g), Some(b)) = (a(0), a(1), a(2), a(3)) {
                    pl.pen_colors.insert(
                        p.max(0.0) as usize,
                        Color::rgb8(
                            r.clamp(0.0, 255.0) as u8,
                            g.clamp(0.0, 255.0) as u8,
                            b.clamp(0.0, 255.0) as u8,
                        ),
                    );
                }
            }
            "PA" | "PR" | "PU" | "PD" => {
                match name.as_str() {
                    "PA" => pl.relative = false,
                    "PR" => pl.relative = true,
                    "PU" => pl.pen_up(ids),
                    _ => pl.down = true,
                }
                for pair in args.chunks_exact(2) {
                    if !pair[0].is_finite() || !pair[1].is_finite() {
                        continue;
                    }
                    let target = if pl.relative {
                        Point::new(pl.pos.x + pair[0], pl.pos.y + pair[1])
                    } else {
                        Point::new(pair[0], pair[1])
                    };
                    pl.move_to(target, ids);
                }
            }
            "CI" => {
                if let Some(r) = a(0) {
                    let path = pl.arc_path(pl.pos, r.abs(), 0.0, 360.0);
                    pl.add_path(path, false, ids);
                }
            }
            "AA" | "AR" => {
                if let (Some(x), Some(y), Some(sweep)) = (a(0), a(1), a(2)) {
                    let center = if name == "AR" {
                        Point::new(pl.pos.x + x, pl.pos.y + y)
                    } else {
                        Point::new(x, y)
                    };
                    let dx = pl.pos.x - center.x;
                    let dy = pl.pos.y - center.y;
                    let r = (dx * dx + dy * dy).sqrt();
                    let start = dy.atan2(dx).to_degrees();
                    let sweep = sweep.clamp(-360.0, 360.0);
                    let end = (start + sweep).to_radians();
                    let endpoint = Point::new(center.x + r * end.cos(), center.y + r * end.sin());
                    if pl.down && pl.polygon.is_none() {
                        let arc = pl.arc_path(center, r, start, sweep);
                        if pl.current_points == 0 {
                            pl.current.move_to(pl.mm(pl.pos));
                            pl.current_points = 1;
                        }
                        for el in arc.elements().iter().skip(1) {
                            pl.current.push(*el);
                            pl.current_points += 1;
                        }
                        let b = arc.bounding_box();
                        pl.note(Point::new(b.x0, b.y0));
                        pl.note(Point::new(b.x1, b.y1));
                    } else if pl.polygon.is_some() {
                        let arc = pl.arc_path(center, r, start, sweep);
                        let prev = pl.mm(pl.pos);
                        if let Some(poly) = &mut pl.polygon {
                            if poly.elements().is_empty() {
                                poly.move_to(prev);
                            }
                            for el in arc.elements().iter().skip(1) {
                                poly.push(*el);
                            }
                        }
                    }
                    pl.pos = endpoint;
                }
            }
            "EA" | "ER" | "RA" | "RR" => {
                if let (Some(x), Some(y)) = (a(0), a(1)) {
                    let other = if name.ends_with('R') {
                        Point::new(pl.pos.x + x, pl.pos.y + y)
                    } else {
                        Point::new(x, y)
                    };
                    let r = Rect::from_points(pl.mm(pl.pos), pl.mm(other));
                    pl.add_path(r.to_path(0.01), name.starts_with('R'), ids);
                }
            }
            "PM" => match a(0).unwrap_or(0.0) as i64 {
                0 => {
                    pl.flush(ids);
                    pl.polygon = Some(BezPath::new());
                }
                1 => {
                    if let Some(poly) = &mut pl.polygon {
                        poly.close_path();
                    }
                }
                _ => {
                    if let Some(poly) = &mut pl.polygon {
                        poly.close_path();
                    }
                }
            },
            "EP" | "FP" => {
                if let Some(poly) = pl.polygon.clone() {
                    if !poly.elements().is_empty() {
                        pl.add_path(poly, name == "FP", ids);
                    }
                }
            }
            "IP" => {
                if let (Some(x), Some(y)) = (a(0), a(1)) {
                    p1 = Point::new(x, y);
                    p2 = match (a(2), a(3)) {
                        (Some(x2), Some(y2)) => Some(Point::new(x2, y2)),
                        _ => None,
                    };
                }
            }
            "SC" => {
                // User units: map xmin..xmax to P1..P2 (uniform only).
                match (a(0), a(1), a(2), a(3), p2) {
                    (Some(xmin), Some(xmax), Some(ymin), Some(ymax), Some(p2))
                        if (xmax - xmin).abs() > 1e-9 && (ymax - ymin).abs() > 1e-9 =>
                    {
                        let sx = (p2.x - p1.x) / (xmax - xmin);
                        let sy = (p2.y - p1.y) / (ymax - ymin);
                        let s = if (sx - sy).abs() < 1e-6 {
                            sx
                        } else {
                            sx.min(sy)
                        };
                        if s.is_finite() && s > 0.0 {
                            pl.scale = s / PLU_PER_MM;
                            pl.offset = Point::new(
                                (p1.x - xmin * s) / PLU_PER_MM,
                                (p1.y - ymin * s) / PLU_PER_MM,
                            );
                        }
                    }
                    (None, ..) => {
                        pl.scale = 1.0 / PLU_PER_MM;
                        pl.offset = Point::ZERO;
                    }
                    _ => pl.warnings.push("SC: non-uniform scaling ignored".into()),
                }
            }
            "LB" | "LT" | "VS" | "SI" | "SR" | "DI" | "DR" | "CS" | "CA" | "SS" | "SA" | "PG"
            | "AF" | "AH" | "EC" | "NR" | "PS" | "RO" | "IW" | "CP" | "LO" | "TL" | "XT" | "YT"
            | "FT" | "WG" | "EW" | "BP" | "NP" | "MC" | "TR" | "QL" | "PE" | "AC" | "SM" | "SL"
            | "ES" | "FI" | "FN" | "SB" | "SD" | "AD" | "DT" | "DV" | "LA" | "RF" | "SV" | "UL"
            | "WU" | "CR" | "CO" | "DL" | "RT" | "BZ" | "BR" => {}
            _ => {}
        }
    }
    pl.pen_up(ids);
    pl.flush(ids);
    if pl.shapes.is_empty() {
        return Err("no drawable commands".into());
    }
    if pl.points >= MAX_POINTS {
        pl.warnings
            .push("point limit reached; the drawing is truncated".into());
    }
    let b = pl
        .bounds
        .unwrap_or_else(|| Rect::new(0.0, 0.0, 297.0, 210.0));
    let size = Size::new(
        (b.x1.max(0.0) + 10.0).clamp(50.0, 5000.0),
        (b.y1.max(0.0) + 10.0).clamp(50.0, 5000.0),
    );
    Ok(Imported {
        shapes: pl.shapes,
        size,
        warnings: pl.warnings,
    })
}

/// A document with the imported strokes on one page.
pub fn to_document(imp: Imported, title: &str) -> Document {
    let mut doc = Document::new(title, imp.size);
    let page = doc.pages[0].id;
    if let Ok(p) = doc.page_mut(page) {
        if let Some(layer) = p.layers.first_mut() {
            layer.shapes = imp.shapes;
        }
    }
    doc
}

// ----- writer ------------------------------------------------------------------

struct Writer<'a> {
    out: String,
    symbols: &'a [tracedraw_core::Symbol],
    /// Pen per outline colour, in order of first use.
    pens: Vec<(Color, f64)>,
    /// Strokes per pen: polylines in plotter units.
    strokes: Vec<Vec<Vec<Point>>>,
}

impl Writer<'_> {
    fn pen_for(&mut self, color: Color, width: f64) -> usize {
        if let Some(i) = self
            .pens
            .iter()
            .position(|(c, w)| *c == color && (*w - width).abs() < 1e-6)
        {
            return i;
        }
        self.pens.push((color, width));
        self.strokes.push(Vec::new());
        self.pens.len() - 1
    }

    fn polylines(path: &BezPath) -> Vec<Vec<Point>> {
        let mut out: Vec<Vec<Point>> = Vec::new();
        let mut cur: Vec<Point> = Vec::new();
        let mut start = Point::ZERO;
        tracedraw_core::geometry::flatten(path, 0.02, &mut |el| match el {
            PathEl::MoveTo(p) => {
                if cur.len() >= 2 {
                    out.push(std::mem::take(&mut cur));
                } else {
                    cur.clear();
                }
                start = p;
                cur.push(p);
            }
            PathEl::LineTo(p) => cur.push(p),
            PathEl::ClosePath if cur.len() >= 2 => cur.push(start),
            _ => {}
        });
        if cur.len() >= 2 {
            out.push(cur);
        }
        out
    }

    fn shape(&mut self, shape: &Shape, parent: Affine) {
        if !shape.visible {
            return;
        }
        if !shape.effects.is_empty() {
            let ev = tracedraw_core::live::evaluate(shape);
            for s in ev
                .below
                .iter()
                .chain(std::iter::once(&ev.main))
                .chain(ev.above.iter())
            {
                let mut s = s.clone();
                s.effects.clear();
                self.shape(&s, parent);
            }
            return;
        }
        let transform = parent * shape.transform;
        match &shape.kind {
            ShapeKind::Group { children } => {
                for c in children {
                    self.shape(c, transform);
                }
                return;
            }
            ShapeKind::Table(_) | ShapeKind::SymbolInstance { .. } => {
                for c in shape.expand(self.symbols) {
                    self.shape(&c, transform);
                }
                return;
            }
            ShapeKind::ClipFrame { frame, contents } => {
                self.shape(frame, transform);
                for c in contents {
                    self.shape(c, transform);
                }
                return;
            }
            ShapeKind::Bitmap { .. } => return,
            _ => {}
        }
        let path = transform * shape.local_path();
        if path.elements().is_empty() {
            return;
        }
        // A plotter draws outlines: filled shapes without an outline are
        // drawn with their fill colour as a thin pen.
        let (color, width) = match (&shape.stroke, &shape.fill) {
            (Some(s), _) => {
                let scale = {
                    let c = transform.as_coeffs();
                    (c[0] * c[3] - c[1] * c[2]).abs().sqrt()
                };
                let w = if s.scale_with_object {
                    s.width * scale
                } else {
                    s.width
                };
                (s.color, w.max(Stroke::HAIRLINE))
            }
            (None, Fill::Solid(c)) => (*c, 0.25),
            (None, Fill::None) => return,
            (None, _) => (Color::BLACK, 0.25),
        };
        let pen = self.pen_for(color, width);
        let polys = Self::polylines(&path);
        for pts in polys {
            let plu: Vec<Point> = pts
                .iter()
                .map(|p| Point::new(p.x * PLU_PER_MM, p.y * PLU_PER_MM))
                .collect();
            self.strokes[pen].push(plu);
        }
    }
}

/// Write one page as HPGL/2 text.
pub fn page_to_plt(doc: &Document, page_index: usize) -> String {
    let mut w = Writer {
        out: String::new(),
        symbols: &doc.symbols,
        pens: Vec::new(),
        strokes: Vec::new(),
    };
    let Some(page) = doc.pages.get(page_index) else {
        return String::new();
    };
    let layers = doc.layers_for_page(page.id).unwrap_or_default();
    for layer in layers.iter().filter(|l| l.visible && l.printable) {
        for s in &layer.shapes {
            w.shape(s, Affine::IDENTITY);
        }
    }
    let _ = write!(w.out, "IN;");
    let _ = write!(
        w.out,
        "IP0,0,{},{};",
        (page.size.width * PLU_PER_MM).round() as i64,
        (page.size.height * PLU_PER_MM).round() as i64
    );
    let fmt = |v: f64| -> String { format!("{}", v.round() as i64) };
    for (i, (color, width)) in w.pens.iter().enumerate() {
        let pen = (i + 1).min(8);
        let [r, g, b] = color.to_rgb8();
        let _ = write!(w.out, "PC{pen},{r},{g},{b};PW{:.3},{pen};", width);
    }
    let _ = writeln!(w.out);
    for (i, strokes) in w.strokes.iter().enumerate() {
        let pen = (i + 1).min(8);
        let _ = write!(w.out, "SP{pen};");
        for pts in strokes {
            let Some(first) = pts.first() else { continue };
            let _ = write!(w.out, "PU{},{};PD", fmt(first.x), fmt(first.y));
            let rest: Vec<String> = pts
                .iter()
                .skip(1)
                .map(|p| format!("{},{}", fmt(p.x), fmt(p.y)))
                .collect();
            let _ = writeln!(w.out, "{};", rest.join(","));
        }
    }
    let _ = write!(w.out, "PU;SP0;IN;");
    w.out
}

pub fn write_plt(doc: &Document, page_index: usize, out: &mut dyn Write) -> io::Result<()> {
    out.write_all(page_to_plt(doc, page_index).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_pen_moves_circles_and_rectangles() {
        let src = b"IN;SP1;PA;PU0,0;PD4000,0,4000,4000;PU;SP2;PW0.5;PU1000,1000;CI400;EA2000,2000;PR;PU100,100;PD200,0;PU;";
        let mut ids = IdSource::default();
        let imp = parse(src, &mut ids).unwrap();
        assert_eq!(imp.shapes.len(), 4);
        // The first stroke: 100 mm along x then 100 mm up, pen 1 black.
        let s = &imp.shapes[0];
        let b = s.bounds();
        assert!((b.width() - 100.0).abs() < 1e-6 && (b.height() - 100.0).abs() < 1e-6);
        assert_eq!(s.stroke.as_ref().map(|s| s.color), Some(Color::BLACK));
        // The circle: radius 10 mm at (25, 25), pen 2 red, 0.5 mm.
        let c = &imp.shapes[1];
        let cb = c.bounds();
        assert!((cb.width() - 20.0).abs() < 0.05, "{cb:?}");
        assert!((cb.center().x - 25.0).abs() < 0.05);
        let st = c.stroke.as_ref().unwrap();
        assert_eq!(st.color, Color::rgb8(255, 0, 0));
        assert!((st.width - 0.5).abs() < 1e-9);
        // The rectangle from (25,25) to (50,50).
        let r = imp.shapes[2].bounds();
        assert!((r.x0 - 25.0).abs() < 1e-6 && (r.x1 - 50.0).abs() < 1e-6);
        // The relative stroke: from (27.5+2.5, ...) 5 mm long.
        let rel = imp.shapes[3].bounds();
        assert!((rel.width() - 5.0).abs() < 1e-6, "{rel:?}");
        assert!(imp.size.width >= 110.0);
    }

    #[test]
    fn polygon_mode_fills_and_edges() {
        let src = b"IN;SP1;PU0,0;PM0;PD4000,0,4000,4000,0,4000;PM2;FP;EP;";
        let mut ids = IdSource::default();
        let imp = parse(src, &mut ids).unwrap();
        assert_eq!(imp.shapes.len(), 2);
        assert!(matches!(imp.shapes[0].fill, Fill::Solid(_)));
        assert!(imp.shapes[0].stroke.is_none());
        assert!(imp.shapes[1].stroke.is_some());
    }

    #[test]
    fn rejects_other_files_and_survives_garbage() {
        let mut ids = IdSource::default();
        assert!(parse(b"%PDF-1.4", &mut ids).is_err());
        assert!(parse(b"IN;SP1;", &mut ids).is_err(), "nothing drawn");
        let mut junk = b"IN;SP1;".to_vec();
        for k in 0..5000u32 {
            junk.extend_from_slice(b"PD");
            junk.push(b'0' + (k % 10) as u8);
            junk.push(if k % 7 == 0 { b';' } else { b',' });
            junk.push((k % 256) as u8);
        }
        let _ = parse(&junk, &mut ids);
        // A file that is all one giant polyline stops at the point limit.
        let _ = parse(b"IN;SP1;PD;" as &[u8], &mut ids);
    }

    #[test]
    fn export_round_trips_strokes_and_pens() {
        let mut doc = Document::new("t", Size::new(100.0, 100.0));
        let mut ids = doc.ids().clone();
        let mut r = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(10.0, 20.0, 60.0, 70.0),
                radius: 0.0,
            },
        );
        r.fill = Fill::None;
        r.stroke = Some(Stroke::new(Color::rgb8(0, 0, 255), 0.7));
        let mut e = Shape::new(
            ids.shape(),
            ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 40.0, 40.0),
                arc: None,
            },
        );
        e.fill = Fill::Solid(Color::rgb8(0, 128, 0));
        e.stroke = None;
        doc.pages[0].layers[0].shapes.push(r);
        doc.pages[0].layers[0].shapes.push(e);
        doc.set_ids(ids);
        let text = page_to_plt(&doc, 0);
        assert!(text.starts_with("IN;IP0,0,4000,4000;"));
        assert!(text.contains("PC1,0,0,255;PW0.700,1;"));
        assert!(text.contains("SP1;PU400,800;PD"));
        assert!(text.contains("SP2;"));
        let mut ids2 = IdSource::default();
        let back = parse(text.as_bytes(), &mut ids2).unwrap();
        assert_eq!(back.shapes.len(), 2);
        let rb = back.shapes[0].bounds();
        assert!(
            (rb.x0 - 10.0).abs() < 0.03 && (rb.y1 - 70.0).abs() < 0.03,
            "{rb:?}"
        );
        let st = back.shapes[0].stroke.as_ref().unwrap();
        assert_eq!(st.color, Color::rgb8(0, 0, 255));
        assert!((st.width - 0.7).abs() < 1e-6);
        let eb = back.shapes[1].bounds();
        assert!((eb.width() - 40.0).abs() < 0.1, "{eb:?}");
        assert_eq!(
            back.shapes[1].stroke.as_ref().map(|s| s.color),
            Some(Color::rgb8(0, 128, 0))
        );
    }
}
