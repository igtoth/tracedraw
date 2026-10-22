//! The engine owns a document and its history.
//!
//! History is snapshot-based for now: before each command the document is
//! cloned. Vector documents are small, so this is cheap and, more
//! importantly, always correct. A delta-based history can replace it later
//! without changing this API.

use crate::command::Command;
use crate::id::ShapeId;
use crate::{Document, Error, Result};

#[derive(Debug, Clone)]
struct HistoryEntry {
    label: &'static str,
    before: Document,
    after: Document,
    /// The command Edit > Repeat repeats for this step, when it is one.
    repeatable: Option<Command>,
}

#[derive(Debug)]
pub struct Engine {
    doc: Document,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    /// Counts applied commands, so views can cache per revision.
    revision: u64,
    /// Revision at the last save, to know whether the document is dirty.
    saved_revision: u64,
    /// Revision right after the last recorded step, so a side effect can
    /// join that step only while nothing happened since.
    step_revision: u64,
    max_history: usize,
    /// Generator of object identities ([`crate::document::Uid`]).
    uid_state: u64,
}

/// A seed no other engine in this run has: std's per-run random hash keys
/// over a counter.
fn fresh_seed() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    h.finish()
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new(Document::default())
    }
}

impl Engine {
    pub fn new(doc: Document) -> Self {
        let mut e = Engine {
            doc,
            undo: Vec::new(),
            redo: Vec::new(),
            revision: 0,
            saved_revision: 0,
            step_revision: u64::MAX,
            max_history: 200,
            uid_state: fresh_seed(),
        };
        e.doc.assign_uids(&mut e.uid_state);
        e.doc.refresh_text_flow();
        e
    }

    pub fn document(&self) -> &Document {
        &self.doc
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    pub fn mark_saved(&mut self) {
        self.saved_revision = self.revision;
    }

    /// Unsaved changes whatever the history holds (a drawing recovered
    /// from a backup), until the next save.
    pub fn mark_unsaved(&mut self) {
        self.saved_revision = u64::MAX;
    }

    /// Replace the document (open file, new document). Clears history.
    pub fn replace(&mut self, doc: Document) {
        self.doc = doc;
        self.doc.assign_uids(&mut self.uid_state);
        self.doc.refresh_text_flow();
        self.undo.clear();
        self.redo.clear();
        self.revision += 1;
        self.saved_revision = self.revision;
        self.step_revision = u64::MAX;
    }

    /// Apply a command and record it. A failing command leaves the document
    /// and history untouched.
    pub fn run(&mut self, cmd: &Command) -> Result<()> {
        self.run_with_label(cmd, cmd.label())
    }

    /// Like [`run`](Self::run) with a custom history label.
    pub fn run_with_label(&mut self, cmd: &Command, label: &'static str) -> Result<()> {
        if cmd.outside_history() {
            // No step, no redo loss, no revision: views and the unsaved
            // marker do not change for it.
            return cmd.apply(&mut self.doc);
        }
        let before = self.doc.clone();
        cmd.apply(&mut self.doc)?;
        self.doc.assign_uids(&mut self.uid_state);
        self.doc.refresh_text_flow();
        self.undo.push(HistoryEntry {
            label,
            before,
            after: self.doc.clone(),
            repeatable: if cmd.is_repeatable() {
                Some(cmd.clone())
            } else {
                None
            },
        });
        self.finish_step();
        Ok(())
    }

    fn finish_step(&mut self) {
        while self.undo.len() > self.max_history {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.revision += 1;
        self.step_revision = self.revision;
    }

    /// Apply several commands as one undo step.
    pub fn run_batch(&mut self, label: &'static str, cmds: &[Command]) -> Result<()> {
        let before = self.doc.clone();
        let mut doc = self.doc.clone();
        for c in cmds {
            c.apply(&mut doc)?;
        }
        doc.assign_uids(&mut self.uid_state);
        doc.refresh_text_flow();
        self.doc = doc;
        self.undo.push(HistoryEntry {
            label,
            before,
            after: self.doc.clone(),
            repeatable: None,
        });
        self.finish_step();
        Ok(())
    }

    /// Apply a command as part of the step just recorded (a side effect of
    /// it, such as the document palette remembering a colour that was just
    /// applied), so one Undo takes both back. Recorded as a step of its own
    /// when anything else happened since.
    pub fn amend(&mut self, cmd: &Command) -> Result<()> {
        if cmd.outside_history() {
            return cmd.apply(&mut self.doc);
        }
        if self.revision != self.step_revision {
            return self.run(cmd);
        }
        let Some(last) = self.undo.last_mut() else {
            return self.run(cmd);
        };
        let mut doc = self.doc.clone();
        cmd.apply(&mut doc)?;
        doc.assign_uids(&mut self.uid_state);
        doc.refresh_text_flow();
        self.doc = doc;
        last.after = self.doc.clone();
        self.revision += 1;
        self.step_revision = self.revision;
        Ok(())
    }

    /// The drawing is as the step just recorded left it: no undo, redo or
    /// change outside the history since. A side effect added now with
    /// [`amend`](Self::amend) joins that step.
    pub fn at_recorded_step(&self) -> bool {
        self.revision == self.step_revision && !self.undo.is_empty()
    }

    /// [`Engine::amend`] for several commands: they join the step just
    /// recorded, or make a step of their own named `label` when anything
    /// else happened since. A failing command leaves the drawing as it was.
    pub fn amend_batch(&mut self, label: &'static str, cmds: &[Command]) -> Result<()> {
        if cmds.is_empty() {
            return Ok(());
        }
        if self.revision != self.step_revision || self.undo.is_empty() {
            return self.run_batch(label, cmds);
        }
        let mut doc = self.doc.clone();
        for c in cmds {
            c.apply(&mut doc)?;
        }
        doc.assign_uids(&mut self.uid_state);
        self.doc = doc;
        if let Some(last) = self.undo.last_mut() {
            last.after = self.doc.clone();
        }
        self.redo.clear();
        self.revision += 1;
        self.step_revision = self.revision;
        Ok(())
    }

    /// Mint a fresh shape id. Ids never rewind on undo.
    pub fn new_shape_id(&mut self) -> ShapeId {
        self.doc.ids_mut().shape()
    }

    fn restore(&mut self, snapshot: &Document) {
        let ids = self.doc.ids().clone();
        let tracker = std::mem::take(&mut self.doc.time_tracker);
        self.doc = snapshot.clone();
        self.doc.set_ids(ids);
        self.doc.time_tracker = tracker;
        self.revision += 1;
    }

    /// Mark the drawing changed without an undo step (an edit of state kept
    /// outside the history, which should still be saved).
    pub fn mark_changed(&mut self) {
        self.revision += 1;
    }

    pub fn undo(&mut self) -> Result<&'static str> {
        let e = self.undo.pop().ok_or(Error::NothingToUndo)?;
        self.restore(&e.before);
        let label = e.label;
        self.redo.push(e);
        Ok(label)
    }

    pub fn redo(&mut self) -> Result<&'static str> {
        let e = self.redo.pop().ok_or(Error::NothingToRedo)?;
        self.restore(&e.after);
        let label = e.label;
        self.undo.push(e);
        Ok(label)
    }

    /// Labels of the undo stack (oldest first) and the redo stack.
    pub fn history_labels(&self) -> (Vec<&'static str>, Vec<&'static str>) {
        (
            self.undo.iter().map(|e| e.label).collect(),
            self.redo.iter().map(|e| e.label).collect(),
        )
    }

    /// How many steps Undo keeps (Options > General > Undo levels); older
    /// steps are dropped at once when the limit shrinks.
    pub fn set_max_history(&mut self, n: usize) {
        self.max_history = n.max(1);
        while self.undo.len() > self.max_history {
            self.undo.remove(0);
        }
    }

    pub fn max_history(&self) -> usize {
        self.max_history
    }

    pub fn undo_label(&self) -> Option<&'static str> {
        self.undo.last().map(|e| e.label)
    }

    pub fn redo_label(&self) -> Option<&'static str> {
        self.redo.last().map(|e| e.label)
    }

    /// The command Edit > Repeat repeats: the newest repeatable step still
    /// applied. Undone steps leave the undo stack, so after an undo the step
    /// before it becomes the one; redoing the step makes it the newest again.
    pub fn repeatable(&self) -> Option<&Command> {
        self.undo.iter().rev().find_map(|e| e.repeatable.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Shape, ShapeKind};
    use crate::geometry::Rect;

    #[test]
    fn objects_keep_their_identity_and_copies_get_their_own() {
        use crate::document::{Shape, ShapeKind};
        use crate::geometry::Rect;
        let mut eng = Engine::default();
        let layer = eng.document().pages[0].layers[0].id;
        let id = eng.new_shape_id();
        let shape = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        eng.run(&Command::AddShape { layer, shape }).unwrap();
        let uid = eng.document().layer(layer).unwrap().shapes[0]
            .uid
            .expect("given one");
        // Saved and opened again: the same.
        let back = Document::from_json(&eng.document().to_json().unwrap()).unwrap();
        let again = Engine::new(back);
        assert_eq!(
            again.document().layer(layer).unwrap().shapes[0].uid,
            Some(uid)
        );
        // A copy carrying it gets its own; the original keeps it.
        let mut copy = eng.document().layer(layer).unwrap().shapes[0].clone();
        copy.id = eng.new_shape_id();
        eng.run(&Command::AddShape { layer, shape: copy }).unwrap();
        let shapes = &eng.document().layer(layer).unwrap().shapes;
        assert_eq!(shapes[0].uid, Some(uid));
        assert!(shapes[1].uid.is_some() && shapes[1].uid != Some(uid));
        // Undo and redo bring back the same identities.
        let second = shapes[1].uid;
        eng.undo().unwrap();
        eng.redo().unwrap();
        assert_eq!(eng.document().layer(layer).unwrap().shapes[1].uid, second);
        // Written as 32 hex digits.
        let json = serde_json::to_string(&uid).unwrap();
        assert_eq!(json.len(), 34);
        assert_eq!(
            serde_json::from_str::<crate::document::Uid>(&json).unwrap(),
            uid
        );
        assert!(serde_json::from_str::<crate::document::Uid>("\"zz\"").is_err());
    }

    #[test]
    fn a_drawing_marked_unsaved_stays_so_until_saved() {
        let mut eng = Engine::default();
        assert!(!eng.is_dirty());
        eng.mark_unsaved();
        assert!(eng.is_dirty());
        eng.mark_saved();
        assert!(!eng.is_dirty());
    }

    #[test]
    fn undo_redo_round_trip() {
        let mut eng = Engine::default();
        let layer = eng.document().pages[0].layers[0].id;
        let id = eng.new_shape_id();
        let shape = Shape::new(
            id,
            ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                arc: None,
            },
        );
        eng.run(&Command::AddShape { layer, shape }).unwrap();
        assert_eq!(eng.document().layer(layer).unwrap().shapes.len(), 1);
        assert!(eng.is_dirty());
        assert_eq!(eng.undo().unwrap(), "Create Object");
        assert_eq!(eng.document().layer(layer).unwrap().shapes.len(), 0);
        eng.redo().unwrap();
        assert_eq!(eng.document().layer(layer).unwrap().shapes.len(), 1);
        assert!(matches!(eng.redo(), Err(Error::NothingToRedo)));
    }

    #[test]
    fn amending_joins_the_last_step_only_right_after_it() {
        let mut eng = Engine::default();
        let layer = eng.document().pages[0].layers[0].id;
        let id = eng.new_shape_id();
        let shape = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        eng.run(&Command::AddShape { layer, shape }).unwrap();
        let red = crate::Color::rgb8(255, 0, 0);
        eng.amend(&Command::SetDocumentPalette { colors: vec![red] })
            .unwrap();
        assert_eq!(eng.history_labels().0.len(), 1);
        eng.undo().unwrap();
        assert!(eng.document().palette.is_empty());
        assert_eq!(eng.document().layer(layer).unwrap().shapes.len(), 0);
        // After an undo there is no step to join: a step of its own.
        eng.amend(&Command::SetDocumentPalette { colors: vec![red] })
            .unwrap();
        assert_eq!(eng.history_labels().0, vec!["Document Palette"]);
    }

    #[test]
    fn batches_respect_the_undo_levels() {
        let mut eng = Engine::default();
        eng.set_max_history(2);
        for i in 0..4 {
            let colors = vec![crate::Color::rgb8(i, 0, 0)];
            eng.run_batch("Palette", &[Command::SetDocumentPalette { colors }])
                .unwrap();
        }
        assert_eq!(eng.history_labels().0.len(), 2);
    }

    #[test]
    fn time_tracker_stays_out_of_the_history() {
        let mut eng = Engine::default();
        let layer = eng.document().pages[0].layers[0].id;
        let id = eng.new_shape_id();
        let shape = Shape::new(
            id,
            ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                arc: None,
            },
        );
        eng.run(&Command::AddShape { layer, shape }).unwrap();
        let rev = eng.revision();
        let mut tracker = crate::TimeTracker::default();
        tracker.tasks.push(crate::TrackedTask {
            name: "Layout".into(),
            seconds: 90.0,
            ..Default::default()
        });
        tracker.active = Some(0);
        eng.run(&Command::SetTimeTracker {
            tracker: tracker.clone(),
        })
        .unwrap();
        eng.amend(&Command::SetTimeTracker {
            tracker: tracker.clone(),
        })
        .unwrap();
        // No step and no new revision for it.
        assert_eq!(eng.history_labels().0, vec!["Create Object"]);
        assert_eq!(eng.revision(), rev);
        // Undo and redo of the drawing leave the records alone.
        eng.undo().unwrap();
        assert_eq!(eng.document().time_tracker, tracker);
        eng.redo().unwrap();
        assert_eq!(eng.document().time_tracker, tracker);
        // An explicit edit still marks the drawing changed, without a step.
        eng.mark_saved();
        assert!(!eng.is_dirty());
        eng.mark_changed();
        assert!(eng.is_dirty());
        assert_eq!(eng.history_labels().0.len(), 1);
    }

    #[test]
    fn recording_moves_the_start_and_finish() {
        let mut t = crate::TimeTracker::default();
        // Without an active task nothing is recorded.
        t.record(10.0, 1000);
        assert!(t.tasks.is_empty() && t.active_task().is_none());
        t.tasks.push(crate::TrackedTask {
            name: "A".into(),
            ..Default::default()
        });
        t.active = Some(0);
        t.record(30.0, 1000);
        t.record(f64::NAN, 2000);
        t.record(-5.0, 2000);
        t.record(15.0, 1100);
        let a = &t.tasks[0];
        assert_eq!(a.seconds, 45.0);
        assert_eq!(a.start, Some(970));
        assert_eq!(a.finish, Some(1100));
        assert_eq!(t.total_seconds(), 45.0);
        // A stale index is no active task.
        t.active = Some(3);
        assert!(t.active_task().is_none());
        t.record(5.0, 1200);
        assert_eq!(t.total_seconds(), 45.0);
    }

    #[test]
    fn failed_command_leaves_history_alone() {
        let mut eng = Engine::default();
        let rev = eng.revision();
        assert!(eng
            .run(&Command::DeleteShapes {
                shapes: vec![crate::ShapeId(42)]
            })
            .is_err());
        assert_eq!(eng.revision(), rev);
        assert!(eng.undo_label().is_none());
    }

    #[test]
    fn repeatable_follows_the_undo_and_redo_stacks() {
        use crate::geometry::{Affine, Vec2};
        let mut eng = Engine::default();
        let page = eng.document().pages[0].id;
        let layer = eng.document().pages[0].layers[0].id;
        let id = eng.new_shape_id();
        let shape = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        eng.run(&Command::AddShape { layer, shape }).unwrap();
        assert!(eng.repeatable().is_none());
        let a = Command::TransformShapes {
            shapes: vec![id],
            transform: Affine::translate(Vec2::new(10.0, 0.0)),
        };
        eng.run(&a).unwrap();
        let b = Command::SetFill {
            shapes: vec![id],
            fill: crate::Fill::None,
        };
        eng.run(&b).unwrap();
        assert_eq!(eng.repeatable(), Some(&b));
        eng.undo().unwrap();
        assert_eq!(eng.repeatable(), Some(&a));
        eng.redo().unwrap();
        assert_eq!(eng.repeatable(), Some(&b));
        // A non-repeatable step on top does not hide the last repeatable one.
        eng.run(&Command::AddLayer {
            page,
            name: "Another".into(),
        })
        .unwrap();
        assert_eq!(eng.repeatable(), Some(&b));
        eng.undo().unwrap();
        eng.undo().unwrap();
        assert_eq!(eng.repeatable(), Some(&a));
        eng.undo().unwrap();
        assert!(eng.repeatable().is_none());
    }
}
