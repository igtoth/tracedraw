//! Formatting kept per paragraph: alignment, spacing, indents, bullets,
//! drop caps, tabs and hyphenation.
//!
//! A text object keeps one alignment and one [`ParagraphStyle`] of its own
//! (what every paragraph has unless told otherwise, and the only place for
//! what belongs to the frame: columns, vertical alignment, fitting, the
//! baseline grid and the frame fill). A paragraph with formatting of its
//! own carries it on its characters ([`TextSpan::paragraph`]): the format
//! of a paragraph's first character rules it. An empty last paragraph
//! (after a final line break) follows the one before it. Files without the
//! field give every paragraph the object's own formatting, as before.

use crate::document::{
    merge_equal_spans, spans_apply, LeadingUnit, ParagraphStyle, ShapeKind, TextAlign,
    TextDirection, TextSpan,
};
use serde::{Deserialize, Serialize};

/// The formatting of one paragraph: the alignment and the paragraph-level
/// fields of a [`ParagraphStyle`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParagraphFormat {
    pub align: TextAlign,
    pub leading_pct: f64,
    pub leading_unit: LeadingUnit,
    pub char_spacing_pct: f64,
    pub word_spacing_pct: f64,
    pub space_before: f64,
    pub space_after: f64,
    pub first_line_indent: f64,
    pub left_indent: f64,
    pub right_indent: f64,
    pub bullets: bool,
    pub bullet_char: String,
    pub bullet_indent: f64,
    pub drop_cap_lines: u32,
    pub tabs: Vec<f64>,
    pub hyphenate: bool,
    #[serde(skip_serializing_if = "TextDirection::is_auto")]
    pub direction: TextDirection,
}

impl Default for ParagraphFormat {
    fn default() -> Self {
        ParagraphFormat::of(TextAlign::Left, &ParagraphStyle::default())
    }
}

impl ParagraphFormat {
    /// The paragraph part of an object's alignment and style.
    pub fn of(align: TextAlign, p: &ParagraphStyle) -> Self {
        ParagraphFormat {
            align,
            leading_pct: p.leading_pct,
            leading_unit: p.leading_unit,
            char_spacing_pct: p.char_spacing_pct,
            word_spacing_pct: p.word_spacing_pct,
            space_before: p.space_before,
            space_after: p.space_after,
            first_line_indent: p.first_line_indent,
            left_indent: p.left_indent,
            right_indent: p.right_indent,
            bullets: p.bullets,
            bullet_char: p.bullet_char.clone(),
            bullet_indent: p.bullet_indent,
            drop_cap_lines: p.drop_cap_lines,
            tabs: p.tabs.clone(),
            hyphenate: p.hyphenate,
            direction: p.direction,
        }
    }

    /// Write the paragraph fields into `p` (its frame fields stay) and
    /// return the alignment.
    pub fn apply_to(&self, p: &mut ParagraphStyle) -> TextAlign {
        p.leading_pct = self.leading_pct;
        p.leading_unit = self.leading_unit;
        p.char_spacing_pct = self.char_spacing_pct;
        p.word_spacing_pct = self.word_spacing_pct;
        p.space_before = self.space_before;
        p.space_after = self.space_after;
        p.first_line_indent = self.first_line_indent;
        p.left_indent = self.left_indent;
        p.right_indent = self.right_indent;
        p.bullets = self.bullets;
        p.bullet_char = self.bullet_char.clone();
        p.bullet_indent = self.bullet_indent;
        p.drop_cap_lines = self.drop_cap_lines;
        p.tabs = self.tabs.clone();
        p.hyphenate = self.hyphenate;
        p.direction = self.direction;
        self.align
    }

    /// Take the fields that differ between `old` and `new` from `new`,
    /// keeping the others: an edit made on one paragraph's values applied
    /// to another paragraph.
    pub fn take_changes(&mut self, old: &ParagraphFormat, new: &ParagraphFormat) {
        macro_rules! take {
            ($($f:ident),*) => {
                $(if old.$f != new.$f {
                    self.$f = new.$f.clone();
                })*
            };
        }
        take!(
            align,
            leading_pct,
            leading_unit,
            char_spacing_pct,
            word_spacing_pct,
            space_before,
            space_after,
            first_line_indent,
            left_indent,
            right_indent,
            bullets,
            bullet_char,
            bullet_indent,
            drop_cap_lines,
            tabs,
            hyphenate,
            direction
        );
    }
}

/// The paragraphs of a span list: (first character, end) in the index
/// space of the concatenated text; `end` is the index of the newline that
/// ends the paragraph (or the text length for the last one).
pub fn paragraph_ranges(spans: &[TextSpan]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    for sp in spans {
        for c in sp.text.chars() {
            if c == '\n' {
                out.push((start, i));
                start = i + 1;
            }
            i += 1;
        }
    }
    out.push((start, i));
    out
}

/// The index of the paragraph holding the character index `at` (a caret
/// position; at a newline, the paragraph it ends).
pub fn paragraph_of(spans: &[TextSpan], at: usize) -> usize {
    let ranges = paragraph_ranges(spans);
    ranges
        .iter()
        .position(|(s, e)| at >= *s && at <= *e)
        .unwrap_or(ranges.len().saturating_sub(1))
}

/// The span holding the character index `at`.
fn span_at(spans: &[TextSpan], at: usize) -> Option<&TextSpan> {
    let mut seen = 0usize;
    for sp in spans {
        let n = sp.char_count();
        if at < seen + n {
            return Some(sp);
        }
        seen += n;
    }
    None
}

/// The format a paragraph starting at `start` carries, if any.
fn carried(spans: &[TextSpan], start: usize) -> Option<&ParagraphFormat> {
    span_at(spans, start)
        .or_else(|| start.checked_sub(1).and_then(|p| span_at(spans, p)))
        .and_then(|sp| sp.paragraph.as_deref())
}

/// Every paragraph's format: its own, or the object's.
pub fn paragraph_formats_of(
    spans: &[TextSpan],
    align: TextAlign,
    para: &ParagraphStyle,
) -> Vec<ParagraphFormat> {
    let base = ParagraphFormat::of(align, para);
    paragraph_ranges(spans)
        .iter()
        .map(|(s, _)| carried(spans, *s).cloned().unwrap_or_else(|| base.clone()))
        .collect()
}

/// Every paragraph's alignment and full style (the object's frame fields
/// with the paragraph's own formatting), in order.
pub fn paragraph_styles(
    spans: &[TextSpan],
    align: TextAlign,
    para: &ParagraphStyle,
) -> Vec<(TextAlign, ParagraphStyle)> {
    // Most texts carry no formatting of their own: one style for all.
    if spans.iter().all(|s| s.paragraph.is_none()) {
        let n = paragraph_ranges(spans).len();
        return vec![(align, para.clone()); n];
    }
    paragraph_formats_of(spans, align, para)
        .into_iter()
        .map(|f| {
            let mut p = para.clone();
            let a = f.apply_to(&mut p);
            (a, p)
        })
        .collect()
}

/// The format of paragraph `index` (the object's for one past the end).
pub fn paragraph_format(
    spans: &[TextSpan],
    align: TextAlign,
    para: &ParagraphStyle,
    index: usize,
) -> ParagraphFormat {
    let ranges = paragraph_ranges(spans);
    ranges
        .get(index)
        .and_then(|(s, _)| carried(spans, *s).cloned())
        .unwrap_or_else(|| ParagraphFormat::of(align, para))
}

/// The paragraphs a selection `from..to` touches (the one holding the
/// caret when `from == to`).
pub fn paragraphs_in(spans: &[TextSpan], from: usize, to: usize) -> Vec<usize> {
    let (a, b) = (from.min(to), from.max(to));
    paragraph_ranges(spans)
        .iter()
        .enumerate()
        .filter(|(_, (s, e))| {
            if a == b {
                a >= *s && a <= *e
            } else {
                a <= *e && b > *s
            }
        })
        .map(|(i, _)| i)
        .collect()
}

/// Change the formatting of a text's paragraphs with `f`: those the
/// character range `range` touches, or with `None` every paragraph and
/// the object's own formatting. A paragraph left with the object's
/// formatting stops carrying its own. Other shapes are left alone.
pub fn edit_paragraph_formats(
    kind: &mut ShapeKind,
    range: Option<(usize, usize)>,
    f: impl Fn(&mut ParagraphFormat),
) {
    let ShapeKind::Text {
        spans, align, para, ..
    } = kind
    else {
        return;
    };
    match range {
        None => {
            let mut own = ParagraphFormat::of(*align, para);
            f(&mut own);
            *align = own.apply_to(para);
            for sp in spans.iter_mut() {
                if let Some(p) = sp.paragraph.as_mut() {
                    f(p);
                }
                if sp.paragraph.as_deref() == Some(&own) {
                    sp.paragraph = None;
                }
            }
            *spans = merge_equal_spans(std::mem::take(spans));
        }
        Some((from, to)) => {
            let own = ParagraphFormat::of(*align, para);
            let ranges = paragraph_ranges(spans);
            let total = ranges.last().map_or(0, |r| r.1);
            for i in paragraphs_in(spans, from, to) {
                let Some(&(s, e)) = ranges.get(i) else {
                    continue;
                };
                let mut fmt = carried(spans, s).cloned().unwrap_or_else(|| own.clone());
                f(&mut fmt);
                let value = (fmt != own).then(|| Box::new(fmt));
                // The paragraph's characters and the newline that ends it.
                let end = (e + 1).min(total);
                if end > s {
                    spans_apply(spans, s, end, |sp| sp.paragraph = value.clone());
                } else if s > 0 {
                    // An empty last paragraph follows the one before it:
                    // only its newline can carry the format.
                    spans_apply(spans, s - 1, s, |sp| sp.paragraph = value.clone());
                } else {
                    // An empty text: its one span.
                    for sp in spans.iter_mut() {
                        sp.paragraph = value.clone();
                    }
                }
            }
        }
    }
}

/// The alignment and style the object shows for its paragraphs as a
/// whole: the first paragraph's.
pub fn first_paragraph_style(
    spans: &[TextSpan],
    align: TextAlign,
    para: &ParagraphStyle,
) -> (TextAlign, ParagraphStyle) {
    let f = paragraph_format(spans, align, para, 0);
    let mut p = para.clone();
    let a = f.apply_to(&mut p);
    (a, p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    fn text(s: &str) -> ShapeKind {
        ShapeKind::Text {
            spans: vec![TextSpan::new(s, "Sans", 12.0)],
            origin: Point::ZERO,
            frame: None,
            align: TextAlign::Left,
            para: ParagraphStyle::default(),
            on_path: None,
        }
    }

    fn spans(k: &ShapeKind) -> &[TextSpan] {
        match k {
            ShapeKind::Text { spans, .. } => spans,
            _ => &[],
        }
    }

    fn styles(k: &ShapeKind) -> Vec<(TextAlign, ParagraphStyle)> {
        match k {
            ShapeKind::Text {
                spans, align, para, ..
            } => paragraph_styles(spans, *align, para),
            _ => Vec::new(),
        }
    }

    #[test]
    fn paragraphs_are_found_between_line_breaks() {
        let s = vec![
            TextSpan::new("ab\n", "A", 1.0),
            TextSpan::new("\ncd", "B", 1.0),
        ];
        assert_eq!(paragraph_ranges(&s), vec![(0, 2), (3, 3), (4, 6)]);
        assert_eq!(paragraph_of(&s, 2), 0);
        assert_eq!(paragraph_of(&s, 3), 1);
        assert_eq!(paragraph_of(&s, 6), 2);
        assert_eq!(paragraphs_in(&s, 1, 4), vec![0, 1]);
        assert_eq!(paragraphs_in(&s, 1, 5), vec![0, 1, 2]);
    }

    #[test]
    fn a_selection_formats_only_its_paragraphs() {
        let mut k = text("one\ntwo\nthree");
        // The caret in "two".
        edit_paragraph_formats(&mut k, Some((5, 5)), |f| {
            f.align = TextAlign::Center;
            f.space_before = 3.0;
        });
        let st = styles(&k);
        assert_eq!(st.len(), 3);
        assert_eq!(st[0].0, TextAlign::Left);
        assert_eq!(st[1].0, TextAlign::Center);
        assert_eq!(st[1].1.space_before, 3.0);
        assert_eq!(st[2].0, TextAlign::Left);
        // The whole object: the change reaches every paragraph and keeps
        // the second one's own alignment.
        let old = ParagraphFormat::of(TextAlign::Left, &ParagraphStyle::default());
        let mut new = old.clone();
        new.left_indent = 4.0;
        edit_paragraph_formats(&mut k, None, |f| f.take_changes(&old, &new));
        let st = styles(&k);
        assert!(st.iter().all(|(_, p)| p.left_indent == 4.0));
        assert_eq!(st[1].0, TextAlign::Center);
        // Set back to the object's formatting: nothing carried any more.
        edit_paragraph_formats(&mut k, Some((5, 5)), |f| {
            f.align = TextAlign::Left;
            f.space_before = 0.0;
        });
        assert!(spans(&k).iter().all(|s| s.paragraph.is_none()));
        assert_eq!(spans(&k).len(), 1);
    }

    #[test]
    fn typing_at_a_paragraph_start_keeps_its_format() {
        let mut k = text("one\ntwo");
        edit_paragraph_formats(&mut k, Some((4, 4)), |f| f.align = TextAlign::Right);
        let ShapeKind::Text { spans, .. } = &mut k else {
            unreachable!()
        };
        crate::document::spans_insert(spans, 4, "x");
        let st = styles(&k);
        assert_eq!(st[1].0, TextAlign::Right);
        assert_eq!(st[0].0, TextAlign::Left);
        // Enter inside the second paragraph: both halves keep its format.
        let ShapeKind::Text { spans, .. } = &mut k else {
            unreachable!()
        };
        crate::document::spans_insert(spans, 6, "\n");
        let st = styles(&k);
        assert_eq!(
            st.iter().map(|s| s.0).collect::<Vec<_>>(),
            vec![TextAlign::Left, TextAlign::Right, TextAlign::Right]
        );
    }

    #[test]
    fn old_files_give_every_paragraph_the_object_format() {
        let json =
            r#"{"text":"a\nb","font_family":"Sans","size_pt":12.0,"bold":false,"italic":false}"#;
        let sp: TextSpan = serde_json::from_str(json).unwrap();
        assert!(sp.paragraph.is_none());
        let p = ParagraphStyle {
            space_after: 2.0,
            ..ParagraphStyle::default()
        };
        let st = paragraph_styles(&[sp], TextAlign::Center, &p);
        assert_eq!(st.len(), 2);
        assert!(st
            .iter()
            .all(|(a, p)| *a == TextAlign::Center && p.space_after == 2.0));
    }
}
