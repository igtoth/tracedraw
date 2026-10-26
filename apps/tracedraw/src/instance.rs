//! One window for every start (#646). A start that finds TraceDraw running
//! hands the files it was given to that window, which opens them in tabs of
//! their own, and ends; started with no file it opens a window of its own,
//! as `--new-instance` always does.
//!
//! The first start holds a lock on `instance.lock` in the settings folder
//! for as long as it runs (the system lets go of it when the process ends,
//! a crash too, so nothing stale is left in the way) and listens on a port
//! of this computer that `instance.json` names with a token only this user
//! can read. A later start sends one line, `{"token":…,"files":[…]}`, and
//! the window answers `{"ok":true}`. The window only takes the paths in
//! between frames, from a channel ([`open_handed_over`]): one `try_recv` a
//! frame when nothing came.

use crate::app::App;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// The file the running window holds a lock on.
pub const LOCK_FILE: &str = "instance.lock";
/// Where the running window listens: port, token and process.
pub const INFO_FILE: &str = "instance.json";
/// The longest request read, in bytes.
const MAX_REQUEST: u64 = 1 << 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Info {
    port: u16,
    token: String,
    pid: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    token: String,
    files: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Reply {
    ok: bool,
}

/// What a start is.
pub enum Claim {
    /// The first: it keeps the window that later starts hand files to.
    Primary(Primary),
    /// Its files went to the running window: it ends.
    HandedOver,
    /// A window of its own (`--new-instance`, no files given, the running
    /// window could not be reached, no settings folder).
    Alone,
}

/// The running window's hold on the place later starts look for.
pub struct Primary {
    _lock: File,
    /// The window to wake when files arrive (set once it exists).
    pub wake: Arc<OnceLock<egui::Context>>,
    stop: Arc<AtomicBool>,
    /// Set when this start gives up on its window: later starts are told
    /// no and open a window of their own (#570).
    pub decline: Arc<AtomicBool>,
    port: u16,
    file: PathBuf,
}

impl Drop for Primary {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Wake the listening thread so that it sees the stop.
        let _ = TcpStream::connect_timeout(
            &([127, 0, 0, 1], self.port).into(),
            Duration::from_millis(200),
        );
        let _ = std::fs::remove_file(&self.file);
    }
}

/// The files of a start's arguments (every one not starting with `--`),
/// absolute, so that another process finds them.
pub fn file_args(args: &[String]) -> Vec<PathBuf> {
    args.iter()
        .filter(|a| !a.starts_with("--"))
        .map(|a| {
            let p = PathBuf::from(a);
            std::fs::canonicalize(&p)
                .or_else(|_| std::path::absolute(&p))
                .unwrap_or(p)
        })
        .collect()
}

/// A start with these arguments hands its files to a running window: not
/// with `--new-instance` and not the starts that open no window or change
/// the settings.
pub fn hands_over(args: &[String]) -> bool {
    !args.iter().any(|a| {
        matches!(
            a.as_str(),
            "--new-instance" | "--mcp" | "--mcp-window" | "--reset-settings"
        )
    })
}

/// Become the first start in `dir` (the settings folder), or hand `files`
/// to the window that is (waiting up to three seconds for one just
/// starting). Files that arrive later go to `inbox`.
pub fn claim(dir: Option<&Path>, files: &[PathBuf], inbox: Sender<Vec<PathBuf>>) -> Claim {
    let Some(dir) = dir else {
        return Claim::Alone;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return Claim::Alone;
    }
    let Ok(lock) = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(LOCK_FILE))
    else {
        return Claim::Alone;
    };
    match lock.try_lock() {
        Ok(()) => match listen(dir, lock, inbox) {
            Ok(p) => Claim::Primary(p),
            Err(e) => {
                log::warn!("no single window: {e}");
                Claim::Alone
            }
        },
        Err(std::fs::TryLockError::WouldBlock) => {
            if !files.is_empty() && hand_over(dir, files, Duration::from_secs(3)) {
                Claim::HandedOver
            } else {
                Claim::Alone
            }
        }
        Err(_) => Claim::Alone,
    }
}

/// Listen on a free port of this computer and name it in `dir`.
fn listen(dir: &Path, lock: File, inbox: Sender<Vec<PathBuf>>) -> std::io::Result<Primary> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    let info = Info {
        port,
        token: crate::mcp_link::new_token(),
        pid: std::process::id(),
    };
    let file = dir.join(INFO_FILE);
    write_private(
        &file,
        &serde_json::to_vec(&info).map_err(std::io::Error::other)?,
    )?;
    let stop = Arc::new(AtomicBool::new(false));
    let decline = Arc::new(AtomicBool::new(false));
    let wake: Arc<OnceLock<egui::Context>> = Arc::new(OnceLock::new());
    let (token, stopped, woken) = (info.token.clone(), stop.clone(), wake.clone());
    let declined = decline.clone();
    std::thread::Builder::new()
        .name("tracedraw-instance".into())
        .spawn(move || {
            for stream in listener.incoming() {
                if stopped.load(Ordering::Relaxed) {
                    break;
                }
                if let Ok(stream) = stream {
                    let inbox = (!declined.load(Ordering::Relaxed)).then_some(&inbox);
                    serve(stream, &token, inbox, &woken);
                }
            }
        })?;
    Ok(Primary {
        _lock: lock,
        wake,
        stop,
        decline,
        port,
        file,
    })
}

/// Write `bytes` to `path` readable by this user only, whole or not at all.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path)
}

/// Answer one start: its files go to `inbox` when the token is right; no
/// inbox (the window gave up, #570) answers no.
fn serve(
    stream: TcpStream,
    token: &str,
    inbox: Option<&Sender<Vec<PathBuf>>>,
    wake: &OnceLock<egui::Context>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let Ok(read) = stream.try_clone() else {
        return;
    };
    let mut line = String::new();
    if BufReader::new(read.take(MAX_REQUEST))
        .read_line(&mut line)
        .is_err()
    {
        return;
    }
    let Ok(req) = serde_json::from_str::<Request>(line.trim()) else {
        return;
    };
    if req.token != token {
        return;
    }
    let ok = inbox.is_some_and(|i| i.send(req.files).is_ok());
    if ok {
        if let Some(ctx) = wake.get() {
            ctx.request_repaint();
        }
    }
    let mut out = stream;
    let _ = serde_json::to_writer(&mut out, &Reply { ok });
    let _ = out.write_all(b"\n");
}

/// Send `files` to the window named in `dir`, trying again until `wait`
/// is over (a window may be starting, or `instance.json` left by a crash).
/// True when the window took them.
pub fn hand_over(dir: &Path, files: &[PathBuf], wait: Duration) -> bool {
    let end = Instant::now() + wait;
    loop {
        if let Some(answer) = try_hand_over(dir, files) {
            return answer;
        }
        if Instant::now() >= end {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// One try: `Some(true)` delivered, `Some(false)` declined, `None` the
/// window not reached.
fn try_hand_over(dir: &Path, files: &[PathBuf]) -> Option<bool> {
    let info: Info = serde_json::from_slice(&std::fs::read(dir.join(INFO_FILE)).ok()?).ok()?;
    let mut stream =
        TcpStream::connect_timeout(&([127, 0, 0, 1], info.port).into(), Duration::from_secs(1))
            .ok()?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let req = Request {
        token: info.token,
        files: files.to_vec(),
    };
    let mut line = serde_json::to_vec(&req).ok()?;
    line.push(b'\n');
    stream.write_all(&line).ok()?;
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).ok()?;
    serde_json::from_str::<Reply>(answer.trim())
        .ok()
        .map(|r| r.ok)
}

/// Open the files later starts handed over, each in a tab of its own (or
/// the tab of a drawing open from it, see `App::open_path`), and bring the
/// window up. Waits while a question or a dialog is up, or the drawings are
/// being closed. True when something was opened.
pub fn open_handed_over(app: &mut App, ctx: &egui::Context, rx: &Receiver<Vec<PathBuf>>) -> bool {
    if app.closing_all || !matches!(app.dialog, crate::ui::dialogs::Dialog::None) {
        return false;
    }
    let mut any = false;
    while let Ok(files) = rx.try_recv() {
        for f in files {
            app.open_path(f);
        }
        any = true;
    }
    if any {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
            egui::UserAttentionType::Informational,
        ));
    }
    any
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tracedraw-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Files are made absolute and flags left out; some flags open a
    /// window of their own.
    #[test]
    fn file_arguments_are_absolute_and_flags_are_skipped() {
        let args: Vec<String> = ["--no-splash", "a.cdr", "--renderer=gl", "/x/b.tdraw"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let files = file_args(&args);
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.is_absolute()), "{files:?}");
        assert!(files[0].ends_with("a.cdr"));
        assert!(hands_over(&args));
        for flag in [
            "--new-instance",
            "--mcp",
            "--mcp-window",
            "--reset-settings",
        ] {
            assert!(!hands_over(&[flag.to_string()]), "{flag}");
        }
    }

    /// A start that gave up on its window (its renderer hung, #570) tells
    /// later starts no at once, so they open a window of their own instead
    /// of handing their files to one that never shows.
    #[test]
    fn a_start_without_its_window_declines_files() {
        let dir = temp_dir("instance-decline");
        let (tx, rx) = channel();
        let Claim::Primary(first) = claim(Some(&dir), &[], tx) else {
            panic!("the first start keeps the window");
        };
        first.decline.store(true, Ordering::Relaxed);
        let file = dir.join("a.cdr");
        let (tx2, _rx2) = channel();
        let asked = Instant::now();
        assert!(matches!(
            claim(Some(&dir), std::slice::from_ref(&file), tx2),
            Claim::Alone
        ));
        assert!(asked.elapsed() < Duration::from_secs(2), "no waiting");
        assert!(rx.try_recv().is_err());
        drop(first);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A second start hands its files to the first, which wakes its
    /// window; one with no files opens a window of its own; once the
    /// first is gone, the next start is the first.
    #[test]
    fn a_second_start_hands_its_files_to_the_first() {
        let dir = temp_dir("instance-hand-over");
        let (tx, rx) = channel();
        let Claim::Primary(first) = claim(Some(&dir), &[], tx) else {
            panic!("the first start keeps the window");
        };
        let ctx = egui::Context::default();
        let _ = first.wake.set(ctx.clone());
        let file = dir.join("a.cdr");
        let (tx2, rx2) = channel();
        assert!(matches!(
            claim(Some(&dir), std::slice::from_ref(&file), tx2.clone()),
            Claim::HandedOver
        ));
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(2)).ok(),
            Some(vec![file])
        );
        assert!(ctx.has_requested_repaint());
        assert!(matches!(claim(Some(&dir), &[], tx2), Claim::Alone));
        assert!(rx2.try_recv().is_err());
        drop(first);
        assert!(!dir.join(INFO_FILE).exists());
        let (tx3, _rx3) = channel();
        assert!(matches!(claim(Some(&dir), &[], tx3), Claim::Primary(_)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An `instance.json` left by a crash (nobody holds the lock) is not
    /// in the way: the start becomes the first and names its own port.
    #[test]
    fn a_crash_leaves_nothing_in_the_way() {
        let dir = temp_dir("instance-crash");
        let dead = TcpListener::bind(("127.0.0.1", 0)).expect("port");
        let port = dead.local_addr().expect("addr").port();
        drop(dead);
        let stale = Info {
            port,
            token: "old".into(),
            pid: 1,
        };
        std::fs::write(
            dir.join(INFO_FILE),
            serde_json::to_vec(&stale).expect("json"),
        )
        .expect("write");
        let (tx, _rx) = channel();
        let Claim::Primary(p) = claim(Some(&dir), &[dir.join("x.cdr")], tx) else {
            panic!("the first start");
        };
        let info: Info = serde_json::from_slice(&std::fs::read(dir.join(INFO_FILE)).expect("info"))
            .expect("json");
        assert_eq!(info.port, p.port);
        assert_ne!(info.token, "old");
        drop(p);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A wrong token or garbage reaches nothing.
    #[test]
    fn strangers_are_refused() {
        let dir = temp_dir("instance-strangers");
        let (tx, rx) = channel();
        let Claim::Primary(p) = claim(Some(&dir), &[], tx) else {
            panic!("the first start");
        };
        for line in [
            b"{\"token\":\"nope\",\"files\":[\"/etc/passwd\"]}\n".to_vec(),
            b"garbage\n".to_vec(),
        ] {
            let mut s = TcpStream::connect(("127.0.0.1", p.port)).expect("connect");
            s.write_all(&line).expect("write");
            let mut answer = String::new();
            let _ = BufReader::new(s).read_line(&mut answer);
            assert!(!answer.contains("true"), "{answer}");
        }
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
        drop(p);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Starts at the same time make one window; the others' files reach it.
    #[test]
    fn starts_at_the_same_time_make_one_window() {
        let dir = temp_dir("instance-together");
        let (tx, rx) = channel();
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let (dir, tx) = (dir.clone(), tx.clone());
                std::thread::spawn(move || {
                    match claim(Some(&dir), &[dir.join(format!("f{i}.cdr"))], tx) {
                        Claim::Primary(p) => Some(p),
                        Claim::HandedOver => None,
                        Claim::Alone => panic!("start {i} found no window"),
                    }
                })
            })
            .collect();
        let firsts: Vec<Primary> = handles
            .into_iter()
            .filter_map(|h| h.join().expect("thread"))
            .collect();
        assert_eq!(firsts.len(), 1);
        let mut got = 0;
        while got < 3 {
            got += rx
                .recv_timeout(Duration::from_secs(3))
                .expect("files")
                .len();
        }
        drop(firsts);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A handed-over file opens in a tab of its own once no dialog is up.
    #[test]
    fn a_handed_over_file_opens_in_a_tab_once_no_question_is_up() {
        let dir = temp_dir("instance-open");
        let path = dir.join("art.tdraw");
        let doc =
            tracedraw_core::Document::new("Art", tracedraw_core::geometry::Size::new(50.0, 50.0));
        std::fs::write(&path, doc.to_json().expect("json")).expect("write");
        let mut app = App::headless();
        let ctx = egui::Context::default();
        let tabs = app.docs.len();
        let (tx, rx) = channel();
        tx.send(vec![path.clone()]).expect("send");
        app.dialog = crate::ui::dialogs::Dialog::ConfirmResetSettings;
        assert!(
            !open_handed_over(&mut app, &ctx, &rx),
            "waits for the question"
        );
        app.dialog = crate::ui::dialogs::Dialog::None;
        assert!(open_handed_over(&mut app, &ctx, &rx));
        assert_eq!(app.docs.len(), tabs + 1);
        assert_eq!(app.file.as_deref(), Some(path.as_path()));
        assert!(!open_handed_over(&mut app, &ctx, &rx), "nothing more");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The Linux launcher gives every file selected to one start.
    #[test]
    fn the_linux_launcher_starts_once_for_several_files() {
        let desktop = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/linux/tracedraw.desktop"),
        )
        .expect("desktop file");
        assert!(desktop.contains("Exec=tracedraw %F"), "{desktop}");
    }
}
