//! Spell check over the document's text. Dictionaries: word lists found on
//! the system (`/usr/share/dict/*`, Hunspell `.dic` files), a built-in
//! core list of very common English and Portuguese words, and the user's
//! own additions (`<config>/dictionary.txt`).

use crate::app::App;
use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};
use tracedraw_core::{document::ShapeKind, Command, ShapeId, TextSpan};

const CORE_WORDS: &str = include_str!("../lang/core-words.txt");

fn dictionary() -> &'static RwLock<HashSet<String>> {
    static DICT: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();
    DICT.get_or_init(|| {
        let mut set: HashSet<String> = CORE_WORDS
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();
        for path in candidate_paths() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                for line in text.lines() {
                    // Hunspell .dic: word/FLAGS; first line is a count.
                    let w = line.split('/').next().unwrap_or("").trim();
                    if !w.is_empty() && !w.chars().all(|c| c.is_ascii_digit()) {
                        set.insert(w.to_lowercase());
                    }
                }
            }
        }
        if let Some(p) = user_dictionary_path() {
            if let Ok(text) = std::fs::read_to_string(p) {
                for w in text.split_whitespace() {
                    set.insert(w.to_lowercase());
                }
            }
        }
        RwLock::new(set)
    })
}

fn candidate_paths() -> Vec<std::path::PathBuf> {
    let mut v = vec![
        std::path::PathBuf::from("/usr/share/dict/words"),
        std::path::PathBuf::from("/usr/share/dict/american-english"),
        std::path::PathBuf::from("/usr/share/dict/british-english"),
        std::path::PathBuf::from("/usr/share/dict/portuguese"),
        std::path::PathBuf::from("/usr/share/dict/brazilian"),
    ];
    for dir in [
        "/usr/share/hunspell",
        "/usr/share/myspell",
        "/Library/Spelling",
        "/usr/local/share/hunspell",
    ] {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("dic") {
                    v.push(p);
                }
            }
        }
    }
    if let Some(cfg) = crate::settings::config_dir() {
        if let Ok(rd) = std::fs::read_dir(cfg.join("dictionaries")) {
            for e in rd.flatten() {
                v.push(e.path());
            }
        }
    }
    v
}

fn user_dictionary_path() -> Option<std::path::PathBuf> {
    crate::settings::config_dir().map(|d| d.join("dictionary.txt"))
}

pub fn add_user_word(word: &str) {
    if let Ok(mut d) = dictionary().write() {
        d.insert(word.to_lowercase());
    }
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
    dictionary().read().map(|d| d.contains(&w)).unwrap_or(true)
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
    let size = dictionary().read().map(|d| d.len()).unwrap_or(0);
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

/// Suggestions by edit distance 1 against the dictionary (capped), plus
/// a split of camel-cased or run-together words.
pub fn suggest(word: &str) -> Vec<String> {
    let w = word.to_lowercase();
    let Ok(d) = dictionary().read() else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    let chars: Vec<char> = w.chars().collect();
    let alphabet = "abcdefghijklmnopqrstuvwxyzáéíóúâêôãõçàü";
    let mut push = |s: String| {
        if s != w && d.contains(&s) && !out.contains(&s) && out.len() < 8 {
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
