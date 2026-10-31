//! Spelling dictionaries downloaded on request (#1405): Options > Text >
//! Spelling lists the Hunspell dictionaries of the wooorm/dictionaries
//! collection on GitHub (each with its own free licence, listed in
//! `ATTRIBUTION.md`), and Download fetches the chosen one's affix file,
//! word file and licence. They go to TraceDraw's own dictionaries folder
//! (desktop, Android); a browser has no folder, so it holds them in
//! memory and fetches them again at each visit (the browser's cache
//! serves them), as the settings remember them. Files the user picks
//! (Add Files) go the same way. See `docs/behavior/writing-tools.md`.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Mutex;

/// Where the dictionaries come from: `<BASE><code>/index.aff`,
/// `index.dic` and `license`.
pub const BASE: &str = "https://raw.githubusercontent.com/wooorm/dictionaries/main/dictionaries/";

/// One dictionary of the collection: its code there, the language's own
/// name and its licence (SPDX).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entry {
    pub code: &'static str,
    pub name: &'static str,
    pub license: &'static str,
}

/// The dictionaries offered, those of the interface's languages first.
pub const CATALOG: &[Entry] = &[
    Entry {
        code: "en",
        name: "English (US)",
        license: "MIT AND BSD",
    },
    Entry {
        code: "en-GB",
        name: "English (UK)",
        license: "MIT AND BSD",
    },
    Entry {
        code: "en-AU",
        name: "English (Australia)",
        license: "MIT AND BSD",
    },
    Entry {
        code: "en-CA",
        name: "English (Canada)",
        license: "MIT AND BSD",
    },
    Entry {
        code: "pt",
        name: "Português (Brasil)",
        license: "LGPL-3.0 OR MPL-2.0",
    },
    Entry {
        code: "pt-PT",
        name: "Português (Portugal)",
        license: "GPL-2.0 OR LGPL-2.1 OR MPL-1.1",
    },
    Entry {
        code: "es",
        name: "Español",
        license: "GPL-3.0 OR LGPL-3.0 OR MPL-1.1",
    },
    Entry {
        code: "fr",
        name: "Français",
        license: "MPL-2.0",
    },
    Entry {
        code: "de",
        name: "Deutsch (Deutschland)",
        license: "GPL-2.0 OR GPL-3.0",
    },
    Entry {
        code: "de-AT",
        name: "Deutsch (Österreich)",
        license: "GPL-2.0 OR GPL-3.0",
    },
    Entry {
        code: "de-CH",
        name: "Deutsch (Schweiz)",
        license: "GPL-2.0 OR GPL-3.0",
    },
    Entry {
        code: "it",
        name: "Italiano",
        license: "GPL-3.0",
    },
    Entry {
        code: "nl",
        name: "Nederlands",
        license: "BSD-3-Clause OR CC-BY-3.0",
    },
    Entry {
        code: "pl",
        name: "Polski",
        license: "GPL-3.0 OR LGPL-3.0 OR MPL-2.0",
    },
    Entry {
        code: "cs",
        name: "Čeština",
        license: "GPL-2.0",
    },
    Entry {
        code: "ru",
        name: "Русский",
        license: "BSD-3-Clause",
    },
    Entry {
        code: "tr",
        name: "Türkçe",
        license: "MIT",
    },
    Entry {
        code: "uk",
        name: "Українська",
        license: "GPL-3.0",
    },
    Entry {
        code: "bg",
        name: "Български",
        license: "GPL-2.0 OR LGPL-2.1 OR MPL-1.1",
    },
    Entry {
        code: "ca",
        name: "Català",
        license: "GPL-2.0 OR LGPL-2.1",
    },
    Entry {
        code: "cy",
        name: "Cymraeg",
        license: "LGPL-3.0",
    },
    Entry {
        code: "da",
        name: "Dansk",
        license: "GPL-2.0 OR LGPL-2.1 OR MPL-1.1",
    },
    Entry {
        code: "el",
        name: "Ελληνικά",
        license: "GPL-2.0 OR LGPL-2.1 OR MPL-1.1",
    },
    Entry {
        code: "et",
        name: "Eesti",
        license: "LGPL-2.1",
    },
    Entry {
        code: "eu",
        name: "Euskara",
        license: "GPL-2.0",
    },
    Entry {
        code: "fa",
        name: "فارسی",
        license: "Apache-2.0",
    },
    Entry {
        code: "ga",
        name: "Gaeilge",
        license: "GPL-2.0",
    },
    Entry {
        code: "gl",
        name: "Galego",
        license: "GPL-3.0",
    },
    Entry {
        code: "he",
        name: "עברית",
        license: "AGPL-3.0",
    },
    Entry {
        code: "hr",
        name: "Hrvatski",
        license: "LGPL-2.1 OR SISSL",
    },
    Entry {
        code: "hu",
        name: "Magyar",
        license: "GPL-2.0 OR LGPL-2.1 OR MPL-1.1",
    },
    Entry {
        code: "ko",
        name: "한국어",
        license: "GPL-2.0 OR LGPL-2.1 OR MPL-1.1",
    },
    Entry {
        code: "lt",
        name: "Lietuvių",
        license: "BSD-3-Clause",
    },
    Entry {
        code: "lv",
        name: "Latviešu",
        license: "LGPL-2.1",
    },
    Entry {
        code: "nb",
        name: "Norsk bokmål",
        license: "GPL-2.0",
    },
    Entry {
        code: "nn",
        name: "Norsk nynorsk",
        license: "GPL-2.0",
    },
    Entry {
        code: "ro",
        name: "Română",
        license: "GPL-2.0 OR LGPL-2.1 OR MPL-1.1",
    },
    Entry {
        code: "sk",
        name: "Slovenčina",
        license: "GPL-2.0 OR LGPL-2.1 OR MPL-1.1",
    },
    Entry {
        code: "sl",
        name: "Slovenščina",
        license: "GPL-3.0 OR LGPL-2.1",
    },
    Entry {
        code: "sr",
        name: "Српски",
        license: "GPL-2.0 OR LGPL-2.1 OR MPL-1.1 OR CC-BY-SA-3.0",
    },
    Entry {
        code: "sv",
        name: "Svenska",
        license: "LGPL-3.0",
    },
    Entry {
        code: "vi",
        name: "Tiếng Việt",
        license: "GPL-2.0",
    },
];

/// The entry with `code`.
pub fn entry(code: &str) -> Option<&'static Entry> {
    CATALOG.iter().find(|e| e.code == code)
}

/// The name the dictionary's files get (`en_GB`), which the spelling
/// options list and switch off.
pub fn file_stem(code: &str) -> String {
    code.replace('-', "_")
}

/// The addresses of a dictionary's affix file, word file and licence.
pub fn urls(code: &str) -> [String; 3] {
    ["index.aff", "index.dic", "license"].map(|f| format!("{BASE}{code}/{f}"))
}

/// What a finished download gives.
#[derive(Debug, Clone, PartialEq)]
pub enum Finished {
    Installed(String),
    Failed(String, String),
}

/// One download under way: the answers to its three requests.
struct Download {
    code: String,
    /// Tell the user when it ends (not for the browser's fetches at the
    /// start of a visit).
    quiet: bool,
    waiting: Vec<Receiver<crate::net::HttpResult>>,
    got: Vec<Option<String>>,
}

static DOWNLOADS: Mutex<Vec<Download>> = Mutex::new(Vec::new());

/// In a browser, once per visit: fetch the dictionaries downloaded on
/// earlier visits (`codes`, from the settings) again.
pub fn fetch_kept(codes: &[String]) {
    static DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !crate::files::WEB || DONE.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    for code in codes {
        start(code, true);
    }
}

/// The status bar's words for a finished download.
pub fn status(f: &Finished) -> String {
    let name = |code: &str| entry(code).map(|e| e.name).unwrap_or(code).to_string();
    match f {
        Finished::Installed(code) => {
            crate::i18n::trf("status.dictionary_installed", &[("name", &name(code))])
        }
        Finished::Failed(code, e) => crate::i18n::trf(
            "status.dictionary_failed",
            &[("name", &name(code)), ("e", e)],
        ),
    }
}

/// Whether `code` is being downloaded.
pub fn downloading(code: &str) -> bool {
    DOWNLOADS
        .lock()
        .map(|d| d.iter().any(|x| x.code == code))
        .unwrap_or(false)
}

/// Start downloading the dictionary `code` (nothing when it already is).
pub fn start(code: &str, quiet: bool) {
    if entry(code).is_none() || downloading(code) {
        return;
    }
    let mut waiting = Vec::new();
    for url in urls(code) {
        let (tx, rx) = std::sync::mpsc::channel();
        crate::net::spawn(
            crate::net::HttpRequest {
                method: "GET",
                url,
                headers: Vec::new(),
                body: String::new(),
                bytes: Vec::new(),
            },
            64 * 1024 * 1024,
            tx,
        );
        waiting.push(rx);
    }
    if let Ok(mut d) = DOWNLOADS.lock() {
        d.push(Download {
            code: code.to_string(),
            quiet,
            got: vec![None; waiting.len()],
            waiting,
        });
    }
}

/// Look at the downloads (each frame): finished ones are installed and
/// reported; the quiet ones are only logged when they fail.
pub fn poll() -> Vec<Finished> {
    let Ok(mut downloads) = DOWNLOADS.lock() else {
        return Vec::new();
    };
    if downloads.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    downloads.retain_mut(|d| {
        let mut failed = None;
        for (i, rx) in d.waiting.iter().enumerate() {
            if d.got[i].is_some() {
                continue;
            }
            match rx.try_recv() {
                Ok(Ok(resp)) if resp.status == 200 => d.got[i] = Some(resp.body),
                Ok(Ok(resp)) => failed = Some(format!("HTTP {}", resp.status)),
                Ok(Err(e)) => failed = Some(e),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => failed = Some("no answer".to_string()),
            }
        }
        if let Some(e) = failed {
            if !d.quiet {
                out.push(Finished::Failed(d.code.clone(), e));
            } else {
                log::warn!("the dictionary {} could not be fetched: {e}", d.code);
            }
            return false;
        }
        if d.got.iter().any(Option::is_none) {
            return true;
        }
        let [aff, dic, license] = [0, 1, 2].map(|i| d.got[i].take().unwrap_or_default());
        let installed = install(&file_stem(&d.code), &aff, &dic, &license);
        log::info!(
            "dictionary {}: {} bytes of words, {:?}",
            d.code,
            dic.len(),
            installed
        );
        match installed {
            Ok(()) if d.quiet => {}
            Ok(()) => out.push(Finished::Installed(d.code.clone())),
            Err(e) => out.push(Finished::Failed(d.code.clone(), e)),
        }
        false
    });
    out
}

/// The folder dictionaries are kept in: TraceDraw's own, none in a
/// browser (they are held in memory there).
fn keep_folder() -> Option<PathBuf> {
    crate::spell::own_folder().filter(|_| !crate::files::WEB)
}

/// Keep a dictionary: in TraceDraw's own folder, or in memory in a
/// browser. Its licence goes beside it.
pub fn install(stem: &str, aff: &str, dic: &str, license: &str) -> Result<(), String> {
    install_into(keep_folder().as_deref(), stem, aff, dic, license)
}

/// [`install`] into `dir` (`None`: in memory).
fn install_into(
    dir: Option<&Path>,
    stem: &str,
    aff: &str,
    dic: &str,
    license: &str,
) -> Result<(), String> {
    if dic.trim().is_empty() {
        return Err("empty word file".to_string());
    }
    let files = [
        (format!("{stem}.aff"), aff),
        (format!("{stem}.dic"), dic),
        (format!("{stem}.license.txt"), license),
    ];
    match dir {
        Some(dir) => {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            for (name, text) in files {
                if !text.is_empty() {
                    std::fs::write(dir.join(name), text).map_err(|e| e.to_string())?;
                }
            }
            crate::spell::reload();
        }
        None => {
            for (name, text) in files.into_iter().take(2) {
                if !text.is_empty() {
                    crate::spell::add_in_memory(&name, text.as_bytes().to_vec());
                }
            }
        }
    }
    Ok(())
}

/// Add dictionary files the user picked (`.dic` and `.aff`): copied into
/// TraceDraw's own folder, or held in memory in a browser for the visit.
/// How many were added.
pub fn add_files(paths: &[PathBuf]) -> usize {
    add_files_into(keep_folder().as_deref(), paths)
}

/// [`add_files`] into `dir` (`None`: in memory).
fn add_files_into(dir: Option<&Path>, paths: &[PathBuf]) -> usize {
    let mut added = 0;
    for p in paths {
        let Some(name) = p.file_name().and_then(|n| n.to_str()).map(str::to_string) else {
            continue;
        };
        let ext = Path::new(&name)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if !matches!(ext.as_deref(), Some("dic" | "aff")) {
            continue;
        }
        let Ok(bytes) = crate::files::read(p) else {
            continue;
        };
        let kept = match dir {
            Some(dir) => std::fs::create_dir_all(dir)
                .and_then(|()| std::fs::write(dir.join(&name), &bytes))
                .is_ok(),
            None => {
                crate::spell::add_in_memory(&name, bytes);
                true
            }
        };
        added += usize::from(kept);
    }
    crate::spell::reload();
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #1405: every entry has a licence and a file name the spelling
    /// options can switch off; the addresses are the collection's.
    #[test]
    fn the_catalog_names_its_files_and_licences() {
        assert!(CATALOG.len() >= 40);
        for e in CATALOG {
            assert!(!e.license.is_empty() && !e.name.is_empty(), "{}", e.code);
            assert!(!file_stem(e.code).contains('-'));
        }
        assert_eq!(file_stem("pt-PT"), "pt_PT");
        let [aff, dic, lic] = urls("en-GB");
        assert_eq!(aff, format!("{BASE}en-GB/index.aff"));
        assert!(dic.ends_with("/en-GB/index.dic") && lic.ends_with("/en-GB/license"));
        assert!(entry("de").is_some() && entry("xx").is_none());
        // Every language of the interface that the collection has.
        for code in [
            "en", "pt", "es", "fr", "de", "it", "nl", "pl", "cs", "ru", "tr",
        ] {
            assert!(entry(code).is_some(), "{code}");
        }
    }

    /// A dictionary installed (as a download gives it) is used at once,
    /// with its licence beside it; files the user picks too; in a browser
    /// (no folder) they are held in memory.
    #[test]
    fn an_installed_dictionary_is_used() {
        let dir = std::env::temp_dir().join(format!("tracedraw-dl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        crate::spell::configure(crate::spell::SpellConfig {
            folder: Some(dir.clone()),
            disabled: Vec::new(),
        });
        assert!(!crate::spell::is_known("frumbleplex"));
        install_into(
            Some(&dir),
            "xx_DL",
            "SET UTF-8\nSFX S Y 1\nSFX S 0 s .\n",
            "1\nfrumbleplex/S\n",
            "MIT",
        )
        .expect("installed");
        assert!(dir.join("xx_DL.dic").is_file());
        assert!(dir.join("xx_DL.license.txt").is_file());
        assert!(crate::spell::is_known("frumbleplexs"));
        assert!(install_into(Some(&dir), "xx_EMPTY", "", " ", "").is_err());
        // Files the user picks.
        let picked = dir.join("picked");
        std::fs::create_dir_all(&picked).unwrap();
        std::fs::write(picked.join("yy_ADD.dic"), "1\nquorbistan\n").unwrap();
        std::fs::write(picked.join("notes.txt"), "x").unwrap();
        let n = add_files_into(
            Some(&dir),
            &[picked.join("yy_ADD.dic"), picked.join("notes.txt")],
        );
        assert_eq!(n, 1);
        assert!(crate::spell::is_known("quorbistan"));
        // In memory (a browser): listed and used, licence left out.
        install_into(None, "zz_MEM", "SET UTF-8\n", "1\nglimtrovic\n", "MIT").expect("memory");
        assert!(crate::spell::is_known("glimtrovic"));
        let found = crate::spell::find_dictionaries(None);
        assert!(found.iter().any(|d| d.name == "zz_MEM" && d.aff.is_some()));
        crate::spell::configure(crate::spell::SpellConfig::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
