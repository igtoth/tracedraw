//! Accessibility settings of the system that the interface follows
//! (#1412): the text size and high contrast.
//!
//! - Windows: "Make text bigger" (100 to 225 %) and the high contrast
//!   themes with their colours, read with the system's `reg` tool.
//! - macOS: "Increase contrast" (Accessibility > Display), asked of
//!   AppKit through `osascript`; macOS has no system text size for
//!   programs of this kind, so the text size option is explained instead.
//! - Linux: GNOME's (and its relatives') text scaling factor and high
//!   contrast through `gsettings`, KDE's font resolution or font size from
//!   its settings files, and the desktop portal's contrast setting
//!   (`org.freedesktop.appearance contrast`) through `gdbus` wherever a
//!   portal runs.
//! - Android: the system's font scale and its contrast setting, through
//!   the activity (`android::accessibility`).
//! - The browser: the page's default font size (the browser's font size
//!   setting) and `prefers-contrast: more` or `forced-colors: active`.
//!
//! Nothing here waits in a frame: a background thread (a timer in the
//! browser) reads the settings at start and every few seconds after, and
//! a frame only takes the last reading ([`get`]); a change asks for a
//! frame, so the interface follows at once.

// Each platform reads with its own parsers (the registry's, AppKit's,
// gsettings', KDE's, the activity's); the others' are used by the tests.
#![cfg_attr(not(test), allow(dead_code))]

use std::sync::{Mutex, OnceLock};

/// High contrast colours, as the desktop names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContrastColors {
    pub window: [u8; 3],
    pub window_text: [u8; 3],
    pub highlight: [u8; 3],
    pub highlight_text: [u8; 3],
    pub button_face: [u8; 3],
    pub button_text: [u8; 3],
    pub gray_text: [u8; 3],
    pub hot_light: [u8; 3],
}

impl ContrastColors {
    /// The classic black theme: white text, cyan links, a yellow-on-blue
    /// highlight. Used when the high contrast theme is picked by hand or
    /// its colours cannot be read.
    pub const BLACK: ContrastColors = ContrastColors {
        window: [0, 0, 0],
        window_text: [255, 255, 255],
        highlight: [26, 235, 255],
        highlight_text: [0, 0, 0],
        button_face: [0, 0, 0],
        button_text: [255, 255, 255],
        gray_text: [63, 242, 63],
        hot_light: [255, 255, 0],
    };

    /// The white theme: black text on white, a purple highlight, dark
    /// blue links. Used on systems whose contrast setting names no
    /// colours while they show light windows.
    pub const WHITE: ContrastColors = ContrastColors {
        window: [255, 255, 255],
        window_text: [0, 0, 0],
        highlight: [55, 0, 110],
        highlight_text: [255, 255, 255],
        button_face: [255, 255, 255],
        button_text: [0, 0, 0],
        gray_text: [96, 0, 0],
        hot_light: [0, 0, 159],
    };

    /// The black or the white theme, for a system that shows dark or
    /// light windows.
    pub fn for_dark(dark: bool) -> ContrastColors {
        if dark {
            Self::BLACK
        } else {
            Self::WHITE
        }
    }

    /// The window is dark (light text on it).
    pub fn is_dark(&self) -> bool {
        let [r, g, b] = self.window;
        (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) < 128.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemPrefs {
    /// Text size factor, 1.0 to 2.25.
    pub text_scale: f32,
    /// The colours of the high contrast theme in use, if one is on.
    pub high_contrast: Option<ContrastColors>,
    /// The system's text size could be read (else the option says why
    /// it does nothing).
    pub text_scale_known: bool,
    /// The system's contrast setting could be read.
    pub contrast_known: bool,
}

impl Default for SystemPrefs {
    fn default() -> Self {
        SystemPrefs {
            text_scale: 1.0,
            high_contrast: None,
            text_scale_known: false,
            contrast_known: false,
        }
    }
}

/// The last reading.
static PREFS: Mutex<Option<SystemPrefs>> = Mutex::new(None);

/// Signalled when the watcher's first reading is in.
#[cfg(not(target_arch = "wasm32"))]
static READ: std::sync::Condvar = std::sync::Condvar::new();

/// Whether [`watch`] started the watcher.
static WATCHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The context woken when a reading changes.
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

/// How often the settings are read again, in seconds.
pub const EVERY_SECS: u64 = 3;

/// How long the first frame waits for the watcher's first reading at
/// most, in milliseconds; the defaults apply until it comes.
#[cfg(not(target_arch = "wasm32"))]
const FIRST_WAIT_MS: u64 = 300;

/// The system's settings as last read. Before the first reading: a
/// moment's wait for the watcher's ([`watch`]), else one reading now.
/// Tests see the defaults (no test depends on the system running it).
pub fn get() -> SystemPrefs {
    let Ok(mut last) = PREFS.lock() else {
        return SystemPrefs::default();
    };
    if let Some(p) = *last {
        return p;
    }
    if cfg!(test) {
        return SystemPrefs::default();
    }
    #[cfg(not(target_arch = "wasm32"))]
    if WATCHING.load(std::sync::atomic::Ordering::Acquire) {
        let wait = std::time::Duration::from_millis(FIRST_WAIT_MS);
        return match READ.wait_timeout_while(last, wait, |p| p.is_none()) {
            Ok((p, _)) => p.unwrap_or_default(),
            Err(_) => SystemPrefs::default(),
        };
    }
    let p = read_now();
    *last = Some(p);
    p
}

/// A new reading: kept, and a frame asked for when it changed.
fn update(p: SystemPrefs) {
    let changed = match PREFS.lock() {
        Ok(mut last) => last.replace(p) != Some(p),
        Err(_) => false,
    };
    #[cfg(not(target_arch = "wasm32"))]
    READ.notify_all();
    if changed {
        log::info!(
            "system text size {:.0} %, high contrast {}",
            p.text_scale * 100.0,
            if p.high_contrast.is_some() {
                "on"
            } else {
                "off"
            }
        );
        if let Some(ctx) = CONTEXT.get() {
            ctx.request_repaint();
        }
    }
}

/// Follow the system's settings from now on: read at once and again
/// every [`EVERY_SECS`] seconds off the interface's thread (on a timer in
/// the browser), waking `ctx` when they change. Called once at start.
pub fn watch(ctx: &egui::Context) {
    if CONTEXT.set(ctx.clone()).is_err() {
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let spawned = std::thread::Builder::new()
            .name("system settings".into())
            .spawn(|| loop {
                update(read_now());
                std::thread::sleep(std::time::Duration::from_secs(EVERY_SECS));
            });
        match spawned {
            Ok(_) => WATCHING.store(true, std::sync::atomic::Ordering::Release),
            Err(e) => log::warn!("the system's settings are read at start only: {e}"),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        WATCHING.store(true, std::sync::atomic::Ordering::Release);
        crate::web::every(EVERY_SECS as i32 * 1000, || update(read_now()));
    }
}

/// A REG_DWORD value from `reg query` output ("    Name    REG_DWORD
/// 0x96").
pub fn parse_dword(output: &str, name: &str) -> Option<u32> {
    output.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        if !parts.next()?.eq_ignore_ascii_case(name) {
            return None;
        }
        if parts.next()? != "REG_DWORD" {
            return None;
        }
        let v = parts.next()?;
        let hex = v.strip_prefix("0x").or_else(|| v.strip_prefix("0X"))?;
        u32::from_str_radix(hex, 16).ok()
    })
}

/// A REG_SZ value from `reg query` output (the rest of the line).
pub fn parse_sz(output: &str, name: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let t = line.trim_start();
        let rest = t.strip_prefix(name)?;
        let rest = rest.trim_start();
        let rest = rest.strip_prefix("REG_SZ")?;
        Some(rest.trim().to_string())
    })
}

/// "R G B" as the desktop stores a colour.
pub fn parse_rgb(v: &str) -> Option<[u8; 3]> {
    let mut it = v.split_whitespace().map(|p| p.parse::<u8>().ok());
    let c = [it.next()??, it.next()??, it.next()??];
    it.next().is_none().then_some(c)
}

/// The text scale factor from `TextScaleFactor` (100 to 225; anything
/// else is 100).
pub fn text_scale_from(output: &str) -> f32 {
    match parse_dword(output, "TextScaleFactor") {
        Some(v) if (100..=225).contains(&v) => v as f32 / 100.0,
        _ => 1.0,
    }
}

/// High contrast is on when bit 1 of the `Flags` value is set (the
/// value is a string of digits).
pub fn high_contrast_on(output: &str) -> bool {
    parse_sz(output, "Flags")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .is_some_and(|f| f & 1 == 1)
}

/// The colours from the `Control Panel\Colors` key, falling back to the
/// classic black theme for any that is missing.
pub fn colors_from(output: &str) -> ContrastColors {
    let d = ContrastColors::BLACK;
    let c = |name: &str, fallback: [u8; 3]| {
        parse_sz(output, name)
            .and_then(|v| parse_rgb(&v))
            .unwrap_or(fallback)
    };
    ContrastColors {
        window: c("Window", d.window),
        window_text: c("WindowText", d.window_text),
        highlight: c("Hilight", d.highlight),
        highlight_text: c("HilightText", d.highlight_text),
        button_face: c("ButtonFace", d.button_face),
        button_text: c("ButtonText", d.button_text),
        gray_text: c("GrayText", d.gray_text),
        hot_light: c("HotTrackingColor", d.hot_light),
    }
}

#[cfg(windows)]
fn reg_query(key: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    // No console window flashes up.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("reg")
        .args(["query", key])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

#[cfg(windows)]
fn read_now() -> SystemPrefs {
    let text = reg_query(r"HKCU\Software\Microsoft\Accessibility");
    let text_scale = text.as_deref().map(text_scale_from).unwrap_or(1.0);
    let contrast = reg_query(r"HKCU\Control Panel\Accessibility\HighContrast");
    let high_contrast = contrast
        .as_deref()
        .filter(|o| high_contrast_on(o))
        .map(|_| {
            reg_query(r"HKCU\Control Panel\Colors")
                .map(|o| colors_from(&o))
                .unwrap_or(ContrastColors::BLACK)
        });
    SystemPrefs {
        text_scale,
        high_contrast,
        // The value is missing while the text size was never changed.
        text_scale_known: true,
        contrast_known: contrast.is_some(),
    }
}

/// The output of `program` with `args`, when it ran and succeeded.
#[cfg(not(any(windows, target_arch = "wasm32", target_os = "android")))]
fn output_of(program: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The JavaScript for `osascript` that asks AppKit whether "Increase
/// contrast" is on and whether the windows are dark: "contrast dark",
/// "normal light" and so on.
pub const MACOS_SCRIPT: &str = "ObjC.import('AppKit'); \
var w = $.NSWorkspace.sharedWorkspace; \
var style = ObjC.unwrap($.NSUserDefaults.standardUserDefaults.stringForKey('AppleInterfaceStyle')); \
(w.accessibilityDisplayShouldIncreaseContrast ? 'contrast' : 'normal') + ' ' + \
(style === 'Dark' ? 'dark' : 'light')";

/// What [`MACOS_SCRIPT`] printed: (contrast on, dark windows); `None`
/// for anything else.
pub fn parse_macos(output: &str) -> Option<(bool, bool)> {
    let mut words = output.split_whitespace();
    let contrast = match words.next()? {
        "contrast" => true,
        "normal" => false,
        _ => return None,
    };
    let dark = match words.next()? {
        "dark" => true,
        "light" => false,
        _ => return None,
    };
    Some((contrast, dark))
}

#[cfg(target_os = "macos")]
fn read_now() -> SystemPrefs {
    let read = output_of("osascript", &["-l", "JavaScript", "-e", MACOS_SCRIPT])
        .and_then(|o| parse_macos(&o));
    SystemPrefs {
        text_scale: 1.0,
        high_contrast: read
            .filter(|(contrast, _)| *contrast)
            .map(|(_, dark)| ContrastColors::for_dark(dark)),
        // macOS has no system text size for programs of this kind.
        text_scale_known: false,
        contrast_known: read.is_some(),
    }
}

/// A `gsettings get` answer: `1.25`, `true`, `'HighContrast'`, `uint32 1`.
pub fn gsettings_value(output: &str) -> &str {
    let v = output.trim();
    let v = v.strip_prefix("uint32 ").unwrap_or(v);
    v.trim_matches('\'')
}

/// GNOME's text scaling factor (`1.25`); `None` when it is not a number.
pub fn gnome_text_scale(output: &str) -> Option<f32> {
    gsettings_value(output)
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
}

/// A GNOME theme name that is a high contrast one (before GNOME 42 high
/// contrast was a theme: `HighContrast`, `HighContrastInverse`).
pub fn gnome_contrast_theme(output: &str) -> bool {
    gsettings_value(output).starts_with("HighContrast")
}

/// The value the desktop portal's `Settings.Read` gave through `gdbus`
/// for a number: `(<<uint32 1>>,)`, `(<uint32 1>,)`.
pub fn portal_number(output: &str) -> Option<u32> {
    let inner = output.trim().trim_start_matches(['(', '<']);
    let inner = inner.strip_prefix("uint32 ").unwrap_or(inner);
    let digits: String = inner.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// A `key=value` of the `[General]` group of a KDE settings file.
pub fn kde_general<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let mut general = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            general = line == "[General]";
            continue;
        }
        if !general {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == key {
                return Some(v.trim());
            }
        }
    }
    None
}

/// KDE's text size: the forced font resolution of `kcmfonts` (dpi over
/// 96), else the general font's size in `kdeglobals` over KDE's default
/// of 10 points; `None` when neither says.
pub fn kde_text_scale(kcmfonts: &str, kdeglobals: &str) -> Option<f32> {
    if let Some(dpi) = kde_general(kcmfonts, "forceFontDPI").and_then(|v| v.parse::<f32>().ok()) {
        if dpi > 0.0 {
            return Some(dpi / 96.0);
        }
    }
    let font = kde_general(kdeglobals, "font")?;
    let size = font.split(',').nth(1)?.trim().parse::<f32>().ok()?;
    (size > 0.0).then_some(size / 10.0)
}

/// KDE's colour scheme is one of its high contrast ones.
pub fn kde_contrast_scheme(kdeglobals: &str) -> bool {
    kde_general(kdeglobals, "ColorScheme").is_some_and(|s| {
        let s = s.to_ascii_lowercase().replace(' ', "");
        s.contains("highcontrast")
    })
}

/// The desktop's settings on Linux and the other free systems.
#[cfg(not(any(
    windows,
    target_os = "macos",
    target_arch = "wasm32",
    target_os = "android"
)))]
fn read_now() -> SystemPrefs {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let kde = desktop.split(':').any(|d| d.eq_ignore_ascii_case("KDE"));
    let gsettings = |schema: &str, key: &str| output_of("gsettings", &["get", schema, key]);
    let portal = |key: &str| {
        output_of(
            "gdbus",
            &[
                "call",
                "--session",
                "--timeout",
                "1",
                "--dest",
                "org.freedesktop.portal.Desktop",
                "--object-path",
                "/org/freedesktop/portal/desktop",
                "--method",
                "org.freedesktop.portal.Settings.Read",
                "org.freedesktop.appearance",
                key,
            ],
        )
        .and_then(|o| portal_number(&o))
    };
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")));
    let kde_file = |name: &str| {
        config
            .as_ref()
            .and_then(|c| std::fs::read_to_string(c.join(name)).ok())
            .unwrap_or_default()
    };
    let (kcmfonts, kdeglobals) = if kde {
        (kde_file("kcmfonts"), kde_file("kdeglobals"))
    } else {
        (String::new(), String::new())
    };
    let text = if kde {
        kde_text_scale(&kcmfonts, &kdeglobals)
    } else {
        gsettings("org.gnome.desktop.interface", "text-scaling-factor")
            .and_then(|o| gnome_text_scale(&o))
    };
    // The portal's contrast (1: higher contrast), else the desktop's own.
    let portal_contrast = portal("contrast");
    let own_contrast = if kde {
        Some(kde_contrast_scheme(&kdeglobals))
    } else {
        let a11y = gsettings("org.gnome.desktop.a11y.interface", "high-contrast")
            .map(|o| gsettings_value(&o) == "true");
        let theme =
            gsettings("org.gnome.desktop.interface", "gtk-theme").map(|o| gnome_contrast_theme(&o));
        match (a11y, theme) {
            (None, None) => None,
            (a, t) => Some(a.unwrap_or(false) || t.unwrap_or(false)),
        }
    };
    let contrast = match (portal_contrast, own_contrast) {
        (None, None) => None,
        (p, o) => Some(p == Some(1) || o == Some(true)),
    };
    // The portal's colour scheme: 1 prefers dark.
    let dark = portal("color-scheme") == Some(1);
    SystemPrefs {
        text_scale: text.unwrap_or(1.0),
        high_contrast: (contrast == Some(true)).then(|| ContrastColors::for_dark(dark)),
        text_scale_known: text.is_some(),
        contrast_known: contrast.is_some(),
    }
}

/// What the Android activity answered: "font-scale contrast dark", such
/// as "1.3 1 0".
pub fn parse_android(answer: &str) -> Option<SystemPrefs> {
    let mut words = answer.split_whitespace();
    let scale = words
        .next()?
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)?;
    let contrast = words.next()? == "1";
    let dark = words.next()? == "1";
    Some(SystemPrefs {
        text_scale: scale,
        high_contrast: contrast.then(|| ContrastColors::for_dark(dark)),
        text_scale_known: true,
        contrast_known: true,
    })
}

#[cfg(target_os = "android")]
fn read_now() -> SystemPrefs {
    crate::android::accessibility::read()
        .and_then(|a| parse_android(&a))
        .unwrap_or_default()
}

/// The browser's settings: the page's default font size over the usual
/// 16 px, and `prefers-contrast: more` or `forced-colors: active`.
#[cfg(target_arch = "wasm32")]
fn read_now() -> SystemPrefs {
    let a = crate::web::accessibility();
    SystemPrefs {
        text_scale: a.font_px.map_or(1.0, |px| px / 16.0),
        high_contrast: a.contrast.then(|| ContrastColors::for_dark(a.dark)),
        text_scale_known: a.font_px.is_some(),
        contrast_known: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_output_is_read_and_nonsense_is_ignored() {
        let acc = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Accessibility\r\n    TextScaleFactor    REG_DWORD    0x96\r\n";
        assert_eq!(text_scale_from(acc), 1.5);
        assert_eq!(
            text_scale_from("    TextScaleFactor    REG_DWORD    0x3e8"),
            1.0
        );
        assert_eq!(text_scale_from(""), 1.0);
        let hc = "    Flags    REG_SZ    127\r\n    High Contrast Scheme    REG_SZ    \r\n";
        assert!(high_contrast_on(hc));
        assert!(!high_contrast_on("    Flags    REG_SZ    126"));
        assert!(!high_contrast_on("    Flags    REG_SZ    x"));
        let colors = "    Window    REG_SZ    0 0 0\r\n    WindowText    REG_SZ    255 255 255\r\n    Hilight    REG_SZ    26 235 255\r\n    HilightText    REG_SZ    0 0 0\r\n    ButtonFace    REG_SZ    10 20 30\r\n    GrayText    REG_SZ    bad\r\n";
        let c = colors_from(colors);
        assert_eq!(c.button_face, [10, 20, 30]);
        assert_eq!(c.window_text, [255, 255, 255]);
        // Missing or unreadable values: the classic black theme's.
        assert_eq!(c.gray_text, ContrastColors::BLACK.gray_text);
        assert_eq!(c.hot_light, ContrastColors::BLACK.hot_light);
        assert!(c.is_dark());
        assert_eq!(parse_rgb("1 2 3 4"), None);
        assert_eq!(parse_rgb("1 2"), None);
        // "Window" does not match "WindowText".
        assert_eq!(
            parse_sz("    WindowText    REG_SZ    1 1 1", "Window"),
            None
        );
    }

    /// macOS (#1412): AppKit's "Increase contrast" and the dark or light
    /// windows, as the script prints them.
    #[test]
    fn macos_contrast_is_read_from_appkit() {
        assert_eq!(parse_macos("contrast dark\n"), Some((true, true)));
        assert_eq!(parse_macos("normal light"), Some((false, false)));
        assert_eq!(parse_macos("contrast light"), Some((true, false)));
        assert_eq!(parse_macos(""), None);
        assert_eq!(parse_macos("execution error: 1"), None);
        assert!(MACOS_SCRIPT.contains("accessibilityDisplayShouldIncreaseContrast"));
        assert_eq!(ContrastColors::for_dark(true), ContrastColors::BLACK);
        assert_eq!(ContrastColors::for_dark(false), ContrastColors::WHITE);
        assert!(!ContrastColors::WHITE.is_dark());
    }

    /// Linux (#1412): GNOME's text scaling factor and high contrast
    /// through gsettings, the portal's contrast through gdbus, KDE's
    /// forced font resolution or font size and its colour scheme.
    #[test]
    fn linux_desktops_are_read() {
        assert_eq!(gnome_text_scale("1.25\n"), Some(1.25));
        assert_eq!(gnome_text_scale("1.0"), Some(1.0));
        assert_eq!(gnome_text_scale("No such key"), None);
        assert_eq!(gnome_text_scale("0"), None);
        assert_eq!(gsettings_value("true"), "true");
        assert!(gnome_contrast_theme("'HighContrast'"));
        assert!(gnome_contrast_theme("'HighContrastInverse'"));
        assert!(!gnome_contrast_theme("'Adwaita'"));
        assert_eq!(portal_number("(<<uint32 1>>,)"), Some(1));
        assert_eq!(portal_number("(<uint32 0>,)"), Some(0));
        assert_eq!(portal_number("Error: GDBus.Error"), None);
        let kcmfonts = "[General]\nforceFontDPI=120\n";
        assert_eq!(kde_text_scale(kcmfonts, ""), Some(1.25));
        let off = "[General]\nforceFontDPI=0\n";
        let globals = "[KDE]\nfont=Wrong,30\n[General]\nColorScheme=BreezeHighContrast\nfont=Noto Sans,12,-1,5,50,0,0,0,0,0\n";
        assert_eq!(kde_text_scale(off, globals), Some(1.2));
        assert_eq!(kde_text_scale("", "[General]\n"), None);
        assert!(kde_contrast_scheme(globals));
        assert!(kde_contrast_scheme(
            "[General]\nColorScheme=Breeze High Contrast\n"
        ));
        assert!(!kde_contrast_scheme("[General]\nColorScheme=BreezeDark\n"));
    }

    /// Android (#1412): the activity's font scale, contrast and dark
    /// windows.
    #[test]
    fn android_font_scale_and_contrast_are_read() {
        let p = parse_android("1.3 1 0").expect("read");
        assert!((p.text_scale - 1.3).abs() < 1e-6);
        assert_eq!(p.high_contrast, Some(ContrastColors::WHITE));
        assert!(p.text_scale_known && p.contrast_known);
        let p = parse_android("0.85 0 1").expect("read");
        assert_eq!(p.high_contrast, None);
        assert_eq!(
            parse_android("2 1 1").and_then(|p| p.high_contrast),
            Some(ContrastColors::BLACK)
        );
        assert_eq!(parse_android(""), None);
        assert_eq!(parse_android("NaN 1 1"), None);
        assert_eq!(parse_android("1.0 1"), None);
    }

    /// Tests see the defaults, whatever the system running them has:
    /// the interface sizes they check stay the same everywhere.
    #[test]
    fn tests_see_the_defaults() {
        assert_eq!(get(), SystemPrefs::default());
    }
}
