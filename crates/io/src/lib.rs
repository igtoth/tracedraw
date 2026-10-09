//! tracedraw-io: export the document model to interchange formats and read the
//! native `.tdraw` file.

pub mod eps;
pub mod pdf;
pub mod svg;
pub mod svg_import;

use std::path::Path;
use tracedraw_core::Document;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Core(#[from] tracedraw_core::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Native format: pretty JSON with a `.tdraw` extension.
pub fn save_native(doc: &Document, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, doc.to_json()?)?;
    Ok(())
}

pub fn load_native(path: impl AsRef<Path>) -> Result<Document> {
    let s = std::fs::read_to_string(path)?;
    Ok(Document::from_json(&s)?)
}

/// Write every page to a PDF file.
pub fn save_pdf(doc: &Document, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, pdf::document_to_pdf(doc))?;
    Ok(())
}

/// Write the first page (or `page_index`) as an SVG file.
pub fn save_svg(doc: &Document, page_index: usize, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, svg::page_to_svg(doc, page_index))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{
        document::{Shape, ShapeKind},
        geometry::{Rect, Size},
        Color, Fill, Pattern, Stroke,
    };

    #[test]
    fn vector_pattern_fill_round_trips_through_the_native_format() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        // A tile with a filled square and an outlined group, built from
        // page-space shapes so the helper has to move them to the origin.
        let mut square = Shape::new(
            tracedraw_core::ShapeId(7),
            ShapeKind::Rect {
                rect: Rect::new(40.0, 40.0, 45.0, 45.0),
                radius: 0.0,
            },
        );
        square.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        square.stroke = None;
        let mut circle = Shape::new(
            tracedraw_core::ShapeId(8),
            ShapeKind::Ellipse {
                rect: Rect::new(45.0, 45.0, 50.0, 50.0),
                arc: None,
            },
        );
        circle.fill = Fill::linear(Color::rgb8(0, 0, 255), Color::WHITE, 45.0);
        circle.stroke = Some(Stroke::new(Color::BLACK, 0.5));
        let group = Shape::new(
            tracedraw_core::ShapeId(9),
            ShapeKind::Group {
                children: vec![circle],
            },
        );
        let pattern = Pattern::vector_from_shapes(&[square, group]);
        assert!(matches!(&pattern, Pattern::Vector { tile, .. } if *tile == Size::new(10.0, 10.0)));
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 60.0, 60.0),
                radius: 0.0,
            },
        );
        s.fill = Fill::Pattern(pattern);
        doc.layer_mut(layer).unwrap().shapes.push(s);

        let path = std::env::temp_dir().join(format!(
            "tracedraw-vector-pattern-{}-{:?}.tdraw",
            std::process::id(),
            std::thread::current().id()
        ));
        save_native(&doc, &path).unwrap();
        let back = load_native(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(back, doc);
        let shapes = &back.pages[0].layers[0].shapes;
        assert!(matches!(
            &shapes[0].fill,
            Fill::Pattern(Pattern::Vector { shapes, .. }) if shapes.len() == 2
        ));
    }

    #[test]
    fn custom_arrowhead_round_trips_through_the_native_format() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let mut tri = tracedraw_core::BezPath::new();
        tri.move_to((0.0, 0.0));
        tri.line_to((20.0, 10.0));
        tri.line_to((0.0, 20.0));
        tri.close_path();
        let id = doc.ids_mut().shape();
        let mut line = Shape::new(
            id,
            ShapeKind::Path {
                path: {
                    let mut p = tracedraw_core::BezPath::new();
                    p.move_to((10.0, 50.0));
                    p.line_to((60.0, 50.0));
                    p
                },
                closed: false,
            },
        );
        line.stroke = Some(Stroke {
            start_arrow: tracedraw_core::Arrowhead::Circle,
            end_arrow: tracedraw_core::Arrowhead::from_shape_path(&tri, "My head"),
            ..Stroke::new(Color::BLACK, 1.0)
        });
        doc.layer_mut(layer).unwrap().shapes.push(line);
        let json = doc.to_json().unwrap();
        assert!(json.contains("\"custom\""));
        assert!(json.contains("My head"));
        let back = Document::from_json(&json).unwrap();
        assert_eq!(back, doc);
        let s = &back.pages[0].layers[0].shapes[0];
        let st = s.stroke.as_ref().unwrap();
        assert!(st.end_arrow.is_custom());
        assert_eq!(st.end_arrow.name(), "My head");
        assert_eq!(st.start_arrow, tracedraw_core::Arrowhead::Circle);
    }
}
