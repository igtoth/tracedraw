//! User interface strings. `tr("menu.file.open")` returns the string in
//! the active language, falling back to English, then to the key itself.
//!
//! Language files are flat `key = "value"` tables in `lang/` (a TOML
//! subset) embedded at build time, so the binary stays self-contained.
//! The browser build embeds English only and fetches another language's
//! file from the page's `lang/` folder when it is chosen (English shows
//! until it arrives), which keeps the module smaller.

use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};

/// Languages shipped with the app, in display order.
pub const LANGUAGES: [(&str, &str); 18] = [
    ("en", "English"),
    ("zh-CN", "中文（简体）"),
    ("hi", "हिन्दी"),
    ("es", "Español"),
    ("fr", "Français"),
    ("ar", "العربية"),
    ("bn", "বাংলা"),
    ("ru", "Русский"),
    ("pt-BR", "Português (Brasil)"),
    ("id", "Bahasa Indonesia"),
    ("de", "Deutsch"),
    ("ja", "日本語"),
    ("zh-TW", "中文（繁體）"),
    ("it", "Italiano"),
    ("nl", "Nederlands"),
    ("pl", "Polski"),
    ("tr", "Türkçe"),
    ("cs", "Čeština"),
];

/// The language files built into the program: all of them on the
/// desktop.
#[cfg(not(target_arch = "wasm32"))]
const FILES: [(&str, &str); 18] = [
    ("en", include_str!("../lang/en.toml")),
    ("zh-CN", include_str!("../lang/zh-CN.toml")),
    ("hi", include_str!("../lang/hi.toml")),
    ("es", include_str!("../lang/es.toml")),
    ("fr", include_str!("../lang/fr.toml")),
    ("ar", include_str!("../lang/ar.toml")),
    ("bn", include_str!("../lang/bn.toml")),
    ("ru", include_str!("../lang/ru.toml")),
    ("pt-BR", include_str!("../lang/pt-BR.toml")),
    ("id", include_str!("../lang/id.toml")),
    ("de", include_str!("../lang/de.toml")),
    ("ja", include_str!("../lang/ja.toml")),
    ("zh-TW", include_str!("../lang/zh-TW.toml")),
    ("it", include_str!("../lang/it.toml")),
    ("nl", include_str!("../lang/nl.toml")),
    ("pl", include_str!("../lang/pl.toml")),
    ("tr", include_str!("../lang/tr.toml")),
    ("cs", include_str!("../lang/cs.toml")),
];

/// The browser build embeds English only; the other languages are
/// fetched from the page (`lang/<code>.toml`, see [`file_url`]).
#[cfg(target_arch = "wasm32")]
const FILES: [(&str, &str); 1] = [("en", include_str!("../lang/en.toml"))];

/// The built-in file of a language.
fn builtin(code: &str) -> Option<&'static str> {
    FILES.iter().find(|(c, _)| *c == code).map(|(_, s)| *s)
}

/// Where the browser build fetches a language's file from, relative to
/// the page; `None` for English (always built in) and unknown codes.
pub fn file_url(code: &str) -> Option<String> {
    LANGUAGES
        .iter()
        .any(|(c, _)| *c == code && *c != "en")
        .then(|| format!("lang/{code}.toml"))
}

struct State {
    /// The chosen language.
    code: String,
    /// The language `table` holds: `code`, or English while the chosen
    /// language's file is on its way (browser) or the code is unknown.
    shown: String,
    table: HashMap<String, String>,
    english: HashMap<String, String>,
    /// Languages fetched by the browser build, kept to switch back.
    fetched: HashMap<String, HashMap<String, String>>,
    /// Languages whose file is being fetched.
    pending: HashSet<String>,
}

impl State {
    fn new(english: HashMap<String, String>) -> Self {
        State {
            code: "en".into(),
            shown: "en".into(),
            table: english.clone(),
            english,
            fetched: HashMap::new(),
            pending: HashSet::new(),
        }
    }

    /// Make `code` the language, from its built-in file or one fetched
    /// before; otherwise English shows and the file to fetch is returned
    /// (once: not while it is already on its way).
    fn choose(&mut self, code: &str, builtin: Option<&str>) -> Option<String> {
        self.code = code.to_string();
        let table = match builtin {
            Some(src) => Some(parse(src)),
            None => self.fetched.get(code).cloned(),
        };
        if let Some(table) = table {
            self.table = table;
            self.shown = code.to_string();
            return None;
        }
        self.table = self.english.clone();
        self.shown = "en".into();
        let url = file_url(code)?;
        self.pending.insert(code.to_string()).then_some(url)
    }

    /// A fetched language file arrived (`None`: the fetch failed, and
    /// choosing the language again tries again). Returns whether the
    /// interface changed language.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn arrived(&mut self, code: &str, src: Option<&str>) -> Result<bool, String> {
        self.pending.remove(code);
        let src = src.ok_or_else(|| format!("could not fetch {code}"))?;
        let table = parse(src);
        // A server's error page is not a language file.
        if !table.contains_key("menu.file") {
            return Err(format!("{code} is not a language file"));
        }
        let shown = self.code == code;
        if shown {
            self.table = table.clone();
            self.shown = code.to_string();
        }
        self.fetched.insert(code.to_string(), table);
        Ok(shown)
    }
}

fn state() -> &'static RwLock<State> {
    static STATE: OnceLock<RwLock<State>> = OnceLock::new();
    STATE.get_or_init(|| RwLock::new(State::new(parse(FILES[0].1))))
}

/// Parse the flat `key = "value"` format. Lines starting with `#` and
/// blank lines are ignored; values support `\"`, `\\` and `\n`.
pub fn parse(src: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in src.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((key, rest)) = line.split_once('=') else {
            continue;
        };
        let rest = rest.trim();
        let Some(body) = rest.strip_prefix('"') else {
            continue;
        };
        let mut value = String::new();
        let mut chars = body.chars();
        while let Some(c) = chars.next() {
            match c {
                '\\' => match chars.next() {
                    Some('n') => value.push('\n'),
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some(o) => {
                        value.push('\\');
                        value.push(o);
                    }
                    None => {}
                },
                '"' => break,
                o => value.push(o),
            }
        }
        map.insert(key.trim().to_string(), value);
    }
    map
}

/// Switch the active language; unknown codes fall back to English. In
/// the browser a language not fetched yet shows in English until its
/// file arrives ([`language_arrived`]).
pub fn set_language(code: &str) {
    let fetch = match state().write() {
        Ok(mut st) => st.choose(code, builtin(code)),
        Err(_) => None,
    };
    if let Some(url) = fetch {
        fetch_file(code, url);
    }
}

#[cfg(target_arch = "wasm32")]
fn fetch_file(code: &str, url: String) {
    crate::web::fetch_language(code.to_string(), url);
}

/// The desktop has every language built in.
#[cfg(not(target_arch = "wasm32"))]
fn fetch_file(code: &str, url: String) {
    log::warn!("language {code} is not built in ({url})");
}

/// A language file the browser build fetched (`None`: the fetch failed;
/// English stays). Returns whether the interface changed language, so
/// the caller asks for a frame.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn language_arrived(code: &str, src: Option<&str>) -> bool {
    let Ok(mut st) = state().write() else {
        return false;
    };
    st.arrived(code, src).unwrap_or_else(|e| {
        log::warn!("interface language: {e}; English stays");
        false
    })
}

/// Every key of the English table.
pub fn english_keys() -> Vec<String> {
    state()
        .read()
        .map(|s| s.english.keys().cloned().collect())
        .unwrap_or_default()
}

pub fn language() -> String {
    state().read().map(|s| s.code.clone()).unwrap_or_default()
}

/// Right-to-left languages flip the docker side and text alignment
/// (once the language shows: English while its file is on its way).
pub fn is_rtl() -> bool {
    state().read().map(|s| s.shown == "ar").unwrap_or(false)
}

#[cfg(test)]
thread_local! {
    /// A language for this thread only, so that a test can lay the
    /// interface out in another language without changing it for the
    /// tests running beside it.
    static THREAD_TABLE: std::cell::RefCell<Option<HashMap<String, String>>> =
        const { std::cell::RefCell::new(None) };
}

/// Run `f` with `code` as the language of this thread only (tests).
#[cfg(test)]
pub fn with_thread_language<R>(code: &str, f: impl FnOnce() -> R) -> R {
    let src = builtin(code).unwrap_or(FILES[0].1);
    let before = THREAD_TABLE.with(|t| t.replace(Some(parse(src))));
    let out = f();
    THREAD_TABLE.with(|t| *t.borrow_mut() = before);
    out
}

/// Translate a key.
pub fn tr(key: &str) -> String {
    #[cfg(test)]
    if let Some(v) = THREAD_TABLE.with(|t| t.borrow().as_ref().and_then(|m| m.get(key).cloned())) {
        return v;
    }
    if let Ok(st) = state().read() {
        if let Some(v) = st.table.get(key) {
            return v.clone();
        }
        if let Some(v) = st.english.get(key) {
            return v.clone();
        }
    }
    log::debug!("missing i18n key {key}");
    key.rsplit('.').next().unwrap_or(key).to_string()
}

/// Whether the English table has `key` (tests of generated keys).
#[cfg(test)]
pub fn has_english(key: &str) -> bool {
    state()
        .read()
        .map(|st| st.english.contains_key(key))
        .unwrap_or(false)
}

/// Translate with `{name}` placeholders replaced.
pub fn trf(key: &str, args: &[(&str, &str)]) -> String {
    let mut s = tr(key);
    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

/// The language to start in: the one saved in the settings, else the
/// system's ([`system_language`]).
pub fn start_language(saved: &str) -> String {
    choose_start(saved, system_language())
}

/// The saved language when there is one, else the system's.
fn choose_start(saved: &str, system: &str) -> String {
    if saved.is_empty() {
        system.to_string()
    } else {
        saved.to_string()
    }
}

/// Pick the language from the system locale (`LANG`, `LC_ALL`, or the
/// platform API later), returning a shipped code.
#[cfg(not(target_arch = "wasm32"))]
pub fn system_language() -> &'static str {
    let env = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    language_for_locale(&env)
}

/// In the browser: the first of the browser's preferred languages
/// (`navigator.languages`) that is shipped, read once.
#[cfg(target_arch = "wasm32")]
pub fn system_language() -> &'static str {
    static CHOSEN: OnceLock<&'static str> = OnceLock::new();
    CHOSEN.get_or_init(|| preferred_language(&crate::web::browser_languages()))
}

/// The first of a list of preferred locales (such as a browser's
/// `en-US`, `pt-BR`, `zh-Hant-TW`) that has a shipped language; English
/// when none has.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn preferred_language(locales: &[String]) -> &'static str {
    locales
        .iter()
        .find_map(|l| shipped_language(l))
        .unwrap_or("en")
}

/// The interface language for a locale such as `pt_BR.UTF-8`, `zh_TW` or
/// `de`; English when none matches.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn language_for_locale(locale: &str) -> &'static str {
    shipped_language(locale).unwrap_or("en")
}

/// The shipped language of a locale, if any.
fn shipped_language(locale: &str) -> Option<&'static str> {
    let env = locale.to_lowercase().replace('_', "-");
    // Chinese written in Traditional characters: Taiwan, Hong Kong, Macau.
    if ["zh-tw", "zh-hk", "zh-mo", "zh-hant"]
        .iter()
        .any(|t| env.starts_with(t))
    {
        return Some("zh-TW");
    }
    for (code, _) in LANGUAGES.iter() {
        if env.starts_with(&code.to_lowercase()) {
            return Some(code);
        }
    }
    for (code, _) in LANGUAGES.iter() {
        let base = code.split('-').next().unwrap_or(code);
        if env.starts_with(base) {
            return Some(code);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales_choose_the_interface_language() {
        assert_eq!(language_for_locale("zh_TW.UTF-8"), "zh-TW");
        assert_eq!(language_for_locale("zh_HK.UTF-8"), "zh-TW");
        assert_eq!(language_for_locale("zh_CN.UTF-8"), "zh-CN");
        assert_eq!(language_for_locale("zh_SG"), "zh-CN");
        assert_eq!(language_for_locale("cs_CZ.UTF-8"), "cs");
        assert_eq!(language_for_locale("nl_BE"), "nl");
        assert_eq!(language_for_locale("it_IT"), "it");
        assert_eq!(language_for_locale("pl_PL"), "pl");
        assert_eq!(language_for_locale("tr_TR.UTF-8"), "tr");
        assert_eq!(language_for_locale("pt_BR.UTF-8"), "pt-BR");
        assert_eq!(language_for_locale("sv_SE"), "en");
        assert_eq!(language_for_locale(""), "en");
    }

    /// Browser language tags (`navigator.languages`).
    #[test]
    fn browser_tags_choose_the_interface_language() {
        for (tag, code) in [
            ("pt-BR", "pt-BR"),
            ("pt", "pt-BR"),
            ("zh-Hant-TW", "zh-TW"),
            ("zh-HK", "zh-TW"),
            ("zh-Hans-CN", "zh-CN"),
            ("es-419", "es"),
            ("en-US", "en"),
            ("fr-CA", "fr"),
            ("de-AT", "de"),
        ] {
            assert_eq!(language_for_locale(tag), code, "{tag}");
        }
        let list = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // The first shipped one wins; English when it comes first.
        assert_eq!(
            preferred_language(&list(&["sv-SE", "fi", "de-CH", "fr"])),
            "de"
        );
        assert_eq!(preferred_language(&list(&["en-GB", "de"])), "en");
        assert_eq!(preferred_language(&list(&["sv", "fi"])), "en");
        assert_eq!(preferred_language(&[]), "en");
    }

    /// The saved language first, then the system's (the browser's list),
    /// which is English when nothing matches.
    #[test]
    fn the_start_language_prefers_the_saved_one() {
        assert_eq!(choose_start("ja", "de"), "ja");
        assert_eq!(choose_start("", "de"), "de");
        let none = preferred_language(&["sv".to_string()]);
        assert_eq!(choose_start("", none), "en");
    }

    #[test]
    fn parses_escapes_and_comments() {
        let m = parse("# c\na.b = \"x \\\"y\\\" \\n z\"\nbad\n");
        assert_eq!(m.get("a.b").map(String::as_str), Some("x \"y\" \n z"));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn every_language_covers_english_keys() {
        let en = parse(FILES[0].1);
        assert!(en.len() > 100);
        for (code, src) in FILES.iter().skip(1) {
            let t = parse(src);
            let missing: Vec<_> = en.keys().filter(|k| !t.contains_key(*k)).collect();
            assert!(missing.is_empty(), "{code} is missing {missing:?}");
        }
    }

    /// The desktop keeps every language built in, in the order of the
    /// language list.
    #[test]
    fn the_desktop_builds_every_language_in() {
        assert_eq!(FILES.len(), LANGUAGES.len());
        for ((file, _), (code, _)) in FILES.iter().zip(LANGUAGES.iter()) {
            assert_eq!(file, code);
            assert!(builtin(code).is_some(), "{code}");
        }
    }

    #[test]
    fn the_browser_fetches_every_language_but_english() {
        assert_eq!(file_url("en"), None);
        assert_eq!(file_url("zz"), None);
        assert_eq!(file_url("pt-BR").as_deref(), Some("lang/pt-BR.toml"));
        for (code, _) in LANGUAGES.iter().skip(1) {
            let url = file_url(code).unwrap();
            let path = format!("{}/{url}", env!("CARGO_MANIFEST_DIR"));
            assert!(std::path::Path::new(&path).is_file(), "{path}");
        }
    }

    /// The browser build: English shows until the chosen language's file
    /// arrives, then the interface switches; the file is asked for once.
    #[test]
    fn a_fetched_language_shows_english_until_it_arrives() {
        let de = builtin("de").unwrap();
        let mut st = State::new(parse(FILES[0].1));
        assert_eq!(st.choose("de", None).as_deref(), Some("lang/de.toml"));
        assert_eq!(st.code, "de");
        assert_eq!(st.shown, "en");
        assert_eq!(st.table.get("menu.file").map(String::as_str), Some("File"));
        // Chosen again while on its way: not asked for twice.
        assert_eq!(st.choose("de", None), None);
        assert_eq!(st.arrived("de", Some(de)), Ok(true));
        assert_eq!(st.shown, "de");
        assert_eq!(st.table, parse(de));
        // Back to English and to German again: no new fetch.
        assert_eq!(st.choose("en", builtin("en")), None);
        assert_eq!(st.shown, "en");
        assert_eq!(st.choose("de", None), None);
        assert_eq!(st.table, parse(de));
    }

    /// A file that arrives after another language was chosen is kept for
    /// later and does not change the interface.
    #[test]
    fn a_late_language_file_is_kept_but_not_shown() {
        let fr = builtin("fr").unwrap();
        let mut st = State::new(parse(FILES[0].1));
        assert!(st.choose("fr", None).is_some());
        assert_eq!(st.choose("it", None).as_deref(), Some("lang/it.toml"));
        assert_eq!(st.arrived("fr", Some(fr)), Ok(false));
        assert_eq!(st.shown, "en");
        assert_eq!(st.choose("fr", None), None);
        assert_eq!(st.shown, "fr");
    }

    /// A missing or broken language file leaves the interface in English,
    /// and choosing the language again asks for the file again.
    #[test]
    fn a_missing_language_falls_back_to_english() {
        let mut st = State::new(parse(FILES[0].1));
        assert!(st.choose("ja", None).is_some());
        assert!(st.arrived("ja", None).is_err());
        assert_eq!((st.code.as_str(), st.shown.as_str()), ("ja", "en"));
        assert_eq!(st.table.get("menu.file").map(String::as_str), Some("File"));
        assert!(st.choose("ja", None).is_some());
        let page = "<!doctype html>\n<meta charset=\"utf-8\">\n<title>404</title>";
        assert!(st.arrived("ja", Some(page)).is_err());
        assert_eq!(st.shown, "en");
        assert!(st.choose("ja", None).is_some());
        // Unknown codes have nothing to fetch.
        assert_eq!(st.choose("zz", None), None);
        assert_eq!(st.shown, "en");
    }

    #[test]
    fn fallback_to_english_then_key() {
        set_language("zz");
        assert_eq!(tr("menu.file"), "File");
        assert_eq!(tr("no.such.key"), "key");
    }
}
