//! Visual tokens: the interface colours of the four themes (Options >
//! Customization > Appearance). Colours live here and nowhere else; the
//! theme in use is picked with [`set_theme`] and read by every token.

use egui::{Color32, Visuals};
use std::sync::atomic::{AtomicU8, Ordering};

/// The interface themes, lightest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    MediumLight,
    MediumDark,
    Dark,
    /// The desktop's high contrast colours (the classic black theme
    /// where the desktop has none).
    HighContrast,
}

impl Theme {
    pub const ALL: [Theme; 5] = [
        Theme::Light,
        Theme::MediumLight,
        Theme::MediumDark,
        Theme::Dark,
        Theme::HighContrast,
    ];

    /// The name kept in the preferences.
    pub fn id(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::MediumLight => "medium_light",
            Theme::MediumDark => "medium_dark",
            Theme::Dark => "dark",
            Theme::HighContrast => "high_contrast",
        }
    }

    /// The theme a preference names; Light for anything else.
    pub fn from_id(id: &str) -> Theme {
        Theme::ALL
            .into_iter()
            .find(|t| t.id() == id)
            .unwrap_or(Theme::Light)
    }

    /// The i18n key of its name.
    pub fn key(self) -> &'static str {
        match self {
            Theme::Light => "options.theme_light",
            Theme::MediumLight => "options.theme_medium_light",
            Theme::MediumDark => "options.theme_medium_dark",
            Theme::Dark => "options.theme_dark",
            Theme::HighContrast => "options.theme_high_contrast",
        }
    }

    /// Light text on dark panels.
    pub fn is_dark(self) -> bool {
        match self {
            Theme::MediumDark | Theme::Dark => true,
            Theme::HighContrast => contrast_colors().is_dark(),
            _ => false,
        }
    }

    /// The page under a tabbed dialog's tabs and its frame line in this
    /// theme: the colour of fields (white in the light themes), so the
    /// labels on it read in every theme (#904).
    pub fn dialog_page(self) -> (Color32, Color32) {
        (self.palette().field, self.palette().border)
    }

    fn palette(self) -> &'static Palette {
        match self {
            Theme::Light => &LIGHT,
            Theme::MediumLight => &MEDIUM_LIGHT,
            Theme::MediumDark => &MEDIUM_DARK,
            Theme::Dark => &DARK,
            Theme::HighContrast => contrast_palette(),
        }
    }
}

/// The high contrast colours: the desktop's when one of its high
/// contrast themes is on, else the classic black theme.
pub fn contrast_colors() -> crate::system_prefs::ContrastColors {
    crate::system_prefs::get()
        .high_contrast
        .unwrap_or(crate::system_prefs::ContrastColors::BLACK)
}

/// Text drawn on a hovered or picked row's highlight.
pub fn on_highlight() -> Color32 {
    if theme() == Theme::HighContrast {
        let [r, g, b] = contrast_colors().highlight_text;
        Color32::from_rgb(r, g, b)
    } else {
        Tokens::text()
    }
}

/// Links in panels (Hints and the like).
pub fn link_color() -> Color32 {
    match theme() {
        Theme::HighContrast => {
            let [r, g, b] = contrast_colors().hot_light;
            Color32::from_rgb(r, g, b)
        }
        t if t.is_dark() => Color32::from_rgb(0x5C, 0xB2, 0xFF),
        _ => Tokens::ACCENT,
    }
}

static CONTRAST: std::sync::OnceLock<Palette> = std::sync::OnceLock::new();

/// The high contrast theme's palette, made from [`contrast_colors`].
fn contrast_palette() -> &'static Palette {
    CONTRAST.get_or_init(|| {
        let c = contrast_colors();
        let col = |v: [u8; 3]| Color32::from_rgb(v[0], v[1], v[2]);
        let (win, text, hi, face, btn_text, gray, hot) = (
            col(c.window),
            col(c.window_text),
            col(c.highlight),
            col(c.button_face),
            col(c.button_text),
            col(c.gray_text),
            col(c.hot_light),
        );
        Palette {
            panel: face,
            menu_bar: face,
            panel_dark: win,
            border: text,
            icon_off: gray,
            text: btn_text,
            text_dim: gray,
            page_shadow: gray,
            page_border: text,
            ruler_bg: face,
            tool_active: hi,
            tool_hover: hi.gamma_multiply(0.6),
            icon: btn_text,
            button: face,
            control_border: text,
            slider_rail: gray,
            field: win,
            menu: win,
            menu_gutter: win,
            menu_hover: hi,
            menu_hover_line: hi,
            menu_text_off: gray,
            title_bar: hi,
            window_border: hi,
            selected_row: hi,
            hovered_row: hi.gamma_multiply(0.5),
            selected_row_line: hot,
            control_fill: win,
            tab_strip: face,
            tab: face,
            tab_active: win,
            ruler_tick: text,
            ruler_label: text,
            ruler_marker: hot,
            desktop: win,
        }
    })
}

/// The status bar's light / dark button: a light theme goes to Dark, a
/// dark one to Light.
pub fn toggled(t: Theme) -> Theme {
    if t.is_dark() {
        Theme::Light
    } else {
        Theme::Dark
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Use `theme` for the interface from now on (the tokens and
/// [`visuals`]).
pub fn set_theme(theme: Theme) {
    let i = Theme::ALL.iter().position(|t| *t == theme).unwrap_or(0);
    CURRENT.store(i as u8, Ordering::Relaxed);
}

static SHADOWS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Customization > Appearance > Shadows under menus and dialogs.
pub fn set_shadows(on: bool) {
    SHADOWS.store(on, Ordering::Relaxed);
}

/// Menus and dialogs cast shadows.
pub fn shadows() -> bool {
    SHADOWS.load(Ordering::Relaxed)
}

/// `s`, or no shadow when shadows are off.
pub fn shadow(s: egui::epaint::Shadow) -> egui::epaint::Shadow {
    if shadows() {
        s
    } else {
        egui::epaint::Shadow::NONE
    }
}

/// The theme in use.
pub fn theme() -> Theme {
    Theme::ALL
        .get(CURRENT.load(Ordering::Relaxed) as usize)
        .copied()
        .unwrap_or(Theme::Light)
}

const fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// One theme's interface colours.
struct Palette {
    panel: Color32,
    menu_bar: Color32,
    panel_dark: Color32,
    border: Color32,
    icon_off: Color32,
    text: Color32,
    text_dim: Color32,
    page_shadow: Color32,
    page_border: Color32,
    ruler_bg: Color32,
    tool_active: Color32,
    tool_hover: Color32,
    icon: Color32,
    button: Color32,
    control_border: Color32,
    slider_rail: Color32,
    field: Color32,
    menu: Color32,
    menu_gutter: Color32,
    menu_hover: Color32,
    menu_hover_line: Color32,
    menu_text_off: Color32,
    title_bar: Color32,
    window_border: Color32,
    selected_row: Color32,
    hovered_row: Color32,
    selected_row_line: Color32,
    control_fill: Color32,
    tab_strip: Color32,
    tab: Color32,
    tab_active: Color32,
    ruler_tick: Color32,
    ruler_label: Color32,
    ruler_marker: Color32,
    /// The desktop around the page, unless the user picked a colour.
    desktop: Color32,
}

const LIGHT: Palette = Palette {
    panel: rgb(0xF4F4F4),
    menu_bar: rgb(0xD8D8D8),
    panel_dark: rgb(0xE1E1E1),
    border: rgb(0xD8D8D8),
    icon_off: rgb(0xC8C8C8),
    text: rgb(0x1E1E1E),
    text_dim: rgb(0x6E6E6E),
    page_shadow: rgb(0xBFBFBF),
    page_border: rgb(0xAAAAAA),
    ruler_bg: rgb(0xF4F4F4),
    tool_active: rgb(0xCCE4F7),
    tool_hover: rgb(0xE5F1FB),
    icon: rgb(0x2A2A2A),
    button: rgb(0xE1E1E1),
    control_border: rgb(0xADADAD),
    slider_rail: rgb(0xC4C4C4),
    field: rgb(0xFFFFFF),
    menu: rgb(0xFFFFFF),
    menu_gutter: rgb(0xF4F4F4),
    menu_hover: rgb(0xE0F0FF),
    menu_hover_line: rgb(0x00ADFE),
    menu_text_off: rgb(0xC8C8C8),
    title_bar: rgb(0xFFFFFF),
    window_border: rgb(0xB2B2B2),
    selected_row: rgb(0xCCE8FF),
    hovered_row: rgb(0xE5F3FF),
    selected_row_line: rgb(0x99D1FF),
    control_fill: rgb(0xEAEAEA),
    tab_strip: rgb(0xEAEAEA),
    tab: rgb(0xD8D8D8),
    tab_active: rgb(0xCEE3FF),
    ruler_tick: rgb(0x8C8C8C),
    ruler_label: rgb(0x767676),
    ruler_marker: rgb(0x0B0B0B),
    desktop: rgb(0xFFFFFF),
};

const MEDIUM_LIGHT: Palette = Palette {
    panel: rgb(0xDCDCDC),
    menu_bar: rgb(0xC8C8C8),
    panel_dark: rgb(0xCDCDCD),
    border: rgb(0xBDBDBD),
    icon_off: rgb(0xA6A6A6),
    text: rgb(0x1A1A1A),
    text_dim: rgb(0x595959),
    page_shadow: rgb(0x9E9E9E),
    page_border: rgb(0x8C8C8C),
    ruler_bg: rgb(0xDCDCDC),
    tool_active: rgb(0xB5D3EC),
    tool_hover: rgb(0xCADFF0),
    icon: rgb(0x262626),
    button: rgb(0xCBCBCB),
    control_border: rgb(0x969696),
    slider_rail: rgb(0xAEAEAE),
    field: rgb(0xF2F2F2),
    menu: rgb(0xE4E4E4),
    menu_gutter: rgb(0xD6D6D6),
    menu_hover: rgb(0xB9D9F2),
    menu_hover_line: rgb(0x7DB8E6),
    menu_text_off: rgb(0x8F8F8F),
    title_bar: rgb(0xEDEDED),
    window_border: rgb(0x868686),
    selected_row: rgb(0xB9D9F2),
    hovered_row: rgb(0xD0E6F6),
    selected_row_line: rgb(0x7DB8E6),
    control_fill: rgb(0xD2D2D2),
    tab_strip: rgb(0xD0D0D0),
    tab: rgb(0xC0C0C0),
    tab_active: rgb(0xB5D3EC),
    ruler_tick: rgb(0x707070),
    ruler_label: rgb(0x5A5A5A),
    ruler_marker: rgb(0x0B0B0B),
    desktop: rgb(0xFFFFFF),
};

const MEDIUM_DARK: Palette = Palette {
    panel: rgb(0x535353),
    menu_bar: rgb(0x474747),
    panel_dark: rgb(0x4A4A4A),
    border: rgb(0x3D3D3D),
    icon_off: rgb(0x7C7C7C),
    text: rgb(0xEAEAEA),
    text_dim: rgb(0xB0B0B0),
    page_shadow: rgb(0x2E2E2E),
    page_border: rgb(0x2A2A2A),
    ruler_bg: rgb(0x535353),
    tool_active: rgb(0x2C5A80),
    tool_hover: rgb(0x3A566C),
    icon: rgb(0xE4E4E4),
    button: rgb(0x656565),
    control_border: rgb(0x7E7E7E),
    slider_rail: rgb(0x6E6E6E),
    field: rgb(0x3F3F3F),
    menu: rgb(0x4B4B4B),
    menu_gutter: rgb(0x444444),
    menu_hover: rgb(0x2C5A80),
    menu_hover_line: rgb(0x3E78A8),
    menu_text_off: rgb(0x8A8A8A),
    title_bar: rgb(0x474747),
    window_border: rgb(0x2E2E2E),
    selected_row: rgb(0x2C5A80),
    hovered_row: rgb(0x3A566C),
    selected_row_line: rgb(0x3E78A8),
    control_fill: rgb(0x5E5E5E),
    tab_strip: rgb(0x4A4A4A),
    tab: rgb(0x5C5C5C),
    tab_active: rgb(0x2C5A80),
    ruler_tick: rgb(0xA8A8A8),
    ruler_label: rgb(0xC0C0C0),
    ruler_marker: rgb(0xF0F0F0),
    desktop: rgb(0x5E5E5E),
};

const DARK: Palette = Palette {
    panel: rgb(0x2B2B2B),
    menu_bar: rgb(0x1F1F1F),
    panel_dark: rgb(0x252525),
    border: rgb(0x1A1A1A),
    icon_off: rgb(0x5A5A5A),
    text: rgb(0xE0E0E0),
    text_dim: rgb(0x9A9A9A),
    page_shadow: rgb(0x1A1A1A),
    page_border: rgb(0x111111),
    ruler_bg: rgb(0x2B2B2B),
    tool_active: rgb(0x2A5C8A),
    tool_hover: rgb(0x3A3A3A),
    icon: rgb(0xD8D8D8),
    button: rgb(0x3A3A3A),
    control_border: rgb(0x505050),
    slider_rail: rgb(0x4A4A4A),
    field: rgb(0x1E1E1E),
    menu: rgb(0x2B2B2B),
    menu_gutter: rgb(0x262626),
    menu_hover: rgb(0x2A5C8A),
    menu_hover_line: rgb(0x3D7AB8),
    menu_text_off: rgb(0x6A6A6A),
    title_bar: rgb(0x1F1F1F),
    window_border: rgb(0x0F0F0F),
    selected_row: rgb(0x2A5C8A),
    hovered_row: rgb(0x353535),
    selected_row_line: rgb(0x3D7AB8),
    control_fill: rgb(0x333333),
    tab_strip: rgb(0x252525),
    tab: rgb(0x333333),
    tab_active: rgb(0x2A5C8A),
    ruler_tick: rgb(0x8A8A8A),
    ruler_label: rgb(0xA8A8A8),
    ruler_marker: rgb(0xF0F0F0),
    desktop: rgb(0x3D3D3D),
};

pub struct Tokens;

macro_rules! tokens {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {
        impl Tokens {
            $(
                $(#[$doc])*
                pub fn $name() -> Color32 {
                    theme().palette().$name
                }
            )*
        }
    };
}

tokens!(
    /// Toolbars, dockers, palettes, the status bar and dialog bodies.
    panel,
    /// The menu bar.
    menu_bar,
    panel_dark,
    /// Light lines: panel separators, frames of lists and docker parts.
    border,
    /// Icons of commands that cannot run now.
    icon_off,
    text,
    text_dim,
    /// The page's drop shadow, offset right and down behind the page.
    page_shadow,
    page_border,
    ruler_bg,
    tool_active,
    tool_hover,
    icon,
    /// Push buttons.
    button,
    /// The border of buttons, check boxes and fields.
    control_border,
    /// A slider's groove.
    slider_rail,
    /// Fields, lists and other places to type or pick in (white when light).
    field,
    /// The body of an open menu.
    menu,
    /// The strip of a menu behind its icons and check marks.
    menu_gutter,
    /// The box behind a hovered menu row, and its frame.
    menu_hover,
    menu_hover_line,
    /// Menu rows that cannot run now.
    menu_text_off,
    /// Dialogs' title bars.
    title_bar,
    /// The frame of dialog windows.
    window_border,
    /// The picked row of a list, a hovered row, and the picked row's frame.
    selected_row,
    hovered_row,
    selected_row_line,
    /// Lists' and spin buttons' fill.
    control_fill,
    /// The document tab strip, its tabs and the tab in front.
    tab_strip,
    tab,
    tab_active,
    /// Ruler ticks, numbers and the pointer's marker.
    ruler_tick,
    ruler_label,
    ruler_marker,
    desktop,
);

impl Tokens {
    /// Accents, the page and the selection stay the same in every theme.
    pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0x78, 0xD7);
    pub const PAGE: Color32 = Color32::WHITE;
    /// Screen offset of the page shadow, pixels.
    pub const PAGE_SHADOW_OFFSET: egui::Vec2 = egui::vec2(6.0, 4.0);
    pub const HANDLE: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
    pub const SELECTION: Color32 = Color32::from_rgb(0x00, 0x78, 0xD7);
    /// The outline that shows where a dragged selection would go.
    pub const DRAG_OUTLINE: Color32 = Color32::from_rgb(0x1E, 0x64, 0xFF);

    pub const RULER: f32 = 18.0;

    /// The side of a toolbox button: 32 px at the medium size, scaled by
    /// the toolbox's button size (the bars' menu > Customize).
    pub fn tool_button() -> f32 {
        (32.0 * button_scale(&TOOLBOX_SCALE)).round()
    }

    /// The side of a standard toolbar button: 32 px at the medium size,
    /// scaled by the default button size.
    pub fn bar_button() -> f32 {
        (32.0 * button_scale(&BAR_SCALE)).round()
    }

    /// The side of a property bar button: 28 px at the medium size,
    /// scaled by the default button size.
    pub fn prop_button() -> f32 {
        (28.0 * button_scale(&BAR_SCALE)).round()
    }
}

/// The default button size's scale and the toolbox's, as f32 bits.
static BAR_SCALE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static TOOLBOX_SCALE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn button_scale(cell: &std::sync::atomic::AtomicU32) -> f32 {
    let bits = cell.load(Ordering::Relaxed);
    if bits == 0 {
        1.0
    } else {
        f32::from_bits(bits)
    }
}

/// Size the bars' buttons from now on: `bars` for every bar, `toolbox`
/// for the toolbox (the bars' menu > Customize > Default Button Size and
/// Toolbox Toolbar > Button Size).
pub fn set_button_sizes(
    bars: crate::settings::ButtonSize,
    toolbox: Option<crate::settings::ButtonSize>,
) {
    BAR_SCALE.store(bars.scale().to_bits(), Ordering::Relaxed);
    TOOLBOX_SCALE.store(toolbox.unwrap_or(bars).scale().to_bits(), Ordering::Relaxed);
}

pub fn visuals() -> Visuals {
    let mut v = if theme().is_dark() {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    // Indented groups have no guide line.
    v.indent_has_left_vline = false;
    if !shadows() {
        v.window_shadow = egui::epaint::Shadow::NONE;
        v.popup_shadow = egui::epaint::Shadow::NONE;
    }
    v.panel_fill = Tokens::panel();
    v.window_fill = Tokens::panel();
    v.extreme_bg_color = Tokens::field();
    v.faint_bg_color = Tokens::panel_dark();
    v.code_bg_color = Tokens::panel_dark();
    v.override_text_color = None;
    v.widgets.noninteractive.bg_fill = Tokens::panel();
    v.widgets.noninteractive.weak_bg_fill = Tokens::panel();
    v.widgets.noninteractive.fg_stroke.color = Tokens::text();
    v.widgets.noninteractive.bg_stroke.color = Tokens::border();
    // Framed controls look like the desktop's: check boxes, radio
    // buttons and fields in the field colour with a border, push buttons
    // a shade darker than the panel.
    v.widgets.inactive.bg_fill = Tokens::field();
    v.widgets.inactive.weak_bg_fill = Tokens::button();
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Tokens::control_border());
    v.widgets.inactive.fg_stroke.color = Tokens::text();
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Tokens::ACCENT);
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, Tokens::ACCENT);
    v.widgets.hovered.bg_fill = Tokens::tool_hover();
    v.widgets.hovered.weak_bg_fill = Tokens::tool_hover();
    v.widgets.hovered.fg_stroke.color = Tokens::text();
    v.widgets.active.bg_fill = Tokens::tool_active();
    v.widgets.active.weak_bg_fill = Tokens::tool_active();
    v.widgets.active.fg_stroke.color = Tokens::text();
    v.widgets.open.bg_fill = Tokens::tool_active();
    v.widgets.open.weak_bg_fill = Tokens::tool_active();
    v.widgets.open.fg_stroke.color = Tokens::text();
    v.selection.bg_fill = Tokens::tool_active();
    // Text on the picked fill: the accent on light themes, white on dark
    // ones (the accent would not read on the dark blue).
    v.selection.stroke.color = if theme().is_dark() {
        Color32::WHITE
    } else {
        Tokens::ACCENT
    };
    v.hyperlink_color = if theme().is_dark() {
        Color32::from_rgb(0x5C, 0xB2, 0xFF)
    } else {
        Tokens::ACCENT
    };
    if theme() == Theme::HighContrast {
        // Text on the highlight in the desktop's highlight text colour,
        // links and focus in its hot-track colour.
        let c = contrast_colors();
        let col = |v: [u8; 3]| Color32::from_rgb(v[0], v[1], v[2]);
        let on_hi = col(c.highlight_text);
        v.selection.stroke.color = on_hi;
        v.widgets.hovered.fg_stroke.color = on_hi;
        v.widgets.active.fg_stroke.color = on_hi;
        v.widgets.open.fg_stroke.color = on_hi;
        v.widgets.hovered.bg_stroke = egui::Stroke::new(2.0, col(c.hot_light));
        v.widgets.active.bg_stroke = egui::Stroke::new(2.0, col(c.hot_light));
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, col(c.window_text));
        v.hyperlink_color = col(c.hot_light);
        v.warn_fg_color = col(c.hot_light);
    }
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

/// Install the bundled interface fonts only (headless drawing).
#[cfg(test)]
pub fn install_fonts_bundled(ctx: &egui::Context) {
    ctx.set_fonts(bundled_font_definitions());
}

/// Install the interface fonts (see [`font_definitions`]).
pub fn install_fonts(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
}

/// The interface fonts: the bundled UI font first, then egui's own and
/// the system fonts covering the scripts of the supported UI languages
/// (CJK, Arabic, Devanagari, Bengali), so menus render in every language
/// on machines that have such fonts. Reads the system font list, which
/// can take a moment: the splash screen builds these on a worker thread.
pub fn font_definitions() -> egui::FontDefinitions {
    let mut defs = bundled_font_definitions();
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
    defs
}

/// The interface fonts without the system ones: the bundled UI font
/// (Latin, Greek, Cyrillic) and egui's own, ready at once.
pub fn bundled_font_definitions() -> egui::FontDefinitions {
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
    defs
}

/// True when the interface language `code` needs a system font for its
/// script (the bundled one covers Latin, Greek and Cyrillic).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn needs_system_font(code: &str) -> bool {
    matches!(code, "zh-CN" | "zh-TW" | "ja" | "ar" | "hi" | "bn")
}

/// The family of the font folder (browser build) that covers the script
/// of the interface language `code`, when no bundled font does.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn ui_script_family(code: &str) -> Option<&'static str> {
    match code {
        "zh-CN" => Some("Noto Sans SC"),
        "zh-TW" => Some("Noto Sans TC"),
        "ja" => Some("Noto Sans JP"),
        _ => None,
    }
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
        // Traditional Chinese.
        &[
            "Noto Sans CJK TC",
            "Noto Sans TC",
            "Microsoft JhengHei",
            "PingFang TC",
            "Source Han Sans TC",
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
        // A variable font draws at its default weight, Thin in some (the
        // browser build's Noto Sans CJK faces): set it to Regular.
        if fd.variation_axes().iter().any(|a| a.tag == *b"wght") {
            fd.tweak.coords = egui::epaint::text::VariationCoords::new([(b"wght", 400.0)]);
        }
        defs.font_data.insert(key.clone(), std::sync::Arc::new(fd));
        added.push(key);
    }
    if !added.is_empty() {
        log::info!("UI fallback fonts: {}", added.join(", "));
    }
    added
}

#[cfg(test)]
mod theme_tests {
    use super::*;

    /// WCAG contrast ratio of two opaque colours.
    fn contrast(a: Color32, b: Color32) -> f32 {
        let lum = |c: Color32| {
            let ch = |v: u8| {
                let v = v as f32 / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
        };
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Text drawn in a field (a list, the Glyphs panel's recent glyphs)
    /// reads in every theme (#660).
    #[test]
    fn text_contrasts_with_fields_in_every_theme() {
        for t in Theme::ALL {
            let p = t.palette();
            assert!(
                contrast(p.field, p.text) >= 4.5,
                "{t:?}: {:?} on {:?}",
                p.text,
                p.field
            );
        }
    }

    /// The labels on a tabbed dialog's page (Color Management, Document
    /// Properties) read in every theme: the page was white in the dark
    /// themes too, under their light text (#904).
    #[test]
    fn dialog_pages_carry_readable_text_in_every_theme() {
        for t in Theme::ALL {
            let (page, _) = t.dialog_page();
            let p = t.palette();
            assert!(
                contrast(page, p.text) >= 4.5,
                "{t:?}: {:?} on {page:?}",
                p.text
            );
            if t.is_dark() {
                assert_ne!(page, Color32::WHITE, "{t:?}");
            }
        }
    }

    #[test]
    fn themes_have_their_names_and_dark_ones_light_text() {
        let lum = |c: Color32| c.r() as u32 + c.g() as u32 + c.b() as u32;
        for t in Theme::ALL {
            assert_eq!(Theme::from_id(t.id()), t);
            let p = t.palette();
            // Text stands out from the panels in every theme.
            assert!(lum(p.text).abs_diff(lum(p.panel)) > 300, "{t:?}");
            assert_eq!(t.is_dark(), lum(p.text) > lum(p.panel), "{t:?}");
        }
        assert_eq!(Theme::from_id(""), Theme::Light);
        assert_eq!(Theme::from_id("purple"), Theme::Light);
        assert_eq!(toggled(Theme::MediumLight), Theme::Dark);
        assert_eq!(toggled(Theme::MediumDark), Theme::Light);
    }

    /// The browser build fetches a font for the interface languages no
    /// bundled font covers (#561).
    #[test]
    fn cjk_interface_languages_have_a_font_to_fetch() {
        for code in ["zh-CN", "zh-TW", "ja"] {
            assert!(needs_system_font(code));
            assert!(ui_script_family(code).is_some(), "{code}");
        }
        assert_eq!(ui_script_family("pt-BR"), None);
    }
}

#[cfg(test)]
mod shadow_tests {
    use super::*;

    #[test]
    fn shadows_can_be_turned_off() {
        let s = egui::epaint::Shadow {
            offset: [2, 2],
            blur: 4,
            spread: 0,
            color: Color32::BLACK,
        };
        set_shadows(false);
        assert_eq!(shadow(s), egui::epaint::Shadow::NONE);
        assert_eq!(visuals().window_shadow, egui::epaint::Shadow::NONE);
        set_shadows(true);
        assert_eq!(shadow(s), s);
    }
}
