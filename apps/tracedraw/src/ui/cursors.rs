//! The pointer over the drawing window shows the active tool and what a
//! press would do. The arrow and the resize and move arrows are the
//! system's own; the cross hair and the text I-beam are drawn, black on a
//! white halo, because the system's invert what is under them and can
//! show white on a white page (#655). The others are drawn too: a
//! magnifier with a plus or a minus
//! for Zoom, an open or closed hand for Pan and middle-button panning, a
//! pipette for the eyedroppers and palette sampling, a paint bucket for
//! Area Fill, turning and slanting arrows for the rotate and skew
//! handles, and the Shape tool's solid arrow with a mark for a node, a
//! segment or a control point under it. Drawn cursors hide the system
//! pointer and are painted above everything at the pointer position.
//!
//! The Pick tools: over a corner handle the diagonal resize arrows, over
//! a side handle the horizontal or vertical ones, over the selection (or
//! the centre of rotation) the move arrows; in rotate mode the corners
//! turn and the sides slant. While dragging, the pointer keeps the shape
//! of what is being done.

use crate::app::{App, Drag, Handle};
use crate::tools::Tool;
use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, Vec2};

/// What the pointer looks like over the drawing window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolCursor {
    /// A system cursor.
    System(egui::CursorIcon),
    ZoomIn,
    ZoomOut,
    Hand {
        closed: bool,
    },
    Pipette,
    Bucket,
    /// Turning the selection (rotate mode, corner handles).
    Rotate,
    /// Slanting the selection (rotate mode, side handles): sideways for
    /// the top and bottom handles, up and down for the others.
    Skew {
        horizontal: bool,
    },
    /// The Shape tool: a solid arrow with what is under it.
    ShapeArrow(ShapeMark),
    /// Moving objects with the Pick tool: the arrow with a dotted box at
    /// its lower right; with `copy` (after a right click, the move leaves
    /// the original) a plus in a square beside the box.
    DragMove {
        copy: bool,
    },
    /// The cross hair with the tool's icon at its lower right (the
    /// drawing, effect and fill tools).
    Badge(Tool),
    /// The cross hair alone (the Pick tool with the cross hair option, the
    /// ruler origin dragged out).
    Cross,
    /// The text I-beam, its middle the hot spot (where a click edits
    /// text).
    IBeam,
    /// A curve tool's badge with what a press there would do: continue
    /// the selected curve from its end node, or close the curve being
    /// drawn on its first node.
    CurveBadge(Tool, CurveMark),
    /// Picking the object a command acts on (Place Inside Frame, Order >
    /// In Front Of and Behind): a thick arrow pointing right, its tip the
    /// hot spot (#379).
    TargetArrow,
    /// The Pen tool: a pen nib whose tip, marked with a dot, is the hot
    /// spot (the node goes there, #362), with what a press would do.
    PenNib(Option<CurveMark>),
    /// The cross hair with the nib's outline around it: a circle of
    /// `radius` screen points, or a square of that half side (the Eraser,
    /// the Smooth, Smear, Twirl, Attract and Repel, Smudge and Roughen
    /// brushes).
    Nib {
        radius: f32,
        square: bool,
    },
    /// The Table tool over a row (pointing right) or column (pointing
    /// down) pointer at the table's edge: a click selects it.
    TableArrow {
        down: bool,
    },
}

/// What a curve tool's press would do besides drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveMark {
    /// Continue the selected curve from its end node.
    Join,
    /// Close the curve being drawn on its first node.
    Close,
    /// A click adds a node on a segment: of the selected curve (Pen,
    /// Bezier) or of the curve being drawn (Pen).
    AddNode,
    /// Pen: a click deletes the node (of the selected curve or of the
    /// curve being drawn).
    DeleteNode,
    /// A node a press edits: a node of the selected curve to drag, or the
    /// last node of the curve being drawn, which a click makes a cusp.
    EditNode,
}

/// What the Shape tool's arrow points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeMark {
    Nothing,
    /// A node (of a curve, or of a rectangle, ellipse or polygon).
    Node,
    /// A segment of a selected curve.
    Segment,
    /// A control point of a selected node.
    Control,
}

/// The system resize arrows for a scale handle.
pub fn resize_icon(h: Handle) -> egui::CursorIcon {
    use egui::CursorIcon as C;
    match h {
        Handle::N | Handle::S => C::ResizeVertical,
        Handle::E | Handle::W => C::ResizeHorizontal,
        Handle::NE | Handle::SW => C::ResizeNeSw,
        Handle::NW | Handle::SE => C::ResizeNwSe,
    }
}

/// The Pick tools' pointer for what is being dragged or what is under
/// `pos` (screen); `None` for the tool's plain pointer.
fn pick_cursor(app: &App, pos: Option<Pos2>) -> Option<ToolCursor> {
    let moving = ToolCursor::System(egui::CursorIcon::Move);
    match &app.drag {
        // Over the document palette letting go adds the colours; after a
        // right click the move leaves the original (a copy moves).
        Drag::Move { .. } if app.over_document_palette() => {
            return Some(ToolCursor::System(egui::CursorIcon::Copy))
        }
        Drag::Move { .. } => {
            return Some(ToolCursor::DragMove {
                copy: app.drag_copy,
            })
        }
        Drag::Pivot => return Some(moving),
        Drag::Scale { handle, .. } => return Some(ToolCursor::System(resize_icon(*handle))),
        Drag::Rotate { .. } | Drag::RotateGuide { .. } => return Some(ToolCursor::Rotate),
        Drag::Skew { handle, .. } => {
            return Some(ToolCursor::Skew {
                horizontal: matches!(handle, Handle::N | Handle::S),
            })
        }
        Drag::MoveGuide { guide, .. } => return Some(across_guide(&guide.line)),
        Drag::NodeMove { .. } => return Some(ToolCursor::ShapeArrow(ShapeMark::Node)),
        Drag::None => {}
        _ => return None,
    }
    let pos = pos?;
    // Node tracking: over a node of a selected curve, the Shape tool's
    // arrow with the node mark.
    if app.settings.node_tracking
        && !app.rotate_mode
        && app.node_at(app.view.to_page(pos)).is_some()
    {
        return Some(ToolCursor::ShapeArrow(ShapeMark::Node));
    }
    if app.pivot_at(pos) || app.center_mark_at(pos) {
        return Some(moving);
    }
    if let Some(h) = app.handle_at(pos) {
        return Some(if !app.rotate_mode {
            ToolCursor::System(resize_icon(h))
        } else if h.is_corner() {
            ToolCursor::Rotate
        } else {
            ToolCursor::Skew {
                horizontal: matches!(h, Handle::N | Handle::S),
            }
        });
    }
    let p = app.view.to_page(pos);
    // A guideline's rotation handles, then the guideline itself where no
    // object is (objects win a press there).
    if app.guide_rotate_handle_at(p) {
        return Some(ToolCursor::Rotate);
    }
    // Rotate mode: a node of the selection turns it (#933).
    if app.grab_turn_point(p).is_some() {
        return Some(ToolCursor::Rotate);
    }
    // Over an object: the move arrows only where a press would move what
    // is selected already (the object or one of its parts); over another
    // object the plain arrow, as a press there picks it first (#885).
    // Objects win a press over guidelines, so no guideline arrows there.
    if let Some(selected) = app.over_selection(p) {
        return selected.then_some(moving);
    }
    let guides = app.page_guides();
    let g = guides.get(app.guide_at(p)?)?;
    (!g.locked).then(|| across_guide(&g.line))
}

/// The arrows for moving a guideline: up and down for a horizontal one,
/// sideways for a vertical one, the move arrows for a slanted one.
fn across_guide(line: &tracedraw_core::document::GuideLine) -> ToolCursor {
    use tracedraw_core::document::GuideLine;
    ToolCursor::System(match line {
        GuideLine::Horizontal { .. } => egui::CursorIcon::ResizeVertical,
        GuideLine::Vertical { .. } => egui::CursorIcon::ResizeHorizontal,
        GuideLine::Angled { .. } => egui::CursorIcon::Move,
    })
}

/// The pointer while a guideline or the ruler origin is dragged out of
/// the rulers (with any tool): the arrows across the new guideline, the
/// cross hair for the origin.
pub fn ruler_drag_cursor(app: &App) -> Option<ToolCursor> {
    match &app.drag {
        Drag::NewGuide { horizontal, .. } => Some(ToolCursor::System(if *horizontal {
            egui::CursorIcon::ResizeVertical
        } else {
            egui::CursorIcon::ResizeHorizontal
        })),
        Drag::RulerOrigin { .. } => Some(ToolCursor::Cross),
        _ => None,
    }
}

/// The Shape tool's pointer: over the handles framing the selected nodes
/// (Stretch and Scale or Rotate and Skew Nodes) the arrows of the Pick
/// tool's handles, else its arrow with a mark.
fn shape_cursor(app: &App, pos: Option<Pos2>) -> ToolCursor {
    use crate::node_edit::NodeTransformMode as M;
    let handle = match &app.drag {
        Drag::NodeTransform { handle, .. } => Some(*handle),
        Drag::None => pos.and_then(|s| app.node_transform_handle_at(app.view.to_page(s))),
        _ => None,
    };
    match (handle, app.node_transform) {
        (Some(h), M::StretchScale) => ToolCursor::System(resize_icon(h)),
        (Some(h), M::RotateSkew) if h.is_corner() => ToolCursor::Rotate,
        (Some(h), M::RotateSkew) => ToolCursor::Skew {
            horizontal: matches!(h, Handle::N | Handle::S),
        },
        _ => ToolCursor::ShapeArrow(shape_mark(app, pos)),
    }
}

/// What the Shape tool's arrow points at: what is being dragged, else
/// what is under `pos` (screen).
fn shape_mark(app: &App, pos: Option<Pos2>) -> ShapeMark {
    if let Some(d) = &app.envelope_drag {
        return match d.part {
            crate::envelope_edit::Part::Node(_) => ShapeMark::Node,
            crate::envelope_edit::Part::Handle { .. } => ShapeMark::Control,
        };
    }
    match &app.drag {
        Drag::Node { .. } | Drag::NodeMove { .. } | Drag::KindNode { .. } => {
            return ShapeMark::Node
        }
        Drag::Handle { .. } => return ShapeMark::Control,
        Drag::SegmentBend { .. } => return ShapeMark::Segment,
        Drag::None => {}
        _ => return ShapeMark::Nothing,
    }
    let Some(pos) = pos else {
        return ShapeMark::Nothing;
    };
    let p = app.view.to_page(pos);
    match app.envelope_part_at(p) {
        Some((_, crate::envelope_edit::Part::Node(_))) => return ShapeMark::Node,
        Some((_, crate::envelope_edit::Part::Handle { .. })) => return ShapeMark::Control,
        None => {}
    }
    if app.ctrl_handle_at(p).is_some() {
        ShapeMark::Control
    } else if app.node_at(p).is_some()
        || app.kind_node_at(p).is_some()
        || app.effect_node_at(p).is_some()
    {
        ShapeMark::Node
    } else if app.segment_at(p).is_some() {
        ShapeMark::Segment
    } else {
        ShapeMark::Nothing
    }
}

/// The cursor for the current tool and input state, the pointer at `pos`
/// (screen).
pub fn for_tool(
    app: &App,
    shift: bool,
    primary_down: bool,
    middle_down: bool,
    pos: Option<Pos2>,
) -> ToolCursor {
    if middle_down {
        return ToolCursor::Hand { closed: true };
    }
    if app.pending_palette_sample {
        return ToolCursor::Pipette;
    }
    if app.pending_clip_frame
        || app.pending_order.is_some()
        || app.pending_text_link.is_some()
        || app.pending_shaping.is_some()
        || app.pending_merge_frame
    {
        return ToolCursor::TargetArrow;
    }
    if matches!(app.tool, Tool::Pick | Tool::FreeformPick) {
        if let Some(c) = pick_cursor(app, pos) {
            return c;
        }
    }
    match app.tool {
        Tool::Zoom if shift => ToolCursor::ZoomOut,
        Tool::Zoom => ToolCursor::ZoomIn,
        Tool::Pan => ToolCursor::Hand {
            closed: primary_down,
        },
        // Apply mode paints what was sampled: the bucket (Shift with the
        // attributes eyedropper samples again: the pipette).
        Tool::ColorEyedropper if app.eyedropper_apply && app.eyedropper_color.is_some() => {
            ToolCursor::Bucket
        }
        Tool::AttributesEyedropper
            if app.eyedropper_apply && app.eyedropper_attrs.is_some() && !shift =>
        {
            ToolCursor::Bucket
        }
        Tool::ColorEyedropper | Tool::AttributesEyedropper => ToolCursor::Pipette,
        Tool::AreaFill => ToolCursor::Bucket,
        // The I-beam where a click edits text, else the cross hair with the
        // tool's sign (a click starts a new text, a drag a frame).
        Tool::Text if over_text(app, pos) => ToolCursor::IBeam,
        Tool::Text => ToolCursor::Badge(Tool::Text),
        Tool::Pick | Tool::FreeformPick if app.settings.crosshair_cursor => ToolCursor::Cross,
        Tool::Pick | Tool::FreeformPick => ToolCursor::System(egui::CursorIcon::Default),
        Tool::Shape => shape_cursor(app, pos),
        Tool::Table => app.table_cursor(pos),
        // Over an envelope's node or handle: the Shape tool's arrow.
        Tool::Envelope
            if app.envelope_drag.is_some()
                || pos.is_some_and(|s| app.envelope_part_at(app.view.to_page(s)).is_some()) =>
        {
            ToolCursor::ShapeArrow(shape_mark(app, pos))
        }
        // The fill's handles move.
        Tool::InteractiveFill
            if matches!(app.drag, Drag::FountainHandle { .. })
                || pos.is_some_and(|s| app.fountain_handle_at(app.view.to_page(s)).is_some()) =>
        {
            ToolCursor::System(egui::CursorIcon::Move)
        }
        Tool::Eraser => ToolCursor::Nib {
            radius: nib_px(app.eraser_width / 2.0, app.view.zoom),
            square: app.eraser_square,
        },
        Tool::Smooth
        | Tool::Smear
        | Tool::Twirl
        | Tool::AttractRepel
        | Tool::Smudge
        | Tool::Roughen => ToolCursor::Nib {
            radius: nib_px(app.brush_radius, app.view.zoom),
            square: false,
        },
        Tool::Pen => ToolCursor::PenNib(curve_mark(app, pos)),
        t @ (Tool::Freehand
        | Tool::Bezier
        | Tool::BSpline
        | Tool::Polyline
        | Tool::TwoPointLine) => match curve_mark(app, pos) {
            Some(m) => ToolCursor::CurveBadge(t, m),
            None => ToolCursor::Badge(t),
        },
        t => ToolCursor::Badge(t),
    }
}

/// A curve tool over the first node of the curve being drawn (three
/// nodes or more: a press closes it) or over an end node of the selected
/// open curve (the new line continues it).
fn curve_mark(app: &App, pos: Option<Pos2>) -> Option<CurveMark> {
    let pos = pos?;
    let p = app.view.to_page(pos);
    let pen = matches!(app.tool, Tool::Bezier | Tool::Pen);
    if app.pen_editing {
        return Some(CurveMark::EditNode);
    }
    if let Some(c) = &app.curve {
        let near = |q: &tracedraw_core::geometry::Point| {
            app.view.to_screen(*q).distance(pos) as f64 <= app.curve_end_reach_px()
        };
        if c.closes_at_first() && c.nodes.first().is_some_and(|(f, _)| near(f)) {
            return Some(CurveMark::Close);
        }
        if pen && c.nodes.len() >= 2 && c.nodes.last().is_some_and(|(l, _)| near(l)) {
            return Some(CurveMark::EditNode);
        }
        return match app.pen_auto_at(p) {
            Some(crate::pen_look::PenAuto::Delete(_)) => Some(CurveMark::DeleteNode),
            Some(crate::pen_look::PenAuto::Add(..)) => Some(CurveMark::AddNode),
            None => None,
        };
    }
    if app.on_join_end(p) {
        return Some(CurveMark::Join);
    }
    match app.pen_edit_at(p) {
        Some(crate::interaction::PenEdit::Node(..)) if app.pen_deletes_nodes() => {
            Some(CurveMark::DeleteNode)
        }
        Some(crate::interaction::PenEdit::Node(..)) => Some(CurveMark::EditNode),
        Some(crate::interaction::PenEdit::Segment(..)) => Some(CurveMark::AddNode),
        None => None,
    }
}

/// The pointer at `pos` is over a text object (or the text being
/// edited), where a Text tool click places the caret.
fn over_text(app: &App, pos: Option<Pos2>) -> bool {
    if matches!(app.drag, Drag::TextSelect) {
        return true;
    }
    let Some(pos) = pos else {
        return false;
    };
    let p = app.view.to_page(pos);
    app.hit_test(p)
        .and_then(|id| app.doc().find_shape(id))
        .is_some_and(|s| matches!(s.kind, tracedraw_core::ShapeKind::Text { .. }))
}

/// A nib of `mm` on screen at `zoom` (points per mm), kept visible and
/// within the window.
fn nib_px(mm: f64, zoom: f32) -> f32 {
    let px = mm as f32 * zoom;
    if px.is_finite() {
        px.clamp(2.0, 400.0)
    } else {
        2.0
    }
}

/// Show `cursor` at `pos` (the pointer): set the system cursor, or hide it
/// and paint the drawn one on top of everything.
pub fn show(ctx: &egui::Context, cursor: ToolCursor, pos: Option<Pos2>) {
    let icon = match cursor {
        ToolCursor::System(icon) => icon,
        // On a software adapter a drawn pointer trails the real one by a
        // slow frame: the system's nearest instead (#616).
        _ if crate::ui::software_renderer() => system_icon(cursor),
        _ => egui::CursorIcon::None,
    };
    ctx.set_cursor_icon(icon);
    refresh_still_pointer(ctx, icon, pos);
    if icon != egui::CursorIcon::None {
        return;
    }
    if let Some(p) = pos {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("tool_cursor"),
        ));
        paint(&painter, cursor, p);
    }
}

/// Windows shows a new system pointer, or hides or shows the pointer,
/// only when the mouse next moves, so a tool picked with its shortcut
/// kept the old pointer over a still mouse (#1026). When the pointer's
/// system icon changes while the mouse stays still, it is put back where
/// it is, which Windows takes as a move and updates the pointer.
fn refresh_still_pointer(ctx: &egui::Context, icon: egui::CursorIcon, pos: Option<Pos2>) {
    let id = egui::Id::new("tool_cursor_icon");
    let last = ctx.data_mut(|d| {
        let last = d.get_temp::<egui::CursorIcon>(id);
        d.insert_temp(id, icon);
        last
    });
    let still = ctx.input(|i| i.pointer.delta() == Vec2::ZERO && !i.pointer.any_down());
    if let Some(p) = pos.filter(|_| needs_refresh(last, icon, still)) {
        if cfg!(target_os = "windows") {
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorPosition(p));
        }
    }
}

/// Whether the pointer must be refreshed: its icon changed from the one
/// shown last frame while the mouse did not move.
fn needs_refresh(last: Option<egui::CursorIcon>, now: egui::CursorIcon, still: bool) -> bool {
    still && last.is_some_and(|l| l != now)
}

/// The system pointer nearest to a drawn one.
pub fn system_icon(cursor: ToolCursor) -> egui::CursorIcon {
    use egui::CursorIcon as C;
    match cursor {
        ToolCursor::System(icon) => icon,
        ToolCursor::ZoomIn => C::ZoomIn,
        ToolCursor::ZoomOut => C::ZoomOut,
        ToolCursor::Hand { closed: true } => C::Grabbing,
        ToolCursor::Hand { closed: false } => C::Grab,
        ToolCursor::Rotate => C::AllScroll,
        ToolCursor::Skew { horizontal: true } => C::ResizeHorizontal,
        ToolCursor::Skew { horizontal: false } => C::ResizeVertical,
        ToolCursor::DragMove { copy: true } => C::Copy,
        ToolCursor::DragMove { copy: false } => C::Move,
        ToolCursor::IBeam => C::Text,
        ToolCursor::ShapeArrow(_) | ToolCursor::TargetArrow | ToolCursor::TableArrow { .. } => {
            C::Default
        }
        ToolCursor::Pipette
        | ToolCursor::Bucket
        | ToolCursor::Badge(_)
        | ToolCursor::CurveBadge(..)
        | ToolCursor::Cross
        | ToolCursor::PenNib(_)
        | ToolCursor::Nib { .. } => C::Crosshair,
    }
}

const INK: Color32 = Color32::BLACK;
const HALO: Color32 = Color32::WHITE;

/// Paint a drawn cursor with its hot spot at `p`.
pub fn paint(painter: &Painter, cursor: ToolCursor, p: Pos2) {
    match cursor {
        ToolCursor::System(_) => {}
        ToolCursor::ZoomIn | ToolCursor::ZoomOut => {
            magnifier(painter, p, cursor == ToolCursor::ZoomIn)
        }
        ToolCursor::Hand { closed } => hand(painter, p, closed),
        ToolCursor::Pipette => pipette(painter, p),
        ToolCursor::Bucket => bucket(painter, p),
        ToolCursor::Rotate => rotate_arrows(painter, p),
        ToolCursor::Skew { horizontal } => skew_arrows(painter, p, horizontal),
        ToolCursor::ShapeArrow(mark) => shape_arrow(painter, p, mark),
        ToolCursor::DragMove { copy } => drag_move(painter, p, copy),
        ToolCursor::Badge(tool) => {
            cross_hair(painter, p);
            badge(painter, p, tool);
        }
        ToolCursor::CurveBadge(tool, mark) => {
            cross_hair(painter, p);
            badge(painter, p, tool);
            curve_mark_sign(painter, p, mark);
        }
        ToolCursor::Cross => cross_hair(painter, p),
        ToolCursor::IBeam => i_beam(painter, p),
        ToolCursor::TargetArrow => target_arrow(painter, p),
        ToolCursor::PenNib(mark) => {
            pen_nib(painter, p);
            if let Some(m) = mark {
                curve_mark_at(painter, p + Vec2::new(4.0, 14.0), m);
            }
        }
        ToolCursor::Nib { radius, square } => {
            nib(painter, p, radius, square);
            cross_hair(painter, p);
        }
        ToolCursor::TableArrow { down } => table_arrow(painter, p, down),
    }
}

/// How far each arm of the cross hair reaches from its middle, points.
const CROSS_ARM: f32 = 10.0;

/// The cross hair's two lines, crossing on the middle of the pixel under
/// the hot spot `p` (`ppp` pixels a point), so they stay sharp.
pub fn cross_hair_lines(p: Pos2, ppp: f32) -> [[Pos2; 2]; 2] {
    use egui::emath::GuiRounding as _;
    let c = p.round_to_pixel_center(ppp);
    [
        [c - Vec2::new(CROSS_ARM, 0.0), c + Vec2::new(CROSS_ARM, 0.0)],
        [c - Vec2::new(0.0, CROSS_ARM), c + Vec2::new(0.0, CROSS_ARM)],
    ]
}

/// The cross hair, black on a white halo: seen on any page colour (the
/// system's inverts what is under it and can show white, #655).
fn cross_hair(painter: &Painter, p: Pos2) {
    let lines = cross_hair_lines(p, painter.pixels_per_point());
    for (w, color) in [(3.0, HALO), (1.0, INK)] {
        for l in lines {
            painter.line_segment(l, Stroke::new(w, color));
        }
    }
}

/// The text I-beam, black on a white halo, its middle at `p`.
fn i_beam(painter: &Painter, p: Pos2) {
    use egui::emath::GuiRounding as _;
    let c = p.round_to_pixel_center(painter.pixels_per_point());
    let (h, s) = (8.0, 3.0);
    let lines = [
        [c - Vec2::new(0.0, h), c + Vec2::new(0.0, h)],
        [c + Vec2::new(-s, -h), c + Vec2::new(s, -h)],
        [c + Vec2::new(-s, h), c + Vec2::new(s, h)],
    ];
    for (w, color) in [(3.0, HALO), (1.0, INK)] {
        for l in lines {
            painter.line_segment(l, Stroke::new(w, color));
        }
    }
}

/// A short thick arrow with its tip at the hot spot, pointing right (a
/// row) or down (a column), black on a white halo.
fn table_arrow(painter: &Painter, tip: Pos2, down: bool) {
    let pts = |l: f32, w: f32| -> Vec<Pos2> {
        let raw = [
            (0.0, 0.0),
            (-6.0, -5.0),
            (-6.0, -2.0),
            (-l, -2.0),
            (-l, 2.0),
            (-6.0, 2.0),
            (-6.0, 5.0),
        ];
        raw.iter()
            .map(|(x, y)| {
                let (x, y) = (*x * w, *y * w);
                if down {
                    tip + Vec2::new(-y, x)
                } else {
                    tip + Vec2::new(x, y)
                }
            })
            .collect()
    };
    let shape = pts(13.0, 1.0);
    painter.add(Shape::closed_line(shape.clone(), Stroke::new(3.0, HALO)));
    let head: Vec<Pos2> = shape
        .iter()
        .take(3)
        .copied()
        .chain([shape[5], shape[6]])
        .collect();
    painter.add(Shape::convex_polygon(head, INK, Stroke::NONE));
    let tail = vec![shape[2], shape[3], shape[4], shape[5]];
    painter.add(Shape::convex_polygon(tail, INK, Stroke::NONE));
}

/// The colour of the tool's icon on the cursor's badge: the cursor's own
/// ink, not the theme's icon colour, which is light on the dark themes
/// and vanished on the badge's light patch (the badge then looked like a
/// plain grey box, #244).
fn badge_ink() -> Color32 {
    INK
}

/// Where the tool's icon goes for the hot spot `p`: in the quarter below
/// and to the right of the cross hair, close to its middle, clear of the
/// arms and their halo. It sat 9 points off each way, which read as a
/// shape trailing the pointer (#493).
pub fn badge_rect(p: Pos2) -> Rect {
    Rect::from_min_size(p + Vec2::splat(BADGE_GAP), Vec2::splat(15.0))
}

/// The gap between the cross hair's arms and the badge's icon, points.
const BADGE_GAP: f32 = 4.0;

/// The tool's icon below and to the right of the cross hair, on a light
/// patch so it reads on any drawing.
fn badge(painter: &Painter, p: Pos2, tool: Tool) {
    let r = badge_rect(p);
    painter.rect_filled(
        r.expand(1.0),
        CornerRadius::same(3),
        Color32::from_white_alpha(200),
    );
    crate::ui::icons::draw(painter, r, tool, badge_ink());
}

/// The outline of the arrow that picks a command's target object, its
/// tip at `tip`, pointing right.
pub fn target_arrow_outline(tip: Pos2) -> Vec<Pos2> {
    [
        (0.0, 0.0),
        (-11.0, -9.0),
        (-11.0, -4.0),
        (-24.0, -4.0),
        (-24.0, 4.0),
        (-11.0, 4.0),
        (-11.0, 9.0),
    ]
    .iter()
    .map(|(x, y)| tip + Vec2::new(*x, *y))
    .collect()
}

/// The target arrow: black on a white halo.
fn target_arrow(painter: &Painter, tip: Pos2) {
    let o = target_arrow_outline(tip);
    painter.add(Shape::closed_line(o.clone(), Stroke::new(3.0, HALO)));
    let head = vec![o[0], o[1], o[2], o[5], o[6]];
    painter.add(Shape::convex_polygon(head, INK, Stroke::NONE));
    let shaft = vec![o[2], o[3], o[4], o[5]];
    painter.add(Shape::convex_polygon(shaft, INK, Stroke::NONE));
}

/// The outline of the Pen tool's nib with its tip at `tip`: the nib
/// leans up and to the right, so the tip is its lowest, leftmost point.
pub fn nib_outline(tip: Pos2) -> Vec<Pos2> {
    // Along the nib (u, from the tip) and across it (v).
    let u = Vec2::new(1.0, -1.0).normalized();
    let v = Vec2::new(u.y, -u.x);
    [
        (0.0, 0.0),
        (9.0, -4.5),
        (15.0, -4.5),
        (17.0, -3.0),
        (17.0, 3.0),
        (15.0, 4.5),
        (9.0, 4.5),
    ]
    .iter()
    .map(|(a, b)| tip + u * *a + v * *b)
    .collect()
}

/// The Pen tool's cursor: a black nib on a white halo, its slit and
/// breather hole white, and a dot at the tip where the node goes.
fn pen_nib(painter: &Painter, tip: Pos2) {
    let outline = nib_outline(tip);
    painter.add(Shape::closed_line(outline.clone(), Stroke::new(2.5, HALO)));
    painter.add(Shape::convex_polygon(outline, INK, Stroke::NONE));
    let u = Vec2::new(1.0, -1.0).normalized();
    painter.line_segment([tip + u * 2.5, tip + u * 8.0], Stroke::new(1.0, HALO));
    painter.circle_filled(tip + u * 9.5, 1.6, HALO);
    // The tip: a dot ringed in white, exactly on the hot spot.
    painter.circle_filled(tip, 2.4, HALO);
    painter.circle_filled(tip, 1.3, INK);
}

/// Above and to the right of the cross hair: a node with the line going
/// on from it (join), or a ring (close).
fn curve_mark_sign(painter: &Painter, p: Pos2, mark: CurveMark) {
    curve_mark_at(painter, p + Vec2::new(12.0, -12.0), mark);
}

/// A curve tool's sign (see [`curve_mark_sign`]) centred at `c`.
fn curve_mark_at(painter: &Painter, c: Pos2, mark: CurveMark) {
    match mark {
        CurveMark::Join => {
            haloed(painter, &[vec![c, c + Vec2::new(7.0, -5.0)]], 1.4);
            let r = Rect::from_center_size(c, Vec2::splat(5.0));
            painter.rect(
                r.expand(1.0),
                CornerRadius::ZERO,
                HALO,
                Stroke::NONE,
                egui::StrokeKind::Inside,
            );
            painter.rect_filled(r, CornerRadius::ZERO, INK);
        }
        CurveMark::Close => {
            painter.circle_stroke(c, 4.0, Stroke::new(3.4, HALO));
            painter.circle_stroke(c, 4.0, Stroke::new(1.4, INK));
        }
        CurveMark::AddNode => {
            haloed(
                painter,
                &[
                    vec![c + Vec2::new(-4.0, 0.0), c + Vec2::new(4.0, 0.0)],
                    vec![c + Vec2::new(0.0, -4.0), c + Vec2::new(0.0, 4.0)],
                ],
                1.4,
            );
        }
        CurveMark::DeleteNode => {
            haloed(
                painter,
                &[vec![c + Vec2::new(-4.0, 0.0), c + Vec2::new(4.0, 0.0)]],
                1.4,
            );
        }
        CurveMark::EditNode => {
            // A hollow node with a small arrow head beside it.
            let r = Rect::from_center_size(c, Vec2::splat(6.0));
            painter.rect(
                r,
                CornerRadius::ZERO,
                HALO,
                Stroke::new(1.2, INK),
                egui::StrokeKind::Inside,
            );
            haloed(
                painter,
                &[vec![c + Vec2::new(-7.0, 3.0), c + Vec2::new(-4.5, 0.5)]],
                1.2,
            );
        }
    }
}

/// The nib's outline around the hot spot: black with a white halo.
fn nib(painter: &Painter, c: Pos2, radius: f32, square: bool) {
    for (w, color) in [(3.0, HALO), (1.0, INK)] {
        let s = Stroke::new(w, color);
        if square {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::splat(radius * 2.0)),
                CornerRadius::ZERO,
                s,
                egui::StrokeKind::Middle,
            );
        } else {
            painter.circle_stroke(c, radius, s);
        }
    }
}

/// Draw `segments` twice: a white halo, then the ink.
fn haloed(painter: &Painter, lines: &[Vec<Pos2>], width: f32) {
    for l in lines {
        painter.add(Shape::line(l.clone(), Stroke::new(width + 2.5, HALO)));
    }
    for l in lines {
        painter.add(Shape::line(l.clone(), Stroke::new(width, INK)));
    }
}

/// An arrow that turns around the hot spot: most of a circle with a head
/// at its end.
fn rotate_arrows(painter: &Painter, c: Pos2) {
    let r = 7.0;
    let arc: Vec<Pos2> = (0..=24)
        .map(|i| {
            let a = (-200.0 + i as f32 * 11.0f32).to_radians();
            c + Vec2::new(a.cos(), a.sin()) * r
        })
        .collect();
    let end = *arc.last().unwrap_or(&c);
    let a = (-200.0f32 + 24.0 * 11.0).to_radians();
    // The tangent at the end, along the turn.
    let t = Vec2::new(-a.sin(), a.cos());
    let n = Vec2::new(a.cos(), a.sin());
    let head = vec![
        end + n * 3.5 - t * 3.5,
        end + t * 1.5,
        end - n * 3.5 - t * 3.5,
    ];
    haloed(painter, &[arc, head], 1.6);
    painter.circle_filled(c, 1.3, INK);
}

/// Two half arrows going opposite ways along the slant: sideways for a
/// horizontal skew, up and down for a vertical one.
fn skew_arrows(painter: &Painter, c: Pos2, horizontal: bool) {
    // Built sideways, then turned for a vertical skew.
    let at = |x: f32, y: f32| {
        if horizontal {
            c + Vec2::new(x, y)
        } else {
            c + Vec2::new(y, -x)
        }
    };
    let top = vec![at(-8.0, -2.5), at(8.0, -2.5), at(4.5, -6.0)];
    let bottom = vec![at(8.0, 2.5), at(-8.0, 2.5), at(-4.5, 6.0)];
    haloed(painter, &[top, bottom], 1.6);
}

/// The Shape tool's pointer: a solid arrow with its tip on the hot spot
/// and, at its lower right, a hollow square over a node, a wave over a
/// segment or a dot over a control point.
fn shape_arrow(painter: &Painter, tip: Pos2, mark: ShapeMark) {
    let at = |x: f32, y: f32| tip + Vec2::new(x, y);
    let head = vec![at(0.0, 0.0), at(0.0, 14.0), at(10.0, 10.0)];
    let tail = vec![at(3.2, 10.6), at(5.6, 9.7), at(8.6, 15.7), at(6.2, 16.6)];
    for poly in [&head, &tail] {
        painter.add(Shape::convex_polygon(
            poly.clone(),
            HALO,
            Stroke::new(2.4, HALO),
        ));
    }
    for poly in [head, tail] {
        painter.add(Shape::convex_polygon(poly, INK, Stroke::new(0.6, INK)));
    }
    let m = at(14.0, 15.0);
    match mark {
        ShapeMark::Nothing => {}
        ShapeMark::Node => {
            let r = Rect::from_center_size(m, Vec2::splat(6.0));
            painter.rect(
                r,
                CornerRadius::ZERO,
                HALO,
                Stroke::new(1.2, INK),
                egui::StrokeKind::Inside,
            );
        }
        ShapeMark::Segment => {
            let wave: Vec<Pos2> = (0..=12)
                .map(|i| {
                    let x = -4.5 + i as f32 * 0.75;
                    m + Vec2::new(x, -2.2 * (x * 0.7).sin())
                })
                .collect();
            haloed(painter, &[wave], 1.4);
        }
        ShapeMark::Control => {
            painter.circle(m, 2.6, INK, Stroke::new(1.2, HALO));
        }
    }
}

/// The pointer while objects are moved: a white arrow outlined in black
/// with its tip on the hot spot, a dotted box under its tail, and with
/// `copy` a plus in a square at the box's lower right.
fn drag_move(painter: &Painter, tip: Pos2, copy: bool) {
    let at = |x: f32, y: f32| tip + Vec2::new(x, y);
    let arrow = vec![
        at(0.0, 0.0),
        at(0.0, 15.0),
        at(3.6, 11.6),
        at(6.2, 17.4),
        at(8.6, 16.4),
        at(6.0, 10.8),
        at(10.8, 10.8),
    ];
    painter.add(Shape::closed_line(arrow.clone(), Stroke::new(3.0, HALO)));
    // Filled as two convex parts (the head and the tail), outlined whole.
    for part in [
        vec![
            at(0.0, 0.0),
            at(0.0, 15.0),
            at(3.6, 11.6),
            at(6.0, 10.8),
            at(10.8, 10.8),
        ],
        vec![at(3.6, 11.6), at(6.2, 17.4), at(8.6, 16.4), at(6.0, 10.8)],
    ] {
        painter.add(Shape::convex_polygon(part, HALO, Stroke::NONE));
    }
    painter.add(Shape::closed_line(arrow, Stroke::new(1.0, INK)));
    // The dotted box.
    let b = Rect::from_min_size(at(7.0, 13.0), Vec2::new(11.0, 8.0));
    painter.rect_filled(b.expand(1.0), CornerRadius::ZERO, HALO);
    let corners = [
        b.left_top(),
        b.right_top(),
        b.right_bottom(),
        b.left_bottom(),
    ];
    for i in 0..4 {
        painter.extend(Shape::dashed_line(
            &[corners[i], corners[(i + 1) % 4]],
            Stroke::new(1.0, INK),
            1.5,
            1.5,
        ));
    }
    if copy {
        copy_plus(painter, at(15.0, 17.0));
    }
}

/// The plus in a square that says a drag leaves the original, at the
/// lower right of the pointer `p` (a scale, turn or slant after a right
/// click; a move's own pointer draws it beside its box).
pub fn paint_copy_plus(ctx: &egui::Context, p: Pos2) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("tool_cursor_copy"),
    ));
    copy_plus(&painter, p + Vec2::new(14.0, 16.0));
}

/// A 9-point square with a plus, its top left at `at`.
fn copy_plus(painter: &Painter, at: Pos2) {
    let s = Rect::from_min_size(at, Vec2::splat(9.0));
    painter.rect(
        s,
        CornerRadius::ZERO,
        HALO,
        Stroke::new(1.0, INK),
        egui::StrokeKind::Inside,
    );
    let c = s.center();
    painter.line_segment(
        [c - Vec2::new(2.5, 0.0), c + Vec2::new(2.5, 0.0)],
        Stroke::new(1.2, INK),
    );
    painter.line_segment(
        [c - Vec2::new(0.0, 2.5), c + Vec2::new(0.0, 2.5)],
        Stroke::new(1.2, INK),
    );
}

/// A lens centred on the hot spot with a handle to the lower right and a
/// plus or a minus inside.
fn magnifier(painter: &Painter, c: Pos2, plus: bool) {
    let r = 6.5;
    let a = c + Vec2::splat(r * 0.72);
    let b = c + Vec2::splat(r * 0.72 + 7.0);
    painter.line_segment([a, b], Stroke::new(5.0, HALO));
    painter.circle(c, r, Color32::from_white_alpha(230), Stroke::new(3.0, HALO));
    painter.line_segment([a, b], Stroke::new(3.0, INK));
    painter.circle_stroke(c, r, Stroke::new(1.5, INK));
    let s = Stroke::new(1.4, INK);
    let d = 3.2;
    painter.line_segment([c - Vec2::new(d, 0.0), c + Vec2::new(d, 0.0)], s);
    if plus {
        painter.line_segment([c - Vec2::new(0.0, d), c + Vec2::new(0.0, d)], s);
    }
}

/// An open hand (fingers up) or a closed one (fingers folded): outlined
/// fingers and thumb drawn as capsules, then the palm over their roots.
/// The hot spot is the middle of the palm.
fn hand(painter: &Painter, c: Pos2, closed: bool) {
    let capsule = |a: Pos2, b: Pos2| {
        painter.line_segment([a, b], Stroke::new(4.2, INK));
        painter.line_segment([a, b], Stroke::new(2.2, HALO));
    };
    let tops: [f32; 4] = if closed {
        [-2.5, -3.5, -3.5, -2.0]
    } else {
        [-8.0, -9.5, -9.0, -6.5]
    };
    for (x, top) in [-4.5f32, -1.5, 1.5, 4.5].iter().zip(tops) {
        capsule(c + Vec2::new(*x, top), c + Vec2::new(*x, 2.0));
    }
    if closed {
        capsule(c + Vec2::new(-7.5, 2.0), c + Vec2::new(-4.5, 4.0));
    } else {
        capsule(c + Vec2::new(-9.5, -0.5), c + Vec2::new(-5.5, 4.0));
    }
    let palm = Rect::from_min_max(c + Vec2::new(-6.2, -1.0), c + Vec2::new(6.2, 8.0));
    painter.rect(
        palm,
        CornerRadius::same(3),
        HALO,
        Stroke::new(1.0, INK),
        egui::StrokeKind::Inside,
    );
    // Hide the palm's top edge between the fingers so they join the palm.
    for x in [-4.5f32, -1.5, 1.5, 4.5] {
        painter.line_segment(
            [c + Vec2::new(x, -0.5), c + Vec2::new(x, 1.5)],
            Stroke::new(2.2, HALO),
        );
    }
}

/// A pipette: the tip at the hot spot, the glass tube rising to the upper
/// right and the rubber bulb at its end.
fn pipette(painter: &Painter, tip: Pos2) {
    let dir = Vec2::new(1.0, -1.0).normalized();
    let tube_start = tip + dir * 1.5;
    let tube_end = tip + dir * 11.0;
    let bulb = tip + dir * 14.5;
    painter.line_segment([tube_start, tube_end], Stroke::new(5.0, HALO));
    painter.circle_filled(bulb, 4.4, HALO);
    painter.line_segment([tube_start, tube_end], Stroke::new(2.6, INK));
    painter.circle_filled(bulb, 3.0, INK);
    painter.circle_filled(tip, 1.4, INK);
}

/// A tipped paint bucket pouring onto the hot spot.
fn bucket(painter: &Painter, spot: Pos2) {
    // The bucket body, a slanted box above and to the right of the spot.
    let o = spot + Vec2::new(4.0, -12.0);
    let pts = [
        o,
        o + Vec2::new(9.0, 4.0),
        o + Vec2::new(5.5, 12.0),
        o + Vec2::new(-3.5, 8.0),
    ];
    painter.add(Shape::convex_polygon(
        pts.to_vec(),
        HALO,
        Stroke::new(3.0, HALO),
    ));
    painter.add(Shape::convex_polygon(
        pts.to_vec(),
        HALO,
        Stroke::new(1.3, INK),
    ));
    // The handle.
    painter.line_segment(
        [o + Vec2::new(1.0, -1.0), o + Vec2::new(7.0, 2.0)],
        Stroke::new(1.2, INK),
    );
    // Paint flowing from the lip to the spot.
    painter.line_segment([o + Vec2::new(-3.0, 7.0), spot], Stroke::new(3.0, HALO));
    painter.line_segment([o + Vec2::new(-3.0, 7.0), spot], Stroke::new(1.6, INK));
    painter.circle_filled(spot, 2.0, INK);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On a software adapter the drawn pointers give way to the system's
    /// nearest (#616): the cross hair for the drawing tools, the I-beam
    /// for text, the hands and magnifiers as the system has them.
    #[test]
    fn a_software_adapter_shows_the_system_pointers() {
        use egui::CursorIcon as C;
        assert_eq!(system_icon(ToolCursor::Cross), C::Crosshair);
        assert_eq!(
            system_icon(ToolCursor::Badge(Tool::Rectangle)),
            C::Crosshair
        );
        assert_eq!(system_icon(ToolCursor::IBeam), C::Text);
        assert_eq!(system_icon(ToolCursor::Hand { closed: true }), C::Grabbing);
        assert_eq!(system_icon(ToolCursor::ZoomOut), C::ZoomOut);
        assert_eq!(system_icon(ToolCursor::DragMove { copy: true }), C::Copy);
        assert_eq!(system_icon(ToolCursor::System(C::Move)), C::Move);
        // Nothing maps to a hidden pointer.
        for c in [
            ToolCursor::Pipette,
            ToolCursor::Bucket,
            ToolCursor::Rotate,
            ToolCursor::Skew { horizontal: false },
            ToolCursor::TargetArrow,
            ToolCursor::TableArrow { down: true },
        ] {
            assert_ne!(system_icon(c), C::None);
        }
    }

    /// A tool picked by its shortcut over a still mouse changes the
    /// system pointer at once on Windows too: the pointer is put back
    /// where it is, which Windows takes as a move (#1026). Nothing happens
    /// when the pointer moved, a button is held or the icon stayed.
    #[test]
    fn a_new_pointer_over_a_still_mouse_is_refreshed() {
        use egui::CursorIcon as C;
        assert!(needs_refresh(Some(C::Default), C::None, true));
        assert!(needs_refresh(Some(C::None), C::Text, true));
        assert!(!needs_refresh(Some(C::Default), C::None, false));
        assert!(!needs_refresh(Some(C::Text), C::Text, true));
        assert!(!needs_refresh(None, C::Text, true));

        let ctx = crate::theme::ui_context();
        let at = Pos2::new(120.0, 80.0);
        let frame = |cursor: ToolCursor| {
            let input = egui::RawInput {
                events: vec![egui::Event::PointerMoved(at)],
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| show(ui.ctx(), cursor, Some(at)));
            out.textures_delta.clear();
            out.viewport_output
                .get(&egui::ViewportId::ROOT)
                .is_some_and(|v| {
                    v.commands
                        .iter()
                        .any(|c| *c == egui::ViewportCommand::CursorPosition(at))
                })
        };
        assert!(!frame(ToolCursor::System(C::Default)));
        assert!(!frame(ToolCursor::System(C::Default)));
        // The Pen tool picked by its shortcut: the drawn nib hides the
        // system arrow, and the same frame puts the pointer back.
        assert_eq!(frame(ToolCursor::PenNib(None)), cfg!(target_os = "windows"));
        assert!(!frame(ToolCursor::PenNib(None)));
    }

    /// The cross hair (alone, with a tool's badge, with a nib) and the
    /// I-beam are drawn black on a white halo with the system pointer
    /// hidden, so they show on a page of any colour: the system's invert
    /// what is under them and could show white on a white page (#655). The
    /// cross hair's lines cross on the hot spot; the system's arrows stay.
    #[test]
    fn the_cross_hair_and_the_i_beam_are_drawn_black_on_white() {
        let ctx = crate::theme::ui_context();
        let at = Pos2::new(200.0, 150.0);
        for c in [
            ToolCursor::Badge(Tool::Rectangle),
            ToolCursor::CurveBadge(Tool::Bezier, CurveMark::Join),
            ToolCursor::Nib {
                radius: 8.0,
                square: false,
            },
            ToolCursor::Cross,
            ToolCursor::IBeam,
        ] {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| show(ui.ctx(), c, Some(at)));
            out.textures_delta.clear();
            assert_eq!(
                out.platform_output.cursor_icon,
                egui::CursorIcon::None,
                "{c:?}"
            );
            let mut ink = Vec::new();
            let mut halo = 0;
            for clipped in &out.shapes {
                if let egui::Shape::LineSegment { points, stroke } = &clipped.shape {
                    if stroke.color == INK && (stroke.width - 1.0).abs() < 1e-3 {
                        ink.push(*points);
                    } else if stroke.color == HALO && stroke.width > 2.0 {
                        halo += 1;
                    }
                }
            }
            let through = |p: &[Pos2; 2]| {
                let (a, b) = (p[0], p[1]);
                let d = b - a;
                let t = ((at - a).dot(d) / d.length_sq()).clamp(0.0, 1.0);
                (a + d * t - at).length() < 0.75
            };
            if c != ToolCursor::IBeam {
                let level = ink
                    .iter()
                    .filter(|p| through(p) && (p[0].y - p[1].y).abs() < 1e-3)
                    .count();
                let upright = ink
                    .iter()
                    .filter(|p| through(p) && (p[0].x - p[1].x).abs() < 1e-3)
                    .count();
                assert!(level >= 1 && upright >= 1, "{c:?}: {ink:?}");
            } else {
                assert!(ink.iter().any(through), "{c:?}");
            }
            assert!(halo >= ink.len().min(2), "a halo under the ink: {c:?}");
        }
        let out = ctx.run_ui(egui::RawInput::default(), |ui| {
            show(
                ui.ctx(),
                ToolCursor::System(egui::CursorIcon::Move),
                Some(at),
            )
        });
        assert_eq!(out.platform_output.cursor_icon, egui::CursorIcon::Move);
        // The lines cross on the middle of the pixel under the hot spot.
        let [h, v] = cross_hair_lines(Pos2::new(10.2, 20.7), 1.0);
        assert_eq!((h[0].y, v[0].x), (20.5, 10.5));
        assert_eq!(h[1].x - 10.5, 10.5 - h[0].x);
    }

    /// The tool's icon sits close to the cross hair's middle, in its lower
    /// right quarter, its patch clear of the arms and their halo (#493).
    #[test]
    fn the_badge_sits_close_to_the_cross_hair() {
        let p = Pos2::new(200.5, 150.5);
        let r = badge_rect(p);
        assert!(r.min.x - p.x <= 5.0 && r.min.y - p.y <= 5.0, "{r:?}");
        let patch = r.expand(1.0);
        // The halo is 3 points wide, centred on the arms.
        assert!(
            patch.min.x > p.x + 1.5 && patch.min.y > p.y + 1.5,
            "{patch:?}"
        );
    }

    /// The badge's icon reads on its light patch under every theme: the
    /// theme's icon colour is light on the dark ones.
    #[test]
    fn the_badge_icon_is_dark_under_every_theme() {
        for theme in crate::theme::Theme::ALL {
            crate::theme::set_theme(theme);
            let ink = badge_ink();
            let luma = 0.299 * ink.r() as f32 + 0.587 * ink.g() as f32 + 0.114 * ink.b() as f32;
            assert!(luma < 80.0, "{theme:?}: {ink:?}");
            assert_eq!(ink.a(), 255);
        }
    }

    #[test]
    fn tools_get_their_cursors() {
        let mut app = App::headless();
        app.tool = Tool::Zoom;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::ZoomIn
        );
        assert_eq!(
            for_tool(&app, true, false, false, None),
            ToolCursor::ZoomOut
        );
        app.tool = Tool::Pan;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::Hand { closed: false }
        );
        assert_eq!(
            for_tool(&app, false, true, false, None),
            ToolCursor::Hand { closed: true }
        );
        app.tool = Tool::ColorEyedropper;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::Pipette
        );
        // A colour sampled: Apply mode paints with the bucket (#403).
        app.eyedropper_color = Some(tracedraw_core::Color::rgb8(255, 0, 0));
        app.eyedropper_apply = true;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::Bucket
        );
        app.eyedropper_reset();
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::Pipette
        );
        app.tool = Tool::AreaFill;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::Bucket
        );
        // The Text tool: the cross hair with its sign away from text, the
        // I-beam over text.
        app.tool = Tool::Text;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::Badge(Tool::Text)
        );
        app.canvas_rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let page = app.page_rect();
        app.view.fit(page, app.canvas_rect);
        app.start_text(tracedraw_core::geometry::Point::new(50.0, 50.0), None);
        for c in "Hello".chars() {
            app.text_insert(&c.to_string());
        }
        app.finish_text();
        let on = app
            .view
            .to_screen(tracedraw_core::geometry::Point::new(52.0, 51.0));
        assert_eq!(
            for_tool(&app, false, false, false, Some(on)),
            ToolCursor::IBeam
        );
        app.tool = Tool::Rectangle;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::Badge(Tool::Rectangle)
        );
        // Curve tools: a join sign over an end node of the selected open
        // curve, a close sign over the first node of the curve being drawn.
        app.tool = Tool::Bezier;
        app.select(Vec::new());
        app.add_drawn_line({
            let mut l = tracedraw_core::geometry::BezPath::new();
            l.move_to((10.0, 10.0));
            l.line_to((60.0, 10.0));
            l
        });
        let end = app
            .view
            .to_screen(tracedraw_core::geometry::Point::new(60.0, 10.0));
        assert_eq!(
            for_tool(&app, false, false, false, Some(end)),
            ToolCursor::CurveBadge(Tool::Bezier, CurveMark::Join)
        );
        app.curve = Some(crate::app::CurveInProgress {
            nodes: vec![
                (tracedraw_core::geometry::Point::new(100.0, 100.0), None),
                (tracedraw_core::geometry::Point::new(150.0, 100.0), None),
                (tracedraw_core::geometry::Point::new(150.0, 150.0), None),
            ],
            smooth: true,
            dragging_handle: false,
            bspline: false,
            ..Default::default()
        });
        let first = app
            .view
            .to_screen(tracedraw_core::geometry::Point::new(100.0, 100.0));
        assert_eq!(
            for_tool(&app, false, false, false, Some(first)),
            ToolCursor::CurveBadge(Tool::Bezier, CurveMark::Close)
        );
        // The Pen: the nib, with the same signs (#362).
        app.tool = Tool::Pen;
        assert_eq!(
            for_tool(&app, false, false, false, Some(first)),
            ToolCursor::PenNib(Some(CurveMark::Close))
        );
        app.curve = None;
        let away = app
            .view
            .to_screen(tracedraw_core::geometry::Point::new(300.0, 300.0));
        assert_eq!(
            for_tool(&app, false, false, false, Some(away)),
            ToolCursor::PenNib(None)
        );
        // Its tip is the hot spot: the outline starts there and leans up
        // and to the right of it.
        let tip = egui::pos2(50.0, 50.0);
        let o = nib_outline(tip);
        assert_eq!(o[0], tip);
        assert!(
            o.iter().all(|q| q.x >= tip.x - 4.0 && q.y <= tip.y + 4.0),
            "{o:?}"
        );
        assert!(o.iter().any(|q| q.x > tip.x + 8.0 && q.y < tip.y - 8.0));
        // Picking the frame for Place Inside Frame: the target arrow.
        app.pending_clip_frame = true;
        assert_eq!(
            for_tool(&app, false, false, false, Some(away)),
            ToolCursor::TargetArrow
        );
        app.pending_clip_frame = false;
        let o = target_arrow_outline(tip);
        assert_eq!(o[0], tip);
        assert!(o.iter().all(|q| q.x <= tip.x));
        // The Eraser and the brushes show their nib at its size on screen.
        app.tool = Tool::Eraser;
        app.eraser_width = 10.0;
        app.view.zoom = 2.0;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::Nib {
                radius: 10.0,
                square: app.eraser_square
            }
        );
        app.tool = Tool::Smudge;
        app.brush_radius = f64::NAN;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::Nib {
                radius: 2.0,
                square: false
            }
        );
        app.tool = Tool::Pick;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::System(egui::CursorIcon::Default)
        );
        // Middle-button panning shows the closed hand with any tool.
        assert_eq!(
            for_tool(&app, false, false, true, None),
            ToolCursor::Hand { closed: true }
        );
    }
}
