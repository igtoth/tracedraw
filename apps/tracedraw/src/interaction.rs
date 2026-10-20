//! Pointer and keyboard handling on the canvas, per tool.

use crate::app::{App, CurveInProgress, Drag, Handle};

/// What a Pen or Bezier press would edit on the selected curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PenEdit {
    /// One of its nodes (curve, element index): dragged like with the
    /// Shape tool; with the Pen's automatic adding and deleting, a click
    /// deletes it.
    Node(tracedraw_core::ShapeId, usize),
    /// A segment (curve, segment index, place along it): a click adds a
    /// node there.
    Segment(tracedraw_core::ShapeId, usize, f64),
}
use crate::tools::Tool;
use egui::{Key, Modifiers, PointerButton, Response};
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Affine, Point, Rect, Shape as _, Vec2},
    Color, Command, Fill, ShapeId,
};

/// A press this close to a handle's centre takes it: within the handle,
/// short of the selection box (the handles stand outside it).
const HANDLE_PX: f32 = 6.0;
/// The selection handles' size, interface points.
pub(crate) const HANDLE_SIZE: f32 = 10.0;
/// How far out of the selection box the handles' centres stand,
/// interface points: a gap of 1.5 points between handle and box.
pub(crate) const HANDLE_OFFSET: f32 = HANDLE_SIZE / 2.0 + 1.5;
/// How far out of the selection box the rotate and skew arrows' centres
/// stand (rotate mode), interface points.
pub(crate) const ROTATE_HANDLE_OFFSET: f32 = 9.0;
/// The rotate and skew arrows take a press this close to their centre.
const ROTATE_HANDLE_PX: f32 = 9.0;

/// The point of the selection box `b` a handle stands for (page space).
pub(crate) fn handle_point(b: Rect, h: Handle) -> Point {
    let c = b.center();
    match h {
        Handle::N => Point::new(c.x, b.y1),
        Handle::S => Point::new(c.x, b.y0),
        Handle::E => Point::new(b.x1, c.y),
        Handle::W => Point::new(b.x0, c.y),
        Handle::NE => Point::new(b.x1, b.y1),
        Handle::NW => Point::new(b.x0, b.y1),
        Handle::SE => Point::new(b.x1, b.y0),
        Handle::SW => Point::new(b.x0, b.y0),
    }
}
/// How near the centre of rotation the pointer grabs it, screen pixels.
const PIVOT_PX: f32 = 8.0;

/// `p` turned about `from` onto the nearest multiple of `step_deg`,
/// keeping how far it reaches along that direction (Ctrl while drawing
/// lines and curves).
pub fn constrain_from(from: Point, p: Point, step_deg: f64) -> Point {
    let d = p - from;
    if d.hypot() < 1e-9 || !(d.x.is_finite() && d.y.is_finite()) {
        return p;
    }
    let step = step_deg.max(0.1).to_radians();
    let a = (d.y.atan2(d.x) / step).round() * step;
    let dir = Vec2::new(a.cos(), a.sin());
    from + dir * d.dot(dir).max(0.0)
}

/// The skew a side handle in rotate mode makes when dragged by `d` (page
/// millimetres): the top and bottom handles slant the selection sideways,
/// the left and right ones up or down. The opposite side stays put (with
/// Shift the centre line); with Ctrl the slant goes in steps of
/// `constrain_deg`.
pub fn skew_transform(
    handle: Handle,
    b: Rect,
    d: Vec2,
    mods: crate::app::HandleMods,
    constrain_deg: f64,
) -> Option<Affine> {
    let horizontal = matches!(handle, Handle::N | Handle::S);
    if handle.is_corner() || b.width() <= 1e-9 || b.height() <= 1e-9 {
        return None;
    }
    // The fixed line and how far the dragged side is from it, signed.
    let (fixed, span, moved) = match handle {
        Handle::N => (
            if mods.from_center { b.center().y } else { b.y0 },
            b.y1,
            d.x,
        ),
        Handle::S => (
            if mods.from_center { b.center().y } else { b.y1 },
            b.y0,
            d.x,
        ),
        Handle::E => (
            if mods.from_center { b.center().x } else { b.x0 },
            b.x1,
            d.y,
        ),
        _ => (
            if mods.from_center { b.center().x } else { b.x1 },
            b.x0,
            d.y,
        ),
    };
    let span = span - fixed;
    if span.abs() < 1e-9 {
        return None;
    }
    let mut k = moved / span;
    if mods.steps {
        let step = constrain_deg.max(0.1).to_radians();
        let a = (k.atan() / step).round() * step;
        // Never a right angle: that slant has no inverse.
        k = a.clamp(-1.5, 1.5).tan();
    }
    if !k.is_finite() {
        return None;
    }
    Some(if horizontal {
        // x' = x + k (y - fixed)
        Affine::new([1.0, 0.0, k, 1.0, -k * fixed, 0.0])
    } else {
        // y' = y + k (x - fixed)
        Affine::new([1.0, k, 0.0, 1.0, 0.0, -k * fixed])
    })
}

/// Whether `events` hold a copy, a cut and a paste request (the forms the
/// windowing layer gives Ctrl+C, Ctrl+X and Ctrl+V), with the pasted text.
fn clipboard_events(events: &[egui::Event]) -> (bool, bool, Option<String>) {
    let mut out = (false, false, None);
    for e in events {
        match e {
            egui::Event::Copy => out.0 = true,
            egui::Event::Cut => out.1 = true,
            egui::Event::Paste(t) => out.2 = Some(t.clone()),
            _ => {}
        }
    }
    out
}

/// The paste shortcuts: Ctrl+V (Cmd+V), Shift+Insert and the Paste key.
fn is_paste_key(key: egui::Key, mods: Modifiers) -> bool {
    key == Key::Paste || (mods.command && key == Key::V) || (mods.shift && key == Key::Insert)
}

/// Whether a paste shortcut went up without a paste event or a press
/// before it. The windowing layer turns the shortcut into a paste event
/// only while the system clipboard holds text and drops the key press
/// otherwise (a picture or files copied in another application), so its
/// release is all that is left of it. `seen` keeps a paste event or press
/// across frames until the key goes up.
fn paste_without_text(events: &[egui::Event], seen: &mut bool) -> bool {
    let mut out = false;
    for e in events {
        match e {
            egui::Event::Paste(_) => *seen = true,
            egui::Event::Key {
                key,
                pressed,
                modifiers,
                ..
            } if is_paste_key(*key, *modifiers) => {
                if *pressed {
                    *seen = true;
                } else {
                    out |= !*seen;
                    *seen = false;
                }
            }
            // V let go after Ctrl: not a paste, and the next one starts
            // afresh.
            egui::Event::Key {
                key: Key::V,
                pressed: false,
                ..
            } => *seen = false,
            _ => {}
        }
    }
    out
}

impl App {
    pub fn hit_test(&self, p: Point) -> Option<ShapeId> {
        self.hit_test_where(p, |_| false)
    }

    /// The topmost visible object at `p` (on a visible, unlocked layer)
    /// when that object is locked: what a right click there unlocks.
    pub fn locked_at(&self, p: Point) -> Option<ShapeId> {
        if self.focus_shape_on_page().is_some() {
            return None;
        }
        let tol = 3.0 / self.view.zoom as f64;
        self.page_layers()
            .into_iter()
            .rev()
            .filter(|l| l.visible && !l.locked)
            .flat_map(|l| l.shapes.iter().rev())
            .find(|s| s.visible && self.shape_hit(s, p, tol))
            .filter(|s| s.locked)
            .map(|s| s.id)
    }

    /// The topmost object at `p` that `skip` does not rule out.
    pub fn hit_test_where(&self, p: Point, skip: impl Fn(ShapeId) -> bool) -> Option<ShapeId> {
        let tol = 3.0 / self.view.zoom as f64;
        // Focus mode: only the parts of the object in focus (or itself).
        if let Some(f) = self.focus_shape_on_page() {
            return match &f.kind {
                ShapeKind::Group { .. } => self
                    .children_hit(&f, p, tol)
                    .into_iter()
                    .find(|c| !skip(c.id))
                    .map(|c| c.id),
                _ => (!skip(f.id) && self.shape_hit(&f, p, tol)).then_some(f.id),
            };
        }
        self.page_layers()
            .into_iter()
            .rev()
            .filter(|l| l.visible && !l.locked)
            .flat_map(|l| l.shapes.iter().rev())
            .find(|s| !s.locked && s.visible && !skip(s.id) && self.shape_hit(s, p, tol))
            .map(|s| s.id)
    }

    /// `s` (in page space) is under `p` within `tol` page units: text,
    /// groups, tables, bitmaps, clip frames and symbols anywhere in their
    /// box; filled objects inside their shape (an open curve only when the
    /// document fills open curves, unfilled ones too with "Treat all
    /// objects as filled"); otherwise near the outline.
    fn shape_hit(&self, s: &tracedraw_core::Shape, p: Point, tol: f64) -> bool {
        if !self.visual_bounds(s).inflate(tol, tol).contains(p) {
            return false;
        }
        if !s.effects.is_empty() {
            // What the object draws: its warped outline and the objects
            // its effects add.
            let ev = tracedraw_core::live::evaluate(s);
            return std::iter::once(ev.main)
                .chain(ev.below)
                .chain(ev.above)
                .any(|mut part| {
                    part.effects.clear();
                    self.shape_hit(&part, p, tol)
                });
        }
        if matches!(
            s.kind,
            ShapeKind::Text { .. }
                | ShapeKind::Group { .. }
                | ShapeKind::Table(_)
                | ShapeKind::Bitmap { .. }
                | ShapeKind::ClipFrame { .. }
                | ShapeKind::SymbolInstance { .. }
        ) {
            return true;
        }
        let path = s.page_path();
        let open = matches!(s.kind, ShapeKind::Path { closed: false, .. });
        let filled =
            !matches!(s.fill, Fill::None) && (!open || self.doc().metadata.fill_open_curves);
        if (filled || self.settings.treat_all_filled) && path.contains(p) {
            return true;
        }
        crate::canvas::distance_to_path(&path, p) <= tol * 1.5
    }

    /// The children of a group (`g` in page space) that are under `p`, top
    /// first, each in page space; a child group counts only where one of
    /// its own objects is. The children of a clip frame are its contents,
    /// and only where they show: inside the frame.
    fn children_hit(
        &self,
        g: &tracedraw_core::Shape,
        p: Point,
        tol: f64,
    ) -> Vec<tracedraw_core::Shape> {
        let children = match &g.kind {
            ShapeKind::Group { children } => children,
            ShapeKind::ClipFrame { frame, contents } => {
                let mut f = (**frame).clone();
                f.transform = g.transform * f.transform;
                if !f.page_path().contains(p) {
                    return Vec::new();
                }
                contents
            }
            _ => return Vec::new(),
        };
        children
            .iter()
            .rev()
            .filter(|c| c.visible && !c.locked)
            .map(|c| {
                let mut c = c.clone();
                c.transform = g.transform * c.transform;
                c
            })
            .filter(|c| {
                if matches!(c.kind, ShapeKind::Group { .. }) {
                    !self.children_hit(c, p, tol).is_empty()
                } else {
                    self.shape_hit(c, p, tol)
                }
            })
            .collect()
    }

    /// The objects of selection group `n` that are still in the drawing.
    pub fn selection_group(&self, n: u8) -> Vec<ShapeId> {
        self.doc()
            .selection_groups
            .get(&n)
            .map(|ids| {
                ids.iter()
                    .copied()
                    .filter(|id| self.doc().find_shape(*id).is_some())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Store `ids` as selection group `n`.
    pub fn set_selection_group(&mut self, n: u8, ids: Vec<ShapeId>) {
        self.run(Command::SetSelectionGroup {
            number: n,
            shapes: ids,
        });
    }

    /// Select selection group `n` (its objects on the current page); when
    /// it is selected already, zoom to it.
    pub fn recall_selection_group(&mut self, n: u8) {
        // Objects on the layers the current page shows (one inside a group
        // counts by the group it is in).
        let shown: Vec<ShapeId> = self
            .page_layers()
            .into_iter()
            .flat_map(|l| l.shapes.iter().map(|s| s.id))
            .collect();
        let ids: Vec<ShapeId> = self
            .selection_group(n)
            .into_iter()
            .filter(|id| {
                let top = self.doc().top_level_of(*id).unwrap_or(*id);
                shown.contains(&top)
            })
            .collect();
        if ids.is_empty() {
            return;
        }
        let same =
            ids.len() == self.selection.len() && ids.iter().all(|id| self.selection.contains(id));
        if same {
            self.zoom_to_selection();
        } else {
            self.select(ids);
        }
    }

    /// Ctrl+click: one group level further in at `p` than what is selected
    /// there (the first level inside the topmost group when nothing of it
    /// is), down to the innermost object.
    pub fn hit_test_step(&self, p: Point) -> Option<ShapeId> {
        let top = self.hit_test(p)?;
        let tol = 3.0 / self.view.zoom as f64;
        let mut chain = vec![top];
        let mut current = self.doc().shape(top).ok()?.1.clone();
        while let Some(c) = self.children_hit(&current, p, tol).into_iter().next() {
            chain.push(c.id);
            current = c;
        }
        let next = match chain.iter().rposition(|id| self.selection.contains(id)) {
            Some(k) => k + 1,
            None => 1,
        };
        chain.get(next.min(chain.len() - 1)).copied()
    }

    /// Every object at `p`, top first: the objects on the layers, or with
    /// `deep` the innermost objects inside groups (Alt+click digs through
    /// these, Ctrl+Alt+click through the deep ones).
    pub fn hits_at(&self, p: Point, deep: bool) -> Vec<ShapeId> {
        let tol = 3.0 / self.view.zoom as f64;
        let mut out = Vec::new();
        fn leaves(
            app: &App,
            s: &tracedraw_core::Shape,
            p: Point,
            tol: f64,
            out: &mut Vec<ShapeId>,
        ) {
            let inner = app.children_hit(s, p, tol);
            for c in &inner {
                leaves(app, c, p, tol, out);
            }
            // A clip frame is an object of its own under its contents.
            if inner.is_empty() || matches!(s.kind, ShapeKind::ClipFrame { .. }) {
                out.push(s.id);
            }
        }
        for s in self
            .page_layers()
            .into_iter()
            .rev()
            .filter(|l| l.visible && !l.locked)
            .flat_map(|l| l.shapes.iter().rev())
            .filter(|s| !s.locked && s.visible && self.shape_hit(s, p, tol))
        {
            if deep {
                leaves(self, s, p, tol, &mut out);
            } else {
                out.push(s.id);
            }
        }
        out
    }

    /// Add the objects `region` takes (wholly inside, or with `touching`
    /// those it touches) to the selection: on unlocked, visible layers,
    /// not locked or hidden.
    pub fn select_in_region(&mut self, region: crate::select_region::Region, touching: bool) {
        let doc = self.doc();
        let fill_open = doc.metadata.fill_open_curves;
        let all_filled = self.settings.treat_all_filled;
        let fill_counts = |s: &tracedraw_core::Shape| {
            all_filled
                || (!matches!(s.fill, Fill::None)
                    && (fill_open || !matches!(s.kind, ShapeKind::Path { closed: false, .. })))
        };
        let mut ids = self.selection.clone();
        for s in self
            .page_layers()
            .into_iter()
            .filter(|l| l.visible && !l.locked)
            .flat_map(|l| &l.shapes)
            .filter(|s| s.visible && !s.locked)
        {
            if !ids.contains(&s.id)
                && crate::select_region::takes(region, s, touching, &fill_counts)
            {
                ids.push(s.id);
            }
        }
        self.select(ids);
        self.marquee_selection = Some(self.selection.clone());
    }

    /// A click that selects with a tool other than Pick (see
    /// [`Tool::click_selects`]): the object under `p` becomes the
    /// selection, Shift adds it or takes it away, Ctrl reaches one group
    /// level further in, and a click on empty space (without Shift) lets
    /// go of the selection. The tool stays.
    pub fn click_select(&mut self, p: Point, mods: Modifiers) {
        let hit = if mods.ctrl {
            self.hit_test_step(p)
        } else {
            self.hit_test(p)
        };
        match hit {
            Some(id) if mods.shift => {
                if let Some(i) = self.selection.iter().position(|s| *s == id) {
                    self.selection.remove(i);
                    self.rotate_mode = false;
                } else {
                    self.select_add(id);
                }
            }
            Some(id) => self.select(vec![id]),
            None if mods.shift => {}
            None => self.select(Vec::new()),
        }
    }

    /// Add `id` to the selection, taking out the groups holding it and the
    /// objects inside it, so nothing is selected twice over.
    pub fn select_add(&mut self, id: ShapeId) {
        let above = self.doc().ancestors(id);
        let doc = self.doc();
        let keep: Vec<ShapeId> = self
            .selection
            .iter()
            .copied()
            .filter(|s| *s != id && !above.contains(s) && !doc.ancestors(*s).contains(&id))
            .collect();
        self.selection = keep;
        self.selection.push(id);
        self.rotate_mode = false;
    }

    /// Screen-space handle positions for the selection box.
    pub fn handle_positions(&self, b: Rect) -> [(Handle, egui::Pos2); 8] {
        let off = if self.rotate_mode {
            ROTATE_HANDLE_OFFSET
        } else {
            HANDLE_OFFSET
        };
        let r = self.view.rect_to_screen(b).expand(off);
        [
            (Handle::NW, r.left_top()),
            (Handle::N, r.center_top()),
            (Handle::NE, r.right_top()),
            (Handle::W, r.left_center()),
            (Handle::E, r.right_center()),
            (Handle::SW, r.left_bottom()),
            (Handle::S, r.center_bottom()),
            (Handle::SE, r.right_bottom()),
        ]
    }

    /// The centre of rotation of the selection: where it was moved to in
    /// rotate mode (now or the last time these objects were selected),
    /// else the middle of the selection.
    pub fn rotation_center(&self) -> Option<Point> {
        let b = self.selection_bounds()?;
        Some(self.moved_pivot(b).unwrap_or(b.center()))
    }

    /// The centre of rotation the user moved for the selection (page
    /// space), if one was moved.
    pub fn moved_rotation_center(&self) -> Option<Point> {
        self.moved_pivot(self.selection_bounds()?)
    }

    /// The centre of rotation moved for the selection, in page space.
    fn moved_pivot(&self, b: Rect) -> Option<Point> {
        let p = match self.selection.as_slice() {
            [] => return None,
            [one] => {
                let local = self.pivots.single.get(one)?;
                self.doc().shape_on_page(*one)?.transform * *local
            }
            many => {
                let mut key = many.to_vec();
                key.sort();
                let (u, v) = self.pivots.several.get(&key)?;
                Point::new(b.x0 + u * b.width(), b.y0 + v * b.height())
            }
        };
        (p.x.is_finite() && p.y.is_finite()).then_some(p)
    }

    /// Put the selection's centre of rotation at `p` (page space), kept
    /// with the object (or the objects) for the next time.
    pub fn set_pivot(&mut self, p: Point) {
        match self.selection.clone().as_slice() {
            [] => {}
            [one] => {
                let Some(t) = self.doc().shape_on_page(*one).map(|s| s.transform) else {
                    return;
                };
                let det = t.determinant();
                if det.is_finite() && det.abs() > 1e-12 {
                    self.pivots.single.insert(*one, t.inverse() * p);
                }
            }
            many => {
                let Some(b) = self.selection_bounds() else {
                    return;
                };
                let (w, h) = (b.width().max(1e-9), b.height().max(1e-9));
                let mut key = many.to_vec();
                key.sort();
                self.pivots
                    .several
                    .insert(key, ((p.x - b.x0) / w, (p.y - b.y0) / h));
            }
        }
    }

    /// The selection's centre of rotation goes back to its middle.
    pub fn reset_pivot(&mut self) {
        match self.selection.as_slice() {
            [] => {}
            [one] => {
                self.pivots.single.remove(one);
            }
            many => {
                let mut key = many.to_vec();
                key.sort();
                self.pivots.several.remove(&key);
            }
        }
    }

    /// The pointer is on the centre of rotation (rotate mode).
    pub fn pivot_at(&self, screen: egui::Pos2) -> bool {
        self.rotate_mode
            && self
                .rotation_center()
                .is_some_and(|c| self.view.to_screen(c).distance(screen) <= PIVOT_PX)
    }

    /// The pointer is on the selection's centre mark, the "x" (outside
    /// rotate mode): a drag there moves the selection, a click toggles the
    /// rotate and skew handles.
    pub fn center_mark_at(&self, screen: egui::Pos2) -> bool {
        !self.rotate_mode
            && matches!(self.tool, Tool::Pick | Tool::FreeformPick)
            && self
                .selection_bounds()
                .is_some_and(|b| self.view.to_screen(b.center()).distance(screen) <= PIVOT_PX)
    }

    /// Start moving the selection from `p` (page).
    fn start_move(&mut self, p: Point) {
        // A text fitted to a path slides along it instead.
        if self.fitted_text().is_some() {
            self.drag = Drag::AlongPath {
                start: p,
                current: p,
            };
            return;
        }
        self.drag = Drag::Move {
            start: p,
            last: p,
            total: Vec2::ZERO,
            start_bounds: self.selection_bounds().unwrap_or(Rect::ZERO),
            source: self.selection_snap_source(p),
        };
    }

    /// Where the next node of the curve being drawn goes for the pointer
    /// at `p`: with `ctrl` in steps of the constrain angle from the last
    /// node.
    pub fn curve_point(&self, p: Point, ctrl: bool) -> Point {
        match self.curve.as_ref().and_then(|c| c.nodes.last()) {
            Some((last, _)) if ctrl => constrain_from(*last, p, self.settings.constrain_angle),
            _ => p,
        }
    }

    /// The pointer is over the document palette.
    pub fn over_document_palette(&self) -> bool {
        self.doc_palette_rect
            .zip(self.pointer_screen)
            .is_some_and(|(r, p)| r.contains(p))
    }

    /// A move, scale, rotation or skew of the selection is being dragged.
    pub fn transforming(&self) -> bool {
        matches!(
            self.drag,
            Drag::Move { .. } | Drag::Scale { .. } | Drag::Rotate { .. } | Drag::Skew { .. }
        ) && matches!(self.tool, Tool::Pick | Tool::FreeformPick)
    }

    /// The end of a move, scale, rotation or skew: on the selection, or on
    /// copies of it (which become the selection) after a right click
    /// during the drag.
    fn finish_transform(&mut self, t: Affine) {
        if std::mem::take(&mut self.drag_copy) {
            self.duplicate_in_place();
        }
        self.transform_selection(t);
    }

    /// Space during a move leaves a copy where the selection is now.
    /// Space while the selection is moved, scaled, turned or slanted drops
    /// a copy where the outline is, and the drag goes on.
    fn stamp_on_space(&mut self, response: &Response) {
        if !response.ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            return;
        }
        let t = match &self.drag {
            Drag::Move { total, .. } => Some(Affine::translate(*total)),
            d @ (Drag::Scale { .. } | Drag::Rotate { .. } | Drag::Skew { .. }) => {
                self.preview_transform_of(d)
            }
            _ => None,
        };
        if let Some(t) = t {
            self.stamp_copies(t);
        }
    }

    /// Copies of the selection moved by `t`, left where they are while the
    /// drag goes on (Space during a move), as one undo step.
    pub fn stamp_copies(&mut self, t: Affine) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let mut cmds = Vec::new();
        for s in self.selected_shapes() {
            let id = self.engine.new_shape_id();
            let mut shape = crate::app::reid_pub(s, id, &mut self.engine);
            shape.transform = t * shape.transform;
            cmds.push(Command::AddShape { layer, shape });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Copy", &cmds) {
            self.status = e.to_string();
        }
    }

    pub(crate) fn handle_at(&self, screen: egui::Pos2) -> Option<Handle> {
        let b = self.selection_bounds()?;
        let reach = if self.rotate_mode {
            ROTATE_HANDLE_PX
        } else {
            HANDLE_PX
        };
        self.handle_positions(b)
            .iter()
            .find(|(_, p)| p.distance(screen) <= reach)
            .map(|(h, _)| *h)
    }

    pub fn canvas_input(&mut self, response: &Response, mods: Modifiers) {
        self.snap_mark.set(None);
        self.clear_live_guides();
        self.pointer_screen = response.ctx.pointer_latest_pos();
        // egui ends a drag when the right button of a right click goes up
        // and then hovers nothing while the left one is held: the
        // transform goes on with the latest pointer position.
        let hover = response.hover_pos().or_else(|| {
            let held = response.ctx.input(|i| i.pointer.primary_down());
            if self.transforming() && held {
                self.pointer_screen
            } else {
                None
            }
        });
        let pointer = hover.map(|h| self.view.to_page(h));
        self.pointer_page = pointer;
        let Some(p) = pointer else {
            let released = self.transforming() && !response.ctx.input(|i| i.pointer.primary_down());
            if response.drag_stopped() || released {
                self.end_drag();
            }
            return;
        };
        let mut screen = hover.unwrap_or_default();
        // A drag starts after the pointer moved a few pixels; hit-test at the
        // press position so small targets like nodes and handles are caught.
        let mut p = p;
        if response.drag_started() {
            if let Some(origin) = response.ctx.input(|i| i.pointer.press_origin()) {
                screen = origin;
                p = self.view.to_page(origin);
            }
        }

        // Middle button always pans.
        if response.dragged_by(PointerButton::Middle) {
            self.view.pan(response.drag_delta());
            return;
        }
        // A right click during a move, scale, rotation or skew: the change
        // goes to copies on release, the originals stay (again: not).
        if self.transforming() {
            if response
                .ctx
                .input(|i| i.pointer.button_pressed(PointerButton::Secondary))
            {
                self.drag_copy = !self.drag_copy;
                self.status = crate::i18n::tr(if self.drag_copy {
                    "status.drag_copy_on"
                } else {
                    "status.drag_copy_off"
                });
            }
            // egui ends a drag when any button goes up (the right one, for
            // a copy): the transform goes on while the left button is held
            // and ends when it goes up.
            if !response.dragged() && !response.drag_stopped_by(PointerButton::Primary) {
                if response.ctx.input(|i| i.pointer.primary_down()) {
                    self.pick_drag_to(p, mods);
                    self.stamp_on_space(response);
                } else {
                    self.end_drag();
                }
                return;
            }
        } else if matches!(self.tool, Tool::Pick | Tool::FreeformPick)
            && self.right_drag_input(response, p)
        {
            // Right-button drags of objects end in the drop menu.
            return;
        }

        // With the Zoom tool a right click zooms out by default.
        if response.secondary_clicked()
            && self.tool == Tool::Zoom
            && self.settings.zoom_right_click_out
        {
            self.view.zoom_at(screen, 0.5);
            return;
        }
        // Right click: context menu (drawn by the UI at this position);
        // none while a transform is dragged.
        if response.secondary_clicked() && !self.pending_clip_frame && !self.transforming() {
            if self.pending_click(response, p) || self.coords_pick_input(response, p) {
                return;
            }
            self.open_context_menu(screen, p);
            return;
        }
        if self.pending_click(response, p) {
            return;
        }
        // The Coordinates docker waits for a click or a drag.
        if self.coords_pick_input(response, p) {
            return;
        }

        if self.pending_clip_frame {
            if response.clicked_by(PointerButton::Primary) {
                self.pending_clip_frame = false;
                if let Some(frame) = self.hit_test(p) {
                    let contents: Vec<ShapeId> = self
                        .selection
                        .iter()
                        .copied()
                        .filter(|id| *id != frame)
                        .collect();
                    if contents.is_empty() {
                        self.status = crate::i18n::tr("status.clip_frame_select_contents");
                    } else {
                        self.place_inside(contents, frame);
                        self.select(vec![frame]);
                    }
                } else {
                    self.status = crate::i18n::tr("status.clip_frame_cancelled");
                }
            }
            if response.secondary_clicked() {
                self.pending_clip_frame = false;
            }
            return;
        }
        // A click with a tool whose click has no job of its own selects;
        // a drag goes on to the tool below.
        if self.tool.click_selects() && response.clicked_by(PointerButton::Primary) {
            self.click_select(p, mods);
            return;
        }
        match self.tool {
            Tool::Pick | Tool::FreeformPick => self.pick_input(response, p, screen, mods),
            Tool::Shape => {
                if !self.envelope_input(response, p) && !self.effect_node_input(response, p) {
                    self.shape_input(response, p, mods)
                }
            }
            Tool::FreeTransform
            | Tool::AttractRepel
            | Tool::Smudge
            | Tool::Roughen
            | Tool::VariableOutline
            | Tool::SegmentDelete
            | Tool::ShapeRecognition
            | Tool::Sketch
            | Tool::HorizontalVerticalDimension
            | Tool::AngularDimension
            | Tool::SegmentDimension
            | Tool::Callout
            | Tool::RightAngleConnector
            | Tool::RoundedConnector
            | Tool::AnchorEditing
            | Tool::BlockShadow
            | Tool::Envelope
            | Tool::Extrude
            | Tool::Distort
            | Tool::AreaFill
            | Tool::MeshFill
            | Tool::OutlinePen
            | Tool::OutlineColor => self.tools3_input(response, p, mods),
            Tool::Zoom => self.zoom_input(response, p, screen, mods),
            Tool::Pan => {
                if response.dragged() {
                    self.view.pan(response.drag_delta());
                }
            }
            t if t.is_box_tool() => self.box_input(response, p, mods),
            Tool::ThreePointRectangle | Tool::ThreePointEllipse | Tool::ThreePointCurve => {
                self.three_point_input(response, p)
            }
            Tool::Freehand => self.freehand_input(response, p, mods),
            Tool::Bezier | Tool::Pen | Tool::Polyline | Tool::TwoPointLine | Tool::BSpline => {
                self.curve_input(response, p, mods)
            }
            Tool::Text => self.text_input(response, p, mods),
            Tool::ColorEyedropper | Tool::AttributesEyedropper => {
                if response.clicked() {
                    let attrs = self.tool == Tool::AttributesEyedropper;
                    self.eyedropper_click(p, attrs, mods.shift);
                }
            }
            Tool::InteractiveFill => self.fill_input(response, p),
            Tool::Crop => self.crop_input(response, p),
            Tool::Contour => self.contour_input(response, p),
            Tool::Knife
            | Tool::Spiral
            | Tool::CommonShapes
            | Tool::Table
            | Tool::BrushStrokes
            | Tool::PainterlyBrush
            | Tool::ParallelDimension
            | Tool::Connector => self.tools2_input(response, p, mods),
            Tool::Blend | Tool::Smooth | Tool::Smear | Tool::Twirl => {
                self.effects_input(response, p, mods)
            }
            Tool::DropShadow => {
                let mut first = false;
                if response.drag_started_by(PointerButton::Primary) {
                    // The press point, not where the drag was noticed.
                    let press = response
                        .ctx
                        .input(|i| i.pointer.press_origin())
                        .map(|o| self.view.to_page(o))
                        .unwrap_or(p);
                    if let Some(handle) = self.shadow_handle_at(press) {
                        self.drag = Drag::ShadowHandle { handle };
                    } else if let Some(id) = self.hit_test(press) {
                        self.select(vec![id]);
                        self.drag = Drag::Shadow {
                            shape: id,
                            start: press,
                        };
                        first = true;
                    }
                }
                if response.dragged_by(PointerButton::Primary) {
                    match self.drag.clone() {
                        Drag::Shadow { shape, start } => {
                            self.drag_new_shadow(shape, start, p, first);
                            self.drag = Drag::Shadow { shape, start };
                        }
                        Drag::ShadowHandle { handle } => self.drag_shadow_handle(handle, p),
                        _ => {}
                    }
                }
                if response.clicked_by(PointerButton::Primary) {
                    if self.shadow_handle_at(p).is_some() {
                        return;
                    }
                    match self.hit_test(p) {
                        Some(id) => self.select(vec![id]),
                        None => self.select(Vec::new()),
                    }
                }
            }
            Tool::Transparency => {
                if response.drag_started_by(PointerButton::Primary) {
                    if let Some(id) = self.hit_test(p) {
                        self.select(vec![id]);
                        self.drag = Drag::FillGradient {
                            shape: id,
                            start: p,
                            current: p,
                        };
                    }
                }
                if response.dragged_by(PointerButton::Primary) {
                    if let Drag::FillGradient { current, .. } = &mut self.drag {
                        *current = p;
                    }
                }
                if response.clicked() {
                    match self.hit_test(p) {
                        Some(id) => {
                            if !self.selection.contains(&id) {
                                self.select(vec![id]);
                            }
                        }
                        None => self.select(Vec::new()),
                    }
                }
            }
            Tool::Eraser => {
                // Drag erases a band; a click erases a dot (double-click
                // on an object deletes it whole).
                if response.double_clicked_by(PointerButton::Primary) {
                    if let Some(id) = self.hit_test(p) {
                        self.run(Command::DeleteShapes { shapes: vec![id] });
                        self.selection.retain(|s| *s != id);
                    }
                } else if response.drag_started_by(PointerButton::Primary) {
                    self.drag = Drag::Freehand { points: vec![p] };
                } else if response.dragged_by(PointerButton::Primary) {
                    if let Drag::Freehand { points } = &mut self.drag {
                        if points
                            .last()
                            .map(|l| (*l - p).hypot() > 0.3)
                            .unwrap_or(true)
                        {
                            points.push(p);
                        }
                    }
                } else if response.clicked_by(PointerButton::Primary) {
                    self.erase_along(&[p]);
                }
            }
            _ => {
                if response.clicked() {
                    self.status = crate::i18n::trf(
                        "status.tool_not_implemented",
                        &[("t", &self.tool.name())],
                    );
                }
            }
        }

        if response.drag_stopped_by(PointerButton::Primary) {
            self.end_drag();
        }
    }

    fn pick_input(&mut self, response: &Response, p: Point, screen: egui::Pos2, mods: Modifiers) {
        // Resting over the selection (not over a handle or the centre of
        // rotation), the pointer shows the snap point or edge a drag
        // would carry, with its name.
        if matches!(self.drag, Drag::None)
            && response.hovered()
            && self.handle_at(screen).is_none()
            && !self.pivot_at(screen)
        {
            if let Some(t) = self.selection_hover_target(p) {
                self.snap_mark.set(Some(t));
            }
        }
        if response.drag_started_by(PointerButton::Primary) {
            self.drag_copy = false;
            // Rotate mode: the centre of rotation moves.
            if self.pivot_at(screen) {
                self.drag = Drag::Pivot;
                return;
            }
            if !self.rotate_mode && self.start_tracked_node_drag(p) {
                return;
            }
            if let Some(h) = self.handle_at(screen) {
                let b = self.selection_bounds().unwrap_or(Rect::ZERO);
                if self.rotate_mode {
                    if h.is_corner() {
                        let c = self.rotation_center().unwrap_or(b.center());
                        let a = (p - c).atan2();
                        self.drag = Drag::Rotate {
                            center: c,
                            start_angle: a,
                            current_angle: a,
                        };
                    } else {
                        self.drag = Drag::Skew {
                            handle: h,
                            start_bounds: b,
                            start: p,
                            current: p,
                            mods: crate::app::HandleMods::of(mods),
                        };
                    }
                } else {
                    // The point of the box the handle stands for, and how
                    // far the pointer is from it.
                    let grab = p - handle_point(b, h);
                    let anchor = match h {
                        Handle::N => Point::new(b.center().x, b.y0),
                        Handle::S => Point::new(b.center().x, b.y1),
                        Handle::E => Point::new(b.x0, b.center().y),
                        Handle::W => Point::new(b.x1, b.center().y),
                        Handle::NE => Point::new(b.x0, b.y0),
                        Handle::NW => Point::new(b.x1, b.y0),
                        Handle::SE => Point::new(b.x0, b.y1),
                        Handle::SW => Point::new(b.x1, b.y1),
                    };
                    self.drag = Drag::Scale {
                        handle: h,
                        anchor,
                        start_bounds: b,
                        current: p - grab,
                        grab,
                        mods: crate::app::HandleMods::of(mods),
                    };
                }
                return;
            }
            // The centre mark moves the selection, whatever lies under it.
            if self.center_mark_at(screen) && !mods.shift {
                self.start_move(p);
                return;
            }
            // Guidelines lie above the layers but objects win a press, as
            // they are what is usually edited; the rotation handles of a
            // guideline in rotate mode win over objects.
            if (self.guide_rotate.is_some() || self.hit_test(p).is_none())
                && self.press_guide(p, mods.shift)
            {
                return;
            }
            // Ctrl takes the object inside a group, unless the press is on
            // what is selected already (Ctrl then keeps the move straight).
            let hit = match self.hit_test(p) {
                Some(top) if mods.ctrl && !self.selection.contains(&top) => self.hit_test_step(p),
                other => other,
            };
            match hit {
                Some(id) => {
                    if mods.shift {
                        if !self.selection.contains(&id) {
                            self.select_add(id);
                        }
                    } else if !self.selection.contains(&id) {
                        self.select(vec![id]);
                    }
                    self.start_move(p);
                }
                None => {
                    if !mods.shift {
                        self.select(Vec::new());
                    }
                    self.deselect_guides();
                    self.drag = if self.tool == Tool::FreeformPick {
                        Drag::Lasso {
                            points: vec![p],
                            touching: mods.alt != self.settings.marquee_touching,
                        }
                    } else {
                        Drag::Marquee {
                            start: p,
                            current: p,
                            touching: mods.alt != self.settings.marquee_touching,
                        }
                    };
                }
            }
        }
        if response.dragged_by(PointerButton::Primary) {
            if matches!(self.drag, Drag::NodeMove { .. }) {
                self.node_move_to(p, mods);
                return;
            }
            self.pick_drag_to(p, mods);
        }
        self.stamp_on_space(response);
        if response.clicked_by(PointerButton::Primary) {
            // A click on the centre of rotation keeps rotate mode.
            if self.handle_at(screen).is_some() || self.pivot_at(screen) {
                return;
            }
            // A click on the centre mark: like a click on the selection.
            if self.center_mark_at(screen) && !mods.shift && !mods.alt {
                self.toggle_rotate_mode();
                return;
            }
            if (self.guide_rotate.is_some() || self.hit_test(p).is_none())
                && self.click_guide(p, mods.shift)
            {
                return;
            }
            self.deselect_guides();
            // Alt digs: the object beneath the selected one there (with
            // Ctrl among the objects inside groups).
            if mods.alt {
                let hits = self.hits_at(p, mods.ctrl);
                if !hits.is_empty() {
                    let below = match self.selection.as_slice() {
                        [one] => hits
                            .iter()
                            .position(|h| h == one)
                            .map(|i| (i + 1) % hits.len()),
                        _ => None,
                    };
                    let id = hits[below.unwrap_or(0)];
                    if mods.shift {
                        self.select_add(id);
                    } else {
                        self.select(vec![id]);
                    }
                    return;
                }
            }
            // Ctrl: one group level further in.
            let hit = if mods.ctrl {
                self.hit_test_step(p)
            } else {
                self.hit_test(p)
            };
            match hit {
                Some(id) if mods.shift => {
                    if let Some(i) = self.selection.iter().position(|s| *s == id) {
                        self.selection.remove(i);
                        self.rotate_mode = false;
                    } else {
                        self.select_add(id);
                    }
                }
                Some(id) => {
                    if self.selection.contains(&id) {
                        // A click on what is selected: toggle the rotate and
                        // skew handles.
                        self.toggle_rotate_mode();
                    } else {
                        self.select(vec![id]);
                    }
                }
                None => self.select(Vec::new()),
            }
        }
        if response.double_clicked_by(PointerButton::Primary) {
            // The centre of rotation goes back to the middle.
            if self.pivot_at(screen) {
                self.reset_pivot();
                return;
            }
            // The page border or its shadow: the page size options.
            let on_frame = response
                .interact_pointer_pos()
                .is_some_and(|s| crate::canvas::on_page_frame(self, s));
            if on_frame && self.hit_test(p).is_none() {
                self.open_page_options();
                return;
            }
            if let Some(id) = self.hit_test(p) {
                // A group or clip frame: into focus mode.
                if self.focus_on_double_click(id) {
                    self.focus_on(id);
                    return;
                }
                if let Some(s) = self.doc().find_shape(id).cloned() {
                    if matches!(s.kind, ShapeKind::Text { .. }) {
                        // Straight into editing that text, caret at the end.
                        self.select(vec![id]);
                        self.edit_selected_text();
                    } else {
                        self.select(vec![id]);
                        self.set_tool(Tool::Shape);
                    }
                }
            }
        }
    }

    /// A Pick tool drag goes on with the pointer at `p` (page) and the
    /// modifier keys `mods`.
    fn pick_drag_to(&mut self, p: Point, mods: Modifiers) {
        if matches!(self.drag, Drag::MoveGuide { .. } | Drag::RotateGuide { .. }) {
            self.drag_guide_to(p);
            return;
        }
        let snapped_move = match &self.drag {
            Drag::Move {
                start,
                start_bounds,
                source,
                ..
            } => {
                // The pointer's way since the press, snapped; Ctrl
                // keeps it horizontal or vertical, whichever is longer.
                let mut d = p - *start;
                let axis = mods.ctrl.then_some(d.x.abs() >= d.y.abs());
                match axis {
                    Some(true) => d.y = 0.0,
                    Some(false) => d.x = 0.0,
                    None => {}
                }
                let mut t = self.snap_move_from(*source, *start_bounds, d);
                match axis {
                    Some(true) => t.y = 0.0,
                    Some(false) => t.x = 0.0,
                    None => {}
                }
                Some((p, t))
            }
            _ => None,
        };
        // Handles snap to other objects, guidelines, the grid and the
        // page, never to the selection's own points: the point of the box
        // a scale or skew handle stands for, and the pointer turning the
        // selection (unless Ctrl turns it in steps).
        let snapped_handle = match &self.drag {
            Drag::Scale { grab, .. } => self.snap_point_excluding(p - *grab, &self.selection),
            Drag::Skew {
                handle,
                start_bounds,
                start,
                ..
            } => {
                let grab = *start - handle_point(*start_bounds, *handle);
                self.snap_point_excluding(p - grab, &self.selection) + grab
            }
            Drag::Rotate { .. } if !mods.ctrl => self.snap_point_excluding(p, &self.selection),
            _ => p,
        };
        match &mut self.drag {
            Drag::AlongPath { current, .. } => *current = p,
            Drag::Move { last, total, .. } => {
                if let Some((np, nt)) = snapped_move {
                    *last = np;
                    *total = nt;
                }
            }
            Drag::Marquee {
                start,
                current,
                touching,
            } => {
                // Ctrl keeps the marquee square.
                let mut q = p;
                if mods.ctrl || mods.command {
                    let d = p - *start;
                    let side = d.x.abs().max(d.y.abs());
                    q = *start + Vec2::new(side * d.x.signum(), side * d.y.signum());
                }
                *current = q;
                *touching = mods.alt != self.settings.marquee_touching;
            }
            Drag::Lasso { points, touching } => {
                // Ctrl keeps the freehand selection a rectangle.
                if (mods.ctrl || mods.command) && !points.is_empty() {
                    let s = points[0];
                    *points = vec![s, Point::new(p.x, s.y), p, Point::new(s.x, p.y)];
                } else {
                    points.push(p);
                }
                *touching = mods.alt != self.settings.marquee_touching;
            }
            Drag::Scale {
                current, mods: m, ..
            } => {
                // The handle snaps to other objects, guidelines, the grid
                // and the page, never to the selection's own points.
                *current = snapped_handle;
                *m = crate::app::HandleMods::of(mods);
            }
            Drag::Skew {
                current, mods: m, ..
            } => {
                *current = snapped_handle;
                *m = crate::app::HandleMods::of(mods);
            }
            Drag::Pivot => {
                // It snaps: a corner of the selection, a node, the grid.
                let q = self.snap_pivot(p);
                self.set_pivot(q);
            }
            Drag::Rotate {
                center,
                start_angle,
                current_angle,
            } => {
                let mut a = (snapped_handle - *center).atan2();
                // Ctrl turns in steps of the constrain angle.
                if mods.ctrl {
                    let step = self.settings.constrain_angle.max(0.1).to_radians();
                    a = *start_angle + ((a - *start_angle) / step).round() * step;
                }
                *current_angle = a;
            }
            _ => {}
        }
    }

    fn shape_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        self.shape_tool_input(response, p, mods);
    }

    fn text_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        // Inside the text being edited, the pointer places the caret and
        // drags a selection; elsewhere it starts a new text or edits
        // another one.
        let on_edited = self
            .text_edit
            .as_ref()
            .map(|te| te.shape)
            .filter(|id| self.hit_test(p) == Some(*id))
            .is_some();
        if response.drag_started_by(PointerButton::Primary) {
            if on_edited {
                if let Some(idx) = self.text_hit_char(p) {
                    self.text_set_caret_at(idx, mods.shift);
                }
                self.drag = Drag::TextSelect;
            } else {
                if self.text_edit.is_some() {
                    self.finish_text();
                }
                self.drag = Drag::TextFrame {
                    start: p,
                    current: p,
                };
            }
        }
        if response.dragged_by(PointerButton::Primary) {
            match &mut self.drag {
                Drag::TextFrame { current, .. } => *current = p,
                Drag::TextSelect => {
                    if let Some(idx) = self.text_hit_char(p) {
                        self.text_set_caret_at(idx, true);
                    }
                }
                _ => {}
            }
        }
        if response.double_clicked_by(PointerButton::Primary) && on_edited {
            if let Some(idx) = self.text_hit_char(p) {
                self.text_select_word_at(idx);
            }
            return;
        }
        if response.clicked_by(PointerButton::Primary) {
            if on_edited {
                if let Some(idx) = self.text_hit_char(p) {
                    self.text_set_caret_at(idx, mods.shift);
                }
                return;
            }
            match self.hit_test(p) {
                Some(id)
                    if matches!(
                        self.doc()
                            .shape(id)
                            .map(|(_, s)| matches!(s.kind, ShapeKind::Text { .. })),
                        Ok(true)
                    ) =>
                {
                    self.begin_text_edit(id);
                    if let Some(idx) = self.text_hit_char(p) {
                        self.text_set_caret_at(idx, false);
                    }
                }
                // On a path's outline the text starts on the path.
                _ if self.start_text_on_path_at(p) => {}
                _ => {
                    let p = self.snap_point(p);
                    self.start_text(p, None);
                }
            }
        }
    }

    fn zoom_input(&mut self, response: &Response, p: Point, screen: egui::Pos2, mods: Modifiers) {
        if response.drag_started_by(PointerButton::Primary) {
            self.drag = Drag::ZoomBox {
                start: p,
                current: p,
            };
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::ZoomBox { current, .. } = &mut self.drag {
                *current = p;
            }
        }
        if response.clicked_by(PointerButton::Primary) {
            self.view
                .zoom_at(screen, if mods.shift { 0.5 } else { 2.0 });
        }
    }

    fn box_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        let p = self.snap_point(p);
        if response.drag_started_by(PointerButton::Primary) {
            // A new shape begins: what was selected lets go at once.
            self.select(Vec::new());
            self.drag = Drag::Box {
                start: p,
                current: p,
                from_center: mods.shift,
            };
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::Box {
                start,
                current,
                from_center,
            } = &mut self.drag
            {
                let mut q = p;
                if mods.ctrl {
                    // Constrain to a square/circle.
                    let d = q - *start;
                    let m = d.x.abs().max(d.y.abs());
                    q = *start + Vec2::new(m * d.x.signum(), m * d.y.signum());
                }
                // Shift draws from the centre (checked while dragging).
                *from_center = mods.shift;
                *current = q;
            }
        }
        // A page frame comes from double-clicking the tool's toolbox
        // button, not from a double click on the drawing.
    }

    /// 3-point rectangle, ellipse and curve: drag the base segment, then
    /// click the third point (Esc cancels, see `keyboard`).
    fn three_point_input(&mut self, response: &Response, p: Point) {
        let p = self.snap_point(p);
        if self.three_point_base.is_some() {
            if response.clicked_by(PointerButton::Primary) {
                if let Some((a, b)) = self.three_point_base.take() {
                    self.finish_three_point(a, b, p);
                }
            }
            return;
        }
        if response.drag_started_by(PointerButton::Primary) {
            self.select(Vec::new());
            self.drag = Drag::ThreePointBase {
                start: p,
                current: p,
            };
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::ThreePointBase { current, .. } = &mut self.drag {
                *current = p;
            }
        }
    }

    fn freehand_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        // A click, then another: a straight line between them (Ctrl keeps
        // it to steps of the constrain angle); Esc drops the first click.
        if response.clicked_by(PointerButton::Primary) && self.tool == Tool::Freehand {
            let p = self.curve_point(self.snap_point(p), mods.ctrl);
            let c = self.curve.get_or_insert_with(|| CurveInProgress {
                nodes: Vec::new(),
                smooth: false,
                dragging_handle: false,
                bspline: false,
                ..Default::default()
            });
            c.nodes.push((p, None));
            if c.nodes.len() >= 2 {
                self.finish_curve();
            }
            return;
        }
        if response.drag_started_by(PointerButton::Primary) {
            self.curve = None;
            self.drag = Drag::Freehand { points: vec![p] };
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::Freehand { points } = &mut self.drag {
                if points
                    .last()
                    .map(|l| (*l - p).hypot() > 0.2)
                    .unwrap_or(true)
                {
                    points.push(p);
                }
            }
        }
    }

    /// Bezier/Pen: click places a node with a straight segment to it (a
    /// node that was pulled before becomes a cusp: the segment leaves it
    /// straight), click-and-drag pulls out a smooth handle (C while
    /// pulling makes the node a cusp: only the outgoing handle follows;
    /// Alt moves the node). Clicking the last node again makes it a cusp.
    /// Polyline and 2-point line place straight segments. Double-click or
    /// Enter finishes; a press on the first node closes the curve, and
    /// dragging from it shapes the closing segment only.
    fn curve_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        // With no curve being drawn, the Pen and Bezier tools edit the
        // selected curve: a click on a segment adds a node, a node is
        // dragged like with the Shape tool.
        if self.pen_edit_input(response, p, mods) {
            return;
        }
        let raw = p;
        let p = self.snap_point(p);
        // Ctrl: the new node in steps of the constrain angle from the last
        // one (a dragged handle, from its node).
        let handle_from = self
            .curve
            .as_ref()
            .filter(|c| c.dragging_handle)
            .and_then(|c| c.nodes.last().map(|n| n.0));
        let p = match handle_from {
            Some(node) if mods.ctrl => constrain_from(node, p, self.settings.constrain_angle),
            Some(_) => p,
            None => self.curve_point(p, mods.ctrl),
        };
        // B-Spline: each click (or the start of a drag) places a control
        // point; there are no handles to pull.
        let bspline = self.tool == Tool::BSpline;
        let smooth = matches!(self.tool, Tool::Bezier | Tool::Pen);
        if response.double_clicked_by(PointerButton::Primary) {
            self.finish_curve();
            return;
        }
        // Every press that changes the curve is a step Ctrl+Z takes back.
        if response.clicked_by(PointerButton::Primary)
            || response.drag_started_by(PointerButton::Primary)
        {
            self.curve_step();
        }
        let reach = self.curve_end_reach_px() / self.view.zoom as f64;
        let near = |q: Point| (q - raw).hypot() <= reach;
        let start_close = self
            .curve
            .as_ref()
            .is_some_and(|c| c.nodes.len() >= 3 && c.nodes.first().is_some_and(|n| near(n.0)));
        let on_last = smooth
            && self
                .curve
                .as_ref()
                .is_some_and(|c| c.nodes.len() >= 2 && c.nodes.last().is_some_and(|n| near(n.0)));
        if response.clicked_by(PointerButton::Primary) {
            if start_close {
                // A click on the first node closes the curve; the closing
                // segment leaves the last node straight.
                if let Some(mut c) = self.curve.take() {
                    c.cusp_last();
                    c.closing = None;
                    self.close_curve(c);
                }
                return;
            }
            if on_last {
                // Clicking the last node again makes it a cusp.
                if let Some(c) = &mut self.curve {
                    c.cusp_last();
                    c.dragging_handle = false;
                }
                return;
            }
            // The Pen's automatic adding and deleting: a click on a node of
            // the curve being drawn deletes it, on a segment adds a node.
            if let Some(auto) = self.pen_auto_at(raw) {
                if let Some(c) = &mut self.curve {
                    match auto {
                        crate::pen_look::PenAuto::Delete(i) => {
                            c.delete_node(i);
                        }
                        crate::pen_look::PenAuto::Add(k, t) => {
                            c.insert_node(k, t);
                        }
                    }
                    c.dragging_handle = false;
                }
                return;
            }
        }
        if response.drag_started_by(PointerButton::Primary) && start_close {
            // Dragging from the first node shapes the closing segment.
            if let Some(c) = &mut self.curve {
                c.closing = Some(raw);
                c.dragging_handle = false;
            }
            return;
        }
        if response.drag_started_by(PointerButton::Primary)
            || response.clicked_by(PointerButton::Primary)
        {
            let c = self.curve.get_or_insert_with(|| CurveInProgress {
                nodes: Vec::new(),
                smooth,
                dragging_handle: false,
                bspline,
                ..Default::default()
            });
            c.smooth = smooth;
            c.bspline = bspline;
            let drag = response.drag_started_by(PointerButton::Primary);
            if !drag && smooth {
                // A click: the segment to the new node is straight, so a
                // node pulled before becomes a cusp.
                c.cusp_last();
            }
            if drag || !c.dragging_handle {
                c.nodes.push((p, None));
            }
            c.dragging_handle = drag;
            c.alt_from = None;
            if self.tool == Tool::TwoPointLine && c.nodes.len() == 2 {
                self.finish_curve();
            }
        }
        if response.dragged_by(PointerButton::Primary) {
            // C held (pressed before or during the pull): a cusp; it stays
            // one after C is let go.
            let c_pressed = response.ctx.input(|i| i.key_down(egui::Key::C));
            if let Some(c) = &mut self.curve {
                if c.closing.is_some() {
                    c.closing = Some(raw);
                } else if c.dragging_handle && smooth {
                    if c_pressed {
                        c.freeze_last_in();
                    }
                    if mods.alt {
                        // Alt: the node moves with its handles.
                        let from = c.alt_from.unwrap_or(p);
                        let d = p - from;
                        let i = c.nodes.len() - 1;
                        if let Some(last) = c.nodes.last_mut() {
                            last.0 += d;
                            last.1 = last.1.map(|h| h + d);
                        }
                        if let Some(Some(h)) = c.in_handles.get_mut(&i) {
                            *h += d;
                        }
                        c.alt_from = Some(p);
                    } else {
                        c.alt_from = None;
                        if let Some(last) = c.nodes.last_mut() {
                            last.1 = Some(p);
                        }
                    }
                }
            }
        }
        if response.drag_stopped_by(PointerButton::Primary) {
            if self.curve.as_ref().is_some_and(|c| c.closing.is_some()) {
                if let Some(mut c) = self.curve.take() {
                    c.closing = Some(raw);
                    self.close_curve(c);
                }
                return;
            }
            if let Some(c) = &mut self.curve {
                c.dragging_handle = false;
                c.alt_from = None;
            }
        }
    }

    /// The curve being drawn, closed onto its first node, becomes a
    /// curve object and is selected.
    fn close_curve(&mut self, c: CurveInProgress) {
        self.curve_steps.clear();
        let path = c.path_closed(None, true);
        if let Some(id) = self.new_shape(ShapeKind::Path { path, closed: true }) {
            self.select(vec![id]);
        }
    }

    /// The Pen and Bezier tools with no curve being drawn and one curve
    /// selected: a click on one of its segments adds a node there, and a
    /// press on one of its nodes (not an end node, which continues the
    /// curve) drags it as the Shape tool does. True when it took the
    /// input.
    fn pen_edit_input(&mut self, response: &Response, p: Point, mods: Modifiers) -> bool {
        if self.pen_editing {
            self.shape_input(response, p, mods);
            // The drag ends with the button (`canvas_input` ends it).
            if response.drag_stopped() || !response.ctx.input(|i| i.pointer.primary_down()) {
                self.pen_editing = false;
            }
            return true;
        }
        if !matches!(self.tool, Tool::Bezier | Tool::Pen) || self.curve.is_some() {
            return false;
        }
        match self.pen_edit_at(p) {
            Some(PenEdit::Node(..)) if response.drag_started_by(PointerButton::Primary) => {
                self.pen_editing = true;
                self.shape_input(response, p, mods);
                true
            }
            Some(PenEdit::Node(id, index)) if response.clicked_by(PointerButton::Primary) => {
                if self.pen_deletes_nodes() {
                    self.pen_delete_node(id, index);
                }
                true
            }
            Some(PenEdit::Segment(id, index, t)) if response.clicked_by(PointerButton::Primary) => {
                if let Some(ShapeKind::Path { path, closed }) =
                    self.doc().find_shape(id).map(|s| s.kind.clone())
                {
                    let np = tracedraw_core::nodes::add_node(&path, index, t);
                    if self
                        .engine
                        .run_with_label(
                            &Command::SetShapeKind {
                                shape: id,
                                kind: ShapeKind::Path { path: np, closed },
                            },
                            "Add Node",
                        )
                        .is_ok()
                    {
                        self.node_selection = vec![(id, index)];
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// What a Pen or Bezier press at `p` would edit on the selected curve
    /// when no curve is being drawn: a node (not an end node of an open
    /// curve, where a new line continues it) or a segment.
    pub(crate) fn pen_edit_at(&self, p: Point) -> Option<PenEdit> {
        if !matches!(self.tool, Tool::Bezier | Tool::Pen) || self.curve.is_some() {
            return None;
        }
        let [id] = self.selection.as_slice() else {
            return None;
        };
        let shape = self.doc().shape_on_page(*id)?;
        let ShapeKind::Path { path, .. } = &shape.kind else {
            return None;
        };
        if self.on_join_end(p) {
            return None;
        }
        let tol = self.settings.nodes.pick_px() / self.view.zoom as f64;
        let lp = shape.transform.inverse() * p;
        if let Some(n) = tracedraw_core::nodes::nodes(path)
            .iter()
            .map(|n| (n.index, (n.pos - lp).hypot()))
            .filter(|(_, d)| *d <= tol)
            .min_by(|a, b| a.1.total_cmp(&b.1))
        {
            return Some(PenEdit::Node(*id, n.0));
        }
        // The Pen adds nodes only with automatic adding and deleting on;
        // the Bezier tool always does.
        if self.tool == Tool::Pen && !self.settings.curves.pen_auto_add_delete {
            return None;
        }
        let (index, t, d) = tracedraw_core::nodes::nearest_segment(path, lp)?;
        (d <= tol).then_some(PenEdit::Segment(*id, index, t))
    }

    /// The Pen with automatic adding and deleting on: a click on a node
    /// of the selected curve deletes it.
    pub(crate) fn pen_deletes_nodes(&self) -> bool {
        self.tool == Tool::Pen && self.settings.curves.pen_auto_add_delete
    }

    /// Delete node `index` of curve `id` (one undo step), keeping at least
    /// two nodes.
    fn pen_delete_node(&mut self, id: tracedraw_core::ShapeId, index: usize) {
        let Some(ShapeKind::Path { path, closed }) =
            self.doc().find_shape(id).map(|s| s.kind.clone())
        else {
            return;
        };
        if tracedraw_core::nodes::nodes(&path).len() <= 2 {
            return;
        }
        let np = tracedraw_core::nodes::delete_node(&path, index);
        if self
            .engine
            .run_with_label(
                &Command::SetShapeKind {
                    shape: id,
                    kind: ShapeKind::Path { path: np, closed },
                },
                "Delete Node",
            )
            .is_ok()
        {
            self.node_selection.clear();
        }
    }

    fn fill_input(&mut self, response: &Response, p: Point) {
        if response.drag_started_by(PointerButton::Primary) {
            if let Some(h) = self.fountain_handle_at(p) {
                self.drag = Drag::FountainHandle { handle: h };
                return;
            }
            if let Some(id) = self.hit_test_inside(p) {
                self.select(vec![id]);
                self.drag = Drag::FillGradient {
                    shape: id,
                    start: p,
                    current: p,
                };
            }
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::FountainHandle { handle } = self.drag {
                self.drag_fountain_handle(handle, p);
                return;
            }
            if let Drag::FillGradient { current, .. } = &mut self.drag {
                *current = p;
            }
        }
        if response.clicked_by(PointerButton::Primary) {
            if self.fountain_handle_at(p).is_some() {
                return;
            }
            if let Some(id) = self.hit_test_inside(p) {
                self.select(vec![id]);
                let fill = match self.default_fill() {
                    Fill::None => Fill::Solid(Color::cmyk_pct(0.0, 0.0, 0.0, 20.0)),
                    f => f,
                };
                self.run(Command::SetFill {
                    shapes: vec![id],
                    fill,
                });
            }
        }
    }

    pub fn end_drag(&mut self) {
        let drag = std::mem::replace(&mut self.drag, Drag::None);
        match drag {
            Drag::Anchor { .. } => {}
            Drag::Box {
                start,
                current,
                from_center,
            } => {
                let r = App::box_rect(start, current, from_center);
                // The knife cuts along the dragged line, in its direction;
                // shape tools take the normalised box.
                let (a, b) = if self.tool == Tool::Knife {
                    (start, current)
                } else {
                    (Point::new(r.x0, r.y0), Point::new(r.x1, r.y1))
                };
                if !self.finish_tools2_box(a, b) {
                    self.create_box_shape(a, b);
                }
            }
            Drag::Connector { from, current, .. } if self.tool == Tool::Blend => {
                if let Some(to) = self.hit_test_inside(current).filter(|id| *id != from) {
                    self.selection = vec![from, to];
                    let (steps, o, c, r) = (
                        self.blend_steps,
                        self.blend_accel_objects,
                        self.blend_accel_colors,
                        self.blend_rotation,
                    );
                    self.blend_selection(steps, o, c, r);
                }
            }
            Drag::Connector {
                from,
                start,
                current,
            } if self.tool == Tool::RightAngleConnector => {
                self.finish_elbow_connector(from, start, current, false)
            }
            Drag::Connector {
                from,
                start,
                current,
            } if self.tool == Tool::RoundedConnector => {
                self.finish_elbow_connector(from, start, current, true)
            }
            Drag::Connector {
                from,
                start,
                current,
            } => self.finish_connector(from, start, current),
            Drag::Freehand { points } if self.tool == Tool::Eraser => self.erase_along(&points),
            Drag::Freehand { points } if self.tool == Tool::Knife => self.knife_cut(&points),
            Drag::Freehand { points } if self.tool == Tool::ShapeRecognition => {
                self.finish_shape_recognition(points)
            }
            Drag::Freehand { points } if self.tool == Tool::Sketch => self.finish_sketch(points),
            Drag::ThreePointBase { start, current } => {
                if (current - start).hypot() > 0.05 {
                    self.three_point_base = Some((start, current));
                }
            }
            Drag::TextSelect => {}
            Drag::TextFrame { start, current } => {
                let r = Rect::from_points(start, current);
                if r.width() > 2.0 && r.height() > 2.0 {
                    self.start_text(
                        Point::new(r.x0, r.y0),
                        Some(tracedraw_core::geometry::Size::new(r.width(), r.height())),
                    );
                }
            }
            // Dropped on the document palette: the selection's colours
            // join it and the objects stay where they were.
            Drag::Move { .. } if self.over_document_palette() => {
                self.drag_copy = false;
                self.palette_from_selection();
            }
            Drag::Move { total, .. } => {
                if total.hypot() > 1e-6 {
                    self.finish_transform(Affine::translate(total));
                }
            }
            Drag::AlongPath { start, current } => self.finish_along_path(start, current),
            Drag::Pivot => {}
            Drag::Scale { .. } | Drag::Rotate { .. } | Drag::Skew { .. } => {
                if let Some(t) = self.preview_transform_of(&drag) {
                    self.finish_transform(t);
                }
            }
            Drag::Marquee {
                start,
                current,
                touching,
            } => {
                let r = Rect::from_points(start, current);
                if r.width() > 0.1 || r.height() > 0.1 {
                    self.select_in_region(crate::select_region::Region::Rect(r), touching);
                }
            }
            Drag::Lasso { points, touching } => {
                if points.len() >= 3 {
                    self.select_in_region(crate::select_region::Region::Polygon(&points), touching);
                }
            }
            Drag::Freehand { points } if self.tool == Tool::PainterlyBrush => {
                self.add_painterly_stroke(&points);
            }
            Drag::Freehand { points } if self.tool == Tool::BrushStrokes => {
                let shapes = self.media_shapes(&points);
                if let Some(layer) = self.active_layer() {
                    let ids: Vec<ShapeId> = shapes.iter().map(|s| s.id).collect();
                    let cmds: Vec<Command> = shapes
                        .into_iter()
                        .map(|s| Command::AddShape { layer, shape: s })
                        .collect();
                    if !cmds.is_empty() {
                        let _ = self.engine.run_batch("Brush Strokes", &cmds);
                        self.selection = ids;
                    }
                }
            }
            Drag::Freehand { points } => {
                // Smoothing (Options > Tools > Curves, the property bar)
                // sets how far the stroke may be simplified, on screen.
                let tol = self.settings.curves.freehand_tolerance_px() / self.view.zoom as f64;
                let pts = tracedraw_core::geometry::simplify(&points, tol.max(0.02));
                if pts.len() >= 2 {
                    let path = tracedraw_core::geometry::smooth_path(&pts, false);
                    self.add_drawn_line(path);
                }
            }
            Drag::FillGradient {
                shape,
                start,
                current,
            } if self.tool == Tool::Transparency => {
                if (current - start).hypot() > 0.5 {
                    let angle = (current - start).atan2().to_degrees();
                    self.transparency_default.kind = 1;
                    self.transparency_default.mask =
                        Fill::linear(Color::WHITE, Color::BLACK, angle);
                    self.selection = vec![shape];
                    let s = self.transparency_default.clone();
                    self.apply_transparency(&s);
                }
            }
            Drag::FillGradient {
                shape,
                start,
                current,
            } => {
                if (current - start).hypot() > 0.5 {
                    let from = match &self.default_fill() {
                        Fill::Solid(c) => *c,
                        _ => Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
                    };
                    let angle = (current - start).atan2().to_degrees();
                    self.run(Command::SetFill {
                        shapes: vec![shape],
                        fill: Fill::linear(from, Color::WHITE, angle),
                    });
                }
            }
            Drag::ZoomBox { start, current } => {
                let r = Rect::from_points(start, current);
                if r.width() > 0.5 && r.height() > 0.5 {
                    self.view.fit(r, self.canvas_rect);
                }
            }
            Drag::NodeMarquee { start, current } => self.finish_node_marquee(start, current),
            Drag::NodeLasso { points } => self.finish_node_lasso(&points),
            Drag::NodeMove { begun: true, .. } => self.join_dropped_end_node(),
            Drag::NodeMove { .. }
            | Drag::NodeTransform { .. }
            | Drag::SegmentBend { .. }
            | Drag::TextChars { .. }
            | Drag::TextSpacing { .. } => {}
            Drag::NewGuide { .. } => self.finish_guide_drag(),
            d @ (Drag::MoveGuide { .. } | Drag::RotateGuide { .. }) => {
                // Released outside the window (no pointer): dropped off.
                let at = self
                    .pointer_page
                    .map(|p| self.view.to_screen(p))
                    .unwrap_or(egui::pos2(f32::NAN, f32::NAN));
                self.drag = d;
                self.finish_guide_move(at);
            }
            Drag::FountainHandle { .. }
            | Drag::ContourDrag { .. }
            | Drag::Shadow { .. }
            | Drag::ShadowHandle { .. }
            | Drag::RulerOrigin { .. }
            | Drag::Node { .. }
            | Drag::KindNode { .. }
            | Drag::Handle { .. }
            | Drag::None => {}
        }
    }

    /// Transform the selection would get if the drag ended now (for preview
    /// and for commit). Translation is handled separately by `Drag::Move`.
    pub fn preview_transform_of(&self, drag: &Drag) -> Option<Affine> {
        match drag {
            Drag::Scale {
                handle,
                anchor,
                start_bounds,
                current,
                mods,
                ..
            } => {
                let b = *start_bounds;
                // From the centre (Shift) the handle's side is half as far
                // from the fixed point.
                let (anchor, half) = if mods.from_center {
                    (b.center(), 0.5)
                } else {
                    (*anchor, 1.0)
                };
                let (w, h) = ((b.width() * half).max(1e-6), (b.height() * half).max(1e-6));
                let (mut sx, mut sy) = (1.0, 1.0);
                match handle {
                    Handle::E | Handle::W => {
                        sx = (current.x - anchor.x) / (if *handle == Handle::E { w } else { -w })
                    }
                    Handle::N | Handle::S => {
                        sy = (current.y - anchor.y) / (if *handle == Handle::N { h } else { -h })
                    }
                    Handle::NE => {
                        sx = (current.x - anchor.x) / w;
                        sy = (current.y - anchor.y) / h;
                    }
                    Handle::NW => {
                        sx = (anchor.x - current.x) / w;
                        sy = (current.y - anchor.y) / h;
                    }
                    Handle::SE => {
                        sx = (current.x - anchor.x) / w;
                        sy = (anchor.y - current.y) / h;
                    }
                    Handle::SW => {
                        sx = (anchor.x - current.x) / w;
                        sy = (anchor.y - current.y) / h;
                    }
                }
                if handle.is_corner() && !self.rotate_mode && !mods.free {
                    // Corners scale proportionally (Alt: freely).
                    let s = if sx.abs() > sy.abs() {
                        sx.abs()
                    } else {
                        sy.abs()
                    };
                    sx = s * sx.signum();
                    sy = s * sy.signum();
                }
                if mods.steps {
                    // Ctrl: whole steps of 100 %, so crossing the other
                    // side gives the mirror image at the same size.
                    let step = |v: f64| v.signum() * v.abs().round().max(1.0);
                    sx = step(sx);
                    sy = step(sy);
                }
                if !(sx.is_finite() && sy.is_finite()) || sx.abs() < 1e-3 || sy.abs() < 1e-3 {
                    return None;
                }
                Some(
                    Affine::translate(anchor.to_vec2())
                        * Affine::scale_non_uniform(sx, sy)
                        * Affine::translate(-anchor.to_vec2()),
                )
            }
            Drag::Rotate {
                center,
                start_angle,
                current_angle,
            } => {
                let a = current_angle - start_angle;
                Some(
                    Affine::translate(center.to_vec2())
                        * Affine::rotate(a)
                        * Affine::translate(-center.to_vec2()),
                )
            }
            Drag::Skew {
                handle,
                start_bounds,
                start,
                current,
                mods,
            } => skew_transform(
                *handle,
                *start_bounds,
                *current - *start,
                *mods,
                self.settings.constrain_angle,
            ),
            _ => None,
        }
    }

    pub fn keyboard(&mut self, ctx: &egui::Context) {
        // A macro's message, input box, modal form or error owns the
        // keyboard (see ui/macros_ui.rs); Esc stops a running macro.
        if self.macros.modal() {
            return;
        }
        if self.macros.run.is_some()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input(|i| i.key_pressed(Key::Escape))
        {
            self.stop_macro();
            return;
        }
        let dialog_open = !matches!(self.dialog, crate::ui::dialogs::Dialog::None);
        // Table cell typing has priority while a cell is active (not
        // behind a dialog, which owns the keyboard).
        if !dialog_open
            && self.table_edit.is_some()
            && self.tool == Tool::Table
            && self.table_keyboard(ctx)
        {
            return;
        }
        // Text typing has priority.
        if self.text_edit.is_some() && !dialog_open {
            self.text_keyboard(ctx);
            return;
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        // An open menu takes the keys: its rows' access keys, arrows, Esc.
        if egui::Popup::is_any_open(ctx) {
            return;
        }
        // A modal dialog owns the keyboard: Esc closes it, the rest is
        // not interpreted as shortcuts.
        if !matches!(self.dialog, crate::ui::dialogs::Dialog::None) {
            // A command waiting for its shortcut (Options > Customization >
            // Commands): Esc stops waiting, other keys are the shortcut.
            if self.shortcut_editor.capture.is_some() {
                // Taken from the input so the dialog does not close too.
                if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                    self.shortcut_editor.capture = None;
                }
                return;
            }
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.dialog = crate::ui::dialogs::Dialog::None;
                // Esc is Cancel for the options dialogs.
                if let Some(s) = self.options_snapshot.take() {
                    s.restore(self);
                }
            }
            return;
        }
        // Esc during a move, scale, rotation or skew cancels it: nothing
        // changes and the selection stays.
        if self.transforming() && ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.drag = Drag::None;
            self.drag_copy = false;
            return;
        }
        let input = ctx.input(|i| i.clone());
        // Each key press with the modifiers held when it happened (a quick
        // Ctrl+E can be released before the frame that sees it). Shortcuts
        // match the modifiers exactly, so Ctrl+Shift+Z is not also Ctrl+Z;
        // only + and - ignore Shift, which some layouts need to type them.
        let presses: Vec<(Key, Modifiers)> = input
            .events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect();
        let pressed = |k: Key, m: Modifiers| {
            presses.iter().any(|(key, mods)| {
                *key == k
                    && if matches!(k, Key::Plus | Key::Equals | Key::Minus) {
                        mods.matches_logically(m)
                    } else {
                        mods.matches_exact(m)
                    }
            })
        };
        let cmd = Modifiers::COMMAND;

        // Menu commands with a shortcut of the user's (Options >
        // Customization > Commands) come first: they may take a built-in
        // shortcut over.
        for (key, mods) in &presses {
            let text = crate::shortcuts::format(*mods, *key);
            if let Some(id) = self
                .settings
                .shortcuts
                .iter()
                .find(|(id, v)| id.starts_with("menu.") && crate::shortcuts::same(v, &text))
                .map(|(id, _)| id.clone())
            {
                self.pending_command = Some(id);
                return;
            }
        }
        // Built-in shortcuts the user removed (Options > Customization >
        // Commands, Delete): their keys do nothing.
        if self.settings.shortcuts.iter().any(|(_, v)| v.is_empty()) {
            if self.shortcut_editor.entries.is_empty() {
                self.shortcut_editor.entries = crate::ui::menus::commands(self, ctx);
            }
            let removed = presses.iter().any(|(key, mods)| {
                let text = crate::shortcuts::format(*mods, *key);
                self.shortcut_editor.entries.iter().any(|e| {
                    crate::shortcuts::same(&e.shortcut, &text)
                        && self
                            .settings
                            .shortcuts
                            .iter()
                            .any(|(id, v)| id == &e.key && v.is_empty())
                })
            });
            if removed {
                return;
            }
        }

        // Tools > Macros > Run Macro (Alt+Shift+F8: Alt+F8 is the
        // Transformations panel's Rotate) and Macro Editor.
        if pressed(Key::F8, Modifiers::ALT | Modifiers::SHIFT) {
            self.open_run_macro();
            return;
        }
        if pressed(Key::F11, Modifiers::ALT) {
            self.open_macro_editor();
            return;
        }
        // The Welcome Screen alone (no drawing open): only the commands
        // that need no drawing.
        if !self.has_document() {
            if pressed(Key::N, cmd) {
                self.request_new_document();
            }
            if pressed(Key::O, cmd) {
                self.open_dialog();
            }
            if pressed(Key::J, cmd) {
                self.dialog = crate::ui::dialogs::Dialog::Options;
            }
            if pressed(Key::F4, Modifiers::ALT) {
                self.request_exit();
            }
            return;
        }

        if pressed(Key::Z, cmd) {
            self.undo();
        }
        if pressed(Key::Z, cmd | Modifiers::SHIFT) {
            self.redo();
        }
        if pressed(Key::S, cmd) {
            self.save(false);
        }
        if pressed(Key::S, cmd | Modifiers::SHIFT) {
            self.save(true);
        }
        if pressed(Key::S, Modifiers::ALT) && !self.selection.is_empty() {
            self.create_symmetry();
        }
        if pressed(Key::E, Modifiers::ALT) {
            self.edit_symmetry();
        }
        if pressed(Key::E, Modifiers::ALT | Modifiers::SHIFT) {
            self.finish_symmetry_edit();
        }
        if pressed(Key::S, Modifiers::ALT | Modifiers::SHIFT) {
            self.remove_symmetry();
        }
        if pressed(Key::X, Modifiers::ALT) && self.selected_symmetry().is_some() {
            self.break_symmetry_link();
        }
        // Docker shortcuts (Ctrl+F9 Contour, Alt+F3 Lens, Alt+Enter
        // Properties, ...): toggle.
        for tab in crate::app::DockerTab::ALL {
            let label = tab.shortcut();
            // Transformations opens on a page, below.
            if label.is_empty() || tab == crate::app::DockerTab::Transformations {
                continue;
            }
            let Some((k, shift)) = Tool::parse_shortcut(label) else {
                continue;
            };
            let mut m = Modifiers::NONE;
            if label.contains("Ctrl+") {
                m |= cmd;
            }
            if label.contains("Alt+") {
                m |= Modifiers::ALT;
            }
            if shift {
                m |= Modifiers::SHIFT;
            }
            if pressed(k, m) {
                self.toggle_docker(tab);
            }
        }
        // Alt+F7 to Alt+F10: the Transformations docker on Position,
        // Rotate, Scale and Mirror or Size.
        for (k, tab) in [
            (Key::F7, crate::app::TransformTab::Position),
            (Key::F8, crate::app::TransformTab::Rotate),
            (Key::F9, crate::app::TransformTab::Scale),
            (Key::F10, crate::app::TransformTab::Size),
        ] {
            if pressed(k, Modifiers::ALT) {
                crate::ui::transform_docker::open(self, tab, true);
            }
        }
        if pressed(Key::O, cmd) {
            self.open_dialog();
        }
        if pressed(Key::N, cmd) {
            self.request_new_document();
        }
        if pressed(Key::E, cmd) {
            self.export();
        }
        if pressed(Key::I, cmd) {
            self.import();
        }
        if pressed(Key::G, cmd) {
            self.group_selection();
        }
        // Ctrl+Shift+G: unselected nodes filled or hollow.
        if pressed(Key::G, cmd | Modifiers::SHIFT) {
            self.settings.nodes.unselected_filled = !self.settings.nodes.unselected_filled;
        }
        if pressed(Key::U, cmd) {
            self.ungroup_selection();
        }
        // Bitmap adjustments of the Effects menu.
        if self
            .selected_shapes()
            .iter()
            .any(|s| matches!(s.kind, ShapeKind::Bitmap { .. }))
        {
            for (key, mods, id) in [
                (Key::B, cmd, "brightness_contrast_intensity"),
                (Key::B, cmd | Modifiers::SHIFT, "color_balance"),
                (Key::U, cmd | Modifiers::SHIFT, "hue_saturation_lightness"),
            ] {
                if pressed(key, mods) {
                    crate::ui::effect_dialog::choose_effect(self, id);
                }
            }
        }
        if pressed(Key::Q, cmd | Modifiers::SHIFT) {
            self.convert_outline_to_object();
        } else if pressed(Key::Q, cmd) {
            self.convert_to_curves();
        }
        if pressed(Key::A, cmd) {
            if self.tool == Tool::Shape && !self.selection.is_empty() {
                self.select_all_nodes();
            } else {
                self.select_all();
            }
        }
        if pressed(Key::L, cmd) {
            self.combine();
        }
        if pressed(Key::J, cmd) {
            self.dialog = crate::ui::dialogs::Dialog::Options;
        }
        // With the Shape tool, Tab steps through the nodes of the curve.
        let shape_tool = self.tool == Tool::Shape;
        if pressed(Key::Tab, Modifiers::NONE) && !(shape_tool && self.cycle_node(true)) {
            self.cycle_selection(true);
        }
        if pressed(Key::Tab, Modifiers::SHIFT) && !(shape_tool && self.cycle_node(false)) {
            self.cycle_selection(false);
        }
        if pressed(Key::K, cmd) {
            self.break_apart();
        }
        // Selection groups 0 to 9: Ctrl+number stores the selection under
        // the number, Alt+number adds the selection to it, the number
        // selects the group again (a second time also zooms to it). With
        // text selected Ctrl+8 and Ctrl+2 keep changing the text size.
        let text_selected = self
            .selected_shapes()
            .iter()
            .any(|s| matches!(s.kind, ShapeKind::Text { .. }));
        for (n, k) in [
            Key::Num0,
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
            Key::Num8,
            Key::Num9,
        ]
        .into_iter()
        .enumerate()
        {
            let n = n as u8;
            if pressed(k, cmd)
                && !self.selection.is_empty()
                && !(text_selected && (n == 2 || n == 8))
            {
                self.set_selection_group(n, self.selection.clone());
            }
            if pressed(k, Modifiers::ALT) && !self.selection.is_empty() {
                let mut ids = self.selection_group(n);
                for id in self.selection.clone() {
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
                self.set_selection_group(n, ids);
            }
            if pressed(k, Modifiers::NONE) {
                self.recall_selection_group(n);
            }
        }
        // With the Shape tool and nodes selected, the segments between
        // them are copied, cut or duplicated.
        let segments = shape_tool && !self.node_selection.is_empty();
        // The windowing layer turns Ctrl+C, Ctrl+X and Ctrl+V (and
        // Ctrl+Insert, Shift+Delete and Shift+Insert) into copy, cut and
        // paste events instead of key presses; a paste event comes only
        // while the system clipboard holds text, which a copy here puts
        // there. Either form counts, once per frame.
        let (copy_event, cut_event, paste_text) = clipboard_events(&input.events);
        let paste_event = paste_text.is_some();
        let pasted_without_text = paste_without_text(&input.events, &mut self.paste_seen);
        let shift_held = input.modifiers.shift;
        if (pressed(Key::C, cmd) || copy_event) && !(segments && self.copy_segments()) {
            self.copy_with_system();
        }
        if (pressed(Key::X, cmd) || cut_event) && !(segments && self.cut_segments()) {
            self.cut();
        }
        if pressed(Key::V, cmd) || (paste_event && !shift_held) || pasted_without_text {
            self.paste_with_event_text(paste_text);
        } else if paste_event && shift_held {
            self.paste_in_view();
        }
        if pressed(Key::D, cmd) && !(segments && self.duplicate_segments()) {
            self.duplicate();
        }
        // Ctrl+Space: the Pick tool, whatever tool is active.
        if pressed(Key::Space, cmd) && self.tool != Tool::Pick {
            self.set_tool(Tool::Pick);
        }
        // Order: Ctrl+Home/End to the front/back of the page, Shift+PgUp/PgDn
        // within the layer, Ctrl+PgUp/PgDn one step.
        if pressed(Key::Home, cmd) || pressed(Key::PageUp, Modifiers::SHIFT) {
            self.order(0);
        }
        if pressed(Key::End, cmd) || pressed(Key::PageDown, Modifiers::SHIFT) {
            self.order(3);
        }
        if pressed(Key::PageUp, cmd) {
            self.order(1);
        }
        if pressed(Key::PageDown, cmd) {
            self.order(2);
        }
        // The rest of the menu shortcuts.
        if pressed(Key::P, cmd) {
            self.print();
        }
        if pressed(Key::R, cmd) {
            self.repeat_last();
        }
        if pressed(Key::V, cmd | Modifiers::SHIFT) && !paste_event {
            self.paste_in_view();
        }
        if pressed(Key::M, cmd) {
            self.table_op(crate::table::TableOp::Merge);
        }
        if pressed(Key::W, cmd) {
            self.raster.borrow_mut().invalidate();
        }
        if pressed(Key::F4, cmd) {
            self.close_document();
        }
        if pressed(Key::F4, Modifiers::ALT) {
            self.request_exit();
        }
        if pressed(Key::F12, Modifiers::ALT) {
            self.align_chars_to_baseline();
        }
        // F11: the fill editor (Properties docker, Fill section). F3: zoom out.
        if pressed(Key::F11, Modifiers::NONE) {
            self.open_fill_editor();
        }
        if pressed(Key::F3, Modifiers::NONE) {
            self.zoom_step(false);
        }
        if pressed(Key::T, cmd | Modifiers::SHIFT) {
            self.edit_selected_text();
        }
        if pressed(Key::Q, Modifiers::ALT) {
            self.snap.off = !self.snap.off;
        }
        if pressed(Key::Z, Modifiers::ALT) {
            self.snap.objects = !self.snap.objects;
        }
        if pressed(Key::Y, cmd) {
            self.snap.grid = !self.snap.grid;
        }
        if pressed(Key::A, Modifiers::ALT | Modifiers::SHIFT) {
            self.snap.alignment_guides = !self.snap.alignment_guides;
        }
        if pressed(Key::D, Modifiers::ALT | Modifiers::SHIFT) {
            self.snap.dynamic_guides = !self.snap.dynamic_guides;
        }
        if pressed(Key::R, Modifiers::ALT | Modifiers::SHIFT) {
            self.show_rulers = !self.show_rulers;
        }
        // Ctrl+8 and Ctrl+2: selected text one keyboard text increment
        // larger or smaller (Options > Text).
        if self
            .selected_shapes()
            .iter()
            .any(|s| matches!(s.kind, ShapeKind::Text { .. }))
        {
            if pressed(Key::Num8, cmd) {
                self.step_text_size(true);
            }
            if pressed(Key::Num2, cmd) {
                self.step_text_size(false);
            }
        }
        if pressed(Key::Plus, cmd) || pressed(Key::Equals, cmd) {
            self.zoom_step(true);
        }
        if pressed(Key::Minus, cmd) {
            self.zoom_step(false);
        }
        if pressed(Key::Delete, Modifiers::NONE) || pressed(Key::Backspace, Modifiers::NONE) {
            if matches!(self.tool, Tool::MeshFill | Tool::Shape)
                && self.selected_mesh_node().is_some()
            {
                self.delete_mesh_lines();
            } else if matches!(self.tool, Tool::Envelope | Tool::Shape)
                && self.selected_envelope_node().is_some()
            {
                // The node goes; a corner stays (the status bar says so).
                self.delete_selected_envelope_node();
            } else if self.tool == Tool::Shape && self.delete_chosen_segment() {
                // A segment clicked on went; its nodes stay.
            } else if self.tool == Tool::Shape && !self.node_selection.is_empty() {
                self.delete_selected_nodes();
            } else if self.tool == Tool::VariableOutline && self.delete_variable_outline_node() {
                // The width node went, the object stays.
            } else if self.tool == Tool::AnchorEditing && self.delete_selected_anchor() {
                // The anchor went, the object stays.
            } else if !self.selected_guides.is_empty() {
                if !self.delete_selected_guides() {
                    self.status = crate::i18n::tr("status.guideline_locked");
                }
            } else {
                self.delete_selection();
            }
        }
        if pressed(Key::F4, Modifiers::SHIFT) {
            self.zoom_to_page();
        }
        if pressed(Key::F4, Modifiers::NONE) {
            self.zoom_to_fit();
        }
        if pressed(Key::F2, Modifiers::SHIFT) {
            self.zoom_to_selection();
        }
        if pressed(Key::F2, Modifiers::NONE) {
            self.set_tool(Tool::Zoom);
        }
        // + and -: with the Shape tool and nodes chosen, a node in the
        // middle of each segment ending at them, and deleting them; with a
        // Pick tool and a selection, + duplicates it in place; otherwise
        // a zoom step about the window's centre.
        let node_keys = self.tool == Tool::Shape && !self.node_selection.is_empty();
        let pick_selection =
            matches!(self.tool, Tool::Pick | Tool::FreeformPick) && !self.selection.is_empty();
        if pressed(Key::Plus, Modifiers::NONE) || pressed(Key::Equals, Modifiers::NONE) {
            if node_keys {
                self.add_node_midpoints();
            } else if pick_selection && pressed(Key::Plus, Modifiers::NONE) {
                self.duplicate_in_place();
                self.status = crate::i18n::tr("status.duplicated_in_place");
            } else {
                let c = self.canvas_rect.center();
                self.view.zoom_at(c, 1.25);
            }
        }
        if pressed(Key::Minus, Modifiers::NONE) {
            if node_keys {
                self.delete_selected_nodes();
            } else {
                let c = self.canvas_rect.center();
                self.view.zoom_at(c, 0.8);
            }
        }
        if pressed(Key::Enter, Modifiers::NONE) {
            if self.tool == Tool::Crop && self.crop_area.is_some() {
                self.apply_crop();
            }
            self.finish_curve();
        }
        // Space finishes a Bezier curve too (then, as the Pick tool's
        // key, it switches to Pick).
        if pressed(Key::Space, Modifiers::NONE) && self.tool == Tool::Bezier {
            self.finish_curve();
        }
        // Esc with a menu or a list open only closes it.
        // Esc with a crop area waiting only clears it.
        if pressed(Key::Escape, Modifiers::NONE)
            && !egui::Popup::is_any_open(ctx)
            && self.crop_area.take().is_none()
        {
            // In focus mode with nothing selected: one level out.
            if self.selection.is_empty() && self.focus_target().is_some() {
                self.focus_up();
            }
            self.curve = None;
            self.curve_steps.clear();
            self.dimension_points.clear();
            self.eyedropper_reset();
            self.pending_palette_sample = false;
            self.palette_shades = None;
            self.palette_expanded = None;
            self.select(Vec::new());
            self.deselect_guides();
            if self.tool != Tool::Pick {
                self.set_tool(Tool::Pick);
            }
        }
        if pressed(Key::PageDown, Modifiers::NONE) {
            let i = self.page_index();
            self.goto_page(i + 1);
        }
        if pressed(Key::PageUp, Modifiers::NONE) {
            let i = self.page_index();
            if i > 0 {
                self.goto_page(i - 1);
            }
        }
        // The Shape tool with nodes chosen: C turns cusp nodes smooth and
        // the others cusp, S symmetrical nodes smooth and the others
        // symmetrical, Y makes them symmetrical (before the alignment and
        // tool letters).
        let nodes_chosen = self.tool == Tool::Shape && !self.node_selection.is_empty();
        if nodes_chosen {
            use tracedraw_core::nodes::NodeType;
            let now = self.current_node_type();
            let toggled = |key: Key, pair: NodeType| {
                let ty = if now == Some(pair) {
                    NodeType::Smooth
                } else {
                    pair
                };
                (key, ty)
            };
            for (k, ty) in [
                toggled(Key::C, NodeType::Cusp),
                toggled(Key::S, NodeType::Symmetrical),
                (Key::Y, NodeType::Symmetrical),
            ] {
                if pressed(k, Modifiers::NONE) {
                    self.set_selected_node_type(ty);
                    return;
                }
            }
            // Home and End: the curve's first and last node.
            if pressed(Key::Home, Modifiers::NONE) && self.select_end_node(true) {
                return;
            }
            if pressed(Key::End, Modifiers::NONE) && self.select_end_node(false) {
                return;
            }
        }
        // Align shortcuts: plain letters with a selection (not while nodes
        // are being edited).
        // (Not while a curve is drawn: C there makes a cusp.)
        if !self.selection.is_empty() && !nodes_chosen && self.curve.is_none() {
            use crate::ops::Align;
            for (k, a) in [
                (Key::L, Align::Left),
                (Key::R, Align::Right),
                (Key::T, Align::Top),
                (Key::B, Align::Bottom),
                (Key::E, Align::CenterH),
                (Key::C, Align::CenterV),
                (Key::P, Align::CenterPage),
            ] {
                if pressed(k, Modifiers::NONE) {
                    self.align(a);
                    return;
                }
            }
        }
        // Tool shortcuts: user overrides from Options, then the defaults.
        for group in crate::tools::GROUPS {
            for t in group.tools {
                let label = self
                    .settings
                    .shortcuts
                    .iter()
                    .find(|(id, _)| id == t.id())
                    .map(|(_, v)| v.clone())
                    .or_else(|| t.shortcut().map(|s| s.to_string()));
                let Some(label) = label else { continue };
                // Any of Ctrl, Alt and Shift with a key, as menu commands.
                let keys = crate::shortcuts::parse(&label).or_else(|| {
                    Tool::parse_shortcut(&label).map(|(k, shift)| {
                        let mods = if shift {
                            Modifiers::SHIFT
                        } else {
                            Modifiers::NONE
                        };
                        (mods, k)
                    })
                });
                if let Some((mods, k)) = keys {
                    if pressed(k, mods) {
                        self.set_tool(*t);
                    }
                }
            }
        }
        // Arrow nudge: plain, Shift super nudge, Ctrl micro nudge
        // (Document Options > Rulers).
        let mut d = Vec2::ZERO;
        for (key, mods) in &presses {
            let step = if mods.matches_exact(Modifiers::NONE) {
                self.nudge_mm
            } else if mods.matches_exact(Modifiers::SHIFT) {
                self.settings.super_nudge_mm.max(1e-6)
            } else if mods.matches_exact(Modifiers::COMMAND) {
                self.settings.micro_nudge_mm.max(1e-6)
            } else {
                continue;
            };
            match key {
                Key::ArrowLeft => d.x -= step,
                Key::ArrowRight => d.x += step,
                Key::ArrowUp => d.y += step,
                Key::ArrowDown => d.y -= step,
                _ => {}
            }
        }
        if d.hypot() > 0.0 {
            // With the Shape tool and nodes chosen, the nodes move.
            if self.tool == Tool::Shape && !self.node_selection.is_empty() {
                self.nudge_nodes(d);
            } else {
                self.nudge(d);
            }
        }
    }
}
