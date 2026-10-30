//! The nodes the Shape tool shows on rectangles, ellipses and polygons,
//! and what dragging them does: a rectangle's
//! corner nodes round, scallop or chamfer its corners (every corner by the
//! same amount, one corner with Ctrl, or the chosen corners when the node
//! dragged is one of them); an
//! ellipse's node opens a pie when the pointer is inside the ellipse and
//! an arc outside it; a polygon's nodes move with all their mirrored
//! counterparts (vertices together, the points between them together).
//! A B-spline shows its control points instead of its nodes: dragging
//! one reshapes the spline, a double-click on the control polygon adds
//! one there and a double-click on a control point removes it.

use tracedraw_core::{
    document::{axis_scale, EllipseArc, Shape, ShapeKind},
    geometry::{Point, Rect, Vec2},
};

/// A node of a rectangle, ellipse or polygon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KindNode {
    /// The end of a rectangle corner's cut on the vertical edge
    /// (`vertical`) or on the horizontal one. A square corner has one node.
    Corner { corner: usize, vertical: bool },
    /// The node of a whole ellipse, or the start of a pie or arc.
    EllipseStart,
    /// The end of a pie or arc.
    EllipseEnd,
    /// A polygon's vertex (`outer`) or the point between two vertices.
    Polygon { outer: bool, index: usize },
    /// A B-spline's control point (its index in [`Shape::spline`]).
    Spline { index: usize },
}

/// The nodes of `s` in page space; empty for other kinds of objects.
pub fn nodes(s: &Shape) -> Vec<(KindNode, Point)> {
    let t = s.transform;
    let mut out = Vec::new();
    if let Some(pts) = &s.spline {
        return pts
            .iter()
            .enumerate()
            .map(|(index, p)| (KindNode::Spline { index }, t * *p))
            .collect();
    }
    match &s.kind {
        ShapeKind::Rect { .. } => {
            let Some((rect, c)) = s.kind.rect_corners() else {
                return out;
            };
            for (corner, (p, v, h)) in c.ends(rect, axis_scale(t)).into_iter().enumerate() {
                if (v - p).hypot() < 1e-12 && (h - p).hypot() < 1e-12 {
                    out.push((
                        KindNode::Corner {
                            corner,
                            vertical: true,
                        },
                        t * p,
                    ));
                } else {
                    out.push((
                        KindNode::Corner {
                            corner,
                            vertical: true,
                        },
                        t * v,
                    ));
                    out.push((
                        KindNode::Corner {
                            corner,
                            vertical: false,
                        },
                        t * h,
                    ));
                }
            }
        }
        ShapeKind::Ellipse { rect, arc } => {
            let at = |deg: f64| t * on_ellipse(*rect, deg);
            match arc {
                None => out.push((KindNode::EllipseStart, at(90.0))),
                Some(a) => {
                    out.push((KindNode::EllipseStart, at(a.start_deg)));
                    out.push((KindNode::EllipseEnd, at(a.end_deg)));
                }
            }
        }
        ShapeKind::Polygon {
            rect,
            points,
            sharpness,
            complex,
            twist,
        } => {
            let n = (*points).max(3) as usize;
            let inner = inner_ratio(n, *sharpness);
            let c = rect.center();
            let (rx, ry) = (rect.width() / 2.0, rect.height() / 2.0);
            for i in 0..n {
                let a = vertex_angle(n, i);
                let b = a - std::f64::consts::PI / n as f64 + clamp_twist(*twist);
                out.push((
                    KindNode::Polygon {
                        outer: true,
                        index: i,
                    },
                    t * Point::new(c.x + rx * a.cos(), c.y + ry * a.sin()),
                ));
                // A complex star has its vertices only.
                if complex.is_some() {
                    continue;
                }
                out.push((
                    KindNode::Polygon {
                        outer: false,
                        index: i,
                    },
                    t * Point::new(c.x + rx * inner * b.cos(), c.y + ry * inner * b.sin()),
                ));
            }
        }
        _ => {}
    }
    out
}

/// The node of `s` within `tol` (page mm) of `p`.
pub fn node_at(s: &Shape, p: Point, tol: f64) -> Option<KindNode> {
    nodes(s)
        .into_iter()
        .map(|(n, q)| (n, (q - p).hypot()))
        .filter(|(_, d)| *d <= tol)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(n, _)| n)
}

/// Point of the ellipse inscribed in `rect` at a parametric angle.
fn on_ellipse(rect: Rect, deg: f64) -> Point {
    let c = rect.center();
    let a = deg.to_radians();
    Point::new(
        c.x + rect.width() / 2.0 * a.cos(),
        c.y + rect.height() / 2.0 * a.sin(),
    )
}

/// Angle of vertex `i` of an `n`-sided polygon: the first at the top, then
/// clockwise (as `geometry::polygon_path` draws them).
fn vertex_angle(n: usize, i: usize) -> f64 {
    std::f64::consts::FRAC_PI_2 - i as f64 * std::f64::consts::TAU / n as f64
}

/// Distance of the points between vertices from the centre, as a part of
/// the vertices' distance: the star's inner points, or the middle of a
/// polygon's sides.
fn inner_ratio(n: usize, sharpness: f64) -> f64 {
    if sharpness > 0.0 {
        1.0 - sharpness.clamp(0.0, 1.0)
    } else {
        (std::f64::consts::PI / n as f64).cos()
    }
}

/// A twist as the polygon path draws it: within half a turn.
fn clamp_twist(twist: f64) -> f64 {
    if twist.is_finite() {
        twist.clamp(-std::f64::consts::PI, std::f64::consts::PI)
    } else {
        0.0
    }
}

/// Wrap degrees into (-180, 180].
fn wrap(deg: f64) -> f64 {
    let d = deg.rem_euclid(360.0);
    if d > 180.0 {
        d - 360.0
    } else {
        d
    }
}

/// The kind of `start` after dragging `node` to `p` (page space). `single`
/// changes one rectangle corner only; otherwise, when the corner dragged
/// is one of `chosen`, the chosen corners change by the same amount, else
/// all four do. `turn` carries the angle (degrees)
/// a whole ellipse's node has turned through since the drag began, which
/// tells clockwise from counter-clockwise.
pub fn drag(
    start: &Shape,
    node: KindNode,
    p: Point,
    single: bool,
    chosen: [bool; 4],
    turn: &mut f64,
) -> Option<ShapeKind> {
    drag_constrained(start, node, p, single, chosen, turn, 15.0)
}

/// [`drag`] with Ctrl (`single`) held: one rectangle corner, a polygon's
/// points keeping their turn, and an ellipse's pie or arc angle in steps
/// of `step` degrees (the constrain angle, #1340).
pub fn drag_constrained(
    start: &Shape,
    node: KindNode,
    p: Point,
    single: bool,
    chosen: [bool; 4],
    turn: &mut f64,
    step: f64,
) -> Option<ShapeKind> {
    let snap = |deg: f64| {
        if single && step.is_finite() && step > 0.01 {
            (deg / step).round() * step
        } else {
            deg
        }
    };
    let inv = start.transform.inverse();
    let lp = inv * p;
    match (&start.kind, node) {
        (ShapeKind::Rect { .. }, KindNode::Corner { corner, vertical }) => {
            let (rect, c) = start.kind.rect_corners()?;
            let rect = rect.abs();
            let page = start.page_corners()?;
            let (sx, sy) = axis_scale(start.transform);
            let (cp, _, _) = *c.ends(rect, (sx, sy)).get(corner)?;
            // Edge directions from the corner towards the rectangle's middle.
            let mid = rect.center();
            let toward = |a: f64, b: f64| if b >= a { 1.0 } else { -1.0 };
            let up = Vec2::new(0.0, toward(cp.y, mid.y));
            let across = Vec2::new(toward(cp.x, mid.x), 0.0);
            let d = lp - cp;
            // Page millimetres per unit of the drag along each edge.
            let k = sx.min(sy);
            let (fv, fh) = if c.fixed { (sy, sx) } else { (k, k) };
            let along_v = d.dot(up).max(0.0) * fv;
            let along_h = d.dot(across).max(0.0) * fh;
            let square = page.radii.get(corner).is_some_and(|r| *r <= 1e-9);
            let value = if square {
                along_v.max(along_h)
            } else if vertical {
                along_v
            } else {
                along_h
            };
            let mut next = page;
            if single {
                next.radii[corner] = value;
            } else {
                let delta = value - page.radii[corner];
                let all = !chosen.get(corner).copied().unwrap_or(false);
                for (i, r) in next.radii.iter_mut().enumerate() {
                    if all || chosen[i] {
                        *r = (page.radii[i] + delta).max(0.0);
                    }
                }
            }
            start.with_page_corners(next)
        }
        (ShapeKind::Ellipse { rect, arc }, KindNode::EllipseStart | KindNode::EllipseEnd) => {
            let c = rect.center();
            let (rx, ry) = (rect.width() / 2.0, rect.height() / 2.0);
            if rx.abs() < 1e-9 || ry.abs() < 1e-9 {
                return None;
            }
            let (ux, uy) = ((lp.x - c.x) / rx, (lp.y - c.y) / ry);
            let angle = uy.atan2(ux).to_degrees();
            let pie = ux * ux + uy * uy < 1.0;
            let next = match arc {
                None => {
                    // The node starts at the top; the side it turns to
                    // decides which end it becomes.
                    *turn += wrap(angle - (90.0 + *turn));
                    // With Ctrl the node goes in steps from the top.
                    let t = snap(*turn);
                    if t.abs() < 0.5 {
                        return Some(start.kind.clone());
                    }
                    if t < 0.0 {
                        EllipseArc {
                            start_deg: 90.0,
                            end_deg: 90.0 + t,
                            pie,
                        }
                    } else {
                        EllipseArc {
                            start_deg: 90.0 + t,
                            end_deg: 90.0,
                            pie,
                        }
                    }
                }
                Some(a) => {
                    let mut a = *a;
                    if node == KindNode::EllipseStart {
                        a.start_deg = snap(angle);
                    } else {
                        a.end_deg = snap(angle);
                    }
                    a.pie = pie;
                    a
                }
            };
            let norm = |d: f64| {
                let d = d.rem_euclid(360.0);
                if (d - 360.0).abs() < 1e-9 {
                    0.0
                } else {
                    d
                }
            };
            Some(ShapeKind::Ellipse {
                rect: *rect,
                arc: Some(EllipseArc {
                    start_deg: norm(next.start_deg),
                    end_deg: norm(next.end_deg),
                    pie: next.pie,
                }),
            })
        }
        (
            ShapeKind::Polygon {
                rect,
                points,
                sharpness,
                complex,
                twist,
            },
            KindNode::Polygon { outer, index },
        ) => {
            let n = (*points).max(3) as usize;
            let c = rect.center();
            let (rx, ry) = (rect.width() / 2.0, rect.height() / 2.0);
            if rx.abs() < 1e-9 || ry.abs() < 1e-9 {
                return None;
            }
            // Distance from the centre as a part of the vertices' distance.
            let rho = ((lp.x - c.x) / rx).hypot((lp.y - c.y) / ry);
            let inner = inner_ratio(n, *sharpness);
            if outer {
                let rho = rho.max(0.01);
                if (rho - 1.0).abs() < 1e-6 {
                    return Some(start.kind.clone());
                }
                let grown = Rect::new(
                    c.x - rx * rho,
                    c.y - ry * rho,
                    c.x + rx * rho,
                    c.y + ry * rho,
                );
                if complex.is_some() {
                    // A complex star grows or shrinks about its centre.
                    return Some(ShapeKind::Polygon {
                        rect: grown,
                        points: *points,
                        sharpness: *sharpness,
                        complex: *complex,
                        twist: *twist,
                    });
                }
                // The vertices move, the points between them stay (and
                // keep their turn).
                let k = inner / rho;
                Some(ShapeKind::Polygon {
                    rect: grown,
                    points: *points,
                    sharpness: (1.0 - k).clamp(0.001, 0.999),
                    complex: None,
                    twist: *twist,
                })
            } else if complex.is_some() {
                None
            } else {
                // In or out sharpens the star; around the centre turns the
                // points between the vertices (a twisted star), unless
                // Ctrl keeps the turn they have.
                let twist = if single {
                    *twist
                } else {
                    let untwisted = vertex_angle(n, index) - std::f64::consts::PI / n as f64;
                    let at = ((lp.y - c.y) / ry).atan2((lp.x - c.x) / rx);
                    let turn = wrap((at - untwisted).to_degrees()).to_radians();
                    // A drag straight in or out stays untwisted.
                    if turn.abs() < 1f64.to_radians() {
                        0.0
                    } else {
                        clamp_twist(turn)
                    }
                };
                Some(ShapeKind::Polygon {
                    rect: *rect,
                    points: *points,
                    sharpness: (1.0 - rho).clamp(0.001, 0.999),
                    complex: None,
                    twist,
                })
            }
        }
        _ => None,
    }
}

/// The undo step's name for dragging a node of `s`.
pub fn label(s: &Shape) -> &'static str {
    if s.spline.is_some() {
        return "Edit B-Spline";
    }
    match s.kind {
        ShapeKind::Rect { .. } => "Corner Radius",
        ShapeKind::Ellipse { .. } => "Pie / Arc",
        _ => "Edit Polygon",
    }
}

/// The node whose drag changes one corner only: a corner node with Ctrl.
pub fn single_corner(node: KindNode, ctrl: bool) -> bool {
    matches!(node, KindNode::Corner { .. }) && ctrl
}

/// The corners of the rectangle `s` a drag of `node` changes together:
/// the corners chosen in the Shape tool (`selected`) when the corner
/// dragged is one of them, else none (all four then change).
pub fn chosen_corners(
    selected: &[(tracedraw_core::ShapeId, usize)],
    s: &Shape,
    node: KindNode,
) -> [bool; 4] {
    let mut out = [false; 4];
    let KindNode::Corner { corner, .. } = node else {
        return out;
    };
    if !selected.contains(&(s.id, corner)) {
        return out;
    }
    for &(id, c) in selected {
        if id == s.id && c < 4 {
            out[c] = true;
        }
    }
    out
}

/// The corners of the rectangles among `shapes` with a corner node for
/// which `inside` holds (page space), as the Shape tool chooses them.
pub fn corners_where(
    shapes: &[Shape],
    inside: impl Fn(Point) -> bool,
) -> Vec<(tracedraw_core::ShapeId, usize)> {
    let mut out = Vec::new();
    for s in shapes {
        if !matches!(s.kind, ShapeKind::Rect { .. }) || s.spline.is_some() {
            continue;
        }
        for (node, q) in nodes(s) {
            if let KindNode::Corner { corner, .. } = node {
                if inside(q) && !out.contains(&(s.id, corner)) {
                    out.push((s.id, corner));
                }
            }
        }
    }
    out
}

/// The page size of corner `corner` of the rectangle `s`.
#[cfg(test)]
fn page_radius(s: &Shape, corner: usize) -> f64 {
    s.page_corners().map(|c| c.radii[corner]).unwrap_or(0.0)
}

/// The control points of the B-spline `start` with the point `index`
/// moved to `p` (page space): what [`Command::SetSpline`] takes for a
/// dragged control point.
pub fn spline_moved(start: &Shape, index: usize, p: Point) -> Option<Vec<Point>> {
    let mut pts = start.spline.clone()?;
    let lp = start.transform.inverse() * p;
    if !lp.is_finite() {
        return None;
    }
    *pts.get_mut(index)? = lp;
    Some(pts)
}

/// The control points of the B-spline `s` with one more at `p` (page
/// space), when `p` lies within `tol` of its control polygon (the
/// dashed lines between the control points; the closing line of a closed
/// spline too): the new point goes between the two the line joins, at
/// the foot of `p` on it.
pub fn spline_with_point(s: &Shape, p: Point, tol: f64) -> Option<Vec<Point>> {
    let pts = s.spline.as_ref()?;
    let closed = matches!(s.kind, ShapeKind::Path { closed: true, .. });
    let n = pts.len();
    let lines = if closed { n } else { n.saturating_sub(1) };
    let mut best: Option<(usize, Point, f64)> = None;
    for i in 0..lines {
        let a = s.transform * pts[i];
        let b = s.transform * pts[(i + 1) % n];
        let ab = b - a;
        let len2 = ab.hypot2();
        let t = if len2 < 1e-12 {
            0.5
        } else {
            ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
        };
        let foot = a + ab * t;
        let d = (foot - p).hypot();
        if d <= tol && best.is_none_or(|b| d < b.2) {
            best = Some((i, foot, d));
        }
    }
    let (i, foot, _) = best?;
    let lp = s.transform.inverse() * foot;
    if !lp.is_finite() {
        return None;
    }
    let mut out = pts.clone();
    out.insert(i + 1, lp);
    Some(out)
}

/// The control points of the B-spline `start` with the points `indices`
/// moved together, so that the point `grab` lands on `p` (page space):
/// several chosen control points dragged as one.
pub fn spline_moved_many(
    start: &Shape,
    indices: &[usize],
    grab: usize,
    p: Point,
) -> Option<Vec<Point>> {
    let mut pts = start.spline.clone()?;
    let lp = start.transform.inverse() * p;
    if !lp.is_finite() {
        return None;
    }
    let d = lp - *pts.get(grab)?;
    for &i in indices {
        if let Some(q) = pts.get_mut(i) {
            *q += d;
        }
    }
    Some(pts)
}

/// The control points of the B-spline `s` without the points `indices`;
/// `None` when fewer than two would remain.
pub fn spline_without_many(s: &Shape, indices: &[usize]) -> Option<Vec<Point>> {
    let pts = s.spline.as_ref()?;
    let out: Vec<Point> = pts
        .iter()
        .enumerate()
        .filter(|(i, _)| !indices.contains(i))
        .map(|(_, p)| *p)
        .collect();
    (out.len() >= 2).then_some(out)
}

/// The control points of the B-spline `s` without the point `index`;
/// `None` when two would not remain.
pub fn spline_without(s: &Shape, index: usize) -> Option<Vec<Point>> {
    let pts = s.spline.as_ref()?;
    if pts.len() <= 2 || index >= pts.len() {
        return None;
    }
    let mut out = pts.clone();
    out.remove(index);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{geometry::Affine, CornerKind, Corners, ShapeId};

    fn rect(corners: Corners) -> Shape {
        Shape::new(
            ShapeId(1),
            ShapeKind::rect_with_corners(Rect::new(0.0, 0.0, 40.0, 20.0), corners),
        )
    }

    #[test]
    fn square_corners_have_one_node_rounded_ones_two() {
        let s = rect(Corners::default());
        assert_eq!(nodes(&s).len(), 4);
        let mut c = Corners::uniform(0.0, CornerKind::Round);
        c.radii[Corners::TOP_LEFT] = 5.0;
        let s = rect(c);
        let n = nodes(&s);
        assert_eq!(n.len(), 5);
        assert!(n.contains(&(
            KindNode::Corner {
                corner: Corners::TOP_LEFT,
                vertical: true
            },
            Point::new(0.0, 15.0)
        )));
        assert!(n.contains(&(
            KindNode::Corner {
                corner: Corners::TOP_LEFT,
                vertical: false
            },
            Point::new(5.0, 20.0)
        )));
    }

    #[test]
    fn dragging_a_corner_rounds_all_corners_or_one() {
        let s = rect(Corners::default());
        let node = node_at(&s, Point::new(40.3, 19.8), 1.0).expect("top right node");
        assert_eq!(
            node,
            KindNode::Corner {
                corner: Corners::TOP_RIGHT,
                vertical: true
            }
        );
        let mut turn = 0.0;
        // Along the top edge towards the middle: 6 mm.
        let k = drag(
            &s,
            node,
            Point::new(34.0, 20.0),
            false,
            [false; 4],
            &mut turn,
        )
        .unwrap();
        let mut moved = s.clone();
        moved.kind = k;
        for i in 0..4 {
            assert!((page_radius(&moved, i) - 6.0).abs() < 1e-9);
        }
        // Ctrl: that corner only; others keep theirs.
        let k = drag(
            &s,
            node,
            Point::new(40.0, 13.0),
            true,
            [false; 4],
            &mut turn,
        )
        .unwrap();
        moved.kind = k;
        assert!((page_radius(&moved, Corners::TOP_RIGHT) - 7.0).abs() < 1e-9);
        assert_eq!(page_radius(&moved, Corners::BOTTOM_LEFT), 0.0);
        // Never past half the shorter side, never below zero.
        let k = drag(
            &s,
            node,
            Point::new(0.0, 20.0),
            false,
            [false; 4],
            &mut turn,
        )
        .unwrap();
        moved.kind = k;
        assert!((page_radius(&moved, 0) - 10.0).abs() < 1e-9);
        let k = drag(
            &s,
            node,
            Point::new(60.0, 30.0),
            false,
            [false; 4],
            &mut turn,
        )
        .unwrap();
        moved.kind = k;
        assert_eq!(page_radius(&moved, 0), 0.0);
    }

    #[test]
    fn rounded_corners_grow_by_the_same_amount() {
        let mut c = Corners::uniform(2.0, CornerKind::Scallop);
        c.radii[Corners::BOTTOM_RIGHT] = 4.0;
        let s = rect(c);
        let node = KindNode::Corner {
            corner: Corners::TOP_LEFT,
            vertical: false,
        };
        let mut turn = 0.0;
        // The top left end on the top edge moves from x = 2 to x = 5.
        let k = drag(
            &s,
            node,
            Point::new(5.0, 21.0),
            false,
            [false; 4],
            &mut turn,
        )
        .unwrap();
        let (_, got) = k.rect_corners().unwrap();
        assert_eq!(got.kind, CornerKind::Scallop);
        assert_eq!(got.radii, [5.0, 5.0, 5.0, 7.0]);
    }

    #[test]
    fn corners_drag_in_page_units_on_scaled_rectangles() {
        let mut s = rect(Corners::default());
        s.transform = Affine::scale(2.0);
        let node = KindNode::Corner {
            corner: Corners::BOTTOM_LEFT,
            vertical: true,
        };
        let mut turn = 0.0;
        // 8 mm up the left edge on the page.
        let k = drag(&s, node, Point::new(0.0, 8.0), false, [false; 4], &mut turn).unwrap();
        s.kind = k;
        assert!((page_radius(&s, Corners::BOTTOM_LEFT) - 8.0).abs() < 1e-9);
        assert!(
            matches!(s.kind, ShapeKind::Rect { radius, corners: None, .. } if (radius - 4.0).abs() < 1e-9)
        );
    }

    #[test]
    fn the_ellipse_node_opens_a_pie_inside_and_an_arc_outside() {
        let s = Shape::new(
            ShapeId(1),
            ShapeKind::Ellipse {
                rect: Rect::new(-10.0, -10.0, 10.0, 10.0),
                arc: None,
            },
        );
        let n = nodes(&s);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].0, KindNode::EllipseStart);
        assert!((n[0].1 - Point::new(0.0, 10.0)).hypot() < 1e-9);
        // Clockwise to 0 degrees, inside: a pie from 90 round to 0.
        let mut turn = 0.0;
        for deg in [80.0f64, 45.0, 0.0] {
            let p = Point::new(5.0 * deg.to_radians().cos(), 5.0 * deg.to_radians().sin());
            let k = drag(&s, KindNode::EllipseStart, p, false, [false; 4], &mut turn).unwrap();
            if deg == 0.0 {
                assert_eq!(
                    k,
                    ShapeKind::Ellipse {
                        rect: Rect::new(-10.0, -10.0, 10.0, 10.0),
                        arc: Some(EllipseArc {
                            start_deg: 90.0,
                            end_deg: 0.0,
                            pie: true
                        })
                    }
                );
            }
        }
        // Counter-clockwise past the left, outside: an arc from 200 to 90.
        let mut turn = 0.0;
        for deg in [120.0f64, 160.0, 200.0] {
            let p = Point::new(15.0 * deg.to_radians().cos(), 15.0 * deg.to_radians().sin());
            let k = drag(&s, KindNode::EllipseStart, p, false, [false; 4], &mut turn).unwrap();
            if deg == 200.0 {
                let ShapeKind::Ellipse { arc: Some(a), .. } = k else {
                    panic!("{k:?}");
                };
                assert!(!a.pie);
                assert!(
                    (a.start_deg - 200.0).abs() < 1e-9 && a.end_deg == 90.0,
                    "{a:?}"
                );
            }
        }
        // An arc's end node moves its end.
        let mut arc = s.clone();
        arc.kind = ShapeKind::Ellipse {
            rect: Rect::new(-10.0, -10.0, 10.0, 10.0),
            arc: Some(EllipseArc {
                start_deg: 0.0,
                end_deg: 270.0,
                pie: true,
            }),
        };
        assert_eq!(nodes(&arc).len(), 2);
        let k = drag(
            &arc,
            KindNode::EllipseEnd,
            Point::new(0.0, 3.0),
            false,
            [false; 4],
            &mut turn,
        )
        .unwrap();
        assert!(
            matches!(k, ShapeKind::Ellipse { arc: Some(a), .. } if (a.end_deg - 90.0).abs() < 1e-9 && a.pie)
        );
    }

    #[test]
    fn polygon_nodes_move_together() {
        let s = Shape::new(
            ShapeId(1),
            ShapeKind::Polygon {
                rect: Rect::new(-10.0, -10.0, 10.0, 10.0),
                points: 5,
                sharpness: 0.0,
                complex: None,
                twist: 0.0,
            },
        );
        let n = nodes(&s);
        assert_eq!(n.len(), 10);
        assert_eq!(
            n[0].0,
            KindNode::Polygon {
                outer: true,
                index: 0
            }
        );
        assert!((n[0].1 - Point::new(0.0, 10.0)).hypot() < 1e-9);
        let mut turn = 0.0;
        // A point between two vertices pulled halfway in: a star.
        let between = KindNode::Polygon {
            outer: false,
            index: 2,
        };
        let k = drag(
            &s,
            between,
            Point::new(0.0, -5.0),
            false,
            [false; 4],
            &mut turn,
        )
        .unwrap();
        assert!(
            matches!(k, ShapeKind::Polygon { points: 5, sharpness, .. } if (sharpness - 0.5).abs() < 1e-9)
        );
        // A vertex pulled out: the vertices move, the points between stay.
        let top = KindNode::Polygon {
            outer: true,
            index: 0,
        };
        let k = drag(&s, top, Point::new(0.0, 20.0), false, [false; 4], &mut turn).unwrap();
        let ShapeKind::Polygon {
            rect, sharpness, ..
        } = k
        else {
            panic!("{k:?}");
        };
        assert!((rect.width() - 40.0).abs() < 1e-9);
        let inner = (std::f64::consts::PI / 5.0).cos() * 10.0;
        assert!(((1.0 - sharpness) * 20.0 - inner).abs() < 1e-9);
    }

    #[test]
    fn points_between_the_vertices_twist_the_star() {
        let s = Shape::new(
            ShapeId(1),
            ShapeKind::Polygon {
                rect: Rect::new(-10.0, -10.0, 10.0, 10.0),
                points: 5,
                sharpness: 0.5,
                complex: None,
                twist: 0.0,
            },
        );
        let mut turn = 0.0;
        let between = KindNode::Polygon {
            outer: false,
            index: 0,
        };
        // The node between the first two vertices sits 36 degrees right of
        // the top; pulled to 20 degrees right of the top, half way out.
        let untwisted = std::f64::consts::FRAC_PI_2 - std::f64::consts::PI / 5.0;
        let at = std::f64::consts::FRAC_PI_2 - 20f64.to_radians();
        let p = Point::new(5.0 * at.cos(), 5.0 * at.sin());
        let k = drag(&s, between, p, false, [false; 4], &mut turn).expect("dragged");
        let ShapeKind::Polygon {
            sharpness, twist, ..
        } = k
        else {
            panic!("{k:?}");
        };
        assert!((sharpness - 0.5).abs() < 1e-9);
        assert!((twist - (at - untwisted)).abs() < 1e-9, "{twist}");
        // The nodes show where the points are now.
        let twisted = Shape {
            kind: k.clone(),
            ..s.clone()
        };
        let shown = nodes(&twisted)
            .into_iter()
            .find(|(n, _)| *n == between)
            .map(|(_, q)| q)
            .expect("node");
        assert!((shown - p).hypot() < 1e-9);
        // Ctrl keeps the turn; straight in or out stays untwisted.
        let k = drag(
            &twisted,
            between,
            Point::new(0.0, 3.0),
            true,
            [false; 4],
            &mut turn,
        )
        .expect("ctrl");
        assert!(
            matches!(k, ShapeKind::Polygon { twist, .. } if (twist - (at - untwisted)).abs() < 1e-9)
        );
        let radial = Point::new(4.0 * untwisted.cos(), 4.0 * untwisted.sin());
        let k = drag(&s, between, radial, false, [false; 4], &mut turn).expect("radial");
        assert!(matches!(k, ShapeKind::Polygon { twist, .. } if twist == 0.0));
        // A vertex pulled out keeps the twist.
        let top = KindNode::Polygon {
            outer: true,
            index: 0,
        };
        let k = drag(
            &twisted,
            top,
            Point::new(0.0, 15.0),
            false,
            [false; 4],
            &mut turn,
        )
        .expect("vertex");
        assert!(
            matches!(k, ShapeKind::Polygon { twist, .. } if (twist - (at - untwisted)).abs() < 1e-9)
        );
    }

    #[test]
    fn single_corner_needs_ctrl_and_chosen_corners_go_together() {
        let s = rect(Corners::default());
        let node = KindNode::Corner {
            corner: 1,
            vertical: true,
        };
        assert!(single_corner(node, true));
        assert!(!single_corner(node, false));
        assert!(!single_corner(KindNode::EllipseStart, true));
        // The chosen corners, when the corner dragged is one of them.
        let chosen = [(s.id, 0), (s.id, 1), (ShapeId(999), 2)];
        assert_eq!(
            chosen_corners(&chosen, &s, node),
            [true, true, false, false]
        );
        let other = KindNode::Corner {
            corner: 3,
            vertical: false,
        };
        assert_eq!(chosen_corners(&chosen, &s, other), [false; 4]);
    }

    /// Two chosen corners round together by the same amount and the
    /// others stay square (#671).
    #[test]
    fn chosen_corners_round_together() {
        let s = rect(Corners::default());
        let node = node_at(&s, Point::new(40.3, 19.8), 1.0).expect("top right node");
        let KindNode::Corner { corner, .. } = node else {
            panic!("a corner");
        };
        let mut chosen = [false; 4];
        chosen[corner] = true;
        chosen[Corners::TOP_LEFT] = true;
        let mut turn = 0.0;
        let k = drag(&s, node, Point::new(34.0, 20.0), false, chosen, &mut turn).unwrap();
        let mut moved = s.clone();
        moved.kind = k;
        for i in 0..4 {
            let want = if chosen[i] { 6.0 } else { 0.0 };
            assert!((page_radius(&moved, i) - want).abs() < 1e-9, "{i}");
        }
        // Corners found inside a region.
        let all = corners_where(std::slice::from_ref(&s), |_| true);
        assert_eq!(all.len(), 4);
        let top = corners_where(std::slice::from_ref(&s), |q| q.y > 10.0);
        let mut got: Vec<usize> = top.iter().map(|k| k.1).collect();
        got.sort();
        let mut want = vec![Corners::TOP_LEFT, Corners::TOP_RIGHT];
        want.sort();
        assert_eq!(got, want);
    }
}
