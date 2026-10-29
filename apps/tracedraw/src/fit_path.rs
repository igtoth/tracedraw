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

/// Which point of each object goes on the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitReference {
    /// The object's centre of rotation: the centre of its box, or where
    /// it was moved to with the Pick tool.
    CenterOfRotation,
    /// One of nine points of the object's bounding box (the Origin).
    BoundingBox,
    /// One of nine points of the box with half the outline's width
    /// around it.
    OutlineBox,
}

/// Turns added to the objects one after another (Modify rotation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitSpin {
    /// No turn added.
    None,
    /// Every object turned by the start angle.
    Uniform,
    /// From the start angle, each object a step further, the whole run
    /// turning by the spin angle times the revolutions.
    Progressive,
    /// The start angle plus a turn of up to the range either way, the
    /// same for the same object every time.
    Jitter,
    /// Progressive, with the jitter added.
    ProgressiveJitter,
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
    /// Leave the selected objects where they are and place copies.
    pub keep_originals: bool,
    /// Group the objects placed.
    pub group: bool,
    /// Walk every subpath of the path, one after another, as one path;
    /// else only the first subpath.
    pub contiguous: bool,
    pub reference: FitReference,
    /// The point of the box with [`FitReference::BoundingBox`] and
    /// [`FitReference::OutlineBox`]: 0 to 8, left to right from the top
    /// row; 4 is the centre.
    pub origin: u8,
    /// Take the turn the objects have off them before they are placed.
    pub ignore_rotation: bool,
    pub spin: FitSpin,
    /// Degrees, for every style but [`FitSpin::None`].
    pub start_angle: f64,
    /// Degrees a revolution turns ([`FitSpin::Progressive`]).
    pub spin_angle: f64,
    pub revolutions: f64,
    /// Degrees either way ([`FitSpin::Jitter`]).
    pub range: f64,
    /// The turns added go clockwise.
    pub clockwise: bool,
}

impl FitPathSettings {
    /// Which Modify rotation fields the style uses: start angle, spin
    /// angle, revolutions, range and direction.
    pub fn spin_fields(&self) -> [bool; 5] {
        match self.spin {
            FitSpin::None => [false; 5],
            FitSpin::Uniform => [true, false, false, false, true],
            FitSpin::Progressive => [true, true, true, false, true],
            FitSpin::Jitter => [true, false, false, true, true],
            FitSpin::ProgressiveJitter => [true; 5],
        }
    }
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
            keep_originals: false,
            group: false,
            contiguous: true,
            reference: FitReference::CenterOfRotation,
            origin: 4,
            ignore_rotation: false,
            spin: FitSpin::None,
            start_angle: 0.0,
            spin_angle: 360.0,
            revolutions: 1.0,
            range: 15.0,
            clockwise: false,
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

/// A path flattened, with the distance walked to each point: its first
/// subpath, or all of them one after another. The step from one subpath
/// to the next is not walked (`jump`).
struct Walk {
    pts: Vec<Point>,
    cum: Vec<f64>,
    /// Per segment: the move to the next subpath.
    jump: Vec<bool>,
    total: f64,
    closed: bool,
}

impl Walk {
    fn new(path: &BezPath, reverse: bool, all: bool) -> Option<Walk> {
        // Each subpath's points and whether it is closed.
        let mut subs: Vec<(Vec<Point>, bool)> = Vec::new();
        let mut done = false;
        kurbo::flatten(path.elements().iter().copied(), 0.05, |el| match el {
            _ if done => {}
            kurbo::PathEl::MoveTo(p) => {
                if !all && !subs.is_empty() {
                    done = true;
                } else {
                    subs.push((vec![p], false));
                }
            }
            kurbo::PathEl::LineTo(p) => {
                if let Some(sub) = subs.last_mut() {
                    sub.0.push(p);
                }
            }
            kurbo::PathEl::ClosePath => {
                if let Some(sub) = subs.last_mut() {
                    if let Some(&first) = sub.0.first() {
                        sub.0.push(first);
                    }
                    sub.1 = true;
                }
                if !all {
                    done = true;
                }
            }
            _ => {}
        });
        for sub in &mut subs {
            sub.0.dedup_by(|a, b| (*a - *b).hypot() < 1e-9);
        }
        subs.retain(|sub| sub.0.len() >= 2);
        if subs.is_empty() {
            return None;
        }
        let closed = subs.iter().all(|sub| sub.1);
        if reverse {
            subs.reverse();
            for sub in &mut subs {
                sub.0.reverse();
            }
        }
        let mut pts = Vec::new();
        let mut jump = Vec::new();
        for (sub, _) in subs {
            if !pts.is_empty() {
                jump.push(true);
            }
            for (k, p) in sub.into_iter().enumerate() {
                if k > 0 {
                    jump.push(false);
                }
                pts.push(p);
            }
        }
        let mut cum = vec![0.0];
        for (k, w) in pts.windows(2).enumerate() {
            let step = if jump[k] { 0.0 } else { (w[1] - w[0]).hypot() };
            cum.push(cum.last().copied().unwrap_or(0.0) + step);
        }
        let total = cum.last().copied().unwrap_or(0.0);
        (total > 1e-9).then_some(Walk {
            pts,
            cum,
            jump,
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
        let mut k = self
            .cum
            .partition_point(|c| *c <= d)
            .saturating_sub(1)
            .min(last);
        // On a move to the next subpath: the end of the one before.
        while k > 0 && self.jump.get(k).copied().unwrap_or(false) {
            k -= 1;
        }
        let seg = self.pts[k + 1] - self.pts[k];
        let len = seg.hypot().max(1e-9);
        let u = ((d - self.cum[k]) / len).clamp(0.0, 1.0);
        (self.pts[k] + seg * u, seg / len)
    }
}

/// An object made ready to place: what takes its initial turn off (with
/// Ignore initial rotation), its box after that, and the point of it that
/// goes on the path, as an offset from the box's centre.
struct Prepared<'a> {
    shape: &'a Shape,
    base: Affine,
    centre: Point,
    half_w: f64,
    half_h: f64,
    offset: Vec2,
}

fn prepare<'a>(
    s: &'a Shape,
    set: &FitPathSettings,
    pivot: &dyn Fn(&Shape) -> Option<Point>,
) -> Prepared<'a> {
    let bounds = s.bounds();
    let c = s.transform.as_coeffs();
    let turn = c[1].atan2(c[0]);
    let base = if set.ignore_rotation && turn.is_finite() && turn.abs() > 1e-9 {
        Affine::rotate_about(-turn, bounds.center())
    } else {
        Affine::IDENTITY
    };
    let mut r = if base == Affine::IDENTITY {
        bounds
    } else {
        kurbo::Shape::bounding_box(&(base * s.page_path()))
    };
    if set.reference == FitReference::OutlineBox {
        if let Some(st) = &s.stroke {
            let w = finite(st.width, 0.0).max(0.0) / 2.0;
            r = r.inflate(w, w);
        }
    }
    let centre = r.center();
    let point = match set.reference {
        FitReference::CenterOfRotation => pivot(s)
            .filter(|p| p.x.is_finite() && p.y.is_finite())
            .map(|p| base * p)
            .unwrap_or(centre),
        FitReference::BoundingBox | FitReference::OutlineBox => {
            let o = set.origin.min(8);
            let u = (o % 3) as f64 / 2.0;
            // The top row first; the page's y axis points up.
            let v = 1.0 - (o / 3) as f64 / 2.0;
            Point::new(r.x0 + u * r.width(), r.y0 + v * r.height())
        }
    };
    Prepared {
        shape: s,
        base,
        centre,
        half_w: r.width() / 2.0,
        half_h: r.height() / 2.0,
        offset: point - centre,
    }
}

/// A number from -1 to 1 that is the same for the same `i` every time.
fn jitter(i: usize) -> f64 {
    let mut z = (i as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
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
/// shape is the path, the others are placed on it. Empty when there is
/// nothing to fit. Centres of rotation are the boxes' centres.
#[cfg(test)]
pub fn fit_to_path_plan(shapes: &[Shape], set: &FitPathSettings) -> Vec<Placement> {
    fit_to_path_plan_with(shapes, set, &|_| None)
}

/// [`fit_to_path_plan`] with the centres of rotation moved by the user
/// (`pivot`, in page space).
pub fn fit_to_path_plan_with(
    shapes: &[Shape],
    set: &FitPathSettings,
    pivot: &dyn Fn(&Shape) -> Option<Point>,
) -> Vec<Placement> {
    let Some((path_shape, objects)) = shapes.split_last() else {
        return Vec::new();
    };
    if objects.is_empty() {
        return Vec::new();
    }
    let Some(walk) = Walk::new(&path_shape.page_path(), set.reverse, set.contiguous) else {
        return Vec::new();
    };
    let objs: Vec<Prepared> = ordered(objects, set.order)
        .into_iter()
        .map(|s| prepare(s, set, pivot))
        .collect();
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
    let start = finite(set.start_angle, 0.0).to_radians();
    let spin_total = (finite(set.spin_angle, 0.0) * finite(set.revolutions, 0.0)).to_radians();
    let range = finite(set.range, 0.0).to_radians();
    let sign = if set.clockwise { -1.0 } else { 1.0 };
    // The turn Modify rotation adds to object `i` of `m`.
    let extra = |i: usize, m: usize| -> f64 {
        let step = || {
            let parts = if walk.closed { m } else { m.saturating_sub(1) };
            if parts == 0 {
                0.0
            } else {
                spin_total * i as f64 / parts as f64
            }
        };
        sign * match set.spin {
            FitSpin::None => 0.0,
            FitSpin::Uniform => start,
            FitSpin::Progressive => start + step(),
            FitSpin::Jitter => start + range * jitter(i),
            FitSpin::ProgressiveJitter => start + step() + range * jitter(i),
        }
    };
    // Object `i` turned as it will be where the path's direction is `phi`,
    // in the path's frame: its length along the path before and after its
    // reference point, its half width across the path and where its
    // reference point is from its centre.
    let frame = |i: usize, m: usize, phi: f64| -> (f64, f64, f64, Vec2) {
        let o = &objs[i % n];
        let a = extra(i, m)
            + match set.rotation {
                FitRotation::Follow => angle,
                FitRotation::Fixed => angle - phi,
            };
        let (sa, ca) = a.sin_cos();
        let (s, c) = (sa.abs(), ca.abs());
        let along = o.half_w * c + o.half_h * s;
        let across = o.half_w * s + o.half_h * c;
        let v = Vec2::new(
            o.offset.x * ca - o.offset.y * sa,
            o.offset.x * sa + o.offset.y * ca,
        );
        (along + v.x, along - v.x, across, v)
    };
    let edges = set.measure == FitMeasure::Edges;
    // How many objects go on the path, given their lengths before and
    // after their reference points.
    let how_many = |ext: &dyn Fn(usize) -> (f64, f64)| -> usize {
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
                    let width = |k: usize| {
                        let (b, a) = ext(k);
                        b + a
                    };
                    let mut used = width(0);
                    let mut k = 1;
                    while k < MAX_PLACED && used + distance + width(k) + tail <= room + 1e-9 {
                        used += distance + width(k);
                        k += 1;
                    }
                    k
                }
            }
        };
        count.clamp(n, MAX_PLACED.max(n))
    };
    // The distance along the path of each object's reference point.
    let layout = |ext: &[(f64, f64)]| -> Vec<f64> {
        let m = ext.len();
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
                let widths: f64 = ext.iter().map(|(b, a)| b + a).sum();
                if !walk.closed && m == 1 {
                    d.push((s0 + s1) / 2.0);
                } else {
                    let (gap, first) = if walk.closed {
                        ((walk.total - widths) / m as f64, s0)
                    } else {
                        ((s1 - s0 - widths) / (m as f64 - 1.0), s0 + ext[0].0)
                    };
                    d.push(first);
                    for i in 1..m {
                        let prev = d[i - 1];
                        d.push(prev + ext[i - 1].1 + gap + ext[i].0);
                    }
                }
            }
            FitSpacing::Distance => {
                // The run from 0, then moved to its place.
                let mut run: Vec<f64> = Vec::with_capacity(m);
                for i in 0..m {
                    run.push(match (i, edges) {
                        (0, false) => 0.0,
                        (0, true) => ext[0].0,
                        (_, false) => distance * i as f64,
                        (_, true) => run[i - 1] + ext[i - 1].1 + distance + ext[i].0,
                    });
                }
                let lo = 0.0;
                let hi = match run.last() {
                    Some(l) if edges => l + ext[m - 1].1,
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
    let mut m = n;
    let passes = if set.rotation == FitRotation::Fixed
        && (edges || set.place != FitPlace::On || objs.iter().any(|o| o.offset.hypot() > 1e-9))
    {
        3
    } else {
        1
    };
    for _ in 0..passes {
        let phi_of = |i: usize| phis.get(i).copied().unwrap_or(0.0);
        // The count first, with the progressive turns of the last count.
        let guess = m;
        let ext_of = |i: usize| {
            let (b, a, _, _) = frame(i, guess, phi_of(i));
            (b, a)
        };
        m = how_many(&ext_of);
        let ext: Vec<(f64, f64)> = (0..m)
            .map(|i| {
                let (b, a, _, _) = frame(i, m, phi_of(i));
                (b, a)
            })
            .collect();
        dist = layout(&ext);
        phis = dist
            .iter()
            .map(|d| {
                let (_, t) = walk.at(*d);
                t.y.atan2(t.x)
            })
            .collect();
    }
    let m = dist.len();
    dist.iter()
        .zip(&phis)
        .enumerate()
        .map(|(i, (d, phi))| {
            let o = &objs[i % n];
            let (pos, t) = walk.at(*d);
            let (_, _, across, v) = frame(i, m, *phi);
            // Above and below keep the reference point's place along the
            // path and move the object off it by its edge.
            let shift = match set.place {
                FitPlace::On => 0.0,
                FitPlace::Above => across + side + v.y,
                FitPlace::Below => -(across + side) + v.y,
            };
            let normal = Vec2::new(-t.y, t.x);
            let turn = extra(i, m)
                + match set.rotation {
                    FitRotation::Follow => phi + angle,
                    FitRotation::Fixed => angle,
                };
            let reference = o.centre + o.offset;
            Placement {
                source: o.shape.id,
                transform: Affine::translate((pos + normal * shift).to_vec2())
                    * Affine::rotate(turn)
                    * Affine::translate(-reference.to_vec2())
                    * o.base,
                copy: set.keep_originals || i >= n,
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

    fn turn_of(p: &Placement) -> f64 {
        let c = p.transform.as_coeffs();
        c[1].atan2(c[0]).to_degrees()
    }

    /// Keep originals places copies only; with Treat subpaths as
    /// contiguous every subpath is walked one after another (the move
    /// between them not counted), else only the first (#1091).
    #[test]
    fn originals_and_subpaths() {
        let mut two = BezPath::new();
        two.move_to((0.0, 0.0));
        two.line_to((40.0, 0.0));
        two.move_to((0.0, 10.0));
        two.line_to((60.0, 10.0));
        let shapes = vec![
            rect(1, 200.0, 0.0, 4.0, 2.0),
            rect(2, 300.0, 0.0, 4.0, 2.0),
            rect(3, 400.0, 0.0, 4.0, 2.0),
            Shape::new(
                ShapeId(9),
                ShapeKind::Path {
                    path: two,
                    closed: false,
                },
            ),
        ];
        let set = FitPathSettings::default();
        let got = centres(&shapes, &fit_to_path_plan(&shapes, &set));
        for (g, p) in got.iter().zip([(0.0, 0.0), (10.0, 10.0), (60.0, 10.0)]) {
            assert!(near(g.1, p) && !g.2, "{got:?}");
        }
        let set = FitPathSettings {
            contiguous: false,
            keep_originals: true,
            ..set
        };
        let got = centres(&shapes, &fit_to_path_plan(&shapes, &set));
        for (g, p) in got.iter().zip([(0.0, 0.0), (20.0, 0.0), (40.0, 0.0)]) {
            assert!(near(g.1, p) && g.2, "{got:?}");
        }
        // Reversed: from the end of the last subpath back.
        let set = FitPathSettings {
            reverse: true,
            ..FitPathSettings::default()
        };
        let got = centres(&shapes, &fit_to_path_plan(&shapes, &set));
        for (g, p) in got.iter().zip([(60.0, 10.0), (10.0, 10.0), (0.0, 0.0)]) {
            assert!(near(g.1, p), "{got:?}");
        }
    }

    /// The reference point goes on the path: the centre of rotation (moved
    /// or not) or one of the nine points of the box, with or without half
    /// the outline (#1091).
    #[test]
    fn the_reference_point_goes_on_the_path() {
        let mut obj = rect(1, 200.0, 0.0, 4.0, 2.0);
        obj.stroke = Some(tracedraw_core::Stroke::new(
            tracedraw_core::Color::BLACK,
            2.0,
        ));
        let shapes = vec![obj, line(9, &[(0.0, 0.0), (100.0, 0.0)], false)];
        let at = |set: FitPathSettings, pivot: Option<Point>| -> Point {
            let plan = fit_to_path_plan_with(&shapes, &set, &|_| pivot);
            plan[0].transform * shapes[0].bounds().center()
        };
        let one = FitPathSettings {
            spacing: FitSpacing::Distance,
            offset: 50.0,
            ..FitPathSettings::default()
        };
        assert!(near(at(one, None), (50.0, 0.0)));
        // A centre of rotation moved to the left edge's middle.
        assert!(near(at(one, Some(Point::new(200.0, 1.0))), (52.0, 0.0)));
        let boxed = FitPathSettings {
            reference: FitReference::BoundingBox,
            ..one
        };
        // Bottom centre (7) on the path: the object sits on it.
        assert!(near(
            at(FitPathSettings { origin: 7, ..boxed }, None),
            (50.0, 1.0)
        ));
        // Top left (0): the object hangs right of and below the point.
        assert!(near(
            at(FitPathSettings { origin: 0, ..boxed }, None),
            (52.0, -1.0)
        ));
        // With half the 2 mm outline around the box.
        let outline = FitPathSettings {
            reference: FitReference::OutlineBox,
            origin: 7,
            ..one
        };
        assert!(near(at(outline, None), (50.0, 2.0)));
        // Above the path the near edge stays the set distance off, the
        // reference point keeping its place along the path.
        let above = FitPathSettings {
            reference: FitReference::BoundingBox,
            origin: 5,
            place: FitPlace::Above,
            side: 1.0,
            ..one
        };
        assert!(near(at(above, None), (48.0, 2.0)), "{:?}", at(above, None));
        // Bad origins are held to the grid.
        assert!(at(
            FitPathSettings {
                origin: 200,
                ..boxed
            },
            None
        )
        .x
        .is_finite());
    }

    /// Ignore initial rotation takes the turn an object has off it; Modify
    /// rotation adds the same turn to all, a growing one, a jitter or
    /// both, either way round (#1091).
    #[test]
    fn rotations_are_reset_and_modified() {
        let mut turned = rect(1, 200.0, 0.0, 4.0, 2.0);
        turned.transform_by(Affine::rotate_about(
            30f64.to_radians(),
            Point::new(202.0, 1.0),
        ));
        let shapes = vec![
            turned,
            rect(2, 300.0, 0.0, 4.0, 2.0),
            rect(3, 400.0, 0.0, 4.0, 2.0),
            line(9, &[(0.0, 0.0), (100.0, 0.0)], false),
        ];
        let plan = |set: FitPathSettings| fit_to_path_plan(&shapes, &set);
        let kept = plan(FitPathSettings::default());
        let reset = plan(FitPathSettings {
            ignore_rotation: true,
            ..FitPathSettings::default()
        });
        let b = |p: &Placement| kurbo::Shape::bounding_box(&(p.transform * shapes[0].page_path()));
        assert!(
            (b(&reset[0]).width() - 4.0).abs() < 1e-6 && (b(&reset[0]).height() - 2.0).abs() < 1e-6
        );
        assert!(b(&kept[0]).width() > 4.0);
        assert!(near(
            reset[0].transform * shapes[0].bounds().center(),
            (0.0, 0.0)
        ));
        let turns = |set: FitPathSettings| -> Vec<f64> { plan(set).iter().map(turn_of).collect() };
        let spin = FitPathSettings {
            start_angle: 10.0,
            spin_angle: 360.0,
            revolutions: 0.5,
            range: 0.0,
            ..FitPathSettings::default()
        };
        let close = |a: &[f64], b: &[f64]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6);
        assert!(close(&turns(spin)[1..], &[0.0, 0.0]), "None adds nothing");
        let uniform = FitPathSettings {
            spin: FitSpin::Uniform,
            ..spin
        };
        assert!(close(&turns(uniform)[1..], &[10.0, 10.0]));
        let prog = FitPathSettings {
            spin: FitSpin::Progressive,
            ..spin
        };
        assert!(
            close(&turns(prog)[1..], &[100.0, -170.0]),
            "{:?}",
            turns(prog)
        );
        let cw = FitPathSettings {
            clockwise: true,
            ..prog
        };
        assert!(close(&turns(cw)[1..], &[-100.0, 170.0]), "{:?}", turns(cw));
        let jit = FitPathSettings {
            spin: FitSpin::Jitter,
            range: 15.0,
            ..spin
        };
        let j = turns(jit);
        assert_eq!(j, turns(jit), "the same every time");
        assert!(
            j[1..].iter().all(|t| (t - 10.0).abs() <= 15.0 + 1e-9),
            "{j:?}"
        );
        assert!((j[1] - j[2]).abs() > 1e-6, "{j:?}");
        let both = FitPathSettings {
            spin: FitSpin::ProgressiveJitter,
            ..jit
        };
        let pj = turns(both);
        assert!((pj[1] - j[1] - 90.0).abs() < 1e-6, "{pj:?} {j:?}");
        // The fields each style uses.
        assert_eq!(spin.spin_fields(), [false; 5]);
        assert_eq!(uniform.spin_fields(), [true, false, false, false, true]);
        assert_eq!(jit.spin_fields(), [true, false, false, true, true]);
        assert_eq!(both.spin_fields(), [true; 5]);
        // Numbers that cannot be used place nothing wrong.
        let bad = FitPathSettings {
            spin: FitSpin::ProgressiveJitter,
            start_angle: f64::NAN,
            spin_angle: f64::INFINITY,
            revolutions: f64::NAN,
            range: f64::NAN,
            ..spin
        };
        for p in plan(bad) {
            assert!(p.transform.as_coeffs().iter().all(|c| c.is_finite()));
        }
    }
}
