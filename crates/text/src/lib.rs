//! Text engine: finds system fonts, shapes a run with rustybuzz and returns
//! glyph outlines as paths in millimetres, so text is just vectors for the
//! renderer and for "convert to curves".

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tracedraw_core::{
    document::{TextAlign, TextSpan},
    geometry::{Affine, BezPath, Point, Rect},
};

/// Points to millimetres.
pub const PT_MM: f64 = 25.4 / 72.0;

pub struct FontSystem {
    db: fontdb::Database,
    faces: Mutex<HashMap<fontdb::ID, Arc<Vec<u8>>>>,
    families: Vec<String>,
    cache: Mutex<HashMap<String, Arc<BezPath>>>,
}

/// Register this engine as the core's text outliner (call once at startup).
pub fn install() {
    tracedraw_core::document::text_outline::set(|spans, width, align| {
        fonts().outline_cached(spans, width, align).as_ref().clone()
    });
}

static SYSTEM: OnceLock<FontSystem> = OnceLock::new();

/// The process-wide font system; loads system fonts on first use.
pub fn fonts() -> &'static FontSystem {
    SYSTEM.get_or_init(FontSystem::load)
}

impl FontSystem {
    fn load() -> Self {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let mut families: Vec<String> = db
            .faces()
            .flat_map(|f| f.families.iter().map(|(n, _)| n.clone()))
            .collect();
        families.sort();
        families.dedup();
        log::info!(
            "loaded {} font faces, {} families",
            db.len(),
            families.len()
        );
        FontSystem {
            db,
            faces: Mutex::new(HashMap::new()),
            families,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Sorted list of family names available on this machine.
    pub fn families(&self) -> &[String] {
        &self.families
    }

    /// Pick a face for a span; falls back to a sans-serif, then to anything.
    fn face_for(&self, family: &str, bold: bool, italic: bool) -> Option<fontdb::ID> {
        let weight = if bold {
            fontdb::Weight::BOLD
        } else {
            fontdb::Weight::NORMAL
        };
        let style = if italic {
            fontdb::Style::Italic
        } else {
            fontdb::Style::Normal
        };
        let fams = [
            fontdb::Family::Name(family),
            fontdb::Family::SansSerif,
            fontdb::Family::Name("DejaVu Sans"),
            fontdb::Family::Name("Liberation Sans"),
            fontdb::Family::Name("Arial"),
        ];
        let q = fontdb::Query {
            families: &fams,
            weight,
            stretch: fontdb::Stretch::Normal,
            style,
        };
        self.db
            .query(&q)
            .or_else(|| self.db.faces().next().map(|f| f.id))
    }

    fn face_data(&self, id: fontdb::ID) -> Option<(Arc<Vec<u8>>, u32)> {
        let mut cache = self.faces.lock().ok()?;
        if let Some(d) = cache.get(&id) {
            let idx = self.db.face(id)?.index;
            return Some((d.clone(), idx));
        }
        let (data, idx) = self
            .db
            .with_face_data(id, |d, i| (Arc::new(d.to_vec()), i))?;
        cache.insert(id, data.clone());
        Some((data, idx))
    }

    /// Cached outline; shaping is expensive and the same text is asked for
    /// every frame.
    pub fn outline_cached(
        &self,
        spans: &[TextSpan],
        width: Option<f64>,
        align: TextAlign,
    ) -> Arc<BezPath> {
        let key = format!(
            "{:?}|{:?}|{}",
            width,
            align,
            spans
                .iter()
                .map(|s| format!(
                    "{}|{}|{}|{}|{}",
                    s.font_family, s.size_pt, s.bold, s.italic, s.text
                ))
                .collect::<Vec<_>>()
                .join("\u{1}")
        );
        if let Ok(c) = self.cache.lock() {
            if let Some(p) = c.get(&key) {
                return p.clone();
            }
        }
        let p = Arc::new(self.outline_wrapped(spans, width, align).path);
        if let Ok(mut c) = self.cache.lock() {
            if c.len() > 2000 {
                c.clear();
            }
            c.insert(key, p.clone());
        }
        p
    }

    /// Paragraph layout: wrap at `width` (mm) by words, align lines.
    pub fn outline_wrapped(
        &self,
        spans: &[TextSpan],
        width: Option<f64>,
        align: TextAlign,
    ) -> TextLayout {
        let Some(width) = width else {
            return self.outline_aligned(spans, align, None);
        };
        // Wrap: measure words with the first span's style (mixed styles per
        // paragraph come later).
        let Some(style) = spans.first() else {
            return self.outline(spans);
        };
        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
        let space_w = self.measure(style, " ");
        let mut lines: Vec<String> = Vec::new();
        for para in text.split('\n') {
            let mut line = String::new();
            let mut line_w = 0.0;
            for word in para.split(' ') {
                let w = self.measure(style, word);
                let add = if line.is_empty() { w } else { space_w + w };
                if !line.is_empty() && line_w + add > width {
                    lines.push(std::mem::take(&mut line));
                    line_w = 0.0;
                }
                if !line.is_empty() {
                    line.push(' ');
                    line_w += space_w;
                }
                line.push_str(word);
                line_w += w;
            }
            lines.push(line);
        }
        let wrapped = TextSpan {
            text: lines.join("\n"),
            ..style.clone()
        };
        self.outline_aligned(&[wrapped], align, Some(width))
    }

    fn measure(&self, style: &TextSpan, text: &str) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        let span = TextSpan {
            text: text.to_string(),
            ..style.clone()
        };
        self.outline(&[span]).bounds.width()
    }

    /// Lay out each line separately so it can be aligned.
    fn outline_aligned(
        &self,
        spans: &[TextSpan],
        align: TextAlign,
        width: Option<f64>,
    ) -> TextLayout {
        if align == TextAlign::Left {
            return self.outline(spans);
        }
        let Some(style) = spans.first() else {
            return self.outline(spans);
        };
        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
        let lines: Vec<&str> = text.split('\n').collect();
        let widths: Vec<f64> = lines.iter().map(|l| self.measure(style, l)).collect();
        let max_w = width.unwrap_or_else(|| widths.iter().cloned().fold(0.0, f64::max));
        let full = self.outline(spans); // for line height and bounds
        let line_h = if lines.len() > 1 {
            (full.bounds.height() - (full.bounds.y1)) / (lines.len() as f64 - 1.0)
        } else {
            0.0
        };
        let _ = line_h;
        let mut path = BezPath::new();
        let mut y = 0.0;
        let step = {
            // Recover the line step from a two-line layout.
            let two = TextSpan {
                text: "x\nx".into(),
                ..style.clone()
            };
            let l = self.outline(&[two]);
            (l.bounds.y1 - l.bounds.y0)
                - self
                    .outline(&[TextSpan {
                        text: "x".into(),
                        ..style.clone()
                    }])
                    .bounds
                    .height()
        };
        for (i, line) in lines.iter().enumerate() {
            let w = widths[i];
            let dx = match align {
                TextAlign::Left | TextAlign::Justify => 0.0,
                TextAlign::Center => (max_w - w) / 2.0,
                TextAlign::Right => max_w - w,
            };
            let span = TextSpan {
                text: line.to_string(),
                ..style.clone()
            };
            let l = self.outline(&[span]);
            path.extend(
                (Affine::translate((dx, y)) * l.path)
                    .elements()
                    .iter()
                    .copied(),
            );
            y -= step;
        }
        TextLayout {
            bounds: Rect::new(0.0, y + step - full.bounds.height(), max_w, full.bounds.y1),
            path,
            glyphs: full.glyphs,
        }
    }

    /// Outline of a text run as a path in mm, baseline at the origin, text
    /// advancing along +x and glyphs extending toward +y (page space, Y up).
    pub fn outline(&self, spans: &[TextSpan]) -> TextLayout {
        let mut path = BezPath::new();
        let mut pen_x = 0.0f64;
        let mut line_y = 0.0f64;
        let mut ascent_mm = 0.0f64;
        let mut descent_mm = 0.0f64;
        let mut max_x = 0.0f64;
        let mut line_height = 0.0f64;
        let mut glyph_count = 0usize;

        for span in spans {
            let Some(id) = self.face_for(&span.font_family, span.bold, span.italic) else {
                continue;
            };
            let Some((data, index)) = self.face_data(id) else {
                continue;
            };
            let Ok(face) = ttf_parser::Face::parse(&data, index) else {
                continue;
            };
            let Some(hb) = rustybuzz::Face::from_slice(&data, index) else {
                continue;
            };
            let upem = face.units_per_em() as f64;
            let scale = span.size_pt * PT_MM / upem;
            let synth_bold = span.bold && face.weight().to_number() < 600;
            let synth_italic = span.italic && !face.is_italic();
            ascent_mm = ascent_mm.max(face.ascender() as f64 * scale);
            descent_mm = descent_mm.max(-(face.descender() as f64) * scale);
            line_height = line_height.max(
                (face.ascender() as f64 - face.descender() as f64 + face.line_gap() as f64) * scale,
            );

            for (li, line) in span.text.split('\n').enumerate() {
                if li > 0 {
                    line_y -= line_height.max(span.size_pt * PT_MM * 1.2);
                    pen_x = 0.0;
                }
                let mut buffer = rustybuzz::UnicodeBuffer::new();
                buffer.push_str(line);
                let glyphs = rustybuzz::shape(&hb, &[], buffer);
                let infos = glyphs.glyph_infos();
                let positions = glyphs.glyph_positions();
                for (info, pos) in infos.iter().zip(positions) {
                    let gid = ttf_parser::GlyphId(info.glyph_id as u16);
                    let x = pen_x + pos.x_offset as f64 * scale;
                    let y = line_y + pos.y_offset as f64 * scale;
                    let mut sink = OutlineSink {
                        path: BezPath::new(),
                        scale,
                        x,
                        y,
                    };
                    face.outline_glyph(gid, &mut sink);
                    let mut gp = sink.path;
                    if synth_italic {
                        gp = Affine::new([1.0, 0.0, 0.2, 1.0, -0.2 * y, 0.0]) * gp;
                    }
                    if synth_bold {
                        // Cheap faux bold: draw twice slightly offset.
                        let off = span.size_pt * PT_MM * 0.02;
                        let mut g2 = Affine::translate((off, 0.0)) * gp.clone();
                        gp.extend(g2.elements().iter().copied());
                        g2 = BezPath::new();
                        let _ = g2;
                    }
                    path.extend(gp.elements().iter().copied());
                    pen_x += pos.x_advance as f64 * scale;
                    glyph_count += 1;
                }
                max_x = max_x.max(pen_x);
            }
        }
        let bounds = Rect::new(0.0, line_y - descent_mm, max_x, ascent_mm);
        TextLayout {
            path,
            bounds,
            glyphs: glyph_count,
        }
    }
}

pub struct TextLayout {
    pub path: BezPath,
    /// Logical bounds (advance width x ascent/descent), baseline at y = 0.
    pub bounds: Rect,
    pub glyphs: usize,
}

struct OutlineSink {
    path: BezPath,
    scale: f64,
    x: f64,
    y: f64,
}

impl OutlineSink {
    fn p(&self, x: f32, y: f32) -> Point {
        Point::new(
            self.x + x as f64 * self.scale,
            self.y + y as f64 * self.scale,
        )
    }
}

impl ttf_parser::OutlineBuilder for OutlineSink {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to(self.p(x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(self.p(x, y));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path.quad_to(self.p(x1, y1), self.p(x, y));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path
            .curve_to(self.p(x1, y1), self.p(x2, y2), self.p(x, y));
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_some_text_if_any_font_exists() {
        let f = fonts();
        let spans = vec![TextSpan {
            text: "Ab".into(),
            font_family: "Nonexistent".into(),
            size_pt: 24.0,
            bold: false,
            italic: false,
        }];
        let layout = f.outline(&spans);
        if f.families().is_empty() {
            assert_eq!(layout.glyphs, 0);
        } else {
            assert_eq!(layout.glyphs, 2);
            assert!(layout.bounds.width() > 1.0);
            assert!(!layout.path.elements().is_empty());
        }
    }
}
