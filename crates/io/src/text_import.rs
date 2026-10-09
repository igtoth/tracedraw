//! Word-processor imports: plain text, RTF and DOCX become one paragraph
//! text frame with the runs' bold, italic, underline, size and font kept.
//!
//! RTF follows the public specification: control words with optional
//! numeric parameters, groups that save and restore the character state,
//! `\'hh` code-page bytes and `\uN` Unicode escapes with `\ucN` skips,
//! and destinations (`\fonttbl`, `\colortbl`, `\stylesheet`, `\info`,
//! `\pict` and any `{\*\...}`) that produce no text. DOCX is read from
//! `word/document.xml`: `w:p` paragraphs, `w:r` runs with `w:rPr`
//! properties, `w:t`, `w:tab` and `w:br`.

use tracedraw_core::geometry::{Point, Size};
use tracedraw_core::id::IdSource;
use tracedraw_core::{ParagraphStyle, Shape, ShapeKind, TextAlign, TextSpan};

/// Size of the frame the text is placed in and its top left corner.
#[derive(Debug, Clone, Copy)]
pub struct Placement {
    pub origin_top_left: Point,
    pub size: Size,
}

impl Placement {
    /// A frame inside an A4 page with 20 mm margins.
    pub fn a4() -> Self {
        Placement {
            origin_top_left: Point::new(20.0, 277.0),
            size: Size::new(170.0, 257.0),
        }
    }
}

/// Result of a text import: the paragraphs as styled spans (paragraphs
/// separated by `\n` inside the span list), the first paragraph's
/// alignment and warnings.
#[derive(Debug, Clone, Default)]
pub struct ImportedText {
    pub spans: Vec<TextSpan>,
    pub align: TextAlign,
    pub warnings: Vec<String>,
}

impl ImportedText {
    pub fn text(&self) -> String {
        tracedraw_core::spans_text(&self.spans)
    }

    /// One paragraph text frame holding the text.
    pub fn to_shape(self, place: Placement, ids: &mut IdSource) -> Shape {
        let spans = if self.spans.is_empty() {
            vec![TextSpan::new("", DEFAULT_FONT, DEFAULT_SIZE_PT)]
        } else {
            tracedraw_core::merge_equal_spans(self.spans)
        };
        let origin = Point::new(
            place.origin_top_left.x,
            place.origin_top_left.y - place.size.height,
        );
        Shape::new(
            ids.shape(),
            ShapeKind::Text {
                spans,
                origin,
                frame: Some(place.size),
                align: self.align,
                para: ParagraphStyle::default(),
                on_path: None,
            },
        )
    }
}

const DEFAULT_FONT: &str = "Arial";
const DEFAULT_SIZE_PT: f64 = 12.0;
/// Largest text accepted from a file, in bytes of UTF-8.
const MAX_TEXT: usize = 4 * 1024 * 1024;

/// Plain text: paragraphs on lines; CRLF and CR normalised.
pub fn parse_txt(bytes: &[u8]) -> ImportedText {
    let text = decode_text(bytes);
    let text: String = text.replace("\r\n", "\n").replace('\r', "\n");
    let text: String = text.chars().take(MAX_TEXT).collect();
    ImportedText {
        spans: vec![TextSpan::new(text, DEFAULT_FONT, DEFAULT_SIZE_PT)],
        align: TextAlign::Left,
        warnings: Vec::new(),
    }
}

/// UTF-8, or UTF-16 with a byte order mark, or Windows-1252.
fn decode_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        let be = bytes[0] == 0xFE;
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| {
                if be {
                    u16::from_be_bytes([c[0], c[1]])
                } else {
                    u16::from_le_bytes([c[0], c[1]])
                }
            })
            .collect();
        return String::from_utf16_lossy(&units);
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| cp1252(b)).collect(),
    }
}

/// Windows-1252 to Unicode (the 0x80..0x9F row differs from Latin-1).
fn cp1252(b: u8) -> char {
    const HIGH: [u32; 32] = [
        0x20AC, 0x81, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160,
        0x2039, 0x0152, 0x8D, 0x017D, 0x8F, 0x90, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013,
        0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x9D, 0x017E, 0x0178,
    ];
    if (0x80..0xA0).contains(&b) {
        char::from_u32(HIGH[(b - 0x80) as usize]).unwrap_or('\u{FFFD}')
    } else {
        b as char
    }
}

// ----- RTF -------------------------------------------------------------------

#[derive(Debug, Clone)]
struct RtfState {
    bold: bool,
    italic: bool,
    underline: bool,
    size_pt: f64,
    font: usize,
    /// Bytes to skip after a `\uN` escape (`\ucN`).
    uc: usize,
    /// Inside a destination that produces no text.
    skip: bool,
    /// Inside the font table: collecting a font name for `font`.
    in_fonttbl: bool,
}

impl Default for RtfState {
    fn default() -> Self {
        RtfState {
            bold: false,
            italic: false,
            underline: false,
            size_pt: DEFAULT_SIZE_PT,
            font: 0,
            uc: 1,
            skip: false,
            in_fonttbl: false,
        }
    }
}

/// Rich Text Format.
pub fn parse_rtf(bytes: &[u8]) -> Result<ImportedText, String> {
    if !bytes.starts_with(b"{\\rtf") {
        return Err("not an RTF file".into());
    }
    let mut out = ImportedText::default();
    let mut fonts: Vec<(usize, String)> = Vec::new();
    let mut font_name = String::new();
    let mut font_idx: Option<usize> = None;
    let mut stack: Vec<RtfState> = Vec::new();
    let mut st = RtfState::default();
    let mut align: Option<TextAlign> = None;
    let mut pending_skip = 0usize;
    let mut text_len = 0usize;
    let mut i = 0usize;
    let n = bytes.len();

    // Append a character in the current style.
    let push = |out: &mut ImportedText, st: &RtfState, fonts: &[(usize, String)], c: char| {
        let family = fonts
            .iter()
            .find(|(k, _)| *k == st.font)
            .map(|(_, f)| f.clone())
            .unwrap_or_else(|| DEFAULT_FONT.to_string());
        let style = TextSpan {
            bold: st.bold,
            italic: st.italic,
            underline: st.underline,
            ..TextSpan::new("", family, st.size_pt)
        };
        match out.spans.last_mut() {
            Some(last)
                if last.bold == style.bold
                    && last.italic == style.italic
                    && last.underline == style.underline
                    && last.font_family == style.font_family
                    && (last.size_pt - style.size_pt).abs() < 1e-9 =>
            {
                last.text.push(c)
            }
            _ => {
                let mut s = style;
                s.text.push(c);
                out.spans.push(s);
            }
        }
    };

    while i < n && text_len < MAX_TEXT {
        let b = bytes[i];
        match b {
            b'{' => {
                stack.push(st.clone());
                if stack.len() > 256 {
                    return Err("RTF groups nested too deep".into());
                }
                i += 1;
                // `{\*\dest ...}`: an unknown destination, ignored whole.
                if bytes[i..].starts_with(b"\\*") {
                    st.skip = true;
                }
            }
            b'}' => {
                if st.in_fonttbl && !st.skip {
                    if let Some(k) = font_idx.take() {
                        let name = font_name.trim().trim_end_matches(';').to_string();
                        if !name.is_empty() {
                            fonts.push((k, name));
                        }
                        font_name.clear();
                    }
                }
                st = stack.pop().unwrap_or_default();
                i += 1;
            }
            b'\\' => {
                i += 1;
                let Some(&c) = bytes.get(i) else { break };
                if c.is_ascii_alphabetic() {
                    // Control word with an optional signed parameter.
                    let start = i;
                    while i < n && bytes[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let word = String::from_utf8_lossy(&bytes[start..i]).into_owned();
                    let mut param: Option<i64> = None;
                    let pstart = i;
                    if i < n && (bytes[i] == b'-' || bytes[i].is_ascii_digit()) {
                        i += 1;
                        while i < n && bytes[i].is_ascii_digit() {
                            i += 1;
                        }
                        param = String::from_utf8_lossy(&bytes[pstart..i]).parse().ok();
                    }
                    // One space after a control word belongs to it.
                    if i < n && bytes[i] == b' ' {
                        i += 1;
                    }
                    if pending_skip > 0 {
                        pending_skip = 0;
                    }
                    match word.as_str() {
                        "fonttbl" => st.in_fonttbl = true,
                        "colortbl" | "stylesheet" | "info" | "pict" | "object" | "header"
                        | "footer" | "footnote" | "listtable" | "listoverridetable"
                        | "themedata" | "colorschememapping" | "latentstyles" | "datastore"
                        | "xmlnstbl" | "rsidtbl" | "generator" | "mmathPr" | "xmlopen"
                        | "fldinst" | "pntext" => st.skip = true,
                        "f" => {
                            let k = param.unwrap_or(0).max(0) as usize;
                            if st.in_fonttbl && !st.skip {
                                if let Some(prev) = font_idx.replace(k) {
                                    let name = font_name.trim().trim_end_matches(';').to_string();
                                    if !name.is_empty() {
                                        fonts.push((prev, name));
                                    }
                                    font_name.clear();
                                }
                            } else {
                                st.font = k;
                            }
                        }
                        "b" => st.bold = param.unwrap_or(1) != 0,
                        "i" => st.italic = param.unwrap_or(1) != 0,
                        "ul" => st.underline = param.unwrap_or(1) != 0,
                        "ulnone" => st.underline = false,
                        "fs" => st.size_pt = param.unwrap_or(24).max(2) as f64 / 2.0,
                        "plain" => {
                            let keep = (st.uc, st.skip, st.in_fonttbl);
                            st = RtfState::default();
                            st.uc = keep.0;
                            st.skip = keep.1;
                            st.in_fonttbl = keep.2;
                        }
                        "uc" => st.uc = param.unwrap_or(1).max(0) as usize,
                        "u" => {
                            if !st.skip && !st.in_fonttbl {
                                let code = param.unwrap_or(0);
                                let code = if code < 0 { code + 65536 } else { code };
                                if let Some(ch) = char::from_u32(code as u32) {
                                    push(&mut out, &st, &fonts, ch);
                                    text_len += 1;
                                }
                            }
                            pending_skip = st.uc;
                        }
                        "par" => {
                            if !st.skip && !st.in_fonttbl {
                                push(&mut out, &st, &fonts, '\n');
                                text_len += 1;
                            }
                        }
                        "line" => {
                            if !st.skip && !st.in_fonttbl {
                                push(&mut out, &st, &fonts, '\u{2028}');
                                text_len += 1;
                            }
                        }
                        "tab" => {
                            if !st.skip && !st.in_fonttbl {
                                push(&mut out, &st, &fonts, '\t');
                                text_len += 1;
                            }
                        }
                        "ql" => {
                            align.get_or_insert(TextAlign::Left);
                        }
                        "qc" => {
                            align.get_or_insert(TextAlign::Center);
                        }
                        "qr" => {
                            align.get_or_insert(TextAlign::Right);
                        }
                        "qj" => {
                            align.get_or_insert(TextAlign::Justify);
                        }
                        "lquote" | "rquote" | "ldblquote" | "rdblquote" | "emdash" | "endash"
                        | "bullet" | "emspace" | "enspace" | "~" => {
                            let ch = match word.as_str() {
                                "lquote" => '\u{2018}',
                                "rquote" => '\u{2019}',
                                "ldblquote" => '\u{201C}',
                                "rdblquote" => '\u{201D}',
                                "emdash" => '\u{2014}',
                                "endash" => '\u{2013}',
                                "bullet" => '\u{2022}',
                                "emspace" => '\u{2003}',
                                "enspace" => '\u{2002}',
                                _ => '\u{00A0}',
                            };
                            if !st.skip && !st.in_fonttbl {
                                push(&mut out, &st, &fonts, ch);
                                text_len += 1;
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Control symbol.
                    i += 1;
                    match c {
                        b'\'' => {
                            let hex = bytes.get(i..i + 2).unwrap_or(&[]);
                            i += hex.len();
                            if pending_skip > 0 {
                                pending_skip -= 1;
                            } else if let Ok(v) =
                                u8::from_str_radix(&String::from_utf8_lossy(hex), 16)
                            {
                                if st.in_fonttbl && !st.skip {
                                    font_name.push(cp1252(v));
                                } else if !st.skip {
                                    push(&mut out, &st, &fonts, cp1252(v));
                                    text_len += 1;
                                }
                            }
                        }
                        b'\\' | b'{' | b'}' => {
                            if !st.skip && !st.in_fonttbl {
                                push(&mut out, &st, &fonts, c as char);
                                text_len += 1;
                            }
                        }
                        b'~' => {
                            if !st.skip && !st.in_fonttbl {
                                push(&mut out, &st, &fonts, '\u{00A0}');
                                text_len += 1;
                            }
                        }
                        b'-' | b'_' | b':' => {}
                        b'*' => st.skip = true,
                        b'\r' | b'\n' => {}
                        _ => {}
                    }
                }
            }
            b'\r' | b'\n' => i += 1,
            _ => {
                i += 1;
                if pending_skip > 0 {
                    pending_skip -= 1;
                    continue;
                }
                if st.in_fonttbl && !st.skip {
                    if font_idx.is_some() {
                        font_name.push(b as char);
                    }
                } else if !st.skip {
                    // Bytes above 0x7F outside an escape: the file's code page.
                    push(&mut out, &st, &fonts, cp1252(b));
                    text_len += 1;
                }
            }
        }
    }
    if text_len >= MAX_TEXT {
        out.warnings.push("text truncated".into());
    }
    // A trailing paragraph mark ends the text rather than adding a line.
    if let Some(last) = out.spans.last_mut() {
        if last.text.ends_with('\n') {
            last.text.pop();
        }
    }
    out.spans.retain(|s| !s.text.is_empty());
    out.align = align.unwrap_or(TextAlign::Left);
    Ok(out)
}

// ----- DOCX ------------------------------------------------------------------

/// Office Open XML word processing document.
pub fn parse_docx(bytes: &[u8]) -> Result<ImportedText, String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor).map_err(|e| format!("not a DOCX file: {e}"))?;
    let mut xml = String::new();
    {
        let mut f = zip
            .by_name("word/document.xml")
            .map_err(|_| "word/document.xml missing".to_string())?;
        if f.size() > (MAX_TEXT as u64) * 8 {
            return Err("document too large".into());
        }
        std::io::Read::read_to_string(&mut f, &mut xml).map_err(|e| e.to_string())?;
    }
    parse_docx_xml(&xml)
}

/// The `word/document.xml` part on its own.
pub fn parse_docx_xml(xml: &str) -> Result<ImportedText, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("document.xml: {e}"))?;
    let mut out = ImportedText::default();
    let mut align: Option<TextAlign> = None;
    let body = doc
        .descendants()
        .find(|n| n.has_tag_name("body"))
        .ok_or("no w:body")?;
    let mut first_para = true;
    for p in body.descendants().filter(|n| n.has_tag_name("p")) {
        if !first_para {
            out.spans
                .push(TextSpan::new("\n", DEFAULT_FONT, DEFAULT_SIZE_PT));
        }
        first_para = false;
        if align.is_none() {
            if let Some(jc) = p
                .children()
                .find(|n| n.has_tag_name("pPr"))
                .and_then(|ppr| ppr.children().find(|n| n.has_tag_name("jc")))
            {
                align = match jc
                    .attribute(("", "val"))
                    .or_else(|| jc.attributes().next().map(|a| a.value()))
                {
                    Some("center") => Some(TextAlign::Center),
                    Some("right") | Some("end") => Some(TextAlign::Right),
                    Some("both") | Some("distribute") => Some(TextAlign::Justify),
                    _ => Some(TextAlign::Left),
                };
            }
        }
        for r in p.descendants().filter(|n| n.has_tag_name("r")) {
            let mut span = TextSpan::new("", DEFAULT_FONT, DEFAULT_SIZE_PT);
            if let Some(rpr) = r.children().find(|n| n.has_tag_name("rPr")) {
                for prop in rpr.children().filter(|n| n.is_element()) {
                    let val = prop
                        .attributes()
                        .find(|a| a.name() == "val")
                        .map(|a| a.value());
                    let on = !matches!(val, Some("0") | Some("false") | Some("none"));
                    match prop.tag_name().name() {
                        "b" => span.bold = on,
                        "i" => span.italic = on,
                        "u" => span.underline = on,
                        "strike" => span.strikethrough = on,
                        "sz" => {
                            if let Some(v) = val.and_then(|v| v.parse::<f64>().ok()) {
                                span.size_pt = (v / 2.0).max(1.0);
                            }
                        }
                        "rFonts" => {
                            if let Some(f) = prop
                                .attributes()
                                .find(|a| a.name() == "ascii" || a.name() == "hAnsi")
                            {
                                span.font_family = f.value().to_string();
                            }
                        }
                        _ => {}
                    }
                }
            }
            let mut text = String::new();
            for c in r.children() {
                match c.tag_name().name() {
                    "t" => text.push_str(c.text().unwrap_or("")),
                    "tab" => text.push('\t'),
                    "br" => text.push('\u{2028}'),
                    "cr" => text.push('\u{2028}'),
                    _ => {}
                }
            }
            if !text.is_empty() {
                span.text = text;
                out.spans.push(span);
            }
        }
        if tracedraw_core::spans_char_count(&out.spans) > MAX_TEXT {
            out.warnings.push("text truncated".into());
            break;
        }
    }
    out.align = align.unwrap_or(TextAlign::Left);
    out.spans = tracedraw_core::merge_equal_spans(out.spans);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_keeps_paragraphs() {
        let t = parse_txt(b"one\r\ntwo\rthree");
        assert_eq!(t.text(), "one\ntwo\nthree");
        let t = parse_txt(&[0xFF, 0xFE, b'h', 0, b'i', 0]);
        assert_eq!(t.text(), "hi");
        let t = parse_txt(&[b'c', b'a', b'f', 0xE9]);
        assert_eq!(t.text(), "café");
    }

    #[test]
    fn rtf_runs_keep_bold_italic_size_and_font() {
        let rtf = br#"{\rtf1\ansi\deff0{\fonttbl{\f0\fswiss Arial;}{\f1\froman Times New Roman;}}
{\colortbl;\red0\green0\blue0;}
\pard\qc\f1\fs28 Hello {\b bold} and {\i\fs20 small italic}\par
Second \'e9 line\u8364? end\par
}"#;
        let t = parse_rtf(rtf).unwrap();
        assert_eq!(
            t.text(),
            "Hello bold and small italic\nSecond \u{e9} line\u{20AC} end"
        );
        assert_eq!(t.align, TextAlign::Center);
        let bold = t.spans.iter().find(|s| s.bold).unwrap();
        assert_eq!(bold.text, "bold");
        assert_eq!(bold.font_family, "Times New Roman");
        assert_eq!(bold.size_pt, 14.0);
        let it = t.spans.iter().find(|s| s.italic).unwrap();
        assert_eq!(it.text, "small italic");
        assert_eq!(it.size_pt, 10.0);
        assert!(t.spans.iter().all(|s| !s.text.contains("Arial;")));
        assert!(parse_rtf(b"hello").is_err());
    }

    #[test]
    fn rtf_ignores_destinations_and_survives_garbage() {
        let rtf =
            b"{\\rtf1{\\*\\generator Riched20;}{\\info{\\title secret}}{\\pict 0011aabb}visible{";
        let t = parse_rtf(rtf).unwrap();
        assert_eq!(t.text(), "visible");
        // Truncated, nested and binary input never panics.
        let mut junk = b"{\\rtf1".to_vec();
        for k in 0..2000u32 {
            junk.extend_from_slice(if k % 3 == 0 { b"{\\u" } else { b"\\'z" });
            junk.push((k % 251) as u8);
        }
        let _ = parse_rtf(&junk);
    }

    #[test]
    fn docx_paragraphs_and_runs() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
<w:p><w:pPr><w:jc w:val="right"/></w:pPr><w:r><w:t>Title</w:t></w:r></w:p>
<w:p><w:r><w:rPr><w:b/><w:sz w:val="32"/><w:rFonts w:ascii="Georgia"/></w:rPr><w:t xml:space="preserve">Bold </w:t></w:r><w:r><w:t>plain</w:t><w:tab/><w:t>after tab</w:t></w:r></w:p>
</w:body></w:document>"#;
        let t = parse_docx_xml(xml).unwrap();
        assert_eq!(t.text(), "Title\nBold plain\tafter tab");
        assert_eq!(t.align, TextAlign::Right);
        let b = t.spans.iter().find(|s| s.bold).unwrap();
        assert_eq!(b.text, "Bold ");
        assert_eq!(b.size_pt, 16.0);
        assert_eq!(b.font_family, "Georgia");
        assert!(parse_docx(b"PK garbage").is_err());
        assert!(parse_docx_xml("<not xml").is_err());
    }

    #[test]
    fn docx_zip_round_trip_to_shape() {
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Zipped</w:t></w:r></w:p></w:body></w:document>"#;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            w.start_file("word/document.xml", opts).unwrap();
            std::io::Write::write_all(&mut w, xml.as_bytes()).unwrap();
            w.finish().unwrap();
        }
        let t = parse_docx(buf.get_ref()).unwrap();
        assert_eq!(t.text(), "Zipped");
        let mut ids = IdSource::default();
        let shape = t.to_shape(Placement::a4(), &mut ids);
        match shape.kind {
            ShapeKind::Text { frame, origin, .. } => {
                assert_eq!(frame, Some(Size::new(170.0, 257.0)));
                assert!((origin.y - 20.0).abs() < 1e-9);
            }
            _ => panic!("expected text"),
        }
    }
}
