//! Autocorrect (Text > Writing Tools > Autocorrect): corrections made
//! while typing, once the word is finished with a space or punctuation.
//! Rules: capitalise the first letter of a sentence, fix two initial
//! capitals ("THis"), typographic quotes by language, and the user's
//! replacement table ("(c)" to "©"). Options persist in settings.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutocorrectPrefs {
    /// Apply the rules while typing.
    pub enabled: bool,
    pub capitalize_sentences: bool,
    pub fix_two_initial_capitals: bool,
    pub typographic_quotes: bool,
    /// (abbreviation, replacement) pairs, matched as whole words.
    pub replacements: Vec<(String, String)>,
}

impl Default for AutocorrectPrefs {
    fn default() -> Self {
        AutocorrectPrefs {
            enabled: true,
            capitalize_sentences: true,
            fix_two_initial_capitals: true,
            typographic_quotes: true,
            replacements: vec![
                ("(c)".into(), "©".into()),
                ("(r)".into(), "®".into()),
                ("(tm)".into(), "™".into()),
                ("1/2".into(), "½".into()),
                ("1/4".into(), "¼".into()),
                ("3/4".into(), "¾".into()),
            ],
        }
    }
}

/// Quotation mark style; chosen from the UI language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteStyle {
    /// “ ” and ‘ ’ (English, Portuguese, Spanish and most others).
    Curly,
    /// „ “ and ‚ ‘ (German).
    German,
    /// « » and ‹ › (French, Russian).
    Guillemets,
}

impl QuoteStyle {
    pub fn for_language(lang: &str) -> Self {
        let base = lang.split('-').next().unwrap_or(lang).to_lowercase();
        match base.as_str() {
            "de" => QuoteStyle::German,
            "fr" | "ru" => QuoteStyle::Guillemets,
            _ => QuoteStyle::Curly,
        }
    }

    /// (open, close) for double quotes.
    pub fn double(self) -> (char, char) {
        match self {
            QuoteStyle::Curly => ('“', '”'),
            QuoteStyle::German => ('„', '“'),
            QuoteStyle::Guillemets => ('«', '»'),
        }
    }

    /// (open, close) for single quotes; the closing one doubles as the apostrophe.
    pub fn single(self) -> (char, char) {
        match self {
            QuoteStyle::Curly => ('‘', '’'),
            QuoteStyle::German => ('‚', '‘'),
            QuoteStyle::Guillemets => ('‹', '›'),
        }
    }
}

/// Characters that end a word and trigger the corrections while typing.
fn is_trigger(c: char) -> bool {
    c.is_whitespace() || matches!(c, '.' | ',' | ';' | ':' | '!' | '?')
}

/// A quote at `pos` opens when nothing, whitespace or an opening bracket
/// or dash precedes it.
fn opens_quote(chars: &[char], pos: usize) -> bool {
    match pos.checked_sub(1).and_then(|i| chars.get(i)) {
        None => true,
        Some(p) => {
            p.is_whitespace() || matches!(p, '(' | '[' | '{' | '-' | '\u{2013}' | '\u{2014}' | '/')
        }
    }
}

/// Straight quotes to typographic ones over the whole text.
pub fn typographic_quotes(text: &str, style: QuoteStyle) -> String {
    let chars: Vec<char> = text.chars().collect();
    let (d_open, d_close) = style.double();
    let (s_open, s_close) = style.single();
    chars
        .iter()
        .enumerate()
        .map(|(i, c)| match c {
            '"' => {
                if opens_quote(&chars, i) {
                    d_open
                } else {
                    d_close
                }
            }
            '\'' => {
                if opens_quote(&chars, i) {
                    s_open
                } else {
                    s_close
                }
            }
            o => *o,
        })
        .collect()
}

/// "THis" to "This": a word of three or more letters whose first two are
/// capitals and the rest lowercase.
pub fn fix_two_initial_capitals_word(word: &str) -> String {
    let chars: Vec<char> = word.chars().collect();
    let letters: Vec<char> = chars
        .iter()
        .copied()
        .filter(|c| c.is_alphabetic())
        .collect();
    if letters.len() < 3
        || !letters[0].is_uppercase()
        || !letters[1].is_uppercase()
        || !letters[2..].iter().all(|c| c.is_lowercase())
    {
        return word.to_string();
    }
    let mut seen = 0;
    chars
        .iter()
        .map(|c| {
            if c.is_alphabetic() {
                seen += 1;
                if seen == 2 {
                    return c.to_lowercase().collect::<String>();
                }
            }
            c.to_string()
        })
        .collect()
}

fn fix_two_initial_capitals(text: &str) -> String {
    map_words(text, fix_two_initial_capitals_word)
}

/// Apply `f` to every run of non-whitespace characters.
fn map_words(text: &str, mut f: impl FnMut(&str) -> String) -> String {
    let mut out = String::new();
    let mut word = String::new();
    for c in text.chars() {
        if c.is_whitespace() {
            if !word.is_empty() {
                out.push_str(&f(&word));
                word.clear();
            }
            out.push(c);
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        out.push_str(&f(&word));
    }
    out
}

/// Capitalise the first letter of every sentence.
pub fn capitalize_sentences(text: &str) -> String {
    let mut out = String::new();
    let mut at_start = true;
    let mut after_terminator = false;
    for c in text.chars() {
        if at_start && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            at_start = false;
            after_terminator = false;
            continue;
        }
        if c == '\n' {
            at_start = true;
            after_terminator = false;
        } else if matches!(c, '.' | '!' | '?') {
            after_terminator = true;
        } else if c.is_whitespace() {
            if after_terminator {
                at_start = true;
            }
        } else if c.is_alphanumeric() {
            at_start = false;
            after_terminator = false;
        } else if !matches!(
            c,
            '"' | '\'' | '“' | '„' | '«' | '‘' | '‚' | '‹' | '(' | '['
        ) {
            // Closing quotes and other punctuation keep the pending state;
            // opening quotes and brackets may precede the first letter.
            after_terminator = after_terminator && matches!(c, ')' | ']' | '”' | '’' | '»' | '“');
        }
        out.push(c);
    }
    out
}

/// Replace whole words found in the table.
pub fn apply_replacements(text: &str, table: &[(String, String)]) -> String {
    map_words(text, |w| replace_word(w, table))
}

fn replace_word(word: &str, table: &[(String, String)]) -> String {
    // Exact match first, then a match that leaves trailing punctuation alone.
    if let Some((_, to)) = table.iter().find(|(from, _)| from == word) {
        return to.clone();
    }
    let trimmed = word.trim_end_matches(|c: char| matches!(c, '.' | ',' | ';' | ':' | '!' | '?'));
    if trimmed.len() != word.len() {
        if let Some((_, to)) = table.iter().find(|(from, _)| from == trimmed) {
            return format!("{to}{}", &word[trimmed.len()..]);
        }
    }
    word.to_string()
}

/// All enabled rules over a whole text (Apply to selection).
pub fn apply_all(text: &str, prefs: &AutocorrectPrefs, style: QuoteStyle) -> String {
    let mut t = text.to_string();
    if !prefs.replacements.is_empty() {
        t = apply_replacements(&t, &prefs.replacements);
    }
    if prefs.fix_two_initial_capitals {
        t = fix_two_initial_capitals(&t);
    }
    if prefs.capitalize_sentences {
        t = capitalize_sentences(&t);
    }
    if prefs.typographic_quotes {
        t = typographic_quotes(&t, style);
    }
    t
}

/// Correction after the last typed character. Only the word just finished
/// (or the quote just typed) is touched, so earlier text the user edited
/// on purpose stays as it is. Returns the new text when anything changed.
pub fn on_typed(text: &str, prefs: &AutocorrectPrefs, style: QuoteStyle) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let last = *chars.last()?;
    let n = chars.len();
    if last == '"' || last == '\'' {
        if !prefs.typographic_quotes {
            return None;
        }
        let opening = opens_quote(&chars, n - 1);
        let (o, c) = if last == '"' {
            style.double()
        } else {
            style.single()
        };
        let mut out: String = chars[..n - 1].iter().collect();
        out.push(if opening { o } else { c });
        return Some(out);
    }
    if !is_trigger(last) {
        return None;
    }
    // The word before the trigger.
    let mut end = n - 1;
    while end > 0 && is_trigger(chars[end - 1]) {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && !chars[start - 1].is_whitespace() {
        start -= 1;
    }
    if start == end {
        return None;
    }
    let word: String = chars[start..end].iter().collect();
    let mut fixed = word.clone();
    if !prefs.replacements.is_empty() {
        fixed = replace_word(&fixed, &prefs.replacements);
    }
    if prefs.fix_two_initial_capitals {
        fixed = fix_two_initial_capitals_word(&fixed);
    }
    if prefs.capitalize_sentences {
        let before: String = chars[..start].iter().collect();
        let trailing_newline = chars[..start]
            .iter()
            .rev()
            .take_while(|c| c.is_whitespace())
            .any(|c| *c == '\n');
        let trimmed = before.trim_end();
        let at_sentence_start = trailing_newline
            || match trimmed.chars().last() {
                None => true,
                Some(p) => {
                    matches!(p, '.' | '!' | '?') && !(p == '.' && is_abbreviation_before(trimmed))
                }
            };
        if at_sentence_start {
            let mut cs = fixed.chars();
            if let Some(f) = cs.next() {
                if f.is_lowercase() {
                    let rest: String = cs.collect();
                    fixed = f.to_uppercase().chain(rest.chars()).collect();
                }
            }
        }
    }
    if fixed == word {
        return None;
    }
    let mut out: String = chars[..start].iter().collect();
    out.push_str(&fixed);
    out.extend(chars[end..].iter());
    Some(out)
}

/// Apply every enabled rule to the selected text objects (span by span,
/// so styling is kept). Returns the number of objects changed.
pub fn apply_to_selection(app: &mut crate::app::App) -> usize {
    use tracedraw_core::{document::ShapeKind, Command, TextSpan};
    let prefs = app.settings.autocorrect.clone();
    let style = QuoteStyle::for_language(&crate::i18n::language());
    let mut changed = 0;
    for s in app.text_shapes() {
        let ShapeKind::Text {
            spans,
            origin,
            frame,
            align,
            para,
            on_path,
        } = s.kind
        else {
            continue;
        };
        let new_spans: Vec<TextSpan> = spans
            .iter()
            .map(|sp| TextSpan {
                text: apply_all(&sp.text, &prefs, style),
                ..sp.clone()
            })
            .collect();
        if new_spans == spans {
            continue;
        }
        app.run(Command::SetShapeKind {
            shape: s.id,
            kind: ShapeKind::Text {
                spans: new_spans,
                origin,
                frame,
                align,
                para,
                on_path,
            },
        });
        changed += 1;
    }
    changed
}

/// The text (already trimmed) ends with an abbreviation point such as
/// "e.g." or "U.S.", a list number "3." or an ellipsis, none of which
/// starts a new sentence.
fn is_abbreviation_before(trimmed: &str) -> bool {
    let Some(stripped) = trimmed.strip_suffix('.') else {
        return false;
    };
    let run = stripped
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric())
        .count();
    run == 1 || stripped.ends_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalises_sentences() {
        assert_eq!(
            capitalize_sentences("hello. how are you? fine!"),
            "Hello. How are you? Fine!"
        );
        assert_eq!(capitalize_sentences("one\ntwo"), "One\nTwo");
        assert_eq!(
            capitalize_sentences("he said. \"yes.\""),
            "He said. \"Yes.\""
        );
        assert_eq!(capitalize_sentences("3.14 is pi"), "3.14 is pi");
    }

    #[test]
    fn two_initial_capitals() {
        assert_eq!(fix_two_initial_capitals_word("THis"), "This");
        assert_eq!(fix_two_initial_capitals_word("THIS"), "THIS");
        assert_eq!(fix_two_initial_capitals_word("McDonald"), "McDonald");
        assert_eq!(fix_two_initial_capitals_word("AB"), "AB");
        assert_eq!(fix_two_initial_capitals_word("THis,"), "This,");
        assert_eq!(fix_two_initial_capitals("THe BIg dog"), "The Big dog");
    }

    #[test]
    fn quotes_follow_the_language() {
        assert_eq!(
            typographic_quotes("\"hi\" it's", QuoteStyle::Curly),
            "“hi” it’s"
        );
        assert_eq!(typographic_quotes("\"hi\"", QuoteStyle::German), "„hi“");
        assert_eq!(typographic_quotes("\"hi\"", QuoteStyle::Guillemets), "«hi»");
        assert_eq!(typographic_quotes("('a')", QuoteStyle::Curly), "(‘a’)");
        assert_eq!(QuoteStyle::for_language("pt-BR"), QuoteStyle::Curly);
        assert_eq!(QuoteStyle::for_language("de"), QuoteStyle::German);
        assert_eq!(QuoteStyle::for_language("fr"), QuoteStyle::Guillemets);
        assert_eq!(QuoteStyle::for_language("es"), QuoteStyle::Curly);
    }

    #[test]
    fn replacement_table() {
        let p = AutocorrectPrefs::default();
        assert_eq!(
            apply_replacements("Acme (c) 2026, (tm).", &p.replacements),
            "Acme © 2026, ™."
        );
        assert_eq!(apply_replacements("copy(c)", &p.replacements), "copy(c)");
    }

    #[test]
    fn apply_all_runs_every_rule() {
        let p = AutocorrectPrefs::default();
        assert_eq!(
            apply_all("THe \"big\" dog. it ran (c)", &p, QuoteStyle::Curly),
            "The “big” dog. It ran ©"
        );
        let off = AutocorrectPrefs {
            capitalize_sentences: false,
            fix_two_initial_capitals: false,
            typographic_quotes: false,
            replacements: Vec::new(),
            ..p
        };
        assert_eq!(
            apply_all("THe \"big\" dog. it", &off, QuoteStyle::Curly),
            "THe \"big\" dog. it"
        );
    }

    #[test]
    fn typing_corrects_the_finished_word_only() {
        let p = AutocorrectPrefs::default();
        let s = QuoteStyle::Curly;
        // Nothing until the word is finished.
        assert_eq!(on_typed("hello", &p, s), None);
        assert_eq!(on_typed("hello ", &p, s).as_deref(), Some("Hello "));
        assert_eq!(
            on_typed("Hello. world.", &p, s).as_deref(),
            Some("Hello. World.")
        );
        assert_eq!(on_typed("Hello world.", &p, s), None);
        assert_eq!(on_typed("Say THis ", &p, s).as_deref(), Some("Say This "));
        assert_eq!(on_typed("Acme (c) ", &p, s).as_deref(), Some("Acme © "));
        assert_eq!(on_typed("Acme (c), ", &p, s).as_deref(), Some("Acme ©, "));
        assert_eq!(on_typed("e.g. this ", &p, s), None);
        assert_eq!(
            on_typed("Line\nnext ", &p, s).as_deref(),
            Some("Line\nNext ")
        );
        // Quotes are converted as they are typed.
        assert_eq!(on_typed("He said \"", &p, s).as_deref(), Some("He said “"));
        assert_eq!(
            on_typed("He said “hi\"", &p, s).as_deref(),
            Some("He said “hi”")
        );
        assert_eq!(on_typed("it'", &p, s).as_deref(), Some("it’"));
        assert_eq!(on_typed("\"", &p, QuoteStyle::German).as_deref(), Some("„"));
        // Earlier text the user lower-cased on purpose is left alone.
        assert_eq!(on_typed("hello world. this is ", &p, s), None);
        let off = AutocorrectPrefs {
            typographic_quotes: false,
            ..AutocorrectPrefs::default()
        };
        assert_eq!(on_typed("x \"", &off, s), None);
    }

    #[test]
    fn prefs_round_trip_and_defaults() {
        let p = AutocorrectPrefs::default();
        let j = serde_json::to_string(&p).unwrap();
        let back: AutocorrectPrefs = serde_json::from_str(&j).unwrap();
        assert_eq!(back, p);
        let partial: AutocorrectPrefs = serde_json::from_str("{\"enabled\":false}").unwrap();
        assert!(!partial.enabled && partial.capitalize_sentences);
    }

    #[test]
    fn malformed_input_never_panics() {
        let p = AutocorrectPrefs::default();
        for s in ["", " ", "\"", "'", ".", "\n", "  . ", "(c)"] {
            let _ = on_typed(s, &p, QuoteStyle::Curly);
            let _ = apply_all(s, &p, QuoteStyle::German);
        }
    }
}
