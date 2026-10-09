//! Text engine: finds system fonts, shapes runs with rustybuzz (OpenType
//! features, kerning) and lays paragraphs out with wrapping, alignment,
//! justification, leading, indents, tabs, bullets, drop caps, columns,
//! hyphenation and text on a path. The result is a path in millimetres,
//! so text is plain vectors for the renderer and for "convert to curves".
//!
//! Coordinates: baseline of the first line at y = 0 for artistic text; the
//! top of the frame at y = 0 for paragraph text. Y is up, as everywhere.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tracedraw_core::{
    document::{text_outline::TextRequest, ParagraphStyle, TextAlign, TextOnPath, TextSpan},
    geometry::{Affine, BezPath, Point, Rect, Shape as _, Vec2},
};

pub mod hyphen;

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
    tracedraw_core::document::text_outline::set(|req| fonts().outline_cached(req).as_ref().clone());
}

static SYSTEM: OnceLock<FontSystem> = OnceLock::new();

/// The process-wide font system; loads system fonts on first use.
pub fn fonts() -> &'static FontSystem {
    SYSTEM.get_or_init(FontSystem::load)
}

/// One shaped glyph, positioned relative to the pen of its line.
#[derive(Clone)]
struct Glyph {
    /// Outline at the pen origin (x = 0, baseline y = 0), mm.
    path: Arc<BezPath>,
    advance: f64,
    x_offset: f64,
    y_offset: f64,
    /// The character this glyph (cluster) starts; used for spaces and tabs.
    ch: char,
    /// Index of that character in the concatenated span text.
    char_idx: usize,
    span: usize,
    /// Width of a trailing hyphen if the line breaks after this glyph.
    hyphen_after: Option<Arc<HyphenGlyph>>,
}

#[derive(Clone)]
struct HyphenGlyph {
    path: BezPath,
    advance: f64,
}

/// Vertical metrics of a span's face at its size, mm.
#[derive(Clone, Copy, Default)]
struct Metrics {
    ascent: f64,
    descent: f64,
    line_height: f64,
    underline_pos: f64,
    underline_thick: f64,
    strike_pos: f64,
}

struct Line {
    glyphs: Vec<Glyph>,
    /// Trailing hyphen to draw when the line was broken inside a word.
    hyphen: Option<Arc<HyphenGlyph>>,
    metrics: Metrics,
    /// Last line of its paragraph (not justified).
    last_in_para: bool,
    first_in_para: bool,
    para: usize,
    /// Index in the concatenated span text of the line's first character
    /// (the paragraph start for an empty line).
    start_char: usize,
}

pub struct TextLayout {
    pub path: BezPath,
    /// Logical bounds, mm.
    pub bounds: Rect,
    pub glyphs: usize,
    /// True when paragraph text did not fit its frame.
    pub overflow: bool,
    /// Number of characters of the concatenated span text (in order, line
    /// breaks and tabs included) placed inside the frame before it
    /// overflowed. Equals the total character count for artistic text and
    /// for frames that hold all their text, so
    /// `tracedraw_core::split_spans_at(spans, fitted_chars)` gives the
    /// spans that stay in this frame and the spans that flow to the next.
    pub fitted_chars: usize,
    /// One entry per laid-out line: (x of the line start, baseline y), mm,
    /// in layout space (frame top or first baseline at y = 0, Y up).
    pub baselines: Vec<(f64, f64)>,
    /// One entry per laid-out line, in order: the character range it holds
    /// and the pen position before each of its characters (for carets).
    pub lines: Vec<LineBox>,
}

/// Caret geometry of one laid-out line, mm, in layout space (Y up).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LineBox {
    /// Index of the line's first character in the concatenated span text.
    pub start_char: usize,
    /// One past the line's last character (the newline that ends a
    /// paragraph is not part of the line).
    pub end_char: usize,
    pub baseline: f64,
    pub ascent: f64,
    pub descent: f64,
    /// Pen x before each character from `start_char` to `end_char`, then
    /// after the last one: `end_char - start_char + 1` entries.
    pub edges: Vec<f64>,
}

impl TextLayout {
    /// Caret position for the character index `idx`: (x, baseline, ascent,
    /// descent) in layout space. Past the end of the text the caret sits
    /// after the last character.
    pub fn caret(&self, idx: usize) -> Option<(f64, f64, f64, f64)> {
        let line = self
            .lines
            .iter()
            .find(|l| idx >= l.start_char && idx < l.end_char)
            .or_else(|| self.lines.iter().rev().find(|l| idx >= l.start_char))
            .or_else(|| self.lines.first())?;
        let i = idx
            .saturating_sub(line.start_char)
            .min(line.edges.len().saturating_sub(1));
        let x = line.edges.get(i).copied()?;
        Some((x, line.baseline, line.ascent, line.descent))
    }

    /// Character index nearest to the layout-space point `p`: the line
    /// whose vertical band holds `p.y` (the nearest one outside all bands)
    /// and the nearest character edge on that line.
    pub fn hit_char(&self, p: Point) -> Option<usize> {
        let line = self
            .lines
            .iter()
            .find(|l| p.y <= l.baseline + l.ascent && p.y >= l.baseline - l.descent)
            .or_else(|| {
                self.lines.iter().min_by(|a, b| {
                    let da = (a.baseline - p.y).abs();
                    let db = (b.baseline - p.y).abs();
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })
            })?;
        let (i, _) = line.edges.iter().enumerate().min_by(|(_, a), (_, b)| {
            let da = (*a - p.x).abs();
            let db = (*b - p.x).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })?;
        Some(line.start_char + i)
    }

    /// Index of the line holding the character index `idx`.
    pub fn line_of(&self, idx: usize) -> Option<usize> {
        self.lines
            .iter()
            .position(|l| idx >= l.start_char && idx < l.end_char)
            .or_else(|| self.lines.iter().rposition(|l| idx >= l.start_char))
    }
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

    /// Raw file data and face index of the first installed family in
    /// `candidates`, for registering UI fallback fonts (scripts the bundled
    /// UI font lacks: CJK, Arabic, Devanagari, Bengali...).
    pub fn first_face_data(&self, candidates: &[&str]) -> Option<(String, Arc<Vec<u8>>, u32)> {
        for name in candidates {
            let Some(id) = self.db.query(&fontdb::Query {
                families: &[fontdb::Family::Name(name)],
                weight: fontdb::Weight::NORMAL,
                stretch: fontdb::Stretch::Normal,
                style: fontdb::Style::Normal,
            }) else {
                continue;
            };
            let got = self.db.with_face_data(id, |d, i| (Arc::new(d.to_vec()), i));
            if let Some((data, index)) = got {
                return Some((name.to_string(), data, index));
            }
        }
        None
    }

    /// Sorted list of family names available on this machine.
    pub fn families(&self) -> &[String] {
        &self.families
    }

    /// Whether a family exists (missing fonts are substituted and reported).
    pub fn has_family(&self, family: &str) -> bool {
        self.families.iter().any(|f| f == family)
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
    pub fn outline_cached(&self, req: &TextRequest) -> Arc<BezPath> {
        let key = format!(
            "{:?}|{:?}|{:?}|{}|{}",
            req.frame,
            req.align,
            req.para,
            req.on_path
                .map(|p| format!(
                    "{:?}|{}|{}|{}",
                    p.path.elements().len(),
                    p.offset,
                    p.distance,
                    p.mirror
                ))
                .unwrap_or_default(),
            req.spans
                .iter()
                .map(|s| format!(
                    "{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{}",
                    s.font_family,
                    s.size_pt,
                    s.bold,
                    s.italic,
                    s.tracking_pct,
                    s.baseline_shift_pt,
                    s.underline,
                    s.strikethrough,
                    s.features,
                    s.text
                ))
                .collect::<Vec<_>>()
                .join("\u{1}")
        );
        if let Ok(c) = self.cache.lock() {
            if let Some(p) = c.get(&key) {
                return p.clone();
            }
        }
        let p = Arc::new(self.layout(req).path);
        if let Ok(mut c) = self.cache.lock() {
            if c.len() > 2000 {
                c.clear();
            }
            c.insert(key, p.clone());
        }
        p
    }

    /// Simple entry point: artistic text, left aligned.
    pub fn outline(&self, spans: &[TextSpan]) -> TextLayout {
        let para = ParagraphStyle::default();
        self.layout(&TextRequest {
            spans,
            frame: None,
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        })
    }

    /// Measure the advance width of a string in a span's style, mm.
    pub fn measure(&self, style: &TextSpan, text: &str) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        let span = TextSpan {
            text: text.to_string(),
            ..style.clone()
        };
        self.outline(&[span]).bounds.width()
    }

    // ----- shaping -----------------------------------------------------------

    fn metrics(&self, span: &TextSpan) -> Metrics {
        let Some(id) = self.face_for(&span.font_family, span.bold, span.italic) else {
            let s = span.size_pt * PT_MM;
            return Metrics {
                ascent: s * 0.8,
                descent: s * 0.2,
                line_height: s * 1.2,
                underline_pos: -s * 0.1,
                underline_thick: s * 0.05,
                strike_pos: s * 0.3,
            };
        };
        let Some((data, index)) = self.face_data(id) else {
            return Metrics::default();
        };
        let Ok(face) = ttf_parser::Face::parse(&data, index) else {
            return Metrics::default();
        };
        let scale = span.size_pt * PT_MM / face.units_per_em() as f64;
        let asc = face.ascender() as f64 * scale;
        let desc = -(face.descender() as f64) * scale;
        let gap = face.line_gap() as f64 * scale;
        let ul = face.underline_metrics();
        let st = face.strikeout_metrics();
        Metrics {
            ascent: asc,
            descent: desc,
            line_height: asc + desc + gap,
            underline_pos: ul.map(|u| u.position as f64 * scale).unwrap_or(-desc * 0.5),
            underline_thick: ul
                .map(|u| u.thickness as f64 * scale)
                .unwrap_or(span.size_pt * PT_MM * 0.05),
            strike_pos: st.map(|s| s.position as f64 * scale).unwrap_or(asc * 0.35),
        }
    }

    /// Shape one run of text in one span's style into glyphs.
    fn shape_run(
        &self,
        span: &TextSpan,
        span_idx: usize,
        text: &str,
        size_scale: f64,
        char_base: usize,
    ) -> Vec<Glyph> {
        let mut out = Vec::new();
        if text.is_empty() {
            return out;
        }
        let Some(id) = self.face_for(&span.font_family, span.bold, span.italic) else {
            return out;
        };
        let Some((data, index)) = self.face_data(id) else {
            return out;
        };
        let Ok(face) = ttf_parser::Face::parse(&data, index) else {
            return out;
        };
        let Some(hb) = rustybuzz::Face::from_slice(&data, index) else {
            return out;
        };
        let size_pt = span.size_pt * size_scale;
        let upem = face.units_per_em() as f64;
        let scale = size_pt * PT_MM / upem;
        let synth_bold = span.bold && face.weight().to_number() < 600;
        let synth_italic = span.italic && !face.is_italic();
        let tracking = span.tracking_pct / 100.0 * size_pt * PT_MM;
        let baseline = span.baseline_shift_pt * PT_MM;

        let features: Vec<rustybuzz::Feature> = span
            .features
            .iter()
            .map(|f| {
                let tag = rustybuzz::ttf_parser::Tag::from_bytes_lossy(f.as_bytes());
                rustybuzz::Feature::new(tag, 1, ..)
            })
            .collect();
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        let glyphs = rustybuzz::shape(&hb, &features, buffer);
        let infos = glyphs.glyph_infos();
        let positions = glyphs.glyph_positions();
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let hyphen_glyph = {
            let gid = face.glyph_index('-');
            gid.map(|g| {
                let mut sink = OutlineSink {
                    path: BezPath::new(),
                    scale,
                    x: 0.0,
                    y: baseline,
                };
                face.outline_glyph(g, &mut sink);
                Arc::new(HyphenGlyph {
                    path: sink.path,
                    advance: face.glyph_hor_advance(g).unwrap_or(0) as f64 * scale,
                })
            })
        };
        for (info, pos) in infos.iter().zip(positions) {
            let gid = ttf_parser::GlyphId(info.glyph_id as u16);
            let mut sink = OutlineSink {
                path: BezPath::new(),
                scale,
                x: 0.0,
                y: baseline,
            };
            face.outline_glyph(gid, &mut sink);
            let mut gp = sink.path;
            if synth_italic {
                gp = Affine::new([1.0, 0.0, 0.2, 1.0, 0.0, 0.0]) * gp;
            }
            if synth_bold {
                let off = size_pt * PT_MM * 0.02;
                let g2 = Affine::translate((off, 0.0)) * gp.clone();
                gp.extend(g2.elements().iter().copied());
            }
            let cluster = chars.iter().position(|(i, _)| *i == info.cluster as usize);
            let ch = cluster
                .and_then(|k| chars.get(k))
                .map(|(_, c)| *c)
                .unwrap_or(' ');
            out.push(Glyph {
                path: Arc::new(gp),
                advance: pos.x_advance as f64 * scale + tracking,
                x_offset: pos.x_offset as f64 * scale,
                y_offset: pos.y_offset as f64 * scale,
                ch,
                char_idx: char_base + cluster.unwrap_or(0),
                span: span_idx,
                hyphen_after: hyphen_glyph.clone(),
            });
        }
        out
    }

    // ----- layout ------------------------------------------------------------

    /// Full layout of a text object.
    pub fn layout(&self, req: &TextRequest) -> TextLayout {
        if req.para.fit_to_frame && req.frame.is_some() {
            // Scale the font size so the text fills the frame height.
            let (mut lo, mut hi) = (0.1, 20.0);
            let mut best = 1.0;
            for _ in 0..14 {
                let mid = (lo + hi) / 2.0;
                let l = self.layout_scaled(req, mid);
                if l.overflow {
                    hi = mid;
                } else {
                    best = mid;
                    lo = mid;
                }
            }
            return self.layout_scaled(req, best);
        }
        self.layout_scaled(req, 1.0)
    }

    fn layout_scaled(&self, req: &TextRequest, size_scale: f64) -> TextLayout {
        let spans = req.spans;
        if spans.is_empty() {
            return TextLayout {
                path: BezPath::new(),
                bounds: Rect::ZERO,
                glyphs: 0,
                overflow: false,
                fitted_chars: 0,
                baselines: Vec::new(),
                lines: Vec::new(),
            };
        }
        let para = req.para;
        let metrics: Vec<Metrics> = spans
            .iter()
            .map(|s| {
                let scaled = TextSpan {
                    size_pt: s.size_pt * size_scale,
                    ..s.clone()
                };
                self.metrics(&scaled)
            })
            .collect();

        // 1. Split the spans into paragraphs of glyph runs. `char_idx`
        // counts characters of the concatenated span text, newlines and
        // tabs included, so lines can report where they start.
        let mut paragraphs: Vec<Vec<Glyph>> = vec![Vec::new()];
        let mut para_starts: Vec<usize> = vec![0];
        let mut char_idx = 0usize;
        for (si, span) in spans.iter().enumerate() {
            let mut first = true;
            for piece in span.text.split('\n') {
                if !first {
                    // The newline itself.
                    char_idx += 1;
                    paragraphs.push(Vec::new());
                    para_starts.push(char_idx);
                }
                first = false;
                // Tabs are kept as glyph-less markers so the line breaker can
                // expand them to the next tab stop.
                for (ti, part) in piece.split('\t').enumerate() {
                    if ti > 0 {
                        if let Some(p) = paragraphs.last_mut() {
                            p.push(Glyph {
                                path: Arc::new(BezPath::new()),
                                advance: 0.0,
                                x_offset: 0.0,
                                y_offset: 0.0,
                                ch: '\t',
                                char_idx,
                                span: si,
                                hyphen_after: None,
                            })
                        }
                        char_idx += 1;
                    }
                    let run = self.shape_run(span, si, part, size_scale, char_idx);
                    char_idx += part.chars().count();
                    if let Some(p) = paragraphs.last_mut() {
                        p.extend(run);
                    }
                }
            }
        }
        let total_chars = char_idx;

        // 2. Break paragraphs into lines (paragraph text) or keep them whole.
        let columns = para.columns.max(1) as usize;
        let col_width = req
            .frame
            .map(|f| ((f.width - para.gutter * (columns as f64 - 1.0)) / columns as f64).max(1.0));
        let mut lines: Vec<Line> = Vec::new();
        for (pi, glyphs) in paragraphs.iter().enumerate() {
            let m = glyphs
                .first()
                .map(|g| metrics[g.span])
                .unwrap_or_else(|| metrics[0]);
            let indent_first = para.left_indent
                + para.first_line_indent
                + if para.bullets {
                    para.bullet_indent
                } else {
                    0.0
                };
            let indent_rest = para.left_indent
                + if para.bullets {
                    para.bullet_indent
                } else {
                    0.0
                };
            let para_start = para_starts.get(pi).copied().unwrap_or(0);
            match col_width {
                None => lines.push(Line {
                    glyphs: glyphs.clone(),
                    hyphen: None,
                    metrics: line_metrics(glyphs, &metrics, m),
                    last_in_para: true,
                    first_in_para: true,
                    para: pi,
                    start_char: para_start,
                }),
                Some(w) => {
                    let avail_first = (w - indent_first - para.right_indent).max(1.0);
                    let avail_rest = (w - indent_rest - para.right_indent).max(1.0);
                    let broken = break_lines(
                        glyphs,
                        avail_first,
                        avail_rest,
                        para,
                        &metrics,
                        pi,
                        para_start,
                    );
                    lines.extend(broken);
                }
            }
        }

        // 3. Place lines: columns, leading, spacing, indents, alignment.
        let mut path = BezPath::new();
        let mut glyph_count = 0usize;
        let mut overflow = false;
        let mut fitted_chars = total_chars;
        let frame_h = req.frame.map(|f| f.height);
        // Baseline grid: baselines snap to the next multiple of the pitch
        // below their natural position (y is negative below the frame top).
        let grid = if req.frame.is_some() && para.baseline_grid_mm > 1e-6 {
            Some(para.baseline_grid_mm)
        } else {
            None
        };
        let snap = |y: f64| -> f64 {
            match grid {
                Some(g) => -(((-y / g) - 1e-9).ceil().max(0.0) * g),
                None => y,
            }
        };
        let mut col = 0usize;
        let mut y = 0.0f64; // top of the frame or baseline of artistic text
        let mut min_y = 0.0f64;
        let mut max_x = 0.0f64;
        let mut prev_para: Option<usize> = None;
        let mut drop_cap_indent: (f64, usize) = (0.0, 0); // (width, lines remaining)
        let artistic = req.frame.is_none();
        let mut line_baselines: Vec<(f64, f64)> = Vec::new();
        let mut line_boxes: Vec<LineBox> = Vec::new();
        // Where each line's characters end: the next line's start, or the
        // paragraph end (before its newline) for the last line of one.
        let line_ends: Vec<usize> = lines
            .iter()
            .enumerate()
            .map(|(li, line)| {
                if line.last_in_para {
                    para_starts
                        .get(line.para + 1)
                        .map(|s| s.saturating_sub(1))
                        .unwrap_or(total_chars)
                } else {
                    lines
                        .get(li + 1)
                        .map(|n| n.start_char)
                        .unwrap_or(total_chars)
                }
            })
            .collect();

        for (li, line) in lines.iter().enumerate() {
            let m = line.metrics;
            let step = m.line_height * para.leading_pct / 100.0;
            // Paragraph spacing.
            let mut advance_y = if artistic {
                if line.para == 0 && line.first_in_para {
                    0.0
                } else {
                    step
                }
            } else if prev_para.is_none() {
                m.ascent
            } else {
                step
            };
            if !artistic && line.first_in_para && prev_para.is_some() {
                advance_y += para.space_before;
                if let Some(pp) = prev_para {
                    if pp != line.para {
                        advance_y += para.space_after;
                    }
                }
            }
            // Column overflow.
            let mut next_y = snap(y - advance_y);
            if let Some(h) = frame_h {
                if next_y - m.descent < -h && (y != 0.0 || col > 0) {
                    col += 1;
                    y = 0.0;
                    advance_y = m.ascent;
                    next_y = snap(y - advance_y);
                    if col >= columns && !overflow {
                        overflow = true;
                        fitted_chars = line.start_char.min(total_chars);
                    }
                }
            }
            y = next_y;
            prev_para = Some(line.para);

            let col_x = col.min(columns - 1) as f64 * (col_width.unwrap_or(0.0) + para.gutter);
            let indent = if line.first_in_para {
                para.left_indent + para.first_line_indent
            } else {
                para.left_indent
            } + if para.bullets {
                para.bullet_indent
            } else {
                0.0
            };

            // Drop cap: the first glyph of a paragraph, scaled over N lines.
            let mut glyphs: &[Glyph] = &line.glyphs;
            let mut x0 = col_x + indent;
            if para.drop_cap_lines > 1 && line.first_in_para && !glyphs.is_empty() && !artistic {
                let n = para.drop_cap_lines as f64;
                let g = &glyphs[0];
                let scale = n * step / m.line_height;
                let gp =
                    Affine::translate((x0, y)) * Affine::scale(scale) * g.path.as_ref().clone();
                path.extend(gp.elements().iter().copied());
                drop_cap_indent = (g.advance * scale + 1.0, para.drop_cap_lines as usize);
                glyphs = &glyphs[1..];
                glyph_count += 1;
            }
            if drop_cap_indent.1 > 0 {
                x0 += drop_cap_indent.0;
                drop_cap_indent.1 -= 1;
            }
            // Bullet.
            if para.bullets && line.first_in_para && !artistic {
                if let Some(span) = spans.get(glyphs.first().map(|g| g.span).unwrap_or(0)) {
                    let b = self.shape_run(span, 0, &para.bullet_char, size_scale, 0);
                    for g in &b {
                        let gp = Affine::translate((col_x + para.left_indent, y))
                            * g.path.as_ref().clone();
                        path.extend(gp.elements().iter().copied());
                    }
                }
            }

            let avail = col_width
                .map(|w| w - indent - para.right_indent - (x0 - col_x - indent))
                .unwrap_or(f64::INFINITY);
            let (content_w, n_spaces) = line_width(glyphs, para, x0 - col_x);
            let hyphen_w = line.hyphen.as_ref().map(|h| h.advance).unwrap_or(0.0);
            let total_w = content_w + hyphen_w;
            let (dx, space_extra) = match req.align {
                TextAlign::Left => (0.0, 0.0),
                TextAlign::Center => (((avail.min(1e9)) - total_w).max(0.0) / 2.0, 0.0),
                TextAlign::Right => ((avail.min(1e9) - total_w).max(0.0), 0.0),
                TextAlign::Justify => {
                    if line.last_in_para || n_spaces == 0 || !avail.is_finite() {
                        (0.0, 0.0)
                    } else {
                        (0.0, (avail - total_w).max(0.0) / n_spaces as f64)
                    }
                }
            };
            let avail_for_center = if avail.is_finite() { avail } else { total_w };
            let dx = if artistic {
                match req.align {
                    TextAlign::Center => -total_w / 2.0 + 0.0,
                    TextAlign::Right => -total_w,
                    _ => 0.0,
                }
            } else {
                dx
            };
            let _ = avail_for_center;

            // Emit glyphs.
            let mut pen = x0 + dx;
            let line_start = pen;
            let mut underline_runs: Vec<(usize, f64, f64)> = Vec::new();
            let end_char = line_ends.get(li).copied().unwrap_or(total_chars);
            let n_chars = end_char.saturating_sub(line.start_char);
            // Pen x before each character; clusters of several characters
            // share their glyph's advance evenly.
            let mut edges: Vec<f64> = vec![line_start; n_chars + 1];
            let mut record = |from: usize, to: usize, x0: f64, x1: f64| {
                let from = from.max(line.start_char);
                let to = to.min(end_char);
                if to <= from {
                    return;
                }
                let n = (to - from) as f64;
                for (k, c) in (from..to).enumerate() {
                    if let Some(e) = edges.get_mut(c - line.start_char) {
                        *e = x0 + (x1 - x0) * k as f64 / n;
                    }
                }
                if let Some(e) = edges.get_mut(to - line.start_char) {
                    *e = x1;
                }
            };
            for (gi, g) in glyphs.iter().enumerate() {
                let next_idx = glyphs
                    .get(gi + 1)
                    .map(|n| n.char_idx)
                    .unwrap_or(end_char)
                    .max(g.char_idx + 1);
                if g.ch == '\t' {
                    let rel = pen - col_x;
                    let next = next_tab(rel, &para.tabs);
                    let before = pen;
                    pen = col_x + next;
                    record(g.char_idx, next_idx, before, pen);
                    continue;
                }
                let gp =
                    Affine::translate((pen + g.x_offset, y + g.y_offset)) * g.path.as_ref().clone();
                path.extend(gp.elements().iter().copied());
                let adv = g.advance + if g.ch == ' ' { space_extra } else { 0.0 };
                let span = &spans[g.span];
                if span.underline || span.strikethrough {
                    match underline_runs.last_mut() {
                        Some((s, _, end)) if *s == g.span && (*end - pen).abs() < 1e-6 => {
                            *end = pen + adv
                        }
                        _ => underline_runs.push((g.span, pen, pen + adv)),
                    }
                }
                record(g.char_idx, next_idx, pen, pen + adv);
                pen += adv;
                glyph_count += 1;
            }
            line_boxes.push(LineBox {
                start_char: line.start_char,
                end_char,
                baseline: y,
                ascent: m.ascent,
                descent: m.descent,
                edges,
            });
            if let Some(h) = &line.hyphen {
                let gp = Affine::translate((pen, y)) * h.path.clone();
                path.extend(gp.elements().iter().copied());
                pen += h.advance;
            }
            for (si, a, b) in underline_runs {
                let span = &spans[si];
                let sm = metrics[si];
                if span.underline {
                    let r = Rect::new(
                        a,
                        y + sm.underline_pos - sm.underline_thick,
                        b,
                        y + sm.underline_pos,
                    );
                    path.extend(r.to_path(0.01).elements().iter().copied());
                }
                if span.strikethrough {
                    let r = Rect::new(
                        a,
                        y + sm.strike_pos - sm.underline_thick / 2.0,
                        b,
                        y + sm.strike_pos + sm.underline_thick / 2.0,
                    );
                    path.extend(r.to_path(0.01).elements().iter().copied());
                }
            }
            line_baselines.push((line_start, y));
            max_x = max_x.max(pen);
            min_y = min_y.min(y - m.descent);
        }

        let top = if artistic {
            lines.first().map(|l| l.metrics.ascent).unwrap_or(0.0)
        } else {
            0.0
        };
        let bounds = Rect::new(
            0.0,
            min_y,
            max_x.max(req.frame.map(|f| f.width).unwrap_or(0.0)),
            top,
        );

        // 4. Text on a path: re-place the glyphs of artistic text along the curve.
        if let (Some(tp), true) = (req.on_path, artistic) {
            let placed = place_on_path(&lines, tp, req.align, spans);
            return TextLayout {
                path: placed,
                bounds,
                glyphs: glyph_count,
                overflow: false,
                fitted_chars: total_chars,
                baselines: line_baselines,
                lines: line_boxes,
            };
        }

        TextLayout {
            path,
            bounds,
            glyphs: glyph_count,
            overflow,
            fitted_chars,
            baselines: line_baselines,
            lines: line_boxes,
        }
    }
}

fn line_metrics(glyphs: &[Glyph], metrics: &[Metrics], fallback: Metrics) -> Metrics {
    let mut m = fallback;
    for g in glyphs {
        let gm = metrics[g.span];
        m.ascent = m.ascent.max(gm.ascent);
        m.descent = m.descent.max(gm.descent);
        m.line_height = m.line_height.max(gm.line_height);
    }
    m
}

/// Width of a line's glyphs with tab expansion; returns (width, spaces).
fn line_width(glyphs: &[Glyph], para: &ParagraphStyle, start: f64) -> (f64, usize) {
    let mut x = start;
    let mut spaces = 0;
    for g in glyphs {
        if g.ch == '\t' {
            x = next_tab(x, &para.tabs);
        } else {
            if g.ch == ' ' {
                spaces += 1;
            }
            x += g.advance;
        }
    }
    (x - start, spaces)
}

fn next_tab(x: f64, tabs: &[f64]) -> f64 {
    if tabs.is_empty() {
        let step = 12.7;
        return (x / step).floor() * step + step;
    }
    tabs.iter()
        .cloned()
        .find(|t| *t > x + 1e-6)
        .unwrap_or(x + 12.7)
}

/// Greedy line breaking at spaces, with optional hyphenation and a fallback
/// to breaking inside words that do not fit alone.
fn break_lines(
    glyphs: &[Glyph],
    avail_first: f64,
    avail_rest: f64,
    para: &ParagraphStyle,
    metrics: &[Metrics],
    pi: usize,
    para_start: usize,
) -> Vec<Line> {
    let mut lines = Vec::new();
    if glyphs.is_empty() {
        lines.push(Line {
            glyphs: Vec::new(),
            hyphen: None,
            metrics: metrics[0],
            last_in_para: true,
            first_in_para: true,
            para: pi,
            start_char: para_start,
        });
        return lines;
    }
    let start_of = |gs: &[Glyph]| gs.first().map(|g| g.char_idx).unwrap_or(para_start);
    // Words: runs separated by spaces; the space belongs to the preceding word.
    let mut words: Vec<Vec<Glyph>> = Vec::new();
    let mut cur: Vec<Glyph> = Vec::new();
    for g in glyphs {
        let is_sep = g.ch == ' ' || g.ch == '\t';
        cur.push(g.clone());
        if is_sep {
            words.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    let width_of = |gs: &[Glyph]| -> f64 {
        gs.iter()
            .map(|g| if g.ch == '\t' { 12.7 } else { g.advance })
            .sum()
    };
    let trimmed = |gs: &[Glyph]| -> f64 {
        let mut w = width_of(gs);
        if let Some(l) = gs.last() {
            if l.ch == ' ' {
                w -= l.advance;
            }
        }
        w
    };

    let mut line: Vec<Glyph> = Vec::new();
    let mut first = true;
    let mut i = 0;
    while i < words.len() {
        let avail = if first { avail_first } else { avail_rest };
        let word = words[i].clone();
        let fits = trimmed(&[line.as_slice(), word.as_slice()].concat()) <= avail;
        if fits {
            line.extend(word.iter().cloned());
            i += 1;
            continue;
        }
        // Try hyphenating the word.
        let mut placed = false;
        if para.hyphenate || line.is_empty() {
            let text: String = word.iter().map(|g| g.ch).collect();
            let points = if para.hyphenate {
                hyphen::break_points(text.trim_end())
            } else {
                // Forced break inside an over-long word: any position.
                (1..word.len()).collect()
            };
            let mut best: Option<usize> = None;
            for p in points {
                if p == 0 || p >= word.len() {
                    continue;
                }
                let head = &word[..p];
                let hy = head
                    .last()
                    .and_then(|g| g.hyphen_after.as_ref())
                    .map(|h| h.advance)
                    .unwrap_or(0.0);
                let w = trimmed(&[line.as_slice(), head].concat())
                    + if para.hyphenate { hy } else { 0.0 };
                if w <= avail {
                    best = Some(p);
                } else {
                    break;
                }
            }
            if let Some(p) = best {
                let head = word[..p].to_vec();
                let hyphen = if para.hyphenate {
                    head.last().and_then(|g| g.hyphen_after.clone())
                } else {
                    None
                };
                line.extend(head);
                lines.push(Line {
                    start_char: start_of(&line),
                    glyphs: std::mem::take(&mut line),
                    hyphen,
                    metrics: Metrics::default(),
                    last_in_para: false,
                    first_in_para: first,
                    para: pi,
                });
                first = false;
                let tail = word[p..].to_vec();
                // Replace the word with its tail and retry.
                let mut rest = vec![tail];
                rest.extend(words[i + 1..].iter().cloned());
                words.truncate(i);
                words.extend(rest);
                placed = true;
            }
        }
        if placed {
            continue;
        }
        if line.is_empty() {
            // Nothing fits at all; put the word anyway to make progress.
            line.extend(word.iter().cloned());
            i += 1;
        }
        lines.push(Line {
            start_char: start_of(&line),
            glyphs: std::mem::take(&mut line),
            hyphen: None,
            metrics: Metrics::default(),
            last_in_para: false,
            first_in_para: first,
            para: pi,
        });
        first = false;
    }
    if line.is_empty() && !lines.is_empty() {
        // The last word went out as its own line: no empty line after it.
        if let Some(l) = lines.last_mut() {
            l.last_in_para = true;
        }
    } else {
        lines.push(Line {
            start_char: start_of(&line),
            glyphs: line,
            hyphen: None,
            metrics: Metrics::default(),
            last_in_para: true,
            first_in_para: first,
            para: pi,
        });
    }
    // Trailing spaces do not count toward alignment: strip them.
    for l in lines.iter_mut() {
        while l.glyphs.last().map(|g| g.ch == ' ').unwrap_or(false) {
            l.glyphs.pop();
        }
        l.metrics = line_metrics(&l.glyphs, metrics, metrics[0]);
    }
    lines
}

/// Place artistic text along a path: each glyph is centred on its arc
/// length position and rotated to the tangent.
fn place_on_path(
    lines: &[Line],
    tp: &TextOnPath,
    align: TextAlign,
    _spans: &[TextSpan],
) -> BezPath {
    let mut out = BezPath::new();
    let Some(line) = lines.first() else {
        return out;
    };
    // Flatten the path into a polyline with cumulative lengths.
    let mut pts: Vec<Point> = Vec::new();
    kurbo::flatten(tp.path.elements().iter().copied(), 0.05, |el| match el {
        tracedraw_core::geometry::PathEl::MoveTo(p) => pts.push(p),
        tracedraw_core::geometry::PathEl::LineTo(p) => pts.push(p),
        _ => {}
    });
    if pts.len() < 2 {
        return out;
    }
    let mut cum = vec![0.0f64];
    for w in pts.windows(2) {
        cum.push(cum.last().copied().unwrap_or(0.0) + (w[1] - w[0]).hypot());
    }
    let total = *cum.last().unwrap_or(&0.0);
    let text_w: f64 = line.glyphs.iter().map(|g| g.advance).sum();
    let start = tp.offset
        + match align {
            TextAlign::Center => (total - text_w) / 2.0,
            TextAlign::Right => total - text_w,
            _ => 0.0,
        };
    let sample = |s: f64| -> (Point, Vec2) {
        let s = s.clamp(0.0, total);
        let i = cum
            .partition_point(|c| *c <= s)
            .saturating_sub(1)
            .min(pts.len() - 2);
        let seg = pts[i + 1] - pts[i];
        let len = seg.hypot().max(1e-9);
        let t = ((s - cum[i]) / len).clamp(0.0, 1.0);
        (pts[i] + seg * t, seg / len)
    };
    let mut pen = start;
    for g in &line.glyphs {
        let mid = pen + g.advance / 2.0;
        let (p, tan) = sample(mid);
        let mut normal = Vec2::new(-tan.y, tan.x);
        let mut angle = tan.y.atan2(tan.x);
        let mut dist = tp.distance;
        if tp.mirror {
            normal = -normal;
            angle += std::f64::consts::PI;
            dist = -dist;
        }
        let _ = dist;
        let place = Affine::translate(p.to_vec2() + normal * tp.distance)
            * Affine::rotate(angle)
            * Affine::translate((-g.advance / 2.0 + g.x_offset, g.y_offset));
        let gp = place * g.path.as_ref().clone();
        out.extend(gp.elements().iter().copied());
        pen += g.advance;
    }
    out
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
    use tracedraw_core::geometry::Size;

    fn span(text: &str) -> TextSpan {
        TextSpan {
            text: text.into(),
            font_family: "DejaVu Sans".into(),
            size_pt: 12.0,
            bold: false,
            italic: false,
            tracking_pct: 0.0,
            baseline_shift_pt: 0.0,
            underline: false,
            strikethrough: false,
            fill: None,
            features: Vec::new(),
        }
    }

    fn has_fonts() -> bool {
        !fonts().families().is_empty()
    }

    #[test]
    fn shapes_some_text_if_any_font_exists() {
        let f = fonts();
        let layout = f.outline(&[span("Ab")]);
        if !has_fonts() {
            assert_eq!(layout.glyphs, 0);
        } else {
            assert_eq!(layout.glyphs, 2);
            assert!(layout.bounds.width() > 1.0);
            assert!(!layout.path.elements().is_empty());
        }
    }

    #[test]
    fn paragraph_wraps_to_frame_width() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("one two three four five six seven eight nine ten");
        let para = ParagraphStyle::default();
        let one_line = f.outline(&[s.clone()]);
        let wrapped = f.layout(&TextRequest {
            spans: &[s],
            frame: Some(Size::new(one_line.bounds.width() / 2.5, 100.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        assert!(wrapped.bounds.height() > one_line.bounds.height() * 2.0);
        assert!(!wrapped.overflow);
    }

    #[test]
    fn tracking_widens_text() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let a = f.outline(&[span("Hello")]).bounds.width();
        let mut t = span("Hello");
        t.tracking_pct = 50.0;
        let b = f.outline(&[t]).bounds.width();
        assert!(b > a * 1.5);
    }

    #[test]
    fn caret_geometry_follows_the_characters_and_lines() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let l = f.outline(&[span("ab\ncd")]);
        assert_eq!(l.lines.len(), 2);
        assert_eq!((l.lines[0].start_char, l.lines[0].end_char), (0, 2));
        assert_eq!((l.lines[1].start_char, l.lines[1].end_char), (3, 5));
        let c0 = l.caret(0).unwrap();
        let c1 = l.caret(1).unwrap();
        let c2 = l.caret(2).unwrap();
        let c3 = l.caret(3).unwrap();
        let c5 = l.caret(5).unwrap();
        assert!(c1.0 > c0.0 && c2.0 > c1.0, "carets advance along the line");
        assert!(c3.1 < c0.1, "the second line is below the first");
        assert!((c3.0 - c0.0).abs() < 1e-6, "line starts share x");
        assert!(c5.0 > c3.0);
        assert_eq!(l.line_of(2), Some(0));
        assert_eq!(l.line_of(3), Some(1));
        // Clicking before the first character of the second line, and
        // after the last of the first.
        assert_eq!(l.hit_char(Point::new(c3.0 - 1.0, c3.1)), Some(3));
        assert_eq!(l.hit_char(Point::new(c2.0 + 50.0, c0.1)), Some(2));
        // A caret past the end sits after the last character.
        assert_eq!(l.caret(99), Some(c5));
    }

    #[test]
    fn overflow_is_reported_and_fit_to_frame_fixes_it() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("a lot of text that will not fit in a tiny frame at all");
        let para = ParagraphStyle::default();
        let fit = ParagraphStyle {
            fit_to_frame: true,
            ..ParagraphStyle::default()
        };
        fn req<'a>(s: &'a TextSpan, p: &'a ParagraphStyle) -> TextRequest<'a> {
            TextRequest {
                spans: std::slice::from_ref(s),
                frame: Some(Size::new(20.0, 5.0)),
                align: TextAlign::Left,
                para: p,
                on_path: None,
            }
        }
        assert!(f.layout(&req(&s, &para)).overflow);
        assert!(!f.layout(&req(&s, &fit)).overflow);
    }

    #[test]
    fn justify_fills_the_width() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("aa bb cc dd ee ff gg hh ii jj kk ll mm nn oo pp");
        let para = ParagraphStyle::default();
        let l = f.layout(&TextRequest {
            spans: &[s],
            frame: Some(Size::new(30.0, 100.0)),
            align: TextAlign::Justify,
            para: &para,
            on_path: None,
        });
        assert!((l.bounds.width() - 30.0).abs() < 0.01);
    }

    #[test]
    fn baseline_grid_snaps_every_baseline_to_the_pitch() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("one\ntwo\nthree");
        let para = ParagraphStyle {
            baseline_grid_mm: 5.0,
            ..ParagraphStyle::default()
        };
        let l = f.layout(&TextRequest {
            spans: std::slice::from_ref(&s),
            frame: Some(Size::new(100.0, 100.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        assert_eq!(l.baselines.len(), 3);
        let mut prev = 0.0;
        for (_, y) in &l.baselines {
            assert!(*y < prev, "baselines go down: {:?}", l.baselines);
            let k = -y / 5.0;
            assert!(
                (k - k.round()).abs() < 1e-6 && k.round() >= 1.0,
                "baseline {y} is not on the 5 mm grid: {:?}",
                l.baselines
            );
            prev = *y;
        }
        assert!(!l.overflow);
        assert_eq!(l.fitted_chars, 13);
        // Without the grid the same text has baselines off the grid
        // (12 pt lines are about 5.9 mm apart, the first one one ascent down).
        let free = ParagraphStyle::default();
        let l2 = f.layout(&TextRequest {
            spans: std::slice::from_ref(&s),
            frame: Some(Size::new(100.0, 100.0)),
            align: TextAlign::Left,
            para: &free,
            on_path: None,
        });
        assert_eq!(l2.baselines.len(), 3);
        assert!(l2.baselines.iter().any(|(_, y)| {
            let k = -y / 5.0;
            (k - k.round()).abs() > 1e-3
        }));
        // Snapping never moves a baseline up.
        for (a, b) in l.baselines.iter().zip(&l2.baselines) {
            assert!(a.1 <= b.1 + 1e-9, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn fitted_chars_counts_what_the_frame_holds() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("one\ntwo\nthree");
        let para = ParagraphStyle::default();
        // Artistic text: everything fits by definition.
        assert_eq!(f.outline(&[s.clone()]).fitted_chars, 13);
        // A frame tall enough for one 12 pt line only.
        let l = f.layout(&TextRequest {
            spans: std::slice::from_ref(&s),
            frame: Some(Size::new(60.0, 6.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        assert!(l.overflow);
        assert_eq!(
            l.fitted_chars, 4,
            "\"one\\n\" fits, \"two\" starts the overflow"
        );
        let (head, tail) = tracedraw_core::split_spans_at(&[s.clone()], l.fitted_chars);
        assert_eq!(head[0].text, "one\n");
        assert_eq!(tail[0].text, "two\nthree");
        // A frame that holds everything reports the full count.
        let l = f.layout(&TextRequest {
            spans: std::slice::from_ref(&s),
            frame: Some(Size::new(60.0, 100.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        assert!(!l.overflow);
        assert_eq!(l.fitted_chars, 13);
    }

    #[test]
    fn fitted_chars_splits_inside_a_wrapped_paragraph_and_across_spans() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let a = span("aaaa bbbb ");
        let mut b = span("cccc dddd eeee");
        b.underline = true;
        let spans = vec![a, b];
        let para = ParagraphStyle::default();
        let one_line = f.outline(&[span("aaaa bbbb")]);
        // Width for one word pair per line, height for two lines.
        let l = f.layout(&TextRequest {
            spans: &spans,
            frame: Some(Size::new(one_line.bounds.width() + 1.0, 11.0)),
            align: TextAlign::Left,
            para: &para,
            on_path: None,
        });
        assert!(l.overflow);
        assert_eq!(
            l.fitted_chars, 20,
            "two lines of two words: 'aaaa bbbb ' + 'cccc dddd ' = 20 chars"
        );
        let (head, tail) = tracedraw_core::split_spans_at(&spans, l.fitted_chars);
        assert_eq!(head.len(), 2);
        assert_eq!(head[1].text, "cccc dddd ");
        assert!(head[1].underline);
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].text, "eeee");
        assert!(tail[0].underline);
    }

    #[test]
    fn text_on_path_follows_the_curve() {
        if !has_fonts() {
            return;
        }
        let f = fonts();
        let s = span("curve");
        let para = ParagraphStyle::default();
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((0.0, 50.0));
        let tp = TextOnPath {
            path: p,
            offset: 0.0,
            distance: 0.0,
            mirror: false,
        };
        let l = f.layout(&TextRequest {
            spans: &[s],
            frame: None,
            align: TextAlign::Left,
            para: &para,
            on_path: Some(&tp),
        });
        let b = l.path.bounding_box();
        assert!(
            b.height() > b.width(),
            "vertical path should give tall text: {b:?}"
        );
    }
}
