//! Editing a selection whose objects have different outlines, fills or
//! transparency: the editors show the values that differ as mixed, and a
//! change sends only the attribute that was edited, so every object keeps
//! its own values for the rest (see `tracedraw_core::patch`).

use crate::app::App;
use crate::effects_ui::TransparencySettings;
use tracedraw_core::document::{Shape, ShapeKind};
use tracedraw_core::live::{Effect, MergeMode};
use tracedraw_core::patch::{
    fill_changes, fill_mixed, stroke_changes, stroke_mixed, FillField, FillMix, StrokeField,
    StrokeMix,
};
use tracedraw_core::{Command, Corners, Fill, Stroke};

/// The objects a group stands for when its fill or outline is shown:
/// everything inside it, at any depth; other objects stand for
/// themselves.
fn leaves(s: &Shape, out: &mut Vec<Shape>) {
    match &s.kind {
        ShapeKind::Group { children } if !children.is_empty() => {
            for c in children {
                leaves(c, out);
            }
        }
        // A clip frame's fill and outline are its frame's (the object
        // drawn); the contents keep their own.
        ShapeKind::ClipFrame { frame, .. } => leaves(frame, out),
        _ => out.push(s.clone()),
    }
}

/// [`leaves`] by reference, without copies.
fn leaf_refs<'a>(s: &'a Shape, out: &mut Vec<&'a Shape>) {
    match &s.kind {
        ShapeKind::Group { children } if !children.is_empty() => {
            for c in children {
                leaf_refs(c, out);
            }
        }
        ShapeKind::ClipFrame { frame, .. } => leaf_refs(frame, out),
        _ => out.push(s),
    }
}

thread_local! {
    /// The revision after the last uniform amount change (or stop or
    /// foreground/background drag) and its field, so the next one of the
    /// same field can replace it (see [`App::patch_transparency`]).
    static LAST_AMOUNT_STEP: std::cell::Cell<Option<(u64, TransparencyField)>> =
        const { std::cell::Cell::new(None) };
}

/// The transparency of one object as the Transparency part shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeTransparency {
    /// 0 none, 1 uniform, 2 fountain, 3 pattern, 4 texture.
    pub kind: usize,
    /// Uniform amount in percent.
    pub amount: f64,
    pub mask: Option<Fill>,
    pub merge: MergeMode,
    pub target: u8,
}

/// Uniform transparency of `amount` % kept with a merge mode other than
/// Normal: the object is drawn through a mask of one grey (its lightness
/// is the opacity), so the merge mode applies as for the other kinds, as
/// the reference program offers it for uniform transparency too.
pub fn uniform_mask(amount: f64) -> Fill {
    Fill::Solid(tracedraw_core::Color::Gray {
        v: (1.0 - (amount / 100.0).clamp(0.0, 1.0)) as f32,
    })
}

/// The amount (%) of a [`uniform_mask`] colour.
fn uniform_amount(c: tracedraw_core::Color) -> f64 {
    let [r, g, b] = c.to_rgb_f32();
    let v = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
    // The grey is kept in single precision: rounded back to what was set.
    (((1.0 - v) * 100.0).clamp(0.0, 100.0) * 1e4).round() / 1e4
}

/// The transparency effect and the opacity that stand for uniform
/// transparency of `amount` % with `merge`.
pub fn uniform_transparency(amount: f64, merge: MergeMode, target: u8) -> (Option<Effect>, f64) {
    if merge == MergeMode::Normal {
        (None, 1.0 - (amount / 100.0).clamp(0.0, 1.0))
    } else {
        (
            Some(Effect::Transparency {
                mask: uniform_mask(amount),
                merge,
                target,
            }),
            1.0,
        )
    }
}

impl ShapeTransparency {
    pub fn of(s: &Shape) -> Self {
        let mask = s.effects.iter().find_map(|e| match e {
            Effect::Transparency {
                mask,
                merge,
                target,
            } => Some((mask.clone(), *merge, *target)),
            _ => None,
        });
        match mask {
            // Uniform transparency with a merge mode: a mask of one grey.
            Some((Fill::Solid(c), merge, target)) => ShapeTransparency {
                kind: 1,
                amount: uniform_amount(c),
                mask: None,
                merge,
                target,
            },
            Some((mask, merge, target)) => ShapeTransparency {
                kind: mask_kind(&mask),
                amount: 0.0,
                mask: Some(mask),
                merge,
                target,
            },
            None => ShapeTransparency {
                kind: if s.opacity < 1.0 { 1 } else { 0 },
                amount: ((1.0 - s.opacity) * 100.0).clamp(0.0, 100.0),
                mask: None,
                merge: MergeMode::Normal,
                target: 2,
            },
        }
    }
}

/// One setting of the Transparency part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransparencyField {
    /// The type (uniform, fountain, pattern, texture).
    Kind,
    /// The uniform amount.
    Amount,
    /// A fountain mask's angle.
    MaskAngle,
    /// A fountain mask's type (linear, radial...).
    MaskKind,
    /// The whole mask: a preset, or a type with a mask of its own kind
    /// (vector, bitmap or two-colour pattern, texture).
    Mask,
    /// A fountain mask's stops (each stop's grey and position).
    MaskStops,
    /// A two-colour pattern's or a texture's foreground and background
    /// greys.
    MaskColors,
    Merge,
    Target,
}

/// The kind number of [`ShapeTransparency::kind`] a mask stands for.
pub fn mask_kind(mask: &Fill) -> usize {
    match mask {
        Fill::Pattern(_) => 3,
        Fill::Texture(_) => 4,
        _ => 2,
    }
}

/// The foreground and background greys of a two-colour pattern or a
/// texture mask.
pub fn mask_colors(mask: &Fill) -> Option<(tracedraw_core::Color, tracedraw_core::Color)> {
    match mask {
        Fill::Pattern(tracedraw_core::Pattern::TwoColor { front, back, .. }) => {
            Some((*front, *back))
        }
        Fill::Texture(t) => Some((t.color_a, t.color_b)),
        _ => None,
    }
}

/// Which transparency settings differ across `ts`.
pub fn transparency_mixed(ts: &[ShapeTransparency]) -> Vec<TransparencyField> {
    let mut out = Vec::new();
    let Some(first) = ts.first() else {
        return out;
    };
    let fountain = |t: &ShapeTransparency| match &t.mask {
        Some(Fill::Fountain(f)) => Some((f.angle, f.kind)),
        _ => None,
    };
    for t in &ts[1..] {
        let mut push = |f| {
            if !out.contains(&f) {
                out.push(f);
            }
        };
        if t.kind != first.kind {
            push(TransparencyField::Kind);
        }
        if t.kind == 1 && first.kind == 1 && t.amount != first.amount {
            push(TransparencyField::Amount);
        }
        if let (Some(a), Some(b)) = (fountain(first), fountain(t)) {
            if a.0 != b.0 {
                push(TransparencyField::MaskAngle);
            }
            if a.1 != b.1 {
                push(TransparencyField::MaskKind);
            }
        }
        if t.mask.is_some() && first.mask.is_some() {
            if t.merge != first.merge {
                push(TransparencyField::Merge);
            }
            if t.target != first.target {
                push(TransparencyField::Target);
            }
        }
    }
    out
}

impl App {
    /// The objects the selection's fill and outline editors show and
    /// change: the selected objects, a group standing for what is in it.
    /// The first of [`Self::paint_targets`], without copying the others
    /// (the bars ask for it every frame, #345).
    pub fn first_paint_target(&self) -> Option<Shape> {
        let selected = self.selected_rc();
        let mut first = Vec::new();
        for s in selected.iter() {
            leaf_refs(s, &mut first);
            if let Some(f) = first.first() {
                return Some((*f).clone());
            }
        }
        None
    }

    pub fn paint_targets(&self) -> Vec<Shape> {
        let mut out = Vec::new();
        for s in self.selected_rc().iter() {
            leaves(s, &mut out);
        }
        out
    }

    /// How the outlines of the selection differ.
    pub fn selection_stroke_mix(&self) -> StrokeMix {
        let selected = self.selected_rc();
        let mut shapes = Vec::new();
        for s in selected.iter() {
            leaf_refs(s, &mut shapes);
        }
        stroke_mixed(shapes.iter().map(|s| s.stroke.as_ref()))
    }

    /// How the fills of the selection differ.
    pub fn selection_fill_mix(&self) -> FillMix {
        let selected = self.selected_rc();
        let mut shapes = Vec::new();
        for s in selected.iter() {
            leaf_refs(s, &mut shapes);
        }
        fill_mixed(shapes.iter().map(|s| &s.fill))
    }

    /// An outline editor showed `old` and the user made it `new`: send the
    /// fields that changed, plus `touched` (fields set to the value they
    /// already showed, such as a width typed into a mixed field), to every
    /// selected object, which keeps its own values for the rest. One undo
    /// step.
    pub fn patch_selection_stroke(
        &mut self,
        old: &Option<Stroke>,
        new: &Option<Stroke>,
        touched: &[StrokeField],
    ) {
        if let Some(c) = self.patch_selection_stroke_as(old, new, touched) {
            self.add_ink_for(c);
            self.remember_color(c);
        }
    }

    /// An outline changed by dragging in the Properties outline editor
    /// (its colour picker, a slider): while the button is held every
    /// change replaces the step the drag made before (one undo step for
    /// the whole drag), and a new colour joins the document palette only
    /// when the button is let go (`end_live_stroke`).
    pub fn patch_selection_stroke_live(
        &mut self,
        old: &Option<Stroke>,
        new: &Option<Stroke>,
        touched: &[StrokeField],
    ) {
        let mut kept = None;
        if let Some((rev, c)) = self.live_stroke {
            if rev == self.engine.revision() {
                let _ = self.engine.undo();
                kept = c;
            }
        }
        let before = self.engine.revision();
        let color = self.patch_selection_stroke_as(old, new, touched).or(kept);
        let after = self.engine.revision();
        self.live_stroke = (after != before).then_some((after, color));
    }

    /// The button is up: an outline dragged in the outline editor is
    /// done, and its new colour joins the document palette.
    pub fn end_live_stroke(&mut self) {
        let Some((rev, color)) = self.live_stroke.take() else {
            return;
        };
        if rev != self.engine.revision() {
            return;
        }
        if let Some(c) = color {
            self.add_ink_for(c);
            self.remember_color(c);
        }
    }

    /// Apply the outline change; the colour it painted, if any, for the
    /// caller to add to the document palette.
    fn patch_selection_stroke_as(
        &mut self,
        old: &Option<Stroke>,
        new: &Option<Stroke>,
        touched: &[StrokeField],
    ) -> Option<tracedraw_core::Color> {
        let shapes = self.selection.clone();
        if shapes.is_empty() {
            return None;
        }
        let Some(n) = new else {
            if old.is_some() || self.selection_stroke_mix().presence {
                self.run(Command::SetStroke {
                    shapes,
                    stroke: None,
                });
            }
            return None;
        };
        let mut fields = match old {
            Some(o) => stroke_changes(o, n),
            // The outline was just turned on: the objects without one get
            // the default outline, the others keep theirs.
            None => stroke_changes(&Stroke::default(), n),
        };
        for t in touched {
            if !fields.contains(t) {
                fields.push(*t);
            }
        }
        if fields.is_empty() && old.is_some() && !self.selection_stroke_mix().presence {
            return None;
        }
        let paint = fields.contains(&StrokeField::Paint) && n.fill.is_none();
        let before = self.engine.revision();
        self.run(Command::PatchStroke {
            shapes,
            stroke: n.clone(),
            fields,
        });
        if paint && self.engine.revision() != before {
            self.current_colors.1 = n.color;
            return Some(n.color);
        }
        None
    }

    /// A fill editor showed `old` and the user made it `new`: a new type
    /// or a uniform colour goes to every selected object as it is (as
    /// [`App::apply_fill`]), a gradient setting goes into each object's
    /// own gradient. `touched` adds fields set to the value they showed.
    pub fn patch_selection_fill(&mut self, old: &Fill, new: &Fill, touched: &[FillField]) {
        if self.selection.is_empty() {
            return;
        }
        let mut fields = fill_changes(old, new);
        for t in touched {
            if !fields.contains(t) {
                fields.push(*t);
            }
        }
        if fields.is_empty() {
            return;
        }
        if fields.contains(&FillField::All) || !matches!(new, Fill::Fountain(_)) {
            self.apply_fill(new.clone());
            return;
        }
        let shapes = self.selection.clone();
        self.run(Command::PatchFill {
            shapes,
            fill: new.clone(),
            fields,
        });
    }

    /// The transparency of each object the Transparency part changes.
    pub fn selection_transparency(&self) -> Vec<ShapeTransparency> {
        self.selected_shapes()
            .iter()
            .map(ShapeTransparency::of)
            .collect()
    }

    /// Set the transparency `fields` from `settings` on every selected
    /// object, each keeping its other transparency settings (one undo
    /// step; a run of slider changes stays one step).
    pub fn patch_transparency(
        &mut self,
        settings: &TransparencySettings,
        fields: &[TransparencyField],
    ) {
        let shapes = self.selected_shapes();
        if shapes.is_empty() || fields.is_empty() {
            return;
        }
        let mut cmds = Vec::new();
        for s in &shapes {
            let mut t = ShapeTransparency::of(s);
            for f in fields {
                match f {
                    TransparencyField::Kind => {
                        let kind = settings.kind as usize + 1;
                        if kind != t.kind {
                            t.kind = kind;
                            t.mask = (kind > 1).then(|| settings.mask.clone());
                            if kind == 1 {
                                t.amount = settings.amount;
                            }
                        }
                    }
                    TransparencyField::Amount => {
                        t.kind = 1;
                        t.mask = None;
                        t.amount = settings.amount;
                    }
                    TransparencyField::MaskAngle | TransparencyField::MaskKind => {
                        let Fill::Fountain(from) = &settings.mask else {
                            continue;
                        };
                        match &mut t.mask {
                            Some(Fill::Fountain(m)) => {
                                if *f == TransparencyField::MaskAngle {
                                    m.angle = from.angle;
                                } else {
                                    m.kind = from.kind;
                                }
                            }
                            _ => {
                                t.kind = 2;
                                t.mask = Some(settings.mask.clone());
                            }
                        }
                    }
                    TransparencyField::Mask => {
                        t.kind = mask_kind(&settings.mask);
                        t.mask = Some(settings.mask.clone());
                    }
                    TransparencyField::MaskStops => {
                        let Fill::Fountain(from) = &settings.mask else {
                            continue;
                        };
                        match &mut t.mask {
                            Some(Fill::Fountain(m)) => m.stops = from.stops.clone(),
                            _ => {
                                t.kind = 2;
                                t.mask = Some(settings.mask.clone());
                            }
                        }
                    }
                    TransparencyField::MaskColors => {
                        let Some((fg, bg)) = mask_colors(&settings.mask) else {
                            continue;
                        };
                        match &mut t.mask {
                            Some(Fill::Pattern(tracedraw_core::Pattern::TwoColor {
                                front,
                                back,
                                ..
                            })) => {
                                *front = fg;
                                *back = bg;
                            }
                            Some(Fill::Texture(x)) => {
                                x.color_a = fg;
                                x.color_b = bg;
                            }
                            // Other masks have no two greys to set.
                            _ => {}
                        }
                    }
                    TransparencyField::Merge => t.merge = settings.merge,
                    TransparencyField::Target => t.target = settings.target,
                }
            }
            let mut effects: Vec<Effect> = s
                .effects
                .iter()
                .filter(|e| !matches!(e, Effect::Transparency { .. }))
                .cloned()
                .collect();
            let mut opacity = 1.0;
            if let (true, Some(mask)) = (t.kind > 1, t.mask.clone()) {
                effects.push(Effect::Transparency {
                    mask,
                    merge: t.merge,
                    target: t.target,
                });
            } else if t.kind == 1 {
                let (effect, o) = uniform_transparency(t.amount, t.merge, t.target);
                effects.extend(effect);
                opacity = o;
            }
            cmds.push(Command::SetEffects {
                shape: s.id,
                effects,
            });
            cmds.push(Command::SetOpacity {
                shapes: vec![s.id],
                opacity,
            });
        }
        // A run of amount changes (a slider being dragged) is one step:
        // the step this made last is replaced while nothing else happened.
        // So is a run of stop or foreground/background changes (a field
        // being dragged in the Properties docker).
        let run = match fields {
            [f @ (TransparencyField::Amount
            | TransparencyField::MaskStops
            | TransparencyField::MaskColors)] => Some(*f),
            _ => None,
        };
        if run.is_some()
            && self.engine.undo_label() == Some("Transparency")
            && LAST_AMOUNT_STEP.with(|c| c.get()) == run.map(|f| (self.engine.revision(), f))
        {
            let _ = self.engine.undo();
        }
        if let Err(e) = self.engine.run_batch("Transparency", &cmds) {
            self.status = e.to_string();
        }
        let rev = run.map(|f| (self.engine.revision(), f));
        LAST_AMOUNT_STEP.with(|c| c.set(rev));
    }

    /// The corners of the selected rectangles, and which of their values
    /// differ: `[kind, top left, top right, bottom right, bottom left
    /// (the radii in `Corners` order), scaling]`.
    pub fn selection_corner_mix(&self) -> Option<(Corners, [bool; 6])> {
        let all: Vec<Corners> = self
            .selected_rc()
            .iter()
            .filter_map(|s| s.page_corners())
            .collect();
        let first = *all.first()?;
        let mut mix = [false; 6];
        for c in &all[1..] {
            mix[0] |= c.kind != first.kind;
            for i in 0..4 {
                mix[i + 1] |= (c.radii[i] - first.radii[i]).abs() > 1e-9;
            }
            mix[5] |= c.fixed != first.fixed;
        }
        Some((first, mix))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::Color;

    const BLUE: Color = Color::Rgb {
        r: 0.0,
        g: 0.0,
        b: 1.0,
    };

    fn two_rects(app: &mut App) -> (tracedraw_core::ShapeId, tracedraw_core::ShapeId) {
        let a = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .unwrap();
        let b = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(30.0, 0.0, 50.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .unwrap();
        let yellow = Color::rgb8(255, 255, 0);
        for (id, c, w) in [(a, BLUE, 1.0), (b, yellow, 3.0)] {
            app.run(Command::SetStroke {
                shapes: vec![id],
                stroke: Some(Stroke {
                    color: c,
                    width: w,
                    ..Stroke::default()
                }),
            });
        }
        app.select(vec![a, b]);
        (a, b)
    }

    fn stroke_of(app: &App, id: tracedraw_core::ShapeId) -> Stroke {
        app.doc().find_shape(id).unwrap().stroke.clone().unwrap()
    }

    #[test]
    fn a_width_change_keeps_each_outline_colour() {
        let mut app = App::headless();
        let (a, b) = two_rects(&mut app);
        let mix = app.selection_stroke_mix();
        assert!(mix.has(StrokeField::Paint) && mix.has(StrokeField::Width));
        let old = Some(stroke_of(&app, a));
        let mut new = old.clone();
        new.as_mut().unwrap().width = 2.0;
        app.patch_selection_stroke(&old, &new, &[]);
        assert_eq!(app.engine.undo_label(), Some("Outline"));
        assert_eq!(stroke_of(&app, a).color, BLUE);
        assert_eq!(stroke_of(&app, b).color, Color::rgb8(255, 255, 0));
        assert_eq!(stroke_of(&app, a).width, 2.0);
        assert_eq!(stroke_of(&app, b).width, 2.0);
        assert!(!app.selection_stroke_mix().has(StrokeField::Width));
        // One undo step takes both back.
        let _ = app.engine.undo();
        assert_eq!(stroke_of(&app, a).width, 1.0);
        assert_eq!(stroke_of(&app, b).width, 3.0);
    }

    /// Every outline attribute the panel edits changes only itself: the
    /// other object keeps its own colour and width, and the step undoes
    /// in one go.
    #[test]
    fn each_outline_attribute_changes_alone() {
        use tracedraw_core::{Arrowhead, LineCap, LineJoin};
        type Edit = fn(&mut Stroke);
        let edits: [(StrokeField, Edit); 9] = [
            (StrokeField::Cap, |s| s.cap = LineCap::Round),
            (StrokeField::Join, |s| s.join = LineJoin::Bevel),
            (StrokeField::Dash, |s| s.dash = vec![4.0, 2.0]),
            (StrokeField::StartArrow, |s| {
                s.start_arrow = Arrowhead::ALL[1].clone()
            }),
            (StrokeField::EndArrow, |s| {
                s.end_arrow = Arrowhead::ALL[1].clone()
            }),
            (StrokeField::BehindFill, |s| s.behind_fill = !s.behind_fill),
            (StrokeField::ScaleWithObject, |s| {
                s.scale_with_object = !s.scale_with_object
            }),
            (StrokeField::Stretch, |s| s.stretch = 0.4),
            (StrokeField::NibAngle, |s| s.nib_angle = 30.0),
        ];
        for (field, edit) in edits {
            let mut app = App::headless();
            let (a, b) = two_rects(&mut app);
            let old = Some(stroke_of(&app, a));
            let mut new = old.clone();
            edit(new.as_mut().unwrap());
            app.patch_selection_stroke(&old, &new, &[]);
            let (sa, sb) = (stroke_of(&app, a), stroke_of(&app, b));
            assert!(field.same(&sa, new.as_ref().unwrap()), "{field:?}");
            assert!(field.same(&sb, new.as_ref().unwrap()), "{field:?}");
            assert_eq!((sa.color, sa.width), (BLUE, 1.0), "{field:?}");
            assert_eq!(
                (sb.color, sb.width),
                (Color::rgb8(255, 255, 0), 3.0),
                "{field:?}"
            );
            let _ = app.engine.undo();
            assert!(!field.same(&stroke_of(&app, b), new.as_ref().unwrap()));
        }
    }

    /// The property bar's width box and the palette's outline colour
    /// change only their own value, also inside a group.
    #[test]
    fn the_property_bar_width_and_colour_keep_the_rest() {
        let mut app = App::headless();
        let (a, b) = two_rects(&mut app);
        app.apply_outline_width(2.5);
        assert_eq!(stroke_of(&app, a).color, BLUE);
        assert_eq!(stroke_of(&app, b).color, Color::rgb8(255, 255, 0));
        assert_eq!(stroke_of(&app, b).width, 2.5);
        let _ = app.engine.undo();
        app.apply_outline_color(Some(Color::rgb8(255, 0, 0)));
        assert_eq!(stroke_of(&app, a).width, 1.0);
        assert_eq!(stroke_of(&app, b).width, 3.0);
        assert_eq!(stroke_of(&app, a).color, Color::rgb8(255, 0, 0));
        let _ = app.engine.undo();
        // A group: the objects in it keep their own colours.
        app.group_selection();
        assert_eq!(app.selection.len(), 1);
        app.apply_outline_width(0.75);
        assert_eq!(stroke_of(&app, a).color, BLUE);
        assert_eq!(stroke_of(&app, b).color, Color::rgb8(255, 255, 0));
        assert_eq!(stroke_of(&app, a).width, 0.75);
        assert_eq!(stroke_of(&app, b).width, 0.75);
        let mix = app.selection_stroke_mix();
        assert!(mix.has(StrokeField::Paint) && !mix.has(StrokeField::Width));
    }

    #[test]
    fn a_colour_change_keeps_each_outline_width() {
        let mut app = App::headless();
        let (a, b) = two_rects(&mut app);
        let old = Some(stroke_of(&app, a));
        let mut new = old.clone();
        new.as_mut().unwrap().color = Color::rgb8(255, 0, 0);
        app.patch_selection_stroke(&old, &new, &[]);
        assert_eq!(stroke_of(&app, a).width, 1.0);
        assert_eq!(stroke_of(&app, b).width, 3.0);
        assert_eq!(stroke_of(&app, b).color, Color::rgb8(255, 0, 0));
    }

    #[test]
    fn a_value_typed_into_a_mixed_field_applies_to_all() {
        let mut app = App::headless();
        let (a, b) = two_rects(&mut app);
        // The first object's width typed again: nothing differs from what
        // was shown, but the field was mixed, so every object takes it.
        let old = Some(stroke_of(&app, a));
        app.patch_selection_stroke(&old, &old.clone(), &[StrokeField::Width]);
        assert_eq!(stroke_of(&app, b).width, 1.0);
        assert_eq!(stroke_of(&app, b).color, Color::rgb8(255, 255, 0));
    }

    #[test]
    fn fills_change_one_setting_at_a_time() {
        let mut app = App::headless();
        let (a, b) = two_rects(&mut app);
        let ga = Fill::linear(Color::BLACK, Color::WHITE, 0.0);
        let gb = Fill::linear(Color::rgb8(255, 0, 0), BLUE, 45.0);
        app.run(Command::SetFill {
            shapes: vec![a],
            fill: ga.clone(),
        });
        app.run(Command::SetFill {
            shapes: vec![b],
            fill: gb,
        });
        let mix = app.selection_fill_mix();
        assert!(!mix.kind && mix.has(FillField::Angle) && mix.has(FillField::StartColor));
        let mut new = ga.clone();
        if let Fill::Fountain(f) = &mut new {
            f.edge_pad = 0.25;
        }
        app.patch_selection_fill(&ga, &new, &[]);
        let Fill::Fountain(fb) = app.doc().find_shape(b).unwrap().fill.clone() else {
            panic!("b keeps a gradient");
        };
        assert_eq!(fb.edge_pad, 0.25);
        assert_eq!(fb.angle, 45.0);
        assert_eq!(fb.stops[0].color, Color::rgb8(255, 0, 0));
        // A uniform colour goes to both and keeps their outlines.
        let red = Fill::Solid(Color::rgb8(255, 0, 0));
        app.patch_selection_fill(&ga, &red, &[]);
        for id in [a, b] {
            assert_eq!(app.doc().find_shape(id).unwrap().fill, red);
        }
        assert_eq!(stroke_of(&app, b).width, 3.0);
        // Uniform colours that differ show as mixed.
        app.run(Command::SetFill {
            shapes: vec![b],
            fill: Fill::Solid(BLUE),
        });
        let mix = app.selection_fill_mix();
        assert!(!mix.kind && mix.has(FillField::All));
    }

    #[test]
    fn transparency_changes_one_setting_at_a_time() {
        let mut app = App::headless();
        let (a, b) = two_rects(&mut app);
        // a: uniform 30 %; b: a fountain mask with Multiply.
        app.run(Command::SetOpacity {
            shapes: vec![a],
            opacity: 0.7,
        });
        let mask = Fill::linear(Color::WHITE, Color::BLACK, 30.0);
        app.run(Command::SetEffects {
            shape: b,
            effects: vec![Effect::Transparency {
                mask: mask.clone(),
                merge: MergeMode::Multiply,
                target: 0,
            }],
        });
        let ts = app.selection_transparency();
        assert_eq!((ts[0].kind, ts[1].kind), (1, 2));
        assert!(transparency_mixed(&ts).contains(&TransparencyField::Kind));
        // Changing the merge mode leaves a's uniform amount and b's mask.
        let settings = TransparencySettings {
            merge: MergeMode::Add,
            ..Default::default()
        };
        app.patch_transparency(&settings, &[TransparencyField::Merge]);
        let ts = app.selection_transparency();
        assert_eq!(ts[0].kind, 1);
        assert!((ts[0].amount - 30.0).abs() < 1e-9);
        assert_eq!(ts[1].mask, Some(mask));
        assert_eq!(ts[1].merge, MergeMode::Add);
        assert_eq!(ts[1].target, 0);
        // An amount makes both uniform at that amount.
        let settings = TransparencySettings {
            amount: 60.0,
            ..Default::default()
        };
        app.patch_transparency(&settings, &[TransparencyField::Amount]);
        let ts = app.selection_transparency();
        assert!(ts
            .iter()
            .all(|t| t.kind == 1 && (t.amount - 60.0).abs() < 1e-9));
        assert!(transparency_mixed(&ts).is_empty());
    }

    /// A whole mask, the stops of a gradient mask and the two greys of a
    /// two-colour pattern or texture mask each patch only that part; a
    /// run of stop changes is one undo step (#1439, #1203).
    #[test]
    fn masks_stops_and_greys_patch_their_part() {
        use tracedraw_core::style::{Pattern, Stop};
        let mut app = App::headless();
        let (a, b) = two_rects(&mut app);
        app.select(vec![a, b]);
        // A whole mask: a two-colour pattern on both.
        let pattern = TransparencySettings::default_mask(2, 0.0);
        let settings = TransparencySettings {
            kind: 2,
            mask: pattern.clone(),
            ..Default::default()
        };
        app.patch_transparency(&settings, &[TransparencyField::Mask]);
        let ts = app.selection_transparency();
        assert!(ts
            .iter()
            .all(|t| t.kind == 3 && t.mask == Some(pattern.clone())));
        // The greys: only front and back change, the tile stays.
        let mut grey = settings.clone();
        if let Fill::Pattern(Pattern::TwoColor { front, back, .. }) = &mut grey.mask {
            *front = Color::Gray { v: 0.25 };
            *back = Color::Gray { v: 0.75 };
        }
        app.patch_transparency(&grey, &[TransparencyField::MaskColors]);
        let ts = app.selection_transparency();
        assert_eq!(
            ts[0].mask.as_ref().and_then(mask_colors),
            Some((Color::Gray { v: 0.25 }, Color::Gray { v: 0.75 }))
        );
        // Stops: a gradient mask takes the new stops and keeps its angle.
        app.select(vec![a]);
        let mut fountain = Fill::linear(Color::WHITE, Color::BLACK, 30.0);
        let settings = TransparencySettings {
            kind: 1,
            mask: fountain.clone(),
            ..Default::default()
        };
        app.patch_transparency(&settings, &[TransparencyField::Mask]);
        let steps = app.engine.history_labels().0.len();
        for v in [0.9, 0.8, 0.7] {
            if let Fill::Fountain(f) = &mut fountain {
                f.angle = 0.0;
                f.stops = vec![
                    Stop::new(0.0, Color::Gray { v }),
                    Stop::new(0.4, Color::Gray { v: 0.5 }),
                    Stop::new(1.0, Color::BLACK),
                ];
            }
            let s = TransparencySettings {
                kind: 1,
                mask: fountain.clone(),
                ..Default::default()
            };
            app.patch_transparency(&s, &[TransparencyField::MaskStops]);
        }
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        let ts = app.selection_transparency();
        let Some(Fill::Fountain(f)) = &ts[0].mask else {
            panic!("a gradient mask");
        };
        assert_eq!(f.stops.len(), 3);
        assert_eq!(f.stops[0].color, Color::Gray { v: 0.7 });
        assert!((f.angle - 30.0).abs() < 1e-9);
        // The greys leave a gradient mask alone.
        app.patch_transparency(&grey, &[TransparencyField::MaskColors]);
        assert_eq!(app.selection_transparency()[0].mask, ts[0].mask);
    }

    #[test]
    fn corner_values_that_differ_are_found() {
        let mut app = App::headless();
        let (a, b) = two_rects(&mut app);
        assert_eq!(app.selection_corner_mix().unwrap().1, [false; 6]);
        app.select(vec![a]);
        crate::ui::propbar::edit_corners(&mut app, crate::ui::propbar::CornerEdit::Size(0, 2.0));
        app.select(vec![a, b]);
        let (_, mix) = app.selection_corner_mix().unwrap();
        assert!(mix[1] && !mix[0] && !mix[5]);
    }
}
