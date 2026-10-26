//! The splash screen. While TraceDraw starts, a small borderless window
//! centred on the screen shows the application icon, the name, the
//! tagline and the version over the brand artwork (see `ui/brand.rs`),
//! with what is loading and a progress bar. The system font list is read
//! on a worker thread meanwhile; when it is in and the splash has been up
//! for [`MIN_TIME`], the editor is built (opening the file named on the
//! command line) and the window becomes the main window, maximized or at
//! the size it was left. Options > General turns the splash off;
//! `--no-splash` or `TRACEDRAW_NO_SPLASH=1` skip it for one run.

use crate::i18n::{tr, trf};
use crate::settings::Settings;
use crate::ui::brand;
use egui::{Color32, CornerRadius, FontId, Pos2, Rect, Stroke, TextureHandle, Vec2};
use std::path::Path;
use web_time::{Duration, Instant};

/// The splash window's size in points.
pub const SIZE: Vec2 = Vec2::new(640.0, 380.0);
/// The main window's size when none was kept, and its minimum.
pub const DEFAULT_SIZE: Vec2 = Vec2::new(1600.0, 950.0);
/// Small enough for a 1280 x 720 screen with a taskbar, at any interface
/// size (see [`min_size_for`]).
pub const MIN_SIZE: Vec2 = Vec2::new(800.0, 500.0);
/// The shortest time the splash stays up, so it never just flashes.
pub const MIN_TIME: Duration = Duration::from_millis(1600);

/// What the shell does after a splash frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Keep showing the splash.
    Wait,
    /// Build the editor and open the main window.
    Start,
}

/// The splash screen's state.
pub struct Splash {
    started: Instant,
    /// The interface fonts, read on a worker thread.
    fonts: Option<std::thread::JoinHandle<egui::FontDefinitions>>,
    /// Frames drawn since the full fonts were installed (they apply from
    /// the next frame on).
    frames_with_fonts: Option<u32>,
    /// The file named on the command line, shown while it opens.
    file_name: Option<String>,
    /// True once the last frame (the "opening" status, a full bar) shows.
    last_frame_drawn: bool,
    icon: Option<TextureHandle>,
    /// F8 was held while the splash showed: the editor asks whether to
    /// reset every preference.
    pub reset_requested: bool,
}

impl Splash {
    /// Start the splash: the bundled fonts at once, the system ones on a
    /// worker thread.
    pub fn start(ctx: &egui::Context, file: Option<&Path>) -> Self {
        ctx.set_fonts(crate::theme::bundled_font_definitions());
        ctx.set_visuals(crate::theme::visuals());
        let fonts = std::thread::Builder::new()
            .name("tracedraw-fonts".into())
            .spawn(crate::theme::font_definitions)
            .map_err(|e| log::warn!("font thread: {e}"))
            .ok();
        Splash {
            started: Instant::now(),
            fonts,
            frames_with_fonts: None,
            file_name: file
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned()),
            last_frame_drawn: false,
            icon: brand::icon(ctx),
            reset_requested: false,
        }
    }

    /// Draw one splash frame in `ui` (the whole window); returns
    /// [`Step::Start`] when the editor should be built.
    pub fn update(&mut self, ui: &mut egui::Ui) -> Step {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.key_down(egui::Key::F8)) {
            self.reset_requested = true;
        }
        if self.frames_with_fonts.is_none() && self.fonts.as_ref().is_none_or(|h| h.is_finished()) {
            let defs = self
                .fonts
                .take()
                .and_then(|h| h.join().ok())
                .unwrap_or_else(crate::theme::font_definitions);
            ctx.set_fonts(defs);
            self.frames_with_fonts = Some(0);
        } else if let Some(n) = self.frames_with_fonts.as_mut() {
            *n = n.saturating_add(1);
        }
        let elapsed = self.started.elapsed();
        let fonts_ready = self.frames_with_fonts.is_some();
        let texts = !crate::theme::needs_system_font(&crate::i18n::language())
            || self.frames_with_fonts.is_some_and(|n| n >= 1);
        let status = status_text(fonts_ready, self.file_name.as_deref());
        paint(
            ui,
            progress(elapsed, fonts_ready),
            texts.then_some(status.as_str()),
            texts,
            self.icon.as_ref(),
        );
        if fonts_ready && elapsed >= MIN_TIME {
            if self.last_frame_drawn {
                return Step::Start;
            }
            self.last_frame_drawn = true;
        }
        ctx.request_repaint_after(Duration::from_millis(16));
        Step::Wait
    }
}

/// The status line: the fonts while they load, then the file being
/// opened or the workspace.
pub fn status_text(fonts_ready: bool, file: Option<&str>) -> String {
    match (fonts_ready, file) {
        (false, _) => tr("splash.loading_fonts"),
        (true, Some(name)) => trf("splash.opening", &[("name", name)]),
        (true, None) => tr("splash.preparing"),
    }
}

/// The progress bar's fill, 0 to 1: it eases towards 80 % over
/// [`MIN_TIME`] while the fonts load and reaches the end once they are in.
pub fn progress(elapsed: Duration, fonts_ready: bool) -> f32 {
    let t = (elapsed.as_secs_f32() / MIN_TIME.as_secs_f32()).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - t) * (1.0 - t);
    let cap = if fonts_ready { 1.0 } else { 0.8 };
    cap * eased
}

/// Paint the splash over the whole of `ui`. `texts` draws the tagline and
/// the copyright line (left out while the fonts for the language load).
fn paint(
    ui: &mut egui::Ui,
    progress: f32,
    status: Option<&str>,
    texts: bool,
    icon: Option<&TextureHandle>,
) {
    let rect = ui.max_rect();
    let painter = ui.painter_at(rect);
    brand::gradient(&painter, rect);
    let s = (rect.width() / SIZE.x).min(rect.height() / SIZE.y).max(0.5);
    brand::curves(&painter, rect, 1.35 * s, &[0.24, 0.62]);

    let pad = 44.0 * s;
    let icon_rect = Rect::from_min_size(rect.min + Vec2::new(pad, 64.0 * s), Vec2::splat(96.0 * s));
    brand::paint_icon(&painter, icon, icon_rect);
    let name = brand::name_galley(&painter, 46.0 * s);
    let tagline = texts.then(|| {
        painter.layout_no_wrap(
            tr("dialog.about_tagline"),
            FontId::proportional(17.0 * s),
            brand::TAGLINE,
        )
    });
    let block_h = name.size().y + tagline.as_ref().map_or(0.0, |g| g.size().y + 2.0 * s);
    let left = icon_rect.right() + 24.0 * s;
    let mut y = icon_rect.center().y - block_h / 2.0;
    let name_h = name.size().y;
    painter.galley(Pos2::new(left, y), name, Color32::WHITE);
    y += name_h + 2.0 * s;
    if let Some(g) = tagline {
        painter.galley(Pos2::new(left + 2.0 * s, y), g, brand::TAGLINE);
    }

    // A darker strip along the bottom for the status, the progress bar
    // and the footer, over the ends of the curves.
    let bar_y = rect.bottom() - 54.0 * s;
    let strip = Rect::from_min_max(
        Pos2::new(rect.left(), bar_y - 40.0 * s),
        rect.right_bottom(),
    );
    painter.rect_filled(
        strip,
        CornerRadius::ZERO,
        Color32::from_rgba_unmultiplied(0x0A, 0x12, 0x22, 225),
    );
    painter.hline(
        strip.x_range(),
        strip.top(),
        Stroke::new(1.0, Color32::from_white_alpha(22)),
    );

    // The progress bar with the status above it.
    let track = Rect::from_min_max(
        Pos2::new(rect.left() + pad, bar_y),
        Pos2::new(rect.right() - pad, bar_y + 3.0 * s),
    );
    painter.rect_filled(track, CornerRadius::same(2), Color32::from_white_alpha(30));
    let mut fill = track;
    fill.set_width(track.width() * progress.clamp(0.0, 1.0));
    painter.rect_filled(fill, CornerRadius::same(2), brand::CURVE);
    if let Some(text) = status {
        let g = painter.layout_no_wrap(
            text.to_string(),
            FontId::proportional(13.0 * s),
            brand::TAGLINE,
        );
        let h = g.size().y;
        painter.galley(
            Pos2::new(track.left(), bar_y - 8.0 * s - h),
            g,
            brand::TAGLINE,
        );
    }

    // The footer: copyright on the left, version on the right.
    let foot_y = rect.bottom() - 34.0 * s;
    let small = FontId::proportional(11.5 * s);
    if texts {
        let g = painter.layout_no_wrap(
            trf(
                "dialog.about_copyright",
                &[("name", env!("CARGO_PKG_AUTHORS"))],
            ),
            small.clone(),
            brand::FOOTNOTE,
        );
        painter.galley(Pos2::new(track.left(), foot_y), g, brand::FOOTNOTE);
    }
    let v = painter.layout_no_wrap(
        format!("v{}", env!("CARGO_PKG_VERSION")),
        small,
        brand::FOOTNOTE,
    );
    let vw = v.size().x;
    painter.galley(Pos2::new(track.right() - vw, foot_y), v, brand::FOOTNOTE);

    painter.rect_stroke(
        rect.shrink(0.5),
        CornerRadius::ZERO,
        Stroke::new(1.0, Color32::from_white_alpha(40)),
        egui::StrokeKind::Inside,
    );
}

/// The main window's size: the one kept from the last session when it
/// is sensible, else the default; never larger than the monitor.
pub fn main_window_size(saved: [f32; 2], monitor: Option<Vec2>) -> Vec2 {
    let saved = Vec2::new(saved[0], saved[1]);
    let mut size = if saved.x.is_finite()
        && saved.y.is_finite()
        && saved.x >= MIN_SIZE.x
        && saved.y >= MIN_SIZE.y
    {
        saved
    } else {
        DEFAULT_SIZE
    };
    if let Some(m) = monitor.filter(|m| m.x > 0.0 && m.y > 0.0) {
        size = size.min(m * 0.95).max(min_size_for(monitor));
    }
    size
}

/// The window's minimum size on a monitor of `monitor` (both in points):
/// [`MIN_SIZE`], never more than four fifths of the monitor, so the
/// window always fits the screen with room for a taskbar.
pub fn min_size_for(monitor: Option<Vec2>) -> Vec2 {
    match monitor.filter(|m| m.x > 0.0 && m.y > 0.0) {
        Some(m) => MIN_SIZE.min(m * 0.8),
        None => MIN_SIZE,
    }
}

/// The main window starts maximized when it was left so, or when the
/// size it would get does not fit the monitor (a small screen).
pub fn start_maximized(saved: [f32; 2], maximized: bool, monitor: Option<Vec2>) -> bool {
    if maximized {
        return true;
    }
    let Some(m) = monitor.filter(|m| m.x > 0.0 && m.y > 0.0) else {
        return false;
    };
    let saved = Vec2::new(saved[0], saved[1]);
    let wanted = if saved.x.is_finite()
        && saved.y.is_finite()
        && saved.x >= MIN_SIZE.x
        && saved.y >= MIN_SIZE.y
    {
        saved
    } else {
        DEFAULT_SIZE
    };
    wanted.x > m.x * 0.95 || wanted.y > m.y * 0.95
}

/// Where a window of `size` sits centred on a monitor of `monitor` (its
/// top-left corner, never off the top or the left).
pub fn centred_position(size: Vec2, monitor: Option<Vec2>) -> Option<Pos2> {
    let m = monitor.filter(|m| m.x > 0.0 && m.y > 0.0)?;
    let p = ((m - size) / 2.0).max(Vec2::ZERO);
    Some(Pos2::new(p.x, p.y))
}

/// The monitor's size in system points (pixels over the system's
/// scaling), whatever the interface size: the unit the window's kept
/// size is in. egui reports it in interface points, which shrink as the
/// interface grows (#787).
fn monitor_in_system_points(ctx: &egui::Context) -> Option<Vec2> {
    let zoom = ctx.zoom_factor();
    ctx.input(|i| i.viewport().monitor_size).map(|m| m * zoom)
}

/// Turn the splash window into the main window: decorations, resizable,
/// the kept size centred on the monitor, maximized when it was left so.
/// Sizes are worked out in system points (see [`remember_window`]) and
/// sent in interface points.
pub fn open_main_window(ctx: &egui::Context, settings: &Settings) {
    use egui::ViewportCommand as V;
    let zoom = ctx.zoom_factor().max(0.01);
    let monitor = monitor_in_system_points(ctx);
    let size = main_window_size(settings.window_size, monitor);
    ctx.send_viewport_cmd(V::Decorations(true));
    // The splash window is made not resizable, which on X11 records its
    // own size as the window's minimum and maximum; making it resizable
    // again restores those. Clear the maximum and set the minimum first,
    // or the main window keeps the splash's maximum and cannot be
    // maximized or resized.
    ctx.send_viewport_cmd(V::MaxInnerSize(egui::Vec2::INFINITY));
    ctx.send_viewport_cmd(V::MinInnerSize(min_size_for(monitor) / zoom));
    ctx.send_viewport_cmd(V::Resizable(true));
    ctx.send_viewport_cmd(V::EnableButtons {
        close: true,
        minimized: true,
        maximize: true,
    });
    ctx.send_viewport_cmd(V::InnerSize(size / zoom));
    if let Some(p) = centred_position(size, monitor) {
        ctx.send_viewport_cmd(V::OuterPosition((p.to_vec2() / zoom).to_pos2()));
    }
    if start_maximized(settings.window_size, settings.window_maximized, monitor) {
        ctx.send_viewport_cmd(V::Maximized(true));
    }
    ctx.send_viewport_cmd(V::Focus);
}

/// Frames after the main window opens during which its state is still
/// settling: what the window system reports then is not kept, and a
/// maximize it dropped is asked for again (#787).
pub const SETTLE_FRAMES: u32 = 30;

/// One of the first [`SETTLE_FRAMES`] frames of the main window
/// (`frame` counted from 0). Some window systems drop a maximize asked
/// for while the splash turns into the main window (on Windows the
/// window's frame changes at the same time), and the window started at
/// its kept size instead, every time: it is asked for again a few frames
/// on while the window still reports it is not maximized. So is a window
/// started without the splash at a kept size larger than the monitor
/// (another screen, a larger scaling), which then fills the screen.
pub fn settle_window(ctx: &egui::Context, settings: &Settings, frame: u32) {
    use egui::ViewportCommand as V;
    let monitor = monitor_in_system_points(ctx);
    let (maximized, minimized) = ctx.input(|i| {
        let v = i.viewport();
        (v.maximized, v.minimized)
    });
    if frame + 1 < SETTLE_FRAMES {
        ctx.request_repaint();
    }
    if minimized == Some(true) {
        return;
    }
    // Left maximized, or a kept size this monitor cannot hold (another
    // screen, a larger scaling): maximized.
    if start_maximized(settings.window_size, settings.window_maximized, monitor)
        && maximized == Some(false)
        && (frame == 3 || frame == 15)
    {
        ctx.send_viewport_cmd(V::Maximized(true));
    }
}

/// Keep the main window's state for the next start: maximized or not,
/// and the size while it is neither maximized nor minimized, in system
/// points: the interface size (Options, or the system's text size)
/// scales the interface points the window reports, and a size kept in
/// them came back smaller at every start, as the splash opens the
/// window before the interface size applies (#787).
pub fn remember_window(ctx: &egui::Context, settings: &mut Settings) {
    let zoom = ctx.zoom_factor();
    let (maximized, minimized, inner) = ctx.input(|i| {
        let v = i.viewport();
        (v.maximized, v.minimized, v.inner_rect)
    });
    if minimized == Some(true) {
        return;
    }
    if let Some(m) = maximized {
        settings.window_maximized = m;
        if !m {
            if let Some(r) = inner
                .map(|r| r.size() * zoom)
                .filter(|r| r.x >= MIN_SIZE.x && r.y >= MIN_SIZE.y)
            {
                settings.window_size = [r.x, r.y];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_main_window_drops_the_splash_maximum_before_it_turns_resizable() {
        use egui::ViewportCommand as V;
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            open_main_window(ui.ctx(), &Settings::default());
        });
        // No painter takes the textures here.
        out.textures_delta.clear();
        let cmds = &out
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .expect("the root viewport")
            .commands;
        let at = |want: &dyn Fn(&V) -> bool| cmds.iter().position(want);
        let max = at(&|c| matches!(c, V::MaxInnerSize(s) if s == &egui::Vec2::INFINITY))
            .expect("the maximum is cleared");
        let min =
            at(&|c| matches!(c, V::MinInnerSize(s) if s == &MIN_SIZE)).expect("the minimum is set");
        let resizable =
            at(&|c| matches!(c, V::Resizable(true))).expect("the window turns resizable");
        assert!(max < resizable && min < resizable, "{cmds:?}");
    }

    #[test]
    fn small_screens_get_a_window_that_fits_and_starts_maximized() {
        // 1366 x 768 and 1280 x 720 screens, at 100 % and at 125 % (the
        // monitor's size in points shrinks as the interface grows).
        for m in [
            Vec2::new(1366.0, 768.0),
            Vec2::new(1280.0, 720.0),
            Vec2::new(1366.0, 768.0) / 1.25,
            Vec2::new(1280.0, 720.0) / 1.25,
        ] {
            let min = min_size_for(Some(m));
            assert!(
                min.x <= m.x * 0.8 + 0.01 && min.y <= m.y * 0.8 + 0.01,
                "{m:?} {min:?}"
            );
            let size = main_window_size([0.0, 0.0], Some(m));
            assert!(size.x <= m.x && size.y <= m.y, "{m:?} {size:?}");
            assert!(size.x >= min.x && size.y >= min.y);
            // The default size does not fit: the window starts maximized.
            assert!(start_maximized([0.0, 0.0], false, Some(m)));
        }
        // A large screen keeps the kept size, not maximized.
        let big = Some(Vec2::new(2560.0, 1440.0));
        assert!(!start_maximized([1600.0, 950.0], false, big));
        assert!(start_maximized([1600.0, 950.0], true, big));
        assert_eq!(min_size_for(big), MIN_SIZE);
    }

    /// A frame with the window reported as `info`, interface size `zoom`.
    fn window_frame(
        zoom: f32,
        info: egui::ViewportInfo,
        f: impl FnMut(&egui::Context),
    ) -> Vec<egui::ViewportCommand> {
        let ctx = egui::Context::default();
        ctx.set_zoom_factor(zoom);
        let mut f = f;
        let mut input = egui::RawInput::default();
        input.viewports.insert(egui::ViewportId::ROOT, info);
        // The zoom applies from the frame after it is set.
        ctx.run_ui(input.clone(), |_| {}).textures_delta.clear();
        let mut out = ctx.run_ui(input, |ui| f(ui.ctx()));
        out.textures_delta.clear();
        out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .clone()
    }

    /// #787: the kept size is in system points, whatever the interface
    /// size, so the window comes back at the size it was left at; it was
    /// kept in interface points and came back smaller at every start
    /// with the interface at 150 % (the splash opens it at 100 %).
    #[test]
    fn the_window_comes_back_at_its_size_with_a_larger_interface() {
        use egui::ViewportCommand as V;
        // 1000 x 600 interface points at 150 %: 1500 x 900 system points.
        let info = egui::ViewportInfo {
            maximized: Some(false),
            minimized: Some(false),
            inner_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(1000.0, 600.0),
            )),
            monitor_size: Some(Vec2::new(2560.0, 1440.0) / 1.5),
            ..Default::default()
        };
        let mut settings = Settings::default();
        window_frame(1.5, info.clone(), |ctx| remember_window(ctx, &mut settings));
        assert!(!settings.window_maximized);
        let [w, h] = settings.window_size;
        assert!(
            (w - 1500.0).abs() < 0.5 && (h - 900.0).abs() < 0.5,
            "{w} {h}"
        );
        // Opened again with the interface at 150 %: the same window.
        let cmds = window_frame(1.5, info.clone(), |ctx| open_main_window(ctx, &settings));
        let size = cmds
            .iter()
            .find_map(|c| match c {
                V::InnerSize(s) => Some(*s),
                _ => None,
            })
            .expect("a size");
        assert!((size - Vec2::new(1000.0, 600.0)).length() < 0.5, "{size:?}");
        assert!(!cmds.iter().any(|c| matches!(c, V::Maximized(true))));
        // And at 100 % (the splash): 1500 x 900, fitting the monitor.
        let at_100 = egui::ViewportInfo {
            monitor_size: Some(Vec2::new(2560.0, 1440.0)),
            ..info
        };
        let cmds = window_frame(1.0, at_100, |ctx| open_main_window(ctx, &settings));
        assert!(cmds.iter().any(
            |c| matches!(c, V::InnerSize(s) if (*s - Vec2::new(1500.0, 900.0)).length() < 0.5)
        ));
    }

    /// #787: a maximize the window system dropped while the window was
    /// set up is asked for again while the window says it is not
    /// maximized, and for a kept size larger than the monitor.
    #[test]
    fn a_dropped_maximize_is_asked_for_again_and_a_large_window_fills_the_screen() {
        use egui::ViewportCommand as V;
        let monitor = Some(Vec2::new(1920.0, 1080.0));
        let not_max = egui::ViewportInfo {
            maximized: Some(false),
            minimized: Some(false),
            inner_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(1600.0, 950.0),
            )),
            monitor_size: monitor,
            ..Default::default()
        };
        let settings = Settings::default();
        assert!(settings.window_maximized);
        let asks = |info: &egui::ViewportInfo, frame| {
            window_frame(1.0, info.clone(), |ctx| {
                settle_window(ctx, &settings, frame)
            })
            .iter()
            .any(|c| matches!(c, V::Maximized(true)))
        };
        assert!(asks(&not_max, 3));
        assert!(asks(&not_max, 15));
        assert!(!asks(&not_max, 4));
        let max = egui::ViewportInfo {
            maximized: Some(true),
            ..not_max.clone()
        };
        assert!(!asks(&max, 3));
        // Kept not maximized but larger than this monitor: maximized.
        let mut large = Settings::default();
        large.window_maximized = false;
        large.window_size = [1700.0, 1000.0];
        let small_screen = egui::ViewportInfo {
            inner_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(1700.0, 1000.0),
            )),
            monitor_size: Some(Vec2::new(1366.0, 768.0)),
            ..not_max.clone()
        };
        let cmds = window_frame(1.0, small_screen.clone(), |ctx| {
            settle_window(ctx, &large, 3)
        });
        assert!(cmds.iter().any(|c| matches!(c, V::Maximized(true))));
        // Kept not maximized and fitting: left alone.
        large.window_size = [1200.0, 700.0];
        let cmds = window_frame(1.0, small_screen, |ctx| settle_window(ctx, &large, 3));
        assert!(!cmds
            .iter()
            .any(|c| matches!(c, V::Maximized(true) | V::InnerSize(_))));
    }

    #[test]
    fn the_main_window_offers_the_maximize_button() {
        use egui::ViewportCommand as V;
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            open_main_window(ui.ctx(), &Settings::default());
        });
        out.textures_delta.clear();
        let cmds = &out.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(cmds
            .iter()
            .any(|c| matches!(c, V::EnableButtons { maximize: true, .. })));
        assert!(cmds.iter().any(|c| matches!(c, V::Decorations(true))));
    }

    #[test]
    fn progress_waits_for_the_fonts_and_the_minimum_time() {
        assert_eq!(progress(Duration::ZERO, false), 0.0);
        let half = progress(MIN_TIME / 2, false);
        assert!(half > 0.4 && half < 0.8, "{half}");
        // Without the fonts the bar stops at 80 %.
        assert!((progress(MIN_TIME * 3, false) - 0.8).abs() < 1e-6);
        // With them it fills once the minimum time is over.
        assert!((progress(MIN_TIME, true) - 1.0).abs() < 1e-6);
        assert!(progress(MIN_TIME / 2, true) < 1.0);
    }

    #[test]
    fn the_status_names_the_file_being_opened() {
        crate::i18n::set_language("en");
        assert_eq!(status_text(false, Some("a.cdr")), "Loading fonts...");
        assert_eq!(status_text(true, Some("a.cdr")), "Opening a.cdr...");
        assert_eq!(status_text(true, None), "Preparing the workspace...");
    }

    #[test]
    fn the_main_window_keeps_a_sensible_size() {
        let monitor = Some(Vec2::new(1920.0, 1080.0));
        // Nothing kept yet: the default size.
        assert_eq!(main_window_size([0.0, 0.0], monitor), DEFAULT_SIZE);
        // A kept size comes back.
        assert_eq!(
            main_window_size([1200.0, 800.0], monitor),
            Vec2::new(1200.0, 800.0)
        );
        // Never larger than the monitor (a 125 % laptop screen).
        let small = Some(Vec2::new(1536.0, 864.0));
        let s = main_window_size([0.0, 0.0], small);
        assert!(s.x <= 1536.0 && s.y <= 864.0 && s.x >= MIN_SIZE.x);
        // Nonsense falls back to the default.
        assert_eq!(main_window_size([f32::NAN, 5.0], None), DEFAULT_SIZE);
    }

    #[test]
    fn the_main_window_is_centred() {
        let p = centred_position(Vec2::new(1600.0, 900.0), Some(Vec2::new(1920.0, 1080.0)));
        assert_eq!(p, Some(Pos2::new(160.0, 90.0)));
        // Larger than the monitor: pinned to the top left.
        let p = centred_position(Vec2::new(2000.0, 900.0), Some(Vec2::new(1920.0, 800.0)));
        assert_eq!(p, Some(Pos2::new(0.0, 0.0)));
        assert_eq!(centred_position(Vec2::new(10.0, 10.0), None), None);
    }

    #[test]
    fn a_splash_frame_draws_and_then_starts_the_editor() {
        crate::i18n::set_language("en");
        let ctx = egui::Context::default();
        let mut splash = Splash::start(&ctx, Some(Path::new("/tmp/a.cdr")));
        // Let the font thread finish and pretend the minimum time is over.
        for _ in 0..3000 {
            if splash.fonts.as_ref().is_none_or(|h| h.is_finished()) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        splash.started = Instant::now()
            .checked_sub(MIN_TIME * 2)
            .unwrap_or_else(Instant::now);
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)),
            ..Default::default()
        };
        let mut steps = Vec::new();
        for _ in 0..4 {
            let mut out = ctx.run_ui(input(), |ui| steps.push(splash.update(ui)));
            out.textures_delta.clear();
        }
        // The first frame installs the fonts, the next shows the last
        // status, then the editor starts.
        assert_eq!(steps[0], Step::Wait);
        assert!(steps.contains(&Step::Start));
        assert!(!splash.reset_requested);
    }

    #[test]
    fn f8_while_starting_asks_to_reset_the_settings() {
        let ctx = egui::Context::default();
        let mut splash = Splash::start(&ctx, None);
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)),
            events: vec![egui::Event::Key {
                key: egui::Key::F8,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            splash.update(ui);
        });
        out.textures_delta.clear();
        assert!(splash.reset_requested);
    }
}
