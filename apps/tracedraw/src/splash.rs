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

/// The main window when there is no splash screen: its kept size, which
/// is also the size a maximized window restores to, and maximized from
/// the start when it was left so (not in a `plain` start for automation).
/// It opens at once at the screen's size: the patched eframe does not ask
/// for the size again after making a maximized window, which on Windows
/// showed it at its normal size first and then maximized it (#787,
/// vendor/README.md).
pub fn main_window_builder(settings: &Settings, plain: bool) -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_inner_size(main_window_size(settings.window_size, None))
        .with_min_inner_size(MIN_SIZE)
        .with_maximized(settings.window_maximized && !plain)
        .with_title("TraceDraw")
}

/// Turn the splash window into the main window: decorations, resizable,
/// the kept size centred on the monitor. Sizes are worked out in system
/// points (see [`remember_window`]) and sent in interface points. A
/// window that starts maximized is maximized by [`WindowSettle`] in the
/// next frames, once the window system has applied this size (#787).
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
    ctx.send_viewport_cmd(V::Focus);
}

/// Frames after the main window opens during which its state is still
/// settling: what the window system reports then is not kept (#787).
pub const SETTLE_FRAMES: u32 = 30;
/// The shortest time, in seconds, the main window's state settles: slow
/// first frames (an interface in a language drawn with a large system
/// font, a new drawing with its panels) can use up the frames before
/// the window system has done what it was asked (#787).
pub const SETTLE_SECONDS: f64 = 3.0;
/// The time, in seconds, the window system gets for one request before
/// the window's state is looked at again.
const SETTLE_GAP: f64 = 0.25;
/// Requests made at most while the window settles, so a window system
/// that never maximizes is not asked forever.
const SETTLE_REQUESTS: u32 = 6;
/// A maximized window's inner size is at least this share of the
/// monitor's in one direction at least: a taskbar sits on one side, and
/// the title bar takes a little of the height. A window at its normal
/// size starts at most 95 % of the monitor in both (see
/// [`main_window_size`]).
const FILLS_SHARE: f32 = 0.96;

/// The main window's first moments (#787). Until it has settled its
/// state is not kept, and a window that should start maximized (left
/// maximized, or a kept size this monitor cannot hold) is maximized
/// here, a frame after the splash asked for its size and position:
/// on Windows a size applied after the maximize left the window at its
/// normal size in the corner of the screen with the maximized button
/// state, and a second maximize does not change such a window. So the
/// window is checked against the monitor, not only by the state it
/// reports: one that says it is maximized but does not fill the screen
/// is restored and then maximized again, and a maximize the window
/// system dropped is asked for again.
#[derive(Debug, Default, Clone)]
pub struct WindowSettle {
    /// Frames seen so far.
    frames: u32,
    /// The input time of the first frame and of the latest one.
    started: Option<f64>,
    now: f64,
    /// The input time of the last request.
    asked_at: Option<f64>,
    /// Since when the window says it is maximized but does not fill the
    /// screen (a window system still resizing it shows that briefly).
    stuck_since: Option<f64>,
    /// Requests made.
    requests: u32,
    /// The window was seen as it should be (maximized and filling the
    /// screen, or not to be maximized): nothing more to do, a later
    /// restore is the user's.
    done: bool,
}

impl WindowSettle {
    /// True while the window's state is still settling.
    pub fn settling(&self) -> bool {
        let elapsed = self.started.map_or(0.0, |s| self.now - s);
        self.frames < SETTLE_FRAMES || elapsed < SETTLE_SECONDS
    }

    /// One frame of the main window while it settles.
    pub fn step(&mut self, ctx: &egui::Context, settings: &Settings) {
        use egui::ViewportCommand as V;
        self.now = ctx.input(|i| i.time);
        self.started.get_or_insert(self.now);
        self.frames += 1;
        // Frames go on while nothing else asks for one: at once for the
        // first ones, then a few a second.
        if self.frames < SETTLE_FRAMES {
            ctx.request_repaint();
        } else if self.settling() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if self.done || self.requests >= SETTLE_REQUESTS {
            return;
        }
        let (maximized, minimized, inner, monitor_points) = ctx.input(|i| {
            let v = i.viewport();
            (v.maximized, v.minimized, v.inner_rect, v.monitor_size)
        });
        if minimized == Some(true) {
            return;
        }
        // Left maximized, or a kept size this monitor cannot hold
        // (another screen, a larger scaling): maximized.
        let monitor = monitor_in_system_points(ctx);
        if !start_maximized(settings.window_size, settings.window_maximized, monitor) {
            self.done = true;
            return;
        }
        if self.asked_at.is_some_and(|t| self.now - t < SETTLE_GAP) {
            return;
        }
        // Both in interface points; unknown sizes count as filling.
        let fills = match (inner, monitor_points) {
            (Some(r), Some(m)) if m.x > 0.0 && m.y > 0.0 => {
                r.width() >= m.x * FILLS_SHARE || r.height() >= m.y * FILLS_SHARE
            }
            _ => true,
        };
        if maximized != Some(true) || fills {
            self.stuck_since = None;
        }
        let request = match maximized {
            Some(true) if fills => {
                self.done = true;
                None
            }
            // Says maximized but has not filled the screen for a while:
            // restored, and maximized at the next look.
            Some(true) => {
                let since = *self.stuck_since.get_or_insert(self.now);
                (self.now - since >= SETTLE_GAP).then_some(V::Maximized(false))
            }
            Some(false) => Some(V::Maximized(true)),
            None => None,
        };
        if let Some(r) = request {
            log::info!(
                "main window: {r:?} (maximized {maximized:?}, inner {:?}, monitor {monitor_points:?})",
                inner.map(|r| r.size()),
            );
            ctx.send_viewport_cmd(r);
            self.asked_at = Some(self.now);
            self.stuck_since = None;
            self.requests += 1;
        }
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

/// What the patched eframe applies again to a window it has just made,
/// with its tests, run here: the copy in vendor/ is outside the workspace
/// (vendor/README.md, #787).
#[cfg(test)]
#[path = "../../../vendor/eframe/src/native/window_start.rs"]
mod eframe_window_start;

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

    /// One settling frame at input time `time` with the window reported
    /// as `info`; the commands it sent.
    fn settle_frame(
        settle: &mut WindowSettle,
        settings: &Settings,
        info: egui::ViewportInfo,
        time: f64,
    ) -> Vec<egui::ViewportCommand> {
        let ctx = egui::Context::default();
        let mut input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        input.viewports.insert(egui::ViewportId::ROOT, info);
        let mut out = ctx.run_ui(input, |ui| settle.step(ui.ctx(), settings));
        out.textures_delta.clear();
        // Only the window's state (a new context also sends its theme).
        out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    egui::ViewportCommand::Maximized(_)
                        | egui::ViewportCommand::InnerSize(_)
                        | egui::ViewportCommand::OuterPosition(_)
                )
            })
            .cloned()
            .collect()
    }

    /// A window system as seen on Windows in #787: a window maximized
    /// already ignores another maximize, and a size applied after the
    /// maximize (it came in late) resizes the window but leaves it
    /// marked maximized, in the corner of the screen at its normal size.
    struct FakeWindow {
        maximized: bool,
        inner: Vec2,
        normal: Vec2,
        work_area: Vec2,
        monitor: Vec2,
    }

    impl FakeWindow {
        fn apply(&mut self, cmd: &egui::ViewportCommand) {
            use egui::ViewportCommand as V;
            match cmd {
                V::Maximized(true) if !self.maximized => {
                    self.maximized = true;
                    self.inner = self.work_area;
                }
                V::Maximized(false) if self.maximized => {
                    self.maximized = false;
                    self.inner = self.normal;
                }
                V::InnerSize(s) => {
                    self.inner = *s;
                    if !self.maximized {
                        self.normal = *s;
                    }
                }
                _ => {}
            }
        }

        fn info(&self) -> egui::ViewportInfo {
            egui::ViewportInfo {
                maximized: Some(self.maximized),
                minimized: Some(false),
                inner_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, self.inner)),
                monitor_size: Some(self.monitor),
                ..Default::default()
            }
        }

        fn fills(&self) -> bool {
            self.maximized && self.inner == self.work_area
        }
    }

    /// The splash's window on a 1920 x 1080 monitor with a taskbar.
    fn fake_windows_window() -> FakeWindow {
        FakeWindow {
            maximized: false,
            inner: SIZE,
            normal: SIZE,
            work_area: Vec2::new(1920.0, 1017.0),
            monitor: Vec2::new(1920.0, 1080.0),
        }
    }

    /// Settling frames 50 ms apart for `seconds`, the late commands
    /// `late` applied after the first request the settling made; the
    /// commands the settling sent.
    fn settle_run(
        window: &mut FakeWindow,
        settings: &Settings,
        mut late: Vec<egui::ViewportCommand>,
        seconds: f64,
    ) -> Vec<egui::ViewportCommand> {
        let mut settle = WindowSettle::default();
        let mut sent = Vec::new();
        let mut t = 10.0;
        while settle.settling() && t < 10.0 + seconds {
            let cmds = settle_frame(&mut settle, settings, window.info(), t);
            for c in &cmds {
                window.apply(c);
            }
            if !cmds.is_empty() {
                for c in late.drain(..) {
                    window.apply(&c);
                }
            }
            sent.extend(cmds);
            t += 0.05;
        }
        sent
    }

    /// #787: with the interface in Chinese and a new drawing at start,
    /// the window came up with the maximized button state but at its
    /// normal size in the corner of the screen: the size the splash
    /// asked for was applied after the maximize. The splash no longer
    /// asks for the maximize with the size, and a window that says it is
    /// maximized but does not fill the screen is restored and maximized
    /// again, until it fills the screen.
    #[test]
    fn a_maximized_window_left_at_its_normal_size_is_maximized_again() {
        use egui::ViewportCommand as V;
        let settings = Settings::default();
        assert!(settings.window_maximized);
        // The splash asks for the size and position only.
        let mut window = fake_windows_window();
        let info = window.info();
        let cmds = window_frame(1.0, info, |ctx| open_main_window(ctx, &settings));
        assert!(
            !cmds.iter().any(|c| matches!(c, V::Maximized(_))),
            "{cmds:?}"
        );
        let size = cmds
            .iter()
            .find_map(|c| match c {
                V::InnerSize(s) => Some(*s),
                _ => None,
            })
            .expect("a size");
        // Worst case: that size lands after the maximize that follows.
        let sent = settle_run(&mut window, &settings, vec![V::InnerSize(size)], 5.0);
        assert!(window.fills(), "{sent:?}");
        assert!(sent.iter().any(|c| matches!(c, V::Maximized(false))));

        // The state the reporter saw (marked maximized, normal size):
        // restored, then maximized.
        let mut stuck = fake_windows_window();
        stuck.apply(&V::InnerSize(DEFAULT_SIZE));
        stuck.apply(&V::Maximized(true));
        stuck.apply(&V::InnerSize(DEFAULT_SIZE));
        assert!(stuck.maximized && !stuck.fills());
        let sent = settle_run(&mut stuck, &settings, Vec::new(), 5.0);
        assert!(stuck.fills(), "{sent:?}");
        let first = sent.iter().position(|c| matches!(c, V::Maximized(false)));
        let last = sent.iter().rposition(|c| matches!(c, V::Maximized(true)));
        assert!(first < last, "{sent:?}");
    }

    /// #787: the settling lasts at least its time and its frames, a slow
    /// first frame does not end it early, and a window that is maximized
    /// and fills the screen is left alone.
    #[test]
    fn the_window_settles_for_its_frames_and_its_time() {
        let settings = Settings::default();
        let mut window = fake_windows_window();
        window.apply(&egui::ViewportCommand::Maximized(true));
        let mut settle = WindowSettle::default();
        assert!(settle.settling());
        // Two frames five seconds apart: frames still to come.
        settle_frame(&mut settle, &settings, window.info(), 1.0);
        settle_frame(&mut settle, &settings, window.info(), 6.0);
        assert!(settle.settling());
        // Thirty frames within a tenth of a second: time still to come.
        let mut settle = WindowSettle::default();
        for i in 0..SETTLE_FRAMES {
            let cmds = settle_frame(&mut settle, &settings, window.info(), f64::from(i) * 0.003);
            assert!(cmds.is_empty(), "{cmds:?}");
        }
        assert!(settle.settling());
        settle_frame(&mut settle, &settings, window.info(), SETTLE_SECONDS + 0.1);
        assert!(!settle.settling());
    }

    /// #787: a maximize the window system dropped is asked for again,
    /// a kept size larger than the monitor fills the screen, a kept size
    /// that fits is left alone, and a window system that never maximizes
    /// is not asked forever.
    #[test]
    fn a_dropped_maximize_is_asked_for_again_and_a_large_window_fills_the_screen() {
        use egui::ViewportCommand as V;
        let not_max = egui::ViewportInfo {
            maximized: Some(false),
            minimized: Some(false),
            inner_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(1600.0, 950.0),
            )),
            monitor_size: Some(Vec2::new(1920.0, 1080.0)),
            ..Default::default()
        };
        let settings = Settings::default();
        let mut settle = WindowSettle::default();
        let mut asked = 0;
        for i in 0..200 {
            let cmds = settle_frame(&mut settle, &settings, not_max.clone(), f64::from(i) * 0.05);
            asked += cmds
                .iter()
                .filter(|c| matches!(c, V::Maximized(true)))
                .count();
        }
        assert!(asked >= 2 && asked as u32 <= SETTLE_REQUESTS, "{asked}");
        // Kept not maximized but larger than this monitor: maximized.
        let mut large = Settings::default();
        large.window_maximized = false;
        large.window_size = [1700.0, 1000.0];
        let small_screen = egui::ViewportInfo {
            inner_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(1300.0, 700.0),
            )),
            monitor_size: Some(Vec2::new(1366.0, 768.0)),
            ..not_max.clone()
        };
        let cmds = settle_frame(
            &mut WindowSettle::default(),
            &large,
            small_screen.clone(),
            0.0,
        );
        assert!(cmds.iter().any(|c| matches!(c, V::Maximized(true))));
        // Kept not maximized and fitting: left alone.
        large.window_size = [1200.0, 700.0];
        let mut settle = WindowSettle::default();
        for i in 0..100 {
            let cmds = settle_frame(
                &mut settle,
                &large,
                small_screen.clone(),
                f64::from(i) * 0.05,
            );
            assert!(!cmds
                .iter()
                .any(|c| matches!(c, V::Maximized(_) | V::InnerSize(_))));
        }
    }

    /// Without the splash screen a window left maximized is made
    /// maximized, with its kept size as the size it restores to, and the
    /// patched eframe does not ask for that size again once the window
    /// exists: on Windows that un-maximized the window, so it showed at
    /// its normal size and then grew to the screen (#787).
    #[test]
    fn without_the_splash_a_maximized_window_opens_at_the_screen_size() {
        let mut settings = Settings::default();
        settings.window_size = [1500.0, 900.0];
        settings.window_maximized = true;
        let builder = main_window_builder(&settings, false);
        assert_eq!(builder.maximized, Some(true));
        assert_eq!(builder.inner_size, Some(Vec2::new(1500.0, 900.0)));
        let after = eframe_window_start::after_creation(&builder);
        assert_eq!(after.inner_size, None);
        assert_eq!(after.position, None);
        assert_eq!(after.maximized, Some(true));
        // A start for automation is never maximized; one left at its
        // normal size gets that size again as before.
        assert_eq!(main_window_builder(&settings, true).maximized, Some(false));
        settings.window_maximized = false;
        let normal = main_window_builder(&settings, false);
        assert_eq!(
            eframe_window_start::after_creation(&normal).inner_size,
            normal.inner_size
        );
        // Every place the patched eframe makes a window goes through it.
        let glow = include_str!("../../../vendor/eframe/src/native/glow_integration.rs");
        let wgpu = include_str!("../../../vendor/eframe/src/native/wgpu_integration.rs");
        let epi = include_str!("../../../vendor/eframe/src/native/epi_integration.rs");
        for source in [glow, wgpu, epi] {
            assert!(!source.contains("egui_winit::create_window("));
            assert_eq!(
                source.matches("apply_viewport_builder_to_window(").count(),
                source.matches("window_start::after_creation(").count(),
            );
        }
        assert_eq!(glow.matches("window_start::after_creation(").count(), 2);
        assert_eq!(wgpu.matches("epi_integration::create_window(").count(), 2);
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
