//! Text > Encode: repair text imported with the wrong character encoding,
//! and Text > Make Text Web Compatible.
//!
//! Mojibake such as "Ã©" for "é" comes from bytes written in one encoding
//! and read in another. The fix maps each character back to the byte it
//! came from (under the "read as" encoding) and decodes the bytes again with
//! the right one. The conversion is pure and tested; the dialog previews it
//! before applying.

use crate::app::App;
use crate::i18n::tr;
use tracedraw_core::{
    document::{ShapeKind, TextSpan},
    live::Effect,
    Color, Command, Fill,
};

/// Windows-1252 code points for bytes 0x80..0x9f (0 = undefined).
const CP1252_HIGH: [u16; 32] = [
    0x20ac, 0, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
    0x0152, 0, 0x017d, 0, 0, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x02dc,
    0x2122, 0x0161, 0x203a, 0x0153, 0, 0x017e, 0x0178,
];

/// Mac OS Roman code points for bytes 0x80..0xff.
const MAC_ROMAN_HIGH: [u16; 128] = [
    0x00c4, 0x00c5, 0x00c7, 0x00c9, 0x00d1, 0x00d6, 0x00dc, 0x00e1, 0x00e0, 0x00e2, 0x00e4, 0x00e3,
    0x00e5, 0x00e7, 0x00e9, 0x00e8, 0x00ea, 0x00eb, 0x00ed, 0x00ec, 0x00ee, 0x00ef, 0x00f1, 0x00f3,
    0x00f2, 0x00f4, 0x00f6, 0x00f5, 0x00fa, 0x00f9, 0x00fb, 0x00fc, 0x2020, 0x00b0, 0x00a2, 0x00a3,
    0x00a7, 0x2022, 0x00b6, 0x00df, 0x00ae, 0x00a9, 0x2122, 0x00b4, 0x00a8, 0x2260, 0x00c6, 0x00d8,
    0x221e, 0x00b1, 0x2264, 0x2265, 0x00a5, 0x00b5, 0x2202, 0x2211, 0x220f, 0x03c0, 0x222b, 0x00aa,
    0x00ba, 0x03a9, 0x00e6, 0x00f8, 0x00bf, 0x00a1, 0x00ac, 0x221a, 0x0192, 0x2248, 0x2206, 0x00ab,
    0x00bb, 0x2026, 0x00a0, 0x00c0, 0x00c3, 0x00d5, 0x0152, 0x0153, 0x2013, 0x2014, 0x201c, 0x201d,
    0x2018, 0x2019, 0x00f7, 0x25ca, 0x00ff, 0x0178, 0x2044, 0x20ac, 0x2039, 0x203a, 0xfb01, 0xfb02,
    0x2021, 0x00b7, 0x201a, 0x201e, 0x2030, 0x00c2, 0x00ca, 0x00c1, 0x00cb, 0x00c8, 0x00cd, 0x00ce,
    0x00cf, 0x00cc, 0x00d3, 0x00d4, 0xf8ff, 0x00d2, 0x00da, 0x00db, 0x00d9, 0x0131, 0x02c6, 0x02dc,
    0x00af, 0x02d8, 0x02d9, 0x02da, 0x00b8, 0x02dd, 0x02db, 0x02c7,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Encoding {
    #[default]
    Utf8,
    Windows1252,
    Latin1,
    MacRoman,
    /// Windows-1251 (Cyrillic).
    Windows1251,
}

impl Encoding {
    pub const ALL: [Encoding; 5] = [
        Encoding::Utf8,
        Encoding::Windows1252,
        Encoding::Latin1,
        Encoding::MacRoman,
        Encoding::Windows1251,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Windows1252 => "Windows-1252 (Western)",
            Encoding::Latin1 => "ISO-8859-1 (Latin-1)",
            Encoding::MacRoman => "Mac OS Roman",
            Encoding::Windows1251 => "Windows-1251 (Cyrillic)",
        }
    }

    /// The character a single byte decodes to, or None when the byte is
    /// undefined in this encoding (or the encoding is multi-byte).
    fn decode_byte(self, b: u8) -> Option<char> {
        if b < 0x80 {
            return Some(b as char);
        }
        let u = match self {
            Encoding::Utf8 => return None,
            Encoding::Latin1 => b as u32,
            Encoding::Windows1252 => {
                if b < 0xa0 {
                    CP1252_HIGH[(b - 0x80) as usize] as u32
                } else {
                    b as u32
                }
            }
            Encoding::MacRoman => MAC_ROMAN_HIGH[(b - 0x80) as usize] as u32,
            Encoding::Windows1251 => cp1251(b) as u32,
        };
        if u == 0 {
            None
        } else {
            char::from_u32(u)
        }
    }

    /// The byte a character came from under this encoding.
    fn encode_char(self, c: char) -> Option<u8> {
        if (c as u32) < 0x80 {
            return Some(c as u8);
        }
        (0x80u8..=0xff).find(|b| self.decode_byte(*b) == Some(c))
    }

    fn decode(self, bytes: &[u8]) -> String {
        match self {
            Encoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
            _ => bytes
                .iter()
                .map(|b| self.decode_byte(*b).unwrap_or('\u{fffd}'))
                .collect(),
        }
    }
}

/// Windows-1251 code point for a byte at or above 0x80.
fn cp1251(b: u8) -> u16 {
    const HIGH: [u16; 64] = [
        0x0402, 0x0403, 0x201a, 0x0453, 0x201e, 0x2026, 0x2020, 0x2021, 0x20ac, 0x2030, 0x0409,
        0x2039, 0x040a, 0x040c, 0x040b, 0x040f, 0x0452, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022,
        0x2013, 0x2014, 0, 0x2122, 0x0459, 0x203a, 0x045a, 0x045c, 0x045b, 0x045f, 0x00a0, 0x040e,
        0x045e, 0x0408, 0x00a4, 0x0490, 0x00a6, 0x00a7, 0x0401, 0x00a9, 0x0404, 0x00ab, 0x00ac,
        0x00ad, 0x00ae, 0x0407, 0x00b0, 0x00b1, 0x0406, 0x0456, 0x0491, 0x00b5, 0x00b6, 0x00b7,
        0x0451, 0x2116, 0x0454, 0x00bb, 0x0458, 0x0405, 0x0455, 0x0457,
    ];
    if b < 0xc0 {
        HIGH[(b - 0x80) as usize]
    } else {
        0x0410 + (b - 0xc0) as u16
    }
}

/// Re-interpret `text`, which was read as `from`, as if it were `to`.
/// Characters that cannot have come from `from` are kept as they are.
pub fn reinterpret(text: &str, from: Encoding, to: Encoding) -> String {
    if from == to {
        return text.to_string();
    }
    let mut out = String::new();
    let mut bytes: Vec<u8> = Vec::new();
    let flush = |bytes: &mut Vec<u8>, out: &mut String| {
        if !bytes.is_empty() {
            out.push_str(&to.decode(bytes));
            bytes.clear();
        }
    };
    for c in text.chars() {
        let b = match from {
            // Text that was read as UTF-8 is already right; only the
            // low range maps one to one.
            Encoding::Utf8 => {
                if (c as u32) < 0x80 {
                    Some(c as u8)
                } else {
                    let mut buf = [0u8; 4];
                    bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                    continue;
                }
            }
            e => e.encode_char(c),
        };
        match b {
            Some(b) => bytes.push(b),
            None => {
                flush(&mut bytes, &mut out);
                out.push(c);
            }
        }
    }
    flush(&mut bytes, &mut out);
    out
}

/// A representative solid colour for any fill (first stop, front colour,
/// first mesh node); black for bitmap and vector tiles and for no fill.
fn solid_of(fill: &Fill) -> Color {
    match fill {
        Fill::Solid(c) => *c,
        Fill::None => Color::BLACK,
        Fill::Fountain(f) => f.stops.first().map(|s| s.color).unwrap_or(Color::BLACK),
        Fill::Pattern(tracedraw_core::Pattern::TwoColor { front, .. }) => *front,
        Fill::Pattern(_) => Color::BLACK,
        Fill::Texture(t) => t.color_a,
        Fill::Mesh(m) => m.nodes.first().map(|n| n.color).unwrap_or(Color::BLACK),
    }
}

impl App {
    /// The selected text objects' content, joined, for the dialog preview.
    pub fn selected_text_content(&self) -> String {
        self.text_shapes()
            .iter()
            .filter_map(|s| match &s.kind {
                ShapeKind::Text { spans, .. } => {
                    Some(spans.iter().map(|x| x.text.as_str()).collect::<String>())
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Text > Encode: apply the re-interpretation to every selected text.
    pub fn reencode_selected_text(&mut self, from: Encoding, to: Encoding) {
        let mut cmds = Vec::new();
        for s in self.text_shapes() {
            let ShapeKind::Text {
                spans,
                origin,
                frame,
                align,
                para,
                on_path,
            } = s.kind.clone()
            else {
                continue;
            };
            let new: Vec<TextSpan> = spans
                .into_iter()
                .map(|mut sp| {
                    sp.text = reinterpret(&sp.text, from, to);
                    sp
                })
                .collect();
            cmds.push(Command::SetShapeKind {
                shape: s.id,
                kind: ShapeKind::Text {
                    spans: new,
                    origin,
                    frame,
                    align,
                    para,
                    on_path,
                },
            });
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Encode Text", &cmds);
        }
    }

    /// Text > Make Text Web Compatible: plain text that any browser renders
    /// the same way. Fountain and pattern fills become a solid colour,
    /// outlines and live effects are dropped, per-span colours are kept
    /// only when solid.
    pub fn make_text_web_compatible(&mut self) {
        let mut cmds = Vec::new();
        for s in self.text_shapes() {
            let solid = solid_of(&s.fill);
            cmds.push(Command::SetFill {
                shapes: vec![s.id],
                fill: Fill::Solid(solid),
            });
            cmds.push(Command::SetStroke {
                shapes: vec![s.id],
                stroke: None,
            });
            if !s.effects.is_empty() {
                let effects: Vec<Effect> = Vec::new();
                cmds.push(Command::SetEffects {
                    shape: s.id,
                    effects,
                });
            }
            if s.shadow.is_some() {
                cmds.push(Command::SetShadow {
                    shapes: vec![s.id],
                    shadow: None,
                });
            }
            if let ShapeKind::Text {
                spans,
                origin,
                frame,
                align,
                para,
                on_path,
            } = s.kind.clone()
            {
                let changed = spans
                    .iter()
                    .any(|sp| matches!(sp.fill, Some(ref f) if !matches!(f, Fill::Solid(_))));
                if changed {
                    let spans = spans
                        .into_iter()
                        .map(|mut sp| {
                            if let Some(f) = &sp.fill {
                                if !matches!(f, Fill::Solid(_)) {
                                    sp.fill = None;
                                }
                            }
                            sp
                        })
                        .collect();
                    cmds.push(Command::SetShapeKind {
                        shape: s.id,
                        kind: ShapeKind::Text {
                            spans,
                            origin,
                            frame,
                            align,
                            para,
                            on_path,
                        },
                    });
                }
            }
        }
        if cmds.is_empty() {
            return;
        }
        let _ = self.engine.run_batch("Make Text Web Compatible", &cmds);
        self.status = tr("status.web_compatible");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_read_as_cp1252_is_repaired() {
        // "é" (c3 a9) read as Windows-1252 shows as "Ã©".
        assert_eq!(
            reinterpret("caf\u{c3}\u{a9}", Encoding::Windows1252, Encoding::Utf8),
            "café"
        );
        // The euro sign: e2 82 ac read as cp1252 is "â‚¬".
        assert_eq!(
            reinterpret(
                "\u{e2}\u{201a}\u{ac}",
                Encoding::Windows1252,
                Encoding::Utf8
            ),
            "€"
        );
    }

    #[test]
    fn latin1_read_as_utf8_round_trips_through_bytes() {
        // Bytes e9 read as Latin-1 give "é"; reading them as cp1252 is the same.
        assert_eq!(
            reinterpret("é", Encoding::Latin1, Encoding::Windows1252),
            "é"
        );
        // Cyrillic bytes read as Latin-1 look like "Ïðèâåò"; as cp1251 they
        // spell "Привет".
        assert_eq!(
            reinterpret(
                "\u{cf}\u{f0}\u{e8}\u{e2}\u{e5}\u{f2}",
                Encoding::Latin1,
                Encoding::Windows1251
            ),
            "Привет"
        );
    }

    #[test]
    fn unmappable_characters_are_kept() {
        assert_eq!(
            reinterpret("a 日本 b", Encoding::Windows1252, Encoding::Utf8),
            "a 日本 b"
        );
        assert_eq!(reinterpret("x", Encoding::Utf8, Encoding::Utf8), "x");
    }
}
