//! Tamil text as an input method types it (#1151): letters with vowel
//! signs written before the consonant (e, ee, ai), split vowel signs
//! around it (o, oo, au), the pulli, and the ksha and shri ligatures come
//! out as the font shapes the whole word, whether the word is typed in a
//! Tamil font, drawn by a fallback face, split between spans (a letter
//! typed in another colour) or typed one letter at a time.

use super::*;

/// Noto Sans Tamil (SIL Open Font License, see
/// `testdata/NotoSansTamil-OFL.txt`): full Tamil GSUB and GPOS.
const NOTO_TAML: &[u8] = include_bytes!("../testdata/NotoSansTamil-VF.ttf");
/// A Latin-only face, for the fallback cases.
const QUICKSAND: &[u8] = include_bytes!("../testdata/Quicksand-VF.ttf");
const TAML: &str = "Noto Sans Tamil";

const CORPUS_TEXT: &str = include_str!("../testdata/tamil-corpus.txt");

fn corpus() -> Vec<&'static str> {
    CORPUS_TEXT
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect()
}

fn system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_data(NOTO_TAML.to_vec());
    db.load_font_data(QUICKSAND.to_vec());
    FontSystem::from_db(db)
}

/// The glyphs the font makes for `text` shaped as one run.
fn whole(text: &str) -> Vec<u16> {
    let face = rustybuzz::Face::from_slice(NOTO_TAML, 0).expect("font");
    let mut b = rustybuzz::UnicodeBuffer::new();
    b.push_str(text);
    b.guess_segment_properties();
    let out = rustybuzz::shape(&face, &[], b);
    out.glyph_infos()
        .iter()
        .map(|i| i.glyph_id as u16)
        .collect()
}

fn taml_gids(f: &FontSystem, spans: &[TextSpan]) -> Vec<u16> {
    let para = ParagraphStyle::default();
    f.glyph_layout(&TextRequest {
        spans,
        frame: None,
        align: TextAlign::Left,
        para: &para,
        on_path: None,
    })
    .glyphs
    .iter()
    .filter(|g| g.font.data.len() == NOTO_TAML.len())
    .map(|g| g.glyph_id)
    .collect()
}

fn failures(mut laid: impl FnMut(&str) -> Vec<u16>) -> Vec<String> {
    corpus()
        .into_iter()
        .filter(|w| laid(w) != whole(w))
        .map(|w| w.to_string())
        .collect()
}

#[test]
fn the_font_reorders_vowel_signs_and_makes_the_ligatures() {
    let face = ttf_parser::Face::parse(NOTO_TAML, 0).expect("font");
    let gid = |c: char| face.glyph_index(c).map(|g| g.0).unwrap_or(u16::MAX);
    // கொ: the e sign comes first, the aa sign after the consonant.
    assert_eq!(whole("கொ"), [gid('\u{0BC6}'), gid('க'), gid('\u{0BBE}')]);
    // கை: the ai sign before the consonant.
    assert_eq!(whole("கை"), [gid('\u{0BC8}'), gid('க')]);
    // க்ஷ and ஸ்ரீ are one glyph each, no pulli or dotted circle.
    for w in ["க்ஷ", "ஸ்ரீ"] {
        let g = whole(w);
        assert_eq!(g.len(), 1, "{w}: {g:?}");
        assert!(!g.contains(&gid('\u{25CC}')), "{w}");
    }
}

#[test]
fn a_word_in_a_tamil_font_is_shaped_whole() {
    let f = system();
    let bad = failures(|w| taml_gids(&f, &[TextSpan::new(w, TAML, 24.0)]));
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn a_tamil_word_drawn_by_the_fallback_face_is_shaped_whole() {
    let f = system();
    let bad = failures(|w| taml_gids(&f, &[TextSpan::new(format!("Ab {w}"), "Quicksand", 24.0)]));
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn a_tamil_word_split_between_spans_is_shaped_whole() {
    let f = system();
    let mut bad = Vec::new();
    for w in corpus() {
        let cuts: Vec<usize> = w.char_indices().map(|(i, _)| i).skip(1).collect();
        for cut in cuts {
            let mut a = TextSpan::new(&w[..cut], TAML, 24.0);
            a.fill = Some(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
                10, 0, 0,
            )));
            let mut b = TextSpan::new(&w[cut..], TAML, 24.0);
            b.fill = Some(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
                200, 0, 0,
            )));
            if taml_gids(&f, &[a, b]) != whole(w) {
                bad.push(format!("{}|{}", &w[..cut], &w[cut..]));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}

/// The ksha and shri ligatures join over the pulli: a consonant after
/// it continues the syllable only through the joiner the font needs, so
/// a narrow frame never breaks a line between க் and ஷ.
#[test]
fn a_tamil_word_broken_across_lines_keeps_its_ligatures() {
    let f = system();
    let para = ParagraphStyle::default();
    for w in ["க்ஷேத்திரம்", "ஸ்ரீரங்கம்"] {
        let spans = [TextSpan::new(w, TAML, 24.0)];
        let req = |frame| TextRequest {
            spans: &spans,
            frame,
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        };
        let wide = f.layout(&req(None)).bounds.width();
        for frac in [0.2, 0.3, 0.5, 0.7] {
            let l = f.layout(&req(Some(Size::new(wide * frac, 500.0))));
            for line in l.lines.iter().skip(1) {
                assert!(line.start_char != 2, "{w} broken in its ligature at {frac}");
            }
        }
    }
}
