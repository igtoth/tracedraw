//! The Effects docker: the effect stack of the selected object, in the
//! order the effects apply. Each row switches its effect on or off (an
//! effect switched off is kept, at its place, and not drawn), moves it up
//! or down the stack, or removes it. Every change is one undo step.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::Ui;
use tracedraw_core::live::Effect;
use tracedraw_core::{Command, Shape};

/// The name an effect goes by in the stack.
pub fn effect_key(e: &Effect) -> &'static str {
    match e {
        Effect::Contour { .. } => "effect.contour",
        Effect::Blend { .. } => "effect.blend",
        Effect::Distort(_) => "effect.distort",
        Effect::Envelope { .. } => "effect.envelope",
        Effect::Perspective { .. } => "effect.perspective",
        Effect::Extrude { .. } => "effect.extrude",
        Effect::Bevel { .. } => "effect.bevel",
        Effect::BlockShadow { .. } => "effect.block_shadow",
        Effect::Symmetry { .. } => "effect.symmetry",
        Effect::Lens(_) => "effect.lens",
        Effect::Transparency { .. } => "effect.transparency",
    }
}

/// A change asked for in the stack.
enum Change {
    Toggle(usize),
    Up(usize),
    Down(usize),
    Remove(usize),
}

/// Apply `change` to the stack of `s`; `None` when it changes nothing.
fn changed_stack(s: &Shape, change: Change) -> Option<Vec<(Effect, bool)>> {
    let mut stack = s.effect_stack();
    let n = stack.len();
    match change {
        Change::Toggle(i) if i < n => stack[i].1 = !stack[i].1,
        Change::Up(i) if i > 0 && i < n => stack.swap(i - 1, i),
        Change::Down(i) if i + 1 < n => stack.swap(i, i + 1),
        Change::Remove(i) if i < n => {
            stack.remove(i);
        }
        _ => return None,
    }
    Some(stack)
}

pub fn effects_docker(app: &mut App, ui: &mut Ui) {
    let Some(shape) = app.selected_shapes().into_iter().next() else {
        ui.label(egui::RichText::new(tr("effects.select_hint")).color(Tokens::text_dim()));
        return;
    };
    let stack = shape.effect_stack();
    if stack.is_empty() {
        ui.label(egui::RichText::new(tr("effects.none")).color(Tokens::text_dim()));
        return;
    }
    let n = stack.len();
    let mut change = None;
    for (i, (effect, on)) in stack.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut shown = *on;
            if ui
                .checkbox(&mut shown, "")
                .rich_tip(tr("effects.toggle"))
                .changed()
            {
                change = Some(Change::Toggle(i));
            }
            let name = egui::RichText::new(tr(effect_key(effect)));
            ui.label(if *on {
                name
            } else {
                name.color(Tokens::text_dim())
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("\u{00D7}")
                    .rich_tip(tr("effects.remove"))
                    .clicked()
                {
                    change = Some(Change::Remove(i));
                }
                if ui
                    .add_enabled(i + 1 < n, egui::Button::new("\u{25BC}").small())
                    .rich_tip(tr("effects.down"))
                    .clicked()
                {
                    change = Some(Change::Down(i));
                }
                if ui
                    .add_enabled(i > 0, egui::Button::new("\u{25B2}").small())
                    .rich_tip(tr("effects.up"))
                    .clicked()
                {
                    change = Some(Change::Up(i));
                }
            });
        });
    }
    ui.add_space(4.0);
    ui.label(egui::RichText::new(tr("effects.order_hint")).color(Tokens::text_dim()));
    if let Some(stack) = change.and_then(|c| changed_stack(&shape, c)) {
        let (effects, parked) = Shape::split_stack(stack);
        app.run(Command::SetEffectStack {
            shape: shape.id,
            effects,
            parked,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Point, Rect, Vec2};
    use tracedraw_core::{Color, ShapeKind};

    #[test]
    fn the_stack_switches_moves_and_removes_effects() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        let shadow = Effect::BlockShadow {
            offset: Vec2::new(1.0, -1.0),
            color: Color::BLACK,
            gap: 0.0,
        };
        let persp = Effect::Perspective {
            corners: [Point::ZERO; 4],
        };
        app.run(Command::SetEffects {
            shape: id,
            effects: vec![shadow.clone(), persp.clone()],
        });
        app.select(vec![id]);
        let shape = |app: &App| app.doc().find_shape(id).expect("shape").clone();
        let apply = |app: &mut App, c: Change| {
            let s = shape(app);
            let (effects, parked) = Shape::split_stack(changed_stack(&s, c).expect("changes"));
            app.run(Command::SetEffectStack {
                shape: id,
                effects,
                parked,
            });
        };
        apply(&mut app, Change::Toggle(0));
        assert_eq!(shape(&app).effects, vec![persp.clone()]);
        assert_eq!(shape(&app).effect_stack()[0], (shadow.clone(), false));
        apply(&mut app, Change::Down(0));
        assert_eq!(
            shape(&app).effect_stack(),
            vec![(persp.clone(), true), (shadow.clone(), false)]
        );
        apply(&mut app, Change::Toggle(1));
        assert_eq!(shape(&app).effects, vec![persp.clone(), shadow.clone()]);
        apply(&mut app, Change::Remove(0));
        assert_eq!(shape(&app).effects, vec![shadow]);
        assert!(changed_stack(&shape(&app), Change::Up(0)).is_none());
        assert!(changed_stack(&shape(&app), Change::Remove(5)).is_none());
        app.undo();
        assert_eq!(shape(&app).effects.len(), 2);
        // The docker draws with and without a selection.
        let ctx = crate::theme::ui_context();
        let mut out = ctx.run_ui(Default::default(), |ui| effects_docker(&mut app, ui));
        out.textures_delta.clear();
        app.select(Vec::new());
        let mut out = ctx.run_ui(Default::default(), |ui| effects_docker(&mut app, ui));
        out.textures_delta.clear();
    }
}
