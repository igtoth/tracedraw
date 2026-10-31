//! Spell check over the document's text. Dictionaries: Hunspell
//! dictionaries (a `.dic` word file with its `.aff` affix file, read by
//! [`crate::hunspell`]) found in the system's folders, in TraceDraw's own
//! `dictionaries` folder and in a folder the user names (Options > Text >
//! Spelling, which also switches each one off); plain word lists
//! (`/usr/share/dict/*`, a `.dic` with no `.aff`); dictionaries held in
//! memory (downloaded or added in a browser, [`add_in_memory`]); the
//! spell checker of Windows (`spell_windows.rs`); a built-in core list of
//! very common English and Portuguese words; and the user's own additions
//! (`<config>/dictionary.txt`). A word is right when any of them knows it.
//! See `docs/behavior/writing-tools.md`.

use crate::app::App;
use crate::hunspell::Hunspell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};
use tracedraw_core::{document::ShapeKind, Command, ShapeId, TextSpan};

const CORE_WORDS: &str = include_str!("../lang/core-words.txt");

/// Which dictionaries to use: a folder of the user's and the names of
/// those switched off.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpellConfig {
    pub folder: Option<PathBuf>,
    pub disabled: Vec<String>,
}

/// A dictionary found on disk: its name (the file name without `.dic`,
/// such as `pt_BR`), its word file and its affix file (none for a plain
/// word list).
#[derive(Debug, Clone, PartialEq)]
pub struct DictionaryFile {
    pub name: String,
    pub dic: PathBuf,
    pub aff: Option<PathBuf>,
}

struct Loaded {
    plain: HashSet<String>,
    hunspell: Vec<Hunspell>,
    size: usize,
    /// The spell checker of Windows is used (not switched off).
    system: bool,
}

/// The folder under which dictionaries held in memory are named.
const MEMORY_ROOT: &str = "/memory-dictionaries";

/// Dictionary files held in memory, by file name (`pt_BR.dic`,
/// `pt_BR.aff`): in a browser, which has no folder to keep them in, the
/// ones downloaded or added during the visit (#1405).
fn memory() -> &'static RwLock<std::collections::BTreeMap<String, Vec<u8>>> {
    static FILES: OnceLock<RwLock<std::collections::BTreeMap<String, Vec<u8>>>> = OnceLock::new();
    FILES.get_or_init(|| RwLock::new(Default::default()))
}

/// Hold a dictionary file (`name.dic` or `name.aff`) in memory and read
/// the dictionaries again on the next check.
pub fn add_in_memory(file_name: &str, bytes: Vec<u8>) {
    memory()
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(file_name.to_string(), bytes);
    reload();
}

/// A dictionary file's bytes: from memory or from disk.
fn read_file(path: &Path) -> Option<Vec<u8>> {
    if let Ok(rest) = path.strip_prefix(MEMORY_ROOT) {
        let name = rest.to_str()?;
        return memory()
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
            .cloned();
    }
    std::fs::read(path).ok()
}

/// The dictionaries held in memory, as [`find_dictionaries`] lists them.
fn memory_dictionaries(kind: Kind, out: &mut Vec<DictionaryFile>) {
    let files = memory().read().unwrap_or_else(|e| e.into_inner());
    for name in files.keys() {
        let Some(stem) = name.strip_suffix(".dic") else {
            continue;
        };
        let lower = stem.to_lowercase();
        let wanted = match kind {
            Kind::Spelling => !lower.starts_with("hyph_") && !lower.starts_with("th_"),
            Kind::Hyphenation => lower.starts_with("hyph_"),
        };
        if !wanted || out.iter().any(|d| d.name == stem) {
            continue;
        }
        let aff = format!("{stem}.aff");
        out.push(DictionaryFile {
            name: stem.to_string(),
            dic: Path::new(MEMORY_ROOT).join(name),
            aff: (kind == Kind::Spelling && files.contains_key(&aff))
                .then(|| Path::new(MEMORY_ROOT).join(aff)),
        });
    }
}

/// The name under which the spell checker of Windows is listed and
/// switched off.
pub const SYSTEM_CHECKER: &str = "Windows";

type State = RwLock<(SpellConfig, Option<Arc<Loaded>>)>;

#[cfg(not(test))]
fn state() -> &'static State {
    static STATE: OnceLock<State> = OnceLock::new();
    STATE.get_or_init(|| RwLock::new((SpellConfig::default(), None)))
}

/// In tests each thread (each test) has dictionaries of its own: an app
/// made by another test configures them from its settings, and shared
/// they changed under a test checking a folder of its own.
#[cfg(test)]
fn state() -> &'static State {
    thread_local! {
        static STATE: &'static State =
            Box::leak(Box::new(RwLock::new((SpellConfig::default(), None))));
    }
    STATE.with(|s| *s)
}

/// Use the dictionaries `cfg` names from now on (read again on the next
/// check when they changed).
pub fn configure(cfg: SpellConfig) {
    let mut st = state().write().unwrap_or_else(|e| e.into_inner());
    if st.0 != cfg {
        *st = (cfg, None);
    }
}

/// Read the dictionaries again on the next check (files were added).
pub fn reload() {
    let mut st = state().write().unwrap_or_else(|e| e.into_inner());
    st.1 = None;
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// A number that changes each time dictionaries were added or the folders
/// are to be read again, so a list of them knows to look again.
pub fn generation() -> u64 {
    GENERATION.load(std::sync::atomic::Ordering::Relaxed)
}

fn loaded() -> Arc<Loaded> {
    if let Some(l) = state().read().unwrap_or_else(|e| e.into_inner()).1.clone() {
        return l;
    }
    let cfg = state().read().unwrap_or_else(|e| e.into_inner()).0.clone();
    let l = Arc::new(load(&cfg));
    let mut st = state().write().unwrap_or_else(|e| e.into_inner());
    if st.0 == cfg {
        st.1 = Some(l.clone());
    }
    l
}

fn load(cfg: &SpellConfig) -> Loaded {
    let mut plain: HashSet<String> = CORE_WORDS
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .collect();
    for path in word_lists() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            plain.extend(
                text.lines()
                    .map(|l| l.trim().to_lowercase())
                    .filter(|w| !w.is_empty()),
            );
        }
    }
    let mut hunspell = Vec::new();
    for d in find_dictionaries(cfg.folder.as_deref()) {
        if cfg.disabled.contains(&d.name) {
            continue;
        }
        let Some(dic) = read_file(&d.dic) else {
            continue;
        };
        let h = match d.aff.as_ref().and_then(|a| read_file(a)) {
            Some(aff) => Hunspell::from_bytes(&aff, &dic),
            None => Hunspell::from_word_list(&dic),
        };
        if !h.is_empty() {
            hunspell.push(h);
        }
    }
    let system = cfg!(windows) && !cfg.disabled.iter().any(|d| d == SYSTEM_CHECKER);
    let size = plain.len()
        + hunspell.iter().map(Hunspell::len).sum::<usize>()
        + if system && !system_languages().is_empty() {
            // Its own dictionaries: tens of thousands of words.
            100_000
        } else {
            0
        };
    Loaded {
        plain,
        hunspell,
        size,
        system,
    }
}

/// Plain word lists of the system, one word per line.
fn word_lists() -> Vec<PathBuf> {
    [
        "/usr/share/dict/words",
        "/usr/share/dict/american-english",
        "/usr/share/dict/british-english",
        "/usr/share/dict/portuguese",
        "/usr/share/dict/brazilian",
    ]
    .into_iter()
    .map(PathBuf::from)
    .filter(|p| p.is_file())
    .collect()
}

/// TraceDraw's own folder for dictionaries (`<config>/dictionaries`).
pub fn own_folder() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join("dictionaries"))
}

/// The folders searched, the user's first: TraceDraw's own folder and the
/// one named in the options (each with the folders inside it, three deep,
/// so a dictionary package unpacked there or another program's dictionary
/// folder works), then the system's ([`system_folders`]).
fn folders(extra: Option<&Path>) -> Vec<(PathBuf, usize)> {
    let mut v: Vec<(PathBuf, usize)> = Vec::new();
    if let Some(own) = own_folder() {
        v.push((own, 3));
    }
    if let Some(e) = extra {
        v.push((e.to_path_buf(), 3));
    }
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    v.extend(system_folders(
        cfg!(windows),
        cfg!(target_os = "macos"),
        &env,
        &|dir| {
            std::fs::read_dir(dir)
                .map(|rd| rd.flatten().map(|e| e.path()).collect())
                .unwrap_or_default()
        },
    ));
    v
}

/// The system's dictionary folders, with how deep to look in each (#1405):
/// - Linux and other Unix systems: the shared Hunspell folders;
/// - macOS: the system's and the user's `Library/Spelling`;
/// - Windows: the user's spelling folders (`%LOCALAPPDATA%` and
///   `%APPDATA%` `\Microsoft\Spelling\<language>`, where Hunspell
///   dictionaries for the system's spell checker and the words added to
///   it are kept) and `%APPDATA%\hunspell`;
/// - everywhere: the dictionary extensions of office suites installed in
///   the usual places (`<program>/share/extensions/dict-*`, inside an
///   application bundle on macOS), which hold Hunspell dictionaries for
///   many languages.
///
/// `env` reads an environment variable and `list` a folder's entries.
fn system_folders(
    windows: bool,
    macos: bool,
    env: &dyn Fn(&str) -> Option<PathBuf>,
    list: &dyn Fn(&Path) -> Vec<PathBuf>,
) -> Vec<(PathBuf, usize)> {
    let mut v: Vec<(PathBuf, usize)> = Vec::new();
    // Programs installed side by side under `parent`: their dictionary
    // extensions.
    let suites = |parent: &Path, inner: &str, v: &mut Vec<(PathBuf, usize)>| {
        for program in list(parent) {
            let ext = program.join(inner);
            if !list(&ext).is_empty() {
                v.push((ext, 2));
            }
        }
    };
    if windows {
        for (var, sub) in [
            ("LOCALAPPDATA", "Microsoft\\Spelling"),
            ("APPDATA", "Microsoft\\Spelling"),
            ("APPDATA", "hunspell"),
        ] {
            if let Some(base) = env(var) {
                v.push((base.join(sub), 2));
            }
        }
        for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
            if let Some(base) = env(var) {
                suites(&base, "share\\extensions", &mut v);
            }
        }
    } else {
        for dir in [
            "/usr/share/hunspell",
            "/usr/share/myspell",
            "/usr/share/myspell/dicts",
            "/usr/local/share/hunspell",
            "/Library/Spelling",
        ] {
            v.push((PathBuf::from(dir), 0));
        }
        if let Some(home) = env("HOME") {
            v.push((home.join("Library/Spelling"), 0));
            v.push((home.join(".local/share/hunspell"), 0));
        }
        if macos {
            suites(
                Path::new("/Applications"),
                "Contents/Resources/extensions",
                &mut v,
            );
        } else {
            for parent in ["/usr/lib", "/usr/lib64", "/opt", "/usr/local/lib"] {
                suites(Path::new(parent), "share/extensions", &mut v);
            }
        }
    }
    let mut seen = HashSet::new();
    v.retain(|(p, _)| seen.insert(p.clone()));
    v
}

/// What a folder scan looks for.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Spelling dictionaries and word lists.
    Spelling,
    /// Hyphenation patterns (`hyph_*.dic`).
    Hyphenation,
}

fn scan(dir: &Path, depth: usize, out: &mut Vec<DictionaryFile>) {
    scan_for(dir, depth, Kind::Spelling, out);
}

fn scan_for(dir: &Path, depth: usize, kind: Kind, out: &mut Vec<DictionaryFile>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            if depth > 0 {
                scan_for(&p, depth - 1, kind, out);
            }
            continue;
        }
        let is_dic = p
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| x.eq_ignore_ascii_case("dic"));
        let Some(name) = p.file_stem().and_then(|n| n.to_str()).map(str::to_string) else {
            continue;
        };
        // Hyphenation patterns and thesauri share the extension.
        let lower = name.to_lowercase();
        let hyph = lower.starts_with("hyph_");
        let wanted = match kind {
            Kind::Spelling => !hyph && !lower.starts_with("th_"),
            Kind::Hyphenation => hyph,
        };
        if !is_dic || !wanted {
            continue;
        }
        if out.iter().any(|d| d.name == name) {
            continue;
        }
        let aff = p.with_extension("aff");
        out.push(DictionaryFile {
            name,
            aff: (kind == Kind::Spelling && aff.is_file()).then_some(aff),
            dic: p,
        });
    }
}

/// The dictionaries found, each name once (the first found wins).
pub fn find_dictionaries(extra: Option<&Path>) -> Vec<DictionaryFile> {
    let mut out = Vec::new();
    for (dir, depth) in folders(extra) {
        scan(&dir, depth, &mut out);
    }
    memory_dictionaries(Kind::Spelling, &mut out);
    out
}

/// The languages the spell checker of Windows checks (none elsewhere).
pub fn system_languages() -> Vec<String> {
    #[cfg(windows)]
    {
        crate::spell_windows::languages()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// The hyphenation pattern files found (`hyph_pt_BR`), each name once.
pub fn find_hyphenation(extra: Option<&Path>) -> Vec<DictionaryFile> {
    let mut out = Vec::new();
    for (dir, depth) in folders(extra) {
        scan_for(&dir, depth, Kind::Hyphenation, &mut out);
    }
    memory_dictionaries(Kind::Hyphenation, &mut out);
    out
}

/// The patterns of a hyphenation file: its first line names the
/// character set the rest is written in.
pub fn read_hyphenation(path: &Path) -> Option<tracedraw_text::hyphen::PatternSet> {
    let bytes = read_file(path)?;
    let first = bytes.split(|b| *b == b'\n').next().unwrap_or(&[]);
    let enc = crate::hunspell::encoding_for(&String::from_utf8_lossy(first));
    let (text, _, _) = enc.decode(&bytes);
    tracedraw_text::hyphen::PatternSet::parse(&text)
}

/// Hyphenate with the pattern file named `name` (found in the dictionary
/// folders; empty: the built-in patterns). False when it was not found or
/// held no patterns, the built-in ones then being used.
pub fn use_hyphenation(name: &str, extra: Option<&Path>) -> bool {
    static CURRENT: OnceLock<RwLock<(String, Option<PathBuf>)>> = OnceLock::new();
    let current = CURRENT.get_or_init(|| RwLock::new((String::new(), None)));
    let key = (name.to_string(), extra.map(Path::to_path_buf));
    if *current.read().unwrap_or_else(|e| e.into_inner()) == key {
        return true;
    }
    let set = if name.is_empty() {
        None
    } else {
        find_hyphenation(extra)
            .into_iter()
            .find(|d| d.name == name)
            .and_then(|d| read_hyphenation(&d.dic))
    };
    let found = name.is_empty() || set.is_some();
    tracedraw_text::hyphen::set_patterns(set);
    *current.write().unwrap_or_else(|e| e.into_inner()) = key;
    found
}

fn user_dictionary_path() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join("dictionary.txt"))
}

/// The user's own words (Add in the Spell Checker), lower case.
fn user_words() -> &'static RwLock<HashSet<String>> {
    static USER: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();
    USER.get_or_init(|| {
        let words = user_dictionary_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| t.split_whitespace().map(str::to_lowercase).collect())
            .unwrap_or_default();
        RwLock::new(words)
    })
}

pub fn add_user_word(word: &str) {
    user_words()
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(word.to_lowercase());
    if let Some(p) = user_dictionary_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mut text = std::fs::read_to_string(&p).unwrap_or_default();
        text.push_str(word);
        text.push('\n');
        let _ = std::fs::write(p, text);
    }
}

pub fn is_known(word: &str) -> bool {
    let w = word.to_lowercase();
    if w.chars().any(|c| c.is_ascii_digit()) || w.chars().count() < 2 {
        return true;
    }
    let l = loaded();
    let user = user_words().read().unwrap_or_else(|e| e.into_inner());
    let known = |p: &str| {
        let lower = p.to_lowercase();
        l.plain.contains(&lower)
            || user.contains(&lower)
            || l.hunspell.iter().any(|h| h.check(p))
            || (l.system && system_knows(p))
    };
    if known(word) {
        return true;
    }
    // A hyphenated word whose parts are words.
    word.contains('-')
        && word
            .split('-')
            .filter(|p| !p.is_empty())
            .all(|p| p.chars().count() < 2 || known(p))
}

/// Whether the spell checker of Windows knows `word` (never elsewhere).
fn system_knows(word: &str) -> bool {
    #[cfg(windows)]
    {
        crate::spell_windows::is_known(word)
    }
    #[cfg(not(windows))]
    {
        let _ = word;
        false
    }
}

/// Words in a text that the dictionary does not know.
pub fn unknown_words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.split(|c: char| !(c.is_alphabetic() || c == '\'' || c == '-')) {
        let w = raw.trim_matches(|c: char| c == '\'' || c == '-');
        if w.is_empty() || is_known(w) {
            continue;
        }
        if !out.iter().any(|x: &String| x == w) {
            out.push(w.to_string());
        }
    }
    out
}

/// Check every text object on the current page: (shape, word) pairs and
/// the dictionary size (0 means no dictionary was found).
pub fn check_document(app: &App) -> (Vec<(ShapeId, String)>, usize) {
    let size = loaded().size + user_words().read().map(|u| u.len()).unwrap_or(0);
    let mut out = Vec::new();
    if let Ok(page) = app.doc().page(app.page) {
        for s in page.layers.iter().flat_map(|l| &l.shapes) {
            if let ShapeKind::Text { spans, .. } = &s.kind {
                let t: String = spans.iter().map(|x| x.text.as_str()).collect();
                for w in unknown_words(&t) {
                    out.push((s.id, w));
                }
            }
        }
    }
    (out, size)
}

/// Suggestions: the Hunspell dictionaries' own (their replacement
/// tables and edits with their letters), then edits by one letter found
/// in the word lists; eight at most.
pub fn suggest(word: &str) -> Vec<String> {
    const MAX: usize = 8;
    let l = loaded();
    let mut out: Vec<String> = Vec::new();
    #[cfg(windows)]
    if l.system {
        out.extend(crate::spell_windows::suggest(word).into_iter().take(MAX));
    }
    for h in &l.hunspell {
        for s in h.suggest(word, MAX) {
            if !out.contains(&s) && out.len() < MAX {
                out.push(s);
            }
        }
    }
    let w = word.to_lowercase();
    let d = &l.plain;
    let chars: Vec<char> = w.chars().collect();
    let alphabet = "abcdefghijklmnopqrstuvwxyzáéíóúâêôãõçàü";
    let mut push = |s: String| {
        if s != w && d.contains(&s) && !out.contains(&s) && out.len() < MAX {
            out.push(s);
        }
    };
    for i in 0..chars.len() {
        // Deletion.
        let mut s: String = chars[..i].iter().collect();
        s.extend(chars[i + 1..].iter());
        push(s);
        // Transposition.
        if i + 1 < chars.len() {
            let mut t = chars.clone();
            t.swap(i, i + 1);
            push(t.iter().collect());
        }
        // Replacement.
        for a in alphabet.chars() {
            let mut t = chars.clone();
            t[i] = a;
            push(t.iter().collect());
        }
    }
    for i in 0..=chars.len() {
        for a in alphabet.chars() {
            let mut t = chars.clone();
            t.insert(i, a);
            push(t.iter().collect());
        }
    }
    out
}

pub fn replace_word(app: &mut App, id: ShapeId, word: &str, replacement: &str) {
    let Some(s) = app.doc().find_shape(id).cloned() else {
        return;
    };
    if let ShapeKind::Text {
        spans,
        origin,
        frame,
        align,
        para,
        on_path,
    } = s.kind
    {
        let new_spans: Vec<TextSpan> = spans
            .into_iter()
            .map(|sp| {
                let text = replace_whole_word(&sp.text, word, replacement);
                TextSpan { text, ..sp }
            })
            .collect();
        app.run(Command::SetShapeKind {
            shape: id,
            kind: ShapeKind::Text {
                spans: new_spans,
                origin,
                frame,
                align,
                para,
                on_path,
            },
        });
    }
}

fn replace_whole_word(text: &str, word: &str, replacement: &str) -> String {
    let mut out = String::new();
    let mut cur = String::new();
    let flush = |cur: &mut String, out: &mut String| {
        if !cur.is_empty() {
            if *cur == word {
                out.push_str(replacement);
            } else {
                out.push_str(cur);
            }
            cur.clear();
        }
    };
    for c in text.chars() {
        if c.is_alphabetic() || c == '\'' || c == '-' {
            cur.push(c);
        } else {
            flush(&mut cur, &mut out);
            out.push(c);
        }
    }
    flush(&mut cur, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_words_are_known_and_nonsense_is_not() {
        assert!(is_known("the"));
        assert!(is_known("casa"));
        assert!(!is_known("qzxvbnm"));
        assert_eq!(unknown_words("the qzxvbnm casa 42"), vec!["qzxvbnm"]);
    }

    /// Dictionaries in a folder of the user's: found with the folders
    /// inside it, hyphenation patterns and thesauri left out, a `.dic`
    /// without `.aff` read as a word list; used once configured, and not
    /// once switched off.
    #[test]
    fn dictionaries_in_a_folder_are_found_and_used() {
        let dir = std::env::temp_dir().join(format!("tracedraw-dicts-{}", std::process::id()));
        let inner = dir.join("blorfish");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(
            inner.join("xx_TEST.aff"),
            "SET UTF-8\nSFX S Y 1\nSFX S 0 s .\nPFX U Y 1\nPFX U 0 un .\n",
        )
        .unwrap();
        std::fs::write(inner.join("xx_TEST.dic"), "2\nblorf/SU\nZquee\n").unwrap();
        std::fs::write(dir.join("hyph_xx.dic"), "UTF-8\n1a\n").unwrap();
        std::fs::write(dir.join("th_xx.dic"), "x").unwrap();
        std::fs::write(dir.join("plainlist.dic"), "wibblewort\n").unwrap();
        let found = find_dictionaries(Some(&dir));
        let names: Vec<&str> = found.iter().map(|d| d.name.as_str()).collect();
        assert!(
            names.contains(&"xx_TEST") && names.contains(&"plainlist"),
            "{names:?}"
        );
        assert!(!names
            .iter()
            .any(|n| n.starts_with("hyph_") || n.starts_with("th_")));
        let test = found.iter().find(|d| d.name == "xx_TEST").unwrap();
        assert!(test.aff.is_some());
        assert!(found
            .iter()
            .find(|d| d.name == "plainlist")
            .unwrap()
            .aff
            .is_none());

        assert!(!is_known("unblorfs"));
        configure(SpellConfig {
            folder: Some(dir.clone()),
            disabled: Vec::new(),
        });
        assert!(is_known("unblorfs"));
        assert!(is_known("Blorf"));
        assert!(is_known("Zquee"));
        assert!(is_known("wibblewort"));
        assert!(is_known("blorf-wibblewort"));
        assert!(suggest("blorff").contains(&"blorf".to_string()));
        configure(SpellConfig {
            folder: Some(dir.clone()),
            disabled: vec!["xx_TEST".into()],
        });
        assert!(!is_known("unblorfs"));
        assert!(is_known("wibblewort"));
        configure(SpellConfig::default());
        // Hyphenation files beside them, in their own character set.
        std::fs::write(
            inner.join("hyph_yy.dic"),
            b"ISO8859-1\nLEFTHYPHENMIN 1\n1\xe7a\n",
        )
        .unwrap();
        let hyph = find_hyphenation(Some(&dir));
        let names: Vec<&str> = hyph.iter().map(|d| d.name.as_str()).collect();
        assert!(
            names.contains(&"hyph_xx") && names.contains(&"hyph_yy"),
            "{names:?}"
        );
        assert_eq!(names.len(), 2);
        let yy = hyph.iter().find(|d| d.name == "hyph_yy").unwrap();
        let set = read_hyphenation(&yy.dic).expect("patterns");
        assert_eq!((set.len(), set.left_min), (1, 1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Real hyphenation dictionaries, where `HUNSPELL_TEST_DIR` holds
    /// them (not in the repository): English, Brazilian Portuguese,
    /// German (eight-bit, two levels) and Italian (apostrophes, words
    /// followed by punctuation, #255).
    #[test]
    #[ignore]
    fn real_hyphenation_files() {
        let Some(dir) = std::env::var_os("HUNSPELL_TEST_DIR") else {
            return;
        };
        let dir = PathBuf::from(dir);
        let found = find_hyphenation(Some(&dir));
        let names: Vec<&str> = found.iter().map(|d| d.name.as_str()).collect();
        eprintln!("{names:?}");
        let cases: [(&str, &str, &[usize]); 8] = [
            ("hyph_en_US", "hyphenation", &[2, 6]),
            ("hyph_pt_BR", "computador", &[3, 5, 7]),
            ("hyph_de_DE", "Haustür", &[4]),
            ("hyph_de_DE", "Schifffahrt", &[6]),
            ("hyph_it_IT", "informazione", &[2, 5, 7]),
            ("hyph_it_IT", "informazione,", &[2, 5, 7]),
            ("hyph_it_IT", "dell'università", &[8, 11]),
            ("hyph_it_IT", "«sillabazione»", &[4, 6, 8]),
        ];
        for (name, word, want) in cases {
            let d = found.iter().find(|d| d.name == name).expect(name);
            let set = read_hyphenation(&d.dic).expect("patterns");
            tracedraw_text::hyphen::set_patterns(Some(set));
            let got = tracedraw_text::hyphen::break_points(word);
            tracedraw_text::hyphen::set_patterns(None);
            eprintln!("{name}: {word} -> {got:?}");
            for w in want {
                assert!(got.contains(w), "{name}: {word} {got:?} lacks {w}");
            }
        }
    }

    /// #1405: the system folders of each platform, with office suites'
    /// dictionary extensions where one is installed.
    #[test]
    fn system_folders_per_platform() {
        let env = |k: &str| match k {
            "LOCALAPPDATA" => Some(PathBuf::from("L")),
            "APPDATA" => Some(PathBuf::from("R")),
            "ProgramFiles" => Some(PathBuf::from("P")),
            "HOME" => Some(PathBuf::from("/home/u")),
            _ => None,
        };
        // Program folders: one office suite with its extensions.
        let list = |dir: &Path| -> Vec<PathBuf> {
            let d = dir.to_string_lossy().replace('\\', "/");
            match d.as_str() {
                "P" => vec![
                    PathBuf::from("P").join("Suite"),
                    PathBuf::from("P").join("Other"),
                ],
                "P/Suite/share/extensions" => vec![PathBuf::from("dict-en")],
                "/opt" => vec![PathBuf::from("/opt/suite")],
                "/opt/suite/share/extensions" => vec![PathBuf::from("dict-pt-BR")],
                "/Applications" => vec![PathBuf::from("/Applications/Suite.app")],
                "/Applications/Suite.app/Contents/Resources/extensions" => {
                    vec![PathBuf::from("dict-de")]
                }
                _ => Vec::new(),
            }
        };
        let norm = |v: Vec<(PathBuf, usize)>| -> Vec<(String, usize)> {
            v.into_iter()
                .map(|(p, d)| (p.to_string_lossy().replace('\\', "/"), d))
                .collect()
        };
        let win = norm(system_folders(true, false, &env, &list));
        assert_eq!(
            win,
            vec![
                ("L/Microsoft/Spelling".to_string(), 2),
                ("R/Microsoft/Spelling".to_string(), 2),
                ("R/hunspell".to_string(), 2),
                ("P/Suite/share/extensions".to_string(), 2),
            ]
        );
        let linux = norm(system_folders(false, false, &env, &list));
        assert!(linux.contains(&("/usr/share/hunspell".to_string(), 0)));
        assert!(linux.contains(&("/home/u/.local/share/hunspell".to_string(), 0)));
        assert!(linux.contains(&("/opt/suite/share/extensions".to_string(), 2)));
        assert!(!linux.iter().any(|(p, _)| p.starts_with("/Applications")));
        let mac = norm(system_folders(false, true, &env, &list));
        assert!(mac.contains(&("/home/u/Library/Spelling".to_string(), 0)));
        assert!(mac.contains(&(
            "/Applications/Suite.app/Contents/Resources/extensions".to_string(),
            2
        )));
    }

    /// Windows' lists of added words are UTF-16 with a byte order mark.
    #[test]
    fn utf16_word_lists_are_read() {
        let mut bytes = vec![0xff, 0xfe];
        for u in "zorblatt\r\n".encode_utf16() {
            bytes.extend(u.to_le_bytes());
        }
        let h = crate::hunspell::Hunspell::from_word_list(&bytes);
        assert!(h.check("zorblatt"));
    }

    #[test]
    fn suggestions_fix_a_typo() {
        let s = suggest("hte");
        assert!(s.contains(&"the".to_string()), "{s:?}");
    }

    #[test]
    fn whole_word_replace() {
        assert_eq!(
            replace_whole_word("a cat catalog", "cat", "dog"),
            "a dog catalog"
        );
    }
}
