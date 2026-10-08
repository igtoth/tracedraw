//! Commands: every mutation of a document is one of these. The UI, the CLI
//! and the control channel all build commands and hand them to the
//! [`crate::Engine`], which applies them and records history.

use crate::document::{Layer, Page, Shape, ShapeKind};
use crate::geometry::{Affine, BezPath, PathEl, Size};
use crate::id::{LayerId, PageId, ShapeId};
use crate::style::{Fill, Stroke};
use crate::{Document, Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    // Document
    SetTitle {
        title: String,
    },

    // Pages
    AddPage {
        name: Option<String>,
        size: Size,
    },
    DeletePage {
        page: PageId,
    },
    ResizePage {
        page: PageId,
        size: Size,
    },

    // Layers
    AddLayer {
        page: PageId,
        name: String,
    },
    DeleteLayer {
        layer: LayerId,
    },
    SetLayerVisible {
        layer: LayerId,
        visible: bool,
    },
    SetLayerLocked {
        layer: LayerId,
        locked: bool,
    },

    // Shapes
    /// Insert a shape at the top of a layer. The shape's id must come from
    /// `Document::ids_mut().shape()` so it stays unique.
    AddShape {
        layer: LayerId,
        shape: Shape,
    },
    DeleteShapes {
        shapes: Vec<ShapeId>,
    },
    /// Pre-multiply the given transform onto each shape's transform
    /// (i.e. apply it in page space).
    TransformShapes {
        shapes: Vec<ShapeId>,
        transform: Affine,
    },
    SetFill {
        shapes: Vec<ShapeId>,
        fill: Fill,
    },
    SetStroke {
        shapes: Vec<ShapeId>,
        stroke: Option<Stroke>,
    },
    SetShapeKind {
        shape: ShapeId,
        kind: ShapeKind,
    },
    SetShapeName {
        shape: ShapeId,
        name: Option<String>,
    },
    /// Move a shape to another layer and position (z-order).
    Reorder {
        shape: ShapeId,
        layer: LayerId,
        index: usize,
    },
    Group {
        shapes: Vec<ShapeId>,
    },
    Ungroup {
        group: ShapeId,
    },
    SetOpacity {
        shapes: Vec<ShapeId>,
        opacity: f64,
    },
    SetLocked {
        shapes: Vec<ShapeId>,
        locked: bool,
    },
    /// Merge several shapes into one curve (even-odd fill), like Ctrl+L.
    Combine {
        shapes: Vec<ShapeId>,
    },
    /// Split a curve into one object per subpath, like Ctrl+K.
    BreakApart {
        shape: ShapeId,
    },
}

impl Command {
    /// Short label for the History panel and the Edit menu ("Undo Move").
    pub fn label(&self) -> &'static str {
        match self {
            Command::SetTitle { .. } => "Rename Document",
            Command::AddPage { .. } => "Insert Page",
            Command::DeletePage { .. } => "Delete Page",
            Command::ResizePage { .. } => "Resize Page",
            Command::AddLayer { .. } => "New Layer",
            Command::DeleteLayer { .. } => "Delete Layer",
            Command::SetLayerVisible { .. } => "Layer Visibility",
            Command::SetLayerLocked { .. } => "Lock Layer",
            Command::AddShape { .. } => "Create Object",
            Command::DeleteShapes { .. } => "Delete",
            Command::TransformShapes { .. } => "Transform",
            Command::SetFill { .. } => "Fill",
            Command::SetStroke { .. } => "Outline",
            Command::SetShapeKind { .. } => "Edit Object",
            Command::SetShapeName { .. } => "Rename Object",
            Command::Reorder { .. } => "Order",
            Command::Group { .. } => "Group",
            Command::Ungroup { .. } => "Ungroup",
            Command::SetOpacity { .. } => "Transparency",
            Command::SetLocked { locked: true, .. } => "Lock Object",
            Command::SetLocked { .. } => "Unlock Object",
            Command::Combine { .. } => "Combine",
            Command::BreakApart { .. } => "Break Apart",
        }
    }

    /// Apply the command to a document. Fails without touching the document
    /// when it refers to objects that do not exist.
    pub fn apply(&self, doc: &mut Document) -> Result<()> {
        match self {
            Command::SetTitle { title } => doc.title = title.clone(),

            Command::AddPage { name, size } => {
                let n = doc.pages.len() + 1;
                let page_id = doc.ids_mut().page();
                let layer_id = doc.ids_mut().layer();
                doc.pages.push(Page {
                    id: page_id,
                    name: name.clone().unwrap_or_else(|| format!("Page {n}")),
                    size: *size,
                    layers: vec![Layer::new(layer_id, "Layer 1")],
                });
            }
            Command::DeletePage { page } => {
                if doc.pages.len() <= 1 {
                    return Err(Error::LastPage);
                }
                let i = doc
                    .pages
                    .iter()
                    .position(|p| p.id == *page)
                    .ok_or(Error::PageNotFound(*page))?;
                doc.pages.remove(i);
            }
            Command::ResizePage { page, size } => doc.page_mut(*page)?.size = *size,

            Command::AddLayer { page, name } => {
                let id = doc.ids_mut().layer();
                doc.page_mut(*page)?
                    .layers
                    .push(Layer::new(id, name.clone()));
            }
            Command::DeleteLayer { layer } => {
                let page = doc
                    .pages
                    .iter_mut()
                    .find(|p| p.layers.iter().any(|l| l.id == *layer))
                    .ok_or(Error::LayerNotFound(*layer))?;
                if page.layers.len() <= 1 {
                    return Err(Error::LastLayer);
                }
                page.layers.retain(|l| l.id != *layer);
            }
            Command::SetLayerVisible { layer, visible } => {
                doc.layer_mut(*layer)?.visible = *visible
            }
            Command::SetLayerLocked { layer, locked } => doc.layer_mut(*layer)?.locked = *locked,

            Command::AddShape { layer, shape } => doc.layer_mut(*layer)?.shapes.push(shape.clone()),
            Command::DeleteShapes { shapes } => {
                for id in shapes {
                    doc.shape(*id)?;
                }
                for layer in doc.pages.iter_mut().flat_map(|p| &mut p.layers) {
                    layer.shapes.retain(|s| !shapes.contains(&s.id));
                }
            }
            Command::TransformShapes { shapes, transform } => {
                for id in shapes {
                    doc.shape(*id)?;
                }
                for id in shapes {
                    let s = doc.shape_mut(*id)?;
                    s.transform = *transform * s.transform;
                }
            }
            Command::SetFill { shapes, fill } => {
                for id in shapes {
                    doc.shape(*id)?;
                }
                for id in shapes {
                    doc.shape_mut(*id)?.fill = fill.clone();
                }
            }
            Command::SetStroke { shapes, stroke } => {
                for id in shapes {
                    doc.shape(*id)?;
                }
                for id in shapes {
                    doc.shape_mut(*id)?.stroke = stroke.clone();
                }
            }
            Command::SetShapeKind { shape, kind } => doc.shape_mut(*shape)?.kind = kind.clone(),
            Command::SetShapeName { shape, name } => doc.shape_mut(*shape)?.name = name.clone(),

            Command::Reorder {
                shape,
                layer,
                index,
            } => {
                let (from_layer, from_idx) = doc.locate(*shape)?;
                doc.layer(*layer)?;
                let s = doc.layer_mut(from_layer)?.shapes.remove(from_idx);
                let target = &mut doc.layer_mut(*layer)?.shapes;
                let index = (*index).min(target.len());
                target.insert(index, s);
            }

            Command::Group { shapes } => {
                if shapes.is_empty() {
                    return Ok(());
                }
                // The group lives in the layer of the topmost member, at the
                // position of the lowest member.
                let mut located: Vec<(ShapeId, LayerId, usize)> = shapes
                    .iter()
                    .map(|id| doc.locate(*id).map(|(l, i)| (*id, l, i)))
                    .collect::<Result<_>>()?;
                located.sort_by_key(|(_, _, i)| *i);
                let layer_id = located[0].1;
                let insert_at = located[0].2;
                let mut children = Vec::with_capacity(shapes.len());
                for (id, l, _) in &located {
                    let layer = doc.layer_mut(*l)?;
                    let i = layer
                        .shapes
                        .iter()
                        .position(|s| s.id == *id)
                        .ok_or(Error::ShapeNotFound(*id))?;
                    children.push(layer.shapes.remove(i));
                }
                let gid = doc.ids_mut().shape();
                let mut group = Shape::new(gid, ShapeKind::Group { children });
                group.stroke = None;
                let target = &mut doc.layer_mut(layer_id)?.shapes;
                target.insert(insert_at.min(target.len()), group);
            }
            Command::SetOpacity { shapes, opacity } => {
                for id in shapes {
                    doc.shape(*id)?;
                }
                for id in shapes {
                    doc.shape_mut(*id)?.opacity = opacity.clamp(0.0, 1.0);
                }
            }
            Command::SetLocked { shapes, locked } => {
                for id in shapes {
                    doc.shape(*id)?;
                }
                for id in shapes {
                    doc.shape_mut(*id)?.locked = *locked;
                }
            }
            Command::Combine { shapes } => {
                if shapes.len() < 2 {
                    return Ok(());
                }
                let mut located: Vec<(ShapeId, LayerId, usize)> = shapes
                    .iter()
                    .map(|id| doc.locate(*id).map(|(l, i)| (*id, l, i)))
                    .collect::<Result<_>>()?;
                located.sort_by_key(|(_, _, i)| *i);
                // Result takes the attributes of the topmost shape, like the editor.
                let top_id = located
                    .last()
                    .map(|(id, _, _)| *id)
                    .ok_or(Error::ShapeNotFound(shapes[0]))?;
                let top = doc.shape(top_id)?.1.clone();
                let layer_id = located[0].1;
                let insert_at = located[0].2;
                let mut path = BezPath::new();
                for (id, _, _) in &located {
                    let s = doc.shape(*id)?.1;
                    path.extend(s.page_path());
                }
                for (id, l, _) in &located {
                    let layer = doc.layer_mut(*l)?;
                    if let Some(i) = layer.shapes.iter().position(|s| s.id == *id) {
                        layer.shapes.remove(i);
                    }
                }
                let mut combined = Shape::new(top_id, ShapeKind::Path { path, closed: true });
                combined.fill = top.fill;
                combined.stroke = top.stroke;
                combined.opacity = top.opacity;
                let target = &mut doc.layer_mut(layer_id)?.shapes;
                target.insert(insert_at.min(target.len()), combined);
            }
            Command::BreakApart { shape } => {
                let (layer_id, idx) = doc.locate(*shape)?;
                let s = doc.shape(*shape)?.1.clone();
                let ShapeKind::Path { path, closed } = &s.kind else {
                    return Ok(());
                };
                let mut parts: Vec<BezPath> = Vec::new();
                let mut cur = BezPath::new();
                for el in path.elements() {
                    if let PathEl::MoveTo(_) = el {
                        if !cur.elements().is_empty() {
                            parts.push(std::mem::take(&mut cur));
                        }
                    }
                    cur.push(*el);
                }
                if !cur.elements().is_empty() {
                    parts.push(cur);
                }
                if parts.len() < 2 {
                    return Ok(());
                }
                let layer = doc.layer_mut(layer_id)?;
                layer.shapes.remove(idx);
                let n = parts.len();
                let mut ids: Vec<ShapeId> = (1..n).map(|_| doc.ids_mut().shape()).collect();
                ids.insert(0, *shape);
                let layer = doc.layer_mut(layer_id)?;
                for (k, (p, id)) in parts.into_iter().zip(ids).enumerate() {
                    let mut piece = Shape::new(
                        id,
                        ShapeKind::Path {
                            path: p,
                            closed: *closed,
                        },
                    );
                    piece.transform = s.transform;
                    piece.fill = s.fill.clone();
                    piece.stroke = s.stroke.clone();
                    piece.opacity = s.opacity;
                    layer.shapes.insert(idx + k, piece);
                }
            }
            Command::Ungroup { group } => {
                let (layer_id, idx) = doc.locate(*group)?;
                let layer = doc.layer_mut(layer_id)?;
                let g = layer.shapes.remove(idx);
                match g.kind {
                    ShapeKind::Group { children } => {
                        for (k, mut c) in children.into_iter().enumerate() {
                            c.transform = g.transform * c.transform;
                            layer.shapes.insert(idx + k, c);
                        }
                    }
                    other => {
                        // Not a group: put it back untouched.
                        layer.shapes.insert(idx, Shape { kind: other, ..g });
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;

    fn rect(doc: &mut Document, x: f64) -> Shape {
        let id = doc.ids_mut().shape();
        Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(x, 0.0, x + 10.0, 10.0),
                radius: 0.0,
            },
        )
    }

    #[test]
    fn group_and_ungroup_keep_page_positions() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let a = rect(&mut doc, 0.0);
        let b = rect(&mut doc, 20.0);
        let (ia, ib) = (a.id, b.id);
        Command::AddShape { layer, shape: a }
            .apply(&mut doc)
            .unwrap();
        Command::AddShape { layer, shape: b }
            .apply(&mut doc)
            .unwrap();
        Command::Group {
            shapes: vec![ia, ib],
        }
        .apply(&mut doc)
        .unwrap();
        assert_eq!(doc.layer(layer).unwrap().shapes.len(), 1);
        let gid = doc.layer(layer).unwrap().shapes[0].id;
        Command::TransformShapes {
            shapes: vec![gid],
            transform: Affine::translate((5.0, 5.0)),
        }
        .apply(&mut doc)
        .unwrap();
        Command::Ungroup { group: gid }.apply(&mut doc).unwrap();
        let shapes = &doc.layer(layer).unwrap().shapes;
        assert_eq!(shapes.len(), 2);
        let bb = shapes[1].bounds();
        assert!((bb.x0 - 25.0).abs() < 1e-9 && (bb.y0 - 5.0).abs() < 1e-9);
    }

    #[test]
    fn combine_then_break_apart_round_trips() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let a = rect(&mut doc, 0.0);
        let b = rect(&mut doc, 20.0);
        let (ia, ib) = (a.id, b.id);
        Command::AddShape { layer, shape: a }
            .apply(&mut doc)
            .unwrap();
        Command::AddShape { layer, shape: b }
            .apply(&mut doc)
            .unwrap();
        Command::Combine {
            shapes: vec![ia, ib],
        }
        .apply(&mut doc)
        .unwrap();
        assert_eq!(doc.layer(layer).unwrap().shapes.len(), 1);
        let combined = doc.layer(layer).unwrap().shapes[0].id;
        assert_eq!(combined, ib);
        Command::BreakApart { shape: combined }
            .apply(&mut doc)
            .unwrap();
        assert_eq!(doc.layer(layer).unwrap().shapes.len(), 2);
    }

    #[test]
    fn delete_unknown_shape_fails_cleanly() {
        let mut doc = Document::default();
        let err = Command::DeleteShapes {
            shapes: vec![ShapeId(999)],
        }
        .apply(&mut doc)
        .unwrap_err();
        assert!(matches!(err, Error::ShapeNotFound(_)));
    }
}
