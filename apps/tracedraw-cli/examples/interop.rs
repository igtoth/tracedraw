//! Writes the interoperability test drawing, `interop.tdraw`, which the
//! bundles under `testdata/interop/` are made from, one `.cdr` file per
//! version:
//!
//! ```text
//! cargo run -p tracedraw-cli --example interop -- <out-dir>
//! for v in $(seq 7 27); do
//!   TRACEDRAW_CDR_VERSION=$v TRACEDRAW_CDR_EXPERIMENTAL=1 \
//!     tracedraw-cli convert <out-dir>/interop.tdraw <out-dir>/v$(printf %02d $v)_drawing.cdr
//! done
//! ```
//!
//! One page with the object kinds the writer handles: a rectangle with a
//! fill and an outline, an ellipse, a filled curve, a line, a gradient,
//! a group of two shapes, an open polyline, and artistic text (plain with
//! accents, runs in bold and italic with a colour of their own, right
//! aligned and centred). Everything is drawn here from scratch in Arial,
//! which falls back to the system's sans serif when missing. Three parts
//! of it are written too, to tell a record kind apart when a file fails:
//! `interop-empty.tdraw` (the page alone), `interop-shapes.tdraw` (the
//! shapes without the texts) and `interop-text.tdraw` (the texts alone).

use std::path::Path;
use tracedraw_core::{
    Affine, BezPath, Color, Document, Fill, Fountain, FountainKind, ParagraphStyle, Point, Rect,
    Shape, ShapeKind, Size, Stroke, TextAlign, TextSpan,
};

const FONT: &str = "Arial";

fn hex(s: &str) -> Color {
    Color::from_hex(s).unwrap_or(Color::BLACK)
}

/// A rectangle from its top-left corner, width and height (y up).
fn rect(doc: &mut Document, x: f64, top: f64, w: f64, h: f64) -> Shape {
    let id = doc.ids_mut().shape();
    Shape::new(
        id,
        ShapeKind::Rect {
            rect: Rect::new(x, top - h, x + w, top),
            radius: 0.0,
            corners: None,
        },
    )
}

fn ellipse(doc: &mut Document, cx: f64, cy: f64, rx: f64, ry: f64) -> Shape {
    let id = doc.ids_mut().shape();
    Shape::new(
        id,
        ShapeKind::Ellipse {
            rect: Rect::new(cx - rx, cy - ry, cx + rx, cy + ry),
            arc: None,
        },
    )
}

fn path(doc: &mut Document, p: BezPath, closed: bool) -> Shape {
    let id = doc.ids_mut().shape();
    Shape::new(id, ShapeKind::Path { path: p, closed })
}

fn text(doc: &mut Document, spans: Vec<TextSpan>, x: f64, y: f64, align: TextAlign) -> Shape {
    let id = doc.ids_mut().shape();
    let mut s = Shape::new(
        id,
        ShapeKind::Text {
            spans,
            origin: Point::ZERO,
            frame: None,
            align,
            para: ParagraphStyle::default(),
            on_path: None,
        },
    );
    s.transform = Affine::translate((x, y));
    s.fill = Fill::Solid(Color::BLACK);
    s.stroke = None;
    s
}

fn styled(mut s: Shape, fill: Option<Color>, stroke: Option<(Color, f64)>, name: &str) -> Shape {
    s.fill = fill.map(Fill::Solid).unwrap_or(Fill::None);
    s.stroke = stroke.map(|(c, w)| Stroke::new(c, w));
    s.name = Some(name.into());
    s
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let dir = Path::new(&dir);
    let page = Size::new(210.0, 148.0);
    let mut doc = Document::new("interop", page);
    // Coordinates are millimetres with y up from the page's bottom edge;
    // the rows below are given by their top.
    let mut shapes = Vec::new();
    let r = rect(&mut doc, 15.0, 133.0, 40.0, 30.0);
    shapes.push(styled(
        r,
        Some(hex("#3366cc")),
        Some((Color::BLACK, 0.5)),
        "rectangle",
    ));
    let e = ellipse(&mut doc, 90.0, 118.0, 20.0, 15.0);
    shapes.push(styled(
        e,
        Some(hex("#33cc33")),
        Some((hex("#003300"), 1.0)),
        "ellipse",
    ));
    let mut curve = BezPath::new();
    curve.move_to((120.0, 133.0));
    curve.curve_to((135.0, 143.0), (150.0, 103.0), (165.0, 133.0));
    curve.line_to((180.0, 108.0));
    curve.curve_to((160.0, 98.0), (140.0, 98.0), (120.0, 108.0));
    curve.close_path();
    let c = path(&mut doc, curve, true);
    shapes.push(styled(c, Some(hex("#cc3399")), None, "curve"));
    let mut line = BezPath::new();
    line.move_to((15.0, 88.0));
    line.line_to((80.0, 73.0));
    let l = path(&mut doc, line, false);
    shapes.push(styled(l, None, Some((Color::BLACK, 1.5)), "line"));
    let mut g = rect(&mut doc, 95.0, 93.0, 50.0, 25.0);
    g.fill = Fill::Fountain(Fountain::two(
        FountainKind::Linear,
        hex("#ffcc00"),
        hex("#cc0000"),
        0.0,
    ));
    g.stroke = Some(Stroke::new(Color::BLACK, 0.25));
    g.name = Some("gradient".into());
    shapes.push(g);
    let a = rect(&mut doc, 155.0, 93.0, 20.0, 20.0);
    let a = styled(a, Some(hex("#ff9900")), None, "group square");
    let b = ellipse(&mut doc, 185.0, 83.0, 10.0, 10.0);
    let b = styled(b, Some(hex("#0099cc")), None, "group circle");
    let gid = doc.ids_mut().shape();
    let mut group = Shape::new(
        gid,
        ShapeKind::Group {
            children: vec![a, b],
        },
    );
    group.name = Some("group".into());
    shapes.push(group);
    let mut poly = BezPath::new();
    poly.move_to((15.0, 58.0));
    poly.line_to((40.0, 63.0));
    poly.line_to((65.0, 53.0));
    poly.line_to((80.0, 60.0));
    let p = path(&mut doc, poly, false);
    shapes.push(styled(p, None, Some((hex("#660066"), 0.75)), "polyline"));
    shapes.push(text(
        &mut doc,
        vec![TextSpan::new("Interop test 0003 éàü", FONT, 9.0)],
        15.0,
        33.0,
        TextAlign::Left,
    ));
    let mut bold = TextSpan::new("bold red", FONT, 8.0);
    bold.bold = true;
    bold.fill = Some(Fill::Solid(hex("#cc0000")));
    let mut italic = TextSpan::new("italic", FONT, 8.0);
    italic.italic = true;
    shapes.push(text(
        &mut doc,
        vec![
            TextSpan::new("Runs: ", FONT, 8.0),
            bold,
            TextSpan::new(" and ", FONT, 8.0),
            italic,
        ],
        15.0,
        16.0,
        TextAlign::Left,
    ));
    shapes.push(text(
        &mut doc,
        vec![TextSpan::new("right aligned", FONT, 7.0)],
        195.0,
        33.0,
        TextAlign::Right,
    ));
    shapes.push(text(
        &mut doc,
        vec![TextSpan::new("centred", FONT, 7.0)],
        150.0,
        16.0,
        TextAlign::Center,
    ));
    // The whole drawing, and three parts of it that tell a record kind
    // apart when a file of one version fails: no objects, the shapes
    // alone and the texts alone.
    let is_text = |s: &Shape| matches!(s.kind, ShapeKind::Text { .. });
    let texts: Vec<Shape> = shapes.iter().filter(|s| is_text(s)).cloned().collect();
    let others: Vec<Shape> = shapes.iter().filter(|s| !is_text(s)).cloned().collect();
    for (name, part) in [
        ("interop.tdraw", shapes),
        ("interop-empty.tdraw", Vec::new()),
        ("interop-shapes.tdraw", others),
        ("interop-text.tdraw", texts),
    ] {
        let mut d = doc.clone();
        d.pages[0].layers[0].shapes = part;
        let path = dir.join(name);
        match tracedraw_io::save_native(&d, &path) {
            Ok(()) => println!("wrote {}", path.display()),
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
    }
}
