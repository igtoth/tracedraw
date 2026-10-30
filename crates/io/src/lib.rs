//! tracedraw-io: export the document model to interchange formats and read the
//! native `.tdraw` file.

#![allow(clippy::field_reassign_with_default)]
pub mod dxf;
pub mod emf;
pub mod eps;
pub mod eps_import;
pub mod ggr;
pub mod html;
pub mod pdf;
pub mod pdf_crypt;
pub(crate) mod pdf_glyphs;
pub mod pdf_import;
pub mod plt;
pub mod psd;
pub mod svg;
pub mod svg_import;
pub mod text_import;
pub mod xlsx;

use std::path::Path;
use tracedraw_core::Document;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Core(#[from] tracedraw_core::Error),
    #[error("{0}")]
    Format(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The document as vector exports should see it: unless the document
/// fills open curves, an open curve's fill is kept for its closed subpaths
/// only (the curve becomes a group of the filled closed part and the
/// outlined whole), or dropped when it has none, as the screen shows it.
pub fn resolve_open_fills(doc: &Document) -> std::borrow::Cow<'_, Document> {
    use tracedraw_core::document::{Shape, ShapeKind};
    use tracedraw_core::Fill;
    fn needs(s: &Shape) -> bool {
        match &s.kind {
            ShapeKind::Path { closed: false, .. } => !matches!(s.fill, Fill::None),
            ShapeKind::Group { children } => children.iter().any(needs),
            ShapeKind::ClipFrame { frame, contents } => needs(frame) || contents.iter().any(needs),
            _ => false,
        }
    }
    fn fix(s: &mut Shape) {
        match &mut s.kind {
            ShapeKind::Path {
                path,
                closed: false,
            } if !matches!(s.fill, Fill::None) => {
                let closed = tracedraw_render::closed_subpaths(path);
                if closed.elements().is_empty() {
                    s.fill = Fill::None;
                    return;
                }
                let mut filled = s.clone();
                filled.kind = ShapeKind::Path {
                    path: closed,
                    closed: true,
                };
                filled.stroke = None;
                filled.effects.clear();
                filled.transform = tracedraw_core::geometry::Affine::IDENTITY;
                let mut outlined = s.clone();
                outlined.fill = Fill::None;
                outlined.effects.clear();
                outlined.transform = tracedraw_core::geometry::Affine::IDENTITY;
                s.kind = ShapeKind::Group {
                    children: vec![filled, outlined],
                };
                s.fill = Fill::None;
                s.stroke = None;
            }
            ShapeKind::Group { children } => children.iter_mut().for_each(fix),
            ShapeKind::ClipFrame { frame, contents } => {
                fix(frame);
                contents.iter_mut().for_each(fix);
            }
            _ => {}
        }
    }
    if doc.metadata.fill_open_curves || !doc.all_layers().flat_map(|l| &l.shapes).any(needs) {
        return std::borrow::Cow::Borrowed(doc);
    }
    let mut out = doc.clone();
    for page in &mut out.pages {
        for layer in &mut page.layers {
            layer.shapes.iter_mut().for_each(fix);
        }
    }
    for layer in &mut out.master {
        layer.shapes.iter_mut().for_each(fix);
    }
    std::borrow::Cow::Owned(out)
}

/// Native format: pretty JSON with a `.tdraw` extension.
pub fn save_native(doc: &Document, path: impl AsRef<Path>) -> Result<()> {
    // Written as it is made, not gathered in memory first.
    let mut w = std::io::BufWriter::new(std::fs::File::create(path)?);
    doc.write_json_version(&mut w, tracedraw_core::document::TDRAW_FORMAT_VERSION)?;
    std::io::Write::flush(&mut w)?;
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

/// Write one page as an ASCII DXF file.
pub fn save_dxf(doc: &Document, page_index: usize, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, dxf::page_to_dxf(doc, page_index))?;
    Ok(())
}

/// Write one page as an HPGL plotter file.
pub fn save_plt(doc: &Document, page_index: usize, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, plt::page_to_plt(doc, page_index))?;
    Ok(())
}

/// Write one page as a Photoshop file (one raster layer per document layer).
pub fn save_psd(doc: &Document, page_index: usize, dpi: f64, path: impl AsRef<Path>) -> Result<()> {
    let bytes = psd::page_to_psd(doc, page_index, dpi)
        .ok_or_else(|| Error::Format("page could not be rendered".into()))?;
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Write one page as an enhanced metafile (EMF).
pub fn save_emf(doc: &Document, page_index: usize, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, emf::page_to_emf(doc, page_index))?;
    Ok(())
}

/// Write one page as a placeable Windows metafile (WMF).
pub fn save_wmf(doc: &Document, page_index: usize, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, emf::page_to_wmf(doc, page_index))?;
    Ok(())
}

/// Write the first page (or `page_index`) as an SVG file.
pub fn save_svg(doc: &Document, page_index: usize, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, svg::page_to_svg(doc, page_index))?;
    Ok(())
}

/// A PNG picture of each page, its longer side `max_side` pixels on a
/// white background, as `(png, width, height)` in page order (the
/// pictures `.cdr` packages store with the drawing). Pages that cannot
/// be rendered are left out of the list's end.
pub fn page_pictures(doc: &Document, max_side: u32) -> Vec<(Vec<u8>, u32, u32)> {
    let mut out = Vec::new();
    for page in &doc.pages {
        let longest = page.size.width.max(page.size.height);
        if !longest.is_finite() || longest <= 0.0 {
            break;
        }
        // Pixels per inch that make the longer side `max_side` pixels.
        let dpi = f64::from(max_side.max(1)) / (longest / 25.4);
        let Some(pm) = tracedraw_render::render_page_image(doc, page.id, dpi) else {
            break;
        };
        let (w, h) = (pm.width(), pm.height());
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for p in pm.pixels() {
            let c = p.demultiply();
            rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
        }
        let png = svg::encode_png(w, h, &rgba);
        if png.is_empty() {
            break;
        }
        out.push((png, w, h));
    }
    out
}

/// The resolution of the drop shadow pictures [`shadow_pictures`] makes
/// for files: enough for a printed page, small enough for a file.
pub const SHADOW_PICTURE_DPI: f64 = 150.0;

/// The drop shadows of the drawing's visible objects as pictures at
/// `dpi`, for file formats that keep no shadows (`.cdr` writes each as a
/// bitmap beneath its object): every object with a shadow, on any page,
/// in any group or clip frame, as `(shape, page rectangle, png, width,
/// height)`, the picture holding the shadow alone with its transparency.
pub fn shadow_pictures(
    doc: &Document,
    dpi: f64,
) -> Vec<(
    tracedraw_core::ShapeId,
    tracedraw_core::Rect,
    Vec<u8>,
    u32,
    u32,
)> {
    use tracedraw_core::{Affine, Shape, ShapeKind};
    fn walk(
        shapes: &[Shape],
        parent: Affine,
        dpi: f64,
        out: &mut Vec<(
            tracedraw_core::ShapeId,
            tracedraw_core::Rect,
            Vec<u8>,
            u32,
            u32,
        )>,
    ) {
        for s in shapes {
            if !s.visible {
                continue;
            }
            if s.shadow.is_some_and(|sh| sh.opacity > 0.0) {
                let mut on_page = s.clone();
                on_page.absorb(parent);
                if let Some((rect, pm)) = tracedraw_render::render_shadow_image(&on_page, dpi) {
                    let (w, h) = (pm.width(), pm.height());
                    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
                    for p in pm.pixels() {
                        let c = p.demultiply();
                        rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
                    }
                    let png = svg::encode_png(w, h, &rgba);
                    if !png.is_empty() {
                        out.push((s.id, rect, png, w, h));
                    }
                }
            }
            let inner = parent * s.transform;
            match &s.kind {
                ShapeKind::Group { children } => walk(children, inner, dpi, out),
                ShapeKind::ClipFrame { frame, contents } => {
                    walk(std::slice::from_ref(frame), inner, dpi, out);
                    walk(contents, inner, dpi, out);
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for page in &doc.pages {
        for layer in &page.layers {
            walk(&layer.shapes, Affine::IDENTITY, dpi, &mut out);
        }
    }
    out
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
    fn open_curves_lose_their_fill_in_vector_exports() {
        use tracedraw_core::geometry::{BezPath, Point};
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let mut path = BezPath::new();
        path.move_to(Point::new(10.0, 10.0));
        path.line_to(Point::new(50.0, 10.0));
        path.line_to(Point::new(50.0, 50.0));
        let mut open = Shape::new(
            tracedraw_core::ShapeId(5),
            ShapeKind::Path {
                path,
                closed: false,
            },
        );
        open.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        open.stroke = Some(Stroke::hairline(Color::BLACK));
        doc.layer_mut(layer).unwrap().shapes.push(open);
        let resolved = resolve_open_fills(&doc);
        let s = &resolved.pages[0].layers[0].shapes[0];
        assert!(matches!(s.fill, Fill::None));
        assert!(s.stroke.is_some());
        let svg = svg::page_to_svg(&doc, 0);
        assert!(!svg.contains("fill=\"#ff0000\""), "{svg}");
        // A document that fills open curves exports the fill.
        doc.metadata.fill_open_curves = true;
        assert!(matches!(
            resolve_open_fills(&doc),
            std::borrow::Cow::Borrowed(_)
        ));
    }

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
                corners: None,
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
                corners: None,
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

    /// The axis values of a variable font set on text are kept in the
    /// native file (#1013): a weight of 600 reopens at 600.
    #[test]
    fn variable_font_axes_round_trip_in_native_files() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut span = tracedraw_core::TextSpan::new("Weight", "MiSans VF", 24.0);
        span.set_variation("wght", Some(600.0));
        span.set_variation("wdth", Some(87.5));
        let s = Shape::new(
            id,
            ShapeKind::Text {
                spans: vec![span],
                origin: tracedraw_core::geometry::Point::new(10.0, 20.0),
                frame: None,
                align: tracedraw_core::TextAlign::Left,
                para: tracedraw_core::document::ParagraphStyle::default(),
                on_path: None,
            },
        );
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let path = std::env::temp_dir().join(format!(
            "tracedraw-font-axes-{}-{:?}.tdraw",
            std::process::id(),
            std::thread::current().id()
        ));
        save_native(&doc, &path).unwrap();
        let back = load_native(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(back, doc);
        let ShapeKind::Text { spans, .. } = &back.pages[0].layers[0].shapes[0].kind else {
            panic!("not text");
        };
        assert_eq!(spans[0].variation("wght"), Some(600.0));
        assert_eq!(spans[0].variation("wdth"), Some(87.5));
    }

    #[test]
    fn outline_position_mitre_limit_dashes_and_shadows_round_trip() {
        use tracedraw_core::shadow::{FeatherDirection, FeatherEdges, Perspective, ShadowEdge};
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(10.0, 10.0, 40.0, 30.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.stroke = Some(Stroke {
            position: tracedraw_core::OutlinePosition::Outside,
            miter_limit: 17.5,
            align_dashes: true,
            dash: vec![5.0, 1.0, 1.0, 1.0],
            ..Stroke::new(Color::BLACK, 1.0)
        });
        s.shadow = Some(tracedraw_core::Shadow {
            merge: tracedraw_core::live::MergeMode::IfDarker,
            feather_direction: FeatherDirection::Outside,
            feather_edges: FeatherEdges::InvertedSquared,
            perspective: Some(Perspective {
                edge: ShadowEdge::Top,
                angle: 75.0,
                stretch: 0.4,
                fade: 0.3,
            }),
            ..Default::default()
        });
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let json = doc.to_json().unwrap();
        let back = Document::from_json(&json).unwrap();
        assert_eq!(back, doc);
    }

    #[test]
    fn corners_that_keep_their_size_survive_native_svg_and_pdf() {
        use tracedraw_core::geometry::{Affine, Shape as _};
        use tracedraw_core::{CornerKind, Corners};
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let mut c = Corners::uniform(5.0, CornerKind::Round);
        c.fixed = true;
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(
            id,
            ShapeKind::rect_with_corners(Rect::new(0.0, 0.0, 50.0, 30.0), c),
        );
        // Stretched to 100 x 30 mm: the corners stay 5 mm circles.
        s.transform = Affine::translate((20.0, 20.0)) * Affine::scale_non_uniform(2.0, 1.0);
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        s.stroke = None;
        let area = s.page_path().area().abs();
        let circles = 100.0 * 30.0 - (4.0 - std::f64::consts::PI) * 25.0;
        assert!((area - circles).abs() < 0.05, "{area}");
        doc.layer_mut(layer).unwrap().shapes.push(s);
        // Native: the flag is written and read back.
        let json = doc.to_json().unwrap();
        assert!(json.contains("\"fixed\": true"), "{json}");
        assert_eq!(Document::from_json(&json).unwrap(), doc);
        // SVG and PDF draw the outline the rectangle shows.
        let mut ids = tracedraw_core::id::IdSource::default();
        let svg = svg_import::parse(&svg::page_to_svg(&doc, 0), &mut ids).expect("svg");
        let pdf = pdf_import::to_document(
            pdf_import::parse(&pdf::document_to_pdf(&doc), &mut ids).expect("pdf"),
            "t",
        );
        let pdf_shapes: Vec<Shape> = pdf.all_layers().flat_map(|l| l.shapes.clone()).collect();
        for (what, shapes) in [("svg", svg.shapes), ("pdf", pdf_shapes)] {
            let got: f64 = shapes.iter().map(|s| s.page_path().area().abs()).sum();
            assert!((got - area).abs() < 1.0, "{what}: {got} vs {area}");
        }
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

#[cfg(test)]
mod shadow_picture_tests {
    use super::*;
    use tracedraw_core::{
        document::{Shape, ShapeKind},
        geometry::{Affine, Rect, Size, Vec2},
        style::Fill,
        Color, Shadow,
    };

    /// Every object with a shadow gets a picture over the shadow's area,
    /// also inside a moved group (its place on the page), and objects
    /// without one get none (#1031).
    #[test]
    fn shadow_pictures_cover_each_shadow_on_the_page() {
        let mut doc = Document::new("s", Size::new(200.0, 100.0));
        let mut ids = doc.ids().clone();
        let shadow = Shadow {
            offset: Vec2::new(4.0, -4.0),
            opacity: 0.7,
            blur: 0.0,
            color: Color::BLACK,
            ..Default::default()
        };
        let mut a = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 30.0, 20.0),
                radius: 0.0,
                corners: None,
            },
        );
        a.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        a.shadow = Some(shadow);
        let mut b = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        b.fill = Fill::Solid(Color::rgb8(0, 0, 255));
        b.shadow = Some(shadow);
        let plain = Shape::new(
            ids.shape(),
            ShapeKind::Ellipse {
                rect: Rect::new(50.0, 50.0, 60.0, 60.0),
                arc: None,
            },
        );
        let (a_id, b_id) = (a.id, b.id);
        let mut group = Shape::new(
            ids.shape(),
            ShapeKind::Group {
                children: vec![b, plain],
            },
        );
        group.transform = Affine::translate((100.0, 50.0));
        doc.pages[0].layers[0].shapes = vec![a, group];
        doc.set_ids(ids);
        let pictures = shadow_pictures(&doc, 150.0);
        assert_eq!(pictures.len(), 2, "{pictures:?}");
        let of = |id| pictures.iter().find(|p| p.0 == id).expect("picture");
        let (_, rect, png, w, h) = of(a_id);
        // The shadow of `a` lies 4 mm right of and below it.
        assert!(
            rect.x0 >= 10.0 && rect.x1 >= 34.0 - 0.01 && rect.y0 <= 6.0 + 0.01,
            "{rect:?}"
        );
        assert!(*w > 0 && *h > 0 && png.starts_with(&[0x89, b'P', b'N', b'G']));
        let (_, rect, ..) = of(b_id);
        // `b` sits at (100, 50) on the page.
        assert!(
            rect.x0 >= 100.0 && rect.x1 >= 114.0 - 0.01 && rect.y0 <= 46.0 + 0.01,
            "{rect:?}"
        );
        assert!(shadow_pictures(&doc, 0.0).is_empty());
    }
}
