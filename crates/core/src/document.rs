//! The document model: pages, layers and shapes. Pure data, serde-friendly,
//! no behaviour beyond lookups and bounds. Mutation goes through commands.

use crate::geometry::{self, Affine, BezPath, Rect, Shape as _};
use crate::id::{IdSource, LayerId, PageId, ShapeId};
use crate::style::{Fill, Stroke};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// Hook for the text engine. The core cannot depend on fonts, so the app
/// registers a function that turns spans into glyph outlines (baseline at
/// the origin, mm). Without it text is drawn as a box.
pub mod text_outline {
    use super::TextSpan;
    use crate::geometry::BezPath;
    use std::sync::OnceLock;

    type Outliner = fn(&[TextSpan]) -> BezPath;
    static OUTLINER: OnceLock<Outliner> = OnceLock::new();

    pub fn set(f: Outliner) {
        let _ = OUTLINER.set(f);
    }

    pub fn outline(spans: &[TextSpan]) -> Option<BezPath> {
        OUTLINER.get().map(|f| f(spans))
    }
}

/// Paper sizes in millimetres.
pub mod paper {
    use crate::geometry::Size;
    pub const A4: Size = Size::new(210.0, 297.0);
    pub const A3: Size = Size::new(297.0, 420.0);
    pub const LETTER: Size = Size::new(215.9, 279.4);
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ShapeKind {
    /// Rectangle in local space; `radius` rounds the corners (mm).
    Rect { rect: Rect, radius: f64 },
    /// Ellipse inscribed in `rect`, in local space.
    Ellipse { rect: Rect },
    /// Polygon or star inscribed in `rect`.
    Polygon {
        rect: Rect,
        points: u32,
        sharpness: f64,
    },
    /// Free path (lines and cubic Beziers) in local space.
    Path { path: BezPath, closed: bool },
    /// Artistic text (the single-line text object).
    Text {
        spans: Vec<TextSpan>,
        origin: crate::geometry::Point,
    },
    /// A group of child shapes.
    Group { children: Vec<Shape> },
    /// A bitmap, PNG-encoded, placed in `rect` (local space).
    Bitmap {
        rect: Rect,
        width_px: u32,
        height_px: u32,
        #[serde(with = "png_bytes")]
        png: Vec<u8>,
    },
}

/// PNG bytes as base64 in the JSON format.
mod png_bytes {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(v))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextSpan {
    pub text: String,
    pub font_family: String,
    /// Font size in points.
    pub size_pt: f64,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    pub id: ShapeId,
    pub name: Option<String>,
    pub kind: ShapeKind,
    /// Local-to-page transform.
    pub transform: Affine,
    pub fill: Fill,
    pub stroke: Option<Stroke>,
    pub visible: bool,
    pub locked: bool,
    /// 1.0 = opaque, 0.0 = invisible (the uniform transparency).
    #[serde(default = "one")]
    pub opacity: f64,
}

fn one() -> f64 {
    1.0
}

impl Shape {
    pub fn new(id: ShapeId, kind: ShapeKind) -> Self {
        Shape {
            id,
            name: None,
            kind,
            transform: Affine::IDENTITY,
            fill: Fill::None,
            stroke: Some(Stroke::default()),
            visible: true,
            locked: false,
            opacity: 1.0,
        }
    }

    /// Outline of the shape in local coordinates. Text and groups return an
    /// approximation (bounds box) until the type engine exists.
    pub fn local_path(&self) -> BezPath {
        match &self.kind {
            ShapeKind::Rect { rect, radius } => geometry::rect_path(*rect, *radius),
            ShapeKind::Ellipse { rect } => geometry::ellipse_path(*rect),
            ShapeKind::Polygon {
                rect,
                points,
                sharpness,
            } => geometry::polygon_path(*rect, *points, *sharpness),
            ShapeKind::Path { path, .. } => path.clone(),
            ShapeKind::Text { spans, origin } => {
                if let Some(p) = text_outline::outline(spans) {
                    return Affine::translate(origin.to_vec2()) * p;
                }
                // No text engine registered: rough box, 0.5 em per character.
                let size_mm: f64 =
                    spans.iter().map(|s| s.size_pt).fold(0.0, f64::max) * 25.4 / 72.0;
                let chars: usize = spans.iter().map(|s| s.text.chars().count()).sum();
                let w = chars as f64 * size_mm * 0.5;
                Rect::new(origin.x, origin.y, origin.x + w, origin.y + size_mm).to_path(0.01)
            }
            ShapeKind::Group { children } => {
                let mut path = BezPath::new();
                for c in children {
                    path.extend(c.page_path());
                }
                path
            }
            ShapeKind::Bitmap { rect, .. } => rect.to_path(0.01),
        }
    }

    /// Outline in page coordinates (transform applied).
    pub fn page_path(&self) -> BezPath {
        self.transform * self.local_path()
    }

    /// Axis-aligned bounds in page coordinates, ignoring outline width.
    pub fn bounds(&self) -> Rect {
        self.page_path().bounding_box()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub printable: bool,
    pub locked: bool,
    /// Bottom to top drawing order.
    pub shapes: Vec<Shape>,
}

impl Layer {
    pub fn new(id: LayerId, name: impl Into<String>) -> Self {
        Layer {
            id,
            name: name.into(),
            visible: true,
            printable: true,
            locked: false,
            shapes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: PageId,
    pub name: String,
    /// Page size in millimetres.
    pub size: crate::geometry::Size,
    /// Bottom to top drawing order.
    pub layers: Vec<Layer>,
}

impl Page {
    pub fn rect(&self) -> Rect {
        Rect::from_origin_size((0.0, 0.0), self.size)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub title: String,
    pub pages: Vec<Page>,
    ids: IdSource,
}

impl Default for Document {
    fn default() -> Self {
        Document::new("Untitled", paper::A4)
    }
}

impl Document {
    /// A document with one page and one layer.
    pub fn new(title: impl Into<String>, size: crate::geometry::Size) -> Self {
        let mut ids = IdSource::default();
        let page = Page {
            id: ids.page(),
            name: "Page 1".into(),
            size,
            layers: vec![Layer::new(ids.layer(), "Layer 1")],
        };
        Document {
            title: title.into(),
            pages: vec![page],
            ids,
        }
    }

    pub fn ids_mut(&mut self) -> &mut IdSource {
        &mut self.ids
    }

    pub fn ids(&self) -> &IdSource {
        &self.ids
    }

    /// Replace the id source. Used by the engine so undo never rewinds ids,
    /// which would let a new object reuse the id of an undone one.
    pub fn set_ids(&mut self, ids: IdSource) {
        self.ids = ids;
    }

    pub fn page(&self, id: PageId) -> Result<&Page> {
        self.pages
            .iter()
            .find(|p| p.id == id)
            .ok_or(Error::PageNotFound(id))
    }

    pub fn page_mut(&mut self, id: PageId) -> Result<&mut Page> {
        self.pages
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(Error::PageNotFound(id))
    }

    pub fn layer(&self, id: LayerId) -> Result<&Layer> {
        self.pages
            .iter()
            .flat_map(|p| &p.layers)
            .find(|l| l.id == id)
            .ok_or(Error::LayerNotFound(id))
    }

    pub fn layer_mut(&mut self, id: LayerId) -> Result<&mut Layer> {
        self.pages
            .iter_mut()
            .flat_map(|p| &mut p.layers)
            .find(|l| l.id == id)
            .ok_or(Error::LayerNotFound(id))
    }

    /// Find a top-level shape and the layer holding it.
    pub fn shape(&self, id: ShapeId) -> Result<(&Layer, &Shape)> {
        for layer in self.pages.iter().flat_map(|p| &p.layers) {
            if let Some(s) = layer.shapes.iter().find(|s| s.id == id) {
                return Ok((layer, s));
            }
        }
        Err(Error::ShapeNotFound(id))
    }

    pub fn shape_mut(&mut self, id: ShapeId) -> Result<&mut Shape> {
        for layer in self.pages.iter_mut().flat_map(|p| &mut p.layers) {
            if let Some(s) = layer.shapes.iter_mut().find(|s| s.id == id) {
                return Ok(s);
            }
        }
        Err(Error::ShapeNotFound(id))
    }

    /// Position of a shape: (layer id, index inside the layer).
    pub fn locate(&self, id: ShapeId) -> Result<(LayerId, usize)> {
        for layer in self.pages.iter().flat_map(|p| &p.layers) {
            if let Some(i) = layer.shapes.iter().position(|s| s.id == id) {
                return Ok((layer.id, i));
            }
        }
        Err(Error::ShapeNotFound(id))
    }

    /// Union of all shape bounds on a page, or None when the page is empty.
    pub fn content_bounds(&self, page: PageId) -> Result<Option<Rect>> {
        let p = self.page(page)?;
        Ok(p.layers
            .iter()
            .flat_map(|l| &l.shapes)
            .map(Shape::bounds)
            .reduce(|a, b| a.union(b)))
    }

    /// Serialize to the native `.tdraw` JSON format.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(s: &str) -> Result<Self> {
        Ok(serde_json::from_str(s)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn json_round_trip() {
        let mut doc = Document::default();
        let id = doc.ids_mut().shape();
        let page = doc.pages[0].id;
        let layer = doc.pages[0].layers[0].id;
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 50.0, 30.0),
                radius: 2.0,
            },
        );
        s.transform = Affine::translate((5.0, 5.0));
        doc.layer_mut(layer).unwrap().shapes.push(s);
        let json = doc.to_json().unwrap();
        let back = Document::from_json(&json).unwrap();
        assert_eq!(doc, back);
        let b = back.content_bounds(page).unwrap().unwrap();
        assert!((b.x0 - 15.0).abs() < 1e-6 && (b.y1 - 35.0).abs() < 1e-6);
        assert_eq!(back.locate(id).unwrap(), (layer, 0));
        let _ = Point::ZERO;
    }
}
