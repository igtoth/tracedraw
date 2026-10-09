//! Thesaurus (Text > Writing Tools > Thesaurus). Reads MyThes-format
//! files (`th_*_v2.dat`): the first line names the encoding, then each
//! entry is `word|n` followed by `n` lines `(pos)|syn1|syn2|...`. Files
//! are looked up in the usual system folders plus a file the user picked
//! (remembered in settings). A tiny built-in English list keeps the
//! feature working when nothing is installed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// One sense of a word: part of speech and its synonyms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meaning {
    pub pos: String,
    pub synonyms: Vec<String>,
}

/// Where the active thesaurus came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    BuiltIn,
    File(PathBuf),
}

#[derive(Debug, Clone)]
pub struct Thesaurus {
    entries: HashMap<String, Vec<Meaning>>,
    pub source: Source,
}

/// Built-in fallback: a few dozen very common English words in MyThes format.
const BUILT_IN: &str = "UTF-8
bad|2
(adj)|poor|inferior|unsatisfactory|faulty
(adj)|harmful|damaging|unfavourable
beautiful|1
(adj)|lovely|attractive|pretty|gorgeous|stunning
begin|1
(verb)|start|commence|open|initiate
big|2
(adj)|large|huge|great|sizeable|vast
(adj)|important|major|significant
bright|2
(adj)|shining|brilliant|vivid|luminous
(adj)|clever|smart|intelligent|quick
build|1
(verb)|construct|erect|assemble|make
buy|1
(verb)|purchase|acquire|obtain
change|2
(verb)|alter|modify|transform|adjust
(noun)|alteration|modification|variation|shift
cheap|1
(adj)|inexpensive|affordable|low-cost|economical
clean|1
(adj)|spotless|tidy|neat|immaculate
cold|1
(adj)|chilly|cool|freezing|icy
dark|1
(adj)|dim|gloomy|shadowy|unlit
easy|1
(adj)|simple|straightforward|effortless|uncomplicated
end|2
(noun)|finish|conclusion|close|ending
(verb)|finish|conclude|stop|terminate
fast|1
(adj)|quick|rapid|swift|speedy
find|1
(verb)|discover|locate|uncover|detect
good|2
(adj)|fine|excellent|great|superior
(adj)|kind|generous|virtuous|decent
happy|1
(adj)|cheerful|glad|joyful|content|pleased
hard|2
(adj)|difficult|tough|demanding|challenging
(adj)|firm|solid|rigid|stiff
help|1
(verb)|assist|aid|support|back
hot|1
(adj)|warm|heated|boiling|scorching
idea|1
(noun)|thought|notion|concept|plan
important|1
(adj)|significant|essential|crucial|vital|key
large|1
(adj)|big|huge|great|sizeable|considerable
little|1
(adj)|small|tiny|minute|slight
make|1
(verb)|create|produce|build|form|construct
new|1
(adj)|fresh|recent|modern|novel|current
nice|1
(adj)|pleasant|agreeable|delightful|lovely
old|2
(adj)|aged|elderly|ancient|mature
(adj)|former|previous|earlier|past
pretty|1
(adj)|attractive|lovely|beautiful|charming
quick|1
(adj)|fast|rapid|swift|speedy|brisk
sad|1
(adj)|unhappy|sorrowful|downcast|gloomy
say|1
(verb)|state|declare|remark|mention|utter
show|1
(verb)|display|exhibit|present|reveal|demonstrate
small|1
(adj)|little|tiny|minute|compact|modest
smart|1
(adj)|clever|intelligent|bright|sharp
strong|1
(adj)|powerful|sturdy|robust|tough|solid
think|1
(verb)|consider|believe|reflect|ponder
use|1
(verb)|employ|utilise|apply|operate
weak|1
(adj)|feeble|frail|fragile|delicate
";

/// Parse a MyThes file body. Malformed lines are skipped; the parser
/// never fails, an unreadable file simply yields no entries.
pub fn parse(src: &str) -> HashMap<String, Vec<Meaning>> {
    let mut map: HashMap<String, Vec<Meaning>> = HashMap::new();
    let mut lines = src.lines();
    // First line: encoding name (already handled by the caller).
    let _ = lines.next();
    let mut lines = lines.peekable();
    while let Some(line) = lines.next() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let Some((word, count)) = line.rsplit_once('|') else {
            continue;
        };
        let Ok(count) = count.trim().parse::<usize>() else {
            continue;
        };
        let mut meanings = Vec::new();
        for _ in 0..count {
            let Some(m) = lines.peek() else {
                break;
            };
            let m = m.trim_end_matches('\r');
            if !m.starts_with('(') && !m.starts_with('|') && !m.starts_with('-') {
                // Not a meaning line: the entry was shorter than announced.
                break;
            }
            let _ = lines.next();
            let mut parts = m.split('|');
            let pos = parts
                .next()
                .unwrap_or("")
                .trim_matches(|c| c == '(' || c == ')')
                .trim()
                .to_string();
            let synonyms: Vec<String> =
                parts.map(clean_synonym).filter(|s| !s.is_empty()).collect();
            if !synonyms.is_empty() {
                meanings.push(Meaning { pos, synonyms });
            }
        }
        if !meanings.is_empty() {
            map.entry(word.trim().to_lowercase())
                .or_default()
                .extend(meanings);
        }
    }
    map
}

/// Drop annotations such as `(generic term)` or `(similar term)`.
fn clean_synonym(s: &str) -> String {
    let base = match s.find('(') {
        Some(i) => &s[..i],
        None => s,
    };
    base.trim().to_string()
}

/// Decode a file: the first line names the encoding; anything that is
/// not UTF-8 is treated as Latin-1, which covers the MyThes files in use.
fn decode(bytes: &[u8]) -> String {
    let first_line_end = bytes
        .iter()
        .position(|b| *b == b'\n')
        .unwrap_or(bytes.len());
    let head = String::from_utf8_lossy(&bytes[..first_line_end]).to_uppercase();
    if head.contains("UTF") {
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        bytes.iter().map(|b| *b as char).collect()
    }
}

/// Thesaurus files found on this system; the user's pick comes first.
pub fn candidate_files(user_file: Option<&Path>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(p) = user_file {
        if p.is_file() {
            out.push(p.to_path_buf());
        }
    }
    let mut dirs: Vec<PathBuf> = [
        "/usr/share/mythes",
        "/usr/share/hunspell",
        "/usr/share/myspell/dicts",
        "/usr/share/myspell",
        "/usr/local/share/mythes",
        "/Library/Spelling",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    if let Some(app) = std::env::var_os("APPDATA") {
        dirs.push(PathBuf::from(app).join("hunspell"));
    }
    if let Some(cfg) = crate::settings::config_dir() {
        dirs.push(cfg.join("thesaurus"));
    }
    let lang = crate::i18n::language();
    let lang_base = lang.split('-').next().unwrap_or("en").to_lowercase();
    let mut found: Vec<PathBuf> = Vec::new();
    for d in dirs {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                let name = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if name.starts_with("th_") && name.ends_with(".dat") {
                    found.push(p);
                }
            }
        }
    }
    // Files for the UI language first, then the rest alphabetically.
    found.sort();
    found.sort_by_key(|p| {
        let n = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        !n.starts_with(&format!("th_{lang_base}"))
    });
    for p in found {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

fn load_file(path: &Path) -> Option<Thesaurus> {
    let bytes = crate::files::read(path).ok()?;
    let entries = parse(&decode(&bytes));
    if entries.is_empty() {
        log::warn!("thesaurus {} has no entries", path.display());
        return None;
    }
    Some(Thesaurus {
        entries,
        source: Source::File(path.to_path_buf()),
    })
}

impl Thesaurus {
    pub fn built_in() -> Self {
        Thesaurus {
            entries: parse(BUILT_IN),
            source: Source::BuiltIn,
        }
    }

    /// First usable file among the candidates, else the built-in list.
    pub fn load(user_file: Option<&Path>) -> Self {
        for p in candidate_files(user_file) {
            if let Some(t) = load_file(&p) {
                return t;
            }
        }
        Thesaurus::built_in()
    }

    /// Number of head words.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Meanings of a word, case-insensitive.
    pub fn lookup(&self, word: &str) -> &[Meaning] {
        self.entries
            .get(&word.trim().to_lowercase())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

fn cache() -> &'static Mutex<Option<(Option<PathBuf>, Arc<Thesaurus>)>> {
    static CACHE: OnceLock<Mutex<Option<(Option<PathBuf>, Arc<Thesaurus>)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

/// The thesaurus for `user_file`, loaded once and shared.
pub fn get(user_file: Option<&Path>) -> Arc<Thesaurus> {
    let key = user_file.map(Path::to_path_buf);
    if let Ok(c) = cache().lock() {
        if let Some((k, t)) = c.as_ref() {
            if *k == key {
                return t.clone();
            }
        }
    }
    let t = Arc::new(Thesaurus::load(user_file));
    if let Ok(mut c) = cache().lock() {
        *c = Some((key, t.clone()));
    }
    t
}

/// First word of a text (letters, apostrophes and hyphens).
pub fn first_word(text: &str) -> Option<String> {
    text.split(|c: char| !(c.is_alphabetic() || c == '\'' || c == '-'))
        .map(|w| w.trim_matches(|c| c == '\'' || c == '-'))
        .find(|w| !w.is_empty())
        .map(str::to_string)
}

/// Give `replacement` the capitalisation of `original` (initial capital
/// or all caps), so "Big" becomes "Large" and "BIG" becomes "LARGE".
pub fn match_case(original: &str, replacement: &str) -> String {
    let letters: Vec<char> = original.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() > 1 && letters.iter().all(|c| c.is_uppercase()) {
        return replacement.to_uppercase();
    }
    if letters.first().map(|c| c.is_uppercase()).unwrap_or(false) {
        let mut cs = replacement.chars();
        return match cs.next() {
            Some(f) => f.to_uppercase().chain(cs).collect(),
            None => String::new(),
        };
    }
    replacement.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "ISO8859-1\nabandon|2\n(verb)|forsake|desert (generic term)|desolate\n(noun)|wildness|unconstraint\nbright|1\n(adj)|shining|brilliant\nbroken|3\n(adj)|only one\n";

    #[test]
    fn parses_entries_and_strips_annotations() {
        let m = parse(SAMPLE);
        let a = &m["abandon"];
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].pos, "verb");
        assert_eq!(a[0].synonyms, vec!["forsake", "desert", "desolate"]);
        assert_eq!(a[1].synonyms, vec!["wildness", "unconstraint"]);
        assert_eq!(m["bright"][0].synonyms, vec!["shining", "brilliant"]);
        // An entry announcing more meanings than it has keeps what it has.
        assert_eq!(m["broken"].len(), 1);
    }

    #[test]
    fn built_in_list_works_without_files() {
        let t = Thesaurus::built_in();
        assert!(t.len() >= 40, "{}", t.len());
        assert_eq!(t.source, Source::BuiltIn);
        let m = t.lookup("Big");
        assert!(m.iter().any(|m| m.synonyms.contains(&"large".to_string())));
        assert!(t.lookup("qzxv").is_empty());
    }

    #[test]
    fn latin1_files_are_decoded() {
        let mut bytes = b"ISO8859-1\ncaf\xe9|1\n(noun)|bistro\n".to_vec();
        bytes.push(b'\n');
        let m = parse(&decode(&bytes));
        assert!(m.contains_key("café"));
        assert_eq!(decode(b"UTF-8\nx").lines().count(), 2);
    }

    #[test]
    fn first_word_and_case_matching() {
        assert_eq!(first_word("  Hello, world").as_deref(), Some("Hello"));
        assert_eq!(first_word("42 ..."), None);
        assert_eq!(match_case("Big", "large"), "Large");
        assert_eq!(match_case("BIG", "large"), "LARGE");
        assert_eq!(match_case("big", "large"), "large");
    }

    #[test]
    fn malformed_input_never_panics() {
        for s in [
            "",
            "UTF-8",
            "UTF-8\n|||\n(x)\n",
            "UTF-8\nword|notanumber\n(adj)|a",
        ] {
            let _ = parse(s);
        }
    }
}
