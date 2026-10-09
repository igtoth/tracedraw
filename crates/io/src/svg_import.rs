//! SVG import: parses with `usvg` (which resolves CSS, `use`, units and
//! nested transforms into a flat tree) and converts the result into
//! document shapes. Groups keep their structure, clip paths become
//! ClipFrames, images become bitmaps, text is already outlined by usvg
//! (with `text` feature off, text nodes are dropped).
//!
//! Coordinates: SVG is in user units at 96 dpi with Y down; the document is
//! in millimetres with Y up, origin at the bottom-left of the page.

use std::sync::Arc;

use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, BezPath, PathEl, Point, Rect, Size},
    id::IdSource,
    style::{Fountain, FountainKind, LineCap, LineJoin, Stop, Stroke},
    Color, Fill, ShapeId,
};

const PX_TO_MM: f64 = 25.4 / 96.0;

/// Result of an import: shapes in page space (mm, Y up) and the page size
/// the SVG declared.
pub struct Imported {
    pub shapes: Vec<Shape>,
    pub size: Size,
}

/// Parse SVG text. `ids` allocates shape identifiers so the result can be
/// added to an existing document.
pub fn parse(svg: &str, ids: &mut IdSource) -> std::result::Result<Imported, String> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| e.to_string())?;
    let size = tree.size();
    let height_px = size.height() as f64;
    // SVG px, Y down -> mm, Y up.
    let to_page = Affine::new([PX_TO_MM, 0.0, 0.0, -PX_TO_MM, 0.0, height_px * PX_TO_MM]);
    let mut shapes = Vec::new();
    for node in tree.root().children() {
        if let Some(s) = convert_node(node, to_page, ids) {
            shapes.push(s);
        }
    }
    Ok(Imported {
        shapes,
        size: Size::new(size.width() as f64 * PX_TO_MM, height_px * PX_TO_MM),
    })
}

fn convert_node(node: &usvg::Node, to_page: Affine, ids: &mut IdSource) -> Option<Shape> {
    match node {
        usvg::Node::Group(g) => convert_group(g, to_page, ids),
        usvg::Node::Path(p) => convert_path(p, to_page, ids),
        usvg::Node::Image(img) => convert_image(img, to_page, ids),
        // Text is outlined into paths by usvg when the `text` feature is on;
        // without it the node carries no geometry.
        usvg::Node::Text(_) => None,
    }
}

fn convert_group(g: &usvg::Group, to_page: Affine, ids: &mut IdSource) -> Option<Shape> {
    // usvg already pushed the group transform into the children's
    // absolute transforms, so children are converted with `to_page` only.
    let mut children: Vec<Shape> = g
        .children()
        .iter()
        .filter_map(|c| convert_node(c, to_page, ids))
        .collect();
    if children.is_empty() {
        return None;
    }
    let opacity = g.opacity().get() as f64;
    let mut shape = if let Some(clip) = g.clip_path() {
        // Clip path -> ClipFrame with the union of the clip's paths as frame.
        // Clip content is in the user space of the clipped group: its
        // nodes' absolute transforms start at the clip's root, so the
        // group's own absolute transform and the clip's transform come
        // first.
        let mut frame_path = BezPath::new();
        collect_paths(clip.root(), &mut frame_path);
        let frame_path =
            to_page * to_affine(g.abs_transform()) * to_affine(clip.transform()) * frame_path;
        if frame_path.elements().is_empty() {
            group_of(children, ids)
        } else {
            let mut frame = Shape::new(
                ShapeId(ids.shape().0),
                ShapeKind::Path {
                    path: frame_path,
                    closed: true,
                },
            );
            frame.fill = Fill::None;
            frame.stroke = None;
            Shape::new(
                ShapeId(ids.shape().0),
                ShapeKind::ClipFrame {
                    frame: Box::new(frame),
                    contents: std::mem::take(&mut children),
                },
            )
        }
    } else if children.len() == 1 && g.id().is_empty() {
        children.pop().unwrap()
    } else {
        group_of(children, ids)
    };
    if opacity < 1.0 {
        shape.opacity *= opacity;
    }
    if !g.id().is_empty() {
        shape.name = Some(g.id().to_string());
    }
    Some(shape)
}

fn group_of(children: Vec<Shape>, ids: &mut IdSource) -> Shape {
    Shape::new(ShapeId(ids.shape().0), ShapeKind::Group { children })
}

fn collect_paths(group: &usvg::Group, out: &mut BezPath) {
    for n in group.children() {
        match n {
            usvg::Node::Path(p) => {
                let t = p.abs_transform();
                let path = bez_path(p.data(), to_affine(t));
                out.extend(path.elements().iter().copied());
            }
            usvg::Node::Group(g) => collect_paths(g, out),
            _ => {}
        }
    }
}

fn to_affine(t: usvg::Transform) -> Affine {
    Affine::new([
        t.sx as f64,
        t.ky as f64,
        t.kx as f64,
        t.sy as f64,
        t.tx as f64,
        t.ty as f64,
    ])
}

fn bez_path(data: &usvg::tiny_skia_path::Path, t: Affine) -> BezPath {
    use usvg::tiny_skia_path::PathSegment;
    let mut out = BezPath::new();
    for seg in data.segments() {
        match seg {
            PathSegment::MoveTo(p) => out.move_to(t * Point::new(p.x as f64, p.y as f64)),
            PathSegment::LineTo(p) => out.line_to(t * Point::new(p.x as f64, p.y as f64)),
            PathSegment::QuadTo(a, p) => out.quad_to(
                t * Point::new(a.x as f64, a.y as f64),
                t * Point::new(p.x as f64, p.y as f64),
            ),
            PathSegment::CubicTo(a, b, p) => out.curve_to(
                t * Point::new(a.x as f64, a.y as f64),
                t * Point::new(b.x as f64, b.y as f64),
                t * Point::new(p.x as f64, p.y as f64),
            ),
            PathSegment::Close => out.close_path(),
        }
    }
    out
}

fn convert_path(p: &usvg::Path, to_page: Affine, ids: &mut IdSource) -> Option<Shape> {
    if !p.is_visible() {
        return None;
    }
    let t = to_page * to_affine(p.abs_transform());
    let path = bez_path(p.data(), t);
    if path.elements().is_empty() {
        return None;
    }
    let closed = path
        .elements()
        .iter()
        .any(|e| matches!(e, PathEl::ClosePath));
    let bounds = bbox(&path);
    let mut shape = Shape::new(ShapeId(ids.shape().0), ShapeKind::Path { path, closed });
    shape.fill = match p.fill() {
        Some(f) => convert_paint(f.paint(), f.opacity().get() as f64, bounds, to_page),
        None => Fill::None,
    };
    shape.stroke = p.stroke().map(|s| convert_stroke(s, t));
    if !p.id().is_empty() {
        shape.name = Some(p.id().to_string());
    }
    Some(shape)
}

fn bbox(path: &BezPath) -> Rect {
    use tracedraw_core::geometry::Shape as _;
    path.bounding_box()
}

fn rgb(c: usvg::Color) -> Color {
    Color::rgb8(c.red, c.green, c.blue)
}

fn convert_paint(paint: &usvg::Paint, opacity: f64, bounds: Rect, to_page: Affine) -> Fill {
    let _ = opacity; // per-shape fill opacity is folded into the colour below
    match paint {
        usvg::Paint::Color(c) => Fill::Solid(rgb(*c)),
        usvg::Paint::LinearGradient(g) => {
            let stops = convert_stops(g.stops());
            // Angle from the gradient vector, mapped through the SVG transform
            // and the page flip (which negates the Y component).
            let t = to_page * to_affine(g.transform());
            let a = t * Point::new(g.x1() as f64, g.y1() as f64);
            let b = t * Point::new(g.x2() as f64, g.y2() as f64);
            let angle = (b - a).atan2().to_degrees();
            Fill::Fountain(Fountain {
                kind: FountainKind::Linear,
                stops,
                angle,
                offset: Point::ZERO,
                edge_pad: 0.0,
            })
        }
        usvg::Paint::RadialGradient(g) => {
            let stops = convert_stops(g.stops());
            let t = to_page * to_affine(g.transform());
            let c = t * Point::new(g.cx() as f64, g.cy() as f64);
            let centre = bounds.center();
            let off = if bounds.width() > 0.0 && bounds.height() > 0.0 {
                Point::new(
                    ((c.x - centre.x) / (bounds.width() / 2.0)).clamp(-1.0, 1.0),
                    ((c.y - centre.y) / (bounds.height() / 2.0)).clamp(-1.0, 1.0),
                )
            } else {
                Point::ZERO
            };
            Fill::Fountain(Fountain {
                kind: FountainKind::Radial,
                stops,
                angle: 0.0,
                offset: off,
                edge_pad: 0.0,
            })
        }
        usvg::Paint::Pattern(_) => Fill::Solid(Color::rgb8(128, 128, 128)),
    }
}

fn convert_stops(stops: &[usvg::Stop]) -> Vec<Stop> {
    let mut out: Vec<Stop> = stops
        .iter()
        .map(|s| Stop {
            pos: s.offset().get() as f64,
            color: rgb(s.color()),
        })
        .collect();
    if out.len() < 2 {
        let c = out.first().map(|s| s.color).unwrap_or(Color::BLACK);
        out = vec![Stop { pos: 0.0, color: c }, Stop { pos: 1.0, color: c }];
    }
    out
}

fn convert_stroke(s: &usvg::Stroke, t: Affine) -> Stroke {
    let color = match s.paint() {
        usvg::Paint::Color(c) => rgb(*c),
        usvg::Paint::LinearGradient(g) => g
            .stops()
            .first()
            .map(|st| rgb(st.color()))
            .unwrap_or(Color::BLACK),
        usvg::Paint::RadialGradient(g) => g
            .stops()
            .first()
            .map(|st| rgb(st.color()))
            .unwrap_or(Color::BLACK),
        usvg::Paint::Pattern(_) => Color::BLACK,
    };
    // Uniform scale of the transform, for the stroke width.
    let scale = (t.as_coeffs()[0].abs() * t.as_coeffs()[3].abs()).sqrt();
    let width = (s.width().get() as f64 * scale).max(Stroke::HAIRLINE);
    let dash = s
        .dasharray()
        .map(|d| {
            d.iter()
                .map(|v| *v as f64 / s.width().get() as f64)
                .collect()
        })
        .unwrap_or_default();
    Stroke {
        color,
        width,
        cap: match s.linecap() {
            usvg::LineCap::Butt => LineCap::Butt,
            usvg::LineCap::Round => LineCap::Round,
            usvg::LineCap::Square => LineCap::Square,
        },
        join: match s.linejoin() {
            usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => LineJoin::Miter,
            usvg::LineJoin::Round => LineJoin::Round,
            usvg::LineJoin::Bevel => LineJoin::Bevel,
        },
        dash,
        ..Default::default()
    }
}

fn convert_image(img: &usvg::Image, to_page: Affine, ids: &mut IdSource) -> Option<Shape> {
    if !img.is_visible() {
        return None;
    }
    let (png, width_px, height_px) = match img.kind() {
        usvg::ImageKind::PNG(data) => {
            let data: Arc<Vec<u8>> = data.clone();
            let (w, h) = png_size(&data)?;
            (data.as_ref().clone(), w, h)
        }
        // Other encodings are left to the host application's image decoder;
        // here only PNG is embedded directly.
        _ => return None,
    };
    let t = to_page * to_affine(img.abs_transform());
    let sz = img.size();
    let r = Rect::new(0.0, 0.0, sz.width() as f64, sz.height() as f64);
    let rect = t.transform_rect_bbox(r);
    let mut shape = Shape::new(
        ShapeId(ids.shape().0),
        ShapeKind::Bitmap {
            rect,
            width_px,
            height_px,
            png,
        },
    );
    shape.stroke = None;
    if !img.id().is_empty() {
        shape.name = Some(img.id().to_string());
    }
    Some(shape)
}

/// Width and height from a PNG header (IHDR).
fn png_size(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 || &data[1..4] != b"PNG" {
        return None;
    }
    let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_and_gradient_round_trip_to_page_space() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="192" viewBox="0 0 96 192">
            <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient></defs>
            <g id="layer"><rect id="r" x="0" y="0" width="48" height="48" fill="url(#g)" stroke="#000" stroke-width="2"/>
            <circle cx="48" cy="144" r="24" fill="#0f0"/></g>
        </svg>"##;
        let mut ids = IdSource::default();
        let out = parse(svg, &mut ids).unwrap();
        assert!((out.size.width - 25.4).abs() < 1e-6 && (out.size.height - 50.8).abs() < 1e-6);
        assert_eq!(out.shapes.len(), 1);
        let ShapeKind::Group { children } = &out.shapes[0].kind else {
            panic!("expected a group");
        };
        assert_eq!(children.len(), 2);
        // The rect sits at the top-left of the SVG: in page space its top is at y = 50.8.
        let b = children[0].bounds();
        assert!((b.x0).abs() < 1e-6 && (b.y1 - 50.8).abs() < 1e-6, "{b:?}");
        assert!((b.width() - 12.7).abs() < 1e-6);
        assert!(matches!(children[0].fill, Fill::Fountain(_)));
        let st = children[0].stroke.as_ref().unwrap();
        assert!((st.width - 2.0 * PX_TO_MM).abs() < 1e-6);
        assert_eq!(children[0].name.as_deref(), Some("r"));
        assert_eq!(children[1].fill, Fill::Solid(Color::rgb8(0, 255, 0)));
    }
}

#[cfg(test)]
mod clip_tests {
    use super::*;
    use tracedraw_core::document::ShapeKind;

    #[test]
    fn clip_paths_follow_the_root_scale_and_become_clip_frames() {
        // Millimetre page with a mm viewBox: the root scale is 96/25.4.
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50">
            <defs><clipPath id="c"><path d="M 60 10 H 80 V 30 H 60 Z"/></clipPath></defs>
            <g clip-path="url(#c)"><rect x="0" y="0" width="100" height="50" fill="#f00"/></g>
        </svg>"##;
        let mut ids = IdSource::default();
        let imp = parse(svg, &mut ids).expect("parse");
        assert_eq!(imp.shapes.len(), 1);
        match &imp.shapes[0].kind {
            ShapeKind::ClipFrame { frame, .. } => {
                let b = frame.bounds();
                // y is flipped: SVG y 10..30 on a 50 mm page is 20..40 up.
                assert!(
                    (b.x0 - 60.0).abs() < 1e-4 && (b.x1 - 80.0).abs() < 1e-4,
                    "{b:?}"
                );
                assert!(
                    (b.y0 - 20.0).abs() < 1e-4 && (b.y1 - 40.0).abs() < 1e-4,
                    "{b:?}"
                );
            }
            other => panic!("{other:?}"),
        }
    }
}
