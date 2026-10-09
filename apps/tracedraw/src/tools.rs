//! The toolbox: the 17 groups of the target design's 2019 toolbox, one
//! button per group, each group a flyout of related tools. Shortcuts
//! follow the target design and can be changed in Options > Shortcuts.

use egui::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    // Pick group
    Pick,
    FreeformPick,
    FreeTransform,
    // Edit shape group
    Shape,
    Smooth,
    Smear,
    Twirl,
    AttractRepel,
    Smudge,
    Roughen,
    // Crop group
    Crop,
    Knife,
    SegmentDelete,
    Eraser,
    // Zoom group
    Zoom,
    Pan,
    // Curve group
    Freehand,
    TwoPointLine,
    Bezier,
    Pen,
    BSpline,
    Polyline,
    ThreePointCurve,
    // Drawing group
    BrushStrokes,
    ShapeRecognition,
    Sketch,
    // Rectangle group
    Rectangle,
    ThreePointRectangle,
    // Ellipse group
    Ellipse,
    ThreePointEllipse,
    // Shape group
    Polygon,
    Star,
    Spiral,
    CommonShapes,
    ActionLines,
    GraphPaper,
    // Text group
    Text,
    Table,
    // Dimension group
    ParallelDimension,
    HorizontalVerticalDimension,
    AngularDimension,
    SegmentDimension,
    Callout,
    // Connector group
    Connector,
    RightAngleConnector,
    RoundedConnector,
    AnchorEditing,
    // Effect group
    DropShadow,
    Contour,
    Blend,
    Distort,
    Envelope,
    Extrude,
    BlockShadow,
    // Transparency
    Transparency,
    // Eyedropper group
    ColorEyedropper,
    AttributesEyedropper,
    // Fill group
    InteractiveFill,
    AreaFill,
    MeshFill,
    // Outline group (hidden by default)
    OutlinePen,
    OutlineColor,
}

pub struct ToolGroup {
    pub tools: &'static [Tool],
    /// Hidden from the toolbox unless enabled in Options (the Outline flyout).
    pub hidden: bool,
}

/// Toolbox order, top to bottom, exactly.
pub const GROUPS: &[ToolGroup] = &[
    ToolGroup {
        tools: &[Tool::Pick, Tool::FreeformPick, Tool::FreeTransform],
        hidden: false,
    },
    ToolGroup {
        tools: &[
            Tool::Shape,
            Tool::Smooth,
            Tool::Smear,
            Tool::Twirl,
            Tool::AttractRepel,
            Tool::Smudge,
            Tool::Roughen,
        ],
        hidden: false,
    },
    ToolGroup {
        tools: &[
            Tool::Crop,
            Tool::Knife,
            Tool::SegmentDelete,
            Tool::Eraser,
        ],
        hidden: false,
    },
    ToolGroup {
        tools: &[Tool::Zoom, Tool::Pan],
        hidden: false,
    },
    ToolGroup {
        tools: &[
            Tool::Freehand,
            Tool::TwoPointLine,
            Tool::Bezier,
            Tool::Pen,
            Tool::BSpline,
            Tool::Polyline,
            Tool::ThreePointCurve,
        ],
        hidden: false,
    },
    ToolGroup {
        tools: &[Tool::BrushStrokes, Tool::ShapeRecognition, Tool::Sketch],
        hidden: false,
    },
    ToolGroup {
        tools: &[Tool::Rectangle, Tool::ThreePointRectangle],
        hidden: false,
    },
    ToolGroup {
        tools: &[Tool::Ellipse, Tool::ThreePointEllipse],
        hidden: false,
    },
    ToolGroup {
        tools: &[
            Tool::Polygon,
            Tool::Star,
            Tool::Spiral,
            Tool::CommonShapes,
            Tool::ActionLines,
            Tool::GraphPaper,
        ],
        hidden: false,
    },
    ToolGroup {
        tools: &[Tool::Text, Tool::Table],
        hidden: false,
    },
    ToolGroup {
        tools: &[
            Tool::ParallelDimension,
            Tool::HorizontalVerticalDimension,
            Tool::AngularDimension,
            Tool::SegmentDimension,
            Tool::Callout,
        ],
        hidden: false,
    },
    ToolGroup {
        tools: &[
            Tool::Connector,
            Tool::RightAngleConnector,
            Tool::RoundedConnector,
            Tool::AnchorEditing,
        ],
        hidden: false,
    },
    ToolGroup {
        tools: &[
            Tool::DropShadow,
            Tool::Contour,
            Tool::Blend,
            Tool::Distort,
            Tool::Envelope,
            Tool::Extrude,
            Tool::BlockShadow,
        ],
        hidden: false,
    },
    ToolGroup {
        tools: &[Tool::Transparency],
        hidden: false,
    },
    ToolGroup {
        tools: &[Tool::ColorEyedropper, Tool::AttributesEyedropper],
        hidden: false,
    },
    ToolGroup {
        tools: &[Tool::InteractiveFill, Tool::AreaFill, Tool::MeshFill],
        hidden: false,
    },
    ToolGroup {
        tools: &[Tool::OutlinePen, Tool::OutlineColor],
        hidden: true,
    },
];

impl Tool {
    pub const ALL: [Tool; 62] = [
        Tool::Pick,
        Tool::FreeformPick,
        Tool::FreeTransform,
        Tool::Shape,
        Tool::Smooth,
        Tool::Smear,
        Tool::Twirl,
        Tool::AttractRepel,
        Tool::Smudge,
        Tool::Roughen,
        Tool::Crop,
        Tool::Knife,
        Tool::SegmentDelete,
        Tool::Eraser,
        Tool::Zoom,
        Tool::Pan,
        Tool::Freehand,
        Tool::TwoPointLine,
        Tool::Bezier,
        Tool::Pen,
        Tool::BSpline,
        Tool::Polyline,
        Tool::ThreePointCurve,
        Tool::BrushStrokes,
        Tool::ShapeRecognition,
        Tool::Sketch,
        Tool::Rectangle,
        Tool::ThreePointRectangle,
        Tool::Ellipse,
        Tool::ThreePointEllipse,
        Tool::Polygon,
        Tool::Star,
        Tool::Spiral,
        Tool::CommonShapes,
        Tool::ActionLines,
        Tool::GraphPaper,
        Tool::Text,
        Tool::Table,
        Tool::ParallelDimension,
        Tool::HorizontalVerticalDimension,
        Tool::AngularDimension,
        Tool::SegmentDimension,
        Tool::Callout,
        Tool::Connector,
        Tool::RightAngleConnector,
        Tool::RoundedConnector,
        Tool::AnchorEditing,
        Tool::DropShadow,
        Tool::Contour,
        Tool::Blend,
        Tool::Distort,
        Tool::Envelope,
        Tool::Extrude,
        Tool::BlockShadow,
        Tool::Transparency,
        Tool::ColorEyedropper,
        Tool::AttributesEyedropper,
        Tool::InteractiveFill,
        Tool::AreaFill,
        Tool::MeshFill,
        Tool::OutlinePen,
        Tool::OutlineColor,
    ];

    /// Stable identifier (settings, scripts, i18n key suffix).
    pub fn id(self) -> &'static str {
        match self {
            Tool::Pick => "pick",
            Tool::FreeformPick => "freehand_pick",
            Tool::FreeTransform => "free_transform",
            Tool::Shape => "shape",
            Tool::Smooth => "smooth",
            Tool::Smear => "smear",
            Tool::Twirl => "twirl",
            Tool::AttractRepel => "attract_repel",
            Tool::Smudge => "smudge",
            Tool::Roughen => "roughen",
            Tool::Crop => "crop",
            Tool::Knife => "knife",
            Tool::SegmentDelete => "segment_delete",
            Tool::Eraser => "eraser",
            Tool::Zoom => "zoom",
            Tool::Pan => "pan",
            Tool::Freehand => "freehand",
            Tool::TwoPointLine => "two_point_line",
            Tool::Bezier => "bezier",
            Tool::Pen => "pen",
            Tool::BSpline => "bspline",
            Tool::Polyline => "polyline",
            Tool::ThreePointCurve => "three_point_curve",
            Tool::BrushStrokes => "brush_strokes",
            Tool::ShapeRecognition => "shape_recognition",
            Tool::Sketch => "sketch",
            Tool::Rectangle => "rectangle",
            Tool::ThreePointRectangle => "three_point_rectangle",
            Tool::Ellipse => "ellipse",
            Tool::ThreePointEllipse => "three_point_ellipse",
            Tool::Polygon => "polygon",
            Tool::Star => "star",
            Tool::Spiral => "spiral",
            Tool::CommonShapes => "common_shapes",
            Tool::ActionLines => "action_lines",
            Tool::GraphPaper => "graph_paper",
            Tool::Text => "text",
            Tool::Table => "table",
            Tool::ParallelDimension => "parallel_dimension",
            Tool::HorizontalVerticalDimension => "hv_dimension",
            Tool::AngularDimension => "angular_dimension",
            Tool::SegmentDimension => "segment_dimension",
            Tool::Callout => "callout",
            Tool::Connector => "connector",
            Tool::RightAngleConnector => "right_angle_connector",
            Tool::RoundedConnector => "rounded_connector",
            Tool::AnchorEditing => "anchor_editing",
            Tool::DropShadow => "drop_shadow",
            Tool::Contour => "contour",
            Tool::Blend => "blend",
            Tool::Distort => "distort",
            Tool::Envelope => "envelope",
            Tool::Extrude => "extrude",
            Tool::BlockShadow => "block_shadow",
            Tool::Transparency => "transparency",
            Tool::ColorEyedropper => "color_eyedropper",
            Tool::AttributesEyedropper => "attributes_eyedropper",
            Tool::InteractiveFill => "interactive_fill",
            Tool::AreaFill => "area_fill",
            Tool::MeshFill => "mesh_fill",
            Tool::OutlinePen => "outline_pen",
            Tool::OutlineColor => "outline_color",
        }
    }

    /// Translated display name.
    pub fn name(self) -> String {
        crate::i18n::tr(&format!("tool.{}", self.id()))
    }

    /// the target design's default shortcut, if any.
    pub fn shortcut(self) -> Option<&'static str> {
        Some(match self {
            Tool::Pick => "Space",
            Tool::Shape => "F10",
            Tool::Zoom => "Z",
            Tool::Pan => "H",
            Tool::Freehand => "F5",
            Tool::Rectangle => "F6",
            Tool::Ellipse => "F7",
            Tool::Polygon => "Y",
            Tool::Text => "F8",
            Tool::InteractiveFill => "G",
            Tool::MeshFill => "M",
            Tool::Eraser => "X",
            Tool::Spiral => "A",
            Tool::Crop => "C",
            Tool::Bezier => "B",
            Tool::GraphPaper => "D",
            Tool::Smear => "W",
            Tool::Smudge => "V",
            Tool::Roughen => "E",
            Tool::ShapeRecognition => "Shift+S",
            Tool::Sketch => "S",
            Tool::BrushStrokes => "I",
            Tool::OutlinePen => "F12",
            Tool::OutlineColor => "Shift+F12",
            _ => return None,
        })
    }

    pub fn shortcut_label(self) -> &'static str {
        self.shortcut().unwrap_or("")
    }

    /// Parse a shortcut label like "Shift+F12" into (key, shift).
    pub fn parse_shortcut(label: &str) -> Option<(Key, bool)> {
        let shift = label.to_lowercase().contains("shift+");
        let key = label.rsplit('+').next()?.trim();
        let k = match key.to_ascii_uppercase().as_str() {
            "SPACE" => Key::Space,
            "ENTER" | "RETURN" => Key::Enter,
            "HOME" => Key::Home,
            "END" => Key::End,
            "PGUP" | "PAGEUP" => Key::PageUp,
            "PGDN" | "PAGEDOWN" => Key::PageDown,
            "F1" => Key::F1,
            "F2" => Key::F2,
            "F3" => Key::F3,
            "F4" => Key::F4,
            "F5" => Key::F5,
            "F6" => Key::F6,
            "F7" => Key::F7,
            "F8" => Key::F8,
            "F9" => Key::F9,
            "F10" => Key::F10,
            "F11" => Key::F11,
            "F12" => Key::F12,
            "A" => Key::A,
            "B" => Key::B,
            "C" => Key::C,
            "D" => Key::D,
            "E" => Key::E,
            "F" => Key::F,
            "G" => Key::G,
            "H" => Key::H,
            "I" => Key::I,
            "J" => Key::J,
            "K" => Key::K,
            "L" => Key::L,
            "M" => Key::M,
            "N" => Key::N,
            "O" => Key::O,
            "P" => Key::P,
            "Q" => Key::Q,
            "R" => Key::R,
            "S" => Key::S,
            "T" => Key::T,
            "U" => Key::U,
            "V" => Key::V,
            "W" => Key::W,
            "X" => Key::X,
            "Y" => Key::Y,
            "Z" => Key::Z,
            _ => return None,
        };
        Some((k, shift))
    }

    /// Tools that draw by dragging a bounding box.
    pub fn is_box_tool(self) -> bool {
        matches!(
            self,
            Tool::Rectangle
                | Tool::Ellipse
                | Tool::Polygon
                | Tool::Star
                | Tool::GraphPaper
                | Tool::ActionLines
        )
    }

    /// Tools that are listed but not implemented yet (shown greyed).
    pub fn implemented(self) -> bool {
        true
    }
}
