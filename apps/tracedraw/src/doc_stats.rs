//! What File > Document Properties reports about a drawing: its objects,
//! curves, text, bitmaps, styles, effects, fills, outlines, colour models
//! and symbols. See docs/behavior/document-properties.md.

use std::collections::{BTreeMap, BTreeSet};
use tracedraw_core::document::ShapeKind;
use tracedraw_core::geometry::PathEl;
use tracedraw_core::{Color, Document, Fill, Shape};

/// The counts, gathered over every page and the master layers, inside
/// groups and clip frames.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocStats {
    pub layers: usize,
    /// Objects other than groups and clip frames.
    pub objects: usize,
    /// Nodes of every curve.
    pub points: usize,
    /// The most nodes and subpaths one curve has.
    pub max_points: usize,
    pub max_subpaths: usize,
    pub curves: usize,
    pub rectangles: usize,
    pub ellipses: usize,
    pub polygons: usize,
    pub groups: usize,
    pub tables: usize,
    pub artistic_texts: usize,
    pub paragraph_texts: usize,
    pub characters: usize,
    pub words: usize,
    pub fonts: BTreeSet<String>,
    pub bitmaps: usize,
    pub bitmap_pixels: u64,
    /// The largest bitmap, in pixels (width, height).
    pub largest_bitmap: Option<(u32, u32)>,
    pub object_styles: usize,
    pub color_styles: usize,
    /// Effect names (live effects by their key, bitmap effects by id) and
    /// how many objects carry each.
    pub effects: BTreeMap<String, usize>,
    /// Fills by kind: none, uniform, fountain, pattern, texture, mesh.
    pub fills: [usize; 7],
    /// Outlines: none, solid, dashed.
    pub outlines: [usize; 3],
    /// Objects using each colour model in their fill or outline.
    pub color_models: BTreeMap<&'static str, usize>,
    pub symbols: usize,
    pub symbol_instances: usize,
}

/// Nodes and subpaths of a path.
fn path_counts(path: &tracedraw_core::BezPath) -> (usize, usize) {
    let mut nodes = 0;
    let mut subpaths = 0;
    for el in path.elements() {
        match el {
            PathEl::MoveTo(_) => {
                nodes += 1;
                subpaths += 1;
            }
            PathEl::LineTo(_) | PathEl::QuadTo(..) | PathEl::CurveTo(..) => nodes += 1,
            PathEl::ClosePath => {}
        }
    }
    (nodes, subpaths)
}

fn fill_colors(fill: &Fill, out: &mut Vec<Color>) {
    match fill {
        Fill::Solid(c) => out.push(*c),
        Fill::Fountain(g) => out.extend(g.stops.iter().map(|s| s.color)),
        Fill::Mesh(m) => out.extend(m.nodes.iter().map(|n| n.color)),
        _ => {}
    }
}

impl DocStats {
    /// The nodes of a shape's outline count as points too.
    fn outline_points(&mut self, s: &Shape) {
        let (n, sp) = path_counts(&s.local_path());
        self.points += n;
        self.max_points = self.max_points.max(n);
        self.max_subpaths = self.max_subpaths.max(sp);
    }

    fn shape(&mut self, s: &Shape) {
        match &s.kind {
            ShapeKind::Group { children } => {
                self.groups += 1;
                children.iter().for_each(|c| self.shape(c));
                return;
            }
            ShapeKind::ClipFrame { frame, contents } => {
                self.shape(frame);
                contents.iter().for_each(|c| self.shape(c));
                return;
            }
            ShapeKind::Path { path, .. } => {
                self.curves += 1;
                let (n, sp) = path_counts(path);
                self.points += n;
                self.max_points = self.max_points.max(n);
                self.max_subpaths = self.max_subpaths.max(sp);
            }
            ShapeKind::Rect { .. } => {
                self.rectangles += 1;
                self.outline_points(s);
            }
            ShapeKind::Ellipse { .. } => {
                self.ellipses += 1;
                self.outline_points(s);
            }
            ShapeKind::Polygon { .. } => {
                self.polygons += 1;
                self.outline_points(s);
            }
            ShapeKind::Table(_) => self.tables += 1,
            ShapeKind::SymbolInstance { .. } => self.symbol_instances += 1,
            ShapeKind::Text { spans, frame, .. } => {
                if frame.is_some() {
                    self.paragraph_texts += 1;
                } else {
                    self.artistic_texts += 1;
                }
                let text = tracedraw_core::document::spans_text(spans);
                self.characters += text.chars().filter(|c| *c != '\n').count();
                self.words += text.split_whitespace().count();
                for sp in spans {
                    self.fonts.insert(sp.font_family.clone());
                }
            }
            ShapeKind::Bitmap {
                width_px,
                height_px,
                fx,
                ..
            } => {
                self.bitmaps += 1;
                self.bitmap_pixels += *width_px as u64 * *height_px as u64;
                let area = |(w, h): (u32, u32)| w as u64 * h as u64;
                if self
                    .largest_bitmap
                    .is_none_or(|l| area(l) < area((*width_px, *height_px)))
                {
                    self.largest_bitmap = Some((*width_px, *height_px));
                }
                if let Some(fx) = fx {
                    for e in &fx.effects {
                        *self.effects.entry(format!("fx.{}", e.id)).or_default() += 1;
                    }
                }
            }
        }
        self.objects += 1;
        for e in &s.effects {
            *self
                .effects
                .entry(crate::ui::effects_docker::effect_key(e).to_string())
                .or_default() += 1;
        }
        if let Some(fx) = &s.raster_fx {
            for e in &fx.effects {
                *self.effects.entry(format!("fx.{}", e.id)).or_default() += 1;
            }
        }
        let kind = match &s.fill {
            Fill::None => 0,
            Fill::Solid(_) => 1,
            Fill::Fountain(_) => 2,
            Fill::Pattern(_) => 3,
            Fill::Texture(_) => 4,
            Fill::Mesh(_) => 5,
            Fill::Procedural(_) => 6,
        };
        self.fills[kind] += 1;
        let mut colors = Vec::new();
        fill_colors(&s.fill, &mut colors);
        match &s.stroke {
            None => self.outlines[0] += 1,
            Some(st) => {
                colors.push(st.color);
                self.outlines[if st.dash.is_empty() { 1 } else { 2 }] += 1;
            }
        }
        let models: BTreeSet<&'static str> = colors.iter().map(|c| c.model_name()).collect();
        for m in models {
            *self.color_models.entry(m).or_default() += 1;
        }
    }
}

/// Gather the statistics of `doc`.
pub fn stats(doc: &Document) -> DocStats {
    let mut st = DocStats {
        object_styles: doc.object_styles.len(),
        color_styles: doc.color_styles.len(),
        symbols: doc.symbols.len(),
        ..Default::default()
    };
    for layer in doc.all_layers() {
        st.layers += 1;
        layer.shapes.iter().for_each(|s| st.shape(s));
    }
    st
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{BezPath, Point, Rect};

    #[test]
    fn counts_curves_text_fills_and_outlines_through_groups() {
        let mut doc = Document::new("t", tracedraw_core::geometry::Size::new(100.0, 100.0));
        let mut ids = tracedraw_core::id::IdSource::default();
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((10.0, 0.0));
        path.line_to((10.0, 10.0));
        path.close_path();
        path.move_to((2.0, 2.0));
        path.line_to((4.0, 2.0));
        let mut curve = Shape::new(ids.shape(), ShapeKind::Path { path, closed: true });
        curve.fill = Fill::Solid(Color::cmyk_pct(0.0, 100.0, 0.0, 0.0));
        curve.stroke = None;
        let mut rect = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 5.0, 5.0),
                radius: 0.0,
                corners: None,
            },
        );
        rect.fill = Fill::None;
        rect.stroke = Some(tracedraw_core::Stroke::hairline(Color::rgb8(0, 0, 0)));
        let mut text = Shape::new(
            ids.shape(),
            ShapeKind::Text {
                spans: vec![tracedraw_core::TextSpan::new("two words", "Selawik", 12.0)],
                origin: Point::new(0.0, 0.0),
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
        );
        text.fill = Fill::Solid(Color::rgb8(0, 0, 0));
        text.stroke = None;
        let group = Shape::new(
            ids.shape(),
            ShapeKind::Group {
                children: vec![curve, rect],
            },
        );
        doc.pages[0].layers[0].shapes = vec![group, text];
        let st = stats(&doc);
        assert_eq!(st.objects, 3);
        assert_eq!(st.groups, 1);
        // The curve's 5 nodes and the rectangle's 4.
        assert_eq!(
            (st.curves, st.points, st.max_points, st.max_subpaths),
            (1, 9, 5, 2)
        );
        assert_eq!(st.rectangles, 1);
        assert_eq!((st.artistic_texts, st.characters, st.words), (1, 9, 2));
        assert!(st.fonts.contains("Selawik"));
        assert_eq!(st.fills, [1, 2, 0, 0, 0, 0, 0]);
        assert_eq!(st.outlines, [2, 1, 0]);
        assert_eq!(st.color_models.get("CMYK"), Some(&1));
        assert_eq!(st.color_models.get("RGB"), Some(&2));
        assert_eq!(st.bitmaps, 0);
        assert!(st.effects.is_empty());
    }
}
