//! The Pick tool's drags in the real window, with pointer events and
//! modifier keys: Ctrl keeps a move straight, a right click during a drag
//! leaves the original, Space drops a copy, Shift scales from the centre,
//! Ctrl scales in steps of 100 % (mirror at the same size), Alt frees a
//! corner, the side handles of rotate mode skew, and the centre of
//! rotation moves. Also the pointer's shape over handles and objects.

use crate::app::{App, Drag, Handle, HandleMods};
use crate::interaction::skew_transform;
use crate::tools::Tool;
use crate::ui::cursors::{for_tool, resize_icon, ShapeMark, ToolCursor};
use egui::{Event, Modifiers, PointerButton, Pos2};
use tracedraw_core::geometry::{Affine, Point, Rect, Vec2};
use tracedraw_core::{ShapeId, ShapeKind};

fn frame(ctx: &egui::Context, app: &mut App, events: Vec<Event>, modifiers: Modifiers) {
    // The keys held come first, as the windowing layer sends them.
    let mut all = vec![Event::ModifiersChanged(modifiers)];
    all.extend(events);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        events: all,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
    out.textures_delta.clear();
}

fn button(pos: Pos2, pressed: bool, button: PointerButton, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers,
    }
}

fn key(key: egui::Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

/// A primary drag from `from` to `to` with `mods` held, `during` sent in
/// the middle of it.
fn drag(
    ctx: &egui::Context,
    app: &mut App,
    from: Pos2,
    to: Pos2,
    mods: Modifiers,
    during: Vec<Event>,
) {
    frame(ctx, app, vec![Event::PointerMoved(from)], mods);
    frame(
        ctx,
        app,
        vec![button(from, true, PointerButton::Primary, mods)],
        mods,
    );
    for i in 1..=4 {
        let p = from + (to - from) * (i as f32 / 4.0);
        let mut events = vec![Event::PointerMoved(p)];
        if i == 2 {
            events.extend(during.clone());
        }
        frame(ctx, app, events, mods);
    }
    frame(
        ctx,
        app,
        vec![button(to, false, PointerButton::Primary, mods)],
        mods,
    );
    frame(ctx, app, Vec::new(), Modifiers::NONE);
}

/// A drawing with a 50 x 30 mm rectangle at (10, 10), selected, the
/// page in view.
fn setup() -> (egui::Context, App, ShapeId) {
    let ctx = crate::theme::ui_context();
    let mut app = App::headless();
    app.show_welcome = false;
    let id = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(10.0, 10.0, 60.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.set_tool(Tool::Pick);
    app.select(vec![id]);
    // Filled, so that a press inside it picks it.
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
        200, 30, 30,
    )));
    // The window's bars settle over the first frames; then the page fits.
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    app.zoom_to_page();
    for _ in 0..2 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    (ctx, app, id)
}

fn bounds(app: &App, id: ShapeId) -> Rect {
    app.doc().find_shape(id).map(|s| s.bounds()).expect("shape")
}

fn handle(app: &App, which: Handle) -> Pos2 {
    let b = app.selection_bounds().expect("selection");
    app.handle_positions(b)
        .iter()
        .find(|(h, _)| *h == which)
        .map(|(_, p)| *p)
        .expect("handle")
}

fn shape_count(app: &App) -> usize {
    app.doc()
        .all_layers()
        .map(|l| l.shapes.len())
        .sum::<usize>()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.05
}

#[test]
fn ctrl_keeps_a_move_straight() {
    let (ctx, mut app, id) = setup();
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(120.0, 35.0),
        Modifiers::CTRL,
        Vec::new(),
    );
    let b = bounds(&app, id);
    assert!(b.x0 > 15.0, "moved sideways: {b:?}");
    assert!(close(b.y0, 10.0), "kept its height: {b:?}");
    // Without Ctrl it follows the pointer both ways.
    let from = app.view.to_screen(b.center());
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(0.0, -60.0),
        Modifiers::NONE,
        Vec::new(),
    );
    assert!(bounds(&app, id).y0 > b.y0 + 5.0);
}

#[test]
fn a_right_click_while_dragging_leaves_the_original() {
    let (ctx, mut app, id) = setup();
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    let to = from + egui::vec2(150.0, 0.0);
    let right = |p: Pos2, down: bool| button(p, down, PointerButton::Secondary, Modifiers::NONE);
    let mid = from + (to - from) * 0.5;
    drag(
        &ctx,
        &mut app,
        from,
        to,
        Modifiers::NONE,
        vec![right(mid, true), right(mid, false)],
    );
    assert_eq!(shape_count(&app), 2);
    assert!(close(bounds(&app, id).x0, 10.0), "the original stays");
    // The copy is the selection, moved.
    let copy = app.selection.first().copied().expect("copy");
    assert_ne!(copy, id);
    assert!(bounds(&app, copy).x0 > 15.0);
    assert!(app.dialog_is_none_or_menu_closed());
}

/// Repeat after a copy made with a right click during a drag makes
/// another copy, as far again from the last one (#363).
#[test]
fn repeat_after_a_right_click_copy_makes_another_copy() {
    let (ctx, mut app, id) = setup();
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    let to = from + egui::vec2(150.0, 0.0);
    let right = |p: Pos2, down: bool| button(p, down, PointerButton::Secondary, Modifiers::NONE);
    let mid = from + (to - from) * 0.5;
    drag(
        &ctx,
        &mut app,
        from,
        to,
        Modifiers::NONE,
        vec![right(mid, true), right(mid, false)],
    );
    assert_eq!(shape_count(&app), 2);
    let first = app.selection[0];
    let step = bounds(&app, first).x0 - bounds(&app, id).x0;
    assert!(step > 5.0, "{step}");
    app.repeat_last();
    assert_eq!(shape_count(&app), 3, "Repeat makes a third copy");
    let second = app.selection[0];
    assert!(second != first && second != id);
    assert!(close(
        bounds(&app, second).x0,
        bounds(&app, first).x0 + step
    ));
    assert!(
        close(bounds(&app, first).x0, bounds(&app, id).x0 + step),
        "the first copy stays"
    );
    app.repeat_last();
    assert_eq!(shape_count(&app), 4);
    // Each copy, the drag's and Repeat's, is one undo step: one Undo
    // takes the newest copy away whole, not just its move (#471).
    let third = app.selection[0];
    let steps = app.engine.history_labels().0.len();
    app.undo();
    assert_eq!(app.engine.history_labels().0.len(), steps - 1);
    assert_eq!(shape_count(&app), 3);
    assert!(app.doc().find_shape(third).is_none());
    app.undo();
    app.undo();
    assert_eq!(shape_count(&app), 1, "three Undos for three copies");
}

/// Repeat after an undo repeats the last action still in effect, not the
/// one just undone; after a redo the redone action is the last again
/// (#366).
#[test]
fn repeat_after_an_undo_takes_the_last_action_still_in_effect() {
    use tracedraw_core::{Affine, Command};
    let (_ctx, mut app, id) = setup();
    app.select(vec![id]);
    // A: a move; B: a rotation.
    app.transform_selection(Affine::translate((10.0, 0.0)));
    let after_a = bounds(&app, id);
    let c = after_a.center();
    app.transform_selection(Affine::rotate_about(std::f64::consts::FRAC_PI_2, c));
    assert!(!close(bounds(&app, id).width(), after_a.width()));
    app.undo();
    assert!(close(bounds(&app, id).width(), after_a.width()));
    // Repeat: the move again, not the rotation.
    app.repeat_last();
    let b = bounds(&app, id);
    assert!(close(b.x0, after_a.x0 + 10.0), "{b:?} {after_a:?}");
    assert!(close(b.width(), after_a.width()), "not turned: {b:?}");
    // Undo that, redo the rotation: Repeat turns again.
    app.undo();
    app.redo();
    let turned = bounds(&app, id);
    app.repeat_last();
    assert!(
        close(bounds(&app, id).width(), after_a.width()),
        "turned a second time: {:?} {turned:?}",
        bounds(&app, id)
    );
    // An undone action followed by another change is never repeated.
    app.undo();
    app.undo();
    app.run(Command::SetOpacity {
        shapes: vec![id],
        opacity: 0.5,
    });
    app.run(Command::SetOpacity {
        shapes: vec![id],
        opacity: 1.0,
    });
    app.undo();
    // The opacity of 0.5 is the last action in effect.
    app.repeat_last();
    let op = app.doc().find_shape(id).map(|s| s.opacity).expect("shape");
    assert!(close(op, 0.5), "{op}");
}

#[test]
fn esc_cancels_a_drag() {
    let (ctx, mut app, id) = setup();
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(150.0, 40.0),
        Modifiers::NONE,
        vec![key(egui::Key::Escape, Modifiers::NONE)],
    );
    assert!(close(bounds(&app, id).x0, 10.0), "nothing moved");
    assert_eq!(app.selection, vec![id], "the selection stays");
    assert!(matches!(app.drag, Drag::None));
}

#[test]
fn space_while_moving_drops_a_copy() {
    let (ctx, mut app, id) = setup();
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(160.0, 0.0),
        Modifiers::NONE,
        vec![key(egui::Key::Space, Modifiers::NONE)],
    );
    // The original went all the way; a copy stayed half way.
    assert_eq!(shape_count(&app), 2);
    let moved = bounds(&app, id);
    let copy = app
        .doc()
        .all_layers()
        .flat_map(|l| &l.shapes)
        .find(|s| s.id != id)
        .map(|s| s.bounds())
        .expect("copy");
    assert!(
        copy.x0 > 10.5 && copy.x0 < moved.x0 - 0.5,
        "{copy:?} {moved:?}"
    );
    assert_eq!(app.tool, Tool::Pick);
}

#[test]
fn shift_scales_from_the_centre_and_ctrl_mirrors() {
    let (ctx, mut app, id) = setup();
    // Shift: the right side goes out, the left side as far the other way.
    let e = handle(&app, Handle::E);
    drag(
        &ctx,
        &mut app,
        e,
        e + egui::vec2(60.0, 0.0),
        Modifiers::SHIFT,
        Vec::new(),
    );
    let b = bounds(&app, id);
    assert!(close(b.center().x, 35.0), "{b:?}");
    assert!(b.width() > 55.0);
    app.undo();
    // Ctrl, dragged past the other side: the mirror image at 100 %.
    let e = handle(&app, Handle::E);
    let w = handle(&app, Handle::W);
    let past = w - egui::vec2((e.x - w.x) * 0.8, 0.0);
    drag(&ctx, &mut app, e, past, Modifiers::CTRL, Vec::new());
    let b = bounds(&app, id);
    assert!(close(b.width(), 50.0), "{b:?}");
    assert!(close(b.x1, 10.0) && close(b.x0, -40.0), "{b:?}");
    let t = app
        .doc()
        .find_shape(id)
        .map(|s| s.transform)
        .expect("shape");
    assert!(t.determinant() < 0.0, "mirrored");
}

#[test]
fn rotate_mode_skews_with_the_sides_and_turns_about_the_moved_centre() {
    let (ctx, mut app, id) = setup();
    app.rotate_mode = true;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // The top handle slants the rectangle sideways; the bottom stays.
    let n = handle(&app, Handle::N);
    drag(
        &ctx,
        &mut app,
        n,
        n + egui::vec2(80.0, 0.0),
        Modifiers::NONE,
        Vec::new(),
    );
    let b = bounds(&app, id);
    assert!(b.width() > 55.0, "slanted: {b:?}");
    assert!(close(b.y0, 10.0) && close(b.y1, 40.0), "{b:?}");
    app.undo();
    // Move the centre of rotation onto the lower left corner, then turn.
    app.rotate_mode = true;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let pivot = app.view.to_screen(Point::new(35.0, 25.0));
    let corner = app.view.to_screen(Point::new(10.0, 10.0));
    drag(&ctx, &mut app, pivot, corner, Modifiers::NONE, Vec::new());
    assert!(app.rotate_mode, "moving the centre keeps rotate mode");
    let c = app.rotation_center().expect("centre");
    assert!(close(c.x, 10.0) && close(c.y, 10.0), "{c:?}");
    // A quarter turn about (10, 10) with Ctrl (15 degree steps).
    let ne = handle(&app, Handle::NE);
    let centre = app.view.to_screen(c);
    let r = ne - centre;
    let quarter = centre + egui::vec2(-r.y, r.x);
    drag(&ctx, &mut app, ne, quarter, Modifiers::CTRL, Vec::new());
    let b = bounds(&app, id);
    // The corner on the centre stays where it was.
    assert!(
        close(b.x0, 10.0) || close(b.x1, 10.0),
        "turned about the corner: {b:?}"
    );
    assert!(close(b.width(), 30.0) && close(b.height(), 50.0), "{b:?}");
}

#[test]
fn handle_modifiers_shape_the_scale() {
    let app = App::headless();
    let b = Rect::new(0.0, 0.0, 40.0, 20.0);
    let scale = |h: Handle, anchor: Point, current: Point, mods: HandleMods| {
        app.preview_transform_of(&Drag::Scale {
            handle: h,
            anchor,
            start_bounds: b,
            current,
            grab: tracedraw_core::geometry::Vec2::ZERO,
            mods,
        })
        .expect("transform")
    };
    let none = HandleMods::default();
    let shift = HandleMods {
        from_center: true,
        ..none
    };
    let ctrl = HandleMods {
        steps: true,
        ..none
    };
    let alt = HandleMods { free: true, ..none };
    let at = |t: Affine, x: f64, y: f64| t * Point::new(x, y);
    // Plain: the left side stays.
    let t = scale(
        Handle::E,
        Point::new(0.0, 10.0),
        Point::new(80.0, 10.0),
        none,
    );
    assert!(close(at(t, 40.0, 10.0).x, 80.0) && close(at(t, 0.0, 10.0).x, 0.0));
    // Shift: about the centre (20).
    let t = scale(
        Handle::E,
        Point::new(0.0, 10.0),
        Point::new(60.0, 10.0),
        shift,
    );
    assert!(close(at(t, 40.0, 10.0).x, 60.0) && close(at(t, 0.0, 10.0).x, -20.0));
    // Ctrl: whole steps; past the anchor the mirror image at 100 %.
    let t = scale(
        Handle::E,
        Point::new(0.0, 10.0),
        Point::new(-35.0, 10.0),
        ctrl,
    );
    assert!(close(at(t, 40.0, 10.0).x, -40.0));
    let t = scale(
        Handle::E,
        Point::new(0.0, 10.0),
        Point::new(70.0, 10.0),
        ctrl,
    );
    assert!(close(at(t, 40.0, 10.0).x, 80.0));
    // Ctrl and Shift: steps about the centre.
    let both = HandleMods {
        from_center: true,
        steps: true,
        ..none
    };
    let t = scale(
        Handle::E,
        Point::new(0.0, 10.0),
        Point::new(-15.0, 10.0),
        both,
    );
    assert!(close(at(t, 40.0, 10.0).x, -20.0));
    // A corner keeps the proportions unless Alt frees it, by where the
    // pointer falls along the box's diagonal (#383): (2 * 40² + 1 * 20²) /
    // (40² + 20²) = 1.8; on the diagonal the corner follows the pointer.
    let t = scale(
        Handle::NE,
        Point::new(0.0, 0.0),
        Point::new(80.0, 20.0),
        none,
    );
    assert!(close(at(t, 40.0, 20.0).y, 36.0) && close(at(t, 40.0, 20.0).x, 72.0));
    let t = scale(
        Handle::NE,
        Point::new(0.0, 0.0),
        Point::new(60.0, 30.0),
        none,
    );
    assert!(close(at(t, 40.0, 20.0).x, 60.0) && close(at(t, 40.0, 20.0).y, 30.0));
    // On a wide, low box a 1 mm move up grows it less than a 1 mm move to
    // the right: the long side sets the pace (with the larger of the two
    // ratios the move up grew it twice as fast).
    let up = scale(
        Handle::NE,
        Point::new(0.0, 0.0),
        Point::new(40.0, 21.0),
        none,
    );
    let right = scale(
        Handle::NE,
        Point::new(0.0, 0.0),
        Point::new(41.0, 20.0),
        none,
    );
    let k = |t: Affine| t.as_coeffs()[0];
    assert!(k(up) < k(right) && k(up) > 1.0, "{} {}", k(up), k(right));
    let t = scale(
        Handle::NE,
        Point::new(0.0, 0.0),
        Point::new(80.0, 20.0),
        alt,
    );
    assert!(close(at(t, 40.0, 20.0).x, 80.0) && close(at(t, 40.0, 20.0).y, 20.0));
}

#[test]
fn side_handles_skew() {
    let b = Rect::new(0.0, 0.0, 40.0, 20.0);
    let none = HandleMods::default();
    let at = |t: Affine, x: f64, y: f64| t * Point::new(x, y);
    // Top handle: the top moves by the drag, the bottom stays.
    let t = skew_transform(Handle::N, b, Vec2::new(20.0, 0.0), none, 15.0).expect("skew");
    assert!(close(at(t, 0.0, 20.0).x, 20.0) && close(at(t, 0.0, 0.0).x, 0.0));
    // Bottom handle: the other way round.
    let t = skew_transform(Handle::S, b, Vec2::new(20.0, 0.0), none, 15.0).expect("skew");
    assert!(close(at(t, 0.0, 0.0).x, 20.0) && close(at(t, 0.0, 20.0).x, 0.0));
    // Right handle: up and down.
    let t = skew_transform(Handle::E, b, Vec2::new(0.0, 10.0), none, 15.0).expect("skew");
    assert!(close(at(t, 40.0, 0.0).y, 10.0) && close(at(t, 0.0, 0.0).y, 0.0));
    // Shift: about the centre line.
    let shift = HandleMods {
        from_center: true,
        ..none
    };
    let t = skew_transform(Handle::N, b, Vec2::new(20.0, 0.0), shift, 15.0).expect("skew");
    assert!(close(at(t, 0.0, 20.0).x, 20.0) && close(at(t, 0.0, 0.0).x, -20.0));
    // Ctrl: the slant in 15 degree steps (37 degrees becomes 30).
    let ctrl = HandleMods {
        steps: true,
        ..none
    };
    let t = skew_transform(Handle::N, b, Vec2::new(15.0, 0.0), ctrl, 15.0).expect("skew");
    let x = at(t, 0.0, 20.0).x;
    assert!((x - 20.0 * 30f64.to_radians().tan()).abs() < 1e-6, "{x}");
    // Corners and flat boxes make none.
    assert!(skew_transform(Handle::NE, b, Vec2::new(5.0, 0.0), none, 15.0).is_none());
    let flat = Rect::new(0.0, 0.0, 40.0, 0.0);
    assert!(skew_transform(Handle::N, flat, Vec2::new(5.0, 0.0), none, 15.0).is_none());
}

#[test]
fn the_pointer_shows_what_a_press_would_do() {
    let (ctx, mut app, id) = setup();
    let cursor = |app: &App, at: Pos2| for_tool(app, false, false, false, Some(at));
    // Scale handles: resize arrows along their direction.
    for h in [
        Handle::N,
        Handle::S,
        Handle::E,
        Handle::W,
        Handle::NE,
        Handle::NW,
        Handle::SE,
        Handle::SW,
    ] {
        assert_eq!(
            cursor(&app, handle(&app, h)),
            ToolCursor::System(resize_icon(h)),
            "{h:?}"
        );
    }
    // The selected object: the move arrows; elsewhere the plain arrow.
    let inside = app.view.to_screen(Point::new(35.0, 25.0));
    assert_eq!(
        cursor(&app, inside),
        ToolCursor::System(egui::CursorIcon::Move)
    );
    let empty = app.view.to_screen(Point::new(150.0, 250.0));
    assert_eq!(
        cursor(&app, empty),
        ToolCursor::System(egui::CursorIcon::Default)
    );
    // Rotate mode: corners turn, sides slant, the centre moves.
    app.rotate_mode = true;
    assert_eq!(cursor(&app, handle(&app, Handle::NE)), ToolCursor::Rotate);
    assert_eq!(
        cursor(&app, handle(&app, Handle::N)),
        ToolCursor::Skew { horizontal: true }
    );
    assert_eq!(
        cursor(&app, handle(&app, Handle::E)),
        ToolCursor::Skew { horizontal: false }
    );
    assert_eq!(
        cursor(&app, inside),
        ToolCursor::System(egui::CursorIcon::Move)
    );
    app.rotate_mode = false;
    // The Shape tool: its arrow, marked over a node and over a segment of
    // a curve.
    app.select(vec![id]);
    app.convert_to_curves();
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let node = app.view.to_screen(Point::new(10.0, 10.0));
    let segment = app.view.to_screen(Point::new(35.0, 10.0));
    assert_eq!(cursor(&app, node), ToolCursor::ShapeArrow(ShapeMark::Node));
    assert_eq!(
        cursor(&app, segment),
        ToolCursor::ShapeArrow(ShapeMark::Segment)
    );
    assert_eq!(
        cursor(&app, empty),
        ToolCursor::ShapeArrow(ShapeMark::Nothing)
    );
    // The handles framing selected nodes: resize arrows, or turning and
    // slant arrows.
    app.select_all_nodes();
    app.node_transform = crate::node_edit::NodeTransformMode::StretchScale;
    let at = |app: &App, h: Handle| {
        let p = app
            .node_transform_handles()
            .into_iter()
            .find(|(k, _)| *k == h)
            .map(|(_, p)| p)
            .expect("handle");
        app.view.to_screen(p)
    };
    assert_eq!(
        cursor(&app, at(&app, Handle::E)),
        ToolCursor::System(resize_icon(Handle::E))
    );
    app.node_transform = crate::node_edit::NodeTransformMode::RotateSkew;
    assert_eq!(cursor(&app, at(&app, Handle::NE)), ToolCursor::Rotate);
    assert_eq!(
        cursor(&app, at(&app, Handle::S)),
        ToolCursor::Skew { horizontal: true }
    );
    app.node_transform = crate::node_edit::NodeTransformMode::None;
    // While dragging, the pointer keeps the drag's shape.
    app.set_tool(Tool::Pick);
    app.drag = Drag::Rotate {
        center: Point::new(0.0, 0.0),
        start_angle: 0.0,
        current_angle: 0.0,
    };
    assert_eq!(cursor(&app, empty), ToolCursor::Rotate);
    // Moving: the arrow with a dotted box; once a right click makes the
    // move leave the original, the box gets a plus.
    app.drag = Drag::Move {
        start: Point::ZERO,
        last: Point::ZERO,
        total: Vec2::ZERO,
        start_bounds: Rect::ZERO,
        source: None,
    };
    assert_eq!(cursor(&app, empty), ToolCursor::DragMove { copy: false });
    app.drag_copy = true;
    assert_eq!(cursor(&app, empty), ToolCursor::DragMove { copy: true });
    app.drag_copy = false;
    app.drag = Drag::None;
}

impl App {
    /// No context menu or drop menu was left open by a right click
    /// during a drag.
    fn dialog_is_none_or_menu_closed(&self) -> bool {
        self.context_menu.is_none() && self.drop_menu.is_none()
    }
}

#[test]
fn the_status_bar_tells_how_far_how_much_and_what_angle() {
    let (_ctx, mut app, _id) = setup();
    let info = |app: &App| crate::ui::status::drag_info(app).unwrap_or_default();
    assert!(info(&app).is_empty(), "nothing while not dragging");
    app.drag = Drag::Move {
        start: Point::new(0.0, 0.0),
        last: Point::new(0.0, 0.0),
        total: Vec2::new(30.0, 40.0),
        start_bounds: Rect::new(10.0, 10.0, 60.0, 40.0),
        source: None,
    };
    let text = info(&app);
    assert!(text.contains("50.000") && text.contains("53.1"), "{text}");
    app.drag = Drag::Scale {
        handle: Handle::E,
        anchor: Point::new(10.0, 25.0),
        start_bounds: Rect::new(10.0, 10.0, 60.0, 40.0),
        current: Point::new(85.0, 25.0),
        grab: tracedraw_core::geometry::Vec2::ZERO,
        mods: HandleMods::default(),
    };
    let text = info(&app);
    assert!(text.contains("150.0") && text.contains("100.0"), "{text}");
    app.drag = Drag::Rotate {
        center: Point::new(0.0, 0.0),
        start_angle: 0.0,
        current_angle: 30f64.to_radians(),
    };
    assert!(info(&app).contains("30.0"), "{}", info(&app));
    app.drag = Drag::Skew {
        handle: Handle::N,
        start_bounds: Rect::new(10.0, 10.0, 60.0, 40.0),
        start: Point::new(35.0, 40.0),
        current: Point::new(65.0, 40.0),
        mods: HandleMods::default(),
    };
    assert!(info(&app).contains("45.0"), "{}", info(&app));
    // A copy says so.
    app.drag_copy = true;
    assert!(info(&app).contains(&crate::i18n::tr("status.drag_copy_short")));
    app.drag = Drag::None;
    app.drag_copy = false;
}

#[test]
fn objects_dropped_on_the_document_palette_give_their_colours() {
    let (ctx, mut app, id) = setup();
    app.settings.palette.show_document = true;
    for _ in 0..2 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let row = app.doc_palette_rect.expect("the document palette shows");
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    drag(
        &ctx,
        &mut app,
        from,
        row.center(),
        Modifiers::NONE,
        Vec::new(),
    );
    // The object stays; its fill joined the document palette.
    assert!(close(bounds(&app, id).x0, 10.0), "{:?}", bounds(&app, id));
    assert!(app
        .doc()
        .palette
        .contains(&tracedraw_core::Color::rgb8(200, 30, 30)));
}

fn click_at(ctx: &egui::Context, app: &mut App, at: Pos2, mods: Modifiers) {
    frame(ctx, app, vec![Event::PointerMoved(at)], mods);
    frame(
        ctx,
        app,
        vec![button(at, true, PointerButton::Primary, mods)],
        mods,
    );
    frame(
        ctx,
        app,
        vec![button(at, false, PointerButton::Primary, mods)],
        mods,
    );
    // Past the double-click time (frames advance the input clock by a
    // sixtieth of a second each), so the next click is a click of its own.
    for _ in 0..20 {
        frame(ctx, app, Vec::new(), mods);
    }
}

fn last_path(app: &App) -> tracedraw_core::BezPath {
    app.doc()
        .all_layers()
        .flat_map(|l| &l.shapes)
        .filter_map(|s| match &s.kind {
            ShapeKind::Path { .. } => Some(s.page_path()),
            _ => None,
        })
        .last()
        .expect("a curve")
}

#[test]
fn ctrl_keeps_lines_to_the_constrain_angle() {
    use crate::interaction::constrain_from;
    let o = Point::new(0.0, 0.0);
    // 5 degrees off the horizontal comes back to it, at the same reach
    // along it.
    let p = constrain_from(o, Point::new(10.0, 5f64.to_radians().tan() * 10.0), 15.0);
    assert!(p.y.abs() < 1e-9 && (p.x - 10.0).abs() < 1e-9, "{p:?}");
    // 40 degrees goes to 45.
    let a = 40f64.to_radians();
    let p = constrain_from(o, Point::new(a.cos() * 10.0, a.sin() * 10.0), 15.0);
    assert!((p.y.atan2(p.x).to_degrees() - 45.0).abs() < 1e-9);
    assert_eq!(constrain_from(o, o, 15.0), o);

    // Freehand: two clicks make a straight line; with Ctrl a level one.
    let (ctx, mut app, _id) = setup();
    app.set_tool(Tool::Freehand);
    let a = app.view.to_screen(Point::new(100.0, 100.0));
    let b = app.view.to_screen(Point::new(160.0, 104.0));
    click_at(&ctx, &mut app, a, Modifiers::NONE);
    click_at(&ctx, &mut app, b, Modifiers::CTRL);
    let path = last_path(&app);
    let pts: Vec<Point> = path
        .elements()
        .iter()
        .filter_map(|e| match e {
            tracedraw_core::geometry::PathEl::MoveTo(p)
            | tracedraw_core::geometry::PathEl::LineTo(p) => Some(*p),
            _ => None,
        })
        .collect();
    assert_eq!(pts.len(), 2, "{pts:?}");
    assert!((pts[0].y - pts[1].y).abs() < 1e-6, "level: {pts:?}");
    assert!(pts[1].x > pts[0].x + 50.0);

    // Polyline: Ctrl keeps the next segment to 15 degree steps.
    app.set_tool(Tool::Polyline);
    let a = app.view.to_screen(Point::new(100.0, 150.0));
    let b = app.view.to_screen(Point::new(150.0, 196.0));
    click_at(&ctx, &mut app, a, Modifiers::NONE);
    click_at(&ctx, &mut app, b, Modifiers::CTRL);
    app.finish_curve();
    let path = last_path(&app);
    let pts: Vec<Point> = path
        .elements()
        .iter()
        .filter_map(|e| match e {
            tracedraw_core::geometry::PathEl::MoveTo(p)
            | tracedraw_core::geometry::PathEl::LineTo(p) => Some(*p),
            _ => None,
        })
        .collect();
    let d = pts[1] - pts[0];
    assert!(
        (d.y.atan2(d.x).to_degrees() - 45.0).abs() < 1e-6,
        "45 degrees: {pts:?}"
    );
}

/// Two filled 20 x 20 mm squares grouped (at x 100 and 140, y 100) and a
/// third square on its own at (100, 150), nothing selected.
fn grouped() -> (egui::Context, App, [ShapeId; 3], ShapeId) {
    let (ctx, mut app, first) = setup();
    app.select(vec![first]);
    app.delete_selection();
    let mut ids = Vec::new();
    for (x, y) in [(100.0, 100.0), (140.0, 100.0), (100.0, 150.0)] {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(x, y, x + 20.0, y + 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
            30, 30, 200,
        )));
        ids.push(id);
    }
    app.select(vec![ids[0], ids[1]]);
    app.group_selection();
    let group = app.selection[0];
    // The group is moved, so its objects' space is not the page's.
    app.run(tracedraw_core::Command::TransformShapes {
        shapes: vec![group],
        transform: Affine::translate((5.0, 0.0)),
    });
    app.select(Vec::new());
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    (ctx, app, [ids[0], ids[1], ids[2]], group)
}

fn page_bounds(app: &App, id: ShapeId) -> Rect {
    app.doc()
        .shape_on_page(id)
        .map(|s| s.bounds())
        .expect("shape")
}

#[test]
fn ctrl_click_chooses_an_object_inside_a_group() {
    let (ctx, mut app, [a, b, _c], group) = grouped();
    let on_a = app.view.to_screen(Point::new(115.0, 110.0));
    // A plain click takes the group.
    click_at(&ctx, &mut app, on_a, Modifiers::NONE);
    assert_eq!(app.selection, vec![group]);
    // Ctrl+click: the square inside it, with round handles and the status
    // bar saying where it is.
    click_at(&ctx, &mut app, on_a, Modifiers::CTRL);
    assert_eq!(app.selection, vec![a]);
    assert!(app.selection_nested());
    assert!(crate::ui::status::object_info(&app).contains("in a group"));
    let before_b = page_bounds(&app, b);
    // Dragging it moves that square only, by the drag on the page.
    let before = page_bounds(&app, a);
    let to = app.view.to_screen(Point::new(115.0, 130.0));
    drag(&ctx, &mut app, on_a, to, Modifiers::NONE, Vec::new());
    let after = page_bounds(&app, a);
    assert!(close(after.y0 - before.y0, 20.0), "{before:?} {after:?}");
    assert!(close(after.x0, before.x0));
    assert_eq!(page_bounds(&app, b), before_b);
    assert_eq!(app.doc().top_level_of(a), Some(group));
    // Undo and redo keep it chosen.
    app.undo();
    assert_eq!(app.selection, vec![a]);
    assert!(close(page_bounds(&app, a).y0, before.y0));
    app.redo();
    assert_eq!(app.selection, vec![a]);
    // A colour reaches it alone.
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
        0, 200, 0,
    )));
    let fill_of = |app: &App, id| app.doc().find_shape(id).map(|s| s.fill.clone());
    assert_ne!(fill_of(&app, a), fill_of(&app, b));
    // Ctrl+Shift+click adds the other square; the group is never selected
    // with its own objects.
    let on_b = app.view.to_screen(Point::new(155.0, 110.0));
    click_at(&ctx, &mut app, on_b, Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(app.selection, vec![a, b]);
    app.select_add(group);
    assert_eq!(app.selection, vec![group]);
    // Delete takes the chosen square out of the group.
    app.select(vec![a]);
    app.delete_selection();
    assert!(app.doc().find_shape(a).is_none());
    assert_eq!(app.doc().top_level_of(b), Some(group));
}

#[test]
fn alt_click_digs_to_the_object_beneath() {
    let (ctx, mut app, [_a, _b, c], _group) = grouped();
    // A fourth square over the third one.
    let top = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(105.0, 155.0, 125.0, 175.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.select(vec![top]);
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
        200, 200, 0,
    )));
    let both = app.view.to_screen(Point::new(110.0, 160.0));
    click_at(&ctx, &mut app, both, Modifiers::NONE);
    assert_eq!(app.selection, vec![top]);
    click_at(&ctx, &mut app, both, Modifiers::ALT);
    assert_eq!(app.selection, vec![c]);
    // Once more: round to the top again.
    click_at(&ctx, &mut app, both, Modifiers::ALT);
    assert_eq!(app.selection, vec![top]);
    assert!(!app.rotate_mode);
}

#[test]
fn alt_marquees_take_what_they_touch() {
    let (ctx, mut app, [_a, _b, c], group) = grouped();
    // Across the lone square and the group's first square, holding
    // neither wholly.
    let from = app.view.to_screen(Point::new(90.0, 112.0));
    let to = app.view.to_screen(Point::new(112.0, 158.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    assert!(app.selection.is_empty());
    drag(&ctx, &mut app, from, to, Modifiers::ALT, Vec::new());
    let mut sel = app.selection.clone();
    sel.sort_by_key(|s| s.raw());
    let mut want = vec![group, c];
    want.sort_by_key(|s| s.raw());
    assert_eq!(sel, want);
    // Round all three: everything.
    app.select(Vec::new());
    let from = app.view.to_screen(Point::new(90.0, 90.0));
    let to = app.view.to_screen(Point::new(180.0, 180.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    assert_eq!(app.selection.len(), 2);
}

#[test]
fn the_touching_option_swaps_what_alt_does() {
    let (ctx, mut app, [_a, _b, c], group) = grouped();
    app.settings.marquee_touching = true;
    let from = app.view.to_screen(Point::new(90.0, 112.0));
    let to = app.view.to_screen(Point::new(112.0, 158.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    let mut sel = app.selection.clone();
    sel.sort_by_key(|s| s.raw());
    let mut want = vec![group, c];
    want.sort_by_key(|s| s.raw());
    assert_eq!(sel, want);
    app.select(Vec::new());
    drag(&ctx, &mut app, from, to, Modifiers::ALT, Vec::new());
    assert!(app.selection.is_empty());
}

/// The screen points of a lasso through `pts` (page), in small steps as a
/// hand draws it.
fn lasso_path(app: &App, pts: &[Point]) -> Vec<Pos2> {
    let mut out = Vec::new();
    for w in pts.windows(2) {
        for i in 0..8 {
            let t = i as f64 / 8.0;
            out.push(app.view.to_screen(w[0] + (w[1] - w[0]) * t));
        }
    }
    if let Some(last) = pts.last() {
        out.push(app.view.to_screen(*last));
    }
    out
}

#[test]
fn the_freehand_pick_lassoes_and_has_the_pick_tools_handles() {
    let (ctx, mut app, [_a, _b, c], group) = grouped();
    app.set_tool(Tool::FreeformPick);
    // A loop round the lone square only.
    let pts = [
        Point::new(90.0, 140.0),
        Point::new(130.0, 140.0),
        Point::new(130.0, 180.0),
        Point::new(90.0, 180.0),
        Point::new(90.0, 141.0),
    ];
    let screen = lasso_path(&app, &pts);
    let lasso = |ctx: &egui::Context, app: &mut App, mods: Modifiers| {
        frame(ctx, app, vec![Event::PointerMoved(screen[0])], mods);
        frame(
            ctx,
            app,
            vec![button(screen[0], true, PointerButton::Primary, mods)],
            mods,
        );
        for p in &screen[1..] {
            frame(ctx, app, vec![Event::PointerMoved(*p)], mods);
        }
        frame(
            ctx,
            app,
            vec![button(
                *screen.last().expect("point"),
                false,
                PointerButton::Primary,
                mods,
            )],
            mods,
        );
        frame(ctx, app, Vec::new(), Modifiers::NONE);
    };
    lasso(&ctx, &mut app, Modifiers::NONE);
    assert_eq!(app.selection, vec![c]);
    // The handles scale as with the Pick tool.
    let before = page_bounds(&app, c);
    let ne = handle(&app, Handle::NE);
    drag(
        &ctx,
        &mut app,
        ne,
        ne + egui::vec2(40.0, -40.0),
        Modifiers::NONE,
        Vec::new(),
    );
    assert!(page_bounds(&app, c).width() > before.width() + 1.0);
    // A loop through the group's first square takes the group with Alt.
    app.select(Vec::new());
    let screen2 = lasso_path(
        &app,
        &[
            Point::new(110.0, 90.0),
            Point::new(130.0, 90.0),
            Point::new(130.0, 130.0),
            Point::new(110.0, 130.0),
        ],
    );
    let mods = Modifiers::ALT;
    frame(&ctx, &mut app, vec![Event::PointerMoved(screen2[0])], mods);
    frame(
        &ctx,
        &mut app,
        vec![button(screen2[0], true, PointerButton::Primary, mods)],
        mods,
    );
    for p in &screen2[1..] {
        frame(&ctx, &mut app, vec![Event::PointerMoved(*p)], mods);
    }
    frame(
        &ctx,
        &mut app,
        vec![button(
            *screen2.last().expect("point"),
            false,
            PointerButton::Primary,
            mods,
        )],
        mods,
    );
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert_eq!(app.selection, vec![group]);
}

#[test]
fn clicking_the_selection_or_its_centre_mark_turns_the_handles() {
    let (ctx, mut app, [_a, _b, c], group) = grouped();
    app.select(vec![group, c]);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // Any selected object toggles rotate mode for the whole selection.
    let on_c = app.view.to_screen(Point::new(110.0, 160.0));
    click_at(&ctx, &mut app, on_c, Modifiers::NONE);
    assert!(app.rotate_mode);
    assert_eq!(app.selection.len(), 2);
    // A moved centre of rotation stays where it was put when rotate mode
    // goes off and on again.
    let moved = Point::new(105.0, 105.0);
    app.set_pivot(moved);
    click_at(&ctx, &mut app, on_c, Modifiers::NONE);
    assert!(!app.rotate_mode);
    click_at(&ctx, &mut app, on_c, Modifiers::NONE);
    assert!(app.rotate_mode);
    assert_eq!(app.rotation_center(), Some(moved));
    // A double click on it puts it back in the middle.
    let middle = app.selection_bounds().expect("bounds").center();
    app.reset_pivot();
    assert_eq!(app.rotation_center(), Some(middle));
    // The centre mark of an outline-only ring: it lies on nothing (with
    // objects not treated as filled), yet a click there turns the handles
    // and a drag moves the ring.
    app.settings.treat_all_filled = false;
    app.select(Vec::new());
    let ring = app
        .new_shape(ShapeKind::Ellipse {
            rect: Rect::new(20.0, 200.0, 60.0, 240.0),
            arc: None,
        })
        .expect("ellipse");
    app.select(vec![ring]);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let mark = app.view.to_screen(Point::new(40.0, 220.0));
    assert_eq!(app.hit_test(Point::new(40.0, 220.0)), None);
    assert_eq!(
        for_tool(&app, false, false, false, Some(mark)),
        ToolCursor::System(egui::CursorIcon::Move)
    );
    let before = page_bounds(&app, ring);
    let to = app.view.to_screen(Point::new(70.0, 220.0));
    drag(&ctx, &mut app, mark, to, Modifiers::NONE, Vec::new());
    assert!(close(page_bounds(&app, ring).x0 - before.x0, 30.0));
    assert_eq!(app.selection, vec![ring]);
    let moved = app.view.to_screen(Point::new(70.0, 220.0));
    click_at(&ctx, &mut app, moved, Modifiers::NONE);
    assert!(app.rotate_mode);
}

#[test]
fn the_pointer_shows_moves_for_objects_and_guidelines() {
    let (ctx, mut app, [a, _b, c], group) = grouped();
    let cursor = |app: &App, at: Pos2| for_tool(app, false, false, false, Some(at));
    let arrow = ToolCursor::System(egui::CursorIcon::Default);
    let moving = ToolCursor::System(egui::CursorIcon::Move);
    // Unselected objects keep the plain arrow (#885): a press there picks
    // them first.
    assert!(app.selection.is_empty());
    let on_a = app.view.to_screen(Point::new(115.0, 110.0));
    let on_b = app.view.to_screen(Point::new(155.0, 110.0));
    let on_c = app.view.to_screen(Point::new(110.0, 160.0));
    assert_eq!(cursor(&app, on_a), arrow);
    assert_eq!(cursor(&app, on_c), arrow);
    // The selected group: the move arrows over it, the arrow over the
    // object beside it.
    app.select(vec![group]);
    assert_eq!(cursor(&app, on_a), moving);
    assert_eq!(cursor(&app, on_b), moving);
    assert_eq!(cursor(&app, on_c), arrow);
    // An object chosen inside the group: only it shows the move arrows,
    // as only it moves; a press on its neighbour in the group picks the
    // group.
    assert!(app.doc().is_nested(a));
    app.select(vec![a]);
    assert_eq!(cursor(&app, on_a), moving);
    assert_eq!(cursor(&app, on_b), arrow);
    // With the cross hair setting, the cross hair over the unselected one.
    app.settings.crosshair_cursor = true;
    assert_eq!(cursor(&app, on_b), ToolCursor::Cross);
    assert_eq!(cursor(&app, on_a), moving);
    app.settings.crosshair_cursor = false;
    // A drag from the unselected object still picks it and moves it; it is
    // then selected and shows the move arrows.
    let before = page_bounds(&app, c);
    let to = app.view.to_screen(Point::new(110.0, 190.0));
    drag(&ctx, &mut app, on_c, to, Modifiers::NONE, Vec::new());
    assert_eq!(app.selection, vec![c]);
    assert!(close(page_bounds(&app, c).y0 - before.y0, 30.0));
    assert_eq!(cursor(&app, to), moving);
    assert_eq!(cursor(&app, on_a), arrow);
    // An unselected object over a guideline: the arrow, not the arrows
    // across the guideline, as the object takes the press.
    app.add_guide(tracedraw_core::document::Guide::horizontal(110.0));
    assert_eq!(cursor(&app, on_a), arrow);
    app.select(Vec::new());
    app.delete_guide(0);
    // Guidelines: arrows across them; a locked one keeps the arrow.
    app.add_guide(tracedraw_core::document::Guide::horizontal(250.0));
    app.add_guide(tracedraw_core::document::Guide::vertical(250.0));
    let on_h = app.view.to_screen(Point::new(200.0, 250.0));
    let on_v = app.view.to_screen(Point::new(250.0, 200.0));
    assert_eq!(
        cursor(&app, on_h),
        ToolCursor::System(egui::CursorIcon::ResizeVertical)
    );
    assert_eq!(
        cursor(&app, on_v),
        ToolCursor::System(egui::CursorIcon::ResizeHorizontal)
    );
    app.set_guides_locked(&[0], true);
    assert_eq!(
        cursor(&app, on_h),
        ToolCursor::System(egui::CursorIcon::Default)
    );
    // Dragging a new guideline out of a ruler.
    app.drag = Drag::NewGuide {
        horizontal: true,
        pos: Point::ZERO,
    };
    assert_eq!(
        crate::ui::cursors::ruler_drag_cursor(&app),
        Some(ToolCursor::System(egui::CursorIcon::ResizeVertical))
    );
    app.drag = Drag::None;
}

#[test]
fn a_right_click_in_the_shape_tool_takes_the_node_or_segment_under_it() {
    let (ctx, mut app, id) = setup();
    app.convert_to_curves();
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let right_click = |ctx: &egui::Context, app: &mut App, at: Pos2| {
        frame(ctx, app, vec![Event::PointerMoved(at)], Modifiers::NONE);
        frame(
            ctx,
            app,
            vec![button(at, true, PointerButton::Secondary, Modifiers::NONE)],
            Modifiers::NONE,
        );
        frame(
            ctx,
            app,
            vec![button(at, false, PointerButton::Secondary, Modifiers::NONE)],
            Modifiers::NONE,
        );
    };
    // The bottom side's middle: that segment's end node is chosen.
    let on_segment = app.view.to_screen(Point::new(35.0, 10.0));
    let (seg_shape, seg_index, _) = app.segment_at(Point::new(35.0, 10.0)).expect("segment");
    right_click(&ctx, &mut app, on_segment);
    assert!(app.context_menu.is_some());
    assert_eq!(app.node_selection, vec![(seg_shape, seg_index)]);
    assert_eq!(seg_shape, id);
    app.context_menu = None;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // A corner: that node.
    let corner = app.view.to_screen(Point::new(60.0, 40.0));
    let hit = app.node_at(Point::new(60.0, 40.0)).expect("node");
    right_click(&ctx, &mut app, corner);
    assert_eq!(app.node_selection, vec![(hit.shape, hit.index)]);
    app.context_menu = None;
}

#[test]
fn scale_handles_snap_to_other_objects_not_to_the_selection() {
    let (ctx, mut app, id) = setup();
    // A second rectangle whose left edge is at x = 80.
    let other = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(80.0, 10.0, 100.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.select(vec![id]);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // Drag the right side handle to a little short of the other's edge.
    let e = handle(&app, Handle::E);
    let short = app.view.to_screen(Point::new(80.0, 25.0)) - egui::vec2(2.0, 0.0);
    drag(&ctx, &mut app, e, short, Modifiers::NONE, Vec::new());
    let b = bounds(&app, id);
    assert!((b.x1 - 80.0).abs() < 1e-6, "{b:?}");
    assert!((b.x0 - 10.0).abs() < 1e-6);
    // With snapping off the handle goes where the pointer is.
    app.undo();
    app.snap.off = true;
    let e = handle(&app, Handle::E);
    drag(&ctx, &mut app, e, short, Modifiers::NONE, Vec::new());
    let b = bounds(&app, id);
    assert!((b.x1 - 80.0).abs() > 0.1, "{b:?}");
    assert!(app.doc().find_shape(other).is_some());
}

#[test]
fn plus_duplicates_in_place_and_edits_nodes_in_the_shape_tool() {
    let (ctx, mut app, id) = setup();
    let n = shape_count(&app);
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Plus, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert_eq!(shape_count(&app), n + 1);
    let copy = app.selection[0];
    assert_ne!(copy, id);
    assert_eq!(bounds(&app, copy), bounds(&app, id));
    // The Shape tool: + adds a node in the middle of the segment ending at
    // the chosen node, - deletes the chosen nodes.
    app.select(vec![id]);
    app.convert_to_curves();
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let count = |app: &App| {
        app.doc()
            .find_shape(id)
            .and_then(|s| match &s.kind {
                ShapeKind::Path { path, .. } => Some(tracedraw_core::nodes::nodes(path).len()),
                _ => None,
            })
            .expect("curve")
    };
    let before = count(&app);
    let hit = app.node_at(Point::new(60.0, 40.0)).expect("node");
    app.node_selection = vec![(hit.shape, hit.index)];
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Plus, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert_eq!(count(&app), before + 1);
    let hit = app.node_at(Point::new(60.0, 40.0)).expect("node");
    app.node_selection = vec![(hit.shape, hit.index)];
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Minus, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert_eq!(count(&app), before);
    // Nothing chosen: + zooms.
    app.node_selection.clear();
    let zoom = app.view.zoom;
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Plus, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert!(app.view.zoom > zoom);
}

#[test]
fn dragging_a_segment_bends_it_and_a_click_marks_where_a_node_goes() {
    let (ctx, mut app, id) = setup();
    app.convert_to_curves();
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let path_of = |app: &App| match app.doc().find_shape(id).map(|s| s.kind.clone()) {
        Some(ShapeKind::Path { path, .. }) => path,
        other => panic!("not a curve: {other:?}"),
    };
    let before = path_of(&app);
    // Drag the middle of the bottom side 10 mm down.
    let (_, index, t) = app.segment_at(Point::new(35.0, 10.0)).expect("segment");
    let from = app.view.to_screen(Point::new(35.0, 10.0));
    let to = app.view.to_screen(Point::new(35.0, 0.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    let bent = path_of(&app);
    let mid = tracedraw_core::nodes::point_on_segment(&bent, index, t).expect("point");
    assert!((mid - Point::new(35.0, 0.0)).hypot() < 0.3, "{mid:?}");
    // The corners stay; one undo puts the side back.
    let corners = |p: &tracedraw_core::BezPath| {
        tracedraw_core::nodes::nodes(p)
            .iter()
            .map(|n| n.pos)
            .collect::<Vec<_>>()
    };
    assert_eq!(corners(&bent), corners(&before));
    app.undo();
    assert_eq!(corners(&path_of(&app)), corners(&before));
    let mid = tracedraw_core::nodes::point_on_segment(&path_of(&app), index, t).expect("point");
    assert!((mid.y - 10.0).abs() < 1e-6);
    // A click on the right side chooses it and marks the spot; + puts a
    // node there.
    let n = corners(&path_of(&app)).len();
    let spot = Point::new(60.0, 32.0);
    let at = app.view.to_screen(spot);
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    let (shape, seg, _) = app.segment_spot.expect("spot");
    assert_eq!(shape, id);
    assert_eq!(app.node_selection, vec![(id, seg)]);
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Plus, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let after = corners(&path_of(&app));
    assert_eq!(after.len(), n + 1);
    assert!(after.iter().any(|q| (*q - spot).hypot() < 0.3), "{after:?}");
}

/// Two open curves for the join tests, the rectangle of `setup` gone.
fn two_open_curves(ctx: &egui::Context, app: &mut App, id: ShapeId) -> (ShapeId, ShapeId) {
    app.select(vec![id]);
    app.delete_selection();
    let mut a = tracedraw_core::BezPath::new();
    a.move_to((10.0, 10.0));
    a.curve_to((15.0, 20.0), (25.0, 20.0), (30.0, 10.0));
    let mut b = tracedraw_core::BezPath::new();
    b.move_to((44.0, 10.0));
    b.line_to((60.0, 30.0));
    let a = app
        .new_shape(ShapeKind::Path {
            path: a,
            closed: false,
        })
        .expect("curve a");
    let b = app
        .new_shape(ShapeKind::Path {
            path: b,
            closed: false,
        })
        .expect("curve b");
    app.select(vec![a, b]);
    app.set_tool(Tool::Shape);
    frame(ctx, app, Vec::new(), Modifiers::NONE);
    (a, b)
}

fn curve_path(app: &App, id: ShapeId) -> tracedraw_core::BezPath {
    match app.doc().find_shape(id).map(|s| s.kind.clone()) {
        Some(ShapeKind::Path { path, .. }) => path,
        other => panic!("not a curve: {other:?}"),
    }
}

#[test]
fn join_fuses_two_end_nodes_of_two_curves_without_a_line() {
    use tracedraw_core::geometry::PathEl;
    let (ctx, mut app, id) = setup();
    let (a, b) = two_open_curves(&ctx, &mut app, id);
    // The end of a (element 1) and the start of b (element 0).
    app.node_selection = vec![(a, 1), (b, 0)];
    app.join_selected_nodes();
    // One curve is left, one subpath through the joined node, no line.
    assert!(app.doc().find_shape(b).is_none());
    let p = curve_path(&app, a);
    assert_eq!(
        p.elements(),
        &[
            PathEl::MoveTo(Point::new(10.0, 10.0)),
            PathEl::CurveTo(
                Point::new(15.0, 20.0),
                Point::new(32.0, 20.0),
                Point::new(37.0, 10.0)
            ),
            PathEl::LineTo(Point::new(60.0, 30.0)),
        ]
    );
    // One undo step brings both curves back.
    app.undo();
    assert!(app.doc().find_shape(b).is_some());
    assert_eq!(tracedraw_core::nodes::nodes(&curve_path(&app, a)).len(), 2);
}

#[test]
fn join_of_the_two_ends_of_one_curve_closes_it_at_one_node() {
    use tracedraw_core::geometry::PathEl;
    let (ctx, mut app, id) = setup();
    let (a, _) = two_open_curves(&ctx, &mut app, id);
    let start = app.node_at(Point::new(10.0, 10.0)).expect("start");
    let end = app.node_at(Point::new(30.0, 10.0)).expect("end");
    app.node_selection = vec![(start.shape, start.index), (end.shape, end.index)];
    app.join_selected_nodes();
    let p = curve_path(&app, a);
    assert!(
        matches!(p.elements().last(), Some(PathEl::ClosePath)),
        "{p:?}"
    );
    // The two ends are one node now: the curve has a single node.
    assert_eq!(tracedraw_core::nodes::nodes(&p).len(), 1);
    assert!(matches!(
        app.doc().find_shape(a).map(|s| &s.kind),
        Some(ShapeKind::Path { closed: true, .. })
    ));
}

/// A node dragged with the Shape tool snaps to the other nodes of its own
/// curve, not only to other objects (#673).
#[test]
fn a_dragged_node_snaps_to_another_node_of_its_own_curve() {
    let (ctx, mut app, id) = setup();
    app.convert_to_curves();
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // The top right corner dropped 3 px off the bottom left one.
    let from = app.view.to_screen(Point::new(60.0, 10.0));
    let to = app.view.to_screen(Point::new(10.0, 40.0)) + egui::vec2(3.0, -3.0);
    click_at(&ctx, &mut app, from, Modifiers::NONE);
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    let p = match app.doc().find_shape(id).map(|s| s.kind.clone()) {
        Some(ShapeKind::Path { path, .. }) => path,
        other => panic!("not a curve: {other:?}"),
    };
    let on_corner = tracedraw_core::nodes::nodes(&p)
        .iter()
        .filter(|n| (n.pos - Point::new(10.0, 40.0)).hypot() < 1e-6)
        .count();
    assert_eq!(on_corner, 2, "{p:?}");
}

/// With Ctrl the node snaps where the move kept on its axis takes it: a
/// middle node dragged right with the pointer below the line lands on
/// the curve's own end node (#673).
#[test]
fn a_ctrl_dragged_node_snaps_to_its_own_curve_on_its_axis() {
    use tracedraw_core::geometry::BezPath;
    let (ctx, mut app, _) = setup();
    let mut p = BezPath::new();
    p.move_to((20.0, 60.0));
    p.line_to((60.0, 60.0));
    p.line_to((100.0, 60.0));
    let curve = app
        .new_shape(ShapeKind::Path {
            path: p,
            closed: false,
        })
        .expect("curve");
    app.select(vec![curve]);
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let from = app.view.to_screen(Point::new(60.0, 60.0));
    // 1 mm short of the end node along the line, 15 px below it: out of
    // reach of the end node unless the move is kept on its axis first.
    let to = app.view.to_screen(Point::new(99.0, 60.0)) + egui::vec2(0.0, 15.0);
    click_at(&ctx, &mut app, from, Modifiers::NONE);
    drag(&ctx, &mut app, from, to, Modifiers::CTRL, Vec::new());
    let p = match app.doc().find_shape(curve).map(|s| s.kind.clone()) {
        Some(ShapeKind::Path { path, .. }) => path,
        other => panic!("not a curve: {other:?}"),
    };
    let ns = tracedraw_core::nodes::nodes(&p);
    assert!(
        (ns[1].pos - Point::new(100.0, 60.0)).hypot() < 1e-6,
        "{:?}",
        ns[1].pos
    );
}

/// A dragged node does not snap to where it started, nor to the segments
/// that move with it (#673).
#[test]
fn a_dragged_node_does_not_snap_to_itself_or_its_moving_segments() {
    let (ctx, mut app, id) = setup();
    app.convert_to_curves();
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let node_at = |app: &App| {
        let p = match app.doc().find_shape(id).map(|s| s.kind.clone()) {
            Some(ShapeKind::Path { path, .. }) => path,
            other => panic!("not a curve: {other:?}"),
        };
        tracedraw_core::nodes::nodes(&p)
            .iter()
            .map(|n| n.pos)
            .min_by(|a, b| {
                let d = |q: &Point| (*q - Point::new(60.0, 10.0)).hypot();
                d(a).total_cmp(&d(b))
            })
            .expect("a node")
    };
    // The top right node, 6 px right and down of where it was (inside
    // the 10 px snapping radius of its start); then on to 6 px off the
    // middle of the top side it bends.
    let off = egui::vec2(6.0, 6.0);
    for target in [Point::new(60.0, 10.0), Point::new(35.0, 10.0)] {
        let from = app.view.to_screen(node_at(&app));
        let to = app.view.to_screen(target) + off;
        click_at(&ctx, &mut app, from, Modifiers::NONE);
        drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
        let want = app.view.to_page(to);
        let at = node_at(&app);
        assert!((at - want).hypot() < 1e-3, "{at:?} snapped, not {want:?}");
    }
}

#[test]
fn dropping_an_end_node_on_another_end_node_joins_them() {
    let (ctx, mut app, id) = setup();
    let (a, b) = two_open_curves(&ctx, &mut app, id);
    // Drag the start of b onto the end of a.
    let from = app.view.to_screen(Point::new(44.0, 10.0));
    let to = app.view.to_screen(Point::new(30.0, 10.0));
    click_at(&ctx, &mut app, from, Modifiers::NONE);
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    assert!(app.doc().find_shape(b).is_none(), "b was not joined");
    let p = curve_path(&app, a);
    assert_eq!(tracedraw_core::nodes::nodes(&p).len(), 3);
    assert_eq!(
        p.elements()
            .iter()
            .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
            .count(),
        1
    );
}

/// Two neighbouring line nodes deleted together leave a curve keeping
/// the outline's general shape, not a straight line (#675).
#[test]
fn deleting_two_neighbouring_line_nodes_leaves_a_curve() {
    use tracedraw_core::geometry::{PathEl, Shape as _};
    let (ctx, mut app, id) = setup();
    app.convert_to_curves();
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let a = app.node_at(Point::new(60.0, 10.0)).expect("node");
    let b = app.node_at(Point::new(60.0, 40.0)).expect("node");
    app.node_selection = vec![(a.shape, a.index), (b.shape, b.index)];
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Delete, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let p = match app.doc().find_shape(id).map(|s| s.kind.clone()) {
        Some(ShapeKind::Path { path, .. }) => path,
        other => panic!("not a curve: {other:?}"),
    };
    assert_eq!(tracedraw_core::nodes::nodes(&p).len(), 2, "{p:?}");
    assert!(
        p.elements()
            .iter()
            .any(|e| matches!(e, PathEl::CurveTo(..))),
        "{p:?}"
    );
    // The curve bulges out towards where the right side was.
    let b = p.bounding_box();
    assert!(b.x1 > 35.0 && b.x1 < 65.0, "{b:?}");
    // One undo step brings the nodes back.
    app.undo();
    let p = match app.doc().find_shape(id).map(|s| s.kind.clone()) {
        Some(ShapeKind::Path { path, .. }) => path,
        other => panic!("not a curve: {other:?}"),
    };
    assert_eq!(tracedraw_core::nodes::nodes(&p).len(), 4);
}

#[test]
fn delete_removes_a_clicked_segment_and_keeps_its_nodes() {
    let (ctx, mut app, id) = setup();
    app.convert_to_curves();
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let path_of = |app: &App| match app.doc().find_shape(id).map(|s| s.kind.clone()) {
        Some(ShapeKind::Path { path, .. }) => path,
        other => panic!("not a curve: {other:?}"),
    };
    let before = path_of(&app);
    let corners = |p: &tracedraw_core::BezPath| {
        tracedraw_core::nodes::nodes(p)
            .iter()
            .map(|n| n.pos)
            .collect::<Vec<_>>()
    };
    let has_close = |p: &tracedraw_core::BezPath| {
        p.elements()
            .iter()
            .any(|e| matches!(e, tracedraw_core::geometry::PathEl::ClosePath))
    };
    assert!(has_close(&before));
    // A click on the right side chooses the segment, not a node.
    let at = app.view.to_screen(Point::new(60.0, 32.0));
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert!(app.chosen_segment().is_some());
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Delete, Modifiers::NONE)],
        Modifiers::NONE,
    );
    // The side went: the curve is open there, every corner stays.
    let after = path_of(&app);
    assert!(!has_close(&after), "{after:?}");
    let mut a = corners(&after);
    let mut b = corners(&before);
    let key_of = |p: &Point| ((p.x * 1000.0).round() as i64, (p.y * 1000.0).round() as i64);
    a.sort_by_key(key_of);
    b.sort_by_key(key_of);
    assert_eq!(a, b);
    assert!(app.chosen_segment().is_none());
    // One undo step puts the side back.
    app.undo();
    assert_eq!(path_of(&app), before);
}

#[test]
fn the_pen_closes_on_its_first_node_with_auto_add_delete_on() {
    let (ctx, mut app, id) = setup();
    app.select(vec![id]);
    app.delete_selection();
    app.set_tool(Tool::Pen);
    app.settings.curves.pen_auto_add_delete = true;
    // A pick reach wider than the join reach, as with large nodes.
    app.settings.curves.join_px = 3.0;
    app.settings.nodes.pick_distance = 10.0;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    for q in [
        Point::new(20.0, 20.0),
        Point::new(60.0, 20.0),
        Point::new(40.0, 50.0),
    ] {
        let at = app.view.to_screen(q);
        click_at(&ctx, &mut app, at, Modifiers::NONE);
    }
    assert_eq!(app.curve.as_ref().map(|c| c.nodes.len()), Some(3));
    // A click 7 px off the first node: beyond the join reach, within the
    // pick reach. It closes the curve; it used to delete the first node.
    let first = app.view.to_screen(Point::new(20.0, 20.0));
    click_at(
        &ctx,
        &mut app,
        first + egui::vec2(7.0, 0.0),
        Modifiers::NONE,
    );
    assert!(app.curve.is_none(), "still drawing: {:?}", app.curve);
    let made = app.selected_shapes();
    let path = match made.first().map(|s| &s.kind) {
        Some(ShapeKind::Path { path, .. }) => path.clone(),
        other => panic!("no curve made: {other:?}"),
    };
    assert!(
        path.elements()
            .iter()
            .any(|e| matches!(e, tracedraw_core::geometry::PathEl::ClosePath)),
        "{path:?}"
    );
    assert_eq!(tracedraw_core::nodes::nodes(&path).len(), 3);
}

#[test]
fn node_keys_in_the_shape_tool() {
    let (ctx, mut app, id) = setup();
    // A curve with a smooth-able middle node.
    let mut path = tracedraw_core::BezPath::new();
    path.move_to((10.0, 10.0));
    path.curve_to((20.0, 10.0), (30.0, 20.0), (40.0, 30.0));
    path.curve_to((45.0, 40.0), (55.0, 30.0), (70.0, 20.0));
    app.select(vec![id]);
    app.delete_selection();
    let c = app
        .new_shape(ShapeKind::Path {
            path,
            closed: false,
        })
        .expect("curve");
    app.select(vec![c]);
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let hit = app.node_at(Point::new(40.0, 30.0)).expect("node");
    app.node_selection = vec![(hit.shape, hit.index)];
    let ty = |app: &App| app.node_type_of(c, hit.index);
    use tracedraw_core::nodes::NodeType;
    assert_eq!(ty(&app), NodeType::Cusp);
    // C switches cusp and smooth, S symmetrical and smooth (a cusp node
    // becomes symmetrical), Y makes it symmetrical. The type is kept with
    // the curve, so the way back shows although the handles stay aligned.
    for (k, want) in [
        (egui::Key::C, NodeType::Smooth),
        (egui::Key::C, NodeType::Cusp),
        (egui::Key::S, NodeType::Symmetrical),
        (egui::Key::S, NodeType::Smooth),
        (egui::Key::S, NodeType::Symmetrical),
        (egui::Key::C, NodeType::Cusp),
        (egui::Key::Y, NodeType::Symmetrical),
    ] {
        frame(
            &ctx,
            &mut app,
            vec![key(k, Modifiers::NONE)],
            Modifiers::NONE,
        );
        assert_eq!(ty(&app), want, "{k:?}");
        // Still the Shape tool, nothing aligned.
        assert_eq!(app.tool, Tool::Shape);
    }
    // C makes the symmetrical node cusp; its handles stay aligned, and a
    // handle dragged now moves alone.
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::C, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert_eq!(ty(&app), NodeType::Cusp);
    let handles = |app: &App| match app.doc().find_shape(c).map(|s| s.kind.clone()) {
        Some(ShapeKind::Path { path, .. }) => tracedraw_core::nodes::nodes(&path)
            .into_iter()
            .find(|n| n.index == hit.index)
            .map(|n| (n.ctrl_in, n.ctrl_out))
            .expect("node"),
        _ => panic!("curve"),
    };
    let (before_in, before_out) = handles(&app);
    let out = before_out.expect("out handle");
    let from = app.view.to_screen(out);
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(0.0, -25.0),
        Modifiers::NONE,
        Vec::new(),
    );
    let (after_in, after_out) = handles(&app);
    assert_ne!(after_out, before_out, "the dragged handle moved");
    assert_eq!(after_in, before_in, "the other handle stayed");
    assert_eq!(ty(&app), NodeType::Cusp);
    app.node_selection = vec![(hit.shape, hit.index)];
    // Home and End choose the curve's first and last node.
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Home, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert_eq!(app.node_selection, vec![(c, 0)]);
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::End, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let last = app.node_selection.clone();
    assert_eq!(last.len(), 1);
    assert!(last[0].1 > hit.index, "{last:?}");
    // The arrow keys move the chosen node, not the curve.
    let before = app.doc().find_shape(c).map(|s| s.bounds()).expect("curve");
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::ArrowRight, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let after = app.doc().find_shape(c).map(|s| s.bounds()).expect("curve");
    assert!(
        (after.x0 - before.x0).abs() < 1e-9,
        "the start stays: {before:?} {after:?}"
    );
    assert!(
        after.x1 > before.x1 + 1e-6,
        "the end node moved right: {before:?} {after:?}"
    );
    // Ctrl+Space: the Pick tool.
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Space, Modifiers::COMMAND)],
        Modifiers::COMMAND,
    );
    assert_eq!(app.tool, Tool::Pick);
}

#[test]
fn two_fingers_pan_without_dragging_anything() {
    let (ctx, mut app, id) = setup();
    let before = bounds(&app, id);
    let origin = app.view.to_screen(Point::new(0.0, 0.0));
    let touch = |id: u64, phase: egui::TouchPhase, pos: Pos2| Event::Touch {
        device_id: egui::TouchDeviceId(1),
        id: egui::TouchId(id),
        phase,
        pos,
        force: None,
    };
    // Both fingers on the object, then both move 60 points right. The
    // windowing layer also turns the first finger into the pointer.
    let (a, b) = (
        app.view.to_screen(Point::new(30.0, 25.0)),
        app.view.to_screen(Point::new(40.0, 25.0)),
    );
    frame(
        &ctx,
        &mut app,
        vec![
            Event::PointerMoved(a),
            button(a, true, PointerButton::Primary, Modifiers::NONE),
            touch(0, egui::TouchPhase::Start, a),
            touch(1, egui::TouchPhase::Start, b),
        ],
        Modifiers::NONE,
    );
    for i in 1..=6 {
        let d = egui::vec2(10.0 * i as f32, 0.0);
        frame(
            &ctx,
            &mut app,
            vec![
                Event::PointerMoved(a + d),
                touch(0, egui::TouchPhase::Move, a + d),
                touch(1, egui::TouchPhase::Move, b + d),
            ],
            Modifiers::NONE,
        );
    }
    let end = egui::vec2(60.0, 0.0);
    frame(
        &ctx,
        &mut app,
        vec![
            button(a + end, false, PointerButton::Primary, Modifiers::NONE),
            touch(0, egui::TouchPhase::End, a + end),
            touch(1, egui::TouchPhase::End, b + end),
        ],
        Modifiers::NONE,
    );
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // The view moved with the fingers; the object did not.
    let moved = app.view.to_screen(Point::new(0.0, 0.0)) - origin;
    assert!(moved.x > 30.0, "{moved:?}");
    assert_eq!(bounds(&app, id), before);
    assert!(matches!(app.drag, Drag::None));
}

#[test]
fn a_moved_centre_of_rotation_is_kept_with_the_object() {
    let (ctx, mut app, id) = setup();
    // Rotate mode: drag the centre of rotation to the rectangle's lower
    // left corner area.
    let inside = app.view.to_screen(Point::new(35.0, 25.0));
    click_at(&ctx, &mut app, inside, Modifiers::NONE);
    assert!(app.rotate_mode);
    let centre = app.view.to_screen(app.rotation_center().expect("centre"));
    let target = app.view.to_screen(Point::new(20.0, 15.0));
    app.snap.off = true;
    drag(&ctx, &mut app, centre, target, Modifiers::NONE, Vec::new());
    let put = app.rotation_center().expect("centre");
    assert!((put - Point::new(20.0, 15.0)).hypot() < 0.5, "{put:?}");
    // Click away (nothing selected), then select the rectangle again and
    // turn rotate mode on: the centre is where it was put.
    let empty = app.view.to_screen(Point::new(150.0, 250.0));
    click_at(&ctx, &mut app, empty, Modifiers::NONE);
    assert!(app.selection.is_empty());
    click_at(&ctx, &mut app, inside, Modifiers::NONE);
    assert_eq!(app.selection, vec![id]);
    click_at(&ctx, &mut app, inside, Modifiers::NONE);
    assert!(app.rotate_mode);
    assert_eq!(app.rotation_center(), Some(put));
    // It travels with the object.
    app.transform_selection(Affine::translate((10.0, 5.0)));
    let after = app.rotation_center().expect("centre");
    assert!(
        (after - (put + Vec2::new(10.0, 5.0))).hypot() < 1e-9,
        "{after:?}"
    );
    // Another drawing has its own centres.
    app.new_document();
    assert!(app.pivots.single.is_empty());
}

/// The windowing layer delivers Ctrl+C, Ctrl+X and Ctrl+V as copy, cut
/// and paste events (a paste only while the system clipboard holds text),
/// never as key presses: those events copy, cut and paste objects.
#[test]
fn copy_cut_and_paste_events_work_on_objects() {
    let (ctx, mut app, id) = setup();
    let n = shape_count(&app);
    frame(&ctx, &mut app, vec![Event::Copy], Modifiers::COMMAND);
    assert!(app.clipboard.is_some());
    frame(
        &ctx,
        &mut app,
        vec![Event::Paste("<svg/>".into())],
        Modifiers::COMMAND,
    );
    assert_eq!(shape_count(&app), n + 1);
    let copy = app.selection[0];
    assert_ne!(copy, id);
    // One paste per frame, even if a key press comes along with it.
    frame(
        &ctx,
        &mut app,
        vec![
            key(egui::Key::V, Modifiers::COMMAND),
            Event::Paste("x".into()),
        ],
        Modifiers::COMMAND,
    );
    assert_eq!(shape_count(&app), n + 2);
    // Cut takes the selection away; paste brings it back.
    frame(&ctx, &mut app, vec![Event::Cut], Modifiers::COMMAND);
    assert_eq!(shape_count(&app), n + 1);
    frame(
        &ctx,
        &mut app,
        vec![Event::Paste("x".into())],
        Modifiers::COMMAND,
    );
    assert_eq!(shape_count(&app), n + 2);
    // With Shift held the paste lands in the middle of the view.
    frame(
        &ctx,
        &mut app,
        vec![Event::Paste("x".into())],
        Modifiers::COMMAND | Modifiers::SHIFT,
    );
    assert_eq!(shape_count(&app), n + 3);
    let view = app.view.visible_page_rect(app.canvas_rect).center();
    let b = app.selection_bounds().expect("bounds");
    assert!((b.center() - view).hypot() < 1e-6);
}

/// With only a picture or files on the system clipboard the windowing
/// layer drops the Ctrl+V press and sends no paste event: the release of
/// V with Ctrl still pastes, once; a release after a paste event does not.
#[test]
fn ctrl_v_pastes_when_only_its_release_comes_through() {
    let (ctx, mut app, _) = setup();
    let n = shape_count(&app);
    let v_up = |mods: Modifiers| Event::Key {
        key: egui::Key::V,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: mods,
    };
    frame(&ctx, &mut app, vec![Event::Copy], Modifiers::COMMAND);
    frame(
        &ctx,
        &mut app,
        vec![v_up(Modifiers::COMMAND)],
        Modifiers::COMMAND,
    );
    assert_eq!(shape_count(&app), n + 1);
    // A paste event, then the release in a later frame: one paste.
    frame(
        &ctx,
        &mut app,
        vec![Event::Paste("x".into())],
        Modifiers::COMMAND,
    );
    frame(
        &ctx,
        &mut app,
        vec![v_up(Modifiers::COMMAND)],
        Modifiers::COMMAND,
    );
    assert_eq!(shape_count(&app), n + 2);
    // V let go without Ctrl is typing, not a paste.
    frame(&ctx, &mut app, vec![v_up(Modifiers::NONE)], Modifiers::NONE);
    assert_eq!(shape_count(&app), n + 2);
}

#[test]
fn handles_stand_outside_the_box_and_grabbing_one_does_not_jump() {
    let (ctx, mut app, id) = setup();
    let before = bounds(&app, id);
    let r = app.view.rect_to_screen(before);
    let off = crate::interaction::HANDLE_OFFSET;
    // Each handle stands off the box, so the outline stays in view.
    let e = handle(&app, Handle::E);
    let nw = handle(&app, Handle::NW);
    assert!((e.x - (r.right() + off)).abs() < 1e-3, "{e:?} {r:?}");
    assert!((nw.x - (r.left() - off)).abs() < 1e-3, "{nw:?} {r:?}");
    assert!((nw.y - (r.top() - off)).abs() < 1e-3, "{nw:?} {r:?}");
    // Dragging the right handle 20 points widens the object by 20 points,
    // not by the handle's distance from the box as well.
    drag(
        &ctx,
        &mut app,
        e,
        e + egui::vec2(20.0, 0.0),
        Modifiers::NONE,
        Vec::new(),
    );
    let after = bounds(&app, id);
    let grown = 20.0 / app.view.zoom as f64;
    assert!(
        close(after.width(), before.width() + grown),
        "{before:?} {after:?}"
    );
    assert!(close(after.x0, before.x0), "{before:?} {after:?}");
}

#[test]
fn turning_snaps_the_pointer_to_another_objects_node() {
    let (ctx, mut app, id) = setup();
    // Another rectangle up and to the right; its bottom left corner is a
    // node at (100, 80).
    let other = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(100.0, 80.0, 130.0, 110.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.select(vec![id]);
    app.rotate_mode = true;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let c = app.rotation_center().expect("centre");
    let ne = handle(&app, Handle::NE);
    let start = app.view.to_page(ne);
    // Released a few points off the node: the turn ends on the node.
    let node = Point::new(100.0, 80.0);
    let near = app.view.to_screen(node) + egui::vec2(3.0, 2.0);
    drag(&ctx, &mut app, ne, near, Modifiers::NONE, Vec::new());
    let t = app
        .doc()
        .find_shape(id)
        .map(|s| s.transform)
        .expect("shape");
    let k = t.as_coeffs();
    let turned = k[1].atan2(k[0]);
    let want = (node - c).atan2() - (start - c).atan2();
    assert!((turned - want).abs() < 1e-6, "{turned} {want}");
    assert!(app.doc().find_shape(other).is_some());
}

#[test]
fn drawing_a_new_shape_lets_the_selection_go_at_once() {
    let (ctx, mut app, id) = setup();
    assert_eq!(app.selection, vec![id]);
    app.set_tool(Tool::Rectangle);
    // Somewhere empty on the page, away from the rectangle.
    let from = app.view.to_screen(Point::new(100.0, 150.0));
    let to = app.view.to_screen(Point::new(140.0, 120.0));
    let none = Modifiers::NONE;
    frame(&ctx, &mut app, vec![Event::PointerMoved(from)], none);
    frame(
        &ctx,
        &mut app,
        vec![button(from, true, PointerButton::Primary, none)],
        none,
    );
    for i in 1..=3 {
        let p = from + (to - from) * (i as f32 / 3.0);
        frame(&ctx, &mut app, vec![Event::PointerMoved(p)], none);
    }
    // While the new rectangle is being drawn, nothing is selected.
    assert!(matches!(app.drag, Drag::Box { .. }));
    assert!(app.selection.is_empty(), "{:?}", app.selection);
    frame(
        &ctx,
        &mut app,
        vec![button(to, false, PointerButton::Primary, none)],
        none,
    );
    frame(&ctx, &mut app, Vec::new(), none);
    // Then the new rectangle alone is.
    assert_eq!(app.selection.len(), 1);
    assert_ne!(app.selection[0], id);
}

#[test]
fn ctrl_click_goes_one_group_level_in_at_a_time() {
    let (ctx, mut app, [a, _b, c], inner) = grouped();
    // The group of two squares and the third square in an outer group.
    app.select(vec![inner, c]);
    app.group_selection();
    let outer = app.selection[0];
    app.select(Vec::new());
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let on_a = app.view.to_screen(Point::new(115.0, 110.0));
    click_at(&ctx, &mut app, on_a, Modifiers::NONE);
    assert_eq!(app.selection, vec![outer]);
    // Each Ctrl+click goes one level further in, then stays on the square.
    click_at(&ctx, &mut app, on_a, Modifiers::CTRL);
    assert_eq!(app.selection, vec![inner]);
    click_at(&ctx, &mut app, on_a, Modifiers::CTRL);
    assert_eq!(app.selection, vec![a]);
    click_at(&ctx, &mut app, on_a, Modifiers::CTRL);
    assert_eq!(app.selection, vec![a]);
}

/// #849: an object chosen inside a group with Ctrl+click is dragged on
/// its own, grabbed anywhere on it (not only at its centre mark), also
/// two group levels in; a press on the group elsewhere still takes the
/// whole group.
#[test]
fn object_chosen_in_a_group_drags_alone_from_anywhere_on_it() {
    let (ctx, mut app, [a, b, c], inner) = grouped();
    app.select(vec![inner, c]);
    app.group_selection();
    let outer = app.selection[0];
    app.select(Vec::new());
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // Off the centre of square a (105..125 x 100..120 on the page).
    let on_a = app.view.to_screen(Point::new(108.0, 103.0));
    click_at(&ctx, &mut app, on_a, Modifiers::CTRL);
    click_at(&ctx, &mut app, on_a, Modifiers::CTRL);
    assert_eq!(app.selection, vec![a]);
    let (before_a, before_b, before_c) = (
        page_bounds(&app, a),
        page_bounds(&app, b),
        page_bounds(&app, c),
    );
    let grab = app.view.to_screen(Point::new(121.0, 117.0));
    let to = app.view.to_screen(Point::new(121.0, 137.0));
    drag(&ctx, &mut app, grab, to, Modifiers::NONE, Vec::new());
    assert_eq!(app.selection, vec![a]);
    let after = page_bounds(&app, a);
    assert!(
        close(after.y0 - before_a.y0, 20.0),
        "{before_a:?} {after:?}"
    );
    assert_eq!(page_bounds(&app, b), before_b);
    assert_eq!(page_bounds(&app, c), before_c);
    assert_eq!(app.doc().top_level_of(a), Some(outer));
    // A Ctrl+drag straight on a square of an unselected group moves that
    // square alone too.
    app.select(Vec::new());
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let on_c = app.view.to_screen(Point::new(103.0, 153.0));
    let to_c = app.view.to_screen(Point::new(123.0, 153.0));
    drag(&ctx, &mut app, on_c, to_c, Modifiers::CTRL, Vec::new());
    assert_eq!(app.selection, vec![c]);
    assert!(close(page_bounds(&app, c).x0 - before_c.x0, 20.0));
    assert_eq!(page_bounds(&app, b), before_b);
    // A press on another part of the group takes the whole group.
    let on_b = app.view.to_screen(Point::new(148.0, 103.0));
    let to_b = app.view.to_screen(Point::new(148.0, 113.0));
    let (a0, b0) = (page_bounds(&app, a), page_bounds(&app, b));
    drag(&ctx, &mut app, on_b, to_b, Modifiers::NONE, Vec::new());
    assert_eq!(app.selection, vec![outer]);
    let moved_b = page_bounds(&app, b).y0 - b0.y0;
    assert!(moved_b > 5.0, "{moved_b}");
    assert!(close(page_bounds(&app, a).y0 - a0.y0, moved_b));
}

#[test]
fn ctrl_keeps_the_marquee_square_and_space_copies_while_turning() {
    let (ctx, mut app, id) = setup();
    // Space while turning leaves a copy at the angle reached.
    app.rotate_mode = true;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let ne = handle(&app, Handle::NE);
    let count = shape_count(&app);
    drag(
        &ctx,
        &mut app,
        ne,
        ne + egui::vec2(-40.0, -60.0),
        Modifiers::NONE,
        vec![key(egui::Key::Space, Modifiers::NONE)],
    );
    assert_eq!(shape_count(&app), count + 1);
    assert!(app.doc().find_shape(id).is_some());
    // A marquee dragged with Ctrl is square.
    app.rotate_mode = false;
    app.select(Vec::new());
    let from = app.view.to_screen(Point::new(150.0, 250.0));
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(from)],
        Modifiers::CTRL,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(from, true, PointerButton::Primary, Modifiers::CTRL)],
        Modifiers::CTRL,
    );
    for i in 1..=3 {
        let p = from + egui::vec2(60.0, 20.0) * (i as f32 / 3.0);
        frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(p)],
            Modifiers::CTRL,
        );
    }
    match app.drag {
        Drag::Marquee { start, current, .. } => {
            let d = current - start;
            assert!((d.x.abs() - d.y.abs()).abs() < 1e-9, "{d:?}");
        }
        ref other => panic!("{other:?}"),
    }
    let end = from + egui::vec2(60.0, 20.0);
    frame(
        &ctx,
        &mut app,
        vec![button(end, false, PointerButton::Primary, Modifiers::CTRL)],
        Modifiers::CTRL,
    );
}

#[test]
fn selection_groups_store_recall_add_and_zoom() {
    let (ctx, mut app, id) = setup();
    let none = Modifiers::NONE;
    let press = |app: &mut App, k: egui::Key, m: Modifiers| frame(&ctx, app, vec![key(k, m)], m);
    // Ctrl+3 stores the selection; 3 selects it again.
    press(&mut app, egui::Key::Num3, Modifiers::COMMAND);
    assert_eq!(app.doc().selection_groups.get(&3), Some(&vec![id]));
    app.select(Vec::new());
    press(&mut app, egui::Key::Num3, none);
    assert_eq!(app.selection, vec![id]);
    // 3 again zooms to it.
    let zoom = app.view.zoom;
    press(&mut app, egui::Key::Num3, none);
    assert!(
        (app.view.zoom - zoom).abs() > 1e-6,
        "{zoom} {}",
        app.view.zoom
    );
    // Alt+3 adds another object to the group.
    let other = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(100.0, 100.0, 120.0, 120.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.select(vec![other]);
    press(&mut app, egui::Key::Num3, Modifiers::ALT);
    app.select(Vec::new());
    press(&mut app, egui::Key::Num3, none);
    assert_eq!(app.selection.len(), 2);
    // A deleted object drops out; an unused number does nothing.
    app.select(vec![other]);
    app.delete_selection();
    press(&mut app, egui::Key::Num3, none);
    assert_eq!(app.selection, vec![id]);
    press(&mut app, egui::Key::Num7, none);
    assert_eq!(app.selection, vec![id]);
}

#[test]
fn ctrl_keeps_the_freehand_selection_a_rectangle() {
    let (ctx, mut app, _id) = setup();
    app.set_tool(Tool::FreeformPick);
    app.select(Vec::new());
    let ctrl = Modifiers::CTRL;
    let from = app.view.to_screen(Point::new(150.0, 250.0));
    frame(&ctx, &mut app, vec![Event::PointerMoved(from)], ctrl);
    frame(
        &ctx,
        &mut app,
        vec![button(from, true, PointerButton::Primary, ctrl)],
        ctrl,
    );
    for i in 1..=4 {
        let p = from
            + egui::vec2(50.0, 30.0) * (i as f32 / 4.0)
            + egui::vec2(0.0, (i % 2) as f32 * 7.0);
        frame(&ctx, &mut app, vec![Event::PointerMoved(p)], ctrl);
    }
    match &app.drag {
        Drag::Lasso { points, .. } => {
            assert_eq!(points.len(), 4, "{points:?}");
            assert!(
                (points[0].y - points[1].y).abs() < 1e-9
                    && (points[1].x - points[2].x).abs() < 1e-9
            );
        }
        other => panic!("{other:?}"),
    }
    let end = from + egui::vec2(50.0, 30.0);
    frame(
        &ctx,
        &mut app,
        vec![button(end, false, PointerButton::Primary, ctrl)],
        ctrl,
    );
}

#[test]
fn node_tracking_drags_a_curves_node_with_the_pick_tool() {
    let (ctx, mut app, id) = setup();
    app.convert_to_curves();
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // The rectangle's corner node at (60, 40), dragged 10 mm right.
    let corner = app.view.to_screen(Point::new(60.0, 40.0));
    let to = app.view.to_screen(Point::new(70.0, 40.0));
    // Off (the default): the whole object moves.
    assert!(!app.settings.node_tracking);
    let before = bounds(&app, id);
    drag(&ctx, &mut app, corner, to, Modifiers::NONE, Vec::new());
    let after = bounds(&app, id);
    assert!(close(after.width(), before.width()), "{before:?} {after:?}");
    assert!(close(after.x0 - before.x0, 10.0), "{before:?} {after:?}");
    app.undo();
    // On: only that node moves; the object widens on its right side.
    app.settings.node_tracking = true;
    app.select(vec![id]);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let before = bounds(&app, id);
    drag(&ctx, &mut app, corner, to, Modifiers::NONE, Vec::new());
    let after = bounds(&app, id);
    assert!(close(after.x0, before.x0), "{before:?} {after:?}");
    assert!(close(after.x1, before.x1 + 10.0), "{before:?} {after:?}");
    assert_eq!(app.tool, Tool::Pick);
    // One undo step puts the node back.
    app.undo();
    assert!(close(bounds(&app, id).x1, before.x1));
}

#[test]
fn bspline_clicks_place_control_points_of_a_smooth_curve() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::BSpline);
    let pts = [
        Point::new(100.0, 150.0),
        Point::new(130.0, 190.0),
        Point::new(180.0, 180.0),
        Point::new(190.0, 140.0),
    ];
    for p in pts {
        let at = app.view.to_screen(p);
        click_at(&ctx, &mut app, at, Modifiers::NONE);
    }
    app.finish_curve();
    let path = last_path(&app);
    let segs: Vec<_> = path.segments().collect();
    // Four control points: one cubic from the first to the last point,
    // the middle two as its handles (not four corners joined by lines).
    assert_eq!(segs.len(), 1, "{segs:?}");
    let tracedraw_core::geometry::PathSeg::Cubic(c) = segs[0] else {
        panic!("a cubic: {segs:?}");
    };
    let near = |a: Point, b: Point| (a - b).hypot() < 0.5;
    assert!(near(c.p0, pts[0]) && near(c.p3, pts[3]), "{c:?}");
    assert!(near(c.p1, pts[1]) && near(c.p2, pts[2]), "{c:?}");
}

/// A B-spline keeps its control points: the Shape tool shows them
/// instead of the curve's nodes, dragging one reshapes the spline, a
/// double-click on the control polygon adds one and a double-click on a
/// control point removes it. Any other edit of the outline leaves an
/// ordinary curve.
#[test]
fn a_bspline_keeps_its_control_points_for_the_shape_tool() {
    use tracedraw_core::geometry::bspline_path;
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::BSpline);
    let pts = [
        Point::new(100.0, 150.0),
        Point::new(130.0, 190.0),
        Point::new(180.0, 180.0),
        Point::new(190.0, 140.0),
    ];
    for p in pts {
        let at = app.view.to_screen(p);
        click_at(&ctx, &mut app, at, Modifiers::NONE);
    }
    app.finish_curve();
    let id = app.selection[0];
    let spline = |app: &App| app.doc().find_shape(id).expect("shape").spline.clone();
    let near = |a: &[Point], b: &[Point]| {
        a.len() == b.len() && a.iter().zip(b).all(|(p, q)| (*p - *q).hypot() < 0.5)
    };
    assert!(
        near(&spline(&app).expect("spline"), &pts),
        "{:?}",
        spline(&app)
    );
    // One undo step for the whole curve.
    app.undo();
    assert!(app.doc().find_shape(id).is_none());
    app.redo();
    assert!(
        near(&spline(&app).expect("spline"), &pts),
        "{:?}",
        spline(&app)
    );

    // The Shape tool: the control points are the nodes shown; a drag of
    // the second one reshapes the spline.
    app.select(vec![id]);
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert!(app.node_at(pts[1]).is_none());
    assert!(matches!(
        app.kind_node_at(pts[1]),
        Some((_, crate::kind_nodes::KindNode::Spline { index: 1 }))
    ));
    let moved = Point::new(130.0, 210.0);
    let (from, to) = (app.view.to_screen(pts[1]), app.view.to_screen(moved));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    let mut now = pts.to_vec();
    now[1] = moved;
    assert!(
        near(&spline(&app).expect("spline"), &now),
        "{:?}",
        spline(&app)
    );
    assert_eq!(
        last_path(&app),
        bspline_path(&spline(&app).expect("spline"), false)
    );
    assert_eq!(app.engine.undo_label(), Some("Edit B-Spline"));

    // A double-click on the control polygon (between the third and the
    // fourth point) adds a control point there.
    let double = |ctx: &egui::Context, app: &mut App, at: Pos2| {
        // Well past the triple-click time since the last click, so these
        // two clicks make a double-click and not a triple.
        for _ in 0..40 {
            frame(ctx, app, Vec::new(), Modifiers::NONE);
        }
        frame(ctx, app, vec![Event::PointerMoved(at)], Modifiers::NONE);
        for _ in 0..2 {
            frame(
                ctx,
                app,
                vec![button(at, true, PointerButton::Primary, Modifiers::NONE)],
                Modifiers::NONE,
            );
            frame(
                ctx,
                app,
                vec![button(at, false, PointerButton::Primary, Modifiers::NONE)],
                Modifiers::NONE,
            );
        }
        for _ in 0..20 {
            frame(ctx, app, Vec::new(), Modifiers::NONE);
        }
    };
    let mid = now[2].midpoint(now[3]);
    let at = app.view.to_screen(mid);
    double(&ctx, &mut app, at);
    let five = spline(&app).expect("spline");
    assert_eq!(five.len(), 5, "{five:?}");
    assert!((five[3] - mid).hypot() < 0.5, "{five:?}");
    // A double-click on a control point removes it.
    let at = app.view.to_screen(five[3]);
    double(&ctx, &mut app, at);
    assert!(
        near(&spline(&app).expect("spline"), &now),
        "{:?}",
        spline(&app)
    );

    // Convert To Curves: the outline stays, the control points go, and
    // the Shape tool shows the curve's nodes.
    let outline = last_path(&app);
    app.select(vec![id]);
    assert!(app.can_convert_to_curves());
    app.convert_to_curves();
    assert_eq!(spline(&app), None);
    assert_eq!(last_path(&app), outline);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert!(app.node_at(pts[0]).is_some());
}

/// Typing while a field has the keyboard (the font search field of the
/// property bar) goes to the field, not to the text being edited (#342).
#[test]
fn typing_in_a_focused_field_leaves_the_edited_text_alone() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.settings.autocorrect.enabled = false;
    app.start_text(Point::new(30.0, 50.0), None);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // Typed on the canvas: into the text.
    frame(
        &ctx,
        &mut app,
        vec![Event::Text("ab".into())],
        Modifiers::NONE,
    );
    assert_eq!(app.edit_text(), "ab");
    // A field takes the keyboard (as a click in the font search field
    // does): what is typed goes there.
    let field = egui::Id::new("some_field");
    ctx.memory_mut(|m| m.request_focus(field));
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    ctx.memory_mut(|m| m.request_focus(field));
    frame(
        &ctx,
        &mut app,
        vec![Event::Text("cd".into())],
        Modifiers::NONE,
    );
    assert_eq!(app.edit_text(), "ab");
    assert!(app.text_edit.is_some());
    // The keyboard back on the canvas: typing edits the text again.
    ctx.memory_mut(|m| m.surrender_focus(field));
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        vec![Event::Text("e".into())],
        Modifiers::NONE,
    );
    assert_eq!(app.edit_text(), "abe");
}

/// The caret of text being edited is black on a white edge, whatever the
/// interface theme (its text colour is light in the dark themes and the
/// caret was lost on the white page, #632).
#[test]
fn the_text_caret_is_black_on_a_white_edge() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.settings.autocorrect.enabled = false;
    app.start_text(Point::new(30.0, 50.0), None);
    frame(
        &ctx,
        &mut app,
        vec![Event::Text("ab".into())],
        Modifiers::NONE,
    );
    let (top, bottom, _) = app.text_caret_geometry().expect("caret");
    let ends = [app.view.to_screen(top), app.view.to_screen(bottom)];
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| crate::ui::root(&mut app, ui));
    out.textures_delta.clear();
    let strokes: Vec<egui::Stroke> = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::LineSegment { points, stroke }
                if points[0].distance(ends[0]) < 0.01 && points[1].distance(ends[1]) < 0.01 =>
            {
                Some(*stroke)
            }
            _ => None,
        })
        .collect();
    assert_eq!(strokes, crate::canvas::TEXT_CARET.to_vec());
    assert_eq!(strokes[1].color, egui::Color32::BLACK);
    assert!(strokes[0].width > strokes[1].width);
}

/// With alignment guides on (and every other snapping off), a move of the
/// rectangle near another one lands its top edge on the other's top and
/// shows the line while the button is down; a scale handle lines up the
/// same way.
#[test]
fn alignment_guides_line_up_a_moved_object_and_show_the_line() {
    let (ctx, mut app, id) = setup();
    let other = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(100.0, 60.0, 130.0, 90.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.select(vec![id]);
    app.snap.objects = false;
    app.snap.page = false;
    app.snap.guides = false;
    app.snap.alignment_guides = true;
    // The radius is 10 px; the top edge goes 1 mm past y = 90.
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    let to = app.view.to_screen(Point::new(35.0, 25.0 + 51.0));
    let none = Modifiers::NONE;
    frame(&ctx, &mut app, vec![Event::PointerMoved(from)], none);
    frame(
        &ctx,
        &mut app,
        vec![button(from, true, PointerButton::Primary, none)],
        none,
    );
    for i in 1..=4 {
        let p = from + (to - from) * (i as f32 / 4.0);
        frame(&ctx, &mut app, vec![Event::PointerMoved(p)], none);
    }
    {
        let o = app.live_guides.borrow();
        assert!(
            o.lines
                .iter()
                .any(|l| (l.a.y - 90.0).abs() < 1e-9 && (l.b.y - 90.0).abs() < 1e-9),
            "{o:?}"
        );
    }
    frame(
        &ctx,
        &mut app,
        vec![button(to, false, PointerButton::Primary, none)],
        none,
    );
    frame(&ctx, &mut app, Vec::new(), none);
    let b = bounds(&app, id);
    assert!((b.y1 - 90.0).abs() < 1e-6, "{b:?}");
    assert!(close(b.x0, 10.0), "{b:?}");
    // The guides go when the button is up.
    assert!(app.live_guides.borrow().is_empty());
    // A right side handle dragged near the other's left edge stops on it.
    let h = handle(&app, Handle::E);
    let target = app.view.to_screen(Point::new(99.0, b.center().y));
    drag(
        &ctx,
        &mut app,
        h,
        egui::pos2(target.x, h.y),
        none,
        Vec::new(),
    );
    let b = bounds(&app, id);
    assert!((b.x1 - 100.0).abs() < 1e-6, "{b:?}");
    // With snapping off (Alt+Q) the same move goes where the pointer goes.
    app.snap.off = true;
    let from = app.view.to_screen(b.center());
    let to = app.view.to_screen(b.center() + Vec2::new(0.0, -5.0));
    drag(&ctx, &mut app, from, to, none, Vec::new());
    let after = bounds(&app, id);
    assert!((after.y1 - 85.0).abs() < 0.2, "{after:?}");
    let _ = other;
}

/// With alignment guides on, a rectangle drawn near the size of another
/// object takes its width and height.
#[test]
fn a_rectangle_drawn_with_alignment_guides_takes_another_objects_size() {
    let (ctx, mut app, id) = setup();
    app.snap.objects = false;
    app.snap.page = false;
    app.snap.guides = false;
    app.snap.alignment_guides = true;
    app.settings.snap.align.page = false;
    app.set_tool(Tool::Rectangle);
    // The other rectangle is 50 x 30 mm; this one is dragged 51 x 31.
    let from = app.view.to_screen(Point::new(100.0, 100.0));
    let to = app.view.to_screen(Point::new(151.0, 131.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    let new = app.selection.first().copied().expect("a new rectangle");
    assert_ne!(new, id);
    let b = bounds(&app, new);
    assert!((b.width() - 50.0).abs() < 1e-6, "{b:?}");
    assert!((b.height() - 30.0).abs() < 1e-6, "{b:?}");
    // Without the option it keeps the height dragged.
    app.settings.snap.align.equal_size = false;
    let from = app.view.to_screen(Point::new(100.0, 200.0));
    // (Far from the first one's edges, which it would line up with.)
    let to = app.view.to_screen(Point::new(157.0, 231.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    let b = bounds(&app, app.selection.first().copied().expect("another"));
    assert!((b.height() - 31.0).abs() < 0.1, "{b:?}");
}

/// A curve tool's drag from `from` to `to`, node placed at the press.
fn pull(ctx: &egui::Context, app: &mut App, from: Point, to: Point) {
    let (a, b) = (app.view.to_screen(from), app.view.to_screen(to));
    drag(ctx, app, a, b, Modifiers::NONE, Vec::new());
    for _ in 0..20 {
        frame(ctx, app, Vec::new(), Modifiers::NONE);
    }
}

fn click_page(ctx: &egui::Context, app: &mut App, p: Point) {
    let at = app.view.to_screen(p);
    click_at(ctx, app, at, Modifiers::NONE);
}

fn cubic(seg: tracedraw_core::geometry::PathSeg) -> tracedraw_core::geometry::CubicBez {
    match seg {
        tracedraw_core::geometry::PathSeg::Cubic(c) => c,
        other => panic!("a cubic: {other:?}"),
    }
}

fn near(a: Point, b: Point) -> bool {
    (a - b).hypot() < 0.6
}

/// A click after a pulled node gives the segment the preview showed: a
/// curve leaving along the pulled handle and arriving straight at the
/// new node, which has no handle of its own (#314).
#[test]
fn a_pen_click_after_a_pulled_node_follows_its_handle() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::Pen);
    let (a, b, c) = (
        Point::new(100.0, 150.0),
        Point::new(140.0, 150.0),
        Point::new(180.0, 150.0),
    );
    click_page(&ctx, &mut app, a);
    pull(&ctx, &mut app, b, Point::new(150.0, 170.0));
    click_page(&ctx, &mut app, c);
    app.finish_curve();
    let segs: Vec<_> = last_path(&app).segments().collect();
    assert_eq!(segs.len(), 2, "{segs:?}");
    // Into the pulled node: a curve arriving along its mirrored handle.
    let s0 = cubic(segs[0]);
    assert!(near(s0.p2, Point::new(130.0, 130.0)), "{s0:?}");
    // Out of it after a plain click: a curve leaving along the pulled
    // handle, arriving straight at the clicked node.
    let s1 = cubic(segs[1]);
    assert!(near(s1.p1, Point::new(150.0, 170.0)), "{s1:?}");
    assert!(near(s1.p2, c) && near(s1.p3, c), "{s1:?}");
    // The next segment leaves the clicked node straight. (The first
    // curve goes, or a click on its end node would continue it, #630.)
    app.delete_selection();
    app.set_tool(Tool::Pen);
    app.select(Vec::new());
    click_page(&ctx, &mut app, a);
    pull(&ctx, &mut app, b, Point::new(150.0, 170.0));
    click_page(&ctx, &mut app, c);
    click_page(&ctx, &mut app, Point::new(220.0, 150.0));
    app.finish_curve();
    let segs: Vec<_> = last_path(&app).segments().collect();
    assert_eq!(segs.len(), 3, "{segs:?}");
    assert!(
        matches!(segs[2], tracedraw_core::geometry::PathSeg::Line(_)),
        "{segs:?}"
    );
}

/// A 3-point curve whose chord starts on an end node of the selected
/// curve continues it, and one that ends on the other end node closes
/// it (the owner's report: the second curve left the object open).
#[test]
fn a_three_point_curve_continues_and_closes_the_selected_curve() {
    let (ctx, mut app, first) = setup();
    app.select(vec![first]);
    app.delete_selection();
    app.set_tool(Tool::ThreePointCurve);
    let before = shape_count(&app);
    let (a, b) = (Point::new(100.0, 150.0), Point::new(160.0, 150.0));
    // The chord, then the point the curve passes through.
    pull(&ctx, &mut app, a, b);
    click_page(&ctx, &mut app, Point::new(130.0, 170.0));
    assert_eq!(shape_count(&app), before + 1);
    let id = app.selection[0];
    assert_eq!(last_path(&app).segments().count(), 1);
    // A second curve from the end node back to the start node: the same
    // object, closed, with two segments.
    pull(&ctx, &mut app, b, a);
    click_page(&ctx, &mut app, Point::new(130.0, 130.0));
    assert_eq!(shape_count(&app), before + 1, "no new object");
    assert_eq!(app.selection, vec![id]);
    let p = last_path(&app);
    assert_eq!(p.segments().count(), 2, "{p:?}");
    assert!(
        p.elements()
            .iter()
            .any(|e| matches!(e, tracedraw_core::geometry::PathEl::ClosePath)),
        "closed: {p:?}"
    );
    // A chord away from the curve: a curve of its own.
    pull(
        &ctx,
        &mut app,
        Point::new(200.0, 200.0),
        Point::new(240.0, 200.0),
    );
    click_page(&ctx, &mut app, Point::new(220.0, 220.0));
    assert_eq!(shape_count(&app), before + 2);
}

#[test]
fn clicking_the_last_node_again_makes_it_a_cusp() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::Pen);
    let (a, b, c) = (
        Point::new(100.0, 150.0),
        Point::new(140.0, 150.0),
        Point::new(180.0, 150.0),
    );
    click_page(&ctx, &mut app, a);
    pull(&ctx, &mut app, b, Point::new(150.0, 170.0));
    // The pointer over the last node shows that a click edits it.
    let over = app.view.to_screen(b);
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(over)],
        Modifiers::NONE,
    );
    assert_eq!(
        for_tool(&app, false, false, false, Some(over)),
        ToolCursor::PenNib(Some(crate::ui::cursors::CurveMark::EditNode))
    );
    click_page(&ctx, &mut app, b);
    assert_eq!(app.curve.as_ref().map(|c| c.nodes.len()), Some(2));
    // The next node is pulled: the segment leaves the cusp without a
    // handle and arrives along the new one; the segment before keeps its
    // curve into the cusp.
    pull(&ctx, &mut app, c, Point::new(190.0, 130.0));
    app.finish_curve();
    let segs: Vec<_> = last_path(&app).segments().collect();
    assert_eq!(segs.len(), 2, "{segs:?}");
    let s0 = cubic(segs[0]);
    assert!(near(s0.p2, Point::new(130.0, 130.0)), "{s0:?}");
    let s1 = cubic(segs[1]);
    assert!(near(s1.p1, b), "{s1:?}");
    assert!(near(s1.p2, Point::new(170.0, 170.0)), "{s1:?}");
}

#[test]
fn dragging_from_the_first_node_shapes_only_the_closing_segment() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::Pen);
    let a = Point::new(100.0, 150.0);
    let a_out = Point::new(110.0, 170.0);
    pull(&ctx, &mut app, a, a_out);
    pull(
        &ctx,
        &mut app,
        Point::new(150.0, 190.0),
        Point::new(170.0, 190.0),
    );
    click_page(&ctx, &mut app, Point::new(160.0, 130.0));
    // The pointer over the first node shows a close.
    let over = app.view.to_screen(a);
    assert_eq!(
        for_tool(&app, false, false, false, Some(over)),
        ToolCursor::PenNib(Some(crate::ui::cursors::CurveMark::Close))
    );
    pull(&ctx, &mut app, a, Point::new(90.0, 140.0));
    assert!(app.curve.is_none());
    let path = last_path(&app);
    let segs: Vec<_> = path.segments().collect();
    // The first node keeps its own outgoing handle.
    let s0 = cubic(segs[0]);
    assert!(near(s0.p1, a_out), "{s0:?}");
    // The closing segment arrives at the first node along the drag: its
    // handle at the first node is where the pointer was let go (#358).
    let last = segs
        .iter()
        .rev()
        .find_map(|s| match s {
            tracedraw_core::geometry::PathSeg::Cubic(c) if near(c.p3, a) => Some(*c),
            _ => None,
        })
        .expect("a curved closing segment");
    assert!(near(last.p2, Point::new(90.0, 140.0)), "{last:?}");
    assert!(near(last.p1, Point::new(160.0, 130.0)), "{last:?}");
}

/// A click on the first node closes the curve with the segment the
/// preview showed: it leaves the last node along its handle and arrives
/// at the first with no handle (#359). Two curved nodes close into a
/// leaf (#356).
#[test]
fn a_click_on_the_first_node_keeps_the_last_handle_and_two_nodes_close() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::Pen);
    let a = Point::new(100.0, 150.0);
    pull(&ctx, &mut app, a, Point::new(110.0, 170.0));
    let b = Point::new(160.0, 150.0);
    let b_out = Point::new(170.0, 130.0);
    pull(&ctx, &mut app, b, b_out);
    // Two nodes: the pointer over the first one shows a close.
    let over = app.view.to_screen(a);
    assert_eq!(
        for_tool(&app, false, false, false, Some(over)),
        ToolCursor::PenNib(Some(crate::ui::cursors::CurveMark::Close))
    );
    click_page(&ctx, &mut app, a);
    assert!(app.curve.is_none(), "closed");
    let path = last_path(&app);
    assert!(
        path.elements()
            .iter()
            .any(|e| matches!(e, tracedraw_core::geometry::PathEl::ClosePath)),
        "{path:?}"
    );
    assert_eq!(tracedraw_core::nodes::nodes(&path).len(), 2, "{path:?}");
    let closing = path
        .segments()
        .filter_map(|s| match s {
            tracedraw_core::geometry::PathSeg::Cubic(c) if near(c.p3, a) => Some(c),
            _ => None,
        })
        .last()
        .expect("a curved closing segment");
    // It leaves the last node along its handle and arrives with none.
    assert!(
        near(closing.p0, b) && near(closing.p1, b_out),
        "{closing:?}"
    );
    assert!(near(closing.p2, a), "{closing:?}");
    // A polyline still needs three nodes to close.
    app.select(Vec::new());
    app.set_tool(Tool::Polyline);
    click_page(&ctx, &mut app, Point::new(20.0, 200.0));
    click_page(&ctx, &mut app, Point::new(60.0, 200.0));
    let over = app.view.to_screen(Point::new(20.0, 200.0));
    assert_ne!(
        for_tool(&app, false, false, false, Some(over)),
        ToolCursor::CurveBadge(Tool::Polyline, crate::ui::cursors::CurveMark::Close)
    );
}

/// A curve from (100, 100) through (150, 100) to (200, 100), selected,
/// with the Shape tool.
fn line_curve(ctx: &egui::Context, app: &mut App) -> ShapeId {
    let mut path = tracedraw_core::BezPath::new();
    path.move_to((100.0, 100.0));
    path.line_to((150.0, 100.0));
    path.line_to((200.0, 100.0));
    let id = app
        .new_shape(ShapeKind::Path {
            path,
            closed: false,
        })
        .expect("curve");
    app.select(vec![id]);
    for _ in 0..2 {
        frame(ctx, app, Vec::new(), Modifiers::NONE);
    }
    id
}

fn node_count(app: &App, id: ShapeId) -> usize {
    match &app.doc().find_shape(id).expect("curve").kind {
        ShapeKind::Path { path, .. } => tracedraw_core::nodes::nodes(path).len(),
        _ => 0,
    }
}

#[test]
fn the_pen_adds_a_node_on_a_segment_and_drags_a_node_of_the_selected_curve() {
    let (ctx, mut app, _) = setup();
    let id = line_curve(&ctx, &mut app);
    app.set_tool(Tool::Pen);
    use crate::ui::cursors::CurveMark;
    let on_segment = app.view.to_screen(Point::new(125.0, 100.0));
    let on_node = app.view.to_screen(Point::new(150.0, 100.0));
    assert_eq!(
        for_tool(&app, false, false, false, Some(on_segment)),
        ToolCursor::PenNib(Some(CurveMark::AddNode))
    );
    // With Auto add/delete (the default) a click on the node deletes it.
    assert_eq!(
        for_tool(&app, false, false, false, Some(on_node)),
        ToolCursor::PenNib(Some(CurveMark::DeleteNode))
    );
    click_at(&ctx, &mut app, on_segment, Modifiers::NONE);
    assert_eq!(node_count(&app, id), 4);
    assert!(app.curve.is_none(), "no new curve was started");
    // Dragging the middle node moves it.
    let to = app.view.to_screen(Point::new(150.0, 130.0));
    drag(&ctx, &mut app, on_node, to, Modifiers::NONE, Vec::new());
    assert!(!app.pen_editing);
    let path = match &app.doc().find_shape(id).expect("curve").kind {
        ShapeKind::Path { path, .. } => path.clone(),
        _ => unreachable!(),
    };
    assert!(
        tracedraw_core::nodes::nodes(&path)
            .iter()
            .any(|n| near(n.pos, Point::new(150.0, 130.0))),
        "{path:?}"
    );
}

#[test]
fn nodes_are_taken_within_the_pick_distance_and_a_near_miss_keeps_the_selection() {
    let (ctx, mut app, _) = setup();
    let id = line_curve(&ctx, &mut app);
    app.set_tool(Tool::Shape);
    // A press 4 screen pixels off the middle node takes it.
    let node = app.view.to_screen(Point::new(150.0, 100.0));
    let from = node + egui::vec2(3.0, 2.5);
    let to = app.view.to_screen(Point::new(150.0, 120.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    let moved = match &app.doc().find_shape(id).expect("curve").kind {
        ShapeKind::Path { path, .. } => tracedraw_core::nodes::nodes(path)
            .iter()
            .any(|n| (n.pos.y - 100.0).abs() > 5.0),
        _ => false,
    };
    assert!(moved, "the node moved");
    let _ = app.undo();
    // A click 10 pixels from the end node and the segment: a near miss,
    // the curve stays selected.
    let end = app.view.to_screen(Point::new(200.0, 100.0));
    let miss = end + egui::vec2(10.0, 0.0);
    click_at(&ctx, &mut app, miss, Modifiers::NONE);
    assert_eq!(app.selection, vec![id]);
    // Clearly on empty canvas: the selection goes.
    let far = end + egui::vec2(120.0, 120.0);
    click_at(&ctx, &mut app, far, Modifiers::NONE);
    assert!(app.selection.is_empty());
}

#[test]
fn the_shape_tool_says_a_rectangle_is_still_a_rectangle() {
    let (ctx, mut app, id) = setup();
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert_eq!(
        crate::ui::status::object_info(&app),
        crate::i18n::tr("status.shape_rectangle")
    );
    app.select(vec![id]);
    app.convert_to_curves();
    let info = crate::ui::status::object_info(&app);
    assert!(info.contains('4'), "{info}");
    assert_ne!(info, crate::i18n::tr("status.shape_rectangle"));
}

#[test]
fn rounded_corners_keep_their_size_when_relative_scaling_is_off() {
    let (ctx, mut app, id) = setup();
    for fixed in [true, false] {
        let mut c = tracedraw_core::Corners::uniform(5.0, tracedraw_core::CornerKind::Round);
        c.fixed = fixed;
        let _ = app.engine.run(&tracedraw_core::Command::SetShapeKind {
            shape: id,
            kind: ShapeKind::rect_with_corners(Rect::new(10.0, 10.0, 60.0, 40.0), c),
        });
        let _ = app.engine.run(&tracedraw_core::Command::TransformShapes {
            shapes: vec![id],
            transform: Affine::IDENTITY,
        });
        app.select(vec![id]);
        app.set_tool(Tool::Pick);
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
        // Twice as wide and high with the corner handle.
        let from = handle(&app, Handle::NE);
        let b = bounds(&app, id);
        let target = Point::new(b.x0 + 2.0 * b.width(), b.y0 + 2.0 * b.height());
        let to = app.view.to_screen(target);
        drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
        let r = app
            .doc()
            .find_shape(id)
            .and_then(|s| s.page_corners())
            .expect("corners")
            .radii[0];
        if fixed {
            assert!(close(r, 5.0), "fixed: {r}");
        } else {
            assert!(r > 8.0, "relative: {r}");
        }
        let _ = app.undo();
    }
}

/// A rectangle and a line of text over it, the rectangle filled or not
/// and below or above the text.
fn text_and_frame(app: &mut App, filled: bool, frame_on_top: bool) -> (ShapeId, ShapeId) {
    use tracedraw_core::{Color, Command, Fill, TextSpan};
    let rect = |app: &mut App| {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 150.0, 80.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: if filled {
                Fill::Solid(Color::rgb8(240, 240, 200))
            } else {
                Fill::None
            },
        });
        id
    };
    let text = |app: &mut App| {
        app.new_shape(ShapeKind::Text {
            spans: vec![TextSpan::new("Invitation", "Sans", 36.0)],
            origin: Point::new(30.0, 50.0),
            frame: None,
            align: tracedraw_core::document::TextAlign::Left,
            para: Default::default(),
            on_path: None,
        })
        .expect("text")
    };
    if frame_on_top {
        let t = text(app);
        (rect(app), t)
    } else {
        let r = rect(app);
        (r, text(app))
    }
}

/// The middle of the text object, in page space.
fn text_middle(app: &App, id: ShapeId) -> Point {
    app.doc().shape_on_page(id).expect("text").bounds().center()
}

/// Write the drawing as `.cdr`, read it back and open it.
fn through_cdr(app: &App) -> App {
    let bytes = tracedraw_cdr::write::document_to_cdr(app.doc());
    let (doc, _) = tracedraw_cdr::open_bytes(&bytes, "t").expect("read back");
    let mut back = App::headless();
    back.open_document(doc, None);
    back.view.zoom = 1.0;
    back
}

/// The ids of the objects on the first layer, bottom to top, by kind.
fn kinds_bottom_up(app: &App) -> Vec<&'static str> {
    app.doc().pages[0]
        .layers
        .iter()
        .flat_map(|l| &l.shapes)
        .map(|s| match s.kind {
            ShapeKind::Text { .. } => "text",
            ShapeKind::ClipFrame { .. } => "clip",
            ShapeKind::Group { .. } => "group",
            _ => "shape",
        })
        .collect()
}

fn first_of(app: &App, kind: &str) -> ShapeId {
    let all: Vec<_> = app.doc().pages[0]
        .layers
        .iter()
        .flat_map(|l| &l.shapes)
        .collect();
    all.iter()
        .zip(kinds_bottom_up(app))
        .find(|(_, k)| *k == kind)
        .map(|(s, _)| s.id)
        .expect("kind")
}

#[test]
fn text_over_a_rectangle_read_from_cdr_is_hit_first() {
    let mut app = App::headless();
    app.view.zoom = 1.0;
    let (rect, text) = text_and_frame(&mut app, true, false);
    let p = text_middle(&app, text);
    assert_eq!(app.hit_test(p), Some(text));
    assert_eq!(app.hits_at(p, false), vec![text, rect]);
    // The file keeps the stacking order: the text is still on top.
    let back = through_cdr(&app);
    assert_eq!(kinds_bottom_up(&back), ["shape", "text"]);
    let t = first_of(&back, "text");
    assert_eq!(back.hit_test(text_middle(&back, t)), Some(t));
}

#[test]
fn an_unfilled_rectangle_over_text_takes_the_click_and_alt_reaches_the_text() {
    let (ctx, mut app, first) = setup();
    app.select(vec![first]);
    app.delete_selection();
    let (rect, text) = text_and_frame(&mut app, false, true);
    let at = app.view.to_screen(text_middle(&app, text));
    // Treated as filled (the default), the empty rectangle on top is hit
    // inside; Alt digs to the text under it.
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert_eq!(app.selection, vec![rect]);
    click_at(&ctx, &mut app, at, Modifiers::ALT);
    assert_eq!(app.selection, vec![text]);
    // Not treated as filled, the text is hit straight away.
    app.settings.treat_all_filled = false;
    app.select(Vec::new());
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert_eq!(app.selection, vec![text]);
    // The same stacking after a round trip through `.cdr`.
    let back = through_cdr(&app);
    assert_eq!(kinds_bottom_up(&back), ["text", "shape"]);
}

/// Moving objects over a clip frame: dropped as they are they land over
/// it; with W held they go into it; with K held they replace its
/// contents. The frame under the pointer is noted during the move.
#[test]
fn dropping_on_a_clip_frame_with_w_or_k_puts_the_objects_in() {
    let (ctx, mut app, first) = setup();
    app.select(vec![first]);
    app.delete_selection();
    let (rect, text) = text_and_frame(&mut app, true, false);
    app.place_inside(vec![text], rect);
    let clip = app.doc().top_level_of(text).expect("clip frame");
    app.settings.clip_frame.auto_center = crate::settings::AutoCenter::Never;
    // A small square away from the frame.
    let square = |app: &mut App| {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(200.0, 20.0, 220.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("square")
    };
    let from = app.view.to_screen(Point::new(210.0, 30.0));
    let to = app.view.to_screen(Point::new(60.0, 45.0));
    let hold = |k: egui::Key| vec![key(k, Modifiers::NONE)];
    let release = |k: egui::Key| Event::Key {
        key: k,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    // No key: the square lands over the frame, on its own.
    let a = square(&mut app);
    app.select(vec![a]);
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    assert_eq!(app.doc().top_level_of(a), Some(a));
    assert!(app.drop_frame.is_none());
    app.select(vec![a]);
    app.delete_selection();
    // W held during the move: into the frame, which is then selected.
    let b = square(&mut app);
    app.select(vec![b]);
    drag(
        &ctx,
        &mut app,
        from,
        to,
        Modifiers::NONE,
        hold(egui::Key::W),
    );
    frame(&ctx, &mut app, vec![release(egui::Key::W)], Modifiers::NONE);
    assert_eq!(app.doc().top_level_of(b), Some(clip), "in the frame");
    assert!(
        app.doc().find_shape(text).is_some(),
        "the old contents stay"
    );
    assert_eq!(app.selection, vec![clip]);
    assert_eq!(app.tool, Tool::Pick, "W picked no tool");
    // K held: the old contents go, the square takes their place.
    let c = square(&mut app);
    app.select(vec![c]);
    let depth = app.engine.history_labels().0.len();
    drag(
        &ctx,
        &mut app,
        from,
        to,
        Modifiers::NONE,
        hold(egui::Key::K),
    );
    frame(&ctx, &mut app, vec![release(egui::Key::K)], Modifiers::NONE);
    assert_eq!(app.doc().top_level_of(c), Some(clip));
    assert!(app.doc().find_shape(text).is_none(), "replaced");
    assert!(app.doc().find_shape(b).is_none(), "replaced");
    assert_eq!(
        app.engine.history_labels().0.len(),
        depth + 2,
        "the move, then the drop"
    );
    assert_eq!(app.tool, Tool::Pick);
}

#[test]
fn ctrl_click_takes_the_contents_of_a_clip_frame() {
    let (ctx, mut app, first) = setup();
    app.select(vec![first]);
    app.delete_selection();
    let (rect, text) = text_and_frame(&mut app, true, false);
    app.place_inside(vec![text], rect);
    let clip = app.doc().top_level_of(text).expect("clip frame");
    let inside = app.view.to_screen(text_middle(&app, text));
    click_at(&ctx, &mut app, inside, Modifiers::NONE);
    assert_eq!(app.selection, vec![clip]);
    click_at(&ctx, &mut app, inside, Modifiers::CTRL);
    assert_eq!(app.selection, vec![text]);
    // Ctrl+Alt digs through the contents to the frame.
    assert_eq!(app.hits_at(text_middle(&app, text), true), vec![text, clip]);
    // Inside the frame but off the text: the frame.
    app.select(Vec::new());
    let off = app.view.to_screen(Point::new(12.0, 12.0));
    click_at(&ctx, &mut app, off, Modifiers::CTRL);
    assert_eq!(app.selection, vec![clip]);
}

#[test]
fn clipped_away_contents_are_not_hit() {
    use tracedraw_core::TextSpan;
    let mut app = App::headless();
    app.view.zoom = 1.0;
    let frame = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(10.0, 10.0, 40.0, 80.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    let text = app
        .new_shape(ShapeKind::Text {
            spans: vec![TextSpan::new("Invitation", "Sans", 36.0)],
            origin: Point::new(30.0, 50.0),
            frame: None,
            align: tracedraw_core::document::TextAlign::Left,
            para: Default::default(),
            on_path: None,
        })
        .expect("text");
    let b = app.doc().shape(text).expect("text").1.bounds();
    assert!(b.x1 > 50.0, "{b:?}");
    app.place_inside(vec![text], frame);
    let clip = app.doc().top_level_of(text).expect("clip frame");
    let y = b.center().y;
    // On the text inside the frame: Ctrl reaches it.
    assert_eq!(app.hit_test_step(Point::new(35.0, y)), Some(text));
    // Its letters past the frame do not show and are not hit.
    let past = Point::new((40.0 + b.x1) / 2.0, y);
    assert_eq!(app.hit_test(past), None);
    assert!(app.hits_at(past, true).is_empty());
    assert_eq!(app.hit_test(Point::new(35.0, y)), Some(clip));
}

#[test]
fn new_rounded_rectangles_keep_circular_corners_when_stretched() {
    use crate::ui::propbar::{edit_corners, CornerEdit};
    use tracedraw_core::geometry::Shape as _;
    let (ctx, mut app, _) = setup();
    // Relative corner scaling is off for new rectangles.
    assert!(app.rect_corners.fixed);
    app.select(Vec::new());
    edit_corners(&mut app, CornerEdit::Size(0, 5.0));
    app.set_tool(Tool::Rectangle);
    app.create_box_shape(Point::new(100.0, 100.0), Point::new(150.0, 130.0));
    let id = app.selected_shapes()[0].id;
    app.set_tool(Tool::Pick);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // Only twice as wide, with the side handle.
    let stretch = |app: &mut App, which: Handle, fx: f64, fy: f64| {
        let from = handle(app, which);
        let b = bounds(app, id);
        let target = Point::new(b.x0 + fx * b.width(), b.y0 + fy * b.height());
        let mut to = app.view.to_screen(target);
        match which {
            Handle::E => to.y = from.y,
            _ => to.x = from.x,
        }
        drag(&ctx, app, from, to, Modifiers::NONE, Vec::new());
    };
    stretch(&mut app, Handle::E, 2.0, 1.0);
    let s = app.doc().find_shape(id).expect("rect").clone();
    let c = s.page_corners().expect("corners");
    assert!(c.fixed && c.radii.iter().all(|r| close(*r, 5.0)), "{c:?}");
    // Circles of 5 mm: the area the four corners cut off is (4 - pi) r^2.
    let path = s.page_path();
    let b = path.bounding_box();
    assert!(b.width() > 90.0, "{b:?}");
    let cut = b.width() * b.height() - path.area().abs();
    let circles = (4.0 - std::f64::consts::PI) * 25.0;
    assert!((cut - circles).abs() < 0.2, "{cut} vs {circles}");
    // Relative scaling turned on: the corners grow with the rectangle.
    app.select(vec![id]);
    edit_corners(&mut app, CornerEdit::Scaling(true));
    let s = app.doc().find_shape(id).expect("rect").clone();
    let c = s.page_corners().expect("corners");
    assert!(!c.fixed && close(c.radii[0], 5.0), "{c:?}");
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    stretch(&mut app, Handle::N, 1.0, 2.0);
    let r = app
        .doc()
        .find_shape(id)
        .and_then(|s| s.page_corners())
        .expect("corners")
        .radii[0];
    assert!(r > 8.0, "relative: {r}");
}

#[test]
fn the_pen_deletes_a_node_of_the_selected_curve_and_the_toggle_turns_it_off() {
    use crate::ui::cursors::CurveMark;
    let (ctx, mut app, _) = setup();
    let id = line_curve(&ctx, &mut app);
    app.set_tool(Tool::Pen);
    let on_node = app.view.to_screen(Point::new(150.0, 100.0));
    let on_segment = app.view.to_screen(Point::new(125.0, 100.0));
    click_at(&ctx, &mut app, on_node, Modifiers::NONE);
    assert_eq!(node_count(&app, id), 2);
    assert!(app.curve.is_none(), "no new curve was started");
    // One undo step; a curve keeps two nodes.
    let _ = app.undo();
    assert_eq!(node_count(&app, id), 3);
    // Auto add/delete off: the node is only edited, a segment click adds
    // nothing.
    app.settings.curves.pen_auto_add_delete = false;
    assert_eq!(
        for_tool(&app, false, false, false, Some(on_node)),
        ToolCursor::PenNib(Some(CurveMark::EditNode))
    );
    assert_eq!(
        for_tool(&app, false, false, false, Some(on_segment)),
        ToolCursor::PenNib(None)
    );
    click_at(&ctx, &mut app, on_node, Modifiers::NONE);
    assert_eq!(node_count(&app, id), 3);
    // The Bezier tool adds on a segment and never deletes.
    app.settings.curves.pen_auto_add_delete = true;
    app.finish_curve();
    app.select(vec![id]);
    app.set_tool(Tool::Bezier);
    assert_eq!(
        for_tool(&app, false, false, false, Some(on_node)),
        ToolCursor::CurveBadge(Tool::Bezier, CurveMark::EditNode)
    );
    assert_eq!(
        for_tool(&app, false, false, false, Some(on_segment)),
        ToolCursor::CurveBadge(Tool::Bezier, CurveMark::AddNode)
    );
    click_at(&ctx, &mut app, on_node, Modifiers::NONE);
    assert!(node_count(&app, id) >= 3);
}

#[test]
fn the_pen_adds_and_deletes_nodes_on_the_curve_being_drawn() {
    use crate::ui::cursors::CurveMark;
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::Pen);
    for x in [100.0, 140.0, 180.0] {
        click_page(&ctx, &mut app, Point::new(x, 150.0));
    }
    assert_eq!(app.curve.as_ref().map(|c| c.nodes.len()), Some(3));
    let on_segment = app.view.to_screen(Point::new(120.0, 150.0));
    let on_node = app.view.to_screen(Point::new(140.0, 150.0));
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(on_segment)],
        Modifiers::NONE,
    );
    assert_eq!(
        for_tool(&app, false, false, false, Some(on_segment)),
        ToolCursor::PenNib(Some(CurveMark::AddNode))
    );
    assert_eq!(
        for_tool(&app, false, false, false, Some(on_node)),
        ToolCursor::PenNib(Some(CurveMark::DeleteNode))
    );
    click_page(&ctx, &mut app, Point::new(120.0, 150.0));
    let xs: Vec<f64> = app
        .curve
        .as_ref()
        .map(|c| c.nodes.iter().map(|n| n.0.x.round()).collect())
        .unwrap_or_default();
    assert_eq!(xs, vec![100.0, 120.0, 140.0, 180.0]);
    click_page(&ctx, &mut app, Point::new(140.0, 150.0));
    let xs: Vec<f64> = app
        .curve
        .as_ref()
        .map(|c| c.nodes.iter().map(|n| n.0.x.round()).collect())
        .unwrap_or_default();
    assert_eq!(xs, vec![100.0, 120.0, 180.0]);
    // Off: a click there places a node.
    app.settings.curves.pen_auto_add_delete = false;
    click_page(&ctx, &mut app, Point::new(110.0, 150.0));
    assert_eq!(app.curve.as_ref().map(|c| c.nodes.len()), Some(4));
}

#[test]
fn c_held_while_pulling_makes_a_cusp_that_stays_and_does_not_align() {
    let (ctx, mut app, rect) = setup();
    // The rectangle stays selected: C must not align it.
    let before = bounds(&app, rect);
    app.set_tool(Tool::Pen);
    app.select(vec![rect]);
    let (a, b, c) = (
        Point::new(100.0, 150.0),
        Point::new(140.0, 150.0),
        Point::new(180.0, 150.0),
    );
    click_page(&ctx, &mut app, a);
    let (from, to) = (
        app.view.to_screen(b),
        app.view.to_screen(Point::new(160.0, 170.0)),
    );
    drag(
        &ctx,
        &mut app,
        from,
        to,
        Modifiers::NONE,
        vec![key(egui::Key::C, Modifiers::NONE)],
    );
    let release = Event::Key {
        key: egui::Key::C,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    frame(&ctx, &mut app, vec![release.clone()], Modifiers::NONE);
    let cv = app.curve.clone().expect("drawing");
    // The incoming handle was frozen before the pull ended: it is not the
    // mirror of the outgoing one.
    let out = cv.nodes[1].1.expect("outgoing handle");
    assert!(near(out, Point::new(160.0, 170.0)), "{out:?}");
    let frozen = cv.in_handles.get(&1).copied().expect("a cusp");
    assert!(
        frozen.is_none_or(|h| !near(h, Point::new(120.0, 130.0))),
        "{frozen:?}"
    );
    assert_eq!(bounds(&app, rect), before, "C did not align the rectangle");
    // C pressed before the press: the node is a cusp from the start (the
    // segment arrives without a handle), and it stays one after C is let
    // go.
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::C, Modifiers::NONE)],
        Modifiers::NONE,
    );
    pull(&ctx, &mut app, c, Point::new(190.0, 130.0));
    frame(&ctx, &mut app, vec![release], Modifiers::NONE);
    let cv = app.curve.clone().expect("drawing");
    assert_eq!(cv.in_handles.get(&2).copied(), Some(None));
    assert!(cv.in_handles.contains_key(&1), "the first cusp stayed");
    // Without C the next pull is smooth.
    pull(
        &ctx,
        &mut app,
        Point::new(220.0, 150.0),
        Point::new(230.0, 160.0),
    );
    let cv = app.curve.clone().expect("drawing");
    assert!(!cv.in_handles.contains_key(&3));
    assert_eq!(bounds(&app, rect), before);
}

#[test]
fn c_during_the_closing_drag_keeps_the_first_node_handle() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::Pen);
    let a = Point::new(100.0, 150.0);
    let a_out = Point::new(110.0, 170.0);
    pull(&ctx, &mut app, a, a_out);
    pull(
        &ctx,
        &mut app,
        Point::new(150.0, 190.0),
        Point::new(170.0, 190.0),
    );
    click_page(&ctx, &mut app, Point::new(160.0, 130.0));
    let (from, to) = (
        app.view.to_screen(a),
        app.view.to_screen(Point::new(90.0, 140.0)),
    );
    drag(
        &ctx,
        &mut app,
        from,
        to,
        Modifiers::NONE,
        vec![key(egui::Key::C, Modifiers::NONE)],
    );
    assert!(app.curve.is_none());
    let segs: Vec<_> = last_path(&app).segments().collect();
    assert!(near(cubic(segs[0]).p1, a_out), "{segs:?}");
    let closing = segs
        .iter()
        .rev()
        .find_map(|s| match s {
            tracedraw_core::geometry::PathSeg::Cubic(c) if near(c.p3, a) => Some(*c),
            _ => None,
        })
        .expect("a curved closing segment");
    assert!(near(closing.p2, Point::new(90.0, 140.0)), "{closing:?}");
}

#[test]
fn the_snap_tip_shows_while_placing_and_pulling_with_the_pen() {
    use crate::snap_points::SnapMode;
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.snap.off = false;
    app.snap.grid = true;
    app.snap.objects = false;
    app.snap.guides = false;
    app.snap.page = false;
    app.set_tool(Tool::Pen);
    click_page(&ctx, &mut app, Point::new(100.2, 150.3));
    // Placing: the pointer near a grid crossing is marked "grid".
    let near_grid = app.view.to_screen(Point::new(120.3, 140.2));
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(near_grid)],
        Modifiers::NONE,
    );
    let mark = app.snap_mark.get().expect("a snap mark");
    assert_eq!(mark.mode, SnapMode::Grid);
    assert!(
        (mark.point - Point::new(120.0, 140.0)).hypot() < 1e-6,
        "{mark:?}"
    );
    // Pulling a handle: the handle snaps and the mark shows too.
    let from = app.view.to_screen(Point::new(140.0, 150.0));
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(from)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(from, true, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    for p in [Point::new(150.0, 160.0), Point::new(160.3, 169.8)] {
        let at = app.view.to_screen(p);
        frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(at)],
            Modifiers::NONE,
        );
    }
    assert!(app.curve.as_ref().is_some_and(|c| c.dragging_handle));
    let mark = app.snap_mark.get().expect("a snap mark while pulling");
    assert_eq!(mark.mode, SnapMode::Grid);
    let handle = app
        .curve
        .as_ref()
        .and_then(|c| c.nodes.last())
        .and_then(|n| n.1);
    assert!(
        handle.is_some_and(|h| (h - Point::new(160.0, 170.0)).hypot() < 1e-6),
        "{handle:?}"
    );
    let up = app.view.to_screen(Point::new(160.3, 169.8));
    frame(
        &ctx,
        &mut app,
        vec![button(up, false, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
}

#[test]
fn space_finishes_a_bezier_curve_then_picks() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::Bezier);
    let before = shape_count(&app);
    let pts = [
        Point::new(100.0, 150.0),
        Point::new(140.0, 170.0),
        Point::new(180.0, 150.0),
    ];
    for p in pts {
        let at = app.view.to_screen(p);
        click_at(&ctx, &mut app, at, Modifiers::NONE);
    }
    assert!(app.curve.is_some());
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Space, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert!(app.curve.is_none(), "Space ends the curve");
    assert_eq!(shape_count(&app), before + 1);
    // Space is also the Pick tool's key: the curve is kept, then Pick.
    assert_eq!(app.tool, Tool::Pick);
    let nodes = tracedraw_core::nodes::nodes(&last_path(&app));
    assert_eq!(nodes.len(), 3, "{nodes:?}");
}

/// Esc while a curve is drawn with the Pen (or Bezier) finishes it as
/// it stands, keeping the nodes placed; the tool stays. A lone first
/// node is dropped (#314).
#[test]
fn esc_finishes_the_curve_being_drawn_and_keeps_it() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::Pen);
    let before = shape_count(&app);
    let pts = [
        Point::new(100.0, 150.0),
        Point::new(140.0, 170.0),
        Point::new(180.0, 150.0),
    ];
    for p in pts {
        let at = app.view.to_screen(p);
        click_at(&ctx, &mut app, at, Modifiers::NONE);
    }
    assert!(app.curve.is_some());
    let esc = || vec![key(egui::Key::Escape, Modifiers::NONE)];
    frame(&ctx, &mut app, esc(), Modifiers::NONE);
    assert!(app.curve.is_none(), "Esc ends the curve");
    assert_eq!(shape_count(&app), before + 1, "the curve is kept");
    assert_eq!(app.tool, Tool::Pen, "the tool stays");
    let nodes = tracedraw_core::nodes::nodes(&last_path(&app));
    assert_eq!(nodes.len(), 3, "{nodes:?}");
    // One node only: nothing to keep.
    app.select(Vec::new());
    let at = app.view.to_screen(Point::new(60.0, 60.0));
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert!(app.curve.is_some());
    frame(&ctx, &mut app, esc(), Modifiers::NONE);
    assert!(app.curve.is_none());
    assert_eq!(shape_count(&app), before + 1);
}

/// With a real mouse the right button goes down and up in frames of its
/// own while the left one is held; the move goes on after it.
#[test]
fn the_move_goes_on_after_a_right_click() {
    let (ctx, mut app, id) = setup();
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    let n = Modifiers::NONE;
    let right = |p: Pos2, down: bool| button(p, down, PointerButton::Secondary, n);
    frame(&ctx, &mut app, vec![Event::PointerMoved(from)], n);
    frame(
        &ctx,
        &mut app,
        vec![button(from, true, PointerButton::Primary, n)],
        n,
    );
    let mut p = from;
    for _ in 0..4 {
        p += egui::vec2(20.0, 0.0);
        frame(&ctx, &mut app, vec![Event::PointerMoved(p)], n);
    }
    frame(&ctx, &mut app, vec![right(p, true)], n);
    p += egui::vec2(5.0, 0.0);
    frame(&ctx, &mut app, vec![Event::PointerMoved(p)], n);
    frame(&ctx, &mut app, vec![right(p, false)], n);
    for _ in 0..4 {
        p += egui::vec2(20.0, 10.0);
        frame(&ctx, &mut app, vec![Event::PointerMoved(p)], n);
    }
    assert!(app.transforming(), "the move goes on");
    frame(
        &ctx,
        &mut app,
        vec![button(p, false, PointerButton::Primary, n)],
        n,
    );
    frame(&ctx, &mut app, Vec::new(), n);
    assert_eq!(shape_count(&app), 2);
    assert!(close(bounds(&app, id).x0, 10.0), "the original stays");
    let copy = app.selection.first().copied().expect("copy");
    assert_ne!(copy, id);
    let moved = app.view.to_page(p) - app.view.to_page(from);
    let b = bounds(&app, copy);
    assert!(close(b.x0, 10.0 + moved.x), "{b:?} {moved:?}");
    assert!(close(b.y0, 10.0 + moved.y), "{b:?} {moved:?}");
    assert!(!app.transforming());
}

#[test]
fn ctrl_z_takes_back_pen_nodes_one_by_one() {
    let (ctx, mut app, id) = setup();
    app.set_tool(Tool::Pen);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let undo_before = app.engine.undo_label();
    for q in [
        Point::new(20.0, 60.0),
        Point::new(60.0, 60.0),
        Point::new(40.0, 90.0),
    ] {
        let at = app.view.to_screen(q);
        click_at(&ctx, &mut app, at, Modifiers::NONE);
    }
    let count = |app: &App| app.curve.as_ref().map(|c| c.nodes.len());
    assert_eq!(count(&app), Some(3));
    let ctrl = Modifiers::COMMAND | Modifiers::CTRL;
    let press = |ctx: &egui::Context, app: &mut App, mods: Modifiers| {
        frame(ctx, app, vec![key(egui::Key::Z, mods)], mods);
    };
    // Ctrl+Z takes the last node back, Ctrl+Shift+Z puts it again.
    press(&ctx, &mut app, ctrl);
    assert_eq!(count(&app), Some(2));
    press(&ctx, &mut app, ctrl | Modifiers::SHIFT);
    assert_eq!(count(&app), Some(3));
    // Node by node until the curve is gone; the document is untouched.
    for want in [Some(2), Some(1), None] {
        press(&ctx, &mut app, ctrl);
        assert_eq!(count(&app), want);
    }
    assert!(app.doc().find_shape(id).is_some());
    assert_eq!(app.engine.undo_label(), undo_before);
    // Redo brings the nodes back one by one.
    press(&ctx, &mut app, ctrl | Modifiers::SHIFT);
    assert_eq!(count(&app), Some(1));
    press(&ctx, &mut app, ctrl | Modifiers::SHIFT);
    assert_eq!(count(&app), Some(2));
    // Once the curve is finished it is one step of the document's history.
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Enter, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert!(app.curve.is_none());
    assert_ne!(app.engine.undo_label(), undo_before);
    // Undoing it leaves the Pen on the first node, drawing again (#631).
    press(&ctx, &mut app, ctrl);
    assert_eq!(count(&app), Some(1));
    assert_eq!(app.engine.undo_label(), undo_before);
}

#[test]
fn a_right_click_on_a_locked_object_offers_to_unlock_it() {
    let (ctx, mut app, id) = setup();
    app.select(vec![id]);
    app.set_locked(true);
    assert!(app.selection.is_empty());
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let right_click = |ctx: &egui::Context, app: &mut App, at: Pos2| {
        frame(ctx, app, vec![Event::PointerMoved(at)], Modifiers::NONE);
        frame(
            ctx,
            app,
            vec![button(at, true, PointerButton::Secondary, Modifiers::NONE)],
            Modifiers::NONE,
        );
        frame(
            ctx,
            app,
            vec![button(at, false, PointerButton::Secondary, Modifiers::NONE)],
            Modifiers::NONE,
        );
    };
    // On the locked rectangle's outline: its menu opens for it, and it is
    // not selected (so it cannot be dragged or edited).
    let on = app.view.to_screen(Point::new(35.0, 10.0));
    right_click(&ctx, &mut app, on);
    assert!(app.context_menu.is_some());
    assert_eq!(app.context_locked, Some(id));
    assert!(app.selection.is_empty());
    // Unlock Object: unlocked and selected.
    app.unlock_object(id);
    assert!(app.doc().find_shape(id).is_some_and(|s| !s.locked));
    assert_eq!(app.selection, vec![id]);
    // Away from it, or on an unlocked object, there is no locked object.
    app.context_menu = None;
    app.context_locked = None;
    right_click(&ctx, &mut app, on);
    assert_eq!(app.context_locked, None);
    let empty = app.view.to_screen(Point::new(150.0, 150.0));
    right_click(&ctx, &mut app, empty);
    assert_eq!(app.context_locked, None);
}

#[test]
fn removed_and_alt_shortcuts_act_on_the_keyboard() {
    let (ctx, mut app, id) = setup();
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let ctrl = Modifiers::COMMAND | Modifiers::CTRL;
    let press = |ctx: &egui::Context, app: &mut App, k: egui::Key, mods: Modifiers| {
        frame(ctx, app, vec![key(k, mods)], mods);
    };
    // Undo's Ctrl+Z removed: the key does nothing, nothing is undone.
    app.settings
        .shortcuts
        .push(("menu.edit.undo".into(), String::new()));
    let last = app.engine.undo_label();
    assert!(last.is_some());
    press(&ctx, &mut app, egui::Key::Z, ctrl);
    assert_eq!(app.engine.undo_label(), last);
    assert!(app.doc().find_shape(id).is_some());
    // Given back: Ctrl+Z undoes again.
    app.settings.shortcuts.clear();
    press(&ctx, &mut app, egui::Key::Z, ctrl);
    assert_ne!(app.engine.undo_label(), last);
    // A tool takes Alt with a key as menu commands do.
    app.settings
        .shortcuts
        .push(("ellipse".into(), "Alt+Shift+K".into()));
    assert_ne!(app.tool, Tool::Ellipse);
    press(
        &ctx,
        &mut app,
        egui::Key::K,
        Modifiers::ALT | Modifiers::SHIFT,
    );
    assert_eq!(app.tool, Tool::Ellipse);
    // A second key of the tool's picks it too; the first still does
    // (#822).
    app.settings
        .shortcuts
        .push(("ellipse".into(), "Alt+Shift+J".into()));
    for k in [egui::Key::J, egui::Key::K] {
        app.set_tool(Tool::Pick);
        press(&ctx, &mut app, k, Modifiers::ALT | Modifiers::SHIFT);
        assert_eq!(app.tool, Tool::Ellipse, "{k:?}");
    }
}

#[test]
fn ctrl_keeps_a_dimension_leg_to_the_constrain_angle() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::ParallelDimension);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let ctrl = Modifiers::COMMAND | Modifiers::CTRL;
    let first = Point::new(100.0, 150.0);
    let at = app.view.to_screen(first);
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert_eq!(app.dimension_points.len(), 1);
    let a = app.dimension_points[0];
    // A second point a little off the horizontal, Ctrl held: the leg lies
    // on it.
    let off = Point::new(160.0, 154.0);
    let at = app.view.to_screen(off);
    click_at(&ctx, &mut app, at, ctrl);
    assert_eq!(app.dimension_points.len(), 2);
    let b = app.dimension_points[1];
    assert!((b.y - a.y).abs() < 1e-6, "{a:?} {b:?}");
    assert!(b.x > a.x + 50.0);
    // Without Ctrl the point is taken as it is.
    app.dimension_points = vec![a];
    let at = app.view.to_screen(off);
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    let free = app.dimension_points[1];
    assert!((free.y - a.y).abs() > 2.0, "{free:?}");
    // The angular tool keeps each side from the vertex.
    app.set_tool(Tool::AngularDimension);
    app.dimension_points = vec![first];
    let near_vertical = Point::new(103.0, 200.0);
    let at = app.view.to_screen(near_vertical);
    click_at(&ctx, &mut app, at, ctrl);
    let side = app.dimension_points[1];
    assert!((side.x - first.x).abs() < 1e-6, "{side:?}");
}

#[test]
fn the_page_bar_does_nothing_on_the_welcome_screen() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let size = |app: &App| {
        let p = app.doc().page(app.page).expect("page");
        (p.size.width, p.size.height)
    };
    let portrait = size(&app);
    assert!(portrait.1 > portrait.0, "{portrait:?}");
    // The page bar's landscape button: found along the bar's row, as
    // the first click there that turns the page.
    let mut landscape = None;
    for x in (380..560).step_by(4) {
        let at = egui::pos2(x as f32, 87.0);
        click_at(&ctx, &mut app, at, Modifiers::NONE);
        if size(&app).0 > size(&app).1 {
            landscape = Some(at);
            break;
        }
    }
    let landscape = landscape.expect("the landscape button is on the bar");
    let _ = app.engine.undo();
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert_eq!(size(&app), portrait);
    // The Welcome Screen shown (the drawing behind it): the bar is pale.
    app.show_welcome = true;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    click_at(&ctx, &mut app, landscape, Modifiers::NONE);
    assert_eq!(size(&app), portrait);
    // Back on the drawing, the same click turns the page.
    app.show_welcome = false;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    click_at(&ctx, &mut app, landscape, Modifiers::NONE);
    let turned = size(&app);
    assert!(turned.0 > turned.1, "{turned:?}");
}

#[test]
fn the_shape_tool_takes_the_nearest_node_of_a_scaled_curve() {
    let (ctx, mut app, id) = setup();
    app.select(vec![id]);
    app.delete_selection();
    // A small curve in its own units, then scaled ten times on the page,
    // as text converted to curves can be: nodes 1 mm apart locally are
    // 10 mm apart on the page.
    let mut path = tracedraw_core::BezPath::new();
    path.move_to((2.0, 2.0));
    path.line_to((3.0, 2.0));
    path.line_to((4.0, 2.0));
    path.line_to((4.0, 3.0));
    let c = app
        .new_shape(ShapeKind::Path {
            path,
            closed: false,
        })
        .expect("curve");
    app.select(vec![c]);
    app.transform_selection(tracedraw_core::geometry::Affine::scale(10.0));
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let page = |x: f64, y: f64| Point::new(x * 10.0, y * 10.0);
    // On each node, that node; a little off it, still the nearest.
    for (i, q) in [
        (0usize, page(2.0, 2.0)),
        (1, page(3.0, 2.0)),
        (2, page(4.0, 2.0)),
        (3, page(4.0, 3.0)),
    ] {
        let hit = app.node_at(q).expect("node");
        assert_eq!((hit.shape, hit.index), (c, i), "at {q:?}");
        let off = Point::new(q.x + 0.4, q.y - 0.3);
        assert_eq!(app.node_at(off).map(|h| h.index), Some(i), "near {q:?}");
    }
    // Halfway between two nodes (5 mm from each): beyond the pick
    // distance, no node.
    assert!(app.node_at(page(3.5, 2.0)).is_none());
    // A segment is found on the page too: the middle of the last one.
    let (_, seg, _) = app.segment_at(page(4.0, 2.5)).expect("segment");
    assert_eq!(seg, 3);
}

/// A browser gives a right button pressed while the left one is held as
/// a pointer move, so it is noted outside egui's events (`web.rs`); the
/// next frame of the drag takes it as the right click that leaves the
/// original. The desktop never notes one.
#[test]
fn a_noted_chorded_right_press_leaves_the_original_too() {
    assert!(crate::interaction::is_chorded_right_press(2, 3));
    assert!(
        !crate::interaction::is_chorded_right_press(2, 2),
        "left not held"
    );
    assert!(
        !crate::interaction::is_chorded_right_press(0, 1),
        "the left button"
    );
    let (ctx, mut app, id) = setup();
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    let to = from + egui::vec2(150.0, 0.0);
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(from)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(from, true, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let mid = from + (to - from) * 0.5;
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(mid)],
        Modifiers::NONE,
    );
    assert!(!app.drag_copy);
    crate::interaction::note_chorded_right_press();
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(mid + egui::vec2(10.0, 0.0))],
        Modifiers::NONE,
    );
    assert!(app.drag_copy, "the press is taken on the next frame");
    assert_eq!(app.status, crate::i18n::tr("status.drag_copy_on"));
    // Taken once: the next frame does not take it back.
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(mid + egui::vec2(20.0, 0.0))],
        Modifiers::NONE,
    );
    assert!(app.drag_copy);
    // A second press takes it back, a third sets it again.
    crate::interaction::note_chorded_right_press();
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(mid + egui::vec2(30.0, 0.0))],
        Modifiers::NONE,
    );
    assert!(!app.drag_copy);
    crate::interaction::note_chorded_right_press();
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(to)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(to, false, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert_eq!(shape_count(&app), 2);
    assert!(close(bounds(&app, id).x0, 10.0), "the original stays");
    let copy = app.selection.first().copied().expect("copy");
    assert_ne!(copy, id);
    assert!(bounds(&app, copy).x0 > 15.0);
}

#[test]
fn a_clip_frame_is_painted_and_reshaped_through_its_frame() {
    let (ctx, mut app, a) = setup();
    let b = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(20.0, 15.0, 40.0, 30.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.place_inside(vec![b], a);
    app.select(vec![a]);
    let frame_of = |app: &App| match &app.doc().find_shape(a).expect("clip").kind {
        ShapeKind::ClipFrame { frame, .. } => (**frame).clone(),
        other => panic!("not a clip frame: {other:?}"),
    };
    let fid = frame_of(&app).id;
    assert_ne!(fid, a);
    // Painting.
    let blue = tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(0, 0, 200));
    app.apply_fill(blue.clone());
    assert_eq!(frame_of(&app).fill, blue);
    assert_eq!(
        app.paint_targets()[0].fill,
        blue,
        "the readouts show the frame's"
    );
    assert_ne!(
        app.doc().find_shape(b).expect("contents").fill,
        blue,
        "the contents keep theirs"
    );
    app.apply_outline_width(1.5);
    assert_eq!(frame_of(&app).stroke.map(|s| s.width), Some(1.5));
    // The Shape tool: the frame's corner node, dragged along the side,
    // rounds the corners.
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let (s, _) = app
        .kind_node_at(Point::new(60.0, 40.0))
        .expect("a corner node");
    assert_eq!(s.id, fid);
    let corner = app.view.to_screen(Point::new(60.0, 40.0));
    let along = app.view.to_screen(Point::new(50.0, 40.0));
    drag(&ctx, &mut app, corner, along, Modifiers::NONE, Vec::new());
    let f = frame_of(&app);
    assert!(
        matches!(f.kind, ShapeKind::Rect { radius, .. } if radius > 5.0),
        "{:?}",
        f.kind
    );
    assert!(
        app.doc().find_shape(b).is_some(),
        "the contents are still inside"
    );
    // Still movable with the Pick tool.
    app.set_tool(Tool::Pick);
    app.select(vec![a]);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let before = bounds(&app, a);
    let from = app.view.to_screen(Point::new(15.0, 15.0));
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(100.0, 0.0),
        Modifiers::NONE,
        Vec::new(),
    );
    assert!(bounds(&app, a).x0 > before.x0 + 5.0);
    assert!(matches!(
        app.doc().find_shape(a).expect("clip").kind,
        ShapeKind::ClipFrame { .. }
    ));
}

/// The key that keeps a drawn segment to the constrain angle is chosen
/// in Options > Tools > Curve tools: Ctrl at first, Shift or Alt on
/// request; the others then do nothing of the kind (#247).
#[test]
fn the_constrain_key_can_be_shift_or_alt() {
    let level = |app: &App| {
        let pts: Vec<Point> = last_path(app)
            .elements()
            .iter()
            .filter_map(|e| match e {
                tracedraw_core::geometry::PathEl::MoveTo(p)
                | tracedraw_core::geometry::PathEl::LineTo(p) => Some(*p),
                _ => None,
            })
            .collect();
        pts.len() == 2 && (pts[0].y - pts[1].y).abs() < 1e-6
    };
    for (key, mods, other) in [
        (
            crate::prefs::ConstrainKey::Shift,
            Modifiers::SHIFT,
            Modifiers::CTRL,
        ),
        (
            crate::prefs::ConstrainKey::Alt,
            Modifiers::ALT,
            Modifiers::CTRL,
        ),
        (
            crate::prefs::ConstrainKey::Ctrl,
            Modifiers::CTRL,
            Modifiers::SHIFT,
        ),
    ] {
        let (ctx, mut app, _id) = setup();
        app.settings.curves.constrain_key = key;
        app.set_tool(Tool::Freehand);
        let a = app.view.to_screen(Point::new(100.0, 100.0));
        let b = app.view.to_screen(Point::new(160.0, 104.0));
        click_at(&ctx, &mut app, a, Modifiers::NONE);
        click_at(&ctx, &mut app, b, mods);
        assert!(level(&app), "{key:?} held: level");
        app.set_tool(Tool::Freehand);
        let a = app.view.to_screen(Point::new(100.0, 150.0));
        let b = app.view.to_screen(Point::new(160.0, 154.0));
        click_at(&ctx, &mut app, a, Modifiers::NONE);
        click_at(&ctx, &mut app, b, other);
        assert!(!level(&app), "{key:?}: another key does not constrain");
    }
}

/// Shift with a marquee turns over the selection of the objects it takes
/// (#401): one picked, a Shift marquee round all three leaves the other
/// two picked.
#[test]
fn a_shift_marquee_inverts_the_objects_it_takes() {
    let (ctx, mut app, first) = setup();
    let rect = |app: &mut App, x: f64| {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x, 10.0, x + 20.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect")
    };
    let b = rect(&mut app, 80.0);
    let c = rect(&mut app, 120.0);
    app.select(vec![first]);
    let from = app.view.to_screen(Point::new(0.0, 60.0));
    let to = app.view.to_screen(Point::new(150.0, 0.0));
    drag(&ctx, &mut app, from, to, Modifiers::SHIFT, Vec::new());
    let mut got = app.selection.clone();
    got.sort();
    let mut want = vec![b, c];
    want.sort();
    assert_eq!(got, want);
    // Without Shift a marquee picks all three.
    app.select(vec![first]);
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    assert_eq!(app.selection.len(), 3);
}

/// With the Shape tool, the align keys align the selected nodes (#364):
/// T to the top node, L to the leftmost, E to the middle of their box;
/// handles move with their node; one undo step each.
#[test]
fn align_keys_align_selected_nodes_in_the_shape_tool() {
    let (ctx, mut app, id) = setup();
    let mut path = tracedraw_core::BezPath::new();
    path.move_to((10.0, 10.0));
    path.curve_to((20.0, 10.0), (30.0, 20.0), (40.0, 30.0));
    path.line_to((70.0, 20.0));
    app.select(vec![id]);
    app.delete_selection();
    let c = app
        .new_shape(ShapeKind::Path {
            path,
            closed: false,
        })
        .expect("curve");
    app.select(vec![c]);
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let ns = |app: &App| match app.doc().find_shape(c).map(|s| s.kind.clone()) {
        Some(ShapeKind::Path { path, .. }) => tracedraw_core::nodes::nodes(&path),
        _ => panic!("curve"),
    };
    let all: Vec<_> = ns(&app).iter().map(|n| (c, n.index)).collect();
    assert_eq!(all.len(), 3);
    app.node_selection = all;
    let press = |app: &mut App, k: egui::Key| {
        frame(&ctx, app, vec![key(k, Modifiers::NONE)], Modifiers::NONE);
    };
    press(&mut app, egui::Key::T);
    let after = ns(&app);
    assert!(
        after.iter().all(|n| (n.pos.y - 30.0).abs() < 1e-9),
        "{after:?}"
    );
    // The first node's outgoing handle moved with it, 20 mm up.
    let first = &after[0];
    assert!((first.ctrl_out.expect("handle").y - 30.0).abs() < 1e-9);
    assert_eq!(app.tool, Tool::Shape);
    assert_eq!(app.selection, vec![c]);
    press(&mut app, egui::Key::L);
    assert!(ns(&app).iter().all(|n| (n.pos.x - 10.0).abs() < 1e-9));
    app.undo();
    let back = ns(&app);
    assert!((back[2].pos.x - 70.0).abs() < 1e-9, "{back:?}");
    press(&mut app, egui::Key::E);
    assert!(ns(&app).iter().all(|n| (n.pos.y - 30.0).abs() < 1e-9));
    // One node: the keys do what they did before (nothing to the nodes).
    app.node_selection.truncate(1);
    assert!(!app.align_nodes_to(crate::ops::Align::Bottom));
}

/// A Pen curve keeps its anchors in the history once it is closed and
/// deselected (#360): Ctrl+Z opens it first, then takes the anchors back
/// one by one, and redo builds it again.
#[test]
fn a_finished_pen_curve_undoes_anchor_by_anchor() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::Pen);
    let pts = [
        Point::new(100.0, 150.0),
        Point::new(150.0, 150.0),
        Point::new(150.0, 200.0),
        Point::new(100.0, 200.0),
    ];
    for p in pts {
        click_page(&ctx, &mut app, p);
        for _ in 0..40 {
            frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
        }
    }
    // A click on the first node closes it.
    click_page(&ctx, &mut app, pts[0]);
    assert!(app.curve.is_none());
    let id = *app.selection.first().expect("the new curve");
    let path_of = |app: &App| match app.doc().find_shape(id).map(|s| s.kind.clone()) {
        Some(ShapeKind::Path { path, closed }) => Some((path, closed)),
        _ => None,
    };
    let count = |p: &tracedraw_core::BezPath| tracedraw_core::nodes::nodes(p).len();
    let (closed_path, closed) = path_of(&app).expect("curve");
    assert!(closed);
    assert_eq!(count(&closed_path), 4);
    // Deselected, then undone step by step.
    app.select(Vec::new());
    app.undo();
    let (p, closed) = path_of(&app).expect("open again");
    assert!(!closed && count(&p) == 4, "{p:?}");
    app.undo();
    assert_eq!(path_of(&app).map(|(p, _)| count(&p)), Some(3));
    app.undo();
    assert_eq!(path_of(&app).map(|(p, _)| count(&p)), Some(2));
    app.undo();
    assert!(path_of(&app).is_none());
    // Redo brings it back the same way, closed at the end.
    for _ in 0..4 {
        app.redo();
    }
    assert_eq!(path_of(&app), Some((closed_path, true)));
    // The curve still has the default fill and outline of a closed curve.
    let s = app.doc().find_shape(id).cloned().expect("curve");
    let mut want = tracedraw_core::Shape::new(
        id,
        ShapeKind::Path {
            path: s.page_path(),
            closed: true,
        },
    );
    app.doc().defaults.apply_to(&mut want);
    assert_eq!((s.fill, s.stroke), (want.fill, want.stroke));
}

/// A scale handle follows the pointer from the frame the drag starts: no
/// jump of the few pixels the pointer goes before a press becomes a drag
/// (#383).
#[test]
fn a_scale_handle_follows_the_pointer_from_the_start() {
    let (ctx, mut app, _) = setup();
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let e = handle(&app, Handle::E);
    let w = app.selection_bounds().expect("bounds").width() as f32 * app.view.zoom;
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(e)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(e, true, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let mut seen = Vec::new();
    for i in 1..=20 {
        let p = e + egui::vec2(i as f32, 0.0);
        frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(p)],
            Modifiers::NONE,
        );
        if let Some(t) = app.preview_transform_of(&app.drag) {
            let want = 1.0 + i as f64 / f64::from(w);
            seen.push((i, t.as_coeffs()[0], want));
        }
    }
    assert!(!seen.is_empty());
    for (i, got, want) in seen {
        assert!((got - want).abs() < 0.01, "step {i}: {got} {want}");
    }
}

/// The Shape tool on a B-spline: a marquee round one control point
/// chooses it alone, Delete removes that point and keeps the curve, a
/// click chooses one and Shift+click adds another, and a drag of a chosen
/// point moves the chosen ones together (a delete took the whole curve).
#[test]
fn bspline_control_points_are_chosen_and_deleted_alone() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::BSpline);
    let pts = [
        Point::new(100.0, 150.0),
        Point::new(130.0, 190.0),
        Point::new(180.0, 180.0),
        Point::new(190.0, 140.0),
        Point::new(150.0, 120.0),
    ];
    for p in pts {
        let at = app.view.to_screen(p);
        click_at(&ctx, &mut app, at, Modifiers::NONE);
    }
    app.finish_curve();
    let id = app.selection[0];
    let spline = |app: &App| app.doc().find_shape(id).and_then(|s| s.spline.clone());
    app.select(vec![id]);
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // A marquee round the second point only.
    let a = app
        .view
        .to_screen(pts[1] + tracedraw_core::geometry::Vec2::new(-6.0, -6.0));
    let b = app
        .view
        .to_screen(pts[1] + tracedraw_core::geometry::Vec2::new(6.0, 6.0));
    drag(&ctx, &mut app, a, b, Modifiers::NONE, Vec::new());
    assert_eq!(app.node_selection, vec![(id, 1)]);
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Delete, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let left = spline(&app).expect("the curve stays");
    assert_eq!(left.len(), 4, "{left:?}");
    assert!(left.iter().all(|p| (*p - pts[1]).hypot() > 1.0));
    // A click chooses one point, Shift+click another; a drag moves both.
    let at = app.view.to_screen(pts[2]);
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert_eq!(app.node_selection, vec![(id, 1)]);
    let at = app.view.to_screen(pts[3]);
    click_at(&ctx, &mut app, at, Modifiers::SHIFT);
    assert_eq!(app.node_selection, vec![(id, 1), (id, 2)]);
    let from = app.view.to_screen(pts[2]);
    let to = app
        .view
        .to_screen(pts[2] + tracedraw_core::geometry::Vec2::new(10.0, 0.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    let moved = spline(&app).expect("spline");
    assert!((moved[1].x - (pts[2].x + 10.0)).abs() < 1.0, "{moved:?}");
    assert!((moved[2].x - (pts[3].x + 10.0)).abs() < 1.0, "{moved:?}");
    assert!((moved[0] - pts[0]).hypot() < 0.5, "{moved:?}");
}

/// Editing a symmetry as in the reference program: a click picks one of
/// its objects (not the group, not a copy) and the mode stays; a drag on a
/// mirror line moves the centre; a click on a line shows handles that turn
/// the lines; a marquee takes members; a new object joins.
#[test]
fn symmetry_editing_picks_members_and_drags_lines() {
    use tracedraw_core::live::Effect;
    let (ctx, mut app, a) = setup();
    let b = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(10.0, 60.0, 30.0, 80.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.select(vec![a, b]);
    app.create_symmetry();
    let g = app.symmetry_edit.expect("editing");
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // The rectangle a spans 10..60 x 10..40; the line is at x = 60.
    let (_, c0, ang0, _) = app.edited_symmetry().expect("symmetry");
    // A click on b picks b alone; the mode and its bar stay.
    let at = app.view.to_screen(Point::new(20.0, 70.0));
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert_eq!(app.selection, vec![b]);
    assert_eq!(app.symmetry_edit, Some(g));
    assert!(app.edited_symmetry().is_some());
    // A click on a copy (mirrored to the right of the line) picks nothing
    // of the symmetry.
    let at = app.view.to_screen(Point::new(100.0, 70.0));
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert!(!app.selection.contains(&g));
    // A drag on the line, away from the centre, moves the centre.
    let from = app.view.to_screen(Point::new(c0.x, c0.y + 40.0));
    let to = app.view.to_screen(Point::new(c0.x + 15.0, c0.y + 40.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    let (_, c1, ang1, _) = app.edited_symmetry().expect("symmetry");
    assert!((c1.x - (c0.x + 15.0)).abs() < 0.6, "{c0:?} -> {c1:?}");
    assert!((ang1 - ang0).abs() < 1e-9);
    // One undo takes the move back.
    app.undo();
    assert!((app.edited_symmetry().unwrap().1.x - c0.x).abs() < 1e-6);
    // A click on the line shows the turning handles; dragging one turns.
    let on_line = app.view.to_screen(Point::new(c0.x, c0.y + 30.0));
    click_at(&ctx, &mut app, on_line, Modifiers::NONE);
    assert!(app.symmetry_turning);
    let hs = app.symmetry_handles();
    assert_eq!(hs.len(), 2);
    let h = hs[0];
    let v = h - c0;
    let turned = c0 + tracedraw_core::geometry::Vec2::new(-v.y, v.x);
    let (hf, ht) = (app.view.to_screen(h), app.view.to_screen(turned));
    drag(&ctx, &mut app, hf, ht, Modifiers::NONE, Vec::new());
    let (_, _, ang2, _) = app.edited_symmetry().expect("symmetry");
    let diff = (ang2 - ang0).rem_euclid(360.0);
    assert!((diff - 90.0).abs() < 2.0, "{ang0} -> {ang2}");
    // A marquee round both originals takes the two members.
    app.set_tool(Tool::Pick);
    let m0 = app.view.to_screen(Point::new(5.0, 5.0));
    let m1 = app.view.to_screen(Point::new(63.0, 85.0));
    // Start the marquee off the lines and the objects.
    app.select(Vec::new());
    drag(&ctx, &mut app, m0, m1, Modifiers::NONE, Vec::new());
    let mut sel = app.selection.clone();
    sel.sort();
    assert_eq!(sel, vec![a, b]);
    // Still one symmetry group holding both.
    assert!(app.doc().find_shape(g).is_some_and(|s| s
        .effects
        .iter()
        .any(|e| matches!(e, Effect::Symmetry { .. }))));
}

/// Frame times on a page of many objects (#345): run with
/// `cargo test --release -- --ignored frame_times_on_a_heavy_page --nocapture`.
#[test]
#[ignore]
fn frame_times_on_a_heavy_page() {
    let (ctx, mut app, _) = setup();
    let mut s = 7u64;
    let mut r = || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((s >> 33) as f64) / (1u64 << 31) as f64
    };
    let n: usize = std::env::var("FRAME_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);
    let only: Option<String> = std::env::var("FRAME_ONLY").ok();
    for _ in 0..n {
        let (x, y) = (r() * 200.0, r() * 280.0);
        let mut p = tracedraw_core::geometry::BezPath::new();
        p.move_to((x, y));
        for _ in 0..6 {
            p.curve_to(
                (x + r() * 20.0 - 10.0, y + r() * 20.0 - 10.0),
                (x + r() * 20.0 - 10.0, y + r() * 20.0 - 10.0),
                (x + r() * 20.0 - 10.0, y + r() * 20.0 - 10.0),
            );
        }
        p.close_path();
        app.new_shape(ShapeKind::Path {
            path: p,
            closed: true,
        });
    }
    app.select(Vec::new());
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let time = |label: &str, app: &mut App, f: &mut dyn FnMut(&mut App)| {
        if only.as_deref().is_some_and(|o| !label.starts_with(o)) {
            return;
        }
        let t = std::time::Instant::now();
        for _ in 0..10 {
            f(app);
        }
        eprintln!("{label}: {:?} per frame", t.elapsed() / 10);
    };
    let mut x = 300.0f32;
    time("hover", &mut app, &mut |app| {
        x += 3.0;
        frame(
            &ctx,
            app,
            vec![Event::PointerMoved(egui::pos2(x, 300.0))],
            Modifiers::NONE,
        );
    });
    let at = app.view.to_screen(Point::new(100.0, 100.0));
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(at)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(at, true, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let mut d = 0.0f32;
    time("drag (marquee or move)", &mut app, &mut |app| {
        d += 4.0;
        frame(
            &ctx,
            app,
            vec![Event::PointerMoved(at + egui::vec2(d, d))],
            Modifiers::NONE,
        );
    });
    frame(
        &ctx,
        &mut app,
        vec![button(
            at + egui::vec2(d, d),
            false,
            PointerButton::Primary,
            Modifiers::NONE,
        )],
        Modifiers::NONE,
    );
    app.select_all();
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    time("all selected, hover", &mut app, &mut |app| {
        x += 3.0;
        frame(
            &ctx,
            app,
            vec![Event::PointerMoved(egui::pos2(x, 320.0))],
            Modifiers::NONE,
        );
    });
}

/// A symmetry made from one object, edited: a click on the object shows
/// its own box, not the box of its copies, and a rectangle drawn with the
/// tool joins the symmetry (the two become its group).
#[test]
fn a_lone_symmetry_object_is_picked_alone_and_takes_new_drawings() {
    let (ctx, mut app, a) = setup();
    app.create_symmetry();
    assert_eq!(app.symmetry_edit, Some(a));
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    app.select(Vec::new());
    let at = app.view.to_screen(Point::new(30.0, 25.0));
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert_eq!(app.selection, vec![a]);
    let b = app.selection_bounds().expect("bounds");
    assert!(
        (b.x0 - 10.0).abs() < 1.0 && (b.x1 - 60.0).abs() < 1.0,
        "the object's own box: {b:?}"
    );
    // Where a mirror line runs over the object, a click picks the
    // object rather than showing the line's turning handles.
    let (_, c, ..) = app.edited_symmetry().expect("symmetry");
    app.select(Vec::new());
    let at = app.view.to_screen(Point::new(c.x - 1.0, 12.0));
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert_eq!(app.selection, vec![a], "centre at {c:?}");
    assert!(!app.symmetry_turning);
    // Drawn with the rectangle tool, below the object.
    app.set_tool(Tool::Rectangle);
    let from = app.view.to_screen(Point::new(10.0, 60.0));
    let to = app.view.to_screen(Point::new(30.0, 80.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    let g = app.symmetry_edit.expect("still editing");
    assert_ne!(g, a, "the object and the drawing became a group");
    let kids = match app.doc().find_shape(g).map(|s| &s.kind) {
        Some(ShapeKind::Group { children }) => children.len(),
        _ => 0,
    };
    assert_eq!(kids, 2);
}

fn double_click_at(ctx: &egui::Context, app: &mut App, at: Pos2) {
    let m = Modifiers::NONE;
    // Past the triple-click time since the last click.
    for _ in 0..40 {
        frame(ctx, app, Vec::new(), m);
    }
    frame(ctx, app, vec![Event::PointerMoved(at)], m);
    for _ in 0..2 {
        frame(
            ctx,
            app,
            vec![button(at, true, PointerButton::Primary, m)],
            m,
        );
        frame(
            ctx,
            app,
            vec![button(at, false, PointerButton::Primary, m)],
            m,
        );
    }
    for _ in 0..20 {
        frame(ctx, app, Vec::new(), m);
    }
}

/// A finished symmetry is one object, as in the reference program: a
/// click picks the whole of it and a drag moves it all; a double click
/// goes back into editing it, where a click picks one object inside.
#[test]
fn a_finished_symmetry_is_picked_whole_and_a_double_click_edits_it() {
    let (ctx, mut app, a) = setup();
    let b = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(10.0, 60.0, 30.0, 80.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.select(vec![a, b]);
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
        200, 30, 30,
    )));
    app.create_symmetry();
    let g = app.symmetry_edit.expect("editing");
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // The bar's Finish, clicked.
    let finish = app.canvas_rect.min + egui::vec2(14.0 + 10.0 + 30.0, 14.0 + 6.0 + 9.0);
    click_at(&ctx, &mut app, finish, Modifiers::NONE);
    assert_eq!(app.symmetry_edit, None, "Finish ends the editing");
    app.select(Vec::new());
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let on_a = app.view.to_screen(Point::new(30.0, 25.0));
    click_at(&ctx, &mut app, on_a, Modifiers::NONE);
    assert_eq!(app.symmetry_edit, None, "a click does not edit");
    assert_eq!(app.selection, vec![g], "the whole symmetry");
    let before = app.doc().shape_on_page(g).expect("g").bounds();
    let to = app.view.to_screen(Point::new(50.0, 25.0));
    drag(&ctx, &mut app, on_a, to, Modifiers::NONE, Vec::new());
    let after = app.doc().shape_on_page(g).expect("g").bounds();
    assert!(
        (after.x0 - before.x0 - 20.0).abs() < 1.0,
        "{before:?} -> {after:?}"
    );
    // Off the selection's centre mark (a double click there resets it).
    let on_a = app.view.to_screen(Point::new(36.0, 14.0));
    double_click_at(&ctx, &mut app, on_a);
    assert_eq!(app.symmetry_edit, Some(g), "a double click edits it");
    click_at(&ctx, &mut app, on_a, Modifiers::NONE);
    assert_eq!(app.selection, vec![a], "one object inside");
}

/// Shape tool: Ctrl or Shift while dragging a handle keeps it to the
/// constrain angle about its node, and a smooth node's other handle keeps
/// following it (Ctrl made the node a cusp instead, Shift did nothing;
/// #510).
#[test]
fn ctrl_or_shift_keeps_a_handle_to_the_constrain_angle() {
    use tracedraw_core::geometry::BezPath;
    use tracedraw_core::nodes;
    for mods in [Modifiers::CTRL, Modifiers::SHIFT] {
        let (ctx, mut app, _) = setup();
        let mut path = BezPath::new();
        path.move_to((10.0, 50.0));
        path.curve_to((20.0, 50.0), (30.0, 60.0), (40.0, 60.0));
        path.curve_to((50.0, 60.0), (60.0, 50.0), (70.0, 50.0));
        let id = app
            .new_shape(ShapeKind::Path {
                path,
                closed: false,
            })
            .expect("curve");
        app.select(vec![id]);
        app.set_tool(Tool::Shape);
        app.node_selection = vec![(id, 1)];
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
        let from = app.view.to_screen(Point::new(50.0, 60.0));
        let to = app.view.to_screen(Point::new(58.0, 63.0));
        drag(&ctx, &mut app, from, to, mods, Vec::new());
        let ShapeKind::Path { path, .. } = &app.doc().find_shape(id).expect("curve").kind else {
            panic!("a curve");
        };
        let n = nodes::nodes(path)
            .into_iter()
            .find(|n| n.index == 1)
            .expect("node");
        let out = n.ctrl_out.expect("out") - n.pos;
        let back = n.ctrl_in.expect("in") - n.pos;
        let a = out.y.atan2(out.x).to_degrees();
        assert!((a - 15.0).abs() < 0.5, "{mods:?}: {a}");
        let b = back.y.atan2(back.x).to_degrees();
        assert!(
            ((b - a).abs() - 180.0).abs() < 0.5,
            "{mods:?}: still smooth, {b}"
        );
    }
}

/// Pen: Ctrl while dragging on the first node to close the curve keeps
/// the closing handle to the constrain angle, as on every other node
/// (#473).
#[test]
fn ctrl_keeps_the_closing_handle_to_the_constrain_angle() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::Pen);
    let a = Point::new(100.0, 150.0);
    pull(&ctx, &mut app, a, Point::new(110.0, 170.0));
    pull(
        &ctx,
        &mut app,
        Point::new(150.0, 190.0),
        Point::new(170.0, 190.0),
    );
    click_page(&ctx, &mut app, Point::new(160.0, 130.0));
    let (from, to) = (
        app.view.to_screen(a),
        app.view.to_screen(Point::new(88.0, 146.0)),
    );
    drag(
        &ctx,
        &mut app,
        from,
        to,
        Modifiers::CTRL,
        vec![key(egui::Key::C, Modifiers::CTRL)],
    );
    assert!(app.curve.is_none());
    let segs: Vec<_> = last_path(&app).segments().collect();
    let closing = segs
        .iter()
        .rev()
        .find_map(|s| match s {
            tracedraw_core::geometry::PathSeg::Cubic(c) if near(c.p3, a) => Some(*c),
            _ => None,
        })
        .expect("a curved closing segment");
    let d = closing.p2 - a;
    let angle = d.y.atan2(d.x).to_degrees().rem_euclid(360.0);
    // Free, the handle would point at 198.4°; kept to 15° steps, 195°.
    assert!((angle - 195.0).abs() < 0.5, "{angle} {closing:?}");
}

/// While a text is edited, Space types a space and Ctrl+Space leaves the
/// text for the Pick tool, the text kept as typed (autocorrect starting
/// the sentence with a capital) (#478).
#[test]
fn ctrl_space_leaves_text_editing_for_the_pick_tool() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.start_text(Point::new(50.0, 50.0), None);
    app.text_insert("ab");
    frame(
        &ctx,
        &mut app,
        vec![
            key(egui::Key::Space, Modifiers::NONE),
            Event::Text(" ".into()),
        ],
        Modifiers::NONE,
    );
    assert!(app.text_edit.is_some());
    assert_eq!(
        app.edit_spans()
            .map(|s| tracedraw_core::document::spans_text(&s)),
        Some("Ab ".into())
    );
    frame(
        &ctx,
        &mut app,
        vec![
            key(egui::Key::Space, Modifiers::CTRL),
            Event::Text(" ".into()),
        ],
        Modifiers::CTRL,
    );
    assert!(app.text_edit.is_none(), "editing ended");
    assert_eq!(app.tool, Tool::Pick);
    let id = app.selection[0];
    match &app.doc().find_shape(id).expect("text").kind {
        ShapeKind::Text { spans, .. } => {
            assert_eq!(tracedraw_core::document::spans_text(spans), "Ab ")
        }
        other => panic!("{other:?}"),
    }
}

/// Alt held while 0193 is typed on the keypad gives Á when Alt is let
/// go, the keystroke the Glyphs docker shows; with Num Lock off the
/// keypad sends Insert, End, PageDown and PageDown, read by their
/// physical keys (#469).
#[test]
fn alt_codes_typed_on_the_keypad_give_their_character() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.start_text(Point::new(50.0, 50.0), None);
    let pad = |logical: egui::Key, physical: egui::Key| Event::Key {
        key: logical,
        physical_key: Some(physical),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::ALT,
    };
    frame(
        &ctx,
        &mut app,
        vec![
            pad(egui::Key::Insert, egui::Key::Num0),
            pad(egui::Key::End, egui::Key::Num1),
            pad(egui::Key::PageDown, egui::Key::Num9),
            pad(egui::Key::PageDown, egui::Key::Num3),
        ],
        Modifiers::ALT,
    );
    assert_eq!(
        app.edit_spans()
            .map(|s| tracedraw_core::document::spans_text(&s)),
        Some(String::new()),
        "nothing typed while Alt is held"
    );
    frame(&ctx, &mut app, vec![], Modifiers::NONE);
    assert_eq!(
        app.edit_spans()
            .map(|s| tracedraw_core::document::spans_text(&s)),
        Some("\u{c1}".into())
    );
}

/// With Num Lock on the windowing layer sends each keypad digit typed
/// under Alt as a key and as the digit's text: only the character the
/// code names is typed, not the digits too. When the system gives the
/// composed character itself as text (Alt still held), it is typed once
/// (#447, #469).
#[test]
fn alt_codes_type_their_character_once_and_not_the_digits() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.start_text(Point::new(50.0, 50.0), None);
    let text = |app: &App| {
        app.edit_spans()
            .map(|s| tracedraw_core::document::spans_text(&s))
            .unwrap_or_default()
    };
    let mut events = Vec::new();
    for (k, d) in [
        (egui::Key::Num0, "0"),
        (egui::Key::Num1, "1"),
        (egui::Key::Num9, "9"),
        (egui::Key::Num3, "3"),
    ] {
        events.push(Event::Key {
            key: k,
            physical_key: Some(k),
            pressed: true,
            repeat: false,
            modifiers: Modifiers::ALT,
        });
        events.push(Event::Text(d.into()));
    }
    frame(&ctx, &mut app, events, Modifiers::ALT);
    assert_eq!(text(&app), "", "no digits typed");
    // Alt let go, the system's own character arriving with it.
    frame(
        &ctx,
        &mut app,
        vec![Event::Text("\u{c1}".into())],
        Modifiers::NONE,
    );
    assert_eq!(text(&app), "\u{c1}");
    // The character the system composes, typed as text while Alt is
    // still down (no digit keys seen): typed.
    frame(
        &ctx,
        &mut app,
        vec![Event::Text("\u{a9}".into())],
        Modifiers::ALT,
    );
    frame(&ctx, &mut app, vec![], Modifiers::NONE);
    assert_eq!(text(&app), "\u{c1}\u{a9}");
}

/// Artistic text converted to a paragraph frame (Text > Convert) grows
/// down with its bottom handle: the flow tab sat under that handle and
/// took the press, so the next click linked the text to another object
/// (#503).
#[test]
fn the_bottom_handle_of_a_converted_frame_drags_it_taller() {
    let (ctx, mut app, rect) = setup();
    app.select(Vec::new());
    app.start_text(Point::new(80.0, 80.0), None);
    app.text_insert("Hello world");
    app.finish_text();
    let id = app.selection[0];
    app.toggle_text_kind();
    assert_eq!(app.selection, vec![id]);
    app.set_tool(Tool::Pick);
    for _ in 0..2 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let size = |app: &App| match &app.doc().find_shape(id).expect("text").kind {
        ShapeKind::Text { frame, .. } => frame.expect("a paragraph frame"),
        other => panic!("{other:?}"),
    };
    let before = size(&app);
    // The lower half of the handle, where the tab was.
    let from = handle(&app, Handle::S) + egui::vec2(0.0, 3.0);
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(0.0, 80.0),
        Modifiers::NONE,
        vec![],
    );
    assert!(app.pending_text_link.is_none(), "no text link started");
    assert!(size(&app).height > before.height + 1.0, "{before:?}");
    // A click on the rectangle afterwards selects it; nothing links.
    let c = app.view.to_screen(bounds(&app, rect).center());
    click_at(&ctx, &mut app, c, Modifiers::NONE);
    assert!(app.next_frame(id).is_none());
}

/// Editing a clip frame's contents, a group inside drags, ungroups, and
/// its members (and an object drawn meanwhile) go back into the frame on
/// Finish; the ungrouped members were left out and the frame lost its
/// contents (#537).
#[test]
fn a_group_inside_a_clip_frame_being_edited_drags_and_ungroups() {
    let (ctx, mut app, frame_id) = setup();
    let mk = |app: &mut App, r: Rect| {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: r,
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
            0, 0, 200,
        )));
        id
    };
    let a = mk(&mut app, Rect::new(15.0, 15.0, 30.0, 30.0));
    let b = mk(&mut app, Rect::new(32.0, 15.0, 45.0, 30.0));
    let outside = mk(&mut app, Rect::new(150.0, 150.0, 160.0, 160.0));
    app.select(vec![a, b]);
    app.group_selection();
    let g = app.selection[0];
    app.place_inside(vec![g], frame_id);
    app.select(vec![frame_id]);
    app.edit_clip_frame();
    assert_eq!(app.selection, vec![g]);
    for _ in 0..2 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let x0 = bounds(&app, g).x0;
    let from = app.view.to_screen(Point::new(22.0, 22.0));
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(60.0, 0.0),
        Modifiers::NONE,
        vec![],
    );
    assert!(bounds(&app, g).x0 > x0 + 1.0, "the group moved");
    app.ungroup_selection();
    assert_eq!(app.selection, vec![a, b]);
    let (focus, _) = app.render_focus().expect("editing");
    assert!(
        focus.contains(&a) && focus.contains(&b),
        "the members stay in full"
    );
    assert!(!focus.contains(&outside));
    let c = mk(&mut app, Rect::new(20.0, 32.0, 25.0, 36.0));
    app.finish_clip_frame_edit();
    match &app.doc().find_shape(frame_id).expect("frame").kind {
        ShapeKind::ClipFrame { contents, .. } => {
            let ids: Vec<ShapeId> = contents.iter().map(|s| s.id).collect();
            assert_eq!(ids, vec![a, b, c]);
        }
        other => panic!("not a clip frame: {other:?}"),
    }
    assert!(app.doc().find_shape(outside).is_some());
}

/// #851: while a clip frame's contents are edited, the objects outside
/// it (and the frame) cannot be picked by a click, a marquee, Select All
/// or Tab; undo inside the edit stops at the edit's start, where it
/// leaves the edit with the frame as it was, and never undoes what was
/// done outside before; redo enters the edit again, and undoing Finish
/// goes back into it.
#[test]
fn clip_frame_edit_keeps_to_its_contents_and_its_own_undo() {
    let (ctx, mut app, frame_id) = setup();
    let mk = |app: &mut App, r: Rect| {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: r,
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
            0, 0, 200,
        )));
        id
    };
    let a = mk(&mut app, Rect::new(15.0, 15.0, 30.0, 30.0));
    let other = mk(&mut app, Rect::new(100.0, 100.0, 120.0, 120.0));
    app.place_inside(vec![a], frame_id);
    // Something done outside before the edit.
    app.select(vec![other]);
    app.run(tracedraw_core::Command::TransformShapes {
        shapes: vec![other],
        transform: Affine::translate((10.0, 0.0)),
    });
    let other_x = bounds(&app, other).x0;
    app.select(vec![frame_id]);
    app.edit_clip_frame();
    assert_eq!(app.selection, vec![a]);
    for _ in 0..2 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    // A click on the object outside takes nothing, a click on the
    // contents takes them.
    let on_other = app.view.to_screen(Point::new(115.0, 110.0));
    click_at(&ctx, &mut app, on_other, Modifiers::NONE);
    assert!(app.selection.is_empty(), "{:?}", app.selection);
    assert!(app.hits_at(Point::new(115.0, 110.0), true).is_empty());
    let on_a = app.view.to_screen(Point::new(18.0, 18.0));
    click_at(&ctx, &mut app, on_a, Modifiers::NONE);
    assert_eq!(app.selection, vec![a]);
    // A marquee over everything, Select All and Tab: the contents only.
    app.select(Vec::new());
    let from = app.view.to_screen(Point::new(0.0, 0.0));
    let to = app.view.to_screen(Point::new(140.0, 140.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    assert_eq!(app.selection, vec![a]);
    app.select_all();
    assert_eq!(app.selection, vec![a]);
    app.select(Vec::new());
    app.cycle_selection(true);
    assert_eq!(app.selection, vec![a]);
    // A move inside the edit, then undo: the move goes, the edit stays.
    let a_x = bounds(&app, a).x0;
    app.run(tracedraw_core::Command::TransformShapes {
        shapes: vec![a],
        transform: Affine::translate((5.0, 0.0)),
    });
    app.undo();
    assert!(app.clip_frame_edit.is_some());
    assert!(close(bounds(&app, a).x0, a_x));
    // Its own history done: the next undo leaves the edit, the frame
    // holding its contents again, and the outside move stays.
    app.undo();
    assert!(app.clip_frame_edit.is_none());
    assert_eq!(app.selection, vec![frame_id]);
    match &app.doc().find_shape(frame_id).expect("frame").kind {
        ShapeKind::ClipFrame { contents, .. } => {
            assert_eq!(contents.iter().map(|c| c.id).collect::<Vec<_>>(), vec![a]);
        }
        other => panic!("not a clip frame: {other:?}"),
    }
    assert!(close(bounds(&app, other).x0, other_x));
    // Redo enters the edit again, with the contents out and selected.
    app.redo();
    assert!(app.clip_frame_edit.is_some());
    assert_eq!(app.selection, vec![a]);
    app.redo();
    assert!(close(bounds(&app, a).x0, a_x + 5.0));
    // Undoing Finish goes back into the edit; redoing it leaves.
    app.finish_clip_frame_edit();
    assert!(app.clip_frame_edit.is_none());
    app.undo();
    assert!(app.clip_frame_edit.is_some());
    assert_eq!(app.selection, vec![a]);
    app.redo();
    assert!(app.clip_frame_edit.is_none());
    assert_eq!(app.selection, vec![frame_id]);
}

/// Finish on the symmetry bar ends the editing: the bar and the mirror
/// lines go, the symmetry stays selected whole, a click takes it whole
/// and a drag moves it whole.
#[test]
fn finish_on_the_symmetry_bar_leaves_one_object() {
    let (ctx, mut app, id) = setup();
    app.select(vec![id]);
    app.create_symmetry();
    let sym = app.selection[0];
    assert_eq!(
        app.symmetry_edit,
        Some(sym),
        "a new symmetry opens for editing"
    );
    let texts = |app: &mut App| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
        let mut all: Vec<(String, Pos2)> = Vec::new();
        fn walk(s: &egui::Shape, all: &mut Vec<(String, Pos2)>) {
            match s {
                egui::Shape::Text(t) => all.push((
                    t.galley.text().to_string(),
                    t.pos + t.galley.rect.center().to_vec2(),
                )),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, all)),
                _ => {}
            }
        }
        out.shapes.iter().for_each(|c| walk(&c.shape, &mut all));
        all
    };
    texts(&mut app);
    let finish = texts(&mut app)
        .into_iter()
        .find(|(t, _)| t.trim() == "Finish")
        .map(|(_, p)| p)
        .expect("the bar's Finish");
    for _ in 0..40 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    click_at(&ctx, &mut app, finish, Modifiers::NONE);
    assert_eq!(app.symmetry_edit, None, "editing ended");
    assert!(app.edited_symmetry().is_none());
    assert_eq!(app.selection, vec![sym]);
    assert!(
        !texts(&mut app).iter().any(|(t, _)| t.trim() == "Finish"),
        "the bar is gone"
    );
    // One object: a click takes it whole, a drag moves it whole.
    for _ in 0..40 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let c = app.view.to_screen(bounds(&app, sym).center());
    click_at(&ctx, &mut app, c, Modifiers::NONE);
    assert_eq!(app.selection, vec![sym]);
    assert_eq!(app.symmetry_edit, None);
    let x0 = bounds(&app, sym).x0;
    drag(
        &ctx,
        &mut app,
        c,
        c + egui::vec2(80.0, 0.0),
        Modifiers::NONE,
        vec![],
    );
    assert!(bounds(&app, sym).x0 > x0 + 5.0);
    assert_eq!(app.symmetry_edit, None);
}

/// Frame times of the Pen with its preview following the pointer, with
/// the preview off and on (#560): run with
/// `cargo test --release -- --ignored frame_times_of_the_pen_preview --nocapture`.
/// The preview may add little to the frame; the white line drawn as a
/// path shape cost about 0.25 ms of 0.7 and the preview lagged.
#[test]
#[ignore]
fn frame_times_of_the_pen_preview() {
    for preview in [false, true] {
        let (ctx, mut app, _) = setup();
        app.select(Vec::new());
        app.settings.curves.pen_preview = preview;
        app.set_tool(Tool::Pen);
        let a = Pos2::new(500.0, 400.0);
        click_at(&ctx, &mut app, a, Modifiers::NONE);
        let n = 300;
        let (mut ui_t, mut tess_t, mut verts) = (0.0, 0.0, 0usize);
        for i in 0..n {
            let p = a + egui::vec2(
                100.0 + (i as f32 * 3.0) % 400.0,
                50.0 + (i as f32 * 7.0) % 200.0,
            );
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(1600.0, 1000.0),
                )),
                events: vec![Event::PointerMoved(p)],
                time: Some(i as f64 * 0.016),
                ..Default::default()
            };
            let t = std::time::Instant::now();
            let mut out = ctx.run_ui(input, |ui| crate::ui::root(&mut app, ui));
            ui_t += t.elapsed().as_secs_f64();
            let t = std::time::Instant::now();
            let clipped = ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
            tess_t += t.elapsed().as_secs_f64();
            for c in &clipped {
                if let egui::epaint::Primitive::Mesh(m) = &c.primitive {
                    verts += m.vertices.len();
                }
            }
            out.textures_delta.clear();
        }
        eprintln!(
            "pen preview {preview}: ui {:.3} ms, tessellation {:.3} ms, {} vertices a frame",
            ui_t * 1000.0 / n as f64,
            tess_t * 1000.0 / n as f64,
            verts / n
        );
    }
}

/// Trim with one object selected waits for a click on the target, found
/// under the selected object too; the selection cuts it, in one undo
/// step. Esc or a click on nothing cancels (#562).
#[test]
fn trim_with_one_object_waits_for_the_target() {
    use crate::ops::Shaping;
    let (ctx, mut app, cutter) = setup();
    let target = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(40.0, 20.0, 90.0, 70.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.select(vec![target]);
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
        30, 30, 200,
    )));
    let settle = |ctx: &egui::Context, app: &mut App| {
        for _ in 0..20 {
            frame(ctx, app, Vec::new(), Modifiers::NONE);
        }
    };
    // Esc cancels.
    app.select(vec![cutter]);
    app.shaping(Shaping::Trim);
    assert!(app.pending_shaping.is_some(), "waits for the target");
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Escape, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert!(app.pending_shaping.is_none());
    settle(&ctx, &mut app);
    // A click on nothing cancels and changes nothing.
    app.select(vec![cutter]);
    app.shaping(Shaping::Trim);
    let rev = app.engine.revision();
    let at = app.view.to_screen(Point::new(150.0, 150.0));
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert!(app.pending_shaping.is_none());
    assert_eq!(app.engine.revision(), rev);
    // The target clicked, where it shows beside the cutter.
    app.select(vec![cutter]);
    app.shaping(Shaping::Trim);
    let before = shape_count(&app);
    let at = app.view.to_screen(Point::new(80.0, 60.0));
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert!(app.pending_shaping.is_none());
    assert!(app.doc().find_shape(cutter).is_some(), "the cutter stays");
    assert!(app.doc().find_shape(target).is_none(), "the target is cut");
    assert_eq!(shape_count(&app), before);
    let cut = app.selection[0];
    let b = bounds(&app, cut);
    assert!(close(b.x1, 90.0) && close(b.y1, 70.0), "{b:?}");
    let s = app.doc().find_shape(cut).expect("cut").clone();
    assert_eq!(
        s.fill,
        tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(30, 30, 200)),
        "the target's fill"
    );
    // The part under the cutter is gone.
    assert!(!crate::tools2::point_in_path(
        &s.page_path(),
        Point::new(50.0, 30.0)
    ));
    // One step undoes it.
    app.undo();
    assert!(app.doc().find_shape(target).is_some());
    assert!(app.doc().find_shape(cut).is_none());
    settle(&ctx, &mut app);
    // The target under the selected object: the top one cuts the one
    // beneath, clicked where they overlap.
    app.select(vec![target]);
    app.shaping(Shaping::Trim);
    let at = app.view.to_screen(Point::new(50.0, 30.0));
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert!(
        app.doc().find_shape(cutter).is_none(),
        "the one beneath is cut"
    );
    assert!(app.doc().find_shape(target).is_some());
}

/// With the Customization options open, a toolbar button dragged off
/// the bars goes away and one dragged along its bar moves; with them
/// closed a drag does nothing to the bar. The Zoom toolbar shows
/// pictures, not words (#563).
#[test]
fn toolbar_buttons_move_and_leave_in_customize_mode() {
    use crate::ui::bar_items::{items, ItemBar};
    let (ctx, mut app, _) = setup();
    app.show_zoom_toolbar = true;
    let settle = |ctx: &egui::Context, app: &mut App| {
        for _ in 0..20 {
            frame(ctx, app, Vec::new(), Modifiers::NONE);
        }
    };
    settle(&ctx, &mut app);
    let slot = |app: &App, bar: ItemBar| {
        app.shown_bar_slots
            .iter()
            .find(|s| s.bar == bar)
            .cloned()
            .expect("bar drawn")
    };
    // The Zoom bar: its level box, then square picture buttons.
    let zoom = slot(&app, ItemBar::Zoom);
    assert_eq!(zoom.items.len(), ItemBar::Zoom.defaults().len());
    let side = crate::theme::Tokens::bar_button();
    for (id, r) in ItemBar::Zoom.defaults().iter().zip(&zoom.items) {
        if id.starts_with("zoom_") && *id != "zoom_levels" {
            assert!((r.width() - side).abs() < 0.5, "{id} {r:?}");
        }
    }
    // Closed options: a drag of Print leaves the bar as it is.
    let print = ItemBar::Standard
        .defaults()
        .iter()
        .position(|i| *i == "print")
        .expect("print");
    let at = slot(&app, ItemBar::Standard).items[print].center();
    drag(
        &ctx,
        &mut app,
        at,
        Pos2::new(60.0, 600.0),
        Modifiers::NONE,
        Vec::new(),
    );
    assert!(items(&app, ItemBar::Standard).contains(&"print".to_string()));
    settle(&ctx, &mut app);
    // Open: off the bars it goes.
    app.options_page = crate::ui::options::OptionsPage::CommandBars;
    app.dialog = crate::ui::dialogs::Dialog::Options;
    settle(&ctx, &mut app);
    let at = slot(&app, ItemBar::Standard).items[print].center();
    drag(
        &ctx,
        &mut app,
        at,
        Pos2::new(60.0, 600.0),
        Modifiers::NONE,
        Vec::new(),
    );
    settle(&ctx, &mut app);
    let now = items(&app, ItemBar::Standard);
    assert!(!now.contains(&"print".to_string()), "{now:?}");
    assert_eq!(slot(&app, ItemBar::Standard).items.len(), now.len());
    // New moved after Save (between the second and third buttons).
    let s = slot(&app, ItemBar::Standard);
    let gap = Pos2::new(
        (s.items[2].right() + s.items[3].left()) / 2.0 + 1.0,
        s.items[2].center().y,
    );
    drag(
        &ctx,
        &mut app,
        s.items[0].center(),
        gap,
        Modifiers::NONE,
        Vec::new(),
    );
    let now = items(&app, ItemBar::Standard);
    assert_eq!(&now[..3], ["open", "save", "new"], "{now:?}");
    // A command dragged from the Commands page onto the Zoom bar joins it.
    app.options_page = crate::ui::options::OptionsPage::Commands;
    app.shortcut_editor.filter = "convert to curves".into();
    settle(&ctx, &mut app);
    let texts = |app: &mut App| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
        let mut all: Vec<(String, Pos2)> = Vec::new();
        fn walk(s: &egui::Shape, all: &mut Vec<(String, Pos2)>) {
            match s {
                egui::Shape::Text(t) => all.push((
                    t.galley.text().to_string(),
                    t.pos + t.galley.rect.center().to_vec2(),
                )),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, all)),
                _ => {}
            }
        }
        out.shapes.iter().for_each(|c| walk(&c.shape, &mut all));
        all
    };
    let row = texts(&mut app)
        .into_iter()
        .find(|(t, _)| t.trim() == "Convert to Curves")
        .map(|(_, p)| p)
        .expect("the command's row");
    settle(&ctx, &mut app);
    let z = slot(&app, ItemBar::Zoom);
    let end = Pos2::new(
        z.items.last().expect("buttons").right() - 2.0,
        z.items[1].center().y,
    );
    drag(&ctx, &mut app, row, end, Modifiers::NONE, Vec::new());
    let zoom_items = items(&app, ItemBar::Zoom);
    assert_eq!(
        zoom_items.last().map(String::as_str),
        Some("menu.object.convert_to_curves"),
        "{zoom_items:?}"
    );
    // Reset brings them back.
    crate::ui::bars::reset(&mut app);
    assert_eq!(items(&app, ItemBar::Standard)[0], "new");
    assert!(items(&app, ItemBar::Standard).contains(&"print".to_string()));
}

/// A symmetry of two objects (one drawn while editing): after Finish the
/// symmetry is selected, not the object picked inside it, and a drag
/// moves the whole symmetry.
#[test]
fn finish_selects_the_whole_symmetry_of_several_objects() {
    let (ctx, mut app, id) = setup();
    app.select(vec![id]);
    app.create_symmetry();
    // Another object drawn while editing joins the symmetry.
    app.set_tool(Tool::Rectangle);
    let a = app.view.to_screen(Point::new(20.0, 60.0));
    let b = app.view.to_screen(Point::new(40.0, 80.0));
    drag(&ctx, &mut app, a, b, Modifiers::NONE, vec![]);
    app.set_tool(Tool::Pick);
    for _ in 0..20 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let sym = app.symmetry_edit.expect("editing");
    assert!(matches!(
        app.doc().find_shape(sym).map(|s| &s.kind),
        Some(ShapeKind::Group { .. })
    ));
    assert_ne!(app.selection, vec![sym], "the new object is picked inside");
    app.finish_symmetry_edit();
    assert_eq!(app.symmetry_edit, None);
    assert_eq!(app.selection, vec![sym], "the whole symmetry");
    for _ in 0..20 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let x0 = bounds(&app, sym).x0;
    let inner = bounds(&app, id).x0;
    let c = app.view.to_screen(bounds(&app, id).center());
    drag(
        &ctx,
        &mut app,
        c,
        c + egui::vec2(80.0, 0.0),
        Modifiers::NONE,
        vec![],
    );
    assert_eq!(app.selection, vec![sym]);
    assert!(bounds(&app, sym).x0 > x0 + 5.0, "the symmetry moved");
    assert!(
        (page_bounds(&app, id).x0 - inner) > 5.0,
        "its objects with it"
    );
}

/// A size typed on the property bar with no text selected asks before it
/// becomes the size of new text, as the reference program does; OK makes
/// it the default, Cancel leaves the default as it was.
#[test]
fn a_size_picked_with_nothing_selected_asks_before_changing_the_default() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_tool(Tool::Text);
    let before = app.text_default_size;
    let texts = |app: &mut App| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
        let mut all: Vec<(String, Pos2)> = Vec::new();
        fn walk(s: &egui::Shape, all: &mut Vec<(String, Pos2)>) {
            match s {
                egui::Shape::Text(t) => all.push((
                    t.galley.text().to_string(),
                    t.pos + t.galley.rect.center().to_vec2(),
                )),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, all)),
                _ => {}
            }
        }
        out.shapes.iter().for_each(|c| walk(&c.shape, &mut all));
        all
    };
    for _ in 0..5 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let shown = format!("{} pt", before);
    let field = texts(&mut app)
        .into_iter()
        .find(|(t, _)| t.trim() == shown)
        .map(|(_, p)| p)
        .expect("the size field");
    click_at(&ctx, &mut app, field, Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::A, Modifiers::COMMAND)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![Event::Text("36".into())],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Enter, Modifiers::NONE)],
        Modifiers::NONE,
    );
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    match &app.dialog {
        crate::ui::dialogs::Dialog::TextDefaults { size, font, .. } => {
            assert_eq!(*size, Some(36.0));
            assert!(font.is_none());
        }
        other => panic!("no question: {:?}", std::mem::discriminant(other)),
    }
    assert_eq!(app.text_default_size, before, "not before OK");
    // Cancel: the default stays; OK: it changes.
    app.dialog = crate::ui::dialogs::Dialog::None;
    assert_eq!(app.text_default_size, before);
    app.set_text_defaults(Some("Arial".into()), None, Some(36.0));
    assert_eq!(app.text_default_size, 36.0);
    assert_eq!(app.text_default_font, "Arial");
}

/// With Options > Text > Text units at millimetres the property bar shows
/// font sizes in millimetres, and a size typed in them is kept in points;
/// a unit typed with the value counts (#529).
#[test]
fn font_sizes_show_and_type_in_the_text_units() {
    use crate::app::Units;
    assert!((Units::Millimeters.from_pt(72.0) - 25.4).abs() < 1e-9);
    assert!((Units::Centimeters.to_pt(2.54) - 72.0).abs() < 1e-9);
    assert!((Units::Points.from_pt(24.0) - 24.0).abs() < 1e-12);
    let (ctx, mut app, _) = setup();
    app.start_text(Point::new(20.0, 100.0), None);
    app.text_insert("Big");
    app.finish_text();
    let t = app.selection[0];
    app.settings.text_units = "mm".into();
    let texts = |app: &mut App| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
        let mut all: Vec<(String, Pos2)> = Vec::new();
        fn walk(s: &egui::Shape, all: &mut Vec<(String, Pos2)>) {
            match s {
                egui::Shape::Text(t) => all.push((
                    t.galley.text().to_string(),
                    t.pos + t.galley.rect.center().to_vec2(),
                )),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, all)),
                _ => {}
            }
        }
        out.shapes.iter().for_each(|c| walk(&c.shape, &mut all));
        all
    };
    for _ in 0..5 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let size = match &app.doc().find_shape(t).expect("text").kind {
        ShapeKind::Text { spans, .. } => spans[0].size_pt,
        _ => unreachable!(),
    };
    let shown = format!(
        "{} mm",
        crate::ui::field::format_number(Units::Millimeters.from_pt(size), false, 0, Some(3))
    );
    // The object's height can read the same: the size field is the
    // rightmost.
    let field = texts(&mut app)
        .into_iter()
        .filter(|(s, _)| s.trim() == shown)
        .max_by(|a, b| a.1.x.total_cmp(&b.1.x))
        .map(|(_, p)| p)
        .unwrap_or_else(|| panic!("the size field showing {shown}"));
    click_at(&ctx, &mut app, field, Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::A, Modifiers::COMMAND)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![Event::Text("1cm".into())],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Enter, Modifiers::NONE)],
        Modifiers::NONE,
    );
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let after = match &app.doc().find_shape(t).expect("text").kind {
        ShapeKind::Text { spans, .. } => spans[0].size_pt,
        _ => unreachable!(),
    };
    assert!(
        (after - Units::Centimeters.to_pt(1.0)).abs() < 0.01,
        "{after}"
    );
}

/// Print Merge's Place Inside Frame with nothing selected waits for a
/// click on the shape, the pointer the Place Inside arrow; the click
/// takes the shape under it, Esc cancels (#577).
#[test]
fn place_inside_frame_waits_for_a_click_with_its_arrow() {
    let (ctx, mut app, id) = setup();
    app.select(Vec::new());
    app.pending_merge_frame = true;
    let at = app.view.to_screen(bounds(&app, id).center());
    assert_eq!(
        for_tool(&app, false, false, false, Some(at)),
        ToolCursor::TargetArrow
    );
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Escape, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert!(!app.pending_merge_frame, "Esc cancels");
    for _ in 0..20 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    app.pending_merge_frame = true;
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert!(!app.pending_merge_frame);
    assert_eq!(app.selection, vec![id], "the shape clicked");
}

/// How the buttons go up after a right click during a drag, as a desktop
/// sends them: each in a frame of its own, the left first or the right
/// first, or both in one frame.
#[derive(Clone, Copy, Debug)]
enum Release {
    RightThenLeft,
    LeftThenRight,
    BothRightFirst,
    BothLeftFirst,
}

const RELEASES: [Release; 4] = [
    Release::RightThenLeft,
    Release::LeftThenRight,
    Release::BothRightFirst,
    Release::BothLeftFirst,
];

/// A primary drag from `from` to `to` with `mods` held and a right click
/// at its end, the buttons going up as `release` says.
fn drag_with_right_click(
    ctx: &egui::Context,
    app: &mut App,
    from: Pos2,
    to: Pos2,
    mods: Modifiers,
    release: Release,
) {
    let left = |p: Pos2, down: bool| button(p, down, PointerButton::Primary, mods);
    let right = |p: Pos2, down: bool| button(p, down, PointerButton::Secondary, mods);
    frame(ctx, app, vec![Event::PointerMoved(from)], mods);
    frame(ctx, app, vec![left(from, true)], mods);
    for i in 1..=4 {
        let p = from + (to - from) * (i as f32 / 4.0);
        frame(ctx, app, vec![Event::PointerMoved(p)], mods);
    }
    frame(ctx, app, vec![right(to, true)], mods);
    match release {
        Release::RightThenLeft => {
            frame(ctx, app, vec![right(to, false)], mods);
            frame(ctx, app, vec![left(to, false)], mods);
        }
        Release::LeftThenRight => {
            frame(ctx, app, vec![left(to, false)], mods);
            frame(ctx, app, vec![right(to, false)], mods);
        }
        Release::BothRightFirst => frame(ctx, app, vec![right(to, false), left(to, false)], mods),
        Release::BothLeftFirst => frame(ctx, app, vec![left(to, false), right(to, false)], mods),
    }
    frame(ctx, app, Vec::new(), Modifiers::NONE);
}

/// A right click during a move leaves the original and moves a copy,
/// whichever button goes up first: the left one first used to count as a
/// click on the bare canvas, which emptied the selection before the copy
/// was made, so nothing happened at all.
#[test]
fn a_right_click_copy_survives_any_release_order() {
    for release in RELEASES {
        let (ctx, mut app, id) = setup();
        let from = app.view.to_screen(Point::new(35.0, 25.0));
        // Dropped on bare canvas, well right of the rectangle.
        let to = app.view.to_screen(Point::new(135.0, 25.0));
        drag_with_right_click(&ctx, &mut app, from, to, Modifiers::NONE, release);
        assert_eq!(shape_count(&app), 2, "{release:?}");
        assert!(
            close(bounds(&app, id).x0, 10.0),
            "the original stays: {release:?}"
        );
        let copy = app
            .selection
            .first()
            .copied()
            .expect("the copy is selected");
        assert_ne!(copy, id, "{release:?}");
        assert!(
            close(bounds(&app, copy).x0, 110.0),
            "{release:?} {:?}",
            bounds(&app, copy)
        );
        assert!(app.context_menu.is_none(), "{release:?}");
        assert_eq!(app.tool, Tool::Pick, "{release:?}");
        assert!(!app.rotate_mode, "{release:?}");
        assert!(!app.transforming(), "{release:?}");
    }
}

/// Dropped on another object, the copy is still of what was dragged; the
/// other object stays as it was.
#[test]
fn a_right_click_copy_dropped_on_another_object_copies_the_dragged_one() {
    for release in [Release::LeftThenRight, Release::BothLeftFirst] {
        let (ctx, mut app, id) = setup();
        let other = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(120.0, 10.0, 150.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![other]);
        app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
            30, 30, 200,
        )));
        app.select(vec![id]);
        let other_before = bounds(&app, other);
        let from = app.view.to_screen(Point::new(35.0, 25.0));
        let to = app.view.to_screen(Point::new(135.0, 25.0));
        drag_with_right_click(&ctx, &mut app, from, to, Modifiers::NONE, release);
        assert_eq!(shape_count(&app), 3, "{release:?}");
        assert_eq!(bounds(&app, other), other_before, "{release:?}");
        assert!(close(bounds(&app, id).x0, 10.0), "{release:?}");
        let copy = app.selection.first().copied().expect("copy");
        assert!(copy != id && copy != other, "{release:?}");
        assert!(close(bounds(&app, copy).x0, 110.0), "{release:?}");
    }
}

/// Mirror duplication with the mouse alone: the left handle dragged
/// across the right one with Ctrl (whole steps of 100 %), a right click,
/// release: the original stays, a mirrored copy lies beside it.
#[test]
fn a_right_click_during_a_ctrl_mirror_leaves_a_mirrored_copy() {
    for release in [Release::RightThenLeft, Release::LeftThenRight] {
        let (ctx, mut app, id) = setup();
        let w = handle(&app, Handle::W);
        let to = egui::pos2(app.view.to_screen(Point::new(110.0, 25.0)).x, w.y);
        drag_with_right_click(&ctx, &mut app, w, to, Modifiers::CTRL, release);
        assert_eq!(shape_count(&app), 2, "{release:?}");
        let original = app.doc().find_shape(id).expect("original");
        assert!(close(original.bounds().x0, 10.0) && close(original.bounds().x1, 60.0));
        assert!(original.transform.determinant() > 0.0);
        let copy = app.selection.first().copied().expect("copy");
        assert_ne!(copy, id);
        let c = app.doc().find_shape(copy).expect("copy");
        let b = c.bounds();
        assert!(close(b.x0, 60.0) && close(b.x1, 110.0), "{release:?} {b:?}");
        assert!(c.transform.determinant() < 0.0, "mirrored: {release:?}");
    }
}

/// In rotate mode (a second click on the selection) a right click during
/// a turn or a slant leaves the original and transforms a copy.
#[test]
fn a_right_click_during_a_turn_or_a_slant_leaves_the_original() {
    for which in [Handle::NE, Handle::N] {
        let (ctx, mut app, id) = setup();
        app.rotate_mode = true;
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
        let from = handle(&app, which);
        let to = from + egui::vec2(-60.0, -60.0);
        drag_with_right_click(
            &ctx,
            &mut app,
            from,
            to,
            Modifiers::NONE,
            Release::LeftThenRight,
        );
        assert_eq!(shape_count(&app), 2, "{which:?}");
        let original = app.doc().find_shape(id).expect("original");
        assert_eq!(original.transform, Affine::IDENTITY, "{which:?}");
        let copy = app.selection.first().copied().expect("copy");
        assert_ne!(copy, id);
        let t = app.doc().find_shape(copy).expect("copy").transform;
        assert_ne!(t, Affine::IDENTITY, "the copy turned or slanted: {which:?}");
    }
}

/// A second right click takes the copy back even when that press is held
/// while the pointer moves: the move goes on from where it began, it does
/// not start again from the right press.
#[test]
fn a_held_second_right_press_takes_the_copy_back_and_the_move_goes_on() {
    let (ctx, mut app, id) = setup();
    let n = Modifiers::NONE;
    let left = |p: Pos2, down: bool| button(p, down, PointerButton::Primary, n);
    let right = |p: Pos2, down: bool| button(p, down, PointerButton::Secondary, n);
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    frame(&ctx, &mut app, vec![Event::PointerMoved(from)], n);
    frame(&ctx, &mut app, vec![left(from, true)], n);
    let mut p = from;
    for _ in 0..4 {
        p += egui::vec2(20.0, 0.0);
        frame(&ctx, &mut app, vec![Event::PointerMoved(p)], n);
    }
    frame(&ctx, &mut app, vec![right(p, true)], n);
    frame(&ctx, &mut app, vec![right(p, false)], n);
    assert!(app.drag_copy);
    // The second press, held while the pointer goes 40 px further.
    frame(&ctx, &mut app, vec![right(p, true)], n);
    for _ in 0..4 {
        p += egui::vec2(10.0, 0.0);
        frame(&ctx, &mut app, vec![Event::PointerMoved(p)], n);
    }
    frame(&ctx, &mut app, vec![right(p, false)], n);
    frame(&ctx, &mut app, vec![left(p, false)], n);
    frame(&ctx, &mut app, Vec::new(), n);
    assert_eq!(shape_count(&app), 1, "no copy");
    let moved = app.view.to_page(p) - app.view.to_page(from);
    assert!(
        close(bounds(&app, id).x0, 10.0 + moved.x),
        "{:?} {moved:?}",
        bounds(&app, id)
    );
    assert_eq!(app.selection, vec![id]);
}

fn sym_center(app: &App, id: ShapeId) -> Point {
    use tracedraw_core::live::Effect;
    app.doc()
        .find_shape(id)
        .and_then(|s| {
            s.effects.iter().find_map(|e| match e {
                Effect::Symmetry { center, .. } => Some(*center),
                _ => None,
            })
        })
        .expect("a symmetry")
}

/// The rectangle with a symmetry about the upright line through its right
/// side (60, 25), finished: the rectangle and its copy, 10..110 x 10..40.
fn finished_symmetry() -> (egui::Context, App, ShapeId) {
    let (ctx, mut app, id) = setup();
    app.create_symmetry();
    assert_eq!(app.symmetry_edit, Some(id));
    app.finish_symmetry_edit();
    assert_eq!(app.symmetry_edit, None);
    assert_eq!(app.selection, vec![id]);
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let vb = app.selection_bounds().expect("bounds");
    assert!(close(vb.x0, 10.0) && close(vb.x1, 110.0), "{vb:?}");
    (ctx, app, id)
}

/// After Finish a drag on the copy moves the whole symmetry: the copy and
/// the mirror line go with the rectangle (the copy used to mirror the
/// move about the line left where it was, as while editing), and the
/// outline shown during the drag is the whole symmetry's. One Undo.
#[test]
fn a_finished_symmetry_moves_whole_with_its_copies() {
    let (ctx, mut app, id) = finished_symmetry();
    let from = app.view.to_screen(Point::new(85.0, 25.0));
    let to = from + egui::vec2(80.0, 0.0);
    drag(&ctx, &mut app, from, to, Modifiers::NONE, vec![]);
    assert_eq!(app.selection, vec![id], "the whole symmetry stays selected");
    let d = app.view.to_page(to).x - app.view.to_page(from).x;
    assert!(d > 5.0);
    let vb = app.selection_bounds().expect("bounds");
    assert!(
        close(vb.x0, 10.0 + d) && close(vb.x1, 110.0 + d),
        "{vb:?} {d}"
    );
    assert!(close(sym_center(&app, id).x, 60.0 + d));
    assert!(close(sym_center(&app, id).y, 25.0));
    app.undo();
    assert!(close(sym_center(&app, id).x, 60.0), "one undo step");
    assert!(close(bounds(&app, id).x0, 10.0));
}

/// After Finish the right handle stretches the whole symmetry: its left
/// side stays and the copy's right side follows the handle.
#[test]
fn a_finished_symmetry_scales_whole_with_its_copies() {
    let (ctx, mut app, id) = finished_symmetry();
    let h = handle(&app, Handle::E);
    let dx = app.view.to_screen(Point::new(160.0, 25.0)).x
        - app.view.to_screen(Point::new(110.0, 25.0)).x;
    drag(
        &ctx,
        &mut app,
        h,
        h + egui::vec2(dx, 0.0),
        Modifiers::NONE,
        vec![],
    );
    assert_eq!(app.selection, vec![id]);
    let vb = app.selection_bounds().expect("bounds");
    assert!(close(vb.x0, 10.0), "{vb:?}");
    assert!((vb.x1 - 160.0).abs() < 0.5, "{vb:?}");
}

/// While the symmetry is edited, moving its object leaves the mirror line
/// where it is (the copy mirrors the move), as before.
#[test]
fn editing_a_symmetry_a_drag_on_the_object_keeps_the_line() {
    let (ctx, mut app, id) = setup();
    app.create_symmetry();
    assert_eq!(app.symmetry_edit, Some(id));
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let at = app.view.to_screen(Point::new(30.0, 25.0));
    drag(
        &ctx,
        &mut app,
        at,
        at + egui::vec2(-40.0, 0.0),
        Modifiers::NONE,
        vec![],
    );
    assert!(bounds(&app, id).x0 < 9.0, "{:?}", bounds(&app, id));
    assert!(close(sym_center(&app, id).x, 60.0) && close(sym_center(&app, id).y, 25.0));
    // One undo step for the move.
    app.undo();
    assert!(close(bounds(&app, id).x0, 10.0));
}

/// A duplicate of a finished symmetry is a whole symmetry at the offset,
/// its line moved with it.
#[test]
fn a_duplicate_of_a_symmetry_takes_its_line_along() {
    let (_ctx, mut app, id) = finished_symmetry();
    app.duplicate();
    let copy = app.selection[0];
    assert_ne!(copy, id);
    let off = bounds(&app, copy).x0 - bounds(&app, id).x0;
    let offy = bounds(&app, copy).y0 - bounds(&app, id).y0;
    let c = sym_center(&app, copy);
    assert!(close(c.x, 60.0 + off) && close(c.y, 25.0 + offy), "{c:?}");
}

fn rect_at(app: &mut App, r: Rect) -> ShapeId {
    app.new_shape(ShapeKind::Rect {
        rect: r,
        radius: 0.0,
        corners: None,
    })
    .expect("rect")
}

/// A symmetry in a group goes with the group: its line stays put in the
/// group (it is stored in the group's space), the group's box takes the
/// moved copy, and ungrouping brings the line onto the page where the
/// object now is.
#[test]
fn a_symmetry_in_a_group_follows_the_group_and_an_ungroup() {
    let (_ctx, mut app, id) = finished_symmetry();
    let other = rect_at(&mut app, Rect::new(10.0, 60.0, 20.0, 70.0));
    app.select(vec![id, other]);
    app.group_selection();
    let g = app.selection[0];
    assert!(g != id && app.doc().shape(id).is_err(), "inside a group");
    app.run(tracedraw_core::Command::TransformShapes {
        shapes: vec![g],
        transform: Affine::translate((30.0, 0.0)),
    });
    assert!(
        close(sym_center(&app, id).x, 60.0),
        "stored in the group's space"
    );
    let group = app.doc().find_shape(g).expect("group").clone();
    let vb = group.visual_bounds();
    assert!(close(vb.x0, 40.0) && close(vb.x1, 140.0), "{vb:?}");
    app.select(vec![g]);
    app.ungroup_selection();
    assert!(app.doc().shape(id).is_ok(), "on the layer again");
    let c = sym_center(&app, id);
    assert!(close(c.x, 90.0) && close(c.y, 25.0), "{c:?}");
    let vb = app.doc().find_shape(id).expect("symmetry").visual_bounds();
    assert!(close(vb.x0, 40.0) && close(vb.x1, 140.0), "{vb:?}");
}

/// With another live effect on the object, the box measured while the
/// symmetry was edited (the object's own, as picked) does not outlast
/// Finish: the whole symmetry's box and handles come back at once, and a
/// click on the copy picks it.
#[test]
fn after_finish_the_box_is_the_whole_symmetrys_with_other_effects_too() {
    let (ctx, mut app, id) = setup();
    app.add_contour();
    app.create_symmetry();
    assert_eq!(app.symmetry_edit, Some(id));
    // Hover and hit tests while editing measure the object alone.
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let _ = app.hit_test(Point::new(85.0, 25.0));
    let _ = app.selection_bounds();
    app.finish_symmetry_edit();
    let vb = app.selection_bounds().expect("bounds");
    assert!(vb.x1 > 105.0, "the copy is in the box: {vb:?}");
    app.select(Vec::new());
    assert_eq!(app.hit_test(Point::new(85.0, 25.0)), Some(id));
}

/// While a lone object's symmetry is edited, its duplicate joins the
/// symmetry without a symmetry of its own (it would mirror it twice).
#[test]
fn a_duplicate_made_while_editing_joins_without_its_own_symmetry() {
    use tracedraw_core::live::Effect;
    let (_ctx, mut app, id) = setup();
    app.create_symmetry();
    assert_eq!(app.symmetry_edit, Some(id));
    app.duplicate();
    let copy = app.selection[0];
    assert_ne!(copy, id);
    let fx = &app.doc().find_shape(copy).expect("copy").effects;
    assert!(
        !fx.iter().any(|e| matches!(e, Effect::Symmetry { .. })),
        "{fx:?}"
    );
    let g = app.symmetry_edit.expect("still editing");
    assert!(app
        .doc()
        .find_shape(g)
        .expect("group")
        .effects
        .iter()
        .any(|e| matches!(e, Effect::Symmetry { .. })));
}

/// While editing, Align leaves the mirror line where it is; out of edit
/// mode Align moves it with the object.
#[test]
fn align_keeps_the_line_while_editing_and_moves_it_after() {
    let (_ctx, mut app, id) = setup();
    app.create_symmetry();
    app.align(crate::ops::Align::CenterPage);
    let moved = bounds(&app, id).x0 - 10.0;
    assert!(moved.abs() > 1.0);
    assert!(close(sym_center(&app, id).x, 60.0), "kept while editing");
    app.undo();
    assert!(close(bounds(&app, id).x0, 10.0) && close(sym_center(&app, id).x, 60.0));
    app.finish_symmetry_edit();
    app.align(crate::ops::Align::CenterPage);
    let moved = bounds(&app, id).x0 - 10.0;
    assert!(
        close(sym_center(&app, id).x, 60.0 + moved),
        "carried after Finish"
    );
}

/// Joining curves rewrites the kept curve in page space without moving
/// it: its symmetry stays where it was.
#[test]
fn joining_curves_leaves_a_symmetry_where_it_was() {
    use tracedraw_core::live::Effect;
    let (_ctx, mut app, _) = setup();
    let line = |app: &mut App, a: (f64, f64), b: (f64, f64)| {
        let mut p = tracedraw_core::geometry::BezPath::new();
        p.move_to(a);
        p.line_to(b);
        let id = app
            .new_shape(ShapeKind::Path {
                path: p,
                closed: false,
            })
            .expect("line");
        app.run(tracedraw_core::Command::SetEffects {
            shape: id,
            effects: vec![Effect::Symmetry {
                center: Point::new(5.0, 60.0),
                angle: 0.0,
                lines: 1,
            }],
        });
        id
    };
    let a = line(&mut app, (0.0, 50.0), (10.0, 50.0));
    let b = line(&mut app, (11.0, 50.0), (20.0, 50.0));
    app.select(vec![a, b]);
    app.transform_selection(Affine::translate((30.0, 0.0)));
    assert!(close(sym_center(&app, a).x, 35.0) && close(sym_center(&app, b).x, 35.0));
    app.join_settings.gap = 5.0;
    app.select(vec![a, b]);
    assert!(app.join_curves() > 0);
    let kept = if app.doc().find_shape(a).is_some() {
        a
    } else {
        b
    };
    let c = sym_center(&app, kept);
    assert!(close(c.x, 35.0) && close(c.y, 60.0), "{c:?}");
}

/// Drawing a rectangle, the corner being dragged is where the pointer is
/// in the same frame (no frame behind it), and the pointer shown is the
/// drawn cross hair, not the system's (#655).
#[test]
fn drawing_a_rectangle_the_corner_keeps_up_with_the_pointer() {
    let (ctx, mut app, _) = setup();
    app.set_tool(Tool::Rectangle);
    app.snap.off = true;
    let from = app.view.to_screen(Point::new(100.0, 150.0));
    let to = app.view.to_screen(Point::new(140.0, 120.0));
    let n = Modifiers::NONE;
    frame(&ctx, &mut app, vec![Event::PointerMoved(from)], n);
    frame(
        &ctx,
        &mut app,
        vec![button(from, true, PointerButton::Primary, n)],
        n,
    );
    let mut checked = 0;
    for i in 1..=8 {
        let p = from + (to - from) * (i as f32 / 8.0);
        frame(&ctx, &mut app, vec![Event::PointerMoved(p)], n);
        if let Drag::Box { current, .. } = app.drag {
            let want = app.view.to_page(p);
            assert!(
                close(current.x, want.x) && close(current.y, want.y),
                "{current:?} {want:?}"
            );
            checked += 1;
        }
        assert_eq!(
            crate::ui::cursors::for_tool(&app, false, true, false, Some(p)),
            ToolCursor::Badge(Tool::Rectangle)
        );
    }
    assert!(checked >= 6, "the drag was followed: {checked}");
    frame(
        &ctx,
        &mut app,
        vec![button(to, false, PointerButton::Primary, n)],
        n,
    );
}

/// Ctrl+Q on an enveloped object makes the envelope part of its curve
/// (#647): it did nothing once the object was a curve.
#[test]
fn ctrl_q_bakes_an_envelope() {
    let (ctx, mut app, id) = setup();
    app.apply_envelope_preset(crate::envelope_edit::Preset::Arch);
    frame(&ctx, &mut app, vec![], Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Q, Modifiers::COMMAND)],
        Modifiers::COMMAND,
    );
    let s = app.doc().find_shape(id).expect("shape");
    assert!(matches!(s.kind, ShapeKind::Path { .. }), "{:?}", s.kind);
    assert!(s.effects.is_empty(), "{:?}", s.effects);
    assert!(s.bounds().y1 > 41.0, "{:?}", s.bounds());
}

#[test]
fn save_as_starts_at_the_cdr_type_and_version_the_options_set() {
    let (ctx, mut app, _) = setup();
    let save_as = |ctx: &egui::Context, app: &mut App| {
        frame(
            ctx,
            app,
            vec![key(egui::Key::S, Modifiers::COMMAND | Modifiers::SHIFT)],
            Modifiers::COMMAND | Modifiers::SHIFT,
        );
        frame(ctx, app, Vec::new(), Modifiers::NONE);
        match &app.dialog {
            crate::ui::dialogs::Dialog::SaveAs(st) => (st.cdr, st.version),
            other => panic!("no Save Drawing dialog: {other:?}"),
        }
    };
    // By default a new drawing is offered as TraceDraw, version 27 for .cdr.
    assert_eq!(
        save_as(&ctx, &mut app),
        (false, tracedraw_cdr::write::NEWEST_VERSION)
    );
    app.dialog = crate::ui::dialogs::Dialog::None;
    // Options > Global > File Formats: version 15 and .cdr for new drawings.
    app.settings.formats.cdr_version = 15;
    app.settings.formats.save_new_as_cdr = true;
    assert_eq!(save_as(&ctx, &mut app), (true, 15));
}

/// One frame; the texts drawn and where.
fn texts_drawn(
    ctx: &egui::Context,
    app: &mut App,
    events: Vec<Event>,
) -> Vec<(String, egui::Rect)> {
    frame_shapes(ctx, app, events)
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) => Some((
                t.galley.text().to_string(),
                t.galley.rect.translate(t.pos.to_vec2()),
            )),
            _ => None,
        })
        .collect()
}

/// The centre of the last text `label` drawn in a frame, if any.
fn text_at(texts: &[(String, egui::Rect)], label: &str) -> Option<Pos2> {
    texts
        .iter()
        .rev()
        .find(|(t, _)| t == label)
        .map(|(_, r)| r.center())
}

/// The steps of the report's recording (#613): Options > Global > File
/// Formats, the version list set to v15, OK; the preferences written and
/// read back as at the next start; Save As then offers v15 for a new
/// drawing, and a drawing opened from a v20 file starts at v20.
#[test]
fn the_cdr_version_chosen_in_options_survives_a_restart_and_starts_save_as() {
    use crate::ui::dialogs::Dialog;
    use crate::ui::options::OptionsPage;
    let (ctx, mut app, _) = setup();
    app.dialog = Dialog::Options;
    app.options_page = OptionsPage::FileFormats;
    let newest = crate::ui::save_as::version_label(tracedraw_cdr::write::NEWEST_VERSION);
    let wanted = crate::ui::save_as::version_label(15);
    let mut texts = Vec::new();
    for _ in 0..4 {
        texts = texts_drawn(&ctx, &mut app, Vec::new());
    }
    let combo = text_at(&texts, &newest).expect("the version list shows v27");
    click_at(&ctx, &mut app, combo, Modifiers::NONE);
    // The list opens; scroll it down until v15 shows.
    let mut item = None;
    for _ in 0..20 {
        texts = texts_drawn(&ctx, &mut app, Vec::new());
        item = text_at(&texts, &wanted);
        if item.is_some() {
            break;
        }
        let over = text_at(&texts, &newest).unwrap_or(combo) + egui::vec2(0.0, 60.0);
        texts_drawn(
            &ctx,
            &mut app,
            vec![
                Event::PointerMoved(over),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, -3.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    let item = item.expect("v15 in the list");
    click_at(&ctx, &mut app, item, Modifiers::NONE);
    assert_eq!(app.settings.formats.cdr_version, 15);
    texts = texts_drawn(&ctx, &mut app, Vec::new());
    assert!(
        texts
            .iter()
            .any(|(t, _)| t == &crate::i18n::tr("options.cdr_version_hint")),
        "the hint under the list"
    );
    let ok = text_at(&texts, &crate::i18n::tr("dialog.ok")).expect("OK");
    click_at(&ctx, &mut app, ok, Modifiers::NONE);
    assert!(matches!(app.dialog, Dialog::None), "{:?}", app.dialog);
    // The preferences as the next start reads them.
    let saved = serde_json::to_string(&app.settings).expect("saved");
    app.settings = serde_json::from_str::<crate::settings::Settings>(&saved)
        .expect("read back")
        .upgrade();
    let save_as = |ctx: &egui::Context, app: &mut App| {
        frame(
            ctx,
            app,
            vec![key(egui::Key::S, Modifiers::COMMAND | Modifiers::SHIFT)],
            Modifiers::COMMAND | Modifiers::SHIFT,
        );
        frame(ctx, app, Vec::new(), Modifiers::NONE);
        let v = match &app.dialog {
            Dialog::SaveAs(st) => st.version,
            other => panic!("no Save Drawing dialog: {other:?}"),
        };
        app.dialog = Dialog::None;
        v
    };
    assert_eq!(save_as(&ctx, &mut app), 15);
    // A drawing opened from a version 20 file keeps its version.
    let dir = std::env::temp_dir().join(format!("td_613_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("folder");
    let v20 = dir.join("v20.cdr");
    let doc = app.doc().clone();
    std::fs::write(
        &v20,
        tracedraw_cdr::write::document_to_cdr_version(&doc, 20),
    )
    .expect("written");
    app.open_path(v20);
    assert_eq!(app.cdr_version, Some(20));
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert_eq!(save_as(&ctx, &mut app), 20);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_options_dialogs_are_tabs_of_one_window() {
    use crate::ui::dialogs::{Dialog, OptionsPage};
    use crate::ui::options::{tab_rect_id, OptionsSet};
    let (ctx, mut app, _) = setup();
    app.dialog = Dialog::Options;
    app.options_page = OptionsPage::Display;
    // The window takes its place over the first frames.
    for _ in 0..4 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let tab = |ctx: &egui::Context, s: OptionsSet| {
        ctx.data(|d| d.get_temp::<egui::Rect>(tab_rect_id(s)))
            .unwrap_or_else(|| panic!("no tab for {s:?}"))
            .center()
    };
    // Every set has its tab, Document Options too while a drawing is open.
    for s in OptionsSet::ALL {
        tab(&ctx, s);
    }
    click_at(
        &ctx,
        &mut app,
        tab(&ctx, OptionsSet::Global),
        Modifiers::NONE,
    );
    assert_eq!(app.options_page, OptionsPage::GlobalGeneral);
    assert!(matches!(app.dialog, Dialog::Options));
    click_at(
        &ctx,
        &mut app,
        tab(&ctx, OptionsSet::Document),
        Modifiers::NONE,
    );
    assert_eq!(app.options_page, OptionsPage::DocumentGeneral);
    // Back on the first tab, the page shown there last.
    click_at(&ctx, &mut app, tab(&ctx, OptionsSet::App), Modifiers::NONE);
    assert_eq!(app.options_page, OptionsPage::Display);
    // Cancel still closes the one window.
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Escape, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert!(matches!(app.dialog, Dialog::None), "{:?}", app.dialog);
}

/// The page radius of each corner of rectangle `id`.
fn corner_radii(app: &App, id: ShapeId) -> [f64; 4] {
    app.doc()
        .find_shape(id)
        .and_then(|s| s.page_corners())
        .map(|c| c.radii)
        .expect("a rectangle")
}

/// Shape tool on a rectangle: a marquee over two corners chooses both,
/// Shift adds a corner to a click's, and dragging a chosen corner then
/// rounds the chosen corners only (#671).
#[test]
fn chosen_rectangle_corners_round_together() {
    let (ctx, mut app, id) = setup();
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let screen = |app: &App, x: f64, y: f64| app.view.to_screen(Point::new(x, y));
    // A marquee round the two corners at y = 40.
    let (from, to) = (screen(&app, 0.0, 45.0), screen(&app, 70.0, 35.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    assert_eq!(
        app.rect_corners_selected.len(),
        2,
        "{:?}",
        app.rect_corners_selected
    );
    let (from, to) = (screen(&app, 60.0, 40.0), screen(&app, 54.0, 40.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    let r = corner_radii(&app, id);
    let rounded: Vec<bool> = r.iter().map(|v| *v > 1.0).collect();
    assert_eq!(rounded.iter().filter(|b| **b).count(), 2, "{r:?}");
    for v in r.iter().filter(|v| **v > 1.0) {
        assert!((v - 6.0).abs() < 0.2, "{r:?}");
    }
    let before = r;
    // A click on one corner at y = 10 and Shift on the other: those two.
    let at = screen(&app, 10.0, 10.0);
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    let at = screen(&app, 60.0, 10.0);
    click_at(&ctx, &mut app, at, Modifiers::SHIFT);
    assert_eq!(
        app.rect_corners_selected.len(),
        2,
        "{:?}",
        app.rect_corners_selected
    );
    let (from, to) = (screen(&app, 60.0, 10.0), screen(&app, 60.0, 14.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    let r = corner_radii(&app, id);
    for i in 0..4 {
        if before[i] > 1.0 {
            assert!((r[i] - before[i]).abs() < 1e-6, "the top ones stay: {r:?}");
        } else {
            assert!((r[i] - 4.0).abs() < 0.2, "the bottom ones round: {r:?}");
        }
    }
    // Shift on a chosen corner takes it out again.
    let at = screen(&app, 60.0, 14.0);
    click_at(&ctx, &mut app, at, Modifiers::SHIFT);
    assert_eq!(
        app.rect_corners_selected.len(),
        1,
        "{:?}",
        app.rect_corners_selected
    );
}

/// Pen: a click on the end node of an open curve that is not selected
/// goes on from it as if drawing had never stopped: the next segment
/// leaves along the mirror of the end's handle, the preview is curved,
/// and the new nodes join that curve smoothly (#630).
#[test]
fn the_pen_continues_an_unselected_curve_with_the_mirrored_handle() {
    let (ctx, mut app, first) = setup();
    app.select(vec![first]);
    app.delete_selection();
    app.set_tool(Tool::Pen);
    let a = Point::new(100.0, 150.0);
    let b = Point::new(140.0, 150.0);
    pull(&ctx, &mut app, a, Point::new(110.0, 140.0));
    pull(&ctx, &mut app, b, Point::new(150.0, 160.0));
    app.finish_curve();
    let id = app.selection[0];
    let before = shape_count(&app);
    // Unselected, then the Pen again on its end node.
    app.select(Vec::new());
    app.set_tool(Tool::Pen);
    click_page(&ctx, &mut app, b);
    assert_eq!(app.selection, vec![id], "the curve is taken up");
    let c = app.curve.clone().expect("a curve going on");
    assert_eq!(c.nodes.len(), 1);
    assert!(near(c.nodes[0].0, b), "{c:?}");
    // The curve arrived at b along the mirror of the pulled handle, at
    // (130, 140); the next segment leaves along (150, 160).
    let out = c.nodes[0].1.expect("a handle to leave along");
    assert!(near(out, Point::new(150.0, 160.0)), "{out:?}");
    let pointer = Point::new(180.0, 150.0);
    let look = crate::pen_look::curve_look(&c, Some(pointer), true, None);
    let preview = look.preview.expect("a preview");
    assert!(
        preview
            .elements()
            .iter()
            .any(|e| matches!(e, tracedraw_core::geometry::PathEl::CurveTo(..))),
        "curved: {preview:?}"
    );
    click_page(&ctx, &mut app, pointer);
    app.finish_curve();
    assert_eq!(shape_count(&app), before, "no new object");
    let segs: Vec<_> = last_path(&app).segments().collect();
    assert_eq!(segs.len(), 2, "{segs:?}");
    // Smooth through b: the handles on either side are in line.
    let (s0, s1) = (cubic(segs[0]), cubic(segs[1]));
    let (i, o) = (s0.p2 - b, s1.p1 - b);
    assert!(
        (i.x * o.y - i.y * o.x).abs() < 1e-6 && i.dot(o) < 0.0,
        "{s0:?} {s1:?}"
    );
}

/// The shapes and texts of one frame: every filled rectangle with its
/// colour, and every text with its centre.
fn painted(
    ctx: &egui::Context,
    app: &mut App,
) -> (Vec<(egui::Rect, egui::Color32)>, Vec<(String, Pos2)>) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
    out.textures_delta.clear();
    let mut rects = Vec::new();
    let mut texts = Vec::new();
    fn walk(
        s: &egui::Shape,
        rects: &mut Vec<(egui::Rect, egui::Color32)>,
        texts: &mut Vec<(String, Pos2)>,
    ) {
        match s {
            egui::Shape::Rect(r) => rects.push((r.rect, r.fill)),
            egui::Shape::Text(t) => texts.push((
                t.galley.text().to_string(),
                t.pos + t.galley.rect.center().to_vec2(),
            )),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, rects, texts)),
            _ => {}
        }
    }
    out.shapes
        .iter()
        .for_each(|c| walk(&c.shape, &mut rects, &mut texts));
    (rects, texts)
}

/// View > Mask Outside Page (#615), as the reference program shows it:
/// an opaque near-black over everything outside the page when the
/// desktop is white (the desktop colour otherwise), the page clear, the
/// selected object's outline drawn in blue over the mask so its part off
/// the page can still be found, the objects editable; the item sits
/// right after Pixels in the View menu and turns the mask off again.
#[test]
fn mask_outside_page_covers_the_desktop_and_keeps_objects_editable() {
    let (ctx, mut app, id) = setup();
    // The rectangle reaches past the page's left edge.
    app.select(vec![id]);
    app.nudge(Vec2::new(-30.0, 0.0));
    let mask = crate::canvas::OUTSIDE_PAGE_DARK;
    assert_eq!(crate::canvas::outside_page_mask_color(&app), mask);
    assert_eq!(mask.a(), 255, "opaque: nothing outside shows through");
    let page = app.view.rect_to_screen(app.page_rect());
    let shape = app.view.rect_to_screen(bounds(&app, id));
    let left_of_page = Pos2::new(page.left() - 5.0, shape.center().y);
    assert!(shape.contains(left_of_page), "the object lies there");
    let masked_at = |rects: &[(egui::Rect, egui::Color32)], p: Pos2| {
        rects.iter().any(|(r, c)| *c == mask && r.contains(p))
    };
    let (rects, _) = painted(&ctx, &mut app);
    assert!(!masked_at(&rects, left_of_page), "off at first");
    let menu_item = |ctx: &egui::Context, app: &mut App, name: &str| {
        let (_, texts) = painted(ctx, app);
        texts
            .into_iter()
            .find(|(t, _)| t.trim() == name)
            .map(|(_, p)| p)
            .unwrap_or_else(|| panic!("{name} in the menus"))
    };
    let view = menu_item(&ctx, &mut app, "View");
    click_at(&ctx, &mut app, view, Modifiers::NONE);
    // Right below Pixels, before Full-Screen Preview.
    let pixels = menu_item(&ctx, &mut app, "Pixels");
    let item = menu_item(&ctx, &mut app, "Mask Outside Page");
    let full = menu_item(&ctx, &mut app, "Full-screen Preview");
    assert!(
        item.y > pixels.y && item.y < full.y && item.y - pixels.y < 40.0,
        "{pixels:?} {item:?} {full:?}"
    );
    click_at(&ctx, &mut app, item, Modifiers::NONE);
    assert!(app.mask_outside_page);
    let (rects, _) = painted(&ctx, &mut app);
    assert!(masked_at(&rects, left_of_page), "the desktop is covered");
    assert!(!masked_at(&rects, page.center()), "the page is not");
    // The selected object's outline over the mask: its top edge off the
    // page in the selection blue.
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        ..Default::default()
    };
    let out = ctx.run_ui(input, |ui| crate::ui::root(&mut app, ui));
    let mut outline = false;
    let mut after_mask = false;
    let mut seen_mask = false;
    fn flat(s: &egui::Shape, f: &mut dyn FnMut(&egui::Shape)) {
        match s {
            egui::Shape::Vec(v) => v.iter().for_each(|s| flat(s, f)),
            s => f(s),
        }
    }
    for c in &out.shapes {
        flat(&c.shape, &mut |s| match s {
            egui::Shape::Rect(r) if r.fill == mask => seen_mask = true,
            egui::Shape::LineSegment { points, stroke }
                if stroke.color == crate::theme::Tokens::SELECTION
                    && points.iter().all(|p| (p.y - shape.top()).abs() < 1.5)
                    && points.iter().any(|p| p.x < page.left() - 1.0) =>
            {
                outline = true;
                after_mask |= seen_mask;
            }
            _ => {}
        });
    }
    assert!(outline && after_mask, "the outline over the mask");
    // Still a live drawing: the rectangle moves with a drag.
    let x0 = bounds(&app, id).x0;
    let c = app.view.to_screen(bounds(&app, id).center());
    drag(
        &ctx,
        &mut app,
        c,
        c + egui::vec2(40.0, 0.0),
        Modifiers::NONE,
        Vec::new(),
    );
    assert!(bounds(&app, id).x0 > x0 + 1.0, "moved");
    // The preference keeps it.
    assert!(app.view_prefs().mask_outside_page);
    // A desktop colour of the user's own is the mask's colour.
    app.settings.desktop_custom = true;
    app.settings.desktop_rgb = [90, 100, 110];
    let own = egui::Color32::from_rgb(90, 100, 110);
    assert_eq!(crate::canvas::outside_page_mask_color(&app), own);
    let (rects, _) = painted(&ctx, &mut app);
    assert!(rects
        .iter()
        .any(|(r, c)| *c == own && r.contains(left_of_page)));
    app.mask_outside_page = false;
    let (rects, _) = painted(&ctx, &mut app);
    assert!(!masked_at(&rects, left_of_page));
}

/// The Autofit Page popup stays open when its Live preview box or its
/// margin field is clicked, so the preview can be switched and watched
/// (#659): real clicks, the preview following the box; a click outside
/// closes it.
#[test]
fn the_autofit_popup_stays_open_when_its_box_is_clicked() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    let popup = crate::ui::propbar::autofit_popup_id();
    egui::Popup::open_id(&ctx, popup);
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    assert!(egui::Popup::is_id_open(&ctx, popup));
    assert!(app.settings.fit_page_preview && app.autofit_preview.is_some());
    let text_at = |ctx: &egui::Context, app: &mut App, name: &str| {
        let (_, texts) = painted(ctx, app);
        texts
            .into_iter()
            .find(|(t, _)| t.trim() == name)
            .map(|(_, p)| p)
            .unwrap_or_else(|| panic!("{name} in the popup"))
    };
    let live = text_at(&ctx, &mut app, "Live preview");
    let margin = text_at(&ctx, &mut app, "Margin:");
    assert!((live.y - margin.y).abs() < 4.0, "on the margin's row");
    click_at(&ctx, &mut app, live, Modifiers::NONE);
    assert!(egui::Popup::is_id_open(&ctx, popup), "still open");
    assert!(!app.settings.fit_page_preview, "the box turned off");
    assert!(app.autofit_preview.is_none(), "no preview");
    click_at(&ctx, &mut app, live, Modifiers::NONE);
    assert!(egui::Popup::is_id_open(&ctx, popup), "still open");
    assert!(app.settings.fit_page_preview && app.autofit_preview.is_some());
    // The margin field, right of its label.
    click_at(
        &ctx,
        &mut app,
        margin + egui::vec2(60.0, 0.0),
        Modifiers::NONE,
    );
    assert!(egui::Popup::is_id_open(&ctx, popup), "open after the field");
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Escape, Modifiers::NONE)],
        Modifiers::NONE,
    );
    // A click on the canvas closes it.
    let page = app.view.rect_to_screen(app.page_rect());
    click_at(&ctx, &mut app, page.center(), Modifiers::NONE);
    assert!(
        !egui::Popup::is_id_open(&ctx, popup),
        "closed by a click outside"
    );
    assert!(app.autofit_preview.is_none());
}

/// The canvas context menu offers Break Apart for an enveloped text, as
/// the Object menu and Ctrl+K do; a real click on it bakes the envelope
/// into one curve per letter (#647).
#[test]
fn the_context_menu_breaks_an_enveloped_text_apart() {
    let (ctx, mut app, rect) = setup();
    app.select(vec![rect]);
    app.delete_selection();
    tracedraw_text::install();
    app.start_text(Point::new(40.0, 120.0), None);
    app.text_insert("ABC");
    let id = app.text_edit.as_ref().expect("editing").shape;
    app.finish_text();
    app.set_tool(Tool::Pick);
    app.select(vec![id]);
    app.apply_envelope_preset(crate::envelope_edit::Preset::Arch);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let at = app.view.to_screen(bounds(&app, id).center());
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(at)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(at, true, PointerButton::Secondary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(at, false, PointerButton::Secondary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert!(app.context_menu.is_some());
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let (_, texts) = painted(&ctx, &mut app);
    let item = texts
        .into_iter()
        .find(|(t, _)| t.trim() == "Break Apart")
        .map(|(_, p)| p)
        .expect("Break Apart in the context menu");
    click_at(&ctx, &mut app, item, Modifiers::NONE);
    assert_eq!(app.selection.len(), 3, "a curve per letter");
    for p in &app.selection {
        let s = app.doc().shape_on_page(*p).expect("piece");
        assert!(matches!(s.kind, ShapeKind::Path { .. }), "{:?}", s.kind);
        assert!(s.effects.is_empty());
    }
}

/// The Shape tool's corner widgets (#635): with no node chosen, dragging
/// one into its corner rounds every corner of the curve together, live,
/// in one undo step; with a node chosen only its corner shows a widget
/// and rounds; an open curve rounds its inner corners.
#[test]
fn corner_widgets_round_every_corner_or_the_chosen_ones() {
    use tracedraw_core::geometry::{BezPath, PathEl, Shape as _};
    let (ctx, mut app, rect) = setup();
    app.select(vec![rect]);
    app.delete_selection();
    let mut square = BezPath::new();
    square.move_to((20.0, 20.0));
    square.line_to((60.0, 20.0));
    square.line_to((60.0, 60.0));
    square.line_to((20.0, 60.0));
    square.close_path();
    let id = app
        .new_shape(ShapeKind::Path {
            path: square.clone(),
            closed: true,
        })
        .expect("square");
    app.select(vec![id]);
    app.set_tool(Tool::Shape);
    // The widgets are off unless the preference turns them on (#884).
    app.settings.nodes.corner_widgets = true;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let path_of = |app: &App| match &app.doc().find_shape(id).expect("curve").kind {
        ShapeKind::Path { path, .. } => path.clone(),
        _ => panic!("a curve"),
    };
    let arcs = |p: &BezPath| {
        p.elements()
            .iter()
            .filter(|e| matches!(e, PathEl::CurveTo(..)))
            .count()
    };
    let widgets = app.corner_widgets();
    assert_eq!(widgets.len(), 4, "one on every corner");
    // Each sits inside the square, near its node.
    let inside = egui::Rect::from_two_pos(
        app.view.to_screen(Point::new(20.0, 20.0)),
        app.view.to_screen(Point::new(60.0, 60.0)),
    );
    assert!(widgets.iter().all(|w| inside.contains(w.at)));
    // Dragged 5 mm into its corner (towards the square's centre).
    let w = widgets[0];
    let centre = app.view.to_screen(Point::new(40.0, 40.0));
    let dir = (centre - w.at).normalized();
    let to = w.at + dir * (5.0 * app.view.zoom);
    let undo_before = app.engine.undo_label().map(str::to_string);
    drag(&ctx, &mut app, w.at, to, Modifiers::NONE, Vec::new());
    let rounded = path_of(&app);
    assert_eq!(arcs(&rounded), 4, "every corner rounded");
    // The arc's middle came about 5 mm into the corner: a fillet of about
    // 5 / (sqrt 2 - 1) mm on a right angle.
    let b = rounded.bounding_box();
    assert!(
        (b.width() - 40.0).abs() < 1e-6,
        "the sides keep their place"
    );
    let r = crate::corner_widgets::radius_for(5.0, std::f64::consts::FRAC_PI_2);
    assert!((r - 12.07).abs() < 0.1, "{r}");
    assert!(
        (rounded.area().abs() - (1600.0 - (4.0 - std::f64::consts::PI) * r * r)).abs() < 3.0,
        "{}",
        rounded.area()
    );
    assert!(app.status.contains("12.07"), "{}", app.status);
    // One undo step.
    app.undo();
    assert_eq!(path_of(&app), square);
    assert_eq!(app.engine.undo_label().map(str::to_string), undo_before);
    // A chosen node: only its corner has a widget, and only it rounds.
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let node = app.view.to_screen(Point::new(60.0, 60.0));
    click_at(&ctx, &mut app, node, Modifiers::NONE);
    assert_eq!(app.node_selection.len(), 1, "{:?}", app.node_selection);
    let widgets = app.corner_widgets();
    assert_eq!(widgets.len(), 1);
    let w = widgets[0];
    let dir = (centre - w.at).normalized();
    drag(
        &ctx,
        &mut app,
        w.at,
        w.at + dir * 30.0,
        Modifiers::NONE,
        Vec::new(),
    );
    assert_eq!(arcs(&path_of(&app)), 1, "that corner only");
    // An open zigzag rounds its two inner corners.
    let mut zig = BezPath::new();
    zig.move_to((20.0, 100.0));
    zig.line_to((50.0, 100.0));
    zig.line_to((50.0, 130.0));
    zig.line_to((80.0, 130.0));
    let open = app
        .new_shape(ShapeKind::Path {
            path: zig,
            closed: false,
        })
        .expect("zigzag");
    app.select(vec![open]);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let widgets = app.corner_widgets();
    assert_eq!(widgets.len(), 2);
    let w = widgets[0];
    let node = app.view.to_screen(Point::new(50.0, 100.0));
    let dir = (w.at - node).normalized();
    drag(
        &ctx,
        &mut app,
        w.at,
        w.at + dir * 20.0,
        Modifiers::NONE,
        Vec::new(),
    );
    match &app.doc().find_shape(open).expect("zigzag").kind {
        ShapeKind::Path { path, closed } => {
            assert!(!closed);
            assert_eq!(arcs(path), 2, "both inner corners");
        }
        _ => panic!("a curve"),
    }
}

/// The corner widgets on curve nodes are a preference, off by default
/// (#884): with it off a selected curve's corners show no widget, a press
/// where one would sit does not round anything, and choosing a node works
/// as before; turning it on shows them again. The preference is saved
/// with the others.
#[test]
fn corner_widgets_follow_their_preference_off_by_default() {
    use tracedraw_core::geometry::{BezPath, PathEl};
    assert!(!crate::settings::Settings::default().nodes.corner_widgets);
    let (ctx, mut app, rect) = setup();
    assert!(!app.settings.nodes.corner_widgets);
    app.select(vec![rect]);
    app.delete_selection();
    let mut square = BezPath::new();
    square.move_to((20.0, 20.0));
    square.line_to((60.0, 20.0));
    square.line_to((60.0, 60.0));
    square.line_to((20.0, 60.0));
    square.close_path();
    let id = app
        .new_shape(ShapeKind::Path {
            path: square.clone(),
            closed: true,
        })
        .expect("square");
    app.select(vec![id]);
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert!(app.corner_widgets().is_empty(), "off by default");
    // Where the widget of the corner at (20, 20) would sit: 18 px from the
    // node towards the centre.
    let node = app.view.to_screen(Point::new(20.0, 20.0));
    let centre = app.view.to_screen(Point::new(40.0, 40.0));
    let at = node + (centre - node).normalized() * crate::corner_widgets::WIDGET_OFFSET_PX;
    assert!(app.corner_widget_at(app.view.to_page(at)).is_none());
    drag(
        &ctx,
        &mut app,
        at,
        at + (centre - node).normalized() * 30.0,
        Modifiers::NONE,
        Vec::new(),
    );
    match &app.doc().find_shape(id).expect("square").kind {
        ShapeKind::Path { path, .. } => assert!(
            !path
                .elements()
                .iter()
                .any(|e| matches!(e, PathEl::CurveTo(..))),
            "no corner rounded"
        ),
        _ => panic!("a curve"),
    }
    // A node is still chosen with a click.
    app.select(vec![id]);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    click_at(&ctx, &mut app, node, Modifiers::NONE);
    assert_eq!(app.node_selection.len(), 1, "{:?}", app.node_selection);
    assert!(app.corner_widgets().is_empty());
    // Turned on, the chosen node's corner has its widget.
    app.settings.nodes.corner_widgets = true;
    assert_eq!(app.corner_widgets().len(), 1);
    // Saved and read back with the other preferences.
    let json = serde_json::to_string(&app.settings).expect("settings");
    let back: crate::settings::Settings = serde_json::from_str(&json).expect("read back");
    assert!(back.nodes.corner_widgets);
    // Settings saved before the preference existed read it as off.
    let old: crate::settings::NodePrefs = serde_json::from_str("{}").expect("old prefs");
    assert!(!old.corner_widgets);
}

/// A corner rounded with a widget can be rounded again (#872): the curve
/// selected later shows a widget off the middle of each arc, and
/// dragging it changes the radius from where it was, further in or back
/// to a sharp corner, in one undo step; with an arc's node chosen only
/// that corner changes.
#[test]
fn rounded_corners_keep_their_widgets_and_round_again() {
    use tracedraw_core::corner_cut::fillets;
    use tracedraw_core::geometry::{BezPath, Shape as _};
    let (ctx, mut app, rect) = setup();
    app.select(vec![rect]);
    app.delete_selection();
    let mut square = BezPath::new();
    square.move_to((20.0, 20.0));
    square.line_to((60.0, 20.0));
    square.line_to((60.0, 60.0));
    square.line_to((20.0, 60.0));
    square.close_path();
    let id = app
        .new_shape(ShapeKind::Path {
            path: square.clone(),
            closed: true,
        })
        .expect("square");
    app.select(vec![id]);
    app.set_tool(Tool::Shape);
    // The widgets are off unless the preference turns them on (#884).
    app.settings.nodes.corner_widgets = true;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let path_of = |app: &App| match &app.doc().find_shape(id).expect("curve").kind {
        ShapeKind::Path { path, .. } => path.clone(),
        _ => panic!("a curve"),
    };
    let centre = app.view.to_screen(Point::new(40.0, 40.0));
    let zoom = app.view.zoom;
    // Every drag below moves far enough to be a drag, not a click.
    assert!(5.0 * zoom > 8.0, "{zoom}");
    let r_of = |d: f64| crate::corner_widgets::radius_for(d, std::f64::consts::FRAC_PI_2);
    // Rounded 7 mm in.
    let w = app.corner_widgets()[0];
    let dir = (centre - w.at).normalized();
    drag(
        &ctx,
        &mut app,
        w.at,
        w.at + dir * (7.0 * zoom),
        Modifiers::NONE,
        Vec::new(),
    );
    let once = path_of(&app);
    let found = fillets(&once);
    assert_eq!(found.len(), 4, "{found:?}");
    assert!(found.iter().all(|f| (f.radius - r_of(7.0)).abs() < 0.05));
    // Selected again: a widget on every rounded corner, off its arc.
    app.select(Vec::new());
    app.select(vec![id]);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // Rounded corners follow the preference too (#884).
    app.settings.nodes.corner_widgets = false;
    assert!(app.corner_widgets().is_empty(), "off: no widget on an arc");
    app.settings.nodes.corner_widgets = true;
    let widgets = app.corner_widgets();
    assert_eq!(widgets.len(), 4, "{widgets:?}");
    let w = widgets[0];
    assert!((w.depth - 7.0).abs() < 0.05, "{}", w.depth);
    let mid = app.view.to_screen(w.corner.at + w.corner.inside * w.depth);
    assert!(
        ((w.at - mid).length() - crate::corner_widgets::WIDGET_OFFSET_PX).abs() < 0.5,
        "{:?} {mid:?}",
        w.at
    );
    // Dragged 5 mm back out: every corner takes the radius whose arc
    // lies 2 mm in.
    let dir = (centre - w.at).normalized();
    drag(
        &ctx,
        &mut app,
        w.at,
        w.at - dir * (5.0 * zoom),
        Modifiers::NONE,
        Vec::new(),
    );
    let twice = path_of(&app);
    let found = fillets(&twice);
    assert_eq!(found.len(), 4, "{found:?}");
    for f in &found {
        assert!((f.radius - r_of(2.0)).abs() < 0.05, "{}", f.radius);
    }
    let b = twice.bounding_box();
    assert!(
        (b.width() - 40.0).abs() < 1e-6,
        "the sides keep their place"
    );
    // One undo step back to the first rounding.
    app.undo();
    assert_eq!(path_of(&app), once);
    // Dragged back out past the corner: sharp again.
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let w = app.corner_widgets()[0];
    let dir = (centre - w.at).normalized();
    drag(
        &ctx,
        &mut app,
        w.at,
        w.at - dir * (10.0 * zoom),
        Modifiers::NONE,
        Vec::new(),
    );
    let sharp = path_of(&app);
    assert!(fillets(&sharp).is_empty());
    assert!(
        (sharp.area().abs() - 1600.0).abs() < 1e-6,
        "{}",
        sharp.area()
    );
    app.undo();
    assert_eq!(path_of(&app), once);
    // An arc's node chosen: only its corner has a widget, and only it
    // changes.
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let target = fillets(&once)[2].clone();
    app.node_selection = vec![(id, target.nodes[0])];
    let widgets = app.corner_widgets();
    assert_eq!(widgets.len(), 1, "{widgets:?}");
    let w = widgets[0];
    assert!((w.corner.at - target.corner).hypot() < 1e-6);
    let dir = (centre - w.at).normalized();
    drag(
        &ctx,
        &mut app,
        w.at,
        w.at - dir * (5.0 * zoom),
        Modifiers::NONE,
        Vec::new(),
    );
    let found = fillets(&path_of(&app));
    assert_eq!(found.len(), 4, "{found:?}");
    let changed: Vec<_> = found
        .iter()
        .filter(|f| (f.radius - r_of(7.0)).abs() > 0.05)
        .collect();
    assert_eq!(changed.len(), 1, "{found:?}");
    assert!((changed[0].corner - target.corner).hypot() < 1e-6);
    assert!(
        (changed[0].radius - r_of(2.0)).abs() < 0.05,
        "{}",
        changed[0].radius
    );
}

/// Every shape of one frame, nested ones taken out of their lists.
fn frame_shapes(ctx: &egui::Context, app: &mut App, events: Vec<Event>) -> Vec<egui::Shape> {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
    out.textures_delta.clear();
    fn walk(s: egui::Shape, all: &mut Vec<egui::Shape>) {
        match s {
            egui::Shape::Vec(v) => v.into_iter().for_each(|s| walk(s, all)),
            s => all.push(s),
        }
    }
    let mut all = Vec::new();
    out.shapes.into_iter().for_each(|c| walk(c.shape, &mut all));
    all
}

/// Overlays show on any colour (#670): a rectangle being drawn has a
/// black line on a white one over its outline, the selection handles
/// are black with a white edge, and a node's control handle is dashed on
/// a white line with a white-edged head.
#[test]
fn overlays_are_drawn_on_a_white_line() {
    let (ctx, mut app, id) = setup();
    // The selection handles: black boxes with a white edge.
    let shapes = frame_shapes(&ctx, &mut app, Vec::new());
    let handles = shapes
        .iter()
        .filter(|s| {
            matches!(s, egui::Shape::Rect(r) if r.fill == egui::Color32::BLACK
                && r.stroke.color == egui::Color32::WHITE && r.stroke.width >= 1.0)
        })
        .count();
    assert!(handles >= 8, "{handles}");
    // A rectangle being drawn.
    app.select(Vec::new());
    app.set_tool(Tool::Rectangle);
    let a = app.view.to_screen(Point::new(80.0, 80.0));
    let b = app.view.to_screen(Point::new(120.0, 110.0));
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(a)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(a, true, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    for i in 1..=3 {
        let p = a + (b - a) * (i as f32 / 3.0);
        frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(p)],
            Modifiers::NONE,
        );
    }
    let shapes = frame_shapes(&ctx, &mut app, Vec::new());
    let line = |color: egui::Color32, width: f32| {
        shapes.iter().position(|s| {
            matches!(s, egui::Shape::LineSegment { points, stroke }
                if stroke.color == color && (stroke.width - width).abs() < 1e-3
                    && points.iter().all(|p| (p.x - b.x).abs() < 2.0 || (p.x - a.x).abs() < 2.0
                        || (p.y - b.y).abs() < 2.0 || (p.y - a.y).abs() < 2.0))
        })
    };
    let white = line(egui::Color32::WHITE, crate::canvas::HALO_WIDTH).expect("a white line");
    let black = line(egui::Color32::BLACK, crate::canvas::HALO_INNER).expect("a black line");
    assert!(black > white, "the black line over the white one");
    frame(
        &ctx,
        &mut app,
        vec![button(b, false, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    // A curve's control handle with the Shape tool.
    use tracedraw_core::geometry::BezPath;
    let mut p = BezPath::new();
    p.move_to((20.0, 120.0));
    p.curve_to((40.0, 150.0), (70.0, 150.0), (90.0, 120.0));
    let curve = app
        .new_shape(ShapeKind::Path {
            path: p,
            closed: false,
        })
        .expect("curve");
    let _ = id;
    app.select(vec![curve]);
    app.set_tool(Tool::Shape);
    app.select_all_nodes();
    let shapes = frame_shapes(&ctx, &mut app, Vec::new());
    let heads = shapes
        .iter()
        .filter(|s| {
            matches!(s, egui::Shape::Path(ps) if ps.closed && ps.points.len() == 3
                && ps.stroke.color == egui::epaint::ColorMode::Solid(egui::Color32::WHITE))
        })
        .count();
    assert!(heads >= 2, "{heads}");
    assert!(shapes.iter().any(|s| matches!(s, egui::Shape::LineSegment { stroke, .. }
        if stroke.color == egui::Color32::WHITE && (stroke.width - crate::canvas::HALO_WIDTH).abs() < 1e-3)));
}

/// An idle window asks for no frame (#616): once the start has settled,
/// frames with no input request no repaint, with a drawing open (and its
/// dockers) and on the Welcome Screen, the window's per-frame work (the
/// preferences autosave, linked bitmaps) included. A frame asked for
/// here keeps a slow computer drawing for nothing.
#[test]
fn an_idle_window_asks_for_no_frame() {
    for welcome in [false, true] {
        let (ctx, mut app, _) = setup();
        app.show_welcome = welcome;
        let mut delays = Vec::new();
        for i in 0..40 {
            // Time passes between the frames: animations end, and a
            // preferences look or write that had to wait is due.
            let back = |t: Option<web_time::Instant>| {
                t.and_then(|t| t.checked_sub(std::time::Duration::from_secs(2)))
            };
            app.settings_looked = back(app.settings_looked);
            if app.settings_pending {
                app.settings_written = back(app.settings_written);
            }
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(1600.0, 1000.0),
                )),
                time: Some(10.0 + i as f64 * 2.0),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                crate::ui::root(&mut app, ui);
                app.autosave_settings(Some(ui.ctx()));
                app.watch_linked_bitmaps(Some(ui.ctx()), std::time::Instant::now());
            });
            out.textures_delta.clear();
            let delay = out
                .viewport_output
                .get(&egui::ViewportId::ROOT)
                .map(|v| v.repaint_delay)
                .unwrap_or(std::time::Duration::MAX);
            delays.push(delay);
        }
        let tail = &delays[delays.len() - 10..];
        assert!(
            tail.iter().all(|d| *d == std::time::Duration::MAX),
            "welcome {welcome}: frames asked for while idle: {delays:?} ({:?})",
            ctx.repaint_causes()
        );
    }
}

/// What a frame costs with the Text tool's font list on the property
/// bar, closed, on a computer with many fonts (#616: 1332 families):
/// `cargo test --release -p tracedraw -- --ignored frame_times_with_many_fonts --nocapture`.
#[test]
#[ignore]
fn frame_times_with_many_fonts() {
    for count in [1332, 10, 1332, 10] {
        let (ctx, mut app, _) = setup();
        app.font_families = (0..count).map(|i| format!("Font Family {i:04}")).collect();
        app.set_tool(Tool::Text);
        for _ in 0..3 {
            frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
        }
        let n = 200;
        let t = std::time::Instant::now();
        for i in 0..n {
            let p = Pos2::new(600.0 + (i % 50) as f32 * 4.0, 500.0);
            frame(
                &ctx,
                &mut app,
                vec![Event::PointerMoved(p)],
                Modifiers::NONE,
            );
        }
        eprintln!(
            "{count} families, hover with the font list on the bar: {:?} per frame",
            t.elapsed() / n
        );
    }
}

/// The Welcome Screen tab closes to its house while a drawing is open;
/// the house shows the Welcome Screen again with its whole tab (#677).
#[test]
fn the_welcome_tab_closes_to_its_house() {
    use crate::ui::tabs::{WELCOME_CLOSE, WELCOME_TAB};
    let (ctx, mut app, _) = setup();
    let rect_of =
        |ctx: &egui::Context, id: &str| ctx.read_response(egui::Id::new(id)).map(|r| r.rect);
    let press = |ctx: &egui::Context, app: &mut App, at: Pos2| {
        frame(ctx, app, vec![Event::PointerMoved(at)], Modifiers::NONE);
        click_at(ctx, app, at, Modifiers::NONE);
        frame(ctx, app, Vec::new(), Modifiers::NONE);
    };
    app.show_welcome = true;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let wide = rect_of(&ctx, WELCOME_TAB).expect("welcome tab");
    let close = rect_of(&ctx, WELCOME_CLOSE).expect("close button");
    press(&ctx, &mut app, close.center());
    assert!(!app.show_welcome, "the drawing shows");
    assert!(app.settings.welcome_tab_closed);
    let house = rect_of(&ctx, WELCOME_TAB).expect("house");
    assert!(house.width() < wide.width() / 2.0, "{house:?} {wide:?}");
    assert!(rect_of(&ctx, WELCOME_CLOSE).is_none());
    press(&ctx, &mut app, house.center());
    assert!(app.show_welcome);
    assert!(!app.settings.welcome_tab_closed);
    let back = rect_of(&ctx, WELCOME_TAB).expect("welcome tab");
    assert!(
        (back.width() - wide.width()).abs() < 0.5,
        "{back:?} {wide:?}"
    );
    // With no drawing open there is nothing else to show: no close button.
    let ctx = crate::theme::ui_context();
    let mut app = App::headless();
    app.close_active_document();
    assert!(!app.has_document());
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert!(rect_of(&ctx, WELCOME_TAB).is_some());
    assert!(rect_of(&ctx, WELCOME_CLOSE).is_none());
}

#[test]
fn the_export_dialog_starts_with_the_last_exports_choices() {
    use crate::ui::dialogs::Dialog;
    let (ctx, mut app, _) = setup();
    let ctrl_e = |ctx: &egui::Context, app: &mut App| {
        app.dialog = Dialog::None;
        frame(
            ctx,
            app,
            vec![key(egui::Key::E, Modifiers::COMMAND)],
            Modifiers::COMMAND,
        );
        match &app.dialog {
            Dialog::Export(st) => st.clone(),
            other => panic!("no Export dialog: {other:?}"),
        }
    };
    let first = ctrl_e(&ctx, &mut app);
    assert!(first.remember);
    assert!(app.settings.formats.remember_last, "on at first");
    // A JPEG at quality 100 and 150 dpi exported...
    let mut st = first.clone();
    st.format = 3;
    st.quality = 100;
    st.dpi = 150.0;
    st.title = "Poster".into();
    st.selection_only = true;
    st.all_pages = true;
    app.settings.formats.remember_last = false;
    app.keep_export_choices(&st);
    // ...is what the next export starts with, the file's title and
    // Selected only left out.
    let next = ctrl_e(&ctx, &mut app);
    assert_eq!((next.format, next.quality, next.dpi), (3, 100, 150.0));
    assert!(next.title.is_empty());
    assert!(!next.selection_only, "Selected only is for one export");
    assert!(next.all_pages);
    // "Do not show filter dialog" goes straight to the file with the same
    // starting choices, so it never exports a selection that is gone.
    assert_eq!(
        app.last_export.as_ref().map(|e| e.selection_only),
        Some(false)
    );
    // With Remember off the next session still starts as the options say.
    assert_ne!(app.settings.formats.export_quality, 100);
    app.settings.formats.remember_last = true;
    app.keep_export_choices(&next);
    assert_eq!(app.settings.formats.export_quality, 100);
    assert_eq!(app.settings.formats.state().format, 3);
    // Export for Web keeps nothing.
    let mut web = crate::ui::dialogs::ExportState::web();
    web.quality = 10;
    app.keep_export_choices(&web);
    assert_eq!(ctrl_e(&ctx, &mut app).quality, 100);
    app.dialog = Dialog::None;
}

/// Drawing a rectangle or an ellipse with a wide black default outline:
/// the outline is previewed at its width and the edge line (white with
/// a black core) is drawn after it, on top, so the preview is not lost
/// under the outline (#621).
#[test]
fn the_edge_of_the_shape_being_drawn_shows_over_a_wide_outline() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.set_default_properties(tracedraw_core::DefaultKind::Graphic, |p| {
        p.stroke = Some(tracedraw_core::Stroke::new(
            tracedraw_core::Color::BLACK,
            3.0,
        ));
    });
    let wide = 3.0 * app.view.zoom;
    for (tool, y) in [(Tool::Rectangle, 80.0), (Tool::Ellipse, 140.0)] {
        let a = app.view.to_screen(Point::new(80.0, y));
        let b = app.view.to_screen(Point::new(140.0, y + 40.0));
        app.set_tool(tool);
        frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(a)],
            Modifiers::NONE,
        );
        frame(
            &ctx,
            &mut app,
            vec![button(a, true, PointerButton::Primary, Modifiers::NONE)],
            Modifiers::NONE,
        );
        frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(a + (b - a) * 0.5)],
            Modifiers::NONE,
        );
        let shapes = frame_shapes(&ctx, &mut app, vec![Event::PointerMoved(b)]);
        let outline = shapes.iter().position(|s| {
            matches!(s, egui::Shape::Path(p) if p.stroke.width >= wide - 0.01
                && p.stroke.color == egui::epaint::ColorMode::Solid(egui::Color32::BLACK))
        });
        // The white line under the edge: segments for a short outline,
        // a path for a long one such as an ellipse's.
        let halo = |stroke: &egui::Stroke| {
            stroke.color == egui::Color32::WHITE
                && (stroke.width - crate::canvas::HALO_WIDTH).abs() < 1e-3
        };
        let edge = shapes.iter().rposition(|s| match s {
            egui::Shape::LineSegment { stroke, .. } => halo(stroke),
            egui::Shape::Path(p) => {
                (p.stroke.width - crate::canvas::HALO_WIDTH).abs() < 1e-3
                    && p.stroke.color == egui::epaint::ColorMode::Solid(egui::Color32::WHITE)
            }
            _ => false,
        });
        assert!(
            matches!((outline, edge), (Some(o), Some(e)) if e > o),
            "{tool:?}: outline {outline:?}, edge {edge:?}"
        );
        frame(
            &ctx,
            &mut app,
            vec![button(b, false, PointerButton::Primary, Modifiers::NONE)],
            Modifiers::NONE,
        );
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
}

#[test]
fn the_export_dialog_lists_only_the_presets_of_its_format() {
    use crate::ui::dialogs::{presets_for, Dialog, ExportState};
    let (ctx, mut app, _) = setup();
    let f = &mut app.settings.formats;
    for (name, format) in [("Web PNG", 2), ("Print JPG", 3), ("Big PNG", 2)] {
        let st = ExportState {
            format,
            ..ExportState::default()
        };
        assert!(f.save_preset(name, &st));
    }
    let names = |st: &ExportState, app: &App| {
        presets_for(&app.settings.formats.export_presets, st)
            .into_iter()
            .map(|p| p.name)
            .collect::<Vec<_>>()
    };
    app.export();
    // A PNG preset chosen, then the format changed to JPEG: the list holds
    // only the JPEG preset and the PNG one is no longer chosen.
    let Dialog::Export(st) = &mut app.dialog else {
        panic!("no Export dialog");
    };
    st.format = 2;
    st.preset = "Web PNG".into();
    let png = st.clone();
    assert_eq!(names(&png, &app), vec!["Web PNG", "Big PNG"]);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let Dialog::Export(st) = &mut app.dialog else {
        panic!("no Export dialog");
    };
    assert_eq!(st.preset, "Web PNG", "a preset of the format stays chosen");
    st.format = 3;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let Dialog::Export(st) = &app.dialog else {
        panic!("no Export dialog");
    };
    assert!(st.preset.is_empty(), "{:?}", st.preset);
    let jpg = st.clone();
    assert_eq!(names(&jpg, &app), vec!["Print JPG"]);
    // Saving a PNG preset under the hidden JPEG preset's name keeps the
    // JPEG one: each format has its own names.
    assert!(app.settings.formats.save_preset("Print JPG", &png));
    assert_eq!(names(&jpg, &app), vec!["Print JPG"]);
    assert_eq!(names(&png, &app), vec!["Web PNG", "Big PNG", "Print JPG"]);
    assert_eq!(app.settings.formats.export_presets.len(), 4);
    // An installed script filter chosen: no presets, and nothing typed
    // for "+" is saved under the last built-in format, out of sight.
    app.settings
        .script_filters
        .push(crate::scripting::ScriptFilter {
            name: "Plotter".into(),
            extensions: vec!["plt".into()],
            import: false,
            export: true,
            source: String::new(),
        });
    let before = app.settings.formats.export_presets.clone();
    let Dialog::Export(st) = &mut app.dialog else {
        panic!("no Export dialog");
    };
    st.format = 2;
    st.script = Some("Plotter".into());
    st.preset_name = Some(("Web PNG".into(), false));
    let scripted = st.clone();
    assert!(names(&scripted, &app).is_empty());
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let Dialog::Export(st) = &app.dialog else {
        panic!("no Export dialog");
    };
    assert_eq!(st.script.as_deref(), Some("Plotter"));
    assert!(st.preset_name.is_none());
    assert_eq!(app.settings.formats.export_presets, before);
}

/// A double click on a dimension's label with the Pick tool edits it in
/// place; what is typed stays, and the dimension stays a dimension with
/// the typed text as its suffix (#644).
#[test]
fn a_double_click_edits_a_dimension_label_in_place() {
    let (ctx, mut app, rect) = setup();
    app.run(tracedraw_core::Command::DeleteShapes { shapes: vec![rect] });
    app.create_dimension(
        Point::new(20.0, 60.0),
        Point::new(80.0, 60.0),
        Point::new(50.0, 80.0),
    );
    let g = app.selection[0];
    let label = match &app.doc().find_shape(g).expect("dimension").kind {
        ShapeKind::Group { children } => children[2].clone(),
        other => panic!("{other:?}"),
    };
    app.set_tool(Tool::Pick);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let at = app.view.to_screen(label.bounds().center());
    double_click_at(&ctx, &mut app, at);
    assert_eq!(app.tool, Tool::Text);
    assert_eq!(app.text_edit.as_ref().map(|te| te.shape), Some(label.id));
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::End, Modifiers::NONE)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![Event::Text(" max".into())],
        Modifiers::NONE,
    );
    app.bake_dimensions();
    let d = crate::dimensions::Dimension::of(app.doc().find_shape(g).expect("dimension"))
        .expect("still a dimension");
    assert_eq!(d.style.suffix, " max");
    assert_eq!(d.text, None);
    match &app.doc().find_shape(label.id).expect("label").kind {
        ShapeKind::Text { spans, .. } => {
            assert_eq!(tracedraw_core::document::spans_text(spans), "60.00 mm max")
        }
        other => panic!("{other:?}"),
    }
    // A Text tool click on the label edits it too, no new text made.
    app.finish_text();
    let count = shape_count(&app);
    let at = app.view.to_screen(
        app.doc()
            .find_shape(label.id)
            .expect("label")
            .bounds()
            .center(),
    );
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert_eq!(app.text_edit.as_ref().map(|te| te.shape), Some(label.id));
    assert_eq!(shape_count(&app), count);
}

/// The dimension bar's Options menu stays open while a field in it is
/// clicked, typed in and confirmed with Enter, and the value applies to
/// the selected dimension (#644).
#[test]
fn the_dimension_options_stay_open_while_typed_in() {
    let (ctx, mut app, rect) = setup();
    app.run(tracedraw_core::Command::DeleteShapes { shapes: vec![rect] });
    app.create_dimension(
        Point::new(20.0, 60.0),
        Point::new(80.0, 60.0),
        Point::new(50.0, 80.0),
    );
    let g = app.selection[0];
    app.set_tool(Tool::Pick);
    let menu = egui::Id::new("dimension_options_menu");
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    egui::Popup::open_id(&ctx, menu);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let area = ctx
        .memory(|m| m.area_rect(menu))
        .expect("the menu is shown");
    // The second row's field, Extension past the line, the widest row:
    // its text, left of the spin arrows.
    let at = Pos2::new(area.max.x - 50.0, area.min.y + 34.0);
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert!(egui::Popup::is_id_open(&ctx, menu), "open after the click");
    frame(
        &ctx,
        &mut app,
        vec![
            key(egui::Key::A, Modifiers::COMMAND),
            Event::Text("3".into()),
            key(egui::Key::Enter, Modifiers::NONE),
        ],
        Modifiers::NONE,
    );
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert!(egui::Popup::is_id_open(&ctx, menu), "open after typing");
    let d = crate::dimensions::Dimension::of(app.doc().find_shape(g).expect("dimension"))
        .expect("dimension");
    assert!(
        (d.style.extension_overshoot - 3.0).abs() < 1e-9,
        "{}",
        d.style.extension_overshoot
    );
}

/// A curve whose second segment runs straight along the middle node's
/// outgoing handle, so that handle lies on the segment, selected for the
/// Shape tool.
fn curve_with_handle_on_segment() -> (egui::Context, App, ShapeId) {
    use tracedraw_core::geometry::BezPath;
    let (ctx, mut app, first) = setup();
    app.select(vec![first]);
    app.delete_selection();
    let mut path = BezPath::new();
    path.move_to((100.0, 150.0));
    path.curve_to((110.0, 150.0), (130.0, 170.0), (140.0, 170.0));
    path.curve_to((150.0, 170.0), (170.0, 170.0), (180.0, 170.0));
    let id = app
        .new_shape(ShapeKind::Path {
            path,
            closed: false,
        })
        .expect("curve");
    app.select(vec![id]);
    app.set_tool(Tool::Shape);
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    (ctx, app, id)
}

/// Each node of curve `id` with its handles.
fn node_spots(app: &App, id: ShapeId) -> Vec<(Point, Option<Point>, Option<Point>)> {
    let ShapeKind::Path { path, .. } = &app.doc().find_shape(id).expect("curve").kind else {
        panic!("a curve");
    };
    tracedraw_core::nodes::nodes(path)
        .iter()
        .map(|n| (n.pos, n.ctrl_in, n.ctrl_out))
        .collect()
}

/// Shape tool: a click on a chosen node's handle keeps the node chosen
/// (it chose the segment under the handle), and dragging the handle then
/// changes the handle, the node staying where it is (#639).
#[test]
fn a_handle_clicked_then_dragged_leaves_its_node_in_place() {
    let (ctx, mut app, id) = curve_with_handle_on_segment();
    click_page(&ctx, &mut app, Point::new(140.0, 170.0));
    assert_eq!(app.node_selection, vec![(id, 1)]);
    let handle = Point::new(150.0, 170.0);
    click_page(&ctx, &mut app, handle);
    assert_eq!(app.node_selection, vec![(id, 1)], "the node stays chosen");
    pull(&ctx, &mut app, handle, Point::new(154.0, 180.0));
    let n = node_spots(&app, id);
    assert!(
        near(n[1].0, Point::new(140.0, 170.0)),
        "the node stays: {n:?}"
    );
    assert!(near(n[2].0, Point::new(180.0, 170.0)), "{n:?}");
    assert!(
        near(n[1].2.expect("out"), Point::new(154.0, 180.0)),
        "{n:?}"
    );
    // A smooth node's other handle follows, on the far side.
    let back = n[1].1.expect("in") - n[1].0;
    assert!(back.x < 0.0 && back.y < 0.0, "{n:?}");
}

/// Shape tool: a node whose handle is retracted onto it still takes
/// clicks once chosen: Ctrl+click drops it from the chosen nodes and a
/// plain click chooses it alone; the handle never swallows them (#639).
#[test]
fn a_click_on_a_chosen_node_with_a_retracted_handle_takes_the_node() {
    use tracedraw_core::geometry::BezPath;
    let (ctx, mut app, first) = setup();
    app.select(vec![first]);
    app.delete_selection();
    let mut path = BezPath::new();
    path.move_to((100.0, 150.0));
    path.curve_to((110.0, 150.0), (130.0, 170.0), (140.0, 170.0));
    path.curve_to((140.0, 170.0), (170.0, 160.0), (180.0, 150.0));
    let id = app
        .new_shape(ShapeKind::Path {
            path,
            closed: false,
        })
        .expect("curve");
    app.select(vec![id]);
    app.set_tool(Tool::Shape);
    app.node_selection = vec![(id, 0), (id, 1), (id, 2)];
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let at = app.view.to_screen(Point::new(140.0, 170.0));
    click_at(&ctx, &mut app, at, Modifiers::COMMAND | Modifiers::CTRL);
    assert_eq!(app.node_selection, vec![(id, 0), (id, 2)], "dropped");
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert_eq!(app.node_selection, vec![(id, 1)], "chosen alone");
}

/// Shape tool: the two handles a chosen segment shows can be dragged;
/// they change the segment's curve and no node moves (#639).
#[test]
fn a_chosen_segments_handles_drag() {
    let (ctx, mut app, id) = curve_with_handle_on_segment();
    // The first segment, chosen by a click on it.
    let on = tracedraw_core::geometry::CubicBez::new(
        (100.0, 150.0),
        (110.0, 150.0),
        (130.0, 170.0),
        (140.0, 170.0),
    );
    use tracedraw_core::geometry::ParamCurve;
    click_page(&ctx, &mut app, on.eval(0.5));
    assert_eq!(app.chosen_segment(), Some((id, 1)));
    pull(
        &ctx,
        &mut app,
        Point::new(110.0, 150.0),
        Point::new(112.0, 140.0),
    );
    let n = node_spots(&app, id);
    assert!(
        near(n[0].2.expect("out"), Point::new(112.0, 140.0)),
        "{n:?}"
    );
    pull(
        &ctx,
        &mut app,
        Point::new(130.0, 170.0),
        Point::new(128.0, 180.0),
    );
    let n = node_spots(&app, id);
    assert!(near(n[1].1.expect("in"), Point::new(128.0, 180.0)), "{n:?}");
    let at: Vec<Point> = n.iter().map(|s| s.0).collect();
    assert!(near(at[0], Point::new(100.0, 150.0)), "{at:?}");
    assert!(near(at[1], Point::new(140.0, 170.0)), "{at:?}");
    assert!(near(at[2], Point::new(180.0, 170.0)), "{at:?}");
}

/// The property bar's wrap style list starts with None, as the reference
/// program's does: it is marked while the selection does not wrap text,
/// and a click on it stops the selection wrapping (#661).
#[test]
fn the_wrap_style_list_starts_with_none() {
    let (ctx, mut app, id) = setup();
    app.set_wrap_style(
        tracedraw_core::WrapMode::Square,
        tracedraw_core::WrapSide::Left,
    );
    assert!(app.wrap_state().0);
    let texts = |app: &mut App| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
        let mut all: Vec<(String, Pos2)> = Vec::new();
        fn walk(s: &egui::Shape, all: &mut Vec<(String, Pos2)>) {
            match s {
                egui::Shape::Text(t) => all.push((
                    t.galley.text().to_string(),
                    t.pos + t.galley.rect.center().to_vec2(),
                )),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, all)),
                _ => {}
            }
        }
        out.shapes.iter().for_each(|c| walk(&c.shape, &mut all));
        all
    };
    // The small arrows of the property bar: the one that opens the wrap
    // styles.
    let arrows: Vec<Pos2> = texts(&mut app)
        .into_iter()
        .filter(|(t, p)| t == "\u{25BE}" && p.y < 140.0)
        .map(|(_, p)| p)
        .collect();
    let mut opened = None;
    for a in arrows {
        click_at(&ctx, &mut app, a, Modifiers::NONE);
        let now = texts(&mut app);
        if now.iter().any(|(t, _)| t == "Text flows left") {
            opened = now.into_iter().find(|(t, _)| t == "None").map(|(_, p)| p);
            break;
        }
        frame(
            &ctx,
            &mut app,
            vec![key(egui::Key::Escape, Modifiers::NONE)],
            Modifiers::NONE,
        );
    }
    let none = opened.expect("the wrap style list with None");
    let shown: Vec<String> = texts(&mut app).into_iter().map(|(t, _)| t).collect();
    for heading in ["Wrap style", "Contour", "Square", "Above/below"] {
        assert!(shown.iter().any(|t| t == heading), "{heading}: {shown:?}");
    }
    click_at(&ctx, &mut app, none, Modifiers::NONE);
    assert!(!app.wrap_state().0, "wrapping is off");
    assert!(!app.doc().find_shape(id).expect("rect").wrap_text);
    // The offset is greyed out under None: typing there does not turn
    // wrapping back on.
    let label = texts(&mut app)
        .into_iter()
        .find(|(t, _)| t == "Text wrap offset:")
        .map(|(_, p)| p)
        .expect("the offset field, still in the open list");
    let field = label + egui::vec2(90.0, 0.0);
    click_at(&ctx, &mut app, field, Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        vec![
            Event::Text("7".into()),
            key(egui::Key::Enter, Modifiers::NONE),
        ],
        Modifiers::NONE,
    );
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert!(!app.wrap_state().0, "still off");
}

/// A print to a slow printer goes on off the interface's thread: frames
/// keep coming while it works, and the status bar says how it ended
/// (#810).
#[test]
fn a_slow_printer_leaves_the_window_usable() {
    let (ctx, mut app, _) = setup();
    app.start_print_job(|| {
        std::thread::sleep(std::time::Duration::from_millis(400));
        Err("the spooler gave up".into())
    });
    app.status = "Printing on Oce TDS600…".into();
    let t = std::time::Instant::now();
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert!(
        t.elapsed() < std::time::Duration::from_millis(300),
        "{:?}",
        t.elapsed()
    );
    assert!(app.print_job.is_some(), "still printing");
    assert!(app.status.starts_with("Printing"), "{}", app.status);
    let t = std::time::Instant::now();
    while app.print_job.is_some() && t.elapsed() < std::time::Duration::from_secs(10) {
        std::thread::sleep(std::time::Duration::from_millis(20));
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    assert!(app.print_job.is_none());
    assert!(
        app.status.ends_with("the spooler gave up"),
        "{}",
        app.status
    );
}

/// File > Print reads the printers off the interface's thread (#614): the
/// dialog opens at once and says it is looking; then the list comes in
/// name order with the default chosen where it stands, and the General
/// tab shows the chosen printer's status, location and comment, next to a
/// Properties button.
#[test]
fn the_print_dialog_reads_the_printers_off_the_interface_thread() {
    use crate::printers::{Printer, PrinterState};
    use crate::ui::dialogs::Dialog;
    fn slow_list() -> Vec<Printer> {
        std::thread::sleep(std::time::Duration::from_millis(300));
        vec![
            Printer {
                name: "Oce TDS600".into(),
                ..Default::default()
            },
            Printer {
                name: "3-102. KONICA MINOLTA C220".into(),
                default: true,
                state: PrinterState::Ready,
                location: "Room 3-102".into(),
                comment: "Colour, A3".into(),
            },
            Printer {
                name: "Generic PDF".into(),
                ..Default::default()
            },
        ]
    }
    fn texts(ctx: &egui::Context, app: &mut App) -> Vec<String> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
        fn walk(s: &egui::Shape, texts: &mut Vec<String>) {
            match s {
                egui::Shape::Text(t) => texts.push(t.galley.text().to_string()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, texts)),
                _ => {}
            }
        }
        let mut texts = Vec::new();
        out.shapes.iter().for_each(|c| walk(&c.shape, &mut texts));
        texts
    }
    let (ctx, mut app, _) = setup();
    app.printers_seen.clear();
    app.print();
    app.fetch_printers(slow_list);
    texts(&ctx, &mut app);
    let state = |app: &App| match &app.dialog {
        Dialog::Print(st) => st.clone(),
        other => panic!("no Print dialog: {other:?}"),
    };
    assert!(state(&app).looking_for_printers, "the dialog does not wait");
    let mut shown = Vec::new();
    for _ in 0..300 {
        shown = texts(&ctx, &mut app);
        if !state(&app).looking_for_printers {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let st = state(&app);
    assert!(!st.looking_for_printers);
    let names: Vec<&str> = st.printers.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        ["3-102. KONICA MINOLTA C220", "Generic PDF", "Oce TDS600"]
    );
    assert_eq!(st.printer.as_deref(), Some("3-102. KONICA MINOLTA C220"));
    shown.extend(texts(&ctx, &mut app));
    for t in [
        crate::i18n::tr("print.status"),
        crate::i18n::tr("print.status_ready"),
        crate::i18n::tr("print.location"),
        "Room 3-102".to_string(),
        crate::i18n::tr("print.comment"),
        "Colour, A3".to_string(),
    ] {
        assert!(shown.contains(&t), "{t} not shown: {shown:?}");
    }
    // The next Print starts from the list read last.
    app.dialog = Dialog::None;
    app.print();
    assert_eq!(state(&app).printers.len(), 3);
    assert_eq!(
        state(&app).printer.as_deref(),
        Some("3-102. KONICA MINOLTA C220")
    );
}

/// A destination chosen in the Print dialog while the printer list is
/// still being read stays when the list comes (#614): the PDF viewer
/// picked before the list stays the PDF viewer, and a printer picked from
/// the list read last stays chosen though the system's default differs.
#[test]
fn a_destination_picked_while_the_printers_are_read_stays() {
    use crate::printers::Printer;
    use crate::ui::dialogs::Dialog;
    fn slow_list() -> Vec<Printer> {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        ["Office", "Plotter", "Studio"]
            .iter()
            .map(|n| Printer {
                name: n.to_string(),
                default: *n == "Studio",
                ..Default::default()
            })
            .collect()
    }
    /// The centre of the last text `label` drawn in one frame.
    fn last_text(ctx: &egui::Context, app: &mut App, label: &str) -> Option<Pos2> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
        fn walk(s: &egui::Shape, label: &str, at: &mut Option<Pos2>) {
            match s {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    *at = Some(t.galley.rect.translate(t.pos.to_vec2()).center())
                }
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, label, at)),
                _ => {}
            }
        }
        let mut at = None;
        out.shapes
            .iter()
            .for_each(|c| walk(&c.shape, label, &mut at));
        at
    }
    let state = |app: &App| match &app.dialog {
        Dialog::Print(st) => st.clone(),
        other => panic!("no Print dialog: {other:?}"),
    };
    let wait = |ctx: &egui::Context, app: &mut App| {
        for _ in 0..400 {
            frame(ctx, app, Vec::new(), Modifiers::NONE);
            if !state(app).looking_for_printers {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("the list never came");
    };
    // The first time: nothing listed yet, the viewer picked.
    let (ctx, mut app, _) = setup();
    app.printers_seen.clear();
    app.print();
    app.fetch_printers(slow_list);
    // The dialog takes its place over the first frames.
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let viewer = crate::i18n::tr("print.dest_viewer");
    let combo = last_text(&ctx, &mut app, &viewer).expect("the destination list");
    click_at(&ctx, &mut app, combo, Modifiers::NONE);
    let item = last_text(&ctx, &mut app, &viewer).expect("the viewer in the list");
    assert!(
        state(&app).looking_for_printers,
        "picked before the list came"
    );
    click_at(&ctx, &mut app, item, Modifiers::NONE);
    wait(&ctx, &mut app);
    let st = state(&app);
    assert_eq!((st.printer.as_deref(), st.destination), (None, 0));
    assert_eq!(st.printers.len(), 3);
    // Again, a printer of the list read last picked before the new one
    // comes: it stays, not the system's default.
    app.dialog = Dialog::None;
    app.print();
    app.fetch_printers(slow_list);
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    assert_eq!(state(&app).printer.as_deref(), Some("Studio"));
    let combo = last_text(&ctx, &mut app, "Studio").expect("the destination list");
    click_at(&ctx, &mut app, combo, Modifiers::NONE);
    let item = last_text(&ctx, &mut app, "Office").expect("Office in the list");
    click_at(&ctx, &mut app, item, Modifiers::NONE);
    wait(&ctx, &mut app);
    assert_eq!(state(&app).printer.as_deref(), Some("Office"));
}

/// 204 wavy rings of `nodes` nodes each, combined into one curve and
/// selected, as the ornamental frame of #764.
fn combined_ornament(nodes: usize) -> (egui::Context, App, ShapeId) {
    let (ctx, mut app, id) = setup();
    app.select(vec![id]);
    app.delete_selection();
    let mut ids = Vec::new();
    for i in 0..204 {
        let a = i as f64 / 204.0 * std::f64::consts::TAU;
        let c = Point::new(100.0 + 60.0 * a.cos(), 150.0 + 60.0 * a.sin());
        let mut path = tracedraw_core::geometry::BezPath::new();
        for k in 0..nodes {
            let b = k as f64 / nodes as f64 * std::f64::consts::TAU;
            let r = 12.0 + 2.0 * (b * 12.0).sin();
            let q = Point::new(c.x + r * b.cos(), c.y + r * b.sin());
            if k == 0 {
                path.move_to(q);
            } else {
                path.line_to(q);
            }
        }
        path.close_path();
        ids.push(
            app.new_shape(ShapeKind::Path { path, closed: true })
                .expect("ring"),
        );
    }
    app.select(ids);
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
        40, 40, 40,
    )));
    app.combine();
    assert_eq!(app.selection.len(), 1);
    let c = app.selection[0];
    for _ in 0..2 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    (ctx, app, c)
}

/// A combined curve of thousands of nodes moves with a drag, and the
/// node under the pointer is still marked for snapping: the snap points
/// near the pointer no longer compare every node with every other, which
/// took seconds a frame and looked as if the curve would not move (#764).
#[test]
fn a_combined_curve_of_many_nodes_moves_and_snaps_to_its_nodes() {
    let (ctx, mut app, c) = combined_ornament(60);
    let before = app.doc().find_shape(c).map(|s| s.bounds()).expect("curve");
    // The first ring's rightmost node, at (100 + 60 + 12, 150).
    let node = Point::new(172.0, 150.0);
    let hover = app.selection_hover_target(node).expect("a snap point");
    assert_eq!(hover.mode, crate::snap_points::SnapMode::Node);
    assert!((hover.point - node).hypot() < 1e-9, "{:?}", hover.point);
    let from = app.view.to_screen(node);
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(40.0, 0.0),
        Modifiers::NONE,
        Vec::new(),
    );
    let after = app.doc().find_shape(c).map(|s| s.bounds()).expect("curve");
    let moved = after.x0 - before.x0;
    let want = 40.0 / app.view.zoom as f64;
    assert!((moved - want).abs() < want * 0.2, "{moved} {want}");
}

/// Frame times over a combined curve of about 40,000 nodes (#764):
/// hovering it and dragging it.
/// `cargo test --release -- --ignored frame_times_over_a_combined_curve --nocapture`.
#[test]
#[ignore]
fn frame_times_over_a_combined_curve() {
    let (ctx, mut app, _) = combined_ornament(200);
    let p0 = app.view.to_screen(Point::new(160.0, 150.0));
    let n = 20;
    let t = std::time::Instant::now();
    for k in 0..n {
        let p = p0 + egui::vec2((k % 5) as f32, 0.0);
        frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(p)],
            Modifiers::NONE,
        );
    }
    println!(
        "combined curve, hover: {:?} per frame",
        t.elapsed() / n as u32
    );
    frame(
        &ctx,
        &mut app,
        vec![button(p0, true, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let t = std::time::Instant::now();
    for k in 1..=n {
        let p = p0 + egui::vec2(2.0 * k as f32, 0.0);
        frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(p)],
            Modifiers::NONE,
        );
    }
    println!(
        "combined curve, drag: {:?} per frame",
        t.elapsed() / n as u32
    );
    frame(
        &ctx,
        &mut app,
        vec![button(p0, false, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
}

/// With hardware acceleration off, the status bar says that this is why
/// a software adapter draws the window, and a click on the note opens
/// Options at its General page; with it on, the note names the missing
/// card driver instead and opens nothing.
#[test]
fn the_software_renderer_note_says_hardware_acceleration_is_off() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.settings.hardware_acceleration = false;
    crate::ui::note_software_renderer(&mut app, "Microsoft Basic Render Driver");
    let off = crate::i18n::trf(
        "status.software_renderer_off",
        &[("name", "Microsoft Basic Render Driver")],
    );
    assert_eq!(app.status, off);
    let (_, texts) = painted(&ctx, &mut app);
    let at = texts
        .iter()
        .find(|(t, _)| t.starts_with(&off))
        .map(|(_, p)| *p)
        .expect("the note on the status bar");
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    assert!(matches!(app.dialog, crate::ui::dialogs::Dialog::Options));
    assert!(matches!(
        app.options_page,
        crate::ui::dialogs::OptionsPage::General
    ));
    assert!(app.status.is_empty());
    // Hardware acceleration on: the driver note, not a link.
    app.dialog = crate::ui::dialogs::Dialog::None;
    app.settings.hardware_acceleration = true;
    app.status_opens_options = None;
    crate::ui::note_software_renderer(&mut app, "Microsoft Basic Render Driver");
    assert!(app.status_opens_options.is_none());
    assert_ne!(app.status, off);
}

/// The arrow keys nudge the selected guidelines when no object is
/// selected: plain by the nudge distance, Shift by the super nudge, Ctrl
/// by the micro nudge, one undo step per press; locked ones stay (#781).
#[test]
fn arrow_keys_nudge_the_selected_guidelines() {
    use tracedraw_core::document::{Guide, GuideLine};
    let (ctx, mut app, _id) = setup();
    app.add_guide(Guide::horizontal(150.0));
    app.add_guide(Guide::vertical(120.0));
    app.add_guide(Guide::vertical(140.0));
    app.set_guides_locked(&[2], true);
    // A click on the horizontal guideline, away from the rectangle.
    let on_h = app.view.to_screen(Point::new(100.0, 150.0));
    click_at(&ctx, &mut app, on_h, Modifiers::NONE);
    assert!(app.selection.is_empty());
    assert_eq!(app.selected_guides, vec![0]);
    let y = |app: &App| match app.page_guides()[0].line {
        GuideLine::Horizontal { y } => y,
        other => panic!("{other:?}"),
    };
    let press = |app: &mut App, k: egui::Key, mods: Modifiers| {
        frame(&ctx, app, vec![key(k, mods)], mods);
        frame(&ctx, app, Vec::new(), Modifiers::NONE);
    };
    let (step, sup, micro) = (
        app.nudge_mm,
        app.settings.super_nudge_mm,
        app.settings.micro_nudge_mm,
    );
    press(&mut app, egui::Key::ArrowUp, Modifiers::NONE);
    assert!(close(y(&app), 150.0 + step), "{}", y(&app));
    press(&mut app, egui::Key::ArrowDown, Modifiers::SHIFT);
    assert!(close(y(&app), 150.0 + step - sup), "{}", y(&app));
    press(&mut app, egui::Key::ArrowUp, Modifiers::COMMAND);
    assert!(close(y(&app), 150.0 + step - sup + micro), "{}", y(&app));
    // A sideways nudge leaves a horizontal guideline where it is.
    press(&mut app, egui::Key::ArrowRight, Modifiers::NONE);
    assert!(close(y(&app), 150.0 + step - sup + micro), "{}", y(&app));
    // All of them: the unlocked vertical one moves, the locked one stays.
    app.select_all_guides();
    press(&mut app, egui::Key::ArrowLeft, Modifiers::NONE);
    let g = app.page_guides();
    assert_eq!(g[1].line, GuideLine::Vertical { x: 120.0 - step });
    assert_eq!(g[2].line, GuideLine::Vertical { x: 140.0 });
    // Each press is one undo step; the sideways one made none.
    let steps = app.engine.history_labels().0;
    assert_eq!(steps.last(), Some(&"Nudge Guidelines"));
    let n = steps.iter().filter(|l| **l == "Nudge Guidelines").count();
    assert_eq!(n, 4, "{steps:?}");
    app.undo();
    assert_eq!(app.page_guides()[1].line, GuideLine::Vertical { x: 120.0 });
    assert!(close(y(&app), 150.0 + step - sup + micro));
    app.undo();
    assert!(close(y(&app), 150.0 + step - sup), "{}", y(&app));
}

/// A drawing of 30 pages (a merged document): the page tabs that fit are
/// shown, a click on one goes to its page, Page Down and Page Up go to the
/// next and previous page, and the tabs slide so the current page's tab
/// shows; the mouse wheel over them slides them without changing the page.
/// Before, the tabs past the row's end were never shown, so their pages
/// could only be reached with Go To Page (#794).
#[test]
fn page_tabs_slide_to_the_current_page_of_a_long_drawing() {
    let (ctx, mut app, _) = setup();
    let rows: Vec<Vec<String>> = (1..=30).map(|i| vec![format!("N{i}")]).collect();
    app.merge_state = Some(crate::ui::dialogs::PrintMergeState {
        headers: vec!["Name".into()],
        rows,
        kinds: vec![crate::print_merge::FieldKind::Text],
        ..Default::default()
    });
    app.create_merged_document(false);
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    assert_eq!(app.doc().pages.len(), 30);
    let names: Vec<String> = app.doc().pages.iter().map(|p| p.name.clone()).collect();
    let tabs = |ctx: &egui::Context, app: &mut App| -> Vec<(String, Pos2)> {
        let (_, texts) = painted(ctx, app);
        texts
            .into_iter()
            .filter(|(t, _)| names.contains(t))
            .collect()
    };
    let shown = tabs(&ctx, &mut app);
    assert!(shown.len() < 30, "the row holds only some: {}", shown.len());
    // A click on the third tab.
    let third = shown
        .iter()
        .find(|(t, _)| *t == names[2])
        .map(|(_, p)| *p)
        .expect("third tab");
    click_at(&ctx, &mut app, third, Modifiers::NONE);
    assert_eq!(app.page_index(), 2);
    // Page Down twenty times: page 23, and its tab shows.
    for _ in 0..20 {
        frame(
            &ctx,
            &mut app,
            vec![key(egui::Key::PageDown, Modifiers::NONE)],
            Modifiers::NONE,
        );
    }
    assert_eq!(app.page_index(), 22);
    let shown = tabs(&ctx, &mut app);
    let at = shown
        .iter()
        .find(|(t, _)| *t == names[22])
        .map(|(_, p)| *p)
        .expect("the current page's tab shows");
    // A click on a tab after it, now in view.
    if let Some(p) = shown.iter().find(|(t, _)| *t == names[23]).map(|(_, p)| *p) {
        click_at(&ctx, &mut app, p, Modifiers::NONE);
        assert_eq!(app.page_index(), 23);
    }
    // Page Up: back one, still in view.
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::PageUp, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let now = app.page_index();
    assert!(tabs(&ctx, &mut app).iter().any(|(t, _)| *t == names[now]));
    // The wheel over the tabs slides them back to the first, the page
    // staying.
    for _ in 0..40 {
        frame(
            &ctx,
            &mut app,
            vec![
                Event::PointerMoved(at),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, 1.0),
                    modifiers: Modifiers::NONE,
                    phase: egui::TouchPhase::Move,
                },
            ],
            Modifiers::NONE,
        );
    }
    assert_eq!(app.page_index(), now);
    assert_eq!(app.page_tabs_first, 0);
    assert!(tabs(&ctx, &mut app).iter().any(|(t, _)| *t == names[0]));
}

/// The Print Merge dialog's Add Column form, for a number field, shows
/// the counter's digits, start, "continually increment" with its step and
/// the ending value (#791).
#[test]
fn add_column_offers_a_number_range() {
    let (ctx, mut app, _) = setup();
    let st = crate::ui::dialogs::PrintMergeState {
        adding: Some(("Check Number".into(), 1)),
        ..Default::default()
    };
    app.dialog = crate::ui::dialogs::Dialog::PrintMerge(st);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let (_, texts) = painted(&ctx, &mut app);
    for key in ["merge.increment", "merge.end_value"] {
        let label = crate::i18n::tr(key);
        assert!(
            texts.iter().any(|(t, _)| t.trim() == label.trim()),
            "{label:?} in {:?}",
            texts.iter().map(|(t, _)| t).collect::<Vec<_>>()
        );
    }
}

/// The Add Column form of a number field has a Format; with Custom it
/// shows the texts before and after the number (#930).
#[test]
fn add_column_offers_number_formats_and_custom_text() {
    let (ctx, mut app, _) = setup();
    let mut st = crate::ui::dialogs::PrintMergeState {
        adding: Some(("Ticket".into(), 1)),
        ..Default::default()
    };
    st.adding_number.format = crate::print_merge::NumberFormat::from_index(3);
    app.dialog = crate::ui::dialogs::Dialog::PrintMerge(st);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let (_, texts) = painted(&ctx, &mut app);
    for key in [
        "merge.format",
        "merge.format_custom",
        "merge.prefix",
        "merge.suffix",
    ] {
        let label = crate::i18n::tr(key);
        assert!(
            texts.iter().any(|(t, _)| t.trim() == label.trim()),
            "{label:?} in {:?}",
            texts.iter().map(|(t, _)| t).collect::<Vec<_>>()
        );
    }
}

/// With the wheel zooming (the default), a notch zooms in one glide over
/// a few frames, as a scroll moves, instead of one jump; the frames
/// together still zoom 1.15 a notch, about the pointer (#766).
#[test]
fn a_wheel_notch_zooms_in_a_glide() {
    let (ctx, mut app, _) = setup();
    assert!(app.settings.wheel_zooms);
    let at = Pos2::new(700.0, 500.0);
    let page_at = app.view.to_page(at);
    let z0 = app.view.zoom;
    let mut t = 10.0;
    let mut step = |app: &mut App, events: Vec<Event>| {
        t += 1.0 / 60.0;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            events,
            time: Some(t),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
    };
    step(&mut app, vec![Event::PointerMoved(at)]);
    step(&mut app, vec![Event::PointerMoved(at)]);
    step(
        &mut app,
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        }],
    );
    let first = app.view.zoom / z0;
    assert!(
        first > 1.0 && first < 1.14,
        "the first frame zooms part of it: {first}"
    );
    for _ in 0..60 {
        step(&mut app, Vec::new());
    }
    let total = app.view.zoom / z0;
    assert!((total - 1.15).abs() < 0.01, "a notch in all: {total}");
    // About the pointer: the page point under it stays.
    let now = app.view.to_page(at);
    assert!(
        (now.x - page_at.x).abs() < 0.5 && (now.y - page_at.y).abs() < 0.5,
        "{now:?} {page_at:?}"
    );
}

/// A right click on a button of the Standard toolbar opens the bar's
/// menu, as one on its empty part does (#835).
#[test]
fn a_right_click_on_a_toolbar_button_opens_the_bar_menu() {
    let (ctx, mut app, _) = setup();
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let slot = app
        .shown_bar_slots
        .iter()
        .find(|s| s.bar == crate::ui::bar_items::ItemBar::Standard)
        .cloned()
        .expect("the Standard toolbar");
    let at = slot.items[1].center();
    let menu =
        egui::Id::new(("bar_background", crate::ui::bars::Bar::Standard as u8)).with("bar_menu");
    assert!(!egui::Popup::is_id_open(&ctx, menu));
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(at)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(at, true, PointerButton::Secondary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(at, false, PointerButton::Secondary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert!(egui::Popup::is_id_open(&ctx, menu), "the bar's menu");
}

/// Ctrl+Z through a curve drawn with the Pen goes back to drawing it: the
/// Pen is chosen again and a line goes on from the last node left,
/// leaving along the mirror of its handle, with a curved preview; that
/// works after closing too, and Ctrl+Shift+Z takes the next node up
/// again (#631).
#[test]
fn undoing_pen_steps_goes_back_to_drawing_the_curve() {
    let (ctx, mut app, first) = setup();
    app.select(vec![first]);
    app.delete_selection();
    app.set_tool(Tool::Pen);
    let (a, b, c) = (
        Point::new(100.0, 150.0),
        Point::new(140.0, 150.0),
        Point::new(140.0, 190.0),
    );
    pull(&ctx, &mut app, a, Point::new(110.0, 140.0));
    pull(&ctx, &mut app, b, Point::new(150.0, 160.0));
    pull(&ctx, &mut app, c, Point::new(130.0, 200.0));
    click_page(&ctx, &mut app, a);
    assert!(app.curve.is_none(), "closed");
    let id = app.selection[0];
    let count = shape_count(&app);
    app.set_tool(Tool::Pick);
    let ctrl = Modifiers::COMMAND | Modifiers::CTRL;
    let undo = |ctx: &egui::Context, app: &mut App, mods: Modifiers| {
        frame(ctx, app, vec![key(egui::Key::Z, mods)], mods);
        frame(ctx, app, Vec::new(), Modifiers::NONE);
    };
    let nodes_of = |app: &App| match &app.doc().find_shape(id).map(|s| &s.kind) {
        Some(ShapeKind::Path { path, .. }) => tracedraw_core::nodes::nodes(path).len(),
        _ => 0,
    };
    let curved_preview = |app: &App| {
        let c = app.curve.as_ref().expect("a curve being drawn");
        crate::pen_look::curve_look(c, Some(Point::new(60.0, 60.0)), true, None)
            .preview
            .is_some_and(|p| {
                p.elements()
                    .iter()
                    .any(|e| matches!(e, tracedraw_core::geometry::PathEl::CurveTo(..)))
            })
    };
    // Undoing the close: the Pen again, on c, leaving along (130, 200).
    undo(&ctx, &mut app, ctrl);
    assert_eq!(app.tool, Tool::Pen);
    assert_eq!(app.selection, vec![id]);
    assert_eq!(nodes_of(&app), 3);
    let cur = app.curve.clone().expect("drawing again");
    assert_eq!(cur.nodes.len(), 1);
    assert!(near(cur.nodes[0].0, c), "{cur:?}");
    assert!(
        near(cur.nodes[0].1.expect("handle"), Point::new(130.0, 200.0)),
        "{cur:?}"
    );
    assert!(curved_preview(&app));
    // One node further back: on b, leaving along (150, 160).
    undo(&ctx, &mut app, ctrl);
    assert_eq!(nodes_of(&app), 2);
    let cur = app.curve.clone().expect("drawing again");
    assert!(near(cur.nodes[0].0, b), "{cur:?}");
    assert!(
        near(cur.nodes[0].1.expect("handle"), Point::new(150.0, 160.0)),
        "{cur:?}"
    );
    assert!(curved_preview(&app));
    // Redo takes c up again.
    undo(&ctx, &mut app, ctrl | Modifiers::SHIFT);
    assert_eq!(nodes_of(&app), 3);
    let cur = app.curve.clone().expect("drawing again");
    assert!(near(cur.nodes[0].0, c), "{cur:?}");
    // Going on from there joins the same curve.
    click_page(&ctx, &mut app, Point::new(90.0, 190.0));
    app.finish_curve();
    assert_eq!(shape_count(&app), count, "no new object");
    assert_eq!(nodes_of(&app), 4);
}

/// Redo leaves a curve being drawn alone: a fresh one stays, and nodes
/// drawn on from a curve an undo took up are not thrown away for the
/// step that undo took back (#631).
#[test]
fn redo_keeps_the_curve_being_drawn() {
    let (ctx, mut app, rect) = setup();
    let ctrl = Modifiers::COMMAND | Modifiers::CTRL;
    let press = |ctx: &egui::Context, app: &mut App, mods: Modifiers| {
        frame(ctx, app, vec![key(egui::Key::Z, mods)], mods);
        frame(ctx, app, Vec::new(), Modifiers::NONE);
    };
    let count = |app: &App| app.curve.as_ref().map(|c| c.nodes.len());
    // Undo the rectangle's fill, then start a fresh curve and redo.
    press(&ctx, &mut app, ctrl);
    app.select(Vec::new());
    app.set_tool(Tool::Pen);
    click_page(&ctx, &mut app, Point::new(100.0, 150.0));
    click_page(&ctx, &mut app, Point::new(140.0, 150.0));
    press(&ctx, &mut app, ctrl | Modifiers::SHIFT);
    assert_eq!(count(&app), Some(2), "the fresh curve stays");
    assert!(app.doc().find_shape(rect).is_some());
    // A curve of three nodes, finished, one node undone, then drawn on.
    click_page(&ctx, &mut app, Point::new(140.0, 190.0));
    app.finish_curve();
    let id = app.selection[0];
    press(&ctx, &mut app, ctrl);
    assert_eq!(count(&app), Some(1), "taken up again");
    click_page(&ctx, &mut app, Point::new(180.0, 150.0));
    press(&ctx, &mut app, ctrl | Modifiers::SHIFT);
    assert_eq!(count(&app), Some(2), "the node drawn on stays");
    app.finish_curve();
    let ShapeKind::Path { path, .. } = &app.doc().find_shape(id).expect("curve").kind else {
        panic!("a curve");
    };
    assert_eq!(tracedraw_core::nodes::nodes(path).len(), 3, "{path:?}");
}

/// Undoing a line that went on from a curve's first node takes the curve
/// up at that node, not at its other end; and after undoing a close, a
/// click on the other end closes the curve again (#631).
#[test]
fn undo_goes_back_to_the_end_drawing_went_on_from() {
    let (ctx, mut app, first) = setup();
    app.select(vec![first]);
    app.delete_selection();
    app.set_tool(Tool::Pen);
    let (a, b, c) = (
        Point::new(100.0, 150.0),
        Point::new(140.0, 150.0),
        Point::new(180.0, 150.0),
    );
    click_page(&ctx, &mut app, a);
    click_page(&ctx, &mut app, b);
    app.finish_curve();
    let id = app.selection[0];
    // On from the first node a.
    app.select(Vec::new());
    app.set_tool(Tool::Pen);
    click_page(&ctx, &mut app, a);
    click_page(&ctx, &mut app, Point::new(100.0, 190.0));
    app.finish_curve();
    assert_eq!(app.selection, vec![id]);
    let ctrl = Modifiers::COMMAND | Modifiers::CTRL;
    frame(&ctx, &mut app, vec![key(egui::Key::Z, ctrl)], ctrl);
    let cur = app.curve.clone().expect("drawing again");
    assert!(near(cur.nodes[0].0, a), "at the first node: {cur:?}");
    // A node further, then a click on the other end b closes the curve.
    click_page(&ctx, &mut app, c);
    click_page(&ctx, &mut app, b);
    assert!(app.curve.is_none(), "closed");
    let ShapeKind::Path { path, closed } = &app.doc().find_shape(id).expect("curve").kind else {
        panic!("a curve");
    };
    assert!(*closed, "{path:?}");
    assert_eq!(tracedraw_core::nodes::nodes(path).len(), 3, "{path:?}");
}

/// Ctrl+A in right-to-left text highlights all of it, not only the first
/// letter at its right end (#629).
#[test]
fn select_all_in_right_to_left_text_highlights_all_of_it() {
    use tracedraw_core::geometry::Shape as _;
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.settings.autocorrect.enabled = false;
    app.start_text(Point::new(30.0, 50.0), None);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        vec![Event::Text("\u{645}\u{631}\u{62D}\u{628}\u{627}".into())],
        Modifiers::NONE,
    );
    let Some((layout, to_page)) = app
        .text_edit
        .as_ref()
        .and_then(|te| app.text_layout_of(te.shape))
    else {
        panic!("a layout");
    };
    if layout.glyphs == 0 {
        return;
    }
    let ink = to_page.transform_rect_bbox(layout.path.bounding_box());
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::A, Modifiers::COMMAND)],
        Modifiers::COMMAND,
    );
    assert_eq!(app.text_selected().chars().count(), 5);
    let (_, _, quads) = app.text_caret_geometry().expect("caret");
    assert_eq!(quads.len(), 1, "{quads:?}");
    let xs = quads[0].iter().map(|p| p.x);
    let (x0, x1) = (
        xs.clone().fold(f64::MAX, f64::min),
        xs.fold(f64::MIN, f64::max),
    );
    assert!(
        x0 <= ink.x0 + 0.5 && x1 >= ink.x1 - 0.5,
        "{x0} {x1} {ink:?}"
    );
}

/// Arabic typed after an English word in the same line runs right to
/// left to the right of it, its first letter at its right end, and
/// selecting all shades both words (#628).
#[test]
fn right_to_left_typed_after_english_runs_right_to_left() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    app.settings.autocorrect.enabled = false;
    app.start_text(Point::new(30.0, 50.0), None);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    for typed in ["Asasa ", "\u{645}\u{631}\u{62D}\u{628}\u{627}"] {
        frame(
            &ctx,
            &mut app,
            vec![Event::Text(typed.into())],
            Modifiers::NONE,
        );
    }
    let Some((layout, _)) = app
        .text_edit
        .as_ref()
        .and_then(|te| app.text_layout_of(te.shape))
    else {
        panic!("a layout");
    };
    if layout.glyphs == 0 {
        return;
    }
    let line = &layout.lines[0];
    // The Arabic letters from the right, all right of "Asasa ".
    assert!(line.starts[6..].windows(2).all(|w| w[1] < w[0]), "{line:?}");
    assert!(
        line.starts[6..].iter().all(|x| *x >= line.ends[5] - 1e-6),
        "{line:?}"
    );
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::A, Modifiers::COMMAND)],
        Modifiers::COMMAND,
    );
    let (_, _, quads) = app.text_caret_geometry().expect("caret");
    assert_eq!(quads.len(), 1, "{quads:?}");
}

/// An Area Fill click inside a circle makes the circle itself: four
/// curves on it, not a jagged outline with many nodes (#734); a click
/// where the circle overlaps a rectangle makes the overlap, with the
/// circle's curve as its round side.
#[test]
fn area_fill_keeps_the_curves_it_fills_between() {
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    // A circle alone, and one overlapping the rectangle (10..60, 10..40).
    for rect in [
        Rect::new(120.0, 70.0, 180.0, 130.0),
        Rect::new(40.0, 20.0, 100.0, 80.0),
    ] {
        app.new_shape(ShapeKind::Ellipse { rect, arc: None })
            .expect("circle");
    }
    app.set_tool(Tool::AreaFill);
    let on_circle = |path: &tracedraw_core::BezPath, centre: Point| {
        let mut far: f64 = 0.0;
        tracedraw_core::geometry::flatten(path, 0.01, &mut |el| {
            if let tracedraw_core::geometry::PathEl::LineTo(q) = el {
                far = far.max(((q - centre).hypot() - 30.0).abs());
            }
        });
        far
    };
    // Inside the circle alone.
    click_page(&ctx, &mut app, Point::new(150.0, 110.0));
    let area = last_path(&app);
    let segs: Vec<_> = area.segments().collect();
    assert_eq!(segs.len(), 4, "{area:?}");
    let far = on_circle(&area, Point::new(150.0, 100.0));
    assert!(far < 0.02, "{far}");
    // The overlap of the rectangle (10..60, 10..40) and the circle.
    click_page(&ctx, &mut app, Point::new(55.0, 35.0));
    let overlap = last_path(&app);
    let b = tracedraw_core::geometry::Shape::bounding_box(&overlap);
    let x0 = 70.0 - (30.0f64.powi(2) - 10.0f64.powi(2)).sqrt();
    assert!(
        (b.x0 - x0).abs() < 0.01 && (b.x1 - 60.0).abs() < 1e-6,
        "{b:?}"
    );
    assert!(
        (b.y0 - (50.0 - (30.0f64.powi(2) - 10.0f64.powi(2)).sqrt())).abs() < 0.01
            && (b.y1 - 40.0).abs() < 1e-6,
        "{b:?}"
    );
    assert!(overlap.segments().count() <= 4, "{overlap:?}");
}

/// Shift+F3 opens Text > Change Case for the selected text, and does
/// nothing without one (#801).
#[test]
fn shift_f3_opens_change_case_for_selected_text() {
    use crate::ui::dialogs::Dialog;
    let (ctx, mut app, rect) = setup();
    app.select(vec![rect]);
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::F3, Modifiers::SHIFT)],
        Modifiers::SHIFT,
    );
    assert!(!matches!(app.dialog, Dialog::ChangeCase(_)));
    tracedraw_text::install();
    app.start_text(Point::new(20.0, 100.0), None);
    app.text_insert("hello world");
    let id = app.text_edit.as_ref().expect("editing").shape;
    app.finish_text();
    app.set_tool(Tool::Pick);
    app.select(vec![id]);
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::F3, Modifiers::SHIFT)],
        Modifiers::SHIFT,
    );
    assert!(
        matches!(app.dialog, Dialog::ChangeCase(_)),
        "{:?}",
        app.dialog
    );
}

/// Edit Fill's eyedropper: a click on the drawing gives the dialog the
/// colour there, though the dialog is open, and no tool takes the click
/// (#814).
#[test]
fn the_edit_fill_eyedropper_picks_a_colour_from_the_drawing() {
    use crate::ui::dialogs::Dialog;
    use tracedraw_core::{Color, Fill};
    let (ctx, mut app, rect) = setup();
    let red = Color::rgb8(220, 20, 20);
    app.select(vec![rect]);
    app.apply_fill(Fill::Solid(red));
    let other = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(100.0, 10.0, 150.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.select(vec![other]);
    app.apply_fill(Fill::Solid(Color::WHITE));
    app.open_edit_fill();
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    crate::ui::color_picker::start_pick(&ctx, egui::Id::new("edit_fill_pick"));
    let revision = app.engine.revision();
    let at = app.view.to_screen(Point::new(35.0, 25.0));
    click_at(&ctx, &mut app, at, Modifiers::NONE);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert!(!crate::ui::color_picker::picking(&ctx));
    let Dialog::EditFill(st) = &app.dialog else {
        panic!("the dialog stays open: {:?}", app.dialog);
    };
    assert_eq!(st.fill, Fill::Solid(red));
    assert_eq!(app.selection, vec![other], "no tool took the click");
    let _ = revision;
    // Esc stops a pick and leaves the dialog open.
    crate::ui::color_picker::start_pick(&ctx, egui::Id::new("edit_fill_pick"));
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Escape, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert!(!crate::ui::color_picker::picking(&ctx));
    assert!(matches!(app.dialog, Dialog::EditFill(_)));
}

/// Frame times of a node drag with the Shape tool on a curve of 3000
/// nodes, the node snapping to the rest of its own curve (#673): the
/// still part is found once a drag and only its pieces near the pointer
/// are looked at, so a frame costs about what it did without it.
#[test]
#[ignore]
fn frame_times_of_a_node_drag_on_a_long_curve() {
    use tracedraw_core::geometry::BezPath;
    let (ctx, mut app, _) = setup();
    let mut p = BezPath::new();
    for i in 0..3000 {
        let q = (
            10.0 + (i % 60) as f64 * 3.0,
            10.0 + (i / 60) as f64 * 5.0 + (i % 2) as f64,
        );
        if i == 0 {
            p.move_to(q);
        } else {
            p.line_to(q);
        }
    }
    let curve = app
        .new_shape(ShapeKind::Path {
            path: p,
            closed: false,
        })
        .expect("curve");
    app.select(vec![curve]);
    app.set_tool(Tool::Shape);
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    // The node at (100, 135): row 25, column 30.
    let from = app.view.to_screen(Point::new(100.0, 135.0));
    click_at(&ctx, &mut app, from, Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        vec![button(from, true, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let n = 300;
    let mut total = 0.0;
    for i in 0..n {
        let to = from + egui::vec2((i % 40) as f32 * 2.0, (i / 40) as f32 * 3.0 + 1.0);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            events: vec![Event::PointerMoved(to)],
            time: Some(i as f64 * 0.016),
            ..Default::default()
        };
        let t = std::time::Instant::now();
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(&mut app, ui));
        total += t.elapsed().as_secs_f64();
        out.textures_delta.clear();
    }
    assert!(
        matches!(app.drag, Drag::NodeMove { .. }),
        "a node drag: {:?}",
        app.drag
    );
    eprintln!(
        "node drag on 3000 nodes: ui {:.3} ms a frame",
        total * 1000.0 / n as f64
    );
}

/// #804, Mesh Fill: a double-click inside the mesh puts a node exactly
/// where it was (the new line went halfway between the nearest nodes),
/// selected, and that node dragged lands under the pointer.
#[test]
fn mesh_nodes_land_where_they_are_clicked_and_dragged() {
    let (ctx, mut app, id) = setup();
    app.mesh_rows = 2;
    app.mesh_cols = 2;
    app.set_tool(Tool::MeshFill);
    // The rectangle (10..60 x 10..40): a click inside gives it a mesh.
    click_page(&ctx, &mut app, Point::new(30.0, 20.0));
    let mesh = |app: &App| match &app.doc().find_shape(id).expect("shape").fill {
        tracedraw_core::Fill::Mesh(m) => m.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!((mesh(&app).rows, mesh(&app).cols), (2, 2));
    let at = Point::new(19.0, 31.0);
    let at_screen = app.view.to_screen(at);
    double_click_at(&ctx, &mut app, at_screen);
    let m = mesh(&app);
    assert_eq!((m.rows, m.cols), (3, 3));
    let tol = 1.0 / app.view.zoom as f64;
    let i = m.nearest(at).expect("node");
    assert!((m.nodes[i].pos - at).hypot() <= tol, "{:?}", m.nodes[i].pos);
    assert_eq!(
        app.selected_effect_node,
        Some((id, crate::mesh_tool::MESH_NODE_BASE + i))
    );
    // Drag that node 15 mm right: it ends under the pointer.
    let to = Point::new(34.0, 31.0);
    let to_screen = app.view.to_screen(to);
    drag(
        &ctx,
        &mut app,
        at_screen,
        to_screen,
        Modifiers::NONE,
        Vec::new(),
    );
    let moved = mesh(&app).nodes[i].pos;
    assert!((moved - to).hypot() <= tol, "{moved:?}");
    // One undo step puts it back.
    app.undo();
    assert!((mesh(&app).nodes[i].pos - at).hypot() <= tol);
}

/// The property bar's grip dragged onto the standard toolbar's row puts
/// the two side by side in one row, the drawing window taking the band
/// the property bar left; dragged to the line under that row it gets a
/// row of its own again (#837, #855).
#[test]
fn a_toolbar_dragged_onto_another_row_shares_it() {
    use crate::ui::bars::{Bar, Dock};
    let (ctx, mut app, _) = setup();
    let rows = |app: &App| -> Vec<Vec<Bar>> {
        app.bar_rows
            .iter()
            .filter(|r| r.dock == Dock::Top)
            .map(|r| r.slots.iter().map(|(b, _)| *b).collect())
            .collect()
    };
    assert_eq!(rows(&app), vec![vec![Bar::Standard], vec![Bar::Property]]);
    let canvas_top = app.canvas_rect.top();
    let slot = |app: &App, bar: Bar| {
        app.bar_rows
            .iter()
            .flat_map(|r| r.slots.iter())
            .find(|(b, _)| *b == bar)
            .map(|(_, r)| *r)
            .expect("slot")
    };
    let grip = slot(&app, Bar::Property);
    let standard_row = app.bar_rows[0].rect;
    drag(
        &ctx,
        &mut app,
        Pos2::new(grip.left() + 6.0, grip.center().y),
        Pos2::new(standard_row.right() - 200.0, standard_row.center().y),
        Modifiers::NONE,
        Vec::new(),
    );
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    assert_eq!(rows(&app), vec![vec![Bar::Standard, Bar::Property]]);
    assert_eq!(
        app.settings.bars.rows_at(Dock::Top)[0],
        vec![Bar::Standard, Bar::Property]
    );
    // One band now, as high as the property bar: the window grows.
    let row = app.bar_rows[0].rect;
    assert!((row.height() - 53.0).abs() <= 1.5, "{row:?}");
    assert!(app.canvas_rect.top() < canvas_top - 30.0);
    // Side by side: the standard toolbar first, whole, the property bar
    // after it.
    let (s, p) = (slot(&app, Bar::Standard), slot(&app, Bar::Property));
    assert!(s.right() <= p.left() + 0.5 && (s.top() - p.top()).abs() < 0.5);
    // Down to the line under the row: a row of its own again.
    let grip = slot(&app, Bar::Property);
    drag(
        &ctx,
        &mut app,
        Pos2::new(grip.left() + 6.0, grip.center().y),
        Pos2::new(600.0, row.bottom() - 2.0),
        Modifiers::NONE,
        Vec::new(),
    );
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    assert_eq!(rows(&app), vec![vec![Bar::Standard], vec![Bar::Property]]);
}

/// Dragging a contour out of a line of letters (#897): the CPU went from
/// 5 % to 60 % because the contour was worked out again for every draw,
/// measure and pick of a frame, the pointer resting on the object included.
/// `cargo test --release -p tracedraw -- --ignored frame_times_of_a_contour_drag --nocapture`.
#[test]
#[ignore]
fn frame_times_of_a_contour_drag() {
    // Letters as outlines: 25 of them, each a wobbly curved ring with a
    // counter, about what a line of text is once its glyphs are outlined.
    let (ctx, mut app, _) = setup();
    app.select(Vec::new());
    let mut path = tracedraw_core::geometry::BezPath::new();
    for letter in 0..25 {
        let (cx, cy) = (20.0 + letter as f64 * 8.0, 100.0);
        for (r, dir) in [(4.0, 1.0), (1.6, -1.0)] {
            let seg = 24;
            let at = |k: usize| {
                let a = dir * k as f64 / seg as f64 * std::f64::consts::TAU;
                let w = r * (1.0 + 0.12 * (a * 5.0).sin());
                (cx + w * a.cos(), cy + w * a.sin())
            };
            path.move_to(at(0));
            for k in 1..=seg {
                let (p0, p1) = (at(k - 1), at(k));
                path.curve_to(
                    (p0.0 + (p1.0 - p0.0) / 3.0, p0.1 + (p1.1 - p0.1) / 3.0),
                    (
                        p0.0 + (p1.0 - p0.0) * 2.0 / 3.0,
                        p0.1 + (p1.1 - p0.1) * 2.0 / 3.0,
                    ),
                    p1,
                );
            }
            path.close_path();
        }
    }
    let text = app
        .new_shape(ShapeKind::Path { path, closed: true })
        .expect("letters");
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::BLACK));
    app.set_tool(Tool::Contour);
    app.select(vec![text]);
    let b = app.doc().shape(text).expect("text").1.bounds();
    app.view.zoom = 1.0;
    let start = Point::new(b.center().x, b.y0 + 1.0);
    assert!(app.begin_contour(start));
    let n = 60;
    let mut drag = std::time::Duration::ZERO;
    let mut paint = std::time::Duration::ZERO;
    for i in 0..n {
        let p = Point::new(start.x, start.y - 1.0 - i as f64 * 0.3);
        let t = std::time::Instant::now();
        app.drag_contour(p);
        drag += t.elapsed();
        let t = std::time::Instant::now();
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
        paint += t.elapsed();
    }
    let time = |label: &str, f: &mut dyn FnMut()| {
        let t = std::time::Instant::now();
        for _ in 0..20 {
            f();
        }
        eprintln!("  {label}: {:?}", t.elapsed() / 20);
    };
    // Pointing at the object while the pointer rests: the contour is not
    // worked out again.
    time("hit_test", &mut || {
        let _ = app.hit_test(start);
    });
    eprintln!(
        "contour drag on text: {:?} per move, {:?} per frame",
        drag / n,
        paint / n
    );
}

/// Over a black fill, what is being drawn shows (#672): a freehand stroke,
/// a polyline in progress and a 3-point tool's base line waiting for the
/// third click are black lines on white ones along them, the Bezier
/// tool's placed segments, in the black outline they will get, lie on a
/// white line too, and the polyline's node squares have a white edge.
#[test]
fn lines_being_drawn_show_on_a_black_fill() {
    use egui::epaint::ColorMode;
    let (ctx, mut app, _) = setup();
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::BLACK));
    app.select(Vec::new());
    // Distance from `p` to the polyline `along`.
    let off = |p: Pos2, along: &[Pos2]| {
        along
            .windows(2)
            .map(|w| {
                let (a, b) = (w[0], w[1]);
                let ab = b - a;
                let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
                (a + ab * t - p).length()
            })
            .fold(f32::MAX, f32::min)
    };
    // The index of a line of `color` and `width` lying along `along`
    // (within 3 px), segment or path.
    let line_at = |shapes: &[egui::Shape], color: egui::Color32, width: f32, along: &[Pos2]| {
        shapes.iter().position(|s| {
            let (pts, w, c): (Vec<Pos2>, f32, bool) = match s {
                egui::Shape::LineSegment { points, stroke } => {
                    (points.to_vec(), stroke.width, stroke.color == color)
                }
                egui::Shape::Path(ps) => (
                    ps.points.clone(),
                    ps.stroke.width,
                    ps.stroke.color == ColorMode::Solid(color),
                ),
                _ => return false,
            };
            c && (w - width).abs() < 1e-3
                && pts.len() > 1
                && pts.iter().all(|p| off(*p, along) < 3.0)
        })
    };
    let haloed = |shapes: &[egui::Shape], inner: egui::Color32, along: &[Pos2]| {
        let white = line_at(
            shapes,
            egui::Color32::WHITE,
            crate::canvas::HALO_WIDTH,
            along,
        );
        let line = line_at(shapes, inner, crate::canvas::HALO_INNER, along);
        matches!((white, line), (Some(w), Some(l)) if l > w)
    };
    let a = app.view.to_screen(Point::new(20.0, 20.0));
    let b = app.view.to_screen(Point::new(50.0, 30.0));
    let c = app.view.to_screen(Point::new(40.0, 35.0));
    // A freehand stroke, while the button is down.
    app.set_tool(Tool::Freehand);
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(a)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(a, true, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let mut stroke = vec![a];
    for i in 1..=4 {
        let p = a + (b - a) * (i as f32 / 4.0) + egui::vec2(0.0, (i % 2) as f32 * 6.0);
        stroke.push(p);
        frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(p)],
            Modifiers::NONE,
        );
    }
    let shapes = frame_shapes(&ctx, &mut app, Vec::new());
    assert!(haloed(&shapes, egui::Color32::BLACK, &stroke), "freehand");
    frame(
        &ctx,
        &mut app,
        vec![button(b, false, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    // A polyline with two points placed, the pointer away.
    app.select(Vec::new());
    app.set_tool(Tool::Polyline);
    click_at(&ctx, &mut app, a, Modifiers::NONE);
    click_at(&ctx, &mut app, b, Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(c)],
        Modifiers::NONE,
    );
    assert!(app.curve.is_some(), "a polyline in progress");
    let shapes = frame_shapes(&ctx, &mut app, Vec::new());
    assert!(
        haloed(&shapes, egui::Color32::BLACK, &[a, b, c]),
        "polyline"
    );
    // Its node squares have a white edge.
    for p in [a, b] {
        assert!(
            shapes.iter().any(|s| matches!(s, egui::Shape::Rect(r)
                if r.stroke.color == egui::Color32::WHITE
                    && (r.rect.center() - p).length() < 1.5)),
            "a white edge at {p:?}"
        );
    }
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Escape, Modifiers::NONE)],
        Modifiers::NONE,
    );
    // The Bezier tool: a segment placed, in the default black outline,
    // away from the freehand stroke and the polyline (Esc keeps the
    // polyline): a click on their end nodes would go on from them.
    app.curve = None;
    app.select(Vec::new());
    app.set_tool(Tool::Bezier);
    // Snapping off: the nodes go where they are clicked.
    app.snap.off = true;
    let (a2, b2) = (
        app.view.to_screen(Point::new(15.0, 37.0)),
        app.view.to_screen(Point::new(55.0, 38.0)),
    );
    click_at(&ctx, &mut app, a2, Modifiers::NONE);
    click_at(&ctx, &mut app, b2, Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(c)],
        Modifiers::NONE,
    );
    assert!(app.curve.is_some(), "a curve in progress");
    let shapes = frame_shapes(&ctx, &mut app, Vec::new());
    let outline = app
        .default_stroke()
        .map(|s| crate::canvas::to_color32(s.color))
        .expect("a default outline");
    assert!(haloed(&shapes, outline, &[a2, b2]), "placed segments");
    app.snap.off = false;
    frame(
        &ctx,
        &mut app,
        vec![key(egui::Key::Escape, Modifiers::NONE)],
        Modifiers::NONE,
    );
    // A 3-point curve's base line, released and waiting for the
    // third click.
    app.curve = None;
    app.select(Vec::new());
    app.set_tool(Tool::ThreePointCurve);
    drag(&ctx, &mut app, a, b, Modifiers::NONE, Vec::new());
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(c)],
        Modifiers::NONE,
    );
    assert!(
        app.three_point_base.is_some(),
        "waiting for the third click"
    );
    let shapes = frame_shapes(&ctx, &mut app, Vec::new());
    assert!(
        haloed(&shapes, egui::Color32::BLACK, &[a, b]),
        "3-point base"
    );
}

#[test]
fn a_click_with_a_drawing_tool_selects_and_a_drag_draws() {
    let (ctx, mut app, id) = setup();
    app.set_tool(Tool::Rectangle);
    app.select(Vec::new());
    let inside = app.view.to_screen(Point::new(35.0, 25.0));
    let before = shape_count(&app);
    // A click on the square selects it and draws nothing.
    click_at(&ctx, &mut app, inside, Modifiers::NONE);
    assert_eq!(app.selection, vec![id]);
    assert_eq!(shape_count(&app), before, "a click draws nothing");
    assert_eq!(app.tool, Tool::Rectangle, "the tool stays");
    // A drag that starts on the square draws a new rectangle.
    let to = app.view.to_screen(Point::new(90.0, 70.0));
    drag(&ctx, &mut app, inside, to, Modifiers::NONE, Vec::new());
    assert_eq!(shape_count(&app), before + 1, "a drag draws");
    assert_eq!(app.tool, Tool::Rectangle);
    assert!(!app.selection.contains(&id));
    // A click on empty space lets go of the selection.
    let empty = app.view.to_screen(Point::new(150.0, 150.0));
    click_at(&ctx, &mut app, empty, Modifiers::NONE);
    assert!(app.selection.is_empty(), "{:?}", app.selection);
    assert_eq!(shape_count(&app), before + 1);
}

#[test]
fn shift_click_with_a_drawing_tool_adds_and_takes_away() {
    let (ctx, mut app, id) = setup();
    let other = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(100.0, 10.0, 140.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.select(vec![other]);
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
        30, 30, 200,
    )));
    for tool in [Tool::Ellipse, Tool::Spiral, Tool::Smear, Tool::Knife] {
        app.set_tool(tool);
        app.select(vec![id]);
        let at = app.view.to_screen(Point::new(120.0, 25.0));
        click_at(&ctx, &mut app, at, Modifiers::SHIFT);
        assert!(
            app.selection.contains(&id) && app.selection.contains(&other),
            "{tool:?}: {:?}",
            app.selection
        );
        click_at(&ctx, &mut app, at, Modifiers::SHIFT);
        assert_eq!(app.selection, vec![id], "{tool:?}");
        // Shift on empty space keeps the selection.
        let empty = app.view.to_screen(Point::new(150.0, 150.0));
        click_at(&ctx, &mut app, empty, Modifiers::SHIFT);
        assert_eq!(app.selection, vec![id], "{tool:?}");
        assert_eq!(app.tool, tool);
    }
}

#[test]
fn line_tools_still_place_points_on_objects() {
    let (ctx, mut app, id) = setup();
    app.select(Vec::new());
    let before = shape_count(&app);
    app.set_tool(Tool::Polyline);
    let a = app.view.to_screen(Point::new(35.0, 25.0));
    let b = app.view.to_screen(Point::new(50.0, 30.0));
    click_at(&ctx, &mut app, a, Modifiers::NONE);
    click_at(&ctx, &mut app, b, Modifiers::NONE);
    assert!(
        !app.selection.contains(&id),
        "a click on an object places a point, it does not select"
    );
    app.finish_curve();
    assert_eq!(shape_count(&app), before + 1, "the line was drawn");
}

#[test]
fn a_click_outside_a_table_selects_with_the_table_tool() {
    let (ctx, mut app, id) = setup();
    app.set_tool(Tool::Table);
    app.select(Vec::new());
    let before = shape_count(&app);
    let inside = app.view.to_screen(Point::new(35.0, 25.0));
    click_at(&ctx, &mut app, inside, Modifiers::NONE);
    assert_eq!(app.selection, vec![id]);
    assert_eq!(shape_count(&app), before);
}

#[test]
fn every_click_selecting_tool_selects_on_click() {
    let (ctx, mut app, id) = setup();
    let inside = app.view.to_screen(Point::new(35.0, 25.0));
    let empty = app.view.to_screen(Point::new(150.0, 150.0));
    for tool in Tool::ALL.iter().copied().filter(|t| t.click_selects()) {
        app.set_tool(tool);
        if app.tool != tool {
            continue; // a tool that is not available here
        }
        app.select(Vec::new());
        let before = shape_count(&app);
        click_at(&ctx, &mut app, inside, Modifiers::NONE);
        assert_eq!(app.selection, vec![id], "{tool:?}");
        click_at(&ctx, &mut app, empty, Modifiers::NONE);
        assert!(app.selection.is_empty(), "{tool:?}");
        assert_eq!(shape_count(&app), before, "{tool:?} drew on a click");
    }
}

/// While text is typed, Ctrl+L, E, R, J, H and N align it; the same keys
/// do not touch text alignment when nothing is being typed.
#[test]
fn alignment_keys_while_typing() {
    use tracedraw_core::TextAlign;
    tracedraw_text::install();
    let (ctx, mut app, _id) = setup();
    app.set_tool(Tool::Text);
    app.start_text(Point::new(100.0, 150.0), None);
    app.text_insert("Hello\nworld");
    let align_of = |app: &App| {
        let id = app.text_edit.as_ref().map(|t| t.shape).expect("editing");
        match &app.doc().find_shape(id).expect("text").kind {
            ShapeKind::Text {
                spans, align, para, ..
            } => tracedraw_core::paragraphs::paragraph_styles(spans, *align, para)
                .last()
                .map(|p| p.0)
                .unwrap_or(*align),
            _ => unreachable!(),
        }
    };
    for (k, a) in [
        (egui::Key::E, TextAlign::Center),
        (egui::Key::R, TextAlign::Right),
        (egui::Key::J, TextAlign::Justify),
        (egui::Key::H, TextAlign::ForceJustify),
        (egui::Key::N, TextAlign::None),
        (egui::Key::L, TextAlign::Left),
    ] {
        frame(
            &ctx,
            &mut app,
            vec![key(k, Modifiers::COMMAND)],
            Modifiers::COMMAND,
        );
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
        assert_eq!(align_of(&app), a, "{k:?}");
        assert!(app.text_edit.is_some(), "still typing after {k:?}");
        // Text typed at a point lines up as a whole.
        let id = app.text_edit.as_ref().map(|t| t.shape).expect("editing");
        if let ShapeKind::Text {
            spans, align, para, ..
        } = &app.doc().find_shape(id).expect("text").kind
        {
            let all = tracedraw_core::paragraphs::paragraph_styles(spans, *align, para);
            assert!(
                all.iter().all(|p| p.0 == a),
                "{k:?}: {:?}",
                all.iter().map(|p| p.0).collect::<Vec<_>>()
            );
        }
    }
}

/// Frame times of a zoom in the Multipage View (#903): nine pages of
/// many objects each, zoomed out step by step until they all show, then
/// left to rest. Run with
/// `cargo test --release -- --ignored frame_times_of_a_multipage_zoom --nocapture`.
#[test]
#[ignore]
fn frame_times_of_a_multipage_zoom() {
    let (ctx, mut app, _) = setup();
    let n: usize = std::env::var("FRAME_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1500);
    let mut s = 11u64;
    let mut r = || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((s >> 33) as f64) / (1u64 << 31) as f64
    };
    let first = app.page;
    for page in 0..9 {
        if page > 0 {
            app.add_page();
        }
        for _ in 0..n {
            let (x, y) = (r() * 200.0, r() * 280.0);
            let mut p = tracedraw_core::geometry::BezPath::new();
            p.move_to((x, y));
            for _ in 0..6 {
                p.curve_to(
                    (x + r() * 20.0 - 10.0, y + r() * 20.0 - 10.0),
                    (x + r() * 20.0 - 10.0, y + r() * 20.0 - 10.0),
                    (x + r() * 20.0 - 10.0, y + r() * 20.0 - 10.0),
                );
            }
            p.close_path();
            app.new_shape(ShapeKind::Path {
                path: p,
                closed: true,
            });
        }
    }
    app.page = first;
    app.select(Vec::new());
    app.set_multipage(true);
    let mut time = 0.0f64;
    let mut timed = |app: &mut App| {
        time += 1.0 / 60.0;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            time: Some(time),
            ..Default::default()
        };
        let t = std::time::Instant::now();
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
        t.elapsed()
    };
    for _ in 0..3 {
        timed(&mut app);
    }
    app.zoom_to_page();
    for _ in 0..30 {
        timed(&mut app);
    }
    let at = egui::pos2(800.0, 500.0);
    let steps = 24u32;
    let mut sum = std::time::Duration::ZERO;
    let mut worst = std::time::Duration::ZERO;
    for _ in 0..steps {
        app.view.zoom_at(at, 0.93);
        let d = timed(&mut app);
        sum += d;
        worst = worst.max(d);
    }
    let mut rest_sum = std::time::Duration::ZERO;
    let mut rest_worst = std::time::Duration::ZERO;
    let mut rest = Vec::new();
    for _ in 0..30 {
        let d = timed(&mut app);
        rest_sum += d;
        rest_worst = rest_worst.max(d);
        rest.push(d.as_millis());
    }
    eprintln!("frames at rest, ms: {rest:?}");
    let t = std::time::Instant::now();
    let copy = app.doc().clone();
    eprintln!(
        "a copy of the drawing ({} pages): {:?}",
        copy.pages.len(),
        t.elapsed()
    );
    eprintln!(
        "multipage zoom: {:?} per frame, worst {:?}; at rest after: {:?} per frame, worst {:?}",
        sum / steps,
        worst,
        rest_sum / 30,
        rest_worst
    );
}

/// The Print dialog's Layout tab has the paper margins, which Print
/// Preview had alone, and a margin typed there reaches the layout (#909).
#[test]
fn the_print_layout_tab_has_the_paper_margins() {
    fn texts(ctx: &egui::Context, app: &mut App) -> Vec<String> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
        fn walk(s: &egui::Shape, texts: &mut Vec<String>) {
            match s {
                egui::Shape::Text(t) => texts.push(t.galley.text().to_string()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, texts)),
                _ => {}
            }
        }
        let mut texts = Vec::new();
        out.shapes.iter().for_each(|c| walk(&c.shape, &mut texts));
        texts
    }
    use crate::ui::dialogs::Dialog;
    let (ctx, mut app, _) = setup();
    app.print();
    match &mut app.dialog {
        Dialog::Print(st) => {
            st.tab = 3;
            st.margins = [7.0, 0.0, 0.0, 0.0];
        }
        other => panic!("no Print dialog: {other:?}"),
    }
    let shown = texts(&ctx, &mut app);
    let shown = texts(&ctx, &mut app)
        .into_iter()
        .chain(shown)
        .collect::<Vec<_>>();
    assert!(
        shown
            .iter()
            .any(|t| *t == crate::i18n::tr("print.paper_margins")),
        "{shown:?}"
    );
    assert!(shown.iter().any(|t| t.starts_with("7")), "{shown:?}");
    let Dialog::Print(st) = &app.dialog else {
        panic!("the Print dialog closed");
    };
    let layout = crate::export::print_layout(&app, st, app.doc());
    assert_eq!(layout.paper_margins[0], 7.0);
}

/// Dragging a drop shadow out of an object (#935): it followed the
/// pointer slowly and in jerks in the browser, where the page is drawn on
/// the window's own thread. Times the drag step, the frame and the page
/// render of each move.
/// `cargo test --release -p tracedraw -- --ignored frame_times_of_a_shadow_drag --nocapture`.
#[test]
#[ignore]
fn frame_times_of_a_shadow_drag() {
    let (ctx, mut app, id) = setup();
    app.set_tool(Tool::DropShadow);
    app.select(vec![id]);
    let b = app.doc().shape(id).expect("rect").1.bounds();
    let start = app.view.to_screen(b.center());
    frame(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(start)],
        Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![button(start, true, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    let ppp = ctx.pixels_per_point();
    let rect = app.canvas_rect;
    let opts = |app: &App| tracedraw_render::RenderOptions {
        width: (rect.width() * ppp) as u32,
        height: (rect.height() * ppp) as u32,
        view: tracedraw_render::ViewTransform {
            zoom: (app.view.zoom * ppp) as f64,
            origin_x: ((app.view.origin.x - rect.min.x) * ppp) as f64,
            origin_y: ((app.view.origin.y - rect.min.y) * ppp) as f64,
        },
        draft: true,
        ..Default::default()
    };
    let n = 40;
    let (mut paint, mut render) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    for i in 1..=n {
        let p = start + egui::vec2(i as f32 * 3.0, i as f32 * 2.0);
        let t = std::time::Instant::now();
        frame(
            &ctx,
            &mut app,
            vec![Event::PointerMoved(p)],
            Modifiers::NONE,
        );
        paint += t.elapsed();
        let o = opts(&app);
        let t = std::time::Instant::now();
        let _ = tracedraw_render::render_page(app.doc(), app.page, &o);
        render += t.elapsed();
    }
    let end = start + egui::vec2(n as f32 * 3.0, n as f32 * 2.0);
    frame(
        &ctx,
        &mut app,
        vec![button(end, false, PointerButton::Primary, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert!(app.doc().find_shape(id).and_then(|s| s.shadow).is_some());
    let o = opts(&app);
    // The same page without the shadow, for comparison: what any render
    // of this window costs.
    let mut plain = app.doc().clone();
    for l in plain.pages.iter_mut().flat_map(|p| p.layers.iter_mut()) {
        for s in &mut l.shapes {
            s.shadow = None;
        }
    }
    let t = std::time::Instant::now();
    for _ in 0..n {
        let _ = tracedraw_render::render_page(&plain, app.page, &o);
    }
    let unshadowed = t.elapsed() / n;
    eprintln!(
        "shadow drag ({} x {} px): {:?} per frame, {:?} per page render ({:?} without the shadow)",
        o.width,
        o.height,
        paint / n,
        render / n,
        unshadowed
    );
}

/// A created drop shadow is picked for editing (#935): a Pick tool click
/// on the shadow, away from its object, selects the object; with the
/// Drop Shadow tool a click there shows its handles and a drag on it
/// moves the whole shadow by the drag, the object staying where it is.
#[test]
fn a_click_on_a_drop_shadow_picks_its_object() {
    let (ctx, mut app, id) = setup();
    app.set_shadow(
        Some(tracedraw_core::Shadow {
            offset: Vec2::new(70.0, 0.0),
            ..Default::default()
        }),
        false,
    );
    let shadow = |app: &App| app.doc().find_shape(id).and_then(|s| s.shadow);
    let click = |ctx: &egui::Context, app: &mut App, at: Pos2| {
        frame(ctx, app, vec![Event::PointerMoved(at)], Modifiers::NONE);
        frame(
            ctx,
            app,
            vec![button(at, true, PointerButton::Primary, Modifiers::NONE)],
            Modifiers::NONE,
        );
        frame(
            ctx,
            app,
            vec![button(at, false, PointerButton::Primary, Modifiers::NONE)],
            Modifiers::NONE,
        );
        frame(ctx, app, Vec::new(), Modifiers::NONE);
    };
    // The rectangle spans 10 to 60 by 10 to 40; its shadow 80 to 130.
    let on_shadow = app.view.to_screen(Point::new(105.0, 25.0));
    let beside = app.view.to_screen(Point::new(170.0, 25.0));
    click(&ctx, &mut app, beside);
    assert!(app.selection.is_empty());
    click(&ctx, &mut app, on_shadow);
    assert_eq!(app.selection, vec![id]);
    // The Drop Shadow tool: a click there selects it too, with the handles.
    click(&ctx, &mut app, beside);
    assert!(app.selection.is_empty());
    app.set_tool(Tool::DropShadow);
    click(&ctx, &mut app, on_shadow);
    assert_eq!(app.selection, vec![id]);
    assert!(app.selected_shadow().is_some());
    // A drag on the shadow moves it whole.
    let before = shadow(&app).expect("shadow").offset;
    let zoom = app.view.zoom;
    drag(
        &ctx,
        &mut app,
        on_shadow,
        on_shadow + egui::vec2(20.0 * zoom, -10.0 * zoom),
        Modifiers::NONE,
        Vec::new(),
    );
    let after = shadow(&app).expect("shadow").offset;
    assert!(
        (after.x - before.x - 20.0).abs() < 0.5 && (after.y - before.y - 10.0).abs() < 0.5,
        "{before:?} -> {after:?}"
    );
    assert!(close(bounds(&app, id).x0, 10.0), "the object stays");
}

/// Dragging an envelope's node (#881): the frame and the enveloped
/// object follow every move; a line of outlined letters and a plain
/// rectangle, zoomed in so the frame is large on screen.
/// `cargo test --release -p tracedraw -- --ignored frame_times_of_an_envelope_drag --nocapture`.
#[test]
#[ignore]
fn frame_times_of_an_envelope_drag() {
    use crate::envelope_edit::Part;
    use tracedraw_core::envelope::Mode;
    let (ctx, mut app, rect) = setup();
    let mut path = tracedraw_core::geometry::BezPath::new();
    for letter in 0..25 {
        let (cx, cy) = (20.0 + letter as f64 * 8.0, 100.0);
        for (r, dir) in [(4.0, 1.0), (1.6, -1.0)] {
            let seg = 24;
            let at = |k: usize| {
                let a = dir * k as f64 / seg as f64 * std::f64::consts::TAU;
                let w = r * (1.0 + 0.12 * (a * 5.0).sin());
                (cx + w * a.cos(), cy + w * a.sin())
            };
            path.move_to(at(0));
            for k in 1..=seg {
                let (p0, p1) = (at(k - 1), at(k));
                path.curve_to(
                    (p0.0 + (p1.0 - p0.0) / 3.0, p0.1 + (p1.1 - p0.1) / 3.0),
                    (
                        p0.0 + (p1.0 - p0.0) * 2.0 / 3.0,
                        p0.1 + (p1.1 - p0.1) * 2.0 / 3.0,
                    ),
                    p1,
                );
            }
            path.close_path();
        }
    }
    app.select(Vec::new());
    let text = app
        .new_shape(ShapeKind::Path { path, closed: true })
        .expect("letters");
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::BLACK));
    app.set_tool(Tool::Envelope);
    for (label, id) in [("rectangle", rect), ("letters", text)] {
        for mode in [Mode::Unconstrained, Mode::SingleArc] {
            app.select(vec![id]);
            app.add_new_envelope();
            app.envelope_mode = mode;
            let (_, f) = app.envelope_of(id).expect("envelope");
            let page = app.doc().shape_on_page(id).expect("shape");
            let start = page.transform * f.nodes[5];
            app.view.zoom = 4.0;
            app.begin_envelope_drag(id, Part::Node(5), start);
            let n = 60;
            let mut drag = std::time::Duration::ZERO;
            let mut paint = std::time::Duration::ZERO;
            for i in 0..n {
                let p = Point::new(start.x + i as f64 * 0.2, start.y + i as f64 * 0.5);
                let t = std::time::Instant::now();
                app.envelope_drag_to(p);
                drag += t.elapsed();
                let t = std::time::Instant::now();
                frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
                paint += t.elapsed();
            }
            app.envelope_drag = None;
            app.clear_envelope();
            eprintln!(
                "envelope drag on {label} ({mode:?}): {:?} per move, {:?} per frame",
                drag / n,
                paint / n
            );
        }
    }
}
