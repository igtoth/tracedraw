use super::*;
use tracedraw_core::{
    document::{Layer, Page, ParagraphStyle},
    geometry::Size,
};

fn doc_with(shapes: Vec<Shape>) -> Document {
    let mut doc = Document::new("w", Size::new(200.0, 100.0));
    doc.pages[0].layers[0].name = "Ink".into();
    doc.pages[0].layers[0].shapes = shapes;
    doc
}

#[test]
fn rectangle_ellipse_curve_and_text_round_trip() {
    let mut doc = Document::new("w", Size::new(200.0, 100.0));
    let mut ids = doc.ids().clone();
    let mut r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(10.0, 20.0, 60.0, 50.0),
            radius: 2.0,
            corners: None,
        },
    );
    r.fill = Fill::Solid(Color::rgb8(255, 0, 0));
    let mut st = Stroke::new(Color::rgb8(0, 0, 255), 1.5);
    st.cap = LineCap::Round;
    st.join = LineJoin::Bevel;
    st.dash = vec![3.0, 1.0];
    st.scale_with_object = true;
    r.stroke = Some(st.clone());
    r.name = Some("box".into());
    r.opacity = 0.5;
    let mut e = Shape::new(
        ids.shape(),
        ShapeKind::Ellipse {
            rect: Rect::new(100.0, 10.0, 140.0, 40.0),
            arc: None,
        },
    );
    e.fill = Fill::Fountain(tracedraw_core::Fountain::two(
        FountainKind::Radial,
        Color::BLACK,
        Color::WHITE,
        0.0,
    ));
    e.stroke = None;
    let mut p = BezPath::new();
    p.move_to((10.0, 60.0));
    p.curve_to((20.0, 90.0), (40.0, 90.0), (50.0, 60.0));
    p.line_to((30.0, 55.0));
    p.close_path();
    let mut c = Shape::new(
        ids.shape(),
        ShapeKind::Path {
            path: p,
            closed: true,
        },
    );
    c.fill = Fill::Solid(Color::cmyk_pct(0.0, 100.0, 100.0, 0.0));
    c.stroke = Some(Stroke::hairline(Color::BLACK));
    c.transform = Affine::translate((100.0, 0.0));
    let mut t = Shape::new(
        ids.shape(),
        ShapeKind::Text {
            spans: vec![{
                let mut s = TextSpan::new("Hello", "Verdana", 24.0);
                s.bold = true;
                s
            }],
            origin: Point::new(20.0, 80.0),
            frame: None,
            align: TextAlign::Left,
            para: ParagraphStyle::default(),
            on_path: None,
        },
    );
    t.fill = Fill::Solid(Color::rgb8(0, 128, 0));
    t.stroke = None;
    doc.pages[0].layers[0].name = "Ink".into();
    doc.pages[0].layers[0].shapes = vec![r, e, c, t];
    doc.set_ids(ids);
    for v in versions() {
        shapes_round_trip(&doc, v);
    }
}

fn shapes_round_trip(doc: &Document, v: u16) {
    let bytes = document_to_cdr_version(doc, v);
    let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
    assert_eq!(rep.version.map(|v| v.0), Some(v));
    assert_eq!(rep.skipped_objects, 0, "{v}: {:?}", rep.warnings);
    assert_eq!(back.pages.len(), 1);
    let page = &back.pages[0];
    assert!((page.size.width - 200.0).abs() < 0.01 && (page.size.height - 100.0).abs() < 0.01);
    assert_eq!(page.layers.len(), 1);
    assert_eq!(page.layers[0].name, "Ink");
    let shapes = &page.layers[0].shapes;
    assert_eq!(shapes.len(), 4, "{v}: {:?}", rep.warnings);
    // Order and geometry.
    let rb = shapes[0].bounds();
    assert!(
        (rb.x0 - 10.0).abs() < 0.01 && (rb.y1 - 50.0).abs() < 0.01,
        "{v}: {rb:?}"
    );
    if v < 15 {
        assert!(matches!(shapes[0].kind, ShapeKind::Rect { .. }), "{v}");
        if let ShapeKind::Rect { radius, .. } = shapes[0].kind {
            assert!((radius - 2.0).abs() < 0.01, "{v}");
        }
    } else {
        // Later rectangles are written as the curve they draw.
        assert!(
            matches!(shapes[0].kind, ShapeKind::Path { closed: true, .. }),
            "{v}"
        );
        let area = shapes[0].page_path().area().abs();
        let rounded = 50.0 * 30.0 - (4.0 - std::f64::consts::PI) * 4.0;
        assert!((area - rounded).abs() < 0.5, "{v}: {area}");
    }
    assert_eq!(shapes[0].fill, Fill::Solid(Color::rgb8(255, 0, 0)), "{v}");
    let s0 = shapes[0].stroke.as_ref().expect("stroke");
    assert_eq!(s0.color, Color::rgb8(0, 0, 255));
    assert!((s0.width - 1.5).abs() < 0.01);
    assert_eq!(s0.cap, LineCap::Round);
    assert_eq!(s0.join, LineJoin::Bevel);
    assert_eq!(s0.dash, vec![3.0, 1.0]);
    assert!(s0.scale_with_object, "{v}");
    assert_eq!(shapes[0].name.as_deref(), Some("box"), "{v}");
    // Versions 13 to 15 do not keep the transparency yet.
    let want = if (13..=15).contains(&v) { 1.0 } else { 0.5 };
    assert!((shapes[0].opacity - want).abs() < 0.01, "{v}");
    let eb = shapes[1].bounds();
    assert!(
        (eb.x0 - 100.0).abs() < 0.01 && (eb.y0 - 10.0).abs() < 0.01,
        "{eb:?}"
    );
    assert!(
        matches!(shapes[1].fill, Fill::Fountain(_)),
        "{v}: {:?}",
        shapes[1].fill
    );
    if let Fill::Fountain(f) = &shapes[1].fill {
        assert_eq!(f.kind, FountainKind::Radial, "{v}");
        assert_eq!(f.stops.len(), 2, "{v}");
        assert_eq!(f.stops[1].color, Color::WHITE, "{v}");
    }
    assert!(shapes[1].stroke.is_none());
    let cb = shapes[2].bounds();
    assert!(
        (cb.x0 - 110.0).abs() < 0.05 && (cb.x1 - 150.0).abs() < 0.05,
        "{cb:?}"
    );
    assert!(matches!(
        shapes[2].kind,
        ShapeKind::Path { closed: true, .. }
    ));
    assert_eq!(
        shapes[2].fill,
        Fill::Solid(Color::cmyk_pct(0.0, 100.0, 100.0, 0.0))
    );
    assert!((shapes[2].stroke.as_ref().expect("hairline").width - Stroke::HAIRLINE).abs() < 1e-9);
    match &shapes[3].kind {
        ShapeKind::Text { spans, .. } => {
            assert_eq!(spans.len(), 1, "{v}: {spans:?}");
            assert_eq!(spans[0].text, "Hello", "{v}");
            assert_eq!(spans[0].font_family, "Verdana", "{v}");
            assert!((spans[0].size_pt - 24.0).abs() < 0.01, "{v}");
            assert!(spans[0].bold, "{v}");
        }
        k => panic!("{v}: {k:?}"),
    }
    let o = shapes[3].transform * Point::ZERO;
    let origin = match &shapes[3].kind {
        ShapeKind::Text { origin, .. } => *origin,
        _ => unreachable!(),
    };
    let at = shapes[3].transform * origin;
    assert!(
        (at.x - 20.0).abs() < 0.05 && (at.y - 80.0).abs() < 0.05,
        "{v}: {o:?} {at:?}"
    );
    assert_eq!(shapes[3].fill, Fill::Solid(Color::rgb8(0, 128, 0)), "{v}");
}

#[test]
fn groups_bitmaps_and_several_pages_round_trip() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut a = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        },
    );
    a.fill = Fill::Solid(Color::BLACK);
    let mut b = a.clone();
    b.id = ids.shape();
    b.transform = Affine::translate((20.0, 0.0));
    let mut g = Shape::new(
        ids.shape(),
        ShapeKind::Group {
            children: vec![a, b],
        },
    );
    g.transform = Affine::translate((5.0, 5.0));
    let png = {
        let mut pm = tiny_skia::Pixmap::new(2, 2).expect("pixmap");
        let px = pm.pixels_mut();
        px[0] = tiny_skia::ColorU8::from_rgba(255, 0, 0, 255).premultiply();
        // See-through pixels keep their transparency.
        px[1] = tiny_skia::ColorU8::from_rgba(0, 255, 0, 128).premultiply();
        px[2] = tiny_skia::ColorU8::from_rgba(0, 0, 255, 0).premultiply();
        px[3] = tiny_skia::ColorU8::from_rgba(255, 255, 255, 255).premultiply();
        pm.encode_png().expect("png")
    };
    let mut bm = Shape::new(
        ids.shape(),
        ShapeKind::Bitmap {
            rect: Rect::new(100.0, 50.0, 140.0, 90.0),
            width_px: 2,
            height_px: 2,
            png: png.into(),
            fx: None,
        },
    );
    bm.fill = Fill::None;
    bm.stroke = None;
    doc.pages[0].layers[0].shapes = vec![g, bm];
    let pid = ids.page();
    let lid = ids.layer();
    let mut second = Page {
        id: pid,
        name: "Back".into(),
        size: Size::new(200.0, 100.0),
        layers: vec![Layer::new(lid, "Second")],
        guides: Vec::new(),
        background: None,
    };
    let mut c = Shape::new(
        ids.shape(),
        ShapeKind::Ellipse {
            rect: Rect::new(10.0, 10.0, 30.0, 30.0),
            arc: None,
        },
    );
    c.fill = Fill::Solid(Color::rgb8(1, 2, 3));
    second.layers[0].shapes.push(c);
    doc.pages.push(second);
    doc.set_ids(ids);
    for v in versions() {
        groups_bitmaps_and_pages(&doc, v);
    }
}

fn groups_bitmaps_and_pages(doc: &Document, v: u16) {
    let bytes = document_to_cdr_version(doc, v);
    let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
    assert_eq!(back.pages.len(), 2, "{v}: {:?}", rep.warnings);
    assert_eq!(rep.skipped_objects, 0, "{:?}", rep.warnings);
    let l = &back.pages[0].layers[0];
    assert_eq!(l.shapes.len(), 2, "{v}: {:?}", rep.warnings);
    match &l.shapes[0].kind {
        ShapeKind::Group { children } => {
            assert_eq!(children.len(), 2);
            let gb = l.shapes[0].bounds();
            assert!(
                (gb.x0 - 5.0).abs() < 0.01 && (gb.x1 - 35.0).abs() < 0.01,
                "{gb:?}"
            );
        }
        k => panic!("{k:?}"),
    }
    match &l.shapes[1].kind {
        ShapeKind::Bitmap {
            width_px,
            height_px,
            png,
            ..
        } => {
            assert_eq!((*width_px, *height_px), (2, 2));
            let pm = tiny_skia::Pixmap::decode_png(png).expect("png");
            let p = pm.pixels();
            assert_eq!((p[0].red(), p[0].green(), p[0].blue()), (255, 0, 0));
            assert_eq!((p[3].red(), p[3].green(), p[3].blue()), (255, 255, 255));
            let alpha: Vec<u8> = p.iter().map(|p| p.alpha()).collect();
            assert_eq!(alpha, [255, 128, 0, 255], "{v}");
            assert!(p[1].demultiply().green() > 250, "{v}");
        }
        k => panic!("{k:?}"),
    }
    let bb = l.shapes[1].bounds();
    assert!(
        (bb.x0 - 100.0).abs() < 0.01 && (bb.y1 - 90.0).abs() < 0.01,
        "{bb:?}"
    );
    assert_eq!(back.pages[1].layers[0].name, "Second");
    assert_eq!(back.pages[1].layers[0].shapes.len(), 1);
}

#[test]
fn a_clipped_bitmap_round_trips_as_a_crop_path() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let png = {
        let mut pm = tiny_skia::Pixmap::new(4, 4).expect("pixmap");
        pm.fill(tiny_skia::Color::from_rgba8(10, 20, 30, 255));
        pm.encode_png().expect("png")
    };
    let mut bm = Shape::new(
        ids.shape(),
        ShapeKind::Bitmap {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            width_px: 4,
            height_px: 4,
            png: png.into(),
            fx: None,
        },
    );
    bm.fill = Fill::None;
    bm.stroke = None;
    bm.transform = Affine::translate((20.0, 20.0));
    let mut frame = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(40.0, 30.0, 80.0, 60.0),
            radius: 0.0,
            corners: None,
        },
    );
    frame.fill = Fill::None;
    frame.stroke = None;
    let mut pc = Shape::new(
        ids.shape(),
        ShapeKind::ClipFrame {
            frame: Box::new(frame),
            contents: vec![bm],
        },
    );
    pc.fill = Fill::None;
    pc.stroke = None;
    doc.pages[0].layers[0].shapes = vec![pc];
    doc.set_ids(ids);
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        clipped_bitmap_reads_back(&back, &rep, v);
    }
}

fn clipped_bitmap_reads_back(back: &Document, rep: &crate::ParseReport, v: u16) {
    let l = &back.pages[0].layers[0];
    assert_eq!(l.shapes.len(), 1, "{v}: {:?}", rep.warnings);
    match &l.shapes[0].kind {
        ShapeKind::ClipFrame { frame, contents } => {
            // Frame and contents live in the clip frame's own space.
            let outer = l.shapes[0].transform;
            let fb = (outer * frame.page_path()).bounding_box();
            assert!(
                (fb.x0 - 40.0).abs() < 0.05 && (fb.x1 - 80.0).abs() < 0.05,
                "{fb:?}"
            );
            assert!(
                (fb.y0 - 30.0).abs() < 0.05 && (fb.y1 - 60.0).abs() < 0.05,
                "{fb:?}"
            );
            assert_eq!(contents.len(), 1);
            let cb = (outer * contents[0].page_path()).bounding_box();
            assert!(
                (cb.x0 - 20.0).abs() < 0.05 && (cb.x1 - 120.0).abs() < 0.05,
                "{cb:?}"
            );
        }
        k => panic!("{k:?}"),
    }
}

#[test]
fn written_files_survive_mutation() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        },
    );
    r.fill = Fill::Solid(Color::BLACK);
    doc.pages[0].layers[0].shapes = vec![r];
    doc.set_ids(ids);
    for v in [7, 12, 13, 15, 21] {
        let bytes = document_to_cdr_version(&doc, v);
        for cut in (0..bytes.len()).step_by(13) {
            let _ = crate::open_bytes(&bytes[..cut], "cut");
        }
        for i in (0..bytes.len()).step_by(7) {
            let mut m = bytes.clone();
            m[i] ^= 0x5a;
            let _ = crate::open_bytes(&m, "flip");
        }
    }
}

#[test]
fn corners_one_radius_cannot_hold_are_written_as_curves_and_stretched_fixed_ones_as_rectangles() {
    use tracedraw_core::{CornerKind, Corners};
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let rect = Rect::new(0.0, 0.0, 40.0, 20.0);
    let mut per_corner = Corners::uniform(2.0, CornerKind::Round);
    per_corner.radii[Corners::TOP_LEFT] = 8.0;
    let mut fixed = Corners::uniform(3.0, CornerKind::Round);
    fixed.fixed = true;
    let mut shapes = Vec::new();
    for (corners, scale) in [
        (Corners::uniform(4.0, CornerKind::Scallop), (1.0, 1.0)),
        (per_corner, (1.0, 1.0)),
        (fixed, (2.0, 1.0)),
        (fixed, (2.0, 2.0)),
    ] {
        let mut s = Shape::new(ids.shape(), ShapeKind::rect_with_corners(rect, corners));
        s.transform = Affine::translate((20.0, 20.0)) * Affine::scale_non_uniform(scale.0, scale.1);
        s.fill = Fill::Solid(Color::BLACK);
        shapes.push(s);
    }
    doc.pages[0].layers[0].shapes = shapes.clone();
    doc.set_ids(ids);
    for v in versions() {
        let (back, rep) =
            crate::open_bytes(&document_to_cdr_version(&doc, v), "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 4, "{v}: {:?}", rep.warnings);
        for (i, (a, b)) in shapes.iter().zip(read).enumerate() {
            let (aa, ab) = (a.page_path().area().abs(), b.page_path().area().abs());
            assert!((aa - ab).abs() < 0.5, "{v} {i}: {aa} vs {ab}");
            let (ba, bb) = (a.bounds(), b.bounds());
            assert!(
                (ba.x0 - bb.x0).abs() < 0.01 && (ba.y1 - bb.y1).abs() < 0.01,
                "{v} {i}"
            );
        }
        assert!(matches!(read[0].kind, ShapeKind::Path { closed: true, .. }));
        assert!(matches!(read[1].kind, ShapeKind::Path { closed: true, .. }));
        if v < 15 {
            // A fixed radius on a stretched rectangle: the stretch goes into
            // the size, the corners stay 3 mm circles.
            assert!(
                matches!(read[2].kind, ShapeKind::Rect { radius, .. } if (radius - 3.0).abs() < 0.01),
                "{v}: {:?}",
                read[2].kind
            );
            let page = read[2].page_corners().expect("corners");
            assert!(page.radii.iter().all(|r| (r - 3.0).abs() < 0.01), "{v}");
            let b = read[2].bounds();
            assert!((b.width() - 80.0).abs() < 0.01 && (b.height() - 20.0).abs() < 0.01);
            // A fixed radius on an evenly scaled rectangle is still a
            // rectangle, its corners 3 mm on the page and kept so.
            assert!(
                matches!(read[3].kind, ShapeKind::Rect { corners: Some(c), .. } if c.fixed),
                "{v}"
            );
            let page = read[3].page_corners().expect("corners");
            assert!(page.radii.iter().all(|r| (r - 3.0).abs() < 0.01), "{v}");
        } else {
            assert!(matches!(read[2].kind, ShapeKind::Path { closed: true, .. }));
        }
    }
}

#[test]
fn complex_stars_are_written_as_curves() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut star = Shape::new(
        ids.shape(),
        ShapeKind::Polygon {
            rect: Rect::new(10.0, 10.0, 50.0, 50.0),
            points: 6,
            sharpness: 0.0,
            complex: Some(1),
            twist: 0.0,
        },
    );
    star.fill = Fill::Solid(Color::BLACK);
    doc.pages[0].layers[0].shapes = vec![star.clone()];
    doc.set_ids(ids);
    for v in [7, 12, 16, 22] {
        complex_star_reads_back(&doc, &star, v);
    }
}

fn complex_star_reads_back(doc: &Document, star: &Shape, v: u16) {
    let (back, rep) =
        crate::open_bytes(&document_to_cdr_version(doc, v), "back").expect("reads back");
    let read = &back.pages[0].layers[0].shapes;
    assert_eq!(read.len(), 1, "{:?}", rep.warnings);
    let ShapeKind::Path { path, .. } = &read[0].kind else {
        panic!("{:?}", read[0].kind);
    };
    // Two triangles, as drawn.
    let moves = path
        .elements()
        .iter()
        .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
        .count();
    assert_eq!(moves, 2);
    let (a, b) = (
        star.page_path().area().abs(),
        read[0].page_path().area().abs(),
    );
    assert!((a - b).abs() < 0.5, "{a} vs {b}");
}

fn text_shape(
    ids: &mut tracedraw_core::id::IdSource,
    spans: Vec<TextSpan>,
    align: TextAlign,
) -> Shape {
    let mut t = Shape::new(
        ids.shape(),
        ShapeKind::Text {
            spans,
            origin: Point::new(0.0, 0.0),
            frame: None,
            align,
            para: tracedraw_core::document::ParagraphStyle::default(),
            on_path: None,
        },
    );
    t.transform = Affine::translate((50.0, 50.0));
    t.fill = Fill::Solid(Color::BLACK);
    t.stroke = None;
    t
}

/// Several runs and lines, each text with its alignment, read back the
/// same from every version.
#[test]
fn runs_lines_and_alignment_round_trip() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut red = TextSpan::new("Red ", "Georgia", 18.0);
    red.fill = Some(Fill::Solid(Color::rgb8(255, 0, 0)));
    red.italic = true;
    let mut big = TextSpan::new("big\nline two", "Verdana", 30.0);
    big.underline = true;
    let shapes = vec![
        text_shape(&mut ids, vec![red, big], TextAlign::Center),
        text_shape(
            &mut ids,
            vec![TextSpan::new("right", "Arial", 12.0)],
            TextAlign::Right,
        ),
    ];
    doc.pages[0].layers[0].shapes = shapes;
    doc.set_ids(ids);
    for v in versions() {
        let (back, rep) =
            crate::open_bytes(&document_to_cdr_version(&doc, v), "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 2, "{v}: {:?}", rep.warnings);
        let ShapeKind::Text { spans, align, .. } = &read[0].kind else {
            panic!("{v}: {:?}", read[0].kind);
        };
        assert_eq!(*align, TextAlign::Center, "{v}");
        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "Red big\nline two", "{v}: {spans:?}");
        let first = &spans[0];
        assert_eq!(first.font_family, "Georgia", "{v}");
        assert!(first.italic && !first.underline, "{v}");
        assert_eq!(first.fill, Some(Fill::Solid(Color::rgb8(255, 0, 0))), "{v}");
        let last = spans.last().expect("span");
        assert_eq!(last.font_family, "Verdana", "{v}");
        assert!((last.size_pt - 30.0).abs() < 0.01, "{v}");
        assert!(last.underline && !last.italic, "{v}");
        let ShapeKind::Text { align, .. } = &read[1].kind else {
            panic!("{v}: {:?}", read[1].kind);
        };
        assert_eq!(*align, TextAlign::Right, "{v}");
    }
}

/// Paragraphs of one text with alignments of their own keep them in
/// every version.
#[test]
fn paragraph_alignments_round_trip() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut t = text_shape(
        &mut ids,
        vec![TextSpan::new("left\ncentre\nright", "Arial", 12.0)],
        TextAlign::Left,
    );
    tracedraw_core::paragraphs::edit_paragraph_formats(&mut t.kind, Some((6, 6)), |f| {
        f.align = TextAlign::Center
    });
    tracedraw_core::paragraphs::edit_paragraph_formats(&mut t.kind, Some((14, 14)), |f| {
        f.align = TextAlign::Right
    });
    doc.pages[0].layers[0].shapes = vec![t];
    doc.set_ids(ids);
    for v in versions() {
        let (back, rep) =
            crate::open_bytes(&document_to_cdr_version(&doc, v), "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 1, "{v}: {:?}", rep.warnings);
        let ShapeKind::Text {
            spans, align, para, ..
        } = &read[0].kind
        else {
            panic!("{v}: {:?}", read[0].kind);
        };
        let aligns: Vec<TextAlign> =
            tracedraw_core::paragraphs::paragraph_styles(spans, *align, para)
                .into_iter()
                .map(|(a, _)| a)
                .collect();
        assert_eq!(
            aligns,
            vec![TextAlign::Left, TextAlign::Center, TextAlign::Right],
            "{v}: {spans:?}"
        );
    }
}

/// Each paragraph's own line spacing goes out in its style string from
/// version 16 and reads back on that paragraph alone.
#[test]
fn paragraph_line_spacing_round_trips_from_version_16() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut t = text_shape(
        &mut ids,
        vec![TextSpan::new("one\ntwo\nthree", "Arial", 12.0)],
        TextAlign::Left,
    );
    tracedraw_core::paragraphs::edit_paragraph_formats(&mut t.kind, Some((5, 5)), |f| {
        f.leading_pct = 150.0
    });
    doc.pages[0].layers[0].shapes = vec![t];
    doc.set_ids(ids);
    for v in versions().filter(|v| *v >= 16) {
        let (back, _) =
            crate::open_bytes(&document_to_cdr_version(&doc, v), "back").expect("reads back");
        let ShapeKind::Text {
            spans, align, para, ..
        } = &back.pages[0].layers[0].shapes[0].kind
        else {
            panic!("{v}");
        };
        let leading: Vec<f64> = tracedraw_core::paragraphs::paragraph_styles(spans, *align, para)
            .iter()
            .map(|(_, p)| (p.leading_pct * 1000.0).round() / 1000.0)
            .collect();
        assert_eq!(leading, vec![100.0, 150.0, 100.0], "{v}");
    }
}

/// Text fitted to a path goes out as the outline of its letters.
#[test]
fn text_on_a_path_is_written_as_its_outline() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut t = text_shape(
        &mut ids,
        vec![TextSpan::new("arc", "Arial", 12.0)],
        TextAlign::Left,
    );
    let mut arc = BezPath::new();
    arc.move_to((0.0, 0.0));
    arc.curve_to((10.0, 20.0), (30.0, 20.0), (40.0, 0.0));
    if let ShapeKind::Text { on_path, .. } = &mut t.kind {
        *on_path = Some(tracedraw_core::TextOnPath {
            path: arc,
            offset: 0.0,
            distance: 0.0,
            mirror: false,
            ..Default::default()
        });
    }
    doc.pages[0].layers[0].shapes = vec![t];
    doc.set_ids(ids);
    for v in [7, 15, 21] {
        let (back, rep) =
            crate::open_bytes(&document_to_cdr_version(&doc, v), "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 1, "{v}: {:?}", rep.warnings);
        assert!(
            matches!(read[0].kind, ShapeKind::Path { .. }),
            "{v}: {:?}",
            read[0].kind
        );
        assert_eq!(read[0].fill, Fill::Solid(Color::BLACK), "{v}");
    }
}

/// The chunks around one rectangle and one text, as files of the version
/// have them.
fn chunk_ids(bytes: &[u8]) -> (Vec<String>, crate::riff::Tree, Vec<u8>) {
    let c = crate::container::detect(bytes).expect("container");
    let riff = crate::container::riff_stream(bytes, &c).expect("riff");
    let ext = crate::container::external_streams(bytes, &c);
    let tree = crate::riff::parse_with_externals(&riff, ext, c.version.0).expect("tree");
    let mut ids = Vec::new();
    tree.root.walk(&mut |ch, depth| {
        let name = match ch.list_type_str() {
            Some(t) => format!("{}{}", "  ".repeat(depth), t),
            None => format!("{}{}", "  ".repeat(depth), ch.id_str()),
        };
        ids.push(name);
    });
    (ids, tree, riff)
}

#[test]
fn files_are_laid_out_as_the_versions_have_them() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        },
    );
    r.fill = Fill::Solid(Color::BLACK);
    let t = text_shape(
        &mut ids,
        vec![TextSpan::new("Hi", "Arial", 12.0)],
        TextAlign::Left,
    );
    doc.pages[0].layers[0].shapes = vec![r, t];
    doc.set_ids(ids);
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        // Containers: RIFF, then ZIP with the RIFF stream, then ZIP with
        // the chunk tree and data files.
        match v {
            7..=13 => assert_eq!(&bytes[..4], b"RIFF", "{v}"),
            _ => assert_eq!(&bytes[..4], b"PK\x03\x04", "{v}"),
        }
        if v >= 14 {
            let z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("zip");
            let names: Vec<&str> = z.file_names().collect();
            if v < 16 {
                // The RIFF stream first and stored; without pictures the
                // summary, names and link list follow.
                assert_eq!(
                    names,
                    [
                        "content/riffData.cdr",
                        "metadata/metadata.xml",
                        "metadata/textinfo.xml",
                        "links.xml"
                    ],
                    "{v}"
                );
            } else {
                for n in [
                    "content/root.dat",
                    "content/dataFileList.dat",
                    "content/data/data1.dat",
                    "content/data/masterPage.dat",
                    "content/data/page1.dat",
                ] {
                    assert!(names.contains(&n), "{v}: {names:?}");
                }
            }
        }
        let (ids, tree, riff) = chunk_ids(&bytes);
        let form = Ver(v).form();
        assert_eq!(tree.root.list_type, Some(form), "{v}");
        assert_eq!(&riff[8..12], &form, "{v}");
        let at = |name: &str| ids.iter().position(|i| i.trim() == name);
        // Version chunks, then the document tables, the master page and
        // the page.
        if v >= 16 {
            assert_eq!(ids[1].trim(), "fver", "{v}");
        }
        assert!(at("vrsn") < at("doc"), "{v}: {ids:?}");
        assert!(
            at("mcfg") < at("ptrt") && at("ptrt") < at("filt"),
            "{v}: {ids:?}"
        );
        assert!(
            at("filc").is_some() && at("otlt").is_some() && at("stlt").is_some(),
            "{v}"
        );
        let pages = ids.iter().filter(|i| i.trim() == "page").count();
        assert_eq!(pages, 2, "{v}: master page and page");
        // A page lists the master's guides and desktop layers, from 14 its
        // own guides, then the drawing's layer and the master's grid.
        let layers = ids.iter().filter(|i| i.trim() == "layr").count();
        assert_eq!(layers, if v >= 14 { 3 + 5 } else { 3 + 4 }, "{v}: {ids:?}");
        // Every object: flags, boxes, static id, attributes with the
        // transform; the text starts with its number and ends with its
        // record.
        let obj = ids.iter().position(|i| i.trim() == "obj").expect("object");
        let next: Vec<&str> = ids[obj + 1..obj + 9].iter().map(|s| s.trim()).collect();
        assert_eq!(
            next,
            ["spnd", "flgs", "bbox", "obbx", "usdn", "lgob", "loda", "ftil"],
            "{v}: the text comes first, front to back"
        );
        assert!(ids.iter().any(|i| i.trim() == "txsm"), "{v}");
        // From 11 the text record is followed by its line breaking tables.
        let txsm = at("txsm").expect("txsm");
        assert_eq!(
            ids.get(txsm + 1).map(|i| i.trim()) == Some("txtj"),
            v >= 11,
            "{v}: {ids:?}"
        );
        let mut flags = Vec::new();
        tree.root.walk(&mut |ch, _| {
            if ch.is(b"flgs") {
                flags.push(tree.data(&riff, ch).to_vec());
            }
        });
        // Text is flagged 9 before version 18, 8 (as other objects) after.
        assert_eq!(flags.contains(&vec![0, 2, 0, 9]), v < 18, "{v}: text flags");
        assert!(flags.contains(&vec![0, 2, 0, 8]), "{v}: object flags");
        assert!(
            flags.iter().any(|f| f.get(2..4) == Some(&[1, 0x90])),
            "{v}: master page"
        );
    }
}

/// `loda` arguments: (type, body).
type Args = Vec<(u32, Vec<u8>)>;

/// The arguments of a `loda` record, in the order its offsets table lists
/// them.
fn loda_args(loda: &[u8]) -> Args {
    let u = |at: usize| {
        loda.get(at..at + 4)
            .and_then(|b| b.try_into().ok())
            .map_or(0, u32::from_le_bytes) as usize
    };
    let (n, offsets, types) = (u(4), u(8), u(12));
    (0..n)
        .map(|i| {
            let ty = u(types + 4 * (n - 1 - i)) as u32;
            let body = loda.get(u(offsets + 4 * i)..u(offsets + 4 * i + 4));
            (ty, body.unwrap_or(&[]).to_vec())
        })
        .collect()
}

/// A layer list's flags and its `loda` arguments.
fn layer_args(
    tree: &crate::riff::Tree,
    riff: &[u8],
    layer: &crate::riff::Chunk,
) -> ([u8; 4], Args) {
    let flags = layer
        .find(b"flgs")
        .and_then(|f| tree.data(riff, f).try_into().ok())
        .unwrap_or([0; 4]);
    let loda = layer
        .find(b"lgob")
        .and_then(|l| l.find(b"loda"))
        .map(|c| tree.data(riff, c))
        .unwrap_or(&[]);
    (flags, loda_args(loda))
}

/// Pages list the master page's guides, desktop and grid layers by the
/// master's GUIDs, as every sample file of versions 14 to 22 does (entries
/// with GUIDs of their own are the likeliest reason earlier files closed
/// the program that opened them; not confirmed yet).
#[test]
fn pages_refer_to_the_master_layers_by_their_guids() {
    let mut doc = doc_with(Vec::new());
    let mut second = doc.pages[0].clone();
    second.layers[0].name = "Second".into();
    doc.pages.push(second);
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        let (_, tree, riff) = chunk_ids(&bytes);
        let mut pages = Vec::new();
        tree.root.walk(&mut |c, _| {
            if c.is(b"page") {
                pages.push(c);
            }
        });
        assert_eq!(pages.len(), 3, "{v}: the master page and two pages");
        let entries = |page: &crate::riff::Chunk| -> Vec<([u8; 4], Args)> {
            page.find(b"gobj")
                .map(|g| {
                    g.find_all(b"layr")
                        .map(|l| layer_args(&tree, &riff, l))
                        .collect()
                })
                .unwrap_or_default()
        };
        let arg = |args: &[(u32, Vec<u8>)], ty: u32| {
            args.iter().find(|(t, _)| *t == ty).map(|(_, b)| b.clone())
        };
        let master = entries(pages[0]);
        let master_flags: Vec<[u8; 4]> = master.iter().map(|(f, _)| *f).collect();
        assert_eq!(
            master_flags,
            [[0x0a, 0, 1, 0x98], [0x08, 0, 1, 0x98], [0x1a, 1, 1, 0x98]],
            "{v}"
        );
        // The master page closes with an empty object list from 16; pages
        // do not.
        assert_eq!(pages[0].find(b"lgob").is_some(), v >= 16, "{v}");
        let mut own_guids = Vec::new();
        for page in &pages[1..] {
            assert!(page.find(b"lgob").is_none(), "{v}");
            let layers = entries(page);
            let flags: Vec<[u8; 4]> = layers.iter().map(|(f, _)| *f).collect();
            let mut expect = vec![[0x0a, 0, 1, 0x98], [0x08, 0, 1, 0x98]];
            if v >= 14 {
                expect.push([0x0a, 0, 0, 0x98]);
            }
            expect.push([0, 0, 0, 0x98]);
            expect.push([0x1a, 1, 1, 0x98]);
            assert_eq!(flags, expect, "{v}");
            for (f, args) in &layers {
                let guid = arg(args, ARG_GUID);
                let name = arg(args, ARG_NAME);
                match master.iter().find(|(mf, _)| mf == f) {
                    // An entry for a master layer: its GUID, from 14 no name.
                    Some((_, m)) => {
                        assert_eq!(guid, arg(m, ARG_GUID), "{v}: {f:?}");
                        assert_eq!(name.is_none(), v >= 14, "{v}: {f:?}");
                    }
                    // A layer of the page's own: a GUID no other layer has.
                    None => {
                        assert!(name.is_some(), "{v}");
                        if let Some(g) = guid.clone() {
                            assert_eq!(g.len(), 16);
                            assert_eq!(g[7] >> 4, 4, "version 4 GUID");
                            assert_eq!(g[8] >> 6, 2, "variant bits");
                            assert!(!master
                                .iter()
                                .any(|(_, m)| arg(m, ARG_GUID) == Some(g.clone())));
                            assert!(!own_guids.contains(&g), "{v}: GUIDs differ");
                            own_guids.push(g);
                        }
                    }
                }
                assert_eq!(guid.is_some(), v >= 14, "{v}");
                // Guides layers carry 16001 = 2 from 17, every layer 12000.
                if v >= 17 {
                    assert_eq!(arg(args, ARG_PROPS), Some(vec![0; 4]), "{v}");
                }
                assert_eq!(
                    arg(args, ARG_GUIDES_MODE),
                    (v >= 17 && f[0] == 0x0a).then(|| vec![2, 0, 0, 0]),
                    "{v}: {f:?}"
                );
            }
        }
        // It still reads back as two pages of one layer each.
        let (back, _) = crate::open_bytes(&bytes, "t").expect("opens");
        assert_eq!(back.pages.len(), 2, "{v}");
        let names: Vec<&str> = back
            .pages
            .iter()
            .map(|p| p.layers[0].name.as_str())
            .collect();
        assert_eq!(names, ["Ink", "Second"], "{v}");
        assert!(back.pages.iter().all(|p| p.layers.len() == 1), "{v}");
    }
}

/// The style table numbers the entries of all its tables in one sequence,
/// and every style names entries that exist, with the three script fonts
/// filled in, as the sample files of versions 11 to 22 have them (styles
/// naming entry 0 or one id in two tables are the second likeliest reason
/// earlier files closed the program that opened them; not confirmed yet).
#[test]
fn the_style_table_numbers_its_entries_once_and_styles_name_them() {
    let uses = StyleUses {
        text_fill: 7,
        graphic_fill: 8,
        outline: 9,
        font: 3,
        formats: Vec::new(),
    };
    for v in versions() {
        let d = stlt(Ver(v), uses.clone());
        let at = std::cell::Cell::new(0usize);
        let mut u = || {
            let x = d
                .get(at.get()..at.get() + 4)
                .and_then(|b| b.try_into().ok())
                .map_or(u32::MAX, u32::from_le_bytes);
            at.set(at.get() + 4);
            x
        };
        let records = u();
        assert_eq!(records, 5, "{v}");
        let mut ids: Vec<u32> = Vec::new();
        // Fills, outlines: id, 0, record (fills from 13 with a matrix).
        let entry = |n: u32, tail: usize, ids: &mut Vec<u32>, u: &mut dyn FnMut() -> u32| {
            let mut values = Vec::new();
            for _ in 0..n {
                ids.push(u());
                assert_eq!(u(), 0);
                values.push(u());
                for _ in 0..tail / 4 {
                    u();
                }
            }
            values
        };
        let n = u();
        let fills = entry(n, if v >= 13 { 48 } else { 0 }, &mut ids, &mut u);
        assert_eq!(fills, [7, 8], "{v}");
        let n = u();
        assert_eq!(entry(n, 0, &mut ids, &mut u), [9], "{v}");
        // One font: three scripts in font 3 at 12 pt.
        assert_eq!(u(), 1, "{v}");
        ids.push(u());
        let mask: Vec<u32> = (0..if v >= 10 { 5 } else { 3 }).map(|_| u()).collect();
        assert_eq!(mask[0], 0, "{v}");
        assert_eq!(u(), 3, "{v}: font 3, character set 0");
        assert_eq!((u(), u()), (3, 3), "{v}: the other scripts' fonts");
        assert_eq!((u(), u(), u()), (42333, 42333, 42333), "{v}: sizes");
        for _ in 0..if v >= 10 { 3 } else { 1 } {
            assert_eq!(u(), 0x40, "{v}");
        }
        let n = u();
        assert_eq!(
            entry(n, 0, &mut ids, &mut u),
            [1, 2, 3, 4],
            "{v}: alignments"
        );
        // One spacing entry: 100 % lines.
        assert_eq!(u(), 1, "{v}");
        ids.push(u());
        let spacing: Vec<u32> = (0..12).map(|_| u()).collect();
        assert_eq!(spacing[5], 1_000_000, "{v}: line spacing");
        assert_eq!(spacing[2], 0, "{v}: character spacing");
        // One set of decoration lines.
        assert_eq!(u(), 1, "{v}");
        ids.push(u());
        assert_eq!(u(), 0);
        for (pos, thick) in DECORATION_LINES {
            assert_eq!((u(), u() as i32, u() as i32), (0, pos, thick), "{v}");
        }
        // No tabs, bullets, indents, hyphenation or drop caps; set11 from 9.
        for _ in 0..5 {
            assert_eq!(u(), 0, "{v}");
        }
        if v >= 9 {
            assert_eq!(u(), 1, "{v}");
            ids.push(u());
            assert_eq!((u(), u()), (0, 0));
        }
        // One sequence from 1, no id twice.
        let expect: Vec<u32> = (1..=ids.len() as u32).collect();
        assert_eq!(ids, expect, "{v}: entry ids");
        // Styles: four for text and one for graphics, naming entries that
        // exist (never 0).
        for style in 1..=5u32 {
            let sections = u();
            assert_eq!(sections, if style < 5 { 2 } else { 1 }, "{v}");
            assert_eq!(u(), style, "{v}");
            assert_eq!(u(), 0, "{v}: no parent");
            u();
            u();
            let len = u() as usize;
            if v >= 16 {
                assert_eq!(len, 0, "{v}: no names from 16");
            }
            at.set(at.get() + len * if v >= 12 { 2 } else { 1 });
            let refs = if style < 5 {
                6 + usize::from(v >= 9)
            } else {
                2
            };
            for _ in 0..refs {
                let r = u();
                assert!(ids.contains(&r), "{v}: style {style} names entry {r}");
            }
        }
        assert_eq!(at.get(), d.len(), "{v}: nothing after the styles");
        // The reader resolves every style.
        let styles = crate::text::read_stlt(&d, v, &mut |w| panic!("{w}"));
        assert_eq!(styles.len(), 5, "{v}");
        for (id, rec) in &styles {
            let s = &rec.style;
            if *id == GRAPHIC_STYLE_ID {
                assert_eq!((s.fill_id, s.outl_id), (Some(8), Some(9)), "{v}");
            } else {
                assert_eq!(
                    (s.fill_id, s.outl_id, s.font_id),
                    (Some(7), Some(9), Some(3)),
                    "{v}"
                );
                assert!((s.size_pt.unwrap_or(0.0) - 12.0).abs() < 0.01, "{v}");
                assert_eq!(s.line_spacing, Some(1.0), "{v}");
                let code = STYLE_ALIGNS[(*id - STYLE_ID) as usize].1;
                assert_eq!(s.align, Some(code), "{v}");
            }
        }
    }
}

/// Graphics refer to the graphic style (before version 16) and text
/// objects carry numbers of their own, as files have them.
#[test]
fn graphics_refer_to_the_graphic_style_and_texts_are_numbered() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        },
    );
    let texts: Vec<Shape> = (0..3)
        .map(|_| {
            text_shape(
                &mut ids,
                vec![TextSpan::new("Hi", "Arial", 12.0)],
                TextAlign::Left,
            )
        })
        .collect();
    doc.pages[0].layers[0].shapes = std::iter::once(r).chain(texts).collect();
    doc.set_ids(ids);
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        let (_, tree, riff) = chunk_ids(&bytes);
        let mut spnd = Vec::new();
        let mut styles = Vec::new();
        tree.root.walk(&mut |c, _| {
            if c.is(b"obj ") {
                if let Some(s) = c.find(b"spnd") {
                    spnd.push(tree.data(&riff, s).to_vec());
                }
                let loda = c.find(b"lgob").and_then(|l| l.find(b"loda"));
                if let Some(l) = loda {
                    let d = tree.data(&riff, l);
                    let kind = d.get(16).copied();
                    if let Some((_, s)) = loda_args(d).into_iter().find(|(t, _)| *t == ARG_STYLE) {
                        styles.push((kind, s));
                    }
                }
            }
        });
        let numbers: Vec<Vec<u8>> = (1u32..=3).map(|n| n.to_le_bytes().to_vec()).collect();
        assert_eq!(spnd, numbers, "{v}: one number per text, in order");
        if v < 16 {
            for (kind, s) in &styles {
                let want = if *kind == Some(OBJ_ARTISTIC_TEXT as u8) {
                    style_for(TextAlign::Left)
                } else {
                    GRAPHIC_STYLE_ID
                };
                assert_eq!(s, &want.to_le_bytes().to_vec(), "{v}: kind {kind:?}");
            }
            assert_eq!(styles.len(), 4, "{v}");
        } else {
            assert!(styles.is_empty(), "{v}: style strings from 16");
        }
    }
}

/// Fonts, style table, fill and outline records keep the sizes files of
/// the version have.
#[test]
fn records_have_the_sizes_of_the_versions_files() {
    let s = Stroke::new(Color::BLACK, 0.5);
    for (v, outline, solid, none) in [
        (11u16, 140usize, 36usize, 8usize),
        (14, 192, 76, 22),
        (15, 269, 97, 22),
        (17, 213, 97, 22),
        (21, 213, 97, 22),
        (22, 213, 97, 22),
    ] {
        assert_eq!(outl(Ver(v), 1, Some(&s)).len(), outline, "{v}");
        assert_eq!(
            fild(Ver(v), 1, &Fill::Solid(Color::BLACK)).len(),
            solid,
            "{v}"
        );
        assert_eq!(fild(Ver(v), 1, &Fill::None).len(), none, "{v}");
        assert_eq!(
            font_record(Ver(v), 1, "Arial", 0x40).len(),
            if v >= 12 { 296 } else { 92 },
            "{v}"
        );
        assert_eq!(
            trfd(Ver(v), Affine::IDENTITY).len(),
            if v >= 13 { 96 } else { 80 }
        );
    }
    // Two-stop gradients: stops of 16 bytes before version 13, 24 in 13
    // and 14, 45 from 15.
    let f = Fill::Fountain(tracedraw_core::Fountain::two(
        FountainKind::Linear,
        Color::BLACK,
        Color::WHITE,
        0.0,
    ));
    let len = |v: u16| fild(Ver(v), 1, &f).len();
    assert_eq!(len(12), 56 + 2 * 16, "version 12");
    assert_eq!(
        len(15) - len(14),
        2 * (45 - 24),
        "a stop grows by 21 bytes in 15"
    );
}

#[test]
fn the_version_21_configuration_record_has_its_fields() {
    let rec = mcfg(Ver(21), 297.0, 210.0);
    assert_eq!(rec.len(), 7944);
    let u32_at = |at: usize| u32::from_le_bytes(rec[at..at + 4].try_into().unwrap_or([0; 4]));
    let i32_at = |at: usize| i32::from_le_bytes(rec[at..at + 4].try_into().unwrap_or([0; 4]));
    let u16_at = |at: usize| u16::from_le_bytes(rec[at..at + 2].try_into().unwrap_or([0; 2]));
    let f64_at = |at: usize| f64::from_le_bytes(rec[at..at + 8].try_into().unwrap_or([0; 8]));
    assert_eq!((u32_at(0), u32_at(4)), (2100, 7936));
    assert_eq!((i32_at(12), i32_at(16)), (2_970_000, 2_100_000));
    assert_eq!(u16_at(22), 1, "landscape");
    assert_eq!((i32_at(670), i32_at(674)), (-1_485_000, -1_050_000));
    assert_eq!((f64_at(678), f64_at(686)), (4.0, 4.0));
    assert_eq!((u16_at(704), u16_at(706)), (2, 2), "millimetres");
    assert_eq!(f64_at(696), 1.0);
    assert_eq!(
        (i32_at(831), i32_at(835), i32_at(839)),
        (50_000, 50_000, 1_000)
    );
    assert!((0..4).all(|k| f64_at(1019 + 8 * k) == 1.0));
    assert_eq!(f64_at(7360), 12.0);
    assert_eq!(&rec[7416..7418], b"I\0");
    // The same layout in version 22; earlier versions keep theirs.
    assert_eq!(mcfg(Ver(22), 210.0, 297.0).len(), 7944);
    assert_eq!(mcfg(Ver(20), 210.0, 297.0).len(), 8323);
    // The page size reads back from it.
    let doc = Document::new("t", tracedraw_core::geometry::Size::new(297.0, 210.0));
    let bytes = document_to_cdr_version(&doc, 21);
    let (back, _) = crate::open_bytes(&bytes, "t").expect("opens");
    assert!((back.pages[0].size.width - 297.0).abs() < 0.01);
}

#[test]
fn pages_of_large_format_work_keep_their_size() {
    // A 22 m by 1.15 m banner page; such pages used to open as A4.
    let doc = Document::new("w", Size::new(22_000.0, 1_150.0));
    for v in versions() {
        let (back, rep) =
            crate::open_bytes(&document_to_cdr_version(&doc, v), "back").expect("reads back");
        let size = back.pages[0].size;
        assert!(
            (size.width - 22_000.0).abs() < 0.01 && (size.height - 1_150.0).abs() < 0.01,
            "{v}: {size:?} {:?}",
            rep.warnings
        );
        assert!(
            !rep.warnings.iter().any(|w| w.contains("looks wrong")),
            "{v}"
        );
    }
    // Past 100 m a page size is taken for a misread record.
    let doc = Document::new("w", Size::new(150_000.0, 1_000.0));
    let (back, rep) =
        crate::open_bytes(&document_to_cdr_version(&doc, 22), "back").expect("reads back");
    assert!(back.pages[0].size.width < 1_000.0);
    assert!(rep.warnings.iter().any(|w| w.contains("looks wrong")));
}

#[test]
fn chosen_node_types_round_trip_in_the_continuity_bits() {
    use tracedraw_core::nodes::{self, NodeType};
    // Node 1 has aligned handles of equal length (reads as symmetrical) but is
    // chosen as a cusp; node 2 reads as symmetrical and is chosen smooth.
    let mut p = BezPath::new();
    p.move_to((0.0, 0.0));
    p.curve_to((10.0, 10.0), (20.0, 10.0), (30.0, 0.0));
    p.curve_to((40.0, -10.0), (50.0, -10.0), (60.0, 0.0));
    p.curve_to((70.0, 10.0), (80.0, 10.0), (90.0, 0.0));
    let mut doc = Document::new("w", Size::new(200.0, 100.0));
    let mut ids = doc.ids().clone();
    let mut c = Shape::new(
        ids.shape(),
        ShapeKind::Path {
            path: p.clone(),
            closed: false,
        },
    );
    assert_eq!(nodes::node_type(&p, 1), NodeType::Symmetrical);
    c.node_types = [(1, NodeType::Cusp), (2, NodeType::Smooth)].into();
    doc.pages[0].layers[0].shapes = vec![c];
    doc.set_ids(ids);
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        let s = &back.pages[0].layers[0].shapes[0];
        assert!(
            matches!(s.kind, ShapeKind::Path { .. }),
            "{v}: {:?}",
            rep.warnings
        );
        assert_eq!(
            s.node_types,
            [(1, NodeType::Cusp), (2, NodeType::Smooth)].into(),
            "{v}"
        );
    }
    // Without chosen types nothing is stored on reading.
    let mut plain = doc.clone();
    plain.pages[0].layers[0].shapes[0].node_types.clear();
    let (back, _) = crate::open_bytes(&document_to_cdr(&plain), "back").expect("reads back");
    assert!(back.pages[0].layers[0].shapes[0].node_types.is_empty());
}

#[test]
fn version_16_and_later_files_are_a_complete_package() {
    use std::io::Read;
    let mut doc = doc_with(Vec::new());
    doc.title = "Pack & go".into();
    let mut ids = doc.ids().clone();
    let mut r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        },
    );
    r.name = Some("Logo <main>".into());
    doc.pages[0].layers[0].shapes = vec![r];
    doc.set_ids(ids);
    let png = Png {
        data: vec![0x89, b'P', b'N', b'G'],
        width: 256,
        height: 128,
    };
    let previews = Previews {
        thumbnail: Some(png.clone()),
        pages: vec![png],
        ..Default::default()
    };
    for v in [16u16, 17, 21, NEWEST_VERSION] {
        let bytes = document_to_cdr_version_with(&doc, v, &previews);
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("zip");
        // The media type first, stored, as packages of these versions
        // open with it.
        {
            let mut first = z.by_index(0).expect("first member");
            assert_eq!(first.name(), "mimetype", "{v}");
            assert_eq!(first.compression(), zip::CompressionMethod::Stored, "{v}");
            let mut t = String::new();
            first.read_to_string(&mut t).expect("text");
            assert_eq!(t, PACKAGE_MEDIA_TYPE);
        }
        let mut read = |name: &str| -> String {
            let mut t = String::new();
            z.by_name(name)
                .unwrap_or_else(|_| panic!("{v}: {name}"))
                .read_to_string(&mut t)
                .expect("text");
            t
        };
        let container = read("META-INF/container.xml");
        assert!(container.contains(r#"crl:file-kind="main" full-path="content/root.dat""#));
        assert!(container.contains(r#"full-path="previews/thumbnail.png""#));
        assert!(container.contains(r#"crl:image-height="128" crl:image-width="256" crl:index="1" full-path="previews/page1.png""#));
        let meta = read("META-INF/metadata.xml");
        assert!(meta.contains(&format!(
            "<cdr:CoreVersion>{}</cdr:CoreVersion>",
            u32::from(v) * 100
        )));
        assert!(meta.contains("Pack &amp; go"));
        let names = read("META-INF/textinfo.xml");
        assert!(
            names.contains("<rdf:li>Logo &lt;main&gt;</rdf:li>"),
            "{names}"
        );
        assert!(read("color/color.xml").starts_with("<Color>"));
        assert!(read("META-INF/links.xml").contains("<linkInfo />"));
        let mut png_bytes = Vec::new();
        z.by_name("previews/page1.png")
            .expect("page picture")
            .read_to_end(&mut png_bytes)
            .expect("bytes");
        assert_eq!(png_bytes[1..4], *b"PNG");
        // The drawing still reads back.
        let (back, _) = crate::open_bytes(&bytes, "back").expect("reads");
        assert_eq!(back.pages.len(), 1);
    }
    // Without pictures: no preview members, and the container names the
    // root file only.
    let bytes = document_to_cdr_version(&doc, 17);
    let z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("zip");
    assert!(!z.file_names().any(|n| n.starts_with("previews/")));
}

#[test]
fn centred_gradients_are_written_from_their_edge_and_read_back() {
    use tracedraw_core::{Fountain, FountainKind, Stop};
    let red = Color::rgb8(255, 0, 0);
    let yellow = Color::rgb8(255, 255, 0);
    for kind in [
        FountainKind::Radial,
        FountainKind::Square,
        FountainKind::Linear,
    ] {
        let f = Fountain {
            kind,
            stops: vec![
                Stop::new(0.0, red),
                Stop::new(0.3, Color::rgb8(255, 128, 0)),
                Stop::new(1.0, yellow),
            ],
            angle: 0.0,
            offset: tracedraw_core::geometry::Point::ZERO,
            edge_pad: 0.0,
            ..Default::default()
        };
        let mut mirrored = f.clone();
        crate::mirror_centred_stops(&mut mirrored);
        if kind == FountainKind::Linear {
            assert_eq!(mirrored, f);
        } else {
            // Edge first: yellow at 0, the orange stop 0.7 from the edge.
            assert_eq!(mirrored.stops[0].color, yellow);
            assert!((mirrored.stops[1].pos - 0.7).abs() < 1e-9);
            let mut back = mirrored.clone();
            crate::mirror_centred_stops(&mut back);
            for (x, y) in back.stops.iter().zip(&f.stops) {
                assert_eq!(x.color, y.color);
                assert!((x.pos - y.pos).abs() < 1e-9);
            }
        }
    }
}

#[test]
fn every_version_writes_what_files_of_that_version_hold() {
    use std::io::Read;
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        },
    );
    doc.pages[0].layers[0].shapes = vec![r];
    doc.set_ids(ids);
    // A 2 x 1 picture: red then blue.
    let mut pm = tiny_skia::Pixmap::new(2, 1).expect("pixmap");
    pm.pixels_mut()[0] = tiny_skia::PremultipliedColorU8::from_rgba(255, 0, 0, 255).expect("red");
    pm.pixels_mut()[1] = tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 255, 255).expect("blue");
    let png = Png {
        data: pm.encode_png().expect("png"),
        width: 2,
        height: 1,
    };
    let previews = Previews {
        thumbnail: Some(png.clone()),
        pages: vec![png],
        ..Default::default()
    };
    for v in versions() {
        let bytes = document_to_cdr_version_with(&doc, v, &previews);
        let (ids, _, _) = chunk_ids(&bytes);
        let has = |name: &str| ids.iter().any(|i| i.trim() == name);
        if v < 14 {
            // RIFF: the picture and the empty keywords and comment after
            // `vrsn`, an empty `uil` list among the document tables.
            assert_eq!(ids[1].trim(), "vrsn", "{v}: {ids:?}");
            assert_eq!(ids[2].trim(), "DISP", "{v}: {ids:?}");
            assert!(has("INFO") && has("IKEY") && has("ICMT"), "{v}");
            assert!(has("uil"), "{v}: {ids:?}");
            let disp = find_chunk(&bytes, b"DISP").expect("DISP");
            let side = if v >= 13 { 128 } else { 96 };
            assert_eq!(u32::from_le_bytes(disp[0..4].try_into().unwrap()), 8);
            assert_eq!(i32::from_le_bytes(disp[8..12].try_into().unwrap()), side);
            let bpp = u16::from_le_bytes(disp[18..20].try_into().unwrap());
            assert_eq!(bpp, if v >= 13 { 24 } else { 8 }, "{v}");
            assert!(!has("pref"));
            continue;
        }
        assert!(!has("DISP") && !has("uil"), "{v}");
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("zip");
        let names: Vec<String> = z.file_names().map(String::from).collect();
        if v < 16 {
            for n in [
                "content/riffData.cdr",
                "metadata/thumbnails/thumbnail.bmp",
                "metadata/thumbnails/page1.bmp",
                "metadata/metadata.xml",
                "metadata/textinfo.xml",
                "links.xml",
            ] {
                assert!(names.iter().any(|m| m == n), "{v}: {n} in {names:?}");
            }
            let mut bmp = Vec::new();
            z.by_name("metadata/thumbnails/page1.bmp")
                .expect("bmp")
                .read_to_end(&mut bmp)
                .expect("bytes");
            assert_eq!(&bmp[..2], b"BM");
            // Bottom-up, B G R: red first, then blue.
            assert_eq!(&bmp[54..60], &[0, 0, 255, 255, 0, 0]);
            assert!(!has("pref"));
        } else {
            for n in ["color/docPalette.xml", "styles/document.cdss"] {
                assert!(names.iter().any(|m| m == n), "{v}: {n}");
            }
            let mut cdss = String::new();
            z.by_name("styles/document.cdss")
                .expect("styles")
                .read_to_string(&mut cdss)
                .expect("text");
            assert!(cdss.contains(&format!("\"version\": {},", u32::from(v) * 100)));
            for style in ["~graphic~", "~artistic~", "~paragraph~", "~artisticmedia~"] {
                assert!(cdss.contains(style), "{v}: {style}");
            }
            // `pref` after `mcfg`.
            let at = |n: &str| ids.iter().position(|i| i.trim() == n);
            assert!(
                at("pref").is_some() && at("mcfg") < at("pref"),
                "{v}: {ids:?}"
            );
        }
    }
}

/// The payload of the first chunk `id` in a RIFF file.
fn find_chunk(bytes: &[u8], id: &[u8; 4]) -> Option<Vec<u8>> {
    let at = bytes.windows(4).position(|w| w == id)?;
    let len = u32::from_le_bytes(bytes.get(at + 4..at + 8)?.try_into().ok()?) as usize;
    bytes.get(at + 8..at + 8 + len).map(<[u8]>::to_vec)
}

#[test]
fn riff_streams_are_compressed_and_end_with_a_summary() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(10.0, 20.0, 30.0, 50.0),
            radius: 0.0,
            corners: None,
        },
    );
    doc.pages[0].layers[0].shapes = vec![r];
    doc.set_ids(ids);
    for v in versions().into_iter().filter(|v| *v < 16) {
        let bytes = document_to_cdr_version(&doc, v);
        let (_, tree, riff) = chunk_ids(&bytes);
        let top: Vec<String> = tree
            .root
            .children
            .iter()
            .map(|c| {
                c.list_type_str()
                    .unwrap_or_else(|| c.id_str())
                    .trim()
                    .to_string()
            })
            .collect();
        let want: Vec<&str> = if v < 14 {
            vec!["vrsn", "INFO", "cmpr", "cmpr", "sumi"]
        } else {
            vec!["vrsn", "cmpr", "cmpr"]
        };
        // Without a picture there is no `DISP`.
        assert_eq!(top, want, "{v}");
        // The second compressed list holds the document and the pages.
        let second = &tree.root.children[top.len() - if v < 14 { 2 } else { 1 }];
        let inner = second.find(b"cmpr").expect("inflated stream");
        assert!(
            inner.find(b"doc ").is_some() && inner.find(b"page").is_some(),
            "{v}"
        );
        // It reads back with the rectangle.
        let (back, _) = crate::open_bytes(&bytes, "back").expect("reads");
        assert_eq!(back.pages[0].layers[0].shapes.len(), 1, "{v}");
        if v < 14 {
            let sumi = tree.root.find(b"sumi").expect("sumi");
            let d = tree.data(&riff, sumi);
            let len = match v {
                0..=9 => 44,
                10 | 11 => 56,
                _ => 60,
            };
            assert_eq!(d.len(), len, "{v}");
            let word = |i: usize| i32::from_le_bytes(d[i * 4..i * 4 + 4].try_into().unwrap());
            // The rectangle's box from the page centre, in units.
            let size = doc.pages[0].size;
            let (cx, cy) = (size.width / 2.0, size.height / 2.0);
            let u = |mm: f64| (mm * 10_000.0).round() as i32;
            assert_eq!(word(0), u(10.0 - cx), "{v}");
            assert_eq!(word(1), u(50.0 - cy), "{v}");
            assert_eq!(word(2), u(30.0 - cx), "{v}");
            assert_eq!(word(3), u(20.0 - cy), "{v}");
            assert_eq!(word(9), i32::from(v) * 100, "{v}");
        }
    }
}

#[test]
fn a_drawings_cmyk_profile_is_stored_and_named() {
    use std::io::Read;
    let mut doc = doc_with(Vec::new());
    let data =
        tracedraw_core::icc::test_profiles::Builder::new(b"prtr", b"CMYK", b"Lab ", 2).build();
    doc.cmyk_profile = Some(tracedraw_core::DocumentProfile::new(
        "Press & Proof v2",
        data.clone(),
    ));
    let bytes = document_to_cdr_version(&doc, 21);
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("zip");
    let mut color = String::new();
    z.by_name("color/color.xml")
        .expect("colour settings")
        .read_to_string(&mut color)
        .expect("text");
    assert!(
        color.starts_with("<Color><ColorContext><ColorProfile id=\"Cmyk\">Press &amp; Proof v2</ColorProfile><ColorModel>"),
        "{color}"
    );
    let mut stored = Vec::new();
    z.by_name("color/profiles/cmyk/pressproofv2.icc")
        .expect("profile")
        .read_to_end(&mut stored)
        .expect("bytes");
    assert_eq!(stored, data);
    // It comes back with the drawing.
    let (back, _) = crate::open_bytes(&bytes, "back").expect("reads");
    assert_eq!(back.cmyk_profile.map(|p| p.data.len()), Some(data.len()));
}

#[test]
fn txtj_holds_the_three_line_breaking_tables() {
    let t = txtj();
    assert_eq!(t.len(), 528);
    let hex =
        |r: std::ops::Range<usize>| -> String { t[r].iter().map(|b| format!("{b:02x}")).collect() };
    // The table bytes of files of versions 16 to 21.
    assert_eq!(
        hex(0..170),
        concat!(
            "0e000000510000002100250029002c002e003f005d007d00",
            "3f003f003f003f003f003f003f003f003f003f003f003f00",
            "3f003f003f003f003f003f002c002e00b7003a003b003f00",
            "21003f003f003f003f003f003f003f003f00920094002900",
            "3f005d007d003e00bb003f003f003f00b000270022003f00",
            "3f00250089003f003f003f003f003f003f003f003f003f00",
            "3f003f003f003f003f003f003f003f003f003f003f003f00",
            "3f00",
        )
    );
    assert_eq!(
        hex(312..356),
        "14000000240028005b005c007b003f009100930028003f005b007b003c00ab003f003f003f003f0024003f00"
    );
    assert_eq!(hex(420..440), "080000002c002e003f003f003f003f002c002e00");
    // Everything between the tables is zero.
    assert!(t[170..312]
        .iter()
        .chain(&t[356..420])
        .chain(&t[440..])
        .all(|&b| b == 0));
}

#[test]
fn transforms_carry_the_tag_of_their_version() {
    // Files of versions 13 to 17 open each `trfd` record with [1300, 0],
    // files of 18 and later with [1740, 56].
    let mut ids = Document::new("w", Size::new(200.0, 100.0)).ids().clone();
    let doc = doc_with(vec![Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(10.0, 10.0, 40.0, 30.0),
            radius: 0.0,
            corners: None,
        },
    )]);
    for v in 7u16..=22 {
        let bytes = document_to_cdr_version(&doc, v);
        let (_, tree, riff) = chunk_ids(&bytes);
        let mut tags = Vec::new();
        tree.root.walk(&mut |ch, _| {
            if ch.id_str() == "trfd" {
                let d = tree.data(&riff, ch);
                tags.push(d.get(24..32).map(|t| {
                    let w = |i: usize| u32::from_le_bytes([t[i], t[i + 1], t[i + 2], t[i + 3]]);
                    (w(0), w(4))
                }));
            }
        });
        assert!(!tags.is_empty(), "{v}");
        let want = match v {
            18.. => Some((1740, 56)),
            13..=17 => Some((1300, 0)),
            _ => None,
        };
        for t in tags {
            if let Some(w) = want {
                assert_eq!(t, Some(w), "{v}");
            }
        }
    }
}

#[test]
fn curve_nodes_carry_the_marks_of_version_16_files() {
    // A closed square, then an open line: move, three lines, the closing
    // line; move, line.
    let mut c = Buf::default();
    c.u32(7);
    for _ in 0..7 {
        c.i32(0).i32(0);
    }
    c.bytes(&[0x00, 0x40, 0x40, 0x40, 0x48, 0x00, 0x40]);
    let mut coords = c.done();
    mark_nodes(&mut coords);
    assert_eq!(
        &coords[4 + 8 * 7..],
        &[0x0c, 0x44, 0x44, 0x44, 0x48, 0x04, 0x44]
    );
    // Control points keep their byte.
    let mut c = Buf::default();
    c.u32(4);
    for _ in 0..4 {
        c.i32(0).i32(0);
    }
    c.bytes(&[0x00, 0xc0, 0xc0, 0x90]);
    let mut coords = c.done();
    mark_nodes(&mut coords);
    assert_eq!(&coords[4 + 8 * 4..], &[0x04, 0xc0, 0xc0, 0x94]);
}

#[test]
fn version_21_text_records_are_numbered_and_complete() {
    let mut ids = Document::new("w", Size::new(200.0, 100.0)).ids().clone();
    let a = text_shape(
        &mut ids,
        vec![TextSpan::new("Ab", "Arial", 12.0)],
        TextAlign::Left,
    );
    let b = text_shape(
        &mut ids,
        vec![TextSpan::new("Cde", "Arial", 12.0)],
        TextAlign::Left,
    );
    let doc = doc_with(vec![a, b]);
    let bytes = document_to_cdr_version(&doc, 21);
    let (_, tree, riff) = chunk_ids(&bytes);
    let mut records = Vec::new();
    let mut numbers = Vec::new();
    tree.root.walk(&mut |ch, _| {
        if ch.is(b"txsm") {
            records.push(tree.data(&riff, ch).to_vec());
        }
        if ch.is(b"spnd") {
            numbers.push(u32::from_le_bytes(
                tree.data(&riff, ch).try_into().expect("u32"),
            ));
        }
    });
    assert_eq!(records.len(), 2);
    for (t, n) in records.iter().zip(&numbers) {
        // The frame is numbered as its object.
        assert_eq!(&t[45..49], &n.to_le_bytes(), "frame number");
        let s = String::from_utf8_lossy(t);
        assert!(
            s.contains(r#""interLineSpacing":"1000000""#),
            "complete style"
        );
        assert!(s.contains(r#"{"character":{}}"#), "empty run style");
    }
    assert_ne!(numbers[0], numbers[1]);
}

#[test]
fn gradient_options_are_written_as_the_closest_plain_gradient() {
    use tracedraw_core::{Fountain, FountainKind, FountainRepeat};
    let red = Color::rgb8(255, 0, 0);
    let blue = Color::rgb8(0, 0, 255);
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut shape_with = |f: Fountain| {
        let mut s = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Fountain(f);
        s.stroke = None;
        s
    };
    let mut inverted = Fountain::two(FountainKind::Linear, red, blue, 0.0);
    inverted.invert = true;
    let mut mirrored = Fountain::two(FountainKind::Linear, red, blue, 0.0);
    mirrored.repeat = FountainRepeat::Mirror;
    mirrored.repeats = 2;
    let shaped = Fountain::two(FountainKind::Shaped, red, blue, 0.0);
    let plain = Fountain::two(FountainKind::Radial, red, blue, 0.0);
    let shapes = vec![
        shape_with(inverted),
        shape_with(mirrored),
        shape_with(shaped),
        shape_with(plain.clone()),
    ];
    doc.pages[0].layers[0].shapes = shapes;
    doc.set_ids(ids);
    for v in [7u16, 13, 17] {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, _) = crate::open_bytes(&bytes, "back").expect("reads back");
        let fills: Vec<Fountain> = back.pages[0].layers[0]
            .shapes
            .iter()
            .filter_map(|s| match &s.fill {
                Fill::Fountain(f) => Some(f.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(fills.len(), 4, "{v}");
        // Inverted: blue first.
        assert_eq!(fills[0].first_color(), blue, "{v}");
        assert_eq!(fills[0].last_color(), red, "{v}");
        // Mirrored twice: red, blue, red.
        assert_eq!(fills[1].stops.len(), 3, "{v}");
        assert_eq!(fills[1].stops[1].color, blue, "{v}");
        assert!((fills[1].stops[1].pos - 0.5).abs() < 0.01, "{v}");
        assert_eq!(fills[1].last_color(), red, "{v}");
        // Shaped: a square gradient, blue (the inside) at the centre.
        assert_eq!(fills[2].kind, FountainKind::Square, "{v}");
        assert_eq!(fills[2].first_color(), blue, "{v}");
        // A plain gradient comes back as it was written.
        assert_eq!(fills[3].kind, plain.kind, "{v}");
        assert_eq!(fills[3].first_color(), red, "{v}");
        assert!(fills.iter().all(|f| f.runs_once()), "{v}");
    }
}

#[test]
fn records_of_versions_13_to_20_are_the_version_21_one_with_two_blocks() {
    let r21 = mcfg(Ver(21), 500.0, 500.0);
    for v in 13u16..=20 {
        let r = mcfg(Ver(v), 500.0, 500.0);
        assert_eq!(r.len(), 8323, "{v}");
        assert_eq!(
            &r[0..8],
            &[0x14, 0x05, 0, 0, 0x7b, 0x20, 0, 0],
            "{v}: tag and size"
        );
        // The page size and every field up to the first block.
        assert_eq!(&r[8..869], &r21[8..869], "{v}");
        // A scale of 1000 twice, for a metric drawing.
        assert_eq!(&r[869..877], &[0xe8, 3, 0, 0, 0xe8, 3, 0, 0], "{v}");
        assert_eq!(&r[877..890], &r21[869..882], "{v}");
        assert_eq!(&r[1261..], &r21[882..], "{v}");
    }
}

#[test]
fn the_version_12_record_keeps_the_page_and_settings_of_version_13() {
    let r12 = mcfg(Ver(12), 500.0, 500.0);
    let r13 = mcfg(Ver(13), 500.0, 500.0);
    assert_eq!(r12.len(), 8152);
    // No tag: the record opens with 4 zero bytes and the page size.
    assert_eq!(&r12[0..4], &[0, 0, 0, 0]);
    assert_eq!(&r12[4..12], &r13[12..20]);
    // The ruler origin and grid follow, moved by the two added bytes.
    assert_eq!(&r12[19..696], &r13[25..702]);
}

#[test]
fn versions_7_and_8_carry_a_whole_configuration_record() {
    let r8 = mcfg(Ver(8), 297.0, 210.0);
    let r7 = mcfg(Ver(7), 297.0, 210.0);
    assert_eq!((r7.len(), r8.len()), (536, 656));
    let i32_at =
        |r: &[u8], at: usize| i32::from_le_bytes(r[at..at + 4].try_into().unwrap_or([0; 4]));
    let f64_at =
        |r: &[u8], at: usize| f64::from_le_bytes(r[at..at + 8].try_into().unwrap_or([0; 8]));
    // Page size first, then the ruler origin at the bottom-left corner
    // and a grid that is not zero.
    assert_eq!((i32_at(&r8, 0), i32_at(&r8, 4)), (2_970_000, 2_100_000));
    assert_eq!((i32_at(&r8, 24), i32_at(&r8, 28)), (-1_485_000, -1_050_000));
    assert_eq!((f64_at(&r8, 32), f64_at(&r8, 40)), (0.2, 0.2));
    assert_eq!(f64_at(&r8, 56), 1.0);
    assert_eq!((i32_at(&r8, 68), i32_at(&r8, 72)), (11_811_024, 11_811_024));
    // Version 7: the white page colour after the paper, the rest moved.
    assert_eq!(&r7[0..12], &r8[0..12]);
    assert_eq!(&r7[12..24], &[5, 0, 5, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0]);
    assert_eq!(&r7[24..44], &r8[12..32]);
    assert_eq!(&r7[48..480], &r8[32..464]);
    assert_eq!(i32_at(&r7, 480), 1);
    assert_eq!(&r7[484..536], &r8[468..520]);
    // Both versions read their page size back.
    for v in [7, 8] {
        let doc = Document::new("t", Size::new(297.0, 210.0));
        let (back, _) = crate::open_bytes(&document_to_cdr_version(&doc, v), "t").expect("opens");
        assert!((back.pages[0].size.width - 297.0).abs() < 0.01, "{v}");
        assert!((back.pages[0].size.height - 210.0).abs() < 0.01, "{v}");
    }
}

#[test]
fn version_16_text_is_one_byte_a_letter_and_reads_back() {
    let mut ids = Document::new("w", Size::new(200.0, 100.0)).ids().clone();
    let t = text_shape(
        &mut ids,
        vec![TextSpan::new("Kombi 2013 é", "Arial", 12.0)],
        TextAlign::Left,
    );
    let doc = doc_with(vec![t]);
    let bytes = document_to_cdr_version(&doc, 16);
    let (_, tree, riff) = chunk_ids(&bytes);
    let mut record = Vec::new();
    tree.root.walk(&mut |ch, _| {
        if ch.is(b"txsm") {
            record = tree.data(&riff, ch).to_vec();
        }
    });
    // The letters close the record: a u32 count of 12 bytes, the text in
    // Windows-1252 and a zero.
    let tail = &record[record.len() - 17..];
    assert_eq!(&tail[..4], &12u32.to_le_bytes());
    assert_eq!(&tail[4..16], b"Kombi 2013 \xe9");
    let (back, _) = crate::open_bytes(&bytes, "back").expect("reads");
    let text: String = back.pages[0].layers[0]
        .shapes
        .iter()
        .filter_map(|s| match &s.kind {
            ShapeKind::Text { spans, .. } => {
                Some(spans.iter().map(|sp| sp.text.clone()).collect::<String>())
            }
            _ => None,
        })
        .collect();
    assert_eq!(text, "Kombi 2013 é");
}

#[test]
fn graded_transparency_is_written_as_a_gradient_from_16() {
    let mut doc = Document::new("w", Size::new(200.0, 100.0));
    let mut ids = doc.ids().clone();
    let mut r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(10.0, 20.0, 60.0, 50.0),
            radius: 0.0,
            corners: None,
        },
    );
    r.fill = Fill::Solid(Color::rgb8(255, 0, 0));
    r.effects.push(tracedraw_core::live::Effect::Transparency {
        mask: Fill::linear(Color::WHITE, Color::Gray { v: 0.5 }, 0.0),
        merge: tracedraw_core::live::MergeMode::Normal,
        target: 2,
    });
    doc.pages[0].layers[0].shapes = vec![r];
    doc.set_ids(ids);
    // Before version 13 the mean opacity; from 16 the gradient itself.
    let bytes = document_to_cdr_version(&doc, 12);
    let (back, _) = crate::open_bytes(&bytes, "back").expect("reads back");
    let s = &back.pages[0].layers[0].shapes[0];
    assert!((s.opacity - 0.75).abs() < 0.01, "12: {}", s.opacity);
    for v in [16u16, 21] {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, _) = crate::open_bytes(&bytes, "back").expect("reads back");
        let s = &back.pages[0].layers[0].shapes[0];
        let mask = s.effects.iter().find_map(|e| match e {
            tracedraw_core::live::Effect::Transparency {
                mask: Fill::Fountain(f),
                target,
                ..
            } => Some((f.clone(), *target)),
            _ => None,
        });
        let (f, target) = mask.unwrap_or_else(|| panic!("{v}: no mask, opacity {}", s.opacity));
        assert_eq!(target, 2, "{v}");
        let ends: Vec<f32> = f.stops.iter().map(|st| st.color.luminance()).collect();
        assert!(
            (ends[0] - 1.0).abs() < 0.01 && (ends[1] - 0.5).abs() < 0.01,
            "{v}: {ends:?}"
        );
        assert!(
            s.opacity > 0.99,
            "{v}: mask and opacity both applied ({})",
            s.opacity
        );
    }
}

#[test]
fn a_clip_frame_keeps_its_fill_under_and_outline_over_the_contents() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let rect = |ids: &mut tracedraw_core::id::IdSource, x: f64| {
        Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(x, 10.0, x + 30.0, 40.0),
                radius: 0.0,
                corners: None,
            },
        )
    };
    let mut frame = rect(&mut ids, 10.0);
    frame.fill = Fill::Solid(Color::rgb8(0, 120, 0));
    frame.stroke = Some(Stroke::hairline(Color::BLACK));
    let mut inside = rect(&mut ids, 20.0);
    inside.fill = Fill::Solid(Color::rgb8(200, 230, 150));
    inside.stroke = None;
    let pc = Shape::new(
        ids.shape(),
        ShapeKind::ClipFrame {
            frame: Box::new(frame),
            contents: vec![inside],
        },
    );
    doc.pages[0].layers[0].shapes = vec![pc];
    doc.set_ids(ids);
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        let l = &back.pages[0].layers[0];
        let ShapeKind::Group { children } = &l.shapes[0].kind else {
            panic!("{v}: {:?} {:?}", l.shapes[0].kind, rep.warnings);
        };
        let looks: Vec<(bool, bool)> = children
            .iter()
            .map(|c| (!matches!(c.fill, Fill::None), c.stroke.is_some()))
            .collect();
        // Bottom to top: the frame's fill, the contents, its outline.
        assert_eq!(looks, [(true, false), (true, false), (false, true)], "{v}");
        assert_eq!(children[0].fill, Fill::Solid(Color::rgb8(0, 120, 0)), "{v}");
    }
}

#[test]
fn layers_go_out_front_to_back_and_come_back_bottom_to_top() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut top = Layer::new(ids.layer(), "Front");
    let mut r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(10.0, 10.0, 20.0, 20.0),
            radius: 0.0,
            corners: None,
        },
    );
    r.fill = Fill::Solid(Color::rgb8(255, 0, 0));
    top.shapes.push(r.clone());
    r.id = ids.shape();
    doc.pages[0].layers[0].shapes.push(r);
    doc.pages[0].layers.push(top);
    doc.set_ids(ids);
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        let names: Vec<&str> = back.pages[0]
            .layers
            .iter()
            .filter(|l| !l.shapes.is_empty())
            .map(|l| l.name.as_str())
            .collect();
        assert_eq!(names, ["Ink", "Front"], "{v}: {:?}", rep.warnings);
    }
    // In the file the front layer is listed first.
    let (_, tree, riff) = chunk_ids(&document_to_cdr_version(&doc, 12));
    let mut names = Vec::new();
    tree.root.walk(&mut |c, _| {
        if c.list_type_str().as_deref() == Some("layr") {
            let (_, args) = layer_args(&tree, &riff, c);
            if let Some((_, name)) = args.iter().find(|(t, _)| *t == 1000) {
                let text = String::from_utf8_lossy(name).replace('\0', "");
                if text == "Front" || text == "Ink" {
                    names.push(text);
                }
            }
        }
    });
    assert_eq!(names, ["Front", "Ink"]);
}

#[test]
fn fills_not_written_yet_go_out_as_their_mean_colour() {
    use tracedraw_core::style::{Pattern, PatternTile, Texture, TextureKind};
    let tile = {
        let mut pm = tiny_skia::Pixmap::new(2, 1).expect("pixmap");
        pm.pixels_mut()[0] = tiny_skia::ColorU8::from_rgba(200, 0, 0, 255).premultiply();
        pm.pixels_mut()[1] = tiny_skia::ColorU8::from_rgba(0, 0, 100, 255).premultiply();
        pm.encode_png().expect("png")
    };
    let fills = [
        (
            Fill::Texture(Texture {
                kind: TextureKind::Clouds,
                color_a: Color::rgb8(40, 40, 40),
                color_b: Color::rgb8(120, 120, 120),
                scale: 5.0,
                seed: 1,
            }),
            [80, 80, 80],
        ),
        (
            Fill::Pattern(Pattern::TwoColor {
                tile: PatternTile::Checker,
                front: Color::rgb8(0, 0, 0),
                back: Color::rgb8(200, 100, 50),
                size_mm: 5.0,
            }),
            [100, 50, 25],
        ),
        (
            Fill::Pattern(Pattern::Bitmap {
                png: tile.into(),
                width_px: 2,
                height_px: 1,
                size_mm: 5.0,
            }),
            [100, 0, 50],
        ),
    ];
    for (fill, want) in fills {
        let mut ids = tracedraw_core::id::IdSource::default();
        let mut r = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 30.0, 30.0),
                radius: 0.0,
                corners: None,
            },
        );
        r.fill = fill.clone();
        let doc = doc_with(vec![r]);
        for v in versions() {
            let bytes = document_to_cdr_version(&doc, v);
            let (back, _) = crate::open_bytes(&bytes, "back").expect("reads back");
            let got = back.pages[0].layers[0].shapes[0]
                .fill
                .preview_color()
                .expect("fill");
            let rgb = got.to_rgb8();
            let near = rgb
                .iter()
                .zip(want)
                .all(|(a, b)| (*a as i32 - b as i32).abs() <= 3);
            assert!(near, "{v}: {fill:?} gave {rgb:?}, want {want:?}");
        }
    }
}

#[test]
fn a_page_background_colour_round_trips_from_version_21() {
    let mut doc = doc_with(Vec::new());
    doc.pages[0].background = Some(Fill::Solid(Color::rgb8(45, 78, 92)));
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        let bg = back.pages[0].background.clone();
        if v >= 21 {
            assert_eq!(
                bg,
                Some(Fill::Solid(Color::rgb8(45, 78, 92))),
                "{v}: {:?}",
                rep.warnings
            );
        } else {
            assert_eq!(bg, None, "{v}");
        }
    }
}

#[test]
fn hidden_objects_are_kept_hidden_from_version_21() {
    let mut ids = tracedraw_core::id::IdSource::default();
    let mut r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(10.0, 10.0, 30.0, 30.0),
            radius: 0.0,
            corners: None,
        },
    );
    r.fill = Fill::Solid(Color::rgb8(255, 255, 255));
    r.visible = false;
    let doc = doc_with(vec![r]);
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        let shapes = &back.pages[0].layers[0].shapes;
        if v >= 21 {
            assert_eq!(shapes.len(), 1, "{v}: {:?}", rep.warnings);
            assert!(!shapes[0].visible, "{v}");
        } else {
            assert!(shapes.is_empty(), "{v}");
        }
    }
}

#[test]
fn versions_23_to_27_carry_their_own_letters() {
    let doc = doc_with(Vec::new());
    for (v, letter) in [
        (22, b'N'),
        (23, b'O'),
        (24, b'P'),
        (25, b'R'),
        (26, b'T'),
        (27, b'U'),
    ] {
        let bytes = document_to_cdr_version(&doc, v);
        let c = crate::container::detect(&bytes).expect("container");
        assert_eq!(c.version.0, v);
        let riff = crate::container::riff_stream(&bytes, &c).expect("riff");
        assert_eq!(&riff[8..12], &[b'C', b'D', b'R', letter], "{v}");
        let (_, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        assert_eq!(rep.version.map(|x| x.0), Some(v));
        assert!(
            rep.warnings.iter().all(|w| !w.contains("vrsn")),
            "{v}: {:?}",
            rep.warnings
        );
    }
}

#[test]
fn bitmap_objects_carry_their_pixel_size() {
    let png = {
        let mut pm = tiny_skia::Pixmap::new(3, 2).expect("pixmap");
        pm.fill(tiny_skia::Color::from_rgba8(10, 20, 30, 255));
        pm.encode_png().expect("png")
    };
    let mut ids = tracedraw_core::id::IdSource::default();
    let mut bm = Shape::new(
        ids.shape(),
        ShapeKind::Bitmap {
            rect: Rect::new(0.0, 0.0, 30.0, 20.0),
            width_px: 3,
            height_px: 2,
            png: png.into(),
            fx: None,
        },
    );
    bm.fill = Fill::None;
    bm.stroke = None;
    let doc = doc_with(vec![bm]);
    for v in versions() {
        let (_, tree, riff) = chunk_ids(&document_to_cdr_version(&doc, v));
        let mut coords = None;
        tree.root.walk(&mut |c, _| {
            if c.is(b"loda") {
                let d = tree.data(&riff, c);
                if d.get(16..20) == Some(&5u32.to_le_bytes()[..]) {
                    coords = loda_args(d)
                        .into_iter()
                        .find(|(t, _)| *t == 30)
                        .map(|(_, b)| b);
                }
            }
        });
        let b = coords.expect("a bitmap object");
        let i = |at: usize| i32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"));
        // Top-left, then bottom-right; the image box again.
        assert_eq!([i(0), i(4), i(8), i(12)], [0, 200_000, 300_000, 0], "{v}");
        assert_eq!(
            [i(16), i(20), i(24), i(28)],
            [0, 200_000, 300_000, 0],
            "{v}"
        );
        assert_eq!([i(32), i(36), i(40)], [24 << 16 | 2, 3, 2], "{v}");
        assert_eq!(i(60), -1, "{v}");
    }
}

#[test]
fn version_15_objects_carry_the_arguments_its_files_have() {
    let mut ids = tracedraw_core::id::IdSource::default();
    let mut r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(10.0, 10.0, 30.0, 30.0),
            radius: 0.0,
            corners: None,
        },
    );
    r.fill = Fill::Solid(Color::BLACK);
    r.opacity = 0.5;
    let t = text_shape(
        &mut ids,
        vec![TextSpan::new("Hi", "Arial", 12.0)],
        TextAlign::Left,
    );
    let doc = doc_with(vec![r, t]);
    for v in versions() {
        let (_, tree, riff) = chunk_ids(&document_to_cdr_version(&doc, v));
        let mut seen: Vec<(u32, Vec<u32>)> = Vec::new();
        tree.root.walk(&mut |c, _| {
            if c.is(b"loda") {
                let d = tree.data(&riff, c);
                let kind = d
                    .get(16..20)
                    .map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap_or([0; 4])));
                seen.push((kind, loda_args(d).iter().map(|(t, _)| *t).collect()));
            }
        });
        let has = |kind: u32, arg: u32| seen.iter().any(|(k, a)| *k == kind && a.contains(&arg));
        // No bare transparency value where these versions keep a record.
        assert_eq!(has(3, 8000) || has(1, 8000), v < 13, "{v}");
        // Text: 16015 from version 15.
        assert_eq!(has(4, 16015), v >= 15, "{v}");
    }
}

#[test]
fn artistic_text_records_before_16_open_as_files_have_them() {
    let mut ids = tracedraw_core::id::IdSource::default();
    let t = text_shape(
        &mut ids,
        vec![TextSpan::new("Hi", "Arial", 12.0)],
        TextAlign::Left,
    );
    let doc = doc_with(vec![t]);
    for (v, layout) in [
        (7u16, 2u32),
        (8, 3),
        (10, 5),
        (11, 5),
        (12, 6),
        (13, 8),
        (14, 9),
        (15, 11),
    ] {
        let (_, tree, riff) = chunk_ids(&document_to_cdr_version(&doc, v));
        let mut rec = None;
        tree.root.walk(&mut |c, _| {
            if c.is(b"txsm") && rec.is_none() {
                rec = Some(tree.data(&riff, c).to_vec());
            }
        });
        let d = rec.expect("a text record");
        let u = |at: usize| u32::from_le_bytes(d[at..at + 4].try_into().expect("4 bytes"));
        assert_eq!(
            (0..9).map(|i| u(i * 4)).collect::<Vec<_>>(),
            [u32::MAX, u32::MAX, 0, u32::MAX, 8, layout, 0, 3, 0],
            "{v}"
        );
        // The frame's matrix is the identity, not zeros.
        let at = 36 + usize::from(v >= 15) + if v < 8 { 4 } else { 0 } + 8;
        let f = |i: usize| f64::from_le_bytes(d[at + i * 8..at + i * 8 + 8].try_into().expect("8"));
        assert_eq!((f(0), f(4)), (1.0, 1.0), "{v}");
    }
}

#[test]
fn text_from_version_12_goes_out_one_byte_a_letter_with_wide_letters_marked() {
    let mut ids = tracedraw_core::id::IdSource::default();
    let t = text_shape(
        &mut ids,
        vec![TextSpan::new("Steć", "Arial", 12.0)],
        TextAlign::Left,
    );
    let doc = doc_with(vec![t]);
    for v in 12u16..=15 {
        let bytes = document_to_cdr_version(&doc, v);
        let (_, tree, riff) = chunk_ids(&bytes);
        let mut rec = None;
        tree.root.walk(&mut |c, _| {
            if c.is(b"txsm") && rec.is_none() {
                rec = Some(tree.data(&riff, c).to_vec());
            }
        });
        let d = rec.expect("a text record");
        // The letters end the record, before the path flag: three single
        // bytes and the UTF-16 unit of the letter Windows-1252 lacks.
        let tail = &d[d.len() - 1 - 5 - 4..d.len() - 1];
        assert_eq!(tail, &[5, 0, 0, 0, b'S', b't', b'e', 0x07, 0x01], "{v}");
        let (back, _) = crate::open_bytes(&bytes, "back").expect("reads back");
        let ShapeKind::Text { spans, .. } = &back.pages[0].layers[0].shapes[0].kind else {
            panic!("text");
        };
        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "Steć", "{v}");
    }
}

/// From version 16 a letter Windows-1252 lacks (Cyrillic here) goes out
/// as a UTF-16 unit of two bytes with bit 0 of its record's flags set;
/// the others stay one byte. The text reads back as written (#368).
#[test]
fn text_from_version_16_marks_the_letters_windows_1252_lacks() {
    let mut ids = tracedraw_core::id::IdSource::default();
    let t = text_shape(
        &mut ids,
        vec![TextSpan::new("Пример é", "Arial", 12.0)],
        TextAlign::Left,
    );
    let doc = doc_with(vec![t]);
    for v in 16u16..=*crate::write::versions().end() {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, _) = crate::open_bytes(&bytes, "back").expect("reads back");
        let text: String = back
            .pages
            .iter()
            .flat_map(|p| &p.layers)
            .flat_map(|l| &l.shapes)
            .filter_map(|s| match &s.kind {
                ShapeKind::Text { spans, .. } => {
                    Some(spans.iter().map(|sp| sp.text.as_str()).collect::<String>())
                }
                _ => None,
            })
            .collect();
        assert_eq!(text, "Пример é", "{v}");
        // The letters as stored: six UTF-16 units, then a space and "é"
        // in one byte each (Windows-1252 0x20 and 0xE9).
        let mut want: Vec<u8> = "Пример"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        want.extend_from_slice(&[0x20, 0xE9]);
        let (_, tree, riff) = chunk_ids(&bytes);
        let mut recs = Vec::new();
        tree.root.walk(&mut |c, _| {
            if c.is(b"txsm") {
                recs.push(tree.data(&riff, c).to_vec());
            }
        });
        assert!(!recs.is_empty(), "{v}: a text record");
        let found = recs
            .iter()
            .any(|d| d.windows(want.len()).any(|w| w == want.as_slice()));
        assert!(found, "{v}: the letters are not stored as expected");
        // No UTF-8 for them.
        let utf8 = "Пример".as_bytes();
        assert!(
            !recs
                .iter()
                .any(|d| d.windows(utf8.len()).any(|w| w == utf8)),
            "{v}"
        );
    }
}

/// A text record holding its letters as unmarked UTF-8, as TraceDraw
/// saved them before #368, reads back as the text it was.
#[test]
fn unmarked_utf8_letters_of_older_saves_read_back() {
    let mut ids = tracedraw_core::id::IdSource::default();
    let t = text_shape(
        &mut ids,
        vec![TextSpan::new("Пример", "Arial", 12.0)],
        TextAlign::Left,
    );
    let doc = doc_with(vec![t]);
    let v = 17u16;
    let bytes = document_to_cdr_version(&doc, v);
    let (_, tree, riff) = chunk_ids(&bytes);
    let mut rec = None;
    tree.root.walk(&mut |c, _| {
        if c.is(b"txsm") && rec.is_none() {
            rec = Some(tree.data(&riff, c).to_vec());
        }
    });
    let d = rec.expect("a text record");
    // Rebuild the end of the record as the old writer had it: the six
    // records with no flag, then the UTF-8 bytes.
    let units: Vec<u8> = "Пример"
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    let at = d
        .windows(units.len())
        .position(|w| w == units.as_slice())
        .expect("the letters");
    let mut old = d[..at - 4].to_vec();
    let recs_at = old.len() - 6 * 8;
    for k in 0..6 {
        old[recs_at + k * 8] = 0;
        old[recs_at + k * 8 + 1] = 0;
    }
    let utf8 = "Пример".as_bytes();
    old.extend_from_slice(&(utf8.len() as u32).to_le_bytes());
    old.extend_from_slice(utf8);
    old.extend_from_slice(&d[at + units.len()..]);
    let mut warnings = Vec::new();
    let txsm = crate::text::read_txsm(&old, v, &mut |w| warnings.push(w)).expect("read");
    let text: String = txsm
        .paragraphs
        .iter()
        .flat_map(|p| &p.runs)
        .map(|r| r.text.as_str())
        .collect();
    assert_eq!(text, "Пример", "{warnings:?}");
}

/// The outline's mitre limit goes out in the object's style string from
/// version 16 and reads back from there (#384).
#[test]
fn the_mitre_limit_goes_out_and_reads_back_from_version_16() {
    let mut ids = tracedraw_core::id::IdSource::default();
    let mut r = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 30.0, 20.0),
            radius: 0.0,
            corners: None,
        },
    );
    let mut st = Stroke::new(Color::BLACK, 2.0);
    st.miter_limit = 29.0;
    r.stroke = Some(st);
    let doc = doc_with(vec![r]);
    for v in 16u16..=*crate::write::versions().end() {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, _) = crate::open_bytes(&bytes, "back").expect("reads back");
        let s = back
            .pages
            .iter()
            .flat_map(|p| &p.layers)
            .flat_map(|l| &l.shapes)
            .find_map(|s| s.stroke.clone())
            .expect("an outline");
        assert!(
            (s.miter_limit - 29.0).abs() < 1e-6,
            "{v}: {}",
            s.miter_limit
        );
    }
}

#[test]
fn before_16_text_frames_take_their_objects_numbers_and_curves_end_padded() {
    let mut ids = tracedraw_core::id::IdSource::default();
    let a = text_shape(
        &mut ids,
        vec![TextSpan::new("A", "Arial", 12.0)],
        TextAlign::Left,
    );
    let b = text_shape(
        &mut ids,
        vec![TextSpan::new("B", "Arial", 12.0)],
        TextAlign::Left,
    );
    let mut c = Shape::new(
        ids.shape(),
        ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        },
    );
    c.fill = Fill::Solid(Color::BLACK);
    let doc = doc_with(vec![a, b, c]);
    for v in [12u16, 13, 14, 15, 16, 21] {
        let (_, tree, riff) = chunk_ids(&document_to_cdr_version(&doc, v));
        let mut pairs = Vec::new();
        let mut curve_tail = None;
        tree.root.walk(&mut |ch, _| {
            if ch.list_type_str().as_deref() == Some("obj ") {
                let spnd = ch.find(b"spnd").map(|x| tree.data(&riff, x).to_vec());
                let mut frame = None;
                ch.walk(&mut |x, _| {
                    if x.is(b"txsm") {
                        let d = tree.data(&riff, x);
                        let at = 36 + usize::from(v >= 15) + 4;
                        frame = d.get(at..at + 4).map(|b| b.to_vec());
                    }
                    if x.is(b"loda") {
                        let d = tree.data(&riff, x);
                        if d.get(16..20) == Some(&3u32.to_le_bytes()[..]) {
                            if let Some((_, b)) = loda_args(d).into_iter().find(|(t, _)| *t == 30) {
                                let n = u32::from_le_bytes(b[0..4].try_into().unwrap_or([0; 4]))
                                    as usize;
                                curve_tail = Some(b.len() - (4 + 9 * n));
                            }
                        }
                    }
                });
                if let (Some(s), Some(f)) = (spnd, frame) {
                    pairs.push((s, f));
                }
            }
        });
        if v < 16 {
            assert_eq!(pairs.len(), 2, "{v}");
            for (s, f) in &pairs {
                assert_eq!(s, f, "{v}: frame number is the object's spnd");
            }
        }
        let want = if v < 16 { 8 } else { 0 };
        if v >= 15 {
            assert_eq!(curve_tail, Some(want), "{v}");
        }
    }
}

#[test]
fn before_16_text_takes_its_look_from_a_style_of_its_own() {
    let mut ids = tracedraw_core::id::IdSource::default();
    let mut big = TextSpan::new("TRAVEL", "Arial", 40.0);
    big.bold = true;
    let small = TextSpan::new("note", "Times New Roman", 9.0);
    let a = text_shape(&mut ids, vec![big], TextAlign::Center);
    let b = text_shape(&mut ids, vec![small], TextAlign::Left);
    let doc = doc_with(vec![a, b]);
    for v in 7u16..=15 {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        let looks: Vec<(String, f64, bool, TextAlign)> = back.pages[0].layers[0]
            .shapes
            .iter()
            .filter_map(|s| match &s.kind {
                ShapeKind::Text { spans, align, .. } => spans.first().map(|sp| {
                    (
                        sp.font_family.clone(),
                        (sp.size_pt * 10.0).round() / 10.0,
                        sp.bold,
                        *align,
                    )
                }),
                _ => None,
            })
            .collect();
        assert!(
            looks.contains(&("Arial".into(), 40.0, true, TextAlign::Center)),
            "{v}: {looks:?} {:?}",
            rep.warnings
        );
        assert!(
            looks.contains(&("Times New Roman".into(), 9.0, false, TextAlign::Left)),
            "{v}: {looks:?}"
        );
    }
}

#[test]
fn the_font_table_lists_every_face_the_text_styles_use() {
    let mut ids = tracedraw_core::id::IdSource::default();
    let mut bold = TextSpan::new("Heading", "Arial", 20.0);
    bold.bold = true;
    let plain = TextSpan::new("body", "Arial", 10.0);
    let a = text_shape(&mut ids, vec![bold], TextAlign::Left);
    let b = text_shape(&mut ids, vec![plain], TextAlign::Left);
    let doc = doc_with(vec![a, b]);
    for v in 7u16..=15 {
        let bytes = document_to_cdr_version(&doc, v);
        // Every font record: its id and the face flags after the charset.
        let (_, tree, riff) = chunk_ids(&bytes);
        let mut faces = Vec::new();
        tree.root.walk(&mut |ch, _| {
            if ch.id_str() == "font" {
                let d = tree.data(&riff, ch);
                let id = u16::from_le_bytes([d[0], d[1]]);
                faces.push((id, u32::from_le_bytes(d[4..8].try_into().expect("u32"))));
            }
        });
        let arial: Vec<u32> = faces
            .iter()
            .filter(|(id, _)| faces.iter().any(|(j, f)| j == id && *f == 0x1000))
            .map(|(_, f)| *f)
            .collect();
        assert!(
            arial.contains(&0x40) && arial.contains(&0x1000),
            "{v}: {faces:x?}"
        );
    }
}

#[test]
fn the_package_lists_the_text_faces_and_embeds_those_allowed() {
    const FONT: &[u8] =
        include_bytes!("../../../../apps/tracedraw/assets/fonts/Selawik-Regular.ttf");
    let faces = vec![
        FontFace {
            family: "Selawik".into(),
            bold: false,
            italic: false,
            data: Some(std::sync::Arc::new(FONT.to_vec())),
        },
        FontFace {
            family: "Missing Sans".into(),
            bold: true,
            italic: false,
            data: None,
        },
    ];
    assert_eq!(embedded_size(&faces), FONT.len() as u64);
    let doc = doc_with(Vec::new());
    for embed in [true, false] {
        let previews = Previews {
            fonts: faces.clone(),
            embed_fonts: embed,
            ..Default::default()
        };
        let bytes = document_to_cdr_version_with(&doc, NEWEST_VERSION, &previews);
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("zip");
        let mut table = Vec::new();
        std::io::Read::read_to_end(
            &mut z.by_name("font/fontTable.dat").expect("table"),
            &mut table,
        )
        .expect("read");
        let u32_at =
            |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().expect("u32"));
        // Size after the first 8 bytes, two entries; the first entry's
        // size counts what follows it, and its family follows the PANOSE.
        assert_eq!(u32_at(&table, 4) as usize, table.len() - 8);
        assert_eq!(u32_at(&table, 8), 2);
        assert_eq!(u32_at(&table, 24), 0x40);
        let family_len = u32_at(&table, 38) as usize;
        let family: String = char::decode_utf16(
            table[42..42 + 2 * (family_len - 1)]
                .chunks(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]])),
        )
        .map(|c| c.unwrap_or('?'))
        .collect();
        assert_eq!(family, "Selawik");
        let embedded = z.file_names().any(|n| n == "embed/embedding0");
        assert_eq!(embedded, embed);
        assert!(!z.file_names().any(|n| n == "embed/embedding1"));
        let (back, rep) = crate::open_bytes(&bytes, "fonts").expect("reads back");
        if embed {
            let mut m = Vec::new();
            std::io::Read::read_to_end(&mut z.by_name("embed/embedding0").expect("member"), &mut m)
                .expect("read");
            assert_eq!(&m[..8], &[1, 0, 0, 0, 3, 8, 0, 0]);
            assert_eq!(back.fonts.len(), 1, "{:?}", rep.warnings);
            assert_eq!(back.fonts[0].family, "Selawik");
            assert_eq!(back.fonts[0].data.as_slice(), FONT);
        } else {
            assert!(back.fonts.is_empty());
        }
    }
    // Files before version 16 keep fonts in their own tables.
    let previews = Previews {
        fonts: faces,
        embed_fonts: true,
        ..Default::default()
    };
    let bytes = document_to_cdr_version_with(&doc, 15, &previews);
    let z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("zip");
    assert!(!z
        .file_names()
        .any(|n| n.starts_with("font/") || n.starts_with("embed/")));
}

/// Embedded fonts are written as files that embed them have them (#352):
/// each font table entry carries its family's font id from the drawing
/// (shared by the faces of a family), entries and embeddings follow
/// those numbers, and the EOT header has charset 0, family and style
/// names ending in a NUL, the empty root string's checksum and code page
/// 1252. The faces read back from the file.
#[test]
fn embedded_fonts_carry_the_drawings_font_ids() {
    const REGULAR: &[u8] =
        include_bytes!("../../../../apps/tracedraw/assets/fonts/Selawik-Regular.ttf");
    const BOLD: &[u8] = include_bytes!("../../../../apps/tracedraw/assets/fonts/Selawik-Bold.ttf");
    let mut ids = tracedraw_core::id::IdSource::default();
    let missing = TextSpan::new("first", "Missing Sans", 12.0);
    let plain = TextSpan::new("plain", "Selawik", 12.0);
    let mut bold = TextSpan::new("bold", "Selawik", 12.0);
    bold.bold = true;
    let doc = doc_with(vec![
        text_shape(&mut ids, vec![missing], TextAlign::Left),
        text_shape(&mut ids, vec![plain, bold], TextAlign::Left),
    ]);
    let face = |family: &str, bold: bool, data: Option<&[u8]>| FontFace {
        family: family.into(),
        bold,
        italic: false,
        data: data.map(|d| std::sync::Arc::new(d.to_vec())),
    };
    // The faces in another order than the drawing names the families.
    let previews = Previews {
        fonts: vec![
            face("Selawik", false, Some(REGULAR)),
            face("Selawik", true, Some(BOLD)),
            face("Missing Sans", false, None),
        ],
        embed_fonts: true,
        ..Default::default()
    };
    let u16_at = |b: &[u8], at: usize| u16::from_le_bytes([b[at], b[at + 1]]);
    let u32_at = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().expect("u32"));
    for v in 16..=NEWEST_VERSION {
        let bytes = document_to_cdr_version_with(&doc, v, &previews);
        // The drawing's font ids by family, from its `font` records.
        let (_, tree, riff) = chunk_ids(&bytes);
        let mut font_ids: Vec<(u16, String)> = Vec::new();
        tree.root.walk(&mut |ch, _| {
            if ch.id_str() == "font" {
                let d = tree.data(&riff, ch);
                let name: Vec<u16> = d[18..]
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .take_while(|&u| u != 0)
                    .collect();
                font_ids.push((u16_at(d, 0), String::from_utf16_lossy(&name)));
            }
        });
        let id_of = |family: &str| {
            font_ids
                .iter()
                .find(|(_, f)| f == family)
                .map(|(id, _)| *id)
                .unwrap_or_else(|| panic!("{v}: {family} in {font_ids:?}"))
        };
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("zip");
        let read = |z: &mut zip::ZipArchive<std::io::Cursor<&Vec<u8>>>, name: &str| {
            let mut m = Vec::new();
            std::io::Read::read_to_end(&mut z.by_name(name).expect(name), &mut m).expect("read");
            m
        };
        let table = read(&mut z, "font/fontTable.dat");
        // Each entry: u32 0, u32 size, then the font number.
        let mut numbers = Vec::new();
        let mut at = 12;
        for _ in 0..u32_at(&table, 8) {
            numbers.push(u16_at(&table, at + 8));
            at += 8 + u32_at(&table, at + 4) as usize;
        }
        assert_eq!(at, table.len(), "{v}");
        let (missing, selawik) = (id_of("Missing Sans"), id_of("Selawik"));
        assert_ne!(missing, selawik, "{v}");
        let mut expected = vec![selawik, selawik, missing];
        expected.sort_unstable();
        assert_eq!(numbers, expected, "{v}: {font_ids:?}");
        assert!(!z.file_names().any(|n| n == "embed/embedding2"), "{v}");
        for (n, (font, style)) in [(REGULAR, "Regular"), (BOLD, "Bold")].iter().enumerate() {
            let m = read(&mut z, &format!("embed/embedding{n}"));
            let eot = &m[2705..];
            assert_eq!(u32_at(eot, 0) as usize, eot.len(), "{v}");
            assert_eq!(eot[26], 0, "{v}: charset");
            let family_len = u16_at(eot, 82) as usize;
            assert_eq!(&eot[84..84 + family_len], &utf16z("Selawik"), "{v}");
            let at = 84 + family_len + 2;
            let style_len = u16_at(eot, at) as usize;
            assert_eq!(&eot[at + 2..at + 2 + style_len], &utf16z(style), "{v}");
            // The 2.2 fields after the full name, before the font data.
            let tail = &eot[eot.len() - font.len() - 24..eot.len() - font.len()];
            assert_eq!(u32_at(tail, 4), 0x5047_5342, "{v}: root string checksum");
            assert_eq!(u32_at(tail, 8), 1252, "{v}: EUDC code page");
        }
        let (back, rep) = crate::open_bytes(&bytes, "fonts").expect("reads back");
        assert_eq!(back.fonts.len(), 2, "{v}: {:?}", rep.warnings);
        assert_eq!(back.fonts[0].data.as_slice(), REGULAR, "{v}");
        assert_eq!(back.fonts[1].data.as_slice(), BOLD, "{v}");
        assert!(back.fonts.iter().all(|f| f.family == "Selawik"), "{v}");
    }
}

/// A family the drawing does not name gets a font number of its own
/// after the drawing's ids, shared by its faces.
#[test]
fn faces_of_families_the_drawing_does_not_name_get_new_numbers() {
    let face = |family: &str, bold: bool| FontFace {
        family: family.into(),
        bold,
        italic: false,
        data: None,
    };
    let faces = [
        face("Other", false),
        face("arial", true),
        face("Other", true),
    ];
    let members = fonts::members(&faces, true, &[(4, "Arial".into())]);
    assert_eq!(members.len(), 1, "nothing to embed");
    let table = &members[0].1;
    let mut numbers = Vec::new();
    let mut at = 12;
    while at < table.len() {
        numbers.push(u16::from_le_bytes([table[at + 8], table[at + 9]]));
        at += 8 + u32::from_le_bytes(table[at + 4..at + 8].try_into().expect("u32")) as usize;
    }
    assert_eq!(numbers, [4, 5, 5]);
}

/// UTF-16LE with a closing NUL.
fn utf16z(s: &str) -> Vec<u8> {
    s.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(|u| u.to_le_bytes())
        .collect()
}

/// The style table of the bidirectional version 8 layout (with the set11
/// table, as later versions write it) reads as version 8 too.
#[test]
fn a_version_8_style_table_with_set11_is_read() {
    let uses = StyleUses {
        text_fill: 1,
        graphic_fill: 2,
        outline: 3,
        font: 1,
        formats: Vec::new(),
    };
    let d = stlt(Ver(9), uses);
    let nine = crate::text::read_stlt(&d, 9, &mut |w| panic!("{w}"));
    assert!(!nine.is_empty());
    let mut notes = Vec::new();
    let eight = crate::text::read_stlt(&d, 8, &mut |w| notes.push(w));
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(eight.len(), nine.len());
}

/// A face named by its family and weight ("Titillium Web SemiBold") is
/// written as files name it, the family at the weight's style code
/// ("Titillium Web" at 1024), and reads back as it was in every version
/// (#624, #892).
#[test]
fn faces_named_with_their_weight_round_trip() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let semi = TextSpan::new("Semi", "Titillium Web SemiBold", 18.0);
    let black = TextSpan::new("Black", "Lato Black", 18.0);
    let shapes = vec![
        text_shape(&mut ids, vec![semi], TextAlign::Left),
        text_shape(&mut ids, vec![black], TextAlign::Left),
    ];
    doc.pages[0].layers[0].shapes = shapes;
    doc.set_ids(ids);
    // Written as the family at the weight's code, as the version 19
    // file of #624 has it; bold text and plain families keep theirs.
    let mut w = Writer::new(Ver(NEWEST_VERSION), &[], "t");
    let mut bold_semi = TextSpan::new("x", "Titillium Web SemiBold", 1.0);
    bold_semi.bold = true;
    let plain = TextSpan::new("x", "Titillium Web", 1.0);
    let semi_italic = {
        let mut sp = TextSpan::new("x", "Titillium Web SemiBold", 1.0);
        sp.italic = true;
        sp
    };
    let file = w
        .file_spans(&[
            TextSpan::new("x", "Titillium Web SemiBold", 1.0),
            bold_semi.clone(),
            plain.clone(),
            semi_italic,
        ])
        .expect("renamed");
    let json = character_json(&file[0], None);
    assert!(
        json.contains(r#""font":"Titillium Web""#) && json.contains(r#""weight":"1024""#),
        "{json}"
    );
    assert_eq!(file[1], bold_semi);
    assert_eq!(file[2], plain);
    assert_eq!(style_flags(&file[3]), 2048, "the italic of 600");
    assert_eq!(file[3].font_family, "Titillium Web");
    assert!(w.file_spans(&[plain, bold_semi]).is_none());
    for v in versions() {
        // The family's font records: one at 0x40, one at the 600 face,
        // as the version 19 file has them.
        let bytes = document_to_cdr_version(&doc, v);
        let (_, tree, riff) = chunk_ids(&bytes);
        let mut faces = Vec::new();
        tree.root.walk(&mut |ch, _| {
            if ch.id_str() == "font" {
                let d = tree.data(&riff, ch);
                let flags = u32::from_le_bytes(d[4..8].try_into().expect("u32"));
                let name: String = if v >= 12 {
                    char::decode_utf16(
                        d[18..80]
                            .chunks(2)
                            .map(|c| u16::from_le_bytes([c[0], c[1]]))
                            .take_while(|&u| u != 0),
                    )
                    .map(|c| c.unwrap_or('?'))
                    .collect()
                } else {
                    d[18..]
                        .iter()
                        .take_while(|&&b| b != 0)
                        .map(|&b| b as char)
                        .collect()
                };
                faces.push((name, flags));
            }
        });
        assert!(
            faces.contains(&("Titillium Web".into(), 0x400))
                && faces.contains(&("Lato".into(), 0x1_0000))
                && !faces.iter().any(|(n, _)| n.contains("SemiBold")),
            "{v}: {faces:x?}"
        );
    }
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 2, "{v}: {:?}", rep.warnings);
        for (s, family) in read.iter().zip(["Titillium Web SemiBold", "Lato Black"]) {
            match &s.kind {
                ShapeKind::Text { spans, .. } => {
                    assert_eq!(spans[0].font_family, family, "{v}");
                    assert!(!spans[0].bold, "{v}");
                }
                k => panic!("{v}: {k:?}"),
            }
        }
    }
}

/// The weight axis of a variable font set on the text is written as the
/// style code of its weight, so the text reopens at that weight in every
/// version (#1013): 600 is 1024 and reads back as the family's SemiBold
/// face (the variable font drawn at 600), 700 as bold, 400 as regular.
#[test]
fn variable_font_weight_axis_round_trips() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let at = |w: Option<f64>, bold: bool, italic: bool| {
        let mut sp = TextSpan::new("Weight", "MiSans VF", 18.0);
        sp.bold = bold;
        sp.italic = italic;
        sp.set_variation("wght", w);
        sp
    };
    // The codes: one bit per 100 of weight, the italic the bit above;
    // a weight between steps rounds, one out of range is clamped.
    assert_eq!(weight_code(&at(Some(600.0), false, false)), 1024);
    assert_eq!(weight_code(&at(Some(600.0), false, true)), 2048);
    assert_eq!(weight_code(&at(Some(640.0), true, false)), 1024);
    assert_eq!(weight_code(&at(Some(100.0), false, false)), 1);
    assert_eq!(weight_code(&at(Some(1000.0), false, true)), 0x2_0000);
    assert_eq!(weight_code(&at(Some(f64::NAN), false, false)), 64);
    assert_eq!(weight_code(&at(None, false, false)), 64);
    assert_eq!(weight_code(&at(None, true, true)), 0x2000);
    assert_eq!(style_flags(&at(Some(500.0), false, false)), 256);
    let json = character_json(&at(Some(600.0), false, true), None);
    assert!(
        json.contains(r#""weight":"1024""#) && json.contains(r#""italic":"1""#),
        "{json}"
    );
    // Set on bold text, the axis is what draws it.
    let json = character_json(&at(Some(400.0), true, false), None);
    assert!(json.contains(r#""weight":"64""#), "{json}");

    let spans = [
        (at(Some(600.0), false, false), "MiSans VF SemiBold", false),
        (at(Some(700.0), false, false), "MiSans VF", true),
        (at(Some(300.0), true, false), "MiSans VF Light", false),
        (at(None, false, false), "MiSans VF", false),
    ];
    doc.pages[0].layers[0].shapes = spans
        .iter()
        .map(|(sp, _, _)| text_shape(&mut ids, vec![sp.clone()], TextAlign::Left))
        .collect();
    doc.set_ids(ids);
    for v in versions() {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, rep) = crate::open_bytes(&bytes, "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), spans.len(), "{v}: {:?}", rep.warnings);
        for (s, (_, family, bold)) in read.iter().zip(&spans) {
            match &s.kind {
                ShapeKind::Text { spans, .. } => {
                    assert_eq!(spans[0].font_family, *family, "{v}");
                    assert_eq!(spans[0].bold, *bold, "{v}: {family}");
                }
                k => panic!("{v}: {k:?}"),
            }
        }
    }
}

/// Bitmap pattern fills with the same picture are one fill record, even
/// when each holds its own copy of the bytes (as a reopened file does);
/// another picture is another record.
#[test]
fn bitmap_pattern_fills_are_matched_by_content() {
    use tracedraw_core::style::Pattern;
    let mut pm = tiny_skia::Pixmap::new(8, 8).expect("pixmap");
    pm.fill(tiny_skia::Color::from_rgba8(10, 200, 30, 255));
    let bytes = pm.encode_png().expect("png");
    let fill = |png: SharedBytes| {
        Fill::Pattern(Pattern::Bitmap {
            png,
            width_px: 8,
            height_px: 8,
            size_mm: 5.0,
        })
    };
    let doc = doc_with(Vec::new());
    let mut w = Writer::new(Ver(NEWEST_VERSION), &doc.symbols, &doc.title);
    let a = w.fill_id(&fill(bytes.clone().into()));
    let b = w.fill_id(&fill(bytes.clone().into()));
    assert_eq!(a, b);
    pm.fill(tiny_skia::Color::from_rgba8(10, 200, 31, 255));
    let other = w.fill_id(&fill(pm.encode_png().expect("png").into()));
    assert_ne!(other, a);
    assert_eq!(w.fills.len(), 2);
}

/// The reader's pictures: RGB when opaque, RGBA otherwise, rows filtered
/// so a photo-like picture packs well; the pixels come back exact.
#[test]
fn reader_pictures_are_filtered_and_exact() {
    let (w, h) = (64u32, 48u32);
    let mut rgba = Vec::new();
    for y in 0..h {
        for x in 0..w {
            rgba.extend_from_slice(&[(x * 3) as u8, (y * 5) as u8, ((x + y) * 2) as u8, 255]);
        }
    }
    let png = crate::parse::encode_png(w, h, &rgba);
    assert_eq!(png[25], 2, "opaque: RGB");
    let pm = tiny_skia::Pixmap::decode_png(&png).expect("decodes");
    assert_eq!(pm.data(), &rgba[..]);
    // A gradient packs far better filtered than as plain rows.
    assert!(png.len() * 4 < rgba.len() / 4, "{} bytes", png.len());
    rgba[3] = 0;
    let png = crate::parse::encode_png(w, h, &rgba);
    assert_eq!(png[25], 6, "see-through: RGBA");
    let back = crate::parse::raw_rows(&png);
    assert_eq!(back[4], 0);
    assert_eq!(&back[5..9], &rgba[4..8]);
    // Short data: the missing rows are clear.
    let png = crate::parse::encode_png(w, h, &rgba[..rgba.len() / 2]);
    assert!(tiny_skia::Pixmap::decode_png(&png).is_ok());
}

/// A font named in Chinese keeps its name through a save and an open in
/// every version that names fonts in Unicode: from 17 the style strings
/// hold it as UTF-8, which used to read back as Windows-1252 and showed
/// the name garbled and reported missing (#898).
#[test]
fn fonts_named_in_chinese_keep_their_names() {
    let mut ids = tracedraw_core::id::IdSource::default();
    let span = TextSpan::new("标题 Title", "方正黑体_GBK", 20.0);
    let doc = doc_with(vec![text_shape(&mut ids, vec![span], TextAlign::Left)]);
    for v in 12u16..=*versions().end() {
        let bytes = document_to_cdr_version(&doc, v);
        let (back, _) = crate::open_bytes(&bytes, "back").expect("reads back");
        let families: Vec<String> = back.pages[0].layers[0]
            .shapes
            .iter()
            .filter_map(|s| match &s.kind {
                ShapeKind::Text { spans, .. } => {
                    Some(spans.iter().map(|sp| sp.font_family.clone()))
                }
                _ => None,
            })
            .flatten()
            .collect();
        assert!(!families.is_empty(), "{v}");
        assert!(
            families.iter().all(|f| f == "方正黑体_GBK"),
            "{v}: {families:?}"
        );
    }
}

/// A blend of two texts goes out as one plain group of the start text
/// and its steps as curves, beside the end text, and reads back so in
/// every version (#946).
#[test]
fn a_blend_of_two_texts_is_written_as_a_group_of_text_and_curves() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let mut start = text_shape(
        &mut ids,
        vec![TextSpan::new("ABC", "Arial", 24.0)],
        TextAlign::Left,
    );
    let mut end = text_shape(
        &mut ids,
        vec![TextSpan::new("XYZ", "Arial", 24.0)],
        TextAlign::Left,
    );
    end.transform = Affine::translate((120.0, 20.0));
    end.fill = Fill::Solid(Color::rgb8(255, 0, 0));
    start.effects.push(tracedraw_core::live::Effect::Blend {
        end: Box::new(end.clone()),
        steps: 5,
        accel_objects: 0.0,
        accel_colors: 0.0,
        rotation: 0.0,
        path: None,
        rotate_on_path: false,
        end_id: Some(end.id),
        options: Default::default(),
    });
    // The end above the start, as the Blend tool leaves them.
    doc.pages[0].layers[0].shapes = vec![start, end];
    doc.set_ids(ids);
    for v in versions() {
        let (back, rep) =
            crate::open_bytes(&document_to_cdr_version(&doc, v), "back").expect("reads back");
        assert_eq!(rep.skipped_objects, 0, "{v}: {:?}", rep.warnings);
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 2, "{v}: {read:?}");
        let ShapeKind::Group { children } = &read[0].kind else {
            panic!("{v}: {:?}", read[0].kind);
        };
        let texts: Vec<&Shape> = children
            .iter()
            .filter(|c| matches!(c.kind, ShapeKind::Text { .. }))
            .collect();
        let steps: Vec<&Shape> = children
            .iter()
            .filter(|c| matches!(c.kind, ShapeKind::Path { closed: true, .. }))
            .collect();
        assert_eq!((texts.len(), steps.len()), (1, 5), "{v}: {children:?}");
        assert_eq!(children.len(), 6, "{v}");
        let ShapeKind::Text { spans, .. } = &texts[0].kind else {
            unreachable!()
        };
        assert_eq!(spans[0].text, "ABC", "{v}");
        // The steps' colours run from the start's black to the end's red.
        for s in &steps {
            let Fill::Solid(c) = s.fill else {
                panic!("{v}: {:?}", s.fill);
            };
            assert!(
                c != Color::BLACK && c != Color::rgb8(255, 0, 0),
                "{v}: {c:?}"
            );
        }
        let ShapeKind::Text { spans, .. } = &read[1].kind else {
            panic!("{v}: {:?}", read[1].kind);
        };
        assert_eq!(spans[0].text, "XYZ", "{v}");
    }
}

/// A face drawn with another face's file (italic text in a family
/// without an italic face) is listed in the font table and the file is
/// embedded once (#1009).
#[test]
fn a_font_file_two_faces_share_is_embedded_once() {
    const FONT: &[u8] =
        include_bytes!("../../../../apps/tracedraw/assets/fonts/Selawik-Regular.ttf");
    let data = std::sync::Arc::new(FONT.to_vec());
    let face = |italic: bool| FontFace {
        family: "Selawik".into(),
        bold: false,
        italic,
        data: Some(data.clone()),
    };
    let faces = vec![face(false), face(true)];
    assert_eq!(embedded_size(&faces), FONT.len() as u64);
    let previews = Previews {
        fonts: faces,
        embed_fonts: true,
        ..Default::default()
    };
    let bytes = document_to_cdr_version_with(&doc_with(Vec::new()), NEWEST_VERSION, &previews);
    let z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("zip");
    assert!(z.file_names().any(|n| n == "embed/embedding0"));
    assert!(!z.file_names().any(|n| n == "embed/embedding1"));
    let (back, _) = crate::open_bytes(&bytes, "shared").expect("reads back");
    assert_eq!(back.fonts.len(), 1);
    assert_eq!(back.fonts[0].data.as_slice(), FONT);
}

/// `font` with every family name (name ID 1) replaced by `name`, which
/// has the old name's length.
fn renamed_font(font: &[u8], name: &str) -> Vec<u8> {
    let mut f = font.to_vec();
    let be16 = |b: &[u8], at: usize| u16::from_be_bytes([b[at], b[at + 1]]) as usize;
    let tables = be16(&f, 4);
    let name_at = (0..tables)
        .map(|i| 12 + 16 * i)
        .find(|&r| &f[r..r + 4] == b"name")
        .map(|r| u32::from_be_bytes(f[r + 8..r + 12].try_into().expect("u32")) as usize)
        .expect("name table");
    let (count, storage) = (be16(&f, name_at + 2), name_at + be16(&f, name_at + 4));
    for i in 0..count {
        let r = name_at + 6 + 12 * i;
        if be16(&f, r + 6) != 1 {
            continue;
        }
        let (len, at) = (be16(&f, r + 8), storage + be16(&f, r + 10));
        let bytes: Vec<u8> = if be16(&f, r) == 1 {
            name.bytes().collect()
        } else {
            name.encode_utf16().flat_map(|u| u.to_be_bytes()).collect()
        };
        assert_eq!(bytes.len(), len, "same length");
        f[at..at + len].copy_from_slice(&bytes);
    }
    f
}

/// The font table lists a face of a family named with its weight under
/// the family, with its weight's flags and the family's font number, as
/// the version 19 file of #624 lists "Titillium Web" at 0x400 (#892); a
/// font whose own family name is the whole name keeps it.
#[test]
fn the_font_table_lists_weight_named_faces_under_their_family() {
    const FONT: &[u8] =
        include_bytes!("../../../../apps/tracedraw/assets/fonts/Selawik-Regular.ttf");
    let named = renamed_font(FONT, "X Light");
    let face = |family: &str, data: &[u8]| FontFace {
        family: family.into(),
        bold: false,
        italic: false,
        data: Some(std::sync::Arc::new(data.to_vec())),
    };
    let faces = vec![
        face("Selawik SemiBold", FONT),
        face("Selawik", FONT),
        face("X Light", &named),
    ];
    assert_eq!(
        fonts::split_family("Selawik SemiBold", false, &faces),
        Some(("Selawik".into(), 600))
    );
    assert_eq!(fonts::split_family("X Light", false, &faces), None);
    assert_eq!(
        fonts::split_family("X Light", false, &[]),
        Some(("X".into(), 300)),
        "with no font file the name is split"
    );
    assert_eq!(fonts::split_family("Selawik SemiBold", true, &faces), None);
    assert_eq!(fonts::split_family("Selawik", false, &faces), None);

    let mut ids = tracedraw_core::id::IdSource::default();
    let doc = doc_with(vec![
        text_shape(
            &mut ids,
            vec![TextSpan::new("semi", "Selawik SemiBold", 12.0)],
            TextAlign::Left,
        ),
        text_shape(
            &mut ids,
            vec![TextSpan::new("plain", "Selawik", 12.0)],
            TextAlign::Left,
        ),
        text_shape(
            &mut ids,
            vec![TextSpan::new("light", "X Light", 12.0)],
            TextAlign::Left,
        ),
    ]);
    let previews = Previews {
        fonts: faces,
        embed_fonts: false,
        ..Default::default()
    };
    let bytes = document_to_cdr_version_with(&doc, 19, &previews);
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("zip");
    let mut table = Vec::new();
    std::io::Read::read_to_end(
        &mut z.by_name("font/fontTable.dat").expect("table"),
        &mut table,
    )
    .expect("read");
    let u32_at = |at: usize| u32::from_le_bytes(table[at..at + 4].try_into().expect("u32"));
    // Each entry: its number, flags and family.
    let mut entries = Vec::new();
    let mut at = 12;
    for _ in 0..u32_at(8) {
        let size = u32_at(at + 4) as usize;
        let e = at + 8;
        let number = u16::from_le_bytes([table[e], table[e + 1]]);
        let flags = u32_at(e + 4);
        let len = u32_at(e + 18) as usize;
        let family: String = char::decode_utf16(
            table[e + 22..e + 22 + 2 * (len - 1)]
                .chunks(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]])),
        )
        .map(|c| c.unwrap_or('?'))
        .collect();
        entries.push((number, flags, family));
        at = e + size;
    }
    let of = |family: &str, flags: u32| {
        entries
            .iter()
            .find(|(_, f, n)| *f == flags && n == family)
            .map(|(id, _, _)| *id)
    };
    let semi = of("Selawik", 0x400).expect("the 600 face under Selawik");
    assert_eq!(Some(semi), of("Selawik", 0x40), "one family, one number");
    assert!(of("X Light", 0x40).is_some(), "{entries:?}");
    assert!(!entries.iter().any(|(_, _, n)| n.contains("SemiBold")));
    // It reads back as it was written.
    let (back, _) = crate::open_bytes(&bytes, "back").expect("reads back");
    let families: Vec<(String, bool)> = back.pages[0].layers[0]
        .shapes
        .iter()
        .filter_map(|s| match &s.kind {
            ShapeKind::Text { spans, .. } => Some((spans[0].font_family.clone(), spans[0].bold)),
            _ => None,
        })
        .collect();
    assert_eq!(
        families,
        [
            ("Selawik SemiBold".to_string(), false),
            ("Selawik".to_string(), false),
            ("X Light".to_string(), false)
        ]
    );
}

/// A text whose only run has a colour of its own, and a text of several
/// paragraphs, one run each, in different colours, keep their colours in
/// every version (#1057: they came back black).
#[test]
fn single_run_colours_round_trip() {
    let mut doc = doc_with(Vec::new());
    let mut ids = doc.ids().clone();
    let red = Fill::Solid(Color::rgb8(255, 0, 0));
    let blue = Fill::Solid(Color::rgb8(0, 0, 255));
    let mut one = TextSpan::new("Red", "Arial", 12.0);
    one.fill = Some(red.clone());
    let mut a = TextSpan::new("Red\n", "Arial", 12.0);
    a.fill = Some(red.clone());
    let mut b = TextSpan::new("Blue", "Arial", 12.0);
    b.fill = Some(blue.clone());
    doc.pages[0].layers[0].shapes = vec![
        text_shape(&mut ids, vec![one], TextAlign::Left),
        text_shape(&mut ids, vec![a, b], TextAlign::Left),
    ];
    doc.set_ids(ids);
    let colour = |s: &Shape, i: usize| {
        let ShapeKind::Text { spans, .. } = &s.kind else {
            panic!("{:?}", s.kind);
        };
        let mut at = 0;
        for sp in spans {
            let n = sp.text.chars().count();
            if i < at + n {
                return sp.fill.clone().unwrap_or_else(|| s.fill.clone());
            }
            at += n;
        }
        panic!("letter {i} past {spans:?}");
    };
    for v in versions() {
        let (back, rep) =
            crate::open_bytes(&document_to_cdr_version(&doc, v), "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 2, "{v}: {:?}", rep.warnings);
        assert_eq!(colour(&read[0], 0), red, "{v}");
        assert_eq!(colour(&read[1], 0), red, "{v}");
        assert_eq!(colour(&read[1], 4), blue, "{v}");
    }
}

/// `font` with its name table replaced by one of `names` (name ID and
/// text, Windows English records), appended at the end of the file.
fn with_names(font: &[u8], names: &[(u16, &str)]) -> Vec<u8> {
    let mut f = font.to_vec();
    let be16 = |b: &[u8], at: usize| u16::from_be_bytes([b[at], b[at + 1]]) as usize;
    let dir = (0..be16(&f, 4))
        .map(|i| 12 + 16 * i)
        .find(|&r| &f[r..r + 4] == b"name")
        .expect("name table");
    let mut storage = Vec::new();
    let mut table = Vec::new();
    table.extend_from_slice(&0u16.to_be_bytes());
    table.extend_from_slice(&(names.len() as u16).to_be_bytes());
    table.extend_from_slice(&(6 + 12 * names.len() as u16).to_be_bytes());
    for (id, text) in names {
        let bytes: Vec<u8> = text.encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
        for v in [
            3u16,
            1,
            0x0409,
            *id,
            bytes.len() as u16,
            storage.len() as u16,
        ] {
            table.extend_from_slice(&v.to_be_bytes());
        }
        storage.extend_from_slice(&bytes);
    }
    table.extend_from_slice(&storage);
    while !f.len().is_multiple_of(4) {
        f.push(0);
    }
    let at = f.len() as u32;
    f[dir + 8..dir + 12].copy_from_slice(&at.to_be_bytes());
    f[dir + 12..dir + 16].copy_from_slice(&(table.len() as u32).to_be_bytes());
    f.extend_from_slice(&table);
    f
}

/// A static face of a large family names the whole name only as its
/// family (name ID 1, "Khand SemiBold") and the family in its
/// typographic names (ID 16 and 17, "Khand" and "SemiBold"): it is
/// written as the family at the SemiBold weight, its font table entry
/// "Khand" at 0x400 with the style "SemiBold", as files list such faces,
/// and its embedded font keeps the font's own names (#1055: the whole
/// name at the regular weight was reported missing where the face was
/// only embedded).
#[test]
fn a_face_with_typographic_names_is_written_under_its_family() {
    const FONT: &[u8] =
        include_bytes!("../../../../apps/tracedraw/assets/fonts/Selawik-Regular.ttf");
    let khand = with_names(
        FONT,
        &[
            (1, "Khand SemiBold"),
            (2, "Regular"),
            (4, "Khand SemiBold"),
            (6, "Khand-SemiBold"),
            (16, "Khand"),
            (17, "SemiBold"),
        ],
    );
    let faces = vec![FontFace {
        family: "Khand SemiBold".into(),
        bold: false,
        italic: false,
        data: Some(std::sync::Arc::new(khand)),
    }];
    assert_eq!(
        fonts::split_family("Khand SemiBold", false, &faces),
        Some(("Khand".into(), 600))
    );
    let mut ids = tracedraw_core::id::IdSource::default();
    let doc = doc_with(vec![text_shape(
        &mut ids,
        vec![TextSpan::new("semi", "Khand SemiBold", 12.0)],
        TextAlign::Left,
    )]);
    let previews = Previews {
        fonts: faces,
        embed_fonts: true,
        ..Default::default()
    };
    let bytes = document_to_cdr_version_with(&doc, NEWEST_VERSION, &previews);
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).expect("zip");
    let mut table = Vec::new();
    std::io::Read::read_to_end(
        &mut z.by_name("font/fontTable.dat").expect("table"),
        &mut table,
    )
    .expect("read");
    let units = |s: &str| -> Vec<u8> {
        s.encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(|u| u.to_le_bytes())
            .collect()
    };
    let has = |s: &str| table.windows(units(s).len()).any(|w| w == units(s));
    let u32_at = |at: usize| u32::from_le_bytes(table[at..at + 4].try_into().expect("u32"));
    assert_eq!(u32_at(8), 1, "one face");
    assert_eq!(u32_at(12 + 8 + 4), 0x400, "the SemiBold flags");
    assert!(
        has("Khand") && !has("Khand SemiBold"),
        "listed as the family"
    );
    assert!(has("SemiBold"), "its style");
    let mut member = Vec::new();
    std::io::Read::read_to_end(
        &mut z.by_name("embed/embedding0").expect("embedded"),
        &mut member,
    )
    .expect("read");
    let eot = crate::eot::read(&member[crate::eot::find(&member).expect("eot")..]).expect("font");
    assert_eq!(eot.family, "Khand SemiBold", "the font's own family");

    let (back, _) = crate::open_bytes(&bytes, "khand").expect("reads back");
    let ShapeKind::Text { spans, .. } = &back.pages[0].layers[0].shapes[0].kind else {
        panic!("not text");
    };
    assert_eq!(spans[0].font_family, "Khand SemiBold");
    assert!(!spans[0].bold);
    assert_eq!(back.fonts.len(), 1);
}
