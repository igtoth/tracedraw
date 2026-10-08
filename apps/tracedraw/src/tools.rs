//! The toolbox, laid out the way the editor lays it out: one button per
//! group, each group a flyout of related tools. Shortcuts follow the editor.

use egui::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    // Pick group
    Pick,
    FreeformPick,
    // Shape group
    Shape,
    Smooth,
    Smear,
    Twirl,
    // Crop group
    Crop,
    Knife,
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
    // Brush strokes
    BrushStrokes,
    // Rectangle group
    Rectangle,
    ThreePointRectangle,
    // Ellipse group
    Ellipse,
    ThreePointEllipse,
    // Polygon group
    Polygon,
    Star,
    Spiral,
    CommonShapes,
    // Text group
    Text,
    Table,
    // Dimension
    ParallelDimension,
    // Connector
    Connector,
    // Effects group
    DropShadow,
    Contour,
    Blend,
    Distort,
    Envelope,
    Extrude,
    // Transparency
    Transparency,
    // Eyedropper group
    ColorEyedropper,
    AttributesEyedropper,
    // Fill group
    InteractiveFill,
    MeshFill,
    AreaFill,
}

pub struct ToolGroup {
    pub tools: &'static [Tool],
}

/// Toolbox order, top to bottom, exactly as in the editor.
pub const GROUPS: &[ToolGroup] = &[
    ToolGroup {
        tools: &[Tool::Pick, Tool::FreeformPick],
    },
    ToolGroup {
        tools: &[Tool::Shape, Tool::Smooth, Tool::Smear, Tool::Twirl],
    },
    ToolGroup {
        tools: &[Tool::Crop, Tool::Knife, Tool::Eraser],
    },
    ToolGroup {
        tools: &[Tool::Zoom, Tool::Pan],
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
    },
    ToolGroup {
        tools: &[Tool::BrushStrokes],
    },
    ToolGroup {
        tools: &[Tool::Rectangle, Tool::ThreePointRectangle],
    },
    ToolGroup {
        tools: &[Tool::Ellipse, Tool::ThreePointEllipse],
    },
    ToolGroup {
        tools: &[Tool::Polygon, Tool::Star, Tool::Spiral, Tool::CommonShapes],
    },
    ToolGroup {
        tools: &[Tool::Text, Tool::Table],
    },
    ToolGroup {
        tools: &[Tool::ParallelDimension],
    },
    ToolGroup {
        tools: &[Tool::Connector],
    },
    ToolGroup {
        tools: &[
            Tool::DropShadow,
            Tool::Contour,
            Tool::Blend,
            Tool::Distort,
            Tool::Envelope,
            Tool::Extrude,
        ],
    },
    ToolGroup {
        tools: &[Tool::Transparency],
    },
    ToolGroup {
        tools: &[Tool::ColorEyedropper, Tool::AttributesEyedropper],
    },
    ToolGroup {
        tools: &[Tool::InteractiveFill, Tool::MeshFill, Tool::AreaFill],
    },
];

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Tool::Pick => "Pick",
            Tool::FreeformPick => "Freehand Pick",
            Tool::Shape => "Shape",
            Tool::Smooth => "Smooth",
            Tool::Smear => "Smear",
            Tool::Twirl => "Twirl",
            Tool::Crop => "Crop",
            Tool::Knife => "Knife",
            Tool::Eraser => "Eraser",
            Tool::Zoom => "Zoom",
            Tool::Pan => "Pan",
            Tool::Freehand => "Freehand",
            Tool::TwoPointLine => "2-Point Line",
            Tool::Bezier => "Bezier",
            Tool::Pen => "Pen",
            Tool::BSpline => "B-Spline",
            Tool::Polyline => "Polyline",
            Tool::ThreePointCurve => "3-Point Curve",
            Tool::BrushStrokes => "Brush Strokes",
            Tool::Rectangle => "Rectangle",
            Tool::ThreePointRectangle => "3-Point Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::ThreePointEllipse => "3-Point Ellipse",
            Tool::Polygon => "Polygon",
            Tool::Star => "Star",
            Tool::Spiral => "Spiral",
            Tool::CommonShapes => "Common Shapes",
            Tool::Text => "Text",
            Tool::Table => "Table",
            Tool::ParallelDimension => "Parallel Dimension",
            Tool::Connector => "Straight-Line Connector",
            Tool::DropShadow => "Drop Shadow",
            Tool::Contour => "Contour",
            Tool::Blend => "Blend",
            Tool::Distort => "Distort",
            Tool::Envelope => "Envelope",
            Tool::Extrude => "Extrude",
            Tool::Transparency => "Transparency",
            Tool::ColorEyedropper => "Color Eyedropper",
            Tool::AttributesEyedropper => "Attributes Eyedropper",
            Tool::InteractiveFill => "Interactive Fill",
            Tool::MeshFill => "Mesh Fill",
            Tool::AreaFill => "Area Fill",
        }
    }

    /// the default shortcut, if any.
    pub fn shortcut(self) -> Option<(Key, bool)> {
        // (key, needs_shift)
        Some(match self {
            Tool::Pick => (Key::Space, false),
            Tool::Shape => (Key::F10, false),
            Tool::Zoom => (Key::Z, false),
            Tool::Pan => (Key::H, false),
            Tool::Freehand => (Key::F5, false),
            Tool::Rectangle => (Key::F6, false),
            Tool::Ellipse => (Key::F7, false),
            Tool::Polygon => (Key::Y, false),
            Tool::Text => (Key::F8, false),
            Tool::InteractiveFill => (Key::G, false),
            Tool::MeshFill => (Key::M, false),
            Tool::Eraser => (Key::X, false),
            Tool::Spiral => (Key::A, false),
            Tool::Crop => (Key::C, false),
            Tool::Bezier => (Key::B, false),
            _ => return None,
        })
    }

    pub fn shortcut_label(self) -> &'static str {
        match self {
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
            _ => "",
        }
    }

    /// Tools that draw by dragging a bounding box.
    pub fn is_box_tool(self) -> bool {
        matches!(
            self,
            Tool::Rectangle
                | Tool::Ellipse
                | Tool::Polygon
                | Tool::Star
                | Tool::ThreePointRectangle
                | Tool::ThreePointEllipse
                | Tool::Table
        )
    }

    /// Tools that are listed but not implemented yet.
    pub fn implemented(self) -> bool {
        matches!(
            self,
            Tool::Pick
                | Tool::Shape
                | Tool::Zoom
                | Tool::Pan
                | Tool::Freehand
                | Tool::TwoPointLine
                | Tool::Bezier
                | Tool::Polyline
                | Tool::Rectangle
                | Tool::Ellipse
                | Tool::Polygon
                | Tool::Star
                | Tool::Text
                | Tool::ColorEyedropper
                | Tool::InteractiveFill
                | Tool::Eraser
                | Tool::Transparency
                | Tool::DropShadow
                | Tool::Contour
                | Tool::Crop
                | Tool::Knife
                | Tool::Spiral
                | Tool::CommonShapes
                | Tool::Table
                | Tool::BrushStrokes
                | Tool::ParallelDimension
                | Tool::Connector
                | Tool::Blend
                | Tool::Extrude
                | Tool::Distort
                | Tool::Smooth
                | Tool::Smear
                | Tool::Twirl
                | Tool::FreeformPick
                | Tool::BSpline
                | Tool::ThreePointRectangle
                | Tool::ThreePointEllipse
                | Tool::ThreePointCurve
        )
    }
}
