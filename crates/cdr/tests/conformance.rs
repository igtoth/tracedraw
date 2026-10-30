//! Round-trip conformance suite for the `.cdr` writer.
//!
//! Every object kind the writer handles (rectangles, ellipses, curves,
//! polygons, artistic and paragraph text, groups, clip frames, bitmaps
//! with and without see-through pixels, uniform, fountain, pattern,
//! texture and mesh fills, outlines, transparency, blends,
//! contours, envelopes and extrusions, pages, layers, hidden objects and
//! page backgrounds) is written in every version the writer produces (7
//! to 27), read back, and checked to survive as the writer promises for
//! that version (`docs/cdr-format.md`, Writing: a few things are written
//! as what they draw, or lost, in some versions; those limits are the
//! `kept_*` helpers below, so a change of them is a change of this file).
//!
//! Every file also goes through the structural checks of
//! [`structure::verify`]: the checker of `tracedraw_cdr::check`, chunk
//! sizes that add up, lists that nest, the chunks each list must hold,
//! bitmap ids that name a picture, and the package members and the data
//! file list of the ZIP containers.
//!
//! `cargo test -p tracedraw-cdr --test conformance` runs it; it is part
//! of `cargo test --workspace`.

use tracedraw_cdr::write::{document_to_cdr_version, versions};
use tracedraw_cdr::{open_bytes, ParseReport};
use tracedraw_core::document::text_outline::Line;
use tracedraw_core::geometry::Shape as _;
use tracedraw_core::live::{Effect, MergeMode};
use tracedraw_core::{
    document::EllipseArc, id::IdSource, Affine, BezPath, Color, Document, Fill, Fountain,
    FountainKind, LineCap, LineJoin, Mesh, ParagraphStyle, Pattern, PatternTile, Point, Rect,
    Shape, ShapeKind, Size, Stop, Stroke, TextAlign, TextSpan, Texture, TextureKind,
};

/// How far a coordinate may move through a round trip (the files round
/// to 1/254000 inch, a tenth of a micrometre; text boxes are laid out
/// again when read).
const EPS: f64 = 0.05;

// ---- what each version keeps ---------------------------------------------

/// Uniform transparency: argument 8000 before 13, the style string from
/// 16; versions 13 to 15 lose it.
fn keeps_opacity(v: u16) -> bool {
    !(13..=15).contains(&v)
}

/// A graded transparency stays a gradient mask only in style strings.
fn keeps_graded_transparency(v: u16) -> bool {
    v >= 16
}

/// Rectangles keep their own record before 15; from 15 they are curves.
fn keeps_native_rects(v: u16) -> bool {
    v < 15
}

/// Hidden objects and the page background need version 21 arguments.
fn keeps_hidden_and_background(v: u16) -> bool {
    v >= 21
}

/// Group names go into the group's own `loda` from version 14, where a
/// file's group carries one; older records are left as they were.
fn keeps_group_names(v: u16) -> bool {
    v >= 14
}

/// A blend goes out live from version 9 (its two ends with a linked group
/// between them, #1192), so it reads back as a blend, on the upper object
/// before 14 and on its start object from 14 (the package member);
/// versions 7 and 8 hold the drawn group.
fn keeps_live_blends(v: u16) -> bool {
    v >= 9
}

/// Pages list only their own layers from version 14; before it every page
/// lists every layer of the drawing in the master page's order, empty
/// where the page has none of that name (#1139).
fn keeps_own_layers_only(v: u16) -> bool {
    v >= 14
}

/// The layer of `page` named `name`.
fn layer<'a>(v: u16, page: &'a tracedraw_core::Page, name: &str) -> &'a tracedraw_core::Layer {
    page.layers
        .iter()
        .find(|l| l.name == name)
        .unwrap_or_else(|| panic!("version {v}: no layer {name}"))
}

/// The page's layers are `names`, in that order, from version 14; before
/// it the page lists them (in that order) among the drawing's other
/// layers, which are empty there.
fn page_layers(v: u16, page: &tracedraw_core::Page, names: &[&str]) {
    let got: Vec<&str> = page.layers.iter().map(|l| l.name.as_str()).collect();
    if keeps_own_layers_only(v) {
        assert_eq!(got, names, "version {v}");
        return;
    }
    let own: Vec<&str> = got.iter().copied().filter(|n| names.contains(n)).collect();
    assert_eq!(own, names, "version {v}: {got:?}");
    for l in page
        .layers
        .iter()
        .filter(|l| !names.contains(&l.name.as_str()))
    {
        assert!(
            l.shapes.is_empty(),
            "version {v}: layer {} of another page holds {} objects",
            l.name,
            l.shapes.len()
        );
    }
}

/// A page of another size than the document's keeps it in its own
/// `lgob` from version 16; before, every page has the document's size.
fn keeps_page_sizes(v: u16) -> bool {
    v >= 16
}

// ---- building drawings ---------------------------------------------------

/// A one-page drawing of `shapes` on one layer, the page `w` by `h` mm.
fn drawing(w: f64, h: f64, build: impl FnOnce(&mut IdSource) -> Vec<Shape>) -> Document {
    let mut doc = Document::new("conformance", Size::new(w, h));
    let mut ids = doc.ids().clone();
    doc.pages[0].layers[0].name = "Ink".into();
    doc.pages[0].layers[0].shapes = build(&mut ids);
    doc.set_ids(ids);
    doc
}

fn rect(ids: &mut IdSource, r: Rect, radius: f64) -> Shape {
    let mut s = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: r,
            radius,
            corners: None,
        },
    );
    s.fill = Fill::Solid(Color::rgb8(220, 40, 40));
    s.stroke = Some(Stroke::new(Color::BLACK, 0.5));
    s
}

fn ellipse(ids: &mut IdSource, r: Rect, arc: Option<EllipseArc>) -> Shape {
    let mut s = Shape::new(ids.shape(), ShapeKind::Ellipse { rect: r, arc });
    s.fill = Fill::Solid(Color::rgb8(40, 180, 60));
    s.stroke = Some(Stroke::new(Color::rgb8(0, 60, 0), 1.0));
    s
}

fn path(ids: &mut IdSource, p: BezPath, closed: bool) -> Shape {
    let mut s = Shape::new(ids.shape(), ShapeKind::Path { path: p, closed });
    s.fill = if closed {
        Fill::Solid(Color::rgb8(200, 60, 160))
    } else {
        Fill::None
    };
    s.stroke = Some(Stroke::new(Color::BLACK, 0.8));
    s
}

fn text(ids: &mut IdSource, spans: Vec<TextSpan>, at: Point, align: TextAlign) -> Shape {
    let mut s = Shape::new(
        ids.shape(),
        ShapeKind::Text {
            spans,
            origin: Point::ZERO,
            frame: None,
            align,
            para: ParagraphStyle::default(),
            on_path: None,
        },
    );
    s.transform = Affine::translate(at.to_vec2());
    s.fill = Fill::Solid(Color::BLACK);
    s.stroke = None;
    s
}

fn group(ids: &mut IdSource, children: Vec<Shape>) -> Shape {
    let mut g = Shape::new(ids.shape(), ShapeKind::Group { children });
    g.fill = Fill::None;
    g.stroke = None;
    g
}

/// A `w` by `h` picture: a red to blue ramp, every other row see-through
/// when `alpha`.
fn picture(w: u32, h: u32, alpha: bool) -> Vec<u8> {
    let mut pm = tiny_skia::Pixmap::new(w, h).expect("pixmap");
    let px = pm.pixels_mut();
    for y in 0..h {
        for x in 0..w {
            let r = (x * 255 / w.max(1)) as u8;
            let a = if alpha && y % 2 == 1 { 96 } else { 255 };
            let c = tiny_skia::ColorU8::from_rgba(r, 30, 255 - r, a).premultiply();
            px[(y * w + x) as usize] = c;
        }
    }
    pm.encode_png().expect("png")
}

fn bitmap(ids: &mut IdSource, r: Rect, w: u32, h: u32, alpha: bool) -> Shape {
    let mut s = Shape::new(
        ids.shape(),
        ShapeKind::Bitmap {
            rect: r,
            width_px: w,
            height_px: h,
            png: picture(w, h, alpha).into(),
            fx: None,
        },
    );
    s.fill = Fill::None;
    s.stroke = None;
    s
}

fn closed_curve() -> BezPath {
    let mut p = BezPath::new();
    p.move_to((10.0, 10.0));
    p.curve_to((20.0, 40.0), (40.0, 40.0), (50.0, 10.0));
    p.line_to((45.0, 5.0));
    p.curve_to((30.0, -5.0), (20.0, -5.0), (10.0, 10.0));
    p.close_path();
    p
}

fn polyline() -> BezPath {
    let mut p = BezPath::new();
    p.move_to((60.0, 10.0));
    p.line_to((70.0, 30.0));
    p.line_to((80.0, 10.0));
    p.line_to((90.0, 30.0));
    p
}

fn donut() -> BezPath {
    let mut p = tracedraw_core::geometry::ellipse_path(Rect::new(100.0, 10.0, 140.0, 50.0));
    p.extend(tracedraw_core::geometry::ellipse_path(Rect::new(
        110.0, 20.0, 130.0, 40.0,
    )));
    p
}

// ---- reading back ----------------------------------------------------------

struct Read {
    v: u16,
    doc: Document,
    report: ParseReport,
}

impl Read {
    /// The shapes of the first page's first layer, bottom to top.
    fn shapes(&self) -> &[Shape] {
        &self.doc.pages[0].layers[0].shapes
    }

    fn shape(&self, i: usize) -> &Shape {
        let shapes = self.shapes();
        assert!(
            i < shapes.len(),
            "version {}: {} shapes, wanted index {i}: {:?}",
            self.v,
            shapes.len(),
            self.report.warnings
        );
        &shapes[i]
    }

    fn count(&self, n: usize) {
        assert_eq!(
            self.shapes().len(),
            n,
            "version {}: {:?}",
            self.v,
            self.shapes()
                .iter()
                .map(|s| kind_name(&s.kind))
                .collect::<Vec<_>>()
        );
    }
}

fn kind_name(k: &ShapeKind) -> &'static str {
    match k {
        ShapeKind::Rect { .. } => "rect",
        ShapeKind::Ellipse { .. } => "ellipse",
        ShapeKind::Polygon { .. } => "polygon",
        ShapeKind::Path { .. } => "path",
        ShapeKind::Text { .. } => "text",
        ShapeKind::Group { .. } => "group",
        ShapeKind::ClipFrame { .. } => "clip frame",
        ShapeKind::Table(_) => "table",
        ShapeKind::SymbolInstance { .. } => "symbol",
        ShapeKind::Bitmap { .. } => "bitmap",
    }
}

/// The package as the suite reads it: the file's own records, as any
/// reader finds them, and of TraceDraw's own members only the blends'
/// (`tracedraw/blends.json`, whose live blends the suite checks, #986).
/// Other members of TraceDraw's own put objects back as TraceDraw keeps
/// them and are checked by their own tests; left in, they would hide
/// what the records hold. RIFF files (before 14) have no members.
fn records_and_blends(bytes: &[u8]) -> Vec<u8> {
    use std::io::{Read as _, Write as _};
    let Ok(mut z) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else {
        return bytes.to_vec();
    };
    let own = |name: &str| name.starts_with("tracedraw/") && name != "tracedraw/blends.json";
    if !z.file_names().any(own) {
        return bytes.to_vec();
    }
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for i in 0..z.len() {
        let mut f = z.by_index(i).expect("member");
        let name = f.name().to_string();
        if own(&name) {
            continue;
        }
        let mut data = Vec::new();
        f.read_to_end(&mut data).expect("read member");
        out.start_file(name, stored).expect("start member");
        out.write_all(&data).expect("write member");
    }
    out.finish().expect("package").into_inner()
}

/// Write `doc` in every version, check each file's structure, read it
/// back (see [`records_and_blends`]) and hand it to `expect`.
fn every_version(doc: &Document, mut expect: impl FnMut(&Read)) {
    for v in versions() {
        let bytes = document_to_cdr_version(doc, v);
        assert!(!bytes.is_empty(), "version {v}: nothing written");
        structure::verify(&bytes, v);
        let (back, report) = open_bytes(&records_and_blends(&bytes), "back")
            .unwrap_or_else(|e| panic!("version {v}: {e}"));
        assert_eq!(report.version.map(|x| x.0), Some(v), "version {v}");
        assert_eq!(
            report.skipped_objects, 0,
            "version {v}: {:?}",
            report.warnings
        );
        expect(&Read {
            v,
            doc: back,
            report,
        });
    }
}

fn near(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

fn same_box(v: u16, got: Rect, want: Rect, eps: f64) {
    assert!(
        near(got.x0, want.x0, eps)
            && near(got.y0, want.y0, eps)
            && near(got.x1, want.x1, eps)
            && near(got.y1, want.y1, eps),
        "version {v}: box {got:?}, wanted {want:?}"
    );
}

fn same_color(v: u16, got: Color, want: Color, what: &str) {
    let (g, w) = (got.to_rgb8(), want.to_rgb8());
    assert!(
        g.iter()
            .zip(&w)
            .all(|(a, b)| (i32::from(*a) - i32::from(*b)).abs() <= 2),
        "version {v}: {what} is {got:?}, wanted {want:?}"
    );
}

fn solid(v: u16, s: &Shape, want: Color, what: &str) {
    match &s.fill {
        Fill::Solid(c) => same_color(v, *c, want, what),
        other => panic!("version {v}: {what} fill is {other:?}, wanted solid {want:?}"),
    }
}

// ---- the cases -----------------------------------------------------------

#[test]
fn rectangles() {
    let turned = Affine::translate((120.0, 20.0)) * Affine::rotate(30f64.to_radians());
    let doc = drawing(200.0, 100.0, |ids| {
        let plain = rect(ids, Rect::new(10.0, 10.0, 50.0, 40.0), 0.0);
        let rounded = rect(ids, Rect::new(60.0, 10.0, 100.0, 40.0), 4.0);
        let mut t = rect(ids, Rect::new(0.0, 0.0, 30.0, 20.0), 0.0);
        t.transform = turned;
        t.name = Some("turned box".into());
        vec![plain, rounded, t]
    });
    let want = [
        Rect::new(10.0, 10.0, 50.0, 40.0),
        Rect::new(60.0, 10.0, 100.0, 40.0),
        (turned * Rect::new(0.0, 0.0, 30.0, 20.0).to_path(0.01)).bounding_box(),
    ];
    every_version(&doc, |r| {
        r.count(3);
        for (i, w) in want.iter().enumerate() {
            let s = r.shape(i);
            same_box(r.v, s.bounds(), *w, EPS);
            solid(r.v, s, Color::rgb8(220, 40, 40), "rectangle");
            let st = s.stroke.as_ref().expect("outline");
            assert!(near(st.width, 0.5, 0.001), "version {}", r.v);
            if keeps_native_rects(r.v) {
                let ShapeKind::Rect { radius, .. } = s.kind else {
                    panic!("version {}: {:?}", r.v, kind_name(&s.kind));
                };
                let want_r = if i == 1 { 4.0 } else { 0.0 };
                assert!(
                    near(radius, want_r, 0.01),
                    "version {}: radius {radius}",
                    r.v
                );
            } else {
                assert!(
                    matches!(s.kind, ShapeKind::Path { closed: true, .. }),
                    "version {}: {}",
                    r.v,
                    kind_name(&s.kind)
                );
            }
        }
        // The rounded corners take their area away in every version.
        let area = r.shape(1).page_path().area().abs();
        let rounded = 40.0 * 30.0 - (4.0 - std::f64::consts::PI) * 16.0;
        assert!(near(area, rounded, 1.0), "version {}: area {area}", r.v);
        assert_eq!(
            r.shape(2).name.as_deref(),
            Some("turned box"),
            "version {}",
            r.v
        );
    });
}

#[test]
fn ellipses_arcs_and_pies() {
    let pie = EllipseArc {
        start_deg: 30.0,
        end_deg: 200.0,
        pie: true,
    };
    let arc = EllipseArc {
        start_deg: 0.0,
        end_deg: 90.0,
        pie: false,
    };
    let doc = drawing(200.0, 100.0, |ids| {
        vec![
            ellipse(ids, Rect::new(10.0, 10.0, 60.0, 40.0), None),
            ellipse(ids, Rect::new(70.0, 10.0, 120.0, 50.0), Some(pie)),
            {
                let mut a = ellipse(ids, Rect::new(130.0, 10.0, 180.0, 60.0), Some(arc));
                a.fill = Fill::None;
                a
            },
        ]
    });
    every_version(&doc, |r| {
        r.count(3);
        let boxes = [
            Rect::new(10.0, 10.0, 60.0, 40.0),
            Rect::new(70.0, 10.0, 120.0, 50.0),
            Rect::new(130.0, 10.0, 180.0, 60.0),
        ];
        let arcs = [None, Some(pie), Some(arc)];
        for (i, (b, a)) in boxes.iter().zip(&arcs).enumerate() {
            let s = r.shape(i);
            let ShapeKind::Ellipse { rect: got, arc: ga } = &s.kind else {
                panic!("version {}: {}", r.v, kind_name(&s.kind));
            };
            same_box(r.v, s.transform.transform_rect_bbox(*got), *b, EPS);
            match (a, ga) {
                (None, None) => {}
                (Some(w), Some(g)) => {
                    assert!(
                        near(g.start_deg, w.start_deg, 0.01) && near(g.end_deg, w.end_deg, 0.01),
                        "version {}: arc {g:?}, wanted {w:?}",
                        r.v
                    );
                    assert_eq!(g.pie, w.pie, "version {}", r.v);
                }
                other => panic!("version {}: arc {other:?}", r.v),
            }
            let st = s.stroke.as_ref().expect("outline");
            same_color(r.v, st.color, Color::rgb8(0, 60, 0), "ellipse outline");
            assert!(near(st.width, 1.0, 0.001), "version {}", r.v);
        }
        solid(r.v, r.shape(0), Color::rgb8(40, 180, 60), "ellipse");
        assert_eq!(r.shape(2).fill, Fill::None, "version {}", r.v);
    });
}

#[test]
fn curves_polylines_and_subpaths() {
    let doc = drawing(200.0, 100.0, |ids| {
        vec![
            path(ids, closed_curve(), true),
            path(ids, polyline(), false),
            path(ids, donut(), true),
        ]
    });
    let want = [closed_curve(), polyline(), donut()];
    every_version(&doc, |r| {
        r.count(3);
        for (i, w) in want.iter().enumerate() {
            let s = r.shape(i);
            let ShapeKind::Path { path: got, closed } = &s.kind else {
                panic!("version {}: {}", r.v, kind_name(&s.kind));
            };
            assert_eq!(*closed, i != 1, "version {}: curve {i}", r.v);
            let page = s.transform * got.clone();
            same_box(r.v, page.bounding_box(), w.bounding_box(), EPS);
            let subpaths = |p: &BezPath| {
                p.elements()
                    .iter()
                    .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
                    .count()
            };
            assert_eq!(subpaths(&page), subpaths(w), "version {}: curve {i}", r.v);
            if i != 1 {
                assert!(
                    near(page.area().abs(), w.area().abs(), 0.5),
                    "version {}: curve {i} area {} wanted {}",
                    r.v,
                    page.area(),
                    w.area()
                );
            }
        }
        solid(r.v, r.shape(0), Color::rgb8(200, 60, 160), "curve");
        assert_eq!(r.shape(1).fill, Fill::None, "version {}", r.v);
        let st = r.shape(1).stroke.as_ref().expect("outline");
        assert!(near(st.width, 0.8, 0.001), "version {}", r.v);
    });
}

#[test]
fn polygons_and_stars_go_out_as_the_curve_they_draw() {
    let doc = drawing(200.0, 100.0, |ids| {
        let mut star = Shape::new(
            ids.shape(),
            ShapeKind::Polygon {
                rect: Rect::new(20.0, 20.0, 80.0, 80.0),
                points: 5,
                sharpness: 0.5,
                complex: None,
                twist: 0.0,
            },
        );
        star.fill = Fill::Solid(Color::rgb8(250, 200, 0));
        let mut hexagon = Shape::new(
            ids.shape(),
            ShapeKind::Polygon {
                rect: Rect::new(100.0, 20.0, 160.0, 80.0),
                points: 6,
                sharpness: 0.0,
                complex: None,
                twist: 0.0,
            },
        );
        hexagon.fill = Fill::Solid(Color::rgb8(0, 120, 250));
        vec![star, hexagon]
    });
    let want: Vec<BezPath> = doc.pages[0].layers[0]
        .shapes
        .iter()
        .map(|s| s.page_path())
        .collect();
    every_version(&doc, |r| {
        r.count(2);
        for (i, w) in want.iter().enumerate() {
            let s = r.shape(i);
            assert!(
                matches!(s.kind, ShapeKind::Path { closed: true, .. }),
                "version {}: {}",
                r.v,
                kind_name(&s.kind)
            );
            let page = s.page_path();
            same_box(r.v, page.bounding_box(), w.bounding_box(), EPS);
            assert!(
                near(page.area().abs(), w.area().abs(), 0.5),
                "version {}: polygon {i}",
                r.v
            );
        }
    });
}

#[test]
fn artistic_text_runs_and_lines() {
    let doc = drawing(200.0, 100.0, |ids| {
        let mut bold = TextSpan::new("Olá, Mundo", "Verdana", 18.0);
        bold.bold = true;
        let mut red = TextSpan::new("Red ", "Georgia", 14.0);
        red.fill = Some(Fill::Solid(Color::rgb8(255, 0, 0)));
        red.italic = true;
        let mut lined = TextSpan::new("under\nnext line", "Arial", 20.0);
        lined.underline = true;
        vec![
            text(ids, vec![bold], Point::new(20.0, 80.0), TextAlign::Left),
            text(
                ids,
                vec![red, lined],
                Point::new(20.0, 40.0),
                TextAlign::Center,
            ),
            text(
                ids,
                vec![TextSpan::new("right", "Arial", 12.0)],
                Point::new(180.0, 20.0),
                TextAlign::Right,
            ),
        ]
    });
    every_version(&doc, |r| {
        r.count(3);
        let first = r.shape(0);
        let ShapeKind::Text { spans, align, .. } = &first.kind else {
            panic!("version {}: {}", r.v, kind_name(&first.kind));
        };
        assert_eq!(*align, TextAlign::Left, "version {}", r.v);
        assert_eq!(spans.len(), 1, "version {}: {spans:?}", r.v);
        assert_eq!(spans[0].text, "Olá, Mundo", "version {}", r.v);
        assert_eq!(spans[0].font_family, "Verdana", "version {}", r.v);
        assert!(near(spans[0].size_pt, 18.0, 0.01), "version {}", r.v);
        assert!(spans[0].bold && !spans[0].italic, "version {}", r.v);
        let at = first.transform * Point::ZERO;
        assert!(
            near(at.x, 20.0, EPS) && near(at.y, 80.0, EPS),
            "version {}: text at {at:?}",
            r.v
        );
        solid(r.v, first, Color::BLACK, "text");
        let second = r.shape(1);
        let ShapeKind::Text { spans, align, .. } = &second.kind else {
            panic!("version {}: {}", r.v, kind_name(&second.kind));
        };
        assert_eq!(*align, TextAlign::Center, "version {}", r.v);
        let joined: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(joined, "Red under\nnext line", "version {}: {spans:?}", r.v);
        assert_eq!(spans[0].font_family, "Georgia", "version {}", r.v);
        assert!(spans[0].italic && !spans[0].underline, "version {}", r.v);
        assert_eq!(
            spans[0].fill,
            Some(Fill::Solid(Color::rgb8(255, 0, 0))),
            "version {}",
            r.v
        );
        let last = spans.last().expect("span");
        assert_eq!(last.font_family, "Arial", "version {}", r.v);
        assert!(near(last.size_pt, 20.0, 0.01), "version {}", r.v);
        assert!(last.underline && !last.italic, "version {}", r.v);
        let ShapeKind::Text { align, .. } = &r.shape(2).kind else {
            panic!("version {}", r.v);
        };
        assert_eq!(*align, TextAlign::Right, "version {}", r.v);
    });
}

/// A line breaker for the paragraph case: no text engine is loaded
/// here, so lines are broken at the widths a plain sans serif has (half
/// an em a letter), one line height apart down from the frame's top.
fn plain_lines(req: &tracedraw_core::document::text_outline::TextRequest) -> Vec<Line> {
    let text: String = req.spans.iter().map(|s| s.text.as_str()).collect();
    let size_mm = req.spans.first().map_or(12.0, |s| s.size_pt) * 25.4 / 72.0;
    let advance = size_mm * 0.5;
    let width = req.frame.map_or(f64::INFINITY, |f| f.width);
    let chars: Vec<char> = text.chars().collect();
    let mut lines = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let fit = ((width / advance).floor() as usize).max(1);
        let mut end = (start + fit).min(chars.len());
        if end < chars.len() {
            if let Some(space) = chars[start..end].iter().rposition(|c| *c == ' ') {
                end = start + space + 1;
            }
        }
        let n = end - start;
        let edges: Vec<f64> = (0..=n).map(|i| i as f64 * advance).collect();
        lines.push(Line {
            start_char: start,
            end_char: end,
            x: 0.0,
            baseline: -(lines.len() as f64 + 1.0) * size_mm * 1.2,
            ascent: size_mm * 0.8,
            descent: size_mm * 0.2,
            starts: edges[..n].to_vec(),
            ends: edges[1..].to_vec(),
            edges,
        });
        start = end;
    }
    lines
}

#[test]
fn paragraph_text_keeps_its_words_and_place() {
    tracedraw_core::document::text_outline::set_lines(plain_lines);
    let words = "The quick brown fox jumps over the lazy dog and keeps on running";
    let doc = drawing(200.0, 100.0, |ids| {
        let mut t = Shape::new(
            ids.shape(),
            ShapeKind::Text {
                spans: vec![TextSpan::new(words, "Arial", 12.0)],
                origin: Point::new(0.0, 0.0),
                frame: Some(Size::new(60.0, 60.0)),
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            },
        );
        t.transform = Affine::translate((20.0, 90.0));
        t.fill = Fill::Solid(Color::BLACK);
        t.stroke = None;
        vec![t]
    });
    let frame_box = doc.pages[0].layers[0].shapes[0].bounds();
    every_version(&doc, |r| {
        r.count(1);
        // One text per line, grouped when there are several.
        let mut texts: Vec<&Shape> = Vec::new();
        fn collect<'a>(s: &'a Shape, out: &mut Vec<&'a Shape>) {
            match &s.kind {
                ShapeKind::Group { children } => children.iter().for_each(|c| collect(c, out)),
                ShapeKind::Text { .. } => out.push(s),
                _ => {}
            }
        }
        collect(r.shape(0), &mut texts);
        assert!(texts.len() > 1, "version {}: {} lines", r.v, texts.len());
        let joined: String = texts
            .iter()
            .map(|t| match &t.kind {
                ShapeKind::Text { spans, .. } => {
                    spans.iter().map(|s| s.text.as_str()).collect::<String>()
                }
                _ => String::new(),
            })
            .collect::<Vec<_>>()
            .join(" ");
        for word in words.split(' ') {
            assert!(joined.contains(word), "version {}: {joined:?}", r.v);
        }
        let b = r.shape(0).bounds();
        assert!(
            b.x0 >= frame_box.x0 - 1.0 && b.x1 <= frame_box.x1 + 1.0,
            "version {}: lines {b:?} outside the frame {frame_box:?}",
            r.v
        );
        assert!(
            b.y1 <= frame_box.y1 + 1.0 && b.y0 >= frame_box.y0 - 1.0,
            "version {}: lines {b:?} outside the frame {frame_box:?}",
            r.v
        );
    });
}

#[test]
fn groups_nest() {
    let doc = drawing(200.0, 100.0, |ids| {
        let a = rect(ids, Rect::new(10.0, 10.0, 30.0, 30.0), 0.0);
        let b = ellipse(ids, Rect::new(40.0, 10.0, 60.0, 30.0), None);
        let c = rect(ids, Rect::new(70.0, 10.0, 90.0, 30.0), 0.0);
        let inner = group(ids, vec![b, c]);
        let mut outer = group(ids, vec![a, inner]);
        outer.name = Some("outer".into());
        outer.transform = Affine::translate((5.0, 5.0));
        vec![outer]
    });
    every_version(&doc, |r| {
        r.count(1);
        let outer = r.shape(0);
        let ShapeKind::Group { children } = &outer.kind else {
            panic!("version {}: {}", r.v, kind_name(&outer.kind));
        };
        assert_eq!(
            children.len(),
            2,
            "version {}: {:?}",
            r.v,
            children
                .iter()
                .map(|c| kind_name(&c.kind))
                .collect::<Vec<_>>()
        );
        same_box(r.v, outer.bounds(), Rect::new(15.0, 15.0, 95.0, 35.0), EPS);
        let ShapeKind::Group { children: inner } = &children[1].kind else {
            panic!("version {}: {}", r.v, kind_name(&children[1].kind));
        };
        assert_eq!(
            inner.len(),
            2,
            "version {}: {:?}",
            r.v,
            inner.iter().map(|c| kind_name(&c.kind)).collect::<Vec<_>>()
        );
        assert!(
            matches!(inner[0].kind, ShapeKind::Ellipse { .. }),
            "version {}",
            r.v
        );
        let inner_box =
            (outer.transform * children[1].transform * inner[0].page_path()).bounding_box();
        same_box(r.v, inner_box, Rect::new(45.0, 15.0, 65.0, 35.0), EPS);
        let want = keeps_group_names(r.v).then_some("outer");
        assert_eq!(outer.name.as_deref(), want, "version {}", r.v);
    });
}

#[test]
fn clip_frames() {
    let doc = drawing(200.0, 100.0, |ids| {
        // A frame around a picture: a bitmap with a crop path.
        let bm = bitmap(ids, Rect::new(0.0, 0.0, 60.0, 30.0), 6, 3, false);
        let mut frame = rect(ids, Rect::new(10.0, 5.0, 40.0, 25.0), 0.0);
        frame.fill = Fill::None;
        frame.stroke = None;
        let mut clipped = Shape::new(
            ids.shape(),
            ShapeKind::ClipFrame {
                frame: Box::new(frame),
                contents: vec![bm],
            },
        );
        clipped.fill = Fill::None;
        clipped.stroke = None;
        clipped.transform = Affine::translate((10.0, 10.0));
        // A frame with a fill and an outline around two shapes.
        let a = rect(ids, Rect::new(100.0, 10.0, 130.0, 40.0), 0.0);
        let b = ellipse(ids, Rect::new(120.0, 20.0, 160.0, 60.0), None);
        let frame = rect(ids, Rect::new(110.0, 15.0, 150.0, 55.0), 0.0);
        let mut shapes = Shape::new(
            ids.shape(),
            ShapeKind::ClipFrame {
                frame: Box::new(frame),
                contents: vec![a, b],
            },
        );
        shapes.fill = Fill::None;
        shapes.stroke = None;
        vec![clipped, shapes]
    });
    every_version(&doc, |r| {
        r.count(2);
        let first = r.shape(0);
        let ShapeKind::ClipFrame { frame, contents } = &first.kind else {
            panic!("version {}: {}", r.v, kind_name(&first.kind));
        };
        let fb = (first.transform * frame.page_path()).bounding_box();
        same_box(r.v, fb, Rect::new(20.0, 15.0, 50.0, 35.0), EPS);
        assert_eq!(contents.len(), 1, "version {}", r.v);
        assert!(
            matches!(contents[0].kind, ShapeKind::Bitmap { .. }),
            "version {}: {}",
            r.v,
            kind_name(&contents[0].kind)
        );
        let cb = (first.transform * contents[0].page_path()).bounding_box();
        same_box(r.v, cb, Rect::new(10.0, 10.0, 70.0, 40.0), EPS);
        // From version 12 a clip container (#1222): the frame with its
        // fill and outline, the contents inside it. Before, the frame's
        // fill under, the contents, the frame's outline over.
        let second = r.shape(1);
        if r.v >= 12 {
            let ShapeKind::ClipFrame { frame, contents } = &second.kind else {
                panic!("version {}: {}", r.v, kind_name(&second.kind));
            };
            solid(r.v, frame, Color::rgb8(220, 40, 40), "frame fill");
            assert!(frame.stroke.is_some(), "version {}", r.v);
            assert_eq!(contents.len(), 2, "version {}", r.v);
            let fb = (second.transform * frame.page_path()).bounding_box();
            same_box(r.v, fb, Rect::new(110.0, 15.0, 150.0, 55.0), EPS);
            let cb = contents
                .iter()
                .map(|c| (second.transform * c.page_path()).bounding_box())
                .reduce(|a, b| a.union(b))
                .unwrap_or_default();
            same_box(r.v, cb, Rect::new(100.0, 10.0, 160.0, 60.0), EPS);
            return;
        }
        let ShapeKind::Group { children } = &second.kind else {
            panic!("version {}: {}", r.v, kind_name(&second.kind));
        };
        assert_eq!(children.len(), 4, "version {}", r.v);
        solid(r.v, &children[0], Color::rgb8(220, 40, 40), "frame fill");
        assert!(children[0].stroke.is_none(), "version {}", r.v);
        assert_eq!(children[3].fill, Fill::None, "version {}", r.v);
        assert!(children[3].stroke.is_some(), "version {}", r.v);
        same_box(
            r.v,
            second.bounds(),
            Rect::new(100.0, 10.0, 160.0, 60.0),
            EPS,
        );
    });
}

#[test]
fn bitmaps_with_and_without_see_through_pixels() {
    let doc = drawing(200.0, 100.0, |ids| {
        let opaque = bitmap(ids, Rect::new(10.0, 10.0, 70.0, 50.0), 8, 6, false);
        let mut clear = bitmap(ids, Rect::new(0.0, 0.0, 40.0, 40.0), 8, 6, true);
        clear.transform = Affine::translate((100.0, 20.0)) * Affine::scale(1.5);
        vec![opaque, clear]
    });
    every_version(&doc, |r| {
        r.count(2);
        let boxes = [
            Rect::new(10.0, 10.0, 70.0, 50.0),
            Rect::new(100.0, 20.0, 160.0, 80.0),
        ];
        for (i, b) in boxes.iter().enumerate() {
            let s = r.shape(i);
            let ShapeKind::Bitmap {
                width_px,
                height_px,
                png,
                ..
            } = &s.kind
            else {
                panic!("version {}: {}", r.v, kind_name(&s.kind));
            };
            assert_eq!((*width_px, *height_px), (8, 6), "version {}", r.v);
            same_box(r.v, s.bounds(), *b, EPS);
            let pm = tiny_skia::Pixmap::decode_png(png).expect("png");
            assert_eq!((pm.width(), pm.height()), (8, 6), "version {}", r.v);
            let see_through = pm.pixels().iter().any(|p| p.alpha() < 255);
            assert_eq!(see_through, i == 1, "version {}: bitmap {i}", r.v);
            let p = pm.pixels()[0].demultiply();
            assert!(p.red() < 40 && p.blue() > 200, "version {}: {p:?}", r.v);
        }
    });
}

#[test]
fn uniform_and_fountain_fills() {
    let linear = {
        let mut f = Fountain::two(
            FountainKind::Linear,
            Color::rgb8(255, 200, 0),
            Color::rgb8(200, 0, 0),
            45.0,
        );
        f.stops.insert(1, Stop::new(0.3, Color::rgb8(0, 0, 255)));
        f
    };
    let radial = Fountain::two(FountainKind::Radial, Color::BLACK, Color::WHITE, 0.0);
    let conical = Fountain::two(
        FountainKind::Conical,
        Color::rgb8(0, 255, 0),
        Color::rgb8(0, 0, 255),
        90.0,
    );
    let square = Fountain::two(
        FountainKind::Square,
        Color::rgb8(255, 255, 255),
        Color::rgb8(40, 40, 40),
        0.0,
    );
    let fills = [
        Fill::Solid(Color::rgb8(10, 200, 100)),
        Fill::Solid(Color::cmyk_pct(0.0, 80.0, 100.0, 10.0)),
        Fill::Solid(Color::Gray { v: 0.5 }),
        Fill::Fountain(linear.clone()),
        Fill::Fountain(radial.clone()),
        Fill::Fountain(conical.clone()),
        Fill::Fountain(square.clone()),
        Fill::None,
    ];
    let doc = drawing(300.0, 100.0, |ids| {
        fills
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let x = 10.0 + 35.0 * i as f64;
                let mut s = rect(ids, Rect::new(x, 10.0, x + 30.0, 60.0), 0.0);
                s.fill = f.clone();
                s
            })
            .collect()
    });
    every_version(&doc, |r| {
        r.count(fills.len());
        for (i, want) in fills.iter().enumerate() {
            let got = &r.shape(i).fill;
            match (want, got) {
                (Fill::Solid(w), Fill::Solid(g)) => same_color(r.v, *g, *w, "fill"),
                (Fill::Fountain(w), Fill::Fountain(g)) => {
                    assert_eq!(g.kind, w.kind, "version {}: fill {i}", r.v);
                    assert_eq!(g.stops.len(), w.stops.len(), "version {}: fill {i}", r.v);
                    for (gs, ws) in g.stops.iter().zip(&w.stops) {
                        assert!(
                            near(gs.pos, ws.pos, 0.011),
                            "version {}: fill {i} stop at {} wanted {}",
                            r.v,
                            gs.pos,
                            ws.pos
                        );
                        same_color(r.v, gs.color, ws.color, "fountain stop");
                    }
                    assert!(
                        near(g.angle, w.angle, 0.01),
                        "version {}: fill {i} angle {} wanted {}",
                        r.v,
                        g.angle,
                        w.angle
                    );
                }
                (Fill::None, Fill::None) => {}
                (w, g) => panic!("version {}: fill {i} is {g:?}, wanted {w:?}", r.v),
            }
        }
    });
}

#[test]
fn pattern_texture_and_mesh_fills_go_out_as_one_colour() {
    let two_colour = Fill::Pattern(Pattern::TwoColor {
        tile: PatternTile::Checker,
        front: Color::rgb8(255, 0, 0),
        back: Color::rgb8(0, 0, 255),
        size_mm: 5.0,
    });
    let tile = picture(4, 4, false);
    let bitmap_pattern = Fill::Pattern(Pattern::Bitmap {
        png: tile.clone().into(),
        width_px: 4,
        height_px: 4,
        size_mm: 8.0,
    });
    let texture = Fill::Texture(Texture {
        kind: TextureKind::Clouds,
        color_a: Color::rgb8(200, 200, 255),
        color_b: Color::rgb8(0, 0, 100),
        scale: 10.0,
        seed: 7,
    });
    let mesh = Fill::Mesh(Mesh::new(
        Rect::new(0.0, 0.0, 30.0, 50.0),
        2,
        2,
        Color::rgb8(255, 128, 0),
    ));
    let fills = [two_colour, bitmap_pattern, texture, mesh];
    let doc = drawing(200.0, 100.0, |ids| {
        fills
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let x = 10.0 + 40.0 * i as f64;
                let mut s = rect(ids, Rect::new(x, 10.0, x + 30.0, 60.0), 0.0);
                s.fill = f.clone();
                s
            })
            .collect()
    });
    // The stand-in colours the writer promises: the middle of the two
    // colours, the tile's mean colour, the middle of the texture's colours
    // and the mesh's first node.
    let tile_mean = {
        let pm = tiny_skia::Pixmap::decode_png(&tile).expect("png");
        let n = pm.pixels().len() as f64;
        let sum = pm.pixels().iter().fold([0.0f64; 3], |mut a, p| {
            let p = p.demultiply();
            a[0] += f64::from(p.red());
            a[1] += f64::from(p.green());
            a[2] += f64::from(p.blue());
            a
        });
        Color::rgb8(
            (sum[0] / n).round() as u8,
            (sum[1] / n).round() as u8,
            (sum[2] / n).round() as u8,
        )
    };
    let want = [
        tracedraw_core::style::lerp_color(Color::rgb8(255, 0, 0), Color::rgb8(0, 0, 255), 0.5),
        tile_mean,
        tracedraw_core::style::lerp_color(Color::rgb8(200, 200, 255), Color::rgb8(0, 0, 100), 0.5),
        Color::rgb8(255, 128, 0),
    ];
    every_version(&doc, |r| {
        r.count(fills.len());
        for (i, w) in want.iter().enumerate() {
            solid(r.v, r.shape(i), *w, "stand-in fill");
        }
    });
}

#[test]
fn outlines() {
    let mut dashed = Stroke::new(Color::rgb8(0, 0, 200), 2.0);
    dashed.cap = LineCap::Round;
    dashed.join = LineJoin::Round;
    dashed.dash = vec![4.0, 2.0];
    let mut scaled = Stroke::new(Color::rgb8(0, 120, 0), 1.2);
    scaled.scale_with_object = true;
    scaled.behind_fill = true;
    scaled.cap = LineCap::Square;
    scaled.join = LineJoin::Bevel;
    let mut calligraphic = Stroke::new(Color::BLACK, 3.0);
    calligraphic.stretch = 0.5;
    calligraphic.nib_angle = 45.0;
    let strokes: Vec<Option<Stroke>> = vec![
        Some(dashed),
        Some(scaled),
        Some(calligraphic),
        Some(Stroke::hairline(Color::rgb8(90, 90, 90))),
        None,
    ];
    let doc = drawing(250.0, 100.0, |ids| {
        strokes
            .iter()
            .enumerate()
            .map(|(i, st)| {
                let x = 10.0 + 45.0 * i as f64;
                let mut s = rect(ids, Rect::new(x, 10.0, x + 30.0, 60.0), 0.0);
                s.stroke = st.clone();
                s
            })
            .collect()
    });
    every_version(&doc, |r| {
        r.count(strokes.len());
        for (i, want) in strokes.iter().enumerate() {
            let got = &r.shape(i).stroke;
            match (want, got) {
                (None, None) => {}
                (Some(w), Some(g)) => {
                    same_color(r.v, g.color, w.color, "outline");
                    assert!(
                        near(g.width, w.width, 0.001),
                        "version {}: outline {i} width {} wanted {}",
                        r.v,
                        g.width,
                        w.width
                    );
                    assert_eq!(g.cap, w.cap, "version {}: outline {i}", r.v);
                    assert_eq!(g.join, w.join, "version {}: outline {i}", r.v);
                    assert_eq!(g.dash, w.dash, "version {}: outline {i}", r.v);
                    assert_eq!(
                        g.scale_with_object, w.scale_with_object,
                        "version {}: outline {i}",
                        r.v
                    );
                    assert_eq!(g.behind_fill, w.behind_fill, "version {}: outline {i}", r.v);
                    assert!(
                        near(g.stretch, w.stretch, 0.011),
                        "version {}: outline {i} stretch {}",
                        r.v,
                        g.stretch
                    );
                    assert!(
                        near(g.nib_angle, w.nib_angle, 0.01),
                        "version {}: outline {i} nib {}",
                        r.v,
                        g.nib_angle
                    );
                }
                (w, g) => panic!("version {}: outline {i} is {g:?}, wanted {w:?}", r.v),
            }
        }
    });
}

#[test]
fn transparency_uniform_and_graded() {
    let mask = Fountain::two(FountainKind::Linear, Color::BLACK, Color::WHITE, 0.0);
    let doc = drawing(200.0, 100.0, |ids| {
        let mut see_through = rect(ids, Rect::new(10.0, 10.0, 50.0, 50.0), 0.0);
        see_through.opacity = 0.4;
        let mut graded = rect(ids, Rect::new(60.0, 10.0, 100.0, 50.0), 0.0);
        graded.effects.push(Effect::Transparency {
            mask: Fill::Fountain(mask.clone()),
            merge: MergeMode::Normal,
            target: 0,
        });
        vec![see_through, graded]
    });
    every_version(&doc, |r| {
        r.count(2);
        let want = if keeps_opacity(r.v) { 0.4 } else { 1.0 };
        assert!(
            near(r.shape(0).opacity, want, 0.002),
            "version {}: opacity {}",
            r.v,
            r.shape(0).opacity
        );
        let graded = r.shape(1);
        if keeps_graded_transparency(r.v) {
            let [Effect::Transparency { mask: m, .. }] = graded.effects.as_slice() else {
                panic!("version {}: {:?}", r.v, graded.effects);
            };
            let Fill::Fountain(f) = m else {
                panic!("version {}: mask {m:?}", r.v);
            };
            assert_eq!(f.kind, FountainKind::Linear, "version {}", r.v);
            assert_eq!(f.stops.len(), 2, "version {}", r.v);
        } else {
            assert!(graded.effects.is_empty(), "version {}", r.v);
            // The mean opacity of the mask when the version cannot keep
            // it, or nothing at all in 13 to 15.
            let want = if keeps_opacity(r.v) { 0.5 } else { 1.0 };
            assert!(
                near(graded.opacity, want, 0.01),
                "version {}: opacity {}",
                r.v,
                graded.opacity
            );
        }
    });
}

#[test]
fn blends_contours_envelopes_and_extrusions() {
    let doc = drawing(300.0, 150.0, |ids| {
        // A blend of a square into a circle, 3 steps, the end drawn by
        // the effect.
        let mut start = rect(ids, Rect::new(10.0, 10.0, 30.0, 30.0), 0.0);
        let mut end = ellipse(ids, Rect::new(80.0, 10.0, 100.0, 30.0), None);
        end.fill = Fill::Solid(Color::rgb8(0, 0, 255));
        start.effects.push(Effect::Blend {
            end: Box::new(end),
            steps: 3,
            accel_objects: 0.0,
            accel_colors: 0.0,
            rotation: 0.0,
            path: None,
            rotate_on_path: false,
            end_id: None,
            options: Default::default(),
        });
        // A contour of two steps outside.
        let mut contoured = rect(ids, Rect::new(120.0, 20.0, 150.0, 50.0), 0.0);
        contoured.effects.push(Effect::Contour {
            steps: 2,
            offset: 3.0,
            outside: true,
            to_center: false,
            fill_to: Color::WHITE,
            outline_to: None,
            color_blend: 0,
            corners: None,
            accel_objects: 0.0,
            accel_colors: 0.0,
        });
        // An envelope pulling the top edge up in the middle.
        let mut enveloped = rect(ids, Rect::new(170.0, 10.0, 210.0, 40.0), 0.0);
        let mut frame =
            tracedraw_core::envelope::Frame::around(Rect::new(170.0, 10.0, 210.0, 40.0));
        for n in frame.nodes.iter_mut() {
            if n.y > 39.0 && (n.x - 190.0).abs() < 1.0 {
                n.y += 20.0;
            }
        }
        enveloped.effects.push(Effect::from_envelope(
            &frame,
            false,
            tracedraw_core::envelope::Mapping::Putty,
        ));
        // An extrusion of 6 mm to the upper right.
        let mut extruded = rect(ids, Rect::new(230.0, 10.0, 270.0, 40.0), 0.0);
        extruded.effects.push(Effect::Extrude {
            depth: tracedraw_core::Vec2::new(6.0, 6.0),
            vanishing: None,
            amount: 1.0,
            shade_from: None,
            shade_to: None,
            light_angle: 0.0,
            light_intensity: 0.0,
            bevel: 0.0,
            kind: None,
            on_object: false,
            solid: Some(Color::rgb8(120, 20, 20)),
            space: Default::default(),
        });
        vec![start, contoured, enveloped, extruded]
    });
    every_version(&doc, |r| {
        let shapes = r.shapes();
        // The blend: one group of the start, the steps and the end (five
        // objects) in versions 7 and 8; from 9 a live blend between the
        // start and the end, which is an object of its own in the file
        // (one object more). A RIFF file (9 to 13) puts the blend on the
        // upper object, the end; from 14 the package member gives it
        // back to the start (#1192).
        let mut shapes: Vec<Shape> = shapes.to_vec();
        if keeps_live_blends(r.v) {
            let (holder, other) = if r.v >= 14 { (0, 1) } else { (1, 0) };
            let [Effect::Blend {
                steps, end, end_id, ..
            }] = shapes[holder].effects.as_slice()
            else {
                let kinds: Vec<String> = shapes
                    .iter()
                    .map(|s| format!("{} ({} effects)", kind_name(&s.kind), s.effects.len()))
                    .collect();
                panic!("version {}: {kinds:?}", r.v);
            };
            assert_eq!(*steps, 3, "version {}", r.v);
            assert_eq!(*end_id, Some(shapes[other].id), "version {}", r.v);
            assert!(shapes[other].effects.is_empty(), "version {}", r.v);
            let (rect, ellipse) = (&shapes[0], &shapes[1]);
            assert!(
                matches!(ellipse.kind, ShapeKind::Ellipse { .. }),
                "version {}: {}",
                r.v,
                kind_name(&ellipse.kind)
            );
            same_box(r.v, rect.bounds(), Rect::new(10.0, 10.0, 30.0, 30.0), EPS);
            same_box(
                r.v,
                ellipse.bounds(),
                Rect::new(80.0, 10.0, 100.0, 30.0),
                EPS,
            );
            same_box(r.v, end.bounds(), shapes[other].bounds(), EPS);
            shapes.remove(1);
        } else {
            let blend = &shapes[0];
            let ShapeKind::Group { children } = &blend.kind else {
                panic!("version {}: {}", r.v, kind_name(&blend.kind));
            };
            assert_eq!(children.len(), 5, "version {}", r.v);
            same_box(r.v, blend.bounds(), Rect::new(10.0, 10.0, 100.0, 30.0), EPS);
        }
        let shapes = &shapes[..];
        // The contour: its two steps as objects of their own beneath the
        // rectangle, the outer one 6 mm out.
        let kinds: Vec<String> = shapes
            .iter()
            .map(|s| format!("{} {:?}", kind_name(&s.kind), s.bounds()))
            .collect();
        let n = shapes.len();
        // The blend, the contour's two steps and its object, the bent
        // rectangle, then the extrusion's body (its faces) and its front.
        assert!(
            n >= 1 + 3 + 1 + 2,
            "version {}: {n} objects {kinds:#?}",
            r.v
        );
        let outer = &shapes[1];
        same_box(
            r.v,
            outer.bounds(),
            Rect::new(114.0, 14.0, 156.0, 56.0),
            0.1,
        );
        same_box(
            r.v,
            shapes[3].bounds(),
            Rect::new(120.0, 20.0, 150.0, 50.0),
            EPS,
        );
        solid(
            r.v,
            &shapes[3],
            Color::rgb8(220, 40, 40),
            "contoured object",
        );
        // The envelope: the rectangle bent into a curve reaching up.
        let bent = &shapes[4];
        assert!(
            matches!(bent.kind, ShapeKind::Path { closed: true, .. }),
            "version {}: {}",
            r.v,
            kind_name(&bent.kind)
        );
        let b = bent.bounds();
        assert!(
            near(b.x0, 170.0, EPS) && near(b.x1, 210.0, EPS) && near(b.y0, 10.0, EPS),
            "version {}: {b:?}",
            r.v
        );
        assert!(b.y1 > 50.0 && b.y1 <= 60.0 + EPS, "version {}: {b:?}", r.v);
        // The extrusion: its body beneath the face, reaching 6 mm out.
        let body = &shapes[5..n - 1];
        assert!(body.len() >= 2, "version {}: {kinds:#?}", r.v);
        for s in body {
            // The faces are shaded from the body's colour, their edges
            // drawn as outlines.
            assert!(
                matches!(s.fill, Fill::Solid(_)) || s.stroke.is_some(),
                "version {}: body fill {:?}",
                r.v,
                s.fill
            );
        }
        let face = &shapes[n - 1];
        same_box(r.v, face.bounds(), Rect::new(230.0, 10.0, 270.0, 40.0), EPS);
        let all = body
            .iter()
            .map(|s| s.bounds())
            .fold(face.bounds(), |a, b| a.union(b));
        same_box(r.v, all, Rect::new(230.0, 10.0, 276.0, 46.0), 0.1);
    });
}

#[test]
fn pages_layers_hidden_objects_and_backgrounds() {
    let mut doc = Document::new("conformance", Size::new(200.0, 100.0));
    let mut ids = doc.ids().clone();
    doc.pages[0].layers[0].name = "Back".into();
    doc.pages[0].layers[0].shapes = vec![rect(&mut ids, Rect::new(10.0, 10.0, 50.0, 50.0), 0.0)];
    let mut front = tracedraw_core::Layer::new(ids.layer(), "Front");
    let mut hidden = rect(&mut ids, Rect::new(60.0, 10.0, 100.0, 50.0), 0.0);
    hidden.visible = false;
    hidden.name = Some("hidden".into());
    front.shapes = vec![
        ellipse(&mut ids, Rect::new(110.0, 10.0, 150.0, 50.0), None),
        hidden,
    ];
    doc.pages[0].layers.push(front);
    doc.pages[0].background = Some(Fill::Solid(Color::rgb8(255, 250, 200)));
    let mut second = tracedraw_core::Page {
        id: ids.page(),
        name: "Second".into(),
        size: Size::new(100.0, 150.0),
        layers: vec![tracedraw_core::Layer::new(ids.layer(), "Only")],
        guides: Vec::new(),
        background: None,
    };
    second.layers[0].shapes = vec![{
        let mut t = text(
            &mut ids,
            vec![TextSpan::new("second page", "Arial", 14.0)],
            Point::new(10.0, 140.0),
            TextAlign::Left,
        );
        t.name = Some("label".into());
        t
    }];
    doc.pages.push(second);
    doc.set_ids(ids);
    every_version(&doc, |r| {
        assert_eq!(r.doc.pages.len(), 2, "version {}", r.v);
        let first = &r.doc.pages[0];
        assert!(
            near(first.size.width, 200.0, 0.01) && near(first.size.height, 100.0, 0.01),
            "version {}: {:?}",
            r.v,
            first.size
        );
        page_layers(r.v, first, &["Back", "Front"]);
        assert_eq!(layer(r.v, first, "Back").shapes.len(), 1, "version {}", r.v);
        let front = &layer(r.v, first, "Front").shapes;
        if keeps_hidden_and_background(r.v) {
            assert_eq!(front.len(), 2, "version {}", r.v);
            assert!(!front[1].visible, "version {}", r.v);
            assert_eq!(front[1].name.as_deref(), Some("hidden"), "version {}", r.v);
            match &first.background {
                Some(Fill::Solid(c)) => same_color(r.v, *c, Color::rgb8(255, 250, 200), "page"),
                other => panic!("version {}: background {other:?}", r.v),
            }
        } else {
            assert_eq!(front.len(), 1, "version {}", r.v);
            assert!(first.background.is_none(), "version {}", r.v);
        }
        assert!(
            matches!(front[0].kind, ShapeKind::Ellipse { .. }),
            "version {}",
            r.v
        );
        let second = &r.doc.pages[1];
        let (w, h) = if keeps_page_sizes(r.v) {
            (100.0, 150.0)
        } else {
            (200.0, 100.0)
        };
        assert!(
            near(second.size.width, w, 0.01) && near(second.size.height, h, 0.01),
            "version {}: {:?}",
            r.v,
            second.size
        );
        page_layers(r.v, second, &["Only"]);
        let label = &layer(r.v, second, "Only").shapes;
        assert_eq!(label.len(), 1, "version {}", r.v);
        assert_eq!(label[0].name.as_deref(), Some("label"), "version {}", r.v);
        let ShapeKind::Text { spans, .. } = &label[0].kind else {
            panic!("version {}: {}", r.v, kind_name(&label[0].kind));
        };
        assert_eq!(spans[0].text, "second page", "version {}", r.v);
        let at = label[0].transform * Point::ZERO;
        assert!(
            near(at.x, 10.0, EPS) && near(at.y, 140.0, EPS),
            "version {}: label at {at:?}",
            r.v
        );
    });
}

/// A file written from a drawing read back from a file is the same
/// drawing again: nothing drifts through a second round.
#[test]
fn a_second_round_trip_changes_nothing() {
    let doc = drawing(200.0, 100.0, |ids| {
        vec![
            rect(ids, Rect::new(10.0, 10.0, 50.0, 40.0), 3.0),
            ellipse(ids, Rect::new(60.0, 10.0, 100.0, 40.0), None),
            path(ids, closed_curve(), true),
            text(
                ids,
                vec![TextSpan::new("twice", "Arial", 12.0)],
                Point::new(20.0, 80.0),
                TextAlign::Left,
            ),
        ]
    });
    for v in versions() {
        let once = document_to_cdr_version(&doc, v);
        let (back, _) = open_bytes(&once, "once").unwrap_or_else(|e| panic!("version {v}: {e}"));
        let twice = document_to_cdr_version(&back, v);
        structure::verify(&twice, v);
        let (again, rep) =
            open_bytes(&twice, "twice").unwrap_or_else(|e| panic!("version {v}: {e}"));
        assert_eq!(rep.skipped_objects, 0, "version {v}: {:?}", rep.warnings);
        let a = &back.pages[0].layers[0].shapes;
        let b = &again.pages[0].layers[0].shapes;
        assert_eq!(a.len(), b.len(), "version {v}");
        for (x, y) in a.iter().zip(b) {
            assert_eq!(kind_name(&x.kind), kind_name(&y.kind), "version {v}");
            same_box(v, y.bounds(), x.bounds(), EPS);
            assert_eq!(x.fill, y.fill, "version {v}");
            assert_eq!(x.stroke, y.stroke, "version {v}");
        }
    }
}

// ---- structural invariants ------------------------------------------------

mod structure {
    use std::collections::HashSet;
    use std::io::Read as _;
    use tracedraw_cdr::riff::{Chunk, Tree};
    use tracedraw_cdr::{check, container, loda, riff};

    /// Every structural check on one written file.
    pub fn verify(bytes: &[u8], v: u16) {
        let findings = check::check_bytes(bytes).unwrap_or_else(|e| panic!("version {v}: {e}"));
        assert!(
            findings.is_empty(),
            "version {v}: the checker reports {findings:?}"
        );
        let c = container::detect(bytes).unwrap_or_else(|e| panic!("version {v}: {e}"));
        assert_eq!(
            c.version.0, v,
            "version {v}: container says {}",
            c.version.0
        );
        let main = container::riff_stream(bytes, &c).unwrap_or_else(|e| panic!("version {v}: {e}"));
        raw_chunks(&main, v);
        let externals = container::external_streams(bytes, &c);
        let tree = riff::parse_with_externals(&main, externals.clone(), v)
            .unwrap_or_else(|e| panic!("version {v}: {e}"));
        nesting(&tree.root, v);
        required_chunks(&tree, &main, v);
        bitmap_references(&tree, &main, v);
        if v >= 16 {
            package_v16(bytes, &main, v);
        } else if v >= 14 {
            package_v14(bytes, v);
        }
    }

    fn u32_at(d: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(d[at..at + 4].try_into().expect("4 bytes"))
    }

    /// The RIFF stream byte by byte: every chunk's size fits its parent,
    /// the chunks of a list fill it exactly, odd sizes are padded with a
    /// zero byte, and the file's own size is its length.
    fn raw_chunks(main: &[u8], v: u16) {
        assert!(
            main.len() >= 12 && &main[0..4] == b"RIFF",
            "version {v}: no RIFF header"
        );
        assert_eq!(
            u32_at(main, 4) as usize + 8,
            main.len(),
            "version {v}: RIFF size"
        );
        let mut count = 0usize;
        walk_raw(main, 12, main.len(), v, &mut count, 0);
        // Before version 16 the stream holds its contents in two
        // compressed lists, which count as one chunk each here.
        assert!(count >= 3, "version {v}: only {count} chunks");
    }

    fn walk_raw(d: &[u8], mut at: usize, end: usize, v: u16, count: &mut usize, depth: usize) {
        assert!(depth < 32, "version {v}: lists nested {depth} deep");
        while at < end {
            assert!(
                at + 8 <= end,
                "version {v}: a chunk header runs past its list at {at}"
            );
            let id = &d[at..at + 4];
            let size = u32_at(d, at + 4) as usize;
            let body = at + 8;
            assert!(
                body + size <= end,
                "version {v}: chunk {} at {at} of {size} bytes runs past its list ending at {end}",
                String::from_utf8_lossy(id)
            );
            *count += 1;
            if id == b"LIST" {
                assert!(size >= 4, "version {v}: a list without a type at {at}");
                let kind = &d[body..body + 4];
                // Compressed lists and the style table before version 16
                // hold records, not chunks.
                let raw = kind == b"cmpr" || (kind == b"stlt" && v < 16);
                if !raw {
                    walk_raw(d, body + 4, body + size, v, count, depth + 1);
                }
            }
            at = body + size;
            if size % 2 == 1 {
                assert!(
                    at < end && d[at] == 0,
                    "version {v}: padding at {at} is not a zero byte"
                );
                at += 1;
            }
        }
        assert_eq!(at, end, "version {v}: a list's chunks overrun its end");
    }

    /// The parsed tree (compressed streams inflated, redirects followed):
    /// children lie inside their parent, in order, without overlapping.
    fn nesting(c: &Chunk, v: u16) {
        let mut last_end = c.start;
        for k in &c.children {
            if k.stream == c.stream {
                assert!(
                    k.start >= last_end && k.end <= c.end && k.start <= k.end,
                    "version {v}: chunk {} [{}..{}] outside its parent {} [{}..{}] or its sibling",
                    k.id_str(),
                    k.start,
                    k.end,
                    c.id_str(),
                    c.start,
                    c.end
                );
                last_end = k.end;
            }
            nesting(k, v);
        }
    }

    fn has(c: &Chunk, name: &[u8; 4]) -> bool {
        c.children.iter().any(|k| k.is(name))
    }

    fn list_type(c: &Chunk) -> Option<[u8; 4]> {
        c.list_type
    }

    /// The chunks each list must hold for a reader to take it.
    fn required_chunks(tree: &Tree, main: &[u8], v: u16) {
        let mut objects = 0usize;
        let mut texts = 0usize;
        // The depth of the `LIST clpt` being walked: the clip container
        // groups in it and the objects they hold carry no static id
        // (`usdn`), as the files have them (#1222).
        let mut clpt: Option<usize> = None;
        tree.root.walk(&mut |c, depth| {
            if clpt.is_some_and(|d| depth <= d) {
                clpt = None;
            }
            let Some(kind) = list_type(c) else { return };
            if kind == *b"clpt" {
                clpt = Some(depth);
            }
            let id: &[&[u8; 4]] = if clpt.is_some() { &[] } else { &[b"usdn"] };
            let need = |names: &[&[u8; 4]]| {
                for n in names {
                    assert!(
                        has(c, n),
                        "version {v}: list {} lacks {}",
                        String::from_utf8_lossy(&kind),
                        String::from_utf8_lossy(*n)
                    );
                }
            };
            match &kind {
                b"doc " => {
                    need(&[b"mcfg", b"ptrt", b"fntt", b"filt", b"otlt"]);
                    assert!(
                        has(c, b"stlt")
                            || c.children.iter().any(|k| list_type(k) == Some(*b"stlt")),
                        "version {v}: no style table"
                    );
                    if v < 16 {
                        need(&[b"pfrd"]);
                    }
                }
                b"page" => need(&[b"flgs", b"bbox", b"gobj"]),
                b"layr" => need(&[b"flgs", b"lgob"]),
                b"grp " => {
                    need(&[b"flgs", b"bbox", b"obbx", b"lgob"]);
                    need(id);
                }
                b"lgob" => need(&[b"loda"]),
                b"trfl" => need(&[b"trfd"]),
                b"obj " => {
                    objects += 1;
                    need(&[b"flgs", b"bbox", b"obbx", b"lgob"]);
                    need(id);
                    let lgob = c.find(b"lgob").expect("lgob");
                    assert!(
                        has(lgob, b"loda") && has(lgob, b"ftil") && has(lgob, b"trfl"),
                        "version {v}: an object's attributes lack loda, ftil or trfl"
                    );
                    let loda = lgob.find(b"loda").expect("loda");
                    let table = loda::table(tree.data(main, loda), v)
                        .unwrap_or_else(|| panic!("version {v}: a loda does not decode"));
                    if table.kind == 0x04 {
                        texts += 1;
                        assert!(has(c, b"spnd"), "version {v}: a text without spnd");
                        assert!(has(c, b"txsm"), "version {v}: a text without txsm");
                        if v >= 11 {
                            assert!(has(c, b"txtj"), "version {v}: a text without txtj");
                        }
                    }
                    // Every object names a fill, an outline and its id.
                    for ty in [10u32, 20, 100] {
                        assert!(
                            table.args.iter().any(|a| a.ty == ty),
                            "version {v}: an object of kind {:#x} lacks argument {ty}",
                            table.kind
                        );
                    }
                    let has_style = table.args.iter().any(|a| a.ty == 200);
                    let has_string = table.args.iter().any(|a| a.ty == 201);
                    assert_eq!(
                        has_style,
                        v < 16,
                        "version {v}: style argument 200 on an object"
                    );
                    assert_eq!(
                        has_string,
                        v >= 16,
                        "version {v}: style string 201 on an object"
                    );
                }
                _ => {}
            }
        });
        let _ = texts;
        assert!(objects > 0, "version {v}: no objects");
    }

    /// Every bitmap object names a picture of the bitmap list.
    fn bitmap_references(tree: &Tree, main: &[u8], v: u16) {
        let mut pictures: HashSet<u32> = HashSet::new();
        let mut wanted: Vec<u32> = Vec::new();
        tree.root.walk(&mut |c, _| {
            if c.is(b"bmp ") {
                let d = tree.data(main, c);
                if d.len() >= 4 {
                    pictures.insert(u32_at(d, 0));
                }
            }
            if c.is(b"loda") {
                let d = tree.data(main, c);
                if let Some(t) = loda::table(d, v) {
                    if t.kind == 0x05 {
                        let coords = t.args.iter().find(|a| a.ty == 30).expect("coords");
                        let at = coords.off + 48;
                        assert!(
                            coords.end >= at + 4,
                            "version {v}: short bitmap coordinates"
                        );
                        wanted.push(u32_at(d, at));
                    }
                }
            }
        });
        for id in wanted {
            assert!(
                pictures.contains(&id),
                "version {v}: a bitmap object names picture {id}, which the file lacks"
            );
        }
    }

    fn members(bytes: &[u8]) -> Vec<(String, Vec<u8>, bool)> {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip");
        let mut out = Vec::new();
        for i in 0..z.len() {
            let mut f = z.by_index(i).expect("member");
            let mut data = Vec::new();
            f.read_to_end(&mut data).expect("read");
            let stored = f.compression() == zip::CompressionMethod::Stored;
            out.push((f.name().to_string(), data, stored));
        }
        out
    }

    /// Versions 14 and 15: the RIFF stream stored and the metadata members.
    fn package_v14(bytes: &[u8], v: u16) {
        let m = members(bytes);
        let names: Vec<&str> = m.iter().map(|(n, _, _)| n.as_str()).collect();
        for want in [
            "content/riffData.cdr",
            "metadata/metadata.xml",
            "metadata/textinfo.xml",
            "links.xml",
        ] {
            assert!(
                names.contains(&want),
                "version {v}: no member {want}: {names:?}"
            );
        }
        let riff = m
            .iter()
            .find(|(n, _, _)| n == "content/riffData.cdr")
            .expect("riff member");
        assert!(riff.2, "version {v}: the RIFF stream is not stored");
    }

    /// Version 16 and later: `mimetype` first and stored, the manifest
    /// naming the root file, every data file listed, and every redirect
    /// record of the root stream inside the data file it names.
    fn package_v16(bytes: &[u8], main: &[u8], v: u16) {
        let m = members(bytes);
        let first = m.first().expect("a member");
        assert_eq!(first.0, "mimetype", "version {v}: first member {}", first.0);
        assert!(
            first.2 && !first.1.is_empty(),
            "version {v}: mimetype not stored"
        );
        let names: Vec<&str> = m.iter().map(|(n, _, _)| n.as_str()).collect();
        for want in [
            "content/root.dat",
            "content/dataFileList.dat",
            "META-INF/container.xml",
            "META-INF/metadata.xml",
            "META-INF/textinfo.xml",
            "color/color.xml",
            "color/docPalette.xml",
            "styles/document.cdss",
        ] {
            assert!(
                names.contains(&want),
                "version {v}: no member {want}: {names:?}"
            );
        }
        let container = m
            .iter()
            .find(|(n, _, _)| n == "META-INF/container.xml")
            .expect("manifest");
        assert!(
            String::from_utf8_lossy(&container.1).contains("full-path=\"content/root.dat\""),
            "version {v}: the manifest does not name the root file"
        );
        let list = m
            .iter()
            .find(|(n, _, _)| n == "content/dataFileList.dat")
            .expect("data file list");
        let listed: Vec<String> = String::from_utf8_lossy(&list.1)
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
        let sizes: Vec<usize> = listed
            .iter()
            .map(|name| {
                m.iter()
                    .find(|(n, _, _)| *n == format!("content/data/{name}"))
                    .unwrap_or_else(|| panic!("version {v}: listed data file {name} missing"))
                    .1
                    .len()
            })
            .collect();
        for (n, _, _) in &m {
            if let Some(name) = n.strip_prefix("content/data/") {
                assert!(
                    listed.iter().any(|l| l == name),
                    "version {v}: data file {name} is not listed"
                );
            }
        }
        // Every leaf of the root stream is a redirect record.
        let mut records = 0usize;
        walk_records(main, 12, main.len(), v, &sizes, &mut records);
        assert!(records > 10, "version {v}: {records} redirect records");
    }

    fn walk_records(d: &[u8], mut at: usize, end: usize, v: u16, sizes: &[usize], n: &mut usize) {
        while at + 8 <= end {
            let id = &d[at..at + 4];
            let size = u32_at(d, at + 4) as usize;
            let body = at + 8;
            if id == b"LIST" {
                walk_records(d, body + 4, body + size, v, sizes, n);
            } else {
                assert_eq!(
                    size,
                    16,
                    "version {v}: chunk {} is not a 16-byte record",
                    String::from_utf8_lossy(id)
                );
                let stream = u32_at(d, body);
                let len = u32_at(d, body + 4) as usize;
                let offset = u32_at(d, body + 8) as usize;
                if stream == u32::MAX {
                    assert!(len <= 8, "version {v}: inline record of {len} bytes");
                } else {
                    let file = stream as usize;
                    assert!(
                        file < sizes.len(),
                        "version {v}: a record names data file {file} of {}",
                        sizes.len()
                    );
                    assert!(
                        offset + len <= sizes[file],
                        "version {v}: a record reaches {} past data file {file} of {} bytes",
                        offset + len,
                        sizes[file]
                    );
                }
                *n += 1;
            }
            at = body + size + size % 2;
        }
    }
}
