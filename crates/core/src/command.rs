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
        /// The name of the page's first layer ("Layer 1" when none).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        layer: Option<String>,
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
    /// The CMYK profile the drawing carries (`None`: the default one).
    SetCmykProfile {
        profile: Option<crate::document::DocumentProfile>,
    },
    /// The node types chosen for curve `shape` (node element index to
    /// type), replacing the ones it had.
    SetNodeTypes {
        shape: ShapeId,
        types: std::collections::BTreeMap<usize, crate::nodes::NodeType>,
    },
    /// Store `shapes` as selection group `number` (0 to 9); no shapes
    /// removes the group.
    SetSelectionGroup {
        number: u8,
        shapes: Vec<ShapeId>,
    },
    /// Replace the Time Tracker records. This never becomes an undo step
    /// and does not mark the drawing changed (see [`Command::outside_history`]).
    SetTimeTracker {
        tracker: crate::document::TimeTracker,
    },
    /// Keep the view the drawing is saved with (#1134). Like
    /// [`Command::SetTimeTracker`] it never becomes an undo step and does
    /// not mark the drawing changed.
    SetSavedView {
        view: Option<crate::document::SavedView>,
    },
    /// Replace the review comments. Comments on pages that are gone are
    /// dropped; ids are kept, and repeated or missing (0) ones get a free
    /// number.
    SetComments {
        comments: Vec<crate::document::Comment>,
    },
    /// Replace the object styles. Styles without an id get a free one;
    /// objects linked to a style take its parts again, and links to a
    /// style that is gone are dropped.
    SetObjectStyles {
        styles: Vec<ObjectStyle>,
    },
    /// Link objects to an object style and give them its parts; `None`
    /// breaks their links and leaves their properties as they are.
    LinkObjectStyle {
        shapes: Vec<ShapeId>,
        style: Option<u32>,
    },
    /// Replace the fill and outline new objects get.
    SetObjectDefaults {
        defaults: crate::document::ObjectDefaults,
    },
    SetColorStyles {
        styles: Vec<ColorStyle>,
    },
    SetDocumentPalette {
        colors: Vec<Color>,
    },
    /// The drawing's spot inks, replacing the ones it had.
    SetInks {
        inks: Vec<crate::document::Ink>,
    },
    /// Draw clip frame `shape` as an opacity mask (`None`: a hard clip).
    SetOpacityMask {
        shape: ShapeId,
        mask: Option<crate::document::OpacityMask>,
    },
    /// Bitmap effects (and their picture) on a vector object; `None`
    /// removes them.
    SetRasterFx {
        shape: ShapeId,
        fx: Option<Box<crate::document::RasterFx>>,
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
    /// Copy only `fields` of `stroke` to each shape's own outline (and to
    /// everything inside a group), keeping its other values; a shape
    /// without an outline gets the default outline with those fields.
    PatchStroke {
        shapes: Vec<ShapeId>,
        stroke: Stroke,
        fields: Vec<crate::patch::StrokeField>,
    },
    /// Copy only `fields` of `fill` to each shape's own fill, as
    /// [`crate::patch::patch_fill`] does.
    PatchFill {
        shapes: Vec<ShapeId>,
        fill: Fill,
        fields: Vec<crate::patch::FillField>,
    },
    /// Each object's fill colour becomes its outline colour and its
    /// outline colour its solid fill; objects inside groups each on their
    /// own. An object without an outline loses its fill, one without a
    /// fill its outline; a fill that is not one colour (a gradient, a
    /// pattern) gives the outline the colour it shows in previews. The
    /// drawing is walked once whatever the number of objects.
    SwapColors {
        shapes: Vec<ShapeId>,
    },
    SetShapeKind {
        shape: ShapeId,
        kind: ShapeKind,
    },
    SetShapeName {
        shape: ShapeId,
        name: Option<String>,
    },
    /// A B-spline's control points (local space; two at least): the
    /// curve's outline becomes their spline, open or closed as the curve
    /// is, and the Shape tool edits them (see [`Shape::spline`]).
    SetSpline {
        shape: ShapeId,
        points: Vec<crate::geometry::Point>,
    },
    /// A QR code object's settings (its children are set with
    /// [`Command::SetShapeKind`] in the same step).
    SetQrCode {
        shape: ShapeId,
        qr: Option<Box<crate::document::QrCode>>,
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
    /// The whole effect stack of `shape`: the effects drawn and the ones
    /// switched off (see `Shape::effect_stack`).
    SetEffectStack {
        shape: ShapeId,
        effects: Vec<crate::live::Effect>,
        parked: Vec<crate::document::ParkedEffect>,
    },
    /// The further fills and outlines of `shapes`, replacing theirs.
    SetExtraPaint {
        shapes: Vec<ShapeId>,
        fills: Vec<Fill>,
        strokes: Vec<Stroke>,
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
    /// Turn wrapping on for `shapes` with this wrap style and offset.
    SetWrapStyle {
        shapes: Vec<ShapeId>,
        style: crate::text_flow::TextWrap,
    },
    SetObjectData {
        shape: ShapeId,
        data: Vec<(String, String)>,
    },
    /// Set the text in font family `from` in family `to` throughout the
    /// drawing: every span of every text and table cell, on every layer,
    /// in groups, clip frames and symbols (a missing font replaced for
    /// good, #936).
    ReplaceFontFamily {
        from: String,
        to: String,
    },
    /// Convert every colour of `shapes` (empty: the whole drawing, with
    /// the pages' backgrounds and the symbols) to `to` through the colour
    /// engine; spot colours too when `spots` (#972).
    ConvertColors {
        shapes: Vec<ShapeId>,
        to: crate::color_convert::ColorTarget,
        #[serde(default)]
        spots: bool,
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
    /// Place objects inside a frame object (a clip frame). The frame keeps its
    /// id; the contents keep their page positions.
    PlaceInside {
        contents: Vec<ShapeId>,
        frame: ShapeId,
    },
    /// Take the contents out of a clip frame, leaving the frame as a plain object.
    ExtractContents {
        clip: ShapeId,
    },
    RenamePage {
        page: PageId,
        name: String,
    },
    /// Copy a page (layers, guides, background) next to it. `before` puts
    /// the copy in front of the page instead of after it; without
    /// `contents` the copied layers are empty, otherwise their shapes are
    /// copied with fresh ids. Older scripts that name only the page get a
    /// copy with its contents after it.
    DuplicatePage {
        page: PageId,
        #[serde(default)]
        before: bool,
        #[serde(default = "yes")]
        contents: bool,
    },
    RenameLayer {
        layer: LayerId,
        name: String,
    },
    /// Move a layer to another index within its page, or a master layer
    /// within the master layers (z-order).
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

/// How many nodes a curve has (0 for other kinds).
fn node_count(kind: &ShapeKind) -> usize {
    match kind {
        ShapeKind::Path { path, .. } => crate::nodes::nodes(path).len(),
        _ => 0,
    }
}

/// Serde default for flags that are on when a saved command leaves them out.
fn yes() -> bool {
    true
}

impl Command {
    /// Duplicate `page` with its contents, the copy after it (Print Merge
    /// and older callers).
    pub fn duplicate_page(page: PageId) -> Command {
        Command::DuplicatePage {
            page,
            before: false,
            contents: true,
        }
    }

    /// True for commands that change state kept outside the undo history
    /// (the Time Tracker's records): the engine applies them without an
    /// undo step, and undo and redo leave that state alone.
    pub fn outside_history(&self) -> bool {
        matches!(
            self,
            Command::SetTimeTracker { .. } | Command::SetSavedView { .. }
        )
    }

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
            Command::SetCmykProfile { .. } => "Color Management",
            Command::SetTimeTracker { .. } => "Time Tracker",
            Command::SetSavedView { .. } => "Saved View",
            Command::SetComments { .. } => "Comments",
            Command::SetSelectionGroup { .. } => "Selection Group",
            Command::SetNodeTypes { .. } => "Node Type",
            Command::SetObjectStyles { .. } => "Object Styles",
            Command::LinkObjectStyle { style: Some(_), .. } => "Apply Style",
            Command::LinkObjectStyle { style: None, .. } => "Break Link to Style",
            Command::SetObjectDefaults { .. } => "Default Properties",
            Command::SetColorStyles { .. } => "Color Styles",
            Command::SetDocumentPalette { .. } => "Document Palette",
            Command::SetInks { .. } => "Spot Inks",
            Command::SetOpacityMask { .. } => "Opacity Mask",
            Command::SetRasterFx { .. } => "Effect",
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
            Command::PatchStroke { .. } => "Outline",
            Command::PatchFill { .. } => "Fill",
            Command::SwapColors { .. } => "Swap Colors",
            Command::SetShapeKind { .. } => "Edit Object",
            Command::SetShapeName { .. } => "Rename Object",
            Command::SetSpline { .. } => "Edit B-Spline",
            Command::SetQrCode { .. } => "QR Code",
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
            Command::SetEffectStack { .. } => "Effects",
            Command::SetExtraPaint { .. } => "Fills and Outlines",
            Command::SetOverprint { .. } => "Overprint",
            Command::SetLink { .. } => "Hyperlink",
            Command::SetWrapText { .. } => "Wrap Paragraph Text",
            Command::SetWrapStyle { .. } => "Wrap Style",
            Command::SetObjectData { .. } => "Object Data",
            Command::ConvertColors { .. } => "Convert Colors",
            Command::ReplaceFontFamily { .. } => "Replace Font",
            Command::OrderRelative { in_front: true, .. } => "In Front Of",
            Command::OrderRelative { .. } => "Behind",
            Command::ReverseOrder { .. } => "Reverse Order",
            Command::Combine { .. } => "Combine",
            Command::BreakApart { .. } => "Break Apart",
            Command::PlaceInside { .. } => "Place Inside Frame",
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

            Command::AddPage { name, size, layer } => {
                let n = doc.pages.len() + 1;
                let page_id = doc.ids_mut().page();
                let layer_id = doc.ids_mut().layer();
                doc.pages.push(Page {
                    id: page_id,
                    name: name.clone().unwrap_or_else(|| format!("Page {n}")),
                    size: *size,
                    layers: vec![Layer::new(
                        layer_id,
                        layer.clone().unwrap_or_else(|| "Layer 1".to_string()),
                    )],
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
                doc.comments.retain(|c| c.page != *page);
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
            Command::SetSavedView { view } => {
                doc.saved_view = *view;
            }
            Command::SetCmykProfile { profile } => {
                doc.cmyk_profile = profile.clone();
            }
            Command::SetSelectionGroup { number, shapes } => {
                if *number > 9 {
                    return Err(Error::SelectionGroup(*number));
                }
                if shapes.is_empty() {
                    doc.selection_groups.remove(number);
                } else {
                    doc.selection_groups.insert(*number, shapes.clone());
                }
            }
            Command::SetTimeTracker { tracker } => {
                doc.time_tracker = tracker.clone();
            }
            Command::SetComments { comments } => {
                let pages: Vec<PageId> = doc.pages.iter().map(|p| p.id).collect();
                let mut kept: Vec<crate::document::Comment> = comments
                    .iter()
                    .filter(|c| pages.contains(&c.page))
                    .cloned()
                    .collect();
                let mut next = kept.iter().map(|c| c.id).max().unwrap_or(0);
                let mut seen = std::collections::HashSet::new();
                for c in kept.iter_mut() {
                    if c.id == 0 || !seen.insert(c.id) {
                        next += 1;
                        c.id = next;
                        seen.insert(next);
                    }
                }
                doc.comments = kept;
            }
            Command::SetObjectStyles { styles } => {
                let mut styles = styles.clone();
                let mut next = styles.iter().map(|s| s.id).max().unwrap_or(0);
                for st in styles.iter_mut().filter(|s| s.id == 0) {
                    next += 1;
                    st.id = next;
                }
                doc.object_styles = styles;
                restyle(doc);
            }
            Command::LinkObjectStyle { shapes, style } => {
                let found = match style {
                    Some(id) => Some(
                        doc.object_styles
                            .iter()
                            .find(|s| s.id == *id)
                            .cloned()
                            .ok_or(Error::StyleNotFound(*id))?,
                    ),
                    None => None,
                };
                for id in shapes {
                    any(doc, *id)?;
                }
                for id in shapes {
                    let shape = any_mut(doc, *id)?;
                    shape.style = found.as_ref().map(|s| s.id);
                    if let Some(st) = &found {
                        st.apply_to(shape);
                    }
                }
            }
            Command::SetObjectDefaults { defaults } => {
                doc.defaults = defaults.clone();
            }
            Command::SetColorStyles { styles } => {
                doc.color_styles = styles.clone();
            }
            Command::SetDocumentPalette { colors } => {
                doc.palette = colors.clone();
            }
            Command::SetInks { inks } => {
                doc.inks = inks.clone();
            }
            Command::SetOpacityMask { shape, mask } => {
                let s = doc
                    .find_shape_mut(*shape)
                    .ok_or(Error::ShapeNotFound(*shape))?;
                if !matches!(s.kind, ShapeKind::ClipFrame { .. }) {
                    return Err(Error::NotClipFrame(*shape));
                }
                s.opacity_mask = *mask;
            }
            Command::SetRasterFx { shape, fx } => {
                let s = doc
                    .find_shape_mut(*shape)
                    .ok_or(Error::ShapeNotFound(*shape))?;
                s.raster_fx = fx.clone();
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
                let (layers, master) = layer_list_mut(doc, *layer)?;
                if !master && layers.len() <= 1 {
                    return Err(Error::LastLayer);
                }
                layers.retain(|l| l.id != *layer);
            }
            Command::SetLayerVisible { layer, visible } => {
                doc.layer_mut(*layer)?.visible = *visible
            }
            Command::SetLayerLocked { layer, locked } => doc.layer_mut(*layer)?.locked = *locked,

            Command::AddShape { layer, shape } => {
                // Ids minted on a copy of the drawing (a script's) stay
                // ahead of the ones this drawing gives out next.
                fn highest(s: &Shape) -> u64 {
                    let inner = match &s.kind {
                        ShapeKind::Group { children } => {
                            children.iter().map(highest).max().unwrap_or(0)
                        }
                        ShapeKind::ClipFrame { frame, contents } => contents
                            .iter()
                            .map(highest)
                            .max()
                            .unwrap_or(0)
                            .max(highest(frame)),
                        _ => 0,
                    };
                    s.id.raw().max(inner)
                }
                let top = highest(shape);
                doc.layer_mut(*layer)?.shapes.push(shape.clone());
                doc.ids_mut().reserve(top);
            }
            Command::DeleteShapes { shapes } => {
                for id in shapes {
                    any(doc, *id)?;
                }
                for layer in doc.all_layers_mut() {
                    remove_shapes(&mut layer.shapes, shapes);
                }
            }
            Command::TransformShapes { shapes, transform } => {
                // The transform is in page space; an object inside a group
                // or a clip frame takes it in its parent's space.
                let mut local = Vec::with_capacity(shapes.len());
                for id in shapes {
                    any(doc, *id)?;
                    let (t, parent) = if doc.shape(*id).is_ok() {
                        (*transform, Affine::IDENTITY)
                    } else {
                        let parent = doc.parent_transform(*id).unwrap_or(Affine::IDENTITY);
                        let det = parent.determinant();
                        if !det.is_finite() || det.abs() < 1e-12 {
                            return Err(Error::ShapeNotFound(*id));
                        }
                        (parent.inverse() * *transform * parent, parent)
                    };
                    local.push((*id, t, parent));
                }
                for (id, t, parent) in local {
                    let shape = any_mut(doc, id)?;
                    // Gradients turn and mirror with the object (#775).
                    shape.carry_fills(*transform, parent);
                    // A symmetry's mirror lines go with the object.
                    shape.transform_by(t);
                }
            }
            Command::SetFill { shapes, fill } => {
                for id in shapes {
                    any(doc, *id)?;
                }
                for id in shapes {
                    // A group passes the fill on to everything in it.
                    for_group_deep(any_mut(doc, *id)?, &mut |s| s.fill = fill.clone());
                }
            }
            Command::SetStroke { shapes, stroke } => {
                for id in shapes {
                    any(doc, *id)?;
                }
                for id in shapes {
                    for_group_deep(any_mut(doc, *id)?, &mut |s| s.stroke = stroke.clone());
                }
            }
            Command::PatchStroke {
                shapes,
                stroke,
                fields,
            } => {
                for id in shapes {
                    any(doc, *id)?;
                }
                for id in shapes {
                    for_group_deep(any_mut(doc, *id)?, &mut |s| {
                        s.stroke = Some(crate::patch::patch_stroke(
                            s.stroke.as_ref(),
                            stroke,
                            fields,
                        ))
                    });
                }
            }
            Command::PatchFill {
                shapes,
                fill,
                fields,
            } => {
                for id in shapes {
                    any(doc, *id)?;
                }
                for id in shapes {
                    for_group_deep(any_mut(doc, *id)?, &mut |s| {
                        s.fill = crate::patch::patch_fill(&s.fill, fill, fields)
                    });
                }
            }
            Command::SwapColors { shapes } => {
                let wanted: std::collections::HashSet<ShapeId> = shapes.iter().copied().collect();
                let mut found = std::collections::HashSet::with_capacity(wanted.len());
                for layer in doc.all_layers_mut() {
                    for s in &mut layer.shapes {
                        swap_colors_under(s, &wanted, &mut found, false);
                    }
                }
                // The engine puts the drawing back when this fails.
                if let Some(id) = shapes.iter().find(|id| !found.contains(id)) {
                    return Err(Error::ShapeNotFound(*id));
                }
            }
            Command::SetShapeKind { shape, kind } => {
                let s = any_mut(doc, *shape)?;
                // Chosen node types follow node indices: a curve that
                // gains or loses nodes drops them.
                if !s.node_types.is_empty() && node_count(&s.kind) != node_count(kind) {
                    s.node_types.clear();
                }
                // A B-spline given any other outline is an ordinary curve.
                s.spline = None;
                s.kind = kind.clone();
            }
            Command::SetSpline { shape, points } => {
                if points.len() < 2 || points.iter().any(|p| !p.is_finite()) {
                    return Err(Error::BadSpline);
                }
                let s = any_mut(doc, *shape)?;
                let closed = matches!(s.kind, ShapeKind::Path { closed: true, .. });
                s.kind = ShapeKind::Path {
                    path: crate::geometry::bspline_path(points, closed),
                    closed,
                };
                s.node_types.clear();
                s.spline = Some(points.clone());
            }
            Command::SetNodeTypes { shape, types } => {
                any_mut(doc, *shape)?.node_types = types.clone()
            }
            Command::SetShapeName { shape, name } => any_mut(doc, *shape)?.name = name.clone(),
            Command::SetQrCode { shape, qr } => any_mut(doc, *shape)?.qr = qr.clone(),

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
                // Bottom to top: by layer (in drawing order), then by place
                // in the layer, so members from several layers keep the
                // order they were drawn in.
                let layer_rank = |l: LayerId| {
                    doc.pages
                        .iter()
                        .flat_map(|p| p.layers.iter())
                        .chain(doc.master.iter())
                        .position(|x| x.id == l)
                        .unwrap_or(usize::MAX)
                };
                located.sort_by_key(|(_, l, i)| (layer_rank(*l), *i));
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
                    any(doc, *id)?;
                }
                for id in shapes {
                    any_mut(doc, *id)?.opacity = opacity.clamp(0.0, 1.0);
                }
            }
            Command::SetLocked { shapes, locked } => {
                for id in shapes {
                    any(doc, *id)?;
                }
                for id in shapes {
                    any_mut(doc, *id)?.locked = *locked;
                }
            }
            Command::SetVisible { shapes, visible } => {
                for id in shapes {
                    any(doc, *id)?;
                }
                for id in shapes {
                    any_mut(doc, *id)?.visible = *visible;
                }
            }
            Command::SetEffects { shape, effects } => {
                any_mut(doc, *shape)?.effects = effects.clone();
            }
            Command::SetEffectStack {
                shape,
                effects,
                parked,
            } => {
                let s = any_mut(doc, *shape)?;
                s.effects = effects.clone();
                s.parked_effects = parked.clone();
            }
            Command::SetExtraPaint {
                shapes,
                fills,
                strokes,
            } => {
                for id in shapes {
                    let s = any_mut(doc, *id)?;
                    s.extra_fills = fills.clone();
                    s.extra_strokes = strokes.clone();
                }
            }
            Command::SetOverprint {
                shapes,
                fill,
                outline,
            } => {
                for id in shapes {
                    any(doc, *id)?;
                }
                for id in shapes {
                    let s = any_mut(doc, *id)?;
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
                    any(doc, *id)?;
                }
                for id in shapes {
                    any_mut(doc, *id)?.wrap_text = *wrap;
                }
            }
            Command::SetWrapStyle { shapes, style } => {
                for id in shapes {
                    any(doc, *id)?;
                }
                let mut style = *style;
                if !style.offset.is_finite() {
                    style.offset = 0.0;
                }
                style.offset = style.offset.clamp(0.0, 1000.0);
                for id in shapes {
                    let s = any_mut(doc, *id)?;
                    s.wrap_text = true;
                    s.wrap_style = style;
                }
            }
            Command::SetLink { shapes, link } => {
                for id in shapes {
                    any(doc, *id)?;
                }
                for id in shapes {
                    any_mut(doc, *id)?.link = link.clone();
                }
            }
            Command::SetObjectData { shape, data } => {
                any_mut(doc, *shape)?.data = data.clone();
            }
            Command::ConvertColors { shapes, to, spots } => {
                for id in shapes {
                    any(doc, *id)?;
                }
                crate::color_convert::convert_document(doc, shapes, *to, *spots);
            }
            Command::ReplaceFontFamily { from, to } => {
                doc.replace_font_family(from, to);
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
                let mut frame_shape = doc.layer_mut(flid)?.shapes.remove(fi);
                // A clip frame already: the objects join its contents,
                // over the ones it holds.
                if let ShapeKind::ClipFrame { contents, .. } = &mut frame_shape.kind {
                    contents.extend(inner);
                    let target = &mut doc.layer_mut(flid)?.shapes;
                    target.insert(fi.min(target.len()), frame_shape);
                    return Ok(());
                }
                // The clip frame keeps the object's id; the frame inside
                // it gets one of its own, so it can be found, painted and
                // reshaped (Extract Contents gives it the id back).
                frame_shape.id = doc.ids_mut().shape();
                let mut clip = Shape::new(
                    *frame,
                    ShapeKind::ClipFrame {
                        frame: Box::new(frame_shape.clone()),
                        contents: inner,
                    },
                );
                clip.name = frame_shape.name.clone();
                // Its object data stays with the object too (a Print Merge
                // placeholder keeps its Align and scale there).
                clip.data = frame_shape.data.clone();
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
            Command::DuplicatePage {
                page,
                before,
                contents,
            } => {
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
                    if *contents {
                        for s in &l.shapes {
                            nl.shapes.push(reid_shape(s, doc));
                        }
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
                let at = if *before { idx } else { idx + 1 };
                doc.pages.insert(at, copy);
            }
            Command::RenameLayer { layer, name } => doc.layer_mut(*layer)?.name = name.clone(),
            Command::ReorderLayer { layer, index } => {
                let (layers, _) = layer_list_mut(doc, *layer)?;
                let i = layers
                    .iter()
                    .position(|l| l.id == *layer)
                    .ok_or(Error::LayerNotFound(*layer))?;
                let l = layers.remove(i);
                let index = (*index).min(layers.len());
                layers.insert(index, l);
            }
            Command::SetShadow { shapes, shadow } => {
                for id in shapes {
                    any(doc, *id)?;
                }
                for id in shapes {
                    any_mut(doc, *id)?.shadow = *shadow;
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
                // The group's perspective stays on each of its objects, which
                // are drawn as they were (#674); its other effects go.
                let group_path = g.local_path();
                let group_effects = g.effects.clone();
                match g.kind {
                    ShapeKind::Group { children } => {
                        for (k, mut c) in children.into_iter().enumerate() {
                            crate::live::take_group_perspectives(
                                &mut c,
                                &group_path,
                                &group_effects,
                            );
                            c.absorb(g.transform);
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
/// Any object, nested ones too.
fn any(doc: &Document, id: ShapeId) -> Result<&Shape> {
    doc.find_shape(id).ok_or(Error::ShapeNotFound(id))
}

/// Any object to change, nested ones too.
fn any_mut(doc: &mut Document, id: ShapeId) -> Result<&mut Shape> {
    doc.find_shape_mut(id).ok_or(Error::ShapeNotFound(id))
}

/// The list holding `layer`: the master layers (then true) or the layers
/// of its page.
fn layer_list_mut(doc: &mut Document, layer: LayerId) -> Result<(&mut Vec<Layer>, bool)> {
    if doc.master.iter().any(|l| l.id == layer) {
        return Ok((&mut doc.master, true));
    }
    doc.pages
        .iter_mut()
        .map(|p| &mut p.layers)
        .find(|layers| layers.iter().any(|l| l.id == layer))
        .map(|layers| (layers, false))
        .ok_or(Error::LayerNotFound(layer))
}

/// Take the objects `ids` out of `shapes`, inside groups and clip frames
/// too; a group left with nothing by it goes as well.
fn remove_shapes(shapes: &mut Vec<Shape>, ids: &[ShapeId]) {
    shapes.retain(|s| !ids.contains(&s.id));
    shapes.retain_mut(|s| match &mut s.kind {
        ShapeKind::Group { children } if !children.is_empty() => {
            remove_shapes(children, ids);
            !children.is_empty()
        }
        ShapeKind::ClipFrame { contents, .. } => {
            remove_shapes(contents, ids);
            true
        }
        _ => true,
    });
}

/// A copy of `s` with new ids all through it: in groups, and in clip
/// frames' frames and contents (a page copy whose clip frames kept their
/// ids had two objects of one id, and a change to the copy went to the
/// original, #377).
fn reid_shape(s: &Shape, doc: &mut Document) -> Shape {
    let mut c = s.clone();
    c.id = doc.ids_mut().shape();
    match &mut c.kind {
        ShapeKind::Group { children } => {
            let copies: Vec<Shape> = children.iter().map(|ch| reid_shape(ch, doc)).collect();
            *children = copies;
        }
        ShapeKind::ClipFrame { frame, contents } => {
            **frame = reid_shape(frame, doc);
            let copies: Vec<Shape> = contents.iter().map(|ch| reid_shape(ch, doc)).collect();
            *contents = copies;
        }
        _ => {}
    }
    c
}

/// Give every object linked to an object style the style's parts again,
/// inside groups and clip frames too; links to missing styles go.
fn restyle(doc: &mut Document) {
    fn walk(shape: &mut Shape, styles: &[ObjectStyle]) {
        if let Some(id) = shape.style {
            match styles.iter().find(|s| s.id == id) {
                Some(st) => st.apply_to(shape),
                None => shape.style = None,
            }
        }
        match &mut shape.kind {
            ShapeKind::Group { children } => {
                for c in children {
                    walk(c, styles);
                }
            }
            ShapeKind::ClipFrame { frame, contents } => {
                walk(frame, styles);
                for c in contents {
                    walk(c, styles);
                }
            }
            _ => {}
        }
    }
    let styles = doc.object_styles.clone();
    for layer in doc.all_layers_mut() {
        for shape in &mut layer.shapes {
            walk(shape, &styles);
        }
    }
}

/// Run `f` on `shape` and, when it is a group, on everything inside it
/// (groups within groups too), as a fill or outline given to a group
/// reaches its objects.
/// `f` on `shape` and on everything a group holds; on a clip frame also on
/// its frame (the object that is drawn), not on its contents.
/// Swap Colors: swap the colours of the objects of `wanted` under
/// `shape` (nested ones too; `on`: `shape` is inside one of them), each
/// once, noting the ones met in `found`. A group's objects swap each on
/// their own; a clip frame swaps its own colours (and its frame's, as a
/// fill given to it would), and objects in it swap only when named.
fn swap_colors_under(
    shape: &mut Shape,
    wanted: &std::collections::HashSet<ShapeId>,
    found: &mut std::collections::HashSet<ShapeId>,
    on: bool,
) {
    let named = wanted.contains(&shape.id);
    if named {
        found.insert(shape.id);
    }
    let on = on || named;
    if on && !matches!(shape.kind, ShapeKind::Group { .. }) {
        swap_own_colors(shape);
    }
    match &mut shape.kind {
        ShapeKind::Group { children } => {
            for c in children {
                swap_colors_under(c, wanted, found, on);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            if on {
                // Swapped with the clip frame already.
                note_wanted(frame, wanted, found);
            } else {
                swap_colors_under(frame, wanted, found, false);
            }
            for c in contents {
                swap_colors_under(c, wanted, found, false);
            }
        }
        _ => {}
    }
}

/// Note `shape` and the objects in it that are in `wanted`.
fn note_wanted(
    shape: &Shape,
    wanted: &std::collections::HashSet<ShapeId>,
    found: &mut std::collections::HashSet<ShapeId>,
) {
    if wanted.contains(&shape.id) {
        found.insert(shape.id);
    }
    match &shape.kind {
        ShapeKind::Group { children } => {
            children.iter().for_each(|c| note_wanted(c, wanted, found))
        }
        ShapeKind::ClipFrame { frame, contents } => {
            note_wanted(frame, wanted, found);
            contents.iter().for_each(|c| note_wanted(c, wanted, found));
        }
        _ => {}
    }
}

/// Swap the fill and outline colours of one object (not a group).
fn swap_own_colors(shape: &mut Shape) {
    let fill_color = shape.fill.preview_color();
    let outline_color = shape.stroke.as_ref().map(|st| st.color);
    if fill_color.is_none() && outline_color.is_none() {
        return;
    }
    let fill = outline_color.map_or(Fill::None, Fill::Solid);
    let paint = fill_color.map(|color| Stroke {
        color,
        ..Stroke::default()
    });
    let fields = [crate::patch::StrokeField::Paint];
    // As a fill and an outline set on the object: a clip frame's frame
    // takes them too.
    for_group_deep(shape, &mut |s| {
        s.fill = fill.clone();
        s.stroke = paint
            .as_ref()
            .map(|p| crate::patch::patch_stroke(s.stroke.as_ref(), p, &fields));
    });
}

fn for_group_deep(shape: &mut Shape, f: &mut dyn FnMut(&mut Shape)) {
    f(shape);
    match &mut shape.kind {
        ShapeKind::Group { children } => {
            for c in children {
                for_group_deep(c, f);
            }
        }
        ShapeKind::ClipFrame { frame, .. } => for_group_deep(frame, f),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    /// Swap Colors: fill and outline colours change places on each
    /// object, inside groups each on its own; an object selected with its
    /// group swaps once; others stay; one undo step; an unknown object
    /// changes nothing.
    #[test]
    fn swap_colors_swaps_each_object_once() {
        use crate::document::ShapeKind;
        use crate::{Color, Fill};
        let red = Color::rgb8(200, 0, 0);
        let blue = Color::rgb8(0, 0, 200);
        let green = Color::rgb8(0, 200, 0);
        let mut doc = Document::new("t", Size::new(100.0, 100.0));
        let mut ids = doc.ids().clone();
        let mut rect = |fill: Option<Color>, stroke: Option<Color>| {
            let mut s = Shape::new(
                ids.shape(),
                ShapeKind::Rect {
                    rect: crate::Rect::new(0.0, 0.0, 10.0, 10.0),
                    radius: 0.0,
                    corners: None,
                },
            );
            s.fill = fill.map_or(Fill::None, Fill::Solid);
            s.stroke = stroke.map(|color| Stroke {
                color,
                width: 2.0,
                ..Stroke::default()
            });
            s
        };
        let a = rect(Some(red), Some(blue));
        let b = rect(Some(blue), None);
        let c = rect(None, Some(green));
        let d = rect(None, None);
        let e = rect(Some(red), Some(blue));
        let (a_id, b_id, c_id, d_id, e_id) = (a.id, b.id, c.id, d.id, e.id);
        let group = Shape::new(
            ids.shape(),
            ShapeKind::Group {
                children: vec![b, c, d],
            },
        );
        let g_id = group.id;
        doc.set_ids(ids);
        doc.pages[0].layers[0].shapes = vec![a, group, e];
        let mut engine = crate::Engine::new(doc);
        let before = engine.document().clone();
        let paint = |e: &crate::Engine, id| {
            let s = e.document().find_shape(id).unwrap();
            (
                s.fill.clone(),
                s.stroke.as_ref().map(|st| (st.color, st.width)),
            )
        };
        engine
            .run(&Command::SwapColors {
                shapes: vec![a_id, g_id, b_id],
            })
            .unwrap();
        // The outline keeps its width, only its colour changes.
        assert_eq!(paint(&engine, a_id), (Fill::Solid(blue), Some((red, 2.0))));
        let hairline = Stroke::default().width;
        assert_eq!(paint(&engine, b_id), (Fill::None, Some((blue, hairline))));
        assert_eq!(paint(&engine, c_id), (Fill::Solid(green), None));
        assert_eq!(paint(&engine, d_id), (Fill::None, None));
        assert_eq!(paint(&engine, e_id), (Fill::Solid(red), Some((blue, 2.0))));
        assert!(engine.undo().is_ok());
        assert_eq!(engine.document(), &before);
        assert!(engine
            .run(&Command::SwapColors {
                shapes: vec![a_id, crate::ShapeId(9999)],
            })
            .is_err());
        assert_eq!(engine.document(), &before);
    }

    /// Swap Colors on many objects is one walk of the drawing: twenty
    /// thousand objects swap in one command (a search of the drawing per
    /// object made this take minutes).
    #[test]
    fn swap_colors_handles_many_objects() {
        use crate::document::ShapeKind;
        use crate::{Color, Fill};
        let mut doc = Document::new("t", Size::new(100.0, 100.0));
        let mut ids = doc.ids().clone();
        let shapes: Vec<Shape> = (0..20_000)
            .map(|_| {
                let mut s = Shape::new(
                    ids.shape(),
                    ShapeKind::Rect {
                        rect: crate::Rect::new(0.0, 0.0, 1.0, 1.0),
                        radius: 0.0,
                        corners: None,
                    },
                );
                s.fill = Fill::Solid(Color::WHITE);
                s.stroke = Some(Stroke::default());
                s
            })
            .collect();
        let all: Vec<_> = shapes.iter().map(|s| s.id).collect();
        doc.set_ids(ids);
        doc.pages[0].layers[0].shapes = shapes;
        let mut engine = crate::Engine::new(doc);
        engine.run(&Command::SwapColors { shapes: all }).unwrap();
        assert!(engine.document().pages[0].layers[0]
            .shapes
            .iter()
            .all(|s| s.fill == Fill::Solid(Color::BLACK)
                && s.stroke.as_ref().map(|st| st.color) == Some(Color::WHITE)));
    }

    /// A missing font replaced for good (#936): every span in that family,
    /// in groups and symbols too, takes the new family; one undo step
    /// brings the old one back; other spans are left alone.
    #[test]
    fn a_font_family_is_replaced_throughout_the_drawing() {
        use crate::document::{ShapeKind, TextSpan};
        use crate::geometry::Point;
        let text = |spans: Vec<TextSpan>| {
            Shape::new(
                crate::ShapeId(0),
                ShapeKind::Text {
                    spans,
                    origin: Point::new(1.0, 1.0),
                    frame: None,
                    align: Default::default(),
                    para: Default::default(),
                    on_path: None,
                },
            )
        };
        let mut doc = Document::new("t", Size::new(100.0, 100.0));
        let mut ids = doc.ids().clone();
        let mut a = text(vec![
            TextSpan::new("Old", "Gone Sans", 12.0),
            TextSpan::new("Kept", "Other", 12.0),
        ]);
        a.id = ids.shape();
        let mut inner = text(vec![TextSpan::new("In", "Gone Sans", 12.0)]);
        inner.id = ids.shape();
        let mut group = Shape::new(
            ids.shape(),
            ShapeKind::Group {
                children: vec![inner],
            },
        );
        group.fill = crate::Fill::None;
        let mut sym = text(vec![TextSpan::new("Sym", "Gone Sans", 12.0)]);
        sym.id = ids.shape();
        doc.set_ids(ids);
        doc.pages[0].layers[0].shapes = vec![a, group];
        doc.symbols.push(crate::document::Symbol {
            name: "s".into(),
            shape: sym,
        });
        let mut engine = crate::Engine::new(doc);
        engine
            .run(&Command::ReplaceFontFamily {
                from: "Gone Sans".into(),
                to: "Liberation Sans".into(),
            })
            .unwrap();
        let fams = engine.document().font_families_used();
        assert_eq!(
            fams,
            vec!["Liberation Sans".to_string(), "Other".to_string()]
        );
        assert!(engine.undo().is_ok());
        assert!(engine
            .document()
            .font_families_used()
            .contains(&"Gone Sans".to_string()));
        // Nothing to replace with: nothing changes.
        let mut d = engine.document().clone();
        assert_eq!(d.replace_font_family("Gone Sans", ""), 0);
        assert_eq!(d.replace_font_family("Gone Sans", "Other"), 3);
    }

    #[test]
    fn comments_get_free_ids_and_go_with_their_page() {
        use crate::document::{Comment, CommentMark};
        use crate::geometry::Point;
        let mut engine = crate::Engine::new(Document::new("t", Size::new(100.0, 100.0)));
        engine
            .run(&Command::AddPage {
                name: None,
                size: Size::new(100.0, 100.0),
                layer: None,
            })
            .unwrap();
        let (p1, p2) = (engine.document().pages[0].id, engine.document().pages[1].id);
        let note = |id: u32, page: PageId, text: &str| Comment {
            id,
            page,
            mark: CommentMark::Pin {
                at: Point::new(10.0, 10.0),
            },
            author: "Ana".into(),
            time: 1_700_000_000,
            text: text.into(),
            replies: Vec::new(),
            resolved: false,
        };
        engine
            .run(&Command::SetComments {
                comments: vec![note(4, p1, "a"), note(4, p2, "b"), note(0, p2, "c")],
            })
            .unwrap();
        let ids: Vec<u32> = engine.document().comments.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![4, 5, 6]);
        assert_eq!(engine.undo_label(), Some("Comments"));
        // Deleting a page takes its comments with it; undo brings them back.
        engine.run(&Command::DeletePage { page: p2 }).unwrap();
        assert_eq!(engine.document().comments.len(), 1);
        engine.undo().unwrap();
        assert_eq!(engine.document().comments.len(), 3);
        // A comment on a page that does not exist is not kept.
        engine
            .run(&Command::SetComments {
                comments: vec![note(1, PageId(9999), "x")],
            })
            .unwrap();
        assert!(engine.document().comments.is_empty());
    }

    #[test]
    fn a_spline_follows_its_control_points_until_the_outline_is_changed_otherwise() {
        use crate::geometry::Point;
        let mut doc = Document::default();
        let page = doc.pages[0].id;
        let layer = doc.page(page).expect("page").layers[0].id;
        let id = doc.ids_mut().shape();
        let shape = Shape::new(
            id,
            ShapeKind::Path {
                path: BezPath::new(),
                closed: false,
            },
        );
        let mut eng = crate::Engine::new(doc);
        eng.run(&Command::AddShape { layer, shape }).expect("added");
        let pts = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 20.0),
            Point::new(30.0, 20.0),
            Point::new(40.0, 0.0),
        ];
        eng.run(&Command::SetSpline {
            shape: id,
            points: pts.clone(),
        })
        .expect("spline");
        let s = eng.document().find_shape(id).expect("shape");
        assert_eq!(s.spline.as_deref(), Some(pts.as_slice()));
        assert_eq!(
            s.kind,
            ShapeKind::Path {
                path: crate::geometry::bspline_path(&pts, false),
                closed: false,
            }
        );
        // The native format keeps the control points.
        let json = serde_json::to_string(eng.document()).expect("json");
        let back: Document = serde_json::from_str(&json).expect("parsed");
        assert_eq!(back.find_shape(id).expect("shape").spline, s.spline);
        // Fewer than two points, or a point that is not finite: refused.
        assert!(eng
            .run(&Command::SetSpline {
                shape: id,
                points: vec![Point::new(0.0, 0.0)],
            })
            .is_err());
        assert!(eng
            .run(&Command::SetSpline {
                shape: id,
                points: vec![Point::new(0.0, 0.0), Point::new(f64::NAN, 0.0)],
            })
            .is_err());
        // Any other outline makes it an ordinary curve.
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((40.0, 0.0));
        eng.run(&Command::SetShapeKind {
            shape: id,
            kind: ShapeKind::Path {
                path: line,
                closed: false,
            },
        })
        .expect("kind");
        assert_eq!(eng.document().find_shape(id).expect("shape").spline, None);
        eng.undo().expect("undo");
        assert!(eng
            .document()
            .find_shape(id)
            .expect("shape")
            .spline
            .is_some());
        // So does Convert To Curves, keeping the outline.
        let s = eng.document().find_shape(id).expect("shape").clone();
        assert!(s.converts_to_curves());
        let mut next = || ShapeId(0);
        assert_eq!(s.curves_kind(&[], &mut next), Some(s.kind.clone()));
    }

    #[test]
    fn chosen_node_types_are_kept_until_the_node_count_changes() {
        use crate::nodes::NodeType;
        let mut doc = Document::default();
        let page = doc.pages[0].id;
        let layer = doc.page(page).expect("page").layers[0].id;
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.curve_to((5.0, 0.0), (10.0, 0.0), (15.0, 0.0));
        path.curve_to((20.0, 0.0), (25.0, 5.0), (30.0, 10.0));
        let id = doc.ids_mut().shape();
        let shape = Shape::new(
            id,
            ShapeKind::Path {
                path: path.clone(),
                closed: false,
            },
        );
        let mut eng = crate::Engine::new(doc);
        eng.run(&Command::AddShape { layer, shape }).expect("added");
        // The middle node has aligned handles of equal length: it reads
        // symmetrical, yet it is kept cusp once chosen so.
        let mid = crate::nodes::nodes(&path)[1].index;
        assert_eq!(crate::nodes::node_type(&path, mid), NodeType::Symmetrical);
        let mut types = std::collections::BTreeMap::new();
        types.insert(mid, NodeType::Cusp);
        eng.run(&Command::SetNodeTypes { shape: id, types })
            .expect("types");
        let s = eng.document().find_shape(id).expect("shape");
        assert_eq!(
            crate::nodes::node_type_with(&path, mid, &s.node_types),
            NodeType::Cusp
        );
        // Saved and read back.
        let json = serde_json::to_string(eng.document()).expect("saved");
        let back: Document = serde_json::from_str(&json).expect("read");
        assert_eq!(back.find_shape(id).expect("shape").node_types.len(), 1);
        // A node moved: the curve keeps its node count, and the type.
        let moved = crate::nodes::move_node(&path, mid, crate::geometry::Vec2::new(0.0, 2.0));
        eng.run(&Command::SetShapeKind {
            shape: id,
            kind: ShapeKind::Path {
                path: moved.clone(),
                closed: false,
            },
        })
        .expect("moved");
        assert_eq!(
            eng.document()
                .find_shape(id)
                .expect("shape")
                .node_types
                .len(),
            1
        );
        // A node more: the types no longer match their nodes and go.
        let added = crate::nodes::add_node(&moved, mid, 0.5);
        eng.run(&Command::SetShapeKind {
            shape: id,
            kind: ShapeKind::Path {
                path: added,
                closed: false,
            },
        })
        .expect("added node");
        assert!(eng
            .document()
            .find_shape(id)
            .expect("shape")
            .node_types
            .is_empty());
    }

    #[test]
    fn selection_groups_are_kept_saved_and_undone() {
        let mut eng = crate::Engine::new(Document::default());
        let a = crate::id::ShapeId(7);
        eng.run(&Command::SetSelectionGroup {
            number: 3,
            shapes: vec![a],
        })
        .expect("group 3");
        assert_eq!(eng.document().selection_groups.get(&3), Some(&vec![a]));
        // Saved with the drawing and read back.
        let json = serde_json::to_string(eng.document()).expect("saved");
        let back: Document = serde_json::from_str(&json).expect("read");
        assert_eq!(back.selection_groups.get(&3), Some(&vec![a]));
        // Undone like any change; no shapes removes a group; 10 is refused.
        eng.undo().expect("undo");
        assert!(eng.document().selection_groups.is_empty());
        eng.redo().expect("redo");
        eng.run(&Command::SetSelectionGroup {
            number: 3,
            shapes: Vec::new(),
        })
        .expect("cleared");
        assert!(eng.document().selection_groups.is_empty());
        assert!(eng
            .run(&Command::SetSelectionGroup {
                number: 10,
                shapes: vec![a],
            })
            .is_err());
    }

    use super::*;

    #[test]
    fn effects_switched_off_keep_their_place_in_the_stack() {
        use crate::live::Effect;
        let a = Effect::Perspective {
            corners: [crate::geometry::Point::ZERO; 4],
        };
        let b = Effect::BlockShadow {
            offset: crate::geometry::Vec2::new(1.0, 1.0),
            color: Color::BLACK,
            gap: 0.0,
        };
        let c = Effect::BlockShadow {
            offset: crate::geometry::Vec2::new(2.0, 2.0),
            color: Color::BLACK,
            gap: 0.0,
        };
        let stack = vec![(a.clone(), true), (b.clone(), false), (c.clone(), true)];
        let (effects, parked) = Shape::split_stack(stack.clone());
        assert_eq!(effects, vec![a.clone(), c.clone()]);
        assert_eq!(parked.len(), 1);
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let mut s = Shape::new(
            doc.ids_mut().shape(),
            ShapeKind::Rect {
                rect: crate::geometry::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        let id = s.id;
        s.effects = vec![a.clone()];
        doc.layer_mut(layer).unwrap().shapes.push(s);
        Command::SetEffectStack {
            shape: id,
            effects,
            parked,
        }
        .apply(&mut doc)
        .unwrap();
        let s = doc.find_shape(id).unwrap();
        assert_eq!(s.effect_stack(), stack);
        assert_eq!(s.effects.len(), 2, "only the effects that are on are drawn");
        // A parked index past the end still comes back, at the end.
        let mut odd = s.clone();
        odd.parked_effects[0].index = 99;
        assert_eq!(odd.effect_stack().len(), 3);
        assert_eq!(odd.effect_stack()[2], (b, false));
    }

    #[test]
    fn only_clip_frames_take_an_opacity_mask() {
        use crate::document::OpacityMask;
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let r = Shape::new(
            doc.ids_mut().shape(),
            ShapeKind::Rect {
                rect: crate::geometry::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        let rid = r.id;
        let clip = Shape::new(
            doc.ids_mut().shape(),
            ShapeKind::ClipFrame {
                frame: Box::new(r.clone()),
                contents: Vec::new(),
            },
        );
        let cid = clip.id;
        doc.layer_mut(layer).unwrap().shapes.extend([r, clip]);
        let mask = Some(OpacityMask { invert: true });
        assert!(Command::SetOpacityMask { shape: rid, mask }
            .apply(&mut doc)
            .is_err());
        Command::SetOpacityMask { shape: cid, mask }
            .apply(&mut doc)
            .unwrap();
        assert_eq!(doc.find_shape(cid).unwrap().opacity_mask, mask);
        let back = Document::from_json(&doc.to_json().unwrap()).unwrap();
        assert_eq!(back.find_shape(cid).unwrap().opacity_mask, mask);
        Command::SetOpacityMask {
            shape: cid,
            mask: None,
        }
        .apply(&mut doc)
        .unwrap();
        assert!(doc.find_shape(cid).unwrap().opacity_mask.is_none());
    }
    use crate::geometry::Rect;

    fn rect(doc: &mut Document, x: f64) -> Shape {
        let id = doc.ids_mut().shape();
        Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(x, 0.0, x + 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        )
    }

    /// Objects inside a group take page-space transforms, fills and
    /// outlines, and can be deleted; a group left empty goes too.
    #[test]
    fn fills_and_outlines_given_to_a_group_reach_its_objects() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let a = rect(&mut doc, 0.0);
        let b = rect(&mut doc, 20.0);
        let (ia, ib) = (a.id, b.id);
        for shape in [a, b] {
            Command::AddShape { layer, shape }.apply(&mut doc).unwrap();
        }
        Command::Group { shapes: vec![ia] }.apply(&mut doc).unwrap();
        let inner = doc.top_level_of(ia).unwrap();
        Command::Group {
            shapes: vec![inner, ib],
        }
        .apply(&mut doc)
        .unwrap();
        let outer = doc.top_level_of(ib).unwrap();
        let red = Fill::Solid(Color::rgb8(255, 0, 0));
        Command::SetFill {
            shapes: vec![outer],
            fill: red.clone(),
        }
        .apply(&mut doc)
        .unwrap();
        Command::SetStroke {
            shapes: vec![outer],
            stroke: None,
        }
        .apply(&mut doc)
        .unwrap();
        for id in [ia, ib] {
            let s = doc.find_shape(id).unwrap();
            assert_eq!(s.fill, red);
            assert!(s.stroke.is_none());
        }
    }

    #[test]
    fn objects_inside_groups_can_be_changed_and_deleted() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let a = rect(&mut doc, 0.0);
        let b = rect(&mut doc, 20.0);
        let (ia, ib) = (a.id, b.id);
        for shape in [a, b] {
            Command::AddShape { layer, shape }.apply(&mut doc).unwrap();
        }
        Command::Group {
            shapes: vec![ia, ib],
        }
        .apply(&mut doc)
        .unwrap();
        let gid = doc.layer(layer).unwrap().shapes[0].id;
        // The group is turned and scaled, so the child's space is not the
        // page's.
        let g = Affine::translate((100.0, 50.0)) * Affine::rotate(0.5) * Affine::scale(2.0);
        Command::TransformShapes {
            shapes: vec![gid],
            transform: g,
        }
        .apply(&mut doc)
        .unwrap();
        assert!(doc.is_nested(ia) && !doc.is_nested(gid));
        assert_eq!(doc.top_level_of(ib), Some(gid));
        assert_eq!(doc.ancestors(ib), vec![gid]);
        assert!(doc.ancestors(gid).is_empty());
        let before = doc.shape_on_page(ia).unwrap().bounds();
        // A page-space move moves the child by exactly that on the page.
        Command::TransformShapes {
            shapes: vec![ia],
            transform: Affine::translate((7.0, -3.0)),
        }
        .apply(&mut doc)
        .unwrap();
        let after = doc.shape_on_page(ia).unwrap().bounds();
        assert!((after.x0 - before.x0 - 7.0).abs() < 1e-9);
        assert!((after.y0 - before.y0 + 3.0).abs() < 1e-9);
        assert!((after.width() - before.width()).abs() < 1e-9);
        // The other child stays where it was.
        let b_page = doc.shape_on_page(ib).unwrap().bounds();
        let expected = {
            use crate::geometry::Shape as _;
            (g * Rect::new(20.0, 0.0, 30.0, 10.0).to_path(0.01)).bounding_box()
        };
        assert!((b_page.x0 - expected.x0).abs() < 1e-6);
        // Fills and outlines reach the child.
        let red = Fill::Solid(Color::rgb8(255, 0, 0));
        Command::SetFill {
            shapes: vec![ia],
            fill: red.clone(),
        }
        .apply(&mut doc)
        .unwrap();
        assert_eq!(doc.find_shape(ia).unwrap().fill, red);
        Command::SetStroke {
            shapes: vec![ib],
            stroke: None,
        }
        .apply(&mut doc)
        .unwrap();
        assert!(doc.find_shape(ib).unwrap().stroke.is_none());
        // Deleting one child keeps the group; deleting the last one takes
        // the group away.
        Command::DeleteShapes { shapes: vec![ia] }
            .apply(&mut doc)
            .unwrap();
        assert!(doc.find_shape(ia).is_none());
        assert_eq!(doc.top_level_of(ib), Some(gid));
        Command::DeleteShapes { shapes: vec![ib] }
            .apply(&mut doc)
            .unwrap();
        assert!(doc.layer(layer).unwrap().shapes.is_empty());
        // Unknown objects are refused.
        assert!(Command::DeleteShapes { shapes: vec![ia] }
            .apply(&mut doc)
            .is_err());
    }

    /// A group's perspective stays on each object when it is ungrouped,
    /// drawn where it was in the group (#674): before the object's own
    /// effects (a contour rings the warped object, a distortion bends it),
    /// and folded into a perspective the object had, so it keeps one.
    #[test]
    fn ungrouping_keeps_the_perspective_on_each_object() {
        use crate::geometry::{Point, Shape as _};
        use crate::live::Effect;
        let contour = Effect::Contour {
            steps: 2,
            offset: 1.5,
            outside: true,
            to_center: false,
            fill_to: Color::WHITE,
            outline_to: None,
            color_blend: 0,
            corners: None,
            accel_objects: 0.0,
            accel_colors: 0.0,
        };
        let distort = Effect::Distort(crate::effects::Distort::PushPull {
            amount: 30.0,
            center: None,
        });
        let own = Effect::Perspective {
            corners: [
                Point::new(1.0, 0.0),
                Point::new(10.0, 2.0),
                Point::new(9.0, 10.0),
                Point::new(0.0, 8.0),
            ],
        };
        for effects in [vec![], vec![contour], vec![distort], vec![own]] {
            let mut doc = Document::default();
            let layer = doc.pages[0].layers[0].id;
            let mut a = rect(&mut doc, 0.0);
            a.effects = effects.clone();
            let mut b = rect(&mut doc, 20.0);
            b.transform = Affine::translate((3.0, 4.0)) * Affine::scale_non_uniform(1.5, 0.5);
            let (ia, ib) = (a.id, b.id);
            for s in [a, b] {
                Command::AddShape { layer, shape: s }
                    .apply(&mut doc)
                    .unwrap();
            }
            Command::Group {
                shapes: vec![ia, ib],
            }
            .apply(&mut doc)
            .unwrap();
            let gid = doc.layer(layer).unwrap().shapes[0].id;
            Command::TransformShapes {
                shapes: vec![gid],
                transform: Affine::translate((5.0, 7.0)) * Affine::rotate(0.3),
            }
            .apply(&mut doc)
            .unwrap();
            let bb = doc.find_shape(gid).unwrap().local_path().bounding_box();
            Command::SetEffects {
                shape: gid,
                effects: vec![Effect::Perspective {
                    corners: [
                        Point::new(bb.x0 + 4.0, bb.y0),
                        Point::new(bb.x1 - 2.0, bb.y0 + 1.0),
                        Point::new(bb.x1, bb.y1),
                        Point::new(bb.x0, bb.y1 - 3.0),
                    ],
                }],
            }
            .apply(&mut doc)
            .unwrap();
            // Where each object is drawn in the group, its effects too.
            let main = crate::live::evaluate(doc.find_shape(gid).unwrap()).main;
            let ShapeKind::Group { children } = &main.kind else {
                panic!("a group: {:?}", main.kind);
            };
            let drawn: Vec<_> = children
                .iter()
                .map(|c| {
                    let mut c = c.clone();
                    c.transform_by(main.transform);
                    c.visual_bounds()
                })
                .collect();
            Command::Ungroup { group: gid }.apply(&mut doc).unwrap();
            for (id, want) in [ia, ib].into_iter().zip(drawn) {
                let s = doc.find_shape(id).unwrap();
                let perspectives = s
                    .effects
                    .iter()
                    .filter(|e| matches!(e, Effect::Perspective { .. }))
                    .count();
                assert_eq!(perspectives, 1, "{:?}", s.effects);
                assert!(
                    matches!(s.effects[0], Effect::Perspective { .. }),
                    "{:?}",
                    s.effects
                );
                // The member's own effects stay after it, an own
                // perspective folded in.
                let own_effects = if id == ia { effects.len() } else { 0 };
                let folded =
                    id == ia && matches!(effects.first(), Some(Effect::Perspective { .. }));
                assert_eq!(s.effects.len(), own_effects + usize::from(!folded));
                let got = s.visual_bounds();
                for (g, w) in [
                    (got.x0, want.x0),
                    (got.y0, want.y0),
                    (got.x1, want.x1),
                    (got.y1, want.y1),
                ] {
                    // Contours are fitted with curves within 0.05 mm (#821).
                    assert!((g - w).abs() < 0.05, "{effects:?}: {got:?} != {want:?}");
                }
            }
        }
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
    fn grouping_objects_of_several_layers_keeps_their_drawing_order() {
        let mut doc = Document::default();
        let page = doc.pages[0].id;
        let lower = doc.pages[0].layers[0].id;
        Command::AddLayer {
            page,
            name: "Upper".into(),
        }
        .apply(&mut doc)
        .unwrap();
        let upper = doc.pages[0].layers[1].id;
        // The lower layer holds two objects, the upper one one: the upper
        // object is drawn last although it is first in its layer.
        let a = rect(&mut doc, 0.0);
        let b = rect(&mut doc, 20.0);
        let c = rect(&mut doc, 40.0);
        let (ia, ib, ic) = (a.id, b.id, c.id);
        for (layer, s) in [(lower, a), (lower, b), (upper, c)] {
            Command::AddShape { layer, shape: s }
                .apply(&mut doc)
                .unwrap();
        }
        Command::Group {
            shapes: vec![ic, ib, ia],
        }
        .apply(&mut doc)
        .unwrap();
        let g = &doc.layer(lower).unwrap().shapes[0];
        let ShapeKind::Group { children } = &g.kind else {
            panic!("a group");
        };
        let order: Vec<_> = children.iter().map(|s| s.id).collect();
        assert_eq!(order, vec![ia, ib, ic]);
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
        Command::duplicate_page(page).apply(&mut doc).unwrap();
        assert_eq!(doc.pages.len(), 2);
        let copy = &doc.pages[1];
        assert_eq!(copy.layers[0].shapes.len(), 1);
        assert_ne!(copy.layers[0].shapes[0].id, ia);
        assert_ne!(copy.layers[0].id, layer);
        // A clip frame's frame and contents get new ids too (#377).
        let f = rect(&mut doc, 10.0);
        let c = rect(&mut doc, 12.0);
        let (fid, cid) = (f.id, c.id);
        let clip_id = doc.ids_mut().shape();
        let clip = Shape::new(
            clip_id,
            ShapeKind::ClipFrame {
                frame: Box::new(f),
                contents: vec![c],
            },
        );
        Command::AddShape { layer, shape: clip }
            .apply(&mut doc)
            .unwrap();
        Command::duplicate_page(page).apply(&mut doc).unwrap();
        let copy = &doc.pages[1];
        let copied = copy.layers[0]
            .shapes
            .iter()
            .find(|s| matches!(s.kind, ShapeKind::ClipFrame { .. }))
            .expect("the clip frame copied");
        let ShapeKind::ClipFrame { frame, contents } = &copied.kind else {
            unreachable!()
        };
        assert_ne!(copied.id, clip_id);
        assert_ne!(frame.id, fid);
        assert_ne!(contents[0].id, cid);
    }

    #[test]
    fn duplicate_page_goes_before_or_after_with_or_without_contents() {
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let page = doc.pages[0].id;
        let a = rect(&mut doc, 0.0);
        Command::AddShape { layer, shape: a }
            .apply(&mut doc)
            .unwrap();
        let mut eng = crate::Engine::new(doc);
        eng.run(&Command::DuplicatePage {
            page,
            before: true,
            contents: false,
        })
        .unwrap();
        let d = eng.document();
        assert_eq!(d.pages.len(), 2);
        // The copy is in front of the original, with its layers but empty.
        assert_eq!(d.pages[1].id, page);
        let copy = &d.pages[0];
        assert_ne!(copy.id, page);
        assert_eq!(copy.layers.len(), d.pages[1].layers.len());
        assert_eq!(copy.layers[0].name, d.pages[1].layers[0].name);
        assert!(copy.layers.iter().all(|l| l.shapes.is_empty()));
        assert_eq!(d.pages[1].layers[0].shapes.len(), 1);
        // One undo step takes it away again.
        eng.undo().unwrap();
        assert_eq!(eng.document().pages.len(), 1);
        assert_eq!(eng.document().pages[0].id, page);
        eng.run(&Command::DuplicatePage {
            page,
            before: false,
            contents: true,
        })
        .unwrap();
        let d = eng.document();
        assert_eq!(d.pages[0].id, page);
        assert_eq!(d.pages[1].layers[0].shapes.len(), 1);
        // An unknown page is an error, not a panic.
        assert!(Command::duplicate_page(PageId(9999))
            .apply(&mut Document::default())
            .is_err());
    }

    #[test]
    fn a_saved_duplicate_page_without_options_copies_contents_after() {
        let page = Document::default().pages[0].id;
        let json = serde_json::json!({ "cmd": "duplicate_page", "page": page });
        let cmd: Command = serde_json::from_value(json).unwrap();
        assert_eq!(cmd, Command::duplicate_page(page));
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

    /// The frame inside a clip frame has an id of its own, so it can be
    /// found and reshaped; a fill or outline given to the clip frame goes
    /// to the frame (the object drawn), not to the contents; Extract
    /// Contents gives the frame the clip frame's id back.
    #[test]
    fn a_clip_frames_frame_is_found_and_painted() {
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
        let ShapeKind::ClipFrame { frame, .. } = &doc.shape(ib).unwrap().1.kind else {
            panic!("not a clip frame");
        };
        let fid = frame.id;
        assert_ne!(fid, ib);
        assert!(matches!(
            doc.find_shape(fid).unwrap().kind,
            ShapeKind::Rect { .. }
        ));
        let red = Fill::Solid(Color::rgb8(255, 0, 0));
        Command::SetFill {
            shapes: vec![ib],
            fill: red.clone(),
        }
        .apply(&mut doc)
        .unwrap();
        let stroke = Stroke {
            width: 2.0,
            ..Default::default()
        };
        Command::SetStroke {
            shapes: vec![ib],
            stroke: Some(stroke.clone()),
        }
        .apply(&mut doc)
        .unwrap();
        assert_eq!(doc.find_shape(fid).unwrap().fill, red);
        assert_eq!(doc.find_shape(fid).unwrap().stroke, Some(stroke));
        assert_ne!(
            doc.find_shape(ia).unwrap().fill,
            red,
            "the contents keep theirs"
        );
        // The frame can be reshaped by its id, and stays the frame.
        Command::SetShapeKind {
            shape: fid,
            kind: ShapeKind::Rect {
                rect: Rect::new(5.0, 5.0, 15.0, 15.0),
                radius: 2.0,
                corners: None,
            },
        }
        .apply(&mut doc)
        .unwrap();
        let ShapeKind::ClipFrame { frame, .. } = &doc.shape(ib).unwrap().1.kind else {
            panic!("not a clip frame any more");
        };
        assert!(matches!(frame.kind, ShapeKind::Rect { radius, .. } if radius == 2.0));
        Command::ExtractContents { clip: ib }
            .apply(&mut doc)
            .unwrap();
        assert_eq!(doc.shape(ib).unwrap().1.fill, red);
        assert!(doc.find_shape(fid).is_none());
    }

    #[test]
    fn styled_objects_follow_their_style() {
        use crate::document::{ObjectStyle, StyleParts};
        let mut doc = Document::default();
        let layer = doc.pages[0].layers[0].id;
        let a = rect(&mut doc, 0.0);
        let b = rect(&mut doc, 20.0);
        let c = rect(&mut doc, 40.0);
        let (ia, ib, ic) = (a.id, b.id, c.id);
        for shape in [a, b, c] {
            Command::AddShape { layer, shape }.apply(&mut doc).unwrap();
        }
        let red = Fill::Solid(Color::rgb8(255, 0, 0));
        let mut style = ObjectStyle::from_shape("Red", doc.shape(ia).unwrap().1);
        style.fill = red.clone();
        style.parts = StyleParts {
            fill: true,
            outline: false,
            transparency: true,
        };
        style.opacity = 0.5;
        // A style without an id gets one.
        Command::SetObjectStyles {
            styles: vec![style.clone()],
        }
        .apply(&mut doc)
        .unwrap();
        let id = doc.object_styles[0].id;
        assert_eq!(id, 1);
        let stroke_before = doc.shape(ia).unwrap().1.stroke.clone();
        Command::LinkObjectStyle {
            shapes: vec![ia, ib],
            style: Some(id),
        }
        .apply(&mut doc)
        .unwrap();
        let a = doc.shape(ia).unwrap().1;
        assert_eq!(
            (a.fill.clone(), a.opacity, a.style),
            (red.clone(), 0.5, Some(id))
        );
        // Parts the style leaves out stay as they were.
        assert_eq!(a.stroke, stroke_before);
        // Grouped objects stay linked and follow too.
        Command::Group {
            shapes: vec![ib, ic],
        }
        .apply(&mut doc)
        .unwrap();
        let blue = Fill::Solid(Color::rgb8(0, 0, 255));
        let mut styles = doc.object_styles.clone();
        styles[0].fill = blue.clone();
        Command::SetObjectStyles { styles }.apply(&mut doc).unwrap();
        assert_eq!(doc.shape(ia).unwrap().1.fill, blue);
        assert_eq!(doc.find_shape(ib).unwrap().fill, blue);
        // The unlinked one keeps its own fill.
        assert_ne!(doc.find_shape(ic).unwrap().fill, blue);
        // Breaking the link keeps the properties; later changes pass it by.
        Command::LinkObjectStyle {
            shapes: vec![ia],
            style: None,
        }
        .apply(&mut doc)
        .unwrap();
        let mut styles = doc.object_styles.clone();
        styles[0].fill = red.clone();
        Command::SetObjectStyles { styles }.apply(&mut doc).unwrap();
        let a = doc.shape(ia).unwrap().1;
        assert_eq!((a.fill.clone(), a.style), (blue, None));
        assert_eq!(doc.find_shape(ib).unwrap().fill, red);
        // Deleting the style drops the links; a missing style is an error.
        Command::SetObjectStyles { styles: Vec::new() }
            .apply(&mut doc)
            .unwrap();
        assert_eq!(doc.find_shape(ib).unwrap().style, None);
        assert!(Command::LinkObjectStyle {
            shapes: vec![ia],
            style: Some(7),
        }
        .apply(&mut doc)
        .is_err());
        // Older files: styles without parts set fill and outline.
        let mut json = serde_json::to_value(&style).unwrap();
        for key in ["id", "parts", "opacity"] {
            json.as_object_mut().unwrap().remove(key);
        }
        let old: ObjectStyle = serde_json::from_value(json).unwrap();
        assert_eq!(old.parts, StyleParts::default());
        assert_eq!((old.id, old.opacity), (0, 1.0));
    }

    #[test]
    fn new_objects_take_the_defaults_of_their_kind() {
        use crate::document::{DefaultKind, ObjectDefaults};
        let mut doc = Document::default();
        let mut defaults = ObjectDefaults::default();
        defaults.paragraph_text.fill = Fill::Solid(Color::rgb8(0, 128, 0));
        Command::SetObjectDefaults {
            defaults: defaults.clone(),
        }
        .apply(&mut doc)
        .unwrap();
        assert_eq!(doc.defaults, defaults);
        let mut s = rect(&mut doc, 0.0);
        s.fill = Fill::Solid(Color::BLACK);
        doc.defaults.apply_to(&mut s);
        assert_eq!(s.fill, Fill::None);
        assert_eq!(s.stroke.as_ref().map(|k| k.width), Some(0.2));
        let text = |frame| ShapeKind::Text {
            spans: Vec::new(),
            origin: crate::geometry::Point::ZERO,
            frame,
            align: Default::default(),
            para: Default::default(),
            on_path: None,
        };
        assert_eq!(DefaultKind::of(&text(None)), DefaultKind::ArtisticText);
        let mut p = Shape::new(crate::id::ShapeId(99), text(Some(Size::new(10.0, 10.0))));
        doc.defaults.apply_to(&mut p);
        assert_eq!(p.fill, Fill::Solid(Color::rgb8(0, 128, 0)));
        assert!(p.stroke.is_none());
        // Paragraph layout defaults reach paragraph text, not artistic text.
        doc.defaults.paragraph.columns = 3;
        doc.defaults.paragraph.tabs = vec![10.0];
        let mut p = Shape::new(crate::id::ShapeId(98), text(Some(Size::new(10.0, 10.0))));
        doc.defaults.apply_to(&mut p);
        let mut a = Shape::new(crate::id::ShapeId(97), text(None));
        doc.defaults.apply_to(&mut a);
        let cols = |s: &Shape| match &s.kind {
            ShapeKind::Text { para, .. } => (para.columns, para.tabs.clone()),
            _ => (0, Vec::new()),
        };
        assert_eq!(cols(&p), (3, vec![10.0]));
        assert_eq!(cols(&a), (1, Vec::new()));
        // Older files have none: the defaults of a new drawing.
        let mut json: serde_json::Value = serde_json::from_str(&doc.to_json().unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("defaults");
        let old = Document::from_json(&json.to_string()).unwrap();
        assert_eq!(old.defaults, ObjectDefaults::default());
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

    #[test]
    fn master_layers_reorder_and_delete_down_to_none() {
        let mut doc = Document::default();
        for name in ["Back", "Front"] {
            Command::AddMasterLayer {
                name: name.into(),
                scope: MasterScope::All,
            }
            .apply(&mut doc)
            .unwrap();
        }
        let names =
            |doc: &Document| -> Vec<String> { doc.master.iter().map(|l| l.name.clone()).collect() };
        let back = doc.master[0].id;
        Command::ReorderLayer {
            layer: back,
            index: 1,
        }
        .apply(&mut doc)
        .unwrap();
        assert_eq!(names(&doc), ["Front", "Back"]);
        for l in doc.master.iter().map(|l| l.id).collect::<Vec<_>>() {
            Command::DeleteLayer { layer: l }.apply(&mut doc).unwrap();
        }
        assert!(doc.master.is_empty());
        // A page keeps its last layer.
        assert_eq!(doc.pages[0].layers.len(), 1);
        let last = doc.pages[0].layers[0].id;
        let err = Command::DeleteLayer { layer: last }
            .apply(&mut doc)
            .unwrap_err();
        assert!(matches!(err, Error::LastLayer));
        let err = Command::DeleteLayer { layer: back }
            .apply(&mut doc)
            .unwrap_err();
        assert!(matches!(err, Error::LayerNotFound(_)));
    }

    /// A gradient turns and mirrors with its object, also inside a group,
    /// and so does a gradient transparency; a move or a scale leaves the
    /// fill as it was (#775).
    #[test]
    fn gradients_turn_and_mirror_with_their_object() {
        use crate::color::Color;
        use crate::geometry::{Point, Rect};
        use crate::style::{Fill, Fountain, FountainKind};
        let mut doc = Document::default();
        let page = doc.pages[0].id;
        let layer = doc.page(page).expect("page").layers[0].id;
        let fountain = Fountain::two(FountainKind::Linear, Color::WHITE, Color::BLACK, 0.0);
        let rect = |doc: &mut Document| {
            let mut s = Shape::new(
                doc.ids_mut().shape(),
                ShapeKind::Rect {
                    rect: Rect::new(10.0, 10.0, 50.0, 30.0),
                    radius: 0.0,
                    corners: None,
                },
            );
            s.fill = Fill::Fountain(fountain.clone());
            s
        };
        let a = rect(&mut doc);
        let a_id = a.id;
        let mut b = rect(&mut doc);
        let b_id = b.id;
        b.fill = Fill::None;
        b.effects.push(crate::live::Effect::Transparency {
            mask: Fill::Fountain(fountain.clone()),
            merge: crate::live::MergeMode::Normal,
            target: 2,
        });
        let child = rect(&mut doc);
        let child_id = child.id;
        let group = Shape::new(
            doc.ids_mut().shape(),
            ShapeKind::Group {
                children: vec![child],
            },
        );
        let group_id = group.id;
        let mut eng = crate::Engine::new(doc);
        for shape in [a, b, group] {
            eng.run(&Command::AddShape { layer, shape }).expect("added");
        }
        // The fill's value at the left and right middle of an object.
        let ends = |eng: &crate::Engine, id: ShapeId, mask: bool| {
            let s = eng.document().find_shape(id).expect("shape");
            let f = if mask {
                match s.effects.first() {
                    Some(crate::live::Effect::Transparency { mask, .. }) => mask.clone(),
                    _ => panic!("no transparency"),
                }
            } else {
                s.fill.clone()
            };
            let Fill::Fountain(f) = f else {
                panic!("not a fountain")
            };
            // A fill placed by its angle, as its nodes would place it.
            let b = s.bounds();
            let mut g = f.clone();
            g.handles = Some(f.handles.unwrap_or_else(|| f.default_handles(b)));
            let c = b.center();
            (
                g.param_at(b, Point::new(b.x0, c.y)).expect("left"),
                g.param_at(b, Point::new(b.x1, c.y)).expect("right"),
            )
        };
        let close =
            |x: (f64, f64), y: (f64, f64)| (x.0 - y.0).abs() < 1e-9 && (x.1 - y.1).abs() < 1e-9;
        let start = ends(&eng, a_id, false);
        assert!(close(start, (0.0, 1.0)), "{start:?}");
        // A move and a scale: nothing changes in the fill.
        let moved = Affine::translate((5.0, 7.0)) * Affine::scale_non_uniform(2.0, 0.5);
        let ids = vec![a_id, b_id, group_id];
        eng.run(&Command::TransformShapes {
            shapes: ids.clone(),
            transform: moved,
        })
        .expect("moved");
        let s = eng.document().find_shape(a_id).expect("a");
        assert_eq!(s.fill, Fill::Fountain(fountain.clone()));
        // Half a turn about each centre: the gradient runs right to left.
        let turn = |eng: &crate::Engine, id: ShapeId| {
            let c = eng
                .document()
                .find_shape(id)
                .expect("shape")
                .bounds()
                .center();
            Affine::translate(c.to_vec2())
                * Affine::rotate(std::f64::consts::PI)
                * Affine::translate(-c.to_vec2())
        };
        for id in [a_id, b_id, group_id] {
            let t = turn(&eng, id);
            eng.run(&Command::TransformShapes {
                shapes: vec![id],
                transform: t,
            })
            .expect("turned");
        }
        for (id, mask) in [(a_id, false), (b_id, true), (child_id, false)] {
            let e = ends(&eng, id, mask);
            assert!(close(e, (1.0, 0.0)), "{id:?} {e:?}");
        }
        // Still placed by its angle, now half a turn round, so the angle
        // field of Edit Fill keeps turning it.
        let Fill::Fountain(f) = &eng.document().find_shape(a_id).expect("a").fill else {
            panic!("fountain")
        };
        assert!(f.handles.is_none());
        assert!((f.angle.abs() - 180.0).abs() < 1e-9, "{}", f.angle);
        // A mirror brings it back; undo goes back to the turned fill.
        let c = eng
            .document()
            .find_shape(a_id)
            .expect("a")
            .bounds()
            .center();
        let mirror = Affine::translate((c.x, 0.0))
            * Affine::scale_non_uniform(-1.0, 1.0)
            * Affine::translate((-c.x, 0.0));
        eng.run(&Command::TransformShapes {
            shapes: vec![a_id],
            transform: mirror,
        })
        .expect("mirrored");
        assert!(close(ends(&eng, a_id, false), (0.0, 1.0)));
        eng.undo().expect("undo");
        assert!(close(ends(&eng, a_id, false), (1.0, 0.0)));
        // A quarter turn: the gradient runs up or down, the same at the
        // left and the right.
        let c = eng
            .document()
            .find_shape(a_id)
            .expect("a")
            .bounds()
            .center();
        let quarter = Affine::translate(c.to_vec2())
            * Affine::rotate(std::f64::consts::FRAC_PI_2)
            * Affine::translate(-c.to_vec2());
        eng.run(&Command::TransformShapes {
            shapes: vec![a_id],
            transform: quarter,
        })
        .expect("quarter");
        let (l, r) = ends(&eng, a_id, false);
        assert!((l - r).abs() < 1e-9, "{l} {r}");
        // A turn of 30 degrees: the box changes shape, so the nodes are
        // placed freely, the gradient keeping its run across the object.
        let s = eng.document().find_shape(a_id).expect("a");
        let c = s.bounds().center();
        let Fill::Fountain(before) = s.fill.clone() else {
            panic!("fountain")
        };
        let b0 = s.bounds();
        let mut g = before.clone();
        g.handles = Some(before.handles.unwrap_or_else(|| before.default_handles(b0)));
        let p0 = Point::new(b0.x0 + 3.0, b0.y0 + 5.0);
        let t0 = g.param_at(b0, p0).expect("before");
        let thirty = Affine::translate(c.to_vec2())
            * Affine::rotate(30f64.to_radians())
            * Affine::translate(-c.to_vec2());
        eng.run(&Command::TransformShapes {
            shapes: vec![a_id],
            transform: thirty,
        })
        .expect("thirty");
        let s = eng.document().find_shape(a_id).expect("a");
        let Fill::Fountain(f) = &s.fill else {
            panic!("fountain")
        };
        assert!(f.handles.is_some());
        let t1 = f.param_at(s.bounds(), thirty * p0).expect("after");
        assert!((t0 - t1).abs() < 1e-9, "{t0} {t1}");
    }
}
