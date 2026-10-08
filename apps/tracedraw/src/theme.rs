//! Visual tokens: a light workspace in the spirit of the default
//! theme. Colours live here and nowhere else.

use egui::{Color32, Visuals};

pub struct Tokens;

impl Tokens {
    pub const PANEL: Color32 = Color32::from_rgb(0xF0, 0xF0, 0xF0);
    pub const PANEL_DARK: Color32 = Color32::from_rgb(0xE1, 0xE1, 0xE1);
    pub const BORDER: Color32 = Color32::from_rgb(0xC8, 0xC8, 0xC8);
    pub const TEXT: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(0x6E, 0x6E, 0x6E);
    pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0x78, 0xD7);
    pub const DESKTOP: Color32 = Color32::from_rgb(0xD4, 0xD4, 0xD4);
    pub const PAGE: Color32 = Color32::WHITE;
    pub const PAGE_SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 70);
    pub const PAGE_BORDER: Color32 = Color32::from_rgb(0x80, 0x80, 0x80);
    pub const RULER_BG: Color32 = Color32::from_rgb(0xF7, 0xF7, 0xF7);
    pub const RULER_TICK: Color32 = Color32::from_rgb(0x50, 0x50, 0x50);
    pub const HANDLE: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
    pub const SELECTION: Color32 = Color32::from_rgb(0x00, 0x78, 0xD7);
    pub const TOOL_ACTIVE: Color32 = Color32::from_rgb(0xCC, 0xE4, 0xF7);
    pub const TOOL_HOVER: Color32 = Color32::from_rgb(0xE5, 0xF1, 0xFB);
    pub const ICON: Color32 = Color32::from_rgb(0x2A, 0x2A, 0x2A);

    pub const TOOLBOX_WIDTH: f32 = 36.0;
    pub const TOOL_BUTTON: f32 = 30.0;
    pub const RULER: f32 = 18.0;
    pub const SCROLLBAR: f32 = 14.0;
    pub const SWATCH: f32 = 16.0;
}

pub fn visuals() -> Visuals {
    let mut v = Visuals::light();
    v.panel_fill = Tokens::PANEL;
    v.window_fill = Tokens::PANEL;
    v.extreme_bg_color = Color32::WHITE;
    v.faint_bg_color = Tokens::PANEL_DARK;
    v.widgets.noninteractive.bg_fill = Tokens::PANEL;
    v.widgets.noninteractive.fg_stroke.color = Tokens::TEXT;
    v.widgets.noninteractive.bg_stroke.color = Tokens::BORDER;
    v.widgets.inactive.bg_fill = Tokens::PANEL;
    v.widgets.inactive.weak_bg_fill = Tokens::PANEL;
    v.widgets.inactive.fg_stroke.color = Tokens::TEXT;
    v.widgets.hovered.bg_fill = Tokens::TOOL_HOVER;
    v.widgets.hovered.weak_bg_fill = Tokens::TOOL_HOVER;
    v.widgets.hovered.fg_stroke.color = Tokens::TEXT;
    v.widgets.active.bg_fill = Tokens::TOOL_ACTIVE;
    v.widgets.active.weak_bg_fill = Tokens::TOOL_ACTIVE;
    v.widgets.active.fg_stroke.color = Tokens::TEXT;
    v.selection.bg_fill = Tokens::TOOL_ACTIVE;
    v.selection.stroke.color = Tokens::ACCENT;
    v.hyperlink_color = Tokens::ACCENT;
    v.widgets.noninteractive.corner_radius = 2.0.into();
    v.widgets.inactive.corner_radius = 2.0.into();
    v.widgets.hovered.corner_radius = 2.0.into();
    v.widgets.active.corner_radius = 2.0.into();
    v.window_corner_radius = 2.0.into();
    v.menu_corner_radius = 2.0.into();
    v
}
