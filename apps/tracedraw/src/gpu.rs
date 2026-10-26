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

/// Whether the window of this run opened (the app is being built): a
/// panic before that is a renderer that failed to start, after it one of
/// the app.
static OPENED: AtomicBool = AtomicBool::new(false);

pub fn window_opened() -> bool {
    OPENED.load(Ordering::SeqCst)
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
                match watch(window_opened(), start.elapsed(), next) {
                    Watch::Wait if window_opened() => return,
                    Watch::Wait => std::thread::sleep(std::time::Duration::from_millis(100)),
                    Watch::Last => {
                        log::warn!(
                            "no window after {}s with {mode:?}, the last renderer; still waiting",
                            WINDOW_WAIT.as_secs()
                        );
                        return;
                    }
                    Watch::Next(_) if window_opened() => return,
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
