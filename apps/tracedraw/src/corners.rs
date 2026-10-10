//! The Corners docker's work: fillet, scallop or chamfer the corners of
//! the selected objects (every corner, or the nodes chosen with the Shape
//! tool on a curve). Objects that are not curves become curves.

use crate::app::App;
use crate::tools::Tool;
use tracedraw_core::{
    corner_cut::{cut_corners, CornerCut},
    document::ShapeKind,
    geometry::BezPath,
    Command, Shape, ShapeId,
};

/// The operation chosen in the docker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CornerOp {
    #[default]
    Fillet,
    Scallop,
    Chamfer,
}

/// The Corners docker's settings (millimetres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornersSettings {
    pub op: CornerOp,
    pub radius: f64,
    /// Chamfer distances along the segment into the corner and out of it.
    pub a: f64,
    pub b: f64,
    /// B follows A.
    pub lock: bool,
}

impl Default for CornersSettings {
    fn default() -> Self {
        CornersSettings {
            op: CornerOp::Fillet,
            radius: 2.0,
            a: 2.0,
            b: 2.0,
            lock: true,
        }
    }
}

impl CornersSettings {
    pub fn cut(&self) -> CornerCut {
        match self.op {
            CornerOp::Fillet => CornerCut::Fillet(self.radius),
            CornerOp::Scallop => CornerCut::Scallop(self.radius),
            CornerOp::Chamfer => {
                CornerCut::Chamfer(self.a, if self.lock { self.a } else { self.b })
            }
        }
    }
}

/// The Join Curves docker's joint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JoinKind {
    #[default]
    Extend,
    Chamfer,
    Fillet,
    Bezier,
}

/// The Join Curves docker's settings (millimetres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JoinSettings {
    pub mode: JoinKind,
    /// Ends further apart than this are not joined.
    pub gap: f64,
    /// The Fillet joint's radius.
    pub radius: f64,
}

impl Default for JoinSettings {
    fn default() -> Self {
        JoinSettings {
            mode: JoinKind::Extend,
            gap: 5.0,
            radius: 2.0,
        }
    }
}

/// One selected object's outline after the cut, in its own space.
pub struct CutResult {
    pub shape: ShapeId,
    pub path: BezPath,
    pub closed: bool,
    pub corners: usize,
}

/// The outline of `s` as a curve, if it has one to cut.
fn outline(s: &Shape) -> Option<(BezPath, bool)> {
    match &s.kind {
        ShapeKind::Path { path, closed } => Some((path.clone(), *closed)),
        ShapeKind::Rect { .. }
        | ShapeKind::Ellipse { .. }
        | ShapeKind::Polygon { .. }
        | ShapeKind::Text { .. } => Some((s.local_path(), true)),
        _ => None,
    }
}

impl App {
    /// What the cut would make of each selected object that has corners
    /// to cut.
    pub fn corner_results(&self, cut: CornerCut) -> Vec<CutResult> {
        let mut out = Vec::new();
        for s in self.selected_shapes() {
            let Some((path, closed)) = outline(&s) else {
                continue;
            };
            // With the Shape tool, a curve's chosen nodes only.
            let chosen: Vec<usize> = self
                .node_selection
                .iter()
                .filter(|(id, _)| *id == s.id)
                .map(|(_, i)| *i)
                .collect();
            let only = (self.tool == Tool::Shape
                && matches!(s.kind, ShapeKind::Path { .. })
                && !chosen.is_empty())
            .then_some(chosen.as_slice());
            let (path, corners) = cut_corners(&path, cut, only);
            if corners > 0 {
                out.push(CutResult {
                    shape: s.id,
                    path,
                    closed,
                    corners,
                });
            }
        }
        out
    }

    /// The docker's preview: the cut outlines on the page.
    pub fn corner_preview(&self) -> Vec<BezPath> {
        let cut = self.corners.cut();
        self.corner_results(cut)
            .into_iter()
            .filter_map(|r| {
                let s = self.doc().find_shape(r.shape)?;
                Some(s.transform * r.path)
            })
            .collect()
    }

    /// Apply the docker's cut; returns the number of corners cut.
    pub fn apply_corners(&mut self) -> usize {
        let cut = self.corners.cut();
        let results = self.corner_results(cut);
        let total = results.iter().map(|r| r.corners).sum();
        let cmds: Vec<Command> = results
            .into_iter()
            .map(|r| Command::SetShapeKind {
                shape: r.shape,
                kind: ShapeKind::Path {
                    path: r.path,
                    closed: r.closed,
                },
            })
            .collect();
        if cmds.is_empty() {
            return 0;
        }
        let label = match self.corners.op {
            CornerOp::Fillet => "Fillet",
            CornerOp::Scallop => "Scallop",
            CornerOp::Chamfer => "Chamfer",
        };
        if let Err(e) = self.engine.run_batch(label, &cmds) {
            self.status = e.to_string();
            return 0;
        }
        self.node_selection.clear();
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Rect, Shape as _};

    #[test]
    fn the_docker_cuts_rectangles_as_curves_in_one_step() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("a rectangle");
        app.select(vec![id]);
        app.corners = CornersSettings {
            op: CornerOp::Chamfer,
            a: 2.0,
            b: 9.0,
            lock: true,
            ..Default::default()
        };
        assert_eq!(app.corner_preview().len(), 1);
        let depth = app.engine.history_labels().0.len();
        assert_eq!(app.apply_corners(), 4);
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let s = app.selected_shapes()[0].clone();
        let ShapeKind::Path { path, closed } = &s.kind else {
            panic!("{:?}", s.kind);
        };
        assert!(*closed);
        assert!((path.area().abs() - 92.0).abs() < 1e-9);
        // Nothing left to cut at this size: nothing happens.
        app.corners.a = 50.0;
        assert_eq!(app.apply_corners(), 0);
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
    }

    #[test]
    fn the_shape_tool_limits_the_cut_to_chosen_nodes() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Path {
                path: Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.01),
                closed: true,
            })
            .expect("a curve");
        app.select(vec![id]);
        app.tool = Tool::Shape;
        app.node_selection = vec![(id, 2)];
        app.corners.op = CornerOp::Scallop;
        assert_eq!(app.apply_corners(), 1);
        // Other tools cut every corner.
        app.tool = Tool::Pick;
        app.select(vec![id]);
        assert!(app.apply_corners() >= 3);
    }

    #[test]
    fn join_curves_keeps_the_last_selected_curve() {
        let mut app = App::headless();
        let line = |app: &mut App, a: (f64, f64), b: (f64, f64)| {
            let mut p = BezPath::new();
            p.move_to(a);
            p.line_to(b);
            app.new_shape(ShapeKind::Path {
                path: p,
                closed: false,
            })
            .expect("a line")
        };
        let a = line(&mut app, (0.0, 0.0), (8.0, 0.0));
        let b = line(&mut app, (10.0, 2.0), (10.0, 10.0));
        app.select(vec![a, b]);
        app.join_settings.gap = 1.0;
        assert_eq!(app.join_curves(), 0);
        assert_eq!(app.selection.len(), 2);
        app.join_settings.gap = 5.0;
        assert_eq!(app.join_curves(), 1);
        assert_eq!(app.selection, vec![b]);
        assert!(app.doc().find_shape(a).is_none());
        let s = app.selected_shapes()[0].clone();
        let pts: Vec<_> = tracedraw_core::nodes::nodes(&s.page_path())
            .into_iter()
            .map(|n| n.pos)
            .collect();
        assert!(
            pts.contains(&tracedraw_core::geometry::Point::new(10.0, 0.0)),
            "{pts:?}"
        );
    }
}
