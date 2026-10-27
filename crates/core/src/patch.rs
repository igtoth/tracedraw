//! Changing one attribute of several objects at once.
//!
//! A selection whose objects have different outlines or fills keeps those
//! differences when one attribute is edited: only the attribute that was
//! changed is copied to each object, the rest stays each object's own.
//! The editors compare the value they showed with the edited one
//! ([`stroke_changes`], [`fill_changes`]) and send the fields that differ
//! with [`crate::Command::PatchStroke`] or [`crate::Command::PatchFill`].
//! The same field lists tell an editor which values differ across a
//! selection ([`stroke_mixed`], [`fill_mixed`]), so it can show them as
//! mixed.

use crate::style::{Fill, Fountain, Stroke};
use serde::{Deserialize, Serialize};

/// One attribute of an outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrokeField {
    /// The colour, or the outline's own fill (gradient, pattern, texture).
    Paint,
    Width,
    Cap,
    Join,
    Dash,
    ScaleWithObject,
    BehindFill,
    StartArrow,
    EndArrow,
    /// Calligraphic nib stretch.
    Stretch,
    /// Calligraphic nib angle.
    NibAngle,
    /// Width nodes of a variable-width outline.
    Widths,
    /// Painterly brush marks.
    Painterly,
    /// Where the outline sits: outside, centred on or inside the path.
    Position,
    /// The mitre limit angle.
    MiterLimit,
    /// Dashes fitted to corners and ends.
    AlignDashes,
}

impl StrokeField {
    pub const ALL: [StrokeField; 16] = [
        StrokeField::Paint,
        StrokeField::Width,
        StrokeField::Cap,
        StrokeField::Join,
        StrokeField::Dash,
        StrokeField::ScaleWithObject,
        StrokeField::BehindFill,
        StrokeField::StartArrow,
        StrokeField::EndArrow,
        StrokeField::Stretch,
        StrokeField::NibAngle,
        StrokeField::Widths,
        StrokeField::Painterly,
        StrokeField::Position,
        StrokeField::MiterLimit,
        StrokeField::AlignDashes,
    ];

    /// True when `a` and `b` have the same value for this field.
    pub fn same(self, a: &Stroke, b: &Stroke) -> bool {
        match self {
            StrokeField::Paint => a.color == b.color && a.fill == b.fill,
            StrokeField::Width => a.width == b.width,
            StrokeField::Cap => a.cap == b.cap,
            StrokeField::Join => a.join == b.join,
            StrokeField::Dash => a.dash == b.dash,
            StrokeField::ScaleWithObject => a.scale_with_object == b.scale_with_object,
            StrokeField::BehindFill => a.behind_fill == b.behind_fill,
            StrokeField::StartArrow => a.start_arrow == b.start_arrow,
            StrokeField::EndArrow => a.end_arrow == b.end_arrow,
            StrokeField::Stretch => a.stretch == b.stretch,
            StrokeField::NibAngle => a.nib_angle == b.nib_angle,
            StrokeField::Widths => a.widths == b.widths,
            StrokeField::Painterly => a.painterly == b.painterly,
            StrokeField::Position => a.position == b.position,
            StrokeField::MiterLimit => a.miter_limit == b.miter_limit,
            StrokeField::AlignDashes => a.align_dashes == b.align_dashes,
        }
    }

    /// Copy this field of `from` into `to`, leaving the others alone.
    pub fn copy(self, from: &Stroke, to: &mut Stroke) {
        match self {
            StrokeField::Paint => {
                to.color = from.color;
                to.fill = from.fill.clone();
            }
            StrokeField::Width => to.width = from.width,
            StrokeField::Cap => to.cap = from.cap,
            StrokeField::Join => to.join = from.join,
            StrokeField::Dash => to.dash = from.dash.clone(),
            StrokeField::ScaleWithObject => to.scale_with_object = from.scale_with_object,
            StrokeField::BehindFill => to.behind_fill = from.behind_fill,
            StrokeField::StartArrow => to.start_arrow = from.start_arrow.clone(),
            StrokeField::EndArrow => to.end_arrow = from.end_arrow.clone(),
            StrokeField::Stretch => to.stretch = from.stretch,
            StrokeField::NibAngle => to.nib_angle = from.nib_angle,
            StrokeField::Widths => to.widths = from.widths.clone(),
            StrokeField::Painterly => to.painterly = from.painterly,
            StrokeField::Position => to.position = from.position,
            StrokeField::MiterLimit => to.miter_limit = from.miter_limit,
            StrokeField::AlignDashes => to.align_dashes = from.align_dashes,
        }
    }
}

/// The fields in which `new` differs from `old`.
pub fn stroke_changes(old: &Stroke, new: &Stroke) -> Vec<StrokeField> {
    StrokeField::ALL
        .into_iter()
        .filter(|f| !f.same(old, new))
        .collect()
}

/// `target` with `fields` taken from `from`. An object without an outline
/// gets the default outline with those fields, so typing a width into a
/// selection where some objects have no outline gives them one.
pub fn patch_stroke(target: Option<&Stroke>, from: &Stroke, fields: &[StrokeField]) -> Stroke {
    let mut s = target.cloned().unwrap_or_default();
    for f in fields {
        f.copy(from, &mut s);
    }
    s
}

/// How the outlines of a selection differ.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StrokeMix {
    /// Some objects have an outline and some have none.
    pub presence: bool,
    /// The fields whose values differ among the objects that have an
    /// outline.
    pub fields: Vec<StrokeField>,
}

impl StrokeMix {
    pub fn has(&self, f: StrokeField) -> bool {
        self.fields.contains(&f)
    }

    pub fn is_empty(&self) -> bool {
        !self.presence && self.fields.is_empty()
    }
}

/// Which outline values differ across `strokes` (one per object).
pub fn stroke_mixed<'a>(strokes: impl IntoIterator<Item = Option<&'a Stroke>>) -> StrokeMix {
    let mut mix = StrokeMix::default();
    let mut first: Option<&Stroke> = None;
    let mut seen_none = false;
    let mut seen_some = false;
    for s in strokes {
        match s {
            None => seen_none = true,
            Some(s) => {
                seen_some = true;
                match first {
                    None => first = Some(s),
                    Some(f) => {
                        for field in StrokeField::ALL {
                            if !field.same(f, s) && !mix.fields.contains(&field) {
                                mix.fields.push(field);
                            }
                        }
                    }
                }
            }
        }
    }
    mix.presence = seen_none && seen_some;
    mix
}

/// One attribute of a fill. A change of fill type, of a uniform colour,
/// or of a pattern, texture or mesh replaces the whole fill; a gradient's
/// settings can be changed one at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillField {
    /// The whole fill (its type and everything in it).
    All,
    FountainKind,
    /// The colour of the first stop.
    StartColor,
    /// The colour of the last stop.
    EndColor,
    /// All the stops (positions and colours).
    Stops,
    Angle,
    Offset,
    EdgePad,
    /// Repeat mode and count.
    Repeat,
    Bilinear,
    Invert,
    /// Where the nodes are placed freely (see `Fountain::handles`), or
    /// none: the angle and the offset place the fill.
    Handles,
}

impl FillField {
    /// The gradient fields, in the order an editor shows them.
    pub const FOUNTAIN: [FillField; 11] = [
        FillField::FountainKind,
        FillField::StartColor,
        FillField::EndColor,
        FillField::Stops,
        FillField::Angle,
        FillField::Offset,
        FillField::EdgePad,
        FillField::Repeat,
        FillField::Bilinear,
        FillField::Invert,
        FillField::Handles,
    ];

    fn same_fountain(self, a: &Fountain, b: &Fountain) -> bool {
        match self {
            FillField::All => a == b,
            FillField::FountainKind => a.kind == b.kind,
            FillField::StartColor => {
                a.stops.first().map(|s| s.color) == b.stops.first().map(|s| s.color)
            }
            FillField::EndColor => {
                a.stops.last().map(|s| s.color) == b.stops.last().map(|s| s.color)
            }
            FillField::Stops => a.stops == b.stops,
            FillField::Angle => a.angle == b.angle,
            FillField::Offset => a.offset == b.offset,
            FillField::EdgePad => a.edge_pad == b.edge_pad,
            FillField::Repeat => a.repeat == b.repeat && a.repeats == b.repeats,
            FillField::Bilinear => a.bilinear == b.bilinear,
            FillField::Invert => a.invert == b.invert,
            FillField::Handles => a.handles == b.handles,
        }
    }

    fn copy_fountain(self, from: &Fountain, to: &mut Fountain) {
        match self {
            FillField::All => *to = from.clone(),
            FillField::FountainKind => to.kind = from.kind,
            FillField::StartColor => {
                if let (Some(f), Some(t)) = (from.stops.first(), to.stops.first_mut()) {
                    t.color = f.color;
                }
            }
            FillField::EndColor => {
                if let (Some(f), Some(t)) = (from.stops.last(), to.stops.last_mut()) {
                    t.color = f.color;
                }
            }
            FillField::Stops => to.stops = from.stops.clone(),
            FillField::Angle => to.angle = from.angle,
            FillField::Offset => to.offset = from.offset,
            FillField::EdgePad => to.edge_pad = from.edge_pad,
            FillField::Repeat => {
                to.repeat = from.repeat;
                to.repeats = from.repeats;
            }
            FillField::Bilinear => to.bilinear = from.bilinear,
            FillField::Invert => to.invert = from.invert,
            FillField::Handles => to.handles = from.handles,
        }
    }
}

/// The type of a fill: 0 none, 1 uniform, 2 fountain, 3 pattern,
/// 4 texture, 5 mesh.
pub fn fill_kind(f: &Fill) -> usize {
    match f {
        Fill::None => 0,
        Fill::Solid(_) => 1,
        Fill::Fountain(_) => 2,
        Fill::Pattern(_) => 3,
        Fill::Texture(_) => 4,
        Fill::Mesh(_) => 5,
    }
}

/// What changed from `old` to `new`: nothing, the whole fill, or some
/// gradient settings when both are gradients. A changed stop list counts
/// as a start or end colour when only that colour moved.
pub fn fill_changes(old: &Fill, new: &Fill) -> Vec<FillField> {
    if old == new {
        return Vec::new();
    }
    let (Fill::Fountain(a), Fill::Fountain(b)) = (old, new) else {
        return vec![FillField::All];
    };
    let mut out: Vec<FillField> = FillField::FOUNTAIN
        .into_iter()
        .filter(|f| *f != FillField::Stops && !f.same_fountain(a, b))
        .collect();
    if a.stops != b.stops {
        // Only the end colours changed (same count and positions, the
        // inner stops alike): keep those fields; anything else is the
        // whole stop list.
        let inner_same = a.stops.len() == b.stops.len()
            && a.stops.len() >= 2
            && a.stops.iter().zip(&b.stops).enumerate().all(|(i, (x, y))| {
                x.pos == y.pos
                    && x.mid == y.mid
                    && x.opacity == y.opacity
                    && (i == 0 || i + 1 == a.stops.len() || x.color == y.color)
            });
        if !inner_same {
            out.retain(|f| !matches!(f, FillField::StartColor | FillField::EndColor));
            out.push(FillField::Stops);
        }
    }
    out
}

/// `target` with `fields` taken from `from`. Gradient fields go into a
/// gradient target; any other target, or [`FillField::All`], takes the
/// whole of `from`.
pub fn patch_fill(target: &Fill, from: &Fill, fields: &[FillField]) -> Fill {
    if fields.is_empty() {
        return target.clone();
    }
    match (target, from) {
        (Fill::Fountain(t), Fill::Fountain(f)) if !fields.contains(&FillField::All) => {
            let mut out = t.clone();
            for field in fields {
                field.copy_fountain(f, &mut out);
            }
            Fill::Fountain(out)
        }
        _ => from.clone(),
    }
}

/// How the fills of a selection differ.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FillMix {
    /// The objects have different fill types.
    pub kind: bool,
    /// Same type, different values. For gradients this lists the
    /// settings that differ; for the other types it is
    /// `[FillField::All]` when anything differs.
    pub fields: Vec<FillField>,
}

impl FillMix {
    pub fn has(&self, f: FillField) -> bool {
        self.kind || self.fields.contains(&f) || self.fields.contains(&FillField::All)
    }

    pub fn is_empty(&self) -> bool {
        !self.kind && self.fields.is_empty()
    }
}

/// Which fill values differ across `fills` (one per object).
pub fn fill_mixed<'a>(fills: impl IntoIterator<Item = &'a Fill>) -> FillMix {
    let mut mix = FillMix::default();
    let mut first: Option<&Fill> = None;
    for f in fills {
        let Some(base) = first else {
            first = Some(f);
            continue;
        };
        if fill_kind(base) != fill_kind(f) {
            mix.kind = true;
            continue;
        }
        let fields = match (base, f) {
            (Fill::Fountain(a), Fill::Fountain(b)) => FillField::FOUNTAIN
                .into_iter()
                .filter(|x| !x.same_fountain(a, b))
                .collect(),
            _ if base != f => vec![FillField::All],
            _ => Vec::new(),
        };
        for x in fields {
            if !mix.fields.contains(&x) {
                mix.fields.push(x);
            }
        }
    }
    if mix.kind {
        mix.fields.clear();
    }
    mix
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Color;

    fn stroke(c: Color, w: f64) -> Stroke {
        Stroke {
            color: c,
            width: w,
            ..Stroke::default()
        }
    }

    #[test]
    fn a_width_change_keeps_each_colour() {
        let blue = Color::rgb8(0, 0, 255);
        let yellow = Color::rgb8(255, 255, 0);
        let a = stroke(blue, 1.0);
        let b = stroke(yellow, 3.0);
        let mut edited = a.clone();
        edited.width = 2.0;
        let fields = stroke_changes(&a, &edited);
        assert_eq!(fields, vec![StrokeField::Width]);
        let pa = patch_stroke(Some(&a), &edited, &fields);
        let pb = patch_stroke(Some(&b), &edited, &fields);
        assert_eq!((pa.color, pa.width), (blue, 2.0));
        assert_eq!((pb.color, pb.width), (yellow, 2.0));
        // No outline: the default outline with the width.
        let pn = patch_stroke(None, &edited, &fields);
        assert_eq!(pn.width, 2.0);
        assert_eq!(pn.color, Stroke::default().color);
    }

    #[test]
    fn position_mitre_and_dash_alignment_change_one_at_a_time() {
        let mut a = stroke(Color::rgb8(255, 0, 0), 1.0);
        let b = stroke(Color::rgb8(0, 0, 255), 2.0);
        let mut new = a.clone();
        new.position = crate::OutlinePosition::Inside;
        new.miter_limit = 30.0;
        new.align_dashes = true;
        let fields = stroke_changes(&a, &new);
        assert_eq!(
            fields,
            vec![
                StrokeField::Position,
                StrokeField::MiterLimit,
                StrokeField::AlignDashes
            ]
        );
        let out = patch_stroke(Some(&b), &new, &fields);
        assert_eq!(out.color, b.color);
        assert_eq!(out.width, b.width);
        assert_eq!(out.position, crate::OutlinePosition::Inside);
        assert_eq!(out.miter_limit, 30.0);
        assert!(out.align_dashes);
        a.position = crate::OutlinePosition::Outside;
        assert!(!StrokeField::Position.same(&a, &b));
    }

    #[test]
    fn mixed_outline_values_are_found() {
        let a = stroke(Color::rgb8(0, 0, 255), 1.0);
        let mut b = stroke(Color::rgb8(255, 255, 0), 1.0);
        b.cap = crate::LineCap::Round;
        let mix = stroke_mixed([Some(&a), Some(&b)]);
        assert!(!mix.presence);
        assert!(mix.has(StrokeField::Paint) && mix.has(StrokeField::Cap));
        assert!(!mix.has(StrokeField::Width));
        let mix = stroke_mixed([Some(&a), None]);
        assert!(mix.presence && mix.fields.is_empty());
        assert!(stroke_mixed([Some(&a), Some(&a.clone())]).is_empty());
    }

    #[test]
    fn gradient_settings_change_one_at_a_time() {
        let a = Fill::linear(Color::BLACK, Color::WHITE, 0.0);
        let b = Fill::linear(Color::rgb8(255, 0, 0), Color::rgb8(0, 0, 255), 45.0);
        let mut edited = a.clone();
        if let Fill::Fountain(f) = &mut edited {
            f.edge_pad = 0.2;
        }
        let fields = fill_changes(&a, &edited);
        assert_eq!(fields, vec![FillField::EdgePad]);
        let Fill::Fountain(pb) = patch_fill(&b, &edited, &fields) else {
            panic!("still a gradient");
        };
        assert_eq!(pb.edge_pad, 0.2);
        assert_eq!(pb.angle, 45.0);
        assert_eq!(pb.stops[0].color, Color::rgb8(255, 0, 0));
        // Only the start colour.
        let mut edited = a.clone();
        if let Fill::Fountain(f) = &mut edited {
            f.stops[0].color = Color::rgb8(0, 255, 0);
        }
        let fields = fill_changes(&a, &edited);
        assert_eq!(fields, vec![FillField::StartColor]);
        let Fill::Fountain(pb) = patch_fill(&b, &edited, &fields) else {
            panic!("still a gradient");
        };
        assert_eq!(pb.stops[0].color, Color::rgb8(0, 255, 0));
        assert_eq!(pb.stops[1].color, Color::rgb8(0, 0, 255));
        // A uniform target takes the whole gradient.
        let solid = Fill::Solid(Color::BLACK);
        assert_eq!(patch_fill(&solid, &edited, &fields), edited);
        // A type change is the whole fill.
        assert_eq!(fill_changes(&solid, &a), vec![FillField::All]);
        assert!(fill_changes(&a, &a).is_empty());
    }

    /// Nodes placed freely (or taken back to the angle and the offset)
    /// are a change of their own that a patch carries (#939).
    #[test]
    fn placed_nodes_are_a_change_of_their_own() {
        let a = Fill::linear(Color::BLACK, Color::WHITE, 0.0);
        let Fill::Fountain(mut fb) = a.clone() else {
            panic!("a gradient");
        };
        fb.handles = Some(crate::style::FountainHandles {
            start: crate::geometry::Point::new(-1.0, 0.0),
            end: crate::geometry::Point::new(0.5, 0.0),
            skew: crate::geometry::Point::new(-1.0, 1.0),
        });
        let b = Fill::Fountain(fb.clone());
        assert_eq!(fill_changes(&a, &b), vec![FillField::Handles]);
        let target = Fill::linear(Color::rgb8(255, 0, 0), Color::WHITE, 0.0);
        let Fill::Fountain(out) = patch_fill(&target, &b, &[FillField::Handles]) else {
            panic!("a gradient");
        };
        assert_eq!(out.handles, fb.handles);
        assert_eq!(out.stops[0].color, Color::rgb8(255, 0, 0));
        assert_eq!(fill_changes(&b, &a), vec![FillField::Handles]);
    }

    #[test]
    fn mixed_fill_values_are_found() {
        let red = Fill::Solid(Color::rgb8(255, 0, 0));
        let blue = Fill::Solid(Color::rgb8(0, 0, 255));
        let g = Fill::linear(Color::BLACK, Color::WHITE, 0.0);
        let g2 = Fill::linear(Color::BLACK, Color::WHITE, 30.0);
        let m = fill_mixed([&red, &blue]);
        assert!(!m.kind && m.has(FillField::All));
        assert!(fill_mixed([&red, &g]).kind);
        let m = fill_mixed([&g, &g2]);
        assert_eq!(m.fields, vec![FillField::Angle]);
        assert!(m.has(FillField::Angle) && !m.has(FillField::EdgePad));
        assert!(fill_mixed([&red, &red]).is_empty());
    }
}
