//! A paragraph text with a drop shadow writes the shadow once (#1031).
//!
//! Paragraph text goes out as one artistic text a line, which needs the
//! text engine's line breaking; this test binary registers a stand-in
//! that breaks after every two words (the registration is process-wide,
//! so it lives in a test binary of its own).

use std::io::{Read as _, Write as _};

use tracedraw_core::document::text_outline::{self, Line, TextRequest};
use tracedraw_core::{
    Affine, Color, Document, Fill, Shadow, Shape, ShapeKind, Size, TextAlign, TextSpan,
};

/// Lines of two words each, 5 mm apart, from the frame's top.
fn two_words_a_line(req: &TextRequest) -> Vec<Line> {
    let text: String = req.spans.iter().map(|s| s.text.as_str()).collect();
    let chars: Vec<char> = text.chars().collect();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut words = 0;
    for (i, c) in chars.iter().enumerate() {
        let end = i + 1 == chars.len();
        if *c == ' ' {
            words += 1;
        }
        if (*c == ' ' && words % 2 == 0) || end {
            let stop = if end { i + 1 } else { i };
            lines.push(Line {
                start_char: start,
                end_char: stop,
                x: 0.0,
                baseline: -5.0 * (lines.len() + 1) as f64,
                ascent: 4.0,
                descent: 1.0,
                edges: Vec::new(),
                starts: Vec::new(),
                ends: Vec::new(),
            });
            start = i + 1;
        }
    }
    lines
}

/// The bitmap objects among `shapes`, in groups too.
fn bitmaps(shapes: &[Shape]) -> usize {
    shapes
        .iter()
        .map(|s| match &s.kind {
            ShapeKind::Bitmap { .. } => 1,
            ShapeKind::Group { children } => bitmaps(children),
            _ => 0,
        })
        .sum()
}

/// The package without TraceDraw's own members: what another reader
/// sees.
fn without_own_members(bytes: &[u8]) -> Vec<u8> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip");
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for i in 0..z.len() {
        let mut f = z.by_index(i).expect("member");
        let name = f.name().to_string();
        if name.starts_with("tracedraw/") {
            continue;
        }
        let mut data = Vec::new();
        f.read_to_end(&mut data).expect("read");
        out.start_file(name, stored).expect("start");
        out.write_all(&data).expect("write");
    }
    out.finish().expect("finish").into_inner()
}

#[test]
fn a_paragraph_text_casts_one_shadow_picture() {
    text_outline::set_lines(two_words_a_line);
    let mut doc = Document::new("p", Size::new(200.0, 100.0));
    let mut ids = doc.ids().clone();
    let mut t = Shape::new(
        ids.shape(),
        ShapeKind::Text {
            spans: vec![TextSpan::new(
                "one two three four five six seven eight",
                "Arial",
                12.0,
            )],
            origin: tracedraw_core::Point::ZERO,
            frame: Some(Size::new(40.0, 60.0)),
            align: TextAlign::Left,
            para: Default::default(),
            on_path: None,
        },
    );
    t.transform = Affine::translate((20.0, 80.0));
    t.fill = Fill::Solid(Color::BLACK);
    t.stroke = None;
    let shadow = Shadow {
        offset: tracedraw_core::Vec2::new(2.0, -2.0),
        opacity: 0.5,
        blur: 0.0,
        color: Color::BLACK,
        ..Default::default()
    };
    t.shadow = Some(shadow);
    let lines = tracedraw_core::document::paragraph_lines(&t.kind)
        .expect("lines")
        .len();
    assert_eq!(lines, 4);
    doc.pages[0].layers[0].shapes = vec![t];
    doc.set_ids(ids);
    for v in [8u16, 13, 14, 16, 21, 27] {
        let bytes = tracedraw_cdr::write::document_to_cdr_version(&doc, v);
        // What any reader sees: one shadow picture beneath the lines
        // (before, each line and the text wrote one: five here).
        let drawn = if v >= 14 {
            without_own_members(&bytes)
        } else {
            bytes.clone()
        };
        let (shown, _) = tracedraw_cdr::open_bytes(&drawn, "drawn").expect("reads");
        let shapes = &shown.pages[0].layers[0].shapes;
        assert_eq!(bitmaps(shapes), 1, "{v}: {shapes:#?}");
        assert!(
            matches!(shapes[0].kind, ShapeKind::Bitmap { .. }),
            "{v}: the picture first: {:?}",
            shapes[0].kind
        );
        let rect = shapes[0].bounds();
        assert!(rect.width() > 0.0 && rect.height() > 0.0, "{v}");
        if v < 14 {
            continue;
        }
        // TraceDraw opens the paragraph text whole, with its shadow.
        let (back, rep) = tracedraw_cdr::open_bytes(&bytes, "back").expect("reads back");
        let read = &back.pages[0].layers[0].shapes;
        assert_eq!(read.len(), 1, "{v}: {read:?} {:?}", rep.warnings);
        assert!(
            matches!(read[0].kind, ShapeKind::Text { frame: Some(_), .. }),
            "{v}: {:?}",
            read[0].kind
        );
        assert_eq!(read[0].shadow, Some(shadow), "{v}");
    }
}
