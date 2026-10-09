//! Linked paragraph text frames (Text > Paragraph Text Frame > Link / Unlink).
//!
//! A chain is a list of paragraph frames; the text of the whole chain lives
//! in the frames in order and overflow flows from one frame into the next.
//! Links are kept in the object data of each frame (`text.next`,
//! `text.prev`, shape ids), so they survive save and load without a model
//! change. Re-flow runs after typing, linking and resizing a frame.

use crate::app::App;
use crate::i18n::tr;
use tracedraw_core::{
    document::text_outline::TextRequest,
    document::{ShapeKind, TextSpan},
    Command, ShapeId,
};

const NEXT: &str = "text.next";
const PREV: &str = "text.prev";

fn data_id(data: &[(String, String)], key: &str) -> Option<ShapeId> {
    data.iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.parse::<u64>().ok())
        .map(ShapeId)
}

fn with_key(
    mut data: Vec<(String, String)>,
    key: &str,
    value: Option<ShapeId>,
) -> Vec<(String, String)> {
    data.retain(|(k, _)| k != key);
    if let Some(v) = value {
        data.push((key.to_string(), v.raw().to_string()));
    }
    data
}

impl App {
    fn frame_data(&self, id: ShapeId) -> Option<Vec<(String, String)>> {
        let s = self.doc().find_shape(id)?;
        matches!(s.kind, ShapeKind::Text { frame: Some(_), .. }).then(|| s.data.clone())
    }

    /// Next frame in the chain, if any and still present.
    pub fn next_frame(&self, id: ShapeId) -> Option<ShapeId> {
        let next = data_id(&self.frame_data(id)?, NEXT)?;
        self.frame_data(next).map(|_| next)
    }

    fn prev_frame(&self, id: ShapeId) -> Option<ShapeId> {
        let prev = data_id(&self.frame_data(id)?, PREV)?;
        self.frame_data(prev).map(|_| prev)
    }

    /// First frame of the chain containing `id`.
    pub fn chain_head(&self, id: ShapeId) -> ShapeId {
        let mut cur = id;
        let mut guard = 0;
        while let Some(p) = self.prev_frame(cur) {
            cur = p;
            guard += 1;
            if guard > 1000 {
                break;
            }
        }
        cur
    }

    /// All frames of the chain starting at its head, in flow order.
    pub fn chain_of(&self, id: ShapeId) -> Vec<ShapeId> {
        let mut out = vec![self.chain_head(id)];
        let mut guard = 0;
        while let Some(n) = self.next_frame(*out.last().unwrap_or(&id)) {
            if out.contains(&n) {
                break;
            }
            out.push(n);
            guard += 1;
            if guard > 1000 {
                break;
            }
        }
        out
    }

    pub fn is_linked_frame(&self, id: ShapeId) -> bool {
        self.next_frame(id).is_some() || self.prev_frame(id).is_some()
    }

    /// Link the two selected paragraph frames: the first selected flows into
    /// the second. The second frame's own text is appended to the chain.
    pub fn link_text_frames(&mut self) {
        let [a, b] = self.selection.as_slice() else {
            self.status = tr("status.link_needs_two_frames");
            return;
        };
        let (a, b) = (*a, *b);
        let (Some(da), Some(db)) = (self.frame_data(a), self.frame_data(b)) else {
            self.status = tr("status.link_needs_two_frames");
            return;
        };
        if self.chain_of(a).contains(&b)
            || self.next_frame(a).is_some()
            || self.prev_frame(b).is_some()
        {
            self.status = tr("status.link_already");
            return;
        }
        let cmds = vec![
            Command::SetObjectData {
                shape: a,
                data: with_key(da, NEXT, Some(b)),
            },
            Command::SetObjectData {
                shape: b,
                data: with_key(db, PREV, Some(a)),
            },
        ];
        let _ = self.engine.run_batch("Link Frames", &cmds);
        self.reflow_chain(a);
        self.status = tr("status.frames_linked");
    }

    /// Unlink the selected frames from their neighbours; each keeps the text
    /// it currently shows.
    pub fn unlink_text_frames(&mut self) {
        let mut cmds = Vec::new();
        for id in self.selection.clone() {
            let Some(d) = self.frame_data(id) else {
                continue;
            };
            let next = data_id(&d, NEXT);
            let prev = data_id(&d, PREV);
            if next.is_none() && prev.is_none() {
                continue;
            }
            cmds.push(Command::SetObjectData {
                shape: id,
                data: with_key(with_key(d, NEXT, None), PREV, None),
            });
            if let Some(n) = next {
                if let Some(dn) = self.frame_data(n) {
                    cmds.push(Command::SetObjectData {
                        shape: n,
                        data: with_key(dn, PREV, None),
                    });
                }
            }
            if let Some(p) = prev {
                if let Some(dp) = self.frame_data(p) {
                    cmds.push(Command::SetObjectData {
                        shape: p,
                        data: with_key(dp, NEXT, None),
                    });
                }
            }
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Unlink Frames", &cmds);
        }
    }

    /// Distribute the chain's text over its frames: each frame keeps what
    /// fits, the rest flows on; the last frame takes whatever is left.
    pub fn reflow_chain(&mut self, any: ShapeId) {
        let chain = self.chain_of(any);
        if chain.len() < 2 {
            return;
        }
        // Gather the whole text in chain order.
        let mut all: Vec<TextSpan> = Vec::new();
        let mut kinds = Vec::new();
        for id in &chain {
            let Some(s) = self.doc().find_shape(*id) else {
                return;
            };
            if let ShapeKind::Text { spans, .. } = &s.kind {
                all.extend(spans.iter().cloned());
            }
            kinds.push(s.kind.clone());
        }
        let fonts = tracedraw_text::fonts();
        let mut cmds = Vec::new();
        let mut rest = all;
        let n = chain.len();
        for (i, (id, kind)) in chain.iter().zip(kinds).enumerate() {
            let ShapeKind::Text {
                origin,
                frame,
                align,
                para,
                on_path,
                spans: old,
            } = kind
            else {
                continue;
            };
            let mine = if i + 1 == n {
                std::mem::take(&mut rest)
            } else {
                let layout = fonts.layout(&TextRequest {
                    spans: &rest,
                    frame,
                    align,
                    para: &para,
                    on_path: on_path.as_ref(),
                });
                let (head, tail) = tracedraw_core::split_spans_at(&rest, layout.fitted_chars);
                rest = tail;
                head
            };
            if mine == old {
                continue;
            }
            cmds.push(Command::SetShapeKind {
                shape: *id,
                kind: ShapeKind::Text {
                    spans: mine,
                    origin,
                    frame,
                    align,
                    para,
                    on_path,
                },
            });
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Text Flow", &cmds);
        }
    }

    /// Re-flow every chain that touches the given shapes (after a resize).
    pub fn reflow_chains_of(&mut self, ids: &[ShapeId]) {
        let mut done: Vec<ShapeId> = Vec::new();
        for id in ids {
            if !self.is_linked_frame(*id) {
                continue;
            }
            let head = self.chain_head(*id);
            if done.contains(&head) {
                continue;
            }
            done.push(head);
            self.reflow_chain(head);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Point, Size};
    use tracedraw_core::TextAlign;

    fn frame(app: &mut App, text: &str, w: f64, h: f64) -> ShapeId {
        let kind = ShapeKind::Text {
            spans: vec![TextSpan::new(text, "DejaVu Sans", 12.0)],
            origin: Point::new(10.0, 10.0),
            frame: Some(Size::new(w, h)),
            align: TextAlign::Left,
            para: Default::default(),
            on_path: None,
        };
        app.new_shape(kind).unwrap()
    }

    #[test]
    fn linking_flows_overflow_into_the_second_frame() {
        tracedraw_text::install();
        let mut app = App::headless();
        let long = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty";
        let a = frame(&mut app, long, 30.0, 8.0);
        let b = frame(&mut app, "", 60.0, 60.0);
        app.select(vec![a, b]);
        app.link_text_frames();
        assert_eq!(app.next_frame(a), Some(b));
        let text_of = |app: &App, id: ShapeId| -> String {
            match &app.doc().find_shape(id).unwrap().kind {
                ShapeKind::Text { spans, .. } => spans.iter().map(|s| s.text.as_str()).collect(),
                _ => String::new(),
            }
        };
        let ta = text_of(&app, a);
        let tb = text_of(&app, b);
        assert!(!ta.is_empty() && !tb.is_empty(), "a={ta:?} b={tb:?}");
        assert_eq!(format!("{ta}{tb}"), long);
        // Unlinking keeps the split text in place.
        app.select(vec![a]);
        app.unlink_text_frames();
        assert_eq!(app.next_frame(a), None);
        assert_eq!(text_of(&app, a), ta);
    }
}
