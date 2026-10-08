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
    max_history: usize,
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new(Document::default())
    }
}

impl Engine {
    pub fn new(doc: Document) -> Self {
        Engine {
            doc,
            undo: Vec::new(),
            redo: Vec::new(),
            revision: 0,
            saved_revision: 0,
            max_history: 200,
        }
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

    /// Replace the document (open file, new document). Clears history.
    pub fn replace(&mut self, doc: Document) {
        self.doc = doc;
        self.undo.clear();
        self.redo.clear();
        self.revision += 1;
        self.saved_revision = self.revision;
    }

    /// Apply a command and record it. A failing command leaves the document
    /// and history untouched.
    pub fn run(&mut self, cmd: &Command) -> Result<()> {
        self.run_with_label(cmd, cmd.label())
    }

    /// Like [`run`](Self::run) with a custom history label.
    pub fn run_with_label(&mut self, cmd: &Command, label: &'static str) -> Result<()> {
        let before = self.doc.clone();
        cmd.apply(&mut self.doc)?;
        self.undo.push(HistoryEntry {
            label,
            before,
            after: self.doc.clone(),
        });
        if self.undo.len() > self.max_history {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.revision += 1;
        Ok(())
    }

    /// Apply several commands as one undo step.
    pub fn run_batch(&mut self, label: &'static str, cmds: &[Command]) -> Result<()> {
        let before = self.doc.clone();
        let mut doc = self.doc.clone();
        for c in cmds {
            c.apply(&mut doc)?;
        }
        self.doc = doc;
        self.undo.push(HistoryEntry {
            label,
            before,
            after: self.doc.clone(),
        });
        self.redo.clear();
        self.revision += 1;
        Ok(())
    }

    /// Mint a fresh shape id. Ids never rewind on undo.
    pub fn new_shape_id(&mut self) -> ShapeId {
        self.doc.ids_mut().shape()
    }

    fn restore(&mut self, snapshot: &Document) {
        let ids = self.doc.ids().clone();
        self.doc = snapshot.clone();
        self.doc.set_ids(ids);
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

    pub fn undo_label(&self) -> Option<&'static str> {
        self.undo.last().map(|e| e.label)
    }

    pub fn redo_label(&self) -> Option<&'static str> {
        self.redo.last().map(|e| e.label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Shape, ShapeKind};
    use crate::geometry::Rect;

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
}
