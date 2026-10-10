//! Guidelines on the active page: hit testing, selection, dragging (move,
//! rotate, drag off the drawing window to delete), locking and presets.
//! The guideline geometry and appearance live in the document
//! (`tracedraw_core::document::Guide`); every change is a command.

use crate::app::{App, Drag};
use tracedraw_core::{
    document::{Guide, GuideLine},
    geometry::{Point, Size, Vec2},
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
                let line = match start.line {
                    GuideLine::Horizontal { y } => GuideLine::Horizontal {
                        y: self.snap_y_without_guides(y + d.y),
                    },
                    GuideLine::Vertical { x } => GuideLine::Vertical {
                        x: self.snap_x_without_guides(x + d.x),
                    },
                    GuideLine::Angled { x, y, angle } => {
                        let q = self.snap_point_without_guides(Point::new(x + d.x, y + d.y));
                        GuideLine::Angled {
                            x: q.x,
                            y: q.y,
                            angle,
                        }
                    }
                };
                self.drag = Drag::MoveGuide {
                    index,
                    press,
                    start,
                    guide: start.with_line(line),
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

    /// Release after dragging a guideline: commit the move or rotation as
    /// one undo step, or delete the guideline when it was dropped outside
    /// the drawing window.
    pub fn finish_guide_move(&mut self, released_at: egui::Pos2) {
        match std::mem::replace(&mut self.drag, Drag::None) {
            Drag::MoveGuide {
                index,
                start,
                guide,
                ..
            } => {
                if !self.canvas_rect.contains(released_at) {
                    self.delete_guide(index);
                } else if guide != start {
                    self.replace_guide(index, guide, "Move Guideline");
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

    fn rotate_handle_at(&self, p: Point) -> Option<(usize, Point)> {
        let (index, pivot, handles) = self.guide_rotate_handles()?;
        let tol = HANDLE_PICK_PX / self.view.zoom.max(1e-6) as f64;
        handles
            .iter()
            .any(|h| (*h - p).hypot() <= tol)
            .then_some((index, pivot))
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
}
