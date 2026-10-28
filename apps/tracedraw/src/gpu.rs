//! Hardware acceleration (Options > General, #445): the window is drawn
//! with wgpu on the graphics card chosen there (the fastest one when none
//! is), or with the system's software adapter when it is off. A start
//! whose renderer cannot open the window starts TraceDraw again with the
//! next one, hardware, then OpenGL, then software, so a virtual machine
//! without a usable graphics card still opens (#439), and a computer
//! whose card only OpenGL can drive is not drawn on the CPU (#616). See
//! docs/behavior/about-and-startup.md.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

/// How a start draws the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// wgpu on a graphics card, never on a software adapter (#616).
    Hardware,
    /// wgpu on the software adapter (WARP on Windows, a CPU Vulkan
    /// driver elsewhere).
    Software,
    /// OpenGL, the renderer of earlier versions.
    Gl,
}

/// The command-line switch that names the renderer of a start.
const ARG: &str = "--renderer=";

impl Mode {
    pub fn arg(self) -> String {
        format!("{ARG}{}", self.name())
    }

    /// The renderer `--renderer=` names among `args`.
    pub fn from_args(args: &[String]) -> Option<Mode> {
        args.iter()
            .find_map(|a| a.strip_prefix(ARG).and_then(Mode::from_name))
    }

    /// How the settings remember it.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Hardware => "hardware",
            Mode::Software => "software",
            Mode::Gl => "gl",
        }
    }

    fn from_name(name: &str) -> Option<Mode> {
        [Mode::Hardware, Mode::Software, Mode::Gl]
            .into_iter()
            .find(|m| m.name() == name)
    }

    /// The renderer a start uses: the one `--renderer=` names (a start
    /// before could not open the window), else the one after `failed`,
    /// the renderer the last start tried without its window ever opening
    /// (it crashed or hung inside the graphics driver, #570), else the one
    /// that opened it last time when the first choice did not, else the
    /// graphics card or, with hardware acceleration off, the software
    /// adapter.
    pub fn for_start(args: &[String], s: &crate::settings::Settings, failed: Option<Mode>) -> Mode {
        let first = Mode::first(s);
        Mode::from_args(args)
            .or_else(|| failed.map(|f| f.next(first).unwrap_or(first)))
            .or_else(|| Mode::from_name(&s.working_renderer).filter(|m| *m != first))
            .unwrap_or(first)
    }

    /// The renderer Options asks for: the graphics card, or the software
    /// adapter with hardware acceleration off.
    pub fn first(s: &crate::settings::Settings) -> Mode {
        if s.hardware_acceleration {
            Mode::Hardware
        } else {
            Mode::Software
        }
    }

    /// The renderers tried in turn when `first` is the choice: OpenGL
    /// before the software adapter, which draws every frame on the CPU,
    /// and the software adapter last because it always opens (#616).
    pub fn chain(first: Mode) -> &'static [Mode] {
        match first {
            Mode::Hardware => &[Mode::Hardware, Mode::Gl, Mode::Software],
            Mode::Software => &[Mode::Software, Mode::Gl],
            Mode::Gl => &[Mode::Gl, Mode::Software],
        }
    }

    /// The renderer to try when this one cannot open the window.
    pub fn next(self, first: Mode) -> Option<Mode> {
        let chain = Mode::chain(first);
        let at = chain.iter().position(|m| *m == self)?;
        chain.get(at + 1).copied()
    }
}

/// What the adapter choice knows of one adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub name: String,
    pub kind: eframe::wgpu::DeviceType,
    /// Whether it can draw on the window.
    pub surface: bool,
}

/// The adapter a start in `mode` draws with (an index into `list`), and
/// for each adapter why it was taken or left, for the log. `wanted` names
/// the graphics card chosen in Options (empty: the fastest). Hardware
/// mode never takes a software adapter: the start fails instead and the
/// next renderer is tried (#616).
pub fn choose(mode: Mode, wanted: &str, list: &[Candidate]) -> (Option<usize>, Vec<&'static str>) {
    let software = |c: &Candidate| c.kind == eframe::wgpu::DeviceType::Cpu;
    let fits = |c: &Candidate| {
        c.surface
            && match mode {
                Mode::Software => software(c),
                _ => !software(c),
            }
    };
    let named = (!wanted.is_empty())
        .then(|| list.iter().position(|c| fits(c) && c.name == wanted))
        .flatten();
    let pick = named.or_else(|| {
        list.iter()
            .enumerate()
            .filter(|(_, c)| fits(c))
            .min_by_key(|(_, c)| rank(c.kind))
            .map(|(i, _)| i)
    });
    let notes = list
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if !c.surface {
                "rejected: cannot draw on the window"
            } else if mode == Mode::Software && !software(c) {
                "rejected: not a software adapter (hardware acceleration is off)"
            } else if mode != Mode::Software && software(c) {
                "rejected: software adapter (hardware acceleration wants a graphics card)"
            } else if Some(i) == pick && named.is_some() {
                "picked: the card chosen in Options"
            } else if Some(i) == pick {
                "picked: the fastest usable"
            } else {
                "usable, not picked"
            }
        })
        .collect();
    (pick, notes)
}

static IN_USE: OnceLock<String> = OnceLock::new();

/// The renderer the window of this run opened with.
static MODE_IN_USE: OnceLock<Mode> = OnceLock::new();

/// The window opened, but the frames the renderer drew came back nearly
/// black when read (#616): the driver takes every call and shows nothing.
/// Noted so that the start goes on with the next renderer once the
/// window closes (see `frame_looks_blind`).
static BLIND: AtomicBool = AtomicBool::new(false);

pub fn mode_in_use() -> Option<Mode> {
    MODE_IN_USE.get().copied()
}

pub fn blind() -> bool {
    BLIND.load(Ordering::SeqCst)
}

pub fn note_blind() {
    BLIND.store(true, Ordering::SeqCst);
}

/// A pixel with a channel at least this bright counts as drawn.
pub const BLIND_BRIGHTNESS: u8 = 40;
/// A frame with fewer drawn pixels than one in this many is nearly
/// black: the reporter's frames of #616 had one in 200 to 500 (stray
/// triangles), a drawn window has more than one in 2 in the light and
/// dark themes and one in 30 in the high-contrast theme with the welcome
/// screen (which is why the probe decides with it, see `renderer_blind`).
pub const BLIND_ONE_IN: usize = 50;
/// The main-window frame (counted from the first) at which the first
/// read-back is asked for.
pub const CHECKED_FRAME: u32 = 4;
/// How many frames after a read-back that came back nearly black the
/// next one is asked for.
pub const RECHECK_FRAMES: u32 = 30;
/// How many read-backs must come back nearly black, with the window in
/// front, before the renderer is blind: one may be of a window another
/// covered (a desktop without composition leaves covered pixels undrawn).
pub const BLIND_READS: u8 = 2;
/// The side, in points, of the probe drawn last at the window's bottom
/// right while a frame is read back, and its distance from the corner.
pub const PROBE_SIDE: f32 = 6.0;

/// Whether a pixel counts as drawn: a channel at [`BLIND_BRIGHTNESS`] or
/// more.
fn bright(c: egui::Color32) -> bool {
    c.r().max(c.g()).max(c.b()) >= BLIND_BRIGHTNESS
}

/// Whether `image`, a frame of the main window read back from the
/// renderer, is nearly black: below its top quarter (the menu bar and the
/// toolbars, the first meshes of a frame, which a failing driver still
/// drew right, #616) fewer than one pixel in [`BLIND_ONE_IN`] is bright.
/// A frame too small to judge passes.
pub fn frame_looks_blind(image: &egui::ColorImage) -> bool {
    let [w, h] = image.size;
    if w < 64 || h < 64 {
        return false;
    }
    let (mut bright_count, mut total) = (0_usize, 0_usize);
    for y in (h / 4..h).step_by(4) {
        for x in (0..w).step_by(4) {
            let Some(c) = image.pixels.get(y * w + x) else {
                continue;
            };
            total += 1;
            if bright(*c) {
                bright_count += 1;
            }
        }
    }
    bright_count * BLIND_ONE_IN < total
}

/// Whether the probe (a bright square drawn last in the frame, see
/// `PROBE_SIDE`) is in `image` around the pixel `[x, y]` its centre lands
/// on: at least five of the nine pixels there are bright.
pub fn probe_drawn(image: &egui::ColorImage, [x, y]: [usize; 2]) -> bool {
    let [w, h] = image.size;
    let mut bright_count = 0;
    for py in y.saturating_sub(1)..=y + 1 {
        for px in x.saturating_sub(1)..=x + 1 {
            if px < w && py < h && image.pixels.get(py * w + px).is_some_and(|c| bright(*c)) {
                bright_count += 1;
            }
        }
    }
    bright_count >= 5
}

/// Whether a frame read back from the renderer says it draws nothing
/// that can be seen (#616): the probe, drawn last, is missing (the
/// reporter's driver drew only the first meshes of a frame right) and
/// the frame is nearly black. A frame with the probe is drawn whatever
/// its brightness (the high-contrast theme is mostly black), and a frame
/// that holds a drawn window but no probe is left alone too.
pub fn renderer_blind(image: &egui::ColorImage, probe: [usize; 2]) -> bool {
    !probe_drawn(image, probe) && frame_looks_blind(image)
}

/// The renderer the start goes on with when this one draws nothing that
/// can be seen; none when this one is the last.
static NEXT: OnceLock<Option<Mode>> = OnceLock::new();

pub fn note_next(next: Option<Mode>) {
    let _ = NEXT.set(next);
}

pub fn next_renderer() -> Option<Mode> {
    NEXT.get().copied().flatten()
}

/// Whether the window of this run opened (the app is being built): a
/// panic before that is a renderer that failed to start, after it one of
/// the app.
static OPENED: AtomicBool = AtomicBool::new(false);

pub fn window_opened() -> bool {
    OPENED.load(Ordering::SeqCst)
}

/// Whether this run handed the window over to a new start with another
/// renderer (`restart_with`): its watch then has nothing left to wait
/// for, or it would start yet another TraceDraw beside the one that
/// opened.
static HANDED_OVER: AtomicBool = AtomicBool::new(false);

fn hand_over() {
    HANDED_OVER.store(true, Ordering::SeqCst);
}

fn handed_over() -> bool {
    HANDED_OVER.load(Ordering::SeqCst)
}

/// The window of this start opened, or a new start took its place.
fn start_settled() -> bool {
    window_opened() || handed_over()
}

/// The file that names the renderer a start is trying, written before
/// the window is opened and removed as soon as it is. A start that finds
/// it knows the last one never got a window with that renderer: the
/// driver crashed the program or hung it, so nothing could hand over to
/// the next renderer (#570).
const ATTEMPT_FILE: &str = "renderer-attempt";

fn attempt_path() -> Option<std::path::PathBuf> {
    crate::settings::config_dir().map(|d| d.join(ATTEMPT_FILE))
}

/// The renderer the last start tried without its window opening.
pub fn failed_attempt() -> Option<Mode> {
    failed_attempt_at(&attempt_path()?)
}

fn failed_attempt_at(path: &std::path::Path) -> Option<Mode> {
    let text = std::fs::read_to_string(path).ok()?;
    Mode::from_name(text.trim())
}

/// Note that this start tries `mode` (see `ATTEMPT_FILE`).
pub fn begin_attempt(mode: Mode) {
    if let Some(path) = attempt_path() {
        begin_attempt_at(&path, mode);
    }
}

fn begin_attempt_at(path: &std::path::Path, mode: Mode) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(path, mode.name()) {
        log::warn!(
            "could not note the renderer tried in {}: {e}",
            path.display()
        );
    }
}

/// The window opened: the attempt worked.
fn end_attempt() {
    if let Some(path) = attempt_path() {
        end_attempt_at(&path);
    }
}

fn end_attempt_at(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
}

/// How long a start waits for its window before it takes the renderer
/// for hung (#570). Opening it takes a second or two, a software adapter
/// on an old computer a few more.
pub const WINDOW_WAIT: std::time::Duration = std::time::Duration::from_secs(20);

/// What a start does once it has waited `waited` for its window.
#[derive(Debug, PartialEq, Eq)]
pub enum Watch {
    /// The window is open, or there is time left.
    Wait,
    /// Start again with this renderer and end this start.
    Next(Mode),
    /// The last renderer: keep waiting, it may only be slow.
    Last,
}

pub fn watch(opened: bool, waited: std::time::Duration, next: Option<Mode>) -> Watch {
    if opened || waited < WINDOW_WAIT {
        return Watch::Wait;
    }
    next.map_or(Watch::Last, Watch::Next)
}

/// Watch this start: a renderer that hangs inside the graphics driver
/// never returns to hand over to the next one, so after `WINDOW_WAIT`
/// without a window the next renderer is started from here and this
/// process ends. `decline` makes this start refuse files from later starts
/// first, so the new one opens its own window (#570).
pub fn watch_start(
    mode: Mode,
    next: Option<Mode>,
    args: Vec<String>,
    decline: Option<Arc<AtomicBool>>,
) {
    let spawned = std::thread::Builder::new()
        .name("tracedraw-start-watch".into())
        .spawn(move || {
            let start = web_time::Instant::now();
            loop {
                match watch(start_settled(), start.elapsed(), next) {
                    Watch::Wait if start_settled() => return,
                    Watch::Wait => std::thread::sleep(std::time::Duration::from_millis(100)),
                    Watch::Last => {
                        log::warn!(
                            "no window after {}s with {mode:?}, the last renderer; still waiting",
                            WINDOW_WAIT.as_secs()
                        );
                        return;
                    }
                    Watch::Next(_) if start_settled() => return,
                    Watch::Next(n) => {
                        log::warn!(
                            "no window after {}s with {mode:?}; starting again with {n:?}",
                            WINDOW_WAIT.as_secs()
                        );
                        if let Some(d) = &decline {
                            d.store(true, Ordering::SeqCst);
                        }
                        if spawn_with(n, &args) {
                            std::process::exit(0);
                        }
                        log::error!("could not start TraceDraw again with {n:?}");
                        return;
                    }
                }
            }
        });
    if let Err(e) = spawned {
        log::warn!("the start is not watched: {e}");
    }
}

/// What draws the window in this run, as Options shows it.
pub fn in_use() -> Option<&'static str> {
    IN_USE.get().map(String::as_str)
}

/// Note what the window was opened with; a renderer other than the
/// first choice is remembered, so the next start goes straight to it.
pub fn note_renderer(cc: &eframe::CreationContext<'_>, mode: Mode) {
    OPENED.store(true, Ordering::SeqCst);
    let _ = MODE_IN_USE.set(mode);
    end_attempt();
    crate::startup_log::quiet_panics(false);
    let mut s = crate::settings::Settings::load();
    let first = Mode::first(&s);
    let remember = if mode == first { "" } else { mode.name() };
    if s.working_renderer != remember {
        s.working_renderer = remember.to_string();
        s.save();
    }
    let what = match cc.wgpu_render_state.as_ref() {
        Some(rs) => {
            let i = rs.adapter.get_info();
            format!("{} ({:?})", i.name, i.backend)
        }
        None => "OpenGL".to_string(),
    };
    if let Some(gl) = cc.gl.as_ref() {
        use eframe::glow::HasContext;
        // SAFETY: plain queries on the current context eframe just made.
        let [vendor, renderer, version, glsl] = [
            eframe::glow::VENDOR,
            eframe::glow::RENDERER,
            eframe::glow::VERSION,
            eframe::glow::SHADING_LANGUAGE_VERSION,
        ]
        .map(|p| unsafe { gl.get_parameter_string(p) });
        log::info!("OpenGL: {vendor}, {renderer}, version {version}, GLSL {glsl}");
    }
    log::info!("the window opened with {mode:?}: {what}");
    let _ = IN_USE.set(what);
}

/// The graphics adapters of this computer by name, software ones last;
/// looked for once.
pub fn adapters() -> &'static [String] {
    static LIST: OnceLock<Vec<String>> = OnceLock::new();
    LIST.get_or_init(|| {
        if cfg!(test) {
            return Vec::new();
        }
        let instance = eframe::wgpu::Instance::new(
            eframe::wgpu::InstanceDescriptor::new_without_display_handle_from_env(),
        );
        let found =
            pollster::block_on(instance.enumerate_adapters(eframe::wgpu::Backends::PRIMARY));
        let mut list: Vec<(u8, String)> = found
            .iter()
            .map(|a| {
                let i = a.get_info();
                (rank(i.device_type), i.name)
            })
            .collect();
        list.sort();
        let mut names: Vec<String> = Vec::new();
        for (_, n) in list {
            if !names.contains(&n) {
                names.push(n);
            }
        }
        names
    })
}

/// The order adapters are preferred in: separate cards, built-in ones,
/// virtual ones, others, then software.
fn rank(t: eframe::wgpu::DeviceType) -> u8 {
    use eframe::wgpu::DeviceType;
    match t {
        DeviceType::DiscreteGpu => 0,
        DeviceType::IntegratedGpu => 1,
        DeviceType::VirtualGpu => 2,
        DeviceType::Other => 3,
        DeviceType::Cpu => 4,
    }
}

/// Whether a software adapter draws the window of this run (#616): every
/// frame is then rasterized on the CPU, so the window paces its frames,
/// casts no shadows and shows the system's pointers.
static SOFTWARE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The software adapter's name, until the status bar has said it once.
static SOFTWARE_NOTE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

pub fn software() -> bool {
    SOFTWARE.load(std::sync::atomic::Ordering::Relaxed)
}

/// The name of the software adapter drawing the window, once: the first
/// call after the window opened on one, then never again.
pub fn take_software_note() -> Option<String> {
    if !software() {
        return None;
    }
    SOFTWARE_NOTE.lock().ok().and_then(|mut n| n.take())
}

/// Whether an adapter or OpenGL renderer `name` rasterizes on the CPU:
/// WARP, the GDI OpenGL 1.1 of a Windows without a card driver, Mesa's
/// llvmpipe, softpipe and lavapipe, SwiftShader.
pub fn is_software_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    [
        "basic render",
        "gdi generic",
        "llvmpipe",
        "softpipe",
        "lavapipe",
        "swiftshader",
    ]
    .iter()
    .any(|s| n.contains(s))
}

/// Note whether the window opened on a software adapter.
fn note_software(cc: &eframe::CreationContext<'_>) {
    let found = if let Some(rs) = cc.wgpu_render_state.as_ref() {
        let i = rs.adapter.get_info();
        (i.device_type == eframe::wgpu::DeviceType::Cpu || is_software_name(&i.name))
            .then_some(i.name)
    } else if let Some(gl) = cc.gl.as_ref() {
        use eframe::glow::HasContext;
        // SAFETY: a plain query on the current context eframe just made.
        let name = unsafe { gl.get_parameter_string(eframe::glow::RENDERER) };
        is_software_name(&name).then_some(name)
    } else {
        None
    };
    if let Some(name) = found {
        log::warn!("a software adapter draws the window ({name}): frames paced to {FRAME_CAP} a second, no shadows, system pointers");
        SOFTWARE.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut n) = SOFTWARE_NOTE.lock() {
            *n = Some(name);
        }
    }
}

/// Frames a second on a software adapter (#616).
pub const FRAME_CAP: u32 = 30;

/// How long a frame that begins at `now` waits so that frames begin at
/// least `gap` apart, the last one having begun at `last`.
pub fn frame_wait(
    last: Option<web_time::Instant>,
    now: web_time::Instant,
    gap: std::time::Duration,
) -> std::time::Duration {
    last.map(|l| gap.saturating_sub(now.saturating_duration_since(l)))
        .unwrap_or_default()
}

/// Keeps a software adapter to [`FRAME_CAP`] frames a second: every
/// pointer move is a frame, and each costs a full-window rasterization on
/// the CPU, so without a cap the window takes every core and input queues
/// up. The pointer moves in between are merged into the next frame.
#[derive(Default)]
pub struct Pacer {
    last: Option<web_time::Instant>,
}

impl Pacer {
    /// The pacer of a window just opened, noting whether a software
    /// adapter draws it.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        note_software(cc);
        Pacer::default()
    }

    /// At the start of a frame: wait as long as the cap asks (never on a
    /// graphics card).
    pub fn pace(&mut self) {
        if !software() {
            return;
        }
        let gap = std::time::Duration::from_secs(1) / FRAME_CAP;
        let wait = frame_wait(self.last, web_time::Instant::now(), gap);
        if !wait.is_zero() {
            std::thread::sleep(wait);
        }
        self.last = Some(web_time::Instant::now());
    }
}

#[cfg(test)]
mod software_tests {
    use super::*;

    /// The renderers that draw on the CPU are known by name, whatever
    /// the case (#616).
    #[test]
    fn software_renderers_are_recognised() {
        for n in [
            "Microsoft Basic Render Driver",
            "GDI Generic",
            "llvmpipe (LLVM 15.0.7, 256 bits)",
            "softpipe",
            "Google SwiftShader",
            "lavapipe",
        ] {
            assert!(is_software_name(n), "{n}");
        }
        for n in [
            "Intel(R) HD Graphics 4000",
            "NVIDIA GeForce GT 730",
            "AMD Radeon HD 6450",
        ] {
            assert!(!is_software_name(n), "{n}");
        }
    }

    /// Frames begin at least the gap apart: one that comes sooner waits
    /// the rest, a later one waits nothing (#616).
    #[test]
    fn frames_are_paced_to_the_cap() {
        let gap = std::time::Duration::from_secs(1) / FRAME_CAP;
        let t0 = web_time::Instant::now();
        let ms = std::time::Duration::from_millis;
        assert_eq!(frame_wait(None, t0, gap), std::time::Duration::ZERO);
        assert_eq!(frame_wait(Some(t0), t0, gap), gap);
        assert_eq!(frame_wait(Some(t0), t0 + ms(10), gap), gap - ms(10));
        assert_eq!(
            frame_wait(Some(t0), t0 + gap, gap),
            std::time::Duration::ZERO
        );
        assert_eq!(
            frame_wait(Some(t0), t0 + ms(500), gap),
            std::time::Duration::ZERO
        );
        // A clock that went back waits at most one gap.
        assert_eq!(frame_wait(Some(t0 + ms(5)), t0, gap), gap);
        // The cap holds over a run of frames that come at once: 30
        // frames a second take at least a second.
        let mut last = None;
        let mut now = t0;
        for _ in 0..FRAME_CAP {
            now += frame_wait(last, now, gap);
            last = Some(now);
        }
        assert!(now - t0 >= gap * (FRAME_CAP - 1));
        // A graphics card is never paced.
        assert!(!software());
        let before = web_time::Instant::now();
        let mut p = Pacer::default();
        p.pace();
        p.pace();
        assert!(before.elapsed() < gap);
    }
}

/// The renderer and the wgpu set-up of a start in `mode`; `wanted` names
/// the graphics card chosen (empty: the fastest).
pub fn options(
    mode: Mode,
    wanted: &str,
) -> (eframe::Renderer, eframe::egui_wgpu::WgpuConfiguration) {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
    if mode == Mode::Gl {
        return (eframe::Renderer::Glow, WgpuConfiguration::default());
    }
    let wanted = wanted.to_string();
    let mut setup = WgpuSetupCreateNew::without_display_handle();
    setup.native_adapter_selector = Some(Arc::new(move |list, surface| {
        let candidates: Vec<Candidate> = list
            .iter()
            .map(|a| {
                let i = a.get_info();
                Candidate {
                    name: i.name,
                    kind: i.device_type,
                    surface: surface.is_none_or(|s| a.is_surface_supported(s)),
                }
            })
            .collect();
        let (pick, notes) = choose(mode, &wanted, &candidates);
        log::info!("{} graphics adapter(s) for {mode:?}:", list.len());
        for (a, note) in list.iter().zip(&notes) {
            let i = a.get_info();
            log::info!(
                "  {} ({:?}, {:?}, {:04x}:{:04x}, driver {} {}): {note}",
                i.name,
                i.backend,
                i.device_type,
                i.vendor,
                i.device,
                i.driver,
                i.driver_info
            );
        }
        match pick.and_then(|i| list.get(i)) {
            Some(a) => Ok(a.clone()),
            None if mode == Mode::Software => {
                Err("no software adapter can draw the window".to_string())
            }
            None => Err(
                "no graphics card can draw the window (a software adapter is left to the last renderer)"
                    .to_string(),
            ),
        }
    }));
    let config = WgpuConfiguration {
        // One frame queued, not two: what is drawn under the pointer (a
        // shape being drawn, a drawn pointer) trails it by a frame less
        // (#655).
        surface: eframe::egui_wgpu::SurfaceConfig::LOW_LATENCY,
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    };
    (eframe::Renderer::Wgpu, config)
}

/// The arguments of a start with `next`, the other arguments kept.
fn args_with(next: Mode, args: &[String]) -> Vec<String> {
    let mut rest: Vec<String> = args
        .iter()
        .filter(|a| !a.starts_with(ARG))
        .cloned()
        .collect();
    rest.push(next.arg());
    rest
}

/// Run TraceDraw again with `next`, the other arguments kept, until it
/// ends; false when it did not start.
pub fn restart_with(next: Mode, args: &[String]) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    hand_over();
    std::process::Command::new(exe)
        .args(args_with(next, args))
        .status()
        .is_ok()
}

/// Start TraceDraw again with `next` without waiting for it; false when
/// it did not start.
fn spawn_with(next: Mode, args: &[String]) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    std::process::Command::new(exe)
        .args(args_with(next, args))
        .spawn()
        .is_ok()
}

/// The patched eframe's context fallback with its tests, run here: the
/// copy in vendor/ is outside the workspace (vendor/README.md, #616).
#[cfg(test)]
#[path = "../../../vendor/eframe/src/native/first_context.rs"]
mod eframe_context_fallback;

/// The arithmetic of the patched eframe's fixed-function painter with its
/// tests, run here for the same reason (#616).
#[cfg(test)]
#[path = "../../../vendor/eframe/src/native/fixed_function_geometry.rs"]
mod eframe_fixed_function_geometry;

#[cfg(test)]
mod tests {
    use super::*;

    /// The OpenGL renderer asks for a compatibility context of any version
    /// when OpenGL 3.3 core and OpenGL ES fail, so a card with OpenGL 2.1
    /// only opens the window on its own driver (#616). That needs the
    /// patched eframe (vendor/README.md) until egui 0.37.
    #[test]
    fn opengl_falls_back_to_a_compatibility_context() {
        let manifest = include_str!("../../../Cargo.toml");
        assert!(manifest.contains("eframe = { path = \"vendor/eframe\" }"));
        let glow = include_str!("../../../vendor/eframe/src/native/glow_integration.rs");
        let tried = glow
            .split("fn context_attributes_to_try(")
            .nth(1)
            .expect("the attempts");
        let compat = tried.find(".with_profile(GlProfile::Compatibility)");
        let gles = tried.find("ContextApi::Gles(None)");
        assert!(compat.is_some() && gles < compat, "compatibility last");
        assert!(glow.contains("create_first_context("));
        // The fixed-function painter takes desktop OpenGL below 3.0, where
        // egui_glow's shaders and buffers draw wrongly on older drivers.
        assert!(glow.contains("GlPainter::new("));
        let painter = include_str!("../../../vendor/eframe/src/native/fixed_function_painter.rs");
        assert!(painter.contains("glVertexPointer") && painter.contains("GL_MODULATE"));
        let choice = include_str!("../../../vendor/eframe/src/native/gl_painter.rs");
        assert!(
            choice.contains("fixed_function_wanted(") && choice.contains("TRACEDRAW_GL_PAINTER")
        );
    }

    /// A main-window frame read back from the renderer is blind when
    /// nearly nothing below the menu bar is drawn: the reporter's driver
    /// drew the first meshes right and the rest nearly black (#616).
    #[test]
    fn a_nearly_black_frame_is_a_blind_renderer() {
        use egui::{Color32, ColorImage};
        let filled = |w, h, c| ColorImage::filled([w, h], c);
        // The light theme: everything bright.
        assert!(!frame_looks_blind(&filled(
            640,
            400,
            Color32::from_gray(245)
        )));
        // The dark theme without a page: dark panels with text on them.
        let mut dark = filled(640, 400, Color32::from_gray(27));
        for y in (0..400).step_by(8) {
            for x in (0..640).step_by(8) {
                dark[(x, y)] = Color32::from_gray(200);
            }
        }
        assert!(!frame_looks_blind(&dark));
        // The reporter's frame: a right menu bar, the rest nearly black.
        let mut broken = filled(1920, 1017, Color32::from_rgb(0, 8, 8));
        for y in 0..60 {
            for x in 0..1920 {
                broken[(x, y)] = Color32::from_gray(240);
            }
        }
        assert!(frame_looks_blind(&broken));
        assert!(frame_looks_blind(&filled(800, 600, Color32::BLACK)));
        // A few drawn pixels are not enough: one in 50 is the line. The
        // reporter's recording had one bright pixel in 400 to 500 on a
        // 26-grey ground (one drawn pixel in 80 by 80 is one in 400 of
        // the samples); one in 20 by 20 is one in 25.
        let mut sparse = filled(800, 600, Color32::from_gray(26));
        for y in (150..600).step_by(80) {
            for x in (0..800).step_by(80) {
                sparse[(x, y)] = Color32::WHITE;
            }
        }
        assert!(frame_looks_blind(&sparse));
        let mut enough = filled(800, 600, Color32::BLACK);
        for y in (150..600).step_by(20) {
            for x in (0..800).step_by(20) {
                enough[(x, y)] = Color32::WHITE;
            }
        }
        assert!(!frame_looks_blind(&enough));
        // Too small to judge.
        assert!(!frame_looks_blind(&filled(32, 32, Color32::BLACK)));
    }

    /// The probe drawn last decides with the frame: a frame with it was
    /// drawn however dark (the high-contrast theme), one without it that
    /// is nearly black was not (#616).
    #[test]
    fn the_probe_decides_with_the_frame() {
        use egui::{Color32, ColorImage};
        let probe = [800 - 9, 600 - 9];
        let mut black = ColorImage::filled([800, 600], Color32::BLACK);
        assert!(!probe_drawn(&black, probe));
        assert!(renderer_blind(&black, probe));
        // The probe, 6 by 6 px around its centre.
        for y in probe[1] - 3..probe[1] + 3 {
            for x in probe[0] - 3..probe[0] + 3 {
                black[(x, y)] = Color32::from_gray(220);
            }
        }
        assert!(probe_drawn(&black, probe));
        assert!(frame_looks_blind(&black));
        assert!(!renderer_blind(&black, probe));
        // The high-contrast welcome screen: black with sparse text, no
        // probe drawn (its last mesh was lost): not a blind renderer.
        let mut contrast = ColorImage::filled([800, 600], Color32::BLACK);
        for y in (150..600).step_by(20) {
            for x in (0..800).step_by(20) {
                contrast[(x, y)] = Color32::WHITE;
            }
        }
        assert!(!renderer_blind(&contrast, probe));
        // A probe at the very corner is judged from the pixels that exist.
        let mut corner = ColorImage::filled([800, 600], Color32::BLACK);
        for y in 598..600 {
            for x in 798..800 {
                corner[(x, y)] = Color32::WHITE;
            }
        }
        assert!(!probe_drawn(&corner, [799, 599]));
        assert!(!probe_drawn(&corner, [0, 0]));
        for y in 597..600 {
            for x in 797..800 {
                corner[(x, y)] = Color32::WHITE;
            }
        }
        assert!(probe_drawn(&corner, [798, 598]));
        assert!(!probe_drawn(&corner, [900, 700]));
    }

    #[test]
    fn a_renderer_that_fails_hands_over_to_the_next() {
        let args = vec!["drawing.cdr".to_string()];
        assert_eq!(Mode::from_args(&args), None);
        let mut m = Mode::Hardware;
        let mut seen = vec![m];
        while let Some(n) = m.next(Mode::Hardware) {
            let mut with = args.clone();
            with.push(n.arg());
            assert_eq!(Mode::from_args(&with), Some(n));
            seen.push(n);
            m = n;
        }
        // OpenGL before the software adapter, which draws on the CPU and
        // always opens, so it comes last (#616).
        assert_eq!(seen, vec![Mode::Hardware, Mode::Gl, Mode::Software]);
        // Hardware acceleration off: the software adapter, then OpenGL.
        assert_eq!(Mode::Software.next(Mode::Software), Some(Mode::Gl));
        assert_eq!(Mode::Gl.next(Mode::Software), None);
        // Remembered OpenGL that fails falls to the software adapter.
        assert_eq!(Mode::Gl.next(Mode::Gl), Some(Mode::Software));
        assert_eq!(Mode::Software.next(Mode::Gl), None);
        // The renderer that opened the window last time, unless it is the
        // first choice anyway; hardware acceleration off starts in software.
        let mut s = crate::settings::Settings::default();
        assert_eq!(Mode::for_start(&args, &s, None), Mode::Hardware);
        s.working_renderer = "gl".into();
        assert_eq!(Mode::for_start(&args, &s, None), Mode::Gl);
        s.working_renderer.clear();
        s.hardware_acceleration = false;
        assert_eq!(Mode::for_start(&args, &s, None), Mode::Software);
        assert_eq!(Mode::for_start(&[Mode::Gl.arg()], &s, None), Mode::Gl);
        assert_eq!(options(Mode::Gl, "").0, eframe::Renderer::Glow);
        assert_eq!(options(Mode::Hardware, "").0, eframe::Renderer::Wgpu);
    }

    /// A start whose window never opened (the driver crashed or hung the
    /// program, so it could not hand over) leaves its renderer noted; the
    /// next start goes on with the renderer after it, and an opened window
    /// clears the note (#570).
    #[test]
    fn a_start_that_never_opened_its_window_is_followed_by_the_next_renderer() {
        let dir = std::env::temp_dir().join(format!(
            "tracedraw-attempt-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let path = dir.join(ATTEMPT_FILE);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(failed_attempt_at(&path), None);
        begin_attempt_at(&path, Mode::Hardware);
        assert_eq!(failed_attempt_at(&path), Some(Mode::Hardware));
        let s = crate::settings::Settings::default();
        let args: Vec<String> = Vec::new();
        let failed = failed_attempt_at(&path);
        assert_eq!(Mode::for_start(&args, &s, failed), Mode::Gl);
        // OpenGL dies the same way: the software adapter.
        assert_eq!(Mode::for_start(&args, &s, Some(Mode::Gl)), Mode::Software);
        // The last of the chain: back to the first choice.
        assert_eq!(
            Mode::for_start(&args, &s, Some(Mode::Software)),
            Mode::Hardware
        );
        // A hand-over names its renderer, which wins over the note.
        assert_eq!(
            Mode::for_start(&[Mode::Software.arg()], &s, failed),
            Mode::Software
        );
        // The window opened: nothing noted any more.
        end_attempt_at(&path);
        assert_eq!(failed_attempt_at(&path), None);
        // A note that is not a renderer is ignored.
        std::fs::write(&path, "nonsense").expect("write");
        assert_eq!(failed_attempt_at(&path), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A renderer that fails at once hands the window over to a new start
    /// with the next one, and this start waits for it: its watch must
    /// count that as settled, or `WINDOW_WAIT` later it starts a second
    /// TraceDraw beside the first (found while checking #741).
    #[test]
    fn a_start_that_handed_over_is_not_started_again() {
        assert!(!handed_over());
        hand_over();
        assert!(start_settled());
        assert_eq!(
            watch(start_settled(), WINDOW_WAIT * 2, Some(Mode::Gl)),
            Watch::Wait
        );
    }

    /// A start whose window does not open in `WINDOW_WAIT` (the driver
    /// hung it in wgpu's set-up) starts the next renderer itself, so the
    /// first start opens, not only the one after it; the last renderer is
    /// left to finish. The new start keeps the files and swaps the switch
    /// (#570).
    #[test]
    fn a_start_hung_without_its_window_goes_on_with_the_next_renderer() {
        use std::time::Duration;
        let early = WINDOW_WAIT - Duration::from_millis(1);
        assert_eq!(watch(false, early, Some(Mode::Gl)), Watch::Wait);
        assert_eq!(watch(true, WINDOW_WAIT * 2, Some(Mode::Gl)), Watch::Wait);
        assert_eq!(
            watch(false, WINDOW_WAIT, Some(Mode::Gl)),
            Watch::Next(Mode::Gl)
        );
        assert_eq!(watch(false, WINDOW_WAIT, None), Watch::Last);
        let args = vec!["a.cdr".to_string(), Mode::Hardware.arg()];
        assert_eq!(
            args_with(Mode::Gl, &args),
            vec!["a.cdr".to_string(), Mode::Gl.arg()]
        );
    }

    fn adapter(name: &str, kind: eframe::wgpu::DeviceType, surface: bool) -> Candidate {
        Candidate {
            name: name.into(),
            kind,
            surface,
        }
    }

    /// Hardware mode takes the card named in Options, else the fastest
    /// card, and never the software adapter, which only a start with
    /// hardware acceleration off (or the last renderer) uses (#616).
    #[test]
    fn hardware_mode_never_draws_on_the_software_adapter() {
        use eframe::wgpu::DeviceType::*;
        let warp = adapter("Microsoft Basic Render Driver", Cpu, true);
        // Only the software adapter: hardware mode fails, so the start
        // goes on to OpenGL.
        let only = [warp.clone()];
        let (pick, notes) = choose(Mode::Hardware, "", &only);
        assert_eq!(pick, None);
        assert!(notes[0].starts_with("rejected: software adapter"));
        assert_eq!(choose(Mode::Software, "", &only).0, Some(0));
        let list = [
            warp.clone(),
            adapter("Built-in", IntegratedGpu, true),
            adapter("Card", DiscreteGpu, true),
            adapter("Elsewhere", DiscreteGpu, false),
        ];
        let (pick, notes) = choose(Mode::Hardware, "", &list);
        assert_eq!(pick, Some(2));
        assert_eq!(notes[2], "picked: the fastest usable");
        assert_eq!(notes[1], "usable, not picked");
        assert_eq!(notes[3], "rejected: cannot draw on the window");
        let (pick, notes) = choose(Mode::Hardware, "Built-in", &list);
        assert_eq!(pick, Some(1));
        assert_eq!(notes[1], "picked: the card chosen in Options");
        // A chosen card that is gone or cannot draw: the fastest.
        assert_eq!(choose(Mode::Hardware, "Elsewhere", &list).0, Some(2));
        // Naming the software adapter does not let hardware mode use it.
        let named = choose(Mode::Hardware, "Microsoft Basic Render Driver", &only);
        assert_eq!(named.0, None);
        let (pick, notes) = choose(Mode::Software, "", &list);
        assert_eq!(pick, Some(0));
        assert!(notes[2].starts_with("rejected: not a software adapter"));
        assert_eq!(choose(Mode::Hardware, "", &[]).0, None);
    }
}
