//! Several open drawings, one tab each.
//!
//! The active drawing's state lives in the `App` fields that the rest of
//! the editor already uses (`engine`, `page`, `view`, `selection`, `file`
//! ...). Every other open drawing waits in a [`DocSlot`]. Switching tabs
//! swaps the fields with the slot, so no other code needs to know how
//! many drawings are open. The slot at `active_doc` is a placeholder
//! while that drawing is active.
//!
//! With no drawing open (only the Welcome Screen) `docs` is empty and the
//! fields hold an empty placeholder document that is never shown.
//!
//! Window > New Window gives a drawing another window (another tab): one
//! slot per window, and the slots of one drawing share its `drawing` id.
//! A slot's fields come in two parts. The window part (page, view,
//! selection ...) belongs to each window. The drawing part (the engine
//! with the document and its undo history, the file ...) exists once: in
//! the `App` fields while one of the drawing's windows is active, else
//! in exactly one of its slots, the one marked `holds`. The other
//! windows' slots keep an empty placeholder there, so a drawing is never
//! copied and an edit made in one window is what every other window of
//! the drawing shows.

use crate::app::{App, Drag, PageNumberSettings};
use crate::view::View;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tracedraw_core::{Document, Engine, PageId, ShapeId};

/// Source of the ids that tell open drawings apart.
static NEXT_DRAWING: AtomicU64 = AtomicU64::new(1);

/// Everything that belongs to one window of an open drawing.
pub struct DocSlot {
    /// Which drawing this window shows (shared by all its windows).
    pub drawing: u64,
    /// The window's number among its drawing's windows (from 1), shown
    /// as "name:n" while the drawing has more than one.
    pub window: u32,
    /// This slot keeps the drawing part for all of the drawing's windows
    /// (none of them is the active one).
    pub holds: bool,
    pub engine: Engine,
    pub page: Option<PageId>,
    pub view: View,
    pub selection: Vec<ShapeId>,
    pub file: Option<PathBuf>,
    pub opened_from: Option<PathBuf>,
    pub remote: Option<crate::github::Remote>,
    pub cdr_version: Option<u16>,
    pub cdr_unsaved_version: Option<u16>,
    pub tdraw_version: Option<u32>,
    pub fit_pending: bool,
    pub page_numbers: PageNumberSettings,
    pub missing_fonts: Vec<String>,
    /// Drawing units of this drawing (the property bar's Units).
    pub units: crate::app::Units,
    /// Centres of rotation moved in this drawing.
    pub pivots: crate::app::Pivots,
    /// The layer made active in the Objects docker.
    pub picked_layer: Option<tracedraw_core::LayerId>,
    /// The drawing's rendered page, kept while another drawing is active
    /// (its window shows it when windows are tiled or cascaded).
    pub raster: crate::raster::Raster,
    /// Where the drawing's canvas began on screen when it was last drawn
    /// (see `App::anchor_view`).
    pub view_anchor: Option<egui::Pos2>,
}

impl Default for DocSlot {
    /// A window of a drawing of its own.
    fn default() -> Self {
        DocSlot {
            drawing: NEXT_DRAWING.fetch_add(1, Ordering::Relaxed),
            window: 1,
            holds: false,
            engine: Engine::default(),
            page: None,
            view: View::default(),
            selection: Vec::new(),
            file: None,
            opened_from: None,
            remote: None,
            cdr_version: None,
            cdr_unsaved_version: None,
            tdraw_version: None,
            fit_pending: false,
            page_numbers: PageNumberSettings::default(),
            missing_fonts: Vec::new(),
            units: Default::default(),
            pivots: Default::default(),
            picked_layer: None,
            raster: Default::default(),
            view_anchor: None,
        }
    }
}

impl App {
    /// True when at least one drawing is open (not only the Welcome Screen).
    pub fn has_document(&self) -> bool {
        !self.docs.is_empty()
    }

    /// Swap the window part of the active fields with slot `i`.
    fn swap_window(&mut self, i: usize) {
        let Some(slot) = self.docs.get_mut(i) else {
            return;
        };
        let page = slot.page.unwrap_or(self.page);
        slot.page = Some(self.page);
        self.page = page;
        std::mem::swap(&mut self.view, &mut slot.view);
        std::mem::swap(&mut self.selection, &mut slot.selection);
        std::mem::swap(&mut self.fit_pending, &mut slot.fit_pending);
        std::mem::swap(&mut self.picked_layer, &mut slot.picked_layer);
        std::mem::swap(self.raster.get_mut(), &mut slot.raster);
        std::mem::swap(&mut self.view_anchor, &mut slot.view_anchor);
        // Measured in another window.
        *self.visual_cache.get_mut() = (u64::MAX, Default::default());
    }

    /// Swap the drawing part of the active fields with slot `i`.
    fn swap_drawing(&mut self, i: usize) {
        let Some(slot) = self.docs.get_mut(i) else {
            return;
        };
        std::mem::swap(&mut self.engine, &mut slot.engine);
        std::mem::swap(&mut self.file, &mut slot.file);
        std::mem::swap(&mut self.opened_from, &mut slot.opened_from);
        std::mem::swap(&mut self.remote, &mut slot.remote);
        std::mem::swap(&mut self.cdr_version, &mut slot.cdr_version);
        std::mem::swap(&mut self.cdr_unsaved_version, &mut slot.cdr_unsaved_version);
        std::mem::swap(&mut self.tdraw_version, &mut slot.tdraw_version);
        std::mem::swap(&mut self.page_numbers, &mut slot.page_numbers);
        std::mem::swap(&mut self.missing_fonts, &mut slot.missing_fonts);
        std::mem::swap(&mut self.units, &mut slot.units);
        std::mem::swap(&mut self.pivots, &mut slot.pivots);
        *self.visual_cache.get_mut() = (u64::MAX, Default::default());
    }

    /// The slot keeping the drawing part of window `i`'s drawing, or
    /// `None` when the active fields have it.
    fn holder(&self, i: usize) -> Option<usize> {
        let drawing = self.docs.get(i)?.drawing;
        self.docs
            .iter()
            .position(|d| d.holds && d.drawing == drawing)
    }

    /// Another window of window `i`'s drawing.
    fn twin(&self, i: usize) -> Option<usize> {
        let drawing = self.docs.get(i)?.drawing;
        (0..self.docs.len()).find(|&j| j != i && self.docs[j].drawing == drawing)
    }

    /// Window `i`'s drawing has another window open.
    pub fn has_twin(&self, i: usize) -> bool {
        self.twin(i).is_some()
    }

    /// The window just brought into the fields may refer to a page or to
    /// objects that another window of its drawing removed.
    fn settle_window(&mut self) {
        // A page id from another drawing means nothing here.
        if self.engine.document().page(self.page).is_err() {
            if let Some(p) = self.engine.document().pages.first() {
                self.page = p.id;
            }
        }
        let doc = self.engine.document();
        if self
            .selection
            .iter()
            .any(|id| doc.find_shape(*id).is_none())
        {
            let kept: Vec<ShapeId> = self
                .selection
                .iter()
                .copied()
                .filter(|id| doc.find_shape(*id).is_some())
                .collect();
            self.selection = kept;
        }
    }

    /// Put the active window, and its drawing, in its slot.
    fn park_active(&mut self) {
        let a = self.active_doc;
        self.swap_window(a);
        self.swap_drawing(a);
        if let Some(slot) = self.docs.get_mut(a) {
            slot.holds = true;
        }
    }

    /// Bring parked window `i` into the fields and its drawing with it,
    /// from whichever slot keeps it.
    fn unpark(&mut self, i: usize) {
        self.swap_window(i);
        if let Some(h) = self.holder(i) {
            self.swap_drawing(h);
            self.docs[h].holds = false;
        }
        self.settle_window();
    }

    /// Show parked window `i` in the fields for a moment (to draw it);
    /// [`App::end_peek`] with the returned value puts everything back.
    pub(crate) fn peek(&mut self, i: usize) -> Option<usize> {
        self.swap_window(i);
        let holder = self.holder(i);
        if let Some(h) = holder {
            self.swap_drawing(h);
        }
        self.settle_window();
        holder
    }

    /// Undo [`App::peek`].
    pub(crate) fn end_peek(&mut self, i: usize, holder: Option<usize>) {
        if let Some(h) = holder {
            self.swap_drawing(h);
        }
        self.swap_window(i);
    }

    /// Drop the in-progress interactions of the drawing being left: they
    /// refer to its objects.
    fn end_transient_editing(&mut self) {
        // Time counted for the drawing being left belongs to it.
        self.flush_tracked_time();
        if self.clip_frame_edit.is_some() {
            self.finish_clip_frame_edit();
        }
        // Editing a symmetry ends with the drawing it is in; it stayed on,
        // its message in the status bar over the next drawing (#442).
        if self.symmetry_edit.is_some() {
            self.finish_symmetry_edit();
        }
        self.drag = Drag::None;
        self.curve = None;
        self.curve_steps.clear();
        self.text_edit = None;
        self.dimension_labels_seen.clear();
        self.table_edit = None;
        self.three_point_base = None;
        self.dimension_points.clear();
        self.node_selection.clear();
        self.selected_guides.clear();
        self.guide_rotate = None;
        self.rotate_mode = false;
        self.pending_clip_frame = false;
        self.pending_order = None;
        self.pending_copy_properties = false;
        self.pending_shaping = None;
        self.pending_merge_frame = false;
        self.pending_copy_effect = None;
        self.pending_copy_envelope = false;
        self.envelope_drag = None;
        self.pending_clone_effect = None;
        self.pending_blend_path = false;
        self.pending_text_path = false;
        self.anchor_sel = None;
        self.effect_node_drag = None;
        self.selected_effect_node = None;
        self.lens_synced_to = None;
        self.extrude_drag = None;
        self.distort_drag = None;
        self.context_menu = None;
        self.navigator_tex = None;
        self.repeatables.clear();
        self.flyout_open = None;
        self.raster.borrow_mut().invalidate();
    }

    /// Open `doc` in a new tab after the others and make it active.
    pub fn open_document(&mut self, doc: Document, file: Option<PathBuf>) {
        self.end_transient_editing();
        // A new drawing starts in the units in use.
        let units = self.units;
        if self.has_document() {
            // Park the active drawing in its slot.
            self.park_active();
        }
        self.page = doc.pages.first().map(|p| p.id).unwrap_or(self.page);
        self.add_carried_fonts(&doc.fonts);
        self.engine = Engine::new(doc);
        self.engine
            .set_max_history(self.settings.undo_levels.max(1));
        self.selection.clear();
        self.file = file;
        self.opened_from = None;
        self.remote = None;
        self.cdr_version = None;
        self.cdr_unsaved_version = None;
        self.tdraw_version = None;
        self.view = View::default();
        self.fit_pending = true;
        self.page_numbers = PageNumberSettings::default();
        self.missing_fonts.clear();
        self.units = units;
        self.pivots = Default::default();
        self.picked_layer = None;
        *self.raster.get_mut() = Default::default();
        self.view_anchor = None;
        self.docs.push(DocSlot::default());
        self.active_doc = self.docs.len() - 1;
        self.show_welcome = false;
        // The status bar's message was about the drawing before, such as
        // where it was saved (#1128); who opens a file says so after.
        self.status.clear();
    }

    /// Fonts a drawing carries serve its text (and the font lists) for the
    /// rest of the session.
    pub fn add_carried_fonts(&mut self, fonts: &[tracedraw_core::DocumentFont]) {
        if !fonts.is_empty() && !tracedraw_text::fonts().add_document_fonts(fonts).is_empty() {
            self.sync_fonts();
        }
    }

    /// Catch up with fonts added since the last frame (fetched by the
    /// browser build when text asked for them, or carried by a drawing):
    /// refresh the font lists and the missing-font list, and draw the
    /// page and the text measurements again, since text drawn with a
    /// stand-in may now have its own font.
    pub fn sync_fonts(&mut self) {
        let fonts = tracedraw_text::fonts();
        let generation = fonts.generation();
        if generation == self.font_generation {
            return;
        }
        self.font_generation = generation;
        self.font_families = fonts.available_families();
        self.raster.borrow_mut().invalidate();
        for slot in &mut self.docs {
            slot.raster.invalidate();
        }
        self.navigator_tex = None;
        *self.overflow_cache.borrow_mut() = Default::default();
        if !self.missing_fonts.is_empty() {
            let mut missing = std::mem::take(&mut self.missing_fonts);
            missing.retain(|f| !self.font_installed(f));
            self.missing_fonts = missing;
        }
    }

    /// The tab to show for `path` when an open drawing was read from it or
    /// saved to it (#645): the active window when it shows that drawing,
    /// else the drawing's first window. None in a browser (uploads of one
    /// name are different files).
    pub fn tab_with_file(&self, path: &std::path::Path) -> Option<usize> {
        if crate::files::WEB || !self.has_document() {
            return None;
        }
        let want = crate::files::canonical(path);
        let is = |file: &Option<PathBuf>, from: &Option<PathBuf>| {
            from.iter().any(|p| crate::files::same_path(p, &want))
                || file
                    .iter()
                    .any(|p| crate::files::same_path(&crate::files::canonical(p), &want))
        };
        let active = self.docs.get(self.active_doc)?.drawing;
        if is(&self.file, &self.opened_from) {
            return Some(self.active_doc);
        }
        let d = self
            .docs
            .iter()
            .find(|d| d.holds && d.drawing != active && is(&d.file, &d.opened_from))?
            .drawing;
        self.docs.iter().position(|s| s.drawing == d)
    }

    /// Bring tab `i` to the front.
    pub fn switch_document(&mut self, i: usize) {
        if i >= self.docs.len() {
            return;
        }
        self.show_welcome = false;
        if i == self.active_doc {
            return;
        }
        self.end_transient_editing();
        self.park_active();
        self.unpark(i);
        self.active_doc = i;
        // The status bar's message was about the other drawing (#1128).
        self.status.clear();
    }

    /// Window > New Window: another window (tab) of the active drawing,
    /// after its last window, showing what the active window shows; it
    /// becomes the active window.
    pub fn new_window(&mut self) {
        if !self.has_document() {
            return;
        }
        self.end_transient_editing();
        let a = self.active_doc.min(self.docs.len() - 1);
        self.active_doc = a;
        let drawing = self.docs[a].drawing;
        let mine = |d: &&DocSlot| d.drawing == drawing;
        let window = self
            .docs
            .iter()
            .filter(mine)
            .map(|d| d.window)
            .max()
            .unwrap_or(1)
            + 1;
        let at = self
            .docs
            .iter()
            .rposition(|d| d.drawing == drawing)
            .unwrap_or(a)
            + 1;
        let slot = DocSlot {
            drawing,
            window,
            page: Some(self.page),
            view: self.view,
            selection: self.selection.clone(),
            fit_pending: self.fit_pending,
            picked_layer: self.picked_layer,
            view_anchor: self.view_anchor,
            ..DocSlot::default()
        };
        self.park_active();
        self.docs.insert(at, slot);
        self.unpark(at);
        self.active_doc = at;
        self.show_welcome = false;
    }

    /// Close the active window without asking (the caller saved or the
    /// user chose not to). While the drawing has another window open
    /// only this window goes; else the drawing closes. The tab to its
    /// right becomes active, else the one to its left; with none left the
    /// Welcome Screen shows.
    pub fn close_active_document(&mut self) {
        if !self.has_document() {
            return;
        }
        self.end_transient_editing();
        let i = self.active_doc.min(self.docs.len() - 1);
        if let Some(j) = self.twin(i) {
            // The drawing stays, with its other window.
            self.swap_drawing(j);
            self.docs[j].holds = true;
        }
        let drawing = self.docs.remove(i).drawing;
        // The drawing's other windows are numbered again from 1, in tab
        // order.
        let mut n = 0;
        for slot in self.docs.iter_mut().filter(|d| d.drawing == drawing) {
            n += 1;
            slot.window = n;
        }
        // The fields hold the closed window (and drawing): drop them.
        self.engine = Engine::default();
        self.page = self.engine.document().pages[0].id;
        self.selection.clear();
        self.file = None;
        self.opened_from = None;
        self.cdr_version = None;
        self.cdr_unsaved_version = None;
        self.tdraw_version = None;
        self.view = View::default();
        self.fit_pending = true;
        self.page_numbers = PageNumberSettings::default();
        self.missing_fonts.clear();
        self.pivots = Default::default();
        self.picked_layer = None;
        *self.raster.get_mut() = Default::default();
        self.view_anchor = None;
        // Its message in the status bar goes with it (#1128).
        self.status.clear();
        if self.docs.is_empty() {
            self.active_doc = 0;
            // The Welcome Screen comes back unless its tab was closed:
            // then the workspace stays empty.
            self.show_welcome = !self.settings.welcome_tab_closed;
            return;
        }
        let next = i.min(self.docs.len() - 1);
        self.unpark(next);
        self.active_doc = next;
    }

    /// Tab label of window `i`: its drawing's name, then ":n" (the
    /// window's number) while the drawing has more than one window, and
    /// an asterisk while it has unsaved changes.
    pub fn document_tab_title(&self, i: usize) -> String {
        let title = self.drawing_title(i);
        let (Some(slot), true) = (self.docs.get(i), self.has_twin(i)) else {
            return title;
        };
        match title.strip_suffix('*') {
            Some(name) => format!("{name}:{}*", slot.window),
            None => format!("{title}:{}", slot.window),
        }
    }

    /// The name of window `i`'s drawing (the file or document name), with
    /// an asterisk while it has unsaved changes.
    pub fn drawing_title(&self, i: usize) -> String {
        let Some(drawing) = self.docs.get(i).map(|d| d.drawing) else {
            return String::new();
        };
        let active = self.docs.get(self.active_doc).map(|d| d.drawing);
        let slot = match self.holder(i) {
            Some(h) => &self.docs[h],
            None if active == Some(drawing) => return self.document_title(),
            None => &self.docs[i],
        };
        let name = slot
            .file
            .as_ref()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| slot.engine.document().title.clone());
        if slot.engine.is_dirty() {
            format!("{name}*")
        } else {
            name
        }
    }

    /// Indices of the open drawings with unsaved changes.
    pub fn dirty_documents(&self) -> Vec<usize> {
        (0..self.docs.len())
            .filter(|&i| {
                if i == self.active_doc {
                    self.engine.is_dirty()
                } else {
                    self.docs[i].engine.is_dirty()
                }
            })
            .collect()
    }

    /// Window > Close All: close every drawing, stopping at each one with
    /// unsaved changes to ask (the question continues the run).
    pub fn close_all_documents(&mut self) {
        self.closing_all = true;
        while self.has_document() {
            // One of several windows of a drawing closes without asking.
            if self.engine.is_dirty() && !self.has_twin(self.active_doc) {
                self.show_welcome = false;
                self.dialog = crate::ui::dialogs::Dialog::ConfirmClose;
                return;
            }
            self.close_active_document();
        }
        self.closing_all = false;
        if self.quit_after_closing {
            self.quit_after_closing = false;
            self.quit_now = true;
        }
    }

    /// File > Exit and the window's close button: ask about every drawing
    /// with unsaved changes, then quit.
    pub fn request_exit(&mut self) {
        if self.dirty_documents().is_empty() {
            self.quit_now = true;
            return;
        }
        self.quit_after_closing = true;
        self.close_all_documents();
    }

    /// The Save / Don't Save / Cancel question was answered.
    pub fn after_close_question(&mut self, closed: bool) {
        if closed && self.closing_all {
            self.close_all_documents();
        } else if !closed {
            self.closing_all = false;
            self.quit_after_closing = false;
        }
    }

    /// The next free "Untitled-N" name of this session.
    pub fn next_untitled_name(&mut self) -> String {
        self.untitled_counter += 1;
        crate::i18n::trf(
            "doc.untitled_n",
            &[("n", &self.untitled_counter.to_string())],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::tr;
    use tracedraw_core::document::ShapeKind;
    use tracedraw_core::geometry::{Rect, Size};

    fn add_rect(app: &mut App) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer")
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("tracedraw-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("temp dir");
        d
    }

    /// The status bar does not keep naming a drawing's file once another
    /// drawing is shown: after closing it and making a new one (by the
    /// New Document dialog's way too), or switching tabs (#1128).
    #[test]
    fn the_status_bar_forgets_the_file_of_the_drawing_before() {
        let dir = temp_dir("status-path");
        let mut app = App::headless();
        app.open_document(
            tracedraw_core::Document::new("Art", Size::new(100.0, 100.0)),
            None,
        );
        add_rect(&mut app);
        app.save_to(dir.join("art.tdraw"));
        assert!(app.status.contains("art.tdraw"), "{}", app.status);
        app.close_active_document();
        assert!(app.status.is_empty(), "{}", app.status);
        app.open_path(dir.join("art.tdraw"));
        assert!(app.status.contains("art.tdraw"), "opening says so");
        app.open_document(
            tracedraw_core::Document::new("Untitled-1", Size::new(50.0, 50.0)),
            None,
        );
        assert!(app.status.is_empty(), "{}", app.status);
        app.switch_document(0);
        app.save_to(dir.join("art.tdraw"));
        assert!(app.status.contains("art.tdraw"));
        app.switch_document(1);
        assert!(app.status.is_empty(), "{}", app.status);
    }

    /// Opening a file that is open already shows its tab instead of a
    /// second copy (#645): from another tab and from the Welcome Screen,
    /// by another spelling of its path, unsaved changes kept; a `.cdr`
    /// read once too, until the drawing is saved under another name.
    #[test]
    fn opening_an_open_file_again_shows_its_tab() {
        let dir = temp_dir("open-again");
        let mut app = App::headless();
        app.open_document(
            tracedraw_core::Document::new("Art", Size::new(100.0, 100.0)),
            None,
        );
        add_rect(&mut app);
        let path = dir.join("art.tdraw");
        app.save_to(path.clone());
        app.open_document(
            tracedraw_core::Document::new("Other", Size::new(50.0, 50.0)),
            None,
        );
        let tabs = app.docs.len();
        let art = tabs - 2;
        app.show_welcome = true;
        app.open_path(path.clone());
        assert_eq!(app.docs.len(), tabs, "no second copy");
        assert_eq!(app.active_doc, art);
        assert!(!app.show_welcome);
        assert!(app.status.contains("art.tdraw"), "{}", app.status);
        // Unsaved changes stay; another spelling of the path finds it.
        add_rect(&mut app);
        app.switch_document(tabs - 1);
        app.open_path(dir.join(".").join("art.tdraw"));
        assert_eq!(app.docs.len(), tabs);
        assert_eq!(app.active_doc, art);
        assert!(app.engine.is_dirty());
        assert_eq!(shapes(&app), 2, "not read again");
        // A .cdr file: read once, then shown.
        let cdr = dir.join("plan.cdr");
        let doc = tracedraw_core::Document::new("Plan", Size::new(80.0, 60.0));
        std::fs::write(
            &cdr,
            tracedraw_cdr::write::document_to_cdr_version(&doc, 21),
        )
        .expect("cdr");
        app.open_path(cdr.clone());
        assert_eq!(app.docs.len(), tabs + 1);
        let plan = app.active_doc;
        assert!(app.file.is_none() && app.opened_from.is_some());
        app.switch_document(art);
        app.open_path(cdr.clone());
        assert_eq!(
            app.docs.len(),
            tabs + 1,
            "the .cdr is shown, not read again"
        );
        assert_eq!(app.active_doc, plan);
        // Saved under another name, it is that file's: the .cdr opens anew.
        app.save_to(dir.join("plan.tdraw"));
        app.open_path(cdr);
        assert_eq!(app.docs.len(), tabs + 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A template makes a new untitled drawing every time, even of a file
    /// that is open; the open drawing keeps its file.
    #[test]
    fn templates_always_open_a_new_drawing() {
        let dir = temp_dir("template-again");
        let mut app = App::headless();
        app.open_document(
            tracedraw_core::Document::new("Art", Size::new(100.0, 100.0)),
            None,
        );
        add_rect(&mut app);
        let path = dir.join("art.tdraw");
        app.save_to(path.clone());
        let tabs = app.docs.len();
        app.open_as_template(path.clone());
        assert_eq!(app.docs.len(), tabs + 1);
        assert!(app.file.is_none() && app.opened_from.is_none());
        app.switch_document(tabs - 1);
        assert_eq!(app.file.as_deref(), Some(path.as_path()));
        let tdt = dir.join("card.tdt");
        app.save_to(tdt.clone());
        app.open_path(tdt.clone());
        app.open_path(tdt);
        assert_eq!(app.docs.len(), tabs + 3, "a template opens anew each time");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Editing a symmetry ends when another drawing comes to the front
    /// or a new one opens: its message stayed in the status bar and the
    /// mode stayed on for the next drawing (#442).
    #[test]
    fn another_drawing_ends_the_symmetry_editing() {
        let mut app = App::headless();
        let a = add_rect(&mut app);
        app.select(vec![a]);
        app.create_symmetry();
        app.edit_symmetry();
        assert!(app.symmetry_edit.is_some());
        assert_eq!(app.status, tr("status.editing_symmetry"));
        app.open_document(Document::new("Second", Size::new(100.0, 50.0)), None);
        assert!(app.symmetry_edit.is_none());
        assert_ne!(app.status, tr("status.editing_symmetry"));
        // Back in the first drawing, editing again, then another tab.
        app.switch_document(0);
        app.edit_symmetry();
        assert!(app.symmetry_edit.is_some());
        app.switch_document(1);
        assert!(app.symmetry_edit.is_none());
        assert_ne!(app.status, tr("status.editing_symmetry"));
    }

    #[test]
    fn each_drawing_keeps_its_objects_history_and_selection() {
        let mut app = App::headless();
        assert_eq!(app.docs.len(), 1);
        let a = add_rect(&mut app);
        app.select(vec![a]);
        app.open_document(Document::new("Second", Size::new(100.0, 50.0)), None);
        assert_eq!(app.docs.len(), 2);
        assert_eq!(app.active_doc, 1);
        assert!(app.selection.is_empty());
        assert_eq!(app.page_size().width, 100.0);
        add_rect(&mut app);
        add_rect(&mut app);
        app.switch_document(0);
        assert_eq!(app.selection, vec![a]);
        assert_eq!(app.page_size().width, 210.0);
        let count = |app: &App| {
            app.doc().pages[0]
                .layers
                .iter()
                .map(|l| l.shapes.len())
                .sum::<usize>()
        };
        assert_eq!(count(&app), 1);
        app.undo();
        assert_eq!(count(&app), 0);
        app.switch_document(1);
        assert_eq!(count(&app), 2);
        assert_eq!(app.document_tab_title(0), app.document_tab_title(0));
        assert!(app.document_tab_title(1).ends_with('*'));
    }

    #[test]
    fn closing_moves_to_a_neighbour_then_to_the_welcome_screen() {
        let mut app = App::headless();
        app.open_document(Document::new("B", Size::new(50.0, 50.0)), None);
        app.open_document(Document::new("C", Size::new(60.0, 60.0)), None);
        app.switch_document(1);
        app.close_active_document();
        assert_eq!(app.docs.len(), 2);
        assert_eq!(app.doc().title, "C");
        app.close_active_document();
        app.close_active_document();
        assert!(!app.has_document());
        assert!(app.show_welcome);
        // Closing again with nothing open is harmless.
        app.close_active_document();
        app.switch_document(3);
    }

    #[test]
    fn exit_asks_about_each_unsaved_drawing_then_quits() {
        let mut app = App::headless();
        add_rect(&mut app);
        app.open_document(Document::new("Clean", Size::new(50.0, 50.0)), None);
        app.open_document(Document::new("Dirty", Size::new(50.0, 50.0)), None);
        add_rect(&mut app);
        app.request_exit();
        // The clean drawings closed; the first unsaved one asks.
        assert!(matches!(
            app.dialog,
            crate::ui::dialogs::Dialog::ConfirmClose
        ));
        assert!(!app.quit_now);
        let asked = app.doc().title.clone();
        // Don't Save: on to the next unsaved drawing.
        app.dialog = crate::ui::dialogs::Dialog::None;
        app.close_active_document();
        app.after_close_question(true);
        assert!(matches!(
            app.dialog,
            crate::ui::dialogs::Dialog::ConfirmClose
        ));
        assert_ne!(app.doc().title, asked);
        // Cancel stops quitting.
        app.after_close_question(false);
        assert!(!app.quit_now && !app.closing_all);
        // Asked again, Don't Save on the last one quits.
        app.request_exit();
        app.close_active_document();
        app.after_close_question(true);
        assert!(app.quit_now);
        assert!(!app.has_document());
    }

    fn shapes(app: &App) -> usize {
        app.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.shapes.len())
            .sum::<usize>()
    }

    #[test]
    fn a_new_window_shares_the_drawing_and_its_undo_history() {
        let mut app = App::headless();
        let a = add_rect(&mut app);
        app.open_document(Document::new("Other", Size::new(50.0, 50.0)), None);
        app.switch_document(0);
        app.new_window();
        assert_eq!(app.docs.len(), 3);
        // Placed after the drawing's window, before the other drawing.
        assert_eq!(app.active_doc, 1);
        assert_eq!(shapes(&app), 1);
        // An edit in the second window shows in the first.
        let b = add_rect(&mut app);
        app.switch_document(0);
        assert_eq!(shapes(&app), 2);
        assert!(app.doc().find_shape(b).is_some());
        // The undo history is the drawing's: undone in the first window,
        // gone in the second, redone there.
        app.undo();
        assert_eq!(shapes(&app), 1);
        app.switch_document(1);
        assert_eq!(shapes(&app), 1);
        assert!(app.doc().find_shape(a).is_some());
        app.redo();
        assert_eq!(shapes(&app), 2);
        // The other drawing is untouched.
        app.switch_document(2);
        assert_eq!(app.doc().title, "Other");
        assert_eq!(shapes(&app), 0);
        app.switch_document(0);
        assert_eq!(shapes(&app), 2);
        // Exactly one place keeps each parked drawing.
        assert_eq!(app.docs.iter().filter(|d| d.holds).count(), 1);
    }

    #[test]
    fn each_window_keeps_its_own_view_page_and_selection() {
        let mut app = App::headless();
        let a = add_rect(&mut app);
        let b = add_rect(&mut app);
        app.select(vec![a]);
        app.view.zoom = 5.0;
        app.new_window();
        // The new window starts as the first one shows the drawing.
        assert_eq!(app.view.zoom, 5.0);
        assert_eq!(app.selection, vec![a]);
        app.view.zoom = 9.0;
        app.select(vec![b]);
        let first_page = app.page;
        app.open_document(Document::new("Away", Size::new(50.0, 50.0)), None);
        app.switch_document(0);
        assert_eq!(app.view.zoom, 5.0);
        assert_eq!(app.selection, vec![a]);
        assert_eq!(app.page, first_page);
        app.switch_document(1);
        assert_eq!(app.view.zoom, 9.0);
        assert_eq!(app.selection, vec![b]);
        // An object deleted in one window leaves the other's selection.
        app.switch_document(0);
        app.select(vec![a]);
        app.delete_selection();
        app.switch_document(1);
        app.select(vec![a, b]);
        app.switch_document(0);
        app.switch_document(1);
        assert_eq!(app.selection, vec![b]);
    }

    #[test]
    fn window_titles_number_the_windows_of_a_drawing() {
        let mut app = App::headless();
        let name = app.document_tab_title(0);
        assert!(!name.contains(':'));
        app.new_window();
        assert_eq!(app.document_tab_title(0), format!("{name}:1"));
        assert_eq!(app.document_tab_title(1), format!("{name}:2"));
        add_rect(&mut app);
        assert_eq!(app.document_tab_title(0), format!("{name}:1*"));
        assert_eq!(app.document_tab_title(1), format!("{name}:2*"));
        app.new_window();
        assert_eq!(app.document_tab_title(2), format!("{name}:3*"));
        // Another drawing keeps a plain name; parked windows of the first
        // still read their drawing.
        app.open_document(Document::new("Plain", Size::new(50.0, 50.0)), None);
        assert_eq!(app.document_tab_title(3), "Plain");
        assert_eq!(app.document_tab_title(1), format!("{name}:2*"));
        assert_eq!(app.drawing_title(1), format!("{name}*"));
    }

    #[test]
    fn closing_one_window_keeps_the_drawing_in_the_other() {
        let mut app = App::headless();
        add_rect(&mut app);
        app.new_window();
        add_rect(&mut app);
        assert!(app.engine.is_dirty());
        // Closing a window of a changed drawing does not ask while another
        // window shows it.
        app.close_document();
        assert!(matches!(app.dialog, crate::ui::dialogs::Dialog::None));
        assert_eq!(app.docs.len(), 1);
        assert_eq!(shapes(&app), 2);
        assert!(app.engine.is_dirty());
        assert!(!app.document_tab_title(0).contains(':'));
        // A window opened again is numbered from 1.
        app.new_window();
        assert!(app.document_tab_title(0).ends_with(":1*"));
        assert!(app.document_tab_title(1).ends_with(":2*"));
        app.close_active_document();
        // The undo history came along.
        app.undo();
        assert_eq!(shapes(&app), 1);
        // The last window asks, as a drawing does.
        app.close_document();
        assert!(matches!(
            app.dialog,
            crate::ui::dialogs::Dialog::ConfirmClose
        ));
        app.dialog = crate::ui::dialogs::Dialog::None;
        app.close_active_document();
        assert!(!app.has_document());
    }

    #[test]
    fn closing_a_parked_drawing_window_and_close_all() {
        let mut app = App::headless();
        add_rect(&mut app);
        app.new_window();
        app.open_document(Document::new("Two", Size::new(50.0, 50.0)), None);
        // Close the first window while the drawing is parked: the second
        // one, next to it, gets the drawing.
        app.switch_document(0);
        app.switch_document(2);
        app.switch_document(0);
        app.close_active_document();
        assert_eq!(app.docs.len(), 2);
        assert_eq!(shapes(&app), 1);
        app.switch_document(1);
        assert_eq!(app.doc().title, "Two");
        app.switch_document(0);
        assert_eq!(shapes(&app), 1);
        // Close All asks once per changed drawing, not per window.
        app.new_window();
        app.close_all_documents();
        assert!(matches!(
            app.dialog,
            crate::ui::dialogs::Dialog::ConfirmClose
        ));
        // The extra window and the unchanged drawing are gone; the last
        // window of the changed drawing waits for the answer.
        assert_eq!(app.docs.len(), 1);
        assert_eq!(shapes(&app), 1);
        assert_eq!(app.dirty_documents().len(), 1);
    }

    #[test]
    fn saving_from_either_window_saves_the_one_drawing() {
        let dir = std::env::temp_dir().join(format!("tracedraw-new-window-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("shared.tdraw");
        let mut app = App::headless();
        add_rect(&mut app);
        app.new_window();
        add_rect(&mut app);
        app.save_to(path.clone());
        assert!(!app.engine.is_dirty());
        app.switch_document(0);
        assert!(!app.engine.is_dirty());
        assert_eq!(app.file.as_deref(), Some(path.as_path()));
        assert_eq!(app.document_tab_title(1), "shared:2");
        add_rect(&mut app);
        app.save(false);
        app.switch_document(1);
        assert!(!app.engine.is_dirty());
        let saved = std::fs::read_to_string(&path).unwrap_or_default();
        let doc = Document::from_json(&saved).expect("saved drawing");
        assert_eq!(
            doc.pages[0]
                .layers
                .iter()
                .map(|l| l.shapes.len())
                .sum::<usize>(),
            3
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tiled_windows_draw_both_windows_of_a_drawing() {
        use crate::ui::windows::{set_layout, WindowLayout};
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        add_rect(&mut app);
        app.new_window();
        app.view.zoom = 7.0;
        let draw = |app: &mut App| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
            out.textures_delta.clear();
        };
        for layout in [WindowLayout::TileVertical, WindowLayout::Cascade] {
            set_layout(&mut app, layout);
            draw(&mut app);
            draw(&mut app);
            assert_eq!(app.active_doc, 1);
            assert_eq!(shapes(&app), 1);
            assert!(!app.docs[0].holds && !app.docs[1].holds);
            assert!(!app.docs[0].fit_pending);
        }
        app.switch_document(0);
        assert_eq!(shapes(&app), 1);
    }

    /// A drawing whose only text, inside a group, is set in `family`.
    fn drawing_with_text_in(family: &str) -> Document {
        let mut doc = Document::new(family, Size::new(100.0, 50.0));
        let text = tracedraw_core::Shape::new(
            ShapeId(1),
            ShapeKind::Text {
                spans: vec![tracedraw_core::TextSpan::new("Hi", family, 12.0)],
                origin: tracedraw_core::geometry::Point::new(10.0, 10.0),
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
        );
        doc.pages[0].layers[0]
            .shapes
            .push(tracedraw_core::Shape::new(
                ShapeId(2),
                ShapeKind::Group {
                    children: vec![text],
                },
            ));
        doc
    }

    #[test]
    fn a_drawing_brings_its_fonts_along() {
        let mut app = App::headless();
        let mut doc = drawing_with_text_in("Selawik");
        doc.fonts.push(tracedraw_core::DocumentFont::new(
            "Selawik",
            include_bytes!("../assets/fonts/Selawik-Regular.ttf").to_vec(),
        ));
        app.open_document(doc, None);
        app.check_missing_fonts();
        assert!(app.font_families.iter().any(|f| f == "Selawik"));
        assert!(app.missing_fonts.is_empty(), "{:?}", app.missing_fonts);
        // A font nobody brings is reported, inside a group too.
        app.open_document(drawing_with_text_in("Nowhere Grotesk"), None);
        app.check_missing_fonts();
        assert_eq!(app.missing_fonts, vec!["Nowhere Grotesk".to_string()]);
        // A face named with its weight is drawn by its family's face of
        // that weight: not reported (#892), unless the family is missing.
        app.open_document(drawing_with_text_in("Selawik SemiBold"), None);
        app.check_missing_fonts();
        assert!(app.missing_fonts.is_empty(), "{:?}", app.missing_fonts);
        app.open_document(drawing_with_text_in("Nowhere Grotesk SemiBold"), None);
        app.check_missing_fonts();
        assert_eq!(
            app.missing_fonts,
            vec!["Nowhere Grotesk SemiBold".to_string()]
        );
    }

    #[test]
    fn fonts_that_arrive_later_refresh_the_lists() {
        let mut app = App::headless();
        app.missing_fonts = vec!["Quicksand".into(), "Nowhere Grotesk".into()];
        // Nothing new: nothing changes.
        app.sync_fonts();
        assert_eq!(app.missing_fonts.len(), 2);
        // A font arrives, as the browser build's font folder delivers them.
        let quicksand = include_bytes!("../../../crates/text/testdata/Quicksand-VF.ttf");
        let _ = tracedraw_text::fonts().add_fonts(vec![quicksand.to_vec()]);
        app.sync_fonts();
        assert_eq!(app.font_generation, tracedraw_text::fonts().generation());
        assert!(app.font_families.iter().any(|f| f == "Quicksand"));
        assert_eq!(app.missing_fonts, vec!["Nowhere Grotesk".to_string()]);
    }

    #[test]
    fn untitled_names_count_up() {
        let mut app = App::headless();
        let a = app.next_untitled_name();
        let b = app.next_untitled_name();
        assert_ne!(a, b);
    }
}
