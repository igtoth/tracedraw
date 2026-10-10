//! Visual tokens: a light workspace in the spirit of the target design's default
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
    pub const PAGE: Color32 = Color32::WHITE;
    /// The page's drop shadow, offset right and down behind the page.
    pub const PAGE_SHADOW: Color32 = Color32::from_rgb(0xBF, 0xBF, 0xBF);
    pub const PAGE_BORDER: Color32 = Color32::from_rgb(0xAA, 0xAA, 0xAA);
    /// Screen offset of the page shadow, pixels.
    pub const PAGE_SHADOW_OFFSET: egui::Vec2 = egui::vec2(6.0, 4.0);
    pub const RULER_BG: Color32 = Color32::from_rgb(0xF4, 0xF4, 0xF4);
    pub const HANDLE: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
    pub const SELECTION: Color32 = Color32::from_rgb(0x00, 0x78, 0xD7);
    pub const TOOL_ACTIVE: Color32 = Color32::from_rgb(0xCC, 0xE4, 0xF7);
    pub const TOOL_HOVER: Color32 = Color32::from_rgb(0xE5, 0xF1, 0xFB);
    pub const ICON: Color32 = Color32::from_rgb(0x2A, 0x2A, 0x2A);
    /// Push buttons and their border, check boxes and fields.
    pub const BUTTON: Color32 = Color32::from_rgb(0xE1, 0xE1, 0xE1);
    pub const CONTROL_BORDER: Color32 = Color32::from_rgb(0xAD, 0xAD, 0xAD);

    pub const TOOLBOX_WIDTH: f32 = 38.0;
    pub const TOOL_BUTTON: f32 = 32.0;
    pub const RULER: f32 = 18.0;
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
    // Framed controls look like the desktop's: white check boxes, radio
    // buttons and fields with a grey border, light grey push buttons.
    v.widgets.inactive.bg_fill = Color32::WHITE;
    v.widgets.inactive.weak_bg_fill = Tokens::BUTTON;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Tokens::CONTROL_BORDER);
    v.widgets.inactive.fg_stroke.color = Tokens::TEXT;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Tokens::ACCENT);
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, Tokens::ACCENT);
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

/// The interface font, bundled (SIL Open Font License, see
/// `assets/fonts/Selawik-OFL.txt`): regular for text, bold for headings and
/// emphasis. Scripts it lacks fall back to egui's fonts and the system's.
const UI_REGULAR: &[u8] = include_bytes!("../assets/fonts/Selawik-Regular.ttf");
const UI_BOLD: &[u8] = include_bytes!("../assets/fonts/Selawik-Bold.ttf");

/// The font family name of the bold interface font.
pub const BOLD: &str = "ui-bold";

/// A bold interface font of the given size.
pub fn bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name(BOLD.into()))
}

/// Install the interface fonts: the bundled UI font first, then egui's
/// own and the system fonts covering the scripts of the supported UI
/// languages (CJK, Arabic, Devanagari, Bengali), so menus render in every
/// language on machines that have such fonts.
pub fn install_fonts(ctx: &egui::Context) {
    let mut defs = egui::FontDefinitions::default();
    defs.font_data.insert(
        "ui-regular".into(),
        std::sync::Arc::new(egui::FontData::from_static(UI_REGULAR)),
    );
    defs.font_data.insert(
        "ui-bold".into(),
        std::sync::Arc::new(egui::FontData::from_static(UI_BOLD)),
    );
    let defaults = defs
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some(list) = defs.families.get_mut(&egui::FontFamily::Proportional) {
        list.insert(0, "ui-regular".into());
    }
    let mut bold = vec!["ui-bold".to_string()];
    bold.extend(defaults);
    defs.families
        .insert(egui::FontFamily::Name(BOLD.into()), bold);
    for key in fallback_fonts(&mut defs) {
        for family in [
            egui::FontFamily::Proportional,
            egui::FontFamily::Monospace,
            egui::FontFamily::Name(BOLD.into()),
        ] {
            if let Some(list) = defs.families.get_mut(&family) {
                list.push(key.clone());
            }
        }
    }
    ctx.set_fonts(defs);
}

/// A context with the interface fonts installed (headless tests draw
/// the real interface, which uses the bold family).
#[cfg(test)]
pub fn ui_context() -> egui::Context {
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    ctx
}

/// Add the system fonts covering the non-Latin UI languages to `defs`;
/// returns their keys.
fn fallback_fonts(defs: &mut egui::FontDefinitions) -> Vec<String> {
    // Per script, the families to try in order; the first installed one wins.
    let groups: &[&[&str]] = &[
        // Arabic (also covered by many Latin system fonts).
        &[
            "Noto Sans Arabic",
            "Noto Naskh Arabic",
            "Segoe UI",
            "Arial",
            "DejaVu Sans",
            "Geeza Pro",
        ],
        // Devanagari (Hindi).
        &[
            "Noto Sans Devanagari",
            "Nirmala UI",
            "Mangal",
            "Kohinoor Devanagari",
            "Lohit Devanagari",
        ],
        // Bengali.
        &[
            "Noto Sans Bengali",
            "Nirmala UI",
            "Vrinda",
            "Kohinoor Bangla",
            "Lohit Bengali",
        ],
        // Simplified Chinese and Japanese.
        &[
            "Noto Sans CJK SC",
            "Noto Sans SC",
            "Microsoft YaHei",
            "PingFang SC",
            "Source Han Sans SC",
            "WenQuanYi Micro Hei",
            "Noto Sans CJK JP",
        ],
        &[
            "Noto Sans CJK JP",
            "Noto Sans JP",
            "Meiryo",
            "Yu Gothic UI",
            "Hiragino Sans",
            "Source Han Sans",
        ],
        // Cyrillic is in the bundled font; Latin extended too.
    ];
    let fs = tracedraw_text::fonts();
    let mut added = Vec::new();
    for group in groups {
        let Some((name, data, index)) = fs.first_face_data(group) else {
            continue;
        };
        let key = format!("fallback-{name}");
        if defs.font_data.contains_key(&key) {
            continue;
        }
        let mut fd = egui::FontData::from_owned(data.as_ref().clone());
        fd.index = index;
        defs.font_data.insert(key.clone(), std::sync::Arc::new(fd));
        added.push(key);
    }
    if !added.is_empty() {
        log::info!("UI fallback fonts: {}", added.join(", "));
    }
    added
}
