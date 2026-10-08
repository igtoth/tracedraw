//! Writes a sample document with every object type, for manual testing:
//! `cargo run -p tracedraw-io --example sample -- out.tdraw`

use tracedraw_core::{
    document::{Shape, ShapeKind, TextSpan},
    geometry::{self, Affine, Point, Rect},
    Color, Document, Fill, Stroke,
};

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "sample.tdraw".into());
    let mut doc = Document::default();
    doc.title = "Sample".into();
    let layer = doc.pages[0].layers[0].id;
    let add =
        |doc: &mut Document, kind: ShapeKind, fill: Fill, stroke: Option<Stroke>, t: Affine| {
            let id = doc.ids_mut().shape();
            let mut s = Shape::new(id, kind);
            s.fill = fill;
            s.stroke = stroke;
            s.transform = t;
            doc.layer_mut(layer).unwrap().shapes.push(s);
        };
    add(
        &mut doc,
        ShapeKind::Rect {
            rect: Rect::new(20.0, 180.0, 90.0, 240.0),
            radius: 5.0,
        },
        Fill::Solid(Color::cmyk_pct(100.0, 0.0, 0.0, 0.0)),
        Some(Stroke::new(Color::BLACK, 0.5)),
        Affine::IDENTITY,
    );
    add(
        &mut doc,
        ShapeKind::Ellipse {
            rect: Rect::new(110.0, 180.0, 190.0, 240.0),
        },
        Fill::Linear {
            from: Color::cmyk_pct(0.0, 100.0, 100.0, 0.0),
            to: Color::WHITE,
            angle: 45.0,
        },
        Some(Stroke::hairline(Color::BLACK)),
        Affine::IDENTITY,
    );
    add(
        &mut doc,
        ShapeKind::Polygon {
            rect: Rect::new(20.0, 80.0, 90.0, 150.0),
            points: 5,
            sharpness: 0.5,
        },
        Fill::Solid(Color::cmyk_pct(0.0, 0.0, 100.0, 0.0)),
        Some(Stroke::new(Color::cmyk_pct(0.0, 0.0, 0.0, 100.0), 1.0)),
        Affine::rotate_about(0.3, Point::new(55.0, 115.0)),
    );
    let pts = [
        Point::new(110.0, 90.0),
        Point::new(130.0, 150.0),
        Point::new(160.0, 85.0),
        Point::new(190.0, 140.0),
    ];
    add(
        &mut doc,
        ShapeKind::Path {
            path: geometry::smooth_path(&pts, false),
            closed: false,
        },
        Fill::None,
        Some(Stroke::new(Color::cmyk_pct(100.0, 100.0, 0.0, 0.0), 1.5)),
        Affine::IDENTITY,
    );
    add(
        &mut doc,
        ShapeKind::Text {
            spans: vec![TextSpan {
                text: "TraceDraw".into(),
                font_family: "Arial".into(),
                size_pt: 36.0,
                bold: true,
                italic: false,
            }],
            origin: Point::new(20.0, 40.0),
            frame: None,
            align: tracedraw_core::TextAlign::Left,
        },
        Fill::Solid(Color::BLACK),
        None,
        Affine::IDENTITY,
    );
    // A semi-transparent rounded square over the text, and a small bitmap.
    add(
        &mut doc,
        ShapeKind::Rect {
            rect: Rect::new(60.0, 25.0, 110.0, 60.0),
            radius: 3.0,
        },
        Fill::Solid(Color::cmyk_pct(100.0, 0.0, 0.0, 0.0)),
        None,
        Affine::IDENTITY,
    );
    if let Some(last) = doc.layer_mut(layer).unwrap().shapes.last_mut() {
        last.opacity = 0.5;
    }
    let mut pm = tiny_skia::Pixmap::new(64, 48).unwrap();
    for y in 0..48 {
        for x in 0..64 {
            let p = pm.pixels_mut().get_mut((y * 64 + x) as usize).unwrap();
            *p = tiny_skia::ColorU8::from_rgba(x as u8 * 4, y as u8 * 5, 128, 255).premultiply();
        }
    }
    let png = pm.encode_png().unwrap();
    add(
        &mut doc,
        ShapeKind::Bitmap {
            rect: Rect::new(130.0, 20.0, 190.0, 65.0),
            width_px: 64,
            height_px: 48,
            png,
        },
        Fill::None,
        None,
        Affine::rotate_about(-0.2, Point::new(160.0, 42.0)),
    );
    tracedraw_io::save_native(&doc, &out).expect("write");
    println!("wrote {out}");
}
