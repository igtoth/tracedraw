//! Fill and outline (stroke) properties of a shape.

use crate::color::Color;
use crate::geometry::{Affine, Point, Rect, Size, Vec2};
use serde::{Deserialize, Serialize};

/// Fountain (gradient) fill geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FountainKind {
    #[default]
    Linear,
    Radial,
    Conical,
    Square,
    /// Follows the object's outline: 0 on its edge, 1 at the point
    /// deepest inside it.
    Shaped,
}

impl FountainKind {
    /// Every kind, in the order the fill editor lists them.
    pub const ALL: [FountainKind; 5] = [
        FountainKind::Linear,
        FountainKind::Radial,
        FountainKind::Conical,
        FountainKind::Square,
        FountainKind::Shaped,
    ];
}

/// How a fountain runs its colours over its span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FountainRepeat {
    /// Once; the end colours hold beyond the span.
    #[default]
    None,
    /// Sawtooth: each cycle starts again at the first colour.
    Repeat,
    /// Triangular: cycles run there and back.
    Mirror,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stop {
    /// 0.0 at the start, 1.0 at the end.
    pub pos: f64,
    pub color: Color,
    /// Where the blend to the next stop is half way, as a fraction of the
    /// way there (0.5: in the middle; the Interactive Fill tool's
    /// midpoint slider). Unused on the last stop.
    #[serde(default = "half", skip_serializing_if = "is_half")]
    pub mid: f64,
    /// 1 opaque, 0 clear (the node's transparency).
    #[serde(default = "one_f32", skip_serializing_if = "is_one_f32")]
    pub opacity: f32,
}

impl Stop {
    /// An opaque stop with its blend half way to the next.
    pub fn new(pos: f64, color: Color) -> Self {
        Stop {
            pos,
            color,
            mid: 0.5,
            opacity: 1.0,
        }
    }
}

fn half() -> f64 {
    0.5
}

fn is_half(v: &f64) -> bool {
    (*v - 0.5).abs() < 1e-12
}

fn one_f32() -> f32 {
    1.0
}

fn is_one_f32(v: &f32) -> bool {
    *v == 1.0
}

/// The blend from one stop to the next at `k` (0..1 of the way) with its
/// midpoint at `mid`: `k^g` with `mid^g = 1/2`.
pub fn midpoint_curve(k: f64, mid: f64) -> f64 {
    let m = mid.clamp(0.01, 0.99);
    if (m - 0.5).abs() < 1e-9 {
        return k;
    }
    k.clamp(0.0, 1.0).powf(0.5f64.ln() / m.ln())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fountain {
    pub kind: FountainKind,
    /// At least two stops, sorted by position.
    pub stops: Vec<Stop>,
    /// Degrees (0 = left to right, counter-clockwise positive).
    pub angle: f64,
    /// Centre offset in bounds-relative units (-1..1), for radial/conical/square.
    pub offset: Point,
    /// Edge padding in 0..0.49: fraction of the span held at the end colours.
    pub edge_pad: f64,
    /// Repeat mode over the span.
    #[serde(default, skip_serializing_if = "is_default")]
    pub repeat: FountainRepeat,
    /// Cycles across the span when `repeat` is not None (1 to
    /// [`Fountain::MAX_REPEATS`]).
    #[serde(default = "one_u32", skip_serializing_if = "is_one_u32")]
    pub repeats: u32,
    /// Mirrored about the centre: the colours run first to last and back.
    #[serde(default, skip_serializing_if = "is_default")]
    pub bilinear: bool,
    /// Colours in reverse order.
    #[serde(default, skip_serializing_if = "is_default")]
    pub invert: bool,
    /// Where the Interactive Fill tool put the fill's nodes (#330): the
    /// start, the end and the skew handle, free of the object's box. When
    /// set they place the fill instead of `angle` and `offset` (which
    /// keep the nearest equivalent for formats that know only those).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handles: Option<FountainHandles>,
}

/// The nodes of a fountain fill placed freely (see [`Fountain::handles`]),
/// relative to the object's bounding box `b`: a point `p` is stored as
/// `((p.x - c.x) / (w / 2), (p.y - c.y) / (h / 2))` with `c` the box's
/// centre, so they follow the object when it is moved or scaled.
///
/// The fill's own frame maps `(u, v)` to `start + u (end - start) +
/// v (skew - start)`. Linear: `t = u` (0 at the start, 1 at the end; the
/// bands run along `skew - start`). Radial (elliptical): `t = |(u, v)|`
/// (the end and the skew handle on the ellipse's two half axes). Conical:
/// the angle of `(u, v)` from the end, mirrored as the conical fill is.
/// Square: `t = max(|u|, |v|)` (the end and the skew handle on the
/// parallelogram's half axes).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FountainHandles {
    pub start: Point,
    pub end: Point,
    pub skew: Point,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

fn one_u32() -> u32 {
    1
}

fn is_one_u32(v: &u32) -> bool {
    *v == 1
}

/// `t` only moves and scales along the page axes, without a mirror: a box
/// stays a box with its sides in the same order.
fn keeps_box_axes(t: Affine) -> bool {
    let [a, b, c, d, _, _] = t.as_coeffs();
    let scale = a.abs().max(d.abs()).max(1e-12);
    b.abs() <= 1e-12 * scale && c.abs() <= 1e-12 * scale && a > 0.0 && d > 0.0
}

impl Default for Fountain {
    /// Black to white, left to right, running once.
    fn default() -> Self {
        Fountain::two(FountainKind::Linear, Color::BLACK, Color::WHITE, 0.0)
    }
}

impl Fountain {
    /// Most cycles a repeating fountain runs across its span.
    pub const MAX_REPEATS: u32 = 50;

    pub fn two(kind: FountainKind, from: Color, to: Color, angle: f64) -> Self {
        Fountain {
            kind,
            stops: vec![Stop::new(0.0, from), Stop::new(1.0, to)],
            angle,
            offset: Point::ZERO,
            edge_pad: 0.0,
            repeat: FountainRepeat::None,
            repeats: 1,
            bilinear: false,
            invert: false,
            handles: None,
        }
    }

    /// A box-relative point (see [`FountainHandles`]) on the page, for the
    /// object's bounding box `b`.
    pub fn rel_to_page(b: Rect, r: Point) -> Point {
        let c = b.center();
        Point::new(c.x + r.x * b.width() / 2.0, c.y + r.y * b.height() / 2.0)
    }

    /// A page point as a box-relative one for `b` (0 on a flat side).
    pub fn page_to_rel(b: Rect, p: Point) -> Point {
        let c = b.center();
        let rx = if b.width() > 1e-9 {
            (p.x - c.x) / (b.width() / 2.0)
        } else {
            0.0
        };
        let ry = if b.height() > 1e-9 {
            (p.y - c.y) / (b.height() / 2.0)
        } else {
            0.0
        };
        Point::new(rx, ry)
    }

    /// The fill's frame on the page for the object's box `b`, mapping
    /// `(u, v)` to `start + u (end - start) + v (skew - start)`: from the
    /// nodes when they are placed freely, else `None` (the angle and the
    /// offset place the fill). A frame that is flat counts as none.
    pub fn frame(&self, b: Rect) -> Option<Affine> {
        if self.kind == FountainKind::Shaped {
            return None;
        }
        let h = self.handles?;
        let s = Self::rel_to_page(b, h.start);
        let e = Self::rel_to_page(b, h.end) - s;
        let k = Self::rel_to_page(b, h.skew) - s;
        let det = e.x * k.y - e.y * k.x;
        if !det.is_finite() || det.abs() < 1e-12 {
            return None;
        }
        Some(Affine::new([e.x, e.y, k.x, k.y, s.x, s.y]))
    }

    /// The fill's parameter `t` (before [`Self::stop_param`]) at the page
    /// point `p`, for a fill whose nodes are placed freely (`None`
    /// otherwise; see [`FountainHandles`]).
    pub fn param_at(&self, b: Rect, p: Point) -> Option<f64> {
        let q = self.frame(b)?.inverse() * p;
        let (u, v) = (q.x, q.y);
        Some(match self.kind {
            FountainKind::Linear => u,
            FountainKind::Radial => u.hypot(v),
            FountainKind::Conical => {
                let a = v.atan2(u).rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU;
                if a < 0.5 {
                    a * 2.0
                } else {
                    2.0 - a * 2.0
                }
            }
            FountainKind::Square => u.abs().max(v.abs()),
            FountainKind::Shaped => return None,
        })
    }

    /// The nodes that place the fill as its angle and offset do now on
    /// the box `b` (where the Interactive Fill tool shows them before they
    /// are first dragged).
    pub fn default_handles(&self, b: Rect) -> FountainHandles {
        let a = self.angle.to_radians();
        let (ca, sa) = (a.cos(), a.sin());
        let c = b.center();
        let rel = |p: Point| Self::page_to_rel(b, p);
        match self.kind {
            FountainKind::Linear | FountainKind::Shaped => {
                let half = (b.width() * ca.abs() + b.height() * sa.abs()) / 2.0;
                let d = Vec2::new(ca, sa) * half;
                let s = c - d;
                let n = Vec2::new(-sa, ca) * (2.0 * half);
                FountainHandles {
                    start: rel(s),
                    end: rel(c + d),
                    skew: rel(s + n),
                }
            }
            FountainKind::Radial => {
                let cc = Point::new(
                    c.x + self.offset.x * b.width() / 2.0,
                    c.y + self.offset.y * b.height() / 2.0,
                );
                let r = b.width().max(b.height()) / 2.0 * std::f64::consts::SQRT_2;
                FountainHandles {
                    start: rel(cc),
                    end: rel(cc + Vec2::new(ca, sa) * r),
                    skew: rel(cc + Vec2::new(-sa, ca) * r),
                }
            }
            // Conical and square fills are measured in the box's own
            // units (half the width and half the height), around the
            // offset; the conical one turned by the angle.
            FountainKind::Conical => {
                let o = self.offset;
                FountainHandles {
                    start: o,
                    end: Point::new(o.x + ca, o.y + sa),
                    skew: Point::new(o.x - sa, o.y + ca),
                }
            }
            FountainKind::Square => {
                let o = self.offset;
                FountainHandles {
                    start: o,
                    end: Point::new(o.x + 1.0, o.y),
                    skew: Point::new(o.x, o.y + 1.0),
                }
            }
        }
    }

    /// Place the fill's nodes (box `b`) and keep the angle and the offset
    /// the nearest the formats that know only those can show: the angle
    /// of the axis, and for the kinds with a centre the start as the
    /// offset.
    pub fn set_handles(&mut self, b: Rect, h: FountainHandles) {
        let s = Self::rel_to_page(b, h.start);
        let e = Self::rel_to_page(b, h.end);
        let d = e - s;
        if d.hypot() > 1e-9 && d.x.is_finite() && d.y.is_finite() {
            self.angle = d.y.atan2(d.x).to_degrees();
        }
        if self.kind != FountainKind::Linear {
            self.offset = Point::new(h.start.x.clamp(-1.0, 1.0), h.start.y.clamp(-1.0, 1.0));
        }
        self.handles = Some(h);
    }

    /// Carry the fill along with `t`, a page-space transformation of its
    /// object whose page box goes from `before` to `after` (#775): a turn,
    /// a mirror or a skew turns, mirrors or skews the fill with the object,
    /// its nodes then placed freely unless its angle and centre place it
    /// the same (a quarter or half turn, a mirror). A move or a scale without a mirror
    /// changes nothing, the fill being placed relative to the box anyway;
    /// a shaped fill follows the outline by itself.
    pub fn carry(&mut self, t: Affine, before: Rect, after: Rect) {
        if self.kind == FountainKind::Shaped || keeps_box_axes(t) {
            return;
        }
        let flat = |b: Rect| !(b.width() > 1e-9 && b.height() > 1e-9);
        if flat(before) || flat(after) {
            return;
        }
        let h = self.handles.unwrap_or_else(|| self.default_handles(before));
        let moved = |p: Point| Self::page_to_rel(after, t * Self::rel_to_page(before, p));
        let h = FountainHandles {
            start: moved(h.start),
            end: moved(h.end),
            skew: moved(h.skew),
        };
        if ![h.start, h.end, h.skew]
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite())
        {
            return;
        }
        // A quarter or half turn, or a mirror, often gives a fill its angle
        // and centre can place exactly as well: it stays one placed that
        // way, so the angle field still turns it.
        let mut placed = self.clone();
        placed.set_handles(after, h);
        placed.handles = None;
        let d = placed.default_handles(after);
        let near = |a: Point, b: Point| (a - b).hypot() <= 1e-9;
        if self.handles.is_none()
            && near(d.start, h.start)
            && near(d.end, h.end)
            && near(d.skew, h.skew)
        {
            *self = placed;
        } else {
            self.set_handles(after, h);
        }
    }

    pub fn first_color(&self) -> Color {
        self.stops.first().map(|s| s.color).unwrap_or(Color::BLACK)
    }

    pub fn last_color(&self) -> Color {
        self.stops.last().map(|s| s.color).unwrap_or(Color::WHITE)
    }

    /// Cycles across the span (1 unless the fountain repeats).
    pub fn cycles(&self) -> u32 {
        match self.repeat {
            FountainRepeat::None => 1,
            _ => self.repeats.clamp(1, Self::MAX_REPEATS),
        }
    }

    /// No repeat, bilinear or invert: the stops run once over the span.
    pub fn runs_once(&self) -> bool {
        self.cycles() == 1 && !self.bilinear && !self.invert
    }

    /// A stop's midpoint is off the middle.
    pub fn has_midpoints(&self) -> bool {
        let n = self.stops.len();
        self.stops
            .iter()
            .take(n.saturating_sub(1))
            .any(|s| !is_half(&s.mid))
    }

    /// The sorted stops with each blend whose midpoint is off the middle
    /// drawn as [`Self::MID_STEPS`] straight pieces along its curve, so a
    /// plain run of stops gives the same colours (the midpoints set back
    /// to the middle).
    pub fn blended_stops(&self) -> Vec<Stop> {
        let base = self.sorted_stops();
        let mut out: Vec<Stop> = Vec::with_capacity(base.len());
        for (i, s) in base.iter().enumerate() {
            out.push(Stop { mid: 0.5, ..*s });
            let Some(next) = base.get(i + 1) else {
                continue;
            };
            if is_half(&s.mid) || next.pos - s.pos < 1e-9 {
                continue;
            }
            for j in 1..Self::MID_STEPS {
                let k = j as f64 / Self::MID_STEPS as f64;
                let w = midpoint_curve(k, s.mid) as f32;
                out.push(Stop {
                    pos: s.pos + (next.pos - s.pos) * k,
                    color: lerp_color(s.color, next.color, w),
                    mid: 0.5,
                    opacity: s.opacity + (next.opacity - s.opacity) * w,
                });
            }
        }
        out
    }

    /// Pieces per blend with its midpoint off the middle (see
    /// [`Self::blended_stops`]).
    pub const MID_STEPS: usize = 16;

    /// Any stop is not fully opaque.
    pub fn has_opacity(&self) -> bool {
        self.stops.iter().any(|s| s.opacity < 1.0)
    }

    /// The stops sorted by position, at least two.
    pub fn sorted_stops(&self) -> Vec<Stop> {
        let mut v = self.stops.clone();
        v.sort_by(|a, b| {
            a.pos
                .partial_cmp(&b.pos)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if v.len() < 2 {
            let c = v.first().map(|s| s.color).unwrap_or(Color::BLACK);
            v = vec![Stop::new(0.0, c), Stop::new(1.0, c)];
        }
        v
    }

    /// Where in the stops the colour at `t` (0..1 across the span) is
    /// taken: the bilinear mirror, then the repeat cycles, the edge pad
    /// and the inversion.
    pub fn stop_param(&self, t: f64) -> f64 {
        let mut u = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if self.bilinear {
            u = 1.0 - (2.0 * u - 1.0).abs();
        }
        let n = self.cycles() as f64;
        if n > 1.0 {
            let x = u * n;
            let k = x.floor();
            let fr = x - k;
            u = match self.repeat {
                FountainRepeat::Mirror if (k as i64) % 2 == 1 => 1.0 - fr,
                FountainRepeat::Mirror => fr,
                _ if x >= n => 1.0,
                _ => fr,
            };
        }
        let e = self.edge_pad.clamp(0.0, 0.49);
        if e > 0.0 {
            u = ((u - e) / (1.0 - 2.0 * e)).clamp(0.0, 1.0);
        }
        if self.invert {
            u = 1.0 - u;
        }
        u
    }

    /// Colour at `t` in 0..1 (sRGB interpolation).
    pub fn color_at(&self, t: f64) -> Color {
        sample_stops(&self.blended_stops(), self.stop_param(t))
    }

    /// The opacity at `t` (0 clear, 1 opaque).
    pub fn opacity_at(&self, t: f64) -> f32 {
        sample_opacity(&self.blended_stops(), self.stop_param(t))
    }

    /// The fountain as plain stops over 0..1 that give the same colours
    /// in one run, with the edge pad, repeat cycles, bilinear mirror and
    /// inversion worked in (for gradient shaders and exports that only
    /// know a single run of stops). Equal positions are hard edges.
    pub fn resolved_stops(&self) -> Vec<Stop> {
        let base = self.blended_stops();
        let cycles = self.cycles();
        let n = cycles as f64;
        let e = self.edge_pad.clamp(0.0, 0.49);
        // Where the mapping from t to the stop parameter bends.
        let mut us = Vec::new();
        for k in 0..=cycles {
            let k = k as f64;
            us.push(k / n);
            if e > 0.0 && k < n {
                us.push((k + e) / n);
                us.push((k + 1.0 - e) / n);
            }
        }
        let mut ts: Vec<f64> = Vec::new();
        for u in us {
            if self.bilinear {
                ts.push(u / 2.0);
                ts.push(1.0 - u / 2.0);
            } else {
                ts.push(u);
            }
        }
        ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        ts.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        let mut out: Vec<Stop> = Vec::new();
        let mut push = |pos: f64, s: Stop| {
            if let Some(l) = out.last() {
                if (l.pos - pos).abs() < 1e-12 && l.color == s.color && l.opacity == s.opacity {
                    return;
                }
            }
            out.push(Stop { pos, ..s });
        };
        for w in ts.windows(2) {
            let (a, b) = (w[0], w[1]);
            if b - a < 1e-12 {
                continue;
            }
            // The mapping is straight between bends: its ends from two
            // samples inside, so a sawtooth's jump lands on the right side.
            let (q1, q3) = (a + (b - a) * 0.25, a + (b - a) * 0.75);
            let (p1, p3) = (self.stop_param(q1), self.stop_param(q3));
            let slope = (p3 - p1) / (q3 - q1);
            let pa = (p1 - slope * (q1 - a)).clamp(0.0, 1.0);
            let pb = (p3 + slope * (b - q3)).clamp(0.0, 1.0);
            push(a, sample_stop(&base, pa));
            if (pb - pa).abs() > 1e-12 {
                let (lo, hi) = (pa.min(pb), pa.max(pb));
                let mut inner: Vec<Stop> = base
                    .iter()
                    .filter(|s| s.pos > lo + 1e-9 && s.pos < hi - 1e-9)
                    .map(|s| Stop {
                        pos: a + (s.pos - pa) / (pb - pa) * (b - a),
                        ..*s
                    })
                    .collect();
                if pb < pa {
                    inner.reverse();
                }
                for s in inner {
                    push(s.pos, s);
                }
            }
            push(b, sample_stop(&base, pb));
        }
        if out.len() < 2 {
            return base;
        }
        out
    }

    /// The closest fountain that runs once with no edge pad and one of
    /// the four geometric kinds, for formats that know nothing more: the
    /// options are worked into the stops (at most `max_stops`), and a
    /// shaped fountain becomes a square one running from the centre out.
    /// A fountain with none of the options comes back unchanged.
    pub fn to_plain(&self, max_stops: usize) -> Fountain {
        if self.runs_once() && self.kind != FountainKind::Shaped && !self.has_midpoints() {
            return self.clone();
        }
        let mut stops = self.resolved_stops();
        if self.kind == FountainKind::Shaped {
            // Shaped: 0 at the edge; square: 0 at the centre.
            stops.reverse();
            for s in &mut stops {
                s.pos = 1.0 - s.pos;
            }
        }
        let max_stops = max_stops.max(2);
        if stops.len() > max_stops {
            let k = (max_stops - 1) as f64;
            stops = (0..max_stops)
                .map(|i| {
                    let pos = i as f64 / k;
                    Stop {
                        pos,
                        ..sample_stop(&stops, pos)
                    }
                })
                .collect();
        }
        Fountain {
            kind: match self.kind {
                FountainKind::Shaped => FountainKind::Square,
                k => k,
            },
            stops,
            angle: self.angle,
            offset: self.offset,
            edge_pad: 0.0,
            repeat: FountainRepeat::None,
            repeats: 1,
            bilinear: false,
            invert: false,
            handles: None,
        }
    }
}

/// Colour at `t` along `stops` (sorted by position): the two around it
/// mixed in sRGB, the end colours beyond them.
pub fn sample_stops(stops: &[Stop], t: f64) -> Color {
    let Some(first) = stops.first() else {
        return Color::BLACK;
    };
    if t <= first.pos {
        return first.color;
    }
    for w in stops.windows(2) {
        if t <= w[1].pos {
            let span = (w[1].pos - w[0].pos).max(1e-9);
            let k = ((t - w[0].pos) / span).clamp(0.0, 1.0) as f32;
            return lerp_color(w[0].color, w[1].color, k);
        }
    }
    stops.last().map(|s| s.color).unwrap_or(first.color)
}

/// Opacity at `t` along `stops` (sorted by position), as
/// [`sample_stops`] mixes the colours.
pub fn sample_opacity(stops: &[Stop], t: f64) -> f32 {
    sample_stop(stops, t).opacity
}

/// The stop at `t` along `stops` (sorted by position): colour and opacity
/// mixed from the two around it, its blend half way.
pub fn sample_stop(stops: &[Stop], t: f64) -> Stop {
    let Some(first) = stops.first() else {
        return Stop::new(t, Color::BLACK);
    };
    if t <= first.pos {
        return Stop {
            pos: t,
            mid: 0.5,
            ..*first
        };
    }
    for w in stops.windows(2) {
        if t <= w[1].pos {
            let span = (w[1].pos - w[0].pos).max(1e-9);
            let k = ((t - w[0].pos) / span).clamp(0.0, 1.0) as f32;
            return Stop {
                pos: t,
                color: lerp_color(w[0].color, w[1].color, k),
                mid: 0.5,
                opacity: w[0].opacity + (w[1].opacity - w[0].opacity) * k,
            };
        }
    }
    let last = stops.last().copied().unwrap_or(*first);
    Stop {
        pos: t,
        mid: 0.5,
        ..last
    }
}

pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let [r1, g1, b1] = a.to_rgb8();
    let [r2, g2, b2] = b.to_rgb8();
    let l = |x: u8, y: u8| {
        (x as f32 + (y as f32 - x as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::rgb8(l(r1, r2), l(g1, g2), l(b1, b2))
}

/// Built-in two-colour pattern tiles (the two-colour pattern fill).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PatternTile {
    #[default]
    Checker,
    Dots,
    Stripes,
    DiagonalStripes,
    Grid,
    Bricks,
    Crosshatch,
    Waves,
}

impl PatternTile {
    pub const ALL: [PatternTile; 8] = [
        PatternTile::Checker,
        PatternTile::Dots,
        PatternTile::Stripes,
        PatternTile::DiagonalStripes,
        PatternTile::Grid,
        PatternTile::Bricks,
        PatternTile::Crosshatch,
        PatternTile::Waves,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PatternTile::Checker => "Checker",
            PatternTile::Dots => "Dots",
            PatternTile::Stripes => "Stripes",
            PatternTile::DiagonalStripes => "Diagonal stripes",
            PatternTile::Grid => "Grid",
            PatternTile::Bricks => "Bricks",
            PatternTile::Crosshatch => "Crosshatch",
            PatternTile::Waves => "Waves",
        }
    }

    /// Is the tile pixel at (u, v) in 0..1 "front" (true) or "back" colour?
    pub fn front(self, u: f64, v: f64) -> bool {
        match self {
            PatternTile::Checker => (u < 0.5) == (v < 0.5),
            PatternTile::Dots => ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() < 0.3,
            PatternTile::Stripes => v < 0.5,
            PatternTile::DiagonalStripes => (u + v).rem_euclid(1.0) < 0.5,
            PatternTile::Grid => u < 0.12 || v < 0.12,
            PatternTile::Bricks => {
                let row = v < 0.5;
                let uu = if row { u } else { (u + 0.5).rem_euclid(1.0) };
                v.rem_euclid(0.5) < 0.06 || uu < 0.06
            }
            PatternTile::Crosshatch => {
                (u + v).rem_euclid(0.5) < 0.07 || (u - v).rem_euclid(0.5) < 0.07
            }
            PatternTile::Waves => ((v - 0.5) - 0.2 * (u * std::f64::consts::TAU).sin()).abs() < 0.1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "pattern", rename_all = "lowercase")]
pub enum Pattern {
    TwoColor {
        tile: PatternTile,
        front: Color,
        back: Color,
        size_mm: f64,
    },
    /// Tiled PNG, `size_mm` is the tile width; height follows the aspect ratio.
    Bitmap {
        png: crate::SharedBytes,
        width_px: u32,
        height_px: u32,
        size_mm: f64,
    },
    /// Full-colour vector tile (the vector pattern fill):
    /// `shapes` are in tile-local coordinates, origin at the bottom-left
    /// corner of the tile, Y up, and the tile repeats every `tile.width` by
    /// `tile.height` millimetres. The shapes carry their own transforms,
    /// fills and outlines; there is no extra pattern transform.
    Vector {
        #[serde(default)]
        shapes: Vec<crate::document::Shape>,
        tile: Size,
    },
}

impl Pattern {
    /// Smallest tile edge, in mm, so a degenerate selection still tiles.
    pub const MIN_TILE_MM: f64 = 0.01;

    /// Build a vector pattern from page-space shapes: the shapes are moved
    /// so their joint bounds start at the origin and the tile takes the
    /// size of those bounds.
    pub fn vector_from_shapes(shapes: &[crate::document::Shape]) -> Pattern {
        let mut bounds: Option<Rect> = None;
        for s in shapes {
            let b = s.bounds();
            if !(b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite()) {
                continue;
            }
            bounds = Some(match bounds {
                Some(acc) => acc.union(b),
                None => b,
            });
        }
        let bounds = bounds.unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        let shift = Affine::translate((-bounds.x0, -bounds.y0));
        let shapes = shapes
            .iter()
            .map(|s| {
                let mut s = s.clone();
                s.transform_by(shift);
                s
            })
            .collect();
        Pattern::Vector {
            shapes,
            tile: Size::new(
                bounds.width().max(Self::MIN_TILE_MM),
                bounds.height().max(Self::MIN_TILE_MM),
            ),
        }
    }
}

/// Procedural textures (the texture fill).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TextureKind {
    #[default]
    Clouds,
    Marble,
    Noise,
    Wood,
}

impl TextureKind {
    pub const ALL: [TextureKind; 4] = [
        TextureKind::Clouds,
        TextureKind::Marble,
        TextureKind::Noise,
        TextureKind::Wood,
    ];
    pub fn name(self) -> &'static str {
        match self {
            TextureKind::Clouds => "Clouds",
            TextureKind::Marble => "Marble",
            TextureKind::Noise => "Noise",
            TextureKind::Wood => "Wood",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Texture {
    pub kind: TextureKind,
    pub color_a: Color,
    pub color_b: Color,
    /// Feature size in mm.
    pub scale: f64,
    pub seed: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "fill", rename_all = "lowercase")]
pub enum Fill {
    #[default]
    None,
    Solid(Color),
    Fountain(Fountain),
    Pattern(Pattern),
    Texture(Texture),
    Mesh(Mesh),
}

/// Mesh fill: a grid of (rows+1) x (cols+1) nodes with a colour each;
/// the cells between them are patches whose sides are the grid lines
/// (cubic curves when the nodes carry handles, straight otherwise),
/// their colour mixed from the corners (Coons patches), clipped by the
/// object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mesh {
    pub rows: u32,
    pub cols: u32,
    /// Row-major, (rows+1)*(cols+1) nodes in local space.
    pub nodes: Vec<MeshNode>,
    #[serde(default)]
    pub smooth: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeshNode {
    pub pos: Point,
    pub color: Color,
    /// 1.0 = opaque.
    #[serde(default = "one_f32")]
    pub alpha: f32,
    /// The handles of the grid lines through the node, in local space:
    /// towards the next column, the previous column, the next row and
    /// the previous row (see [`MeshHandle`]). `None` draws straight
    /// lines to the neighbours.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handles: Option<[Point; 4]>,
}

/// Which of a node's four handles: the index into [`MeshNode::handles`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshHandle {
    /// Towards the next column (the node to the right in the grid).
    Right = 0,
    /// Towards the previous column.
    Left = 1,
    /// Towards the next row.
    Down = 2,
    /// Towards the previous row.
    Up = 3,
}

impl MeshNode {
    /// A node without handles.
    pub fn new(pos: Point, color: Color) -> Self {
        MeshNode {
            pos,
            color,
            alpha: 1.0,
            handles: None,
        }
    }

    /// The handle `h`, or the point a third of the way to `towards` when
    /// the node has none there (a handle on the node itself counts as
    /// none: a straight grid line).
    pub fn handle(&self, h: MeshHandle, towards: Point) -> Point {
        match self.handles {
            Some(hs) if hs[h as usize] != self.pos => hs[h as usize],
            _ => self.pos.lerp(towards, 1.0 / 3.0),
        }
    }

    /// Set the handle `h`, keeping the others (straight when unset).
    pub fn set_handle(&mut self, h: MeshHandle, p: Point) {
        let hs = self.handles.get_or_insert([self.pos; 4]);
        hs[h as usize] = p;
    }

    /// The node moved by `d`, its handles with it.
    pub fn translate(&mut self, d: Vec2) {
        self.pos += d;
        if let Some(hs) = &mut self.handles {
            for h in hs.iter_mut() {
                *h += d;
            }
        }
    }

    /// The node and its handles through `t`.
    pub fn transform(&mut self, t: Affine) {
        self.pos = t * self.pos;
        if let Some(hs) = &mut self.handles {
            for h in hs.iter_mut() {
                *h = t * *h;
            }
        }
    }
}

impl Mesh {
    pub fn new(bounds: crate::geometry::Rect, rows: u32, cols: u32, base: Color) -> Self {
        let mut nodes = Vec::new();
        for r in 0..=rows {
            for c in 0..=cols {
                nodes.push(MeshNode::new(
                    Point::new(
                        bounds.x0 + bounds.width() * c as f64 / cols as f64,
                        bounds.y0 + bounds.height() * r as f64 / rows as f64,
                    ),
                    base,
                ));
            }
        }
        Mesh {
            rows,
            cols,
            nodes,
            smooth: false,
        }
    }

    pub fn node(&self, r: u32, c: u32) -> Option<&MeshNode> {
        self.nodes.get((r * (self.cols + 1) + c) as usize)
    }

    /// The mesh with every node and handle through `t`.
    pub fn transformed(&self, t: Affine) -> Mesh {
        let mut m = self.clone();
        for n in m.nodes.iter_mut() {
            n.transform(t);
        }
        m
    }

    /// Whether any node carries handles (curved grid lines).
    pub fn is_curved(&self) -> bool {
        self.nodes.iter().any(|n| n.handles.is_some())
    }

    /// The grid line between the nodes at `(r, c)` and `(r, c + 1)`
    /// (`across` true) or `(r + 1, c)` (false), as a cubic curve.
    pub fn edge(&self, r: u32, c: u32, across: bool) -> Option<crate::geometry::CubicBez> {
        let (b, h0, h1) = if across {
            (self.node(r, c + 1)?, MeshHandle::Right, MeshHandle::Left)
        } else {
            (self.node(r + 1, c)?, MeshHandle::Down, MeshHandle::Up)
        };
        let a = self.node(r, c)?;
        Some(crate::geometry::CubicBez::new(
            a.pos,
            a.handle(h0, b.pos),
            b.handle(h1, a.pos),
            b.pos,
        ))
    }

    /// The point of the cell `(r, c)` at `(u, v)` (0 to 1 across and
    /// down): the Coons patch bounded by the cell's four grid lines.
    pub fn patch_point(&self, r: u32, c: u32, u: f64, v: f64) -> Option<Point> {
        use crate::geometry::ParamCurve;
        let top = self.edge(r, c, true)?;
        let bottom = self.edge(r + 1, c, true)?;
        let left = self.edge(r, c, false)?;
        let right = self.edge(r, c + 1, false)?;
        let (a, b, d, e) = (top.p0, top.p3, bottom.p0, bottom.p3);
        let p = top.eval(u).to_vec2() * (1.0 - v)
            + bottom.eval(u).to_vec2() * v
            + left.eval(v).to_vec2() * (1.0 - u)
            + right.eval(v).to_vec2() * u
            - a.to_vec2() * ((1.0 - u) * (1.0 - v))
            - b.to_vec2() * (u * (1.0 - v))
            - d.to_vec2() * ((1.0 - u) * v)
            - e.to_vec2() * (u * v);
        Some(p.to_point())
    }

    /// Index of the node nearest to `p` (local space).
    pub fn nearest(&self, p: Point) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .min_by(|a, b| {
                (a.1.pos - p)
                    .hypot()
                    .partial_cmp(&(b.1.pos - p).hypot())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    }

    /// The node grid's (row, column) of node `i`.
    pub fn row_col(&self, i: usize) -> (usize, usize) {
        let cols = self.cols as usize + 1;
        (i / cols, i % cols)
    }

    /// The mesh as the grid sees it at row `v` and column `u` (fractions
    /// of the rows and columns, 0 to 1): position, colour and opacity
    /// mixed from the four nodes around it.
    pub fn sample(&self, u: f64, v: f64) -> Option<MeshNode> {
        let (rows, cols) = (self.rows.max(1) as f64, self.cols.max(1) as f64);
        let (x, y) = (u.clamp(0.0, 1.0) * cols, v.clamp(0.0, 1.0) * rows);
        let c = (x.floor() as u32).min(self.cols.saturating_sub(1));
        let r = (y.floor() as u32).min(self.rows.saturating_sub(1));
        let (fu, fv) = (x - c as f64, y - r as f64);
        let (a, b, d, e) = (
            *self.node(r, c)?,
            *self.node(r, c + 1)?,
            *self.node(r + 1, c)?,
            *self.node(r + 1, c + 1)?,
        );
        let mix = |p: MeshNode, q: MeshNode, t: f64| MeshNode {
            pos: p.pos.lerp(q.pos, t),
            color: lerp_color(p.color, q.color, t as f32),
            alpha: p.alpha + (q.alpha - p.alpha) * t as f32,
            handles: None,
        };
        Some(mix(mix(a, b, fu), mix(d, e, fu), fv))
    }

    /// The same mesh on a grid of `rows` by `cols` cells spread evenly over
    /// its nodes' bounds, each new node taking the colour and opacity the
    /// old grid has there.
    pub fn regrid(&self, rows: u32, cols: u32) -> Mesh {
        let (rows, cols) = (rows.clamp(1, 50), cols.clamp(1, 50));
        let bounds = self
            .nodes
            .iter()
            .map(|n| crate::geometry::Rect::from_points(n.pos, n.pos))
            .reduce(|a, b| a.union(b))
            .unwrap_or_default();
        let base = self.nodes.first().map(|n| n.color).unwrap_or(Color::WHITE);
        let mut out = Mesh::new(bounds, rows, cols, base);
        out.smooth = self.smooth;
        for r in 0..=rows {
            for c in 0..=cols {
                let i = (r * (cols + 1) + c) as usize;
                if let (Some(old), Some(n)) = (
                    self.sample(c as f64 / cols as f64, r as f64 / rows as f64),
                    out.nodes.get_mut(i),
                ) {
                    n.color = old.color;
                    n.alpha = old.alpha;
                }
            }
        }
        out
    }

    /// Remove the grid lines through node `i` that are not on the edge:
    /// its column, its row, or both. False when both are on the edge.
    /// Where the removed line had cut a curved grid line, the neighbours'
    /// handles are stretched back so the curve through them is restored.
    pub fn remove_lines_through(&mut self, i: usize) -> bool {
        if i >= self.nodes.len() {
            return false;
        }
        let (r, c) = self.row_col(i);
        let cols = self.cols as usize + 1;
        let drop_col = c > 0 && c < self.cols as usize;
        let drop_row = r > 0 && r < self.rows as usize;
        if !drop_col && !drop_row {
            return false;
        }
        if drop_col {
            for rr in 0..=self.rows as usize {
                let k = rr * cols + c;
                if let (Some(&removed), Some(&a), Some(&b)) = (
                    self.nodes.get(k),
                    self.nodes.get(k - 1),
                    self.nodes.get(k + 1),
                ) {
                    let (mut a, mut b) = (a, b);
                    merge_nodes(&mut a, &mut b, &removed, true);
                    self.nodes[k - 1] = a;
                    self.nodes[k + 1] = b;
                }
            }
        }
        if drop_row {
            for cc in 0..cols {
                let k = r * cols + cc;
                if let (Some(&removed), Some(&a), Some(&b)) = (
                    self.nodes.get(k),
                    self.nodes.get(k - cols),
                    self.nodes.get(k + cols),
                ) {
                    let (mut a, mut b) = (a, b);
                    merge_nodes(&mut a, &mut b, &removed, false);
                    self.nodes[k - cols] = a;
                    self.nodes[k + cols] = b;
                }
            }
        }
        let nodes: Vec<MeshNode> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(k, _)| !(drop_col && k % cols == c) && !(drop_row && k / cols == r))
            .map(|(_, n)| *n)
            .collect();
        self.nodes = nodes;
        if drop_col {
            self.cols -= 1;
        }
        if drop_row {
            self.rows -= 1;
        }
        true
    }

    /// The cell of the grid that holds `p` (local space) and where in
    /// it: (row, column, fraction across, fraction up), the fractions
    /// those of the cell's patch (the Coons patch of its grid lines; the
    /// bilinear patch when they are straight).
    pub fn locate(&self, p: Point) -> Option<(usize, usize, f64, f64)> {
        for r in 0..self.rows {
            for c in 0..self.cols {
                // Newton steps on the patch from its middle, the slopes
                // taken by small steps; kept near the cell, as a point in
                // another cell makes the steps wander.
                let (mut u, mut v) = (0.5, 0.5);
                const STEP: f64 = 1e-6;
                let at = |u: f64, v: f64| self.patch_point(r, c, u, v);
                for _ in 0..16 {
                    let (Some(q), Some(qu), Some(qv)) =
                        (at(u, v), at(u + STEP, v), at(u, v + STEP))
                    else {
                        break;
                    };
                    let f = q - p;
                    if f.hypot() < 1e-9 {
                        break;
                    }
                    let du = (qu - q) / STEP;
                    let dv = (qv - q) / STEP;
                    let det = du.x * dv.y - du.y * dv.x;
                    if det.abs() < 1e-12 {
                        break;
                    }
                    u -= (f.x * dv.y - f.y * dv.x) / det;
                    v -= (du.x * f.y - du.y * f.x) / det;
                    if !(u.is_finite() && v.is_finite()) {
                        break;
                    }
                    u = u.clamp(-0.5, 1.5);
                    v = v.clamp(-0.5, 1.5);
                }
                const EPS: f64 = 1e-6;
                if (-EPS..=1.0 + EPS).contains(&u)
                    && (-EPS..=1.0 + EPS).contains(&v)
                    && at(u, v).is_some_and(|q| (q - p).hypot() < 1e-6)
                {
                    return Some((r as usize, c as usize, u.clamp(0.0, 1.0), v.clamp(0.0, 1.0)));
                }
            }
        }
        None
    }

    /// A column of nodes between columns `c` and `c + 1`, `t` of the way
    /// across, positions, colours and opacity mixed from its neighbours;
    /// a curved grid line is cut there, the neighbours' handles shortened
    /// to the cut.
    pub fn insert_column(&mut self, c: usize, t: f64) {
        let cols = self.cols as usize + 1;
        if c + 1 >= cols {
            return;
        }
        let mut nodes = Vec::with_capacity(self.nodes.len() + self.rows as usize + 1);
        for (k, n) in self.nodes.iter().enumerate() {
            let mut n = *n;
            if k % cols == c {
                if let Some(next) = self.nodes.get(k + 1) {
                    let mut next = *next;
                    let between = split_nodes(&mut n, &mut next, t, true);
                    nodes.push(n);
                    nodes.push(between);
                    continue;
                }
            } else if k % cols == c + 1 {
                if let Some(prev) = self.nodes.get(k - 1) {
                    let mut prev = *prev;
                    split_nodes(&mut prev, &mut n, t, true);
                }
            }
            nodes.push(n);
        }
        self.nodes = nodes;
        self.cols += 1;
    }

    /// A row of nodes between rows `r` and `r + 1`, `t` of the way up.
    pub fn insert_row(&mut self, r: usize, t: f64) {
        let cols = self.cols as usize + 1;
        if r >= self.rows as usize {
            return;
        }
        let at = (r + 1) * cols;
        let mut above: Vec<MeshNode> = Vec::with_capacity(cols);
        let mut below: Vec<MeshNode> = Vec::with_capacity(cols);
        let row: Vec<MeshNode> = (0..cols)
            .filter_map(|c| {
                let mut n = *self.nodes.get(r * cols + c)?;
                let mut next = *self.nodes.get((r + 1) * cols + c)?;
                let between = split_nodes(&mut n, &mut next, t, false);
                above.push(n);
                below.push(next);
                Some(between)
            })
            .collect();
        if row.len() != cols || at > self.nodes.len() {
            return;
        }
        for (c, (a, b)) in above.into_iter().zip(below).enumerate() {
            if let Some(n) = self.nodes.get_mut(r * cols + c) {
                *n = a;
            }
            if let Some(n) = self.nodes.get_mut((r + 1) * cols + c) {
                *n = b;
            }
        }
        self.nodes.splice(at..at, row);
        self.rows += 1;
    }

    /// Double-click at `p` (local space) inside the mesh: a node there,
    /// on a new grid line across and a new one up and down through `p`;
    /// within `tol` of a grid line only the line across it is added, and
    /// on a node nothing. The lines go through `p` itself, not halfway
    /// between the nearest nodes (#804). False when nothing changed.
    pub fn add_lines_at(&mut self, p: Point, tol: f64) -> bool {
        let Some((r, c, u, v)) = self.locate(p) else {
            return false;
        };
        let cols = self.cols as usize + 1;
        let pos = |r: usize, c: usize| self.nodes.get(r * cols + c).map(|n| n.pos);
        // The cell's size across and up, at `p`.
        let (Some(a), Some(b), Some(d), Some(e)) =
            (pos(r, c), pos(r, c + 1), pos(r + 1, c), pos(r + 1, c + 1))
        else {
            return false;
        };
        let width = (a.lerp(d, v) - b.lerp(e, v)).hypot().max(1e-9);
        let height = (a.lerp(b, u) - d.lerp(e, u)).hypot().max(1e-9);
        let near_column = u * width <= tol || (1.0 - u) * width <= tol;
        let near_row = v * height <= tol || (1.0 - v) * height <= tol;
        if !near_column {
            self.insert_column(c, u);
        }
        if !near_row {
            self.insert_row(r, v);
        }
        !(near_column && near_row)
    }
}

/// The node `removed` between `a` and `b` on a grid line (`across`
/// true: along a row, else down a column) goes: the handles of `a` and
/// `b` towards it are stretched back to span the whole line, undoing the
/// cut `split_nodes` made (the cut's place taken from the chords).
fn merge_nodes(a: &mut MeshNode, b: &mut MeshNode, removed: &MeshNode, across: bool) {
    if a.handles.is_none() && b.handles.is_none() && removed.handles.is_none() {
        return;
    }
    let (fwd, back) = if across {
        (MeshHandle::Right, MeshHandle::Left)
    } else {
        (MeshHandle::Down, MeshHandle::Up)
    };
    let la = (removed.pos - a.pos).hypot();
    let lb = (b.pos - removed.pos).hypot();
    if la + lb <= 0.0 {
        return;
    }
    let t = (la / (la + lb)).clamp(0.05, 0.95);
    let ha = a.pos + (a.handle(fwd, removed.pos) - a.pos) / t;
    let hb = b.pos + (b.handle(back, removed.pos) - b.pos) / (1.0 - t);
    a.set_handle(fwd, ha);
    b.set_handle(back, hb);
}

/// The node `t` of the way from `a` to `b` along the grid line between
/// them (`across` true: `b` is the next column, else the next row).
/// With handles, the line is cut there: the new node takes the cut's
/// handles and `a` and `b` have theirs shortened to it, so the line
/// keeps its shape; its handles across the line are mixed from the two.
fn split_nodes(a: &mut MeshNode, b: &mut MeshNode, t: f64, across: bool) -> MeshNode {
    let color = lerp_color(a.color, b.color, t as f32);
    let alpha = a.alpha + (b.alpha - a.alpha) * t as f32;
    if a.handles.is_none() && b.handles.is_none() {
        return MeshNode {
            pos: a.pos.lerp(b.pos, t),
            color,
            alpha,
            handles: None,
        };
    }
    use crate::geometry::{CubicBez, ParamCurve};
    let (fwd, back, side0, side1) = if across {
        (
            MeshHandle::Right,
            MeshHandle::Left,
            MeshHandle::Down,
            MeshHandle::Up,
        )
    } else {
        (
            MeshHandle::Down,
            MeshHandle::Up,
            MeshHandle::Right,
            MeshHandle::Left,
        )
    };
    let curve = CubicBez::new(a.pos, a.handle(fwd, b.pos), b.handle(back, a.pos), b.pos);
    let (first, second) = (curve.subsegment(0.0..t), curve.subsegment(t..1.0));
    let pos = first.p3;
    // The sideways handles: the neighbours' offsets mixed.
    let side = |h: MeshHandle| {
        let oa = a.handle(h, a.pos) - a.pos;
        let ob = b.handle(h, b.pos) - b.pos;
        pos + oa.lerp(ob, t)
    };
    let mut n = MeshNode {
        pos,
        color,
        alpha,
        handles: None,
    };
    n.set_handle(side0, side(side0));
    n.set_handle(side1, side(side1));
    n.set_handle(back, first.p2);
    n.set_handle(fwd, second.p1);
    a.set_handle(fwd, first.p1);
    b.set_handle(back, second.p2);
    n
}

impl Fill {
    /// Carry a fountain fill along with a turn, mirror or skew of its
    /// object (see [`Fountain::carry`]); other fills need nothing.
    pub fn carry(&mut self, t: Affine, before: Rect, after: Rect) {
        if let Fill::Fountain(f) = self {
            f.carry(t, before, after);
        }
    }

    pub fn linear(from: Color, to: Color, angle: f64) -> Self {
        Fill::Fountain(Fountain::two(FountainKind::Linear, from, to, angle))
    }

    pub fn radial(from: Color, to: Color) -> Self {
        Fill::Fountain(Fountain::two(FountainKind::Radial, from, to, 0.0))
    }

    /// A representative colour (first stop, front colour) for swatches.
    pub fn preview_color(&self) -> Option<Color> {
        match self {
            Fill::None => None,
            Fill::Solid(c) => Some(*c),
            Fill::Fountain(f) => Some(f.first_color()),
            Fill::Pattern(Pattern::TwoColor { front, .. }) => Some(*front),
            Fill::Pattern(Pattern::Bitmap { .. }) => Some(Color::Gray { v: 0.5 }),
            Fill::Pattern(Pattern::Vector { shapes, .. }) => Some(
                shapes
                    .iter()
                    .find_map(|s| s.fill.preview_color())
                    .unwrap_or(Color::Gray { v: 0.5 }),
            ),
            Fill::Texture(t) => Some(t.color_a),
            Fill::Mesh(m) => m.nodes.first().map(|n| n.color),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// The preset arrowhead size in outline widths (the head-space unit; see
/// [`Arrowhead`]). Hairlines count as 0.25 mm and the size is at least 1 mm.
pub const HEAD_SIZE_PER_WIDTH: f64 = 4.0;

/// Arrowheads at the ends of open curves.
///
/// Presets are drawn by [`arrowhead_paths`]; a `Custom` head carries its own
/// outline in a normalised head space: the bounding box is centred on
/// y = 0, the tip (largest x) sits at x = 0 and the shape extends towards
/// negative x, i.e. back along the curve, exactly one unit tall
/// ([`Arrowhead::from_shape_path`]). Heads an importer reads with their own
/// placement and size ([`Arrowhead::from_placed_path`]) keep both instead:
/// the origin is the curve's end point and the head may reach past it. At
/// draw time the unit is the preset size ([`HEAD_SIZE_PER_WIDTH`] outline
/// widths, at least 1 mm), x follows the curve direction at the end point
/// and y its left normal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Arrowhead {
    #[default]
    None,
    Arrow,
    OpenArrow,
    Circle,
    Square,
    Bar,
    Diamond,
    /// User-drawn head (Object > Create > Arrowhead).
    Custom {
        path: crate::geometry::BezPath,
        name: String,
    },
}

impl Arrowhead {
    /// The built-in presets, in menu order. Custom heads are not listed.
    pub const ALL: [Arrowhead; 7] = [
        Arrowhead::None,
        Arrowhead::Arrow,
        Arrowhead::OpenArrow,
        Arrowhead::Circle,
        Arrowhead::Square,
        Arrowhead::Bar,
        Arrowhead::Diamond,
    ];

    pub fn name(&self) -> &str {
        match self {
            Arrowhead::None => "None",
            Arrowhead::Arrow => "Arrow",
            Arrowhead::OpenArrow => "Open arrow",
            Arrowhead::Circle => "Circle",
            Arrowhead::Square => "Square",
            Arrowhead::Bar => "Bar",
            Arrowhead::Diamond => "Diamond",
            Arrowhead::Custom { name, .. } => name,
        }
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, Arrowhead::Custom { .. })
    }

    /// Build a custom head from any closed outline (page or local space,
    /// orientation: tip on the right, pointing towards +x). The outline is
    /// moved and scaled into head space (see the enum docs). An empty or
    /// degenerate outline (no height and no width) gives `Arrowhead::None`.
    pub fn from_shape_path(path: &crate::geometry::BezPath, name: impl Into<String>) -> Arrowhead {
        use crate::geometry::Shape as _;
        if path.elements().is_empty() {
            return Arrowhead::None;
        }
        let b = path.bounding_box();
        if !(b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite()) {
            return Arrowhead::None;
        }
        // A flat outline (a bar seen edge on) still gets a height from its
        // width so the head stays visible.
        let height = if b.height() > 1e-9 {
            b.height()
        } else if b.width() > 1e-9 {
            b.width()
        } else {
            return Arrowhead::None;
        };
        let scale = 1.0 / height;
        let center_y = (b.y0 + b.y1) / 2.0;
        let normalised =
            Affine::scale(scale) * Affine::translate((-b.x1, -center_y)) * path.clone();
        Arrowhead::Custom {
            path: normalised,
            name: name.into(),
        }
    }

    /// Build a custom head from an imported outline that is already placed
    /// the way the file draws it: the curve's end point at the origin, the
    /// curve coming in from -x, and `units_per_width` outline units making
    /// one outline width. The placement and size are kept as they are (no
    /// normalising), so the head is drawn as the file sizes it, whatever
    /// part of it lies ahead of or beside the end point. A head larger than
    /// 100 outline widths is scaled down to that. An empty or degenerate
    /// outline, or a unit that is not a finite positive number, gives
    /// `Arrowhead::None`.
    pub fn from_placed_path(
        path: &crate::geometry::BezPath,
        name: impl Into<String>,
        units_per_width: f64,
    ) -> Arrowhead {
        use crate::geometry::Shape as _;
        if path.elements().is_empty() || !(units_per_width.is_finite() && units_per_width > 0.0) {
            return Arrowhead::None;
        }
        let b = path.bounding_box();
        if !(b.x0.is_finite() && b.y0.is_finite() && b.x1.is_finite() && b.y1.is_finite()) {
            return Arrowhead::None;
        }
        let extent = b.width().max(b.height()) / units_per_width;
        if extent <= 1e-9 {
            return Arrowhead::None;
        }
        // Head space: one unit is HEAD_SIZE_PER_WIDTH outline widths.
        let scale = 1.0 / (units_per_width * HEAD_SIZE_PER_WIDTH) * (100.0 / extent).min(1.0);
        Arrowhead::Custom {
            path: Affine::scale(scale) * path.clone(),
            name: name.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub color: Color,
    /// Width in millimetres. A "hairline" is 0.0762 mm (0.216 pt).
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    /// Dash pattern as alternating on/off lengths, in multiples of the width.
    /// Empty = solid.
    pub dash: Vec<f64>,
    /// When true, the outline scales with the object ("Scale with
    /// object"). When false, width stays fixed under transforms.
    pub scale_with_object: bool,
    /// Draw the outline behind the fill.
    pub behind_fill: bool,
    #[serde(default)]
    pub start_arrow: Arrowhead,
    #[serde(default)]
    pub end_arrow: Arrowhead,
    /// Calligraphic nib stretch 0..1 and angle (1.0 = round nib).
    #[serde(default = "one")]
    pub stretch: f64,
    #[serde(default)]
    pub nib_angle: f64,
    /// A fill for the outline itself (a gradient, pattern or texture);
    /// None paints it with `color`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Box<Fill>>,
    /// Width nodes of a variable-width outline, in order along the path.
    /// Empty: the outline has `width` everywhere.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub widths: Vec<WidthNode>,
    /// Drawn as painterly brush marks along the path instead of a line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub painterly: Option<crate::painterly::Painterly>,
    /// Where a closed path's outline lies: outside the path, centred on
    /// it or inside it. Open paths are always centred.
    #[serde(default, skip_serializing_if = "OutlinePosition::is_center")]
    pub position: OutlinePosition,
    /// Mitre limit as an angle in degrees: mitred corners sharper than
    /// this are bevelled. Outlines saved before the field existed keep the
    /// limit they were drawn with (a length ratio of 4, about 29 degrees).
    #[serde(default = "legacy_miter_limit")]
    pub miter_limit: f64,
    /// Fit the dash pattern to each run between corners so that corners
    /// and line ends fall in the middle of a dash.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub align_dashes: bool,
}

/// Where an outline lies relative to the path of a closed object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutlinePosition {
    /// The whole width outside the path; the object looks larger.
    Outside,
    /// Half the width on each side of the path.
    #[default]
    Center,
    /// The whole width inside the path; the object keeps its size.
    Inside,
}

impl OutlinePosition {
    pub const ALL: [OutlinePosition; 3] = [
        OutlinePosition::Outside,
        OutlinePosition::Center,
        OutlinePosition::Inside,
    ];

    pub fn is_center(&self) -> bool {
        *self == OutlinePosition::Center
    }
}

fn legacy_miter_limit() -> f64 {
    Stroke::miter_angle_from_ratio(4.0)
}

/// A point of a variable-width outline: where it is along the path and
/// how far the outline reaches on each side of it there.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WidthNode {
    /// Place along the path, 0 (start) to 1 (end), by length over all
    /// subpaths.
    pub at: f64,
    /// Reach to the left of the path's direction, in millimetres.
    pub left: f64,
    /// Reach to the right, in millimetres.
    pub right: f64,
}

impl WidthNode {
    pub fn symmetric(at: f64, width: f64) -> Self {
        WidthNode {
            at,
            left: width / 2.0,
            right: width / 2.0,
        }
    }
}

/// The left and right reach of a variable outline at `t` (0 to 1): the
/// path's ends keep half the base width unless a node sits there, and
/// the reach eases between neighbouring nodes.
pub fn width_at(base: f64, nodes: &[WidthNode], t: f64) -> (f64, f64) {
    let half = (base / 2.0).max(0.0);
    let t = t.clamp(0.0, 1.0);
    let mut prev = WidthNode {
        at: 0.0,
        left: half,
        right: half,
    };
    let mut next = WidthNode {
        at: 1.0,
        left: half,
        right: half,
    };
    let mut found_next = false;
    for n in nodes {
        let n = WidthNode {
            at: n.at.clamp(0.0, 1.0),
            left: n.left.max(0.0),
            right: n.right.max(0.0),
        };
        if n.at <= t {
            prev = n;
        } else if !found_next {
            next = n;
            found_next = true;
        }
    }
    let span = next.at - prev.at;
    if span <= 1e-12 {
        return (prev.left, prev.right);
    }
    let u = ((t - prev.at) / span).clamp(0.0, 1.0);
    let s = u * u * (3.0 - 2.0 * u);
    (
        prev.left + (next.left - prev.left) * s,
        prev.right + (next.right - prev.right) * s,
    )
}

fn one() -> f64 {
    1.0
}

impl Stroke {
    pub const HAIRLINE: f64 = 0.0762;
    /// Mitre limit of new outlines, degrees: a corner sharper than a
    /// right angle's half is bevelled, so a thick outline on lettering
    /// does not grow long spikes at its narrow turns (#894).
    pub const DEFAULT_MITER_LIMIT: f64 = 45.0;
    /// The range the mitre limit is kept in, degrees.
    pub const MITER_LIMIT_RANGE: std::ops::RangeInclusive<f64> = 1.0..=179.0;

    /// The mitre limit as the ratio of the mitre length to the outline
    /// width (what renderers, SVG and PDF take): `1 / sin(angle / 2)`.
    /// A value that is not a finite angle uses the default.
    pub fn miter_ratio(&self) -> f64 {
        let a = if self.miter_limit.is_finite() {
            self.miter_limit
        } else {
            Self::DEFAULT_MITER_LIMIT
        };
        let a = a.clamp(
            *Self::MITER_LIMIT_RANGE.start(),
            *Self::MITER_LIMIT_RANGE.end(),
        );
        1.0 / (a.to_radians() / 2.0).sin()
    }

    /// The mitre limit angle (degrees) for a ratio of mitre length to
    /// width (SVG `stroke-miterlimit`, PDF `M`): `2 asin(1 / ratio)`.
    /// Ratios below 1 or not finite give the default.
    pub fn miter_angle_from_ratio(ratio: f64) -> f64 {
        if !ratio.is_finite() || ratio < 1.0 {
            return Self::DEFAULT_MITER_LIMIT;
        }
        (2.0 * (1.0 / ratio).asin().to_degrees()).clamp(
            *Self::MITER_LIMIT_RANGE.start(),
            *Self::MITER_LIMIT_RANGE.end(),
        )
    }

    pub fn hairline(color: Color) -> Self {
        Stroke {
            color,
            width: Self::HAIRLINE,
            ..Default::default()
        }
    }

    pub fn new(color: Color, width: f64) -> Self {
        Stroke {
            color,
            width,
            ..Default::default()
        }
    }
}

impl Default for Stroke {
    fn default() -> Self {
        Stroke {
            color: Color::BLACK,
            width: Self::HAIRLINE,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash: Vec::new(),
            scale_with_object: false,
            behind_fill: false,
            start_arrow: Arrowhead::None,
            end_arrow: Arrowhead::None,
            stretch: 1.0,
            nib_angle: 0.0,
            fill: None,
            widths: Vec::new(),
            painterly: None,
            position: OutlinePosition::Center,
            miter_limit: Self::DEFAULT_MITER_LIMIT,
            align_dashes: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #804: a double-click inside a mesh puts a node exactly where it
    /// was, also in a cell pulled out of shape, its colour mixed from
    /// the cell's corners; next to a grid line only the line across it
    /// is added, and on a node nothing.
    #[test]
    fn a_double_click_puts_the_new_mesh_node_where_it_was() {
        let b = crate::geometry::Rect::new(0.0, 0.0, 40.0, 20.0);
        let mut m = Mesh::new(b, 1, 2, Color::WHITE);
        // Pull the top-right node of the first cell out of shape and
        // colour the first cell's corners.
        m.nodes[4].pos = Point::new(26.0, 27.0);
        m.nodes[0].color = Color::rgb8(0, 0, 0);
        let p = Point::new(9.0, 13.0);
        let (r, c, u, v) = m.locate(p).expect("inside");
        assert_eq!((r, c), (0, 0));
        assert!(m.add_lines_at(p, 0.5));
        assert_eq!((m.rows, m.cols), (2, 3));
        let i = m.nearest(p).expect("node");
        assert!((m.nodes[i].pos - p).hypot() < 1e-6, "{:?}", m.nodes[i].pos);
        assert_eq!(m.row_col(i), (1, 1));
        // Its colour: the patch's colour there.
        let k = ((1.0 - u) * (1.0 - v) * 255.0).round();
        let red = m.nodes[i].color.to_rgb8()[0] as f64;
        assert!((red - (255.0 - k)).abs() <= 2.0, "{red} {k}");
        // Next to the new column, only a row; on a node, nothing.
        let below = m.node(0, 1).expect("node").pos;
        let on_column = m.nodes[i].pos.midpoint(below);
        let q = Point::new(on_column.x + 0.2, on_column.y);
        assert!(m.add_lines_at(q, 0.5));
        assert_eq!((m.rows, m.cols), (3, 3));
        assert!(!m.add_lines_at(p, 0.5));
        assert_eq!((m.rows, m.cols), (3, 3));
        assert!(m.locate(Point::new(-5.0, 5.0)).is_none());
        assert!(!m.add_lines_at(Point::new(-5.0, 5.0), 0.5));
    }

    /// A node's handles follow it when it moves or the mesh is
    /// transformed; a column added through a curved cell cuts the curve
    /// where the new node is, keeping the grid line's shape (#896).
    #[test]
    fn mesh_handles_move_with_their_nodes_and_cut_at_new_lines() {
        use crate::geometry::{Affine, ParamCurve, Vec2};
        let b = crate::geometry::Rect::new(0.0, 0.0, 30.0, 10.0);
        let mut m = Mesh::new(b, 1, 1, Color::WHITE);
        assert!(!m.is_curved());
        // The top line bows down to y = 7.5 in the middle.
        m.nodes[0].set_handle(MeshHandle::Right, Point::new(10.0, 10.0));
        m.nodes[1].set_handle(MeshHandle::Left, Point::new(20.0, 10.0));
        assert!(m.is_curved());
        let before = m.edge(0, 0, true).unwrap();
        assert!((before.eval(0.5).y - 7.5).abs() < 1e-9);
        // The other handles are straight thirds.
        let third = m.nodes[0].handle(MeshHandle::Down, m.nodes[2].pos);
        assert!(
            (third - Point::new(0.0, 10.0 / 3.0)).hypot() < 1e-9,
            "{third:?}"
        );
        // Moved and transformed, the handles come along.
        let mut moved = m.clone();
        moved.nodes[0].translate(Vec2::new(1.0, 2.0));
        assert_eq!(moved.nodes[0].handles.unwrap()[0], Point::new(11.0, 12.0));
        let scaled = m.transformed(Affine::scale(2.0));
        assert_eq!(scaled.nodes[1].handles.unwrap()[1], Point::new(40.0, 20.0));
        assert_eq!(scaled.nodes[1].pos, Point::new(60.0, 0.0));
        // A column through the middle: the curve is cut there and keeps
        // its shape; the new node sits on it.
        m.insert_column(0, 0.5);
        assert_eq!((m.rows, m.cols), (1, 2));
        let n = m.node(0, 1).unwrap();
        assert!((n.pos - before.eval(0.5)).hypot() < 1e-9, "{:?}", n.pos);
        let left = m.edge(0, 0, true).unwrap();
        let right = m.edge(0, 1, true).unwrap();
        assert!((left.eval(0.5) - before.eval(0.25)).hypot() < 1e-9);
        assert!((right.eval(0.5) - before.eval(0.75)).hypot() < 1e-9);
        // The straight bottom line stays straight.
        let bottom = m.edge(1, 0, true).unwrap();
        assert!((bottom.eval(0.5) - Point::new(7.5, 10.0)).hypot() < 1e-9);
        // The patch's corner and middle.
        let mid = m.patch_point(0, 0, 0.5, 0.5).unwrap();
        assert!(
            (mid.x - 7.5).abs() < 1e-9 && mid.y > 5.0 && mid.y < 10.0,
            "{mid:?}"
        );
        assert_eq!(m.patch_point(0, 0, 0.0, 0.0).unwrap(), Point::new(0.0, 0.0));
        // Locating goes by the curved patch: a point under the bowed top
        // line is in the cell, a point just above it (inside the straight
        // box) is not; the place found maps back to the point.
        let (r, c, u, v) = m.locate(Point::new(7.5, 9.0)).expect("inside");
        assert_eq!((r, c), (0, 0));
        let back = m.patch_point(r as u32, c as u32, u, v).unwrap();
        assert!((back - Point::new(7.5, 9.0)).hypot() < 1e-6, "{back:?}");
        assert!(m.locate(Point::new(15.0, 6.5)).is_none());
        // Removing the added column stretches the handles back: the top
        // line is the curve it was.
        let i = m.nearest(n.pos).unwrap();
        assert!(m.remove_lines_through(i));
        assert_eq!((m.rows, m.cols), (1, 1));
        let restored = m.edge(0, 0, true).unwrap();
        for k in 0..=10 {
            let t = k as f64 / 10.0;
            assert!((restored.eval(t) - before.eval(t)).hypot() < 1e-6, "{t}");
        }
    }

    #[test]
    fn meshes_regrid_and_lose_lines_keeping_their_colours() {
        let b = crate::geometry::Rect::new(0.0, 0.0, 20.0, 20.0);
        let mut m = Mesh::new(b, 1, 1, Color::WHITE);
        // Left column red, right column blue.
        for (i, n) in m.nodes.iter_mut().enumerate() {
            n.color = if i % 2 == 0 {
                Color::rgb8(255, 0, 0)
            } else {
                Color::rgb8(0, 0, 255)
            };
        }
        let g = m.regrid(2, 4);
        assert_eq!((g.rows, g.cols, g.nodes.len()), (2, 4, 15));
        // The middle column is the mix of red and blue; the ends keep theirs.
        let mid = g.node(1, 2).expect("node").color.to_rgb8();
        assert!(
            (mid[0] as i32 - 128).abs() <= 2 && (mid[2] as i32 - 128).abs() <= 2,
            "{mid:?}"
        );
        assert_eq!(g.node(0, 0).expect("node").color.to_rgb8(), [255, 0, 0]);
        assert_eq!(g.node(2, 4).expect("node").color.to_rgb8(), [0, 0, 255]);
        assert_eq!(g.node(2, 4).expect("node").pos, Point::new(20.0, 20.0));
        // The middle node's lines go; an edge node's cannot.
        let mut h = g.clone();
        let centre = 7; // row 1, column 2
        assert_eq!(h.row_col(centre), (1, 2));
        assert!(h.remove_lines_through(centre));
        assert_eq!((h.rows, h.cols, h.nodes.len()), (1, 3, 8));
        assert!(!h.remove_lines_through(0));
        assert!(!h.remove_lines_through(99));
        // Sampling between nodes mixes them.
        let s = m.sample(0.5, 0.5).expect("sample");
        assert_eq!(s.pos, Point::new(10.0, 10.0));
    }

    #[test]
    fn fountain_interpolates_stops() {
        let f = Fountain {
            kind: FountainKind::Linear,
            stops: vec![
                Stop::new(0.0, Color::rgb8(0, 0, 0)),
                Stop::new(0.5, Color::rgb8(255, 0, 0)),
                Stop::new(1.0, Color::rgb8(255, 255, 255)),
            ],
            angle: 0.0,
            offset: Point::ZERO,
            edge_pad: 0.0,
            ..Default::default()
        };
        assert_eq!(f.color_at(0.25).to_rgb8(), [128, 0, 0]);
        assert_eq!(f.color_at(0.5).to_rgb8(), [255, 0, 0]);
        assert_eq!(f.color_at(1.5).to_rgb8(), [255, 255, 255]);
    }

    /// Free nodes: a fill placed with its default nodes looks as the
    /// angle and offset place it, for every kind; a skew handle tilts
    /// the bands; the nodes follow the box (#330).
    #[test]
    fn midpoints_and_node_opacity_shape_the_blend() {
        let mut f = Fountain::two(FountainKind::Linear, Color::BLACK, Color::WHITE, 0.0);
        let grey = |c: Color| c.to_rgb8()[0];
        // In the middle by default.
        assert!((126..=129).contains(&grey(f.color_at(0.5))));
        // The midpoint at a quarter: half way there at 0.25, lighter at 0.5.
        f.stops[0].mid = 0.25;
        assert!(f.has_midpoints());
        assert!(
            (122..=134).contains(&grey(f.color_at(0.25))),
            "{:?}",
            f.color_at(0.25)
        );
        assert!(grey(f.color_at(0.5)) > 160);
        // The plain run of stops gives the same colours.
        let plain = f.resolved_stops();
        assert!(plain.len() > 2);
        let at = sample_stops(&plain, 0.25);
        assert!((i32::from(grey(at)) - i32::from(grey(f.color_at(0.25)))).abs() <= 3);
        // Opacity runs from node to node like the colour.
        f.stops[0].mid = 0.5;
        f.stops[1].opacity = 0.0;
        assert!(f.has_opacity());
        assert!((f.opacity_at(0.5) - 0.5).abs() < 0.02);
        assert_eq!(f.resolved_stops().last().map(|s| s.opacity), Some(0.0));
        // Saved without the defaults; read back from files that lack them.
        let plain_json = serde_json::to_string(&Stop::new(0.0, Color::BLACK)).unwrap();
        assert!(!plain_json.contains("mid") && !plain_json.contains("opacity"));
        let back: Stop = serde_json::from_str(&plain_json).unwrap();
        assert_eq!(back, Stop::new(0.0, Color::BLACK));
    }

    #[test]
    fn free_nodes_place_the_fill_as_the_frame_says() {
        let b = Rect::new(10.0, 20.0, 110.0, 70.0);
        for kind in [
            FountainKind::Linear,
            FountainKind::Radial,
            FountainKind::Conical,
            FountainKind::Square,
        ] {
            let mut f = Fountain::two(kind, Color::BLACK, Color::WHITE, 30.0);
            f.offset = Point::new(0.2, -0.1);
            let h = f.default_handles(b);
            let mut g = f.clone();
            g.handles = Some(h);
            for p in [
                Point::new(30.0, 30.0),
                Point::new(60.0, 45.0),
                Point::new(100.0, 65.0),
            ] {
                let t = g.param_at(b, p).expect("a frame");
                let want = legacy_param(&f, b, p);
                assert!((t - want).abs() < 1e-9, "{kind:?} at {p:?}: {t} vs {want}");
            }
        }
        // Linear from (20, 40) to (60, 40): the start is 0, the end 1.
        let mut f = Fountain::two(FountainKind::Linear, Color::BLACK, Color::WHITE, 0.0);
        let rel = |p| Fountain::page_to_rel(b, p);
        f.set_handles(
            b,
            FountainHandles {
                start: rel(Point::new(20.0, 40.0)),
                end: rel(Point::new(60.0, 40.0)),
                skew: rel(Point::new(20.0, 60.0)),
            },
        );
        assert!((f.param_at(b, Point::new(20.0, 50.0)).unwrap()).abs() < 1e-9);
        assert!((f.param_at(b, Point::new(40.0, 40.0)).unwrap() - 0.5).abs() < 1e-9);
        assert!((f.angle).abs() < 1e-9);
        // A skew handle leaning right: the bands lean with it.
        f.handles = Some(FountainHandles {
            skew: rel(Point::new(40.0, 60.0)),
            ..f.handles.unwrap()
        });
        assert!((f.param_at(b, Point::new(40.0, 60.0)).unwrap()).abs() < 1e-9);
        // The nodes follow the box: the same fill on a box twice as wide
        // has its start twice as far from the centre.
        let wide = Rect::new(10.0, 20.0, 210.0, 70.0);
        let s = Fountain::rel_to_page(wide, f.handles.unwrap().start);
        assert!((s.x - (110.0 - 2.0 * 40.0)).abs() < 1e-9, "{s:?}");
        // A flat frame places nothing.
        f.handles = Some(FountainHandles {
            start: Point::ZERO,
            end: Point::ZERO,
            skew: Point::ZERO,
        });
        assert!(f.frame(b).is_none());
    }

    /// The parameter as the renderer works it out without free nodes.
    fn legacy_param(f: &Fountain, b: Rect, p: Point) -> f64 {
        let a = f.angle.to_radians();
        let c = b.center();
        match f.kind {
            FountainKind::Linear => {
                let half = (b.width() * a.cos().abs() + b.height() * a.sin().abs()) / 2.0;
                let s = Point::new(c.x - half * a.cos(), c.y - half * a.sin());
                let d = Vec2::new(a.cos(), a.sin()) * (2.0 * half);
                (p - s).dot(d) / d.hypot2()
            }
            FountainKind::Radial => {
                let cc = Point::new(
                    c.x + f.offset.x * b.width() / 2.0,
                    c.y + f.offset.y * b.height() / 2.0,
                );
                (p - cc).hypot() / (b.width().max(b.height()) / 2.0 * std::f64::consts::SQRT_2)
            }
            _ => {
                let u = (p.x - b.x0) / b.width();
                let v = (p.y - b.y0) / b.height();
                let dx = u - 0.5 - f.offset.x / 2.0;
                let dy = v - 0.5 - f.offset.y / 2.0;
                if f.kind == FountainKind::Conical {
                    let a = (dy.atan2(dx) - a).rem_euclid(std::f64::consts::TAU)
                        / std::f64::consts::TAU;
                    if a < 0.5 {
                        a * 2.0
                    } else {
                        2.0 - a * 2.0
                    }
                } else {
                    dx.abs().max(dy.abs()) * 2.0
                }
            }
        }
    }

    fn black_white() -> Fountain {
        Fountain::two(FountainKind::Linear, Color::BLACK, Color::WHITE, 0.0)
    }

    fn grey(c: Color) -> u8 {
        c.to_rgb8()[0]
    }

    #[test]
    fn fountain_repeats_mirrors_and_inverts() {
        // Sawtooth, three cycles: dark again at the start of each.
        let mut f = black_white();
        f.repeat = FountainRepeat::Repeat;
        f.repeats = 3;
        assert_eq!(grey(f.color_at(0.0)), 0);
        assert!(grey(f.color_at(0.33)) > 240);
        assert!(grey(f.color_at(0.34)) < 15);
        assert_eq!(grey(f.color_at(1.0)), 255);
        // Triangular: there and back.
        f.repeat = FountainRepeat::Mirror;
        f.repeats = 2;
        assert_eq!(grey(f.color_at(0.5)), 255);
        assert_eq!(grey(f.color_at(1.0)), 0);
        assert_eq!(grey(f.color_at(0.25)), grey(f.color_at(0.75)));
        // No repeat: the count is ignored.
        f.repeat = FountainRepeat::None;
        assert_eq!(f.cycles(), 1);
        assert_eq!(grey(f.color_at(0.5)), 128);
        // Bilinear: first colour at both ends, last in the middle.
        let mut b = black_white();
        b.bilinear = true;
        assert_eq!(grey(b.color_at(0.0)), 0);
        assert_eq!(grey(b.color_at(0.5)), 255);
        assert_eq!(grey(b.color_at(1.0)), 0);
        // Inverted: the order reversed.
        let mut i = black_white();
        i.invert = true;
        assert_eq!(grey(i.color_at(0.0)), 255);
        assert_eq!(grey(i.color_at(1.0)), 0);
        // Out of range and not a number stay inside the stops.
        assert_eq!(grey(i.color_at(f64::NAN)), 255);
        let mut huge = black_white();
        huge.repeat = FountainRepeat::Repeat;
        huge.repeats = u32::MAX;
        assert_eq!(huge.cycles(), Fountain::MAX_REPEATS);
    }

    #[test]
    fn resolved_stops_draw_the_same_colours_in_one_run() {
        let three = Fountain {
            stops: vec![
                Stop::new(0.0, Color::rgb8(255, 0, 0)),
                Stop::new(0.4, Color::rgb8(0, 255, 0)),
                Stop::new(1.0, Color::rgb8(0, 0, 255)),
            ],
            ..Default::default()
        };
        for repeat in [
            FountainRepeat::None,
            FountainRepeat::Repeat,
            FountainRepeat::Mirror,
        ] {
            for repeats in [1, 2, 3] {
                for bilinear in [false, true] {
                    for invert in [false, true] {
                        for edge_pad in [0.0, 0.2] {
                            let f = Fountain {
                                repeat,
                                repeats,
                                bilinear,
                                invert,
                                edge_pad,
                                ..three.clone()
                            };
                            let r = f.resolved_stops();
                            assert_eq!(r[0].pos, 0.0);
                            assert!((r[r.len() - 1].pos - 1.0).abs() < 1e-12);
                            assert!(r.windows(2).all(|w| w[0].pos <= w[1].pos));
                            for k in 0..=200 {
                                // Away from the sawtooth's jumps.
                                let t = (k as f64 + 0.37) / 201.0;
                                let a = f.color_at(t).to_rgb8();
                                let b = sample_stops(&r, t).to_rgb8();
                                for c in 0..3 {
                                    assert!(
                                        (a[c] as i32 - b[c] as i32).abs() <= 2,
                                        "{f:?} at {t}: {a:?} {b:?}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        // A sawtooth's jump is a hard edge: two stops at one place.
        let mut saw = black_white();
        saw.repeat = FountainRepeat::Repeat;
        saw.repeats = 2;
        let r = saw.resolved_stops();
        assert_eq!(r.len(), 4);
        assert!((r[1].pos - 0.5).abs() < 1e-12 && (r[2].pos - 0.5).abs() < 1e-12);
        assert_eq!((grey(r[1].color), grey(r[2].color)), (255, 0));
        // A plain fountain keeps its stops.
        assert_eq!(three.resolved_stops(), three.stops);
    }

    #[test]
    fn plain_versions_carry_the_options_in_their_stops() {
        let f = black_white();
        assert_eq!(f.to_plain(64), f);
        let mut i = black_white();
        i.invert = true;
        i.edge_pad = 0.1;
        let p = i.to_plain(64);
        assert!(p.runs_once() && p.edge_pad == 0.0);
        assert_eq!(grey(p.first_color()), 255);
        // Shaped becomes square, the inside (1) at the centre (0).
        let s = Fountain::two(FountainKind::Shaped, Color::BLACK, Color::WHITE, 0.0).to_plain(64);
        assert_eq!(s.kind, FountainKind::Square);
        assert_eq!(grey(s.first_color()), 255);
        assert_eq!(grey(s.last_color()), 0);
        // Too many stops: sampled down evenly.
        let mut many = black_white();
        many.repeat = FountainRepeat::Repeat;
        many.repeats = 50;
        assert_eq!(many.to_plain(16).stops.len(), 16);
    }

    #[test]
    fn fountains_without_the_options_read_and_write_as_before() {
        // As files written before the options had it.
        let old = r#"{"kind":"radial","stops":[{"pos":0.0,"color":{"model":"rgb","r":0.0,"g":0.0,"b":0.0}},{"pos":1.0,"color":{"model":"gray","v":1.0}}],"angle":0.0,"offset":{"x":0.0,"y":0.0},"edge_pad":0.1}"#;
        let f: Fountain = serde_json::from_str(old).expect("old fountain reads");
        assert_eq!(f.kind, FountainKind::Radial);
        assert!(f.runs_once());
        assert_eq!((f.repeat, f.repeats), (FountainRepeat::None, 1));
        // Defaults are not written, so files stay as they were.
        let json = serde_json::to_string(&black_white()).expect("json");
        for field in ["repeat", "bilinear", "invert"] {
            assert!(!json.contains(field), "{json}");
        }
        let mut g = black_white();
        g.kind = FountainKind::Shaped;
        g.repeat = FountainRepeat::Mirror;
        g.repeats = 4;
        g.bilinear = true;
        g.invert = true;
        let json = serde_json::to_string(&g).expect("json");
        assert!(
            json.contains("\"shaped\"") && json.contains("\"mirror\""),
            "{json}"
        );
        let back: Fountain = serde_json::from_str(&json).expect("reads");
        assert_eq!(back, g);
    }

    #[test]
    fn pattern_tiles_have_both_colours() {
        for t in PatternTile::ALL {
            let mut front = 0;
            for i in 0..20 {
                for j in 0..20 {
                    if t.front(i as f64 / 20.0, j as f64 / 20.0) {
                        front += 1;
                    }
                }
            }
            assert!(front > 0 && front < 400, "{t:?}");
        }
    }

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> crate::document::Shape {
        let mut s = crate::document::Shape::new(
            crate::ShapeId(1),
            crate::document::ShapeKind::Rect {
                rect: Rect::new(x0, y0, x1, y1),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        s.stroke = None;
        s
    }

    #[test]
    fn vector_pattern_from_shapes_moves_bounds_to_origin() {
        let shapes = vec![
            square(30.0, 40.0, 35.0, 45.0),
            square(35.0, 45.0, 40.0, 50.0),
        ];
        let Pattern::Vector { shapes, tile } = Pattern::vector_from_shapes(&shapes) else {
            panic!("expected a vector pattern");
        };
        assert_eq!(tile, Size::new(10.0, 10.0));
        assert_eq!(shapes.len(), 2);
        let b = shapes[0].bounds();
        assert!((b.x0).abs() < 1e-9 && (b.y0).abs() < 1e-9, "{b:?}");
        assert!(
            (b.x1 - 5.0).abs() < 1e-9 && (b.y1 - 5.0).abs() < 1e-9,
            "{b:?}"
        );
        let b = shapes[1].bounds();
        assert!(
            (b.x1 - 10.0).abs() < 1e-9 && (b.y1 - 10.0).abs() < 1e-9,
            "{b:?}"
        );
    }

    #[test]
    fn vector_pattern_from_nothing_has_a_minimal_tile() {
        let Pattern::Vector { shapes, tile } = Pattern::vector_from_shapes(&[]) else {
            panic!("expected a vector pattern");
        };
        assert!(shapes.is_empty());
        assert!(tile.width > 0.0 && tile.height > 0.0);
    }

    #[test]
    fn vector_pattern_round_trips_through_json() {
        let fill = Fill::Pattern(Pattern::vector_from_shapes(&[square(0.0, 0.0, 5.0, 5.0)]));
        let json = serde_json::to_string(&fill).unwrap();
        assert!(json.contains("\"pattern\":\"vector\""));
        let back: Fill = serde_json::from_str(&json).unwrap();
        assert_eq!(back, fill);
    }

    #[test]
    fn older_pattern_fills_still_load() {
        let json = r#"{"fill":"pattern","pattern":"twocolor","tile":"dots","front":{"model":"gray","v":0.0},"back":{"model":"gray","v":1.0},"size_mm":5.0}"#;
        let back: Fill = serde_json::from_str(json).unwrap();
        assert!(matches!(
            back,
            Fill::Pattern(Pattern::TwoColor {
                tile: PatternTile::Dots,
                ..
            })
        ));
        // A vector tile without shapes is still a valid (empty) tile.
        let json = r#"{"fill":"pattern","pattern":"vector","tile":{"width":4.0,"height":2.0}}"#;
        let back: Fill = serde_json::from_str(json).unwrap();
        assert!(
            matches!(back, Fill::Pattern(Pattern::Vector { ref shapes, .. }) if shapes.is_empty())
        );
    }
}

/// Arrowhead outlines (page space) for an open path's ends, sized by the
/// outline width (hairlines use 0.25 mm). Filled with the outline colour.
pub fn arrowhead_paths(
    path: &crate::geometry::BezPath,
    stroke: &Stroke,
) -> Vec<crate::geometry::BezPath> {
    use crate::geometry::{BezPath, PathEl, Vec2};
    let w = if stroke.width <= Stroke::HAIRLINE + 1e-9 {
        0.25
    } else {
        stroke.width
    };
    let size = (w * HEAD_SIZE_PER_WIDTH).max(1.0);
    let mut pts: Vec<Point> = Vec::new();
    let mut closed = false;
    kurbo::flatten(path.elements().iter().copied(), 0.05, &mut |el| match el {
        PathEl::MoveTo(p) => {
            if pts.is_empty() {
                pts.push(p)
            }
        }
        PathEl::LineTo(p) => pts.push(p),
        PathEl::ClosePath => closed = true,
        _ => {}
    });
    if closed || pts.len() < 2 {
        return Vec::new();
    }
    let head = |tip: Point, dir: Vec2, kind: &Arrowhead| -> Option<BezPath> {
        let dir = dir.normalize();
        if !dir.x.is_finite() {
            return None;
        }
        let n = Vec2::new(-dir.y, dir.x);
        let mut p = BezPath::new();
        match kind {
            Arrowhead::None => return None,
            Arrowhead::Custom { path: outline, .. } => {
                // Head space (unit tall, tip at the origin, body towards -x)
                // to page space: x along the curve direction, y along its
                // left normal, scaled to the preset size.
                let place =
                    Affine::new([dir.x, dir.y, n.x, n.y, tip.x, tip.y]) * Affine::scale(size);
                p = place * outline.clone();
            }
            Arrowhead::Arrow => {
                p.move_to(tip);
                p.line_to(tip - dir * size + n * (size * 0.4));
                p.line_to(tip - dir * size * 0.7);
                p.line_to(tip - dir * size - n * (size * 0.4));
                p.close_path();
            }
            Arrowhead::OpenArrow => {
                let t = w.max(0.2);
                p.move_to(tip);
                p.line_to(tip - dir * size + n * (size * 0.45));
                p.line_to(tip - dir * size + n * (size * 0.45) - dir * 0.0 + n * (-t) - dir * t);
                p.line_to(tip - dir * (t * 1.4));
                p.line_to(tip - dir * size - n * (size * 0.45) + n * t - dir * t);
                p.line_to(tip - dir * size - n * (size * 0.45));
                p.close_path();
            }
            Arrowhead::Circle => {
                let c = tip - dir * (size * 0.35);
                p = crate::geometry::Circle::new(c, size * 0.35).to_path(0.01);
            }
            Arrowhead::Square => {
                let c = tip - dir * (size * 0.35);
                let h = size * 0.35;
                p.move_to(c + dir * h + n * h);
                p.line_to(c - dir * h + n * h);
                p.line_to(c - dir * h - n * h);
                p.line_to(c + dir * h - n * h);
                p.close_path();
            }
            Arrowhead::Bar => {
                let t = w.max(0.2);
                p.move_to(tip + n * (size * 0.5));
                p.line_to(tip - dir * t + n * (size * 0.5));
                p.line_to(tip - dir * t - n * (size * 0.5));
                p.line_to(tip - n * (size * 0.5));
                p.close_path();
            }
            Arrowhead::Diamond => {
                let h = size * 0.5;
                p.move_to(tip);
                p.line_to(tip - dir * h + n * (h * 0.6));
                p.line_to(tip - dir * h * 2.0);
                p.line_to(tip - dir * h - n * (h * 0.6));
                p.close_path();
            }
        }
        Some(p)
    };
    use crate::geometry::Shape as _;
    let mut out = Vec::new();
    let n = pts.len();
    if let Some(p) = head(pts[0], pts[0] - pts[1.min(n - 1)], &stroke.start_arrow) {
        out.push(p);
    }
    if let Some(p) = head(pts[n - 1], pts[n - 1] - pts[n - 2], &stroke.end_arrow) {
        out.push(p);
    }
    out
}

#[cfg(test)]
mod arrowhead_tests {
    use super::*;
    use crate::geometry::{BezPath, Shape as _};

    /// A triangle pointing right: tip at (30, 15), base from (10, 5) to (10, 25).
    fn triangle() -> BezPath {
        let mut p = BezPath::new();
        p.move_to((10.0, 5.0));
        p.line_to((30.0, 15.0));
        p.line_to((10.0, 25.0));
        p.close_path();
        p
    }

    #[test]
    fn custom_head_is_normalised_to_head_space() {
        let head = Arrowhead::from_shape_path(&triangle(), "Tri");
        let Arrowhead::Custom { path, name } = &head else {
            panic!("expected a custom head");
        };
        assert_eq!(name, "Tri");
        assert!(head.is_custom());
        assert_eq!(head.name(), "Tri");
        let b = path.bounding_box();
        assert!((b.height() - 1.0).abs() < 1e-9, "{b:?}");
        assert!(
            (b.y0 + 0.5).abs() < 1e-9 && (b.y1 - 0.5).abs() < 1e-9,
            "{b:?}"
        );
        assert!(b.x1.abs() < 1e-9, "tip must be at x = 0: {b:?}");
        assert!((b.x0 + 1.0).abs() < 1e-9, "20 wide over 20 tall: {b:?}");
    }

    #[test]
    fn placed_head_keeps_its_size_and_place() {
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((50.0, 0.0));
        // 20 wide and 20 tall, tip at (10, 0): with 10 units per outline
        // width it is two widths tall and reaches one width past the end
        // point.
        let mut placed = BezPath::new();
        placed.move_to((-10.0, -10.0));
        placed.line_to((10.0, 0.0));
        placed.line_to((-10.0, 10.0));
        placed.close_path();
        let head = Arrowhead::from_placed_path(&placed, "Tri", 10.0);
        assert_eq!(head.name(), "Tri");
        for width in [1.0, 0.5, 2.0] {
            let stroke = Stroke {
                end_arrow: head.clone(),
                ..Stroke::new(Color::BLACK, width)
            };
            let heads = arrowhead_paths(&line, &stroke);
            assert_eq!(heads.len(), 1);
            let b = heads[0].bounding_box();
            assert!((b.height() - 2.0 * width).abs() < 1e-9, "{width}: {b:?}");
            assert!((b.x1 - (50.0 + width)).abs() < 1e-9, "{width}: {b:?}");
            assert!((b.y0 + width).abs() < 1e-9, "{width}: {b:?}");
        }
        // Huge heads are capped at 100 outline widths.
        let Arrowhead::Custom { path, .. } = Arrowhead::from_placed_path(&triangle(), "Tri", 1e-6)
        else {
            panic!("expected a custom head");
        };
        assert!((path.bounding_box().height() - 100.0 / HEAD_SIZE_PER_WIDTH).abs() < 1e-9);
        for bad in [0.0, -3.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                Arrowhead::from_placed_path(&triangle(), "Tri", bad),
                Arrowhead::None
            );
        }
        assert_eq!(
            Arrowhead::from_placed_path(&BezPath::new(), "empty", 5.0),
            Arrowhead::None
        );
    }

    #[test]
    fn degenerate_outlines_give_no_head() {
        assert_eq!(
            Arrowhead::from_shape_path(&BezPath::new(), "empty"),
            Arrowhead::None
        );
        let mut dot = BezPath::new();
        dot.move_to((3.0, 3.0));
        dot.line_to((3.0, 3.0));
        dot.close_path();
        assert_eq!(Arrowhead::from_shape_path(&dot, "dot"), Arrowhead::None);
        // A flat bar keeps a height taken from its width.
        let mut bar = BezPath::new();
        bar.move_to((0.0, 1.0));
        bar.line_to((4.0, 1.0));
        bar.close_path();
        assert!(Arrowhead::from_shape_path(&bar, "bar").is_custom());
    }

    #[test]
    fn presets_are_not_custom_and_all_lists_only_presets() {
        for a in Arrowhead::ALL.iter() {
            assert!(!a.is_custom(), "{a:?}");
        }
        assert_eq!(Arrowhead::ALL.len(), 7);
        assert_eq!(Arrowhead::Arrow.name(), "Arrow");
    }

    #[test]
    fn custom_head_round_trips_through_json_and_presets_stay_strings() {
        let stroke = Stroke {
            start_arrow: Arrowhead::Diamond,
            end_arrow: Arrowhead::from_shape_path(&triangle(), "Tri"),
            ..Stroke::new(Color::BLACK, 1.0)
        };
        let json = serde_json::to_string(&stroke).unwrap();
        assert!(json.contains("\"start_arrow\":\"diamond\""));
        assert!(json.contains("\"end_arrow\":{\"custom\":"));
        let back: Stroke = serde_json::from_str(&json).unwrap();
        assert_eq!(back, stroke);
        // Files written before custom heads existed still load.
        let old = r#"{"color":{"model":"gray","v":0.0},"width":1.0,"cap":"butt","join":"miter","dash":[],"scale_with_object":false,"behind_fill":false,"start_arrow":"none","end_arrow":"arrow","stretch":1.0,"nib_angle":0.0}"#;
        let back: Stroke = serde_json::from_str(old).unwrap();
        assert_eq!(back.end_arrow, Arrowhead::Arrow);
        // ... and keep the mitre limit and position they were drawn with.
        assert!((back.miter_ratio() - 4.0).abs() < 1e-9);
        assert_eq!(back.position, OutlinePosition::Center);
        assert!(!back.align_dashes);
    }

    #[test]
    fn position_mitre_limit_and_dash_alignment_round_trip() {
        let stroke = Stroke {
            position: OutlinePosition::Inside,
            miter_limit: 12.5,
            align_dashes: true,
            dash: vec![4.0, 2.0],
            ..Stroke::new(Color::BLACK, 1.0)
        };
        let json = serde_json::to_string(&stroke).unwrap();
        assert!(json.contains("\"position\":\"inside\""), "{json}");
        let back: Stroke = serde_json::from_str(&json).unwrap();
        assert_eq!(back, stroke);
        // A centred outline without alignment does not write the fields.
        let plain = serde_json::to_string(&Stroke::new(Color::BLACK, 1.0)).unwrap();
        assert!(!plain.contains("position") && !plain.contains("align_dashes"));
    }

    #[test]
    fn mitre_limit_angles_and_ratios_convert_both_ways() {
        let s = Stroke::new(Color::BLACK, 1.0);
        assert_eq!(s.miter_limit, 45.0);
        // 1 / sin(22.5 degrees)
        assert!(
            (s.miter_ratio() - 2.6131).abs() < 1e-3,
            "{}",
            s.miter_ratio()
        );
        for r in [1.5, 4.0, 10.0] {
            let a = Stroke::miter_angle_from_ratio(r);
            let back = Stroke {
                miter_limit: a,
                ..s.clone()
            };
            assert!((back.miter_ratio() - r).abs() < 1e-9, "{r}");
        }
        for bad in [f64::NAN, 0.5, f64::INFINITY] {
            assert_eq!(Stroke::miter_angle_from_ratio(bad), 45.0);
        }
        let odd = Stroke {
            miter_limit: f64::NAN,
            ..s
        };
        assert!(odd.miter_ratio().is_finite());
    }

    #[test]
    fn custom_head_is_placed_at_the_line_end_pointing_forward() {
        // A horizontal line from (0, 0) to (50, 0) with a custom end head.
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((50.0, 0.0));
        let stroke = Stroke {
            end_arrow: Arrowhead::from_shape_path(&triangle(), "Tri"),
            ..Stroke::new(Color::BLACK, 1.0)
        };
        let heads = arrowhead_paths(&line, &stroke);
        assert_eq!(heads.len(), 1);
        let b = heads[0].bounding_box();
        // Size is four widths = 4 mm: the head is 4 mm tall, centred on the
        // line, with its tip at the end point and its body behind it.
        assert!((b.x1 - 50.0).abs() < 1e-6, "{b:?}");
        assert!((b.x0 - 46.0).abs() < 1e-6, "{b:?}");
        assert!(
            (b.y0 + 2.0).abs() < 1e-6 && (b.y1 - 2.0).abs() < 1e-6,
            "{b:?}"
        );

        // The same head at the start points the other way.
        let stroke = Stroke {
            start_arrow: stroke.end_arrow.clone(),
            end_arrow: Arrowhead::None,
            ..Stroke::new(Color::BLACK, 1.0)
        };
        let heads = arrowhead_paths(&line, &stroke);
        assert_eq!(heads.len(), 1);
        let b = heads[0].bounding_box();
        assert!(b.x0.abs() < 1e-6 && (b.x1 - 4.0).abs() < 1e-6, "{b:?}");
    }
}
