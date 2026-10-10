//! Commands: every mutation of a document is one of these. The UI, the CLI
//! and the control channel all build commands and hand them to the
//! [`crate::Engine`], which applies them and records history.

use crate::document::{
    ColorStyle, Guide, Layer, MasterScope, Metadata, ObjectStyle, Page, Shadow, Shape, ShapeKind,
    Symbol,
};
use crate::geometry::{Affine, BezPath, PathEl, Size};
use crate::id::{LayerId, PageId, ShapeId};
use crate::style::{Fill, Stroke};
use crate::Color;
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
    AddMasterLayer {
        name: String,
        scope: MasterScope,
    },
    SetLayerScope {
        layer: LayerId,
        scope: MasterScope,
    },
    SetLayerPrintable {
        layer: LayerId,
        printable: bool,
    },
    SetPageBackground {
        page: PageId,
        background: Option<Fill>,
    },
    SetMetadata {
        metadata: Metadata,
    },
    SetObjectStyles {
        styles: Vec<ObjectStyle>,
    },
    SetColorStyles {
        styles: Vec<ColorStyle>,
    },
    SetDocumentPalette {
        colors: Vec<Color>,
    },
    AddSymbol {
        symbol: Symbol,
    },
    /// Move a page to another index (Page Sorter drag).
    MovePage {
        page: PageId,
        to: usize,
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
    /// Hide or show objects (Object > Hide); hidden objects are not drawn
    /// and cannot be selected on the canvas, only in the Objects docker.
    SetVisible {
        shapes: Vec<ShapeId>,
        visible: bool,
    },
    /// Replace the live effect stack of a shape.
    SetEffects {
        shape: ShapeId,
        effects: Vec<crate::live::Effect>,
    },
    SetOverprint {
        shapes: Vec<ShapeId>,
        fill: Option<bool>,
        outline: Option<bool>,
    },
    SetLink {
        shapes: Vec<ShapeId>,
        link: Option<String>,
    },
    SetWrapText {
        shapes: Vec<ShapeId>,
        wrap: bool,
    },
    SetObjectData {
        shape: ShapeId,
        data: Vec<(String, String)>,
    },
    /// Move `shapes` directly in front of (or behind) `reference` in its layer.
    OrderRelative {
        shapes: Vec<ShapeId>,
        reference: ShapeId,
        in_front: bool,
    },
    /// Reverse the stacking order of the given shapes among themselves.
    ReverseOrder {
        shapes: Vec<ShapeId>,
    },
    /// Merge several shapes into one curve (even-odd fill), like Ctrl+L.
    Combine {
        shapes: Vec<ShapeId>,
    },
    /// Split a curve into one object per subpath, like Ctrl+K.
    BreakApart {
        shape: ShapeId,
    },
    SetShadow {
        shapes: Vec<ShapeId>,
        shadow: Option<Shadow>,
    },
    /// Place objects inside a frame object (ClipFrame). The frame keeps its
    /// id; the contents keep their page positions.
    PlaceInside {
        contents: Vec<ShapeId>,
        frame: ShapeId,
    },
    /// Take the contents out of a ClipFrame, leaving the frame as a plain object.
    ExtractContents {
        clip: ShapeId,
    },
    RenamePage {
        page: PageId,
        name: String,
    },
    /// Copy a page (layers, shapes with fresh ids, guides) right after it.
    DuplicatePage {
        page: PageId,
    },
    RenameLayer {
        layer: LayerId,
        name: String,
    },
    /// Move a layer to another index within its page (z-order).
    ReorderLayer {
        layer: LayerId,
        index: usize,
    },
    AddGuide {
        page: PageId,
        guide: Guide,
    },
    MoveGuide {
        page: PageId,
        index: usize,
        guide: Guide,
    },
    DeleteGuide {
        page: PageId,
        index: usize,
    },
    /// Replace all guidelines of a page (several deleted, locked or
    /// restyled at once, presets applied).
    SetGuides {
        page: PageId,
        guides: Vec<Guide>,
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
            Command::AddMasterLayer { .. } => "New Master Layer",
            Command::SetLayerScope { .. } => "Master Layer Scope",
            Command::SetLayerPrintable { .. } => "Layer Printable",
            Command::SetPageBackground { .. } => "Page Background",
            Command::SetMetadata { .. } => "Document Properties",
            Command::SetObjectStyles { .. } => "Object Styles",
            Command::SetColorStyles { .. } => "Color Styles",
            Command::SetDocumentPalette { .. } => "Document Palette",
            Command::AddSymbol { .. } => "New Symbol",
            Command::MovePage { .. } => "Move Page",
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
            Command::SetVisible { visible: false, .. } => "Hide Object",
            Command::SetVisible { .. } => "Show Object",
            Command::SetEffects { effects, .. } if effects.is_empty() => "Clear Effects",
            Command::SetEffects { .. } => "Effect",
            Command::SetOverprint { .. } => "Overprint",
            Command::SetLink { .. } => "Hyperlink",
            Command::SetWrapText { .. } => "Wrap Paragraph Text",
            Command::SetObjectData { .. } => "Object Data",
            Command::OrderRelative { in_front: true, .. } => "In Front Of",
            Command::OrderRelative { .. } => "Behind",
            Command::ReverseOrder { .. } => "Reverse Order",
            Command::Combine { .. } => "Combine",
            Command::BreakApart { .. } => "Break Apart",
            Command::PlaceInside { .. } => "ClipFrame",
            Command::ExtractContents { .. } => "Extract Contents",
            Command::RenamePage { .. } => "Rename Page",
            Command::DuplicatePage { .. } => "Duplicate Page",
            Command::RenameLayer { .. } => "Rename Layer",
            Command::ReorderLayer { .. } => "Reorder Layer",
            Command::SetShadow {
                shadow: Some(_), ..
            } => "Drop Shadow",
            Command::SetShadow { .. } => "Clear Drop Shadow",
            Command::AddGuide { .. } => "Add Guideline",
            Command::MoveGuide { .. } => "Move Guideline",
            Command::DeleteGuide { .. } => "Delete Guideline",
            Command::SetGuides { .. } => "Guidelines",
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
                    guides: Vec::new(),
                    background: None,
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
            Command::AddMasterLayer { name, scope } => {
                let id = doc.ids_mut().layer();
                doc.master.push(Layer::master(id, name.clone(), *scope));
            }
            Command::SetLayerScope { layer, scope } => {
                doc.layer_mut(*layer)?.scope = *scope;
            }
            Command::SetLayerPrintable { layer, printable } => {
                doc.layer_mut(*layer)?.printable = *printable;
            }
            Command::SetPageBackground { page, background } => {
                doc.page_mut(*page)?.background = background.clone();
            }
            Command::SetMetadata { metadata } => {
                doc.metadata = metadata.clone();
            }
            Command::SetObjectStyles { styles } => {
                doc.object_styles = styles.clone();
            }
            Command::SetColorStyles { styles } => {
                doc.color_styles = styles.clone();
            }
            Command::SetDocumentPalette { colors } => {
                doc.palette = colors.clone();
            }
            Command::AddSymbol { symbol } => {
                doc.symbols.push(symbol.clone());
            }
            Command::MovePage { page, to } => {
                let from = doc
                    .pages
                    .iter()
                    .position(|p| p.id == *page)
                    .ok_or(Error::PageNotFound(*page))?;
                let p = doc.pages.remove(from);
                let to = (*to).min(doc.pages.len());
                doc.pages.insert(to, p);
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
            Command::SetVisible { shapes, visible } => {
                for id in shapes {
                    doc.shape(*id)?;
                }
                for id in shapes {
                    doc.shape_mut(*id)?.visible = *visible;
                }
            }
            Command::SetEffects { shape, effects } => {
                let s = doc
                    .shape_mut(*shape)
                    .map_err(|_| Error::ShapeNotFound(*shape))?;
                s.effects = effects.clone();
            }
            Command::SetOverprint {
                shapes,
                fill,
                outline,
            } => {
                for id in shapes {
                    doc.shape(*id)?;
                }
                for id in shapes {
                    let s = doc.shape_mut(*id)?;
                    if let Some(f) = fill {
                        s.overprint_fill = *f;
                    }
                    if let Some(o) = outline {
                        s.overprint_outline = *o;
                    }
                }
            }
            Command::SetWrapText { shapes, wrap } => {
                for id in shapes {
                    doc.shape(*id)?;
                }
                for id in shapes {
                    doc.shape_mut(*id)?.wrap_text = *wrap;
                }
            }
            Command::SetLink { shapes, link } => {
                for id in shapes {
                    doc.shape(*id)?;
                }
                for id in shapes {
                    doc.shape_mut(*id)?.link = link.clone();
                }
            }
            Command::SetObjectData { shape, data } => {
                doc.shape_mut(*shape)?.data = data.clone();
            }
            Command::OrderRelative {
                shapes,
                reference,
                in_front,
            } => {
                let (layer, _) = doc.locate(*reference)?;
                let mut moved = Vec::new();
                for id in shapes {
                    if *id == *reference {
                        continue;
                    }
                    let (l, i) = doc.locate(*id)?;
                    if l != layer {
                        continue;
                    }
                    moved.push(doc.layer_mut(l)?.shapes.remove(i));
                }
                let (_, ri) = doc.locate(*reference)?;
                let at = if *in_front { ri + 1 } else { ri };
                let target = doc.layer_mut(layer)?;
                for (k, s) in moved.into_iter().enumerate() {
                    target.shapes.insert(at + k, s);
                }
            }
            Command::ReverseOrder { shapes } => {
                let mut slots: Vec<(LayerId, usize)> = Vec::new();
                for id in shapes {
                    slots.push(doc.locate(*id)?);
                }
                // Per layer, swap the shapes at the chosen indices in reverse.
                let mut by_layer: std::collections::BTreeMap<LayerId, Vec<usize>> =
                    Default::default();
                for (l, i) in slots {
                    by_layer.entry(l).or_default().push(i);
                }
                for (l, mut idx) in by_layer {
                    idx.sort_unstable();
                    let layer = doc.layer_mut(l)?;
                    let n = idx.len();
                    for k in 0..n / 2 {
                        layer.shapes.swap(idx[k], idx[n - 1 - k]);
                    }
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
                // Result takes the attributes of the topmost shape.
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
            Command::PlaceInside { contents, frame } => {
                if contents.contains(frame) {
                    return Ok(());
                }
                doc.shape(*frame)?;
                for id in contents {
                    doc.shape(*id)?;
                }
                let mut inner = Vec::new();
                for id in contents {
                    let (lid, i) = doc.locate(*id)?;
                    inner.push(doc.layer_mut(lid)?.shapes.remove(i));
                }
                let (flid, fi) = doc.locate(*frame)?;
                let frame_shape = doc.layer_mut(flid)?.shapes.remove(fi);
                let mut clip = Shape::new(
                    *frame,
                    ShapeKind::ClipFrame {
                        frame: Box::new(frame_shape.clone()),
                        contents: inner,
                    },
                );
                clip.name = frame_shape.name.clone();
                clip.opacity = frame_shape.opacity;
                let target = &mut doc.layer_mut(flid)?.shapes;
                target.insert(fi.min(target.len()), clip);
            }
            Command::ExtractContents { clip } => {
                let (lid, i) = doc.locate(*clip)?;
                let s = doc.layer_mut(lid)?.shapes.remove(i);
                match s.kind {
                    ShapeKind::ClipFrame { frame, contents } => {
                        let layer = doc.layer_mut(lid)?;
                        let mut f = *frame;
                        f.id = *clip;
                        layer.shapes.insert(i, f);
                        for (k, c) in contents.into_iter().enumerate() {
                            layer.shapes.insert(i + 1 + k, c);
                        }
                    }
                    other => doc
                        .layer_mut(lid)?
                        .shapes
                        .insert(i, Shape { kind: other, ..s }),
                }
            }
            Command::RenamePage { page, name } => doc.page_mut(*page)?.name = name.clone(),
            Command::DuplicatePage { page } => {
                let idx = doc
                    .pages
                    .iter()
                    .position(|p| p.id == *page)
                    .ok_or(Error::PageNotFound(*page))?;
                let src = doc.pages[idx].clone();
                let new_id = doc.ids_mut().page();
                let mut layers = Vec::with_capacity(src.layers.len());
                for l in &src.layers {
                    let lid = doc.ids_mut().layer();
                    let mut nl = Layer::new(lid, l.name.clone());
                    nl.visible = l.visible;
                    nl.printable = l.printable;
                    nl.locked = l.locked;
                    for s in &l.shapes {
                        nl.shapes.push(reid_shape(s, doc));
                    }
                    layers.push(nl);
                }
                let copy = Page {
                    id: new_id,
                    name: format!("{} (copy)", src.name),
                    size: src.size,
                    layers,
                    guides: src.guides.clone(),
                    background: src.background.clone(),
                };
                doc.pages.insert(idx + 1, copy);
            }
            Command::RenameLayer { layer, name } => doc.layer_mut(*layer)?.name = name.clone(),
            Command::ReorderLayer { layer, index } => {
                let page = doc
                    .pages
                    .iter_mut()
                    .find(|p| p.layers.iter().any(|l| l.id == *layer))
                    .ok_or(Error::LayerNotFound(*layer))?;
                let i = page
                    .layers
                    .iter()
                    .position(|l| l.id == *layer)
                    .ok_or(Error::LayerNotFound(*layer))?;
                let l = page.layers.remove(i);
                let index = (*index).min(page.layers.len());
                page.layers.insert(index, l);
            }
            Command::SetShadow { shapes, shadow } => {
                for id in shapes {
                    doc.shape(*id)?;
                }
                for id in shapes {
                    doc.shape_mut(*id)?.shadow = *shadow;
                }
            }
            Command::AddGuide { page, guide } => doc.page_mut(*page)?.guides.push(*guide),
            Command::MoveGuide { page, index, guide } => {
                let p = doc.page_mut(*page)?;
                if let Some(g) = p.guides.get_mut(*index) {
                    *g = *guide;
                }
            }
            Command::DeleteGuide { page, index } => {
                let p = doc.page_mut(*page)?;
                if *index < p.guides.len() {
                    p.guides.remove(*index);
                }
            }
            Command::SetGuides { page, guides } => doc.page_mut(*page)?.guides = guides.clone(),
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

/// Deep copy of a shape with fresh ids (groups included).
fn reid_shape(s: &Shape, doc: &mut Document) -> Shape {
    let mut c = s.clone();
    c.id = doc.ids_mut().shape();
    if let ShapeKind::Group { children } = &mut c.kind {
        let copies: Vec<Shape> = children.iter().map(|ch| reid_shape(ch, doc)).collect();
        *children = copies;
    }
    c
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
    fn duplicate_page_copies_shapes_with_new_ids() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let page = doc.pages[0].id;
        let a = rect(&mut doc, 0.0);
        let ia = a.id;
        Command::AddShape { layer, shape: a }
            .apply(&mut doc)
            .unwrap();
        Command::DuplicatePage { page }.apply(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 2);
        let copy = &doc.pages[1];
        assert_eq!(copy.layers[0].shapes.len(), 1);
        assert_ne!(copy.layers[0].shapes[0].id, ia);
        assert_ne!(copy.layers[0].id, layer);
    }

    #[test]
    fn clip_frame_place_and_extract() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let a = rect(&mut doc, 0.0);
        let b = rect(&mut doc, 5.0);
        let (ia, ib) = (a.id, b.id);
        Command::AddShape { layer, shape: a }
            .apply(&mut doc)
            .unwrap();
        Command::AddShape { layer, shape: b }
            .apply(&mut doc)
            .unwrap();
        Command::PlaceInside {
            contents: vec![ia],
            frame: ib,
        }
        .apply(&mut doc)
        .unwrap();
        assert_eq!(doc.layer(layer).unwrap().shapes.len(), 1);
        assert!(matches!(
            doc.shape(ib).unwrap().1.kind,
            ShapeKind::ClipFrame { .. }
        ));
        Command::ExtractContents { clip: ib }
            .apply(&mut doc)
            .unwrap();
        assert_eq!(doc.layer(layer).unwrap().shapes.len(), 2);
        assert!(doc.shape(ia).is_ok());
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
