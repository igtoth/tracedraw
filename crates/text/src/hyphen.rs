//! Hyphenation: break points inside a word (character indices after which
//! a hyphen may be inserted). Uses a compact Liang-style pattern set for
//! English and Portuguese plus a vowel/consonant heuristic for everything
//! else. Minimum 2 letters before and 3 after a break, as the reference
//! editor defaults to.

use std::collections::HashMap;
use std::sync::OnceLock;

const MIN_BEFORE: usize = 2;
const MIN_AFTER: usize = 3;

/// A few common Liang patterns; digits mark odd (allowed) and even
/// (forbidden) break positions. Enough for ordinary words; the heuristic
/// handles the rest.
const PATTERNS_EN: &str = ".ab3a .ad4der .an3ti .ar5s .as3c .be5g .co3e .de3o .dis1 .ex1 .im1p .in1 .mis1 .non1 .out1 .over1 .pre1 .re1 .un1 .un3der 1ba 1be 1bi 1bo 1bu 1ca 1ce 1ci 1co 1cu 1da 1de 1di 1do 1du 1fa 1fe 1fi 1fo 1fu 1ga 1ge 1gi 1go 1gu 1ha 1he 1hi 1ho 1hu 1ja 1je 1jo 1ju 1ka 1ke 1ki 1ko 1la 1le 1li 1lo 1lu 1ma 1me 1mi 1mo 1mu 1na 1ne 1ni 1no 1nu 1pa 1pe 1pi 1po 1pu 1ra 1re 1ri 1ro 1ru 1sa 1se 1si 1so 1su 1ta 1te 1ti 1to 1tu 1va 1ve 1vi 1vo 1vu 1wa 1we 1wi 1wo 1za 1ze 1zo 2ch 2ck 2ll 2ss 2tt 2st 2sh 2th 2ph 2ng 4ed. 4er. 2es. 4ing. 2ly. 1ment 2ness 1tion 1sion 4ful. 2ble. 2ple. 2tle. 2gle. 2cle. 2dle. 2kle. 2zle. 4ous. 1ly 2ed2 3ment. 2ly. 1er 2ers. 1ing 2ngs. 1al 2ally";

const PATTERNS_PT: &str = "1ba 1be 1bi 1bo 1bu 1ca 1ce 1ci 1co 1cu 1da 1de 1di 1do 1du 1fa 1fe 1fi 1fo 1fu 1ga 1ge 1gi 1go 1gu 1ja 1je 1ji 1jo 1ju 1la 1le 1li 1lo 1lu 1ma 1me 1mi 1mo 1mu 1na 1ne 1ni 1no 1nu 1pa 1pe 1pi 1po 1pu 1ra 1re 1ri 1ro 1ru 1sa 1se 1si 1so 1su 1ta 1te 1ti 1to 1tu 1va 1ve 1vi 1vo 1vu 1xa 1xe 1xi 1xo 1xu 1za 1ze 1zi 1zo 1zu 1ça 1ço 1çu 2ch 2lh 2nh 2qu 2gu 2rr 2ss 1br 1bl 1cr 1cl 1dr 1fr 1fl 1gr 1gl 1pr 1pl 1tr 1vr 2bs 2ns 2rs 2ls 2ps 1ção 1mente";

type Patterns = HashMap<String, Vec<u8>>;

fn parse(src: &str) -> Patterns {
    let mut map = HashMap::new();
    for pat in src.split_whitespace() {
        let mut letters = String::new();
        let mut digits: Vec<u8> = Vec::new();
        let mut pending = 0u8;
        for c in pat.chars() {
            if let Some(d) = c.to_digit(10) {
                pending = d as u8;
            } else {
                digits.push(pending);
                pending = 0;
                letters.push(c);
            }
        }
        digits.push(pending);
        map.insert(letters, digits);
    }
    map
}

fn patterns(lang: &str) -> &'static Patterns {
    static EN: OnceLock<Patterns> = OnceLock::new();
    static PT: OnceLock<Patterns> = OnceLock::new();
    if lang.starts_with("pt") || lang.starts_with("es") {
        PT.get_or_init(|| parse(PATTERNS_PT))
    } else {
        EN.get_or_init(|| parse(PATTERNS_EN))
    }
}

/// Liang's algorithm over the pattern set, then the heuristic for words
/// the patterns leave unbroken.
pub fn break_points_lang(word: &str, lang: &str) -> Vec<usize> {
    let chars: Vec<char> = word.chars().collect();
    if chars.len() < MIN_BEFORE + MIN_AFTER || !chars.iter().all(|c| c.is_alphabetic()) {
        return Vec::new();
    }
    let lower: Vec<char> = chars.iter().flat_map(|c| c.to_lowercase()).collect();
    let dotted: Vec<char> = std::iter::once('.')
        .chain(lower.iter().copied())
        .chain(std::iter::once('.'))
        .collect();
    let n = dotted.len();
    let mut levels = vec![0u8; n + 1];
    let pats = patterns(lang);
    for start in 0..n {
        for end in start + 1..=n.min(start + 8) {
            let key: String = dotted[start..end].iter().collect();
            if let Some(d) = pats.get(&key) {
                for (k, v) in d.iter().enumerate() {
                    let idx = start + k;
                    if idx < levels.len() && *v > levels[idx] {
                        levels[idx] = *v;
                    }
                }
            }
        }
    }
    // levels[i] refers to the position before dotted[i]; word index = i - 1.
    let mut out: Vec<usize> = (1..chars.len())
        .filter(|&i| levels[i + 1 - 1 + 1] % 2 == 1 || levels[i] % 2 == 1)
        .filter(|&i| i >= MIN_BEFORE && chars.len() - i >= MIN_AFTER)
        .collect();
    if out.is_empty() {
        out = heuristic(&lower);
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Default language: English patterns.
pub fn break_points(word: &str) -> Vec<usize> {
    break_points_lang(word, "en")
}

fn is_vowel(c: char) -> bool {
    matches!(
        c,
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'y'
            | 'á'
            | 'é'
            | 'í'
            | 'ó'
            | 'ú'
            | 'â'
            | 'ê'
            | 'ô'
            | 'ã'
            | 'õ'
            | 'à'
    )
}

/// Break between a vowel and a following consonant+vowel group (V-CV).
fn heuristic(w: &[char]) -> Vec<usize> {
    let mut out = Vec::new();
    for i in 1..w.len().saturating_sub(1) {
        if is_vowel(w[i - 1])
            && !is_vowel(w[i])
            && i + 1 < w.len()
            && is_vowel(w[i + 1])
            && i >= MIN_BEFORE
            && w.len() - i >= MIN_AFTER
        {
            out.push(i);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_minimums() {
        assert!(break_points("abc").is_empty());
        for p in break_points("information") {
            assert!(p >= 2 && 11 - p >= 3);
        }
    }

    #[test]
    fn finds_a_break_in_common_words() {
        assert!(!break_points("hyphenation").is_empty());
        assert!(!break_points_lang("computador", "pt").is_empty());
    }

    #[test]
    fn skips_non_letters() {
        assert!(break_points("12345678").is_empty());
    }
}
