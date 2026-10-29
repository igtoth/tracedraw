//! Fit Objects to Path: where each object, and each copy, goes along a
//! path, from the Fit Objects to Path panel's settings (#1027, #1091,
//! #1196). The formulas are in `docs/behavior/fit-to-path.md`.

use tracedraw_core::{
    document::Shape,
    geometry::{Affine, BezPath, Point, Vec2},
    ShapeId,
};

/// The most objects one fit places, copies included.
pub const MAX_PLACED: usize = 1000;

/// How Fit Objects to Path spaces the objects along the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitSpacing {
    /// Spread evenly over the path between the start and end offsets.
    Even,
    /// A fixed distance from one object to the next.
    Distance,
}

/// What the spacing is measured between.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMeasure {
    /// The centres of neighbouring objects.
    Centres,
    /// The facing edges of neighbouring objects, along the path: with
    /// even spacing every gap is the same whatever the objects' sizes.
    Edges,
}

/// Where the objects sit across the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitPlace {
    /// Centred on the path.
    On,
    /// On the left of the path's direction (above a path drawn from left
    /// to right), the near edge a set distance from it.
    Above,
    /// On the right of the path's direction (below it).
    Below,
}

/// How the objects are turned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitRotation {
    /// With the direction of the path, plus the angle.
    Follow,
    /// All by the same angle, whatever the path does.
    Fixed,
}

/// The order the objects go along the path in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitOrder {
    /// The order they were selected in.
    AsSelected,
    /// The selection order backwards.
    Reversed,
    /// By their centres, from left to right.
    LeftToRight,
    /// By their centres, from right to left.
    RightToLeft,
    /// By area, the largest first.
    LargestFirst,
    /// By area, the smallest first.
    SmallestFirst,
    /// Largest, smallest, next largest, next smallest and so on.
    Alternate,
}

/// Copies made along the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitCopies {
    /// Each selected object is placed once.
    None,
    /// `count` objects in all: the selected ones, then copies of them in
    /// the same order until the count is reached.
    Count,
    /// With a fixed distance: as many copies as fit on the path.
    Fill,
}

/// Where a run of objects at a fixed distance sits on the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitAlign {
    /// The first object at the start offset.
    Start,
    /// The run centred between the offsets (half way round a closed path,
    /// counted from the start offset).
    Middle,
    /// The last object at the end offset.
    End,
}

/// The Fit Objects to Path panel's settings (millimetres and degrees,
/// #1027, #1091, #1196).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitPathSettings {
    pub spacing: FitSpacing,
    pub measure: FitMeasure,
    /// The distance with [`FitSpacing::Distance`].
    pub distance: f64,
    /// Where along the path the objects start.
    pub offset: f64,
    /// How far before the end of an open path they stop.
    pub end_offset: f64,
    pub align: FitAlign,
    pub place: FitPlace,
    /// The distance from the path with [`FitPlace::Above`] and
    /// [`FitPlace::Below`].
    pub side: f64,
    pub rotation: FitRotation,
    /// Degrees counter-clockwise, added to the path's direction with
    /// [`FitRotation::Follow`].
    pub angle: f64,
    pub order: FitOrder,
    pub copies: FitCopies,
    /// The number of objects with [`FitCopies::Count`].
    pub count: u32,
    /// Start from the path's last node.
    pub reverse: bool,
}

impl Default for FitPathSettings {
    fn default() -> Self {
        FitPathSettings {
            spacing: FitSpacing::Even,
            measure: FitMeasure::Centres,
            distance: 10.0,
            offset: 0.0,
            end_offset: 0.0,
            align: FitAlign::Start,
            place: FitPlace::On,
            side: 0.0,
            rotation: FitRotation::Fixed,
            angle: 0.0,
            order: FitOrder::AsSelected,
            copies: FitCopies::None,
            count: 10,
            reverse: false,
        }
    }
}

/// One object placed by a fit: the `source` object moves by `transform`
/// or, with `copy`, a copy of it is made there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub source: ShapeId,
    pub transform: Affine,
    pub copy: bool,
}

/// The first subpath of a path, flattened, with the distance walked to
/// each point.
struct Walk {
    pts: Vec<Point>,
    cum: Vec<f64>,
    total: f64,
    closed: bool,
}

impl Walk {
    fn new(path: &BezPath, reverse: bool) -> Option<Walk> {
        let mut pts: Vec<Point> = Vec::new();
        let mut closed = false;
        let mut done = false;
        kurbo::flatten(path.elements().iter().copied(), 0.05, |el| match el {
            _ if done => {}
            kurbo::PathEl::MoveTo(p) => {
                if pts.is_empty() {
                    pts.push(p);
                } else {
                    done = true;
                }
            }
            kurbo::PathEl::LineTo(p) => pts.push(p),
            kurbo::PathEl::ClosePath => {
                if let Some(&first) = pts.first() {
                    pts.push(first);
                }
                closed = true;
                done = true;
            }
            _ => {}
        });
        pts.dedup_by(|a, b| (*a - *b).hypot() < 1e-9);
        if pts.len() < 2 {
            return None;
        }
        if reverse {
            pts.reverse();
        }
        let mut cum = vec![0.0];
        for w in pts.windows(2) {
            cum.push(cum.last().copied().unwrap_or(0.0) + (w[1] - w[0]).hypot());
        }
        let total = cum.last().copied().unwrap_or(0.0);
        (total > 1e-9).then_some(Walk {
            pts,
            cum,
            total,
            closed,
        })
    }

    /// A distance along the path brought onto it: round a closed path,
    /// held at the ends of an open one.
    fn wrap(&self, d: f64) -> f64 {
        if self.closed {
            d.rem_euclid(self.total)
        } else {
            d.clamp(0.0, self.total)
        }
    }

    /// The point `d` along the path and the unit direction there.
    fn at(&self, d: f64) -> (Point, Vec2) {
        let d = self.wrap(d);
        let last = self.pts.len() - 2;
        let k = self
            .cum
            .partition_point(|c| *c <= d)
            .saturating_sub(1)
            .min(last);
        let seg = self.pts[k + 1] - self.pts[k];
        let len = seg.hypot().max(1e-9);
        let u = ((d - self.cum[k]) / len).clamp(0.0, 1.0);
        (self.pts[k] + seg * u, seg / len)
    }
}

/// The objects in the order they go along the path.
fn ordered(objects: &[Shape], order: FitOrder) -> Vec<&Shape> {
    let mut v: Vec<&Shape> = objects.iter().collect();
    let area = |s: &Shape| s.bounds().area();
    let x = |s: &Shape| s.bounds().center().x;
    match order {
        FitOrder::AsSelected => {}
        FitOrder::Reversed => v.reverse(),
        FitOrder::LeftToRight => v.sort_by(|a, b| x(a).total_cmp(&x(b))),
        FitOrder::RightToLeft => v.sort_by(|a, b| x(b).total_cmp(&x(a))),
        FitOrder::LargestFirst => v.sort_by(|a, b| area(b).total_cmp(&area(a))),
        FitOrder::SmallestFirst => v.sort_by(|a, b| area(a).total_cmp(&area(b))),
        FitOrder::Alternate => {
            v.sort_by(|a, b| area(b).total_cmp(&area(a)));
            let mut out = Vec::with_capacity(v.len());
            let (mut lo, mut hi) = (0, v.len());
            while lo < hi {
                out.push(v[lo]);
                lo += 1;
                if lo < hi {
                    hi -= 1;
                    out.push(v[hi]);
                }
            }
            v = out;
        }
    }
    v
}

fn finite(v: f64, or: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        or
    }
}

/// Where the objects (and copies) of a Fit Objects to Path go: the last
/// shape is the path, the others are placed on its first subpath. Empty
/// when there is nothing to fit.
pub fn fit_to_path_plan(shapes: &[Shape], set: &FitPathSettings) -> Vec<Placement> {
    let Some((path_shape, objects)) = shapes.split_last() else {
        return Vec::new();
    };
    if objects.is_empty() {
        return Vec::new();
    }
    let Some(walk) = Walk::new(&path_shape.page_path(), set.reverse) else {
        return Vec::new();
    };
    let objs = ordered(objects, set.order);
    let n = objs.len();
    let angle = finite(set.angle, 0.0).to_radians();
    let distance = finite(set.distance, 0.0).max(0.0);
    let side = finite(set.side, 0.0);
    let s0 = finite(set.offset, 0.0);
    let s1 = if walk.closed {
        s0 + walk.total
    } else {
        walk.total - finite(set.end_offset, 0.0)
    };
    // Half the object's length along the path and across it, turned as
    // it will be where the path's direction is `phi`.
    let half = |i: usize, phi: f64| -> (f64, f64) {
        let r = objs[i % n].bounds();
        let a = match set.rotation {
            FitRotation::Follow => angle,
            FitRotation::Fixed => angle - phi,
        };
        let (w, h) = (r.width() / 2.0, r.height() / 2.0);
        let (s, c) = (a.sin().abs(), a.cos().abs());
        (w * c + h * s, w * s + h * c)
    };
    let edges = set.measure == FitMeasure::Edges;
    // How many objects go on the path, given their half lengths.
    let how_many = |al: &dyn Fn(usize) -> f64| -> usize {
        let count = match (set.copies, set.spacing) {
            (FitCopies::None, _) | (FitCopies::Fill, FitSpacing::Even) => n,
            (FitCopies::Count, _) => set.count as usize,
            (FitCopies::Fill, FitSpacing::Distance) => {
                let room = if walk.closed { walk.total } else { s1 - s0 };
                if !edges {
                    if distance < 1e-6 {
                        MAX_PLACED
                    } else if walk.closed {
                        (room / distance + 1e-9).floor() as usize
                    } else {
                        ((room / distance + 1e-9).floor() as usize).saturating_add(1)
                    }
                } else {
                    // A closed path also keeps the gap between the last
                    // object and the first.
                    let tail = if walk.closed { distance } else { 0.0 };
                    let mut used = 2.0 * al(0);
                    let mut k = 1;
                    while k < MAX_PLACED && used + distance + 2.0 * al(k) + tail <= room + 1e-9 {
                        used += distance + 2.0 * al(k);
                        k += 1;
                    }
                    k
                }
            }
        };
        count.clamp(n, MAX_PLACED.max(n))
    };
    // The distance along the path of each object's centre.
    let layout = |al: &[f64]| -> Vec<f64> {
        let m = al.len();
        let mut d = Vec::with_capacity(m);
        match set.spacing {
            FitSpacing::Even if !edges => {
                for i in 0..m {
                    d.push(if walk.closed {
                        s0 + walk.total * i as f64 / m as f64
                    } else if m == 1 {
                        (s0 + s1) / 2.0
                    } else {
                        s0 + (s1 - s0) * i as f64 / (m as f64 - 1.0)
                    });
                }
            }
            FitSpacing::Even => {
                let widths: f64 = al.iter().map(|a| 2.0 * a).sum();
                if !walk.closed && m == 1 {
                    d.push((s0 + s1) / 2.0);
                } else {
                    let (gap, first) = if walk.closed {
                        ((walk.total - widths) / m as f64, s0)
                    } else {
                        ((s1 - s0 - widths) / (m as f64 - 1.0), s0 + al[0])
                    };
                    d.push(first);
                    for i in 1..m {
                        let prev = d[i - 1];
                        d.push(prev + al[i - 1] + gap + al[i]);
                    }
                }
            }
            FitSpacing::Distance => {
                // The run from 0, then moved to its place.
                let mut run = Vec::with_capacity(m);
                for i in 0..m {
                    run.push(match (i, edges) {
                        (0, false) => 0.0,
                        (0, true) => al[0],
                        (_, false) => distance * i as f64,
                        (_, true) => run[i - 1] + al[i - 1] + distance + al[i],
                    });
                }
                let lo = 0.0;
                let hi = match run.last() {
                    Some(l) if edges => l + al[m - 1],
                    Some(l) => *l,
                    None => 0.0,
                };
                let base = match set.align {
                    FitAlign::Start => s0 - lo,
                    FitAlign::Middle => (s0 + s1) / 2.0 - (lo + hi) / 2.0,
                    FitAlign::End => s1 - hi,
                };
                d.extend(run.into_iter().map(|r| base + r));
            }
        }
        d
    };
    // The objects' lengths along the path depend on its direction where
    // they land (with a fixed angle), so lay them out, read the
    // directions there and lay them out again.
    let mut phis: Vec<f64> = Vec::new();
    let mut dist: Vec<f64> = Vec::new();
    let passes = if set.rotation == FitRotation::Fixed && (edges || set.place != FitPlace::On) {
        3
    } else {
        1
    };
    for _ in 0..passes {
        let phi_of = |i: usize| phis.get(i).copied().unwrap_or(0.0);
        let al_of = |i: usize| half(i, phi_of(i)).0;
        let m = how_many(&al_of);
        let al: Vec<f64> = (0..m).map(al_of).collect();
        dist = layout(&al);
        phis = dist
            .iter()
            .map(|d| {
                let (_, t) = walk.at(*d);
                t.y.atan2(t.x)
            })
            .collect();
    }
    dist.iter()
        .zip(&phis)
        .enumerate()
        .map(|(i, (d, phi))| {
            let s = objs[i % n];
            let (pos, t) = walk.at(*d);
            let across = half(i, *phi).1;
            let shift = match set.place {
                FitPlace::On => 0.0,
                FitPlace::Above => across + side,
                FitPlace::Below => -(across + side),
            };
            let normal = Vec2::new(-t.y, t.x);
            let turn = match set.rotation {
                FitRotation::Follow => phi + angle,
                FitRotation::Fixed => angle,
            };
            let c = s.bounds().center();
            Placement {
                source: s.id,
                transform: Affine::translate((pos + normal * shift).to_vec2())
                    * Affine::rotate(turn)
                    * Affine::translate(-c.to_vec2()),
                copy: i >= n,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::ShapeKind;

    fn rect(id: u64, x: f64, y: f64, w: f64, h: f64) -> Shape {
        Shape::new(
            ShapeId(id),
            ShapeKind::Rect {
                rect: Rect::new(x, y, x + w, y + h),
                radius: 0.0,
                corners: None,
            },
        )
    }

    fn line(id: u64, pts: &[(f64, f64)], closed: bool) -> Shape {
        let mut p = BezPath::new();
        for (i, q) in pts.iter().enumerate() {
            if i == 0 {
                p.move_to(*q);
            } else {
                p.line_to(*q);
            }
        }
        if closed {
            p.close_path();
        }
        Shape::new(ShapeId(id), ShapeKind::Path { path: p, closed })
    }

    fn centres(shapes: &[Shape], plan: &[Placement]) -> Vec<(u64, Point, bool)> {
        plan.iter()
            .map(|p| {
                let s = shapes.iter().find(|s| s.id == p.source).expect("source");
                (p.source.0, p.transform * s.bounds().center(), p.copy)
            })
            .collect()
    }

    fn near(a: Point, b: (f64, f64)) -> bool {
        (a - Point::from(b)).hypot() < 1e-6
    }

    /// A count spreads copies of the objects, in order, over the path;
    /// Fill puts as many as fit at the distance, and the copies cycle
    /// through the objects (#1091, #1196).
    #[test]
    fn copies_fill_the_path_in_order() {
        let shapes = vec![
            rect(1, 200.0, 0.0, 4.0, 2.0),
            rect(2, 300.0, 0.0, 4.0, 2.0),
            line(9, &[(0.0, 0.0), (100.0, 0.0)], false),
        ];
        let set = FitPathSettings {
            copies: FitCopies::Count,
            count: 5,
            ..FitPathSettings::default()
        };
        let got = centres(&shapes, &fit_to_path_plan(&shapes, &set));
        let want = [
            (1, 0.0, false),
            (2, 25.0, false),
            (1, 50.0, true),
            (2, 75.0, true),
            (1, 100.0, true),
        ];
        assert_eq!(got.len(), 5);
        for ((id, p, copy), (wid, x, wcopy)) in got.iter().zip(want) {
            assert_eq!((*id, *copy), (wid, wcopy));
            assert!(near(*p, (x, 0.0)), "{p:?} {x}");
        }
        // A count below the number of objects still places them all.
        let set = FitPathSettings { count: 1, ..set };
        assert_eq!(fit_to_path_plan(&shapes, &set).len(), 2);
        // Fill at 30 mm: 0, 30, 60, 90.
        let set = FitPathSettings {
            spacing: FitSpacing::Distance,
            distance: 30.0,
            copies: FitCopies::Fill,
            ..FitPathSettings::default()
        };
        let got = centres(&shapes, &fit_to_path_plan(&shapes, &set));
        let xs: Vec<f64> = got.iter().map(|g| g.1.x).collect();
        assert_eq!(xs.len(), 4, "{xs:?}");
        for (x, w) in xs.iter().zip([0.0, 30.0, 60.0, 90.0]) {
            assert!((x - w).abs() < 1e-6, "{xs:?}");
        }
        // Between edges: 4 mm wide objects with 6 mm gaps from 0 to 100
        // make 10, their centres 2, 12, ... 92.
        let set = FitPathSettings {
            measure: FitMeasure::Edges,
            distance: 6.0,
            ..set
        };
        let got = centres(&shapes, &fit_to_path_plan(&shapes, &set));
        assert_eq!(got.len(), 10);
        assert!(
            near(got[0].1, (2.0, 0.0)) && near(got[9].1, (92.0, 0.0)),
            "{got:?}"
        );
        // Centred on the path: the run of 10 is 94 mm long, from 3 to 97.
        let set = FitPathSettings {
            align: FitAlign::Middle,
            ..set
        };
        let got = centres(&shapes, &fit_to_path_plan(&shapes, &set));
        assert!(
            near(got[0].1, (5.0, 0.0)) && near(got[9].1, (95.0, 0.0)),
            "{got:?}"
        );
        // At the end, after an end offset of 10 mm: the last edge at 90.
        let set = FitPathSettings {
            align: FitAlign::End,
            end_offset: 10.0,
            ..set
        };
        let got = centres(&shapes, &fit_to_path_plan(&shapes, &set));
        assert_eq!(got.len(), 9);
        assert!(near(got[8].1, (88.0, 0.0)), "{got:?}");
        // A zero distance cannot fill forever.
        let set = FitPathSettings {
            measure: FitMeasure::Centres,
            distance: 0.0,
            ..set
        };
        assert_eq!(fit_to_path_plan(&shapes, &set).len(), MAX_PLACED);
    }

    /// Even spacing between edges leaves the same gap between objects of
    /// different sizes, between the start and end offsets.
    #[test]
    fn even_gaps_between_edges() {
        let shapes = vec![
            rect(1, 200.0, 0.0, 10.0, 2.0),
            rect(2, 300.0, 0.0, 20.0, 2.0),
            rect(3, 400.0, 0.0, 10.0, 2.0),
            line(9, &[(0.0, 0.0), (100.0, 0.0)], false),
        ];
        let set = FitPathSettings {
            measure: FitMeasure::Edges,
            offset: 10.0,
            end_offset: 10.0,
            ..FitPathSettings::default()
        };
        let got = centres(&shapes, &fit_to_path_plan(&shapes, &set));
        // 80 mm of room, 40 mm of objects: two gaps of 20 mm.
        for (g, x) in got.iter().zip([15.0, 50.0, 85.0]) {
            assert!(near(g.1, (x, 0.0)), "{got:?}");
        }
    }

    /// Above and below put the near edge the set distance from the path,
    /// on the left and right of its direction; a fixed angle turns every
    /// object alike; the orders sort by place and by size (#1091, #1196).
    #[test]
    fn placement_rotation_and_order() {
        let shapes = vec![
            rect(1, 300.0, 0.0, 4.0, 2.0),   // area 8, right
            rect(2, 100.0, 0.0, 6.0, 6.0),   // area 36, left
            rect(3, 200.0, 0.0, 2.0, 2.0),   // area 4, middle
            rect(4, 400.0, 0.0, 10.0, 10.0), // area 100, far right
            line(9, &[(0.0, 0.0), (90.0, 0.0)], false),
        ];
        let order = |o: FitOrder| -> Vec<u64> {
            let set = FitPathSettings {
                order: o,
                ..FitPathSettings::default()
            };
            fit_to_path_plan(&shapes, &set)
                .iter()
                .map(|p| p.source.0)
                .collect()
        };
        assert_eq!(order(FitOrder::AsSelected), [1, 2, 3, 4]);
        assert_eq!(order(FitOrder::Reversed), [4, 3, 2, 1]);
        assert_eq!(order(FitOrder::LeftToRight), [2, 3, 1, 4]);
        assert_eq!(order(FitOrder::RightToLeft), [4, 1, 3, 2]);
        assert_eq!(order(FitOrder::LargestFirst), [4, 2, 1, 3]);
        assert_eq!(order(FitOrder::SmallestFirst), [3, 1, 2, 4]);
        assert_eq!(order(FitOrder::Alternate), [4, 3, 2, 1]);

        let one = vec![
            rect(1, 300.0, 0.0, 4.0, 2.0),
            line(9, &[(0.0, 0.0), (0.0, 100.0)], false),
        ];
        let at = |set: FitPathSettings| -> kurbo::Rect {
            let p = fit_to_path_plan(&one, &set)[0];
            kurbo::Shape::bounding_box(&(p.transform * one[0].page_path()))
        };
        // Up a vertical path, following it: the 4 x 2 object stands up,
        // the left of the path is -x; 3 mm away the near edge is at -3.
        let set = FitPathSettings {
            rotation: FitRotation::Follow,
            place: FitPlace::Above,
            side: 3.0,
            ..FitPathSettings::default()
        };
        let r = at(set);
        assert!(
            (r.x1 + 3.0).abs() < 1e-6
                && (r.width() - 2.0).abs() < 1e-6
                && (r.center().y - 50.0).abs() < 1e-6,
            "{r:?}"
        );
        let r = at(FitPathSettings {
            place: FitPlace::Below,
            ..set
        });
        assert!((r.x0 - 3.0).abs() < 1e-6, "{r:?}");
        // Fixed at 0: the object stays as it is, its near edge 3 mm off
        // (its half width across a vertical path is 2).
        let r = at(FitPathSettings {
            rotation: FitRotation::Fixed,
            ..set
        });
        assert!(
            (r.x1 + 3.0).abs() < 1e-6 && (r.width() - 4.0).abs() < 1e-6,
            "{r:?}"
        );
        // Fixed at 90 degrees: turned upright on any path.
        let r = at(FitPathSettings {
            rotation: FitRotation::Fixed,
            place: FitPlace::On,
            angle: 90.0,
            ..set
        });
        assert!(
            (r.width() - 2.0).abs() < 1e-6 && (r.height() - 4.0).abs() < 1e-6,
            "{r:?}"
        );
        // Reversed: from the top down.
        let p = fit_to_path_plan(
            &one,
            &FitPathSettings {
                spacing: FitSpacing::Distance,
                offset: 10.0,
                reverse: true,
                ..FitPathSettings::default()
            },
        )[0];
        assert!(near(p.transform * one[0].bounds().center(), (0.0, 90.0)));
    }

    /// A closed path's copies go round evenly, the gap from the last back
    /// to the first included; bad numbers place nothing wrong.
    #[test]
    fn closed_paths_and_bad_numbers() {
        let shapes = vec![
            rect(1, 200.0, 0.0, 4.0, 2.0),
            line(
                9,
                &[(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)],
                true,
            ),
        ];
        let set = FitPathSettings {
            spacing: FitSpacing::Distance,
            distance: 20.0,
            copies: FitCopies::Fill,
            ..FitPathSettings::default()
        };
        assert_eq!(fit_to_path_plan(&shapes, &set).len(), 8);
        let set = FitPathSettings {
            measure: FitMeasure::Edges,
            distance: 16.0,
            ..set
        };
        // 4 mm objects and 16 mm gaps: 160 / 20.
        assert_eq!(fit_to_path_plan(&shapes, &set).len(), 8);
        let set = FitPathSettings {
            distance: f64::NAN,
            offset: f64::INFINITY,
            angle: f64::NAN,
            side: f64::NAN,
            end_offset: f64::NAN,
            ..set
        };
        for p in fit_to_path_plan(&shapes, &set) {
            let c = p.transform * Point::ZERO;
            assert!(c.x.is_finite() && c.y.is_finite());
        }
        assert!(fit_to_path_plan(&shapes[1..], &set).is_empty());
        assert!(fit_to_path_plan(&[], &set).is_empty());
    }
}
