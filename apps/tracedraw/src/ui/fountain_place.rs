//! Placement of a fountain fill in numbers (#1200): where its nodes sit on
//! the object's box, as the Interactive Fill tool drags them, shown and
//! set as W, H, X, Y, skew and rotation fields in the Properties docker.
//!
//! The nodes (`Fountain::handles`, relative to the box) span an axis from
//! the start to the end node and a side from the start to the skew node.
//! W and H are the lengths of the axis and the side as percentages of
//! their lengths in the default placement at the same rotation (so a
//! fresh fill reads 100 % and 100 % whatever the box's shape, and a
//! rotation typed in keeps the fill reaching across the box as the
//! default does); rotation is the axis's angle; skew is how far the side
//! leans from the axis's perpendicular; X and Y place the origin (the
//! middle of the axis for a linear fill, the start node for the others)
//! from the box's centre, as a percentage of the box's half width and
//! half height.

use crate::i18n::tr;
use crate::ui::tips::RichTip;
use egui::Ui;
use tracedraw_core::geometry::{Point, Rect, Vec2};
use tracedraw_core::style::FountainHandles;
use tracedraw_core::{Fountain, FountainKind};

/// The placement in the docker's numbers: percentages and degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub w: f64,
    pub h: f64,
    pub rotate: f64,
    pub skew: f64,
    pub x: f64,
    pub y: f64,
}

/// An angle in degrees brought to -180..180.
fn wrap(deg: f64) -> f64 {
    let d = (deg + 180.0).rem_euclid(360.0) - 180.0;
    if d <= -180.0 {
        d + 360.0
    } else {
        d
    }
}

/// The page lengths of the default axis and side of `f` on `b` turned to
/// `rotate` degrees: what 100 % of W and H mean.
fn reference(f: &Fountain, b: Rect, rotate: f64) -> (f64, f64) {
    let mut g = f.clone();
    g.angle = rotate;
    g.handles = None;
    let d = g.default_handles(b);
    let s = Fountain::rel_to_page(b, d.start);
    let axis = (Fountain::rel_to_page(b, d.end) - s).hypot();
    let side = (Fountain::rel_to_page(b, d.skew) - s).hypot();
    (axis.max(1e-9), side.max(1e-9))
}

/// Whether the fill's origin is the middle of the axis (linear) or its
/// start node (the kinds with a centre).
fn origin_is_middle(kind: FountainKind) -> bool {
    matches!(kind, FountainKind::Linear | FountainKind::Shaped)
}

/// The placement of `f` on the box `b`: of its nodes, or of the default
/// ones while it has none.
pub fn placement_of(f: &Fountain, b: Rect) -> Placement {
    let h = f.handles.unwrap_or_else(|| f.default_handles(b));
    let s = Fountain::rel_to_page(b, h.start);
    let e = Fountain::rel_to_page(b, h.end);
    let k = Fountain::rel_to_page(b, h.skew);
    let axis = e - s;
    let side = k - s;
    let rotate = if axis.hypot() > 1e-9 {
        axis.y.atan2(axis.x).to_degrees()
    } else {
        f.angle
    };
    let side_angle = if side.hypot() > 1e-9 {
        side.y.atan2(side.x).to_degrees()
    } else {
        rotate + 90.0
    };
    let (la, ls) = reference(f, b, rotate);
    let origin = if origin_is_middle(f.kind) {
        h.start.midpoint(h.end)
    } else {
        h.start
    };
    Placement {
        w: axis.hypot() / la * 100.0,
        h: side.hypot() / ls * 100.0,
        rotate: wrap(rotate),
        skew: wrap(side_angle - rotate - 90.0),
        x: origin.x * 100.0,
        y: origin.y * 100.0,
    }
}

/// The nodes that put `f` on the box `b` as `p` says.
pub fn handles_of(f: &Fountain, b: Rect, p: &Placement) -> FountainHandles {
    let (la, ls) = reference(f, b, p.rotate);
    let r = p.rotate.to_radians();
    let axis = Vec2::new(r.cos(), r.sin()) * (la * p.w / 100.0);
    let sr = (p.rotate + 90.0 + p.skew).to_radians();
    let side = Vec2::new(sr.cos(), sr.sin()) * (ls * p.h / 100.0);
    let origin = Fountain::rel_to_page(b, Point::new(p.x / 100.0, p.y / 100.0));
    let start = if origin_is_middle(f.kind) {
        origin - axis / 2.0
    } else {
        origin
    };
    FountainHandles {
        start: Fountain::page_to_rel(b, start),
        end: Fountain::page_to_rel(b, start + axis),
        skew: Fountain::page_to_rel(b, start + side),
    }
}

/// The fields; a change places the fill's nodes. Nothing for a shaped
/// fill, which follows the outline. Lock keeps W and H in proportion.
pub fn placement_editor(ui: &mut Ui, f: &mut Fountain, b: Rect) -> bool {
    if f.kind == FountainKind::Shaped || !(b.width() > 1e-9 && b.height() > 1e-9) {
        return false;
    }
    let before = placement_of(f, b);
    let mut p = before;
    let lock_id = ui.make_persistent_id("fountain_place_lock");
    let mut lock = ui.data_mut(|d| d.get_temp::<bool>(lock_id).unwrap_or(true));
    ui.label(tr("docker.fill_placement"))
        .rich_tip(tr("docker.fill_placement_tip"));
    let field = |ui: &mut Ui,
                 key: &str,
                 v: &mut f64,
                 range: std::ops::RangeInclusive<f64>,
                 suffix: &str,
                 speed: f64|
     -> bool {
        ui.label(tr(key));
        ui.add(
            crate::ui::field::NumField::new(v)
                .range(range)
                .speed(speed)
                .max_decimals(1)
                .suffix(suffix)
                .width(64.0),
        )
        .rich_tip(tr("docker.fill_placement_tip"))
        .changed()
    };
    let mut changed = false;
    ui.horizontal(|ui| {
        let w = field(ui, "docker.fill_w", &mut p.w, 1.0..=1000.0, " %", 1.0);
        let h = field(ui, "docker.fill_h", &mut p.h, 1.0..=1000.0, " %", 1.0);
        if ui
            .selectable_label(lock, "\u{1F512}")
            .rich_tip(tr("docker.fill_lock_ratio"))
            .clicked()
        {
            lock = !lock;
            ui.data_mut(|d| d.insert_temp(lock_id, lock));
        }
        if lock && before.w > 0.0 && before.h > 0.0 {
            if w {
                p.h = (before.h * p.w / before.w).clamp(1.0, 1000.0);
            } else if h {
                p.w = (before.w * p.h / before.h).clamp(1.0, 1000.0);
            }
        }
        changed |= w || h;
    });
    ui.horizontal(|ui| {
        changed |= field(ui, "docker.fill_x", &mut p.x, -500.0..=500.0, " %", 1.0);
        changed |= field(ui, "docker.fill_y", &mut p.y, -500.0..=500.0, " %", 1.0);
    });
    ui.horizontal(|ui| {
        changed |= field(ui, "docker.fill_skew", &mut p.skew, -89.0..=89.0, "°", 1.0);
        changed |= field(
            ui,
            "docker.fill_rotate",
            &mut p.rotate,
            -180.0..=180.0,
            "°",
            1.0,
        );
    });
    if changed {
        let h = handles_of(f, b, &p);
        f.set_handles(b, h);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{Color, Stop};

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn fountain(kind: FountainKind) -> Fountain {
        Fountain {
            kind,
            stops: vec![Stop::new(0.0, Color::BLACK), Stop::new(1.0, Color::WHITE)],
            angle: 0.0,
            offset: Point::ZERO,
            edge_pad: 0.0,
            repeat: Default::default(),
            repeats: 1,
            bilinear: false,
            invert: false,
            handles: None,
        }
    }

    /// A fill without nodes reads as the default placement: 100 % by
    /// 100 %, no turn, no skew, the origin at the centre; and the nodes
    /// built from that placement are the default ones.
    #[test]
    fn a_fresh_fill_reads_as_the_default_placement() {
        let b = Rect::new(10.0, 10.0, 60.0, 40.0);
        for kind in [
            FountainKind::Linear,
            FountainKind::Radial,
            FountainKind::Conical,
            FountainKind::Square,
        ] {
            let f = fountain(kind);
            let p = placement_of(&f, b);
            assert!(close(p.w, 100.0) && close(p.h, 100.0), "{kind:?}: {p:?}");
            assert!(
                close(p.rotate, 0.0) && close(p.skew, 0.0),
                "{kind:?}: {p:?}"
            );
            assert!(close(p.x, 0.0) && close(p.y, 0.0), "{kind:?}: {p:?}");
            let d = f.default_handles(b);
            let h = handles_of(&f, b, &p);
            for (a, c) in [(h.start, d.start), (h.end, d.end), (h.skew, d.skew)] {
                assert!(
                    close(a.x, c.x) && close(a.y, c.y),
                    "{kind:?}: {h:?} vs {d:?}"
                );
            }
        }
    }

    /// A placement typed in comes back as typed, and places the nodes:
    /// half the width at 30 degrees, leaning 20 degrees, from a moved
    /// origin.
    #[test]
    fn a_placement_round_trips_through_the_nodes() {
        let b = Rect::new(0.0, 0.0, 80.0, 20.0);
        for kind in [FountainKind::Linear, FountainKind::Radial] {
            let mut f = fountain(kind);
            let p = Placement {
                w: 50.0,
                h: 120.0,
                rotate: 30.0,
                skew: 20.0,
                x: 25.0,
                y: -10.0,
            };
            let h = handles_of(&f, b, &p);
            f.set_handles(b, h);
            let back = placement_of(&f, b);
            for (a, c) in [
                (back.w, p.w),
                (back.h, p.h),
                (back.rotate, p.rotate),
                (back.skew, p.skew),
                (back.x, p.x),
                (back.y, p.y),
            ] {
                assert!(close(a, c), "{kind:?}: {back:?} vs {p:?}");
            }
            // The axis really is half the default's, turned by 30 degrees.
            let s = Fountain::rel_to_page(b, h.start);
            let e = Fountain::rel_to_page(b, h.end);
            let (la, _) = reference(&f, b, 30.0);
            assert!(close((e - s).hypot(), la / 2.0), "{kind:?}");
            assert!(close((e - s).y.atan2((e - s).x).to_degrees(), 30.0));
            // The fill's angle follows, for formats that know only it.
            assert!(close(f.angle, 30.0), "{}", f.angle);
            if kind == FountainKind::Radial {
                // The origin is the start node, kept as the offset too.
                assert!(close(h.start.x, 0.25) && close(h.start.y, -0.1));
                assert!(close(f.offset.x, 0.25) && close(f.offset.y, -0.1));
            }
        }
    }

    #[test]
    fn angles_wrap_to_a_half_turn_each_way() {
        assert!(close(wrap(190.0), -170.0));
        assert!(close(wrap(-190.0), 170.0));
        assert!(close(wrap(180.0), 180.0));
        assert!(close(wrap(0.0), 0.0));
    }
}
