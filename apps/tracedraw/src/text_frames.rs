//! Paragraph text frames with the Pick tool: scaling a paragraph text with
//! the handles (or the property bar's size and scale) changes its frame and
//! the text flows anew in it, the letters keeping their size; turning,
//! slanting, mirroring and moving act on the whole text as before. Frames
//! show as dashed lines (Text > Show Text Frames). See
//! docs/behavior/text.md.

use crate::app::App;
use tracedraw_core::geometry::{Affine, Point, Rect, Size};
use tracedraw_core::{Command, Shape, ShapeId, ShapeKind};

/// A paragraph text frame as shown on screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShownFrame {
    pub id: tracedraw_core::ShapeId,
    /// The frame flows into another (its tab shows a link).
    pub linked: bool,
    /// Another frame flows into this one (its top tab shows a link).
    pub receives: bool,
    /// Page-space corners, the frame's lower left first.
    pub corners: [Point; 4],
    /// The text is selected or being edited.
    pub chosen: bool,
    /// The text is being edited (its frame is drawn black).
    pub editing: bool,
    /// The text does not fit (drawn red, with a tab below).
    pub overflow: bool,
}

/// The factors of an axis-aligned scale by positive amounts (with any
/// move), else `None`.
fn axis_scale(a: Affine) -> Option<(f64, f64)> {
    let [sx, b, c, sy, e, f] = a.as_coeffs();
    let finite = [sx, b, c, sy, e, f].iter().all(|v| v.is_finite());
    (finite && b.abs() < 1e-12 && c.abs() < 1e-12 && sx > 1e-9 && sy > 1e-9).then_some((sx, sy))
}

/// The page-space box of a paragraph text's frame.
fn frame_box(s: &Shape) -> Option<Rect> {
    let c = s.text_frame_corners()?;
    Some(
        c.iter()
            .skip(1)
            .fold(Rect::from_points(c[0], c[0]), |r, p| r.union_pt(*p)),
    )
}

/// The text kind of `s` (a paragraph text whose transform is an axis-
/// aligned scale) with its frame made the image of its page-space frame
/// under `t`.
fn resized_kind(s: &Shape, t: Affine) -> Option<ShapeKind> {
    let (tx, ty) = axis_scale(s.transform)?;
    let ShapeKind::Text {
        spans,
        align,
        para,
        on_path: None,
        frame: Some(_),
        ..
    } = &s.kind
    else {
        return None;
    };
    let old = frame_box(s)?;
    let a = t * Point::new(old.x0, old.y0);
    let b = t * Point::new(old.x1, old.y1);
    let new = Rect::from_points(a, b);
    let origin = s.transform.inverse() * Point::new(new.x0, new.y0);
    Some(ShapeKind::Text {
        spans: spans.clone(),
        origin,
        frame: Some(Size::new(
            (new.width() / tx).max(0.5),
            (new.height() / ty).max(0.5),
        )),
        align: *align,
        para: para.clone(),
        on_path: None,
    })
}

impl App {
    /// `t` (page space) resizes the frame of `id` instead of scaling it: a
    /// paragraph text lying on a layer, neither turned nor slanted, and `t`
    /// an axis-aligned scale by positive amounts that is not just a move.
    pub fn resizes_text_frame(&self, id: ShapeId, t: Affine) -> bool {
        let Some((sx, sy)) = axis_scale(t) else {
            return false;
        };
        if (sx - 1.0).abs() < 1e-9 && (sy - 1.0).abs() < 1e-9 {
            return false;
        }
        self.doc()
            .shape(id)
            .ok()
            .is_some_and(|(_, s)| resized_kind(s, t).is_some())
    }

    /// Apply `t` to the selection as one undo step: paragraph texts that
    /// take it as a new frame get it so, the rest are transformed.
    /// Returns false when no paragraph frame was involved (the caller
    /// transforms as usual).
    pub(crate) fn transform_with_frames(&mut self, shapes: &[ShapeId], t: Affine) -> bool {
        let frames: Vec<ShapeId> = shapes
            .iter()
            .copied()
            .filter(|id| self.resizes_text_frame(*id, t))
            .collect();
        if frames.is_empty() {
            return false;
        }
        let mut cmds = Vec::new();
        let others: Vec<ShapeId> = shapes
            .iter()
            .copied()
            .filter(|id| !frames.contains(id))
            .collect();
        if !others.is_empty() {
            cmds.push(Command::TransformShapes {
                shapes: others,
                transform: t,
            });
        }
        for id in &frames {
            let Ok((_, s)) = self.doc().shape(*id) else {
                continue;
            };
            if let Some(kind) = resized_kind(s, t) {
                cmds.push(Command::SetShapeKind { shape: *id, kind });
            }
            // A frame's symmetry goes with it, as an object's does in a
            // transform (not the one being edited).
            if self.symmetry_edit != Some(*id)
                && s.effect_stack()
                    .iter()
                    .any(|(e, _)| matches!(e, tracedraw_core::live::Effect::Symmetry { .. }))
            {
                let mut moved = s.clone();
                moved.carry_effects(t);
                cmds.push(Command::SetEffectStack {
                    shape: *id,
                    effects: moved.effects,
                    parked: moved.parked_effects,
                });
            }
        }
        if let Err(e) = self.run_batch_keeping_lines("Transform", &cmds) {
            self.status = e.to_string();
        }
        true
    }

    /// Paragraph text `s` does not fit its frame (the last frame of a
    /// linked chain included); cached per document revision.
    pub fn text_overflows(&self, s: &Shape) -> bool {
        let rev = self.engine.revision();
        let mut cache = self.overflow_cache.borrow_mut();
        if cache.0 != rev {
            *cache = (rev, Default::default());
        }
        if let Some(v) = cache.1.get(&s.id) {
            return *v;
        }
        let v = match &s.kind {
            ShapeKind::Text {
                spans,
                frame: Some(f),
                align,
                para,
                on_path: None,
                ..
            } => {
                let req = tracedraw_core::document::text_outline::TextRequest {
                    spans,
                    frame: Some(*f),
                    align: *align,
                    para,
                    on_path: None,
                };
                tracedraw_text::fonts().layout(&req).overflow
            }
            _ => false,
        };
        cache.1.insert(s.id, v);
        v
    }

    /// The paragraph text frames to show, in page space, each with whether
    /// it is chosen (selected or edited): every frame on the page with
    /// Text > Show Text Frames, else those of the selected texts and the
    /// text being edited; the selected frames being dragged take `preview`
    /// (resized for a scale, else moved, turned or slanted with the
    /// selection).
    pub fn shown_text_frames(&self, preview: Option<Affine>) -> Vec<ShownFrame> {
        let doc = self.doc();
        let Ok(page) = doc.page(self.page) else {
            return Vec::new();
        };
        let editing = self.text_edit.as_ref().map(|te| te.shape);
        let mut out = Vec::new();
        for s in page
            .layers
            .iter()
            .filter(|l| l.visible)
            .flat_map(|l| &l.shapes)
            .filter(|s| s.visible)
        {
            // Only text frames: asking every object whether it is selected
            // cost a search of the selection each (#367).
            let Some(mut c) = s.text_frame_corners() else {
                continue;
            };
            let selected = self.selection.contains(&s.id);
            let chosen = selected || editing == Some(s.id);
            if !(self.settings.show_text_frames || chosen) {
                continue;
            }
            if let Some(t) = preview.filter(|t| selected && self.resizes_text_frame(s.id, *t)) {
                if let Some(kind) = resized_kind(s, t) {
                    let mut moved = s.clone();
                    moved.kind = kind;
                    c = moved.text_frame_corners().unwrap_or(c);
                }
            } else if let Some(t) = preview.filter(|_| selected) {
                c = c.map(|p| t * p);
            }
            out.push(ShownFrame {
                id: s.id,
                linked: self.next_frame(s.id).is_some(),
                receives: self.prev_frame(s.id).is_some(),
                corners: c,
                chosen,
                editing: editing == Some(s.id),
                overflow: self.text_overflows(s),
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paragraph(app: &mut App) -> ShapeId {
        // The real text engine, so the words wrap in the frame.
        tracedraw_text::install();
        app.start_text(Point::new(20.0, 20.0), Some(Size::new(60.0, 30.0)));
        for c in "Some words to flow in the frame".chars() {
            app.text_insert(&c.to_string());
        }
        let id = app.selection[0];
        app.finish_text();
        app.select(vec![id]);
        id
    }

    fn frame_of(app: &App, id: ShapeId) -> (Point, Size) {
        match app.doc().shape(id).map(|(_, s)| s.kind.clone()) {
            Ok(ShapeKind::Text {
                origin,
                frame: Some(f),
                ..
            }) => (origin, f),
            other => panic!("not paragraph text: {other:?}"),
        }
    }

    #[test]
    fn scaling_paragraph_text_resizes_its_frame_and_keeps_the_letters() {
        let mut app = App::headless();
        let id = paragraph(&mut app);
        let size_before = match app.doc().shape(id).map(|(_, s)| s.kind.clone()) {
            Ok(ShapeKind::Text { spans, .. }) => spans[0].size_pt,
            _ => panic!("text"),
        };
        // The frame counts in the bounds, empty space and all.
        let b = app.selection_bounds().expect("bounds");
        assert!(
            (b.width() - 60.0).abs() < 1e-6 && (b.height() - 30.0).abs() < 1e-6,
            "{b:?}"
        );
        // Twice as wide from the left edge: the frame doubles, the letters
        // stay, the transform stays the identity.
        let t = Affine::translate((20.0, 0.0))
            * Affine::scale_non_uniform(2.0, 1.0)
            * Affine::translate((-20.0, 0.0));
        app.transform_selection(t);
        let (origin, f) = frame_of(&app, id);
        assert!((f.width - 120.0).abs() < 1e-6 && (f.height - 30.0).abs() < 1e-6);
        assert!((origin.x - 20.0).abs() < 1e-6 && (origin.y - 20.0).abs() < 1e-6);
        let s = app.doc().shape(id).expect("shape").1.clone();
        assert_eq!(s.transform, Affine::IDENTITY);
        match &s.kind {
            ShapeKind::Text { spans, .. } => assert_eq!(spans[0].size_pt, size_before),
            _ => panic!("text"),
        }
        // One undo step brings the old frame back.
        app.undo();
        assert!((frame_of(&app, id).1.width - 60.0).abs() < 1e-6);
        // A turn still turns the whole text; a move moves it.
        let turn = Affine::rotate(0.3);
        app.select(vec![id]);
        app.transform_selection(turn);
        let s = app.doc().shape(id).expect("shape").1.clone();
        assert_eq!(s.transform, turn);
        // Once turned, a scale scales the text as a whole.
        assert!(!app.resizes_text_frame(id, Affine::scale(2.0)));
    }

    #[test]
    fn a_frame_too_small_for_its_text_overflows() {
        let mut app = App::headless();
        let id = paragraph(&mut app);
        let s = app.doc().shape(id).expect("shape").1.clone();
        assert!(!app.text_overflows(&s));
        // A tenth of the height: the words no longer fit.
        app.transform_selection(Affine::scale_non_uniform(1.0, 0.1));
        let s = app.doc().shape(id).expect("shape").1.clone();
        assert!(app.text_overflows(&s));
        assert!(app.shown_text_frames(None)[0].overflow);
    }

    #[test]
    fn the_frame_being_edited_is_drawn_black_and_a_selected_one_blue() {
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        tracedraw_text::install();
        app.start_text(Point::new(20.0, 20.0), Some(Size::new(60.0, 30.0)));
        app.text_insert("Words");
        let f = app.shown_text_frames(None);
        assert!(f[0].editing && f[0].chosen);
        assert_eq!(crate::canvas::text_frame_color(&f[0]), egui::Color32::BLACK);
        let id = app.selection[0];
        app.finish_text();
        app.select(vec![id]);
        let f = app.shown_text_frames(None);
        assert!(!f[0].editing && f[0].chosen);
        assert_eq!(
            crate::canvas::text_frame_color(&f[0]),
            crate::theme::Tokens::SELECTION
        );
        // The selected characters' shade is a see-through grey.
        let [r, g, b, a] = crate::canvas::TEXT_SELECTION.to_array();
        assert!(r == g && g == b && a > 0 && a < 255);
        // Plain to see on the white page: a mid grey there (#632).
        let on_white = 255.0 * (1.0 - a as f32 / 255.0) + r as f32;
        assert!(on_white < 170.0, "{on_white}");
    }

    #[test]
    fn frames_show_dashed_and_follow_a_scale_being_dragged() {
        let mut app = App::headless();
        let id = paragraph(&mut app);
        assert!(app.settings.show_text_frames);
        assert_eq!(app.shown_text_frames(None).len(), 1);
        let t = Affine::scale_non_uniform(1.5, 1.0);
        let c = app.shown_text_frames(Some(t));
        assert!((c[0].corners[1].x - 120.0).abs() < 1e-6, "{c:?}");
        assert!(c[0].chosen && !c[0].overflow);
        // Off: only the selected text's frame.
        app.settings.show_text_frames = false;
        assert_eq!(app.shown_text_frames(None).len(), 1);
        app.select(Vec::new());
        assert!(app.shown_text_frames(None).is_empty());
        assert!(app.doc().shape(id).is_ok());
    }
}
