//! Guidelines on the active page: hit testing, selection, dragging (move,
//! rotate, drag off the drawing window to delete), locking and presets.
//! The guideline geometry and appearance live in the document
//! (`tracedraw_core::document::Guide`); every change is a command.

use crate::app::{App, Drag};
use tracedraw_core::{
    document::{Guide, GuideLine},
    geometry::{Affine, Point, Rect, Size, Vec2},
    Color, Command,
};

/// Screen distance (px) within which a click picks a guideline.
const PICK_PX: f64 = 4.0;
/// Screen distance from the pivot to each rotation handle of a guideline
/// in rotate mode (second click on a selected guideline).
pub const ROTATE_HANDLE_PX: f64 = 70.0;
/// Screen radius within which a press grabs a rotation handle.
const HANDLE_PICK_PX: f64 = 7.0;

/// Which preset guidelines Document Options > Guidelines > Presets adds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuidePreset {
    /// Guidelines at a margin from each page edge.
    Margins {
        top: f64,
        bottom: f64,
        left: f64,
        right: f64,
    },
    /// Equal columns between the left and right page edges, with a gutter.
    Columns { count: u32, gutter: f64 },
    /// A grid of rows and columns over the page.
    Grid { rows: u32, columns: u32 },
}

impl GuidePreset {
    /// The guideline lines this preset adds to a page of `size`.
    pub fn lines(self, size: Size) -> Vec<GuideLine> {
        let mut out = Vec::new();
        match self {
            GuidePreset::Margins {
                top,
                bottom,
                left,
                right,
            } => {
                out.push(GuideLine::Horizontal {
                    y: size.height - top,
                });
                out.push(GuideLine::Horizontal { y: bottom });
                out.push(GuideLine::Vertical { x: left });
                out.push(GuideLine::Vertical {
                    x: size.width - right,
                });
            }
            GuidePreset::Columns { count, gutter } => {
                let n = count.clamp(1, 100) as f64;
                let gutter = gutter.max(0.0);
                let col = ((size.width - gutter * (n - 1.0)) / n).max(0.0);
                let mut x = 0.0;
                for i in 0..count.clamp(1, 100) {
                    if i > 0 {
                        out.push(GuideLine::Vertical { x });
                    }
                    x += col;
                    if i + 1 < count.clamp(1, 100) {
                        out.push(GuideLine::Vertical { x });
                        x += gutter;
                    }
                }
                out.dedup();
            }
            GuidePreset::Grid { rows, columns } => {
                let (r, c) = (rows.clamp(1, 200), columns.clamp(1, 200));
                for i in 0..=c {
                    out.push(GuideLine::Vertical {
                        x: size.width * i as f64 / c as f64,
                    });
                }
                for i in 0..=r {
                    out.push(GuideLine::Horizontal {
                        y: size.height * i as f64 / r as f64,
                    });
                }
            }
        }
        out
    }
}

impl App {
    /// The guidelines of the active page.
    pub fn page_guides(&self) -> Vec<Guide> {
        self.doc()
            .page(self.page)
            .map(|p| p.guides.clone())
            .unwrap_or_default()
    }

    /// The topmost guideline under `p` (page coordinates), when guidelines
    /// are shown.
    pub fn guide_at(&self, p: Point) -> Option<usize> {
        if !self.show_guides {
            return None;
        }
        let tol = PICK_PX / self.view.zoom.max(1e-6) as f64;
        let page = self.doc().page(self.page).ok()?;
        page.guides.iter().rposition(|g| g.distance(p).abs() <= tol)
    }

    /// The colour a guideline is drawn with: its own, or the document's
    /// default guideline colour.
    pub fn guide_color(&self, g: &Guide) -> Color {
        g.color.unwrap_or(self.doc().metadata.guides.color)
    }

    pub fn add_guide(&mut self, guide: Guide) {
        let page = self.page;
        self.run(Command::AddGuide { page, guide });
    }

    /// Replace the page's guidelines in one undoable step.
    pub fn set_guides(&mut self, guides: Vec<Guide>, label: &'static str) {
        let page = self.page;
        let n = guides.len();
        if let Err(e) = self
            .engine
            .run_with_label(&Command::SetGuides { page, guides }, label)
        {
            self.status = format!("{label}: {e}");
        }
        self.selected_guides.retain(|i| *i < n);
        if self.guide_rotate.is_some_and(|i| i >= n) {
            self.guide_rotate = None;
        }
    }

    /// Change one guideline (position, angle, colour, style or lock).
    pub fn replace_guide(&mut self, index: usize, guide: Guide, label: &'static str) {
        let page = self.page;
        if let Err(e) = self
            .engine
            .run_with_label(&Command::MoveGuide { page, index, guide }, label)
        {
            self.status = format!("{label}: {e}");
        }
    }

    /// The line the one selected guideline takes (a value typed in the
    /// Guidelines panel's fields or on the property bar). False when no
    /// guideline or several are selected, or it is locked (the status
    /// bar says so).
    pub fn set_selected_guide_line(&mut self, line: GuideLine) -> bool {
        let [i] = self.selected_guides.as_slice() else {
            return false;
        };
        let i = *i;
        let Some(g) = self.page_guides().get(i).copied() else {
            return false;
        };
        if g.locked {
            self.status = crate::i18n::tr("status.guideline_locked");
            return false;
        }
        if g.line == line {
            return true;
        }
        self.replace_guide(i, g.with_line(line), "Move Guideline");
        true
    }

    /// The Guidelines panel for guideline `index`, as a double click on
    /// it or Properties in its menu asks: the guideline selected (alone),
    /// the panel shown with its values in the fields.
    pub fn open_guide_properties(&mut self, index: usize) {
        let Some(g) = self.page_guides().get(index).copied() else {
            return;
        };
        if !self.selection.is_empty() {
            self.select(Vec::new());
        }
        self.selected_guides = vec![index];
        self.guide_rotate = None;
        self.show_dockers = true;
        self.docker_tab = crate::app::DockerTab::Guidelines;
        self.guide_form_for = Some(index);
        let mut form = self.guide_form;
        form.load(self, &g.line);
        self.guide_form = form;
    }

    /// The Guidelines panel's fields follow the one selected guideline:
    /// when another one becomes the selection, its values load.
    pub fn sync_guide_form(&mut self) {
        let single = match self.selected_guides.as_slice() {
            [i] => Some(*i),
            _ => None,
        };
        if single == self.guide_form_for {
            return;
        }
        self.guide_form_for = single;
        if let Some(g) = single.and_then(|i| self.page_guides().get(i).copied()) {
            let mut form = self.guide_form;
            form.load(self, &g.line);
            self.guide_form = form;
        }
    }

    pub fn delete_guide(&mut self, index: usize) {
        let page = self.page;
        self.run(Command::DeleteGuide { page, index });
        self.selected_guides.clear();
        self.guide_rotate = None;
    }

    /// Delete the selected guidelines that are not locked. Returns false
    /// when nothing was deleted.
    pub fn delete_selected_guides(&mut self) -> bool {
        let guides = self.page_guides();
        let keep: Vec<Guide> = guides
            .iter()
            .enumerate()
            .filter(|(i, g)| g.locked || !self.selected_guides.contains(i))
            .map(|(_, g)| *g)
            .collect();
        if keep.len() == guides.len() {
            return false;
        }
        self.selected_guides.clear();
        self.guide_rotate = None;
        self.set_guides(keep, "Delete Guideline");
        true
    }

    /// Edit > Copy (and Cut) with guidelines selected and no objects: the
    /// guidelines go on the clipboard, to be pasted where they are (#376).
    /// False when no guideline is selected.
    pub fn copy_selected_guides(&mut self) -> bool {
        let guides = self.page_guides();
        let copied: Vec<Guide> = self
            .selected_guides
            .iter()
            .filter_map(|i| guides.get(*i))
            .map(|g| Guide {
                locked: false,
                ..*g
            })
            .collect();
        if copied.is_empty() {
            return false;
        }
        let n = copied.len();
        self.clipboard = Some(crate::app::Clipboard {
            shapes: Vec::new(),
            styles: Vec::new(),
            symbols: Vec::new(),
            system: Default::default(),
            guides: copied,
        });
        self.mark_system_clipboard(None);
        self.status = crate::i18n::trf("status.guides_copied", &[("n", &n.to_string())]);
        true
    }

    /// Edit > Paste of copied guidelines: copies at the same places on the
    /// current page, selected, in one undo step (#376).
    pub fn paste_guides(&mut self, copied: &[Guide]) {
        let mut guides = self.page_guides();
        let n = guides.len();
        guides.extend_from_slice(copied);
        self.select(Vec::new());
        self.set_guides(guides, "Paste Guidelines");
        self.selected_guides = (n..n + copied.len()).collect();
        self.guide_rotate = None;
    }

    /// The box the Transformations docker works on for the selected
    /// guidelines: round the points that place them (a vertical one at the
    /// page's middle height, a horizontal one at its middle width, an
    /// angled one at its point). `None` with none selected.
    pub fn selected_guides_bounds(&self) -> Option<Rect> {
        let guides = self.page_guides();
        let mid = self.page_rect().center();
        self.selected_guides
            .iter()
            .filter_map(|i| guides.get(*i))
            .map(|g| {
                let p = line_point(g.line, mid);
                Rect::from_points(p, p)
            })
            .reduce(|a, b| a.union(b))
    }

    /// The Transformations docker on guidelines (no objects selected): the
    /// selected ones moved, turned, scaled or skewed by `t` (page space),
    /// or with `copies` that many copies, each the one before transformed
    /// again, the last ones selected. Locked guidelines stay. One undo
    /// step named `label`; false when nothing changed (#376), as for a
    /// horizontal guideline moved sideways (no undo step then).
    pub fn transform_selected_guides(
        &mut self,
        t: Affine,
        copies: u32,
        label: &'static str,
    ) -> bool {
        let mut guides = self.page_guides();
        let mid = self.page_rect().center();
        let picked: Vec<usize> = self
            .selected_guides
            .iter()
            .copied()
            .filter(|i| guides.get(*i).is_some_and(|g| copies > 0 || !g.locked))
            .collect();
        if picked.is_empty() {
            return false;
        }
        if copies == 0 {
            let before = guides.clone();
            for &i in &picked {
                guides[i].line = transformed_line(guides[i].line, t, mid);
            }
            if guides == before {
                return false;
            }
            self.set_guides(guides, label);
            return true;
        }
        let mut last: Vec<Guide> = picked.iter().map(|i| guides[*i]).collect();
        for _ in 0..copies {
            last = last
                .iter()
                .map(|g| Guide {
                    locked: false,
                    line: transformed_line(g.line, t, mid),
                    ..*g
                })
                .collect();
            guides.extend_from_slice(&last);
        }
        let k = last.len();
        let total = guides.len();
        self.set_guides(guides, label);
        self.selected_guides = (total - k..total).collect();
        true
    }

    /// Edit > Select All > Guidelines: every guideline of the page.
    pub fn select_all_guides(&mut self) {
        self.select(Vec::new());
        self.selected_guides = (0..self.page_guides().len()).collect();
        self.guide_rotate = None;
    }

    /// Lock or unlock the given guidelines.
    pub fn set_guides_locked(&mut self, indices: &[usize], locked: bool) {
        let mut guides = self.page_guides();
        let mut changed = false;
        for &i in indices {
            if let Some(g) = guides.get_mut(i) {
                changed |= g.locked != locked;
                g.locked = locked;
            }
        }
        if changed {
            self.set_guides(
                guides,
                if locked {
                    "Lock Guideline"
                } else {
                    "Unlock Guideline"
                },
            );
        }
    }

    /// Pick tool press: select the guideline under `p` (Shift adds to the
    /// selection) and start moving it unless it is locked. A press on a
    /// rotation handle of the guideline in rotate mode starts rotating.
    /// Returns true when the press was taken.
    pub fn press_guide(&mut self, p: Point, extend: bool) -> bool {
        if let Some((index, pivot)) = self.rotate_handle_at(p) {
            if let Some(g) = self.page_guides().get(index).copied() {
                self.drag = Drag::RotateGuide {
                    index,
                    pivot,
                    guide: g,
                };
                return true;
            }
        }
        let Some(index) = self.guide_at(p) else {
            return false;
        };
        let Some(g) = self.page_guides().get(index).copied() else {
            return false;
        };
        if extend {
            if !self.selected_guides.contains(&index) {
                self.selected_guides.push(index);
            }
        } else if !self.selected_guides.contains(&index) {
            self.selected_guides = vec![index];
        }
        if !self.selection.is_empty() {
            self.select(Vec::new());
        }
        self.drag = if g.locked {
            Drag::None
        } else {
            self.guide_rotate = None;
            Drag::MoveGuide {
                index,
                press: p,
                start: g,
                guide: g,
                delta: Vec2::ZERO,
            }
        };
        true
    }

    /// Pick tool click (press and release without moving) on a guideline:
    /// selects it; a click on the only selected guideline toggles its
    /// rotation handles.
    pub fn click_guide(&mut self, p: Point, extend: bool) -> bool {
        let Some(index) = self.guide_at(p) else {
            return false;
        };
        if !self.selection.is_empty() {
            self.select(Vec::new());
        }
        if extend {
            if let Some(i) = self.selected_guides.iter().position(|g| *g == index) {
                self.selected_guides.remove(i);
            } else {
                self.selected_guides.push(index);
            }
            self.guide_rotate = None;
            return true;
        }
        if self.selected_guides == [index] {
            self.guide_rotate = match self.guide_rotate {
                Some(i) if i == index => None,
                _ => Some(index),
            };
            if let Some(g) = self.page_guides().get(index) {
                // The pivot is where the guideline was clicked.
                let (o, d) = g.line.point_and_direction();
                let t = (p - o).dot(d);
                self.guide_pivot = o + d * t;
            }
        } else {
            self.selected_guides = vec![index];
            self.guide_rotate = None;
        }
        true
    }

    /// Duplicate the selected guidelines (Edit > Duplicate, Ctrl+D, with
    /// guidelines and no objects selected), moved by the duplicate offset;
    /// the copies become the selection. False when no guideline is
    /// selected.
    pub fn duplicate_selected_guides(&mut self) -> bool {
        if self.selected_guides.is_empty() {
            return false;
        }
        let mut guides = self.page_guides();
        let n = guides.len();
        let d = self.duplicate_offset;
        let copies: Vec<Guide> = self
            .selected_guides
            .iter()
            .filter_map(|i| guides.get(*i))
            .map(|g| Guide {
                locked: false,
                ..g.with_line(moved_line(g.line, d))
            })
            .collect();
        if copies.is_empty() {
            return false;
        }
        let k = copies.len();
        guides.extend(copies);
        self.set_guides(guides, "Duplicate Guidelines");
        self.selected_guides = (n..n + k).collect();
        true
    }

    /// Delete every guideline of one kind on the page (the Guidelines
    /// docker's Clear): horizontal, vertical or angled. One undo step.
    pub fn clear_guides_of_kind(&mut self, kind: crate::ui::layout_options::GuideKind) {
        use crate::ui::layout_options::GuideKind;
        let kept: Vec<Guide> = self
            .page_guides()
            .into_iter()
            .filter(|g| GuideKind::of(&g.line) != kind)
            .collect();
        if kept.len() != self.page_guides().len() {
            self.set_guides(kept, "Clear Guidelines");
            self.selected_guides.clear();
            self.guide_rotate = None;
        }
    }

    /// Clear the guideline selection and rotate mode.
    pub fn deselect_guides(&mut self) {
        self.selected_guides.clear();
        self.guide_rotate = None;
    }

    /// Follow the pointer while a guideline is dragged: a moved guideline
    /// keeps its offset from the pointer and snaps to the grid, the page
    /// and objects; a rotated one turns about its pivot in whole degrees.
    pub fn drag_guide_to(&mut self, p: Point) {
        match self.drag.clone() {
            Drag::MoveGuide {
                index,
                press,
                start,
                ..
            } => {
                let d = p - press;
                let (line, delta) = match start.line {
                    GuideLine::Horizontal { y } => {
                        let ny = self.snap_y_without_guides(y + d.y);
                        (GuideLine::Horizontal { y: ny }, Vec2::new(d.x, ny - y))
                    }
                    GuideLine::Vertical { x } => {
                        let nx = self.snap_x_without_guides(x + d.x);
                        (GuideLine::Vertical { x: nx }, Vec2::new(nx - x, d.y))
                    }
                    GuideLine::Angled { x, y, angle } => {
                        let q = self.snap_point_without_guides(Point::new(x + d.x, y + d.y));
                        (
                            GuideLine::Angled {
                                x: q.x,
                                y: q.y,
                                angle,
                            },
                            Vec2::new(q.x - x, q.y - y),
                        )
                    }
                };
                self.drag = Drag::MoveGuide {
                    index,
                    press,
                    start,
                    guide: start.with_line(line),
                    delta,
                };
            }
            Drag::RotateGuide {
                index,
                pivot,
                guide,
            } => {
                let v: Vec2 = p - pivot;
                if v.hypot() > 1e-9 {
                    let a = v.y.atan2(v.x).to_degrees().round();
                    self.drag = Drag::RotateGuide {
                        index,
                        pivot,
                        guide: guide.with_line(guide.line.rotated_to(a, pivot)),
                    };
                }
            }
            _ => {}
        }
    }

    /// The guidelines a drag carries and where it has them now: the one
    /// pressed, then the other selected ones that are not locked, moved as
    /// far (#376).
    pub fn guide_drag_moves(&self) -> Vec<(usize, Guide)> {
        let Drag::MoveGuide {
            index,
            guide,
            delta,
            ..
        } = &self.drag
        else {
            return Vec::new();
        };
        let guides = self.page_guides();
        let mut out = vec![(*index, *guide)];
        for &i in &self.selected_guides {
            if i == *index {
                continue;
            }
            if let Some(g) = guides.get(i).filter(|g| !g.locked) {
                out.push((i, g.with_line(moved_line(g.line, *delta))));
            }
        }
        out
    }

    /// Release after dragging guidelines: commit the move or rotation as
    /// one undo step; after a right click during the drag, copies take the
    /// move and the guidelines stay; dropped outside the drawing window,
    /// the guidelines go.
    pub fn finish_guide_move(&mut self, released_at: egui::Pos2) {
        let moves = self.guide_drag_moves();
        let copy = std::mem::replace(&mut self.drag_copy, false);
        match std::mem::replace(&mut self.drag, Drag::None) {
            Drag::MoveGuide { start, guide, .. } => {
                let mut guides = self.page_guides();
                if !self.canvas_rect.contains(released_at) {
                    if copy {
                        return;
                    }
                    if let [(index, _)] = moves.as_slice() {
                        self.delete_guide(*index);
                        return;
                    }
                    let gone: Vec<usize> = moves.iter().map(|(i, _)| *i).collect();
                    let kept: Vec<Guide> = guides
                        .into_iter()
                        .enumerate()
                        .filter(|(i, _)| !gone.contains(i))
                        .map(|(_, g)| g)
                        .collect();
                    self.set_guides(kept, "Delete Guideline");
                    self.selected_guides.clear();
                } else if guide != start {
                    if copy {
                        let n = guides.len();
                        guides.extend(moves.iter().map(|(_, g)| *g));
                        self.set_guides(guides, "Copy Guidelines");
                        self.selected_guides = (n..n + moves.len()).collect();
                    } else {
                        for (i, g) in &moves {
                            if let Some(slot) = guides.get_mut(*i) {
                                *slot = *g;
                            }
                        }
                        let label = if moves.len() > 1 {
                            "Move Guidelines"
                        } else {
                            "Move Guideline"
                        };
                        self.set_guides(guides, label);
                    }
                }
            }
            Drag::RotateGuide { index, guide, .. } => {
                self.replace_guide(index, guide, "Rotate Guideline");
            }
            other => self.drag = other,
        }
    }

    /// A new guideline dragged out of a ruler is added where it is
    /// released inside the drawing window.
    pub fn finish_guide_drag(&mut self) {
        if let Drag::NewGuide { horizontal, pos } = self.drag.clone() {
            if self.canvas_rect.contains(self.view.to_screen(pos)) {
                let guide = if horizontal {
                    Guide::horizontal(pos.y)
                } else {
                    Guide::vertical(pos.x)
                };
                self.add_guide(guide);
                self.selected_guides = vec![self.page_guides().len().saturating_sub(1)];
            }
            self.drag = Drag::None;
        }
    }

    /// Screen positions of the rotation handles of the guideline in rotate
    /// mode: the pivot and one handle each side along the line.
    pub fn guide_rotate_handles(&self) -> Option<(usize, Point, [Point; 2])> {
        let index = self.guide_rotate?;
        let g = self.page_guides().get(index).copied()?;
        let line = match &self.drag {
            Drag::RotateGuide {
                index: i, guide, ..
            } if *i == index => guide.line,
            _ => g.line,
        };
        let (_, d) = line.point_and_direction();
        let reach = ROTATE_HANDLE_PX / self.view.zoom.max(1e-6) as f64;
        let pivot = self.guide_pivot;
        Some((index, pivot, [pivot + d * reach, pivot - d * reach]))
    }

    /// `p` (page) is on a rotation handle of the guideline in rotate mode.
    pub fn guide_rotate_handle_at(&self, p: Point) -> bool {
        self.rotate_handle_at(p).is_some()
    }

    fn rotate_handle_at(&self, p: Point) -> Option<(usize, Point)> {
        let (index, pivot, handles) = self.guide_rotate_handles()?;
        let tol = HANDLE_PICK_PX / self.view.zoom.max(1e-6) as f64;
        handles
            .iter()
            .any(|h| (*h - p).hypot() <= tol)
            .then_some((index, pivot))
    }
}

/// A guideline line moved by `d`: a horizontal one only up and down, a
/// vertical one only sideways, an angled one both ways.
/// A point that places `line`: on a horizontal one at `mid`'s x, on a
/// vertical one at `mid`'s y, an angled one's own point.
fn line_point(line: GuideLine, mid: Point) -> Point {
    match line {
        GuideLine::Horizontal { y } => Point::new(mid.x, y),
        GuideLine::Vertical { x } => Point::new(x, mid.y),
        GuideLine::Angled { x, y, .. } => Point::new(x, y),
    }
}

/// `line` under `t` (page space): the line through the image of a point on
/// it, along the image of its direction; horizontal or vertical again when
/// it ends up so. `mid` places the point taken on a horizontal or vertical
/// line (see [`line_point`]).
pub fn transformed_line(line: GuideLine, t: Affine, mid: Point) -> GuideLine {
    let p = line_point(line, mid);
    let dir = match line {
        GuideLine::Horizontal { .. } => Vec2::new(1.0, 0.0),
        GuideLine::Vertical { .. } => Vec2::new(0.0, 1.0),
        GuideLine::Angled { angle, .. } => {
            let a = angle.to_radians();
            Vec2::new(a.cos(), a.sin())
        }
    };
    let q = t * p;
    let d = t * (p + dir) - q;
    if !(q.x.is_finite() && q.y.is_finite()) || d.hypot() < 1e-12 {
        return line;
    }
    let angle = d.y.atan2(d.x).to_degrees().rem_euclid(180.0);
    let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
    if near(angle, 0.0) || near(angle, 180.0) {
        GuideLine::Horizontal { y: q.y }
    } else if near(angle, 90.0) {
        GuideLine::Vertical { x: q.x }
    } else {
        GuideLine::Angled {
            x: q.x,
            y: q.y,
            angle,
        }
    }
}

pub fn moved_line(line: GuideLine, d: Vec2) -> GuideLine {
    match line {
        GuideLine::Horizontal { y } => GuideLine::Horizontal { y: y + d.y },
        GuideLine::Vertical { x } => GuideLine::Vertical { x: x + d.x },
        GuideLine::Angled { x, y, angle } => GuideLine::Angled {
            x: x + d.x,
            y: y + d.y,
            angle,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::headless();
        app.view.zoom = 1.0;
        app.canvas_rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        app
    }

    /// A value typed for the one selected guideline moves it at once (the
    /// panel's fields, the property bar); a locked one stays and the
    /// status bar says so; a double click opens the panel on it with its
    /// values, and the fields follow the selection (#306).
    #[test]
    fn typed_values_move_the_selected_guideline_and_the_panel_follows_it() {
        let mut app = app();
        app.add_guide(Guide::horizontal(50.0));
        app.add_guide(Guide::vertical(20.0));
        app.selected_guides = vec![1];
        assert!(app.set_selected_guide_line(GuideLine::Vertical { x: 35.0 }));
        assert_eq!(app.page_guides()[1].line, GuideLine::Vertical { x: 35.0 });
        assert_eq!(app.page_guides()[0].line, GuideLine::Horizontal { y: 50.0 });
        // Several selected: nothing moves.
        app.selected_guides = vec![0, 1];
        assert!(!app.set_selected_guide_line(GuideLine::Vertical { x: 1.0 }));
        // Locked: nothing moves, the status bar says so.
        app.set_guides_locked(&[1], true);
        app.selected_guides = vec![1];
        app.status.clear();
        assert!(!app.set_selected_guide_line(GuideLine::Vertical { x: 1.0 }));
        assert_eq!(app.page_guides()[1].line, GuideLine::Vertical { x: 35.0 });
        assert!(!app.status.is_empty());
        // The panel on the horizontal guideline, its value in the field.
        app.show_dockers = false;
        app.open_guide_properties(0);
        assert_eq!(app.selected_guides, vec![0]);
        assert!(app.show_dockers);
        assert_eq!(app.docker_tab, crate::app::DockerTab::Guidelines);
        assert!((app.guide_form.y - app.to_ruler(Point::new(0.0, 50.0)).y).abs() < 1e-9);
        // Selecting the other one by a click: the fields follow.
        assert!(app.click_guide(Point::new(35.0, 10.0), false));
        app.sync_guide_form();
        assert!((app.guide_form.x - app.to_ruler(Point::new(35.0, 0.0)).x).abs() < 1e-9);
        // The property bar draws for the selected guideline.
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 80.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            ui.horizontal(|ui| crate::ui::propbar::guide_bar(&mut app, ui));
        });
        out.textures_delta.clear();
    }

    #[test]
    fn dragging_a_guide_is_one_undo_step_and_leaves_other_guides_alone() {
        let mut app = app();
        app.add_guide(Guide::horizontal(50.0));
        app.add_guide(Guide::vertical(20.0));
        // Move the vertical guide, then the horizontal one.
        assert!(app.press_guide(Point::new(20.0, 10.0), false));
        app.drag_guide_to(Point::new(35.0, 10.0));
        app.finish_guide_move(egui::pos2(100.0, 100.0));
        assert!(app.press_guide(Point::new(5.0, 50.0), false));
        app.drag_guide_to(Point::new(5.0, 70.0));
        app.drag_guide_to(Point::new(5.0, 80.0));
        app.finish_guide_move(egui::pos2(100.0, 100.0));
        let g = app.page_guides();
        assert_eq!(g[0].line, GuideLine::Horizontal { y: 80.0 });
        assert_eq!(g[1].line, GuideLine::Vertical { x: 35.0 });
        // One undo puts back only the last move.
        app.engine.undo().unwrap();
        let g = app.page_guides();
        assert_eq!(g[0].line, GuideLine::Horizontal { y: 50.0 });
        assert_eq!(g[1].line, GuideLine::Vertical { x: 35.0 });
    }

    #[test]
    fn dropping_a_guide_outside_the_window_deletes_it() {
        let mut app = app();
        app.add_guide(Guide::horizontal(50.0));
        assert!(app.press_guide(Point::new(0.0, 50.0), false));
        app.drag_guide_to(Point::new(0.0, 60.0));
        app.finish_guide_move(egui::pos2(-20.0, 100.0));
        assert!(app.page_guides().is_empty());
    }

    #[test]
    fn locked_guides_select_but_do_not_move_or_delete() {
        let mut app = app();
        app.add_guide(Guide::vertical(20.0));
        app.add_guide(Guide::vertical(40.0));
        app.set_guides_locked(&[0], true);
        assert!(app.press_guide(Point::new(20.0, 0.0), false));
        assert!(matches!(app.drag, Drag::None));
        assert_eq!(app.selected_guides, vec![0]);
        app.select_all_guides();
        assert_eq!(app.selected_guides, vec![0, 1]);
        assert!(app.delete_selected_guides());
        let g = app.page_guides();
        assert_eq!(g.len(), 1);
        assert!(g[0].locked);
        // Nothing left that can be deleted.
        app.select_all_guides();
        assert!(!app.delete_selected_guides());
    }

    #[test]
    fn second_click_rotates_a_guide_about_the_clicked_point() {
        let mut app = app();
        app.add_guide(Guide::horizontal(50.0));
        let p = Point::new(30.0, 50.0);
        assert!(app.click_guide(p, false));
        assert_eq!(app.guide_rotate, None);
        assert!(app.click_guide(p, false));
        assert_eq!(app.guide_rotate, Some(0));
        let (_, pivot, handles) = app.guide_rotate_handles().unwrap();
        assert_eq!(pivot, p);
        // Grab the right handle and turn it to 45 degrees.
        assert!(app.press_guide(handles[0], false));
        app.drag_guide_to(pivot + Vec2::new(10.0, 10.0));
        app.finish_guide_move(egui::pos2(100.0, 100.0));
        assert_eq!(
            app.page_guides()[0].line,
            GuideLine::Angled {
                x: 30.0,
                y: 50.0,
                angle: 45.0
            }
        );
    }

    #[test]
    fn preset_lines_cover_margins_columns_and_grids() {
        let cols = GuidePreset::Columns {
            count: 3,
            gutter: 10.0,
        }
        .lines(Size::new(120.0, 100.0));
        // Three 33.33 mm columns with two 10 mm gutters: four inner lines.
        assert_eq!(cols.len(), 4);
        assert!(matches!(cols[0], GuideLine::Vertical { x } if (x - 100.0 / 3.0).abs() < 1e-9));
        let margins = GuidePreset::Margins {
            top: 5.0,
            bottom: 6.0,
            left: 7.0,
            right: 8.0,
        }
        .lines(Size::new(100.0, 50.0));
        assert!(margins.contains(&GuideLine::Horizontal { y: 45.0 }));
        assert!(margins.contains(&GuideLine::Vertical { x: 92.0 }));
        let grid = GuidePreset::Grid {
            rows: 2,
            columns: 4,
        }
        .lines(Size::new(100.0, 50.0));
        assert_eq!(grid.len(), 5 + 3);
        assert!(grid.contains(&GuideLine::Horizontal { y: 25.0 }));
    }

    /// Several selected guidelines move together; a right click during
    /// the drag leaves copies; Duplicate copies them; Clear empties a
    /// type (#376).
    /// Copy and Paste of selected guidelines (no objects selected) puts
    /// copies where they were, selected, in one undo step; Cut removes
    /// them; the Transformations docker moves, turns and copies them; the
    /// duplicate offset is the document's (#376).
    #[test]
    fn guidelines_copy_paste_in_place_and_take_the_transformations_docker() {
        let mut app = app();
        app.add_guide(Guide::horizontal(50.0));
        app.add_guide(Guide::vertical(200.0));
        app.add_guide(Guide::from(GuideLine::Angled {
            x: 10.0,
            y: 10.0,
            angle: 30.0,
        }));
        app.select(Vec::new());
        app.selected_guides = vec![0, 2];
        app.copy_with_system();
        assert_eq!(
            app.status,
            crate::i18n::trf("status.guides_copied", &[("n", "2")])
        );
        let depth = app.engine.history_labels().0.len();
        app.paste();
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let g = app.page_guides();
        assert_eq!(g.len(), 5);
        assert_eq!(g[3].line, g[0].line, "in the same place");
        assert_eq!(g[4].line, g[2].line);
        assert_eq!(app.selected_guides, vec![3, 4]);
        // Cut takes the pasted ones away; Paste brings them back.
        app.cut();
        assert_eq!(app.page_guides().len(), 3);
        app.paste();
        assert_eq!(app.page_guides().len(), 5);
        app.undo();
        app.undo();
        app.undo();
        assert_eq!(app.page_guides().len(), 3);
        // The docker: moved 10 right and 5 up.
        app.selected_guides = vec![0, 1];
        app.transform = Default::default();
        app.transform.position = [10.0, 5.0];
        crate::ui::transform_docker::apply(&mut app);
        let g = app.page_guides();
        assert_eq!(g[0].line, GuideLine::Horizontal { y: 55.0 });
        assert_eq!(g[1].line, GuideLine::Vertical { x: 210.0 });
        // Turned 90 degrees about their box's centre: the horizontal one
        // becomes vertical and the vertical one horizontal.
        app.transform.tab = crate::app::TransformTab::Rotate;
        app.transform.angle = 90.0;
        crate::ui::transform_docker::apply(&mut app);
        let g = app.page_guides();
        assert!(
            matches!(g[0].line, GuideLine::Vertical { .. }),
            "{:?}",
            g[0].line
        );
        assert!(
            matches!(g[1].line, GuideLine::Horizontal { .. }),
            "{:?}",
            g[1].line
        );
        // Two copies, each 20 further right, the last ones selected.
        app.undo();
        app.transform.tab = crate::app::TransformTab::Position;
        app.transform.position = [20.0, 0.0];
        app.transform.copies = 2;
        app.selected_guides = vec![1];
        crate::ui::transform_docker::apply(&mut app);
        let g = app.page_guides();
        assert_eq!(g.len(), 5);
        assert_eq!(g[4].line, GuideLine::Vertical { x: 250.0 });
        assert_eq!(app.selected_guides, vec![4]);
        // An angled one keeps its angle when moved.
        let angled = transformed_line(
            GuideLine::Angled {
                x: 0.0,
                y: 0.0,
                angle: 30.0,
            },
            Affine::translate((5.0, 0.0)),
            Point::ZERO,
        );
        match angled {
            GuideLine::Angled { x, angle, .. } => {
                assert!((x - 5.0).abs() < 1e-9 && (angle - 30.0).abs() < 1e-9)
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn selected_guidelines_move_copy_duplicate_and_clear_together() {
        let mut app = app();
        app.add_guide(Guide::horizontal(50.0));
        app.add_guide(Guide {
            ..Guide::from(GuideLine::Angled {
                x: 10.0,
                y: 10.0,
                angle: 30.0,
            })
        });
        app.add_guide(Guide::vertical(200.0));
        app.selected_guides = vec![0, 1];
        let inside = egui::pos2(100.0, 100.0);
        // Pressed on the horizontal one, both go.
        assert!(app.press_guide(Point::new(30.0, 50.0), false));
        assert_eq!(app.selected_guides, vec![0, 1]);
        app.drag_guide_to(Point::new(37.0, 61.0));
        assert_eq!(app.guide_drag_moves().len(), 2);
        app.finish_guide_move(inside);
        let gs = app.page_guides();
        let GuideLine::Horizontal { y } = gs[0].line else {
            panic!("horizontal");
        };
        let dy = y - 50.0;
        assert!(dy.abs() > 1.0, "{gs:?}");
        let GuideLine::Angled { x: ax, y: ay, .. } = gs[1].line else {
            panic!("angled");
        };
        assert!(
            (ay - 10.0 - dy).abs() < 1e-9 && (ax - 17.0).abs() < 1e-9,
            "{gs:?}"
        );
        assert_eq!(gs[2].line, GuideLine::Vertical { x: 200.0 });
        // One undo puts both back.
        app.undo();
        assert_eq!(app.page_guides()[0].line, GuideLine::Horizontal { y: 50.0 });
        // A right click during the drag: copies, the originals stay.
        app.selected_guides = vec![0, 1];
        assert!(app.press_guide(Point::new(30.0, 50.0), false));
        app.drag_guide_to(Point::new(30.0, 70.0));
        app.drag_copy = true;
        app.finish_guide_move(inside);
        let gs = app.page_guides();
        assert_eq!(gs.len(), 5);
        assert_eq!(gs[0].line, GuideLine::Horizontal { y: 50.0 });
        assert_eq!(app.selected_guides, vec![3, 4]);
        assert!(!app.drag_copy);
        // Duplicate with guidelines (and no objects) selected.
        app.select(Vec::new());
        app.selected_guides = vec![2];
        app.duplicate_offset = Vec2::new(5.0, 5.0);
        app.duplicate();
        let gs = app.page_guides();
        assert_eq!(gs.len(), 6);
        assert_eq!(gs[5].line, GuideLine::Vertical { x: 205.0 });
        assert_eq!(app.selected_guides, vec![5]);
        // Clear the vertical ones: one step.
        app.clear_guides_of_kind(crate::ui::layout_options::GuideKind::Vertical);
        assert!(app
            .page_guides()
            .iter()
            .all(|g| !matches!(g.line, GuideLine::Vertical { .. })));
        app.undo();
        assert_eq!(app.page_guides().len(), 6);
    }
}
