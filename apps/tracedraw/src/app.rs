//! Application state. The UI modules under `ui/` draw it; `interaction.rs`
//! turns pointer and keyboard input into commands. Every document change
//! goes through `Engine::run`, so the UI stays thin and replayable.

use crate::theme;
use crate::tools::Tool;
use crate::view::View;
use egui::Pos2;
use std::path::PathBuf;
use tracedraw_core::{
    document::{paper, Shape, ShapeKind, TextSpan},
    geometry::{Affine, Point, Rect, Size, Vec2},
    Color, Command, Engine, Fill, LayerId, PageId, ShapeId, Stroke,
};

/// What a drag with the Free Transform tool does. Modifier keys override
/// the mode: Alt scales, Ctrl skews, Shift reflects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreeTransformMode {
    Rotation,
    Reflection,
    Scale,
    Skew,
}

impl FreeTransformMode {
    pub const ALL: [FreeTransformMode; 4] = [
        FreeTransformMode::Rotation,
        FreeTransformMode::Reflection,
        FreeTransformMode::Scale,
        FreeTransformMode::Skew,
    ];

    pub fn label_key(self) -> &'static str {
        match self {
            FreeTransformMode::Rotation => "toolbar.ft_rotation",
            FreeTransformMode::Reflection => "toolbar.ft_reflection",
            FreeTransformMode::Scale => "toolbar.ft_scale",
            FreeTransformMode::Skew => "toolbar.ft_skew",
        }
    }
}

/// How inserted page numbers look and where they go.
#[derive(Debug, Clone, PartialEq)]
pub struct PageNumberSettings {
    pub start_at: i64,
    pub prefix: String,
    pub suffix: String,
    /// 0 arabic, 1 upper roman, 2 lower roman, 3 upper alpha, 4 lower alpha.
    pub style: u8,
    /// 0 bottom centre, 1 bottom outer corner, 2 top centre, 3 top outer corner.
    pub position: u8,
    pub size_pt: f64,
}

impl Default for PageNumberSettings {
    fn default() -> Self {
        PageNumberSettings {
            start_at: 1,
            prefix: String::new(),
            suffix: String::new(),
            style: 0,
            position: 0,
            size_pt: 12.0,
        }
    }
}

impl PageNumberSettings {
    /// The field these settings make: how the number is written.
    pub fn field(&self) -> tracedraw_core::document::PageNumberField {
        tracedraw_core::document::PageNumberField {
            start_at: self.start_at,
            style: self.style,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
        }
    }

    /// Text for the page at `index` (0-based).
    pub fn label(&self, index: usize) -> String {
        self.field().label(index)
    }
}

thread_local! {
    /// Dots per inch of the pixel unit: the active drawing's resolution.
    static PIXEL_DPI: std::cell::Cell<f64> = const { std::cell::Cell::new(96.0) };
}

/// Set the resolution pixels are measured at (each frame, from the active
/// drawing; the Create a New Document dialog uses its own while drawn).
pub fn set_pixel_dpi(dpi: f64) {
    if dpi.is_finite() && dpi > 0.0 {
        PIXEL_DPI.with(|c| c.set(dpi));
    }
}

/// The resolution pixels are measured at.
pub fn pixel_dpi() -> f64 {
    PIXEL_DPI.with(|c| c.get())
}

thread_local! {
    /// World millimetres per page millimetre: the active drawing's scale
    /// (Document Options > Rulers > Drawing scale), 1 at 1:1.
    static DRAWING_SCALE: std::cell::Cell<f64> = const { std::cell::Cell::new(1.0) };
}

/// Set the drawing scale distances are shown at (each frame, from the
/// active drawing). Values that are not positive and finite mean 1:1.
pub fn set_drawing_scale(world_per_page: f64) {
    let s = if world_per_page.is_finite() && world_per_page > 0.0 {
        world_per_page
    } else {
        1.0
    };
    DRAWING_SCALE.with(|c| c.set(s));
}

/// The drawing scale: world millimetres per page millimetre.
pub fn drawing_scale() -> f64 {
    DRAWING_SCALE.with(|c| c.get())
}

/// Units of measure, in the order the Units list shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Units {
    Inches,
    #[default]
    Millimeters,
    Picas,
    Points,
    Pixels,
    Ciceros,
    Didots,
    Feet,
    Yards,
    Miles,
    Centimeters,
    Meters,
    Kilometers,
}

/// One didot point, mm.
const DIDOT_MM: f64 = 0.376_065;

impl Units {
    pub fn id(self) -> &'static str {
        match self {
            Units::Inches => "in",
            Units::Millimeters => "mm",
            Units::Picas => "pc",
            Units::Points => "pt",
            Units::Pixels => "px",
            Units::Ciceros => "cc",
            Units::Didots => "dd",
            Units::Feet => "ft",
            Units::Yards => "yd",
            Units::Miles => "mi",
            Units::Centimeters => "cm",
            Units::Meters => "m",
            Units::Kilometers => "km",
        }
    }
    pub fn from_id(id: &str) -> Self {
        Units::ALL
            .into_iter()
            .find(|u| u.id() == id)
            .unwrap_or(Units::Millimeters)
    }
    pub const ALL: [Units; 13] = [
        Units::Inches,
        Units::Millimeters,
        Units::Picas,
        Units::Points,
        Units::Pixels,
        Units::Ciceros,
        Units::Didots,
        Units::Feet,
        Units::Yards,
        Units::Miles,
        Units::Centimeters,
        Units::Meters,
        Units::Kilometers,
    ];
    pub fn label(self) -> String {
        crate::i18n::tr(match self {
            Units::Inches => "units.inches",
            Units::Millimeters => "units.millimeters",
            Units::Picas => "units.picas",
            Units::Points => "units.points",
            Units::Pixels => "units.pixels",
            Units::Ciceros => "units.ciceros",
            Units::Didots => "units.didots",
            Units::Feet => "units.feet",
            Units::Yards => "units.yards",
            Units::Miles => "units.miles",
            Units::Centimeters => "units.centimeters",
            Units::Meters => "units.meters",
            Units::Kilometers => "units.kilometers",
        })
    }
    pub fn short(self) -> &'static str {
        match self {
            Units::Inches => "\"",
            Units::Millimeters => "mm",
            Units::Picas => "pc",
            Units::Points => "pt",
            Units::Pixels => "px",
            Units::Ciceros => "c",
            Units::Didots => "dd",
            Units::Feet => "ft",
            Units::Yards => "yd",
            Units::Miles => "mi",
            Units::Centimeters => "cm",
            Units::Meters => "m",
            Units::Kilometers => "km",
        }
    }
    /// Millimetres per unit. A pixel is one dot at the active drawing's
    /// resolution (see [`set_pixel_dpi`]).
    pub fn mm(self) -> f64 {
        match self {
            Units::Inches => 25.4,
            Units::Millimeters => 1.0,
            Units::Picas => 25.4 / 6.0,
            Units::Points => 25.4 / 72.0,
            Units::Pixels => 25.4 / pixel_dpi(),
            Units::Ciceros => 12.0 * DIDOT_MM,
            Units::Didots => DIDOT_MM,
            Units::Feet => 304.8,
            Units::Yards => 914.4,
            Units::Miles => 1_609_344.0,
            Units::Centimeters => 10.0,
            Units::Meters => 1000.0,
            Units::Kilometers => 1_000_000.0,
        }
    }
    pub fn from_mm(self, v: f64) -> f64 {
        v / self.mm()
    }
    /// A font size in points shown in this unit (#529).
    pub fn from_pt(self, pt: f64) -> f64 {
        self.from_mm(pt * 25.4 / 72.0)
    }
    /// A font size typed in this unit, in points.
    pub fn to_pt(self, v: f64) -> f64 {
        self.to_mm(v) * 72.0 / 25.4
    }
    pub fn to_mm(self, v: f64) -> f64 {
        v * self.mm()
    }
    /// Page millimetres per unit of a distance in the drawing: [`mm`]
    /// through the drawing scale (a world metre at 1:100 is 10 page mm).
    ///
    /// [`mm`]: Units::mm
    pub fn world_mm(self) -> f64 {
        self.mm() / drawing_scale()
    }
    /// A distance in the drawing (a position, a size, an offset, a length)
    /// from page millimetres to this unit, through the drawing scale. Page
    /// sizes, outline widths and text sizes use [`from_mm`] instead: they
    /// are as printed.
    ///
    /// [`from_mm`]: Units::from_mm
    pub fn world_from_mm(self, v: f64) -> f64 {
        v / self.world_mm()
    }
    /// A distance in the drawing typed in this unit, as page millimetres.
    pub fn world_to_mm(self, v: f64) -> f64 {
        v * self.world_mm()
    }
}

/// What the modifier keys do while a handle of the selection is dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HandleMods {
    /// Shift: from the centre of the selection instead of the opposite
    /// side.
    pub from_center: bool,
    /// Ctrl: scales in whole steps of 100 % (so crossing the other side
    /// mirrors at the same size), skews in steps of the constrain angle.
    pub steps: bool,
    /// Alt: corner handles scale freely instead of keeping the
    /// proportions.
    pub free: bool,
}

impl HandleMods {
    pub fn of(mods: egui::Modifiers) -> Self {
        HandleMods {
            from_center: mods.shift,
            steps: mods.ctrl,
            free: mods.alt,
        }
    }
}

/// Which handle of the selection box the pointer grabbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

impl Handle {
    pub fn is_corner(self) -> bool {
        matches!(self, Handle::NE | Handle::NW | Handle::SE | Handle::SW)
    }
}

#[derive(Debug, Clone)]
pub enum Drag {
    None,
    /// Drawing a new box shape from `start` (page space); with Shift the
    /// box grows from `start` as its centre.
    Box {
        start: Point,
        current: Point,
        from_center: bool,
    },
    /// Base segment of a 3-point rectangle, ellipse or curve.
    ThreePointBase {
        start: Point,
        current: Point,
    },
    /// Sliding the selected fitted text along its path (and away from
    /// it) with the Pick tool.
    AlongPath {
        start: Point,
        current: Point,
    },
    /// Moving the selection.
    Move {
        /// Where the drag began.
        start: Point,
        last: Point,
        total: Vec2,
        start_bounds: Rect,
        /// The selection's snap point under the pointer when the move
        /// began: it snaps to other objects' points.
        source: Option<Point>,
    },
    /// Scaling the selection with a handle; `anchor` stays fixed (the
    /// centre with Shift).
    Scale {
        handle: Handle,
        anchor: Point,
        start_bounds: Rect,
        /// Where the point of the selection box the handle stands for has
        /// been dragged to (the pointer less `grab`).
        current: Point,
        /// The pointer's offset from that point when the handle was
        /// grabbed: handles stand outside the box, so the box follows the
        /// pointer without a jump.
        grab: tracedraw_core::geometry::Vec2,
        mods: HandleMods,
    },
    /// Skewing the selection with a side handle in rotate mode: the
    /// opposite side stays (the centre line with Shift).
    Skew {
        handle: Handle,
        start_bounds: Rect,
        start: Point,
        current: Point,
        mods: HandleMods,
    },
    /// Moving the centre of rotation in rotate mode.
    Pivot,
    /// Rotating the selection about `center`. `grab` is the selection's
    /// own point (a node) the turn started from in rotate mode, which the
    /// turn carries onto other objects' points and edges (#933); `None`
    /// for a rotate handle.
    Rotate {
        center: Point,
        start_angle: f64,
        current_angle: f64,
        grab: Option<Point>,
    },
    /// Rubber-band selection; with `touching` (Alt) objects it touches
    /// count too.
    Marquee {
        start: Point,
        current: Point,
        touching: bool,
        /// Shift at the press: the objects taken change state (selected
        /// ones are deselected, the others added).
        toggle: bool,
    },
    /// Freehand Pick lasso; with `touching` (Alt) objects it touches count
    /// too.
    Lasso {
        points: Vec<Point>,
        touching: bool,
        /// Shift at the press, as for the marquee.
        toggle: bool,
    },
    /// Freehand stroke being drawn.
    Freehand {
        points: Vec<Point>,
    },
    /// Dragging the selected nodes (Shape tool).
    Node {
        last: Point,
    },
    /// Dragging a corner widget (Shape tool): the corners round as it
    /// moves (#635).
    CornerWidget(Box<crate::corner_widgets::CornerDrag>),
    /// Dragging a control handle (Shape tool).
    Handle {
        shape: ShapeId,
        index: usize,
        which: tracedraw_core::nodes::Which,
    },
    /// Dragging a node of a rectangle, ellipse or polygon (Shape tool):
    /// the object as it was when the drag began; `single` changes one
    /// rectangle corner only; `begun` once the first step is recorded.
    KindNode {
        shape: ShapeId,
        node: crate::kind_nodes::KindNode,
        start: Box<tracedraw_core::Shape>,
        single: bool,
        /// The rectangle corners the drag changes along with the one
        /// dragged (the chosen ones, when it is among them).
        chosen: [bool; 4],
        begun: bool,
        /// Degrees an ellipse's node has turned through.
        turn: f64,
    },
    /// Moving the selected nodes (Shape tool): the curves as they were,
    /// where the drag began and the node it began on.
    NodeMove {
        start: Vec<crate::node_edit::StartCurve>,
        from: Point,
        grab: Point,
        begun: bool,
        /// What stays put of the curves being edited, which the node snaps
        /// to (#673): found at the first move, once a drag.
        still: Option<std::rc::Rc<crate::snap::StillGeometry>>,
    },
    /// Bending a segment (Shape tool): the curve as it was when the drag
    /// began (local path, closed flag), the segment (the element ending
    /// it), the parameter of the point grabbed, where the drag began
    /// (page); `begun` once the first step is recorded.
    SegmentBend {
        shape: ShapeId,
        path: tracedraw_core::BezPath,
        closed: bool,
        index: usize,
        t: f64,
        from: Point,
        begun: bool,
    },
    /// Dragging a node transform handle (Shape tool).
    NodeTransform {
        handle: Handle,
        start: Vec<crate::node_edit::StartCurve>,
        bounds: Rect,
        from: Point,
        begun: bool,
    },
    /// Moving chosen characters of a text (Shape tool): the text as it
    /// was, the characters, where the drag began.
    TextChars {
        start: Box<tracedraw_core::Shape>,
        chars: Vec<usize>,
        from: Point,
        begun: bool,
    },
    /// Dragging a text's spacing arrow (Shape tool): horizontal for
    /// character (or, with `word`, word) spacing, else line spacing.
    TextSpacing {
        start: Box<tracedraw_core::Shape>,
        horizontal: bool,
        word: bool,
        from: Point,
        begun: bool,
    },
    /// Freehand selection of nodes (Shape tool).
    NodeLasso {
        points: Vec<Point>,
    },
    /// Rubber-band selection of nodes (Shape tool).
    NodeMarquee {
        start: Point,
        current: Point,
    },
    /// Interactive fill: linear gradient from start to current.
    FillGradient {
        shape: ShapeId,
        start: Point,
        current: Point,
    },
    /// Moving a fountain handle of the selected object (Interactive Fill).
    FountainHandle {
        handle: crate::fill_tool::FountainHandle,
        /// The drag's undo step is recorded (the next moves join it).
        begun: bool,
    },
    /// Zoom tool rubber band.
    ZoomBox {
        start: Point,
        current: Point,
    },
    /// Contour tool drag sets the offset.
    ContourDrag {
        start: Point,
    },
    /// Connector tool drag from one object to another.
    Connector {
        from: ShapeId,
        start: Point,
        current: Point,
    },
    /// Moving a custom connector anchor (Anchor Editing tool).
    Anchor {
        shape: ShapeId,
        index: usize,
    },
    /// Dragging a drop shadow offset (Drop Shadow tool).
    Shadow {
        shape: ShapeId,
        start: Point,
    },
    /// Dragging a handle of the selected drop shadow (Drop Shadow tool),
    /// to the pointer plus `grab` (zero on a handle; from the press to
    /// the end handle when the shadow itself was taken).
    ShadowHandle {
        handle: crate::shadow_tool::ShadowHandle,
        grab: tracedraw_core::geometry::Vec2,
    },
    /// Dragging a selection across the text being edited (Text tool).
    TextSelect,
    /// Dragging a paragraph text frame (Text tool).
    TextFrame {
        start: Point,
        current: Point,
    },
    /// Dragging a new guideline out of a ruler.
    NewGuide {
        horizontal: bool,
        pos: Point,
    },
    /// The mirror lines of the symmetry being edited, dragged with the
    /// Pick tool: moved (the centre follows the pointer) or, from a
    /// turning handle, turned about the centre.
    SymmetryLines {
        id: ShapeId,
        turn: bool,
        from: Point,
        start_center: Point,
        start_angle: f64,
    },
    /// Dragging an existing guideline: where the press was, the guideline
    /// as it was and as it is now (committed on release).
    MoveGuide {
        index: usize,
        press: Point,
        start: tracedraw_core::document::Guide,
        guide: tracedraw_core::document::Guide,
        /// How far the drag carries the other selected guidelines: the
        /// pressed one's snapped move, the pointer's along a free axis.
        delta: Vec2,
    },
    /// Turning a guideline about its pivot with a rotation handle.
    RotateGuide {
        index: usize,
        pivot: Point,
        guide: tracedraw_core::document::Guide,
    },
    /// Dragging the ruler origin out of the corner between the rulers.
    RulerOrigin {
        pos: Point,
    },
}

/// Steps of the curve being drawn, for Ctrl+Z and Ctrl+Shift+Z while
/// drawing: how it was before each press that changed it (`None`: before
/// its first node), and the states undone. Cleared when the curve is
/// finished, closed or dropped.
#[derive(Debug, Clone, Default)]
pub struct CurveSteps {
    pub undo: Vec<Option<CurveInProgress>>,
    pub redo: Vec<Option<CurveInProgress>>,
}

impl CurveSteps {
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

/// Where a line drawn with the Pen or Bezier tool goes on from: a node
/// (page) and the handle the next segment leaves along.
pub type PenEnd = (Point, Option<Point>);

/// A step that made or changed a curve drawn with the Pen or Bezier tool
/// (#631): the curve, the tool, and where the line goes on from once the
/// step is undone (`undo_to`) or redone (`redo_to`; none when it closed
/// the curve).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PenStep {
    pub shape: ShapeId,
    pub tool: Tool,
    pub undo_to: Option<PenEnd>,
    pub redo_to: Option<PenEnd>,
}

/// Bezier / polyline tool in progress.
#[derive(Debug, Clone, Default)]
pub struct CurveInProgress {
    /// Taken up again by an undo or redo of a Pen or Bezier step (#631):
    /// another undo or redo replaces it.
    pub resumed: bool,
    /// Nodes placed so far with their outgoing handle (dragged with the
    /// Bezier/Pen tool); the incoming handle mirrors the previous node's.
    pub nodes: Vec<(Point, Option<Point>)>,
    /// Polyline tools make straight segments; curve tools make curves.
    pub smooth: bool,
    /// Pointer is being dragged to set the handle of the last node.
    pub dragging_handle: bool,
    /// B-Spline tool: the nodes are control points the curve runs near.
    pub bspline: bool,
    /// Nodes made cusps (clicking the last node again, C while pulling a
    /// handle, a click after a dragged node): their incoming handle as it
    /// was frozen (`None`: the segment arrives straight), no longer the
    /// mirror of their outgoing one.
    pub in_handles: std::collections::BTreeMap<usize, Option<Point>>,
    /// A press on the first node closes the curve: while it is dragged,
    /// where the pointer is (it pulls the closing segment's handle at the
    /// first node; the first node's own outgoing handle stays).
    pub closing: Option<Point>,
    /// Alt held while a handle is pulled moves the node instead: the last
    /// pointer position of that move.
    pub alt_from: Option<Point>,
}

impl CurveInProgress {
    /// The incoming handle of node `i`: frozen for a cusp, else the mirror
    /// of its outgoing handle about the node.
    pub fn in_handle(&self, i: usize) -> Option<Point> {
        if let Some(h) = self.in_handles.get(&i) {
            return *h;
        }
        let (p, out) = *self.nodes.get(i)?;
        out.map(|o| p - (o - p))
    }

    /// Make the last node a cusp: its incoming handle stays as it is now
    /// and it has no outgoing handle, so the next segment leaves it
    /// straight (unless a new handle is pulled).
    pub fn cusp_last(&mut self) {
        let Some(i) = self.nodes.len().checked_sub(1) else {
            return;
        };
        let incoming = self.in_handle(i);
        self.in_handles.insert(i, incoming);
        self.nodes[i].1 = None;
    }

    /// Freeze the last node's incoming handle where it is, so pulling its
    /// handle further moves the outgoing side only (C while dragging).
    pub fn freeze_last_in(&mut self) {
        let Some(i) = self.nodes.len().checked_sub(1) else {
            return;
        };
        let incoming = self.in_handle(i);
        self.in_handles.insert(i, incoming);
    }

    /// The handle the closing drag gives the first node for the closing
    /// segment: where the pointer is, so the segment arrives from the side
    /// the drag goes to (drag left, the handle lies left), and the first
    /// node's own outgoing handle stays (#358). `None` for a press
    /// without a drag: the segment arrives with no handle.
    pub fn closing_handle(&self) -> Option<Point> {
        let first = self.nodes.first()?.0;
        let p = self.closing?;
        ((p - first).hypot() > 1e-9).then_some(p)
    }

    /// A press on the first node closes the curve: from two nodes with
    /// the curve tools that pull handles (a leaf or a lens of two nodes,
    /// #356), from three with the others.
    pub fn closes_at_first(&self) -> bool {
        let needed = if self.smooth && !self.bspline { 2 } else { 3 };
        self.nodes.len() >= needed
    }

    /// Path through the nodes, optionally extended to a preview point.
    pub fn path(&self, preview: Option<Point>) -> tracedraw_core::BezPath {
        self.path_closed(preview, false)
    }

    /// [`Self::path`], closed onto the first node when `closed`.
    pub fn path_closed(&self, preview: Option<Point>, closed: bool) -> tracedraw_core::BezPath {
        if closed && self.bspline {
            let pts: Vec<Point> = self.nodes.iter().map(|n| n.0).collect();
            return tracedraw_core::geometry::bspline_path(&pts, true);
        }
        let mut path = self.open_path(preview);
        if closed {
            // The closing segment: curved when the last node leaves with a
            // handle or the closing drag pulled one at the first node.
            let last = self.nodes.len().saturating_sub(1);
            if let (Some((first, _)), Some((end, end_out))) =
                (self.nodes.first().copied(), self.nodes.last().copied())
            {
                let into = self.closing_handle();
                if self.smooth && last > 0 && (end_out.is_some() || into.is_some()) {
                    path.curve_to(end_out.unwrap_or(end), into.unwrap_or(first), first);
                }
            }
            path.close_path();
        }
        path
    }

    fn open_path(&self, preview: Option<Point>) -> tracedraw_core::BezPath {
        let mut path = tracedraw_core::BezPath::new();
        let mut nodes: Vec<(Point, Option<Point>)> = self.nodes.clone();
        if let Some(p) = preview {
            nodes.push((p, None));
        }
        if self.bspline {
            let pts: Vec<Point> = nodes.iter().map(|n| n.0).collect();
            return tracedraw_core::geometry::bspline_path(&pts, false);
        }
        for (i, (p, _)) in nodes.iter().enumerate() {
            if i == 0 {
                path.move_to(*p);
                continue;
            }
            let (prev, prev_out) = nodes[i - 1];
            let cur_in = if i < self.nodes.len() {
                self.in_handle(i)
            } else {
                None
            };
            match (self.smooth, prev_out, cur_in) {
                (false, _, _) | (true, None, None) => path.line_to(*p),
                (true, po, ci) => path.curve_to(po.unwrap_or(prev), ci.unwrap_or(*p), *p),
            }
        }
        path
    }
}

/// Artistic text being typed.
pub use crate::text_editing::TextEdit;

#[derive(Debug, Clone, Default)]
pub struct Clipboard {
    pub shapes: Vec<Shape>,
    /// The object styles of the drawing copied from, so that pasted
    /// objects keep their links only to the same styles.
    pub styles: Vec<tracedraw_core::ObjectStyle>,
    /// The symbols of the drawing copied from, so that pasted instances
    /// show the same symbols in any drawing (#678).
    pub symbols: Vec<tracedraw_core::Symbol>,
    /// What the system clipboard held right after this copy (see
    /// `clipboard::Mark`).
    pub system: crate::clipboard::Mark,
    /// Guidelines copied with no objects selected: pasted where they were
    /// (#376).
    pub guides: Vec<tracedraw_core::document::Guide>,
}

/// Centres of rotation the user moved. One object's is kept in the
/// object's own space, so it travels with the object when it is moved,
/// scaled or turned; that of several objects together as a place within
/// their bounds (keyed by the objects, in id order). Both stay after the
/// selection changes, so selecting the same objects again finds them.
#[derive(Debug, Clone, Default)]
pub struct Pivots {
    pub single: std::collections::HashMap<ShapeId, Point>,
    pub several: std::collections::HashMap<Vec<ShapeId>, (f64, f64)>,
}

/// The extensions File > Open reads.
pub const OPEN_EXTENSIONS: &[&str] = &[
    "cdr", "cdt", "cdrt", "cmx", "tdraw", "svg", "svgz", "pdf", "ai", "eps", "ps", "prn", "dxf",
    "psd", "psb", "emf", "wmf", "plt", "hpgl", "hgl", "txt", "rtf", "docx",
];

/// The extensions File > Import reads (besides those of installed script
/// filters).
pub const IMPORT_EXTENSIONS: &[&str] = &[
    "cdr", "cdt", "cdrt", "cmx", "svg", "svgz", "pdf", "ai", "eps", "ps", "prn", "dxf", "psd",
    "psb", "emf", "wmf", "plt", "hpgl", "hgl", "txt", "rtf", "docx", "png", "jpg", "jpeg", "bmp",
    "gif", "webp", "tif", "tiff",
];

/// Auto-backups being written on a thread: the folder, and each file's
/// name with how its write went.
struct AutoBackupJob {
    dir: PathBuf,
    thread: std::thread::JoinHandle<Vec<(String, Result<(), String>)>>,
}

pub struct App {
    pub engine: Engine,
    pub page: PageId,
    pub view: View,
    pub tool: Tool,
    pub previous_tool: Tool,
    pub selection: Vec<ShapeId>,
    /// Focus mode: the object being edited on its own (see `focus.rs`).
    pub focus: Vec<ShapeId>,
    /// Second click on a selected object switches to rotate/skew handles.
    pub rotate_mode: bool,
    /// Where the user moved the centre of rotation (otherwise the
    /// selection's centre), kept for each object and each set of objects
    /// after the selection changes (see `Pivots`).
    pub pivots: Pivots,
    pub drag: Drag,
    /// A right click during a move, scale, rotation or skew: on release
    /// the change goes to copies and the originals stay.
    pub drag_copy: bool,
    /// The clip frame under the pointer while the selection is moved
    /// over it: a release there with W adds the selection to it, with K
    /// replaces its contents (see `docs/behavior/clip-frame.md`).
    pub drop_frame: Option<ShapeId>,
    /// W and K held during that move.
    pub drop_keys: (bool, bool),
    /// The Pen or Bezier tool is dragging a node of the selected curve
    /// (as the Shape tool does), not drawing.
    pub pen_editing: bool,
    /// A paste event or paste shortcut press came and its key is still
    /// down (see `interaction::paste_without_text`).
    pub paste_seen: bool,
    pub curve: Option<CurveInProgress>,
    /// Undo and redo steps of `curve`, node by node.
    pub curve_steps: CurveSteps,
    /// The steps that made or changed a curve drawn with the Pen or
    /// Bezier tool, by (engine instance, step id). Undoing or redoing one
    /// takes the curve up again with that tool.
    pub pen_curve_steps: std::collections::HashMap<(u64, u64), PenStep>,
    pub text_edit: Option<TextEdit>,
    /// Text an input method is composing at the caret (Chinese, Japanese,
    /// Korean and other input methods), shown underlined until it is
    /// committed.
    pub ime_preedit: String,
    /// The last trace made (its preset and settings), for Repeat Last
    /// Trace and Open Last Trace Settings (#452).
    pub last_trace: Option<(crate::trace::Preset, crate::trace::Settings)>,
    /// The digits typed with Alt held while editing text, for a character
    /// by its number (#447; the system does it on Windows).
    pub alt_code: String,
    /// The character the last Alt code typed, so the same character
    /// coming from the system as text right after is not typed twice.
    pub alt_composed: Option<char>,
    /// The last colours picked for a fill and for an outline (the
    /// palette's left and right clicks): the "current colours" the first
    /// built-in gradient runs between.
    pub current_colors: (Color, Color),
    /// A fill being dragged in a colour picker: the revision of the step it
    /// made and its colour (see `App::apply_fill_live`).
    pub live_fill: Option<(u64, Option<Color>)>,
    /// An outline being dragged in the outline editor: the revision of the
    /// step it made and its new colour (see
    /// `App::patch_selection_stroke_live`).
    pub live_stroke: Option<(u64, Option<Color>)>,
    pub file: Option<PathBuf>,
    /// The file the drawing was read from, in any format (canonical), until
    /// it is saved under a name of its own: opening that file again shows
    /// this drawing instead of a second copy (#645).
    pub opened_from: Option<PathBuf>,
    /// The drawing's file in a GitHub repository, when it came from one.
    pub remote: Option<crate::github::Remote>,
    /// The last vector object's picture without its bitmap effects, by its
    /// id and state (see `App::vector_base_cached`).
    #[allow(clippy::type_complexity)]
    pub vector_fx_base: std::cell::RefCell<
        Option<(
            (ShapeId, u64),
            (tracedraw_core::SharedBytes, u32, u32, tracedraw_core::Rect),
        )>,
    >,
    /// File > GitHub's session: the token, the request out, the history.
    pub github: crate::github::GithubSession,
    /// The `.cdr` version the active drawing is saved in: the version of
    /// the `.cdr` file it came from, or the one last chosen when saving
    /// it. `None`: the version of Options > Global > File Formats.
    pub cdr_version: Option<u16>,
    /// The `.cdr` version the active drawing was opened from, when it is
    /// one TraceDraw does not save in (older than the checked versions):
    /// Save asks for a version first.
    pub cdr_unsaved_version: Option<u16>,
    /// The `.tdraw` format version the active drawing is saved in: the
    /// version of the file it came from, or the one chosen in Save As.
    /// `None`: the current one.
    pub tdraw_version: Option<u32>,
    /// Save As's choice: the CMYK profile goes into `.cdr` files.
    pub cdr_embed_profile: bool,
    /// Save As's choice: the fonts the text uses go into `.cdr` files
    /// (version 16 and later, faces whose licence allows it).
    pub cdr_embed_fonts: bool,
    /// Save As's choice: the installed fonts the text uses go into
    /// `.tdraw` files (faces whose licence allows it).
    pub tdraw_embed_fonts: bool,
    pub status: String,
    /// The status message that opens Options > General when clicked, while
    /// it is the one shown: the hint that hardware acceleration is off.
    pub status_opens_options: Option<String>,
    pub fit_pending: bool,
    pub units: Units,
    pub nudge_mm: f64,
    pub show_rulers: bool,
    pub show_grid: bool,
    pub show_status_bar: bool,
    pub show_dockers: bool,
    pub docker_tab: DockerTab,
    pub flyout_open: Option<usize>,
    /// The tool each toolbox group shows: the one last used from its flyout.
    pub toolbox_last: Vec<Tool>,
    /// The widest tool name among the toolbox's buttons, as measured when
    /// the toolbox was last drawn with captions (sizes its buttons).
    pub toolbox_caption_w: f32,
    pub clipboard: Option<Clipboard>,
    pub duplicate_offset: Vec2,
    /// The last Duplicate: each original and its copy. While the copies
    /// are the selection, the next Duplicate repeats what was done to
    /// them since (moved, scaled, turned), as the reference program's
    /// smart duplicate does (#559).
    pub last_duplicate: Vec<(ShapeId, ShapeId)>,
    // Defaults for new objects (no fill, black hairline).
    pub polygon_points: u32,
    pub star_sharpness: f64,
    /// Star tool: draw complex stars (sides crossing), their points and
    /// sharpness.
    pub star_complex: bool,
    pub complex_points: u32,
    pub complex_sharpness: u32,
    /// Corners of new rectangles (style, sizes, relative scaling).
    pub rect_corners: tracedraw_core::Corners,
    /// The property bar's lock: one corner size edits them all.
    pub corners_together: bool,
    /// The Shape tool's chosen rectangle corners (a click, Shift or Ctrl
    /// click, or a marquee): dragging one of them changes those corners
    /// only.
    pub rect_corners_selected: Vec<(tracedraw_core::ShapeId, usize)>,
    /// The selection as a marquee (or freehand region) left it. While the
    /// selection still equals it, shaping and joining take the bottom-most
    /// object as their target instead of the one selected last.
    pub marquee_selection: Option<Vec<tracedraw_core::ShapeId>>,
    /// The engine revision clones were last brought up to date at (see
    /// `object_clone.rs`).
    pub clone_sync_revision: u64,
    pub ellipse_arc: Option<tracedraw_core::EllipseArc>,
    /// The Contour tool's and panel's values and the tool's drag.
    pub contour: crate::contour_tool::ContourSettings,
    pub contour_drag: Option<crate::contour_tool::ContourDrag>,
    /// The Blend tool's slider being dragged on the canvas.
    pub blend_handle_drag: Option<crate::blend_tool::BlendHandle>,
    pub spiral_revolutions: u32,
    pub spiral_logarithmic: bool,
    pub common_shape: crate::tools2::CommonShape,
    pub table_rows: u32,
    pub table_cols: u32,
    /// Create New Table's width and height in millimetres.
    pub table_size: (f64, f64),
    /// Cell fill and border of the next table drawn.
    pub table_fill: Fill,
    /// Block shadow colour and gap for the next drag.
    pub block_shadow_color: Color,
    pub block_shadow_gap: f64,
    pub table_border: Option<Stroke>,
    pub graph_rows: u32,
    pub graph_cols: u32,
    pub action_lines_count: u32,
    pub action_lines_radial: bool,
    /// 3-point tools: the base segment already dragged, waiting for the third click.
    pub three_point_base: Option<(Point, Point)>,
    pub media_width: f64,
    pub media_angle: f64,
    pub dimension_points: Vec<Point>,
    pub blend_steps: u32,
    /// The Blend docker's options for the next blend (and the shown one).
    pub blend_options: tracedraw_core::live::BlendOptions,
    /// The Blend docker's step mode: a fixed distance between steps along
    /// a path instead of a step count.
    pub blend_spacing_mm: f64,
    /// Blend docker: the colour acceleration follows the object one.
    pub blend_link_accel: bool,
    /// The Extrude docker's page (camera, rotation, lighting, colour,
    /// bevel).
    pub extrude_page: usize,
    /// Extrude docker: the vanishing point's X and Y are measured from the
    /// object's centre instead of the page origin.
    pub extrude_measure_object: bool,
    /// The Distort tool's values (its bar) and its drag under way.
    pub distort: crate::distort_tool::DistortSettings,
    pub distort_drag: Option<crate::distort_tool::DistortDrag>,
    pub brush_radius: f64,
    /// Eraser nib: thickness in mm and shape.
    pub eraser_width: f64,
    pub eraser_square: bool,
    pub shadow_default: tracedraw_core::Shadow,
    pub text_font: String,
    pub text_size_pt: f64,
    /// The font and size of new text (Options > Text > Display): the
    /// fields above follow the text selected, these do not (#449).
    pub text_default_font: String,
    pub text_default_size: f64,
    pub text_bold: bool,
    pub text_italic: bool,
    pub text_underline: bool,
    pub text_align: tracedraw_core::TextAlign,
    pub eyedropper_color: Option<Color>,
    pub eyedropper_attrs: Option<crate::eyedropper::SampledAttrs>,
    pub eyedropper_groups: crate::eyedropper::AttrGroups,
    /// Apply mode (after a sample) versus Select mode.
    pub eyedropper_apply: bool,
    /// Bitmap sample box side in pixels: 1, 2 or 5.
    pub eyedropper_sample: u32,
    pub canvas_rect: egui::Rect,
    pub pointer_page: Option<Point>,
    /// The palette shown in the bottom strip (the first visible one).
    pub palette: Vec<(String, Color)>,
    /// [`Self::palette`] as an RGB drawing shows it, worked out when the
    /// palette changes (#620).
    pub palette_rgb: Vec<(String, Color)>,
    /// First swatch shown in the bottom strip (its scroll position).
    pub palette_scroll: usize,
    /// The same for the document palette row.
    pub doc_palette_scroll: usize,
    /// The document palette swatch clicked last (Delete color acts on it).
    pub doc_palette_current: Option<usize>,
    /// Pop-up of a palette colour's shades (click and hold a swatch): the
    /// colour and where the swatch is.
    pub palette_shades: Option<(Color, egui::Pos2)>,
    /// All colours of a palette at once (the expand button): which row.
    pub palette_expanded: Option<crate::ui::palette::Row>,
    pub raster: std::cell::RefCell<crate::raster::Raster>,
    /// The visual bounds of objects with live effects, for the document
    /// revision they were measured at (see [`App::visual_bounds`]).
    pub visual_cache: std::cell::RefCell<(u64, std::collections::HashMap<ShapeId, Rect>)>,
    /// The Edit Fill dialog's fill is shown on the selection as an engine
    /// preview (#436).
    pub fill_preview: bool,
    /// The Outline Pen dialog's outline is shown on the selection as an
    /// engine preview (#741).
    pub outline_preview: bool,
    /// The selected objects in page space, for the drawing, its revision
    /// and the selection they were gathered for (see
    /// [`App::selected_rc`]).
    pub selected_cache: std::cell::RefCell<Option<SelectedCache>>,
    /// The selection's outlines flattened for Mask Outside Page, with the
    /// gathered selection and the zoom they were made for (see
    /// [`App::selected_outlines`]).
    #[allow(clippy::type_complexity)]
    pub outline_cache:
        std::cell::RefCell<Option<(std::rc::Rc<Vec<Shape>>, u32, std::rc::Rc<Vec<Vec<Point>>>)>>,
    /// Where the canvas began on screen when it was last drawn, so the
    /// drawing stays put in its window when the window moves (see
    /// `App::anchor_view`).
    pub view_anchor: Option<egui::Pos2>,
    /// How the drawings' windows are arranged (Window > Cascade, Tile
    /// Horizontally, Tile Vertically; tabs otherwise), and the
    /// arrangement and number of windows last laid out.
    pub window_layout: crate::ui::windows::WindowLayout,
    pub window_layout_seen: Option<(crate::ui::windows::WindowLayout, usize)>,
    /// Whether each paragraph text overflows its frame, by document
    /// revision (laying text out every frame would be slow).
    pub overflow_cache: std::cell::RefCell<(u64, std::collections::HashMap<ShapeId, bool>)>,
    /// The corners the Shape tool's corner widgets stand on.
    pub corner_cache: std::cell::RefCell<crate::corner_widgets::CornerCache>,
    pub wireframe: bool,
    /// Families the font lists show: installed, carried by drawings, and
    /// (browser build) offered by the page's font folder.
    pub font_families: Vec<String>,
    /// What the Font Manager loaded and drew.
    pub font_catalog: crate::ui::font_manager::Catalog,
    /// The font lists' pictures of family names in their own fonts.
    pub font_previews: crate::ui::font_list::FontPreviews,
    /// `tracedraw_text::fonts().generation()` when `font_families` was
    /// taken (see `App::sync_fonts`).
    pub font_generation: u64,
    /// The carried CMYK profile the colour engine was installed with
    /// (its data's address), see `App::sync_document_colors`.
    pub color_profile_key: Option<usize>,
    /// The Transformations docker's page and fields.
    pub transform: crate::ui::transform_docker::TransformForm,
    /// Selected nodes: (shape, element index).
    pub node_selection: Vec<(ShapeId, usize)>,
    /// The Shape tool's spot on a segment (a click there): the curve, the
    /// segment (the element ending it) and where along it; Add Node puts
    /// the node there.
    pub segment_spot: Option<(ShapeId, usize, f64)>,
    pub snap: crate::snap::SnapSettings,
    /// The Corners docker's operation and sizes.
    pub corners: crate::corners::CornersSettings,
    /// The Join Curves docker's joint, gap tolerance and radius.
    pub join_settings: crate::corners::JoinSettings,
    /// Outlines a docker previews on the canvas this frame (page space).
    pub docker_preview: Vec<tracedraw_core::geometry::BezPath>,
    /// A point a docker marks on the drawing (the Coordinates docker's
    /// origin point), page space.
    pub docker_preview_point: Option<Point>,
    /// The point the pointer snapped to this frame, for its mark.
    pub snap_mark: std::cell::Cell<Option<crate::snap_points::Target>>,
    /// The alignment and dynamic guides to draw this frame.
    pub live_guides: std::cell::RefCell<crate::live_guides::Overlay>,
    /// The snap points the pointer acquired, newest first: dynamic guides
    /// come from them.
    pub guide_anchors: std::cell::RefCell<Vec<Point>>,
    /// The other objects' boxes for alignment guides, kept while the
    /// drawing, the page, the view and the objects left out stay the same.
    pub align_cache: std::cell::RefCell<Option<crate::snap::AlignCache>>,
    pub dialog: crate::ui::dialogs::Dialog,
    /// The Help page of the dialog being drawn, for its "?" button
    /// (`help.rs`).
    pub dialog_help: &'static str,
    /// Layout > Page Number Settings.
    pub page_numbers: PageNumberSettings,
    /// Last loaded print-merge data, for Edit / Perform without reloading.
    pub merge_state: Option<crate::ui::dialogs::PrintMergeState>,
    /// The Print Merge docker's choices.
    pub merge_docker: crate::ui::print_merge_docker::MergeDocker,
    pub show_welcome: bool,
    /// Object > Clip to Frame > Place Inside Frame is waiting for a click on the frame.
    pub pending_clip_frame: bool,
    /// A frame's flow tab was clicked: the next click says where its text
    /// flows on (#371).
    pub pending_text_link: Option<ShapeId>,
    /// A dialog to go back to when the one opened from it closes (Edit
    /// Line Style from the Outline Pen dialog).
    pub dialog_return: Option<Box<crate::ui::dialogs::Dialog>>,
    /// The Import PDF question was answered for the file about to be read.
    pub pdf_text_chosen: bool,
    /// Where a drag for the new frame of a flow began.
    pub text_link_drag: Option<Point>,
    /// Clip frame being edited in place: (frame id, the page's other
    /// objects when editing began; see `clip_edit_contents`).
    pub clip_frame_edit: Option<(ShapeId, std::collections::HashSet<ShapeId>)>,
    /// The undo steps that began and finished editing a clip frame's
    /// contents, so undo and redo across them leave or enter the edit
    /// (#851).
    pub clip_edit_marks: Vec<crate::ops2::ClipEditMark>,
    /// Object > Symmetry > Edit Symmetry: the object whose symmetry is
    /// being edited.
    pub symmetry_edit: Option<ShapeId>,
    /// The symmetry whose last change made the undo step "Edit Symmetry",
    /// so the next change joins it.
    pub symmetry_step: Option<ShapeId>,
    /// A click on a mirror line shows round handles that turn the lines.
    pub symmetry_turning: bool,
    /// The drawing without the edited symmetry's copies (Show full
    /// preview off), kept for the revision it was made at.
    pub symmetry_doc:
        std::cell::RefCell<Option<(u64, ShapeId, std::rc::Rc<tracedraw_core::Document>)>>,
    pub show_guides: bool,
    /// Selected guidelines of the active page (indices).
    pub selected_guides: Vec<usize>,
    /// The guideline showing rotation handles (second click on it).
    pub guide_rotate: Option<usize>,
    /// Rotation centre of that guideline, where it was clicked.
    pub guide_pivot: Point,
    /// The Guidelines docker's entry fields.
    pub guide_form: crate::ui::layout_options::GuideForm,
    /// The guideline whose values the Guidelines panel's fields hold
    /// (`sync_guide_form`); `None` with none or several selected.
    pub guide_form_for: Option<usize>,
    /// Document Options > Guidelines presets section.
    pub guide_presets: crate::ui::layout_options::PresetForm,
    /// Document Options > Grid shows lines per unit instead of spacing.
    pub grid_frequency: bool,
    // ----- added with the full menu structure -----
    pub settings: crate::settings::Settings,
    pub welcome_tab: WelcomeTab,
    pub fullscreen_preview: bool,
    /// File > Print Preview, while it is open.
    pub print_preview: Option<crate::ui::print_preview::PrintPreview>,
    /// The printer list being read off the interface's thread for the
    /// Print dialog, and the last list read (#614).
    pub printer_fetch: Option<std::sync::mpsc::Receiver<Vec<crate::printers::Printer>>>,
    /// A print on its way to a printer, done off the interface's thread:
    /// a printer's spooler can take minutes and the window stays usable
    /// meanwhile (#810). The status it ends with.
    pub print_job: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    pub printers_seen: Vec<crate::printers::Printer>,
    pub preview_selected_only: bool,
    pub page_sorter: bool,
    /// View > Multipage View: every page laid out beside the current one.
    pub multipage: bool,
    /// The Font Sampler docker.
    pub font_sampler: crate::ui::font_sampler::FontSampler,
    /// View > Draw in Perspective.
    pub perspective: Option<crate::perspective_draw::Session>,
    /// Effects > Effect Picker.
    pub effect_picker: crate::ui::effect_picker::EffectPicker,
    /// The Comments docker's state.
    pub comments: crate::comments::CommentsState,
    /// The Painterly Brush tool's brush and size.
    pub painterly: crate::painterly_tool::PainterlySettings,
    /// The Variable Outline tool's picked node and drag.
    pub vo: crate::variable_outline::VariableOutlineState,
    /// Textures of the other pages in the Multipage View.
    pub page_rasters: std::cell::RefCell<std::collections::HashMap<PageId, crate::raster::Raster>>,
    /// The drawing at a revision, shared by the other pages' renders on
    /// their threads (one copy a change, not one a page) (#903).
    pub page_snapshot: std::cell::RefCell<Option<(u64, std::sync::Arc<tracedraw_core::Document>)>>,
    pub view_mode: ViewMode,
    pub proof_colors: bool,
    pub show_page_border: bool,
    pub simulate_overprints: bool,
    pub rasterize_complex_effects: bool,
    pub show_bleed: bool,
    pub show_printable_area: bool,
    /// View > Mask Outside Page: an opaque colour is drawn over what
    /// lies outside the page, with the page border over the objects, so
    /// the page shows as it will print while every object stays editable
    /// (#615).
    pub mask_outside_page: bool,
    pub show_pixel_grid: bool,
    pub show_baseline_grid: bool,
    pub options_page: crate::ui::dialogs::OptionsPage,
    /// Options > Text's tab (Display, Paragraph Text, Fonts, Spelling,
    /// Autocorrect).
    pub options_text_tab: usize,
    /// What an open options dialog may change, for its Cancel button.
    pub options_snapshot: Option<crate::ui::options::OptionsSnapshot>,
    /// Options > Customization > Commands: the commands listed and the one
    /// waiting for its keys.
    pub shortcut_editor: crate::ui::options::ShortcutEditor,
    /// A menu command to run this frame (its key), from a shortcut of the
    /// user's.
    pub pending_command: Option<String>,
    /// When the last auto-backup ran (set on the first frame).
    pub last_auto_backup: Option<web_time::Instant>,
    /// The auto-backup being written on a thread of its own.
    auto_backup_job: Option<AutoBackupJob>,
    /// Linked bitmaps' files as last seen (`watch_linked_bitmaps`): the
    /// modification time of each, and when they were last looked at.
    pub linked_mtimes: std::collections::HashMap<ShapeId, std::time::SystemTime>,
    pub linked_checked: Option<web_time::Instant>,
    /// Session recovery: this session's marker in the auto-backup folder
    /// (made on the first frame, desktop only).
    pub session: Option<crate::recovery::Session>,
    /// Help > Highlight What's New: the menus mark what is new.
    pub highlight_whats_new: bool,
    /// Help > Updates and the daily check at start.
    pub updates: crate::updates::Checker,
    /// Help > Updates, Install: an AppImage replaced in place.
    pub installer: crate::updates::Installer,
    /// The running check was started at start, not asked for: only a
    /// newer release, not skipped, is shown.
    pub update_quiet: bool,
    /// Page size changes apply to every page (the page bar's All pages).
    pub page_size_all: bool,
    /// The property bar's object origin: (column, row) of the 3 x 3
    /// reference point selector, (1, 1) the centre.
    pub object_origin: (u8, u8),
    /// The property bar keeps width and height proportional.
    pub scale_locked: bool,
    pub pending_copy_properties: bool,
    /// Autofit Page's popup open: the margin shown and the frame it was
    /// drawn in, for the outline of the page it would make (#594).
    pub autofit_preview: Option<(f64, u64)>,
    /// The choices of the last File > Export of this session, which the
    /// next one starts with (#617).
    pub last_export: Option<crate::ui::dialogs::ExportState>,
    /// Print Merge's Place Inside Frame waiting for a click on the shape
    /// that holds the picture (#577).
    pub pending_merge_frame: bool,
    /// Weld, Trim or Intersect waiting for a click on the target, with
    /// what it leaves of the source and the target (#562).
    pub pending_shaping: Option<(crate::ops::Shaping, bool, bool)>,
    pub pending_copy_effect: Option<crate::copy_effect::CopyKind>,
    /// Copy Envelope Properties waits for the object to copy from.
    pub pending_copy_envelope: bool,
    pub pending_clone_effect: Option<EffectKind>,
    /// Document Options > Page Size state.
    pub options_labels: bool,
    /// Navigator pop-up (corner of the scrollbars).
    pub navigator_open: bool,
    /// A right-button drag of the selection and the menu where it ended
    /// (see drop_menu.rs).
    pub right_drag: Option<crate::drop_menu::RightDrag>,
    /// A bar being moved (see ui/bars.rs).
    pub bar_drag: Option<crate::ui::bars::BarDrag>,
    /// The rows of toolbars drawn this frame, for the drops of a dragged
    /// toolbar (see ui/bars.rs).
    pub bar_rows: Vec<crate::ui::bars::RowSpot>,
    /// A toolbar button being moved, and where the bars that take them
    /// were drawn this frame (see ui/bar_items.rs).
    pub bar_item_drag: Option<crate::ui::bar_items::ItemDrag>,
    pub bar_slots: Vec<crate::ui::bar_items::Slot>,
    /// The same, as the last frame left them.
    pub shown_bar_slots: Vec<crate::ui::bar_items::Slot>,
    /// The Time Tracker's clock (see time_tracker.rs).
    pub tracker: crate::time_tracker::Clock,
    pub drop_menu: Option<crate::drop_menu::DropMenu>,
    /// Objects docker: layers shown closed, groups shown open.
    pub objects_collapsed: std::collections::HashSet<tracedraw_core::LayerId>,
    pub objects_open_groups: std::collections::HashSet<ShapeId>,
    /// Objects docker: the page and the Master Page nodes are shown
    /// closed.
    pub objects_page_closed: bool,
    pub objects_master_closed: bool,
    /// When `autosave_settings` last wrote the preferences, and the
    /// preferences as last saved or looked at (JSON), to save them only
    /// when they changed.
    pub settings_written: Option<web_time::Instant>,
    pub settings_saved: Option<Vec<u8>>,
    /// When `autosave_settings` last looked at the preferences (#616).
    pub settings_looked: Option<web_time::Instant>,
    /// A change waits to be written (see `autosave_settings`).
    pub settings_pending: bool,
    /// The layer made active in the Objects docker: new objects go there
    /// while it is one of the current page's layers or a master layer
    /// showing on it, visible and unlocked (see [`App::active_layer`]).
    pub picked_layer: Option<LayerId>,
    pub navigator_tex: Option<egui::TextureHandle>,
    pub options_label_index: usize,
    pub options_current_only: bool,
    /// Selected custom anchor index (Anchor Editing tool).
    pub anchor_sel: Option<usize>,
    /// Object > Order > In Front Of / Behind is waiting for a click: Some(in_front).
    pub pending_order: Option<bool>,
    /// What Edit > Repeat can repeat, newest last, each with the id of
    /// the undo step it made: Repeat takes the newest whose step is still
    /// in effect (an undone action is never repeated, #366).
    pub repeatables: Vec<(u64, Repeatable)>,
    pub show_non_printing: bool,
    pub text_hyphenation: bool,
    /// Macro recording in progress: the commands run since Record started.
    pub recording: Option<Vec<Command>>,
    pub workspace: Workspace,
    pub palettes: Vec<crate::palette::Palette>,
    pub visible_palettes: Vec<usize>,
    pub show_standard_toolbar: bool,
    pub show_property_bar: bool,
    pub show_toolbox: bool,
    pub show_text_toolbar: bool,
    pub show_zoom_toolbar: bool,
    pub show_transform_toolbar: bool,
    /// Right-click context menu anchor (screen position, page position).
    pub context_menu: Option<(egui::Pos2, Point)>,
    /// A locked object right-clicked with a pick tool: the context menu
    /// offers to unlock it (it is not selected, so it cannot be edited).
    pub context_locked: Option<ShapeId>,
    pub scripts_output: Vec<String>,
    pub script_source: String,
    /// Scripts panel debugger: the running script, the line it is paused
    /// at, the breakpoints (lines) and the expression to evaluate.
    pub script_debug: Option<crate::scripting::DebugRun>,
    pub debug_paused: Option<u32>,
    pub breakpoints: std::collections::BTreeSet<u32>,
    pub debug_expr: String,
    /// VBA macro projects, the running macro and its dialogs.
    pub macros: crate::macros::MacroState,
    /// AI Drawing: the request, the key in use, the conversation.
    pub ai: crate::ai::AiSession,
    /// AI Generate and Remove Background.
    pub ai_image: crate::ai_image::ImageSession,
    /// AI agents editing the open drawing (MCP), while allowed.
    #[cfg(not(target_arch = "wasm32"))]
    pub mcp_link: Option<crate::mcp_link::Link>,
    pub find_state: crate::ui::dialogs::FindReplaceState,
    pub step_repeat: crate::ops2::StepRepeat,
    pub align_to: crate::ops2::AlignTo,
    /// Align and Distribute use the boxes with half the outline width.
    pub align_use_outline: bool,
    /// Align and Distribute docker: spread between the outermost objects
    /// or across the page.
    pub distribute_to: crate::ops::DistributeTo,
    pub lens: crate::lens::LensSettings,
    /// How dragging an envelope node reshapes its sides.
    pub envelope_mode: tracedraw_core::envelope::Mode,
    /// The mapping new envelopes take.
    pub envelope_mapping: tracedraw_core::envelope::Mapping,
    /// An envelope node or handle being dragged.
    pub envelope_drag: Option<crate::envelope_edit::EnvelopeDrag>,
    pub bevel: crate::effects_ui::BevelSettings,
    /// The Extrude tool's and panel's values and the tool's drag.
    pub extrude: crate::extrude_tool::ExtrudeSettings,
    pub extrude_drag: Option<crate::extrude_tool::ExtrudeDrag>,
    /// The Crop tool's area waiting to be applied (page space) and its
    /// drag.
    pub crop_area: Option<Rect>,
    pub crop_drag: Option<crate::crop_tool::CropDrag>,
    pub transparency_default: crate::effects_ui::TransparencySettings,
    /// The Glyphs docker's font and character range.
    pub glyph_font: String,
    pub glyph_range: usize,
    /// The Glyphs docker's search: characters or a code point (#375).
    pub glyph_search: String,
    pub table_edit: Option<crate::table::TableEdit>,
    /// The Table tool's drag and bar choices.
    pub table_ui: crate::table_tool::TableUi,
    pub missing_fonts: Vec<String>,
    /// Thumbnails of recent drawings loaded for the Welcome Screen (none
    /// when a drawing has no thumbnail).
    pub recent_thumbs: std::collections::HashMap<PathBuf, Option<egui::TextureHandle>>,
    /// The Shelf docker's items, kept across drawings and sessions.
    pub shelf: crate::shelf::Shelf,
    /// The Export docker's assets.
    pub export_assets: crate::export_assets::ExportAssets,
    /// A colour dragged from a palette (`Some(None)`: No Color).
    pub palette_drag: Option<Option<Color>>,
    /// Where the document palette is this frame (a drop target).
    pub doc_palette_rect: Option<egui::Rect>,
    /// The pointer on screen, wherever it is (drops outside the drawing).
    pub pointer_screen: Option<egui::Pos2>,
    /// The Shaping docker's operation, and what Weld, Trim and Intersect
    /// leave there: [source objects, target] each.
    pub shaping_op: crate::ops::Shaping,
    pub shaping_leave: [[bool; 2]; 3],
    pub color_model: usize,
    pub mixer_color: Color,
    /// The Color docker's views, hue and palette.
    pub color_docker: crate::ui::color_docker::ColorDockerState,
    pub lens_synced_to: Option<ShapeId>,
    pub pending_blend_path: bool,
    /// Text > Fit Text to Path waits for a click on the path.
    pub pending_text_path: bool,
    /// The document palette's eyedropper waits for a click on the drawing.
    pub pending_palette_sample: bool,
    pub envelope_keep_lines: bool,
    pub media_mode: crate::media::MediaMode,
    pub media_preset: usize,
    pub media_spacing: f64,
    pub media_pressure: f64,
    pub media_smoothing: f64,
    /// The Bitmap Mask docker: its colour slots, the slot picked, hide or
    /// show the colours, and the eyedropper filling the picked slot.
    pub mask: crate::bitmap_fx::MaskSettings,
    pub font_filter: String,
    /// Envelope/perspective node being dragged with the Shape tool: (shape, index).
    pub effect_node_drag: Option<(ShapeId, usize)>,
    /// A step of the effect node drag under way is on the undo list.
    pub effect_node_stepped: bool,
    /// The dockers of the default column (at the right; `docker_tab` is
    /// the one in front, `show_dockers` whether the column is open).
    pub open_dockers: Vec<DockerTab>,
    /// The dockers of the column at the other side of the window (the
    /// left), the one in front and whether the column is open.
    pub left_dockers: Vec<DockerTab>,
    pub left_tab: DockerTab,
    pub show_left_dockers: bool,
    /// Dockers floating over the window, each in a window of its own.
    pub floating_dockers: Vec<FloatingDocker>,
    /// A docker being moved by its tab or its title (see ui/dockers.rs).
    pub docker_drag: Option<DockerDrag>,
    /// The default column's front tab and whether it was open, as the
    /// last frame left them: a docker of another place brought to the
    /// front through `docker_tab` is put in front there and the column
    /// goes back to this (see [`App::reconcile_dockers`]).
    pub docker_last: (DockerTab, bool),
    pub free_transform_last: Point,
    pub free_transform_mode: FreeTransformMode,
    pub free_transform_duplicate: bool,
    pub free_transform_reflected: bool,
    pub roughen_amount: f64,
    /// The shaping brushes' settings (see `ui::brush_bars`), 1 to 100 where
    /// a percentage: Smooth's rate, Smear's and Smudge's pressure, Twirl's
    /// speed and turn, Attract and Repel's speed and mode, Roughen's spikes
    /// per 10 mm.
    pub brush: crate::ui::brush_bars::BrushSettings,
    /// A step of the brush stroke under way is on the undo list.
    pub brush_stepped: bool,
    /// The style new dimensions take (the dimension tools' bar).
    pub dimension_style: crate::dimensions::DimensionStyle,
    /// The document revision dimensions were last checked at for moves
    /// to bake (see [`App::bake_dimensions`]).
    pub dimensions_checked: u64,
    /// How the labels of dimensions saved before labels were noted read
    /// when an edit of them in place began (see
    /// [`crate::dimensions::Dimension::read_back`]).
    pub dimension_labels_seen: std::collections::HashMap<ShapeId, crate::dimensions::DrawnLabel>,
    /// A linear dimension's first point was given by pressing to drag.
    pub dimension_dragging: bool,
    /// The style new 3-point callouts take (the callout tool's bar).
    pub callout_style: crate::callouts::CalloutStyle,
    /// The document revision callouts were last fitted at (see
    /// [`App::refresh_callouts`]).
    pub callouts_checked: u64,
    /// The document revision blends were last brought up to date with
    /// their end objects (see [`App::refresh_blends`]).
    pub blends_checked: u64,
    /// Rounded connectors' corner radius, mm.
    pub connector_radius: f64,
    /// The document revision connectors were last checked at (see
    /// [`App::refresh_connectors`]).
    pub connectors_checked: u64,
    /// The revision and selection of the last connector radius change,
    /// so a drag of the field is one undo step.
    pub connector_radius_step: Option<(u64, Vec<ShapeId>)>,
    /// Shape Recognition's levels and Sketch's settings (their bars).
    pub recognize: crate::recognize::RecognizeSettings,
    pub sketch: crate::recognize::SketchSettings,
    /// The revision, objects and effect kind of the last effect edit from
    /// a panel or bar, so a slider drag is one undo step.
    pub effect_edit_step: Option<(u64, Vec<ShapeId>, String)>,
    /// What new blends take besides their steps: rotation (degrees) and
    /// the object and colour accelerations (-1 to 1).
    pub blend_rotation: f64,
    /// The colour node picked on the selected fill's line (Interactive
    /// Fill; an index into its stops), for the node bar and the property
    /// bar's node fields.
    pub fill_node: Option<usize>,
    /// A font shown in the drawing while a font list points at it.
    pub live_font: Option<crate::ui::font_list::LiveFont>,
    pub blend_accel_objects: f64,
    pub blend_accel_colors: f64,
    /// The Knife's mode, gap and pieces (its bar).
    pub knife: crate::knife::KnifeSettings,
    /// Properties docker section to expand: 0 fill, 1 outline.
    /// A section of the Properties docker to expand on the next frame
    /// (0 fill, 1 outline), set by F11 and F12.
    pub properties_open: Option<usize>,
    pub area_fill_color: Color,
    /// Smart Fill: the fill of the objects it makes (0 the default fill,
    /// 1 `area_fill_color`, 2 none) and their outline (0 the default
    /// outline, 1 `area_outline_width` in `area_outline_color`, 2 none).
    pub area_fill_mode: u8,
    pub area_outline_mode: u8,
    pub area_outline_width: f64,
    pub area_outline_color: Color,
    pub mesh_rows: u32,
    pub mesh_cols: u32,
    /// Shape tool elastic mode: dragging one node pulls its neighbours.
    pub elastic_mode: bool,
    /// Shape tool: the node transform mode on the property bar.
    pub node_transform: crate::node_edit::NodeTransformMode,
    /// Shape tool: Reflect Nodes horizontally, vertically.
    pub reflect_nodes: (bool, bool),
    /// Shape tool: the freehand marquee for nodes (else rectangular).
    pub node_lasso: bool,
    /// Shape tool: the last run of nodes Shift+click selected.
    pub node_run: Option<crate::node_edit::NodeRun>,
    /// The Coordinates docker's fields, the "Set ... interactively" pick
    /// it waits for and where that pick's drag began.
    pub coords: crate::coords::CoordsState,
    /// The Vector Mosaic docker's settings.
    pub vector_mosaic: crate::vector_mosaic::VectorMosaicSettings,
    /// The Picture Mosaic docker's settings and indexed library.
    pub picture_mosaic: crate::picture_mosaic::PictureMosaicSettings,
    /// A Picture Mosaic library being indexed in the background.
    pub picture_mosaic_job: Option<crate::picture_mosaic::IndexJob>,
    pub coord_pick: Option<crate::coords::CoordPick>,
    pub coord_drag: Option<Point>,
    /// Shape tool: the Curve smoothness slider and the curves it started from.
    pub curve_smoothness: f64,
    pub smoothing: Option<Vec<crate::node_edit::StartCurve>>,
    /// Last clicked effect/mesh node (palette clicks colour a mesh node).
    pub selected_effect_node: Option<(ShapeId, usize)>,
    /// Open drawings, one tab each (see `documents.rs`); the active one's
    /// state lives in the fields above.
    pub docs: Vec<crate::documents::DocSlot>,
    pub active_doc: usize,
    /// Last "Untitled-N" number handed out this session.
    pub untitled_counter: u32,
    /// Window > Close All (or Exit) is going through the open drawings.
    pub closing_all: bool,
    /// Quit once every drawing is closed (File > Exit with unsaved work).
    pub quit_after_closing: bool,
    /// Close the window on the next frame.
    pub quit_now: bool,
    /// Share of the drawing window's bottom row given to the page tabs
    /// (the rest is the horizontal scrollbar); set with the splitter.
    pub page_tabs_fraction: f32,
    /// The first page tab shown in that row, and the page it last slid
    /// to show: the tabs slide so the current page's tab shows, and the
    /// mouse wheel over them slides them (#794).
    pub page_tabs_first: usize,
    pub page_tabs_followed: Option<tracedraw_core::PageId>,
    /// The Hints docker's page and history.
    pub hints: crate::ui::hints::HintsState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeTab {
    GetStarted,
    Workspace,
    News,
    Learn,
    Templates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Wireframe,
    Normal,
    Enhanced,
    Pixels,
}

impl ViewMode {
    /// The name kept in the preferences.
    pub fn id(self) -> &'static str {
        match self {
            ViewMode::Wireframe => "wireframe",
            ViewMode::Normal => "normal",
            ViewMode::Enhanced => "enhanced",
            ViewMode::Pixels => "pixels",
        }
    }

    /// The mode a name from the preferences stands for (enhanced for an
    /// unknown name).
    pub fn from_id(id: &str) -> ViewMode {
        match id {
            "wireframe" => ViewMode::Wireframe,
            "normal" => ViewMode::Normal,
            "pixels" => ViewMode::Pixels,
            _ => ViewMode::Enhanced,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    Shadow,
    Transparency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workspace {
    Default,
    Lite,
    Classic,
    Illustration,
    PageLayout,
    /// For touch screens: larger bars and buttons, no rulers, panels or
    /// status bar; two fingers pan and pinch zooms.
    Touch,
}

impl Workspace {
    /// Every built-in workspace, in the order the menus list them.
    pub const ALL: [Workspace; 6] = [
        Workspace::Default,
        Workspace::Lite,
        Workspace::Classic,
        Workspace::Illustration,
        Workspace::PageLayout,
        Workspace::Touch,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Workspace::Default => "default",
            Workspace::Lite => "lite",
            Workspace::Classic => "classic",
            Workspace::Illustration => "illustration",
            Workspace::PageLayout => "page_layout",
            Workspace::Touch => "touch",
        }
    }
    pub fn from_id(id: &str) -> Self {
        match id {
            "lite" => Workspace::Lite,
            "classic" => Workspace::Classic,
            "illustration" => Workspace::Illustration,
            "page_layout" => Workspace::PageLayout,
            "touch" => Workspace::Touch,
            _ => Workspace::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerTab {
    Properties,
    Objects,
    Hints,
    Transformations,
    Undo,
    AlignDistribute,
    Shaping,
    StepAndRepeat,
    Text,
    Glyphs,
    Color,
    ColorStyles,
    ObjectStyles,
    FindReplace,
    Scripts,
    Palettes,
    Lens,
    Blend,
    Contour,
    Envelope,
    Extrude,
    Bevel,
    BrushStrokes,
    BitmapMask,
    ObjectData,
    Links,
    Symbols,
    Pages,
    Guidelines,
    Fonts,
    FontSampler,
    Corners,
    JoinCurves,
    Coordinates,
    VectorMosaic,
    PictureMosaic,
    Shelf,
    Export,
    Comments,
    AiGenerate,
    Effects,
    Macros,
    PrintMerge,
}

impl DockerTab {
    pub const ALL: [DockerTab; 43] = [
        DockerTab::Properties,
        DockerTab::Objects,
        DockerTab::Hints,
        DockerTab::Transformations,
        DockerTab::Undo,
        DockerTab::AlignDistribute,
        DockerTab::Shaping,
        DockerTab::StepAndRepeat,
        DockerTab::Text,
        DockerTab::Glyphs,
        DockerTab::Color,
        DockerTab::ColorStyles,
        DockerTab::ObjectStyles,
        DockerTab::FindReplace,
        DockerTab::AiGenerate,
        DockerTab::Export,
        DockerTab::Comments,
        DockerTab::Scripts,
        DockerTab::Palettes,
        DockerTab::Lens,
        DockerTab::Blend,
        DockerTab::Contour,
        DockerTab::Envelope,
        DockerTab::Extrude,
        DockerTab::Bevel,
        DockerTab::BrushStrokes,
        DockerTab::BitmapMask,
        DockerTab::ObjectData,
        DockerTab::Links,
        DockerTab::Symbols,
        DockerTab::Pages,
        DockerTab::Guidelines,
        DockerTab::Fonts,
        DockerTab::FontSampler,
        DockerTab::Corners,
        DockerTab::JoinCurves,
        DockerTab::Coordinates,
        DockerTab::VectorMosaic,
        DockerTab::PictureMosaic,
        DockerTab::Shelf,
        DockerTab::Effects,
        DockerTab::Macros,
        DockerTab::PrintMerge,
    ];

    /// Window > Panels, in groups divided by separators, in the order of
    /// the usual Dockers menu; the effect panels go in their own submenu
    /// (`EFFECTS_MENU`).
    pub const MENU_GROUPS: [&'static [DockerTab]; 9] = [
        &[
            DockerTab::Properties,
            DockerTab::Objects,
            DockerTab::Pages,
            DockerTab::Comments,
            DockerTab::ObjectData,
            DockerTab::ObjectStyles,
            DockerTab::Symbols,
            DockerTab::FindReplace,
            DockerTab::Export,
            DockerTab::PrintMerge,
            DockerTab::AiGenerate,
        ],
        &[DockerTab::Transformations, DockerTab::Coordinates],
        &[DockerTab::Guidelines],
        &[DockerTab::AlignDistribute, DockerTab::StepAndRepeat],
        &[
            DockerTab::Shaping,
            DockerTab::Corners,
            DockerTab::JoinCurves,
        ],
        &[
            DockerTab::Text,
            DockerTab::FontSampler,
            DockerTab::Glyphs,
            DockerTab::Fonts,
        ],
        &[
            DockerTab::Color,
            DockerTab::Palettes,
            DockerTab::ColorStyles,
        ],
        &[DockerTab::Shelf, DockerTab::Links],
        &[
            DockerTab::Undo,
            DockerTab::Hints,
            DockerTab::Scripts,
            DockerTab::Macros,
        ],
    ];

    /// Window > Panels > Effects.
    pub const EFFECTS_MENU: [DockerTab; 11] = [
        DockerTab::Effects,
        DockerTab::Blend,
        DockerTab::Contour,
        DockerTab::Envelope,
        DockerTab::Extrude,
        DockerTab::Bevel,
        DockerTab::Lens,
        DockerTab::BrushStrokes,
        DockerTab::BitmapMask,
        DockerTab::VectorMosaic,
        DockerTab::PictureMosaic,
    ];

    /// i18n key of the docker's title.
    pub fn key(self) -> &'static str {
        match self {
            DockerTab::Properties => "docker.properties",
            DockerTab::Objects => "docker.objects",
            DockerTab::Hints => "docker.hints",
            DockerTab::Transformations => "docker.transformations",
            DockerTab::Undo => "docker.undo",
            DockerTab::AlignDistribute => "docker.align_distribute",
            DockerTab::Shaping => "docker.shaping",
            DockerTab::StepAndRepeat => "docker.step_and_repeat",
            DockerTab::Text => "docker.text",
            DockerTab::Glyphs => "docker.glyphs",
            DockerTab::Color => "docker.color",
            DockerTab::ColorStyles => "docker.color_styles",
            DockerTab::ObjectStyles => "docker.object_styles",
            DockerTab::FindReplace => "docker.find_replace",
            DockerTab::Scripts => "docker.scripts",
            DockerTab::Palettes => "docker.palettes",
            DockerTab::Lens => "docker.lens",
            DockerTab::Blend => "docker.blend",
            DockerTab::Contour => "docker.contour",
            DockerTab::Envelope => "docker.envelope",
            DockerTab::Extrude => "docker.extrude",
            DockerTab::Bevel => "docker.bevel",
            DockerTab::BrushStrokes => "docker.brush_strokes",
            DockerTab::BitmapMask => "docker.bitmap_mask",
            DockerTab::ObjectData => "docker.object_data",
            DockerTab::Links => "docker.links",
            DockerTab::Symbols => "docker.symbols",
            DockerTab::Pages => "docker.pages",
            DockerTab::Guidelines => "docker.guidelines",
            DockerTab::Fonts => "docker.fonts",
            DockerTab::FontSampler => "docker.font_sampler",
            DockerTab::Corners => "docker.corners_docker",
            DockerTab::JoinCurves => "docker.join_curves",
            DockerTab::Coordinates => "docker.coordinates",
            DockerTab::VectorMosaic => "docker.vector_mosaic",
            DockerTab::PictureMosaic => "docker.picture_mosaic",
            DockerTab::Shelf => "docker.shelf",
            DockerTab::Export => "docker.export",
            DockerTab::Comments => "docker.comments",
            DockerTab::AiGenerate => "docker.ai_generate",
            DockerTab::Macros => "docker.macros",
            DockerTab::Effects => "docker.effects",
            DockerTab::PrintMerge => "docker.print_merge",
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            DockerTab::Properties => "Alt+Enter",
            DockerTab::AlignDistribute => "Ctrl+Shift+A",
            DockerTab::Text => "Ctrl+T",
            DockerTab::Glyphs => "Ctrl+F11",
            DockerTab::Color => "Shift+F11",
            DockerTab::Contour => "Ctrl+F9",
            DockerTab::Envelope => "Ctrl+F7",
            DockerTab::Lens => "Alt+F3",
            DockerTab::Symbols => "Ctrl+F3",
            DockerTab::Blend => "Ctrl+F8",
            DockerTab::Extrude => "Ctrl+F10",
            DockerTab::Fonts => "Ctrl+F12",
            DockerTab::FindReplace => "Ctrl+F",
            DockerTab::StepAndRepeat => "Ctrl+Shift+D",
            DockerTab::Transformations => "Alt+F7",
            DockerTab::Undo => "",
            _ => "",
        }
    }

    /// Dockers shown in the tab strip by default (the rest open from menus).
    pub fn default_strip() -> Vec<DockerTab> {
        vec![DockerTab::Hints, DockerTab::Properties, DockerTab::Objects]
    }
}

/// The most outline points Mask Outside Page draws for the selection.
pub const SELECTED_OUTLINE_POINTS: usize = 200_000;

/// The selected objects as [`App::selected_rc`] gathered them: the
/// drawing's engine, its revision and the page, and the selection.
#[derive(Debug, Clone)]
pub struct SelectedCache {
    /// The drawing, its revision, the page and the symmetry edited (its
    /// copies are left out of the bounds while it is).
    key: (u64, u64, tracedraw_core::PageId, Option<ShapeId>),
    selection: Vec<ShapeId>,
    shapes: std::rc::Rc<Vec<Shape>>,
    /// Their visual bounds, once asked for.
    bounds: std::cell::RefCell<Option<std::rc::Rc<Vec<Rect>>>>,
}

/// An action Edit > Repeat can repeat.
#[derive(Debug, Clone, PartialEq)]
pub enum Repeatable {
    /// A command, run again on the selection of the time.
    Command(Box<Command>),
    /// Copies of the selection transformed by the affine (a drag ended
    /// with a right click), made again from the selection of the time.
    Copy(Affine),
}

/// Where a docker lies: the default column (the right side of the
/// window; the left in a right-to-left language), the column at the
/// other side, or a window of its own over the drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerPlace {
    Right,
    Left,
    Floating,
}

/// A docker in a window of its own: its tab, where the window is and how
/// large (screen points).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingDocker {
    pub tab: DockerTab,
    pub pos: Pos2,
    pub size: egui::Vec2,
    /// The window is put at `pos` on the next frame (it was just dropped
    /// or moved); otherwise it keeps where egui has it.
    pub place: bool,
}

impl FloatingDocker {
    /// The size a docker floats at when first dropped.
    pub const DEFAULT_SIZE: egui::Vec2 = egui::vec2(340.0, 480.0);
}

/// A docker being moved: its tab, where on the window (or the tab) it
/// was grabbed, and the last pointer position seen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockerDrag {
    pub tab: DockerTab,
    pub grab: egui::Vec2,
    pub last: Option<Pos2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformTab {
    Position,
    Rotate,
    Scale,
    Size,
    Skew,
}

/// The preferences' JSON, into a buffer of `hint` bytes so that it is
/// not grown on the way (#616).
fn settings_json(s: &crate::settings::Settings, hint: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(hint);
    serde_json::to_writer(&mut out, s).ok()?;
    Some(out)
}

/// Whether this frame's input could have changed a preference: a key,
/// text, a click or a release, the clipboard or a dropped file. Pointer
/// moves, held buttons and the wheel cannot by themselves (a drag ends in
/// a release).
fn input_may_change_settings(i: &egui::InputState) -> bool {
    use egui::Event;
    !i.raw.dropped_files.is_empty()
        || i.events.iter().any(|e| {
            matches!(
                e,
                Event::Key { .. }
                    | Event::Text(_)
                    | Event::Paste(_)
                    | Event::Copy
                    | Event::Cut
                    | Event::Ime(_)
                    | Event::PointerButton { .. }
                    | Event::WindowFocused(_)
            )
        })
}

impl App {
    /// The editor in the window of `ctx`, whose fonts are already
    /// installed (see `theme::install_fonts` and the splash screen).
    pub fn new(ctx: &egui::Context, open: Option<PathBuf>) -> Self {
        // egui quits on Ctrl+Q by default; here Ctrl+Q is Convert to Curves
        // and the application closes through File > Exit (Alt+F4).
        // The Bitmap Effects lens runs the application's bitmap effects.
        tracedraw_render::set_pixel_effects(crate::fx::lens_effects);
        Self::setup_context(ctx);
        let mut app = Self::build(open);
        if !crate::files::WEB {
            app.keep_default_font_installed();
        }
        app
    }

    /// The interface style every window context takes: options, visuals
    /// and text sizes (the headless Help screenshots use it too).
    pub(crate) fn setup_context(ctx: &egui::Context) {
        ctx.options_mut(|o| {
            o.quit_shortcuts.clear();
            // Ctrl+plus and Ctrl+minus zoom the drawing, not the interface;
            // the interface size is an option (see `ui_size`).
            o.zoom_with_keyboard = false;
        });
        // The theme is the one chosen in the options, whatever the
        // system's (the browser's dark mode): egui would otherwise switch
        // to its own dark style whenever the system reports dark.
        ctx.set_theme(egui::ThemePreference::Light);
        ctx.set_visuals_of(egui::Theme::Dark, theme::visuals());
        ctx.set_visuals(theme::visuals());
        ctx.all_styles_mut(|style| {
            // Menus, lists and dialogs appear at once, without fading in
            // (#4).
            style.animation_time = 0.0;
            style.spacing.item_spacing = egui::vec2(4.0, 3.0);
            style.spacing.button_padding = egui::vec2(5.0, 2.0);
            // Interface text at 13 px, the size labels, lists, menus and
            // the status bar use in this font;
            // headings in the bold face.
            use egui::{FontFamily, FontId, TextStyle};
            style.text_styles = [
                (
                    TextStyle::Small,
                    FontId::new(11.0, FontFamily::Proportional),
                ),
                (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
                (
                    TextStyle::Button,
                    FontId::new(13.0, FontFamily::Proportional),
                ),
                (
                    TextStyle::Monospace,
                    FontId::new(13.0, FontFamily::Monospace),
                ),
                (TextStyle::Heading, theme::bold(17.0)),
            ]
            .into();
        });
    }

    /// An app without a window, for tests, scripts and the MCP server.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn headless() -> Self {
        if std::env::var_os("TRACEDRAW_CONFIG_DIR").is_none() {
            std::env::set_var(
                "TRACEDRAW_CONFIG_DIR",
                std::env::temp_dir().join("tracedraw-headless"),
            );
        }
        let mut app = Self::build(None);
        if !app.has_document() {
            app.new_document();
        }
        app.show_welcome = false;
        // Tests read PDFs without the question in between.
        app.settings.formats.pdf_ask_text = false;
        app
    }

    fn build(open: Option<PathBuf>) -> Self {
        let engine = Engine::default();
        let page = engine.document().pages[0].id;
        let mut app = App {
            engine,
            page,
            view: View::default(),
            tool: Tool::Pick,
            previous_tool: Tool::Pick,
            selection: Vec::new(),
            focus: Vec::new(),
            rotate_mode: false,
            pivots: Pivots::default(),
            drag: Drag::None,
            drag_copy: false,
            drop_frame: None,
            drop_keys: (false, false),
            pen_editing: false,
            paste_seen: false,
            curve: None,
            text_edit: None,
            ime_preedit: String::new(),
            last_trace: None,
            alt_code: String::new(),
            alt_composed: None,
            current_colors: (Color::BLACK, Color::WHITE),
            live_fill: None,
            live_stroke: None,
            file: None,
            opened_from: None,
            remote: None,
            vector_fx_base: Default::default(),
            github: Default::default(),
            cdr_version: None,
            cdr_unsaved_version: None,
            tdraw_version: None,
            cdr_embed_profile: true,
            cdr_embed_fonts: false,
            tdraw_embed_fonts: false,
            status: String::new(),
            status_opens_options: None,
            fit_pending: true,
            units: Units::Millimeters,
            nudge_mm: 2.54,
            show_rulers: true,
            show_grid: false,
            show_status_bar: true,
            show_dockers: true,
            docker_tab: DockerTab::Hints,
            flyout_open: None,
            toolbox_last: crate::tools::GROUPS.iter().map(|g| g.tools[0]).collect(),
            toolbox_caption_w: 0.0,
            clipboard: None,
            duplicate_offset: Vec2::new(6.35, 6.35),
            last_duplicate: Vec::new(),
            polygon_points: 5,
            star_sharpness: 0.5,
            star_complex: false,
            complex_points: 9,
            complex_sharpness: 2,
            rect_corners: tracedraw_core::Corners::tool_default(),
            corners_together: true,
            rect_corners_selected: Vec::new(),
            marquee_selection: None,
            clone_sync_revision: 0,
            ellipse_arc: None,
            contour: Default::default(),
            contour_drag: None,
            blend_handle_drag: None,
            spiral_revolutions: 4,
            spiral_logarithmic: false,
            common_shape: crate::tools2::CommonShape::RightArrow,
            table_rows: 3,
            table_cols: 4,
            table_size: (75.0, 100.0),
            table_fill: Fill::None,
            block_shadow_color: Color::cmyk_pct(0.0, 0.0, 0.0, 60.0),
            block_shadow_gap: 0.0,
            table_border: Some(Stroke::hairline(Color::BLACK)),
            graph_rows: 4,
            graph_cols: 3,
            action_lines_count: 12,
            action_lines_radial: false,
            three_point_base: None,
            media_width: 3.0,
            media_angle: 45.0,
            dimension_points: Vec::new(),
            blend_steps: 20,
            blend_options: Default::default(),
            blend_spacing_mm: 5.0,
            blend_link_accel: false,
            extrude_page: 0,
            extrude_measure_object: false,
            distort: Default::default(),
            distort_drag: None,
            brush_radius: 10.0,
            eraser_width: 6.35,
            eraser_square: false,
            shadow_default: tracedraw_core::Shadow::default(),
            text_font: "Arial".into(),
            text_size_pt: 24.0,
            text_default_font: "Arial".into(),
            text_default_size: 24.0,
            text_bold: false,
            text_italic: false,
            text_underline: false,
            text_align: tracedraw_core::TextAlign::Left,
            eyedropper_color: None,
            eyedropper_attrs: None,
            eyedropper_groups: Default::default(),
            eyedropper_apply: false,
            eyedropper_sample: 1,
            canvas_rect: egui::Rect::NOTHING,
            pointer_page: None,
            palette: crate::palette::default_cmyk(),
            palette_rgb: crate::ui::palette::as_rgb(&crate::palette::default_cmyk()),
            palette_scroll: 0,
            doc_palette_scroll: 0,
            doc_palette_current: None,
            palette_shades: None,
            palette_expanded: None,
            raster: std::cell::RefCell::new(crate::raster::Raster::default()),
            visual_cache: std::cell::RefCell::new((u64::MAX, Default::default())),
            fill_preview: false,
            outline_preview: false,
            selected_cache: std::cell::RefCell::new(None),
            outline_cache: std::cell::RefCell::new(None),
            view_anchor: None,
            window_layout: Default::default(),
            window_layout_seen: None,
            overflow_cache: Default::default(),
            corner_cache: Default::default(),
            wireframe: false,
            font_families: tracedraw_text::fonts().available_families(),
            font_catalog: Default::default(),
            font_previews: Default::default(),
            font_generation: tracedraw_text::fonts().generation(),
            color_profile_key: None,
            transform: Default::default(),
            node_selection: Vec::new(),
            segment_spot: None,
            curve_steps: CurveSteps::default(),
            pen_curve_steps: std::collections::HashMap::new(),
            snap: crate::snap::SnapSettings::default(),
            snap_mark: std::cell::Cell::new(None),
            live_guides: Default::default(),
            guide_anchors: Default::default(),
            align_cache: Default::default(),
            corners: Default::default(),
            join_settings: Default::default(),
            docker_preview: Vec::new(),
            docker_preview_point: None,
            dialog: crate::ui::dialogs::Dialog::None,
            dialog_help: crate::help::HOME,
            merge_state: None,
            merge_docker: Default::default(),
            page_numbers: PageNumberSettings::default(),
            show_welcome: false,
            pending_clip_frame: false,
            pending_text_link: None,
            dialog_return: None,
            pdf_text_chosen: false,
            text_link_drag: None,
            clip_frame_edit: None,
            clip_edit_marks: Vec::new(),
            symmetry_edit: None,
            symmetry_step: None,
            symmetry_turning: false,
            symmetry_doc: Default::default(),
            show_guides: true,
            selected_guides: Vec::new(),
            guide_rotate: None,
            guide_pivot: Point::ZERO,
            guide_form: Default::default(),
            guide_form_for: None,
            guide_presets: Default::default(),
            grid_frequency: false,
            settings: crate::settings::Settings::default(),
            welcome_tab: WelcomeTab::GetStarted,
            fullscreen_preview: false,
            print_preview: None,
            printer_fetch: None,
            print_job: None,
            printers_seen: Vec::new(),
            preview_selected_only: false,
            page_sorter: false,
            multipage: false,
            font_sampler: Default::default(),
            perspective: None,
            effect_picker: Default::default(),
            comments: Default::default(),
            painterly: Default::default(),
            vo: Default::default(),
            page_rasters: Default::default(),
            page_snapshot: Default::default(),
            view_mode: ViewMode::Enhanced,
            proof_colors: false,
            show_page_border: true,
            simulate_overprints: false,
            rasterize_complex_effects: true,
            show_bleed: false,
            show_printable_area: false,
            mask_outside_page: false,
            show_pixel_grid: true,
            show_baseline_grid: false,
            options_page: crate::ui::dialogs::OptionsPage::General,
            options_text_tab: 0,
            options_snapshot: None,
            shortcut_editor: Default::default(),
            pending_command: None,
            last_auto_backup: None,
            auto_backup_job: None,
            linked_mtimes: Default::default(),
            linked_checked: None,
            session: None,
            highlight_whats_new: false,
            updates: Default::default(),
            installer: Default::default(),
            update_quiet: false,
            recent_thumbs: Default::default(),
            shelf: Default::default(),
            export_assets: crate::export_assets::ExportAssets::new(),
            palette_drag: None,
            doc_palette_rect: None,
            pointer_screen: None,
            page_size_all: true,
            object_origin: (1, 1),
            scale_locked: false,
            pending_copy_properties: false,
            pending_shaping: None,
            pending_merge_frame: false,
            autofit_preview: None,
            last_export: None,
            pending_copy_effect: None,
            pending_copy_envelope: false,
            pending_clone_effect: None,
            options_labels: false,
            navigator_open: false,
            right_drag: None,
            bar_drag: None,
            bar_rows: Vec::new(),
            bar_item_drag: None,
            bar_slots: Vec::new(),
            shown_bar_slots: Vec::new(),
            tracker: Default::default(),
            drop_menu: None,
            objects_collapsed: Default::default(),
            objects_open_groups: Default::default(),
            settings_written: None,
            settings_saved: None,
            settings_looked: None,
            settings_pending: false,
            objects_page_closed: false,
            objects_master_closed: false,
            picked_layer: None,
            navigator_tex: None,
            options_label_index: 0,
            options_current_only: false,
            anchor_sel: None,
            pending_order: None,
            repeatables: Vec::new(),
            show_non_printing: false,
            text_hyphenation: false,
            recording: None,
            workspace: Workspace::Default,
            palettes: crate::palette::builtin_palettes(),
            visible_palettes: vec![0],
            show_standard_toolbar: true,
            show_property_bar: true,
            show_toolbox: true,
            show_text_toolbar: false,
            show_zoom_toolbar: false,
            show_transform_toolbar: false,
            context_menu: None,
            context_locked: None,
            scripts_output: Vec::new(),
            script_source: String::new(),
            script_debug: None,
            macros: Default::default(),
            debug_paused: None,
            breakpoints: Default::default(),
            debug_expr: String::new(),
            ai: Default::default(),
            ai_image: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            mcp_link: None,
            find_state: Default::default(),
            step_repeat: Default::default(),
            align_to: Default::default(),
            align_use_outline: false,
            distribute_to: Default::default(),
            lens: Default::default(),
            envelope_mode: Default::default(),
            envelope_mapping: Default::default(),
            envelope_drag: None,
            bevel: Default::default(),
            extrude: Default::default(),
            extrude_drag: None,
            crop_area: None,
            crop_drag: None,
            transparency_default: Default::default(),
            glyph_font: String::new(),
            glyph_range: 0,
            glyph_search: String::new(),
            table_edit: None,
            table_ui: Default::default(),
            missing_fonts: Vec::new(),
            shaping_op: crate::ops::Shaping::Weld,
            shaping_leave: crate::settings::PanelPrefs::default().shaping_leave,
            color_model: 1,
            mixer_color: Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
            color_docker: Default::default(),
            lens_synced_to: None,
            pending_blend_path: false,
            pending_text_path: false,
            pending_palette_sample: false,
            envelope_keep_lines: false,
            media_mode: crate::media::MediaMode::Calligraphic,
            media_preset: 0,
            media_spacing: 8.0,
            media_pressure: 0.5,
            media_smoothing: 25.0,
            mask: Default::default(),
            font_filter: String::new(),
            effect_node_drag: None,
            effect_node_stepped: false,
            open_dockers: DockerTab::default_strip(),
            left_dockers: Vec::new(),
            left_tab: DockerTab::Hints,
            show_left_dockers: false,
            floating_dockers: Vec::new(),
            docker_drag: None,
            docker_last: (DockerTab::Hints, true),
            free_transform_last: Point::ZERO,
            free_transform_mode: FreeTransformMode::Rotation,
            free_transform_duplicate: false,
            free_transform_reflected: false,
            roughen_amount: 2.0,
            brush: Default::default(),
            brush_stepped: false,
            dimension_style: Default::default(),
            dimensions_checked: u64::MAX,
            dimension_labels_seen: Default::default(),
            dimension_dragging: false,
            callout_style: Default::default(),
            callouts_checked: 0,
            blends_checked: 0,
            connector_radius: 3.0,
            connectors_checked: u64::MAX,
            connector_radius_step: None,
            recognize: Default::default(),
            sketch: Default::default(),
            effect_edit_step: None,
            blend_rotation: 0.0,
            fill_node: None,
            live_font: None,
            blend_accel_objects: 0.0,
            blend_accel_colors: 0.0,
            knife: Default::default(),
            properties_open: None,
            area_fill_color: Color::cmyk_pct(0.0, 0.0, 100.0, 0.0),
            area_fill_mode: 0,
            area_outline_mode: 0,
            area_outline_width: 0.2,
            area_outline_color: Color::BLACK,
            mesh_rows: 2,
            mesh_cols: 2,
            elastic_mode: false,
            node_transform: Default::default(),
            reflect_nodes: (false, false),
            node_lasso: false,
            node_run: None,
            coords: Default::default(),
            vector_mosaic: Default::default(),
            picture_mosaic: Default::default(),
            picture_mosaic_job: None,
            coord_pick: None,
            coord_drag: None,
            curve_smoothness: 0.0,
            smoothing: None,
            selected_effect_node: None,
            docs: Vec::new(),
            active_doc: 0,
            untitled_counter: 0,
            closing_all: false,
            quit_after_closing: false,
            quit_now: false,
            page_tabs_fraction: 0.45,
            page_tabs_first: 0,
            page_tabs_followed: None,
            hints: Default::default(),
        };
        app.load_settings();
        app.apply_runtime_settings();
        if app.settings.github.remember_token {
            app.github.token = app.settings.github.token.clone();
        }
        // Start-up: the file given on the command line, else what Options >
        // General asks for: the Welcome Screen alone (no drawing open), a
        // new drawing, or the last drawing edited.
        if let Some(p) = open {
            app.open_path(p);
        }
        if !app.has_document() {
            match app.settings.startup {
                crate::settings::Startup::WelcomeScreen => app.show_welcome = true,
                crate::settings::Startup::NewDocument => app.new_document(),
                crate::settings::Startup::LastDocument => {
                    match app.settings.recent_files.first().cloned() {
                        Some(p) if crate::files::exists(&p) => app.open_path(p),
                        _ => {}
                    }
                    if !app.has_document() {
                        app.show_welcome = true;
                    }
                }
            }
        }
        if crate::files::WEB {
            app.show_web_notice();
        }
        app
    }

    /// Each time the browser version opens: a message that it is the same
    /// Rust program compiled for the browser, what is slower or missing
    /// there, and the way to the desktop app.
    pub fn show_web_notice(&mut self) {
        self.dialog = crate::ui::dialogs::Dialog::WebNotice;
    }

    /// A new document with localised default page and layer names.
    pub fn localized_document(title: impl Into<String>, size: Size) -> tracedraw_core::Document {
        let mut doc = tracedraw_core::Document::new(title, size);
        for (i, p) in doc.pages.iter_mut().enumerate() {
            p.name = crate::i18n::trf("doc.page_n", &[("n", &(i + 1).to_string())]);
            for (k, l) in p.layers.iter_mut().enumerate() {
                l.name = crate::i18n::trf("docker.layer_n", &[("n", &(k + 1).to_string())]);
            }
        }
        doc
    }

    /// F11, and a double click on the status bar's fill: the fill
    /// settings (the Properties docker's Fill section).
    pub fn open_fill_editor(&mut self) {
        self.show_dockers = true;
        self.docker_tab = DockerTab::Properties;
        self.properties_open = Some(0);
    }

    /// F12, and a double click on the status bar's outline: the outline
    /// settings (the Properties docker's Outline section).
    pub fn open_outline_editor(&mut self) {
        self.show_dockers = true;
        self.docker_tab = DockerTab::Properties;
        self.properties_open = Some(1);
    }

    /// The resolution of the active drawing, for the pixel unit.
    pub fn document_dpi(&self) -> f64 {
        let dpi = self.doc().metadata.resolution_dpi;
        if dpi > 0.0 {
            dpi
        } else {
            self.settings.default_dpi.max(1.0)
        }
    }

    /// The Options dialog at its General page (hardware acceleration).
    pub fn open_general_options(&mut self) {
        self.options_page = crate::ui::dialogs::OptionsPage::General;
        self.dialog = crate::ui::dialogs::Dialog::Options;
    }

    /// Layout > Page Setup and a double click on the page border or
    /// shadow: the Options dialog at the page size page.
    pub fn open_page_options(&mut self) {
        self.options_page = crate::ui::dialogs::OptionsPage::PageSize;
        self.dialog = crate::ui::dialogs::Dialog::Options;
    }

    /// Settings that act on the editor directly (undo levels), applied
    /// after loading the preferences and when an options dialog closes.
    pub fn apply_runtime_settings(&mut self) {
        crate::ui::writing_tools::install_custom_hyphens(self);
        crate::spell::configure(self.settings.spelling.config());
        let sp = &self.settings.spelling;
        let had = sp.hyphenation_patterns.clone();
        if !crate::spell::use_hyphenation(&had, sp.dictionary_folder.as_deref()) {
            self.status = crate::i18n::trf("status.hyphenation_missing", &[("name", &had)]);
        }
        self.raster.borrow_mut().invalidate();
        let levels = self.settings.undo_levels.max(1);
        self.engine.set_max_history(levels);
        for slot in &mut self.docs {
            slot.engine.set_max_history(levels);
        }
    }

    /// Where auto-backups go unless Options > Save names a folder: a
    /// `TraceDraw` folder in the temporary folder (none in a browser).
    pub fn auto_backup_dir_default() -> Option<PathBuf> {
        if crate::files::WEB {
            return None;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Some(std::env::temp_dir().join("TraceDraw"))
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    }

    /// Double-click on a ruler: Document Options at the Rulers page.
    pub fn open_ruler_options(&mut self) {
        self.options_page = crate::ui::dialogs::OptionsPage::Rulers;
        self.dialog = crate::ui::dialogs::Dialog::Options;
    }

    /// The ruler origin, page coordinates in mm.
    pub fn ruler_origin(&self) -> Point {
        let r = self.doc().metadata.rulers.clone();
        Point::new(r.origin_x, r.origin_y)
    }

    /// A page position as the rulers count it (mm from the ruler origin).
    pub fn to_ruler(&self, p: Point) -> Point {
        let o = self.ruler_origin();
        Point::new(p.x - o.x, p.y - o.y)
    }

    /// A position counted from the ruler origin, back to page coordinates.
    pub fn from_ruler(&self, p: Point) -> Point {
        let o = self.ruler_origin();
        Point::new(p.x + o.x, p.y + o.y)
    }

    /// Move the ruler origin (undoable, saved with the drawing).
    pub fn set_ruler_origin(&mut self, origin: Point) {
        let mut metadata = self.doc().metadata.clone();
        if metadata.rulers.origin_x == origin.x && metadata.rulers.origin_y == origin.y {
            return;
        }
        metadata.rulers.origin_x = origin.x;
        metadata.rulers.origin_y = origin.y;
        if let Err(e) = self
            .engine
            .run_with_label(&Command::SetMetadata { metadata }, "Ruler Origin")
        {
            self.status = format!("Ruler Origin: {e}");
        }
    }

    /// True while the ruler origin is being dragged out of the corner.
    pub fn drag_is_origin(&self) -> bool {
        matches!(self.drag, Drag::RulerOrigin { .. })
    }

    /// The colour around the page (Options > Customization): the theme's
    /// until one is picked (white included, #530).
    pub fn desktop_color(&self) -> egui::Color32 {
        // The default white desktop follows the theme.
        if !self.settings.desktop_custom && self.settings.desktop_rgb == [255, 255, 255] {
            return crate::theme::Tokens::desktop();
        }
        let [r, g, b] = self.settings.desktop_rgb;
        egui::Color32::from_rgb(r, g, b)
    }

    /// A fallback title (a file without a usable name).
    pub fn untitled_name() -> String {
        crate::i18n::trf("doc.untitled_n", &[("n", "1")])
    }

    /// The name the next new drawing will get (without using it up).
    pub fn peek_untitled_name(&self) -> String {
        crate::i18n::trf(
            "doc.untitled_n",
            &[("n", &(self.untitled_counter + 1).to_string())],
        )
    }

    /// Apply persisted settings (language, workspace, snapping, units).
    pub fn load_settings(&mut self) {
        self.apply_settings(crate::settings::Settings::load());
        // Fonts installed for TraceDraw in the Font Manager.
        if !crate::ui::font_manager::install_saved(&self.settings.font_manager).is_empty() {
            self.font_families = tracedraw_text::fonts().available_families();
        }
    }

    /// A font for new text that is no longer installed gives way to Arial,
    /// or the first font there is: text typed in it was drawn with a
    /// stand-in while the bars named the missing font (#504).
    pub fn keep_default_font_installed(&mut self) {
        let installed = |f: &str| self.font_families.iter().any(|g| g.eq_ignore_ascii_case(f));
        if self.font_families.is_empty() || installed(&self.text_default_font) {
            return;
        }
        let Some(font) = ["Arial", "Liberation Sans", "DejaVu Sans", "Selawik"]
            .iter()
            .find(|f| installed(f))
            .map(|f| f.to_string())
            .or_else(|| self.font_families.first().cloned())
        else {
            return;
        };
        log::info!(
            "the font for new text, {}, is not installed: {font} instead",
            self.text_default_font
        );
        if self.text_font == self.text_default_font {
            self.text_font = font.clone();
        }
        self.text_default_font = font;
    }

    /// Apply a set of preferences to the running editor.
    pub fn apply_settings(&mut self, mut s: crate::settings::Settings) {
        s.bars.migrate();
        s.migrate_ai();
        self.ai.key = crate::ai::remembered_key(&s.ai);
        let lang = if s.language.is_empty() {
            crate::i18n::system_language().to_string()
        } else {
            s.language.clone()
        };
        crate::i18n::set_language(&lang);
        self.workspace = Workspace::from_id(&s.workspace);
        self.set_workspace(self.workspace);
        // The bars as they were left, over the workspace's own.
        if let Some(shown) = s.shown_bars {
            self.set_shown_bars(shown);
        }
        // The dockers as they were left: a docker closed stays closed.
        if let Some(setup) = &s.dockers {
            self.set_docker_setup(setup);
        }
        self.snap.grid = s.snap.grid;
        self.snap.guides = s.snap.guides;
        self.snap.objects = s.snap.objects;
        self.snap.page = s.snap.page;
        self.snap.pixels = s.snap.pixels;
        self.snap.baseline_grid = s.snap.baseline_grid;
        self.snap.alignment_guides = s.snap.alignment_guides;
        self.snap.dynamic_guides = s.snap.dynamic_guides;
        self.snap.off = s.snap.off;
        self.apply_tool_prefs(&s.tools);
        self.apply_view_prefs(&s.view);
        self.apply_panel_prefs(&s.panels);
        self.nudge_mm = s.nudge_mm;
        self.apply_toolbox_picks(&s.toolbox);
        self.duplicate_offset = Vec2::new(s.duplicate_offset_mm[0], s.duplicate_offset_mm[1]);
        self.units = Units::from_id(&s.units);
        self.settings = s;
        self.apply_color_settings();
    }

    /// Install the colour engine from the settings: ICC profiles loaded from
    /// disk when paths are set, the built-in sRGB and generic CMYK model
    /// otherwise. Errors fall back to the built-in model with a status line.
    pub fn apply_color_settings(&mut self) {
        use tracedraw_core::color::{engine, IccEngine};
        use tracedraw_core::icc::{Intent, Profile};
        let c = &self.settings.color;
        let load = |path: &str| -> Option<Profile> {
            if path.is_empty() {
                return None;
            }
            match crate::files::read(std::path::Path::new(path)).map_err(|e| e.to_string()) {
                Ok(bytes) => Profile::parse(&bytes).map_err(|e| e.to_string()).ok(),
                Err(_) => None,
            }
        };
        let rgb = load(&c.rgb_profile_path);
        // A drawing that carries its CMYK profile (a `.cdr` file's) shows
        // its CMYK colours through it; others through the settings' one.
        let carried = self.doc().cmyk_profile.clone();
        // The open policy "assign the default profile" ignores the
        // drawing's own one.
        let cmyk = carried_cmyk_profile(self.doc())
            .filter(|_| c.open_cmyk == 0)
            .or_else(|| load(&c.cmyk_profile_path));
        let intent = Intent::from_name(&c.intent).unwrap_or_default();
        let bpc = c.black_point_compensation;
        self.color_profile_key = carried.map(|p| std::sync::Arc::as_ptr(&p.data) as usize);
        tracedraw_core::color::engine::set_map_gray_to_black(c.map_gray_to_black);
        // The built-in formulas, when chosen as the colour engine.
        if c.builtin_engine || (rgb.is_none() && cmyk.is_none()) {
            engine::clear();
            self.raster.borrow_mut().invalidate();
            return;
        }
        engine::install(IccEngine::new(rgb, cmyk, intent, bpc));
        self.raster.borrow_mut().invalidate();
    }

    /// Show the active drawing through the CMYK profile it carries, or the
    /// settings' one when it carries none: called every frame, it installs
    /// the colour engine again only when the drawing's profile changed
    /// (another tab, a drawing opened or reverted).
    pub fn sync_document_colors(&mut self) {
        let key = self
            .doc()
            .cmyk_profile
            .as_ref()
            .map(|p| std::sync::Arc::as_ptr(&p.data) as usize);
        if key != self.color_profile_key {
            self.apply_color_settings();
        }
    }

    /// The CMYK profile Colour Management loaded from a file, as a
    /// drawing would carry it (`None` for the built-in model).
    pub fn settings_cmyk_profile(&self) -> Option<tracedraw_core::DocumentProfile> {
        let c = &self.settings.color;
        if c.cmyk_profile_path.is_empty() {
            return None;
        }
        let data = crate::files::read(std::path::Path::new(&c.cmyk_profile_path)).ok()?;
        tracedraw_core::icc::Profile::parse(&data).ok()?;
        Some(tracedraw_core::DocumentProfile::new(
            c.cmyk_profile.clone(),
            data,
        ))
    }

    /// Pick an `.icc`/`.icm` file for the default grayscale profile.
    pub fn load_gray_profile(&mut self) {
        crate::files::Dialog::new()
            .add_filter("ICC", &["icc", "icm"])
            .pick_file(self, move |app, path| {
                let path = crate::files::keep(&path);
                match crate::files::read(&path)
                    .ok()
                    .and_then(|b| tracedraw_core::icc::Profile::parse(&b).ok())
                {
                    Some(p) => {
                        let name = p
                            .description()
                            .map(str::to_string)
                            .unwrap_or_else(|| path.display().to_string());
                        app.settings.color.gray_profile = name.clone();
                        app.settings.color.gray_profile_path = path.display().to_string();
                        app.status = crate::i18n::trf("status.icc_loaded", &[("name", &name)]);
                    }
                    None => {
                        app.status = crate::i18n::trf("status.icc_failed", &[("e", "ICC")]);
                    }
                }
            });
    }

    /// Pick an `.icc`/`.icm` file for the RGB or CMYK slot.
    pub fn load_icc_profile(&mut self, cmyk: bool) {
        crate::files::Dialog::new()
            .add_filter("ICC", &["icc", "icm"])
            .pick_file(self, move |app, path| {
                let path = crate::files::keep(&path);
                app.use_icc_profile(cmyk, path);
            });
    }

    fn use_icc_profile(&mut self, cmyk: bool, path: PathBuf) {
        let bytes = match crate::files::read(&path) {
            Ok(b) => b,
            Err(e) => {
                self.status = crate::i18n::trf("status.icc_failed", &[("e", &e.to_string())]);
                return;
            }
        };
        match tracedraw_core::icc::Profile::parse(&bytes) {
            Ok(p) => {
                let name = p
                    .description()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| path.display().to_string());
                let c = &mut self.settings.color;
                if cmyk {
                    c.cmyk_profile = name.clone();
                    c.cmyk_profile_path = path.display().to_string();
                } else {
                    c.rgb_profile = name.clone();
                    c.rgb_profile_path = path.display().to_string();
                }
                self.apply_color_settings();
                self.status = crate::i18n::trf("status.icc_loaded", &[("name", &name)]);
            }
            Err(e) => {
                self.status = crate::i18n::trf("status.icc_failed", &[("e", &e.to_string())]);
            }
        }
    }

    /// Every preference back to how TraceDraw comes (history kept: see
    /// `Settings::reset_keeping_history`), applied and saved: shortcuts,
    /// bars, toolbox, palette options, interface size and the rest.
    /// Drawings are not touched.
    pub fn reset_settings(&mut self) {
        let fresh = self.settings.reset_keeping_history();
        self.toolbox_last = crate::tools::GROUPS.iter().map(|g| g.tools[0]).collect();
        self.apply_settings(fresh);
        self.show_page_border = true;
        self.shortcut_editor = Default::default();
        self.recent_thumbs.clear();
        self.apply_runtime_settings();
        self.save_settings();
        self.status = crate::i18n::tr("status.settings_reset");
    }

    pub fn save_settings(&mut self) {
        self.sync_settings();
        self.settings.save();
        let hint = self.settings_saved.as_ref().map_or(0, Vec::len);
        self.settings_saved = settings_json(&self.settings, hint);
        self.settings_pending = false;
    }

    /// Copy the editor's state (workspace, snapping, toolbox, units...)
    /// into the preferences, without writing them.
    pub fn sync_settings(&mut self) {
        self.settings.workspace = self.workspace.id().into();
        self.settings.snap.grid = self.snap.grid;
        self.settings.snap.guides = self.snap.guides;
        self.settings.snap.objects = self.snap.objects;
        self.settings.snap.page = self.snap.page;
        self.settings.snap.pixels = self.snap.pixels;
        self.settings.snap.baseline_grid = self.snap.baseline_grid;
        self.settings.snap.alignment_guides = self.snap.alignment_guides;
        self.settings.snap.dynamic_guides = self.snap.dynamic_guides;
        self.settings.snap.off = self.snap.off;
        self.settings.tools = self.tool_prefs();
        self.settings.view = self.view_prefs();
        self.settings.panels = self.panel_prefs();
        self.settings.nudge_mm = self.nudge_mm;
        self.settings.toolbox = self
            .toolbox_last
            .iter()
            .map(|t| t.id().to_string())
            .collect();
        self.settings.duplicate_offset_mm = [self.duplicate_offset.x, self.duplicate_offset.y];
        self.settings.units = self.units.id().into();
        self.settings.language = crate::i18n::language();
        self.settings.shown_bars = Some(self.shown_bars());
        self.settings.dockers = Some(self.docker_setup());
    }

    /// The tool settings now (see [`crate::settings::ToolPrefs`]).
    pub fn tool_prefs(&self) -> crate::settings::ToolPrefs {
        crate::settings::ToolPrefs {
            polygon_points: self.polygon_points,
            star_sharpness: self.star_sharpness,
            star_complex: self.star_complex,
            complex_points: self.complex_points,
            complex_sharpness: self.complex_sharpness,
            rect_corners: self.rect_corners,
            corners_together: self.corners_together,
            ellipse_arc: self.ellipse_arc,
            spiral_revolutions: self.spiral_revolutions,
            spiral_logarithmic: self.spiral_logarithmic,
            table_rows: self.table_rows,
            table_cols: self.table_cols,
            graph_rows: self.graph_rows,
            graph_cols: self.graph_cols,
            blend_steps: self.blend_steps,
            contour: self.contour,
            eraser_width: self.eraser_width,
            eraser_square: self.eraser_square,
            text_font: self.text_default_font.clone(),
            text_size_pt: self.text_default_size,
            text_hyphenation: self.text_hyphenation,
            envelope_mode: self.envelope_mode.id().into(),
            envelope_mapping: self.envelope_mapping.id().into(),
            envelope_keep_lines: self.envelope_keep_lines,
            mesh_rows: self.mesh_rows,
            mesh_cols: self.mesh_cols,
            smart_fill_mode: self.area_fill_mode,
            smart_fill_color: self.area_fill_color,
            smart_outline_mode: self.area_outline_mode,
            smart_outline_width: self.area_outline_width,
            smart_outline_color: self.area_outline_color,
            brush_radius: self.brush_radius,
            roughen_amount: self.roughen_amount,
            brush: self.brush,
            dimension_style: self.dimension_style.clone(),
            callout_style: self.callout_style.clone(),
            distort: self.distort,
            extrude: self.extrude,
            connector_radius: self.connector_radius,
            recognize: self.recognize,
            sketch: self.sketch,
            knife: self.knife,
        }
    }

    /// Take the tool settings of `t`, each brought into its range (a
    /// preferences file edited by hand cannot ask for a 0-sided polygon).
    pub fn apply_tool_prefs(&mut self, t: &crate::settings::ToolPrefs) {
        let finite = |v: f64, lo: f64, hi: f64, or: f64| {
            if v.is_finite() {
                v.clamp(lo, hi)
            } else {
                or
            }
        };
        self.polygon_points = t.polygon_points.clamp(3, 500);
        self.star_sharpness = finite(t.star_sharpness, 0.0, 1.0, 0.5);
        self.star_complex = t.star_complex;
        use tracedraw_core::geometry::{complex_star_max_sharpness, COMPLEX_STAR_MIN_POINTS};
        self.complex_points = t.complex_points.clamp(COMPLEX_STAR_MIN_POINTS, 500);
        self.complex_sharpness = t
            .complex_sharpness
            .clamp(1, complex_star_max_sharpness(self.complex_points));
        self.rect_corners = t.rect_corners;
        self.corners_together = t.corners_together;
        self.ellipse_arc = t.ellipse_arc;
        self.spiral_revolutions = t.spiral_revolutions.clamp(1, 100);
        self.spiral_logarithmic = t.spiral_logarithmic;
        self.table_rows = t.table_rows.clamp(1, 1000);
        self.table_cols = t.table_cols.clamp(1, 1000);
        self.graph_rows = t.graph_rows.clamp(1, 1000);
        self.graph_cols = t.graph_cols.clamp(1, 1000);
        self.blend_steps = t.blend_steps.clamp(1, 999);
        self.contour = t.contour.clamped();
        self.eraser_width = finite(t.eraser_width, 0.025, 2540.0, 6.35);
        self.eraser_square = t.eraser_square;
        if !t.text_font.trim().is_empty() {
            self.text_font = t.text_font.clone();
        }
        self.text_size_pt = finite(t.text_size_pt, 0.5, 3000.0, 24.0);
        self.text_default_font = self.text_font.clone();
        self.text_default_size = self.text_size_pt;
        self.text_hyphenation = t.text_hyphenation;
        self.envelope_mode = tracedraw_core::envelope::Mode::from_id(&t.envelope_mode);
        self.envelope_mapping = tracedraw_core::envelope::Mapping::from_id(&t.envelope_mapping);
        self.envelope_keep_lines = t.envelope_keep_lines;
        self.mesh_rows = t.mesh_rows.clamp(1, 50);
        self.mesh_cols = t.mesh_cols.clamp(1, 50);
        self.area_fill_mode = t.smart_fill_mode.min(2);
        self.area_fill_color = t.smart_fill_color;
        self.area_outline_mode = t.smart_outline_mode.min(2);
        self.area_outline_width = finite(t.smart_outline_width, 0.0, 1000.0, 0.2);
        self.area_outline_color = t.smart_outline_color;
        self.brush_radius = finite(t.brush_radius, 0.5, 500.0, 10.0);
        self.roughen_amount = finite(t.roughen_amount, 0.05, 50.0, 2.0);
        self.brush = t.brush.clamped();
        self.dimension_style = crate::dimensions::DimensionStyle {
            precision: t.dimension_style.precision.min(6),
            text_size: finite(t.dimension_style.text_size, 1.0, 500.0, 10.0),
            ..t.dimension_style.clone()
        };
        self.connector_radius = finite(t.connector_radius, 0.0, 1000.0, 3.0);
        self.callout_style = t.callout_style.clamped();
        self.distort = t.distort.clamped();
        self.extrude = t.extrude.clamped();
        self.recognize = crate::recognize::RecognizeSettings {
            recognition: t.recognize.recognition.min(5),
            smoothing: t.recognize.smoothing.min(5),
        };
        self.knife = crate::knife::KnifeSettings {
            gap: finite(t.knife.gap, 0.0, 1000.0, 0.0),
            ..t.knife
        };
        self.sketch = crate::recognize::SketchSettings {
            join_distance: finite(t.sketch.join_distance, 0.0, 1000.0, 5.0),
            smoothing: t.sketch.smoothing.min(5),
            ..t.sketch
        };
    }

    /// What the drawing window shows now (see
    /// [`crate::settings::ViewPrefs`]).
    pub fn view_prefs(&self) -> crate::settings::ViewPrefs {
        crate::settings::ViewPrefs {
            grid: self.show_grid,
            guides: self.show_guides,
            page_border: self.show_page_border,
            bleed: self.show_bleed,
            printable_area: self.show_printable_area,
            mask_outside_page: self.mask_outside_page,
            pixel_grid: self.show_pixel_grid,
            baseline_grid: self.show_baseline_grid,
            non_printing: self.show_non_printing,
            proof_colors: self.proof_colors,
            simulate_overprints: self.simulate_overprints,
            rasterize_complex_effects: self.rasterize_complex_effects,
            mode: self.view_mode.id().into(),
            grid_frequency: self.grid_frequency,
        }
    }

    pub fn apply_view_prefs(&mut self, v: &crate::settings::ViewPrefs) {
        self.show_grid = v.grid;
        self.show_guides = v.guides;
        self.show_page_border = v.page_border;
        self.show_bleed = v.bleed;
        self.show_printable_area = v.printable_area;
        self.mask_outside_page = v.mask_outside_page;
        self.show_pixel_grid = v.pixel_grid;
        self.show_baseline_grid = v.baseline_grid;
        self.show_non_printing = v.non_printing;
        self.proof_colors = v.proof_colors;
        self.simulate_overprints = v.simulate_overprints;
        self.rasterize_complex_effects = v.rasterize_complex_effects;
        self.view_mode = ViewMode::from_id(&v.mode);
        self.grid_frequency = v.grid_frequency;
    }

    /// Dockers' own choices now (see [`crate::settings::PanelPrefs`]).
    pub fn panel_prefs(&self) -> crate::settings::PanelPrefs {
        crate::settings::PanelPrefs {
            color_model: crate::ui::color_docker::MODELS
                .get(self.color_model)
                .copied()
                .unwrap_or("CMYK")
                .into(),
            color_views: self.color_docker.shown,
            align_to: self.align_to.id().into(),
            align_use_outline: self.align_use_outline,
            distribute_to: self.distribute_to.id().into(),
            shaping_op: self.shaping_op.id().into(),
            shaping_leave: self.shaping_leave,
        }
    }

    pub fn apply_panel_prefs(&mut self, p: &crate::settings::PanelPrefs) {
        if let Some(i) = crate::ui::color_docker::MODELS
            .iter()
            .position(|m| *m == p.color_model)
        {
            if i != self.color_model {
                self.color_model = i;
                self.mixer_color = self.mixer_color.convert_to(&p.color_model);
            }
        }
        self.color_docker.shown = p.color_views;
        self.align_to = crate::ops2::AlignTo::from_id(&p.align_to);
        self.align_use_outline = p.align_use_outline;
        self.distribute_to = crate::ops::DistributeTo::from_id(&p.distribute_to);
        self.shaping_op = crate::ops::Shaping::from_id(&p.shaping_op);
        self.shaping_leave = p.shaping_leave;
    }

    /// Save the preferences when they changed: looked at every frame, so a
    /// change in Options, the menus, the bars or the dockers lasts even
    /// when TraceDraw does not close normally (and in the browser, where
    /// closing the tab saves nothing); written at most once a second (a
    /// window being resized changes them every frame), with a frame asked
    /// for when a write has to wait. Not while Options is open: its Cancel
    /// puts them back. Looked at on the frames whose input could change
    /// them (a key, a click, text, a dropped file) and at most once a
    /// second otherwise, not on every pointer move or animation frame,
    /// which cost a full serialization each on slow computers (#616); a
    /// frame that does not look asks for one when the second is up.
    pub fn autosave_settings(&mut self, ctx: Option<&egui::Context>) {
        if self.options_snapshot.is_some() {
            return;
        }
        let now = web_time::Instant::now();
        let input = ctx.is_none_or(|c| c.input(input_may_change_settings));
        let second = std::time::Duration::from_secs(1);
        let since = self.settings_looked.map(|t| now.duration_since(t));
        if input || since.is_none_or(|d| d >= second) {
            self.settings_looked = Some(now);
            if self.settings_changed() {
                self.settings_pending = true;
            }
        } else if let (Some(ctx), Some(d)) = (ctx, since) {
            // A change this frame made (a maximized window, a worker's
            // result) is looked at when the second is up, even if no
            // frame comes by itself: one frame after a burst, none idle.
            ctx.request_repaint_after(second.saturating_sub(d));
        }
        if !self.settings_pending {
            return;
        }
        let gap = std::time::Duration::from_secs(1);
        let wait = self
            .settings_written
            .map(|t| gap.saturating_sub(now.duration_since(t)))
            .unwrap_or_default();
        if wait.is_zero() {
            self.settings.save();
            self.settings_written = Some(now);
            self.settings_pending = false;
        } else if let Some(ctx) = ctx {
            ctx.request_repaint_after(wait);
        }
    }

    /// Copy the editor's state into the preferences; true when they differ
    /// from the ones last saved or looked at, which are then remembered.
    pub fn settings_changed(&mut self) -> bool {
        self.sync_settings();
        let hint = self.settings_saved.as_ref().map_or(0, Vec::len);
        let Some(now) = settings_json(&self.settings, hint + 64) else {
            return false;
        };
        if self.settings_saved.as_ref() == Some(&now) {
            return false;
        }
        self.settings_saved = Some(now);
        true
    }

    /// The dockers as they are now: the default column, the other column
    /// and the floating ones (see [`crate::settings::DockerSetup`]).
    pub fn docker_setup(&self) -> crate::settings::DockerSetup {
        let keys = |tabs: &[DockerTab]| tabs.iter().map(|t| t.key().to_string()).collect();
        crate::settings::DockerSetup {
            open: keys(&self.open_dockers),
            front: self.docker_tab.key().to_string(),
            left: keys(&self.left_dockers),
            left_front: self.left_tab.key().to_string(),
            left_shown: self.show_left_dockers,
            floating: self
                .floating_dockers
                .iter()
                .map(|f| crate::settings::FloatingSetup {
                    tab: f.tab.key().to_string(),
                    pos: [f.pos.x, f.pos.y],
                    size: [f.size.x, f.size.y],
                })
                .collect(),
        }
    }

    /// Put the dockers back as `setup` says; names of no docker are
    /// skipped, and a docker named in two places keeps the first.
    pub fn set_docker_setup(&mut self, setup: &crate::settings::DockerSetup) {
        // AI Drawing is a mode of AI Generate now.
        let by_key = |k: &str| {
            let k = if k == "docker.ai_drawing" {
                "docker.ai_generate"
            } else {
                k
            };
            DockerTab::ALL.iter().copied().find(|t| t.key() == k)
        };
        let mut seen: Vec<DockerTab> = Vec::new();
        let mut take = |keys: &[String]| -> Vec<DockerTab> {
            let mut out = Vec::new();
            for t in keys.iter().filter_map(|k| by_key(k)) {
                if !seen.contains(&t) {
                    seen.push(t);
                    out.push(t);
                }
            }
            out
        };
        self.open_dockers = take(&setup.open);
        self.left_dockers = take(&setup.left);
        self.floating_dockers = setup
            .floating
            .iter()
            .filter_map(|f| {
                let tab = by_key(&f.tab)?;
                if seen.contains(&tab) {
                    return None;
                }
                seen.push(tab);
                let finite = f.pos.iter().chain(&f.size).all(|v| v.is_finite());
                let size = if finite && f.size[0] >= 120.0 && f.size[1] >= 120.0 {
                    egui::vec2(f.size[0], f.size[1])
                } else {
                    FloatingDocker::DEFAULT_SIZE
                };
                Some(FloatingDocker {
                    tab,
                    pos: if finite {
                        Pos2::new(f.pos[0].max(0.0), f.pos[1].max(0.0))
                    } else {
                        Pos2::new(200.0, 100.0)
                    },
                    size,
                    place: true,
                })
            })
            .collect();
        if let Some(t) = by_key(&setup.front) {
            self.docker_tab = t;
        }
        if self.open_dockers.is_empty() {
            self.show_dockers = false;
        } else if !self.open_dockers.contains(&self.docker_tab) {
            self.docker_tab = self.open_dockers[0];
        }
        if let Some(t) = by_key(&setup.left_front) {
            self.left_tab = t;
        }
        if self.left_dockers.is_empty() {
            self.show_left_dockers = false;
        } else {
            if !self.left_dockers.contains(&self.left_tab) {
                self.left_tab = self.left_dockers[0];
            }
            self.show_left_dockers = setup.left_shown;
        }
        self.docker_last = (self.docker_tab, self.show_dockers);
    }

    /// Where `tab` lies, if it is open anywhere.
    pub fn docker_place(&self, tab: DockerTab) -> Option<DockerPlace> {
        if self.open_dockers.contains(&tab) {
            Some(DockerPlace::Right)
        } else if self.left_dockers.contains(&tab) {
            Some(DockerPlace::Left)
        } else if self.floating_dockers.iter().any(|f| f.tab == tab) {
            Some(DockerPlace::Floating)
        } else {
            None
        }
    }

    /// `tab` is open somewhere (in a column, in front or behind, or
    /// floating).
    pub fn docker_open(&self, tab: DockerTab) -> bool {
        self.docker_place(tab).is_some()
    }

    /// `tab` is on screen: in front of an open column, or floating.
    pub fn docker_shown(&self, tab: DockerTab) -> bool {
        match self.docker_place(tab) {
            Some(DockerPlace::Right) => self.show_dockers && self.docker_tab == tab,
            Some(DockerPlace::Left) => self.show_left_dockers && self.left_tab == tab,
            Some(DockerPlace::Floating) => true,
            None => false,
        }
    }

    /// The tabs of a column, the one in front and whether it is open.
    pub fn docker_column(&self, place: DockerPlace) -> (&[DockerTab], DockerTab, bool) {
        match place {
            DockerPlace::Left => (&self.left_dockers, self.left_tab, self.show_left_dockers),
            _ => (&self.open_dockers, self.docker_tab, self.show_dockers),
        }
    }

    /// Open or close a column, and put `front` in front of it when given.
    pub fn set_docker_column(&mut self, place: DockerPlace, shown: bool, front: Option<DockerTab>) {
        match place {
            DockerPlace::Left => {
                self.show_left_dockers = shown;
                if let Some(t) = front {
                    self.left_tab = t;
                }
            }
            _ => {
                self.show_dockers = shown;
                if let Some(t) = front {
                    self.docker_tab = t;
                }
                self.docker_last = (self.docker_tab, self.show_dockers);
            }
        }
    }

    /// Bring `tab` to the front where it lies; a docker open nowhere
    /// joins the default column.
    pub fn open_docker(&mut self, tab: DockerTab) {
        match self.docker_place(tab) {
            Some(DockerPlace::Left) => {
                self.left_tab = tab;
                self.show_left_dockers = true;
            }
            Some(DockerPlace::Floating) => {}
            Some(DockerPlace::Right) | None => {
                if !self.open_dockers.contains(&tab) {
                    self.open_dockers.push(tab);
                }
                self.docker_tab = tab;
                self.show_dockers = true;
                self.docker_last = (tab, true);
            }
        }
    }

    /// Put `tab` at `place`: at the front of that column (opening it), or
    /// floating with its top left corner at `at` (where it last floated,
    /// or a stock place, when `None`). It leaves where it was.
    pub fn move_docker(&mut self, tab: DockerTab, place: DockerPlace, at: Option<Pos2>) {
        let was = self
            .floating_dockers
            .iter()
            .find(|f| f.tab == tab)
            .map(|f| (f.pos, f.size));
        self.take_docker(tab);
        match place {
            DockerPlace::Right => {
                self.open_dockers.push(tab);
                self.docker_tab = tab;
                self.show_dockers = true;
                self.docker_last = (tab, true);
            }
            DockerPlace::Left => {
                self.left_dockers.push(tab);
                self.left_tab = tab;
                self.show_left_dockers = true;
            }
            DockerPlace::Floating => {
                let (pos, size) =
                    was.unwrap_or((Pos2::new(200.0, 100.0), FloatingDocker::DEFAULT_SIZE));
                let pos = at.filter(|p| p.is_finite()).unwrap_or(pos);
                self.floating_dockers.push(FloatingDocker {
                    tab,
                    pos: Pos2::new(pos.x.max(0.0), pos.y.max(0.0)),
                    size,
                    place: true,
                });
            }
        }
    }

    /// Take `tab` out of wherever it is, the column it leaves showing the
    /// tab next to it (or closing when none is left).
    fn take_docker(&mut self, tab: DockerTab) {
        let at = self.open_dockers.iter().position(|t| *t == tab);
        self.open_dockers.retain(|t| *t != tab);
        if let Some(i) = at {
            if self.docker_tab == tab {
                match self
                    .open_dockers
                    .get(i.min(self.open_dockers.len().saturating_sub(1)))
                {
                    Some(t) => self.docker_tab = *t,
                    None => self.show_dockers = false,
                }
            }
            if self.open_dockers.is_empty() {
                self.show_dockers = false;
            }
            self.docker_last = (self.docker_tab, self.show_dockers);
        }
        let at = self.left_dockers.iter().position(|t| *t == tab);
        self.left_dockers.retain(|t| *t != tab);
        if let Some(i) = at {
            if self.left_tab == tab {
                match self
                    .left_dockers
                    .get(i.min(self.left_dockers.len().saturating_sub(1)))
                {
                    Some(t) => self.left_tab = *t,
                    None => self.show_left_dockers = false,
                }
            }
            if self.left_dockers.is_empty() {
                self.show_left_dockers = false;
            }
        }
        self.floating_dockers.retain(|f| f.tab != tab);
    }

    /// Close a docker wherever it is: its tab leaves its column and the
    /// next one comes to the front (with none left the column closes), or
    /// its window closes.
    pub fn close_docker(&mut self, tab: DockerTab) {
        self.take_docker(tab);
    }

    /// Once a frame, before the dockers draw: a docker brought to the
    /// front the old way (`docker_tab` set and `show_dockers` on) that
    /// lies in the other column or floats is put in front there, and the
    /// default column goes back to how the last frame left it; one open
    /// nowhere joins the default column.
    pub fn reconcile_dockers(&mut self) {
        if self.show_dockers && !self.open_dockers.contains(&self.docker_tab) {
            let tab = self.docker_tab;
            match self.docker_place(tab) {
                Some(DockerPlace::Left) => {
                    self.left_tab = tab;
                    self.show_left_dockers = true;
                    (self.docker_tab, self.show_dockers) = self.docker_last;
                }
                Some(DockerPlace::Floating) => {
                    (self.docker_tab, self.show_dockers) = self.docker_last;
                }
                _ => self.open_dockers.push(tab),
            }
            // The column's front must be one of its tabs.
            if !self.open_dockers.contains(&self.docker_tab) {
                match self.open_dockers.first() {
                    Some(t) => self.docker_tab = *t,
                    None => self.show_dockers = false,
                }
            }
        }
        self.docker_last = (self.docker_tab, self.show_dockers);
    }

    /// The tool each toolbox group shows, from tool ids (one per group, in
    /// group order); ids that name no tool of their group are skipped.
    pub fn apply_toolbox_picks(&mut self, ids: &[String]) {
        for (gi, id) in ids.iter().enumerate() {
            let group = crate::tools::GROUPS.get(gi);
            let tool = Tool::ALL.iter().find(|t| t.id() == id);
            if let (Some(g), Some(t), Some(slot)) = (group, tool, self.toolbox_last.get_mut(gi)) {
                if g.tools.contains(t) {
                    *slot = *t;
                }
            }
        }
    }

    /// The bars shown now (the palette's visibility is a palette option).
    pub fn shown_bars(&self) -> crate::settings::ShownBars {
        crate::settings::ShownBars {
            standard_toolbar: self.show_standard_toolbar,
            property_bar: self.show_property_bar,
            toolbox: self.show_toolbox,
            status_bar: self.show_status_bar,
            dockers: self.show_dockers,
            rulers: self.show_rulers,
            text_toolbar: self.show_text_toolbar,
            zoom_toolbar: self.show_zoom_toolbar,
            transform_toolbar: self.show_transform_toolbar,
        }
    }

    /// Show and hide the bars as `shown` says.
    pub fn set_shown_bars(&mut self, shown: crate::settings::ShownBars) {
        self.show_standard_toolbar = shown.standard_toolbar;
        self.show_property_bar = shown.property_bar;
        self.show_toolbox = shown.toolbox;
        self.show_status_bar = shown.status_bar;
        self.show_dockers = shown.dockers;
        self.show_rulers = shown.rulers;
        self.show_text_toolbar = shown.text_toolbar;
        self.show_zoom_toolbar = shown.zoom_toolbar;
        self.show_transform_toolbar = shown.transform_toolbar;
    }

    /// Tools > Save Settings as Default: persist current tool defaults.
    pub fn save_defaults(&mut self) {
        // New drawings start with this drawing's grid, rulers and
        // guideline settings.
        if self.has_document() {
            self.settings.document_defaults = crate::settings::DocumentDefaults::of(self.doc());
        }
        self.save_settings();
        self.status = crate::i18n::tr("status.settings_saved");
    }

    pub fn set_workspace(&mut self, ws: Workspace) {
        self.workspace = ws;
        // The interface size follows at once (the Touch workspace draws it
        // larger).
        self.settings.workspace = ws.id().into();
        match ws {
            Workspace::Default | Workspace::Classic => {
                self.show_standard_toolbar = true;
                self.show_property_bar = true;
                self.show_toolbox = true;
                self.show_status_bar = true;
                self.show_dockers = true;
                self.show_rulers = true;
            }
            Workspace::Lite => {
                self.show_standard_toolbar = true;
                self.show_property_bar = true;
                self.show_toolbox = true;
                self.show_status_bar = true;
                self.show_dockers = false;
                self.show_rulers = false;
            }
            Workspace::Illustration => {
                self.show_standard_toolbar = true;
                self.show_property_bar = true;
                self.show_toolbox = true;
                self.show_status_bar = true;
                self.show_dockers = true;
                self.docker_tab = DockerTab::Properties;
                self.show_rulers = true;
            }
            Workspace::PageLayout => {
                self.show_standard_toolbar = true;
                self.show_property_bar = true;
                self.show_toolbox = true;
                self.show_status_bar = true;
                self.show_dockers = true;
                self.docker_tab = DockerTab::Pages;
                self.show_rulers = true;
                self.show_guides = true;
            }
            Workspace::Touch => {
                self.show_standard_toolbar = true;
                self.show_property_bar = true;
                self.show_toolbox = true;
                self.show_status_bar = false;
                self.show_dockers = false;
                self.show_rulers = false;
            }
        }
    }

    pub fn document_title(&self) -> String {
        let name = self.document_name();
        if self.engine.is_dirty() {
            format!("{name}*")
        } else {
            name
        }
    }

    /// The active drawing's name: its file's name without the extension,
    /// else the document title (an Untitled name for a new drawing).
    pub fn document_name(&self) -> String {
        // Imported files (.cdr, .svg) have no native path yet; their title
        // comes from the document.
        self.file
            .as_ref()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| {
                let t = self.doc().title.trim();
                if t.is_empty() || t == "Untitled" {
                    App::untitled_name()
                } else {
                    t.to_string()
                }
            })
    }

    /// File > Close: close the active drawing, asking to save unsaved
    /// changes first.
    pub fn close_document(&mut self) {
        if !self.has_document() {
            return;
        }
        // The drawing stays open in its other windows: nothing to ask.
        if self.engine.is_dirty() && !self.has_twin(self.active_doc) {
            self.dialog = crate::ui::dialogs::Dialog::ConfirmClose;
            return;
        }
        self.close_active_document();
    }

    /// Tools > AI Generate: the AI Generate docker in front.
    /// The AI Generate docker in its Draw with words mode.
    pub fn open_ai_drawing(&mut self) {
        self.ai_image.mode = crate::ai_image::Mode::Draw;
        self.ai_image.info = false;
        self.open_ai_generate();
    }

    pub fn open_ai_generate(&mut self) {
        self.open_docker(DockerTab::AiGenerate);
    }

    /// File > New From Template.
    pub fn open_template_dialog(&mut self) {
        self.dialog = crate::ui::dialogs::Dialog::NewFromTemplate(Box::default());
    }

    pub fn save_as_template(&mut self) {
        crate::files::Dialog::new()
            .add_filter(crate::i18n::tr("file.template"), &["tdt"])
            .set_file_name("Template1.tdt")
            .save_file(self, |app, path| app.save_template_to(path));
    }

    fn save_template_to(&mut self, path: PathBuf) {
        let written = tracedraw_text::fonts()
            .with_carried_fonts(self.engine.document())
            .to_json()
            .map_err(|e| e.to_string())
            .and_then(|json| crate::files::write(&path, json).map_err(|e| e.to_string()));
        match written {
            Ok(()) => {
                self.status = crate::i18n::trf(
                    "status.saved_template",
                    &[("p", &path.display().to_string())],
                )
            }
            Err(e) => {
                self.status = crate::i18n::trf("status.save_failed", &[("e", &e.to_string())])
            }
        }
    }

    /// The action Edit > Repeat repeats: the newest one whose undo step is
    /// still in effect.
    pub fn last_repeatable(&self) -> Option<&Repeatable> {
        self.repeatables
            .iter()
            .rev()
            .find(|(id, _)| self.engine.step_in_effect(*id))
            .map(|(_, r)| r)
    }

    /// Remember `r` as the action of the undo step just recorded.
    pub(crate) fn note_repeatable(&mut self, r: Repeatable) {
        let Some(id) = self.engine.last_step_id() else {
            return;
        };
        self.repeatables.retain(|(i, _)| *i != id);
        self.repeatables.push((id, r));
        // Only the newest few matter; the ones undone and replaced by
        // other steps never come back.
        if self.repeatables.len() > 64 {
            let engine = &self.engine;
            self.repeatables.retain(|(i, _)| engine.step_in_effect(*i));
            let extra = self.repeatables.len().saturating_sub(64);
            self.repeatables.drain(..extra);
        }
    }

    /// Copies of the selection, moved (scaled, turned) by `t`, become the
    /// selection: the end of a drag with a right click, and its Repeat.
    pub(crate) fn copy_selection_by(&mut self, t: Affine) {
        if self.selection.is_empty() {
            return;
        }
        // One step: the copies and their move (#471).
        let depth = self.engine.history_labels().0.len();
        self.duplicate_in_place();
        self.transform_selection(t);
        self.engine.squash_since(depth, "Duplicate");
        self.note_repeatable(Repeatable::Copy(t));
    }

    /// Edit > Repeat: the last action still in effect again, on the
    /// current selection (a copy with a right click during a drag makes
    /// another copy, moved as far again, #363).
    pub fn repeat_last(&mut self) {
        let cmd = match self.last_repeatable().cloned() {
            Some(Repeatable::Command(cmd)) => *cmd,
            Some(Repeatable::Copy(t)) => {
                self.copy_selection_by(t);
                return;
            }
            None => return,
        };
        let shapes = self.selection.clone();
        let cmd = match cmd {
            Command::TransformShapes { transform, .. } => {
                Command::TransformShapes { shapes, transform }
            }
            Command::SetFill { fill, .. } => Command::SetFill { shapes, fill },
            Command::SetStroke { stroke, .. } => Command::SetStroke { shapes, stroke },
            Command::PatchStroke { stroke, fields, .. } => Command::PatchStroke {
                shapes,
                stroke,
                fields,
            },
            Command::PatchFill { fill, fields, .. } => Command::PatchFill {
                shapes,
                fill,
                fields,
            },
            Command::SetOpacity { opacity, .. } => Command::SetOpacity { shapes, opacity },
            other => other,
        };
        self.run(cmd);
    }

    /// Edit > Paste in View: paste centred on the visible area.
    pub fn paste_in_view(&mut self) {
        let before: Vec<_> = self.selection.clone();
        self.paste();
        if self.selection == before {
            return;
        }
        if let Some(b) = self.selection_bounds() {
            let view_rect = self.view.visible_page_rect(self.canvas_rect);
            let d = view_rect.center() - b.center();
            let shapes = self.selection.clone();
            self.run(Command::TransformShapes {
                shapes,
                transform: Affine::translate(d),
            });
        }
    }

    pub fn select_all_of(&mut self, pred: impl Fn(&Shape) -> bool) {
        let ids: Vec<ShapeId> = self
            .doc()
            .page(self.page)
            .map(|p| {
                p.layers
                    .iter()
                    .filter(|l| l.visible && !l.locked)
                    .flat_map(|l| &l.shapes)
                    .filter(|s| !s.locked && s.visible && pred(s))
                    .map(|s| s.id)
                    .collect()
            })
            .unwrap_or_default();
        self.select(ids);
    }

    pub fn zoom_step(&mut self, zoom_in: bool) {
        let levels = [
            10.0, 25.0, 50.0, 75.0, 100.0, 150.0, 200.0, 300.0, 400.0, 800.0, 1600.0,
        ];
        let cur = self.zoom_percent();
        let next = if zoom_in {
            levels
                .iter()
                .cloned()
                .find(|l| *l > cur + 0.5)
                .unwrap_or(cur * 2.0)
        } else {
            levels
                .iter()
                .rev()
                .cloned()
                .find(|l| *l < cur - 0.5)
                .unwrap_or(cur / 2.0)
        };
        self.set_zoom_percent(next);
    }

    // ----- document helpers --------------------------------------------------

    pub fn doc(&self) -> &tracedraw_core::Document {
        self.engine.document()
    }

    /// Keep the drawing where it was in its window when the canvas begins
    /// at `at` on screen: the view moves with the canvas (a window tiled,
    /// cascaded or moved, a bar docked elsewhere, the rulers shown).
    pub fn anchor_view(&mut self, at: egui::Pos2) {
        if let Some(before) = self.view_anchor {
            self.view.origin += at - before;
        }
        self.view_anchor = Some(at);
    }

    pub fn page_size(&self) -> Size {
        self.doc()
            .page(self.page)
            .map(|p| p.size)
            .unwrap_or(paper::A4)
    }

    pub fn page_rect(&self) -> Rect {
        Rect::from_origin_size((0.0, 0.0), self.page_size())
    }

    /// The layer new objects go to: the one made active in the Objects
    /// docker while it shows on the current page, visible and unlocked;
    /// else the page's topmost visible, unlocked layer (hidden layers
    /// cannot take new objects); with none visible, the topmost unlocked
    /// one.
    pub fn active_layer(&self) -> Option<LayerId> {
        if let Some(id) = self.picked_layer {
            if self
                .page_layers()
                .iter()
                .any(|l| l.id == id && l.visible && !l.locked)
            {
                return Some(id);
            }
        }
        let page = self.doc().page(self.page).ok()?;
        page.layers
            .iter()
            .rev()
            .find(|l| !l.locked && l.visible)
            .or_else(|| page.layers.iter().rev().find(|l| !l.locked))
            .map(|l| l.id)
    }

    /// The layers drawn on the current page, bottom to top: the master
    /// layers showing on it, then its own. Clicks, marquees, snapping and
    /// Select All take objects from these.
    pub fn page_layers(&self) -> Vec<&tracedraw_core::Layer> {
        self.doc().layers_for_page(self.page).unwrap_or_default()
    }

    /// The selected objects, each in page space (an object inside a group
    /// with its group's transform taken in).
    pub fn selected_shapes(&self) -> Vec<Shape> {
        if self.selection.len() > 8 {
            return (*self.selected_rc()).clone();
        }
        self.gather_selected()
    }

    /// [`Self::selected_shapes`] shared, without a copy: gathered once for
    /// a drawing, its revision and the selection, so the bars, the canvas
    /// and the status bar asking for it in one frame cost one pass (a few
    /// thousand selected objects made a frame take a fifth of a second,
    /// #367).
    pub fn selected_rc(&self) -> std::rc::Rc<Vec<Shape>> {
        let key = (
            self.engine.instance(),
            self.engine.revision(),
            self.page,
            self.symmetry_edit,
        );
        if let Ok(cache) = self.selected_cache.try_borrow() {
            if let Some(c) = cache.as_ref() {
                if c.key == key && c.selection == self.selection {
                    return c.shapes.clone();
                }
            }
        }
        let shapes = std::rc::Rc::new(self.gather_selected());
        if let Ok(mut cache) = self.selected_cache.try_borrow_mut() {
            *cache = Some(SelectedCache {
                key,
                selection: self.selection.clone(),
                shapes: shapes.clone(),
                bounds: Default::default(),
            });
        }
        shapes
    }

    fn gather_selected(&self) -> Vec<Shape> {
        let doc = self.doc();
        if self.selection.len() <= 8 {
            return self
                .selection
                .iter()
                .filter_map(|id| doc.shape_on_page(*id))
                .collect();
        }
        // Many objects (Select All): one pass over the layers finds the
        // ones lying on a layer, instead of a search of the whole drawing
        // for each, which made a frame take a fifth of a second with a
        // few thousand objects selected (#367). Objects inside groups are
        // found as before.
        let wanted: std::collections::HashSet<ShapeId> = self.selection.iter().copied().collect();
        let mut found: std::collections::HashMap<ShapeId, &Shape> =
            std::collections::HashMap::with_capacity(wanted.len());
        for s in doc.all_layers().flat_map(|l| &l.shapes) {
            if wanted.contains(&s.id) {
                found.entry(s.id).or_insert(s);
            }
        }
        self.selection
            .iter()
            .filter_map(|id| match found.get(id) {
                Some(s) => Some((*s).clone()),
                None => doc.shape_on_page(*id),
            })
            .collect()
    }

    /// The selected objects with the target of Trim, Weld, Intersect,
    /// Join Curves and the like last. Objects picked one by one keep
    /// their order (the one selected last is the target); a selection
    /// made with a marquee, while unchanged, puts the bottom-most object
    /// last.
    pub fn selected_shapes_target_last(&self) -> Vec<Shape> {
        let mut shapes = self.selected_shapes();
        if self.marquee_selection.as_ref() == Some(&self.selection) {
            let doc = self.doc();
            let layers: Vec<tracedraw_core::LayerId> =
                self.page_layers().iter().map(|l| l.id).collect();
            let key = |s: &Shape| {
                let id = doc.ancestors(s.id).first().copied().unwrap_or(s.id);
                doc.locate(id)
                    .map(|(l, i)| (layers.iter().position(|x| *x == l).unwrap_or(0), i))
                    .unwrap_or((0, 0))
            };
            // Topmost first, bottom-most last.
            shapes.sort_by_key(|s| std::cmp::Reverse(key(s)));
        }
        shapes
    }

    /// Some selected object lies inside a group (chosen with Ctrl+click or
    /// in the Objects docker).
    pub fn selection_nested(&self) -> bool {
        let doc = self.doc();
        if self.selection.len() <= 8 {
            return self.selection.iter().any(|id| doc.is_nested(*id));
        }
        // One pass over the layers rather than a search per object (#367).
        let on_layer: std::collections::HashSet<ShapeId> = doc
            .all_layers()
            .flat_map(|l| l.shapes.iter().map(|s| s.id))
            .collect();
        self.selection
            .iter()
            .any(|id| !on_layer.contains(id) && doc.find_shape(*id).is_some())
    }

    /// Turn the rotate and skew handles on or off (a moved centre of
    /// rotation stays where it was put).
    pub fn toggle_rotate_mode(&mut self) {
        self.rotate_mode = !self.rotate_mode;
    }

    /// The selection's bounds as drawn: live effects (an envelope, a
    /// contour...) included.
    pub fn selection_bounds(&self) -> Option<Rect> {
        self.selected_bounds()
            .iter()
            .copied()
            .reduce(|a, b| a.union(b))
    }

    /// The visual bounds of each of [`Self::selected_rc`], in its order:
    /// kept with the gathered selection, since the bars, the canvas, the
    /// cursor and the status bar ask for them many times a frame and
    /// thousands of selected objects made each answer slow (#345).
    pub fn selected_bounds(&self) -> std::rc::Rc<Vec<Rect>> {
        let shapes = self.selected_rc();
        let cached = |c: &SelectedCache| {
            std::rc::Rc::ptr_eq(&c.shapes, &shapes)
                .then(|| c.bounds.borrow().clone())
                .flatten()
        };
        if let Ok(cache) = self.selected_cache.try_borrow() {
            if let Some(b) = cache.as_ref().and_then(cached) {
                return b;
            }
        }
        let b = std::rc::Rc::new(
            shapes
                .iter()
                .map(|s| self.visual_bounds(s))
                .collect::<Vec<_>>(),
        );
        if let Ok(cache) = self.selected_cache.try_borrow() {
            if let Some(c) = cache
                .as_ref()
                .filter(|c| std::rc::Rc::ptr_eq(&c.shapes, &shapes))
            {
                *c.bounds.borrow_mut() = Some(b.clone());
            }
        }
        b
    }

    /// The outlines of the selected objects in page space, flattened
    /// finely enough for the current zoom: what Mask Outside Page draws
    /// over the mask (#615). Kept with the gathered selection and the
    /// zoom, so a frame only maps the points to the screen; at most
    /// [`SELECTED_OUTLINE_POINTS`] points.
    pub fn selected_outlines(&self) -> std::rc::Rc<Vec<Vec<Point>>> {
        let shapes = self.selected_rc();
        let zoom = self.view.zoom.to_bits();
        if let Ok(cache) = self.outline_cache.try_borrow() {
            if let Some((s, z, lines)) = cache.as_ref() {
                if *z == zoom && std::rc::Rc::ptr_eq(s, &shapes) {
                    return lines.clone();
                }
            }
        }
        // A quarter of a screen pixel.
        let tolerance = 0.25 / f64::from(self.view.zoom.max(1e-6));
        let mut lines: Vec<Vec<Point>> = Vec::new();
        let mut count = 0usize;
        'shapes: for s in shapes.iter() {
            let mut line: Vec<Point> = Vec::new();
            let mut start = None;
            let mut done = Vec::new();
            tracedraw_core::geometry::flatten(&s.page_path(), tolerance, &mut |el| {
                use tracedraw_core::geometry::PathEl;
                match el {
                    PathEl::MoveTo(p) => {
                        if line.len() > 1 {
                            done.push(std::mem::take(&mut line));
                        }
                        line.clear();
                        line.push(p);
                        start = Some(p);
                    }
                    PathEl::LineTo(p) => line.push(p),
                    PathEl::ClosePath => {
                        if let Some(p) = start {
                            line.push(p);
                        }
                    }
                    _ => {}
                }
            });
            if line.len() > 1 {
                done.push(line);
            }
            for l in done {
                count += l.len();
                lines.push(l);
                if count >= SELECTED_OUTLINE_POINTS {
                    break 'shapes;
                }
            }
        }
        let lines = std::rc::Rc::new(lines);
        if let Ok(mut cache) = self.outline_cache.try_borrow_mut() {
            *cache = Some((shapes, zoom, lines.clone()));
        }
        lines
    }

    /// The bounds of what `s` (in page space) draws, its live effects
    /// included; measured once per change of the drawing.
    pub fn visual_bounds(&self, s: &Shape) -> Rect {
        if !s.has_live_effects() {
            return s.bounds();
        }
        // The symmetry being edited: the object's own box, as picked.
        if let Some(own) = self.without_edited_symmetry(s) {
            return own.visual_bounds();
        }
        let revision = self.engine.revision();
        let Ok(mut cache) = self.visual_cache.try_borrow_mut() else {
            return s.visual_bounds();
        };
        if cache.0 != revision {
            cache.0 = revision;
            cache.1.clear();
        }
        if let Some(b) = cache.1.get(&s.id) {
            return *b;
        }
        let b = s.visual_bounds();
        cache.1.insert(s.id, b);
        b
    }

    pub fn run(&mut self, cmd: Command) {
        self.break_clone_links_for(&cmd);
        let keep = self.edited_lines_kept(std::slice::from_ref(&cmd));
        if let Err(e) = self.engine.run(&cmd) {
            self.status = format!("{}: {e}", cmd.label());
            return;
        }
        if let Some(k) = keep {
            if let Err(e) = self.engine.amend(&k) {
                self.status = e.to_string();
            }
        }
        self.sync_effect_clones_for(&cmd);
        self.sync_object_clones();
        if matches!(
            cmd,
            Command::TransformShapes { .. }
                | Command::SetFill { .. }
                | Command::SetStroke { .. }
                | Command::PatchStroke { .. }
                | Command::PatchFill { .. }
                | Command::SetOpacity { .. }
        ) {
            self.note_repeatable(Repeatable::Command(Box::new(cmd.clone())));
        }
        if let Some(rec) = &mut self.recording {
            rec.push(cmd);
        }
    }

    /// Open the docker (or hide the column when it is already the active
    /// tab), as the docker shortcuts do.
    pub fn toggle_docker(&mut self, tab: DockerTab) {
        if self.docker_shown(tab) {
            match self.docker_place(tab) {
                Some(DockerPlace::Left) => self.show_left_dockers = false,
                Some(DockerPlace::Floating) => self.close_docker(tab),
                _ => self.set_docker_column(DockerPlace::Right, false, None),
            }
        } else {
            self.open_docker(tab);
        }
    }

    pub fn set_tool(&mut self, tool: Tool) {
        if self.tool != tool {
            self.finish_curve();
            self.finish_text();
            self.table_edit = None;
            self.three_point_base = None;
            self.crop_area = None;
            self.crop_drag = None;
            self.dimension_points.clear();
            self.previous_tool = self.tool;
            self.tool = tool;
            self.rotate_mode = false;
            self.flyout_open = None;
            if let Some(gi) = crate::tools::GROUPS
                .iter()
                .position(|g| g.tools.contains(&tool))
            {
                if let Some(slot) = self.toolbox_last.get_mut(gi) {
                    *slot = tool;
                }
            }
            if !tool.implemented() {
                self.status =
                    crate::i18n::trf("status.tool_not_implemented", &[("t", &tool.name())]);
            }
        }
    }

    pub fn select(&mut self, ids: Vec<ShapeId>) {
        self.selection = ids;
        self.marquee_selection = None;
        self.rotate_mode = false;
        self.node_selection.clear();
        self.segment_spot = None;
        self.rect_corners_selected.clear();
    }

    // ----- object creation ---------------------------------------------------

    pub fn new_shape(&mut self, kind: ShapeKind) -> Option<ShapeId> {
        let layer = self.active_layer()?;
        let id = self.engine.new_shape_id();
        let mut shape = Shape::new(id, kind);
        // The fill and outline of its kind (Default Properties).
        self.doc().defaults.apply_to(&mut shape);
        self.run(Command::AddShape { layer, shape });
        // Drawn while a symmetry is edited: part of it.
        if self.symmetry_edit.is_some() {
            self.join_symmetry(&[id]);
        }
        Some(id)
    }

    /// The rectangle of a box drag: corner to corner, or centred on the
    /// start point when drawn with Shift.
    pub fn box_rect(start: Point, current: Point, from_center: bool) -> Rect {
        if from_center {
            let d = current - start;
            Rect::from_points(start - d, start + d)
        } else {
            Rect::from_points(start, current)
        }
    }

    pub fn create_box_shape(&mut self, a: Point, b: Point) {
        let rect = Rect::from_points(a, b);
        if rect.width() < 0.05 || rect.height() < 0.05 {
            return;
        }
        let kind = match self.tool {
            Tool::Rectangle | Tool::ThreePointRectangle => {
                ShapeKind::rect_with_corners(rect, self.rect_corners)
            }
            Tool::Ellipse | Tool::ThreePointEllipse => ShapeKind::Ellipse {
                rect,
                arc: self.ellipse_arc,
            },
            // The dragged box is the polygon's own bounding box.
            Tool::Polygon => ShapeKind::Polygon {
                rect: tracedraw_core::geometry::polygon_rect_for_bounds(
                    rect,
                    self.polygon_points,
                    0.0,
                ),
                points: self.polygon_points,
                sharpness: 0.0,
                complex: None,
                twist: 0.0,
            },
            // A complex star's vertices are those of a polygon.
            Tool::Star if self.star_complex => ShapeKind::Polygon {
                rect: tracedraw_core::geometry::polygon_rect_for_bounds(
                    rect,
                    self.complex_points,
                    0.0,
                ),
                points: self.complex_points,
                sharpness: 0.0,
                complex: Some(self.complex_sharpness),
                twist: 0.0,
            },
            Tool::Star => ShapeKind::Polygon {
                rect: tracedraw_core::geometry::polygon_rect_for_bounds(
                    rect,
                    self.polygon_points,
                    self.star_sharpness,
                ),
                points: self.polygon_points,
                sharpness: self.star_sharpness,
                complex: None,
                twist: 0.0,
            },
            Tool::GraphPaper => {
                self.create_graph_paper(rect);
                return;
            }
            Tool::ActionLines => {
                self.create_action_lines(rect);
                return;
            }
            _ => return,
        };
        if let Some(id) = self.new_shape(kind) {
            self.select(vec![id]);
        }
    }

    /// Graph Paper: a grid of `graph_rows` x `graph_cols` rectangles, grouped.
    pub fn create_graph_paper(&mut self, rect: Rect) {
        let (rows, cols) = (self.graph_rows.max(1), self.graph_cols.max(1));
        let Some(layer) = self.active_layer() else {
            return;
        };
        let cw = rect.width() / cols as f64;
        let ch = rect.height() / rows as f64;
        let mut cmds = Vec::new();
        let mut ids = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let cell = Rect::new(
                    rect.x0 + cw * c as f64,
                    rect.y0 + ch * r as f64,
                    rect.x0 + cw * (c + 1) as f64,
                    rect.y0 + ch * (r + 1) as f64,
                );
                let id = self.engine.new_shape_id();
                let mut s = Shape::new(
                    id,
                    ShapeKind::Rect {
                        rect: cell,
                        radius: 0.0,
                        corners: None,
                    },
                );
                s.fill = self.default_fill();
                s.stroke = self
                    .default_stroke()
                    .or_else(|| Some(Stroke::hairline(Color::BLACK)));
                ids.push(id);
                cmds.push(Command::AddShape { layer, shape: s });
            }
        }
        cmds.push(Command::Group { shapes: ids });
        let _ = self.engine.run_batch("Graph Paper", &cmds);
        if let Some(id) = self
            .doc()
            .page(self.page)
            .ok()
            .and_then(|p| p.layers.iter().find(|l| l.id == layer))
            .and_then(|l| l.shapes.last())
            .map(|s| s.id)
        {
            self.select(vec![id]);
        }
    }

    /// Action lines: speed lines across `rect`, parallel (left to right, random
    /// lengths) or radial (from the centre); one combined curve object.
    pub fn create_action_lines(&mut self, rect: Rect) {
        let path = crate::tools2::action_lines_path(
            rect,
            self.action_lines_count,
            self.action_lines_radial,
        );
        if let Some(id) = self.new_shape(ShapeKind::Path {
            path,
            closed: false,
        }) {
            let stroke = self
                .default_stroke()
                .unwrap_or_else(|| Stroke::hairline(Color::BLACK));
            self.run(Command::SetStroke {
                shapes: vec![id],
                stroke: Some(stroke),
            });
            self.run(Command::SetFill {
                shapes: vec![id],
                fill: Fill::None,
            });
            self.select(vec![id]);
        }
    }

    /// 3-point tools: the base segment `a -> b` was dragged, `c` is the third
    /// click. Rectangle and ellipse take the base as one side and `c` as the
    /// height; the curve passes through `c`, a bow (an arc of a circle)
    /// with `arc` (the constrain key held at the click, #934).
    pub fn finish_three_point(&mut self, a: Point, b: Point, c: Point, arc: bool) {
        let base = b - a;
        let len = base.hypot();
        if len < 0.05 {
            return;
        }
        let angle = base.atan2();
        let dir = base / len;
        let normal = Vec2::new(-dir.y, dir.x);
        let height = (c - a).dot(normal);
        match self.tool {
            Tool::ThreePointRectangle | Tool::ThreePointEllipse => {
                if height.abs() < 0.05 {
                    return;
                }
                let local = Rect::new(0.0, height.min(0.0), len, height.max(0.0));
                let kind = if self.tool == Tool::ThreePointRectangle {
                    ShapeKind::rect_with_corners(local, self.rect_corners)
                } else {
                    ShapeKind::Ellipse {
                        rect: local,
                        arc: self.ellipse_arc,
                    }
                };
                if let Some(id) = self.new_shape(kind) {
                    let t = Affine::translate(a.to_vec2()) * Affine::rotate(angle);
                    self.run(Command::TransformShapes {
                        shapes: vec![id],
                        transform: t,
                    });
                    self.select(vec![id]);
                }
            }
            Tool::ThreePointCurve => {
                let path = three_point_curve(a, b, c, arc);
                // The curve continues the selected curve when it starts
                // or ends on one of its end nodes (closing it when it
                // joins both ends), else it is a curve of its own.
                let joined = self.join_candidate().map(|c| c.id);
                if let Some(id) = self.add_drawn_line(path) {
                    if joined != Some(id) {
                        self.run(Command::SetFill {
                            shapes: vec![id],
                            fill: Fill::None,
                        });
                    }
                    self.select(vec![id]);
                }
            }
            _ => {}
        }
    }

    /// The curve being drawn is done: it continues the selected curve
    /// when it starts or ends on one of its end nodes, otherwise it becomes
    /// a curve of its own (see `curve_join.rs`).
    pub fn finish_curve(&mut self) {
        let history = std::mem::take(&mut self.curve_steps.undo);
        self.curve_steps.clear();
        let Some(c) = self.curve.take() else { return };
        if c.nodes.len() < 2 {
            return;
        }
        let line = c.path(None);
        // A curve of its own keeps its anchors in the history (#360): undo
        // takes them back one by one, as while it was drawn.
        if !c.bspline && !self.would_join(&line) {
            let prior = Self::anchor_paths(&history, None);
            if let Some(id) = self.new_curve_in_steps(
                prior,
                c.nodes.first().copied(),
                c.nodes.last().copied(),
                ShapeKind::Path {
                    path: line,
                    closed: false,
                },
            ) {
                self.select(vec![id]);
            }
            return;
        }
        let id = self.add_drawn_line(line.clone());
        if let Some(id) = id {
            self.note_pen_curve_step(id, c.nodes.first().copied(), c.nodes.last().copied());
        }
        if c.bspline {
            // A B-spline of its own keeps its control points (one that
            // went on from another curve is a curve like that one).
            if let Some(id) = id {
                self.keep_spline(id, &line, c.nodes.iter().map(|n| n.0).collect());
            }
        }
    }

    /// The open outlines a curve had while it was drawn (`history`, the
    /// Pen and Bezier tools' steps), from two nodes on, then `last` (the
    /// curve as it was just before it closed), with repeats left out;
    /// each with its last node and that node's handle, where drawing went
    /// on from.
    pub(crate) fn anchor_paths(
        history: &[Option<CurveInProgress>],
        last: Option<&CurveInProgress>,
    ) -> Vec<(tracedraw_core::BezPath, PenEnd)> {
        let mut out: Vec<(tracedraw_core::BezPath, PenEnd)> = Vec::new();
        for c in history.iter().flatten().chain(last) {
            if c.nodes.len() < 2 || c.bspline {
                continue;
            }
            let p = c.path(None);
            let Some(end) = c.nodes.last().copied() else {
                continue;
            };
            match out.last_mut() {
                Some(prev) if prev.0 == p => prev.1 = end,
                _ => out.push((p, end)),
            }
        }
        out
    }

    /// A new curve that ends as `last`, made in steps: it appears as the
    /// first of `prior`, then takes each next one and at last `last`, each
    /// an undo step of its own, so Ctrl+Z goes back through them. Its fill
    /// and outline are those `last` takes from the default properties.
    /// One step when there is nothing before, or while a symmetry is
    /// edited. `first` is the curve's first node, where drawing goes on
    /// from once the first step is undone, and `last_end` where it goes
    /// on from after the last step (none for a closed curve).
    pub(crate) fn new_curve_in_steps(
        &mut self,
        prior: Vec<(tracedraw_core::BezPath, PenEnd)>,
        first: Option<PenEnd>,
        last_end: Option<PenEnd>,
        last: ShapeKind,
    ) -> Option<ShapeId> {
        let prior: Vec<_> = prior
            .into_iter()
            .filter(|(p, _)| !matches!(&last, ShapeKind::Path { path, .. } if path == p))
            .collect();
        if prior.is_empty() || self.symmetry_edit.is_some() {
            let id = self.new_shape(last);
            if let Some(id) = id {
                self.note_pen_curve_step(id, first, last_end);
            }
            return id;
        }
        // Where drawing goes on from before and after each step.
        let ends: Vec<Option<PenEnd>> = std::iter::once(first)
            .chain(prior.iter().map(|(_, e)| Some(*e)))
            .chain(std::iter::once(last_end))
            .collect();
        let (prior, _): (Vec<_>, Vec<_>) = prior.into_iter().unzip();
        let layer = self.active_layer()?;
        let id = self.engine.new_shape_id();
        let mut shape = Shape::new(id, last.clone());
        self.doc().defaults.apply_to(&mut shape);
        let mut kinds = prior.into_iter().map(|path| ShapeKind::Path {
            path,
            closed: false,
        });
        let first = kinds.next()?;
        self.run(Command::AddShape {
            layer,
            shape: Shape {
                kind: first,
                ..shape
            },
        });
        self.note_pen_curve_step(id, ends[0], ends[1]);
        for (k, kind) in kinds.chain(std::iter::once(last)).enumerate() {
            if self
                .engine
                .run_with_label(&Command::SetShapeKind { shape: id, kind }, "Curve Node")
                .is_err()
            {
                break;
            }
            let (undo_to, redo_to) = (ends.get(k + 1), ends.get(k + 2));
            self.note_pen_curve_step(id, undo_to.copied().flatten(), redo_to.copied().flatten());
        }
        Some(id)
    }

    /// The step just run made or changed curve `id` drawn with the Pen or
    /// Bezier tool: undoing or redoing it takes the curve up again (#631).
    fn note_pen_curve_step(
        &mut self,
        id: ShapeId,
        undo_to: Option<PenEnd>,
        redo_to: Option<PenEnd>,
    ) {
        if !matches!(self.tool, Tool::Pen | Tool::Bezier) {
            return;
        }
        let Some(step) = self.engine.last_step_id() else {
            return;
        };
        let instance = self.engine.instance();
        // Steps that fell out of the history are dropped now and then.
        if self.pen_curve_steps.len() >= 512 {
            let max = self.engine.max_history() as u64;
            self.pen_curve_steps
                .retain(|k, _| k.0 == instance && k.1 + 2 * max >= step);
        }
        self.pen_curve_steps.insert(
            (instance, step),
            PenStep {
                shape: id,
                tool: self.tool,
                undo_to,
                redo_to,
            },
        );
    }

    /// After an undo (`undone`) or redo of the step `step` (see
    /// [`Self::pen_curve_steps`]): a curve drawn with the Pen or Bezier
    /// tool goes on being drawn with it, from where it stood then.
    fn resume_pen_step(&mut self, step: Option<u64>, undone: bool) {
        let key = step.map(|s| (self.engine.instance(), s));
        if let Some(&ps) = key.and_then(|k| self.pen_curve_steps.get(&k)) {
            let to = if undone { ps.undo_to } else { ps.redo_to };
            self.resume_curve(ps.shape, ps.tool, to);
        }
    }

    /// The curve `id`, just made with the outline `line` from the B-spline
    /// tool, keeps its control points `points` (folded into its undo
    /// step) when its outline is still `line`.
    pub(crate) fn keep_spline(
        &mut self,
        id: ShapeId,
        line: &tracedraw_core::BezPath,
        points: Vec<Point>,
    ) {
        let fresh = matches!(
            self.doc().find_shape(id).map(|s| &s.kind),
            Some(ShapeKind::Path { path, .. }) if path == line
        );
        if fresh {
            let _ = self.engine.amend(&Command::SetSpline { shape: id, points });
        }
    }

    /// Start artistic text at a point, or paragraph text in a frame.
    /// Start editing an existing text object in place.
    pub fn begin_text_edit(&mut self, id: ShapeId) {
        let len = match self.doc().find_shape(id).map(|s| &s.kind) {
            Some(ShapeKind::Text { spans, .. }) => tracedraw_core::spans_char_count(spans),
            _ => return,
        };
        self.finish_text();
        self.text_edit = Some(TextEdit::at_end(id, len));
        self.select(vec![id]);
        self.sync_text_defaults_from(id);
    }

    pub fn start_text(&mut self, at: Point, frame: Option<Size>) {
        self.finish_text();
        let span = TextSpan {
            bold: self.text_bold,
            italic: self.text_italic,
            ..TextSpan::new("", self.text_font.clone(), self.text_size_pt)
        };
        let mut para = tracedraw_core::ParagraphStyle::default();
        para.hyphenate = self.text_hyphenation;
        if let Some(id) = self.new_shape(ShapeKind::Text {
            spans: vec![span],
            origin: at,
            frame,
            align: self.text_align,
            para,
            on_path: None,
        }) {
            self.text_edit = Some(TextEdit::at_end(id, 0));
            self.select(vec![id]);
        }
    }

    /// How much larger than its font size a text shows on the page: the
    /// vertical scale of its transformation (with its groups'). A text
    /// scaled with the Pick tool shows its size as seen, and a size typed
    /// for it is the size it shows.
    pub fn text_scale(&self, id: ShapeId) -> f64 {
        let Some(s) = self.doc().find_shape(id) else {
            return 1.0;
        };
        let parent = self.doc().parent_transform(id).unwrap_or(Affine::IDENTITY);
        let k = tracedraw_core::document::axis_scale(parent * s.transform).1;
        if k.is_finite() && k > 1e-9 {
            k
        } else {
            1.0
        }
    }

    /// Property bar defaults follow the text object being edited.
    pub fn sync_text_defaults_from(&mut self, id: ShapeId) {
        let kind = self.doc().shape(id).map(|(_, s)| s.kind.clone());
        if let Ok(ShapeKind::Text { spans, align, .. }) = &kind {
            // While editing, the style at the caret (of the character before it).
            let at = self
                .text_edit
                .as_ref()
                .filter(|te| te.shape == id)
                .map(|te| te.caret.saturating_sub(1));
            let span_at = at.and_then(|at| {
                let mut seen = 0usize;
                spans.iter().find(|sp| {
                    let n = sp.char_count();
                    let hit = at < seen + n;
                    seen += n;
                    hit
                })
            });
            if let Some(sp) = span_at.or_else(|| spans.first()) {
                self.text_font = sp.font_family.clone();
                // The size as seen on the page, to the hundredth of a point.
                let seen = sp.size_pt * self.text_scale(id);
                self.text_size_pt = (seen * 100.0).round() / 100.0;
                self.text_bold = sp.bold;
                self.text_italic = sp.italic;
                self.text_underline = sp.underline;
            }
            // The alignment of the paragraph at the caret.
            let index = self.paragraph_range(id).map_or(0, |(a, _)| {
                tracedraw_core::paragraphs::paragraph_of(spans, a)
            });
            self.text_align = match &kind {
                Ok(ShapeKind::Text { para, .. }) => {
                    tracedraw_core::paragraphs::paragraph_format(spans, *align, para, index).align
                }
                _ => *align,
            };
        }
    }

    pub fn finish_text(&mut self) {
        self.ime_preedit.clear();
        let empty = self.edit_text().trim().is_empty();
        if let Some(te) = self.text_edit.take() {
            if empty {
                let _ = self.engine.run(&Command::DeleteShapes {
                    shapes: vec![te.shape],
                });
                self.selection.retain(|s| *s != te.shape);
            } else if self.settings.hyphens_when_typing && !self.settings.custom_hyphens.is_empty()
            {
                // Custom optional hyphens break the lines of text typed
                // with a defined word.
                crate::ui::writing_tools::apply_custom_hyphens(self, &[te.shape]);
            }
            // A dimension's label read back in this step (deleted, the
            // measured one comes back, #644); then what was noted for its
            // edit goes.
            self.bake_dimensions();
            self.dimension_labels_seen.clear();
        }
    }

    // ----- editing -----------------------------------------------------------

    pub fn delete_selection(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let shapes = std::mem::take(&mut self.selection);
        self.run(Command::DeleteShapes { shapes });
    }

    pub fn select_all(&mut self) {
        let ids = self
            .page_layers()
            .into_iter()
            .filter(|l| !l.locked)
            .flat_map(|l| &l.shapes)
            .map(|s| s.id)
            .filter(|id| !self.outside_clip_edit(*id))
            .collect();
        self.select(ids);
    }

    pub fn copy(&mut self) {
        let shapes = self.selected_shapes();
        if !shapes.is_empty() {
            self.clipboard = Some(Clipboard {
                shapes,
                styles: self.doc().object_styles.clone(),
                symbols: self.doc().symbols.clone(),
                system: Default::default(),
                guides: Vec::new(),
            });
            self.status = crate::i18n::trf(
                "status.objects_copied",
                &[("n", &self.selection.len().to_string())],
            );
        }
    }

    pub fn cut(&mut self) {
        // Guidelines selected and no objects: they go (#376).
        if self.selection.is_empty() && self.copy_selected_guides() {
            self.delete_selected_guides();
            return;
        }
        self.copy_with_system();
        self.delete_selection();
    }

    pub fn paste(&mut self) {
        let Some(cb) = self.clipboard.clone() else {
            return;
        };
        if !cb.guides.is_empty() && cb.shapes.is_empty() {
            self.paste_guides(&cb.guides);
            return;
        }
        let Some(layer) = self.active_layer() else {
            return;
        };
        let mut ids = Vec::new();
        let mut cmds = Vec::new();
        let here = self.doc().object_styles.clone();
        let mut symbols = self.doc().symbols.clone();
        let had_symbols = symbols.len();
        let mut copies = Vec::new();
        for s in &cb.shapes {
            let id = self.engine.new_shape_id();
            let mut shape = reid(s.clone(), id, &mut self.engine);
            crate::styles::keep_links_to(&mut shape, &cb.styles, &here);
            crate::symbol_paste::bring(&mut shape, &cb.symbols, &mut symbols);
            ids.push(id);
            copies.push(shape);
        }
        // Symbols this drawing lacks come first, for the instances.
        for symbol in symbols.drain(had_symbols..) {
            cmds.push(Command::AddSymbol { symbol });
        }
        crate::textflow::relink_copies(&cb.shapes, &mut copies);
        // Copies of connected objects still in the drawing come with
        // their connectors (#476).
        // Only objects of this drawing count (the same id and identity):
        // another drawing's ids may name other objects here.
        let originals: Vec<ShapeId> = cb
            .shapes
            .iter()
            .zip(&copies)
            .map(|(o, c)| {
                let here = self.doc().find_shape(o.id).is_some_and(|s| s.uid == o.uid);
                if here {
                    o.id
                } else {
                    c.id
                }
            })
            .collect();
        let connectors = self.connector_copies(&originals, &copies);
        for shape in copies.into_iter().chain(connectors) {
            cmds.push(Command::AddShape { layer, shape });
        }
        if let Err(e) = self.engine.run_batch("Paste", &cmds) {
            self.status = e.to_string();
        }
        if self.settings.hyphens_when_pasting && !self.settings.custom_hyphens.is_empty() {
            crate::ui::writing_tools::apply_custom_hyphens(self, &ids);
        }
        self.select(ids.clone());
        if self.symmetry_edit.is_some() {
            self.join_symmetry(&ids);
        }
    }

    pub fn duplicate(&mut self) {
        // Guidelines selected and no objects: the guidelines (#376).
        if self.selection.is_empty() && self.duplicate_selected_guides() {
            return;
        }
        let shapes = self.selected_shapes();
        let Some(layer) = self.active_layer() else {
            return;
        };
        // The copies of the last Duplicate still selected: each new copy
        // takes the step from its original to it (the offset, or what was
        // done to the copy since).
        let step_of = |id: ShapeId| -> Option<Affine> {
            let (orig, _) = self.last_duplicate.iter().find(|(_, c)| *c == id)?;
            let o = self.doc().find_shape(*orig)?;
            let c = self.doc().find_shape(id)?;
            (o.transform.determinant().abs() > 1e-12).then(|| c.transform * o.transform.inverse())
        };
        let steps: Vec<Option<Affine>> = shapes.iter().map(|s| step_of(s.id)).collect();
        let smart = !self.last_duplicate.is_empty()
            && self.selection.len() == self.last_duplicate.len()
            && steps.iter().all(Option::is_some);
        let mut ids = Vec::new();
        let mut copies = Vec::new();
        let mut pairs = Vec::new();
        for (s, step) in shapes.iter().zip(steps) {
            let id = self.engine.new_shape_id();
            let mut shape = reid(s.clone(), id, &mut self.engine);
            let step = match step {
                Some(m) if smart => m,
                _ => Affine::translate(self.duplicate_offset),
            };
            shape.transform_by(step);
            ids.push(id);
            pairs.push((s.id, id));
            copies.push(shape);
        }
        crate::textflow::relink_copies(&shapes, &mut copies);
        // A copy of a connected object comes with its connector (#476).
        let originals: Vec<ShapeId> = shapes.iter().map(|s| s.id).collect();
        let connectors = self.connector_copies(&originals, &copies);
        let cmds: Vec<Command> = copies
            .into_iter()
            .chain(connectors)
            .map(|shape| Command::AddShape { layer, shape })
            .collect();
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Duplicate", &cmds) {
            self.status = e.to_string();
        }
        self.select(ids.clone());
        self.last_duplicate = pairs;
        if self.symmetry_edit.is_some() {
            self.join_symmetry(&ids);
        }
    }

    pub fn group_selection(&mut self) {
        if self.selection.len() < 2 {
            return;
        }
        let shapes = std::mem::take(&mut self.selection);
        self.run(Command::Group { shapes });
        let group = self
            .page_layers()
            .into_iter()
            .flat_map(|l| &l.shapes)
            .filter(|s| matches!(s.kind, ShapeKind::Group { .. }))
            .max_by_key(|s| s.id)
            .map(|g| g.id);
        if let Some(g) = group {
            self.selection = vec![g];
        }
    }

    pub fn ungroup_selection(&mut self) {
        let groups: Vec<ShapeId> = self
            .selected_shapes()
            .iter()
            .filter(|s| matches!(s.kind, ShapeKind::Group { .. }))
            .map(|s| s.id)
            .collect();
        if groups.is_empty() {
            return;
        }
        // The members come out selected.
        let mut members: Vec<ShapeId> = Vec::new();
        for s in self.selected_shapes() {
            if let ShapeKind::Group { children } = &s.kind {
                members.extend(children.iter().map(|c| c.id));
            }
        }
        self.selection.clear();
        for g in groups {
            self.run(Command::Ungroup { group: g });
        }
        let doc = self.doc();
        members.retain(|id| doc.find_shape(*id).is_some());
        self.select(members);
    }

    /// Convert to Curves: shapes and text become curves; a table becomes a
    /// group of curves (cell fills, borders, text), a dimension a group of
    /// its lines, arrowheads and label as curves, and a symbol instance
    /// a group of its symbol's objects as curves. An envelope, perspective
    /// or distortion becomes part of the curves (#647). Bitmaps are left as
    /// they are, with a note in the status bar.
    pub fn convert_to_curves(&mut self) {
        self.convert_selection_to_curves(true);
    }

    /// [`App::convert_to_curves`]; `bake` false keeps the outline warps
    /// live (the Shape tool converting an object to edit its nodes).
    pub(crate) fn convert_selection_to_curves(&mut self, bake: bool) {
        let shapes = self.selected_shapes();
        self.convert_shapes_to_curves(shapes, bake);
    }

    /// [`App::convert_selection_to_curves`] for `shapes`, one undo step.
    pub(crate) fn convert_shapes_to_curves(&mut self, shapes: Vec<Shape>, bake: bool) {
        let symbols = self.doc().symbols.clone();
        let mut cmds = Vec::new();
        for s in &shapes {
            // A dimension becomes a group of plain curves (#640).
            if bake {
                if let Some(mut c) = self.dimension_curves_commands(s) {
                    cmds.append(&mut c);
                    continue;
                }
            }
            let engine = &mut self.engine;
            let mut next_id = || engine.new_shape_id();
            let c = if bake {
                s.converted_to_curves(&symbols, &mut next_id)
            } else {
                s.curves_kind(&symbols, &mut next_id)
                    .map(|kind| Shape { kind, ..s.clone() })
            };
            let Some(c) = c else {
                continue;
            };
            cmds.push(Command::SetShapeKind {
                shape: s.id,
                kind: c.kind,
            });
            if c.effects != s.effects || c.parked_effects != s.parked_effects {
                cmds.push(Command::SetEffectStack {
                    shape: s.id,
                    effects: c.effects,
                    parked: c.parked_effects,
                });
            }
        }
        if shapes
            .iter()
            .any(|s| matches!(s.kind, ShapeKind::Bitmap { .. }))
        {
            self.status = crate::i18n::tr("status.bitmap_not_curves");
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Convert To Curves", &cmds) {
                self.status = e.to_string();
            }
        }
    }

    /// Something selected that Convert to Curves would change.
    pub fn can_convert_to_curves(&self) -> bool {
        self.selected_shapes().iter().any(|s| {
            s.converts_to_curves() || s.has_outline_warp() || crate::dimensions::is_dimension(s)
        })
    }

    /// Order: 0 = to front, 1 = forward one, 2 = back one, 3 = to back.
    pub fn order(&mut self, op: u8) {
        let doc = self.doc();
        let mut cmds = Vec::new();
        for id in &self.selection {
            if let Ok((layer_id, idx)) = doc.locate(*id) {
                let len = doc.layer(layer_id).map(|l| l.shapes.len()).unwrap_or(1);
                let target = match op {
                    0 => len.saturating_sub(1),
                    1 => (idx + 1).min(len.saturating_sub(1)),
                    2 => idx.saturating_sub(1),
                    _ => 0,
                };
                if target != idx {
                    cmds.push(Command::Reorder {
                        shape: *id,
                        layer: layer_id,
                        index: target,
                    });
                }
            }
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Order", &cmds) {
                self.status = e.to_string();
            }
        }
    }

    pub fn apply_fill(&mut self, fill: Fill) {
        self.apply_fill_as(fill, true);
    }

    /// A fill picked by dragging in a colour picker (Properties > Fill):
    /// while the button is held every change replaces the step the drag
    /// made before (one undo step for the whole drag), and the colour joins
    /// the document palette only when the button is let go
    /// (`end_live_fill`).
    pub fn apply_fill_live(&mut self, fill: Fill) {
        let mut color = match &fill {
            Fill::Solid(c) => Some(*c),
            _ => None,
        };
        if let Some((rev, c)) = self.live_fill {
            if rev == self.engine.revision() {
                let _ = self.engine.undo();
                // A non-uniform fill keeps the colour from before.
                color = color.or(c);
            }
        }
        let before = self.engine.revision();
        self.apply_fill_as(fill, false);
        let after = self.engine.revision();
        self.live_fill = (after != before).then_some((after, color));
    }

    /// The button is up: a fill dragged in a colour picker is done, and
    /// its colour joins the document palette as part of its step.
    pub fn end_live_fill(&mut self) {
        let Some((rev, color)) = self.live_fill.take() else {
            return;
        };
        if rev != self.engine.revision() {
            return;
        }
        if let Some(c) = color {
            self.add_ink_for(c);
            self.remember_color(c);
        }
    }

    /// Apply a fill; `remember`: its colour joins the document palette
    /// (and its ink the drawing) now.
    fn apply_fill_as(&mut self, fill: Fill, remember: bool) {
        if let Fill::Solid(c) = &fill {
            self.current_colors.0 = *c;
        }
        // Characters selected in the text being edited take the colour.
        if self.text_selection_active() {
            let used = match &fill {
                Fill::Solid(c) => Some(*c),
                _ => None,
            };
            if self.text_apply_fill(fill) && remember {
                if let Some(c) = used {
                    self.add_ink_for(c);
                    self.remember_color(c);
                }
            }
            return;
        }
        // With the Mesh Fill or Shape tool, a colour goes to the selected
        // mesh node.
        if let Fill::Solid(c) = &fill {
            if matches!(self.tool, Tool::MeshFill | Tool::Shape) && self.color_mesh_node(*c) {
                return;
            }
        }
        // With the Table tool, a colour fills the selected cells.
        if self.tool == Tool::Table && self.table_set_cell_fill(Some(fill.clone())) {
            return;
        }
        // Monochrome bitmaps take the colour as their background.
        if let Fill::Solid(c) = &fill {
            if self.recolor_mono_bitmaps(false, *c) {
                return;
            }
        }
        if self.selection.is_empty() {
            self.request_default_change(crate::defaults::DefaultChange::Fill(fill));
        } else {
            let shapes = self.selection.clone();
            let used = match &fill {
                Fill::Solid(c) => Some(*c),
                _ => None,
            };
            let before = self.engine.revision();
            self.run(Command::SetFill { shapes, fill });
            if let Some(c) = used.filter(|_| remember && self.engine.revision() != before) {
                self.add_ink_for(c);
                self.remember_color(c);
            }
        }
    }

    /// A spot colour from a colour book (a palette's spot colour) whose
    /// ink the drawing does not have yet: the ink joins the drawing's list,
    /// named as in the palette, as part of the step that applied it.
    pub fn add_ink_for(&mut self, c: Color) {
        let Color::Spot {
            ink,
            c: cc,
            m,
            y,
            k,
            ..
        } = c
        else {
            return;
        };
        if self.doc().ink(ink).is_some() {
            return;
        }
        let name = self
            .palettes
            .iter()
            .flat_map(|p| &p.colors)
            .find(|(_, pc)| matches!(pc, Color::Spot { ink: i, .. } if *i == ink))
            .map(|(n, _)| n.clone())
            .unwrap_or_else(|| format!("Spot {ink:08x}"));
        let mut inks = self.doc().inks.clone();
        inks.push(tracedraw_core::document::Ink {
            id: ink,
            name,
            cmyk: [cc, m, y, k],
        });
        let _ = self.engine.amend(&Command::SetInks { inks });
    }

    /// Add a colour to the document palette as a step of its own (its
    /// eyedropper, Add from selection); nothing when it is there already.
    pub fn add_to_document_palette(&mut self, c: Color) {
        if self.doc().palette.contains(&c) {
            return;
        }
        let mut colors = self.doc().palette.clone();
        colors.push(c);
        self.run(Command::SetDocumentPalette { colors });
    }

    /// The document palette collects the colours applied to objects; the addition joins the step that applied it.
    pub fn remember_color(&mut self, c: Color) {
        if !self.settings.palette.auto_update_document
            || matches!(c, Color::Registration)
            || self.doc().palette.contains(&c)
        {
            return;
        }
        let mut colors = self.doc().palette.clone();
        colors.push(c);
        if let Err(e) = self.engine.amend(&Command::SetDocumentPalette { colors }) {
            self.status = e.to_string();
        }
    }

    pub fn apply_outline_color(&mut self, color: Option<Color>) {
        if let Some(c) = color {
            self.current_colors.1 = c;
        }
        // Characters selected in the text being edited take the outline.
        if self.text_selection_active() {
            if self.text_apply_outline_color(color) {
                if let Some(c) = color {
                    self.add_ink_for(c);
                    self.remember_color(c);
                }
            }
            return;
        }
        // Monochrome bitmaps take the colour as their foreground.
        if let Some(c) = color {
            if self.recolor_mono_bitmaps(true, c) {
                return;
            }
        }
        if self.selection.is_empty() {
            self.request_default_change(crate::defaults::DefaultChange::OutlineColor(color));
        } else {
            // Only the colour changes: each object (and each object in a
            // group) keeps its own width, dash and the rest.
            let shapes = self.selection.clone();
            let cmds = vec![match color {
                Some(c) => Command::PatchStroke {
                    shapes,
                    stroke: Stroke {
                        color: c,
                        ..Stroke::default()
                    },
                    fields: vec![tracedraw_core::patch::StrokeField::Paint],
                },
                None => Command::SetStroke {
                    shapes,
                    stroke: None,
                },
            }];
            if let Err(e) = self.engine.run_batch("Outline Color", &cmds) {
                self.status = e.to_string();
            } else if let Some(c) = color {
                self.add_ink_for(c);
                self.remember_color(c);
            }
        }
    }

    pub fn apply_outline_width(&mut self, width: f64) {
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            self.request_default_change(crate::defaults::DefaultChange::OutlineWidth(width));
            return;
        }
        // Only the width changes: each object (and each object in a
        // group) keeps its own colour and the rest of its outline.
        let cmds = vec![Command::PatchStroke {
            shapes: shapes.iter().map(|s| s.id).collect(),
            stroke: Stroke {
                width,
                ..Stroke::default()
            },
            fields: vec![tracedraw_core::patch::StrokeField::Width],
        }];
        if let Err(e) = self.engine.run_batch("Outline Width", &cmds) {
            self.status = e.to_string();
        }
    }

    /// The symmetry being edited, when `cmds` move or scale it: a command
    /// that puts its effects back as they are now. While a symmetry is
    /// edited its mirror lines stay where they are as its object is moved,
    /// scaled or turned; out of edit mode they go with the object
    /// (`Shape::transform_by`).
    pub fn edited_lines_kept(&self, cmds: &[Command]) -> Option<Command> {
        let id = self.symmetry_edit?;
        cmds.iter()
            .any(|c| matches!(c, Command::TransformShapes { shapes, .. } if shapes.contains(&id)))
            .then(|| self.effects_as_now(id))
            .flatten()
    }

    /// [`Engine::run_batch`] keeping the edited symmetry's mirror lines
    /// in place (see [`App::edited_lines_kept`]), in the same undo step.
    pub fn run_batch_keeping_lines(
        &mut self,
        label: &'static str,
        cmds: &[Command],
    ) -> tracedraw_core::Result<()> {
        let keep = self.edited_lines_kept(cmds);
        self.engine.run_batch(label, cmds)?;
        if let Some(k) = keep {
            self.engine.amend(&k)?;
        }
        Ok(())
    }

    /// Put `id`'s live effects back as they are now (those switched off
    /// too), for a command that changes only the space an outline is
    /// written in, not where it is: a TransformShapes doing that would
    /// carry a symmetry's mirror lines away from the outline.
    pub fn effects_as_now(&self, id: ShapeId) -> Option<Command> {
        let s = self.doc().find_shape(id)?;
        Some(Command::SetEffectStack {
            shape: id,
            effects: s.effects.clone(),
            parked: s.parked_effects.clone(),
        })
    }

    pub fn transform_selection(&mut self, t: Affine) {
        if self.selection.is_empty() {
            return;
        }
        let shapes = self.selection.clone();
        // Paragraph text takes a scale as a new frame (see text_frames.rs).
        if !self.transform_with_frames(&shapes, t) {
            self.run(Command::TransformShapes {
                shapes: shapes.clone(),
                transform: t,
            });
        }
        self.compensate_unlocked_clip_frames(&shapes, t);
        self.apply_hinting(&shapes);
        self.reflow_chains_of(&shapes);
    }

    /// Move the selection by `d`; with no object selected, the selected
    /// guidelines that are not locked (#781).
    pub fn nudge(&mut self, d: Vec2) {
        if self.selection.is_empty() {
            self.transform_selected_guides(Affine::translate(d), 0, "Nudge Guidelines");
            return;
        }
        self.transform_selection(Affine::translate(d));
    }

    pub fn mirror(&mut self, horizontal: bool) {
        let Some(b) = self.selection_bounds() else {
            return;
        };
        let c = b.center();
        let t = if horizontal {
            Affine::translate((c.x, 0.0))
                * Affine::scale_non_uniform(-1.0, 1.0)
                * Affine::translate((-c.x, 0.0))
        } else {
            Affine::translate((0.0, c.y))
                * Affine::scale_non_uniform(1.0, -1.0)
                * Affine::translate((0.0, -c.y))
        };
        self.transform_selection(t);
    }

    pub fn undo(&mut self) {
        // While a curve is drawn, a step of it goes first.
        if self.curve_undo_step() {
            return;
        }
        self.curve_steps.clear();
        self.finish_text();
        let step = self.engine.last_step_id();
        let undone = match self.engine.undo() {
            Ok(l) => {
                self.status = crate::i18n::trf("status.undo_n", &[("l", l)]);
                true
            }
            Err(e) => {
                self.status = e.to_string();
                false
            }
        };
        self.selection
            .retain(|id| self.engine.document().find_shape(*id).is_some());
        if undone {
            self.clip_edit_across(step, true);
            // A curve taken up by an earlier undo goes; undoing a Pen or
            // Bezier step takes its curve up again (#631). A curve being
            // drawn otherwise stays.
            if self.curve.as_ref().is_some_and(|c| c.resumed) {
                self.curve = None;
            }
            if self.curve.is_none() {
                self.resume_pen_step(step, true);
            }
        }
    }

    pub fn redo(&mut self) {
        if self.curve_redo_step() {
            return;
        }
        // Nodes drawn on from a curve an undo took up replace what was
        // undone: there is nothing to redo while they are drawn (#631).
        if self.curve.as_ref().is_some_and(|c| c.resumed) && !self.curve_steps.undo.is_empty() {
            self.status = tracedraw_core::Error::NothingToRedo.to_string();
            return;
        }
        self.curve_steps.clear();
        let redone = match self.engine.redo() {
            Ok(l) => {
                self.status = crate::i18n::trf("status.redo_n", &[("l", l)]);
                true
            }
            Err(e) => {
                self.status = e.to_string();
                false
            }
        };
        self.selection
            .retain(|id| self.engine.document().find_shape(*id).is_some());
        if redone {
            self.clip_edit_across(self.engine.last_step_id(), false);
            if self.curve.as_ref().is_some_and(|c| c.resumed) {
                self.curve = None;
            }
            if self.curve.is_none() {
                self.resume_pen_step(self.engine.last_step_id(), false);
            }
        }
    }

    // ----- files -------------------------------------------------------------

    /// A new drawing in its own tab, with the settings last used in the
    /// Create a New Document dialog.
    pub fn new_document(&mut self) {
        let name = self.next_untitled_name();
        let mut doc = self.settings.new_document.build(name);
        self.settings.document_defaults.apply(&mut doc);
        self.open_document(doc, None);
        self.status.clear();
    }

    /// File > New, Ctrl+N, the New button and the "+" tab: the Create a
    /// New Document dialog, unless it was turned off, then a new drawing
    /// with the last used settings.
    pub fn request_new_document(&mut self) {
        if self.settings.show_new_document_dialog {
            self.dialog = crate::ui::dialogs::Dialog::NewDocument(
                crate::new_document::NewDocState::from_settings(
                    &self.settings.new_document,
                    self.peek_untitled_name(),
                ),
            );
        } else {
            self.new_document();
        }
    }

    pub fn open_dialog(&mut self) {
        crate::files::Dialog::new()
            .add_filter(crate::i18n::tr("file.all_supported"), OPEN_EXTENSIONS)
            .add_filter("HPGL plotter (*.plt, *.hpgl)", &["plt", "hpgl", "hgl"])
            .add_filter("Text (*.txt, *.rtf, *.docx)", &["txt", "rtf", "docx"])
            .add_filter("SVG (*.svg, *.svgz)", &["svg", "svgz"])
            .add_filter("PDF, AI (*.pdf, *.ai)", &["pdf", "ai"])
            .add_filter("PostScript (*.ps, *.eps, *.prn)", &["ps", "eps", "prn"])
            .add_filter("DXF (*.dxf)", &["dxf"])
            .add_filter("Windows Metafile (*.emf, *.wmf)", &["emf", "wmf"])
            .add_filter("Photoshop (*.psd, *.psb)", &["psd", "psb"])
            .add_filter(crate::i18n::tr("file.cdr_files"), &["cdr", "cdt", "cdrt"])
            .add_filter("CMX (*.cmx)", &["cmx"])
            .add_filter("TraceDraw (*.tdraw)", &["tdraw"])
            .pick_file(self, |app, p| app.open_path(p));
    }

    /// Open `path`, or show the drawing already open from it (read from
    /// it, or saved to it) instead of reading it into a second tab (#645).
    /// Every way of opening a file comes here; templates (always a new,
    /// untitled drawing, see [`App::open_as_template`]) and Revert read the
    /// file again.
    pub fn open_path(&mut self, path: PathBuf) {
        if !is_template_path(&path) {
            if let Some(i) = self.tab_with_file(&path) {
                self.switch_document(i);
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.status = crate::i18n::trf("status.already_open", &[("name", &name)]);
                self.settings.touch_recent(&path);
                self.settings.save();
                return;
            }
        }
        self.open_path_with_password(path, "");
    }

    /// Open `path` as a template: a new, untitled drawing even when the file
    /// is open already.
    pub fn open_as_template(&mut self, path: PathBuf) {
        let tabs = self.docs.len();
        self.open_path_with_password(path, "");
        if self.docs.len() > tabs {
            self.file = None;
            self.opened_from = None;
            self.show_welcome = false;
        }
    }

    /// The kind of file `path` holds, by its extension, except that an
    /// `.ai` file written as PostScript (the format's versions before the
    /// PDF-based ones) reads as EPS.
    pub(crate) fn file_kind(path: &std::path::Path) -> String {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "ai" && crate::files::read(path).is_ok_and(|b| b.starts_with(b"%!")) {
            return "eps".into();
        }
        // Print files: PostScript, often after a printer job header.
        if ext == "prn" {
            return "ps".into();
        }
        // Templates (`.cdt`, and `.cdrt` from recent versions) are
        // drawings of the same format; opened, they are untitled.
        if ext == "cdt" || ext == "cdrt" {
            return "cdr".into();
        }
        ext
    }

    /// Open `path`; `password` opens a protected PDF (empty: none).
    pub fn open_path_with_password(&mut self, path: PathBuf, password: &str) {
        let ext = Self::file_kind(&path);
        // Objects of a `.cdr` file left out because they could not be read.
        let mut unread = 0usize;
        // The version of an opened `.cdr` file: saving keeps it.
        let mut cdr_version = None;
        let mut opened_version = None;
        // The format version of an opened `.tdraw` file: saving keeps it.
        let mut tdraw_version = None;
        let result = if ext == "svg" || ext == "svgz" {
            Self::read_svg_text(&path)
                .and_then(|text| {
                    tracedraw_io::svg_import::parse(
                        &text,
                        &mut tracedraw_core::id::IdSource::default(),
                    )
                })
                .map(|imported| {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(App::untitled_name);
                    let mut doc = App::localized_document(title, imported.size);
                    let mut ids = doc.ids().clone();
                    let n = imported.shapes.len();
                    for s in imported.shapes {
                        let shape = reid_with(s, &mut ids);
                        doc.pages[0].layers[0].shapes.push(shape);
                    }
                    doc.set_ids(ids);
                    (doc, opened_objects("SVG", n))
                })
        } else if ext == "psd" || ext == "psb" {
            crate::files::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    tracedraw_io::psd::parse(&bytes, &mut tracedraw_core::id::IdSource::default())
                })
                .map(|imported| {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(App::untitled_name);
                    for w in &imported.warnings {
                        log::warn!("psd: {w}");
                    }
                    let mut doc = App::localized_document(title, imported.size);
                    let mut ids = doc.ids().clone();
                    let n = imported.shapes.len();
                    for s in imported.shapes {
                        let shape = reid_with(s, &mut ids);
                        doc.pages[0].layers[0].shapes.push(shape);
                    }
                    doc.set_ids(ids);
                    (doc, format!("PSD: {n} layer(s)"))
                })
        } else if ext == "emf" || ext == "wmf" {
            crate::files::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    tracedraw_io::emf::parse(&bytes, &mut tracedraw_core::id::IdSource::default())
                })
                .map(|imported| {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(App::untitled_name);
                    for w in &imported.warnings {
                        log::warn!("metafile: {w}");
                    }
                    let mut doc = App::localized_document(title, imported.size);
                    let mut ids = doc.ids().clone();
                    let n = imported.shapes.len();
                    for s in imported.shapes {
                        let shape = reid_with(s, &mut ids);
                        doc.pages[0].layers[0].shapes.push(shape);
                    }
                    doc.set_ids(ids);
                    (doc, opened_objects(&ext.to_ascii_uppercase(), n))
                })
        } else if ext == "eps" || ext == "ps" {
            crate::files::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    tracedraw_io::eps_import::parse(
                        &bytes,
                        &mut tracedraw_core::id::IdSource::default(),
                    )
                })
                .map(|imported| {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(App::untitled_name);
                    for w in &imported.warnings {
                        log::warn!("eps: {w}");
                    }
                    // Every page of a PostScript file (print files have
                    // several), each at its own size.
                    let pages = imported.pages();
                    let first = pages
                        .first()
                        .map(|p| p.1)
                        .unwrap_or(tracedraw_core::document::paper::A4);
                    let mut doc = App::localized_document(title, first);
                    let layer_name = doc.pages[0].layers[0].name.clone();
                    let mut ids = doc.ids().clone();
                    let count = pages.len();
                    let mut n = 0;
                    for (i, (shapes, size)) in pages.into_iter().enumerate() {
                        n += shapes.len();
                        let shapes: Vec<_> =
                            shapes.into_iter().map(|s| reid_with(s, &mut ids)).collect();
                        if i == 0 {
                            doc.pages[0].layers[0].shapes = shapes;
                            continue;
                        }
                        let mut layer =
                            tracedraw_core::document::Layer::new(ids.layer(), layer_name.clone());
                        layer.shapes = shapes;
                        doc.pages.push(tracedraw_core::document::Page {
                            id: ids.page(),
                            name: crate::i18n::trf("doc.page_n", &[("n", &(i + 1).to_string())]),
                            size,
                            layers: vec![layer],
                            guides: Vec::new(),
                            background: None,
                        });
                    }
                    doc.set_ids(ids);
                    let format = if ext == "eps" { "EPS" } else { "PostScript" };
                    let msg = if count > 1 {
                        crate::i18n::trf(
                            "status.opened_pages",
                            &[
                                ("format", format),
                                ("pages", &count.to_string()),
                                ("objects", &n.to_string()),
                            ],
                        )
                    } else {
                        opened_objects(format, n)
                    };
                    (doc, msg)
                })
        } else if matches!(ext.as_str(), "plt" | "hpgl" | "hgl") {
            crate::files::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    tracedraw_io::plt::parse(&bytes, &mut tracedraw_core::id::IdSource::default())
                })
                .map(|imported| {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(App::untitled_name);
                    for w in &imported.warnings {
                        log::warn!("plt: {w}");
                    }
                    let mut doc = App::localized_document(title, imported.size);
                    let mut ids = doc.ids().clone();
                    let n = imported.shapes.len();
                    for s in imported.shapes {
                        let shape = reid_with(s, &mut ids);
                        doc.pages[0].layers[0].shapes.push(shape);
                    }
                    doc.set_ids(ids);
                    (doc, format!("HPGL: {n} stroke(s)"))
                })
        } else if matches!(ext.as_str(), "txt" | "rtf" | "docx") {
            Self::parse_text_file(&path).map(|imported| {
                let title = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(App::untitled_name);
                for w in &imported.warnings {
                    log::warn!("text: {w}");
                }
                let n = imported.text().chars().count();
                let mut doc = App::localized_document(title, tracedraw_core::document::paper::A4);
                let mut ids = doc.ids().clone();
                let mut shape =
                    imported.to_shape(tracedraw_io::text_import::Placement::a4(), &mut ids);
                shape.id = ids.shape();
                shape.fill = Fill::Solid(Color::BLACK);
                shape.stroke = None;
                doc.pages[0].layers[0].shapes.push(shape);
                doc.set_ids(ids);
                (doc, format!("Text: {n} character(s)"))
            })
        } else if ext == "dxf" {
            crate::files::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    tracedraw_io::dxf::parse(&text, &mut tracedraw_core::id::IdSource::default())
                })
                .map(|imported| {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(App::untitled_name);
                    for w in &imported.warnings {
                        log::warn!("dxf: {w}");
                    }
                    let n = imported.shapes.len();
                    let doc = tracedraw_io::dxf::to_document(imported, &title);
                    let mut doc2 = App::localized_document(title, doc.pages[0].size);
                    let mut ids = tracedraw_core::id::IdSource::default();
                    let pid = ids.page();
                    doc2.pages[0].id = pid;
                    doc2.pages[0].layers.clear();
                    for l in &doc.pages[0].layers {
                        let mut layer =
                            tracedraw_core::document::Layer::new(ids.layer(), l.name.clone());
                        for s in &l.shapes {
                            layer.shapes.push(reid_with(s.clone(), &mut ids));
                        }
                        doc2.pages[0].layers.push(layer);
                    }
                    doc2.set_ids(ids);
                    (doc2, opened_objects("DXF", n))
                })
        } else if ext == "pdf" || ext == "ai" {
            use tracedraw_io::pdf_import::ImportError;
            if self.ask_pdf_text(&path, false) {
                return;
            }
            let options = self.pdf_options();
            let parsed = crate::files::read(&path)
                .map_err(|e| ImportError::Failed(e.to_string()))
                .and_then(|bytes| {
                    tracedraw_io::pdf_import::parse_with_options(
                        &bytes,
                        &mut tracedraw_core::id::IdSource::default(),
                        password,
                        options,
                    )
                });
            if matches!(parsed, Err(ImportError::NeedsPassword)) {
                // The answer stands for the password's retry.
                self.pdf_text_chosen = true;
                self.ask_pdf_password(path, false, !password.is_empty());
                return;
            }
            parsed.map_err(|e| e.to_string()).map(|mut imported| {
                if options.text_as_curves {
                    crate::ui::pdf_import_dialog::remaining_text_to_curves(&mut imported.pages);
                }
                let title = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(App::untitled_name);
                for w in &imported.warnings {
                    log::warn!("pdf: {w}");
                }
                let pages = imported.pages.len();
                let n: usize = imported.pages.iter().map(|p| p.shapes.len()).sum();
                let title = imported.title.clone().unwrap_or(title);
                let first = imported
                    .pages
                    .first()
                    .map(|p| p.size)
                    .unwrap_or(tracedraw_core::document::paper::A4);
                let mut doc2 = App::localized_document(title, first);
                let layer_name = doc2.pages[0].layers[0].name.clone();
                doc2.pages.clear();
                let mut ids = tracedraw_core::id::IdSource::default();
                for (i, p) in imported.pages.into_iter().enumerate() {
                    let pid = ids.page();
                    let lid = ids.layer();
                    let mut layer = tracedraw_core::document::Layer::new(lid, layer_name.clone());
                    for s in p.shapes {
                        layer.shapes.push(reid_with(s, &mut ids));
                    }
                    doc2.pages.push(tracedraw_core::document::Page {
                        id: pid,
                        name: crate::i18n::trf("doc.page_n", &[("n", &(i + 1).to_string())]),
                        size: p.size,
                        layers: vec![layer],
                        guides: Vec::new(),
                        background: None,
                    });
                }
                doc2.set_ids(ids);
                (
                    doc2,
                    crate::i18n::trf(
                        "status.opened_pages",
                        &[
                            ("format", "PDF"),
                            ("pages", &pages.to_string()),
                            ("objects", &n.to_string()),
                        ],
                    ),
                )
            })
        } else if ext == "cmx" {
            Self::open_cdr(&path)
                .map(|(doc, report)| {
                    for w in &report.warnings {
                        log::warn!("cmx: {w}");
                    }
                    unread = report.skipped_objects;
                    (
                        doc,
                        crate::i18n::trf(
                            "status.opened_pages",
                            &[
                                ("format", "CMX"),
                                ("pages", &report.pages.to_string()),
                                ("objects", &report.shapes.to_string()),
                            ],
                        ),
                    )
                })
                .map_err(|e| e.to_string())
        } else if ext == "cdr" {
            Self::open_cdr(&path)
                .map(|(doc, report)| {
                    let ver = report.version.map(|v| v.name()).unwrap_or_default();
                    for w in &report.warnings {
                        log::warn!("cdr: {w}");
                    }
                    unread = report.skipped_objects;
                    cdr_version = report.version.map(|v| writable_cdr_version(v.0));
                    opened_version = report.version.map(|v| v.0);
                    let mut msg = crate::i18n::trf(
                        "status.opened_cdr",
                        &[
                            ("version", &ver.to_string()),
                            ("pages", &report.pages.to_string()),
                            ("objects", &report.shapes.to_string()),
                            ("skipped", &report.skipped_objects.to_string()),
                        ],
                    );
                    // An older version than TraceDraw saves: say which
                    // version Save will write.
                    if let (Some(v), Some(w)) = (report.version, cdr_version) {
                        if v.0 != w {
                            msg.push(' ');
                            msg.push_str(&crate::i18n::trf(
                                "status.cdr_saves_as",
                                &[("version", &w.to_string())],
                            ));
                        }
                    }
                    (doc, msg)
                })
                .map_err(|e| e.to_string())
        } else {
            crate::files::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|json| {
                    // A newer file saves back in the newest version known
                    // here (while that is 0, the min is a no-op).
                    #[allow(clippy::unnecessary_min_or_max)]
                    let known = tracedraw_core::document::tdraw_text_version(&json)
                        .min(tracedraw_core::document::TDRAW_FORMAT_VERSION);
                    tdraw_version = Some(known);
                    tracedraw_core::Document::from_json(&json).map_err(|e| e.to_string())
                })
                .map(|d| (d, crate::i18n::tr("status.opened")))
        };
        match result {
            Ok((doc, msg)) => {
                self.open_document(doc, None);
                self.opened_from = (!crate::files::WEB && !is_template_path(&path))
                    .then(|| crate::files::canonical(&path));
                self.cdr_version = cdr_version;
                self.cdr_unsaved_version =
                    opened_version.filter(|v| *v < tracedraw_cdr::write::CHECKED_SINCE);
                self.tdraw_version = tdraw_version;
                if cdr_version.is_some() {
                    self.apply_open_policy();
                }
                // A template starts a new drawing: it saves in the version
                // Options sets, not the template's own (#613).
                if is_template_path(&path) {
                    self.cdr_version = None;
                    self.cdr_unsaved_version = None;
                }
                self.file = if matches!(
                    ext.as_str(),
                    "cdr"
                        | "cmx"
                        | "svg"
                        | "svgz"
                        | "pdf"
                        | "ai"
                        | "eps"
                        | "ps"
                        | "dxf"
                        | "psd"
                        | "psb"
                        | "emf"
                        | "wmf"
                        | "plt"
                        | "hpgl"
                        | "hgl"
                        | "txt"
                        | "rtf"
                        | "docx"
                ) {
                    None
                } else {
                    Some(path.clone())
                };
                self.status = format!("{}: {msg}", path.display());
                self.fit_pending = true;
                self.settings.touch_recent(&path);
                self.settings.last_dir = path.parent().map(|p| p.to_path_buf());
                self.settings.save();
                self.store_thumbnail(&path);
                self.show_welcome = false;
                self.check_missing_fonts();
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.show_open_notes(name, unread);
            }
            Err(e) => {
                self.status = crate::i18n::trf(
                    "status.could_not_open",
                    &[("p", &path.display().to_string()), ("e", &e.to_string())],
                )
            }
        }
    }

    pub fn save(&mut self, save_as: bool) {
        // A drawing from GitHub saves back as a commit (Save As saves a
        // file as usual).
        if !save_as && self.remote.is_some() {
            self.github.status.clear();
            self.dialog = crate::ui::dialogs::Dialog::GithubSave(self.github_form(true));
            return;
        }
        // A drawing opened from a `.cdr` version TraceDraw does not save
        // in: say so and let the user choose the version first.
        if !save_as {
            if let Some(version) = self.cdr_unsaved_version {
                self.dialog = crate::ui::dialogs::Dialog::OldCdrVersion { version };
                return;
            }
        }
        // In a browser every save is a download, so there is no path to
        // write back to.
        if !save_as && !crate::files::WEB {
            if let Some(path) = self.file.clone() {
                self.save_to(path);
                return;
            }
        }
        self.save_as_dialog();
    }

    /// Save As for a drawing opened from a `.cdr` version TraceDraw does
    /// not save in, offering the oldest version it saves.
    pub(crate) fn save_as_from_old_cdr(&mut self) {
        self.save_as_dialog();
        if let crate::ui::dialogs::Dialog::SaveAs(st) = &mut self.dialog {
            st.version = tracedraw_cdr::write::CHECKED_SINCE;
        }
    }

    /// The `.cdr` version the active drawing is saved in.
    pub fn cdr_save_version(&self) -> u16 {
        writable_cdr_version(
            self.cdr_version
                .unwrap_or(self.settings.formats.cdr_version),
        )
    }

    /// Save to the `.cdr` file `path` in `version`, which the drawing then
    /// keeps (Save writes it again).
    pub(crate) fn save_cdr_to(&mut self, path: PathBuf, version: u16) {
        let version = writable_cdr_version(version);
        self.cdr_version = Some(version);
        self.cdr_unsaved_version = None;
        self.save_to(path);
    }

    pub(crate) fn save_to(&mut self, path: PathBuf) {
        // Time counted for the active task goes into the file too.
        self.flush_tracked_time();
        let bytes = if is_cdr_path(&path) {
            let doc = self.engine.document();
            let changed;
            let doc = if !self.cdr_embed_profile && doc.cmyk_profile.is_some() {
                let mut d = doc.clone();
                d.cmyk_profile = None;
                changed = d;
                &changed
            } else if self.cdr_embed_profile && doc.cmyk_profile.is_none() {
                // A drawing without its own profile takes the one Colour
                // Management loaded, when there is one.
                match self.settings_cmyk_profile() {
                    Some(p) => {
                        let mut d = doc.clone();
                        d.cmyk_profile = Some(p);
                        changed = d;
                        &changed
                    }
                    None => doc,
                }
            } else {
                doc
            };
            let mut previews = cdr_previews(doc);
            previews.embed_fonts = self.cdr_embed_fonts;
            Ok(tracedraw_cdr::write::document_to_cdr_version_with(
                doc,
                self.cdr_save_version(),
                &previews,
            ))
        } else {
            // Fonts that came with other drawings go along (text imported
            // or pasted from them), and, with Embed fonts on, the
            // installed fonts the text uses whose licence allows it.
            self.backup_before_save(&path);
            let version = self
                .tdraw_version
                .unwrap_or(tracedraw_core::document::TDRAW_FORMAT_VERSION);
            let fonts = tracedraw_text::fonts();
            let carried = fonts.with_carried_fonts(self.engine.document());
            let doc = if self.tdraw_embed_fonts {
                fonts.with_embedded_fonts(&carried)
            } else {
                std::borrow::Cow::Borrowed(&*carried)
            };
            // Written to the file as it is made: the JSON of a large
            // drawing is several times its size.
            let result = crate::files::write_with(&path, |w| {
                doc.write_json_version(w, version)
                    .map_err(std::io::Error::other)
            })
            .map_err(|e| e.to_string());
            return self.saved_to(path, result);
        };
        self.backup_before_save(&path);
        let result = bytes.and_then(|b| crate::files::write(&path, b).map_err(|e| e.to_string()));
        self.saved_to(path, result);
    }

    /// After writing the drawing to `path`: its state, the status line and
    /// the recent files.
    fn saved_to(&mut self, path: PathBuf, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.engine.mark_saved();
                self.file = Some(path.clone());
                // The drawing is the saved file's now, not the one it was
                // read from.
                self.opened_from = None;
                self.status =
                    crate::i18n::trf("status.saved", &[("p", &path.display().to_string())]);
                self.settings.touch_recent(&path);
                self.settings.save();
                self.store_thumbnail(&path);
            }
            Err(e) => {
                self.status = crate::i18n::trf("status.save_failed", &[("e", &e.to_string())])
            }
        }
    }

    /// Options > Save: copy the file about to be replaced to
    /// `backup_of_<name>`, next to it or in the chosen folder.
    fn backup_before_save(&mut self, path: &std::path::Path) {
        let b = &self.settings.backup;
        if !b.before_save || !crate::files::exists(path) {
            return;
        }
        let Some(name) = path.file_name() else {
            return;
        };
        let dir = b
            .before_save_dir
            .clone()
            .or_else(|| path.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default();
        let target = dir.join(format!("backup_of_{}", name.to_string_lossy()));
        if let Err(e) = crate::files::copy(path, &target) {
            log::warn!("backup of {} failed: {e}", path.display());
        }
    }

    /// Options > Save > Auto-backup: every few minutes, save each open
    /// drawing with unsaved changes as `AutoBackup_of_<name>.tdraw` in the
    /// auto-backup folder. Called every frame; returns true when it saved.
    pub fn auto_backup_tick(&mut self, now: web_time::Instant) -> bool {
        let b = self.settings.backup.clone();
        if !b.auto || crate::files::WEB {
            return false;
        }
        if self.auto_backup_job.is_some() {
            return self.finish_auto_backup(false);
        }
        let due = std::time::Duration::from_secs(u64::from(b.minutes.max(1)) * 60);
        let last = *self.last_auto_backup.get_or_insert(now);
        if now.duration_since(last) < due {
            return false;
        }
        self.last_auto_backup = Some(now);
        let Some(dir) = b.auto_dir.clone().or_else(App::auto_backup_dir_default) else {
            return false;
        };
        // Copies of the drawings (pictures are shared, not copied) go to
        // a thread that writes them as they are made, so the window does
        // not stop while a large drawing is written.
        let mut jobs: Vec<(String, tracedraw_core::Document)> = Vec::new();
        let fonts = tracedraw_text::fonts();
        if self.has_document() && self.engine.is_dirty() {
            let doc = fonts.with_carried_fonts(self.doc()).into_owned();
            jobs.push((self.backup_name(None), doc));
        }
        for (i, slot) in self.docs.iter().enumerate() {
            if i != self.active_doc && slot.engine.is_dirty() {
                let doc = fonts
                    .with_carried_fonts(slot.engine.document())
                    .into_owned();
                jobs.push((self.backup_name(Some(i)), doc));
            }
        }
        if jobs.is_empty() {
            return false;
        }
        let to = dir.clone();
        let thread = std::thread::Builder::new()
            .name("auto-backup".into())
            .spawn(move || {
                jobs.into_iter()
                    .map(|(name, doc)| {
                        let written = crate::files::write_creating_with(to.join(&name), |w| {
                            doc.write_json_version(
                                w,
                                tracedraw_core::document::TDRAW_FORMAT_VERSION,
                            )
                            .map_err(std::io::Error::other)
                        });
                        (name, written.map_err(|e| e.to_string()))
                    })
                    .collect()
            });
        match thread {
            Ok(thread) => self.auto_backup_job = Some(AutoBackupJob { dir, thread }),
            Err(e) => log::warn!("auto-backup not started: {e}"),
        }
        false
    }

    /// The auto-backup being written, when it is done (or at once with
    /// `wait`): its files go into this session's records. True when one
    /// was saved.
    fn finish_auto_backup(&mut self, wait: bool) -> bool {
        if !wait
            && !self
                .auto_backup_job
                .as_ref()
                .is_some_and(|j| j.thread.is_finished())
        {
            return false;
        }
        let Some(job) = self.auto_backup_job.take() else {
            return false;
        };
        let Ok(results) = job.thread.join() else {
            log::warn!("auto-backup stopped");
            return false;
        };
        let mut saved = false;
        for (name, result) in results {
            match result {
                Ok(()) => {
                    saved = true;
                    if let Some(s) = self.session.as_mut().filter(|s| s.dir() == job.dir) {
                        s.wrote(&name, crate::recovery::now_secs());
                    }
                }
                Err(e) => log::warn!("auto-backup {name} failed: {e}"),
            }
        }
        saved
    }

    /// Session recovery, called every frame with the time in seconds: on
    /// the first call, start this session's marker and offer what a
    /// session that ended without closing left; then check in now and
    /// then. Nothing happens in a browser or with auto-backup off.
    pub fn session_tick(&mut self, now: u64) {
        if crate::files::WEB || !self.settings.backup.auto {
            return;
        }
        // Tests leave the real auto-backup folder alone unless they name one.
        if cfg!(test) && self.settings.backup.auto_dir.is_none() {
            return;
        }
        if self.session.is_none() {
            let Some(dir) = self
                .settings
                .backup
                .auto_dir
                .clone()
                .or_else(App::auto_backup_dir_default)
            else {
                return;
            };
            let id = crate::recovery::new_id();
            let found = crate::recovery::find(&dir, now, Some(&id));
            let (offer, empty): (Vec<_>, Vec<_>) =
                found.into_iter().partition(|f| !f.backups.is_empty());
            crate::recovery::dismiss(&empty, false);
            if !offer.is_empty() && matches!(self.dialog, crate::ui::dialogs::Dialog::None) {
                self.dialog = crate::ui::dialogs::Dialog::Recover(offer);
            }
            self.session = Some(crate::recovery::Session::new(dir, id));
        }
        if let Some(s) = self.session.as_mut() {
            s.heartbeat(now);
        }
    }

    /// Open the drawings a session that ended without closing left, as
    /// unsaved drawings without a file, and forget them in the folder.
    pub fn recover(&mut self, found: &[crate::recovery::Found]) {
        let mut opened = Vec::new();
        for path in found.iter().flat_map(|f| &f.backups) {
            let doc = crate::files::read_to_string(path)
                .map_err(|e| e.to_string())
                .and_then(|json| {
                    tracedraw_core::Document::from_json(&json).map_err(|e| e.to_string())
                });
            match doc {
                Ok(mut doc) => {
                    // The tab shows the drawing's name the backup carries.
                    if let Some(name) = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .and_then(|s| s.strip_prefix("AutoBackup_of_"))
                        .filter(|s| !s.is_empty())
                    {
                        doc.title = name.to_string();
                    }
                    self.open_document(doc, None);
                    self.engine.mark_unsaved();
                    opened.push(path.clone());
                }
                Err(e) => log::warn!("recovering {}: {e}", path.display()),
            }
        }
        // What opened goes from the folder; a backup that did not open
        // stays there for the user.
        for path in &opened {
            let _ = crate::files::remove(path);
        }
        crate::recovery::dismiss(found, false);
    }

    /// Customization > Command Bars > Reset: the bars of a new
    /// installation, where they first were.
    pub fn reset_command_bars(&mut self) {
        self.settings.bars = crate::ui::bars::BarLayout::default();
        self.settings.palette.show_palette = true;
        self.show_standard_toolbar = true;
        self.show_property_bar = true;
        self.show_toolbox = true;
        self.show_status_bar = true;
        self.show_text_toolbar = false;
        self.show_zoom_toolbar = false;
        self.show_transform_toolbar = false;
    }

    /// Help > Updates: check now and show the answer.
    pub fn check_for_updates(&mut self, ctx: Option<egui::Context>) {
        self.update_quiet = false;
        self.updates.start(ctx);
        self.dialog = crate::ui::dialogs::Dialog::Updates;
    }

    /// Called every frame with the time in seconds: once a day (Options >
    /// Global > General), start a quiet check; when it finds a release
    /// newer than this one that the user did not skip, show it.
    pub fn update_tick(&mut self, now: u64, ctx: Option<egui::Context>) {
        if crate::files::WEB || cfg!(test) {
            return;
        }
        if self.settings.check_updates
            && now.saturating_sub(self.settings.last_update_check) >= crate::updates::DAILY
            && self.updates.get() == crate::updates::UpdateState::Idle
        {
            self.settings.last_update_check = now;
            self.update_quiet = true;
            self.updates.start(ctx);
        }
        if self.update_quiet {
            match self.updates.get() {
                crate::updates::UpdateState::Checking => {}
                crate::updates::UpdateState::Available(r)
                    if r.version != self.settings.skipped_update
                        && matches!(self.dialog, crate::ui::dialogs::Dialog::None) =>
                {
                    self.update_quiet = false;
                    self.dialog = crate::ui::dialogs::Dialog::Updates;
                }
                crate::updates::UpdateState::Available(_) => {}
                _ => self.update_quiet = false,
            }
        }
    }

    /// A clean exit: this session's marker and auto-backups go.
    pub fn end_session(&mut self) {
        // A backup still being written finishes first, so the clean exit
        // removes it with the others.
        self.finish_auto_backup(true);
        if let Some(s) = self.session.take() {
            s.end();
        }
    }

    /// `AutoBackup_of_<drawing name>.tdraw` for the active drawing (`None`)
    /// or another open one.
    fn backup_name(&self, slot: Option<usize>) -> String {
        let title = match slot {
            None => self.drawing_title(self.active_doc),
            Some(i) => self.drawing_title(i),
        };
        let stem: String = title
            .trim_end_matches('*')
            .trim()
            .trim_end_matches(".tdraw")
            .trim_end_matches(".cdr")
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        format!("AutoBackup_of_{stem}.tdraw")
    }

    /// Ctrl+8 and Ctrl+2: the selected text (or the text being edited)
    /// one keyboard text increment larger or smaller.
    pub fn step_text_size(&mut self, up: bool) {
        let step = self.settings.text_increment_pt.max(0.1);
        let size = if up {
            self.text_size_pt + step
        } else {
            self.text_size_pt - step
        };
        self.text_size_pt = size.clamp(1.0, 3000.0);
        self.apply_text_style();
    }

    /// Put `contents` inside `frame` (a clip frame), centring them in the
    /// frame first when Options > Clip Frames asks for it: always, or when
    /// they lie completely outside the frame (the default).
    pub fn place_inside(&mut self, contents: Vec<ShapeId>, frame: ShapeId) {
        use crate::settings::AutoCenter;
        let bounds = |app: &App, ids: &[ShapeId]| -> Option<Rect> {
            ids.iter()
                .filter_map(|id| app.doc().shape(*id).ok().map(|(_, s)| s.bounds()))
                .reduce(|a, b| a.union(b))
        };
        let mut cmds = Vec::new();
        if let (Some(c), Some(f)) = (bounds(self, &contents), bounds(self, &[frame])) {
            let center = match self.settings.clip_frame.auto_center {
                AutoCenter::Always => true,
                AutoCenter::Never => false,
                AutoCenter::WhenOutside => c.intersect(f).area() <= 0.0,
            };
            if center {
                let d = f.center() - c.center();
                cmds.push(Command::TransformShapes {
                    shapes: contents.clone(),
                    transform: Affine::translate(d),
                });
            }
        }
        cmds.push(Command::PlaceInside { contents, frame });
        if let Err(e) = self.engine.run_batch("Place Inside Frame", &cmds) {
            self.status = format!("ClipFrame: {e}");
        }
    }

    /// File > Export (Ctrl+E, the toolbar button): the Export dialog with
    /// every format, starting with the choices of the last export of this
    /// session, else those of Options > Global > File Formats (#617).
    pub fn export(&mut self) {
        let mut st = self
            .last_export
            .clone()
            .unwrap_or_else(|| self.settings.formats.state());
        st.remember = true;
        // "Do not show filter dialog": straight to the file.
        if self.settings.formats.skip_export_dialog {
            crate::ui::dialogs::export_with_file_dialog(self, &st);
            self.keep_export_choices(&st);
            return;
        }
        self.dialog = crate::ui::dialogs::Dialog::Export(st);
    }

    /// A File > Export done with `st`: the next export of the session
    /// starts with its choices, and with Options > Global > File Formats >
    /// Remember the settings of the last export on, so does the next
    /// session (#617). The presets for the web and office keep nothing.
    pub fn keep_export_choices(&mut self, st: &crate::ui::dialogs::ExportState) {
        if !st.remember {
            return;
        }
        let mut kept = st.clone();
        // The file's own title and tags are not choices, and Selected only
        // was for that selection: the next export, which may go straight to
        // the file, starts with the whole page again.
        kept.title.clear();
        kept.tags.clear();
        kept.selection_only = false;
        kept.preset_name = None;
        self.last_export = Some(kept);
        if self.settings.formats.remember_last {
            self.settings.formats.remember(st);
            self.settings.save();
        }
    }

    /// After a drawing opened: say which objects could not be read and
    /// which fonts are missing, as far as Options > Global > Warnings asks
    /// for it, unless another dialog is up.
    pub fn show_open_notes(&mut self, name: String, unread: usize) {
        if !matches!(self.dialog, crate::ui::dialogs::Dialog::None) {
            return;
        }
        let w = &self.settings.warnings;
        let notes = crate::ui::dialogs::OpenNotes {
            name,
            unread: if w.unread_objects { unread } else { 0 },
            fonts: if w.missing_fonts {
                self.missing_fonts.clone()
            } else {
                Vec::new()
            },
            dont_show: false,
        };
        if notes.unread > 0 || !notes.fonts.is_empty() {
            self.dialog = crate::ui::dialogs::Dialog::OpenNotes(notes);
        }
    }

    /// File > Print (Ctrl+P, the toolbar button): the Print dialog,
    /// starting with the choices of Options > Global > Printing.
    pub fn print(&mut self) {
        let mut st = self.settings.printing.state();
        // The drawing's own bleed, when it has one, is the bleed limit.
        let bleed = self.doc().metadata.bleed;
        if bleed > 0.0 {
            st.bleed_limit = bleed;
        }
        // The system's printers, the dialog starting on the one printed
        // to last, else the default one. Reading them can take seconds, so
        // it happens off the interface's thread; the list read last shows
        // meanwhile (#614).
        st.printers = self.printers_seen.clone();
        if !st.printers.is_empty() {
            st.printer = crate::printers::starting_printer(st.printer.as_deref(), &st.printers);
        }
        st.looking_for_printers = true;
        self.fetch_printers(crate::printers::system_printers);
        self.dialog = crate::ui::dialogs::Dialog::Print(st);
    }

    /// Read the printer list with `list` off the interface's thread (at
    /// once in the browser, which has no threads and no printers).
    pub fn fetch_printers(&mut self, list: fn() -> Vec<crate::printers::Printer>) {
        let (tx, rx) = std::sync::mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let spawned = std::thread::Builder::new()
                .name("printers".into())
                .spawn(move || {
                    let _ = tx.send(list());
                });
            if let Err(e) = spawned {
                log::warn!("no printer list: {e}");
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = tx.send(list());
        self.printer_fetch = Some(rx);
    }

    /// Run `job` (a print to a printer) off the interface's thread; its
    /// outcome goes to the status bar when it is done (#810).
    pub fn start_print_job(
        &mut self,
        job: impl FnOnce() -> Result<String, String> + Send + 'static,
    ) {
        let (tx, rx) = std::sync::mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let spawned = std::thread::Builder::new()
                .name("print".into())
                .spawn(move || {
                    let _ = tx.send(job());
                });
            if let Err(e) = spawned {
                self.status = format!("{}: {e}", crate::i18n::tr("dialog.print_failed"));
                return;
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = tx.send(job());
        self.print_job = Some(rx);
    }

    /// The outcome of the print on its way, once there is one, into the
    /// status bar. Returns whether a print is still on its way.
    pub fn poll_print_job(&mut self) -> bool {
        let Some(rx) = &self.print_job else {
            return false;
        };
        let done = match rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return true,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err(crate::i18n::tr("dialog.print_failed"))
            }
        };
        self.status = match done {
            Ok(msg) => msg,
            Err(e) => format!("{}: {e}", crate::i18n::tr("dialog.print_failed")),
        };
        self.print_job = None;
        false
    }

    /// The printer list once it has been read: into the Print dialog's
    /// state, keeping the printer chosen when the system has it, else the
    /// default (a destination picked meanwhile stays). Returns whether the
    /// list is still being read.
    pub fn poll_printers(&mut self, st: &mut crate::ui::dialogs::PrintState) -> bool {
        let Some(rx) = &self.printer_fetch else {
            st.looking_for_printers = false;
            return false;
        };
        match rx.try_recv() {
            Ok(list) => {
                let list = crate::printers::by_name(list);
                // A destination the user chose meanwhile stays, unless it
                // names a printer the system does not have.
                let gone = st
                    .printer
                    .as_ref()
                    .is_some_and(|p| !list.iter().any(|q| &q.name == p));
                if !st.printer_picked || gone {
                    st.printer = crate::printers::starting_printer(st.printer.as_deref(), &list);
                }
                self.printers_seen = list.clone();
                st.printers = list;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => return true,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
        }
        self.printer_fetch = None;
        st.looking_for_printers = false;
        false
    }

    /// File > Print Preview: the sheets for the given print settings, or
    /// for those of Options > Global > Printing.
    pub fn print_preview(&mut self, st: Option<crate::ui::dialogs::PrintState>) {
        let mut st = st.unwrap_or_else(|| self.settings.printing.state());
        // The printers, for the preview's printer choice (#792): the list
        // read last at once, the system's read again off the interface's
        // thread, as the Print dialog does.
        if st.printers.is_empty() {
            st.printers = self.printers_seen.clone();
            if !st.printers.is_empty() && !st.printer_picked {
                st.printer = crate::printers::starting_printer(st.printer.as_deref(), &st.printers);
            }
        }
        if self.printer_fetch.is_none() && !crate::files::WEB {
            st.looking_for_printers = true;
            self.fetch_printers(crate::printers::system_printers);
        }
        self.print_preview = Some(crate::ui::print_preview::PrintPreview::new(st));
    }

    pub fn import(&mut self) {
        let mut all: Vec<String> = IMPORT_EXTENSIONS.iter().map(|e| e.to_string()).collect();
        let scripted: Vec<_> = self
            .settings
            .script_filters
            .iter()
            .filter(|f| f.import)
            .cloned()
            .collect();
        for f in &scripted {
            all.extend(f.extensions.iter().cloned());
        }
        let all: Vec<&str> = all.iter().map(String::as_str).collect();
        let mut dialog =
            crate::files::Dialog::new().add_filter(crate::i18n::tr("file.all_importable"), &all);
        for f in &scripted {
            let exts: Vec<&str> = f.extensions.iter().map(String::as_str).collect();
            dialog = dialog.add_filter(f.label(), &exts);
        }
        dialog
            .add_filter("HPGL plotter (*.plt, *.hpgl)", &["plt", "hpgl", "hgl"])
            .add_filter("Text (*.txt, *.rtf, *.docx)", &["txt", "rtf", "docx"])
            .add_filter(crate::i18n::tr("file.cdr_files"), &["cdr", "cdt", "cdrt"])
            .add_filter("CMX (*.cmx)", &["cmx"])
            .add_filter("SVG (*.svg, *.svgz)", &["svg", "svgz"])
            .add_filter("PDF, AI (*.pdf, *.ai)", &["pdf", "ai"])
            .add_filter("PostScript (*.ps, *.eps, *.prn)", &["ps", "eps", "prn"])
            .add_filter("DXF (*.dxf)", &["dxf"])
            .add_filter("Windows Metafile (*.emf, *.wmf)", &["emf", "wmf"])
            .add_filter("Photoshop (*.psd, *.psb)", &["psd", "psb"])
            .add_filter(
                crate::i18n::tr("file.images"),
                &["png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff"],
            )
            .pick_file(self, |app, path| app.import_path(&path));
    }

    /// Import a file into the active layer, by extension.
    /// File > Import of `path`: the objects are added to the active
    /// layer and selected, as one undo step; Options > Global > File
    /// Formats may centre them in the view.
    /// Imported text with a word that has custom optional hyphens gets
    /// hyphenation (Text > Custom Optional Hyphens, "When pasting and
    /// importing text").
    fn hyphenate_imported(&mut self, ids: &[ShapeId]) {
        if self.settings.hyphens_when_pasting && !self.settings.custom_hyphens.is_empty() {
            crate::ui::writing_tools::apply_custom_hyphens(self, ids);
        }
    }

    pub fn import_path(&mut self, path: &std::path::Path) {
        let at = (self.settings.formats.import_place == crate::prefs::ImportPlace::ViewCenter)
            .then(|| self.view_center());
        self.import_path_placed(path, at);
    }

    /// The middle of the drawing window, page space.
    pub fn view_center(&self) -> Point {
        self.view.visible_page_rect(self.canvas_rect).center()
    }

    /// Import `path` like File > Import; `at` puts the middle of what came
    /// there instead of where the file has it.
    pub fn import_path_placed(&mut self, path: &std::path::Path, at: Option<Point>) {
        let before = self.selection.clone();
        self.import_path_as_in_file(path);
        let Some(at) = at else {
            return;
        };
        if self.selection == before || self.selection.is_empty() {
            return;
        }
        let Some(b) = self.selection_bounds() else {
            return;
        };
        let d = at - b.center();
        if !(d.x.is_finite() && d.y.is_finite()) {
            return;
        }
        // Part of the import's own undo step.
        let cmd = Command::TransformShapes {
            shapes: self.selection.clone(),
            transform: Affine::translate(d),
        };
        if let Err(e) = self.engine.amend(&cmd) {
            self.status = e.to_string();
        }
    }

    fn import_path_as_in_file(&mut self, path: &std::path::Path) {
        let path = path.to_path_buf();
        // An installed script filter goes before the built-in readers.
        let raw_ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if let Some(filter) = self.import_filter_for(&raw_ext) {
            self.import_with_script(&path, &filter);
            return;
        }
        let ext = Self::file_kind(&path);
        if ext == "svg" || ext == "svgz" {
            self.import_svg(&path);
            return;
        }
        if ext == "pdf" || ext == "ai" {
            self.import_pdf(&path);
            return;
        }
        if ext == "dxf" {
            self.import_dxf(&path);
            return;
        }
        if ext == "emf" || ext == "wmf" {
            self.import_metafile(&path);
            return;
        }
        if matches!(ext.as_str(), "plt" | "hpgl" | "hgl") {
            self.import_plt(&path);
            return;
        }
        if matches!(ext.as_str(), "txt" | "rtf" | "docx") {
            self.import_text_file(&path);
            return;
        }
        if ext == "eps" || ext == "ps" {
            self.import_eps(&path);
            return;
        }
        if ext == "psd" || ext == "psb" {
            self.import_psd(&path);
            return;
        }
        if ext != "cdr" && ext != "cmx" {
            self.import_bitmap(&path);
            return;
        }
        match Self::open_cdr(&path) {
            Ok((doc, _)) => {
                let Some(layer) = self.active_layer() else {
                    return;
                };
                // The imported text keeps its letters; saving the drawing
                // carries the fonts along.
                self.add_carried_fonts(&doc.fonts);
                let mut ids = Vec::new();
                let originals: Vec<Shape> = doc
                    .pages
                    .iter()
                    .flat_map(|p| &p.layers)
                    .flat_map(|l| &l.shapes)
                    .cloned()
                    .collect();
                let mut copies = Vec::new();
                for s in &originals {
                    let id = self.engine.new_shape_id();
                    let mut shape = reid(s.clone(), id, &mut self.engine);
                    self.apply_import_policy(
                        std::slice::from_mut(&mut shape),
                        doc.cmyk_profile.as_ref(),
                    );
                    ids.push(id);
                    copies.push(shape);
                }
                // Linked text frames imported together stay a chain.
                crate::textflow::relink_copies(&originals, &mut copies);
                let cmds: Vec<Command> = copies
                    .into_iter()
                    .map(|shape| Command::AddShape { layer, shape })
                    .collect();
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                self.hyphenate_imported(&ids);
                self.select(ids);
            }
            Err(e) => {
                self.status = crate::i18n::trf("status.import_failed", &[("e", &e.to_string())])
            }
        }
    }

    /// Read a `.cdr` file through the file layer (uploads in a browser).
    fn open_cdr(
        path: &std::path::Path,
    ) -> std::result::Result<(tracedraw_core::Document, tracedraw_cdr::ParseReport), String> {
        let bytes = crate::files::read(path).map_err(|e| e.to_string())?;
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
        tracedraw_cdr::open_bytes(&bytes, &title).map_err(|e| e.to_string())
    }

    /// Read an SVG file (plain or gzip-compressed) as text.
    fn read_svg_text(path: &std::path::Path) -> std::result::Result<String, String> {
        let bytes = crate::files::read(path).map_err(|e| e.to_string())?;
        if bytes.len() > 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            use std::io::Read;
            let mut out = String::new();
            flate2::read::GzDecoder::new(&bytes[..])
                .read_to_string(&mut out)
                .map_err(|e| e.to_string())?;
            Ok(out)
        } else {
            String::from_utf8(bytes).map_err(|e| e.to_string())
        }
    }

    /// Import an SVG into the active layer, keeping its groups and clips.
    pub fn import_svg(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = Self::read_svg_text(path).and_then(|text| {
            tracedraw_io::svg_import::parse(&text, &mut tracedraw_core::id::IdSource::default())
        });
        match parsed {
            Ok(imported) => {
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                self.hyphenate_imported(&ids);
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import the first page of a PDF or AI file into the active layer.
    pub fn import_pdf(&mut self, path: &std::path::Path) {
        self.import_pdf_with_password(path, "");
    }

    /// [`App::import_pdf`] with the password of a protected file.
    pub fn import_pdf_with_password(&mut self, path: &std::path::Path, password: &str) {
        use tracedraw_io::pdf_import::ImportError;
        let Some(layer) = self.active_layer() else {
            return;
        };
        if self.ask_pdf_text(path, true) {
            return;
        }
        let options = self.pdf_options();
        let parsed = crate::files::read(path)
            .map_err(|e| ImportError::Failed(e.to_string()))
            .and_then(|bytes| {
                tracedraw_io::pdf_import::parse_with_options(
                    &bytes,
                    &mut tracedraw_core::id::IdSource::default(),
                    password,
                    options,
                )
            });
        if matches!(parsed, Err(ImportError::NeedsPassword)) {
            self.pdf_text_chosen = true;
            self.ask_pdf_password(path.to_path_buf(), true, !password.is_empty());
            return;
        }
        let parsed = parsed.map_err(|e| e.to_string());
        match parsed {
            Ok(mut imported) => {
                if options.text_as_curves {
                    crate::ui::pdf_import_dialog::remaining_text_to_curves(&mut imported.pages);
                }
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                let Some(first) = imported.pages.into_iter().next() else {
                    return;
                };
                for s in first.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                if !imported.warnings.is_empty() {
                    log::warn!("pdf import: {}", imported.warnings.join("; "));
                }
                self.hyphenate_imported(&ids);
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import a Photoshop file: one bitmap per layer, grouped.
    pub fn import_psd(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = crate::files::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                tracedraw_io::psd::parse(&bytes, &mut tracedraw_core::id::IdSource::default())
            });
        match parsed {
            Ok(imported) => {
                let mut children = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    children.push(reid(s, id, &mut self.engine));
                }
                let shape = match children.len() {
                    0 => return,
                    1 => children.remove(0),
                    _ => {
                        let id = self.engine.new_shape_id();
                        let mut g = Shape::new(id, ShapeKind::Group { children });
                        g.fill = Fill::None;
                        g.stroke = None;
                        g.name = path.file_stem().map(|s| s.to_string_lossy().to_string());
                        g
                    }
                };
                let id = shape.id;
                self.run(Command::AddShape { layer, shape });
                self.select(vec![id]);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import an HPGL plotter file into the active layer.
    pub fn import_plt(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = crate::files::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                tracedraw_io::plt::parse(&bytes, &mut tracedraw_core::id::IdSource::default())
            });
        match parsed {
            Ok(imported) => {
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                for w in &imported.warnings {
                    log::warn!("plt import: {w}");
                }
                self.hyphenate_imported(&ids);
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Parse a text document (TXT, RTF, DOCX) by extension.
    fn parse_text_file(
        path: &std::path::Path,
    ) -> Result<tracedraw_io::text_import::ImportedText, String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let bytes = crate::files::read(path).map_err(|e| e.to_string())?;
        match ext.as_str() {
            "rtf" => tracedraw_io::text_import::parse_rtf(&bytes),
            "docx" => tracedraw_io::text_import::parse_docx(&bytes),
            _ => Ok(tracedraw_io::text_import::parse_txt(&bytes)),
        }
    }

    /// Import a text document (TXT, RTF, DOCX) as a paragraph text frame
    /// filling the page inside 20 mm margins.
    pub fn import_text_file(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        match Self::parse_text_file(path) {
            Ok(imported) => {
                for w in &imported.warnings {
                    log::warn!("text import: {w}");
                }
                let page = self.page_rect();
                let margin = 20.0_f64.min(page.width() / 4.0).min(page.height() / 4.0);
                let place = tracedraw_io::text_import::Placement {
                    origin_top_left: Point::new(page.x0 + margin, page.y1 - margin),
                    size: Size::new(page.width() - 2.0 * margin, page.height() - 2.0 * margin),
                };
                let mut ids = tracedraw_core::id::IdSource::default();
                let shape = imported.to_shape(place, &mut ids);
                let id = self.engine.new_shape_id();
                let mut shape = reid(shape, id, &mut self.engine);
                shape.fill = Fill::Solid(Color::BLACK);
                shape.stroke = None;
                shape.name = path.file_stem().map(|s| s.to_string_lossy().to_string());
                self.run(Command::AddShape { layer, shape });
                self.select(vec![id]);
                self.check_missing_fonts();
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import an EPS file into the active layer.
    pub fn import_eps(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = crate::files::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                tracedraw_io::eps_import::parse(
                    &bytes,
                    &mut tracedraw_core::id::IdSource::default(),
                )
            });
        match parsed {
            Ok(imported) => {
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                if !imported.warnings.is_empty() {
                    log::warn!("eps import: {}", imported.warnings.join("; "));
                }
                self.hyphenate_imported(&ids);
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import an EMF or WMF metafile into the active layer.
    pub fn import_metafile(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = crate::files::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                tracedraw_io::emf::parse(&bytes, &mut tracedraw_core::id::IdSource::default())
            });
        match parsed {
            Ok(imported) => {
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                if !imported.warnings.is_empty() {
                    log::warn!("metafile import: {}", imported.warnings.join("; "));
                }
                self.hyphenate_imported(&ids);
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import a DXF drawing into the active layer.
    pub fn import_dxf(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = crate::files::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                tracedraw_io::dxf::parse(&text, &mut tracedraw_core::id::IdSource::default())
            });
        match parsed {
            Ok(imported) => {
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                self.hyphenate_imported(&ids);
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Reset rotation/skew/scale, keeping the object where it is.
    pub fn clear_transformations(&mut self) {
        let shapes = self.selected_shapes();
        let cmds: Vec<Command> = shapes
            .iter()
            .map(|s| {
                let c = s.bounds().center();
                let local_c = s.transform.inverse() * c;
                let t = Affine::translate(c - local_c);
                Command::TransformShapes {
                    shapes: vec![s.id],
                    transform: t * s.transform.inverse(),
                }
            })
            .collect();
        if !cmds.is_empty() {
            let _ = self.run_batch_keeping_lines("Clear Transformations", &cmds);
        }
    }

    /// File > Publish to PDF: the dialog for the active drawing.
    pub fn publish_pdf(&mut self) {
        self.dialog = crate::ui::dialogs::Dialog::PublishPdf(Box::new(
            crate::ui::publish_pdf::PublishState::new(self),
        ));
    }

    // ----- view --------------------------------------------------------------

    pub fn zoom_to_page(&mut self) {
        self.fit_pending = true;
    }

    /// Zoom so the page's width fills the window, keeping the vertical
    /// position.
    pub fn zoom_to_page_width(&mut self) {
        let page = self.page_rect();
        let cy = self.view.to_page(self.canvas_rect.center()).y;
        let r = Rect::new(page.x0, cy - 1e-3, page.x1, cy + 1e-3);
        self.view.fit(r, self.canvas_rect);
    }

    /// Zoom so the page's height fills the window, keeping the horizontal
    /// position.
    pub fn zoom_to_page_height(&mut self) {
        let page = self.page_rect();
        let cx = self.view.to_page(self.canvas_rect.center()).x;
        let r = Rect::new(cx - 1e-3, page.y0, cx + 1e-3, page.y1);
        self.view.fit(r, self.canvas_rect);
    }

    pub fn zoom_to_fit(&mut self) {
        if self.multipage {
            self.fit_pending = true;
            return;
        }
        let Some(b) = self.doc().content_bounds(self.page).ok().flatten() else {
            self.fit_pending = true;
            return;
        };
        self.view.fit(
            b.inflate(b.width() * 0.05 + 1.0, b.height() * 0.05 + 1.0),
            self.canvas_rect,
        );
    }

    pub fn zoom_to_selection(&mut self) {
        if let Some(b) = self.selection_bounds() {
            self.view.fit(
                b.inflate(b.width() * 0.1 + 1.0, b.height() * 0.1 + 1.0),
                self.canvas_rect,
            );
        }
    }

    /// Interface points per mm at 100%: real size at the calibrated screen
    /// resolution (Options > Display > Ruler calibration, 96 per inch by
    /// default), or in the Pixels view one pixel at the system's scaling
    /// per document pixel. Both go through the interface's zoom, which
    /// sizes the menus and bars, never the drawing.
    fn zoom_100(&self) -> f32 {
        let per_inch = if self.view_mode == ViewMode::Pixels {
            self.document_dpi().max(1.0) as f32
                / crate::ui::ui_zoom(crate::settings::effective_ui_size(&self.settings))
        } else {
            crate::settings::screen_points_per_inch(&self.settings)
        };
        per_inch / 25.4
    }

    pub fn zoom_percent(&self) -> f32 {
        self.view.zoom / self.zoom_100() * 100.0
    }

    pub fn set_zoom_percent(&mut self, pct: f32) {
        let anchor: Pos2 = self.canvas_rect.center();
        let target = pct / 100.0 * self.zoom_100();
        self.view.zoom_at(anchor, target / self.view.zoom);
    }

    pub fn add_page(&mut self) {
        let size = self.page_size();
        self.run(Command::AddPage { name: None, size });
        if let Some(p) = self.doc().pages.last() {
            self.page = p.id;
        }
        self.selection.clear();
        self.fit_pending = true;
    }

    /// The document navigator's insert buttons: a page like the current
    /// one, before or after it, made current.
    pub fn insert_page(&mut self, after: bool) {
        let idx = self.page_index();
        let size = self.page_size();
        self.run(Command::AddPage { name: None, size });
        let Some(new) = self.doc().pages.last().map(|p| p.id) else {
            return;
        };
        let to = if after { idx + 1 } else { idx };
        if to + 1 < self.doc().pages.len() {
            self.run(Command::MovePage { page: new, to });
        }
        self.page = new;
        self.selection.clear();
        self.fit_pending = true;
    }

    /// Duplicate the current page before or after it, with its layers
    /// only or with their contents too; the copy becomes the current page.
    pub fn duplicate_page(&mut self, before: bool, contents: bool) {
        let page = self.page;
        let old: Vec<_> = self.doc().pages.iter().map(|p| p.id).collect();
        self.run(Command::DuplicatePage {
            page,
            before,
            contents,
        });
        let new = self.doc().pages.iter().position(|p| !old.contains(&p.id));
        if let Some(i) = new {
            self.goto_page(i);
        }
    }

    pub fn delete_page(&mut self) {
        let page = self.page;
        self.run(Command::DeletePage { page });
        if let Some(p) = self.doc().pages.first() {
            self.page = p.id;
        }
        self.selection.clear();
        self.fit_pending = true;
    }

    /// Layout > Delete Page: the pages from index `a` to `b` (0 = the
    /// first, both kept in range) in one undo step, never every page; the
    /// page after them (else the one before) is the current one (#500).
    pub fn delete_pages(&mut self, a: usize, b: usize) {
        let ids: Vec<_> = self.doc().pages.iter().map(|p| p.id).collect();
        let n = ids.len();
        if n == 0 || a > b || a >= n || b + 1 - a >= n {
            return;
        }
        let b = b.min(n - 1);
        let cmds: Vec<Command> = ids[a..=b]
            .iter()
            .map(|&page| Command::DeletePage { page })
            .collect();
        if let Err(e) = self.engine.run_batch("Delete Page", &cmds) {
            self.status = e.to_string();
            return;
        }
        let next = ids
            .get(b + 1)
            .or_else(|| a.checked_sub(1).and_then(|i| ids.get(i)));
        if let Some(&p) = next {
            self.page = p;
        }
        self.selection.clear();
        self.fit_pending = true;
    }

    /// Layout > Delete Page asks which pages, the current one first.
    pub fn ask_delete_page(&mut self) {
        let i = self.page_index() + 1;
        self.dialog = crate::ui::dialogs::Dialog::DeletePage {
            from: i,
            to: i,
            through: false,
        };
    }

    pub fn goto_page(&mut self, idx: usize) {
        if self.multipage {
            if let Some(id) = self.doc().pages.get(idx).map(|p| p.id) {
                self.multipage_activate(id);
            }
            return;
        }
        if let Some(p) = self.doc().pages.get(idx) {
            self.page = p.id;
            self.selection.clear();
            self.fit_pending = true;
        }
    }

    pub fn page_index(&self) -> usize {
        self.doc()
            .pages
            .iter()
            .position(|p| p.id == self.page)
            .unwrap_or(0)
    }
}

/// Give a copied shape (and any children) fresh ids.
/// Re-number a shape tree with fresh engine ids (for other modules).
pub fn reid_pub(s: Shape, id: ShapeId, engine: &mut Engine) -> Shape {
    reid(s, id, engine)
}

fn reid(mut s: Shape, id: ShapeId, engine: &mut Engine) -> Shape {
    s.id = id;
    // A copy is not in its original's text chain (duplicate, paste and
    // import link the copies of a chain copied whole: `relink_copies`).
    crate::textflow::without_links(&mut s.data);
    match &mut s.kind {
        ShapeKind::Group { children } => {
            for c in children.iter_mut() {
                let nid = engine.new_shape_id();
                *c = reid(c.clone(), nid, engine);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            let fid = engine.new_shape_id();
            **frame = reid((**frame).clone(), fid, engine);
            for c in contents.iter_mut() {
                let nid = engine.new_shape_id();
                *c = reid(c.clone(), nid, engine);
            }
        }
        _ => {}
    }
    s
}

/// Re-number a shape tree from an id source (documents built outside the engine).
fn reid_with(mut s: Shape, ids: &mut tracedraw_core::id::IdSource) -> Shape {
    s.id = ids.shape();
    crate::textflow::without_links(&mut s.data);
    match &mut s.kind {
        ShapeKind::Group { children } => {
            for c in children.iter_mut() {
                *c = reid_with(c.clone(), ids);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            **frame = reid_with((**frame).clone(), ids);
            for c in contents.iter_mut() {
                *c = reid_with(c.clone(), ids);
            }
        }
        _ => {}
    }
    s
}

pub fn fill_preview_color(fill: &Fill) -> Option<egui::Color32> {
    match fill {
        Fill::None => None,
        Fill::Solid(c) => Some(crate::canvas::to_color32(*c)),
        other => other.preview_color().map(crate::canvas::to_color32),
    }
}

pub fn fill_description(fill: &Fill) -> String {
    use crate::i18n::{tr, trf};
    match fill {
        Fill::None => tr("fill.none"),
        Fill::Solid(c) => color_description(*c),
        Fill::Fountain(f) => match f.kind {
            tracedraw_core::FountainKind::Linear => tr("fill.linear_fountain"),
            tracedraw_core::FountainKind::Radial => tr("fill.radial_fountain"),
            tracedraw_core::FountainKind::Conical => tr("fill.conical_fountain"),
            tracedraw_core::FountainKind::Square => tr("fill.square_fountain"),
            tracedraw_core::FountainKind::Shaped => tr("fill.shaped_fountain"),
        },
        Fill::Pattern(tracedraw_core::Pattern::TwoColor { tile, .. }) => {
            trf("fill.two_color_pattern", &[("t", tile.name())])
        }
        Fill::Pattern(tracedraw_core::Pattern::Bitmap { .. }) => tr("fill.bitmap_pattern"),
        Fill::Pattern(tracedraw_core::Pattern::Vector { .. }) => tr("fill.vector_pattern"),
        Fill::Texture(_) => tr("fill.texture_fill"),
        Fill::Mesh(m) => trf(
            "fill.mesh_fill_n",
            &[("r", &m.rows.to_string()), ("c", &m.cols.to_string())],
        ),
    }
}

pub fn color_description(c: Color) -> String {
    match c {
        Color::Rgb { r, g, b } => format!(
            "R:{} G:{} B:{}",
            (r * 255.0).round(),
            (g * 255.0).round(),
            (b * 255.0).round()
        ),
        Color::Cmyk { c, m, y, k } => format!(
            "C:{} M:{} Y:{} K:{}",
            (c * 100.0).round(),
            (m * 100.0).round(),
            (y * 100.0).round(),
            (k * 100.0).round()
        ),
        Color::Gray { v } => format!("Gray {}", (v * 255.0).round()),
        Color::Hsb { h, s, b } => format!(
            "H:{} S:{} B:{}",
            h.round(),
            (s * 100.0).round(),
            (b * 100.0).round()
        ),
        Color::Hsl { h, s, l } => format!(
            "H:{} S:{} L:{}",
            h.round(),
            (s * 100.0).round(),
            (l * 100.0).round()
        ),
        Color::Lab { l, a, b } => format!("L:{} a:{} b:{}", l.round(), a.round(), b.round()),
        Color::Yiq { y, i, q } => format!(
            "Y:{} I:{} Q:{}",
            (y * 255.0).round(),
            (i * 255.0).round(),
            (q * 255.0).round()
        ),
        Color::Registration => "Registration".into(),
        Color::Spot { ink, tint, .. } => format!("Spot {ink} {:.0}%", tint * 100.0),
    }
}

/// True for a template's path (`.tdt`, `.cdt`, `.cdrt`, any case): opened,
/// it makes a new untitled drawing every time.
pub(crate) fn is_template_path(path: &std::path::Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        ["tdt", "cdt", "cdrt"]
            .iter()
            .any(|t| e.eq_ignore_ascii_case(t))
    })
}

/// True for a path ending in `.cdr` (any case).
pub(crate) fn is_cdr_path(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cdr"))
}

/// The `.cdr` version saving writes for `version`: only versions checked
/// to open in other programs are saved, so older ones become the oldest
/// of those and newer ones the newest.
pub(crate) fn writable_cdr_version(version: u16) -> u16 {
    version.clamp(
        tracedraw_cdr::write::CHECKED_SINCE,
        tracedraw_cdr::write::CHECKED_UNTIL,
    )
}

/// The status after opening a file of `format` with `n` objects.
fn opened_objects(format: &str, n: usize) -> String {
    crate::i18n::trf(
        "status.opened_objects",
        &[("format", format), ("objects", &n.to_string())],
    )
}

/// The CMYK profile a drawing carries, when it reads as a CMYK profile.
pub(crate) fn carried_cmyk_profile(
    doc: &tracedraw_core::Document,
) -> Option<tracedraw_core::icc::Profile> {
    let p = doc.cmyk_profile.as_ref()?;
    let profile = tracedraw_core::icc::Profile::parse(&p.data).ok()?;
    (profile.color_space() == tracedraw_core::icc::ColorSpace::Cmyk).then_some(profile)
}

/// The pictures a version 16+ `.cdr` file stores with the drawing: a
/// thumbnail and each page, 256 pixels on the longer side.
pub(crate) fn cdr_previews(doc: &tracedraw_core::Document) -> tracedraw_cdr::write::Previews {
    let pages: Vec<tracedraw_cdr::write::Png> = tracedraw_io::page_pictures(doc, 256)
        .into_iter()
        .map(|(data, width, height)| tracedraw_cdr::write::Png {
            data,
            width,
            height,
        })
        .collect();
    tracedraw_cdr::write::Previews {
        thumbnail: pages.first().cloned(),
        pages,
        fonts: cdr_font_faces(doc),
        embed_fonts: false,
    }
}

/// The faces the drawing's text uses, with their font files where
/// installed or carried, for the `.cdr` font table and embedding.
pub(crate) fn cdr_font_faces(
    doc: &tracedraw_core::Document,
) -> Vec<tracedraw_cdr::write::FontFace> {
    let fonts = tracedraw_text::fonts();
    doc.font_faces_used()
        .into_iter()
        .map(|(family, bold, italic)| tracedraw_cdr::write::FontFace {
            data: fonts.face_file(&family, bold, italic),
            family,
            bold,
            italic,
        })
        .collect()
}

/// The curve of the 3-point curve tool from `a` to `b` through `c`: a
/// quadratic through `c` at its middle, written as a cubic; with `arc`
/// (the constrain key held, #934) the arc of the circle through the
/// three points, a bow standing evenly on its chord wherever `c` is
/// (the quadratic when the points are in a line or nearly).
pub fn three_point_curve(a: Point, b: Point, c: Point, arc: bool) -> tracedraw_core::BezPath {
    if arc {
        if let Some(p) = three_point_arc(a, b, c) {
            return p;
        }
    }
    let ctrl = Point::new(2.0 * c.x - 0.5 * (a.x + b.x), 2.0 * c.y - 0.5 * (a.y + b.y));
    let mut path = tracedraw_core::BezPath::new();
    path.move_to(a);
    path.curve_to(
        a + (ctrl - a) * (2.0 / 3.0),
        b + (ctrl - b) * (2.0 / 3.0),
        b,
    );
    path
}

/// The arc from `a` to `b` through `c` of the circle through all three,
/// as cubic segments of at most a quarter turn; `None` when the points
/// are (nearly) in a line, the circle then too large to draw.
fn three_point_arc(a: Point, b: Point, c: Point) -> Option<tracedraw_core::BezPath> {
    use std::f64::consts::TAU;
    let chord = (b - a).hypot().max((c - a).hypot()).max((c - b).hypot());
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    // Twice the triangle's area against the square of its longest side:
    // flatter than about one part in a thousand is a line.
    let flat = d.abs() <= 2e-3 * chord * chord;
    if chord.is_nan() || chord <= 1e-9 || d.is_nan() || flat {
        return None;
    }
    let (a2, b2, c2) = (
        a.to_vec2().hypot2(),
        b.to_vec2().hypot2(),
        c.to_vec2().hypot2(),
    );
    let center = Point::new(
        (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d,
        (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d,
    );
    let r = (a - center).hypot();
    if !(r.is_finite() && center.x.is_finite() && center.y.is_finite()) {
        return None;
    }
    let angle = |p: Point| (p - center).atan2();
    let from = angle(a);
    let turn = |p: Point| (angle(p) - from).rem_euclid(TAU);
    // The way round from `a` to `b` that passes `c`.
    let to_b = turn(b);
    let sweep = if turn(c) < to_b { to_b } else { to_b - TAU };
    let n = (sweep.abs() / (TAU / 4.0)).ceil().max(1.0) as usize;
    let step = sweep / n as f64;
    // Each piece's handles lie along its tangents, k radii long, k being
    // 4/3 tan(a quarter of the piece's angle), as for any circular arc.
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let on = |t: f64| center + Vec2::new(t.cos(), t.sin()) * r;
    let tangent = |t: f64| Vec2::new(-t.sin(), t.cos()) * r;
    let mut path = tracedraw_core::BezPath::new();
    path.move_to(a);
    for i in 0..n {
        let t0 = from + step * i as f64;
        let t1 = t0 + step;
        let p0 = if i == 0 { a } else { on(t0) };
        let p1 = if i + 1 == n { b } else { on(t1) };
        path.curve_to(p0 + tangent(t0) * k, p1 - tangent(t1) * k, p1);
    }
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Duplicate, then the copy moved or scaled while it stays selected:
    /// the next Duplicate repeats that step from the copy, as the
    /// reference program does; with something else selected it is the
    /// plain offset again (#559).
    #[test]
    fn duplicate_repeats_what_was_done_to_the_copy() {
        let mut app = App::headless();
        let a = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![a]);
        let x0 = |app: &App, id: ShapeId| app.doc().find_shape(id).expect("shape").bounds().x0;
        let w = |app: &App, id: ShapeId| app.doc().find_shape(id).expect("shape").bounds().width();
        app.duplicate();
        let b = app.selection[0];
        assert!(
            (x0(&app, b) - app.duplicate_offset.x).abs() < 1e-9,
            "the offset at first"
        );
        // Moved to 20 mm from the original.
        let dx = 20.0 - app.duplicate_offset.x;
        app.transform_selection(Affine::translate((dx, -app.duplicate_offset.y)));
        assert!((x0(&app, b) - 20.0).abs() < 1e-9);
        app.duplicate();
        let c = app.selection[0];
        assert!((x0(&app, c) - 40.0).abs() < 1e-6, "{}", x0(&app, c));
        app.duplicate();
        let d = app.selection[0];
        assert!((x0(&app, d) - 60.0).abs() < 1e-6);
        // Scaled by 10 % about its left edge: each next copy 10 % larger.
        app.select(vec![a]);
        app.duplicate();
        let e = app.selection[0];
        let left = x0(&app, e);
        app.transform_selection(
            Affine::translate((left, 0.0)) * Affine::scale(1.1) * Affine::translate((-left, 0.0)),
        );
        app.duplicate();
        let f = app.selection[0];
        assert!((w(&app, f) - 12.1).abs() < 1e-6, "{}", w(&app, f));
        // Something else selected: the plain offset.
        app.select(vec![a]);
        app.duplicate();
        let g = app.selection[0];
        assert!((x0(&app, g) - app.duplicate_offset.x).abs() < 1e-9);
    }

    #[test]
    fn templates_open_as_drawings() {
        use std::path::Path;
        assert_eq!(App::file_kind(Path::new("a/Logo.CDT")), "cdr");
        assert_eq!(App::file_kind(Path::new("menu.cdrt")), "cdr");
        assert_eq!(App::file_kind(Path::new("x.cdr")), "cdr");
    }

    #[test]
    fn palette_picks_set_the_current_colours() {
        let mut app = App::headless();
        assert_eq!(app.current_colors, (Color::BLACK, Color::WHITE));
        let red = Color::rgb8(255, 0, 0);
        let blue = Color::rgb8(0, 0, 255);
        app.apply_fill(Fill::Solid(red));
        app.apply_outline_color(Some(blue));
        app.apply_outline_color(None);
        app.apply_fill(Fill::None);
        assert_eq!(app.current_colors, (red, blue));
    }

    #[test]
    fn a_drawing_shows_through_the_cmyk_profile_it_carries() {
        use tracedraw_core::icc::test_profiles as tp;
        let mut doc = tracedraw_core::Document::default();
        assert!(carried_cmyk_profile(&doc).is_none());
        doc.cmyk_profile = Some(tracedraw_core::DocumentProfile::new(
            "Synthetic CMYK",
            tp::lut16_cmyk_profile(),
        ));
        let p = carried_cmyk_profile(&doc).expect("profile");
        assert_eq!(p.description(), Some("Synthetic CMYK"));
        // Not a CMYK profile, or not a profile: the settings decide.
        let rgb = tp::matrix_trc_profile(tp::para_srgb(), tp::para_srgb(), tp::para_srgb());
        doc.cmyk_profile = Some(tracedraw_core::DocumentProfile::new("RGB", rgb));
        assert!(carried_cmyk_profile(&doc).is_none());
        doc.cmyk_profile = Some(tracedraw_core::DocumentProfile::new("Junk", vec![1, 2, 3]));
        assert!(carried_cmyk_profile(&doc).is_none());
    }

    /// Dockers move between the two columns and float; one brought to
    /// the front the old way is put in front where it lies; the places
    /// survive the preferences.
    #[test]
    fn dockers_move_between_the_columns_and_float() {
        let mut app = App::headless();
        assert_eq!(app.open_dockers, DockerTab::default_strip());
        app.move_docker(DockerTab::Hints, DockerPlace::Left, None);
        assert_eq!(app.left_dockers, [DockerTab::Hints]);
        assert!(app.show_left_dockers && app.left_tab == DockerTab::Hints);
        assert_eq!(
            app.open_dockers,
            [DockerTab::Properties, DockerTab::Objects]
        );
        assert_eq!(app.docker_tab, DockerTab::Properties);
        assert!(app.docker_shown(DockerTab::Hints) && app.docker_shown(DockerTab::Properties));
        // The old way of bringing a docker to the front, from a menu or
        // a shortcut: the left column shows it and the right column stays
        // as it was.
        app.show_dockers = false;
        app.reconcile_dockers();
        app.show_left_dockers = false;
        app.docker_tab = DockerTab::Hints;
        app.show_dockers = true;
        app.reconcile_dockers();
        assert!(app.show_left_dockers && app.left_tab == DockerTab::Hints);
        assert!(!app.show_dockers && app.docker_tab == DockerTab::Properties);
        assert_eq!(
            app.open_dockers,
            [DockerTab::Properties, DockerTab::Objects]
        );
        // Toggling a left docker closes its column; again, opens it.
        app.toggle_docker(DockerTab::Hints);
        assert!(!app.show_left_dockers);
        app.toggle_docker(DockerTab::Hints);
        assert!(app.show_left_dockers);
        // Floating: where it is dropped; closed, it is gone; opened again
        // from a menu, it joins the default column.
        app.move_docker(
            DockerTab::Objects,
            DockerPlace::Floating,
            Some(Pos2::new(300.0, 200.0)),
        );
        assert_eq!(
            app.docker_place(DockerTab::Objects),
            Some(DockerPlace::Floating)
        );
        assert_eq!(app.floating_dockers[0].pos, Pos2::new(300.0, 200.0));
        assert!(app.docker_shown(DockerTab::Objects));
        assert_eq!(app.open_dockers, [DockerTab::Properties]);
        // The places survive the preferences.
        let setup = app.docker_setup();
        let mut next = App::headless();
        next.set_docker_setup(&setup);
        assert_eq!(next.left_dockers, [DockerTab::Hints]);
        assert!(next.show_left_dockers);
        assert_eq!(next.floating_dockers[0].tab, DockerTab::Objects);
        assert_eq!(next.floating_dockers[0].pos, Pos2::new(300.0, 200.0));
        assert_eq!(next.open_dockers, [DockerTab::Properties]);
        app.close_docker(DockerTab::Objects);
        assert_eq!(app.docker_place(DockerTab::Objects), None);
        app.toggle_docker(DockerTab::Objects);
        assert_eq!(
            app.open_dockers,
            [DockerTab::Properties, DockerTab::Objects]
        );
        assert!(app.show_dockers && app.docker_tab == DockerTab::Objects);
        // Taking the last docker out of the left column closes it.
        app.move_docker(DockerTab::Hints, DockerPlace::Right, None);
        assert!(app.left_dockers.is_empty() && !app.show_left_dockers);
        assert_eq!(app.docker_tab, DockerTab::Hints);
    }

    /// Many selected objects (Select All) are gathered in one pass and
    /// kept for the frame: the same list while nothing changes, a new one
    /// when the drawing or the selection changes; objects inside groups
    /// are found too, in the order of the selection (#367).
    #[test]
    fn the_selected_objects_are_gathered_once_and_kept_while_nothing_changes() {
        use tracedraw_core::{geometry::Rect, Command};
        let mut app = App::headless();
        let mut ids = Vec::new();
        for k in 0..20 {
            let x = k as f64 * 5.0;
            ids.push(
                app.new_shape(ShapeKind::Rect {
                    rect: Rect::new(x, 0.0, x + 4.0, 4.0),
                    radius: 0.0,
                    corners: None,
                })
                .expect("rect"),
            );
        }
        // Two of them grouped, one picked inside the group.
        app.run(Command::Group {
            shapes: vec![ids[0], ids[1]],
        });
        let mut sel: Vec<ShapeId> = ids[2..].iter().rev().copied().collect();
        sel.push(ids[0]);
        app.select(sel.clone());
        let a = app.selected_rc();
        let b = app.selected_rc();
        assert!(std::rc::Rc::ptr_eq(&a, &b), "kept while nothing changes");
        let got: Vec<ShapeId> = a.iter().map(|s| s.id).collect();
        assert_eq!(got, sel, "every object, in the selection's order");
        let slow: Vec<Shape> = sel
            .iter()
            .filter_map(|id| app.doc().shape_on_page(*id))
            .collect();
        assert_eq!(*a, slow, "the same objects as one by one");
        assert!(app.selection_nested());
        // A change of the drawing: gathered again, with the change.
        app.run(Command::SetOpacity {
            shapes: vec![ids[5]],
            opacity: 0.5,
        });
        let c = app.selected_rc();
        assert!(!std::rc::Rc::ptr_eq(&a, &c));
        assert!(c
            .iter()
            .any(|s| s.id == ids[5] && (s.opacity - 0.5).abs() < 1e-9));
        // A change of the selection too.
        app.select(ids[2..12].to_vec());
        assert_eq!(app.selected_rc().len(), 10);
        assert!(!app.selection_nested());
    }

    #[test]
    fn a_closed_docker_stays_closed_the_next_time() {
        let mut app = App::headless();
        app.set_docker_setup(&crate::settings::DockerSetup {
            open: DockerTab::default_strip()
                .iter()
                .map(|t| t.key().to_string())
                .collect(),
            front: DockerTab::Hints.key().into(),
            ..Default::default()
        });
        app.show_dockers = true;
        assert_eq!(app.open_dockers, DockerTab::default_strip());
        // Closing Hints takes its tab away and brings the next one forward.
        app.close_docker(DockerTab::Hints);
        assert_eq!(
            app.open_dockers,
            [DockerTab::Properties, DockerTab::Objects]
        );
        assert_eq!(app.docker_tab, DockerTab::Properties);
        assert!(app.show_dockers);
        // The next start puts the column back as it was left.
        app.sync_settings();
        let saved = app.settings.clone();
        assert_eq!(
            saved.dockers.as_ref().map(|d| d.front.as_str()),
            Some("docker.properties")
        );
        let mut next = App::headless();
        next.apply_settings(saved);
        assert_eq!(
            next.open_dockers,
            [DockerTab::Properties, DockerTab::Objects]
        );
        assert_eq!(next.docker_tab, DockerTab::Properties);
        assert!(next.show_dockers);
        // Closing every docker closes the area, and it stays closed.
        next.close_docker(DockerTab::Objects);
        next.close_docker(DockerTab::Properties);
        assert!(next.open_dockers.is_empty() && !next.show_dockers);
        next.sync_settings();
        let mut third = App::headless();
        third.apply_settings(next.settings.clone());
        assert!(third.open_dockers.is_empty() && !third.show_dockers);
        // A docker opened from a menu joins the column again.
        third.toggle_docker(DockerTab::Hints);
        assert!(third.show_dockers && third.docker_tab == DockerTab::Hints);
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(&mut third, ui));
        out.textures_delta.clear();
        assert_eq!(third.open_dockers, [DockerTab::Hints]);
        // Names of no docker are skipped; preferences from before keep
        // the default column.
        let mut fourth = App::headless();
        fourth.set_docker_setup(&crate::settings::DockerSetup {
            open: vec!["no.such.key".into(), "docker.objects".into()],
            front: "no.such.key".into(),
            ..Default::default()
        });
        assert_eq!(fourth.open_dockers, [DockerTab::Objects]);
        assert_eq!(fourth.docker_tab, DockerTab::Objects);
        let mut old = App::headless().settings.clone();
        old.dockers = None;
        let mut fifth = App::headless();
        fifth.apply_settings(old);
        assert_eq!(fifth.open_dockers, DockerTab::default_strip());
    }

    /// Tool settings, what the window shows, snapping and the dockers'
    /// choices come back the next time, through the preferences file
    /// (JSON, as in the browser's storage).
    /// The desktop is the theme's colour until one is picked; white picked
    /// is white (it stood for the theme's colour, so picking it did
    /// nothing, #530); Reset goes back to the theme's.
    #[test]
    fn a_white_desktop_picked_is_white() {
        let mut app = App::headless();
        let theme = crate::theme::Tokens::desktop();
        assert_eq!(app.desktop_color(), theme);
        app.settings.desktop_rgb = [255, 255, 255];
        app.settings.desktop_custom = true;
        assert_eq!(app.desktop_color(), egui::Color32::WHITE);
        app.settings.desktop_custom = false;
        assert_eq!(app.desktop_color(), theme);
        // A colour kept before the flag existed still shows.
        app.settings.desktop_rgb = [200, 10, 10];
        assert_eq!(app.desktop_color(), egui::Color32::from_rgb(200, 10, 10));
    }

    /// Delete Page with a range deletes those pages in one undo step and
    /// never every page; the page after them is the current one (#500).
    #[test]
    fn delete_page_takes_a_range_of_pages() {
        let mut app = App::headless();
        for _ in 0..4 {
            app.add_page();
        }
        let ids: Vec<_> = app.doc().pages.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), 5);
        assert_eq!(crate::ui::dialogs::delete_page_range(4, 2, true, 5), (1, 3));
        assert_eq!(
            crate::ui::dialogs::delete_page_range(9, 2, false, 5),
            (4, 4)
        );
        app.delete_pages(1, 3);
        let left: Vec<_> = app.doc().pages.iter().map(|p| p.id).collect();
        assert_eq!(left, vec![ids[0], ids[4]]);
        assert_eq!(app.page, ids[4]);
        app.undo();
        assert_eq!(app.doc().pages.len(), 5, "one undo step");
        app.delete_pages(0, 4);
        assert_eq!(app.doc().pages.len(), 5, "a drawing keeps a page");
        app.delete_pages(3, 4);
        assert_eq!(app.doc().pages.len(), 3);
        assert_eq!(app.page, ids[2], "the page before them");
    }

    /// A font for new text that is not installed (uninstalled since it
    /// was kept) gives way to an installed one; an installed one stays,
    /// whatever its case (#504).
    #[test]
    fn a_default_font_no_longer_installed_gives_way() {
        let mut app = App::headless();
        app.font_families = vec!["Arial".into(), "Georgia".into()];
        app.text_default_font = "georgia".into();
        app.keep_default_font_installed();
        assert_eq!(app.text_default_font, "georgia");
        app.text_default_font = "Uninstalled Font".into();
        app.text_font = "Uninstalled Font".into();
        app.keep_default_font_installed();
        assert_eq!(app.text_default_font, "Arial");
        assert_eq!(app.text_font, "Arial");
        app.font_families = vec!["Georgia".into()];
        app.text_default_font = "Gone".into();
        app.keep_default_font_installed();
        assert_eq!(app.text_default_font, "Georgia");
    }

    #[test]
    fn tools_view_snapping_and_dockers_survive_a_restart() {
        let mut app = App::headless();
        // A first start has the preferences' defaults.
        assert_eq!(app.tool_prefs(), crate::settings::ToolPrefs::default());
        assert_eq!(app.view_prefs(), crate::settings::ViewPrefs::default());
        assert_eq!(app.panel_prefs(), crate::settings::PanelPrefs::default());
        app.polygon_points = 8;
        app.spiral_revolutions = 7;
        app.text_font = "Georgia".into();
        app.text_size_pt = 36.0;
        // The font and size of new text are what is kept (#449).
        app.text_default_font = "Georgia".into();
        app.text_default_size = 36.0;
        app.eraser_square = true;
        app.show_grid = true;
        app.show_page_border = false;
        app.view_mode = ViewMode::Wireframe;
        app.snap.dynamic_guides = true;
        app.snap.off = true;
        app.settings.snap.dynamic.angles = vec![30.0, 60.0];
        app.settings.snap.align.each_object = true;
        app.color_model = 0;
        app.color_docker.shown = [false, true, true];
        app.align_to = crate::ops2::AlignTo::PageCenter;
        app.distribute_to = crate::ops::DistributeTo::Page;
        app.envelope_mode = tracedraw_core::envelope::Mode::DoubleArc;
        app.envelope_mapping = tracedraw_core::envelope::Mapping::Vertical;
        app.envelope_keep_lines = true;
        app.mesh_rows = 4;
        app.area_outline_mode = 1;
        app.area_outline_width = 1.5;
        app.callout_style.shape = crate::callouts::CalloutShape::Ellipse;
        app.callout_style.gap = 2.5;
        app.distort.mode = crate::distort_tool::DistortMode::Zipper;
        app.distort.frequency = 9;
        // The palette starts at the bottom; moved to the right, it stays.
        use crate::ui::bars::{move_bar, Bar, Dock};
        assert_eq!(app.settings.bars.dock(Bar::Palette), Dock::Bottom);
        move_bar(&mut app, Bar::Palette, Dock::Right, None);
        app.sync_settings();
        let json = serde_json::to_string(&app.settings).expect("json");
        let saved: crate::settings::Settings = serde_json::from_str(&json).expect("read back");
        let mut next = App::headless();
        next.apply_settings(saved);
        assert_eq!(next.polygon_points, 8);
        assert_eq!(next.spiral_revolutions, 7);
        assert_eq!(next.text_font, "Georgia");
        assert_eq!(next.text_size_pt, 36.0);
        assert!(next.eraser_square);
        assert!(next.show_grid && !next.show_page_border);
        assert_eq!(next.view_mode, ViewMode::Wireframe);
        assert!(next.snap.dynamic_guides && next.snap.off);
        assert_eq!(next.settings.snap.dynamic.angles, vec![30.0, 60.0]);
        assert!(next.settings.snap.align.each_object);
        assert_eq!(next.color_model, 0);
        assert_eq!(next.color_docker.shown, [false, true, true]);
        assert_eq!(next.align_to, crate::ops2::AlignTo::PageCenter);
        assert_eq!(next.distribute_to, crate::ops::DistributeTo::Page);
        assert_eq!(next.settings.bars.dock(Bar::Palette), Dock::Right);
        assert_eq!(
            next.envelope_mode,
            tracedraw_core::envelope::Mode::DoubleArc
        );
        assert_eq!(
            next.envelope_mapping,
            tracedraw_core::envelope::Mapping::Vertical
        );
        assert!(next.envelope_keep_lines);
        assert_eq!(next.mesh_rows, 4);
        assert_eq!((next.area_outline_mode, next.area_outline_width), (1, 1.5));
        assert_eq!(
            (next.callout_style.shape, next.callout_style.gap),
            (crate::callouts::CalloutShape::Ellipse, 2.5)
        );
        assert_eq!(
            (next.distort.mode, next.distort.frequency),
            (crate::distort_tool::DistortMode::Zipper, 9)
        );
        // Values out of range in a file edited by hand are brought in.
        let mut odd = next.settings.clone();
        odd.tools.polygon_points = 0;
        odd.tools.text_size_pt = f64::NAN;
        odd.tools.text_font = " ".into();
        odd.tools.callout_style.gap = f64::NAN;
        odd.view.mode = "no.such.key".into();
        next.apply_settings(odd);
        assert_eq!(next.polygon_points, 3);
        assert_eq!(next.text_size_pt, 24.0);
        assert_eq!(next.text_font, "Georgia");
        assert_eq!(next.view_mode, ViewMode::Enhanced);
        assert_eq!(next.callout_style.gap, 1.0);
        // Preferences written before these existed start as the defaults.
        let old: crate::settings::Settings =
            serde_json::from_str(r#"{"language":"en"}"#).expect("old file");
        assert_eq!(old.tools, crate::settings::ToolPrefs::default());
        assert_eq!(old.view, crate::settings::ViewPrefs::default());
        // Alignment and dynamic guides start off, with their defaults.
        assert!(!old.snap.alignment_guides && !old.snap.dynamic_guides && !old.snap.off);
        assert_eq!(old.snap.align, crate::live_guides::AlignOptions::default());
        assert_eq!(
            old.snap.dynamic,
            crate::live_guides::DynamicOptions::default()
        );
    }

    /// The preferences are saved when something changed, not every
    /// second, and not while Options is open (Cancel puts them back).
    #[test]
    fn preferences_are_saved_when_they_change() {
        let mut app = App::headless();
        // The first look finds nothing saved yet.
        assert!(app.settings_changed());
        assert!(!app.settings_changed());
        app.polygon_points = 12;
        assert!(app.settings_changed());
        assert!(!app.settings_changed());
        // A save remembers what it wrote.
        app.show_grid = !app.show_grid;
        app.save_settings();
        assert!(!app.settings_changed());
        // Options open: autosave waits; after the dialog closes the first
        // look sees the change and writes it.
        app.options_snapshot = Some(crate::ui::options::OptionsSnapshot::take(&app));
        app.polygon_points = 9;
        app.autosave_settings(None);
        assert!(app.settings_written.is_none() && !app.settings_pending);
        app.options_snapshot = None;
        app.autosave_settings(None);
        assert!(app.settings_written.is_some() && !app.settings_pending);
        assert_eq!(app.settings.tools.polygon_points, 9);
        // Another change within the second waits for the next frame after
        // it, which it asks for.
        app.polygon_points = 10;
        let ctx = egui::Context::default();
        app.settings_looked = None;
        app.autosave_settings(Some(&ctx));
        assert!(app.settings_pending);
        app.settings_written = app
            .settings_written
            .and_then(|t| t.checked_sub(std::time::Duration::from_secs(2)));
        app.autosave_settings(Some(&ctx));
        assert!(!app.settings_pending);
    }

    /// A frame of `events` through `app`'s autosave, as the window runs
    /// it after the interface.
    /// The frame it asks for next.
    fn autosave_frame(
        app: &mut App,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> std::time::Duration {
        let input = egui::RawInput {
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.autosave_settings(Some(ui.ctx())));
        out.textures_delta.clear();
        out.viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|v| v.repaint_delay)
            .unwrap_or(std::time::Duration::MAX)
    }

    /// The preferences are looked at on the frames whose input could
    /// change them and at most once a second otherwise: a pointer move
    /// or an animation frame does not serialize them all (#616).
    #[test]
    fn preferences_are_looked_at_only_when_they_could_have_changed() {
        let mut app = App::headless();
        let ctx = egui::Context::default();
        autosave_frame(&mut app, &ctx, Vec::new());
        let looked = app.settings_looked.expect("the first frame looks");
        let seen = app.settings_saved.clone();
        // Within the second, pointer moves alone look at nothing; the
        // frame asks for one when the second is up.
        app.polygon_points = 7;
        let moved = egui::Event::PointerMoved(egui::pos2(10.0, 10.0));
        let next = autosave_frame(&mut app, &ctx, vec![moved.clone()]);
        assert_eq!(app.settings_looked, Some(looked));
        assert_eq!(app.settings_saved, seen);
        assert!(next <= std::time::Duration::from_secs(1), "{next:?}");
        // A click is looked at at once.
        let click = egui::Event::PointerButton {
            pos: egui::pos2(10.0, 10.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        };
        autosave_frame(&mut app, &ctx, vec![click]);
        assert_ne!(app.settings_saved, seen);
        assert!(app.settings_pending || app.settings_written.is_some());
        // A second later any frame looks again (a change from a worker or
        // a resized window).
        app.polygon_points = 8;
        let before = app.settings_saved.clone();
        app.settings_looked = app
            .settings_looked
            .and_then(|t| t.checked_sub(std::time::Duration::from_secs(2)));
        autosave_frame(&mut app, &ctx, vec![moved]);
        assert_ne!(app.settings_saved, before);
        // Only discrete input counts.
        let key = egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let i = |events| {
            let c = egui::Context::default();
            let input = egui::RawInput {
                events,
                ..Default::default()
            };
            c.run_ui(input, |_| {}).textures_delta.clear();
            c.input(input_may_change_settings)
        };
        assert!(i(vec![key]));
        assert!(i(vec![egui::Event::Text("a".into())]));
        assert!(!i(vec![egui::Event::PointerMoved(egui::pos2(1.0, 1.0))]));
        assert!(!i(vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        }]));
        assert!(!i(Vec::new()));
    }

    /// What the preferences check costs a frame, before (the whole JSON
    /// text built and compared on every frame) and now: run with
    /// `cargo test --release -p tracedraw -- --ignored
    /// settings_check_cost --nocapture`.
    #[test]
    #[ignore]
    fn settings_check_cost() {
        let mut app = App::headless();
        app.settings.recent_files = (0..20)
            .map(|i| format!("/some/folder/drawing {i}.cdr").into())
            .collect();
        let n = 2000;
        let mut last = None;
        let t = std::time::Instant::now();
        for _ in 0..n {
            app.sync_settings();
            let now = serde_json::to_string(&app.settings).ok();
            if now != last {
                last = now;
            }
        }
        eprintln!("before, JSON text every frame: {:?}", t.elapsed() / n);
        let t = std::time::Instant::now();
        for _ in 0..n {
            app.settings_changed();
        }
        eprintln!("a full look now: {:?}", t.elapsed() / n);
        let ctx = egui::Context::default();
        let moved = egui::RawInput {
            events: vec![egui::Event::PointerMoved(egui::pos2(10.0, 10.0))],
            ..Default::default()
        };
        ctx.run_ui(moved, |_| {}).textures_delta.clear();
        app.settings_looked = Some(web_time::Instant::now());
        let t = std::time::Instant::now();
        for _ in 0..n {
            app.autosave_settings(Some(&ctx));
        }
        eprintln!("a pointer-move frame now: {:?}", t.elapsed() / n);
    }

    #[test]
    fn resetting_the_settings_brings_back_the_defaults() {
        let mut app = App::headless();
        app.settings.shortcuts = vec![("menu.file.export".into(), "Ctrl+Shift+X".into())];
        app.settings.ui_size = 1.5;
        app.settings.recent_files = vec!["/kept.tdraw".into()];
        app.show_status_bar = false;
        app.show_page_border = false;
        app.set_workspace(Workspace::Lite);
        app.toolbox_last[0] =
            crate::tools::GROUPS[0].tools[crate::tools::GROUPS[0].tools.len() - 1];
        app.reset_settings();
        assert!(app.settings.shortcuts.is_empty());
        assert_eq!(app.settings.ui_size, 0.0);
        assert_eq!(
            app.settings.recent_files,
            vec![PathBuf::from("/kept.tdraw")]
        );
        assert_eq!(app.workspace, Workspace::Default);
        assert!(app.show_status_bar && app.show_page_border && app.show_dockers);
        assert_eq!(app.toolbox_last[0], crate::tools::GROUPS[0].tools[0]);
    }

    fn rect_shape(app: &mut App, r: Rect, filled: bool) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: r,
                radius: 0.0,
                corners: None,
            })
            .expect("a layer");
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: if filled {
                Fill::Solid(Color::BLACK)
            } else {
                Fill::None
            },
        });
        id
    }

    #[test]
    fn clip_frame_content_outside_the_frame_is_centred_in_one_step() {
        let mut app = App::headless();
        let frame = rect_shape(&mut app, Rect::new(0.0, 0.0, 100.0, 100.0), true);
        let far = rect_shape(&mut app, Rect::new(200.0, 200.0, 220.0, 220.0), true);
        let depth = app.engine.history_labels().0.len();
        app.place_inside(vec![far], frame);
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let ShapeKind::ClipFrame { contents, .. } = &app.doc().find_shape(frame).unwrap().kind
        else {
            panic!("not a ClipFrame");
        };
        let c = contents[0].bounds().center();
        assert!((c.x - 50.0).abs() < 1e-9 && (c.y - 50.0).abs() < 1e-9);
        // Content overlapping the frame stays where it is by default.
        let mut app = App::headless();
        let frame = rect_shape(&mut app, Rect::new(0.0, 0.0, 100.0, 100.0), true);
        let near = rect_shape(&mut app, Rect::new(90.0, 90.0, 120.0, 120.0), true);
        app.place_inside(vec![near], frame);
        let ShapeKind::ClipFrame { contents, .. } = &app.doc().find_shape(frame).unwrap().kind
        else {
            panic!("not a ClipFrame");
        };
        assert!((contents[0].bounds().x0 - 90.0).abs() < 1e-9);
    }

    #[test]
    fn unfilled_objects_hit_only_near_the_outline_unless_treated_as_filled() {
        let mut app = App::headless();
        app.view.zoom = 1.0;
        let id = rect_shape(&mut app, Rect::new(10.0, 10.0, 60.0, 60.0), false);
        let inside = Point::new(35.0, 35.0);
        // Treated as filled by default: a click inside takes it.
        assert!(app.settings.treat_all_filled);
        assert_eq!(app.hit_test(inside), Some(id));
        app.settings.treat_all_filled = false;
        assert_eq!(app.hit_test(inside), None);
        assert_eq!(app.hit_test(Point::new(10.5, 35.0)), Some(id));
        app.settings.treat_all_filled = true;
        assert_eq!(app.hit_test(inside), Some(id));
    }

    #[test]
    fn filled_ellipses_are_hit_inside_their_curve_not_their_box() {
        let mut app = App::headless();
        app.view.zoom = 1.0;
        let id = app
            .new_shape(ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 100.0, 100.0),
                arc: None,
            })
            .expect("a layer");
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(Color::BLACK),
        });
        assert_eq!(app.hit_test(Point::new(50.0, 50.0)), Some(id));
        // The box corner is outside the circle.
        assert_eq!(app.hit_test(Point::new(3.0, 3.0)), None);
    }

    #[test]
    fn undo_levels_limit_the_history() {
        let mut app = App::headless();
        app.settings.undo_levels = 2;
        app.apply_runtime_settings();
        for i in 0..5 {
            rect_shape(&mut app, Rect::new(0.0, 0.0, 10.0 + i as f64, 10.0), true);
        }
        assert_eq!(app.engine.history_labels().0.len(), 2);
    }

    #[test]
    fn clip_frames_turn_into_opacity_masks_and_back() {
        let mut app = App::headless();
        let frame = Shape::new(
            ShapeId(0),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        let clip = app
            .new_shape(ShapeKind::ClipFrame {
                frame: Box::new(frame),
                contents: Vec::new(),
            })
            .expect("clip");
        app.select(vec![clip]);
        app.set_opacity_masks(true, false);
        let mask = |app: &App| app.doc().find_shape(clip).and_then(|s| s.opacity_mask);
        assert_eq!(
            mask(&app),
            Some(tracedraw_core::document::OpacityMask { invert: false })
        );
        app.set_opacity_masks(true, true);
        assert_eq!(
            mask(&app),
            Some(tracedraw_core::document::OpacityMask { invert: true })
        );
        app.set_opacity_masks(false, false);
        assert_eq!(mask(&app), None);
        app.undo();
        assert!(mask(&app).is_some_and(|m| m.invert));
    }

    #[test]
    fn drawings_a_session_left_without_closing_are_recovered() {
        let dir =
            std::env::temp_dir().join(format!("tracedraw-recover-{}", crate::recovery::new_id()));
        let _ = std::fs::remove_dir_all(&dir);
        let start = crate::recovery::now_secs();
        // The first session backs a drawing up and then is gone.
        let mut first = App::headless();
        first.settings.backup.auto_dir = Some(dir.clone());
        first.settings.backup.minutes = 1;
        first.session_tick(start);
        let t0 = web_time::Instant::now();
        assert!(!first.auto_backup_tick(t0));
        rect_shape(&mut first, Rect::new(0.0, 0.0, 10.0, 10.0), true);
        // Written on a thread of its own: not saved on the frame it starts.
        assert!(!first.auto_backup_tick(t0 + std::time::Duration::from_secs(61)));
        assert!(first.finish_auto_backup(true));
        // A second session while the first still checks in: nothing.
        let mut second = App::headless();
        second.settings.backup.auto_dir = Some(dir.clone());
        second.session_tick(start + 10);
        assert!(matches!(second.dialog, crate::ui::dialogs::Dialog::None));
        // Long after the first stopped: its drawing is offered.
        let mut third = App::headless();
        third.settings.backup.auto_dir = Some(dir.clone());
        let later = crate::recovery::now_secs() + crate::recovery::STALE_SECS + 10;
        third.session_tick(later);
        let crate::ui::dialogs::Dialog::Recover(found) = third.dialog.clone() else {
            panic!("{:?}", third.dialog);
        };
        assert_eq!(found.iter().map(|f| f.backups.len()).sum::<usize>(), 1);
        third.dialog = crate::ui::dialogs::Dialog::None;
        third.recover(&found);
        assert!(third.engine.is_dirty());
        assert!(third.file.is_none());
        assert!(!third.doc().title.is_empty());
        let shapes: usize = third.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.shapes.len())
            .sum();
        assert_eq!(shapes, 1);
        assert!(!found[0].backups[0].exists());
        assert!(!found[0].marker.exists());
        // A clean exit leaves nothing behind.
        third.end_session();
        second.end_session();
        let left: Vec<_> = std::fs::read_dir(&dir)
            .map(|d| d.flatten().collect())
            .unwrap_or_default();
        assert!(left.is_empty(), "{left:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_over_a_file_keeps_a_backup_and_auto_backup_saves_dirty_drawings() {
        let dir = std::env::temp_dir().join(format!("tracedraw-backup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut app = App::headless();
        let path = dir.join("art.tdraw");
        app.save_to(path.clone());
        assert!(path.is_file());
        assert!(
            !dir.join("backup_of_art.tdraw").exists(),
            "nothing to back up yet"
        );
        rect_shape(&mut app, Rect::new(0.0, 0.0, 10.0, 10.0), true);
        app.save_to(path.clone());
        assert!(dir.join("backup_of_art.tdraw").is_file());
        // Auto-backup: due after the interval, only for unsaved changes.
        app.settings.backup.auto_dir = Some(dir.join("auto"));
        app.settings.backup.minutes = 1;
        let t0 = web_time::Instant::now();
        assert!(!app.auto_backup_tick(t0));
        rect_shape(&mut app, Rect::new(0.0, 0.0, 20.0, 10.0), true);
        assert!(!app.auto_backup_tick(t0 + std::time::Duration::from_secs(30)));
        let due = t0 + std::time::Duration::from_secs(61);
        assert!(!app.auto_backup_tick(due));
        // A later frame reports it once the thread has written it.
        let waited = web_time::Instant::now();
        while !app.auto_backup_tick(due) {
            assert!(
                waited.elapsed().as_secs() < 60,
                "auto-backup never finished"
            );
            std::thread::yield_now();
        }
        let saved: Vec<_> = std::fs::read_dir(dir.join("auto"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(saved, ["AutoBackup_of_art.tdraw"], "no .part left");
        let back = std::fs::read_to_string(dir.join("auto").join(&saved[0])).unwrap();
        assert!(tracedraw_core::Document::from_json(&back).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn postscript_ai_files_open_as_eps() {
        let dir = std::env::temp_dir().join(format!("tracedraw-ai-kind-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let old = dir.join("old.ai");
        std::fs::write(
            &old,
            b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 100 100\nnewpath 10 10 moveto 90 10 lineto 90 90 lineto closepath fill\n%%EOF\n",
        )
        .unwrap();
        let new = dir.join("new.ai");
        std::fs::write(&new, b"%PDF-1.5\n").unwrap();
        assert_eq!(App::file_kind(&old), "eps");
        assert_eq!(App::file_kind(&new), "ai");
        assert_eq!(App::file_kind(&dir.join("missing.AI")), "ai");
        assert_eq!(App::file_kind(&dir.join("x.PDF")), "pdf");
        // The old file opens through the PostScript interpreter.
        let mut app = App::headless();
        app.open_path(old);
        let n: usize = app.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.shapes.len())
            .sum();
        assert!(n >= 1, "{}", app.status);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn print_files_open_with_every_page() {
        let dir = std::env::temp_dir().join(format!("tracedraw-prn-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Two pages written by the exporter, behind a printer job header.
        let mut app = App::headless();
        rect_shape(&mut app, Rect::new(10.0, 10.0, 50.0, 50.0), true);
        app.add_page();
        rect_shape(&mut app, Rect::new(20.0, 20.0, 40.0, 40.0), true);
        let mut prn =
            b"\x1b%-12345X@PJL JOB NAME=\"art\"\n@PJL ENTER LANGUAGE=POSTSCRIPT\n".to_vec();
        prn.extend_from_slice(tracedraw_io::eps::document_to_ps(app.doc()).as_bytes());
        let path = dir.join("art.prn");
        std::fs::write(&path, prn).unwrap();
        let mut back = App::headless();
        back.open_path(path);
        assert_eq!(back.doc().pages.len(), 2, "{}", back.status);
        for p in &back.doc().pages {
            let n: usize = p.layers.iter().map(|l| l.shapes.len()).sum();
            assert!(n >= 1, "{:?}", p.name);
        }
        // Not saved over: a print file is not a drawing.
        assert!(back.file.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_browser_notice_shows_at_every_visit() {
        let mut app = App::headless();
        app.show_web_notice();
        assert!(matches!(app.dialog, crate::ui::dialogs::Dialog::WebNotice));
        app.dialog = crate::ui::dialogs::Dialog::None;
        let mut next = App::headless();
        next.show_web_notice();
        assert!(matches!(next.dialog, crate::ui::dialogs::Dialog::WebNotice));
    }
}
