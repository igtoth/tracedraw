//! Connector anchors and the Anchor Editing tool.
//!
//! Every object has four implicit anchors, the midpoints of its bounding
//! box sides. Custom anchors live in the object data (`anchor.<n>` =
//! `fx,fy`, fractions of the bounding box, so they move and scale with
//! the object). Connectors start and end at the anchor nearest to where the
//! drag began and ended; an anchor marked `!` (auto) is one the reference
//! editor would also pick first, which here is every custom anchor.
//!
//! Anchor Editing tool: click an object to show its anchors, click inside
//! it to add one, drag an anchor to move it, Delete removes the selected
//! anchor, double-click an anchor removes it.

use crate::app::{App, Drag};
use crate::i18n::tr;
use egui::{PointerButton, Response};
use tracedraw_core::{geometry::Point, Command, ShapeId};

const PREFIX: &str = "anchor.";
/// Hit radius for anchors, in mm at the current zoom (converted by caller).
const HIT_PX: f64 = 6.0;

/// Parse the custom anchors (fractions of the bounds) from object data.
pub fn custom_anchors(data: &[(String, String)]) -> Vec<(f64, f64)> {
    let mut out: Vec<(u32, (f64, f64))> = data
        .iter()
        .filter_map(|(k, v)| {
            let n: u32 = k.strip_prefix(PREFIX)?.parse().ok()?;
            let (x, y) = v.split_once(',')?;
            Some((n, (x.trim().parse().ok()?, y.trim().parse().ok()?)))
        })
        .collect();
    out.sort_by_key(|(n, _)| *n);
    out.into_iter().map(|(_, p)| p).collect()
}

/// Object data with the custom anchors replaced by `anchors`.
pub fn with_anchors(
    mut data: Vec<(String, String)>,
    anchors: &[(f64, f64)],
) -> Vec<(String, String)> {
    data.retain(|(k, _)| !k.starts_with(PREFIX));
    for (i, (x, y)) in anchors.iter().enumerate() {
        data.push((format!("{PREFIX}{i}"), format!("{x:.4},{y:.4}")));
    }
    data
}

/// The four side midpoints as fractions.
pub const SIDE_ANCHORS: [(f64, f64); 4] = [(0.5, 1.0), (1.0, 0.5), (0.5, 0.0), (0.0, 0.5)];

impl App {
    /// All anchors of an object in page coordinates: the four side
    /// midpoints followed by the custom ones.
    pub fn anchors_of(&self, id: ShapeId) -> Vec<Point> {
        let Some(s) = self.doc().find_shape(id) else {
            return Vec::new();
        };
        let b = s.bounds();
        SIDE_ANCHORS
            .iter()
            .copied()
            .chain(custom_anchors(&s.data))
            .map(|(fx, fy)| Point::new(b.x0 + fx * b.width(), b.y0 + fy * b.height()))
            .collect()
    }

    /// The anchor of `id` closest to `p`.
    pub fn nearest_anchor(&self, id: ShapeId, p: Point) -> Option<Point> {
        self.anchors_of(id)
            .into_iter()
            .min_by(|a, b| (*a - p).hypot().total_cmp(&(*b - p).hypot()))
    }

    /// Connector end points: the anchor of `from` nearest the drag start
    /// and the anchor of `to` nearest the drop point, unless the two
    /// objects face each other more naturally through the closest pair.
    pub fn connector_anchors(
        &self,
        from: ShapeId,
        start: Point,
        to: ShapeId,
        end: Point,
    ) -> Option<(Point, Point)> {
        let a = self.anchors_of(from);
        let b = self.anchors_of(to);
        if a.is_empty() || b.is_empty() {
            return None;
        }
        let near = |pts: &[Point], p: Point| {
            pts.iter()
                .copied()
                .min_by(|x, y| (*x - p).hypot().total_cmp(&(*y - p).hypot()))
        };
        let (pa, pb) = (near(&a, start)?, near(&b, end)?);
        // A drag that started and ended well inside the objects (not on
        // an anchor) uses the closest anchor pair instead.
        let tol = self.anchor_tolerance();
        let on_a = (pa - start).hypot() <= tol;
        let on_b = (pb - end).hypot() <= tol;
        if on_a && on_b {
            return Some((pa, pb));
        }
        let mut best = (pa, pb);
        let mut best_d = f64::MAX;
        for x in &a {
            for y in &b {
                let d = (*x - *y).hypot();
                if d < best_d {
                    best_d = d;
                    best = (*x, *y);
                }
            }
        }
        Some((
            if on_a { pa } else { best.0 },
            if on_b { pb } else { best.1 },
        ))
    }

    /// Anchor hit radius in mm at the current zoom.
    pub fn anchor_tolerance(&self) -> f64 {
        HIT_PX / (self.view.zoom as f64).max(1e-6)
    }

    fn custom_anchor_at(&self, id: ShapeId, p: Point) -> Option<usize> {
        let Some(s) = self.doc().find_shape(id) else {
            return None;
        };
        let b = s.bounds();
        let tol = self.anchor_tolerance();
        custom_anchors(&s.data).iter().position(|(fx, fy)| {
            let q = Point::new(b.x0 + fx * b.width(), b.y0 + fy * b.height());
            (q - p).hypot() <= tol
        })
    }

    fn set_custom_anchors(&mut self, id: ShapeId, anchors: &[(f64, f64)]) {
        let Some(s) = self.doc().find_shape(id) else {
            return;
        };
        let data = with_anchors(s.data.clone(), anchors);
        let _ = self
            .engine
            .run_with_label(&Command::SetObjectData { shape: id, data }, "Anchor");
    }

    /// Add a custom anchor at a page point (clamped to the bounds).
    pub fn add_anchor(&mut self, id: ShapeId, p: Point) {
        let Some(s) = self.doc().find_shape(id) else {
            return;
        };
        let b = s.bounds();
        if b.width() <= 0.0 || b.height() <= 0.0 {
            return;
        }
        let fx = ((p.x - b.x0) / b.width()).clamp(0.0, 1.0);
        let fy = ((p.y - b.y0) / b.height()).clamp(0.0, 1.0);
        let mut anchors = custom_anchors(&s.data);
        anchors.push((fx, fy));
        self.set_custom_anchors(id, &anchors);
        self.anchor_sel = Some(anchors.len() - 1);
    }

    pub fn move_anchor(&mut self, id: ShapeId, index: usize, p: Point) {
        let Some(s) = self.doc().find_shape(id) else {
            return;
        };
        let b = s.bounds();
        if b.width() <= 0.0 || b.height() <= 0.0 {
            return;
        }
        let mut anchors = custom_anchors(&s.data);
        if let Some(a) = anchors.get_mut(index) {
            *a = (
                ((p.x - b.x0) / b.width()).clamp(0.0, 1.0),
                ((p.y - b.y0) / b.height()).clamp(0.0, 1.0),
            );
            self.set_custom_anchors(id, &anchors);
        }
    }

    pub fn remove_anchor(&mut self, id: ShapeId, index: usize) {
        let Some(s) = self.doc().find_shape(id) else {
            return;
        };
        let mut anchors = custom_anchors(&s.data);
        if index < anchors.len() {
            anchors.remove(index);
            self.set_custom_anchors(id, &anchors);
        }
        self.anchor_sel = None;
    }

    /// Property bar: drop every custom anchor of the object.
    pub fn clear_anchors(&mut self, id: ShapeId) {
        self.set_custom_anchors(id, &[]);
        self.anchor_sel = None;
    }

    /// Delete key while the Anchor Editing tool has an anchor selected.
    pub fn delete_selected_anchor(&mut self) -> bool {
        let (Some(id), Some(i)) = (self.selection.first().copied(), self.anchor_sel) else {
            return false;
        };
        self.remove_anchor(id, i);
        true
    }

    /// Anchor Editing tool input.
    pub fn anchor_input(&mut self, response: &Response, p: Point) {
        let target = self.selection.first().copied();
        if response.double_clicked_by(PointerButton::Primary) {
            if let Some(id) = target {
                if let Some(i) = self.custom_anchor_at(id, p) {
                    self.remove_anchor(id, i);
                    return;
                }
            }
        }
        if response.drag_started_by(PointerButton::Primary) {
            if let Some(id) = target {
                if let Some(i) = self.custom_anchor_at(id, p) {
                    self.anchor_sel = Some(i);
                    self.drag = Drag::Anchor {
                        shape: id,
                        index: i,
                    };
                    return;
                }
            }
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::Anchor { shape, index } = self.drag {
                self.move_anchor(shape, index, p);
                return;
            }
        }
        if response.drag_stopped_by(PointerButton::Primary) {
            if matches!(self.drag, Drag::Anchor { .. }) {
                self.drag = Drag::None;
                return;
            }
        }
        if response.clicked_by(PointerButton::Primary) {
            if let Some(id) = target {
                if let Some(i) = self.custom_anchor_at(id, p) {
                    self.anchor_sel = Some(i);
                    return;
                }
                if let Some(s) = self.doc().find_shape(id) {
                    if s.bounds().contains(p) {
                        self.add_anchor(id, p);
                        self.status = tr("status.anchor_added");
                        return;
                    }
                }
            }
            match self.hit_test(p) {
                Some(id) => {
                    self.select(vec![id]);
                    self.anchor_sel = None;
                    self.status = tr("status.anchor_hint");
                }
                None => {
                    self.select(Vec::new());
                    self.anchor_sel = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::document::ShapeKind;
    use tracedraw_core::geometry::Rect;

    fn rect(app: &mut App, r: Rect) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: r,
            radius: 0.0,
        })
        .unwrap()
    }

    #[test]
    fn anchors_round_trip_through_object_data() {
        let data = with_anchors(
            vec![("hinting".into(), "1".into())],
            &[(0.25, 0.75), (1.0, 0.0)],
        );
        assert_eq!(custom_anchors(&data), vec![(0.25, 0.75), (1.0, 0.0)]);
        assert!(data.iter().any(|(k, _)| k == "hinting"));
        let data = with_anchors(data, &[]);
        assert!(custom_anchors(&data).is_empty());
    }

    #[test]
    fn custom_anchor_scales_with_the_object_and_connectors_use_it() {
        let mut app = App::headless();
        let a = rect(&mut app, Rect::new(0.0, 0.0, 10.0, 10.0));
        let b = rect(&mut app, Rect::new(30.0, 0.0, 40.0, 10.0));
        app.add_anchor(a, Point::new(10.0, 10.0)); // top-right corner
        assert_eq!(app.anchors_of(a).len(), 5);
        assert_eq!(app.anchors_of(a)[4], Point::new(10.0, 10.0));
        app.select(vec![a]);
        app.transform_selection(tracedraw_core::Affine::scale(2.0));
        let anchors = app.anchors_of(a);
        let b_a = app.doc().find_shape(a).unwrap().bounds();
        assert!((anchors[4].x - b_a.x1).abs() < 1e-6 && (anchors[4].y - b_a.y1).abs() < 1e-6);
        // A connector dragged from the custom anchor ends at b's nearest side.
        let (pa, pb) = app
            .connector_anchors(a, anchors[4], b, Point::new(35.0, 5.0))
            .unwrap();
        assert_eq!(pa, anchors[4]);
        assert_eq!(pb, Point::new(30.0, 5.0));
        // Removing it leaves the four side anchors.
        app.remove_anchor(a, 0);
        assert_eq!(app.anchors_of(a).len(), 4);
    }
}
