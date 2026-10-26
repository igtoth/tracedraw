//! The Shape tool's corner widgets (#635): a small circle inside each
//! corner of the curves being edited; dragging one into the corner rounds
//! it, live, with the arc's middle under the pointer. With nodes chosen
//! only their corners show a widget and round; with none, every corner
//! rounds together. Open curves round their inner corners too. They show
//! only with Options > Nodes and Handles > corner widgets on (#884). See
//! docs/behavior/shape-tool.md, Corner widgets.

use crate::app::{App, Drag};
use crate::tools::Tool;
use tracedraw_core::corner_cut::{corners, cut_corners, max_fillet, Corner, CornerCut};
use tracedraw_core::geometry::{Affine, BezPath, Point};
use tracedraw_core::{Command, ShapeId, ShapeKind};

/// How far a widget sits from its node, along the corner's middle,
/// screen pixels.
pub const WIDGET_OFFSET_PX: f32 = 18.0;
/// The widget's radius and the reach of a press on it, screen pixels.
pub const WIDGET_RADIUS_PX: f32 = 4.0;
const WIDGET_PICK_PX: f32 = 7.0;
/// A curve with more corners than this shows widgets only on its chosen
/// nodes (a traced outline would be covered in them).
const MAX_WIDGETS: usize = 32;

/// The corners of the curves the Shape tool edits, worked out once per
/// change of the drawing or of the selection.
#[derive(Default)]
pub struct CornerCache {
    key: Option<(u64, Vec<ShapeId>)>,
    curves: Vec<(ShapeId, Affine, Vec<Corner>)>,
}

/// One widget on the screen: the curve, its corner, and where it is.
#[derive(Debug, Clone, Copy)]
pub struct Widget {
    pub shape: ShapeId,
    pub corner: Corner,
    pub at: egui::Pos2,
}

/// The drag of a corner widget: the curve as it was, the corners it
/// rounds (`None`: every corner), the grabbed corner (local space), the
/// largest radius they all take, and where the drag began (local).
#[derive(Debug, Clone, PartialEq)]
pub struct CornerDrag {
    pub shape: ShapeId,
    pub path: BezPath,
    pub closed: bool,
    pub only: Option<Vec<usize>>,
    pub corner: Corner,
    pub max: f64,
    pub from: Point,
    pub begun: bool,
}

/// The fillet radius whose arc's middle is `d` further into a corner of
/// angle `theta` than the corner was: the arc's middle lies
/// `r (1 / sin(theta / 2) - 1)` from the corner.
pub fn radius_for(d: f64, theta: f64) -> f64 {
    let k = 1.0 / (theta / 2.0).sin() - 1.0;
    if d > 0.0 && k > 1e-9 && k.is_finite() {
        d / k
    } else {
        0.0
    }
}

impl App {
    /// The corner widgets to show now, in screen space; none while one is
    /// dragged, and none at all unless Options > Nodes and Handles turns
    /// them on (#884).
    pub fn corner_widgets(&self) -> Vec<Widget> {
        if !self.settings.nodes.corner_widgets
            || self.tool != Tool::Shape
            || matches!(self.drag, Drag::CornerWidget(_))
        {
            return Vec::new();
        }
        let targets: Vec<ShapeId> = self
            .selection
            .iter()
            .map(|id| self.node_target(*id))
            .collect();
        let key = (self.engine.revision(), targets);
        let mut cache = self.corner_cache.borrow_mut();
        if cache.key.as_ref() != Some(&key) {
            cache.curves = key
                .1
                .iter()
                .filter_map(|id| {
                    let s = self.doc().find_shape(*id)?;
                    let ShapeKind::Path { path, .. } = &s.kind else {
                        return None;
                    };
                    if s.spline.is_some() {
                        return None;
                    }
                    let t = self.doc().parent_transform(*id)? * s.transform;
                    Some((*id, t, corners(path)))
                })
                .collect();
            cache.key = Some(key);
        }
        let mut out = Vec::new();
        for (id, t, list) in &cache.curves {
            let chosen: Vec<usize> = self
                .node_selection
                .iter()
                .filter(|(s, _)| s == id)
                .map(|(_, i)| *i)
                .collect();
            if chosen.is_empty() && list.len() > MAX_WIDGETS {
                continue;
            }
            for c in list {
                if !chosen.is_empty() && !chosen.contains(&c.node) {
                    continue;
                }
                let node = self.view.to_screen(*t * c.at);
                let toward = self.view.to_screen(*t * (c.at + c.inside));
                let d = toward - node;
                if d.length() < 1e-6 || !d.x.is_finite() || !d.y.is_finite() {
                    continue;
                }
                out.push(Widget {
                    shape: *id,
                    corner: *c,
                    at: node + d.normalized() * WIDGET_OFFSET_PX,
                });
            }
        }
        out
    }

    /// The corner widget under the page point `p`.
    pub fn corner_widget_at(&self, p: Point) -> Option<Widget> {
        let s = self.view.to_screen(p);
        self.corner_widgets()
            .into_iter()
            .map(|w| (w, (w.at - s).length()))
            .filter(|(_, d)| *d <= WIDGET_PICK_PX)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(w, _)| w)
    }

    /// Begin dragging the widget `w` from the page point `p`.
    pub fn start_corner_drag(&mut self, w: Widget, p: Point) -> bool {
        let Some(s) = self.doc().find_shape(w.shape) else {
            return false;
        };
        let ShapeKind::Path { path, closed } = &s.kind else {
            return false;
        };
        let Some(t) = self
            .doc()
            .parent_transform(w.shape)
            .map(|pt| pt * s.transform)
        else {
            return false;
        };
        if t.determinant().abs() < 1e-12 {
            return false;
        }
        let chosen: Vec<usize> = self
            .node_selection
            .iter()
            .filter(|(id, _)| *id == w.shape)
            .map(|(_, i)| *i)
            .collect();
        let only = (!chosen.is_empty()).then_some(chosen);
        let Some(max) = max_fillet(path, only.as_deref()) else {
            return false;
        };
        self.drag = Drag::CornerWidget(Box::new(CornerDrag {
            shape: w.shape,
            path: path.clone(),
            closed: *closed,
            only,
            corner: w.corner,
            max,
            from: t.inverse() * p,
            begun: false,
        }));
        // The cut renumbers the nodes.
        self.node_selection.clear();
        self.segment_spot = None;
        true
    }

    /// The widget dragged to the page point `p`: the corners round so the
    /// grabbed one's arc passes under the pointer, at most as far as
    /// every one of them still fits; one undo step for the drag.
    pub fn corner_drag_to(&mut self, mut d: CornerDrag, p: Point) {
        let Some(t) = self
            .doc()
            .find_shape(d.shape)
            .and_then(|s| Some(self.doc().parent_transform(d.shape)? * s.transform))
        else {
            return;
        };
        if t.determinant().abs() < 1e-12 {
            return;
        }
        let lp = t.inverse() * p;
        let along: f64 = (lp - d.from).dot(d.corner.inside);
        let r = radius_for(along, d.corner.theta).min(d.max * 0.999);
        let path = if r > 1e-6 {
            cut_corners(&d.path, CornerCut::Fillet(r), d.only.as_deref()).0
        } else {
            d.path.clone()
        };
        let cmd = Command::SetShapeKind {
            shape: d.shape,
            kind: ShapeKind::Path {
                path,
                closed: d.closed,
            },
        };
        let _ = if d.begun {
            self.engine.amend(&cmd)
        } else {
            self.engine.run_with_label(&cmd, "Fillet")
        };
        let u = self.units;
        let digits = self.settings.precision.min(10) as usize;
        self.status = crate::i18n::trf(
            "status.corner_radius",
            &[(
                "r",
                &format!("{:.*} {}", digits, u.world_from_mm(r.max(0.0)), u.label()),
            )],
        );
        d.begun = true;
        self.drag = Drag::CornerWidget(Box::new(d));
    }
}

/// Draw the corner widgets: white circles with a ring in the node
/// colour, over the nodes.
pub fn draw(app: &App, painter: &egui::Painter, color: egui::Color32) {
    for w in app.corner_widgets() {
        painter.circle(
            w.at,
            WIDGET_RADIUS_PX,
            egui::Color32::WHITE,
            egui::Stroke::new(1.5, color),
        );
        painter.circle_filled(w.at, 1.5, color);
    }
}
