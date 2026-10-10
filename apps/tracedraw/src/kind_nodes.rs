//! The nodes the Shape tool shows on rectangles, ellipses and polygons,
//! and what dragging them does: a rectangle's
//! corner nodes round, scallop or chamfer its corners (every corner by the
//! same amount, or one corner with Ctrl or once its node is selected); an
//! ellipse's node opens a pie when the pointer is inside the ellipse and
//! an arc outside it; a polygon's nodes move with all their mirrored
//! counterparts (vertices together, the points between them together).

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
}

/// The nodes of `s` in page space; empty for other kinds of objects.
pub fn nodes(s: &Shape) -> Vec<(KindNode, Point)> {
    let t = s.transform;
    let mut out = Vec::new();
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
        } => {
            let n = (*points).max(3) as usize;
            let inner = inner_ratio(n, *sharpness);
            let c = rect.center();
            let (rx, ry) = (rect.width() / 2.0, rect.height() / 2.0);
            for i in 0..n {
                let a = vertex_angle(n, i);
                let b = a - std::f64::consts::PI / n as f64;
                out.push((
                    KindNode::Polygon {
                        outer: true,
                        index: i,
                    },
                    t * Point::new(c.x + rx * a.cos(), c.y + ry * a.sin()),
                ));
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
/// changes one rectangle corner only. `turn` carries the angle (degrees)
/// a whole ellipse's node has turned through since the drag began, which
/// tells clockwise from counter-clockwise.
pub fn drag(
    start: &Shape,
    node: KindNode,
    p: Point,
    single: bool,
    turn: &mut f64,
) -> Option<ShapeKind> {
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
                next.radii = page.radii.map(|r| (r + delta).max(0.0));
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
                    if turn.abs() < 0.5 {
                        return Some(start.kind.clone());
                    }
                    if *turn < 0.0 {
                        EllipseArc {
                            start_deg: 90.0,
                            end_deg: 90.0 + *turn,
                            pie,
                        }
                    } else {
                        EllipseArc {
                            start_deg: 90.0 + *turn,
                            end_deg: 90.0,
                            pie,
                        }
                    }
                }
                Some(a) => {
                    let mut a = *a;
                    if node == KindNode::EllipseStart {
                        a.start_deg = angle;
                    } else {
                        a.end_deg = angle;
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
            },
            KindNode::Polygon { outer, .. },
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
                // The vertices move, the points between them stay.
                let k = inner / rho;
                Some(ShapeKind::Polygon {
                    rect: Rect::new(
                        c.x - rx * rho,
                        c.y - ry * rho,
                        c.x + rx * rho,
                        c.y + ry * rho,
                    ),
                    points: *points,
                    sharpness: (1.0 - k).clamp(0.001, 0.999),
                })
            } else {
                Some(ShapeKind::Polygon {
                    rect: *rect,
                    points: *points,
                    sharpness: (1.0 - rho).clamp(0.001, 0.999),
                })
            }
        }
        _ => None,
    }
}

/// The undo step's name for dragging a node of `s`.
pub fn label(s: &Shape) -> &'static str {
    match s.kind {
        ShapeKind::Rect { .. } => "Corner Radius",
        ShapeKind::Ellipse { .. } => "Pie / Arc",
        _ => "Edit Polygon",
    }
}

/// The node whose drag changes one corner only: Ctrl, or the corner node
/// the Shape tool selected with a click.
pub fn single_corner(
    selected: Option<(tracedraw_core::ShapeId, usize)>,
    s: &Shape,
    node: KindNode,
    ctrl: bool,
) -> bool {
    match node {
        KindNode::Corner { corner, .. } => ctrl || selected == Some((s.id, corner)),
        _ => false,
    }
}

/// The page size of corner `corner` of the rectangle `s`.
#[cfg(test)]
fn page_radius(s: &Shape, corner: usize) -> f64 {
    s.page_corners().map(|c| c.radii[corner]).unwrap_or(0.0)
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
        let k = drag(&s, node, Point::new(34.0, 20.0), false, &mut turn).unwrap();
        let mut moved = s.clone();
        moved.kind = k;
        for i in 0..4 {
            assert!((page_radius(&moved, i) - 6.0).abs() < 1e-9);
        }
        // Ctrl: that corner only; others keep theirs.
        let k = drag(&s, node, Point::new(40.0, 13.0), true, &mut turn).unwrap();
        moved.kind = k;
        assert!((page_radius(&moved, Corners::TOP_RIGHT) - 7.0).abs() < 1e-9);
        assert_eq!(page_radius(&moved, Corners::BOTTOM_LEFT), 0.0);
        // Never past half the shorter side, never below zero.
        let k = drag(&s, node, Point::new(0.0, 20.0), false, &mut turn).unwrap();
        moved.kind = k;
        assert!((page_radius(&moved, 0) - 10.0).abs() < 1e-9);
        let k = drag(&s, node, Point::new(60.0, 30.0), false, &mut turn).unwrap();
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
        let k = drag(&s, node, Point::new(5.0, 21.0), false, &mut turn).unwrap();
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
        let k = drag(&s, node, Point::new(0.0, 8.0), false, &mut turn).unwrap();
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
            let k = drag(&s, KindNode::EllipseStart, p, false, &mut turn).unwrap();
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
            let k = drag(&s, KindNode::EllipseStart, p, false, &mut turn).unwrap();
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
        let k = drag(&s, between, Point::new(0.0, -5.0), false, &mut turn).unwrap();
        assert!(
            matches!(k, ShapeKind::Polygon { points: 5, sharpness, .. } if (sharpness - 0.5).abs() < 1e-9)
        );
        // A vertex pulled out: the vertices move, the points between stay.
        let top = KindNode::Polygon {
            outer: true,
            index: 0,
        };
        let k = drag(&s, top, Point::new(0.0, 20.0), false, &mut turn).unwrap();
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
    fn single_corner_needs_ctrl_or_the_selected_node() {
        let s = rect(Corners::default());
        let node = KindNode::Corner {
            corner: 1,
            vertical: true,
        };
        assert!(single_corner(None, &s, node, true));
        assert!(!single_corner(None, &s, node, false));
        assert!(single_corner(Some((s.id, 1)), &s, node, false));
        assert!(!single_corner(Some((s.id, 2)), &s, node, false));
        assert!(!single_corner(
            Some((s.id, 1)),
            &s,
            KindNode::EllipseStart,
            false
        ));
    }
}
