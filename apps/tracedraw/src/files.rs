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
    std::fs::write(path, bytes)
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

/// Run the dialogs that finished since the last frame (browser). Called
/// once per frame before the interface is drawn.
pub fn run_finished(app: &mut App) {
    let jobs: Vec<(Job, PathBuf)> = FINISHED.with(|f| std::mem::take(&mut *f.borrow_mut()));
    for (job, path) in jobs {
        job(app, path.clone());
        forget(&path);
    }
}

/// The path picked for `key` by [`Dialog::pick_into`], once.
pub fn take_picked(key: &str) -> Option<PathBuf> {
    PICKED.with(|p| p.borrow_mut().remove(key))
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
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = self.native_pick() {
            then(app, path);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = app;
            self.web_pick(Box::new(then));
        }
    }

    /// Pick a file and keep its path for [`take_picked`]`(key)`.
    pub fn pick_into(self, key: &str) {
        let key = key.to_string();
        #[cfg(not(target_arch = "wasm32"))]
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
    /// with [`write`]. In a browser there is no dialog: the path is
    /// `/download/<file name>` and the write becomes a download.
    pub fn save_file(self, app: &mut App, then: impl FnOnce(&mut App, PathBuf)) {
        if let Some(path) = self.save_path() {
            then(app, path);
        }
    }

    /// The path to save to, or `None` when the user cancels: a save
    /// dialog natively, `/download/<file name>` in a browser (write it with
    /// [`write`] to download it).
    pub fn save_path(self) -> Option<PathBuf> {
        if WEB {
            let name = self.file_name.unwrap_or_else(|| "download".into());
            return Some(PathBuf::from("/download").join(name));
        }
        #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(not(target_arch = "wasm32"))]
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
        for (name, exts) in &self.filters {
            let exts: Vec<&str> = exts.iter().map(|e| e.as_str()).collect();
            d = d.add_filter(name, &exts);
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
