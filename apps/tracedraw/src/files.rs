//! File access for both builds.
//!
//! Natively these are the file system and the system file dialogs, used
//! synchronously as before. In a browser there is no file system: a file
//! the user picks is kept in memory under a synthetic path
//! (`/upload/<name>`), so the path-based open and import code reads it with
//! [`read`] like any file, and [`write`] offers the bytes as a download
//! named after the path's file name.
//!
//! Dialogs come in three shapes:
//! - [`Dialog::pick_file`] runs a closure with the picked path: at once
//!   natively, on a later frame in a browser (see [`run_finished`]).
//! - [`Dialog::pick_into`] stores the picked path under a key that a
//!   dialog's own code collects with [`take_picked`] on the same frame
//!   (natively) or a later one (browser).
//! - [`Dialog::save_file`] runs a closure with the chosen path at once: a
//!   save dialog natively, `/download/<file name>` in a browser.

use crate::app::App;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

type Job = Box<dyn FnOnce(&mut App, PathBuf)>;

thread_local! {
    /// Dialogs finished in the background (browser), waiting for a frame.
    static FINISHED: RefCell<Vec<(Job, PathBuf)>> = const { RefCell::new(Vec::new()) };
    /// Picked paths waiting for the dialog that asked for them.
    static PICKED: RefCell<HashMap<String, PathBuf>> = RefCell::new(HashMap::new());
    /// Uploaded files by synthetic path (browser only).
    static STORE: RefCell<HashMap<PathBuf, Vec<u8>>> = RefCell::new(HashMap::new());
}

/// True in the browser build: no file system, downloads instead of saves.
pub const WEB: bool = cfg!(target_arch = "wasm32");

/// True in the Android app: files come and go through the system's
/// document picker, named by [`document_path`] (`android.rs`,
/// docs/behavior/android.md).
pub const ANDROID: bool = cfg!(target_os = "android");

/// The folder under which a document the Android picker returned is
/// named: `/content/<its URI, escaped>/<its display name>`. The name
/// gives the title and the type (its extension), the URI the bytes.
pub const DOCUMENT_ROOT: &str = "/content";

/// The path naming the document at the content URI `uri` shown as
/// `name`: `/content/<uri escaped>/<name>`, so that `file_name` and
/// `extension` work as for any file and [`read`] and [`write`] find the
/// URI again with [`document_uri`].
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub fn document_path(uri: &str, name: &str) -> PathBuf {
    let mut escaped = String::with_capacity(uri.len());
    for c in uri.chars() {
        match c {
            '%' => escaped.push_str("%25"),
            '/' => escaped.push_str("%2F"),
            c => escaped.push(c),
        }
    }
    let name = name.replace(['/', '\\'], "_");
    let name = if name.trim().is_empty() {
        "document".to_string()
    } else {
        name
    };
    PathBuf::from(DOCUMENT_ROOT).join(escaped).join(name)
}

/// The content URI of a path made by [`document_path`]; `None` for any
/// other path.
pub fn document_uri(path: &Path) -> Option<String> {
    let rest = path
        .to_str()?
        .strip_prefix(DOCUMENT_ROOT)?
        .strip_prefix('/')?;
    let (escaped, _name) = rest.split_once('/')?;
    let mut uri = String::with_capacity(escaped.len());
    let mut chars = escaped.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let code: String = chars.by_ref().take(2).collect();
            match code.as_str() {
                "25" => uri.push('%'),
                "2F" => uri.push('/'),
                other => {
                    uri.push('%');
                    uri.push_str(other);
                }
            }
        } else {
            uri.push(c);
        }
    }
    Some(uri)
}

/// True for a path made by [`document_path`].
pub fn is_document_path(path: &Path) -> bool {
    document_uri(path).is_some()
}

/// A picked document's name with an extension: its own, else one for the
/// MIME type its provider gave (a photo called "IMG_1" of type image/jpeg
/// becomes "IMG_1.jpg"), so the editor can tell the type.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub fn name_with_extension(name: &str, mime: &str) -> String {
    if Path::new(name).extension().is_some() {
        return name.to_string();
    }
    let ext = match mime.trim().to_ascii_lowercase().as_str() {
        "application/pdf" => "pdf",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/webp" => "webp",
        "image/tiff" => "tiff",
        "image/svg+xml" => "svg",
        "text/plain" => "txt",
        "text/csv" => "csv",
        "application/json" => "json",
        "application/postscript" => "eps",
        _ => return name.to_string(),
    };
    format!("{name}.{ext}")
}

/// The MIME type the system's save picker is told for a file with the
/// extension `ext` (any case); a plain byte stream for the formats it
/// does not know, which keeps the name as typed.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub fn mime_for_extension(ext: &str) -> &'static str {
    match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "zip" => "application/zip",
        "eps" | "ps" => "application/postscript",
        _ => "application/octet-stream",
    }
}

/// The folder the Save Drawing dialog starts in when the platform fixes
/// it: the Android app's documents folder; `None` elsewhere (the last
/// folder used, Documents or home).
pub fn save_dir() -> Option<PathBuf> {
    #[cfg(target_os = "android")]
    {
        let dir = android_documents_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::warn!("cannot create {}: {e}", dir.display());
        }
        return Some(dir);
    }
    #[allow(unreachable_code)]
    None
}

/// Where the Android app saves drawings and exports: its own folder under
/// the shared storage (`Android/data/<package>/files`, which file managers
/// show), set by `android_main`; the settings folder when it is missing.
#[cfg(target_os = "android")]
fn android_documents_dir() -> PathBuf {
    std::env::var_os("TRACEDRAW_DOCUMENTS_DIR")
        .map(PathBuf::from)
        .or_else(crate::settings::config_dir)
        .unwrap_or_else(|| PathBuf::from("/sdcard/Download"))
}

/// The path naming the same file however it was reached (links, `.`
/// and `..`); the absolute path when that fails (the file is gone);
/// `path` as it is in a browser.
pub fn canonical(path: &Path) -> PathBuf {
    if WEB || is_document_path(path) {
        return path.to_path_buf();
    }
    std::fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

/// Two canonical paths name one file: on Windows without regard to the
/// letter case and to the `\\?\` prefix.
pub fn same_path(a: &Path, b: &Path) -> bool {
    if cfg!(windows) {
        let key = |p: &Path| {
            let s = p.to_string_lossy().to_lowercase();
            s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(s)
        };
        key(a) == key(b)
    } else {
        a == b
    }
}

/// A canonical path as people type it: on Windows without the `\\?\`
/// prefix (`\\?\C:\a` is `C:\a`, `\\?\UNC\server\share` is
/// `\\server\share`); other paths as they are.
pub fn plain_path(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    match s.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

/// Read a whole file (or an uploaded one in a browser).
pub fn read(path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
    let path = path.as_ref();
    if WEB {
        return STORE
            .with(|s| s.borrow().get(path).cloned())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("{} is not available in the browser", path.display()),
                )
            });
    }
    #[cfg(target_os = "android")]
    if let Some(uri) = document_uri(path) {
        return crate::android::documents::read(&uri);
    }
    std::fs::read(path)
}

/// Read a whole file as UTF-8 text.
pub fn read_to_string(path: impl AsRef<Path>) -> std::io::Result<String> {
    let bytes = read(path)?;
    String::from_utf8(bytes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
}

/// Write a whole file; in a browser, download it under the path's name.
pub fn write(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> std::io::Result<()> {
    let path = path.as_ref();
    if WEB {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "download".into());
        return download(&name, bytes.as_ref());
    }
    #[cfg(target_os = "android")]
    if let Some(uri) = document_uri(path) {
        return crate::android::documents::write(&uri, bytes.as_ref());
    }
    std::fs::write(path, bytes)
}

/// Write a file as `fill` makes it, without holding the whole file in
/// memory; in a browser, the bytes are gathered and downloaded.
pub fn write_with(
    path: impl AsRef<Path>,
    fill: impl FnOnce(&mut dyn std::io::Write) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let path = path.as_ref();
    // A download, or a document the Android picker returned: gathered,
    // then handed over whole.
    if WEB || is_document_path(path) {
        let mut bytes = Vec::new();
        fill(&mut bytes)?;
        return write(path, bytes);
    }
    let mut w = std::io::BufWriter::new(std::fs::File::create(path)?);
    fill(&mut w)?;
    std::io::Write::flush(&mut w)
}

/// Write a file as `fill` makes it, creating its folder first
/// (auto-backups). It is written under a `.part` name and renamed when
/// complete, so a write cut short never leaves half a file under the
/// real name. Nothing happens in a browser.
pub fn write_creating_with(
    path: impl AsRef<Path>,
    fill: impl FnOnce(&mut dyn std::io::Write) -> std::io::Result<()>,
) -> std::io::Result<()> {
    if WEB {
        return Ok(());
    }
    let path = path.as_ref();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut part = path.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let written = std::fs::File::create(&part).and_then(|f| {
        let mut w = std::io::BufWriter::new(f);
        fill(&mut w)?;
        std::io::Write::flush(&mut w)
    });
    match written.and_then(|()| std::fs::rename(&part, path)) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

/// Copy a file, creating the target folder (backups). Nothing happens in
/// a browser, which has no files to copy.
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> std::io::Result<()> {
    if WEB {
        return Ok(());
    }
    if let Some(dir) = to.as_ref().parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::copy(from, to).map(|_| ())
}

/// True when a file exists on disk (never in a browser). A document the
/// Android picker returned counts as existing: the system holds it and
/// asked about replacing it already.
pub fn exists(path: impl AsRef<Path>) -> bool {
    if WEB {
        return false;
    }
    let path = path.as_ref();
    (ANDROID && is_document_path(path)) || path.is_file()
}

/// Write a file, creating its folder first (auto-backups).
pub fn write_creating(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> std::io::Result<()> {
    if WEB {
        return Ok(());
    }
    if let Some(dir) = path.as_ref().parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, bytes)
}

/// Delete a file (nothing in a browser).
pub fn remove(path: impl AsRef<Path>) -> std::io::Result<()> {
    if WEB {
        return Ok(());
    }
    std::fs::remove_file(path)
}

/// Choose a folder (desktop only; `None` when cancelled, in a browser or
/// on Android, where the editor never needs a folder of its own).
pub fn pick_folder() -> Option<PathBuf> {
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        rfd::FileDialog::new().pick_folder()
    }
    #[cfg(any(target_arch = "wasm32", target_os = "android"))]
    {
        None
    }
}

/// The files directly inside a folder (not its subfolders), in no
/// particular order; empty when it cannot be read or in a browser.
pub fn list_dir(path: impl AsRef<Path>) -> Vec<PathBuf> {
    if WEB {
        return Vec::new();
    }
    match std::fs::read_dir(path) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// One entry of a folder listing (the Save Drawing dialog).
#[derive(Debug, Clone, PartialEq)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    /// Bytes (files).
    pub size: u64,
    /// Seconds since 1970, when known.
    pub modified: Option<i64>,
}

/// The folders and files directly inside `dir`: folders first, then by
/// name ignoring case; hidden ones (a leading dot) left out. Empty in a
/// browser or when the folder cannot be read.
pub fn list_entries(dir: impl AsRef<Path>) -> Vec<DirEntry> {
    if WEB {
        return Vec::new();
    }
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<DirEntry> = read
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                return None;
            }
            let path = e.path();
            // Follow links: a link to a folder lists as a folder.
            let meta = std::fs::metadata(&path).ok();
            let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
            let size = meta.as_ref().map_or(0, |m| m.len());
            let modified = meta
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);
            Some(DirEntry {
                name,
                path,
                is_dir,
                size,
                modified,
            })
        })
        .collect();
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    out
}

/// True for a folder on disk (never in a browser).
pub fn is_dir(path: impl AsRef<Path>) -> bool {
    !WEB && path.as_ref().is_dir()
}

/// Make a folder (nothing in a browser).
pub fn create_dir(path: impl AsRef<Path>) -> std::io::Result<()> {
    if WEB {
        return Ok(());
    }
    std::fs::create_dir(path)
}

/// Make a folder and the folders it is in (nothing in a browser).
pub fn create_dir_all(path: impl AsRef<Path>) -> std::io::Result<()> {
    if WEB {
        return Ok(());
    }
    std::fs::create_dir_all(path)
}

/// Rename a file or folder; one already called `to` is never replaced
/// (nothing in a browser).
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> std::io::Result<()> {
    if WEB {
        return Ok(());
    }
    if to.as_ref().exists() {
        return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
    }
    std::fs::rename(from, to)
}

/// A folder worth a shortcut in the Save Drawing dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Home,
    Desktop,
    Documents,
    Downloads,
    Pictures,
    /// A drive or the file system's root.
    Root,
}

/// The user's home folder.
pub fn home_dir() -> Option<PathBuf> {
    if WEB {
        return None;
    }
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

/// The shortcuts that exist on this computer: home, desktop, documents,
/// downloads and pictures (the names the system gives them, read from
/// the user's folder settings on Linux), then the drives (Windows) or
/// the root. Empty in a browser.
pub fn places() -> Vec<(Place, PathBuf)> {
    let mut out = Vec::new();
    let Some(home) = home_dir() else {
        return out;
    };
    out.push((Place::Home, home.clone()));
    let xdg = xdg_user_dirs(&home);
    for (place, key, default) in [
        (Place::Desktop, "XDG_DESKTOP_DIR", "Desktop"),
        (Place::Documents, "XDG_DOCUMENTS_DIR", "Documents"),
        (Place::Downloads, "XDG_DOWNLOAD_DIR", "Downloads"),
        (Place::Pictures, "XDG_PICTURES_DIR", "Pictures"),
    ] {
        let dir = xdg
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, p)| p.clone())
            .unwrap_or_else(|| home.join(default));
        if dir.is_dir() && dir != home {
            out.push((place, dir));
        }
    }
    if cfg!(windows) {
        for letter in b'C'..=b'Z' {
            let root = PathBuf::from(format!("{}:\\", letter as char));
            if root.is_dir() {
                out.push((Place::Root, root));
            }
        }
    } else {
        out.push((Place::Root, PathBuf::from("/")));
    }
    out
}

/// The `XDG_*_DIR` folders of `~/.config/user-dirs.dirs` (Linux), with
/// `$HOME` expanded.
fn xdg_user_dirs(home: &Path) -> Vec<(String, PathBuf)> {
    let Ok(text) = std::fs::read_to_string(home.join(".config/user-dirs.dirs")) else {
        return Vec::new();
    };
    parse_user_dirs(&text, home)
}

/// Read `KEY="$HOME/Folder"` lines.
pub(crate) fn parse_user_dirs(text: &str, home: &Path) -> Vec<(String, PathBuf)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let value = value.trim().trim_matches('"');
            let path = match value.strip_prefix("$HOME") {
                Some(rest) => home.join(rest.trim_start_matches('/')),
                None => PathBuf::from(value),
            };
            Some((key.trim().to_string(), path))
        })
        .collect()
}

/// Decode an image file (any format the image crate was built with).
pub fn open_image(path: impl AsRef<Path>) -> Result<image::DynamicImage, String> {
    let bytes = read(path).map_err(|e| e.to_string())?;
    image::load_from_memory(&bytes).map_err(|e| e.to_string())
}

/// Keep bytes under a synthetic path so [`read`] finds them (browser).
pub fn store(path: &Path, bytes: Vec<u8>) {
    STORE.with(|s| {
        s.borrow_mut().insert(path.to_path_buf(), bytes);
    });
}

/// Keep an uploaded file past the dialog that picked it (a profile that
/// is read again later): returns the path to read it from. Natively the
/// file stays where it is.
pub fn keep(path: &Path) -> PathBuf {
    if !WEB {
        return path.to_path_buf();
    }
    let kept = PathBuf::from("/kept").join(path.file_name().unwrap_or_default());
    if let Ok(bytes) = read(path) {
        store(&kept, bytes);
    }
    kept
}

/// Forget an uploaded file once nothing needs it any more.
pub fn forget(path: &Path) {
    STORE.with(|s| {
        s.borrow_mut().remove(path);
    });
}

/// Run the dialogs that finished since the last frame (browser, Android).
/// Called once per frame before the interface is drawn.
pub fn run_finished(app: &mut App) {
    #[cfg(target_os = "android")]
    android_pickers::run_answers(app);
    let jobs: Vec<(Job, PathBuf)> = FINISHED.with(|f| std::mem::take(&mut *f.borrow_mut()));
    for (job, path) in jobs {
        job(app, path.clone());
        forget(&path);
    }
}

/// The Android document picker (Storage Access Framework): a request
/// shows the system's picker and remembers what to do with the answer,
/// which the activity hands back on a later frame (`android.rs`).
#[cfg(target_os = "android")]
mod android_pickers {
    use super::*;
    use crate::android::documents;

    /// What a request waits to do with the picked documents.
    enum Waiting {
        /// Run with the first document.
        One(Job),
        /// Run with every document.
        Many(Box<dyn FnOnce(&mut App, Vec<PathBuf>)>),
        /// Keep the first document for [`take_picked`] under this key.
        Into(String),
    }

    thread_local! {
        static WAITING: RefCell<HashMap<i32, Waiting>> = RefCell::new(HashMap::new());
    }

    fn wait(request: i32, waiting: Waiting) {
        WAITING.with(|w| {
            w.borrow_mut().insert(request, waiting);
        });
    }

    fn forget(request: i32) {
        WAITING.with(|w| {
            w.borrow_mut().remove(&request);
        });
    }

    /// Show the picker for one document (or several) and run `then`.
    fn pick(multiple: bool, then: Waiting) {
        let request = documents::next_request();
        wait(request, then);
        if !documents::open(request, multiple) {
            forget(request);
        }
    }

    pub fn pick_one(then: Job) {
        pick(false, Waiting::One(then));
    }

    pub fn pick_many(then: Box<dyn FnOnce(&mut App, Vec<PathBuf>)>) {
        pick(true, Waiting::Many(then));
    }

    pub fn pick_into(key: String) {
        pick(false, Waiting::Into(key));
    }

    /// Show the picker for a place to save `name` and run `then` with the
    /// document made there.
    pub fn save(name: &str, then: Job) {
        let ext = Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let request = documents::next_request();
        wait(request, Waiting::One(then));
        if !documents::create(request, mime_for_extension(ext), name) {
            forget(request);
        }
    }

    /// The paths of an answer's documents.
    fn paths(answer: &documents::Answer) -> Vec<PathBuf> {
        answer
            .picked
            .iter()
            .map(|p| document_path(&p.uri, &name_with_extension(&p.name, &p.mime)))
            .collect()
    }

    /// Run what the answers that arrived since the last frame were
    /// waited for; a cancelled picker runs nothing.
    pub fn run_answers(app: &mut App) {
        for answer in documents::take_answers() {
            let Some(waiting) = WAITING.with(|w| w.borrow_mut().remove(&answer.request)) else {
                continue;
            };
            let paths = paths(&answer);
            match waiting {
                Waiting::One(job) => {
                    if let Some(path) = paths.into_iter().next() {
                        job(app, path);
                    }
                }
                Waiting::Many(job) => {
                    if !paths.is_empty() {
                        job(app, paths);
                    }
                }
                Waiting::Into(key) => {
                    if let Some(path) = paths.into_iter().next() {
                        PICKED.with(|p| {
                            p.borrow_mut().insert(key, path);
                        });
                    }
                }
            }
        }
    }
}

/// Files dropped on the window this frame: `then` runs with each one's
/// path, at once natively, on a later frame in a browser (where the bytes
/// are read first and kept under `/upload/<name>` like a picked file).
pub fn take_dropped(
    ctx: &egui::Context,
    app: &mut App,
    then: impl FnOnce(&mut App, PathBuf) + Clone + 'static,
) {
    let dropped = ctx.input(|i| i.raw.dropped_files.clone());
    for file in dropped {
        #[cfg(not(target_arch = "wasm32"))]
        then.clone()(app, file.path().to_path_buf());
        #[cfg(target_arch = "wasm32")]
        {
            let _ = &app;
            let then = then.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match file.bytes_async().await {
                    Ok(bytes) => {
                        let name = file.path().file_name().unwrap_or_default();
                        let path = PathBuf::from("/upload").join(name);
                        store(&path, bytes);
                        let job: Job = Box::new(then);
                        FINISHED.with(|f| f.borrow_mut().push((job, path)));
                        crate::web::request_repaint();
                    }
                    Err(e) => log::warn!("{}: {e}", file.path().display()),
                }
            });
        }
    }
}

/// The path picked for `key` by [`Dialog::pick_into`], once.
pub fn take_picked(key: &str) -> Option<PathBuf> {
    PICKED.with(|p| p.borrow_mut().remove(key))
}

/// Whether the file picker of a browser with this user agent, platform
/// and number of touch points filters by the system's file types rather
/// than by extension: on an iPhone, iPad or iPod and on Android, where
/// files whose extension the system has no type for (`.cdr`, `.cmx`,
/// `.tdraw` ...) can be greyed out once the page names the extensions it
/// takes (#1311). iPadOS browsers call themselves a Mac ("MacIntel",
/// "Macintosh") with a touch screen; Macs have no touch points. Desktop
/// browsers match the extensions themselves and keep the filters.
#[cfg(any(target_arch = "wasm32", test))]
pub fn picker_filters_by_type(user_agent: &str, platform: &str, touch_points: u32) -> bool {
    let named = ["iPhone", "iPad", "iPod"]
        .iter()
        .any(|d| user_agent.contains(d) || platform.starts_with(d));
    let mac = platform == "MacIntel" || user_agent.contains("Macintosh");
    named || (mac && touch_points > 1) || user_agent.contains("Android")
}

/// A file dialog description, the same in both builds.
#[derive(Debug, Clone, Default)]
pub struct Dialog {
    filters: Vec<(String, Vec<String>)>,
    file_name: Option<String>,
}

impl Dialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_filter(mut self, name: impl Into<String>, extensions: &[&str]) -> Self {
        self.filters.push((
            name.into(),
            extensions.iter().map(|e| e.to_string()).collect(),
        ));
        self
    }

    pub fn set_file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }

    /// Pick a file and run `then` with its path: at once natively, on a
    /// later frame in a browser.
    pub fn pick_file(self, app: &mut App, then: impl FnOnce(&mut App, PathBuf) + 'static) {
        #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
        if let Some(path) = self.native_pick() {
            then(app, path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = app;
            self.web_pick(Box::new(then));
        }
        // The system's document picker; `then` runs when it answers.
        #[cfg(target_os = "android")]
        {
            let _ = app;
            android_pickers::pick_one(Box::new(then));
        }
    }

    /// Pick one or more files and run `then` with their paths (one file in
    /// a browser).
    pub fn pick_files(self, app: &mut App, then: impl FnOnce(&mut App, Vec<PathBuf>) + 'static) {
        #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
        {
            let mut d = rfd::FileDialog::new();
            for (name, exts) in &self.filters {
                let exts: Vec<&str> = exts.iter().map(|e| e.as_str()).collect();
                d = d.add_filter(name, &exts);
            }
            if let Some(paths) = d.pick_files() {
                then(app, paths);
            }
        }
        #[cfg(target_arch = "wasm32")]
        self.pick_file(app, move |app, p| then(app, vec![p]));
        #[cfg(target_os = "android")]
        {
            let _ = app;
            android_pickers::pick_many(Box::new(then));
        }
    }

    /// Pick a file and keep its path for [`take_picked`]`(key)`.
    pub fn pick_into(self, key: &str) {
        let key = key.to_string();
        #[cfg(target_os = "android")]
        android_pickers::pick_into(key);
        #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
        if let Some(path) = self.native_pick() {
            PICKED.with(|p| {
                p.borrow_mut().insert(key, path);
            });
        }
        // The upload outlives this job: the dialog reads it on a later frame.
        #[cfg(target_arch = "wasm32")]
        self.web_pick(Box::new(move |_app, path| {
            let kept = keep(&path);
            PICKED.with(|p| {
                p.borrow_mut().insert(key, kept);
            });
        }));
    }

    /// Choose where to save and run `then` with that path; `then` writes
    /// with [`write`]. A save dialog natively, at once; in a browser there
    /// is no dialog: the path is `/download/<file name>` and the write
    /// becomes a download; on Android the system's picker asks where,
    /// and `then` runs on a later frame with the document it made.
    pub fn save_file(self, app: &mut App, then: impl FnOnce(&mut App, PathBuf) + 'static) {
        #[cfg(target_os = "android")]
        {
            let _ = app;
            let name = self.suggested_name();
            android_pickers::save(&name, Box::new(then));
        }
        #[cfg(not(target_os = "android"))]
        if let Some(path) = self.save_path() {
            then(app, path);
        }
    }

    /// The name offered for a new file: the one set, with the first
    /// filter's extension when it has none.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    fn suggested_name(&self) -> String {
        let name = self
            .file_name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "drawing".into());
        if Path::new(&name).extension().is_some() {
            return name;
        }
        match self.filters.first().and_then(|(_, exts)| exts.first()) {
            Some(ext) => format!("{name}.{ext}"),
            None => name,
        }
    }

    /// The path to save to, or `None` when the user cancels: a save
    /// dialog natively, `/download/<file name>` in a browser (write it with
    /// [`write`] to download it). On Android, where the picker answers on
    /// a later frame (see [`Dialog::save_file`]), the few callers that
    /// need a path at once get one in the app's own documents folder.
    pub fn save_path(self) -> Option<PathBuf> {
        if WEB {
            let name = self.file_name.unwrap_or_else(|| "download".into());
            return Some(PathBuf::from("/download").join(name));
        }
        #[cfg(target_os = "android")]
        {
            let name = self.suggested_name();
            let dir = android_documents_dir();
            if let Err(e) = std::fs::create_dir_all(&dir) {
                log::warn!("cannot create {}: {e}", dir.display());
            }
            log::info!("saved without a picker to {}", dir.display());
            return Some(dir.join(name));
        }
        #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
        {
            let mut d = rfd::FileDialog::new();
            for (name, exts) in &self.filters {
                let exts: Vec<&str> = exts.iter().map(|e| e.as_str()).collect();
                d = d.add_filter(name, &exts);
            }
            if let Some(n) = &self.file_name {
                d = d.set_file_name(n);
            }
            return d.save_file();
        }
        #[allow(unreachable_code)]
        None
    }

    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    fn native_pick(&self) -> Option<PathBuf> {
        let mut d = rfd::FileDialog::new();
        for (name, exts) in &self.filters {
            let exts: Vec<&str> = exts.iter().map(|e| e.as_str()).collect();
            d = d.add_filter(name, &exts);
        }
        d.pick_file()
    }

    #[cfg(target_arch = "wasm32")]
    fn web_pick(self, then: Job) {
        let mut d = rfd::AsyncFileDialog::new();
        // On an iPhone, iPad or Android device the picker can grey out
        // every file whose extension the system does not know, `.cdr`
        // among them, as soon as the page names the extensions it takes:
        // there it offers every file instead (#1311).
        if !crate::web::picker_filters_by_type() {
            for (name, exts) in &self.filters {
                let exts: Vec<&str> = exts.iter().map(|e| e.as_str()).collect();
                d = d.add_filter(name, &exts);
            }
        }
        wasm_bindgen_futures::spawn_local(async move {
            let Some(handle) = d.pick_file().await else {
                return;
            };
            let name = handle.file_name();
            let bytes = handle.read().await;
            let path = PathBuf::from("/upload").join(name);
            store(&path, bytes);
            FINISHED.with(|f| f.borrow_mut().push((then, path)));
            crate::web::request_repaint();
        });
    }
}

/// Offer bytes as a browser download (no-op natively).
#[cfg(target_arch = "wasm32")]
fn download(name: &str, bytes: &[u8]) -> std::io::Result<()> {
    crate::web::download(name, bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

#[cfg(not(target_arch = "wasm32"))]
fn download(_name: &str, _bytes: &[u8]) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// iPhones and iPads (also iPadOS, which calls itself a Mac with a
    /// touch screen) and Android browsers get a picker without extension
    /// filters; Macs, Windows and Linux keep them (#1311).
    #[test]
    fn mobile_browsers_pick_without_extension_filters() {
        let ipad_os = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";
        assert!(picker_filters_by_type(ipad_os, "MacIntel", 5));
        assert!(!picker_filters_by_type(ipad_os, "MacIntel", 0), "a Mac");
        let iphone = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15";
        assert!(picker_filters_by_type(iphone, "iPhone", 5));
        let old_ipad = "Mozilla/5.0 (iPad; CPU OS 12_0 like Mac OS X) AppleWebKit/605.1.15";
        assert!(picker_filters_by_type(old_ipad, "iPad", 5));
        let android = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/130.0 Mobile Safari/537.36";
        assert!(picker_filters_by_type(android, "Linux armv8l", 5));
        let firefox_android =
            "Mozilla/5.0 (Android 14; Mobile; rv:131.0) Gecko/131.0 Firefox/131.0";
        assert!(picker_filters_by_type(firefox_android, "Linux aarch64", 5));
        let windows = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/130.0 Safari/537.36";
        assert!(
            !picker_filters_by_type(windows, "Win32", 10),
            "a touch laptop"
        );
        let linux = "Mozilla/5.0 (X11; Linux x86_64; rv:131.0) Gecko/20100101 Firefox/131.0";
        assert!(!picker_filters_by_type(linux, "Linux x86_64", 0));
    }

    #[test]
    fn picked_paths_are_taken_once() {
        PICKED.with(|p| {
            p.borrow_mut().insert("k".into(), PathBuf::from("/x/a.csv"));
        });
        assert_eq!(take_picked("k"), Some(PathBuf::from("/x/a.csv")));
        assert_eq!(take_picked("k"), None);
    }

    #[test]
    fn finished_jobs_run_once_and_free_their_upload() {
        let mut app = App::headless();
        let path = PathBuf::from("/upload/t.txt");
        store(&path, b"hello".to_vec());
        FINISHED.with(|f| {
            f.borrow_mut().push((
                Box::new(|app: &mut App, p: PathBuf| {
                    app.status = format!("got {}", p.display());
                }),
                path.clone(),
            ))
        });
        run_finished(&mut app);
        assert_eq!(app.status, "got /upload/t.txt");
        run_finished(&mut app);
        assert_eq!(app.status, "got /upload/t.txt");
        assert!(STORE.with(|s| s.borrow().get(&path).is_none()));
    }

    /// A document the Android picker returned is named by its URI and its
    /// display name: the name gives the title and the type, the URI comes
    /// back whole from the path, however many slashes and percent signs
    /// it holds (#893).
    #[test]
    fn a_picked_document_keeps_its_uri_and_shows_its_name() {
        let uri = "content://com.android.externalstorage.documents/document/primary%3ADrawings%2Flogo.cdr";
        let path = document_path(uri, "logo.cdr");
        assert_eq!(path.file_name().unwrap(), "logo.cdr");
        assert_eq!(path.extension().unwrap(), "cdr");
        assert!(path.starts_with(DOCUMENT_ROOT));
        assert!(is_document_path(&path));
        assert_eq!(document_uri(&path).as_deref(), Some(uri));
        assert_eq!(canonical(&path), path);
        // A name with a slash cannot become a folder; an empty one is named.
        assert_eq!(
            document_path(uri, "a/b.svg").file_name().unwrap(),
            "a_b.svg"
        );
        assert_eq!(document_path(uri, "  ").file_name().unwrap(), "document");
        // Ordinary paths are not documents.
        assert_eq!(document_uri(Path::new("/home/a/content/x")), None);
        assert_eq!(document_uri(Path::new("/content")), None);
        assert!(!is_document_path(Path::new("/tmp/logo.cdr")));
    }

    /// A picked document without an extension gets one from its MIME
    /// type, and the save picker is told the type of the name it saves.
    #[test]
    fn names_and_mime_types_follow_each_other() {
        assert_eq!(name_with_extension("IMG_1", "image/jpeg"), "IMG_1.jpg");
        assert_eq!(
            name_with_extension("report", "application/pdf"),
            "report.pdf"
        );
        assert_eq!(name_with_extension("logo.cdr", ""), "logo.cdr");
        assert_eq!(
            name_with_extension("notes", "application/x-unknown"),
            "notes"
        );
        assert_eq!(mime_for_extension("pdf"), "application/pdf");
        assert_eq!(mime_for_extension(".SVG"), "image/svg+xml");
        assert_eq!(mime_for_extension("tdraw"), "application/octet-stream");
        assert_eq!(mime_for_extension("cdr"), "application/octet-stream");
        assert_eq!(
            Dialog::new()
                .add_filter("PDF", &["pdf"])
                .set_file_name("Graphic1")
                .suggested_name(),
            "Graphic1.pdf"
        );
        assert_eq!(
            Dialog::new()
                .add_filter("PDF", &["pdf"])
                .set_file_name("Graphic1.pdf")
                .suggested_name(),
            "Graphic1.pdf"
        );
        assert_eq!(Dialog::new().suggested_name(), "drawing");
    }

    #[test]
    fn native_read_and_write_use_the_file_system() {
        let dir = std::env::temp_dir().join(format!("tracedraw-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.bin");
        write(&p, [1u8, 2, 3]).unwrap();
        assert_eq!(read(&p).unwrap(), vec![1, 2, 3]);
        assert!(read(dir.join("missing")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
