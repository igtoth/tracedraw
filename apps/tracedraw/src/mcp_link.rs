//! The open window as an MCP server: with Options > Global > General >
//! "Let AI agents on this computer edit the open drawing" on, TraceDraw
//! listens on a port of this computer only (127.0.0.1, a free one) and
//! answers Model Context Protocol requests (JSON-RPC 2.0 in HTTP POSTs to
//! `/mcp`) on the drawings in the window, with the tools of the headless
//! server (`mcp.rs`): every change is a command with undo, and the window
//! shows it at once.
//!
//! The port and a random token are written to `mcp-window.json` in the
//! preferences folder; requests without the token, or sent by a web page
//! (an `Origin` header), are refused. `tracedraw --mcp-window` is the
//! bridge an MCP client starts: it reads that file and forwards standard
//! input to the window, line by line. See docs/behavior/mcp.md.

use crate::app::App;
use std::io::{BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

/// The file that tells the bridge where the window listens.
pub const LINK_FILE: &str = "mcp-window.json";
/// Longest request body taken, in bytes.
const MAX_BODY: usize = 16 * 1024 * 1024;
/// How long a request waits for the window to answer.
const ANSWER_WAIT: Duration = Duration::from_secs(120);

/// Where the window listens, as the link file has it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LinkInfo {
    pub port: u16,
    pub token: String,
    pub pid: u32,
}

/// A request from the server thread, waiting for the window.
struct Incoming {
    line: String,
    reply: Sender<Option<String>>,
}

/// The listening window: requests arrive here and are answered on the
/// window's thread by [`Link::poll`].
pub struct Link {
    rx: Receiver<Incoming>,
    pub info: LinkInfo,
    stop: Arc<AtomicBool>,
    file: Option<PathBuf>,
}

impl Link {
    /// Listen on a free port of this computer and write the link file
    /// into `dir` (none: no file). `wake` asks the window for a frame when
    /// a request arrives.
    pub fn start(dir: Option<PathBuf>, wake: impl Fn() + Send + 'static) -> std::io::Result<Link> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        let info = LinkInfo {
            port,
            token: new_token(),
            pid: std::process::id(),
        };
        let file = match dir {
            Some(dir) => {
                std::fs::create_dir_all(&dir)?;
                let path = dir.join(LINK_FILE);
                let json = serde_json::to_string(&info).map_err(std::io::Error::other)?;
                std::fs::write(&path, json)?;
                Some(path)
            }
            None => None,
        };
        let (tx, rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let token = info.token.clone();
        let stopping = stop.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if stopping.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(stream) = stream else {
                    continue;
                };
                let tx = tx.clone();
                let token = token.clone();
                let wake_now = &wake;
                // One connection at a time: requests are answered in order
                // by the window anyway.
                serve_connection(stream, &token, &tx, wake_now);
            }
        });
        Ok(Link {
            rx,
            info,
            stop,
            file,
        })
    }

    /// Answer the requests that arrived; true when there were any.
    pub fn poll(&self, app: &mut App) -> bool {
        let mut any = false;
        while let Ok(req) = self.rx.try_recv() {
            any = true;
            let answer = crate::mcp::handle_line(app, &req.line);
            let _ = req.reply.send(answer);
        }
        any
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Wake the accept loop so it sees the stop.
        let _ = TcpStream::connect(("127.0.0.1", self.info.port));
        if let Some(path) = &self.file {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// A token nobody else can guess: random bytes from the time, the process
/// and the address of a fresh allocation, hashed twice.
pub(crate) fn new_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for round in 0..2u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(round);
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        h.write_u32(std::process::id());
        let probe = Box::new(round);
        h.write_usize(&*probe as *const u64 as usize);
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

/// Read one HTTP request from `stream`, check it, hand its body to the
/// window and write the answer.
fn serve_connection(stream: TcpStream, token: &str, tx: &Sender<Incoming>, wake: &dyn Fn()) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let mut reader = std::io::BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    let mut out = stream;
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
        return;
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let mut length = 0usize;
    let mut auth = String::new();
    let mut origin = false;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        match name.trim().to_ascii_lowercase().as_str() {
            "content-length" => length = value.trim().parse().unwrap_or(usize::MAX),
            "authorization" => auth = value.trim().to_string(),
            "origin" => origin = true,
            _ => {}
        }
    }
    let respond = |out: &mut TcpStream, status: &str, body: &str| {
        let _ = write!(
            out,
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = out.flush();
    };
    // A refused request's body is read (up to the size limit) before the
    // answer: closing a socket with unread data resets the connection,
    // and the client may then lose the answer.
    let refuse = |reader: &mut std::io::BufReader<TcpStream>, out: &mut TcpStream, status: &str| {
        let _ = std::io::copy(
            &mut reader.by_ref().take(length.min(MAX_BODY) as u64),
            &mut std::io::sink(),
        );
        respond(out, status, "{}");
        let _ = out.shutdown(std::net::Shutdown::Write);
    };
    if path != "/mcp" {
        refuse(&mut reader, &mut out, "404 Not Found");
        return;
    }
    if method != "POST" {
        refuse(&mut reader, &mut out, "405 Method Not Allowed");
        return;
    }
    // Web pages may not drive the window, and only the bridge (or a client
    // given the token) may.
    if origin || auth != format!("Bearer {token}") {
        refuse(&mut reader, &mut out, "403 Forbidden");
        return;
    }
    if length > MAX_BODY {
        respond(&mut out, "413 Payload Too Large", "{}");
        return;
    }
    let mut body = vec![0u8; length];
    if reader.read_exact(&mut body).is_err() {
        return;
    }
    let line = String::from_utf8_lossy(&body).trim().to_string();
    let (reply_tx, reply_rx) = channel();
    if tx
        .send(Incoming {
            line,
            reply: reply_tx,
        })
        .is_err()
    {
        respond(&mut out, "503 Service Unavailable", "{}");
        return;
    }
    wake();
    match reply_rx.recv_timeout(ANSWER_WAIT) {
        Ok(Some(answer)) => respond(&mut out, "200 OK", &answer),
        // A notification: accepted, nothing to say.
        Ok(None) => respond(&mut out, "202 Accepted", ""),
        Err(_) => respond(&mut out, "503 Service Unavailable", "{}"),
    }
}

/// `tracedraw --mcp-window`: forward standard input to the open window
/// and its answers to standard output, one JSON-RPC message per line.
/// Without a window that accepts agents, every request gets an error
/// saying so.
pub fn bridge() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let info = read_link(crate::settings::config_dir());
        let Some(answer) = forward(info.as_ref(), line) else {
            continue;
        };
        let mut out = stdout.lock();
        if writeln!(out, "{answer}").and_then(|_| out.flush()).is_err() {
            break;
        }
    }
}

/// The link file in `dir`, when a window wrote one.
pub fn read_link(dir: Option<PathBuf>) -> Option<LinkInfo> {
    let text = std::fs::read_to_string(dir?.join(LINK_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Send one message to the window `info` describes; its answer (`None`
/// for a notification), or a JSON-RPC error when the window cannot be
/// reached.
pub fn forward(info: Option<&LinkInfo>, line: &str) -> Option<String> {
    let id = serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(|v| v.get("id").cloned());
    let error = |message: &str| {
        id.as_ref().map(|id| {
            serde_json::json!({"jsonrpc": "2.0", "id": id,
                "error": {"code": -32000, "message": message}})
            .to_string()
        })
    };
    let Some(info) = info else {
        return error("no TraceDraw window accepts agents: open TraceDraw and turn on Options > Global > General > Let AI agents on this computer edit the open drawing");
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(ANSWER_WAIT + Duration::from_secs(5)))
        .http_status_as_error(false)
        // Straight to this computer, never through a proxy.
        .proxy(None)
        .build()
        .into();
    let url = format!("http://127.0.0.1:{}/mcp", info.port);
    let result = agent
        .post(&url)
        .header("content-type", "application/json")
        .header("authorization", &format!("Bearer {}", info.token))
        .send(line);
    match result {
        Ok(mut resp) => {
            let status = resp.status().as_u16();
            let body = resp.body_mut().read_to_string().unwrap_or_default();
            match status {
                200 => Some(body),
                202 => None,
                _ => error(&format!("the TraceDraw window answered {status}")),
            }
        }
        Err(e) => error(&format!("the TraceDraw window cannot be reached: {e}")),
    }
}

impl App {
    /// Start or stop listening as Options > Global > General says (and
    /// answer the requests that arrived). True when requests were
    /// answered, so the window draws again.
    pub fn sync_mcp_link(&mut self, ctx: &egui::Context) -> bool {
        // Not on Android, where Options hides the switch (#1415): a
        // setting carried over from elsewhere listens for no one.
        let wanted = self.settings.mcp_window && crate::ui::options::DesktopOnly::Agents.shown();
        match (&self.mcp_link, wanted) {
            (None, true) => {
                let ctx = ctx.clone();
                match Link::start(crate::settings::config_dir(), move || ctx.request_repaint()) {
                    Ok(link) => {
                        self.status = crate::i18n::trf(
                            "status.mcp_window_on",
                            &[("port", &link.info.port.to_string())],
                        );
                        self.mcp_link = Some(link);
                    }
                    Err(e) => {
                        self.settings.mcp_window = false;
                        self.status =
                            crate::i18n::trf("status.mcp_window_failed", &[("e", &e.to_string())]);
                    }
                }
            }
            (Some(_), false) => {
                self.mcp_link = None;
                self.status = crate::i18n::tr("status.mcp_window_off");
            }
            _ => {}
        }
        // The link answers on this thread; take it out while it does.
        let Some(link) = self.mcp_link.take() else {
            return false;
        };
        let any = link.poll(self);
        self.mcp_link = Some(link);
        any
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tracedraw-{name}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Run `client` on a thread while the window answers on this one.
    fn with_window<T: Send + 'static>(
        app: &mut App,
        link: &Link,
        client: impl FnOnce() -> T + Send + 'static,
    ) -> T {
        let handle = std::thread::spawn(client);
        while !handle.is_finished() {
            link.poll(app);
            std::thread::sleep(Duration::from_millis(5));
        }
        handle.join().expect("client")
    }

    #[test]
    fn agents_edit_the_drawing_in_the_window() {
        let dir = temp_dir("mcp-link");
        let mut app = App::headless();
        let link = Link::start(Some(dir.clone()), || {}).expect("listening");
        let info = read_link(Some(dir.clone())).expect("link file");
        assert_eq!(info, link.info);
        assert_eq!(info.token.len(), 32);
        let shapes = |app: &App| {
            app.doc().pages[0]
                .layers
                .iter()
                .map(|l| l.shapes.len())
                .sum::<usize>()
        };
        let before = shapes(&app);
        let i = info.clone();
        let answers = with_window(&mut app, &link, move || {
            let init = forward(
                Some(&i),
                &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
                    .to_string(),
            );
            let note = forward(
                Some(&i),
                &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string(),
            );
            let draw = forward(
                Some(&i),
                &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "run_script",
                    "arguments": {"source": "ActiveLayer.CreateEllipse(10, 10, 30, 20); print('ok');"}}})
                .to_string(),
            );
            (init, note, draw)
        });
        let init: serde_json::Value =
            serde_json::from_str(&answers.0.expect("an answer")).expect("json");
        assert_eq!(init["result"]["serverInfo"]["name"], "tracedraw");
        assert_eq!(answers.1, None, "notifications get no answer");
        let draw: serde_json::Value =
            serde_json::from_str(&answers.2.expect("an answer")).expect("json");
        assert_eq!(draw["result"]["content"][0]["text"], "ok");
        assert_eq!(shapes(&app), before + 1, "the window's drawing changed");
        // Undo in the window takes it back.
        app.undo();
        assert_eq!(shapes(&app), before);
        drop(link);
        assert!(
            read_link(Some(dir.clone())).is_none(),
            "the file goes with the link"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn strangers_and_web_pages_are_refused() {
        let mut app = App::headless();
        let link = Link::start(None, || {}).expect("listening");
        let port = link.info.port;
        let token = link.info.token.clone();
        let statuses = with_window(&mut app, &link, move || {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .http_status_as_error(false)
                .proxy(None)
                .build()
                .into();
            let url = format!("http://127.0.0.1:{port}/mcp");
            let body = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
            let status = |r: Result<ureq::http::Response<ureq::Body>, ureq::Error>| {
                r.map(|r| r.status().as_u16()).unwrap_or(0)
            };
            vec![
                // No token, a wrong one.
                status(agent.post(&url).send(body)),
                status(
                    agent
                        .post(&url)
                        .header("authorization", "Bearer nope")
                        .send(body),
                ),
                // A web page, even with the token.
                status(
                    agent
                        .post(&url)
                        .header("authorization", &format!("Bearer {token}"))
                        .header("origin", "https://example.org")
                        .send(body),
                ),
                // Other methods and paths.
                status(agent.get(&url).call()),
                status(
                    agent
                        .post(format!("http://127.0.0.1:{port}/other"))
                        .send(body),
                ),
                // The right token.
                status(
                    agent
                        .post(&url)
                        .header("authorization", &format!("Bearer {token}"))
                        .send(body),
                ),
            ]
        });
        assert_eq!(statuses, vec![403, 403, 403, 405, 404, 200]);
    }

    #[test]
    fn without_a_window_the_bridge_says_so() {
        let answer =
            forward(None, r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#).expect("an error answer");
        let v: serde_json::Value = serde_json::from_str(&answer).expect("json");
        assert_eq!(v["id"], 7);
        assert!(v["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("Options"));
        // A window that went away.
        let gone = LinkInfo {
            port: {
                let l = TcpListener::bind(("127.0.0.1", 0)).expect("port");
                l.local_addr().expect("address").port()
            },
            token: "x".into(),
            pid: 0,
        };
        let answer = forward(Some(&gone), r#"{"jsonrpc":"2.0","id":8,"method":"ping"}"#)
            .expect("an error answer");
        assert!(answer.contains("cannot be reached"), "{answer}");
        // Notifications get nothing back, window or not.
        assert_eq!(
            forward(
                None,
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#
            ),
            None
        );
    }

    #[test]
    fn the_option_starts_and_stops_listening() {
        let mut app = App::headless();
        let ctx = egui::Context::default();
        assert!(app.mcp_link.is_none());
        app.settings.mcp_window = true;
        app.sync_mcp_link(&ctx);
        let port = app
            .mcp_link
            .as_ref()
            .map(|l| l.info.port)
            .expect("listening");
        assert!(TcpStream::connect(("127.0.0.1", port)).is_ok());
        app.settings.mcp_window = false;
        app.sync_mcp_link(&ctx);
        assert!(app.mcp_link.is_none());
    }
}
