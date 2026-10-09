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
    /// Text for the page at `index` (0-based).
    pub fn label(&self, index: usize) -> String {
        let n = self.start_at + index as i64;
        let body = match self.style {
            1 => roman(n),
            2 => roman(n).to_lowercase(),
            3 => alpha(n),
            4 => alpha(n).to_lowercase(),
            _ => n.to_string(),
        };
        format!("{}{}{}", self.prefix, body, self.suffix)
    }
}

fn roman(mut n: i64) -> String {
    if n <= 0 {
        return n.to_string();
    }
    const T: [(i64, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (v, s) in T {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

fn alpha(n: i64) -> String {
    if n <= 0 {
        return n.to_string();
    }
    // A..Z, then AA, AB... like spreadsheet columns.
    let mut n = n - 1;
    let mut out = Vec::new();
    loop {
        out.push((b'A' + (n % 26) as u8) as char);
        n /= 26;
        if n == 0 {
            break;
        }
        n -= 1;
    }
    out.iter().rev().collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Units {
    Millimeters,
    Centimeters,
    Inches,
    Points,
    Pixels,
}

impl Units {
    pub fn id(self) -> &'static str {
        match self {
            Units::Millimeters => "mm",
            Units::Centimeters => "cm",
            Units::Inches => "in",
            Units::Points => "pt",
            Units::Pixels => "px",
        }
    }
    pub fn from_id(id: &str) -> Self {
        match id {
            "cm" => Units::Centimeters,
            "in" => Units::Inches,
            "pt" => Units::Points,
            "px" => Units::Pixels,
            _ => Units::Millimeters,
        }
    }
    pub const ALL: [Units; 5] = [
        Units::Millimeters,
        Units::Centimeters,
        Units::Inches,
        Units::Points,
        Units::Pixels,
    ];
    pub fn label(self) -> String {
        crate::i18n::tr(match self {
            Units::Millimeters => "units.millimeters",
            Units::Centimeters => "units.centimeters",
            Units::Inches => "units.inches",
            Units::Points => "units.points",
            Units::Pixels => "units.pixels",
        })
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
    /// Base segment of a 3-point rectangle, ellipse or curve.
    ThreePointBase {
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
    // Defaults for new objects (the target design: no fill, black hairline).
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
    /// Layout > Page Number Settings.
    pub page_numbers: PageNumberSettings,
    /// Last loaded print-merge data, for Edit / Perform without reloading.
    pub merge_state: Option<crate::ui::dialogs::PrintMergeState>,
    pub show_welcome: bool,
    /// Object > ClipFrame > Place Inside Frame is waiting for a click on the frame.
    pub pending_clip_frame: bool,
    /// ClipFrame being edited in place: (frame id, content ids).
    pub clip_frame_edit: Option<(ShapeId, Vec<ShapeId>)>,
    pub show_guides: bool,
    pub selected_guide: Option<usize>,
    pub transform_values: [f64; 4],
    // ----- added with the full menu structure -----
    pub settings: crate::settings::Settings,
    pub welcome_tab: WelcomeTab,
    pub fullscreen_preview: bool,
    pub preview_selected_only: bool,
    pub page_sorter: bool,
    pub view_mode: ViewMode,
    pub proof_colors: bool,
    pub show_page_border: bool,
    pub simulate_overprints: bool,
    pub rasterize_complex_effects: bool,
    pub show_bleed: bool,
    pub show_printable_area: bool,
    pub show_pixel_grid: bool,
    pub show_baseline_grid: bool,
    pub options_page: crate::ui::dialogs::OptionsPage,
    pub pending_copy_properties: bool,
    pub pending_copy_effect: Option<EffectKind>,
    pub pending_clone_effect: Option<EffectKind>,
    /// Selected custom anchor index (Anchor Editing tool).
    pub anchor_sel: Option<usize>,
    /// Object > Order > In Front Of / Behind is waiting for a click: Some(in_front).
    pub pending_order: Option<bool>,
    pub last_repeatable: Option<Command>,
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
    pub scripts_output: Vec<String>,
    pub script_source: String,
    pub find_state: crate::ui::dialogs::FindReplaceState,
    pub step_repeat: crate::ops2::StepRepeat,
    pub align_to: crate::ops2::AlignTo,
    pub lens: crate::lens::LensSettings,
    pub envelope_mode: crate::effects_ui::EnvelopeMode,
    pub bevel: crate::effects_ui::BevelSettings,
    pub extrude: crate::effects_ui::ExtrudeSettings,
    pub transparency_default: crate::effects_ui::TransparencySettings,
    pub glyph_filter: String,
    pub table_edit: Option<crate::table::TableEdit>,
    pub missing_fonts: Vec<String>,
    pub bitmap_fx_amount: f32,
    pub last_bitmap_fx: Option<crate::bitmap_fx::Fx>,
    pub shaping_keep_source: bool,
    pub shaping_keep_target: bool,
    pub color_model: usize,
    pub mixer_color: Color,
    pub lens_synced_to: Option<ShapeId>,
    pub extrude_synced_to: Option<ShapeId>,
    pub pending_blend_path: bool,
    pub envelope_keep_lines: bool,
    pub media_mode: crate::media::MediaMode,
    pub media_preset: usize,
    pub media_spacing: f64,
    pub media_pressure: f64,
    pub media_smoothing: f64,
    pub mask_colors: Vec<Color>,
    pub mask_tolerance: u8,
    pub font_filter: String,
    /// Envelope/perspective node being dragged with the Shape tool: (shape, index).
    pub effect_node_drag: Option<(ShapeId, usize)>,
    /// Dockers added to the tab strip beyond the defaults.
    pub open_dockers: Vec<DockerTab>,
    pub free_transform_last: Point,
    pub roughen_amount: f64,
    /// Properties docker section to expand: 0 fill, 1 outline.
    pub properties_section: usize,
    pub area_fill_color: Color,
    pub mesh_rows: u32,
    pub mesh_cols: u32,
    /// Context menu > Frame Type > Text: next click/drag adds a text frame.
    pub pending_text_frame: bool,
    /// Shape tool elastic mode: dragging one node pulls its neighbours.
    pub elastic_mode: bool,
    /// Last clicked effect/mesh node (palette clicks colour a mesh node).
    pub selected_effect_node: Option<(ShapeId, usize)>,
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
}

impl Workspace {
    pub fn id(self) -> &'static str {
        match self {
            Workspace::Default => "default",
            Workspace::Lite => "lite",
            Workspace::Classic => "classic",
            Workspace::Illustration => "illustration",
            Workspace::PageLayout => "page_layout",
        }
    }
    pub fn from_id(id: &str) -> Self {
        match id {
            "lite" => Workspace::Lite,
            "classic" => Workspace::Classic,
            "illustration" => Workspace::Illustration,
            "page_layout" => Workspace::PageLayout,
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
}

impl DockerTab {
    pub const ALL: [DockerTab; 30] = [
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
        vec![
            DockerTab::Hints,
            DockerTab::Properties,
            DockerTab::Objects,
            DockerTab::Transformations,
            DockerTab::Undo,
        ]
    }
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
        theme::install_fallback_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(theme::visuals());
        cc.egui_ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(4.0, 3.0);
            style.spacing.button_padding = egui::vec2(5.0, 2.0);
        });
        Self::build(open)
    }

    /// An app without a window, for tests and scripts.
    #[cfg(test)]
    pub fn headless() -> Self {
        std::env::set_var(
            "TRACEDRAW_CONFIG_DIR",
            std::env::temp_dir().join("tracedraw-test"),
        );
        let mut app = Self::build(None);
        app.show_welcome = false;
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
            graph_rows: 4,
            graph_cols: 3,
            action_lines_count: 12,
            action_lines_radial: false,
            three_point_base: None,
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
            merge_state: None,
            page_numbers: PageNumberSettings::default(),
            show_welcome: false,
            pending_clip_frame: false,
            clip_frame_edit: None,
            show_guides: true,
            selected_guide: None,
            transform_values: [0.0, 0.0, 100.0, 100.0],
            settings: crate::settings::Settings::default(),
            welcome_tab: WelcomeTab::GetStarted,
            fullscreen_preview: false,
            preview_selected_only: false,
            page_sorter: false,
            view_mode: ViewMode::Enhanced,
            proof_colors: false,
            show_page_border: true,
            simulate_overprints: false,
            rasterize_complex_effects: true,
            show_bleed: false,
            show_printable_area: false,
            show_pixel_grid: false,
            show_baseline_grid: false,
            options_page: crate::ui::dialogs::OptionsPage::General,
            pending_copy_properties: false,
            pending_copy_effect: None,
            pending_clone_effect: None,
            anchor_sel: None,
            pending_order: None,
            last_repeatable: None,
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
            scripts_output: Vec::new(),
            script_source: String::new(),
            find_state: Default::default(),
            step_repeat: Default::default(),
            align_to: Default::default(),
            lens: Default::default(),
            envelope_mode: Default::default(),
            bevel: Default::default(),
            extrude: Default::default(),
            transparency_default: Default::default(),
            glyph_filter: String::new(),
            table_edit: None,
            missing_fonts: Vec::new(),
            bitmap_fx_amount: 50.0,
            last_bitmap_fx: None,
            shaping_keep_source: false,
            shaping_keep_target: false,
            color_model: 1,
            mixer_color: Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
            lens_synced_to: None,
            extrude_synced_to: None,
            pending_blend_path: false,
            envelope_keep_lines: false,
            media_mode: crate::media::MediaMode::Calligraphic,
            media_preset: 0,
            media_spacing: 8.0,
            media_pressure: 0.5,
            media_smoothing: 25.0,
            mask_colors: Vec::new(),
            mask_tolerance: 20,
            font_filter: String::new(),
            effect_node_drag: None,
            open_dockers: Vec::new(),
            free_transform_last: Point::ZERO,
            roughen_amount: 2.0,
            properties_section: 0,
            area_fill_color: Color::cmyk_pct(0.0, 0.0, 100.0, 0.0),
            mesh_rows: 2,
            mesh_cols: 2,
            pending_text_frame: false,
            elastic_mode: false,
            selected_effect_node: None,
        };
        app.load_settings();
        // Default names follow the UI language chosen by the settings.
        app.new_document();
        if let Some(p) = open {
            app.open_path(p);
        } else if app.settings.show_welcome_on_start {
            app.show_welcome = true;
        }
        app
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

    /// Default title for a new document.
    pub fn untitled_name() -> String {
        crate::i18n::trf("doc.untitled_n", &[("n", "1")])
    }

    /// Apply persisted settings (language, workspace, snapping, units).
    pub fn load_settings(&mut self) {
        let s = crate::settings::Settings::load();
        let lang = if s.language.is_empty() {
            crate::i18n::system_language().to_string()
        } else {
            s.language.clone()
        };
        crate::i18n::set_language(&lang);
        self.workspace = Workspace::from_id(&s.workspace);
        self.set_workspace(self.workspace);
        self.snap.grid = s.snap.grid;
        self.snap.guides = s.snap.guides;
        self.snap.objects = s.snap.objects;
        self.snap.page = s.snap.page;
        self.nudge_mm = s.nudge_mm;
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
            match std::fs::read(path).map_err(|e| e.to_string()) {
                Ok(bytes) => Profile::parse(&bytes).map_err(|e| e.to_string()).ok(),
                Err(_) => None,
            }
        };
        let rgb = load(&c.rgb_profile_path);
        let cmyk = load(&c.cmyk_profile_path);
        if rgb.is_none() && cmyk.is_none() {
            engine::clear();
            return;
        }
        let intent = Intent::from_name(&c.intent).unwrap_or_default();
        engine::install(IccEngine::new(
            rgb,
            cmyk,
            intent,
            c.black_point_compensation,
        ));
        self.raster.borrow_mut().invalidate();
    }

    /// Pick an `.icc`/`.icm` file for the RGB or CMYK slot.
    pub fn load_icc_profile(&mut self, cmyk: bool) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("ICC", &["icc", "icm"])
            .pick_file()
        else {
            return;
        };
        let bytes = match std::fs::read(&path) {
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

    pub fn save_settings(&mut self) {
        self.settings.workspace = self.workspace.id().into();
        self.settings.snap.grid = self.snap.grid;
        self.settings.snap.guides = self.snap.guides;
        self.settings.snap.objects = self.snap.objects;
        self.settings.snap.page = self.snap.page;
        self.settings.nudge_mm = self.nudge_mm;
        self.settings.duplicate_offset_mm = [self.duplicate_offset.x, self.duplicate_offset.y];
        self.settings.units = self.units.id().into();
        self.settings.language = crate::i18n::language();
        self.settings.save();
    }

    /// Tools > Save Settings as Default: persist current tool defaults.
    pub fn save_defaults(&mut self) {
        self.save_settings();
        self.status = crate::i18n::tr("status.settings_saved");
    }

    pub fn set_workspace(&mut self, ws: Workspace) {
        self.workspace = ws;
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
        }
    }

    pub fn document_title(&self) -> String {
        // Imported files (.cdr, .svg) have no native path yet; their title
        // comes from the document, like the target design shows it.
        let name = self
            .file
            .as_ref()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| {
                let t = self.doc().title.trim();
                if t.is_empty() || t == "Untitled" {
                    App::untitled_name()
                } else {
                    t.to_string()
                }
            });
        if self.engine.is_dirty() {
            format!("{name}*")
        } else {
            name
        }
    }

    /// File > Close: back to an empty document (one document per window).
    pub fn close_document(&mut self) {
        if self.engine.is_dirty() {
            self.dialog = crate::ui::dialogs::Dialog::ConfirmClose;
            return;
        }
        self.new_document();
        self.show_welcome = true;
    }

    pub fn save_as_template(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(crate::i18n::tr("file.template"), &["tdt"])
            .set_file_name("Template1.tdt")
            .save_file()
        else {
            return;
        };
        match tracedraw_io::save_native(self.engine.document(), &path) {
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

    /// Edit > Repeat: run the last repeatable command on the current selection.
    pub fn repeat_last(&mut self) {
        let Some(cmd) = self.last_repeatable.clone() else {
            return;
        };
        let shapes = self.selection.clone();
        let cmd = match cmd {
            Command::TransformShapes { transform, .. } => {
                Command::TransformShapes { shapes, transform }
            }
            Command::SetFill { fill, .. } => Command::SetFill { shapes, fill },
            Command::SetStroke { stroke, .. } => Command::SetStroke { shapes, stroke },
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
        self.break_clone_links_for(&cmd);
        if let Err(e) = self.engine.run(&cmd) {
            self.status = format!("{}: {e}", cmd.label());
            return;
        }
        self.sync_effect_clones_for(&cmd);
        if matches!(
            cmd,
            Command::TransformShapes { .. }
                | Command::SetFill { .. }
                | Command::SetStroke { .. }
                | Command::SetOpacity { .. }
        ) {
            self.last_repeatable = Some(cmd.clone());
        }
        if let Some(rec) = &mut self.recording {
            rec.push(cmd);
        }
    }

    /// Open the docker (or hide the column when it is already the active
    /// tab), as the docker shortcuts do.
    pub fn toggle_docker(&mut self, tab: DockerTab) {
        if self.show_dockers && self.docker_tab == tab {
            self.show_dockers = false;
        } else {
            self.show_dockers = true;
            self.docker_tab = tab;
        }
    }

    pub fn set_tool(&mut self, tool: Tool) {
        if self.tool != tool {
            self.finish_curve();
            self.finish_text();
            if self.table_edit.is_some() {
                self.table_commit_text();
                self.table_edit = None;
            }
            self.three_point_base = None;
            self.previous_tool = self.tool;
            self.tool = tool;
            self.rotate_mode = false;
            self.flyout_open = None;
            if !tool.implemented() {
                self.status =
                    crate::i18n::trf("status.tool_not_implemented", &[("t", &tool.name())]);
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
            // the target design: text defaults to black fill and no outline.
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
                    },
                );
                s.fill = self.default_fill.clone();
                s.stroke = self
                    .default_stroke
                    .clone()
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
        use tracedraw_core::geometry::BezPath;
        let n = self.action_lines_count.clamp(2, 500) as usize;
        let mut path = BezPath::new();
        // Deterministic pseudo-random lengths so the result is reproducible.
        let mut seed: u32 = 0x9E37_79B9 ^ n as u32;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed % 1000) as f64 / 1000.0
        };
        if self.action_lines_radial {
            let c = rect.center();
            let rmax = (rect.width().min(rect.height())) / 2.0;
            for i in 0..n {
                let a = std::f64::consts::TAU * i as f64 / n as f64;
                let dir = Vec2::new(a.cos(), a.sin());
                let r0 = rmax * (0.25 + 0.35 * rnd());
                let r1 = rmax * (0.75 + 0.25 * rnd());
                path.move_to(c + dir * r0);
                path.line_to(c + dir * r1);
            }
        } else {
            for i in 0..n {
                let y = rect.y0 + rect.height() * (i as f64 + 0.5) / n as f64;
                let len = rect.width() * (0.3 + 0.7 * rnd());
                let x0 = rect.x1 - len;
                path.move_to(Point::new(x0, y));
                path.line_to(Point::new(rect.x1, y));
            }
        }
        if let Some(id) = self.new_shape(ShapeKind::Path {
            path,
            closed: false,
        }) {
            let stroke = self
                .default_stroke
                .clone()
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
    /// height; the curve passes through `c`.
    pub fn finish_three_point(&mut self, a: Point, b: Point, c: Point) {
        use tracedraw_core::geometry::BezPath;
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
                    ShapeKind::Rect {
                        rect: local,
                        radius: self.rect_radius,
                    }
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
                // Quadratic through c at t = 0.5, written as a cubic.
                let ctrl = Point::new(2.0 * c.x - 0.5 * (a.x + b.x), 2.0 * c.y - 0.5 * (a.y + b.y));
                let c1 = a + (ctrl - a) * (2.0 / 3.0);
                let c2 = b + (ctrl - b) * (2.0 / 3.0);
                let mut path = BezPath::new();
                path.move_to(a);
                path.curve_to(c1, c2, b);
                if let Some(id) = self.new_shape(ShapeKind::Path {
                    path,
                    closed: false,
                }) {
                    self.run(Command::SetFill {
                        shapes: vec![id],
                        fill: Fill::None,
                    });
                    self.select(vec![id]);
                }
            }
            _ => {}
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
    /// Start editing an existing text object in place.
    pub fn begin_text_edit(&mut self, id: ShapeId) {
        let text = match self.doc().find_shape(id).map(|s| s.kind.clone()) {
            Some(ShapeKind::Text { spans, .. }) => {
                spans.iter().map(|s| s.text.as_str()).collect::<String>()
            }
            _ => return,
        };
        self.finish_text();
        self.text_edit = Some(TextEdit { shape: id, text });
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
        let Some(mut te) = self.text_edit.clone() else {
            return;
        };
        // Autocorrect acts once the word is finished (space or punctuation).
        if self.settings.autocorrect.enabled {
            let style = crate::autocorrect::QuoteStyle::for_language(&crate::i18n::language());
            if let Some(fixed) =
                crate::autocorrect::on_typed(&te.text, &self.settings.autocorrect, style)
            {
                te.text = fixed;
                self.text_edit = Some(te.clone());
            }
        }
        if let Ok((_, s)) = self.doc().shape(te.shape) {
            if let ShapeKind::Text {
                spans,
                origin,
                frame,
                align,
                para,
                on_path,
            } = &s.kind
            {
                let mut spans = spans.clone();
                // Typing edits the whole text as one run with the first span's style.
                let style = spans.first().cloned().unwrap_or_else(|| {
                    TextSpan::new("", self.text_font.clone(), self.text_size_pt)
                });
                spans = vec![TextSpan {
                    text: te.text.clone(),
                    ..style
                }];
                let mut para = para.clone();
                para.hyphenate = self.text_hyphenation;
                let kind = ShapeKind::Text {
                    spans,
                    origin: *origin,
                    frame: *frame,
                    align: *align,
                    para,
                    on_path: on_path.clone(),
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
                if self.is_linked_frame(te.shape) {
                    self.reflow_chain(te.shape);
                }
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
            self.status = crate::i18n::trf(
                "status.objects_copied",
                &[("n", &self.selection.len().to_string())],
            );
        }
    }

    pub fn cut(&mut self) {
        self.copy_with_system();
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
            self.status = crate::i18n::tr("status.default_fill_changed");
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
            shapes: shapes.clone(),
            transform: t,
        });
        self.compensate_unlocked_clip_frames(&shapes, t);
        self.apply_hinting(&shapes);
        self.reflow_chains_of(&shapes);
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
            Ok(l) => self.status = crate::i18n::trf("status.undo_n", &[("l", l)]),
            Err(e) => self.status = e.to_string(),
        }
        self.selection
            .retain(|id| self.engine.document().shape(*id).is_ok());
    }

    pub fn redo(&mut self) {
        match self.engine.redo() {
            Ok(l) => self.status = crate::i18n::trf("status.redo_n", &[("l", l)]),
            Err(e) => self.status = e.to_string(),
        }
        self.selection
            .retain(|id| self.engine.document().shape(*id).is_ok());
    }

    // ----- files -------------------------------------------------------------

    pub fn new_document(&mut self) {
        let doc =
            App::localized_document(App::untitled_name(), tracedraw_core::document::paper::A4);
        self.page = doc.pages[0].id;
        self.engine.replace(doc);
        self.selection.clear();
        self.file = None;
        self.fit_pending = true;
        self.status.clear();
    }

    pub fn open_dialog(&mut self) {
        let picked = rfd::FileDialog::new()
            .add_filter(
                crate::i18n::tr("file.all_supported"),
                &["cdr", "tdraw", "svg", "svgz", "pdf", "ai", "dxf"],
            )
            .add_filter("SVG (*.svg, *.svgz)", &["svg", "svgz"])
            .add_filter("PDF, AI (*.pdf, *.ai)", &["pdf", "ai"])
            .add_filter("DXF (*.dxf)", &["dxf"])
            .add_filter(crate::i18n::tr("file.cdr_files"), &["cdr"])
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
                    (doc, format!("SVG: {n} object(s)"))
                })
        } else if ext == "dxf" {
            std::fs::read(&path)
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
                    (doc2, format!("DXF: {n} object(s)"))
                })
        } else if ext == "pdf" || ext == "ai" {
            std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    tracedraw_io::pdf_import::parse(
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
                        let mut layer =
                            tracedraw_core::document::Layer::new(lid, layer_name.clone());
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
                    (doc2, format!("PDF: {pages} page(s), {n} object(s)"))
                })
        } else if ext == "cdr" {
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
                self.file = if matches!(ext.as_str(), "cdr" | "svg" | "svgz" | "pdf" | "ai" | "dxf")
                {
                    None
                } else {
                    Some(path.clone())
                };
                self.status = format!("{}: {msg}", path.display());
                self.fit_pending = true;
                self.settings.touch_recent(&path);
                self.settings.last_dir = path.parent().map(|p| p.to_path_buf());
                self.settings.save();
                self.show_welcome = false;
                self.check_missing_fonts();
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
                self.status =
                    crate::i18n::trf("status.saved", &[("p", &path.display().to_string())]);
                self.settings.touch_recent(&path);
                self.settings.save();
            }
            Err(e) => {
                self.status = crate::i18n::trf("status.save_failed", &[("e", &e.to_string())])
            }
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
            Ok(()) => {
                self.status =
                    crate::i18n::trf("status.exported", &[("path", &path.display().to_string())])
            }
            Err(e) => {
                self.status = crate::i18n::trf("status.export_failed", &[("e", &e.to_string())])
            }
        }
    }

    pub fn import(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                crate::i18n::tr("file.all_importable"),
                &[
                    "cdr", "svg", "svgz", "pdf", "ai", "dxf", "png", "jpg", "jpeg", "bmp", "gif",
                    "webp", "tif", "tiff",
                ],
            )
            .add_filter(crate::i18n::tr("file.cdr_files"), &["cdr"])
            .add_filter("SVG (*.svg, *.svgz)", &["svg", "svgz"])
            .add_filter("PDF, AI (*.pdf, *.ai)", &["pdf", "ai"])
            .add_filter("DXF (*.dxf)", &["dxf"])
            .add_filter(
                crate::i18n::tr("file.images"),
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
            Err(e) => {
                self.status = crate::i18n::trf("status.import_failed", &[("e", &e.to_string())])
            }
        }
    }

    /// Read an SVG file (plain or gzip-compressed) as text.
    fn read_svg_text(path: &std::path::Path) -> std::result::Result<String, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
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
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import the first page of a PDF or AI file into the active layer.
    pub fn import_pdf(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                tracedraw_io::pdf_import::parse(
                    &bytes,
                    &mut tracedraw_core::id::IdSource::default(),
                )
            });
        match parsed {
            Ok(imported) => {
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
        let parsed = std::fs::read(path)
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
            Ok(()) => {
                self.status =
                    crate::i18n::trf("status.exported", &[("path", &path.display().to_string())])
            }
            Err(e) => {
                self.status = crate::i18n::trf("status.export_failed", &[("e", &e.to_string())])
            }
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
        // 100% = 96 dpi, as the target design shows it.
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
/// Re-number a shape tree with fresh engine ids (for other modules).
pub fn reid_pub(s: Shape, id: ShapeId, engine: &mut Engine) -> Shape {
    reid(s, id, engine)
}

fn reid(mut s: Shape, id: ShapeId, engine: &mut Engine) -> Shape {
    s.id = id;
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

/// the target design's default CMYK palette, top to bottom.
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
    }
}
