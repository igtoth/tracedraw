//! User interface strings. `tr("menu.file.open")` returns the string in
//! the active language, falling back to English, then to the key itself.
//!
//! Language files are flat `key = "value"` tables in `lang/` (a TOML
//! subset) embedded at build time, so the binary stays self-contained.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Languages shipped with the app, in display order.
pub const LANGUAGES: [(&str, &str); 12] = [
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
];

const FILES: [(&str, &str); 12] = [
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
];

struct State {
    code: String,
    table: HashMap<String, String>,
    english: HashMap<String, String>,
}

fn state() -> &'static RwLock<State> {
    static STATE: OnceLock<RwLock<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        let english = parse(FILES[0].1);
        RwLock::new(State {
            code: "en".into(),
            table: english.clone(),
            english,
        })
    })
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

/// Switch the active language; unknown codes fall back to English.
pub fn set_language(code: &str) {
    let src = FILES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, s)| *s)
        .unwrap_or(FILES[0].1);
    if let Ok(mut st) = state().write() {
        st.code = code.to_string();
        st.table = parse(src);
    }
}

pub fn language() -> String {
    state().read().map(|s| s.code.clone()).unwrap_or_default()
}

/// Right-to-left languages flip the docker side and text alignment.
pub fn is_rtl() -> bool {
    language() == "ar"
}

/// Translate a key.
pub fn tr(key: &str) -> String {
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

/// Pick the language from the system locale (`LANG`, `LC_ALL`, or the
/// platform API later), returning a shipped code.
pub fn system_language() -> &'static str {
    let env = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default()
        .to_lowercase();
    let env = env.replace('_', "-");
    for (code, _) in LANGUAGES.iter() {
        if env.starts_with(&code.to_lowercase()) {
            return code;
        }
    }
    for (code, _) in LANGUAGES.iter() {
        let base = code.split('-').next().unwrap_or(code);
        if env.starts_with(base) {
            return code;
        }
    }
    "en"
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn fallback_to_english_then_key() {
        set_language("zz");
        assert_eq!(tr("menu.file"), "File");
        assert_eq!(tr("no.such.key"), "key");
    }
}
