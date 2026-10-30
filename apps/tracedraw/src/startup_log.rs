//! What a start leaves behind when it goes wrong. The Windows build has
//! no console, so the log and a panic went nowhere and a start that
//! failed showed nothing at all (#570). The log goes to `tracedraw.log`
//! in the settings folder as well (written afresh each start), and a
//! panic is written to `crash.log` there and shown in a message box.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// While a renderer starts and another is left to try, a panic is that
/// renderer failing (eframe's OpenGL set-up can panic, #616): it is
/// logged, and neither written to `crash.log` nor shown.
static QUIET: AtomicBool = AtomicBool::new(false);

pub fn quiet_panics(on: bool) {
    QUIET.store(on, Ordering::SeqCst);
}

/// The log file of this run, in `dir`.
pub fn log_path(dir: &Path) -> PathBuf {
    dir.join("tracedraw.log")
}

/// Where a panic is written, in `dir`.
pub fn crash_path(dir: &Path) -> PathBuf {
    dir.join("crash.log")
}

/// Every log line to stderr and to the file.
struct Tee {
    file: Option<std::fs::File>,
}

impl Write for Tee {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = std::io::stderr().write_all(buf);
        if let Some(f) = self.file.as_mut() {
            let _ = f.write_all(buf);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let _ = std::io::stderr().flush();
        if let Some(f) = self.file.as_mut() {
            let _ = f.flush();
        }
        Ok(())
    }
}

/// Start the log (stderr and the file) and the panic report.
pub fn init() {
    let dir = crate::settings::config_dir();
    // A start handed over to the next renderer adds to the log of the
    // one that failed.
    let restarted = std::env::args().any(|a| a.starts_with("--renderer="));
    let file = dir.as_ref().and_then(|d| {
        let _ = std::fs::create_dir_all(d);
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(restarted)
            .truncate(!restarted)
            .open(log_path(d))
            .ok()
    });
    crate::frame_stats::mark_start();
    // wgpu's own lines on which backends started and which adapters it
    // left out come at debug; they are few and only at the start (#616).
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu_core::instance=debug"),
    )
    .target(env_logger::Target::Pipe(Box::new(Tee { file })))
    .init();
    log::info!(
        "TraceDraw {} on {} {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    log::info!("{}", system());
    let dir = Mutex::new(dir);
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let report = report(info);
        if QUIET.load(Ordering::SeqCst) {
            log::warn!("the renderer failed to start: {report}");
            return;
        }
        log::error!("{report}");
        let saved = dir.lock().ok().and_then(|d| d.clone()).and_then(|d| {
            let path = crash_path(&d);
            std::fs::write(&path, &report).ok().map(|_| path)
        });
        default(info);
        show(&report, saved.as_deref());
    }));
}

/// The computer, for the log (#616): the processor and how many threads
/// it runs, and on Windows the build.
fn system() -> String {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let cpu = std::env::var("PROCESSOR_IDENTIFIER")
        .ok()
        .or_else(cpu_model)
        .unwrap_or_else(|| "unknown processor".into());
    let mut text = format!("{cpu}, {threads} threads");
    if let Some(v) = windows_version() {
        text.push_str(&format!(", Windows {v}"));
    }
    text
}

/// The processor's name from `/proc/cpuinfo` (Linux).
fn cpu_model() -> Option<String> {
    let info = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    info.lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.trim().to_string())
}

/// The Windows version and build, as the system reports it (not the
/// compatibility version an application manifest would get).
#[cfg(windows)]
fn windows_version() -> Option<String> {
    #[repr(C)]
    struct OsVersionInfo {
        size: u32,
        major: u32,
        minor: u32,
        build: u32,
        platform: u32,
        service_pack: [u16; 128],
    }
    #[link(name = "ntdll", kind = "raw-dylib")]
    unsafe extern "system" {
        fn RtlGetVersion(info: *mut OsVersionInfo) -> i32;
    }
    let mut info = OsVersionInfo {
        size: std::mem::size_of::<OsVersionInfo>() as u32,
        major: 0,
        minor: 0,
        build: 0,
        platform: 0,
        service_pack: [0; 128],
    };
    // SAFETY: the structure is OSVERSIONINFOW with its size filled in, as
    // RtlGetVersion expects; it only writes into it.
    let status = unsafe { RtlGetVersion(&mut info) };
    (status == 0).then(|| format!("{}.{} build {}", info.major, info.minor, info.build))
}

#[cfg(not(windows))]
fn windows_version() -> Option<String> {
    None
}

/// The panic as text: where and what, and the version.
pub fn report(info: &std::panic::PanicHookInfo<'_>) -> String {
    let what = info
        .payload()
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown".into());
    let place = info
        .location()
        .map(|l| format!("{}:{}", l.file(), l.line()))
        .unwrap_or_default();
    format!(
        "TraceDraw {} ({} {}) stopped: {what} at {place}\n{}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::backtrace::Backtrace::force_capture()
    )
}

/// Tell the user that no renderer could open the window, naming the log
/// (#616): the Windows build has no console, so they would see nothing.
pub fn no_window(reason: &str) {
    let log = crate::settings::config_dir()
        .map(|d| log_path(&d).display().to_string())
        .unwrap_or_default();
    let text = crate::i18n::trf("dialog.no_window", &[("reason", reason), ("log", &log)]);
    message(text);
}

/// An error box, with the system's dialogs; Android has none, so the
/// system log gets it (`adb logcat -s tracedraw`).
fn message(text: String) {
    #[cfg(not(target_os = "android"))]
    let _ = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("TraceDraw")
        .set_description(text)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    #[cfg(target_os = "android")]
    log::error!("{text}");
}

/// Tell the user, who would otherwise see nothing (no console on
/// Windows). Once: a panic while showing it shows nothing more.
fn show(report: &str, saved: Option<&Path>) {
    static SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if SHOWN.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    let first = report.lines().next().unwrap_or_default();
    let mut text = format!("{first}\n\n");
    if let Some(p) = saved {
        text.push_str(&format!(
            "The details are in {}. Please attach that file to an issue at https://github.com/igtoth/tracedraw/issues",
            p.display()
        ));
    }
    message(text);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The files sit in the settings folder; a panic's report says the
    /// version, what happened and where.
    #[test]
    fn the_log_and_the_crash_report_go_in_the_settings_folder() {
        let d = Path::new("/somewhere/TraceDraw");
        assert_eq!(log_path(d), d.join("tracedraw.log"));
        assert_eq!(crash_path(d), d.join("crash.log"));
        let seen = std::sync::Arc::new(Mutex::new(String::new()));
        let keep = seen.clone();
        let before = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // Other tests may panic meanwhile: only this one counts.
            let r = report(info);
            if r.contains("no adapter") {
                *keep.lock().unwrap() = r;
            }
        }));
        let _ = std::panic::catch_unwind(|| panic!("no adapter"));
        std::panic::set_hook(before);
        let text = seen.lock().unwrap().clone();
        assert!(text.contains("no adapter"), "{text}");
        assert!(text.contains(env!("CARGO_PKG_VERSION")));
        assert!(text.contains("startup_log.rs"));
    }

    /// The log names the processor and its threads (#616).
    #[test]
    fn the_log_names_the_computer() {
        let s = system();
        assert!(s.contains(" threads"), "{s}");
        assert!(!s.starts_with(", "), "{s}");
    }
}
