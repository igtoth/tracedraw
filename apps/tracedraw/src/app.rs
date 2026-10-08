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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Units {
    Millimeters,
    Centimeters,
    Inches,
    Points,
    Pixels,
}

impl Units {
    pub const ALL: [Units; 5] = [
        Units::Millimeters,
        Units::Centimeters,
        Units::Inches,
        Units::Points,
        Units::Pixels,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Units::Millimeters => "millimeters",
            Units::Centimeters => "centimeters",
            Units::Inches => "inches",
            Units::Points => "points",
            Units::Pixels => "pixels",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            Units::Millimeters => "mm",
            Units::Centimeters => "cm",
            Units::Inches => "\"",
            Units::Points => "pt",
            Units::Pixels => "px",
        }
    }
    /// Millimetres per unit.
    pub fn mm(self) -> f64 {
        match self {
            Units::Millimeters => 1.0,
            Units::Centimeters => 10.0,
            Units::Inches => 25.4,
            Units::Points => 25.4 / 72.0,
            Units::Pixels => 25.4 / 96.0,
        }
    }
    pub fn from_mm(self, v: f64) -> f64 {
        v / self.mm()
    }
    pub fn to_mm(self, v: f64) -> f64 {
        v * self.mm()
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
    /// Drawing a new box shape from `start` (page space).
    Box {
        start: Point,
        current: Point,
    },
    /// Moving the selection.
    Move {
        last: Point,
        total: Vec2,
        start_bounds: Rect,
    },
    /// Scaling the selection with a handle; `anchor` stays fixed.
    Scale {
        handle: Handle,
        anchor: Point,
        start_bounds: Rect,
        current: Point,
    },
    /// Rotating the selection about `center`.
    Rotate {
        center: Point,
        start_angle: f64,
        current_angle: f64,
    },
    /// Rubber-band selection.
    Marquee {
        start: Point,
        current: Point,
    },
    /// Freehand stroke being drawn.
    Freehand {
        points: Vec<Point>,
    },
    /// Dragging the selected nodes (Shape tool).
    Node {
        last: Point,
    },
    /// Dragging a control handle (Shape tool).
    Handle {
        shape: ShapeId,
        index: usize,
        which: tracedraw_core::nodes::Which,
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
        current: Point,
    },
    /// Dragging a drop shadow offset (Drop Shadow tool).
    Shadow {
        shape: ShapeId,
        start: Point,
    },
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
    /// Dragging an existing guideline.
    MoveGuide {
        index: usize,
    },
}

/// Bezier / polyline tool in progress.
#[derive(Debug, Clone, Default)]
pub struct CurveInProgress {
    /// Nodes placed so far with their outgoing handle (dragged with the
    /// Bezier/Pen tool); the incoming handle mirrors the previous node's.
    pub nodes: Vec<(Point, Option<Point>)>,
    /// Polyline tools make straight segments; curve tools make curves.
    pub smooth: bool,
    /// Pointer is being dragged to set the handle of the last node.
    pub dragging_handle: bool,
}

impl CurveInProgress {
    /// Path through the nodes, optionally extended to a preview point.
    pub fn path(&self, preview: Option<Point>) -> tracedraw_core::BezPath {
        let mut path = tracedraw_core::BezPath::new();
        let mut nodes: Vec<(Point, Option<Point>)> = self.nodes.clone();
        if let Some(p) = preview {
            nodes.push((p, None));
        }
        for (i, (p, _)) in nodes.iter().enumerate() {
            if i == 0 {
                path.move_to(*p);
                continue;
            }
            let (prev, prev_out) = nodes[i - 1];
            let cur_in = nodes[i].1.map(|out| *p - (out - *p));
            match (self.smooth, prev_out, cur_in) {
                (false, _, _) | (true, None, None) => path.line_to(*p),
                (true, po, ci) => path.curve_to(po.unwrap_or(prev), ci.unwrap_or(*p), *p),
            }
        }
        path
    }
}

/// Artistic text being typed.
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub shape: ShapeId,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Clipboard {
    pub shapes: Vec<Shape>,
}

pub struct App {
    pub engine: Engine,
    pub page: PageId,
    pub view: View,
    pub tool: Tool,
    pub previous_tool: Tool,
    pub selection: Vec<ShapeId>,
    /// Second click on a selected object switches to rotate/skew handles.
    pub rotate_mode: bool,
    pub drag: Drag,
    pub curve: Option<CurveInProgress>,
    pub text_edit: Option<TextEdit>,
    pub file: Option<PathBuf>,
    pub status: String,
    pub fit_pending: bool,
    pub units: Units,
    pub nudge_mm: f64,
    pub show_rulers: bool,
    pub show_grid: bool,
    pub show_status_bar: bool,
    pub show_dockers: bool,
    pub docker_tab: DockerTab,
    pub flyout_open: Option<usize>,
    pub clipboard: Option<Clipboard>,
    pub duplicate_offset: Vec2,
    // Defaults for new objects (the editor: no fill, black hairline).
    pub default_fill: Fill,
    pub default_stroke: Option<Stroke>,
    pub polygon_points: u32,
    pub star_sharpness: f64,
    pub rect_radius: f64,
    pub ellipse_arc: Option<tracedraw_core::EllipseArc>,
    pub contour_steps: u32,
    pub contour_offset: f64,
    pub contour_direction: crate::tools2::ContourDirection,
    pub contour_color: Color,
    pub spiral_revolutions: u32,
    pub spiral_logarithmic: bool,
    pub common_shape: crate::tools2::CommonShape,
    pub table_rows: u32,
    pub table_cols: u32,
    pub media_width: f64,
    pub media_angle: f64,
    pub dimension_points: Vec<Point>,
    pub blend_steps: u32,
    pub extrude_depth: Vec2,
    pub distort_mode: crate::tools2::DistortMode,
    pub distort_amount: f64,
    pub distort_frequency: u32,
    pub brush_radius: f64,
    pub shadow_default: tracedraw_core::Shadow,
    pub text_font: String,
    pub text_size_pt: f64,
    pub text_bold: bool,
    pub text_italic: bool,
    pub text_align: tracedraw_core::TextAlign,
    pub eyedropper_color: Option<Color>,
    pub canvas_rect: egui::Rect,
    pub pointer_page: Option<Point>,
    pub about_open: bool,
    pub palette: Vec<Color>,
    pub raster: std::cell::RefCell<crate::raster::Raster>,
    pub wireframe: bool,
    pub font_families: Vec<String>,
    pub transform_tab: TransformTab,
    /// Selected nodes: (shape, element index).
    pub node_selection: Vec<(ShapeId, usize)>,
    pub snap: crate::snap::SnapSettings,
    pub dialog: crate::ui::dialogs::Dialog,
    pub show_welcome: bool,
    pub show_guides: bool,
    pub selected_guide: Option<usize>,
    pub transform_values: [f64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerTab {
    Properties,
    Objects,
    Hints,
    Transformations,
    Undo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformTab {
    Position,
    Rotate,
    Scale,
    Size,
    Skew,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Option<PathBuf>) -> Self {
        cc.egui_ctx.set_visuals(theme::visuals());
        cc.egui_ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(4.0, 3.0);
            style.spacing.button_padding = egui::vec2(5.0, 2.0);
        });

        let engine = Engine::default();
        let page = engine.document().pages[0].id;
        let mut app = App {
            engine,
            page,
            view: View::default(),
            tool: Tool::Pick,
            previous_tool: Tool::Pick,
            selection: Vec::new(),
            rotate_mode: false,
            drag: Drag::None,
            curve: None,
            text_edit: None,
            file: None,
            status: String::new(),
            fit_pending: true,
            units: Units::Millimeters,
            nudge_mm: 2.54,
            show_rulers: true,
            show_grid: false,
            show_status_bar: true,
            show_dockers: true,
            docker_tab: DockerTab::Properties,
            flyout_open: None,
            clipboard: None,
            duplicate_offset: Vec2::new(6.35, 6.35),
            default_fill: Fill::None,
            default_stroke: Some(Stroke::default()),
            polygon_points: 5,
            star_sharpness: 0.5,
            rect_radius: 0.0,
            ellipse_arc: None,
            contour_steps: 3,
            contour_offset: 2.0,
            contour_direction: crate::tools2::ContourDirection::Outside,
            contour_color: Color::WHITE,
            spiral_revolutions: 4,
            spiral_logarithmic: false,
            common_shape: crate::tools2::CommonShape::RightArrow,
            table_rows: 3,
            table_cols: 4,
            media_width: 3.0,
            media_angle: 45.0,
            dimension_points: Vec::new(),
            blend_steps: 5,
            extrude_depth: Vec2::new(8.0, -8.0),
            distort_mode: crate::tools2::DistortMode::PushPull,
            distort_amount: 20.0,
            distort_frequency: 12,
            brush_radius: 10.0,
            shadow_default: tracedraw_core::Shadow::default(),
            text_font: "Arial".into(),
            text_size_pt: 24.0,
            text_bold: false,
            text_italic: false,
            text_align: tracedraw_core::TextAlign::Left,
            eyedropper_color: None,
            canvas_rect: egui::Rect::NOTHING,
            pointer_page: None,
            about_open: false,
            palette: default_palette(),
            raster: std::cell::RefCell::new(crate::raster::Raster::default()),
            wireframe: false,
            font_families: tracedraw_text::fonts().families().to_vec(),
            transform_tab: TransformTab::Position,
            node_selection: Vec::new(),
            snap: crate::snap::SnapSettings::default(),
            dialog: crate::ui::dialogs::Dialog::None,
            show_welcome: false,
            show_guides: true,
            selected_guide: None,
            transform_values: [0.0, 0.0, 100.0, 100.0],
        };
        if let Some(p) = open {
            app.open_path(p);
        }
        app
    }

    // ----- document helpers --------------------------------------------------

    pub fn doc(&self) -> &tracedraw_core::Document {
        self.engine.document()
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

    pub fn active_layer(&self) -> Option<LayerId> {
        self.doc()
            .page(self.page)
            .ok()
            .and_then(|p| p.layers.iter().rev().find(|l| !l.locked))
            .map(|l| l.id)
    }

    pub fn selected_shapes(&self) -> Vec<Shape> {
        let doc = self.doc();
        self.selection
            .iter()
            .filter_map(|id| doc.shape(*id).ok().map(|(_, s)| s.clone()))
            .collect()
    }

    pub fn selection_bounds(&self) -> Option<Rect> {
        self.selected_shapes()
            .iter()
            .map(Shape::bounds)
            .reduce(|a, b| a.union(b))
    }

    pub fn run(&mut self, cmd: Command) {
        if let Err(e) = self.engine.run(&cmd) {
            self.status = format!("{}: {e}", cmd.label());
        }
    }

    pub fn set_tool(&mut self, tool: Tool) {
        if self.tool != tool {
            self.finish_curve();
            self.finish_text();
            self.previous_tool = self.tool;
            self.tool = tool;
            self.rotate_mode = false;
            self.flyout_open = None;
            if !tool.implemented() {
                self.status = format!("{} tool is not implemented yet", tool.name());
            }
        }
    }

    pub fn select(&mut self, ids: Vec<ShapeId>) {
        self.selection = ids;
        self.rotate_mode = false;
        self.node_selection.clear();
    }

    // ----- object creation ---------------------------------------------------

    pub fn new_shape(&mut self, kind: ShapeKind) -> Option<ShapeId> {
        let layer = self.active_layer()?;
        let id = self.engine.new_shape_id();
        let mut shape = Shape::new(id, kind);
        shape.fill = self.default_fill.clone();
        shape.stroke = self.default_stroke.clone();
        if matches!(shape.kind, ShapeKind::Text { .. }) {
            // the editor: text defaults to black fill and no outline.
            shape.fill = Fill::Solid(Color::BLACK);
            shape.stroke = None;
        }
        self.run(Command::AddShape { layer, shape });
        Some(id)
    }

    pub fn create_box_shape(&mut self, a: Point, b: Point) {
        let rect = Rect::from_points(a, b);
        if rect.width() < 0.05 || rect.height() < 0.05 {
            return;
        }
        let kind = match self.tool {
            Tool::Rectangle | Tool::ThreePointRectangle => ShapeKind::Rect {
                rect,
                radius: self.rect_radius,
            },
            Tool::Ellipse | Tool::ThreePointEllipse => ShapeKind::Ellipse {
                rect,
                arc: self.ellipse_arc,
            },
            Tool::Polygon => ShapeKind::Polygon {
                rect,
                points: self.polygon_points,
                sharpness: 0.0,
            },
            Tool::Star => ShapeKind::Polygon {
                rect,
                points: self.polygon_points,
                sharpness: self.star_sharpness,
            },
            _ => return,
        };
        if let Some(id) = self.new_shape(kind) {
            self.select(vec![id]);
        }
    }

    pub fn finish_curve(&mut self) {
        let Some(c) = self.curve.take() else { return };
        if c.nodes.len() < 2 {
            return;
        }
        let path = c.path(None);
        if let Some(id) = self.new_shape(ShapeKind::Path {
            path,
            closed: false,
        }) {
            self.select(vec![id]);
        }
    }

    /// Start artistic text at a point, or paragraph text in a frame.
    pub fn start_text(&mut self, at: Point, frame: Option<Size>) {
        self.finish_text();
        let span = TextSpan {
            text: String::new(),
            font_family: self.text_font.clone(),
            size_pt: self.text_size_pt,
            bold: self.text_bold,
            italic: self.text_italic,
        };
        if let Some(id) = self.new_shape(ShapeKind::Text {
            spans: vec![span],
            origin: at,
            frame,
            align: self.text_align,
        }) {
            self.text_edit = Some(TextEdit {
                shape: id,
                text: String::new(),
            });
            self.select(vec![id]);
        }
    }

    /// Property bar defaults follow the text object being edited.
    pub fn sync_text_defaults_from(&mut self, id: ShapeId) {
        let kind = self.doc().shape(id).map(|(_, s)| s.kind.clone());
        if let Ok(kind) = kind {
            if let ShapeKind::Text { spans, align, .. } = &kind {
                if let Some(sp) = spans.first() {
                    self.text_font = sp.font_family.clone();
                    self.text_size_pt = sp.size_pt;
                    self.text_bold = sp.bold;
                    self.text_italic = sp.italic;
                }
                self.text_align = *align;
            }
        }
    }

    pub fn update_text(&mut self) {
        let Some(te) = self.text_edit.clone() else {
            return;
        };
        if let Ok((_, s)) = self.doc().shape(te.shape) {
            if let ShapeKind::Text {
                spans,
                origin,
                frame,
                align,
            } = &s.kind
            {
                let mut spans = spans.clone();
                if let Some(first) = spans.first_mut() {
                    first.text = te.text.clone();
                }
                let kind = ShapeKind::Text {
                    spans,
                    origin: *origin,
                    frame: *frame,
                    align: *align,
                };
                // Typing is one undo step per text object; collapse by undoing the
                // previous keystroke entry when it was also a SetShapeKind on this shape.
                if self.engine.undo_label() == Some("Edit Text") {
                    let _ = self.engine.undo();
                }
                let _ = self.engine.run_with_label(
                    &Command::SetShapeKind {
                        shape: te.shape,
                        kind,
                    },
                    "Edit Text",
                );
            }
        }
    }

    pub fn finish_text(&mut self) {
        if let Some(te) = self.text_edit.take() {
            if te.text.trim().is_empty() {
                let _ = self.engine.run(&Command::DeleteShapes {
                    shapes: vec![te.shape],
                });
                self.selection.retain(|s| *s != te.shape);
            }
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
        if let Ok(p) = self.doc().page(self.page) {
            let ids = p
                .layers
                .iter()
                .filter(|l| !l.locked)
                .flat_map(|l| &l.shapes)
                .map(|s| s.id)
                .collect();
            self.select(ids);
        }
    }

    pub fn copy(&mut self) {
        let shapes = self.selected_shapes();
        if !shapes.is_empty() {
            self.clipboard = Some(Clipboard { shapes });
            self.status = format!("{} object(s) copied", self.selection.len());
        }
    }

    pub fn cut(&mut self) {
        self.copy();
        self.delete_selection();
    }

    pub fn paste(&mut self) {
        let Some(cb) = self.clipboard.clone() else {
            return;
        };
        let Some(layer) = self.active_layer() else {
            return;
        };
        let mut ids = Vec::new();
        let mut cmds = Vec::new();
        for s in cb.shapes {
            let id = self.engine.new_shape_id();
            let shape = reid(s, id, &mut self.engine);
            ids.push(id);
            cmds.push(Command::AddShape { layer, shape });
        }
        if let Err(e) = self.engine.run_batch("Paste", &cmds) {
            self.status = e.to_string();
        }
        self.select(ids);
    }

    pub fn duplicate(&mut self) {
        let shapes = self.selected_shapes();
        let Some(layer) = self.active_layer() else {
            return;
        };
        let mut ids = Vec::new();
        let mut cmds = Vec::new();
        for s in shapes {
            let id = self.engine.new_shape_id();
            let mut shape = reid(s, id, &mut self.engine);
            shape.transform = Affine::translate(self.duplicate_offset) * shape.transform;
            ids.push(id);
            cmds.push(Command::AddShape { layer, shape });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Duplicate", &cmds) {
            self.status = e.to_string();
        }
        self.select(ids);
    }

    pub fn group_selection(&mut self) {
        if self.selection.len() < 2 {
            return;
        }
        let shapes = std::mem::take(&mut self.selection);
        self.run(Command::Group { shapes });
        let doc = self.doc();
        if let Ok(p) = doc.page(self.page) {
            if let Some(g) = p
                .layers
                .iter()
                .flat_map(|l| &l.shapes)
                .filter(|s| matches!(s.kind, ShapeKind::Group { .. }))
                .max_by_key(|s| s.id)
            {
                self.selection = vec![g.id];
            }
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
        self.selection.clear();
        for g in groups {
            self.run(Command::Ungroup { group: g });
        }
    }

    pub fn convert_to_curves(&mut self) {
        let shapes = self.selected_shapes();
        let cmds: Vec<Command> = shapes
            .iter()
            .filter(|s| !matches!(s.kind, ShapeKind::Path { .. } | ShapeKind::Group { .. }))
            .map(|s| Command::SetShapeKind {
                shape: s.id,
                kind: ShapeKind::Path {
                    path: s.local_path(),
                    closed: true,
                },
            })
            .collect();
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Convert To Curves", &cmds) {
                self.status = e.to_string();
            }
        }
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
        if self.selection.is_empty() {
            self.default_fill = fill;
            self.status = "Default fill changed for new objects".into();
        } else {
            let shapes = self.selection.clone();
            self.run(Command::SetFill { shapes, fill });
        }
    }

    pub fn apply_outline_color(&mut self, color: Option<Color>) {
        let make = |existing: Option<&Stroke>| -> Option<Stroke> {
            color.map(|c| {
                let mut s = existing.cloned().unwrap_or_default();
                s.color = c;
                s
            })
        };
        if self.selection.is_empty() {
            self.default_stroke = make(self.default_stroke.as_ref());
        } else {
            let shapes = self.selected_shapes();
            let cmds: Vec<Command> = shapes
                .iter()
                .map(|s| Command::SetStroke {
                    shapes: vec![s.id],
                    stroke: make(s.stroke.as_ref()),
                })
                .collect();
            if let Err(e) = self.engine.run_batch("Outline Color", &cmds) {
                self.status = e.to_string();
            }
        }
    }

    pub fn apply_outline_width(&mut self, width: f64) {
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            if let Some(s) = &mut self.default_stroke {
                s.width = width;
            }
            return;
        }
        let cmds: Vec<Command> = shapes
            .iter()
            .map(|s| {
                let mut st = s.stroke.clone().unwrap_or_default();
                st.width = width;
                Command::SetStroke {
                    shapes: vec![s.id],
                    stroke: Some(st),
                }
            })
            .collect();
        if let Err(e) = self.engine.run_batch("Outline Width", &cmds) {
            self.status = e.to_string();
        }
    }

    pub fn transform_selection(&mut self, t: Affine) {
        if self.selection.is_empty() {
            return;
        }
        let shapes = self.selection.clone();
        self.run(Command::TransformShapes {
            shapes,
            transform: t,
        });
    }

    pub fn nudge(&mut self, d: Vec2) {
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
        self.finish_text();
        match self.engine.undo() {
            Ok(l) => self.status = format!("Undo {l}"),
            Err(e) => self.status = e.to_string(),
        }
        self.selection
            .retain(|id| self.engine.document().shape(*id).is_ok());
    }

    pub fn redo(&mut self) {
        match self.engine.redo() {
            Ok(l) => self.status = format!("Redo {l}"),
            Err(e) => self.status = e.to_string(),
        }
        self.selection
            .retain(|id| self.engine.document().shape(*id).is_ok());
    }

    // ----- files -------------------------------------------------------------

    pub fn new_document(&mut self) {
        let doc = tracedraw_core::Document::default();
        self.page = doc.pages[0].id;
        self.engine.replace(doc);
        self.selection.clear();
        self.file = None;
        self.fit_pending = true;
        self.status.clear();
    }

    pub fn open_dialog(&mut self) {
        let picked = rfd::FileDialog::new()
            .add_filter("All supported", &["cdr", "tdraw"])
            .add_filter("the editor (*.cdr)", &["cdr"])
            .add_filter("TraceDraw (*.tdraw)", &["tdraw"])
            .pick_file();
        if let Some(p) = picked {
            self.open_path(p);
        }
    }

    pub fn open_path(&mut self, path: PathBuf) {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let result = if ext == "cdr" {
            tracedraw_cdr::open(&path)
                .map(|(doc, report)| {
                    let ver = report.version.map(|v| v.name()).unwrap_or_default();
                    for w in &report.warnings {
                        log::warn!("cdr: {w}");
                    }
                    (
                        doc,
                        format!(
                            "{ver}: {} page(s), {} object(s), {} skipped",
                            report.pages, report.shapes, report.skipped_objects
                        ),
                    )
                })
                .map_err(|e| e.to_string())
        } else {
            tracedraw_io::load_native(&path)
                .map(|d| (d, "Opened".to_string()))
                .map_err(|e| e.to_string())
        };
        match result {
            Ok((doc, msg)) => {
                self.page = doc.pages[0].id;
                self.engine.replace(doc);
                self.selection.clear();
                self.file = if ext == "cdr" {
                    None
                } else {
                    Some(path.clone())
                };
                self.status = format!("{}: {msg}", path.display());
                self.fit_pending = true;
            }
            Err(e) => self.status = format!("Could not open {}: {e}", path.display()),
        }
    }

    pub fn save(&mut self, save_as: bool) {
        let path = if save_as || self.file.is_none() {
            rfd::FileDialog::new()
                .add_filter("TraceDraw (*.tdraw)", &["tdraw"])
                .set_file_name("Graphic1.tdraw")
                .save_file()
        } else {
            self.file.clone()
        };
        let Some(path) = path else { return };
        match tracedraw_io::save_native(self.engine.document(), &path) {
            Ok(()) => {
                self.engine.mark_saved();
                self.file = Some(path.clone());
                self.status = format!("Saved {}", path.display());
            }
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }

    pub fn export(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("SVG (*.svg)", &["svg"])
            .set_file_name("Graphic1.svg")
            .save_file()
        else {
            return;
        };
        let idx = self
            .doc()
            .pages
            .iter()
            .position(|p| p.id == self.page)
            .unwrap_or(0);
        match tracedraw_io::save_svg(self.engine.document(), idx, &path) {
            Ok(()) => self.status = format!("Exported {}", path.display()),
            Err(e) => self.status = format!("Export failed: {e}"),
        }
    }

    pub fn import(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "All importable",
                &[
                    "cdr", "png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff",
                ],
            )
            .add_filter("the editor (*.cdr)", &["cdr"])
            .add_filter(
                "Images",
                &["png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff"],
            )
            .pick_file()
        else {
            return;
        };
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "cdr" {
            self.import_bitmap(&path);
            return;
        }
        match tracedraw_cdr::open(&path) {
            Ok((doc, _)) => {
                let Some(layer) = self.active_layer() else {
                    return;
                };
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in doc
                    .pages
                    .iter()
                    .flat_map(|p| &p.layers)
                    .flat_map(|l| &l.shapes)
                {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s.clone(), id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                self.select(ids);
            }
            Err(e) => self.status = format!("Import failed: {e}"),
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
            let _ = self.engine.run_batch("Clear Transformations", &cmds);
        }
    }

    pub fn export_pdf(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF (*.pdf)", &["pdf"])
            .set_file_name("Graphic1.pdf")
            .save_file()
        else {
            return;
        };
        match tracedraw_io::save_pdf(self.engine.document(), &path) {
            Ok(()) => self.status = format!("Exported {}", path.display()),
            Err(e) => self.status = format!("Export failed: {e}"),
        }
    }

    // ----- view --------------------------------------------------------------

    pub fn zoom_to_page(&mut self) {
        self.fit_pending = true;
    }

    pub fn zoom_to_fit(&mut self) {
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

    pub fn zoom_percent(&self) -> f32 {
        // 100% = 96 dpi, as the editor shows it.
        self.view.zoom / (96.0 / 25.4) * 100.0
    }

    pub fn set_zoom_percent(&mut self, pct: f32) {
        let anchor: Pos2 = self.canvas_rect.center();
        let target = pct / 100.0 * (96.0 / 25.4);
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

    pub fn delete_page(&mut self) {
        let page = self.page;
        self.run(Command::DeletePage { page });
        if let Some(p) = self.doc().pages.first() {
            self.page = p.id;
        }
        self.selection.clear();
        self.fit_pending = true;
    }

    pub fn goto_page(&mut self, idx: usize) {
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
fn reid(mut s: Shape, id: ShapeId, engine: &mut Engine) -> Shape {
    s.id = id;
    if let ShapeKind::Group { children } = &mut s.kind {
        for c in children.iter_mut() {
            let nid = engine.new_shape_id();
            *c = reid(c.clone(), nid, engine);
        }
    }
    s
}

/// the default CMYK palette, top to bottom.
pub fn default_palette() -> Vec<Color> {
    let mut v = vec![
        Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
        Color::cmyk_pct(0.0, 0.0, 0.0, 90.0),
        Color::cmyk_pct(0.0, 0.0, 0.0, 80.0),
        Color::cmyk_pct(0.0, 0.0, 0.0, 70.0),
        Color::cmyk_pct(0.0, 0.0, 0.0, 60.0),
        Color::cmyk_pct(0.0, 0.0, 0.0, 50.0),
        Color::cmyk_pct(0.0, 0.0, 0.0, 40.0),
        Color::cmyk_pct(0.0, 0.0, 0.0, 30.0),
        Color::cmyk_pct(0.0, 0.0, 0.0, 20.0),
        Color::cmyk_pct(0.0, 0.0, 0.0, 10.0),
        Color::cmyk_pct(0.0, 0.0, 0.0, 0.0),
        Color::cmyk_pct(100.0, 0.0, 0.0, 0.0),
        Color::cmyk_pct(0.0, 100.0, 0.0, 0.0),
        Color::cmyk_pct(0.0, 0.0, 100.0, 0.0),
        Color::cmyk_pct(100.0, 100.0, 0.0, 0.0),
        Color::cmyk_pct(0.0, 100.0, 100.0, 0.0),
        Color::cmyk_pct(100.0, 0.0, 100.0, 0.0),
    ];
    // Tints and shades across the hue wheel, like the default palette's long run.
    for i in 0..24 {
        let h = i as f32 / 24.0;
        let [r, g, b] = hsv_to_rgb(h, 1.0, 1.0);
        v.push(Color::Rgb { r, g, b });
        let [r, g, b] = hsv_to_rgb(h, 0.55, 1.0);
        v.push(Color::Rgb { r, g, b });
        let [r, g, b] = hsv_to_rgb(h, 1.0, 0.55);
        v.push(Color::Rgb { r, g, b });
    }
    v
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    match (i as i32).rem_euclid(6) {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

pub fn fill_preview_color(fill: &Fill) -> Option<egui::Color32> {
    match fill {
        Fill::None => None,
        Fill::Solid(c) => Some(crate::canvas::to_color32(*c)),
        Fill::Linear { from, .. } | Fill::Radial { from, .. } => {
            Some(crate::canvas::to_color32(*from))
        }
    }
}

pub fn fill_description(fill: &Fill) -> String {
    match fill {
        Fill::None => "None".into(),
        Fill::Solid(c) => color_description(*c),
        Fill::Linear { .. } => "Linear fountain".into(),
        Fill::Radial { .. } => "Radial fountain".into(),
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
    }
}
